// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{JavaRandom, WorldConfig, WorldParameters};
use std::{ffi::OsString, sync::atomic::{AtomicU64, Ordering}};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const WORLD_BYTES: &[u8] = include_bytes!("../tests/fixtures/object_stream_codec/world.ser");

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        for _ in 0..128 {
            let path = std::env::temp_dir().join(format!(
                "bgr-world-file-{}-{}", std::process::id(), TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("Cannot create isolated test directory: {error}"),
            }
        }
        panic!("Test directory names are occupied");
    }
    fn contents(&self) -> Vec<OsString> {
        let mut names: Vec<_> = fs::read_dir(&self.0).unwrap()
            .map(|item| item.unwrap().file_name()).collect();
        names.sort();
        names
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("Cannot remove owned test directory");
    }
}

fn caller() -> World {
    World::with_random(
        WorldConfig { initial_attempts: 0, ..WorldConfig::default() },
        WorldParameters::default(), JavaRandom::new(7224),
    ).unwrap()
}

fn source_world(caller: &World) -> World {
    let limits = Limits::default();
    let document = StreamReader::new(WORLD_BYTES, limits).unwrap().read_document().unwrap();
    caller.decode_legacy_document(&document, limits, limits.allocated_bytes).unwrap()
}

#[test]
fn real_save_and_open_preserve_bytes_and_leave_old_roots_and_session_unchanged() {
    let directory = TestDirectory::new();
    let path = directory.0.join("world.bgw");
    fs::write(&path, b"previous file").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    }
    let caller = caller();
    let caller_before = caller.clone();
    let old_handle = caller.handle();
    let old_statistics = caller.open_statistics();
    let original = source_world(&caller);
    let original_before = original.clone();
    let source_handle = original.handle();
    let random = caller.random_snapshot();
    let parameters = caller.parameters();
    let limits = Limits::default();
    let expected = original.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap();
    original.save_legacy_file(&path, limits, limits.allocated_bytes).unwrap();
    assert_eq!(fs::read(&path).unwrap(), expected);
    let loaded = caller.load_legacy_file(&path, limits, limits.allocated_bytes).unwrap();
    assert_eq!(loaded.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap(), expected);
    assert!(loaded.session().same_owner(&caller.session()));
    assert!(!loaded.handle().same_owner(&old_handle));
    assert!(!loaded.handle().same_owner(&source_handle));
    assert_eq!(caller, caller_before);
    assert_eq!(original, original_before);
    assert_eq!(caller.open_statistics(), old_statistics);
    assert_eq!(caller.random_snapshot(), random);
    assert_eq!(caller.parameters(), parameters);
    assert_eq!(directory.contents(), [OsString::from("world.bgw")]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o640);
    }
}

#[test]
fn rejected_streams_leave_the_old_root_handles_and_rng_usable() {
    let directory = TestDirectory::new();
    let path = directory.0.join("world.bgw");
    let caller = caller();
    let before = caller.clone();
    let handle = caller.handle();
    let statistics = caller.open_statistics();
    let random = caller.random_snapshot();
    let limits = Limits::default();
    for (bytes, typed_failure) in [
        (b"bad".to_vec(), false),
        (WORLD_BYTES[..WORLD_BYTES.len() - 1].to_vec(), false),
        (include_bytes!("../tests/fixtures/object_stream_codec/gene.ser").to_vec(), true),
        ([WORLD_BYTES, &[0x70]].concat(), true),
        ([WORLD_BYTES, &[0x00]].concat(), false),
    ] {
        fs::write(&path, bytes).unwrap();
        let result = caller.load_legacy_file(&path, limits, limits.allocated_bytes);
        if typed_failure { assert!(matches!(result, Err(WorldFileError::Decode { .. }))); }
        else { assert!(matches!(result, Err(WorldFileError::Stream(_)))); }
        assert!(caller.handle().same_owner(&handle));
        assert_eq!(caller, before);
        assert_eq!(caller.open_statistics(), statistics);
        assert_eq!(caller.random_snapshot(), random);
    }
}

#[test]
fn input_limit_is_exact_and_checked_before_allocating_an_oversized_file() {
    let directory = TestDirectory::new();
    let path = directory.0.join("world.bgw");
    fs::write(&path, WORLD_BYTES).unwrap();
    let caller = caller();
    let limits = Limits { input_bytes: WORLD_BYTES.len(), ..Limits::default() };
    assert!(caller.load_legacy_file(&path, limits, limits.allocated_bytes).is_ok());
    let limits = Limits { input_bytes: WORLD_BYTES.len() - 1, ..limits };
    assert!(matches!(caller.load_legacy_file(&path, limits, limits.allocated_bytes),
        Err(WorldFileError::Stream(StreamError { kind: ErrorKind::Limit(LimitKind::Input), .. }))));
    fs::File::create(&path).unwrap().set_len(WORLD_BYTES.len() as u64 + 10_000).unwrap();
    assert!(matches!(caller.load_legacy_file(&path, limits, limits.allocated_bytes),
        Err(WorldFileError::Stream(StreamError { kind: ErrorKind::Limit(LimitKind::Input), .. }))));
}

