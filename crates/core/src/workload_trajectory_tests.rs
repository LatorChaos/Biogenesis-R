// SPDX-License-Identifier: GPL-2.0-or-later

//! Exact compact trajectory checks; expected outputs never initialize simulation state

use super::*;
use crate::statistics::tests::{Observation, observed_field};

const WORLD_SCHEMA: [(&str, &str); 9] = [
    ("NEXT_ID", "int"),
    ("_CH4", "double"),
    ("_CO2", "double"),
    ("_O2", "double"),
    ("_corridorexists", "boolean"),
    ("_height", "int"),
    ("_population", "int"),
    ("_width", "int"),
    ("nFrames", "int"),
];
const STATS_SCHEMA: [(&str, &str); 37] = [
    ("aliveBeingMostChildrenNumber", "int"),
    ("aliveBeingMostInfectionsNumber", "int"),
    ("aliveBeingMostKillsNumber", "int"),
    ("beingMostChildrenNumber", "int"),
    ("beingMostChildrenTime", "long"),
    ("beingMostInfectionsNumber", "int"),
    ("beingMostInfectionsTime", "long"),
    ("beingMostKillsNumber", "int"),
    ("beingMostKillsTime", "long"),
    ("birthLastTime", "int"),
    ("birthSum", "long"),
    ("createdOrganisms", "int"),
    ("deathLastTime", "int"),
    ("deathSum", "long"),
    ("infectionsSum", "long"),
    ("massExtintionState", "int"),
    ("massExtintions", "int"),
    ("maxBirths", "int"),
    ("maxCarbonDioxide", "double"),
    ("maxCarbonDioxideTime", "long"),
    ("maxDeaths", "int"),
    ("maxMethane", "double"),
    ("maxMethaneTime", "long"),
    ("maxOxygen", "double"),
    ("maxOxygenTime", "long"),
    ("maxPopulation", "int"),
    ("maxPopulationTime", "long"),
    ("minCarbonDioxide", "double"),
    ("minCarbonDioxideTime", "long"),
    ("minMethane", "double"),
    ("minMethaneTime", "long"),
    ("minOxygen", "double"),
    ("minOxygenTime", "long"),
    ("minPopulation", "int"),
    ("minPopulationTime", "long"),
    ("populationSum", "long"),
    ("time", "long"),
];
const GETTER_SCHEMA: [(&str, &str); 9] = [
    ("_population", "int"),
    ("organism_count", "int"),
    ("corpses", "int"),
    ("_O2", "double"),
    ("_CO2", "double"),
    ("_CH4", "double"),
    ("NEXT_ID", "int"),
    ("nFrames", "long"),
    ("time", "long"),
];
const TAG_COUNTS: [(&str, usize); 11] = [
    ("C", 4),
    ("P", 40),
    ("U", 588),
    ("X", 8),
    ("K", 320),
    ("V", 2916),
    ("S", 11988),
    ("W", 2916),
    ("R", 324),
    ("Q", 324),
    ("E", 4),
];

fn current_values(world: &World) -> BTreeMap<(String, String), (String, String)> {
    let mut values = BTreeMap::new();
    let mut fields = world_fields(world);
    fields.insert("nFrames".into(), world.frame().to_string());
    for (name, kind) in WORLD_SCHEMA {
        values.insert(
            ("V".into(), name.into()),
            (kind.into(), fields[name].clone()),
        );
    }
    for (name, kind) in STATS_SCHEMA {
        let (observed_kind, value) = match observed_field(&world.statistics_ref().borrow(), name) {
            Observation::Int(value) => ("int", value.to_string()),
            Observation::Long(value) => ("long", value.to_string()),
            Observation::Double(value) => ("double", bits(value)),
            _ => panic!("Nonprimitive statistics field {name}"),
        };
        assert_eq!(observed_kind, kind);
        values.insert(("S".into(), name.into()), (kind.into(), value));
    }
    let mut getters = world_fields(world);
    getters.insert("corpses".into(), world.corpse_count().unwrap().to_string());
    getters.insert("nFrames".into(), world.frame().to_string());
    getters.insert("time".into(), world.time().to_string());
    for (name, kind) in GETTER_SCHEMA {
        values.insert(
            ("W".into(), name.into()),
            (kind.into(), getters[name].clone()),
        );
    }
    assert_eq!(values.len(), 55);
    values
}

