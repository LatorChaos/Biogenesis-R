// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::{AutomaticBackupMode, SettingKey, SettingValue};
use super::recovery_store::{PreparedCheckpoint, RecoveryPolicy, RecoveryStore, SnapshotInfo};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

#[derive(Clone, Copy)]
pub(super) struct Policy {
    enabled: bool,
    delay: i32,
    mode: AutomaticBackupMode,
}

#[derive(Default)]
struct DueLatch {
    active: bool,
    last_attempt_time: Option<i64>,
}
impl DueLatch {
    fn observe(&mut self, enabled: bool, delay: i32, time: i64, mode: AutomaticBackupMode) -> Result<bool, &'static str> {
        if enabled && delay <= 0 {
            *self = Self::default();
            return Err("Automatic backup delay must be positive; no automatic file was written");
        }
        let due = enabled && time > 0 && time % i64::from(delay) == 0;
        if !due { *self = Self::default(); return Ok(false); }
        let already_attempted = match mode {
            AutomaticBackupMode::Adjacent => self.active,
            AutomaticBackupMode::Managed => self.last_attempt_time == Some(time),
        };
        self.active = true;
        if already_attempted { return Ok(false); }
        self.last_attempt_time = Some(time);
        Ok(true)
    }
}

struct SavedFacts { time: i64 }

type WorkerResult = Result<(RecoveryStore, Result<SnapshotInfo, (String, bool)>), String>;
type WorkerInput = (RecoveryStore, PreparedCheckpoint, WorldConfig, WorldParameters, JavaRandom);

#[derive(Default)]
pub(super) struct AutomaticBackup {
    store: Option<RecoveryStore>,
    worker: Option<JoinHandle<WorkerResult>>,
    directory: Option<PathBuf>,
    // The UI owns the filename; this projection derives adjacent names and protects managed files
    pub(super) active_path: Option<PathBuf>,
    latch: DueLatch,
    pub(super) source_pending: bool,
    manual_pending: bool,
    manual_warning: String,
    attempts: i64,
    last_attempt: Option<i64>,
    last_success: Option<SavedFacts>,
    last_path: String,
    error: String,
    recovery_warning: String,
    committed: bool,
    outcome: &'static str,
}

fn bounded(value: impl AsRef<str>) -> String { value.as_ref().chars().take(2048).collect() }

impl AutomaticBackup {
    fn failure(&mut self, error: impl AsRef<str>, committed: bool) {
        self.error = bounded(error);
        self.committed = committed;
        self.outcome = "failed";
    }

    fn success(&mut self, world: &World, path: &Path) {
        self.success_at(world.time(), path);
    }

    fn success_at(&mut self, time: i64, path: &Path) {
        self.last_success = Some(SavedFacts { time });
        self.last_path = bounded(path.to_string_lossy());
        self.error.clear();
        self.committed = false;
        self.outcome = "saved";
    }

    fn ensure_store(&mut self) -> Result<&mut RecoveryStore, String> {
        self.finish_pending();
        if self.store.is_none() {
            let directory = self.directory.as_ref().ok_or("Configure the owned recovery directory first")?;
            let store = RecoveryStore::open(directory, RecoveryPolicy::default()).map_err(|error| error.to_string())?;
            self.manual_pending = store.manual_pending().map_err(|error| error.to_string())?;
            self.store = Some(store);
        }
        Ok(self.store.as_mut().expect("Store was installed"))
    }

    pub(super) fn finish_pending(&mut self) {
        let Some(worker) = self.worker.take() else { return; };
        match worker.join() {
            Ok(Ok((store, result))) => {
                self.store = Some(store);
                match result {
                    Ok(info) => self.success_at(info.time, &info.path),
                    Err((error, committed)) => self.failure(error, committed),
                }
            }
            Ok(Err(error)) => self.failure(error, true),
            Err(_) => self.failure("Recovery worker stopped unexpectedly", true),
        }
    }

