// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Retained statistics windows and source logical graph commands

use super::*;
use crate::ColorKind;

/// Retain the actual statistics object across replacement of the World's current owner
#[derive(Clone)]
pub struct StatisticsHandle(pub(super) Rc<RefCell<WorldStatistics>>, pub(super) WorldHandle);

impl fmt::Debug for StatisticsHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("StatisticsHandle(..)")
    }
}

impl PartialEq for StatisticsHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for StatisticsHandle {}

/// A source color-panel failure without silently discarding a missing genetic color
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatisticsError {
    History(crate::HistoryError),
    List(crate::ListAccessError),
    Genetics(crate::GeneticsError),
    MissingGenome { organism_id: i64 },
    MissingGeneColor { organism_id: i64, gene_index: usize },
}

impl fmt::Display for StatisticsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Genetics(error) => error.fmt(formatter),
            Self::History(error) => error.fmt(formatter),
            Self::List(error) => error.fmt(formatter),
            Self::MissingGenome { organism_id } => write!(formatter, "Organism {organism_id} has no primary genome"),
            Self::MissingGeneColor {
                organism_id,
                gene_index,
            } => write!(
                formatter,
                "Organism {organism_id} gene {gene_index} has no color"
            ),
        }
    }
}

impl std::error::Error for StatisticsError {}
impl From<crate::GeneticsError> for StatisticsError {
    fn from(error: crate::GeneticsError) -> Self { Self::Genetics(error) }
}

/// Current World values remain distinct from the window's retained statistics
#[derive(Clone, Debug, PartialEq)]
pub struct WorldStatisticsValues {
    pub time: i64,
    pub population: i32,
    pub corpses: i32,
    pub oxygen: f64,
    pub carbon_dioxide: f64,
    pub methane: f64,
}

/// Unrounded numeric getters copied at explicit refresh
#[derive(Clone, Debug, PartialEq)]
pub struct StatisticsNumbers {
    pub time: i64,
    pub max_population: i32,
    pub max_birth: i32,
    pub max_deaths: i32,
    pub max_population_time: i64,
    pub min_population: i32,
    pub min_population_time: i64,
    pub mass_extinctions: i32,
    pub created_organisms: i32,
    pub average_population: f64,
    pub average_deaths: f64,
    pub average_births: f64,
    pub average_infections: f64,
    pub max_oxygen: f64,
    pub max_oxygen_time: i64,
    pub min_oxygen: f64,
    pub min_oxygen_time: i64,
    pub max_carbon_dioxide: f64,
    pub max_carbon_dioxide_time: i64,
    pub min_carbon_dioxide: f64,
    pub min_carbon_dioxide_time: i64,
    pub max_methane: f64,
    pub max_methane_time: i64,
    pub min_methane: f64,
    pub min_methane_time: i64,
    pub alive_being_most_children_number: i32,
    pub alive_being_most_kills_number: i32,
    pub alive_being_most_infections_number: i32,
    pub being_most_children_number: i32,
    pub being_most_children_time: i64,
    pub being_most_kills_number: i32,
    pub being_most_kills_time: i64,
    pub being_most_infections_number: i32,
    pub being_most_infections_time: i64,
}

impl StatisticsNumbers {
    fn capture(statistics: &WorldStatistics) -> Self {
        Self {
            time: statistics.get_time(),
            max_population: statistics.get_max_population(),
            max_birth: statistics.get_max_birth(),
            max_deaths: statistics.get_max_deaths(),
            max_population_time: statistics.get_max_population_time(),
            min_population: statistics.get_min_population(),
            min_population_time: statistics.get_min_population_time(),
            mass_extinctions: statistics.get_mass_extinctions(),
            created_organisms: statistics.get_created_organisms(),
            average_population: statistics.get_average_population(),
            average_deaths: statistics.get_average_deaths(),
            average_births: statistics.get_average_births(),
            average_infections: statistics.get_average_infections(),
            max_oxygen: statistics.get_max_oxygen(),
            max_oxygen_time: statistics.get_max_oxygen_time(),
            min_oxygen: statistics.get_min_oxygen(),
            min_oxygen_time: statistics.get_min_oxygen_time(),
            max_carbon_dioxide: statistics.get_max_carbon_dioxide(),
            max_carbon_dioxide_time: statistics.get_max_carbon_dioxide_time(),
            min_carbon_dioxide: statistics.get_min_carbon_dioxide(),
            min_carbon_dioxide_time: statistics.get_min_carbon_dioxide_time(),
            max_methane: statistics.get_max_methane(),
            max_methane_time: statistics.get_max_methane_time(),
            min_methane: statistics.get_min_methane(),
            min_methane_time: statistics.get_min_methane_time(),
            alive_being_most_children_number: statistics.get_alive_being_most_children_number(),
            alive_being_most_kills_number: statistics.get_alive_being_most_kills_number(),
            alive_being_most_infections_number: statistics.get_alive_being_most_infections_number(),
            being_most_children_number: statistics.get_being_most_children_number(),
            being_most_children_time: statistics.get_being_most_children_time(),
            being_most_kills_number: statistics.get_being_most_kills_number(),
            being_most_kills_time: statistics.get_being_most_kills_time(),
            being_most_infections_number: statistics.get_being_most_infections_number(),
            being_most_infections_time: statistics.get_being_most_infections_time(),
        }
    }
}