fn validate_preferences(preferences: &BTreeMap<String, (String, String)>, attempts: u32) {
    let expected = BTreeMap::from([
        ("VERSION".into(), ("int".into(), "700".into())),
        (
            "INITIAL_ORGANISMS".into(),
            ("int".into(), attempts.to_string()),
        ),
        ("WORLD_WIDTH".into(), ("int".into(), "7000".into())),
        ("WORLD_HEIGHT".into(), ("int".into(), "7000".into())),
        (
            "ACCEPT_CONNECTIONS".into(),
            ("boolean".into(), "false".into()),
        ),
        (
            "CONNECT_TO_SERVER".into(),
            ("boolean".into(), "false".into()),
        ),
        ("AUTO_BACKUP".into(), ("boolean".into(), "false".into())),
        ("HARDWARE_ACCELERATION".into(), ("int".into(), "0".into())),
        ("DELAY".into(), ("int".into(), "20".into())),
        ("LOCALE".into(), ("String".into(), "en".into())),
    ]);
    assert_eq!(preferences, &expected, "Actual source preference controls");
}

#[derive(Default)]
struct Counts {
    rows: BTreeMap<String, usize>,
    values: usize,
    runtime_values: usize,
    source_invariants: usize,
    snapshots: usize,
    calls: usize,
    cohorts: usize,
}

#[derive(Clone, Copy)]
enum TrajectoryMode {
    Original80,
    Evolving512,
    Dense256,
}

