// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay frozen Java maintenance observations without constructing actors from expected output

use super::death_oracle_tests::{
    apply_reference_input, compare_genomes, statistics_schemas, validate, validate_genome,
    world_schema,
};
use super::*;
use crate::organism::{GenomeRef, OrganismRef};
use crate::spatial::SpatialTree;
use crate::statistics::tests::{assert_observation, observed_field, observed_getter};
use std::rc::Rc;

pub(super) const WORLD_EXCLUSIONS: [&str; 4] = [
    "_visibleWorld",
    "inCorridors",
    "outCorridors",
    "_corridorexists",
];

#[derive(Default)]
struct Snapshot {
    organisms: BTreeMap<String, Fields>,
    references: BTreeMap<(String, String), String>,
    world: Fields,
    world_instances: Fields,
    statistics: BTreeMap<String, (String, String)>,
    getters: BTreeMap<String, (String, String)>,
    genomes: BTreeMap<String, Stage>,
    queries: Fields,
    insertions: Vec<(String, IntRect)>,
    booleans: BTreeMap<String, bool>,
    suffix: Option<(i64, i32, u64)>,
}

enum Step {
    Input {
        target: String,
        kind: String,
        value: String,
    },
    Actor {
        role: String,
        genome: String,
        center: [i32; 2],
    },
    Call {
        stage: String,
        operation: String,
        role: String,
        argument: String,
    },
    Discover {
        stage: String,
        role: String,
        id: i32,
        genome: String,
    },
    Snapshot(String),
}

struct MaintenanceCase {
    name: String,
    seed: i64,
    settings: Fields,
    input_genomes: BTreeMap<String, Stage>,
    snapshots: BTreeMap<String, Snapshot>,
    steps: Vec<Step>,
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Ledger {
    rows: usize,
    cases: usize,
    snapshots: usize,
    actor_views: usize,
    newborns: usize,
    observations: usize,
    input_genotypes: usize,
    organism_collaborators: usize,
    world_exclusions: usize,
    selection_metadata: usize,
    inputs_and_commands: usize,
    calls: BTreeMap<String, usize>,
}

fn expected_ledger() -> Ledger {
    Ledger {
        rows: 446_161,
        cases: 134,
        snapshots: 291,
        actor_views: 882,
        newborns: 9,
        observations: 342_785,
        input_genotypes: 77_720,
        organism_collaborators: 1_764,
        world_exclusions: 1_164,
        selection_metadata: 134,
        inputs_and_commands: 22_594,
        calls: BTreeMap::from([
            ("breath".into(), 33),
            ("symmetric_bounds".into(), 9),
            ("build_tree".into(), 6),
            ("decay".into(), 83),
            ("die".into(), 5),
            ("useDecayEnergy".into(), 7),
            ("blueSpore".into(), 2),
            ("hatch".into(), 11),
            ("cyanSpore".into(), 1),
        ]),
    }
}

pub(super) fn instance_schema() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("NEXT_ID", "int"),
        ("_CH4", "double"),
        ("_CO2", "double"),
        ("_O2", "double"),
        ("_corridorexists", "boolean"),
        ("_height", "int"),
        ("_organisms", "List"),
        ("_population", "int"),
        ("_visibleWorld", "VisibleWorld"),
        ("_width", "int"),
        ("colDetTree", "STRtree"),
        ("inCorridors", "List"),
        ("nFrames", "int"),
        ("outCorridors", "List"),
        ("worldStatistics", "WorldStatistics"),
    ])
}

pub(super) fn reference_label(value: &str) {
    if value != "~" {
        value.strip_prefix('g').unwrap().parse::<usize>().unwrap();
    }
}

pub(super) fn role_list(value: &str) -> Vec<&str> {
    if value.is_empty() || value == "~" {
        Vec::new()
    } else {
        value.split(',').collect()
    }
}

