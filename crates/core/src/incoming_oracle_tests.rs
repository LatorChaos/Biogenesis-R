// SPDX-License-Identifier: GPL-2.0-or-later

//! Compare actual queue and frame behavior against three separately frozen source corpora

use super::death_oracle_tests::{
    apply_reference_input, compare_genomes, statistics_schemas, validate, validate_genome,
    world_schema,
};
use super::maintenance_oracle_tests::{
    actor_role, bind_reference, instance_schema, query_roles, reference_label, role_list,
    world_views,
};
use super::*;
use crate::organism::{GenomeRef, OrganismRef};
use crate::statistics::tests::{Observation, assert_observation, observed_field, observed_getter};
use crate::world::incoming::IncomingCorridorRef;
use std::rc::Rc;

const CORRIDOR_SCHEMA: [(&str, &str); 7] = [
    ("_reference", "String"),
    ("bounds", "Rectangle"),
    ("pendingOrganisms", "Queue"),
    ("nextOrganism", "Organism"),
    ("world", "World"),
    ("travellingOrganism", "Organism"),
    ("connection", "Connection"),
];
const OMITTED_WORLD: [&str; 2] = ["_visibleWorld", "outCorridors"];
const OMITTED_CORRIDOR: [&str; 2] = ["travellingOrganism", "connection"];
const NET015_CASE: &str = "zero-span-corridor-constructor-rejects-before-candidate-creation";

#[derive(Default)]
struct Snapshot {
    organisms: BTreeMap<String, Fields>,
    references: BTreeMap<(String, String), String>,
    corridors: BTreeMap<String, Fields>,
    world: Fields,
    instances: Fields,
    statistics: BTreeMap<String, (String, String)>,
    getters: BTreeMap<String, (String, String)>,
    genotypes: BTreeMap<String, Stage>,
    scopes: Fields,
    queries: Fields,
    insertions: BTreeMap<String, IntRect>,
    returned: Option<bool>,
    suffix: Option<(i64, i32, u64)>,
    exception: Option<(String, String)>,
    roles: BTreeSet<String>,
    corridor_roles: BTreeSet<String>,
}

enum Step {
    Input {
        stage: String,
        target: String,
        kind: String,
        value: String,
    },
    Actor {
        role: String,
        genotype: String,
        center: [i32; 2],
    },
    Call {
        stage: String,
        operation: String,
        target: String,
        argument: String,
    },
    Discover {
        stage: String,
        role: String,
        id: i32,
        genotype: String,
    },
    Snapshot(String),
}

