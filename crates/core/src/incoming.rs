// Biogenesis (C) 2006-2010 Joan Queralt Molina
// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use std::collections::{HashMap, VecDeque};

pub(crate) type IncomingCorridorRef = Rc<RefCell<IncomingCorridor>>;

/// Queue references and the one retained candidate remain authoritative on the simulation thread
#[derive(Debug, PartialEq)]
pub(crate) struct IncomingCorridor {
    pub bounds: IntRect,
    pub pending: VecDeque<Option<GenomeRef>>,
    pub candidate: Option<OrganismRef>,
}

impl IncomingCorridor {
    pub(crate) fn receive(&mut self, genome: Option<GenomeRef>) {
        self.pending.push_back(genome);
    }

    fn frame(&mut self, world: &World) -> Result<(), WorldError> {
        if self.candidate.is_none() {
            if let Some(Some(genome)) = self.pending.pop_front() {
                self.candidate = Some(Rc::new(RefCell::new(world.new_pasted_candidate(genome))));
            }
        }
        if let Some(candidate) = self.candidate.as_ref().map(Rc::clone) {
            let x = self
                .bounds
                .x
                .wrapping_add(coordinate(world, self.bounds.width, "width")?);
            let y = self
                .bounds
                .y
                .wrapping_add(coordinate(world, self.bounds.height, "height")?);
            let succeeds = {
                let mut actor = candidate.borrow_mut();
                world.attempt_paste(Some(&candidate), &mut actor, [x, y])?
            };
            if succeeds {
                world.add_organism(candidate, None)?;
                self.candidate = None;
            }
        }
        Ok(())
    }

    fn copy_for_world(&self, actors: &mut OrganismCloner) -> Self {
        Self {
            bounds: self.bounds,
            pending: self
                .pending
                .iter()
                .map(|genome| {
                    genome
                        .as_ref()
                        .map(|genome| actors.genome_cloner().copy(genome))
                })
                .collect(),
            candidate: self.candidate.as_ref().map(|actor| actors.copy(actor)),
        }
    }
}

fn coordinate(world: &World, bound: i32, axis: &'static str) -> Result<i32, WorldError> {
    if bound <= 0 {
        return Err(WorldError::InvalidCorridorDimension { axis, value: bound });
    }
    Ok(world.session.0.random.borrow_mut().next_bounded(bound)
        .expect("Positive Java corridor extent"))
}

pub(super) fn copy_list(
    original: &[IncomingCorridorRef],
    actors: &mut OrganismCloner,
    copied: &mut HashMap<*const RefCell<IncomingCorridor>, IncomingCorridorRef>,
) -> Vec<IncomingCorridorRef> {
    original
        .iter()
        .map(|corridor| {
            let identity = Rc::as_ptr(corridor);
            if let Some(existing) = copied.get(&identity) {
                return Rc::clone(existing);
            }
            let result = Rc::new(RefCell::new(corridor.borrow().copy_for_world(actors)));
            copied.insert(identity, Rc::clone(&result));
            result
        })
        .collect()
}

impl World {
    pub(super) fn corridor_bounds(&self) -> IntRect {
        let width = (self.config.get().width as i32).min(100);
        let height = (self.config.get().height as i32).min(100);
        let x_span = self.config.get().width as i32 - width;
        let y_span = self.config.get().height as i32 - height;
        // NET-015: zero placement span uses position zero without consuming randomness
        let x = if x_span == 0 {
            0
        } else {
            self.session.0.random.borrow_mut()
                .next_bounded(x_span)
                .expect("Positive placement span")
        };
        let y = if y_span == 0 {
            0
        } else {
            self.session.0.random.borrow_mut()
                .next_bounded(y_span)
                .expect("Positive placement span")
        };
        IntRect {
            x,
            y,
            width,
            height,
        }
    }

    pub(super) fn new_incoming_corridor(&self) -> IncomingCorridorRef {
        Rc::new(RefCell::new(IncomingCorridor {
            bounds: self.corridor_bounds(),
            pending: VecDeque::new(),
            candidate: None,
        }))
    }