fn parse(source: &str) -> (Vec<MaintenanceCase>, Ledger) {
    let (setting_types, organism_types) = primitive_reference_schemas();
    let (statistic_types, getter_types) = statistics_schemas();
    let world_types = world_schema();
    let instances = instance_schema();
    let mut ledger = Ledger::default();
    let mut cases = Vec::new();
    let mut current: Option<MaintenanceCase> = None;
    let mut names = BTreeSet::new();
    for line in source.lines() {
        ledger.rows += 1;
        let row: Vec<_> = line.split('\t').collect();
        match row[0] {
            "C" => {
                assert_eq!(row.len(), 4);
                assert!(current.is_none());
                assert_eq!(row[2], "canonical");
                assert!(names.insert(row[1].to_owned()));
                current = Some(MaintenanceCase {
                    name: row[1].into(),
                    seed: row[3].parse().unwrap(),
                    settings: Fields::new(),
                    input_genomes: BTreeMap::new(),
                    snapshots: BTreeMap::new(),
                    steps: Vec::new(),
                });
                ledger.cases += 1;
                ledger.inputs_and_commands += 1;
            }
            "U" => {
                assert_eq!(row.len(), 4);
                assert_eq!(setting_types[row[1]], row[2]);
                validate(row[2], row[3]);
                assert!(
                    current
                        .as_mut()
                        .unwrap()
                        .settings
                        .insert(row[1].into(), row[3].into())
                        .is_none()
                );
                ledger.inputs_and_commands += 1;
            }
            "I" => {
                assert_eq!(row.len(), 5);
                assert!(row[1] == "setup" || row[1].starts_with("step-"));
                let parts: Vec<_> = row[2].split(':').collect();
                match parts.as_slice() {
                    ["visible", "selected"] => {
                        assert_eq!((row[3], row[4]), ("String", "selector"));
                        ledger.selection_metadata += 1;
                    }
                    ["genome", label, "construction"] => {
                        reference_label(label);
                        assert_eq!(row[3], "String");
                        assert!(row[4] == "explicit" || row[4].starts_with("clone:g"));
                        ledger.inputs_and_commands += 1;
                    }
                    ["actor", _, field] => {
                        assert_eq!(organism_types[*field], row[3]);
                        if row[3] == "GeneticCode" {
                            reference_label(row[4]);
                        } else {
                            validate(row[3], row[4]);
                        }
                        ledger.inputs_and_commands += 1;
                    }
                    ["world", field] => {
                        assert_eq!(instances[*field], row[3]);
                        validate(row[3], row[4]);
                        ledger.inputs_and_commands += 1;
                    }
                    ["rng", "seed"] => {
                        assert_eq!(row[3], "long");
                        validate(row[3], row[4]);
                        ledger.inputs_and_commands += 1;
                    }
                    target => panic!("Unknown maintenance input target {target:?}"),
                }
                current.as_mut().unwrap().steps.push(Step::Input {
                    target: row[2].into(),
                    kind: row[3].into(),
                    value: row[4].into(),
                });
            }
            "A" => {
                assert_eq!(row.len(), 5);
                reference_label(row[2]);
                current.as_mut().unwrap().steps.push(Step::Actor {
                    role: row[1].into(),
                    genome: row[2].into(),
                    center: [row[3].parse().unwrap(), row[4].parse().unwrap()],
                });
                ledger.inputs_and_commands += 1;
            }
            "K" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[3], "victim");
                match row[2] {
                    "breath" | "decay" | "hatch" | "die" | "symmetric_bounds" | "blueSpore"
                    | "cyanSpore" => assert_eq!(row[4], "~"),
                    "useDecayEnergy" => validate("double", row[4]),
                    "build_tree" => {
                        role_list(row[4]);
                    }
                    operation => panic!("Unknown maintenance operation {operation}"),
                }
                current.as_mut().unwrap().steps.push(Step::Call {
                    stage: row[1].into(),
                    operation: row[2].into(),
                    role: row[3].into(),
                    argument: row[4].into(),
                });
                *ledger.calls.entry(row[2].into()).or_default() += 1;
                ledger.inputs_and_commands += 1;
            }
            "D" => {
                assert_eq!(row.len(), 5);
                reference_label(row[4]);
                current.as_mut().unwrap().steps.push(Step::Discover {
                    stage: row[1].into(),
                    role: row[2].into(),
                    id: row[3].parse().unwrap(),
                    genome: row[4].into(),
                });
                ledger.newborns += 1;
                ledger.observations += 1;
            }
            "P" => {
                assert_eq!(row.len(), 2);
                let case = current.as_mut().unwrap();
                case.snapshots.entry(row[1].into()).or_default();
                assert!(
                    !case
                        .steps
                        .iter()
                        .any(|step| matches!(step,Step::Snapshot(name) if name==row[1]))
                );
                case.steps.push(Step::Snapshot(row[1].into()));
                ledger.snapshots += 1;
                ledger.inputs_and_commands += 1;
            }
            "O" => {
                assert_eq!(row.len(), 6);
                assert_eq!(organism_types[row[3]], row[4]);
                validate(row[4], row[5]);
                assert!(
                    current
                        .as_mut()
                        .unwrap()
                        .snapshots
                        .entry(row[1].into())
                        .or_default()
                        .organisms
                        .entry(row[2].into())
                        .or_default()
                        .insert(row[3].into(), row[5].into())
                        .is_none()
                );
                if row[3] == "_ID" {
                    ledger.actor_views += 1;
                }
                if ["_world", "_visibleWorld"].contains(&row[3]) {
                    assert_eq!(row[5], "present");
                    ledger.organism_collaborators += 1;
                } else {
                    ledger.observations += 1;
                }
            }
            "H" => {
                assert_eq!(row.len(), 5);
                assert!(["genotype", "infected", "saved"].contains(&row[3]));
                reference_label(row[4]);
                assert!(
                    current
                        .as_mut()
                        .unwrap()
                        .snapshots
                        .entry(row[1].into())
                        .or_default()
                        .references
                        .insert((row[2].into(), row[3].into()), row[4].into())
                        .is_none()
                );
                ledger.observations += 1;
            }
            "W" | "V" => {
                assert_eq!(row.len(), 5);
                let snapshot = current
                    .as_mut()
                    .unwrap()
                    .snapshots
                    .entry(row[1].into())
                    .or_default();
                if row[0] == "W" {
                    assert_eq!(world_types[row[2]], row[3]);
                    validate(row[3], row[4]);
                    assert!(
                        snapshot
                            .world
                            .insert(row[2].into(), row[4].into())
                            .is_none()
                    );
                    ledger.observations += 1;
                } else {
                    assert_eq!(instances[row[2]], row[3]);
                    match row[2] {
                        "_organisms" => {
                            role_list(row[4]);
                        }
                        "inCorridors" | "outCorridors" => assert!(row[4].is_empty()),
                        "_visibleWorld" | "worldStatistics" => assert_eq!(row[4], "present"),
                        "colDetTree" => {
                            let (presence, size) = row[4].split_once(',').unwrap();
                            assert_eq!(presence, "present");
                            size.parse::<usize>().unwrap();
                        }
                        _ => validate(row[3], row[4]),
                    }
                    assert!(
                        snapshot
                            .world_instances
                            .insert(row[2].into(), row[4].into())
                            .is_none()
                    );
                    if WORLD_EXCLUSIONS.contains(&row[2]) {
                        ledger.world_exclusions += 1;
                    } else {
                        ledger.observations += 1;
                    }
                }
            }
            "S" | "T" => {
                assert_eq!(row.len(), 5);
                let schema = if row[0] == "S" {
                    &statistic_types
                } else {
                    &getter_types
                };
                assert_eq!(schema[row[2]], row[3]);
                if row[3] == "GeneticCode" {
                    reference_label(row[4]);
                } else {
                    validate(row[3], row[4]);
                }
                let snapshot = current
                    .as_mut()
                    .unwrap()
                    .snapshots
                    .entry(row[1].into())
                    .or_default();
                let target = if row[0] == "S" {
                    &mut snapshot.statistics
                } else {
                    &mut snapshot.getters
                };
                assert!(
                    target
                        .insert(row[2].into(), (row[3].into(), row[4].into()))
                        .is_none()
                );
                ledger.observations += 1;
            }
            "G" | "N" => {
                let case = current.as_mut().unwrap();
                assert_eq!(row.len(), if row[0] == "G" { 5 } else { 6 });
                reference_label(row[2]);
                let target = if row[1] == "input" {
                    ledger.input_genotypes += 1;
                    &mut case.input_genomes
                } else {
                    ledger.observations += 1;
                    &mut case.snapshots.entry(row[1].into()).or_default().genomes
                };
                let code = target.entry(row[2].into()).or_default();
                if row[0] == "G" {
                    assert!(code.genome.insert(row[3].into(), row[4].into()).is_none());
                } else {
                    assert!(
                        code.genes
                            .entry(row[3].parse().unwrap())
                            .or_default()
                            .insert(row[4].into(), row[5].into())
                            .is_none()
                    );
                }
            }
            "R" => {
                assert_eq!(row.len(), 5);
                validate("long", row[2]);
                validate("int", row[3]);
                validate("double", row[4]);
                assert!(
                    current
                        .as_mut()
                        .unwrap()
                        .snapshots
                        .entry(row[1].into())
                        .or_default()
                        .suffix
                        .replace((
                            row[2].parse().unwrap(),
                            row[3].parse().unwrap(),
                            u64::from_str_radix(row[4], 16).unwrap()
                        ))
                        .is_none()
                );
                ledger.observations += 1;
            }
            "J" => {
                assert_eq!(row.len(), 7);
                let values: Vec<i32> = row[3..]
                    .iter()
                    .map(|value| value.parse().unwrap())
                    .collect();
                current
                    .as_mut()
                    .unwrap()
                    .snapshots
                    .entry(row[1].into())
                    .or_default()
                    .insertions
                    .push((
                        row[2].into(),
                        IntRect {
                            x: values[0],
                            y: values[1],
                            width: values[2],
                            height: values[3],
                        },
                    ));
                ledger.observations += 1;
            }
            "Q" => {
                assert_eq!(row.len(), 4);
                role_list(row[3]);
                assert!(
                    current
                        .as_mut()
                        .unwrap()
                        .snapshots
                        .entry(row[1].into())
                        .or_default()
                        .queries
                        .insert(row[2].into(), row[3].into())
                        .is_none()
                );
                ledger.observations += 1;
            }
            "B" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[2], "useDecayEnergy");
                assert!(
                    current
                        .as_mut()
                        .unwrap()
                        .snapshots
                        .entry(row[1].into())
                        .or_default()
                        .booleans
                        .insert(row[2].into(), row[3].parse().unwrap())
                        .is_none()
                );
                ledger.observations += 1;
            }
            "E" => {
                assert_eq!(row.len(), 1);
                cases.push(current.take().unwrap());
                ledger.inputs_and_commands += 1;
            }
            tag => panic!("Unknown maintenance row type {tag}"),
        }
    }
    assert!(current.is_none());
    assert_eq!(ledger, expected_ledger(), "Frozen pre-replay source ledger");
    assert_eq!(
        ledger.rows,
        ledger.observations
            + ledger.input_genotypes
            + ledger.organism_collaborators
            + ledger.world_exclusions
            + ledger.selection_metadata
            + ledger.inputs_and_commands
    );
    let mut actor_views = 0;
    for case in &cases {
        validate_case(
            case,
            &setting_types,
            &organism_types,
            &statistic_types,
            &getter_types,
        );
        actor_views += case
            .snapshots
            .values()
            .map(|snapshot| snapshot.organisms.len())
            .sum::<usize>();
    }
    assert_eq!(ledger.actor_views, actor_views);
    assert_eq!(ledger, expected_ledger());
    (cases, ledger)
}