struct Case {
    name: String,
    mode: String,
    seed: i64,
    settings: Fields,
    input_genotypes: BTreeMap<String, Stage>,
    snapshots: BTreeMap<String, Snapshot>,
    steps: Vec<Step>,
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Ledger {
    rows: usize,
    tags: BTreeMap<String, usize>,
    eligible: usize,
    input_genotypes: usize,
    collaborators: usize,
    other: usize,
    calls: BTreeMap<String, usize>,
    targets: BTreeMap<(String, String), usize>,
}

fn rectangle_input(value: &str) -> IntRect {
    let values: Vec<i32> = value.split(',').map(|part| part.parse().unwrap()).collect();
    assert_eq!(values.len(), 4);
    IntRect {
        x: values[0],
        y: values[1],
        width: values[2],
        height: values[3],
    }
}

fn known_role(role: &str) {
    if !["selector", "blocker", "victim"].contains(&role) {
        role.strip_prefix("arrival")
            .unwrap()
            .parse::<usize>()
            .unwrap();
    }
}

fn corridor_role(role: &str) {
    role.strip_prefix('c').unwrap().parse::<usize>().unwrap();
}

fn insert_unique(fields: &mut Fields, key: &str, value: &str) {
    assert!(
        fields.insert(key.to_owned(), value.to_owned()).is_none(),
        "Duplicate field {key}"
    );
}

fn expected_ledger(family: &str) -> Ledger {
    let (rows, eligible, input_genotypes, collaborators, other, tags, calls) = match family {
        "ordinary" => (
            46_662,
            41_295,
            3_140,
            396,
            1_831,
            vec![
                ("C", 9),
                ("U", 1323),
                ("I", 101),
                ("G", 8820),
                ("N", 13640),
                ("A", 10),
                ("P", 54),
                ("O", 14784),
                ("H", 288),
                ("W", 540),
                ("V", 810),
                ("Q", 96),
                ("S", 2808),
                ("T", 2646),
                ("Y", 270),
                ("R", 54),
                ("K", 45),
                ("B", 1),
                ("E", 9),
                ("F", 336),
                ("D", 10),
                ("J", 8),
            ],
            vec![
                ("construct", 1),
                ("paste", 1),
                ("corridor.new", 9),
                ("corridor.frame", 20),
                ("corridor.receive", 11),
                ("time", 3),
            ],
        ),
        "diagnostic" => (
            3_833,
            2_866,
            600,
            20,
            347,
            vec![
                ("C", 2),
                ("U", 294),
                ("I", 14),
                ("G", 980),
                ("N", 1080),
                ("A", 1),
                ("P", 5),
                ("O", 770),
                ("H", 15),
                ("W", 50),
                ("V", 75),
                ("Q", 5),
                ("S", 260),
                ("T", 245),
                ("Y", 25),
                ("R", 5),
                ("K", 3),
                ("B", 1),
                ("E", 2),
                ("X", 1),
            ],
            vec![("construct", 1), ("paste", 1), ("corridor.new", 1)],
        ),
        "empty" => (
            10_720,
            10_081,
            260,
            112,
            267,
            vec![
                ("C", 1),
                ("U", 147),
                ("I", 12),
                ("G", 2240),
                ("N", 1920),
                ("A", 1),
                ("P", 15),
                ("O", 4158),
                ("H", 81),
                ("W", 150),
                ("V", 225),
                ("Q", 27),
                ("S", 780),
                ("T", 735),
                ("Y", 75),
                ("R", 15),
                ("K", 14),
                ("F", 98),
                ("D", 1),
                ("J", 24),
                ("E", 1),
            ],
            vec![("corridor.new", 1), ("corridor.receive", 1), ("time", 12)],
        ),
        _ => panic!("Unknown incoming family {family}"),
    };
    let targets = match family {
        "ordinary" => vec![
            ("actor:arrival0:bounds", "Rectangle", 2),
            ("actor:arrival1:bounds", "Rectangle", 1),
            ("actor:blocker:bounds", "Rectangle", 2),
            ("corridor:c0:bounds", "Rectangle", 9),
            ("corridor:c1:bounds", "Rectangle", 1),
            ("genome:g0:gene0._branch", "int", 1),
            ("visible:selected", "String", 9),
            ("world:_CH4", "double", 9),
            ("world:_CO2", "double", 9),
            ("world:_O2", "double", 9),
            ("world:_corridorexists", "boolean", 2),
            ("world:inCorridors", "List", 2),
        ],
        "diagnostic" => vec![
            ("visible:selected", "String", 1),
            ("world:_CH4", "double", 1),
            ("world:_CO2", "double", 1),
            ("world:_O2", "double", 1),
        ],
        "empty" => vec![
            ("corridor:c0:bounds", "Rectangle", 1),
            ("visible:selected", "String", 1),
            ("world:_CH4", "double", 1),
            ("world:_CO2", "double", 1),
            ("world:_O2", "double", 1),
            ("world:_corridorexists", "boolean", 1),
            ("world:inCorridors", "List", 1),
        ],
        _ => unreachable!(),
    };
    let mut targets: BTreeMap<_, _> = targets
        .into_iter()
        .map(|(target, kind, count)| ((target.to_owned(), kind.to_owned()), count))
        .collect();
    let cases = match family {
        "ordinary" => 9,
        "diagnostic" => 2,
        "empty" => 1,
        _ => unreachable!(),
    };
    for index in 0..5 {
        targets.insert(
            (format!("genome:g{index}:construction"), "String".into()),
            cases,
        );
    }
    Ledger {
        rows,
        eligible,
        input_genotypes,
        collaborators,
        other,
        targets,
        tags: tags
            .into_iter()
            .map(|(tag, count)| (tag.to_owned(), count))
            .collect(),
        calls: calls
            .into_iter()
            .map(|(op, count)| (op.to_owned(), count))
            .collect(),
    }
}

fn parse(source: &str, family: &str) -> (Vec<Case>, Ledger) {
    let (setting_types, organism_types) = primitive_reference_schemas();
    let (statistic_types, getter_types) = statistics_schemas();
    let gene_schema = gene_fields(&Gene::new(1.0, 0.0, Some(0xff00ff00), 0, [0; 36]))
        .into_keys()
        .collect();
    let mut ledger = Ledger::default();
    let mut cases = Vec::new();
    let mut current: Option<Case> = None;
    let mut names = BTreeSet::new();
    let mut known = BTreeSet::new();
    let mut corridors = BTreeSet::new();
    let mut stage = String::new();
    let mut last_call = String::new();
    let mut operation = String::new();
    let mut calls = 0;
    let mut discoveries = 0;
    for (line_number, line) in source.lines().enumerate() {
        assert!(!line.is_empty(), "Empty incoming row {}", line_number + 1);
        let row: Vec<_> = line.split('\t').collect();
        ledger.rows += 1;
        *ledger.tags.entry(row[0].to_owned()).or_default() += 1;
        if row[0] == "C" {
            assert_eq!(row.len(), 4);
            assert!(current.is_none() && names.insert(row[1].to_owned()));
            assert_eq!(
                row[2],
                if family == "diagnostic" {
                    "incoming-diagnostic"
                } else {
                    "incoming"
                }
            );
            current = Some(Case {
                name: row[1].into(),
                mode: row[2].into(),
                seed: row[3].parse().unwrap(),
                settings: Fields::new(),
                input_genotypes: BTreeMap::new(),
                snapshots: BTreeMap::new(),
                steps: Vec::new(),
            });
            known.clear();
            corridors.clear();
            stage.clear();
            last_call.clear();
            operation.clear();
            calls = 0;
            discoveries = 0;
            ledger.other += 1;
            continue;
        }
        let case = current.as_mut().expect("Incoming row outside a case");
        match row[0] {
            "U" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[2], setting_types[row[1]]);
                validate(row[2], row[3]);
                insert_unique(&mut case.settings, row[1], row[3]);
                ledger.other += 1;
            }
            "I" => {
                assert_eq!(row.len(), 5);
                assert!(row[1] == "setup" || row[1] == format!("step-{calls}"));
                *ledger
                    .targets
                    .entry((row[2].into(), row[3].into()))
                    .or_default() += 1;
                match row[2] {
                    "world:inCorridors" => {
                        assert_eq!(row[3], "List");
                        for name in role_list(row[4]) {
                            assert!(corridors.contains(name));
                        }
                    }
                    "visible:selected" => {
                        assert_eq!((row[3], row[4]), ("String", "selector"));
                        assert!(known.contains(row[4]));
                    }
                    _ => validate(row[3], row[4]),
                }
                if row[2].ends_with(":construction") {
                    assert_eq!(row[1], "setup");
                    assert!(known.is_empty());
                    let name = row[2]
                        .strip_prefix("genome:")
                        .unwrap()
                        .strip_suffix(":construction")
                        .unwrap();
                    assert!(["g0", "g1", "g2", "g3", "g4"].contains(&name));
                    assert_eq!(row[4], if name == "g4" { "clone:g0" } else { "explicit" });
                }
                case.steps.push(Step::Input {
                    stage: row[1].into(),
                    target: row[2].into(),
                    kind: row[3].into(),
                    value: row[4].into(),
                });
                ledger.other += 1;
            }
            "A" => {
                assert_eq!(row.len(), 5);
                known_role(row[1]);
                assert!(["selector", "blocker"].contains(&row[1]));
                assert!(known.insert(row[1].to_owned()));
                reference_label(row[2]);
                assert_ne!(row[2], "~");
                case.steps.push(Step::Actor {
                    role: row[1].into(),
                    genotype: row[2].into(),
                    center: [row[3].parse().unwrap(), row[4].parse().unwrap()],
                });
                ledger.other += 1;
            }
            "K" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], format!("step-{calls}"));
                calls += 1;
                last_call = row[1].into();
                operation = row[2].into();
                match row[2] {
                    "corridor.new" => {
                        corridor_role(row[3]);
                        assert_eq!(row[4], "~");
                        assert!(corridors.insert(row[3].into()));
                    }
                    "corridor.receive" => {
                        assert!(corridors.contains(row[3]));
                        reference_label(row[4]);
                    }
                    "corridor.frame" => {
                        assert!(corridors.contains(row[3]));
                        assert_eq!(row[4], "~");
                    }
                    "construct" => {
                        assert_eq!(row[3], "victim");
                        assert!(known.insert(row[3].into()));
                        reference_label(row[4]);
                        assert_ne!(row[4], "~");
                    }
                    "paste" => {
                        assert!(known.contains(row[3]));
                        let p: Vec<i32> = row[4].split(',').map(|v| v.parse().unwrap()).collect();
                        assert_eq!(p.len(), 2);
                    }
                    "time" => assert_eq!((row[3], row[4]), ("victim", "~")),
                    other => panic!("Unknown incoming operation {other}"),
                }
                if case.name == NET015_CASE {
                    assert_eq!(row[2], "corridor.new");
                    corridors.remove(row[3]);
                }
                case.steps.push(Step::Call {
                    stage: row[1].into(),
                    operation: row[2].into(),
                    target: row[3].into(),
                    argument: row[4].into(),
                });
                *ledger.calls.entry(row[2].into()).or_default() += 1;
                ledger.other += 1;
            }
            "D" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], last_call);
                assert_eq!(row[2], format!("arrival{discoveries}"));
                discoveries += 1;
                assert!(known.insert(row[2].into()));
                reference_label(row[4]);
                assert_ne!(row[4], "~");
                case.steps.push(Step::Discover {
                    stage: row[1].into(),
                    role: row[2].into(),
                    id: row[3].parse().unwrap(),
                    genotype: row[4].into(),
                });
                ledger.other += 1;
            }
            "J" => {
                assert_eq!(row.len(), 7);
                assert_eq!(row[1], last_call);
                assert!(known.contains(row[2]));
                let bounds = IntRect {
                    x: row[3].parse().unwrap(),
                    y: row[4].parse().unwrap(),
                    width: row[5].parse().unwrap(),
                    height: row[6].parse().unwrap(),
                };
                assert!(
                    case.snapshots
                        .entry(row[1].into())
                        .or_default()
                        .insertions
                        .insert(row[2].into(), bounds)
                        .is_none()
                );
                ledger.eligible += 1;
            }
            "B" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[1], last_call);
                assert_eq!((row[2], operation.as_str()), ("paste", "paste"));
                assert!(
                    case.snapshots
                        .entry(row[1].into())
                        .or_default()
                        .returned
                        .replace(row[3].parse().unwrap())
                        .is_none()
                );
                ledger.eligible += 1;
            }
            "X" => {
                assert_eq!(row.len(), 4);
                assert_eq!(case.name, NET015_CASE);
                assert_eq!(row[1], last_call);
                assert_eq!(row[2], "java.lang.IllegalArgumentException");
                assert_eq!(row[3], "Ym91bmQgbXVzdCBiZSBwb3NpdGl2ZQ==");
                assert!(
                    case.snapshots
                        .entry(row[1].into())
                        .or_default()
                        .exception
                        .replace((row[2].into(), row[3].into()))
                        .is_none()
                );
                ledger.other += 1;
            }
            "P" => {
                assert_eq!(row.len(), 2);
                assert!(row[1] == "initial" || row[1] == last_call);
                stage = row[1].into();
                let snapshot = case.snapshots.entry(stage.clone()).or_default();
                assert!(snapshot.suffix.is_none());
                snapshot.roles = known.clone();
                snapshot.corridor_roles = corridors.clone();
                case.steps.push(Step::Snapshot(stage.clone()));
                ledger.other += 1;
            }
            "O" => {
                assert_eq!(row.len(), 6);
                assert_eq!(row[1], stage);
                assert!(known.contains(row[2]));
                assert_eq!(row[4], organism_types[row[3]]);
                validate(row[4], row[5]);
                insert_unique(
                    case.snapshots
                        .get_mut(&stage)
                        .unwrap()
                        .organisms
                        .entry(row[2].into())
                        .or_default(),
                    row[3],
                    row[5],
                );
                if ["_world", "_visibleWorld"].contains(&row[3]) {
                    assert_eq!(row[5], "present");
                    ledger.collaborators += 1;
                } else {
                    ledger.eligible += 1;
                }
            }
            "H" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], stage);
                assert!(known.contains(row[2]));
                assert!(["genotype", "infected", "saved"].contains(&row[3]));
                reference_label(row[4]);
                assert!(
                    case.snapshots
                        .get_mut(&stage)
                        .unwrap()
                        .references
                        .insert((row[2].into(), row[3].into()), row[4].into())
                        .is_none()
                );
                ledger.eligible += 1;
            }
            "F" => {
                assert_eq!(row.len(), 6);
                assert_eq!(row[1], stage);
                assert!(corridors.contains(row[2]));
                assert_eq!(
                    row[4],
                    CORRIDOR_SCHEMA
                        .iter()
                        .find(|(key, _)| *key == row[3])
                        .unwrap()
                        .1
                );
                match row[3] {
                    "_reference" => assert_eq!(row[2], row[5]),
                    "bounds" => {
                        rectangle_input(row[5]);
                    }
                    "pendingOrganisms" => {
                        for code in row[5].split(',').filter(|s| !s.is_empty()) {
                            reference_label(code);
                        }
                    }
                    "nextOrganism" => {
                        if row[5] != "~" {
                            assert!(known.contains(row[5]));
                        }
                    }
                    "world" => assert_eq!(row[5], "present"),
                    "travellingOrganism" | "connection" => assert_eq!(row[5], "~"),
                    _ => unreachable!(),
                }
                insert_unique(
                    case.snapshots
                        .get_mut(&stage)
                        .unwrap()
                        .corridors
                        .entry(row[2].into())
                        .or_default(),
                    row[3],
                    row[5],
                );
                if OMITTED_CORRIDOR.contains(&row[3]) {
                    ledger.collaborators += 1;
                } else {
                    ledger.eligible += 1;
                }
            }
            "W" | "V" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], stage);
                let schema = if row[0] == "W" {
                    world_schema()
                } else {
                    instance_schema()
                };
                assert_eq!(row[3], schema[row[2]]);
                match row[2] {
                    "_organisms" => {
                        for role in role_list(row[4]) {
                            assert!(known.contains(role));
                        }
                    }
                    "inCorridors" => {
                        for role in role_list(row[4]) {
                            assert!(corridors.contains(role));
                        }
                    }
                    "outCorridors" => assert_eq!(row[4], ""),
                    "worldStatistics" | "_visibleWorld" => assert_eq!(row[4], "present"),
                    "colDetTree" => {
                        let v: Vec<_> = row[4].split(',').collect();
                        assert_eq!(v.len(), 2);
                        assert_eq!(v[0], "present");
                        v[1].parse::<usize>().unwrap();
                    }
                    _ => validate(row[3], row[4]),
                }
                let snapshot = case.snapshots.get_mut(&stage).unwrap();
                insert_unique(
                    if row[0] == "W" {
                        &mut snapshot.world
                    } else {
                        &mut snapshot.instances
                    },
                    row[2],
                    row[4],
                );
                if row[0] == "V" && OMITTED_WORLD.contains(&row[2]) {
                    ledger.collaborators += 1;
                } else {
                    ledger.eligible += 1;
                }
            }
            "Q" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[1], stage);
                assert!(known.contains(row[2]));
                for role in role_list(row[3]) {
                    assert!(known.contains(role));
                }
                insert_unique(
                    &mut case.snapshots.get_mut(&stage).unwrap().queries,
                    row[2],
                    row[3],
                );
                ledger.eligible += 1;
            }
            "S" | "T" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], stage);
                let schema = if row[0] == "S" {
                    &statistic_types
                } else {
                    &getter_types
                };
                assert_eq!(row[3], schema[row[2]]);
                if row[3] == "GeneticCode" {
                    reference_label(row[4]);
                } else {
                    validate(row[3], row[4]);
                }
                let snapshot = case.snapshots.get_mut(&stage).unwrap();
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
                ledger.eligible += 1;
            }
            "Y" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[1], stage);
                reference_label(row[2]);
                assert_eq!(row[3], "observable");
                insert_unique(
                    &mut case.snapshots.get_mut(&stage).unwrap().scopes,
                    row[2],
                    row[3],
                );
                ledger.other += 1;
            }
            "G" | "N" => {
                assert_eq!(row.len(), if row[0] == "G" { 5 } else { 6 });
                reference_label(row[2]);
                assert_ne!(row[2], "~");
                let codes = if row[1] == "input" {
                    &mut case.input_genotypes
                } else {
                    assert_eq!(row[1], stage);
                    &mut case.snapshots.get_mut(&stage).unwrap().genotypes
                };
                let code = codes.entry(row[2].into()).or_default();
                if row[0] == "G" {
                    insert_unique(&mut code.genome, row[3], row[4]);
                } else {
                    insert_unique(
                        code.genes.entry(row[3].parse().unwrap()).or_default(),
                        row[4],
                        row[5],
                    );
                }
                if row[1] == "input" {
                    ledger.input_genotypes += 1;
                } else {
                    ledger.eligible += 1;
                }
            }
            "R" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], stage);
                validate("double", row[4]);
                assert!(
                    case.snapshots
                        .get_mut(&stage)
                        .unwrap()
                        .suffix
                        .replace((
                            row[2].parse().unwrap(),
                            row[3].parse().unwrap(),
                            u64::from_str_radix(row[4], 16).unwrap()
                        ))
                        .is_none()
                );
                ledger.eligible += 1;
            }
            "E" => {
                assert_eq!(row.len(), 1);
                assert_eq!(
                    case.settings.keys().collect::<BTreeSet<_>>(),
                    setting_types.keys().collect()
                );
                assert_eq!(
                    case.input_genotypes
                        .keys()
                        .map(String::as_str)
                        .collect::<BTreeSet<_>>(),
                    BTreeSet::from(["g0", "g1", "g2", "g3", "g4"])
                );
                assert_eq!(case.snapshots.len(), calls + 1);
                for code in case.input_genotypes.values() {
                    validate_genome(code, &gene_schema);
                }
                for snapshot in case.snapshots.values() {
                    assert!(snapshot.suffix.is_some());
                    assert_eq!(
                        snapshot.organisms.keys().cloned().collect::<BTreeSet<_>>(),
                        snapshot.roles
                    );
                    assert_eq!(
                        snapshot.corridors.keys().cloned().collect::<BTreeSet<_>>(),
                        snapshot.corridor_roles
                    );
                    assert_eq!(snapshot.references.len(), 3 * snapshot.roles.len());
                    assert_eq!(
                        snapshot.queries.keys().cloned().collect::<BTreeSet<_>>(),
                        snapshot.roles
                    );
                    for fields in snapshot.organisms.values() {
                        assert_eq!(
                            fields.keys().collect::<BTreeSet<_>>(),
                            organism_types.keys().collect()
                        );
                    }
                    for fields in snapshot.corridors.values() {
                        assert_eq!(
                            fields.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                            CORRIDOR_SCHEMA.into_iter().map(|(key, _)| key).collect()
                        );
                    }
                    assert_eq!(
                        snapshot
                            .world
                            .keys()
                            .map(String::as_str)
                            .collect::<BTreeSet<_>>(),
                        world_schema().into_keys().collect()
                    );
                    assert_eq!(
                        snapshot
                            .instances
                            .keys()
                            .map(String::as_str)
                            .collect::<BTreeSet<_>>(),
                        instance_schema().into_keys().collect()
                    );
                    assert_eq!(
                        snapshot.statistics.keys().collect::<BTreeSet<_>>(),
                        statistic_types.keys().collect()
                    );
                    assert_eq!(
                        snapshot.getters.keys().collect::<BTreeSet<_>>(),
                        getter_types.keys().collect()
                    );
                    assert_eq!(
                        snapshot.scopes.keys().collect::<BTreeSet<_>>(),
                        snapshot.genotypes.keys().collect()
                    );
                    for code in snapshot.genotypes.values() {
                        validate_genome(code, &gene_schema);
                    }
                    for (role, _) in &snapshot.insertions {
                        assert!(snapshot.roles.contains(role));
                    }
                }
                cases.push(current.take().unwrap());
                ledger.other += 1;
            }
            tag => panic!("Unknown incoming row {tag} at {}", line_number + 1),
        }
    }
    assert!(current.is_none());
    assert_eq!(
        ledger,
        expected_ledger(family),
        "Incoming fixed precomparison ledger"
    );
    assert_eq!(
        ledger.rows,
        ledger.eligible + ledger.input_genotypes + ledger.collaborators + ledger.other
    );
    (cases, ledger)
}

