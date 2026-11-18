// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::organism::primary_genome;

use super::{Organism, World};
use crate::{ColorKind, geometry::GROWTH_SCALES};

/// Preserve source self calls without manufacturing two mutable references to one organism
pub(super) enum ContactActor<'a> {
    Recipient,
    Other(&'a mut Organism),
}

fn tint(recipient: &mut Organism, actor: &mut ContactActor<'_>, color: ColorKind) {
    let color = Some(color.color_ref().expect("Contact tint must name a reference color"));
    match actor {
        ContactActor::Recipient => recipient.set_display_color(color),
        ContactActor::Other(other) => other.set_display_color(color),
    }
}

fn original_gene(organism: &Organism, index: usize) -> Result<(f64, Option<u32>), crate::WorldError> {
    let genome = primary_genome(&organism.genome)?.borrow();
    let gene_ref = genome.gene_for_segment(index)?;
    let gene = gene_ref.borrow();
    Ok((gene.length(), gene.color()))
}

fn mint_overcomes_immunity(recipient: &Organism, actor: &ContactActor<'_>) -> bool {
    if recipient.effects.antiviral == 0 {
        return true;
    }
    let actor = match actor {
        ContactActor::Recipient => recipient,
        ContactActor::Other(other) => other,
    };
    actor.effects.is_plant
        || actor.effects.is_consumer
        || actor.effects.is_fungus
        || actor.effects.is_coral
}

impl World {
    fn contact_payment(
        &self,
        recipient: &mut Organism,
        actor: &mut ContactActor<'_>,
        cost: f64,
    ) -> Result<bool, crate::WorldError> {
        Ok({
        match actor {
            ContactActor::Recipient => {
                recipient.use_energy(cost)?
            }
            ContactActor::Other(other) => {
                other.use_energy(cost)?
            }
        }
    
        })
    }

    /// Preserve Organism.survive, including unconditional gray tint and exact MAGENTA equality
    pub(super) fn survive_from(&self, recipient: &mut Organism, mut actor: ContactActor<'_>) -> Result<(), crate::WorldError> {
        recipient.effects.healing = 0;
        for index in recipient.body.indices() {
            if recipient.body.color_argb(index)? == ColorKind::Magenta.argb().unwrap() {
                recipient.body.set_color(index, ColorKind::DarkFire.color_ref())?;
                recipient.effects.use_extra_effects = true;
                recipient.effects.is_injured = true;
            }
        }
        tint(recipient, &mut actor, ColorKind::Gray);
        Ok(())
    }

    /// Pay in source segment order, then restore caches even when no segment changed
    pub(super) fn inhibited_from(&self, recipient: &mut Organism, mut actor: ContactActor<'_>) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let cost = self.session.0.parameters.borrow().lifecycle.fallow_energy_consumption;
        for index in recipient.body.indices() {
            let kind = ColorKind::from_argb(recipient.body.color_argb(index)?);
            let update = match kind {
                Spore if recipient.effects.spore_version > 0 => true,
                Auburn | Indigo => true,
                Yellow | Blond => false,
                _ => continue,
            };
            if self.contact_payment(recipient, &mut actor, cost)? {
                recipient.body.set_color(index, if recipient.effects.is_plant {
                    Broken
                } else {
                    LightBrown
                }
                .color_ref())?;
                tint(recipient, &mut actor, Fallow);
                if update {
                    recipient.body.update_effects = 2;
                }
            }
        }
        self.restore_contact_effects(recipient)?;
        Ok(())
    }

    pub(super) fn frozen_from(&self, recipient: &mut Organism, actor: ContactActor<'_>) -> Result<(), crate::WorldError> {
        self.freeze_contact(recipient, actor, true)?;
        Ok(())
    }

    pub(super) fn paralyzed_from(&self, recipient: &mut Organism, actor: ContactActor<'_>) -> Result<(), crate::WorldError> {
        self.freeze_contact(recipient, actor, false)?;
        Ok(())
    }

