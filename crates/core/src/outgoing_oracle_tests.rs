// SPDX-License-Identifier: GPL-2.0-or-later

//! Compare outgoing transfer and paint methods against two separately frozen source corpora

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
use crate::world::outgoing::{
    ConnectionRef, DrawCommand as RuntimeDrawCommand, OutgoingConfig, OutgoingCorridorRef,
    OutgoingFailure, TransferOutcome,
};
use crate::world::incoming::IncomingCorridorRef;
use std::rc::Rc;

const CORRIDOR_SCHEMA: [(&str, &str); 6] = [
    ("_reference", "String"),
    ("bounds", "Rectangle"),
    ("connection", "Connection"),
    ("travellingOrganism", "Organism"),
    ("world", "World"),
    ("canSendOrganism", "boolean"),
];
const INCOMING_SCHEMA: [(&str, &str); 7] = [
    ("_reference", "String"),
    ("bounds", "Rectangle"),
    ("pendingOrganisms", "Queue"),
    ("nextOrganism", "Organism"),
    ("world", "World"),
    ("travellingOrganism", "Organism"),
    ("connection", "Connection"),
];
const CONNECTION_SCHEMA: [(&str, &str); 3] = [
    ("_reference", "String"),
    ("state", "int"),
    ("recordedGenotypes", "List"),
];
const OMITTED_WORLD: [&str; 2] = ["_visibleWorld", "inCorridors"];
const NULL_CLADE_CASE: &str = "null-clade-fails-before-send-and-slot-assignment";
const SEND_FAILURE_CASE: &str = "recording-send-throws-after-clade-prefix-before-other-effects";

#[derive(Clone, Debug, PartialEq, Eq)]
struct PaintCommand {
    operation: String,
    coordinates: [i32; 4],
    argb: u32,
}

#[derive(Default)]
struct Snapshot {
    organisms: BTreeMap<String, Fields>,
    references: BTreeMap<(String, String), String>,
    corridors: BTreeMap<String, Fields>,
    connections: BTreeMap<String, Fields>,
    incoming: BTreeMap<String, Fields>,
    owners: Fields,
    paint: Vec<PaintCommand>,
    world: Fields,
    instances: Fields,
    statistics: BTreeMap<String, (String, String)>,
    getters: BTreeMap<String, (String, String)>,
    genotypes: BTreeMap<String, Stage>,
    scopes: Fields,
    queries: Fields,
    insertions: BTreeMap<String, IntRect>,
    returned: Option<(String, String)>,
    suffix: Option<(i64, i32, u64)>,
    exception: Option<(String, String)>,
    roles: BTreeSet<String>,
    corridor_roles: BTreeSet<String>,
    connection_roles: BTreeSet<String>,
    incoming_roles: BTreeSet<String>,
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
    if !["selector", "victim", "killer", "blocker"].contains(&role) {
        role.strip_prefix("arrival").unwrap().parse::<usize>().unwrap();
    }
}
fn connection_role(role: &str) {
    role.strip_prefix('n').unwrap().parse::<usize>().unwrap();
}

fn corridor_role(role: &str) {
    role.strip_prefix('c').unwrap().parse::<usize>().unwrap();
}

fn incoming_role(role: &str) {
    role.strip_prefix('i').unwrap().parse::<usize>().unwrap();
}

fn insert_unique(fields: &mut Fields, key: &str, value: &str) {
    assert!(
        fields.insert(key.to_owned(), value.to_owned()).is_none(),
        "Duplicate field {key}"
    );
}

fn expected_ledger(family: &str) -> Ledger {
    match family {
        "core" => Ledger {
            rows: 185514,
            eligible: 166432,
            input_genotypes: 11200,
            collaborators: 1400,
            other: 6482,
            tags: BTreeMap::from([
                ("C".into(), 32),
                ("U".into(), 4704),
                ("I".into(), 437),
                ("G".into(), 28980),
                ("N".into(), 43640),
                ("A".into(), 96),
                ("P".into(), 175),
                ("O".into(), 80850),
                ("H".into(), 1575),
                ("W".into(), 1750),
                ("V".into(), 2625),
                ("Q".into(), 525),
                ("S".into(), 9100),
                ("T".into(), 8575),
                ("Y".into(), 875),
                ("R".into(), 175),
                ("K".into(), 129),
                ("CC".into(), 450),
                ("F".into(), 696),
                ("B".into(), 43),
                ("E".into(), 32),
                ("X".into(), 2),
                ("L".into(), 48),
            ]),
            calls: BTreeMap::from([
                ("connection.new".into(), 34),
                ("corridor.new".into(), 34),
                ("canSend".into(), 5),
                ("send".into(), 27),
                ("die".into(), 2),
                ("draw".into(), 13),
                ("world.draw".into(), 1),
                ("move".into(), 8),
                ("checkHitCorridor".into(), 5),
            ]),
            targets: BTreeMap::from([
                (("actor:victim:_age".into(), "int".into()), 8),
                (("actor:victim:_canmove".into(), "boolean".into()), 1),
                (("actor:victim:_centerX".into(), "int".into()), 5),
                (("actor:victim:_color".into(), "Color".into()), 3),
                (("actor:victim:_dCenterX".into(), "double".into()), 1),
                (("actor:victim:_energy".into(), "double".into()), 3),
                (("actor:victim:_framesColor".into(), "int".into()), 5),
                (("actor:victim:_growthRatio".into(), "int".into()), 9),
                (("actor:victim:_haseyes".into(), "boolean".into()), 6),
                (("actor:victim:_segColor".into(), "Color[]".into()), 2),
                (("actor:victim:active".into(), "boolean".into()), 1),
                (("actor:victim:dtheta".into(), "double".into()), 8),
                (("actor:victim:dx".into(), "double".into()), 9),
                (("actor:victim:dy".into(), "double".into()), 8),
                (("connection:n0:failOnSend".into(), "boolean".into()), 2),
                (("connection:n0:state".into(), "int".into()), 4),
                (("corridor:c0:bounds".into(), "Rectangle".into()), 11),
                (("corridor:c1:bounds".into(), "Rectangle".into()), 2),
                (("genome:g0:_cladeID".into(), "String".into()), 4),
                (("genome:g0:_generation".into(), "int".into()), 1),
                (("genome:g0:construction".into(), "String".into()), 32),
                (("genome:g0:gene0._length".into(), "double".into()), 1),
                (("genome:g1:construction".into(), "String".into()), 32),
                (("genome:g2:construction".into(), "String".into()), 32),
                (("genome:g3:construction".into(), "String".into()), 32),
                (("genome:g4:construction".into(), "String".into()), 32),
                (("utils:USER_NAME".into(), "String".into()), 32),
                (("visible:selected".into(), "String".into()), 32),
                (("world:_CH4".into(), "double".into()), 32),
                (("world:_CO2".into(), "double".into()), 32),
                (("world:_O2".into(), "double".into()), 34),
                (("world:_corridorexists".into(), "boolean".into()), 10),
                (("world:_population".into(), "int".into()), 1),
                (("world:outCorridors".into(), "List".into()), 10),
            ]),
        },
        "frames" => Ledger {
            rows: 27438,
            eligible: 25498,
            input_genotypes: 1020,
            collaborators: 216,
            other: 704,
            tags: BTreeMap::from([
                ("C".into(), 3),
                ("U".into(), 441),
                ("I".into(), 67),
                ("G".into(), 4200),
                ("N".into(), 6000),
                ("A".into(), 9),
                ("P".into(), 27),
                ("O".into(), 12474),
                ("H".into(), 243),
                ("W".into(), 270),
                ("V".into(), 405),
                ("Q".into(), 81),
                ("S".into(), 1404),
                ("T".into(), 1323),
                ("Y".into(), 135),
                ("R".into(), 27),
                ("K".into(), 19),
                ("CC".into(), 96),
                ("F".into(), 168),
                ("J".into(), 23),
                ("L".into(), 20),
                ("E".into(), 3),
            ]),
            calls: BTreeMap::from([
                ("connection.new".into(), 4),
                ("corridor.new".into(), 4),
                ("time".into(), 6),
                ("draw".into(), 3),
                ("world.draw".into(), 2),
            ]),
            targets: BTreeMap::from([
                (("actor:killer:_age".into(), "int".into()), 2),
                (("actor:killer:_centerX".into(), "int".into()), 1),
                (("actor:killer:_centerY".into(), "int".into()), 1),
                (("actor:killer:_dCenterX".into(), "double".into()), 1),
                (("actor:killer:_dCenterY".into(), "double".into()), 1),
                (("actor:killer:_growthRatio".into(), "int".into()), 1),
                (("actor:killer:dtheta".into(), "double".into()), 1),
                (("actor:killer:dx".into(), "double".into()), 2),
                (("actor:victim:_age".into(), "int".into()), 3),
                (("actor:victim:_growthRatio".into(), "int".into()), 3),
                (("actor:victim:dtheta".into(), "double".into()), 4),
                (("actor:victim:dx".into(), "double".into()), 4),
                (("actor:victim:dy".into(), "double".into()), 3),
                (("corridor:c0:bounds".into(), "Rectangle".into()), 3),
                (("corridor:c1:bounds".into(), "Rectangle".into()), 1),
                (("genome:g0:construction".into(), "String".into()), 3),
                (("genome:g1:construction".into(), "String".into()), 3),
                (("genome:g2:construction".into(), "String".into()), 3),
                (("genome:g3:construction".into(), "String".into()), 3),
                (("genome:g4:construction".into(), "String".into()), 3),
                (("utils:USER_NAME".into(), "String".into()), 3),
                (("visible:selected".into(), "String".into()), 3),
                (("world:_CH4".into(), "double".into()), 3),
                (("world:_CO2".into(), "double".into()), 3),
                (("world:_O2".into(), "double".into()), 3),
                (("world:_corridorexists".into(), "boolean".into()), 3),
                (("world:outCorridors".into(), "List".into()), 3),
            ]),
        },
        "world-rules" => world_rules_ledger(),
        other => panic!("Unknown outgoing family {other}"),
    }
}

