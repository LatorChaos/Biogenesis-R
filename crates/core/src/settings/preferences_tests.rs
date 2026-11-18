// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

include!("preferences_fixture_map.rs");

fn fixture(case: &str, phase: &str) -> PreferenceMap {
    parse_preferences_xml(
        XML.iter()
            .find(|entry| entry.0 == case && entry.1 == phase)
            .unwrap()
            .2,
    )
    .unwrap()
}

fn encoded(text: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in text.as_bytes().chunks(3) {
        let bits = u32::from(chunk[0]) << 16
            | u32::from(*chunk.get(1).unwrap_or(&0)) << 8
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for index in 0..4 {
            result.push(if index > chunk.len() {
                '='
            } else {
                ALPHABET[((bits >> (18 - index * 6)) & 63) as usize] as char
            });
        }
    }
    result
}

#[test]
fn actual_original_preferences_match_all_typed_snapshots_maps_and_startup_transitions() {
    let mut current = None;
    let mut case = "";
    let mut scalars = 0;
    let mut cases = 0;
    let mut snapshots = 0;
    let mut calls = 0;
    for line in include_str!("../../tests/fixtures/preferences/observations.tsv").lines() {
        let row: Vec<_> = line.split('\t').collect();
        match row[0] {
            "C" => {
                case = row[1];
                let input = fixture(case, "input");
                let unchanged = input.clone();
                let loaded = SettingsState::from_preferences(&input, "en".to_owned());
                assert_eq!(input, unchanged, "Import must not clear the legacy map");
                let mut source_projection = match loaded.disposition {
                    PreferenceDisposition::Loaded => input.clone(),
                    PreferenceDisposition::DefaultsForVersion { .. } => PreferenceMap::new(),
                    PreferenceDisposition::InvalidAutomaticBackupMode
                    | PreferenceDisposition::InvalidUserNameEncoding => panic!("Legacy imports cannot interpret native metadata"),
                };
                if let Some(value) = loaded.hardware.persisted_override {
                    source_projection.insert("HARDWARE_ACCELERATION".to_owned(), value.to_string());
                }
                assert_eq!(
                    source_projection,
                    fixture(case, "read"),
                    "{case}: named source map projection"
                );
                loaded.state.update_preferences(&mut source_projection);
                assert_eq!(
                    source_projection,
                    fixture(case, "save"),
                    "{case}: ordinary source save map"
                );
                current = Some(loaded.state);
                cases += 1;
            }
            "P" => snapshots += 1,
            "K" => calls += 1,
            "S" => {
                assert_eq!(row[1], case);
                let key = ALL_SETTING_KEYS
                    .into_iter()
                    .find(|key| key.source_name() == row[3])
                    .unwrap();
                let value = current.as_ref().unwrap().get(key);
                let observed = match value {
                    SettingValue::Int(value) => {
                        assert_eq!(row[4], "int");
                        value.to_string()
                    }
                    SettingValue::Double(value) => {
                        assert_eq!(row[4], "double");
                        format!("{:016x}", value.to_bits())
                    }
                    SettingValue::Boolean(value) => {
                        assert_eq!(row[4], "boolean");
                        value.to_string()
                    }
                    SettingValue::Text(value) => {
                        assert_eq!(row[4], "String");
                        encoded(&value)
                    }
                };
                assert_eq!(observed, row[5], "{case}/{} {}", row[2], row[3]);
                scalars += 1;
            }
            // Input, XML custody, RNG, and Java2D metadata retain their separate producer evidence
            "I" | "R" | "F" | "N" | "M" | "Y" => {}
            _ => panic!("Unknown frozen preference record"),
        }
    }
    assert_eq!((cases, snapshots, calls, scalars), (13, 26, 26, 3926));
    for (_, _, bytes) in XML {
        let map = parse_preferences_xml(bytes).unwrap();
        assert_eq!(
            parse_preferences_xml(&encode_preferences_xml(&map).unwrap()).unwrap(),
            map
        );
    }
}

