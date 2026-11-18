// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay independent Java death observations through real construction and lifecycle methods

use super::*;
use crate::ColorKind;
use crate::organism::{GenomeRef, OrganismRef};
use crate::statistics::tests::{
    assert_observation, observed_field, observed_getter, set_statistics_input,
};
use crate::world::contact_state::ContactActor;
use crate::world::death::Killer;
use crate::world::infection::{InfectionKind, Infector};
use std::rc::Rc;

#[derive(Default)]
struct Snapshot {
    organisms: BTreeMap<String, Fields>,
    world: Fields,
    statistics: BTreeMap<String, (String, String)>,
    getters: BTreeMap<String, (String, String)>,
    genomes: BTreeMap<String, Stage>,
    references: BTreeMap<(String, String), String>,
    suffix: Option<(i64, i32, u64)>,
}

enum Step {
    Actor {
        role: String,
        genome: String,
        center: [i32; 2],
    },
    Input {
        stage: String,
        target: String,
        kind: String,
        value: String,
    },
    Call {
        stage: String,
        operation: String,
        arguments: Vec<String>,
    },
    Snapshot(String),
}

struct DeathCase {
    name: String,
    mode: String,
    seed: i64,
    settings: Fields,
    input_genomes: BTreeMap<String, Stage>,
    snapshots: BTreeMap<String, Snapshot>,
    boundary_results: BTreeMap<String, bool>,
    steps: Vec<Step>,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct Coverage {
    pub(super) cases: usize,
    pub(super) snapshots: usize,
    pub(super) calls: BTreeMap<String, usize>,
    pub(super) compared_rows: usize,
    pub(super) collaborator_rows: usize,
    pub(super) selection_metadata_rows: usize,
    pub(super) input_genotype_rows: usize,
    pub(super) input_and_command_rows: usize,
}

pub(super) fn validate(kind: &str, value: &str) {
    if let Some(element) = kind.strip_suffix("[]") {
        if value != "~" && !value.is_empty() {
            for item in value.split(',') {
                validate(element, item);
            }
        }
        return;
    }
    match kind {
        "int" => {
            value.parse::<i32>().unwrap();
        }
        "long" => {
            value.parse::<i64>().unwrap();
        }
        "double" => {
            assert_eq!(value.len(), 16);
            u64::from_str_radix(value, 16).unwrap();
        }
        "boolean" => {
            value.parse::<bool>().unwrap();
        }
        "String" => {}
        "Color" => {
            if value != "~" {
                assert_eq!(value.len(), 8);
                u32::from_str_radix(value, 16).unwrap();
            }
        }
        "Rectangle" => {
            input_rectangle(value);
        }
        "World" | "VisibleWorld" | "GeneticCode" => {
            assert!(["present", "~"].contains(&value));
        }
        "List" => {
            if !value.is_empty() {
                for item in value.split(',') {
                    validate("double", item);
                }
            }
        }
        kind => panic!("Unknown death field type {kind}"),
    }
}

fn input_rectangle(value: &str) -> IntRect {
    let values: Vec<i32> = value.split(',').map(|item| item.parse().unwrap()).collect();
    assert_eq!(values.len(), 4);
    IntRect {
        x: values[0],
        y: values[1],
        width: values[2],
        height: values[3],
    }
}

pub(super) fn statistics_schemas() -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let mut fields = BTreeMap::new();
    let mut getters = BTreeMap::new();
    for line in include_str!("../tests/fixtures/statistics.tsv")
        .lines()
        .take_while(|line| *line != "E")
    {
        let row: Vec<_> = line.split('\t').collect();
        if ["S", "T"].contains(&row[0]) {
            assert_eq!(row.len(), 5);
            let target = if row[0] == "S" {
                &mut fields
            } else {
                &mut getters
            };
            if let Some(previous) = target.insert(row[2].to_owned(), row[3].to_owned()) {
                assert_eq!(previous, row[3]);
            }
        }
    }
    assert_eq!((fields.len(), getters.len()), (52, 49));
    (fields, getters)
}

pub(super) fn world_schema() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("_population", "int"),
        ("organism_count", "int"),
        ("corpses", "int"),
        ("_O2", "double"),
        ("_CO2", "double"),
        ("_CH4", "double"),
        ("NEXT_ID", "int"),
        ("nFrames", "long"),
        ("time", "long"),
        ("organism_ids", "int[]"),
    ])
}

