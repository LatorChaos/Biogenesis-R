// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Complete SPIKEPOINT, OCHRE, and SILVER branches from Organism.java:11281-12742
//! Each entry includes the shared peer-energy death guard at Organism.java:13106-13108

use crate::organism::primary_genome;

use super::death::Killer;
use super::draining_contacts::age_bark;
use super::infection::{InfectionKind, Infector};
use super::{Organism, World, WorldError};
use crate::{ColorKind, TOLERANCE, physics::between};
use crate::organism::same_genome;

fn color(actor: &Organism, segment: usize) -> Result<ColorKind, crate::WorldError> {
    Ok(actor.body.color_kind(segment)?)
}

fn tint(actor: &mut Organism, color: ColorKind) {
    actor.set_display_color(Some(color.color_ref().expect("Tint requires a named source literal")));
}

fn infection_differs(recipient: &Organism, donor: &Organism) -> bool {
    !same_genome(recipient.infected_genome.as_ref(), donor.genome.as_ref())
}

fn spike_prey(actor: &Organism, peer: &Organism) -> bool {
    peer.effects.is_consumer
        || peer.effects.is_fungus
        || (!peer.effects.is_plant
            && (actor.effects.is_consumer || actor.effects.is_fungus || peer.effects.is_coral))
}

fn silver_infection(actor: &Organism, peer: &Organism) -> bool {
    infection_differs(peer, actor) && peer.state.total_infected < actor.state.total_infected
}

fn silver_experience(actor: &mut Organism) -> bool {
    // The original assignment occurs before the enhanced alternative is evaluated
    (actor.state.total_kills > 0
        && (actor.effects.is_consumer
            || (actor.effects.is_fungus && {
                actor.effects.is_consumer = true;
                true
            })))
        || actor.effects.is_enhanced
}

fn ochre_damping(actor: &mut Organism, rubbing: f64) {
    actor.state.velocity[0] = crate::java_math::mul(actor.state.velocity[0], rubbing / 1.5);
    if actor.state.velocity[0].abs() < TOLERANCE {
        actor.state.velocity[0] = 0.0;
    }
    actor.state.velocity[1] = crate::java_math::mul(actor.state.velocity[1], rubbing / 1.5);
    if actor.state.velocity[1].abs() < TOLERANCE {
        actor.state.velocity[1] = 0.0;
    }
    actor.state.angular_velocity = crate::java_math::mul(actor.state.angular_velocity, rubbing / 1.5);
    if actor.state.angular_velocity.abs() < TOLERANCE {
        actor.state.angular_velocity = 0.0;
    }
}

impl World {
    fn special_peer_death(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        if peer.state.alive && peer.state.energy < TOLERANCE {
            self.die(peer, Some(Killer::Other(actor)))?;
        }
        Ok(())
    }

    fn spike_paid_amount(
        &self,
        actor: &mut Organism,
        peer: &Organism,
        segment: usize,
        cost: f64,
        factor: f64,
    ) -> Result<Option<f64>, crate::WorldError> {
        if actor.use_energy(cost)? {
            let taken = between(
                crate::java_math::mul(
                    crate::java_math::mul(factor, crate::java_math::sqrt(actor.body.physical_length(segment)?)),
                    self.session.0.parameters.borrow().lifecycle.organic_obtained_energy,
                ),
                0.0,
                peer.state.energy,
            );
            Ok(Some(taken))
        } else {
            Ok(None)
        }
    }

    fn spike_paid(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        cost: f64,
        factor: f64,
        peer_tint: ColorKind,
    ) -> Result<Option<f64>, crate::WorldError> {
        let Some(taken) = self.spike_paid_amount(actor, peer, segment, cost, factor)? else { return Ok(None); };
        tint(peer, peer_tint);
        tint(actor, ColorKind::Spike);
        Ok(Some(taken))
    }

    fn spike_hit(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        ordinary_factor: f64,
        enhanced_factor: f64,
        enhanced_tint: ColorKind,
    ) -> Result<f64, crate::WorldError> {
        let (cost, factor, target) = if actor.effects.is_enhanced {
            (
                self.session.0.parameters.borrow().lifecycle.mosquito_energy_consumption,
                enhanced_factor,
                enhanced_tint,
            )
        } else if actor.effects.is_plant || spike_prey(actor, peer) {
            (
                self.session.0.parameters.borrow().lifecycle.spike_energy_consumption,
                ordinary_factor,
                ColorKind::DarkLilac,
            )
        } else {
            return Ok(0.0);
        };
        Ok(self.spike_paid(actor, peer, segment, cost, factor, target)?
            .unwrap_or(0.0))
    }

