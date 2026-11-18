// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::{JavaColor, JavaRandom, JavaText, source_palette_color};
use std::{cell::RefCell, f64::consts::PI, fmt, rc::Rc};

/// Gene.randomize draw order, including all 36 independent reaction channels
pub const REACTION_NAMES: [&str; 36] = [
    "red",
    "green",
    "blue",
    "plague",
    "scourge",
    "white",
    "gray",
    "silver",
    "default",
    "consumer",
    "plant",
    "magenta",
    "pink",
    "coral",
    "orange",
    "bark",
    "violet",
    "virus",
    "maroon",
    "olive",
    "mint",
    "cream",
    "spike",
    "fallow",
    "lightblue",
    "ochre",
    "sky",
    "lilac",
    "fire",
    "lightbrown",
    "greenbrown",
    "brown",
    "ice",
    "broken",
    "sick",
    "friend",
];

/// Gene.randomizeColor selection order and Utils default weights
/// Yellow remains last because Java selects it through its final fallback
const fn color_rgb(kind: crate::ColorKind) -> u32 {
    match kind.argb() {
        Some(argb) => argb & 0xffffff,
        None => panic!("Default gene colors must have an ARGB identity"),
    }
}

pub const DEFAULT_COLORS: [(&str, u32, i32); 46] = [
    ("RED", color_rgb(crate::ColorKind::Red), 1),
    ("GREEN", color_rgb(crate::ColorKind::Green), 3),
    ("BLUE", color_rgb(crate::ColorKind::Blue), 2),
    ("CYAN", color_rgb(crate::ColorKind::Cyan), 8),
    ("WHITE", color_rgb(crate::ColorKind::White), 4),
    ("GRAY", color_rgb(crate::ColorKind::Gray), 1),
    ("MAGENTA", color_rgb(crate::ColorKind::Magenta), 5),
    ("PINK", color_rgb(crate::ColorKind::Pink), 2),
    ("CORAL", color_rgb(crate::ColorKind::Coral), 1),
    ("ORANGE", color_rgb(crate::ColorKind::Orange), 1),
    ("FOREST", color_rgb(crate::ColorKind::Forest), 3),
    ("SPRING", color_rgb(crate::ColorKind::Spring), 3),
    ("LEAF", color_rgb(crate::ColorKind::Leaf), 3),
    ("SUMMER", color_rgb(crate::ColorKind::Summer), 3),
    ("LIME", color_rgb(crate::ColorKind::Lime), 3),
    ("BARK", color_rgb(crate::ColorKind::Bark), 3),
    ("VIOLET", color_rgb(crate::ColorKind::Violet), 1),
    ("TEAL", color_rgb(crate::ColorKind::Teal), 8),
    ("EYE", color_rgb(crate::ColorKind::Eye), 4),
    ("MAROON", color_rgb(crate::ColorKind::Maroon), 2),
    ("OLIVE", color_rgb(crate::ColorKind::Olive), 1),
    ("MINT", color_rgb(crate::ColorKind::Mint), 2),
    ("CREAM", color_rgb(crate::ColorKind::Cream), 1),
    ("ROSE", color_rgb(crate::ColorKind::Rose), 2),
    ("DARK", color_rgb(crate::ColorKind::Dark), 1),
    ("OCHRE", color_rgb(crate::ColorKind::Ochre), 1),
    ("SKY", color_rgb(crate::ColorKind::Sky), 1),
    ("LILAC", color_rgb(crate::ColorKind::Lilac), 1),
    ("SILVER", color_rgb(crate::ColorKind::Silver), 1),
    ("FIRE", color_rgb(crate::ColorKind::Fire), 1),
    ("DARKGRAY", color_rgb(crate::ColorKind::DarkGray), 5),
    ("GOLD", color_rgb(crate::ColorKind::Gold), 2),
    ("BLOND", color_rgb(crate::ColorKind::Blond), 2),
    ("FLOWER", color_rgb(crate::ColorKind::Flower), 2),
    ("AUBURN", color_rgb(crate::ColorKind::Auburn), 2),
    ("PLAGUE", color_rgb(crate::ColorKind::Plague), 1),
    ("SPIKE", color_rgb(crate::ColorKind::Spike), 1),
    ("INDIGO", color_rgb(crate::ColorKind::Indigo), 2),
    ("LAVENDER", color_rgb(crate::ColorKind::Lavender), 2),
    ("FALLOW", color_rgb(crate::ColorKind::Fallow), 1),
    ("SPORE", color_rgb(crate::ColorKind::Spore), 2),
    ("JADE", color_rgb(crate::ColorKind::Jade), 3),
    ("C4", color_rgb(crate::ColorKind::C4), 3),
    ("GRASS", color_rgb(crate::ColorKind::Grass), 3),
    ("PURPLE", color_rgb(crate::ColorKind::Purple), 3),
    ("YELLOW", color_rgb(crate::ColorKind::Yellow), 2),
];

