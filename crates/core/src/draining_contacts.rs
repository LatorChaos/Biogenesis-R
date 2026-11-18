// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Complete ORANGE, FIRE, RED, PINK, and MAROON branches from Organism.java:8692-10526
//! Each named branch includes the shared peer death guard at Organism.java:13105-13108

use crate::organism::primary_genome;

use super::death::Killer;
use super::infection::{InfectionKind, Infector};
use super::{Organism, World};
use crate::{ColorKind, GROWTH_SCALES, TOLERANCE, physics::between};
use crate::organism::same_genome;

#[derive(Clone, Copy)]
enum Byproduct {
    Respiration,
    Decomposition,
}

fn color(organism: &Organism, segment: usize) -> Result<ColorKind, crate::WorldError> {
    Ok(organism.body.color_kind(segment)?)
}

fn tint(organism: &mut Organism, color: ColorKind) {
    organism.set_display_color(Some(color.color_ref().expect("Tint requires a named source literal")));
}

fn classified_orange_prey(peer: &Organism) -> bool {
    peer.effects.is_plant
        || peer.effects.is_consumer
        || peer.effects.is_fungus
        || peer.effects.plague_version > 0
        || peer.effects.is_auburn
}

fn experienced_silver(peer: &Organism) -> bool {
    (peer.state.total_kills > 0 && (peer.effects.is_consumer || peer.effects.is_fungus))
        || peer.effects.is_enhanced
}

fn full_fire_prey(actor: &Organism, peer: &Organism) -> bool {
    peer.effects.is_consumer
        || (actor.effects.sky_version > 0 && peer.effects.is_frozen && peer.effects.is_plant)
}

fn infected_by_peer(actor: &Organism, peer: &Organism) -> bool {
    same_genome(actor.infected_genome.as_ref(), peer.genome.as_ref())
}

pub(super) fn age_bark(peer: &mut Organism, segment: usize) -> Result<(), crate::WorldError> {
    peer.body.set_color(segment, ColorKind::OldBark.color_ref())?;
    if peer.state.active {
        peer.effects.photosynthesis -= peer.body.photo(segment)?
            * GROWTH_SCALES[crate::geometry::growth_index(peer.body.growth_ratio)?];
    }
    Ok(())
    }

impl World {
    fn drain_amount(&self, actor: &Organism, peer: &Organism, segment: usize, factor: f64) -> Result<f64, crate::WorldError> {
        let length = actor.body.physical_length(segment)?.sqrt();
        // The unit coefficient is absent from the original arithmetic expression
        let length = if factor == 1.0 {
            length
        } else {
            factor * length
        };
        Ok(between(
            length * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
            0.0,
            peer.state.energy,
        ))
    }

    fn paid_drain(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        cost: f64,
        factor: f64,
        actor_tint: ColorKind,
        peer_tint: ColorKind,
    ) -> Result<f64, crate::WorldError> {
        if actor.use_energy(cost)? {
            let amount = self.drain_amount(actor, peer, segment, factor)?;
            tint(peer, peer_tint);
            tint(actor, actor_tint);
            Ok(amount)
        } else {
            Ok(0.0)
        }
    }

