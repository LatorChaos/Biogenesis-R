// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay frozen caller observations without constructing bodies from newborn or candidate outputs

use super::death_oracle_tests::{
    apply_reference_input, compare_genomes, statistics_schemas, validate, validate_genome,
    world_schema,
};
use super::maintenance_oracle_tests::{
    WORLD_EXCLUSIONS, actor_role, bind_reference, instance_schema, query_roles, reference_label,
    role_list, world_views,
};
use super::*;
use crate::ColorKind;
use crate::organism::{GenomeRef, OrganismRef};
use crate::statistics::tests::{Observation, assert_observation, observed_field, observed_getter};
use std::rc::Rc;

#[path = "color_matrix_full_oracle_tests.rs"]
mod color_matrix_full_oracle_tests;

#[path = "stateful_color_oracle_tests.rs"]
mod stateful_color_oracle_tests;

const STATEFUL_MODE: &str = "stateful-color";

const FULL_MATRIX_MODE: &str = "color-matrix-full";
const MATRIX_COLORS: &str = "RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW";

fn matrix_mode(mode: &str) -> bool {
    mode == "color-matrix" || mode == FULL_MATRIX_MODE
}

fn matrix_pair(name: &str) -> (&str, &str) {
    let (actor, target) = name.strip_prefix("pair-").unwrap().split_once('-').unwrap();
    for color in [actor, target] {
        assert!(
            MATRIX_COLORS
                .split_ascii_whitespace()
                .any(|known| color == known)
        );
    }
    (actor, target)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ReturnValue {
    Boolean(bool),
    Actor(Option<String>),
}

#[derive(Default)]
struct Snapshot {
    organisms: BTreeMap<String, Fields>,
    references: BTreeMap<(String, String), String>,
    world: Fields,
    instances: Fields,
    statistics: BTreeMap<String, (String, String)>,
    getters: BTreeMap<String, (String, String)>,
    genotypes: BTreeMap<String, Stage>,
    scopes: Fields,
    queries: Fields,
    insertions: Vec<(String, IntRect)>,
    returned: Option<ReturnValue>,
    suffix: Option<(i64, i32, u64)>,
    rejected: BTreeSet<String>,
    members: BTreeSet<String>,
    tree_members: BTreeSet<String>,
    roles: BTreeSet<String>,
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
    Candidate {
        stage: String,
        role: String,
        available: bool,
    },
    Snapshot(String),
}

struct CallerCase {
    name: String,
    mode: String,
    seed: i64,
    settings: Fields,
    input_genotypes: BTreeMap<String, Stage>,
    snapshots: BTreeMap<String, Snapshot>,
    steps: Vec<Step>,
    draw_commands: BTreeMap<String, Vec<[i32; 5]>>,
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Ledger {
    rows: usize,
    cases: usize,
    snapshots: usize,
    observations: usize,
    supplied_genotypes: usize,
    organism_collaborators: usize,
    world_collaborators: usize,
    selection_metadata: usize,
    scope_metadata: usize,
    rejected_candidates: usize,
    rejected_genotypes: usize,
    inputs_and_commands: usize,
    calls: BTreeMap<String, usize>,
}

fn expected_ledger(mode: &str) -> Ledger {
    match mode {
        STATEFUL_MODE => stateful_color_oracle_tests::declared_ledger(),
        "player-actions" => Ledger {
            rows: 194_538,
            cases: 34,
            snapshots: 184,
            observations: 173_682,
            supplied_genotypes: 12_160,
            organism_collaborators: 1_088,
            world_collaborators: 552,
            selection_metadata: 37,
            scope_metadata: 926,
            inputs_and_commands: 6_093,
            calls: [
                ("feed", 9),
                ("weaken", 7),
                ("kill", 21),
                ("revive", 9),
                ("disperse", 8),
                ("reproduce", 5),
                ("rejuvenate", 3),
                ("build_tree", 3),
                ("time", 2),
                ("pick", 8),
            ]
            .into_iter()
            .map(|(name, count)| (name.to_owned(), count))
            .collect(),
            ..Ledger::default()
        },
        "draw" => Ledger {
            rows: 106_904,
            cases: 27,
            snapshots: 97,
            observations: 84_626,
            supplied_genotypes: 16_740,
            organism_collaborators: 198,
            world_collaborators: 388,
            scope_metadata: 491,
            inputs_and_commands: 4_461,
            calls: [("draw", 32), ("bounds", 4), ("die", 1), ("genesis", 1)]
                .into_iter()
                .map(|(name, count)| (name.to_owned(), count))
                .collect(),
            ..Ledger::default()
        },
        "color-matrix" => Ledger {
            rows: 394_082,
            cases: 46,
            snapshots: 368,
            observations: 364_136,
            supplied_genotypes: 15_640,
            organism_collaborators: 2_392,
            world_collaborators: 1_104,
            selection_metadata: 46,
            scope_metadata: 1_932,
            rejected_candidates: 0,
            rejected_genotypes: 0,
            inputs_and_commands: 8_832,
            calls: [
                ("symmetric", 92),
                ("bounds", 92),
                ("build_tree", 46),
                ("checkHit", 46),
                ("time", 46),
            ]
            .map(|(name, count)| (name.to_owned(), count))
            .into_iter()
            .collect(),
        },
        "frame" => Ledger {
            rows: 99_418,
            cases: 27,
            snapshots: 71,
            observations: 83_557,
            supplied_genotypes: 9_820,
            organism_collaborators: 466,
            world_collaborators: 284,
            selection_metadata: 1,
            scope_metadata: 503,
            rejected_candidates: 0,
            rejected_genotypes: 0,
            inputs_and_commands: 4_787,
            calls: [("time", 39), ("genesis", 4), ("die", 1)]
                .map(|(name, count)| (name.to_owned(), count))
                .into_iter()
                .collect(),
        },
        "contact" => Ledger {
            rows: 222_501,
            cases: 61,
            snapshots: 186,
            observations: 180_816,
            supplied_genotypes: 22_700,
            organism_collaborators: 1_132,
            world_collaborators: 744,
            selection_metadata: 61,
            scope_metadata: 934,
            rejected_candidates: 0,
            rejected_genotypes: 0,
            inputs_and_commands: 16_114,
            calls: [("build_tree", 64), ("contact", 49), ("checkHit", 12)]
                .map(|(name, count)| (name.to_owned(), count))
                .into_iter()
                .collect(),
        },
        "plague-fallow" => Ledger {
            rows: 1_372_793,
            cases: 395,
            snapshots: 1_187,
            observations: 1_125_418,
            supplied_genotypes: 134_580,
            organism_collaborators: 7_342,
            world_collaborators: 4_748,
            selection_metadata: 395,
            scope_metadata: 5_950,
            rejected_candidates: 0,
            rejected_genotypes: 0,
            inputs_and_commands: 94_360,
            calls: [
                ("build_tree", 397),
                ("touchEffects2.PLAGUE", 208),
                ("touchEffects2.FALLOW", 187),
            ]
            .map(|(name, count)| (name.to_owned(), count))
            .into_iter()
            .collect(),
        },
        "cooperative" => Ledger {
            rows: 707_139,
            cases: 206,
            snapshots: 618,
            observations: 584_802,
            supplied_genotypes: 70_800,
            organism_collaborators: 3_768,
            world_collaborators: 2_472,
            selection_metadata: 206,
            scope_metadata: 3_117,
            rejected_candidates: 0,
            rejected_genotypes: 0,
            inputs_and_commands: 41_974,
            calls: [
                ("build_tree", 206),
                ("touchEffects2.MAGENTA", 79),
                ("touchEffects2.MINT", 40),
                ("touchEffects2.LAVENDER", 25),
                ("touchEffects2.ROSE", 17),
                ("touchEffects2.YELLOW", 35),
                ("touchEffects2.BLUE", 10),
            ]
            .map(|(name, count)| (name.to_owned(), count))
            .into_iter()
            .collect(),
        },
        "coral" => Ledger {
            rows: 576_825,
            cases: 169,
            snapshots: 510,
            observations: 479_204,
            supplied_genotypes: 57_780,
            organism_collaborators: 3_086,
            world_collaborators: 2_040,
            selection_metadata: 169,
            scope_metadata: 2_563,
            rejected_candidates: 0,
            rejected_genotypes: 0,
            inputs_and_commands: 31_983,
            calls: [("build_tree", 170), ("touchEffects1", 171)]
                .map(|(name, count)| (name.to_owned(), count))
                .into_iter()
                .collect(),
        },
        "reproduction" => Ledger {
            rows: 327_958,
            cases: 73,
            snapshots: 222,
            observations: 274_872,
            supplied_genotypes: 36_980,
            organism_collaborators: 1_522,
            world_collaborators: 888,
            selection_metadata: 73,
            scope_metadata: 1_235,
            rejected_candidates: 0,
            rejected_genotypes: 0,
            inputs_and_commands: 12_388,
            calls: [
                ("build_tree", 73),
                ("reproduce", 39),
                ("reproduceEarly", 5),
                ("reproduceEarlyPlant", 8),
                ("reproduceEarlyVirus", 4),
                ("reproduceSpore", 5),
                ("reproduceYellow", 5),
                ("reproduceVirus", 4),
                ("reproduceforeignVirus", 4),
                ("blueSpore", 1),
                ("cyanSpore", 1),
            ]
            .map(|(name, count)| (name.to_owned(), count))
            .into_iter()
            .collect(),
        },
        "transformation" => Ledger {
            rows: 221_788,
            cases: 48,
            snapshots: 149,
            observations: 183_284,
            supplied_genotypes: 26_560,
            organism_collaborators: 986,
            world_collaborators: 596,
            selection_metadata: 48,
            scope_metadata: 842,
            rejected_candidates: 790,
            rejected_genotypes: 512,
            inputs_and_commands: 8_170,
            calls: [
                ("build_tree", 48),
                ("transform", 4),
                ("inheritTransformation", 46),
                ("grow", 1),
                ("offset_velocity", 1),
                ("bounds", 1),
            ]
            .map(|(name, count)| (name.to_owned(), count))
            .into_iter()
            .collect(),
        },
        mode => panic!("Unknown caller fixture mode {mode}"),
    }
}

fn world_exclusion(mode: &str, field: &str) -> bool {
    WORLD_EXCLUSIONS.contains(&field)
        && !((matrix_mode(mode) || mode == "player-actions" || mode == STATEFUL_MODE)
            && field == "_corridorexists")
}

fn rectangle_input(value: &str) -> IntRect {
    let numbers: Vec<i32> = value.split(',').map(|item| item.parse().unwrap()).collect();
    assert_eq!(numbers.len(), 4);
    IntRect {
        x: numbers[0],
        y: numbers[1],
        width: numbers[2],
        height: numbers[3],
    }
}

fn operation_argument(operation: &str, argument: &str) {
    match operation {
        "time" | "genesis" | "die" | "draw" | "feed" | "weaken" | "kill" | "revive"
        | "disperse" | "rejuvenate" => assert_eq!(argument, "~"),
        "pick" => {
            let point: Vec<i32> = argument
                .split(',')
                .map(|item| item.parse().unwrap())
                .collect();
            assert_eq!(point.len(), 2);
        }
        "contact" => assert!(["killer", "selector"].contains(&argument)),
        "checkHit" => assert_eq!(argument, "~"),
        "build_tree" => {
            role_list(argument);
        }
        "inheritTransformation" | "bounds" => {
            validate("boolean", argument);
        }
        "touchEffects1"
        | "touchEffects2.BLUE"
        | "touchEffects2.YELLOW"
        | "touchEffects2.ROSE"
        | "touchEffects2.MINT"
        | "touchEffects2.LAVENDER"
        | "touchEffects2.MAGENTA"
        | "touchEffects2.PLAGUE"
        | "touchEffects2.FALLOW" => {
            let arguments: Vec<_> = argument.split(',').collect();
            assert_eq!(arguments.len(), 3);
            assert_eq!(arguments[0], "killer");
            arguments[1].parse::<usize>().unwrap();
            arguments[2].parse::<usize>().unwrap();
        }
        "reproduce"
        | "reproduceEarly"
        | "reproduceEarlyPlant"
        | "reproduceEarlyVirus"
        | "reproduceSpore"
        | "reproduceYellow"
        | "reproduceVirus"
        | "reproduceforeignVirus"
        | "transform"
        | "blueSpore"
        | "cyanSpore"
        | "grow"
        | "offset_velocity"
        | "symmetric" => assert_eq!(argument, "~"),
        operation => panic!("Unknown caller operation {operation}"),
    }
}

fn source_role(role: &str, matrix: bool) {
    if ["selector", "victim", "killer"].contains(&role) {
        return;
    }
    let number = role
        .strip_prefix("newborn")
        .or_else(|| role.strip_prefix("candidate"))
        .or_else(|| matrix.then(|| role.strip_prefix("arrival")).flatten())
        .expect("Expected a source actor identity, not rectangle coordinates");
    number.parse::<usize>().unwrap();
}

fn parse(source: &str, mode: &str) -> (Vec<CallerCase>, Ledger) {
    let (setting_types, organism_types) = primitive_reference_schemas();
    let (statistic_types, getter_types) = statistics_schemas();
    let mut ledger = Ledger::default();
    let mut cases = Vec::new();
    let mut current: Option<CallerCase> = None;
    let mut names = BTreeSet::new();
    let mut known = BTreeSet::new();
    let mut members = BTreeSet::new();
    let mut rejected = BTreeSet::new();
    let mut tree_members = BTreeSet::new();
    let mut call_stage = String::new();
    let mut call_operation = String::new();
    let mut stage = String::new();
    let mut declared_snapshots = BTreeSet::new();
    let mut newborns = 0;
    let mut candidates = 0;
    let mut call_index = 0;
    let mut stateful_family_seen = false;
    let mut stateful_witnesses = Vec::new();
    for (line_number, line) in source.lines().enumerate() {
        assert!(!line.is_empty(), "Empty caller row {}", line_number + 1);
        let row: Vec<_> = line.split('\t').collect();
        ledger.rows += 1;
        if row[0] == "C" {
            assert_eq!(row.len(), 4);
            assert_eq!(
                row[2],
                if mode == FULL_MATRIX_MODE {
                    "color-matrix"
                } else {
                    mode
                }
            );
            assert!(current.is_none() && names.insert(row[1].to_owned()));
            current = Some(CallerCase {
                name: row[1].to_owned(),
                mode: mode.to_owned(),
                seed: row[3].parse().unwrap(),
                settings: Fields::new(),
                input_genotypes: BTreeMap::new(),
                snapshots: BTreeMap::new(),
                steps: Vec::new(),
                draw_commands: BTreeMap::new(),
            });
            known.clear();
            members.clear();
            rejected.clear();
            tree_members.clear();
            call_stage.clear();
            call_operation.clear();
            stage.clear();
            declared_snapshots.clear();
            newborns = 0;
            candidates = 0;
            call_index = 0;
            stateful_family_seen = false;
            stateful_witnesses.clear();
            ledger.cases += 1;
            ledger.inputs_and_commands += 1;
            continue;
        }
        let case = current.as_mut().expect("Caller row outside a case");
        match row[0] {
            "F" => {
                assert_eq!(mode, STATEFUL_MODE);
                assert!(!stateful_family_seen);
                stateful_color_oracle_tests::validate_family(&case.name, &row);
                stateful_family_seen = true;
                ledger.inputs_and_commands += 1;
            }
            "U" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[2], setting_types[row[1]]);
                validate(row[2], row[3]);
                assert!(
                    case.settings
                        .insert(row[1].to_owned(), row[3].to_owned())
                        .is_none()
                );
                ledger.inputs_and_commands += 1;
            }
            "I" => {
                assert_eq!(row.len(), 5);
                assert!(row[1] == "setup" || row[1] == format!("step-{call_index}"));
                if row[3] == "GeneticCode" {
                    assert!(["g0", "g1", "g2", "g3", "g4", "~"].contains(&row[4]));
                } else {
                    validate(row[3], row[4]);
                }
                if row[2].ends_with(":construction") {
                    assert_eq!(row[1], "setup");
                    assert!(known.is_empty());
                    let parts: Vec<_> = row[2].split(':').collect();
                    assert_eq!(parts.len(), 3);
                    assert_eq!(parts[0], "genome");
                    assert!(["g0", "g1", "g2", "g3", "g4"].contains(&parts[1]));
                    assert_eq!(row[3], "String");
                    assert_eq!(
                        row[4],
                        if parts[1] == "g4" {
                            "clone:g0"
                        } else {
                            "explicit"
                        }
                    );
                }
                case.steps.push(Step::Input {
                    stage: row[1].to_owned(),
                    target: row[2].to_owned(),
                    kind: row[3].to_owned(),
                    value: row[4].to_owned(),
                });
                if row[2] == "visible:selected" {
                    if mode == "player-actions" {
                        assert_eq!(row[3], "String");
                        assert!(row[4] == "~" || known.contains(row[4]));
                    } else {
                        assert_eq!((row[3], row[4]), ("String", "selector"));
                    }
                    ledger.selection_metadata += 1;
                } else {
                    ledger.inputs_and_commands += 1;
                }
            }
            "A" => {
                assert_eq!(row.len(), 5);
                assert!(["selector", "victim", "killer"].contains(&row[1]));
                assert!(["g0", "g1", "g2"].contains(&row[2]));
                assert!(known.insert(row[1].to_owned()) && members.insert(row[1].to_owned()));
                case.steps.push(Step::Actor {
                    role: row[1].to_owned(),
                    genotype: row[2].to_owned(),
                    center: [row[3].parse().unwrap(), row[4].parse().unwrap()],
                });
                ledger.inputs_and_commands += 1;
            }
            "K" => {
                assert_eq!(row.len(), 5);
                if mode == STATEFUL_MODE {
                    stateful_color_oracle_tests::validate_call(&case.name, call_index, &row);
                } else if matrix_mode(mode) {
                    let operations = [
                        ("symmetric", "victim", "~"),
                        ("bounds", "victim", "true"),
                        ("symmetric", "killer", "~"),
                        ("bounds", "killer", "true"),
                        ("build_tree", "victim", "members"),
                        ("checkHit", "victim", "~"),
                        ("time", "victim", "~"),
                    ];
                    assert_eq!((row[2], row[3], row[4]), operations[call_index]);
                } else if mode == "player-actions" {
                    let callback = [
                        "feed",
                        "weaken",
                        "kill",
                        "revive",
                        "disperse",
                        "reproduce",
                        "rejuvenate",
                    ]
                    .contains(&row[2]);
                    assert!(callback || ["build_tree", "time", "pick"].contains(&row[2]));
                    assert_eq!(row[3], if callback { "selected" } else { "world" });
                } else if mode == "draw" {
                    assert!(["draw", "bounds", "genesis", "die"].contains(&row[2]));
                    assert!(["victim", "newborn0"].contains(&row[3]));
                    if row[2] == "draw" {
                        assert_eq!(row[3], "victim");
                        assert!(
                            case.draw_commands
                                .insert(row[1].to_owned(), Vec::new())
                                .is_none()
                        );
                    }
                } else if mode == "frame" {
                    assert!(["time", "genesis", "die"].contains(&row[2]));
                    assert_eq!(
                        row[3],
                        if row[2] == "die" {
                            "selector"
                        } else {
                            "victim"
                        }
                    );
                } else {
                    assert_eq!(row[3], "victim");
                    assert!(!["time", "genesis", "die"].contains(&row[2]));
                }
                assert_eq!(row[1], format!("step-{call_index}"));
                call_index += 1;
                if [
                    "feed",
                    "weaken",
                    "kill",
                    "revive",
                    "disperse",
                    "rejuvenate",
                    "pick",
                ]
                .contains(&row[2])
                {
                    assert_eq!(mode, "player-actions");
                }
                if mode == STATEFUL_MODE && row[2].ends_with(".DEFAULT") {
                    stateful_color_oracle_tests::validate_default(&case.name, row[2], row[4]);
                } else {
                    operation_argument(row[2], row[4]);
                }
                if ["contact", "checkHit"].contains(&row[2]) {
                    assert!(mode == "contact" || matrix_mode(mode) || mode == STATEFUL_MODE);
                }
                if row[2] == "symmetric" {
                    assert!(matrix_mode(mode) || mode == STATEFUL_MODE);
                }
                if mode == "contact" {
                    assert!(["build_tree", "contact", "checkHit"].contains(&row[2]));
                }
                if row[2] == "touchEffects1" {
                    assert_eq!(mode, "coral");
                }
                if mode == "coral" {
                    assert!(["build_tree", "touchEffects1"].contains(&row[2]));
                }
                if row[2].starts_with("touchEffects2.") {
                    assert!(["cooperative", "plague-fallow", STATEFUL_MODE].contains(&mode));
                }
                if mode == "plague-fallow" {
                    assert!(
                        ["build_tree", "touchEffects2.PLAGUE", "touchEffects2.FALLOW"]
                            .contains(&row[2])
                    );
                }
                if mode == "cooperative" {
                    assert!(
                        [
                            "build_tree",
                            "touchEffects2.BLUE",
                            "touchEffects2.YELLOW",
                            "touchEffects2.ROSE",
                            "touchEffects2.MINT",
                            "touchEffects2.LAVENDER",
                            "touchEffects2.MAGENTA",
                        ]
                        .contains(&row[2])
                    );
                }
                if mode == "reproduction" {
                    assert!(
                        ![
                            "inheritTransformation",
                            "transform",
                            "grow",
                            "offset_velocity",
                            "bounds"
                        ]
                        .contains(&row[2])
                    );
                }
                if row[2] == "build_tree" && (matrix_mode(mode) || mode == "player-actions") {
                    assert_eq!(row[4], "members");
                    tree_members = members.clone();
                } else if row[2] == "build_tree" {
                    tree_members = role_list(row[4])
                        .iter()
                        .map(|role| {
                            assert!(known.contains(*role));
                            (*role).to_owned()
                        })
                        .collect();
                } else if (mode == "frame"
                    || matrix_mode(mode)
                    || mode == "player-actions"
                    || mode == STATEFUL_MODE)
                    && row[2] == "time"
                {
                    tree_members = members.clone();
                } else if ["frame", "draw"].contains(&mode) && row[2] == "genesis" {
                    members.clear();
                }
                call_stage = row[1].to_owned();
                call_operation = row[2].to_owned();
                case.steps.push(Step::Call {
                    stage: call_stage.clone(),
                    operation: call_operation.clone(),
                    target: row[3].to_owned(),
                    argument: row[4].to_owned(),
                });
                *ledger.calls.entry(row[2].to_owned()).or_default() += 1;
                ledger.inputs_and_commands += 1;
            }
            "L" => {
                assert_eq!(mode, "draw");
                assert_eq!(row.len(), 8);
                assert_eq!(row[1], call_stage);
                assert_eq!(call_operation, "draw");
                let commands = case.draw_commands.get_mut(row[1]).unwrap();
                assert_eq!(row[2].parse::<usize>().unwrap(), commands.len());
                commands.push(std::array::from_fn(|index| row[index + 3].parse().unwrap()));
                ledger.observations += 1;
            }
            "J" => {
                assert_eq!(row.len(), 7);
                assert_eq!(row[1], call_stage);
                assert!(
                    (matrix_mode(mode) || mode == "player-actions" || mode == STATEFUL_MODE)
                        || call_operation == "build_tree"
                        || (mode == "frame" && call_operation == "time")
                );
                assert!(known.contains(row[2]));
                assert!(tree_members.contains(row[2]));
                let bounds = IntRect {
                    x: row[3].parse().unwrap(),
                    y: row[4].parse().unwrap(),
                    width: row[5].parse().unwrap(),
                    height: row[6].parse().unwrap(),
                };
                case.snapshots
                    .entry(row[1].to_owned())
                    .or_default()
                    .insertions
                    .push((row[2].to_owned(), bounds));
                ledger.observations += 1;
            }
            "D" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], call_stage);
                let prefix =
                    if matrix_mode(mode) || mode == "player-actions" || mode == STATEFUL_MODE {
                        "arrival"
                    } else {
                        "newborn"
                    };
                assert_eq!(row[2], format!("{prefix}{newborns}"));
                if mode == "color-matrix" {
                    assert_eq!(row[1], "step-6");
                } else if mode == FULL_MATRIX_MODE {
                    assert!(["step-5", "step-6"].contains(&row[1]));
                }
                newborns += 1;
                assert!(known.insert(row[2].to_owned()) && members.insert(row[2].to_owned()));
                reference_label(row[4]);
                assert_ne!(row[4], "~");
                case.steps.push(Step::Discover {
                    stage: row[1].to_owned(),
                    role: row[2].to_owned(),
                    id: row[3].parse().unwrap(),
                    genotype: row[4].to_owned(),
                });
                ledger.observations += 1;
            }
            "B" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[1], call_stage);
                assert_eq!(call_operation, row[2]);
                let returned = match row[2] {
                    "inheritTransformation" | "contact" => {
                        ReturnValue::Boolean(row[3].parse().unwrap())
                    }
                    "pick" => {
                        assert_eq!(mode, "player-actions");
                        assert!(row[3] == "~" || known.contains(row[3]));
                        ReturnValue::Actor((row[3] != "~").then(|| row[3].to_owned()))
                    }
                    "checkHit" => {
                        if mode == STATEFUL_MODE {
                            assert!(row[3] == "~" || known.contains(row[3]));
                        } else {
                            assert!(["victim", "~"].contains(&row[3]));
                        }
                        ReturnValue::Actor((row[3] != "~").then(|| row[3].to_owned()))
                    }
                    operation => panic!("Unexpected return operation {operation}"),
                };
                assert!(
                    case.snapshots
                        .entry(row[1].to_owned())
                        .or_default()
                        .returned
                        .replace(returned)
                        .is_none()
                );
                ledger.observations += 1;
            }
            "Z" => {
                if mode == STATEFUL_MODE {
                    assert_eq!(row[1], call_stage);
                    assert_eq!(row[1], stage);
                    stateful_color_oracle_tests::validate_witness(&case.name, &row);
                    stateful_witnesses.push((row[1].to_owned(), row[2].to_owned()));
                    ledger.inputs_and_commands += 1;
                } else {
                    assert_eq!(row.len(), 4);
                    assert_eq!(row[1], call_stage);
                    assert_eq!(call_operation, "inheritTransformation");
                    assert_eq!(row[2], format!("candidate{candidates}"));
                    candidates += 1;
                    assert!(known.insert(row[2].to_owned()));
                    assert!(["available", "rejected"].contains(&row[3]));
                    let available = row[3] == "available";
                    assert_eq!(
                        case.snapshots[row[1]].returned,
                        Some(ReturnValue::Boolean(available))
                    );
                    if !available {
                        assert!(rejected.insert(row[2].to_owned()));
                    }
                    case.steps.push(Step::Candidate {
                        stage: row[1].to_owned(),
                        role: row[2].to_owned(),
                        available,
                    });
                    ledger.scope_metadata += 1;
                }
            }
            "P" => {
                assert_eq!(row.len(), 2);
                assert!(
                    row[1] == "initial"
                        || row[1] == call_stage
                        || (["draw", "player-actions"].contains(&mode)
                            && row[1] == format!("before-step-{call_index}"))
                );
                assert!(declared_snapshots.insert(row[1].to_owned()));
                stage = row[1].to_owned();
                let snapshot = case.snapshots.entry(stage.clone()).or_default();
                snapshot.rejected = rejected.clone();
                snapshot.members = members.clone();
                snapshot.tree_members = tree_members.clone();
                snapshot.roles = known.clone();
                case.steps.push(Step::Snapshot(stage.clone()));
                ledger.snapshots += 1;
                ledger.inputs_and_commands += 1;
            }
            "O" => {
                assert_eq!(row.len(), 6);
                assert_eq!(row[1], stage);
                assert!(known.contains(row[2]));
                assert_eq!(row[4], organism_types[row[3]]);
                validate(row[4], row[5]);
                assert!(
                    case.snapshots
                        .get_mut(row[1])
                        .unwrap()
                        .organisms
                        .entry(row[2].to_owned())
                        .or_default()
                        .insert(row[3].to_owned(), row[5].to_owned())
                        .is_none()
                );
                if rejected.contains(row[2]) {
                    ledger.rejected_candidates += 1;
                } else if ["_world", "_visibleWorld"].contains(&row[3]) {
                    assert_eq!(row[5], "present");
                    ledger.organism_collaborators += 1;
                } else {
                    ledger.observations += 1;
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
                        .get_mut(row[1])
                        .unwrap()
                        .references
                        .insert((row[2].to_owned(), row[3].to_owned()), row[4].to_owned())
                        .is_none()
                );
                if rejected.contains(row[2]) {
                    ledger.rejected_candidates += 1;
                } else {
                    ledger.observations += 1;
                }
            }
            "W" | "V" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], stage);
                if row[0] == "W" {
                    assert_eq!(row[3], world_schema()[row[2]]);
                    validate(row[3], row[4]);
                } else {
                    assert_eq!(row[3], instance_schema()[row[2]]);
                    match row[2] {
                        "_organisms" => {
                            for role in role_list(row[4]) {
                                source_role(
                                    role,
                                    matrix_mode(mode)
                                        || mode == "player-actions"
                                        || mode == STATEFUL_MODE,
                                );
                                assert!(
                                    members.contains(role),
                                    "Unknown World actor identity {role}"
                                );
                            }
                            if mode == "frame"
                                || matrix_mode(mode)
                                || mode == "player-actions"
                                || mode == STATEFUL_MODE
                            {
                                members =
                                    role_list(row[4]).into_iter().map(str::to_owned).collect();
                                case.snapshots.get_mut(row[1]).unwrap().members = members.clone();
                            }
                        }
                        "inCorridors" | "outCorridors" => assert_eq!(row[4], ""),
                        "_visibleWorld" | "worldStatistics" => assert_eq!(row[4], "present"),
                        "colDetTree" => {
                            let parts: Vec<_> = row[4].split(',').collect();
                            assert_eq!(parts.len(), 2);
                            assert_eq!(parts[0], "present");
                            assert_eq!(parts[1].parse::<usize>().unwrap(), tree_members.len());
                        }
                        _ => validate(row[3], row[4]),
                    }
                }
                let snapshot = case.snapshots.get_mut(row[1]).unwrap();
                let target = if row[0] == "W" {
                    &mut snapshot.world
                } else {
                    &mut snapshot.instances
                };
                assert!(
                    target
                        .insert(row[2].to_owned(), row[4].to_owned())
                        .is_none()
                );
                if row[0] == "V" && world_exclusion(mode, row[2]) {
                    ledger.world_collaborators += 1;
                } else {
                    ledger.observations += 1;
                }
            }
            "Q" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[1], stage);
                assert!(known.contains(row[2]));
                for role in role_list(row[3]) {
                    source_role(
                        role,
                        matrix_mode(mode) || mode == "player-actions" || mode == STATEFUL_MODE,
                    );
                    assert!(
                        tree_members.contains(role),
                        "Unknown query actor identity {role}"
                    );
                }
                assert!(
                    case.snapshots
                        .get_mut(row[1])
                        .unwrap()
                        .queries
                        .insert(row[2].to_owned(), row[3].to_owned())
                        .is_none()
                );
                if rejected.contains(row[2]) {
                    ledger.rejected_candidates += 1;
                } else {
                    ledger.observations += 1;
                }
            }
            "S" | "T" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], stage);
                let types = if row[0] == "S" {
                    &statistic_types
                } else {
                    &getter_types
                };
                assert_eq!(row[3], types[row[2]]);
                if row[3] == "GeneticCode" {
                    reference_label(row[4]);
                } else {
                    validate(row[3], row[4]);
                }
                let snapshot = case.snapshots.get_mut(row[1]).unwrap();
                let target = if row[0] == "S" {
                    &mut snapshot.statistics
                } else {
                    &mut snapshot.getters
                };
                assert!(
                    target
                        .insert(row[2].to_owned(), (row[3].to_owned(), row[4].to_owned()))
                        .is_none()
                );
                ledger.observations += 1;
            }
            "Y" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[1], stage);
                reference_label(row[2]);
                assert_ne!(row[2], "~");
                assert!(["observable", "rejected-only"].contains(&row[3]));
                assert!(
                    case.snapshots
                        .get_mut(row[1])
                        .unwrap()
                        .scopes
                        .insert(row[2].to_owned(), row[3].to_owned())
                        .is_none()
                );
                ledger.scope_metadata += 1;
            }
            "G" | "N" => {
                assert_eq!(row.len(), if row[0] == "G" { 5 } else { 6 });
                reference_label(row[2]);
                assert_ne!(row[2], "~");
                let target = if row[1] == "input" {
                    assert!(["g0", "g1", "g2", "g3", "g4"].contains(&row[2]));
                    assert!(known.is_empty());
                    ledger.supplied_genotypes += 1;
                    &mut case.input_genotypes
                } else {
                    assert_eq!(row[1], stage);
                    let snapshot = case.snapshots.get_mut(row[1]).unwrap();
                    if snapshot.scopes[row[2]] == "rejected-only" {
                        ledger.rejected_genotypes += 1;
                    } else {
                        ledger.observations += 1;
                    }
                    &mut snapshot.genotypes
                };
                let genotype = target.entry(row[2].to_owned()).or_default();
                if row[0] == "G" {
                    assert!(
                        genotype
                            .genome
                            .insert(row[3].to_owned(), row[4].to_owned())
                            .is_none()
                    );
                } else {
                    assert!(
                        genotype
                            .genes
                            .entry(row[3].parse().unwrap())
                            .or_default()
                            .insert(row[4].to_owned(), row[5].to_owned())
                            .is_none()
                    );
                }
            }
            "R" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], stage);
                validate("double", row[4]);
                assert!(
                    case.snapshots
                        .get_mut(row[1])
                        .unwrap()
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
            "E" => {
                assert_eq!(row.len(), 1);
                ledger.inputs_and_commands += 1;
                assert_eq!(
                    case.settings.keys().collect::<BTreeSet<_>>(),
                    setting_types.keys().collect()
                );
                assert_eq!(
                    case.snapshots.keys().cloned().collect::<BTreeSet<_>>(),
                    declared_snapshots
                );
                if mode == STATEFUL_MODE {
                    assert!(stateful_family_seen);
                    stateful_color_oracle_tests::validate_case(case, &stateful_witnesses);
                }
                validate_case(case, &organism_types, &statistic_types, &getter_types);
                cases.push(current.take().unwrap());
            }
            tag => panic!("Unknown caller record {tag} at line {}", line_number + 1),
        }
    }
    assert!(current.is_none());
    (cases, ledger)
}

