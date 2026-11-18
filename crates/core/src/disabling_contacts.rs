// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Complete VIOLET, SKY, OLIVE, and GRAY branches from Organism.java:15862-17569
//! Each entry includes the shared peer death guard at Organism.java:17917-17919

use crate::organism::primary_genome;

use super::contact_state::ContactActor;
use super::death::Killer;
use super::draining_contacts::age_bark;
use super::{Organism, World};
use crate::{ColorKind, GROWTH_SCALES, TOLERANCE, physics::between};

fn color(actor: &Organism, segment: usize) -> Result<ColorKind, crate::WorldError> {
    Ok(actor.body.color_kind(segment)?)
}

fn tint(actor: &mut Organism, color: ColorKind) {
    actor.set_display_color(Some(color.color_ref().expect("Tint requires a named source literal")));
}

fn classified(actor: &Organism) -> bool {
    actor.effects.is_consumer || actor.effects.is_plant || actor.effects.is_fungus
}

fn plant_color(color: ColorKind) -> bool {
    use ColorKind::*;
    matches!(
        color,
        Green
            | Forest
            | Spring
            | Summer
            | Winter
            | Lime
            | Leaf
            | C4
            | Grass
            | Jade
            | DarkJade
            | DarkGreen
            | Purple
    )
}

fn poison_color(peer: &mut Organism, segment: usize) -> Result<(), crate::WorldError> {
    peer.body.set_color(segment, if peer.effects.is_plant {
        ColorKind::GreenBrown
    } else {
        ColorKind::LightBrown
    }
    .color_ref())?;
    Ok(())
    }

fn healing_survival(peer: &Organism) -> bool {
    peer.effects.healing > 0
        && (!peer.effects.is_gray || (!peer.effects.is_consumer && !peer.effects.is_fungus))
}

fn can_paralyze(actor: &Organism, peer: &Organism) -> bool {
    peer.effects.can_move
        && (peer.effects.sky_version != 1 || classified(actor) || classified(peer))
}

impl World {
    fn disabling_peer_death(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        if peer.state.alive && peer.state.energy < TOLERANCE {
            self.die(peer, Some(Killer::Other(actor)))?;
        }
        Ok(())
    }

