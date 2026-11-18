// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Active segment effects from Organism.java:17936-19130

use crate::organism::primary_genome;

use super::{Organism, World, WorldError};
use crate::{ColorKind, geometry::GROWTH_SCALES, physics::between};
use std::f64::consts::PI;

#[derive(Clone, Copy)]
enum Healing {
    None,
    Positive,
    Unconditional,
}

fn original_color(organism: &Organism, index: usize) -> Result<Option<crate::JavaColor>, crate::WorldError> {
    let genome = primary_genome(&organism.genome)?.borrow();
    Ok(genome.gene_for_segment(index)?.borrow().color_handle().cloned())
}

fn leaf_tint(organism: &mut Organism) -> Result<(), WorldError> {
    if organism.state.temporary_color_frames == 1
        && matches!(
            ColorKind::from_argb(organism.state.required_temporary_color()?.argb()),
            ColorKind::Blond | ColorKind::LightBrown | ColorKind::Yellow
                | ColorKind::GreenBrown | ColorKind::DarkLilac
        )
    {
        organism.set_display_color(ColorKind::DarkGreen.color_ref());
    }
    Ok(())
}

impl World {
    fn frame_draw(&self, bound: i32) -> Result<i32, WorldError> {
        self.session.0.random.borrow_mut()
            .next_bounded(bound)
            .map_err(|error| WorldError::InvalidParameters(error.to_string()))
    }

    fn frame_motion(&self, organism: &mut Organism, index: usize, teal: bool) -> Result<(), WorldError> {
        let cost = if teal {
            self.session.0.parameters.borrow().lifecycle.teal_energy_consumption
        } else {
            self.session.0.parameters.borrow().lifecycle.cyan_energy_consumption
        };
        if self.session.0.random.borrow_mut().bounded(100) < 8
            && organism.use_energy(cost)?
        {
            let strength = if teal { 11.88 } else { 12.0 };
            let limits = self.session.0.parameters.borrow().physics;
            for axis in 0..2 {
                organism.state.velocity[axis] = between(
                    organism.state.velocity[axis]
                        + crate::java_math::div(strength * f64::from(organism.body.rotated_end(index, axis)?.wrapping_sub(organism.body.rotated_start(index, axis)?)),
                            organism.body.mass),
                    -limits.max_velocity,
                    limits.max_velocity,
                );
            }
            let sign = (self.session.0.random.borrow_mut().bounded(2) << 1) - 1;
            organism.state.angular_velocity = between(
                organism.state.angular_velocity
                    + crate::java_math::div(f64::from(sign) * organism.body.physical_length(index)? * PI,
                        organism.body.inertia),
                -limits.max_rotation,
                limits.max_rotation,
            );
        }
        Ok(())
    }

    /// Use cached coefficients directly, including negative coefficients unrelated to current color
    pub(super) fn segments_frame_effects(
        &self,
        organism: &mut Organism,
    ) -> Result<(), WorldError> {
        for index in organism.body.reverse_indices() {
            let coefficient = organism.body.photo(index)?;
            if coefficient <= -3.0 {
                self.frame_motion(organism, index, coefficient == -4.0)?;
            }
        }
        Ok(())
    }

    pub(super) fn frame_methanotrophy(&self, organism: &mut Organism) -> Result<(), WorldError> {
        if organism.effects.methanotrophy > 0.0 && (organism.state.age & 0x07) == 0 {
            organism.state.energy += organism.state.world.resolve()?.methanotrophy(organism.effects.methanotrophy);
        }
        Ok(())
    }

    pub(super) fn frame_photo(&self, organism: &mut Organism, base: f64, specialphoto: f64) -> Result<(), WorldError> {
        let colony = organism.effects.colony_photosynthesis > 0.0;
        let lower = organism.effects.lower_symmetric;
        if !(lower > 0 && organism.state.world.resolve()?.carbon_dioxide() < f64::from(lower))
            || f64::from(self.session.0.random.borrow_mut().bounded(lower)) < organism.state.world.resolve()?.carbon_dioxide()
        {
            let mut request = base;
            if specialphoto > 0.0 {
                request -= specialphoto
                    * GROWTH_SCALES[crate::geometry::growth_index(organism.body.growth_ratio)?];
            }
            if colony {
                request += organism.effects.colony_photosynthesis;
            }
            organism.state.energy += organism.state.world.resolve()?.photosynthesis(request);
        }
        if colony {
            organism.effects.colony_photosynthesis = 0.0;
        }
        Ok(())
    }

    pub(super) fn leaf_extra_effects(&self, organism: &mut Organism) -> Result<(), WorldError> {
        self.frame_methanotrophy(organism)?;
        leaf_tint(organism)?;
        self.frame_photo(
            organism,
            organism.effects.photosynthesis - organism.effects.leaf_photo,
            0.0,
        )?;
        Ok(())
    }

    fn heal_segment(
        &self,
        organism: &mut Organism,
        index: usize,
        healing: Healing,
        darkfire: bool,
    ) -> Result<(), WorldError> {
        let eligible = match healing {
            Healing::None => false,
            Healing::Positive => organism.effects.healing > 0,
            Healing::Unconditional => true,
        };
        if eligible
            && self.frame_draw(self.session.0.parameters.borrow().lifecycle.healing)? <= organism.effects.healing
            && organism.use_energy(
                self.session.0.parameters.borrow().lifecycle.magenta_energy_consumption)?
        {
            {
                let destination = organism.body.phenotype_colors.clone();
                let value = original_color(organism, index)?;
                crate::Body::store_color(destination, index, value)?;
            }
            if darkfire {
                self.darkfire_restore_effects(organism)?;
            } else {
                organism.effects.is_frozen = false;
                organism.effects.is_injured = false;
                organism.effects.all_frozen = false;
                organism.body.update_effects = 2;
                self.restore_contact_effects(organism)?;
            }
        }
        Ok(())
    }

    fn darkfire_frame(
        &self,
        organism: &mut Organism,
        index: usize,
        healing: Healing,
    ) -> Result<(), WorldError> {
        self.heal_segment(organism, index, healing, true)?;
        let threshold = {
            let genome = primary_genome(&organism.genome)?.borrow();
            if genome.symmetry() != 3 || genome.mirror() == 0 {
                genome.symmetry()
            } else {
                2
            }
        };
        // Natural restoration still draws after a successful paid restoration
        if self.session.0.random.borrow_mut().bounded(100) < threshold {
            {
                let destination = organism.body.phenotype_colors.clone();
                let value = original_color(organism, index)?;
                crate::Body::store_color(destination, index, value)?;
            }
            self.darkfire_restore_effects(organism)?;
        } else {
            organism.effects.use_extra_effects = true;
        }
        Ok(())
    }

    fn ability_frame(&self, organism: &mut Organism, index: usize, restored: ColorKind) -> Result<(), WorldError> {
        if self.session.0.random.borrow_mut().bounded(100) < 8 {
            organism.body.set_color(index, restored.color_ref())?;
        } else {
            organism.effects.use_extra_effects = true;
        }
        Ok(())
    }

