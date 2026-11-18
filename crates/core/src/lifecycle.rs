// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

/// Persisted organism energy costs and frame settings
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LifecycleParams {
    pub segment_cost_divisor: i32,
    pub healing: i32,
    pub immune_system: i32,
    pub rubbing: f64,
    pub cyan_energy_consumption: f64,
    pub teal_energy_consumption: f64,
    pub yellow_energy_consumption: f64,
    pub sky_energy_consumption: f64,
    pub fallow_energy_consumption: f64,
    pub magenta_energy_consumption: f64,
    pub dark_gray_energy_consumption: f64,
    pub organic_obtained_energy: f64,
    pub coral_energy_consumption: f64,
    pub dodge_energy_consumption: f64,
    pub indigo_energy_consumption: f64,
    pub lavender_energy_consumption: f64,
    pub orange_energy_consumption: f64,
    pub fire_energy_consumption: f64,
    pub red_energy_consumption: f64,
    pub pink_energy_consumption: f64,
    pub maroon_energy_consumption: f64,
    pub blue_energy_consumption: f64,
    pub organic_subs_produced: f64,
    pub cream_energy_consumption: f64,
    pub lilac_energy_consumption: f64,
    pub silver_energy_consumption: f64,
    pub mosquito_energy_consumption: f64,
    pub ochre_energy_consumption: f64,
    pub experience_energy_consumption: f64,
    pub spike_energy_consumption: f64,
    pub white_energy_consumption: f64,
    pub virus_energy_consumption: f64,
    pub plague_energy_consumption: f64,
    pub scourge_energy_consumption: f64,
    pub violet_energy_consumption: f64,
    pub olive_energy_consumption: f64,
    pub gray_energy_consumption: f64,
    pub cream_organic_subs_produced: f64,
    pub crowded_forest_energy_consumption: f64,
    pub crowded_lime_energy_consumption: f64,
    pub mint_energy_consumption: f64,
    pub rose_energy_consumption: f64,
    pub dark_energy_consumption: f64,
    pub blond_energy_consumption: f64,
    pub flower_energy_consumption: f64,
    pub auburn_energy_consumption: f64,
    pub eye_energy_consumption: f64,
    pub decay_energy: f64,
}

impl Default for LifecycleParams {
    fn default() -> Self {
        Self {
            segment_cost_divisor: 5000,
            healing: 40_000,
            immune_system: 40_000,
            rubbing: 0.98,
            cyan_energy_consumption: 0.0,
            teal_energy_consumption: 0.0,
            yellow_energy_consumption: 0.0,
            sky_energy_consumption: 0.05,
            fallow_energy_consumption: 0.0,
            magenta_energy_consumption: 0.01,
            dark_gray_energy_consumption: 10.0,
            organic_obtained_energy: 2.0,
            coral_energy_consumption: 0.01,
            dodge_energy_consumption: 0.0,
            indigo_energy_consumption: 1.0,
            lavender_energy_consumption: 1.0,
            orange_energy_consumption: 0.0,
            fire_energy_consumption: 0.0,
            red_energy_consumption: 0.0,
            pink_energy_consumption: 0.0,
            maroon_energy_consumption: 0.0,
            blue_energy_consumption: 0.0,
            organic_subs_produced: 0.1,
            cream_energy_consumption: 0.0,
            lilac_energy_consumption: 0.0,
            silver_energy_consumption: 0.1,
            mosquito_energy_consumption: 0.0,
            ochre_energy_consumption: 0.4,
            experience_energy_consumption: 0.0,
            spike_energy_consumption: 0.0,
            white_energy_consumption: 0.1,
            virus_energy_consumption: 0.01,
            plague_energy_consumption: 10.0,
            scourge_energy_consumption: 0.1,
            violet_energy_consumption: 0.1,
            olive_energy_consumption: 0.1,
            gray_energy_consumption: 0.1,
            cream_organic_subs_produced: 0.0125,
            crowded_forest_energy_consumption: 1.0,
            crowded_lime_energy_consumption: 0.5,
            mint_energy_consumption: 0.075,
            rose_energy_consumption: 0.01,
            dark_energy_consumption: 0.01,
            blond_energy_consumption: 0.037,
            flower_energy_consumption: 0.01,
            auburn_energy_consumption: 0.0,
            eye_energy_consumption: 0.0,
            decay_energy: 0.1,
        }
    }
}

