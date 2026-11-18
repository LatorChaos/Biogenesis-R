// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Complete WHITE and FRUIT C2 branches from Organism.java:13120-14141
//! Both named methods include the shared peer death guard at Organism.java:17916-17919

use crate::organism::primary_genome;

use super::death::Killer;
use super::draining_contacts::age_bark;
use super::infection::{InfectionKind, Infector};
use super::{Organism, World};
use crate::{ColorKind, TOLERANCE, physics::between};
use crate::organism::same_genome;

fn color(organism: &Organism, segment: usize) -> Result<ColorKind, crate::WorldError> {
    Ok(organism.body.color_kind(segment)?)
}

fn tint(organism: &mut Organism, color: ColorKind) {
    organism.set_display_color(Some(color.color_ref().expect("Tint requires a named source literal")));
}

fn infected_with_actor(actor: &Organism, peer: &Organism) -> bool {
    same_genome(peer.infected_genome.as_ref(), actor.genome.as_ref())
}

fn classified_host(peer: &Organism) -> bool {
    peer.effects.is_plant || peer.effects.is_consumer
}

fn pure_auburn(actor: &Organism) -> bool {
    actor.effects.is_auburn
        && !actor.effects.is_plant
        && !actor.effects.is_consumer
        && !actor.effects.is_fungus
}

fn enhanced_white_bypass(actor: &Organism, peer: &Organism) -> bool {
    actor.effects.is_enhanced
        && peer.effects.is_plant
        && !actor.effects.is_killer
        && peer.effects.antiviral == 0
}

fn white_sky_eligible(actor: &Organism, peer: &Organism) -> bool {
    ((peer.effects.sky_version == 2
        || actor.effects.is_plant
        || actor.effects.is_consumer
        || actor.effects.is_fungus)
        && (!actor.effects.is_frozen || actor.effects.healing > 0))
        || enhanced_white_bypass(actor, peer)
}

fn fruit_host_available(peer: &Organism) -> bool {
    peer.infected_genome.is_some() || peer.effects.spore_version != 1
}

