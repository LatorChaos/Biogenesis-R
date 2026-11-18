// SPDX-License-Identifier: GPL-2.0-or-later

//! Core projections of original StatisticsWindow observations, with presentation retained separately

use super::*;
use crate::world::organism_oracle_tests::{
    empty_reference_world, explicit_genome, gene_fields, genome_fields, Fields, Stage,
};
use crate::world::organism_oracle_tests::settings_oracle_tests::text;
use std::collections::{BTreeMap, BTreeSet};

const SOURCE: &str = include_str!("../tests/fixtures/statistics_ui_source.tsv");
const RECORDS: [RecordKind; 6] = [
    RecordKind::AliveChildren,
    RecordKind::HistoricalChildren,
    RecordKind::AliveKills,
    RecordKind::HistoricalKills,
    RecordKind::AliveInfections,
    RecordKind::HistoricalInfections,
];
// Original ColorPanel's preferred dimensions, independent of observed output rows
const COLOR_SIZE: [i32; 2] = [640, 25];

type Row<'a> = Vec<&'a str>;

#[derive(Default, Debug, PartialEq, Eq)]
struct Ledger {
    partitions: BTreeMap<&'static str, usize>,
    cases: usize,
    snapshots: usize,
    operations: usize,
    core_operations: usize,
}

fn disposition(row: &[&str], raw: bool) -> &'static str {
    match row[0] {
        "C" | "K" | "P" | "E" => "schema_commands",
        "U" | "I" | "Q" | "N" | "A0" => "constructor_control_inputs",
        "A" if row.len() == 5 => "constructor_control_inputs",
        _ if raw => "raw_diagnostic_unpassed",
        "W" | "A" | "G" | "D" | "C0" | "R" => "core_observation_comparisons",
        "H" if ["current_statistics", "window_statistics", "clipboard"].contains(&row[2]) => {
            "owner_canvas_projections"
        }
        "H" => "core_observation_comparisons",
        "Z" => "owner_canvas_projections",
        "F" if row[3] == "0" => "source_host_observer_metadata",
        "F" => "core_observation_comparisons",
        "V" if row[2].starts_with("retained_alive_") || row[2].starts_with("preview") => {
            "core_observation_comparisons"
        }
        "V" | "L" | "T" | "M" => "presentation_deferred",
        "B" | "Y" => "source_host_observer_metadata",
        _ => panic!("Unmapped source row {row:?}"),
    }
}

fn validate_row(row: &[&str]) {
    let length = match row[0] {
        "C" | "U" | "V" | "L" | "T" | "X" => 4,
        "I" | "K" | "A0" | "Y" | "Z" | "W" | "C0" => 5,
        "Q" | "R" => 6,
        "G" => 10,
        "N" | "D" | "M" => 7,
        "P" | "E" => 2,
        "H" => 4,
        "B" | "F" => 9,
        "A" => { assert!([5, 11].contains(&row.len())); row.len() }
        other => panic!("Unknown source tag {other}"),
    };
    assert_eq!(row.len(), length, "Source row width {row:?}");
}

fn bits(value: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(value, 16).unwrap())
}

fn assert_bits(actual: f64, expected: &str, context: &str) {
    let expected_value = bits(expected);
    assert!(
        (actual.is_nan() && expected_value.is_nan()) || actual.to_bits() == expected_value.to_bits(),
        "{context}: actual {:016x}, source {expected}", actual.to_bits()
    );
}

fn assert_value(actual: impl ToString, expected: &str, context: &str) {
    assert_eq!(actual.to_string(), expected, "{context}");
}

struct RetainedView {
    handle: StatisticsHandle,
    view: StatisticsView,
}

struct Replay {
    world: World,
    genomes: BTreeMap<String, GenomeRef>,
    actors: BTreeMap<String, OrganismRef>,
    owners: Vec<StatisticsHandle>,
    window: Option<RetainedView>,
    clipboard: Option<GenomeHandle>,
    prepared_genomes: BTreeSet<String>,
}