fn validate_case(
    case: &MaintenanceCase,
    settings: &BTreeMap<String, String>,
    organisms: &BTreeMap<String, String>,
    statistics: &BTreeMap<String, String>,
    getters: &BTreeMap<String, String>,
) {
    assert_eq!(
        case.settings.keys().collect::<BTreeSet<_>>(),
        settings.keys().collect()
    );
    assert_eq!(
        case.input_genomes
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["g0", "g1", "g2", "g3", "g4"]
    );
    let sample = explicit_genome(&case.input_genomes["g0"]);
    let genome_schema: BTreeSet<_> = genome_fields(&sample)
        .keys()
        .cloned()
        .chain(["_reference".into(), "_nGenes".into()])
        .collect();
    let gene_schema = gene_fields(&sample.gene_ref(0).unwrap().borrow()).keys().cloned().collect();
    for input in case.input_genomes.values() {
        validate_genome(input, &gene_schema);
    }
    let mut roles = BTreeSet::new();
    let mut observed_stages = BTreeSet::new();
    let mut calls = BTreeMap::new();
    let mut recipes = BTreeSet::new();
    for step in &case.steps {
        match step {
            Step::Actor { role, genome, .. } => {
                assert!(case.input_genomes.contains_key(genome));
                assert!(roles.insert(role.clone()));
            }
            Step::Discover { role, genome, .. } => {
                reference_label(genome);
                assert!(role.starts_with("newborn"));
                assert!(roles.insert(role.clone()));
            }
            Step::Input { target, .. } => {
                if let Some(label) = target
                    .strip_prefix("genome:")
                    .and_then(|value| value.strip_suffix(":construction"))
                {
                    assert!(recipes.insert(label));
                }
            }
            Step::Call {
                stage, operation, ..
            } => {
                assert!(calls.insert(stage.clone(), operation.as_str()).is_none());
            }
            Step::Snapshot(name) => {
                assert!(observed_stages.insert(name.clone()));
                let snapshot = &case.snapshots[name];
                assert_eq!(
                    snapshot.organisms.keys().cloned().collect::<BTreeSet<_>>(),
                    roles
                );
                for actor in snapshot.organisms.values() {
                    assert_eq!(
                        actor.keys().collect::<BTreeSet<_>>(),
                        organisms.keys().collect()
                    );
                }
                let expected_refs: BTreeSet<_> = roles
                    .iter()
                    .flat_map(|role| {
                        ["genotype", "infected", "saved"]
                            .into_iter()
                            .map(move |holder| (role.clone(), holder.to_owned()))
                    })
                    .collect();
                assert_eq!(
                    snapshot.references.keys().cloned().collect::<BTreeSet<_>>(),
                    expected_refs
                );
                assert_eq!(
                    snapshot
                        .world
                        .keys()
                        .map(String::as_str)
                        .collect::<BTreeSet<_>>(),
                    world_schema().keys().copied().collect()
                );
                assert_eq!(
                    snapshot
                        .world_instances
                        .keys()
                        .map(String::as_str)
                        .collect::<BTreeSet<_>>(),
                    instance_schema().keys().copied().collect()
                );
                assert_eq!(
                    snapshot.statistics.keys().collect::<BTreeSet<_>>(),
                    statistics.keys().collect()
                );
                assert_eq!(
                    snapshot.getters.keys().collect::<BTreeSet<_>>(),
                    getters.keys().collect()
                );
                assert_eq!(
                    snapshot.queries.keys().cloned().collect::<BTreeSet<_>>(),
                    roles
                );
                validate_actor_roles(snapshot, &roles);
                for (role, _) in &snapshot.insertions {
                    assert!(roles.contains(role), "Unknown source insertion role {role}");
                }
                assert!(snapshot.suffix.is_some());
                if name == "initial" {
                    assert!(snapshot.booleans.is_empty() && snapshot.insertions.is_empty());
                } else {
                    let operation = calls[name];
                    assert_eq!(
                        snapshot.booleans.len(),
                        usize::from(operation == "useDecayEnergy")
                    );
                    if operation != "build_tree" {
                        assert!(snapshot.insertions.is_empty());
                    }
                }
                for (label, genome) in &snapshot.genomes {
                    reference_label(label);
                    assert_eq!(genome.genome["_reference"], *label);
                    assert_eq!(
                        genome.genome.keys().cloned().collect::<BTreeSet<_>>(),
                        genome_schema
                    );
                    let count = integer(&genome.genome, "_nGenes");
                    assert!(count >= 0);
                    assert_eq!(genome.genes.len(), count as usize);
                    for (index, (position, gene)) in genome.genes.iter().enumerate() {
                        assert_eq!(index, *position);
                        assert_eq!(gene.keys().cloned().collect::<BTreeSet<_>>(), gene_schema);
                        for (field, value) in gene {
                            validate(
                                match field.as_str() {
                                    "_length" | "_theta" => "double",
                                    "_color" => "Color",
                                    _ => "int",
                                },
                                value,
                            );
                        }
                    }
                }
            }
        }
    }
    assert_eq!(
        recipes,
        case.input_genomes.keys().map(String::as_str).collect()
    );
    assert_eq!(observed_stages, case.snapshots.keys().cloned().collect());
    assert_eq!(case.snapshots.len(), calls.len() + 1);
    assert!(case.snapshots.contains_key("initial"));
}

