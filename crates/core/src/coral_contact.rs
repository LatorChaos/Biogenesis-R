// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! CORAL touchEffects1 branch and its common peer-energy death guard

use super::death::Killer;
use super::infection::{InfectionKind, Infector};
use super::{Organism, OrganismRef, World, WorldError};
use crate::{ColorKind, TOLERANCE, physics::between};

fn eligible(actor: &Organism, peer: &Organism) -> bool {
    !peer.effects.is_plant
        && !peer.effects.is_consumer
        && (!peer.effects.is_fungus || (actor.effects.is_enhanced && !peer.effects.is_silver))
}

fn transfer_target(peer: &Organism) -> bool {
    !peer.effects.transfers_energy
        || peer.effects.is_infectious
        || peer.effects.is_coral
        || peer.effects.is_fungus
}

fn coral_tint(actor: &mut Organism) {
    actor.set_display_color(ColorKind::Coral.color_ref());
}

impl World {
    fn coral_indigo_payment(&self, actor: &mut Organism, peer: &mut Organism, extra: f64) -> Result<(), crate::WorldError> {
        Ok({
        let cost = (self.session.0.parameters.borrow().lifecycle.coral_energy_consumption + extra)
            / self.session.0.parameters.borrow().lifecycle.indigo_energy_consumption;
        if actor.use_energy(cost)? {
            peer.set_display_color(ColorKind::Indigo.color_ref());
            coral_tint(actor);
        }
    
        })
    }

    fn coral_convert(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
    ) -> Result<(), WorldError> {
        if actor.use_energy(
            self.session.0.parameters.borrow().lifecycle.coral_energy_consumption)? {
            coral_tint(actor);
            self.infect_from(peer, Infector::Other(&mut *actor), InfectionKind::Direct)?;
            if peer.state.energy >= 1.0 {
                self.transform_from(peer_ref, peer, Some((actor_ref, &*actor)))?;
            }
            // Failed placement still kills the original recipient
            self.die(peer, Some(Killer::Other(&mut *actor)))?;
        }
        Ok(())
    }

    fn coral_shield_attack(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
        mark_dodged: bool,
    ) -> Result<(), WorldError> {
        if peer.effects.lavender > 0 {
            if actor.effects.is_auburn
                && !actor.effects.is_plant
                && !actor.effects.is_consumer
                && !actor.effects.is_fungus
                && !peer.effects.is_fungus
            {
                self.weak_lavender_shield(peer);
            } else {
                self.lavender_shield(peer);
            }
            coral_tint(actor);
            if mark_dodged {
                peer.effects.has_dodged = true;
            }
        }
        if peer.effects.lavender <= 0 {
            self.coral_convert(actor_ref, actor, peer_ref, peer)?;
        }
        Ok(())
    }

