// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::organism::primary_genome;

use super::{Organism, OrganismRef, World, WorldError};
use crate::{ColorKind, GeometryError, TOLERANCE, java_math::min, physics::between};
use std::f64::consts::PI;

impl World {
    /// Apply active maintenance after reproduction and before frame removal
    pub(super) fn breath_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        parent.state.age = parent.state.age.wrapping_add(1);
        let maintenance = min(parent.effects.maintenance, parent.state.energy);
        let can_breathe =
            parent.use_energy(maintenance)?;
        let aged_out = (parent.state.age >> 8) > parent.state.reproduction.maximum_age;
        if aged_out || !can_breathe {
            if parent.effects.spore_version == -1
                && parent.state.energy > f64::from(parent.state.reproduction.energy / 2)
                && parent.body.growth_ratio == 1
                && aged_out
            {
                parent.state.reproduction.energy = (parent.state.energy as i32).wrapping_add(
                    16_i32.wrapping_add(3_i32.wrapping_mul(primary_genome(&parent.genome)?.borrow().symmetry())),
                );
                self.reproduce_spore_from(parent_ref, parent, peer)?;
                parent.state.reproduction.energy =
                    40_i32.wrapping_add(3_i32.wrapping_mul(parent.body.segment_count));
            }
            self.die(parent, None)?;
        } else {
            let threshold = parent.state.reproduction.energy;
            if parent.state.energy > f64::from(threshold) {
                let twice_threshold = threshold.wrapping_mul(2);
                if parent.state.energy > f64::from(twice_threshold) {
                    let amount = (parent.state.energy as i32).wrapping_sub(twice_threshold);
                    parent.use_energy(
                        f64::from(amount))?;
                    if parent.effects.reproduce_late > 0
                        && !parent.effects.is_infectious
                        && parent.state.reproduction.countdown > 0
                    {
                        parent.state.reproduction.countdown = 0;
                    }
                } else if parent.effects.reproduce_late <= 0 || parent.effects.is_infectious {
                    let amount = (parent.state.energy as i32).wrapping_sub(threshold);
                    parent.use_energy(
                        f64::from(amount) * 0.005)?;
                } else if parent.state.reproduction.countdown > 0
                    && parent.state.energy
                        > f64::from(twice_threshold.wrapping_sub(parent.effects.reproduce_late))
                {
                    parent.state.reproduction.countdown = 0;
                }
            } else if parent.state.energy <= TOLERANCE {
                // Exhaustion bypasses die's phenotype and cached-effect clearing
                parent.state.active = false;
                self.record_exhaustion(parent)?;
            }
        }
        Ok(())
    }

    fn record_exhaustion(&self, organism: &mut Organism) -> Result<(), WorldError> {
        organism.state.alive = false;
        organism.state.world.resolve()?.decrease_population();
        organism.state.world.resolve()?.statistics_ref()
            .borrow_mut()
            .event_organism_die(organism, None);
        Ok(())
    }

    pub(super) fn use_decay_energy(
        &self, organism: &mut Organism, amount: f64,
    ) -> Result<bool, WorldError> {
        if organism.state.energy < amount { return Ok(false); }
        let target = organism.state.world.resolve()?;
        let consumed = target.transfer_gas(amount, true);
        organism.state.energy = crate::java_math::sub(organism.state.energy, consumed);
        Ok(!(consumed < amount))
    }

    fn spore_movement(&self, organism: &mut Organism, slow: bool) -> Result<(), WorldError> {
        let mass = if slow {
            crate::java_math::mul(9.0, organism.body.mass)
        } else {
            organism.body.mass
        };
        let limits = self.session.0.parameters.borrow().physics;
        for index in organism.body.reverse_indices() {
            if self.session.0.random.borrow_mut().bounded(100) < 8 {
                    for axis in 0..2 {
                    let delta = organism.body.rotated_end(index, axis)?.wrapping_sub(organism.body.rotated_start(index, axis)?);
                    organism.state.velocity[axis] = between(
                        crate::java_math::add(
                            organism.state.velocity[axis],
                            crate::java_math::div(crate::java_math::mul(12.0, f64::from(delta)), mass),
                        ),
                        -limits.max_velocity,
                        limits.max_velocity,
                    );
                }
                let sign = (self.session.0.random.borrow_mut().bounded(2) << 1) - 1;
                organism.state.angular_velocity = between(
                    crate::java_math::add(
                        organism.state.angular_velocity,
                        crate::java_math::div(
                            crate::java_math::mul(
                                crate::java_math::mul(f64::from(sign), organism.body.physical_length(index)?),
                                PI,
                            ),
                            organism.body.inertia,
                        ),
                    ),
                    -limits.max_rotation,
                    limits.max_rotation,
                );
            }
        }
        Ok(())
    }

    /// Apply all inactive-spore branches or one corpse decomposition payment
    pub(super) fn decay_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        if !parent.state.alive {
            self.use_decay_energy(
                parent,
                min(parent.state.energy, self.session.0.parameters.borrow().lifecycle.decay_energy),
            )?;
            return Ok(());
        }
        match parent.effects.spore_version {
            2 => {
                parent.state.age = parent.state.age.wrapping_add(1);
                if (parent.state.age >> 8) > parent.state.reproduction.maximum_age {
                    self.die(parent, None)?;
                } else if parent.state.energy <= TOLERANCE {
                    self.record_exhaustion(parent)?;
                }
            }
            version @ (1 | 5) => {
                parent.state.age = parent.state.age.wrapping_add(1);
                let cost = if version == 1 { 0.003 } else { 0.0027 };
                let can_breathe = parent.use_energy(
                    min(cost, parent.state.energy))?;
                if (parent.state.age >> 8) >= parent.state.reproduction.maximum_age || !can_breathe
                {
                    self.die(parent, None)?;
                } else if parent.state.energy <= TOLERANCE {
                    self.record_exhaustion(parent)?;
                } else {
                    if !parent.effects.is_frozen {
                        self.spore_movement(parent, version == 5)?;
                    }
                    if version == 5
                        && parent.state.energy > 0.5 * f64::from(parent.state.reproduction.energy)
                        && parent.use_energy(
                            self.session.0.parameters.borrow().spore_energy_consumption)?
                    {
                        restore_gene_colors(parent, true).map_err(WorldError::Geometry)?;
                        parent.state.temporary_color_frames = 0;
                        parent.effects.is_consumer = false;
                        parent.effects.is_frozen = false;
                        parent.effects.is_injured = false;
                        parent.effects.all_frozen = false;
                        parent.state.active = true;
                        parent.state.has_moved = true;
                        parent.state.age = 1;
                        self.refresh_hatched_effects(parent)?;
                    }
                }
            }
            6 => {
                if parent.effects.is_injured
                    && !self.use_decay_energy(parent, min(0.001, parent.state.energy))?
                {
                    return Ok(());
                }
                parent.state.age = parent.state.age.wrapping_add(1);
                if (parent.state.age >> 8) > parent.state.reproduction.maximum_age {
                    self.die(parent, None)?;
                } else if parent.state.energy <= TOLERANCE {
                    self.record_exhaustion(parent)?;
                } else if (parent.effects.is_blond
                    && parent.effects.gold >= 0
                    && parent.state.energy
                        > 0.5 * f64::from(parent.state.reproduction.early_energy))
                    || parent.state.energy > 0.5 * f64::from(parent.state.reproduction.energy)
                {
                    if (parent.effects.is_blond
                        || parent.effects.gold > 0
                        || parent.effects.spore_time < 0)
                        && parent.infected_genome.is_some()
                    {
                        self.reproduce_virus_from(parent_ref, parent, peer)?;
                        parent.state.virus_children = 0;
                        if parent.state.energy <= TOLERANCE {
                            self.record_exhaustion(parent)?;
                        }
                    } else if parent.state.reproduction.countdown == 0 {
                        if parent.use_energy(
                            self.session.0.parameters.borrow().spore_energy_consumption)? {
                            restore_gene_colors(parent, true).map_err(WorldError::Geometry)?;
                            parent.effects.photosynthesis = 0.0;
                            parent.effects.methanotrophy = 0.0;
                            parent.effects.leaf_photo = 0.0;
                            parent.effects.forest_photo = 0.0;
                            parent.state.temporary_color_frames = 0;
                            parent.effects.is_plant = false;
                            if parent.effects.jade_factor > 0 {
                                parent.effects.jade_factor = 1;
                            }
                            parent.effects.is_frozen = false;
                            parent.effects.is_injured = false;
                            parent.effects.all_frozen = false;
                            parent.state.active = true;
                            parent.state.has_moved = true;
                            parent.effects.gold = 0;
                            parent.state.age = 1;
                            self.refresh_hatched_effects(parent)?;
                        }
                    } else {
                        parent.state.reproduction.countdown =
                            parent.state.reproduction.countdown.wrapping_sub(1);
                        parent.set_display_color(ColorKind::Flower.color_ref());
                    }
                } else if !parent.effects.is_injured {
                    if parent.effects.methanotrophy > 0.0 {
                        if parent.state.age & 7 == 0 {
                            let obtained = parent.state.world.resolve()?.methanotrophy(parent.effects.methanotrophy);
                            parent.state.energy = crate::java_math::add(parent.state.energy, obtained);
                        }
                    } else if parent.effects.is_lime {
                        let amount = if parent.state.world.resolve()?.fast_check_hit(parent, parent_ref, parent, peer)? {
                            parent.effects.photosynthesis
                        } else {
                            parent.effects.leaf_photo
                        };
                        let obtained = parent.state.world.resolve()?.photosynthesis(amount);
                        parent.state.energy = crate::java_math::add(parent.state.energy, obtained);
                    } else if parent.effects.colony_photosynthesis > 0.0 {
                        let obtained = parent.state.world.resolve()?.photosynthesis(
                            crate::java_math::add(parent.effects.photosynthesis, parent.effects.colony_photosynthesis),
                        );
                        parent.state.energy = crate::java_math::add(parent.state.energy, obtained);
                        parent.effects.colony_photosynthesis = 0.0;
                    } else {
                        let obtained = parent.state.world.resolve()?.photosynthesis(parent.effects.photosynthesis);
                        parent.state.energy = crate::java_math::add(parent.state.energy, obtained);
                    }
                }
            }
            version if version >= 3 => {
                if parent.effects.is_injured
                    && !self.use_decay_energy(parent, min(0.001, parent.state.energy))?
                {
                    return Ok(());
                }
                if parent.state.energy <= TOLERANCE {
                    self.record_exhaustion(parent)?;
                } else if parent.state.reproduction.countdown == 0 {
                    if parent.use_energy(
                        self.session.0.parameters.borrow().spore_energy_consumption)? {
                        restore_gene_colors(parent, false).map_err(WorldError::Geometry)?;
                        parent.state.temporary_color_frames = 0;
                        parent.effects.is_enhanced = false;
                        parent.effects.is_frozen = false;
                        parent.effects.is_injured = false;
                        parent.effects.all_frozen = false;
                        parent.state.active = true;
                        parent.state.has_moved = true;
                        parent.state.age = 1;
                        self.refresh_hatched_effects(parent)?;
                    }
                } else if !parent.effects.is_injured {
                    if version == 4 && parent.state.temporary_color_frames <= 1 {
                        parent.set_display_color(ColorKind::Spore.color_ref());
                    }
                    parent.state.reproduction.countdown =
                        parent.state.reproduction.countdown.wrapping_sub(1);
                }
            }
            -3 => {
                if parent.effects.is_injured
                    && !self.use_decay_energy(parent, min(0.001, parent.state.energy))?
                {
                    return Ok(());
                }
                if !parent.use_energy(
                    min(0.001, parent.state.energy))? {
                    self.die(parent, None)?;
                } else if parent.state.energy <= TOLERANCE {
                    self.record_exhaustion(parent)?;
                } else {
                    if !parent.effects.is_frozen {
                        if parent.state.temporary_color_frames <= 1 {
                            parent.set_display_color(ColorKind::Cyan.color_ref());
                        }
                        self.spore_movement(parent, false)?;
                    }
                    self.dormant_hatching(parent_ref, parent, peer)?;
                }
            }
            _ => {
                if parent.effects.is_injured
                    && !self.use_decay_energy(parent, min(0.001, parent.state.energy))?
                {
                    return Ok(());
                }
                if parent.state.energy <= TOLERANCE {
                    self.record_exhaustion(parent)?;
                } else {
                    self.dormant_hatching(parent_ref, parent, peer)?;
                }
            }
        }
        Ok(())
    }

    fn dormant_hatching(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        if parent.state.reproduction.countdown == 0 {
            if !parent.effects.is_injured
                && (!parent.state.world.resolve()?.fast_check_hit(parent, parent_ref, parent, peer)?
                    || parent.effects.gold == 0)
            {
                self.hatch(parent)?;
            } else if parent.effects.gold > 0 {
                parent.effects.gold = parent.effects.gold.wrapping_sub(1);
            }
        } else {
            parent.state.reproduction.countdown =
                parent.state.reproduction.countdown.wrapping_sub(1);
        }
        Ok(())
    }

    /// Restore a dormant spore, or preserve partial spending when hatching fails
    pub(super) fn hatch(&self, organism: &mut Organism) -> Result<(), WorldError> {
        if organism.use_energy(
            self.session.0.parameters.borrow().spore_energy_consumption)? {
            restore_gene_colors(organism, true).map_err(WorldError::Geometry)?;
            organism.state.temporary_color_frames = 0;
            organism.effects.spore_version = if primary_genome(&organism.genome)?.borrow().modifiers().spore <= 9 {
                -2
            } else {
                -3
            };
            organism.state.reproduction.countdown = 0;
            organism.effects.gold = 0;
            organism.effects.is_plant = false;
            organism.effects.is_frozen = false;
            organism.effects.is_injured = false;
            organism.effects.all_frozen = false;
            organism.state.active = true;
            organism.state.has_moved = true;
            organism.state.age = organism.state.age.wrapping_add(1);
            self.refresh_hatched_effects(organism)?;
        } else {
            organism.use_energy(
                min(0.01, organism.state.energy))?;
            if organism.state.energy <= TOLERANCE {
                self.record_exhaustion(organism)?;
            }
        }
        Ok(())
    }

    fn refresh_hatched_effects(&self, organism: &mut Organism) -> Result<(), WorldError> {
        let genome = organism.genome.as_ref().map(|code| code.borrow());
        organism.effects.apply(
            genome.as_deref(),
            &mut organism.body,
            organism.state.age,
            &mut organism.state.reproduction,
            &self.session.0.parameters.borrow().effects,
        )?;
        drop(genome);
        if organism.body.eyes.enabled {
            organism.rebuild_body().map_err(WorldError::Geometry)?;
        }
        Ok(())
    }
}