#[test]
fn input_parser_and_hydration_share_one_allocation_and_work_budget() {
    let directory = TestDirectory::new();
    let path = directory.0.join("world.bgw");
    fs::write(&path, include_bytes!("../tests/fixtures/world_hydration/actor_closure-call_0.ser")).unwrap();
    let caller = caller();
    let before = caller.clone();
    let limits = Limits::default();
    let bytes = read_world_bytes(&path, limits.input_bytes, limits.allocated_bytes, limits.work).unwrap();
    let (document, allocated, work) = StreamReader::new(&bytes, limits).unwrap()
        .read_document_with_usage().unwrap();
    assert!(caller.decode_legacy_document(&document, limits, limits.allocated_bytes).is_ok());
    assert!(matches!(caller.decode_legacy_document(&document, limits, bytes.capacity()),
        Err(crate::world::legacy::WorldDecodeError::Graph(error))
            if error.kind == crate::legacy_graph::GraphErrorKind::Budget));
    let allocation = bytes.capacity().checked_add(allocated).unwrap();
    assert!(matches!(caller.load_legacy_file(&path, limits, allocation), Err(WorldFileError::Decode { .. })));
    let limited = Limits { work: bytes.len().checked_add(work).unwrap(), ..limits };
    assert!(matches!(caller.load_legacy_file(&path, limited, limited.allocated_bytes),
        Err(WorldFileError::Decode { .. })));
    assert!(matches!(caller.load_legacy_file(&path, limits, 0),
        Err(WorldFileError::Stream(StreamError { kind: ErrorKind::Limit(LimitKind::Allocation), .. }))));
    let limited = Limits { work: 0, ..limits };
    assert!(matches!(caller.load_legacy_file(&path, limited, limited.allocated_bytes),
        Err(WorldFileError::Stream(StreamError { kind: ErrorKind::Limit(LimitKind::Work), .. }))));
    assert_eq!(caller, before);
}

#[test]
fn encoding_failures_preserve_destination_bytes_and_directory() {
    let directory = TestDirectory::new();
    let path = directory.0.join("world.bgw");
    fs::write(&path, b"previous file").unwrap();
    let original = source_world(&caller());
    let before = original.clone();
    let random = original.random_snapshot();
    for (limits, maximum, stream_failure) in [
        (Limits::default(), 0, false),
        (Limits { output_bytes: 0, ..Limits::default() }, usize::MAX, true),
        (Limits { work: 0, ..Limits::default() }, usize::MAX, false),
    ] {
        let result = original.save_legacy_file(&path, limits, maximum);
        if stream_failure {
            assert!(matches!(result, Err(WorldFileError::Stream(StreamError {
                kind: ErrorKind::Limit(LimitKind::Output), ..
            }))));
        } else { assert!(matches!(result, Err(WorldFileError::Encode { .. }))); }
        assert_eq!(fs::read(&path).unwrap(), b"previous file");
        assert_eq!(directory.contents(), [OsString::from("world.bgw")]);
        assert_eq!(original, before);
        assert_eq!(original.random_snapshot(), random);
    }
}

#[test]
fn missing_parent_and_nonregular_paths_return_io_errors_without_changes() {
    let directory = TestDirectory::new();
    let path = directory.0.join("missing/world.bgw");
    let caller = caller();
    let original = source_world(&caller);
    let limits = Limits::default();
    assert!(matches!(caller.load_legacy_file(&path, limits, limits.allocated_bytes),
        Err(WorldFileError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound));
    assert!(matches!(original.save_legacy_file(&path, limits, limits.allocated_bytes),
        Err(WorldFileError::Replace(FileReplaceError { replacement_committed: false, .. }))));
    assert!(directory.contents().is_empty());
    assert!(matches!(caller.load_legacy_file(&directory.0, limits, limits.allocated_bytes),
        Err(WorldFileError::Io { source, .. }) if source.kind() == io::ErrorKind::InvalidInput));
    let keep = directory.0.join("keep");
    fs::write(&keep, b"previous file").unwrap();
    assert!(matches!(original.save_legacy_file(&directory.0, limits, limits.allocated_bytes),
        Err(WorldFileError::Replace(FileReplaceError { replacement_committed: false, .. }))));
    assert_eq!(fs::read(&keep).unwrap(), b"previous file");
}

#[cfg(unix)]
#[test]
fn symlink_and_readonly_save_destinations_are_preserved() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let directory = TestDirectory::new();
    let target = directory.0.join("keep.bgw");
    fs::write(&target, b"previous file").unwrap();
    let link = directory.0.join("link.bgw");
    symlink(&target, &link).unwrap();
    let original = source_world(&caller());
    let limits = Limits::default();
    assert!(matches!(original.save_legacy_file(&link, limits, limits.allocated_bytes),
        Err(WorldFileError::Replace(FileReplaceError { replacement_committed: false, .. }))));
    assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
    fs::set_permissions(&target, fs::Permissions::from_mode(0o400)).unwrap();
    assert!(matches!(original.save_legacy_file(&target, limits, limits.allocated_bytes),
        Err(WorldFileError::Replace(FileReplaceError { replacement_committed: false, .. }))));
    assert_eq!(fs::read(&target).unwrap(), b"previous file");
    assert_eq!(directory.contents(), [OsString::from("keep.bgw"), OsString::from("link.bgw")]);
}