impl Replay {
    fn from_inputs(rows: &[Row<'_>]) -> Self {
        let mut settings = Fields::new();
        let mut stages: BTreeMap<String, Stage> = BTreeMap::new();
        for row in rows {
            match row[0] {
                "U" => { assert!(settings.insert(row[1].into(), row[3].into()).is_none()); }
                "I" if row[2].starts_with("utils:") => {
                    assert_eq!(row[1], "setup");
                    let value = match row[3] {
                        "double" => format!("{:016x}", row[4].parse::<f64>().unwrap().to_bits()),
                        "String" => text(row[4]),
                        "boolean" | "int" => row[4].into(),
                        _ => panic!("Unknown Utils input {row:?}"),
                    };
                    settings.insert(row[2][6..].into(), value);
                }
                "Q" => {
                    assert_eq!(row[1], "input");
                    let value = if row[4] == "String" { text(row[5]) } else { row[5].into() };
                    assert!(stages.entry(row[2].into()).or_default().genome.insert(row[3].into(), value).is_none());
                }
                "N" => {
                    assert_eq!(row[1], "input");
                    assert!(stages.entry(row[2].into()).or_default().genes.entry(row[3].parse().unwrap())
                        .or_default().insert(row[4].into(), row[6].into()).is_none());
                }
                _ => {}
            }
        }
        assert_eq!(settings.len(), 149);
        assert_eq!(stages.len(), 3);
        let seed = rows[0][3].parse().unwrap();
        let world = empty_reference_world(&settings, seed);
        let mut genomes = BTreeMap::new();
        for (name, stage) in stages {
            assert_eq!(stage.genome.len(), 26);
            assert!(!stage.genes.is_empty());
            for (index, fields) in &stage.genes {
                assert!(*index < stage.genes.len());
                assert_eq!(fields.len(), 40);
            }
            let genome = explicit_genome(&stage);
            assert_eq!(genome_fields(&genome), stage.genome, "Constructor genotype {name}");
            for (index, fields) in stage.genes {
                assert_eq!(gene_fields(&genome.gene_ref(index).unwrap().borrow()), fields, "Constructor gene {name}:{index}");
            }
            genomes.insert(name, Rc::new(RefCell::new(genome)));
        }
        Self { world, genomes, actors: BTreeMap::new(), owners: Vec::new(), window: None,
               clipboard: None, prepared_genomes: BTreeSet::new() }
    }

    fn genome_handle(&self, label: &str) -> GenomeHandle {
        GenomeHandle::from_shared(Rc::clone(&self.genomes[label]))
    }

    fn genome_label(&self, handle: Option<&GenomeHandle>) -> String {
        let Some(handle) = handle else { return "~".into(); };
        self.genomes.keys().find_map(|name| {
            (self.genome_handle(name) == *handle).then(|| name.clone())
        }).expect("Unexpected genotype identity")
    }

    fn owner_label(&mut self, handle: Option<&StatisticsHandle>) -> String {
        let Some(handle) = handle else { return "~".into(); };
        let index = if let Some(index) = self.owners.iter().position(|known| known == handle) { index }
        else { self.owners.push(handle.clone()); self.owners.len() - 1 };
        format!("s{index}")
    }

    fn paste(&mut self, row: &[&str]) {
        assert!(!self.actors.contains_key(row[1]), "Duplicate actor input");
        let genome = self.genome_handle(row[2]);
        let handle = self.world.paste_genome_handle(&genome,
            [row[3].parse().unwrap(), row[4].parse().unwrap()]).unwrap().expect("Actual input paste failed");
        self.actors.insert(row[1].into(), Rc::clone(&handle.0));
    }

    fn input(&mut self, row: &[&str]) {
        assert!(!row[2].is_empty());
        if row[2].starts_with("utils:") { return; }
        let path: Vec<_> = row[2].split(':').collect();
        match path.as_slice() {
            ["rng", "seed"] => { assert_eq!(row[3], "long"); assert_eq!(self.world.random_snapshot(), crate::JavaRandom::new(row[4].parse().unwrap())); }
            ["genome", name, "construction"] => {
                assert_eq!(row[3], "String"); assert_eq!(row[4], "explicit");
                assert!(self.genomes.contains_key(*name));
                assert!(self.prepared_genomes.insert((*name).into()));
            }
            ["statistics", field] => {
                assert_eq!(row[3], "List");
                assert!(["populationList", "deathList", "birthList", "oxygenList", "carbonDioxideList", "methaneList"].contains(field));
                crate::statistics::tests::set_statistics_input(&mut self.world.statistics_ref().borrow_mut(), field, row[3], row[4], &self.genomes);
            }
            ["actor", name, field] => {
                let mut actor = self.actors[*name].borrow_mut();
                match *field {
                    "_nTotalChildren" => { assert_eq!(row[3], "int"); actor.state.total_children = row[4].parse().unwrap(); }
                    "_nTotalKills" => { assert_eq!(row[3], "int"); actor.state.total_kills = row[4].parse().unwrap(); }
                    "_nTotalInfected" => { assert_eq!(row[3], "int"); actor.state.total_infected = row[4].parse().unwrap(); }
                    "active" => { assert_eq!(row[3], "boolean"); actor.state.active = row[4].parse().unwrap(); }
                    _ => panic!("Unknown actor input {row:?}"),
                }
            }
            ["genome", name, "_cladeID"] => { assert_eq!(row[3], "String"); self.genomes[*name].borrow_mut().set_clade_id(Some(text(row[4]))); }
            ["genome", name, "_generation"] => { assert_eq!(row[3], "int"); self.genomes[*name].borrow_mut().set_generation(row[4].parse().unwrap()); }
            ["gene", name, index, "_length"] => { assert_eq!(row[3], "double"); self.genomes[*name].borrow().gene_ref(index.parse().unwrap()).unwrap().borrow_mut().set_length(row[4].parse().unwrap()); }
            ["gene", name, index, "_color"] => {
                assert_eq!((row[3], row[4]), ("Color", "~"));
                self.genomes[*name].borrow().gene_ref(index.parse().unwrap()).unwrap().borrow_mut().set_color(None);
            }
            ["clipboard"] => { assert_eq!(row[3], "GenomeRef"); self.clipboard = Some(self.genome_handle(row[4])); }
            _ => panic!("Unknown controlled input {row:?}"),
        }
    }

    fn operation(&mut self, row: &[&str]) -> bool {
        match row[2] {
            "open" => {
                assert_eq!((&row[3], &row[4]), (&"~", &"~"));
                let handle = self.world.open_statistics();
                let view = self.world.refresh_statistics(&handle).unwrap();
                self.window = Some(RetainedView { handle, view });
            }
            "update" => {
                assert_eq!((&row[3], &row[4]), (&"~", &"~"));
                let window = self.window.as_mut().unwrap();
                window.view = self.world.refresh_statistics(&window.handle).unwrap();
            }
            "close" => { assert_eq!((&row[3], &row[4]), (&"~", &"~")); return false; }
            "copy" => {
                assert_eq!(row[3], "~");
                let record = self.window.as_ref().unwrap().view.record(RECORDS[row[4].parse::<usize>().unwrap()]);
                if let Some(genome) = record.genome { self.clipboard = Some(genome); }
                return false;
            }
            "die" => {
                let actor = Rc::clone(&self.actors[row[3]]);
                let mut actor = actor.borrow_mut();
                if row[4] == "~" { self.world.die(&mut actor, None).unwrap(); }
                else {
                    let killer = Rc::clone(&self.actors[row[4]]);
                    assert!(!Rc::ptr_eq(&killer, &self.actors[row[3]]));
                    self.world.die(&mut actor, Some(crate::world::death::Killer::Other(&mut killer.borrow_mut()))).unwrap();
                }
            }
            "infect" => {
                let recipient = Rc::clone(&self.actors[row[3]]);
                let donor = Rc::clone(&self.actors[row[4]]);
                assert!(!Rc::ptr_eq(&recipient, &donor));
                self.world.infect_from(&mut recipient.borrow_mut(), crate::world::infection::Infector::Other(&mut donor.borrow_mut()), crate::world::infection::InfectionKind::Direct).unwrap();
            }
            "record_birth" => {
                let child = self.actors[row[3]].borrow();
                let parent = self.actors[row[4]].borrow();
                self.world.statistics_ref().borrow_mut().event_organism_born(&child, &parent);
            }
            "time" => {
                assert_eq!(row[3], "~"); assert!(self.world.organisms.borrow().is_empty());
                let count: usize = row[4].parse().unwrap(); assert_eq!(count, 256);
                for _ in 0..count { self.world.single_step().unwrap(); }
            }
            "new_game" => {
                assert_eq!((&row[3], &row[4]), (&"~", &"~"));
                let (width, height) = self.world.dimensions();
                let parameters = self.world.parameters().clone();
                self.world.new_game(width, height, 0, parameters).unwrap();
            }
            // The adjacent explicit A0 input performs the actual paste
            "paste" => { assert!(!self.actors.contains_key(row[3])); assert!(self.genomes.contains_key(row[4])); }
            _ => panic!("Unknown original operation {row:?}"),
        }
        true
    }

    fn view(&self) -> &StatisticsView { &self.window.as_ref().unwrap().view }

    fn preview(&self, index: usize) -> Option<GenomeHandle> { self.view().record(RECORDS[index]).genome }

    fn history_size(&self, index: usize) -> usize {
        let statistics = self.window.as_ref().unwrap().handle.0.borrow();
        match index {
            0 => statistics.get_death_list().unwrap().len(), 1 => statistics.get_birth_list().unwrap().len(),
            2 => statistics.get_population_list().unwrap().len(), 3 => statistics.get_oxygen_list().unwrap().len(),
            4 => statistics.get_carbon_dioxide_list().unwrap().len(), 5 => statistics.get_methane_list().unwrap().len(),
            _ => panic!("Unknown graph series"),
        }
    }

    fn observe(&mut self, row: &[&str], context: &str) {
        match row[0] {
            "W" => match row[2] {
                "time" => assert_value(self.world.time(), row[4], context),
                "frame" => assert_value(self.world.frame(), row[4], context),
                "population" => assert_value(self.world.population(), row[4], context),
                "corpses" => assert_value(self.world.corpse_count().unwrap(), row[4], context),
                "members" => assert_value(self.world.organisms.borrow().len(), row[4], context),
                "oxygen" => assert_bits(self.world.oxygen(), row[4], context),
                "carbon_dioxide" => assert_bits(self.world.carbon_dioxide(), row[4], context),
                "methane" => assert_bits(self.world.methane(), row[4], context),
                _ => panic!("Unknown World observation {row:?}"),
            },
            "A" => {
                let actor = self.actors[row[2]].borrow();
                let actual = [actor.id.to_string(), actor.state.alive.to_string(), actor.state.active.to_string(),
                    actor.state.total_children.to_string(), actor.state.total_kills.to_string(), actor.state.total_infected.to_string(),
                    self.world.organisms.borrow().iter().any(|member| Rc::ptr_eq(member, &self.actors[row[2]])).to_string(),
                    self.genome_label(Some(&GenomeHandle::from_shared(Rc::clone(actor.genome.as_ref().unwrap()))))];
                for (value, expected) in actual.iter().zip(&row[3..]) { assert_value(value, expected, context); }
            }
            "H" => {
                let actual = match row[2] {
                    "current_statistics" => { let handle = self.world.open_statistics(); self.owner_label(Some(&handle)) }
                    "window_statistics" => { let handle = self.window.as_ref().map(|window| window.handle.clone()); self.owner_label(handle.as_ref()) }
                    "clipboard" => self.genome_label(self.clipboard.as_ref()),
                    "retained_alive_children" | "retained_alive_kills" | "retained_alive_infections" => {
                        let statistics = self.window.as_ref().unwrap().handle.0.borrow();
                        let genome = match row[2] {
                            "retained_alive_children" => statistics.get_alive_being_most_children(),
                            "retained_alive_kills" => statistics.get_alive_being_most_kills(),
                            _ => statistics.get_alive_being_most_infections(),
                        }.map(|genome| GenomeHandle::from_shared(Rc::clone(genome)));
                        self.genome_label(genome.as_ref())
                    }
                    target if target.starts_with("preview") => self.genome_label(self.preview(target[7..].parse().unwrap()).as_ref()),
                    _ => panic!("Unknown reference observation {row:?}"),
                };
                assert_value(actual, row[3], context);
            }
            "V" if row[2].starts_with("retained_alive_") => {
                let statistics = self.window.as_ref().unwrap().handle.0.borrow();
                let actual = match row[2] {
                    "retained_alive_children_count" => statistics.get_alive_being_most_children_number(),
                    "retained_alive_kills_count" => statistics.get_alive_being_most_kills_number(),
                    "retained_alive_infections_count" => statistics.get_alive_being_most_infections_number(),
                    _ => panic!("Unknown retained count {row:?}"),
                };
                assert_value(actual, row[3], context);
            }
            "V" if row[2].starts_with("preview") => {
                let (name, field) = row[2].split_once(':').unwrap();
                let handle = self.preview(name[7..].parse().unwrap()).unwrap();
                let genome = handle.detached_clone();
                match field {
                    "clade" => assert_value(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref().unwrap_or("~"), &text(row[3]), context),
                    "generation" => assert_value(genome.generation(), row[3], context),
                    "gene0_length" => assert_bits(genome.gene_ref(0).unwrap().borrow().length(), row[3], context),
                    _ => panic!("Unknown live preview field {row:?}"),
                }
            }
            "G" => {
                let index = row[2].parse::<usize>().unwrap() * 3 + row[3].parse::<usize>().unwrap();
                let graph = &self.view().plot_series[index];
                assert_value(graph.argb as i32, row[4], context);
                assert_bits(graph.maximum, row[6], context); assert_bits(graph.minimum, row[7], context);
                assert_value(self.history_size(index), row[8], context);
                assert_value(graph.points.len(), row[9], context);
            }
            "D" => {
                let index = row[2].parse::<usize>().unwrap() * 3 + row[3].parse::<usize>().unwrap();
                let point = self.view().plot_series[index].points[row[4].parse::<usize>().unwrap()];
                assert_value(point[0], row[5], context); assert_value(point[1], row[6], context);
            }
            "C0" => {
                let entry = self.view().color_counts[row[2].parse::<usize>().unwrap()];
                assert_value(entry.count, row[3], context); assert_value(entry.argb as i32, row[4], context);
            }
            "Z" => { assert_eq!(row[2], "color_size"); assert_value(COLOR_SIZE[0], row[3], context); assert_value(COLOR_SIZE[1], row[4], context); }
            "F" if row[3] != "0" => {
                assert_eq!(row[2], "color");
                let rectangles = self.view().color_rectangles(COLOR_SIZE[0], COLOR_SIZE[1]);
                let entry = rectangles[row[3].parse::<usize>().unwrap() - 1];
                let actual = [entry.bounds.x, entry.bounds.y, entry.bounds.width, entry.bounds.height, entry.argb as i32];
                for (value, expected) in actual.iter().zip(&row[4..]) { assert_value(value, expected, context); }
            }
            "R" => {
                let before = self.world.random_snapshot();
                let mut copy = before.clone();
                assert_value(copy.next_i64(), row[3], context); assert_value(copy.bounded(10000), row[4], context);
                assert_bits(copy.next_f64(), row[5], context); assert_eq!(self.world.random_snapshot(), before);
            }
            "Y" => { assert_eq!(row[3], row[4], "Source purity invariant {context}"); }
            "B" | "L" | "T" | "M" | "V" | "F" => {}
            _ => panic!("Unmapped observation {row:?}"),
        }
    }
}

fn replay(source: &str) -> Ledger {
    let mut ledger = Ledger::default();
    let mut pending = Vec::new();
    for line in source.lines() {
        let row: Row<'_> = line.split('\t').collect();
        validate_row(&row);
        let end = row[0] == "E";
        pending.push(row);
        if !end { continue; }
        assert_eq!(pending[0][0], "C");
        let raw = pending[0][2] == "raw-diagnostic";
        assert!(raw || pending[0][2] == "canonical");
        ledger.cases += 1;
        let mut state = if raw { None } else { Some(Replay::from_inputs(&pending)) };
        let mut stage = "setup";
        let mut snapshots = 0;
        let mut calls = 0;
        for (index, row) in pending.iter().enumerate() {
            *ledger.partitions.entry(disposition(row, raw)).or_default() += 1;
            if raw { continue; }
            let state = state.as_mut().unwrap();
            let context = format!("{} row {} {:?}", pending[0][1], index + 1, row);
            match row[0] {
                "C" | "U" | "Q" | "N" => {}
                "I" => state.input(row),
                "A" if row.len() == 5 => state.paste(row),
                "A0" => {
                    assert!(index > 0 && pending[index - 1][0] == "K" && pending[index - 1][2] == "paste");
                    assert_eq!(row[1], pending[index - 1][3]); assert_eq!(row[2], pending[index - 1][4]);
                    state.paste(row);
                }
                "K" => {
                    assert_eq!(row[1], format!("step-{calls}")); calls += 1; ledger.operations += 1;
                    ledger.core_operations += usize::from(state.operation(row));
                }
                "P" => { stage = row[1]; snapshots += 1; ledger.snapshots += 1; }
                "E" => { assert_eq!(row[1], pending[0][1]); assert_eq!(snapshots, calls + 1); assert_eq!(state.prepared_genomes.len(), 3); }
                _ => { assert_eq!(row[1], stage); state.observe(row, &context); }
            }
        }
        pending.clear();
    }
    assert!(pending.is_empty());
    ledger
}

#[test]
fn original_statistics_ui_core_projections() {
    let ledger = replay(SOURCE);
    assert_eq!(ledger.partitions, BTreeMap::from([
        ("schema_commands", 105), ("constructor_control_inputs", 5175),
        ("core_observation_comparisons", 5196), ("owner_canvas_projections", 163),
        ("source_host_observer_metadata", 207), ("presentation_deferred", 2170),
        ("raw_diagnostic_unpassed", 276),
    ]));
    assert_eq!((ledger.cases, ledger.snapshots, ledger.operations, ledger.core_operations), (9, 44, 36, 25));
    assert_eq!(ledger.partitions.values().sum::<usize>(), 13292);
}

fn reject_changed_observation(tag: &str, predicate: impl Fn(&[&str]) -> bool, column: usize, value: &str, message: &str) {
    let mut lines: Vec<_> = SOURCE.lines().map(str::to_owned).collect();
    let index = lines.iter().position(|line| {
        let row: Vec<_> = line.split('\t').collect(); row[0] == tag && predicate(&row)
    }).expect("Missing declared negative-control output row");
    let mut row: Vec<_> = lines[index].split('\t').map(str::to_owned).collect();
    assert_ne!(row[column], value); row[column] = value.into(); lines[index] = row.join("\t");
    let changed = lines.join("\n") + "\n";
    let failure = std::panic::catch_unwind(|| replay(&changed)).expect_err("Changed source output was accepted");
    let text = failure.downcast_ref::<String>().map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied()).unwrap_or("");
    assert!(text.contains(message), "Wrong negative-control failure: {text}");
}

