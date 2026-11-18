// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Actor effects, reproduction scheduling, and maintenance from Organism.move:4233-4557

use super::{Organism, OrganismRef, World, WorldError};
use crate::{ColorKind, GROWTH_SCALES, physics::between};
use std::f64::consts::PI;

const SPRING_SCALES: [f64; 16] = [
    0.01, 0.009, 0.008, 0.0071, 0.0063, 0.0056, 0.005, 0.0045, 0.004, 0.0036, 0.0032, 0.0028,
    0.0025, 0.0022, 0.002, 0.0018,
];

fn flower_reproduction_energy(actor: &mut Organism) -> Result<(), WorldError> {
    if actor.effects.reproduce_late > 0 && actor.effects.is_infectious {
        actor.state.reproduction.energy = 40_i32
            .wrapping_add(3_i32.wrapping_mul(actor.body.segment_count))
            .wrapping_add(
                (f64::from(actor.effects.reproduce_late)
                    * GROWTH_SCALES[crate::geometry::growth_index(actor.body.growth_ratio)?])
                    as i32,
            );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EffectsState, Gene, Genome, GenomeProperties, IntRect, JavaRandom, WorldConfig,
        WorldParameters,
    };
    use std::rc::Rc;

    fn actor(color: ColorKind) -> (World, OrganismRef) {
        let mut parameters = WorldParameters::default();
        parameters.genetics.min_mutation_rate = 0;
        parameters.genetics.max_mutation_rate = 0;
        parameters.genetics.meta_mutation_rate = 0;
        let world = World::with_parameters(
            WorldConfig {
                initial_attempts: 0,
                ..WorldConfig::default()
            },
            parameters,
        )
        .unwrap();
        let gene = Gene::new(12.0, 0.0, color.argb(), -1, [0; 36]).shared();
        world
            .paste_genome(
                Genome::from_parts(
                    &[gene],
                    GenomeProperties {
                        symmetry: 1,
                        ..GenomeProperties::default()
                    },
                ),
                [1000, 1000],
            )
            .unwrap()
            .unwrap();
        let actor_ref = Rc::clone(&world.organisms.borrow()[0]);
        {
            let mut actor = actor_ref.borrow_mut();
            actor.effects = EffectsState {

                ..EffectsState::default()
            };
            actor.effects.summer_inactivity = -1;
            actor.body.update_effects = 0;
            actor.state.energy = 100.0;
            actor.state.age = 0;
            actor.state.reproduction.energy = 10000;
            actor.state.reproduction.early_energy = 0;
            actor.state.reproduction.countdown = 10;
            actor.state.reproduction.maximum_countdown = 10;
            actor.state.reproduction.maximum_age = 1000;
            actor.state.temporary_color = None;
            actor.state.temporary_color_frames = 0;
        }
        world.oxygen.set(1000.0);
        world.carbon_dioxide.set(0.0);
        world.methane.set(0.0);
        world.rebuild_spatial();
        (world, actor_ref)
    }

    fn finish(world: &mut World, actor_ref: &OrganismRef) {
        world
            .finish_actor_frame(actor_ref, &mut actor_ref.borrow_mut())
            .unwrap();
    }

    fn adult(world: &mut World, actor_ref: &OrganismRef, ratio: i32) {
        {
            let mut actor = actor_ref.borrow_mut();
            actor.body.growth_ratio = ratio;
            actor.rebuild_body().unwrap();
            crate::movement::calculate_bounds(&mut actor, true).unwrap();
            actor.state.reproduction.energy = 43;
            actor.state.reproduction.countdown = 0;
        }
        world.rebuild_spatial();
    }

    #[test]
    fn immunity_uses_two_inclusive_draws_and_keeps_the_second_partial_payment() {
        let (mut world, actor_ref) = actor(ColorKind::Green);
        world.session.0.parameters.borrow_mut().lifecycle.immune_system = 2;
        world.session.0.parameters.borrow_mut().lifecycle.mint_energy_consumption = 1.0;
        world.oxygen.set(1.5);
        world.session.0.random.replace(JavaRandom::new(0));
        {
            let mut actor = actor_ref.borrow_mut();
            actor.effects.use_friend_effects = 2;
            actor.effects.antiviral = 1;
            actor.effects.reproduce_late = -1;
            actor.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
            actor.saved_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        }
        finish(&mut world, &actor_ref);
        let actor = actor_ref.borrow();
        assert!(actor.infected_genome.is_none() && actor.saved_genome.is_none());
        assert_eq!(actor.effects.reproduce_late, -1);
        assert_eq!(actor.state.energy, 98.5);
        assert_eq!([world.oxygen.get(), world.carbon_dioxide.get()], [0.0, 1.5]);
        let mut expected = JavaRandom::new(0);
        assert_eq!(expected.bounded(2), 1);
        assert_eq!(expected.bounded(2), 1);
        assert_eq!(world.random_snapshot(), expected);
    }

    #[test]
    fn friend_effect_mode_gates_immunity_and_the_healing_request() {
        let (mut world, actor_ref) = actor(ColorKind::Green);
        {
            let mut actor = actor_ref.borrow_mut();
            actor.effects.use_friend_effects = 1;
            actor.effects.antiviral = 1;
            actor.effects.healing = 1;
            actor.effects.is_injured = true;
            actor.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        }
        let random = world.random_snapshot();
        finish(&mut world, &actor_ref);
        assert!(actor_ref.borrow().infected_genome.is_some());
        assert!(!actor_ref.borrow().effects.use_extra_effects);
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn lavender_wraps_before_capping_and_reduces_delay_before_the_countdown() {
        let (mut world, actor_ref) = actor(ColorKind::Green);
        world.session.0.parameters.borrow_mut().effects.lavender_shield = i32::MAX;
        {
            let mut actor = actor_ref.borrow_mut();
            actor.effects.use_friend_effects = 2;
            actor.effects.create_lavender = i32::MAX;
            actor.effects.lavender = i32::MAX - 10;
            actor.state.reproduction.countdown = 100;
        }
        finish(&mut world, &actor_ref);
        assert_eq!(actor_ref.borrow().effects.lavender, -12);
        assert_eq!(actor_ref.borrow().state.reproduction.countdown, 9);
    }

    #[test]
    fn extra_effects_can_reassert_after_the_entry_flag_is_consumed() {
        let (mut world, actor_ref) = actor(ColorKind::DarkOlive);
        actor_ref.borrow_mut().effects.use_extra_effects = true;
        world.session.0.random.replace(JavaRandom::new(0));
        finish(&mut world, &actor_ref);
        assert!(actor_ref.borrow().effects.use_extra_effects);
        assert_eq!(
            actor_ref.borrow().body.color_argb(0).unwrap(),
            ColorKind::DarkOlive.argb().unwrap()
        );
        let mut expected = JavaRandom::new(0);
        assert_eq!(expected.bounded(100), 60);
        assert_eq!(world.random_snapshot(), expected);
    }

    #[test]
    fn update_priority_consumes_extra_and_renews_flower_energy_before_maintenance() {
        let (mut world, actor_ref) = actor(ColorKind::Green);
        world.session.0.parameters.borrow_mut().lifecycle.segment_cost_divisor = 10;
        {
            let mut actor = actor_ref.borrow_mut();
            actor.body.mass = 5.0;
            actor.body.update_effects = 2;
            actor.effects.use_extra_effects = true;
            actor.effects.maintenance = 7.0;
            actor.effects.reproduce_late = 17;
            actor.effects.is_infectious = true;
            actor.state.energy = 50.0;
        }
        finish(&mut world, &actor_ref);
        let actor = actor_ref.borrow();
        assert_eq!(actor.effects.maintenance, 0.5);
        assert_eq!(actor.body.update_effects, 0);
        assert!(!actor.effects.use_extra_effects);
        assert_eq!(actor.state.reproduction.energy, 46);
        assert_eq!(actor.state.age, 1);
    }

    #[test]
    fn seasonal_updates_use_the_current_world_frame_zero() {
        for frame in [0, 1] {
            let (mut world, actor_ref) = actor(ColorKind::Summer);
            world.frame.set(frame);
            {
                let mut actor = actor_ref.borrow_mut();
                actor.effects.is_plant = true;
                actor.effects.summer_inactivity = 0;
            }
            finish(&mut world, &actor_ref);
            assert_eq!(
                actor_ref.borrow().body.color_argb(0).unwrap(),
                if frame == 0 {
                    ColorKind::Winter
                } else {
                    ColorKind::Summer
                }
                .argb()
                .unwrap()
            );
        }
    }

    #[test]
    fn lime_crowding_uses_removed_frozen_members_and_their_live_rectangles() {
        let (mut world, actor_ref) = actor(ColorKind::Lime);
        let code = actor_ref.borrow().genome.as_ref().unwrap().borrow().clone();
        world.paste_genome(code, [3000, 3000]).unwrap().unwrap();
        let peer_ref = Rc::clone(&world.organisms.borrow()[1]);
        world.rebuild_spatial();
        let peer_bounds = peer_ref.borrow().state.bounds.get();
        world.organisms.borrow_mut().remove(1);
        {
            let mut peer = peer_ref.borrow_mut();
            peer.state.energy = 0.0;
            peer.state.alive = false;
            peer.state.active = false;
            let mut actor = actor_ref.borrow_mut();
            actor.state.bounds.set(peer_bounds);
            actor.effects.is_lime = true;
            actor.effects.is_plant = true;
            actor.body.set_photo(0, 10.0).unwrap();
        }
        finish(&mut world, &actor_ref);
        assert!(actor_ref.borrow().effects.crowded);
        peer_ref.borrow().state.bounds.set(IntRect {
            x: 5000,
            y: 5000,
            width: 5,
            height: 5,
        });
        finish(&mut world, &actor_ref);
        assert!(!actor_ref.borrow().effects.crowded);
        assert_eq!(world.frame_objects.borrow().len(), 2);
    }

    #[test]
    fn plant_resources_are_obtained_before_active_respiration() {
        let (mut world, actor_ref) = actor(ColorKind::Purple);
        world.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        world.oxygen.set(0.0);
        world.carbon_dioxide.set(1.0);
        world.methane.set(1.0);
        {
            let mut actor = actor_ref.borrow_mut();
            actor.effects.is_plant = true;
            actor.effects.methanotrophy = 1.0;
            actor.effects.photosynthesis = 1.0;
            actor.effects.maintenance = 2.0;
        }
        finish(&mut world, &actor_ref);
        assert_eq!(
            [world.oxygen.get(), world.carbon_dioxide.get(), world.methane.get()],
            [0.0, 2.0, 0.0]
        );
        assert!(actor_ref.borrow().state.alive);
        assert_eq!(actor_ref.borrow().state.energy, 100.0);
    }

    #[test]
    fn positive_methane_guard_skips_zero_photo_and_retains_colony_credit() {
        for methane in [0.0, 1.0] {
            let (mut world, actor_ref) = actor(ColorKind::Green);
            world.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
            world.carbon_dioxide.set(10.0);
            {
                let mut actor = actor_ref.borrow_mut();
                actor.effects.is_plant = true;
                actor.effects.methanotrophy = methane;
                actor.effects.colony_photosynthesis = 10.0;
                actor.state.age = 1;
            }
            finish(&mut world, &actor_ref);
            assert_eq!(
                actor_ref.borrow().effects.colony_photosynthesis,
                if methane > 0.0 { 10.0 } else { 0.0 }
            );
            assert_eq!(world.carbon_dioxide.get(), if methane > 0.0 { 10.0 } else { 0.0 });
        }
    }

    #[test]
    fn leaf_branch_retints_and_uses_its_reduced_photo_before_maintenance() {
        let (mut world, actor_ref) = actor(ColorKind::Leaf);
        world.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        world.carbon_dioxide.set(10.0);
        world.methane.set(4.0);
        {
            let mut actor = actor_ref.borrow_mut();
            actor.effects.is_plant = true;
            actor.effects.photosynthesis = 10.0;
            actor.effects.leaf_photo = 4.0;
            actor.effects.methanotrophy = 4.0;
            actor.state.temporary_color = (ColorKind::Blond.argb()).map(crate::JavaColor::from_argb);
            actor.state.temporary_color_frames = 1;
        }
        finish(&mut world, &actor_ref);
        let actor = actor_ref.borrow();
        assert_eq!(actor.state.energy, 110.0);
        assert_eq!([world.carbon_dioxide.get(), world.methane.get()], [4.0, 0.0]);
        assert_eq!(actor.state.temporary_argb(), ColorKind::DarkGreen.argb());
        assert_eq!(actor.state.temporary_color_frames, 10);
    }

    #[test]
    fn plant_spin_uses_source_direction_and_the_fullgrown_scale() {
        for clockwise in [false, true] {
            let (mut world, actor_ref) = actor(ColorKind::Spring);
            world.session.0.parameters.borrow_mut().physics.max_rotation = 10.0;
            {
                let mut actor = actor_ref.borrow_mut();
                actor.effects.is_plant = true;
                actor.effects.spin = 100;
                actor.effects.clockwise = clockwise;
                actor.body.growth_ratio = 1;
                actor.body.inertia = 1.0;
            }
            finish(&mut world, &actor_ref);
            assert_eq!(
                actor_ref.borrow().state.angular_velocity,
                if clockwise { PI } else { -PI }
            );
        }
    }

    #[test]
    fn dodge_reset_retains_the_source_processed_and_disabled_cases() {
        for (can, processed, dodge, after, marker) in [
            (true, true, true, false, false),
            (true, false, false, true, false),
            (false, true, true, true, true),
        ] {
            let (mut world, actor_ref) = actor(ColorKind::Green);
            {
                let mut actor = actor_ref.borrow_mut();
                actor.effects.can_dodge = can;
                actor.effects.has_dodged = processed;
                actor.effects.dodge = dodge;
            }
            finish(&mut world, &actor_ref);
            let actor = actor_ref.borrow();
            assert_eq!(
                (actor.effects.dodge, actor.effects.has_dodged),
                (after, marker)
            );
        }
    }

    #[test]
    fn countdown_reaching_zero_defers_birth_until_the_next_call() {
        let (mut world, actor_ref) = actor(ColorKind::Green);
        adult(&mut world, &actor_ref, 1);
        actor_ref.borrow_mut().state.reproduction.countdown = 1;
        let random = world.random_snapshot();
        finish(&mut world, &actor_ref);
        assert_eq!(actor_ref.borrow().state.reproduction.countdown, 0);
        assert_eq!(world.organisms.borrow().len(), 1);
        assert_eq!(world.random_snapshot(), random);
        finish(&mut world, &actor_ref);
        assert_eq!(world.organisms.borrow().len(), 2);
        assert_eq!(actor_ref.borrow().state.total_children, 1);
        assert_eq!(world.frame_objects.borrow().len(), 1);
    }

    #[test]
    fn failed_early_threshold_does_not_fall_through_to_an_eligible_adult_birth() {
        let (mut world, actor_ref) = actor(ColorKind::Green);
        adult(&mut world, &actor_ref, 1);
        actor_ref.borrow_mut().state.reproduction.early_energy = 100;
        let random = world.random_snapshot();
        finish(&mut world, &actor_ref);
        assert_eq!(world.organisms.borrow().len(), 1);
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn early_callers_keep_classification_growth_and_saved_child_identity() {
        for (class, ratio, spore, born) in [
            (0, 1, 0, true),
            (0, 2, 0, false),
            (1, 1, 0, true),
            (1, 2, 0, false),
            (2, 2, -1, false),
            (2, 2, 0, true),
            (2, 1, -1, true),
            (3, 2, 0, true),
            (4, 2, 0, true),
            (4, 16, 0, false),
        ] {
            let (mut world, actor_ref) = actor(ColorKind::Green);
            adult(&mut world, &actor_ref, ratio);
            {
                let mut actor = actor_ref.borrow_mut();
                actor.state.reproduction.early_energy = 1;
                actor.effects.spore_version = spore;
                actor.effects.is_plant = class == 0;
                actor.effects.is_consumer = class == 1;
                actor.effects.is_fungus = class == 2;
                actor.effects.transfers_energy = class == 3;
            }
            finish(&mut world, &actor_ref);
            assert_eq!(
                world.organisms.borrow().len(),
                if born { 2 } else { 1 },
                "class={class} ratio={ratio} spore={spore}"
            );
            if born && class == 4 {
                assert!(Rc::ptr_eq(
                    actor_ref.borrow().saved_genome.as_ref().unwrap(), world.organisms.borrow()[1].borrow().genome.as_ref().unwrap()));
            }
        }
    }

    #[test]
    fn adult_inhibition_keeps_failed_payment_fallback_and_uninfected_bypass() {
        for (infected, oxygen, born, lavender) in [
            (true, 0.5, true, 500),
            (true, 100.0, false, 315),
            (false, 100.0, true, 500),
        ] {
            let (mut world, actor_ref) = actor(ColorKind::Green);
            adult(&mut world, &actor_ref, 1);
            world.session.0.parameters.borrow_mut().lifecycle.mint_energy_consumption = 10.0;
            world.oxygen.set(oxygen);
            {
                let mut actor = actor_ref.borrow_mut();
                actor.state.energy = 50.0;
                actor.effects.antiviral = 1;
                actor.effects.lavender = 500;
                if infected {
                    actor.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
                }
            }
            finish(&mut world, &actor_ref);
            assert_eq!(world.organisms.borrow().len(), if born { 2 } else { 1 });
            assert_eq!(actor_ref.borrow().effects.lavender, lavender);
        }
    }

    #[test]
    fn inactive_corpse_uses_decay_without_age_dodge_or_countdown_changes() {
        let (mut world, actor_ref) = actor(ColorKind::Brown);
        world.session.0.parameters.borrow_mut().lifecycle.decay_energy = 0.5;
        {
            let mut actor = actor_ref.borrow_mut();
            actor.state.active = false;
            actor.state.alive = false;
            actor.effects.can_dodge = true;
            actor.effects.has_dodged = true;
            actor.effects.dodge = true;
        }
        finish(&mut world, &actor_ref);
        let actor = actor_ref.borrow();
        assert_eq!(actor.state.age, 0);
        assert_eq!(actor.state.reproduction.countdown, 10);
        assert!(actor.effects.has_dodged && actor.effects.dodge);
        assert_eq!(
            [world.oxygen.get(), world.carbon_dioxide.get(), world.methane.get()],
            [999.5, 0.0, 0.5]
        );
    }

    #[test]
    fn inactive_hatching_does_not_reenter_active_maintenance_in_the_same_call() {
        let (mut world, actor_ref) = actor(ColorKind::Green);
        world.session.0.parameters.borrow_mut().spore_energy_consumption = 0.0;
        {
            let mut actor = actor_ref.borrow_mut();
            actor.state.active = false;
            actor.effects.spore_version = 3;
            actor.state.reproduction.countdown = 0;
        }
        finish(&mut world, &actor_ref);
        assert!(actor_ref.borrow().state.active);
        assert_eq!(actor_ref.borrow().state.age, 1);
    }
    #[test]
    fn ordinary_plant_energy_propagates_methane_owner_failure_before_photo() {
        let (world, reference) = actor(ColorKind::Purple);
        let mut organism = reference.borrow_mut();
        organism.state.world = Default::default();
        organism.state.age = 8;
        organism.effects.leaf_photo = 0.0;
        organism.effects.methanotrophy = 1.0;
        organism.effects.photosynthesis = 2.0;
        organism.effects.colony_photosynthesis = 3.0;
        let energy = organism.state.energy;
        assert_eq!(world.ordinary_plant_frame_energy(&mut organism), Err(WorldError::MissingWorld));
        assert_eq!(organism.state.energy, energy);
        assert_eq!(organism.effects.colony_photosynthesis, 3.0);
    }

    #[test]
    fn seasonal_frame_uses_associated_clock_with_held_actor_and_collections() {
        let (caller, actor_ref) = actor(ColorKind::Summer);
        let owner = World::empty_in_session(caller.config.get(), caller.session());
        let mut organism = actor_ref.borrow_mut();
        organism.state.world = owner.association();
        organism.effects.summer_inactivity = 0;
        organism.effects.lower_symmetric = 0;
        organism.effects.colony_photosynthesis = 0.0;
        organism.body.growth_ratio = 1;
        organism.body.set_photo(0, 0.0).unwrap();
        let random = caller.random_snapshot();
        let _caller_members = caller.organisms.borrow_mut();
        let _owner_members = owner.organisms.borrow_mut();
        for (caller_frame, owner_frame, expected) in [
            (0, 1, ColorKind::Summer),
            (1, 0, ColorKind::Winter),
        ] {
            caller.frame.set(caller_frame);
            owner.frame.set(owner_frame);
            organism.body.set_color(0, ColorKind::Summer.color_ref()).unwrap();
            organism.body.update_effects = 0;
            organism.effects.use_extra_effects = false;
            caller.active_plant_effects(&actor_ref, &mut organism).unwrap();
            assert_eq!(organism.body.color_argb(0).unwrap(), expected.argb().unwrap());
            assert_eq!(organism.body.update_effects, 0);
            assert_eq!((caller.frame(), owner.frame()), (caller_frame, owner_frame));
        }
        assert_eq!((caller.time(), owner.time()), (0, 0));
        assert_eq!(caller.random_snapshot(), random);
    }

    #[test]
    fn seasonal_frame_resolves_null_and_expired_owners_only_after_inactivity_guard() {
        for expired in [false, true] {
            for inactivity in [-2, -1, 0] {
                let (caller, actor_ref) = actor(ColorKind::White);
                let association = if expired {
                    let owner = World::empty_in_session(caller.config.get(), caller.session());
                    owner.association()
                } else {
                    Default::default()
                };
                let mut organism = actor_ref.borrow_mut();
                organism.state.world = association;
                organism.effects.summer_inactivity = inactivity;
                organism.effects.use_extra_effects = true;
                organism.effects.forest_photo = 17.0;
                organism.effects.leaf_photo = 19.0;
                organism.effects.methanotrophy = 23.0;
                organism.body.growth_ratio = 1;
                organism.body.update_effects = 1;
                let random = caller.random_snapshot();
                let result = caller.active_plant_effects(&actor_ref, &mut organism);
                if inactivity < 0 {
                    assert_eq!(result, Ok(()));
                    assert_eq!(organism.body.update_effects, 0);
                    assert!(!organism.effects.use_extra_effects);
                    assert_eq!((organism.effects.forest_photo, organism.effects.leaf_photo,
                        organism.effects.methanotrophy), (0.0, 0.0, 0.0));
                } else {
                    assert_eq!(result, Err(if expired {
                        WorldError::ExpiredWorld
                    } else {
                        WorldError::MissingWorld
                    }));
                    assert_eq!(organism.body.update_effects, 1);
                    assert!(organism.effects.use_extra_effects);
                    assert_eq!((organism.effects.forest_photo, organism.effects.leaf_photo,
                        organism.effects.methanotrophy), (17.0, 19.0, 23.0));
                }
                assert_eq!(caller.random_snapshot(), random);
            }
        }
    }

    #[test]
    fn lime_crowding_uses_owner_tree_frozen_identities_and_live_rectangles() {
        for owner_hit in [false, true] {
            let (caller, reference) = actor(ColorKind::Lime);
            let (_peer_world, peer_ref) = actor(ColorKind::White);
            let owner = World::empty_in_session(caller.config.get(), caller.session());
            let bounds = IntRect { x: 100, y: 100, width: 10, height: 10 };
            reference.borrow().state.bounds.set(bounds);
            {
                let mut peer = peer_ref.borrow_mut();
                peer.id = reference.borrow().id;
                peer.state.bounds.set(bounds);
                peer.state.alive = false;
                peer.state.active = false;
            }
            for (world, hit) in [(&caller, !owner_hit), (&owner, owner_hit)] {
                world.frame_objects.replace(vec![Rc::clone(&reference), Rc::clone(&peer_ref)]);
                let mut entries = vec![(0, bounds)];
                if hit { entries.push((1, bounds)); }
                world.spatial.replace(crate::spatial::SpatialTree::build_actor_entries(entries, |token| world.spatial_actor(token)));
            }
            owner.organisms.borrow_mut().push(Rc::clone(&peer_ref));
            let removed = owner.organisms.borrow_mut().pop().unwrap();
            assert!(Rc::ptr_eq(&removed, &peer_ref));
            let random = caller.random_snapshot();
            let mut organism = reference.borrow_mut();
            organism.state.world = owner.association();
            organism.effects.is_lime = true;
            organism.effects.is_plant = true;
            organism.effects.summer_inactivity = -1;
            organism.effects.lower_symmetric = 0;
            organism.effects.colony_photosynthesis = 0.0;
            organism.body.growth_ratio = 1;
            organism.body.set_photo(0, 0.0).unwrap();
            let _caller_members = caller.organisms.borrow_mut();
            let _owner_members = owner.organisms.borrow_mut();
            for receiver in [&caller, &owner] {
                for initial in [false, true] {
                    organism.effects.crowded = initial;
                    organism.effects.forest_photo = 17.0;
                    organism.effects.use_extra_effects = false;
                    organism.body.update_effects = 0;
                    receiver.active_plant_effects(&reference, &mut organism).unwrap();
                    assert_eq!(organism.effects.crowded, owner_hit);
                    assert_eq!(organism.effects.forest_photo,
                        if initial == owner_hit { 17.0 } else { 0.0 });
                    assert_eq!(organism.body.update_effects, 0);
                    assert_eq!(caller.random_snapshot(), random);
                }
            }
            if owner_hit {
                peer_ref.borrow().state.bounds.set(IntRect {
                    x: 5000, y: 5000, width: 10, height: 10,
                });
                caller.active_plant_effects(&reference, &mut organism).unwrap();
                assert!(!organism.effects.crowded);
                assert_eq!(owner.frame_objects.borrow().len(), 2);
                assert_eq!(caller.random_snapshot(), random);
            }
        }
    }

    #[test]
    fn lime_query_resolves_missing_and_expired_owners_only_at_the_lime_gate() {
        for expired in [false, true] {
            for lime in [false, true] {
                let (caller, reference) = actor(ColorKind::White);
                let association = if expired {
                    let owner = World::empty_in_session(caller.config.get(), caller.session());
                    owner.association()
                } else {
                    Default::default()
                };
                let mut organism = reference.borrow_mut();
                organism.state.world = association;
                organism.effects.is_lime = lime;
                organism.effects.is_plant = true;
                organism.effects.summer_inactivity = -1;
                organism.effects.crowded = true;
                organism.effects.use_extra_effects = true;
                organism.effects.forest_photo = 17.0;
                organism.effects.leaf_photo = 19.0;
                organism.effects.methanotrophy = 23.0;
                organism.body.update_effects = 1;
                organism.body.growth_ratio = 1;
                let energy = organism.state.energy;
                let random = caller.random_snapshot();
                let result = caller.active_plant_effects(&reference, &mut organism);
                if lime {
                    assert_eq!(result, Err(if expired {
                        WorldError::ExpiredWorld
                    } else {
                        WorldError::MissingWorld
                    }));
                    assert!(organism.effects.crowded && organism.effects.use_extra_effects);
                    assert_eq!(organism.body.update_effects, 1);
                    assert_eq!((organism.effects.forest_photo, organism.effects.leaf_photo,
                        organism.effects.methanotrophy), (17.0, 19.0, 23.0));
                } else {
                    assert_eq!(result, Ok(()));
                    assert!(organism.effects.crowded);
                    assert!(!organism.effects.use_extra_effects);
                    assert_eq!(organism.body.update_effects, 0);
                    assert_eq!((organism.effects.forest_photo, organism.effects.leaf_photo,
                        organism.effects.methanotrophy), (0.0, 0.0, 0.0));
                }
                assert_eq!(organism.state.energy, energy);
                assert_eq!(caller.random_snapshot(), random);
            }
        }
    }

}

impl World {
    fn actor_immunity_and_lavender(&self, actor: &mut Organism) -> Result<(), crate::WorldError> {
        Ok({
        if actor.effects.healing > 0 && actor.effects.is_injured {
            actor.effects.use_extra_effects = true;
        }
        if actor.effects.antiviral > 0 {
            if actor.infected_genome.is_some()
                && self.session.0.random.borrow_mut().bounded(self.session.0.parameters.borrow().lifecycle.immune_system)
                    <= actor.effects.antiviral
                && actor.use_energy(
                    self.session.0.parameters.borrow().lifecycle.mint_energy_consumption)?
            {
                actor.infected_genome = None;
                actor.saved_genome = None;
                actor.set_display_color(ColorKind::Mint.color_ref());
            }
            if actor.effects.reproduce_late < 0
                && self.session.0.random.borrow_mut().bounded(self.session.0.parameters.borrow().lifecycle.immune_system)
                    <= actor.effects.antiviral
                && actor.use_energy(
                    self.session.0.parameters.borrow().lifecycle.mint_energy_consumption)?
            {
                actor.effects.reproduce_late = 0;
                actor.set_display_color(ColorKind::Mint.color_ref());
            }
        }
        let production = actor.effects.create_lavender;
        if production > 0 {
            if actor.effects.lavender < self.session.0.parameters.borrow().effects.lavender_shield {
                actor.effects.lavender = actor.effects.lavender.wrapping_add(production);
                if actor.effects.lavender >= self.session.0.parameters.borrow().effects.lavender_shield {
                    actor.effects.lavender = self.session.0.parameters.borrow().effects.lavender_shield;
                }
            }
            if actor.state.reproduction.countdown > actor.state.reproduction.maximum_countdown {
                actor.state.reproduction.countdown = actor
                    .state
                    .reproduction
                    .countdown
                    .wrapping_sub(production / 20);
                if actor.state.reproduction.countdown < actor.state.reproduction.maximum_countdown {
                    actor.state.reproduction.countdown = actor.state.reproduction.maximum_countdown;
                }
            }
        }
    
        })
    }

    fn ordinary_plant_frame_energy(&self, actor: &mut Organism) -> Result<(), WorldError> {
        if actor.effects.leaf_photo > 0.0 && actor.state.temporary_color_frames > 0 {
            self.leaf_extra_effects(actor)?;
        } else if actor.effects.methanotrophy > 0.0 {
            self.frame_methanotrophy(actor)?;
            if actor.effects.photosynthesis > 0.0 {
                self.frame_photo(actor, actor.effects.photosynthesis, 0.0)?;
            }
        } else {
            self.frame_photo(actor, actor.effects.photosynthesis, 0.0)?;
        }
        Ok(())
    }

    fn active_plant_effects(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
    ) -> Result<(), WorldError> {
        if actor.effects.is_lime {
            let crowded = actor.state.world.resolve()?.fast_check_hit(actor, actor_ref, actor, None)?;
            if actor.effects.crowded != crowded {
                actor.effects.crowded = crowded;
                actor.body.update_effects = 2;
            }
        }
        if actor.effects.summer_inactivity > -1 && actor.state.world.resolve()?.frame.get() == 0 {
            actor.body.update_effects = 2;
        }
        if actor.body.update_effects > 0 {
            actor.effects.use_extra_effects = false;
            self.plants_update_effects(actor)?;
            actor.body.update_effects = 0;
            flower_reproduction_energy(actor)?;
        } else if actor.effects.use_extra_effects {
            actor.effects.use_extra_effects = false;
            if actor.effects.jade_factor > 0 {
                self.jade_extra_effects(actor)?;
            } else {
                if actor.effects.healing > 0 && actor.effects.is_injured {
                    self.segments_healing_effects(actor)?;
                } else if actor.effects.use_frame_movement {
                    self.segments_extra_effects(actor)?;
                } else {
                    self.segments_extra_plants_effects(actor)?;
                }
                self.ordinary_plant_frame_energy(actor)?;
            }
        } else {
            if actor.effects.use_frame_movement {
                self.segments_frame_effects(actor)?;
            }
            self.ordinary_plant_frame_energy(actor)?;
        }
        if actor.effects.spin > 0 {
            let impulse = (f64::from(actor.effects.spin)
                * SPRING_SCALES[crate::geometry::growth_index(actor.body.growth_ratio)?])
                * PI
                / actor.body.inertia;
            let rotated = if actor.effects.clockwise {
                actor.state.angular_velocity + impulse
            } else {
                actor.state.angular_velocity - impulse
            };
            actor.state.angular_velocity = between(
                rotated,
                -self.session.0.parameters.borrow().physics.max_rotation,
                self.session.0.parameters.borrow().physics.max_rotation,
            );
        }
        Ok(())
    }

    fn active_nonplant_effects(&self, actor: &mut Organism) -> Result<(), WorldError> {
        if actor.body.update_effects > 0 {
            actor.effects.use_extra_effects = false;
            self.segments_update_effects(actor)?;
            actor.body.update_effects = 0;
            flower_reproduction_energy(actor)?;
        } else if actor.effects.use_extra_effects {
            actor.effects.use_extra_effects = false;
            if actor.effects.healing > 0 && actor.effects.is_injured {
                self.segments_healing_effects(actor)?;
            } else {
                self.segments_extra_effects(actor)?;
            }
        } else if actor.effects.use_frame_movement {
            self.segments_frame_effects(actor)?;
        }
        Ok(())
    }

    fn adult_actor_reproduction(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        uninfected_bypass: bool,
    ) -> Result<(), WorldError> {
        if actor.state.energy > f64::from(actor.state.reproduction.energy)
            && actor.body.growth_ratio == 1
        {
            if actor.effects.antiviral == 0
                || actor.effects.lavender == 0
                || (uninfected_bypass && actor.infected_genome.is_none())
            {
                self.reproduce_from(actor_ref, actor, None)?;
            } else if actor.use_energy(
                self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 10.0)? {
                if actor.effects.lavender > 0 {
                    self.lavender_inhibition(actor);
                }
                if actor.effects.lavender <= 0 {
                    self.reproduce_from(actor_ref, actor, None)?;
                }
            } else {
                self.reproduce_from(actor_ref, actor, None)?;
            }
        }
        Ok(())
    }

    fn actor_reproduction(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
    ) -> Result<(), WorldError> {
        if actor.state.reproduction.countdown > 0 {
            actor.state.reproduction.countdown = actor.state.reproduction.countdown.wrapping_sub(1);
        } else if actor.state.reproduction.early_energy > 0 {
            if actor.state.energy > f64::from(actor.state.reproduction.early_energy) {
                if actor.infected_genome.is_none() {
                    if actor.effects.is_plant {
                        if actor.body.growth_ratio == 1 {
                            self.reproduce_early_plant_from(actor_ref, actor, None)?;
                        }
                    } else if actor.effects.is_consumer {
                        if actor.body.growth_ratio == 1 {
                            self.reproduce_early_from(actor_ref, actor, None)?;
                        }
                    } else if actor.effects.is_fungus {
                        if (actor.effects.spore_version < 0 && actor.body.growth_ratio == 1)
                            || (actor.effects.spore_version >= 0 && actor.body.growth_ratio < 16)
                        {
                            self.reproduce_early_from(actor_ref, actor, None)?;
                        }
                    } else if actor.effects.transfers_energy {
                        if actor.body.growth_ratio < 16 {
                            self.reproduce_early_from(actor_ref, actor, None)?;
                        }
                    } else if actor.body.growth_ratio < 16 {
                        self.reproduce_early_virus_from(actor_ref, actor, None)?;
                    }
                } else {
                    self.adult_actor_reproduction(actor_ref, actor, false)?;
                }
            }
        } else {
            self.adult_actor_reproduction(actor_ref, actor, true)?;
        }
        Ok(())
    }

    /// Complete the source active/inactive branch after motion and contacts, before World removal
    pub(super) fn finish_actor_frame(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
    ) -> Result<(), WorldError> {
        if actor.state.active {
            if actor.effects.use_friend_effects == 2 {
                self.actor_immunity_and_lavender(actor)?;
            }
            if actor.effects.is_plant {
                self.active_plant_effects(actor_ref, actor)?;
            } else {
                self.active_nonplant_effects(actor)?;
            }
            if actor.effects.can_dodge {
                if actor.effects.has_dodged {
                    actor.effects.dodge = false;
                    actor.effects.has_dodged = false;
                } else if !actor.effects.dodge {
                    actor.effects.dodge = true;
                }
            }
            self.actor_reproduction(actor_ref, actor)?;
            self.breath_from(actor_ref, actor, None)?;
        } else {
            self.decay_from(actor_ref, actor, None)?;
        }
        Ok(())
    }
}