#[test]
fn xml_rejects_unsafe_shapes_entities_versions_and_invalid_characters() {
    for text in [
        "",
        "<map>",
        "<map/><map/>",
        "<map><other/></map>",
        "<map><entry key=\"x\" value=\"1\"/><entry key=\"x\" value=\"2\"/></map>",
        "<map><entry key=\"x\"/></map>",
        "<map><entry key=\"x\" value=\"1\" extra=\"x\"/></map>",
        "<map><entry key=\"x\" value=\"&secret;\"/></map>",
        "<!DOCTYPE map SYSTEM \"file:///etc/passwd\"><map/>",
        "<!DOCTYPE map [<!ENTITY secret 'value'>]><map/>",
        "<map MAP_XML_VERSION=\"2.0\"/>",
        "<map><entry key=\"x\" value=\"&#0;\"/></map>",
        "<map><entry key=\"x\" value=\"bad<value\"/></map>",
        "<?xml version=\"1.0\" version=\"1.0\"?><map/>",
        "<?xml version=\"1.1\"?><map/>",
        "<?xml version=\"1.0\" encoding=\"UTF-16\"?><map/>",
        "<map>text</map>",
        "<map>\0</map>",
    ] {
        assert!(
            parse_preferences_xml(text.as_bytes()).is_err(),
            "Accepted {text:?}"
        );
    }
    let map = PreferenceMap::from([("x\t\n\r<&>\"".to_owned(), "value\t\r\n\u{1f40e}".to_owned())]);
    assert_eq!(
        parse_preferences_xml(&encode_preferences_xml(&map).unwrap()).unwrap(),
        map
    );
    let normalized =
        parse_preferences_xml(b"<map><entry key=\"x\" value=\"a\r\nb\tc&#xA;d\"/></map>").unwrap();
    assert_eq!(normalized["x"], "a b c\nd");
}

#[test]
fn map_and_file_budgets_reject_before_a_settings_owner_changes() {
    assert!(matches!(
        parse_preferences_xml(&vec![b' '; MAX_PREFERENCE_BYTES + 1]),
        Err(PreferenceError::ResourceLimit("input bytes"))
    ));
    let map = PreferenceMap::from([("huge".to_owned(), "&".repeat(MAX_PREFERENCE_BYTES / 5 + 1))]);
    assert!(matches!(
        encode_preferences_xml(&map),
        Err(PreferenceError::ResourceLimit("output bytes"))
    ));
    let map: PreferenceMap = (0..MAX_ENTRIES + 1)
        .map(|index| (index.to_string(), "x".to_owned()))
        .collect();
    assert!(matches!(
        encode_preferences_xml(&map),
        Err(PreferenceError::ResourceLimit("entries"))
    ));
    let map = PreferenceMap::from([
        ("VERSION".to_owned(), "bad".to_owned()),
        ("INITIAL_ORGANISMS".to_owned(), "7".to_owned()),
    ]);
    let load = SettingsState::from_preferences(&map, "ru".to_owned());
    assert_eq!(
        load.disposition,
        PreferenceDisposition::DefaultsForVersion { stored: None }
    );
    assert_eq!(load.state.genesis().initial_attempts, 12500);
    assert_eq!(load.state.language(), "ru");
    assert_eq!(map["INITIAL_ORGANISMS"], "7");
}

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        for _ in 0..128 {
            let path = std::env::temp_dir().join(format!(
                "bgr-preferences-{}-{}",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("Cannot create isolated test directory: {error}"),
            }
        }
        panic!("Test directory names are occupied");
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("Cannot remove owned test directory");
    }
}

