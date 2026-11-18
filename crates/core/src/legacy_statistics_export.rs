// SPDX-License-Identifier: GPL-2.0-or-later

use super::WorldStatistics;
use crate::object_stream::FieldValue;
use crate::world::legacy_export::{ExportResult, WorldEmitter, WorldExportError};
use crate::legacy_graph::{GraphError, GraphErrorKind};

impl WorldStatistics {
    pub(crate) fn legacy_export_field(&self, emitter: &mut WorldEmitter, field: &'static str) -> ExportResult<FieldValue> {
        Ok(match field {
            "birthList" => FieldValue::Reference(emitter.history(self.birth_list.as_ref())?),
            "carbonDioxideList" => FieldValue::Reference(emitter.history(self.carbon_dioxide_list.as_ref())?),
            "deathList" => FieldValue::Reference(emitter.history(self.death_list.as_ref())?),
            "methaneList" => FieldValue::Reference(emitter.history(self.methane_list.as_ref())?),
            "oxygenList" => FieldValue::Reference(emitter.history(self.oxygen_list.as_ref())?),
            "populationList" => FieldValue::Reference(emitter.history(self.population_list.as_ref())?),
            "aliveBeingMostChildren" => FieldValue::Reference(emitter.genome(self.alive_being_most_children.as_ref())?),
            "aliveBeingMostChildrenNumber" => FieldValue::Int(self.alive_being_most_children_number),
            "aliveBeingMostInfections" => FieldValue::Reference(emitter.genome(self.alive_being_most_infections.as_ref())?),
            "aliveBeingMostInfectionsNumber" => FieldValue::Int(self.alive_being_most_infections_number),
            "aliveBeingMostKills" => FieldValue::Reference(emitter.genome(self.alive_being_most_kills.as_ref())?),
            "aliveBeingMostKillsNumber" => FieldValue::Int(self.alive_being_most_kills_number),
            "beingMostChildren" => FieldValue::Reference(emitter.genome(self.being_most_children.as_ref())?),
            "beingMostChildrenNumber" => FieldValue::Int(self.being_most_children_number),
            "beingMostChildrenTime" => FieldValue::Long(self.being_most_children_time),
            "beingMostInfections" => FieldValue::Reference(emitter.genome(self.being_most_infections.as_ref())?),
            "beingMostInfectionsNumber" => FieldValue::Int(self.being_most_infections_number),
            "beingMostInfectionsTime" => FieldValue::Long(self.being_most_infections_time),
            "beingMostKills" => FieldValue::Reference(emitter.genome(self.being_most_kills.as_ref())?),
            "beingMostKillsNumber" => FieldValue::Int(self.being_most_kills_number),
            "beingMostKillsTime" => FieldValue::Long(self.being_most_kills_time),
            "birthLastTime" => FieldValue::Int(self.birth_last_time),
            "birthSum" => FieldValue::Long(self.birth_sum),
            "createdOrganisms" => FieldValue::Int(self.created_organisms),
            "deathLastTime" => FieldValue::Int(self.death_last_time),
            "deathSum" => FieldValue::Long(self.death_sum),
            "infectionsSum" => FieldValue::Long(self.infections_sum),
            "lastBornBeing" => FieldValue::Reference(emitter.genome(self.last_born_being.as_ref())?),
            "lastDeadBeing" => FieldValue::Reference(emitter.genome(self.last_dead_being.as_ref())?),
            "lastInfectedBeing" => FieldValue::Reference(emitter.genome(self.last_infected_being.as_ref())?),
            "massExtintionState" => FieldValue::Int(self.mass_extinction_state),
            "massExtintions" => FieldValue::Int(self.mass_extinctions),
            "maxBirths" => FieldValue::Int(self.max_births),
            "maxCarbonDioxide" => FieldValue::DoubleBits(self.max_carbon_dioxide.to_bits()),
            "maxCarbonDioxideTime" => FieldValue::Long(self.max_carbon_dioxide_time),
            "maxDeaths" => FieldValue::Int(self.max_deaths),
            "maxMethane" => FieldValue::DoubleBits(self.max_methane.to_bits()),
            "maxMethaneTime" => FieldValue::Long(self.max_methane_time),
            "maxOxygen" => FieldValue::DoubleBits(self.max_oxygen.to_bits()),
            "maxOxygenTime" => FieldValue::Long(self.max_oxygen_time),
            "maxPopulation" => FieldValue::Int(self.max_population),
            "maxPopulationTime" => FieldValue::Long(self.max_population_time),
            "minCarbonDioxide" => FieldValue::DoubleBits(self.min_carbon_dioxide.to_bits()),
            "minCarbonDioxideTime" => FieldValue::Long(self.min_carbon_dioxide_time),
            "minMethane" => FieldValue::DoubleBits(self.min_methane.to_bits()),
            "minMethaneTime" => FieldValue::Long(self.min_methane_time),
            "minOxygen" => FieldValue::DoubleBits(self.min_oxygen.to_bits()),
            "minOxygenTime" => FieldValue::Long(self.min_oxygen_time),
            "minPopulation" => FieldValue::Int(self.min_population),
            "minPopulationTime" => FieldValue::Long(self.min_population_time),
            "populationSum" => FieldValue::Long(self.population_sum),
            "time" => FieldValue::Long(self.time),
            _ => return Err(WorldExportError::Graph(GraphError { node: None, field: Some(field), kind: GraphErrorKind::Field })),
        })
    }
}
