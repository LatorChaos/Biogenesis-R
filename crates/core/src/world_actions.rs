// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use super::{OrganismRef, OrganismSnapshot, World};
use std::rc::Rc;

impl World {
    /// Select the first overlapping live member, or the last overlapping corpse
    pub(crate) fn find_organism_from_position(&self, position: [i32; 2]) -> Result<Option<OrganismRef>, crate::WorldError> {
        let mut corpse = None;
        for index in 0..self.organisms.len()? {
            let reference = self.organisms.actor(index)?;
            let actor = reference.borrow();
            if actor.state.bounds.get().contains(position) {
                if actor.state.alive {
                    return Ok(Some(Rc::clone(&reference)));
                }
                corpse = Some(Rc::clone(&reference));
            }
        }
        Ok(corpse)
    }

    /// Return detached inspection data for the member selected by source rectangle priority
    pub fn inspect_at_position(&self, position: [i32; 2]) -> Result<Option<OrganismSnapshot>, crate::WorldError> {
        Ok(self.find_organism_from_position(position)?.map(|actor| actor.borrow().snapshot()))
    }

    /// Kill live members synchronously and leave corpse removal to subsequent frames
    pub fn kill_all(&self) -> Result<(), crate::WorldError> {
        let count = self.organisms.len()?;
        for index in 0..count {
            let reference = self.organisms.actor(index)?;
            let mut actor = reference.borrow_mut();
            if actor.state.alive {
                self.die(&mut actor, None)?;
            }
        }
        Ok(())
    }

