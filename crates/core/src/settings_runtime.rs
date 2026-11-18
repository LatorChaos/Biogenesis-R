// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::SettingsState;

impl World {
    /// Project accepted settings without rebuilding resources or cached organism state
    pub fn apply_settings_parameters(&self, settings: &SettingsState) {
        self.session.apply_settings_parameters(settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorKind, Gene, GenomeProperties, SettingKey, SettingsDraft};
    use std::num::NonZeroUsize;

    fn world() -> World {
        let mut parameters = WorldParameters::default();
        parameters.outgoing = Some(OutgoingConfig {
            user_name: "runtime".into(),
            pending_per_connection: NonZeroUsize::new(2).unwrap(),
        });
        let world = World::with_parameters(
            WorldConfig {
                seed: 827364,
                initial_attempts: 0,
                ..WorldConfig::default()
            },
            parameters,
        )
        .unwrap();
        let code = Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        world.paste_genome(code, [100, 100]).unwrap().unwrap();
        world.single_step().unwrap();
        world.resume();
        world
    }

    #[test]
    fn runtime_projection_preserves_every_nonparameter_field_and_actual_member_handles() {
        let world = world();
        let before = world.clone();
        let members = world.organisms.borrow().clone();
        let frozen = world.frame_objects.borrow().clone();
        let mut settings = SettingsState::new("en".to_owned());
        let mut draft = SettingsDraft::new(&settings);
        for (key, text) in [
            (SettingKey::BlueEnergyConsumption, "0.125"),
            (SettingKey::InitialO2, "4321"),
            (SettingKey::WorldWidth, "1234"),
            (SettingKey::MaxAge, "201"),
        ] {
            draft.set_editor(key, text.to_owned()).unwrap();
        }
        draft
            .ok(&mut settings, |_| panic!("Locale unchanged"))
            .unwrap();
        world.apply_settings_parameters(&settings);
        assert_eq!(world.parameters().outgoing, before.parameters().outgoing);
        assert_eq!(world.parameters().lifecycle.blue_energy_consumption, 0.125);
        assert_eq!(world.parameters().atmosphere.initial_oxygen, 4321.0);
        assert_eq!(world.parameters().effects.max_age, 201);
        assert_eq!(world.config.get().width, before.config.get().width);
        let expected = before;
        expected.session.0.parameters.replace(world.parameters().clone());
        assert_eq!(world, expected);
        for (old, current) in members.iter().zip(world.organisms.borrow().iter()) {
            assert!(Rc::ptr_eq(old, current));
        }
        for (old, current) in frozen.iter().zip(world.frame_objects.borrow().iter()) {
            assert!(Rc::ptr_eq(old, current));
        }
    }

    #[test]
    fn accepted_partial_bounds_fail_safely_before_genesis_without_repairing_settings() {
        let world = world();
        let handle = world.pick_organism([100, 100]).unwrap().unwrap();
        let mut settings = SettingsState::new("en".to_owned());
        let mut draft = SettingsDraft::new(&settings);
        draft
            .set_editor(SettingKey::MaxMutationRate, "7".to_owned())
            .unwrap();
        draft
            .set_editor(SettingKey::MinMutationRate, "7".to_owned())
            .unwrap();
        draft
            .ok(&mut settings, |_| panic!("Locale unchanged"))
            .unwrap();
        let mut draft = SettingsDraft::new(&settings);
        draft
            .set_editor(SettingKey::MaxMutationRate, "3".to_owned())
            .unwrap();
        draft
            .set_editor(SettingKey::MinMutationRate, "6".to_owned())
            .unwrap();
        draft
            .ok(&mut settings, |_| panic!("Locale unchanged"))
            .unwrap();
        world.apply_settings_parameters(&settings);
        assert_eq!(world.parameters().genetics.max_mutation_rate, 3);
        assert_eq!(world.parameters().genetics.min_mutation_rate, 7);
        let before = world.clone();
        let failure = world
            .new_game(1234, 2345, 0, world.parameters().clone())
            .unwrap_err();
        assert!(!failure.entered_genesis);
        assert!(matches!(failure.error, WorldError::InvalidParameters(_)));
        assert_eq!(world, before);
        assert!(world.inspect_handle(&handle).is_some());
        assert!(Rc::ptr_eq(&handle.0, &world.organisms.borrow()[0]));
        assert_eq!(settings.parameters().genetics.min_mutation_rate, 7);
    }
}
