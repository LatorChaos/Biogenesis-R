// SPDX-License-Identifier: GPL-2.0-or-later

//! Observe the first actual IncomingOracle diagnostic without deriving constructors from output rows

use super::death_oracle_tests::{statistics_schemas, world_schema};
use super::maintenance_oracle_tests::{WORLD_EXCLUSIONS, instance_schema, world_views};
use super::*;
use crate::organism::{GenomeRef, OrganismRef};
use crate::statistics::tests::{assert_observation, observed_field, observed_getter};
use std::rc::Rc;

type Rows = Vec<Vec<&'static str>>;

fn source_rows() -> Rows {
    let rows: Rows = include_str!("../tests/fixtures/incoming_diagnostic.tsv")
        .lines()
        .take_while(|line| *line != "E")
        .map(|line| line.split('\t').collect())
        .collect();
    assert_eq!(
        rows[0],
        [
            "C",
            "empty-genome-constructor-and-nonorigin-paste",
            "incoming-diagnostic",
            "827364"
        ]
    );
    rows
}

fn input_genome(rows: &Rows, label: &str) -> Genome {
    assert!(rows.iter().any(|r| r.as_slice()
        == [
            "I",
            "setup",
            &format!("genome:{label}:construction"),
            "String",
            "explicit"
        ]));
    let mut stage = Stage::default();
    for row in rows {
        if row[0] == "G" && row[1] == "input" && row[2] == label {
            assert_eq!(row.len(), 5);
            assert!(stage.genome.insert(row[3].into(), row[4].into()).is_none());
        }
        if row[0] == "N" && row[1] == "input" && row[2] == label {
            assert_eq!(row.len(), 6);
            assert!(
                stage
                    .genes
                    .entry(row[3].parse().unwrap())
                    .or_default()
                    .insert(row[4].into(), row[5].into())
                    .is_none()
            );
        }
    }
    assert_eq!(stage.genome.len(), 28);
    assert_eq!(stage.genome["_reference"], label);
    assert_eq!(
        stage.genes.len(),
        stage.genome["_nGenes"].parse::<usize>().unwrap()
    );
    assert!(stage.genes.values().all(|fields| fields.len() == 40));
    explicit_genome(&stage)
}

fn snapshot(
    rows: &Rows,
    stage: &str,
    world: &World,
    actor: &Organism,
    genomes: &BTreeMap<String, GenomeRef>,
    actors: &BTreeMap<String, OrganismRef>,
) -> usize {
    let view = View::from(actor);
    let mut actual = organism_fields(&view);
    for key in ["_world", "_visibleWorld"] {
        actual.remove(key);
    }
    let expected: Fields = rows
        .iter()
        .filter(|r| {
            r[0] == "O"
                && r[1] == stage
                && r[2] == "victim"
                && !["_world", "_visibleWorld"].contains(&r[3])
        })
        .map(|r| (r[3].to_owned(), r[5].to_owned()))
        .collect();
    assert_eq!(expected.len(), 152);
    assert_eq!(actual, expected, "{stage} complete victim state");
    let mut count = 152;
    let mut code = genome_fields(&actor.genome.as_ref().unwrap().borrow());
    code.insert("_reference".into(), "g0".into());
    code.insert(
        "_nGenes".into(),
        actor.genome.as_ref().unwrap().borrow().genes().unwrap().len().to_string(),
    );
    let expected: Fields = rows
        .iter()
        .filter(|r| r[0] == "G" && r[1] == stage && r[2] == "g0")
        .map(|r| (r[3].to_owned(), r[4].to_owned()))
        .collect();
    assert_eq!(code, expected, "{stage} genotype");
    count += 28;
    for name in ["genotype", "infected", "saved"] {
        let row = rows
            .iter()
            .find(|r| r[0] == "H" && r[1] == stage && r[2] == "victim" && r[3] == name)
            .unwrap();
        match name {
            "genotype" => {
                assert_eq!(row[4], "g0");
                assert!(Rc::ptr_eq(actor.genome.as_ref().unwrap(),  &genomes["g0"]));
            }
            "infected" => {
                assert_eq!(row[4], "~");
                assert!(actor.infected_genome.is_none());
            }
            "saved" => {
                assert_eq!(row[4], "~");
                assert!(actor.saved_genome.is_none());
            }
            _ => unreachable!(),
        }
        count += 1;
    }
    let (world_getters, world_instances) = world_views(world, actors);
    let (statistics, getters) = statistics_schemas();
    let mut groups = BTreeMap::<&str, usize>::new();
    for row in rows
        .iter()
        .filter(|r| ["W", "V", "S", "T"].contains(&r[0]) && r[1] == stage)
    {
        if row[0] == "V" && WORLD_EXCLUSIONS.contains(&row[2]) {
            continue;
        }
        match row[0] {
            "W" => {
                assert_eq!(world_schema()[row[2]], row[3]);
                assert_eq!(world_getters[row[2]], row[4], "{stage} W {}", row[2]);
            }
            "V" => {
                assert_eq!(instance_schema()[row[2]], row[3]);
                assert_eq!(world_instances[row[2]], row[4], "{stage} V {}", row[2]);
            }
            "S" => {
                assert_eq!(statistics[row[2]], row[3]);
                assert_observation(
                    observed_field(&world.statistics_ref().borrow(), row[2]),
                    row[3],
                    row[4],
                    genomes,
                    stage,
                );
            }
            "T" => {
                assert_eq!(getters[row[2]], row[3]);
                assert_observation(
                    observed_getter(&world.statistics_ref().borrow(), row[2]),
                    row[3],
                    row[4],
                    genomes,
                    stage,
                );
            }
            _ => unreachable!(),
        }
        *groups.entry(row[0]).or_default() += 1;
        count += 1;
    }
    assert_eq!(
        groups,
        BTreeMap::from([("W", 10), ("V", 11), ("S", 52), ("T", 49)])
    );
    let expected = rows.iter().find(|r| r[0] == "R" && r[1] == stage).unwrap();
    let mut random = world.random_snapshot();
    assert_eq!(
        [
            random.next_i64().to_string(),
            random.next_bounded(10000).unwrap().to_string(),
            bits(random.next_f64())
        ],
        [expected[2], expected[3], expected[4]]
    );
    count + 1
}