    fn spike_dodge(&self, actor: &mut Organism, peer: &mut Organism) -> Result<bool, crate::WorldError> {
        Ok({
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            tint(peer, ColorKind::Teal);
            tint(actor, ColorKind::Spike);
            true
        } else {
            false
        }
    
        })
    }

    fn spike_dodge_hit(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        ordinary_factor: f64,
        enhanced_factor: f64,
        enhanced_tint: ColorKind,
    ) -> Result<f64, crate::WorldError> {
        if (actor.effects.is_enhanced || actor.effects.is_plant) && self.spike_dodge(actor, peer)? {
            Ok(0.0)
        } else {
            self.spike_hit(
                actor,
                peer,
                segment,
                ordinary_factor,
                enhanced_factor,
                enhanced_tint,
            )
        }
    }

    /// Apply the endpoint-specific SPIKE branch, including its respiration-only ordinary damage
    pub(super) fn touch_spikepoint(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        let mut taken = 0.0;
        peer.effects.has_dodged = true;
        match color(peer, peer_segment)? {
            SpikePoint | OldBark | Cream => {}
            Spike => {
                if peer.effects.is_plant || peer.effects.is_enhanced {
                    taken = self.spike_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
                }
            }
            Brown => {
                if !actor.effects.is_enhanced && peer.state.alive && actor.effects.is_plant {
                    if let Some(amount) = self.spike_paid(
                        actor,
                        peer,
                        segment,
                        self.session.0.parameters.borrow().lifecycle.spike_energy_consumption,
                        5.0,
                        DarkLilac,
                    )? {
                        taken = amount;
                        if !peer.effects.is_enhanced {
                            peer.effects.is_injured = true;
                        }
                    }
                }
            }
            Blue => {
                if peer.use_energy(
                    self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)? {
                    if peer.effects.is_enhanced
                        && (actor.effects.is_enhanced || actor.effects.is_plant)
                    {
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
                        tint(actor, Spike);
                    }
                    tint(peer, Blue);
                } else {
                    taken = self.spike_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
                }
            }
            Sky => {
                if peer.use_energy(
                    self.session.0.parameters.borrow().lifecycle.sky_energy_consumption)? {
                    peer.body.set_color(peer_segment, DeepSky.color_ref())?;
                    peer.effects.use_extra_effects = true;
                    tint(actor, Spike);
                    tint(peer, DeepSky);
                } else {
                    taken = self.spike_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
                }
            }
            DeepSky => {
                tint(actor, Spike);
                tint(peer, DeepSky);
            }
            Ochre => {
                if peer.effects.is_plant {
                    taken = self.spike_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
                }
            }
            Coral => {
                if !actor.effects.is_plant
                    && !actor.effects.is_consumer
                    && !actor.effects.is_fungus
                    && spike_prey(actor, peer)
                {
                    taken = self
                        .spike_paid(
                            actor,
                            peer,
                            segment,
                            self.session.0.parameters.borrow().lifecycle.spike_energy_consumption,
                            5.0,
                            DarkLilac,
                        )?
                        .unwrap_or(0.0);
                }
            }
            Red => {
                if !peer.effects.is_enhanced {
                    taken = self.spike_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
                }
            }
            Mint | Lavender | Magenta | Rose => {
                if !actor.effects.altruist {
                    taken = self.spike_dodge_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
                }
            }
            White => {
                if !actor.effects.is_enhanced
                    || peer.effects.is_plant
                    || peer.effects.is_consumer
                    || peer.effects.is_fungus
                    || peer.effects.plague_version > 0
                    || peer.effects.is_auburn
                {
                    taken = self.spike_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
                }
            }
            Grass => {
                taken = self.spike_dodge_hit(actor, peer, segment, 2.5, 0.4, GreenBrown)?;
            }
            Green | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade | DarkJade
            | DarkGreen | Purple | Teal | Cyan | Auburn | Indigo | Blond | Flower | DarkGray
            | Gold | Spore => {
                taken = self.spike_dodge_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
            }
            Yellow => {
                if actor.effects.is_enhanced {
                    if !self.spike_dodge(actor, peer)? {
                        let (factor, target) = if peer.effects.is_consumer
                            || peer.effects.is_fungus
                            || peer.effects.is_plant
                            || peer.effects.transfers_energy
                        {
                            (0.8, Yellow)
                        } else {
                            (0.1, GreenBrown)
                        };
                        taken = self
                            .spike_paid(
                                actor,
                                peer,
                                segment,
                                self.session.0.parameters.borrow().lifecycle.mosquito_energy_consumption,
                                factor,
                                target,
                            )?
                            .unwrap_or(0.0);
                    }
                } else {
                    taken = self.spike_dodge_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
                }
            }
            Fruit => {
                if actor.effects.is_enhanced {
                    if peer.effects.spore_version == 2 {
                        if let Some(amount) = self.spike_paid_amount(
                            actor,
                            peer,
                            segment,
                            self.session.0.parameters.borrow().lifecycle.mosquito_energy_consumption,
                            0.8,
                        )? {
                            taken = amount;
                            tint(peer, Yellow);
                            if infection_differs(actor, peer) && actor.effects.antiviral == 0 {
                                self.infect_from(
                                    actor,
                                    Infector::Other(&mut *peer),
                                    InfectionKind::Fruit,
                                )?;
                                tint(actor, Blond);
                            } else {
                                tint(actor, Spike);
                            }
                        }
                    }
                } else if actor.effects.is_plant {
                    taken = self
                        .spike_paid(
                            actor,
                            peer,
                            segment,
                            self.session.0.parameters.borrow().lifecycle.spike_energy_consumption,
                            5.0,
                            DarkLilac,
                        )?
                        .unwrap_or(0.0);
                }
            }
            Maroon => {
                if actor.effects.is_enhanced || actor.effects.is_plant {
                    taken = self.spike_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
                } else if spike_prey(actor, peer) {
                    taken = self
                        .spike_paid(
                            actor,
                            peer,
                            segment,
                            self.session.0.parameters.borrow().lifecycle.spike_energy_consumption,
                            0.8,
                            DarkLilac,
                        )?
                        .unwrap_or(0.0);
                }
            }
            Silver => {
                let target = if actor.state.total_kills >= peer.state.total_kills
                    || (!peer.effects.is_consumer && !peer.effects.is_fungus)
                {
                    Yellow
                } else {
                    Gold
                };
                taken = self.spike_hit(actor, peer, segment, 5.0, 0.8, target)?;
            }
            Plague => {
                let (factor, target) = if peer.effects.is_infectious {
                    (0.8, Yellow)
                } else {
                    (0.1, GreenBrown)
                };
                taken = self.spike_hit(actor, peer, segment, 5.0, factor, target)?;
            }
            Fallow => {
                if !actor.effects.is_enhanced {
                    taken = self.spike_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
                }
            }
            Bark => age_bark(peer, peer_segment)?,
            Olive => {}
            _ => {
                taken = self.spike_hit(actor, peer, segment, 5.0, 0.8, Yellow)?;
            }
        }
        if taken > 0.0 {
            if actor.effects.is_enhanced {
                peer.state.energy = crate::java_math::sub(peer.state.energy, taken);
                actor.state.energy = crate::java_math::add(actor.state.energy, taken);
                let waste = crate::java_math::mul(taken, self.session.0.parameters.borrow().lifecycle.organic_subs_produced);
                actor.use_energy(waste)?;
            } else {
                peer.use_energy(taken)?;
            }
        }
        self.special_peer_death(actor, peer)?;
        Ok(())
    }

    /// Preserve the random gate, integer-center subtraction, and source shield-specific push costs
    pub(super) fn touch_ochre(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        actor.effects.has_dodged = true;
        let target = color(peer, peer_segment)?;
        if target != Olive && (target != Brown || peer.state.alive) && self.session.0.random.borrow_mut().next_bool() {
            let cost = if actor.effects.is_plant {
                self.session.0.parameters.borrow().lifecycle.ochre_energy_consumption
            } else {
                self.session.0.parameters.borrow().lifecycle.ochre_energy_consumption / 100.0
            };
            if actor.use_energy(cost)? {
                if !actor.effects.use_frame_movement {
                    ochre_damping(actor, self.session.0.parameters.borrow().lifecycle.rubbing);
                }
                if matches!(target, Sky | DeepSky)
                    && peer.effects.sky_version == 1
                    && actor.effects.is_plant
                {
                    tint(actor, Ochre);
                    tint(peer, DeepSky);
                } else {
                    let length = actor.body.physical_length(segment)?;
                    peer.state.velocity[0] = between(
                        crate::java_math::div(
                            crate::java_math::mul(crate::java_math::mul(crate::java_math::mul(
                                f64::from(peer.state.integer_center[0].wrapping_sub(actor.state.integer_center[0])),
                                0.2,
                            ), length), length),
                            peer.body.mass,
                        ),
                        -self.session.0.parameters.borrow().physics.max_velocity,
                        self.session.0.parameters.borrow().physics.max_velocity,
                    );
                    peer.state.velocity[1] = between(
                        crate::java_math::div(
                            crate::java_math::mul(crate::java_math::mul(crate::java_math::mul(
                                f64::from(peer.state.integer_center[1].wrapping_sub(actor.state.integer_center[1])),
                                0.2,
                            ), length), length),
                            peer.body.mass,
                        ),
                        -self.session.0.parameters.borrow().physics.max_velocity,
                        self.session.0.parameters.borrow().physics.max_velocity,
                    );
                    if actor.effects.is_enhanced
                        && !matches!(target, Cream | Coral)
                        && (target != Red || peer.effects.create_lavender == 0)
                    {
                        let blue_protected = target == Blue
                            && peer.use_energy(
                                self.session.0.parameters.borrow().lifecycle.blue_energy_consumption)?;
                        if blue_protected {
                            tint(peer, Blue);
                        } else {
                            if peer.effects.lavender > 0 {
                                self.lavender_shield(peer);
                            }
                            if peer.effects.lavender <= 0 {
                                let speed =
                                    peer.state.velocity[0].abs() + peer.state.velocity[1].abs();
                                let organic = self.session.0.parameters.borrow().lifecycle.organic_obtained_energy;
                                let amount = match target {
                                    Ochre if peer.effects.is_enhanced => speed * (organic / 250.0),
                                    Maroon if peer.effects.is_enhanced => 0.5 * speed * organic,
                                    Maroon => speed * organic,
                                    _ => 2.0 * speed * organic,
                                };
                                peer.use_energy(
                                    between(amount, 0.0, peer.state.energy))?;
                                tint(peer, DarkLilac);
                                if target == Brown && !peer.effects.is_enhanced {
                                    peer.effects.is_injured = true;
                                }
                            }
                        }
                    }
                    tint(actor, Ochre);
                }
            }
        }
        self.special_peer_death(actor, peer)?;
        Ok(())
    }

    fn silver_infect(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        Ok({
        if actor.use_energy(
            self.session.0.parameters.borrow().lifecycle.silver_energy_consumption)? {
            self.infect_from(peer, Infector::Other(&mut *actor), InfectionKind::Direct)?;
            tint(peer, ColorKind::Blond);
            tint(actor, ColorKind::Silver);
        }
    
        })
    }

    fn silver_indigo(&self, actor: &mut Organism, peer: &mut Organism) -> Result<(), crate::WorldError> {
        Ok({
        if actor.effects.indigo == 0
            || actor.effects.is_plant
            || actor.effects.is_consumer
            || actor.effects.is_fungus
        {
            let cost = (self.session.0.parameters.borrow().lifecycle.silver_energy_consumption + 0.01)
                / self.session.0.parameters.borrow().lifecycle.indigo_energy_consumption;
            if actor.use_energy(cost)? {
                tint(peer, ColorKind::Indigo);
                tint(actor, ColorKind::Silver);
            }
        }
    
        })
    }

    fn silver_take(
        &self,
        actor: &mut Organism,
        peer: &Organism,
        segment: usize,
        factor: Option<f64>,
    ) -> Result<Option<f64>, WorldError> {
        if actor.use_energy(
            self.session.0.parameters.borrow().lifecycle.experience_energy_consumption)? {
            let denominator = peer.state.total_kills.wrapping_add(12);
            if denominator == 0 {
                return Err(WorldError::Arithmetic(
                    "SILVER kill ratio has a zero denominator",
                ));
            }
            let ratio = actor
                .state
                .total_kills
                .wrapping_add(12)
                .wrapping_div(denominator);
            let length = crate::java_math::sqrt(actor.body.physical_length(segment)?);
            let length = match factor {
                Some(factor) => crate::java_math::mul(factor, length),
                None => length,
            };
            Ok(Some(between(
                crate::java_math::mul(crate::java_math::mul(f64::from(ratio), length), self.session.0.parameters.borrow().lifecycle.organic_obtained_energy),
                0.0,
                peer.state.energy,
            )))
        } else {
            Ok(None)
        }
    }

    fn silver_attack(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        factor: Option<f64>,
        target: ColorKind,
    ) -> Result<f64, WorldError> {
        if let Some(amount) = self.silver_take(actor, peer, segment, factor)? {
            tint(peer, target);
            tint(actor, ColorKind::Gold);
            Ok(amount)
        } else {
            Ok(0.0)
        }
    }

    fn silver_attack_dodge(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        factor: Option<f64>,
        target: ColorKind,
    ) -> Result<f64, WorldError> {
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            tint(peer, ColorKind::Teal);
            tint(actor, ColorKind::Gold);
            Ok(0.0)
        } else {
            self.silver_attack(actor, peer, segment, factor, target)
        }
    }

    fn silver_infection_dodge(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        always_mark: bool,
    ) -> Result<bool, crate::WorldError> {
        Ok({
        if peer.effects.dodge
            && peer.use_energy(
                self.session.0.parameters.borrow().lifecycle.dodge_energy_consumption)?
        {
            if always_mark
                || !peer.effects.is_only_c4
                || (!actor.effects.is_plant
                    && (!peer.effects.is_auburn || peer.state.temporary_color_frames > 8))
                || (peer.state.temporary_color_frames > 8 && !peer.effects.is_auburn)
            {
                peer.effects.has_dodged = true;
            }
            tint(peer, ColorKind::Teal);
            tint(actor, ColorKind::Silver);
            true
        } else {
            false
        }
    
        })
    }

    /// Apply infection before experience attacks and preserve the fungus-to-consumer assignment
    pub(super) fn touch_silver(
        &self,
        actor: &mut Organism,
        peer: &mut Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        let mut taken = 0.0;
        match color(peer, peer_segment)? {
            Silver => {
                if silver_infection(actor, peer)
                    && (peer.effects.is_plant
                        || peer.effects.is_consumer
                        || (peer.effects.is_fungus && peer.state.total_kills > 0))
                {
                    self.silver_infect(actor, peer)?;
                }
                if peer.effects.indigo > 0
                    && actor.state.total_infected > 0
                    && !peer.effects.is_plant
                    && !peer.effects.is_consumer
                    && !peer.effects.is_fungus
                {
                    self.silver_indigo(actor, peer)?;
                }
                if silver_experience(actor) {
                    let target = if (peer.state.total_kills > 0
                        && (peer.effects.is_consumer || peer.effects.is_fungus))
                        || peer.effects.is_enhanced
                    {
                        Gold
                    } else {
                        Yellow
                    };
                    taken = self.silver_attack(actor, peer, segment, None, target)?;
                }
            }
            White | Plague => {
                if silver_infection(actor, peer)
                    && (peer.effects.is_plant || peer.effects.is_consumer)
                {
                    self.silver_infect(actor, peer)?;
                }
                if peer.effects.indigo > 0
                    && actor.state.total_infected > 0
                    && !peer.effects.is_plant
                    && !peer.effects.is_consumer
                    && !peer.effects.is_fungus
                {
                    self.silver_indigo(actor, peer)?;
                }
                if (peer.effects.is_plant
                    || peer.effects.is_consumer
                    || peer.effects.is_fungus
                    || peer.effects.plague_version > 0
                    || peer.effects.is_auburn)
                    && silver_experience(actor)
                {
                    taken = self.silver_attack(actor, peer, segment, None, Yellow)?;
                }
            }
            Fruit => {
                if peer.effects.spore_version == 2 && silver_experience(actor) {
                    if let Some(amount) = self.silver_take(actor, peer, segment, Some(0.1))? {
                        taken = amount;
                        tint(peer, GreenBrown);
                        if infection_differs(actor, peer) {
                            self.infect_from(
                                actor,
                                Infector::Other(&mut *peer),
                                InfectionKind::Fruit,
                            )?;
                            tint(actor, Blond);
                        } else {
                            tint(actor, Gold);
                        }
                    }
                }
            }
            Indigo => {
                if actor.state.total_infected > 0 {
                    self.silver_indigo(actor, peer)?;
                }
                if silver_experience(actor) {
                    taken = self.silver_attack_dodge(actor, peer, segment, None, Yellow)?;
                }
            }
            Grass => {
                if silver_infection(actor, peer) && !self.silver_infection_dodge(actor, peer, true)?
                {
                    if peer.effects.jade_factor > 1 {
                        peer.body.set_color(peer_segment, DarkGreen.color_ref())?;
                        tint(actor, Silver);
                        peer.effects.has_dodged = true;
                        peer.effects.use_extra_effects = true;
                    } else {
                        self.silver_infect(actor, peer)?;
                    }
                }
                if silver_experience(actor) {
                    taken =
                        self.silver_attack_dodge(actor, peer, segment, Some(0.5), GreenBrown)?;
                }
            }
            Green | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade | DarkJade => {
                if silver_infection(actor, peer) && !self.silver_infection_dodge(actor, peer, false)?
                {
                    if peer.effects.jade_factor > 1 {
                        let target =
                            if peer.body.color_argb(peer_segment)? == Jade.argb().unwrap() {
                                DarkJade
                            } else {
                                DarkGreen
                            };
                        peer.body.set_color(peer_segment, target.color_ref())?;
                        tint(actor, Silver);
                        peer.effects.has_dodged = true;
                        peer.effects.use_extra_effects = true;
                    } else {
                        self.silver_infect(actor, peer)?;
                    }
                }
                if silver_experience(actor) {
                    taken = self.silver_attack_dodge(actor, peer, segment, None, Yellow)?;
                }
            }
            DarkGreen | Purple | Teal | Cyan | Yellow | Auburn | Blond | Flower | DarkGray
            | Gold | Spore | Magenta | Rose => {
                let altruist_break =
                    matches!(color(peer, peer_segment)?, Magenta | Rose) && actor.effects.altruist;
                if !altruist_break {
                    if silver_infection(actor, peer)
                        && (peer.effects.is_plant || peer.effects.is_consumer)
                        && !self.silver_infection_dodge(actor, peer, false)?
                    {
                        self.silver_infect(actor, peer)?;
                    }
                    if silver_experience(actor) {
                        taken = self.silver_attack_dodge(actor, peer, segment, None, Yellow)?;
                    }
                }
            }
            Red | Fire | Orange => {
                let target = color(peer, peer_segment)?;
                if silver_infection(actor, peer) {
                    self.silver_infect(actor, peer)?;
                }
                if silver_experience(actor) {
                    taken = self.silver_attack(actor, peer, segment, None, target)?;
                }
            }
            Pink => {
                if silver_infection(actor, peer) && primary_genome(&peer.genome)?.borrow().modifiers().modifies_pink {
                    self.silver_infect(actor, peer)?;
                }
                if silver_experience(actor) {
                    taken = self.silver_attack(actor, peer, segment, None, Yellow)?;
                }
            }
            Spike => {
                if silver_infection(actor, peer)
                    && (peer.effects.is_plant || peer.effects.is_consumer)
                {
                    self.silver_infect(actor, peer)?;
                }
                if (peer.effects.is_plant
                    || peer.effects.is_enhanced
                    || peer.state.total_kills < actor.state.total_kills)
                    && silver_experience(actor)
                {
                    taken = self.silver_attack(actor, peer, segment, None, Yellow)?;
                }
            }
            SpikePoint => {
                if peer.state.total_kills < actor.state.total_kills && silver_experience(actor) {
                    let target = if peer.effects.is_enhanced {
                        Spike
                    } else {
                        Yellow
                    };
                    taken = self.silver_attack(actor, peer, segment, None, target)?;
                }
            }
            Lavender | Mint => {
                if !actor.effects.altruist && silver_experience(actor) {
                    taken = self.silver_attack_dodge(actor, peer, segment, None, Yellow)?;
                }
            }
            Blue => {
                if silver_infection(actor, peer)
                    && (peer.effects.is_plant || peer.effects.is_consumer)
                {
                    self.silver_infect(actor, peer)?;
                }
                if silver_experience(actor) {
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
                            tint(actor, Gold);
                        }
                        tint(peer, Blue);
                    } else {
                        taken = self.silver_attack(actor, peer, segment, None, Yellow)?;
                    }
                }
            }
            Cream => {
                if silver_infection(actor, peer) && peer.effects.is_plant {
                    self.silver_infect(actor, peer)?;
                }
            }
            Maroon | Coral | Ochre | Sky | Olive | OldBark | Fallow => {
                if silver_infection(actor, peer)
                    && (peer.effects.is_plant || peer.effects.is_consumer)
                {
                    self.silver_infect(actor, peer)?;
                }
            }
            Bark => {
                age_bark(peer, peer_segment)?;
                if silver_infection(actor, peer)
                    && !actor.effects.is_plant
                    && !actor.effects.is_consumer
                    && !actor.effects.is_fungus
                {
                    self.silver_infect(actor, peer)?;
                }
            }
            Brown => {}
            _ => {
                if silver_infection(actor, peer)
                    && (peer.effects.is_plant || peer.effects.is_consumer)
                {
                    self.silver_infect(actor, peer)?;
                }
                if silver_experience(actor) {
                    taken = self.silver_attack(actor, peer, segment, None, Yellow)?;
                }
            }
        }
        if !peer.effects.has_dodged
            && ((actor.state.total_kills > 0 && actor.effects.is_consumer)
                || actor.effects.is_enhanced)
        {
            peer.effects.has_dodged = true;
        }
        if taken > 0.0 {
            peer.state.energy = crate::java_math::sub(peer.state.energy, taken);
            actor.state.energy = crate::java_math::add(actor.state.energy, taken);
            let waste = crate::java_math::mul(taken, self.session.0.parameters.borrow().lifecycle.organic_subs_produced);
            actor.use_energy(waste)?;
        }
        self.special_peer_death(actor, peer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use crate::{Gene, Genome, GenomeProperties, JavaRandom, WorldConfig};
    use std::cell::RefCell;

    fn setup(
        peer_color: ColorKind,
    ) -> (World, super::super::OrganismRef, super::super::OrganismRef) {
        let world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        let code = Genome::from_parts(
            &[Gene::new(6.25, 0.0, ColorKind::Green.argb(), -1, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        for center in [[100, 100], [300, 300]] {
            world.paste_genome(code.clone(), center).unwrap().unwrap();
        }
        let actor = Rc::clone(&world.organisms.borrow()[0]);
        let peer = Rc::clone(&world.organisms.borrow()[1]);
        for reference in [&actor, &peer] {
            let mut organism = reference.borrow_mut();
            organism.effects = crate::EffectsState::default();
            organism.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![0.0]));
            organism.effects.has_dodged = false;
            organism.state.energy = 100.0;
            organism.state.temporary_color = None;
            organism.state.temporary_color_frames = 0;
            organism.body.set_physical_length(0, 6.25).unwrap();
        }
        actor.borrow_mut().state.energy = 20.0;
        peer.borrow_mut().body.set_color_argb(0, peer_color.argb().unwrap()).unwrap();
        world.oxygen.set(40.0);
        world.carbon_dioxide.set(0.0);
        world.session.0.random.replace(JavaRandom::new(0));
        (world, actor, peer)
    }

    #[test]
    fn spike_ordinary_damage_respires_while_enhanced_damage_transfers_energy() {
        for enhanced in [false, true] {
            let (world, actor, peer) = setup(ColorKind::Green);
            let mut actor = actor.borrow_mut();
            let mut peer = peer.borrow_mut();
            actor.effects.is_enhanced = enhanced;
            actor.effects.is_plant = !enhanced;
            let reactions = peer.body.reaction_values(0).unwrap();
            world.touch_spikepoint(&mut actor, &mut peer, 0, 0).unwrap();
            if enhanced {
                assert_eq!(peer.state.energy, 96.0);
                assert_eq!(actor.state.energy, 23.6);
                assert_eq!(world.oxygen.get(), 39.6);
                assert_eq!(world.carbon_dioxide.get(), 0.4);
            } else {
                assert_eq!(peer.state.energy, 75.0);
                assert_eq!(actor.state.energy, 20.0);
                assert_eq!(world.oxygen.get(), 15.0);
                assert_eq!(world.carbon_dioxide.get(), 25.0);
            }
            assert_eq!(peer.body.reaction_values(0).unwrap(), reactions);
            assert!(peer.effects.has_dodged);
        }
    }

    #[test]
    fn spike_failed_dodge_and_attack_keep_both_partial_respirations() {
        let (world, actor, peer) = setup(ColorKind::Green);
        world.session.0.parameters.borrow_mut().lifecycle.dodge_energy_consumption = 1.0;
        world.session.0.parameters.borrow_mut().lifecycle.mosquito_energy_consumption = 0.25;
        world.oxygen.set(0.5);
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.effects.is_enhanced = true;
        peer.effects.dodge = true;
        world.touch_spikepoint(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 0.5);
        assert_eq!(peer.state.energy, 99.5);
        assert_eq!(actor.state.energy, 20.0);
        assert_eq!(actor.state.temporary_argb(), None);
        assert_eq!(peer.state.temporary_argb(), None);
    }

    #[test]
    fn translucent_cream_enters_the_default_spike_branch_without_losing_argb() {
        for (argb, energy) in [(0xffd0c08c, 100.0), (0x12d0c08c, 96.0)] {
            let (world, actor, peer) = setup(ColorKind::Cream);
            let mut actor = actor.borrow_mut();
            let mut peer = peer.borrow_mut();
            actor.effects.is_enhanced = true;
            peer.body.set_color_argb(0, argb).unwrap();
            world.touch_spikepoint(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(peer.state.energy, energy);
            assert_eq!(peer.body.color_argb(0).unwrap(), argb);
        }
    }

    #[test]
    fn ochre_keeps_internal_zero_mass_nan_motion_and_paid_damage() {
        let (world, actor, peer) = setup(ColorKind::Green);
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.effects.is_enhanced = true;
        actor.state.integer_center = [0, 0];
        peer.state.integer_center = [0, 0];
        peer.body.mass = 0.0;
        world.touch_ochre(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(peer.state.velocity.iter().all(|value| value.is_nan()));
        assert!(peer.state.energy.is_nan() && world.oxygen.get().is_nan());
        assert!(world.carbon_dioxide.get().is_nan());
        assert!(peer.state.alive);
        assert_eq!(peer.state.temporary_argb(), ColorKind::DarkLilac.argb());
        assert_eq!(actor.state.temporary_argb(), ColorKind::Ochre.argb());
    }

    #[test]
    fn reflected_spike_exhaustion_precedes_the_common_recipient_death() {
        let (world, actor, peer) = setup(ColorKind::Blue);
        world.session.0.parameters.borrow_mut().lifecycle.blue_energy_consumption = 0.1;
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.effects.is_enhanced = true;
        peer.effects.is_enhanced = true;
        actor.state.energy = 1.0;
        peer.state.energy = 0.1;
        world.touch_spikepoint(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(!actor.state.alive && !peer.state.alive);
        assert_eq!(world.population.get(), 0);
        assert_eq!(actor.state.total_kills, 1);
        assert_eq!(peer.state.total_kills, 1);
        assert_eq!(peer.state.temporary_argb(), ColorKind::Blue.argb());
        assert!(Rc::ptr_eq(
            world.statistics_ref().borrow().get_last_dead_being().unwrap(), peer.genome.as_ref().unwrap()));
    }

    #[test]
    fn bark_aging_preserves_gene_reactions_and_photo_coefficient_caches() {
        let (world, actor, peer) = setup(ColorKind::Bark);
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        peer.body.growth_ratio = 1;
        peer.effects.photosynthesis = 10.0;
        peer.body.set_photo(0, 3.0).unwrap();
        peer.body.set_reaction(0, 17, 1234).unwrap();
        let angle = peer.state.last_angle;
        world.touch_spikepoint(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.effects.photosynthesis, 7.0);
        assert_eq!(
            peer.body.color_argb(0).unwrap(),
            ColorKind::OldBark.argb().unwrap()
        );
        assert_eq!(
            peer.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow().color(),
            ColorKind::Green.argb()
        );
        assert_eq!(peer.body.photo_values().unwrap(), [3.0]);
        peer.body.set_color_argb(0, ColorKind::Bark.argb().unwrap()).unwrap();
        world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.effects.photosynthesis, 4.0);
        assert_eq!(peer.body.reaction(0, 17).unwrap(), 1234);
        assert_eq!(peer.state.last_angle, angle);
    }

    #[test]
    fn ochre_draws_only_for_eligible_colors_before_attempting_payment() {
        for (color, alive, draws) in [
            (ColorKind::Olive, true, false),
            (ColorKind::Brown, false, false),
            (ColorKind::Green, true, true),
        ] {
            let (world, actor, peer) = setup(color);
            let mut expected = world.random_snapshot();
            if draws {
                expected.next_bool();
            }
            let mut actor = actor.borrow_mut();
            let mut peer = peer.borrow_mut();
            actor.state.energy = 0.0;
            peer.state.alive = alive;
            peer.state.velocity = [3.0, 4.0];
            world.touch_ochre(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(world.session.0.random.borrow_mut().next_i64(), expected.next_i64());
            assert!(actor.effects.has_dodged);
            assert_eq!(peer.state.velocity, [3.0, 4.0]);
        }
    }

    #[test]
    fn ochre_uses_wrapped_integer_centers_and_damps_only_nonmoving_attackers() {
        let (world, actor, peer) = setup(ColorKind::Cream);
        assert!(world.random_snapshot().next_bool());
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.body.set_physical_length(0, 1.0).unwrap();
        peer.body.mass = 1.0;
        actor.state.integer_center = [i32::MAX, 4];
        peer.state.integer_center = [i32::MIN, 4];
        actor.state.center = [-999.0; 2];
        peer.state.center = [999.0; 2];
        actor.state.velocity = [1e-12, 9.0];
        actor.state.angular_velocity = -1e-12;
        actor.effects.is_enhanced = true;
        world.touch_ochre(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.state.velocity, [0.2, 0.0]);
        assert_eq!(actor.state.velocity, [0.0, 5.88]);
        assert_eq!(actor.state.angular_velocity.to_bits(), 0);
        assert_eq!(peer.state.energy, 100.0);
        assert_eq!(actor.state.integer_center, [i32::MAX, 4]);
        assert_eq!(peer.state.integer_center, [i32::MIN, 4]);
    }

    #[test]
    fn ochre_sky_version_one_blocks_plant_push_after_payment_and_damping() {
        let (world, actor, peer) = setup(ColorKind::Sky);
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.effects.is_plant = true;
        actor.effects.is_enhanced = true;
        actor.state.velocity = [0.0, 1.5];
        peer.state.velocity = [3.0, 4.0];
        peer.effects.sky_version = 1;
        world.touch_ochre(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.state.velocity, [3.0, 4.0]);
        assert_eq!(peer.state.energy, 100.0);
        assert_eq!(actor.state.energy, 19.6);
        assert_eq!(actor.state.velocity, [0.0, 0.98]);
        assert_eq!(peer.state.temporary_argb(), ColorKind::DeepSky.argb());
        assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::Sky.argb().unwrap());
    }

    #[test]
    fn ochre_exhausted_lavender_allows_damage_but_a_remaining_point_blocks_it() {
        for shield in [185, 186] {
            let (world, actor, peer) = setup(ColorKind::Maroon);
            let mut actor = actor.borrow_mut();
            let mut peer = peer.borrow_mut();
            actor.effects.is_enhanced = true;
            actor.body.set_physical_length(0, 1.0).unwrap();
            actor.state.integer_center = [0, 0];
            peer.state.integer_center = [10, 0];
            peer.body.mass = 1.0;
            peer.effects.is_enhanced = true;
            peer.effects.lavender = shield;
            world.touch_ochre(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(peer.state.velocity, [2.0, 0.0]);
            if shield == 185 {
                assert_eq!(peer.state.energy, 98.0);
                assert_eq!(peer.effects.lavender, 0);
                assert_eq!(peer.state.temporary_argb(), ColorKind::DarkLilac.argb());
            } else {
                assert_eq!(peer.state.energy, 100.0);
                assert_eq!(peer.effects.lavender, 1);
                assert_eq!(peer.state.temporary_argb(), ColorKind::LightBlue.argb());
            }
        }
    }

    #[test]
    fn silver_integer_ratio_can_zero_damage_after_promoting_a_fungus() {
        let (world, actor, peer) = setup(ColorKind::Silver);
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.effects.is_fungus = true;
        actor.state.total_kills = 1;
        peer.effects.is_consumer = true;
        peer.state.total_kills = 2;
        world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(actor.effects.is_consumer && actor.effects.is_fungus);
        assert_eq!(actor.state.energy, 20.0);
        assert_eq!(peer.state.energy, 100.0);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Gold.argb());
        assert_eq!(peer.state.temporary_argb(), ColorKind::Gold.argb());
        assert!(peer.effects.has_dodged);
    }

    #[test]
    fn silver_infection_distinguishes_shared_identity_from_equal_valued_codes() {
        let (world, actor, peer) = setup(ColorKind::Cream);
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.state.total_infected = 1;
        peer.effects.is_plant = true;
        world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.state.total_infected, 2);
        assert!(Rc::ptr_eq(
            peer.infected_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
        let paid_energy = actor.state.energy;
        world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.state.total_infected, 2);
        assert_eq!(actor.state.energy, paid_energy);
        peer.infected_genome = Some(Rc::new(RefCell::new(actor.genome.as_ref().unwrap().borrow().clone())));
        world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.state.total_infected, 3);
        assert!(Rc::ptr_eq(
            peer.infected_genome.as_ref().unwrap(), actor.genome.as_ref().unwrap()));
    }

    #[test]
    fn silver_infection_and_experience_dodges_make_two_ordered_payments() {
        let (world, actor, peer) = setup(ColorKind::Green);
        world.session.0.parameters.borrow_mut().lifecycle.dodge_energy_consumption = 0.25;
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.state.total_infected = 1;
        actor.state.total_kills = 1;
        actor.effects.is_consumer = true;
        peer.effects.dodge = true;
        peer.state.energy = 1.0;
        world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.state.energy, 0.5);
        assert_eq!(actor.state.total_infected, 1);
        assert!(peer.infected_genome.is_none());
        assert_eq!(actor.state.temporary_argb(), ColorKind::Gold.argb());
        assert!(peer.effects.has_dodged);
    }

    #[test]
    fn silver_c4_dodge_flag_keeps_the_strict_display_frame_threshold() {
        for (plant, auburn, frames, marked) in [
            (true, true, 8, false),
            (true, true, 9, false),
            (true, false, 8, false),
            (true, false, 9, true),
            (false, true, 8, false),
            (false, true, 9, true),
        ] {
            let (world, actor, peer) = setup(ColorKind::C4);
            let mut actor = actor.borrow_mut();
            let mut peer = peer.borrow_mut();
            actor.effects.is_plant = plant;
            actor.state.total_infected = 1;
            peer.effects.dodge = true;
            peer.effects.is_only_c4 = true;
            peer.effects.is_auburn = auburn;
            peer.state.temporary_color_frames = frames;
            world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap();
            assert_eq!(peer.effects.has_dodged, marked);
            assert_eq!(peer.state.temporary_argb(), ColorKind::Teal.argb());
        }
    }

    #[test]
    fn silver_fruit_infection_ignores_antiviral_and_retains_the_real_donor_code() {
        let (world, actor, peer) = setup(ColorKind::Fruit);
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.effects.is_enhanced = true;
        actor.effects.antiviral = 99;
        peer.effects.spore_version = 2;
        world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap();
        assert!(Rc::ptr_eq(
            actor.infected_genome.as_ref().unwrap(), peer.genome.as_ref().unwrap()));
        assert_eq!(peer.state.total_infected, 1);
        assert_eq!(actor.state.temporary_argb(), ColorKind::Blond.argb());
        assert_eq!(peer.state.energy, 99.5);
        assert_eq!(actor.state.energy, 20.45);
    }

    #[test]
    fn silver_zero_divisor_preserves_source_payment_and_returns_an_error() {
        let (mut world, actor, peer) = setup(ColorKind::Red);
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.effects.is_enhanced = true;
        actor.state.total_kills = 1;
        peer.state.total_kills = -12;
        actor.state.energy = 1000.0;
        peer.state.energy = 1000.0;
        world.oxygen.set(1120.0);
        world.carbon_dioxide.set(499880.0);
        world.session.0.parameters.borrow_mut().lifecycle.experience_energy_consumption = 0.625;
        let random = world.random_snapshot();
        let mut genomes = crate::organism::GenomeCloner::default();
        let statistics = world
            .statistics_ref()
            .borrow()
            .copy_for_world(&mut genomes)
            .into_shared();
        let mut peer_before = peer.copy_for_world(&mut genomes);
        let error = world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap_err();
        assert_eq!(
            error,
            WorldError::Arithmetic("SILVER kill ratio has a zero denominator")
        );
        assert_eq!(actor.state.energy.to_bits(), 0x408f3b0000000000);
        assert_eq!(world.oxygen.get().to_bits(), 0x40917d8000000000);
        assert_eq!(world.carbon_dioxide.get().to_bits(), 0x411e82a280000000);
        assert!(actor.state.alive);
        assert_eq!(actor.state.total_kills, 1);
        assert_eq!(actor.state.temporary_argb(), None);
        assert_eq!(&*peer, &peer_before);
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(world.statistics_ref(), statistics);
        world.oxygen.set(0.25);
        let energy = actor.state.energy;
        world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(actor.state.energy, energy - 0.25);
        assert_eq!(world.oxygen.get(), 0.0);
        peer_before.effects.has_dodged = true;
        assert_eq!(&*peer, &peer_before);
    }

    #[test]
    fn silver_wrapped_integer_division_preserves_java_minimum_over_negative_one() {
        let (world, actor, peer) = setup(ColorKind::Red);
        let mut actor = actor.borrow_mut();
        let mut peer = peer.borrow_mut();
        actor.effects.is_enhanced = true;
        actor.state.total_kills = i32::MAX - 11;
        peer.state.total_kills = -13;
        world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap();
        assert_eq!(peer.state.energy, 100.0);
        assert_eq!(actor.state.energy, 20.0);
        assert_eq!(peer.state.temporary_argb(), ColorKind::Red.argb());
    }

    #[test]
    fn all_three_noop_branches_still_use_the_strict_peer_death_guard() {
        for (branch, color) in [
            (0, ColorKind::Cream),
            (1, ColorKind::Olive),
            (2, ColorKind::Brown),
        ] {
            for energy in [TOLERANCE, 0.0] {
                let (world, actor, peer) = setup(color);
                let mut actor = actor.borrow_mut();
                let mut peer = peer.borrow_mut();
                peer.state.energy = energy;
                match branch {
                    0 => world.touch_spikepoint(&mut actor, &mut peer, 0, 0).unwrap(),
                    1 => world.touch_ochre(&mut actor, &mut peer, 0, 0).unwrap(),
                    _ => world.touch_silver(&mut actor, &mut peer, 0, 0).unwrap(),
                }
                assert_eq!(peer.state.alive, energy == TOLERANCE);
                assert_eq!(
                    actor.state.total_kills,
                    if energy == TOLERANCE { 0 } else { 1 }
                );
            }
        }
    }
}