fn restore_gene_colors(organism: &mut Organism, clear_photo: bool) -> Result<(), GeometryError> {
    for index in organism.body.indices() {
        let destination = organism.body.phenotype_colors.clone();
        let code = primary_genome(&organism.genome)?.borrow();
        if code.gene_count()? == 0 {
            return Err(GeometryError::EmptyGenome);
        }
        let value = code.gene_for_segment(index)?.borrow().color_handle().cloned();
        crate::Body::store_color(destination, index, value)?;
        if clear_photo {
            organism.body.set_photo(index, 0.0)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorKind, Genome, JavaRandom, WorldConfig};
    use std::rc::Rc;

    fn world_and_actor() -> (World, OrganismRef) {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        world
            .paste_genome(Genome::random(&mut JavaRandom::new(0)), [100, 100])
            .unwrap()
            .unwrap();
        let actor = Rc::clone(&world.organisms.borrow()[0]);
        actor.borrow_mut().effects.maintenance = 0.0;
        (world, actor)
    }

    #[test]
    fn energy_exhaustion_retains_phenotype_and_infection_instead_of_applying_full_death() {
        let (world, actor) = world_and_actor();
        let mut parent = actor.borrow_mut();
        parent.state.energy = 0.0;
        parent.effects.photosynthesis = 7.0;
        parent.infected_genome = Some(Rc::clone(parent.genome.as_ref().unwrap()));
        parent.saved_genome = Some(Rc::clone(parent.genome.as_ref().unwrap()));
        let colors: Vec<_> = (parent.body.indices()).map(|index| parent.body.color_argb(index).unwrap()).collect();
        world.breath_from(&actor, &mut parent, None).unwrap();
        assert!(!parent.state.alive && !parent.state.active);
        assert_eq!(world.population(), 0);
        assert_eq!(parent.effects.photosynthesis, 7.0);
        assert!(Rc::ptr_eq(
            parent.infected_genome.as_ref().unwrap(), parent.genome.as_ref().unwrap()));
        assert!(Rc::ptr_eq(
            parent.saved_genome.as_ref().unwrap(), parent.genome.as_ref().unwrap()));
        assert_eq!(
            (parent.body.indices()).map(|index| parent.body.color_argb(index).unwrap())
                .collect::<Vec<_>>(),
            colors
        );
        assert!(Rc::ptr_eq(
            world.statistics_ref().borrow().get_last_dead_being().unwrap(), parent.genome.as_ref().unwrap()));
    }

    #[test]
    fn age_uses_the_incremented_signed_frame_counter_and_full_death_path() {
        for (age, final_age, survives) in [
            (254, 255, true),
            (255, 256, false),
            (i32::MAX, i32::MIN, true),
        ] {
            let (world, actor) = world_and_actor();
            let mut parent = actor.borrow_mut();
            parent.state.age = age;
            parent.state.reproduction.maximum_age = 0;
            parent.effects.photosynthesis = 7.0;
            world.breath_from(&actor, &mut parent, None).unwrap();
            assert_eq!(parent.state.age, final_age);
            assert_eq!(parent.state.alive, survives);
            assert_eq!(
                parent.effects.photosynthesis,
                if survives { 7.0 } else { 0.0 }
            );
            if !survives {
                assert!((parent.body.indices()).all(|index| {
                    [
                        ColorKind::Eye.argb().unwrap(),
                        ColorKind::Brown.argb().unwrap(),
                    ]
                    .contains(&parent.body.color_argb(index).unwrap())
                }));
            }
        }
    }

    #[test]
    fn excess_energy_preserves_java_integer_overflow_and_negative_spending() {
        let (world, actor) = world_and_actor();
        world.oxygen.set(10.0);
        world.carbon_dioxide.set(20.0);
        let mut parent = actor.borrow_mut();
        parent.state.energy = 2_000_000_000.0;
        parent.state.reproduction.energy = 1_500_000_000;
        parent.state.reproduction.countdown = 8;
        parent.effects.reproduce_late = 1;
        parent.effects.is_infectious = false;
        world.breath_from(&actor, &mut parent, None).unwrap();
        assert_eq!(parent.state.energy, 3_000_000_000.0);
        assert_eq!(world.oxygen.get(), 1_000_000_010.0);
        assert_eq!(world.carbon_dioxide.get(), -999_999_980.0);
        assert_eq!(parent.state.reproduction.countdown, 0);
    }

    #[test]
    fn dormant_types_use_distinct_maximum_age_comparisons() {
        for (version, survives) in [(1, false), (2, true), (5, false)] {
            let (world, actor) = world_and_actor();
            let mut parent = actor.borrow_mut();
            parent.state.age = 255;
            parent.state.active = false;
            parent.state.reproduction.maximum_age = 1;
            parent.effects.spore_version = version;
            parent.effects.is_frozen = true;
            world.decay_from(&actor, &mut parent, None).unwrap();
            assert_eq!(parent.state.age, 256);
            assert_eq!(parent.state.alive, survives, "Spore version {version}");
            assert_eq!(world.population(), i32::from(survives));
        }
    }

    #[test]
    fn corpse_decomposition_uses_partial_oxygen_without_respiration_or_another_death() {
        let (mut world, actor) = world_and_actor();
        let mut parent = actor.borrow_mut();
        world.die(&mut parent, None).unwrap();
        parent.state.energy = 1.0;
        world.oxygen.set(0.05);
        world.carbon_dioxide.set(20.0);
        world.methane.set(30.0);
        for _ in 0..2 {
            world.decay_from(&actor, &mut parent, None).unwrap();
            assert_eq!(parent.state.energy, 0.95);
            assert_eq!(world.oxygen.get(), 0.0);
            assert_eq!(world.carbon_dioxide.get(), 20.0);
            assert_eq!(world.methane.get(), 30.05);
            assert_eq!(world.population(), 0);
        }
        world
            .statistics_ref()
            .borrow_mut()
            .event_time(0, 0.0, 20.0, 30.05).unwrap();
        assert_eq!(crate::statistics::tests::samples(world.statistics_ref().borrow().get_death_list()), [1.0]);
    }

    #[test]
    fn failed_hatching_retains_the_partial_payment_and_inactive_phenotype() {
        let (world, actor) = world_and_actor();
        world.session.0.parameters.borrow_mut().spore_energy_consumption = 2.0;
        world.oxygen.set(1.0);
        world.carbon_dioxide.set(20.0);
        let mut parent = actor.borrow_mut();
        parent.state.active = false;
        parent.state.temporary_color_frames = 6;
        let colors: Vec<_> = (parent.body.indices()).map(|index| parent.body.color_argb(index).unwrap())
            .collect();
        world.hatch(&mut parent).unwrap();
        assert_eq!(parent.state.energy, 39.0);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 21.0);
        assert!(!parent.state.active && parent.state.alive);
        assert_eq!(parent.state.temporary_color_frames, 6);
        assert_eq!(
            (parent.body.indices()).map(|index| parent.body.color_argb(index).unwrap())
                .collect::<Vec<_>>(),
            colors
        );
    }

    #[test]
    fn null_hatched_refresh_retains_shared_prefix_before_missing_genome() {
        let (world, actor) = world_and_actor();
        let mut actor = actor.borrow_mut();
        actor.genome = None;
        actor.effects.spin = 11;
        actor.effects.healing = 12;
        actor.effects.antiviral = 13;
        actor.effects.create_lavender = 14;
        actor.effects.yellow_count = 15;
        actor.effects.indigo = 16;
        actor.effects.gold = 17;
        actor.effects.reproduce_late = 18;
        actor.effects.sibling_battle = true;
        actor.effects.altruist = true;
        actor.state.reproduction.early_energy = 19;
        actor.body.eyes.enabled = true;
        actor.body.update_effects = 0;
        let photo = actor.body.photo_values().unwrap();
        let population = world.population();
        assert_eq!(world.refresh_hatched_effects(&mut actor), Err(WorldError::Geometry(GeometryError::MissingGenome)));
        assert_eq!((actor.effects.spin, actor.effects.healing, actor.effects.antiviral, actor.effects.create_lavender), (0, 0, 0, 0));
        assert_eq!((actor.effects.yellow_count, actor.effects.indigo, actor.effects.gold, actor.state.reproduction.early_energy), (0, 0, 0, 0));
        assert_eq!(actor.effects.reproduce_late, 0);
        assert!(!actor.body.eyes.enabled);
        assert_eq!(actor.body.update_effects, 2);
        assert!(actor.effects.sibling_battle && actor.effects.altruist);
        assert_eq!(actor.body.photo_values().unwrap(), photo);
        assert_eq!(world.population(), population);
        assert!(actor.genome.is_none());
    }


    #[test]
    fn dormant_gains_use_parent_world_after_existing_branch_guards() {
        let (caller, reference) = world_and_actor();
        let target = World::empty_in_session(caller.config.get(), caller.session());
        caller.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        caller.carbon_dioxide.set(100.0);
        caller.methane.set(20.0);
        caller.oxygen.set(50.0);
        target.methane.set(2.0);
        target.oxygen.set(0.0);
        let mut parent = reference.borrow_mut();
        parent.effects = crate::EffectsState::default();
        parent.effects.spore_version = 6;
        parent.effects.methanotrophy = 1.0;
        parent.effects.photosynthesis = 3.0;
        parent.state.world = target.association();
        parent.state.age = 7;
        parent.state.energy = 10.0;
        parent.state.reproduction.energy = 1000;
        parent.state.reproduction.maximum_age = 1000;
        caller.decay_from(&reference, &mut parent, None).unwrap();
        assert_eq!((parent.state.age, parent.state.energy), (8, 11.0));
        assert_eq!((target.methane.get(), target.oxygen.get()), (1.0, 1.0));
        parent.state.world = Default::default();
        parent.effects.is_lime = true;
        caller.decay_from(&reference, &mut parent, None).unwrap();
        assert_eq!((parent.state.age, parent.state.energy), (9, 11.0));
        parent.effects.is_lime = false;
        parent.effects.methanotrophy = 0.0;
        for (colony, gain) in [(0.0, 3.0), (2.0, 5.0)] {
            parent.state.world = target.association();
            parent.effects.colony_photosynthesis = colony;
            parent.state.energy = 10.0;
            target.carbon_dioxide.set(20.0);
            target.oxygen.set(0.0);
            caller.decay_from(&reference, &mut parent, None).unwrap();
            assert_eq!(parent.state.energy, 10.0 + gain);
            assert_eq!((target.carbon_dioxide.get(), target.oxygen.get()), (20.0 - gain, gain));
            assert_eq!(parent.effects.colony_photosynthesis, 0.0);
        }
        parent.state.world = Default::default();
        parent.state.energy = 10.0;
        parent.effects.colony_photosynthesis = 2.0;
        assert_eq!(caller.decay_from(&reference, &mut parent, None), Err(WorldError::MissingWorld));
        assert_eq!(parent.state.energy, 10.0);
        assert_eq!(parent.effects.colony_photosynthesis, 2.0);
        assert_eq!((caller.carbon_dioxide.get(), caller.methane.get(), caller.oxygen.get()), (100.0, 20.0, 50.0));
    }

    #[test]
    fn dormant_lime_uses_associated_tree_and_reuses_held_actor_identities() {
        for owner_hit in [false, true] {
            let (caller, reference) = world_and_actor();
            let (_peer_world, peer_ref) = world_and_actor();
            let target = World::empty_in_session(caller.config.get(), caller.session());
            caller.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
            let bounds = crate::IntRect { x: 100, y: 100, width: 10, height: 10 };
            reference.borrow().state.bounds.set(bounds);
            peer_ref.borrow().state.bounds.set(bounds);
            for (world, hit) in [(&caller, !owner_hit), (&target, owner_hit)] {
                world.frame_objects.replace(vec![Rc::clone(&reference), Rc::clone(&peer_ref)]);
                let mut entries = vec![(0, bounds)];
                if hit { entries.push((1, bounds)); }
                world.spatial.replace(crate::spatial::SpatialTree::build_actor_entries(entries, |token| world.spatial_actor(token)));
            }
            caller.carbon_dioxide.set(100.0);
            caller.oxygen.set(50.0);
            target.carbon_dioxide.set(20.0);
            target.oxygen.set(0.0);
            let mut parent = reference.borrow_mut();
            parent.effects = crate::EffectsState::default();
            parent.effects.spore_version = 6;
            parent.effects.is_lime = true;
            parent.effects.photosynthesis = 9.0;
            parent.effects.leaf_photo = 2.0;
            parent.state.world = target.association();
            parent.state.energy = 10.0;
            parent.state.reproduction.energy = 1000;
            parent.state.reproduction.maximum_age = 1000;
            let peer = peer_ref.borrow_mut();
            caller.decay_from(&reference, &mut parent, Some((&peer_ref, &peer))).unwrap();
            let gain = if owner_hit { 9.0 } else { 2.0 };
            assert_eq!(parent.state.energy, 10.0 + gain);
            assert_eq!((target.carbon_dioxide.get(), target.oxygen.get()), (20.0 - gain, gain));
            assert_eq!((caller.carbon_dioxide.get(), caller.oxygen.get()), (100.0, 50.0));
            parent.state.world = Default::default();
            parent.effects.colony_photosynthesis = 7.0;
            let energy = parent.state.energy;
            assert_eq!(caller.decay_from(&reference, &mut parent, Some((&peer_ref, &peer))), Err(WorldError::MissingWorld));
            assert_eq!(parent.state.energy, energy);
            assert_eq!(parent.effects.colony_photosynthesis, 7.0);
        }
    }
    fn hatching_actor() -> (World, OrganismRef) {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        }).unwrap();
        let gene = crate::Gene::new(12.0, 0.0, ColorKind::White.argb(), -1, [0; 36]).shared();
        world.paste_genome(Genome::from_parts(&[gene], crate::GenomeProperties {
            symmetry: 1,
            ..crate::GenomeProperties::default()
        }), [100, 100]).unwrap().unwrap();
        let reference = Rc::clone(&world.organisms.borrow()[0]);
        (world, reference)
    }

    fn opposite_hatching_trees(
        caller: &World,
        owner: &World,
        parent: &OrganismRef,
        peer: &OrganismRef,
        owner_hit: bool,
    ) -> crate::IntRect {
        let bounds = crate::IntRect { x: 100, y: 100, width: 10, height: 10 };
        parent.borrow().state.bounds.set(bounds);
        peer.borrow().state.bounds.set(bounds);
        for (world, hit) in [(caller, !owner_hit), (owner, owner_hit)] {
            world.frame_objects.replace(vec![Rc::clone(parent), Rc::clone(peer)]);
            let mut entries = vec![(0, bounds)];
            if hit { entries.push((1, bounds)); }
            world.spatial.replace(crate::spatial::SpatialTree::build_actor_entries(entries, |token| world.spatial_actor(token)));
        }
        bounds
    }

    #[test]
    fn dormant_hatching_uses_current_owner_and_reuses_held_parent_and_peer() {
        for owner_hit in [false, true] {
            let (caller, reference) = hatching_actor();
            let (_peer_world, peer_ref) = hatching_actor();
            let owner = World::empty_in_session(caller.config.get(), caller.session());
            caller.session.0.parameters.borrow_mut().spore_energy_consumption = 0.0;
            let bounds = opposite_hatching_trees(&caller, &owner, &reference, &peer_ref, owner_hit);
            let random = caller.random_snapshot();
            let mut parent = reference.borrow_mut();
            let peer = peer_ref.borrow_mut();
            let _caller_members = caller.organisms.borrow_mut();
            let _owner_members = owner.organisms.borrow_mut();
            for (receiver, associated_owner, hit) in [
                (&caller, true, owner_hit), (&owner, false, !owner_hit),
                (&caller, true, owner_hit), (&owner, true, owner_hit),
            ] {
                parent.state.world = if associated_owner { owner.association() } else { caller.association() };
                parent.state.bounds.set(bounds);
                parent.effects = crate::EffectsState::default();
                parent.effects.gold = 2;
                parent.state.active = false;
                parent.state.has_moved = false;
                parent.state.alive = true;
                parent.state.age = 7;
                parent.state.energy = 10.0;
                parent.state.reproduction.countdown = 0;
                receiver.dormant_hatching(&reference, &mut parent, Some((&peer_ref, &peer))).unwrap();
                assert_eq!(parent.effects.gold, if hit { 1 } else { 0 });
                assert_eq!(parent.state.active, !hit);
                assert_eq!(parent.state.has_moved, !hit);
                assert_eq!(parent.state.age, if hit { 7 } else { 8 });
                assert_eq!(parent.state.reproduction.countdown, 0);
                assert_eq!(parent.state.energy, 10.0);
                assert_eq!(caller.random_snapshot(), random);
            }
        }
    }

    #[test]
    fn dormant_hatching_keeps_countdown_injury_and_gold_zero_owner_order() {
        for expired in [false, true] {
            for (countdown, injured, gold, next_countdown, next_gold, reached) in [
                (1, false, 2, 0, 2, false),
                (-1, false, 2, -2, 2, false),
                (i32::MIN, false, 2, i32::MAX, 2, false),
                (0, true, 2, 0, 1, false),
                (0, true, 0, 0, 0, false),
                (0, true, -1, 0, -1, false),
                (0, false, 2, 0, 2, true),
                (0, false, 0, 0, 0, true),
            ] {
                let (caller, reference) = hatching_actor();
                let (_peer_world, peer_ref) = hatching_actor();
                let owner = World::empty_in_session(caller.config.get(), caller.session());
                opposite_hatching_trees(&caller, &owner, &reference, &peer_ref, false);
                let association = if expired { owner.association() } else { Default::default() };
                drop(owner);
                let mut parent = reference.borrow_mut();
                parent.state.world = association;
                parent.effects.gold = gold;
                parent.effects.is_injured = injured;
                parent.state.reproduction.countdown = countdown;
                parent.state.active = false;
                let energy = parent.state.energy;
                let random = caller.random_snapshot();
                let colors: Vec<_> = parent.body.indices()
                    .map(|index| parent.body.color_argb(index).unwrap()).collect();
                let result = caller.dormant_hatching(&reference, &mut parent, None);
                assert_eq!(result, if reached { Err(if expired {
                    WorldError::ExpiredWorld
                } else {
                    WorldError::MissingWorld
                }) } else { Ok(()) });
                assert_eq!(parent.state.reproduction.countdown, next_countdown);
                assert_eq!(parent.effects.gold, next_gold);
                assert!(!parent.state.active);
                assert_eq!(parent.state.energy, energy);
                assert_eq!(parent.body.indices().map(|index| parent.body.color_argb(index).unwrap())
                    .collect::<Vec<_>>(), colors);
                assert_eq!(caller.random_snapshot(), random);
            }
        }
    }

    #[test]
    fn dormant_query_preserves_preceding_payment_color_movement_and_rng() {
        for missing_motion_array in [false, true] {
            let (caller, reference) = hatching_actor();
            let (_peer_world, peer_ref) = hatching_actor();
            let owner = World::empty_in_session(caller.config.get(), caller.session());
            opposite_hatching_trees(&caller, &owner, &reference, &peer_ref, true);
            caller.session.0.parameters.borrow_mut().physics.max_velocity = 10.0;
            caller.session.0.parameters.borrow_mut().physics.max_rotation = 10.0;
            caller.oxygen.set(50.0);
            caller.carbon_dioxide.set(100.0);
            owner.oxygen.set(5.0);
            owner.carbon_dioxide.set(20.0);
            caller.session.0.random.replace(JavaRandom::new(18));
            let mut expected = JavaRandom::new(18);
            assert_eq!(expected.bounded(100), 0);
            let mut parent = reference.borrow_mut();
            parent.state.world = owner.association();
            parent.effects = crate::EffectsState::default();
            parent.effects.spore_version = -3;
            parent.effects.gold = 2;
            parent.state.active = false;
            parent.state.alive = true;
            parent.state.energy = 10.0;
            parent.state.age = 7;
            parent.state.temporary_color_frames = 0;
            parent.state.reproduction.countdown = 0;
            parent.state.velocity = [0.5, -0.5];
            parent.state.angular_velocity = 0.0;
            parent.body.mass = 12.0;
            parent.body.inertia = PI;
            parent.body.set_physical_length(0, 1.0).unwrap();
            parent.body.set_rotated_segment(0, crate::RotatedSegment {
                start: [0, 0], end: [1, 2],
            }).unwrap();
            if missing_motion_array {
                parent.body.arrays.x2 = None;
            } else {
                assert_eq!(expected.bounded(2), 1);
            }
            let peer = peer_ref.borrow_mut();
            let result = caller.decay_from(&reference, &mut parent, Some((&peer_ref, &peer)));
            if missing_motion_array {
                assert_eq!(result, Err(WorldError::Geometry(GeometryError::MissingPrimitiveArray {
                    field: "x2",
                })));
                assert_eq!(parent.state.velocity, [0.5, -0.5]);
                assert_eq!(parent.state.angular_velocity, 0.0);
                assert_eq!(parent.effects.gold, 2);
            } else {
                assert_eq!(result, Ok(()));
                assert_eq!(parent.state.velocity, [1.5, 1.5]);
                assert_eq!(parent.state.angular_velocity, 1.0);
                assert_eq!(parent.effects.gold, 1);
            }
            assert_eq!(parent.state.energy, 10.0 - 0.001);
            assert_eq!(parent.state.temporary_argb(), ColorKind::Cyan.argb());
            assert_eq!(parent.state.temporary_color_frames, 10);
            assert!(!parent.state.active);
            assert_eq!(parent.state.age, 7);
            assert_eq!((owner.oxygen.get(), owner.carbon_dioxide.get()), (5.0 - 0.001, 20.0 + 0.001));
            assert_eq!((caller.oxygen.get(), caller.carbon_dioxide.get()), (50.0, 100.0));
            assert_eq!(caller.random_snapshot(), expected);
        }
    }


    #[test]
    fn exhaustion_owner_notifications_keep_branch_prefixes_and_cached_state() {
        for active_breath in [false, true] {
            let (caller, actor_ref) = world_and_actor();
            let owner = World::with_session(
                WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, caller.session(),
            ).unwrap();
            owner.population.set(9);
            let random = caller.random_snapshot();
            let mut actor = actor_ref.borrow_mut();
            actor.state.world = owner.association();
            actor.state.energy = 0.0;
            actor.state.age = 0;
            actor.state.reproduction.maximum_age = 10;
            actor.state.alive = true;
            actor.state.active = true;
            actor.effects.spore_version = 2;
            actor.effects.photosynthesis = 7.0;
            actor.infected_genome = actor.genome.clone();
            actor.saved_genome = actor.genome.clone();
            let colors: Vec<_> = actor.body.indices().map(|i| actor.body.color_argb(i).unwrap()).collect();
            if active_breath {
                caller.breath_from(&actor_ref, &mut actor, None).unwrap();
            } else {
                caller.decay_from(&actor_ref, &mut actor, None).unwrap();
            }
            assert!(!actor.state.alive);
            assert_eq!(actor.state.active, !active_breath);
            assert_eq!(actor.state.age, 1);
            assert_eq!(owner.population(), 8);
            assert_eq!(caller.population(), 1);
            assert!(Rc::ptr_eq(owner.statistics_ref().borrow().get_last_dead_being().unwrap(), actor.genome.as_ref().unwrap()));
            assert!(caller.statistics_ref().borrow().get_last_dead_being().is_none());
            assert_eq!(actor.effects.photosynthesis, 7.0);
            assert!(Rc::ptr_eq(actor.infected_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
            assert!(Rc::ptr_eq(actor.saved_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
            assert_eq!(actor.body.indices().map(|i| actor.body.color_argb(i).unwrap()).collect::<Vec<_>>(), colors);
            assert_eq!(caller.random_snapshot(), random);
        }
        for expired in [false, true] {
            let (caller, actor_ref) = world_and_actor();
            let random = caller.random_snapshot();
            let mut actor = actor_ref.borrow_mut();
            actor.state.world = if expired {
                let temporary = World::with_session(
                    WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, caller.session(),
                ).unwrap();
                temporary.association()
            } else { Default::default() };
            actor.state.energy = 0.0;
            actor.state.age = 0;
            actor.state.reproduction.maximum_age = 10;
            actor.state.alive = true;
            actor.state.active = true;
            actor.effects.spore_version = 2;
            actor.effects.photosynthesis = 7.0;
            actor.infected_genome = actor.genome.clone();
            actor.saved_genome = actor.genome.clone();
            assert_eq!(caller.decay_from(&actor_ref, &mut actor, None),
                Err(if expired { WorldError::ExpiredWorld } else { WorldError::MissingWorld }));
            assert!(!actor.state.alive);
            assert!(actor.state.active);
            assert_eq!(actor.state.age, 1);
            assert_eq!(actor.effects.photosynthesis, 7.0);
            assert!(Rc::ptr_eq(actor.infected_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
            assert!(Rc::ptr_eq(actor.saved_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
            assert_eq!(caller.population(), 1);
            assert!(caller.statistics_ref().borrow().get_last_dead_being().is_none());
            assert_eq!(caller.random_snapshot(), random);
        }
    }
}