fn validate_matrix_case(case: &CallerCase) {
    let (actor, _) = matrix_pair(&case.name);
    if case.mode == "color-matrix" {
        assert_eq!(actor, "GREEN");
    }
    assert_eq!(case.seed, 827364);
    assert_eq!(case.snapshots.len(), 8);
    let mut actual = BTreeMap::new();
    for step in &case.steps {
        if let Step::Input { target, kind, .. } = step {
            assert!(actual.insert(target.clone(), kind.clone()).is_none());
        }
    }
    let mut expected = BTreeMap::new();
    for role in ["victim", "killer"] {
        for field in ["_age", "_growthRatio", "_centerX", "_centerY"] {
            expected.insert(format!("actor:{role}:{field}"), "int".to_owned());
        }
        for field in ["_energy", "_theta", "_dCenterX", "_dCenterY"] {
            expected.insert(format!("actor:{role}:{field}"), "double".to_owned());
        }
    }
    expected.insert("actor:victim:dx".to_owned(), "double".to_owned());
    for label in ["g0", "g1", "g2", "g3", "g4"] {
        expected.insert(format!("genome:{label}:construction"), "String".to_owned());
    }
    expected.insert("visible:selected".to_owned(), "String".to_owned());
    for gas in ["_O2", "_CO2", "_CH4"] {
        expected.insert(format!("world:{gas}"), "double".to_owned());
    }
    assert_eq!(actual, expected);
    assert!(
        case.snapshots
            .values()
            .all(|s| s.scopes.values().all(|v| v == "observable"))
    );
}