    pub(super) fn poll_pending(&mut self) {
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) { self.finish_pending(); }
    }

    fn start_background(&mut self, world: &World, protected: Option<&Path>) -> Result<(), (String, bool)> {
        let prepared = self.ensure_store().and_then(|store| store.prepare_checkpoint(world, protected).map_err(|error| error.to_string()))
            .map_err(|error| (error, false))?;
        let (width, height) = world.dimensions();
        let config = WorldConfig { seed: world.seed(), width, height, initial_attempts: world.initial_attempts() };
        let parameters = world.parameters();
        let random = world.random_snapshot();
        let (sender, receiver) = std::sync::mpsc::sync_channel::<WorkerInput>(1);
        let worker = std::thread::Builder::new().name("bgr-recovery".into()).spawn(move || {
            let (mut store, prepared, config, parameters, random) = receiver.recv().map_err(|error| error.to_string())?;
            let caller = World::detached_validation_context(config, parameters, random);
            let result = store.finish_checkpoint(&caller, prepared)
                .map_err(|error| (error.to_string(), error.replacement_committed()));
            Ok((store, result))
        }).map_err(|error| (error.to_string(), false))?;
        let store = self.store.take().expect("Prepared checkpoint owns its store");
        if let Err(error) = sender.send((store, prepared, config, parameters, random)) {
            self.store = Some(error.0.0);
            let message = if worker.join().is_err() {
                "Recovery worker stopped before accepting the checkpoint"
            } else {
                "Recovery worker could not accept the checkpoint"
            };
            return Err((message.into(), false));
        }
        self.worker = Some(worker);
        self.error.clear();
        self.outcome = "pending";
        Ok(())
    }

    fn resolve_manual(&mut self, world: &World, path: &Path) -> Result<(), String> {
        let result = self.ensure_store()?.checkpoint(world, Some(path))
            .map_err(|error| error.to_string())?;
        self.ensure_store()?.set_manual_pending(false).map_err(|error| error.to_string())?;
        self.manual_pending = false;
        self.manual_warning.clear();
        self.success(world, &result.path);
        Ok(())
    }

    pub(super) fn observe(&mut self, world: &World, policy: Policy) {
        self.observe_with_mode(world, policy, false);
    }

    pub(super) fn observe_background(&mut self, world: &World, policy: Policy) {
        self.observe_with_mode(world, policy, true);
    }

    fn observe_with_mode(&mut self, world: &World, policy: Policy, background: bool) {
        self.poll_pending();
        let attempt = match self.latch.observe(policy.enabled, policy.delay, world.time(), policy.mode) {
            Ok(attempt) => attempt,
            Err(error) => { self.source_pending = false; self.failure(error, false); return; }
        };
        if !policy.enabled || world.time() <= 0 || world.time() % i64::from(policy.delay) != 0 {
            self.source_pending = false;
        }
        if policy.mode != AutomaticBackupMode::Adjacent { self.source_pending = false; }
        if !attempt { return; }
        self.attempts = self.attempts.saturating_add(1);
        self.last_attempt = Some(world.time());
        self.committed = false;
        match policy.mode {
            AutomaticBackupMode::Adjacent => {
                self.finish_pending();
                let Some(path) = self.active_path.clone() else {
                    self.source_pending = true;
                    self.outcome = "save_as_required";
                    self.error.clear();
                    return;
                };
                match super::world_file::save_manual_backup(world, &path) {
                    Ok(path) => self.success(world, &path),
                    Err(super::world_file::ManualBackupError::Destination(error)) => self.failure(error, false),
                    Err(super::world_file::ManualBackupError::Save(error)) => {
                        let committed = matches!(&error, biogenesis_core::WorldFileError::Replace(error) if error.replacement_committed);
                        self.failure(error.to_string(), committed);
                    }
                }
            }
            AutomaticBackupMode::Managed => {
                let protected = self.active_path.clone();
                if background {
                    if let Err((error, committed)) = self.start_background(world, protected.as_deref()) {
                        self.failure(error, committed);
                    }
                    return;
                }
                let result = match self.ensure_store() {
                    Ok(store) => store.checkpoint(world, protected.as_deref())
                        .map_err(|error| (error.to_string(), error.replacement_committed())),
                    Err(error) => Err((error, false)),
                };
                match result {
                    Ok(info) => self.success(world, &info.path),
                    Err((error, committed)) => self.failure(error, committed),
                }
            }
        }
    }

    pub(super) fn end_world(&mut self) {
        self.finish_pending();
        self.source_pending = false;
        self.latch = DueLatch::default();
        self.active_path = None;
        self.store = None;
    }
}