/// Shared mutable genes preserve explicit construction and unchanged inheritance aliases
pub type GeneRef = Rc<RefCell<Gene>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneticsError {
    InvalidWeights,
    InvalidComplexity,
    InvalidMutationRange,
    InvalidRandomBound,
    InvalidGeneCount,
    AllocationFailed,
    MissingClade,
    NullGeneArray,
    ZeroGeneDivisor,
    NullGene { index: usize },
    GeneIndex { index: i64, length: usize },
    GeneIndexDomain { index: usize },
}

impl fmt::Display for GeneticsError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::InvalidWeights => {
                "Color weights must be nonnegative with a positive Java integer sum"
            }
            Self::InvalidComplexity => "Initial complexity must be positive",
            Self::InvalidMutationRange => {
                "Mutation limits must form an inclusive positive Java random bound"
            }
            Self::InvalidRandomBound => {
                "Inherited mutation arithmetic produced an invalid Java random bound"
            }
            Self::InvalidGeneCount => "Genetic construction produced an invalid Java gene count",
            Self::AllocationFailed => "Cannot allocate the requested genes",
            Self::MissingClade => "A structural lineage update requires a clade identifier",
            Self::NullGeneArray => "GeneticCode has no Gene array",
            Self::ZeroGeneDivisor => "Gene index modulo divisor is zero",
            Self::NullGene { .. } => "GeneticCode slot contains no Gene",
            Self::GeneIndex { .. } => "Gene index is outside the stored array",
            Self::GeneIndexDomain { .. } => "Gene index exceeds the Java int domain",
        })
    }
}
impl std::error::Error for GeneticsError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneticsParams {
    pub color_weights: [i32; 46],
    pub initial_complexity: i32,
    pub min_mutation_rate: i32,
    pub max_mutation_rate: i32,
    pub meta_mutation_rate: i32,
    pub clade_complexity: i32,
}

impl Default for GeneticsParams {
    fn default() -> Self {
        Self {
            color_weights: DEFAULT_COLORS.map(|entry| entry.2),
            initial_complexity: 2,
            min_mutation_rate: 0,
            max_mutation_rate: 100,
            meta_mutation_rate: 1000,
            clade_complexity: 25,
        }
    }
}

impl GeneticsParams {
    pub fn validate(&self) -> Result<(), GeneticsError> {
        color_total(&self.color_weights)?;
        if self.initial_complexity <= 0 {
            return Err(GeneticsError::InvalidComplexity);
        }
        if self
            .max_mutation_rate
            .checked_sub(self.min_mutation_rate)
            .and_then(|difference| difference.checked_add(1))
            .is_none_or(|bound| bound <= 0)
        {
            return Err(GeneticsError::InvalidMutationRange);
        }
        Ok(())
    }
}

fn color_total(weights: &[i32; 46]) -> Result<i32, GeneticsError> {
    let mut total = 0_i32;
    for weight in weights {
        if *weight < 0 {
            return Err(GeneticsError::InvalidWeights);
        }
        total = total
            .checked_add(*weight)
            .ok_or(GeneticsError::InvalidWeights)?;
    }
    if total <= 0 {
        return Err(GeneticsError::InvalidWeights);
    }
    Ok(total)
}