    fn coral_dodge_attack(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
    ) -> Result<(), WorldError> {
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            peer.set_display_color(ColorKind::Teal.color_ref());
            coral_tint(actor);
            peer.effects.has_dodged = true;
        } else {
            if peer.effects.antiviral > 0 {
                self.virus_neutralized(actor)?;
            }
            self.coral_shield_attack(actor_ref, actor, peer_ref, peer, true)?;
        }
        Ok(())
    }

    /// Retain held objects through infection, transformation, death, and the final energy check
    pub(super) fn coral_contact(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        match ColorKind::from_argb(peer.body.color_argb(peer_segment)?) {
            Coral => {
                if eligible(actor, peer)
                    && peer.effects.indigo > 0
                    && !peer.effects.is_fungus
                    && (actor.effects.indigo == 0
                        || actor.effects.is_plant
                        || actor.effects.is_consumer
                        || actor.effects.is_fungus)
                {
                    self.coral_indigo_payment(actor, peer, 0.1)?;
                }
            }
            White | Silver | Plague => {
                if eligible(actor, peer) && transfer_target(peer) {
                    if peer.effects.indigo > 0 && !peer.effects.is_fungus {
                        self.coral_indigo_payment(actor, peer, 0.01)?;
                        if peer.effects.antiviral > 0 {
                            self.virus_neutralized(actor)?;
                        }
                    } else {
                        self.coral_dodge_attack(actor_ref, actor, peer_ref, peer)?;
                    }
                }
            }
            Indigo => {
                if eligible(actor, peer) {
                    if !peer.effects.is_infectious {
                        self.coral_indigo_payment(actor, peer, 0.1)?;
                        if peer.effects.antiviral > 0 {
                            self.virus_neutralized(actor)?;
                        }
                    } else {
                        self.coral_dodge_attack(actor_ref, actor, peer_ref, peer)?;
                    }
                }
            }
            Teal | Cyan | Auburn | Blond | Flower => {
                if eligible(actor, peer) && transfer_target(peer) {
                    self.coral_dodge_attack(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Yellow | DarkGray | Gold | Vision => {
                if eligible(actor, peer) && peer.state.active && transfer_target(peer) {
                    if peer.effects.antiviral > 0 {
                        self.virus_neutralized(actor)?;
                    }
                    if peer.effects.is_fungus {
                        self.coral_shield_attack(actor_ref, actor, peer_ref, peer, false)?;
                    }
                }
            }
            Spore => {
                if eligible(actor, peer) && transfer_target(peer) {
                    if peer.effects.antiviral > 0 {
                        self.virus_neutralized(actor)?;
                    }
                    if peer.effects.spore_version == 6 {
                        self.coral_shield_attack(actor_ref, actor, peer_ref, peer, false)?;
                    }
                }
            }
            LightBrown | GreenBrown | DarkFire | Broken | Ice => {
                if eligible(actor, peer) && transfer_target(peer) {
                    self.coral_convert(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Magenta | Rose => {
                if !actor.effects.altruist && eligible(actor, peer) && transfer_target(peer) {
                    self.coral_dodge_attack(actor_ref, actor, peer_ref, peer)?;
                }
            }
            Lavender => {
                if eligible(actor, peer) && transfer_target(peer) {
                    if peer.effects.antiviral > 0 {
                        self.virus_neutralized(actor)?;
                    }
                    let (cost, increment) = if peer.effects.is_enhanced {
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
                        between(cost, 0.0, actor.state.energy))?;
                    let shield = self.session.0.parameters.borrow().effects.lavender_shield;
                    if peer.effects.lavender < shield {
                        peer.effects.lavender = peer.effects.lavender.wrapping_add(increment);
                        if peer.effects.lavender >= shield {
                            peer.effects.lavender = shield;
                        }
                    }
                    peer.set_display_color(Lavender.color_ref());
                    actor.set_display_color(DeadBark.color_ref());
                    if actor.state.energy < TOLERANCE {
                        self.die(actor, Some(Killer::Other(&mut *peer)))?;
                    }
                }
            }
            Blue | Sky | DeepSky | Ochre | Olive | DarkOlive | Violet | Gray | Lilac
            | DarkLilac | Spike | SpikePoint | Fallow | Mint => {
                if eligible(actor, peer)
                    && peer.state.active
                    && transfer_target(peer)
                    && peer.effects.antiviral > 0
                {
                    self.virus_neutralized(actor)?;
                }
            }
            Fruit => {
                if peer.effects.spore_version == 1 {
                    if peer.state.temporary_color_frames > 0
                        || actor.effects.is_plant
                        || (actor.effects.is_auburn
                            && !actor.effects.is_consumer
                            && !actor.effects.is_fungus)
                    {
                        self.coral_convert(actor_ref, actor, peer_ref, peer)?;
                    } else {
                        peer.set_display_color(Broken.color_ref());
                    }
                }
            }
            Bark | OldBark | DeadBark | Brown => {}
            _ => {
                if eligible(actor, peer) && transfer_target(peer) {
                    if peer.effects.antiviral > 0 {
                        self.virus_neutralized(actor)?;
                    }
                    self.coral_shield_attack(actor_ref, actor, peer_ref, peer, false)?;
                }
            }
        }
        if peer.state.alive && peer.state.energy < TOLERANCE {
            self.die(peer, Some(Killer::Other(actor)))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gene, Genome, GenomeProperties, WorldConfig, WorldParameters};
    use std::rc::Rc;

    fn pair(peer_color: ColorKind) -> (World, OrganismRef, OrganismRef) {
        let mut parameters = WorldParameters::default();
        parameters.genetics.min_mutation_rate = 0;
        parameters.genetics.meta_mutation_rate = 0;
        let world = World::with_parameters(
            WorldConfig {
                initial_attempts: 0,
                ..WorldConfig::default()
            },
            parameters,
        )
        .unwrap();
        for (color, center) in [(ColorKind::Coral, [500, 500]), (peer_color, [1500, 1500])] {
            let genome = Genome::from_parts(
                &[Gene::new(12.0, 0.0, color.argb(), -1, [0; 36]).shared()],
                GenomeProperties {
                    symmetry: 1,
                    ..GenomeProperties::default()
                },
            );
            world.paste_genome(genome, center).unwrap().unwrap();
        }
        let actor = Rc::clone(&world.organisms.borrow()[0]);
        let peer = Rc::clone(&world.organisms.borrow()[1]);
        {
            let mut actor = actor.borrow_mut();
            actor.state.energy = 20.0;
            actor.effects.is_plant = false;
            actor.effects.is_consumer = false;
            actor.effects.is_fungus = false;
        }
        {
            let mut peer = peer.borrow_mut();
            peer.state.energy = 0.5;
            peer.effects.is_plant = false;
            peer.effects.is_consumer = false;
            peer.effects.is_fungus = false;
            peer.effects.transfers_energy = false;
            peer.effects.indigo = 0;
            peer.effects.antiviral = 0;
            peer.effects.lavender = 0;
            peer.effects.dodge = false;
            peer.effects.has_dodged = false;
        }
        (world, actor, peer)
    }

    fn contact(world: &mut World, actor: &OrganismRef, peer: &OrganismRef) {
        world
            .coral_contact(
                actor,
                &mut actor.borrow_mut(),
                peer,
                &mut peer.borrow_mut(),
                0,
            )
            .unwrap();
    }

    #[test]
    fn failed_transformation_still_kills_and_records_the_infection() {
        let (mut world, actor, peer) = pair(ColorKind::White);
        let blocker = Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), -1, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        world.paste_genome(blocker, [2500, 2500]).unwrap().unwrap();
        world.organisms.borrow()[2].borrow().state.bounds.set(crate::IntRect {
            x: 1400,
            y: 1400,
            width: 200,
            height: 200,
        });
        peer.borrow_mut().state.energy = 2.0;
        world.rebuild_spatial();
        let random = world.random_snapshot();
        let next_id = world.next_id.get();
        contact(&mut world, &actor, &peer);
        assert_eq!(world.organisms.borrow().len(), 3);
        assert_eq!(world.population(), 2);
        assert_eq!(world.next_id.get(), next_id);
        assert_ne!(world.random_snapshot(), random);
        assert_eq!(actor.borrow().state.total_infected, 1);
        assert_eq!(actor.borrow().state.total_kills, 1);
        assert!(!peer.borrow().state.alive);
        assert!(peer.borrow().infected_genome.is_none());
        assert!(peer.borrow().saved_genome.is_none());
        assert_eq!(world.statistics_ref().borrow().get_created_organisms(), 3);
    }

    #[test]
    fn immunity_mutates_the_attacker_then_continues_with_the_weak_shield() {
        let (mut world, actor, peer) = pair(ColorKind::White);
        actor.borrow_mut().effects.is_auburn = true;
        actor.borrow_mut().effects.gold = 1;
        peer.borrow_mut().effects.antiviral = 1;
        peer.borrow_mut().effects.lavender = 2221;
        let random = world.random_snapshot();
        contact(&mut world, &actor, &peer);
        let actor = actor.borrow();
        let peer = peer.borrow();
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::LightBrown.argb().unwrap()
        );
        assert_eq!(
            actor.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow().color(),
            ColorKind::Coral.argb()
        );
        assert!(actor.effects.is_injured && actor.effects.is_coral);
        assert_eq!(actor.body.update_effects, 2);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Coral.argb());
        assert_eq!(actor.state.energy, 20.0);
        assert_eq!(peer.effects.lavender, 1);
        assert!(peer.effects.has_dodged);
        assert_eq!(peer.state.temporary_argb(), ColorKind::LightBlue.argb());
        assert!(peer.state.alive);
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn normal_shields_keep_the_source_has_dodged_asymmetry() {
        for (kind, fungus, spore, mark) in [
            (ColorKind::White, false, 0, true),
            (ColorKind::Green, false, 0, false),
            (ColorKind::Spore, false, 6, false),
            (ColorKind::Yellow, true, 0, false),
        ] {
            let (mut world, actor, peer) = pair(kind);
            actor.borrow_mut().effects.is_enhanced = fungus;
            {
                let mut peer = peer.borrow_mut();
                peer.effects.is_fungus = fungus;
                peer.effects.spore_version = spore;
                peer.effects.lavender = 186;
                peer.state.active = true;
            }
            contact(&mut world, &actor, &peer);
            assert_eq!(peer.borrow().effects.lavender, 1, "{kind:?}");
            assert_eq!(peer.borrow().effects.has_dodged, mark, "{kind:?}");
            assert!(peer.borrow().state.alive);
            assert_eq!(actor.borrow().state.total_infected, 0);
        }
    }

    #[test]
    fn failed_dodge_and_attack_preserve_ordered_partial_payments() {
        for (peer_energy, expected_actor, expected_peer) in [(1.0, 1.0, 0.875), (0.1, 0.875, 0.1)] {
            let (mut world, actor, peer) = pair(ColorKind::White);
            world.session.0.parameters.borrow_mut().lifecycle.dodge_energy_consumption = 0.25;
            world.session.0.parameters.borrow_mut().lifecycle.coral_energy_consumption = 0.25;
            world.oxygen.set(0.125);
            world.carbon_dioxide.set(0.0);
            actor.borrow_mut().state.energy = 1.0;
            peer.borrow_mut().state.energy = peer_energy;
            peer.borrow_mut().effects.dodge = true;
            let random = world.random_snapshot();
            contact(&mut world, &actor, &peer);
            assert_eq!(actor.borrow().state.energy, expected_actor);
            assert_eq!(peer.borrow().state.energy, expected_peer);
            assert_eq!(world.oxygen.get(), 0.0);
            assert_eq!(world.carbon_dioxide.get(), 0.125);
            assert_eq!(actor.borrow().state.total_infected, 0);
            assert!(!peer.borrow().effects.has_dodged);
            assert!(peer.borrow().state.alive);
            assert_eq!(world.random_snapshot(), random);
        }
    }

    #[test]
    fn depleted_shield_converts_below_one_energy_without_constructing_a_child() {
        let (mut world, actor, peer) = pair(ColorKind::White);
        peer.borrow_mut().effects.lavender = 185;
        let random = world.random_snapshot();
        contact(&mut world, &actor, &peer);
        assert_eq!(peer.borrow().effects.lavender, 0);
        assert!(peer.borrow().effects.has_dodged);
        assert!(!peer.borrow().state.alive);
        assert_eq!(actor.borrow().state.total_infected, 1);
        assert_eq!(actor.borrow().state.total_kills, 1);
        assert_eq!(world.organisms.borrow().len(), 2);
        assert_eq!(world.population(), 1);
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn fruit_warning_changes_only_the_tint_then_enables_conversion() {
        let (mut world, actor, peer) = pair(ColorKind::Fruit);
        peer.borrow_mut().effects.spore_version = 1;
        peer.borrow_mut().state.temporary_color_frames = 0;
        contact(&mut world, &actor, &peer);
        assert_eq!(
            peer.borrow().body.color_argb(0).unwrap(),
            ColorKind::Fruit.argb().unwrap()
        );
        assert_eq!(
            peer.borrow().state.temporary_argb(),
            ColorKind::Broken.argb()
        );
        assert_eq!(actor.borrow().state.total_infected, 0);
        contact(&mut world, &actor, &peer);
        assert!(!peer.borrow().state.alive);
        assert_eq!(actor.borrow().state.total_infected, 1);
        assert_eq!(actor.borrow().state.total_kills, 1);
    }

    #[test]
    fn lavender_counter_wraps_and_exhaustion_kills_the_attacker() {
        let (mut world, actor, peer) = pair(ColorKind::Lavender);
        actor.borrow_mut().state.energy = 0.1;
        peer.borrow_mut().effects.lavender = 9999;
        world.oxygen.set(1.0);
        contact(&mut world, &actor, &peer);
        assert!(!actor.borrow().state.alive);
        assert_eq!(
            actor.borrow().state.temporary_argb(),
            ColorKind::DeadBark.argb()
        );
        assert_eq!(
            peer.borrow().state.temporary_argb(),
            ColorKind::Lavender.argb()
        );
        assert_eq!(peer.borrow().effects.lavender, 10000);
        assert_eq!(peer.borrow().state.total_kills, 1);
        assert_eq!(world.population(), 1);

        let (mut world, actor, peer) = pair(ColorKind::Lavender);
        world.session.0.parameters.borrow_mut().effects.lavender_shield = i32::MAX;
        peer.borrow_mut().effects.is_enhanced = true;
        peer.borrow_mut().effects.lavender = i32::MAX - 500;
        contact(&mut world, &actor, &peer);
        assert_eq!(peer.borrow().effects.lavender, i32::MIN + 499);
        assert_eq!(actor.borrow().state.energy, 19.0);
        assert!(actor.borrow().state.alive);
    }

    #[test]
    fn no_op_branches_still_apply_the_strict_common_death_guard() {
        for (color, energy, dead) in [
            (ColorKind::Bark, 0.0, true),
            (ColorKind::Brown, -0.1, true),
            (ColorKind::Magenta, 0.0, true),
            (ColorKind::Bark, TOLERANCE, false),
        ] {
            let (mut world, actor, peer) = pair(color);
            actor.borrow_mut().effects.altruist = true;
            peer.borrow_mut().state.energy = energy;
            contact(&mut world, &actor, &peer);
            assert_eq!(!peer.borrow().state.alive, dead, "{color:?}");
            assert_eq!(actor.borrow().state.total_kills, i32::from(dead));
            assert_eq!(actor.borrow().state.total_infected, 0);
        }
    }

    #[test]
    fn indigo_penalty_rejects_partial_payment_but_still_neutralizes_the_attacker() {
        let (mut world, actor, peer) = pair(ColorKind::Indigo);
        peer.borrow_mut().effects.transfers_energy = true;
        peer.borrow_mut().effects.is_infectious = false;
        peer.borrow_mut().effects.antiviral = 1;
        actor.borrow_mut().effects.gold = 1;
        actor
            .borrow_mut()
            .set_display_color(ColorKind::Gray.color_ref());
        world.session.0.parameters.borrow_mut().lifecycle.indigo_energy_consumption = 2.0;
        world.oxygen.set(0.03);
        world.carbon_dioxide.set(0.0);
        contact(&mut world, &actor, &peer);
        let actor = actor.borrow();
        assert_eq!(actor.state.energy, 19.97);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 0.03);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Gray.argb());
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::LightBrown.argb().unwrap()
        );
        assert!(actor.effects.is_injured);
        assert_eq!(actor.body.update_effects, 2);
        assert_eq!(actor.state.total_infected, 0);
        assert!(peer.borrow().state.alive);
    }

    #[test]
    fn coral_notification_error_keeps_payment_and_infection_before_transform_and_death() {
        let (world, actor_ref, peer_ref) = pair(ColorKind::White);
        world.session.0.parameters.borrow_mut().lifecycle.coral_energy_consumption = 2.0;
        world.oxygen.set(100.0);
        world.carbon_dioxide.set(3.0);
        let random = world.random_snapshot();
        let next_id = world.next_id.get();
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.state.total_infected = i32::MAX;
        peer.state.world = Default::default();
        peer.state.energy = 5.0;
        peer.state.reproduction.infected_energy = 77;
        peer.saved_genome = peer.genome.clone();
        assert_eq!(world.coral_convert(&actor_ref, &mut actor, &peer_ref, &mut peer), Err(WorldError::MissingWorld));
        assert_eq!(actor.state.energy, 18.0);
        assert_eq!(world.oxygen.get(), 98.0);
        assert_eq!(world.carbon_dioxide.get(), 5.0);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Coral.argb());
        assert_eq!(actor.state.total_infected, i32::MIN);
        assert_eq!(actor.state.total_kills, 0);
        assert!(Rc::ptr_eq(peer.infected_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
        assert!(peer.saved_genome.is_none());
        assert_eq!(peer.state.reproduction.infected_energy, 77);
        assert!(peer.state.alive && peer.state.active);
        assert_eq!(peer.state.energy, 5.0);
        assert_eq!(world.organisms.borrow().len(), 2);
        assert_eq!(world.population(), 2);
        assert!(world.statistics_ref().borrow().get_last_infected_being().is_none());
        assert!(world.statistics_ref().borrow().get_last_dead_being().is_none());
        assert_eq!(world.next_id.get(), next_id);
        assert_eq!(world.random_snapshot(), random);
    }
}
