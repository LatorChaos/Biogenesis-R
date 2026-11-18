// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::geometry::{Body, java_round};
use crate::{ColorKind, Genome, GeometryError};

#[derive(Clone, Debug, PartialEq)]
pub struct EffectsParams {
    pub max_age: i32,
    pub age_divisor: i32,
    pub gold_divisor: f64,
    pub green_energy_divisor: i32,
    pub dark_jade_delay: i32,
    pub lavender_shield: i32,
    pub indigo_divisor: f64,
    pub green: f64,
    pub forest: f64,
    pub spring: f64,
    pub summer: f64,
    pub leaf: f64,
    pub lime: f64,
    pub bark: f64,
    pub jade: f64,
    pub grass: f64,
    pub purple: f64,
    pub c4: f64,
    pub symbiont: f64,
}

impl Default for EffectsParams {
    fn default() -> Self {
        Self {
            max_age: 6,
            age_divisor: 4,
            gold_divisor: 2.0,
            green_energy_divisor: 1000,
            dark_jade_delay: 4,
            lavender_shield: 10_000,
            indigo_divisor: 10.0,
            green: 1.1,
            forest: 1.0,
            spring: 1.0,
            summer: 1.65,
            leaf: 1.15,
            lime: 1.25,
            bark: 0.7,
            jade: 1.0,
            grass: 1.0,
            purple: 4.0,
            c4: 0.55,
            symbiont: 0.6,
        }
    }
}

impl EffectsParams {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.age_divisor <= 0
            || self.green_energy_divisor <= 0
            || !(self.gold_divisor > 0.0)
            || !(self.indigo_divisor > 0.0)
        {
            return Err("Effect divisors must be positive");
        }
        if [
            self.green,
            self.forest,
            self.spring,
            self.summer,
            self.leaf,
            self.lime,
            self.bark,
            self.jade,
            self.grass,
            self.purple,
            self.c4,
            self.symbiont,
        ]
        .iter()
        .any(|value| value.is_nan() || *value == f64::NEG_INFINITY)
        {
            return Err("Effect energy parameters must not be NaN or negative infinity");
        }
        Ok(())
    }
}

/// Color aggregates for one organism; cached primitive arrays belong to Body
#[derive(Clone, Debug, PartialEq)]
pub struct EffectsState {
    pub indigo: i32,
    pub antiviral: i32,
    pub lavender: i32,
    pub create_lavender: i32,
    pub healing: i32,
    pub spore_time: i32,
    pub spore_version: i32,
    pub sky_version: i32,
    pub fallow_version: i32,
    pub plague_version: i32,
    pub summer_inactivity: i32,
    pub yellow_count: i32,
    pub reproduce_late: i32,
    pub gold: i32,
    pub maintenance: f64,
    pub methanotrophy: f64,
    pub photosynthesis: f64,
    pub colony_photosynthesis: f64,
    pub forest_photo: f64,
    pub leaf_photo: f64,
    pub spin: i32,
    pub jade_factor: i32,
    pub friend_length: i32,
    pub friend_theta: i32,
    pub sibling_battle: bool,
    pub generation_battle: bool,
    pub clockwise: bool,
    pub altruist: bool,
    pub lower_symmetric: i32,
    pub is_plant: bool,
    pub is_consumer: bool,
    pub is_fungus: bool,
    pub is_killer: bool,
    pub is_gray: bool,
    pub is_spike: bool,
    pub can_move: bool,
    pub can_react: bool,
    pub is_frozen: bool,
    pub all_frozen: bool,
    pub is_injured: bool,
    pub is_reproductive: bool,
    pub is_infectious: bool,
    pub is_coral: bool,
    pub is_auburn: bool,
    pub is_blond: bool,
    pub is_silver: bool,
    pub is_enhanced: bool,
    pub is_lime: bool,
    pub is_only_c4: bool,
    pub transfers_energy: bool,
    pub crowded: bool,
    pub can_dodge: bool,
    pub dodge: bool,
    pub has_dodged: bool,
    pub segment_pre_touch_effects: i32,
    pub use_frame_movement: bool,
    pub use_extra_effects: bool,
    pub use_pre_touch_effects: bool,
    pub use_friend_effects: i32,
}

