// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::{ColorArrayRef, ColorKind, Genome, JavaColor, java_math};
use std::{f64::consts::PI, fmt};

const EYE: u32 = ColorKind::Eye.argb().unwrap();

pub const GROWTH_SCALES: [f64; 16] = [
    1.00, 0.90, 0.80, 0.71, 0.63, 0.56, 0.50, 0.45, 0.40, 0.36, 0.32, 0.28, 0.25, 0.22, 0.20, 0.18,
];

/// Check the reached Java table access after signed subtraction wraps
pub(crate) fn growth_index(growth_ratio: i32) -> Result<usize, GeometryError> {
    let index = growth_ratio.wrapping_sub(1);
    if (0..16).contains(&index) {
        Ok(index as usize)
    } else {
        Err(GeometryError::GrowthIndex { index })
    }
}

/// Source polar vector operations do not normalize angles or replace negative lengths
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PolarVector {
    pub modulus: f64,
    pub theta: f64,
}

impl PolarVector {
    pub fn cartesian(self) -> [f64; 2] {
        [
            self.modulus * self.theta.cos(),
            self.modulus * self.theta.sin(),
        ]
    }
    pub fn invert_x(&mut self) {
        self.theta = PI - self.theta;
    }
    pub fn invert_y(&mut self) {
        self.theta = 2.0 * PI - self.theta;
    }
    pub fn add_angle(&mut self, angle: f64) {
        self.theta += angle;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IntRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl IntRect {
    /// Preserve Rectangle.contains overflow semantics without normalizing signed extents
    pub fn contains(self, point: [i32; 2]) -> bool {
        let [x, y] = point;
        if self.width < 0 || self.height < 0 || x < self.x || y < self.y {
            return false;
        }
        let right = self.width.wrapping_add(self.x);
        let bottom = self.height.wrapping_add(self.y);
        (right < self.x || right > x) && (bottom < self.y || bottom > y)
    }
}

/// Classification comes from the organism owner, independently of current drawing colors
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EyeState {
    pub enabled: bool,
    pub good_vision: bool,
    pub consumer: bool,
    pub fungus: bool,
    pub infectious: bool,
}

/// Detached inspection data; edits do not change the primitive owners
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub start: [i32; 2],
    pub end: [i32; 2],
    pub reactions: [i32; 36],
    pub physical_length: f64,
    pub start_radius: f64,
    pub end_radius: f64,
}

#[derive(Debug, PartialEq)]
pub struct Body {
    /// Saved Java _segments, independent of every array length
    pub segment_count: i32,
    pub arrays: crate::OrganismArrays,
    pub phenotype_colors: Option<ColorArrayRef>,
    pub size_rect: Option<crate::RectangleRef>,
    pub mass: f64,
    pub inertia: f64,
    pub eyes: EyeState,
    pub update_effects: i32,
    /// Saved Java _growthRatio; checked constructors constrain only their own inputs
    pub growth_ratio: i32,
}

impl Clone for Body {
    fn clone(&self) -> Self {
        let mut copied = self.copy_with_colors(
            self.phenotype_colors
                .as_ref()
                .map(ColorArrayRef::detached_copy),
        );
        copied.arrays = self.arrays.copy_with_owners(
            &mut std::collections::HashMap::new(),
            &mut std::collections::HashMap::new(),
        );
        copied.size_rect = self
            .size_rect
            .as_ref()
            .map(crate::RectangleRef::detached_copy);
        copied
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RotatedSegment {
    pub start: [i32; 2],
    pub end: [i32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct RotatedBody {
    pub segments: Vec<RotatedSegment>,
    pub center: [i32; 2],
    pub bounds: IntRect,
}

/// Ordered unscaled drawLine arguments plus the source Graphics2D transform
#[derive(Clone, Debug, PartialEq)]
pub struct GenomePreview {
    pub segments: Vec<RotatedSegment>,
    pub colors: Vec<u32>,
    pub translation: [i32; 2],
    pub scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryError {
    MissingGenome,
    Genetics(crate::GeneticsError),
    EmptyGenome,
    InvalidSymmetry,
    InvalidGrowth,
    GrowthIndex {
        index: i32,
    },
    InvalidBranch {
        gene: usize,
    },
    MissingColor {
        gene: usize,
    },
    NonfiniteGene {
        gene: usize,
    },
    PhenotypeLength,
    MissingColorArray,
    ColorIndex {
        segment: usize,
        length: usize,
    },
    MissingPhenotypeColor {
        segment: usize,
    },
    MissingTemporaryColor,
    MissingRectangle {
        field: &'static str,
    },
    RectangleAssociation,
    PrimitiveIndexDomain {
        index: usize,
    },
    MissingPrimitiveArray {
        field: &'static str,
    },
    PrimitiveIndex {
        field: &'static str,
        index: i32,
        length: usize,
    },
    GeneIndex {
        index: usize,
        length: usize,
    },
    ReactionLength,
    NonfiniteTransform,
    InvalidViewport,
    Capacity,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Genetics(error) => error.fmt(f),
            Self::MissingGenome => f.write_str("Organism has no primary genome"),
            Self::EmptyGenome => f.write_str("Geometry requires at least one gene"),
            Self::InvalidSymmetry => f.write_str("Geometry symmetry must be between one and eight"),
            Self::InvalidGrowth => f.write_str("Growth ratio must be between one and sixteen"),
            Self::GrowthIndex { index } => {
                write!(f, "Growth table index {index} is outside array length 16")
            }
            Self::InvalidBranch { gene } => {
                write!(f, "Gene {gene} references an unavailable predecessor")
            }
            Self::MissingColor { gene } => write!(f, "Gene {gene} has no color"),
            Self::NonfiniteGene { gene } => {
                write!(f, "Gene {gene} has a nonfinite length or angle")
            }
            Self::PhenotypeLength => f.write_str("Phenotype length differs from the segment count"),
            Self::MissingColorArray => f.write_str("Organism has no phenotype Color array"),
            Self::ColorIndex { segment, length } => {
                write!(f, "Color index {segment} is outside array length {length}")
            }
            Self::MissingPhenotypeColor { segment } => {
                write!(f, "Phenotype segment {segment} has no Color")
            }
            Self::MissingTemporaryColor => f.write_str("Organism has no temporary Color"),
            Self::MissingRectangle { field } => write!(f, "Organism has no {field} Rectangle"),
            Self::RectangleAssociation => {
                f.write_str("Rectangle actor association is inconsistent")
            }
            Self::PrimitiveIndexDomain { index } => {
                write!(f, "Primitive index {index} exceeds the Java int domain")
            }
            Self::MissingPrimitiveArray { field } => write!(f, "Organism has no {field} array"),
            Self::PrimitiveIndex {
                field,
                index,
                length,
            } => write!(f, "{field} index {index} is outside array length {length}"),
            Self::GeneIndex { index, length } => {
                write!(f, "Gene index {index} is outside array length {length}")
            }
            Self::ReactionLength => {
                f.write_str("Reaction phenotype length differs from the segment count")
            }
            Self::NonfiniteTransform => {
                f.write_str("Geometry transform must contain finite numbers")
            }
            Self::InvalidViewport => f.write_str("Preview dimensions must be positive"),
            Self::Capacity => f.write_str("Geometry exceeds the available segment capacity"),
        }
    }
}

impl std::error::Error for GeometryError {}
impl From<crate::GeneticsError> for GeometryError {
    fn from(error: crate::GeneticsError) -> Self {
        match error {
            crate::GeneticsError::GeneIndex { index, length } if index >= 0 => Self::GeneIndex { index: index as usize, length },
            _ => Self::Genetics(error),
        }
    }
}

/// Math.round(double) followed by Java's narrowing long-to-int conversion
pub(crate) fn java_round(value: f64) -> i32 {
    java_round_long(value) as i32
}

fn java_round_long(value: f64) -> i64 {
    let lower = value.floor();
    // Adding 0.5 first erases the distinction immediately below a positive half
    if value - lower >= 0.5 {
        (lower + 1.0) as i64
    } else {
        lower as i64
    }
}

fn segment_dimensions(genome: &Genome) -> Result<(usize, usize), GeometryError> {
    let count = genome.gene_count()?;
    let symmetry = genome.symmetry();
    if !(1..=8).contains(&symmetry) {
        return Err(GeometryError::InvalidSymmetry);
    }
    let total = count
        .checked_mul(symmetry as usize)
        .filter(|&n| n <= i32::MAX as usize)
        .ok_or(GeometryError::Capacity)?;
    Ok((count, total))
}

fn validate_gene_scalars(gene: &crate::Gene, index: usize) -> Result<(), GeometryError> {
    if gene.color().is_none() {
        return Err(GeometryError::MissingColor { gene: index });
    }
    if !gene.length().is_finite() || !gene.theta().is_finite() {
        return Err(GeometryError::NonfiniteGene { gene: index });
    }
    Ok(())
}

pub(crate) fn preview_dimensions(genome: &Genome) -> Result<(usize, usize), GeometryError> {
    let (count, total) = segment_dimensions(genome)?;
    for index in 0..count {
        let gene_ref = genome.gene_ref(index)?;
        let gene = gene_ref.borrow();
        validate_gene_scalars(&gene, index)?;
        if index > 0 && (gene.branch() < -1 || i64::from(gene.branch()) > count as i64) {
            return Err(GeometryError::InvalidBranch { gene: index });
        }
    }
    Ok((count, total))
}

fn vector_start(genome: &Genome, repetition: usize, gene: usize) -> Result<f64, GeometryError> {
    let theta = genome.gene_ref(gene)?.borrow().theta();
    Ok(if genome.mirrored() && repetition % 2 != 0 {
        PI - (theta + (repetition - 1) as f64 * 2.0 * PI / genome.symmetry() as f64)
    } else {
        theta + repetition as f64 * 2.0 * PI / genome.symmetry() as f64
    })
}

fn radius(point: [i32; 2]) -> f64 {
    // Java multiplies and adds signed ints before converting the sum to double
    let square = point[0]
        .wrapping_mul(point[0])
        .wrapping_add(point[1].wrapping_mul(point[1]));
    (square as f64).sqrt()
}

impl Body {
    pub fn size(&self) -> Result<IntRect, GeometryError> {
        Ok(crate::rectangles::required_rectangle(self.size_rect.as_ref(), "_sizeRect")?.get())
    }

    pub(crate) fn copy_with_colors(&self, phenotype_colors: Option<ColorArrayRef>) -> Self {
        Self {
            segment_count: self.segment_count,
            arrays: self.arrays.clone(),
            phenotype_colors,
            size_rect: self.size_rect.clone(),
            mass: self.mass,
            inertia: self.inertia,
            eyes: self.eyes,
            update_effects: self.update_effects,
            growth_ratio: self.growth_ratio,
        }
    }

    /// Read only at the source array access; null paint remains a valid returned value
    pub fn color(&self, segment: usize) -> Result<Option<JavaColor>, GeometryError> {
        Self::read_color(self.phenotype_colors.as_ref(), segment)
    }

    fn read_color(
        array: Option<&ColorArrayRef>,
        segment: usize,
    ) -> Result<Option<JavaColor>, GeometryError> {
        let array = array.ok_or(GeometryError::MissingColorArray)?;
        array.get(segment).ok_or(GeometryError::ColorIndex {
            segment,
            length: array.len(),
        })
    }

    fn read_required_color(
        array: Option<&ColorArrayRef>,
        segment: usize,
    ) -> Result<JavaColor, GeometryError> {
        Self::read_color(array, segment)?.ok_or(GeometryError::MissingPhenotypeColor { segment })
    }

    pub fn required_color(&self, segment: usize) -> Result<JavaColor, GeometryError> {
        Self::read_required_color(self.phenotype_colors.as_ref(), segment)
    }

    /// Explicit value observation after an actual Color dereference
    pub fn color_argb(&self, segment: usize) -> Result<u32, GeometryError> {
        Ok(self.required_color(segment)?.argb())
    }

    pub fn color_kind(&self, segment: usize) -> Result<ColorKind, GeometryError> {
        Ok(ColorKind::from_argb(self.color_argb(segment)?))
    }

    /// The value argument is evaluated before these destination checks
    pub fn set_color(&self, segment: usize, value: Option<JavaColor>) -> Result<(), GeometryError> {
        Self::store_color(self.phenotype_colors.clone(), segment, value)
    }

    pub(crate) fn store_color(
        array: Option<ColorArrayRef>,
        segment: usize,
        value: Option<JavaColor>,
    ) -> Result<(), GeometryError> {
        let array = array.ok_or(GeometryError::MissingColorArray)?;
        array
            .replace(segment, value)
            .map_err(|length| GeometryError::ColorIndex { segment, length })
    }

    /// Explicit compatibility input creates a fresh Color rather than interning its ARGB
    pub fn set_color_argb(&self, segment: usize, argb: u32) -> Result<(), GeometryError> {
        self.set_color(segment, Some(JavaColor::from_argb(argb)))
    }

    /// Constructor state has no created arrays; this is independent of the supplied genotype
    pub(crate) fn uncreated() -> Self {
        Self {
            segment_count: 0,
            arrays: crate::OrganismArrays::default(),
            phenotype_colors: None,
            size_rect: Some(crate::RectangleRef::default()),
            mass: 0.0,
            inertia: 0.0,
            eyes: EyeState::default(),
            update_effects: 0,
            growth_ratio: 0,
        }
    }

    /// Publish the Color array before source per-slot stores and EYE dereferences
    pub(crate) fn create_arrays(&mut self, genome: &Genome) -> Result<(), GeometryError> {
        let (count, total) = segment_dimensions(genome)?;
        self.segment_count = total as i32;
        let mut colors = Vec::new();
        colors
            .try_reserve_exact(total)
            .map_err(|_| GeometryError::Capacity)?;
        colors.resize_with(total, || None);
        self.phenotype_colors = Some(ColorArrayRef::from_slots(colors));
        for channel in [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 24, 25, 29,
            31, 34, 26, 27, 32, 33, 28, 35, 30, 23, 9, 10,
        ] {
            self.arrays
                .allocate_int(crate::IntField::Reaction(channel), total)?;
        }
        for index in 0..total {
            let gene_ref = genome.gene_ref(index % count)?;
            let gene = gene_ref.borrow();
            self.set_color(index, gene.color_handle().cloned())?;
            let color = self.color(index)?.ok_or(GeometryError::MissingColor {
                gene: index % count,
            })?;
            if color.argb() == EYE {
                self.eyes.enabled = true;
            }
            for channel in crate::organism_arrays::REACTION_STORE_ORDER {
                self.set_reaction(index, channel, gene.reactions()[channel])?;
            }
        }
        self.allocate_geometry(total)
    }

    fn allocate_geometry(&mut self, total: usize) -> Result<(), GeometryError> {
        use crate::{DoubleField::*, IntField::*};
        for field in [StartX, StartY, EndX, EndY] {
            self.arrays.allocate_int(field, total)?;
        }
        for field in [StartRadius, EndRadius, Length, Photo] {
            self.arrays.allocate_double(field, total)?;
        }
        for field in [X1, Y1, X2, Y2] {
            self.arrays.allocate_int(field, total)?;
        }
        Ok(())
    }

    /// Organism.create and symmetric before initial color aggregate classification
    pub(crate) fn newborn(genome: &Genome) -> Self {
        Self::at_growth(genome, 16).expect("Newborn genotype must have valid geometry")
    }

    pub fn at_growth(genome: &Genome, growth_ratio: i32) -> Result<Self, GeometryError> {
        let mut body = Self::uncreated();
        body.create_arrays(genome)?;
        for index in 0..genome.gene_count()? {
            let gene_ref = genome.gene_ref(index)?;
            let gene = gene_ref.borrow();
            validate_gene_scalars(&gene, index)?;
            if index > 0 && (gene.branch() < -1 || gene.branch() as i64 > index as i64) {
                return Err(GeometryError::InvalidBranch { gene: index });
            }
        }
        if !(1..=16).contains(&growth_ratio) {
            return Err(GeometryError::InvalidGrowth);
        }
        body.rebuild(genome, growth_ratio, body.eyes, 0)
    }

    /// Cached rebuilding retains the actual array and completed Color/eye/mass prefixes
    pub fn rebuild(
        &mut self,
        genome: &Genome,
        growth_ratio: i32,
        eyes: EyeState,
        update_effects: i32,
    ) -> Result<Self, GeometryError> {
        self.rebuild_in_place(genome, growth_ratio, eyes, update_effects)?;
        Ok(self.copy_with_colors(self.phenotype_colors.clone()))
    }

    pub(crate) fn rebuild_in_place(
        &mut self,
        genome: &Genome,
        growth_ratio: i32,
        eyes: EyeState,
        update_effects: i32,
    ) -> Result<(), GeometryError> {
        use crate::{DoubleField::*, IntField::*};
        let symmetry = genome.symmetry();
        if symmetry == 0 {
            return Err(GeometryError::InvalidSymmetry);
        }
        let sequence = self.segment_count.wrapping_div(symmetry);
        self.eyes = eyes;
        if update_effects == 0 {
            self.update_effects = 1;
        } else {
            self.update_effects = update_effects;
        }
        self.mass = 0.0;
        self.growth_ratio = growth_ratio;
        let mut theta = 0.0;
        let (mut left, mut right, mut top, mut bottom) = (0_i32, 0_i32, 0_i32, 0_i32);
        let mut segment = 0_i32;
        for repetition in 0..symmetry {
            for index in 0..sequence {
                let index = index as usize;
                let eye = self.eyes.enabled && self.required_color(index)?.argb() == EYE;
                let gene_ref = genome.gene_ref(index)?;
                let gene = gene_ref.borrow();
                let scale = GROWTH_SCALES[growth_index(growth_ratio)?];
                let length = if eye {
                    (if self.eyes.good_vision { 2.0 } else { 1.5 }) * (gene.length() * scale)
                } else {
                    gene.length() * scale
                };
                let value = if eye {
                    (if self.eyes.consumer || self.eyes.fungus || self.eyes.infectious {
                        0.026
                    } else {
                        0.025
                    }) * length
                } else {
                    length
                };
                self.arrays.write_double(Length, segment, value)?;
                self.mass += self.arrays.read_double(Length, segment)?;
                let reflected = genome.mirrored() && repetition % 2 != 0;
                if index == 0 || gene.branch() == 0 {
                    self.arrays.write_int(StartX, segment, 0)?;
                    self.arrays.write_int(StartY, segment, 0)?;
                    theta = vector_start(genome, repetition as usize, index)?;
                } else {
                    let parent = if gene.branch() == -1 {
                        segment.wrapping_sub(1)
                    } else {
                        repetition
                            .wrapping_mul(sequence)
                            .wrapping_add(gene.branch())
                            .wrapping_sub(1)
                    };
                    let destination = self.arrays.start_x.clone();
                    let value = self.arrays.read_int(EndX, parent)?;
                    crate::OrganismArrays::store_int(destination, StartX, segment, value)?;
                    let destination = self.arrays.start_y.clone();
                    let value = self.arrays.read_int(EndY, parent)?;
                    crate::OrganismArrays::store_int(destination, StartY, segment, value)?;
                    if self.eyes.enabled && self.required_color(parent as usize)?.argb() == EYE {
                        self.set_color(parent as usize, ColorKind::Vision.color_ref())?;
                        self.eyes.good_vision = true;
                    }
                    theta += if reflected {
                        -gene.theta()
                    } else {
                        gene.theta()
                    };
                }
                let destination = self.arrays.end_x.clone();
                let value = java_round(
                    length * theta.cos() + self.arrays.read_int(StartX, segment)? as f64,
                );
                crate::OrganismArrays::store_int(destination, EndX, segment, value)?;
                let destination = self.arrays.end_y.clone();
                let value = java_round(
                    length * theta.sin() + self.arrays.read_int(StartY, segment)? as f64,
                );
                crate::OrganismArrays::store_int(destination, EndY, segment, value)?;
                left = left.min(self.arrays.read_int(EndX, segment)?);
                right = right.max(self.arrays.read_int(EndX, segment)?);
                top = top.min(self.arrays.read_int(EndY, segment)?);
                bottom = bottom.max(self.arrays.read_int(EndY, segment)?);
                segment = segment.wrapping_add(1);
            }
        }
        crate::rectangles::required_rectangle(self.size_rect.as_ref(), "_sizeRect")?.set(IntRect {
            x: left,
            y: top,
            width: right.wrapping_sub(left).wrapping_add(1),
            height: bottom.wrapping_sub(top).wrapping_add(1),
        });
        let center = [left.wrapping_add(right) >> 1, top.wrapping_add(bottom) >> 1];
        self.mass = 0.001 * java_round_long(1000.0 * self.mass) as f64;
        self.inertia = 0.0;
        for index in 0..self.segment_count {
            for (field, offset) in [
                (StartX, center[0]),
                (StartY, center[1]),
                (EndX, center[0]),
                (EndY, center[1]),
            ] {
                let value = self.arrays.read_int(field, index)?.wrapping_sub(offset);
                self.arrays.write_int(field, index, value)?;
            }
            let destination = self.arrays.start_radius.clone();
            let x = self.arrays.read_int(StartX, index)?;
            let y = self.arrays.read_int(StartY, index)?;
            crate::OrganismArrays::store_double(
                destination,
                StartRadius,
                index,
                (x.wrapping_mul(x).wrapping_add(y.wrapping_mul(y)) as f64).sqrt(),
            )?;
            let destination = self.arrays.end_radius.clone();
            let x = self.arrays.read_int(EndX, index)?;
            let y = self.arrays.read_int(EndY, index)?;
            crate::OrganismArrays::store_double(
                destination,
                EndRadius,
                index,
                (x.wrapping_mul(x).wrapping_add(y.wrapping_mul(y)) as f64).sqrt(),
            )?;
            let cx = self
                .arrays
                .read_int(StartX, index)?
                .wrapping_add(self.arrays.read_int(EndX, index)?) as f64
                / 2.0;
            let cy = self
                .arrays
                .read_int(StartY, index)?
                .wrapping_add(self.arrays.read_int(EndY, index)?) as f64
                / 2.0;
            self.inertia += java_math::pow3(self.arrays.read_double(Length, index)?) / 12.0
                + self.arrays.read_double(Length, index)? * cx * cx
                + cy * cy;
        }
        Ok(())
    }

    /// ARGB-only fixture and API input constructs independent Color objects
    pub fn from_phenotype(
        genome: &Genome,
        growth_ratio: i32,
        colors: &[u32],
        eyes: EyeState,
        update_effects: i32,
    ) -> Result<Self, GeometryError> {
        Self::from_segment_state_inner(genome, growth_ratio, colors, None, eyes, update_effects)
    }

    /// Geometry and reactions retain their existing complete-input boundary
    pub fn from_segment_state(
        genome: &Genome,
        growth_ratio: i32,
        colors: &[u32],
        reactions: &[[i32; 36]],
        eyes: EyeState,
        update_effects: i32,
    ) -> Result<Self, GeometryError> {
        Self::from_segment_state_inner(
            genome,
            growth_ratio,
            colors,
            Some(reactions),
            eyes,
            update_effects,
        )
    }

    fn from_segment_state_inner(
        genome: &Genome,
        growth_ratio: i32,
        colors: &[u32],
        reactions: Option<&[[i32; 36]]>,
        eyes: EyeState,
        update_effects: i32,
    ) -> Result<Self, GeometryError> {
        if colors.len() != segment_dimensions(genome)?.1 {
            return Err(GeometryError::PhenotypeLength);
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(colors.len())
            .map_err(|_| GeometryError::Capacity)?;
        slots.extend(colors.iter().map(|argb| Some(JavaColor::from_argb(*argb))));
        for index in 0..genome.gene_count()? {
            let gene_ref = genome.gene_ref(index)?;
            let gene = gene_ref.borrow();
            if !gene.length().is_finite() || !gene.theta().is_finite() {
                return Err(GeometryError::NonfiniteGene { gene: index });
            }
            if index > 0 && (gene.branch() < -1 || gene.branch() as i64 > index as i64) {
                return Err(GeometryError::InvalidBranch { gene: index });
            }
        }
        if !(1..=16).contains(&growth_ratio) {
            return Err(GeometryError::InvalidGrowth);
        }
        let total = slots.len();
        if reactions.is_some_and(|v| v.len() != total) {
            return Err(GeometryError::ReactionLength);
        }
        let mut body = Self::uncreated();
        body.segment_count = total as i32;
        body.phenotype_colors = Some(ColorArrayRef::from_slots(slots));
        for channel in 0..36 {
            body.arrays
                .allocate_int(crate::IntField::Reaction(channel), total)?;
        }
        for index in 0..total {
            let values = match reactions {
                Some(values) => values[index],
                None => *genome
                    .gene_ref(index % genome.gene_count()?)?
                    .borrow()
                    .reactions(),
            };
            for (channel, value) in values.into_iter().enumerate() {
                body.set_reaction(index, channel, value)?;
            }
        }
        body.allocate_geometry(total)?;
        body.rebuild(genome, growth_ratio, eyes, update_effects)
    }

    pub fn rotated(&self, angle: f64, center: [f64; 2]) -> Result<RotatedBody, GeometryError> {
        if !angle.is_finite() || !center.iter().all(|number| number.is_finite()) {
            return Err(GeometryError::NonfiniteTransform);
        }
        self.rotated_for_simulation(angle, center)
    }

    /// Internal state can retain nonfinite results from valid zero-mass bodies
    pub(crate) fn rotated_for_simulation(
        &self,
        angle: f64,
        center: [f64; 2],
    ) -> Result<RotatedBody, GeometryError> {
        let mut segments = Vec::new();
        segments
            .try_reserve_exact(self.forward_count())
            .map_err(|_| GeometryError::Capacity)?;
        for index in self.indices() {
            let start = rotate_cached(
                [self.start(index, 0)?, self.start(index, 1)?],
                self.start_radius(index)?,
                angle,
            );
            let end = rotate_cached(
                [self.end(index, 0)?, self.end(index, 1)?],
                self.end_radius(index)?,
                angle,
            );
            segments.push(RotatedSegment { start, end });
        }
        let bounds = bounds_from_rotated(&segments, center);
        Ok(RotatedBody {
            segments,
            center: [center[0] as i32, center[1] as i32],
            bounds,
        })
    }
}

/// Recalculate bounds from cached endpoints without changing their rotation
pub(crate) fn bounds_from_rotated(segments: &[RotatedSegment], center: [f64; 2]) -> IntRect {
    let (mut left, mut top) = (f64::MAX, f64::MAX);
    // Double.MIN_VALUE is the smallest positive value, not the negative maximum
    let (mut right, mut bottom) = (f64::from_bits(1), f64::from_bits(1));
    for segment in segments.iter().rev() {
        left = java_math::min(
            java_math::min(left, f64::from(segment.start[0]) + center[0]),
            f64::from(segment.end[0]) + center[0],
        );
        right = java_math::max(
            java_math::max(right, f64::from(segment.start[0]) + center[0]),
            f64::from(segment.end[0]) + center[0],
        );
        top = java_math::min(
            java_math::min(top, f64::from(segment.start[1]) + center[1]),
            f64::from(segment.end[1]) + center[1],
        );
        bottom = java_math::max(
            java_math::max(bottom, f64::from(segment.start[1]) + center[1]),
            f64::from(segment.end[1]) + center[1],
        );
    }
    IntRect {
        x: left as i32,
        y: top as i32,
        width: ((right - left + 1.0) as i32).wrapping_add(1),
        height: ((bottom - top + 1.0) as i32).wrapping_add(1),
    }
}

impl GenomePreview {
    pub fn new(genome: &Genome, width: i32, height: i32) -> Result<Self, GeometryError> {
        if width <= 0 || height <= 0 {
            return Err(GeometryError::InvalidViewport);
        }
        let (count, total) = preview_dimensions(genome)?;
        let mut segments = Vec::<RotatedSegment>::new();
        segments
            .try_reserve_exact(total)
            .map_err(|_| GeometryError::Capacity)?;
        // GeneticCode.draw reads current and future slots from zero-filled endpoint arrays
        segments.resize(
            total,
            RotatedSegment {
                start: [0; 2],
                end: [0; 2],
            },
        );
        let mut colors = Vec::new();
        colors
            .try_reserve_exact(total)
            .map_err(|_| GeometryError::Capacity)?;
        let (mut left, mut right, mut top, mut bottom) = (0_i32, 0_i32, 0_i32, 0_i32);
        let mut theta = 0.0;
        for repetition in 0..genome.symmetry() as usize {
            for index in 0..count {
                let gene_ref = genome.gene_ref(index)?;
                let gene = gene_ref.borrow();
                let segment_index = repetition * count + index;
                let start = if index == 0 || gene.branch() == 0 {
                    theta = vector_start(genome, repetition, index)?;
                    [0, 0]
                } else {
                    let parent = if gene.branch() == -1 {
                        segment_index - 1
                    } else {
                        repetition * count + gene.branch() as usize - 1
                    };
                    theta += if genome.mirrored() && repetition % 2 != 0 {
                        -gene.theta()
                    } else {
                        gene.theta()
                    };
                    segments[parent].end
                };
                let end = [
                    java_round(gene.length() * theta.cos() + start[0] as f64),
                    java_round(gene.length() * theta.sin() + start[1] as f64),
                ];
                left = left.min(start[0]).min(end[0]);
                right = right.max(start[0]).max(end[0]);
                top = top.min(start[1]).min(end[1]);
                bottom = bottom.max(start[1]).max(end[1]);
                segments[segment_index] = RotatedSegment { start, end };
                colors.push(gene.color().expect("Color was validated"));
            }
        }
        let mut scale = 1.0_f64;
        let dx = right.wrapping_sub(left);
        let dy = bottom.wrapping_sub(top);
        if dx > width {
            scale = width as f64 / dx as f64;
        }
        if dy > height {
            scale = scale.min(height as f64 / dy as f64);
        }
        let shift = [
            left.wrapping_neg().wrapping_sub(right) / 2,
            top.wrapping_neg().wrapping_sub(bottom) / 2,
        ];
        for segment in &mut segments {
            for axis in 0..2 {
                segment.start[axis] = segment.start[axis].wrapping_add(shift[axis]);
                segment.end[axis] = segment.end[axis].wrapping_add(shift[axis]);
            }
        }
        Ok(Self {
            segments,
            colors,
            translation: [width / 2, height / 2],
            scale,
        })
    }
}

fn rotate_cached(point: [i32; 2], distance: f64, angle: f64) -> [i32; 2] {
    let theta = angle + java_math::atan2(point[1] as f64, point[0] as f64);
    [
        (distance * theta.cos()) as i32,
        (distance * theta.sin()) as i32,
    ]
}

/// Match calculateBounds' cached polar rotation and truncating casts
pub(crate) fn rotate(point: [i32; 2], angle: f64) -> [i32; 2] {
    rotate_cached(point, radius(point), angle)
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../tests/geometry/cases.rs");

    #[test]
    fn java_rounding_preserves_negative_half_boundaries() {
        for (input, expected) in [(-2.5, -2), (-1.5, -1), (-0.5, 0), (0.5, 1), (1.5, 2)] {
            assert_eq!(java_round(input), expected);
        }
        assert_eq!(rotate([0, 0], PI), [0, 0]);
        assert_eq!(java_round(f64::from_bits(0.5_f64.to_bits() - 1)), 0);
        assert_eq!(java_round(f64::from_bits((-0.5_f64).to_bits() + 1)), -1);
        let mut checks = 0;
        for line in include_str!("../tests/fixtures/construction.tsv").lines() {
            let fields: Vec<_> = line.split('\t').collect();
            if fields[0] == "Q" {
                let coordinate = f64::from_bits(u64::from_str_radix(fields[1], 16).unwrap());
                assert_eq!(java_round(coordinate), fields[2].parse::<i32>().unwrap());
                checks += 1;
            }
        }
        assert_eq!(checks, 9);
    }
}