    fn ordinary_extra_segment(
        &self,
        organism: &mut Organism,
        index: usize,
        healing: Healing,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        match ColorKind::from_argb(organism.body.color_argb(index)?) {
            Teal if organism.body.photo(index)? == -4.0 => {
                self.frame_motion(organism, index, true)?;
            }
            Cyan => self.frame_motion(organism, index, false)?,
            GreenBrown | LightBrown | Broken | LightBlue | Ice => {
                self.heal_segment(organism, index, healing, false)?;
            }
            DarkFire => self.darkfire_frame(organism, index, healing)?,
            DarkOlive => self.ability_frame(organism, index, Olive)?,
            DarkLilac => self.ability_frame(organism, index, Lilac)?,
            DeepSky => self.ability_frame(organism, index, Sky)?,
            _ => {}
        }
        Ok(())
    }

    pub(super) fn segments_healing_effects(
        &self,
        organism: &mut Organism,
    ) -> Result<(), WorldError> {
        for index in organism.body.reverse_indices() {
            self.ordinary_extra_segment(organism, index, Healing::Unconditional)?;
        }
        Ok(())
    }

    pub(super) fn segments_extra_effects(
        &self,
        organism: &mut Organism,
    ) -> Result<(), WorldError> {
        for index in organism.body.reverse_indices() {
            self.ordinary_extra_segment(organism, index, Healing::None)?;
        }
        Ok(())
    }