#[test]
fn file_load_releases_owned_input_before_hydrating_large_text() {
    let directory = TestDirectory::new();
    let path = directory.0.join("world.bgw");
    let caller = caller();
    let before = caller.clone();
    let handle = caller.handle();
    let random = caller.random_snapshot();
    let generous = Limits::default();
    let document = StreamReader::new(
        include_bytes!("../tests/fixtures/world_hydration/actor_closure-call_0.ser"), generous,
    ).unwrap().read_document().unwrap();
    let original = caller.decode_legacy_document(&document, generous, generous.allocated_bytes).unwrap();
    let text = crate::JavaText::from_units(vec![u16::from(b'A'); 256 * 1024]);
    original.organisms.borrow()[0].borrow_mut().state.network_position = Some(text.clone());
    original.save_legacy_file(&path, generous, generous.allocated_bytes).unwrap();
    let bounded = Limits { handles: 4096, allocated_bytes: 2560 * 1024, ..generous };
    let loaded = caller.load_legacy_file(&path, bounded, bounded.allocated_bytes).unwrap();
    let restored = loaded.organisms.borrow()[0].borrow().state.network_position.clone().unwrap();
    assert_eq!(restored.units(), text.units());
    assert!(loaded.session().same_owner(&caller.session()));
    assert!(!loaded.handle().same_owner(&handle));
    assert!(caller.handle().same_owner(&handle));
    assert_eq!(caller, before);
    assert_eq!(caller.random_snapshot(), random);
}

#[test]
fn public_file_encoding_matches_save_bytes_and_error_categories_without_mutation() {
    let directory = TestDirectory::new();
    let path = directory.0.join("encoded.bgw");
    let world = source_world(&caller());
    let before = world.clone();
    let random = world.random_snapshot();
    let limits = Limits::default();
    let bytes = world.encode_legacy_file_bytes(limits, limits.allocated_bytes).unwrap();
    world.save_legacy_file(&path, limits, limits.allocated_bytes).unwrap();
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert!(matches!(world.encode_legacy_file_bytes(limits, 0), Err(WorldFileError::Encode { .. })));
    let limited = Limits { output_bytes: 0, ..limits };
    assert!(matches!(world.encode_legacy_file_bytes(limited, limits.allocated_bytes),
        Err(WorldFileError::Stream(_))));
    assert_eq!(world, before);
    assert_eq!(world.random_snapshot(), random);
}

#[test]
fn foreign_owner_file_roundtrip_and_failed_save_preserve_complete_committed_graph() {
    let directory = TestDirectory::new();
    let path = directory.0.join("owners.bgw");
    let input = crate::oracle_tests::fixture_path("crates/core")
        .join("tests/fixtures/world_owner_roles/foreign-peer-world-authored.ser");
    let expected_bytes = fs::read(&input).unwrap();
    let expected = StreamReader::new(&expected_bytes, Limits::default()).unwrap().read_document().unwrap();
    let caller = caller(); let owner = caller.handle(); let random = caller.random_snapshot();
    let limits = Limits::default();
    let world = caller.load_legacy_file(&input, limits, limits.allocated_bytes).unwrap();
    world.save_legacy_file(&path, limits, limits.allocated_bytes).unwrap();
    let committed = fs::read(&path).unwrap();
    let reopened = caller.load_legacy_file(&path, limits, limits.allocated_bytes).unwrap();
    let actual = reopened.encode_legacy_document(limits, limits.allocated_bytes).unwrap();
    super::super::legacy_export::tests::compare(&expected, &actual);
    let names = directory.contents();
    assert!(world.save_legacy_file(&path, Limits { output_bytes: 1, ..limits }, limits.allocated_bytes).is_err());
    assert_eq!(fs::read(&path).unwrap(), committed);
    assert_eq!(directory.contents(), names);
    assert!(caller.handle().same_owner(&owner));
    assert!(reopened.session().same_owner(&caller.session()));
    assert_eq!(caller.random_snapshot(), random);
}