#[test]
fn native_save_preserves_unknown_keys_legacy_bytes_and_prior_file_on_failure() {
    let directory = TestDirectory::new();
    let path = directory.0.join("native.xml");
    let legacy = directory.0.join("legacy.xml");
    let original = XML
        .iter()
        .find(|entry| entry.0 == "full-valid" && entry.1 == "input")
        .unwrap()
        .2;
    fs::write(&legacy, original).unwrap();
    assert!(read_preferences_file(&path).unwrap().is_none());
    let loaded = SettingsState::from_preferences(
        &read_preferences_file(&legacy).unwrap().unwrap(),
        "en".to_owned(),
    );
    fs::write(
        &path,
        encode_preferences_xml(&PreferenceMap::from([(
            "unknown".to_owned(),
            "preserved".to_owned(),
        )]))
        .unwrap(),
    )
    .unwrap();
    loaded.state.save_preferences(&path).unwrap();
    let first = fs::read(&path).unwrap();
    let saved = read_preferences_file(&path).unwrap().unwrap();
    assert_eq!(saved["unknown"], "preserved");
    assert!(!saved.contains_key("LOCALE_INDEX") && !saved.contains_key("SEED"));
    assert_eq!(
        saved["WINDOW_WIDTH"],
        loaded.state.application.window_width.to_string()
    );
    loaded.state.save_preferences(&path).unwrap();
    assert_eq!(
        fs::read(&path).unwrap(),
        first,
        "Repeated save must be idempotent"
    );
    for version in ["701", "invalid"] {
        let future = PreferenceMap::from([
            ("VERSION".to_owned(), version.to_owned()),
            ("WORLD_WIDTH".to_owned(), "123".to_owned()),
            ("unknown".to_owned(), "future data".to_owned()),
        ]);
        let bytes = encode_preferences_xml(&future).unwrap();
        fs::write(&path, &bytes).unwrap();
        let error = loaded.state.save_preferences(&path).unwrap_err();
        assert!(
            matches!(error, PreferenceError::UnsupportedNativeVersion { stored } if stored == java_int(version))
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read(&legacy).unwrap(), original);
    }
    fs::write(&path, b"broken").unwrap();
    assert!(loaded.state.save_preferences(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"broken");
    assert_eq!(fs::read(&legacy).unwrap(), original);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(&path, &first).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let error = loaded.state.save_preferences(&path).unwrap_err();
        assert!(
            matches!(error, PreferenceError::Replace(ref error) if !error.replacement_committed)
        );
        assert_eq!(fs::read(&path).unwrap(), first);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&legacy, &path).unwrap();
        assert!(loaded.state.save_preferences(&path).is_err());
        assert!(read_preferences_file(&path).is_err());
        assert_eq!(fs::read(&legacy).unwrap(), original);
    }
}

#[test]
fn native_display_extension_and_locale_do_not_change_source_keys_or_genesis() {
    let map = PreferenceMap::from([
        ("VERSION".to_owned(), "700".to_owned()),
        ("LOCALE".to_owned(), "PT_BR".to_owned()),
        ("DELAY".to_owned(), "-5".to_owned()),
        ("BGR_CRISP_LINES".to_owned(), "TrUe".to_owned()),
    ]);
    let loaded = SettingsState::from_native_preferences(&map, "en".to_owned());
    assert!(loaded.state.crisp_lines());
    assert_eq!(loaded.state.language(), "pt_br");
    assert_eq!(loaded.state.locale_index(), 1);
    assert_eq!(loaded.state.get(SettingKey::Delay), SettingValue::Int(-5));
    assert_eq!(ALL_SETTING_KEYS.len(), 151);
    assert_eq!(NUMERIC_EDITOR_KEYS.len(), 132);
    let legacy = SettingsState::from_preferences(&map, "en".to_owned());
    assert!(!legacy.state.crisp_lines());
    let mut ordinary = PreferenceMap::new();
    loaded.state.update_preferences(&mut ordinary);
    assert!(!ordinary.contains_key("BGR_CRISP_LINES"));
    assert!(!SettingsState::new("en".to_owned()).crisp_lines());
}