fn validate_actor_roles(snapshot: &Snapshot, roles: &BTreeSet<String>) {
    let members = role_list(&snapshot.world_instances["_organisms"]);
    for role in &members {
        assert!(roles.contains(*role), "Unknown source member role {role}");
    }
    assert_eq!(members.len(), roles.len());
    assert_eq!(
        members
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>(),
        *roles
    );
    for hits in snapshot.queries.values() {
        for role in role_list(hits) {
            assert!(roles.contains(role), "Unknown source query role {role}");
        }
    }
}

pub(super) fn bind_reference(
    expected: &str,
    actual: Option<&GenomeRef>,
    genomes: &mut BTreeMap<String, GenomeRef>,
    context: &str,
) {
    match (expected, actual) {
        ("~", None) => {}
        (label, Some(actual)) if label != "~" => {
            if let Some(previous) = genomes.get(label) {
                assert!(
                    Rc::ptr_eq(actual, previous),
                    "{context}: genotype {label} changed identity"
                );
            } else {
                assert!(
                    !genomes
                        .values()
                        .any(|previous| Rc::ptr_eq(actual, previous)),
                    "{context}: distinct genotype labels incorrectly alias"
                );
                genomes.insert(label.to_owned(), Rc::clone(actual));
            }
        }
        _ => panic!("{context}: genotype presence differs from {expected}"),
    }
}

