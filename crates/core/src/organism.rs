// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::{
    Body, ColorArrayRef, EffectsParams, EffectsState, Gene, GeneRef, Genome, GeometryError,
    IntRect, JavaColor, JavaText, ReproductionState,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// Shared genotype identity; source object comparisons require Rc::ptr_eq rather than value equality
pub(crate) type GenomeRef = Rc<RefCell<Genome>>;

/// Read an actual primary reference at the source dereference boundary
pub(crate) fn primary_genome(code: &Option<GenomeRef>) -> Result<&GenomeRef, GeometryError> {
    code.as_ref().ok_or(GeometryError::MissingGenome)
}

/// Java reference equality includes two null references and excludes equal-valued distinct objects
pub(crate) fn same_genome(left: Option<&GenomeRef>, right: Option<&GenomeRef>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
        _ => false,
    }
}

/// Internal organism identity remains independent of the wrapping legacy identifier
pub(crate) type OrganismRef = Rc<RefCell<Organism>>;

#[derive(Default)]
pub(crate) struct OrganismCloner {
    organisms: HashMap<*const RefCell<Organism>, OrganismRef>,
    pub(crate) worlds: HashMap<usize, std::rc::Weak<crate::world::WorldState>>,
    genomes: GenomeCloner,
    color_arrays: HashMap<usize, ColorArrayRef>,
    int_arrays: HashMap<usize, crate::IntArrayRef>,
    double_arrays: HashMap<usize, crate::DoubleArrayRef>,
    rectangles: HashMap<usize, crate::RectangleRef>,
}

impl OrganismCloner {
    pub(crate) fn copied_actors(&self) -> impl Iterator<Item = &OrganismRef> {
        self.organisms.values()
    }

    pub(crate) fn genome_cloner(&mut self) -> &mut GenomeCloner {
        &mut self.genomes
    }

    pub fn copy(&mut self, original: &OrganismRef) -> OrganismRef {
        let identity = Rc::as_ptr(original);
        if let Some(copied) = self.organisms.get(&identity) {
            return Rc::clone(copied);
        }
        let copied = Rc::new(RefCell::new(original.borrow().copy_for_world_with_arrays(
            &mut self.genomes,
            &mut self.color_arrays,
            &mut self.int_arrays,
            &mut self.double_arrays,
            &mut self.rectangles,
        )));
        if !self.worlds.is_empty() { copied.borrow_mut().state.world.retarget(&self.worlds); }
        self.organisms.insert(identity, Rc::clone(&copied));
        copied
            .borrow()
            .state
            .bounds
            .bind_actor(&copied)
            .expect("Copied inherited Rectangle has one actor");
        let links = {
            let original = original.borrow();
            [
                Some(original.state.bounds.clone()),
                original.body.size_rect.clone(),
                original.state.last_frame.clone(),
            ]
        };
        for link in links.into_iter().flatten() {
            if let Some(target) = link.actor_target() {
                let target = self.copy(&target);
                self.rectangles[&link.identity()]
                    .bind_actor(&target)
                    .expect("Copied actor association is injective");
            }
        }
        copied
    }
}

#[derive(Default)]
pub(crate) struct GenomeCloner {
    genomes: HashMap<*const RefCell<Genome>, GenomeRef>,
    genes: HashMap<*const RefCell<Gene>, GeneRef>,
    arrays: HashMap<usize, crate::GeneArrayRef>,
}

