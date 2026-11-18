// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay real empty Java frames; raw settings bypass guards only inside this test module

use super::death_oracle_tests::statistics_schemas;
use super::maintenance_oracle_tests::{WORLD_EXCLUSIONS, instance_schema, world_views};
use super::*;
use crate::statistics::tests::{assert_observation, observed_field, observed_getter};

type Typed = BTreeMap<String, (String, String)>;

#[derive(Default)]
struct Snapshot {
    world: Typed,
    statistics: Typed,
    getters: Typed,
    time: i64,
    suffix: (i64, i32, u64),
}

struct Case {
    name: String,
    seed: i64,
    settings: Fields,
    inputs: Fields,
    before: Snapshot,
    after: Snapshot,
}

struct Rows<'a> {
    rows: Vec<Vec<&'a str>>,
    at: usize,
}

impl<'a> Rows<'a> {
    fn next(&mut self, tag: &str, size: usize) -> Vec<&'a str> {
        let row = self.rows[self.at].clone();
        self.at += 1;
        assert_eq!(row.len(), size, "row {}", self.at);
        assert_eq!(row[0], tag, "row {}", self.at);
        row
    }

    fn typed(&mut self, tag: &str, stage: &str, schema: &BTreeMap<String, String>) -> Typed {
        schema
            .iter()
            .map(|(field, kind)| {
                let row = self.next(tag, 5);
                assert_eq!(
                    (row[1], row[2], row[3]),
                    (stage, field.as_str(), kind.as_str())
                );
                ((field.to_owned()), (kind.to_owned(), row[4].to_owned()))
            })
            .collect()
    }

    fn snapshot(&mut self, stage: &str) -> Snapshot {
        let schema = instance_schema()
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        let (fields, getters) = statistics_schemas();
        let world = self.typed("W", stage, &schema);
        let statistics = self.typed("S", stage, &fields);
        let getters = self.typed("T", stage, &getters);
        let row = self.next("G", 5);
        assert_eq!(&row[1..4], &[stage, "World.getTime", "long"]);
        let time = row[4].parse().unwrap();
        let row = self.next("R", 5);
        assert_eq!(row[1], stage);
        let suffix = (
            row[2].parse().unwrap(),
            row[3].parse().unwrap(),
            u64::from_str_radix(row[4], 16).unwrap(),
        );
        Snapshot {
            world,
            statistics,
            getters,
            time,
            suffix,
        }
    }
}

fn parse(text: &str, diagnostic: bool) -> Vec<Case> {
    let mut rows = Rows {
        rows: text
            .lines()
            .map(|line| line.split('\t').collect())
            .collect(),
        at: 0,
    };
    let total = rows.rows.len();
    let mut cases = Vec::new();
    let mut names = BTreeSet::new();
    while rows.at < total {
        let row = rows.next("C", 4);
        assert!(names.insert(row[1].to_owned()));
        assert_eq!(
            row[2],
            if diagnostic {
                "ui-hook-diagnostic"
            } else {
                "raw-empty-frame"
            }
        );
        let name = row[1].to_owned();
        let seed = row[3].parse().unwrap();
        let mut settings = Fields::new();
        for (key, kind) in [
            ("INITIAL_ORGANISMS", "int"),
            ("INITIAL_O2", "double"),
            ("INITIAL_CO2", "double"),
            ("INITIAL_CH4", "double"),
            ("WORLD_WIDTH", "int"),
            ("WORLD_HEIGHT", "int"),
            ("CO2_TO_CH4_DIVISOR", "int"),
            ("CH4_TO_CO2_DIVISOR", "int"),
        ] {
            let row = rows.next("U", 4);
            assert_eq!((row[1], row[2]), (key, kind));
            super::death_oracle_tests::validate(kind, row[3]);
            assert!(settings.insert(key.to_owned(), row[3].to_owned()).is_none());
        }
        let mut inputs = Fields::new();
        for (key, kind) in [
            ("_CO2", "double"),
            ("_CH4", "double"),
            ("_O2", "double"),
            ("nFrames", "int"),
        ] {
            let row = rows.next("I", 4);
            assert_eq!((row[1], row[2]), (key, kind));
            super::death_oracle_tests::validate(kind, row[3]);
            assert!(inputs.insert(key.to_owned(), row[3].to_owned()).is_none());
        }
        let before = rows.snapshot("before");
        assert_eq!(rows.next("K", 2)[1], "time");
        let row = rows.next("B", 3);
        assert_eq!(
            &row[1..],
            &["completed", if diagnostic { "false" } else { "true" }]
        );
        if diagnostic {
            assert_eq!(
                &rows.next("X", 3)[1..],
                &["diagnostic_exception", "java.lang.NullPointerException"]
            );
        }
        assert_eq!(&rows.next("B", 3)[1..], &["tree_replaced", "true"]);
        assert_eq!(&rows.next("B", 3)[1..], &["random_unchanged", "true"]);
        let after = rows.snapshot("after");
        rows.next("E", 1);
        cases.push(Case {
            name,
            seed,
            settings,
            inputs,
            before,
            after,
        });
    }
    assert_eq!(rows.at, total);
    if diagnostic {
        assert_eq!((cases.len(), total), (2, 510));
    } else {
        assert_eq!((cases.len(), total), (39, 9_906));
        assert_eq!(39 * (230 + 8 + 1 + 15), total);
    }
    cases
}