fn world_rules_ledger() -> Ledger {
    Ledger {
        rows: 54800,
        eligible: 51968,
        input_genotypes: 1440,
        collaborators: 385,
        other: 1007,
        tags: BTreeMap::from([
            ("A".into(), 12),
            ("C".into(), 4),
            ("CC".into(), 45),
            ("D".into(), 4),
            ("E".into(), 4),
            ("F".into(), 84),
            ("G".into(), 7700),
            ("H".into(), 504),
            ("I".into(), 60),
            ("ID".into(), 509),
            ("IN".into(), 91),
            ("J".into(), 24),
            ("K".into(), 31),
            ("L".into(), 13),
            ("N".into(), 12560),
            ("O".into(), 25872),
            ("P".into(), 49),
            ("Q".into(), 168),
            ("R".into(), 49),
            ("S".into(), 2548),
            ("T".into(), 2401),
            ("U".into(), 588),
            ("V".into(), 735),
            ("W".into(), 490),
            ("Y".into(), 255),
        ]),
        calls: BTreeMap::from([
            ("connection.new".into(), 1),
            ("corridor.new".into(), 1),
            ("die".into(), 3),
            ("disperseAll".into(), 6),
            ("genesis".into(), 1),
            ("incoming.frame".into(), 1),
            ("incoming.new".into(), 1),
            ("incoming.receive".into(), 3),
            ("killAll".into(), 6),
            ("time".into(), 6),
            ("world.addCorridors".into(), 1),
            ("world.draw".into(), 1),
        ]),
        targets: BTreeMap::from([
            (("actor:blocker:bounds".into(), "Rectangle".into()), 1),
            (("actor:killer:_energy".into(), "double".into()), 3),
            (("actor:selector:_energy".into(), "double".into()), 3),
            (("actor:victim:_age".into(), "int".into()), 1),
            (("actor:victim:_energy".into(), "double".into()), 3),
            (("actor:victim:_growthRatio".into(), "int".into()), 1),
            (("actor:victim:dtheta".into(), "double".into()), 1),
            (("actor:victim:dx".into(), "double".into()), 1),
            (("actor:victim:dy".into(), "double".into()), 1),
            (("corridor:c0:bounds".into(), "Rectangle".into()), 1),
            (("genome:g0:construction".into(), "String".into()), 4),
            (("genome:g1:construction".into(), "String".into()), 4),
            (("genome:g2:construction".into(), "String".into()), 4),
            (("genome:g3:construction".into(), "String".into()), 4),
            (("genome:g4:construction".into(), "String".into()), 4),
            (("incoming:i0:bounds".into(), "Rectangle".into()), 1),
            (("utils:USER_NAME".into(), "String".into()), 4),
            (("visible:selected".into(), "String".into()), 4),
            (("world:_CH4".into(), "double".into()), 4),
            (("world:_CO2".into(), "double".into()), 4),
            (("world:_O2".into(), "double".into()), 7),
        ]),
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
    let mut connections = BTreeSet::new();
    let mut incoming = BTreeSet::new();
    let mut discoveries = 0;
    let mut stage = String::new();
    let mut last_call = String::new();
    let mut operation = String::new();
    let mut calls = 0;
    let mut paint_calls = 0;
    let mut global_calls = 0;
    for (line_number, line) in source.lines().enumerate() {
        assert!(!line.is_empty(), "Empty outgoing row {}", line_number + 1);
        let row: Vec<_> = line.split('\t').collect();
        ledger.rows += 1;
        *ledger.tags.entry(row[0].to_owned()).or_default() += 1;
        if row[0] == "C" {
            assert_eq!(row.len(), 4);
            assert!(current.is_none() && names.insert(row[1].to_owned()));
            assert_eq!(
                row[2],
                if family == "world-rules" {
                    "world-rules"
                } else if family == "frames" {
                    "outgoing-frames"
                } else {
                    "outgoing"
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
            connections.clear();
            incoming.clear();
            discoveries = 0;
            stage.clear();
            last_call.clear();
            operation.clear();
            calls = 0;
            paint_calls = 0;
            global_calls = 0;
            ledger.other += 1;
            continue;
        }
        let case = current.as_mut().expect("Outgoing row outside a case");
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
                        assert_eq!(family, "world-rules");
                        assert_eq!(row[3], "List");
                        for name in role_list(row[4]) { assert!(incoming.contains(name)); }
                    }
                    "world:outCorridors" => {
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
                assert!(["selector", "victim", "killer"].contains(&row[1])
                    || (family == "world-rules" && row[1] == "blocker"));
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
                    "incoming.new" => {
                        assert_eq!(family, "world-rules");
                        incoming_role(row[3]);
                        assert_eq!(row[4], "~");
                        assert!(incoming.insert(row[3].into()));
                    }
                    "incoming.receive" => {
                        assert_eq!(family, "world-rules");
                        assert!(incoming.contains(row[3]));
                        reference_label(row[4]);
                        assert_ne!(row[4], "~");
                    }
                    "incoming.frame" => {
                        assert_eq!(family, "world-rules");
                        assert!(incoming.contains(row[3]));
                        assert_eq!(row[4], "~");
                    }
                    "world.addCorridors" => {
                        assert_eq!(family, "world-rules");
                        assert!(incoming.contains(row[3]) && corridors.contains(row[4]));
                    }
                    "genesis" | "killAll" | "disperseAll" => {
                        global_calls += 1;
                        assert_eq!(family, "world-rules");
                        assert_eq!((row[3], row[4]), ("victim", "~"));
                    }
                    "connection.new" => {
                        connection_role(row[3]);
                        assert_eq!(row[4], "~");
                        assert!(connections.insert(row[3].into()));
                    }
                    "corridor.new" => {
                        corridor_role(row[3]);
                        assert!(connections.contains(row[4]));
                        assert!(corridors.insert(row[3].into()));
                    }
                    "send" => {
                        assert!(corridors.contains(row[3]) && known.contains(row[4]));
                    }
                    "canSend" | "draw" => {
                        assert!(corridors.contains(row[3]));
                        assert_eq!(row[4], "~");
                    }
                    "move" | "checkHitCorridor" | "die" => {
                        assert!(known.contains(row[3]));
                        assert_eq!(row[4], "~");
                    }
                    "time" => {
                        assert!(["frames", "world-rules"].contains(&family));
                        assert_eq!((row[3], row[4]), ("victim", "~"));
                    }
                    "world.draw" => assert_eq!((row[3], row[4]), ("victim", "~")),
                    other => panic!("Unknown outgoing operation {other}"),
                }
                if ["draw", "world.draw"].contains(&row[2]) {
                    paint_calls += 1;
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
                assert_eq!(family, "world-rules");
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], last_call);
                assert_eq!(row[2], format!("arrival{discoveries}"));
                discoveries += 1;
                assert!(known.insert(row[2].into()));
                reference_label(row[4]);
                assert_ne!(row[4], "~");
                case.steps.push(Step::Discover {
                    stage: row[1].into(), role: row[2].into(),
                    id: row[3].parse().unwrap(), genotype: row[4].into(),
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
                assert_eq!(row[2], operation);
                if row[2] == "checkHitCorridor" {
                    assert!(row[3] == "~" || corridors.contains(row[3]));
                } else {
                    assert!(["send", "canSend", "move"].contains(&row[2]));
                    validate("boolean", row[3]);
                }
                assert!(
                    case.snapshots
                        .entry(row[1].into())
                        .or_default()
                        .returned
                        .replace((row[2].into(), row[3].into()))
                        .is_none()
                );
                ledger.eligible += 1;
            }
            "X" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[1], last_call);
                assert_eq!(operation, "send");
                match case.name.as_str() {
                    NULL_CLADE_CASE => {
                        assert_eq!((row[2], row[3]), ("java.lang.NullPointerException", "~"))
                    }
                    SEND_FAILURE_CASE => assert_eq!(
                        (row[2], row[3]),
                        (
                            "java.lang.IllegalStateException",
                            "cmVjb3JkaW5nLXNlbmQtZmFpbHVyZQ=="
                        )
                    ),
                    other => panic!("Unattributed Java exception {other}"),
                }
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
            "L" => {
                assert_eq!(row.len(), 9);
                assert_eq!(row[1], last_call);
                assert!(["draw", "world.draw"].contains(&operation.as_str()));
                assert!(["drawLine", "drawRect", "fillRect"].contains(&row[3]));
                let target = &mut case.snapshots.entry(row[1].into()).or_default().paint;
                assert_eq!(row[2].parse::<usize>().unwrap(), target.len());
                target.push(PaintCommand {
                    operation: row[3].into(),
                    coordinates: [
                        row[4].parse().unwrap(),
                        row[5].parse().unwrap(),
                        row[6].parse().unwrap(),
                        row[7].parse().unwrap(),
                    ],
                    argb: row[8].parse::<i32>().unwrap() as u32,
                });
                ledger.eligible += 1;
            }
            "P" => {
                assert_eq!(row.len(), 2);
                assert!(
                    row[1] == "initial"
                        || row[1] == last_call
                        || row[1] == format!("before-step-{calls}")
                );
                stage = row[1].into();
                let snapshot = case.snapshots.entry(stage.clone()).or_default();
                assert!(snapshot.suffix.is_none());
                snapshot.roles = known.clone();
                snapshot.corridor_roles = corridors.clone();
                snapshot.connection_roles = connections.clone();
                snapshot.incoming_roles = incoming.clone();
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
                    "travellingOrganism" => {
                        if row[5] != "~" {
                            assert!(known.contains(row[5]));
                        }
                    }
                    "connection" => assert!(connections.contains(row[5])),
                    "world" => assert_eq!(row[5], "present"),
                    "canSendOrganism" => validate("boolean", row[5]),
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
                ledger.eligible += 1;
            }
            "IN" => {
                assert_eq!(family, "world-rules");
                assert_eq!(row.len(), 6);
                assert_eq!(row[1], stage);
                assert!(incoming.contains(row[2]));
                assert_eq!(row[4], INCOMING_SCHEMA.iter().find(|(key, _)| *key == row[3]).unwrap().1);
                match row[3] {
                    "_reference" => assert_eq!(row[2], row[5]),
                    "bounds" => { rectangle_input(row[5]); }
                    "pendingOrganisms" => {
                        for label in role_list(row[5]) {
                            reference_label(label);
                            assert_ne!(label, "~");
                        }
                    }
                    "nextOrganism" => assert!(row[5] == "~" || known.contains(row[5])),
                    "world" => assert_eq!(row[5], "present"),
                    "travellingOrganism" | "connection" => assert_eq!(row[5], "~"),
                    _ => unreachable!(),
                }
                insert_unique(case.snapshots.get_mut(&stage).unwrap().incoming.entry(row[2].into()).or_default(), row[3], row[5]);
                ledger.eligible += 1;
            }
            "ID" => {
                assert_eq!(family, "world-rules");
                assert_eq!(row.len(), 4);
                assert_eq!(row[1], stage);
                let paths = owner_paths(&known, &incoming, &corridors, &connections);
                assert!(paths.contains(row[2]), "Unknown owner path {}", row[2]);
                assert!(!row[3].is_empty() && row[3] != "~");
                insert_unique(&mut case.snapshots.get_mut(&stage).unwrap().owners, row[2], row[3]);
                ledger.eligible += 1;
            }
            "CC" => {
                assert_eq!(row.len(), 6);
                assert_eq!(row[1], stage);
                assert!(connections.contains(row[2]));
                assert_eq!(
                    row[4],
                    CONNECTION_SCHEMA
                        .iter()
                        .find(|(key, _)| *key == row[3])
                        .unwrap()
                        .1
                );
                match row[3] {
                    "_reference" => assert_eq!(row[2], row[5]),
                    "state" => validate("int", row[5]),
                    "recordedGenotypes" => {
                        for label in role_list(row[5]) {
                            reference_label(label);
                            assert_ne!(label, "~");
                        }
                    }
                    _ => unreachable!(),
                }
                insert_unique(
                    case.snapshots
                        .get_mut(&stage)
                        .unwrap()
                        .connections
                        .entry(row[2].into())
                        .or_default(),
                    row[3],
                    row[5],
                );
                ledger.eligible += 1;
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
                    "outCorridors" => {
                        for role in role_list(row[4]) {
                            assert!(corridors.contains(role));
                        }
                    }
                    "inCorridors" => {
                        if family == "world-rules" {
                            for role in role_list(row[4]) { assert!(incoming.contains(role)); }
                        } else { assert_eq!(row[4], ""); }
                    }
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
                if row[0] == "V" && omitted_world(family).contains(&row[2]) {
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
                assert_eq!(case.snapshots.len(), calls + 1 + paint_calls + global_calls);
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
                    assert_eq!(
                        snapshot
                            .connections
                            .keys()
                            .cloned()
                            .collect::<BTreeSet<_>>(),
                        snapshot.connection_roles
                    );
                    for fields in snapshot.connections.values() {
                        assert_eq!(
                            fields.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                            CONNECTION_SCHEMA.into_iter().map(|(key, _)| key).collect()
                        );
                    }
                    assert_eq!(snapshot.incoming.keys().cloned().collect::<BTreeSet<_>>(), snapshot.incoming_roles);
                    for fields in snapshot.incoming.values() {
                        assert_eq!(fields.keys().map(String::as_str).collect::<BTreeSet<_>>(), INCOMING_SCHEMA.into_iter().map(|(key, _)| key).collect());
                    }
                    assert_eq!(snapshot.owners.keys().cloned().collect::<BTreeSet<_>>(),
                        if family == "world-rules" { owner_paths(&snapshot.roles, &snapshot.incoming_roles, &snapshot.corridor_roles, &snapshot.connection_roles) } else { BTreeSet::new() });
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
            tag => panic!("Unknown outgoing row {tag} at {}", line_number + 1),
        }
    }
    assert!(current.is_none());
    if family == "world-rules" {
        assert_eq!(names, BTreeSet::from([
            "active-pair-genesis-retains-queue-candidate-traveller",
            "global-mixed-sufficient-o2",
            "global-mixed-partial-o2",
            "global-mixed-zero-o2",
        ].map(String::from)));
    }
    assert_eq!(
        ledger,
        expected_ledger(family),
        "Outgoing fixed precomparison ledger"
    );
    assert_eq!(
        ledger.rows,
        ledger.eligible + ledger.input_genotypes + ledger.collaborators + ledger.other
    );
    (cases, ledger)
}

fn omitted_world(family: &str) -> &[&str] {
    if family == "world-rules" { &["_visibleWorld"] } else { &OMITTED_WORLD }
}

fn owner_paths(actors: &BTreeSet<String>, incoming: &BTreeSet<String>, outgoing: &BTreeSet<String>, connections: &BTreeSet<String>) -> BTreeSet<String> {
    let mut paths = BTreeSet::from(["world", "statistics", "members", "inCorridors", "outCorridors"].map(String::from));
    paths.extend(actors.iter().map(|role| format!("actor:{role}:world")));
    for role in incoming {
        paths.extend([format!("incoming:{role}"), format!("pending:{role}"), format!("incoming:{role}:world")]);
    }
    for role in outgoing {
        paths.extend([format!("outgoing:{role}"), format!("outgoing:{role}:world"), format!("outgoing:{role}:connection")]);
    }
    paths.extend(connections.iter().map(|role| format!("connection:{role}")));
    paths
}

fn incoming_label<'a>(reference: &IncomingCorridorRef, corridors: &'a BTreeMap<String, IncomingCorridorRef>) -> &'a str {
    corridors.iter().find_map(|(role, known)| Rc::ptr_eq(reference, known).then_some(role.as_str())).expect("Unknown actual incoming corridor")
}