#[test]
fn native_geometry_and_crisp_roundtrip_preserve_unknown_keys_and_reject_partial_geometry() {
    let directory = TestDirectory::new();
    let path = directory.0.join("native-display.xml");
    fs::write(&path, encode_preferences_xml(&PreferenceMap::from([
        ("VERSION".to_owned(), "700".to_owned()),
        ("unknown".to_owned(), "keep".to_owned()),
    ])).unwrap()).unwrap();
    let mut state = SettingsState::new("pt_br".to_owned());
    let before = state.clone();
    assert_eq!(state.record_window_geometry([2, 3, 0, 700, 0]), Err(SettingsError::InvalidWindowGeometry));
    assert_eq!(state, before);
    state.record_window_geometry([-20, 32, 1000, 760, 6]).unwrap();
    state.set_crisp_lines(true);
    state.save_preferences(&path).unwrap();
    let saved = read_preferences_file(&path).unwrap().unwrap();
    assert_eq!(saved["unknown"], "keep");
    assert_eq!(saved["WINDOW_X"], "-20");
    assert_eq!(saved["WINDOW_HEIGHT"], "760");
    assert_eq!(saved["WINDOW_STATE"], "6");
    assert_eq!(saved["BGR_CRISP_LINES"], "true");
    assert_eq!(saved["LOCALE"], "pt_br");
    assert!(!saved.contains_key("SEED") && !saved.contains_key("LOCALE_INDEX"));
    let loaded = SettingsState::from_native_preferences(&saved, "en".to_owned());
    assert!(loaded.state.crisp_lines());
    assert_eq!(loaded.state.get(SettingKey::WindowX), SettingValue::Int(-20));
    assert_eq!(loaded.state.language(), "pt_br");
    state.set_crisp_lines(false);
    state.save_preferences(&path).unwrap();
    assert!(!SettingsState::from_native_preferences(&read_preferences_file(&path).unwrap().unwrap(), "en".to_owned()).state.crisp_lines());
}

#[test]
fn native_automatic_backup_mode_defaults_roundtrips_and_rejects_invalid_files() {
    let directory = TestDirectory::new();
    let path = directory.0.join("mode.xml");
    let mut state = SettingsState::new("en".into());
    assert_eq!(state.automatic_backup_mode(), AutomaticBackupMode::Adjacent);
    state.set_automatic_backup_mode(AutomaticBackupMode::Managed);
    state.save_preferences(&path).unwrap();
    let map = read_preferences_file(&path).unwrap().unwrap();
    assert_eq!(map.get("BGR_AUTOMATIC_BACKUP_MODE").unwrap(), "managed");
    let loaded = SettingsState::from_native_preferences(&map, "en".into());
    assert_eq!(loaded.disposition, PreferenceDisposition::Loaded);
    assert_eq!(loaded.state.automatic_backup_mode(), AutomaticBackupMode::Managed);
    assert_eq!(SettingsState::from_preferences(&map, "en".into()).state.automatic_backup_mode(), AutomaticBackupMode::Adjacent);
    let mut absent = map.clone();
    absent.remove("BGR_AUTOMATIC_BACKUP_MODE");
    assert_eq!(SettingsState::from_native_preferences(&absent, "en".into()).state.automatic_backup_mode(), AutomaticBackupMode::Adjacent);
    let mut invalid = map;
    invalid.insert("BGR_AUTOMATIC_BACKUP_MODE".into(), "unknown-mode".into());
    fs::write(&path, encode_preferences_xml(&invalid).unwrap()).unwrap();
    let before = fs::read(&path).unwrap();
    let rejected = SettingsState::from_native_preferences(&invalid, "en".into());
    assert_eq!(rejected.disposition, PreferenceDisposition::InvalidAutomaticBackupMode);
    assert!(matches!(state.save_preferences(&path), Err(PreferenceError::Malformed(_))));
    assert_eq!(fs::read(&path).unwrap(), before);
}