fn validate_case(
    case: &CallerCase,
    organism_types: &BTreeMap<String, String>,
    statistic_types: &BTreeMap<String, String>,
    getter_types: &BTreeMap<String, String>,
) {
    assert_eq!(
        case.input_genotypes
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["g0", "g1", "g2", "g3", "g4"]
    );
    if matrix_mode(&case.mode) {
        validate_matrix_case(case);
    }
    let prototype = explicit_genome(&case.input_genotypes["g0"]);
    let gene_schema = gene_fields(&prototype.gene_ref(0).unwrap().borrow()).keys().cloned().collect();
    for genotype in case.input_genotypes.values() {
        validate_genome(genotype, &gene_schema);
    }
    let source_actor_order: Vec<_> = case
        .steps
        .iter()
        .filter_map(|step| match step {
            Step::Actor { role, .. } => Some(role.as_str()),
            _ => None,
        })
        .collect();
    if ["frame", "draw", "player-actions"].contains(&case.mode.as_str()) {
        assert!(matches!(
            source_actor_order.as_slice(),
            [] | ["victim"] | ["victim", "killer"] | ["selector", "victim", "killer"]
        ));
    } else {
        assert_eq!(source_actor_order, ["selector", "victim", "killer"]);
    }
    for (stage, snapshot) in &case.snapshots {
        assert_eq!(
            snapshot.organisms.keys().cloned().collect::<BTreeSet<_>>(),
            snapshot.roles
        );
        assert_eq!(
            snapshot.queries.keys().cloned().collect::<BTreeSet<_>>(),
            snapshot.roles
        );
        assert_eq!(snapshot.references.len(), snapshot.roles.len() * 3);
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
                .instances
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            instance_schema().keys().copied().collect()
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
        assert!(snapshot.suffix.is_some());
        for (role, fields) in &snapshot.organisms {
            assert_eq!(
                fields.keys().collect::<BTreeSet<_>>(),
                organism_types.keys().collect()
            );
            for holder in ["genotype", "infected", "saved"] {
                let reference = &snapshot.references[&(role.clone(), holder.to_owned())];
                if reference != "~" {
                    assert!(snapshot.genotypes.contains_key(reference));
                }
            }
        }
        for genotype in snapshot.genotypes.values() {
            // Output validation reads fields without constructing an expected newborn or candidate
            validate_genome(genotype, &gene_schema);
        }
        for (_, (kind, value)) in &snapshot.statistics {
            if kind == "GeneticCode" && value != "~" {
                assert_eq!(
                    snapshot.scopes[value], "observable",
                    "{} {stage} retained statistics genotype",
                    case.name
                );
            }
        }
        let roles: Vec<_> = role_list(&snapshot.instances["_organisms"])
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            roles.iter().cloned().collect::<BTreeSet<_>>(),
            snapshot.members
        );
        assert_eq!(roles.len(), snapshot.members.len());
        if stage == "initial" {
            assert!(snapshot.insertions.is_empty());
        }
    }
}