impl Drop for AutomaticBackup {
    fn drop(&mut self) { self.finish_pending(); }
}

impl BgrSimulation {
    pub(super) fn automatic_policy(&self) -> Policy {
        let state = self.settings.as_ref();
        Policy {
            enabled: state.is_some_and(|state| state.get(SettingKey::AutoBackup) == SettingValue::Boolean(true)),
            delay: state.map_or(100, |state| match state.get(SettingKey::BackupDelay) { SettingValue::Int(value) => value, _ => unreachable!() }),
            mode: state.map_or(AutomaticBackupMode::Adjacent, |state| state.automatic_backup_mode()),
        }
    }

    pub(super) fn automatic_backup_check(&mut self) {
        if self.settings_applying { return; }
        let policy = self.automatic_policy();
        if let Some(world) = self.world.as_ref() { self.automatic_backup.observe(world, policy); }
    }

    pub(super) fn automatic_status(&self) -> VarDictionary {
        let policy = self.automatic_policy();
        let state = &self.automatic_backup;
        let mut output = VarDictionary::new();
        output.set("enabled", policy.enabled);
        output.set("mode", policy.mode.native_value());
        output.set("delay", policy.delay);
        output.set("configured", state.directory.is_some());
        output.set("directory", state.directory.as_ref().map_or(String::new(), |path| path.to_string_lossy().into_owned()));
        output.set("source_save_pending", state.source_pending);
        output.set("checkpoint_pending", state.worker.is_some());
        output.set("manual_checkpoint_pending", state.manual_pending);
        output.set("manual_checkpoint_warning", state.manual_warning.as_str());
        output.set("attempts", state.attempts);
        output.set("attempted", state.last_attempt.is_some());
        output.set("last_attempt_time", state.last_attempt.unwrap_or(0));
        output.set("has_success", state.last_success.is_some());
        let mut last = VarDictionary::new();
        if let Some(facts) = state.last_success.as_ref() {
            last.set("path", state.last_path.as_str());
            last.set("time", facts.time);
        }
        output.set("last", &last);
        output.set("error", state.error.as_str());
        output.set("recovery_warning", state.recovery_warning.as_str());
        output.set("replacement_committed", state.committed);
        output.set("outcome", state.outcome);
        output.set("maximum_bytes", 1_i64 << 30);
        output.set("maximum_snapshots", 8_i64);
        output
    }
}

#[godot_api(secondary)]
impl BgrSimulation {
    #[func]
    fn configure_automatic_backups(&mut self, user_directory: GString, active_path: GString) -> VarDictionary {
        if self.settings_applying { return self.automatic_status(); }
        let provided = PathBuf::from(user_directory.to_string());
        let owned = PathBuf::from(godot::classes::Os::singleton().get_user_data_dir().to_string());
        if !provided.is_absolute() || provided != owned {
            self.automatic_backup.failure("Recovery storage must use this application's owned user-data directory", false);
            return self.automatic_status();
        }
        let directory = provided.join("managed-recovery");
        if self.automatic_backup.directory.as_ref() != Some(&directory) {
            self.automatic_backup.finish_pending();
            self.automatic_backup.store = None;
        }
        self.automatic_backup.directory = Some(directory.clone());
        if directory.exists() {
            if let Err(error) = self.automatic_backup.ensure_store() {
                self.automatic_backup.manual_pending = true;
                self.automatic_backup.manual_warning = bounded(error);
            }
        }
        self.refresh_automatic_backups(active_path)
    }