impl LifecycleParams {
    /// Apply field ranges, including the source positive-Infinity behavior
    /// Java preferences bypass dialog validation and can contain values rejected here
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.segment_cost_divisor <= 0 || self.healing <= 0 || self.immune_system <= 0 {
            return Err("Lifecycle divisors and random bounds must be positive");
        }
        if [
            self.rubbing,
            self.organic_subs_produced,
            self.cream_organic_subs_produced,
        ]
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err("Rubbing and organic byproducts must be finite and between zero and one");
        }
        if [
            self.cyan_energy_consumption,
            self.teal_energy_consumption,
            self.yellow_energy_consumption,
            self.sky_energy_consumption,
            self.fallow_energy_consumption,
            self.magenta_energy_consumption,
            self.dark_gray_energy_consumption,
            self.organic_obtained_energy,
            self.coral_energy_consumption,
            self.dodge_energy_consumption,
            self.orange_energy_consumption,
            self.fire_energy_consumption,
            self.red_energy_consumption,
            self.pink_energy_consumption,
            self.maroon_energy_consumption,
            self.blue_energy_consumption,
            self.cream_energy_consumption,
            self.lilac_energy_consumption,
            self.silver_energy_consumption,
            self.mosquito_energy_consumption,
            self.ochre_energy_consumption,
            self.experience_energy_consumption,
            self.spike_energy_consumption,
            self.white_energy_consumption,
            self.virus_energy_consumption,
            self.plague_energy_consumption,
            self.scourge_energy_consumption,
            self.violet_energy_consumption,
            self.olive_energy_consumption,
            self.gray_energy_consumption,
            self.mint_energy_consumption,
            self.dark_energy_consumption,
            self.blond_energy_consumption,
            self.auburn_energy_consumption,
            self.eye_energy_consumption,
            self.decay_energy,
        ]
        .iter()
        .any(|value| !(*value >= 0.0))
        {
            return Err("Lifecycle energy consumption must be nonnegative");
        }
        if [
            self.crowded_forest_energy_consumption,
            self.crowded_lime_energy_consumption,
            self.rose_energy_consumption,
            self.flower_energy_consumption,
            self.lavender_energy_consumption,
        ]
        .iter()
        .any(|value| !(*value > 0.0))
        {
            return Err(
                "Crowded photosynthesis, rose, flower, and lavender coefficients must be positive",
            );
        }
        if !(self.indigo_energy_consumption >= 1.0) {
            return Err("Indigo energy consumption must be at least one");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn defaults_match_actual_java_settings_observations() {
        let fixture = include_str!("../tests/fixtures/organism_state.tsv");
        assert_eq!(
            fixture.lines().next(),
            Some("C\tconstructor-only\tconstructor\t827364")
        );
        let mut settings = BTreeMap::new();
        for line in fixture.lines().take_while(|line| *line != "E") {
            let columns: Vec<_> = line.split('\t').collect();
            if columns[0] == "U" {
                assert_eq!(columns.len(), 4);
                assert!(
                    settings
                        .insert(columns[1], (columns[2], columns[3]))
                        .is_none()
                );
            }
        }
        let parameters = LifecycleParams::default();
        for (name, actual) in [
            ("SEGMENT_COST_DIVISOR", parameters.segment_cost_divisor),
            ("HEALING", parameters.healing),
            ("IMMUNE_SYSTEM", parameters.immune_system),
        ] {
            let (kind, expected) = settings[name];
            assert_eq!(kind, "int", "{name}");
            assert_eq!(actual, expected.parse::<i32>().unwrap(), "{name}");
        }
        for (name, actual) in [
            ("RUBBING", parameters.rubbing),
            ("ORGANIC_SUBS_PRODUCED", parameters.organic_subs_produced),
            (
                "CREAM_ORGANIC_SUBS_PRODUCED",
                parameters.cream_organic_subs_produced,
            ),
            (
                "CREAM_ENERGY_CONSUMPTION",
                parameters.cream_energy_consumption,
            ),
            (
                "LILAC_ENERGY_CONSUMPTION",
                parameters.lilac_energy_consumption,
            ),
            (
                "SILVER_ENERGY_CONSUMPTION",
                parameters.silver_energy_consumption,
            ),
            (
                "MOSQUITO_ENERGY_CONSUMPTION",
                parameters.mosquito_energy_consumption,
            ),
            (
                "OCHRE_ENERGY_CONSUMPTION",
                parameters.ochre_energy_consumption,
            ),
            (
                "EXPERIENCE_ENERGY_CONSUMPTION",
                parameters.experience_energy_consumption,
            ),
            (
                "SPIKE_ENERGY_CONSUMPTION",
                parameters.spike_energy_consumption,
            ),
            (
                "WHITE_ENERGY_CONSUMPTION",
                parameters.white_energy_consumption,
            ),
            (
                "VIRUS_ENERGY_CONSUMPTION",
                parameters.virus_energy_consumption,
            ),
            (
                "PLAGUE_ENERGY_CONSUMPTION",
                parameters.plague_energy_consumption,
            ),
            (
                "SCOURGE_ENERGY_CONSUMPTION",
                parameters.scourge_energy_consumption,
            ),
            (
                "VIOLET_ENERGY_CONSUMPTION",
                parameters.violet_energy_consumption,
            ),
            (
                "OLIVE_ENERGY_CONSUMPTION",
                parameters.olive_energy_consumption,
            ),
            (
                "GRAY_ENERGY_CONSUMPTION",
                parameters.gray_energy_consumption,
            ),
            (
                "ORANGE_ENERGY_CONSUMPTION",
                parameters.orange_energy_consumption,
            ),
            (
                "FIRE_ENERGY_CONSUMPTION",
                parameters.fire_energy_consumption,
            ),
            ("RED_ENERGY_CONSUMPTION", parameters.red_energy_consumption),
            (
                "PINK_ENERGY_CONSUMPTION",
                parameters.pink_energy_consumption,
            ),
            (
                "MAROON_ENERGY_CONSUMPTION",
                parameters.maroon_energy_consumption,
            ),
            (
                "BLUE_ENERGY_CONSUMPTION",
                parameters.blue_energy_consumption,
            ),
            (
                "CYAN_ENERGY_CONSUMPTION",
                parameters.cyan_energy_consumption,
            ),
            (
                "TEAL_ENERGY_CONSUMPTION",
                parameters.teal_energy_consumption,
            ),
            (
                "YELLOW_ENERGY_CONSUMPTION",
                parameters.yellow_energy_consumption,
            ),
            ("SKY_ENERGY_CONSUMPTION", parameters.sky_energy_consumption),
            (
                "FALLOW_ENERGY_CONSUMPTION",
                parameters.fallow_energy_consumption,
            ),
            (
                "MAGENTA_ENERGY_CONSUMPTION",
                parameters.magenta_energy_consumption,
            ),
            (
                "DARKGRAY_ENERGY_CONSUMPTION",
                parameters.dark_gray_energy_consumption,
            ),
            (
                "ORGANIC_OBTAINED_ENERGY",
                parameters.organic_obtained_energy,
            ),
            (
                "CORAL_ENERGY_CONSUMPTION",
                parameters.coral_energy_consumption,
            ),
            (
                "DODGE_ENERGY_CONSUMPTION",
                parameters.dodge_energy_consumption,
            ),
            (
                "INDIGO_ENERGY_CONSUMPTION",
                parameters.indigo_energy_consumption,
            ),
            (
                "LAVENDER_ENERGY_CONSUMPTION",
                parameters.lavender_energy_consumption,
            ),
            (
                "CROWDEDFOREST_ENERGY_CONSUMPTION",
                parameters.crowded_forest_energy_consumption,
            ),
            (
                "CROWDEDLIME_ENERGY_CONSUMPTION",
                parameters.crowded_lime_energy_consumption,
            ),
            (
                "MINT_ENERGY_CONSUMPTION",
                parameters.mint_energy_consumption,
            ),
            (
                "ROSE_ENERGY_CONSUMPTION",
                parameters.rose_energy_consumption,
            ),
            (
                "DARK_ENERGY_CONSUMPTION",
                parameters.dark_energy_consumption,
            ),
            (
                "BLOND_ENERGY_CONSUMPTION",
                parameters.blond_energy_consumption,
            ),
            (
                "FLOWER_ENERGY_CONSUMPTION",
                parameters.flower_energy_consumption,
            ),
            (
                "AUBURN_ENERGY_CONSUMPTION",
                parameters.auburn_energy_consumption,
            ),
            ("EYE_ENERGY_CONSUMPTION", parameters.eye_energy_consumption),
            ("DECAY_ENERGY", parameters.decay_energy),
        ] {
            let (kind, expected) = settings[name];
            assert_eq!(kind, "double", "{name}");
            assert_eq!(
                actual.to_bits(),
                u64::from_str_radix(expected, 16).unwrap(),
                "{name}"
            );
        }
        assert_eq!(parameters.validate(), Ok(()));
    }

    #[test]
    fn divisor_and_random_bounds_accept_full_positive_java_integer_range() {
        let fields: [fn(&mut LifecycleParams) -> &mut i32; 3] = [
            |p| &mut p.segment_cost_divisor,
            |p| &mut p.healing,
            |p| &mut p.immune_system,
        ];
        for field in fields {
            for value in [i32::MIN, -1, 0, 1, i32::MAX] {
                let mut parameters = LifecycleParams::default();
                *field(&mut parameters) = value;
                assert_eq!(parameters.validate().is_ok(), value > 0, "{parameters:?}");
            }
        }
    }

    #[test]
    fn zero_cost_actions_preserve_source_nonnegative_boundaries() {
        let fields: [fn(&mut LifecycleParams) -> &mut f64; 36] = [
            |p| &mut p.cyan_energy_consumption,
            |p| &mut p.teal_energy_consumption,
            |p| &mut p.yellow_energy_consumption,
            |p| &mut p.sky_energy_consumption,
            |p| &mut p.fallow_energy_consumption,
            |p| &mut p.magenta_energy_consumption,
            |p| &mut p.dark_gray_energy_consumption,
            |p| &mut p.organic_obtained_energy,
            |p| &mut p.coral_energy_consumption,
            |p| &mut p.dodge_energy_consumption,
            |p| &mut p.orange_energy_consumption,
            |p| &mut p.fire_energy_consumption,
            |p| &mut p.red_energy_consumption,
            |p| &mut p.pink_energy_consumption,
            |p| &mut p.maroon_energy_consumption,
            |p| &mut p.blue_energy_consumption,
            |p| &mut p.cream_energy_consumption,
            |p| &mut p.lilac_energy_consumption,
            |p| &mut p.silver_energy_consumption,
            |p| &mut p.mosquito_energy_consumption,
            |p| &mut p.ochre_energy_consumption,
            |p| &mut p.experience_energy_consumption,
            |p| &mut p.spike_energy_consumption,
            |p| &mut p.white_energy_consumption,
            |p| &mut p.virus_energy_consumption,
            |p| &mut p.plague_energy_consumption,
            |p| &mut p.scourge_energy_consumption,
            |p| &mut p.violet_energy_consumption,
            |p| &mut p.olive_energy_consumption,
            |p| &mut p.gray_energy_consumption,
            |p| &mut p.mint_energy_consumption,
            |p| &mut p.dark_energy_consumption,
            |p| &mut p.blond_energy_consumption,
            |p| &mut p.auburn_energy_consumption,
            |p| &mut p.eye_energy_consumption,
            |p| &mut p.decay_energy,
        ];
        for field in fields {
            for value in [-0.0, 0.0, f64::from_bits(1), f64::MAX, f64::INFINITY] {
                let mut parameters = LifecycleParams::default();
                *field(&mut parameters) = value;
                assert_eq!(parameters.validate(), Ok(()), "{parameters:?}");
                assert_eq!(field(&mut parameters).to_bits(), value.to_bits());
            }
            for value in [-f64::from_bits(1), f64::NAN, f64::NEG_INFINITY] {
                let mut parameters = LifecycleParams::default();
                *field(&mut parameters) = value;
                assert!(parameters.validate().is_err(), "{parameters:?}");
            }
        }
    }

    #[test]
    fn positive_coefficients_reject_zero_and_accept_smallest_positive_values() {
        let fields: [fn(&mut LifecycleParams) -> &mut f64; 5] = [
            |p| &mut p.crowded_forest_energy_consumption,
            |p| &mut p.crowded_lime_energy_consumption,
            |p| &mut p.rose_energy_consumption,
            |p| &mut p.flower_energy_consumption,
            |p| &mut p.lavender_energy_consumption,
        ];
        for field in fields {
            for value in [f64::from_bits(1), f64::MAX, f64::INFINITY] {
                let mut parameters = LifecycleParams::default();
                *field(&mut parameters) = value;
                assert_eq!(parameters.validate(), Ok(()), "{parameters:?}");
            }
            for value in [-1.0, -0.0, 0.0, f64::NAN, f64::NEG_INFINITY] {
                let mut parameters = LifecycleParams::default();
                *field(&mut parameters) = value;
                assert!(parameters.validate().is_err(), "{parameters:?}");
            }
        }
    }

    #[test]
    fn bounded_coefficients_accept_closed_unit_interval_without_clamping() {
        let fields: [fn(&mut LifecycleParams) -> &mut f64; 3] = [
            |p| &mut p.rubbing,
            |p| &mut p.organic_subs_produced,
            |p| &mut p.cream_organic_subs_produced,
        ];
        for field in fields {
            for value in [-0.0, 0.0, 1.0] {
                let mut parameters = LifecycleParams::default();
                *field(&mut parameters) = value;
                assert_eq!(parameters.validate(), Ok(()));
                assert_eq!(field(&mut parameters).to_bits(), value.to_bits());
            }
            for value in [
                -f64::from_bits(1),
                f64::from_bits(1.0_f64.to_bits() + 1),
                f64::NAN,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ] {
                let mut parameters = LifecycleParams::default();
                *field(&mut parameters) = value;
                assert!(parameters.validate().is_err());
            }
        }
    }

    #[test]
    fn indigo_consumption_preserves_the_inclusive_source_dialog_minimum() {
        for value in [1.0, f64::MAX, f64::INFINITY] {
            let parameters = LifecycleParams {
                indigo_energy_consumption: value,
                ..LifecycleParams::default()
            };
            assert_eq!(parameters.validate(), Ok(()));
        }
        for value in [
            f64::from_bits(1.0_f64.to_bits() - 1),
            0.0,
            -1.0,
            f64::NAN,
            f64::NEG_INFINITY,
        ] {
            let parameters = LifecycleParams {
                indigo_energy_consumption: value,
                ..LifecycleParams::default()
            };
            assert!(parameters.validate().is_err());
        }
    }
}