fn suffix(random: &JavaRandom) -> (i64, i32, u64) {
    let mut peek = random.clone();
    (
        peek.next_i64(),
        peek.next_bounded(10000).unwrap(),
        peek.next_f64().to_bits(),
    )
}

fn corridor_label<'a>(
    reference: &IncomingCorridorRef,
    corridors: &'a BTreeMap<String, IncomingCorridorRef>,
) -> &'a str {
    corridors
        .iter()
        .find_map(|(role, known)| Rc::ptr_eq(reference, known).then_some(role.as_str()))
        .expect("Every registered corridor needs its actual identity")
}

fn genome_label<'a>(reference: &GenomeRef, genomes: &'a BTreeMap<String, GenomeRef>) -> &'a str {
    genomes
        .iter()
        .find_map(|(role, known)| Rc::ptr_eq(reference, known).then_some(role.as_str()))
        .expect("Every queued genome needs its actual identity")
}

fn tree_view(world: &World, actors: &BTreeMap<String, OrganismRef>) -> BTreeMap<String, IntRect> {
    world.spatial.borrow()
        .stored_envelopes()
        .into_iter()
        .map(|(token, envelope)| {
            let values = envelope.map(|value| {
                let integer = value as i32;
                assert_eq!(value, f64::from(integer));
                integer
            });
            (
                actor_role(&world.frame_objects.borrow()[token], actors).to_owned(),
                IntRect {
                    x: values[0],
                    y: values[1],
                    width: values[2],
                    height: values[3],
                },
            )
        })
        .collect()
}