#[test]
fn native_username_companion_roundtrips_raw_units_without_replacement_text() {
    let directory = TestDirectory::new();
    let path = directory.0.join("username.xml");
    fs::write(&path, encode_preferences_xml(&PreferenceMap::from([
        ("VERSION".into(), "700".into()),
        ("unknown".into(), "retain".into()),
    ])).unwrap()).unwrap();
    let mut state = SettingsState::new("en".into());
    for (units, expected) in [(vec![0xd800], "1:d800"), (vec![0], "1:0000"), (vec![0xfffe], "1:fffe")] {
        let value = JavaText::from_units(units.clone());
        assert!(state.apply_network_editor(false, "8888", "1", value).user_name_applied);
        state.save_preferences(&path).unwrap();
        let saved = read_preferences_file(&path).unwrap().unwrap();
        assert_eq!(saved["USER_NAME"], "");
        assert_eq!(saved[USER_NAME_UTF16_KEY], expected);
        assert_eq!(saved["unknown"], "retain");
        let loaded = SettingsState::from_native_preferences(&saved, "en".into());
        assert_eq!(loaded.disposition, PreferenceDisposition::Loaded);
        assert_eq!(loaded.state.network_settings().user_name.units(), units.as_slice());
        let legacy = SettingsState::from_preferences(&saved, "en".into());
        assert_eq!(legacy.disposition, PreferenceDisposition::Loaded);
        assert!(legacy.state.network_settings().user_name.units().is_empty());
        let first = fs::read(&path).unwrap();
        loaded.state.save_preferences(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), first);
    }
}

#[test]
fn native_username_returns_to_ordinary_xml_for_supplementary_and_empty_names() {
    let directory = TestDirectory::new();
    let path = directory.0.join("username-upgrade.xml");
    let mut state = SettingsState::new("en".into());
    state.apply_network_editor(false, "8888", "1", JavaText::from_units(vec![0xdfff]));
    state.save_preferences(&path).unwrap();
    for text in ["\u{1f600}<&", ""] {
        let value = JavaText::from_utf8(text);
        state.apply_network_editor(false, "8888", "1", value.clone());
        state.save_preferences(&path).unwrap();
        let saved = read_preferences_file(&path).unwrap().unwrap();
        assert_eq!(saved["USER_NAME"], text);
        assert!(!saved.contains_key(USER_NAME_UTF16_KEY));
        let loaded = SettingsState::from_native_preferences(&saved, "en".into());
        assert_eq!(loaded.disposition, PreferenceDisposition::Loaded);
        assert_eq!(loaded.state.network_settings().user_name, value);
    }
}

#[test]
fn invalid_native_username_metadata_rejects_load_and_preserves_file_on_save() {
    let directory = TestDirectory::new();
    let path = directory.0.join("username-invalid.xml");
    let state = SettingsState::new("en".into());
    let mut cases = vec!["".to_owned(), "1:".to_owned(), "2:d800".to_owned(), "1:d80".to_owned(), "1:gggg".to_owned()];
    cases.push(format!("1:{}", "d800".repeat(16)));
    for (encoded, ordinary) in cases.into_iter().map(|encoded| (encoded, "")).chain([(String::from("1:d800"), "ambiguous")]) {
        let map = PreferenceMap::from([
            ("VERSION".into(), "700".into()),
            ("USER_NAME".into(), ordinary.into()),
            ("LOCAL_PORT".into(), "123".into()),
            (USER_NAME_UTF16_KEY.into(), encoded),
            ("unknown".into(), "retain".into()),
        ]);
        let loaded = SettingsState::from_native_preferences(&map, "pt".into());
        assert_eq!(loaded.disposition, PreferenceDisposition::InvalidUserNameEncoding);
        assert_eq!(loaded.state, SettingsState::new("pt".into()));
        let legacy = SettingsState::from_preferences(&map, "pt".into());
        assert_eq!(legacy.disposition, PreferenceDisposition::Loaded);
        assert_eq!(legacy.state.get(SettingKey::LocalPort), SettingValue::Int(123));
        assert_eq!(legacy.state.get(SettingKey::UserName), SettingValue::Text(ordinary.into()));
        let bytes = encode_preferences_xml(&map).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert!(matches!(state.save_preferences(&path), Err(PreferenceError::InvalidUserNameEncoding)));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}
