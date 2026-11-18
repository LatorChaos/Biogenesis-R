// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Successful birth overrides from Organism.java:2068-2494 and dormant constructors at 4932-5002

use crate::organism::primary_genome;

use crate::organism::Organism;
use crate::{ColorKind, EffectsParams, GeometryError, JavaRandom};

#[derive(Clone, Copy)]
enum SporeOrigin<'a> {
    Ordinary(&'a Organism),
    Transformation,
}

impl SporeOrigin<'_> {
    fn matches(self, version: i32) -> bool {
        match self {
            Self::Ordinary(parent) => parent.effects.spore_version == version,
            Self::Transformation => true,
        }
    }

    fn disperses(self) -> Result<bool, GeometryError> {
        Ok(match self {
            Self::Ordinary(parent) => primary_genome(&parent.genome)?.borrow().modifiers().disperse_children,
            Self::Transformation => false,
        })
    }
}

/// Apply Organism.inherit's successful-birth overrides after its ordinary effects initialization
pub(crate) fn initialize_birth_spore(
    child: &mut Organism,
    parent: &Organism,
    random: &mut JavaRandom,
    parameters: &EffectsParams,
    spore_energy_consumption: f64,
) -> Result<(), crate::WorldError> {
    initialize_spore(
        child,
        SporeOrigin::Ordinary(parent),
        random,
        parameters,
        spore_energy_consumption,
    )
}

/// Apply transformation overrides without ordinary birth's parent-version or dispersal conditions
pub(crate) fn initialize_transformation_spore(
    child: &mut Organism,
    random: &mut JavaRandom,
    parameters: &EffectsParams,
    spore_energy_consumption: f64,
) -> Result<(), crate::WorldError> {
    initialize_spore(
        child,
        SporeOrigin::Transformation,
        random,
        parameters,
        spore_energy_consumption,
    )
}

fn initialize_spore(
    child: &mut Organism,
    origin: SporeOrigin<'_>,
    random: &mut JavaRandom,
    parameters: &EffectsParams,
    spore_energy_consumption: f64,
) -> Result<(), crate::WorldError> {
    if child.effects.spore_time <= 0 {
        return Ok(());
    }
    let version = child.effects.spore_version;
    match version {
        1 | 2 => {
            if !origin.matches(version) {
                return Ok(());
            }
            // A partial first payment remains spent before the fallback requests the remaining energy
            if child.use_energy(spore_energy_consumption)?
                || child.use_energy(child.state.energy)?
            {
                replace_segments(
                    child,
                    ColorKind::Fruit,
                    if version == 1 { -2.0 } else { 0.0 },
                )?;
            }
            make_inactive(child);
            child.effects.is_enhanced = false;
            if version == 1 {
                if !child.effects.is_plant && !child.effects.is_consumer && !child.effects.is_fungus
                {
                    child.effects.is_enhanced = true;
                }
            } else if child.effects.is_plant
                && !child.effects.is_fungus
                && !child.effects.is_consumer
            {
                child.effects.is_enhanced = true;
                child.state.reproduction.maximum_age = child.effects.spore_time.min(20);
            } else {
                child.state.reproduction.maximum_age =
                    child.effects.spore_time.wrapping_sub(1).min(19);
            }
            clear_classification(child);
            remove_eyes(child)?;
        }
        3 => {
            if !origin.matches(3) {
                return Ok(());
            }
            let consumer = child.effects.is_consumer || child.effects.is_fungus;
            if origin.disperses()? {
                let multiplier = if (child.effects.is_plant && consumer) || child.effects.can_move {
                    2.0
                } else {
                    3.0
                };
                child.state.velocity[0] = multiplier * child.state.velocity[0];
                child.state.velocity[1] = multiplier * child.state.velocity[1];
            }
            child.effects.is_enhanced = !consumer;
            let color = if child.effects.is_plant || !consumer {
                ColorKind::Bark
            } else {
                ColorKind::Vision
            };
            replace_segments(child, color, 0.0)?;
            child.state.reproduction.countdown = 104_i32.wrapping_sub(child.effects.spore_time);
            make_inactive(child);
            clear_classification(child);
            remove_eyes(child)?;
        }
        4 => {
            if !origin.matches(4) {
                return Ok(());
            }
            replace_segments(child, ColorKind::Brown, 0.0)?;
            if child.effects.is_consumer || child.effects.is_fungus {
                child.state.reproduction.countdown = 115_i32
                    .wrapping_sub(child.effects.spore_time)
                    .wrapping_add(random.bounded(2));
                child.effects.is_enhanced = false;
            } else {
                child.state.reproduction.countdown = 104_i32.wrapping_sub(child.effects.spore_time);
                child.effects.is_enhanced = true;
            }
            make_inactive(child);
            clear_classification(child);
            remove_eyes(child)?;
        }
        5 => {
            if !origin.matches(5) {
                return Ok(());
            }
            replace_segments(child, ColorKind::Cream, -1.0)?;
            make_inactive(child);
            child.effects.is_plant = false;
            child.effects.is_consumer = true;
            child.effects.is_fungus = false;
            child.effects.is_enhanced = false;
            remove_eyes(child)?;
        }
        6 => {
            if origin.matches(6) {
                initialize_photosynthetic_spore(child, parameters)?;
            }
        }
        _ => {
            let modifiers = primary_genome(&child.genome)?.borrow().modifiers();
            if modifiers.spore > 12 {
                return Ok(());
            }
            let sporify = if modifiers.adapt_spore <= 20 {
                true
            } else if modifiers.adapt_spore <= 30 {
                random.next_bool()
            } else if modifiers.adapt_spore <= 40 {
                random.bounded(4) == 0
            } else if modifiers.adapt_spore <= 50 {
                random.bounded(8) == 0
            } else if modifiers.adapt_spore <= 60 {
                random.bounded(16) == 0
            } else {
                false
            };
            if sporify {
                if modifiers.spore <= 9 {
                    blue_spore(child)?;
                } else {
                    cyan_spore(child)?;
                }
            }
        }
    }
    Ok(())
}

