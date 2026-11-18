// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use super::death::Killer;
use super::{Organism, OrganismRef, World, WorldError};
use crate::{ColorKind, TOLERANCE};

impl World {
    /// Dispatch every original touchEffects1 case using the current phenotype
    pub(super) fn touch_effects_one(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        match ColorKind::from_argb(actor.body.color_argb(segment)?) {
            Orange => self.touch_orange(actor, peer, segment, peer_segment)?,
            Fire => self.touch_fire(actor, peer, segment, peer_segment)?,
            Red => self.touch_red(actor, peer, segment, peer_segment)?,
            Pink => self.touch_pink(actor, peer, segment, peer_segment)?,
            Maroon => self.touch_maroon(actor, peer, segment, peer_segment)?,
            Cream => self.touch_cream(actor, peer, segment, peer_segment)?,
            Lilac => self.touch_lilac(actor, peer, segment, peer_segment)?,
            SpikePoint => self.touch_spikepoint(actor, peer, segment, peer_segment)?,
            Ochre => self.touch_ochre(actor, peer, segment, peer_segment)?,
            Silver => self.touch_silver(actor, peer, segment, peer_segment)?,
            Coral => self.coral_contact(actor_ref, actor, peer_ref, peer, peer_segment)?,
            _ => {
                if peer.state.alive && peer.state.energy < TOLERANCE {
                    self.die(peer, Some(Killer::Other(actor)))?;
                }
            }
        }
        Ok(())
    }

    /// Dispatch every original touchEffects2 case using the current phenotype
    pub(super) fn touch_effects_two(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        match ColorKind::from_argb(actor.body.color_argb(segment)?) {
            White => self.touch_white(actor, peer, peer_segment)?,
            Fruit => self.touch_fruit(actor, peer, peer_segment)?,
            Plague => self.touch_plague(actor_ref, actor, peer_ref, peer, segment, peer_segment)?,
            Fallow => self.touch_fallow(actor, peer, segment, peer_segment)?,
            Violet => self.touch_violet(actor, peer, segment, peer_segment)?,
            Sky => self.touch_sky(actor, peer, segment, peer_segment)?,
            Olive => self.touch_olive(actor, peer, segment, peer_segment)?,
            Gray => self.touch_gray(actor, peer, segment, peer_segment)?,
            Blue => self.touch_blue(actor, peer, peer_segment)?,
            Yellow => self.touch_yellow(actor_ref, actor, peer_ref, peer, segment, peer_segment)?,
            Rose => self.touch_rose(actor, peer, peer_segment)?,
            Mint => self.touch_mint(actor, peer, peer_segment)?,
            Lavender => self.touch_lavender(actor, peer, peer_segment)?,
            Magenta => self.touch_magenta(actor, peer, peer_segment)?,
            _ => {
                if peer.state.alive && peer.state.energy < TOLERANCE {
                    self.die(peer, Some(Killer::Other(actor)))?;
                }
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
    fn translucent_weapon_uses_both_source_defaults_and_still_checks_peer_death() {
        for dispatch in [World::touch_effects_one, World::touch_effects_two] {
            let mut world = World::new(WorldConfig {
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
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            actor.body.set_color_argb(0, 0x12ff0000).unwrap();
            actor.state.energy = 100.0;
            peer.state.energy = 1.0;
            let gases = [world.oxygen.get(), world.carbon_dioxide.get(), world.methane.get()];
            let random = world.random_snapshot();
            dispatch(
                &mut world, &actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0,
            )
            .unwrap();
            assert_eq!(actor.state.energy, 100.0);
            assert_eq!(peer.state.energy, 1.0);
            assert_eq!([world.oxygen.get(), world.carbon_dioxide.get(), world.methane.get()], gases);
            assert_eq!(world.random_snapshot(), random);
            assert!(peer.state.alive);
            peer.state.energy = 0.0;
            dispatch(
                &mut world, &actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0,
            )
            .unwrap();
            assert!(!peer.state.alive);
            assert_eq!(actor.state.total_kills, 1);
            assert_eq!(world.population.get(), 1);
        }
    }
}