pub(super) fn actor_role<'a>(
    reference: &OrganismRef,
    actors: &'a BTreeMap<String, OrganismRef>,
) -> &'a str {
    actors
        .iter()
        .find_map(|(role, actor)| Rc::ptr_eq(reference, actor).then_some(role.as_str()))
        .expect("Every observed member needs its actual object identity")
}

pub(super) fn world_views(
    world: &World,
    actors: &BTreeMap<String, OrganismRef>,
) -> (Fields, Fields) {
    let getters = world_schema()
        .into_keys()
        .map(|field| {
            let value = match field {
                "_population" => world.population().to_string(),
                "organism_count" => world.organisms.borrow().len().to_string(),
                "corpses" => world.corpse_count().unwrap().to_string(),
                "_O2" => bits(world.oxygen.get()),
                "_CO2" => bits(world.carbon_dioxide.get()),
                "_CH4" => bits(world.methane.get()),
                "NEXT_ID" => world.next_id.get().to_string(),
                "nFrames" => world.frame.get().to_string(),
                "time" => world.time().to_string(),
                "organism_ids" => array(world.organisms.borrow().iter().map(|actor| actor.borrow().id)),
                _ => panic!("Unmapped maintenance World getter {field}"),
            };
            (field.to_owned(), value)
        })
        .collect();
    let instances = instance_schema()
        .into_keys()
        .filter(|field| !WORLD_EXCLUSIONS.contains(field))
        .map(|field| {
            let value = match field {
                "_width" => world.config.get().width.to_string(),
                "_height" => world.config.get().height.to_string(),
                "NEXT_ID" => world.next_id.get().to_string(),
                "nFrames" => world.frame.get().to_string(),
                "_population" => world.population().to_string(),
                "_O2" => bits(world.oxygen.get()),
                "_CO2" => bits(world.carbon_dioxide.get()),
                "_CH4" => bits(world.methane.get()),
                "_organisms" => array(
                    world.organisms.borrow()
                        .iter()
                        .map(|actor| actor_role(actor, actors)),
                ),
                "worldStatistics" => presence(Some(&world.statistics_ref().borrow())),
                "colDetTree" => format!(
                    "{},{}",
                    presence(Some(&world.spatial.borrow())),
                    world.frame_objects.borrow().len()
                ),
                _ => panic!("Unmapped supported World instance field {field}"),
            };
            (field.to_owned(), value)
        })
        .collect();
    (getters, instances)
}

