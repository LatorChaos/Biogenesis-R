// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Ordered reproduction callers from Organism.java:3667-4156

use crate::organism::primary_genome;

use super::*;
use crate::java_math::min;

fn base_children(parent: &Organism) -> i32 {
    if !parent.effects.is_silver {
        1
    } else if parent.effects.is_consumer || parent.effects.is_fungus || parent.effects.is_infectious
    {
        [1, 9, 36, 100, 225, 441, 784]
            .iter()
            .position(|threshold| parent.state.total_children < *threshold)
            .map_or(8, |index| index as i32 + 1)
    } else {
        [2, 5, 9, 14, 20, 27]
            .iter()
            .position(|threshold| parent.state.total_children < *threshold)
            .map_or(8, |index| index as i32 + 2)
    }
}

impl World {
    fn ordinary_children(&self, parent: &mut Organism) -> Result<(), crate::WorldError> {
        Ok({
        parent.state.children = base_children(parent);
        if parent.effects.yellow_count > 0
            && parent.use_energy(
                self.session.0.parameters.borrow().lifecycle.yellow_energy_consumption)?
        {
            parent.state.children = parent
                .state
                .children
                .wrapping_add(parent.effects.yellow_count);
        }
    
        })
    }

    fn add_reproduced_child(&self, parent: &mut Organism, child: &OrganismRef) -> Result<(), WorldError> {
        parent.state.total_children = parent.state.total_children.wrapping_add(1);
        parent.state.world.resolve()?.add_organism(Rc::clone(child), Some(parent))?;
        Ok(())
    }

