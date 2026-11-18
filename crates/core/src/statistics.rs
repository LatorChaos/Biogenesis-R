// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::history::{HistoryError, HistoryListRef};
use crate::organism::{GenomeCloner, GenomeRef, Organism, OrganismRef};
use crate::{GenomeHandle, RecordKind, RecordView, World};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[path = "legacy_statistics.rs"]
pub(crate) mod legacy;

const EXTINCTION_NO: i32 = 0;
const EXTINCTION_POSSIBLE: i32 = 1;
const EXTINCTION_CONFIRMED: i32 = 2;
const EXTINCTION_FINISHING: i32 = 3;

/// Synchronous statistics retain live genotype identities, including records for removed organisms
#[derive(Debug, PartialEq)]
pub(crate) struct WorldStatistics {
    time: i64,
    max_population: i32,
    max_births: i32,
    max_deaths: i32,
    max_population_time: i64,
    min_population: i32,
    min_population_time: i64,
    mass_extinctions: i32,
    mass_extinction_state: i32,
    created_organisms: i32,
    population_sum: i64,
    death_sum: i64,
    death_last_time: i32,
    birth_sum: i64,
    birth_last_time: i32,
    infections_sum: i64,
    max_oxygen: f64,
    max_oxygen_time: i64,
    min_oxygen: f64,
    min_oxygen_time: i64,
    max_carbon_dioxide: f64,
    max_methane: f64,
    max_carbon_dioxide_time: i64,
    max_methane_time: i64,
    min_carbon_dioxide: f64,
    min_methane: f64,
    min_carbon_dioxide_time: i64,
    min_methane_time: i64,
    alive_being_most_children: Option<GenomeRef>,
    alive_being_most_children_number: i32,
    alive_being_most_kills: Option<GenomeRef>,
    alive_being_most_kills_number: i32,
    alive_being_most_infections: Option<GenomeRef>,
    alive_being_most_infections_number: i32,
    being_most_children: Option<GenomeRef>,
    being_most_children_number: i32,
    being_most_children_time: i64,
    being_most_kills: Option<GenomeRef>,
    being_most_kills_number: i32,
    being_most_kills_time: i64,
    being_most_infections: Option<GenomeRef>,
    being_most_infections_number: i32,
    being_most_infections_time: i64,
    last_born_being: Option<GenomeRef>,
    last_dead_being: Option<GenomeRef>,
    last_infected_being: Option<GenomeRef>,
    pub(crate) population_list: Option<HistoryListRef>,
    pub(crate) death_list: Option<HistoryListRef>,
    pub(crate) birth_list: Option<HistoryListRef>,
    pub(crate) oxygen_list: Option<HistoryListRef>,
    pub(crate) carbon_dioxide_list: Option<HistoryListRef>,
    pub(crate) methane_list: Option<HistoryListRef>,
}

impl WorldStatistics {
    pub(crate) fn retained_genomes(&self) -> [Option<&GenomeRef>; 9] {
        [
            self.alive_being_most_children.as_ref(), self.alive_being_most_kills.as_ref(),
            self.alive_being_most_infections.as_ref(), self.being_most_children.as_ref(),
            self.being_most_kills.as_ref(), self.being_most_infections.as_ref(),
            self.last_born_being.as_ref(), self.last_dead_being.as_ref(),
            self.last_infected_being.as_ref(),
        ]
    }