fn initialize_photosynthetic_spore(
    child: &mut Organism,
    parameters: &EffectsParams,
) -> Result<(), GeometryError> {
    let divisor = f64::from(parameters.green_energy_divisor);
    child.effects.use_pre_touch_effects = false;
    if child.effects.is_plant {
        if child.effects.is_consumer || child.effects.is_fungus {
            if child.effects.methanotrophy > 0.0 {
                if replace_photosynthetic_segments(child, ColorKind::Purple, 0.0)? {
                    child.effects.methanotrophy = (60.0 * child.body.mass) / divisor;
                }
            } else {
                if child.effects.jade_factor > 0 {
                    child.effects.jade_factor = 1;
                }
                child.effects.is_lime = false;
                if child.effects.can_move {
                    set_photosynthesis(child, ColorKind::C4, 0.0, 7.5, divisor)?;
                } else {
                    set_photosynthesis(child, ColorKind::Leaf, 0.0, 8.0, divisor)?;
                }
            }
        } else if child.effects.methanotrophy > 0.0 {
            if replace_photosynthetic_segments(child, ColorKind::Purple, 0.0)? {
                child.effects.methanotrophy = (120.0 * child.body.mass) / divisor;
            }
        } else if child.effects.segment_pre_touch_effects == 2 {
            child.effects.is_lime = false;
            set_photosynthesis(child, ColorKind::Winter, 0.0, 12.0, divisor)?;
        } else if child.effects.is_killer || child.effects.fallow_version > 0 {
            child.effects.is_lime = false;
            set_photosynthesis(child, ColorKind::Summer, 0.0, 15.0, divisor)?;
        } else if child.effects.is_lime {
            if replace_photosynthetic_segments(child, ColorKind::Lime, 0.0)? {
                child.effects.photosynthesis = (15.0 * child.body.mass) / divisor;
                child.effects.leaf_photo = (20.0 * child.body.mass) / divisor;
            }
        } else if child.effects.forest_photo > 0.0 {
            if replace_photosynthetic_segments(child, ColorKind::Forest, 1.0)? {
                child.effects.photosynthesis = (18.0 * child.body.mass) / divisor;
                child.effects.forest_photo = child.effects.photosynthesis;
                child.effects.use_pre_touch_effects = true;
            }
        } else if child.effects.is_enhanced {
            set_photosynthesis(child, ColorKind::Grass, 0.0, 19.0, divisor)?;
        } else if child.effects.jade_factor > 1 && !child.effects.is_blond {
            set_photosynthesis(child, ColorKind::Jade, 0.0, 19.0, divisor)?;
        } else {
            set_photosynthesis(child, ColorKind::Green, 0.0, 19.0, divisor)?;
        }
    } else if child.effects.is_consumer || child.effects.is_fungus {
        child.effects.is_plant = true;
        set_photosynthesis(child, ColorKind::Spring, 0.0, 7.0, divisor)?;
    } else {
        child.effects.is_plant = true;
        child.effects.gold = -1;
        if replace_photosynthetic_segments(child, ColorKind::Purple, 0.0)? {
            child.effects.methanotrophy = (22.0 * child.body.mass) / divisor;
        }
    }
    let code = primary_genome(&child.genome)?.borrow();
    if code.symmetry() == 3 {
        if code.mirror() == 0 {
            child.effects.photosynthesis = (5.0 / 6.0) * child.effects.photosynthesis;
            child.effects.methanotrophy = (5.0 / 6.0) * child.effects.methanotrophy;
        } else {
            child.effects.photosynthesis = (2.0 / 3.0) * child.effects.photosynthesis;
            child.effects.methanotrophy = (2.0 / 3.0) * child.effects.methanotrophy;
        }
    }
    drop(code);
    if child.effects.gold != 0 {
        child.state.reproduction.maximum_age = parameters
            .max_age
            .wrapping_add(child.body.segment_count / parameters.age_divisor);
    }
    if child.effects.is_blond && child.effects.jade_factor > 0 {
        child.effects.jade_factor = 1;
    }
    // FOREST can reenable pretouch above; the source retains that value here
    child.effects.use_friend_effects = 0;
    child.effects.can_react = false;
    child.effects.can_dodge = false;
    child.state.active = false;
    child.state.has_moved = true;
    child.effects.is_reproductive = false;
    remove_eyes(child)
}