fn selected_color(random: &mut JavaRandom, weights: &[i32; 46], total: i32) -> JavaColor {
    let choice = random.bounded(total);
    let mut accumulated = 0;
    for (index, &(_, rgb, _)) in DEFAULT_COLORS.iter().enumerate() {
        accumulated += weights[index];
        if choice < accumulated {
            return source_palette_color(crate::ColorKind::from_argb(0xff000000 | rgb) as usize)
                .expect("Selected named palette color");
        }
    }
    unreachable!("Validated color weights cover every random outcome")
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gene {
    pub(crate) length: f64,
    pub(crate) theta: f64,
    color: Option<JavaColor>,
    pub(crate) branch: i32,
    reactions: [i32; 36],
}

impl Default for Gene {
    fn default() -> Self {
        Self {
            length: 0.0,
            theta: 0.0,
            color: None,
            branch: 0,
            reactions: [0; 36],
        }
    }
}

impl Gene {
    /// Match the explicit Java constructor without validating supplied scalar values
    pub fn new(
        length: f64,
        theta: f64,
        color: Option<u32>,
        branch: i32,
        reactions: [i32; 36],
    ) -> Self {
        Self::new_with_color(
            length,
            theta,
            color.map(JavaColor::from_argb),
            branch,
            reactions,
        )
    }

    /// Assign an actual shared Color reference without replacing its metadata
    pub fn new_with_color(
        length: f64,
        theta: f64,
        color: Option<JavaColor>,
        branch: i32,
        reactions: [i32; 36],
    ) -> Self {
        Self {
            length,
            theta,
            color,
            branch,
            reactions,
        }
    }

    pub fn shared(self) -> GeneRef {
        Rc::new(RefCell::new(self))
    }

    pub fn random(random: &mut JavaRandom) -> Self {
        Self::random_with(random, &GeneticsParams::default().color_weights)
            .expect("Valid reference defaults")
    }

    pub fn random_with(
        random: &mut JavaRandom,
        weights: &[i32; 46],
    ) -> Result<Self, GeneticsError> {
        let mut gene = Self::default();
        gene.randomize(random, weights)?;
        Ok(gene)
    }

    /// Match Gene.randomize without changing its branch
    pub fn randomize(
        &mut self,
        random: &mut JavaRandom,
        weights: &[i32; 46],
    ) -> Result<(), GeneticsError> {
        let total = color_total(weights)?;
        self.randomize_length(random);
        self.randomize_theta(random);
        self.color = Some(selected_color(random, weights, total));
        for reaction in &mut self.reactions {
            *reaction = random.bounded(6);
        }
        Ok(())
    }

    pub fn randomize_length(&mut self, random: &mut JavaRandom) {
        self.length = 2.0 + random.next_f64() * 16.0;
    }
    pub fn randomize_theta(&mut self, random: &mut JavaRandom) {
        self.theta = random.next_f64() * 2.0 * PI;
    }
    pub fn randomize_color(
        &mut self,
        random: &mut JavaRandom,
        weights: &[i32; 46],
    ) -> Result<(), GeneticsError> {
        self.color = Some(selected_color(random, weights, color_total(weights)?));
        Ok(())
    }
    pub fn randomize_reaction(&mut self, index: usize, random: &mut JavaRandom) {
        let reaction = &mut self.reactions[index];
        *reaction = random.bounded(6);
    }

    pub fn length(&self) -> f64 {
        self.length
    }
    pub fn theta(&self) -> f64 {
        self.theta
    }
    /// Return initialized RGB; null colors must be rejected at the consuming boundary
    pub fn rgb(&self) -> u32 {
        self.color.as_ref().expect("Gene color is null").argb() & 0xffffff
    }
    /// Project the stored full ARGB value without changing the shared Color owner
    pub fn color(&self) -> Option<u32> {
        self.color.as_ref().map(JavaColor::argb)
    }

    pub fn color_handle(&self) -> Option<&JavaColor> {
        self.color.as_ref()
    }

    /// Assign the supplied reference; edits replace holders rather than shared Color data
    pub fn set_color_handle(&mut self, value: Option<JavaColor>) {
        self.color = value;
    }
    pub fn branch(&self) -> i32 {
        self.branch
    }
    pub fn reactions(&self) -> &[i32; 36] {
        &self.reactions
    }
    pub fn reaction(&self, index: usize) -> i32 {
        self.reactions[index]
    }
    pub fn set_length(&mut self, value: f64) {
        self.length = value;
    }
    pub fn set_theta(&mut self, value: f64) {
        self.theta = value;
    }
    pub fn set_color(&mut self, value: Option<u32>) {
        self.color = value.map(JavaColor::from_argb);
    }
    pub fn set_branch(&mut self, value: i32) {
        self.branch = value;
    }
    pub fn set_reaction(&mut self, index: usize, value: i32) {
        self.reactions[index] = value;
    }
    pub fn set_reactions(&mut self, value: [i32; 36]) {
        self.reactions = value;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GenomeModifiers {
    pub cream: i32,
    pub fallow: i32,
    pub spore: i32,
    pub adapt_spore: i32,
    pub plague: bool,
    pub disperse_children: bool,
    pub generation_battle: bool,
    pub sibling_battle: bool,
    pub altruist: bool,
    pub familial: bool,
    pub social: bool,
    pub peaceful: bool,
    pub passive: bool,
    pub clockwise: bool,
    pub mimic_all: bool,
    pub modifies_pink: bool,
    pub modifies_lilac: bool,
    pub modifies_sky: bool,
    pub selfish: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GenomeProperties {
    pub symmetry: i32,
    pub mirror: i32,
    pub mutation_rate: i32,
    pub activity: i32,
    pub modifiers: GenomeModifiers,
}

#[derive(Debug, PartialEq)]
pub struct Genome {
    pub(crate) genes: Option<crate::GeneArrayRef>,
    pub(crate) symmetry: i32,
    pub(crate) mirror: i32,
    mutation_rate: i32,
    activity: i32,
    modifiers: GenomeModifiers,
    generation: i32,
    clade_id: Option<JavaText>,
    update_clade: bool,
}

/// Detached value snapshots preserve nulls and independently copy every present Gene slot
impl Clone for Genome {
    fn clone(&self) -> Self {
        self.with_array(self.genes.as_ref().map(|array| {
            crate::GeneArrayRef::from_slots(
                array
                    .slots()
                    .iter()
                    .map(|slot| slot.as_ref().map(|gene| gene.borrow().clone().shared()))
                    .collect(),
            )
        }))
    }
}

impl Genome {
    /// Copy the supplied container while retaining its shared Gene objects
    pub fn from_parts(genes: &[GeneRef], properties: GenomeProperties) -> Self {
        Self::from_raw_array(Some(crate::GeneArrayRef::from_slots(genes.iter().cloned().map(Some).collect())), properties, 0, None, false)
    }

    /// The explicit Java list constructor copies slots while retaining nullable Gene references
    pub fn from_nullable_parts(genes: &[Option<GeneRef>], properties: GenomeProperties) -> Self {
        Self::from_raw_array(Some(crate::GeneArrayRef::from_slots(genes.to_vec())), properties, 0, None, false)
    }

    /// Attach raw stored owners without construction, validation, or random draws
    pub fn from_raw_array(
        genes: Option<crate::GeneArrayRef>,
        properties: GenomeProperties,
        generation: i32,
        clade_id: Option<JavaText>,
        update_clade: bool,
    ) -> Self {
        Self {
            genes,
            symmetry: properties.symmetry,
            mirror: properties.mirror,
            mutation_rate: properties.mutation_rate,
            activity: properties.activity,
            modifiers: properties.modifiers,
            generation,
            clade_id,
            update_clade,
        }
    }
    pub(crate) fn with_array(&self, genes: Option<crate::GeneArrayRef>) -> Self {
        Self::from_raw_array(
            genes,
            self.properties(),
            self.generation,
            self.clade_id.clone(),
            self.update_clade,
        )
    }
    pub fn gene_array(&self) -> Option<&crate::GeneArrayRef> {
        self.genes.as_ref()
    }
    pub fn set_gene_array(&mut self, array: Option<crate::GeneArrayRef>) {
        self.genes = array;
    }
    pub fn gene_count(&self) -> Result<usize, GeneticsError> {
        Ok(self
            .genes
            .as_ref()
            .ok_or(GeneticsError::NullGeneArray)?
            .len())
    }
    /// Java getGene returns null slots successfully and checks the array before the index
    pub fn gene_slot(&self, index: i32) -> Result<Option<GeneRef>, GeneticsError> {
        self.genes
            .as_ref()
            .ok_or(GeneticsError::NullGeneArray)?
            .get(index)
    }
    pub fn gene_ref(&self, index: usize) -> Result<GeneRef, GeneticsError> {
        let array = self.genes.as_ref().ok_or(GeneticsError::NullGeneArray)?;
        let signed = i32::try_from(index).map_err(|_| GeneticsError::GeneIndexDomain { index })?;
        array.get(signed)?.ok_or(GeneticsError::NullGene { index })
    }
    pub fn gene_for_segment(&self, index: usize) -> Result<GeneRef, GeneticsError> {
        let count = self.gene_count()?;
        i32::try_from(index).map_err(|_| GeneticsError::GeneIndexDomain { index })?;
        let gene = index.checked_rem(count).ok_or(GeneticsError::ZeroGeneDivisor)?;
        self.gene_ref(gene)
    }
    /// Explicit checked batch for consumers which require every slot to contain a Gene
    pub fn genes(&self) -> Result<Vec<GeneRef>, GeneticsError> {
        let count = self.gene_count()?;
        let mut genes = Vec::new();
        genes.try_reserve_exact(count).map_err(|_| GeneticsError::AllocationFailed)?;
        for index in 0..count { genes.push(self.gene_ref(index)?); }
        Ok(genes)
    }

    /// GeneticCode.clone dereferences each source slot in order before returning a new object
    pub fn java_clone(&self) -> Result<Self, GeneticsError> {
        let count = self.gene_count()?;
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(count)
            .map_err(|_| GeneticsError::AllocationFailed)?;
        for index in 0..count {
            slots.push(Some(self.gene_ref(index)?.borrow().clone().shared()));
        }
        Ok(self.with_array(Some(crate::GeneArrayRef::from_slots(slots))))
    }

    pub fn random(random: &mut JavaRandom) -> Self {
        Self::random_with(random, &GeneticsParams::default()).expect("Valid reference defaults")
    }

    pub fn random_with(
        random: &mut JavaRandom,
        params: &GeneticsParams,
    ) -> Result<Self, GeneticsError> {
        params.validate()?;
        let mirror = random.bounded(2);
        let symmetry = random.bounded(8) + 1;
        // Preserve Java int multiplication and remainder before dividing back to a gene count
        let mut segments = (1 + random.bounded(params.initial_complexity)).wrapping_mul(symmetry);
        if segments % symmetry != 0 {
            segments = segments.wrapping_add(symmetry - segments % symmetry);
        }
        let count = segments / symmetry;
        if count < 0 {
            return Err(GeneticsError::InvalidGeneCount);
        }
        let mut genes = Vec::new();
        genes
            .try_reserve_exact(count as usize)
            .map_err(|_| GeneticsError::AllocationFailed)?;
        for index in 0..count {
            let mut gene = Gene::random_with(random, &params.color_weights)?;
            gene.branch = random_branch(random, index as usize);
            genes.push(gene.shared());
        }
        let mutation_rate = random.bounded(params.max_mutation_rate - params.min_mutation_rate + 1)
            + params.min_mutation_rate;
        let activity = random.bounded(3);
        let modifiers = random_modifiers(random);
        Ok(Self::from_parts(
            &genes,
            GenomeProperties {
                symmetry,
                mirror,
                mutation_rate,
                activity,
                modifiers,
            },
        ))
    }

    /// Preserve every mutation gate and subsequent value draw from GeneticCode(parentCode)
    pub fn inherit_with(
        parent: &Self,
        random: &mut JavaRandom,
        params: &GeneticsParams,
    ) -> Result<Self, GeneticsError> {
        params.validate()?;
        let mut mutation_rate = parent.mutation_rate;
        if gate(random, params.meta_mutation_rate) {
            if random.bounded(10) < 1 {
                if random.next_bool() {
                    if mutation_rate < params.max_mutation_rate {
                        let bound = params
                            .max_mutation_rate
                            .wrapping_sub(mutation_rate)
                            .wrapping_add(1);
                        mutation_rate = random
                            .next_bounded(bound)
                            .map_err(|_| GeneticsError::InvalidRandomBound)?
                            .wrapping_add(mutation_rate);
                    }
                } else if mutation_rate > params.min_mutation_rate {
                    let bound = mutation_rate
                        .wrapping_sub(params.min_mutation_rate)
                        .wrapping_add(1);
                    mutation_rate = random
                        .next_bounded(bound)
                        .map_err(|_| GeneticsError::InvalidRandomBound)?
                        .wrapping_add(params.min_mutation_rate);
                }
            } else if random.next_bool() {
                mutation_rate = mutation_rate
                    .wrapping_add(random.bounded(2))
                    .wrapping_add(1);
            } else {
                mutation_rate = mutation_rate
                    .wrapping_sub(random.bounded(2))
                    .wrapping_sub(1);
            }
        }
        if mutation_rate > params.max_mutation_rate {
            mutation_rate = params.max_mutation_rate;
        } else if mutation_rate < params.min_mutation_rate {
            mutation_rate = params.min_mutation_rate;
        }
        let mirror = if gate(random, mutation_rate) {
            random.bounded(2)
        } else {
            parent.mirror
        };
        let mut added = None;
        let mut removed = None;
        let mut repair_branch = false;
        let mut count;
        let mut update_clade = false;
        let symmetry;
        if gate(random, mutation_rate) {
            symmetry = random.bounded(8) + 1;
            count =
                i32::try_from(parent.gene_count()?).map_err(|_| GeneticsError::InvalidGeneCount)?;
            update_clade = true;
        } else {
            symmetry = parent.symmetry;
            if gate(random, mutation_rate) {
                if random.next_bool() {
                    let parent_count = i32::try_from(parent.gene_count()?)
                        .map_err(|_| GeneticsError::InvalidGeneCount)?;
                    count = parent_count;
                    if parent_count < 100 {
                        count += 1;
                        added = Some(random.bounded(count) as usize);
                        update_clade = true;
                    }
                } else {
                    let parent_count = i32::try_from(parent.gene_count()?)
                        .map_err(|_| GeneticsError::InvalidGeneCount)?;
                    count = parent_count;
                    if parent_count > 1 {
                        count -= 1;
                        removed = Some(random.bounded(parent_count) as usize);
                        update_clade = true;
                        repair_branch = true;
                    }
                }
            } else {
                count = i32::try_from(parent.gene_count()?)
                    .map_err(|_| GeneticsError::InvalidGeneCount)?;
            }
        }
        let mut genes = Vec::new();
        genes
            .try_reserve_exact(count as usize)
            .map_err(|_| GeneticsError::AllocationFailed)?;
        let mut index = 0;
        let mut source = 0_i32;
        while index < count as usize {
            if removed == Some(source as usize) {
                source += 1;
                continue;
            }
            let mut cloned = 0;
            if added == Some(index) {
                if random.bounded(8) < 1 {
                    if index == 0 {
                        cloned = -1;
                    } else if index < count as usize - 1 {
                        if random.next_bool() {
                            cloned = -1;
                        } else {
                            cloned = 1;
                            source -= 1;
                        }
                    } else {
                        cloned = 1;
                        source -= 1;
                    }
                } else {
                    let mut gene = Gene::random_with(random, &params.color_weights)?;
                    gene.branch = random_branch(random, index);
                    genes.push(Some(gene.shared()));
                    index += 1;
                    continue;
                }
            }
            let mut changes = [false; 40];
            for change in &mut changes {
                *change = gate(random, mutation_rate);
            }
            if changes.iter().any(|change| *change) || repair_branch || cloned != 0 {
                let mut gene = Gene::default();
                if cloned != 0 {
                    if random.next_bool() || changes[0] {
                        gene.randomize_length(random);
                    } else {
                        gene.length = parent.gene_ref(source as usize)?.borrow().length();
                    }
                    gene.randomize_theta(random);
                    if random.bounded(5) < 1 || changes[2] {
                        gene.branch = random_branch(random, index);
                    } else {
                        gene.branch = parent.gene_ref(source as usize)?.borrow().branch();
                        if cloned == 1 {
                            if gene.branch == -1 {
                                if random.next_bool() {
                                    gene.branch = index as i32 - 1;
                                } else if random.next_bool() {
                                    gene.branch = index as i32;
                                }
                            } else if random.next_bool() {
                                if random.next_bool() {
                                    gene.branch = -1;
                                } else {
                                    gene.branch = index as i32;
                                }
                            }
                        }
                    }
                } else {
                    if changes[0] {
                        gene.randomize_length(random);
                    } else {
                        gene.length = parent.gene_ref(source as usize)?.borrow().length();
                    }
                    if changes[1] {
                        gene.randomize_theta(random);
                    } else {
                        gene.theta = parent.gene_ref(source as usize)?.borrow().theta();
                    }
                    if changes[2] {
                        gene.branch = random_branch(random, index);
                    } else {
                        gene.branch = parent.gene_ref(source as usize)?.borrow().branch();
                        if repair_branch && gene.branch > index as i32 {
                            gene.branch = index as i32;
                        }
                    }
                }
                for reaction in 0..36 {
                    if changes[reaction + 3] {
                        gene.randomize_reaction(reaction, random);
                    } else {
                        gene.reactions[reaction] = parent
                            .gene_ref(source as usize)?
                            .borrow()
                            .reaction(reaction);
                    }
                }
                if changes[39] {
                    gene.randomize_color(random, &params.color_weights)?;
                } else {
                    gene.color = parent
                        .gene_ref(source as usize)?
                        .borrow()
                        .color_handle()
                        .cloned();
                }
                genes.push(Some(gene.shared()));
                if cloned == -1 {
                    source -= 1;
                }
            } else {
                genes.push(parent.gene_slot(source)?);
            }
            index += 1;
            source += 1;
        }
        let activity = mutate_int(parent.activity, 3, 0, mutation_rate, random);
        let modifiers = mutate_modifiers(parent.modifiers, mutation_rate, random);
        let mut child = Self::from_raw_array(
            Some(crate::GeneArrayRef::from_slots(genes)),
            GenomeProperties {
                symmetry,
                mirror,
                mutation_rate,
                activity,
                modifiers,
            },
            0,
            None,
            false,
        );
        child.generation = parent.generation.wrapping_add(1);
        child.clade_id = parent.clade_id.clone();
        child.update_clade = update_clade;
        Ok(child)
    }

    /// Apply Organism.inherit's structural lineage suffix after successful placement
    pub fn update_lineage(
        &mut self,
        parent: &Self,
        id: i32,
        complexity: i32,
    ) -> Result<(), GeneticsError> {
        if !self.update_clade {
            return Ok(());
        }
        self.update_clade = false;
        let clade = self.clade_id.as_ref().ok_or(GeneticsError::MissingClade)?;
        let markers = clade
            .units()
            .iter()
            .filter(|&&unit| matches!(unit, 43 | 45 | 60 | 62))
            .count();
        if (markers as i64) < complexity as i64 {
            if self.gene_count()? > parent.gene_count()? {
                self.clade_id = Some(
                    self.clade_id
                        .as_ref()
                        .unwrap()
                        .with_suffix(&format!("+{id}")),
                );
            } else if self.gene_count()? < parent.gene_count()? {
                self.clade_id = Some(
                    self.clade_id
                        .as_ref()
                        .unwrap()
                        .with_suffix(&format!("-{id}")),
                );
            }
            if self.symmetry > parent.symmetry {
                self.clade_id = Some(
                    self.clade_id
                        .as_ref()
                        .unwrap()
                        .with_suffix(&format!(">{id}")),
                );
            } else if self.symmetry < parent.symmetry {
                self.clade_id = Some(
                    self.clade_id
                        .as_ref()
                        .unwrap()
                        .with_suffix(&format!("<{id}")),
                );
            }
        }
        Ok(())
    }

    pub fn symmetry(&self) -> i32 {
        self.symmetry
    }
    pub fn mirror(&self) -> i32 {
        self.mirror
    }
    pub fn mirrored(&self) -> bool {
        self.mirror != 0
    }
    pub fn mutation_rate(&self) -> i32 {
        self.mutation_rate
    }
    pub fn activity(&self) -> i32 {
        self.activity
    }
    pub fn modifiers(&self) -> GenomeModifiers {
        self.modifiers
    }
    pub fn generation(&self) -> i32 {
        self.generation
    }
    pub fn clade_id(&self) -> Option<&JavaText> {
        self.clade_id.as_ref()
    }
    pub fn update_clade(&self) -> bool {
        self.update_clade
    }
    pub fn properties(&self) -> GenomeProperties {
        GenomeProperties {
            symmetry: self.symmetry,
            mirror: self.mirror,
            mutation_rate: self.mutation_rate,
            activity: self.activity,
            modifiers: self.modifiers,
        }
    }
    pub fn set_properties(&mut self, properties: GenomeProperties) {
        self.symmetry = properties.symmetry;
        self.mirror = properties.mirror;
        self.mutation_rate = properties.mutation_rate;
        self.activity = properties.activity;
        self.modifiers = properties.modifiers;
    }
    pub fn set_generation(&mut self, value: i32) {
        self.generation = value;
    }
    pub fn set_clade_id(&mut self, value: Option<String>) {
        self.clade_id = value.map(|value| JavaText::from_utf8(&value));
    }
    /// Assign the actual immutable owner without a UTF8 roundtrip
    pub fn set_clade_text(&mut self, value: Option<JavaText>) {
        self.clade_id = value;
    }
    pub fn set_update_clade(&mut self, value: bool) {
        self.update_clade = value;
    }
}

fn random_branch(random: &mut JavaRandom, index: usize) -> i32 {
    if random.next_bool() {
        -1
    } else {
        random.bounded(index as i32 + 1)
    }
}
fn gate(random: &mut JavaRandom, rate: i32) -> bool {
    random.bounded(10000) < rate
}
fn mutate_int(value: i32, bound: i32, offset: i32, rate: i32, random: &mut JavaRandom) -> i32 {
    if gate(random, rate) {
        random.bounded(bound) + offset
    } else {
        value
    }
}
fn mutate_bool(value: bool, rate: i32, random: &mut JavaRandom) -> bool {
    if gate(random, rate) {
        random.next_bool()
    } else {
        value
    }
}

fn random_modifiers(random: &mut JavaRandom) -> GenomeModifiers {
    GenomeModifiers {
        cream: random.bounded(3) + 1,
        fallow: random.bounded(4) + 1,
        spore: random.bounded(12) + 1,
        adapt_spore: random.bounded(70) + 1,
        plague: random.next_bool(),
        disperse_children: random.next_bool(),
        generation_battle: random.next_bool(),
        sibling_battle: random.next_bool(),
        altruist: random.next_bool(),
        familial: random.next_bool(),
        social: random.next_bool(),
        peaceful: random.next_bool(),
        passive: random.next_bool(),
        clockwise: random.next_bool(),
        mimic_all: random.next_bool(),
        modifies_pink: random.next_bool(),
        modifies_lilac: random.next_bool(),
        modifies_sky: random.next_bool(),
        selfish: random.next_bool(),
    }
}
fn mutate_modifiers(
    parent: GenomeModifiers,
    rate: i32,
    random: &mut JavaRandom,
) -> GenomeModifiers {
    GenomeModifiers {
        cream: mutate_int(parent.cream, 3, 1, rate, random),
        fallow: mutate_int(parent.fallow, 4, 1, rate, random),
        spore: mutate_int(parent.spore, 12, 1, rate, random),
        adapt_spore: mutate_int(parent.adapt_spore, 70, 1, rate, random),
        plague: mutate_bool(parent.plague, rate, random),
        disperse_children: mutate_bool(parent.disperse_children, rate, random),
        generation_battle: mutate_bool(parent.generation_battle, rate, random),
        sibling_battle: mutate_bool(parent.sibling_battle, rate, random),
        altruist: mutate_bool(parent.altruist, rate, random),
        familial: mutate_bool(parent.familial, rate, random),
        social: mutate_bool(parent.social, rate, random),
        peaceful: mutate_bool(parent.peaceful, rate, random),
        passive: mutate_bool(parent.passive, rate, random),
        clockwise: mutate_bool(parent.clockwise, rate, random),
        mimic_all: mutate_bool(parent.mimic_all, rate, random),
        modifies_pink: mutate_bool(parent.modifies_pink, rate, random),
        modifies_lilac: mutate_bool(parent.modifies_lilac, rate, random),
        modifies_sky: mutate_bool(parent.modifies_sky, rate, random),
        selfish: mutate_bool(parent.selfish, rate, random),
    }
}