impl GenomeCloner {
    pub fn copy(&mut self, original: &GenomeRef) -> GenomeRef {
        let identity = Rc::as_ptr(original);
        if let Some(copied) = self.genomes.get(&identity) {
            return Rc::clone(copied);
        }
        let original = original.borrow();
        let array = original.gene_array().map(|array| {
            if let Some(copied) = self.arrays.get(&array.identity()) {
                return copied.clone();
            }
            let slots = array
                .slots()
                .iter()
                .map(|slot| {
                    slot.as_ref().map(|gene| {
                        Rc::clone(
                            self.genes
                                .entry(Rc::as_ptr(gene))
                                .or_insert_with(|| gene.borrow().clone().shared()),
                        )
                    })
                })
                .collect();
            let copied = crate::GeneArrayRef::from_slots(slots);
            self.arrays.insert(array.identity(), copied.clone());
            copied
        });
        let mut copied = original.with_array(array);
        copied.set_generation(original.generation());
        copied.set_clade_text(original.clade_id().cloned());
        copied.set_update_clade(original.update_clade());
        let copied = Rc::new(RefCell::new(copied));
        self.genomes.insert(identity, Rc::clone(&copied));
        copied
    }
}

/// Non-genetic organism state, independent of the frontend and derived drawing data
#[derive(Debug, PartialEq)]
pub struct OrganismState {
    pub world: crate::world::WorldAssociation,
    pub parent_id: i32,
    pub children: i32,
    pub virus_children: i32,
    pub total_children: i32,
    pub total_kills: i32,
    pub total_infected: i32,
    pub network_position: Option<JavaText>,
    pub center: [f64; 2],
    pub integer_center: [i32; 2],
    pub angle: f64,
    pub last_angle: f64,
    pub velocity: [f64; 2],
    pub angular_velocity: f64,
    pub energy: f64,
    pub age: i32,
    pub temporary_color: Option<JavaColor>,
    pub temporary_color_frames: i32,
    pub reproduction: ReproductionState,
    pub has_grown: i32,
    pub has_moved: bool,
    pub last_frame: Option<crate::RectangleRef>,
    pub bounds: crate::RectangleRef,
    pub alive: bool,
    pub active: bool,
}

impl OrganismState {
    pub(crate) fn copy_with_rectangles(
        &self,
        owners: &mut HashMap<usize, crate::RectangleRef>,
    ) -> Self {
        Self {
            world: self.world.clone(),
            parent_id: self.parent_id,
            children: self.children,
            virus_children: self.virus_children,
            total_children: self.total_children,
            total_kills: self.total_kills,
            total_infected: self.total_infected,
            network_position: self.network_position.clone(),
            center: self.center,
            integer_center: self.integer_center,
            angle: self.angle,
            last_angle: self.last_angle,
            velocity: self.velocity,
            angular_velocity: self.angular_velocity,
            energy: self.energy,
            age: self.age,
            temporary_color: self.temporary_color.clone(),
            temporary_color_frames: self.temporary_color_frames,
            reproduction: self.reproduction.clone(),
            has_grown: self.has_grown,
            has_moved: self.has_moved,
            last_frame: self.last_frame.as_ref().map(|value| value.copied(owners)),
            bounds: self.bounds.copied(owners),
            alive: self.alive,
            active: self.active,
        }
    }

    pub fn temporary_argb(&self) -> Option<u32> {
        self.temporary_color.as_ref().map(JavaColor::argb)
    }

    pub(crate) fn required_temporary_color(&self) -> Result<&JavaColor, GeometryError> {
        self.temporary_color
            .as_ref()
            .ok_or(GeometryError::MissingTemporaryColor)
    }
}

impl Clone for OrganismState {
    fn clone(&self) -> Self {
        self.copy_with_rectangles(&mut HashMap::new())
    }
}

impl Default for OrganismState {
    fn default() -> Self {
        Self {
            world: crate::world::WorldAssociation::default(),
            parent_id: 0,
            children: 0,
            virus_children: 0,
            total_children: 0,
            total_kills: 0,
            total_infected: 0,
            network_position: None,
            center: [0.0; 2],
            integer_center: [0; 2],
            angle: 0.0,
            last_angle: -1.0,
            velocity: [0.0; 2],
            angular_velocity: 0.0,
            energy: 0.0,
            age: 0,
            temporary_color: None,
            temporary_color_frames: 0,
            reproduction: ReproductionState::default(),
            has_grown: 0,
            has_moved: true,
            last_frame: Some(crate::RectangleRef::default()),
            bounds: crate::RectangleRef::actor(IntRect::default()),
            alive: true,
            active: true,
        }
    }
}