pub(super) fn query_roles(
    world: &World,
    bounds: IntRect,
    actors: &BTreeMap<String, OrganismRef>,
) -> String {
    array(
        world.spatial.borrow()
            .query(bounds)
            .into_iter()
            .map(|token| actor_role(&world.frame_objects.borrow()[token], actors)),
    )
}

fn apply_input(
    case: &MaintenanceCase,
    target: &str,
    kind: &str,
    value: &str,
    world: &mut World,
    actors: &BTreeMap<String, OrganismRef>,
    genomes: &mut BTreeMap<String, GenomeRef>,
    errors: &mut Vec<String>,
) {
    let parts: Vec<_> = target.split(':').collect();
    match parts.as_slice() {
        ["rng", "seed"] => {
            assert_eq!(kind, "long");
            *world.session.0.random.borrow_mut() = JavaRandom::new(value.parse().unwrap());
        }
        ["world", "_O2"] => {
            assert_eq!(kind, "double");
            world.oxygen.set(decoded(value));
        }
        ["world", "_CO2"] => {
            assert_eq!(kind, "double");
            world.carbon_dioxide.set(decoded(value));
        }
        ["world", "_CH4"] => {
            assert_eq!(kind, "double");
            world.methane.set(decoded(value));
        }
        ["world", "NEXT_ID"] => {
            assert_eq!(kind, "int");
            world.next_id.set(value.parse().unwrap());
        }
        ["actor", role, "_growthRatio"] => {
            assert_eq!(kind, "int");
            actors[*role].borrow_mut().body.growth_ratio =
                value.parse::<i32>().unwrap();
        }
        _ => apply_reference_input(
            &case.name,
            &case.input_genomes,
            target,
            kind,
            value,
            world,
            actors,
            genomes,
            errors,
        ),
    }
}

