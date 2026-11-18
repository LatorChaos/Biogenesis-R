// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::{WorldFileError, object_stream::Limits};
use std::path::{Path, PathBuf};

pub(super) fn world_file_limits() -> Limits {
    Limits {
        allocated_bytes: 1024 * 1024 * 1024,
        work: 512_000_000,
        ..Limits::default()
    }
}

#[godot_api(secondary)]
impl BgrSimulation {
    #[func]
    fn world_destination(&self, path: GString) -> VarDictionary {
        laboratory::destination(Path::new(&path.to_string()))
    }

    #[func]
    fn save_world_file(&mut self, path: GString) -> VarDictionary {
        self.automatic_backup.finish_pending();
        let Some(world) = self.world.as_ref() else {
            return laboratory::outcome(Some("Initialize a world first"));
        };
        let limits = world_file_limits();
        let error = world.save_legacy_file(
            Path::new(&path.to_string()), limits, limits.allocated_bytes,
        ).err();
        file_outcome(error.as_ref())
    }

    #[func]
    fn backup_world_file(&mut self, path: GString) -> VarDictionary {
        self.automatic_backup.finish_pending();
        let Some(world) = self.world.as_ref() else {
            let mut result = laboratory::outcome(Some("Initialize a world first"));
            result.set("replacement_committed", false);
            return result;
        };
        match save_manual_backup(world, Path::new(&path.to_string())) {
            Ok(path) => {
                let mut result = file_outcome(None);
                result.set("backup_path", path.to_str().expect("Backup path was validated as UTF-8"));
                result
            }
            Err(ManualBackupError::Destination(message)) => {
                let mut result = laboratory::outcome(Some(&message));
                result.set("replacement_committed", false);
                result
            }
            Err(ManualBackupError::Save(error)) => file_outcome(Some(&error)),
        }
    }

    #[func]
    fn open_world_file(&mut self, path: GString) -> VarDictionary {
        let error = self.open_world_path(Path::new(&path.to_string())).err();
        self.response(error.as_deref())
    }
}

pub(super) fn file_outcome(error: Option<&WorldFileError>) -> VarDictionary {
    let message = error.map(ToString::to_string);
    let mut result = laboratory::outcome(message.as_deref());
    result.set("replacement_committed", matches!(
        error, Some(WorldFileError::Replace(error)) if error.replacement_committed
    ));
    result
}

#[derive(Debug)]
pub(super) enum ManualBackupError {
    Destination(String),
    Save(WorldFileError),
}

fn backup_destination(current: &Path, time: i64) -> Result<PathBuf, String> {
    let text = current.to_str().ok_or("The world path must be valid UTF-8")?;
    if text.contains('\0') {
        return Err("The world path contains a NUL character".into());
    }
    let name = current.file_name().and_then(|name| name.to_str())
        .ok_or("Choose a world filename before creating a backup")?;
    let has_extension = name.get(name.len().saturating_sub(4)..)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(".bgw"));
    let mut base = if has_extension { &name[..name.len() - 4] } else { name };
    if has_extension {
        if let Some((prefix, stamp)) = base.rsplit_once('@') {
            if stamp.bytes().all(|byte| byte.is_ascii_digit()) {
                base = prefix;
            }
        }
    }
    if base.is_empty() || base == "." || base == ".." {
        return Err("The world filename needs a nonempty base name".into());
    }
    let backup_name = format!("{base}@{time:05}.bgw");
    if backup_name.eq_ignore_ascii_case(name) {
        return Err("Backup would replace the current world file. Use Save As with a different base filename.".into());
    }
    Ok(current.with_file_name(backup_name))
}

