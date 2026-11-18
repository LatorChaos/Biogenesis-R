// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::settings::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(in crate::world) fn text(value: &str) -> String {
    assert_ne!(value, "~", "A logical Core string is never null");
    assert_eq!(value.len() % 4, 0);
    let mut bytes = Vec::new();
    for chunk in value.as_bytes().chunks_exact(4) {
        let digit = |byte| match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("Invalid Base64 digit"),
        };
        let a = digit(chunk[0]);
        let b = digit(chunk[1]);
        bytes.push((a << 2) | (b >> 4));
        if chunk[2] == b'=' {
            assert_eq!(chunk[3], b'=');
            assert_eq!(b & 15, 0);
            continue;
        }
        let c = digit(chunk[2]);
        bytes.push((b << 4) | (c >> 2));
        if chunk[3] == b'=' {
            assert_eq!(c & 3, 0);
            continue;
        }
        bytes.push((c << 6) | digit(chunk[3]));
    }
    String::from_utf8(bytes).unwrap()
}

fn value(kind: &str, raw: &str) -> SettingValue {
    match kind {
        "int" => SettingValue::Int(raw.parse().unwrap()),
        "double" => {
            assert_eq!(raw.len(), 16);
            SettingValue::Double(f64::from_bits(u64::from_str_radix(raw, 16).unwrap()))
        }
        "boolean" => SettingValue::Boolean(match raw {
            "true" => true,
            "false" => false,
            _ => panic!("Invalid boolean"),
        }),
        "String" => SettingValue::Text(text(raw)),
        _ => panic!("Unknown Core scalar type {kind}"),
    }
}

fn equal(actual: SettingValue, expected: SettingValue, context: &str) {
    match (actual, expected) {
        (SettingValue::Double(actual), SettingValue::Double(expected)) => {
            assert_eq!(actual.to_bits(), expected.to_bits(), "{context}")
        }
        (actual, expected) => assert_eq!(actual, expected, "{context}"),
    }
}

#[derive(Default)]
struct Snapshot {
    stage: String,
    seen: BTreeMap<String, BTreeSet<String>>,
    settings: Fields,
    preference_count: Option<usize>,
}

fn close_snapshot(
    snapshot: &mut Option<Snapshot>,
    state: &SettingsState,
    drafts: &Option<SettingsDraft>,
) {
    let Some(snapshot) = snapshot.take() else {
        return;
    };
    let keys = |tag: &str| snapshot.seen.get(tag).map(BTreeSet::len).unwrap_or(0);
    assert_eq!(
        (
            keys("S"),
            keys("W"),
            keys("V"),
            keys("T"),
            keys("R"),
            keys("Y"),
            keys("A")
        ),
        (151, 15, 52, 49, 1, 3, 42)
    );
    assert_eq!(keys("D"), if snapshot.stage == "initial" { 0 } else { 141 });
    assert_eq!(keys("F"), snapshot.preference_count.unwrap() + 1);
    assert_eq!(
        format!("{:?}", state.parameters()),
        format!("{:?}", parameters(&snapshot.settings)),
        "Independent existing owner projection at {}",
        snapshot.stage
    );
    assert_eq!(drafts.is_some(), snapshot.stage != "initial");
}

