// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::World;

fn edit(draft: &mut SettingsDraft, key: SettingKey, value: &str) {
    draft.set_editor(key, value.to_owned()).unwrap();
}

#[test]
fn locale_precedes_independent_numeric_assignments_and_defaults_remain_a_draft() {
    let mut state = SettingsState::new("en".to_owned());
    state
        .assign(SettingKey::MaxMutationRate, SettingValue::Int(7))
        .unwrap();
    state
        .assign(SettingKey::MinMutationRate, SettingValue::Int(7))
        .unwrap();
    let original = state.clone();
    let mut cancelled = SettingsDraft::new(&state);
    cancelled.defaults().unwrap();
    cancelled.cancel().unwrap();
    assert_eq!(state, original);
    assert_eq!(cancelled.defaults(), Err(SettingsError::ClosedDraft));
    let mut draft = SettingsDraft::new(&state);
    draft
        .set_choices(3, true, HardwareSelection::OpenGl, false)
        .unwrap();
    edit(&mut draft, SettingKey::WorldWidth, "11");
    edit(&mut draft, SettingKey::MaxMutationRate, "3");
    edit(&mut draft, SettingKey::MinMutationRate, "6");
    edit(&mut draft, SettingKey::BlueEnergyConsumption, "0.125");
    let mut callbacks = Vec::new();
    let report = draft
        .ok(&mut state, |before_numeric| {
            callbacks.push(before_numeric.clone())
        })
        .unwrap();
    assert_eq!(callbacks.len(), 1);
    assert_eq!(callbacks[0].language(), "de");
    assert_eq!(callbacks[0].genesis().width, original.genesis().width);
    assert_eq!(callbacks[0].parameters().genetics.max_mutation_rate, 7);
    assert_eq!(state.genesis().width, 11);
    assert_eq!(state.parameters().genetics.max_mutation_rate, 3);
    assert_eq!(state.parameters().genetics.min_mutation_rate, 7);
    assert_eq!(state.parameters().lifecycle.blue_energy_consumption, 0.125);
    assert_eq!(
        state.parameters().lifecycle.red_energy_consumption,
        original.parameters().lifecycle.red_energy_consumption
    );
    assert_eq!(
        state.get(SettingKey::AutoBackup),
        SettingValue::Boolean(true)
    );
    assert_eq!(
        state.get(SettingKey::HardwareAcceleration),
        SettingValue::Int(0)
    );
    let min = NUMERIC_EDITOR_KEYS
        .iter()
        .position(|key| *key == SettingKey::MinMutationRate)
        .unwrap();
    assert_eq!(report.editors[min], EditorOutcome::OutsideRange);
}

#[test]
fn settings_lexemes_have_single_line_editing_without_the_xml_length_cap() {
    let mut state = SettingsState::new("en".to_owned());
    let mut draft = SettingsDraft::new(&state);
    edit(
        &mut draft,
        SettingKey::WorldWidth,
        &("0".repeat(5000) + "8"),
    );
    edit(&mut draft, SettingKey::Rubbing, " \t+0.5\n");
    let rubbing = NUMERIC_EDITOR_KEYS
        .iter()
        .position(|key| *key == SettingKey::Rubbing)
        .unwrap();
    assert_eq!(draft.editors()[rubbing], " \t+0.5 ");
    draft
        .ok(&mut state, |_| panic!("Unchanged locale"))
        .unwrap();
    assert_eq!(state.genesis().width, 8);
    assert_eq!(
        state.parameters().lifecycle.rubbing.to_bits(),
        0.5_f64.to_bits()
    );
}

#[test]
fn sixty_source_unbounded_doubles_reach_their_responsible_parameter_owners() {
    let mut accepted = 0;
    let mut bounded = 0;
    for key in NUMERIC_EDITOR_KEYS {
        if !matches!(
            SettingsState::new("en".to_owned()).get(key),
            SettingValue::Double(_)
        ) {
            continue;
        }
        let mut state = SettingsState::new("en".to_owned());
        let old = state.get(key);
        let mut draft = SettingsDraft::new(&state);
        edit(&mut draft, key, "Infinity");
        draft
            .ok(&mut state, |_| panic!("Unchanged locale"))
            .unwrap();
        if state.accepts(key, &SettingValue::Double(f64::INFINITY)) {
            accepted += 1;
            assert_eq!(state.get(key), SettingValue::Double(f64::INFINITY));
            let config = WorldConfig {
                initial_attempts: 0,
                ..state.world_config(827364).unwrap()
            };
            World::with_parameters(config, state.parameters().clone()).unwrap();
        } else {
            bounded += 1;
            assert_eq!(state.get(key), old);
        }
    }
    assert_eq!((accepted, bounded), (60, 4));
    for max_velocity in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut parameters = WorldParameters::default();
        parameters.physics.max_velocity = max_velocity;
        assert!(
            World::with_parameters(
                WorldConfig {
                    initial_attempts: 0,
                    ..WorldConfig::default()
                },
                parameters
            )
            .is_err()
        );
    }
}

#[test]
fn pending_genesis_preserves_caller_seed_and_defers_world_resource_limits() {
    let mut state = SettingsState::new("zz".to_owned());
    assert_eq!(
        state.genesis(),
        GenesisSettings {
            width: 7000,
            height: 7000,
            initial_attempts: 12500
        }
    );
    let mut draft = SettingsDraft::new(&state);
    edit(&mut draft, SettingKey::InitialOrganisms, "2147483647");
    draft
        .ok(&mut state, |_| panic!("Fallback locale index unchanged"))
        .unwrap();
    assert_eq!(state.language(), "zz");
    let config = state.world_config(-123).unwrap();
    assert_eq!(config.seed, -123);
    assert_eq!(config.initial_attempts, i32::MAX as u32);
    let config = crate::WorldConfig { width: 1, height: 1, ..config };
    assert!(matches!(
        World::with_parameters(config, state.parameters().clone()),
        Err(crate::WorldError::BodyDoesNotFit { .. })
    ));
}