#[derive(Default)]
struct OwnerObservations {
    identities: BTreeMap<String, Vec<usize>>,
    statistics: Vec<Rc<RefCell<crate::statistics::WorldStatistics>>>,
}

impl OwnerObservations {
    fn label(&mut self, path: &str, pointer: usize, prefix: &str) -> String {
        let identities = self.identities.entry(path.into()).or_default();
        let index = identities.iter().position(|known| *known == pointer).unwrap_or_else(|| {
            identities.push(pointer);
            identities.len() - 1
        });
        format!("{prefix}{index}")
    }

    fn fields(&mut self, world: &World, actors: &BTreeMap<String, OrganismRef>, incoming: &BTreeMap<String, IncomingCorridorRef>, outgoing: &BTreeMap<String, OutgoingCorridorRef>, connections: &BTreeMap<String, ConnectionRef>) -> Fields {
        let mut fields = Fields::new();
        let world_pointer = world.association().identity().unwrap();
        let world_label = self.label("world", world_pointer, "world");
        fields.insert("world".into(), world_label.clone());
        let statistics = world.statistics_ref();
        fields.insert("statistics".into(), self.label("statistics", Rc::as_ptr(&statistics) as usize, "statistics"));
        if !self.statistics.iter().any(|known| Rc::ptr_eq(known, &statistics)) { self.statistics.push(statistics); }
        fields.insert("members".into(), self.label("members", world.organisms.required().unwrap().identity(), "members"));
        fields.insert("inCorridors".into(), self.label("inCorridors", &world.incoming_corridors as *const _ as usize, "incoming-list"));
        fields.insert("outCorridors".into(), self.label("outCorridors", &world.outgoing_corridors as *const _ as usize, "outgoing-list"));
        for (role, actor) in actors {
            let actual = actor.borrow().state.world.identity().unwrap();
            let label = self.label("world", actual, "world");
            fields.insert(format!("actor:{role}:world"), label);
        }
        for (role, corridor) in incoming {
            let pointer = Rc::as_ptr(corridor) as usize;
            let path = format!("incoming:{role}");
            fields.insert(path.clone(), self.label(&path, pointer, &format!("{role}:")));
            // The Java queue container is projected onto its stable Rust corridor owner
            let path = format!("pending:{role}");
            fields.insert(path.clone(), self.label(&path, pointer, &format!("queue:{role}:")));
            // Rust corridor frame calls supply the owning World directly
            fields.insert(format!("incoming:{role}:world"), world_label.clone());
        }
        for (role, corridor) in outgoing {
            let path = format!("outgoing:{role}");
            fields.insert(path.clone(), self.label(&path, Rc::as_ptr(corridor) as usize, &format!("{role}:")));
            fields.insert(format!("outgoing:{role}:world"), world_label.clone());
            let connection = &corridor.borrow().connection;
            let label = connection_label(connection, connections);
            let path = format!("connection:{label}");
            fields.insert(format!("outgoing:{role}:connection"), self.label(&path, Rc::as_ptr(connection) as usize, &format!("{label}:")));
        }
        for (role, connection) in connections {
            let path = format!("connection:{role}");
            fields.insert(path.clone(), self.label(&path, Rc::as_ptr(connection) as usize, &format!("{role}:")));
        }
        fields
    }
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
    reference: &OutgoingCorridorRef,
    corridors: &'a BTreeMap<String, OutgoingCorridorRef>,
) -> &'a str {
    corridors
        .iter()
        .find_map(|(role, known)| Rc::ptr_eq(reference, known).then_some(role.as_str()))
        .expect("Unknown actual outgoing corridor")
}