    fn freeze_contact(
        &self,
        recipient: &mut Organism,
        mut actor: ContactActor<'_>,
        plants: bool,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let cost = self.session.0.parameters.borrow().lifecycle.sky_energy_consumption;
        let mut changed = false;
        if plants {
            recipient.effects.all_frozen = true;
            if recipient.effects.sky_version > 0 {
                recipient.effects.sky_version = -1;
            }
            if recipient.effects.jade_factor > 0 {
                recipient.effects.jade_factor = 1;
            }
            recipient.effects.spin = 0;
        }
        recipient.effects.can_move = false;
        recipient.effects.can_react = false;
        recipient.effects.use_frame_movement = false;
        for index in recipient.body.indices() {
            let kind = ColorKind::from_argb(recipient.body.color_argb(index)?);
            match kind {
                Sky | DeepSky if plants => {
                    if self.contact_payment(recipient, &mut actor, cost)? {
                        recipient.body.set_color(index, LightBlue.color_ref())?;
                        changed = true;
                        recipient.effects.is_injured = true;
                        if !recipient.effects.is_plant && !primary_genome(&recipient.genome)?.borrow().modifiers().modifies_sky {
                            recipient.body.update_effects = 2;
                        }
                    } else {
                        if recipient.effects.sky_version <= 0 {
                            recipient.effects.sky_version =
                                if primary_genome(&recipient.genome)?.borrow().modifiers().modifies_sky { 2 } else { 1 };
                        }
                        recipient.effects.all_frozen = false;
                    }
                }
                Teal | Cyan => {
                    if self.contact_payment(recipient, &mut actor, cost)? {
                        recipient.body.set_color(index, Ice.color_ref())?;
                        if kind == Teal && primary_genome(&recipient.genome)?.borrow().modifiers().passive {
                            recipient.body.update_effects = 2;
                        }
                        changed = true;
                        recipient.effects.is_injured = true;
                        recipient.effects.is_frozen = true;
                        recipient.body.set_photo(index, 0.0)?;
                    } else {
                        if plants {
                            recipient.effects.all_frozen = false;
                        }
                        recipient.effects.can_move = true;
                        if kind == Teal {
                            recipient.effects.can_react = true;
                            if !primary_genome(&recipient.genome)?.borrow().modifiers().passive {
                                recipient.effects.use_frame_movement = true;
                            }
                        } else {
                            recipient.effects.use_frame_movement = true;
                        }
                    }
                }
                Winter | GreenBrown if plants => {
                    if self.contact_payment(recipient, &mut actor, cost)? {
                        recipient.body.set_color(index, Ice.color_ref())?;
                        changed = true;
                        recipient.effects.is_injured = true;
                        recipient.effects.is_frozen = true;
                    } else {
                        recipient.effects.all_frozen = false;
                    }
                }
                Jade | DarkJade | Spring | DarkGreen if plants => {
                    if self.contact_payment(recipient, &mut actor, cost)? {
                        recipient.body.set_color(index, Ice.color_ref())?;
                        changed = true;
                        recipient.effects.is_injured = true;
                        recipient.effects.is_frozen = true;
                        recipient.body.update_effects = 2;
                    } else {
                        recipient.effects.all_frozen = false;
                        if matches!(kind, Jade | DarkJade) {
                            recipient.effects.jade_factor = 2;
                        }
                        if kind == Spring {
                            let (length, _) = original_gene(recipient, index)?;
                            recipient.effects.spin =
                                recipient.effects.spin.wrapping_add((10.0 * length) as i32);
                        }
                        if kind == DarkGreen {
                            let (length, original) = original_gene(recipient, index)?;
                            let original =
                                original.ok_or(crate::GeometryError::MissingColor { gene: index % primary_genome(&recipient.genome)?.borrow().gene_count()? })?;
                            if original == Jade.argb().unwrap() {
                                recipient.effects.jade_factor = 2;
                            }
                            if original == Spring.argb().unwrap() {
                                recipient.effects.spin =
                                    recipient.effects.spin.wrapping_add((10.0 * length) as i32);
                            }
                        }
                    }
                }
                Summer | Leaf | Lime | Green | Forest | Grass | C4 | Purple if plants => {
                    if self.contact_payment(recipient, &mut actor, cost)? {
                        recipient.body.set_color(index, Ice.color_ref())?;
                        changed = true;
                        recipient.effects.is_injured = true;
                        recipient.effects.is_frozen = true;
                        recipient.body.update_effects = 2;
                    } else {
                        recipient.effects.all_frozen = false;
                    }
                }
                OldBark | Bark if plants => {
                    if self.contact_payment(recipient, &mut actor, cost)? {
                        recipient.body.set_color(index, DeadBark.color_ref())?;
                        changed = true;
                        recipient.effects.is_frozen = true;
                        recipient.body.update_effects = 2;
                    } else {
                        recipient.effects.all_frozen = false;
                    }
                }
                _ => {}
            }
        }
        if changed {
            tint(recipient, &mut actor, Sky);
        }
        if !recipient.effects.can_react && !recipient.effects.is_only_c4 {
            recipient.effects.can_dodge = false;
            recipient.effects.dodge = false;
        }
        if plants && recipient.effects.jade_factor > 1 {
            recipient.effects.jade_factor = self.session.0.parameters.borrow().effects
                .dark_jade_delay
                .wrapping_mul(if primary_genome(&recipient.genome)?.borrow().symmetry() == 1 { 2 } else { primary_genome(&recipient.genome)?.borrow().symmetry() });
        }
        Ok(())
    }