#[test]
fn empty_genome_constructor_and_successful_paste_match_actual_original() {
    let rows = source_rows();
    let (schema, _) = primitive_reference_schemas();
    let settings: Fields = rows
        .iter()
        .filter(|r| r[0] == "U")
        .map(|r| {
            assert_eq!(r[2], schema[r[1]]);
            (r[1].to_owned(), r[3].to_owned())
        })
        .collect();
    assert_eq!(settings.len(), 147);
    let world = World::empty_with_parameters(
        WorldConfig {
            seed: rows[0][3].parse().unwrap(),
            width: integer(&settings, "WORLD_WIDTH").try_into().unwrap(),
            height: integer(&settings, "WORLD_HEIGHT").try_into().unwrap(),
            initial_attempts: integer(&settings, "INITIAL_ORGANISMS").try_into().unwrap(),
        },
        parameters(&settings),
    )
    .unwrap();
    for row in rows
        .iter()
        .filter(|r| r[0] == "I" && r[1] == "setup" && r[2].starts_with("world:"))
    {
        assert_eq!(row[3], "double");
        match row[2] {
            "world:_CO2" => world.carbon_dioxide.set(decoded(row[4])),
            "world:_O2" => world.oxygen.set(decoded(row[4])),
            "world:_CH4" => world.methane.set(decoded(row[4])),
            _ => panic!("Unexpected setup input"),
        }
    }
    let g0 = Rc::new(RefCell::new(input_genome(&rows, "g0")));
    let g2 = Rc::new(RefCell::new(input_genome(&rows, "g2")));
    let add = rows.iter().find(|r| r[0] == "A").unwrap();
    assert_eq!(&add[1..3], &["selector", "g2"]);
    let selector = world
        .construct_pasted_shared(
            Rc::clone(&g2),
            [add[3].parse().unwrap(), add[4].parse().unwrap()],
        )
        .unwrap()
        .unwrap();
    let selector = Rc::new(RefCell::new(selector));
    world.add_organism(Rc::clone(&selector), None);
    let actors = BTreeMap::from([("selector".into(), selector)]);
    let genomes = BTreeMap::from([("g0".into(), Rc::clone(&g0)), ("g2".into(), g2)]);
    let calls: Vec<_> = rows.iter().filter(|r| r[0] == "K").collect();
    assert_eq!(&calls[0][1..], &["step-0", "construct", "victim", "g0"]);
    let mut actor = world.new_pasted_candidate(g0);
    assert!(!actor.creation_complete);
    let mut compared = snapshot(&rows, "step-0", &world, &actor, &genomes, &actors);
    assert_eq!(&calls[1][1..4], &["step-1", "paste", "victim"]);
    let center: Vec<i32> = calls[1][4]
        .split(',')
        .map(|value| value.parse().unwrap())
        .collect();
    assert_eq!(center.len(), 2);
    let outcome = rows
        .iter()
        .find(|r| r[0] == "B" && r[1] == "step-1" && r[2] == "paste")
        .unwrap();
    assert_eq!(
        world.attempt_paste(None, &mut actor, [center[0], center[1]]),
        Ok(outcome[3].parse::<bool>().unwrap())
    );
    compared += 1;
    assert!(actor.creation_complete);
    compared += snapshot(&rows, "step-1", &world, &actor, &genomes, &actors);
    assert_eq!(compared, 613);
    eprintln!(
        "EMPTY_SOURCE_LEDGER exact=613 source_collaborator_rows=12 scope=first_empty_case_step0_step1_only"
    );
}