    pub(super) fn add_incoming_corridor(&self, corridor: IncomingCorridorRef) {
        self.incoming_corridors.borrow_mut().push(corridor);
        self.corridors_exist.set(true);
    }

    pub(super) fn frame_incoming_corridor(
        &self,
        corridor: &IncomingCorridorRef,
    ) -> Result<(), WorldError> {
        corridor.borrow_mut().frame(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistics::tests::{Observation, observed_field};
    use crate::{ColorKind, Gene, GenomeProperties};

    fn world(width: i32, height: i32) -> World {
        World::new(WorldConfig {
            seed: 827364,
            width,
            height,
            initial_attempts: 0,
        })
        .unwrap()
    }

    fn genome(length: f64, symmetry: i32) -> GenomeRef {
        Rc::new(RefCell::new(Genome::from_parts(
            &[Gene::new(length, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
            GenomeProperties {
                symmetry,
                ..GenomeProperties::default()
            },
        )))
    }

    fn at(world: &mut World, bounds: IntRect) -> IncomingCorridorRef {
        let corridor = world.new_incoming_corridor();
        corridor.borrow_mut().bounds = bounds;
        corridor
    }

    #[test]
    fn zero_span_placement_is_usable_without_draws_and_positive_spans_keep_draw_order() {
        for size in [1, 99, 100] {
            let world = world(size, size);
            let before = world.random_snapshot();
            let corridor = world.new_incoming_corridor();
            assert_eq!(
                corridor.borrow().bounds,
                IntRect {
                    x: 0,
                    y: 0,
                    width: size as i32,
                    height: size as i32
                }
            );
            assert_eq!(world.random_snapshot(), before);
            assert!(!world.corridors_exist.get() && world.incoming_corridors.borrow().is_empty());
        }
        let world = world(101, 100);
        let mut expected = world.random_snapshot();
        expected.next_bounded(1).unwrap();
        let corridor = world.new_incoming_corridor();
        assert_eq!(
            corridor.borrow().bounds,
            IntRect {
                x: 0,
                y: 0,
                width: 100,
                height: 100
            }
        );
        assert_eq!(world.random_snapshot(), expected);
        let world = self::world(7000, 7000);
        let mut expected = world.random_snapshot();
        let position = [
            expected.next_bounded(6900).unwrap(),
            expected.next_bounded(6900).unwrap(),
        ];
        let corridor = world.new_incoming_corridor();
        assert_eq!(
            [corridor.borrow().bounds.x, corridor.borrow().bounds.y],
            position
        );
        assert_eq!(world.random_snapshot(), expected);
    }

    #[test]
    fn null_fifo_entry_consumes_one_frame_and_direct_success_does_not_register() {
        let mut world = world(7000, 7000);
        let corridor = at(
            &mut world,
            IntRect {
                x: 1000,
                y: 1000,
                width: 1,
                height: 1,
            },
        );
        let code = genome(8.0, 1);
        corridor.borrow_mut().receive(None);
        corridor.borrow_mut().receive(Some(Rc::clone(&code)));
        let mut expected = world.random_snapshot();
        world.frame_incoming_corridor(&corridor).unwrap();
        assert_eq!(world.random_snapshot(), expected);
        assert_eq!(corridor.borrow().pending.len(), 1);
        assert!(corridor.borrow().candidate.is_none() && world.organisms.borrow().is_empty());
        expected.next_f64();
        expected.next_bounded(1).unwrap();
        expected.next_bounded(1).unwrap();
        world.frame_incoming_corridor(&corridor).unwrap();
        assert_eq!(world.random_snapshot(), expected);
        assert!(corridor.borrow().pending.is_empty() && corridor.borrow().candidate.is_none());
        assert_eq!(world.organisms.borrow().len(), 1);
        assert_eq!(world.organisms.borrow()[0].borrow().state.age, 0);
        assert!(world.incoming_corridors.borrow().is_empty() && !world.corridors_exist.get());
        assert!(Rc::ptr_eq(world.organisms.borrow()[0].borrow().genome.as_ref().unwrap(),  &code));
    }

    #[test]
    fn invalid_extent_retains_constructor_and_only_prior_coordinate_draws() {
        let mut world = world(7000, 7000);
        let corridor = at(
            &mut world,
            IntRect {
                x: 1000,
                y: 1000,
                width: 0,
                height: 1,
            },
        );
        let code = genome(8.0, 1);
        corridor.borrow_mut().receive(Some(Rc::clone(&code)));
        let mut expected = world.random_snapshot();
        expected.next_f64();
        assert_eq!(
            world.frame_incoming_corridor(&corridor),
            Err(WorldError::InvalidCorridorDimension {
                axis: "width",
                value: 0
            })
        );
        assert_eq!(world.random_snapshot(), expected);
        let candidate = Rc::clone(corridor.borrow().candidate.as_ref().unwrap());
        assert!(!candidate.borrow().creation_complete);
        assert_eq!(code.borrow().generation(), 0);
        corridor.borrow_mut().bounds.width = 1;
        corridor.borrow_mut().bounds.height = 0;
        expected.next_bounded(1).unwrap();
        assert_eq!(
            world.frame_incoming_corridor(&corridor),
            Err(WorldError::InvalidCorridorDimension {
                axis: "height",
                value: 0
            })
        );
        assert_eq!(world.random_snapshot(), expected);
        assert!(Rc::ptr_eq(
            corridor.borrow().candidate.as_ref().unwrap(),
            &candidate
        ));
        assert_eq!(candidate.borrow().state.parent_id, 0);
        assert!(!candidate.borrow().creation_complete && world.organisms.borrow().is_empty());
    }

    #[test]
    fn blocked_candidate_keeps_identity_orientation_and_fifo_until_live_blocker_moves() {
        let mut world = world(7000, 7000);
        let blocker = world
            .construct_pasted_shared(genome(2000.0, 4), [1050, 1050])
            .unwrap()
            .unwrap();
        world.add_organism(Rc::new(RefCell::new(blocker)), None).unwrap();
        let corridor = at(
            &mut world,
            IntRect {
                x: 1000,
                y: 1000,
                width: 100,
                height: 100,
            },
        );
        let code = genome(8.0, 1);
        corridor.borrow_mut().receive(Some(Rc::clone(&code)));
        corridor.borrow_mut().receive(Some(genome(8.0, 1)));
        world.frame_incoming_corridor(&corridor).unwrap();
        let candidate = Rc::clone(corridor.borrow().candidate.as_ref().unwrap());
        let angle = candidate.borrow().state.angle.to_bits();
        assert_eq!(corridor.borrow().pending.len(), 1);
        world.frame_incoming_corridor(&corridor).unwrap();
        assert!(Rc::ptr_eq(
            corridor.borrow().candidate.as_ref().unwrap(),
            &candidate
        ));
        assert_eq!(candidate.borrow().state.angle.to_bits(), angle);
        assert_eq!(corridor.borrow().pending.len(), 1);
        {
            let blocker_ref = Rc::clone(&world.organisms.borrow()[0]);
            let mut blocker = blocker_ref.borrow_mut();
            crate::movement::offset(&mut blocker, [3000.0, 3000.0], 0.0);
            crate::movement::calculate_bounds(&mut blocker, true).unwrap();
        }
        world.frame_incoming_corridor(&corridor).unwrap();
        assert!(corridor.borrow().candidate.is_none());
        assert_eq!(corridor.borrow().pending.len(), 1);
        assert!(Rc::ptr_eq(&world.organisms.borrow()[1], &candidate));
        assert!(Rc::ptr_eq(candidate.borrow().genome.as_ref().unwrap(),  &code));
        assert_eq!(candidate.borrow().state.angle.to_bits(), angle);
    }

    #[test]
    fn incoming_error_clears_old_frame_tree_before_stopping_motion_gases_and_clock() {
        let mut world = world(7000, 7000);
        let actor = world
            .construct_pasted_shared(genome(8.0, 1), [3000, 3000])
            .unwrap()
            .unwrap();
        world.add_organism(Rc::new(RefCell::new(actor)), None).unwrap();
        world.rebuild_spatial();
        assert_eq!(world.frame_objects.borrow().len(), 1);
        let corridor = at(
            &mut world,
            IntRect {
                x: 1000,
                y: 1000,
                width: 0,
                height: 1,
            },
        );
        corridor.borrow_mut().receive(Some(genome(8.0, 1)));
        world.add_incoming_corridor(Rc::clone(&corridor));
        let gases = [
            world.oxygen.get().to_bits(),
            world.carbon_dioxide.get().to_bits(),
            world.methane.get().to_bits(),
        ];
        assert_eq!(
            world.single_step(),
            Err(WorldError::InvalidCorridorDimension {
                axis: "width",
                value: 0
            })
        );
        assert!(world.frame_objects.borrow().is_empty() && world.spatial.borrow().stored_envelopes().is_empty());
        assert_eq!(world.organisms.borrow()[0].borrow().state.age, 0);
        assert_eq!(
            [
                world.oxygen.get().to_bits(),
                world.carbon_dioxide.get().to_bits(),
                world.methane.get().to_bits()
            ],
            gases
        );
        assert_eq!((world.frame.get(), world.tick.get()), (0, 0));
        assert!(corridor.borrow().candidate.is_some());
    }

    #[test]
    fn ordered_corridors_use_live_earlier_arrivals_and_move_successes_in_the_same_frame() {
        let mut world = world(7000, 7000);
        let first = at(
            &mut world,
            IntRect {
                x: 1000,
                y: 1000,
                width: 1,
                height: 1,
            },
        );
        let second = at(&mut world, first.borrow().bounds);
        let code = genome(8.0, 1);
        first.borrow_mut().receive(Some(Rc::clone(&code)));
        second.borrow_mut().receive(Some(Rc::clone(&code)));
        world.add_incoming_corridor(Rc::clone(&first));
        world.add_incoming_corridor(Rc::clone(&second));
        assert!(world.single_step().unwrap());
        assert!(first.borrow().candidate.is_none() && second.borrow().candidate.is_some());
        assert_eq!(
            (world.organisms.borrow().len(), world.population.get(), world.next_id.get()),
            (1, 1, 1)
        );
        assert_eq!(world.organisms.borrow()[0].borrow().state.age, 1);
        assert!(Rc::ptr_eq(&world.frame_objects.borrow()[0], &world.organisms.borrow()[0]));
        assert!(Rc::ptr_eq(world.organisms.borrow()[0].borrow().genome.as_ref().unwrap(), second.borrow().candidate.as_ref().unwrap().borrow().genome.as_ref().unwrap()));
        assert_eq!(code.borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("0"));
    }

    #[test]
    fn duplicate_registered_occurrences_each_process_one_queue_entry() {
        let mut world = world(7000, 7000);
        let corridor = at(
            &mut world,
            IntRect {
                x: 1000,
                y: 1000,
                width: 1,
                height: 1,
            },
        );
        corridor.borrow_mut().receive(None);
        corridor.borrow_mut().receive(Some(genome(8.0, 1)));
        world.add_incoming_corridor(Rc::clone(&corridor));
        world.add_incoming_corridor(Rc::clone(&corridor));
        assert!(world.single_step().unwrap());
        assert_eq!(world.organisms.borrow().len(), 1);
        assert_eq!(world.organisms.borrow()[0].borrow().state.age, 1);
        assert!(corridor.borrow().pending.is_empty() && corridor.borrow().candidate.is_none());
    }

    #[test]
    fn world_clone_preserves_corridor_actor_genotype_aliases_and_detaches_every_owner() {
        let world = world(7000, 7000);
        let code = genome(8.0, 1);
        let actor = world
            .construct_pasted_shared(Rc::clone(&code), [1000, 1000])
            .unwrap()
            .unwrap();
        let actor = Rc::new(RefCell::new(actor));
        world.add_organism(Rc::clone(&actor), None).unwrap();
        world.die(&mut actor.borrow_mut(), None).unwrap();
        world.rebuild_spatial();
        let first = world.new_incoming_corridor();
        first.borrow_mut().candidate = Some(Rc::clone(&actor));
        first.borrow_mut().receive(Some(Rc::clone(&code)));
        first.borrow_mut().receive(None);
        first.borrow_mut().receive(Some(Rc::clone(&code)));
        let second = world.new_incoming_corridor();
        second.borrow_mut().candidate = Some(Rc::new(RefCell::new(
            world.new_pasted_candidate(Rc::clone(&code)),
        )));
        world.add_incoming_corridor(Rc::clone(&first));
        world.add_incoming_corridor(Rc::clone(&first));
        world.add_incoming_corridor(Rc::clone(&second));
        let cloned = world.clone();
        assert!(cloned.corridors_exist.get());
        assert!(Rc::ptr_eq(
            &cloned.incoming_corridors.borrow()[0],
            &cloned.incoming_corridors.borrow()[1]
        ));
        assert!(!Rc::ptr_eq(&cloned.incoming_corridors.borrow()[0], &first));
        assert!(!Rc::ptr_eq(&cloned.organisms.borrow()[0], &actor));
        assert!(Rc::ptr_eq(&cloned.organisms.borrow()[0], &cloned.frame_objects.borrow()[0]));
        let corridor_ref = Rc::clone(&cloned.incoming_corridors.borrow()[0]);
        let corridor = corridor_ref.borrow();
        assert!(Rc::ptr_eq(
            corridor.candidate.as_ref().unwrap(),
            &cloned.organisms.borrow()[0]
        ));
        let cloned_code = Rc::clone(cloned.organisms.borrow()[0].borrow().genome.as_ref().unwrap());
        assert!(Rc::ptr_eq(
            corridor.pending[0].as_ref().unwrap(),
            &cloned_code
        ));
        assert!(corridor.pending[1].is_none());
        assert!(Rc::ptr_eq(
            corridor.pending[2].as_ref().unwrap(),
            &cloned_code
        ));
        let uncreated_ref = Rc::clone(&cloned.incoming_corridors.borrow()[2]);
        let uncreated = uncreated_ref.borrow();
        assert!(
            !uncreated
                .candidate
                .as_ref()
                .unwrap()
                .borrow()
                .creation_complete
        );
        assert!(Rc::ptr_eq(uncreated.candidate.as_ref().unwrap().borrow().genome.as_ref().unwrap(), 
            &cloned_code
        ));
        match observed_field(&cloned.statistics_ref().borrow(), "lastDeadBeing") {
            Observation::Genome(Some(record)) => assert!(Rc::ptr_eq(record, &cloned_code)),
            _ => panic!("Missing retained statistics genotype"),
        }
        cloned_code.borrow().genes().unwrap()[0]
            .borrow_mut()
            .set_length(99.0);
        assert_eq!(code.borrow().gene_ref(0).unwrap().borrow().length(), 8.0);
        assert_eq!(world.random_snapshot(), cloned.random_snapshot());
    }

    #[test]
    fn genesis_preserves_corridors_queues_and_sticky_flag_while_reset_reconstructs() {
        let mut world = world(7000, 7000);
        let corridor = world.new_incoming_corridor();
        corridor.borrow_mut().receive(Some(genome(8.0, 1)));
        let bounds = corridor.borrow().bounds;
        world.add_incoming_corridor(Rc::clone(&corridor));
        world.config.get().width = 100;
        world.config.get().height = 100;
        world.genesis().unwrap();
        assert!(world.corridors_exist.get() && Rc::ptr_eq(&world.incoming_corridors.borrow()[0], &corridor));
        assert_eq!(corridor.borrow().bounds, bounds);
        assert_eq!(corridor.borrow().pending.len(), 1);
        world.incoming_corridors.borrow_mut().clear();
        assert!(world.corridors_exist.get());
        world.reset().unwrap();
        assert!(!world.corridors_exist.get() && world.incoming_corridors.borrow().is_empty());
    }
}
