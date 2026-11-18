// SPDX-License-Identifier: GPL-2.0-or-later

//! Compare original movement observations through actual Rust primitives and explicit typed fixture inputs

use super::*;
use crate::organism::{GenomeRef, OrganismRef};
use std::{collections::BTreeSet, rc::Rc};

#[derive(Default)]
struct MovementStage {
    organism: Fields,
    genotypes: BTreeMap<String, Stage>,
    suffix: Option<(i64, i32, u64)>,
}

struct Input {
    stage: String,
    target: String,
    kind: String,
    value: String,
}

struct Action {
    stage: String,
    operation: String,
    arguments: Vec<String>,
}

struct MovementCase {
    name: String,
    setup: String,
    seed: i64,
    settings: Fields,
    stages: BTreeMap<String, MovementStage>,
    inputs: Vec<Input>,
    actions: Vec<Action>,
    facts: Fields,
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Coverage {
    cases: usize,
    snapshots: usize,
    calls: usize,
    operations: BTreeMap<String, usize>,
    compared_rows: usize,
    collaborator_rows: usize,
    input_genotype_rows: usize,
    input_and_command_rows: usize,
}

#[derive(Default)]
struct References(BTreeMap<String, GenomeRef>);

impl References {
    fn observe(
        &mut self,
        expected: &str,
        actual: Option<&GenomeRef>,
        context: &str,
        errors: &mut Vec<String>,
    ) {
        match (expected, actual) {
            ("~", None) => {}
            (expected, Some(actual)) if expected != "~" => {
                if let Some(previous) = self.0.get(expected) {
                    if !Rc::ptr_eq(previous, actual) {
                        errors.push(format!("{context}\tgenotype {expected} changed identity"));
                    }
                } else {
                    if let Some((name, _)) = self
                        .0
                        .iter()
                        .find(|(_, previous)| Rc::ptr_eq(previous, actual))
                    {
                        errors.push(format!(
                            "{context}\tgenotypes {name}/{expected} incorrectly alias"
                        ));
                    }
                    self.0.insert(expected.into(), Rc::clone(actual));
                }
            }
            _ => errors.push(format!(
                "{context}\tgenotype presence differs from {expected}"
            )),
        }
    }
}

fn validate(kind: &str, value: &str) {
    if let Some(scalar) = kind.strip_suffix("[]") {
        if value != "~" && !value.is_empty() {
            for item in value.split(',') {
                validate(scalar, item);
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
            rectangle_input(value);
        }
        "World" | "VisibleWorld" | "GeneticCode" => assert!(["present", "~"].contains(&value)),
        kind => panic!("Unknown movement field type {kind}"),
    }
}

fn rectangle_input(value: &str) -> IntRect {
    let parts: Vec<i32> = value.split(',').map(|part| part.parse().unwrap()).collect();
    assert_eq!(parts.len(), 4);
    IntRect {
        x: parts[0],
        y: parts[1],
        width: parts[2],
        height: parts[3],
    }
}

fn validate_case(
    case: &MovementCase,
    setting_types: &BTreeMap<String, String>,
    organism_types: &BTreeMap<String, String>,
) {
    assert_eq!(
        case.settings.keys().collect::<BTreeSet<_>>(),
        setting_types.keys().collect()
    );
    let input = &case.stages["input"];
    assert!(input.organism.is_empty() && input.suffix.is_none());
    assert_eq!(
        input
            .genotypes
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["genotype"]
    );
    let source_genome = explicit_genome(&input.genotypes["genotype"]);
    let schema = genome_fields(&source_genome);
    let genome_keys: BTreeSet<_> = schema
        .keys()
        .cloned()
        .chain(["_reference".into(), "_nGenes".into()])
        .collect();
    let gene_keys: BTreeSet<_> = gene_fields(&source_genome.gene_ref(0).unwrap().borrow())
        .keys()
        .cloned()
        .collect();
    assert_eq!(genome_keys.len(), 28);
    assert_eq!(gene_keys.len(), 40);
    for (name, stage) in &case.stages {
        if name != "input" {
            assert_eq!(
                stage.organism.keys().collect::<BTreeSet<_>>(),
                organism_types.keys().collect()
            );
            assert_eq!(
                stage
                    .genotypes
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                ["genotype", "infected", "saved"]
            );
            assert!(stage.suffix.is_some());
        }
        for genotype in stage.genotypes.values() {
            let reference = &genotype.genome["_reference"];
            if reference == "~" {
                assert_eq!(genotype.genome.len(), 1);
                assert!(genotype.genes.is_empty());
                continue;
            }
            assert!(reference.starts_with('g'));
            reference[1..].parse::<usize>().unwrap();
            assert_eq!(
                genotype.genome.keys().cloned().collect::<BTreeSet<_>>(),
                genome_keys
            );
            assert_eq!(
                genotype.genes.len(),
                integer(&genotype.genome, "_nGenes") as usize
            );
            for (key, value) in &genotype.genome {
                match key.as_str() {
                    "_reference" | "_cladeID" => {}
                    "_nGenes" => validate("int", value),
                    _ => validate(
                        if ["true", "false"].contains(&schema[key].as_str()) {
                            "boolean"
                        } else {
                            "int"
                        },
                        value,
                    ),
                }
            }
            for (position, (index, gene)) in genotype.genes.iter().enumerate() {
                assert_eq!(position, *index);
                assert_eq!(gene.keys().cloned().collect::<BTreeSet<_>>(), gene_keys);
                for (key, value) in gene {
                    validate(
                        match key.as_str() {
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
    let expected_stages: BTreeSet<_> = ["input".to_owned(), "initial".to_owned()]
        .into_iter()
        .chain(case.actions.iter().map(|action| action.stage.clone()))
        .collect();
    assert_eq!(
        case.stages.keys().cloned().collect::<BTreeSet<_>>(),
        expected_stages
    );
    for input in &case.inputs {
        assert!(
            input.stage == "parent-input" && case.setup == "spore_birth"
                || case
                    .actions
                    .iter()
                    .any(|action| action.stage == input.stage)
        );
    }
    if case.setup == "spore_birth" {
        assert_eq!(
            case.facts.keys().map(String::as_str).collect::<Vec<_>>(),
            ["birth_seed_reset", "parent_tree_bounds"]
        );
    } else {
        assert!(case.facts.is_empty());
    }
}

fn parse(text: &str, coverage: &mut Coverage) -> Vec<MovementCase> {
    let (setting_types, organism_types) = primitive_reference_schemas();
    let mut current = None;
    let mut cases = Vec::new();
    let mut names = BTreeSet::new();
    let mut input_keys = BTreeSet::new();
    for (index, line) in text.lines().enumerate() {
        let row: Vec<_> = line.split('\t').collect();
        assert!(!line.is_empty(), "Empty movement record {}", index + 1);
        if row[0] == "C" {
            assert_eq!(row.len(), 4);
            assert!(current.is_none());
            assert!(names.insert(row[1].to_owned()));
            assert!(["pasted", "spore_birth"].contains(&row[2]));
            current = Some(MovementCase {
                name: row[1].into(),
                setup: row[2].into(),
                seed: row[3].parse().unwrap(),
                settings: Fields::new(),
                stages: BTreeMap::new(),
                inputs: Vec::new(),
                actions: Vec::new(),
                facts: Fields::new(),
            });
            input_keys.clear();
            coverage.input_and_command_rows += 1;
            continue;
        }
        let case = current.as_mut().expect("Movement row outside a case");
        match row[0] {
            "U" => {
                assert_eq!(row.len(), 4);
                assert_eq!(setting_types.get(row[1]).map(String::as_str), Some(row[2]));
                validate(row[2], row[3]);
                assert!(case.settings.insert(row[1].into(), row[3].into()).is_none());
                coverage.input_and_command_rows += 1;
            }
            "G" | "N" => {
                assert_eq!(row.len(), if row[0] == "G" { 5 } else { 6 });
                assert!(["genotype", "infected", "saved"].contains(&row[2]));
                let genotype = case
                    .stages
                    .entry(row[1].into())
                    .or_default()
                    .genotypes
                    .entry(row[2].into())
                    .or_default();
                if row[0] == "G" {
                    assert!(
                        genotype
                            .genome
                            .insert(row[3].into(), row[4].into())
                            .is_none()
                    );
                } else {
                    assert!(
                        genotype
                            .genes
                            .entry(row[3].parse().unwrap())
                            .or_default()
                            .insert(row[4].into(), row[5].into())
                            .is_none()
                    );
                }
                if row[1] == "input" {
                    coverage.input_genotype_rows += 1;
                    coverage.input_and_command_rows += 1;
                }
            }
            "O" => {
                assert_eq!(row.len(), 5);
                assert_eq!(organism_types.get(row[2]).map(String::as_str), Some(row[3]));
                validate(row[3], row[4]);
                assert!(
                    case.stages
                        .entry(row[1].into())
                        .or_default()
                        .organism
                        .insert(row[2].into(), row[4].into())
                        .is_none()
                );
            }
            "R" => {
                assert_eq!(row.len(), 5);
                assert!(
                    case.stages
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
            }
            "I" => {
                assert_eq!(row.len(), 5);
                if row[3] == "GeneticCode" {
                    assert!(["clone", "shared"].contains(&row[4]));
                } else {
                    validate(row[3], row[4]);
                }
                assert!(input_keys.insert((row[1].to_owned(), row[2].to_owned())));
                case.inputs.push(Input {
                    stage: row[1].into(),
                    target: row[2].into(),
                    kind: row[3].into(),
                    value: row[4].into(),
                });
                coverage.input_and_command_rows += 1;
            }
            "F" => {
                assert_eq!(row.len(), 3);
                match row[1] {
                    "parent_tree_bounds" => {
                        rectangle_input(row[2]);
                    }
                    "birth_seed_reset" => {
                        validate("long", row[2]);
                        coverage.input_and_command_rows += 1;
                    }
                    key => panic!("Unknown movement setup fact {key}"),
                }
                assert!(case.facts.insert(row[1].into(), row[2].into()).is_none());
            }
            "K" => {
                assert_eq!(row[1], format!("step-{}", case.actions.len()));
                match row[2] {
                    "offset" => {
                        assert_eq!(row.len(), 6);
                        for value in &row[3..] {
                            validate("double", value);
                        }
                    }
                    "bounds" => {
                        assert_eq!(row.len(), 4);
                        validate("boolean", row[3]);
                    }
                    "symmetric" | "grow" | "rubbing" | "offset_velocity" => {
                        assert_eq!(row.len(), 3)
                    }
                    operation => panic!("Unknown movement operation {operation}"),
                }
                case.actions.push(Action {
                    stage: row[1].into(),
                    operation: row[2].into(),
                    arguments: row[3..].iter().map(|value| (*value).to_owned()).collect(),
                });
                coverage.input_and_command_rows += 1;
            }
            "E" => {
                assert_eq!(row.len(), 1);
                validate_case(case, &setting_types, &organism_types);
                cases.push(current.take().unwrap());
                coverage.input_and_command_rows += 1;
            }
            kind => panic!("Unknown movement row {kind} at {}", index + 1),
        }
    }
    assert!(current.is_none());
    cases
}

fn apply_input(organism: &mut Organism, input: &Input) {
    let value = &input.value;
    match (input.target.as_str(), input.kind.as_str()) {
        ("gene0._length", "double") => organism
            .genome.as_ref().unwrap().borrow()
            .gene_ref(0).unwrap().borrow_mut()
            .set_length(decoded(value)),
        ("_infectedGeneticCode" | "_savedGeneticCode", "GeneticCode") => {
            let reference = match value.as_str() {
                "shared" => Rc::clone(organism.genome.as_ref().unwrap()),
                "clone" => Rc::new(RefCell::new(organism.genome.as_ref().unwrap().borrow().java_clone().unwrap())),
                mode => panic!("Unknown genotype control {mode}"),
            };
            if input.target == "_infectedGeneticCode" {
                organism.infected_genome = Some(reference);
            } else {
                organism.saved_genome = Some(reference);
            }
        }
        ("_growthRatio", "int") => {
            organism.body.growth_ratio = value.parse::<i32>().unwrap()
        }
        ("_age", "int") => organism.state.age = value.parse().unwrap(),
        ("_energy", "double") => organism.state.energy = decoded(value),
        ("_theta", "double") => organism.state.angle = decoded(value),
        ("_lastTheta", "double") => organism.state.last_angle = decoded(value),
        ("_dCenterX", "double") => organism.state.center[0] = decoded(value),
        ("_dCenterY", "double") => organism.state.center[1] = decoded(value),
        ("_centerX", "int") => organism.state.integer_center[0] = value.parse().unwrap(),
        ("_centerY", "int") => organism.state.integer_center[1] = value.parse().unwrap(),
        ("_framesColor", "int") => organism.state.temporary_color_frames = value.parse().unwrap(),
        ("dx", "double") => organism.state.velocity[0] = decoded(value),
        ("dy", "double") => organism.state.velocity[1] = decoded(value),
        ("dtheta", "double") => organism.state.angular_velocity = decoded(value),
        ("hasGrown", "int") => organism.state.has_grown = value.parse().unwrap(),
        ("_reproduceEnergy", "int") => organism.state.reproduction.energy = value.parse().unwrap(),
        ("_earlyReproduceEnergy", "int") => {
            organism.state.reproduction.early_energy = value.parse().unwrap()
        }
        ("_nChildren", "int") => organism.state.children = value.parse().unwrap(),
        ("_nVirusChildren", "int") => organism.state.virus_children = value.parse().unwrap(),
        ("_sporetime", "int") => organism.effects.spore_time = value.parse().unwrap(),
        ("_sporeversion", "int") => organism.effects.spore_version = value.parse().unwrap(),
        ("_isinfectious", "boolean") => organism.effects.is_infectious = value.parse().unwrap(),
        ("_isblond", "boolean") => organism.effects.is_blond = value.parse().unwrap(),
        ("_isfrozen", "boolean") => organism.effects.is_frozen = value.parse().unwrap(),
        ("_isinjured", "boolean") => organism.effects.is_injured = value.parse().unwrap(),
        ("_allfrozen", "boolean") => organism.effects.all_frozen = value.parse().unwrap(),
        ("active", "boolean") => organism.state.active = value.parse().unwrap(),
        ("x1" | "y1" | "x2" | "y2", "int[]") => {
            let values: Vec<i32> = value
                .split(',')
                .map(|value| value.parse().unwrap())
                .collect();
            assert_eq!(values.len(), organism.body.forward_count());
            let field = match input.target.as_str() { "x1" => crate::IntField::X1, "y1" => crate::IntField::Y1,
                "x2" => crate::IntField::X2, "y2" => crate::IntField::Y2, _ => unreachable!() };
            *organism.body.arrays.int_mut(field) = Some(crate::IntArrayRef::from_values(values));
        }
        _ => panic!("Unsupported movement input {}:{}", input.target, input.kind),
    }
}

fn compare_genotype(
    case: &str,
    stage: &str,
    owner: &str,
    actual: Option<&GenomeRef>,
    expected: &Stage,
    references: &mut References,
    coverage: &mut Coverage,
    errors: &mut Vec<String>,
) {
    references.observe(
        &expected.genome["_reference"],
        actual,
        &format!("{case}\t{stage}\t{owner}"),
        errors,
    );
    coverage.compared_rows +=
        expected.genome.len() + expected.genes.values().map(Fields::len).sum::<usize>();
    let Some(actual) = actual else {
        return;
    };
    let actual = actual.borrow();
    let mut fields = genome_fields(&actual);
    fields.insert("_nGenes".into(), actual.genes().unwrap().len().to_string());
    let mut target = expected.genome.clone();
    target.remove("_reference");
    compare_fields(case, stage, &format!("G.{owner}"), &fields, &target, errors);
    for (index, gene) in &expected.genes {
        if *index < actual.genes().unwrap().len() {
            compare_fields(
                case,
                stage,
                &format!("N.{owner}.{index}"),
                &gene_fields(&actual.gene_ref(*index).unwrap().borrow()),
                gene,
                errors,
            );
        } else {
            errors.push(format!("{case}\t{stage}\t{owner}\tmissing gene {index}"));
        }
    }
}

fn snapshot(
    case: &MovementCase,
    name: &str,
    world: &World,
    focus: &OrganismRef,
    references: &mut References,
    coverage: &mut Coverage,
    errors: &mut Vec<String>,
) {
    let expected = &case.stages[name];
    let focus = focus.borrow();
    let mut actual = organism_fields(&View::from(&*focus));
    let mut target = expected.organism.clone();
    // Rust stores neither collaborator reference; validate the fixture environment without claiming pointer comparison
    for (key, environment) in [("_world", "present"), ("_visibleWorld", "~")] {
        assert_eq!(target.remove(key).as_deref(), Some(environment));
        assert!(actual.remove(key).is_some());
        coverage.collaborator_rows += 1;
    }
    compare_fields(&case.name, name, "O", &actual, &target, errors);
    coverage.compared_rows += target.len();
    for (owner, expected) in &expected.genotypes {
        let actual = match owner.as_str() {
            "genotype" => focus.genome.as_ref(),
            "infected" => focus.infected_genome.as_ref(),
            "saved" => focus.saved_genome.as_ref(),
            owner => panic!("Unknown movement genotype {owner}"),
        };
        compare_genotype(
            &case.name, name, owner, actual, expected, references, coverage, errors,
        );
    }
    let mut random = world.random_snapshot();
    let actual = (
        random.next_i64(),
        random.next_bounded(10000).unwrap(),
        random.next_f64().to_bits(),
    );
    if expected.suffix != Some(actual) {
        errors.push(format!(
            "{}\t{name}\tR\tactual={actual:?}\texpected={:?}",
            case.name, expected.suffix
        ));
    }
    coverage.compared_rows += 1;
    coverage.snapshots += 1;
}

fn run(case: MovementCase, coverage: &mut Coverage, errors: &mut Vec<String>) {
    let mut parameters = parameters(&case.settings);
    parameters.spore_energy_consumption = real(&case.settings, "SPORE_ENERGY_CONSUMPTION");
    parameters.atmosphere = crate::AtmosphereParams {
        initial_oxygen: real(&case.settings, "INITIAL_O2"),
        initial_carbon_dioxide: real(&case.settings, "INITIAL_CO2"),
        initial_methane: real(&case.settings, "INITIAL_CH4"),
        carbon_to_methane_divisor: integer(&case.settings, "CO2_TO_CH4_DIVISOR"),
        methane_to_carbon_divisor: integer(&case.settings, "CH4_TO_CO2_DIVISOR"),
        drain_divisor: integer(&case.settings, "DRAIN_SUBS_DIVISOR"),
    };
    let world = World::with_parameters(
        WorldConfig {
            seed: case.seed,
            width: integer(&case.settings, "WORLD_WIDTH"),
            height: integer(&case.settings, "WORLD_HEIGHT"),
            initial_attempts: 0,
        },
        parameters,
    )
    .unwrap();
    let input = &case.stages["input"].genotypes["genotype"];
    let parent_id = world
        .paste_genome(explicit_genome(input), [3500, 3500])
        .unwrap()
        .unwrap();
    let parent = Rc::clone(world.organisms.borrow().last().unwrap());
    let mut references = References::default();
    // The source labels this genotype as an input before observing the focus organism
    assert_eq!(input.genome["_reference"], "g0");
    references
        .0
        .insert("g0".into(), Rc::clone(parent.borrow().genome.as_ref().unwrap()));
    let focus = if case.setup == "spore_birth" {
        {
            let mut parent = parent.borrow_mut();
            for input in case
                .inputs
                .iter()
                .filter(|input| input.stage == "parent-input")
            {
                apply_input(&mut parent, input);
            }
        }
        let bounds = rectangle(parent.borrow().state.bounds.get());
        if bounds != case.facts["parent_tree_bounds"] {
            errors.push(format!(
                "{}\tF\tparent_tree_bounds\tactual={bounds}\texpected={}",
                case.name, case.facts["parent_tree_bounds"]
            ));
        }
        coverage.compared_rows += 1;
        world.rebuild_spatial();
        assert_eq!(
            case.facts["birth_seed_reset"].parse::<i64>().unwrap(),
            case.seed
        );
        *world.session.0.random.borrow_mut() = JavaRandom::new(case.seed);
        world.spawn_child(parent_id, true).unwrap().unwrap();
        Rc::clone(world.organisms.borrow().last().unwrap())
    } else {
        parent
    };
    snapshot(
        &case,
        "initial",
        &world,
        &focus,
        &mut references,
        coverage,
        errors,
    );
    for action in &case.actions {
        {
            let mut focus = focus.borrow_mut();
            for input in case
                .inputs
                .iter()
                .filter(|input| input.stage == action.stage)
            {
                apply_input(&mut focus, input);
            }
            match action.operation.as_str() {
                "symmetric" => focus.rebuild_body().unwrap(),
                "bounds" => crate::movement::calculate_bounds(
                    &mut focus,
                    action.arguments[0].parse().unwrap(),
                )
                .unwrap(),
                "grow" => crate::movement::grow(&mut focus).unwrap(),
                "rubbing" => crate::movement::rubbing(&mut focus, real(&case.settings, "RUBBING")),
                "offset" => crate::movement::offset(
                    &mut focus,
                    [decoded(&action.arguments[0]), decoded(&action.arguments[1])],
                    decoded(&action.arguments[2]),
                ),
                "offset_velocity" => {
                    let velocity = focus.state.velocity;
                    let rotation = focus.state.angular_velocity;
                    crate::movement::offset(&mut focus, velocity, rotation);
                }
                operation => panic!("Unknown movement action {operation}"),
            }
        }
        coverage.calls += 1;
        *coverage
            .operations
            .entry(action.operation.clone())
            .or_default() += 1;
        snapshot(
            &case,
            &action.stage,
            &world,
            &focus,
            &mut references,
            coverage,
            errors,
        );
    }
    coverage.cases += 1;
}

#[test]
fn actual_movement_primitives_match_recorded_java_state_and_randomness() {
    let text = include_str!("../tests/fixtures/movement.tsv");
    assert_eq!(text.lines().count(), 157_137);
    let mut coverage = Coverage::default();
    let cases = parse(text, &mut coverage);
    let mut errors = Vec::new();
    for case in cases {
        run(case, &mut coverage, &mut errors);
    }
    let expected = Coverage {
        cases: 97,
        snapshots: 423,
        calls: 326,
        operations: [
            ("symmetric", 68),
            ("bounds", 166),
            ("grow", 68),
            ("offset", 8),
            ("rubbing", 14),
            ("offset_velocity", 2),
        ]
        .map(|(name, count)| (name.to_owned(), count))
        .into_iter()
        .collect(),
        compared_rows: 126_722,
        collaborator_rows: 846,
        input_genotype_rows: 14_036,
        input_and_command_rows: 29_569,
    };
    if coverage != expected {
        errors.push(format!(
            "Movement comparison scope changed: actual={coverage:?}; expected={expected:?}"
        ));
    }
    if !errors.is_empty() {
        let report = std::env::temp_dir().join("movement-oracle-mismatches.tsv");
        std::fs::write(&report, errors.join("\n") + "\n").unwrap();
        panic!(
            "{} movement mismatches; report={}\n{}",
            errors.len(),
            report.display(),
            errors
                .iter()
                .take(40)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    println!(
        "MOVEMENT_COMPARISON {coverage:?}; constructor inputs and collaborator fields explicitly disposed; no full-frame claim"
    );
}