#[test]
fn captured_nullable_colors_survive_public_file_load_draw_and_save_reopen() {
    let source = include_str!("../tests/fixtures/phenotype-color/phenotype_color.tsv");
    let (_, case) = source.split_once("C\tdraw_eyes_null_prefix\tcontrolled_raw_color_boundary\t827364\n").unwrap();
    let (case, _) = case.split_once("\nE\tdraw_eyes_null_prefix\n").unwrap();
    let row = |prefix: &str| {
        let mut matches = case.lines().filter(|line| line.starts_with(prefix));
        let found = matches.next().unwrap();
        assert!(matches.next().is_none(), "One captured {prefix} row");
        found
    };
    assert_eq!(row("M\tinitial\t"), "M\tinitial\tselector,victim,killer");
    assert!(row("X\tstep_0\t").starts_with(
        "X\tstep_0\tjava.lang.NullPointerException\tbiogenesis.Organism\tdraw\t",
    ));
    let expected_line: [i32; 5] = row("L\tstep_0\t").split('\t').skip(3)
        .map(|value| value.parse().unwrap()).collect::<Vec<_>>().try_into().unwrap();
    let expected_paint: i32 = row("D\tstep_0\t").split('\t').nth(2).unwrap().parse().unwrap();
    let expected_slots: Vec<Option<u32>> = case.lines()
        .filter(|line| line.starts_with("AC\tinitial\tvictim\t"))
        .map(|line| match line.split('\t').nth(5).unwrap() {
            "~" => None,
            argb => Some(u32::from_str_radix(argb, 16).unwrap()),
        }).collect();
    assert_eq!(expected_slots.len(), 3);
    assert!(expected_slots[1].is_none());
    let expected_tint = u32::from_str_radix(row("O\tinitial\tvictim\t_color\t")
        .rsplit_once(':').unwrap().1, 16).unwrap();
    let input = crate::oracle_tests::fixture_path("crates/core")
        .join("tests/fixtures/phenotype-color/draw_eyes_null_prefix-initial.ser");
    let limits = Limits::default();
    let expected = StreamReader::new(&fs::read(&input).unwrap(), limits).unwrap().read_document().unwrap();
    let directory = TestDirectory::new();
    let path = directory.0.join("nullable-colors.bgw");
    let caller = caller();
    let caller_before = caller.clone();
    let random = caller.random_snapshot();
    let world = caller.load_legacy_file(&input, limits, limits.allocated_bytes).unwrap();
    let check_draw = |world: &World| {
        let members = world.organisms.borrow();
        assert_eq!(members.len(), 3);
        let victim = members[1].borrow();
        let id = victim.id;
        let colors = victim.body.phenotype_colors.as_ref().unwrap();
        let actual: Vec<_> = colors.slots().iter()
            .map(|color| color.as_ref().map(crate::JavaColor::argb)).collect();
        assert_eq!(actual, expected_slots);
        assert_eq!(victim.state.temporary_color.as_ref().unwrap().argb(), expected_tint);
        drop(victim);
        drop(members);
        let crate::WorldError::Drawing { error, commands, paint } = world.render_segments().unwrap_err() else {
            panic!("Expected the captured nullable drawing failure");
        };
        assert_eq!(error, crate::GeometryError::MissingPhenotypeColor { segment: 1 });
        assert_eq!(paint, expected_paint as u32);
        // Source records victim.draw; other saved members provide World context only
        let victim_lines: Vec<_> = commands.iter().filter_map(|command| match command {
            crate::DrawCommand::Line(line) if line.organism_id == id => Some((
                [line.x1, line.y1, line.x2, line.y2],
                ((u32::from(line.alpha) << 24) | line.rgb) as i32,
            )),
            _ => None,
        }).collect();
        assert_eq!(victim_lines, [(
            [f64::from(expected_line[0]), f64::from(expected_line[1]),
                f64::from(expected_line[2]), f64::from(expected_line[3])],
            expected_line[4],
        )]);
    };
    check_draw(&world);
    world.save_legacy_file(&path, limits, limits.allocated_bytes).unwrap();
    let committed = StreamReader::new(&fs::read(&path).unwrap(), limits).unwrap().read_document().unwrap();
    super::super::legacy_export::tests::compare(&expected, &committed);
    let reopened = caller.load_legacy_file(&path, limits, limits.allocated_bytes).unwrap();
    check_draw(&reopened);
    let bytes = reopened.encode_legacy_file_bytes(limits, limits.allocated_bytes).unwrap();
    let restored = StreamReader::new(&bytes, limits).unwrap().read_document().unwrap();
    super::super::legacy_export::tests::compare(&expected, &restored);
    assert_eq!(caller, caller_before);
    assert_eq!(caller.random_snapshot(), random);
    assert_eq!(directory.contents(), [OsString::from("nullable-colors.bgw")]);
}
