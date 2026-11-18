// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::java_math::min;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphereParams {
    pub initial_oxygen: f64,
    pub initial_carbon_dioxide: f64,
    pub initial_methane: f64,
    pub carbon_to_methane_divisor: i32,
    pub methane_to_carbon_divisor: i32,
    pub drain_divisor: i32,
}

impl Default for AtmosphereParams {
    fn default() -> Self {
        Self {
            initial_oxygen: 0.0,
            initial_carbon_dioxide: 500_000.0,
            initial_methane: 0.0,
            carbon_to_methane_divisor: 999,
            methane_to_carbon_divisor: 1000,
            drain_divisor: 5000,
        }
    }
}

impl AtmosphereParams {
    pub fn validate(&self) -> Result<(), &'static str> {
        if [
            self.initial_oxygen,
            self.initial_carbon_dioxide,
            self.initial_methane,
        ]
        .iter()
        .any(|amount| !(*amount >= 0.0))
        {
            return Err("Initial atmospheric quantities must be nonnegative");
        }
        if self.carbon_to_methane_divisor <= 0
            || self.methane_to_carbon_divisor <= 0
            || self.drain_divisor <= 0
        {
            return Err("Atmospheric divisors must be positive");
        }
        Ok(())
    }
}

pub(crate) fn take(available: &mut f64, requested: f64) -> f64 {
    let obtained = min(requested, *available);
    *available = crate::java_math::sub(*available, obtained);
    obtained
}

pub(crate) fn transfer(source: &mut f64, target: &mut f64, requested: f64) -> f64 {
    let obtained = take(source, requested);
    *target = crate::java_math::add(*target, obtained);
    obtained
}

pub(crate) fn obtain(source: &mut f64, oxygen: &mut f64, requested: f64, divisor: i32) -> f64 {
    let available_rate = min(requested, crate::java_math::div(crate::java_math::mul(requested, *source), f64::from(divisor)));
    transfer(source, oxygen, available_rate)
}

pub(crate) fn transfer_cells(
    source: &std::cell::Cell<f64>, target: &std::cell::Cell<f64>, requested: f64,
) -> f64 {
    let obtained = min(requested, source.get());
    source.set(crate::java_math::sub(source.get(), obtained));
    target.set(crate::java_math::add(target.get(), obtained));
    obtained
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{World, WorldConfig, WorldParameters};

    fn decode(value: &str) -> f64 {
        f64::from_bits(u64::from_str_radix(value, 16).unwrap())
    }

    #[test]
    fn all_recorded_java_resource_boundaries_preserve_exact_amounts() {
        let mut cases = 0;
        for row in include_str!("../tests/fixtures/gases.tsv").lines() {
            let columns: Vec<_> = row.split('\t').collect();
            assert_eq!(columns.len(), 9);
            let mut parameters = WorldParameters::default();
            parameters.atmosphere.initial_oxygen = decode(columns[2]);
            parameters.atmosphere.initial_carbon_dioxide = decode(columns[3]);
            parameters.atmosphere.initial_methane = decode(columns[4]);
            let world = World::with_parameters(
                WorldConfig {
                    initial_attempts: 0,
                    ..WorldConfig::default()
                },
                parameters,
            )
            .unwrap();
            let requested = decode(columns[1]);
            let result = match columns[0] {
                "respiration" => Some(world.respiration(requested)),
                "decomposition" => Some(world.decomposition(requested)),
                "photosynthesis" => Some(world.photosynthesis(requested)),
                "methanotrophy" => Some(world.methanotrophy(requested)),
                "decreaseO2" => {
                    world.decrease_oxygen(requested);
                    None
                }
                "decreaseCO2" => {
                    world.decrease_carbon_dioxide(requested);
                    None
                }
                "decreaseCH4" => {
                    world.decrease_methane(requested);
                    None
                }
                name => panic!("Unknown gas observation {name}"),
            };
            assert_eq!(
                result.map(f64::to_bits),
                if columns[5] == "~" {
                    None
                } else {
                    Some(decode(columns[5]).to_bits())
                }
            );
            assert_eq!(
                [world.oxygen(), world.carbon_dioxide(), world.methane()].map(f64::to_bits),
                [decode(columns[6]), decode(columns[7]), decode(columns[8])].map(f64::to_bits),
                "{}",
                columns[0]
            );
            cases += 1;
        }
        assert_eq!(cases, 22);
    }

    #[test]
    fn configured_gases_and_ordered_divisors_survive_reset() {
        let mut parameters = WorldParameters::default();
        parameters.atmosphere = AtmosphereParams {
            initial_oxygen: 3.0,
            initial_carbon_dioxide: 80.0,
            initial_methane: 0.0,
            carbon_to_methane_divisor: 4,
            methane_to_carbon_divisor: 2,
            drain_divisor: 10,
        };
        let mut world = World::with_parameters(
            WorldConfig {
                initial_attempts: 0,
                ..WorldConfig::default()
            },
            parameters,
        )
        .unwrap();
        world.single_step().unwrap();
        assert_eq!(
            [world.oxygen(), world.carbon_dioxide(), world.methane()],
            [3.0, 70.0, 10.0]
        );
        assert_eq!(world.photosynthesis(10.0), 10.0);
        assert_eq!(world.respiration(100.0), 13.0);
        world.reset().unwrap();
        assert_eq!(
            [world.oxygen(), world.carbon_dioxide(), world.methane()],
            [3.0, 80.0, 0.0]
        );
        for invalid in [0, -1, i32::MIN] {
            let mut invalid_parameters = WorldParameters::default();
            invalid_parameters.atmosphere.drain_divisor = invalid;
            assert!(World::with_parameters(WorldConfig::default(), invalid_parameters).is_err());
        }
        let mut invalid_parameters = WorldParameters::default();
        invalid_parameters.atmosphere.initial_oxygen = f64::NAN;
        assert!(World::with_parameters(WorldConfig::default(), invalid_parameters).is_err());
    }
}