    /// Preserve mint eligibility and defer the recipient injury flag until all segments finish
    pub(super) fn neutralized_from(
        &self,
        recipient: &mut Organism,
        mut actor: ContactActor<'_>,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let cost = self.session.0.parameters.borrow().lifecycle.mint_energy_consumption / 10.0;
        let mut changed = false;
        for index in recipient.body.indices() {
            let kind = ColorKind::from_argb(recipient.body.color_argb(index)?);
            match kind {
                White | Plague => {
                    let plant = recipient.effects.is_plant;
                    let consumer = recipient.effects.is_consumer || recipient.effects.is_coral;
                    if (plant || consumer || mint_overcomes_immunity(recipient, &actor))
                        && self.contact_payment(recipient, &mut actor, cost)?
                    {
                        let color = if plant {
                            Broken
                        } else if consumer {
                            LightBrown
                        } else if recipient.effects.healing > 0
                            && recipient.effects.plague_version == 0
                        {
                            recipient.effects.use_extra_effects = true;
                            DarkFire
                        } else {
                            LightBrown
                        };
                        recipient.body.set_color(index, color.color_ref())?;
                        changed = true;
                    }
                }
                Coral => {
                    if self.contact_payment(recipient, &mut actor, cost)? {
                        { let destination = recipient.body.phenotype_colors.clone(); let value = if recipient.effects.is_plant {
                            Broken
                        } else {
                            if recipient.effects.gold > 0
                                && !recipient.effects.is_consumer
                                && !recipient.effects.is_fungus
                            {
                                recipient.body.update_effects = 2;
                            }
                            LightBrown
                        }
                        .color_ref(); crate::Body::store_color(destination, index, value)?; }
                        changed = true;
                    }
                }
                Fallow => {
                    let plant = recipient.effects.is_plant;
                    let consumer = recipient.effects.is_consumer || recipient.effects.is_coral;
                    if (plant || consumer || mint_overcomes_immunity(recipient, &actor))
                        && self.contact_payment(recipient, &mut actor, cost)?
                    {
                        recipient.body.set_color(index, if plant { Broken } else { LightBrown }.color_ref())?;
                        changed = true;
                        recipient.body.update_effects = 2;
                    }
                }
                _ => {}
            }
        }
        if changed {
            tint(recipient, &mut actor, Mint);
            recipient.effects.is_injured = true;
        }
        Ok(())
    }