    /// Share the completed statistics owner without cloning its records or histories
    pub(crate) fn into_shared(self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self))
    }

    /// Capture configured initial attempts and gases before any successful placement or time event
    pub(crate) fn new(
        initial_attempts: i32,
        initial_oxygen: f64,
        initial_carbon_dioxide: f64,
        initial_methane: f64,
    ) -> Self {
        Self {
            time: 0,
            max_population: 0,
            max_births: 0,
            max_deaths: 0,
            max_population_time: 0,
            min_population: initial_attempts,
            min_population_time: 0,
            mass_extinctions: 0,
            mass_extinction_state: EXTINCTION_NO,
            created_organisms: 0,
            population_sum: 0,
            death_sum: 0,
            death_last_time: 0,
            birth_sum: 0,
            birth_last_time: 0,
            infections_sum: 0,
            max_oxygen: initial_oxygen,
            max_oxygen_time: 0,
            min_oxygen: initial_oxygen,
            min_oxygen_time: 0,
            max_carbon_dioxide: initial_carbon_dioxide,
            max_methane: initial_methane,
            max_carbon_dioxide_time: 0,
            max_methane_time: 0,
            min_carbon_dioxide: initial_carbon_dioxide,
            min_methane: initial_methane,
            min_carbon_dioxide_time: 0,
            min_methane_time: 0,
            alive_being_most_children: None,
            alive_being_most_children_number: 0,
            alive_being_most_kills: None,
            alive_being_most_kills_number: 0,
            alive_being_most_infections: None,
            alive_being_most_infections_number: 0,
            being_most_children: None,
            being_most_children_number: 0,
            being_most_children_time: 0,
            being_most_kills: None,
            being_most_kills_number: 0,
            being_most_kills_time: 0,
            being_most_infections: None,
            being_most_infections_number: 0,
            being_most_infections_time: 0,
            last_born_being: None,
            last_dead_being: None,
            last_infected_being: None,
            population_list: Some(HistoryListRef::empty()),
            death_list: Some(HistoryListRef::empty()),
            birth_list: Some(HistoryListRef::empty()),
            oxygen_list: Some(HistoryListRef::empty()),
            carbon_dioxide_list: Some(HistoryListRef::empty()),
            methane_list: Some(HistoryListRef::empty()),
        }
    }

    pub(crate) fn get_time(&self) -> i64 {
        self.time
    }
    pub(crate) fn get_max_population(&self) -> i32 {
        self.max_population
    }
    pub(crate) fn get_max_birth(&self) -> i32 {
        self.max_births
    }
    pub(crate) fn get_max_deaths(&self) -> i32 {
        self.max_deaths
    }
    pub(crate) fn get_max_population_time(&self) -> i64 {
        self.max_population_time
    }
    pub(crate) fn get_min_population(&self) -> i32 {
        self.min_population
    }
    pub(crate) fn get_min_population_time(&self) -> i64 {
        self.min_population_time
    }
    pub(crate) fn get_mass_extinctions(&self) -> i32 {
        self.mass_extinctions
    }
    pub(crate) fn get_created_organisms(&self) -> i32 {
        self.created_organisms
    }

    pub(crate) fn get_average_population(&self) -> f64 {
        if self.time > 0 {
            self.population_sum as f64 / self.time as f64
        } else {
            0.0
        }
    }

    pub(crate) fn get_average_deaths(&self) -> f64 {
        if self.time > 0 {
            self.death_sum as f64 / self.time as f64
        } else {
            0.0
        }
    }

    pub(crate) fn get_average_births(&self) -> f64 {
        if self.time > 0 {
            self.birth_sum as f64 / self.time as f64
        } else {
            0.0
        }
    }

    pub(crate) fn get_average_infections(&self) -> f64 {
        if self.time > 0 {
            self.infections_sum as f64 / self.time as f64
        } else {
            0.0
        }
    }

    pub(crate) fn get_max_oxygen(&self) -> f64 {
        self.max_oxygen
    }
    pub(crate) fn get_max_oxygen_time(&self) -> i64 {
        self.max_oxygen_time
    }
    pub(crate) fn get_min_oxygen(&self) -> f64 {
        self.min_oxygen
    }
    pub(crate) fn get_min_oxygen_time(&self) -> i64 {
        self.min_oxygen_time
    }
    pub(crate) fn get_max_carbon_dioxide(&self) -> f64 {
        self.max_carbon_dioxide
    }
    pub(crate) fn get_max_carbon_dioxide_time(&self) -> i64 {
        self.max_carbon_dioxide_time
    }
    pub(crate) fn get_min_carbon_dioxide(&self) -> f64 {
        self.min_carbon_dioxide
    }
    pub(crate) fn get_min_carbon_dioxide_time(&self) -> i64 {
        self.min_carbon_dioxide_time
    }
    pub(crate) fn get_max_methane(&self) -> f64 {
        self.max_methane
    }
    pub(crate) fn get_max_methane_time(&self) -> i64 {
        self.max_methane_time
    }
    pub(crate) fn get_min_methane(&self) -> f64 {
        self.min_methane
    }
    pub(crate) fn get_min_methane_time(&self) -> i64 {
        self.min_methane_time
    }
    pub(crate) fn get_alive_being_most_children(&self) -> Option<&GenomeRef> {
        self.alive_being_most_children.as_ref()
    }
    pub(crate) fn get_alive_being_most_children_number(&self) -> i32 {
        self.alive_being_most_children_number
    }
    pub(crate) fn get_alive_being_most_kills(&self) -> Option<&GenomeRef> {
        self.alive_being_most_kills.as_ref()
    }
    pub(crate) fn get_alive_being_most_kills_number(&self) -> i32 {
        self.alive_being_most_kills_number
    }
    pub(crate) fn get_alive_being_most_infections(&self) -> Option<&GenomeRef> {
        self.alive_being_most_infections.as_ref()
    }
    pub(crate) fn get_alive_being_most_infections_number(&self) -> i32 {
        self.alive_being_most_infections_number
    }
    pub(crate) fn get_being_most_children(&self) -> Option<&GenomeRef> {
        self.being_most_children.as_ref()
    }
    pub(crate) fn get_being_most_children_number(&self) -> i32 {
        self.being_most_children_number
    }
    pub(crate) fn get_being_most_children_time(&self) -> i64 {
        self.being_most_children_time
    }
    pub(crate) fn get_being_most_kills(&self) -> Option<&GenomeRef> {
        self.being_most_kills.as_ref()
    }
    pub(crate) fn get_being_most_kills_number(&self) -> i32 {
        self.being_most_kills_number
    }
    pub(crate) fn get_being_most_kills_time(&self) -> i64 {
        self.being_most_kills_time
    }
    pub(crate) fn get_being_most_infections(&self) -> Option<&GenomeRef> {
        self.being_most_infections.as_ref()
    }
    pub(crate) fn get_being_most_infections_number(&self) -> i32 {
        self.being_most_infections_number
    }
    pub(crate) fn get_being_most_infections_time(&self) -> i64 {
        self.being_most_infections_time
    }
    pub(crate) fn get_last_born_being(&self) -> Option<&GenomeRef> {
        self.last_born_being.as_ref()
    }
    pub(crate) fn get_last_dead_being(&self) -> Option<&GenomeRef> {
        self.last_dead_being.as_ref()
    }
    pub(crate) fn get_last_infected_being(&self) -> Option<&GenomeRef> {
        self.last_infected_being.as_ref()
    }

    /// Return live nullable list references without reading their elements
    pub(crate) fn eq_non_histories(&self, other: &Self) -> bool {
        self.time == other.time
            && self.max_population == other.max_population
            && self.max_births == other.max_births
            && self.max_deaths == other.max_deaths
            && self.max_population_time == other.max_population_time
            && self.min_population == other.min_population
            && self.min_population_time == other.min_population_time
            && self.mass_extinctions == other.mass_extinctions
            && self.mass_extinction_state == other.mass_extinction_state
            && self.created_organisms == other.created_organisms
            && self.population_sum == other.population_sum
            && self.death_sum == other.death_sum
            && self.death_last_time == other.death_last_time
            && self.birth_sum == other.birth_sum
            && self.birth_last_time == other.birth_last_time
            && self.infections_sum == other.infections_sum
            && self.max_oxygen == other.max_oxygen
            && self.max_oxygen_time == other.max_oxygen_time
            && self.min_oxygen == other.min_oxygen
            && self.min_oxygen_time == other.min_oxygen_time
            && self.max_carbon_dioxide == other.max_carbon_dioxide
            && self.max_methane == other.max_methane
            && self.max_carbon_dioxide_time == other.max_carbon_dioxide_time
            && self.max_methane_time == other.max_methane_time
            && self.min_carbon_dioxide == other.min_carbon_dioxide
            && self.min_methane == other.min_methane
            && self.min_carbon_dioxide_time == other.min_carbon_dioxide_time
            && self.min_methane_time == other.min_methane_time
            && self.alive_being_most_children == other.alive_being_most_children
            && self.alive_being_most_children_number == other.alive_being_most_children_number
            && self.alive_being_most_kills == other.alive_being_most_kills
            && self.alive_being_most_kills_number == other.alive_being_most_kills_number
            && self.alive_being_most_infections == other.alive_being_most_infections
            && self.alive_being_most_infections_number == other.alive_being_most_infections_number
            && self.being_most_children == other.being_most_children
            && self.being_most_children_number == other.being_most_children_number
            && self.being_most_children_time == other.being_most_children_time
            && self.being_most_kills == other.being_most_kills
            && self.being_most_kills_number == other.being_most_kills_number
            && self.being_most_kills_time == other.being_most_kills_time
            && self.being_most_infections == other.being_most_infections
            && self.being_most_infections_number == other.being_most_infections_number
            && self.being_most_infections_time == other.being_most_infections_time
            && self.last_born_being == other.last_born_being
            && self.last_dead_being == other.last_dead_being
            && self.last_infected_being == other.last_infected_being
    }
    pub(crate) fn history_refs(&self) -> [Option<&HistoryListRef>; 6] {
        [self.population_list.as_ref(), self.death_list.as_ref(), self.birth_list.as_ref(), self.oxygen_list.as_ref(), self.carbon_dioxide_list.as_ref(), self.methane_list.as_ref()]
    }
    pub(crate) fn get_population_list(&self) -> Option<&HistoryListRef> {
        self.population_list.as_ref()
    }
    pub(crate) fn get_death_list(&self) -> Option<&HistoryListRef> {
        self.death_list.as_ref()
    }
    pub(crate) fn get_birth_list(&self) -> Option<&HistoryListRef> {
        self.birth_list.as_ref()
    }
    pub(crate) fn get_oxygen_list(&self) -> Option<&HistoryListRef> {
        self.oxygen_list.as_ref()
    }
    pub(crate) fn get_carbon_dioxide_list(&self) -> Option<&HistoryListRef> {
        self.carbon_dioxide_list.as_ref()
    }
    pub(crate) fn get_methane_list(&self) -> Option<&HistoryListRef> {
        self.methane_list.as_ref()
    }

    pub(crate) fn event_population_increase(&mut self, new_population: i32) {
        if new_population > self.max_population {
            self.max_population = new_population;
            self.max_population_time = self.time;
        }
    }

    pub(crate) fn event_population_decrease(&mut self, new_population: i32) {
        if new_population < self.min_population {
            self.min_population = new_population;
            self.min_population_time = self.time;
        }
    }

    pub(crate) fn event_organism_created(&mut self) {
        self.created_organisms = self.created_organisms.wrapping_add(1);
    }

    pub(crate) fn event_organism_born(&mut self, new_organism: &Organism, parent: &Organism) {
        self.last_born_being = new_organism.genome.clone();
        if parent.state.total_children > self.being_most_children_number {
            self.being_most_children = parent.genome.clone();
            self.being_most_children_number = parent.state.total_children;
            self.being_most_children_time = self.time;
        }
        self.birth_sum = self.birth_sum.wrapping_add(1);
        self.birth_last_time = self.birth_last_time.wrapping_add(1);
    }

    pub(crate) fn event_organism_die(
        &mut self,
        dying_organism: &Organism,
        killing_organism: Option<&Organism>,
    ) {
        self.last_dead_being = dying_organism.genome.clone();
        if let Some(killer) = killing_organism {
            if killer.state.total_kills > self.being_most_kills_number {
                self.being_most_kills = killer.genome.clone();
                self.being_most_kills_number = killer.state.total_kills;
                self.being_most_kills_time = self.time;
            }
        }
        self.death_sum = self.death_sum.wrapping_add(1);
        self.death_last_time = self.death_last_time.wrapping_add(1);
    }

    pub(crate) fn event_organism_infects(
        &mut self,
        infected_organism: &Organism,
        infecting_organism: Option<&Organism>,
    ) {
        self.last_infected_being = infected_organism.genome.clone();
        if let Some(infector) = infecting_organism {
            if infector.state.total_infected > self.being_most_infections_number {
                self.being_most_infections = infector.genome.clone();
                self.being_most_infections_number = infector.state.total_infected;
                self.being_most_infections_time = self.time;
            }
        }
        self.infections_sum = self.infections_sum.wrapping_add(1);
    }

    pub(crate) fn event_time(
        &mut self,
        population: i32,
        oxygen: f64,
        carbon_dioxide: f64,
        methane: f64,
    ) -> Result<(), HistoryError> {
        self.time = self.time.wrapping_add(1);
        // The new time denominator applies before interval counters are reset
        if f64::from(self.death_last_time) > 1.5 * self.get_average_deaths() {
            if f64::from(self.death_last_time) > 3.0 * self.get_average_deaths() {
                if self.mass_extinction_state != EXTINCTION_CONFIRMED
                    && self.mass_extinction_state != EXTINCTION_FINISHING
                {
                    self.mass_extinctions = self.mass_extinctions.wrapping_add(1);
                }
                self.mass_extinction_state = EXTINCTION_CONFIRMED;
            } else {
                match self.mass_extinction_state {
                    EXTINCTION_NO => self.mass_extinction_state = EXTINCTION_POSSIBLE,
                    EXTINCTION_POSSIBLE => {
                        self.mass_extinction_state = EXTINCTION_CONFIRMED;
                        self.mass_extinctions = self.mass_extinctions.wrapping_add(1);
                    }
                    EXTINCTION_FINISHING => self.mass_extinction_state = EXTINCTION_CONFIRMED,
                    _ => {}
                }
            }
        } else {
            match self.mass_extinction_state {
                EXTINCTION_POSSIBLE | EXTINCTION_FINISHING => {
                    self.mass_extinction_state = EXTINCTION_NO
                }
                EXTINCTION_CONFIRMED => self.mass_extinction_state = EXTINCTION_FINISHING,
                _ => {}
            }
        }
        self.population_sum = self.population_sum.wrapping_add(i64::from(population));
        if oxygen > self.max_oxygen {
            self.max_oxygen = oxygen;
            self.max_oxygen_time = self.time;
        }
        if oxygen < self.min_oxygen {
            self.min_oxygen = oxygen;
            self.min_oxygen_time = self.time;
        }
        if carbon_dioxide > self.max_carbon_dioxide {
            self.max_carbon_dioxide = carbon_dioxide;
            self.max_carbon_dioxide_time = self.time;
        }
        if carbon_dioxide < self.min_carbon_dioxide {
            self.min_carbon_dioxide = carbon_dioxide;
            self.min_carbon_dioxide_time = self.time;
        }
        if methane > self.max_methane {
            self.max_methane = methane;
            self.max_methane_time = self.time;
        }
        if methane < self.min_methane {
            self.min_methane = methane;
            self.min_methane_time = self.time;
        }
        if self.birth_last_time > self.max_births {
            self.max_births = self.birth_last_time;
        }
        if self.death_last_time > self.max_deaths {
            self.max_deaths = self.death_last_time;
        }
        self.population_list.as_ref().ok_or(HistoryError::MissingList { field: "populationList" })?.append("populationList", f64::from(population))?;
        self.death_list.as_ref().ok_or(HistoryError::MissingList { field: "deathList" })?.append("deathList", f64::from(self.death_last_time))?;
        self.birth_list.as_ref().ok_or(HistoryError::MissingList { field: "birthList" })?.append("birthList", f64::from(self.birth_last_time))?;
        self.oxygen_list.as_ref().ok_or(HistoryError::MissingList { field: "oxygenList" })?.append("oxygenList", oxygen)?;
        self.carbon_dioxide_list.as_ref().ok_or(HistoryError::MissingList { field: "carbonDioxideList" })?.append("carbonDioxideList", carbon_dioxide)?;
        self.methane_list.as_ref().ok_or(HistoryError::MissingList { field: "methaneList" })?.append("methaneList", methane)?;
        self.death_last_time = 0;
        self.birth_last_time = 0;
        Ok(())
    }

    fn reset_alive_records(&mut self) {
        self.alive_being_most_children = None;
        self.alive_being_most_children_number = 0;
        self.alive_being_most_kills = None;
        self.alive_being_most_kills_number = 0;
        self.alive_being_most_infections = None;
        self.alive_being_most_infections_number = 0;
    }
    fn consider_alive_actor(&mut self, organism: &crate::organism::Organism) {
            if organism.state.alive {
                if organism.state.total_children > self.alive_being_most_children_number {
                    self.alive_being_most_children_number = organism.state.total_children;
                    self.alive_being_most_children = organism.genome.clone();
                }
                if organism.state.total_kills > self.alive_being_most_kills_number {
                    self.alive_being_most_kills_number = organism.state.total_kills;
                    self.alive_being_most_kills = organism.genome.clone();
                }
                if organism.state.total_infected > self.alive_being_most_infections_number {
                    self.alive_being_most_infections_number = organism.state.total_infected;
                    self.alive_being_most_infections = organism.genome.clone();
                }
            }
    }
    pub(crate) fn find_best_alive_beings(&mut self, organisms: &[OrganismRef]) {
        self.reset_alive_records();
        for organism in organisms { self.consider_alive_actor(&organism.borrow()); }
    }
    pub(crate) fn find_best_alive_members(&mut self, organisms: &crate::membership::Membership) -> Result<(), crate::ListAccessError> {
        self.reset_alive_records();
        for index in 0..organisms.len()? {
            self.consider_alive_actor(&organisms.actor(index)?.borrow());
        }
        Ok(())
    }

    /// Reuse the world's genotype map so records preserve aliases without sharing mutable cross-world state
    pub(crate) fn copy_for_world(&self, genomes: &mut GenomeCloner) -> Self {
        self.copy_with_histories(genomes, &mut HashMap::new())
    }

    pub(crate) fn copy_with_histories(
        &self,
        genomes: &mut GenomeCloner,
        histories: &mut HashMap<usize, HistoryListRef>,
    ) -> Self {
        let lists = self.history_refs().map(|value| value.map(|value| value.copied(histories)));
        self.copy_with_lists(genomes, lists)
    }
    pub(crate) fn copy_with_lists(&self, genomes: &mut GenomeCloner, histories: [Option<HistoryListRef>; 6]) -> Self {
        let [population_list, death_list, birth_list, oxygen_list, carbon_dioxide_list, methane_list] = histories;
        Self {
            time: self.time,
            max_population: self.max_population,
            max_births: self.max_births,
            max_deaths: self.max_deaths,
            max_population_time: self.max_population_time,
            min_population: self.min_population,
            min_population_time: self.min_population_time,
            mass_extinctions: self.mass_extinctions,
            mass_extinction_state: self.mass_extinction_state,
            created_organisms: self.created_organisms,
            population_sum: self.population_sum,
            death_sum: self.death_sum,
            death_last_time: self.death_last_time,
            birth_sum: self.birth_sum,
            birth_last_time: self.birth_last_time,
            infections_sum: self.infections_sum,
            max_oxygen: self.max_oxygen,
            max_oxygen_time: self.max_oxygen_time,
            min_oxygen: self.min_oxygen,
            min_oxygen_time: self.min_oxygen_time,
            max_carbon_dioxide: self.max_carbon_dioxide,
            max_methane: self.max_methane,
            max_carbon_dioxide_time: self.max_carbon_dioxide_time,
            max_methane_time: self.max_methane_time,
            min_carbon_dioxide: self.min_carbon_dioxide,
            min_methane: self.min_methane,
            min_carbon_dioxide_time: self.min_carbon_dioxide_time,
            min_methane_time: self.min_methane_time,
            alive_being_most_children: self
                .alive_being_most_children
                .as_ref()
                .map(|code| genomes.copy(code)),
            alive_being_most_children_number: self.alive_being_most_children_number,
            alive_being_most_kills: self
                .alive_being_most_kills
                .as_ref()
                .map(|code| genomes.copy(code)),
            alive_being_most_kills_number: self.alive_being_most_kills_number,
            alive_being_most_infections: self
                .alive_being_most_infections
                .as_ref()
                .map(|code| genomes.copy(code)),
            alive_being_most_infections_number: self.alive_being_most_infections_number,
            being_most_children: self
                .being_most_children
                .as_ref()
                .map(|code| genomes.copy(code)),
            being_most_children_number: self.being_most_children_number,
            being_most_children_time: self.being_most_children_time,
            being_most_kills: self
                .being_most_kills
                .as_ref()
                .map(|code| genomes.copy(code)),
            being_most_kills_number: self.being_most_kills_number,
            being_most_kills_time: self.being_most_kills_time,
            being_most_infections: self
                .being_most_infections
                .as_ref()
                .map(|code| genomes.copy(code)),
            being_most_infections_number: self.being_most_infections_number,
            being_most_infections_time: self.being_most_infections_time,
            last_born_being: self.last_born_being.as_ref().map(|code| genomes.copy(code)),
            last_dead_being: self.last_dead_being.as_ref().map(|code| genomes.copy(code)),
            last_infected_being: self
                .last_infected_being
                .as_ref()
                .map(|code| genomes.copy(code)),
            population_list,
            death_list,
            birth_list,
            oxygen_list,
            carbon_dioxide_list,
            methane_list,
        }
    }
}

