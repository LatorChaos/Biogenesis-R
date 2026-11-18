// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Shared genotype clipboard references and isolated laboratory drafts

use super::*;
use crate::{Gene, GeneticsError, GenomeModifiers, GenomePreview, GenomeProperties};

/// Cloning retains the exact genotype object held by an actor or clipboard
#[derive(Clone)]
pub struct GenomeHandle(GenomeRef);

impl fmt::Debug for GenomeHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GenomeHandle(..)")
    }
}

impl PartialEq for GenomeHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for GenomeHandle {}

impl GenomeHandle {
    pub(crate) fn from_shared(genome: GenomeRef) -> Self {
        Self(genome)
    }

    pub fn from_owned(genome: Genome) -> Self {
        Self(Rc::new(RefCell::new(genome)))
    }

    /// Copy every Gene slot independently, retaining values and lineage in the detached result
    pub fn detached_clone(&self) -> Genome {
        self.0.borrow().clone()
    }

    pub fn draft(&self) -> Result<GenomeDraft, GeneticsError> {
        Ok(GenomeDraft {
            genome: self.0.borrow().java_clone()?,
        })
    }

    pub fn preview(&self, width: i32, height: i32) -> Result<GenomePreview, GeometryError> {
        GenomePreview::new(&self.0.borrow(), width, height)
    }

    /// Render current shared genotype values without rebuilding an organism or advancing RNG
    /// The budget covers RGB8 pixels and temporary preview geometry, excluding allocator overhead
    pub fn preview_image(
        &self,
        width: i32,
        height: i32,
        budget_bytes: usize,
    ) -> Result<OrganismImage, PreviewImageError> {
        preview_image::from_genome(&self.0.borrow(), width, height, budget_bytes)
    }

    /// Read lineage metadata without cloning genes or replacing shared identities
    pub fn lineage(&self) -> (Option<crate::JavaText>, i32) {
        let genome = self.0.borrow();
        (genome.clade_id().cloned(), genome.generation())
    }
}

/// An editable local copy; no draft Gene reference reaches the simulation or clipboard
pub struct GenomeDraft {
    genome: Genome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenomeEstimates {
    pub segments: i32,
    pub energy: i32,
    pub life: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftError {
    InvalidIndex { index: usize, count: usize },
    AllocationFailed,
    InvalidAngleDegrees { degrees: i32 },
}

impl fmt::Display for DraftError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIndex { index, count } => {
                write!(
                    formatter,
                    "Gene index {index} is outside {count} editable slots"
                )
            }
            Self::AllocationFailed => formatter.write_str("Cannot allocate another editable gene"),
            Self::InvalidAngleDegrees { degrees } => {
                write!(
                    formatter,
                    "Gene angle must be between zero and 359 degrees, received {degrees}"
                )
            }
        }
    }
}

impl std::error::Error for DraftError {}

impl GenomeDraft {
    pub fn properties(&self) -> GenomeProperties {
        self.genome.properties()
    }

    /// Return a detached batch containing every Gene field and reaction
    pub fn genes(&self) -> Vec<Gene> {
        self.genome
            .genes()
            .expect("Draft construction validates nullable source slots")
            .iter()
            .map(|gene| gene.borrow().clone())
            .collect()
    }

    /// Preserve raw explicit-constructor values; interactive controls impose their own source bounds
    pub fn set_properties(&mut self, properties: GenomeProperties) {
        self.genome.set_properties(properties);
    }

    fn references(&self) -> Vec<crate::GeneRef> {
        self.genome
            .genes()
            .expect("Only validated private draft owners are editable")
    }

    fn editable_gene(&self, index: usize) -> Result<crate::GeneRef, DraftError> {
        let count = self.genome.gene_count().expect("Draft array is present");
        if index >= count {
            return Err(DraftError::InvalidIndex { index, count });
        }
        Ok(self
            .genome
            .gene_ref(index)
            .expect("Draft construction validates Gene slots"))
    }

    pub fn replace_gene(&mut self, index: usize, gene: Gene) -> Result<(), DraftError> {
        *self.editable_gene(index)?.borrow_mut() = gene;
        Ok(())
    }

    /// Reading the initial angle display must not replace the stored raw radians
    pub fn gene_angle_degrees(&self, index: usize) -> Result<i32, DraftError> {
        Ok(
            crate::geometry::java_round(self.editable_gene(index)?.borrow().theta() * 180.0 / PI)
                .rem_euclid(360),
        )
    }

    pub fn set_gene_angle_degrees(&mut self, index: usize, degrees: i32) -> Result<(), DraftError> {
        let gene = self.editable_gene(index)?;
        if !(0..=359).contains(&degrees) {
            return Err(DraftError::InvalidAngleDegrees { degrees });
        }
        gene.borrow_mut().set_theta(f64::from(degrees) * PI / 180.0);
        Ok(())
    }

    /// Resizing replaces the detached draft holder rather than mutating a shared runtime array
    pub fn insert_gene(&mut self, index: usize) -> Result<(), DraftError> {
        let mut genes = self.references();
        let count = genes.len();
        if index > count {
            return Err(DraftError::InvalidIndex { index, count });
        }
        genes
            .try_reserve(1)
            .map_err(|_| DraftError::AllocationFailed)?;
        genes.insert(
            index,
            Gene::new(2.0, 0.0, crate::ColorKind::Green.argb(), -1, [0; 36]).shared(),
        );
        self.genome
            .set_gene_array(Some(crate::GeneArrayRef::from_slots(
                genes.into_iter().map(Some).collect(),
            )));
        Ok(())
    }