    /// Restore source caches in reverse segment order without recreating initial organism effects
    pub(super) fn restore_contact_effects(&self, organism: &mut Organism) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let primary = &organism.genome;
        let code = || primary_genome(primary);
        let params = &self.session.0.parameters.borrow().effects;
        let body = &mut organism.body;
        let effects = &mut organism.effects;
        let reproduction = &mut organism.state.reproduction;
        let segments = body.segment_count;
        let base_energy = 40_i32.wrapping_add(3_i32.wrapping_mul(segments));
        effects.spin = 0;
        if effects.jade_factor > 0 {
            effects.jade_factor = 1;
        }
        effects.healing = 0;
        effects.antiviral = 0;
        effects.create_lavender = 0;
        effects.yellow_count = 0;
        let mut fertility = 0.0;
        effects.indigo = 0;
        reproduction.early_energy = 0;
        if effects.reproduce_late > 0 {
            effects.reproduce_late = 0;
            reproduction.energy = base_energy;
        }
        if effects.gold > 0 {
            effects.gold = 0;
            reproduction.maximum_age = params.max_age.wrapping_add(segments / params.age_divisor);
        }
        effects.is_reproductive = false;
        if effects.sky_version > 0 {
            effects.sky_version = -1;
        }
        if effects.is_auburn && (effects.is_consumer || effects.is_plant || effects.is_fungus) {
            effects.is_auburn = false;
        }
        for index in body.reverse_indices() {
            let length = || -> Result<f64, crate::GeometryError> {
                let genome = code()?.borrow();
                let value = genome.gene_for_segment(index)?.borrow().length();
                Ok(value)
            };
            let length_tenths = || -> Result<i32, crate::GeometryError> {
                Ok((10.0 * length()?) as i32)
            };
            let immune_length = || -> Result<i32, crate::GeometryError> {
                let genome = code()?.borrow();
                let factor = if genome.symmetry() != 3 { 10.0 } else if genome.mirror() == 0 { 8.0 } else { 6.0 };
                Ok((factor * length()?) as i32)
            };
            match body.color_kind(index)? {
                Jade | DarkJade => effects.jade_factor = 2,
                Spring => effects.spin = effects.spin.wrapping_add(length_tenths()?),
                DarkGreen => {
                    let genome = code()?.borrow();
                    let color = genome.gene_for_segment(index)?.borrow()
                        .color()
                        .ok_or(crate::GeometryError::MissingColor { gene: index % genome.gene_count()? })?;
                    if color == Jade.argb().unwrap() {
                        effects.jade_factor = 2;
                    }
                    if color == Spring.argb().unwrap() {
                        effects.spin = effects.spin.wrapping_add(length_tenths()?);
                    }
                }
                Teal => {
                    effects.can_move = true;
                    effects.can_react = true;
                    if !code()?.borrow().modifiers().passive {
                        effects.use_frame_movement = true;
                        body.set_photo(index, -4.0)?;
                    }
                }
                Cyan => {
                    effects.can_move = true;
                    effects.use_frame_movement = true;
                    body.set_photo(index, -3.0)?;
                }
                Spore => {
                    if code()?.borrow().modifiers().spore <= 6 {
                        effects.is_reproductive = true;
                        if code()?.borrow().modifiers().spore <= 2 {
                            reproduction.early_energy = reproduction
                                .early_energy
                                .wrapping_add(20_i32.wrapping_mul(code()?.borrow().gene_count()? as i32));
                        }
                    }
                }
                Blond => {
                    reproduction.early_energy = reproduction
                        .early_energy
                        .wrapping_add(30_i32.wrapping_add(length_tenths()?));
                }
                Flower => {
                    if effects.reproduce_late < 0 {
                        effects.reproduce_late = 0;
                    }
                    effects.reproduce_late = effects.reproduce_late.wrapping_add(length_tenths()?);
                }
                Gold => effects.gold = effects.gold.wrapping_add(length_tenths()?),
                Auburn => {
                    effects.is_auburn = true;
                    effects.is_reproductive = true;
                }
                Yellow => {
                    effects.is_reproductive = true;
                    effects.yellow_count = effects.yellow_count.wrapping_add(1);
                    fertility += if code()?.borrow().symmetry() != 3 { length()? } else { 0.0 * length()? };
                }
                Indigo => {
                    effects.is_reproductive = true;
                    effects.indigo = effects.indigo.wrapping_add(immune_length()?);
                }
                Lavender => {
                    effects.create_lavender = effects.create_lavender.wrapping_add(length_tenths()?);
                }
                Mint => effects.antiviral = effects.antiviral.wrapping_add(immune_length()?),
                Magenta => effects.healing = effects.healing.wrapping_add(immune_length()?),
                Sky | DeepSky => {
                    if effects.sky_version <= 0 {
                        effects.sky_version = if code()?.borrow().modifiers().modifies_sky { 2 } else { 1 };
                    }
                }
                LightBrown | GreenBrown | DarkFire | LightBlue | Broken => {
                    effects.is_injured = true;
                }
                Ice => {
                    effects.is_frozen = true;
                    effects.is_injured = true;
                }
                DeadBark => effects.is_frozen = true,
                _ => {}
            }
        }
        if effects.can_react
            && !effects.is_consumer
            && !effects.is_fungus
            && !effects.is_killer
            && effects.sky_version == 0
            && ((!effects.is_infectious && !effects.is_coral && effects.plague_version == 0)
                || (!effects.is_plant && effects.methanotrophy == 0.0 && code()?.borrow().modifiers().passive))
        {
            effects.can_dodge = true;
        }
        if effects.jade_factor > 1 {
            effects.jade_factor =
                params
                    .dark_jade_delay
                    .wrapping_mul(if code()?.borrow().symmetry() == 1 { 2 } else { code()?.borrow().symmetry() });
        }
        if effects.create_lavender > 0 {
            effects.create_lavender /= code()?.borrow().symmetry();
        }
        if effects.yellow_count > 0 {
            effects.yellow_count =
                (effects.yellow_count / code()?.borrow().symmetry()).wrapping_add((fertility / 23.0) as i32);
        }
        if effects.indigo > 0 {
            effects.indigo = (f64::from(effects.indigo) / params.indigo_divisor) as i32;
        }
        if reproduction.early_energy > 0 {
            reproduction.early_energy =
                crate::java_math::round_integer_tenth(reproduction.early_energy);
            effects.is_reproductive = true;
            let minimum = if effects.is_plant {
                40
            } else if effects.is_consumer
                || effects.is_fungus
                || (effects.transfers_energy && effects.is_infectious && effects.is_blond)
                || effects.spore_version < 0
                || effects.spore_version == 6
            {
                21
            } else {
                14
            };
            reproduction.early_energy = reproduction
                .energy
                .wrapping_sub(reproduction.early_energy)
                .max(minimum);
        }
        if effects.reproduce_late > 0 {
            effects.reproduce_late = crate::java_math::round_integer_tenth(effects.reproduce_late);
            if effects.is_infectious {
                reproduction.energy = base_energy.wrapping_add(
                    (f64::from(effects.reproduce_late)
                        * GROWTH_SCALES[crate::geometry::growth_index(body.growth_ratio)?]) as i32,
                );
            } else if effects
                .reproduce_late
                .wrapping_sub(3_i32.wrapping_mul(code()?.borrow().symmetry()))
                > 0
            {
                reproduction.energy = base_energy
                    .wrapping_add(effects.reproduce_late)
                    .wrapping_sub(3_i32.wrapping_mul(code()?.borrow().symmetry()));
            }
        }
        if effects.gold > 0 {
            reproduction.maximum_age = params
                .max_age
                .wrapping_add(segments / params.age_divisor)
                .wrapping_add(crate::geometry::java_round(
                    f64::from(effects.gold / 10) / params.gold_divisor,
                ));
            effects.gold = 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gene, Genome, GenomeProperties, WorldConfig};
    use std::{cell::RefCell, rc::Rc};