fn compare_snapshot(
    case: &CallerCase,
    name: &str,
    world: &World,
    actors: &BTreeMap<String, OrganismRef>,
    genotypes: &mut BTreeMap<String, GenomeRef>,
    returned: &mut BTreeMap<String, ReturnValue>,
    insertions: &mut BTreeMap<String, Vec<(String, IntRect)>>,
    errors: &mut Vec<String>,
) {
    let expected = &case.snapshots[name];
    let available: BTreeSet<_> = expected
        .roles
        .difference(&expected.rejected)
        .cloned()
        .collect();
    assert_eq!(actors.keys().cloned().collect::<BTreeSet<_>>(), available);
    let mut roots = Vec::new();
    for label in case.input_genotypes.keys() {
        roots.push(Rc::clone(&genotypes[label]));
    }
    for ((role, holder), label) in &expected.references {
        if expected.rejected.contains(role) {
            continue;
        }
        let actor = actors[role].borrow();
        let reference = match holder.as_str() {
            "genotype" => actor.genome.as_ref(),
            "infected" => actor.infected_genome.as_ref(),
            "saved" => actor.saved_genome.as_ref(),
            _ => unreachable!(),
        };
        bind_reference(
            label,
            reference,
            genotypes,
            &format!("{} {name} {role}:{holder}", case.name),
        );
        if let Some(reference) = reference {
            roots.push(Rc::clone(reference));
        }
    }
    for (field, (kind, label)) in &expected.statistics {
        if kind != "GeneticCode" {
            continue;
        }
        let statistics_owner = world.statistics_ref();
        let statistics = statistics_owner.borrow();
        let Observation::Genome(actual) = observed_field(&statistics, field) else {
            panic!("Statistics genotype schema changed")
        };
        // Failed candidates can remain observable through actual statistics after border death
        bind_reference(
            label,
            actual,
            genotypes,
            &format!("{} {name} statistics:{field}", case.name),
        );
        if let Some(actual) = actual {
            roots.push(Rc::clone(actual));
        }
    }
    for (label, scope) in &expected.scopes {
        let reachable = genotypes
            .get(label)
            .is_some_and(|code| roots.iter().any(|root| Rc::ptr_eq(root, code)));
        assert_eq!(
            reachable,
            scope == "observable",
            "{} {name} genotype scope {label}",
            case.name
        );
    }
    for (role, target) in &expected.organisms {
        if expected.rejected.contains(role) {
            continue;
        }
        let actor = actors[role].borrow();
        let mut actual = organism_fields(&View::from(&*actor));
        let mut target = target.clone();
        for field in ["_world", "_visibleWorld"] {
            assert_eq!(target.remove(field).as_deref(), Some("present"));
            assert!(actual.remove(field).is_some());
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
    let (actual_world, mut actual_instances) = world_views(world, actors);
    if matrix_mode(&case.mode) || case.mode == "player-actions" || case.mode == STATEFUL_MODE {
        actual_instances.insert(
            "_corridorexists".to_owned(),
            world.corridors_exist.get().to_string(),
        );
    }
    compare_fields(
        &case.name,
        name,
        "W",
        &actual_world,
        &expected.world,
        errors,
    );
    let expected_instances: Fields = expected
        .instances
        .iter()
        .filter(|(field, _)| !world_exclusion(&case.mode, field))
        .map(|(field, value)| (field.clone(), value.clone()))
        .collect();
    compare_fields(
        &case.name,
        name,
        "V",
        &actual_instances,
        &expected_instances,
        errors,
    );
    for (role, expected_query) in &expected.queries {
        if expected.rejected.contains(role) {
            continue;
        }
        let actual = query_roles(world, actors[role].borrow().state.bounds.get(), actors);
        assert_eq!(
            &actual, expected_query,
            "{} {name} {role} frozen query order",
            case.name
        );
    }
    for (field, (kind, value)) in &expected.statistics {
        assert_observation(
            observed_field(&world.statistics_ref().borrow(), field),
            kind,
            value,
            genotypes,
            &format!("{} {name} S {field}", case.name),
        );
    }
    for (getter, (kind, value)) in &expected.getters {
        assert_observation(
            observed_getter(&world.statistics_ref().borrow(), getter),
            kind,
            value,
            genotypes,
            &format!("{} {name} T {getter}", case.name),
        );
    }
    let target_codes: BTreeMap<_, _> = expected
        .genotypes
        .iter()
        .filter(|(label, _)| expected.scopes[*label] == "observable")
        .map(|(label, stage)| {
            (
                label.clone(),
                Stage {
                    genome: stage.genome.clone(),
                    genes: stage.genes.clone(),
                    ..Stage::default()
                },
            )
        })
        .collect();
    let actual_codes = target_codes
        .keys()
        .map(|label| (label.clone(), Rc::clone(&genotypes[label])))
        .collect();
    compare_genomes(&case.name, name, &target_codes, &actual_codes, errors);
    assert_eq!(
        returned.remove(name),
        expected.returned,
        "{} {name} caller return",
        case.name
    );
    assert_eq!(
        insertions.remove(name).unwrap_or_default(),
        expected.insertions,
        "{} {name} frozen tree insertion",
        case.name
    );
    let mut random = world.random_snapshot();
    let suffix = (
        random.next_i64(),
        random.next_bounded(10000).unwrap(),
        random.next_f64().to_bits(),
    );
    assert_eq!(
        Some(suffix),
        expected.suffix,
        "{} {name} RNG suffix",
        case.name
    );
}

fn apply_input(
    case: &CallerCase,
    target: &str,
    kind: &str,
    value: &str,
    world: &mut World,
    actors: &BTreeMap<String, OrganismRef>,
    genotypes: &mut BTreeMap<String, GenomeRef>,
    errors: &mut Vec<String>,
) {
    let parts: Vec<_> = target.split(':').collect();
    match parts.as_slice() {
        ["actor", role, "_clockwise"] if case.mode == "player-actions" => {
            assert_eq!((*role, kind), ("victim", "boolean"));
            actors[*role].borrow_mut().effects.clockwise = value.parse().unwrap();
        }
        ["actor", role, "_hasgoodvision"] if case.mode == "player-actions" => {
            assert_eq!((*role, kind), ("victim", "boolean"));
            actors[*role].borrow_mut().body.eyes.good_vision = value.parse().unwrap();
        }
        ["actor", role, field]
            if case.mode == "draw" && ["_theta", "_lastTheta"].contains(field) =>
        {
            assert_eq!(kind, "double");
            assert_eq!(*role, "victim");
            let mut actor = actors[*role].borrow_mut();
            if *field == "_theta" {
                actor.state.angle = decoded(value);
            } else {
                actor.state.last_angle = decoded(value);
            }
        }
        ["actor", role, "_theta"] if case.mode == STATEFUL_MODE => {
            assert_eq!(kind, "double");
            assert!(actors.contains_key(*role));
            actors[*role].borrow_mut().state.angle = decoded(value);
        }
        ["actor", role, "_theta"] if matrix_mode(&case.mode) => {
            assert_eq!(kind, "double");
            assert!(["victim", "killer"].contains(role));
            actors[*role].borrow_mut().state.angle = decoded(value);
        }
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
        ["world", "nFrames"] => {
            assert_eq!(case.mode, "frame");
            assert_eq!(kind, "int");
            world.frame.set(value.parse().unwrap());
        }
        _ => apply_reference_input(
            &case.name,
            &case.input_genotypes,
            target,
            kind,
            value,
            world,
            actors,
            genotypes,
            errors,
        ),
    }
}

fn build_tree(
    world: &mut World,
    actors: &BTreeMap<String, OrganismRef>,
    argument: &str,
    explicit_members: bool,
) -> Vec<(String, IntRect)> {
    if explicit_members {
        *world.frame_objects.borrow_mut() = role_list(argument)
            .into_iter()
            .map(|role| Rc::clone(&actors[role]))
            .collect();
        *world.spatial.borrow_mut() = crate::spatial::SpatialTree::build_actor_entries(
            world.frame_objects.borrow()
                .iter()
                .enumerate()
                .map(|(token, actor)| (token, actor.borrow().state.bounds.get())),
        |token| Rc::clone(&world.frame_objects.borrow()[token]));
    } else if argument == "members" {
        world.rebuild_spatial();
    } else if argument == "~" {
        world.frame_objects.borrow_mut().clear();
        *world.spatial.borrow_mut() = crate::spatial::SpatialTree::empty();
    } else {
        assert_eq!(
            array(
                world.organisms.borrow()
                    .iter()
                    .map(|actor| actor_role(actor, actors))
            ),
            argument
        );
        world.rebuild_spatial();
    }
    world.frame_objects.borrow()
        .iter()
        .map(|actor| {
            (
                actor_role(actor, actors).to_owned(),
                actor.borrow().state.bounds.get(),
            )
        })
        .collect()
}

fn stored_tree(world: &World, actors: &BTreeMap<String, OrganismRef>) -> Vec<(String, IntRect)> {
    world.spatial.borrow()
        .stored_envelopes()
        .into_iter()
        .map(|(token, envelope)| {
            let [x, y, width, height] = envelope.map(|value| {
                let integer = value as i32;
                assert_eq!(f64::from(integer), value, "Non-integer frame envelope");
                integer
            });
            (
                actor_role(&world.frame_objects.borrow()[token], actors).to_owned(),
                IntRect {
                    x,
                    y,
                    width,
                    height,
                },
            )
        })
        .collect()
}

fn call(
    case: &CallerCase,
    stage: &str,
    operation: &str,
    target: &str,
    argument: &str,
    world: &mut World,
    actors: &BTreeMap<String, OrganismRef>,
) -> (
    Option<ReturnValue>,
    Option<OrganismRef>,
    Vec<(String, IntRect)>,
) {
    if operation == "draw" {
        assert_eq!(case.mode, "draw");
        assert!(!world.corridors_exist.get());
        let actual: Vec<[i32; 5]> = world
            .advance_presentation()
            .unwrap()
            .into_iter()
            .map(|command| {
                let crate::world::outgoing::DrawCommand::Line(line) = command else {
                    panic!("Noncorridor drawing emitted a rectangle")
                };
                let coordinates = [line.x1, line.y1, line.x2, line.y2];
                let mut result = [0; 5];
                for (index, value) in coordinates.into_iter().enumerate() {
                    result[index] = value as i32;
                    assert_eq!(value, f64::from(result[index]));
                }
                result[4] = ((u32::from(line.alpha) << 24) | line.rgb) as i32;
                result
            })
            .collect();
        assert_eq!(
            actual, case.draw_commands[stage],
            "{} {stage} ordered drawing commands",
            case.name
        );
        return (None, None, Vec::new());
    }
    if operation == "time" {
        world
            .advance()
            .unwrap_or_else(|error| panic!("{} {stage} {operation}: {error}", case.name));
        let inserted = stored_tree(world, actors);
        return (None, None, inserted);
    }
    if operation == "genesis" {
        world
            .genesis()
            .unwrap_or_else(|error| panic!("{} {stage} {operation}: {error}", case.name));
        return (None, None, Vec::new());
    }
    if operation == "die" {
        world.die(&mut actors[target].borrow_mut(), None).unwrap();
        return (None, None, Vec::new());
    }
    if operation == "build_tree" {
        return (
            None,
            None,
            build_tree(
                world,
                actors,
                argument,
                case.mode == "contact" || (case.mode == STATEFUL_MODE && argument != "members"),
            ),
        );
    }
    let parent_ref = Rc::clone(&actors[target]);
    let mut parent = parent_ref.borrow_mut();
    if (matrix_mode(&case.mode) || case.mode == "draw" || case.mode == STATEFUL_MODE)
        && ["symmetric", "bounds"].contains(&operation)
    {
        if operation == "symmetric" {
            parent.rebuild_body().unwrap();
        } else {
            crate::movement::calculate_bounds(&mut parent, argument.parse().unwrap()).unwrap();
        }
        return (None, None, Vec::new());
    }
    if operation == "checkHit" {
        let result = world
            .check_contact_from(&parent_ref, &mut parent)
            .unwrap_or_else(|error| panic!("{} {stage} {operation}: {error}", case.name));
        return (
            Some(ReturnValue::Actor(
                result.map(|reference| actor_role(&reference, actors).to_owned()),
            )),
            None,
            Vec::new(),
        );
    }
    let peer_ref = Rc::clone(
        &actors[if operation == "contact" {
            argument
        } else {
            "killer"
        }],
    );
    // Hold the peer mutably so frozen queries must use the explicit borrowed contact view
    let mut peer = peer_ref.borrow_mut();
    if operation == "contact" {
        let result = world
            .contact_from(&parent_ref, &mut parent, &peer_ref, &mut peer)
            .unwrap_or_else(|error| panic!("{} {stage} {operation}: {error}", case.name));
        return (Some(ReturnValue::Boolean(result)), None, Vec::new());
    }
    if case.mode == STATEFUL_MODE && operation.ends_with(".DEFAULT") {
        let arguments: Vec<_> = argument.split(',').collect();
        assert_eq!(arguments.len(), 3);
        assert_eq!(arguments[0], "killer");
        let segment: usize = arguments[1].parse().unwrap();
        let peer_segment: usize = arguments[2].parse().unwrap();
        assert_eq!(
            ColorKind::from_argb(parent.body.color_argb(segment).unwrap()),
            ColorKind::NoColor
        );
        let result = if operation == "touchEffects1.DEFAULT" {
            world.touch_effects_one(
                &parent_ref,
                &mut parent,
                &peer_ref,
                &mut peer,
                segment,
                peer_segment,
            )
        } else {
            assert_eq!(operation, "touchEffects2.DEFAULT");
            world.touch_effects_two(
                &parent_ref,
                &mut parent,
                &peer_ref,
                &mut peer,
                segment,
                peer_segment,
            )
        };
        result.unwrap_or_else(|error| panic!("{} {stage} {operation}: {error}", case.name));
        return (None, None, Vec::new());
    }
    if operation == "touchEffects1" {
        let arguments: Vec<_> = argument.split(',').collect();
        let segment: usize = arguments[1].parse().unwrap();
        let peer_segment = arguments[2].parse().unwrap();
        assert_eq!(
            parent.body.color_argb(segment).unwrap(),
            ColorKind::Coral.argb().unwrap()
        );
        world
            .touch_effects_one(
                &parent_ref,
                &mut parent,
                &peer_ref,
                &mut peer,
                segment,
                peer_segment,
            )
            .unwrap_or_else(|error| panic!("{} {stage} {operation}: {error}", case.name));
        return (None, None, Vec::new());
    }
    let contact_color = match operation {
        "touchEffects2.BLUE" => Some(ColorKind::Blue),
        "touchEffects2.YELLOW" => Some(ColorKind::Yellow),
        "touchEffects2.ROSE" => Some(ColorKind::Rose),
        "touchEffects2.MINT" => Some(ColorKind::Mint),
        "touchEffects2.LAVENDER" => Some(ColorKind::Lavender),
        "touchEffects2.MAGENTA" => Some(ColorKind::Magenta),
        "touchEffects2.PLAGUE" => Some(ColorKind::Plague),
        "touchEffects2.FALLOW" => Some(ColorKind::Fallow),
        _ => None,
    };
    if let Some(color) = contact_color {
        let arguments: Vec<_> = argument.split(',').collect();
        let segment: usize = arguments[1].parse().unwrap();
        let peer_segment = arguments[2].parse().unwrap();
        assert_eq!(parent.body.color_argb(segment).unwrap(), color.argb().unwrap());
        world
            .touch_effects_two(
                &parent_ref,
                &mut parent,
                &peer_ref,
                &mut peer,
                segment,
                peer_segment,
            )
            .unwrap_or_else(|error| panic!("{} {stage} {operation}: {error}", case.name));
        return (None, None, Vec::new());
    }
    let peer_view = Some((&peer_ref, &*peer));
    let result = match operation {
        "reproduce" => world.reproduce_from(&parent_ref, &mut parent, peer_view),
        "reproduceEarly" => world.reproduce_early_from(&parent_ref, &mut parent, peer_view),
        "reproduceEarlyPlant" => {
            world.reproduce_early_plant_from(&parent_ref, &mut parent, peer_view)
        }
        "reproduceEarlyVirus" => {
            world.reproduce_early_virus_from(&parent_ref, &mut parent, peer_view)
        }
        "reproduceSpore" => world.reproduce_spore_from(&parent_ref, &mut parent, peer_view),
        "reproduceYellow" => world.reproduce_yellow_from(&parent_ref, &mut parent, peer_view),
        "reproduceVirus" => world.reproduce_virus_from(&parent_ref, &mut parent, peer_view),
        "reproduceforeignVirus" => {
            world.reproduce_foreign_virus_from(&parent_ref, &mut parent, peer_view)
        }
        "transform" => world.transform_from(&parent_ref, &mut parent, peer_view),
        "inheritTransformation" => {
            let child = world
                .construct_transformation_from(
                    &parent_ref,
                    &mut parent,
                    argument.parse().unwrap(),
                    peer_view,
                )
                .unwrap_or_else(|error| panic!("{} {stage} {operation}: {error}", case.name));
            return (
                Some(ReturnValue::Boolean(child.is_some())),
                child,
                Vec::new(),
            );
        }
        "blueSpore" => crate::birth_spore::blue_spore(&mut parent).map_err(WorldError::Geometry),
        "cyanSpore" => crate::birth_spore::cyan_spore(&mut parent).map_err(WorldError::Geometry),
        "grow" => crate::movement::grow(&mut parent).map_err(WorldError::Geometry),
        "offset_velocity" => {
            let displacement = parent.state.velocity;
            let rotation = parent.state.angular_velocity;
            crate::movement::offset(&mut parent, displacement, rotation);
            Ok(())
        }
        "bounds" => crate::movement::calculate_bounds(&mut parent, argument.parse().unwrap())
            .map_err(WorldError::Geometry),
        operation => panic!("Unknown replay caller operation {operation}"),
    };
    result.unwrap_or_else(|error| panic!("{} {stage} {operation}: {error}", case.name));
    (None, None, Vec::new())
}

fn replay(case: &CallerCase, errors: &mut Vec<String>) {
    let mut world = if ["frame", "draw"].contains(&case.mode.as_str()) {
        let mut source_parameters = parameters(&case.settings);
        let lifecycle = source_parameters.lifecycle;
        source_parameters.lifecycle = crate::LifecycleParams::default();
        let world = World::empty_with_parameters(
            WorldConfig {
                seed: case.seed,
                width: integer(&case.settings, "WORLD_WIDTH").try_into().unwrap(),
                height: integer(&case.settings, "WORLD_HEIGHT").try_into().unwrap(),
                initial_attempts: integer(&case.settings, "INITIAL_ORGANISMS")
                    .try_into()
                    .unwrap(),
            },
            source_parameters,
        )
        .unwrap();
        world.session.0.parameters.borrow_mut().lifecycle = lifecycle;
        world
    } else {
        assert_eq!(integer(&case.settings, "INITIAL_ORGANISMS"), 0);
        empty_reference_world(&case.settings, case.seed)
    };
    let mut actors = BTreeMap::new();
    let mut genotypes = BTreeMap::new();
    let mut returned = BTreeMap::new();
    let mut insertions = BTreeMap::new();
    let mut pending_candidates: BTreeMap<String, Option<OrganismRef>> = BTreeMap::new();
    let mut newborns = 0;
    let mut candidates = 0;
    let mut latest_call = None;
    let mut selected: Option<String> = None;
    let mut frontend_absent_callbacks = 0;
    let mut stateful_actor_order = Vec::new();
    for step in &case.steps {
        match step {
            Step::Input {
                stage,
                target,
                kind,
                value,
            } => {
                assert!(stage == "setup" || stage.starts_with("step-"));
                if case.mode == "player-actions" && target == "visible:selected" {
                    assert_eq!(kind, "String");
                    selected = if value == "~" {
                        None
                    } else {
                        let reference = actors
                            .get(value)
                            .expect("Selection must use an actual constructed role");
                        assert!(
                            world.organisms.borrow()
                                .iter()
                                .any(|member| Rc::ptr_eq(member, reference))
                        );
                        Some(value.clone())
                    };
                    continue;
                }
                apply_input(
                    case,
                    target,
                    kind,
                    value,
                    &mut world,
                    &actors,
                    &mut genotypes,
                    errors,
                );
            }
            Step::Actor {
                role,
                genotype,
                center,
            } => {
                let actor = world
                    .construct_pasted_shared(Rc::clone(&genotypes[genotype]), *center)
                    .unwrap()
                    .unwrap();
                let actor = Rc::new(RefCell::new(actor));
                world.add_organism(Rc::clone(&actor), None);
                assert!(actors.insert(role.clone(), actor).is_none());
                if case.mode == STATEFUL_MODE {
                    stateful_actor_order.push(role.clone());
                }
            }
            Step::Call {
                stage,
                operation,
                target,
                argument,
            } => {
                let (result, child, inserted) =
                    if case.mode == "player-actions" && operation == "pick" {
                        let point: Vec<i32> = argument
                            .split(',')
                            .map(|value| value.parse().unwrap())
                            .collect();
                        let role = super::super::player_actions::source_tests::pick_role(
                            &world,
                            [point[0], point[1]],
                            &actors,
                        );
                        (Some(ReturnValue::Actor(role)), None, Vec::new())
                    } else if case.mode == "player-actions"
                        && [
                            "feed",
                            "weaken",
                            "kill",
                            "revive",
                            "disperse",
                            "reproduce",
                            "rejuvenate",
                        ]
                        .contains(&operation.as_str())
                    {
                        if let Some(role) = &selected {
                            super::super::player_actions::source_tests::invoke_selected(
                                &mut world,
                                &actors[role],
                                operation,
                            ).unwrap();
                        } else {
                            assert_eq!(case.name, "null-selection-guards");
                            frontend_absent_callbacks += 1;
                        }
                        (None, None, Vec::new())
                    } else {
                        call(
                            case, stage, operation, target, argument, &mut world, &actors,
                        )
                    };
                if let Some(result) = result {
                    assert!(returned.insert(stage.clone(), result).is_none());
                }
                if operation == "inheritTransformation" {
                    assert!(pending_candidates.insert(stage.clone(), child).is_none());
                }
                if matrix_mode(&case.mode)
                    || case.mode == "player-actions"
                    || case.mode == STATEFUL_MODE
                {
                    assert!(
                        insertions
                            .insert(
                                stage.clone(),
                                if case.mode == STATEFUL_MODE {
                                    stateful_color_oracle_tests::stored_in_source_actor_order(
                                        &world,
                                        &actors,
                                        &stateful_actor_order,
                                    )
                                } else {
                                    stored_tree(&world, &actors)
                                }
                            )
                            .is_none()
                    );
                } else if operation == "build_tree" || operation == "time" {
                    assert!(insertions.insert(stage.clone(), inserted).is_none());
                }
                latest_call = Some(stage.as_str());
            }
            Step::Discover {
                stage,
                role,
                id,
                genotype,
            } => {
                assert_eq!(latest_call, Some(stage.as_str()));
                let prefix = if matrix_mode(&case.mode)
                    || case.mode == "player-actions"
                    || case.mode == STATEFUL_MODE
                {
                    "arrival"
                } else {
                    "newborn"
                };
                assert_eq!(*role, format!("{prefix}{newborns}"));
                newborns += 1;
                let child = world.organisms.borrow()
                    .iter()
                    .find(|member| !actors.values().any(|known| Rc::ptr_eq(member, known)))
                    .cloned()
                    .unwrap_or_else(|| {
                        panic!("{} {stage}: no actual new member {role}", case.name)
                    });
                assert_eq!(
                    child.borrow().id,
                    i64::from(*id),
                    "{} {stage} {role} actual ID",
                    case.name
                );
                bind_reference(
                    genotype,
                    child.borrow().genome.as_ref(),
                    &mut genotypes,
                    &format!("{} {stage} {role}", case.name),
                );
                assert!(actors.insert(role.clone(), child).is_none());
                if case.mode == STATEFUL_MODE {
                    stateful_actor_order.push(role.clone());
                }
            }
            Step::Candidate {
                stage,
                role,
                available,
            } => {
                assert_eq!(latest_call, Some(stage.as_str()));
                assert_eq!(*role, format!("candidate{candidates}"));
                candidates += 1;
                let actual = pending_candidates
                    .remove(stage)
                    .expect("Missing actual direct construction");
                assert_eq!(
                    actual.is_some(),
                    *available,
                    "{} {stage} candidate availability",
                    case.name
                );
                if let Some(actual) = actual {
                    assert!(actors.insert(role.clone(), actual).is_none());
                }
            }
            Step::Snapshot(name) => {
                for member in world.organisms.borrow().clone() {
                    actor_role(&member, &actors);
                }
                compare_snapshot(
                    case,
                    name,
                    &world,
                    &actors,
                    &mut genotypes,
                    &mut returned,
                    &mut insertions,
                    errors,
                );
            }
        }
    }
    assert!(returned.is_empty() && insertions.is_empty() && pending_candidates.is_empty());
    if case.mode == "player-actions" {
        assert_eq!(
            frontend_absent_callbacks,
            if case.name == "null-selection-guards" {
                7
            } else {
                0
            }
        );
    }
}

pub(in crate::world) fn compare_fixture(source: &str, mode: &str) {
    compare_declared_fixture(source, mode, expected_ledger(mode));
}

fn compare_declared_fixture(source: &str, mode: &str, declared: Ledger) -> Ledger {
    let (cases, ledger) = parse(source, mode);
    assert_eq!(ledger, declared);
    assert_eq!(
        ledger.rows,
        ledger.observations
            + ledger.supplied_genotypes
            + ledger.organism_collaborators
            + ledger.world_collaborators
            + ledger.selection_metadata
            + ledger.scope_metadata
            + ledger.rejected_candidates
            + ledger.rejected_genotypes
            + ledger.inputs_and_commands
    );
    let mut errors = Vec::new();
    for case in cases {
        assert_eq!(case.mode, mode);
        replay(&case, &mut errors);
    }
    if !errors.is_empty() {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!(
            "{mode}-oracle-mismatches-{}.tsv",
            std::process::id()
        ));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        writeln!(file, "{}", errors.join("\n")).unwrap();
        panic!(
            "{} {mode} mismatches; report={}\n{}",
            errors.len(),
            path.display(),
            errors
                .iter()
                .take(40)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    println!(
        "CALLER_COMPARISON {mode} {ledger:?}; method observations only; no complete frame/UI claim"
    );
    ledger
}

#[test]
fn actual_java_reproduction_callers_preserve_state_identity_statistics_and_rng() {
    compare_fixture(
        include_str!("../tests/fixtures/reproduction.tsv"),
        "reproduction",
    );
}

#[test]
fn actual_java_draw_commands_preserve_order_argb_caches_and_state() {
    compare_fixture(include_str!("../tests/fixtures/draw.tsv"), "draw");
}

#[test]
fn draw_checker_rejects_changed_argb_without_using_it_as_model_input() {
    let (mut cases, ledger) = parse(include_str!("../tests/fixtures/draw.tsv"), "draw");
    assert_eq!(ledger, expected_ledger("draw"));
    let case = cases
        .iter_mut()
        .find(|case| case.name == "pasted-single")
        .unwrap();
    let command = &mut case.draw_commands.get_mut("step-0").unwrap()[0];
    command[4] ^= 1;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        replay(case, &mut Vec::new());
    }));
    let failure = result.expect_err("Changed drawing output must fail");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(
        message.contains("ordered drawing commands"),
        "Unexpected rejection: {message}"
    );
}