fn call(
    world: &mut World,
    actors: &BTreeMap<String, OrganismRef>,
    operation: &str,
    role: &str,
    argument: &str,
) -> Result<(Option<bool>, Vec<(String, IntRect)>), WorldError> {
    if operation == "build_tree" {
        *world.frame_objects.borrow_mut() = role_list(argument)
            .iter()
            .map(|role| Rc::clone(&actors[*role]))
            .collect();
        let inserted: Vec<_> = world.frame_objects.borrow()
            .iter()
            .map(|actor| {
                (
                    actor_role(actor, actors).to_owned(),
                    actor.borrow().state.bounds.get(),
                )
            })
            .collect();
        *world.spatial.borrow_mut() = SpatialTree::build_actor_entries(
            inserted
                .iter()
                .enumerate()
                .map(|(token, (_, bounds))| (token, *bounds)),
        |token| Rc::clone(&world.frame_objects.borrow()[token]));
        return Ok((None, inserted));
    }
    let reference = &actors[role];
    let mut actor = reference.borrow_mut();
    let result = match operation {
        "breath" => {
            world.breath_from(reference, &mut actor, None)?;
            None
        }
        "decay" => {
            world.decay_from(reference, &mut actor, None)?;
            None
        }
        "hatch" => {
            world.hatch(&mut actor)?;
            None
        }
        "die" => {
            world.die(&mut actor, None)?;
            None
        }
        "useDecayEnergy" => Some(world.use_decay_energy(&mut actor, decoded(argument))?),
        "blueSpore" => {
            crate::birth_spore::blue_spore(&mut actor)?;
            None
        }
        "cyanSpore" => {
            crate::birth_spore::cyan_spore(&mut actor)?;
            None
        }
        "symmetric_bounds" => {
            actor.rebuild_body()?;
            crate::movement::calculate_bounds(&mut actor, true)?;
            None
        }
        _ => panic!("Unmapped maintenance operation {operation}"),
    };
    Ok((result, Vec::new()))
}

fn compare_snapshot(
    case: &MaintenanceCase,
    name: &str,
    world: &World,
    actors: &BTreeMap<String, OrganismRef>,
    genomes: &mut BTreeMap<String, GenomeRef>,
    results: &mut BTreeMap<String, BTreeMap<String, bool>>,
    insertions: &mut BTreeMap<String, Vec<(String, IntRect)>>,
    errors: &mut Vec<String>,
) {
    let expected = &case.snapshots[name];
    assert_eq!(
        actors.keys().collect::<BTreeSet<_>>(),
        expected.organisms.keys().collect()
    );
    for ((role, holder), label) in &expected.references {
        let actor = actors[role].borrow();
        let actual = match holder.as_str() {
            "genotype" => actor.genome.as_ref(),
            "infected" => actor.infected_genome.as_ref(),
            "saved" => actor.saved_genome.as_ref(),
            _ => unreachable!(),
        };
        bind_reference(
            label,
            actual,
            genomes,
            &format!("{} {name} {role}:{holder}", case.name),
        );
    }
    for (role, target) in &expected.organisms {
        let actor = actors[role].borrow();
        let mut actual = organism_fields(&View::from(&*actor));
        let mut target = target.clone();
        for collaborator in ["_world", "_visibleWorld"] {
            // World ownership and GUI references are named exclusions, never simulated pointer values
            assert_eq!(target.remove(collaborator).as_deref(), Some("present"));
            assert!(actual.remove(collaborator).is_some());
        }
        compare_fields(
            &case.name,
            &format!("{name}:{role}"),
            "O",
            &actual,
            &target,
            errors,
        );
    }
    let (actual_world, actual_instances) = world_views(world, actors);
    compare_fields(
        &case.name,
        name,
        "W",
        &actual_world,
        &expected.world,
        errors,
    );
    let target_instances: Fields = expected
        .world_instances
        .iter()
        .filter(|(field, _)| !WORLD_EXCLUSIONS.contains(&field.as_str()))
        .map(|(field, value)| (field.clone(), value.clone()))
        .collect();
    compare_fields(
        &case.name,
        name,
        "V",
        &actual_instances,
        &target_instances,
        errors,
    );
    for (role, query) in &expected.queries {
        let bounds = actors[role].borrow().state.bounds.get();
        let actual = query_roles(world, bounds, actors);
        assert_eq!(actual, *query, "{} {name} tree query for {role}", case.name);
    }
    assert_eq!(
        insertions.remove(name).unwrap_or_default(),
        expected.insertions,
        "{} {name} actual inserted envelopes",
        case.name
    );
    assert_eq!(
        results.remove(name).unwrap_or_default(),
        expected.booleans,
        "{} {name} actual method returns",
        case.name
    );
    for (field, (kind, value)) in &expected.statistics {
        assert_observation(
            observed_field(&world.statistics_ref().borrow(), field),
            kind,
            value,
            genomes,
            &format!("{} {name} statistics {field}", case.name),
        );
    }
    for (getter, (kind, value)) in &expected.getters {
        assert_observation(
            observed_getter(&world.statistics_ref().borrow(), getter),
            kind,
            value,
            genomes,
            &format!("{} {name} getter {getter}", case.name),
        );
    }
    compare_genomes(&case.name, name, &expected.genomes, genomes, errors);
    let mut random = world.random_snapshot();
    let suffix = (
        random.next_i64(),
        random.next_bounded(10000).unwrap(),
        random.next_f64().to_bits(),
    );
    assert_eq!(
        suffix,
        expected.suffix.unwrap(),
        "{} {name} exact RNG suffix",
        case.name
    );
}