/// Apply Organism.blueSpore without advancing hatching or maintenance
pub(crate) fn blue_spore(child: &mut Organism) -> Result<(), GeometryError> {
    let (modifier, symmetry) = {
        let code = primary_genome(&child.genome)?.borrow();
        (code.modifiers().spore, code.symmetry())
    };
    replace_segments(
        child,
        ColorKind::Blue,
        if modifier == 7 { 0.0 } else { -2.0 },
    )?;
    child.effects.spore_version = -2;
    let delay = child.effects.spore_time / symmetry;
    child.state.reproduction.countdown = delay.wrapping_mul(delay).wrapping_mul(delay) / 400;
    finish_blue_spore(child)
}

/// Apply Organism.cyanSpore; its initial segment color remains BLUE
pub(crate) fn cyan_spore(child: &mut Organism) -> Result<(), GeometryError> {
    let (modifier, symmetry) = {
        let code = primary_genome(&child.genome)?.borrow();
        (code.modifiers().spore, code.symmetry())
    };
    replace_segments(
        child,
        ColorKind::Blue,
        if modifier == 10 { 0.0 } else { -2.0 },
    )?;
    child.effects.spore_version = -3;
    let delay = child.effects.spore_time / symmetry;
    child.state.reproduction.countdown = delay.wrapping_mul(delay) / 40;
    finish_blue_spore(child)
}

fn finish_blue_spore(child: &mut Organism) -> Result<(), GeometryError> {
    child.effects.gold = 20;
    make_inactive(child);
    child.effects.is_plant = false;
    child.effects.is_consumer = false;
    if child.effects.is_fungus {
        child.effects.is_fungus = false;
        child.effects.spin = -1;
    }
    child.effects.is_enhanced = false;
    remove_eyes(child)
}

fn make_inactive(child: &mut Organism) {
    child.effects.use_pre_touch_effects = false;
    child.effects.use_friend_effects = 0;
    child.effects.can_react = false;
    child.state.active = false;
    child.state.has_moved = true;
}

fn clear_classification(child: &mut Organism) {
    child.effects.is_plant = false;
    child.effects.is_consumer = false;
    child.effects.is_fungus = false;
}