    #[func]
    fn refresh_automatic_backups(&mut self, active_path: GString) -> VarDictionary {
        if !self.settings_applying {
            let text = active_path.to_string();
            self.automatic_backup.active_path = if text.is_empty() { None } else { Some(PathBuf::from(text)) };
            self.automatic_backup_check();
        }
        self.automatic_status()
    }

    #[func]
    fn automatic_backup_status(&mut self) -> VarDictionary {
        self.automatic_backup.poll_pending();
        self.automatic_status()
    }

    #[func]
    fn complete_automatic_backup(&mut self, succeeded: bool, message: GString, committed: bool, active_path: GString) -> VarDictionary {
        if !self.automatic_backup.source_pending { return self.automatic_status(); }
        self.automatic_backup.source_pending = false;
        let path = PathBuf::from(active_path.to_string());
        if succeeded {
            self.automatic_backup.active_path = Some(path.clone());
            if let Some(world) = self.world.as_ref() { self.automatic_backup.success(world, &path); }
        } else {
            let message = message.to_string();
            self.automatic_backup.failure(if message.is_empty() { "Automatic Backup Save As cancelled" } else { &message }, committed);
            if message.is_empty() { self.automatic_backup.outcome = "cancelled"; }
        }
        self.automatic_status()
    }

    #[func]
    fn save_manual_world_file(&mut self, path: GString, file_only_acknowledged: bool) -> VarDictionary {
        self.automatic_backup.finish_pending();
        let policy = self.automatic_policy();
        let managed = policy.enabled && policy.mode == AutomaticBackupMode::Managed;
        let Some(world) = self.world.as_ref() else { return laboratory::outcome(Some("Initialize a world first")); };
        let path = PathBuf::from(path.to_string());
        if managed && !file_only_acknowledged {
            let admission = self.automatic_backup.ensure_store().and_then(|store| {
                store.set_manual_pending(true).map_err(|error| error.to_string())
            });
            if let Err(error) = admission {
                self.automatic_backup.manual_pending = true;
                self.automatic_backup.manual_warning = bounded(&error);
                let mut output = laboratory::outcome(Some(&error));
                output.set("file_only_acknowledgment_required", true);
                output.set("replacement_committed", false);
                return output;
            }
            self.automatic_backup.manual_pending = true;
        }
        let limits = super::world_file::world_file_limits();
        let error = world.save_legacy_file(&path, limits, limits.allocated_bytes).err();
        let mut output = super::world_file::file_outcome(error.as_ref());
        if error.is_none() && managed && !file_only_acknowledged {
            if let Err(error) = self.automatic_backup.resolve_manual(world, &path) {
                self.automatic_backup.manual_warning = bounded(error);
            }
            // A manual checkpoint already covers this scheduled boundary
            let _ = self.automatic_backup.latch.observe(policy.enabled, policy.delay, world.time(), policy.mode);
        }
        if file_only_acknowledged {
            self.automatic_backup.manual_warning = "File-only Save: latest-manual automatic recovery protection was explicitly waived".into();
        }
        output.set("file_only_acknowledgment_required", false);
        output.set("file_only_waived", file_only_acknowledged);
        output.set("manual_checkpoint_pending", self.automatic_backup.manual_pending);
        output
    }