    fn empty_world() -> World {
        World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap()
    }

    fn placed(world: &mut World, colors: &[u32], symmetry: i32, center: [i32; 2]) -> Organism {
        let genes: Vec<_> = colors
            .iter()
            .map(|color| Gene::new(12.0, 0.0, Some(*color), 0, [0; 36]).shared())
            .collect();
        let code = Genome::from_parts(
            &genes,
            GenomeProperties {
                symmetry,
                ..GenomeProperties::default()
            },
        );
        world
            .construct_pasted_shared(Rc::new(RefCell::new(code)), center)
            .unwrap()
            .unwrap()
    }

    fn colors(kinds: &[ColorKind]) -> Vec<u32> {
        kinds.iter().map(|kind| kind.argb().unwrap()).collect()
    }

    #[test]
    fn survival_uses_full_argb_and_preserves_self_tint() {
        let mut world = empty_world();
        let mut target = placed(
            &mut world,
            &[ColorKind::Magenta.argb().unwrap(), 0x12ff00ff],
            1,
            [100, 100],
        );
        target.effects.healing = 999;
        world.survive_from(&mut target, ContactActor::Recipient).unwrap();
        assert_eq!(
            target.body.color_argb(0).unwrap(),
            ColorKind::DarkFire.argb().unwrap()
        );
        assert_eq!(target.body.color_argb(1).unwrap(), 0x12ff00ff);
        assert_eq!(target.effects.healing, 0);
        assert!(target.effects.use_extra_effects && target.effects.is_injured);
        assert_eq!(target.state.temporary_argb(), ColorKind::Gray.argb());
        assert_eq!(target.state.temporary_color_frames, 10);
    }