#[test]
fn actual_java_transformation_preserves_available_and_rejected_outcomes() {
    compare_fixture(
        include_str!("../tests/fixtures/transformation.tsv"),
        "transformation",
    );
}

#[test]
fn actual_java_coral_contacts_preserve_conversion_death_and_dynamic_identity() {
    compare_fixture(include_str!("../tests/fixtures/coral.tsv"), "coral");
}

#[test]
fn actual_java_cooperative_contacts_preserve_live_births_cures_sharing_and_rng() {
    compare_fixture(
        include_str!("../tests/fixtures/cooperative.tsv"),
        "cooperative",
    );
}

#[test]
fn actual_java_plague_and_fallow_preserve_live_births_inhibition_and_identity() {
    compare_fixture(
        include_str!("../tests/fixtures/plague_fallow.tsv"),
        "plague-fallow",
    );
}

#[test]
fn actual_java_contact_and_world_queries_preserve_order_identity_and_effects() {
    compare_fixture(include_str!("../tests/fixtures/contact.tsv"), "contact");
}

#[test]
fn actual_java_world_frames_preserve_sequential_state_identity_and_clocks() {
    compare_fixture(include_str!("../tests/fixtures/frame.tsv"), "frame");
}

#[test]
fn actual_java_green_palette_queries_and_following_frames_match() {
    compare_fixture(
        include_str!("../tests/fixtures/matrix-green.tsv"),
        "color-matrix",
    );
}