/// Exact optional genotype identities selected during explicit refresh
#[derive(Clone, Debug, PartialEq)]
pub struct StatisticsGenomes {
    pub alive_being_most_children: Option<GenomeHandle>,
    pub alive_being_most_kills: Option<GenomeHandle>,
    pub alive_being_most_infections: Option<GenomeHandle>,
    pub being_most_children: Option<GenomeHandle>,
    pub being_most_kills: Option<GenomeHandle>,
    pub being_most_infections: Option<GenomeHandle>,
    pub last_born_being: Option<GenomeHandle>,
    pub last_dead_being: Option<GenomeHandle>,
    pub last_infected_being: Option<GenomeHandle>,
}

impl StatisticsGenomes {
    fn capture(statistics: &WorldStatistics) -> Self {
        Self {
            alive_being_most_children: statistics
                .get_alive_being_most_children()
                .map(|genome| GenomeHandle::from_shared(Rc::clone(genome))),
            alive_being_most_kills: statistics
                .get_alive_being_most_kills()
                .map(|genome| GenomeHandle::from_shared(Rc::clone(genome))),
            alive_being_most_infections: statistics
                .get_alive_being_most_infections()
                .map(|genome| GenomeHandle::from_shared(Rc::clone(genome))),
            being_most_children: statistics
                .get_being_most_children()
                .map(|genome| GenomeHandle::from_shared(Rc::clone(genome))),
            being_most_kills: statistics
                .get_being_most_kills()
                .map(|genome| GenomeHandle::from_shared(Rc::clone(genome))),
            being_most_infections: statistics
                .get_being_most_infections()
                .map(|genome| GenomeHandle::from_shared(Rc::clone(genome))),
            last_born_being: statistics
                .get_last_born_being()
                .map(|genome| GenomeHandle::from_shared(Rc::clone(genome))),
            last_dead_being: statistics
                .get_last_dead_being()
                .map(|genome| GenomeHandle::from_shared(Rc::clone(genome))),
            last_infected_being: statistics
                .get_last_infected_being()
                .map(|genome| GenomeHandle::from_shared(Rc::clone(genome))),
        }
    }
}