impl World {
    /// Read the three retained historical records without refreshing the statistics view
    pub fn all_time_records(&self) -> [RecordView; 3] {
        let statistics = self.statistics_ref();
        let statistics = statistics.borrow();
        [
            (
                RecordKind::HistoricalChildren,
                &statistics.being_most_children,
                statistics.being_most_children_number,
                statistics.being_most_children_time,
            ),
            (
                RecordKind::HistoricalKills,
                &statistics.being_most_kills,
                statistics.being_most_kills_number,
                statistics.being_most_kills_time,
            ),
            (
                RecordKind::HistoricalInfections,
                &statistics.being_most_infections,
                statistics.being_most_infections_number,
                statistics.being_most_infections_time,
            ),
        ]
        .map(|(kind, genome, count, time)| RecordView {
            kind,
            genome: genome
                .as_ref()
                .map(|genome| GenomeHandle::from_shared(Rc::clone(genome))),
            count,
            time: Some(time),
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        ColorKind, Gene, Genome, GenomeModifiers, GenomeProperties, OrganismState, REACTION_NAMES,
    };
    use std::{
        cell::RefCell,
        collections::{BTreeMap, BTreeSet},
    };

    pub(crate) fn samples(history: Option<&HistoryListRef>) -> Vec<f64> {
        history.expect("Ordinary fixture history is non-null").numeric_prefix("fixture", usize::MAX).unwrap()
    }

    fn actor(children: i32, kills: i32, infections: i32) -> OrganismRef {
        let gene = Gene::new(12.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared();
        let genome = Genome::from_parts(
            &[gene],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        let organism = Organism::unplaced(
            Rc::new(RefCell::new(genome)),
            OrganismState {
                total_children: children,
                total_kills: kills,
                total_infected: infections,
                ..OrganismState::default()
            },
        )
        .unwrap();
        Rc::new(RefCell::new(organism))
    }

    #[test]
    fn all_time_records_preserve_retained_metadata_and_world_state() {
        let world = World::new(crate::WorldConfig {
            initial_attempts: 0,
            ..crate::WorldConfig::default()
        })
        .unwrap();
        let empty = world.all_time_records();
        assert!(empty.iter().all(|record| {
            record.genome.is_none() && record.count == 0 && record.time == Some(0)
        }));

        let winner = actor(3, 4, 5);
        let child = actor(0, 0, 0);
        {
            let statistics = world.statistics_ref();
            let mut statistics = statistics.borrow_mut();
            statistics.event_organism_born(&child.borrow(), &winner.borrow());
            statistics.event_organism_infects(&child.borrow(), Some(&winner.borrow()));
            statistics.being_most_children_time = i64::MAX;
            statistics.being_most_kills_number = -4;
            statistics.being_most_kills_time = i64::MIN;
        }
        let limits = crate::object_stream::Limits::default();
        let before = world
            .encode_legacy_file_bytes(limits, limits.allocated_bytes)
            .unwrap();
        let rng = world.random_snapshot();
        let records = world.all_time_records();
        assert_eq!(
            records
                .each_ref()
                .map(|record| (record.kind, record.count, record.time)),
            [
                (RecordKind::HistoricalChildren, 3, Some(i64::MAX)),
                (RecordKind::HistoricalKills, -4, Some(i64::MIN)),
                (RecordKind::HistoricalInfections, 5, Some(0)),
            ]
        );
        assert_eq!(world.population(), 0);
        assert!(records[1].genome.is_none());
        assert_eq!(records[0].genome, records[2].genome);
        let preview = records[0]
            .genome
            .as_ref()
            .unwrap()
            .preview_image(100, 100, 64 * 1024 * 1024)
            .unwrap();
        assert_eq!(
            (preview.width, preview.height, preview.rgb.len()),
            (100, 100, 30_000)
        );
        assert_eq!(world.random_snapshot(), rng);
        assert_eq!(
            world
                .encode_legacy_file_bytes(limits, limits.allocated_bytes)
                .unwrap(),
            before
        );
        let statistics = world.refresh_statistics(&world.open_statistics()).unwrap();
        assert_eq!(
            records,
            records.each_ref().map(|record| statistics.record(record.kind))
        );
        world.statistics_ref().borrow_mut().population_list = None;
        assert_eq!(world.all_time_records(), records);
    }

    #[test]
    fn synchronous_events_keep_first_ties_and_live_genotype_references() {
        let first = actor(3, 4, 5);
        let tied = actor(3, 4, 5);
        let child = actor(0, 0, 0);
        let mut statistics = WorldStatistics::new(12_500, 0.0, 500_000.0, 0.0);
        statistics.event_organism_born(&child.borrow(), &first.borrow());
        statistics.event_organism_die(&child.borrow(), Some(&first.borrow()));
        statistics.event_organism_infects(&child.borrow(), Some(&first.borrow()));
        assert_eq!(
            (
                statistics.birth_sum,
                statistics.death_sum,
                statistics.infections_sum
            ),
            (1, 1, 1)
        );
        assert_eq!(
            (statistics.birth_last_time, statistics.death_last_time),
            (1, 1)
        );
        assert_eq!(statistics.get_average_births(), 0.0);
        assert!(Rc::ptr_eq(
            statistics.get_last_born_being().unwrap(), child.borrow().genome.as_ref().unwrap()));
        statistics.event_time(2, 0.0, 500_000.0, 0.0).unwrap();
        statistics.event_organism_born(&child.borrow(), &tied.borrow());
        statistics.event_organism_die(&child.borrow(), Some(&tied.borrow()));
        statistics.event_organism_infects(&child.borrow(), Some(&tied.borrow()));
        assert_eq!(
            (
                statistics.get_being_most_children_time(),
                statistics.get_being_most_kills_time(),
                statistics.get_being_most_infections_time()
            ),
            (0, 0, 0)
        );
        for recorded in [
            statistics.get_being_most_children(),
            statistics.get_being_most_kills(),
            statistics.get_being_most_infections(),
        ] {
            assert!(Rc::ptr_eq(recorded.unwrap(), first.borrow().genome.as_ref().unwrap()));
        }
        first.borrow().genome.as_ref().unwrap().borrow_mut().set_generation(92);
        assert_eq!(
            statistics
                .get_being_most_children()
                .unwrap()
                .borrow()
                .generation(),
            92
        );
        statistics.event_organism_die(&tied.borrow(), None);
        statistics.event_organism_infects(&tied.borrow(), None);
        assert_eq!((statistics.death_sum, statistics.infections_sum), (3, 3));
        assert!(Rc::ptr_eq(
            statistics.get_last_dead_being().unwrap(), tied.borrow().genome.as_ref().unwrap()));
    }

    #[test]
    fn living_spores_are_eligible_and_scan_order_breaks_positive_ties() {
        let first = actor(3, 4, 5);
        let spore = actor(3, 4, 5);
        spore.borrow_mut().state.active = false;
        let corpse = actor(100, 100, 100);
        corpse.borrow_mut().state.alive = false;
        let mut statistics = WorldStatistics::new(10, 0.0, 0.0, 0.0);
        for ordered in [
            vec![Rc::clone(&first), Rc::clone(&spore), Rc::clone(&corpse)],
            vec![Rc::clone(&spore), Rc::clone(&first), Rc::clone(&corpse)],
        ] {
            statistics.find_best_alive_beings(&ordered);
            for recorded in [
                statistics.get_alive_being_most_children(),
                statistics.get_alive_being_most_kills(),
                statistics.get_alive_being_most_infections(),
            ] {
                assert!(Rc::ptr_eq(recorded.unwrap(), ordered[0].borrow().genome.as_ref().unwrap()));
            }
        }
        statistics.find_best_alive_beings(&[corpse, actor(0, 0, 0), actor(-1, -1, -1)]);
        assert!(statistics.get_alive_being_most_children().is_none());
        assert!(statistics.get_alive_being_most_kills().is_none());
        assert!(statistics.get_alive_being_most_infections().is_none());
        assert_eq!(
            (
                statistics.get_alive_being_most_children_number(),
                statistics.get_alive_being_most_kills_number(),
                statistics.get_alive_being_most_infections_number()
            ),
            (0, 0, 0)
        );
    }

    #[test]
    fn extinction_thresholds_use_incremented_time_and_strict_comparisons() {
        let expected_states = [[0, 1, 1, 2], [0, 2, 2, 2], [3, 2, 2, 2], [0, 2, 2, 2]];
        let expected_increments = [[0, 0, 0, 1], [0, 1, 1, 1], [0, 0, 0, 0], [0, 0, 0, 0]];
        for initial_state in 0..4 {
            for (level, deaths) in [3, 4, 6, 7].into_iter().enumerate() {
                let mut statistics = WorldStatistics::new(0, 0.0, 0.0, 0.0);
                statistics.time = 3;
                statistics.death_sum = 8;
                statistics.death_last_time = deaths;
                statistics.mass_extinction_state = initial_state as i32;
                statistics.event_time(0, 0.0, 0.0, 0.0).unwrap();
                assert_eq!(
                    statistics.mass_extinction_state,
                    expected_states[initial_state][level]
                );
                assert_eq!(
                    statistics.mass_extinctions,
                    expected_increments[initial_state][level]
                );
                assert_eq!(statistics.get_average_deaths(), 2.0);
                assert_eq!(samples(statistics.get_death_list()), &[f64::from(deaths)]);
                assert_eq!(statistics.death_last_time, 0);
            }
        }
    }

    #[test]
    fn history_retains_last_hundred_samples_and_ieee_payloads() {
        let nan = f64::from_bits(0xfff8_0000_0000_0042);
        let mut statistics = WorldStatistics::new(0, -0.0, 0.0, f64::INFINITY);
        statistics.event_time(0, 0.0, nan, f64::NEG_INFINITY).unwrap();
        assert_eq!(statistics.get_max_oxygen().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(statistics.get_min_oxygen().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(
            samples(statistics.get_carbon_dioxide_list())[0].to_bits(),
            nan.to_bits()
        );
        assert_eq!(statistics.get_max_carbon_dioxide(), 0.0);
        assert_eq!(statistics.get_min_carbon_dioxide(), 0.0);
        assert_eq!(statistics.get_max_methane(), f64::INFINITY);
        assert_eq!(statistics.get_min_methane(), f64::NEG_INFINITY);
        for sample in 1..102 {
            statistics.event_time(
                sample,
                f64::from(sample),
                -f64::from(sample),
                f64::from(sample) / 2.0,
            ).unwrap();
        }
        assert_eq!(samples(statistics.get_population_list()).len(), 100);
        assert_eq!(samples(statistics.get_population_list()).first(), Some(&2.0));
        assert_eq!(samples(statistics.get_population_list()).last(), Some(&101.0));
        assert_eq!(
            samples(statistics.get_oxygen_list()),
            samples(statistics.get_population_list())
        );
        assert_eq!(samples(statistics.get_carbon_dioxide_list())[0], -2.0);
        assert_eq!(samples(statistics.get_methane_list())[0], 1.0);
        assert_eq!(samples(statistics.get_birth_list()), &[0.0; 100]);
        assert_eq!(samples(statistics.get_death_list()), &[0.0; 100]);
    }

    #[test]
    fn world_copy_reuses_external_genotype_map_and_isolates_mutation() {
        let parent = actor(1, 2, 3);
        let child = actor(0, 0, 0);
        let killer = actor(2, 4, 6);
        let mut statistics = WorldStatistics::new(20, 1.0, 2.0, 3.0);
        statistics.event_organism_created();
        statistics.event_population_increase(4);
        statistics.event_population_decrease(3);
        statistics.event_organism_born(&child.borrow(), &parent.borrow());
        statistics.event_organism_die(&child.borrow(), Some(&killer.borrow()));
        statistics.event_organism_infects(&parent.borrow(), Some(&killer.borrow()));
        statistics.find_best_alive_beings(&[
            Rc::clone(&parent),
            Rc::clone(&child),
            Rc::clone(&killer),
        ]);
        statistics.event_time(3, 4.0, 1.0, 8.0).unwrap();
        let mut genomes = GenomeCloner::default();
        let independently_copied_parent = genomes.copy(parent.borrow().genome.as_ref().unwrap());
        let independently_copied_child = genomes.copy(child.borrow().genome.as_ref().unwrap());
        let independently_copied_killer = genomes.copy(killer.borrow().genome.as_ref().unwrap());
        let copied = statistics.copy_for_world(&mut genomes);
        assert_eq!(statistics, copied);
        assert!(!Rc::ptr_eq(
            &independently_copied_parent,
            &independently_copied_child
        ));
        assert!(!Rc::ptr_eq(
            &independently_copied_parent,
            &independently_copied_killer
        ));
        for (recorded, expected) in [
            (
                copied.get_alive_being_most_children(),
                &independently_copied_killer,
            ),
            (
                copied.get_alive_being_most_kills(),
                &independently_copied_killer,
            ),
            (
                copied.get_alive_being_most_infections(),
                &independently_copied_killer,
            ),
            (
                copied.get_being_most_children(),
                &independently_copied_parent,
            ),
            (copied.get_being_most_kills(), &independently_copied_killer),
            (
                copied.get_being_most_infections(),
                &independently_copied_killer,
            ),
            (copied.get_last_born_being(), &independently_copied_child),
            (copied.get_last_dead_being(), &independently_copied_child),
            (
                copied.get_last_infected_being(),
                &independently_copied_parent,
            ),
        ] {
            assert!(Rc::ptr_eq(recorded.unwrap(), expected));
            assert!(!Rc::ptr_eq(recorded.unwrap(), parent.borrow().genome.as_ref().unwrap()));
            assert!(!Rc::ptr_eq(recorded.unwrap(), child.borrow().genome.as_ref().unwrap()));
            assert!(!Rc::ptr_eq(recorded.unwrap(), killer.borrow().genome.as_ref().unwrap()));
        }
        parent.borrow().genome.as_ref().unwrap().borrow_mut().set_generation(12);
        assert_eq!(
            statistics
                .get_being_most_children()
                .unwrap()
                .borrow()
                .generation(),
            12
        );
        assert_eq!(
            copied
                .get_being_most_children()
                .unwrap()
                .borrow()
                .generation(),
            0
        );
        statistics.event_time(5, 9.0, 4.0, 3.0).unwrap();
        assert_eq!(samples(copied.get_population_list()), &[3.0]);
        assert_eq!(samples(statistics.get_population_list()), &[3.0, 5.0]);
    }

    #[test]
    fn java_counter_overflow_wraps_and_nonpositive_time_returns_zero_averages() {
        let parent = actor(i32::MAX, i32::MAX, i32::MAX);
        let mut statistics = WorldStatistics::new(0, 0.0, 0.0, 0.0);
        statistics.created_organisms = i32::MAX;
        statistics.birth_sum = i64::MAX;
        statistics.birth_last_time = i32::MAX;
        statistics.death_sum = i64::MAX;
        statistics.death_last_time = i32::MAX;
        statistics.infections_sum = i64::MAX;
        statistics.event_organism_created();
        statistics.event_organism_born(&parent.borrow(), &parent.borrow());
        statistics.event_organism_die(&parent.borrow(), Some(&parent.borrow()));
        statistics.event_organism_infects(&parent.borrow(), Some(&parent.borrow()));
        assert_eq!(statistics.created_organisms, i32::MIN);
        assert_eq!(
            (
                statistics.birth_sum,
                statistics.death_sum,
                statistics.infections_sum
            ),
            (i64::MIN, i64::MIN, i64::MIN)
        );
        assert_eq!(
            (statistics.birth_last_time, statistics.death_last_time),
            (i32::MIN, i32::MIN)
        );
        statistics.time = i64::MAX;
        statistics.population_sum = i64::MAX;
        statistics.event_time(1, 0.0, 0.0, 0.0).unwrap();
        assert_eq!(
            (statistics.time, statistics.population_sum),
            (i64::MIN, i64::MIN)
        );
        for average in [
            statistics.get_average_population(),
            statistics.get_average_births(),
            statistics.get_average_deaths(),
            statistics.get_average_infections(),
        ] {
            assert_eq!(average.to_bits(), 0.0_f64.to_bits());
        }
        assert_eq!((statistics.max_births, statistics.max_deaths), (0, 0));
        assert_eq!(
            (
                samples(statistics.get_birth_list())[0],
                samples(statistics.get_death_list())[0]
            ),
            (f64::from(i32::MIN), f64::from(i32::MIN))
        );

        let mut extinction = WorldStatistics::new(0, 0.0, 0.0, 0.0);
        extinction.mass_extinctions = i32::MAX;
        extinction.death_last_time = 1;
        extinction.event_time(0, 0.0, 0.0, 0.0).unwrap();
        assert_eq!(extinction.mass_extinctions, i32::MIN);
        assert_eq!(extinction.mass_extinction_state, EXTINCTION_CONFIRMED);
    }

    pub(crate) enum Observation<'a> {
        Int(i32),
        Long(i64),
        Double(f64),
        Genome(Option<&'a GenomeRef>),
        History(Option<&'a HistoryListRef>),
    }

    fn double(value: &str) -> f64 {
        f64::from_bits(u64::from_str_radix(value, 16).unwrap())
    }

    fn history(value: &str) -> Vec<f64> {
        if value.is_empty() {
            Vec::new()
        } else {
            value.split(',').map(double).collect()
        }
    }

    fn reference(value: &str, genomes: &BTreeMap<String, GenomeRef>) -> Option<GenomeRef> {
        if value == "~" {
            None
        } else {
            Some(Rc::clone(&genomes[value]))
        }
    }

    pub(crate) fn assert_observation(
        actual: Observation<'_>,
        kind: &str,
        expected: &str,
        genomes: &BTreeMap<String, GenomeRef>,
        context: &str,
    ) {
        match actual {
            Observation::Int(value) => {
                assert_eq!(kind, "int", "{context}");
                assert_eq!(value, expected.parse::<i32>().unwrap(), "{context}");
            }
            Observation::Long(value) => {
                assert_eq!(kind, "long", "{context}");
                assert_eq!(value, expected.parse::<i64>().unwrap(), "{context}");
            }
            Observation::Double(value) => {
                assert_eq!(kind, "double", "{context}");
                assert_eq!(value.to_bits(), double(expected).to_bits(), "{context}");
            }
            Observation::Genome(value) => {
                assert_eq!(kind, "GeneticCode", "{context}");
                if expected == "~" {
                    assert!(value.is_none(), "{context}");
                } else {
                    assert!(
                        Rc::ptr_eq(value.expect(context), &genomes[expected]),
                        "{context}"
                    );
                }
            }
            Observation::History(value) => {
                assert_eq!(kind, "List", "{context}");
                let actual: Vec<_> = value.expect(context).numeric_prefix("fixture", usize::MAX).expect(context).iter().map(|value| value.to_bits()).collect();
                let expected: Vec<_> = history(expected)
                    .iter()
                    .map(|value| value.to_bits())
                    .collect();
                assert_eq!(actual, expected, "{context}");
            }
        }
    }

    fn input_genome(rows: &[Vec<&str>], label: &str) -> Genome {
        let fields: BTreeMap<_, _> = rows
            .iter()
            .filter(|row| row[0] == "G" && row[1] == "input" && row[2] == label)
            .map(|row| {
                assert_eq!(row.len(), 5);
                (row[3], row[4])
            })
            .collect();
        assert_eq!(fields.len(), 28, "{label}");
        assert_eq!(fields["_reference"], label);
        let number = |name| fields[name].parse::<i32>().unwrap();
        let flag = |name| fields[name].parse::<bool>().unwrap();
        let indices: BTreeSet<_> = rows
            .iter()
            .filter(|row| row[0] == "N" && row[1] == "input" && row[2] == label)
            .map(|row| row[3].parse::<usize>().unwrap())
            .collect();
        let mut genes = Vec::new();
        for index in indices {
            assert_eq!(index, genes.len(), "{label}");
            let fields: BTreeMap<_, _> = rows
                .iter()
                .filter(|row| {
                    row[0] == "N"
                        && row[1] == "input"
                        && row[2] == label
                        && row[3].parse::<usize>().unwrap() == index
                })
                .map(|row| {
                    assert_eq!(row.len(), 6);
                    (row[4], row[5])
                })
                .collect();
            assert_eq!(fields.len(), 40, "{label} gene {index}");
            let reactions = std::array::from_fn(|reaction| {
                fields[format!("_{}reaction", REACTION_NAMES[reaction]).as_str()]
                    .parse::<i32>()
                    .unwrap()
            });
            let color = fields["_color"];
            genes.push(
                Gene::new(
                    double(fields["_length"]),
                    double(fields["_theta"]),
                    if color == "~" {
                        None
                    } else {
                        Some(u32::from_str_radix(color, 16).unwrap())
                    },
                    fields["_branch"].parse().unwrap(),
                    reactions,
                )
                .shared(),
            );
        }
        assert_eq!(genes.len(), number("_nGenes") as usize);
        let properties = GenomeProperties {
            symmetry: number("_symmetry"),
            mirror: number("_mirror"),
            mutation_rate: number("_mutationrate"),
            activity: number("_activity"),
            modifiers: GenomeModifiers {
                cream: number("_modifiescream"),
                fallow: number("_modifiesfallow"),
                spore: number("_modifiesspore"),
                adapt_spore: number("_adaptspore"),
                plague: flag("_plague"),
                disperse_children: flag("_disperseChildren"),
                generation_battle: flag("_generationBattle"),
                sibling_battle: flag("_siblingBattle"),
                altruist: flag("_altruist"),
                familial: flag("_familial"),
                social: flag("_social"),
                peaceful: flag("_peaceful"),
                passive: flag("_passive"),
                clockwise: flag("_clockwise"),
                mimic_all: flag("_mimicall"),
                modifies_pink: flag("_modifiespink"),
                modifies_lilac: flag("_modifieslilac"),
                modifies_sky: flag("_modifiessky"),
                selfish: flag("_selfish"),
            },
        };
        let mut genome = Genome::from_parts(&genes, properties);
        genome.set_generation(number("_generation"));
        genome.set_clade_id(if fields["_cladeID"] == "~" {
            None
        } else {
            Some(fields["_cladeID"].to_owned())
        });
        genome.set_update_clade(flag("_updateClade"));
        genome
    }

    fn genome_value(genome: &Genome, field: &str) -> String {
        let properties = genome.properties();
        let modifiers = properties.modifiers;
        match field {
            "_nGenes" => genome.genes().unwrap().len().to_string(),
            "_symmetry" => properties.symmetry.to_string(),
            "_mirror" => properties.mirror.to_string(),
            "_mutationrate" => properties.mutation_rate.to_string(),
            "_activity" => properties.activity.to_string(),
            "_modifiescream" => modifiers.cream.to_string(),
            "_modifiesfallow" => modifiers.fallow.to_string(),
            "_modifiesspore" => modifiers.spore.to_string(),
            "_adaptspore" => modifiers.adapt_spore.to_string(),
            "_plague" => modifiers.plague.to_string(),
            "_disperseChildren" => modifiers.disperse_children.to_string(),
            "_generationBattle" => modifiers.generation_battle.to_string(),
            "_siblingBattle" => modifiers.sibling_battle.to_string(),
            "_altruist" => modifiers.altruist.to_string(),
            "_familial" => modifiers.familial.to_string(),
            "_social" => modifiers.social.to_string(),
            "_peaceful" => modifiers.peaceful.to_string(),
            "_passive" => modifiers.passive.to_string(),
            "_clockwise" => modifiers.clockwise.to_string(),
            "_mimicall" => modifiers.mimic_all.to_string(),
            "_modifiespink" => modifiers.modifies_pink.to_string(),
            "_modifieslilac" => modifiers.modifies_lilac.to_string(),
            "_modifiessky" => modifiers.modifies_sky.to_string(),
            "_selfish" => modifiers.selfish.to_string(),
            "_generation" => genome.generation().to_string(),
            "_cladeID" => genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref().unwrap_or("~").to_owned(),
            "_updateClade" => genome.update_clade().to_string(),
            _ => panic!("Unmapped Java genome field {field}"),
        }
    }

    fn gene_value(genome: &Genome, index: usize, field: &str) -> String {
        let gene_handle = genome.gene_ref(index).unwrap();
        let gene = gene_handle.borrow();
        match field {
            "_length" => format!("{:016x}", gene.length().to_bits()),
            "_theta" => format!("{:016x}", gene.theta().to_bits()),
            "_branch" => gene.branch().to_string(),
            "_color" => gene
                .color()
                .map_or_else(|| "~".to_owned(), |color| format!("{color:08x}")),
            _ => {
                let reaction = field
                    .strip_prefix('_')
                    .unwrap()
                    .strip_suffix("reaction")
                    .unwrap();
                let index = REACTION_NAMES
                    .iter()
                    .position(|name| *name == reaction)
                    .expect(field);
                gene.reactions()[index].to_string()
            }
        }
    }

    fn apply_input(
        row: &[&str],
        statistics: &mut WorldStatistics,
        actors: &BTreeMap<String, OrganismRef>,
        genomes: &BTreeMap<String, GenomeRef>,
    ) {
        assert_eq!(row.len(), 5);
        let target: Vec<_> = row[2].split(':').collect();
        let (kind, value) = (row[3], row[4]);
        match target[0] {
            "statistics" => {
                assert_eq!(target.len(), 2);
                set_statistics_input(statistics, target[1], kind, value, genomes);
            }
            "actor" => {
                assert_eq!(target.len(), 3);
                let mut actor = actors[target[1]].borrow_mut();
                match target[2] {
                    "_nTotalChildren" => {
                        assert_eq!(kind, "int");
                        actor.state.total_children = value.parse().unwrap();
                    }
                    "_nTotalKills" => {
                        assert_eq!(kind, "int");
                        actor.state.total_kills = value.parse().unwrap();
                    }
                    "_nTotalInfected" => {
                        assert_eq!(kind, "int");
                        actor.state.total_infected = value.parse().unwrap();
                    }
                    "_age" => {
                        assert_eq!(kind, "int");
                        actor.state.age = value.parse().unwrap();
                    }
                    "alive" => {
                        assert_eq!(kind, "boolean");
                        actor.state.alive = value.parse().unwrap();
                    }
                    "active" => {
                        assert_eq!(kind, "boolean");
                        actor.state.active = value.parse().unwrap();
                    }
                    "_geneticCode" => {
                        assert_eq!(kind, "GeneticCode");
                        actor.genome = reference(value, genomes);
                    }
                    _ => panic!("Unmapped controlled Java actor field {}", row[2]),
                }
            }
            "genome" => {
                assert_eq!(target.len(), 3);
                if target[2] == "construction" {
                    assert_eq!(kind, "String");
                    return;
                }
                let mut genome = genomes[target[1]].borrow_mut();
                match target[2] {
                    "_generation" => {
                        assert_eq!(kind, "int");
                        genome.set_generation(value.parse().unwrap());
                    }
                    "_cladeID" => {
                        assert_eq!(kind, "String");
                        genome.set_clade_id(if value == "~" {
                            None
                        } else {
                            Some(value.to_owned())
                        });
                    }
                    "_updateClade" => {
                        assert_eq!(kind, "boolean");
                        genome.set_update_clade(value.parse().unwrap());
                    }
                    "gene0._length" => {
                        assert_eq!(kind, "double");
                        genome.genes().unwrap()[0].borrow_mut().set_length(double(value));
                    }
                    _ => panic!("Unmapped controlled Java genotype field {}", row[2]),
                }
            }
            _ => panic!("Unmapped controlled Java target {}", row[2]),
        }
    }

    fn call_event(
        row: &[&str],
        statistics: &mut WorldStatistics,
        actors: &BTreeMap<String, OrganismRef>,
    ) {
        match row[2] {
            "eventPopulationIncrease" => {
                assert_eq!(row.len(), 4);
                statistics.event_population_increase(row[3].parse().unwrap());
            }
            "eventPopulationDecrease" => {
                assert_eq!(row.len(), 4);
                statistics.event_population_decrease(row[3].parse().unwrap());
            }
            "eventOrganismCreated" => {
                assert_eq!(row.len(), 3);
                statistics.event_organism_created();
            }
            "eventOrganismBorn" => {
                assert_eq!(row.len(), 5);
                statistics.event_organism_born(&actors[row[3]].borrow(), &actors[row[4]].borrow());
            }
            "eventOrganismDie" => {
                assert_eq!(row.len(), 5);
                let victim = actors[row[3]].borrow();
                if row[4] == "~" {
                    statistics.event_organism_die(&victim, None);
                } else {
                    statistics.event_organism_die(&victim, Some(&actors[row[4]].borrow()));
                }
            }
            "eventOrganismInfects" => {
                assert_eq!(row.len(), 5);
                let receiver = actors[row[3]].borrow();
                if row[4] == "~" {
                    statistics.event_organism_infects(&receiver, None);
                } else {
                    statistics.event_organism_infects(&receiver, Some(&actors[row[4]].borrow()));
                }
            }
            "eventTime" => {
                assert_eq!(row.len(), 7);
                statistics.event_time(
                    row[3].parse().unwrap(),
                    double(row[4]),
                    double(row[5]),
                    double(row[6]),
                ).unwrap();
            }
            "findBestAliveBeings" => {
                assert_eq!(row.len(), 4);
                let ordered: Vec<_> = if row[3] == "~" {
                    Vec::new()
                } else {
                    row[3]
                        .split(',')
                        .map(|label| Rc::clone(&actors[label]))
                        .collect()
                };
                statistics.find_best_alive_beings(&ordered);
            }
            _ => panic!("Unmapped Java statistics call {}", row[2]),
        }
    }

    fn replay_statistics_case(rows: &[Vec<&str>]) -> (usize, usize, usize, usize) {
        assert_eq!(rows[0].len(), 4);
        assert_eq!(rows[0][0], "C");
        rows[0][3].parse::<i64>().unwrap();
        let case = rows[0][1];
        let settings: BTreeMap<_, _> = rows
            .iter()
            .filter(|row| row[0] == "U")
            .map(|row| {
                assert_eq!(row.len(), 4);
                (row[1], (row[2], row[3]))
            })
            .collect();
        assert_eq!(settings.len(), 4);
        let integer_setting = |name| {
            let (kind, value) = settings[name];
            assert_eq!(kind, "int");
            value.parse::<i32>().unwrap()
        };
        let double_setting = |name| {
            let (kind, value) = settings[name];
            assert_eq!(kind, "double");
            double(value)
        };
        let mut statistics = WorldStatistics::new(
            integer_setting("INITIAL_ORGANISMS"),
            double_setting("INITIAL_O2"),
            double_setting("INITIAL_CO2"),
            double_setting("INITIAL_CH4"),
        );
        let mut genomes: BTreeMap<String, GenomeRef> = BTreeMap::new();
        for row in rows
            .iter()
            .filter(|row| row[0] == "I" && row[2].ends_with(":construction"))
        {
            assert_eq!(row.len(), 5);
            assert_eq!(row[3], "String");
            let parts: Vec<_> = row[2].split(':').collect();
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], "genome");
            let genome = if row[4] == "explicit" {
                input_genome(rows, parts[1])
            } else {
                genomes[row[4].strip_prefix("clone:").unwrap()]
                    .borrow()
                    .clone()
            };
            assert!(
                genomes
                    .insert(parts[1].to_owned(), Rc::new(RefCell::new(genome)))
                    .is_none()
            );
        }
        assert_eq!(genomes.len(), 3);
        let mut actors = BTreeMap::new();
        for row in rows.iter().filter(|row| row[0] == "A") {
            assert_eq!(row.len(), 9);
            let state = OrganismState {
                total_children: row[3].parse().unwrap(),
                total_kills: row[4].parse().unwrap(),
                total_infected: row[5].parse().unwrap(),
                age: row[6].parse().unwrap(),
                alive: row[7].parse().unwrap(),
                active: row[8].parse().unwrap(),
                ..OrganismState::default()
            };
            let actor = Organism::unplaced(Rc::clone(&genomes[row[2]]), state).unwrap();
            assert!(
                actors
                    .insert(row[1].to_owned(), Rc::new(RefCell::new(actor)))
                    .is_none()
            );
        }
        let (mut snapshots, mut compared, mut excluded_rng, mut calls) = (0, 0, 0, 0);
        let mut stage = None;
        let mut fields = BTreeSet::new();
        let mut getters = BTreeSet::new();
        for row in rows {
            match row[0] {
                "C" | "U" | "A" => {}
                "I" => apply_input(row, &mut statistics, &actors, &genomes),
                "K" => {
                    call_event(row, &mut statistics, &actors);
                    calls += 1;
                }
                "P" => {
                    assert_eq!(row.len(), 2);
                    if stage.is_some() {
                        assert_eq!(fields.len(), 52);
                        assert_eq!(getters.len(), 49);
                    }
                    fields.clear();
                    getters.clear();
                    stage = Some(row[1]);
                    snapshots += 1;
                }
                "S" | "T" => {
                    assert_eq!(row.len(), 5);
                    assert_eq!(stage, Some(row[1]));
                    let context = format!("{case} {} {} {}", row[0], row[1], row[2]);
                    let value = if row[0] == "S" {
                        assert!(fields.insert(row[2]), "{context}");
                        observed_field(&statistics, row[2])
                    } else {
                        assert!(getters.insert(row[2]), "{context}");
                        observed_getter(&statistics, row[2])
                    };
                    assert_observation(value, row[3], row[4], &genomes, &context);
                    compared += 1;
                }
                "G" => {
                    assert_eq!(row.len(), 5);
                    if row[1] != "input" {
                        assert_eq!(stage, Some(row[1]));
                    }
                    if row[3] == "_reference" {
                        assert!(
                            Rc::ptr_eq(&genomes[row[2]], &genomes[row[4]]),
                            "{case} {row:?}"
                        );
                    } else {
                        assert_eq!(
                            genome_value(&genomes[row[2]].borrow(), row[3]),
                            row[4],
                            "{case} {row:?}"
                        );
                    }
                    compared += 1;
                }
                "N" => {
                    assert_eq!(row.len(), 6);
                    if row[1] != "input" {
                        assert_eq!(stage, Some(row[1]));
                    }
                    assert_eq!(
                        gene_value(&genomes[row[2]].borrow(), row[3].parse().unwrap(), row[4]),
                        row[5],
                        "{case} {row:?}"
                    );
                    compared += 1;
                }
                "R" => {
                    // Java constructor draws belong to actor setup, outside the statistics module
                    assert_eq!(row.len(), 5);
                    assert_eq!(stage, Some(row[1]));
                    row[2].parse::<i64>().unwrap();
                    row[3].parse::<i32>().unwrap();
                    double(row[4]);
                    excluded_rng += 1;
                }
                "E" => {
                    assert_eq!(row.len(), 1);
                    assert_eq!(fields.len(), 52);
                    assert_eq!(getters.len(), 49);
                }
                _ => panic!("Unmapped statistics fixture row {row:?}"),
            }
        }
        (snapshots, compared, excluded_rng, calls)
    }

    #[test]
    fn all_recorded_java_statistics_fields_getters_and_events_match_exactly() {
        // Null-genotype diagnostic actors are outside the valid Organism contract
        let fixture = include_str!("../tests/fixtures/statistics.tsv");
        let mut current = Vec::new();
        let mut totals = (0, 0, 0, 0, 0);
        for line in fixture.lines() {
            let row: Vec<_> = line.split('\t').collect();
            if row[0] == "C" {
                assert!(current.is_empty());
                assert_eq!(row[2], "canonical");
            }
            let end = row[0] == "E";
            current.push(row);
            if end {
                let (snapshots, compared, excluded_rng, calls) = replay_statistics_case(&current);
                totals.0 += 1;
                totals.1 += snapshots;
                totals.2 += compared;
                totals.3 += excluded_rng;
                totals.4 += calls;
                current.clear();
            }
        }
        assert!(current.is_empty());
        assert_eq!(totals, (29, 134, 46_786, 134, 255));
    }

    pub(crate) fn set_statistics_input(
        statistics: &mut WorldStatistics,
        field: &str,
        kind: &str,
        value: &str,
        genomes: &BTreeMap<String, GenomeRef>,
    ) {
        match field {
            "time" => {
                assert_eq!(kind, "long");
                statistics.time = value.parse::<i64>().unwrap();
            }
            "maxPopulation" => {
                assert_eq!(kind, "int");
                statistics.max_population = value.parse::<i32>().unwrap();
            }
            "maxBirths" => {
                assert_eq!(kind, "int");
                statistics.max_births = value.parse::<i32>().unwrap();
            }
            "maxDeaths" => {
                assert_eq!(kind, "int");
                statistics.max_deaths = value.parse::<i32>().unwrap();
            }
            "maxPopulationTime" => {
                assert_eq!(kind, "long");
                statistics.max_population_time = value.parse::<i64>().unwrap();
            }
            "minPopulation" => {
                assert_eq!(kind, "int");
                statistics.min_population = value.parse::<i32>().unwrap();
            }
            "minPopulationTime" => {
                assert_eq!(kind, "long");
                statistics.min_population_time = value.parse::<i64>().unwrap();
            }
            "massExtintions" => {
                assert_eq!(kind, "int");
                statistics.mass_extinctions = value.parse::<i32>().unwrap();
            }
            "massExtintionState" => {
                assert_eq!(kind, "int");
                statistics.mass_extinction_state = value.parse::<i32>().unwrap();
            }
            "createdOrganisms" => {
                assert_eq!(kind, "int");
                statistics.created_organisms = value.parse::<i32>().unwrap();
            }
            "populationSum" => {
                assert_eq!(kind, "long");
                statistics.population_sum = value.parse::<i64>().unwrap();
            }
            "deathSum" => {
                assert_eq!(kind, "long");
                statistics.death_sum = value.parse::<i64>().unwrap();
            }
            "deathLastTime" => {
                assert_eq!(kind, "int");
                statistics.death_last_time = value.parse::<i32>().unwrap();
            }
            "birthSum" => {
                assert_eq!(kind, "long");
                statistics.birth_sum = value.parse::<i64>().unwrap();
            }
            "birthLastTime" => {
                assert_eq!(kind, "int");
                statistics.birth_last_time = value.parse::<i32>().unwrap();
            }
            "infectionsSum" => {
                assert_eq!(kind, "long");
                statistics.infections_sum = value.parse::<i64>().unwrap();
            }
            "maxOxygen" => {
                assert_eq!(kind, "double");
                statistics.max_oxygen = double(value);
            }
            "maxOxygenTime" => {
                assert_eq!(kind, "long");
                statistics.max_oxygen_time = value.parse::<i64>().unwrap();
            }
            "minOxygen" => {
                assert_eq!(kind, "double");
                statistics.min_oxygen = double(value);
            }
            "minOxygenTime" => {
                assert_eq!(kind, "long");
                statistics.min_oxygen_time = value.parse::<i64>().unwrap();
            }
            "maxCarbonDioxide" => {
                assert_eq!(kind, "double");
                statistics.max_carbon_dioxide = double(value);
            }
            "maxMethane" => {
                assert_eq!(kind, "double");
                statistics.max_methane = double(value);
            }
            "maxCarbonDioxideTime" => {
                assert_eq!(kind, "long");
                statistics.max_carbon_dioxide_time = value.parse::<i64>().unwrap();
            }
            "maxMethaneTime" => {
                assert_eq!(kind, "long");
                statistics.max_methane_time = value.parse::<i64>().unwrap();
            }
            "minCarbonDioxide" => {
                assert_eq!(kind, "double");
                statistics.min_carbon_dioxide = double(value);
            }
            "minMethane" => {
                assert_eq!(kind, "double");
                statistics.min_methane = double(value);
            }
            "minCarbonDioxideTime" => {
                assert_eq!(kind, "long");
                statistics.min_carbon_dioxide_time = value.parse::<i64>().unwrap();
            }
            "minMethaneTime" => {
                assert_eq!(kind, "long");
                statistics.min_methane_time = value.parse::<i64>().unwrap();
            }
            "aliveBeingMostChildren" => {
                assert_eq!(kind, "GeneticCode");
                statistics.alive_being_most_children = reference(value, genomes);
            }
            "aliveBeingMostChildrenNumber" => {
                assert_eq!(kind, "int");
                statistics.alive_being_most_children_number = value.parse::<i32>().unwrap();
            }
            "aliveBeingMostKills" => {
                assert_eq!(kind, "GeneticCode");
                statistics.alive_being_most_kills = reference(value, genomes);
            }
            "aliveBeingMostKillsNumber" => {
                assert_eq!(kind, "int");
                statistics.alive_being_most_kills_number = value.parse::<i32>().unwrap();
            }
            "aliveBeingMostInfections" => {
                assert_eq!(kind, "GeneticCode");
                statistics.alive_being_most_infections = reference(value, genomes);
            }
            "aliveBeingMostInfectionsNumber" => {
                assert_eq!(kind, "int");
                statistics.alive_being_most_infections_number = value.parse::<i32>().unwrap();
            }
            "beingMostChildren" => {
                assert_eq!(kind, "GeneticCode");
                statistics.being_most_children = reference(value, genomes);
            }
            "beingMostChildrenNumber" => {
                assert_eq!(kind, "int");
                statistics.being_most_children_number = value.parse::<i32>().unwrap();
            }
            "beingMostChildrenTime" => {
                assert_eq!(kind, "long");
                statistics.being_most_children_time = value.parse::<i64>().unwrap();
            }
            "beingMostKills" => {
                assert_eq!(kind, "GeneticCode");
                statistics.being_most_kills = reference(value, genomes);
            }
            "beingMostKillsNumber" => {
                assert_eq!(kind, "int");
                statistics.being_most_kills_number = value.parse::<i32>().unwrap();
            }
            "beingMostKillsTime" => {
                assert_eq!(kind, "long");
                statistics.being_most_kills_time = value.parse::<i64>().unwrap();
            }
            "beingMostInfections" => {
                assert_eq!(kind, "GeneticCode");
                statistics.being_most_infections = reference(value, genomes);
            }
            "beingMostInfectionsNumber" => {
                assert_eq!(kind, "int");
                statistics.being_most_infections_number = value.parse::<i32>().unwrap();
            }
            "beingMostInfectionsTime" => {
                assert_eq!(kind, "long");
                statistics.being_most_infections_time = value.parse::<i64>().unwrap();
            }
            "lastBornBeing" => {
                assert_eq!(kind, "GeneticCode");
                statistics.last_born_being = reference(value, genomes);
            }
            "lastDeadBeing" => {
                assert_eq!(kind, "GeneticCode");
                statistics.last_dead_being = reference(value, genomes);
            }
            "lastInfectedBeing" => {
                assert_eq!(kind, "GeneticCode");
                statistics.last_infected_being = reference(value, genomes);
            }
            "populationList" => {
                assert_eq!(kind, "List");
                statistics.population_list = Some(HistoryListRef::from_values(history(value)));
            }
            "deathList" => {
                assert_eq!(kind, "List");
                statistics.death_list = Some(HistoryListRef::from_values(history(value)));
            }
            "birthList" => {
                assert_eq!(kind, "List");
                statistics.birth_list = Some(HistoryListRef::from_values(history(value)));
            }
            "oxygenList" => {
                assert_eq!(kind, "List");
                statistics.oxygen_list = Some(HistoryListRef::from_values(history(value)));
            }
            "carbonDioxideList" => {
                assert_eq!(kind, "List");
                statistics.carbon_dioxide_list = Some(HistoryListRef::from_values(history(value)));
            }
            "methaneList" => {
                assert_eq!(kind, "List");
                statistics.methane_list = Some(HistoryListRef::from_values(history(value)));
            }
            _ => panic!("Unmapped controlled Java statistics field {field}"),
        }
    }

    pub(crate) fn observed_field<'a>(statistics: &'a WorldStatistics, name: &str) -> Observation<'a> {
        match name {
            "time" => Observation::Long(statistics.time),
            "maxPopulation" => Observation::Int(statistics.max_population),
            "maxBirths" => Observation::Int(statistics.max_births),
            "maxDeaths" => Observation::Int(statistics.max_deaths),
            "maxPopulationTime" => Observation::Long(statistics.max_population_time),
            "minPopulation" => Observation::Int(statistics.min_population),
            "minPopulationTime" => Observation::Long(statistics.min_population_time),
            "massExtintions" => Observation::Int(statistics.mass_extinctions),
            "massExtintionState" => Observation::Int(statistics.mass_extinction_state),
            "createdOrganisms" => Observation::Int(statistics.created_organisms),
            "populationSum" => Observation::Long(statistics.population_sum),
            "deathSum" => Observation::Long(statistics.death_sum),
            "deathLastTime" => Observation::Int(statistics.death_last_time),
            "birthSum" => Observation::Long(statistics.birth_sum),
            "birthLastTime" => Observation::Int(statistics.birth_last_time),
            "infectionsSum" => Observation::Long(statistics.infections_sum),
            "maxOxygen" => Observation::Double(statistics.max_oxygen),
            "maxOxygenTime" => Observation::Long(statistics.max_oxygen_time),
            "minOxygen" => Observation::Double(statistics.min_oxygen),
            "minOxygenTime" => Observation::Long(statistics.min_oxygen_time),
            "maxCarbonDioxide" => Observation::Double(statistics.max_carbon_dioxide),
            "maxMethane" => Observation::Double(statistics.max_methane),
            "maxCarbonDioxideTime" => Observation::Long(statistics.max_carbon_dioxide_time),
            "maxMethaneTime" => Observation::Long(statistics.max_methane_time),
            "minCarbonDioxide" => Observation::Double(statistics.min_carbon_dioxide),
            "minMethane" => Observation::Double(statistics.min_methane),
            "minCarbonDioxideTime" => Observation::Long(statistics.min_carbon_dioxide_time),
            "minMethaneTime" => Observation::Long(statistics.min_methane_time),
            "aliveBeingMostChildren" => {
                Observation::Genome(statistics.alive_being_most_children.as_ref())
            }
            "aliveBeingMostChildrenNumber" => {
                Observation::Int(statistics.alive_being_most_children_number)
            }
            "aliveBeingMostKills" => {
                Observation::Genome(statistics.alive_being_most_kills.as_ref())
            }
            "aliveBeingMostKillsNumber" => {
                Observation::Int(statistics.alive_being_most_kills_number)
            }
            "aliveBeingMostInfections" => {
                Observation::Genome(statistics.alive_being_most_infections.as_ref())
            }
            "aliveBeingMostInfectionsNumber" => {
                Observation::Int(statistics.alive_being_most_infections_number)
            }
            "beingMostChildren" => Observation::Genome(statistics.being_most_children.as_ref()),
            "beingMostChildrenNumber" => Observation::Int(statistics.being_most_children_number),
            "beingMostChildrenTime" => Observation::Long(statistics.being_most_children_time),
            "beingMostKills" => Observation::Genome(statistics.being_most_kills.as_ref()),
            "beingMostKillsNumber" => Observation::Int(statistics.being_most_kills_number),
            "beingMostKillsTime" => Observation::Long(statistics.being_most_kills_time),
            "beingMostInfections" => Observation::Genome(statistics.being_most_infections.as_ref()),
            "beingMostInfectionsNumber" => {
                Observation::Int(statistics.being_most_infections_number)
            }
            "beingMostInfectionsTime" => Observation::Long(statistics.being_most_infections_time),
            "lastBornBeing" => Observation::Genome(statistics.last_born_being.as_ref()),
            "lastDeadBeing" => Observation::Genome(statistics.last_dead_being.as_ref()),
            "lastInfectedBeing" => Observation::Genome(statistics.last_infected_being.as_ref()),
            "populationList" => Observation::History(statistics.population_list.as_ref()),
            "deathList" => Observation::History(statistics.death_list.as_ref()),
            "birthList" => Observation::History(statistics.birth_list.as_ref()),
            "oxygenList" => Observation::History(statistics.oxygen_list.as_ref()),
            "carbonDioxideList" => Observation::History(statistics.carbon_dioxide_list.as_ref()),
            "methaneList" => Observation::History(statistics.methane_list.as_ref()),
            _ => panic!("Unmapped Java statistics field {name}"),
        }
    }

    pub(crate) fn observed_getter<'a>(statistics: &'a WorldStatistics, name: &str) -> Observation<'a> {
        match name {
            "getTime" => Observation::Long(statistics.get_time()),
            "getMaxPopulation" => Observation::Int(statistics.get_max_population()),
            "getMaxBirth" => Observation::Int(statistics.get_max_birth()),
            "getMaxDeaths" => Observation::Int(statistics.get_max_deaths()),
            "getMaxPopulationTime" => Observation::Long(statistics.get_max_population_time()),
            "getMinPopulation" => Observation::Int(statistics.get_min_population()),
            "getMinPopulationTime" => Observation::Long(statistics.get_min_population_time()),
            "getMassExtintions" => Observation::Int(statistics.get_mass_extinctions()),
            "getCreatedOrganisms" => Observation::Int(statistics.get_created_organisms()),
            "getAveragePopulation" => Observation::Double(statistics.get_average_population()),
            "getAverageDeaths" => Observation::Double(statistics.get_average_deaths()),
            "getAverageBirths" => Observation::Double(statistics.get_average_births()),
            "getAverageInfections" => Observation::Double(statistics.get_average_infections()),
            "getMaxOxygen" => Observation::Double(statistics.get_max_oxygen()),
            "getMaxOxygenTime" => Observation::Long(statistics.get_max_oxygen_time()),
            "getMinOxygen" => Observation::Double(statistics.get_min_oxygen()),
            "getMinOxygenTime" => Observation::Long(statistics.get_min_oxygen_time()),
            "getMaxCarbonDioxide" => Observation::Double(statistics.get_max_carbon_dioxide()),
            "getMaxCarbonDioxideTime" => {
                Observation::Long(statistics.get_max_carbon_dioxide_time())
            }
            "getMinCarbonDioxide" => Observation::Double(statistics.get_min_carbon_dioxide()),
            "getMinCarbonDioxideTime" => {
                Observation::Long(statistics.get_min_carbon_dioxide_time())
            }
            "getMaxMethane" => Observation::Double(statistics.get_max_methane()),
            "getMaxMethaneTime" => Observation::Long(statistics.get_max_methane_time()),
            "getMinMethane" => Observation::Double(statistics.get_min_methane()),
            "getMinMethaneTime" => Observation::Long(statistics.get_min_methane_time()),
            "getAliveBeingMostChildren" => {
                Observation::Genome(statistics.get_alive_being_most_children())
            }
            "getAliveBeingMostChildrenNumber" => {
                Observation::Int(statistics.get_alive_being_most_children_number())
            }
            "getAliveBeingMostKills" => {
                Observation::Genome(statistics.get_alive_being_most_kills())
            }
            "getAliveBeingMostKillsNumber" => {
                Observation::Int(statistics.get_alive_being_most_kills_number())
            }
            "getAliveBeingMostInfections" => {
                Observation::Genome(statistics.get_alive_being_most_infections())
            }
            "getAliveBeingMostInfectionsNumber" => {
                Observation::Int(statistics.get_alive_being_most_infections_number())
            }
            "getBeingMostChildren" => Observation::Genome(statistics.get_being_most_children()),
            "getBeingMostChildrenNumber" => {
                Observation::Int(statistics.get_being_most_children_number())
            }
            "getBeingMostChildrenTime" => {
                Observation::Long(statistics.get_being_most_children_time())
            }
            "getBeingMostKills" => Observation::Genome(statistics.get_being_most_kills()),
            "getBeingMostKillsNumber" => Observation::Int(statistics.get_being_most_kills_number()),
            "getBeingMostKillsTime" => Observation::Long(statistics.get_being_most_kills_time()),
            "getBeingMostInfections" => Observation::Genome(statistics.get_being_most_infections()),
            "getBeingMostInfectionsNumber" => {
                Observation::Int(statistics.get_being_most_infections_number())
            }
            "getBeingMostInfectionsTime" => {
                Observation::Long(statistics.get_being_most_infections_time())
            }
            "getLastBornBeing" => Observation::Genome(statistics.get_last_born_being()),
            "getLastDeadBeing" => Observation::Genome(statistics.get_last_dead_being()),
            "getLastInfectedBeing" => Observation::Genome(statistics.get_last_infected_being()),
            "getPopulationList" => Observation::History(statistics.get_population_list()),
            "getDeathList" => Observation::History(statistics.get_death_list()),
            "getBirthList" => Observation::History(statistics.get_birth_list()),
            "getOxygenList" => Observation::History(statistics.get_oxygen_list()),
            "getCarbonDioxideList" => Observation::History(statistics.get_carbon_dioxide_list()),
            "getMethaneList" => Observation::History(statistics.get_methane_list()),
            _ => panic!("Unmapped Java statistics getter {name}"),
        }
    }
}

#[path = "legacy_statistics_export.rs"]
mod legacy_export;