fn parse(source: &str, coverage: &mut Coverage) -> Vec<DeathCase> {
    let (setting_types, organism_types) = primitive_reference_schemas();
    let (statistic_types, getter_types) = statistics_schemas();
    let mut cases = Vec::new();
    let mut current: Option<DeathCase> = None;
    let mut names = BTreeSet::new();
    let mut stage = String::new();
    for (line_index, line) in source.lines().enumerate() {
        assert!(!line.is_empty(), "Empty death row at {}", line_index + 1);
        let row: Vec<_> = line.split('\t').collect();
        if row[0] == "C" {
            assert_eq!(row.len(), 4);
            assert!(current.is_none());
            assert!(names.insert(row[1]));
            current = Some(DeathCase {
                name: row[1].to_owned(),
                mode: row[2].to_owned(),
                seed: row[3].parse().unwrap(),
                settings: Fields::new(),
                input_genomes: BTreeMap::new(),
                snapshots: BTreeMap::new(),
                boundary_results: BTreeMap::new(),
                steps: Vec::new(),
            });
            stage.clear();
            coverage.cases += 1;
            coverage.input_and_command_rows += 1;
            continue;
        }
        let case = current.as_mut().expect("Death row outside a case");
        match row[0] {
            "U" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[2], setting_types[row[1]]);
                validate(row[2], row[3]);
                assert!(
                    case.settings
                        .insert(row[1].to_owned(), row[3].to_owned())
                        .is_none()
                );
                coverage.input_and_command_rows += 1;
            }
            "A" => {
                assert_eq!(row.len(), 5);
                case.steps.push(Step::Actor {
                    role: row[1].to_owned(),
                    genome: row[2].to_owned(),
                    center: [row[3].parse().unwrap(), row[4].parse().unwrap()],
                });
                coverage.input_and_command_rows += 1;
            }
            "I" => {
                assert_eq!(row.len(), 5);
                if row[3] != "GeneticCode" {
                    validate(row[3], row[4]);
                }
                case.steps.push(Step::Input {
                    stage: row[1].to_owned(),
                    target: row[2].to_owned(),
                    kind: row[3].to_owned(),
                    value: row[4].to_owned(),
                });
                if row[2] == "visible:selected" {
                    assert_eq!(row[3], "String");
                    assert!(
                        case.steps
                            .iter()
                            .any(|step| matches!(step, Step::Actor { role, .. } if role == row[4]))
                    );
                    coverage.selection_metadata_rows += 1;
                } else {
                    coverage.input_and_command_rows += 1;
                }
            }
            "K" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[3], "victim");
                match row[2] {
                    "die" => assert!(["killer", "self", "~"].contains(&row[4])),
                    "isInsideWorld" | "grow" | "offset_velocity" => assert_eq!(row[4], "~"),
                    "bounds" => validate("boolean", row[4]),
                    "infectedByCode" => {
                        assert!(row[4] == "~" || case.input_genomes.contains_key(row[4]))
                    }
                    "infectedBy" | "infectedByFruit" | "infectedByFruitWeak" => {
                        assert!(["killer", "self"].contains(&row[4]))
                    }
                    "survive" | "inhibited" | "frozen" | "paralyzed" | "neutralized" => {
                        assert!(["killer", "self"].contains(&row[4]))
                    }
                    "segmentsRestoreEffects"
                    | "virusneutralized"
                    | "lavenderinhibition"
                    | "lavendershield"
                    | "weaklavendershield"
                    | "segmentsFrameEffects"
                    | "leafExtraEffects"
                    | "jadeExtraEffects"
                    | "segmentsHealingEffects"
                    | "segmentsExtraEffects"
                    | "segmentsExtraPlantsEffects"
                    | "plantsUpdateEffects"
                    | "segmentsUpdateEffects"
                    | "darkfireRestoreEffects" => assert_eq!(row[4], "~"),
                    "pretouchEffects"
                    | "touchfriendEffects"
                    | "touchfriendReaction"
                    | "touchReaction"
                    | "touchfriendBrake"
                    | "touchBrake"
                    | "touchHatch"
                    | "touchEffects1.ORANGE"
                    | "touchEffects1.FIRE"
                    | "touchEffects1.RED"
                    | "touchEffects1.PINK"
                    | "touchEffects1.MAROON"
                    | "touchEffects1.CREAM"
                    | "touchEffects1.LILAC"
                    | "touchEffects1.SPIKEPOINT"
                    | "touchEffects1.OCHRE"
                    | "touchEffects1.SILVER"
                    | "touchEffects2.WHITE"
                    | "touchEffects2.FRUIT"
                    | "touchEffects2.VIOLET"
                    | "touchEffects2.SKY"
                    | "touchEffects2.OLIVE"
                    | "touchEffects2.GRAY" => {
                        let arguments: Vec<_> = row[4].split(',').collect();
                        assert_eq!(arguments.len(), 3);
                        assert_eq!(arguments[0], "killer");
                        arguments[1].parse::<usize>().unwrap();
                        arguments[2].parse::<usize>().unwrap();
                    }
                    operation => panic!("Unknown death operation {operation}"),
                }
                case.steps.push(Step::Call {
                    stage: row[1].to_owned(),
                    operation: row[2].to_owned(),
                    arguments: row[3..].iter().map(|item| (*item).to_owned()).collect(),
                });
                *coverage.calls.entry(row[2].to_owned()).or_default() += 1;
                coverage.input_and_command_rows += 1;
            }
            "P" => {
                assert_eq!(row.len(), 2);
                stage = row[1].to_owned();
                assert!(
                    case.snapshots
                        .insert(stage.clone(), Snapshot::default())
                        .is_none()
                );
                case.steps.push(Step::Snapshot(stage.clone()));
                coverage.input_and_command_rows += 1;
                coverage.snapshots += 1;
            }
            "O" => {
                assert_eq!(row.len(), 6);
                assert_eq!(row[1], stage);
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
                if ["_world", "_visibleWorld"].contains(&row[3]) {
                    assert_eq!(row[5], "present");
                    coverage.collaborator_rows += 1;
                } else {
                    coverage.compared_rows += 1;
                }
            }
            "H" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], stage);
                assert!(
                    case.steps
                        .iter()
                        .any(|step| matches!(step, Step::Actor { role, .. } if role == row[2]))
                );
                assert!(["genotype", "infected", "saved"].contains(&row[3]));
                assert!(row[4] == "~" || ["g0", "g1", "g2", "g3", "g4"].contains(&row[4]));
                assert!(
                    case.snapshots
                        .get_mut(row[1])
                        .unwrap()
                        .references
                        .insert((row[2].to_owned(), row[3].to_owned()), row[4].to_owned())
                        .is_none()
                );
                coverage.compared_rows += 1;
            }
            "W" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], stage);
                assert_eq!(row[3], world_schema()[row[2]]);
                validate(row[3], row[4]);
                assert!(
                    case.snapshots
                        .get_mut(row[1])
                        .unwrap()
                        .world
                        .insert(row[2].to_owned(), row[4].to_owned())
                        .is_none()
                );
                coverage.compared_rows += 1;
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
                if row[3] != "GeneticCode" {
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
                coverage.compared_rows += 1;
            }
            "G" | "N" => {
                assert_eq!(row.len(), if row[0] == "G" { 5 } else { 6 });
                let genomes = if row[1] == "input" {
                    coverage.input_genotype_rows += 1;
                    &mut case.input_genomes
                } else {
                    assert_eq!(row[1], stage);
                    coverage.compared_rows += 1;
                    &mut case.snapshots.get_mut(row[1]).unwrap().genomes
                };
                let genome = genomes.entry(row[2].to_owned()).or_default();
                if row[0] == "G" {
                    assert!(
                        genome
                            .genome
                            .insert(row[3].to_owned(), row[4].to_owned())
                            .is_none()
                    );
                } else {
                    assert!(
                        genome
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
                coverage.compared_rows += 1;
            }
            "B" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[2], "inside");
                assert!(
                    case.boundary_results
                        .insert(row[1].to_owned(), row[3].parse().unwrap())
                        .is_none()
                );
                coverage.compared_rows += 1;
            }
            "E" => {
                assert_eq!(row.len(), 1);
                assert_eq!(case.mode, "canonical");
                assert_eq!(
                    case.settings.keys().collect::<BTreeSet<_>>(),
                    setting_types.keys().collect()
                );
                let actor_roles: BTreeSet<_> = case
                    .steps
                    .iter()
                    .filter_map(|step| match step {
                        Step::Actor { role, .. } => Some(role),
                        _ => None,
                    })
                    .collect();
                assert_eq!(actor_roles.len(), 3);
                for snapshot in case.snapshots.values() {
                    assert!(snapshot.suffix.is_some());
                    assert_eq!(snapshot.references.len(), 9);
                    assert_eq!(
                        snapshot.organisms.keys().collect::<BTreeSet<_>>(),
                        actor_roles
                    );
                    for organism in snapshot.organisms.values() {
                        assert_eq!(
                            organism.keys().collect::<BTreeSet<_>>(),
                            organism_types.keys().collect()
                        );
                    }
                    assert_eq!(
                        snapshot.statistics.keys().collect::<BTreeSet<_>>(),
                        statistic_types.keys().collect()
                    );
                    assert_eq!(
                        snapshot.getters.keys().collect::<BTreeSet<_>>(),
                        getter_types.keys().collect()
                    );
                    assert_eq!(
                        snapshot
                            .world
                            .keys()
                            .map(String::as_str)
                            .collect::<BTreeSet<_>>(),
                        world_schema().keys().copied().collect()
                    );
                }
                let boundary_calls: BTreeSet<_> = case
                    .steps
                    .iter()
                    .filter_map(|step| match step {
                        Step::Call {
                            stage, operation, ..
                        } if operation == "isInsideWorld" => Some(stage),
                        _ => None,
                    })
                    .collect();
                assert_eq!(boundary_calls, case.boundary_results.keys().collect());
                cases.push(current.take().unwrap());
                coverage.input_and_command_rows += 1;
            }
            kind => panic!("Unknown death row {kind} at {}", line_index + 1),
        }
    }
    assert!(current.is_none());
    cases
}

