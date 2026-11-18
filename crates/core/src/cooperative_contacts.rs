// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! BLUE, YELLOW, ROSE, MINT, LAVENDER, and MAGENTA from Organism.java:17570-17919

use super::contact_state::ContactActor;
use super::death::Killer;
use super::{Organism, OrganismRef, World, WorldError};
use crate::{ColorKind, TOLERANCE, physics::between};
use crate::organism::same_genome;

fn kind(peer: &Organism, segment: usize) -> Result<ColorKind, WorldError> {
    peer.body.color_kind(segment).map_err(WorldError::from)
}

fn tint(actor: &mut Organism, color: ColorKind) {
    actor.set_display_color(Some(color.color_ref().expect("Tint requires a named source literal")));
}

fn has_code(actor: &Organism, code: Option<&super::GenomeRef>) -> bool {
    same_genome(actor.infected_genome.as_ref(), code)
}

fn viral_yellow(actor: &Organism) -> bool {
    actor.effects.is_infectious
        && !actor.effects.is_consumer
        && !actor.effects.is_plant
        && !actor.effects.is_fungus
}

fn yellow_host(peer: &Organism) -> bool {
    peer.effects.is_plant || peer.effects.is_consumer || peer.effects.is_fungus
}

fn mint_target(actor: &Organism, peer: &Organism) -> bool {
    peer.effects.is_infectious
        || peer.effects.is_coral
        || peer.effects.plague_version > 0
        || (peer.effects.fallow_version > 0
            && (peer.effects.is_plant
                || peer.effects.fallow_version == 4
                || actor.effects.is_enhanced))
}

impl World {
    fn cooperative_peer_death(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        if peer.state.alive && peer.state.energy < TOLERANCE {
            self.die(peer, Some(Killer::Other(actor)))?;
        }
        Ok(())
    }