impl Default for EffectsState {
    fn default() -> Self {
        Self {
            indigo: 0,
            antiviral: 0,
            lavender: 0,
            create_lavender: 0,
            healing: 0,
            spore_time: 0,
            spore_version: 0,
            sky_version: 0,
            fallow_version: 0,
            plague_version: 0,
            summer_inactivity: -1,
            yellow_count: 0,
            reproduce_late: 0,
            gold: 0,
            maintenance: 0.0,
            methanotrophy: 0.0,
            photosynthesis: 0.0,
            colony_photosynthesis: 0.0,
            forest_photo: 0.0,
            leaf_photo: 0.0,
            spin: 0,
            jade_factor: 0,
            friend_length: -1,
            friend_theta: -1,
            sibling_battle: false,
            generation_battle: false,
            clockwise: false,
            altruist: false,
            lower_symmetric: 0,
            is_plant: false,
            is_consumer: false,
            is_fungus: false,
            is_killer: false,
            is_gray: false,
            is_spike: false,
            can_move: false,
            can_react: false,
            is_frozen: false,
            all_frozen: false,
            is_injured: false,
            is_reproductive: false,
            is_infectious: false,
            is_coral: false,
            is_auburn: false,
            is_blond: false,
            is_silver: false,
            is_enhanced: false,
            is_lime: false,
            is_only_c4: false,
            transfers_energy: false,
            crowded: false,
            can_dodge: false,
            dodge: false,
            has_dodged: true,
            segment_pre_touch_effects: 0,
            use_frame_movement: false,
            use_extra_effects: false,
            use_pre_touch_effects: false,
            use_friend_effects: 0,
        }
    }
}

/// Lifecycle values changed by construction and segment aggregation
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReproductionState {
    pub early_energy: i32,
    pub energy: i32,
    pub infected_energy: i32,
    pub countdown: i32,
    pub maximum_countdown: i32,
    pub maximum_age: i32,
}

