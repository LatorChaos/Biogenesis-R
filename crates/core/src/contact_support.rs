// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::organism::primary_genome;

use super::{Organism, World};
use crate::{ColorKind, physics::between};
use crate::organism::same_genome;

fn familial(actor: &Organism, peer: &Organism) -> Result<bool, crate::WorldError> {
    Ok(primary_genome(&actor.genome)?.borrow().modifiers().familial
        && (actor.effects.friend_length == peer.effects.friend_length
            || actor.effects.friend_theta == peer.effects.friend_theta
            || actor.state.parent_id == peer.id as i32
            || actor.id as i32 == peer.state.parent_id
            || (actor.state.parent_id == peer.state.parent_id && actor.state.parent_id != -1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use crate::{Gene, Genome, GenomeModifiers, GenomeProperties, WorldConfig};

    fn organism(world: &World, colors: &[ColorKind], modifiers: GenomeModifiers) -> Organism {
        let genes: Vec<_> = colors
            .iter()
            .enumerate()
            .map(|(index, color)| Gene::new(12.0, index as f64, color.argb(), -1, [0; 36]).shared())
            .collect();
        Organism::unplaced(
            Rc::new(std::cell::RefCell::new(Genome::from_parts(
                &genes,
                GenomeProperties {
                    symmetry: 1,
                    modifiers,
                    ..GenomeProperties::default()
                },
            ))),
            crate::OrganismState { world: world.association(), ..Default::default() },
        )
        .unwrap()
    }

    fn world() -> World {
        World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap()
    }

    #[test]
    fn healing_keeps_the_first_partial_payment_when_the_second_payment_fails() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Magenta], GenomeModifiers::default());
        let mut peer = organism(&world, &[ColorKind::GreenBrown], GenomeModifiers::default());
        actor.effects.use_friend_effects = 2;
        actor.effects.altruist = true;
        peer.effects.altruist = true;
        peer.effects.is_injured = true;
        peer.state.active = true;
        actor.state.energy = 1.0;
        peer.state.energy = 1.0;
        world.oxygen.set(0.005);
        let carbon = world.carbon_dioxide.get();
        world.touch_friend_effects(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.state.energy, 0.995);
        assert_eq!(actor.state.energy, 1.0);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), carbon + 0.005);
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::GreenBrown.argb().unwrap()
        );
        assert!(peer.effects.is_injured);
    }

    #[test]
    fn mint_checks_genotype_identity_and_treats_delayed_reproduction_as_a_separate_payment() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Mint], GenomeModifiers::default());
        let mut peer = organism(&world, &[ColorKind::Green], GenomeModifiers::default());
        actor.effects.use_friend_effects = 2;
        actor.effects.altruist = true;
        peer.effects.altruist = true;
        actor.state.energy = 10.0;
        peer.state.energy = 10.0;
        world.oxygen.set(100.0);
        peer.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
        peer.saved_genome = Some(Rc::clone(peer.genome.as_ref().unwrap()));
        peer.effects.reproduce_late = -1;
        world.touch_friend_effects(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(Rc::ptr_eq(
            peer.infected_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
        assert!(peer.saved_genome.is_some());
        assert_eq!(peer.effects.reproduce_late, 0);
        assert_eq!(peer.state.energy, 10.0 - 0.075 / 2.0);
        peer.infected_genome = Some(Rc::clone(peer.genome.as_ref().unwrap()));
        world.touch_friend_effects(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(peer.infected_genome.is_none() && peer.saved_genome.is_none());
    }

    #[test]
    fn lavender_keeps_wrapping_arithmetic_and_the_inhibition_tint_gate() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Lavender], GenomeModifiers::default());
        actor.effects.lavender = 186;
        actor.state.temporary_color_frames = 10;
        actor.set_display_color(ColorKind::Rose.color_ref());
        world.lavender_inhibition(&mut actor);
        assert_eq!(actor.effects.lavender, 1);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Rose.argb());
        actor.effects.lavender = i32::MIN;
        world.weak_lavender_shield(&mut actor);
        assert_eq!(actor.effects.lavender, i32::MIN.wrapping_sub(2220));
        assert_eq!(actor.state.temporary_argb(), ColorKind::LightBlue.argb());
        actor.effects.lavender = 185;
        world.lavender_shield(&mut actor);
        assert_eq!(actor.effects.lavender, 0);
        assert_eq!(actor.state.temporary_argb(), ColorKind::LightBlue.argb());
    }

    #[test]
    fn mimic_copies_all_reaction_channels_without_changing_the_genotype() {
        let world = world();
        let mut actor = organism(&world, 
            &[ColorKind::Dark, ColorKind::Dark],
            GenomeModifiers {
                mimic_all: true,
                ..GenomeModifiers::default()
            },
        );
        let mut peer = organism(&world, &[ColorKind::Red], GenomeModifiers::default());
        peer.body.set_reaction_values(0, std::array::from_fn(|index| index as i32 - 18)).unwrap();
        actor.effects.segment_pre_touch_effects = 2;
        world.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
        for color_index in actor.body.indices() {
            assert_eq!(actor.body.color_argb(color_index).unwrap(), ColorKind::Red.argb().unwrap());
            assert!(actor.body.required_color(color_index).unwrap().same_instance(&peer.body.required_color(0).unwrap()));
            assert_eq!(actor.body.reaction_values(color_index).unwrap(), peer.body.reaction_values(0).unwrap());
        }
        assert_eq!(
            actor.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow().color(),
            ColorKind::Dark.argb()
        );
    }

    #[test]
    fn forest_contacts_distinguish_first_repeat_and_dark_gray_support() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Forest], GenomeModifiers::default());
        let mut peer = organism(&world, &[ColorKind::Forest], GenomeModifiers::default());
        actor.effects.forest_photo = 2.0;
        actor.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![1.0]));
        peer.effects.photosynthesis = 1.0;
        peer.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![1.0]));
        world.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(
            actor.effects.colony_photosynthesis.to_bits(),
            (1.01_f64 * 2.0).to_bits()
        );
        world.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(
            actor.effects.colony_photosynthesis.to_bits(),
            (1.01_f64 * 2.0 + 0.98 * 2.0).to_bits()
        );
        actor.body.set_color_argb(0, ColorKind::DarkGray.argb().unwrap()).unwrap();
        actor.effects.is_consumer = true;
        actor.effects.colony_photosynthesis = 0.0;
        world.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.effects.colony_photosynthesis, 10.0);
        peer.effects.is_enhanced = true;
        world.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.effects.colony_photosynthesis, 10.0);
    }

    #[test]
    fn default_pretouch_does_not_classify_a_null_peer_color() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Red], GenomeModifiers::default());
        let mut peer = organism(&world, &[ColorKind::Green], GenomeModifiers::default());
        actor.effects.segment_pre_touch_effects = 2;
        actor.effects.forest_photo = 0.0;
        peer.body.set_color(0, None).unwrap();
        world.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(peer.body.color(0).unwrap().is_none());
    }

    #[test]
    fn cached_colony_pair_does_not_read_either_nullable_color_array() {
        let world = world();
        let mut actor = organism(&world, &[ColorKind::Forest], GenomeModifiers::default());
        let mut peer = organism(&world, &[ColorKind::Forest], GenomeModifiers::default());
        actor.effects.segment_pre_touch_effects = 0;
        actor.effects.forest_photo = 2.0;
        actor.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![0.0]));
        actor.effects.colony_photosynthesis = 0.0;
        peer.effects.photosynthesis = 1.0;
        peer.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![-1.0]));
        actor.body.phenotype_colors = None;
        peer.body.phenotype_colors = None;
        world.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.effects.colony_photosynthesis.to_bits(), (0.31_f64 * 2.0).to_bits());
    }

    #[test]
    fn corpse_methane_uses_actor_world_and_preserves_distinct_zero_guards() {
        let caller = world();
        let target = World::empty_in_session(caller.config.get(), caller.session());
        caller.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        caller.methane.set(20.0);
        caller.oxygen.set(50.0);
        for kind in [ColorKind::Purple, ColorKind::DarkGray] {
            let mut actor = organism(&target, &[kind], GenomeModifiers::default());
            let mut peer = organism(&target, &[ColorKind::Brown], GenomeModifiers::default());
            actor.effects.segment_pre_touch_effects = 2;
            actor.effects.is_enhanced = true;
            actor.effects.is_killer = false;
            actor.effects.forest_photo = 0.0;
            actor.effects.colony_photosynthesis = 0.0;
            actor.effects.methanotrophy = 1.0;
            actor.state.energy = 10.0;
            peer.state.alive = false;
            target.methane.set(2.0);
            target.oxygen.set(5.0);
            caller.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(actor.state.energy, 11.0);
            assert_eq!((target.methane.get(), target.oxygen.get()), (1.0, 6.0));
            actor.state.world = Default::default();
            actor.effects.methanotrophy = 0.0;
            let result = caller.pre_touch_effects(&mut actor, &mut peer, 0, 0);
            assert_eq!(result, if kind == ColorKind::Purple { Err(crate::WorldError::MissingWorld) } else { Ok(()) });
            actor.effects.methanotrophy = 1.0;
            assert_eq!(caller.pre_touch_effects(&mut actor, &mut peer, 0, 0), Err(crate::WorldError::MissingWorld));
            peer.state.alive = true;
            caller.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
            peer.body.phenotype_colors = None;
            assert!(matches!(caller.pre_touch_effects(&mut actor, &mut peer, 0, 0), Err(crate::WorldError::Geometry(_))));
            assert_eq!(actor.state.energy, 11.0);
        }
        assert_eq!((caller.methane.get(), caller.oxygen.get()), (20.0, 50.0));
    }

    #[test]
    fn rose_credits_peer_through_actor_world_and_ignores_peer_association() {
        let caller = world();
        let target = World::empty_in_session(caller.config.get(), caller.session());
        let peer_world = World::empty_in_session(caller.config.get(), caller.session());
        caller.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        caller.carbon_dioxide.set(100.0);
        caller.methane.set(20.0);
        caller.oxygen.set(50.0);
        peer_world.carbon_dioxide.set(200.0);
        peer_world.methane.set(30.0);
        peer_world.oxygen.set(60.0);
        for peer_present in [true, false] {
            let mut actor = organism(&target, &[ColorKind::Rose], GenomeModifiers::default());
            let mut peer = organism(&peer_world, &[ColorKind::Green], GenomeModifiers::default());
            actor.effects.segment_pre_touch_effects = 0;
            actor.effects.forest_photo = 0.0;
            actor.effects.colony_photosynthesis = 2.0;
            actor.effects.is_plant = false;
            actor.state.temporary_color_frames = 0;
            actor.state.energy = 10.0;
            peer.effects.is_plant = true;
            peer.effects.photosynthesis = 4.0;
            peer.effects.methanotrophy = 4.0;
            peer.effects.forest_photo = 0.0;
            peer.effects.transfers_energy = false;
            peer.state.active = true;
            peer.state.energy = 10.0;
            if !peer_present { peer.state.world = Default::default(); }
            target.carbon_dioxide.set(10.0);
            target.methane.set(10.0);
            target.oxygen.set(0.0);
            caller.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(peer.state.energy, 19.0);
            assert_eq!(actor.state.energy, 10.0);
            assert_eq!(actor.state.temporary_argb(), ColorKind::Rose.argb());
            assert_eq!((target.carbon_dioxide.get(), target.methane.get(), target.oxygen.get()), (2.0, 9.0, 9.0));
            actor.state.world = Default::default();
            actor.state.temporary_color = None;
            actor.state.temporary_color_frames = 0;
            assert_eq!(caller.pre_touch_effects(&mut actor, &mut peer, 0, 0), Err(crate::WorldError::MissingWorld));
            assert_eq!(peer.state.energy, 19.0);
            assert!(actor.state.temporary_color.is_none());
            actor.state.temporary_color_frames = 1;
            caller.pre_touch_effects(&mut actor, &mut peer, 0, 0).unwrap();
        }
        assert_eq!((caller.carbon_dioxide.get(), caller.methane.get(), caller.oxygen.get()), (100.0, 20.0, 50.0));
        assert_eq!((peer_world.carbon_dioxide.get(), peer_world.methane.get(), peer_world.oxygen.get()), (200.0, 30.0, 60.0));
    }

}