    pub(super) fn reproduce_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        parent.state.reproduction.countdown = parent.state.reproduction.maximum_countdown;
        self.ordinary_children(parent)?;
        if parent.effects.is_auburn
            && parent.infected_genome.is_some()
            && parent.use_energy(
                self.session.0.parameters.borrow().lifecycle.auburn_energy_consumption)?
        {
            parent.state.children = parent.state.children.wrapping_add(1);
        }
        let spore = parent.effects.spore_version > 0;
        let mut index = 0;
        while index < parent.state.children.clamp(1, 8) {
            let Some(child) = self.construct_child_from(parent_ref, parent, index == 0, peer)?
            else {
                break;
            };
            self.add_reproduced_child(parent, &child)?;
            if !spore {
                parent.infected_genome = None;
            }
            parent.saved_genome = None;
            index += 1;
        }
        Ok(())
    }

    pub(super) fn reproduce_early_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        parent.state.reproduction.countdown = parent.state.reproduction.maximum_countdown;
        self.ordinary_children(parent)?;
        let mut use_blond_costs = true;
        let mut index = 0;
        while index < parent.state.children.clamp(1, 8) {
            let Some(child) = self.construct_child_from(parent_ref, parent, index == 0, peer)?
            else {
                break;
            };
            self.add_reproduced_child(parent, &child)?;
            if parent.effects.is_blond && use_blond_costs {
                use_blond_costs = false;
                parent.use_energy(
                    min(
                        parent.state.energy,
                        self.session.0.parameters.borrow().lifecycle.blond_energy_consumption,
                    ))?;
            }
            parent.saved_genome = None;
            index += 1;
        }
        Ok(())
    }

    pub(super) fn reproduce_early_plant_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        parent.state.reproduction.countdown = parent.state.reproduction.maximum_countdown;
        self.ordinary_children(parent)?;
        let mut use_blond_costs = true;
        let mut index = 0;
        while index < parent.state.children.clamp(1, 8) {
            let Some(child) = self.construct_child_from(parent_ref, parent, index == 0, peer)?
            else {
                break;
            };
            self.add_reproduced_child(parent, &child)?;
            if parent.effects.is_blond && use_blond_costs {
                use_blond_costs = false;
                let amount = if parent.effects.is_only_c4 || parent.effects.reproduce_late > 0 {
                    self.session.0.parameters.borrow().lifecycle.blond_energy_consumption
                } else {
                    self.session.0.parameters.borrow().lifecycle.blond_energy_consumption
                        * f64::from(
                            parent
                                .state
                                .reproduction
                                .energy
                                .wrapping_sub(parent.state.reproduction.early_energy),
                        )
                };
                parent.use_energy(
                    min(parent.state.energy, amount))?;
            }
            parent.saved_genome = None;
            index += 1;
        }
        Ok(())
    }

    pub(super) fn reproduce_early_virus_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        parent.state.reproduction.countdown = parent.state.reproduction.maximum_countdown;
        parent.state.children = 1;
        if let Some(child) = self.construct_child_from(parent_ref, parent, true, peer)? {
            self.add_reproduced_child(parent, &child)?;
            if parent.effects.is_blond {
                parent.use_energy(
                    min(
                        parent.state.energy,
                        self.session.0.parameters.borrow().lifecycle.blond_energy_consumption,
                    ))?;
            }
            if parent.effects.spore_version <= 4 {
                parent.state.reproduction.countdown = 0;
                parent.saved_genome = child.borrow().genome.clone();
            } else {
                parent.saved_genome = None;
            }
        }
        Ok(())
    }

    pub(super) fn reproduce_spore_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        parent.state.reproduction.countdown = parent.state.reproduction.maximum_countdown;
        parent.state.children = 1;
        let initial = self.initial_child_state(parent)?;
        if parent.infected_genome.is_some() {
            parent.infected_genome = None;
            parent.saved_genome = None;
        }
        if let Some(child) =
            self.construct_child_with_initial_from(parent_ref, parent, initial, true, peer)?
        {
            self.add_reproduced_child(parent, &child)?;
            parent.saved_genome = None;
            let mut child = child.borrow_mut();
            if child.effects.spore_version < 0 && child.state.active {
                let moving = primary_genome(&child.genome)?.borrow().modifiers().spore >= 10;
                if moving {
                    crate::birth_spore::cyan_spore(&mut child)
                } else {
                    crate::birth_spore::blue_spore(&mut child)
                }
                .map_err(WorldError::Geometry)?;
            }
        }
        Ok(())
    }

    pub(super) fn reproduce_yellow_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        if parent.state.reproduction.countdown < parent.state.reproduction.maximum_countdown {
            parent.state.reproduction.countdown = parent.state.reproduction.maximum_countdown;
        }
        let mut victimized = false;
        self.ordinary_children(parent)?;
        if parent.effects.is_auburn
            && parent.use_energy(
                self.session.0.parameters.borrow().lifecycle.auburn_energy_consumption)?
        {
            parent.state.children = parent.state.children.wrapping_add(1);
        }
        let mut index = 0;
        while index < parent.state.virus_children.clamp(1, 8) {
            if parent.infected_genome.is_some() {
                let Some(child) =
                    self.construct_child_from(parent_ref, parent, index == 0, peer)?
                else {
                    break;
                };
                self.add_reproduced_child(parent, &child)?;
                parent.saved_genome = None;
                victimized = true;
            }
            index += 1;
        }
        if !victimized {
            parent.state.virus_children = 0;
        }
        Ok(())
    }

    pub(super) fn reproduce_virus_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        parent.state.virus_children = 1;
        parent.state.children = base_children(parent);
        if parent.effects.yellow_count > 0
            && parent.use_energy(
                self.session.0.parameters.borrow().lifecycle.yellow_energy_consumption)?
        {
            parent.state.children = (f64::from(parent.state.children)
                + f64::from(parent.effects.yellow_count)
                    * crate::GROWTH_SCALES[crate::geometry::growth_index(parent.body.growth_ratio)?])
                as i32;
        }
        if parent.effects.is_auburn
            && parent.use_energy(
                self.session.0.parameters.borrow().lifecycle.auburn_energy_consumption)?
        {
            parent.state.children = parent.state.children.wrapping_add(1);
        }
        if let Some(child) = self.construct_child_from(parent_ref, parent, true, peer)? {
            self.add_reproduced_child(parent, &child)?;
            parent.infected_genome = None;
            parent.saved_genome = None;
            if parent.state.reproduction.countdown < parent.state.reproduction.maximum_countdown {
                parent.state.reproduction.countdown = parent.state.reproduction.maximum_countdown;
            }
        } else {
            parent.state.virus_children = 0;
        }
        Ok(())
    }

    pub(super) fn reproduce_foreign_virus_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<(), WorldError> {
        parent.state.children = base_children(parent);
        if parent.state.virus_children != -1 {
            parent.state.virus_children = -1;
            if parent.effects.is_auburn
                && parent.use_energy(
                    self.session.0.parameters.borrow().lifecycle.auburn_energy_consumption)?
            {
                parent.state.children = parent.state.children.wrapping_add(1);
            }
        }
        if let Some(child) = self.construct_child_from(parent_ref, parent, true, peer)? {
            self.add_reproduced_child(parent, &child)?;
            parent.infected_genome = None;
            parent.saved_genome = None;
            if parent.state.reproduction.countdown < parent.state.reproduction.maximum_countdown {
                parent.state.reproduction.countdown = parent.state.reproduction.maximum_countdown;
            }
        } else {
            parent.state.virus_children = 0;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorKind, Gene, GenomeModifiers, GenomeProperties};

    fn parent_world() -> (World, OrganismRef) {
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
        let gene = Gene::new(12.0, 0.25, ColorKind::Green.argb(), -1, [0; 36]).shared();
        let genome = Genome::from_parts(
            &[gene],
            GenomeProperties {
                symmetry: 1,
                modifiers: GenomeModifiers {
                    cream: 1,
                    fallow: 1,
                    spore: 1,
                    adapt_spore: 1,
                    ..GenomeModifiers::default()
                },
                ..GenomeProperties::default()
            },
        );
        let parent = Rc::new(RefCell::new(
            world
                .construct_pasted(genome, [3500, 3500])
                .unwrap()
                .unwrap(),
        ));
        world.add_organism(Rc::clone(&parent), None).unwrap();
        world.rebuild_spatial();
        (world, parent)
    }

    #[test]
    fn silver_thresholds_preserve_signed_totals_and_all_three_consumer_flags() {
        let (_, parent) = parent_world();
        let mut parent = parent.borrow_mut();
        assert_eq!(base_children(&parent), 1);
        parent.effects.is_silver = true;
        for (total, consumer, other) in [
            (i32::MIN, 1, 2),
            (0, 1, 2),
            (1, 2, 2),
            (2, 2, 3),
            (4, 2, 3),
            (5, 2, 4),
            (8, 2, 4),
            (9, 3, 5),
            (13, 3, 5),
            (14, 3, 6),
            (19, 3, 6),
            (20, 3, 7),
            (26, 3, 7),
            (27, 3, 8),
            (35, 3, 8),
            (36, 4, 8),
            (99, 4, 8),
            (100, 5, 8),
            (224, 5, 8),
            (225, 6, 8),
            (440, 6, 8),
            (441, 7, 8),
            (783, 7, 8),
            (784, 8, 8),
            (i32::MAX, 8, 8),
        ] {
            parent.state.total_children = total;
            for flag in 0..4 {
                parent.effects.is_consumer = flag == 1;
                parent.effects.is_fungus = flag == 2;
                parent.effects.is_infectious = flag == 3;
                assert_eq!(
                    base_children(&parent),
                    if flag == 0 { other } else { consumer },
                    "total={total} flag={flag}"
                );
            }
        }
    }

    #[test]
    fn yellow_without_infection_pays_front_costs_without_allocating_children() {
        let (world, parent_ref) = parent_world();
        world.session.0.parameters.borrow_mut().lifecycle.yellow_energy_consumption = 0.25;
        world.session.0.parameters.borrow_mut().lifecycle.auburn_energy_consumption = 0.5;
        let random = world.random_snapshot();
        let id = world.next_id.get();
        let gases = [world.oxygen.get(), world.carbon_dioxide.get()];
        let mut parent = parent_ref.borrow_mut();
        parent.state.energy = 20.0;
        parent.state.virus_children = 4;
        parent.effects.yellow_count = 3;
        parent.effects.is_auburn = true;
        world
            .reproduce_yellow_from(&parent_ref, &mut parent, None)
            .unwrap();
        assert_eq!(parent.state.children, 5);
        assert_eq!(parent.state.virus_children, 0);
        assert_eq!(parent.state.energy, 19.25);
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(world.next_id.get(), id);
        assert_eq!(world.organisms.borrow().len(), 1);
        assert_eq!(world.oxygen.get(), gases[0] - 0.75);
        assert_eq!(world.carbon_dioxide.get(), gases[1] + 0.75);
    }

    #[test]
    fn successful_birth_updates_wrapping_parent_count_before_synchronous_statistics() {
        let (mut world, parent_ref) = parent_world();
        let mut parent = parent_ref.borrow_mut();
        parent.state.total_children = i32::MAX;
        world
            .reproduce_from(&parent_ref, &mut parent, None)
            .unwrap();
        assert_eq!(parent.state.total_children, i32::MIN);
        assert_eq!(world.organisms.borrow().len(), 2);
        assert_eq!(world.population.get(), 2);
        assert_eq!(world.statistics_ref().borrow().get_created_organisms(), 2);
        assert!(
            world
                .statistics_ref()
                .borrow()
                .get_being_most_children()
                .is_none()
        );
        assert!(Rc::ptr_eq(
            world.statistics_ref().borrow().get_last_born_being().unwrap(), world.organisms.borrow()[1].borrow().genome.as_ref().unwrap()));
        assert_eq!(world.frame_objects.borrow().len(), 1);
    }

    #[test]
    fn early_virus_saves_the_actual_inserted_child_genotype() {
        let (world, parent_ref) = parent_world();
        let mut parent = parent_ref.borrow_mut();
        parent.effects.is_blond = true;
        world
            .reproduce_early_virus_from(&parent_ref, &mut parent, None)
            .unwrap();
        assert_eq!(world.organisms.borrow().len(), 2);
        assert_eq!(parent.state.total_children, 1);
        assert_eq!(parent.state.reproduction.countdown, 0);
        assert!(Rc::ptr_eq(
            parent.saved_genome.as_ref().unwrap(), world.organisms.borrow()[1].borrow().genome.as_ref().unwrap()));
    }


    fn ordinary_birth_pair() -> (World, World, OrganismRef) {
        let (caller, parent_ref) = parent_world();
        let owner = World::with_session(
            WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, caller.session(),
        ).unwrap();
        {
            let mut parent = parent_ref.borrow_mut();
            parent.state.world = owner.association();
            parent.effects = crate::EffectsState::default();
            parent.state.energy = 100.0;
            parent.state.velocity = [2.0, -3.0];
            parent.state.children = 1;
            parent.state.virus_children = 1;
            parent.state.total_children = 0;
            parent.state.reproduction.energy = 60;
            parent.state.reproduction.early_energy = 0;
            parent.state.reproduction.infected_energy = 60;
            parent.state.reproduction.maximum_countdown = 19;
            parent.state.reproduction.countdown = 3;
        }
        owner.frame_objects.replace(vec![Rc::clone(&parent_ref)]);
        owner.spatial.replace(SpatialTree::build_actor_entries([(0, parent_ref.borrow().state.bounds.get())], |token| owner.spatial_actor(token)));
        caller.next_id.set(117);
        owner.next_id.set(501);
        (caller, owner, parent_ref)
    }

    fn placement_child(owner: &World, parent: &Organism) -> Organism {
        Organism::unplaced(
            Rc::clone(parent.genome.as_ref().unwrap()),
            OrganismState { world: owner.association(), energy: 10.0, ..OrganismState::default() },
        ).unwrap()
    }

    fn frozen_blocker(owner: &World, bounds: IntRect) -> OrganismRef {
        let actor = Organism::pasted_candidate(None, OrganismState {
            world: owner.association(), alive: false, active: false, ..OrganismState::default()
        });
        actor.state.bounds.set(bounds);
        Rc::new(RefCell::new(actor))
    }

    #[test]
    fn ordinary_birth_variants_publish_in_parent_owner_and_wrap_its_id() {
        for variant in 0..8 {
            let (caller, owner, parent_ref) = ordinary_birth_pair();
            let mut config = caller.config.get();
            config.width = 128;
            config.height = 128;
            caller.config.set(config);
            owner.next_id.set(i32::MAX);
            let random_before = caller.random_snapshot();
            let mut parent = parent_ref.borrow_mut();
            parent.infected_genome = parent.genome.clone();
            match variant {
                0 => caller.reproduce_from(&parent_ref, &mut parent, None),
                1 => caller.reproduce_early_from(&parent_ref, &mut parent, None),
                2 => caller.reproduce_early_plant_from(&parent_ref, &mut parent, None),
                3 => caller.reproduce_early_virus_from(&parent_ref, &mut parent, None),
                4 => caller.reproduce_spore_from(&parent_ref, &mut parent, None),
                5 => caller.reproduce_yellow_from(&parent_ref, &mut parent, None),
                6 => caller.reproduce_virus_from(&parent_ref, &mut parent, None),
                7 => caller.reproduce_foreign_virus_from(&parent_ref, &mut parent, None),
                _ => unreachable!(),
            }.unwrap();
            assert_eq!(parent.state.total_children, 1, "variant={variant}");
            assert_eq!(parent.state.energy, 70.0, "variant={variant}");
            assert_eq!(caller.next_id.get(), 117);
            assert_eq!(owner.next_id.get(), i32::MIN);
            assert_eq!(caller.organisms.borrow().len(), 1);
            assert_eq!(owner.organisms.borrow().len(), 1);
            assert_eq!(caller.population.get(), 1);
            assert_eq!(owner.population.get(), 1);
            assert_eq!(caller.statistics_ref().borrow().get_created_organisms(), 1);
            assert_eq!(owner.statistics_ref().borrow().get_created_organisms(), 1);
            let child_ref = Rc::clone(&owner.organisms.borrow()[0]);
            let child = child_ref.borrow();
            assert_eq!(child.id, i64::from(i32::MAX));
            assert_eq!(child.state.parent_id, parent.id as i32);
            assert_eq!(child.state.world, owner.association());
            assert_ne!(child.state.world, caller.association());
            assert!(Rc::ptr_eq(
                owner.statistics_ref().borrow().get_last_born_being().unwrap(),
                child.genome.as_ref().unwrap(),
            ));
            assert_eq!(owner.frame_objects.borrow().len(), 1);
            assert!(caller.session().same_owner(&owner.session()));
            assert_eq!(caller.random_snapshot(), owner.random_snapshot());
            assert_ne!(caller.random_snapshot(), random_before);
        }
    }

    #[test]
    fn child_constructor_checks_world_before_theta_and_spore_reference_clears() {
        let (caller, owner, parent_ref) = ordinary_birth_pair();
        let mut expected = caller.random_snapshot();
        let theta = expected.next_f64() * PI * 2.0;
        let initial = caller.initial_child_state(&parent_ref.borrow()).unwrap();
        assert_eq!(initial.world, owner.association());
        assert_eq!(initial.angle.to_bits(), theta.to_bits());
        assert_eq!(caller.random_snapshot(), expected);
        for expired in [false, true] {
            let (caller, _owner, parent_ref) = ordinary_birth_pair();
            let association = if expired {
                let temporary = World::with_session(
                    WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, caller.session(),
                ).unwrap();
                temporary.association()
            } else {
                WorldAssociation::default()
            };
            let before = caller.random_snapshot();
            let mut parent = parent_ref.borrow_mut();
            let code = Rc::clone(parent.genome.as_ref().unwrap());
            parent.infected_genome = Some(Rc::clone(&code));
            parent.saved_genome = Some(Rc::clone(&code));
            parent.genome = None;
            parent.state.world = association;
            let error = caller.reproduce_spore_from(&parent_ref, &mut parent, None).unwrap_err();
            assert_eq!(error, if expired { WorldError::ExpiredWorld } else { WorldError::MissingWorld });
            assert_eq!(parent.state.reproduction.countdown, 19);
            assert_eq!(parent.state.children, 1);
            assert_eq!(parent.state.total_children, 0);
            assert_eq!(parent.state.energy, 100.0);
            assert!(parent.genome.is_none());
            assert!(Rc::ptr_eq(parent.infected_genome.as_ref().unwrap(), &code));
            assert!(Rc::ptr_eq(parent.saved_genome.as_ref().unwrap(), &code));
            assert_eq!(caller.random_snapshot(), before);
            assert_eq!(caller.next_id.get(), 117);
            assert_eq!(caller.organisms.borrow().len(), 1);
        }
        let (caller, _owner, parent_ref) = ordinary_birth_pair();
        let mut expected = caller.random_snapshot();
        expected.next_f64();
        {
            let mut parent = parent_ref.borrow_mut();
            let code = Rc::clone(parent.genome.as_ref().unwrap());
            parent.infected_genome = Some(Rc::clone(&code));
            parent.saved_genome = Some(code);
            parent.genome = None;
            assert_eq!(caller.reproduce_spore_from(&parent_ref, &mut parent, None),
                Err(WorldError::Geometry(GeometryError::MissingGenome)));
            assert!(parent.infected_genome.is_none());
            assert!(parent.saved_genome.is_none());
            assert_eq!(parent.state.energy, 100.0);
            assert_eq!(parent.state.total_children, 0);
            assert_eq!(caller.random_snapshot(), expected);
        }
        let (caller, _owner, parent_ref) = ordinary_birth_pair();
        let before = caller.random_snapshot();
        let mut parent = parent_ref.borrow_mut();
        parent.state.world = WorldAssociation::default();
        parent.genome = None;
        caller.reproduce_yellow_from(&parent_ref, &mut parent, None).unwrap();
        assert_eq!(parent.state.reproduction.countdown, 19);
        assert_eq!(parent.state.children, 1);
        assert_eq!(parent.state.virus_children, 0);
        assert_eq!(parent.state.energy, 100.0);
        assert_eq!(caller.random_snapshot(), before);
        assert_eq!(caller.next_id.get(), 117);
    }

    #[test]
    fn child_placement_uses_owner_frozen_tree_live_rectangles_and_held_identities() {
        let (caller, owner, parent_ref) = ordinary_birth_pair();
        let huge = IntRect { x: 0, y: 0, width: 7000, height: 7000 };
        let peer_ref = frozen_blocker(&owner, huge);
        let caller_blocker = frozen_blocker(&caller, huge);
        caller.frame_objects.replace(vec![caller_blocker]);
        caller.spatial.replace(SpatialTree::build_actor_entries([(0, huge)], |token| caller.spatial_actor(token)));
        owner.frame_objects.replace(vec![Rc::clone(&parent_ref), Rc::clone(&peer_ref)]);
        owner.spatial.replace(SpatialTree::build_actor_entries([(0, huge), (1, huge)], |token| owner.spatial_actor(token)));
        let mut parent = parent_ref.borrow_mut();
        let peer = peer_ref.borrow_mut();
        let mut child = placement_child(&owner, &parent);
        parent.state.world = caller.association();
        parent.state.children = 2;
        assert_eq!(child.id, peer.id);
        let mut expected = caller.random_snapshot();
        expected.bounded(8);
        assert_eq!(caller.place_child(&parent_ref, &mut parent, Some((&peer_ref, &peer)), &mut child, true), Ok(false));
        assert_eq!(caller.random_snapshot(), expected);
        assert_eq!(owner.next_id.get(), 501);
        assert_eq!(parent.state.energy, 100.0);
        assert_eq!(child.id, 0);
        assert!(owner.organisms.borrow().is_empty());
        assert_eq!(parent.state.network_position.as_ref().unwrap().units(), &[48, 49, 50, 51, 52, 53, 54, 55]);
        peer.state.bounds.set(IntRect { x: -10000, y: -10000, width: 1, height: 1 });
        expected.bounded(8);
        assert_eq!(caller.place_child(&parent_ref, &mut parent, Some((&peer_ref, &peer)), &mut child, false), Ok(true));
        assert_eq!(caller.random_snapshot(), expected);
        assert_eq!(child.id, 501);
        assert_eq!(child.state.velocity, [2.0, -3.0]);
        assert_eq!(parent.state.energy, 90.0);
        assert_eq!(owner.next_id.get(), 502);
        assert_eq!(caller.next_id.get(), 117);
        assert_eq!(owner.frame_objects.borrow().len(), 2);
        assert_eq!(parent.state.network_position.as_ref().unwrap().units().iter().filter(|&&unit| unit == u16::from(b'X')).count(), 1);
        let mut config = owner.config.get();
        config.width = 128;
        config.height = 128;
        owner.config.set(config);
        expected.bounded(8);
        assert_eq!(caller.place_child(&parent_ref, &mut parent, Some((&peer_ref, &peer)), &mut child, false), Ok(false));
        assert_eq!(caller.random_snapshot(), expected);
        assert_eq!(child.id, 501);
        assert_eq!(owner.next_id.get(), 502);
        assert_eq!(parent.state.energy, 90.0);
    }

    #[test]
    fn litter_position_null_is_reached_after_query_before_genome_id_and_debit() {
        let (caller, owner, parent_ref) = ordinary_birth_pair();
        let huge = IntRect { x: 0, y: 0, width: 7000, height: 7000 };
        let blocker = frozen_blocker(&owner, huge);
        owner.frame_objects.replace(vec![blocker]);
        owner.spatial.replace(SpatialTree::build_actor_entries([(0, huge)], |token| owner.spatial_actor(token)));
        let mut parent = parent_ref.borrow_mut();
        let mut child = placement_child(&owner, &parent);
        child.state.velocity = [9.0, 8.0];
        parent.state.children = 2;
        parent.state.network_position = None;
        parent.genome = None;
        assert_eq!(caller.place_child(&parent_ref, &mut parent, None, &mut child, false), Ok(false));
        owner.spatial.replace(SpatialTree::empty());
        let mut expected = caller.random_snapshot();
        expected.bounded(8);
        assert_eq!(caller.place_child(&parent_ref, &mut parent, None, &mut child, false), Err(WorldError::MissingNetworkPosition));
        assert_eq!(caller.random_snapshot(), expected);
        assert_eq!(child.state.velocity, [9.0, 8.0]);
        assert_eq!(child.id, 0);
        assert_eq!(parent.state.energy, 100.0);
        assert_eq!(owner.next_id.get(), 501);
        parent.state.network_position = Some(crate::JavaText::from_utf8("01234567"));
        assert_eq!(caller.place_child(&parent_ref, &mut parent, None, &mut child, false), Err(WorldError::Geometry(GeometryError::MissingGenome)));
        assert_eq!(owner.next_id.get(), 501);
        parent.state.children = 1;
        parent.state.center = [-10000.0, -10000.0];
        child.state.world = WorldAssociation::default();
        assert_eq!(caller.place_child(&parent_ref, &mut parent, None, &mut child, false), Ok(false));
        assert_eq!(child.id, 0);
        assert_eq!(parent.state.energy, 100.0);
    }

    #[test]
    fn late_birth_lineage_failure_retains_owner_id_energy_and_shared_genome_prefix() {
        let (caller, owner, parent_ref) = ordinary_birth_pair();
        let mut parent = parent_ref.borrow_mut();
        parent.effects.spore_version = 1;
        parent.effects.spore_time = 20;
        let code = Rc::clone(parent.genome.as_ref().unwrap());
        code.borrow_mut().set_clade_id(None);
        code.borrow_mut().set_update_clade(true);
        let mut expected = caller.random_snapshot();
        expected.next_f64();
        expected.bounded(8);
        let error = caller.reproduce_from(&parent_ref, &mut parent, None).unwrap_err();
        assert_eq!(error, WorldError::InvalidParameters(crate::GeneticsError::MissingClade.to_string()));
        assert_eq!(owner.next_id.get(), 502);
        assert_eq!(caller.next_id.get(), 117);
        assert_eq!(parent.state.energy, 80.0);
        assert_eq!(parent.state.total_children, 0);
        assert!(parent.saved_genome.is_none());
        assert!(Rc::ptr_eq(parent.genome.as_ref().unwrap(), &code));
        assert!(!code.borrow().update_clade());
        assert_eq!(caller.random_snapshot(), expected);
        assert!(owner.organisms.borrow().is_empty());
        assert_eq!(owner.population.get(), 0);
        assert_eq!(owner.statistics_ref().borrow().get_created_organisms(), 0);
    }

    #[test]
    fn birth_publication_resolves_parent_after_count_without_retargeting_child() {
        let (caller, owner, parent_ref) = ordinary_birth_pair();
        let child_ref = Rc::new(RefCell::new(placement_child(&caller, &parent_ref.borrow())));
        let mut parent = parent_ref.borrow_mut();
        parent.state.total_children = i32::MAX;
        caller.add_reproduced_child(&mut parent, &child_ref).unwrap();
        assert_eq!(parent.state.total_children, i32::MIN);
        assert!(Rc::ptr_eq(&owner.organisms.borrow()[0], &child_ref));
        assert_eq!(child_ref.borrow().state.world, caller.association());
        assert_eq!(caller.organisms.borrow().len(), 1);
        assert_eq!(owner.population.get(), 1);
        assert_eq!(owner.statistics_ref().borrow().get_created_organisms(), 1);
        for expired in [false, true] {
            parent.state.world = if expired {
                let temporary = World::with_session(
                    WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, caller.session(),
                ).unwrap();
                temporary.association()
            } else {
                WorldAssociation::default()
            };
            parent.state.total_children = i32::MAX;
            let error = caller.add_reproduced_child(&mut parent, &child_ref).unwrap_err();
            assert_eq!(error, if expired { WorldError::ExpiredWorld } else { WorldError::MissingWorld });
            assert_eq!(parent.state.total_children, i32::MIN);
            assert_eq!(owner.organisms.borrow().len(), 1);
            assert_eq!(caller.organisms.borrow().len(), 1);
            assert_eq!(owner.population.get(), 1);
        }
    }

    #[test]
    fn host_spawn_child_publishes_in_parent_owner_without_incrementing_total_children() {
        let (caller, owner, parent_ref) = ordinary_birth_pair();
        let parent_id = parent_ref.borrow().id;
        parent_ref.borrow_mut().state.total_children = 15;
        assert_eq!(caller.spawn_child(parent_id, true), Ok(Some(501)));
        assert_eq!(parent_ref.borrow().state.total_children, 15);
        assert_eq!(caller.organisms.borrow().len(), 1);
        assert_eq!(owner.organisms.borrow().len(), 1);
        assert_eq!(owner.organisms.borrow()[0].borrow().state.world, owner.association());
        assert_eq!(owner.next_id.get(), 502);
        assert_eq!(caller.next_id.get(), 117);
    }
}