pub(super) fn validate_genome(stage: &Stage, gene_schema: &BTreeSet<String>) {
    let code = Genome::from_parts(&[], GenomeProperties::default());
    let schema = genome_fields(&code);
    let expected_keys: BTreeSet<_> = schema
        .keys()
        .cloned()
        .chain(["_reference".to_owned(), "_nGenes".to_owned()])
        .collect();
    assert_eq!(
        stage.genome.keys().cloned().collect::<BTreeSet<_>>(),
        expected_keys
    );
    assert_eq!(
        stage.genes.len(),
        integer(&stage.genome, "_nGenes") as usize
    );
    assert!(stage.genome["_reference"].starts_with('g'));
    stage.genome["_reference"][1..].parse::<usize>().unwrap();
    for (field, value) in &stage.genome {
        match field.as_str() {
            "_reference" | "_cladeID" => {}
            "_nGenes" => validate("int", value),
            _ => validate(
                if ["true", "false"].contains(&schema[field].as_str()) {
                    "boolean"
                } else {
                    "int"
                },
                value,
            ),
        }
    }
    for (index, (position, gene)) in stage.genes.iter().enumerate() {
        assert_eq!(index, *position);
        assert_eq!(gene.keys().cloned().collect::<BTreeSet<_>>(), *gene_schema);
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

pub(super) fn compare_genomes(
    case_name: &str,
    stage: &str,
    expected: &BTreeMap<String, Stage>,
    actual: &BTreeMap<String, GenomeRef>,
    errors: &mut Vec<String>,
) {
    assert_eq!(
        expected.keys().collect::<BTreeSet<_>>(),
        actual.keys().collect()
    );
    for (label, observation) in expected {
        assert_eq!(&observation.genome["_reference"], label);
        let actual_code = &actual[label];
        for (other_label, other_code) in actual {
            assert_eq!(
                Rc::ptr_eq(actual_code, other_code),
                label == other_label,
                "{} {stage} genotype identity {label}/{other_label}",
                case_name
            );
        }
        let code = actual_code.borrow();
        let mut fields = genome_fields(&code);
        fields.insert("_reference".to_owned(), label.clone());
        fields.insert("_nGenes".to_owned(), code.genes().unwrap().len().to_string());
        compare_fields(
            case_name,
            stage,
            &format!("G:{label}"),
            &fields,
            &observation.genome,
            errors,
        );
        assert_eq!(code.genes().unwrap().len(), observation.genes.len());
        for (index, gene) in &observation.genes {
            compare_fields(
                case_name,
                stage,
                &format!("N:{label}:{index}"),
                &gene_fields(&code.gene_ref(*index).unwrap().borrow()),
                gene,
                errors,
            );
        }
    }
}

fn compare_snapshot(
    case: &DeathCase,
    name: &str,
    world: &World,
    actors: &BTreeMap<String, OrganismRef>,
    genomes: &BTreeMap<String, GenomeRef>,
    errors: &mut Vec<String>,
) {
    let expected = &case.snapshots[name];
    assert_eq!(
        actors.keys().collect::<BTreeSet<_>>(),
        expected.organisms.keys().collect()
    );
    for (role, target) in &expected.organisms {
        let actor = actors[role].borrow();
        let supplied_genome = case
            .steps
            .iter()
            .find_map(|step| match step {
                Step::Actor {
                    role: supplied_role,
                    genome,
                    ..
                } if supplied_role == role => Some(genome),
                _ => None,
            })
            .unwrap();
        assert!(
            Rc::ptr_eq(actor.genome.as_ref().unwrap(),  &genomes[supplied_genome]),
            "{} {name} {role} genotype identity",
            case.name
        );
        assert!(
            world.organisms.borrow()
                .iter()
                .any(|member| Rc::ptr_eq(member, &actors[role]))
        );
        let mut actual = organism_fields(&View::from(&*actor));
        let mut target = target.clone();
        for collaborator in ["_world", "_visibleWorld"] {
            assert_eq!(target.remove(collaborator).as_deref(), Some("present"));
            assert!(actual.remove(collaborator).is_some());
        }
        compare_fields(&case.name, name, "O", &actual, &target, errors);
    }
    for ((role, holder), reference) in &expected.references {
        let actor = actors[role].borrow();
        let actual = match holder.as_str() {
            "genotype" => actor.genome.as_ref(),
            "infected" => actor.infected_genome.as_ref(),
            "saved" => actor.saved_genome.as_ref(),
            holder => panic!("Unknown death reference holder {holder}"),
        };
        if reference == "~" {
            assert!(
                actual.is_none(),
                "{} {name} {role}:{holder} must be null",
                case.name
            );
        } else {
            assert!(
                Rc::ptr_eq(
                    actual.expect("Missing death genotype reference"),
                    &genomes[reference]
                ),
                "{} {name} {role}:{holder} reference {reference}",
                case.name
            );
        }
    }
    let actual_world: Fields = expected
        .world
        .keys()
        .map(|field| {
            let value = match field.as_str() {
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
                field => panic!("Unknown death world field {field}"),
            };
            (field.clone(), value)
        })
        .collect();
    compare_fields(
        &case.name,
        name,
        "W",
        &actual_world,
        &expected.world,
        errors,
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
        "{} {name} RNG suffix",
        case.name
    );
}

fn call(
    case: &DeathCase,
    stage: &str,
    operation: &str,
    arguments: &[String],
    world: &mut World,
    actors: &BTreeMap<String, OrganismRef>,
    genomes: &BTreeMap<String, GenomeRef>,
) {
    match operation {
        "die" => {
            assert_eq!(arguments.len(), 2);
            let mut victim = actors[&arguments[0]].borrow_mut();
            match arguments[1].as_str() {
                "~" => world.die(&mut victim, None).unwrap(),
                "self" => world.die(&mut victim, Some(Killer::Victim)).unwrap(),
                killer => {
                    let mut killer = actors[killer].borrow_mut();
                    world.die(&mut victim, Some(Killer::Other(&mut killer))).unwrap();
                }
            }
        }
        "isInsideWorld" => {
            assert_eq!(arguments.len(), 2);
            assert_eq!(arguments[1], "~");
            let actual = world.is_inside_world(&mut actors[&arguments[0]].borrow_mut()).unwrap();
            assert_eq!(
                actual, case.boundary_results[stage],
                "{} {stage} inside",
                case.name
            );
        }
        "grow" | "offset_velocity" => {
            assert_eq!(arguments.len(), 2);
            assert_eq!(arguments[1], "~");
            let mut actor = actors[&arguments[0]].borrow_mut();
            match operation {
                "grow" => crate::movement::grow(&mut actor).unwrap(),
                "offset_velocity" => {
                    let displacement = actor.state.velocity;
                    let angle = actor.state.angular_velocity;
                    crate::movement::offset(&mut actor, displacement, angle);
                }
                _ => unreachable!(),
            }
        }
        "bounds" => {
            assert_eq!(arguments.len(), 2);
            let force = arguments[1].parse::<bool>().unwrap();
            crate::movement::calculate_bounds(&mut actors[&arguments[0]].borrow_mut(), force)
                .unwrap();
        }
        "infectedByCode" => {
            assert_eq!(arguments.len(), 2);
            let code = if arguments[1] == "~" {
                None
            } else {
                Some(Rc::clone(&genomes[&arguments[1]]))
            };
            world.infect_with_code(&mut actors[&arguments[0]].borrow_mut(), code).unwrap();
        }
        "infectedBy" | "infectedByFruit" | "infectedByFruitWeak" => {
            assert_eq!(arguments.len(), 2);
            let kind = match operation {
                "infectedBy" => InfectionKind::Direct,
                "infectedByFruit" => InfectionKind::Fruit,
                _ => InfectionKind::WeakFruit,
            };
            let mut recipient = actors[&arguments[0]].borrow_mut();
            if arguments[1] == "self" {
                world.infect_from(&mut recipient, Infector::Recipient, kind).unwrap();
            } else {
                world.infect_from(
                    &mut recipient,
                    Infector::Other(&mut actors[&arguments[1]].borrow_mut()),
                    kind,
                ).unwrap();
            }
        }
        "survive" | "inhibited" | "frozen" | "paralyzed" | "neutralized" => {
            let action = |world: &mut World, recipient: &mut Organism, actor: ContactActor<'_>| {
                match operation {
                    "survive" => world.survive_from(recipient, actor).unwrap(),
                    "inhibited" => world.inhibited_from(recipient, actor).unwrap(),
                    "frozen" => world.frozen_from(recipient, actor).unwrap(),
                    "paralyzed" => world.paralyzed_from(recipient, actor).unwrap(),
                    _ => world.neutralized_from(recipient, actor).unwrap(),
                }
            };
            let mut recipient = actors[&arguments[0]].borrow_mut();
            if arguments[1] == "self" {
                action(world, &mut recipient, ContactActor::Recipient);
            } else {
                action(
                    world,
                    &mut recipient,
                    ContactActor::Other(&mut actors[&arguments[1]].borrow_mut()),
                );
            }
        }
        "segmentsRestoreEffects" => {
            world.restore_contact_effects(&mut actors[&arguments[0]].borrow_mut()).unwrap();
        }
        "segmentsFrameEffects"
        | "leafExtraEffects"
        | "jadeExtraEffects"
        | "segmentsHealingEffects"
        | "segmentsExtraEffects"
        | "segmentsExtraPlantsEffects"
        | "plantsUpdateEffects"
        | "segmentsUpdateEffects"
        | "darkfireRestoreEffects" => {
            let mut actor = actors[&arguments[0]].borrow_mut();
            match operation {
                "segmentsFrameEffects" => world.segments_frame_effects(&mut actor).unwrap(),
                "leafExtraEffects" => world.leaf_extra_effects(&mut actor).unwrap(),
                "jadeExtraEffects" => world.jade_extra_effects(&mut actor).unwrap(),
                "segmentsHealingEffects" => world.segments_healing_effects(&mut actor).unwrap(),
                "segmentsExtraEffects" => world.segments_extra_effects(&mut actor).unwrap(),
                "segmentsExtraPlantsEffects" => {
                    world.segments_extra_plants_effects(&mut actor).unwrap()
                }
                "plantsUpdateEffects" => world.plants_update_effects(&mut actor).unwrap(),
                "segmentsUpdateEffects" => world.segments_update_effects(&mut actor).unwrap(),
                "darkfireRestoreEffects" => world.darkfire_restore_effects(&mut actor).unwrap(),
                _ => unreachable!(),
            }
        }
        "virusneutralized" | "lavenderinhibition" | "lavendershield" | "weaklavendershield" => {
            let mut actor = actors[&arguments[0]].borrow_mut();
            match operation {
                "virusneutralized" => world.virus_neutralized(&mut actor).unwrap(),
                "lavenderinhibition" => world.lavender_inhibition(&mut actor),
                "lavendershield" => world.lavender_shield(&mut actor),
                "weaklavendershield" => world.weak_lavender_shield(&mut actor),
                _ => unreachable!(),
            }
        }
        "pretouchEffects"
        | "touchfriendEffects"
        | "touchfriendReaction"
        | "touchReaction"
        | "touchfriendBrake"
        | "touchBrake"
        | "touchHatch"
        | "touchEffects1.ORANGE"
        | "touchEffects1.FIRE"
        | "touchEffects1.RED"
        | "touchEffects1.PINK"
        | "touchEffects1.MAROON"
        | "touchEffects1.CREAM"
        | "touchEffects1.LILAC"
        | "touchEffects1.SPIKEPOINT"
        | "touchEffects1.OCHRE"
        | "touchEffects1.SILVER"
        | "touchEffects2.WHITE"
        | "touchEffects2.FRUIT"
        | "touchEffects2.VIOLET"
        | "touchEffects2.SKY"
        | "touchEffects2.OLIVE"
        | "touchEffects2.GRAY" => {
            let parameters: Vec<_> = arguments[1].split(',').collect();
            let mut actor = actors[&arguments[0]].borrow_mut();
            let mut peer = actors[parameters[0]].borrow_mut();
            let segment: usize = parameters[1].parse().unwrap();
            let peer_segment: usize = parameters[2].parse().unwrap();
            let labelled = match operation {
                "touchEffects1.ORANGE" => Some(ColorKind::Orange),
                "touchEffects1.FIRE" => Some(ColorKind::Fire),
                "touchEffects1.RED" => Some(ColorKind::Red),
                "touchEffects1.PINK" => Some(ColorKind::Pink),
                "touchEffects1.MAROON" => Some(ColorKind::Maroon),
                "touchEffects1.CREAM" => Some(ColorKind::Cream),
                "touchEffects1.LILAC" => Some(ColorKind::Lilac),
                "touchEffects1.SPIKEPOINT" => Some(ColorKind::SpikePoint),
                "touchEffects1.OCHRE" => Some(ColorKind::Ochre),
                "touchEffects1.SILVER" => Some(ColorKind::Silver),
                _ => None,
            };
            if let Some(kind) = labelled {
                assert_eq!(actor.body.color_argb(segment).unwrap(), kind.argb().unwrap());
                world
                    .touch_effects_one(
                        &actors[&arguments[0]],
                        &mut actor,
                        &actors[parameters[0]],
                        &mut peer,
                        segment,
                        peer_segment,
                    )
                    .unwrap();
                return;
            }
            if let Some(kind) = match operation {
                "touchEffects2.WHITE" => Some(ColorKind::White),
                "touchEffects2.FRUIT" => Some(ColorKind::Fruit),
                "touchEffects2.VIOLET" => Some(ColorKind::Violet),
                "touchEffects2.SKY" => Some(ColorKind::Sky),
                "touchEffects2.OLIVE" => Some(ColorKind::Olive),
                "touchEffects2.GRAY" => Some(ColorKind::Gray),
                _ => None,
            } {
                assert_eq!(actor.body.color_argb(segment).unwrap(), kind.argb().unwrap());
                world
                    .touch_effects_two(
                        &actors[&arguments[0]],
                        &mut actor,
                        &actors[parameters[0]],
                        &mut peer,
                        segment,
                        peer_segment,
                    )
                    .unwrap();
                return;
            }
            match operation {
                "pretouchEffects" => {
                    world.pre_touch_effects(&mut actor, &mut peer, segment, peer_segment).unwrap()
                }
                "touchfriendEffects" => {
                    world.touch_friend_effects(&mut actor, &mut peer, segment, peer_segment).unwrap()
                }
                "touchfriendReaction" => {
                    world.touch_friend_reaction(&mut actor, &peer, segment, peer_segment).unwrap()
                }
                "touchReaction" => world.touch_reaction(&mut actor, &peer, segment, peer_segment).unwrap(),
                "touchfriendBrake" => {
                    world.touch_friend_brake(&mut actor, &peer, segment, peer_segment).unwrap()
                }
                "touchBrake" => world.touch_brake(&mut actor, &peer, segment, peer_segment).unwrap(),
                "touchHatch" => world.touch_hatch(&mut actor, &peer, peer_segment).unwrap(),
                _ => unreachable!(),
            }
        }
        operation => panic!("Unknown death operation {operation}"),
    }
}

fn apply_input(
    case: &DeathCase,
    target: &str,
    kind: &str,
    value: &str,
    world: &mut World,
    actors: &BTreeMap<String, OrganismRef>,
    genomes: &mut BTreeMap<String, GenomeRef>,
    errors: &mut Vec<String>,
) {
    apply_reference_input(
        &case.name,
        &case.input_genomes,
        target,
        kind,
        value,
        world,
        actors,
        genomes,
        errors,
    );
}

pub(super) fn apply_reference_input(
    case_name: &str,
    input_genomes: &BTreeMap<String, Stage>,
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
        ["world", field] => match (*field, kind) {
            ("_population", "int") => world.population.set(value.parse().unwrap()),
            ("NEXT_ID", "int") => world.next_id.set(value.parse().unwrap()),
            ("nFrames", "int") => world.frame.set(value.parse().unwrap()),
            ("_O2", "double") => world.oxygen.set(decoded(value)),
            ("_CO2", "double") => world.carbon_dioxide.set(decoded(value)),
            ("_CH4", "double") => world.methane.set(decoded(value)),
            _ => panic!("Unknown world input {target} {kind}"),
        },
        ["visible", "selected"] => {
            assert_eq!(kind, "String");
            assert!(actors.contains_key(value));
            // Callback selection belongs to the actual Java collaborator and has no Rust model field
        }
        ["genome", label, "construction"] => {
            assert_eq!(kind, "String");
            let code = if value == "explicit" {
                explicit_genome(&input_genomes[*label])
            } else {
                let source = value
                    .strip_prefix("clone:")
                    .expect("Unknown death genome recipe");
                genomes[source].borrow().clone()
            };
            assert!(
                genomes
                    .insert((*label).to_owned(), Rc::new(RefCell::new(code)))
                    .is_none()
            );
            let expected = BTreeMap::from([(
                (*label).to_owned(),
                Stage {
                    organism: Fields::new(),
                    world: Fields::new(),
                    genome: input_genomes[*label].genome.clone(),
                    genes: input_genomes[*label].genes.clone(),
                },
            )]);
            let actual = BTreeMap::from([((*label).to_owned(), Rc::clone(&genomes[*label]))]);
            compare_genomes(case_name, "input", &expected, &actual, errors);
        }
        ["genome", label, field] => {
            let mut code = genomes[*label].borrow_mut();
            if let Some(gene_field) = field.strip_prefix("gene") {
                let (index, property) = gene_field.split_once("._").expect("Malformed gene input");
                let gene_handle = code.gene_ref(index.parse().unwrap()).unwrap();
                let mut gene = gene_handle.borrow_mut();
                match (property, kind) {
                    ("length", "double") => gene.set_length(decoded(value)),
                    ("color", "Color") => gene.set_color(if value == "~" {
                        None
                    } else {
                        Some(u32::from_str_radix(value, 16).unwrap())
                    }),
                    _ => panic!("Unknown gene input {target} {kind}"),
                }
                return;
            }
            match (*field, kind) {
                ("_cladeID", "String") => code.set_clade_id(if value == "~" {
                    None
                } else {
                    Some(value.to_owned())
                }),
                ("_generation", "int") => code.set_generation(value.parse().unwrap()),
                ("_updateClade", "boolean") => code.set_update_clade(value.parse().unwrap()),
                (
                    "_familial" | "_peaceful" | "_mimicall" | "_modifieslilac" | "_modifiespink",
                    "boolean",
                ) => {
                    let mut modifiers = code.modifiers();
                    match *field {
                        "_familial" => modifiers.familial = value.parse().unwrap(),
                        "_peaceful" => modifiers.peaceful = value.parse().unwrap(),
                        "_mimicall" => modifiers.mimic_all = value.parse().unwrap(),
                        "_modifieslilac" => modifiers.modifies_lilac = value.parse().unwrap(),
                        "_modifiespink" => modifiers.modifies_pink = value.parse().unwrap(),
                        _ => unreachable!(),
                    }
                    let mut properties = code.properties();
                    properties.modifiers = modifiers;
                    code.set_properties(properties);
                }
                ("_modifiesspore" | "_adaptspore" | "_modifiescream", "int") => {
                    let mut modifiers = code.modifiers();
                    if *field == "_modifiesspore" {
                        modifiers.spore = value.parse().unwrap();
                    } else if *field == "_adaptspore" {
                        modifiers.adapt_spore = value.parse().unwrap();
                    } else {
                        modifiers.cream = value.parse().unwrap();
                    }
                    let mut properties = code.properties();
                    properties.modifiers = modifiers;
                    code.set_properties(properties);
                }
                _ => panic!("Unknown death genome input {target} {kind}"),
            }
        }
        ["statistics", field] => set_statistics_input(
            &mut world.statistics_ref().borrow_mut(),
            field,
            kind,
            value,
            genomes,
        ),
        ["actor", role, field] => {
            let mut actor = actors[*role].borrow_mut();
            if let Some(channel) = crate::REACTION_NAMES
                .iter()
                .position(|name| *field == format!("_seg{name}Reaction"))
            {
                assert_eq!(kind, "int[]");
                let values: Vec<i32> = value.split(',').map(|item| item.parse().unwrap()).collect();
                assert_eq!(values.len(), actor.body.forward_count());
                actor.body.arrays.reactions[channel] = Some(crate::IntArrayRef::from_values(values));
                return;
            }
            match (*field, kind) {
                ("_mass", "double") => actor.body.mass = decoded(value),
                ("_I", "double") => actor.body.inertia = decoded(value),
                ("_centerX", "int") => actor.state.integer_center[0] = value.parse().unwrap(),
                ("_centerY", "int") => actor.state.integer_center[1] = value.parse().unwrap(),
                ("_dCenterX", "double") => actor.state.center[0] = decoded(value),
                ("_dCenterY", "double") => actor.state.center[1] = decoded(value),
                ("x1" | "y1" | "x2" | "y2", "int[]") => {
                    let values: Vec<i32> =
                        value.split(',').map(|item| item.parse().unwrap()).collect();
                    assert_eq!(values.len(), actor.body.forward_count());
                    let field = match *field { "x1" => crate::IntField::X1, "y1" => crate::IntField::Y1,
                        "x2" => crate::IntField::X2, "y2" => crate::IntField::Y2, _ => unreachable!() };
                    *actor.body.arrays.int_mut(field) = Some(crate::IntArrayRef::from_values(values));
                }
                ("_growthRatio", "int") => {
                    let ratio: i32 = value.parse().unwrap();
                    assert!((1..=16).contains(&ratio));
                    actor.body.growth_ratio = ratio;
                }
                ("_ID", "int") => actor.id = i64::from(value.parse::<i32>().unwrap()),
                ("_parentID", "int") => actor.state.parent_id = value.parse().unwrap(),
                ("_infectedGeneticCode" | "_savedGeneticCode", "GeneticCode") => {
                    let code = if value == "~" {
                        None
                    } else {
                        Some(Rc::clone(&genomes[value]))
                    };
                    if *field == "_infectedGeneticCode" {
                        actor.infected_genome = code;
                    } else {
                        actor.saved_genome = code;
                    }
                }
                ("_segColor", "Color[]") => {
                    actor.body.phenotype_colors = if value == "~" {
                        None
                    } else {
                        Some(crate::ColorArrayRef::from_slots(value.split(',').map(|item| {
                            if item == "~" { None } else {
                                Some(crate::JavaColor::from_argb(u32::from_str_radix(item, 16).unwrap()))
                            }
                        }).collect()))
                    };
                }
                ("_mphoto", "double[]") => {
                    let coefficients: Vec<_> = value.split(',').map(decoded).collect();
                    assert_eq!(coefficients.len(), actor.body.forward_count());
                    actor.body.arrays.photo = Some(crate::DoubleArrayRef::from_values(coefficients));
                }
                ("_m", "double[]") => {
                    let lengths: Vec<_> = value.split(',').map(decoded).collect();
                    assert_eq!(lengths.len(), actor.body.forward_count());
                    actor.body.arrays.length = Some(crate::DoubleArrayRef::from_values(lengths));
                }
                ("bounds", "Rectangle") => actor.state.bounds.set(input_rectangle(value)),
                ("dx", "double") => actor.state.velocity[0] = decoded(value),
                ("dy", "double") => actor.state.velocity[1] = decoded(value),
                ("dtheta", "double") => actor.state.angular_velocity = decoded(value),
                ("_energy", "double") => actor.state.energy = decoded(value),
                ("_nTotalKills", "int") => actor.state.total_kills = value.parse().unwrap(),
                ("_nTotalChildren", "int") => actor.state.total_children = value.parse().unwrap(),
                ("_nTotalInfected", "int") => actor.state.total_infected = value.parse().unwrap(),
                ("_nChildren", "int") => actor.state.children = value.parse().unwrap(),
                ("_nVirusChildren", "int") => actor.state.virus_children = value.parse().unwrap(),
                ("_max_age", "int") => {
                    actor.state.reproduction.maximum_age = value.parse().unwrap()
                }
                ("_reproduceEnergy", "int") => {
                    actor.state.reproduction.energy = value.parse().unwrap()
                }
                ("_earlyReproduceEnergy", "int") => {
                    actor.state.reproduction.early_energy = value.parse().unwrap()
                }
                ("_infectedReproduceEnergy", "int") => {
                    actor.state.reproduction.infected_energy = value.parse().unwrap()
                }
                ("_timeToReproduce", "int") => {
                    actor.state.reproduction.countdown = value.parse().unwrap()
                }
                ("_timeToReproduceMax", "int") => {
                    actor.state.reproduction.maximum_countdown = value.parse().unwrap()
                }
                ("_nPosition", "String") => {
                    actor.state.network_position = if value == "~" {
                        None
                    } else {
                        Some(crate::JavaText::from_utf8(value))
                    }
                }
                ("_age", "int") => actor.state.age = value.parse().unwrap(),
                ("_framesColor", "int") => {
                    actor.state.temporary_color_frames = value.parse().unwrap()
                }
                ("_color", "Color") => {
                    actor.state.temporary_color = (if value == "~" {
                        None
                    } else {
                        Some(u32::from_str_radix(value, 16).unwrap())
                    }).map(crate::JavaColor::from_argb)}
                ("hasMoved", "boolean") => actor.state.has_moved = value.parse().unwrap(),
                ("active", "boolean") => actor.state.active = value.parse().unwrap(),
                ("alive", "boolean") => actor.state.alive = value.parse().unwrap(),
                ("_altruist", "boolean") => actor.effects.altruist = value.parse().unwrap(),
                ("_photosynthesis", "double") => actor.effects.photosynthesis = decoded(value),
                ("_methanotrophy", "double") => actor.effects.methanotrophy = decoded(value),
                ("_maintenance", "double") => actor.effects.maintenance = decoded(value),
                ("_colonyPhotosynthesis", "double") => {
                    actor.effects.colony_photosynthesis = decoded(value)
                }
                ("_forestphoto", "double") => actor.effects.forest_photo = decoded(value),
                ("_leafphoto", "double") => actor.effects.leaf_photo = decoded(value),
                ("_allfrozen", "boolean") => actor.effects.all_frozen = value.parse().unwrap(),
                ("_antiviral", "int") => actor.effects.antiviral = value.parse().unwrap(),
                ("_candodge", "boolean") => actor.effects.can_dodge = value.parse().unwrap(),
                ("_createlavender", "int") => {
                    actor.effects.create_lavender = value.parse().unwrap()
                }
                ("_crowded", "boolean") => actor.effects.crowded = value.parse().unwrap(),
                ("_dodge", "boolean") => actor.effects.dodge = value.parse().unwrap(),
                ("_fallowversion", "int") => actor.effects.fallow_version = value.parse().unwrap(),
                ("_gold", "int") => actor.effects.gold = value.parse().unwrap(),
                ("_haseyes", "boolean") => actor.body.eyes.enabled = value.parse().unwrap(),
                ("_hasdodged", "boolean") => actor.effects.has_dodged = value.parse().unwrap(),
                ("_healing", "int") => actor.effects.healing = value.parse().unwrap(),
                ("_indigo", "int") => actor.effects.indigo = value.parse().unwrap(),
                ("_isaconsumer", "boolean") => actor.effects.is_consumer = value.parse().unwrap(),
                ("_isafungus", "boolean") => actor.effects.is_fungus = value.parse().unwrap(),
                ("_isakiller", "boolean") => actor.effects.is_killer = value.parse().unwrap(),
                ("_isaplant", "boolean") => actor.effects.is_plant = value.parse().unwrap(),
                ("_isauburn", "boolean") => actor.effects.is_auburn = value.parse().unwrap(),
                ("_isblond", "boolean") => actor.effects.is_blond = value.parse().unwrap(),
                ("_iscoral", "boolean") => actor.effects.is_coral = value.parse().unwrap(),
                ("_isenhanced", "boolean") => actor.effects.is_enhanced = value.parse().unwrap(),
                ("_isfrozen", "boolean") => actor.effects.is_frozen = value.parse().unwrap(),
                ("_isgray", "boolean") => actor.effects.is_gray = value.parse().unwrap(),
                ("_isinfectious", "boolean") => {
                    actor.effects.is_infectious = value.parse().unwrap()
                }
                ("_isinjured", "boolean") => actor.effects.is_injured = value.parse().unwrap(),
                ("_islime", "boolean") => actor.effects.is_lime = value.parse().unwrap(),
                ("_isonlyc4", "boolean") => actor.effects.is_only_c4 = value.parse().unwrap(),
                ("_isreproductive", "boolean") => {
                    actor.effects.is_reproductive = value.parse().unwrap()
                }
                ("_issilver", "boolean") => actor.effects.is_silver = value.parse().unwrap(),
                ("_isspike", "boolean") => actor.effects.is_spike = value.parse().unwrap(),
                ("_generationbattle", "boolean") => {
                    actor.effects.generation_battle = value.parse().unwrap()
                }
                ("_siblingbattle", "boolean") => {
                    actor.effects.sibling_battle = value.parse().unwrap()
                }
                ("hasGrown", "int") => actor.state.has_grown = value.parse().unwrap(),
                ("_jadefactor", "int") => actor.effects.jade_factor = value.parse().unwrap(),
                ("_lavender", "int") => actor.effects.lavender = value.parse().unwrap(),
                ("_lengthfriend", "int") => actor.effects.friend_length = value.parse().unwrap(),
                ("_lowersymmetric", "int") => {
                    actor.effects.lower_symmetric = value.parse().unwrap()
                }
                ("_plagueversion", "int") => actor.effects.plague_version = value.parse().unwrap(),
                ("_reproducelate", "int") => actor.effects.reproduce_late = value.parse().unwrap(),
                ("_segmentpretoucheffects", "int") => {
                    actor.effects.segment_pre_touch_effects = value.parse().unwrap()
                }
                ("_skyversion", "int") => actor.effects.sky_version = value.parse().unwrap(),
                ("_spin", "int") => actor.effects.spin = value.parse().unwrap(),
                ("_sporetime", "int") => actor.effects.spore_time = value.parse().unwrap(),
                ("_sporeversion", "int") => actor.effects.spore_version = value.parse().unwrap(),
                ("_summerinactivity", "int") => {
                    actor.effects.summer_inactivity = value.parse().unwrap()
                }
                ("_thetafriend", "int") => actor.effects.friend_theta = value.parse().unwrap(),
                ("_transfersenergy", "boolean") => {
                    actor.effects.transfers_energy = value.parse().unwrap()
                }
                ("_updateEffects", "int") => actor.body.update_effects = value.parse().unwrap(),
                ("_yellowCounter", "int") => actor.effects.yellow_count = value.parse().unwrap(),
                ("_usepretoucheffects", "boolean") => {
                    actor.effects.use_pre_touch_effects = value.parse().unwrap()
                }
                ("_usefriendeffects", "int") => {
                    actor.effects.use_friend_effects = value.parse().unwrap()
                }
                ("_useframemovement", "boolean") => {
                    actor.effects.use_frame_movement = value.parse().unwrap()
                }
                ("_useextraeffects", "boolean") => {
                    actor.effects.use_extra_effects = value.parse().unwrap()
                }
                ("_canmove", "boolean") => actor.effects.can_move = value.parse().unwrap(),
                ("_canreact", "boolean") => actor.effects.can_react = value.parse().unwrap(),
                _ => panic!("Unknown death actor input {target} {kind}"),
            }
        }
        _ => panic!("Unknown death input {target} {kind}"),
    }
}