impl World {
    fn try_white_infection(&self, actor: &mut Organism, peer: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        let cost = if actor.effects.is_plant {
            self.session.0.parameters.borrow().lifecycle.white_energy_consumption
        } else {
            self.session.0.parameters.borrow().lifecycle.virus_energy_consumption
        };
        if actor.use_energy(cost)? {
            self.infect_from(peer, Infector::Other(&mut *actor), InfectionKind::Direct)?;
            tint(
                peer,
                if actor.effects.is_plant {
                    ColorKind::Blond
                } else {
                    ColorKind::LightBrown
                },
            );
            tint(actor, ColorKind::White);
            true
        } else {
            false
        }
    
        })
    }

    fn white_indigo_penalty(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        Ok({
        let cost = if actor.effects.is_plant {
            (self.session.0.parameters.borrow().lifecycle.white_energy_consumption + 0.01)
                / self.session.0.parameters.borrow().lifecycle.indigo_energy_consumption
        } else if actor.effects.indigo == 0 || actor.effects.is_consumer || actor.effects.is_fungus
        {
            (self.session.0.parameters.borrow().lifecycle.virus_energy_consumption + 0.1)
                / self.session.0.parameters.borrow().lifecycle.indigo_energy_consumption
        } else {
            return Ok(());
        };
        if actor.use_energy(cost)? {
            tint(peer, ColorKind::Indigo);
            tint(actor, ColorKind::White);
        }
    
        })
    }

    fn white_dodge(&self, actor: &mut Organism, peer: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            // Inspect the old timer before tinting resets it to ten
            if !peer.effects.is_only_c4
                || (!actor.effects.is_plant
                    && (!peer.effects.is_auburn || peer.state.temporary_color_frames > 8))
                || (peer.state.temporary_color_frames > 8 && !peer.effects.is_auburn)
            {
                peer.effects.has_dodged = true;
            }
            tint(peer, ColorKind::Teal);
            tint(actor, ColorKind::White);
            true
        } else {
            false
        }
    
        })
    }

    fn fruit_dodge(&self, peer: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            if !peer.effects.is_only_c4 || peer.state.temporary_color_frames > 8 {
                peer.effects.has_dodged = true;
            }
            tint(peer, ColorKind::Teal);
            true
        } else {
            false
        }
    
        })
    }

    fn lavender_white_fruit_defence(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        amount: f64,
        increment: i32,
        clear_infection: bool,
    ) -> Result<(), crate::WorldError> {
        let spent = between(amount, 0.0, actor.state.energy);
        actor.use_energy(spent)?;
        if peer.effects.lavender < self.session.0.parameters.borrow().effects.lavender_shield {
            peer.effects.lavender = peer.effects.lavender.wrapping_add(increment);
            if peer.effects.lavender >= self.session.0.parameters.borrow().effects.lavender_shield {
                peer.effects.lavender = self.session.0.parameters.borrow().effects.lavender_shield;
            }
        }
        if clear_infection && peer.infected_genome.is_some() {
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

    fn finish_white_fruit_contact(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        if peer.state.alive && peer.state.energy < TOLERANCE {
            self.die(peer, Some(Killer::Other(actor)))?;
        }
        Ok(())
    }

    pub(super) fn touch_white(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let different = !infected_with_actor(actor, peer);
        match color(peer, peer_segment)? {
            White | GreenBrown => {
                if different {
                    if classified_host(peer) {
                        self.try_white_infection(actor, peer)?;
                    } else if peer.effects.indigo > 0 && !peer.effects.is_fungus {
                        self.white_indigo_penalty(actor, peer)?;
                    }
                }
            }
            Silver => {
                if different {
                    if classified_host(peer)
                        || (peer.effects.is_fungus && peer.state.total_kills > 0)
                    {
                        if peer.effects.is_enhanced
                            || actor.state.total_infected >= peer.state.total_infected
                        {
                            self.try_white_infection(actor, peer)?;
                        }
                    } else if peer.effects.indigo > 0 && !peer.effects.is_fungus {
                        self.white_indigo_penalty(actor, peer)?;
                    }
                }
            }
            Plague => {
                if different {
                    if classified_host(peer) {
                        if self.try_white_infection(actor, peer)?
                            && !peer.effects.is_infectious
                            && !peer.effects.is_consumer
                            && !peer.effects.is_fungus
                        {
                            peer.state.reproduction.infected_energy = 4;
                        }
                    } else if peer.effects.indigo > 0 && !peer.effects.is_fungus {
                        self.white_indigo_penalty(actor, peer)?;
                    }
                }
            }
            Indigo => {
                self.white_indigo_penalty(actor, peer)?;
                if peer.effects.antiviral > 0 && actor.effects.plague_version > 0 {
                    self.virus_neutralized(actor)?;
                }
            }
            Green | Grass | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade
            | DarkJade => {
                if different && !self.white_dodge(actor, peer)? {
                    if peer.effects.jade_factor > 1 {
                        peer.body.set_color(peer_segment, if color(peer, peer_segment)? == Jade
                        {
                            DarkJade.color_ref()
                        } else {
                            DarkGreen.color_ref()
                        })?;
                        tint(actor, White);
                        peer.effects.has_dodged = true;
                        peer.effects.use_extra_effects = true;
                        if peer.effects.antiviral > 0 && actor.effects.plague_version > 0 {
                            self.virus_neutralized(actor)?;
                        }
                    } else {
                        self.try_white_infection(actor, peer)?;
                    }
                }
            }
            DarkGreen | Purple | Teal | Cyan | Yellow | Auburn | Blond | Flower | DarkGray
            | Gold => {
                if different && classified_host(peer) && !self.white_dodge(actor, peer)? {
                    self.try_white_infection(actor, peer)?;
                }
            }
            Spore => {
                if different && classified_host(peer) {
                    if peer.effects.spore_version >= 3 && peer.effects.is_plant {
                        if !self.white_dodge(actor, peer)?
                            && self.try_white_infection(actor, peer)?
                            && !peer.effects.is_consumer
                            && !peer.effects.is_fungus
                        {
                            peer.state.reproduction.infected_energy =
                                peer.state.reproduction.infected_energy.wrapping_neg();
                        }
                    } else {
                        tint(actor, White);
                        tint(peer, Spore);
                    }
                }
            }
            Blue => {
                if different && classified_host(peer) {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
                        if peer.effects.is_enhanced {
                            let amount = between(
                                crate::java_math::mul(
                                    crate::java_math::mul(0.5, crate::java_math::sqrt(peer.body.physical_length(peer_segment)?)),
                                    self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                                ),
                                0.0,
                                actor.state.energy,
                            );
                            actor.use_energy(amount)?;
                            tint(actor, DarkLilac);
                            if actor.state.energy < TOLERANCE {
                                self.die(actor, Some(Killer::Other(&mut *peer)))?;
                            }
                        } else {
                            tint(actor, White);
                        }
                        tint(peer, Blue);
                        peer.effects.has_dodged = true;
                    } else {
                        self.try_white_infection(actor, peer)?;
                    }
                }
            }
            LightBlue => {
                if different && classified_host(peer) {
                    if !actor.effects.is_enhanced
                        && actor.effects.is_plant
                        && peer.use_energy(
                            self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)?
                    {
                        tint(actor, White);
                        tint(peer, LightBlue);
                        peer.effects.has_dodged = true;
                    } else {
                        self.try_white_infection(actor, peer)?;
                    }
                }
            }
            Sky => {
                if white_sky_eligible(actor, peer)
                    && (peer.effects.is_plant
                        || (peer.effects.is_consumer && peer.effects.sky_version == 2))
                {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                        peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                        peer.effects.use_extra_effects = true;
                        tint(actor, White);
                        tint(peer, DeepSky);
                        peer.effects.has_dodged = true;
                    } else if different {
                        self.try_white_infection(actor, peer)?;
                    }
                }
            }
            DeepSky => {
                if white_sky_eligible(actor, peer)
                    && (peer.effects.is_plant
                        || (peer.effects.is_consumer && peer.effects.sky_version == 2))
                {
                    if enhanced_white_bypass(actor, peer) {
                        if different {
                            self.try_white_infection(actor, peer)?;
                        }
                    } else {
                        tint(actor, White);
                        tint(peer, DeepSky);
                        peer.effects.has_dodged = true;
                    }
                }
            }
            Olive | DarkOlive => {
                if different
                    && classified_host(peer)
                    && (!actor.effects.is_plant || enhanced_white_bypass(actor, peer))
                {
                    self.try_white_infection(actor, peer)?;
                }
            }
            Violet => {
                if different {
                    if classified_host(peer) {
                        if !actor.effects.is_plant
                            || peer.effects.is_consumer
                            || (actor.effects.is_enhanced
                                && !actor.effects.is_killer
                                && peer.effects.antiviral == 0)
                        {
                            self.try_white_infection(actor, peer)?;
                        }
                    } else if peer.effects.is_fungus
                        && !peer.effects.is_silver
                        && pure_auburn(actor)
                    {
                        self.try_white_infection(actor, peer)?;
                    }
                }
            }
            LightBrown => {
                if different
                    && (classified_host(peer)
                        || (peer.effects.is_fungus
                            && !peer.effects.is_silver
                            && pure_auburn(actor)))
                {
                    self.try_white_infection(actor, peer)?;
                }
            }
            Ice | DeadBark => {
                if different
                    && classified_host(peer)
                    && (!actor.effects.is_plant
                        || peer.effects.is_consumer
                        || (actor.effects.is_enhanced && !actor.effects.is_killer))
                {
                    self.try_white_infection(actor, peer)?;
                }
            }
            Lavender => {
                if classified_host(peer) {
                    let (numerator, increment) = if actor.effects.plague_version > 0
                        || actor.effects.fallow_version > 0
                        || peer.effects.is_enhanced
                    {
                        if peer.effects.is_enhanced {
                            (1.0, 1000)
                        } else {
                            (0.2, 200)
                        }
                    } else {
                        (0.02, 20)
                    };
                    self.lavender_white_fruit_defence(
                        actor,
                        peer,
                        numerator / self.session.0.parameters.borrow().lifecycle.lavender_energy_consumption,
                        increment,
                        true,
                    )?;
                }
            }
            Magenta | Rose => {
                if !actor.effects.altruist
                    && different
                    && classified_host(peer)
                    && !self.white_dodge(actor, peer)?
                {
                    self.try_white_infection(actor, peer)?;
                }
            }
            Pink => {
                if (primary_genome(&peer.genome)?.borrow().modifiers().modifies_pink
                    || (peer.state.reproduction.countdown
                        > peer.state.reproduction.maximum_countdown
                        && peer.effects.reproduce_late <= 0)
                    || pure_auburn(actor))
                    && different
                {
                    self.try_white_infection(actor, peer)?;
                }
            }
            Cream => {
                if peer.effects.indigo == 0 && different {
                    if peer.effects.is_plant {
                        if !actor.effects.is_plant
                            || (actor.effects.is_enhanced
                                && !actor.effects.is_killer
                                && (!peer.effects.is_enhanced
                                    || primary_genome(&peer.genome)?.borrow().modifiers().cream != 2))
                        {
                            self.try_white_infection(actor, peer)?;
                        }
                    } else if actor.effects.is_enhanced
                        && !actor.effects.is_killer
                        && !actor.effects.is_plant
                        && !actor.effects.is_consumer
                        && !actor.effects.is_fungus
                        && (!peer.effects.is_enhanced
                            || primary_genome(&peer.genome)?.borrow().modifiers().cream != 2)
                    {
                        self.try_white_infection(actor, peer)?;
                    }
                }
            }
            Spike => {
                if different && (peer.effects.is_plant || peer.effects.is_enhanced) {
                    self.try_white_infection(actor, peer)?;
                }
            }
            SpikePoint => {
                if actor.effects.is_enhanced
                    && !actor.effects.is_killer
                    && ((peer.effects.is_plant && peer.effects.antiviral == 0)
                        || peer.effects.is_enhanced)
                    && different
                {
                    self.try_white_infection(actor, peer)?;
                }
            }
            Coral => {
                if different
                    && classified_host(peer)
                    && (actor.effects.is_plant
                        || actor.effects.is_consumer
                        || !actor.effects.is_blond)
                {
                    self.try_white_infection(actor, peer)?;
                }
            }
            Fallow => {
                if enhanced_white_bypass(actor, peer)
                    && !actor.effects.is_plant
                    && !actor.effects.is_consumer
                    && !actor.effects.is_fungus
                    && different
                    && classified_host(peer)
                {
                    self.try_white_infection(actor, peer)?;
                }
            }
            Bark => age_bark(peer, peer_segment)?,
            OldBark | Ochre | Mint | Brown => {}
            // The original default infects only plant/consumer hosts with a different code
            NoColor | Red | Fire | Orange | Maroon | Lilac | Gray | Dark | Eye | DarkFire
            | DarkLilac | Fruit | Vision | Broken => {
                if different && classified_host(peer) {
                    self.try_white_infection(actor, peer)?;
                }
            }
        }
        self.finish_white_fruit_contact(actor, peer)?;
        Ok(())
    }

    fn fruit_donation(&self, actor: &mut Organism, peer: &mut Organism) {
        let amount = between(crate::java_math::sub(actor.state.energy, 0.0000002), 0.0, actor.state.energy);
        peer.state.energy = crate::java_math::add(peer.state.energy, amount);
        actor.state.energy = crate::java_math::sub(actor.state.energy, amount);
        tint(peer, ColorKind::LightBrown);
        peer.state.reproduction.countdown = 0;
    }

    fn infect_fruit_host(&self, actor: &mut Organism, peer: &mut Organism, weak: bool) -> Result<(), crate::WorldError> {
        self.infect_from(
            peer,
            Infector::Other(&mut *actor),
            if weak {
                InfectionKind::WeakFruit
            } else {
                InfectionKind::Fruit
            },
        )?;
        self.fruit_donation(actor, peer);
        Ok(())
    }

    pub(super) fn touch_fruit(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let receptive = !infected_with_actor(actor, peer) && actor.effects.spore_version == 1;
        match color(peer, peer_segment)? {
            Green | Grass | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade
            | DarkJade => {
                if receptive && fruit_host_available(peer) && !self.fruit_dodge(peer)? {
                    if peer.effects.antiviral > 0
                        && peer.use_energy(
                            self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 10.0)?
                    {
                        for color_index in actor.body.indices() {
                            if matches!(ColorKind::from_argb(actor.body.color_argb(color_index)?), Fruit | DarkFire) {
                                actor.body.set_color(color_index, Broken.color_ref())?;
                            }
                        }
                        tint(peer, Mint);
                        actor.state.temporary_color_frames = 0;
                        actor.effects.is_injured = true;
                        actor.effects.is_frozen = true;
                        actor.effects.all_frozen = true;
                    } else if peer.effects.jade_factor > 1 {
                        peer.body.set_color(peer_segment, if color(peer, peer_segment)? == Jade
                        {
                            DarkJade.color_ref()
                        } else {
                            DarkGreen.color_ref()
                        })?;
                        peer.effects.has_dodged = true;
                        peer.effects.use_extra_effects = true;
                    } else {
                        self.infect_fruit_host(actor, peer, false)?;
                    }
                }
            }
            DarkGreen | Purple | Teal | Cyan | Yellow | Auburn | Blond | Flower | DarkGray
            | Gold => {
                if receptive
                    && peer.effects.is_plant
                    && fruit_host_available(peer)
                    && !self.fruit_dodge(peer)?
                {
                    self.infect_fruit_host(actor, peer, false)?;
                }
            }
            Spore => {
                if receptive && peer.effects.is_plant {
                    if fruit_host_available(peer) && peer.effects.spore_version > 0 {
                        if !self.fruit_dodge(peer)? {
                            self.infect_fruit_host(actor, peer, false)?;
                            if !peer.effects.is_consumer
                                && !peer.effects.is_fungus
                                && peer.effects.spore_version >= 3
                            {
                                peer.state.reproduction.infected_energy =
                                    peer.state.reproduction.infected_energy.wrapping_neg();
                            }
                        }
                    } else if peer.effects.spore_version != 1 {
                        tint(peer, Spore);
                    }
                }
            }
            Indigo => {
                if peer.effects.is_plant && actor.effects.spore_version == 1 {
                    let amount = between(crate::java_math::sub(actor.state.energy, 0.0000002), 0.0, actor.state.energy);
                    peer.state.energy = crate::java_math::add(peer.state.energy, amount);
                    actor.state.energy = crate::java_math::sub(actor.state.energy, amount);
                    tint(peer, Indigo);
                    peer.state.reproduction.countdown = 0;
                }
            }
            Magenta | Rose => {
                if receptive
                    && !actor.effects.altruist
                    && peer.effects.is_plant
                    && fruit_host_available(peer)
                    && !self.fruit_dodge(peer)?
                {
                    self.infect_fruit_host(actor, peer, false)?;
                }
            }
            Silver => {
                if receptive
                    && !((peer.state.total_kills > 0
                        && (peer.effects.is_consumer || peer.effects.is_fungus))
                        || peer.effects.is_enhanced)
                    && peer.effects.is_plant
                    && fruit_host_available(peer)
                {
                    self.infect_fruit_host(actor, peer, false)?;
                }
            }
            Sky => {
                if peer.effects.sky_version == 2
                    && receptive
                    && peer.effects.is_plant
                    && fruit_host_available(peer)
                {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                        peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                        peer.effects.use_extra_effects = true;
                        tint(peer, DeepSky);
                    } else {
                        let weak = !(actor.effects.is_enhanced && !actor.effects.is_blond)
                            && !peer.effects.is_blond;
                        self.infect_fruit_host(actor, peer, weak)?;
                    }
                }
            }
            Spike | DeepSky | Olive | DarkOlive | Bark | OldBark | Ochre => {
                if receptive && peer.effects.is_plant && fruit_host_available(peer) {
                    let weak = !(actor.effects.is_enhanced && !actor.effects.is_blond)
                        && !peer.effects.is_blond;
                    self.infect_fruit_host(actor, peer, weak)?;
                }
            }
            Plague => {
                if receptive && peer.effects.is_plant && fruit_host_available(peer) {
                    let weak = !peer.effects.is_infectious
                        && !peer.effects.is_consumer
                        && !peer.effects.is_fungus;
                    self.infect_from(
                        peer,
                        Infector::Other(&mut *actor),
                        if weak {
                            InfectionKind::WeakFruit
                        } else {
                            InfectionKind::Fruit
                        },
                    )?;
                    if weak {
                        peer.state.reproduction.infected_energy = 4;
                    }
                    self.fruit_donation(actor, peer);
                }
            }
            Lavender => {
                if receptive && peer.effects.is_plant && fruit_host_available(peer) {
                    let (numerator, increment) = if peer.effects.is_enhanced {
                        (1.0, 1000)
                    } else {
                        (0.2, 200)
                    };
                    self.lavender_white_fruit_defence(
                        actor,
                        peer,
                        numerator / self.session.0.parameters.borrow().lifecycle.lavender_energy_consumption,
                        increment,
                        false,
                    )?;
                }
            }
            Red | Orange | Fire | Maroon | Pink | Cream | SpikePoint | Blue | Violet | Coral
            | Mint | Brown => {}
            // The original default donates to other receptive plant hosts
            NoColor | Lilac | Gray | Fallow | White | Dark | Eye | DarkFire | DarkLilac | Fruit
            | Vision | Ice | LightBlue | LightBrown | GreenBrown | Broken | DeadBark => {
                if receptive && peer.effects.is_plant && fruit_host_available(peer) {
                    self.infect_fruit_host(actor, peer, false)?;
                }
            }
        }
        self.finish_white_fruit_contact(actor, peer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use crate::{Gene, Genome, GenomeProperties, OrganismState, WorldConfig};
    use std::cell::RefCell;

    fn organism(world: &World, colors: &[ColorKind]) -> Organism {
        let genes: Vec<_> = colors
            .iter()
            .map(|color| Gene::new(12.0, 0.0, color.argb(), 0, [0; 36]).shared())
            .collect();
        let code = Genome::from_parts(
            &genes,
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        let mut organism =
            Organism::unplaced(Rc::new(RefCell::new(code)), OrganismState { world: world.association(), ..OrganismState::default() }).unwrap();
        organism.state.energy = 10.0;
        organism.state.reproduction.energy = 100;
        organism.effects.has_dodged = false;
        organism
    }

    fn world() -> World {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        world.session.0.parameters.borrow_mut().lifecycle.white_energy_consumption = 1.0;
        world.session.0.parameters.borrow_mut().lifecycle.virus_energy_consumption = 0.5;
        world.session.0.parameters.borrow_mut().lifecycle.indigo_energy_consumption = 2.0;
        world.session.0.parameters.borrow_mut().lifecycle.lavender_energy_consumption = 1.0;
        world.session.0.parameters.borrow_mut().lifecycle.dodge_energy_consumption = 1.0;
        world.session.0.parameters.borrow_mut().lifecycle.mint_energy_consumption = 1.0;
        world.oxygen.set(10.0);
        world.carbon_dioxide.set(0.0);
        world
    }

    #[test]
    fn identical_code_skips_infection_but_indigo_still_charges_and_neutralizes_plague() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::White]);
        let mut peer = organism(&world, &[ColorKind::Green]);
        actor.effects.is_plant = true;
        peer.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        world.touch_white(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(actor.state.energy, 10.0);
        assert_eq!(actor.state.total_infected, 0);
        peer.body.set_color_argb(0, ColorKind::Indigo.argb().unwrap()).unwrap();
        peer.effects.antiviral = 1;
        actor.effects.plague_version = 1;
        world.session.0.parameters.borrow_mut().lifecycle.white_energy_consumption = 0.99;
        world.touch_white(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(actor.state.energy, 9.5);
        assert_eq!(world.carbon_dioxide.get(), 0.5);
        assert_eq!(peer.state.temporary_argb(), ColorKind::Indigo.argb());
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::Broken.argb().unwrap()
        );
        assert_eq!(actor.state.total_infected, 0);
    }

    #[test]
    fn value_equal_distinct_code_does_not_suppress_direct_infection() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::White]);
        let mut peer = organism(&world, &[ColorKind::Green]);
        actor.effects.is_plant = true;
        let distinct = Rc::new(RefCell::new(actor.genome.as_ref().unwrap().borrow().clone()));
        assert!(!Rc::ptr_eq(&distinct, actor.genome.as_ref().unwrap()));
        assert_eq!(*distinct.borrow(), *actor.genome.as_ref().unwrap().borrow());
        peer.infected_genome = Some(distinct);
        world.touch_white(&mut actor, &mut peer, 0).unwrap();
        assert!(Rc::ptr_eq(
            peer.infected_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
        assert_eq!(actor.state.total_infected, 1);
        assert_eq!(actor.state.energy, 9.0);
    }

    #[test]
    fn white_spore_infection_preserves_java_negation_and_counter_wrapping() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::White]);
        let mut peer = organism(&world, &[ColorKind::Spore]);
        actor.effects.is_plant = true;
        actor.state.reproduction.energy = i32::MIN;
        actor.state.total_infected = i32::MAX;
        peer.effects.is_plant = true;
        peer.effects.spore_version = 3;
        peer.saved_genome = Some(Rc::clone(peer.genome.as_ref().unwrap()));
        world.touch_white(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(peer.state.reproduction.infected_energy, i32::MIN);
        assert_eq!(actor.state.total_infected, i32::MIN);
        assert!(peer.saved_genome.is_none());
        assert!(Rc::ptr_eq(
            peer.infected_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
    }

    #[test]
    fn white_dodge_reads_the_old_timer_and_partial_payment_precedes_jade_defence() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::White]);
        let mut peer = organism(&world, &[ColorKind::Green]);
        actor.effects.is_plant = true;
        peer.effects.dodge = true;
        peer.effects.is_only_c4 = true;
        peer.effects.is_auburn = true;
        peer.state.temporary_color_frames = 0;
        world.touch_white(&mut actor, &mut peer, 0).unwrap();
        assert!(!peer.effects.has_dodged);
        assert_eq!(peer.state.temporary_color_frames, 10);
        assert_eq!(peer.state.energy, 9.0);
        assert!(peer.infected_genome.is_none());
        let mut actor = organism(&world, &[ColorKind::White]);
        let mut peer = organism(&world, &[ColorKind::Jade]);
        peer.effects.dodge = true;
        peer.effects.jade_factor = 2;
        world.oxygen.set(0.5);
        let carbon = world.carbon_dioxide.get();
        world.touch_white(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(actor.state.energy, 10.0);
        assert_eq!(peer.state.energy, 9.5);
        assert_eq!(world.carbon_dioxide.get(), carbon + 0.5);
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::DarkJade.argb().unwrap()
        );
        assert!(peer.effects.has_dodged && peer.effects.use_extra_effects);
    }

    #[test]
    fn white_lavender_wraps_after_failed_payment_and_retains_saved_only_codes() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::White]);
        let mut peer = organism(&world, &[ColorKind::Lavender]);
        actor.state.energy = 0.25;
        peer.effects.is_plant = true;
        peer.effects.is_enhanced = true;
        peer.effects.lavender = i32::MAX - 500;
        peer.saved_genome = Some(Rc::clone(peer.genome.as_ref().unwrap()));
        world.session.0.parameters.borrow_mut().effects.lavender_shield = i32::MAX;
        world.session.0.parameters.borrow_mut().lifecycle.lavender_energy_consumption = 0.0;
        world.oxygen.set(0.0);
        world.touch_white(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(actor.state.energy, 0.25);
        assert_eq!(peer.effects.lavender, i32::MIN + 499);
        assert!(peer.saved_genome.is_some());
        assert_eq!(peer.state.temporary_argb(), ColorKind::Lavender.argb());
        assert_eq!(actor.state.temporary_argb(), ColorKind::DeadBark.argb());
        assert!(peer.effects.has_dodged);
    }

    #[test]
    fn fruit_antiviral_defence_preserves_geometry_reactions_and_unrelated_cached_flags() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Fruit, ColorKind::DarkFire, ColorKind::Eye]);
        let mut peer = organism(&world, &[ColorKind::Green]);
        actor.effects.spore_version = 1;
        actor.effects.can_move = true;
        actor.effects.can_react = true;
        actor.effects.use_frame_movement = true;
        actor.effects.use_extra_effects = false;
        actor.body.update_effects = 1;
        actor.state.temporary_color = (ColorKind::Red.argb()).map(crate::JavaColor::from_argb);
        actor.state.temporary_color_frames = 7;
        peer.effects.antiviral = 1;
        let body = actor.body.clone();
        let rotated = actor.body.rotated_values().unwrap();
        world.touch_fruit(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(
            (actor.body.indices()).map(|index| actor.body.color_argb(index).unwrap())
                .collect::<Vec<_>>(),
            vec![
                ColorKind::Broken.argb().unwrap(),
                ColorKind::Broken.argb().unwrap(),
                ColorKind::Eye.argb().unwrap()
            ]
        );
        assert_eq!(actor.body.rotated_values().unwrap(), rotated);
        assert_eq!(actor.body.mass.to_bits(), body.mass.to_bits());
        assert_eq!(actor.body.inertia.to_bits(), body.inertia.to_bits());
        for (actual, previous) in actor.body.segment_values().unwrap().into_iter().zip(body.segment_values().unwrap()) {
            assert_eq!(actual.reactions, previous.reactions);
            assert_eq!((actual.start, actual.end), (previous.start, previous.end));
        }
        assert_eq!(actor.body.update_effects, 1);
        assert!(
            actor.effects.can_move && actor.effects.can_react && actor.effects.use_frame_movement
        );
        assert!(!actor.effects.use_extra_effects);
        assert!(actor.effects.is_frozen && actor.effects.all_frozen && actor.effects.is_injured);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Red.argb());
        assert_eq!(actor.state.temporary_color_frames, 0);
        assert_eq!(peer.state.energy, 9.9);
    }

    #[test]
    fn fruit_green_nonplant_host_infects_before_donation_and_keeps_the_literal_reserve() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Fruit]);
        let mut peer = organism(&world, &[ColorKind::Green]);
        actor.effects.spore_version = 1;
        actor.effects.spore_time = 2;
        actor.state.energy = 0.0000004;
        peer.state.energy = 0.0;
        peer.state.reproduction.countdown = 42;
        let random = world.random_snapshot();
        world.touch_fruit(&mut actor, &mut peer, 0).unwrap();
        assert!(!peer.effects.is_plant);
        assert_eq!(peer.state.reproduction.infected_energy, 109);
        assert_eq!(peer.state.energy.to_bits(), 0.0000002_f64.to_bits());
        assert_eq!(actor.state.energy.to_bits(), 0.0000002_f64.to_bits());
        assert_eq!(peer.state.reproduction.countdown, 0);
        assert_eq!(peer.state.temporary_argb(), ColorKind::LightBrown.argb());
        assert!(Rc::ptr_eq(
            peer.infected_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn fruit_spore_negation_wraps_and_zero_version_only_tints() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Fruit]);
        let mut peer = organism(&world, &[ColorKind::Spore]);
        actor.effects.spore_version = 1;
        actor.state.energy = 0.0000004;
        peer.effects.is_plant = true;
        peer.effects.spore_version = 3;
        peer.state.reproduction.energy = i32::MIN;
        world.touch_fruit(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(peer.state.reproduction.infected_energy, i32::MIN);
        let mut actor = organism(&world, &[ColorKind::Fruit]);
        let mut peer = organism(&world, &[ColorKind::Spore]);
        actor.effects.spore_version = 1;
        peer.effects.is_plant = true;
        peer.effects.spore_version = 0;
        peer.state.reproduction.countdown = 42;
        world.touch_fruit(&mut actor, &mut peer, 0).unwrap();
        assert_eq!((actor.state.energy, peer.state.energy), (10.0, 10.0));
        assert!(peer.infected_genome.is_none());
        assert_eq!(peer.state.temporary_argb(), ColorKind::Spore.argb());
        assert_eq!(peer.state.reproduction.countdown, 42);
    }

    #[test]
    fn lavender_clears_infection_only_in_the_white_branch() {
        let world = world();
        let mut fruit = organism(&world, &[ColorKind::Fruit]);
        let mut peer = organism(&world, &[ColorKind::Lavender]);
        fruit.effects.spore_version = 1;
        peer.effects.is_plant = true;
        peer.infected_genome = Some(Rc::clone(peer.genome.as_ref().unwrap()));
        peer.saved_genome = Some(Rc::clone(peer.genome.as_ref().unwrap()));
        world.touch_fruit(&mut fruit, &mut peer, 0).unwrap();
        assert!(Rc::ptr_eq(
            peer.infected_genome.as_ref().unwrap(), peer.genome.as_ref().unwrap()));
        assert!(peer.saved_genome.is_some());
        let mut white = organism(&world, &[ColorKind::White]);
        world.touch_white(&mut white, &mut peer, 0).unwrap();
        assert!(peer.infected_genome.is_none() && peer.saved_genome.is_none());
    }

    #[test]
    fn enhanced_blond_fruit_stays_weak_unless_the_host_is_blond() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Fruit]);
        let mut peer = organism(&world, &[ColorKind::Spike]);
        actor.effects.spore_version = 1;
        actor.effects.is_enhanced = true;
        actor.effects.is_blond = true;
        actor.state.energy = 0.0000004;
        peer.effects.is_plant = true;
        world.touch_fruit(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(peer.state.reproduction.infected_energy, 0);
        peer.effects.is_blond = true;
        peer.infected_genome = None;
        actor.state.energy = 0.0000004;
        world.touch_fruit(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(peer.state.reproduction.infected_energy, 100);
    }

    #[test]
    fn indigo_accepts_infinite_fruit_energy_without_infection_or_self_death() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Fruit]);
        let mut peer = organism(&world, &[ColorKind::Indigo]);
        actor.effects.spore_version = 1;
        actor.state.energy = f64::INFINITY;
        peer.effects.is_plant = true;
        peer.state.reproduction.countdown = 42;
        world.touch_fruit(&mut actor, &mut peer, 0).unwrap();
        assert!(actor.state.energy.is_nan());
        assert_eq!(peer.state.energy, f64::INFINITY);
        assert!(actor.state.alive && peer.state.alive);
        assert!(peer.infected_genome.is_none());
        assert_eq!(actor.state.total_infected, 0);
        assert_eq!(peer.state.reproduction.countdown, 0);
    }

    #[test]
    fn fruit_notification_error_precedes_donation_tint_and_countdown_stores() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Fruit]);
        let mut peer = organism(&world, &[ColorKind::Green]);
        actor.state.energy = 8.0;
        peer.state.energy = 2.0;
        peer.state.world = Default::default();
        peer.state.reproduction.countdown = 19;
        peer.state.reproduction.infected_energy = 77;
        peer.saved_genome = peer.genome.clone();
        peer.set_display_color(ColorKind::Red.color_ref());
        let random = world.random_snapshot();
        assert_eq!(world.infect_fruit_host(&mut actor, &mut peer, false), Err(crate::WorldError::MissingWorld));
        assert_eq!(actor.state.total_infected, 1);
        assert!(Rc::ptr_eq(peer.infected_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
        assert!(peer.saved_genome.is_none());
        assert_eq!(actor.state.energy, 8.0);
        assert_eq!(peer.state.energy, 2.0);
        assert_eq!(peer.state.temporary_argb(), ColorKind::Red.argb());
        assert_eq!(peer.state.reproduction.countdown, 19);
        assert_eq!(peer.state.reproduction.infected_energy, 77);
        assert!(world.statistics_ref().borrow().get_last_infected_being().is_none());
        assert_eq!(world.random_snapshot(), random);
    }
}