#[test]
fn matrix_checker_rejects_changed_source_rng_after_real_setup() {
    let (mut cases, ledger) = parse(
        include_str!("../tests/fixtures/matrix-green.tsv"),
        "color-matrix",
    );
    assert_eq!(ledger, expected_ledger("color-matrix"));
    let case = &mut cases[0];
    case.snapshots
        .get_mut("initial")
        .unwrap()
        .suffix
        .as_mut()
        .unwrap()
        .0 ^= 1;
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        replay(case, &mut Vec::new())
    }))
    .unwrap_err();
    let text = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(text.contains("initial RNG suffix"), "{text}");
}

#[test]
fn matrix_checker_rejects_changed_post_contact_phenotype_without_using_it_as_input() {
    let (mut cases, ledger) = parse(
        include_str!("../tests/fixtures/matrix-green.tsv"),
        "color-matrix",
    );
    assert_eq!(ledger, expected_ledger("color-matrix"));
    let case = &mut cases[0];
    let field = case
        .snapshots
        .get_mut("step-5")
        .unwrap()
        .organisms
        .get_mut("victim")
        .unwrap()
        .get_mut("_segColor")
        .unwrap();
    let color = u32::from_str_radix(field, 16).unwrap();
    *field = format!("{:08x}", color ^ 1);
    let mut errors = Vec::new();
    replay(case, &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("step-5:victim") && errors[0].contains("_segColor"),
        "{errors:?}"
    );
}

