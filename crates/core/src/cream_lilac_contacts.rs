// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Complete CREAM and LILAC touchEffects1 branches, including the shared peer-energy death check

use crate::organism::primary_genome;

use super::death::Killer;
use super::{Organism, World};
use crate::{ColorKind, TOLERANCE, geometry::GROWTH_SCALES, physics::between};

fn tint(organism: &mut Organism, color: ColorKind) {
    organism.set_display_color(Some(color.color_ref().expect("Tint requires a named source literal")));
}

impl World {
    fn cream_take(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        actor_color: ColorKind,
        peer_color: ColorKind,
        shield: bool,
    ) -> Result<Option<f64>, crate::WorldError> {
        if shield && peer.effects.lavender > 0 {
            self.lavender_shield(peer);
            tint(actor, actor_color);
        }
        if (!shield || peer.effects.lavender <= 0)
            && actor.use_energy(
                self.session.0.parameters.borrow().lifecycle.cream_energy_consumption)?
        {
            let taken = between(
                (0.1 * actor.body.physical_length(segment)?.sqrt())
                    * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                0.0,
                peer.state.energy,
            );
            tint(peer, peer_color);
            tint(actor, actor_color);
            Ok(Some(taken))
        } else {
            Ok(None)
        }
    }

    fn cream_dodge_take(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
    ) -> Result<f64, crate::WorldError> {
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            tint(peer, ColorKind::Teal);
            tint(actor, ColorKind::Cream);
            Ok(0.0)
        } else {
            Ok(self.cream_take(
                actor,
                peer,
                segment,
                ColorKind::Cream,
                ColorKind::GreenBrown,
                true,
            )?
            .unwrap_or(0.0))
        }
    }

    fn blue_counter_drain(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
        ordinary_tint: ColorKind,
    ) -> Result<(), crate::WorldError> {
        if peer.effects.is_enhanced {
            let amount = between(
                (0.5 * peer.body.physical_length(peer_segment)?.sqrt())
                    * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                0.0,
                actor.state.energy,
            );
            actor.use_energy(amount)?;
            tint(actor, ColorKind::DarkLilac);
            if actor.state.energy < TOLERANCE {
                self.die(actor, Some(Killer::Other(&mut *peer)))?;
            }
        } else {
            tint(actor, ordinary_tint);
        }
        tint(peer, ColorKind::Blue);
        Ok(())
    }

    fn cream_lilac_peer_death(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        if peer.state.alive && peer.state.energy < TOLERANCE {
            self.die(peer, Some(Killer::Other(actor)))?;
        }
        Ok(())
    }

    /// Apply the source CREAM branch and its final peer death check
    pub(super) fn touch_cream(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        peer.effects.has_dodged = true;
        let enhanced = actor.effects.is_enhanced;
        let mut taken = 0.0;
        match ColorKind::from_argb(peer.body.color_argb(peer_segment)?) {
            White => {
                if peer.effects.is_plant
                    || peer.effects.is_consumer
                    || peer.effects.is_fungus
                    || peer.effects.plague_version > 0
                    || peer.effects.is_auburn
                {
                    taken = self
                        .cream_take(actor, peer, segment, Cream, GreenBrown, true)?
                        .unwrap_or(0.0);
                } else if enhanced && primary_genome(&actor.genome)?.borrow().modifiers().cream == 2 {
                    taken = self
                        .cream_take(actor, peer, segment, DarkGray, GreenBrown, true)?
                        .unwrap_or(0.0);
                }
            }
            Green | Grass | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade
            | DarkJade | DarkGreen | Purple | Teal | Cyan | Yellow | Auburn | Indigo | Blond
            | Flower | DarkGray | Gold | Spore => {
                taken = self.cream_dodge_take(actor, peer, segment)?;
            }
            Spike => {
                if (peer.effects.is_plant && peer.effects.create_lavender == 0)
                    || peer.effects.is_enhanced
                {
                    taken = self
                        .cream_take(actor, peer, segment, Cream, GreenBrown, true)?
                        .unwrap_or(0.0);
                }
            }
            Blue => {
                if (!enhanced
                    || primary_genome(&actor.genome)?.borrow().modifiers().cream != 3
                    || !peer.effects.is_plant
                    || peer.effects.create_lavender > 0)
                    && peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)?
                {
                    self.blue_counter_drain(actor, peer, peer_segment, Cream)?;
                } else {
                    let color = if enhanced && primary_genome(&actor.genome)?.borrow().modifiers().cream == 3 {
                        DarkGray
                    } else {
                        Cream
                    };
                    taken = self
                        .cream_take(actor, peer, segment, color, GreenBrown, true)?
                        .unwrap_or(0.0);
                }
            }
            Sky => {
                if actor.state.active || peer.effects.sky_version == 1 {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                        peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                        peer.effects.use_extra_effects = true;
                        tint(actor, Cream);
                        tint(peer, DeepSky);
                    } else {
                        taken = self
                            .cream_take(actor, peer, segment, Cream, GreenBrown, true)?
                            .unwrap_or(0.0);
                    }
                }
            }
            DeepSky => {
                if (enhanced || !actor.state.active) && primary_genome(&actor.genome)?.borrow().modifiers().cream == 3 {
                    taken = self
                        .cream_take(actor, peer, segment, DarkGray, GreenBrown, true)?
                        .unwrap_or(0.0);
                } else {
                    tint(actor, Cream);
                    tint(peer, DeepSky);
                }
            }
            Fallow => {
                if enhanced && primary_genome(&actor.genome)?.borrow().modifiers().cream == 3 {
                    taken = self
                        .cream_take(actor, peer, segment, DarkGray, GreenBrown, true)?
                        .unwrap_or(0.0);
                }
            }
            OldBark => {
                if (enhanced || !actor.state.active) && primary_genome(&actor.genome)?.borrow().modifiers().cream == 3 {
                    taken = self
                        .cream_take(actor, peer, segment, DarkGray, GreenBrown, true)?
                        .unwrap_or(0.0);
                }
            }
            Bark => {
                peer.body.set_color(peer_segment, OldBark.color_ref())?;
                if peer.state.active {
                    peer.effects.photosynthesis -= peer.body.photo(peer_segment)?
                        * GROWTH_SCALES[crate::geometry::growth_index(peer.body.growth_ratio)?];
                }
            }
            Silver => {
                if !((peer.state.total_kills > 0
                    && (peer.effects.is_consumer || peer.effects.is_fungus))
                    || peer.effects.is_enhanced)
                {
                    taken = self
                        .cream_take(actor, peer, segment, Cream, GreenBrown, true)?
                        .unwrap_or(0.0);
                }
            }
            Fire | Orange | Red | Maroon => {
                if enhanced && primary_genome(&actor.genome)?.borrow().modifiers().cream == 1 {
                    taken = self
                        .cream_take(actor, peer, segment, DarkGray, GreenBrown, true)?
                        .unwrap_or(0.0);
                }
            }
            Fruit => {
                if peer.effects.spore_version == 2 {
                    taken = self
                        .cream_take(actor, peer, segment, Cream, GreenBrown, false)?
                        .unwrap_or(0.0);
                } else if enhanced && primary_genome(&actor.genome)?.borrow().modifiers().cream == 2 {
                    taken = self
                        .cream_take(actor, peer, segment, DarkGray, GreenBrown, false)?
                        .unwrap_or(0.0);
                }
            }
            Magenta | Rose => {
                if !actor.effects.altruist {
                    taken = self.cream_dodge_take(actor, peer, segment)?;
                }
            }
            Cream => {
                if enhanced && primary_genome(&actor.genome)?.borrow().modifiers().cream == 2 {
                    let unshielded =
                        peer.effects.is_enhanced && primary_genome(&peer.genome)?.borrow().modifiers().cream == 2;
                    taken = self
                        .cream_take(
                            actor,
                            peer,
                            segment,
                            DarkGray,
                            if unshielded { DarkGray } else { GreenBrown },
                            !unshielded,
                        )?
                        .unwrap_or(0.0);
                }
            }
            Pink => {
                if (enhanced || !actor.state.active) && primary_genome(&actor.genome)?.borrow().modifiers().cream == 2 {
                    taken = self
                        .cream_take(actor, peer, segment, DarkGray, GreenBrown, true)?
                        .unwrap_or(0.0);
                }
            }
            Coral => {
                if enhanced && primary_genome(&actor.genome)?.borrow().modifiers().cream == 2 {
                    taken = self
                        .cream_take(actor, peer, segment, DarkGray, GreenBrown, true)?
                        .unwrap_or(0.0);
                }
            }
            Lavender => {
                let (amount, shield_increment) = if peer.effects.is_enhanced {
                    (
                        1.0 / self.session.0.parameters.borrow().lifecycle.lavender_energy_consumption,
                        1000,
                    )
                } else {
                    (
                        0.2 / self.session.0.parameters.borrow().lifecycle.lavender_energy_consumption,
                        200,
                    )
                };
                actor.use_energy(
                    between(amount, 0.0, actor.state.energy))?;
                let shield = self.session.0.parameters.borrow().effects.lavender_shield;
                if peer.effects.lavender < shield {
                    peer.effects.lavender = peer.effects.lavender.wrapping_add(shield_increment);
                    if peer.effects.lavender >= shield {
                        peer.effects.lavender = shield;
                    }
                }
                tint(peer, Lavender);
                tint(actor, DeadBark);
                if actor.state.energy < TOLERANCE {
                    self.die(actor, Some(Killer::Other(&mut *peer)))?;
                }
            }
            Mint => {
                if peer.use_energy(
                    self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 10.0)? {
                    if actor.effects.healing > 0 {
                        actor.body.set_color(segment, DarkFire.color_ref())?;
                        actor.effects.use_extra_effects = true;
                    } else {
                        actor.body.set_color(segment, if actor.effects.is_plant {
                            Broken
                        } else {
                            LightBrown
                        }
                        .color_ref())?;
                    }
                    tint(peer, Mint);
                    actor.effects.is_injured = true;
                }
            }
            SpikePoint => {}
            Brown => {
                if !actor.state.active && primary_genome(&actor.genome)?.borrow().modifiers().cream == 1 {
                    if let Some(amount) =
                        self.cream_take(actor, peer, segment, Cream, GreenBrown, false)?
                    {
                        taken = amount;
                        if peer.state.alive && !peer.effects.is_enhanced {
                            peer.effects.is_injured = true;
                        }
                    }
                }
            }
            // These are the source switch's default colors, including unknown full ARGB values
            _ => {
                taken = self
                    .cream_take(actor, peer, segment, Cream, GreenBrown, true)?
                    .unwrap_or(0.0);
            }
        }
        if taken > 0.0 {
            peer.state.energy -= taken;
            actor.state.energy += taken;
            taken *= self.session.0.parameters.borrow().lifecycle.cream_organic_subs_produced;
            actor.use_energy(taken)?;
        }
        self.cream_lilac_peer_death(actor, peer)?;
        Ok(())
    }

    fn lilac_take(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        factor: f64,
    ) -> Result<Option<f64>, crate::WorldError> {
        if actor.use_energy(
            self.session.0.parameters.borrow().lifecycle.lilac_energy_consumption)? {
            let amount = between(
                (factor * actor.body.physical_length(segment)?.sqrt())
                    * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                0.0,
                peer.state.energy,
            );
            tint(peer, ColorKind::DarkLilac);
            tint(actor, ColorKind::Lilac);
            if !actor.effects.is_enhanced {
                actor.body.set_color(segment, ColorKind::DarkLilac.color_ref())?;
                actor.effects.use_extra_effects = true;
            }
            Ok(Some(amount))
        } else {
            Ok(None)
        }
    }

    fn lilac_dodge_take(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        factor: f64,
    ) -> Result<f64, crate::WorldError> {
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            tint(peer, ColorKind::Teal);
            tint(actor, ColorKind::Lilac);
            Ok(0.0)
        } else {
            Ok(match self.lilac_take(actor, peer, segment, factor)? {
                Some(amount) => amount,
                None => 0.0,
            })
        }
    }

    /// Apply the source LILAC branch and its final peer death check
    pub(super) fn touch_lilac(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        peer.effects.has_dodged = true;
        let mut taken = 0.0;
        match ColorKind::from_argb(peer.body.color_argb(peer_segment)?) {
            Lilac => {
                if let Some(amount) = self.lilac_take(actor, peer, segment, 10.0)? {
                    taken = amount;
                    if !peer.effects.is_enhanced {
                        peer.body.set_color(peer_segment, DarkLilac.color_ref())?;
                        peer.effects.use_extra_effects = true;
                    }
                }
            }
            White | OldBark | DeadBark | Ice | Coral | Fruit => {
                if primary_genome(&actor.genome)?.borrow().modifiers().modifies_lilac
                    || peer.effects.is_consumer
                    || peer.effects.is_fungus
                    || peer.effects.plague_version > 0
                    || peer.effects.fallow_version > 0
                    || (peer.effects.is_coral && !actor.effects.is_plant && !actor.effects.is_consumer) {
                    if let Some(amount) = self.lilac_take(actor, peer, segment, 10.0)? { taken = amount; }
                }
            }
            Grass => {
                if primary_genome(&actor.genome)?.borrow().modifiers().modifies_lilac
                    || peer.effects.is_consumer
                    || peer.effects.is_fungus
                    || peer.effects.plague_version > 0
                    || peer.effects.fallow_version > 0
                    || (peer.effects.is_coral && !actor.effects.is_plant && !actor.effects.is_consumer) {
                    taken = self.lilac_dodge_take(actor, peer, segment, 5.0)?;
                }
            }
            Green | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade | DarkJade
            | DarkGreen | Purple | Teal | Cyan | Yellow | Auburn | Indigo | Blond | Flower
            | DarkGray | Gold | Spore => {
                if primary_genome(&actor.genome)?.borrow().modifiers().modifies_lilac
                    || peer.effects.is_consumer
                    || peer.effects.is_fungus
                    || peer.effects.plague_version > 0
                    || peer.effects.fallow_version > 0
                    || (peer.effects.is_coral && !actor.effects.is_plant && !actor.effects.is_consumer) {
                    taken = self.lilac_dodge_take(actor, peer, segment, 10.0)?;
                }
            }
            Blue => {
                if peer.use_energy(
                    self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
                    self.blue_counter_drain(actor, peer, peer_segment, Lilac)?;
                } else {
                    if let Some(amount) = self.lilac_take(actor, peer, segment, 10.0)? { taken = amount; }
                }
            }
            Red => {
                if !peer.effects.is_enhanced {
                    if let Some(amount) = self.lilac_take(actor, peer, segment, 10.0)? { taken = amount; }
                }
            }
            Mint | Lavender | Magenta | Rose => {
                if !actor.effects.altruist {
                    taken = self.lilac_dodge_take(actor, peer, segment, 10.0)?;
                }
            }
            Spike => {
                if peer.effects.is_plant || peer.effects.is_enhanced {
                    if let Some(amount) = self.lilac_take(actor, peer, segment, 10.0)? { taken = amount; }
                }
            }
            SpikePoint => {
                if (peer.effects.is_plant && primary_genome(&actor.genome)?.borrow().modifiers().modifies_lilac) || (peer.effects.is_enhanced && !primary_genome(&actor.genome)?.borrow().modifiers().modifies_lilac) {
                    if let Some(amount) = self.lilac_take(actor, peer, segment, 10.0)? { taken = amount; }
                }
            }
            Olive => {}
            Brown => {
                if peer.state.alive && (primary_genome(&actor.genome)?.borrow().modifiers().modifies_lilac || !peer.effects.is_enhanced) {
                    if let Some(amount) = self.lilac_take(actor, peer, segment, 10.0)? {
                        taken = amount;
                        if !peer.effects.is_enhanced {
                            peer.effects.is_injured = true;
                        }
                    }
                }
            }
            Bark => {
                peer.body.set_color(peer_segment, OldBark.color_ref())?;
                if peer.state.active {
                    peer.effects.photosynthesis -= peer.body.photo(peer_segment)?
                        * GROWTH_SCALES[crate::geometry::growth_index(peer.body.growth_ratio)?];
                }
            }
            // The source default weakens every remaining base, internal, and unknown ARGB color
            _ => if let Some(amount) = self.lilac_take(actor, peer, segment, 10.0)? { taken = amount; },
        }
        if taken > 0.0 {
            peer.use_energy(taken)?;
        }
        self.cream_lilac_peer_death(actor, peer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gene, Genome, GenomeProperties, WorldConfig};
    use std::rc::Rc;

    fn pair(
        actor: ColorKind,
        peer: ColorKind,
    ) -> (World, super::super::OrganismRef, super::super::OrganismRef) {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        for (color, center) in [(actor, [100, 100]), (peer, [300, 300])] {
            let code = Genome::from_parts(
                &[Gene::new(12.0, 0.0, color.argb(), 0, [0; 36]).shared()],
                GenomeProperties {
                    symmetry: 1,
                    ..GenomeProperties::default()
                },
            );
            world.paste_genome(code, center).unwrap().unwrap();
        }
        let actor = Rc::clone(&world.organisms.borrow()[0]);
        let peer = Rc::clone(&world.organisms.borrow()[1]);
        actor.borrow_mut().body.set_physical_length(0, 100.0).unwrap();
        peer.borrow_mut().body.set_physical_length(0, 100.0).unwrap();
        actor.borrow_mut().state.energy = 20.0;
        peer.borrow_mut().state.energy = 5.0;
        world.oxygen.set(1000.0);
        world.carbon_dioxide.set(0.0);
        (world, actor, peer)
    }

    #[test]
    fn failed_partial_dodge_payment_precedes_cream_transfer_and_failed_byproduct_payment() {
        let (world, actor, peer) = pair(ColorKind::Cream, ColorKind::Green);
        world.session.0.parameters.borrow_mut().lifecycle.dodge_energy_consumption = 1.0;
        world.oxygen.set(0.5);
        peer.borrow_mut().effects.dodge = true;
        world.touch_cream(&mut actor.borrow_mut(), &mut peer.borrow_mut(), 0, 0).unwrap();
        assert_eq!(peer.borrow().state.energy, 2.5);
        assert_eq!(actor.borrow().state.energy, 22.0);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 0.5);
        assert!(peer.borrow().effects.has_dodged);
        assert_eq!(
            peer.borrow().state.temporary_argb(),
            ColorKind::GreenBrown.argb()
        );
        assert_eq!(
            actor.borrow().state.temporary_argb(),
            ColorKind::Cream.argb()
        );
    }

    #[test]
    fn cream_rechecks_the_drained_lavender_shield_before_the_same_attack() {
        for (shield, actor_energy, peer_energy) in [(185, 21.975, 3.0), (186, 20.0, 5.0)] {
            let (world, actor, peer) = pair(ColorKind::Cream, ColorKind::Green);
            peer.borrow_mut().effects.lavender = shield;
            world.touch_cream(&mut actor.borrow_mut(), &mut peer.borrow_mut(), 0, 0).unwrap();
            assert_eq!(actor.borrow().state.energy, actor_energy);
            assert_eq!(peer.borrow().state.energy, peer_energy);
            assert_eq!(peer.borrow().effects.lavender, shield - 185);
        }
    }

    #[test]
    fn enhanced_blue_counter_death_precedes_the_final_peer_death_event() {
        let (world, actor, peer) = pair(ColorKind::Cream, ColorKind::Blue);
        actor.borrow_mut().state.energy = 0.1;
        peer.borrow_mut().state.energy = 0.0;
        peer.borrow_mut().effects.is_enhanced = true;
        world.oxygen.set(0.1);
        world.touch_cream(&mut actor.borrow_mut(), &mut peer.borrow_mut(), 0, 0).unwrap();
        assert!(!actor.borrow().state.alive && !peer.borrow().state.alive);
        assert_eq!(world.population(), 0);
        assert_eq!(actor.borrow().state.total_kills, 1);
        assert_eq!(peer.borrow().state.total_kills, 1);
        assert_eq!(
            actor.borrow().state.temporary_argb(),
            ColorKind::DarkLilac.argb()
        );
        assert_eq!(peer.borrow().state.temporary_argb(), ColorKind::Blue.argb());
        assert!(Rc::ptr_eq(
            world.statistics_ref().borrow().get_last_dead_being().unwrap(), peer.borrow().genome.as_ref().unwrap()));
    }

    #[test]
    fn mint_changes_cream_phenotype_only_after_a_complete_peer_payment() {
        let (world, actor, peer) = pair(ColorKind::Cream, ColorKind::Mint);
        actor.borrow_mut().effects.healing = 1;
        actor.borrow_mut().effects.is_injured = false;
        actor.borrow_mut().effects.use_extra_effects = false;
        actor.borrow_mut().body.update_effects = 0;
        let coefficients = actor.borrow().body.photo_values().unwrap();
        world.oxygen.set(0.003);
        world.touch_cream(&mut actor.borrow_mut(), &mut peer.borrow_mut(), 0, 0).unwrap();
        assert_eq!(
            actor.borrow().body.color_argb(0).unwrap(),
            ColorKind::Cream.argb().unwrap()
        );
        assert!(!actor.borrow().effects.is_injured);
        assert_eq!(world.oxygen.get(), 0.0);
        assert!(peer.borrow().state.energy < 5.0);
        world.oxygen.set(1.0);
        world.touch_cream(&mut actor.borrow_mut(), &mut peer.borrow_mut(), 0, 0).unwrap();
        assert_eq!(
            actor.borrow().body.color_argb(0).unwrap(),
            ColorKind::DarkFire.argb().unwrap()
        );
        assert!(actor.borrow().effects.is_injured && actor.borrow().effects.use_extra_effects);
        assert_eq!(actor.borrow().body.update_effects, 0);
        assert_eq!(actor.borrow().body.photo_values().unwrap(), coefficients);
        assert_eq!(actor.borrow().state.energy, 20.0);
        assert_eq!(actor.borrow().state.temporary_argb(), None);
    }

    #[test]
    fn lavender_receives_wrapping_shield_growth_even_when_the_actor_payment_fails() {
        let (world, actor, peer) = pair(ColorKind::Cream, ColorKind::Lavender);
        world.session.0.parameters.borrow_mut().effects.lavender_shield = i32::MAX;
        peer.borrow_mut().effects.lavender = i32::MAX - 50;
        peer.borrow_mut().effects.is_enhanced = false;
        world.oxygen.set(0.0);
        world.touch_cream(&mut actor.borrow_mut(), &mut peer.borrow_mut(), 0, 0).unwrap();
        assert_eq!(peer.borrow().effects.lavender, -2_147_483_499);
        assert_eq!(actor.borrow().state.energy, 20.0);
        assert_eq!(
            actor.borrow().state.temporary_argb(),
            ColorKind::DeadBark.argb()
        );
        assert_eq!(
            peer.borrow().state.temporary_argb(),
            ColorKind::Lavender.argb()
        );
    }

    #[test]
    fn lilac_grass_has_half_the_ordinary_green_damage_without_actor_energy_gain() {
        for (color, remaining) in [(ColorKind::Grass, 90.0), (ColorKind::Green, 80.0)] {
            let (world, actor, peer) = pair(ColorKind::Lilac, color);
            actor.borrow_mut().body.set_physical_length(0, 1.0).unwrap();
            peer.borrow_mut().state.energy = 100.0;
            peer.borrow_mut().effects.is_consumer = true;
            world.touch_lilac(&mut actor.borrow_mut(), &mut peer.borrow_mut(), 0, 0).unwrap();
            assert_eq!(peer.borrow().state.energy, remaining);
            assert_eq!(actor.borrow().state.energy, 20.0);
            assert_eq!(
                actor.borrow().body.color_argb(0).unwrap(),
                ColorKind::DarkLilac.argb().unwrap()
            );
            assert!(actor.borrow().effects.use_extra_effects);
        }
    }

    #[test]
    fn lilac_recharges_after_its_paid_attack_even_when_no_oxygen_remains_for_damage() {
        let (world, actor, peer) = pair(ColorKind::Lilac, ColorKind::Fire);
        world.session.0.parameters.borrow_mut().lifecycle.lilac_energy_consumption = 1.0;
        world.oxygen.set(1.0);
        world.touch_lilac(&mut actor.borrow_mut(), &mut peer.borrow_mut(), 0, 0).unwrap();
        assert_eq!(actor.borrow().state.energy, 19.0);
        assert_eq!(peer.borrow().state.energy, 5.0);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 1.0);
        assert_eq!(
            actor.borrow().body.color_argb(0).unwrap(),
            ColorKind::DarkLilac.argb().unwrap()
        );
        assert!(actor.borrow().effects.use_extra_effects);
        assert_eq!(
            peer.borrow().state.temporary_argb(),
            ColorKind::DarkLilac.argb()
        );
    }

    #[test]
    fn full_argb_defaults_remain_distinct_from_the_altruist_magenta_exemption() {
        for (color, operation) in [
            (
                ColorKind::Cream,
                World::touch_cream as fn(&World, &mut Organism, &mut Organism, usize, usize) -> Result<(), crate::WorldError>,
            ),
            (ColorKind::Lilac, World::touch_lilac),
        ] {
            let (mut world, actor, peer) = pair(color, ColorKind::Magenta);
            actor.borrow_mut().effects.altruist = true;
            operation(
                &mut world,
                &mut actor.borrow_mut(),
                &mut peer.borrow_mut(),
                0,
                0,
            ).unwrap();
            assert_eq!(peer.borrow().state.energy, 5.0);
            peer.borrow_mut().body.set_color_argb(0, 0x12ff00ff).unwrap();
            operation(
                &mut world,
                &mut actor.borrow_mut(),
                &mut peer.borrow_mut(),
                0,
                0,
            ).unwrap();
            assert!(peer.borrow().state.energy < 5.0);
        }
    }

    #[test]
    fn blocked_lilac_still_checks_peer_death_without_repeating_an_existing_death() {
        let (world, actor, peer) = pair(ColorKind::Lilac, ColorKind::Olive);
        peer.borrow_mut().state.energy = 0.0;
        world.touch_lilac(&mut actor.borrow_mut(), &mut peer.borrow_mut(), 0, 0).unwrap();
        assert!(!peer.borrow().state.alive);
        assert_eq!(world.population(), 1);
        assert_eq!(actor.borrow().state.total_kills, 1);
        world.touch_lilac(&mut actor.borrow_mut(), &mut peer.borrow_mut(), 0, 0).unwrap();
        assert_eq!(world.population(), 1);
        assert_eq!(actor.borrow().state.total_kills, 1);
    }

    #[test]
    fn lilac_missing_destination_preserves_payment_and_tints_before_failure() {
        let (world, actor_ref, peer_ref) = pair(ColorKind::Lilac, ColorKind::Green);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        {
            let mut genome = actor.genome.as_ref().unwrap().borrow_mut();
            let mut properties = genome.properties();
            properties.modifiers.modifies_lilac = true;
            genome.set_properties(properties);
        }
        actor.effects.is_enhanced = false;
        actor.effects.use_extra_effects = false;
        actor.body.phenotype_colors = None;
        let energy = actor.state.energy;
        let peer_energy = peer.state.energy;
        let oxygen = world.oxygen.get();
        let carbon = world.carbon_dioxide.get();
        let cost = world.parameters().lifecycle.lilac_energy_consumption;
        let random = world.random_snapshot();
        assert_eq!(world.touch_lilac(&mut actor, &mut peer, 0, 0),
            Err(crate::WorldError::Geometry(crate::GeometryError::MissingColorArray)));
        assert_eq!(actor.state.energy, energy - cost);
        assert_eq!(world.oxygen.get(), oxygen - cost);
        assert_eq!(world.carbon_dioxide.get(), carbon + cost);
        assert_eq!(peer.state.energy, peer_energy);
        assert!(actor.state.temporary_color.as_ref().unwrap()
            .same_instance(&ColorKind::Lilac.color_ref().unwrap()));
        assert!(peer.state.temporary_color.as_ref().unwrap()
            .same_instance(&ColorKind::DarkLilac.color_ref().unwrap()));
        assert_eq!(actor.state.temporary_color_frames, 10);
        assert_eq!(peer.state.temporary_color_frames, 10);
        assert!(!actor.effects.use_extra_effects);
        assert_eq!(world.random_snapshot(), random);
    }

}
