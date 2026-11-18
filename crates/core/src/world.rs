// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::organism::primary_genome;

use crate::geometry::Body;
use crate::organism::{GenomeRef, Organism, OrganismCloner, OrganismRef};
use crate::physics::rectangles_intersect as bounds_overlap;
use crate::spatial::SpatialTree;
use crate::statistics::WorldStatistics;
use crate::{
    EffectsParams, GeneticsParams, Genome, GeometryError, IntRect, JavaRandom, OrganismSnapshot,
    OrganismState,
};
use std::{
    cell::RefCell,
    f64::consts::PI,
    fmt,
    rc::Rc,
};

#[path = "incoming.rs"]
mod incoming;
use incoming::IncomingCorridorRef;
#[path = "outgoing.rs"]
mod outgoing;
use outgoing::OutgoingCorridorRef;
pub use outgoing::{DrawCommand, OutgoingConfig, OutgoingError, OutgoingFailure};
#[path = "network_peer.rs"]
mod network_peer;
pub use network_peer::{NetworkPeer, NetworkPeerError, NetworkTicket, NetworkTransferOutcome};

#[path = "session_rng.rs"]
mod session_rng;
#[path = "settings_runtime.rs"]
mod settings_runtime;
pub use session_rng::{NewGameError, WorldStartError};

#[path = "active_organism_frame.rs"]
mod active_organism_frame;
#[path = "contact_engine.rs"]
mod contact_engine;
#[path = "contact_state.rs"]
mod contact_state;
#[path = "contact_support.rs"]
mod contact_support;
#[path = "contacts.rs"]
mod contacts;
#[path = "cooperative_contacts.rs"]
mod cooperative_contacts;
#[path = "coral_contact.rs"]
mod coral_contact;
#[path = "cream_lilac_contacts.rs"]
mod cream_lilac_contacts;
#[path = "death.rs"]
mod death;
#[path = "disabling_contacts.rs"]
mod disabling_contacts;
#[path = "draining_contacts.rs"]
mod draining_contacts;
#[path = "frame_effects.rs"]
mod frame_effects;
#[path = "hatching_contact.rs"]
mod hatching_contact;
#[path = "infection.rs"]
mod infection;
#[path = "metabolism.rs"]
mod metabolism;
#[path = "organism_frame.rs"]
mod organism_frame;
#[path = "organism_image.rs"]
mod organism_image;
#[path = "preview_image.rs"]
mod preview_image;
#[path = "plague_fallow_contacts.rs"]
mod plague_fallow_contacts;
pub use organism_image::{OrganismImage, OrganismImageError};
pub use preview_image::PreviewImageError;

#[path = "statistics_view.rs"]
mod statistics_view;
pub use statistics_view::{
    ColorCount, ColorRectangle, GraphSeries, GraphSeriesKind, RecordKind, RecordView,
    StatisticsError, StatisticsGenomes, StatisticsHandle, StatisticsHistories, HistorySlotSnapshot, StatisticsNumbers,
    StatisticsView, WorldStatisticsValues,
};

#[path = "laboratory.rs"]
mod laboratory;
#[path = "player_actions.rs"]
mod player_actions;
#[path = "selection_counters.rs"]
mod selection_counters;
pub use selection_counters::{CounterReadout, OrganismCounters};
#[path = "feeding_observer.rs"]
mod feeding_observer;
pub use feeding_observer::{FeedingReadout, FeedingStamp, FeedingSubject};
pub use laboratory::{DraftError, GenomeDraft, GenomeEstimates, GenomeHandle};
#[path = "reactions.rs"]
mod reactions;
#[path = "reproduction.rs"]
mod reproduction;
#[path = "simulation.rs"]
mod simulation;
#[path = "special_contacts.rs"]
mod special_contacts;
#[path = "transformation.rs"]
mod transformation;
#[path = "white_fruit_contacts.rs"]
mod white_fruit_contacts;
#[path = "world_actions.rs"]
mod world_actions;
pub use player_actions::{LivingOrganismSummary, OrganismAction, OrganismHandle};

const SIDES: [[i32; 2]; 8] = [
    [1, 0],
    [1, 1],
    [0, 1],
    [-1, 1],
    [-1, 0],
    [-1, -1],
    [0, -1],
    [1, -1],
];

/// Maximum attempt count accepted by the Source nonnegative Integer setting
pub const MAX_INITIAL_ATTEMPTS: u32 = i32::MAX as u32;