pub(in crate::world) fn reject_player_actions_rng_change(source: &str) {
    let (mut cases, ledger) = parse(source, "player-actions");
    assert_eq!(ledger, expected_ledger("player-actions"));
    let case = &mut cases[0];
    case.snapshots
        .get_mut("initial")
        .unwrap()
        .suffix
        .as_mut()
        .unwrap()
        .0 ^= 1;
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        replay(case, &mut Vec::new())
    }))
    .unwrap_err();
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(message.contains("initial RNG suffix"), "{message}");
}

pub(in crate::world) fn reject_player_actions_energy_change(source: &str) {
    let (mut cases, ledger) = parse(source, "player-actions");
    assert_eq!(ledger, expected_ledger("player-actions"));
    let case = cases
        .iter_mut()
        .find(|case| case.name == "coherent-empty-body-revive")
        .unwrap();
    let value = case
        .snapshots
        .get_mut("step-1")
        .unwrap()
        .organisms
        .get_mut("victim")
        .unwrap()
        .get_mut("_energy")
        .unwrap();
    *value = format!("{:016x}", u64::from_str_radix(value, 16).unwrap() ^ 1);
    let mut errors = Vec::new();
    replay(case, &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("step-1:victim") && errors[0].contains("_energy"),
        "{errors:?}"
    );
}