#[test]
fn rejects_changed_cached_graph_point() {
    reject_changed_observation("D", |row| row[1] == "after-0", 5, "12345", "D\"");
}

#[test]
fn rejects_changed_retained_statistics_identity() {
    reject_changed_observation("H", |row| row[2] == "window_statistics" && row[3] == "s0", 3, "s999", "window_statistics");
}


#[test]
fn original_statistics_null_color_failure_preserves_retained_state() {
    let rows: Vec<Row<'_>> = SOURCE.lines().map(|line| line.split('\t').collect()).collect();
    let start = rows.iter().position(|row| row[0] == "C" && row[1] == "null-gene-color-before-alive-scan").unwrap();
    let tail = &rows[start..];
    let end = tail.iter().position(|row| row[0] == "E").unwrap();
    let rows = &tail[..=end];
    assert_eq!(rows[0][2], "raw-diagnostic");
    assert_eq!(rows.len(), 651);
    let mut state = Replay::from_inputs(rows);
    let (mut observations, mut failures, mut commands, mut snapshots) = (0, 0, 0, 0);
    for (index, row) in rows.iter().enumerate() {
        validate_row(row);
        let context = format!("null-color row {} {:?}", index + 1, row);
        match row[0] {
            "C" | "U" | "Q" | "N" | "E" => {}
            "I" => state.input(row),
            "A" if row.len() == 5 => state.paste(row),
            "K" if row[2] == "update" => {
                assert_eq!(row.as_slice(), ["K", "step-1", "update", "~", "~"]);
                assert_eq!(rows[index + 1].as_slice(), ["X", "step-1", "java.lang.NullPointerException", "~"]);
                let retained = state.view().clone();
                let random = state.world.random_snapshot();
                let handle = &state.window.as_ref().unwrap().handle;
                assert_eq!(state.world.refresh_statistics(handle),
                    Err(StatisticsError::MissingGeneColor { organism_id: 0, gene_index: 0 }));
                assert_eq!(state.view(), &retained);
                assert_eq!(state.world.random_snapshot(), random);
                failures += 1;
                commands += 1;
            }
            "K" => { state.operation(row); commands += 1; }
            "X" => assert_eq!(failures, 1),
            "P" => snapshots += 1,
            _ => match disposition(row, false) {
                "core_observation_comparisons" | "owner_canvas_projections" => {
                    state.observe(row, &context);
                    observations += 1;
                }
                "source_host_observer_metadata" | "presentation_deferred" => {}
                other => panic!("Unexecuted null-color row {other}: {row:?}"),
            }
        }
    }
    assert_eq!((observations, failures, commands, snapshots), (178, 1, 3, 4));
    assert_eq!(state.prepared_genomes.len(), 3);
}