fn replay(case: &DeathCase, errors: &mut Vec<String>) {
    let mut world = empty_reference_world(&case.settings, case.seed);
    let mut actors = BTreeMap::new();
    let mut genomes = BTreeMap::new();
    let first_nonempty = case
        .input_genomes
        .values()
        .find(|stage| !stage.genes.is_empty())
        .unwrap();
    let sample = explicit_genome(first_nonempty);
    let gene_schema = gene_fields(&sample.gene_ref(0).unwrap().borrow()).keys().cloned().collect();
    for stage in case.input_genomes.values() {
        validate_genome(stage, &gene_schema);
    }
    for snapshot in case.snapshots.values() {
        for stage in snapshot.genomes.values() {
            validate_genome(stage, &gene_schema);
        }
    }
    for step in &case.steps {
        match step {
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
                    &mut genomes,
                    errors,
                );
            }
            Step::Call {
                stage,
                operation,
                arguments,
            } => {
                call(
                    case, stage, operation, arguments, &mut world, &actors, &genomes,
                );
            }
            Step::Snapshot(name) => compare_snapshot(case, name, &world, &actors, &genomes, errors),
        }
    }
    assert_eq!(actors.len(), 3);
    assert_eq!(world.organisms.borrow().len(), 3);
    assert_eq!(
        genomes.keys().collect::<BTreeSet<_>>(),
        case.input_genomes.keys().collect()
    );
}

