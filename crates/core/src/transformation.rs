// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Transformation construction and placement from Organism.java:2534-2995,3467-3510,4160-4170

use crate::organism::primary_genome;

use super::*;
use crate::organism::initialize_life;

impl World {
    pub(super) fn transform_from(
        &self,
        victim_ref: &OrganismRef,
        victim: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        if let Some(child) = self.construct_transformation_from(victim_ref, victim, true, peer)? {
            victim.state.world.resolve()?.add_organism(child, Some(victim))?;
            victim.infected_genome = None;
            victim.saved_genome = None;
        }
        Ok(())
    }

    pub(super) fn construct_transformation_from(
        &self,
        victim_ref: &OrganismRef,
        victim: &mut Organism,
        first: bool,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<Option<OrganismRef>, WorldError> {
        let mut initial = self.initial_child_state(victim)?;
        let inherited = victim.infected_genome.as_ref().cloned().ok_or_else(|| {
            WorldError::InvalidParameters("Transformation requires an infected genotype".to_owned())
        })?;
        let genome = Genome::inherit_with(
            &inherited.borrow(),
            &mut self.session.0.random.borrow_mut(),
            &self.session.0.parameters.borrow().genetics,
        )
        .map_err(WorldError::from)?;
        initial.parent_id = victim.id as i32;
        initial.energy = victim.state.energy;
        if !first {
            // The rejected candidate's uninitialized metadata remains outside the valid world model
            return Ok(None);
        }
        let mut child = Organism::unplaced(Rc::new(RefCell::new(genome)), initial)
            .map_err(WorldError::Geometry)?;
        if !self.place_transformation(victim_ref, victim, peer, &mut child)? {
            return Ok(None);
        }
        initialize_life(
            &primary_genome(&child.genome)?.borrow(),
            child.body.segment_count,
            &mut child.state.reproduction,
            &self.session.0.parameters.borrow().effects,
        )?;
        child.effects.apply(
            Some(&primary_genome(&child.genome)?.borrow()),
            &mut child.body,
            child.state.age,
            &mut child.state.reproduction,
            &self.session.0.parameters.borrow().effects,
        )?;
        crate::birth_spore::initialize_transformation_spore(
            &mut child,
            &mut self.session.0.random.borrow_mut(),
            &self.session.0.parameters.borrow().effects,
            self.session.0.parameters.borrow().spore_energy_consumption,
        )?;
        primary_genome(&child.genome)?.borrow_mut()
            .update_lineage(
                &inherited.borrow(),
                child.id as i32,
                self.session.0.parameters.borrow().genetics.clade_complexity,
            )
            .map_err(WorldError::from)?;
        Ok(Some(Rc::new(RefCell::new(child))))
    }

    fn transformation_center_hit(
        &self,
        child: &Organism,
        victim_ref: &OrganismRef,
        victim: &Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<bool, WorldError> {
        let candidates = self.spatial.borrow().query_slots(child.state.bounds.get())?;
        for (index,item) in candidates.into_iter().enumerate() {
            let reference = Self::query_actor(item,index)?;
            if self.with_frame_candidate(&reference,victim_ref,victim,peer,|candidate| {
                !std::ptr::eq(child,candidate)
                    && child.id != i64::from(candidate.state.parent_id)
                    && i64::from(child.state.parent_id) != candidate.id
                    && bounds_overlap(child.state.bounds.get(),candidate.state.bounds.get())
            }) { return Ok(true); }
        }
        Ok(false)
    }

    fn place_transformation(
        &self,
        victim_ref: &OrganismRef,
        victim: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
        child: &mut Organism,
    ) -> Result<bool, WorldError> {
        child.state.center = victim.state.center;
        child.state.integer_center = child.state.center.map(|value| value as i32);
        crate::movement::calculate_bounds(child, true).map_err(WorldError::Geometry)?;
        if self.is_inside_world(child)?
            && !child.state.world.resolve()?.transformation_center_hit(child, victim_ref, victim, peer)?
        {
            child.state.velocity = victim.state.velocity;
            child.id = child.state.world.resolve()?.allocate_id();
            victim.state.energy -= child.state.energy;
            return Ok(true);
        }
        let mut direction = self.session.0.random.borrow_mut().bounded(8) as usize;
        for _ in 0..8 {
            for axis in 0..2 {
                let parent_size = if axis == 0 {
                    victim.state.bounds.get().width
                } else {
                    victim.state.bounds.get().height
                };
                let child_size = if axis == 0 {
                    child.state.bounds.get().width
                } else {
                    child.state.bounds.get().height
                };
                let offset = (parent_size / 2)
                    .wrapping_add(child_size / 2)
                    .wrapping_add(1)
                    .wrapping_mul(SIDES[direction][axis]);
                child.state.center[axis] = victim.state.center[axis] + f64::from(offset);
                child.state.integer_center[axis] = child.state.center[axis] as i32;
            }
            crate::movement::calculate_bounds(child, true).map_err(WorldError::Geometry)?;
            if self.is_inside_world(child)? && !child.state.world.resolve()?.fast_check_hit(child, victim_ref, victim, peer)?
            {
                child.state.velocity = victim.state.velocity;
                child.id = child.state.world.resolve()?.allocate_id();
                victim.state.energy -= child.state.energy;
                return Ok(true);
            }
            direction = (direction + 1) % 8;
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorKind, Gene, GenomeModifiers, GenomeProperties};

    fn genome(spore: i32, spore_gene: bool) -> Genome {
        let gene = Gene::new(
            if spore_gene { 120.0 } else { 12.0 },
            0.25,
            if spore_gene {
                ColorKind::Spore.argb()
            } else {
                ColorKind::Green.argb()
            },
            -1,
            [0; 36],
        )
        .shared();
        Genome::from_parts(
            &[gene],
            GenomeProperties {
                symmetry: 1,
                modifiers: GenomeModifiers {
                    cream: 1,
                    fallow: 1,
                    spore,
                    adapt_spore: 1,
                    disperse_children: true,
                    ..GenomeModifiers::default()
                },
                ..GenomeProperties::default()
            },
        )
    }

    fn world_with_blocker(blocker_parent: i32) -> (World, OrganismRef, OrganismRef) {
        let mut parameters = WorldParameters::default();
        parameters.atmosphere.initial_oxygen = 1000.0;
        parameters.genetics.meta_mutation_rate = 0;
        parameters.genetics.min_mutation_rate = 0;
        parameters.genetics.max_mutation_rate = 0;
        let world = World::with_parameters(
            WorldConfig {
                seed: 827364,
                initial_attempts: 0,
                ..WorldConfig::default()
            },
            parameters,
        )
        .unwrap();
        let victim = Rc::new(RefCell::new(
            world
                .construct_pasted(genome(1, false), [3500, 3500])
                .unwrap()
                .unwrap(),
        ));
        let blocker = Rc::new(RefCell::new(
            world
                .construct_pasted(genome(1, false), [3500, 3500])
                .unwrap()
                .unwrap(),
        ));
        world.add_organism(Rc::clone(&victim), None).unwrap();
        world.add_organism(Rc::clone(&blocker), None).unwrap();
        victim.borrow_mut().id = 10;
        victim.borrow_mut().state.velocity = [3.0, -2.0];
        victim.borrow_mut().state.angular_velocity = 0.7;
        victim.borrow_mut().infected_genome = Some(Rc::new(RefCell::new(genome(1, false))));
        blocker.borrow_mut().id = 20;
        blocker.borrow_mut().state.parent_id = blocker_parent;
        world.next_id.set(30);
        world.rebuild_spatial();
        (world, victim, blocker)
    }

    #[test]
    fn default_candidate_id_preserves_legacy_exclusion_for_unrelated_blocker() {
        let (world, victim_ref, blocker_ref) = world_with_blocker(0);
        let mut victim = victim_ref.borrow_mut();
        let blocker = blocker_ref.borrow_mut();
        let child = world
            .construct_transformation_from(
                &victim_ref,
                &mut victim,
                true,
                Some((&blocker_ref, &blocker)),
            )
            .unwrap()
            .unwrap();
        let child = child.borrow();
        assert_eq!(child.state.center, victim.state.center);
        assert_eq!(child.id, 30);
        assert_eq!(child.state.parent_id, 10);
        assert_eq!(child.state.velocity, [3.0, -2.0]);
        assert_eq!(child.state.angular_velocity, 0.0);
        assert_eq!(victim.state.energy, 0.0);
        assert_eq!(world.next_id.get(), 31);
        assert_eq!(world.organisms.borrow().len(), 2);
    }

    #[test]
    fn fallback_uses_held_contact_peer_and_preserves_parent_velocity_without_dispersal() {
        let (world, victim_ref, blocker_ref) = world_with_blocker(-2);
        let mut victim = victim_ref.borrow_mut();
        let blocker = blocker_ref.borrow_mut();
        let child = world
            .construct_transformation_from(
                &victim_ref,
                &mut victim,
                true,
                Some((&blocker_ref, &blocker)),
            )
            .unwrap()
            .unwrap();
        let child = child.borrow();
        assert_ne!(child.state.center, victim.state.center);
        assert_eq!(child.state.velocity, [3.0, -2.0]);
        assert_eq!(child.state.angular_velocity, 0.0);
    }

    #[test]
    fn first_false_consumes_construction_rng_without_placing_or_spending_resources() {
        let (world, victim_ref, blocker_ref) = world_with_blocker(0);
        let before = (
            world.next_id.get(),
            world.organisms.borrow().len(),
            world.oxygen.get().to_bits(),
            world.carbon_dioxide.get().to_bits(),
        );
        let random = world.random_snapshot();
        let mut victim = victim_ref.borrow_mut();
        let energy = victim.state.energy.to_bits();
        let blocker = blocker_ref.borrow_mut();
        assert!(
            world
                .construct_transformation_from(
                    &victim_ref,
                    &mut victim,
                    false,
                    Some((&blocker_ref, &blocker))
                )
                .unwrap()
                .is_none()
        );
        assert_ne!(world.random_snapshot(), random);
        assert_eq!(victim.state.energy.to_bits(), energy);
        assert_eq!(
            (
                world.next_id.get(),
                world.organisms.borrow().len(),
                world.oxygen.get().to_bits(),
                world.carbon_dioxide.get().to_bits()
            ),
            before
        );
    }

    #[test]
    fn wrapper_adds_birth_without_incrementing_children_or_killing_the_victim() {
        let (world, victim_ref, blocker_ref) = world_with_blocker(0);
        let mut victim = victim_ref.borrow_mut();
        victim.state.total_children = 7;
        let blocker = blocker_ref.borrow_mut();
        world
            .transform_from(&victim_ref, &mut victim, Some((&blocker_ref, &blocker)))
            .unwrap();
        assert_eq!(victim.state.total_children, 7);
        assert!(victim.state.alive);
        assert!(victim.infected_genome.is_none());
        assert!(victim.saved_genome.is_none());
        assert_eq!(world.organisms.borrow().len(), 3);
        assert_eq!(world.population.get(), 3);
        assert_eq!(world.statistics_ref().borrow().get_created_organisms(), 3);
        assert_eq!(
            world.statistics_ref().borrow().get_being_most_children_number(),
            7
        );
        assert!(Rc::ptr_eq(
            world.statistics_ref().borrow().get_last_born_being().unwrap(), world.organisms.borrow()[2].borrow().genome.as_ref().unwrap()));
    }

    #[test]
    fn transformation_applies_all_spore_origins_without_matching_victim_version() {
        for modifier in 1..=12 {
            let (world, victim_ref, blocker_ref) = world_with_blocker(0);
            let mut victim = victim_ref.borrow_mut();
            assert_eq!(victim.effects.spore_version, 0);
            victim.infected_genome = Some(Rc::new(RefCell::new(genome(modifier, true))));
            let blocker = blocker_ref.borrow_mut();
            let child = world
                .construct_transformation_from(
                    &victim_ref,
                    &mut victim,
                    true,
                    Some((&blocker_ref, &blocker)),
                )
                .unwrap()
                .unwrap();
            let child = child.borrow();
            assert!(!child.state.active, "modifier={modifier}");
            if modifier == 3 {
                assert_eq!(child.state.velocity, [3.0, -2.0]);
            }
        }
    }

    fn foreign_transformation_worlds() -> (World, World, OrganismRef, OrganismRef) {
        let (caller, victim_ref, peer_ref) = world_with_blocker(0);
        let owner = World::with_session(
            WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, caller.session(),
        ).unwrap();
        {
            let mut victim = victim_ref.borrow_mut();
            victim.state.world = owner.association();
            victim.state.energy = 100.0;
            victim.state.total_children = 7;
            victim.saved_genome = victim.infected_genome.clone();
        }
        owner.frame_objects.replace(vec![Rc::clone(&victim_ref)]);
        owner.spatial.replace(SpatialTree::build_actor_entries([(0, victim_ref.borrow().state.bounds.get())], |token| owner.spatial_actor(token)));
        caller.next_id.set(117);
        owner.next_id.set(501);
        (caller, owner, victim_ref, peer_ref)
    }

    fn transformation_candidate(owner: &World, victim: &Organism) -> Organism {
        Organism::unplaced(
            Rc::clone(victim.infected_genome.as_ref().unwrap()),
            OrganismState {
                world: owner.association(), parent_id: victim.id as i32, energy: 10.0,
                ..OrganismState::default()
            },
        ).unwrap()
    }

    #[test]
    fn transformation_constructor_keeps_owner_null_and_rng_precedence() {
        for expired in [false, true] {
            let (caller, owner, victim_ref, _peer_ref) = foreign_transformation_worlds();
            let before = caller.random_snapshot();
            let mut victim = victim_ref.borrow_mut();
            let saved = Rc::clone(victim.saved_genome.as_ref().unwrap());
            victim.infected_genome = None;
            victim.state.world = if expired {
                let temporary = World::with_session(
                    WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, caller.session(),
                ).unwrap();
                temporary.association()
            } else {
                WorldAssociation::default()
            };
            assert_eq!(caller.construct_transformation_from(&victim_ref, &mut victim, true, None).err(),
                Some(if expired { WorldError::ExpiredWorld } else { WorldError::MissingWorld }));
            assert_eq!(caller.random_snapshot(), before);
            assert_eq!(victim.state.energy, 100.0);
            assert_eq!(victim.state.total_children, 7);
            assert!(Rc::ptr_eq(victim.saved_genome.as_ref().unwrap(), &saved));
            assert_eq!(caller.next_id.get(), 117);
            assert_eq!(owner.next_id.get(), 501);
            assert!(owner.organisms.borrow().is_empty());
        }
        let (caller, owner, victim_ref, _peer_ref) = foreign_transformation_worlds();
        let mut expected = caller.random_snapshot();
        expected.next_f64();
        let mut victim = victim_ref.borrow_mut();
        victim.infected_genome = None;
        assert_eq!(caller.construct_transformation_from(&victim_ref, &mut victim, true, None).err(),
            Some(WorldError::InvalidParameters("Transformation requires an infected genotype".to_owned())));
        assert_eq!(caller.random_snapshot(), expected);
        assert_eq!(owner.next_id.get(), 501);
        assert_eq!(victim.state.energy, 100.0);
        assert!(victim.saved_genome.is_some());
    }

    #[test]
    fn transformation_placement_uses_child_owner_and_distinct_center_fallback_rules() {
        let (caller, owner, victim_ref, peer_ref) = foreign_transformation_worlds();
        let mut config = caller.config.get();
        config.width = 128;
        config.height = 128;
        caller.config.set(config);
        let huge = IntRect { x: 0, y: 0, width: 7000, height: 7000 };
        owner.frame_objects.replace(vec![Rc::clone(&victim_ref), Rc::clone(&peer_ref)]);
        owner.spatial.replace(SpatialTree::build_actor_entries([(0, huge), (1, huge)], |token| owner.spatial_actor(token)));
        let mut victim = victim_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        peer.state.alive = false;
        peer.state.active = false;
        peer.state.bounds.set(huge);
        let mut child = transformation_candidate(&owner, &victim);
        owner.next_id.set(i32::MAX);
        let mut expected = caller.random_snapshot();
        assert_eq!(child.id, i64::from(peer.state.parent_id));
        assert_eq!(caller.place_transformation(&victim_ref, &mut victim, Some((&peer_ref, &peer)), &mut child), Ok(true));
        assert_eq!(child.state.center, victim.state.center);
        assert_eq!(child.id, i64::from(i32::MAX));
        assert_eq!(child.state.velocity, [3.0, -2.0]);
        assert_eq!(owner.next_id.get(), i32::MIN);
        assert_eq!(caller.next_id.get(), 117);
        assert_eq!(victim.state.energy, 90.0);
        assert_eq!(caller.random_snapshot(), expected);
        peer.id = 0;
        peer.state.parent_id = -2;
        victim.state.energy = 100.0;
        owner.next_id.set(501);
        child = transformation_candidate(&owner, &victim);
        assert_eq!(child.id, peer.id);
        expected.bounded(8);
        assert_eq!(caller.place_transformation(&victim_ref, &mut victim, Some((&peer_ref, &peer)), &mut child), Ok(false));
        assert_eq!(caller.random_snapshot(), expected);
        assert_eq!(owner.next_id.get(), 501);
        assert_eq!(victim.state.energy, 100.0);
        peer.state.bounds.set(victim.state.bounds.get());
        expected.bounded(8);
        assert_eq!(caller.place_transformation(&victim_ref, &mut victim, Some((&peer_ref, &peer)), &mut child), Ok(true));
        assert_ne!(child.state.center, victim.state.center);
        assert_eq!(child.state.velocity, [3.0, -2.0]);
        assert_eq!(child.id, 501);
        assert_eq!(owner.next_id.get(), 502);
        assert_eq!(caller.next_id.get(), 117);
        assert_eq!(victim.state.energy, 90.0);
        assert_eq!(caller.random_snapshot(), expected);
        assert!(owner.organisms.borrow().is_empty());
        assert_eq!(owner.frame_objects.borrow().len(), 2);
        let mut config = owner.config.get();
        config.width = 128;
        config.height = 128;
        owner.config.set(config);
        let mut config = caller.config.get();
        config.width = 7000;
        config.height = 7000;
        caller.config.set(config);
        child = transformation_candidate(&owner, &victim);
        expected.bounded(8);
        assert_eq!(caller.place_transformation(&victim_ref, &mut victim, Some((&peer_ref, &peer)), &mut child), Ok(false));
        assert_eq!(owner.next_id.get(), 502);
        assert_eq!(victim.state.energy, 90.0);
        assert_eq!(caller.random_snapshot(), expected);
        victim.state.center = [-10000.0, -10000.0];
        child.state.world = WorldAssociation::default();
        expected.bounded(8);
        assert_eq!(caller.place_transformation(&victim_ref, &mut victim, Some((&peer_ref, &peer)), &mut child), Ok(false));
        assert_eq!(caller.random_snapshot(), expected);
        assert_eq!(owner.next_id.get(), 502);
        assert_eq!(victim.state.energy, 90.0);
    }

    #[test]
    fn foreign_transformation_spore_payment_and_late_failure_keep_owner_prefixes() {
        let (caller, owner, victim_ref, _peer_ref) = foreign_transformation_worlds();
        caller.session.0.parameters.borrow_mut().spore_energy_consumption = 1.0;
        caller.oxygen.set(41.0);
        caller.carbon_dioxide.set(43.0);
        owner.oxygen.set(101.0);
        owner.carbon_dioxide.set(103.0);
        let mut victim = victim_ref.borrow_mut();
        victim.infected_genome = Some(Rc::new(RefCell::new(genome(1, true))));
        let child = caller.construct_transformation_from(&victim_ref, &mut victim, true, None).unwrap().unwrap();
        let child = child.borrow();
        assert_eq!(child.state.world, owner.association());
        assert!(!child.state.active);
        assert_eq!(child.state.energy, 99.0);
        assert_eq!(owner.oxygen.get(), 100.0);
        assert_eq!(owner.carbon_dioxide.get(), 104.0);
        assert_eq!(caller.oxygen.get(), 41.0);
        assert_eq!(caller.carbon_dioxide.get(), 43.0);
        assert_eq!(victim.state.energy, 0.0);
        assert_eq!(child.id, 501);
        assert_eq!(owner.next_id.get(), 502);
        assert_eq!(caller.next_id.get(), 117);
        assert!(owner.organisms.borrow().is_empty());
        drop(child);
        drop(victim);
        let (caller, owner, victim_ref, _peer_ref) = foreign_transformation_worlds();
        {
            let mut parameters = caller.session.0.parameters.borrow_mut();
            parameters.genetics.min_mutation_rate = 10000;
            parameters.genetics.max_mutation_rate = 10000;
        }
        let mut victim = victim_ref.borrow_mut();
        let inherited = Rc::clone(victim.infected_genome.as_ref().unwrap());
        inherited.borrow_mut().set_clade_id(None);
        assert_eq!(caller.construct_transformation_from(&victim_ref, &mut victim, true, None).err(),
            Some(WorldError::InvalidParameters(crate::GeneticsError::MissingClade.to_string())));
        assert_eq!(owner.next_id.get(), 502);
        assert_eq!(caller.next_id.get(), 117);
        assert_eq!(victim.state.energy, 0.0);
        assert_eq!(victim.state.total_children, 7);
        assert!(Rc::ptr_eq(victim.infected_genome.as_ref().unwrap(), &inherited));
        assert!(Rc::ptr_eq(victim.saved_genome.as_ref().unwrap(), &inherited));
        assert!(owner.organisms.borrow().is_empty());
        assert_eq!(owner.population.get(), 0);
        assert_eq!(owner.statistics_ref().borrow().get_created_organisms(), 0);
    }

    #[test]
    fn transformation_wrapper_publishes_in_victim_owner_without_birth_or_death_counters() {
        let (caller, owner, victim_ref, _peer_ref) = foreign_transformation_worlds();
        let mut config = caller.config.get();
        config.width = 128;
        config.height = 128;
        caller.config.set(config);
        let mut victim = victim_ref.borrow_mut();
        caller.transform_from(&victim_ref, &mut victim, None).unwrap();
        assert_eq!(victim.state.total_children, 7);
        assert!(victim.state.alive);
        assert!(victim.infected_genome.is_none());
        assert!(victim.saved_genome.is_none());
        assert_eq!(caller.organisms.borrow().len(), 2);
        assert_eq!(caller.population.get(), 2);
        assert_eq!(caller.statistics_ref().borrow().get_created_organisms(), 2);
        assert_eq!(owner.organisms.borrow().len(), 1);
        assert_eq!(owner.population.get(), 1);
        assert_eq!(owner.statistics_ref().borrow().get_created_organisms(), 1);
        assert_eq!(owner.statistics_ref().borrow().get_being_most_children_number(), 7);
        let child_ref = Rc::clone(&owner.organisms.borrow()[0]);
        let child = child_ref.borrow();
        assert_eq!(child.state.world, owner.association());
        assert_ne!(child.state.world, caller.association());
        assert_eq!(child.id, 501);
        assert_eq!(owner.next_id.get(), 502);
        assert_eq!(caller.next_id.get(), 117);
        assert!(Rc::ptr_eq(owner.statistics_ref().borrow().get_last_born_being().unwrap(), child.genome.as_ref().unwrap()));
        assert_eq!(owner.frame_objects.borrow().len(), 1);
        assert!(caller.session().same_owner(&owner.session()));
        assert_eq!(caller.random_snapshot(), owner.random_snapshot());
    }
}