    /// Spend every corpse's current energy in membership order, subject to available oxygen
    pub fn disperse_all(&self) -> Result<(), crate::WorldError> {
        Ok({
        let count = self.organisms.len()?;
        for index in 0..count {
            let reference = self.organisms.actor(index)?;
            let mut actor = reference.borrow_mut();
            if !actor.state.alive {
                let energy = actor.state.energy;
                let _ = actor.use_energy(energy)?;
            }
        }
    
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistics::tests::{Observation, observed_field};
    use crate::{ColorKind, Gene, Genome, GenomeProperties, IntRect, WorldConfig};

    fn world() -> World {
        World::new(WorldConfig {
            seed: 827364,
            initial_attempts: 0,
            width: 4000,
            height: 4000,
        })
        .unwrap()
    }

    fn member(world: &mut World, center: [i32; 2]) -> OrganismRef {
        let code = Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        world.paste_genome(code, center).unwrap().unwrap();
        Rc::clone(world.organisms.borrow().last().unwrap())
    }

    #[test]
    fn positional_selection_preserves_live_priority_last_corpse_and_duplicate_id_identity() {
        let mut world = world();
        let actors: Vec<_> = [100, 300, 500, 700]
            .into_iter()
            .map(|x| member(&mut world, [x, 100]))
            .collect();
        world.rebuild_spatial();
        for (index, reference) in actors.iter().enumerate() {
            let mut actor = reference.borrow_mut();
            actor.id = 7;
            actor.state.energy = 10.0 + index as f64;
            actor.state.bounds.set(IntRect {
                x: 2000,
                y: 2000,
                width: 20,
                height: 20,
            });
        }
        world.die(&mut actors[0].borrow_mut(), None).unwrap();
        world.die(&mut actors[2].borrow_mut(), None).unwrap();
        assert!(
            world.spatial.borrow().query(IntRect {
                    x: 2001,
                    y: 2001,
                    width: 1,
                    height: 1
                })
                .is_empty()
        );
        assert!(Rc::ptr_eq(
            &world.find_organism_from_position([2001, 2001]).unwrap().unwrap(),
            &actors[1]
        ));
        assert_eq!(
            world
                .inspect_at_position([2001, 2001]).unwrap()
                .unwrap()
                .state
                .energy,
            11.0
        );
        world.die(&mut actors[1].borrow_mut(), None).unwrap();
        assert!(Rc::ptr_eq(
            &world.find_organism_from_position([2001, 2001]).unwrap().unwrap(),
            &actors[3]
        ));
        world.kill_all().unwrap();
        actors[3].borrow_mut().state.energy = 0.0;
        assert!(Rc::ptr_eq(
            &world.find_organism_from_position([2001, 2001]).unwrap().unwrap(),
            &actors[3]
        ));
        actors[3].borrow().state.bounds.update(|bounds| bounds.x = 3000);
        assert!(Rc::ptr_eq(
            &world.find_organism_from_position([2001, 2001]).unwrap().unwrap(),
            &actors[2]
        ));
        let before = world.clone();
        let mut snapshot = world.inspect_at_position([2001, 2001]).unwrap().unwrap();
        snapshot.state.energy = 999.0;
        assert_eq!(actors[2].borrow().state.energy, 12.0);
        assert_eq!(world, before);
    }

    #[test]
    fn positional_selection_uses_exact_rectangle_edges_zero_extents_and_wrapping_endpoints() {
        let mut world = world();
        let actor = member(&mut world, [100, 100]);
        for (bounds, point, selected) in [
            (
                IntRect {
                    x: 10,
                    y: 20,
                    width: 2,
                    height: 3,
                },
                [10, 20],
                true,
            ),
            (
                IntRect {
                    x: 10,
                    y: 20,
                    width: 2,
                    height: 3,
                },
                [12, 20],
                false,
            ),
            (
                IntRect {
                    x: 10,
                    y: 20,
                    width: 2,
                    height: 3,
                },
                [11, 23],
                false,
            ),
            (
                IntRect {
                    x: 10,
                    y: 20,
                    width: 0,
                    height: 3,
                },
                [10, 20],
                false,
            ),
            (
                IntRect {
                    x: 10,
                    y: 20,
                    width: 2,
                    height: 0,
                },
                [10, 20],
                false,
            ),
            (
                IntRect {
                    x: 10,
                    y: 20,
                    width: -1,
                    height: 3,
                },
                [10, 20],
                false,
            ),
            (
                IntRect {
                    x: 10,
                    y: 20,
                    width: 2,
                    height: -1,
                },
                [10, 20],
                false,
            ),
            (
                IntRect {
                    x: i32::MAX - 1,
                    y: 0,
                    width: 5,
                    height: 1,
                },
                [i32::MAX, 0],
                true,
            ),
            (
                IntRect {
                    x: i32::MAX - 1,
                    y: 0,
                    width: 5,
                    height: 1,
                },
                [i32::MIN, 0],
                false,
            ),
            (
                IntRect {
                    x: 0,
                    y: i32::MAX - 1,
                    width: 1,
                    height: 5,
                },
                [0, i32::MAX],
                true,
            ),
            (
                IntRect {
                    x: 0,
                    y: i32::MAX - 1,
                    width: 1,
                    height: 5,
                },
                [0, i32::MIN],
                false,
            ),
        ] {
            actor.borrow().state.bounds.set(bounds);
            let actual = world.find_organism_from_position(point).unwrap();
            assert_eq!(actual.is_some(), selected, "{bounds:?} {point:?}");
            if let Some(actual) = actual {
                assert!(Rc::ptr_eq(&actual, &actor));
            }
        }
    }

    #[test]
    fn kill_all_calls_real_death_for_live_members_in_order_and_skips_repeated_corpses() {
        let mut world = world();
        let first = member(&mut world, [100, 100]);
        let second = member(&mut world, [300, 100]);
        let corpse = member(&mut world, [500, 100]);
        world.die(&mut corpse.borrow_mut(), None).unwrap();
        first.borrow_mut().saved_genome = Some(Rc::clone(second.borrow().genome.as_ref().unwrap()));
        world.rebuild_spatial();
        let energies: Vec<_> = world.organisms.borrow().iter()
            .map(|actor| actor.borrow().state.energy.to_bits())
            .collect();
        let gases = (
            world.oxygen.get().to_bits(),
            world.carbon_dioxide.get().to_bits(),
            world.methane.get().to_bits(),
        );
        let random = world.random_snapshot();
        world.kill_all().unwrap();
        assert_eq!(world.population(), 0);
        assert_eq!((world.organisms.borrow().len(), world.frame_objects.borrow().len()), (3, 3));
        assert!(
            world.organisms.borrow().iter()
                .all(|actor| !actor.borrow().state.alive)
        );
        assert!(first.borrow().saved_genome.is_none());
        assert!(matches!(
            observed_field(&world.statistics_ref().borrow(), "deathSum"),
            Observation::Long(3)
        ));
        assert!(Rc::ptr_eq(
            world.statistics_ref().borrow().get_last_dead_being().unwrap(), second.borrow().genome.as_ref().unwrap()));
        assert_eq!(
            world.organisms.borrow().iter()
                .map(|actor| actor.borrow().state.energy.to_bits())
                .collect::<Vec<_>>(),
            energies
        );
        assert_eq!(
            (
                world.oxygen.get().to_bits(),
                world.carbon_dioxide.get().to_bits(),
                world.methane.get().to_bits()
            ),
            gases
        );
        assert_eq!(world.random_snapshot(), random);
        let before = world.clone();
        world.kill_all().unwrap();
        assert_eq!(world, before);
    }

    #[test]
    fn disperse_all_allocates_scarce_oxygen_in_order_and_frame_removes_only_depleted_corpses() {
        let mut world = world();
        let first = member(&mut world, [100, 100]);
        let second = member(&mut world, [300, 100]);
        world.kill_all().unwrap();
        first.borrow_mut().state.energy = 4.0;
        second.borrow_mut().state.energy = 9.0;
        world.oxygen.set(5.0);
        world.carbon_dioxide.set(20.0);
        world.methane.set(7.0);
        let statistics = world.clone().statistics_ref();
        world.rebuild_spatial();
        world.disperse_all().unwrap();
        assert_eq!(
            (first.borrow().state.energy, second.borrow().state.energy),
            (0.0, 8.0)
        );
        assert_eq!(
            (world.oxygen(), world.carbon_dioxide(), world.methane()),
            (0.0, 25.0, 7.0)
        );
        assert_eq!(
            (
                world.population(),
                world.organisms.borrow().len(),
                world.frame_objects.borrow().len()
            ),
            (0, 2, 2)
        );
        assert_eq!(world.statistics_ref(), statistics);
        assert!(world.single_step().unwrap());
        assert_eq!(world.organisms.borrow().len(), 1);
        assert!(Rc::ptr_eq(&world.organisms.borrow()[0], &second));
        assert_eq!(second.borrow().state.energy, 8.0);
        assert_eq!(world.frame_objects.borrow().len(), 2);
        assert!(Rc::ptr_eq(&world.frame_objects.borrow()[0], &first));
        assert_eq!(world.population(), 0);
    }

    #[test]
    fn disperse_all_keeps_live_members_and_processes_negative_and_signed_zero_corpse_energy() {
        let mut world = world();
        let negative = member(&mut world, [100, 100]);
        let zero = member(&mut world, [300, 100]);
        let live = member(&mut world, [500, 100]);
        world.die(&mut negative.borrow_mut(), None).unwrap();
        world.die(&mut zero.borrow_mut(), None).unwrap();
        negative.borrow_mut().state.energy = -2.0;
        zero.borrow_mut().state.energy = -0.0;
        live.borrow_mut().state.energy = 13.0;
        world.oxygen.set(3.0);
        world.carbon_dioxide.set(10.0);
        world.methane.set(7.0);
        let live_before = live.borrow().snapshot();
        let statistics = world.clone().statistics_ref();
        world.disperse_all().unwrap();
        assert_eq!(
            (world.oxygen(), world.carbon_dioxide(), world.methane()),
            (5.0, 8.0, 7.0)
        );
        assert_eq!(negative.borrow().state.energy.to_bits(), 0.0_f64.to_bits());
        assert_eq!(zero.borrow().state.energy.to_bits(), 0.0_f64.to_bits());
        assert_eq!(live.borrow().snapshot(), live_before);
        assert_eq!(world.statistics_ref(), statistics);
        assert_eq!((world.population(), world.organisms.borrow().len()), (1, 3));
    }
}
