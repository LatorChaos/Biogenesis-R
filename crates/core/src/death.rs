// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use super::{Organism, World};
use crate::ColorKind;

/// Preserve a self-killer without constructing aliasing mutable references
pub(super) enum Killer<'a> {
    Victim,
    Other(&'a mut Organism),
}

impl World {
    /// Apply die synchronously, including repeated calls on an existing corpse
    pub(super) fn die(&self, victim: &mut Organism, killer: Option<Killer<'_>>) -> Result<(), crate::WorldError> {
        victim.state.active = false;
        victim.state.alive = false;
        victim.effects.use_pre_touch_effects = false;
        victim.effects.use_friend_effects = 0;
        victim.effects.photosynthesis = 0.0;
        victim.effects.use_frame_movement = false;
        victim.effects.can_move = false;
        victim.effects.can_react = false;
        victim.state.has_moved = true;
        victim.infected_genome = None;
        victim.saved_genome = None;
        for index in victim.body.indices() {
            let kind = if victim.body.color_argb(index)? == ColorKind::Eye.argb().unwrap() {
                ColorKind::Eye
            } else {
                ColorKind::Brown
            };
            victim.body.set_color(index, kind.color_ref())?;
        }
        victim.state.world.resolve()?.decrease_population();
        match killer {
            None => victim.state.world.resolve()?.statistics_ref()
                .borrow_mut()
                .event_organism_die(victim, None),
            Some(Killer::Victim) => {
                victim.state.total_kills = victim.state.total_kills.wrapping_add(1);
                let owner = victim.state.world.resolve()?;
                owner.statistics_ref().borrow_mut().event_organism_die(victim, Some(victim));
                owner.notify_kills(victim);
            }
            Some(Killer::Other(killer)) => {
                killer.state.total_kills = killer.state.total_kills.wrapping_add(1);
                let owner = victim.state.world.resolve()?;
                owner.statistics_ref().borrow_mut().event_organism_die(victim, Some(killer));
                owner.notify_kills(killer);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gene, Genome, GenomeProperties, WorldConfig};
    use std::rc::Rc;

    #[test]
    fn death_preserves_energy_and_cached_effects_but_repeats_events_and_killer_updates() {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        let code = Genome::from_parts(
            &[
                Gene::new(12.0, 0.0, ColorKind::Eye.argb(), 0, [0; 36]).shared(),
                Gene::new(12.0, 0.0, Some(0x00004040), 0, [0; 36]).shared(),
            ],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        world.paste_genome(code, [100, 100]).unwrap().unwrap();
        let victim = Rc::clone(&world.organisms.borrow()[0]);
        let mut victim = victim.borrow_mut();
        victim.infected_genome = Some(Rc::clone(victim.genome.as_ref().unwrap()));
        victim.saved_genome = Some(Rc::clone(victim.genome.as_ref().unwrap()));
        victim.effects.photosynthesis = 3.0;
        victim.effects.methanotrophy = 7.0;
        victim.effects.use_extra_effects = true;
        victim.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![1.0, 2.0]));
        victim.state.temporary_color = (ColorKind::Red.argb()).map(crate::JavaColor::from_argb);
        victim.state.temporary_color_frames = 6;
        let energy = victim.state.energy;
        world.die(&mut victim, Some(Killer::Victim)).unwrap();
        assert!(!victim.state.active && !victim.state.alive);
        assert!(victim.state.has_moved);
        assert!(victim.infected_genome.is_none() && victim.saved_genome.is_none());
        assert_eq!(victim.state.energy, energy);
        assert_eq!(victim.effects.photosynthesis, 0.0);
        assert_eq!(victim.effects.methanotrophy, 7.0);
        assert!(victim.effects.use_extra_effects);
        assert_eq!(victim.body.photo_values().unwrap(), [1.0, 2.0]);
        assert_eq!(victim.state.temporary_argb(), ColorKind::Red.argb());
        assert_eq!(victim.state.temporary_color_frames, 6);
        assert_eq!(victim.body.color_argb(0).unwrap(), ColorKind::Eye.argb().unwrap());
        assert_eq!(
            victim.body.color_argb(1).unwrap(),
            ColorKind::Brown.argb().unwrap()
        );
        assert_eq!(world.statistics_ref().borrow().get_being_most_kills_number(), 1);
        assert!(Rc::ptr_eq(
            world.statistics_ref().borrow().get_last_dead_being().unwrap(), victim.genome.as_ref().unwrap()));
        world.die(&mut victim, None).unwrap();
        assert_eq!(world.population(), -1);
        assert_eq!(world.corpse_count().unwrap(), 2);
        assert_eq!(victim.state.total_kills, 1);
        world.statistics_ref().borrow_mut().event_time(-1, 0.0, 0.0, 0.0).unwrap();
        assert_eq!(crate::statistics::tests::samples(world.statistics_ref().borrow().get_death_list()), [2.0]);
    }
}