impl EffectsState {
    /// Preserve Organism.segmentsCreateEffects ordering, including state retained between calls
    pub(crate) fn apply(
        &mut self,
        genome: Option<&Genome>,
        body: &mut Body,
        age: i32,
        reproduction: &mut ReproductionState,
        params: &EffectsParams,
    ) -> Result<(), GeometryError> {
        use ColorKind::*;
        let segments = body.segment_count;
        self.spin = 0;
        self.healing = 0;
        self.antiviral = 0;
        self.create_lavender = 0;
        self.yellow_count = 0;
        self.indigo = 0;
        reproduction.early_energy = 0;
        if self.reproduce_late > 0 {
            self.reproduce_late = 0;
        }
        self.gold = 0;
        body.eyes.enabled = false;
        let mut enhanced_consumer = false;
        let mut is_pink = false;
        let mut protective = 0;
        let mut fertility = 0.0;
        body.update_effects = 2;
        let genome = genome.ok_or(GeometryError::MissingGenome)?;
        let symmetry = genome.symmetry();
        let modifiers = genome.modifiers();
        self.sibling_battle |= modifiers.sibling_battle;
        self.generation_battle |= modifiers.generation_battle;
        self.altruist |= modifiers.altruist;
        let count = genome.gene_count()?;
        let photo_factor = if symmetry == 8 { 1961.0 } else { 1961.329 }
            + if count == 1 {
                2200.0
            } else {
                f64::from(java_round(6000.0 / (count as f64 + 2.0)))
            }
            + f64::from(java_round(6315.0 / f64::from(symmetry)));
        if symmetry != 8 {
            self.lower_symmetric = if symmetry >= 6 {
                870
            } else if symmetry >= 4 {
                940
            } else {
                700 + 1200 / (symmetry + 1)
            };
        }
        let multiplier = (photo_factor * 0.0006) / f64::from(params.green_energy_divisor);

        for index in body.reverse_indices() {
            let gene_index = || -> Result<usize, GeometryError> {
                if count == 0 { return Err(GeometryError::EmptyGenome); }
                Ok(index % count)
            };
            let length = || -> Result<f64, GeometryError> { Ok(genome.gene_ref(gene_index()?)?.borrow().length()) };
            match ColorKind::from_argb(body.color_argb(index)?) {
                Green | Grass | Bark | Summer | Winter | Leaf | Forest | Lime | Jade | DarkJade
                | Spring => {
                    let energy = match ColorKind::from_argb(body.color_argb(index)?) {
                        Green => params.green,
                        Grass => params.grass,
                        Bark => params.bark,
                        Summer | Winter => params.summer,
                        Leaf => params.leaf,
                        Forest => params.forest,
                        Lime => params.lime,
                        Jade | DarkJade => params.jade,
                        Spring => params.spring,
                        _ => unreachable!(),
                    };
                    body.set_photo(index, energy * multiplier * length()?)?;
                    self.is_plant = true;
                    match ColorKind::from_argb(body.color_argb(index)?) {
                        Summer | Winter if self.summer_inactivity == -1 => {
                            self.summer_inactivity = if genome.activity() == 0 {
                                0
                            } else if genome.activity() == 1 {
                                1
                            } else {
                                2
                            };
                        }
                        Forest => {
                            self.forest_photo = 1.0;
                            if self.segment_pre_touch_effects == 0 {
                                self.segment_pre_touch_effects = 1;
                            }
                            self.use_pre_touch_effects = true;
                        }
                        Lime => self.is_lime = true,
                        Jade | DarkJade => self.jade_factor = 2,
                        Spring => {
                            self.spin = self.spin.wrapping_add((10.0 * length()?) as i32);
                            self.clockwise |= modifiers.clockwise;
                        }
                        _ => {}
                    }
                }
                C4 => {
                    body.set_photo(index, params.c4 * multiplier * (10.0 + length()?))?;
                    self.is_only_c4 = true;
                }
                DarkGreen => {
                    if genome.gene_ref(gene_index()?)?.borrow().color() == Jade.argb() {
                        self.jade_factor = 2;
                    }
                    if genome.gene_ref(gene_index()?)?.borrow().color() == Spring.argb() {
                        self.spin = self.spin.wrapping_add((10.0 * length()?) as i32);
                    }
                }
                Purple => {
                    if age == 0 {
                        body.set_photo(index, params.purple * multiplier * length()?)?;
                    }
                    self.methanotrophy = 1.0;
                }
                Teal => {
                    self.can_move = true;
                    self.can_react = true;
                    if !modifiers.passive {
                        self.use_frame_movement = true;
                        body.set_photo(index, -4.0)?;
                    }
                }
                Cyan => {
                    self.can_move = true;
                    self.use_frame_movement = true;
                    body.set_photo(index, -3.0)?;
                }
                Red | Fire | Orange | Maroon | Cream => {
                    self.is_consumer = true;
                    body.set_photo(index, -1.0)?;
                }
                Pink => {
                    is_pink = true;
                    self.is_fungus = true;
                    self.is_consumer |= modifiers.modifies_pink;
                    body.set_photo(index, -1.0)?;
                }
                Auburn => {
                    self.is_auburn = true;
                    self.is_reproductive = true;
                }
                Yellow => {
                    self.is_reproductive = true;
                    self.yellow_count = self.yellow_count.wrapping_add(1);
                    // Java's integer quotients 5/6 and 2/3 are zero
                    if symmetry != 3 {
                        fertility += length()?;
                    }
                }
                Indigo | Mint | Magenta => {
                    let amount = if symmetry != 3 {
                        (10.0 * length()?) as i32
                    } else if genome.mirror() == 0 {
                        (8.0 * length()?) as i32
                    } else {
                        (6.0 * length()?) as i32
                    };
                    match ColorKind::from_argb(body.color_argb(index)?) {
                        Indigo => {
                            self.is_reproductive = true;
                            self.indigo = self.indigo.wrapping_add(amount);
                        }
                        Mint => {
                            self.antiviral = self.antiviral.wrapping_add(amount);
                            body.set_photo(index, -2.0)?;
                            self.use_friend_effects = 2;
                        }
                        Magenta => {
                            self.healing = self.healing.wrapping_add(amount);
                            body.set_photo(index, -2.0)?;
                            self.use_friend_effects = 2;
                        }
                        _ => unreachable!(),
                    }
                }
                Silver => {
                    enhanced_consumer = true;
                    self.is_silver = true;
                    body.set_photo(index, -1.0)?;
                }
                White => {
                    self.is_infectious = true;
                    body.set_photo(index, -2.0)?;
                }
                Plague => {
                    if self.plague_version == 0 {
                        self.plague_version = if modifiers.plague { 2 } else { 1 };
                    }
                    body.set_photo(index, -2.0)?;
                }
                Coral => {
                    self.is_coral = true;
                    body.set_photo(index, -1.0)?;
                }
                Fallow => {
                    if protective == 0 {
                        protective = 1;
                    }
                    if self.fallow_version == 0 {
                        self.fallow_version = match modifiers.fallow {
                            1 => 1,
                            2 => 2,
                            3 => 3,
                            _ => 4,
                        };
                    }
                    body.set_photo(index, -2.0)?;
                }
                DarkGray => {
                    self.is_enhanced = true;
                    body.set_photo(index, 1.0)?;
                }
                Eye => body.eyes.enabled = true,
                Gold => self.gold = self.gold.wrapping_add((10.0 * length()?) as i32),
                Flower => {
                    self.reproduce_late = self.reproduce_late.max(0).wrapping_add((10.0 * length()?) as i32);
                }
                Blond => {
                    self.is_blond = true;
                    reproduction.early_energy = reproduction
                        .early_energy
                        .wrapping_add(30_i32.wrapping_add((10.0 * length()?) as i32));
                }
                Spore => {
                    if modifiers.spore <= 6 {
                        self.is_reproductive = true;
                        if modifiers.spore <= 2 {
                            reproduction.early_energy = reproduction
                                .early_energy
                                .wrapping_add(20_i32.wrapping_mul(count as i32));
                        }
                    }
                    if age == 0 {
                        self.spore_time = self.spore_time.wrapping_add((10.0 * length()?) as i32);
                    }
                }
                Lavender => {
                    self.create_lavender = self.create_lavender.wrapping_add((10.0 * length()?) as i32);
                    if self.altruist {
                        body.set_photo(index, -2.0)?;
                    }
                    self.use_friend_effects = 2;
                }
                Rose => {
                    if !self.transfers_energy {
                        self.transfers_energy = true;
                        self.friend_length = length()? as i32;
                        self.friend_theta = genome.gene_ref(gene_index()?)?.borrow().theta() as i32;
                    }
                    if self.altruist {
                        body.set_photo(index, -2.0)?;
                    }
                    self.use_friend_effects = self.use_friend_effects.max(1);
                }
                Violet => body.set_photo(index, -2.0)?,
                Ochre => {
                    if protective == 0 {
                        protective = 1;
                    }
                    body.set_photo(index, -1.0)?;
                }
                Olive | DarkOlive => {
                    protective = 2;
                    body.set_photo(index, -2.0)?;
                }
                Blue => {
                    if protective == 0 {
                        protective = 1;
                    }
                }
                Sky | DeepSky => {
                    if protective == 0 {
                        protective = 1;
                    }
                    if self.sky_version <= 0 {
                        self.sky_version = if modifiers.modifies_sky { 2 } else { 1 };
                    }
                    body.set_photo(index, -2.0)?;
                }
                Gray => {
                    self.is_gray = true;
                    self.is_killer = true;
                    body.set_photo(index, -2.0)?;
                }
                Lilac | DarkLilac => {
                    self.is_killer = true;
                    body.set_photo(index, -1.0)?;
                }
                Spike | SpikePoint => {
                    enhanced_consumer = true;
                    self.is_spike = true;
                    self.is_killer = true;
                    self.use_pre_touch_effects = true;
                    body.set_photo(index, if ColorKind::from_argb(body.color_argb(index)?) == Spike {
                        0.0
                    } else {
                        -1.0
                    })?;
                }
                Dark => {
                    protective = 2;
                    self.segment_pre_touch_effects = 2;
                    self.use_pre_touch_effects = true;
                    body.set_photo(index, 0.0)?;
                }
                _ => {}
            }
        }
        if enhanced_consumer {
            self.is_consumer |= self.is_enhanced;
            if self.is_silver {
                self.is_consumer |= is_pink;
                self.is_infectious |= self.is_coral;
                self.is_fungus |=
                    self.is_killer || (self.plague_version > 0 && !self.is_infectious);
            }
        }
        if self.is_only_c4 {
            if !self.is_plant
                && self.methanotrophy == 0.0
                && !self.is_consumer
                && !self.is_fungus
                && !self.is_killer
                && self.sky_version == 0
                && !self.is_infectious
                && !self.is_coral
                && self.plague_version == 0
                && protective == 0
            {
                self.can_dodge = true;
                if self.spore_time == 0 || modifiers.spore <= 6 {
                    for index in body.reverse_indices() {
                        if ColorKind::from_argb(body.color_argb(index)?) == C4 {
                            body.set_photo(index, params.c4
                                * multiplier
                                * (11.0 + genome.gene_for_segment(index)?.borrow().length()))?;
                        }
                    }
                }
                if self.indigo > 0 && self.jade_factor == 0 {
                    self.jade_factor = -1;
                }
            } else {
                self.is_only_c4 = false;
            }
            self.is_plant = true;
        }
        if self.can_react {
            if !self.is_consumer
                && !self.is_fungus
                && !self.is_killer
                && self.sky_version == 0
                && protective <= 1
                && ((!self.is_infectious && !self.is_coral && self.plague_version == 0)
                    || (!self.is_plant && self.methanotrophy == 0.0 && modifiers.passive))
            {
                self.can_dodge = true;
            } else {
                self.can_dodge = false;
                self.dodge = false;
            }
        }
        if self.jade_factor > 1 {
            self.jade_factor =
                params
                    .dark_jade_delay
                    .wrapping_mul(if symmetry == 1 { 2 } else { symmetry });
        }
        if self.methanotrophy > 0.0 {
            if self.is_enhanced && !self.is_killer {
                self.segment_pre_touch_effects = 2;
                self.use_pre_touch_effects = true;
            }
            if !self.is_plant {
                if self.jade_factor == 0 {
                    self.jade_factor = -1;
                }
            } else if age == 0 {
                for index in body.reverse_indices() {
                    if ColorKind::from_argb(body.color_argb(index)?) == Purple {
                        body.set_photo(index, 0.55 * params.purple * multiplier * genome.gene_for_segment(index)?.borrow().length())?;
                    }
                }
            }
            self.is_plant = true;
        }
        if self.transfers_energy && !self.is_plant {
            if !self.is_consumer && !self.is_fungus {
                if self.segment_pre_touch_effects == 0 {
                    self.segment_pre_touch_effects = 1;
                }
                self.use_pre_touch_effects = true;
                if self.friend_length > 0 {
                    self.colony_photosynthesis =
                        params.symbiont * f64::from(self.friend_length.wrapping_add(9));
                    self.friend_length = self.friend_length.wrapping_neg();
                }
            } else {
                self.colony_photosynthesis = 0.0;
            }
        }
        if self.create_lavender > 0 {
            self.create_lavender /= symmetry;
            if age == 0 {
                self.lavender = params.lavender_shield;
            }
        }
        if self.spore_time > 0 {
            if age == 0 {
                if modifiers.spore <= 6 {
                    self.spore_version = modifiers.spore;
                    self.spore_time = (self.spore_time / symmetry / 10).wrapping_add(2);
                    if self.create_lavender > 0 {
                        self.lavender = 200;
                    }
                    if !self.is_plant
                        && !self.is_consumer
                        && !self.is_fungus
                        && self.spore_version <= 2
                    {
                        reproduction.countdown = reproduction.maximum_countdown;
                    }
                } else {
                    self.spore_version = if modifiers.spore <= 9 { -2 } else { -3 };
                    reproduction.maximum_countdown = reproduction.maximum_countdown.wrapping_sub(6);
                }
            } else if age == 1
                && !self.is_plant
                && !self.is_consumer
                && !self.is_fungus
                && self.spore_version >= 5
            {
                reproduction.countdown = reproduction.maximum_countdown;
                if self.spore_version == 5 {
                    self.is_fungus = true;
                }
            }
        }
        if self.yellow_count > 0 {
            self.yellow_count =
                (self.yellow_count / symmetry).wrapping_add((fertility / 23.0) as i32);
            if self.is_infectious && !self.is_consumer && !self.is_plant && !self.is_fungus {
                for index in body.reverse_indices() {
                    if ColorKind::from_argb(body.color_argb(index)?) == Yellow {
                        body.set_photo(index, -2.0)?;
                    }
                }
            }
        }
        if self.indigo > 0 {
            self.indigo = (f64::from(self.indigo) / params.indigo_divisor) as i32;
        }
        if reproduction.early_energy > 0 {
            reproduction.early_energy =
                crate::java_math::round_integer_tenth(reproduction.early_energy);
            self.is_reproductive = true;
            let minimum = if self.is_plant {
                40
            } else if self.is_consumer || self.is_fungus {
                21
            } else if self.transfers_energy && self.is_infectious && self.is_blond {
                if age == 0 {
                    reproduction.countdown = reproduction.maximum_countdown;
                }
                21
            } else if self.spore_version < 0 || self.spore_version == 6 {
                21
            } else {
                14
            };
            reproduction.early_energy = reproduction
                .energy
                .wrapping_sub(reproduction.early_energy)
                .max(minimum);
            if !self.is_plant
                && !self.is_consumer
                && !self.is_fungus
                && self.create_lavender > 0
                && self.is_infectious
                && self.is_blond
                && age == 0
            {
                self.lavender = 0;
            }
        }
        if self.reproduce_late > 0 {
            self.reproduce_late = crate::java_math::round_integer_tenth(self.reproduce_late);
            if self.is_infectious {
                reproduction.energy = 40_i32
                    .wrapping_add(3_i32.wrapping_mul(segments))
                    .wrapping_add(
                        (f64::from(self.reproduce_late)
                            * crate::geometry::GROWTH_SCALES[crate::geometry::growth_index(body.growth_ratio)?])
                            as i32,
                    );
            } else {
                let additional = self
                    .reproduce_late
                    .wrapping_sub(3_i32.wrapping_mul(symmetry));
                if additional > 0 {
                    reproduction.energy = 40_i32
                        .wrapping_add(3_i32.wrapping_mul(segments))
                        .wrapping_add(additional);
                }
                if age == 0 {
                    reproduction.maximum_countdown = 14;
                }
            }
        }
        if self.gold > 0 {
            reproduction.maximum_age = params
                .max_age
                .wrapping_add(segments / params.age_divisor)
                .wrapping_add(java_round(f64::from(self.gold / 10) / params.gold_divisor));
            self.gold = 1;
        }
        Ok(())
    }
}
