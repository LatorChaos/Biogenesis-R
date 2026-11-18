// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Organism offsets, cached bounds, growth, and rubbing follow the pinned Java frame primitives

use crate::organism::primary_genome;

use crate::birth_spore::{blue_spore, cyan_spore};
use crate::java_math::{add, div, mul, sqrt};
use crate::organism::Organism;
use crate::{ColorKind, GeometryError, TOLERANCE};

/// Apply Organism.offset without normalizing rotation or rejecting internal IEEE state
pub(crate) fn offset(organism: &mut Organism, translation: [f64; 2], angle: f64) {
    organism.state.center[0] = add(organism.state.center[0], translation[0]);
    organism.state.center[1] = add(organism.state.center[1], translation[1]);
    organism.state.angle = add(organism.state.angle, angle);
    organism.state.integer_center = organism.state.center.map(|coordinate| coordinate as i32);
}

/// Preserve calculateBounds' rotation cache while updating the rectangle for the current double center
pub(crate) fn calculate_bounds(organism: &mut Organism, force: bool) -> Result<(), GeometryError> {
    use crate::{IntField::*, DoubleField::*};
    let (mut left, mut top) = (f64::MAX, f64::MAX);
    let (mut right, mut bottom) = (f64::from_bits(1), f64::from_bits(1));
    for index in organism.body.reverse_indices() {
        let index = index as i32;
        let arrays = &organism.body.arrays;
        if organism.state.last_angle != organism.state.angle || force {
            let theta = organism.state.angle + crate::java_math::atan2(arrays.read_int(StartY, index)? as f64, arrays.read_int(StartX, index)? as f64);
            let destination = arrays.x1.clone();
            let value = (arrays.read_double(StartRadius, index)? * theta.cos()) as i32;
            crate::OrganismArrays::store_int(destination, X1, index, value)?;
            let destination = arrays.y1.clone();
            let value = (arrays.read_double(StartRadius, index)? * theta.sin()) as i32;
            crate::OrganismArrays::store_int(destination, Y1, index, value)?;
            let theta = organism.state.angle + crate::java_math::atan2(arrays.read_int(EndY, index)? as f64, arrays.read_int(EndX, index)? as f64);
            let destination = arrays.x2.clone();
            let value = (arrays.read_double(EndRadius, index)? * theta.cos()) as i32;
            crate::OrganismArrays::store_int(destination, X2, index, value)?;
            let destination = arrays.y2.clone();
            let value = (arrays.read_double(EndRadius, index)? * theta.sin()) as i32;
            crate::OrganismArrays::store_int(destination, Y2, index, value)?;
        }
        left = crate::java_math::min(crate::java_math::min(left, arrays.read_int(X1, index)? as f64 + organism.state.center[0]), arrays.read_int(X2, index)? as f64 + organism.state.center[0]);
        right = crate::java_math::max(crate::java_math::max(right, arrays.read_int(X1, index)? as f64 + organism.state.center[0]), arrays.read_int(X2, index)? as f64 + organism.state.center[0]);
        top = crate::java_math::min(crate::java_math::min(top, arrays.read_int(Y1, index)? as f64 + organism.state.center[1]), arrays.read_int(Y2, index)? as f64 + organism.state.center[1]);
        bottom = crate::java_math::max(crate::java_math::max(bottom, arrays.read_int(Y1, index)? as f64 + organism.state.center[1]), arrays.read_int(Y2, index)? as f64 + organism.state.center[1]);
    }
    organism.state.bounds.set(crate::IntRect { x: left as i32, y: top as i32,
        width: ((right - left + 1.0) as i32).wrapping_add(1), height: ((bottom - top + 1.0) as i32).wrapping_add(1) });
    organism.state.last_angle = organism.state.angle;
    Ok(())
}

