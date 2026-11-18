// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Constructor-free stored statistics assembly with shared per-Document owners

use super::WorldStatistics;
use crate::legacy_genetics::GeneticsStager;
use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::legacy_history::HistorySource;
use crate::object_stream::Value;
use std::{cell::RefCell, mem::size_of, rc::Rc};

const CLASS: &str = "biogenesis.WorldStatistics";
pub(crate) type StatisticsRef = Rc<RefCell<WorldStatistics>>;

fn error(kind: GraphErrorKind) -> GraphError {
    GraphError { node: None, field: None, kind }
}

/// Reuse this table and the supplied contexts for every World in one import batch
pub(crate) struct StatisticsStager<'graph, 'document> {
    graph: &'graph LegacyGraph<'document>,
    owners: Vec<Option<StatisticsRef>>,
    maximum: usize,
    used: usize,
}

impl<'graph, 'document> StatisticsStager<'graph, 'document> {
    pub(crate) fn new(
        graph: &'graph LegacyGraph<'document>,
        additional_bytes: usize,
    ) -> Result<Self, GraphError> {
        let maximum = additional_bytes.min(graph.limits().allocated_bytes);
        let count = graph.arena().nodes.len();
        graph.charge(count)?;
        let bytes = count.checked_mul(size_of::<Option<StatisticsRef>>())
            .ok_or(error(GraphErrorKind::Budget))?;
        if bytes > maximum { return Err(error(GraphErrorKind::Budget)); }
        graph.allocate(bytes)?;
        let mut owners = Vec::new();
        owners.try_reserve_exact(count).map_err(|_| error(GraphErrorKind::Allocation))?;
        let used = owners.capacity().checked_mul(size_of::<Option<StatisticsRef>>())
            .ok_or(error(GraphErrorKind::Budget))?;
        if used > maximum { return Err(error(GraphErrorKind::Budget)); }
        graph.allocate(used - bytes)?;
        owners.resize_with(count, || None);
        Ok(Self { graph, owners, maximum, used })
    }

    pub(crate) fn allocated_bytes(&self) -> usize { self.used }