fn supports(actor: &Organism, peer: &Organism) -> Result<bool, crate::WorldError> {
    Ok((actor.effects.altruist && peer.effects.altruist) || familial(actor, peer)?)
}

fn sick(kind: ColorKind) -> bool {
    use ColorKind::*;
    matches!(
        kind,
        LightBrown | GreenBrown | Broken | LightBlue | Ice | DarkFire
    )
}

fn colony_category(kind: ColorKind) -> usize {
    use ColorKind::*;
    match kind {
        Forest => 0,
        Bark | OldBark => 1,
        Purple | GreenBrown | DeadBark | Ice => 2,
        DarkGray => 3,
        _ => 4,
    }
}

impl World {
    fn mimic_segment(actor: &mut Organism, peer: &Organism, index: usize, peer_index: usize) -> Result<(), crate::WorldError> {
        let destination = actor.body.phenotype_colors.clone();
        let value = if peer.body.color_argb(peer_index)? == ColorKind::Bark.argb().unwrap()
            || peer.body.color_argb(peer_index)? == ColorKind::OldBark.argb().unwrap()
        {
            if actor.effects.is_plant { ColorKind::Bark } else { ColorKind::OldBark }.color_ref()
        } else {
            peer.body.color(peer_index)?
        };
        crate::Body::store_color(destination, index, value)?;
        for channel in 0..36 {
            let destination = actor.body.arrays.reactions[channel].clone();
            let value = peer.body.reaction(peer_index, channel)?;
            crate::OrganismArrays::store_int(destination, crate::IntField::Reaction(channel), index as i32, value)?;
        }
        Ok(())
    }