/// Apply Organism.grow before this frame increments age or processes maintenance
pub(crate) fn grow(organism: &mut Organism) -> Result<(), GeometryError> {
    if organism.body.growth_ratio > 1 && organism.state.energy >= organism.body.mass / 10.0 {
        if (organism.state.active || organism.body.growth_ratio == 16)
            && (organism.state.age & 7) == 7
        {
            organism.body.growth_ratio = organism.body.growth_ratio.wrapping_sub(1);
            let mass = organism.body.mass;
            let inertia = organism.body.inertia;
            organism.rebuild_body()?;
            let factor = sqrt(div(mass, organism.body.mass));
            organism.state.velocity[0] = mul(organism.state.velocity[0], factor);
            organism.state.velocity[1] = mul(organism.state.velocity[1], factor);
            organism.state.angular_velocity = mul(
                organism.state.angular_velocity,
                sqrt(div(inertia, organism.body.inertia)),
            );
            organism.state.has_grown = 1;
            if organism.effects.spore_version < -1 {
                organism.effects.spore_version = -1;
            }
        } else {
            organism.state.has_grown = 0;
        }
    } else if organism.state.energy < organism.body.mass / 12.0 {
        if organism.body.growth_ratio < 15 {
            organism.body.growth_ratio = organism.body.growth_ratio.wrapping_add(1);
            let mass = organism.body.mass;
            let inertia = organism.body.inertia;
            organism.rebuild_body()?;
            let factor = sqrt(div(mass, organism.body.mass));
            organism.state.velocity[0] = mul(organism.state.velocity[0], factor);
            organism.state.velocity[1] = mul(organism.state.velocity[1], factor);
            organism.state.angular_velocity = mul(
                organism.state.angular_velocity,
                sqrt(div(inertia, organism.body.inertia)),
            );
            organism.state.has_grown = -1;
        } else {
            organism.state.has_grown = 0;
            if organism.effects.spore_version == -1
                && organism.state.active
                && ((!organism.effects.is_infectious
                    && !organism.effects.is_blond
                    && organism.state.temporary_color_frames <= 9)
                    || organism.state.temporary_color_frames == 0)
            {
                organism.effects.is_frozen = false;
                organism.effects.is_injured = false;
                organism.effects.all_frozen = false;
                let moving = primary_genome(&organism.genome)?.borrow().modifiers().spore >= 10;
                if moving {
                    cyan_spore(organism)?;
                    organism.set_display_color(ColorKind::Cyan.color_ref());
                } else {
                    blue_spore(organism)?;
                    organism.set_display_color(ColorKind::Blue.color_ref());
                }
                if organism.infected_genome.is_some() {
                    organism.infected_genome = None;
                    organism.saved_genome = None;
                }
            }
        }
    } else {
        organism.state.has_grown = 0;
    }
    Ok(())
}