    pub fn remove_gene(&mut self, index: usize) -> Result<(), DraftError> {
        let mut genes = self.references();
        let count = genes.len();
        if index >= count {
            return Err(DraftError::InvalidIndex { index, count });
        }
        genes.remove(index);
        self.genome
            .set_gene_array(Some(crate::GeneArrayRef::from_slots(
                genes.into_iter().map(Some).collect(),
            )));
        Ok(())
    }

    pub fn clear(&mut self) {
        self.genome
            .set_gene_array(Some(crate::GeneArrayRef::from_slots(Vec::new())));
    }

    fn preview_genome(&self) -> Genome {
        let current = self.genome.modifiers();
        let properties = GenomeProperties {
            modifiers: GenomeModifiers {
                cream: current.cream,
                fallow: current.fallow,
                spore: current.spore,
                adapt_spore: current.adapt_spore,
                ..GenomeModifiers::default()
            },
            ..self.genome.properties()
        };
        Genome::from_parts(&self.references(), properties)
    }

    pub fn preview(&self, width: i32, height: i32) -> Result<GenomePreview, GeometryError> {
        GenomePreview::new(&self.preview_genome(), width, height)
    }

    /// Render the Laboratory's temporary genotype with a shared pixels-and-geometry budget
    pub fn preview_image(
        &self,
        width: i32,
        height: i32,
        budget_bytes: usize,
    ) -> Result<OrganismImage, PreviewImageError> {
        preview_image::from_genome(&self.preview_genome(), width, height, budget_bytes)
    }

    /// An empty commit leaves the caller's clipboard unchanged; nonempty commits start fresh lineage
    pub fn commit(self) -> Option<GenomeHandle> {
        if self.references().is_empty() {
            return None;
        }
        Some(GenomeHandle::from_owned(Genome::from_parts(
            &self.references(),
            self.genome.properties(),
        )))
    }
}

impl World {
    /// Read current mutation settings as a detached snapshot
    pub fn genetics_parameters(&self) -> GeneticsParams {
        self.session.0.parameters.borrow().genetics.clone()
    }

    /// Empty laboratories use fixed local defaults and the current configured mutation midpoint
    pub fn new_genome_draft(
        &self,
        source: Option<&GenomeHandle>,
    ) -> Result<GenomeDraft, GeneticsError> {
        if let Some(source) = source {
            return source.draft();
        }
        let genetics = &self.session.0.parameters.borrow().genetics;
        Ok(GenomeDraft {
            genome: Genome::from_parts(
                &[],
                GenomeProperties {
                    symmetry: 2,
                    mirror: 0,
                    mutation_rate: genetics
                        .min_mutation_rate
                        .wrapping_add(genetics.max_mutation_rate)
                        / 2,
                    activity: 2,
                    modifiers: GenomeModifiers {
                        cream: 2,
                        fallow: 2,
                        spore: 4,
                        adapt_spore: 4,
                        ..GenomeModifiers::default()
                    },
                },
            ),
        })
    }

    /// Match the laboratory's integer labels without invoking organism construction
    pub fn genome_draft_estimates(
        &self,
        draft: &GenomeDraft,
    ) -> Result<GenomeEstimates, WorldError> {
        let divisor = self.session.0.parameters.borrow().effects.age_divisor;
        if divisor == 0 {
            return Err(WorldError::Arithmetic(
                "Genome estimate age divisor is zero",
            ));
        }
        let segments = (draft.genome.gene_count()? as i32).wrapping_mul(draft.genome.symmetry());
        Ok(GenomeEstimates {
            segments,
            energy: 40_i32.wrapping_add(3_i32.wrapping_mul(segments)),
            life: self.session.0.parameters.borrow().effects
                .max_age
                .wrapping_add(segments.wrapping_div(divisor)),
        })
    }

    /// Outer absence rejects eligibility; inner absence retains an eligible null primary
    pub fn copy_genome(&self, actor: &OrganismHandle) -> Option<Option<GenomeHandle>> {
        if !self.contains_handle(actor) {
            return None;
        }
        let actor = actor.0.borrow();
        actor.state.alive.then(|| {
            actor
                .genome
                .as_ref()
                .map(|code| GenomeHandle(Rc::clone(code)))
        })
    }

    /// Inspection distinguishes a missing member from its actual nullable primary reference
    pub fn inspect_genome(&self, actor: &OrganismHandle) -> Option<Option<GenomeHandle>> {
        self.contains_handle(actor).then(|| {
            actor
                .0
                .borrow()
                .genome
                .as_ref()
                .map(|code| GenomeHandle(Rc::clone(code)))
        })
    }

    /// Use the exact supplied genotype, preserving failed placement's real RNG and metadata changes
    pub fn paste_genome_handle(
        &self,
        genome: &GenomeHandle,
        center: [i32; 2],
    ) -> Result<Option<OrganismHandle>, WorldError> {
        let Some(actor) = self.construct_pasted_shared(Rc::clone(&genome.0), center)? else {
            return Ok(None);
        };
        let reference = Rc::new(RefCell::new(actor));
        let handle = OrganismHandle(Rc::clone(&reference), self.handle());
        self.add_organism(reference, None)?;
        Ok(Some(handle))
    }

    /// RandomCreateAction draws its genotype before orientation, unlike ordinary genesis
    pub fn random_genome_at(
        &self,
        center: [i32; 2],
    ) -> Result<Option<OrganismHandle>, WorldError> {
        let genome = Genome::random_with(&mut self.session.0.random.borrow_mut(), &self.session.0.parameters.borrow().genetics)
            .map_err(|error| WorldError::InvalidParameters(error.to_string()))?;
        self.paste_genome_handle(&GenomeHandle::from_owned(genome), center)
    }
}

#[cfg(test)]
#[path = "laboratory_tests.rs"]
mod tests;