pub(super) fn save_manual_backup(world: &World, current: &Path) -> Result<PathBuf, ManualBackupError> {
    let backup = backup_destination(current, world.time()).map_err(ManualBackupError::Destination)?;
    let inspect = |path: &Path| {
        format!("Cannot inspect backup path {}", path.display())
    };
    let current_owner = match std::fs::metadata(current) {
        Ok(metadata) if metadata.is_file() => Some(std::fs::canonicalize(current)
            .map_err(|error| ManualBackupError::Destination(format!("{}: {error}", inspect(current))))?),
        Ok(_) => return Err(ManualBackupError::Destination("The current world path must name a regular file".into())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(ManualBackupError::Destination(format!("{}: {error}", inspect(current)))),
    };
    match std::fs::canonicalize(&backup) {
        Ok(destination) if current_owner.as_ref() == Some(&destination) => {
            return Err(ManualBackupError::Destination(
                "Backup resolves to the current world file. Use Save As with a different base filename.".into(),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(ManualBackupError::Destination(format!("{}: {error}", inspect(&backup)))),
    }
    let limits = world_file_limits();
    world.save_legacy_file(&backup, limits, limits.allocated_bytes)
        .map_err(ManualBackupError::Save)?;
    Ok(backup)
}

impl BgrSimulation {
    pub(super) fn open_world_path(&mut self, path: &Path) -> Result<(), String> {
        self.automatic_backup.finish_pending();
        // Source approval disposes Statistics before file decoding, including failed Open
        self.statistics_session = None;
        let current = self.world.as_ref().ok_or("Initialize a world first")?;
        let limits = world_file_limits();
        let loaded = current.load_legacy_file(path, limits, limits.allocated_bytes)
            .map_err(|error| error.to_string())?;
        // A drawing failure must not publish a root that the frontend cannot accept
        loaded.render_segments().map_err(|error| error.to_string())?;
        loaded.pause();
        self.network_world_ready(&loaded);
        self.living_scene.clear();
        self.feeding_scene.clear();
        self.world = Some(loaded);
        self.tracked = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::atomic::{AtomicU64, Ordering}};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            for _ in 0..128 {
                let path = std::env::temp_dir().join(format!(
                    "bgr-world-ui-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed),
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("Cannot create private directory: {error}"),
                }
            }
            panic!("Private directory names are occupied");
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("Cannot remove owned test directory");
        }
    }

    #[test]
    fn open_publishes_a_new_paused_root_without_rebinding_escaped_owners_or_session() {
        let directory = Directory::new();
        let path = directory.0.join("world.bgw");
        fs::write(&path, include_bytes!("../../core/tests/fixtures/object_stream_codec/world.ser")).unwrap();
        let mut simulation = crate::tests::session();
        simulation.start_session_world(827364, 7000, 7000, 32).unwrap();
        crate::tests::select_first(&mut simulation);
        let current = simulation.world.as_ref().unwrap();
        current.resume();
        let old = current.handle();
        let oxygen = old.oxygen();
        let random = current.random_snapshot();
        let parameters = current.parameters();
        let actor = simulation.selected.clone().unwrap();
        let clipboard = simulation.clipboard.clone();
        let preview = simulation.preview_genome.clone();
        let shown_energy = simulation.inspector.as_ref().unwrap().energy;
        simulation.living_scene.cache_for_test(current);

        simulation.open_world_path(&path).unwrap();
        assert!(simulation.living_scene.is_empty_for_test());

        let loaded = simulation.world.as_ref().unwrap();
        assert!(!loaded.handle().same_owner(&old));
        assert!(loaded.session().same_owner(&old.session()));
        assert_eq!(loaded.random_snapshot(), random);
        assert_eq!(loaded.parameters(), parameters);
        assert!(loaded.paused());
        assert_eq!(old.oxygen(), oxygen);
        assert!(loaded.inspect_handle(&actor).is_none());
        assert_eq!(simulation.selected, Some(actor.clone()));
        assert!(simulation.tracked.is_none() && simulation.statistics_session.is_none());
        assert_eq!(simulation.preview_genome, preview);
        assert_eq!(simulation.inspector.as_ref().unwrap().energy, shown_energy);
        assert_eq!(simulation.clipboard, clipboard);
    }

    #[test]
    fn open_failure_retains_current_root_handles_and_session_state() {
        let directory = Directory::new();
        let path = directory.0.join("bad.bgw");
        fs::write(&path, b"not a Java stream").unwrap();
        let mut simulation = crate::tests::session();
        simulation.start_session_world(827364, 7000, 7000, 32).unwrap();
        crate::tests::select_first(&mut simulation);
        simulation.open_statistics_state().unwrap();
        let current = simulation.world.as_ref().unwrap();
        current.resume();
        let old = current.handle();
        let before = current.clone();
        let random = current.random_snapshot();
        let selected = simulation.selected.clone();
        let clipboard = simulation.clipboard.clone();
        simulation.living_scene.cache_for_test(current);

        assert!(simulation.open_world_path(&path).is_err());
        assert!(!simulation.living_scene.is_empty_for_test());
        assert!(simulation.statistics_session.is_none());
        simulation.open_statistics_state().unwrap();
        assert!(simulation.open_world_path(&directory.0.join("absent.bgw")).is_err());
        assert!(simulation.statistics_session.is_none());

        let current = simulation.world.as_ref().unwrap();
        assert!(current.handle().same_owner(&old));
        assert_eq!(current, &before);
        assert_eq!(current.random_snapshot(), random);
        assert_eq!(simulation.selected, selected);
        assert_eq!(simulation.tracked, selected);
        assert_eq!(simulation.clipboard, clipboard);
        assert!(simulation.preview_genome.is_some());
        assert!(!current.paused());
    }

    #[test]
    fn backup_destination_preserves_source_stamps_without_active_path_aliases() {
        for (name, time, expected) in [
            ("world.bgw", 0, "world@00000.bgw"),
            ("world.BGW", 12, "world@00012.bgw"),
            ("world@00123.bgw", 99, "world@00099.bgw"),
            ("world@.bgw", 12, "world@00012.bgw"),
            ("world@draft.bgw", 12, "world@draft@00012.bgw"),
            ("world.custom", 12, "world.custom@00012.bgw"),
            ("world", 100_000, "world@100000.bgw"),
            ("world.bgw", -1, "world@-0001.bgw"),
            ("world.bgw", i64::MIN, "world@-9223372036854775808.bgw"),
        ] {
            let parent = Path::new("parent.bgw");
            assert_eq!(backup_destination(&parent.join(name), time).unwrap(), parent.join(expected));
        }
        for name in ["", ".", "..", ".bgw", "@00123.bgw", "bad\0.bgw", "world@00000.bgw", "world@00000.BGW"] {
            assert!(backup_destination(Path::new(name), 0).is_err(), "Accepted {name:?}");
        }
    }

    #[test]
    fn manual_backup_uses_game_time_overwrites_same_time_and_preserves_running_world() {
        let directory = Directory::new();
        let current = directory.0.join("world.BGW");
        fs::write(&current, b"protected current world bytes").unwrap();
        let world = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap();
        world.resume();
        let before = world.clone();
        let owner = world.handle();
        let random = world.random_snapshot();
        let first = save_manual_backup(&world, &current).unwrap();
        assert_eq!(first, directory.0.join("world@00000.bgw"));
        assert_eq!(world, before);
        assert!(world.handle().same_owner(&owner));
        assert_eq!(world.random_snapshot(), random);
        assert!(!world.paused());
        let first_bytes = fs::read(&first).unwrap();
        assert_eq!(&first_bytes[..4], &[0xac, 0xed, 0, 5]);
        world.add_carbon_dioxide(7.0);
        assert_eq!(save_manual_backup(&world, &current).unwrap(), first);
        assert_ne!(fs::read(&first).unwrap(), first_bytes);
        let limits = Limits::default();
        let reopened = world.load_legacy_file(&first, limits, limits.allocated_bytes).unwrap();
        assert_eq!(reopened.carbon_dioxide(), world.carbon_dioxide());
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 2);
        for _ in 0..256 { assert!(world.step().unwrap()); }
        assert_eq!(world.tick(), 256);
        assert_eq!(world.time(), 1);
        assert_eq!(save_manual_backup(&world, &current).unwrap(), directory.0.join("world@00001.bgw"));
        assert!(!world.paused());
        assert_eq!(fs::read(&current).unwrap(), b"protected current world bytes");
    }

    #[test]
    fn manual_backup_failures_preserve_active_bytes_and_world() {
        let directory = Directory::new();
        let current = directory.0.join("world.bgw");
        let target = directory.0.join("world@00000.bgw");
        fs::write(&current, b"protected current").unwrap();
        fs::create_dir(&target).unwrap();
        let world = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap();
        let before = world.clone();
        let random = world.random_snapshot();
        let error = save_manual_backup(&world, &current).unwrap_err();
        assert!(matches!(error, ManualBackupError::Save(WorldFileError::Replace(ref error)) if !error.replacement_committed));
        assert!(target.is_dir());
        assert_eq!(fs::read(&current).unwrap(), b"protected current");
        assert_eq!(world, before);
        assert_eq!(world.random_snapshot(), random);
        assert!(world.paused());
        assert!(matches!(save_manual_backup(&world, &directory.0), Err(ManualBackupError::Destination(_))));
        fs::remove_dir(&target).unwrap();
        fs::write(&target, b"protected active timestamp").unwrap();
        assert!(matches!(save_manual_backup(&world, &target), Err(ManualBackupError::Destination(_))));
        assert_eq!(fs::read(&target).unwrap(), b"protected active timestamp");
    }

    #[cfg(unix)]
    #[test]
    fn manual_backup_rejects_current_file_aliases_and_protected_destinations() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let directory = Directory::new();
        let current = directory.0.join("world.bgw");
        let target = directory.0.join("world@00000.bgw");
        fs::write(&target, b"protected target").unwrap();
        symlink(&target, &current).unwrap();
        let world = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap();
        assert!(matches!(save_manual_backup(&world, &current), Err(ManualBackupError::Destination(_))));
        assert_eq!(fs::read(&target).unwrap(), b"protected target");
        fs::remove_file(&current).unwrap();
        fs::write(&current, b"protected current").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o400)).unwrap();
        assert!(matches!(save_manual_backup(&world, &current), Err(ManualBackupError::Save(WorldFileError::Replace(ref error))) if !error.replacement_committed));
        assert_eq!(fs::read(&target).unwrap(), b"protected target");
        assert_eq!(fs::read(&current).unwrap(), b"protected current");
        fs::remove_file(&target).unwrap();
        symlink(&current, &target).unwrap();
        assert!(save_manual_backup(&world, &current).is_err());
        assert_eq!(fs::read(&current).unwrap(), b"protected current");
    }

    #[test]
    fn open_retains_colliding_selection_preview_and_original_counter_subscription() {
        use biogenesis_core::{ColorKind, Gene, Genome, GenomeHandle, GenomeProperties};

        let directory = Directory::new();
        let path = directory.0.join("colliding-world.bgw");
        let mut simulation = crate::tests::session();
        simulation.start_session_world(827364, 1200, 800, 0).unwrap();
        let code = GenomeHandle::from_owned(Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
            GenomeProperties { symmetry: 1, mutation_rate: 0, ..GenomeProperties::default() },
        ));
        simulation.world.as_mut().unwrap().paste_genome_handle(&code, [600, 400]).unwrap().unwrap();
        crate::tests::select_first(&mut simulation);
        let actor = simulation.selected.clone().unwrap();
        let owner = actor.owner_world();
        let before = owner.inspect_handle(&actor).unwrap();
        let bounds = before.state.bounds.get();
        let counters = simulation.inspector.as_ref().unwrap().counters.clone();
        let children_before = counters.snapshot().children;
        let limits = world_file_limits();
        simulation.world.as_ref().unwrap().save_legacy_file(&path, limits, limits.allocated_bytes).unwrap();
        simulation.open_statistics_state().unwrap();
        simulation.open_world_path(&path).unwrap();
        let loaded = simulation.world.as_ref().unwrap();
        let collision = loaded.pick_organism([bounds.x, bounds.y]).unwrap().unwrap();
        assert_eq!(loaded.inspect_handle(&collision).unwrap().id, before.id);
        assert_ne!(collision, actor);
        assert_eq!(simulation.selected, Some(actor.clone()));
        assert!(loaded.inspect_handle(&actor).is_none());
        assert_eq!(simulation.preview_genome, Some(Some(code.clone())));
        assert!(simulation.statistics_session.is_none() && simulation.tracked.is_none());
        assert!(actor.act_in_visible_world(loaded, OrganismAction::Feed).unwrap());
        assert!(actor.act_in_visible_world(loaded, OrganismAction::Reproduce).unwrap());
        let children = owner.inspect_handle(&actor).unwrap().state.total_children;
        assert!(children > children_before);
        assert_eq!(counters.snapshot().children, children);
        assert_eq!(simulation.inspector.as_ref().unwrap().counters.snapshot().children, children);
        assert_eq!(loaded.inspect_handle(&collision).unwrap().state.total_children, before.state.total_children);
        loaded.single_step().unwrap();
        recalculate_inspector(loaded, Some(&actor), &mut simulation.inspector);
        assert_eq!(simulation.inspector.as_ref().unwrap().energy, owner.inspect_handle(&actor).unwrap().state.energy);
        simulation.clipboard = None;
        simulation.copy_preview_state();
        assert_eq!(simulation.clipboard, Some(code));
        simulation.select_position_state([bounds.x, bounds.y], false).unwrap();
        assert_eq!(simulation.selected, Some(collision));
        assert!(simulation.inspector.is_none());
        simulation.release_world_state();
        assert!(simulation.selected.is_none() && simulation.inspector.is_none() && simulation.preview_genome.is_none());
    }

}