/// Detached inspection data; editing its genotype cannot mutate the world
#[derive(Clone, Debug, PartialEq)]
pub struct OrganismSnapshot {
    pub id: i64,
    pub genotype: Option<Genome>,
    pub infected_genotype: Option<Genome>,
    pub saved_genotype: Option<Genome>,
    pub state: OrganismState,
    pub effects: EffectsState,
    pub body: Body,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Organism {
    pub id: i64,
    pub genome: Option<GenomeRef>,
    pub infected_genome: Option<GenomeRef>,
    pub saved_genome: Option<GenomeRef>,
    pub body: Body,
    pub state: OrganismState,
    pub effects: EffectsState,
    pub creation_complete: bool,
}

impl Organism {
    /// Assign stored parts without constructors, events, or geometry calculations
    pub(crate) fn from_persistent_parts(
        id: i32,
        genome: Option<GenomeRef>,
        infected_genome: Option<GenomeRef>,
        saved_genome: Option<GenomeRef>,
        body: Body,
        state: OrganismState,
        effects: EffectsState,
    ) -> Self {
        Self {
            id: i64::from(id), genome, infected_genome, saved_genome, body, state, effects,
            creation_complete: true,
        }
    }

    pub(crate) fn set_display_color(&mut self, color: Option<JavaColor>) {
        self.state.temporary_color = color;
        self.state.temporary_color_frames = 10;
    }

    /// The real genotype remains attached before create allocates any segment geometry
    pub(crate) fn pasted_candidate(genome: Option<GenomeRef>, state: OrganismState) -> Self {
        Self {
            id: 0,
            genome,
            infected_genome: None,
            saved_genome: None,
            body: Body::uncreated(),
            state,
            effects: EffectsState::default(),
            creation_complete: false,
        }
    }

    /// Paste create replaces arrays without discarding retained classification and lifecycle state
    pub(crate) fn create_for_paste(&mut self) -> Result<(), GeometryError> {
        self.creation_complete = false;
        self.body
            .create_arrays(&primary_genome(&self.genome)?.borrow())?;
        self.creation_complete = true;
        self.rebuild_body()
    }

    pub(crate) fn unplaced(genome: GenomeRef, state: OrganismState) -> Result<Self, GeometryError> {
        let body = Body::at_growth(&genome.borrow(), 16)?;
        let effects = EffectsState::default();
        Ok(Self {
            id: 0,
            genome: Some(genome),
            infected_genome: None,
            saved_genome: None,
            body,
            state,
            effects,
            creation_complete: true,
        })
    }

    pub(crate) fn use_energy(&mut self, amount: f64) -> Result<bool, crate::WorldError> {
        if self.state.energy < amount { return Ok(false); }
        let target = self.state.world.resolve()?;
        let consumed = target.transfer_gas(amount, false);
        self.state.energy = crate::java_math::sub(self.state.energy, consumed);
        Ok(!(consumed < amount))
    }

    pub(crate) fn rebuild_body(&mut self) -> Result<(), GeometryError> {
        let eyes = crate::EyeState {
            consumer: self.effects.is_consumer,
            fungus: self.effects.is_fungus,
            infectious: self.effects.is_infectious,
            ..self.body.eyes
        };
        self.body.rebuild_in_place(
            &primary_genome(&self.genome)?.borrow(),
            self.body.growth_ratio,
            eyes,
            self.body.update_effects,
        )
    }