#[test]
fn actual_java_nullable_draw_case_preserves_real_graphics_retention() {
    let source = include_str!("../tests/fixtures/draw.tsv");
    let nullable = source.split_inclusive("\nE\n").find(|case|
        case.starts_with("C\tnullable-phenotype-uses-real-graphics-retention\t")
    ).unwrap();
    assert_eq!(nullable.lines().count(), 3037);
    let (cases, _) = parse(nullable, "draw");
    assert_eq!(cases.len(), 1);
    let mut errors = Vec::new();
    replay(&cases[0], &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}


/// Reuse the real callback and original raw diagnostic without initializing from observations
pub(in crate::world) fn compare_raw_null_gene_revive_prefix() {
    let source = include_str!("../tests/fixtures/player-actions-diagnostic.tsv");
    let lines: Vec<_> = source.lines().take_while(|line| *line != "E").collect();
    assert_eq!(lines[0], "C\traw-null-gene-color\tplayer-actions-diagnostic\t827364");
    assert_eq!(lines.len() + 1, 5_240);
    let exceptions: Vec<_> = lines.iter().copied().filter(|line| line.starts_with("X\t")).collect();
    assert_eq!(exceptions, ["X\tstep-1\tjava.lang.NullPointerException\t~"]);
    // Adapt only the parser's mode marker; X is checked against the actual typed callback below
    let mut parsed = lines.iter().filter(|line| !line.starts_with("X\t"))
        .map(|line| (*line).to_owned()).collect::<Vec<_>>();
    parsed[0] = "C\traw-null-gene-color\tplayer-actions\t827364".into();
    parsed.push("E".into());
    let (mut cases, ledger) = parse(&parsed.join("\n"), "player-actions");
    assert_eq!((ledger.cases, ledger.snapshots, ledger.rows), (1, 5, 5_239));
    assert_eq!(ledger.observations, 4_660);
    assert_eq!(ledger.scope_metadata, 25);
    assert_eq!((ledger.organism_collaborators, ledger.world_collaborators), (30, 15));
    let case = cases.remove(0);
    let mut world = empty_reference_world(&case.settings, case.seed);
    let mut actors = BTreeMap::new();
    let mut genomes = BTreeMap::new();
    let mut errors = Vec::new();
    let mut selected = None;
    let mut returned = BTreeMap::new();
    let mut insertions = BTreeMap::new();
    let mut calls = 0;
    let mut compared = 0;
    for step in &case.steps {
        match step {
            Step::Input { target, kind, value, .. } => {
                if target == "visible:selected" {
                    assert_eq!(kind, "String");
                    assert!(actors.contains_key(value));
                    selected = Some(value.clone());
                } else {
                    apply_input(&case, target, kind, value, &mut world, &actors, &mut genomes, &mut errors);
                }
            }
            Step::Actor { role, genotype, center } => {
                let actor = world.construct_pasted_shared(Rc::clone(&genomes[genotype]), *center)
                    .unwrap().unwrap();
                let actor = Rc::new(RefCell::new(actor));
                world.add_organism(Rc::clone(&actor), None);
                assert!(actors.insert(role.clone(), actor).is_none());
            }
            Step::Call { stage, operation, target, argument } => {
                assert_eq!((target.as_str(), argument.as_str()), ("selected", "~"));
                let role = selected.as_ref().unwrap();
                let result = super::super::player_actions::source_tests::invoke_selected(
                    &mut world, &actors[role], operation);
                if stage == "step-0" {
                    assert_eq!(operation, "kill");
                    result.unwrap();
                } else {
                    assert_eq!((stage.as_str(), operation.as_str()), ("step-1", "revive"));
                    assert_eq!(result, Err(WorldError::Geometry(
                        crate::GeometryError::MissingPhenotypeColor { segment: 0 })));
                }
                insertions.insert(stage.clone(), stored_tree(&world, &actors));
                calls += 1;
            }
            Step::Snapshot(name) => {
                compare_snapshot(&case, name, &world, &actors, &mut genomes,
                    &mut returned, &mut insertions, &mut errors);
                compared += 1;
            }
            _ => panic!("Unexpected raw Revive diagnostic step"),
        }
    }
    assert_eq!((calls, compared), (2, 5));
    assert!(returned.is_empty() && insertions.is_empty());
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