fn remove_eyes(child: &mut Organism) -> Result<(), GeometryError> {
    if child.body.eyes.enabled {
        child.body.eyes.enabled = false;
        child.rebuild_body()?;
    }
    Ok(())
}

fn replace_segment(
    child: &mut Organism,
    index: usize,
    color: ColorKind,
    coefficient: f64,
) -> Result<(), GeometryError> {
    child.body.set_color(index, color.color_ref())?;
    child.body.set_photo(index, coefficient)?;
    Ok(())
}

fn replace_segments(
    child: &mut Organism,
    color: ColorKind,
    coefficient: f64,
) -> Result<(), GeometryError> {
    for index in child.body.indices() {
        replace_segment(child, index, color, coefficient)?;
    }
    Ok(())
}

fn replace_photosynthetic_segments(
    child: &mut Organism,
    color: ColorKind,
    coefficient: f64,
) -> Result<bool, GeometryError> {
    let mut large_enough = false;
    for index in child.body.indices() {
        replace_segment(child, index, color, coefficient)?;
        if child.body.physical_length(index)? >= 1.0 {
            large_enough = true;
        }
    }
    Ok(large_enough)
}

fn set_photosynthesis(
    child: &mut Organism,
    color: ColorKind,
    coefficient: f64,
    rate: f64,
    divisor: f64,
) -> Result<(), GeometryError> {
    if replace_photosynthetic_segments(child, color, coefficient)? {
        child.effects.photosynthesis = (rate * child.body.mass) / divisor;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Gene, Genome, GenomeModifiers, GenomeProperties, IntRect, OrganismState, RotatedSegment,
    };
    use std::{cell::RefCell, rc::Rc};

    fn organism(color: ColorKind, modifier: i32) -> Organism {
        let reactions = std::array::from_fn(|index| index as i32 - 18);
        let gene = Gene::new(8.0, 0.0, color.argb(), 0, reactions).shared();
        let code = Genome::from_parts(
            &[gene],
            GenomeProperties {
                symmetry: 1,
                modifiers: GenomeModifiers {
                    spore: modifier,
                    ..GenomeModifiers::default()
                },
                ..GenomeProperties::default()
            },
        );
        Organism::unplaced(Rc::new(RefCell::new(code)), OrganismState::default()).unwrap()
    }

    #[test]
    fn partial_spore_payment_does_not_recolor_or_refund() {
        let mut child = organism(ColorKind::Green, 1);
        child.state.energy = 3.0;
        child.effects.spore_time = 2;
        child.effects.spore_version = 1;
        child.effects.is_plant = true;
        child.effects.use_pre_touch_effects = true;
        child.effects.use_friend_effects = 2;
        child.effects.can_react = true;
        child.body.set_photo(0, 3.0).unwrap();
        let mut parent = organism(ColorKind::Green, 1);
        parent.effects.spore_version = 1;
        let mut parameters = crate::WorldParameters::default();
        parameters.atmosphere.initial_oxygen = 1.0;
        parameters.atmosphere.initial_carbon_dioxide = 4.0;
        let world = crate::World::with_parameters(
            crate::WorldConfig { initial_attempts: 0, ..crate::WorldConfig::default() }, parameters,
        ).unwrap();
        child.state.world = world.association();
        parent.state.world = world.association();
        let mut random = JavaRandom::new(42);
        let unchanged_random = random.clone();
        initialize_birth_spore(
            &mut child,
            &parent,
            &mut random,
            &EffectsParams::default(),
            2.0,
        )
        .unwrap();
        let (oxygen, carbon_dioxide) = (world.oxygen(), world.carbon_dioxide());
        assert_eq!(
            (child.state.energy, oxygen, carbon_dioxide),
            (2.0, 0.0, 5.0)
        );
        assert_eq!(
            child.body.color_argb(0).unwrap(),
            ColorKind::Green.argb().unwrap()
        );
        assert_eq!(child.body.photo_values().unwrap(), [3.0]);
        assert!(!child.state.active);
        assert!(child.state.alive);
        assert!(!child.effects.is_plant);
        assert!(!child.effects.is_enhanced);
        assert!(!child.effects.can_react);
        assert!(!child.effects.use_pre_touch_effects);
        assert_eq!(child.effects.use_friend_effects, 0);
        assert_eq!(random, unchanged_random);
    }

    #[test]
    fn dormant_spores_preserve_reactions_and_cached_geometry() {
        for moving in [false, true] {
            let mut child = organism(ColorKind::Eye, if moving { 10 } else { 7 });
            child.effects.spore_time = i32::MAX;
            child.effects.is_fungus = true;
            child.effects.spin = 99;
            child.effects.can_move = true;
            child.effects.is_reproductive = true;
            child.state.bounds.set(IntRect {
                x: 17,
                y: 29,
                width: 31,
                height: 43,
            });
            child.body.set_rotated_values(&(vec![RotatedSegment {
                start: [11, 23],
                end: [37, 41],
            }]));
            let cached_bounds = child.state.bounds.get();
            let cached_segments = child.body.rotated_values().unwrap();
            let reactions = child.body.reaction_values(0).unwrap();
            assert!(child.body.eyes.enabled);
            if moving {
                cyan_spore(&mut child).unwrap();
            } else {
                blue_spore(&mut child).unwrap();
            }
            assert_eq!(
                child.state.reproduction.countdown,
                if moving { 0 } else { 5_368_709 }
            );
            assert_eq!(child.effects.spore_version, if moving { -3 } else { -2 });
            assert_eq!(child.body.color_argb(0).unwrap(), ColorKind::Blue.argb().unwrap());
            assert_eq!(child.body.reaction_values(0).unwrap(), reactions);
            assert_eq!(child.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow().color(), ColorKind::Eye.argb());
            assert_eq!(child.body.photo_values().unwrap(), [0.0]);
            assert_eq!(child.state.bounds.get(), cached_bounds);
            assert_eq!(child.body.rotated_values().unwrap(), cached_segments);
            assert!(!child.body.eyes.enabled);
            assert_eq!(child.effects.spin, -1);
            assert!(!child.effects.is_fungus);
            assert!(child.effects.can_move);
            assert!(child.effects.is_reproductive);
            assert!(child.state.alive);
            assert!(!child.state.active);
        }
    }
    #[test]
    fn non_photosynthetic_spores_bypass_length_and_version_six_keeps_ordered_failure_prefix() {
        // call, modifier, color, coefficient, countdown, maximum age, enhanced, consumer
        for (call, modifier, color, coefficient, countdown, maximum_age, enhanced, consumer) in [
            (1, 1, ColorKind::Fruit, -2.0, 777, 222, false, false),
            (2, 2, ColorKind::Fruit, 0.0, 777, 9, true, false),
            (3, 3, ColorKind::Bark, 0.0, 95, 222, true, false),
            (4, 4, ColorKind::Brown, 0.0, 95, 222, true, false),
            (5, 5, ColorKind::Cream, -1.0, 777, 222, false, true),
            (-2, 7, ColorKind::Blue, 0.0, 1, 222, false, false),
            (-2, 8, ColorKind::Blue, -2.0, 1, 222, false, false),
            (-3, 10, ColorKind::Blue, 0.0, 2, 222, false, false),
            (-3, 11, ColorKind::Blue, -2.0, 2, 222, false, false),
        ] {
            let mut child = organism(ColorKind::Green, modifier);
            child.body.arrays.length = None;
            child.body.eyes.enabled = false;
            child.effects.spore_time = 9;
            child.effects.spore_version = call;
            child.effects.is_plant = true;
            child.effects.is_enhanced = true;
            child.effects.use_pre_touch_effects = true;
            child.effects.use_friend_effects = 2;
            child.effects.can_react = true;
            child.state.energy = 100.0;
            child.state.reproduction.countdown = 777;
            child.state.reproduction.maximum_age = 222;
            let colors = child.body.phenotype_colors.clone().unwrap();
            let photo = child.body.arrays.photo.clone().unwrap();
            let mut parent = organism(ColorKind::Green, modifier);
            parent.effects.spore_version = call;
            let mut random = JavaRandom::new(449951);
            let before_random = random.clone();
            let mut parameters = crate::WorldParameters::default();
            parameters.atmosphere.initial_oxygen = 100.0;
            parameters.atmosphere.initial_carbon_dioxide = 0.0;
            let world = crate::World::with_parameters(
                crate::WorldConfig { initial_attempts: 0, ..crate::WorldConfig::default() }, parameters,
            ).unwrap();
            child.state.world = world.association();
            parent.state.world = world.association();
            if call == -2 {
                blue_spore(&mut child).unwrap();
            } else if call == -3 {
                cyan_spore(&mut child).unwrap();
            } else {
                initialize_birth_spore(
                    &mut child,
                    &parent,
                    &mut random,
                    &EffectsParams::default(),
                    1.0,
                )
                .unwrap();
            }
            assert!(child.body.arrays.length.is_none());
            assert!(
                child
                    .body
                    .phenotype_colors
                    .as_ref()
                    .unwrap()
                    .same_instance(&colors)
            );
            assert!(
                child
                    .body
                    .arrays
                    .photo
                    .as_ref()
                    .unwrap()
                    .same_instance(&photo)
            );
            assert!(
                child
                    .body
                    .required_color(0)
                    .unwrap()
                    .same_instance(&color.color_ref().unwrap())
            );
            assert_eq!(child.body.photo(0).unwrap(), coefficient);
            assert_eq!(child.state.reproduction.countdown, countdown);
            assert_eq!(child.state.reproduction.maximum_age, maximum_age);
            assert!(!child.state.active && child.state.has_moved);
            assert!(!child.effects.use_pre_touch_effects && !child.effects.can_react);
            assert_eq!(child.effects.use_friend_effects, 0);
            assert!(!child.effects.is_plant && !child.effects.is_fungus);
            assert_eq!(child.effects.is_consumer, consumer);
            assert_eq!(child.effects.is_enhanced, enhanced);
            assert_eq!(child.effects.spore_version, call);
            assert_eq!(child.effects.gold, if call < 0 { 20 } else { 0 });
            assert_eq!(random, before_random);
            let (oxygen, carbon_dioxide) = (world.oxygen(), world.carbon_dioxide());
            let paid = if call == 1 || call == 2 { 1.0 } else { 0.0 };
            assert_eq!(
                (child.state.energy, oxygen, carbon_dioxide),
                (100.0 - paid, 100.0 - paid, paid)
            );
        }
        for length in [None, Some(crate::DoubleArrayRef::from_values(vec![2.0]))] {
            let mut child = organism(ColorKind::Red, 6);
            child.body.segment_count = 2;
            child.body.phenotype_colors = Some(crate::ColorArrayRef::from_slots(vec![
                ColorKind::Red.color_ref(),
                ColorKind::Red.color_ref(),
            ]));
            child.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![9.0, 8.0]));
            child.body.arrays.length = length;
            child.body.eyes.enabled = false;
            child.effects.spore_time = 9;
            child.effects.spore_version = 6;
            child.effects.is_plant = true;
            child.effects.photosynthesis = 37.0;
            child.effects.use_pre_touch_effects = true;
            child.state.reproduction.countdown = 777;
            child.state.has_moved = false;
            let failed_index = usize::from(child.body.arrays.length.is_some());
            let expected = if failed_index == 0 {
                GeometryError::MissingPrimitiveArray { field: "_m" }
            } else {
                GeometryError::PrimitiveIndex {
                    field: "_m",
                    index: 1,
                    length: 1,
                }
            };
            assert_eq!(
                initialize_photosynthetic_spore(&mut child, &EffectsParams::default()),
                Err(expected)
            );
            for index in 0..=failed_index {
                assert!(
                    child
                        .body
                        .required_color(index)
                        .unwrap()
                        .same_instance(&ColorKind::Green.color_ref().unwrap())
                );
                assert_eq!(child.body.photo(index).unwrap().to_bits(), 0);
            }
            if failed_index == 0 {
                assert!(
                    child
                        .body
                        .required_color(1)
                        .unwrap()
                        .same_instance(&ColorKind::Red.color_ref().unwrap())
                );
                assert_eq!(child.body.photo(1).unwrap(), 8.0);
            }
            assert_eq!(child.effects.photosynthesis, 37.0);
            assert!(!child.effects.use_pre_touch_effects);
            assert!(child.state.active && child.effects.is_plant);
            assert!(!child.state.has_moved);
            assert_eq!(child.state.reproduction.countdown, 777);
        }
    }

}