    fn disabling_dodge(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        weapon: ColorKind,
        require_clear_tint: bool,
    ) -> Result<bool, crate::WorldError> {
        Ok({
        if peer.effects.dodge
            && (!require_clear_tint || peer.state.temporary_color_frames == 0)
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            tint(peer, ColorKind::Teal);
            tint(actor, weapon);
            true
        } else {
            false
        }
    
        })
    }

    fn blue_disabling_shield(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
        weapon: ColorKind,
    ) -> Result<bool, crate::WorldError> {
        if !peer.use_energy(
            self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
            return Ok(false);
        }
        if peer.effects.is_enhanced {
            actor.use_energy(
                between(
                    (0.5 * peer.body.physical_length(peer_segment)?.sqrt())
                        * self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                    0.0,
                    actor.state.energy,
                ))?;
            tint(actor, ColorKind::DarkLilac);
            if actor.state.energy < TOLERANCE {
                self.die(actor, Some(Killer::Other(&mut *peer)))?;
            }
        } else {
            tint(actor, weapon);
        }
        tint(peer, ColorKind::Blue);
        Ok(true)
    }

    fn olive_disabling_shield(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
        weapon: ColorKind,
        cost: f64,
        dark: bool,
    ) -> Result<bool, crate::WorldError> {
        use ColorKind::*;
        if !peer.use_energy(
            self.session.0.parameters.borrow().lifecycle.olive_energy_consumption)? {
            return Ok(false);
        }
        actor.body.set_color(segment, if actor.effects.is_plant {
            Broken
        } else {
            LightBrown
        }
        .color_ref())?;
        actor.effects.is_injured = true;
        if peer.effects.healing == 0
            && actor.use_energy(cost)?
        {
            peer.body.set_color(peer_segment, if dark { LightBlue } else { DarkOlive }.color_ref())?;
            if !dark {
                peer.effects.use_extra_effects = true;
            }
            tint(peer, DarkOlive);
            tint(actor, weapon);
            if dark {
                if !peer.effects.is_plant {
                    peer.body.update_effects = 2;
                }
                peer.effects.is_injured = true;
            }
        } else {
            tint(peer, Olive);
        }
        Ok(true)
    }

    fn violet_payment(&self, actor: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        actor.use_energy(
            self.session.0.parameters.borrow().lifecycle.violet_energy_consumption)?
    
        })
    }

    fn violet_poison(&self, actor: &mut Organism, peer: &mut Organism, segment: usize) -> Result<bool, crate::WorldError> {
        Ok({
        if self.violet_payment(actor)? {
            poison_color(peer, segment)?;
            tint(actor, ColorKind::Violet);
            true
        } else {
            false
        }
            })
    }

    fn check_disabling_plant(&self, peer: &mut Organism) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let primary = &peer.genome;
        let genome = || primary_genome(primary);
        peer.effects.spin = 0;
        if peer.effects.jade_factor > 0 {
            peer.effects.jade_factor = 1;
        }
        for index in peer.body.reverse_indices() {
            match color(peer, index)? {
                Jade | DarkJade => peer.effects.jade_factor = 2,
                Spring => {
                    let code = genome()?.borrow();
                    let gene_ref = code.gene_for_segment(index)?;
    let gene = gene_ref.borrow();
                    peer.effects.spin = peer
                        .effects
                        .spin
                        .wrapping_add((10.0 * gene.length()) as i32)
                }
                DarkGreen => {
                    let code = genome()?.borrow();
                    let gene_ref = code.gene_for_segment(index)?;
    let gene = gene_ref.borrow();
                    let original = gene
                        .color()
                        .ok_or(crate::GeometryError::MissingColor { gene: index % code.gene_count()? })?;
                    if original == Jade.argb().unwrap() {
                        peer.effects.jade_factor = 2;
                    }
                    if original == Spring.argb().unwrap() {
                        peer.effects.spin = peer
                            .effects
                            .spin
                            .wrapping_add((10.0 * gene.length()) as i32);
                    }
                }
                _ => {}
            }
        }
        if peer.effects.jade_factor > 1 {
            peer.effects.jade_factor =
                self.session.0.parameters.borrow()
                    .effects
                    .dark_jade_delay
                    .wrapping_mul(if genome()?.borrow().symmetry() == 1 {
                        2
                    } else {
                        genome()?.borrow().symmetry()
                    });
        }
        Ok(())
    }

    fn violet_plant(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        update: bool,
        check: bool,
    ) -> Result<(), crate::WorldError> {
        if !classified(actor) {
            return Ok(());
        }
        if peer.state.active {
            if !self.disabling_dodge(actor, peer, ColorKind::Violet, false)?
                && self.violet_payment(actor)?
            {
                peer.body.set_color(segment, ColorKind::GreenBrown.color_ref())?;
                if update {
                    peer.body.update_effects = 2;
                }
                tint(actor, ColorKind::Violet);
                peer.effects.is_injured = true;
                if check {
                    self.check_disabling_plant(peer)?;
                }
            }
        } else if self.violet_payment(actor)? {
            for color_index in peer.body.indices() {
                if plant_color(ColorKind::from_argb(peer.body.color_argb(color_index)?)) {
                    peer.body.set_color(color_index, ColorKind::GreenBrown.color_ref())?;
                }
            }
            tint(actor, ColorKind::Violet);
            peer.state.temporary_color_frames = 0;
            peer.effects.is_injured = true;
            peer.effects.is_frozen = true;
            peer.effects.all_frozen = true;
        }
        Ok(())
    }

    fn violet_healing_shield(&self, actor: &mut Organism, peer: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        if peer.effects.healing > 0
            && peer.state.temporary_color_frames == 0
            && (actor.effects.is_consumer || actor.effects.is_fungus)
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.magenta_energy_consumption)?
        {
            tint(peer, ColorKind::Magenta);
            tint(actor, ColorKind::Violet);
            true
        } else {
            false
        }
    
        })
    }

    /// Poison the current phenotype while preserving color-specific cache restoration and retaliation
    pub(super) fn touch_violet(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        if !peer.effects.has_dodged
            && (!peer.effects.is_only_c4 || peer.state.temporary_color_frames > 8)
        {
            peer.effects.has_dodged = true;
        }
        match color(peer, peer_segment)? {
            LightBrown | GreenBrown | OldBark => {}
            DarkFire => {
                if peer.body.photo(peer_segment)? != -1.0
                    && peer.state.active
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    peer.effects.is_injured = true;
                }
            }
            Violet | White => {
                if ((!actor.effects.is_consumer && !actor.effects.is_fungus)
                    && (actor.effects.is_plant || actor.effects.is_coral || peer.effects.is_coral))
                    || classified(peer)
                    || peer.effects.plague_version > 0
                    || peer.effects.is_auburn
                {
                    if self.violet_poison(actor, peer, peer_segment)? {
                        peer.effects.is_injured = true;
                    }
                }
            }
            Green | Grass | Forest | Summer | Lime | Leaf | C4 | Purple => {
                self.violet_plant(actor, peer, peer_segment, true, false)?
            }
            Spring | DarkGreen => self.violet_plant(actor, peer, peer_segment, true, true)?,
            Winter => self.violet_plant(actor, peer, peer_segment, false, false)?,
            Jade | DarkJade => {
                if classified(actor) && !self.disabling_dodge(actor, peer, Violet, false)? {
                    peer.body.set_color(peer_segment, if color(peer, peer_segment)? == Jade {
                        DarkJade
                    } else {
                        DarkGreen
                    }
                    .color_ref())?;
                    tint(actor, Violet);
                    peer.effects.use_extra_effects = true;
                }
            }
            Cyan | Teal => {
                let teal = color(peer, peer_segment)? == Teal;
                if !self.disabling_dodge(actor, peer, Violet, false)? && self.violet_payment(actor)? {
                    poison_color(peer, peer_segment)?;
                    peer.body.set_photo(peer_segment, 0.0)?;
                    tint(actor, Violet);
                    if teal && primary_genome(&peer.genome)?.borrow().modifiers().passive {
                        peer.body.update_effects = 2;
                    }
                    peer.effects.is_injured = true;
                }
            }
            Auburn | Indigo | Flower | Gold | Spore | Yellow | Blond => {
                let update = !matches!(color(peer, peer_segment)?, Yellow | Blond);
                if !self.disabling_dodge(actor, peer, Violet, false)?
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    self.restore_contact_effects(peer)?;
                    if update {
                        peer.body.update_effects = 2;
                    }
                }
            }
            DarkGray => {
                if !self.disabling_dodge(actor, peer, Violet, false)?
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    peer.effects.is_injured = true;
                }
            }
            Blue => {
                if !self.blue_disabling_shield(actor, peer, peer_segment, Violet)?
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    if !peer.effects.is_plant {
                        peer.body.update_effects = 2;
                    }
                    peer.effects.is_injured = true;
                }
            }
            Sky => {
                if peer.effects.sky_version == 2 || peer.effects.is_plant {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                        peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                        peer.effects.use_extra_effects = true;
                        tint(peer, DeepSky);
                        tint(actor, Violet);
                    } else if self.violet_poison(actor, peer, peer_segment)? {
                        self.restore_contact_effects(peer)?;
                        if !peer.effects.is_plant && !primary_genome(&peer.genome)?.borrow().modifiers().modifies_sky
                        {
                            peer.body.update_effects = 2;
                        }
                    }
                }
            }
            DeepSky => {
                if peer.effects.sky_version == 2 || peer.effects.is_plant {
                    tint(peer, DeepSky);
                    tint(actor, Violet);
                }
            }
            Rose => {
                if !actor.effects.altruist
                    && !self.disabling_dodge(actor, peer, Violet, false)?
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    peer.body.update_effects = 2;
                    peer.effects.is_injured = true;
                }
            }
            Magenta | Mint | Lavender => {
                let target = color(peer, peer_segment)?;
                if !actor.effects.altruist && !self.disabling_dodge(actor, peer, Violet, false)? {
                    if target == Magenta
                        && peer.effects.is_enhanced
                        && !peer.effects.is_consumer
                        && !peer.effects.is_fungus
                        && peer.use_energy(
                            self.session.0.parameters.borrow().lifecycle.magenta_energy_consumption)?
                    {
                        tint(peer, Magenta);
                        tint(actor, Violet);
                    } else if self.violet_poison(actor, peer, peer_segment)? {
                        self.restore_contact_effects(peer)?;
                        if target == Lavender {
                            peer.body.update_effects = 2;
                        }
                    }
                }
            }
            Olive | DarkOlive => {
                let dark = color(peer, peer_segment)? == DarkOlive;
                if !self.olive_disabling_shield(
                    actor,
                    peer,
                    segment,
                    peer_segment,
                    Violet,
                    self.session.0.parameters.borrow().lifecycle.violet_energy_consumption,
                    dark,
                )? && self.violet_poison(actor, peer, peer_segment)?
                {
                    if !peer.effects.is_plant {
                        peer.body.update_effects = 2;
                    }
                    peer.effects.is_injured = true;
                }
            }
            Silver => {
                if ((!actor.effects.is_consumer && !actor.effects.is_fungus)
                    || actor.state.total_kills >= peer.state.total_kills
                    || (!peer.effects.is_consumer && !peer.effects.is_fungus))
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    peer.effects.is_injured = true;
                }
            }
            Fire | Cream => {
                let target = color(peer, peer_segment)?;
                let plain = if target == Fire {
                    (!actor.effects.is_consumer && !actor.effects.is_fungus)
                        || peer.effects.is_enhanced
                } else {
                    !actor.effects.is_consumer
                        && !actor.effects.is_fungus
                        && !actor.effects.is_infectious
                };
                if plain {
                    if self.violet_poison(actor, peer, peer_segment)? {
                        peer.effects.is_injured = true;
                    }
                } else if self.violet_payment(actor)? {
                    peer.body.set_color(peer_segment, DarkFire.color_ref())?;
                    peer.effects.use_extra_effects = true;
                    tint(actor, Violet);
                    peer.effects.is_injured = true;
                }
            }
            Plague => {
                if (peer.effects.is_infectious
                    || (!actor.effects.is_consumer && !actor.effects.is_fungus))
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    peer.effects.is_injured = true;
                }
            }
            Fruit => {
                if !actor.effects.is_consumer
                    && !actor.effects.is_fungus
                    && self.violet_payment(actor)?
                {
                    peer.body.set_color(peer_segment, DarkFire.color_ref())?;
                    tint(actor, Violet);
                    peer.effects.is_injured = true;
                }
            }
            Spike => {
                if (peer.effects.is_plant || peer.effects.is_enhanced)
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    peer.effects.is_injured = true;
                }
            }
            Lilac | DarkLilac => {
                if ((!actor.effects.is_consumer && !actor.effects.is_fungus)
                    || actor.effects.is_enhanced)
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    peer.effects.is_injured = true;
                }
            }
            Fallow => {
                if !actor.effects.is_consumer
                    && !actor.effects.is_fungus
                    && !actor.effects.is_infectious
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    peer.body.update_effects = 2;
                    peer.effects.is_injured = true;
                }
            }
            Coral => {
                if !self.violet_healing_shield(actor, peer)?
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    if peer.effects.gold > 0
                        && !peer.effects.is_plant
                        && !peer.effects.is_consumer
                        && !peer.effects.is_fungus
                    {
                        peer.body.update_effects = 2;
                    }
                    peer.effects.is_injured = true;
                }
            }
            Maroon => {
                if !self.violet_healing_shield(actor, peer)?
                    && self.violet_poison(actor, peer, peer_segment)?
                {
                    peer.body.update_effects = 2;
                    peer.effects.is_injured = true;
                }
            }
            Pink => {
                if self.violet_poison(actor, peer, peer_segment)? {
                    peer.body.update_effects = 2;
                    peer.effects.is_injured = true;
                }
            }
            Bark => age_bark(peer, peer_segment)?,
            Ochre | SpikePoint | Vision | Ice | DeadBark | Brown => {}
            _ => {
                if self.violet_poison(actor, peer, peer_segment)? {
                    peer.effects.is_injured = true;
                }
            }
        }
        self.disabling_peer_death(actor, peer)?;
        Ok(())
    }

    fn sky_standard(&self, actor: &mut Organism, peer: &mut Organism, healing_gate: bool) -> Result<(), crate::WorldError> {
        if actor.effects.sky_version == 1 {
            if can_paralyze(actor, peer)
                && (!healing_gate
                    || peer.effects.healing == 0
                    || !actor.effects.is_plant
                    || actor.effects.is_enhanced)
            {
                self.paralyzed_from(peer, ContactActor::Other(actor))?;
            }
        } else if !peer.effects.all_frozen {
            self.frozen_from(peer, ContactActor::Other(actor))?;
        }
        Ok(())
    }

    fn sky_matching_shield(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
        fresh: bool,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        if actor.effects.sky_version == 1 {
            if peer.effects.can_move && peer.effects.sky_version == 2 {
                self.paralyzed_from(peer, ContactActor::Other(actor))?;
            }
        } else if peer.effects.sky_version == 2 {
            if fresh {
                if actor.use_energy(
                    self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                        actor.body.set_color(segment, DeepSky.color_ref())?;
                        actor.effects.use_extra_effects = true;
                        peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                        peer.effects.use_extra_effects = true;
                        tint(actor, DeepSky);
                        tint(peer, DeepSky);
                    } else if !peer.effects.all_frozen {
                        self.frozen_from(peer, ContactActor::Other(actor))?;
                    }
                }
            } else if !peer.effects.all_frozen {
                self.frozen_from(peer, ContactActor::Other(actor))?;
            }
        } else if actor.effects.is_plant
            && !actor.effects.is_enhanced
            && peer.use_energy(
                12.0 * self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)?
        {
            if fresh {
                peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                peer.effects.use_extra_effects = true;
            }
            tint(actor, Sky);
            tint(peer, DeepSky);
        } else if !peer.effects.all_frozen {
            self.frozen_from(peer, ContactActor::Other(actor))?;
        }
        Ok(())
    }

    fn sky_magenta_shield(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<bool, crate::WorldError> {
        Ok(if peer.body.color_argb(peer_segment)? == ColorKind::Magenta.argb().unwrap()
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.magenta_energy_consumption)?
        {
            tint(peer, ColorKind::Magenta);
            tint(actor, ColorKind::Sky);
            true
        } else {
            false
        })
    }

    fn sky_dormant(&self, actor: &mut Organism, peer: &mut Organism, blue: bool) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        if actor.use_energy(
            self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
            for color_index in peer.body.indices() {
                if (blue && ColorKind::from_argb(peer.body.color_argb(color_index)?) == Blue)
                    || (!blue && matches!(ColorKind::from_argb(peer.body.color_argb(color_index)?), Fruit | DarkFire))
                {
                    peer.body.set_color(color_index, Ice.color_ref())?;
                }
            }
            tint(actor, Sky);
            peer.state.temporary_color_frames = 0;
            peer.effects.is_injured = true;
            peer.effects.is_frozen = true;
            peer.effects.all_frozen = true;
        }
        Ok(())
    }

    /// Preserve the separate paralysis and full-freezing paths, including sequential shield payments
    pub(super) fn touch_sky(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        if !peer.effects.has_dodged {
            if (actor.effects.sky_version == 1
                || (!peer.effects.can_react && peer.state.temporary_color_frames > 0))
                && (!peer.effects.is_only_c4 || !peer.effects.is_enhanced)
            {
                peer.effects.dodge = false;
            }
            if !peer.effects.is_only_c4
                || !peer.effects.is_enhanced
                || peer.state.temporary_color_frames > 8
            {
                peer.effects.has_dodged = true;
            }
        }
        match color(peer, peer_segment)? {
            Sky => self.sky_matching_shield(actor, peer, segment, peer_segment, true)?,
            DeepSky => self.sky_matching_shield(actor, peer, segment, peer_segment, false)?,
            Green | Grass | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | DarkGreen
            | Purple | Teal | Cyan | Yellow | Auburn | Indigo | Blond | Flower | DarkGray
            | Gold | Spore => {
                if !self.disabling_dodge(actor, peer, Sky, true)? {
                    self.sky_standard(actor, peer, false)?;
                }
            }
            Blue => {
                if peer.state.active {
                    if !self.blue_disabling_shield(actor, peer, peer_segment, Sky)? {
                        self.sky_standard(actor, peer, true)?;
                    }
                } else if primary_genome(&peer.genome)?.borrow().modifiers().spore >= 10
                    && (actor.effects.sky_version == 2 || !actor.effects.is_plant)
                {
                    self.sky_dormant(actor, peer, true)?;
                }
            }
            Jade | DarkJade => {
                let dark = color(peer, peer_segment)? == DarkJade;
                if !self.disabling_dodge(actor, peer, Sky, true)? {
                    if actor.effects.sky_version == 1 {
                        if peer.effects.can_move {
                            self.paralyzed_from(peer, ContactActor::Other(actor))?;
                        }
                    } else if !actor.effects.is_enhanced {
                        if !dark || peer.effects.healing == 0 || !peer.state.active {
                            peer.body.set_color(peer_segment, if dark { DarkGreen } else { DarkJade }.color_ref())?;
                            tint(actor, Sky);
                            peer.effects.use_extra_effects = true;
                        }
                    } else {
                        self.frozen_from(peer, ContactActor::Other(actor))?;
                    }
                }
            }
            Mint | Lavender | Rose | Magenta => {
                if !actor.effects.altruist && !self.disabling_dodge(actor, peer, Sky, true)? {
                    if actor.effects.sky_version == 1 {
                        if can_paralyze(actor, peer) {
                            if peer.effects.healing == 0 {
                                self.paralyzed_from(peer, ContactActor::Other(actor))?;
                            } else if (!actor.effects.is_plant || actor.effects.is_enhanced)
                                && !self.sky_magenta_shield(actor, peer, peer_segment)?
                            {
                                self.paralyzed_from(peer, ContactActor::Other(actor))?;
                            }
                        }
                    } else if !peer.effects.all_frozen
                        && !self.sky_magenta_shield(actor, peer, peer_segment)?
                    {
                        self.frozen_from(peer, ContactActor::Other(actor))?;
                    }
                }
            }
            Maroon => {
                if actor.effects.sky_version == 1 {
                    if peer.effects.can_move {
                        if peer.effects.healing == 0 {
                            self.paralyzed_from(peer, ContactActor::Other(actor))?;
                        } else if !actor.effects.is_plant || actor.effects.is_enhanced {
                            if !actor.effects.is_enhanced
                                && peer.state.temporary_color_frames == 0
                                && (actor.effects.is_consumer || actor.effects.is_fungus)
                                && peer.use_energy(
                                    self.session.0.parameters.borrow().lifecycle.magenta_energy_consumption)?
                            {
                                tint(peer, Magenta);
                                tint(actor, Sky);
                            } else {
                                self.paralyzed_from(peer, ContactActor::Other(actor))?;
                            }
                        }
                    }
                } else if !peer.effects.all_frozen {
                    self.frozen_from(peer, ContactActor::Other(actor))?;
                }
            }
            OldBark | Violet | Gray | LightBrown | GreenBrown | Broken | Ice | DeadBark => {
                self.sky_standard(actor, peer, false)?
            }
            Coral => {
                if actor.effects.sky_version == 1 {
                    self.sky_standard(actor, peer, true)?;
                }
            }
            Cream | Fruit | DarkFire => {
                let darkfire = color(peer, peer_segment)? == DarkFire;
                if peer.state.active {
                    self.sky_standard(actor, peer, !darkfire)?;
                } else if !peer.effects.is_frozen {
                    if peer.effects.spore_version == 1 {
                        self.sky_dormant(actor, peer, false)?;
                    } else if actor.effects.sky_version == 1
                        && peer.effects.spore_version == 5
                        && actor.use_energy(
                            self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)?
                    {
                        tint(actor, Sky);
                        peer.effects.is_frozen = true;
                    }
                }
            }
            Bark => {
                if actor.effects.sky_version == 1 {
                    if peer.effects.can_move {
                        self.paralyzed_from(peer, ContactActor::Other(&mut *actor))?;
                    }
                    age_bark(peer, peer_segment)?;
                } else if !peer.effects.all_frozen {
                    self.frozen_from(peer, ContactActor::Other(actor))?;
                }
            }
            Fallow => {
                if actor.effects.sky_version == 1
                    && peer.effects.can_move
                    && (peer.effects.is_plant || peer.effects.is_infectious)
                {
                    self.paralyzed_from(peer, ContactActor::Other(actor))?;
                }
            }
            Spike => {
                if peer.effects.is_plant || peer.effects.is_enhanced {
                    self.sky_standard(actor, peer, false)?;
                }
            }
            Olive | SpikePoint | Brown => {}
            _ => self.sky_standard(actor, peer, true)?,
        }
        self.disabling_peer_death(actor, peer)?;
        Ok(())
    }

    fn olive_payment(&self, actor: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        actor.use_energy(
            self.session.0.parameters.borrow().lifecycle.olive_energy_consumption)?
    
        })
    }

    fn olive_ice(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        peer_segment: usize,
    ) -> Result<bool, crate::WorldError> {
        Ok(if self.olive_payment(actor)? {
            peer.body.set_color(peer_segment, ColorKind::LightBlue.color_ref())?;
            tint(actor, ColorKind::Olive);
            true
        } else {
            false
        })
    }

    /// Apply olive's plant-specific damage and its ordered double-payment shield contest
    pub(super) fn touch_olive(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        _segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        match color(peer, peer_segment)? {
            Olive | DarkOlive => {
                let dark = color(peer, peer_segment)? == DarkOlive;
                if peer.effects.is_plant {
                    if !actor.effects.is_plant
                        || (actor.effects.is_enhanced
                            && peer.effects.is_infectious
                            && !actor.effects.is_infectious)
                    {
                        if self.olive_ice(actor, peer, peer_segment)? {
                            peer.effects.is_injured = true;
                        }
                    } else if !dark
                        && (peer.effects.is_infectious || !actor.effects.is_infectious)
                        && self.olive_payment(actor)?
                    {
                        if peer.use_energy(
                            self.session.0.parameters.borrow().lifecycle.olive_energy_consumption)? {
                            peer.body.set_color(peer_segment, DarkOlive.color_ref())?;
                            tint(actor, Olive);
                            peer.effects.use_extra_effects = true;
                        } else {
                            peer.body.set_color(peer_segment, LightBlue.color_ref())?;
                            tint(actor, Olive);
                            peer.effects.is_injured = true;
                        }
                    }
                }
            }
            Blue => {
                if peer.state.active {
                    if !actor.effects.is_enhanced
                        && (!peer.effects.is_plant
                            || (actor.effects.is_infectious && actor.effects.is_plant))
                        && peer.use_energy(
                            self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)?
                    {
                        tint(actor, Olive);
                        tint(peer, Blue);
                    } else if self.olive_ice(actor, peer, peer_segment)? {
                        if !peer.effects.is_plant {
                            peer.body.update_effects = 2;
                        }
                        peer.effects.is_injured = true;
                    }
                } else if primary_genome(&peer.genome)?.borrow().modifiers().spore <= 9
                    && ((!actor.effects.is_plant && !actor.effects.is_infectious)
                        || actor.effects.is_coral
                        || actor.effects.is_enhanced)
                    && self.olive_payment(actor)?
                {
                    for color_index in peer.body.indices() {
                        if ColorKind::from_argb(peer.body.color_argb(color_index)?) == Blue {
                            peer.body.set_color(color_index, LightBlue.color_ref())?;
                        }
                    }
                    tint(actor, Olive);
                    peer.state.temporary_color_frames = 0;
                    peer.effects.is_injured = true;
                    peer.effects.is_frozen = true;
                    peer.effects.all_frozen = true;
                }
            }
            Ochre => {
                if (actor.effects.is_enhanced || peer.effects.is_plant)
                    && self.olive_ice(actor, peer, peer_segment)?
                {
                    peer.effects.is_injured = true;
                }
            }
            Sky | DeepSky => {
                if self.olive_ice(actor, peer, peer_segment)? {
                    self.restore_contact_effects(peer)?;
                    if !peer.effects.is_plant && !primary_genome(&peer.genome)?.borrow().modifiers().modifies_sky {
                        peer.body.update_effects = 2;
                    }
                }
            }
            Lilac | DarkLilac | Spike | SpikePoint => {
                if self.olive_payment(actor)? {
                    peer.body.set_color(peer_segment, if peer.effects.is_plant {
                        Broken
                    } else {
                        LightBrown
                    }
                    .color_ref())?;
                    tint(actor, Olive);
                    peer.effects.is_injured = true;
                }
            }
            Fallow => {
                if ((!actor.effects.is_consumer
                    && !actor.effects.is_fungus
                    && (!actor.effects.is_infectious || !actor.effects.is_plant))
                    || actor.effects.is_enhanced)
                    && peer.effects.is_plant
                    && self.olive_payment(actor)?
                {
                    peer.body.set_color(peer_segment, if peer.effects.is_plant {
                        Broken
                    } else {
                        LightBrown
                    }
                    .color_ref())?;
                    tint(actor, Olive);
                    peer.body.update_effects = 2;
                    peer.effects.is_injured = true;
                }
            }
            Bark | OldBark => {
                let fresh = color(peer, peer_segment)? == Bark;
                if ((!actor.effects.is_plant && !actor.effects.is_consumer)
                    || peer.effects.is_consumer
                    || actor.effects.is_enhanced
                    || peer.effects.is_enhanced)
                    && peer.state.active
                {
                    if self.olive_payment(actor)? {
                        peer.body.set_color(peer_segment, DeadBark.color_ref())?;
                        if fresh {
                            peer.effects.photosynthesis -= peer.body.photo(peer_segment)?
                                * GROWTH_SCALES[crate::geometry::growth_index(peer.body.growth_ratio)?];
                        }
                        tint(actor, Olive);
                        peer.effects.is_frozen = true;
                    }
                } else if fresh {
                    age_bark(peer, peer_segment)?;
                }
            }
            _ => {}
        }
        self.disabling_peer_death(actor, peer)?;
        Ok(())
    }

    fn gray_payment(&self, actor: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        actor.use_energy(
            self.session.0.parameters.borrow().lifecycle.gray_energy_consumption)?
    
        })
    }

    fn gray_outcome(&self, actor: &mut Organism, peer: &mut Organism, survive: bool) -> Result<(), crate::WorldError> {
        if self.gray_payment(actor)? {
            if survive {
                self.survive_from(peer, ContactActor::Other(actor))?;
            } else {
                self.die(peer, Some(Killer::Other(&mut *actor)))?;
                tint(actor, ColorKind::Gray);
            }
        }
        Ok(())
    }

    fn gray_healing_shield(&self, actor: &mut Organism, peer: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        if peer.use_energy(
            self.session.0.parameters.borrow().lifecycle.magenta_energy_consumption)? {
            tint(peer, ColorKind::Magenta);
            tint(actor, ColorKind::Gray);
            true
        } else {
            false
        }
    
        })
    }

    /// Preserve gray's immediate deaths and healing-survival paths, including their source cache side effects
    pub(super) fn touch_gray(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        if !peer.effects.has_dodged
            && (!peer.effects.is_only_c4
                || !peer.effects.is_enhanced
                || peer.state.temporary_color_frames > 8)
        {
            peer.effects.has_dodged = true;
        }
        match color(peer, peer_segment)? {
            Brown | SpikePoint => {}
            Fallow => {
                if actor.effects.is_enhanced
                    && (peer.effects.is_plant || peer.effects.is_infectious)
                {
                    let survive = healing_survival(peer);
                    if !survive || !self.gray_healing_shield(actor, peer)? {
                        self.gray_outcome(actor, peer, survive)?;
                    }
                }
            }
            Pink | OldBark => {
                if actor.effects.is_enhanced
                    && !actor.effects.is_consumer
                    && !actor.effects.is_fungus
                {
                    let survive = healing_survival(peer) && peer.state.active;
                    if !survive || !self.gray_healing_shield(actor, peer)? {
                        self.gray_outcome(actor, peer, survive)?;
                    }
                }
            }
            Coral => {
                if (!actor.effects.is_plant || actor.effects.is_enhanced)
                    && !actor.effects.is_consumer
                    && !actor.effects.is_fungus
                {
                    self.gray_outcome(actor, peer, healing_survival(peer))?;
                }
            }
            Maroon => {
                if !(peer.effects.healing > 0
                    && peer.state.temporary_color_frames == 0
                    && (actor.effects.is_consumer || actor.effects.is_fungus)
                    && self.gray_healing_shield(actor, peer)?)
                {
                    self.gray_outcome(
                        actor,
                        peer,
                        peer.effects.healing > 0 && !peer.effects.is_gray,
                    )?;
                }
            }
            Green | Grass | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | DarkGreen
            | Purple | Teal | Cyan | Yellow | Auburn | Indigo | Blond | Flower | DarkGray
            | Gold | Spore => {
                if !self.disabling_dodge(actor, peer, Gray, false)? {
                    self.gray_outcome(actor, peer, healing_survival(peer) && peer.state.active)?;
                }
            }
            Jade | DarkJade => {
                let dark = color(peer, peer_segment)? == DarkJade;
                if !self.disabling_dodge(actor, peer, Gray, false)? {
                    if peer.effects.healing > 0 && !actor.effects.is_enhanced && peer.state.active {
                        peer.body.set_color(peer_segment, if dark { DarkGreen } else { DarkJade }.color_ref())?;
                        tint(actor, Gray);
                        peer.effects.use_extra_effects = true;
                    } else {
                        self.gray_outcome(actor, peer, healing_survival(peer) && peer.state.active)?;
                    }
                }
            }
            White => {
                if classified(peer) || peer.effects.plague_version > 0 || actor.effects.is_enhanced
                {
                    self.gray_outcome(actor, peer, healing_survival(peer))?;
                } else {
                    let survive = healing_survival(peer);
                    if !survive || !self.gray_healing_shield(actor, peer)? {
                        self.gray_outcome(actor, peer, survive)?;
                    }
                }
            }
            Blue => {
                if !self.blue_disabling_shield(actor, peer, peer_segment, Gray)? {
                    self.gray_outcome(actor, peer, healing_survival(peer) && peer.state.active)?;
                }
            }
            Sky => {
                if peer.effects.sky_version == 2 || peer.effects.is_plant {
                    if peer.use_energy(
                        self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                        peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                        peer.effects.use_extra_effects = true;
                        tint(peer, DeepSky);
                        tint(actor, Gray);
                    } else {
                        self.gray_outcome(actor, peer, healing_survival(peer))?;
                    }
                }
            }
            DeepSky => {
                if peer.effects.sky_version == 2 || peer.effects.is_plant {
                    if actor.effects.is_consumer || actor.effects.is_fungus {
                        tint(peer, DeepSky);
                        tint(actor, Gray);
                    } else {
                        self.gray_outcome(actor, peer, healing_survival(peer))?;
                    }
                }
            }
            Ochre => {
                if peer.effects.is_plant {
                    self.gray_outcome(actor, peer, healing_survival(peer) && peer.state.active)?;
                }
            }
            Mint | Lavender | Rose | Magenta => {
                if !actor.effects.altruist && !self.disabling_dodge(actor, peer, Gray, false)? {
                    if !(peer.body.color_argb(peer_segment)? == Magenta.argb().unwrap()
                        && !peer.effects.is_gray
                        && self.gray_healing_shield(actor, peer)?)
                    {
                        self.gray_outcome(actor, peer, healing_survival(peer))?;
                    }
                }
            }
            Red => {
                if !peer.effects.is_enhanced {
                    self.gray_outcome(
                        actor,
                        peer,
                        peer.effects.healing > 0 && !peer.effects.is_gray,
                    )?;
                }
            }
            Silver => {
                if actor.state.total_kills >= peer.state.total_kills
                    || (!peer.effects.is_consumer && !peer.effects.is_fungus)
                {
                    self.gray_outcome(actor, peer, healing_survival(peer))?;
                }
            }
            Spike => {
                if peer.effects.is_plant || peer.effects.is_enhanced {
                    self.gray_outcome(actor, peer, healing_survival(peer))?;
                }
            }
            Olive | DarkOlive => {
                let dark = color(peer, peer_segment)? == DarkOlive;
                if !self.olive_disabling_shield(
                    actor,
                    peer,
                    segment,
                    peer_segment,
                    Gray,
                    self.session.0.parameters.borrow().lifecycle.gray_energy_consumption,
                    dark,
                )? {
                    self.gray_outcome(actor, peer, healing_survival(peer))?;
                }
            }
            Violet => {
                if healing_survival(peer) {
                    if self.gray_payment(actor)? {
                        self.survive_from(peer, ContactActor::Other(actor))?;
                        peer.body.set_color(peer_segment, DarkFire.color_ref())?;
                        peer.effects.use_extra_effects = true;
                        peer.effects.is_injured = true;
                    }
                } else {
                    self.gray_outcome(actor, peer, false)?;
                }
            }
            Bark => age_bark(peer, peer_segment)?,
            _ => self.gray_outcome(actor, peer, healing_survival(peer) && peer.state.active)?,
        }
        self.disabling_peer_death(actor, peer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EffectsState, Gene, Genome, GenomeProperties, WorldConfig};
    use std::rc::Rc;

    fn setup(
        weapon: ColorKind,
        targets: &[ColorKind],
    ) -> (World, super::super::OrganismRef, super::super::OrganismRef) {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        for (colors, center) in [(&[weapon][..], [1000, 1000]), (targets, [3500, 3500])] {
            let genes: Vec<_> = colors
                .iter()
                .enumerate()
                .map(|(index, color)| {
                    Gene::new(12.0, index as f64 * 0.25, color.argb(), -1, [0; 36]).shared()
                })
                .collect();
            world
                .paste_genome(
                    Genome::from_parts(
                        &genes,
                        GenomeProperties {
                            symmetry: 1,
                            ..GenomeProperties::default()
                        },
                    ),
                    center,
                )
                .unwrap()
                .unwrap();
        }
        let actor = Rc::clone(&world.organisms.borrow()[0]);
        let peer = Rc::clone(&world.organisms.borrow()[1]);
        for reference in [&actor, &peer] {
            let mut organism = reference.borrow_mut();
            organism.effects = EffectsState {

                ..EffectsState::default()
            };
            organism.effects.has_dodged = false;
            organism.state.energy = 100.0;
            organism.state.active = true;
            organism.state.temporary_color = None;
            organism.state.temporary_color_frames = 0;
            organism.body.set_physical_length(0, 6.25).unwrap();
        }
        actor.borrow_mut().state.energy = 20.0;
        world.oxygen.set(1000.0);
        world.carbon_dioxide.set(0.0);
        (world, actor, peer)
    }

    #[test]
    fn violet_jade_transitions_are_free_but_the_following_poison_requires_payment() {
        let (world, actor_ref, peer_ref) = setup(ColorKind::Violet, &[ColorKind::Jade]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.is_plant = true;
        actor.state.energy = 0.0;
        world.touch_violet(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::DarkJade.argb().unwrap()
        );
        world.touch_violet(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::DarkGreen.argb().unwrap()
        );
        world.touch_violet(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::DarkGreen.argb().unwrap()
        );
        assert!(peer.effects.use_extra_effects);
        assert!(!peer.effects.is_injured);
        assert_eq!(actor.state.energy, 0.0);
    }

    #[test]
    fn violet_spring_rechecks_original_gene_lengths_and_wraps_spin_without_rebuilding() {
        let (world, actor_ref, peer_ref) = setup(
            ColorKind::Violet,
            &[
                ColorKind::Spring,
                ColorKind::Spring,
                ColorKind::Jade,
                ColorKind::Spring,
            ],
        );
        world.session.0.parameters.borrow_mut().effects.dark_jade_delay = 7;
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.is_plant = true;
        peer.body.set_color_argb(2, ColorKind::DarkGreen.argb().unwrap()).unwrap();
        for index in [1, 3] {
            peer.genome.as_ref().unwrap().borrow().gene_ref(index).unwrap().borrow_mut().set_length(1e20);
        }
        peer.effects.spin = 123;
        peer.effects.jade_factor = 0;
        let geometry = peer.body.rotated_values().unwrap();
        world.touch_violet(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.effects.spin, -2);
        assert_eq!(peer.effects.jade_factor, 14);
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::GreenBrown.argb().unwrap()
        );
        assert_eq!(peer.body.update_effects, 2);
        assert_eq!(peer.body.rotated_values().unwrap(), geometry);
    }

    #[test]
    fn violet_movement_poison_resets_photo_and_only_passive_teal_requests_cache_update() {
        for (target, passive, update) in [
            (ColorKind::Cyan, false, 7),
            (ColorKind::Teal, false, 7),
            (ColorKind::Teal, true, 2),
        ] {
            let (world, actor_ref, peer_ref) = setup(ColorKind::Violet, &[target]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            let mut properties = peer.genome.as_ref().unwrap().borrow().properties();
            properties.modifiers.passive = passive;
            peer.genome.as_ref().unwrap().borrow_mut().set_properties(properties);
            peer.body.set_photo(0, 9.0).unwrap();
            peer.body.update_effects = 7;
            world.touch_violet(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(peer.body.photo_values().unwrap(), [0.0]);
            assert_eq!(peer.body.update_effects, update);
            assert!(peer.effects.is_injured);
        }
    }

    #[test]
    fn violet_keeps_the_failed_sky_payment_before_its_own_poison_attempt() {
        let (world, actor_ref, peer_ref) = setup(ColorKind::Violet, &[ColorKind::Sky]);
        world.session.0.parameters.borrow_mut().lifecycle.sky_energy_consumption = 0.25;
        world.session.0.parameters.borrow_mut().lifecycle.violet_energy_consumption = 0.25;
        world.oxygen.set(0.125);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        peer.effects.sky_version = 2;
        world.touch_violet(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.state.energy, 99.875);
        assert_eq!(actor.state.energy, 20.0);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 0.125);
        assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::Sky.argb().unwrap());
        assert_eq!(actor.state.temporary_argb(), None);
    }

    #[test]
    fn violet_fruit_darkfire_does_not_enable_the_cream_extra_effects_flag() {
        for target in [ColorKind::Cream, ColorKind::Fruit] {
            let (world, actor_ref, peer_ref) = setup(ColorKind::Violet, &[target]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            actor.effects.is_infectious = true;
            world.touch_violet(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(
                peer.body.color_argb(0).unwrap(),
                ColorKind::DarkFire.argb().unwrap()
            );
            assert_eq!(peer.effects.use_extra_effects, target == ColorKind::Cream);
            assert!(peer.effects.is_injured);
        }
    }

    #[test]
    fn olive_retaliation_keeps_defender_payment_and_attacker_injury_before_a_failed_counterpayment()
    {
        for gray in [false, true] {
            let (world, actor_ref, peer_ref) = setup(
                if gray {
                    ColorKind::Gray
                } else {
                    ColorKind::Violet
                },
                &[ColorKind::Olive],
            );
            world.session.0.parameters.borrow_mut().lifecycle.olive_energy_consumption = 0.25;
            world.session.0.parameters.borrow_mut().lifecycle.violet_energy_consumption = 0.25;
            world.session.0.parameters.borrow_mut().lifecycle.gray_energy_consumption = 0.25;
            world.oxygen.set(0.375);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            if gray {
                world.touch_gray(&mut actor, &mut peer, 0, 0).unwrap();
            } else {
                world.touch_violet(&mut actor, &mut peer, 0, 0).unwrap();
            }
            assert_eq!(peer.state.energy, 99.75);
            assert_eq!(actor.state.energy, 19.875);
            assert_eq!(world.oxygen.get(), 0.0);
            assert_eq!(world.carbon_dioxide.get(), 0.375);
            assert_eq!(
                actor.body.color_argb(0).unwrap(),
                ColorKind::LightBrown.argb().unwrap()
            );
            assert!(actor.effects.is_injured);
            assert_eq!(actor.state.temporary_argb(), None);
            assert_eq!(peer.state.temporary_argb(), ColorKind::Olive.argb());
            assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::Olive.argb().unwrap());
            assert!(peer.state.alive);
        }
    }

    #[test]
    fn sky_preserves_previous_dodge_processing_and_clears_only_an_unprocessed_dodge() {
        for processed in [false, true] {
            let (world, actor_ref, peer_ref) = setup(ColorKind::Sky, &[ColorKind::Green]);
            world.session.0.parameters.borrow_mut().lifecycle.dodge_energy_consumption = 0.25;
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            actor.effects.sky_version = 1;
            peer.effects.has_dodged = processed;
            peer.effects.dodge = true;
            peer.effects.can_move = false;
            world.touch_sky(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(peer.effects.dodge, processed);
            assert!(peer.effects.has_dodged);
            assert_eq!(peer.state.energy, if processed { 99.75 } else { 100.0 });
        }
    }

    #[test]
    fn sky_matching_shields_preserve_both_partial_payments_before_freezing_fallback() {
        let (world, actor_ref, peer_ref) = setup(ColorKind::Sky, &[ColorKind::Sky]);
        world.session.0.parameters.borrow_mut().lifecycle.sky_energy_consumption = 0.25;
        world.oxygen.set(0.375);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.sky_version = 2;
        peer.effects.sky_version = 2;
        world.touch_sky(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.state.energy, 19.75);
        assert_eq!(peer.state.energy, 99.875);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 0.375);
        assert_eq!(actor.body.color_argb(0).unwrap(), ColorKind::Sky.argb().unwrap());
        assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::Sky.argb().unwrap());
        assert!(!peer.effects.all_frozen);
    }

    #[test]
    fn enhanced_sky_retries_jade_freezing_even_when_the_all_frozen_cache_is_set() {
        let (world, actor_ref, peer_ref) = setup(ColorKind::Sky, &[ColorKind::Jade]);
        world.session.0.parameters.borrow_mut().lifecycle.sky_energy_consumption = 0.25;
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.sky_version = 2;
        actor.effects.is_enhanced = true;
        peer.effects.all_frozen = true;
        world.touch_sky(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.state.energy, 19.75);
        assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::Ice.argb().unwrap());
        assert!(peer.effects.is_injured && peer.effects.is_frozen && peer.effects.all_frozen);
    }

    #[test]
    fn sky_version_five_dormancy_changes_only_the_frozen_flag_after_payment() {
        let (world, actor_ref, peer_ref) = setup(ColorKind::Sky, &[ColorKind::Cream]);
        world.session.0.parameters.borrow_mut().lifecycle.sky_energy_consumption = 0.25;
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        actor.effects.sky_version = 1;
        peer.state.active = false;
        peer.effects.spore_version = 5;
        peer.state.temporary_color_frames = 7;
        peer.effects.photosynthesis = 9.0;
        world.touch_sky(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(peer.effects.is_frozen);
        assert!(!peer.effects.all_frozen && !peer.effects.is_injured);
        assert_eq!(peer.state.temporary_color_frames, 7);
        assert_eq!(peer.effects.photosynthesis, 9.0);
        assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::Cream.argb().unwrap());
        assert_eq!(actor.state.energy, 19.75);
    }

    #[test]
    fn olive_sky_restoration_precedes_the_modifier_specific_update_flag() {
        for modified in [false, true] {
            let (world, actor_ref, peer_ref) = setup(ColorKind::Olive, &[ColorKind::Sky]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            let mut properties = peer.genome.as_ref().unwrap().borrow().properties();
            properties.modifiers.modifies_sky = modified;
            peer.genome.as_ref().unwrap().borrow_mut().set_properties(properties);
            peer.body.update_effects = 7;
            world.touch_olive(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(
                peer.body.color_argb(0).unwrap(),
                ColorKind::LightBlue.argb().unwrap()
            );
            assert_eq!(peer.body.update_effects, if modified { 7 } else { 2 });
            assert_eq!(peer.effects.sky_version, 0);
        }
    }

    #[test]
    fn olive_failed_eligible_bark_attack_does_not_take_the_ineligible_aging_branch() {
        for eligible in [false, true] {
            let (world, actor_ref, peer_ref) = setup(ColorKind::Olive, &[ColorKind::Bark]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            actor.state.energy = 0.0;
            actor.effects.is_plant = !eligible;
            actor.effects.is_consumer = !eligible;
            peer.body.growth_ratio = 1;
            peer.effects.photosynthesis = 10.0;
            peer.body.set_photo(0, 3.0).unwrap();
            world.touch_olive(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(
                peer.body.color_argb(0).unwrap(),
                if eligible {
                    ColorKind::Bark
                } else {
                    ColorKind::OldBark
                }
                .argb()
                .unwrap()
            );
            assert_eq!(
                peer.effects.photosynthesis,
                if eligible { 10.0 } else { 7.0 }
            );
            assert!(!peer.effects.is_frozen);
        }
    }

    #[test]
    fn gray_healing_survival_respects_the_gray_consumer_exception_and_changes_all_magenta() {
        for consumer in [false, true] {
            let (world, actor_ref, peer_ref) =
                setup(ColorKind::Gray, &[ColorKind::Green, ColorKind::Magenta]);
            let mut actor = actor_ref.borrow_mut();
            let mut peer = peer_ref.borrow_mut();
            peer.effects.healing = 1;
            peer.effects.is_gray = true;
            peer.effects.is_consumer = consumer;
            world.touch_gray(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(peer.state.alive, !consumer);
            assert_eq!(
                peer.body.color_argb(1).unwrap(),
                if consumer {
                    ColorKind::Brown
                } else {
                    ColorKind::DarkFire
                }
                .argb()
                .unwrap()
            );
            if !consumer {
                assert_eq!(peer.effects.healing, 0);
                assert!(peer.effects.use_extra_effects && peer.effects.is_injured);
            }
            assert_eq!(actor.state.total_kills, i32::from(consumer));
        }
    }

    #[test]
    fn gray_white_keeps_a_failed_healing_payment_before_the_attack_payment() {
        let (world, actor_ref, peer_ref) = setup(ColorKind::Gray, &[ColorKind::White]);
        world.session.0.parameters.borrow_mut().lifecycle.magenta_energy_consumption = 0.25;
        world.session.0.parameters.borrow_mut().lifecycle.gray_energy_consumption = 0.25;
        world.oxygen.set(0.125);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        peer.effects.healing = 1;
        world.touch_gray(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.state.energy, 99.875);
        assert_eq!(actor.state.energy, 20.0);
        assert_eq!(peer.effects.healing, 1);
        assert!(peer.state.alive);
        assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::White.argb().unwrap());
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 0.125);
    }

    #[test]
    fn gray_violet_survival_applies_the_hit_segment_damage_after_survive() {
        let (world, actor_ref, peer_ref) =
            setup(ColorKind::Gray, &[ColorKind::Violet, ColorKind::Magenta]);
        let mut actor = actor_ref.borrow_mut();
        let mut peer = peer_ref.borrow_mut();
        peer.effects.healing = 1;
        world.touch_gray(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(peer.state.alive && peer.effects.use_extra_effects && peer.effects.is_injured);
        assert_eq!(peer.effects.healing, 0);
        assert!(
            (peer.body.indices()).all(|index| peer.body.color_argb(index).unwrap() == ColorKind::DarkFire.argb().unwrap())
        );
        assert_eq!(actor.state.temporary_argb(), ColorKind::Gray.argb());
    }

    #[test]
    fn every_disabling_noop_still_uses_the_strict_alive_peer_death_guard() {
        for branch in 0..4 {
            for energy in [TOLERANCE, f64::from_bits(TOLERANCE.to_bits() - 1)] {
                let (world, actor_ref, peer_ref) =
                    setup(ColorKind::Violet, &[ColorKind::Brown]);
                let mut actor = actor_ref.borrow_mut();
                let mut peer = peer_ref.borrow_mut();
                peer.state.energy = energy;
                match branch {
                    0 => world.touch_violet(&mut actor, &mut peer, 0, 0).unwrap(),
                    1 => world.touch_sky(&mut actor, &mut peer, 0, 0).unwrap(),
                    2 => world.touch_olive(&mut actor, &mut peer, 0, 0).unwrap(),
                    _ => world.touch_gray(&mut actor, &mut peer, 0, 0).unwrap(),
                }
                assert_eq!(peer.state.alive, energy == TOLERANCE);
                assert_eq!(actor.state.total_kills, i32::from(energy < TOLERANCE));
            }
        }
    }
}