    /// A failure never changes an existing statistics owner or publishes a partial owner
    pub(crate) fn statistics(
        &mut self,
        value: Value,
        genetics: &mut GeneticsStager<'graph, 'document>,
        histories: &mut impl HistorySource,
    ) -> Result<Option<StatisticsRef>, GraphError> {
        if !genetics.uses_graph(self.graph) || !histories.uses_graph(self.graph) {
            return Err(error(GraphErrorKind::Reference));
        }
        self.graph.charge(1)?;
        let Value::Handle(id) = value else { return Ok(None); };
        self.graph.node(id)?;
        if let Some(owner) = &self.owners[id.0 as usize] {
            return Ok(Some(Rc::clone(owner)));
        }
        genetics.object(id, CLASS)?;
        let bytes = size_of::<RefCell<WorldStatistics>>() + 2 * size_of::<usize>();
        let next = self.used.checked_add(bytes).ok_or(error(GraphErrorKind::Budget))?;
        if next > self.maximum { return Err(error(GraphErrorKind::Budget)); }
        self.graph.allocate(bytes)?;
        self.used = next;
        let statistics = WorldStatistics {
            alive_being_most_children_number: self.graph.int(id, CLASS, "aliveBeingMostChildrenNumber")?,
            alive_being_most_infections_number: self.graph.int(id, CLASS, "aliveBeingMostInfectionsNumber")?,
            alive_being_most_kills_number: self.graph.int(id, CLASS, "aliveBeingMostKillsNumber")?,
            being_most_children_number: self.graph.int(id, CLASS, "beingMostChildrenNumber")?,
            being_most_children_time: self.graph.long(id, CLASS, "beingMostChildrenTime")?,
            being_most_infections_number: self.graph.int(id, CLASS, "beingMostInfectionsNumber")?,
            being_most_infections_time: self.graph.long(id, CLASS, "beingMostInfectionsTime")?,
            being_most_kills_number: self.graph.int(id, CLASS, "beingMostKillsNumber")?,
            being_most_kills_time: self.graph.long(id, CLASS, "beingMostKillsTime")?,
            birth_last_time: self.graph.int(id, CLASS, "birthLastTime")?,
            birth_sum: self.graph.long(id, CLASS, "birthSum")?,
            created_organisms: self.graph.int(id, CLASS, "createdOrganisms")?,
            death_last_time: self.graph.int(id, CLASS, "deathLastTime")?,
            death_sum: self.graph.long(id, CLASS, "deathSum")?,
            infections_sum: self.graph.long(id, CLASS, "infectionsSum")?,
            mass_extinction_state: self.graph.int(id, CLASS, "massExtintionState")?,
            mass_extinctions: self.graph.int(id, CLASS, "massExtintions")?,
            max_births: self.graph.int(id, CLASS, "maxBirths")?,
            max_carbon_dioxide: f64::from_bits(self.graph.double_bits(id, CLASS, "maxCarbonDioxide")?),
            max_carbon_dioxide_time: self.graph.long(id, CLASS, "maxCarbonDioxideTime")?,
            max_deaths: self.graph.int(id, CLASS, "maxDeaths")?,
            max_methane: f64::from_bits(self.graph.double_bits(id, CLASS, "maxMethane")?),
            max_methane_time: self.graph.long(id, CLASS, "maxMethaneTime")?,
            max_oxygen: f64::from_bits(self.graph.double_bits(id, CLASS, "maxOxygen")?),
            max_oxygen_time: self.graph.long(id, CLASS, "maxOxygenTime")?,
            max_population: self.graph.int(id, CLASS, "maxPopulation")?,
            max_population_time: self.graph.long(id, CLASS, "maxPopulationTime")?,
            min_carbon_dioxide: f64::from_bits(self.graph.double_bits(id, CLASS, "minCarbonDioxide")?),
            min_carbon_dioxide_time: self.graph.long(id, CLASS, "minCarbonDioxideTime")?,
            min_methane: f64::from_bits(self.graph.double_bits(id, CLASS, "minMethane")?),
            min_methane_time: self.graph.long(id, CLASS, "minMethaneTime")?,
            min_oxygen: f64::from_bits(self.graph.double_bits(id, CLASS, "minOxygen")?),
            min_oxygen_time: self.graph.long(id, CLASS, "minOxygenTime")?,
            min_population: self.graph.int(id, CLASS, "minPopulation")?,
            min_population_time: self.graph.long(id, CLASS, "minPopulationTime")?,
            population_sum: self.graph.long(id, CLASS, "populationSum")?,
            time: self.graph.long(id, CLASS, "time")?,
            alive_being_most_children: genetics.genome(self.graph.reference(id, CLASS, "aliveBeingMostChildren")?)?,
            alive_being_most_infections: genetics.genome(self.graph.reference(id, CLASS, "aliveBeingMostInfections")?)?,
            alive_being_most_kills: genetics.genome(self.graph.reference(id, CLASS, "aliveBeingMostKills")?)?,
            being_most_children: genetics.genome(self.graph.reference(id, CLASS, "beingMostChildren")?)?,
            being_most_infections: genetics.genome(self.graph.reference(id, CLASS, "beingMostInfections")?)?,
            being_most_kills: genetics.genome(self.graph.reference(id, CLASS, "beingMostKills")?)?,
            birth_list: histories.list(self.graph.reference(id, CLASS, "birthList")?)?,
            carbon_dioxide_list: histories.list(self.graph.reference(id, CLASS, "carbonDioxideList")?)?,
            death_list: histories.list(self.graph.reference(id, CLASS, "deathList")?)?,
            last_born_being: genetics.genome(self.graph.reference(id, CLASS, "lastBornBeing")?)?,
            last_dead_being: genetics.genome(self.graph.reference(id, CLASS, "lastDeadBeing")?)?,
            last_infected_being: genetics.genome(self.graph.reference(id, CLASS, "lastInfectedBeing")?)?,
            methane_list: histories.list(self.graph.reference(id, CLASS, "methaneList")?)?,
            oxygen_list: histories.list(self.graph.reference(id, CLASS, "oxygenList")?)?,
            population_list: histories.list(self.graph.reference(id, CLASS, "populationList")?)?,
        };
        let owner = statistics.into_shared();
        self.owners[id.0 as usize] = Some(Rc::clone(&owner));
        Ok(Some(owner))
    }
}

#[cfg(test)]
#[path = "legacy_statistics_tests.rs"]
mod tests;