#[test]
fn real_java_death_boundary_state_statistics_and_rng_match() {
    let expected = Coverage {
        cases: 36,
        snapshots: 84,
        calls: [
            ("die", 23),
            ("isInsideWorld", 22),
            ("grow", 1),
            ("offset_velocity", 1),
            ("bounds", 1),
        ]
        .map(|(operation, count)| (operation.to_owned(), count))
        .into_iter()
        .collect(),
        compared_rows: 141_370,
        collaborator_rows: 504,
        selection_metadata_rows: 36,
        input_genotype_rows: 43_320,
        input_and_command_rows: 5_959,
    };
    compare_reference(
        "death",
        include_str!("../tests/fixtures/death.tsv"),
        expected,
    );
}

pub(super) fn compare_reference(label: &str, source: &str, expected: Coverage) {
    let expected_rows = expected.compared_rows
        + expected.collaborator_rows
        + expected.selection_metadata_rows
        + expected.input_genotype_rows
        + expected.input_and_command_rows;
    assert_eq!(source.lines().count(), expected_rows);
    let mut coverage = Coverage::default();
    let cases = parse(source, &mut coverage);
    // Fix the observation scope before executing any production comparison
    assert_eq!(coverage, expected);
    assert_eq!(
        coverage.compared_rows
            + coverage.collaborator_rows
            + coverage.selection_metadata_rows
            + coverage.input_genotype_rows
            + coverage.input_and_command_rows,
        source.lines().count()
    );
    let mut errors = Vec::new();
    for case in cases {
        replay(&case, &mut errors);
    }
    if !errors.is_empty() {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!(
            "{label}-oracle-mismatches-{}.tsv",
            std::process::id()
        ));
        let mut report = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        writeln!(report, "{}", errors.join("\n")).unwrap();
        panic!(
            "{} {label} mismatches; report={}\n{}",
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
        "{}_COMPARISON {coverage:?}; supplied genotype checks and source collaborator exclusions separately counted; no full-frame claim",
        label.to_ascii_uppercase()
    );
}