fn compare_snapshot(
    case: &Case,
    name: &str,
    world: &World,
    actors: &BTreeMap<String, OrganismRef>,
    corridors: &BTreeMap<String, IncomingCorridorRef>,
    genomes: &mut BTreeMap<String, GenomeRef>,
    returned: &mut BTreeMap<String, bool>,
    corrected_random: &Option<JavaRandom>,
    errors: &mut Vec<String>,
) {
    let target = &case.snapshots[name];
    assert_eq!(
        actors.keys().cloned().collect::<BTreeSet<_>>(),
        target.roles,
        "{} {name} retained actor identities",
        case.name
    );
    assert_eq!(
        corridors.keys().cloned().collect::<BTreeSet<_>>(),
        target.corridor_roles
    );
    let mut roots: Vec<_> = case
        .input_genotypes
        .keys()
        .map(|label| Rc::clone(&genomes[label]))
        .collect();
    for ((role, holder), label) in &target.references {
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
        if let Some(actual) = actual {
            roots.push(Rc::clone(actual));
        }
    }
    for (field, (kind, label)) in &target.statistics {
        if kind == "GeneticCode" {
            let statistics_owner = world.statistics_ref();
            let statistics = statistics_owner.borrow();
            let Observation::Genome(actual) = observed_field(&statistics, field)
            else {
                panic!("Statistics reference schema changed")
            };
            bind_reference(
                label,
                actual,
                genomes,
                &format!("{} {name} statistics:{field}", case.name),
            );
            if let Some(actual) = actual {
                roots.push(Rc::clone(actual));
            }
        }
    }
    for (label, scope) in &target.scopes {
        let actual = genomes
            .get(label)
            .is_some_and(|code| roots.iter().any(|root| Rc::ptr_eq(root, code)));
        assert_eq!(
            actual,
            scope == "observable",
            "{} {name} {label} actual genotype roots",
            case.name
        );
    }
    let (_, organism_schema) = primitive_reference_schemas();
    for (role, expected) in &target.organisms {
        let actor = actors[role].borrow();
        let mut actual = organism_fields(&View::from(&*actor));
        if !actor.creation_complete {
            // Only Java allocation phase is projected; controlled scalars remain actual state
            for (field, kind) in &organism_schema {
                if kind.ends_with("[]") {
                    actual.insert(field.clone(), "~".into());
                }
            }
        }
        let mut expected = expected.clone();
        for field in ["_world", "_visibleWorld"] {
            assert_eq!(expected.remove(field).as_deref(), Some("present"));
            assert!(actual.remove(field).is_some());
        }
        compare_fields(
            &case.name,
            &format!("{name}:{role}"),
            "O",
            &actual,
            &expected,
            errors,
        );
    }
    let (actual_world, mut actual_instances) = world_views(world, actors);
    actual_instances.insert("_corridorexists".into(), world.corridors_exist.get().to_string());
    actual_instances.insert(
        "inCorridors".into(),
        array(
            world.incoming_corridors.borrow()
                .iter()
                .map(|reference| corridor_label(reference, corridors)),
        ),
    );
    compare_fields(&case.name, name, "W", &actual_world, &target.world, errors);
    let expected_instances = target
        .instances
        .iter()
        .filter(|(field, _)| !OMITTED_WORLD.contains(&field.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    compare_fields(
        &case.name,
        name,
        "V",
        &actual_instances,
        &expected_instances,
        errors,
    );
    for (role, expected) in &target.corridors {
        let current = corridors[role].borrow();
        let actual = Fields::from([
            (
                "_reference".into(),
                corridor_label(&corridors[role], corridors).into(),
            ),
            ("bounds".into(), rectangle(current.bounds)),
            (
                "pendingOrganisms".into(),
                array(current.pending.iter().map(|code| {
                    code.as_ref()
                        .map_or("~", |code| genome_label(code, genomes))
                })),
            ),
            (
                "nextOrganism".into(),
                current
                    .candidate
                    .as_ref()
                    .map_or("~", |actor| actor_role(actor, actors))
                    .into(),
            ),
            // F.world is an explicit logical owner projection, not a fabricated Java object
            ("world".into(), "present".into()),
        ]);
        let expected = expected
            .iter()
            .filter(|(field, _)| !OMITTED_CORRIDOR.contains(&field.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        compare_fields(
            &case.name,
            &format!("{name}:{role}"),
            "F",
            &actual,
            &expected,
            errors,
        );
    }
    for (role, expected) in &target.queries {
        assert_eq!(
            &query_roles(world, actors[role].borrow().state.bounds.get(), actors),
            expected,
            "{} {name} {role} actual tree query",
            case.name
        );
    }
    assert_eq!(
        tree_view(world, actors),
        target.insertions,
        "{} {name} actual stored envelopes",
        case.name
    );
    for (field, (kind, value)) in &target.statistics {
        assert_observation(
            observed_field(&world.statistics_ref().borrow(), field),
            kind,
            value,
            genomes,
            &format!("{} {name} S {field}", case.name),
        );
    }
    for (getter, (kind, value)) in &target.getters {
        assert_observation(
            observed_getter(&world.statistics_ref().borrow(), getter),
            kind,
            value,
            genomes,
            &format!("{} {name} T {getter}", case.name),
        );
    }
    let actual_codes = target
        .genotypes
        .keys()
        .map(|label| (label.clone(), Rc::clone(&genomes[label])))
        .collect();
    compare_genomes(&case.name, name, &target.genotypes, &actual_codes, errors);
    assert_eq!(
        returned.remove(name),
        target.returned,
        "{} {name} actual paste result",
        case.name
    );
    if target.exception.is_some() {
        assert_eq!(case.name, NET015_CASE);
        assert_eq!(name, "step-0");
        // The source throws before drawing; corrected Rust placement deliberately draws one positive-y coordinate
        assert_eq!(
            target.suffix, case.snapshots["initial"].suffix,
            "NET015 source exception RNG invariant"
        );
        assert_eq!(
            &world.random_snapshot(),
            corrected_random
                .as_ref()
                .expect("Missing corrected constructor observation")
        );
    } else {
        assert_eq!(
            Some(suffix(&world.random_snapshot())),
            target.suffix,
            "{} {name} actual RNG suffix",
            case.name
        );
    }
}

fn apply_input(
    case: &Case,
    target: &str,
    kind: &str,
    value: &str,
    world: &mut World,
    actors: &BTreeMap<String, OrganismRef>,
    corridors: &BTreeMap<String, IncomingCorridorRef>,
    genomes: &mut BTreeMap<String, GenomeRef>,
    errors: &mut Vec<String>,
) {
    let path: Vec<_> = target.split(':').collect();
    match path.as_slice() {
        ["corridor", label, "bounds"] => {
            assert_eq!(kind, "Rectangle");
            corridors[*label].borrow_mut().bounds = rectangle_input(value);
        }
        ["world", "inCorridors"] => {
            assert_eq!(kind, "List");
            *world.incoming_corridors.borrow_mut() = role_list(value)
                .into_iter()
                .map(|role| Rc::clone(&corridors[role]))
                .collect();
        }
        ["world", "_corridorexists"] => {
            assert_eq!(kind, "boolean");
            world.corridors_exist.set(value.parse().unwrap());
        }
        ["genome", label, "gene0._branch"] => {
            assert_eq!(kind, "int");
            genomes[*label]
                .borrow()
                .gene_ref(0).unwrap().borrow_mut()
                .set_branch(value.parse().unwrap());
        }
        _ => apply_reference_input(
            &case.name,
            &case.input_genotypes,
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

fn replay(case: &Case, errors: &mut Vec<String>) {
    assert!(["incoming", "incoming-diagnostic"].contains(&case.mode.as_str()));
    assert_eq!(integer(&case.settings, "INITIAL_ORGANISMS"), 0);
    let mut world = empty_reference_world(&case.settings, case.seed);
    // Original World construction leaves gases zero; explicit I records install any genesis-like gas inputs
    world.oxygen.set(0.0);
    world.carbon_dioxide.set(0.0);
    world.methane.set(0.0);
    let mut actors = BTreeMap::<String, OrganismRef>::new();
    let mut corridors = BTreeMap::<String, IncomingCorridorRef>::new();
    let mut genomes = BTreeMap::<String, GenomeRef>::new();
    let mut returned = BTreeMap::new();
    let mut corrected_random = None;
    let mut corrected_corridor = None;
    let mut latest_call = String::new();
    for step in &case.steps {
        match step {
            Step::Input {
                stage,
                target,
                kind,
                value,
            } => {
                assert!(stage == "setup" || stage.starts_with("step-"));
                apply_input(
                    case,
                    target,
                    kind,
                    value,
                    &mut world,
                    &actors,
                    &corridors,
                    &mut genomes,
                    errors,
                );
            }
            Step::Actor {
                role,
                genotype,
                center,
            } => {
                let actor = Rc::new(RefCell::new(
                    world.new_pasted_candidate(Rc::clone(&genomes[genotype])),
                ));
                assert!(
                    world
                        .attempt_paste(Some(&actor), &mut actor.borrow_mut(), *center)
                        .unwrap()
                );
                world.add_organism(Rc::clone(&actor), None);
                assert!(actors.insert(role.clone(), actor).is_none());
            }
            Step::Call {
                stage,
                operation,
                target,
                argument,
            } => {
                latest_call = stage.clone();
                match operation.as_str() {
                    "corridor.new" => {
                        if case.name == NET015_CASE {
                            assert_eq!(world.config.get().width, 100);
                            let mut expected = world.random_snapshot();
                            let y = expected
                                .next_bounded((world.config.get().height as i32) - 100)
                                .unwrap();
                            let actual = world.new_incoming_corridor();
                            // Nine independent corrected-object assertions; no expected source object exists
                            assert_eq!(actual.borrow().bounds.x, 0);
                            assert_eq!(actual.borrow().bounds.y, y);
                            assert_eq!(actual.borrow().bounds.width, 100);
                            assert_eq!(actual.borrow().bounds.height, 100);
                            assert!(!world.corridors_exist.get());
                            assert!(world.incoming_corridors.borrow().is_empty());
                            assert!(actual.borrow().candidate.is_none());
                            assert!(actual.borrow().pending.is_empty());
                            assert_eq!(world.random_snapshot(), expected);
                            corrected_random = Some(expected);
                            corrected_corridor = Some(actual);
                        } else {
                            let actual = world.new_incoming_corridor();
                            assert!(corridors.insert(target.clone(), actual).is_none());
                        }
                    }
                    "corridor.receive" => {
                        corridors[target].borrow_mut().receive(if argument == "~" {
                            None
                        } else {
                            Some(Rc::clone(&genomes[argument]))
                        })
                    }
                    "corridor.frame" => world.frame_incoming_corridor(&corridors[target]).unwrap(),
                    "construct" => {
                        let actor = Rc::new(RefCell::new(
                            world.new_pasted_candidate(Rc::clone(&genomes[argument])),
                        ));
                        assert!(actors.insert(target.clone(), actor).is_none());
                    }
                    "paste" => {
                        let point: Vec<i32> = argument
                            .split(',')
                            .map(|value| value.parse().unwrap())
                            .collect();
                        let actor = &actors[target];
                        let actual = world
                            .attempt_paste(
                                Some(actor),
                                &mut actor.borrow_mut(),
                                [point[0], point[1]],
                            )
                            .unwrap();
                        assert!(returned.insert(stage.clone(), actual).is_none());
                    }
                    "time" => assert!(world.single_step().unwrap()),
                    other => panic!("Unknown actual incoming call {other}"),
                }
            }
            Step::Discover {
                stage,
                role,
                id,
                genotype,
            } => {
                assert_eq!(&latest_call, stage);
                let actual = world.organisms.borrow()
                    .iter()
                    .find(|actor| !actors.values().any(|known| Rc::ptr_eq(actor, known)))
                    .cloned()
                    .or_else(|| {
                        corridors.values().find_map(|corridor| {
                            corridor
                                .borrow()
                                .candidate
                                .as_ref()
                                .filter(|actor| {
                                    !actors.values().any(|known| Rc::ptr_eq(actor, known))
                                })
                                .cloned()
                        })
                    })
                    .unwrap_or_else(|| {
                        panic!("{} {stage}: no actual discovered object {role}", case.name)
                    });
                assert_eq!(actual.borrow().id, i64::from(*id));
                bind_reference(
                    genotype,
                    actual.borrow().genome.as_ref(),
                    &mut genomes,
                    &format!("{} {stage} D {role}", case.name),
                );
                assert!(actors.insert(role.clone(), actual).is_none());
            }
            Step::Snapshot(name) => {
                for member in world.organisms.borrow().clone() {
                    actor_role(&member, &actors);
                }
                for corridor in corridors.values() {
                    if let Some(actor) = &corridor.borrow().candidate {
                        actor_role(actor, &actors);
                    }
                }
                compare_snapshot(
                    case,
                    name,
                    &world,
                    &actors,
                    &corridors,
                    &mut genomes,
                    &mut returned,
                    &corrected_random,
                    errors,
                );
            }
        }
        assert!(
            errors.is_empty(),
            "{}: first mismatch {:?}",
            case.name,
            errors.first()
        );
    }
    assert!(returned.is_empty());
    assert_eq!(corrected_corridor.is_some(), case.name == NET015_CASE);
}

fn compare_fixture(source: &str, family: &str) {
    let (cases, ledger) = parse(source, family);
    // Freeze the entire schema and ledger before any source-derived runtime construction
    assert_eq!(ledger, expected_ledger(family));
    let mut errors = Vec::new();
    for case in &cases {
        replay(case, &mut errors);
    }
    assert!(
        errors.is_empty(),
        "Incoming mismatches: {}",
        errors.join("\n")
    );
    println!(
        "INCOMING_MATCH family={family} rows={} eligible={} input_GN={} collaborators={} other={} logical_owner_projections={} source_rng_invariants={} corrected_object_assertions={}",
        ledger.rows,
        ledger.eligible,
        ledger.input_genotypes,
        ledger.collaborators,
        ledger.other,
        ledger.tags["P"] + ledger.tags.get("F").copied().unwrap_or(0) / 7,
        usize::from(family == "diagnostic"),
        9 * usize::from(family == "diagnostic")
    );
}

#[test]
fn ordinary_incoming_queue_candidates_and_frames_match_original_java() {
    compare_fixture(include_str!("../tests/fixtures/incoming.tsv"), "ordinary");
}

#[test]
fn direct_empty_paste_and_named_zero_span_correction_preserve_the_full_diagnostic_ledger() {
    compare_fixture(
        include_str!("../tests/fixtures/incoming_diagnostic.tsv"),
        "diagnostic",
    );
}

#[test]
fn supplemental_empty_arrival_keeps_exact_nonfinite_motion_and_frozen_envelopes() {
    compare_fixture(
        include_str!("../tests/fixtures/incoming_empty.tsv"),
        "empty",
    );
}

#[test]
fn checker_rejects_changed_source_rng_without_using_it_as_construction_input() {
    let (mut cases, _) = parse(include_str!("../tests/fixtures/incoming.tsv"), "ordinary");
    let case = &mut cases[0];
    let original = case.snapshots["initial"].suffix.unwrap();
    case.snapshots.get_mut("initial").unwrap().suffix =
        Some((original.0.wrapping_add(1), original.1, original.2));
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        replay(case, &mut Vec::new())
    }))
    .expect_err("Corrupted source RNG was accepted");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(
        message.contains("initial actual RNG suffix"),
        "Wrong rejection: {message}"
    );
}
