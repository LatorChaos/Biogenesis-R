// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Complete PLAGUE and FALLOW touchEffects2 branches from Organism.java:14142-15861
//! Each entry includes the shared peer death guard at Organism.java:17917-17919

use crate::organism::primary_genome;

use super::contact_state::ContactActor;
use super::death::Killer;
use super::draining_contacts::age_bark;
use super::{Organism, OrganismRef, World, WorldError};
use crate::{ColorKind, TOLERANCE, physics::between};
use crate::organism::same_genome;

fn color(organism: &Organism, segment: usize) -> Result<ColorKind, crate::WorldError> {
    Ok(organism.body.color_kind(segment)?)
}

fn tint(organism: &mut Organism, color: ColorKind) {
    organism.set_display_color(Some(color.color_ref().expect("Tint requires a named source literal")));
}

fn infected_by_actor(actor: &Organism, peer: &Organism) -> bool {
    same_genome(peer.infected_genome.as_ref(), actor.genome.as_ref())
}

fn plague_infection(actor: &Organism, peer: &Organism) -> bool {
    infected_by_actor(actor, peer)
        || (!actor.effects.is_infectious && peer.infected_genome.is_some())
}

fn plague_class(actor: &Organism, peer: &Organism) -> bool {
    ((actor.effects.plague_version == 2 || !actor.effects.is_infectious)
        && peer.effects.is_consumer)
        || (actor.effects.plague_version == 1 && peer.effects.is_plant)
}

fn fallow_relation(actor: &Organism, peer: &Organism) -> bool {
    peer.effects.fallow_version != actor.effects.fallow_version
        || peer.effects.is_infectious
        || (!actor.effects.is_consumer && !actor.effects.is_fungus)
        || actor.effects.is_enhanced
}

fn fallow_class(actor: &Organism, peer: &Organism) -> bool {
    peer.effects.is_plant
        || peer.effects.is_consumer
        || (peer.effects.is_fungus
            && ((actor.effects.fallow_version == 2 && peer.effects.antiviral == 0)
                || !actor.effects.is_plant))
}

#[derive(Clone, Copy)]
enum Inhibition {
    Classified,
    Reproductive,
    ImmuneSegment,
}

impl World {
    fn plague_fallow_peer_death(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        if peer.state.alive && peer.state.energy < TOLERANCE {
            self.die(peer, Some(Killer::Other(actor)))?;
        }
        Ok(())
    }