    pub(super) fn virus_neutralized(&self, organism: &mut Organism) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        for color_index in organism.body.indices() {
            if matches!(ColorKind::from_argb(organism.body.color_argb(color_index)?), White | Plague | Coral) {
                organism.body.set_color(color_index, if organism.effects.is_plant {
                    Broken
                } else {
                    LightBrown
                }
                .color_ref())?;
                organism.effects.is_injured = true;
                if !organism.effects.is_plant
                    && organism.effects.gold > 0
                    && !organism.effects.is_consumer
                    && !organism.effects.is_fungus
                {
                    organism.body.update_effects = 2;
                }
            }
        }
        Ok(())
    }

    pub(super) fn lavender_inhibition(&self, organism: &mut Organism) {
        Self::drain_lavender(organism, 185, true);
    }

    pub(super) fn lavender_shield(&self, organism: &mut Organism) {
        Self::drain_lavender(organism, 185, false);
    }

    pub(super) fn weak_lavender_shield(&self, organism: &mut Organism) {
        Self::drain_lavender(organism, 2220, false);
    }

    fn drain_lavender(organism: &mut Organism, amount: i32, preserve_tint: bool) {
        organism.effects.lavender = organism.effects.lavender.wrapping_sub(amount);
        if organism.effects.lavender <= 0 {
            organism.effects.lavender = 0;
        } else if !preserve_tint || organism.state.temporary_color_frames < 10 {
            organism.set_display_color(ColorKind::LightBlue.color_ref());
        }
    }

    pub(super) fn pre_touch_effects(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        if actor.effects.segment_pre_touch_effects == 2 {
            match ColorKind::from_argb(actor.body.color_argb(segment)?) {
                Purple
                    if peer.body.color_kind(peer_segment)? == Brown
                        && actor.effects.is_enhanced
                        && !actor.effects.is_killer
                        && !peer.state.alive =>
                {
                    actor.state.energy += actor.state.world.resolve()?.methanotrophy(actor.effects.methanotrophy);
                }
                DarkGray
                    if peer.body.color_kind(peer_segment)? == Brown
                        && actor.effects.methanotrophy > 0.0
                        && !actor.effects.is_killer
                        && !peer.state.alive =>
                {
                    actor.state.energy += actor.state.world.resolve()?.methanotrophy(actor.effects.methanotrophy);
                }
                Dark if !matches!(
                    peer.body.color_kind(peer_segment)?,
                    Dark | Green
                        | Grass
                        | Forest
                        | Spring
                        | Summer
                        | Winter
                        | Lime
                        | Leaf
                        | C4
                        | Jade
                        | DarkJade
                        | DarkGreen
                        | Purple
                        | Spore
                        | Fruit
                        | Vision
                        | DarkFire
                        | Ice
                        | LightBlue
                        | LightBrown
                        | GreenBrown
                        | Broken
                        | DeadBark
                        | Brown
                ) =>
                {
                    let mimic_all = primary_genome(&actor.genome)?.borrow().modifiers().mimic_all;
                    if mimic_all {
                        for index in actor.body.indices() {
                            if actor.body.color_kind(index)? == Dark {
                                Self::mimic_segment(actor, peer, index, peer_segment)?;
                            }
                        }
                    } else {
                        Self::mimic_segment(actor, peer, segment, peer_segment)?;
                    }
                    actor.effects.apply(
                        Some(&primary_genome(&actor.genome)?.borrow()),
                        &mut actor.body,
                        actor.state.age,
                        &mut actor.state.reproduction,
                        &self.session.0.parameters.borrow().effects,
                    )?;
                }
                _ => {}
            }
        }
        if actor.effects.forest_photo > 0.0 {
            if peer.effects.photosynthesis > 0.0 {
                self.colony_contact(actor, peer, segment, peer_segment)?;
            }
        } else if actor.state.temporary_color_frames == 0
            && actor.effects.colony_photosynthesis > 0.0
            && !actor.effects.is_plant
            && peer.effects.is_plant
            && peer.state.active
        {
            if peer.effects.photosynthesis > 0.0 {
                let photo = if peer.effects.transfers_energy {
                    if peer.effects.forest_photo > 0.0 {
                        actor.effects.colony_photosynthesis
                            * ((1.5 * peer.effects.photosynthesis)
                                + (0.375 * peer.effects.forest_photo))
                    } else {
                        actor.effects.colony_photosynthesis * (1.5 * peer.effects.photosynthesis)
                    }
                } else if peer.effects.forest_photo > 0.0 {
                    actor.effects.colony_photosynthesis
                        * (peer.effects.photosynthesis + (0.25 * peer.effects.forest_photo))
                } else {
                    actor.effects.colony_photosynthesis * peer.effects.photosynthesis
                };
                peer.state.energy += actor.state.world.resolve()?.photosynthesis(photo);
                actor.set_display_color(Rose.color_ref());
            }
            if peer.effects.methanotrophy > 0.0 {
                let methane = actor.effects.colony_photosynthesis
                    * (if peer.effects.transfers_energy {
                        0.1875
                    } else {
                        0.125
                    } * peer.effects.methanotrophy);
                peer.state.energy += actor.state.world.resolve()?.methanotrophy(methane);
                actor.set_display_color(Rose.color_ref());
            }
        }
        Ok(())
    }

    fn colony_contact(
        &self,
        actor: &mut Organism,
        peer: &Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        let actor_photo = actor.body.photo(segment)?;
        let pair = if actor_photo <= 0.0 {
            let peer_photo = peer.body.photo(peer_segment)?;
            if peer_photo <= 0.0 {
                (0.31, 0.28)
            } else {
                match colony_category(peer.body.color_kind(peer_segment)?) {
                    0 => (0.51, 0.48),
                    1 | 2 => (0.31, 0.28),
                    3 => return Ok(()),
                    _ => (0.41, 0.38),
                }
            }
        } else {
            let actor_category = colony_category(actor.body.color_kind(segment)?);
            if actor_category == 3 {
                if !peer.effects.is_enhanced {
                    actor.effects.colony_photosynthesis +=
                        if !actor.effects.is_consumer && !actor.effects.is_killer {
                            self.session.0.parameters.borrow().lifecycle.dark_gray_energy_consumption * actor.effects.forest_photo
                        } else {
                            0.5 * self.session.0.parameters.borrow().lifecycle.dark_gray_energy_consumption * actor.effects.forest_photo
                        };
                }
                return Ok(());
            }
            let peer_photo = peer.body.photo(peer_segment)?;
            if peer_photo <= 0.0 {
                match actor_category {
                    0 => (0.51, 0.48),
                    1 | 2 => (0.31, 0.28),
                    _ => (0.41, 0.38),
                }
            } else {
                match (actor_category, colony_category(peer.body.color_kind(peer_segment)?)) {
                    (_, 3) => return Ok(()),
                    (0, 0) => (1.01, 0.98),
                    (0, 1) => (0.61, 0.58),
                    (0, 2) => (0.51, 0.48),
                    (0, _) => (0.91, 0.88),
                    (1, 0) => (0.61, 0.58),
                    (1, 1) => (0.41, 0.38),
                    (1, 2) => (0.31, 0.28),
                    (1, _) => (0.51, 0.48),
                    (2, 0) => (0.51, 0.48),
                    (2, 1 | 2) => (0.31, 0.28),
                    (2, _) => (0.41, 0.38),
                    (_, 0) => (0.91, 0.88),
                    (_, 1) => (0.51, 0.48),
                    (_, 2) => (0.41, 0.38),
                    _ => (0.81, 0.78),
                }
            }
        };
        actor.effects.colony_photosynthesis += if actor.effects.colony_photosynthesis == 0.0 {
            pair.0 * actor.effects.forest_photo
        } else {
            pair.1 * actor.effects.forest_photo
        };
        Ok(())
    }

    pub(super) fn shared_support_payment(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        cost: f64,
    ) -> Result<bool, crate::WorldError> {
        Ok({
        peer.use_energy(cost)?
            && actor.use_energy(cost)?
    
        })
    }

    pub(super) fn heal_contact_segments(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        let mut healed = false;
        for index in peer.body.indices() {
            if sick(ColorKind::from_argb(peer.body.color_argb(index)?))
                && self.shared_support_payment(
                    actor,
                    peer,
                    self.session.0.parameters.borrow().lifecycle.magenta_energy_consumption / 2.0,
                )?
            {
                let destination = peer.body.phenotype_colors.clone();
                let genome = primary_genome(&peer.genome)?.borrow();
                if genome.gene_count()? == 0 { return Err(crate::GeometryError::EmptyGenome.into()); }
                let value = genome.gene_for_segment(index)?.borrow().color_handle().cloned();
                crate::Body::store_color(destination, index, value)?;
                actor.set_display_color(ColorKind::Magenta.color_ref());
                healed = true;
            }
        }
        if healed {
            peer.effects.is_frozen = false;
            peer.effects.is_injured = false;
            peer.effects.all_frozen = false;
            peer.body.update_effects = 2;
            self.restore_contact_effects(peer)?;
        }
        Ok(())
    }

    pub(super) fn share_lavender_contact(&self, actor: &mut Organism, peer: &mut Organism) {
        let shield = self.session.0.parameters.borrow().effects.lavender_shield;
        if actor.effects.lavender >= shield && peer.effects.lavender < shield {
            peer.effects.lavender = peer
                .effects
                .lavender
                .wrapping_add(actor.effects.create_lavender);
            if peer.effects.lavender >= shield {
                peer.effects.lavender = shield;
            }
            peer.set_display_color(ColorKind::Cyan.color_ref());
            actor.set_display_color(ColorKind::Lavender.color_ref());
        }
        if actor.state.reproduction.countdown <= actor.state.reproduction.maximum_countdown
            && peer.state.reproduction.countdown > peer.state.reproduction.maximum_countdown
        {
            peer.state.reproduction.countdown = peer
                .state
                .reproduction
                .countdown
                .wrapping_sub(actor.effects.create_lavender / 20);
            if peer.state.reproduction.countdown < peer.state.reproduction.maximum_countdown {
                peer.state.reproduction.countdown = peer.state.reproduction.maximum_countdown;
            }
            peer.set_display_color(ColorKind::Cyan.color_ref());
            actor.set_display_color(ColorKind::Lavender.color_ref());
        }
    }

    pub(super) fn touch_friend_effects(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        if actor.effects.transfers_energy
            && actor.body.growth_ratio == 1
            && actor.state.energy > peer.state.energy + 2.0
        {
            let wakes_spore = supports(actor, peer)?;
            if wakes_spore || (primary_genome(&actor.genome)?.borrow().modifiers().peaceful
                && !peer.effects.altruist
                && peer.effects.transfers_energy
                && peer.state.active
                && !peer.effects.is_plant
                && !peer.effects.is_consumer
                && !peer.effects.is_fungus
                && (actor.effects.is_plant || actor.effects.is_consumer || actor.effects.is_fungus)) {
                let energy = between(
                    2.0 * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                    0.0,
                    actor.state.energy - (peer.state.energy + 2.0),
                );
                peer.set_display_color(Cyan.color_ref());
                actor.set_display_color(Rose.color_ref());
                peer.state.energy += energy;
                actor.state.energy -= energy;
                if wakes_spore
                    && !peer.state.active
                    && matches!(peer.effects.spore_version, 3 | 4)
                    && peer.state.energy > 0.5 * f64::from(peer.state.reproduction.energy)
                {
                    peer.state.reproduction.countdown = 0;
                }
            }
        }
        if actor.effects.use_friend_effects != 2 {
            return Ok(());
        }
        if peer.effects.is_injured
            && (actor.body.color_argb(segment)? == Magenta.argb().unwrap()
                || (actor.effects.transfers_energy && actor.effects.healing > 0))
        {
            let peer_kind = ColorKind::from_argb(peer.body.color_argb(peer_segment)?);
            if peer_kind != Brown {
                if peer_kind == Magenta || (supports(actor, peer)? && peer.state.active) {
                    self.heal_contact_segments(actor, peer)?;
                } else if supports(actor, peer)?
                    && !peer.state.active
                    && primary_genome(&peer.genome)?.borrow().modifiers().spore >= 7
                {
                    for color_index in peer.body.indices() {
                        peer.body.set_color(color_index, Blue.color_ref())?;
                    }
                    peer.effects.is_injured = false;
                    peer.effects.is_frozen = false;
                    peer.effects.all_frozen = false;
                }
            }
        }
        if actor.body.color_argb(segment)? == Mint.argb().unwrap()
            || (actor.effects.transfers_energy && actor.effects.antiviral > 0)
        {
            let kind = ColorKind::from_argb(peer.body.color_argb(peer_segment)?);
            if kind != Brown && (kind == Mint || supports(actor, peer)?) {
                let foreign = peer
                    .infected_genome
                    .as_ref()
                    .is_some_and(|code| !same_genome(Some(code), actor.genome.as_ref()));
                if peer.infected_genome.is_some()
                    && (kind == Mint || foreign)
                    && self.shared_support_payment(
                        actor,
                        peer,
                        self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 2.0,
                    )?
                {
                    peer.infected_genome = None;
                    peer.saved_genome = None;
                    peer.set_display_color(Cyan.color_ref());
                    actor.set_display_color(Mint.color_ref());
                }
                if peer.effects.reproduce_late < 0
                    && self.shared_support_payment(
                        actor,
                        peer,
                        self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 2.0,
                    )?
                {
                    peer.effects.reproduce_late = 0;
                    peer.set_display_color(Cyan.color_ref());
                    actor.set_display_color(Mint.color_ref());
                }
            }
        }
        if actor.body.color_argb(segment)? == Lavender.argb().unwrap()
            || (actor.effects.transfers_energy && actor.effects.create_lavender > 0)
        {
            let kind = ColorKind::from_argb(peer.body.color_argb(peer_segment)?);
            if kind != Brown && (kind == Lavender || supports(actor, peer)?) {
                self.share_lavender_contact(actor, peer);
            }
        }
        Ok(())
    }
}
