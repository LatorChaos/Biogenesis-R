// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use super::{Organism, OrganismRef, World, WorldError, bounds_overlap};
use crate::TOLERANCE;
use std::rc::Rc;

impl World {
    pub(super) fn check_actor_contact_from(
        &self, actor_ref: &OrganismRef, actor: &mut Organism,
    ) -> Result<Option<OrganismRef>, WorldError> {
        actor.state.world.resolve()?.check_contact_from(actor_ref, actor)
    }

    /// Query frozen envelopes once, then inspect live bodies in the original tree order
    pub(super) fn check_contact_from(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
    ) -> Result<Option<OrganismRef>, WorldError> {
        let candidates = self.spatial.borrow().query_slots(actor.state.bounds.get())?;
        for (index, item) in candidates.into_iter().enumerate() {
            let peer_ref = Self::query_actor(item,index)?;
            if Rc::ptr_eq(actor_ref, &peer_ref) {
                continue;
            }
            let mut peer = peer_ref.borrow_mut();
            if bounds_overlap(actor.state.bounds.get(), peer.state.bounds.get())
                && peer.state.energy >= TOLERANCE
                && actor.state.energy >= TOLERANCE
                && self.contact_from(actor_ref, actor, &peer_ref, &mut peer)?
            {
                return Ok(Some(Rc::clone(actor_ref)));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorKind, EffectsState, Gene, Genome, GenomeProperties, IntRect, WorldConfig};

    fn setup() -> (World, OrganismRef, OrganismRef) {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..Default::default()
        })
        .unwrap();
        let code = Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), -1, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..Default::default()
            },
        );
        for center in [[1000, 1000], [3500, 3500]] {
            world.paste_genome(code.clone(), center).unwrap().unwrap();
        }
        let actor_ref = Rc::clone(&world.organisms.borrow()[0]);
        let peer_ref = Rc::clone(&world.organisms.borrow()[1]);
        for (reference, start, end) in [
            (&actor_ref, [0, 0], [10, 10]),
            (&peer_ref, [0, 10], [10, 0]),
        ] {
            let mut actor = reference.borrow_mut();
            actor.state.integer_center = [0, 0];
            actor.state.center = [0.0, 0.0];
            actor.state.bounds.set(IntRect {
                x: 0,
                y: 0,
                width: 20,
                height: 20,
            });
            { let value = start; actor.body.set_rotated_start(0, 0, value[0]).unwrap(); actor.body.set_rotated_start(0, 1, value[1]).unwrap(); };
            { let value = end; actor.body.set_rotated_end(0, 0, value[0]).unwrap(); actor.body.set_rotated_end(0, 1, value[1]).unwrap(); };
            actor.body.set_physical_length(0, 2.0).unwrap();
            actor.state.energy = 1.0;
            actor.effects = EffectsState {

                ..Default::default()
            };
        }
        world.rebuild_spatial();
        (world, actor_ref, peer_ref)
    }

    #[test]
    fn query_excludes_only_self_identity_and_retains_removed_dead_bodies() {
        let (world, actor_ref, peer_ref) = setup();
        let mut actor = actor_ref.borrow_mut();
        actor.id = 0;
        {
            let mut peer = peer_ref.borrow_mut();
            peer.id = 0;
            world.die(&mut actor, None).unwrap();
            world.die(&mut peer, None).unwrap();
        }
        world.organisms.borrow_mut().remove(1);
        assert!(!actor.state.alive && !peer_ref.borrow().state.alive);
        let result = world
            .check_contact_from(&actor_ref, &mut actor)
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&result, &actor_ref));
        assert!(!Rc::ptr_eq(&result, &peer_ref));
    }

    #[test]
    fn query_preserves_both_inclusive_energy_gates_and_nan_rejection() {
        let (world, actor_ref, peer_ref) = setup();
        let mut actor = actor_ref.borrow_mut();
        actor.state.energy = f64::from_bits(TOLERANCE.to_bits() - 1);
        assert!(
            world
                .check_contact_from(&actor_ref, &mut actor)
                .unwrap()
                .is_none()
        );
        actor.state.energy = TOLERANCE;
        peer_ref.borrow_mut().state.energy = f64::NAN;
        assert!(
            world
                .check_contact_from(&actor_ref, &mut actor)
                .unwrap()
                .is_none()
        );
        peer_ref.borrow_mut().state.energy = TOLERANCE;
        assert!(
            world
                .check_contact_from(&actor_ref, &mut actor)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn query_keeps_frozen_envelopes_but_rechecks_live_rectangles() {
        let (world, actor_ref, peer_ref) = setup();
        let mut actor = actor_ref.borrow_mut();
        let original = actor.state.bounds.get();
        let distant = IntRect {
            x: 1000,
            y: 1000,
            ..original
        };
        peer_ref.borrow().state.bounds.set(distant);
        assert!(
            world
                .check_contact_from(&actor_ref, &mut actor)
                .unwrap()
                .is_none()
        );
        peer_ref.borrow().state.bounds.set(original);
        assert!(
            world
                .check_contact_from(&actor_ref, &mut actor)
                .unwrap()
                .is_some()
        );
        actor.state.bounds.set(distant);
        peer_ref.borrow().state.bounds.set(distant);
        assert!(
            world
                .check_contact_from(&actor_ref, &mut actor)
                .unwrap()
                .is_none()
        );
    }

    fn frame_world() -> World {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..Default::default()
        })
        .unwrap();
        world.session.0.parameters.borrow_mut().genetics.min_mutation_rate = 0;
        world.session.0.parameters.borrow_mut().genetics.max_mutation_rate = 0;
        world.session.0.parameters.borrow_mut().genetics.meta_mutation_rate = 0;
        let code = Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), -1, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..Default::default()
            },
        );
        for center in [[1000, 1000], [3500, 3500]] {
            world.paste_genome(code.clone(), center).unwrap().unwrap();
            let actor_ref = Rc::clone(world.organisms.borrow().last().unwrap());
            let mut actor = actor_ref.borrow_mut();
            actor.effects = EffectsState {

                ..Default::default()
            };
            actor.body.update_effects = 0;
            actor.state.energy = 100.0;
            actor.state.reproduction.energy = 10_000;
            actor.state.reproduction.countdown = 10;
            actor.state.reproduction.maximum_age = 1000;
        }
        world.oxygen.set(1000.0);
        world
    }

    #[test]
    fn frame_removal_preserves_frozen_objects_and_moves_the_shifted_member_once() {
        let world = frame_world();
        let removed = Rc::clone(&world.organisms.borrow()[0]);
        let shifted = Rc::clone(&world.organisms.borrow()[1]);
        world.die(&mut removed.borrow_mut(), None).unwrap();
        removed.borrow_mut().state.energy = 0.0;
        assert!(world.single_step().unwrap());
        assert_eq!(world.organisms.borrow().len(), 1);
        assert!(Rc::ptr_eq(&world.organisms.borrow()[0], &shifted));
        assert_eq!(shifted.borrow().state.age, 1);
        assert_eq!(shifted.borrow().state.reproduction.countdown, 9);
        assert_eq!(world.population.get(), 1);
        assert_eq!(world.frame_objects.borrow().len(), 2);
        assert!(Rc::ptr_eq(&world.frame_objects.borrow()[0], &removed));
        assert!(world.single_step().unwrap());
        assert_eq!(shifted.borrow().state.age, 2);
        assert_eq!(world.frame_objects.borrow().len(), 1);
        assert!(Rc::ptr_eq(&world.frame_objects.borrow()[0], &shifted));
    }

    #[test]
    fn synchronous_newborns_are_neither_moved_nor_indexed_until_the_next_frame() {
        let world = frame_world();
        let parent = Rc::clone(&world.organisms.borrow()[0]);
        {
            let mut actor = parent.borrow_mut();
            actor.body.growth_ratio = 1;
            actor.rebuild_body().unwrap();
            crate::movement::calculate_bounds(&mut actor, true).unwrap();
            actor.state.reproduction.energy = 43;
            actor.state.reproduction.countdown = 0;
        }
        assert!(world.single_step().unwrap());
        assert_eq!(world.organisms.borrow().len(), 3);
        assert_eq!(parent.borrow().state.total_children, 1);
        let newborn = Rc::clone(&world.organisms.borrow()[2]);
        assert_eq!(newborn.borrow().state.age, 0);
        assert_eq!(world.frame_objects.borrow().len(), 2);
        assert!(
            world.frame_objects.borrow().iter()
                .all(|actor| !Rc::ptr_eq(actor, &newborn))
        );
        assert!(world.single_step().unwrap());
        assert_eq!(newborn.borrow().state.age, 1);
        assert!(
            world.frame_objects.borrow().iter()
                .any(|actor| Rc::ptr_eq(actor, &newborn))
        );
    }

    #[test]
    fn genesis_resets_live_world_and_statistics_but_retains_rng_and_frozen_objects() {
        let world = frame_world();
        world.rebuild_spatial();
        let old_actor = Rc::clone(&world.organisms.borrow()[0]);
        let random = world.random_snapshot();
        world.frame.set(19);
        world.tick.set(19);
        world.oxygen.set(3.0);
        world.carbon_dioxide.set(5.0);
        world.methane.set(7.0);
        world.resume();
        world.genesis().unwrap();
        assert!(world.organisms.borrow().is_empty());
        assert_eq!(
            (world.population.get(), world.next_id.get(), world.frame.get(), world.tick.get()),
            (0, 0, 0, 0)
        );
        assert_eq!(world.statistics_ref().borrow().get_created_organisms(), 0);
        assert_eq!(world.random_snapshot(), random);
        assert!(!world.paused());
        assert_eq!(
            [world.oxygen.get(), world.carbon_dioxide.get(), world.methane.get()],
            [0.0, 500_000.0, 0.0]
        );
        assert_eq!(world.frame_objects.borrow().len(), 2);
        assert!(Rc::ptr_eq(&world.frame_objects.borrow()[0], &old_actor));
    }
}