fn make_world(case: &Case) -> World {
    assert_eq!(integer(&case.settings, "INITIAL_ORGANISMS"), 0);
    let mut parameters = WorldParameters::default();
    parameters.atmosphere.initial_oxygen = real(&case.settings, "INITIAL_O2");
    parameters.atmosphere.initial_carbon_dioxide = real(&case.settings, "INITIAL_CO2");
    parameters.atmosphere.initial_methane = real(&case.settings, "INITIAL_CH4");
    let world = World::empty_with_parameters(
        WorldConfig {
            seed: case.seed,
            width: integer(&case.settings, "WORLD_WIDTH").try_into().unwrap(),
            height: integer(&case.settings, "WORLD_HEIGHT").try_into().unwrap(),
            initial_attempts: 0,
        },
        parameters,
    )
    .unwrap();
    // The Java constructor ignores these globals; install raw inputs after the guarded Rust constructor
    world.session.0.parameters.borrow_mut().atmosphere.carbon_to_methane_divisor =
        integer(&case.settings, "CO2_TO_CH4_DIVISOR");
    world.session.0.parameters.borrow_mut().atmosphere.methane_to_carbon_divisor =
        integer(&case.settings, "CH4_TO_CO2_DIVISOR");
    world.carbon_dioxide.set(real(&case.inputs, "_CO2"));
    world.methane.set(real(&case.inputs, "_CH4"));
    world.oxygen.set(real(&case.inputs, "_O2"));
    world.frame.set(integer(&case.inputs, "nFrames"));
    world
}

fn compare_snapshot(case: &Case, stage: &str, expected: &Snapshot, world: &World) -> usize {
    let context = format!("{} {stage}", case.name);
    assert!(
        world.organisms.borrow().is_empty() && world.frame_objects.borrow().is_empty(),
        "{context}"
    );
    let (_, mut actual) = world_views(world, &BTreeMap::new());
    // This observer records tree presence, unlike the shared member-count grammar
    let tree = actual["colDetTree"].split(',').next().unwrap().to_owned();
    actual.insert("colDetTree".into(), tree);
    assert_eq!(actual["worldStatistics"], "present");
    actual.insert("worldStatistics".into(), "s0".into());
    assert_eq!(actual.len(), 11);
    let mut compared = 0;
    for (key, (kind, value)) in &expected.world {
        assert_eq!(instance_schema()[key.as_str()], kind);
        if WORLD_EXCLUSIONS.contains(&key.as_str()) {
            continue;
        }
        assert_eq!(actual[key], *value, "{context} W {key}");
        compared += 1;
    }
    for (key, (kind, value)) in &expected.statistics {
        assert_observation(
            observed_field(&world.statistics_ref().borrow(), key),
            kind,
            value,
            &BTreeMap::new(),
            &format!("{context} S {key}"),
        );
        compared += 1;
    }
    for (key, (kind, value)) in &expected.getters {
        assert_observation(
            observed_getter(&world.statistics_ref().borrow(), key),
            kind,
            value,
            &BTreeMap::new(),
            &format!("{context} T {key}"),
        );
        compared += 1;
    }
    assert_eq!(world.time(), expected.time, "{context} World.getTime");
    compared += 1;
    let mut random = world.random_snapshot();
    let suffix = (
        random.next_i64(),
        random.next_bounded(10_000).unwrap(),
        random.next_f64().to_bits(),
    );
    assert_eq!(suffix, expected.suffix, "{context} RNG");
    compared + 1
}

#[test]
fn replay_actual_java_empty_atmosphere_frames_exactly() {
    let cases = parse(
        include_str!("../tests/fixtures/atmosphere_frame.tsv"),
        false,
    );
    let diagnostics = parse(
        include_str!("../tests/fixtures/atmosphere_frame_ui_diagnostic.tsv"),
        true,
    );
    assert_eq!(diagnostics.len(), 2);
    let mut compared = 0;
    for case in cases {
        let world = make_world(&case);
        let original_random = world.random_snapshot();
        let original_statistics = std::ptr::addr_of!(world.statistics);
        compared += compare_snapshot(&case, "before", &case.before, &world);
        // Exercise the public paused-frame boundary; its result must represent an executed frame
        assert!(world.single_step().unwrap(), "{} completed", case.name);
        compared += 1;
        assert_eq!(
            world.random_snapshot(), original_random,
            "{} random_unchanged",
            case.name
        );
        compared += 1;
        assert!(std::ptr::eq(
            std::ptr::addr_of!(world.statistics),
            original_statistics
        ));
        compared += compare_snapshot(&case, "after", &case.after, &world);
    }
    assert_eq!(compared, 8_970);
    eprintln!(
        "ATMOSPHERE_LEDGER exact=8970 collaborators=312 allocation_metadata=39 inputs_commands=585 diagnostics_source_only=510"
    );
}
