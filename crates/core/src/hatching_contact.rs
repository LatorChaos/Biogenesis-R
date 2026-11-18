// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::organism::primary_genome;

use super::{Organism, World, WorldError};
use crate::ColorKind;

fn classified_trigger(actor: &Organism, peer: &Organism, consumer_mode: i32) -> Result<bool, WorldError> {
    let target = if peer.effects.is_consumer {
        Some(consumer_mode)
    } else if peer.effects.is_fungus {
        Some(5)
    } else if peer.effects.is_plant {
        Some(if peer.effects.is_infectious { 6 } else { 7 })
    } else if peer.state.active {
        Some(0)
    } else {
        None
    };
    let Some(target) = target else { return Ok(false); };
    Ok(primary_genome(&actor.genome)?.borrow().modifiers().adapt_spore % 10 == target)
}

fn contact_triggers_hatching(actor: &Organism, peer: &Organism, peer_segment: usize) -> Result<bool, WorldError> {
    use ColorKind::*;
    let mode = || -> Result<i32, WorldError> {
        Ok(primary_genome(&actor.genome)?.borrow().modifiers().adapt_spore % 10)
    };
    let plant = peer.effects.is_plant && !peer.effects.is_consumer && !peer.effects.is_fungus;
    Ok(match ColorKind::from_argb(peer.body.color_argb(peer_segment)?) {
        Red => mode()? == 1,
        Fire => mode()? == 2,
        Orange | Maroon => mode()? == 3,
        Pink => mode()? == 5,
        Blue | LightBlue | Ochre | Fallow | Bark | OldBark | DeadBark | Ice => plant && mode()? == 9,
        Olive | DarkOlive | Sky | DeepSky | Violet => actor.effects.healing > 0 && plant && mode()? == 9,
        Spike | SpikePoint | Gray => plant && mode()? == 8,
        Lilac | DarkLilac => !primary_genome(&peer.genome)?.borrow().modifiers().modifies_lilac && plant && mode()? == 8,
        Silver => classified_trigger(actor, peer, 2)?,
        Coral => {
            if peer.effects.is_consumer || peer.effects.is_fungus || peer.effects.is_plant {
                classified_trigger(actor, peer, 4)?
            } else {
                peer.state.active && !actor.effects.is_infectious && !actor.effects.is_coral && mode()? == 0
            }
        }
        Fruit => {
            let spore = primary_genome(&peer.genome)?.borrow().modifiers().spore;
            mode()? == if spore == 1 { 0 } else { 6 }
        }
        Cream | Brown => actor.effects.spin == -1,
        _ => classified_trigger(actor, peer, 4)?,
    })
}

impl World {
    /// Apply source contact gates before invoking the actual resource-sensitive hatch method
    pub(super) fn touch_hatch(
        &self,
        actor: &mut Organism,
        peer: &Organism,
        peer_segment: usize,
    ) -> Result<(), WorldError> {
        if contact_triggers_hatching(actor, peer, peer_segment)? {
            self.hatch(actor)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gene, Genome, GenomeModifiers, GenomeProperties, WorldConfig};
    use std::{cell::RefCell, rc::Rc};

    fn organism(color: ColorKind, mode: i32) -> Organism {
        Organism::unplaced(
            Rc::new(RefCell::new(Genome::from_parts(
                &[Gene::new(12.0, 0.0, color.argb(), -1, [0; 36]).shared()],
                GenomeProperties {
                    symmetry: 1,
                    modifiers: GenomeModifiers {
                        adapt_spore: mode,
                        ..GenomeModifiers::default()
                    },
                    ..GenomeProperties::default()
                },
            ))),
            Default::default(),
        )
        .unwrap()
    }

    #[test]
    fn silver_and_coral_keep_consumer_before_fungus_before_plant_precedence() {
        let mut peer = organism(ColorKind::Silver, 0);
        peer.effects.is_consumer = true;
        peer.effects.is_fungus = true;
        peer.effects.is_plant = true;
        peer.effects.is_infectious = true;
        assert!(contact_triggers_hatching(
            &organism(ColorKind::Blue, 12),
            &peer,
            0
        ).unwrap());
        assert!(!contact_triggers_hatching(
            &organism(ColorKind::Blue, 15),
            &peer,
            0
        ).unwrap());
        peer.body.set_color_argb(0, ColorKind::Coral.argb().unwrap()).unwrap();
        assert!(contact_triggers_hatching(
            &organism(ColorKind::Blue, 14),
            &peer,
            0
        ).unwrap());
        assert!(!contact_triggers_hatching(
            &organism(ColorKind::Blue, 12),
            &peer,
            0
        ).unwrap());
    }

    #[test]
    fn protective_and_corpse_contacts_retain_distinct_healing_and_spin_gates() {
        let mut actor = organism(ColorKind::Blue, 19);
        let mut peer = organism(ColorKind::Sky, 0);
        peer.effects.is_plant = true;
        assert!(!contact_triggers_hatching(&actor, &peer, 0).unwrap());
        actor.effects.healing = 1;
        assert!(contact_triggers_hatching(&actor, &peer, 0).unwrap());
        peer.effects.is_consumer = true;
        assert!(!contact_triggers_hatching(&actor, &peer, 0).unwrap());
        peer.body.set_color_argb(0, ColorKind::Brown.argb().unwrap()).unwrap();
        actor.effects.spin = -1;
        assert!(contact_triggers_hatching(&actor, &peer, 0).unwrap());
        actor.effects.spin = 0;
        assert!(!contact_triggers_hatching(&actor, &peer, 0).unwrap());
    }

    #[test]
    fn signed_adaptation_and_contact_payment_flow_through_actual_hatching() {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        let peer = organism(ColorKind::Fire, 0);
        let mut actor = organism(ColorKind::Green, -2);
        actor.state.world = world.association();
        actor.state.active = false;
        actor.state.energy = 10.0;
        world.touch_hatch(&mut actor, &peer, 0).unwrap();
        assert!(!actor.state.active);
        let mut actor = organism(ColorKind::Green, 12);
        actor.state.world = world.association();
        actor.state.active = false;
        actor.state.energy = 10.0;
        world.touch_hatch(&mut actor, &peer, 0).unwrap();
        assert!(actor.state.active);
        assert_eq!(actor.state.age, 1);
        assert_eq!(actor.effects.spore_version, -2);
    }
}
