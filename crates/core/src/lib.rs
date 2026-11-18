// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Engine-independent Biogenesis model
//!
//! Genetics, organism mechanics, ordinary local frames, and drawing data follow the pinned Java reference
//! Corridors, complete persistence, settings compatibility, and final product acceptance remain unfinished

mod atmosphere;
mod atomic_file;
mod birth;
mod birth_spore;
mod color;
mod effects;
mod gene_array;
pub use gene_array::GeneArrayRef;
mod genetics;
mod genotype_file;
mod geometry;
mod history;
mod shared_world_owners;
mod membership;
pub use shared_world_owners::ListAccessError;
mod legacy_shared_world_owners;
mod legacy_spatial_owners;
mod legacy_history;
mod java_math;
mod java_text;
mod java_color;
mod java_numbers;
mod legacy_graph;
mod legacy_color;
mod legacy_genetics;
mod legacy_organism_arrays;
mod legacy_organism;
#[cfg(test)]
mod legacy_spatial;
mod lifecycle;
mod movement;
pub mod object_stream;
pub mod network_wire;
pub mod network_session;
mod organism;
mod organism_arrays;
mod physics;
mod random;
mod settings;
mod spatial;
pub use spatial::SpatialAccessError;
mod statistics;
mod world;
mod xml;
mod xml_charset;

pub(crate) const TOLERANCE: f64 = 1e-7;

pub use atmosphere::AtmosphereParams;
pub use atomic_file::{FileReplaceError, replace_file_bytes};
pub use color::{ColorKind, source_palette_color};
pub use effects::{EffectsParams, EffectsState, ReproductionState};
pub use genetics::{
    DEFAULT_COLORS, Gene, GeneRef, GeneticsError, GeneticsParams, Genome, GenomeModifiers,
    GenomeProperties, REACTION_NAMES,
};
pub use genotype_file::{GenomeFileError, load_genome_xml, save_genome_xml};
pub use geometry::{
    Body, EyeState, GROWTH_SCALES, GenomePreview, GeometryError, IntRect, PolarVector, RotatedBody,
    RotatedSegment, Segment,
};
pub use lifecycle::LifecycleParams;
pub use history::HistoryError;
pub use organism_arrays::{IntArrayRef, DoubleArrayRef, OrganismArrays, IntField, DoubleField};
pub use organism::{OrganismSnapshot, OrganismState};
pub use physics::PhysicsParams;
pub use random::{JavaRandom, RandomError};
pub use java_text::JavaText;
pub use java_color::{ColorArrayRef, JavaColor, JavaFloatArray, JavaIccColorSpace, JavaIccProfile, NamedIccProfile};
pub use world::{
    CounterReadout, OrganismCounters, FeedingReadout, FeedingStamp, FeedingSubject,
    ColorCount, ColorRectangle, DraftError, DrawCommand, GenomeDraft, GenomeEstimates,
    GenomeHandle, GraphSeries, GraphSeriesKind, NewGameError, NetworkPeer, NetworkTicket,
    NetworkPeerError, NetworkTransferOutcome, LivingOrganismSummary, OrganismAction, OrganismHandle,
    OrganismImage, OrganismImageError, OutgoingConfig, OutgoingError, OutgoingFailure, PreviewImageError, RecordKind,
    RecordView, RenderSegment, StatisticsError, StatisticsGenomes, StatisticsHandle,
    StatisticsHistories, HistorySlotSnapshot, StatisticsNumbers, StatisticsView, World, WorldConfig, WorldError,
    WorldParameters, WorldStartError, WorldStatisticsValues, SessionRef, WorldHandle, WorldAssociation,
};
pub use xml::{XmlError, encode_genome_xml, parse_genome_xml};

#[cfg(test)]
mod oracle_tests;

pub use settings::{
    ALL_SETTING_KEYS, EditorOutcome, GenesisSettings, HardwareSelection, LANGUAGE_CODES,
    NUMERIC_EDITOR_KEYS, SettingKey, SettingValue, SettingsApply, SettingsDraft, SettingsError,
    SettingsState, AutomaticBackupMode, NetworkSettings, NetworkSettingsApply, parse_network_remote_port,
};

pub use settings::{
    HardwareStartup, MAX_PREFERENCE_BYTES, PreferenceDisposition, PreferenceError, PreferenceLoad,
    PreferenceMap, encode_preferences_xml, parse_preferences_xml, read_preferences_file,
};

mod rectangles;
pub use rectangles::RectangleRef;

mod legacy_rectangles;

#[cfg(test)]
mod gene_array_tests;

pub use world::WorldFileError;