    pub(super) fn touch_blue(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<(), WorldError> {
        if !actor.state.active {
            self.touch_hatch(actor, peer, peer_segment)?;
        }
        self.cooperative_peer_death(actor, peer)?;
        Ok(())
    }

    fn yellow_reproduction_contact(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
    ) -> Result<(), WorldError> {
        if peer.state.reproduction.countdown < peer.state.reproduction.maximum_countdown {
            peer.state.reproduction.countdown = peer.state.reproduction.maximum_countdown;
            if actor.state.reproduction.countdown == 0 {
                tint(actor, ColorKind::Fallow);
                tint(peer, ColorKind::Flower);
            }
        }
        if has_code(peer, actor.genome.as_ref())
            && (peer.state.energy > f64::from(peer.state.reproduction.energy)
                || (!peer.state.active
                    && peer.state.energy > 0.5 * f64::from(peer.state.reproduction.energy)))
            && actor.state.energy > self.session.0.parameters.borrow().lifecycle.virus_energy_consumption
        {
            tint(actor, ColorKind::Plague);
            peer.state.virus_children = 1_i32.wrapping_add(actor.effects.yellow_count);
            self.reproduce_yellow_from(peer_ref, peer, Some((actor_ref, &*actor)))?;
            if peer.state.virus_children > 0 {
                if peer.effects.antiviral > 0 {
                    peer.infected_genome = None;
                    peer.saved_genome = None;
                }
                tint(peer, ColorKind::DarkLilac);
                actor.use_energy(
                    self.session.0.parameters.borrow().lifecycle.virus_energy_consumption)?;
                peer.state.virus_children = 0;
                if actor.state.reproduction.countdown < 10 {
                    actor.state.reproduction.countdown = 10;
                }
            }
        }
        Ok(())
    }

    pub(super) fn touch_yellow(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        match kind(peer, peer_segment)? {
            Brown => {}
            Blue => {
                if viral_yellow(actor) && yellow_host(peer) {
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
                            tint(actor, Fallow);
                        }
                        tint(peer, Blue);
                    } else {
                        self.yellow_reproduction_contact(actor_ref, actor, peer_ref, peer)?;
                    }
                }
            }
            Mint => {
                if viral_yellow(actor)
                    && peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 10.0)?
                {
                    actor.body.set_color(segment, LightBrown.color_ref())?;
                    tint(peer, Mint);
                    self.restore_contact_effects(actor)?;
                }
            }
            _ => {
                if viral_yellow(actor) && yellow_host(peer) {
                    self.yellow_reproduction_contact(actor_ref, actor, peer_ref, peer)?;
                }
            }
        }
        self.cooperative_peer_death(actor, peer)?;
        Ok(())
    }

    pub(super) fn touch_rose(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        if kind(peer, peer_segment)? != ColorKind::Brown
            && actor.effects.altruist
            && peer.effects.altruist
            && actor.body.growth_ratio == 1
            && peer.state.active
            && actor.state.energy > peer.state.energy + 2.0
            && actor.state.energy > f64::from(actor.state.reproduction.energy / 2)
            && peer.state.energy < f64::from(peer.state.reproduction.energy / 2)
        {
            let transferred = between(
                0.5 * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                0.0,
                actor.state.energy - (peer.state.energy + 2.0),
            );
            tint(peer, ColorKind::Cyan);
            tint(actor, ColorKind::Rose);
            peer.state.energy += transferred;
            actor.state.energy -= transferred;
        }
        self.cooperative_peer_death(actor, peer)?;
        Ok(())
    }

    fn mint_clear_own_infection(&self, actor: &mut Organism, peer: &Organism) -> Result<(), crate::WorldError> {
        Ok({
        if has_code(actor, peer.genome.as_ref())
            && actor.use_energy(
                self.session.0.parameters.borrow().lifecycle.mint_energy_consumption)?
        {
            tint(actor, ColorKind::Mint);
            actor.infected_genome = None;
            actor.saved_genome = None;
        }
    
        })
    }

    fn mint_help_ally(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        Ok({
        if !actor.effects.altruist || !peer.effects.altruist {
            return Ok(());
        }
        if peer.infected_genome.is_some()
            && !has_code(peer, actor.genome.as_ref())
            && self.shared_support_payment(
                actor,
                peer,
                self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 2.0,
            )?
        {
            peer.infected_genome = None;
            peer.saved_genome = None;
            tint(peer, ColorKind::Cyan);
            tint(actor, ColorKind::Mint);
        }
        if peer.effects.reproduce_late < 0
            && self.shared_support_payment(
                actor,
                peer,
                self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 2.0,
            )?
        {
            peer.effects.reproduce_late = 0;
            tint(peer, ColorKind::Cyan);
            tint(actor, ColorKind::Mint);
        }
    
        })
    }

    fn mint_freeze_spore(&self, actor: &mut Organism, peer: &mut Organism, fruit: bool) -> Result<(), crate::WorldError> {
        if !peer.effects.all_frozen
            && actor.use_energy(
                self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 10.0)?
        {
            for color_index in peer.body.indices() {
                if matches!(ColorKind::from_argb(peer.body.color_argb(color_index)?), ColorKind::DarkFire)
                    || ColorKind::from_argb(peer.body.color_argb(color_index)?)
                        == if fruit {
                            ColorKind::Fruit
                        } else {
                            ColorKind::Cream
                        }
                {
                    peer.body.set_color(color_index, if fruit {
                        ColorKind::Broken
                    } else {
                        ColorKind::LightBrown
                    }
                    .color_ref())?;
                }
            }
            tint(actor, ColorKind::Mint);
            peer.state.temporary_color_frames = 0;
            peer.effects.is_injured = true;
            peer.effects.is_frozen = true;
            peer.effects.all_frozen = true;
            if !fruit {
                peer.effects.is_consumer = false;
            }
        }
        Ok(())
    }

    pub(super) fn touch_mint(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        match kind(peer, peer_segment)? {
            Brown => {
                if peer.effects.is_infectious {
                    self.mint_clear_own_infection(actor, peer)?;
                }
            }
            Blue => {
                if mint_target(actor, peer) && peer.state.active {
                    if (!peer.effects.is_plant
                        || ((actor.effects.is_consumer || actor.effects.is_fungus)
                            && !actor.effects.is_enhanced))
                        && peer.use_energy(
                            self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)?
                    {
                        tint(actor, Mint);
                        tint(peer, Blue);
                    } else {
                        self.neutralized_from(peer, ContactActor::Other(&mut *actor))?;
                        self.mint_clear_own_infection(actor, peer)?;
                    }
                }
                self.mint_help_ally(actor, peer)?;
            }
            Mint => {
                if peer.effects.is_infectious {
                    self.mint_clear_own_infection(actor, peer)?;
                }
                self.mint_help_ally(actor, peer)?;
            }
            _ => {
                if peer.state.active {
                    if mint_target(actor, peer) {
                        self.neutralized_from(peer, ContactActor::Other(&mut *actor))?;
                        self.mint_clear_own_infection(actor, peer)?;
                    }
                } else if matches!(peer.effects.spore_version, 1 | 2) {
                    self.mint_freeze_spore(actor, peer, true)?;
                    self.mint_clear_own_infection(actor, peer)?;
                } else if peer.effects.spore_version == 5 {
                    self.mint_freeze_spore(actor, peer, false)?;
                }
                self.mint_help_ally(actor, peer)?;
            }
        }
        self.cooperative_peer_death(actor, peer)?;
        Ok(())
    }

    pub(super) fn touch_lavender(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        if !matches!(
            kind(peer, peer_segment)?,
            White | Plague | Coral | Cream | Fallow | Fruit | Brown
        ) && actor.effects.altruist
            && peer.effects.altruist
        {
            self.share_lavender_contact(actor, peer);
        }
        self.cooperative_peer_death(actor, peer)?;
        Ok(())
    }

    pub(super) fn touch_magenta(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        if kind(peer, peer_segment)? != ColorKind::Brown
            && peer.effects.is_injured
            && peer.state.active
            && actor.effects.altruist
            && peer.effects.altruist
        {
            self.heal_contact_segments(actor, peer)?;
        }
        self.cooperative_peer_death(actor, peer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use crate::{EffectsState, Gene, Genome, GenomeProperties, WorldConfig};

    fn setup(
        actor_colors: &[ColorKind],
        peer_colors: &[ColorKind],
    ) -> (World, OrganismRef, OrganismRef) {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        world.session.0.parameters.borrow_mut().genetics.min_mutation_rate = 0;
        world.session.0.parameters.borrow_mut().genetics.max_mutation_rate = 0;
        for (colors, center) in [(actor_colors, [1000, 1000]), (peer_colors, [3500, 3500])] {
            let genes: Vec<_> = colors
                .iter()
                .enumerate()
                .map(|(index, color)| {
                    Gene::new(12.0, index as f64 * 0.25, color.argb(), -1, [0; 36]).shared()
                })
                .collect();
            let code = Genome::from_parts(
                &genes,
                GenomeProperties {
                    symmetry: 1,
                    ..Default::default()
                },
            );
            world.paste_genome(code, center).unwrap().unwrap();
        }
        let actor = Rc::clone(&world.organisms.borrow()[0]);
        let peer = Rc::clone(&world.organisms.borrow()[1]);
        for reference in [&actor, &peer] {
            let mut organism = reference.borrow_mut();
            organism.effects = EffectsState {

                ..Default::default()
            };
            organism.state.energy = 100.0;
            organism.state.active = true;
            organism.state.temporary_color = None;
            organism.state.temporary_color_frames = 0;
        }
        world.oxygen.set(1000.0);
        world.carbon_dioxide.set(0.0);
        world.rebuild_spatial();
        (world, actor, peer)
    }

    #[test]
    fn yellow_resets_the_host_countdown_before_checking_infection_identity() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Yellow], &[ColorKind::Green]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.is_infectious = true;
        peer.effects.is_plant = true;
        actor.state.reproduction.countdown = 0;
        peer.state.reproduction.maximum_countdown = 23;
        peer.state.reproduction.countdown = 1;
        let energy = [actor.state.energy, peer.state.energy];
        let random = world.random_snapshot();
        world
            .touch_yellow(&actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0)
            .unwrap();
        assert_eq!(peer.state.reproduction.countdown, 23);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Fallow.argb());
        assert_eq!(peer.state.temporary_argb(), ColorKind::Flower.argb());
        assert_eq!([actor.state.energy, peer.state.energy], energy);
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(world.organisms.borrow().len(), 2);
    }

    #[test]
    fn yellow_requires_energy_strictly_above_the_virus_cost_before_any_birth() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Yellow], &[ColorKind::Green]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.is_infectious = true;
        peer.effects.is_plant = true;
        peer.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        world.session.0.parameters.borrow_mut().lifecycle.virus_energy_consumption = 1.0;
        actor.state.energy = 1.0;
        let random = world.random_snapshot();
        world
            .touch_yellow(&actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0)
            .unwrap();
        assert_eq!(world.organisms.borrow().len(), 2);
        assert_eq!(actor.state.energy, 1.0);
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn yellow_creates_real_children_while_both_contact_actors_are_held() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Yellow], &[ColorKind::Green]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.is_infectious = true;
        actor.effects.yellow_count = 2;
        peer.effects.is_plant = true;
        peer.effects.antiviral = 1;
        peer.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        peer.saved_genome = Some(Rc::clone(peer.genome.as_ref().unwrap()));
        peer.state.energy = 1000.0;
        world.session.0.parameters.borrow_mut().lifecycle.virus_energy_consumption = 0.125;
        world
            .touch_yellow(&actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0)
            .unwrap();
        assert!(world.organisms.borrow().len() > 2);
        assert!(
            world.organisms.borrow()
                .iter()
                .skip(2)
                .all(|child| child.borrow().state.parent_id == peer.id as i32)
        );
        assert_eq!(actor.state.energy, 99.875);
        assert_eq!(actor.state.reproduction.countdown, 10);
        assert_eq!(peer.state.virus_children, 0);
        assert!(peer.infected_genome.is_none() && peer.saved_genome.is_none());
        assert_eq!(peer.state.temporary_argb(), ColorKind::DarkLilac.argb());
    }

    #[test]
    fn yellow_blue_retaliation_kills_the_attacker_before_the_exhausted_defender() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Yellow], &[ColorKind::Blue]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.is_infectious = true;
        actor.state.energy = 0.25;
        peer.effects.is_plant = true;
        peer.effects.is_enhanced = true;
        peer.state.energy = 0.25;
        peer.body.set_physical_length(0, 16.0).unwrap();
        world.session.0.parameters.borrow_mut().lifecycle.blue_energy_consumption = 0.25;
        world
            .touch_yellow(&actor_ref, &mut actor, &peer_ref, &mut peer, 0, 0)
            .unwrap();
        assert!(!actor.state.alive && !peer.state.alive);
        assert_eq!(actor.state.total_kills, 1);
        assert_eq!(peer.state.total_kills, 1);
        assert_eq!(peer.state.temporary_argb(), ColorKind::Blue.argb());
        assert_eq!(world.population.get(), 0);
    }

    #[test]
    fn rose_thresholds_divide_integer_reproduction_energy_before_widening() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Rose], &[ColorKind::Green]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.altruist = true;
        peer.effects.altruist = true;
        actor.body.growth_ratio = 1;
        actor.state.reproduction.energy = 101;
        peer.state.reproduction.energy = 21;
        actor.state.energy = 50.25;
        peer.state.energy = 9.75;
        let gases = [world.oxygen.get(), world.carbon_dioxide.get()];
        world.touch_rose(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(actor.state.energy, 49.25);
        assert_eq!(peer.state.energy, 10.75);
        assert_eq!([world.oxygen.get(), world.carbon_dioxide.get()], gases);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Rose.argb());
        assert_eq!(peer.state.temporary_argb(), ColorKind::Cyan.argb());
    }

    #[test]
    fn mint_altruist_cure_retains_the_first_partial_payment() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Mint], &[ColorKind::Green]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.altruist = true;
        peer.effects.altruist = true;
        peer.infected_genome = Some(Rc::clone(peer.genome.as_ref().unwrap()));
        peer.saved_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        peer.effects.reproduce_late = -3;
        world.session.0.parameters.borrow_mut().lifecycle.mint_energy_consumption = 2.0;
        world.oxygen.set(0.5);
        world.touch_mint(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(peer.state.energy, 99.5);
        assert_eq!(actor.state.energy, 100.0);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 0.5);
        assert!(peer.infected_genome.is_some() && peer.saved_genome.is_some());
        assert_eq!(peer.effects.reproduce_late, -3);
    }

    #[test]
    fn mint_own_cure_uses_code_identity_and_has_a_separate_brown_gate() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Mint], &[ColorKind::Brown]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.infected_genome = Some(Rc::clone(peer.genome.as_ref().unwrap()));
        actor.saved_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        world.touch_mint(&mut actor, &mut peer, 0).unwrap();
        assert!(actor.infected_genome.is_some());
        peer.effects.is_infectious = true;
        world.touch_mint(&mut actor, &mut peer, 0).unwrap();
        assert!(actor.infected_genome.is_none() && actor.saved_genome.is_none());
        actor.infected_genome = Some(Rc::new(std::cell::RefCell::new(
            peer.genome.as_ref().unwrap().borrow().clone(),
        )));
        world.touch_mint(&mut actor, &mut peer, 0).unwrap();
        assert!(actor.infected_genome.is_some());
    }

    #[test]
    fn mint_inactive_spores_rewrite_only_their_source_specific_colors() {
        for (version, original, replacement) in [
            (1, ColorKind::Fruit, ColorKind::Broken),
            (5, ColorKind::Cream, ColorKind::LightBrown),
        ] {
            let (world, actor_ref, peer_ref) = setup(
                &[ColorKind::Mint],
                &[original, ColorKind::Green, ColorKind::DarkFire],
            );
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            peer.state.active = false;
            peer.effects.spore_version = version;
            peer.effects.is_consumer = true;
            peer.state.temporary_color_frames = 7;
            world.touch_mint(&mut actor, &mut peer, 0).unwrap();
            assert_eq!(
                (peer.body.indices()).map(|index| peer.body.color_argb(index).unwrap())
                    .collect::<Vec<_>>(),
                vec![
                    replacement.argb().unwrap(),
                    ColorKind::Green.argb().unwrap(),
                    replacement.argb().unwrap()
                ]
            );
            assert!(peer.effects.all_frozen && peer.effects.is_frozen && peer.effects.is_injured);
            assert_eq!(peer.effects.is_consumer, version == 1);
            assert_eq!(peer.state.temporary_color_frames, 0);
            let energy = actor.state.energy;
            world.touch_mint(&mut actor, &mut peer, 0).unwrap();
            assert_eq!(actor.state.energy, energy);
        }
    }

    #[test]
    fn lavender_keeps_wrapped_shields_and_java_integer_timer_division() {
        let (world, actor_ref, peer_ref) = setup(&[ColorKind::Lavender], &[ColorKind::Green]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.altruist = true;
        peer.effects.altruist = true;
        actor.effects.lavender = 10000;
        actor.effects.create_lavender = i32::MAX;
        peer.effects.lavender = 1;
        actor.state.reproduction.countdown = 0;
        peer.state.reproduction.maximum_countdown = 7;
        peer.state.reproduction.countdown = 100;
        world.touch_lavender(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(peer.effects.lavender, i32::MIN);
        assert_eq!(peer.state.reproduction.countdown, 7);
        actor.effects.create_lavender = 19;
        peer.state.reproduction.countdown = 8;
        world.touch_lavender(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(peer.state.reproduction.countdown, 8);
        peer.body.set_color_argb(0, ColorKind::Fruit.argb().unwrap()).unwrap();
        let shield = peer.effects.lavender;
        world.touch_lavender(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(peer.effects.lavender, shield);
    }

    #[test]
    fn magenta_restores_caches_after_partial_healing_and_retains_later_partial_payments() {
        let (world, actor_ref, peer_ref) =
            setup(&[ColorKind::Magenta], &[ColorKind::Green, ColorKind::Red]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.altruist = true;
        peer.effects.altruist = true;
        peer.effects.is_injured = true;
        peer.effects.is_frozen = true;
        peer.effects.all_frozen = true;
        peer.body.set_color_argb(0, ColorKind::Broken.argb().unwrap()).unwrap();
        peer.body.set_color_argb(1, ColorKind::LightBrown.argb().unwrap()).unwrap();
        world.session.0.parameters.borrow_mut().lifecycle.magenta_energy_consumption = 2.0;
        world.oxygen.set(2.5);
        world.touch_magenta(&mut actor, &mut peer, 0).unwrap();
        assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::Green.argb().unwrap());
        assert_eq!(
            peer.body.color_argb(1).unwrap(),
            ColorKind::LightBrown.argb().unwrap()
        );
        assert_eq!(actor.state.energy, 99.0);
        assert_eq!(peer.state.energy, 98.5);
        assert_eq!(world.oxygen.get(), 0.0);
        assert!(peer.effects.is_injured);
        assert!(!peer.effects.is_frozen && !peer.effects.all_frozen);
        assert_eq!(peer.body.update_effects, 2);
    }

    #[test]
    fn cooperative_ignored_colors_still_apply_the_strict_peer_death_guard() {
        for energy in [TOLERANCE, f64::from_bits(TOLERANCE.to_bits() - 1)] {
            let (world, actor_ref, peer_ref) = setup(&[ColorKind::Rose], &[ColorKind::Brown]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            peer.state.energy = energy;
            world.touch_rose(&mut actor, &mut peer, 0).unwrap();
            assert_eq!(peer.state.alive, energy == TOLERANCE);
            assert_eq!(actor.state.total_kills, i32::from(energy < TOLERANCE));
        }
    }
}