#[test]
fn original_settings_dialog_replays_exact_core_values_and_logical_controls() {
    let mut tags = BTreeMap::<String, usize>::new();
    let mut state = SettingsState::new("en".to_owned());
    let defaults = state.clone();
    let mut draft: Option<SettingsDraft> = None;
    let mut snapshot: Option<Snapshot> = None;
    let mut name = String::new();
    let mut case_open = false;
    let mut stages = Vec::new();
    let mut commands = Vec::new();
    let mut locale_events = VecDeque::<(String, i32, i32, i32)>::new();
    let mut source_invariants = BTreeMap::<String, Vec<String>>::new();
    let mut schemas = BTreeMap::<String, BTreeSet<String>>::new();
    let mut core_rows = 0;
    let mut core_comparisons = 0;
    let mut default_checks = 0;
    let mut source_invariant_rows = 0;
    let mut projected_dialog_exclusions = 0;
    let mut excluded_rows = 0;
    let mut input_rows = 0;
    let mut metadata_rows = 0;
    let mut cases = BTreeSet::new();
    let mut supplied_defaults = BTreeSet::new();
    for (index, line) in include_str!("../tests/fixtures/settings.tsv")
        .lines()
        .enumerate()
    {
        let row: Vec<_> = line.split('\t').collect();
        let tag = row[0];
        *tags.entry(tag.to_owned()).or_default() += 1;
        let columns = match tag {
            "C" | "U" | "Y" => 4,
            "E" | "P" => 2,
            "I" | "S" | "F" | "W" | "V" | "T" | "A" | "D" => 5,
            "K" => 3,
            "R" => 6,
            "L" => 9,
            _ => panic!("Unknown tag"),
        };
        assert_eq!(row.len(), columns, "row{}", index + 1);
        let context = format!(
            "{name}:row{}:{tag}:{}",
            index + 1,
            row.get(2).unwrap_or(&"")
        );
        if tag == "C" {
            assert!(!case_open);
            assert!(cases.insert(row[1].to_owned()));
            assert_eq!(row[2], "827364");
            assert_eq!(row[3], "settings-dialog");
            name = row[1].to_owned();
            state = defaults.clone();
            draft = None;
            stages.clear();
            commands.clear();
            source_invariants.clear();
            supplied_defaults.clear();
            assert!(locale_events.is_empty());
            case_open = true;
            metadata_rows += 1;
            continue;
        }
        assert!(case_open, "{context}");
        match tag {
            "U" => {
                let key = SettingKey::from_source_name(row[1]).expect("Declared Utils key");
                assert!(supplied_defaults.insert(row[1].to_owned()));
                equal(defaults.get(key), value(row[2], row[3]), &context);
                default_checks += 1;
                input_rows += 1;
            }
            "I" => {
                input_rows += 1;
                if let Some(key) = row[2].strip_prefix("utils:") {
                    assert_eq!(row[1], "setup");
                    state
                        .source_input(
                            SettingKey::from_source_name(key).unwrap(),
                            value(row[3], row[4]),
                        )
                        .unwrap();
                } else if let Some(key) = row[2].strip_prefix("editor:") {
                    assert_eq!(row[1], "edited");
                    assert_eq!(row[3], "String");
                    draft
                        .as_mut()
                        .unwrap()
                        .set_editor(SettingKey::from_source_name(key).unwrap(), text(row[4]))
                        .unwrap();
                } else if let Some(key) = row[2].strip_prefix("choice:") {
                    assert_eq!(row[1], "edited");
                    assert_eq!(row[3], "String");
                    let draft = draft.as_mut().unwrap();
                    let input = text(row[4]);
                    let (mut locale, mut backup, mut hardware, mut fbo) = (
                        draft.locale_index(),
                        draft.automatic_backup(),
                        draft.hardware(),
                        draft.disable_framebuffer(),
                    );
                    match key {
                        "localeCombo" => locale = input.parse().unwrap(),
                        "autoBackupsCheck" => backup = input.parse().unwrap(),
                        "hardware" => match input.as_str() {
                            "none" => {
                                hardware = HardwareSelection::None;
                                fbo = false;
                            }
                            "plain" => {
                                hardware = HardwareSelection::OpenGl;
                                fbo = false;
                            }
                            "fbo" => {
                                hardware = HardwareSelection::OpenGl;
                                fbo = true;
                            }
                            _ => panic!("Unknown hardware choice"),
                        },
                        _ => panic!("Unknown choice"),
                    }
                    draft.set_choices(locale, backup, hardware, fbo).unwrap();
                } else {
                    assert_eq!(row[1], "setup");
                    match row[2] {
                        "locale" => {
                            assert_eq!(row[3], "String");
                            state
                                .source_input(SettingKey::Locale, value(row[3], row[4]))
                                .unwrap();
                        }
                        "hardware:setHardwareAcceleration" => {
                            assert_eq!(row[3], "int");
                            state
                                .source_input(
                                    SettingKey::HardwareAcceleration,
                                    value(row[3], row[4]),
                                )
                                .unwrap();
                        }
                        "rng:seed" => {
                            assert_eq!(row[3], "long");
                            assert_eq!(row[4], "827364");
                        }
                        "property:sun.java2d.opengl"
                        | "property:sun.java2d.noddraw"
                        | "property:sun.java2d.opengl.fbobject" => {
                            assert_eq!(row[3], "String");
                            assert_eq!(row[4], "~");
                        }
                        "preferences:VERSION" => {
                            assert_eq!(row[3], "String");
                            assert_eq!(text(row[4]), "700");
                        }
                        "preferences:HARNESS_SENTINEL" => {
                            assert_eq!(row[3], "String");
                            assert_eq!(text(row[4]), name);
                        }
                        _ => panic!("Undeclared input target"),
                    }
                }
            }
            "P" => {
                close_snapshot(&mut snapshot, &state, &draft);
                assert_eq!(supplied_defaults.len(), 149);
                if row[1] == "opened" {
                    assert!(draft.is_none());
                    draft = Some(SettingsDraft::new(&state));
                }
                snapshot = Some(Snapshot {
                    stage: row[1].to_owned(),
                    ..Snapshot::default()
                });
                stages.push(row[1].to_owned());
                metadata_rows += 1;
            }
            "K" => {
                close_snapshot(&mut snapshot, &state, &draft);
                assert_eq!(row[1], format!("action-{}", commands.len()));
                commands.push(row[2].to_owned());
                match row[2] {
                    "ok" => {
                        draft
                            .as_mut()
                            .unwrap()
                            .ok(&mut state, |at_callback| {
                                locale_events.push_back((
                                    at_callback.language().to_owned(),
                                    at_callback.genesis().width,
                                    at_callback.parameters().genetics.max_mutation_rate,
                                    at_callback.parameters().genetics.min_mutation_rate,
                                ));
                            })
                            .unwrap();
                    }
                    "cancel" => draft.as_mut().unwrap().cancel().unwrap(),
                    "defaults" => draft.as_mut().unwrap().defaults().unwrap(),
                    _ => panic!("Unknown original command"),
                }
                metadata_rows += 1;
            }
            "L" => {
                assert_eq!(row[1], format!("action-{}", commands.len() - 1));
                assert_eq!(row[2].parse::<usize>().unwrap(), tags["L"] - 1);
                assert_eq!(row[3], "text");
                text(row[4]);
                let actual = locale_events
                    .pop_front()
                    .expect("Immediate actual Core locale callback");
                assert_eq!(
                    actual,
                    (
                        text(row[5]),
                        row[6].parse().unwrap(),
                        row[7].parse().unwrap(),
                        row[8].parse().unwrap()
                    ),
                    "{context}"
                );
                core_rows += 1;
                core_comparisons += 4;
            }
            "E" => {
                close_snapshot(&mut snapshot, &state, &draft);
                assert_eq!(row[1], name);
                assert!(locale_events.is_empty());
                let expected: Vec<_> = [
                    "initial".to_owned(),
                    "opened".to_owned(),
                    "edited".to_owned(),
                ]
                .into_iter()
                .chain((0..commands.len()).map(|i| format!("after-{i}")))
                .collect();
                assert_eq!(stages, expected);
                assert!(!draft.as_ref().unwrap().is_open());
                case_open = false;
                metadata_rows += 1;
            }
            _ => {
                let observed = snapshot.as_mut().expect("Snapshot observations");
                assert_eq!(row[1], observed.stage, "{context}");
                let field = if tag == "R" { "R" } else { row[2] };
                assert!(
                    observed
                        .seen
                        .entry(tag.to_owned())
                        .or_default()
                        .insert(field.to_owned()),
                    "Duplicate field {context}"
                );
                if matches!(tag, "S" | "W" | "V" | "T" | "Y" | "A" | "D") {
                    schemas
                        .entry(tag.to_owned())
                        .or_default()
                        .insert(if tag == "Y" {
                            field.to_owned()
                        } else {
                            format!("{field}:{}", row[3])
                        });
                }
                match tag {
                    "S" => {
                        let key = SettingKey::from_source_name(field).unwrap();
                        equal(state.get(key), value(row[3], row[4]), &context);
                        observed
                            .settings
                            .insert(field.to_owned(), row[4].to_owned());
                        core_rows += 1;
                        core_comparisons += 1;
                    }
                    "D" => {
                        let draft = draft.as_ref().unwrap();
                        if let Some(key) = SettingKey::from_source_name(field) {
                            assert_eq!(row[3], "String");
                            let i = NUMERIC_EDITOR_KEYS
                                .iter()
                                .position(|candidate| *candidate == key)
                                .unwrap();
                            assert_eq!(draft.editors()[i], text(row[4]), "{context}");
                        } else {
                            let actual = match field {
                                "localeCombo" => SettingValue::Int(draft.locale_index() as i32),
                                "autoBackupsCheck" => {
                                    SettingValue::Boolean(draft.automatic_backup())
                                }
                                "hardwareNoneRadio" => SettingValue::Boolean(
                                    draft.hardware() == HardwareSelection::None,
                                ),
                                "hardwareOpenGLRadio" => SettingValue::Boolean(
                                    draft.hardware() == HardwareSelection::OpenGl,
                                ),
                                "hardwareFBObjectCheck" => {
                                    SettingValue::Boolean(draft.disable_framebuffer())
                                }
                                "backupDelayText.enabled" => {
                                    SettingValue::Boolean(draft.backup_delay_enabled())
                                }
                                "hardwareFBObjectCheck.enabled" => {
                                    SettingValue::Boolean(draft.framebuffer_control_enabled())
                                }
                                "dialog.displayable" | "dialog.visible" => {
                                    assert_eq!(row[3], "boolean");
                                    row[4].parse::<bool>().unwrap();
                                    projected_dialog_exclusions += 1;
                                    continue;
                                }
                                _ => panic!("Unknown draft field"),
                            };
                            equal(actual, value(row[3], row[4]), &context);
                        }
                        core_rows += 1;
                        core_comparisons += 1;
                    }
                    "W" | "V" | "T" | "R" => {
                        let key = format!("{tag}:{field}");
                        let payload: Vec<_> =
                            row[2..].iter().map(|entry| (*entry).to_owned()).collect();
                        if observed.stage == "initial" {
                            assert!(source_invariants.insert(key, payload).is_none());
                        } else {
                            assert_eq!(
                                source_invariants.get(&key).unwrap(),
                                &payload,
                                "Source cached/purity invariant {context}"
                            );
                        }
                        source_invariant_rows += 1;
                    }
                    "F" => {
                        if field == "_key_count" {
                            assert_eq!(row[3], "int");
                            observed.preference_count = Some(row[4].parse().unwrap());
                        } else {
                            assert_eq!(row[3], "String");
                            text(row[4]);
                        }
                        excluded_rows += 1;
                    }
                    "A" | "Y" => {
                        if (tag == "Y" || row[3] == "String") && row.last() != Some(&"~") {
                            text(row[row.len() - 1]);
                        }
                        excluded_rows += 1;
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
    assert!(!case_open);
    assert_eq!(cases.len(), 68);
    let expected: BTreeMap<_, _> = [
        ("C", 68),
        ("U", 10132),
        ("I", 13098),
        ("P", 276),
        ("S", 41676),
        ("Y", 828),
        ("F", 10253),
        ("W", 4140),
        ("V", 14352),
        ("T", 13524),
        ("R", 276),
        ("A", 11592),
        ("D", 29328),
        ("K", 72),
        ("E", 68),
        ("L", 7),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value))
    .collect();
    assert_eq!(tags, expected);
    assert_eq!(tags.values().sum::<usize>(), 149690);
    assert_eq!(
        (
            core_rows,
            core_comparisons,
            default_checks,
            source_invariant_rows,
            excluded_rows + projected_dialog_exclusions,
            input_rows,
            metadata_rows
        ),
        (70595, 70616, 10132, 32292, 23089, 23230, 484)
    );
    for (tag, count) in [
        ("S", 151),
        ("D", 141),
        ("W", 15),
        ("V", 52),
        ("T", 49),
        ("Y", 3),
        ("A", 42),
    ] {
        assert_eq!(schemas[tag].len(), count, "Complete typed schema {tag}");
    }
    println!(
        "SETTINGS_CORE cases=68 snapshots=276 buttons=72 raw_rows=149690 Core_rows=70595 Core_comparisons=70616 default_input_checks=10132 source_invariant_rows=32292 source_only_rows=23089"
    );
}
