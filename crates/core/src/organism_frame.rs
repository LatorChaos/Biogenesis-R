// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Organism.move motion, rollback, and final energy gate from Java4177-4232/4559-4560
//!
//! Outgoing admission runs after border response and before physical contacts

use super::{Organism, OrganismRef, World, WorldError};
use crate::{TOLERANCE, movement};

impl World {
    /// Move one held organism, preserving completed effects when an ordered operation fails
    pub(super) fn move_actor_frame(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
    ) -> Result<bool, WorldError> {
        let destination = actor.state.last_frame.clone();
        let value = actor.state.bounds.get();
        crate::rectangles::required_rectangle(destination.as_ref(), "lastFrame")
            .map_err(WorldError::Geometry)?.set(value);
        movement::rubbing(actor, self.session.0.parameters.borrow().lifecycle.rubbing);
        movement::grow(actor).map_err(WorldError::Geometry)?;
        if actor.state.temporary_color_frames > 0 {
            actor.state.temporary_color_frames = actor.state.temporary_color_frames.wrapping_sub(1);
        }
        if actor.state.velocity[0] != 0.0
            || actor.state.velocity[1] != 0.0
            || actor.state.angular_velocity != 0.0
            || actor.state.has_grown != 0
            || (actor.body.eyes.enabled && actor.effects.can_move)
        {
            let translation = actor.state.velocity;
            let rotation = actor.state.angular_velocity;
            movement::offset(actor, translation, rotation);
            movement::calculate_bounds(actor, actor.state.has_grown != 0)
                .map_err(WorldError::Geometry)?;
            let mut collision = !self.is_inside_world(actor)?;
            let corridor_world = actor.state.world.resolve()?;
            if corridor_world.corridors_exist.get() && actor.state.active {
                if let Some(corridor) = corridor_world.check_outgoing(actor) {
                    let available = {
                        let corridor = corridor.borrow();
                        let available = corridor.can_send() && !corridor.connection.borrow().pending.is_full();
                        available
                    };
                    if available && corridor_world.send_outgoing(&corridor, actor_ref, actor)? {
                        return Ok(false);
                    }
                }
            }
            if self.check_actor_contact_from(actor_ref, actor)?.is_some() {
                collision = true;
            }
            if collision {
                actor.state.has_moved = false;
                movement::offset(actor, translation.map(|value| -value), -rotation);
                if actor.state.has_grown != 0 {
                    actor.body.growth_ratio = actor.body.growth_ratio.wrapping_add(actor.state.has_grown);
                    actor.rebuild_body().map_err(WorldError::Geometry)?;
                    if actor.body.update_effects == 1 {
                        actor.body.update_effects = 0;
                    }
                }
                movement::calculate_bounds(actor, actor.state.has_grown != 0)
                    .map_err(WorldError::Geometry)?;
            } else {
                actor.state.has_moved = true;
            }
        } else {
            actor.state.has_moved = false;
        }
        self.finish_actor_frame(actor_ref, actor)?;
        Ok(actor.state.energy > TOLERANCE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorKind, EffectsState, Gene, Genome, GenomeProperties, IntRect, WorldConfig};
    use std::{f64::consts::FRAC_PI_2, rc::Rc};

    fn add_actor(world: &mut World, colors: &[ColorKind], center: [i32; 2]) -> OrganismRef {
        let genes = colors
            .iter()
            .map(|color| Gene::new(12.0, 0.0, color.argb(), 0, [0; 36]).shared())
            .collect::<Vec<_>>();
        world
            .paste_genome(
                Genome::from_parts(
                    &genes,
                    GenomeProperties {
                        symmetry: 1,
                        ..Default::default()
                    },
                ),
                center,
            )
            .unwrap()
            .unwrap();
        let reference = Rc::clone(world.organisms.borrow().last().unwrap());
        {
            let mut actor = reference.borrow_mut();
            actor.effects = EffectsState {

                ..Default::default()
            };
            actor.effects.summer_inactivity = -1;
            actor.body.growth_ratio = 1;
            actor.rebuild_body().unwrap();
            actor.body.update_effects = 0;
            actor.state.angle = 0.0;
            actor.state.velocity = [0.0; 2];
            actor.state.angular_velocity = 0.0;
            actor.state.energy = 100.0;
            actor.state.age = 0;
            actor.state.parent_id = -1;
            actor.state.reproduction.energy = 10000;
            actor.state.reproduction.early_energy = 0;
            actor.state.reproduction.countdown = 10;
            actor.state.reproduction.maximum_countdown = 10;
            actor.state.reproduction.maximum_age = 1000;
            actor.state.temporary_color = None;
            actor.state.temporary_color_frames = 0;
            movement::calculate_bounds(&mut actor, true).unwrap();
        }
        reference
    }

    fn empty_world() -> World {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..Default::default()
        })
        .unwrap();
        world.session.0.parameters.borrow_mut().lifecycle.rubbing = 1.0;
        world.oxygen.set(1000.0);
        world.carbon_dioxide.set(500000.0);
        world.methane.set(0.0);
        world
    }

    fn setup() -> (World, OrganismRef) {
        let mut world = empty_world();
        let actor = add_actor(&mut world, &[ColorKind::Green], [1000, 1000]);
        world.oxygen.set(1000.0);
        world.carbon_dioxide.set(0.0);
        world.rebuild_spatial();
        (world, actor)
    }

    fn move_held(world: &mut World, reference: &OrganismRef) -> Result<bool, WorldError> {
        world.move_actor_frame(reference, &mut reference.borrow_mut())
    }

    fn relocate(reference: &OrganismRef, center: [f64; 2], angle: f64) {
        let mut actor = reference.borrow_mut();
        actor.state.center = center;
        actor.state.integer_center = center.map(|value| value as i32);
        actor.state.angle = angle;
        movement::calculate_bounds(&mut actor, true).unwrap();
    }

    #[test]
    fn stationary_frames_preserve_cached_bounds_and_only_decrement_positive_display_time() {
        let (mut world, reference) = setup();
        let sentinel = IntRect {
            x: 10,
            y: 20,
            width: 30,
            height: 40,
        };
        {
            let mut actor = reference.borrow_mut();
            actor.state.bounds.set(sentinel);
            actor.state.last_angle = -5.0;
            actor.state.velocity = [-0.0, 0.0];
            actor.state.angular_velocity = -0.0;
            actor.state.temporary_color_frames = 2;
        }
        assert!(move_held(&mut world, &reference).unwrap());
        {
            let actor = reference.borrow();
            assert_eq!(actor.state.last_frame.as_ref().unwrap().get(), sentinel);
            assert_eq!(actor.state.bounds.get(), sentinel);
            assert_eq!(actor.state.last_angle, -5.0);
            assert_eq!(actor.state.center, [1000.0; 2]);
            assert!(!actor.state.has_moved);
            assert_eq!(actor.state.age, 1);
            assert_eq!(actor.state.temporary_color_frames, 1);
        }
        reference.borrow_mut().state.temporary_color_frames = -1;
        move_held(&mut world, &reference).unwrap();
        assert_eq!(reference.borrow().state.temporary_color_frames, -1);
    }

    #[test]
    fn ordinary_motion_uses_rubbed_velocity_and_the_original_frame_rectangle() {
        let (mut world, reference) = setup();
        world.session.0.parameters.borrow_mut().lifecycle.rubbing = 0.5;
        let bounds = reference.borrow().state.bounds.get();
        {
            let mut actor = reference.borrow_mut();
            actor.state.velocity = [2.0, -4.0];
            actor.state.angular_velocity = 0.5;
            actor.state.temporary_color_frames = 1;
        }
        assert!(move_held(&mut world, &reference).unwrap());
        let actor = reference.borrow();
        assert_eq!(actor.state.last_frame.as_ref().unwrap().get(), bounds);
        assert_eq!(actor.state.center, [1001.0, 998.0]);
        assert_eq!(actor.state.integer_center, [1001, 998]);
        assert_eq!(actor.state.angle, 0.25);
        assert_eq!(actor.state.last_angle, 0.25);
        assert_eq!(actor.state.velocity, [1.0, -2.0]);
        assert_eq!(actor.state.angular_velocity, 0.25);
        assert!(actor.state.has_moved);
        assert_eq!(actor.state.temporary_color_frames, 0);
        assert_eq!(actor.state.age, 1);
    }

    #[test]
    fn border_rollback_retains_reflected_velocity_and_cleared_angular_velocity() {
        let (mut world, reference) = setup();
        relocate(&reference, [8.0, 1000.0], 0.0);
        let bounds = reference.borrow().state.bounds.get();
        {
            let mut actor = reference.borrow_mut();
            actor.state.velocity = [-4.0, 0.0];
            actor.state.angular_velocity = 0.1;
        }
        world.rebuild_spatial();
        move_held(&mut world, &reference).unwrap();
        let actor = reference.borrow();
        assert_eq!(actor.state.center, [8.0, 1000.0]);
        assert_eq!(actor.state.angle, 0.0);
        assert_eq!(actor.state.bounds.get(), bounds);
        assert_eq!(actor.state.last_frame.as_ref().unwrap().get(), bounds);
        assert_eq!(actor.state.velocity, [4.0, 0.0]);
        assert_eq!(actor.state.angular_velocity, 0.0);
        assert!(!actor.state.has_moved);
    }

    #[test]
    fn physical_collision_rolls_back_translation_without_reversing_the_impulse() {
        let (mut world, reference) = setup();
        let peer = add_actor(&mut world, &[ColorKind::Green], [3000, 3000]);
        relocate(&peer, [1000.0; 2], FRAC_PI_2);
        reference.borrow_mut().state.velocity = [0.0, 1.0];
        world.rebuild_spatial();
        move_held(&mut world, &reference).unwrap();
        let actor = reference.borrow();
        let peer = peer.borrow();
        assert_eq!(actor.state.center, [1000.0; 2]);
        assert_eq!(peer.state.center, [1000.0; 2]);
        assert!(!actor.state.has_moved);
        assert!(actor.state.velocity[1] > 0.0 && actor.state.velocity[1] < 1.0);
        assert!(peer.state.velocity[1] > 0.0);
        assert_eq!(actor.state.age, 1);
        assert_eq!(peer.state.age, 0);
    }

    #[test]
    fn border_failure_still_runs_the_ordered_biological_contact() {
        let (mut world, reference) = setup();
        let peer = add_actor(&mut world, &[ColorKind::Green], [3000, 3000]);
        relocate(&reference, [8.0, 1000.0], 0.0);
        relocate(&peer, [4.0, 1000.0], FRAC_PI_2);
        {
            let mut actor = reference.borrow_mut();
            actor.body.set_color_argb(0, ColorKind::Gray.argb().unwrap()).unwrap();
            actor.body.set_photo(0, -2.0).unwrap();
            actor.state.velocity = [-4.0, 0.0];
        }
        world.rebuild_spatial();
        move_held(&mut world, &reference).unwrap();
        assert!(!peer.borrow().state.alive);
        assert_eq!(world.population.get(), 1);
        assert_eq!(reference.borrow().state.center, [8.0, 1000.0]);
        assert!(!reference.borrow().state.has_moved);
    }

    #[test]
    fn growth_rollback_restores_the_body_but_retains_growth_scaled_motion_and_dirty_priority() {
        for update in [0, 2] {
            let (mut world, reference) = setup();
            world.session.0.parameters.borrow_mut().lifecycle.segment_cost_divisor = 1;
            {
                let mut actor = reference.borrow_mut();
                actor.body.growth_ratio = 2;
                actor.rebuild_body().unwrap();
                actor.body.set_color_argb(0, ColorKind::LightBrown.argb().unwrap()).unwrap();
                actor.body.set_reaction(0, 17, 1234).unwrap();
                actor.body.update_effects = update;
                actor.effects.maintenance = 7.0;
                actor.state.age = 7;
                actor.state.velocity = [-4.0, 0.0];
            }
            relocate(&reference, [8.0, 1000.0], 0.0);
            let body = reference.borrow().body.clone();
            world.rebuild_spatial();
            move_held(&mut world, &reference).unwrap();
            let actor = reference.borrow();
            assert_eq!(actor.body.growth_ratio, 2);
            assert_eq!(actor.body.mass, body.mass);
            assert_eq!(actor.body.inertia, body.inertia);
            assert_eq!(actor.body.segment_values().unwrap(), body.segment_values().unwrap());
            assert_eq!(actor.state.center, [8.0, 1000.0]);
            assert_eq!(actor.state.has_grown, 1);
            assert!(actor.state.velocity[0] > 0.0 && actor.state.velocity[0] < 4.0);
            assert_eq!(actor.body.update_effects, 0);
            assert_eq!(
                actor.effects.maintenance,
                if update == 0 { 7.0 } else { body.mass }
            );
            assert!(!actor.state.has_moved);
        }
    }

    #[test]
    fn eye_motion_gate_runs_false_return_sensing_without_displacement() {
        for can_move in [false, true] {
            let mut world = empty_world();
            let reference = add_actor(
                &mut world,
                &[ColorKind::Teal, ColorKind::Green],
                [1000, 1000],
            );
            let peer = add_actor(&mut world, &[ColorKind::Green], [3000, 3000]);
            relocate(&peer, [1000.0; 2], FRAC_PI_2);
            world.session.0.parameters.borrow_mut().lifecycle.teal_energy_consumption = 1.0;
            {
                let mut actor = reference.borrow_mut();
                actor.body.eyes.enabled = true;
                actor.body.set_physical_length(0, 0.5).unwrap();
                actor.body.set_physical_length(1, 0.5).unwrap();
                actor.body.set_color_argb(1, ColorKind::Eye.argb().unwrap()).unwrap();
                actor.body.set_reaction_values(1, [1; 36]).unwrap();
                actor.effects.can_move = can_move;
                actor.effects.can_react = false;
            }
            world.rebuild_spatial();
            let random = world.random_snapshot();
            move_held(&mut world, &reference).unwrap();
            let actor = reference.borrow();
            assert_eq!(actor.state.center, [1000.0; 2]);
            assert_eq!(actor.state.energy, if can_move { 99.0 } else { 100.0 });
            assert_eq!(actor.state.has_moved, can_move);
            assert_eq!(peer.borrow().state.energy, 100.0);
            assert_eq!(world.random_snapshot(), random);
        }
    }

    fn silver_failure() -> (World, OrganismRef, OrganismRef) {
        let (mut world, reference) = setup();
        let peer = add_actor(&mut world, &[ColorKind::Teal, ColorKind::Red], [3000, 3000]);
        relocate(&peer, [1000.0; 2], FRAC_PI_2);
        {
            let mut actor = reference.borrow_mut();
            actor.body.set_color_argb(0, ColorKind::Silver.argb().unwrap()).unwrap();
            actor.body.set_photo(0, -1.0).unwrap();
            actor.effects.is_enhanced = true;
            actor.state.total_kills = 1;
            actor.state.energy = 1000.0;
            actor.state.velocity = [0.0, 1.0];
            actor.state.temporary_color_frames = 2;
        }
        {
            let mut peer = peer.borrow_mut();
            peer.body.set_physical_length(0, 0.5).unwrap();
            peer.body.set_reaction_values(1, [1; 36]).unwrap();
            peer.effects.can_react = true;
            peer.effects.has_dodged = false;
            peer.state.total_kills = -12;
            peer.state.energy = 1000.0;
        }
        world.session.0.parameters.borrow_mut().lifecycle.experience_energy_consumption = 0.625;
        world.session.0.parameters.borrow_mut().lifecycle.teal_energy_consumption = 1.0;
        world.oxygen.set(1120.0);
        world.carbon_dioxide.set(499880.0);
        world.methane.set(3.0);
        world.rebuild_spatial();
        (world, reference, peer)
    }

    #[test]
    fn arithmetic_failure_keeps_motion_and_payment_before_rollback_reverse_effects_and_breath() {
        let (mut world, reference, peer) = silver_failure();
        let bounds = reference.borrow().state.bounds.get();
        let error = move_held(&mut world, &reference).unwrap_err();
        assert_eq!(
            error,
            WorldError::Arithmetic("SILVER kill ratio has a zero denominator")
        );
        let actor = reference.borrow();
        assert_eq!(actor.state.last_frame.as_ref().unwrap().get(), bounds);
        assert_eq!(actor.state.center, [1000.0, 1001.0]);
        assert_eq!(actor.state.temporary_color_frames, 1);
        assert_eq!(actor.state.age, 0);
        assert_eq!(actor.state.reproduction.countdown, 10);
        assert_eq!(actor.state.energy.to_bits(), 0x408f3b0000000000);
        assert_eq!(world.oxygen.get().to_bits(), 0x40917d8000000000);
        assert_eq!(world.carbon_dioxide.get().to_bits(), 0x411e82a280000000);
        assert_eq!(peer.borrow().state.energy, 1000.0);
        assert!(!peer.borrow().effects.has_dodged);
        assert!(actor.state.velocity[1] < 1.0);
        assert!(peer.borrow().state.velocity[1] > 0.0);
    }

    #[test]
    fn public_failure_pauses_and_aborts_later_actors_clocks_and_atmosphere() {
        for running in [false, true] {
            let (mut world, reference, peer) = silver_failure();
            let later = add_actor(&mut world, &[ColorKind::Green], [5000, 5000]);
            later.borrow_mut().state.temporary_color_frames = 7;
            world.oxygen.set(1120.0);
            world.carbon_dioxide.set(499880.0);
            if running {
                world.resume();
            }
            let tick = world.tick.get();
            let frame = world.frame.get();
            let random = world.random_snapshot();
            let result = if running {
                world.step()
            } else {
                world.single_step()
            };
            assert_eq!(
                result,
                Err(WorldError::Arithmetic(
                    "SILVER kill ratio has a zero denominator"
                ))
            );
            assert!(world.paused());
            assert_eq!(world.tick.get(), tick);
            assert_eq!(world.frame.get(), frame);
            assert_eq!(world.methane.get(), 3.0);
            assert_eq!(world.oxygen.get().to_bits(), 0x40917d8000000000);
            assert_eq!(world.carbon_dioxide.get().to_bits(), 0x411e82a280000000);
            assert_eq!(world.organisms.borrow().len(), 3);
            assert_eq!(world.random_snapshot(), random);
            assert_eq!(reference.borrow().state.age, 0);
            assert_eq!(peer.borrow().state.age, 0);
            assert_eq!(later.borrow().state.age, 0);
            assert_eq!(later.borrow().state.temporary_color_frames, 7);
        }
    }

    #[test]
    fn final_energy_gate_is_strict_and_retains_energetic_corpses() {
        for (energy, keep) in [
            (TOLERANCE, false),
            (f64::from_bits(TOLERANCE.to_bits() + 1), true),
            (f64::NAN, false),
        ] {
            let (mut world, reference) = setup();
            world.session.0.parameters.borrow_mut().lifecycle.decay_energy = 0.0;
            {
                let mut actor = reference.borrow_mut();
                actor.state.active = false;
                actor.state.alive = false;
                actor.state.energy = energy;
            }
            assert_eq!(move_held(&mut world, &reference).unwrap(), keep);
        }
    }
}