/// Stored Java dimensions are signed; startup validation remains separate
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldConfig {
    pub seed: i64,
    pub width: i32,
    pub height: i32,
    pub initial_attempts: u32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            width: 7_000,
            height: 7_000,
            initial_attempts: 12_500,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldParameters {
    pub atmosphere: crate::AtmosphereParams,
    pub physics: crate::PhysicsParams,
    pub lifecycle: crate::LifecycleParams,
    pub genetics: GeneticsParams,
    pub effects: EffectsParams,
    pub initial_energy: f64,
    pub spore_energy_consumption: f64,
    pub outgoing: Option<OutgoingConfig>,
}

impl Default for WorldParameters {
    fn default() -> Self {
        Self {
            atmosphere: crate::AtmosphereParams::default(),
            physics: crate::PhysicsParams::default(),
            lifecycle: crate::LifecycleParams::default(),
            genetics: GeneticsParams::default(),
            effects: EffectsParams::default(),
            initial_energy: 40.0,
            spore_energy_consumption: 0.0,
            outgoing: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorldError {
    MissingWorld,
    ImportRetentionAllocation,
    ExpiredWorld,
    MissingNetworkPosition,
    InvalidDimensions,
    InitialAttemptsExceeded { maximum: u32 },
    BodyDoesNotFit { width: u32, height: u32 },
    InvalidParameters(String),
    Arithmetic(&'static str),
    Geometry(GeometryError),
    History(crate::HistoryError),
    List(crate::ListAccessError),
    Spatial(crate::spatial::SpatialAccessError),
    Drawing { error: GeometryError, commands: Vec<crate::DrawCommand>, paint: u32 },
    DrawingList { error: crate::ListAccessError, commands: Vec<crate::DrawCommand>, paint: u32 },
    OrganismNotFound { id: i64 },
    InvalidOrganismHandle,
    Outgoing(OutgoingError),
    InvalidCorridorDimension { axis: &'static str, value: i32 },
}

impl From<crate::ListAccessError> for WorldError { fn from(error: crate::ListAccessError) -> Self { Self::List(error) } }
impl From<crate::spatial::SpatialAccessError> for WorldError { fn from(error: crate::spatial::SpatialAccessError) -> Self { Self::Spatial(error) } }

impl fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImportRetentionAllocation => formatter.write_str("Imported Rectangle retention allocation failed"),
            Self::MissingWorld => formatter.write_str("Missing organism World"),
            Self::ExpiredWorld => formatter.write_str("Expired retained organism World"),
            Self::MissingNetworkPosition => formatter.write_str("Missing reproduction placement positions"),

            Self::InvalidDimensions => {
                formatter.write_str("World dimensions must be positive Java integers")
            }
            Self::InitialAttemptsExceeded { maximum } => write!(
                formatter,
                "This checkpoint supports at most {maximum} initial attempts"
            ),
            Self::BodyDoesNotFit { width, height } => write!(
                formatter,
                "World must exceed the generated body's {width} by {height} placement rectangle"
            ),
            Self::InvalidParameters(message) => formatter.write_str(message),
            Self::Arithmetic(message) => write!(formatter, "Arithmetic error: {message}"),
            Self::Geometry(error) | Self::Drawing { error, .. } => error.fmt(formatter),
            Self::History(error) => error.fmt(formatter),
            Self::Spatial(error) => error.fmt(formatter),
            Self::List(error) | Self::DrawingList { error, .. } => error.fmt(formatter),
            Self::OrganismNotFound { id } => write!(formatter, "Organism {id} is unavailable"),
            Self::InvalidOrganismHandle => {
                formatter.write_str("Organism handle is not a current member of this world")
            }
            Self::Outgoing(error) => {
                write!(formatter, "Outgoing transfer failed: {:?}", error.kind)
            }
            Self::InvalidCorridorDimension { axis, value } => write!(
                formatter,
                "Incoming corridor {axis} must be positive, received {value}"
            ),
        }
    }
}

impl std::error::Error for WorldError {}

impl From<crate::GeneticsError> for WorldError {
    fn from(error: crate::GeneticsError) -> Self {
        match error {
            crate::GeneticsError::NullGeneArray | crate::GeneticsError::ZeroGeneDivisor | crate::GeneticsError::NullGene { .. }
            | crate::GeneticsError::GeneIndex { .. } | crate::GeneticsError::GeneIndexDomain { .. } => Self::Geometry(error.into()),
            _ => Self::InvalidParameters(error.to_string()),
        }
    }
}

impl From<GeometryError> for WorldError {
    fn from(error: GeometryError) -> Self {
        Self::Geometry(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderSegment {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub rgb: u32,
    pub alpha: u8,
    pub organism_id: i64,
}

#[path = "world_root.rs"]
mod root;
pub use root::{SessionRef, World, WorldHandle, WorldState, WorldAssociation};

#[path = "legacy_world.rs"]
pub(crate) mod legacy;

impl World {
    /// Build an isolated caller for validating a saved World against a captured live session
    pub fn detached_validation_context(config: WorldConfig, parameters: WorldParameters, random: JavaRandom) -> Self {
        Self::empty_in_session(config, SessionRef::new(parameters, random))
    }

    pub fn new(config: WorldConfig) -> Result<Self, WorldError> {
        Self::with_parameters(config, WorldParameters::default())
    }

    pub fn with_parameters(
        config: WorldConfig,
        parameters: WorldParameters,
    ) -> Result<Self, WorldError> {
        let world = Self::empty_with_parameters(config, parameters)?;
        world.genesis()?;
        Ok(world)
    }

    fn empty_with_parameters(
        config: WorldConfig,
        parameters: WorldParameters,
    ) -> Result<Self, WorldError> {
        Self::validate_initialization(config, &parameters)?;
        Ok(Self::empty_validated(
            config,
            parameters,
            JavaRandom::new(config.seed),
        ))
    }

    fn validate_initialization(
        config: WorldConfig,
        parameters: &WorldParameters,
    ) -> Result<(), WorldError> {
        if config.width <= 0 || config.height <= 0 {
            return Err(WorldError::InvalidDimensions);
        }
        if config.initial_attempts > MAX_INITIAL_ATTEMPTS {
            return Err(WorldError::InitialAttemptsExceeded {
                maximum: MAX_INITIAL_ATTEMPTS,
            });
        }
        parameters
            .lifecycle
            .validate()
            .map_err(|error| WorldError::InvalidParameters(error.to_owned()))?;
        parameters
            .physics
            .validate()
            .map_err(|error| WorldError::InvalidParameters(error.to_owned()))?;
        parameters
            .atmosphere
            .validate()
            .map_err(|error| WorldError::InvalidParameters(error.to_owned()))?;
        parameters
            .genetics
            .validate()
            .map_err(|error| WorldError::InvalidParameters(error.to_string()))?;
        parameters
            .effects
            .validate()
            .map_err(|error| WorldError::InvalidParameters(error.to_owned()))?;
        if parameters.spore_energy_consumption.is_nan()
            || parameters.spore_energy_consumption == f64::NEG_INFINITY
        {
            return Err(WorldError::InvalidParameters(
                "Spore energy consumption must not be NaN or negative infinity".to_owned(),
            ));
        }
        if !parameters.initial_energy.is_finite() || parameters.initial_energy < 0.0 {
            return Err(WorldError::InvalidParameters(
                "Initial energy must be finite and nonnegative".to_owned(),
            ));
        }
        Ok(())
    }

    fn empty_validated(
        config: WorldConfig,
        parameters: WorldParameters,
        random: JavaRandom,
    ) -> Self {
        Self::empty_in_session(config, SessionRef::new(parameters, random))
    }

    /// Repopulate using the current RNG while retaining the source frozen collision tree
    pub fn genesis(&self) -> Result<(), WorldError> {
        self.invalidate_feeding();
        self.frame.set(0);
        self.tick.set(0);
        self.oxygen.set(self.session.0.parameters.borrow().atmosphere.initial_oxygen);
        self.carbon_dioxide.set(self.session.0.parameters.borrow().atmosphere.initial_carbon_dioxide);
        self.methane.set(self.session.0.parameters.borrow().atmosphere.initial_methane);
        self.next_id.set(0);
        self.population.set(0);
        self.organisms.clear()?;
        self.statistics.replace(Some(WorldStatistics::new(
            self.config.get().initial_attempts as i32,
            self.oxygen.get(),
            self.carbon_dioxide.get(),
            self.methane.get(),
        )
        .into_shared()));
        for _ in 0..self.config.get().initial_attempts {
            if let Some(organism) = self.construct_random()? {
                self.add_organism(Rc::new(RefCell::new(organism)), None)?;
            }
        }
        Ok(())
    }

    fn initial_organism_state(&self) -> OrganismState {
        OrganismState {
            world: self.association(),
            angle: self.session.0.random.borrow_mut().next_f64() * PI * 2.0,
            ..OrganismState::default()
        }
    }

    // The source constructor reads its World before drawing the orientation
    fn initial_child_state(&self, parent: &Organism) -> Result<OrganismState, WorldError> {
        let owner = parent.state.world.resolve()?;
        Ok(OrganismState {
            world: owner.association(),
            angle: self.session.0.random.borrow_mut().next_f64() * PI * 2.0,
            ..OrganismState::default()
        })
    }

    fn allocate_id(&self) -> i64 {
        let id = self.next_id.get();
        self.next_id.set(self.next_id.get().wrapping_add(1));
        i64::from(id)
    }

    fn construct_random(&self) -> Result<Option<Organism>, WorldError> {
        let mut initial = self.initial_organism_state();
        let mut genome = Genome::random_with(&mut self.session.0.random.borrow_mut(), &self.session.0.parameters.borrow().genetics)
            .map_err(|error| WorldError::InvalidParameters(error.to_string()))?;
        genome.set_generation(1);
        initial.energy = crate::java_math::min(self.session.0.parameters.borrow().initial_energy, self.carbon_dioxide.get());
        // Failed placement also consumes atmospheric carbon in the reference
        self.carbon_dioxide.set(self.carbon_dioxide.get() - (initial.energy));
        self.oxygen.set(self.oxygen.get() + (initial.energy));
        let body = Body::newborn(&genome);
        for _ in 0..12 {
            let x_bound = (self.config.get().width as i32).wrapping_sub(body.size().map_err(WorldError::Geometry)?.width);
            if x_bound <= 0 {
                return Err(WorldError::BodyDoesNotFit { width: body.size().map_err(WorldError::Geometry)?.width as u32,
                    height: body.size().map_err(WorldError::Geometry)?.height as u32 });
            }
            let x = self.session.0.random.borrow_mut().bounded(x_bound);
            let y_bound = (self.config.get().height as i32).wrapping_sub(body.size().map_err(WorldError::Geometry)?.height);
            if y_bound <= 0 {
                return Err(WorldError::BodyDoesNotFit { width: body.size().map_err(WorldError::Geometry)?.width as u32,
                    height: body.size().map_err(WorldError::Geometry)?.height as u32 });
            }
            let origin = [x, self.session.0.random.borrow_mut().bounded(y_bound)];
            let candidate = IntRect {
                x: origin[0], y: origin[1],
                width: body.size().map_err(WorldError::Geometry)?.width,
                height: body.size().map_err(WorldError::Geometry)?.height,
            };
            let mut overlaps = false;
            for index in 0..self.organisms.len()? {
                if bounds_overlap(candidate, self.organisms.actor(index)?.borrow().state.bounds.get()) { overlaps = true; break; }
            }
            if overlaps { continue; }
            let id = self.allocate_id();
            genome.set_clade_id(Some(id.to_string()));
            return Ok(Some(Organism::placed(
                id,
                Rc::new(RefCell::new(genome)),
                body,
                initial,
                origin,
                &self.session.0.parameters.borrow().effects,
            )?));
        }
        Ok(None)
    }

    pub fn pause(&self) {
        self.paused.set(true);
    }
    pub fn resume(&self) {
        self.paused.set(false);
    }

    /// Advance one frame only when running; retain partial state and pause on failure
    pub fn step(&self) -> Result<bool, WorldError> {
        if self.paused.get() {
            return Ok(false);
        }
        if let Err(error) = self.advance() {
            self.pause();
            return Err(error);
        }
        Ok(true)
    }

    /// Advance one frame while retaining the paused state and any partial failure state
    pub fn single_step(&self) -> Result<bool, WorldError> {
        if !self.paused.get() {
            return Ok(false);
        }
        self.advance()?;
        Ok(true)
    }

    fn advance(&self) -> Result<(), WorldError> {
        self.spatial.replace(SpatialTree::empty());
        self.frame_objects.borrow_mut().clear();
        self.retire_imported_actors()?;
        if self.corridors_exist.get() {
            let incoming = self.incoming_corridors.borrow().clone();
            for corridor in incoming {
                self.frame_incoming_corridor(&corridor)?;
            }
        }
        self.rebuild_spatial()?;
        let mut remaining = self.organisms.len()?;
        let mut index = 0;
        while index < remaining {
            let actor_ref = self.organisms.actor(index)?;
            let survives = self.move_actor_frame(&actor_ref, &mut actor_ref.borrow_mut())?;
            if survives {
                index += 1;
            } else {
                self.organisms.remove(index)?;
                remaining -= 1;
            }
        }
        // World.time applies the second reaction to the updated methane amount
        let carbon_to_methane =
            crate::java_math::div(self.carbon_dioxide.get(), f64::from(self.session.0.parameters.borrow().atmosphere.carbon_to_methane_divisor));
        crate::atmosphere::transfer_cells(
            &self.carbon_dioxide,
            &self.methane,
            carbon_to_methane,
        );
        let methane_to_carbon =
            crate::java_math::div(self.methane.get(), f64::from(self.session.0.parameters.borrow().atmosphere.methane_to_carbon_divisor));
        crate::atmosphere::transfer_cells(
            &self.methane,
            &self.carbon_dioxide,
            methane_to_carbon,
        );
        self.tick.set(self.tick.get()
            .checked_add(1)
            .ok_or(WorldError::Arithmetic("world tick overflow"))?);
        self.frame.set(self.frame.get().wrapping_add(1));
        if self.frame.get() % 256 == 0 {
            self.frame.set(0);
            self.statistics_ref().borrow_mut().event_time(
                self.population.get(),
                self.oxygen.get(),
                self.carbon_dioxide.get(),
                self.methane.get(),
            ).map_err(WorldError::History)?;
        }
        Ok(())
    }

    fn rebuild_spatial(&self) -> Result<(), WorldError> {
        self.frame_objects.borrow_mut().clear();
        self.spatial.replace(SpatialTree::empty());
        for index in 0..self.organisms.len()? {
            let actor = self.organisms.actor(index)?;
            let bounds = actor.borrow().state.bounds.get();
            self.frame_objects.borrow_mut().push(Rc::clone(&actor));
            self.spatial.borrow().insert_actor(index, &actor, bounds)?;
        }
        Ok(())
    }

    /// Restore the configured seed and initial paused state atomically
    pub fn reset(&mut self) -> Result<(), WorldError> {
        *self = Self::with_parameters(self.config.get(), self.session.parameters())?;
        Ok(())
    }

    pub fn seed(&self) -> i64 {
        self.config.get().seed
    }
    pub fn tick(&self) -> u64 {
        self.tick.get()
    }
    pub fn paused(&self) -> bool {
        self.paused.get()
    }
    /// Read all current settings without changing their World ownership
    pub fn parameters(&self) -> WorldParameters {
        self.session.parameters()
    }

    /// Read the actual configured physics values without mutable access
    pub fn physics_parameters(&self) -> crate::PhysicsParams {
        self.session.0.parameters.borrow().physics
    }
    pub fn initial_attempts(&self) -> u32 {
        self.config.get().initial_attempts
    }
    pub fn dimensions(&self) -> (i32, i32) {
        (self.config.get().width, self.config.get().height)
    }
    pub fn population(&self) -> i32 {
        self.population.get()
    }
    pub fn corpse_count(&self) -> Result<i32, WorldError> {
        Ok((self.organisms.len()? as i32).wrapping_sub(self.population.get()))
    }
    pub fn frame(&self) -> i32 {
        self.frame.get()
    }
    pub fn time(&self) -> i64 {
        self.statistics_ref().borrow().get_time()
    }
    pub fn oxygen(&self) -> f64 {
        self.oxygen.get()
    }
    pub fn carbon_dioxide(&self) -> f64 {
        self.carbon_dioxide.get()
    }
    pub fn methane(&self) -> f64 {
        self.methane.get()
    }

    pub fn add_oxygen(&self, amount: f64) {
        self.oxygen.set(crate::java_math::add(self.oxygen.get(), amount));
    }
    pub fn add_carbon_dioxide(&self, amount: f64) {
        self.carbon_dioxide.set(crate::java_math::add(self.carbon_dioxide.get(), amount));
    }
    pub fn add_methane(&self, amount: f64) {
        self.methane.set(crate::java_math::add(self.methane.get(), amount));
    }
    pub fn decrease_oxygen(&self, amount: f64) {
        self.oxygen.set(crate::java_math::sub(self.oxygen.get(), crate::java_math::min(amount, self.oxygen.get())));
    }
    pub fn decrease_carbon_dioxide(&self, amount: f64) {
        self.carbon_dioxide.set(crate::java_math::sub(self.carbon_dioxide.get(), crate::java_math::min(amount, self.carbon_dioxide.get())));
    }
    pub fn decrease_methane(&self, amount: f64) {
        self.methane.set(crate::java_math::sub(self.methane.get(), crate::java_math::min(amount, self.methane.get())));
    }
    pub fn respiration(&self, amount: f64) -> f64 {
        self.transfer_gas(amount, false)
    }
    pub fn decomposition(&self, amount: f64) -> f64 {
        self.transfer_gas(amount, true)
    }
    pub fn photosynthesis(&self, amount: f64) -> f64 {
        let available = self.carbon_dioxide.get();
        let divisor = self.session.0.parameters.borrow().atmosphere.drain_divisor;
        let obtained = crate::java_math::min(
            crate::java_math::min(amount, crate::java_math::div(crate::java_math::mul(amount, available), f64::from(divisor))), available,
        );
        self.carbon_dioxide.set(crate::java_math::sub(available, obtained));
        self.oxygen.set(crate::java_math::add(self.oxygen.get(), obtained));
        obtained
    }
    pub fn methanotrophy(&self, amount: f64) -> f64 {
        let available = self.methane.get();
        let divisor = self.session.0.parameters.borrow().atmosphere.drain_divisor;
        let obtained = crate::java_math::min(
            crate::java_math::min(amount, crate::java_math::div(crate::java_math::mul(amount, available), f64::from(divisor))), available,
        );
        self.methane.set(crate::java_math::sub(available, obtained));
        self.oxygen.set(crate::java_math::add(self.oxygen.get(), obtained));
        obtained
    }

    pub fn inspect(&self, id: i64) -> Result<Option<OrganismSnapshot>, WorldError> {
        for index in 0..self.organisms.len()? {
            let actor = self.organisms.actor(index)?;
            if actor.borrow().id == id { return Ok(Some(actor.borrow().snapshot())); }
        }
        Ok(None)
    }

    /// Attempt one child construction; automatic reproduction scheduling remains a separate frame rule
    /// Publish through the parent association without incrementing its total-child counter
    pub fn spawn_child(&self, parent_id: i64, first: bool) -> Result<Option<i64>, WorldError> {
        let mut parent = None;
        for index in 0..self.organisms.len()? {
            let actor = self.organisms.actor(index)?;
            if actor.borrow().id == parent_id { parent = Some(actor); break; }
        }
        let parent = parent.ok_or(WorldError::OrganismNotFound { id: parent_id })?;
        let mut parent_state = parent.borrow_mut();
        let result = self.construct_child_from(&parent, &mut parent_state, first, None)?;
        if let Some(child) = result {
            let id = child.borrow().id;
            parent_state.state.world.resolve()?.add_organism(child, Some(&parent_state))?;
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn construct_child_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        first: bool,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<Option<OrganismRef>, WorldError> {
        let initial = self.initial_child_state(parent)?;
        self.construct_child_with_initial_from(parent_ref, parent, initial, first, peer)
    }

    fn construct_child_with_initial_from(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        initial: OrganismState,
        first: bool,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<Option<OrganismRef>, WorldError> {
        let Some(mut birth) = crate::birth::prepare(
            parent,
            initial,
            first,
            &mut self.session.0.random.borrow_mut(),
            &self.session.0.parameters.borrow().genetics,
        )?
        else {
            return Ok(None);
        };
        if !self.place_child(parent_ref, parent, peer, &mut birth.child, first)? {
            birth.failed(parent);
            return Ok(None);
        }
        if birth.initialize(parent, &self.session.0.parameters.borrow().effects)? {
            crate::birth_spore::initialize_birth_spore(
                &mut birth.child,
                parent,
                &mut self.session.0.random.borrow_mut(),
                &self.session.0.parameters.borrow().effects,
                self.session.0.parameters.borrow().spore_energy_consumption,
            )?;
        }
        birth.update_lineage(self.session.0.parameters.borrow().genetics.clade_complexity)?;
        Ok(Some(Rc::new(RefCell::new(birth.child))))
    }

    fn add_organism(&self, child: OrganismRef, parent: Option<&Organism>) -> Result<(), WorldError> {
        child.borrow().state.bounds.bind_actor(&child).expect("Each organism owns its inherited Rectangle");
        self.organisms.push(Rc::clone(&child))?;
        if let Some(parent) = parent {
            self.notify_children(parent);
            self.statistics_ref()
                .borrow_mut()
                .event_organism_born(&child.borrow(), parent);
        }
        self.statistics_ref().borrow_mut().event_organism_created();
        self.increase_population();
        Ok(())
    }

    fn increase_population(&self) {
        self.population.set(self.population.get().wrapping_add(1));
        self.statistics_ref()
            .borrow_mut()
            .event_population_increase(self.population.get());
    }

    fn decrease_population(&self) {
        self.population.set(self.population.get().wrapping_sub(1));
        self.statistics_ref()
            .borrow_mut()
            .event_population_decrease(self.population.get());
    }

    fn is_inside_world(&self, organism: &mut Organism) -> Result<bool, crate::WorldError> {
        let bounds = organism.state.bounds.get();
        if bounds.x <= 0 || bounds.y <= 0
            || bounds.x.wrapping_add(bounds.width) >= organism.state.world.resolve()?.config.get().width
            || bounds.y.wrapping_add(bounds.height) >= organism.state.world.resolve()?.config.get().height
        {
            if bounds.height <= 1 && bounds.width <= 1 && organism.state.alive
                && bounds.x == 0 && bounds.y == 0
            {
                self.die(organism, None)?;
            }
            if bounds.x <= 0
                || bounds.x.wrapping_add(bounds.width) >= organism.state.world.resolve()?.config.get().width
            {
                organism.state.velocity[0] = -organism.state.velocity[0];
            }
            if bounds.y <= 0
                || bounds.y.wrapping_add(bounds.height) >= organism.state.world.resolve()?.config.get().height
            {
                organism.state.velocity[1] = -organism.state.velocity[1];
            }
            organism.state.angular_velocity = 0.0;
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn with_frame_candidate<R>(
        &self,
        reference: &OrganismRef,
        parent_ref: &OrganismRef,
        parent: &Organism,
        peer: Option<(&OrganismRef, &Organism)>,
        inspect: impl FnOnce(&Organism) -> R,
    ) -> R {
        if Rc::ptr_eq(reference, parent_ref) { return inspect(parent); }
        if let Some((peer_ref, peer)) = peer {
            if Rc::ptr_eq(reference, peer_ref) { return inspect(peer); }
        }
        inspect(&reference.borrow())
    }

    fn query_actor(item: crate::shared_world_owners::SharedSlot, index: usize) -> Result<OrganismRef, WorldError> {
        use crate::shared_world_owners::SharedSlot;
        match item {
            SharedSlot::Actor(actor) => Ok(actor),
            SharedSlot::Null => Err(crate::ListAccessError::MissingElement { field: "query", index }.into()),
            value => Err(crate::ListAccessError::WrongClass { field: "query", index, expected: "biogenesis.Organism", actual: value.class_name() }.into()),
        }
    }
    fn fast_check_hit(
        &self,
        candidate: &Organism,
        parent_ref: &OrganismRef,
        parent: &Organism,
        peer: Option<(&OrganismRef, &Organism)>,
    ) -> Result<bool, WorldError> {
        let candidates = self.spatial.borrow().query_slots(candidate.state.bounds.get())?;
        for (index, item) in candidates.into_iter().enumerate() {
            let reference = Self::query_actor(item,index)?;
            if self.with_frame_candidate(&reference,parent_ref,parent,peer,|other| {
                !std::ptr::eq(candidate,other) && bounds_overlap(candidate.state.bounds.get(),other.state.bounds.get())
            }) { return Ok(true); }
        }
        Ok(false)
    }

    fn place_child(
        &self,
        parent_ref: &OrganismRef,
        parent: &mut Organism,
        peer: Option<(&OrganismRef, &Organism)>,
        child: &mut Organism,
        first: bool,
    ) -> Result<bool, WorldError> {
        let multiple = parent.state.children > 1 || parent.state.virus_children > 1;
        if multiple && first {
            parent.state.network_position = Some(crate::java_text::initial_positions());
        }
        let mut direction = self.session.0.random.borrow_mut().bounded(8) as usize;
        for _ in 0..8 {
            for axis in 0..2 {
                let parent_size = if axis == 0 {
                    parent.state.bounds.get().width
                } else {
                    parent.state.bounds.get().height
                };
                let child_size = if axis == 0 {
                    child.state.bounds.get().width
                } else {
                    child.state.bounds.get().height
                };
                let offset = (parent_size / 2)
                    .wrapping_add(child_size / 2)
                    .wrapping_add(1)
                    .wrapping_mul(SIDES[direction][axis]);
                child.state.center[axis] = parent.state.center[axis] + f64::from(offset);
                child.state.integer_center[axis] = child.state.center[axis] as i32;
            }
            crate::movement::calculate_bounds(child, true).map_err(WorldError::Geometry)?;
            if self.is_inside_world(child)? && !child.state.world.resolve()?.fast_check_hit(child, parent_ref, parent, peer)?
            {
                let reserved = u16::from(b'0' + direction as u8);
                let available = if multiple {
                    parent.state.network_position.as_ref()
                        .ok_or(WorldError::MissingNetworkPosition)?
                        .units().contains(&reserved)
                } else {
                    true
                };
                if available {
                    child.state.velocity = if primary_genome(&parent.genome)?.borrow().modifiers().disperse_children {
                        [
                            f64::from(SIDES[direction][0]),
                            f64::from(SIDES[direction][1]),
                        ]
                    } else {
                        parent.state.velocity
                    };
                    child.id = child.state.world.resolve()?.allocate_id();
                    parent.state.energy -= child.state.energy;
                    if multiple {
                        let positions = parent
                            .state
                            .network_position
                            .as_mut()
                            .expect("Litter placement requires reserved directions");
                        let units = positions.units().iter().map(|&unit| {
                            if unit == reserved { u16::from(b'X') } else { unit }
                        }).collect();
                        *positions = crate::JavaText::from_units(units);
                    }
                    return Ok(true);
                }
            }
            direction = (direction + 1) % 8;
        }
        Ok(false)
    }

    /// Insert a supplied genotype through the reference paste construction path
    pub fn paste_genome(
        &self,
        genome: Genome,
        center: [i32; 2],
    ) -> Result<Option<i64>, WorldError> {
        match self.construct_pasted(genome, center)? {
            Some(organism) => {
                let id = organism.id;
                self.add_organism(Rc::new(RefCell::new(organism)), None)?;
                Ok(Some(id))
            }
            None => Ok(None),
        }
    }

    fn construct_pasted(
        &self,
        genome: Genome,
        center: [i32; 2],
    ) -> Result<Option<Organism>, WorldError> {
        self.construct_pasted_shared(Rc::new(RefCell::new(genome)), center)
    }

    fn construct_pasted_shared(
        &self,
        genome: GenomeRef,
        center: [i32; 2],
    ) -> Result<Option<Organism>, WorldError> {
        let mut organism = self.new_pasted_candidate(genome);
        if self.attempt_paste(None, &mut organism, center)? {
            Ok(Some(organism))
        } else {
            Ok(None)
        }
    }

    fn new_pasted_candidate(&self, genome: GenomeRef) -> Organism {
        Organism::pasted_candidate(Some(genome), self.initial_organism_state())
    }

    // A held candidate can already be a member after an earlier source callback failed
    fn attempt_paste(
        &self,
        actor_ref: Option<&OrganismRef>,
        organism: &mut Organism,
        center: [i32; 2],
    ) -> Result<bool, WorldError> {
        organism.state.parent_id = -1;
        {
            let mut code = primary_genome(&organism.genome)?.borrow_mut();
            if code.generation() == 0 {
                code.set_generation(1);
            }
        }
        organism.body.growth_ratio = 16;
        organism.create_for_paste().map_err(WorldError::Geometry)?;
        // Paste rejects even the first branch, which geometry intentionally ignores
        let invalid_branch = {
            let code = primary_genome(&organism.genome)?.borrow();
            { let mut invalid = false; for index in 0..code.gene_count().map_err(GeometryError::from)? { if code.gene_ref(index).map_err(GeometryError::from)?.borrow().branch() > index as i32 { invalid = true; break; } } invalid }
        };
        if invalid_branch {
            return Ok(false);
        }
        organism.state.center = center.map(f64::from);
        organism.state.integer_center = center;
        crate::movement::calculate_bounds(organism, true).map_err(WorldError::Geometry)?;
        if !self.is_inside_world(organism)? { return Ok(false); }
        for index in 0..self.organisms.len()? {
            let other = self.organisms.actor(index)?;
            if actor_ref.is_some_and(|reference| Rc::ptr_eq(reference, &other)) { continue; }
            if bounds_overlap(organism.state.bounds.get(), other.borrow().state.bounds.get()) { return Ok(false); }
        }
        organism.id = self.allocate_id();
        {
            let mut code = primary_genome(&organism.genome)?.borrow_mut();
            if code.clade_id().is_none() {
                code.set_clade_id(Some(organism.id.to_string()));
            }
        }
        organism.state.energy =
            crate::java_math::min(self.session.0.parameters.borrow().initial_energy, self.carbon_dioxide.get());
        self.carbon_dioxide.set(self.carbon_dioxide.get() - (organism.state.energy));
        self.oxygen.set(self.oxygen.get() + (organism.state.energy));
        let code = primary_genome(&organism.genome)?.borrow();
        crate::organism::initialize_life(
            &code,
            organism.body.segment_count,
            &mut organism.state.reproduction,
            &self.session.0.parameters.borrow().effects,
        )?;
        organism.effects.apply(
            Some(&code),
            &mut organism.body,
            organism.state.age,
            &mut organism.state.reproduction,
            &self.session.0.parameters.borrow().effects,
        )?;
        drop(code);
        Ok(true)
    }

    /// Return owned, ordered segment data without advancing state or consuming randomness
    /// Begin with white; null temporary colors retain the preceding draw color
    pub fn render_segments(&self) -> Result<Vec<RenderSegment>, WorldError> {
        self.render_segments_from_color(0xffff_ffff)
    }

    /// Continue ordered segment drawing with the supplied inherited paint
    pub fn render_segments_from_color(&self, mut draw_color: u32) -> Result<Vec<RenderSegment>, WorldError> {
        let mut output = Vec::new();
        let count = self.organisms.len().map_err(|error| WorldError::DrawingList { error, commands: Vec::new(), paint: draw_color })?;
        for index in 0..count {
            let organism = match self.organisms.actor(index) {
                Ok(actor) => actor,
                Err(error) => return Err(WorldError::DrawingList { error, commands: output.into_iter().map(crate::DrawCommand::Line).collect(), paint: draw_color }),
            };
            if let Err(error) = outgoing::emit_actor_segments(&organism.borrow(), &mut draw_color, |segment| {
                output.push(segment)
            }) {
                return Err(WorldError::Drawing {
                    error, commands: output.into_iter().map(crate::DrawCommand::Line).collect(),
                    paint: draw_color,
                });
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
#[path = "organism_oracle_tests.rs"]
mod organism_oracle_tests;

#[cfg(test)]
#[path = "world_identity_tests.rs"]
mod world_identity_tests;

#[cfg(test)]
#[path = "frame_storage_tests.rs"]
mod frame_storage_tests;

#[cfg(test)]
#[path = "world_event_tests.rs"]
mod world_event_tests;

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;

#[cfg(test)]
#[path = "paste_retry_tests.rs"]
mod paste_retry_tests;

#[cfg(test)]
#[path = "nullable_primary_tests.rs"]
mod nullable_primary_tests;

#[cfg(test)]
#[path = "phenotype_color_tests.rs"]
mod phenotype_color_tests;

#[cfg(test)]
#[path = "organism_array_tests.rs"]
mod organism_array_tests;

#[cfg(test)]
#[path = "history_oracle_tests.rs"]
mod history_oracle_tests;

#[cfg(test)]
#[path = "signed_growth_ratio_tests.rs"]
mod signed_growth_ratio_tests;

#[path = "legacy_world_export.rs"]
pub(crate) mod legacy_export;

#[path = "world_file.rs"]
mod world_file;
pub use world_file::WorldFileError;

#[cfg(test)]
#[path = "shared_world_runtime_review_tests.rs"]
mod shared_world_runtime_review_tests;

#[cfg(test)]
#[path = "live_spatial_review_tests.rs"]
mod live_spatial_review_tests;