fn replay(case: &MaintenanceCase, errors: &mut Vec<String>) {
    assert_eq!(integer(&case.settings, "INITIAL_ORGANISMS"), 0);
    let mut world = World::with_parameters(
        WorldConfig {
            seed: case.seed,
            width: integer(&case.settings, "WORLD_WIDTH").try_into().unwrap(),
            height: integer(&case.settings, "WORLD_HEIGHT").try_into().unwrap(),
            initial_attempts: 0,
        },
        parameters(&case.settings),
    )
    .unwrap();
    let mut actors = BTreeMap::new();
    let mut genomes = BTreeMap::new();
    let mut results: BTreeMap<String, BTreeMap<String, bool>> = BTreeMap::new();
    let mut insertions = BTreeMap::new();
    let mut latest_call = None;
    for step in &case.steps {
        match step {
            Step::Input {
                target,
                kind,
                value,
            } => apply_input(
                case,
                target,
                kind,
                value,
                &mut world,
                &actors,
                &mut genomes,
                errors,
            ),
            Step::Actor {
                role,
                genome,
                center,
            } => {
                let actor = world
                    .construct_pasted_shared(Rc::clone(&genomes[genome]), *center)
                    .unwrap()
                    .unwrap();
                let actor = Rc::new(RefCell::new(actor));
                world.add_organism(Rc::clone(&actor), None);
                assert!(actors.insert(role.clone(), actor).is_none());
            }
            Step::Call {
                stage,
                operation,
                role,
                argument,
            } => {
                let (result, inserted) = call(&mut world, &actors, operation, role, argument).unwrap();
                if let Some(result) = result {
                    assert!(
                        results
                            .entry(stage.clone())
                            .or_default()
                            .insert(operation.clone(), result)
                            .is_none()
                    );
                }
                if operation == "build_tree" {
                    assert!(insertions.insert(stage.clone(), inserted).is_none());
                }
                latest_call = Some(stage.as_str());
            }
            Step::Discover {
                stage,
                role,
                id,
                genome,
            } => {
                assert_eq!(latest_call, Some(stage.as_str()));
                assert_eq!(*role, format!("newborn{}", actors.len() - 3));
                // Discover new storage in live member order; legacy identifiers can wrap or duplicate
                let actual = world.organisms.borrow()
                    .iter()
                    .find(|member| !actors.values().any(|known| Rc::ptr_eq(member, known)))
                    .cloned()
                    .unwrap_or_else(|| {
                        panic!("{} {stage}: missing actual new member {role}", case.name)
                    });
                assert_eq!(
                    actual.borrow().id,
                    i64::from(*id),
                    "{} {stage}: actual new member ID",
                    case.name
                );
                bind_reference(
                    genome,
                    actual.borrow().genome.as_ref(),
                    &mut genomes,
                    &format!("{} {stage} {role}", case.name),
                );
                assert!(actors.insert(role.clone(), actual).is_none());
            }
            Step::Snapshot(name) => {
                assert!(name == "initial" || latest_call == Some(name.as_str()));
                for member in world.organisms.borrow().clone() {
                    actor_role(&member, &actors);
                }
                compare_snapshot(
                    case,
                    name,
                    &world,
                    &actors,
                    &mut genomes,
                    &mut results,
                    &mut insertions,
                    errors,
                );
            }
        }
    }
    assert!(results.is_empty() && insertions.is_empty());
}

#[test]
fn actual_java_maintenance_preserves_fields_aliases_resources_statistics_and_rng() {
    let (cases, ledger) = parse(include_str!("../tests/fixtures/maintenance.tsv"));
    assert_eq!(ledger, expected_ledger());
    let mut errors = Vec::new();
    for case in &cases {
        replay(case, &mut errors);
    }
    assert!(
        errors.is_empty(),
        "Maintenance oracle mismatches:\n{}",
        errors.join("\n")
    );
}

#[test]
fn rectangle_tuples_cannot_replace_source_actor_identity_labels() {
    for (members, hits) in [("victim", "0,0,1,1"), ("0,0,1,1", "")] {
        let snapshot = Snapshot {
            world_instances: BTreeMap::from([("_organisms".to_owned(), members.to_owned())]),
            queries: BTreeMap::from([("victim".to_owned(), hits.to_owned())]),
            ..Snapshot::default()
        };
        let roles = BTreeSet::from(["victim".to_owned()]);
        assert!(std::panic::catch_unwind(|| validate_actor_roles(&snapshot, &roles)).is_err());
    }
}