    fn drain_dodge(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        actor_tint: ColorKind,
    ) -> Result<bool, crate::WorldError> {
        Ok({
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            tint(peer, ColorKind::Teal);
            tint(actor, actor_tint);
            true
        } else {
            false
        }
    
        })
    }

    fn finish_drain(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        amount: f64,
        byproduct: Byproduct,
    ) -> Result<(), crate::WorldError> {
        if amount > 0.0 {
            peer.state.energy -= amount;
            actor.state.energy += amount;
            self.notify_feeding(actor, peer, amount);
            let waste = amount * self.session.0.parameters.borrow().lifecycle.organic_subs_produced;
            match byproduct {
                Byproduct::Respiration => {
                    actor.use_energy(waste)?;
                }
                Byproduct::Decomposition => {
                    self.use_decay_energy(actor, waste)?;
                }
            }
        }
        if peer.state.alive && peer.state.energy < TOLERANCE {
            self.die(peer, Some(Killer::Other(actor)))?;
        }
        Ok(())
    }

    pub(super) fn touch_orange(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let mut taken = 0.0;
        let cost = self.session.0.parameters.borrow().lifecycle.orange_energy_consumption;
        peer.effects.has_dodged = true;
        match color(peer, peer_segment)? {
            White => {
                if classified_orange_prey(peer) {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Yellow)?;
                }
            }
            Violet => {
                let (factor, target) = if classified_orange_prey(peer) {
                    (1.0, Yellow)
                } else {
                    (0.5, GreenBrown)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, Orange, target)?;
            }
            Grass => {
                if !self.drain_dodge(actor, peer, Orange)? {
                    taken = self.paid_drain(actor, peer, segment, cost, 0.5, Orange, GreenBrown)?;
                }
            }
            Green | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade | DarkJade
            | DarkGreen | Purple | Teal | Cyan | Auburn | Indigo | Blond | Flower | DarkGray
            | Gold | Spore => {
                if !self.drain_dodge(actor, peer, Orange)? {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Yellow)?;
                }
            }
            Yellow => {
                if !self.drain_dodge(actor, peer, Orange)? {
                    let (factor, target) = if peer.effects.is_consumer
                        || peer.effects.is_fungus
                        || peer.effects.is_plant
                        || peer.effects.transfers_energy
                    {
                        (1.0, Yellow)
                    } else {
                        (0.1, GreenBrown)
                    };
                    taken = self.paid_drain(actor, peer, segment, cost, factor, Orange, target)?;
                }
            }
            Orange => {
                taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Orange)?;
            }
            Fire => {
                taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Fire)?;
            }
            Silver => {
                let target = if experienced_silver(peer) {
                    Gold
                } else {
                    Yellow
                };
                taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, target)?;
            }
            Coral => {
                let (factor, target) = if peer.effects.is_consumer {
                    (1.0, Yellow)
                } else {
                    (0.5, GreenBrown)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, Orange, target)?;
            }
            Plague => {
                let (factor, target) = if peer.effects.is_infectious {
                    (1.0, Yellow)
                } else {
                    (0.1, GreenBrown)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, Orange, target)?;
            }
            Spike => {
                if peer.effects.is_plant || peer.effects.is_enhanced {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Yellow)?;
                }
            }
            Fallow => {
                let (factor, target) = if !peer.effects.is_plant && !peer.effects.is_infectious {
                    if actor.effects.is_enhanced && peer.effects.fallow_version >= 3 {
                        (0.1, GreenBrown)
                    } else {
                        (0.01, Broken)
                    }
                } else if (actor.effects.is_enhanced && peer.effects.fallow_version >= 3)
                    || (peer.effects.is_infectious && peer.effects.fallow_version == 4)
                {
                    (1.0, Yellow)
                } else {
                    (0.1, GreenBrown)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, Orange, target)?;
            }
            Blue => {
                if peer.use_energy(
                    self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
                    if peer.effects.is_enhanced {
                        let retaliation = between(
                            (0.5 * peer.body.physical_length(peer_segment)?.sqrt())
                                * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                            0.0,
                            actor.state.energy,
                        );
                        actor.use_energy(retaliation)?;
                        tint(actor, DarkLilac);
                        if actor.state.energy < TOLERANCE {
                            self.die(actor, Some(Killer::Other(&mut *peer)))?;
                        }
                    } else {
                        tint(actor, Orange);
                    }
                    tint(peer, Blue);
                } else {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Yellow)?;
                }
            }
            LightBlue => {
                if peer.use_energy(
                    self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
                    tint(actor, Orange);
                    tint(peer, LightBlue);
                } else {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Yellow)?;
                }
            }
            Sky => {
                if peer.use_energy(
                    self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                    peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                    peer.effects.use_extra_effects = true;
                    tint(actor, Orange);
                    tint(peer, DeepSky);
                } else {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Yellow)?;
                }
            }
            DeepSky => {
                tint(actor, Orange);
                tint(peer, DeepSky);
            }
            Mint | Lavender | Magenta | Rose => {
                if !actor.effects.altruist && !self.drain_dodge(actor, peer, Orange)? {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Yellow)?;
                }
            }
            Fruit => {
                if peer.effects.spore_version == 2
                    && actor.use_energy(cost)?
                {
                    let (factor, target) = if peer.effects.is_enhanced {
                        (0.1, GreenBrown)
                    } else {
                        (1.0, Yellow)
                    };
                    taken = self.drain_amount(actor, peer, segment, factor)?;
                    tint(peer, target);
                    if !infected_by_peer(actor, peer) {
                        self.infect_from(actor, Infector::Other(&mut *peer), InfectionKind::Fruit)?;
                        tint(actor, Blond);
                    } else {
                        tint(actor, Orange);
                    }
                }
            }
            Ice | DeadBark => {
                if actor.effects.sky_version > 0 || actor.effects.is_enhanced {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Yellow)?;
                }
            }
            Bark => age_bark(peer, peer_segment)?,
            OldBark | Red | Maroon | Cream | Ochre | Olive | DarkOlive | SpikePoint | Brown => {}
            // The original default drains unclassified and unlisted colors at full strength
            NoColor | Pink | Lilac | Gray | Dark | DarkFire | DarkLilac | Vision | Eye
            | LightBrown | GreenBrown | Broken => {
                taken = self.paid_drain(actor, peer, segment, cost, 1.0, Orange, Yellow)?;
            }
        }
        self.finish_drain(actor, peer, taken, Byproduct::Respiration)?;
        Ok(())
    }

    pub(super) fn touch_fire(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let mut taken = 0.0;
        let cost = self.session.0.parameters.borrow().lifecycle.fire_energy_consumption;
        peer.effects.has_dodged = true;
        let full = full_fire_prey(actor, peer);
        match color(peer, peer_segment)? {
            White => {
                let (factor, own) = if full { (1.0, Fire) } else { (0.25, DarkFire) };
                taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
            }
            Grass => {
                if full {
                    taken = self.paid_drain(actor, peer, segment, cost, 0.5, Fire, GreenBrown)?;
                } else if !self.drain_dodge(actor, peer, DarkFire)? {
                    taken =
                        self.paid_drain(actor, peer, segment, cost, 0.125, DarkFire, GreenBrown)?;
                }
            }
            Green | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade | DarkJade
            | DarkGreen | Purple | Teal | Cyan | Auburn | Indigo | Blond | Flower | DarkGray
            | Gold | Spore => {
                if full {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Fire, Yellow)?;
                } else if !self.drain_dodge(actor, peer, DarkFire)? {
                    taken = self.paid_drain(actor, peer, segment, cost, 0.25, DarkFire, Yellow)?;
                }
            }
            Yellow => {
                if full {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Fire, Yellow)?;
                } else if !self.drain_dodge(actor, peer, DarkFire)? {
                    let (factor, target) = if peer.effects.is_fungus
                        || peer.effects.is_plant
                        || peer.effects.transfers_energy
                    {
                        (0.25, Yellow)
                    } else {
                        (0.1, GreenBrown)
                    };
                    taken = self.paid_drain(actor, peer, segment, cost, factor, DarkFire, target)?;
                }
            }
            Red => taken = self.paid_drain(actor, peer, segment, cost, 1.0, Fire, Red)?,
            Fire => taken = self.paid_drain(actor, peer, segment, cost, 1.0, Fire, Fire)?,
            Orange => taken = self.paid_drain(actor, peer, segment, cost, 1.0, Fire, Orange)?,
            Spike => {
                if peer.effects.is_plant || peer.effects.is_enhanced {
                    let (factor, own) = if full { (1.0, Fire) } else { (0.25, DarkFire) };
                    taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
                }
            }
            Silver => {
                let (factor, own, target) = if experienced_silver(peer) {
                    (1.0, Fire, Gold)
                } else if full {
                    (1.0, Fire, Yellow)
                } else {
                    (0.25, DarkFire, Yellow)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, own, target)?;
            }
            Pink => {
                let (factor, own) =
                    if peer.effects.is_gray && primary_genome(&peer.genome)?.borrow().modifiers().modifies_pink {
                        (1.0, Fire)
                    } else {
                        (0.25, DarkFire)
                    };
                taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
            }
            Maroon => taken = self.paid_drain(actor, peer, segment, cost, 0.25, DarkFire, Yellow)?,
            Cream => {
                let (factor, target) =
                    if peer.effects.is_enhanced && primary_genome(&peer.genome)?.borrow().modifiers().cream == 2 {
                        (0.1, GreenBrown)
                    } else {
                        (0.25, Yellow)
                    };
                taken = self.paid_drain(actor, peer, segment, cost, factor, DarkFire, target)?;
            }
            Fallow => {
                let (factor, own, target) = if !peer.effects.is_plant && !peer.effects.is_infectious
                {
                    (0.01, DarkFire, Broken)
                } else if peer.effects.is_infectious && peer.effects.fallow_version == 4 {
                    if full {
                        (1.0, Fire, Yellow)
                    } else {
                        (0.25, DarkFire, Yellow)
                    }
                } else {
                    (0.1, DarkFire, GreenBrown)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, own, target)?;
            }
            Plague => {
                let (factor, own, target) = if peer.effects.is_infectious {
                    if full {
                        (1.0, Fire, Yellow)
                    } else {
                        (0.25, DarkFire, Yellow)
                    }
                } else {
                    (0.1, DarkFire, GreenBrown)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, own, target)?;
            }
            Blue => {
                if peer.use_energy(
                    self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
                    if peer.effects.is_enhanced {
                        let retaliation = between(
                            (0.5 * peer.body.physical_length(peer_segment)?.sqrt())
                                * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                            0.0,
                            actor.state.energy,
                        );
                        actor.use_energy(retaliation)?;
                        tint(actor, DarkLilac);
                        if actor.state.energy < TOLERANCE {
                            self.die(actor, Some(Killer::Other(&mut *peer)))?;
                        }
                    } else {
                        tint(actor, DarkFire);
                    }
                    tint(peer, Blue);
                } else {
                    taken = self.paid_drain(actor, peer, segment, cost, 0.25, DarkFire, Yellow)?;
                }
            }
            LightBlue => {
                if peer.use_energy(
                    self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
                    tint(actor, DarkFire);
                    tint(peer, LightBlue);
                } else {
                    taken = self.paid_drain(actor, peer, segment, cost, 0.25, DarkFire, Yellow)?;
                }
            }
            Sky => {
                if !actor.effects.is_enhanced || peer.effects.sky_version == 2 {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                        peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                        peer.effects.use_extra_effects = true;
                        tint(actor, DarkFire);
                        tint(peer, DeepSky);
                    } else {
                        taken = self.paid_drain(actor, peer, segment, cost, 0.25, DarkFire, Yellow)?;
                    }
                }
            }
            DeepSky => {
                if !actor.effects.is_enhanced || peer.effects.sky_version == 2 {
                    tint(actor, DarkFire);
                    tint(peer, DeepSky);
                }
            }
            Violet => {
                if full {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Fire, Yellow)?;
                } else if peer.effects.is_fungus
                    || (!actor.effects.is_enhanced
                        && (peer.effects.is_plant
                            || peer.effects.plague_version > 0
                            || peer.effects.is_auburn)
                        && (!peer.effects.is_enhanced || peer.effects.is_infectious))
                {
                    taken = self.paid_drain(actor, peer, segment, cost, 0.25, DarkFire, Yellow)?;
                }
            }
            Fruit => {
                if peer.effects.spore_version == 2 {
                    if actor.use_energy(cost)? {
                        taken = self.drain_amount(actor, peer, segment, 0.1)?;
                        tint(peer, GreenBrown);
                        if !infected_by_peer(actor, peer) {
                            self.infect_from(
                                actor,
                                Infector::Other(&mut *peer),
                                InfectionKind::Fruit,
                            )?;
                            tint(actor, Blond);
                        } else {
                            tint(actor, DarkFire);
                        }
                    }
                } else {
                    taken = self.paid_drain(actor, peer, segment, cost, 0.25, DarkFire, Yellow)?;
                }
            }
            Ice | DeadBark => {
                if actor.effects.sky_version > 0 {
                    let (factor, own) = if peer.effects.is_consumer || peer.effects.is_plant {
                        (1.0, Fire)
                    } else {
                        (0.25, DarkFire)
                    };
                    taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
                }
            }
            Bark => age_bark(peer, peer_segment)?,
            OldBark | Ochre | Olive | DarkOlive | SpikePoint => {}
            LightBrown => taken = self.paid_drain(actor, peer, segment, cost, 1.0, Fire, Yellow)?,
            Brown => {
                if actor.effects.is_enhanced {
                    if !peer.state.alive {
                        taken = self.paid_drain(actor, peer, segment, cost, 1.0, Fire, Yellow)?;
                    } else if actor.use_energy(cost)? {
                        taken = self.drain_amount(actor, peer, segment, 0.25)?;
                        tint(peer, Yellow);
                        tint(actor, DarkFire);
                        if !peer.effects.is_enhanced {
                            peer.effects.is_injured = true;
                        }
                    }
                }
            }
            GreenBrown => {
                if actor.effects.is_enhanced {
                    let (factor, own) = if full { (1.0, Fire) } else { (0.25, DarkFire) };
                    taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
                }
            }
            Mint | Lavender | Magenta | Rose => {
                if !actor.effects.altruist {
                    if full {
                        taken = self.paid_drain(actor, peer, segment, cost, 1.0, Fire, Yellow)?;
                    } else if !self.drain_dodge(actor, peer, DarkFire)? {
                        taken = self.paid_drain(actor, peer, segment, cost, 0.25, DarkFire, Yellow)?;
                    }
                }
            }
            // The original default uses this same consumer/frozen-plant classification
            NoColor | Coral | Lilac | Gray | Dark | DarkFire | DarkLilac | Vision | Eye
            | Broken => {
                let (factor, own) = if full { (1.0, Fire) } else { (0.25, DarkFire) };
                taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
            }
        }
        self.finish_drain(actor, peer, taken, Byproduct::Respiration)?;
        Ok(())
    }

    pub(super) fn touch_red(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let mut taken = 0.0;
        let cost = self.session.0.parameters.borrow().lifecycle.red_energy_consumption;
        match color(peer, peer_segment)? {
            Red => taken = self.paid_drain(actor, peer, segment, cost, 1.0, Red, Red)?,
            Fire => taken = self.paid_drain(actor, peer, segment, cost, 1.0, Red, Fire)?,
            Orange | Maroon | Pink => {
                taken = self.paid_drain(actor, peer, segment, cost, 1.0, Red, Yellow)?;
            }
            Silver => {
                if experienced_silver(peer) {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Red, Gold)?;
                }
            }
            Spike => {
                if peer.effects.is_enhanced {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Red, Yellow)?;
                }
            }
            SpikePoint => {
                if actor.effects.is_enhanced && peer.effects.is_enhanced {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Red, Yellow)?;
                }
            }
            Gray => {
                if actor.effects.is_enhanced && (peer.effects.is_consumer || peer.effects.is_fungus)
                {
                    let (factor, own) = if peer.effects.is_enhanced {
                        (1.0, Red)
                    } else {
                        (0.25, DarkFire)
                    };
                    taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
                }
            }
            Cream => {
                if actor.effects.is_enhanced {
                    let (factor, target) = if peer.effects.is_enhanced
                        && primary_genome(&peer.genome)?.borrow().modifiers().cream == 2
                    {
                        (0.1, GreenBrown)
                    } else {
                        (0.25, Yellow)
                    };
                    taken = self.paid_drain(actor, peer, segment, cost, factor, DarkFire, target)?;
                }
            }
            Lilac | DarkLilac => {
                if actor.effects.is_enhanced && (peer.effects.is_consumer || peer.effects.is_fungus)
                {
                    taken = self.paid_drain(actor, peer, segment, cost, 0.25, DarkFire, Yellow)?;
                }
            }
            Bark => age_bark(peer, peer_segment)?,
            // All remaining colors follow the original RED no-op default
            NoColor | Green | Forest | Spring | Summer | Lime | Leaf | C4 | Jade | Grass
            | Purple | Violet | Olive | Sky | Blue | Ochre | Fallow | Spore | White | Plague
            | Coral | Mint | Lavender | Magenta | Rose | Cyan | Teal | Yellow | Auburn | Indigo
            | Blond | Flower | DarkGray | Gold | Dark | Eye | Winter | OldBark | DarkJade
            | DarkGreen | DarkFire | DeepSky | DarkOlive | Fruit | Vision | Ice | LightBlue
            | LightBrown | GreenBrown | Broken | DeadBark | Brown => {}
        }
        self.finish_drain(actor, peer, taken, Byproduct::Respiration)?;
        Ok(())
    }

    pub(super) fn touch_pink(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let mut taken = 0.0;
        let cost = self.session.0.parameters.borrow().lifecycle.pink_energy_consumption;
        peer.effects.has_dodged = true;
        match color(peer, peer_segment)? {
            Pink => {
                let (factor, own) = if primary_genome(&actor.genome)?.borrow().modifiers().modifies_pink {
                    (1.0, Pink)
                } else {
                    (0.25, DarkFire)
                };
                let target = if primary_genome(&peer.genome)?.borrow().modifiers().modifies_pink {
                    Pink
                } else {
                    DarkFire
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, own, target)?;
            }
            White | LightBrown | Ice => {
                taken = self.paid_drain(actor, peer, segment, cost, 1.0, Pink, Yellow)?;
            }
            Fruit => {
                if actor.use_energy(cost)? {
                    taken = self.drain_amount(actor, peer, segment, 1.0)?;
                    tint(peer, Yellow);
                    if peer.effects.spore_version == 2 && !infected_by_peer(actor, peer) {
                        if primary_genome(&actor.genome)?.borrow().modifiers().modifies_pink || actor.effects.is_blond {
                            self.infect_from(
                                actor,
                                Infector::Other(&mut *peer),
                                InfectionKind::Fruit,
                            )?;
                            tint(actor, Blond);
                        } else if actor.effects.indigo == 0 {
                            self.infect_from(
                                actor,
                                Infector::Other(&mut *peer),
                                InfectionKind::WeakFruit,
                            )?;
                            tint(actor, Blond);
                        } else {
                            tint(actor, Pink);
                        }
                    } else {
                        tint(actor, Pink);
                    }
                }
            }
            Cream => {
                let (factor, target) = if (peer.effects.is_enhanced || !peer.state.active)
                    && primary_genome(&peer.genome)?.borrow().modifiers().cream == 2
                {
                    (0.1, GreenBrown)
                } else {
                    (1.0, Yellow)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, Pink, target)?;
            }
            GreenBrown | Broken | LightBlue | DarkOlive | DeepSky => {
                let (factor, own) = if primary_genome(&actor.genome)?.borrow().modifiers().modifies_pink
                    || (peer.effects.is_blond && peer.effects.is_plant)
                    || peer.effects.is_frozen
                    || !peer.state.active
                {
                    (1.0, Pink)
                } else {
                    (0.25, DarkFire)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
            }
            DeadBark => {
                let (factor, own) = if primary_genome(&actor.genome)?.borrow().modifiers().modifies_pink
                    || peer.effects.is_blond
                    || (peer.effects.is_frozen && actor.effects.sky_version > 0)
                    || !peer.state.active
                {
                    (1.0, Pink)
                } else {
                    (0.25, DarkFire)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
            }
            DarkFire => {
                let (factor, own) = if primary_genome(&actor.genome)?.borrow().modifiers().modifies_pink
                    || (!peer.effects.is_plant
                        && !peer.effects.is_consumer
                        && !peer.effects.is_fungus)
                    || !peer.state.active
                {
                    (1.0, Pink)
                } else {
                    (0.25, DarkFire)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
            }
            Coral => {
                let (factor, target) = if !primary_genome(&actor.genome)?.borrow().modifiers().modifies_pink || peer.effects.is_consumer {
                    (1.0, Yellow)
                } else {
                    (0.5, GreenBrown)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, Pink, target)?;
            }
            Brown => {
                if !actor.effects.is_gray || primary_genome(&actor.genome)?.borrow().modifiers().modifies_pink {
                    if peer.state.alive {
                        if actor.use_energy(cost)? {
                            taken = self.drain_amount(actor, peer, segment, 1.0)?;
                            tint(peer, Yellow);
                            tint(actor, Pink);
                            if !peer.effects.is_enhanced {
                                peer.effects.is_injured = true;
                            }
                        }
                    } else if actor.use_energy(cost)? {
                        taken = if actor.effects.fallow_version > 0
                            && (peer.state.age >> 8)
                                >= peer.state.reproduction.maximum_age.wrapping_mul(2)
                        {
                            peer.state.energy
                        } else {
                            self.drain_amount(actor, peer, segment, 1.0)?
                        };
                        tint(peer, Yellow);
                        tint(actor, Pink);
                    }
                }
            }
            Plague => {
                let (factor, target) = if peer.effects.is_infectious {
                    (1.0, Yellow)
                } else {
                    (0.1, GreenBrown)
                };
                taken = self.paid_drain(actor, peer, segment, cost, factor, Pink, target)?;
            }
            OldBark => {
                if actor.effects.is_enhanced {
                    if peer.effects.is_enhanced && peer.state.active {
                        let full_drain = primary_genome(&actor.genome)?.borrow().modifiers().modifies_pink
                            || peer.effects.is_blond;
                        if actor.use_energy(cost)? {
                            if peer.state.temporary_color_frames > 0 {
                                let (factor, own) = if full_drain {
                                    (0.01, Pink)
                                } else {
                                    (0.0025, DarkFire)
                                };
                                taken = self.drain_amount(actor, peer, segment, factor)?;
                                tint(peer, Broken);
                                tint(actor, own);
                            } else {
                                tint(peer, Broken);
                            }
                        }
                    } else {
                        let (factor, own) = if primary_genome(&actor.genome)?.borrow().modifiers().modifies_pink || peer.effects.is_blond {
                            (1.0, Pink)
                        } else {
                            (0.25, DarkFire)
                        };
                        taken = self.paid_drain(actor, peer, segment, cost, factor, own, Yellow)?;
                    }
                }
            }
            Bark => age_bark(peer, peer_segment)?,
            // All remaining colors follow the original PINK no-op default
            NoColor | Green | Forest | Spring | Summer | Lime | Leaf | C4 | Jade | Grass
            | Purple | Red | Fire | Orange | Maroon | Silver | Spike | Lilac | Gray | Violet
            | Olive | Sky | Blue | Ochre | Fallow | Spore | Mint | Lavender | Magenta | Rose
            | Cyan | Teal | Yellow | Auburn | Indigo | Blond | Flower | DarkGray | Gold | Dark
            | Eye | Winter | DarkJade | DarkGreen | DarkLilac | SpikePoint | Vision => {}
        }
        self.finish_drain(actor, peer, taken, Byproduct::Decomposition)?;
        Ok(())
    }

    pub(super) fn touch_maroon(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let mut taken = 0.0;
        let cost = self.session.0.parameters.borrow().lifecycle.maroon_energy_consumption;
        peer.effects.has_dodged = true;
        match color(peer, peer_segment)? {
            White | Fallow | Ochre | DeepSky | DarkOlive | Spike | Ice => {
                if peer.effects.is_plant {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Maroon, Yellow)?;
                }
            }
            Grass => {
                if !self.drain_dodge(actor, peer, Maroon)? {
                    taken = self.paid_drain(actor, peer, segment, cost, 0.5, Maroon, GreenBrown)?;
                }
            }
            Green | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade | DarkJade
            | DarkGreen | Purple => {
                if !self.drain_dodge(actor, peer, Maroon)? {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Maroon, Yellow)?;
                }
            }
            Blue => {
                if peer.effects.is_plant {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
                        if peer.effects.is_enhanced && !actor.effects.is_enhanced {
                            let retaliation = between(
                                (0.25 * peer.body.physical_length(peer_segment)?.sqrt())
                                    * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                                0.0,
                                actor.state.energy,
                            );
                            actor.use_energy(
                                retaliation)?;
                            tint(actor, DarkLilac);
                            if actor.state.energy < TOLERANCE {
                                self.die(actor, Some(Killer::Other(&mut *peer)))?;
                            }
                        } else {
                            tint(actor, Maroon);
                        }
                        tint(peer, Blue);
                    } else {
                        taken = self.paid_drain(actor, peer, segment, cost, 1.0, Maroon, Yellow)?;
                    }
                }
            }
            Plague => {
                if peer.effects.is_plant {
                    let (factor, target) = if peer.effects.is_infectious {
                        (1.0, Yellow)
                    } else {
                        (0.1, GreenBrown)
                    };
                    taken = self.paid_drain(actor, peer, segment, cost, factor, Maroon, target)?;
                }
            }
            Fruit => {
                if peer.effects.spore_version == 2
                    && actor.use_energy(cost)?
                {
                    taken = self.drain_amount(actor, peer, segment, 1.0)?;
                    tint(peer, Yellow);
                    if !infected_by_peer(actor, peer) {
                        self.infect_from(actor, Infector::Other(&mut *peer), InfectionKind::Fruit)?;
                        tint(actor, Blond);
                    } else {
                        tint(actor, Maroon);
                    }
                }
            }
            Broken | LightBlue | DeadBark => {
                if peer.effects.is_plant || !peer.state.active {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Maroon, Yellow)?;
                }
            }
            OldBark => {
                if peer.state.active {
                    if peer.effects.is_enhanced {
                        if actor.use_energy(cost)? {
                            if peer.state.temporary_color_frames > 0 {
                                taken = self.drain_amount(actor, peer, segment, 0.01)?;
                                tint(peer, Broken);
                                tint(actor, Maroon);
                            } else {
                                tint(peer, Broken);
                            }
                        }
                    } else {
                        taken = self.paid_drain(actor, peer, segment, cost, 1.0, Maroon, Yellow)?;
                    }
                } else if peer.effects.is_enhanced {
                    taken = self.paid_drain(actor, peer, segment, cost, 0.5, Maroon, GreenBrown)?;
                } else {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Maroon, Yellow)?;
                }
            }
            Bark => {
                age_bark(peer, peer_segment)?;
                if !peer.state.active && !peer.effects.is_enhanced {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Maroon, Yellow)?;
                }
            }
            Cyan | Teal | Yellow | Auburn | Indigo | Blond | Flower | DarkGray | Gold | Spore => {
                if actor.effects.is_enhanced
                    && peer.effects.is_plant
                    && !self.drain_dodge(actor, peer, Maroon)?
                {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Auburn, Yellow)?;
                }
            }
            Silver => {
                if !experienced_silver(peer) && actor.effects.is_enhanced && peer.effects.is_plant {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Auburn, Yellow)?;
                }
            }
            SpikePoint => {
                if actor.effects.is_enhanced && !peer.effects.is_enhanced && peer.effects.is_plant {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Auburn, Yellow)?;
                }
            }
            Mint | Lavender | Magenta | Rose => {
                if !actor.effects.altruist
                    && actor.effects.is_enhanced
                    && peer.effects.is_plant
                    && !self.drain_dodge(actor, peer, Maroon)?
                {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Auburn, Yellow)?;
                }
            }
            Sky => {
                if actor.effects.is_enhanced && peer.effects.is_plant {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                        peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                        peer.effects.use_extra_effects = true;
                        tint(actor, Auburn);
                        tint(peer, DeepSky);
                    } else {
                        taken = self.paid_drain(actor, peer, segment, cost, 1.0, Auburn, Yellow)?;
                    }
                }
            }
            GreenBrown | Vision => {
                if actor.effects.is_enhanced && peer.effects.is_plant {
                    taken = self.paid_drain(actor, peer, segment, cost, 1.0, Auburn, Yellow)?;
                }
            }
            // All remaining colors follow the original MAROON no-op default
            NoColor | Red | Fire | Orange | Maroon | Pink | Cream | Lilac | Gray | Violet
            | Olive | Coral | Dark | Eye | DarkFire | DarkLilac | LightBrown | Brown => {}
        }
        self.finish_drain(actor, peer, taken, Byproduct::Respiration)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use crate::{Gene, Genome, GenomeProperties, OrganismState, WorldConfig};
    use std::cell::RefCell;

    fn organism(world: &World, kind: ColorKind) -> Organism {
        let gene = Gene::new(4.0, 0.0, kind.argb(), 0, [0; 36]).shared();
        let code = Genome::from_parts(
            &[gene],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        let mut organism =
            Organism::unplaced(Rc::new(RefCell::new(code)), OrganismState { world: world.association(), ..OrganismState::default() }).unwrap();
        organism.body.set_physical_length(0, 4.0).unwrap();
        organism.state.energy = 10.0;
        organism
    }

    fn world() -> World {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        world.oxygen.set(10.0);
        world.carbon_dioxide.set(0.0);
        world.methane.set(0.0);
        world
    }

    #[test]
    fn partial_dodge_payment_precedes_attack_payment_and_has_dodged_does_not_block_repeated_dodges()
    {
        let world = world();
        let mut actor = organism(&world, ColorKind::Orange);
        let mut peer = organism(&world, ColorKind::Grass);
        world.session.0.parameters.borrow_mut().lifecycle.orange_energy_consumption = 1.0;
        world.session.0.parameters.borrow_mut().lifecycle.dodge_energy_consumption = 1.0;
        world.oxygen.set(0.5);
        peer.effects.dodge = true;
        peer.effects.has_dodged = false;
        world.touch_orange(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.state.energy, 10.0);
        assert_eq!(peer.state.energy, 9.5);
        assert_eq!((world.oxygen.get(), world.carbon_dioxide.get()), (0.0, 0.5));
        assert!(peer.effects.has_dodged);
        assert_eq!(actor.state.temporary_argb(), None);
        assert_eq!(peer.state.temporary_argb(), None);
        world.oxygen.set(10.0);
        world.touch_orange(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.state.energy, 10.0);
        assert_eq!(peer.state.energy, 8.5);
        assert_eq!(peer.state.temporary_argb(), ColorKind::Teal.argb());
        assert_eq!(actor.state.temporary_argb(), ColorKind::Orange.argb());
    }

    #[test]
    fn enhanced_blue_retaliation_precedes_the_common_peer_death_guard() {
        let world = world();
        let mut actor = organism(&world, ColorKind::Orange);
        let mut peer = organism(&world, ColorKind::Blue);
        actor.state.energy = 0.25;
        peer.state.energy = 0.0;
        peer.effects.is_enhanced = true;
        world.oxygen.set(0.25);
        world.touch_orange(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(!actor.state.alive && !peer.state.alive);
        assert_eq!((actor.state.total_kills, peer.state.total_kills), (1, 1));
        assert_eq!(world.population(), -2);
        assert_eq!(actor.state.temporary_argb(), ColorKind::DarkLilac.argb());
        assert_eq!(peer.state.temporary_argb(), ColorKind::Blue.argb());
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::Brown.argb().unwrap()
        );
        assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::Brown.argb().unwrap());
    }

    #[test]
    fn pink_byproduct_uses_methane_and_old_corpse_thresholds_wrap_as_java_integers() {
        let world = world();
        let mut actor = organism(&world, ColorKind::Pink);
        let mut peer = organism(&world, ColorKind::White);
        world.touch_pink(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!((actor.state.energy, peer.state.energy), (13.6, 6.0));
        assert_eq!(
            (world.oxygen.get(), world.carbon_dioxide.get(), world.methane.get()),
            (9.6, 0.0, 0.4)
        );
        let mut actor = organism(&world, ColorKind::Pink);
        let mut peer = organism(&world, ColorKind::Brown);
        actor.effects.fallow_version = 1;
        peer.state.alive = false;
        peer.state.age = 0;
        peer.state.reproduction.maximum_age = i32::MAX;
        world.touch_pink(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!((actor.state.energy, peer.state.energy), (19.0, 0.0));
        assert_eq!(actor.state.total_kills, 0);
    }

    #[test]
    fn enhanced_old_bark_first_contact_pays_and_primes_the_peer_without_tinting_the_attacker() {
        let world = world();
        let mut actor = organism(&world, ColorKind::Pink);
        let mut peer = organism(&world, ColorKind::OldBark);
        actor.effects.is_enhanced = true;
        peer.effects.is_enhanced = true;
        actor.state.temporary_color = (ColorKind::Blue.argb()).map(crate::JavaColor::from_argb);
        peer.state.temporary_color_frames = 0;
        world.session.0.parameters.borrow_mut().lifecycle.pink_energy_consumption = 1.0;
        world.touch_pink(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!((actor.state.energy, peer.state.energy), (9.0, 10.0));
        assert_eq!(actor.state.temporary_argb(), ColorKind::Blue.argb());
        assert_eq!(peer.state.temporary_argb(), ColorKind::Broken.argb());
        assert_eq!(peer.state.temporary_color_frames, 10);
        world.touch_pink(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.state.energy, 9.99);
        assert_eq!(actor.state.temporary_argb(), ColorKind::DarkFire.argb());
    }

    #[test]
    fn inactive_bark_drains_only_for_the_maroon_branch_and_retains_photo_coefficients() {
        let world = world();
        let mut actor = organism(&world, ColorKind::Maroon);
        let mut peer = organism(&world, ColorKind::Bark);
        peer.state.active = false;
        peer.effects.photosynthesis = 5.0;
        peer.body.set_photo(0, 3.0).unwrap();
        world.touch_maroon(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!((actor.state.energy, peer.state.energy), (13.6, 6.0));
        assert_eq!(peer.effects.photosynthesis, 5.0);
        assert_eq!(peer.body.photo_values().unwrap(), vec![3.0]);
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::OldBark.argb().unwrap()
        );
        let mut actor = organism(&world, ColorKind::Orange);
        let mut peer = organism(&world, ColorKind::Bark);
        peer.state.active = false;
        world.touch_orange(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!((actor.state.energy, peer.state.energy), (10.0, 10.0));
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::OldBark.argb().unwrap()
        );
    }

    #[test]
    fn red_gray_attack_uses_quarter_strength_without_marking_dodge_state() {
        let world = world();
        let mut actor = organism(&world, ColorKind::Red);
        let mut peer = organism(&world, ColorKind::Gray);
        actor.effects.is_enhanced = true;
        peer.effects.is_consumer = true;
        peer.effects.has_dodged = false;
        world.touch_red(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!((actor.state.energy, peer.state.energy), (10.9, 9.0));
        assert!(!peer.effects.has_dodged);
        assert_eq!(actor.state.temporary_argb(), ColorKind::DarkFire.argb());
    }

    #[test]
    fn fruit_infection_retains_the_exact_peer_code_and_wraps_its_infection_counter() {
        let world = world();
        let mut actor = organism(&world, ColorKind::Orange);
        let mut peer = organism(&world, ColorKind::Fruit);
        actor.saved_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        peer.effects.spore_version = 2;
        peer.state.total_infected = i32::MAX;
        world.touch_orange(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(Rc::ptr_eq(
            actor.infected_genome.as_ref().unwrap(), peer.genome.as_ref().unwrap()));
        assert!(actor.saved_genome.is_none());
        assert_eq!(peer.state.total_infected, i32::MIN);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Blond.argb());
        assert_eq!((actor.state.energy, peer.state.energy), (13.6, 6.0));
    }

    #[test]
    fn fire_uses_the_attackers_sky_flag_and_preserves_the_enhanced_sky_shield_gate() {
        let world = world();
        let mut actor = organism(&world, ColorKind::Fire);
        let mut peer = organism(&world, ColorKind::Green);
        actor.effects.sky_version = 1;
        peer.effects.is_frozen = true;
        peer.effects.is_plant = true;
        world.touch_fire(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!((actor.state.energy, peer.state.energy), (13.6, 6.0));
        assert_eq!(actor.state.temporary_argb(), ColorKind::Fire.argb());
        let mut actor = organism(&world, ColorKind::Fire);
        let mut peer = organism(&world, ColorKind::Sky);
        actor.effects.is_enhanced = true;
        peer.effects.is_plant = true;
        peer.effects.sky_version = 1;
        world.touch_fire(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::Sky.argb().unwrap());
        assert_eq!(peer.state.energy, 10.0);
        peer.effects.sky_version = 2;
        world.touch_fire(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::DeepSky.argb().unwrap()
        );
        assert!(peer.effects.use_extra_effects);
        assert_eq!(peer.state.energy, 9.95);
        assert_eq!(actor.state.temporary_argb(), ColorKind::DarkFire.argb());
    }

    #[test]
    fn common_death_guard_runs_for_no_op_colors_and_keeps_the_strict_tolerance() {
        let world = world();
        let mut actor = organism(&world, ColorKind::Red);
        let mut peer = organism(&world, ColorKind::Green);
        peer.effects.has_dodged = false;
        peer.state.energy = TOLERANCE;
        world.touch_red(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(peer.state.alive);
        peer.state.energy = TOLERANCE / 2.0;
        world.touch_red(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(!peer.state.alive);
        assert_eq!(actor.state.total_kills, 1);
        assert!(!peer.effects.has_dodged);
    }
}

#[cfg(test)]
mod feeding_observer_tests {
    use super::*;
    use crate::{Gene, Genome, GenomeHandle, GenomeProperties, WorldConfig};

    fn world() -> World {
        let world = World::new(WorldConfig {
            width: 1200, height: 800, initial_attempts: 0, seed: 827364,
        }).unwrap();
        let genome = GenomeHandle::from_owned(Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
            GenomeProperties { symmetry: 1, mutation_rate: 0, ..GenomeProperties::default() },
        ));
        for position in [[200, 200], [800, 500]] {
            world.paste_genome_handle(&genome, position).unwrap().unwrap();
        }
        world
    }

    fn transfer(world: &World, amount: f64, byproduct: Byproduct) {
        let actor = world.organisms.actor(0).unwrap();
        let peer = world.organisms.actor(1).unwrap();
        world.finish_drain(&mut actor.borrow_mut(), &mut peer.borrow_mut(), amount, byproduct).unwrap();
    }

    fn save(world: &World) -> Vec<u8> {
        let limits = crate::object_stream::Limits::default();
        world.encode_legacy_file_bytes(limits, limits.allocated_bytes).unwrap()
    }

    #[test]
    fn positive_transfer_observation_preserves_full_state_rng_and_legacy_bytes() {
        let enabled = world();
        let disabled = enabled.clone();
        let before = save(&enabled);
        let readout = enabled.observe_feeding();
        assert_eq!(save(&enabled), before);
        for byproduct in [Byproduct::Respiration, Byproduct::Decomposition] {
            readout.begin_batch();
            transfer(&enabled, 1.0, byproduct);
            transfer(&disabled, 1.0, byproduct);
            let event = readout.snapshot(&enabled).unwrap();
            assert_eq!(event.gross_energy, 1.0);
            assert_eq!(event.tick, enabled.tick());
            assert_eq!(event.predator.id, enabled.organisms.actor(0).unwrap().borrow().id);
            assert_eq!(event.prey.center, disabled.organisms.actor(1).unwrap().borrow().state.center);
            assert_eq!(enabled, disabled);
            assert_eq!(enabled.random_snapshot(), disabled.random_snapshot());
            assert_eq!(save(&enabled), save(&disabled));
        }
        enabled.resume();
        disabled.resume();
        for _ in 0..4 {
            assert_eq!(enabled.step().unwrap(), disabled.step().unwrap());
        }
        assert_eq!(enabled, disabled);
        assert_eq!(enabled.random_snapshot(), disabled.random_snapshot());
        assert_eq!(save(&enabled), save(&disabled));
    }

    #[test]
    fn zero_transfer_and_dead_peer_do_not_report_active_feeding() {
        let world = world();
        let readout = world.observe_feeding();
        readout.begin_batch();
        transfer(&world, 0.0, Byproduct::Respiration);
        assert!(readout.snapshot(&world).is_none());
        let peer = world.organisms.actor(1).unwrap();
        world.die(&mut peer.borrow_mut(), None).unwrap();
        transfer(&world, 1.0, Byproduct::Decomposition);
        assert!(readout.snapshot(&world).is_none());
    }

    #[test]
    fn observer_keeps_only_latest_stamp_and_invalidates_on_replacement_or_genesis() {
        let mut world = world();
        let old = world.observe_feeding();
        old.begin_batch();
        transfer(&world, 1.0, Byproduct::Respiration);
        transfer(&world, 2.0, Byproduct::Respiration);
        assert_eq!(old.snapshot(&world).unwrap().gross_energy, 2.0);
        old.begin_batch();
        assert!(old.snapshot(&world).is_none());
        let current = world.observe_feeding();
        assert!(!old.is_current_for(&world));
        transfer(&world, 1.0, Byproduct::Respiration);
        let copied = world.clone();
        assert!(current.snapshot(&copied).is_none());
        assert!(copied.feeding_observer.borrow().upgrade().is_none());
        world.genesis().unwrap();
        assert!(current.snapshot(&world).is_none());
        let after_genesis = world.observe_feeding();
        world.reset().unwrap();
        assert!(after_genesis.snapshot(&world).is_none());
    }
}