fn compare_trajectory_rows(text: &str, mode: TrajectoryMode) -> (Counts, Option<World>, Fields) {
    let (cohorts, call_groups): (&[u32], [u32; 2]) = match mode {
        TrajectoryMode::Original80 => (&[0, 32, 512, 12500], [16, 64]),
        TrajectoryMode::Evolving512 => (&[512], [16, 496]),
        TrajectoryMode::Dense256 => (&[12500], [16, 240]),
    };
    let ticks = (call_groups[0] + call_groups[1]) as i32;
    let (setting_schema, _) = primitive_reference_schemas();
    let mut totals = Counts::default();
    let mut settings = Fields::new();
    let mut preferences = BTreeMap::new();
    let mut world: Option<World> = None;
    let mut retained_world = None;
    let mut attempts = None;
    let mut stage = 0;
    let mut seen = BTreeSet::new();
    let mut omissions = BTreeSet::new();
    let mut snapshot_values = BTreeMap::new();
    let mut snapshot_random = None;
    let mut suffix_seen = false;
    let mut completed_snapshot = false;
    for (line_number, line) in text.lines().enumerate() {
        let row: Vec<_> = line.split('\t').collect();
        let tag = row[0];
        assert!(TAG_COUNTS.iter().any(|(known, _)| *known == tag));
        *totals.rows.entry(tag.into()).or_default() += 1;
        let context = format!(
            "attempts={attempts:?} frame={stage} line={}",
            line_number + 1
        );
        match tag {
            "C" => {
                assert_eq!(row.len(), 8);
                assert!(attempts.is_none());
                let count: u32 = row[2].parse().unwrap();
                assert_eq!(count, cohorts[totals.cohorts]);
                assert_eq!(row[1], format!("workload-{count}"));
                assert_eq!(&row[3..6], &["827364", "7000", "7000"]);
                assert_eq!(row[6], call_groups[0].to_string());
                assert_eq!(row[7], call_groups[1].to_string());
                attempts = Some(count);
                settings.clear();
                preferences.clear();
                omissions.clear();
                seen.clear();
                stage = 0;
                world = None;
                suffix_seen = false;
                completed_snapshot = false;
            }
            "P" => {
                assert_eq!(row.len(), 4);
                assert!(world.is_none());
                assert!(
                    preferences
                        .insert(row[1].into(), (row[2].into(), row[3].into()))
                        .is_none()
                );
            }
            "U" => {
                assert_eq!(row.len(), 4);
                assert!(world.is_none());
                assert_eq!(row[2], setting_schema[row[1]]);
                assert!(settings.insert(row[1].into(), row[3].into()).is_none());
            }
            "X" => {
                assert_eq!(row.len(), 4);
                assert!(world.is_none());
                assert_eq!(row[2], "nonprimitive_fields");
                assert!(omissions.insert(row[1]));
                let expected = match row[1] {
                    "World" => {
                        "_organisms,_visibleWorld,colDetTree,inCorridors,outCorridors,worldStatistics"
                    }
                    "WorldStatistics" => {
                        "aliveBeingMostChildren,aliveBeingMostInfections,aliveBeingMostKills,beingMostChildren,beingMostInfections,beingMostKills,birthList,carbonDioxideList,deathList,lastBornBeing,lastDeadBeing,lastInfectedBeing,methaneList,oxygenList,populationList"
                    }
                    _ => panic!("Unknown omitted owner"),
                };
                assert_eq!(row[3], expected);
            }
            "K" => {
                assert_eq!(row.len(), 3);
                assert!(completed_snapshot);
                assert_eq!(row[2], "time");
                assert_eq!(row[1].parse::<i32>().unwrap(), stage + 1);
                assert!(stage < ticks);
                let active = world.as_mut().unwrap();
                assert!(active.paused());
                assert!(active.single_step().unwrap());
                stage += 1;
                totals.calls += 1;
                seen.clear();
                suffix_seen = false;
                completed_snapshot = false;
            }
            "V" | "S" | "W" | "R" => {
                if world.is_none() {
                    let count = attempts.unwrap();
                    assert_eq!(settings.len(), 147);
                    assert!(settings.keys().eq(setting_schema.keys()));
                    validate_preferences(&preferences, count);
                    assert_eq!(integer(&settings, "INITIAL_ORGANISMS"), count as i32);
                    assert_eq!(integer(&settings, "WORLD_WIDTH"), 7000);
                    assert_eq!(integer(&settings, "WORLD_HEIGHT"), 7000);
                    assert_eq!(omissions, BTreeSet::from(["World", "WorldStatistics"]));
                    world = Some(
                        World::with_parameters(
                            WorldConfig {
                                seed: 827364,
                                width: 7000,
                                height: 7000,
                                initial_attempts: count,
                            },
                            parameters(&settings),
                        )
                        .unwrap(),
                    );
                }
                let active = world.as_ref().unwrap();
                assert_eq!(active.frame(), stage % 256, "{context} frame rollover");
                assert_eq!(
                    active.time(),
                    i64::from(stage / 256),
                    "{context} time rollover"
                );
                assert_eq!(row[1].parse::<i32>().unwrap(), stage);
                assert!(!completed_snapshot);
                if seen.is_empty() {
                    snapshot_values = current_values(active);
                    snapshot_random = Some(active.random_snapshot());
                }
                if tag == "R" {
                    assert_eq!(row.len(), 8);
                    assert!(!suffix_seen);
                    assert_eq!(seen.len(), 55);
                    let mut copy = active.random_snapshot();
                    let actual = [
                        active.random_snapshot().state48().to_string(),
                        copy.next_i64().to_string(),
                        copy.next_bounded(10000).unwrap().to_string(),
                        bits(copy.next_f64()),
                    ];
                    for (actual, index) in actual.iter().zip([2, 5, 6, 7]) {
                        assert_eq!(actual, row[index], "{context} R[{index}]");
                        totals.values += 1;
                        totals.runtime_values += 1;
                    }
                    // The game never calls nextGaussian; these source invariants are not Rust runtime cache observations
                    assert_eq!(row[3], "false", "{context} source Gaussian flag invariant");
                    assert_eq!(
                        row[4], "0000000000000000",
                        "{context} source Gaussian cache invariant"
                    );
                    totals.values += 2;
                    totals.source_invariants += 2;
                    assert_eq!(active.random_snapshot(), snapshot_random.as_ref().unwrap().clone());
                    suffix_seen = true;
                } else {
                    assert_eq!(row.len(), 5);
                    let key = (tag.into(), row[2].into());
                    assert!(seen.insert(key.clone()));
                    let (kind, value) = &snapshot_values[&key];
                    assert_eq!(row[3], kind);
                    assert_eq!(row[4], value, "{context} {tag}.{}", row[2]);
                    totals.runtime_values += 1;
                    totals.values += 1;
                }
            }
            "Q" => {
                assert_eq!(row.len(), 6);
                assert_eq!(row[1].parse::<i32>().unwrap(), stage);
                assert_eq!(
                    &row[2..],
                    &[
                        "random_serialization_pure",
                        "true",
                        "world_statistics_scalars_pure",
                        "true"
                    ]
                );
                assert!(suffix_seen && seen.len() == 55 && !completed_snapshot);
                let active = world.as_ref().unwrap();
                assert_eq!(current_values(active), snapshot_values);
                assert_eq!(active.random_snapshot(), snapshot_random.as_ref().unwrap().clone());
                completed_snapshot = true;
                totals.snapshots += 1;
            }
            "E" => {
                assert_eq!(row.len(), 1);
                assert_eq!(stage, ticks);
                assert!(completed_snapshot);
                assert!(world.as_ref().unwrap().paused());
                totals.cohorts += 1;
                attempts = None;
                if matches!(mode, TrajectoryMode::Dense256) {
                    retained_world = world.take();
                } else {
                    world = None;
                }
            }
            _ => unreachable!(),
        }
    }
    assert!(attempts.is_none());
    (totals, retained_world, settings)
}