/// Apply rubbingFramesEffects with the strict source tolerance comparison
pub(crate) fn rubbing(organism: &mut Organism, rubbing: f64) {
    organism.state.velocity[0] = mul(organism.state.velocity[0], rubbing);
    if organism.state.velocity[0].abs() < TOLERANCE {
        organism.state.velocity[0] = 0.0;
    }
    organism.state.velocity[1] = mul(organism.state.velocity[1], rubbing);
    if organism.state.velocity[1].abs() < TOLERANCE {
        organism.state.velocity[1] = 0.0;
    }
    organism.state.angular_velocity = mul(organism.state.angular_velocity, rubbing);
    if organism.state.angular_velocity.abs() < TOLERANCE {
        organism.state.angular_velocity = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Gene, Genome, GenomeModifiers, GenomeProperties, IntRect, OrganismState, RotatedSegment,
    };
    use std::{cell::RefCell, rc::Rc};

    fn organism(length: f64, ratio: i32, spore: i32) -> Organism {
        let gene = Gene::new(
            length,
            0.0,
            ColorKind::Green.argb(),
            0,
            std::array::from_fn(|index| index as i32 - 18),
        )
        .shared();
        let genome = Genome::from_parts(
            &[gene],
            GenomeProperties {
                symmetry: 1,
                modifiers: GenomeModifiers {
                    spore,
                    ..GenomeModifiers::default()
                },
                ..GenomeProperties::default()
            },
        );
        let mut organism =
            Organism::unplaced(Rc::new(RefCell::new(genome)), OrganismState::default()).unwrap();
        organism.body.growth_ratio = ratio;
        organism.rebuild_body().unwrap();
        organism
    }

    #[test]
    fn cached_bounds_preserve_endpoints_and_integer_centers_until_forced() {
        let mut organism = organism(20.0, 16, 1);
        organism.state.center = [-10.75, -20.5];
        organism.state.integer_center = [777, -777];
        organism.state.angle = 0.0;
        organism.state.last_angle = -0.0;
        organism.body.set_rotated_segment(0, RotatedSegment {
            start: [-3, -4],
            end: [-1, -2],
        }).unwrap();
        let cached = organism.body.rotated_values().unwrap();
        calculate_bounds(&mut organism, false).unwrap();
        assert_eq!(organism.body.rotated_values().unwrap(), cached);
        assert_eq!(organism.state.integer_center, [777, -777]);
        // Positive Double.MIN_VALUE keeps right and bottom above every negative endpoint
        assert_eq!(
            organism.state.bounds.get(),
            IntRect {
                x: -13,
                y: -24,
                width: 15,
                height: 26
            }
        );
        assert_eq!(organism.state.last_angle.to_bits(), 0.0_f64.to_bits());
        calculate_bounds(&mut organism, true).unwrap();
        assert_ne!(organism.body.rotated_values().unwrap(), cached);
        assert_eq!(organism.state.integer_center, [777, -777]);
        assert!(
            organism
                .body
                .rotated_values().unwrap()
                .iter()
                .all(|segment| segment.start[1] == 0 && segment.end[1] == 0)
        );
    }

    #[test]
    fn offset_keeps_signed_zero_and_java_nonfinite_casts() {
        let mut organism = organism(12.0, 16, 1);
        organism.state.center = [-0.0, -0.0];
        organism.state.angle = -0.0;
        offset(&mut organism, [-0.0, -0.0], -0.0);
        assert_eq!(
            organism.state.center.map(f64::to_bits),
            [(-0.0_f64).to_bits(); 2]
        );
        assert_eq!(organism.state.angle.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(organism.state.integer_center, [0, 0]);
        offset(
            &mut organism,
            [f64::INFINITY, f64::NEG_INFINITY],
            f64::INFINITY,
        );
        assert_eq!(organism.state.integer_center, [i32::MAX, i32::MIN]);
        calculate_bounds(&mut organism, true).unwrap();
        assert_eq!(
            organism.state.bounds.get(),
            IntRect {
                x: i32::MAX,
                y: i32::MIN,
                width: i32::MIN,
                height: i32::MIN
            }
        );
        assert!(
            organism
                .body
                .rotated_values().unwrap()
                .iter()
                .all(|segment| segment.start == [0, 0] && segment.end == [0, 0])
        );
    }

    #[test]
    fn growth_gates_preserve_inactive_newborns_shrink_limit_and_current_phenotype() {
        let mut newborn = organism(12.0, 16, 7);
        newborn.state.active = false;
        newborn.state.age = 6;
        newborn.state.energy = newborn.body.mass / 10.0;
        newborn.effects.spore_version = -2;
        newborn.body.set_color_argb(0, 0x80504020).unwrap();
        let reactions = newborn.body.reaction_values(0).unwrap();
        let cached = newborn.body.rotated_values().unwrap();
        let bounds = newborn.state.bounds.get();
        let energy = newborn.state.energy;
        grow(&mut newborn).unwrap();
        assert_eq!(
            (newborn.body.growth_ratio, newborn.state.has_grown),
            (16, 0)
        );
        newborn.state.age = 7;
        grow(&mut newborn).unwrap();
        assert_eq!(
            (newborn.body.growth_ratio, newborn.state.has_grown),
            (15, 1)
        );
        assert_eq!(newborn.effects.spore_version, -1);
        assert!(!newborn.state.active);
        assert_eq!(newborn.state.energy, energy);
        assert_eq!(newborn.body.color_argb(0).unwrap(), 0x80504020);
        assert_eq!(newborn.body.reaction_values(0).unwrap(), reactions);
        assert_eq!(newborn.body.rotated_values().unwrap(), cached);
        assert_eq!(newborn.state.bounds.get(), bounds);

        let mut shrinking = organism(12.0, 14, 1);
        shrinking.state.active = false;
        shrinking.state.energy = shrinking.body.mass / 12.0;
        grow(&mut shrinking).unwrap();
        assert_eq!(
            (shrinking.body.growth_ratio, shrinking.state.has_grown),
            (14, 0)
        );
        shrinking.state.energy = 0.0;
        grow(&mut shrinking).unwrap();
        assert_eq!(
            (shrinking.body.growth_ratio, shrinking.state.has_grown),
            (15, -1)
        );
        grow(&mut shrinking).unwrap();
        assert_eq!(
            (shrinking.body.growth_ratio, shrinking.state.has_grown),
            (15, 0)
        );

        let mut zero_mass = organism(0.0, 16, 1);
        zero_mass.state.age = 7;
        grow(&mut zero_mass).unwrap();
        assert_eq!(
            (zero_mass.body.growth_ratio, zero_mass.state.has_grown),
            (15, 1)
        );
        assert!(zero_mass.state.velocity.iter().all(|value| value.is_nan()));
        assert!(zero_mass.state.angular_velocity.is_nan());
        let velocity = zero_mass.state.velocity;
        let rotation = zero_mass.state.angular_velocity;
        offset(&mut zero_mass, velocity, rotation);
        calculate_bounds(&mut zero_mass, false).unwrap();
        assert_eq!(
            zero_mass.state.bounds.get(),
            IntRect {
                x: 0,
                y: 0,
                width: 1,
                height: 1
            }
        );
    }

    #[test]
    fn sporification_keeps_saved_without_infection_and_obeys_display_gates() {
        for (infectious, blond, frames, converts) in [
            (false, false, 9, true),
            (false, false, 10, false),
            (true, false, 9, false),
            (true, true, 0, true),
        ] {
            let mut organism = organism(12.0, 15, 10);
            organism.effects.spore_version = -1;
            organism.effects.spore_time = 21;
            organism.effects.is_infectious = infectious;
            organism.effects.is_blond = blond;
            organism.effects.is_frozen = true;
            organism.effects.is_injured = true;
            organism.effects.all_frozen = true;
            organism.state.temporary_color_frames = frames;
            organism.saved_genome = Some(Rc::clone(organism.genome.as_ref().unwrap()));
            grow(&mut organism).unwrap();
            assert_eq!(organism.state.has_grown, 0);
            assert_eq!(
                organism.effects.spore_version,
                if converts { -3 } else { -1 }
            );
            assert_eq!(organism.state.active, !converts);
            assert_eq!(organism.effects.is_frozen, !converts);
            assert_eq!(organism.effects.is_injured, !converts);
            assert_eq!(organism.effects.all_frozen, !converts);
            assert!(organism.saved_genome.is_some());
            if converts {
                assert_eq!(organism.state.temporary_argb(), ColorKind::Cyan.argb());
                assert_eq!(organism.state.temporary_color_frames, 10);
                assert_eq!(
                    organism.body.color_argb(0).unwrap(),
                    ColorKind::Blue.argb().unwrap()
                );
            }
        }
        let mut infected = organism(12.0, 15, 7);
        infected.effects.spore_version = -1;
        infected.effects.spore_time = 21;
        infected.infected_genome = Some(Rc::clone(infected.genome.as_ref().unwrap()));
        infected.saved_genome = Some(Rc::clone(infected.genome.as_ref().unwrap()));
        grow(&mut infected).unwrap();
        assert!(infected.infected_genome.is_none());
        assert!(infected.saved_genome.is_none());
        assert_eq!(infected.state.temporary_argb(), ColorKind::Blue.argb());
    }

    #[test]
    fn rubbing_keeps_exact_tolerance_and_resets_small_values_to_positive_zero() {
        let mut organism = organism(12.0, 16, 1);
        organism.state.velocity = [-TOLERANCE, TOLERANCE];
        organism.state.angular_velocity = -0.0;
        rubbing(&mut organism, 1.0);
        assert_eq!(
            organism.state.velocity.map(f64::to_bits),
            [(-TOLERANCE).to_bits(), TOLERANCE.to_bits()]
        );
        assert_eq!(organism.state.angular_velocity.to_bits(), 0.0_f64.to_bits());
        organism.state.velocity = [-TOLERANCE / 2.0, TOLERANCE / 2.0];
        rubbing(&mut organism, 1.0);
        assert_eq!(
            organism.state.velocity.map(f64::to_bits),
            [0.0_f64.to_bits(); 2]
        );
        organism.state.velocity = [f64::NAN, f64::INFINITY];
        rubbing(&mut organism, 0.98);
        assert!(organism.state.velocity[0].is_nan());
        assert_eq!(organism.state.velocity[1], f64::INFINITY);
    }
}