    #[func]
    fn open_manual_world_file(&mut self, path: GString) -> VarDictionary {
        let path = PathBuf::from(path.to_string());
        if let Err(error) = self.open_world_path(&path) { return self.response(Some(&error)); }
        if self.automatic_backup.manual_pending {
            if let Some(world) = self.world.as_ref() {
                if let Err(error) = self.automatic_backup.resolve_manual(world, &path) {
                    self.automatic_backup.manual_warning = bounded(error);
                }
            }
        }
        self.response(None)
    }

    #[func]
    fn recover_startup_world(&mut self) -> VarDictionary {
        let admission = self.automatic_backup.ensure_store().and_then(|store| {
            if store.manual_pending().map_err(|error| error.to_string())? {
                Err("A manual Save has no confirmed matching checkpoint. Open the manual file or explicitly choose an older managed checkpoint.".into())
            } else { Ok(()) }
        });
        if let Err(error) = admission {
            self.automatic_backup.manual_pending = true;
            self.automatic_backup.manual_warning = bounded(&error);
            return self.response(Some(&error));
        }
        self.recover_latest_world()
    }

    #[func]
    fn recover_acknowledged_world(&mut self) -> VarDictionary {
        match self.recover_latest_state() {
            Ok(path) => {
                if let Err(error) = self.automatic_backup.ensure_store().and_then(|store| {
                    store.set_manual_pending(false).map_err(|error| error.to_string())
                }) {
                    self.automatic_backup.manual_warning = bounded(error);
                } else {
                    self.automatic_backup.manual_pending = false;
                    self.automatic_backup.manual_warning.clear();
                }
                let mut output = self.response(None);
                output.set("recovered_path", path.to_string_lossy().as_ref());
                output
            }
            Err(error) => self.response(Some(&error)),
        }
    }

    #[func]
    fn recover_latest_world(&mut self) -> VarDictionary {
        match self.recover_latest_state() {
            Ok(path) => {
                let mut output = self.response(None);
                output.set("recovered_path", path.to_string_lossy().as_ref());
                output.set("recovery_warning", self.automatic_backup.recovery_warning.as_str());
                output
            }
            Err(error) => self.response(Some(&error)),
        }
    }
}