fn connection_label<'a>(
    reference: &ConnectionRef,
    connections: &'a BTreeMap<String, ConnectionRef>,
) -> &'a str {
    connections
        .iter()
        .find_map(|(role, known)| Rc::ptr_eq(reference, known).then_some(role.as_str()))
        .expect("Unknown actual connection owner")
}

fn genome_label<'a>(reference: &GenomeRef, genomes: &'a BTreeMap<String, GenomeRef>) -> &'a str {
    genomes
        .iter()
        .find_map(|(role, known)| Rc::ptr_eq(reference, known).then_some(role.as_str()))
        .expect("Unknown actual admitted genotype")
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

fn paint_commands(commands: Vec<RuntimeDrawCommand>) -> Vec<PaintCommand> {
    commands
        .into_iter()
        .map(|command| match command {
            RuntimeDrawCommand::Outline { bounds, argb } => PaintCommand {
                operation: "drawRect".into(),
                coordinates: [bounds.x, bounds.y, bounds.width, bounds.height],
                argb,
            },
            RuntimeDrawCommand::Fill { bounds, argb } => PaintCommand {
                operation: "fillRect".into(),
                coordinates: [bounds.x, bounds.y, bounds.width, bounds.height],
                argb,
            },
            RuntimeDrawCommand::Line(line) => {
                let coordinates = [line.x1, line.y1, line.x2, line.y2].map(|value| {
                    let integer = value as i32;
                    assert_eq!(
                        value,
                        f64::from(integer),
                        "Actual draw coordinates must retain exact source integers"
                    );
                    integer
                });
                assert_eq!(line.rgb & 0xff00_0000, 0);
                PaintCommand {
                    operation: "drawLine".into(),
                    coordinates,
                    argb: line.rgb | (u32::from(line.alpha) << 24),
                }
            }
        })
        .collect()
}