    pub(super) fn segments_extra_plants_effects(
        &self,
        organism: &mut Organism,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        for index in organism.body.reverse_indices() {
            if organism.body.photo(index)? < 0.0 {
                match ColorKind::from_argb(organism.body.color_argb(index)?) {
                    DarkFire => self.darkfire_frame(organism, index, Healing::None)?,
                    DarkOlive => self.ability_frame(organism, index, Olive)?,
                    DarkLilac => self.ability_frame(organism, index, Lilac)?,
                    DeepSky => self.ability_frame(organism, index, Sky)?,
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn jade_frame(
        &self,
        organism: &mut Organism,
        index: usize,
        update: bool,
        addphoto: &mut f64,
        specialphoto: &mut f64,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        let original_ref = original_color(organism, index)?;
        let original = original_ref.as_ref().ok_or(crate::GeometryError::MissingColor { gene: index })?.argb();
        if original == Summer.argb().unwrap() {
            let active = organism.state.world.resolve()?.time() % 3 != i64::from(organism.effects.summer_inactivity);
            if active {
                if update {
                    *addphoto += organism.body.photo(index)?;
                }
                *specialphoto += organism.body.photo(index)?;
            }
            if self.frame_draw(organism.effects.jade_factor)? < 8 {
                organism.body.set_color(index, if active { Summer } else { Winter }.color_ref())?;
            } else {
                organism.effects.use_extra_effects = true;
            }
        } else {
            if update {
                *addphoto += if original == Lime.argb().unwrap() && organism.effects.crowded { self.session.0.parameters.borrow().lifecycle.crowded_lime_energy_consumption * organism.body.photo(index)? } else { organism.body.photo(index)? };
                if original == Forest.argb().unwrap() {
                    organism.effects.forest_photo += organism.body.photo(index)?;
                }
                if original == Leaf.argb().unwrap() {
                    organism.effects.leaf_photo += organism.body.photo(index)?;
                }
            }
            if original != Leaf.argb().unwrap() || organism.state.temporary_color_frames == 0 {
                *specialphoto += if original == Lime.argb().unwrap() && organism.effects.crowded { self.session.0.parameters.borrow().lifecycle.crowded_lime_energy_consumption * organism.body.photo(index)? } else { organism.body.photo(index)? };
            }
            if self.frame_draw(organism.effects.jade_factor)? < 8 {
                organism.body.set_color(index, original_ref.clone())?;
            } else {
                organism.effects.use_extra_effects = true;
            }
        }
        Ok(())
    }

    pub(super) fn jade_extra_effects(&self, organism: &mut Organism) -> Result<(), WorldError> {
        use ColorKind::*;
        let mut unused_photo = 0.0;
        let mut specialphoto = 0.0;
        for index in organism.body.reverse_indices() {
            match ColorKind::from_argb(organism.body.color_argb(index)?) {
                DarkJade => {
                    if self.frame_draw(organism.effects.jade_factor)? < 8 {
                        organism.body.set_color(index, Jade.color_ref())?;
                    } else {
                        organism.effects.use_extra_effects = true;
                    }
                }
                DarkGreen => {
                    self.jade_frame(organism, index, false, &mut unused_photo, &mut specialphoto)?;
                }
                _ => self.ordinary_extra_segment(organism, index, Healing::Positive)?,
            }
        }
        self.frame_methanotrophy(organism)?;
        let leaf = organism.effects.leaf_photo > 0.0 && organism.state.temporary_color_frames > 0;
        if leaf {
            leaf_tint(organism)?;
        }
        let request = if leaf {
            organism.effects.photosynthesis - organism.effects.leaf_photo
        } else {
            organism.effects.photosynthesis
        };
        self.frame_photo(organism, request, specialphoto)?;
        Ok(())
    }

    fn maintenance_segment(
        &self,
        organism: &mut Organism,
        index: usize,
        plant: bool,
        maintenance: &mut f64,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        let kind = ColorKind::from_argb(organism.body.color_argb(index)?);
        let params = self.session.0.parameters.borrow().lifecycle;
        match kind {
            Teal => {
                if organism.body.photo(index)? == -4.0 {
                    self.frame_motion(organism, index, true)?;
                } else {
                    *maintenance -= 0.95 * organism.body.physical_length(index)?;
                }
            }
            Cyan => self.frame_motion(organism, index, false)?,
            Maroon | Indigo => *maintenance -= 0.8 * organism.body.physical_length(index)?,
            Spore | Auburn => *maintenance -= 0.99 * organism.body.physical_length(index)?,
            Violet if !plant && !organism.effects.is_consumer && !organism.effects.is_fungus => {
                *maintenance -= 0.95 * organism.body.physical_length(index)?;
            }
            Pink if !organism.effects.is_gray => *maintenance -= 0.8 * organism.body.physical_length(index)?,
            Flower => *maintenance -= organism.body.physical_length(index)? - (params.flower_energy_consumption * organism.body.physical_length(index)?),
            Lavender => {
                *maintenance -= if plant || organism.effects.is_consumer {
                    0.75 * organism.body.physical_length(index)?
                } else {
                    0.95 * organism.body.physical_length(index)?
                };
            }
            Eye => *maintenance += params.eye_energy_consumption * organism.body.physical_length(index)?,
            Olive if !plant => *maintenance -= 0.55 * organism.body.physical_length(index)?,
            Sky if !plant && organism.effects.sky_version == 1 => *maintenance -= 0.9 * organism.body.physical_length(index)?,
            Blue if !plant => *maintenance -= 0.9 * organism.body.physical_length(index)?,
            Fallow => {
                if !plant {
                    *maintenance -= 0.99 * organism.body.physical_length(index)?;
                } else if organism.effects.fallow_version <= 3 {
                    *maintenance -= 0.9 * organism.body.physical_length(index)?;
                } else if !organism.effects.is_infectious {
                    *maintenance -= 0.8 * organism.body.physical_length(index)?;
                }
            }
            Coral
                if !plant
                    && organism.effects.gold > 0
                    && !organism.effects.is_consumer
                    && !organism.effects.is_fungus =>
            {
                *maintenance -= 0.9 * organism.body.physical_length(index)?;
            }
            Rose => {
                *maintenance -= if !organism.effects.is_killer {
                    organism.body.physical_length(index)? - (params.rose_energy_consumption * organism.body.physical_length(index)?)
                } else {
                    (0.75 * organism.body.physical_length(index)?) - (params.rose_energy_consumption * organism.body.physical_length(index)?)
                };
            }
            Vision => {
                *maintenance -= if organism.body.eyes.enabled {
                    organism.body.physical_length(index)?
                } else {
                    0.95 * organism.body.physical_length(index)?
                };
            }
            Gold => {
                *maintenance -= if plant {
                    if organism.effects.is_only_c4 {
                        0.9 * organism.body.physical_length(index)?
                    } else {
                        0.874 * organism.body.physical_length(index)?
                    }
                } else if organism.effects.is_consumer || organism.effects.is_fungus {
                    0.874 * organism.body.physical_length(index)?
                } else {
                    0.95 * organism.body.physical_length(index)?
                };
            }
            Dark => *maintenance -= organism.body.physical_length(index)? - (params.dark_energy_consumption * organism.body.physical_length(index)?),
            DarkOlive => {
                if !plant {
                    *maintenance -= 0.5 * organism.body.physical_length(index)?;
                }
                self.ability_frame(organism, index, Olive)?;
            }
            DeepSky => {
                if !plant && organism.effects.sky_version == 1 {
                    *maintenance -= 0.9 * organism.body.physical_length(index)?;
                }
                self.ability_frame(organism, index, Sky)?;
            }
            DarkLilac => self.ability_frame(organism, index, Lilac)?,
            GreenBrown | LightBrown | Broken | LightBlue | Ice => {
                self.heal_segment(organism, index, Healing::Positive, false)?;
            }
            DarkFire => self.darkfire_frame(organism, index, Healing::Positive)?,
            _ => {}
        }
        Ok(())
    }

    pub(super) fn segments_update_effects(
        &self,
        organism: &mut Organism,
    ) -> Result<(), WorldError> {
        let mut maintenance = organism.body.mass;
        for index in organism.body.reverse_indices() {
            self.maintenance_segment(organism, index, false, &mut maintenance)?;
        }
        organism.effects.maintenance =
            maintenance / f64::from(self.session.0.parameters.borrow().lifecycle.segment_cost_divisor);
        Ok(())
    }

    pub(super) fn plants_update_effects(
        &self,
        organism: &mut Organism,
    ) -> Result<(), WorldError> {
        use ColorKind::*;
        let mut maintenance = organism.body.mass;
        let mut photo = 0.0;
        let mut specialphoto = 0.0;
        organism.effects.forest_photo = 0.0;
        organism.effects.leaf_photo = 0.0;
        organism.effects.methanotrophy = 0.0;
        for index in organism.body.reverse_indices() {
                let kind = ColorKind::from_argb(organism.body.color_argb(index)?);
            match kind {
                Lime => {
                    photo += if organism.effects.crowded {
                        self.session.0.parameters.borrow().lifecycle.crowded_lime_energy_consumption * organism.body.photo(index)?
                    } else {
                        organism.body.photo(index)?
                    };
                }
                Forest => {
                    photo += organism.body.photo(index)?;
                    organism.effects.forest_photo += organism.body.photo(index)?;
                }
                Leaf => {
                    photo += organism.body.photo(index)?;
                    organism.effects.leaf_photo += organism.body.photo(index)?;
                }
                Spring | Grass | Jade | Green | C4 | Bark => photo += organism.body.photo(index)?,
                Summer => {
                    if organism.state.world.resolve()?.time() % 3 != i64::from(organism.effects.summer_inactivity) {
                        photo += organism.body.photo(index)?;
                    } else {
                        organism.body.set_color(index, Winter.color_ref())?;
                    }
                }
                Purple => {
                    organism.effects.methanotrophy += organism.body.photo(index)?;
                    maintenance -= 0.99 * organism.body.physical_length(index)?;
                }
                Winter => {
                    if organism.state.world.resolve()?.time() % 3 != i64::from(organism.effects.summer_inactivity) {
                        organism.body.set_color(index, Summer.color_ref())?;
                        photo += organism.body.photo(index)?;
                    }
                }
                DarkJade => {
                    photo += organism.body.photo(index)?;
                    if self.frame_draw(organism.effects.jade_factor)? < 8 {
                        organism.body.set_color(index, Jade.color_ref())?;
                    } else {
                        organism.effects.use_extra_effects = true;
                    }
                }
                DarkGreen => {
                    self.jade_frame(organism, index, true, &mut photo, &mut specialphoto)?
                }
                _ => self.maintenance_segment(organism, index, true, &mut maintenance)?,
            }
        }
        organism.effects.maintenance =
            maintenance / f64::from(self.session.0.parameters.borrow().lifecycle.segment_cost_divisor);
        let scale = GROWTH_SCALES[crate::geometry::growth_index(organism.body.growth_ratio)?];
        if organism.effects.methanotrophy > 0.0 {
            organism.effects.methanotrophy *= scale;
            // The source keeps the positive guard before scaling, including zero-amount gas calls
            if organism.state.age & 0x07 == 0 {
                organism.state.energy += organism.state.world.resolve()?.methanotrophy(organism.effects.methanotrophy);
            }
        }
        organism.effects.photosynthesis = photo * scale;
        if organism.effects.photosynthesis > 0.0 {
            if organism.effects.forest_photo > 0.0 {
                organism.effects.forest_photo =
                    self.session.0.parameters.borrow().lifecycle.crowded_forest_energy_consumption
                        * (organism.effects.forest_photo * scale);
            }
            let has_leaf = organism.effects.leaf_photo > 0.0;
            if has_leaf {
                organism.effects.leaf_photo *= scale;
            }
            let leaf = has_leaf && organism.state.temporary_color_frames > 0;
            if leaf {
                leaf_tint(organism)?;
            }
            let request = if leaf {
                organism.effects.photosynthesis - organism.effects.leaf_photo
            } else {
                organism.effects.photosynthesis
            };
            self.frame_photo(organism, request, specialphoto)?;
        }
        Ok(())
    }

    /// Recompute only healing and injury, retaining unrelated freeze and movement caches
    pub(super) fn darkfire_restore_effects(&self, organism: &mut Organism) -> Result<(), crate::WorldError> {
        use ColorKind::*;
        organism.effects.healing = 0;
        organism.effects.is_injured = false;
        for index in organism.body.reverse_indices() {
            match ColorKind::from_argb(organism.body.color_argb(index)?) {
                Magenta => {
                    let genome = primary_genome(&organism.genome)?.borrow();
                    let factor = if genome.symmetry() != 3 {
                        10.0
                    } else if genome.mirror() == 0 {
                        8.0
                    } else {
                        6.0
                    };
                    let length = genome.gene_for_segment(index)?.borrow().length();
                    organism.effects.healing = organism
                        .effects
                        .healing
                        .wrapping_add((factor * length) as i32);
                }
                LightBrown | GreenBrown | DarkFire | LightBlue | Broken | Ice => {
                    organism.effects.is_injured = true;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gene, Genome, GenomeProperties, JavaRandom, WorldConfig};
    use std::{cell::RefCell, rc::Rc};

    fn world() -> World {
        World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap()
    }

    fn placed(world: &mut World, colors: &[ColorKind]) -> Organism {
        let genes: Vec<_> = colors
            .iter()
            .map(|kind| Gene::new(12.0, 0.25, kind.argb(), 0, [0; 36]).shared())
            .collect();
        let genome = Genome::from_parts(
            &genes,
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        world
            .construct_pasted_shared(Rc::new(RefCell::new(genome)), [100, 100])
            .unwrap()
            .unwrap()
    }

    fn set_clock_time(world: &World, time: i64) {
        crate::statistics::tests::set_statistics_input(
            &mut world.statistics_ref().borrow_mut(), "time", "long", &time.to_string(),
            &Default::default(),
        );
    }

    fn clock_actor(world: &mut World, current: ColorKind, original: ColorKind) -> Organism {
        let mut actor = placed(world, &[original]);
        actor.body.set_color(0, current.color_ref()).unwrap();
        actor.body.growth_ratio = 1;
        actor.body.set_photo(0, 2.0).unwrap();
        actor.effects.summer_inactivity = 0;
        actor.effects.lower_symmetric = 0;
        actor.effects.jade_factor = 1;
        actor.effects.use_extra_effects = false;
        actor.effects.photosynthesis = 7.0;
        actor.effects.forest_photo = 19.0;
        actor.effects.leaf_photo = 0.0;
        actor.effects.methanotrophy = 0.0;
        actor.effects.colony_photosynthesis = 0.0;
        actor.state.age = 1;
        actor.state.energy = 0.0;
        actor.state.temporary_color_frames = 0;
        actor
    }

    #[test]
    fn cached_movement_pays_in_reverse_order_and_omits_failed_payment_sign_draw() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Green, ColorKind::Green]);
        actor.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![-3.0, -4.0]));
        actor.body.mass = 1.0;
        actor.body.inertia = 1.0;
        actor.state.velocity = [0.0; 2];
        actor.state.angular_velocity = 0.0;
        for index in actor.body.indices() {
            actor.body.set_rotated_segment(index, crate::RotatedSegment { start: [0, 0], end: [index as i32 + 1, 0] }).unwrap();
        }
        world.session.0.parameters.borrow_mut().lifecycle.teal_energy_consumption = 0.25;
        world.session.0.parameters.borrow_mut().lifecycle.cyan_energy_consumption = 0.25;
        world.session.0.parameters.borrow_mut().physics.max_velocity = 100.0;
        world.oxygen.set(0.375);
        world.carbon_dioxide.set(0.0);
        actor.state.energy = 1.0;
        world.session.0.random.replace(JavaRandom::new(33));
        let mut expected = world.random_snapshot();
        assert_eq!(expected.next_bounded(100).unwrap(), 3);
        expected.next_bounded(2).unwrap();
        assert_eq!(expected.next_bounded(100).unwrap(), 3);
        world.segments_frame_effects(&mut actor).unwrap();
        assert_eq!(actor.state.velocity, [23.76, 0.0]);
        assert_eq!(actor.state.energy, 0.625);
        assert_eq!(world.oxygen.get(), 0.0);
        assert_eq!(world.carbon_dioxide.get(), 0.375);
        assert_eq!(world.random_snapshot(), expected);
        assert!(
            (actor.body.indices()).all(|index| actor.body.color_argb(index).unwrap() == ColorKind::Green.argb().unwrap())
        );
    }

    #[test]
    fn paid_darkfire_healing_still_runs_the_natural_retry() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Magenta]);
        actor.body.set_color_argb(0, ColorKind::DarkFire.argb().unwrap()).unwrap();
        actor.effects.healing = i32::MAX;
        actor.effects.use_extra_effects = false;
        actor.state.energy = 10.0;
        world.oxygen.set(10.0);
        world.session.0.random.replace(JavaRandom::new(0));
        let mut expected = world.random_snapshot();
        expected
            .next_bounded(world.parameters().lifecycle.healing)
            .unwrap();
        assert!(expected.next_bounded(100).unwrap() >= 1);
        world.segments_healing_effects(&mut actor).unwrap();
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::Magenta.argb().unwrap()
        );
        assert_eq!(actor.effects.healing, 120);
        assert!(!actor.effects.is_injured);
        assert!(actor.effects.use_extra_effects);
        assert_eq!(world.random_snapshot(), expected);
        assert_eq!(actor.state.energy, 9.99);
    }

    #[test]
    fn restoration_changes_the_healing_gate_for_later_segments() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Magenta, ColorKind::Green]);
        actor.body.set_color_argb(0, ColorKind::DarkFire.argb().unwrap()).unwrap();
        actor.body.set_color_argb(1, ColorKind::Broken.argb().unwrap()).unwrap();
        actor.effects.healing = i32::MAX;
        actor.state.energy = 10.0;
        world.oxygen.set(10.0);
        world.session.0.random.replace(JavaRandom::new(0));
        let mut expected = world.random_snapshot();
        expected
            .next_bounded(world.parameters().lifecycle.healing)
            .unwrap();
        assert!(expected.next_bounded(100).unwrap() >= 1);
        world.jade_extra_effects(&mut actor).unwrap();
        assert_eq!(
            actor.body.color_argb(1).unwrap(),
            ColorKind::Green.argb().unwrap()
        );
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::DarkFire.argb().unwrap()
        );
        assert_eq!(actor.effects.healing, 0);
        assert!(actor.effects.is_injured);
        assert_eq!(world.random_snapshot(), expected);
    }

    #[test]
    fn plant_extra_effects_observe_negative_cache_gate_and_exact_argb() {
        let mut world = world();
        let mut actor = placed(
            &mut world,
            &[ColorKind::Olive, ColorKind::Lilac, ColorKind::Sky],
        );
        for (index, kind) in [
            ColorKind::DarkOlive,
            ColorKind::DarkLilac,
            ColorKind::DeepSky,
        ].into_iter().enumerate() {
            actor.body.set_color_argb(index, kind.argb().unwrap()).unwrap();
        }
        actor.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![0.0, -0.0, f64::NAN]));
        world.session.0.random.replace(JavaRandom::new(0));
        let expected = world.random_snapshot();
        world.segments_extra_plants_effects(&mut actor).unwrap();
        assert_eq!(world.random_snapshot(), expected);
        actor.body.set_photo(0, -1.0).unwrap();
        actor.body.set_color_argb(0, 0x126b8e23).unwrap();
        world.segments_extra_plants_effects(&mut actor).unwrap();
        assert_eq!(world.random_snapshot(), expected);
        actor.body.set_color_argb(0, ColorKind::DarkOlive.argb().unwrap()).unwrap();
        let mut expected = expected;
        assert_eq!(expected.next_bounded(100).unwrap(), 60);
        world.segments_extra_plants_effects(&mut actor).unwrap();
        assert_eq!(world.random_snapshot(), expected);
        assert!(actor.effects.use_extra_effects);
    }

    #[test]
    fn leaf_photo_clears_failed_colony_attempt_after_methane_and_tint() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Leaf]);
        actor.effects.methanotrophy = 1.0;
        actor.effects.photosynthesis = 10.0;
        actor.effects.leaf_photo = 2.0;
        actor.effects.colony_photosynthesis = 3.0;
        actor.effects.lower_symmetric = 10;
        actor.state.age = 8;
        actor.state.energy = 0.0;
        actor.set_display_color(ColorKind::Blond.color_ref());
        actor.state.temporary_color_frames = 1;
        world.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        world.methane.set(2.0);
        world.oxygen.set(0.0);
        world.carbon_dioxide.set(0.0);
        let mut expected = world.random_snapshot();
        expected.next_bounded(10).unwrap();
        world.leaf_extra_effects(&mut actor).unwrap();
        assert_eq!(actor.state.energy, 1.0);
        assert_eq!(world.methane.get(), 1.0);
        assert_eq!(world.oxygen.get(), 1.0);
        assert_eq!(world.carbon_dioxide.get(), 0.0);
        assert_eq!(actor.effects.colony_photosynthesis, 0.0);
        assert_eq!(actor.state.temporary_argb(), ColorKind::DarkGreen.argb());
        assert_eq!(actor.state.temporary_color_frames, 10);
        assert_eq!(world.random_snapshot(), expected);
    }

    #[test]
    fn plant_update_respects_season_scaled_caches_and_unchanged_geometry() {
        let mut world = world();
        let mut actor = placed(
            &mut world,
            &[
                ColorKind::Forest,
                ColorKind::Leaf,
                ColorKind::Purple,
                ColorKind::Summer,
            ],
        );
        actor.body.growth_ratio = 1;
        actor.body.mass = 100.0;
        for index in actor.body.indices() { actor.body.set_physical_length(index, 1.0).unwrap(); }
        let bounds = actor.state.bounds.get();
        let rotated = actor.body.rotated_values().unwrap();
        actor.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![1.0, 2.0, 3.0, 4.0]));
        actor.effects.summer_inactivity = 0;
        actor.effects.lower_symmetric = 0;
        actor.effects.colony_photosynthesis = 0.0;
        actor.state.age = 1;
        actor.state.energy = 0.0;
        actor.set_display_color(ColorKind::Blond.color_ref());
        actor.state.temporary_color_frames = 1;
        world.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        world.carbon_dioxide.set(100.0);
        world.plants_update_effects(&mut actor).unwrap();
        assert_eq!(actor.effects.photosynthesis, 3.0);
        assert_eq!(actor.effects.forest_photo, 1.0);
        assert_eq!(actor.effects.leaf_photo, 2.0);
        assert_eq!(actor.effects.methanotrophy, 3.0);
        assert_eq!(actor.effects.maintenance, 99.01 / 5000.0);
        assert_eq!(actor.state.energy, 1.0);
        assert_eq!(world.carbon_dioxide.get(), 99.0);
        assert_eq!(
            actor.body.color_argb(3).unwrap(),
            ColorKind::Winter.argb().unwrap()
        );
        assert_eq!(actor.state.bounds.get(), bounds);
        assert_eq!(actor.body.rotated_values().unwrap(), rotated);
        assert_eq!(actor.state.temporary_argb(), ColorKind::DarkGreen.argb());
    }

    #[test]
    fn jade_restoration_uses_cached_photo_without_rebuilding_aggregates() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Forest]);
        actor.body.growth_ratio = 1;
        actor.body.set_color_argb(0, ColorKind::DarkGreen.argb().unwrap()).unwrap();
        actor.body.set_photo(0, 2.0).unwrap();
        actor.effects.photosynthesis = 10.0;
        actor.effects.leaf_photo = 0.0;
        actor.effects.forest_photo = 19.0;
        actor.effects.jade_factor = 1;
        actor.effects.lower_symmetric = 0;
        actor.state.energy = 0.0;
        world.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        world.carbon_dioxide.set(100.0);
        let before = actor.body.update_effects;
        let mut expected = world.random_snapshot();
        expected.next_bounded(1).unwrap();
        world.jade_extra_effects(&mut actor).unwrap();
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::Forest.argb().unwrap()
        );
        assert_eq!(actor.state.energy, 8.0);
        assert_eq!(actor.effects.photosynthesis, 10.0);
        assert_eq!(actor.effects.forest_photo, 19.0);
        assert_eq!(actor.body.photo_values().unwrap(), [2.0]);
        assert_eq!(actor.body.update_effects, before);
        assert_eq!(world.random_snapshot(), expected);
    }

    #[test]
    fn nonfinite_photo_does_not_spend_or_clear_colony_when_positive_test_fails() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Green]);
        actor.body.set_photo(0, f64::NAN).unwrap();
        actor.effects.colony_photosynthesis = 7.0;
        let energy = actor.state.energy;
        let carbon = world.carbon_dioxide.get();
        let random = world.random_snapshot();
        world.plants_update_effects(&mut actor).unwrap();
        assert!(actor.effects.photosynthesis.is_nan());
        assert_eq!(actor.effects.colony_photosynthesis, 7.0);
        assert_eq!(actor.state.energy, energy);
        assert_eq!(world.carbon_dioxide.get(), carbon);
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn positive_leaf_cache_keeps_its_branch_when_scaling_underflows() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Green, ColorKind::Leaf]);
        actor.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![1.0, f64::from_bits(1)]));
        actor.set_display_color(ColorKind::Blond.color_ref());
        actor.state.temporary_color_frames = 1;
        world.plants_update_effects(&mut actor).unwrap();
        assert_eq!(actor.effects.leaf_photo.to_bits(), 0);
        assert_eq!(actor.state.temporary_argb(), ColorKind::DarkGreen.argb());
        assert_eq!(actor.state.temporary_color_frames, 10);
    }

    #[test]
    fn positive_methane_cache_keeps_its_zero_amount_call_after_scaling() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Purple]);
        actor.body.set_photo(0, f64::from_bits(1)).unwrap();
        actor.state.age = 8;
        actor.state.energy = -0.0;
        world.oxygen.set(-0.0);
        world.methane.set(1.0);
        world.plants_update_effects(&mut actor).unwrap();
        assert_eq!(actor.effects.methanotrophy.to_bits(), 0);
        assert_eq!(actor.state.energy.to_bits(), 0);
        assert_eq!(world.oxygen.get().to_bits(), 0);
        assert_eq!(world.methane.get(), 1.0);
    }

    #[test]
    fn invalid_jade_bound_fails_without_drawing_or_finishing_atmospheric_work() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Jade]);
        actor.body.set_color_argb(0, ColorKind::DarkJade.argb().unwrap()).unwrap();
        actor.effects.jade_factor = 0;
        actor.effects.methanotrophy = 1.0;
        actor.state.age = 8;
        let energy = actor.state.energy;
        let random = world.random_snapshot();
        assert!(world.jade_extra_effects(&mut actor).is_err());
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(actor.state.energy, energy);
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::DarkJade.argb().unwrap()
        );
    }

    #[test]
    fn darkfire_restoration_keeps_unrelated_freeze_and_photo_state() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Magenta, ColorKind::Green]);
        actor.body.set_color_argb(1, ColorKind::Ice.argb().unwrap()).unwrap();
        actor.effects.healing = 999;
        actor.effects.is_injured = false;
        actor.effects.is_frozen = true;
        actor.effects.all_frozen = true;
        actor.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(vec![9.0, 8.0]));
        actor.effects.can_react = false;
        let random = world.random_snapshot();
        world.darkfire_restore_effects(&mut actor).unwrap();
        assert_eq!(actor.effects.healing, 120);
        assert!(actor.effects.is_injured && actor.effects.is_frozen && actor.effects.all_frozen);
        assert!(!actor.effects.can_react);
        assert_eq!(actor.body.photo_values().unwrap(), [9.0, 8.0]);
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn nonplant_update_uses_current_phenotype_and_retains_photo_aggregates() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Blue]);
        actor.body.mass = 100.0;
        actor.body.set_physical_length(0, 10.0).unwrap();
        actor.effects.photosynthesis = 19.0;
        actor.effects.methanotrophy = 23.0;
        actor.effects.colony_photosynthesis = 29.0;
        world.session.0.parameters.borrow_mut().lifecycle.segment_cost_divisor = 1;
        world.segments_update_effects(&mut actor).unwrap();
        assert_eq!(actor.effects.maintenance, 91.0);
        actor.body.set_color_argb(0, 0x120000ff).unwrap();
        world.segments_update_effects(&mut actor).unwrap();
        assert_eq!(actor.effects.maintenance, 100.0);
        assert_eq!(actor.effects.photosynthesis, 19.0);
        assert_eq!(actor.effects.methanotrophy, 23.0);
        assert_eq!(actor.effects.colony_photosynthesis, 29.0);
    }

    #[test]
    fn unconditional_healing_keeps_the_zero_threshold_draw() {
        let mut world = world();
        let mut actor = placed(&mut world, &[ColorKind::Green]);
        actor.body.set_color_argb(0, ColorKind::Broken.argb().unwrap()).unwrap();
        actor.effects.healing = 0;
        actor.state.energy = 10.0;
        world.oxygen.set(10.0);
        world.session.0.parameters.borrow_mut().lifecycle.healing = 1;
        let random = world.random_snapshot();
        world.segments_extra_effects(&mut actor).unwrap();
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::Broken.argb().unwrap()
        );
        let mut expected = random;
        expected.next_bounded(1).unwrap();
        world.segments_healing_effects(&mut actor).unwrap();
        assert_eq!(world.random_snapshot(), expected);
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::Green.argb().unwrap()
        );
        assert_eq!(actor.state.energy, 9.99);
        assert_eq!(actor.body.update_effects, 2);
    }

    #[test]
    fn methane_uses_associated_world_and_skips_unreached_null_access() {
        let mut caller = world();
        let target = World::empty_in_session(caller.config.get(), caller.session());
        let mut actor = placed(&mut caller, &[ColorKind::Purple]);
        caller.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        caller.methane.set(20.0);
        caller.oxygen.set(50.0);
        target.methane.set(2.0);
        target.oxygen.set(5.0);
        actor.state.world = target.association();
        actor.effects.methanotrophy = 1.0;
        actor.state.age = 8;
        actor.state.energy = 10.0;
        caller.frame_methanotrophy(&mut actor).unwrap();
        assert_eq!(actor.state.energy, 11.0);
        assert_eq!((target.methane.get(), target.oxygen.get()), (1.0, 6.0));
        assert_eq!((caller.methane.get(), caller.oxygen.get()), (20.0, 50.0));
        actor.state.world = Default::default();
        for (amount, age) in [(0.0, 8), (-1.0, 8), (f64::NAN, 8), (1.0, 7), (1.0, 9)] {
            actor.effects.methanotrophy = amount;
            actor.state.age = age;
            caller.frame_methanotrophy(&mut actor).unwrap();
        }
        actor.effects.methanotrophy = 1.0;
        actor.state.age = 8;
        assert_eq!(caller.frame_methanotrophy(&mut actor), Err(WorldError::MissingWorld));
        assert_eq!(actor.state.energy, 11.0);
    }

    #[test]
    fn photo_uses_associated_carbon_and_keeps_rng_growth_and_clear_order() {
        let mut caller = world();
        let target = World::empty_in_session(caller.config.get(), caller.session());
        let mut actor = placed(&mut caller, &[ColorKind::Green]);
        caller.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        caller.carbon_dioxide.set(100.0);
        caller.oxygen.set(50.0);
        target.carbon_dioxide.set(20.0);
        target.oxygen.set(5.0);
        actor.state.world = target.association();
        actor.effects.lower_symmetric = 0;
        actor.effects.colony_photosynthesis = 0.0;
        actor.state.energy = 10.0;
        caller.frame_photo(&mut actor, 4.0, 0.0).unwrap();
        assert_eq!(actor.state.energy, 14.0);
        assert_eq!((target.carbon_dioxide.get(), target.oxygen.get()), (16.0, 9.0));
        target.carbon_dioxide.set(0.0);
        actor.effects.lower_symmetric = 10;
        actor.effects.colony_photosynthesis = 3.0;
        actor.body.growth_ratio = 0;
        let mut expected = caller.random_snapshot();
        expected.next_bounded(10).unwrap();
        caller.frame_photo(&mut actor, 4.0, 1.0).unwrap();
        assert_eq!(caller.random_snapshot(), expected);
        assert_eq!(actor.effects.colony_photosynthesis, 0.0);
        assert_eq!(actor.state.energy, 14.0);
        assert_eq!((caller.carbon_dioxide.get(), caller.oxygen.get()), (100.0, 50.0));
        actor.state.world = Default::default();
        actor.effects.colony_photosynthesis = 3.0;
        assert_eq!(caller.frame_photo(&mut actor, 4.0, 1.0), Err(WorldError::MissingWorld));
        actor.effects.lower_symmetric = 0;
        assert_eq!(caller.frame_photo(&mut actor, 4.0, 1.0),
            Err(WorldError::Geometry(crate::GeometryError::GrowthIndex { index: -1 })));
        assert_eq!(actor.effects.colony_photosynthesis, 3.0);
        actor.body.growth_ratio = 1;
        actor.effects.colony_photosynthesis = 0.0;
        assert_eq!(caller.frame_photo(&mut actor, 0.0, 0.0), Err(WorldError::MissingWorld));
        assert_eq!(actor.state.energy, 14.0);
        assert_eq!(caller.random_snapshot(), expected);
    }

    #[test]
    fn leaf_and_jade_keep_earlier_failures_before_atmospheric_access() {
        let mut caller = world();
        let mut actor = placed(&mut caller, &[ColorKind::Green]);
        actor.state.world = Default::default();
        actor.effects.methanotrophy = 1.0;
        actor.state.age = 8;
        actor.state.temporary_color_frames = 1;
        actor.state.temporary_color = None;
        let energy = actor.state.energy;
        let random = caller.random_snapshot();
        assert_eq!(caller.leaf_extra_effects(&mut actor), Err(WorldError::MissingWorld));
        actor.effects.methanotrophy = 0.0;
        assert_eq!(caller.leaf_extra_effects(&mut actor),
            Err(WorldError::Geometry(crate::GeometryError::MissingTemporaryColor)));
        actor.body.set_color_argb(0, ColorKind::DarkGreen.argb().unwrap()).unwrap();
        actor.genome = None;
        actor.effects.methanotrophy = 1.0;
        assert_eq!(caller.jade_extra_effects(&mut actor),
            Err(WorldError::Geometry(crate::GeometryError::MissingGenome)));
        assert_eq!(actor.state.energy, energy);
        assert_eq!(actor.state.temporary_color_frames, 1);
        assert_eq!(caller.random_snapshot(), random);
    }

    #[test]
    fn plant_recompute_routes_gains_and_keeps_zero_amount_null_prefix() {
        let mut caller = world();
        let target = World::empty_in_session(caller.config.get(), caller.session());
        let mut actor = placed(&mut caller, &[ColorKind::Green, ColorKind::Purple]);
        caller.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
        caller.carbon_dioxide.set(100.0);
        caller.methane.set(20.0);
        caller.oxygen.set(50.0);
        target.carbon_dioxide.set(20.0);
        target.methane.set(3.0);
        target.oxygen.set(0.0);
        actor.state.world = target.association();
        actor.state.age = 8;
        actor.state.energy = 0.0;
        actor.body.growth_ratio = 1;
        actor.body.set_photo(0, 2.0).unwrap();
        actor.body.set_photo(1, 1.0).unwrap();
        actor.effects.lower_symmetric = 0;
        caller.plants_update_effects(&mut actor).unwrap();
        assert_eq!(actor.state.energy, 3.0);
        assert_eq!((target.carbon_dioxide.get(), target.methane.get(), target.oxygen.get()), (18.0, 2.0, 3.0));
        assert_eq!((caller.carbon_dioxide.get(), caller.methane.get(), caller.oxygen.get()), (100.0, 20.0, 50.0));
        let mut underflow = placed(&mut caller, &[ColorKind::Purple]);
        underflow.state.world = Default::default();
        underflow.state.age = 8;
        underflow.state.energy = 4.0;
        underflow.body.growth_ratio = 16;
        underflow.body.mass = 0.0;
        underflow.body.set_physical_length(0, 0.0).unwrap();
        underflow.body.set_photo(0, f64::from_bits(1)).unwrap();
        underflow.effects.maintenance = -1.0;
        underflow.effects.photosynthesis = 77.0;
        assert_eq!(caller.plants_update_effects(&mut underflow), Err(WorldError::MissingWorld));
        assert_eq!(underflow.effects.maintenance, 0.0);
        assert_eq!(underflow.effects.methanotrophy.to_bits(), 0);
        assert_eq!(underflow.effects.photosynthesis, 77.0);
        assert_eq!(underflow.state.energy, 4.0);
    }
    #[test]
    fn seasonal_time_uses_foreign_owner_for_summer_winter_and_both_jade_paths() {
        for (current, extra, active_photo, inactive_photo, active_energy, inactive_energy) in [
            (ColorKind::Summer, false, 2.0, 0.0, 2.0, 0.0),
            (ColorKind::Winter, false, 2.0, 0.0, 2.0, 0.0),
            (ColorKind::DarkGreen, true, 7.0, 7.0, 5.0, 7.0),
            (ColorKind::DarkGreen, false, 2.0, 0.0, 0.0, 0.0),
        ] {
            for (caller_time, owner_time, active) in [(0, 1, true), (1, 0, false)] {
                let mut caller = world();
                let owner = World::empty_in_session(caller.config.get(), caller.session());
                let mut actor = clock_actor(&mut caller, current, ColorKind::Summer);
                actor.state.world = owner.association();
                caller.session.0.parameters.borrow_mut().atmosphere.drain_divisor = 1;
                caller.carbon_dioxide.set(100.0);
                owner.carbon_dioxide.set(50.0);
                caller.frame.set(9);
                owner.frame.set(19);
                set_clock_time(&caller, caller_time);
                set_clock_time(&owner, owner_time);
                let caller_statistics = caller.statistics_ref().borrow()
                    .copy_for_world(&mut Default::default());
                let owner_statistics = owner.statistics_ref().borrow()
                    .copy_for_world(&mut Default::default());
                let mut expected_random = caller.random_snapshot();
                if current == ColorKind::DarkGreen {
                    expected_random.next_bounded(1).unwrap();
                }
                if extra {
                    caller.jade_extra_effects(&mut actor).unwrap();
                } else {
                    caller.plants_update_effects(&mut actor).unwrap();
                }
                let expected_energy = if active { active_energy } else { inactive_energy };
                assert_eq!(actor.body.color_argb(0).unwrap(), if active {
                    ColorKind::Summer.argb().unwrap()
                } else {
                    ColorKind::Winter.argb().unwrap()
                });
                assert_eq!(actor.effects.photosynthesis,
                    if active { active_photo } else { inactive_photo });
                assert_eq!(actor.state.energy, expected_energy);
                assert_eq!(owner.carbon_dioxide.get(), 50.0 - expected_energy);
                assert_eq!(caller.carbon_dioxide.get(), 100.0);
                assert_eq!(actor.effects.forest_photo, if extra { 19.0 } else { 0.0 });
                assert_eq!((actor.effects.leaf_photo, actor.effects.methanotrophy), (0.0, 0.0));
                assert!(!actor.effects.use_extra_effects);
                assert_eq!((caller.frame(), owner.frame()), (9, 19));
                assert_eq!(*caller.statistics_ref().borrow(), caller_statistics);
                assert_eq!(*owner.statistics_ref().borrow(), owner_statistics);
                assert!(caller.session().same_owner(&owner.session()));
                assert_eq!(caller.random_snapshot(), expected_random);
                assert_eq!(owner.random_snapshot(), expected_random);
            }
        }
    }

    #[test]
    fn seasonal_time_reuses_current_association_with_actor_peer_and_collections_held() {
        let mut caller = world();
        let mut owner = World::empty_in_session(caller.config.get(), caller.session());
        let actor_ref = Rc::new(RefCell::new(clock_actor(
            &mut caller, ColorKind::Winter, ColorKind::Summer,
        )));
        let peer_ref = Rc::new(RefCell::new(placed(&mut owner, &[ColorKind::White])));
        caller.organisms.borrow_mut().push(Rc::clone(&actor_ref));
        owner.organisms.borrow_mut().push(Rc::clone(&peer_ref));
        set_clock_time(&caller, 0);
        set_clock_time(&owner, 1);
        let random = caller.random_snapshot();
        {
            let mut actor = actor_ref.borrow_mut();
            let peer = peer_ref.borrow_mut();
            let _caller_members = caller.organisms.borrow_mut();
            let _owner_members = owner.organisms.borrow_mut();
            actor.body.set_photo(0, 0.0).unwrap();
            actor.state.world = owner.association();
            caller.plants_update_effects(&mut actor).unwrap();
            assert_eq!(actor.body.color_argb(0).unwrap(), ColorKind::Summer.argb().unwrap());
            actor.state.world = caller.association();
            owner.plants_update_effects(&mut actor).unwrap();
            assert_eq!(actor.body.color_argb(0).unwrap(), ColorKind::Winter.argb().unwrap());
            actor.state.world = owner.association();
            caller.plants_update_effects(&mut actor).unwrap();
            assert_eq!(actor.body.color_argb(0).unwrap(), ColorKind::Summer.argb().unwrap());
            assert_eq!(peer.body.color_argb(0).unwrap(), ColorKind::White.argb().unwrap());
        }
        assert!(caller.statistics_ref().try_borrow_mut().is_ok());
        assert!(owner.statistics_ref().try_borrow_mut().is_ok());
        assert_eq!((caller.time(), owner.time()), (0, 1));
        assert_eq!(caller.random_snapshot(), random);
    }

    #[test]
    fn seasonal_time_keeps_null_and_expired_gates_and_reads_at_negative_inactivity() {
        for expired in [false, true] {
            let mut caller = world();
            let association = if expired {
                let owner = World::empty_in_session(caller.config.get(), caller.session());
                owner.association()
            } else {
                Default::default()
            };
            let mut white = clock_actor(&mut caller, ColorKind::White, ColorKind::White);
            white.state.world = association.clone();
            white.body.set_photo(0, 0.0).unwrap();
            let random = caller.random_snapshot();
            caller.plants_update_effects(&mut white).unwrap();
            assert_eq!(caller.random_snapshot(), random);
            for extra in [false, true] {
                let mut green = clock_actor(&mut caller, ColorKind::DarkGreen, ColorKind::Green);
                green.state.world = association.clone();
                green.body.set_photo(0, 0.0).unwrap();
                let mut expected = caller.random_snapshot();
                expected.next_bounded(1).unwrap();
                if extra {
                    caller.jade_frame(&mut green, 0, false, &mut 0.0, &mut 0.0).unwrap();
                } else {
                    caller.plants_update_effects(&mut green).unwrap();
                }
                assert_eq!(green.body.color_argb(0).unwrap(), ColorKind::Green.argb().unwrap());
                assert_eq!(caller.random_snapshot(), expected);
            }
            for (current, extra) in [
                (ColorKind::Summer, false), (ColorKind::Winter, false),
                (ColorKind::DarkGreen, false), (ColorKind::DarkGreen, true),
            ] {
                let mut actor = clock_actor(&mut caller, current, ColorKind::Summer);
                actor.state.world = association.clone();
                actor.effects.summer_inactivity = -1;
                actor.body.arrays.photo = None;
                let random = caller.random_snapshot();
                let mut photo = 11.0;
                let mut special = 13.0;
                let result = if extra {
                    caller.jade_frame(&mut actor, 0, false, &mut photo, &mut special)
                } else {
                    caller.plants_update_effects(&mut actor)
                };
                assert_eq!(result, Err(if expired {
                    WorldError::ExpiredWorld
                } else {
                    WorldError::MissingWorld
                }));
                assert_eq!(actor.body.color_argb(0).unwrap(), current.argb().unwrap());
                assert_eq!((photo, special), (11.0, 13.0));
                assert_eq!(actor.effects.forest_photo, if extra { 19.0 } else { 0.0 });
                assert_eq!(caller.random_snapshot(), random);
            }
        }
    }

    #[test]
    fn seasonal_gene_world_photo_and_rng_failures_keep_source_precedence() {
        let mut caller = world();
        set_clock_time(&caller, 1);
        for update in [false, true] {
            for fault in 0..3 {
                let mut actor = clock_actor(&mut caller, ColorKind::DarkGreen, ColorKind::Summer);
                actor.state.world = Default::default();
                let mut index = 0;
                let error = match fault {
                    0 => {
                        actor.genome = None;
                        WorldError::Geometry(crate::GeometryError::MissingGenome)
                    }
                    1 => {
                        index = i32::MAX as usize + 1;
                        WorldError::Geometry(crate::GeometryError::Genetics(
                            crate::GeneticsError::GeneIndexDomain { index },
                        ))
                    }
                    _ => {
                        actor.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap()
                            .borrow_mut().set_color(None);
                        WorldError::Geometry(crate::GeometryError::MissingColor { gene: 0 })
                    }
                };
                let random = caller.random_snapshot();
                let (mut photo, mut special) = (5.0, 7.0);
                assert_eq!(caller.jade_frame(&mut actor, index, update, &mut photo, &mut special),
                    Err(error));
                assert_eq!((photo, special), (5.0, 7.0));
                assert_eq!(caller.random_snapshot(), random);
            }
            let mut actor = clock_actor(&mut caller, ColorKind::DarkGreen, ColorKind::Summer);
            actor.state.world = Default::default();
            actor.effects.jade_factor = 0;
            let random = caller.random_snapshot();
            let (mut photo, mut special) = (5.0, 7.0);
            assert_eq!(caller.jade_frame(&mut actor, 0, update, &mut photo, &mut special),
                Err(WorldError::MissingWorld));
            assert_eq!((photo, special), (5.0, 7.0));
            actor.state.world = caller.association();
            assert_eq!(caller.jade_frame(&mut actor, 0, update, &mut photo, &mut special),
                Err(WorldError::InvalidParameters("Random bounds must be positive Java integers".into())));
            assert_eq!((photo, special), (if update { 7.0 } else { 5.0 }, 9.0));
            assert_eq!(actor.body.color_argb(0).unwrap(), ColorKind::DarkGreen.argb().unwrap());
            assert_eq!(caller.random_snapshot(), random);
        }
        for current in [ColorKind::Summer, ColorKind::Winter] {
            let mut actor = clock_actor(&mut caller, current, ColorKind::Summer);
            actor.body.arrays.photo = None;
            let random = caller.random_snapshot();
            assert_eq!(caller.plants_update_effects(&mut actor), Err(WorldError::Geometry(
                crate::GeometryError::MissingPrimitiveArray { field: "_mphoto" },
            )));
            assert_eq!(actor.body.color_argb(0).unwrap(), ColorKind::Summer.argb().unwrap());
            assert_eq!(caller.random_snapshot(), random);
        }
    }

    #[test]
    fn seasonal_clock_failure_keeps_reverse_order_rng_color_and_cache_prefixes() {
        for update in [false, true] {
            let mut caller = world();
            let mut actor = placed(&mut caller, &[ColorKind::Summer, ColorKind::Jade]);
            actor.body.set_color(0, ColorKind::DarkGreen.color_ref()).unwrap();
            actor.body.set_color(1, ColorKind::DarkJade.color_ref()).unwrap();
            actor.state.world = Default::default();
            actor.effects.summer_inactivity = 0;
            actor.effects.jade_factor = 1;
            actor.effects.use_extra_effects = false;
            actor.effects.forest_photo = 19.0;
            actor.effects.leaf_photo = 23.0;
            actor.effects.methanotrophy = 29.0;
            actor.effects.photosynthesis = 31.0;
            actor.effects.maintenance = 37.0;
            let energy = actor.state.energy;
            let mut expected = caller.random_snapshot();
            expected.next_bounded(1).unwrap();
            let result = if update {
                caller.plants_update_effects(&mut actor)
            } else {
                caller.jade_extra_effects(&mut actor)
            };
            assert_eq!(result, Err(WorldError::MissingWorld));
            assert_eq!(actor.body.color_argb(1).unwrap(), ColorKind::Jade.argb().unwrap());
            assert_eq!(actor.body.color_argb(0).unwrap(), ColorKind::DarkGreen.argb().unwrap());
            assert_eq!((actor.effects.forest_photo, actor.effects.leaf_photo,
                actor.effects.methanotrophy), if update { (0.0, 0.0, 0.0) } else { (19.0, 23.0, 29.0) });
            assert_eq!((actor.effects.photosynthesis, actor.effects.maintenance), (31.0, 37.0));
            assert_eq!(actor.state.energy, energy);
            assert_eq!(caller.random_snapshot(), expected);
        }
    }

    #[test]
    fn seasonal_clock_boundaries_keep_same_owner_and_signed_remainders() {
        for (time, inactivity, expected_color) in [
            (0, 0, ColorKind::Winter), (1, 0, ColorKind::Summer),
            (2, 0, ColorKind::Summer), (3, 0, ColorKind::Winter),
            (-1, -1, ColorKind::Winter), (-2, -2, ColorKind::Winter),
            (-3, 0, ColorKind::Winter), (-1, 2, ColorKind::Summer),
            (-2, 1, ColorKind::Summer),
        ] {
            for (current, jade_update) in [
                (ColorKind::Summer, None), (ColorKind::Winter, None),
                (ColorKind::DarkGreen, Some(false)), (ColorKind::DarkGreen, Some(true)),
            ] {
                let mut caller = world();
                let mut actor = clock_actor(&mut caller, current, ColorKind::Summer);
                actor.effects.summer_inactivity = inactivity;
                actor.body.set_photo(0, 0.0).unwrap();
                set_clock_time(&caller, time);
                let mut expected = caller.random_snapshot();
                if let Some(update) = jade_update {
                    expected.next_bounded(1).unwrap();
                    caller.jade_frame(&mut actor, 0, update, &mut 0.0, &mut 0.0).unwrap();
                } else {
                    caller.plants_update_effects(&mut actor).unwrap();
                }
                assert_eq!(actor.body.color_argb(0).unwrap(), expected_color.argb().unwrap());
                assert_eq!(caller.time(), time);
                assert_eq!(caller.random_snapshot(), expected);
            }
        }
    }

}