fn compare_trajectories(text: &str) -> Counts {
    let (totals, _, _) = compare_trajectory_rows(text, TrajectoryMode::Original80);
    assert_eq!(
        totals.rows,
        TAG_COUNTS.into_iter().map(|(k, v)| (k.into(), v)).collect()
    );
    assert_eq!(
        (
            totals.cohorts,
            totals.calls,
            totals.snapshots,
            totals.values
        ),
        (4, 320, 324, 19764)
    );
    assert_eq!(
        (totals.runtime_values, totals.source_invariants),
        (19116, 648)
    );
    totals
}

fn compare_evolving_trajectory(text: &str) -> Counts {
    let (totals, _, _) = compare_trajectory_rows(text, TrajectoryMode::Evolving512);
    let tag_counts: [(&str, usize); 11] = [
        ("C", 1),
        ("P", 10),
        ("U", 147),
        ("X", 2),
        ("K", 512),
        ("V", 4617),
        ("S", 18981),
        ("W", 4617),
        ("R", 513),
        ("Q", 513),
        ("E", 1),
    ];
    assert_eq!(
        totals.rows,
        tag_counts.into_iter().map(|(k, v)| (k.into(), v)).collect()
    );
    assert_eq!(totals.rows.values().sum::<usize>(), 29914);
    assert_eq!(
        (
            totals.cohorts,
            totals.calls,
            totals.snapshots,
            totals.values
        ),
        (1, 512, 513, 31293)
    );
    assert_eq!(
        (totals.runtime_values, totals.source_invariants),
        (30267, 1026)
    );
    totals
}

#[test]
fn compact_workload_trajectories_match_all_original_scalar_and_rng_observations() {
    let totals = compare_trajectories(include_str!("../tests/fixtures/workload_trajectory.tsv"));
    println!(
        "WORKLOAD_TRAJECTORY PASS rows=19432 obligations={} runtime={} source_invariants={} snapshots={} calls={}",
        totals.values,
        totals.runtime_values,
        totals.source_invariants,
        totals.snapshots,
        totals.calls
    );
}

#[test]
fn checker_rejects_changed_original_seed_before_advancing_a_frame() {
    let text = include_str!("../tests/fixtures/workload_trajectory.tsv");
    let changed = text.replacen("R\t0\t25214089609\t", "R\t0\t25214089610\t", 1);
    assert_ne!(changed, text, "Original seed mutation anchor missing");
    assert!(std::panic::catch_unwind(|| compare_trajectories(&changed)).is_err());
}

#[test]
fn compact_512_tick_workload_matches_original_scalar_and_rng_observations() {
    let totals = compare_evolving_trajectory(include_str!(
        "../tests/fixtures/workload_trajectory_512_ticks.tsv"
    ));
    println!(
        "EVOLVING_WORLD_512 PASS rows=29914 obligations={} runtime={} source_invariants={} snapshots={} calls={}",
        totals.values,
        totals.runtime_values,
        totals.source_invariants,
        totals.snapshots,
        totals.calls
    );
}

pub(super) fn dense_rollover_world(text: &str) -> (World, Fields) {
    let (totals, world, settings) = compare_trajectory_rows(text, TrajectoryMode::Dense256);
    let tag_counts: [(&str, usize); 11] = [
        ("C", 1),
        ("P", 10),
        ("U", 147),
        ("X", 2),
        ("K", 256),
        ("V", 2313),
        ("S", 9509),
        ("W", 2313),
        ("R", 257),
        ("Q", 257),
        ("E", 1),
    ];
    assert_eq!(
        totals.rows,
        tag_counts.into_iter().map(|(k, v)| (k.into(), v)).collect()
    );
    assert_eq!(totals.rows.values().sum::<usize>(), 15066);
    assert_eq!(
        (
            totals.cohorts,
            totals.calls,
            totals.snapshots,
            totals.values
        ),
        (1, 256, 257, 15677)
    );
    assert_eq!(
        (totals.runtime_values, totals.source_invariants),
        (15163, 514)
    );
    println!("DENSE_ROLLOVER_TRAJECTORY PASS rows=15066 obligations=15677 runtime=15163 source_invariants=514 snapshots=257 calls=256");
    (world.expect("Dense rollover retains the replayed World"), settings)
}
