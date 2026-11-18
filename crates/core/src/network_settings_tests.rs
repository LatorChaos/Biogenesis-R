// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;

#[test]
fn network_editor_applies_valid_fields_independently_and_preserves_unrelated_settings() {
    let mut map = PreferenceMap::from([
        ("VERSION".into(), "700".into()),
        ("unknown".into(), "retain".into()),
        ("WINDOW_X".into(), "-12".into()),
        ("SERVER_PORT".into(), "9123".into()),
        ("USER_NAME".into(), "before".into()),
    ]);
    let mut state = SettingsState::from_native_preferences(&map, "pt".into()).state;
    state.set_crisp_lines(true);
    state.set_automatic_backup_mode(AutomaticBackupMode::Managed);
    let original = state.clone();
    let report = state.apply_network_editor(true, "-1", "+7", JavaText::from_utf8("after"));
    assert!(!report.before.accept_connections);
    assert!(report.after.accept_connections);
    assert_eq!(report.before.user_name.units(), JavaText::from_utf8("before").units());
    assert_eq!(report.local_port, EditorOutcome::OutsideRange);
    assert_eq!(report.max_connections, EditorOutcome::Applied);
    assert_eq!(report.after.local_port, report.before.local_port);
    assert_eq!(report.after.max_connections, 7);
    assert!(report.user_name_applied);
    assert_eq!(state.network_settings(), report.after);
    for key in ALL_SETTING_KEYS {
        if !matches!(key, SettingKey::AcceptConnections | SettingKey::LocalPort
            | SettingKey::MaxConnections | SettingKey::UserName)
        {
            assert_eq!(state.get(key), original.get(key), "{key:?}");
        }
    }
    assert!(state.crisp_lines());
    assert_eq!(state.automatic_backup_mode(), AutomaticBackupMode::Managed);
    state.update_preferences(&mut map);
    assert_eq!(map["unknown"], "retain");
    assert_eq!(map["WINDOW_X"], "-12");
    let report = state.apply_network_editor(false, "65536", "overflow", JavaText::from_utf8(""));
    assert_eq!(report.local_port, EditorOutcome::Applied);
    assert_eq!(report.max_connections, EditorOutcome::InvalidNumber);
    assert_eq!(report.after.local_port, 65536);
    assert_eq!(report.after.max_connections, 7);
    assert!(!report.after.accept_connections);
    assert!(report.after.user_name.units().is_empty());
}

#[test]
fn network_integer_editors_preserve_java_parse_and_nonnegative_boundaries() {
    let mut state = SettingsState::new("en".into());
    for (text, expected) in [("0", None), ("65535", Some(65535)), ("65536", None),
        ("+\u{0661}\u{0968}\u{ff13}", Some(123)), (" 80", None), ("80 ", None),
        ("2147483648", None), ("-1", None)]
    {
        assert_eq!(parse_network_remote_port(text), expected, "{text:?}");
    }
    for text in ["", "+", " 2", "2 ", "2147483648", "-2147483649", "\u{1d7d9}"] {
        let before = state.network_settings();
        let report = state.apply_network_editor(true, text, text, before.user_name.clone());
        assert_eq!(report.local_port, EditorOutcome::InvalidNumber, "{text:?}");
        assert_eq!(report.max_connections, EditorOutcome::InvalidNumber, "{text:?}");
        assert_eq!(report.after.local_port, before.local_port);
        assert_eq!(report.after.max_connections, before.max_connections);
    }
    for text in ["-1", "-2147483648"] {
        let before = state.network_settings();
        let report = state.apply_network_editor(true, text, text, before.user_name.clone());
        assert_eq!(report.local_port, EditorOutcome::OutsideRange);
        assert_eq!(report.max_connections, EditorOutcome::OutsideRange);
        assert_eq!(report.after.local_port, before.local_port);
        assert_eq!(report.after.max_connections, before.max_connections);
    }
    for (text, expected) in [("-0", 0), ("+\u{0661}\u{0968}\u{ff13}", 123), ("65536", 65536), ("2147483647", i32::MAX)] {
        let report = state.apply_network_editor(false, text, text, JavaText::from_utf8("ID"));
        assert_eq!(report.local_port, EditorOutcome::Applied);
        assert_eq!(report.max_connections, EditorOutcome::Applied);
        assert_eq!(report.after.local_port, expected);
        assert_eq!(report.after.max_connections, expected);
    }
}

#[test]
fn network_username_retains_raw_units_and_owner_at_the_fifteen_unit_boundary() {
    let mut state = SettingsState::new("en".into());
    let mut units = vec![b'a' as u16; 14];
    units.push(0xd800);
    let accepted = JavaText::from_units(units.clone());
    let report = state.apply_network_editor(false, "8888", "1", accepted.clone());
    assert!(report.user_name_applied);
    assert_eq!(report.after.user_name.units(), units.as_slice());
    assert!(state.network_settings().user_name.same_instance(&accepted));
    units.push(0xdc00);
    let rejected = JavaText::from_units(units);
    let report = state.apply_network_editor(true, "0", "0", rejected);
    assert!(!report.user_name_applied);
    assert!(report.before.user_name.same_instance(&accepted));
    assert!(report.after.user_name.same_instance(&accepted));
    assert!(report.after.accept_connections);
    assert_eq!(report.after.local_port, 0);
    assert_eq!(report.after.max_connections, 0);
}

#[test]
fn network_username_counts_supplementary_units_and_accepts_empty_text() {
    let mut state = SettingsState::new("en".into());
    let accepted = JavaText::from_utf8(&("a".repeat(13) + "\u{1f600}"));
    assert!(state.apply_network_editor(false, "8888", "1", accepted.clone()).user_name_applied);
    let too_long = JavaText::from_utf8(&("a".repeat(14) + "\u{1f600}"));
    assert!(!state.apply_network_editor(false, "8888", "1", too_long).user_name_applied);
    assert!(state.network_settings().user_name.same_instance(&accepted));
    let empty = JavaText::from_units(vec![]);
    let report = state.apply_network_editor(false, "8888", "1", empty.clone());
    assert!(report.user_name_applied);
    assert!(report.after.user_name.units().is_empty());
    assert!(state.network_settings().user_name.same_instance(&empty));
}
