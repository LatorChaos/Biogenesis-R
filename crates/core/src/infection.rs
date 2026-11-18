// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use super::{GenomeRef, Organism, World, WorldError};

pub(super) enum Infector<'a> {
    Recipient,
    Other(&'a mut Organism),
}

#[derive(Clone, Copy)]
pub(super) enum InfectionKind {
    Direct,
    Fruit,
    WeakFruit,
}

impl World {
    /// Preserve code-only notification, including a null code and the unchanged energy threshold
    pub(super) fn infect_with_code(&self, recipient: &mut Organism, code: Option<GenomeRef>) -> Result<(), WorldError> {
        recipient.infected_genome = code;
        recipient.saved_genome = None;
        recipient.state.world.resolve()?.statistics_ref()
            .borrow_mut()
            .event_organism_infects(recipient, None);
        Ok(())
    }

    /// Notify after assigning the shared code, then copy or calculate the infection threshold
    pub(super) fn infect_from(
        &self,
        recipient: &mut Organism,
        mut infector: Infector<'_>,
        kind: InfectionKind,
    ) -> Result<(), WorldError> {
        let code = match &mut infector {
            Infector::Recipient => {
                recipient.state.total_infected = recipient.state.total_infected.wrapping_add(1);
                recipient.genome.clone()
            }
            Infector::Other(donor) => {
                donor.state.total_infected = donor.state.total_infected.wrapping_add(1);
                donor.genome.clone()
            }
        };
        recipient.infected_genome = code;
        recipient.saved_genome = None;
        let owner = recipient.state.world.resolve()?;
        match &infector {
            Infector::Recipient => {
                owner.statistics_ref().borrow_mut().event_organism_infects(recipient, Some(recipient));
                owner.notify_infections(recipient);
            }
            Infector::Other(donor) => {
                owner.statistics_ref().borrow_mut().event_organism_infects(recipient, Some(donor));
                owner.notify_infections(donor);
            }
        }
        let (energy, spore_time) = match &infector {
            Infector::Recipient => (recipient.state.reproduction.energy, recipient.effects.spore_time),
            Infector::Other(donor) => (donor.state.reproduction.energy, donor.effects.spore_time),
        };
        recipient.state.reproduction.infected_energy = match kind {
            InfectionKind::Direct => energy,
            InfectionKind::Fruit | InfectionKind::WeakFruit => {
                let base = if recipient.effects.reproduce_late == 0 {
                    recipient.state.reproduction.energy
                } else {
                    40_i32.wrapping_add(3_i32.wrapping_mul(recipient.body.segment_count))
                };
                let additional = f64::from(base) * 0.048 * f64::from(spore_time);
                match kind {
                    InfectionKind::Fruit => (f64::from(base) + additional) as i32,
                    InfectionKind::WeakFruit => additional as i32,
                    InfectionKind::Direct => unreachable!(),
                }
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use crate::{ColorKind, Gene, Genome, GenomeProperties, WorldConfig};

    fn world_and_actors() -> (World, super::super::OrganismRef, super::super::OrganismRef) {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        let code = Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), -1, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        for center in [[100, 100], [300, 300]] {
            world.paste_genome(code.clone(), center).unwrap().unwrap();
        }
        let recipient = Rc::clone(&world.organisms.borrow()[0]);
        let donor = Rc::clone(&world.organisms.borrow()[1]);
        (world, recipient, donor)
    }

    #[test]
    fn infection_retains_code_identity_wraps_donor_count_and_records_each_notification() {
        let (world, recipient, donor) = world_and_actors();
        let mut recipient = recipient.borrow_mut();
        let mut donor = donor.borrow_mut();
        recipient.saved_genome = recipient.genome.clone();
        donor.state.total_infected = i32::MAX;
        donor.state.reproduction.energy = 119;
        world.infect_from(
            &mut recipient,
            Infector::Other(&mut donor),
            InfectionKind::Direct,
        ).unwrap();
        assert_eq!(donor.state.total_infected, i32::MIN);
        assert_eq!(recipient.state.reproduction.infected_energy, 119);
        assert!(recipient.saved_genome.is_none());
        assert!(Rc::ptr_eq(
            recipient.infected_genome.as_ref().unwrap(), donor.genome.as_ref().unwrap()));
        assert!(Rc::ptr_eq(
            world.statistics_ref().borrow().get_last_infected_being().unwrap(), recipient.genome.as_ref().unwrap()));
        recipient.state.reproduction.energy = 73;
        world.infect_from(&mut recipient, Infector::Recipient, InfectionKind::Direct).unwrap();
        assert_eq!(recipient.state.total_infected, 1);
        assert_eq!(recipient.state.reproduction.infected_energy, 73);
        assert!(Rc::ptr_eq(
            recipient.infected_genome.as_ref().unwrap(), recipient.genome.as_ref().unwrap()));
        world.infect_with_code(&mut recipient, None).unwrap();
        assert!(recipient.infected_genome.is_none());
        assert_eq!(recipient.state.reproduction.infected_energy, 73);
        world.statistics_ref().borrow_mut().event_time(2, 0.0, 0.0, 0.0).unwrap();
        assert_eq!(world.statistics_ref().borrow().get_average_infections(), 3.0);
    }

    #[test]
    fn fruit_thresholds_use_recipient_energy_or_segment_base_and_java_saturating_casts() {
        for (late, energy, spore_time, fruit, weak) in [
            (0, 100, 10, 148, 48),
            (1, 100, 10, 63, 20),
            (-1, 100, -10, 22, -20),
            (0, 100, -10, 52, -48),
            (0, i32::MAX, i32::MAX, i32::MAX, i32::MAX),
            (0, i32::MAX, i32::MIN, i32::MIN, i32::MIN),
        ] {
            let (world, recipient, donor) = world_and_actors();
            let mut recipient = recipient.borrow_mut();
            let mut donor = donor.borrow_mut();
            recipient.effects.reproduce_late = late;
            recipient.state.reproduction.energy = energy;
            donor.effects.spore_time = spore_time;
            world.infect_from(
                &mut recipient,
                Infector::Other(&mut donor),
                InfectionKind::Fruit,
            ).unwrap();
            assert_eq!(recipient.state.reproduction.infected_energy, fruit);
            world.infect_from(
                &mut recipient,
                Infector::Other(&mut donor),
                InfectionKind::WeakFruit,
            ).unwrap();
            assert_eq!(recipient.state.reproduction.infected_energy, weak);
        }
    }

    fn foreign_notification_worlds() -> (World, World, World, super::super::OrganismRef, super::super::OrganismRef) {
        let (caller, recipient, donor) = world_and_actors();
        let config = WorldConfig { initial_attempts: 0, ..WorldConfig::default() };
        let owner = World::with_session(config, caller.session()).unwrap();
        let donor_owner = World::with_session(config, caller.session()).unwrap();
        recipient.borrow_mut().state.world = owner.association();
        donor.borrow_mut().state.world = donor_owner.association();
        (caller, owner, donor_owner, recipient, donor)
    }

    #[test]
    fn foreign_infection_notifications_keep_recipient_owner_and_actual_genome_aliases() {
        for (kind, threshold) in [(InfectionKind::Direct, 119), (InfectionKind::Fruit, 148), (InfectionKind::WeakFruit, 48)] {
            let (caller, owner, donor_owner, recipient_ref, donor_ref) = foreign_notification_worlds();
            let random = caller.random_snapshot();
            let mut recipient = recipient_ref.borrow_mut();
            let mut donor = donor_ref.borrow_mut();
            recipient.saved_genome = recipient.genome.clone();
            recipient.state.reproduction.energy = 100;
            recipient.effects.reproduce_late = 0;
            donor.state.total_infected = 4;
            donor.state.reproduction.energy = 119;
            donor.effects.spore_time = 10;
            caller.infect_from(&mut recipient, Infector::Other(&mut donor), kind).unwrap();
            assert_eq!(donor.state.total_infected, 5);
            assert_eq!(recipient.state.reproduction.infected_energy, threshold);
            assert!(recipient.saved_genome.is_none());
            assert!(Rc::ptr_eq(recipient.infected_genome.as_ref().unwrap(), donor.genome.as_ref().unwrap()));
            assert!(Rc::ptr_eq(owner.statistics_ref().borrow().get_last_infected_being().unwrap(), recipient.genome.as_ref().unwrap()));
            assert!(Rc::ptr_eq(owner.statistics_ref().borrow().get_being_most_infections().unwrap(), donor.genome.as_ref().unwrap()));
            assert_eq!(owner.statistics_ref().borrow().get_being_most_infections_number(), 5);
            assert!(caller.statistics_ref().borrow().get_last_infected_being().is_none());
            assert!(donor_owner.statistics_ref().borrow().get_last_infected_being().is_none());
            assert_eq!(caller.population(), 2);
            assert_eq!(owner.population(), 0);
            assert_eq!(donor_owner.population(), 0);
            assert_eq!(caller.random_snapshot(), random);
        }
        let (caller, owner, _donor_owner, recipient_ref, _donor_ref) = foreign_notification_worlds();
        let random = caller.random_snapshot();
        let mut recipient = recipient_ref.borrow_mut();
        recipient.saved_genome = recipient.genome.clone();
        recipient.state.reproduction.infected_energy = 77;
        caller.infect_with_code(&mut recipient, None).unwrap();
        assert!(recipient.infected_genome.is_none() && recipient.saved_genome.is_none());
        assert_eq!(recipient.state.reproduction.infected_energy, 77);
        recipient.state.reproduction.energy = 73;
        caller.infect_from(&mut recipient, Infector::Recipient, InfectionKind::Direct).unwrap();
        assert_eq!(recipient.state.total_infected, 1);
        assert_eq!(recipient.state.reproduction.infected_energy, 73);
        assert!(Rc::ptr_eq(recipient.infected_genome.as_ref().unwrap(), recipient.genome.as_ref().unwrap()));
        assert!(Rc::ptr_eq(owner.statistics_ref().borrow().get_last_infected_being().unwrap(), recipient.genome.as_ref().unwrap()));
        assert!(caller.statistics_ref().borrow().get_last_infected_being().is_none());
        assert_eq!(caller.random_snapshot(), random);
    }

    #[test]
    fn infection_owner_failure_keeps_counter_reference_prefix_and_prior_threshold() {
        for expired in [false, true] {
            for kind in [InfectionKind::Direct, InfectionKind::Fruit, InfectionKind::WeakFruit] {
                let (caller, owner, donor_owner, recipient_ref, donor_ref) = foreign_notification_worlds();
                let random = caller.random_snapshot();
                let mut recipient = recipient_ref.borrow_mut();
                let mut donor = donor_ref.borrow_mut();
                recipient.state.world = if expired {
                    let temporary = World::with_session(
                        WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, caller.session(),
                    ).unwrap();
                    temporary.association()
                } else { Default::default() };
                recipient.saved_genome = recipient.genome.clone();
                recipient.state.reproduction.infected_energy = 77;
                donor.state.total_infected = i32::MAX;
                assert_eq!(caller.infect_from(&mut recipient, Infector::Other(&mut donor), kind),
                    Err(if expired { WorldError::ExpiredWorld } else { WorldError::MissingWorld }));
                assert_eq!(donor.state.total_infected, i32::MIN);
                assert!(Rc::ptr_eq(recipient.infected_genome.as_ref().unwrap(), donor.genome.as_ref().unwrap()));
                assert!(recipient.saved_genome.is_none());
                assert_eq!(recipient.state.reproduction.infected_energy, 77);
                assert!(caller.statistics_ref().borrow().get_last_infected_being().is_none());
                assert!(owner.statistics_ref().borrow().get_last_infected_being().is_none());
                assert!(donor_owner.statistics_ref().borrow().get_last_infected_being().is_none());
                assert_eq!(caller.random_snapshot(), random);
            }
            let (caller, owner, _donor_owner, recipient_ref, _donor_ref) = foreign_notification_worlds();
            let mut recipient = recipient_ref.borrow_mut();
            recipient.state.world = if expired {
                let temporary = World::with_session(
                    WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, caller.session(),
                ).unwrap();
                temporary.association()
            } else { Default::default() };
            recipient.saved_genome = recipient.genome.clone();
            recipient.state.reproduction.infected_energy = 77;
            assert_eq!(caller.infect_with_code(&mut recipient, None),
                Err(if expired { WorldError::ExpiredWorld } else { WorldError::MissingWorld }));
            assert!(recipient.infected_genome.is_none() && recipient.saved_genome.is_none());
            assert_eq!(recipient.state.reproduction.infected_energy, 77);
            assert!(owner.statistics_ref().borrow().get_last_infected_being().is_none());
        }
        let (caller, owner, _donor_owner, recipient_ref, _donor_ref) = foreign_notification_worlds();
        let mut recipient = recipient_ref.borrow_mut();
        recipient.state.world = Default::default();
        recipient.state.total_infected = i32::MAX;
        recipient.state.reproduction.infected_energy = 77;
        recipient.saved_genome = recipient.genome.clone();
        recipient.genome = None;
        assert_eq!(caller.infect_from(&mut recipient, Infector::Recipient, InfectionKind::Direct), Err(WorldError::MissingWorld));
        assert_eq!(recipient.state.total_infected, i32::MIN);
        assert!(recipient.infected_genome.is_none() && recipient.saved_genome.is_none());
        assert_eq!(recipient.state.reproduction.infected_energy, 77);
        assert!(owner.statistics_ref().borrow().get_last_infected_being().is_none());
    }
}