    fn plague_dodge(&self, actor: &mut Organism, peer: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            tint(peer, ColorKind::Teal);
            tint(actor, ColorKind::Plague);
            peer.effects.has_dodged = true;
            true
        } else {
            false
        }
    
        })
    }

    fn plague_shield(&self, actor: &mut Organism, peer: &mut Organism) -> bool {
        if peer.effects.lavender > 0 {
            if actor.effects.is_enhanced
                && !actor.effects.is_infectious
                && !actor.effects.is_consumer
                && !actor.effects.is_fungus
            {
                self.weak_lavender_shield(peer);
            } else {
                self.lavender_shield(peer);
            }
            tint(actor, ColorKind::Plague);
        }
        peer.effects.lavender <= 0
    }

    fn plague_birth(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
    ) -> Result<(), WorldError> {
        let cost = if actor.effects.is_infectious && actor.effects.is_plant {
            self.session.0.parameters.borrow().lifecycle.plague_energy_consumption
        } else {
            self.session.0.parameters.borrow().lifecycle.scourge_energy_consumption
        };
        if actor.state.energy > cost {
            if actor.effects.is_infectious {
                self.reproduce_virus_from(peer_ref, peer, Some((actor_ref, actor)))?;
            } else {
                if !actor.effects.is_consumer && !actor.effects.is_fungus {
                    peer.state.virus_children = -1;
                }
                self.reproduce_foreign_virus_from(peer_ref, peer, Some((actor_ref, actor)))?;
            }
            tint(actor, ColorKind::Plague);
        }
        Ok(())
    }

    fn plague_shield_birth(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
    ) -> Result<(), WorldError> {
        if self.plague_shield(actor, peer) {
            self.plague_birth(actor_ref, actor, peer_ref, peer)?;
        }
        Ok(())
    }

    fn plague_indigo(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        Ok({
        let cost = if actor.effects.is_plant && actor.effects.is_infectious {
            (self.session.0.parameters.borrow().lifecycle.plague_energy_consumption + 0.01)
                / self.session.0.parameters.borrow().lifecycle.indigo_energy_consumption
        } else if actor.effects.indigo == 0
            || actor.effects.is_plant
            || actor.effects.is_consumer
            || actor.effects.is_fungus
        {
            (self.session.0.parameters.borrow().lifecycle.scourge_energy_consumption + 0.1)
                / self.session.0.parameters.borrow().lifecycle.indigo_energy_consumption
        } else {
            return Ok(());
        };
        if actor.use_energy(cost)? {
            tint(peer, ColorKind::Indigo);
            tint(actor, ColorKind::Plague);
        }
    
        })
    }

    fn plague_lavender(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        let (amount, refill) = if peer.effects.is_enhanced {
            (1.0, 1000)
        } else {
            (0.2, 200)
        };
        actor.use_energy(
            between(
                amount / self.session.0.parameters.borrow().lifecycle.lavender_energy_consumption,
                0.0,
                actor.state.energy,
            ))?;
        if peer.effects.lavender < self.session.0.parameters.borrow().effects.lavender_shield {
            peer.effects.lavender = peer.effects.lavender.wrapping_add(refill);
            if peer.effects.lavender >= self.session.0.parameters.borrow().effects.lavender_shield {
                peer.effects.lavender = self.session.0.parameters.borrow().effects.lavender_shield;
            }
        }
        if infected_by_actor(actor, peer) {
            peer.infected_genome = None;
            peer.saved_genome = None;
        }
        tint(peer, ColorKind::Lavender);
        tint(actor, ColorKind::DeadBark);
        peer.effects.has_dodged = true;
        if actor.state.energy < TOLERANCE {
            self.die(actor, Some(Killer::Other(peer)))?;
        }
        Ok(())
    }

    /// Force actual viral births through the held peer and actor references before charging the actor
    pub(super) fn touch_plague(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
        _segment: usize,
        peer_segment: usize,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        match color(peer, peer_segment)? {
            White | Plague => {
                if plague_infection(actor, peer)
                    && peer.effects.is_infectious
                    && plague_class(actor, peer)
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
                if peer.effects.indigo > 0
                    && !peer.effects.is_plant
                    && !peer.effects.is_consumer
                    && !peer.effects.is_fungus
                {
                    self.plague_indigo(actor, peer)?;
                }
            }
            Silver => {
                if plague_infection(actor, peer)
                    && (peer.effects.is_enhanced
                        || actor.state.total_infected >= peer.state.total_infected)
                    && plague_class(actor, peer)
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
                if peer.effects.indigo > 0
                    && !peer.effects.is_plant
                    && !peer.effects.is_consumer
                    && !peer.effects.is_fungus
                {
                    self.plague_indigo(actor, peer)?;
                }
            }
            Indigo => {
                if plague_class(actor, peer)
                    || (!peer.effects.is_plant
                        && !peer.effects.is_consumer
                        && !peer.effects.is_fungus)
                {
                    self.plague_indigo(actor, peer)?;
                    if peer.effects.antiviral > 0 && actor.effects.is_infectious {
                        self.virus_neutralized(actor)?;
                    }
                }
            }
            Green | Grass | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade
            | DarkJade => {
                if plague_infection(actor, peer)
                    && actor.effects.plague_version == 1
                    && !self.plague_dodge(actor, peer)?
                    && self.plague_shield(actor, peer)
                {
                    if peer.effects.jade_factor > 1 {
                        peer.body.set_color(peer_segment, if peer.body.color_argb(peer_segment)? == Jade.argb().unwrap() {
                                DarkJade
                            } else {
                                DarkGreen
                            }
                            .color_ref())?;
                        tint(actor, Plague);
                        peer.effects.has_dodged = true;
                        peer.effects.use_extra_effects = true;
                        if peer.effects.antiviral > 0 && actor.effects.is_infectious {
                            self.virus_neutralized(actor)?;
                        }
                    } else {
                        self.plague_birth(actor_ref, actor, peer_ref, peer)?;
                    }
                }
            }
            DarkGreen | Purple => {
                if plague_infection(actor, peer)
                    && actor.effects.plague_version == 1
                    && !self.plague_dodge(actor, peer)?
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Blue => {
                if plague_infection(actor, peer)
                    && actor.effects.plague_version == 1
                    && peer.effects.is_plant
                {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
                        if peer.effects.is_enhanced {
                            actor.use_energy(
                                between(
                                    (0.5 * peer.body.physical_length(peer_segment)?.sqrt())
                                        * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                                    0.0,
                                    actor.state.energy,
                                ))?;
                            tint(actor, DarkLilac);
                            if actor.state.energy < TOLERANCE {
                                self.die(actor, Some(Killer::Other(&mut *peer)))?;
                            }
                        } else {
                            tint(actor, Plague);
                        }
                        tint(peer, Blue);
                    } else {
                        self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                    }
                }
            }
            Ochre | Bark | OldBark => {
                if plague_infection(actor, peer)
                    && actor.effects.plague_version == 1
                    && peer.effects.is_plant
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Sky | DeepSky | Fallow => {
                if ((infected_by_actor(actor, peer) && actor.effects.is_enhanced)
                    || (!actor.effects.is_infectious && peer.infected_genome.is_some()))
                    && actor.effects.plague_version == 1
                    && peer.effects.is_plant
                    && peer.effects.create_lavender == 0
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Maroon | Orange | Fire => {
                if plague_infection(actor, peer)
                    && (actor.effects.plague_version == 2 || !actor.effects.is_infectious)
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Red | Cream => {
                if ((infected_by_actor(actor, peer) && actor.effects.is_enhanced)
                    || (!actor.effects.is_infectious && peer.infected_genome.is_some()))
                    && (actor.effects.plague_version == 2 || !actor.effects.is_infectious)
                    && peer.effects.create_lavender == 0
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Pink => {
                if ((infected_by_actor(actor, peer)
                    && (primary_genome(&peer.genome)?.borrow().modifiers().modifies_pink
                        || (actor.effects.is_auburn
                            && !actor.effects.is_plant
                            && !actor.effects.is_consumer
                            && !actor.effects.is_fungus)))
                    || (!actor.effects.is_infectious && peer.infected_genome.is_some()))
                    && (actor.effects.plague_version == 2 || !actor.effects.is_infectious)
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Spike => {
                if (peer.effects.is_plant || peer.effects.is_enhanced)
                    && plague_infection(actor, peer)
                    && plague_class(actor, peer)
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Lavender => {
                if peer.effects.is_consumer || peer.effects.is_plant {
                    self.plague_lavender(actor, peer)?;
                }
            }
            Spore => {
                if plague_infection(actor, peer) && plague_class(actor, peer) {
                    if peer.effects.spore_version >= 1 && peer.effects.is_consumer {
                        if !self.plague_dodge(actor, peer)? {
                            self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                        }
                    } else {
                        tint(actor, Plague);
                        tint(peer, Spore);
                    }
                }
            }
            Cyan | Teal | Yellow | Auburn | Blond | Flower | DarkGray | Gold => {
                if plague_infection(actor, peer)
                    && plague_class(actor, peer)
                    && !self.plague_dodge(actor, peer)?
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Magenta | Rose => {
                if plague_infection(actor, peer)
                    && !actor.effects.altruist
                    && plague_class(actor, peer)
                    && !self.plague_dodge(actor, peer)?
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
            SpikePoint => {
                if peer.effects.is_enhanced
                    && plague_infection(actor, peer)
                    && plague_class(actor, peer)
                {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Mint | Coral | Olive | Brown => {}
            _ => {
                if plague_infection(actor, peer) && plague_class(actor, peer) {
                    self.plague_shield_birth(actor_ref, actor, peer_ref, peer)?;
                }
            }
        }
        if peer.state.virus_children != 0 {
            let cost = if actor.effects.is_infectious && actor.effects.is_plant {
                self.session.0.parameters.borrow().lifecycle.plague_energy_consumption
            } else {
                self.session.0.parameters.borrow().lifecycle.scourge_energy_consumption
            };
            actor.use_energy(cost)?;
            tint(peer, DarkLilac);
            if actor.effects.is_infectious && peer.effects.antiviral > 0 {
                self.virus_neutralized(actor)?;
            }
            peer.state.virus_children = 0;
        }
        self.plague_fallow_peer_death(actor, peer)?;
        Ok(())
    }

    fn fallow_dodge(&self, actor: &mut Organism, peer: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            tint(peer, ColorKind::Teal);
            tint(actor, ColorKind::Fallow);
            true
        } else {
            false
        }
    
        })
    }

    fn fallow_shield(&self, actor: &mut Organism, peer: &mut Organism) -> bool {
        if peer.effects.lavender > 0 {
            if actor.effects.is_enhanced && !actor.effects.is_infectious {
                self.weak_lavender_shield(peer);
            } else {
                self.lavender_shield(peer);
            }
            tint(actor, ColorKind::Fallow);
        }
        peer.effects.lavender <= 0
    }

    fn fallow_payment(&self, actor: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        actor.use_energy(
            self.session.0.parameters.borrow().lifecycle.fallow_energy_consumption)?
    
        })
    }

    fn fallow_immune_segment(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let update = match color(peer, peer_segment)? {
            Spore if peer.effects.spore_version > 0 => true,
            Auburn | Indigo => true,
            Yellow | Blond => false,
            _ => return Ok(()),
        };
        if self.fallow_payment(actor)? {
            peer.body.set_color(peer_segment, if peer.effects.is_plant {
                Broken
            } else {
                LightBrown
            }
            .color_ref())?;
            tint(actor, Fallow);
            self.restore_contact_effects(peer)?;
            if update {
                peer.body.update_effects = 2;
            }
        }
        Ok(())
    }

    fn fallow_changes(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
        inhibition: Inhibition,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let version = actor.effects.fallow_version;
        if !infected_by_actor(actor, peer) {
            let threshold = if version <= 2 {
                if version == 2 || peer.effects.spore_version > 0 {
                    Some(
                        peer.state
                            .reproduction
                            .maximum_countdown
                            .wrapping_add((10.0 * actor.body.physical_length(segment)?) as i32),
                    )
                } else {
                    None
                }
            } else if version == 4 {
                Some(
                    peer.state
                        .reproduction
                        .maximum_countdown
                        .wrapping_add((5.0 * actor.body.physical_length(segment)?) as i32),
                )
            } else if actor.effects.is_infectious {
                Some(peer.state.reproduction.maximum_countdown)
            } else {
                Some(
                    peer.state
                        .reproduction
                        .maximum_countdown
                        .wrapping_add(actor.body.physical_length(segment)? as i32),
                )
            };
            if let Some(threshold) = threshold {
                if peer.state.reproduction.countdown < threshold && self.fallow_payment(actor)? {
                    tint(actor, Fallow);
                    tint(peer, Flower);
                    peer.state.reproduction.countdown = threshold;
                }
            }
        } else if peer.state.reproduction.countdown > 4 && self.fallow_payment(actor)? {
            tint(actor, Fallow);
            tint(peer, Blond);
            peer.state.reproduction.countdown = 4;
        }
        if version == 3 {
            if peer.effects.reproduce_late <= 0 {
                let threshold = if actor.effects.is_infectious {
                    -2
                } else {
                    ((actor.body.physical_length(segment)? / 2.0 + 1.0) as i32).wrapping_neg()
                };
                if peer.effects.reproduce_late > threshold && self.fallow_payment(actor)? {
                    peer.effects.reproduce_late = threshold;
                    tint(actor, Fallow);
                    tint(peer, Flower);
                }
            }
        } else if version == 4 {
            let eligible = peer.effects.is_reproductive
                && (matches!(inhibition, Inhibition::Reproductive)
                    || peer.effects.is_consumer
                    || peer.effects.is_plant);
            if eligible {
                if peer.effects.antiviral == 0 {
                    self.inhibited_from(peer, ContactActor::Other(actor))?;
                } else if matches!(inhibition, Inhibition::ImmuneSegment) {
                    self.fallow_immune_segment(actor, peer, peer_segment)?;
                }
            }
        } else if version == 1
            && peer.effects.spore_time > 0
            && peer.effects.spore_version > 0
            && self.fallow_payment(actor)?
        {
            peer.effects.spore_time = peer.effects.spore_time.wrapping_neg();
            tint(actor, Fallow);
            tint(peer, Flower);
        }
        Ok(())
    }

    fn fallow_shield_changes(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
        inhibition: Inhibition,
    ) -> Result<(), crate::WorldError> {
        if self.fallow_shield(actor, peer) {
            self.fallow_changes(actor, peer, segment, peer_segment, inhibition)?;
        }
        Ok(())
    }

    fn fallow_lavender(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        let (amount, refill) = if peer.effects.is_enhanced {
            (1.0, 1000)
        } else {
            (0.2, 200)
        };
        actor.use_energy(
            between(
                amount / self.session.0.parameters.borrow().lifecycle.lavender_energy_consumption,
                0.0,
                actor.state.energy,
            ))?;
        if peer.effects.lavender < self.session.0.parameters.borrow().effects.lavender_shield {
            peer.effects.lavender = peer.effects.lavender.wrapping_add(refill);
            if peer.effects.lavender >= self.session.0.parameters.borrow().effects.lavender_shield {
                peer.effects.lavender = self.session.0.parameters.borrow().effects.lavender_shield;
            }
        }
        tint(peer, ColorKind::Lavender);
        tint(actor, ColorKind::DeadBark);
        if actor.state.energy < TOLERANCE {
            self.die(actor, Some(Killer::Other(peer)))?;
        }
        Ok(())
    }

    fn fallow_dormant(&self, actor: &mut Organism, peer: &mut Organism, bark: bool) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        if actor.effects.fallow_version != 2 || !actor.effects.is_infectious {
            self.fallow_shield(actor, peer);
            if (peer.effects.lavender <= 0
                || (!actor.effects.is_plant && !actor.effects.is_infectious)
                || actor.effects.fallow_version == 1)
                && self.fallow_payment(actor)?
            {
                for color_index in peer.body.indices() {
                    let kind = ColorKind::from_argb(peer.body.color_argb(color_index)?);
                    if (bark && matches!(kind, Bark | OldBark))
                        || (!bark && matches!(kind, Brown | Vision))
                    {
                        peer.body.set_color(color_index, if bark { Broken } else { LightBrown }.color_ref())?;
                    }
                }
                tint(actor, Fallow);
                peer.state.temporary_color_frames = 0;
                peer.effects.is_injured = true;
                peer.effects.is_frozen = true;
                peer.effects.all_frozen = true;
            }
        }
        Ok(())
    }

    /// Preserve sequential reproduction delays, targeted inhibition, and dormant phenotype damage
    pub(super) fn touch_fallow(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        if !peer.effects.has_dodged
            && (!peer.effects.is_only_c4 || peer.effects.reproduce_late <= 0)
        {
            peer.effects.has_dodged = true;
        }
        match color(peer, peer_segment)? {
            Fallow => {
                if (peer.effects.is_killer || peer.effects.is_blond)
                    && fallow_relation(actor, peer)
                    && fallow_class(actor, peer)
                {
                    self.fallow_shield_changes(
                        actor,
                        peer,
                        segment,
                        peer_segment,
                        Inhibition::Classified,
                    )?;
                }
            }
            White | Purple | Teal | Cyan | Yellow | Auburn | Indigo | Blond | Spore | Gold => {
                if fallow_relation(actor, peer)
                    && fallow_class(actor, peer)
                    && !self.fallow_dodge(actor, peer)?
                {
                    self.fallow_shield_changes(
                        actor,
                        peer,
                        segment,
                        peer_segment,
                        Inhibition::ImmuneSegment,
                    )?;
                }
            }
            Jade | Green | Grass | Forest | Spring | Summer | Winter | Lime | Leaf | C4
            | DarkJade | DarkGreen => {
                if (peer.effects.jade_factor <= 1
                    || (actor.effects.fallow_version == 1 && peer.effects.spore_version > 0)
                    || !peer.state.active)
                    && fallow_relation(actor, peer)
                    && !self.fallow_dodge(actor, peer)?
                {
                    self.fallow_shield_changes(
                        actor,
                        peer,
                        segment,
                        peer_segment,
                        Inhibition::Reproductive,
                    )?;
                }
            }
            Rose | Magenta => {
                if !actor.effects.altruist
                    && fallow_relation(actor, peer)
                    && fallow_class(actor, peer)
                    && !self.fallow_dodge(actor, peer)?
                    && self.fallow_shield(actor, peer)
                {
                    if peer.body.color_argb(peer_segment)? == Magenta.argb().unwrap()
                        && peer.use_energy(
                            self.session.0.parameters.borrow().lifecycle.magenta_energy_consumption)?
                    {
                        tint(peer, Magenta);
                        tint(actor, Fallow);
                    } else {
                        self.fallow_changes(
                            actor,
                            peer,
                            segment,
                            peer_segment,
                            Inhibition::Classified,
                        )?;
                    }
                }
            }
            Blue => {
                if fallow_relation(actor, peer) && fallow_class(actor, peer) {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
                        tint(actor, Fallow);
                        tint(peer, Blue);
                    } else {
                        self.fallow_shield_changes(
                            actor,
                            peer,
                            segment,
                            peer_segment,
                            Inhibition::Classified,
                        )?;
                    }
                }
            }
            Red => {
                if fallow_relation(actor, peer)
                    && ((actor.effects.is_enhanced && peer.effects.create_lavender == 0)
                        || !actor.effects.is_plant
                        || actor.effects.fallow_version == 1)
                {
                    self.fallow_shield_changes(
                        actor,
                        peer,
                        segment,
                        peer_segment,
                        Inhibition::Reproductive,
                    )?;
                }
            }
            Lavender => {
                if fallow_class(actor, peer) {
                    self.fallow_lavender(actor, peer)?;
                }
            }
            Mint => {
                if (actor.effects.is_plant
                    || actor.effects.is_consumer
                    || actor.effects.is_coral
                    || actor.effects.antiviral == 0
                    || peer.effects.is_plant
                    || peer.effects.is_consumer
                    || peer.effects.is_fungus
                    || peer.effects.is_coral)
                    && peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 10.0)?
                {
                    actor.body.set_color(segment, if actor.effects.is_plant {
                        Broken
                    } else {
                        LightBrown
                    }
                    .color_ref())?;
                    tint(peer, Mint);
                    actor.effects.is_injured = true;
                    actor.body.update_effects = 2;
                }
            }
            Cream | SpikePoint | Olive | Flower | DarkGray | Coral | Fruit => {}
            Brown | Vision => {
                if peer.state.alive && !peer.state.active {
                    self.fallow_dormant(actor, peer, false)?;
                }
            }
            Bark => {
                age_bark(peer, peer_segment)?;
                if !peer.state.active {
                    self.fallow_dormant(actor, peer, true)?;
                }
            }
            _ => {
                if !peer.state.active
                    && peer.body.color_argb(peer_segment)? == OldBark.argb().unwrap()
                {
                    self.fallow_dormant(actor, peer, true)?;
                } else if fallow_relation(actor, peer) && fallow_class(actor, peer) {
                    self.fallow_shield_changes(
                        actor,
                        peer,
                        segment,
                        peer_segment,
                        Inhibition::Classified,
                    )?;
                }
            }
        }
        self.plague_fallow_peer_death(actor, peer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use crate::{
        EffectsState, Gene, Genome, GenomeProperties, JavaRandom, WorldConfig, WorldParameters,
    };
    use std::cell::RefCell;

    fn setup(colors: &[ColorKind]) -> (World, OrganismRef, OrganismRef) {
        let mut parameters = WorldParameters::default();
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
        for (center, colors) in [
            ([1000, 1000], &[ColorKind::Plague][..]),
            ([3500, 3500], colors),
        ] {
            let genes: Vec<_> = colors
                .iter()
                .map(|color| Gene::new(12.0, 0.25, color.argb(), -1, [0; 36]).shared())
                .collect();
            let code = Genome::from_parts(
                &genes,
                GenomeProperties {
                    symmetry: 1,
                    ..GenomeProperties::default()
                },
            );
            world.paste_genome(code, center).unwrap().unwrap();
        }
        let actor = Rc::clone(&world.organisms.borrow()[0]);
        let peer = Rc::clone(&world.organisms.borrow()[1]);
        for reference in [&actor, &peer] {
            let mut organism = reference.borrow_mut();
            organism.effects = EffectsState::default();
            organism.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![0.0; organism.body.forward_count()]));
            organism.effects.has_dodged = false;
            organism.state.energy = 100.0;
            organism.state.temporary_color = None;
            organism.state.temporary_color_frames = 0;
            organism.body.set_physical_length(0, 6.25).unwrap();
        }
        actor.borrow_mut().state.energy = 20.0;
        world.oxygen.set(1000.0);
        world.carbon_dioxide.set(0.0);
        world.rebuild_spatial();
        (world, actor, peer)
    }

    #[test]
    fn plague_uses_code_identity_and_preserves_the_strict_actor_threshold() {
        for shared in [true, false] {
            let (world, actor_ref, peer_ref) = setup(&[ColorKind::Gray]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            actor.effects.is_plant = true;
            actor.effects.is_infectious = true;
            actor.effects.plague_version = 2;
            actor.state.energy = 10.0;
            peer.effects.is_consumer = true;
            peer.effects.lavender = 186;
            peer.infected_genome = Some(if shared {
                Rc::clone(actor.genome.as_ref().unwrap())
            } else {
                Rc::new(RefCell::new(actor.genome.as_ref().unwrap().borrow().clone()))
            });
            let random = world.random_snapshot();
            world
                .touch_plague(&actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0)
                .unwrap();
            assert_eq!(peer.effects.lavender, if shared { 1 } else { 186 });
            assert_eq!(actor.state.energy, 10.0);
            assert_eq!(world.random_snapshot(), random);
            assert_eq!(world.organisms.borrow().len(), 2);
        }
    }

    #[test]
    fn plague_inserts_a_real_child_before_charging_and_clearing_the_birth_marker() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Green]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.is_plant = true;
        actor.effects.is_infectious = true;
        actor.effects.plague_version = 1;
        peer.effects.is_plant = true;
        peer.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        peer.state.reproduction.infected_energy = 43;
        world
            .touch_plague(&actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0)
            .unwrap();
        assert_eq!(world.organisms.borrow().len(), 3);
        assert_eq!(world.population.get(), 3);
        assert_eq!(peer.state.total_children, 1);
        assert_eq!(peer.state.virus_children, 0);
        assert!(peer.infected_genome.is_none() && peer.saved_genome.is_none());
        assert_eq!(actor.state.energy, 10.0);
        assert_eq!(peer.state.temporary_argb(), ColorKind::DarkLilac.argb());
        let child_ref = Rc::clone(&world.organisms.borrow()[2]);
        let child = child_ref.borrow();
        assert_eq!(child.state.parent_id, peer.id as i32);
        assert!(Rc::ptr_eq(
            world.statistics_ref().borrow().get_last_born_being().unwrap(), child.genome.as_ref().unwrap()));
    }

    #[test]
    fn plague_jade_neutralization_still_reaches_the_unconditional_birth_marker_charge() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Jade]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.is_plant = true;
        actor.effects.is_infectious = true;
        actor.effects.plague_version = 1;
        peer.effects.jade_factor = 2;
        peer.effects.antiviral = 1;
        peer.state.virus_children = -1;
        peer.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        world
            .touch_plague(&actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0)
            .unwrap();
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::DarkJade.argb().unwrap()
        );
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::Broken.argb().unwrap()
        );
        assert!(peer.effects.has_dodged && peer.effects.use_extra_effects);
        assert_eq!(peer.state.virus_children, 0);
        assert_eq!(actor.state.energy, 10.0);
        assert_eq!(peer.state.temporary_argb(), ColorKind::DarkLilac.argb());
        assert_eq!(world.organisms.borrow().len(), 2);
    }

    #[test]
    fn plague_noop_color_still_spends_partial_oxygen_for_a_preexisting_birth_marker() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Brown]);
        world.oxygen.set(5.0);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.is_plant = true;
        actor.effects.is_infectious = true;
        peer.state.virus_children = i32::MIN;
        let random = world.random_snapshot();
        world
            .touch_plague(&actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0)
            .unwrap();
        assert_eq!(actor.state.energy, 15.0);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 5.0);
        assert_eq!(peer.state.virus_children, 0);
        assert_eq!(peer.state.temporary_argb(), ColorKind::DarkLilac.argb());
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn plague_lavender_refills_and_clears_only_the_shared_infection_despite_failed_payment() {
        for shared in [true, false] {
            let (world, actor_ref, peer_ref) = setup(&[ColorKind::Lavender]);
            world.oxygen.set(0.0);
            world.session.0.parameters.borrow_mut().effects.lavender_shield = i32::MAX;
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            peer.effects.is_consumer = true;
            peer.effects.lavender = i32::MAX - 100;
            let code = if shared {
                Rc::clone(actor.genome.as_ref().unwrap())
            } else {
                Rc::new(RefCell::new(actor.genome.as_ref().unwrap().borrow().clone()))
            };
            peer.infected_genome = Some(Rc::clone(&code));
            peer.saved_genome = Some(Rc::clone(&code));
            world
                .touch_plague(&actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0)
                .unwrap();
            assert_eq!(peer.effects.lavender, i32::MIN + 99);
            assert_eq!(peer.infected_genome.is_none(), shared);
            assert_eq!(peer.saved_genome.is_none(), shared);
            assert_eq!(actor.state.energy, 20.0);
            assert!(peer.effects.has_dodged);
        }
    }

    #[test]
    fn fallow_owned_infection_accelerates_while_equal_valued_code_remains_a_foreign_target() {
        for shared in [true, false] {
            let (world, actor_ref, peer_ref) = setup(&[ColorKind::Gray]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            actor.effects.fallow_version = 2;
            peer.effects.is_consumer = true;
            peer.state.reproduction.countdown = 8;
            peer.state.reproduction.maximum_countdown = 10;
            peer.infected_genome = Some(if shared {
                Rc::clone(actor.genome.as_ref().unwrap())
            } else {
                Rc::new(RefCell::new(actor.genome.as_ref().unwrap().borrow().clone()))
            });
            world.touch_fallow(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(
                peer.state.reproduction.countdown,
                if shared { 4 } else { 72 }
            );
            assert_eq!(
                peer.state.temporary_argb(),
                if shared {
                    ColorKind::Blond
                } else {
                    ColorKind::Flower
                }
                .argb()
            );
        }
    }

    #[test]
    fn fallow_delay_and_late_reproduction_use_two_nonrefundable_payments() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Gray]);
        world.session.0.parameters.borrow_mut().lifecycle.fallow_energy_consumption = 1.0;
        world.oxygen.set(1.5);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.fallow_version = 3;
        peer.effects.is_consumer = true;
        peer.state.reproduction.maximum_countdown = 10;
        peer.state.reproduction.countdown = 0;
        peer.effects.reproduce_late = 0;
        world.touch_fallow(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.state.reproduction.countdown, 16);
        assert_eq!(peer.effects.reproduce_late, 0);
        assert_eq!(actor.state.energy, 18.5);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 1.5);
    }

    #[test]
    fn fallow_delay_preserves_java_casts_and_wrapping_thresholds() {
        for (length, maximum, before, after) in [
            (6.25, i32::MAX - 2, 0, 0),
            (f64::NAN, 10, 0, 10),
            (f64::INFINITY, 10, 0, 0),
            (f64::NEG_INFINITY, 10, i32::MIN, i32::MIN + 10),
        ] {
            let (world, actor_ref, peer_ref) = setup(&[ColorKind::Gray]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            actor.effects.fallow_version = 2;
            actor.body.set_physical_length(0, length).unwrap();
            peer.effects.is_consumer = true;
            peer.state.reproduction.maximum_countdown = maximum;
            peer.state.reproduction.countdown = before;
            world.touch_fallow(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(peer.state.reproduction.countdown, after);
        }
    }

    #[test]
    fn fallow_antiviral_inhibition_changes_only_the_hit_segment_and_retains_genes() {
        for (hit, update) in [(0, 2), (1, 7)] {
            let (world, actor_ref, peer_ref) = setup(&[ColorKind::Auburn, ColorKind::Yellow]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            actor.effects.fallow_version = 4;
            peer.effects.is_reproductive = true;
            peer.effects.is_plant = true;
            peer.effects.antiviral = 1;
            peer.body.update_effects = 7;
            let original = peer.body.color_argb(1 - hit).unwrap();
            world.touch_fallow(&mut actor, &mut peer, 0, hit).unwrap();
            assert_eq!(
                peer.body.color_argb(hit).unwrap(),
                ColorKind::Broken.argb().unwrap()
            );
            assert_eq!(peer.body.color_argb(1 - hit).unwrap(), original);
            assert_eq!(peer.body.update_effects, update);
            assert_eq!(
                peer.genome.as_ref().unwrap().borrow().gene_ref(hit).unwrap().borrow().color(),
                if hit == 0 {
                    ColorKind::Auburn
                } else {
                    ColorKind::Yellow
                }
                .argb()
            );
        }
    }

    #[test]
    fn fallow_dormant_damage_bypasses_a_remaining_shield_without_restoring_caches() {
        let (world, actor_ref, peer_ref) =
            setup(&[ColorKind::Green, ColorKind::Green, ColorKind::Green]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.fallow_version = 3;
        peer.state.active = false;
        for (index, color) in [
            ColorKind::Brown,
            ColorKind::Vision,
            ColorKind::OldBark,
        ].into_iter().enumerate() {
            peer.body.set_color_argb(index, color.argb().unwrap()).unwrap();
        }
        peer.effects.lavender = 1000;
        peer.effects.photosynthesis = 9.0;
        peer.state.temporary_color = (ColorKind::Red.argb()).map(crate::JavaColor::from_argb);
        peer.state.temporary_color_frames = 8;
        let update = peer.body.update_effects;
        world.touch_fallow(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.effects.lavender, 815);
        assert_eq!(
            (peer.body.indices()).map(|index| peer.body.color_argb(index).unwrap())
                .collect::<Vec<_>>(),
            [
                ColorKind::LightBrown.argb().unwrap(),
                ColorKind::LightBrown.argb().unwrap(),
                ColorKind::OldBark.argb().unwrap()
            ]
        );
        assert_eq!(peer.effects.photosynthesis, 9.0);
        assert_eq!(peer.body.update_effects, update);
        assert_eq!(peer.state.temporary_argb(), ColorKind::LightBlue.argb());
        assert_eq!(peer.state.temporary_color_frames, 0);
        assert!(peer.effects.is_injured && peer.effects.is_frozen && peer.effects.all_frozen);
    }

    #[test]
    fn full_argb_controls_mint_retaliation_and_the_generic_fallow_branch() {
        for opaque in [true, false] {
            let (world, actor_ref, peer_ref) = setup(&[ColorKind::Mint]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            actor.effects.fallow_version = 2;
            actor.effects.is_fungus = true;
            actor.effects.antiviral = 1;
            peer.effects.is_consumer = true;
            peer.state.reproduction.maximum_countdown = 10;
            peer.state.reproduction.countdown = 0;
            if !opaque {
                peer.body.set_color_argb(0, peer.body.color_argb(0).unwrap() & (0x12ff_ffff)).unwrap();
            }
            let before = actor.body.color_argb(0).unwrap();
            world.touch_fallow(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(
                actor.body.color_argb(0).unwrap(),
                if opaque {
                    ColorKind::LightBrown.argb().unwrap()
                } else {
                    before
                }
            );
            assert_eq!(
                peer.state.reproduction.countdown,
                if opaque { 0 } else { 72 }
            );
            assert_eq!(actor.effects.is_injured, opaque);
        }
    }

    #[test]
    fn both_noop_contacts_keep_the_strict_alive_peer_death_guard() {
        for plague in [true, false] {
            for energy in [TOLERANCE, 0.0] {
                let (world, actor_ref, peer_ref) = setup(&[ColorKind::Olive]);
                let mut actor = actor_ref.borrow_mut();
                let mut peer = peer_ref.borrow_mut();
                peer.state.energy = energy;
                let random = JavaRandom::new(0);
                world.session.0.random.replace(random.clone());
                if plague {
                    world
                        .touch_plague(&actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0)
                        .unwrap();
                } else {
                    world.touch_fallow(&mut actor, &mut peer, 0, 0).unwrap();
                }
                assert_eq!(peer.state.alive, energy == TOLERANCE);
                assert_eq!(
                    actor.state.total_kills,
                    if energy == TOLERANCE { 0 } else { 1 }
                );
                assert_eq!(world.random_snapshot(), random);
            }
        }
    }
}