impl BgrSimulation {
    fn recover_latest_state(&mut self) -> Result<PathBuf, String> {
        if self.settings_applying || self.automatic_backup.source_pending {
            return Err("Finish the pending settings or Source Backup action before recovery".into());
        }
        let Some(current) = self.world.as_ref() else { return Err("Initialize a world first".into()); };
        let loaded = match self.automatic_backup.ensure_store() {
            Ok(store) => store.recover_latest(current).map_err(|error| error.to_string()),
            Err(error) => Err(error),
        };
        let world = match loaded {
            Ok(Some(world)) => world,
            Ok(None) => return Err("No readable managed recovery snapshot is available".into()),
            Err(error) => return Err(error),
        };
        if let Err(error) = world.render_segments() { return Err(error.to_string()); }
        let store = self.automatic_backup.store.as_ref().expect("Successful recovery owns a store");
        let info = store.last_recovered().expect("Successful recovery reports its source path");
        let path = info.path.clone();
        self.automatic_backup.recovery_warning = store.rejections().last().map_or(String::new(), |rejection| {
            bounded(format!("Recovered {} at game time {}. Skipped {}: {}", path.display(), info.time,
                rejection.path.display(), rejection.details))
        });
        world.pause();
        self.living_scene.clear();
        self.feeding_scene.clear();
        self.world = Some(world);
        self.selected = None;
        self.tracked = None;
        self.preview_genome = None;
        self.statistics_session = None;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn empty_world() -> World {
        World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap()
    }

    #[test]
    fn automatic_due_latch_preserves_source_failure_and_contiguous_multiple_rules() {
        let mut latch = DueLatch::default();
        assert!(!latch.observe(true, 2, 0, AutomaticBackupMode::Adjacent).unwrap());
        assert!(latch.observe(true, 2, 2, AutomaticBackupMode::Adjacent).unwrap());
        // Completing, failing, or cancelling an attempt does not reset this latch
        assert!(!latch.observe(true, 2, 2, AutomaticBackupMode::Adjacent).unwrap());
        assert!(!latch.observe(true, 2, 3, AutomaticBackupMode::Adjacent).unwrap());
        assert!(latch.observe(true, 2, 4, AutomaticBackupMode::Adjacent).unwrap());
        assert!(!latch.observe(false, 2, 4, AutomaticBackupMode::Adjacent).unwrap());
        assert!(latch.observe(true, 2, 4, AutomaticBackupMode::Adjacent).unwrap());
        let mut every_time = DueLatch::default();
        assert!(every_time.observe(true, 1, 1, AutomaticBackupMode::Adjacent).unwrap());
        assert!(!every_time.observe(true, 1, 2, AutomaticBackupMode::Adjacent).unwrap());
        assert!(!every_time.observe(true, 1, 3, AutomaticBackupMode::Adjacent).unwrap());
    }

    #[test]
    fn automatic_due_checks_actual_game_time_inside_bulk_advancement() {
        let mut world = empty_world();
        let mut latch = DueLatch::default();
        let mut saved = Vec::new();
        world.resume();
        advance_world(&mut world, 4096, || false, |world| {
            if latch.observe(true, 2, world.time(), AutomaticBackupMode::Adjacent).unwrap() {
                saved.push((world.tick(), world.time()));
            }
        }).unwrap();
        assert_eq!(saved, vec![(512, 2), (1024, 4), (1536, 6), (2048, 8),
            (2560, 10), (3072, 12), (3584, 14), (4096, 16)]);
    }

    #[test]
    fn automatic_unnamed_source_stops_at_exact_boundary_until_decision() {
        let mut world = empty_world();
        let mut state = AutomaticBackup::default();
        let policy = Policy { enabled: true, delay: 2, mode: AutomaticBackupMode::Adjacent };
        let barrier = Cell::new(false);
        world.resume();
        advance_world(&mut world, 4096, || barrier.get(), |world| {
            state.observe(world, policy);
            barrier.set(state.source_pending);
        }).unwrap();
        assert_eq!((world.tick(), world.time(), state.attempts), (512, 2, 1));
        assert!(state.source_pending);
        state.source_pending = false;
        state.failure("cancelled", false);
        state.observe(&world, policy);
        assert!(!state.source_pending);
        assert_eq!(state.attempts, 1);
        barrier.set(false);
        advance_world(&mut world, 4096, || barrier.get(), |world| {
            state.observe(world, policy);
            barrier.set(state.source_pending);
        }).unwrap();
        assert_eq!((world.tick(), world.time(), state.attempts), (1024, 4, 2));
    }

    #[test]
    fn failed_recovery_preserves_running_root_owners_and_random_stream() {
        let mut simulation = crate::tests::session();
        simulation.start_session_world(827364, 1200, 800, 0).unwrap();
        let world = simulation.world.as_ref().unwrap();
        world.resume();
        let owner = world.handle();
        let random = world.random_snapshot();
        let drawing = world.render_segments().unwrap();
        assert!(simulation.recover_latest_state().is_err());
        let world = simulation.world.as_mut().unwrap();
        assert!(world.handle().same_owner(&owner));
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(world.render_segments().unwrap(), drawing);
        assert!(!world.paused());
        assert!(world.step().unwrap());
    }

    #[test]
    fn automatic_persistence_failure_and_invalid_delay_do_not_fail_biology() {
        let mut world = empty_world();
        let mut state = AutomaticBackup::default();
        let policy = Policy { enabled: true, delay: 2, mode: AutomaticBackupMode::Managed };
        world.resume();
        advance_world(&mut world, 1024, || false, |world| state.observe(world, policy)).unwrap();
        assert_eq!((world.tick(), world.time(), state.attempts), (1024, 4, 2));
        assert!(!world.paused() && !state.source_pending && !state.error.is_empty());
        assert!(state.last_success.is_none());
        for delay in [0, -1, i32::MIN] {
            state.observe(&world, Policy { delay, ..policy });
            assert!(state.error.contains("positive"));
            assert_eq!(state.attempts, 2);
        }
    }

    #[test]
    fn managed_delay_one_attempts_each_world_time() {
        let mut world = empty_world();
        let mut state = AutomaticBackup::default();
        let policy = Policy { enabled: true, delay: 1, mode: AutomaticBackupMode::Managed };
        world.resume();
        advance_world(&mut world, 1024, || false, |world| state.observe(world, policy)).unwrap();
        assert_eq!((world.tick(), world.time(), state.attempts, state.last_attempt), (1024, 4, 4, Some(4)));
        assert!(!world.paused() && !state.source_pending);
    }

    #[test]
    fn managed_due_latch_deduplicates_the_same_time() {
        let mut latch = DueLatch::default();
        assert!(latch.observe(true, 1, 1, AutomaticBackupMode::Managed).unwrap());
        assert!(!latch.observe(true, 1, 1, AutomaticBackupMode::Managed).unwrap());
        assert!(latch.observe(true, 1, 2, AutomaticBackupMode::Managed).unwrap());
        assert!(!latch.observe(true, 1, 2, AutomaticBackupMode::Adjacent).unwrap());
        assert!(!latch.observe(false, 1, 2, AutomaticBackupMode::Managed).unwrap());
        assert!(latch.observe(true, 1, 2, AutomaticBackupMode::Managed).unwrap());
        assert!(latch.observe(true, 0, 2, AutomaticBackupMode::Managed).is_err());
        assert!(latch.observe(true, 1, 2, AutomaticBackupMode::Managed).unwrap());
    }

    #[test]
    fn managed_delay_one_writes_distinct_recovery_points() {
        struct TemporaryDirectory(PathBuf);
        impl Drop for TemporaryDirectory {
            fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
        }
        let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("bgr-managed-delay-{}-{unique}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        let directory = TemporaryDirectory(path);
        let mut world = empty_world();
        let mut state = AutomaticBackup::default();
        state.directory = Some(directory.0.join("managed-recovery"));
        let policy = Policy { enabled: true, delay: 1, mode: AutomaticBackupMode::Managed };
        world.resume();
        advance_world(&mut world, 768, || false, |world| state.observe(world, policy)).unwrap();
        assert_eq!((world.time(), state.attempts, state.last_success.as_ref().map(|saved| saved.time)), (3, 3, Some(3)));
        assert_eq!(state.outcome, "saved");
        let count = std::fs::read_dir(directory.0.join("managed-recovery")).unwrap()
            .filter_map(Result::ok).filter(|entry| entry.file_name().to_string_lossy().starts_with("recovery-")).count();
        assert_eq!(count, 3);
    }

    #[test]
    fn background_checkpoint_preserves_captured_world_after_later_steps() {
        let root = std::env::temp_dir().join(format!("bgr-background-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&root).unwrap();
        let mut world = empty_world();
        let mut state = AutomaticBackup::default();
        state.directory = Some(root.join("recovery"));
        let policy = Policy { enabled: true, delay: 1, mode: AutomaticBackupMode::Managed };
        world.resume();
        advance_world(&mut world, 256, || false, |_| {}).unwrap();
        let limits = super::super::world_file::world_file_limits();
        let expected = world.encode_legacy_file_bytes(limits, limits.allocated_bytes).unwrap();
        state.observe_background(&world, policy);
        assert_eq!(state.outcome, "pending");
        advance_world(&mut world, 1, || false, |_| {}).unwrap();
        state.finish_pending();
        assert_eq!(state.outcome, "saved", "{}", state.error);
        assert_eq!(state.last_success.as_ref().unwrap().time, 1);
        assert_eq!(std::fs::read(&state.last_path).unwrap(), expected);
        drop(state);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn background_work_does_not_block_unrelated_ticks() {
        let root = std::env::temp_dir().join(format!("bgr-gated-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&root).unwrap();
        let mut state = AutomaticBackup::default();
        state.directory = Some(root.join("recovery"));
        state.ensure_store().unwrap();
        let store = state.store.take().unwrap();
        let (release, gate) = std::sync::mpsc::channel::<()>();
        state.worker = Some(std::thread::spawn(move || {
            let result = gate.recv_timeout(std::time::Duration::from_secs(3));
            Ok((store, Err((if result.is_ok() { "released" } else { "blocked" }.into(), false))))
        }));
        state.outcome = "pending";
        let mut world = empty_world();
        world.resume();
        advance_world(&mut world, 1, || false, |world| {
            state.observe_background(world, Policy { enabled: false, delay: 1, mode: AutomaticBackupMode::Managed });
        }).unwrap();
        assert_eq!(world.tick(), 1);
        assert_eq!(state.outcome, "pending");
        release.send(()).unwrap();
        state.finish_pending();
        assert_eq!(state.error, "released");
        drop(state);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn background_checkpoints_keep_every_due_boundary() {
        let root = std::env::temp_dir().join(format!("bgr-boundaries-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&root).unwrap();
        let mut state = AutomaticBackup::default();
        state.directory = Some(root.join("recovery"));
        let mut world = empty_world();
        world.resume();
        advance_world(&mut world, 768, || false, |world| {
            state.observe_background(world, Policy { enabled: true, delay: 1, mode: AutomaticBackupMode::Managed });
        }).unwrap();
        state.finish_pending();
        assert_eq!((state.attempts, state.last_success.as_ref().map(|s| s.time)), (3, Some(3)));
        assert_eq!(state.outcome, "saved", "{}", state.error);
        let count = std::fs::read_dir(root.join("recovery")).unwrap()
            .filter_map(Result::ok).filter(|e| e.file_name().to_string_lossy().starts_with("recovery-")).count();
        assert_eq!(count, 3);
        drop(state);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn adjacent_backup_waits_for_prior_managed_worker() {
        let root = std::env::temp_dir().join(format!("bgr-switch-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&root).unwrap();
        let mut state = AutomaticBackup::default();
        state.directory = Some(root.join("recovery"));
        state.ensure_store().unwrap();
        let store = state.store.take().unwrap();
        let (release, gate) = std::sync::mpsc::channel::<()>();
        let (done, observed) = std::sync::mpsc::channel::<()>();
        state.worker = Some(std::thread::spawn(move || {
            gate.recv().unwrap();
            Ok((store, Err(("fixture complete".into(), false))))
        }));
        let coordinator = std::thread::spawn(move || {
            let returned_early = observed.recv_timeout(std::time::Duration::from_millis(100)).is_ok();
            release.send(()).unwrap();
            if !returned_early { observed.recv_timeout(std::time::Duration::from_secs(3)).unwrap(); }
            returned_early
        });
        let mut world = empty_world();
        world.resume();
        advance_world(&mut world, 256, || false, |_| {}).unwrap();
        state.observe_background(&world, Policy { enabled: true, delay: 1, mode: AutomaticBackupMode::Adjacent });
        done.send(()).unwrap();
        assert!(!coordinator.join().unwrap(), "Adjacent backup started before managed storage was released");
        assert!(state.worker.is_none() && state.source_pending);
        drop(state);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_worker_reports_uncertain_commit_and_releases_state() {
        let mut state = AutomaticBackup::default();
        state.worker = Some(std::thread::spawn(|| -> WorkerResult { panic!("fixture worker failure") }));
        state.finish_pending();
        assert_eq!(state.outcome, "failed");
        assert!(state.committed);
        assert!(state.error.contains("stopped unexpectedly"));
        assert!(state.worker.is_none());
    }
}