    pub fn placed(
        id: i64,
        genome: GenomeRef,
        mut body: Body,
        initial: OrganismState,
        origin: [i32; 2],
        parameters: &EffectsParams,
    ) -> Result<Self, GeometryError> {
        let segments = body.segment_count;
        let mut state = initial;
        state.parent_id = -1;
        state.bounds.set(IntRect {
            x: origin[0],
            y: origin[1],
            width: body.size()?.width,
            height: body.size()?.height,
        });
        let center = [
            origin[0].wrapping_add(body.size()?.width >> 1),
            origin[1].wrapping_add(body.size()?.height >> 1),
        ];
        state.center = center.map(f64::from);
        state.integer_center = center;
        initialize_life(
            &genome.borrow(),
            segments,
            &mut state.reproduction,
            parameters,
        )?;
        let code = genome.borrow();
        let mut effects = EffectsState::default();
        effects.apply(
            Some(&code),
            &mut body,
            state.age,
            &mut state.reproduction,
            parameters,
        )?;
        drop(code);
        Ok(Self {
            id,
            genome: Some(genome),
            infected_genome: None,
            saved_genome: None,
            body,
            state,
            effects,
            creation_complete: true,
        })
    }

    pub fn snapshot(&self) -> OrganismSnapshot {
        OrganismSnapshot {
            id: self.id,
            genotype: self.genome.as_ref().map(|code| code.borrow().clone()),
            infected_genotype: self
                .infected_genome
                .as_ref()
                .map(|code| code.borrow().clone()),
            saved_genotype: self.saved_genome.as_ref().map(|code| code.borrow().clone()),
            state: self.state.clone(),
            effects: self.effects.clone(),
            body: self.body.clone(),
        }
    }

    pub fn copy_for_world(&self, genomes: &mut GenomeCloner) -> Self {
        self.copy_for_world_with_arrays(
            genomes,
            &mut HashMap::new(),
            &mut HashMap::new(),
            &mut HashMap::new(),
            &mut HashMap::new(),
        )
    }

    fn copy_for_world_with_arrays(
        &self,
        genomes: &mut GenomeCloner,
        arrays: &mut HashMap<usize, ColorArrayRef>,
        int_arrays: &mut HashMap<usize, crate::IntArrayRef>,
        double_arrays: &mut HashMap<usize, crate::DoubleArrayRef>,
        rectangles: &mut HashMap<usize, crate::RectangleRef>,
    ) -> Self {
        let colors = self.body.phenotype_colors.as_ref().map(|original| {
            arrays
                .entry(original.identity())
                .or_insert_with(|| original.detached_copy())
                .clone()
        });
        let mut body = self.body.copy_with_colors(colors);
        body.arrays = self.body.arrays.copy_with_owners(int_arrays, double_arrays);
        body.size_rect = self
            .body
            .size_rect
            .as_ref()
            .map(|value| value.copied(rectangles));
        Self {
            id: self.id,
            genome: self.genome.as_ref().map(|code| genomes.copy(code)),
            infected_genome: self.infected_genome.as_ref().map(|code| genomes.copy(code)),
            saved_genome: self.saved_genome.as_ref().map(|code| genomes.copy(code)),
            state: self.state.copy_with_rectangles(rectangles),
            effects: self.effects.clone(),
            body,
            creation_complete: self.creation_complete,
        }
    }
}

/// Initialize base life values before color-derived effects or stillbirth handling
pub(crate) fn initialize_life(
    code: &Genome,
    segments: i32,
    reproduction: &mut ReproductionState,
    parameters: &EffectsParams,
) -> Result<(), GeometryError> {
    reproduction.maximum_age = parameters
        .max_age
        .wrapping_add(segments / parameters.age_divisor);
    reproduction.energy = 40_i32.wrapping_add(3_i32.wrapping_mul(segments));
    reproduction.maximum_countdown = 8;
    let mut additional_center_segments: i32 = 0;
    for index in 0..code.gene_count()? {
        let branch = code.gene_ref(index)?.borrow().branch();
        let additional = if branch != 0 || index == 0 {
            6
        } else {
            additional_center_segments = additional_center_segments.wrapping_add(code.symmetry());
            if code.symmetry().wrapping_add(additional_center_segments) > 8 {
                9
            } else {
                6
            }
        };
        reproduction.maximum_countdown = reproduction.maximum_countdown.wrapping_add(additional);
    }
    Ok(())
}