/// Detached nullable numeric slots, including the unplotted tail, at manual refresh
/// Detached slot descriptions do not cast values outside the plotted numeric prefix
#[derive(Clone, Debug, PartialEq)]
pub enum HistorySlotSnapshot { Null, DoubleBits(u64), OtherClass(&'static str) }

#[derive(Clone, Debug, PartialEq)]
pub struct StatisticsHistories {
    pub population_list: Vec<HistorySlotSnapshot>,
    pub death_list: Vec<HistorySlotSnapshot>,
    pub birth_list: Vec<HistorySlotSnapshot>,
    pub oxygen_list: Vec<HistorySlotSnapshot>,
    pub carbon_dioxide_list: Vec<HistorySlotSnapshot>,
    pub methane_list: Vec<HistorySlotSnapshot>,
}

impl StatisticsHistories {
    fn capture(statistics: &WorldStatistics) -> Self {
        fn slots(list: Option<&crate::history::HistoryListRef>) -> Vec<HistorySlotSnapshot> {
            list.expect("Graph success established non-null histories").shared_ref().slots().iter().map(|slot| match slot {
                crate::shared_world_owners::SharedSlot::Null => HistorySlotSnapshot::Null,
                crate::shared_world_owners::SharedSlot::Double(value) => HistorySlotSnapshot::DoubleBits(value.bits()),
                value => HistorySlotSnapshot::OtherClass(value.class_name()),
            }).collect()
        }
        Self {
            population_list: slots(statistics.get_population_list()),
            death_list: slots(statistics.get_death_list()),
            birth_list: slots(statistics.get_birth_list()),
            oxygen_list: slots(statistics.get_oxygen_list()),
            carbon_dioxide_list: slots(statistics.get_carbon_dioxide_list()),
            methane_list: slots(statistics.get_methane_list()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordKind {
    AliveChildren,
    HistoricalChildren,
    AliveKills,
    HistoricalKills,
    AliveInfections,
    HistoricalInfections,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordView {
    pub kind: RecordKind,
    pub genome: Option<GenomeHandle>,
    pub count: i32,
    pub time: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphSeriesKind {
    Deaths,
    Births,
    Population,
    Oxygen,
    CarbonDioxide,
    Methane,
}

/// Integer points use the original 100 by 80 logical area, including unbounded y coordinates
#[derive(Clone, Debug, PartialEq)]
pub struct GraphSeries {
    pub kind: GraphSeriesKind,
    pub argb: u32,
    pub minimum: f64,
    pub maximum: f64,
    pub points: Vec<[i32; 2]>,
}

impl GraphSeries {
    pub(crate) fn from_history(
        kind: GraphSeriesKind,
        argb: u32,
        history: Option<&crate::history::HistoryListRef>,
        field: &'static str,
        maximum: f64,
    ) -> Result<Self, crate::HistoryError> {
        let values = history.ok_or(crate::HistoryError::MissingList { field })?.numeric_prefix(field, 100)?;
        Ok(Self::new(kind, argb, &values, maximum))
    }

    fn new(kind: GraphSeriesKind, argb: u32, history: &[f64], maximum: f64) -> Self {
        let minimum = 0.0;
        let points = history
            .iter()
            .take(100)
            .enumerate()
            .map(|(index, sample)| {
                let y = 80.0 - (sample - minimum) * 80.0 / (maximum - minimum);
                [index as i32, y as i32]
            })
            .collect();
        Self {
            kind,
            argb,
            minimum,
            maximum,
            points,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorCount {
    pub source_palette_index: u8,
    pub argb: u32,
    pub count: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorRectangle {
    pub bounds: IntRect,
    pub argb: u32,
}

/// Frozen numeric and drawing data retain live genotype handles separately
#[derive(Clone, Debug, PartialEq)]
pub struct StatisticsView {
    pub current: WorldStatisticsValues,
    pub numbers: StatisticsNumbers,
    pub genomes: StatisticsGenomes,
    pub histories: StatisticsHistories,
    pub plot_series: [GraphSeries; 6],
    pub color_counts: [ColorCount; 46],
}

impl StatisticsView {
    pub fn record(&self, kind: RecordKind) -> RecordView {
        let n = &self.numbers;
        let g = &self.genomes;
        let (genome, count, time) = match kind {
            RecordKind::AliveChildren => (
                &g.alive_being_most_children,
                n.alive_being_most_children_number,
                None,
            ),
            RecordKind::HistoricalChildren => (
                &g.being_most_children,
                n.being_most_children_number,
                Some(n.being_most_children_time),
            ),
            RecordKind::AliveKills => (
                &g.alive_being_most_kills,
                n.alive_being_most_kills_number,
                None,
            ),
            RecordKind::HistoricalKills => (
                &g.being_most_kills,
                n.being_most_kills_number,
                Some(n.being_most_kills_time),
            ),
            RecordKind::AliveInfections => (
                &g.alive_being_most_infections,
                n.alive_being_most_infections_number,
                None,
            ),
            RecordKind::HistoricalInfections => (
                &g.being_most_infections,
                n.being_most_infections_number,
                Some(n.being_most_infections_time),
            ),
        };
        RecordView {
            kind,
            genome: genome.clone(),
            count,
            time,
        }
    }

    /// Repaint proportions without rescanning genes or refreshing the model
    pub fn color_rectangles(&self, width: i32, height: i32) -> Vec<ColorRectangle> {
        let total = self
            .color_counts
            .iter()
            .fold(0_i32, |sum, entry| sum.wrapping_add(entry.count));
        if total <= 0 {
            return Vec::new();
        }
        let mut x = 0_i32;
        self.color_counts
            .iter()
            .map(|entry| {
                let length = width.wrapping_mul(entry.count) / total;
                let rectangle = ColorRectangle {
                    bounds: IntRect {
                        x,
                        y: 0,
                        width: length,
                        height,
                    },
                    argb: entry.argb,
                };
                x = x.wrapping_add(length);
                rectangle
            })
            .collect()
    }
}

const PALETTE: [ColorKind; 46] = [
    ColorKind::Green,
    ColorKind::Forest,
    ColorKind::Spring,
    ColorKind::Summer,
    ColorKind::Lime,
    ColorKind::Leaf,
    ColorKind::C4,
    ColorKind::Jade,
    ColorKind::Grass,
    ColorKind::Bark,
    ColorKind::Purple,
    ColorKind::Red,
    ColorKind::Fire,
    ColorKind::Orange,
    ColorKind::Maroon,
    ColorKind::Pink,
    ColorKind::Cream,
    ColorKind::Silver,
    ColorKind::Spike,
    ColorKind::Lilac,
    ColorKind::Gray,
    ColorKind::Violet,
    ColorKind::Olive,
    ColorKind::Sky,
    ColorKind::Blue,
    ColorKind::Ochre,
    ColorKind::Fallow,
    ColorKind::Spore,
    ColorKind::White,
    ColorKind::Plague,
    ColorKind::Coral,
    ColorKind::Mint,
    ColorKind::Lavender,
    ColorKind::Magenta,
    ColorKind::Rose,
    ColorKind::Cyan,
    ColorKind::Teal,
    ColorKind::Yellow,
    ColorKind::Auburn,
    ColorKind::Indigo,
    ColorKind::Blond,
    ColorKind::Flower,
    ColorKind::DarkGray,
    ColorKind::Gold,
    ColorKind::Dark,
    ColorKind::Eye,
];

impl World {
    pub fn open_statistics(&self) -> StatisticsHandle {
        StatisticsHandle(Rc::clone(&self.statistics_ref()), self.handle())
    }

    /// Refresh against this World's current members while retaining the supplied statistics owner
    /// Frontends keep the handle bound to its original World and close it on World replacement
    pub fn refresh_statistics(
        &self,
        handle: &StatisticsHandle,
    ) -> Result<StatisticsView, StatisticsError> {
        let statistics = handle.0.borrow();
        let atmosphere = self.oxygen.get() + self.carbon_dioxide.get() + self.methane.get();
        let plot_series = [
            GraphSeries::from_history(GraphSeriesKind::Deaths, 0xffff0000, statistics.get_death_list(), "deathList",
                crate::java_math::max(statistics.get_average_population(), f64::from(statistics.get_max_deaths()))).map_err(StatisticsError::History)?,
            GraphSeries::from_history(GraphSeriesKind::Births, 0xff00ff00, statistics.get_birth_list(), "birthList",
                crate::java_math::max(statistics.get_average_population(), f64::from(statistics.get_max_birth()))).map_err(StatisticsError::History)?,
            GraphSeries::from_history(GraphSeriesKind::Population, 0xffffffff, statistics.get_population_list(), "populationList",
                f64::from(statistics.get_max_population())).map_err(StatisticsError::History)?,
            GraphSeries::from_history(GraphSeriesKind::Oxygen, 0xff0000ff, statistics.get_oxygen_list(), "oxygenList", atmosphere).map_err(StatisticsError::History)?,
            GraphSeries::from_history(GraphSeriesKind::CarbonDioxide, 0xffffffff, statistics.get_carbon_dioxide_list(), "carbonDioxideList", atmosphere).map_err(StatisticsError::History)?,
            GraphSeries::from_history(GraphSeriesKind::Methane, 0xffff00ff, statistics.get_methane_list(), "methaneList", atmosphere).map_err(StatisticsError::History)?,
        ];
        let histories = StatisticsHistories::capture(&statistics);
        drop(statistics);
        let mut colors = std::array::from_fn(|index| ColorCount {
            source_palette_index: index as u8,
            argb: PALETTE[index].argb().unwrap(),
            count: 0,
        });
        for index in 0..self.organisms.len().map_err(StatisticsError::List)? {
            let member = self.organisms.actor(index).map_err(StatisticsError::List)?;
            let member = member.borrow();
            let genome = member.genome.as_ref().ok_or(StatisticsError::MissingGenome { organism_id: member.id })?.borrow();
            for index in 0..genome.gene_count()? {
                let argb = genome
                    .gene_ref(index)?.borrow()
                    .color()
                    .ok_or(StatisticsError::MissingGeneColor {
                        organism_id: member.id,
                        gene_index: index,
                    })?;
                for color in &mut colors {
                    if color.argb == argb {
                        color.count = color.count.wrapping_add(1);
                    }
                }
            }
        }
        colors.sort_by_key(|color| color.count);
        colors.reverse();

        let current = WorldStatisticsValues {
            // New Game can leave handle pointing at the previous statistics object
            time: self.time(),
            population: self.population(),
            corpses: self.corpse_count().map_err(|error| { let WorldError::List(error) = error else { unreachable!() }; StatisticsError::List(error) })?,
            oxygen: self.oxygen.get(),
            carbon_dioxide: self.carbon_dioxide.get(),
            methane: self.methane.get(),
        };
        let mut statistics = handle.0.borrow_mut();
        statistics.find_best_alive_members(&self.organisms).map_err(StatisticsError::List)?;
        let numbers = StatisticsNumbers::capture(&statistics);
        let genomes = StatisticsGenomes::capture(&statistics);
        Ok(StatisticsView {
            current,
            numbers,
            genomes,
            histories,
            plot_series,
            color_counts: colors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gene, GenomeProperties};

    fn world() -> World {
        World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap()
    }

    fn add(world: &mut World, colors: &[u32], symmetry: i32, x: i32) -> OrganismHandle {
        let genes: Vec<_> = colors
            .iter()
            .map(|color| Gene::new(12.0, 0.0, Some(*color), 0, [0; 36]).shared())
            .collect();
        let code = Genome::from_parts(
            &genes,
            GenomeProperties {
                symmetry,
                ..GenomeProperties::default()
            },
        );
        let id = world.paste_genome(code, [x, 100]).unwrap().unwrap();
        OrganismHandle(Rc::clone(
            world.organisms.borrow().iter()
                .find(|actor| actor.borrow().id == id)
                .unwrap(),
        ), world.handle())
    }

    #[test]
    fn new_game_retains_old_statistics_and_refreshes_actual_current_members() {
        let mut world = world();
        let parent = add(&mut world, &[0xff00ff00], 1, 100);
        let child = add(&mut world, &[0xffff0000], 1, 250);
        parent.0.borrow_mut().state.total_children = 4;
        world
            .statistics_ref()
            .borrow_mut()
            .event_organism_born(&child.0.borrow(), &parent.0.borrow());
        world.statistics_ref().borrow_mut().event_time(2, 7.0, 8.0, 9.0).unwrap();
        let old = world.open_statistics();
        let weak = Rc::downgrade(&old.0);
        assert_eq!(old, world.open_statistics());
        let old_genome = world.inspect_genome(&parent).unwrap().unwrap();
        let parameters = world.parameters().clone();
        world.new_game(1200, 800, 0, parameters).unwrap();
        let current = world.open_statistics();
        assert_ne!(old, current);
        let next = add(&mut world, &[0xff0000ff], 1, 100);
        next.0.borrow_mut().state.total_children = 7;
        let next_genome = world.inspect_genome(&next).unwrap().unwrap();
        let before = world.clone();
        let view = world.refresh_statistics(&old).unwrap();
        assert_eq!(world, before);
        assert_eq!((view.current.time, view.numbers.time), (0, 1));
        assert_eq!(
            (view.current.population, view.numbers.created_organisms),
            (1, 2)
        );
        assert_eq!(view.histories.population_list, [HistorySlotSnapshot::DoubleBits(2.0_f64.to_bits())]);
        assert_eq!(
            view.record(RecordKind::HistoricalChildren).genome,
            Some(old_genome)
        );
        assert_eq!(
            view.record(RecordKind::AliveChildren).genome,
            Some(next_genome)
        );
        assert_eq!(
            world
                .statistics_ref()
                .borrow()
                .get_alive_being_most_children_number(),
            0
        );
        assert_eq!(view.record(RecordKind::AliveChildren).count, 7);
        drop(old);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn record_views_keep_live_genotypes_after_later_champion_replacement() {
        let mut world = world();
        let first = add(&mut world, &[0xff00ff00], 1, 100);
        let second = add(&mut world, &[0xffff0000], 1, 250);
        first.0.borrow_mut().state.total_children = 5;
        let owner = world.open_statistics();
        let old = world.refresh_statistics(&owner).unwrap();
        let record = old.record(RecordKind::AliveChildren);
        let retained = record.genome.unwrap();
        assert_eq!(retained, world.inspect_genome(&first).unwrap().unwrap());
        first
            .0
            .borrow()
            .genome.as_ref().unwrap().borrow_mut()
            .set_clade_id(Some("changed-after-refresh".to_owned()));
        second.0.borrow_mut().state.total_children = 9;
        world.statistics_ref().borrow_mut().event_time(2, 1.0, 2.0, 3.0).unwrap();
        let new = world.refresh_statistics(&owner).unwrap();
        assert_eq!(old.record(RecordKind::AliveChildren).count, 5);
        assert_eq!(
            retained.detached_clone().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(),
            Some("changed-after-refresh")
        );
        assert_ne!(Some(retained), new.record(RecordKind::AliveChildren).genome);
        assert!(old.histories.population_list.is_empty());
        assert_eq!(new.histories.population_list, [HistorySlotSnapshot::DoubleBits(2.0_f64.to_bits())]);
    }

    #[test]
    fn world_clone_uses_a_fresh_owner_and_preserves_copied_actor_record_aliases() {
        let mut world = world();
        let first = add(&mut world, &[0xff00ff00], 1, 100);
        let second = add(&mut world, &[0xffff0000], 1, 250);
        first.0.borrow_mut().state.total_children = 5;
        world
            .statistics_ref()
            .borrow_mut()
            .event_organism_born(&second.0.borrow(), &first.0.borrow());
        let source_owner = world.open_statistics();
        let source_view = world.refresh_statistics(&source_owner).unwrap();
        let mut copy = world.clone();
        let copy_owner = copy.open_statistics();
        assert_ne!(source_owner, copy_owner);
        let copy_view = copy.refresh_statistics(&copy_owner).unwrap();
        let copied_actor = OrganismHandle(Rc::clone(&copy.organisms.borrow()[0]), copy.handle());
        let copied_record = copy_view
            .record(RecordKind::HistoricalChildren)
            .genome
            .unwrap();
        assert_eq!(copied_record, copy.inspect_genome(&copied_actor).unwrap().unwrap());
        assert_ne!(
            Some(copied_record.clone()),
            source_view.record(RecordKind::HistoricalChildren).genome
        );
        copied_actor
            .0
            .borrow()
            .genome.as_ref().unwrap().borrow_mut()
            .set_clade_id(Some("only-copy".to_owned()));
        assert_eq!(copied_record.detached_clone().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("only-copy"));
        assert_ne!(
            world
                .inspect_genome(&first)
                .unwrap().unwrap()
                .detached_clone()
                .clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(),
            Some("only-copy")
        );
        add(&mut copy, &[0xff0000ff], 1, 400);
        assert_eq!(world.statistics_ref().borrow().get_created_organisms(), 2);
        assert_eq!(copy.statistics_ref().borrow().get_created_organisms(), 3);
    }

    #[test]
    fn color_counts_use_genetic_slots_corpses_exact_argb_and_reverse_stable_ties() {
        let mut world = world();
        let corpse = add(&mut world, &[0xff00ff00, 0xff00ff00, 0xff004040], 4, 100);
        add(&mut world, &[0xffff0000, 0x0100ff00], 1, 400);
        world.die(&mut corpse.0.borrow_mut(), None).unwrap();
        let view = world.refresh_statistics(&world.open_statistics()).unwrap();
        assert_eq!(view.current.corpses, 1);
        assert_eq!(
            view.color_counts
                .iter()
                .map(|entry| entry.count)
                .sum::<i32>(),
            4
        );
        assert_eq!(
            view.color_counts[0],
            ColorCount {
                source_palette_index: 0,
                argb: 0xff00ff00,
                count: 2
            }
        );
        assert_eq!(
            view.color_counts[1],
            ColorCount {
                source_palette_index: 45,
                argb: 0xff004040,
                count: 1
            }
        );
        assert_eq!(
            view.color_counts[2],
            ColorCount {
                source_palette_index: 11,
                argb: 0xffff0000,
                count: 1
            }
        );
        assert!(
            view.color_counts[3..]
                .windows(2)
                .all(|pair| pair[0].source_palette_index > pair[1].source_palette_index)
        );
    }

    #[test]
    fn graph_commands_preserve_source_prefix_ieee_casts_and_unclamped_points() {
        let samples = [
            0.0,
            25.0,
            50.0,
            100.0,
            -25.0,
            125.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];
        let graph = GraphSeries::new(GraphSeriesKind::Population, 0xffffffff, &samples, 100.0);
        assert_eq!(
            graph.points,
            [
                [0, 80],
                [1, 60],
                [2, 40],
                [3, 0],
                [4, 100],
                [5, -20],
                [6, 0],
                [7, i32::MIN],
                [8, i32::MAX]
            ]
        );
        let zero = GraphSeries::new(
            GraphSeriesKind::Population,
            0xffffffff,
            &[0.0, 1.0, -1.0],
            0.0,
        );
        assert_eq!(zero.points, [[0, 0], [1, i32::MIN], [2, i32::MAX]]);
        let negative_zero = GraphSeries::new(GraphSeriesKind::Population, 0xffffffff, &[1.0], -0.0);
        assert_eq!(negative_zero.points, [[0, i32::MAX]]);
        let long = GraphSeries::new(
            GraphSeriesKind::Births,
            0xff00ff00,
            &(0..101).map(f64::from).collect::<Vec<_>>(),
            100.0,
        );
        assert_eq!((long.points.len(), long.points[99]), (100, [99, 0]));
        assert!(
            GraphSeries::new(GraphSeriesKind::Deaths, 0xffff0000, &[], 0.0)
                .points
                .is_empty()
        );
    }

    #[test]
    fn color_rectangles_keep_truncation_remainders_and_wrapping_products() {
        let world = world();
        let mut view = world.refresh_statistics(&world.open_statistics()).unwrap();
        assert!(view.color_rectangles(640, 25).is_empty());
        view.color_counts[0].count = 2;
        view.color_counts[1].count = 1;
        let small = view.color_rectangles(2, 25);
        assert_eq!((small[0].bounds.width, small[1].bounds.width), (1, 0));
        assert_eq!(small[2].bounds.x, 1);
        let wrapped = view.color_rectangles(i32::MAX, 25);
        assert_eq!(wrapped[0].bounds.width, 0);
        assert_eq!(wrapped[1].bounds.width, 715_827_882);
        view.color_counts[0].count = i32::MAX;
        assert!(view.color_rectangles(640, 25).is_empty());
    }

    #[test]
    fn missing_color_fails_before_the_source_alive_scan_changes_records() {
        let mut world = world();
        let actor = add(&mut world, &[0xff00ff00], 1, 100);
        actor.0.borrow_mut().state.total_children = 5;
        let handle = world.open_statistics();
        world.refresh_statistics(&handle).unwrap();
        actor.0.borrow_mut().state.total_children = 10;
        actor.0.borrow().genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow_mut().set_color(None);
        let random = world.random_snapshot();
        assert_eq!(
            world.refresh_statistics(&handle),
            Err(StatisticsError::MissingGeneColor {
                organism_id: 0,
                gene_index: 0
            })
        );
        assert_eq!(
            world
                .statistics_ref()
                .borrow()
                .get_alive_being_most_children_number(),
            5
        );
        assert_eq!(world.random_snapshot(), random);
        actor
            .0
            .borrow()
            .genome.as_ref().unwrap().borrow()
            .gene_ref(0).unwrap().borrow_mut()
            .set_color(Some(0xff00ff00));
        assert_eq!(
            world
                .refresh_statistics(&handle)
                .unwrap()
                .record(RecordKind::AliveChildren)
                .count,
            10
        );
    }
}

#[cfg(test)]
#[path = "statistics_ui_source_tests.rs"]
mod source_tests;