type Queues = BTreeMap<String, Vec<(u64, GenomeRef)>>;

fn queue_references(connections: &BTreeMap<String, ConnectionRef>) -> Queues {
    connections
        .iter()
        .map(|(label, connection)| {
            (
                label.clone(),
                connection
                    .borrow()
                    .pending
                    .entries()
                    .map(|entry| (entry.ticket, Rc::clone(entry.genome())))
                    .collect(),
            )
        })
        .collect()
}

fn observe_admissions(
    before: &Queues,
    connections: &BTreeMap<String, ConnectionRef>,
    histories: &mut BTreeMap<String, Vec<GenomeRef>>,
) {
    for (label, current) in queue_references(connections) {
        let prior = before.get(&label).map_or(&[][..], Vec::as_slice);
        assert!(
            current.len() >= prior.len(),
            "A source K operation unexpectedly completed an owned transfer"
        );
        for ((old_ticket, old), (new_ticket, new)) in prior.iter().zip(&current) {
            assert_eq!(old_ticket, new_ticket);
            assert!(Rc::ptr_eq(old, new), "Existing queue identity changed");
        }
        histories.entry(label).or_default().extend(
            current
                .into_iter()
                .skip(prior.len())
                .map(|(_, genome)| genome),
        );
    }
}

fn compare_snapshot(
    case: &Case,
    name: &str,
    world: &World,
    actors: &BTreeMap<String, OrganismRef>,
    corridors: &BTreeMap<String, OutgoingCorridorRef>,
    connections: &BTreeMap<String, ConnectionRef>,
    histories: &BTreeMap<String, Vec<GenomeRef>>,
    incoming: &BTreeMap<String, IncomingCorridorRef>,
    owners: &mut OwnerObservations,
    genomes: &mut BTreeMap<String, GenomeRef>,
    returned: &mut BTreeMap<String, (String, String)>,
    paints: &mut BTreeMap<String, Vec<PaintCommand>>,
    failures: &mut BTreeMap<String, OutgoingFailure>,
    errors: &mut Vec<String>,
) {
    let target = &case.snapshots[name];
    assert_eq!(
        actors.keys().cloned().collect::<BTreeSet<_>>(),
        target.roles
    );
    assert_eq!(
        corridors.keys().cloned().collect::<BTreeSet<_>>(),
        target.corridor_roles
    );
    assert_eq!(
        connections.keys().cloned().collect::<BTreeSet<_>>(),
        target.connection_roles
    );
    assert_eq!(incoming.keys().cloned().collect::<BTreeSet<_>>(), target.incoming_roles);
    if case.mode == "world-rules" {
        assert_eq!(owners.fields(world, actors, incoming, corridors, connections), target.owners, "{} {name} actual owner identities", case.name);
    }
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
                panic!("Statistics genotype schema changed")
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
    for (role, expected) in &target.organisms {
        let actor = actors[role].borrow();
        assert!(
            actor.creation_complete || case.mode == "world-rules",
            "Every outgoing actor must originate from a successful real paste"
        );
        let mut actual = organism_fields(&View::from(&*actor));
        if case.mode == "world-rules" && !actor.creation_complete {
            let (_, schema) = primitive_reference_schemas();
            // Match the existing incoming allocation projection before a successful paste
            for (field, kind) in schema {
                if kind.ends_with("[]") { actual.insert(field, "~".into()); }
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
    let (actual_world, mut instances) = world_views(world, actors);
    instances.insert("_corridorexists".into(), world.corridors_exist.get().to_string());
    instances.insert(
        "outCorridors".into(),
        array(
            world.outgoing_corridors.borrow()
                .iter()
                .map(|reference| corridor_label(reference, corridors)),
        ),
    );
    if case.mode == "world-rules" {
        instances.insert("inCorridors".into(), array(world.incoming_corridors.borrow().iter().map(|reference| incoming_label(reference, incoming))));
    }
    compare_fields(&case.name, name, "W", &actual_world, &target.world, errors);
    let expected_instances = target
        .instances
        .iter()
        .filter(|(field, _)| !omitted_world(&case.mode).contains(&field.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    compare_fields(
        &case.name,
        name,
        "V",
        &instances,
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
                "connection".into(),
                connection_label(&current.connection, connections).into(),
            ),
            (
                "travellingOrganism".into(),
                current
                    .traveller
                    .as_ref()
                    .map_or("~", |actor| actor_role(actor, actors))
                    .into(),
            ),
            // The source back-reference has one explicit logical World owner
            ("world".into(), "present".into()),
            ("canSendOrganism".into(), current.can_send().to_string()),
        ]);
        compare_fields(
            &case.name,
            &format!("{name}:{role}"),
            "F",
            &actual,
            expected,
            errors,
        );
    }
    for (role, expected) in &target.incoming {
        let current = incoming[role].borrow();
        let actual = Fields::from([
            ("_reference".into(), incoming_label(&incoming[role], incoming).into()),
            ("bounds".into(), rectangle(current.bounds)),
            ("pendingOrganisms".into(), array(current.pending.iter().map(|code| code.as_ref().map_or("~", |code| genome_label(code, genomes))))),
            ("nextOrganism".into(), current.candidate.as_ref().map_or("~", |actor| actor_role(actor, actors)).into()),
            ("world".into(), "present".into()),
            ("travellingOrganism".into(), "~".into()),
            ("connection".into(), "~".into()),
        ]);
        compare_fields(&case.name, &format!("{name}:{role}"), "IN", &actual, expected, errors);
    }
    for (role, expected) in &target.connections {
        let actual = Fields::from([
            (
                "_reference".into(),
                connection_label(&connections[role], connections).into(),
            ),
            ("state".into(), connections[role].borrow().state.to_string()),
            // Observer history uses only actual admitted or rejected references, never production queue contents as source history
            (
                "recordedGenotypes".into(),
                array(
                    histories[role]
                        .iter()
                        .map(|genome| genome_label(genome, genomes)),
                ),
            ),
        ]);
        compare_fields(
            &case.name,
            &format!("{name}:{role}"),
            "CC",
            &actual,
            expected,
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
    if !name.starts_with("before-") {
        assert_eq!(
            tree_view(world, actors),
            target.insertions,
            "{} {name} actual stored envelopes",
            case.name
        );
    } else {
        assert!(
            target.insertions.is_empty(),
            "The producer emits no stored-envelope map before paint"
        );
    }
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
        "{} {name} actual method result",
        case.name
    );
    assert_eq!(
        paints.remove(name).unwrap_or_default(),
        target.paint,
        "{} {name} actual paint commands",
        case.name
    );
    match &target.exception {
        None => assert!(
            failures.remove(name).is_none(),
            "Unexpected typed outgoing rejection"
        ),
        Some(_) => {
            let expected = match case.name.as_str() {
                NULL_CLADE_CASE => OutgoingFailure::MissingClade,
                SEND_FAILURE_CASE => OutgoingFailure::QueueFull,
                _ => unreachable!(),
            };
            assert_eq!(
                failures.remove(name),
                Some(expected),
                "{} {name} separately attributed Rust typed rejection",
                case.name
            );
        }
    }
    assert_eq!(
        Some(suffix(&world.random_snapshot())),
        target.suffix,
        "{} {name} actual RNG suffix",
        case.name
    );
}
fn apply_input(
    case: &Case,
    target: &str,
    kind: &str,
    value: &str,
    world: &mut World,
    actors: &BTreeMap<String, OrganismRef>,
    corridors: &BTreeMap<String, OutgoingCorridorRef>,
    connections: &BTreeMap<String, ConnectionRef>,
    incoming: &BTreeMap<String, IncomingCorridorRef>,
    prefills: &mut BTreeMap<String, u64>,
    genomes: &mut BTreeMap<String, GenomeRef>,
    errors: &mut Vec<String>,
) {
    let path: Vec<_> = target.split(':').collect();
    match path.as_slice() {
        ["actor", label, "_growthRatio"] => {
            assert_eq!(kind, "int");
            let ratio = value.parse::<i32>().unwrap();
            assert!((1..=17).contains(&ratio));
            actors[*label].borrow_mut().body.growth_ratio = ratio;
        }
        ["utils", "USER_NAME"] => {
            assert_eq!((kind, value), ("String", "oracle"));
            assert!(connections.is_empty());
            world.session.0.parameters.borrow_mut().outgoing = Some(OutgoingConfig {
                user_name: value.into(),
                pending_per_connection: std::num::NonZeroUsize::new(1).unwrap(),
            });
        }
        ["connection", label, "state"] => {
            assert_eq!(kind, "int");
            connections[*label].borrow_mut().state = value.parse().unwrap();
        }
        ["connection", label, "failOnSend"] => {
            assert_eq!(kind, "boolean");
            if value.parse::<bool>().unwrap() {
                assert!(!prefills.contains_key(*label));
                let mut connection = connections[*label].borrow_mut();
                assert!(connection.pending.entries().next().is_none());
                // This explicit boundary projection fills the real queue with supplied input g2
                let ticket = connection
                    .pending
                    .try_enqueue(Rc::clone(&genomes["g2"]))
                    .unwrap();
                assert!(prefills.insert((*label).into(), ticket).is_none());
            } else {
                let ticket = prefills
                    .remove(*label)
                    .expect("Missing explicit queue sentinel");
                let actual = connections[*label]
                    .borrow_mut()
                    .pending
                    .finish(ticket, TransferOutcome::Cancelled)
                    .expect("Real sentinel completion failed");
                assert!(
                    Rc::ptr_eq(&actual, &genomes["g2"]),
                    "Queue completion returned a different input object"
                );
            }
        }
        ["incoming", label, "bounds"] => {
            assert_eq!(case.mode, "world-rules");
            assert_eq!(kind, "Rectangle");
            incoming[*label].borrow_mut().bounds = rectangle_input(value);
        }
        ["corridor", label, "bounds"] => {
            assert_eq!(kind, "Rectangle");
            corridors[*label].borrow_mut().bounds = rectangle_input(value);
        }
        ["world", "inCorridors"] => {
            assert_eq!(case.mode, "world-rules");
            assert_eq!(kind, "List");
            *world.incoming_corridors.borrow_mut() = role_list(value).into_iter().map(|role| Rc::clone(&incoming[role])).collect();
        }
        ["world", "outCorridors"] => {
            assert_eq!(kind, "List");
            *world.outgoing_corridors.borrow_mut() = role_list(value)
                .into_iter()
                .map(|role| Rc::clone(&corridors[role]))
                .collect();
        }
        ["world", "_corridorexists"] => {
            assert_eq!(kind, "boolean");
            world.corridors_exist.set(value.parse().unwrap());
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
    assert!(["outgoing", "outgoing-frames", "world-rules"].contains(&case.mode.as_str()));
    let mut world = if case.mode == "world-rules" {
        let mut source_parameters = parameters(&case.settings);
        let lifecycle = source_parameters.lifecycle;
        source_parameters.lifecycle = crate::LifecycleParams::default();
        let world = World::empty_with_parameters(WorldConfig {
            seed: case.seed,
            width: integer(&case.settings, "WORLD_WIDTH").try_into().unwrap(),
            height: integer(&case.settings, "WORLD_HEIGHT").try_into().unwrap(),
            initial_attempts: integer(&case.settings, "INITIAL_ORGANISMS").try_into().unwrap(),
        }, source_parameters).unwrap();
        world.session.0.parameters.borrow_mut().lifecycle = lifecycle;
        world
    } else {
        assert_eq!(integer(&case.settings, "INITIAL_ORGANISMS"), 0);
        empty_reference_world(&case.settings, case.seed)
    };
    world.oxygen.set(0.0);
    world.carbon_dioxide.set(0.0);
    world.methane.set(0.0);
    let mut actors = BTreeMap::<String, OrganismRef>::new();
    let mut corridors = BTreeMap::<String, OutgoingCorridorRef>::new();
    let mut connections = BTreeMap::<String, ConnectionRef>::new();
    let mut incoming = BTreeMap::<String, IncomingCorridorRef>::new();
    let mut owners = OwnerObservations::default();
    let mut latest_call = String::new();
    let mut histories = BTreeMap::<String, Vec<GenomeRef>>::new();
    let mut prefills = BTreeMap::<String, u64>::new();
    let mut genomes = BTreeMap::<String, GenomeRef>::new();
    let mut returned = BTreeMap::<String, (String, String)>::new();
    let mut paints = BTreeMap::<String, Vec<PaintCommand>>::new();
    let mut failures = BTreeMap::<String, OutgoingFailure>::new();
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
                    &connections,
                    &incoming,
                    &mut prefills,
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
                let before = queue_references(&connections);
                let result = (|| -> Result<(), WorldError> {
                    match operation.as_str() {
                        "incoming.new" => { assert!(incoming.insert(target.clone(), world.new_incoming_corridor()).is_none()); }
                        "incoming.receive" => incoming[target].borrow_mut().receive(Some(Rc::clone(&genomes[argument]))),
                        "incoming.frame" => world.frame_incoming_corridor(&incoming[target])?,
                        "world.addCorridors" => world.add_corridors(Rc::clone(&incoming[target]), Rc::clone(&corridors[argument])),
                        "genesis" => world.genesis()?,
                        "killAll" => world.kill_all()?,
                        "disperseAll" => world.disperse_all()?,
                        "connection.new" => {
                            assert!(
                                connections
                                    .insert(target.clone(), world.new_connection_core(51)?)
                                    .is_none()
                            );
                            assert!(histories.insert(target.clone(), Vec::new()).is_none());
                        }
                        "corridor.new" => {
                            let actual =
                                world.new_outgoing_corridor(Rc::clone(&connections[argument]));
                            assert!(corridors.insert(target.clone(), actual).is_none());
                        }
                        "send" => {
                            let actor = &actors[argument];
                            let result = world.send_outgoing(
                                &corridors[target],
                                actor,
                                &mut actor.borrow_mut(),
                            )?;
                            assert!(
                                returned
                                    .insert(stage.clone(), (operation.clone(), result.to_string()))
                                    .is_none()
                            );
                        }
                        "canSend" => {
                            let result = corridors[target].borrow().can_send();
                            assert!(
                                returned
                                    .insert(stage.clone(), (operation.clone(), result.to_string()))
                                    .is_none()
                            );
                        }
                        "checkHitCorridor" => {
                            let actual = world.check_outgoing(&actors[target].borrow());
                            let label = actual
                                .as_ref()
                                .map_or("~", |reference| corridor_label(reference, &corridors));
                            assert!(
                                returned
                                    .insert(stage.clone(), (operation.clone(), label.into()))
                                    .is_none()
                            );
                        }
                        "move" => {
                            let actor = &actors[target];
                            let result = world.move_actor_frame(actor, &mut actor.borrow_mut())?;
                            assert!(
                                returned
                                    .insert(stage.clone(), (operation.clone(), result.to_string()))
                                    .is_none()
                            );
                        }
                        "die" => world.die(&mut actors[target].borrow_mut(), None).unwrap(),
                        "time" => assert!(world.single_step()?),
                        "draw" => {
                            assert!(world.paused());
                            let actual = paint_commands(world.draw_outgoing(&corridors[target])?);
                            assert!(paints.insert(stage.clone(), actual).is_none());
                        }
                        "world.draw" => {
                            assert!(world.paused());
                            let actual = paint_commands(world.advance_presentation()?);
                            assert!(paints.insert(stage.clone(), actual).is_none());
                        }
                        other => panic!("Unknown actual outgoing call {other}"),
                    }
                    Ok(())
                })();
                observe_admissions(&before, &connections, &mut histories);
                if let Err(error) = result {
                    let WorldError::Outgoing(error) = error else {
                        panic!("Unexpected non-outgoing source comparison failure: {error:?}")
                    };
                    if error.kind == OutgoingFailure::QueueFull {
                        let connection = error
                            .connection()
                            .expect("Actual queue rejection must retain its real connection");
                        let attempted = error
                            .attempted()
                            .expect("Actual queue rejection must retain its attempted genotype");
                        let label = connection_label(connection, &connections);
                        histories.get_mut(label).unwrap().push(Rc::clone(attempted));
                    }
                    assert!(failures.insert(stage.clone(), error.kind).is_none());
                }
            }
            Step::Discover { stage, role, id, genotype } => {
                assert_eq!(&latest_call, stage);
                let actual = world.organisms.borrow().iter()
                    .find(|actor| !actors.values().any(|known| Rc::ptr_eq(actor, known))).cloned()
                    .or_else(|| incoming.values().find_map(|corridor| corridor.borrow().candidate.as_ref().filter(|actor| !actors.values().any(|known| Rc::ptr_eq(actor, known))).cloned()))
                    .unwrap_or_else(|| panic!("{} {stage}: no actual discovered object {role}", case.name));
                assert_eq!(actual.borrow().id, i64::from(*id));
                bind_reference(genotype, actual.borrow().genome.as_ref(), &mut genomes, &format!("{} {stage} D {role}", case.name));
                assert!(actors.insert(role.clone(), actual).is_none());
            }
            Step::Snapshot(name) => {
                for member in world.organisms.borrow().clone() {
                    actor_role(&member, &actors);
                }
                for corridor in corridors.values() {
                    if let Some(actor) = &corridor.borrow().traveller {
                        actor_role(actor, &actors);
                    }
                }
                for corridor in incoming.values() {
                    if let Some(actor) = &corridor.borrow().candidate { actor_role(actor, &actors); }
                }
                compare_snapshot(
                    case,
                    name,
                    &world,
                    &actors,
                    &corridors,
                    &connections,
                    &histories,
                    &incoming,
                    &mut owners,
                    &mut genomes,
                    &mut returned,
                    &mut paints,
                    &mut failures,
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
    assert!(returned.is_empty() && paints.is_empty() && failures.is_empty() && prefills.is_empty());
}

fn compare_fixture(source: &str, family: &str) {
    let (cases, ledger) = parse(source, family);
    assert_eq!(ledger, expected_ledger(family));
    let mut errors = Vec::new();
    for case in &cases {
        replay(case, &mut errors);
    }
    assert!(
        errors.is_empty(),
        "Outgoing mismatches: {}",
        errors.join("\n")
    );
    println!(
        "OUTGOING_MATCH family={family} rows={} eligible={} input_GN={} collaborators={} other={} logical_owner_projections={} controlled_boundary_history_rows={} source_exception_metadata={}",
        ledger.rows,
        ledger.eligible,
        ledger.input_genotypes,
        ledger.collaborators,
        ledger.other,
        ledger.tags["P"] + ledger.tags["F"] / 6,
        ledger.tags["CC"] / 3,
        ledger.tags.get("X").copied().unwrap_or(0)
    );
    if family == "world-rules" {
        println!("WORLD_RULES_EXTENSIONS incoming_rows={} owner_rows={} pending_container_projection=logical-corridor-owner", ledger.tags["IN"], ledger.tags["ID"]);
    }
}

#[test]
fn core_outgoing_methods_and_paint_commands_match_original_java() {
    compare_fixture(include_str!("../tests/fixtures/outgoing.tsv"), "core");
}

#[test]
fn outgoing_frame_removal_and_frozen_aliases_match_original_java() {
    compare_fixture(
        include_str!("../tests/fixtures/outgoing_frames.tsv"),
        "frames",
    );
}

#[test]
fn checker_rejects_changed_source_rng_without_using_it_as_construction_input() {
    let (mut cases, _) = parse(include_str!("../tests/fixtures/outgoing.tsv"), "core");
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

#[test]
fn checker_rejects_changed_paint_argb_without_using_it_as_presentation_input() {
    let (mut cases, _) = parse(include_str!("../tests/fixtures/outgoing.tsv"), "core");
    let case = cases
        .iter_mut()
        .find(|case| case.name == "paint15-reaches16-before-next-paint-releases-capacity")
        .unwrap();
    case.snapshots.get_mut("step-3").unwrap().paint[0].argb ^= 1;
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        replay(case, &mut Vec::new())
    }))
    .expect_err("Corrupted paint ARGB was accepted");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(
        message.contains("actual paint commands"),
        "Wrong rejection: {message}"
    );
}


fn reject_world_rule_corruption(action: impl FnOnce(), expected: &str) {
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action))
        .expect_err("Corrupted world-rule evidence was accepted");
    let message = failure.downcast_ref::<String>().map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied()).unwrap();
    assert!(message.contains(expected), "Wrong world-rule rejection: {message}");
}

#[test]
fn actual_java_global_world_rules_and_active_genesis_preserve_complete_state() {
    let source = include_str!("../tests/fixtures/world_rules.tsv");
    compare_fixture(source, "world-rules");
    for field in ["_O2", "_CO2"] {
        let (mut cases, _) = parse(source, "world-rules");
        let case = cases.iter_mut().find(|case| case.name == "global-mixed-sufficient-o2").unwrap();
        let snapshot = case.snapshots.get_mut("step-3").unwrap();
        let original = &snapshot.world[field];
        let changed = format!("{:016x}", u64::from_str_radix(original, 16).unwrap() ^ 1);
        snapshot.world.insert(field.into(), changed);
        reject_world_rule_corruption(|| replay(case, &mut Vec::new()), field);
    }
    for field in ["pendingOrganisms", "nextOrganism", "travellingOrganism"] {
        let (mut cases, _) = parse(source, "world-rules");
        let case = cases.iter_mut().find(|case| case.name == "active-pair-genesis-retains-queue-candidate-traveller").unwrap();
        let snapshot = case.snapshots.get_mut("before-step-9").unwrap();
        match field {
            "pendingOrganisms" => {
                assert_eq!(snapshot.incoming["i0"][field], "g0,g4");
                snapshot.incoming.get_mut("i0").unwrap().insert(field.into(), "g4,g0".into());
            }
            "nextOrganism" => {
                assert_eq!(snapshot.incoming["i0"][field], "arrival0");
                snapshot.incoming.get_mut("i0").unwrap().insert(field.into(), "selector".into());
            }
            "travellingOrganism" => {
                assert_eq!(snapshot.corridors["c0"][field], "victim");
                snapshot.corridors.get_mut("c0").unwrap().insert(field.into(), "selector".into());
            }
            _ => unreachable!(),
        }
        reject_world_rule_corruption(|| replay(case, &mut Vec::new()), field);
    }
    for field in ["lastDeadBeing", "deathSum"] {
        let (mut cases, _) = parse(source, "world-rules");
        let case = cases.iter_mut().find(|case| case.name == "global-mixed-sufficient-o2").unwrap();
        let snapshot = case.snapshots.get_mut("step-1").unwrap();
        let value = &mut snapshot.statistics.get_mut(field).unwrap().1;
        if field == "lastDeadBeing" {
            *value = if value.as_str() == "g0" { "g1" } else { "g0" }.into();
        } else {
            *value = value.parse::<i64>().unwrap().wrapping_add(1).to_string();
        }
        reject_world_rule_corruption(|| replay(case, &mut Vec::new()), field);
    }
    let (mut cases, _) = parse(source, "world-rules");
    let case = cases.iter_mut().find(|case| case.name == "global-mixed-zero-o2").unwrap();
    let snapshot = case.snapshots.get_mut("step-5").unwrap();
    let original = snapshot.suffix.unwrap();
    snapshot.suffix = Some((original.0.wrapping_add(1), original.1, original.2));
    reject_world_rule_corruption(|| replay(case, &mut Vec::new()), "step-5 actual RNG suffix");

    let first_rng = source.lines().find(|line| line.starts_with("R\t")).unwrap();
    let missing = source.replacen(&format!("{first_rng}\n"), "", 1);
    reject_world_rule_corruption(|| { parse(&missing, "world-rules"); }, "assertion");
    let first_setting = source.lines().find(|line| line.starts_with("U\t")).unwrap();
    let duplicate = source.replacen(first_setting, &format!("{first_setting}\n{first_setting}"), 1);
    reject_world_rule_corruption(|| { parse(&duplicate, "world-rules"); }, "Duplicate field");
    let unknown = source.replacen("U\t", "UNKNOWN\t", 1);
    reject_world_rule_corruption(|| { parse(&unknown, "world-rules"); }, "Unknown outgoing row");
}