    #[test]
    fn inhibition_keeps_partial_payments_and_always_restores_reproduction_caches() {
        let mut world = empty_world();
        world.session.0.parameters.borrow_mut().lifecycle.fallow_energy_consumption = 0.25;
        let mut target = placed(
            &mut world,
            &colors(&[ColorKind::Spore, ColorKind::Auburn, ColorKind::Yellow]),
            1,
            [100, 100],
        );
        let mut actor = placed(&mut world, &colors(&[ColorKind::Fallow]), 1, [200, 200]);
        world.oxygen.set(0.375);
        actor.state.energy = 1.0;
        target.effects.is_plant = true;
        target.effects.spore_version = 1;
        target.state.reproduction.early_energy = 999;
        world.inhibited_from(&mut target, ContactActor::Other(&mut actor)).unwrap();
        assert_eq!(
            target.body.color_argb(0).unwrap(),
            ColorKind::Broken.argb().unwrap()
        );
        assert_eq!(
            target.body.color_argb(1).unwrap(),
            ColorKind::Auburn.argb().unwrap()
        );
        assert_eq!(
            target.body.color_argb(2).unwrap(),
            ColorKind::Yellow.argb().unwrap()
        );
        assert_eq!(actor.state.energy.to_bits(), 0.625_f64.to_bits());
        assert_eq!(world.oxygen.get().to_bits(), 0.0_f64.to_bits());
        assert_eq!(target.state.reproduction.early_energy, 0);
        assert_eq!(target.effects.yellow_count, 1);
        assert!(target.effects.is_auburn && target.effects.is_injured);
        assert_eq!(target.body.update_effects, 2);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Fallow.argb());
    }

    #[test]
    fn freezing_preserves_ordered_partial_costs_and_paralysis_retains_unrelated_state() {
        let mut world = empty_world();
        world.session.0.parameters.borrow_mut().lifecycle.sky_energy_consumption = 0.25;
        let mut target = placed(
            &mut world,
            &colors(&[ColorKind::Teal, ColorKind::Cyan]),
            1,
            [100, 100],
        );
        let mut actor = placed(&mut world, &colors(&[ColorKind::Sky]), 1, [200, 200]);
        world.oxygen.set(0.375);
        actor.state.energy = 1.0;
        target.effects.can_dodge = true;
        target.effects.dodge = true;
        target.body.set_photo(1, -3.0).unwrap();
        world.frozen_from(&mut target, ContactActor::Other(&mut actor)).unwrap();
        assert_eq!(target.body.color_argb(0).unwrap(), ColorKind::Ice.argb().unwrap());
        assert_eq!(
            target.body.color_argb(1).unwrap(),
            ColorKind::Cyan.argb().unwrap()
        );
        assert_eq!(actor.state.energy.to_bits(), 0.625_f64.to_bits());
        assert!(!target.effects.all_frozen && target.effects.is_frozen);
        assert!(target.effects.can_move && target.effects.use_frame_movement);
        assert!(!target.effects.can_react && !target.effects.can_dodge && !target.effects.dodge);
        assert_eq!(target.body.photo_values().unwrap(), [0.0, -3.0]);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Sky.argb());

        let mut green = placed(&mut world, &colors(&[ColorKind::Green]), 1, [300, 300]);
        green.effects.all_frozen = false;
        green.effects.sky_version = 2;
        green.effects.jade_factor = 99;
        green.effects.spin = 7;
        world.paralyzed_from(&mut green, ContactActor::Recipient).unwrap();
        assert_eq!(
            green.body.color_argb(0).unwrap(),
            ColorKind::Green.argb().unwrap()
        );
        assert!(!green.effects.all_frozen);
        assert_eq!(green.effects.sky_version, 2);
        assert_eq!(green.effects.jade_factor, 99);
        assert_eq!(green.effects.spin, 7);
        assert_eq!(green.state.temporary_argb(), None);
    }

    #[test]
    fn neutralization_preserves_immunity_and_live_genotype_caches() {
        let mut world = empty_world();
        world.session.0.parameters.borrow_mut().lifecycle.mint_energy_consumption = 1.25;
        let mut target = placed(
            &mut world,
            &colors(&[ColorKind::White, ColorKind::Fallow, ColorKind::Coral]),
            1,
            [100, 100],
        );
        let mut actor = placed(&mut world, &colors(&[ColorKind::Mint]), 1, [200, 200]);
        target.effects.is_plant = false;
        target.effects.is_consumer = false;
        target.effects.is_coral = false;
        target.effects.antiviral = 1;
        target.effects.healing = 10;
        target.effects.plague_version = 0;
        target.effects.gold = 1;
        target.effects.is_infectious = true;
        target.infected_genome = Some(Rc::clone(target.genome.as_ref().unwrap()));
        target.saved_genome = Some(Rc::clone(target.genome.as_ref().unwrap()));
        world.neutralized_from(&mut target, ContactActor::Other(&mut actor)).unwrap();
        assert_eq!(
            target.body.color_argb(0).unwrap(),
            ColorKind::White.argb().unwrap()
        );
        assert_eq!(
            target.body.color_argb(1).unwrap(),
            ColorKind::Fallow.argb().unwrap()
        );
        assert_eq!(
            target.body.color_argb(2).unwrap(),
            ColorKind::LightBrown.argb().unwrap()
        );
        assert_eq!(target.body.update_effects, 2);
        actor.effects.is_plant = true;
        world.neutralized_from(&mut target, ContactActor::Other(&mut actor)).unwrap();
        assert_eq!(
            target.body.color_argb(0).unwrap(),
            ColorKind::DarkFire.argb().unwrap()
        );
        assert_eq!(
            target.body.color_argb(1).unwrap(),
            ColorKind::LightBrown.argb().unwrap()
        );
        assert!(target.effects.use_extra_effects && target.effects.is_injured);
        assert!(target.effects.is_infectious);
        assert!(Rc::ptr_eq(
            target.infected_genome.as_ref().unwrap(), target.genome.as_ref().unwrap()));
        assert!(Rc::ptr_eq(
            target.saved_genome.as_ref().unwrap(), target.genome.as_ref().unwrap()));
        assert_eq!(actor.state.temporary_argb(), ColorKind::Mint.argb());
    }

    #[test]
    fn restoration_preserves_integer_fertility_and_implicit_float_rounding() {
        let mut world = empty_world();
        let mut yellow = placed(&mut world, &colors(&[ColorKind::Yellow]), 3, [100, 100]);
        world.restore_contact_effects(&mut yellow).unwrap();
        assert_eq!(yellow.effects.yellow_count, 1);

        let mut blond = placed(&mut world, &colors(&[ColorKind::Blond]), 1, [200, 200]);
        blond.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow_mut().set_length(214748360.0);
        blond.state.reproduction.energy = i32::MAX;
        world.restore_contact_effects(&mut blond).unwrap();
        assert_eq!(blond.state.reproduction.early_energy, 1_932_735_279);
    }
}
