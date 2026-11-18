// SPDX-License-Identifier: GPL-2.0-or-later

//! Compare real child construction with pinned Java observations and explicit inaccessible-state exclusions

use super::*;
use crate::organism::GenomeRef;
use std::{collections::BTreeSet, rc::Rc};

const ORGANISM_KEYS: &str = "_ID _parentID _energy _reproduceEnergy _earlyReproduceEnergy _infectedReproduceEnergy _growthRatio _segments _age _max_age _nChildren _nVirusChildren _nTotalChildren _nTotalInfected _nPosition _timeToReproduce _timeToReproduceMax _reproducelate _sporetime _sporeversion _indigo _jadefactor _isaconsumer _isafungus _isaplant _isreproductive _isinfectious _isblond _isauburn _haseyes _hasgoodvision _canmove _canreact _candodge _usepretoucheffects _usefriendeffects _isenhanced _photosynthesis _methanotrophy _mphoto _segColor _color _framesColor active alive hasMoved dx dy dtheta _centerX _centerY _dCenterX _dCenterY _theta _mass _I _sizeRect bounds";
const GENOME_KEYS: &str = "_reference _nGenes _symmetry _mirror _mutationrate _activity _modifiescream _modifiesfallow _modifiesspore _adaptspore _plague _disperseChildren _generationBattle _siblingBattle _altruist _familial _social _peaceful _passive _clockwise _mimicall _modifiespink _modifieslilac _modifiessky _selfish _generation _cladeID _updateClade";
const ALIAS_KEYS: &str = "child_genotype_is_parent child_genotype_is_infected child_genotype_is_saved child_genotype_is_failed saved_is_failed infected_is_original child_genotype_is_initial_saved";

#[derive(Default)]
struct BirthStage {
    organisms: BTreeMap<String, Fields>,
    genotypes: BTreeMap<String, Stage>,
    assertions: Fields,
    world: Fields,
    suffix: Option<(i64, i32, u64)>,
}

struct Input {
    kind: String,
    value: String,
}

#[derive(Default)]
struct Tree {
    count: Option<usize>,
    members: Vec<(String, IntRect, [u64; 4])>,
}

struct BirthCase {
    name: String,
    operation: String,
    seed: i64,
    settings: Fields,
    inputs: BTreeMap<(String, String), Input>,
    stages: BTreeMap<String, BirthStage>,
    trees: BTreeMap<String, Tree>,
    list_operations: BTreeMap<(String, String), String>,
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Coverage {
    cases: usize,
    attempts: usize,
    successes: usize,
    failures: usize,
    snapshots: usize,
    compared_rows: usize,
    failed_organism_rows: usize,
    failed_genome_rows: usize,
    failed_gene_rows: usize,
    failed_alias_rows: usize,
    population_rows: usize,
    metadata_rows: usize,
}

#[derive(Default)]
struct References {
    named: BTreeMap<String, GenomeRef>,
}

impl References {
    fn compare(
        &mut self,
        actual: Option<&GenomeRef>,
        expected: &str,
        context: &str,
        errors: &mut Vec<String>,
    ) {
        match (actual, expected) {
            (None, "~") => {}
            (Some(actual), expected) if expected != "~" => {
                if let Some(previous) = self.named.get(expected) {
                    if !Rc::ptr_eq(actual, previous) {
                        errors.push(format!(
                            "{context}\tgenotype reference {expected} changed identity"
                        ));
                    }
                } else {
                    if let Some((name, _)) = self
                        .named
                        .iter()
                        .find(|(_, previous)| Rc::ptr_eq(actual, previous))
                    {
                        errors.push(format!(
                            "{context}\tgenotype references {name}/{expected} incorrectly alias"
                        ));
                    }
                    self.named.insert(expected.into(), Rc::clone(actual));
                }
            }
            _ => errors.push(format!(
                "{context}\tgenotype presence differs from {expected}"
            )),
        }
    }
}

fn keys(text: &str) -> BTreeSet<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

fn rect(value: &str) -> IntRect {
    let values: Vec<i32> = value
        .split(',')
        .map(|value| value.parse().unwrap())
        .collect();
    assert_eq!(values.len(), 4);
    IntRect {
        x: values[0],
        y: values[1],
        width: values[2],
        height: values[3],
    }
}

fn validate_primitive(kind: &str, value: &str) {
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
        other => panic!("Unknown primitive type {other}"),
    }
}

fn validate_value(kind: &str, value: &str) {
    if kind.ends_with("[]") {
        if value != "~" && !value.is_empty() {
            for item in value.split(',') {
                validate_value(&kind[..kind.len() - 2], item);
            }
        }
    } else {
        match kind {
            "String" => {}
            "Color" => {
                if value != "~" {
                    assert_eq!(value.len(), 8);
                    u32::from_str_radix(value, 16).unwrap();
                }
            }
            "Rectangle" => {
                rect(value);
            }
            "GeneticCode" | "World" | "VisibleWorld" => assert!(["~", "present"].contains(&value)),
            other => validate_primitive(other, value),
        }
    }
}

fn schemas(full: bool) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let (settings, mut organism) = primitive_reference_schemas();
    if !full {
        let declared = keys(ORGANISM_KEYS);
        organism.retain(|key, _| declared.contains(key));
        assert_eq!(organism.len(), 58);
    }
    (settings, organism)
}

fn parse(text: &str, full: bool, coverage: &mut Coverage) -> Vec<BirthCase> {
    let (setting_types, organism_types) = schemas(full);
    assert_eq!(setting_types.len(), 147);
    let mut cases = Vec::new();
    let mut current = None;
    let mut names = BTreeSet::new();
    let organism_keys: BTreeSet<_> = organism_types.keys().cloned().collect();
    for (index, line) in text.lines().enumerate() {
        let row: Vec<_> = line.split('\t').collect();
        assert!(!line.is_empty(), "Empty birth record at {}", index + 1);
        if row[0] == "C" {
            assert_eq!(row.len(), 4);
            assert!(current.is_none());
            assert!(names.insert(row[1].to_owned()));
            assert!(
                [
                    "single",
                    "retry",
                    "blocked-direction",
                    "litter",
                    "spore-wake"
                ]
                .contains(&row[2])
            );
            current = Some(BirthCase {
                name: row[1].into(),
                operation: row[2].into(),
                seed: row[3].parse().unwrap(),
                settings: Fields::new(),
                inputs: BTreeMap::new(),
                stages: BTreeMap::new(),
                trees: BTreeMap::new(),
                list_operations: BTreeMap::new(),
            });
            coverage.metadata_rows += 1;
            continue;
        }
        let case = current.as_mut().expect("Birth record outside a case");
        match row[0] {
            "U" => {
                assert_eq!(row.len(), 4);
                assert_eq!(setting_types.get(row[1]).map(String::as_str), Some(row[2]));
                validate_primitive(row[2], row[3]);
                assert!(case.settings.insert(row[1].into(), row[3].into()).is_none());
                coverage.metadata_rows += 1;
            }
            "I" => {
                assert_eq!(row.len(), 5);
                match row[3] {
                    "Rectangle" => {
                        rect(row[4]);
                    }
                    "GeneticCode" => assert!(
                        ["explicit-virus-genome", "explicit-saved-genome"].contains(&row[4])
                    ),
                    other => validate_primitive(other, row[4]),
                }
                assert!(
                    case.inputs
                        .insert(
                            (row[1].into(), row[2].into()),
                            Input {
                                kind: row[3].into(),
                                value: row[4].into()
                            }
                        )
                        .is_none()
                );
                coverage.metadata_rows += 1;
            }
            "O" => {
                assert_eq!(row.len(), 6);
                assert!(["parent", "child"].contains(&row[2]));
                assert!(organism_keys.contains(row[3]));
                assert_eq!(organism_types.get(row[3]).map(String::as_str), Some(row[4]));
                validate_value(row[4], row[5]);
                assert!(
                    case.stages
                        .entry(row[1].into())
                        .or_default()
                        .organisms
                        .entry(row[2].into())
                        .or_default()
                        .insert(row[3].into(), row[5].into())
                        .is_none()
                );
            }
            "G" => {
                assert_eq!(row.len(), 5);
                assert!(
                    [
                        "parent.genotype",
                        "parent.infected",
                        "parent.saved",
                        "child.genotype",
                        "failed.genotype"
                    ]
                    .contains(&row[2])
                );
                assert!(keys(GENOME_KEYS).contains(row[3]));
                assert!(
                    case.stages
                        .entry(row[1].into())
                        .or_default()
                        .genotypes
                        .entry(row[2].into())
                        .or_default()
                        .genome
                        .insert(row[3].into(), row[4].into())
                        .is_none()
                );
            }
            "N" => {
                assert_eq!(row.len(), 6);
                assert!(
                    [
                        "parent.genotype",
                        "parent.infected",
                        "parent.saved",
                        "child.genotype",
                        "failed.genotype"
                    ]
                    .contains(&row[2])
                );
                assert!(
                    case.stages
                        .entry(row[1].into())
                        .or_default()
                        .genotypes
                        .entry(row[2].into())
                        .or_default()
                        .genes
                        .entry(row[3].parse().unwrap())
                        .or_default()
                        .insert(row[4].into(), row[5].into())
                        .is_none()
                );
            }
            "A" => {
                assert_eq!(row.len(), 4);
                if row[2].starts_with("direction_") {
                    row[3].parse::<i32>().unwrap();
                } else {
                    row[3].parse::<bool>().unwrap();
                }
                assert!(
                    case.stages
                        .entry(row[1].into())
                        .or_default()
                        .assertions
                        .insert(row[2].into(), row[3].into())
                        .is_none()
                );
            }
            "W" => {
                assert_eq!(row.len(), 4);
                assert!(
                    [
                        "CO2",
                        "O2",
                        "CH4",
                        "NEXT_ID",
                        "stored_organisms",
                        "population"
                    ]
                    .contains(&row[2])
                );
                assert!(
                    case.stages
                        .entry(row[1].into())
                        .or_default()
                        .world
                        .insert(row[2].into(), row[3].into())
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
            "T" => {
                let tree = case.trees.entry(row[1].into()).or_default();
                if row[2] == "rebuild" {
                    assert_eq!(row.len(), 4);
                    assert!(tree.count.replace(row[3].parse().unwrap()).is_none());
                } else {
                    assert_eq!(row.len(), 9);
                    assert_eq!(row[2].parse::<usize>().unwrap(), tree.members.len());
                    tree.members.push((
                        row[3].into(),
                        rect(row[4]),
                        std::array::from_fn(|i| u64::from_str_radix(row[5 + i], 16).unwrap()),
                    ));
                }
            }
            "L" => {
                assert_eq!(row.len(), 4);
                assert!(["append_without_tree_rebuild", "remove_blocker"].contains(&row[2]));
                assert!(
                    case.list_operations
                        .insert((row[1].into(), row[2].into()), row[3].into())
                        .is_none()
                );
                coverage.metadata_rows += 1;
            }
            "E" => {
                assert_eq!(row.len(), 1);
                assert_eq!(
                    case.settings.keys().cloned().collect::<BTreeSet<_>>(),
                    setting_types.keys().cloned().collect()
                );
                for stage in case.stages.values() {
                    for organism in stage.organisms.values() {
                        assert_eq!(
                            organism.keys().cloned().collect::<BTreeSet<_>>(),
                            organism_keys
                        );
                    }
                    for genotype in stage.genotypes.values() {
                        let reference = &genotype.genome["_reference"];
                        if reference == "~" {
                            assert_eq!(genotype.genome.len(), 1);
                            assert!(genotype.genes.is_empty());
                        } else {
                            assert!(reference.starts_with('g'));
                            reference[1..].parse::<usize>().unwrap();
                            assert_eq!(
                                genotype.genome.keys().cloned().collect::<BTreeSet<_>>(),
                                keys(GENOME_KEYS)
                            );
                            for (key, value) in &genotype.genome {
                                match key.as_str() {
                                    "_reference" | "_cladeID" => {}
                                    key if "_plague _disperseChildren _generationBattle _siblingBattle _altruist _familial _social _peaceful _passive _clockwise _mimicall _modifiespink _modifieslilac _modifiessky _selfish _updateClade".split_whitespace().any(|name| name == key) => validate_primitive("boolean", value),
                                    _ => validate_primitive("int", value),
                                }
                            }
                            assert_eq!(
                                genotype.genes.len(),
                                integer(&genotype.genome, "_nGenes") as usize
                            );
                            for (i, (index, gene)) in genotype.genes.iter().enumerate() {
                                assert_eq!(i, *index);
                                let expected: BTreeSet<_> =
                                    ["_length", "_theta", "_color", "_branch"]
                                        .map(str::to_owned)
                                        .into_iter()
                                        .chain(
                                            REACTION_NAMES.map(|name| format!("_{name}reaction")),
                                        )
                                        .collect();
                                assert_eq!(gene.keys().cloned().collect::<BTreeSet<_>>(), expected);
                                for (key, value) in gene {
                                    validate_value(
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
                    assert_eq!(stage.world.len(), 6);
                    assert!(stage.suffix.is_some());
                }
                cases.push(current.take().unwrap());
                coverage.metadata_rows += 1;
            }
            kind => panic!("Unknown birth row {kind} at {}", index + 1),
        }
    }
    assert!(current.is_none());
    cases
}

fn apply_control(parent: &mut Organism, name: &str, input: &Input) {
    match (name, input.kind.as_str()) {
        ("_energy", "double") => parent.state.energy = decoded(&input.value),
        ("dx", "double") => parent.state.velocity[0] = decoded(&input.value),
        ("dy", "double") => parent.state.velocity[1] = decoded(&input.value),
        ("_earlyReproduceEnergy", "int") => {
            parent.state.reproduction.early_energy = input.value.parse().unwrap()
        }
        ("_reproduceEnergy", "int") => {
            parent.state.reproduction.energy = input.value.parse().unwrap()
        }
        ("_infectedReproduceEnergy", "int") => {
            parent.state.reproduction.infected_energy = input.value.parse().unwrap()
        }
        ("_nChildren", "int") => parent.state.children = input.value.parse().unwrap(),
        ("_nVirusChildren", "int") => parent.state.virus_children = input.value.parse().unwrap(),
        ("_indigo", "int") => parent.effects.indigo = input.value.parse().unwrap(),
        ("_sporetime", "int") => parent.effects.spore_time = input.value.parse().unwrap(),
        ("_timeToReproduce", "int") => {
            parent.state.reproduction.countdown = input.value.parse().unwrap()
        }
        ("_isreproductive", "boolean") => {
            parent.effects.is_reproductive = input.value.parse().unwrap()
        }
        _ => panic!("Unsupported typed birth control {name}:{}", input.kind),
    }
}

#[derive(Clone, Copy)]
enum Child<'a> {
    Absent,
    Constructor(&'a Organism),
    Placed(&'a Organism),
    Failed,
}

fn same(left: Option<&GenomeRef>, right: Option<&GenomeRef>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if Rc::ptr_eq(left, right))
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
    references.compare(
        actual,
        &expected.genome["_reference"],
        &format!("{case}\t{stage}\t{owner}"),
        errors,
    );
    coverage.compared_rows +=
        expected.genome.len() + expected.genes.values().map(Fields::len).sum::<usize>();
    let Some(actual) = actual else {
        return;
    };
    let actual = actual.borrow();
    let mut target = expected.genome.clone();
    target.remove("_reference");
    let mut fields = genome_fields(&actual);
    fields.insert("_nGenes".into(), actual.genes().unwrap().len().to_string());
    compare_fields(case, stage, owner, &fields, &target, errors);
    for (index, gene) in &expected.genes {
        if *index < actual.genes().unwrap().len() {
            compare_fields(
                case,
                stage,
                &format!("{owner}.gene{index}"),
                &gene_fields(&actual.gene_ref(*index).unwrap().borrow()),
                gene,
                errors,
            );
        } else {
            errors.push(format!("{case}\t{stage}\t{owner}\tmissing gene{index}"));
        }
    }
}

fn snapshot(
    case: &BirthCase,
    name: &str,
    world: &World,
    child: Child<'_>,
    failed: Option<&GenomeRef>,
    original_infected: Option<&GenomeRef>,
    initial_saved: Option<&GenomeRef>,
    random: &JavaRandom,
    pending_append: bool,
    references: &mut References,
    coverage: &mut Coverage,
    errors: &mut Vec<String>,
) {
    let expected = &case.stages[name];
    let parent_ref = Rc::clone(&world.organisms.borrow()[0]);
    let parent = parent_ref.borrow();
    let constructor = if let Child::Constructor(actor) = child {
        Some(View::from(actor))
    } else {
        None
    };
    for (owner, target) in &expected.organisms {
        let view = if owner == "parent" {
            View::from(&*parent)
        } else {
            match child {
                Child::Placed(child) => View::from(child),
                Child::Constructor(_) => {
                    let view = constructor.as_ref().unwrap();
                    let mut actual = organism_fields(view);
                    actual.retain(|key, _| target.contains_key(key));
                    compare_fields(&case.name, name, "O", &actual, target, errors);
                    coverage.compared_rows += target.len();
                    continue;
                }
                Child::Failed => {
                    coverage.failed_organism_rows += target.len();
                    continue;
                }
                Child::Absent => panic!("Unexpected child snapshot {}:{name}", case.name),
            }
        };
        let mut actual = organism_fields(&view);
        actual.retain(|key, _| target.contains_key(key));
        compare_fields(&case.name, name, "O", &actual, target, errors);
        coverage.compared_rows += target.len();
    }
    let current_child = if let Child::Placed(child) = child {
        child.genome.as_ref()
    } else {
        None
    };
    for (owner, target) in &expected.genotypes {
        let actual = match owner.as_str() {
            "parent.genotype" => parent.genome.as_ref(),
            "parent.infected" => parent.infected_genome.as_ref(),
            "parent.saved" => parent.saved_genome.as_ref(),
            "child.genotype" if matches!(child, Child::Failed) => {
                coverage.failed_genome_rows += target.genome.len();
                coverage.failed_gene_rows += target.genes.values().map(Fields::len).sum::<usize>();
                continue;
            }
            "child.genotype" => current_child,
            "failed.genotype" => failed,
            owner => panic!("Unknown birth genotype owner {owner}"),
        };
        compare_genotype(
            &case.name, name, owner, actual, target, references, coverage, errors,
        );
    }
    let mut expected_aliases = keys(ALIAS_KEYS);
    if name.ends_with("-after") {
        expected_aliases.extend(keys("inherit_success direction_x direction_y"));
    }
    assert_eq!(
        expected.assertions.keys().cloned().collect::<BTreeSet<_>>(),
        expected_aliases
    );
    for (key, target) in &expected.assertions {
        if matches!(child, Child::Failed)
            && (key.starts_with("child_genotype_") || key.starts_with("direction_"))
        {
            coverage.failed_alias_rows += 1;
            continue;
        }
        let actual = match key.as_str() {
            "child_genotype_is_parent" => same(current_child, parent.genome.as_ref()).to_string(),
            "child_genotype_is_infected" => {
                same(current_child, parent.infected_genome.as_ref()).to_string()
            }
            "child_genotype_is_saved" => {
                same(current_child, parent.saved_genome.as_ref()).to_string()
            }
            "child_genotype_is_failed" => same(current_child, failed).to_string(),
            "child_genotype_is_initial_saved" => same(current_child, initial_saved).to_string(),
            "saved_is_failed" => same(parent.saved_genome.as_ref(), failed).to_string(),
            "infected_is_original" => {
                same(parent.infected_genome.as_ref(), original_infected).to_string()
            }
            "inherit_success" => matches!(child, Child::Placed(_)).to_string(),
            "direction_x" | "direction_y" => {
                let Child::Placed(child) = child else {
                    unreachable!()
                };
                let axis = usize::from(key == "direction_y");
                child.state.integer_center[axis]
                    .wrapping_sub(parent.state.integer_center[axis])
                    .signum()
                    .to_string()
            }
            key => panic!("Unknown birth assertion {key}"),
        };
        if &actual != target {
            errors.push(format!(
                "{}\t{name}\tA\t{key}\tactual={actual}\texpected={target}",
                case.name
            ));
        }
        coverage.compared_rows += 1;
    }
    let mut actual = Fields::new();
    actual.insert("CO2".into(), bits(world.carbon_dioxide.get()));
    actual.insert("O2".into(), bits(world.oxygen.get()));
    actual.insert("CH4".into(), bits(world.methane.get()));
    actual.insert("NEXT_ID".into(), world.next_id.get().to_string());
    actual.insert(
        "stored_organisms".into(),
        (world.organisms.borrow().len() - usize::from(pending_append)).to_string(),
    );
    let mut target = expected.world.clone();
    assert_eq!(target.remove("population").as_deref(), Some("0"));
    // The oracle appends without addOrganism; population and statistics integration belong to frame execution
    coverage.population_rows += 1;
    compare_fields(&case.name, name, "W", &actual, &target, errors);
    coverage.compared_rows += target.len();
    let mut peek = random.clone();
    let actual = (
        peek.next_i64(),
        peek.next_bounded(10000).unwrap(),
        peek.next_f64().to_bits(),
    );
    if Some(actual) != expected.suffix {
        errors.push(format!(
            "{}\t{name}\tR\tactual={actual:?}\texpected={:?}",
            case.name, expected.suffix
        ));
    }
    coverage.compared_rows += 1;
    coverage.snapshots += 1;
}

fn rebuild_tree(
    case: &BirthCase,
    stage: &str,
    world: &mut World,
    parent_id: i64,
    blocker_id: Option<i64>,
    coverage: &mut Coverage,
    errors: &mut Vec<String>,
) {
    let tree = &case.trees[stage];
    assert_eq!(tree.count, Some(tree.members.len()));
    assert_eq!(tree.members.len(), world.organisms.borrow().len());
    *world.frame_objects.borrow_mut() = world.organisms.borrow().clone();
    *world.spatial.borrow_mut() = crate::spatial::SpatialTree::build_actor_entries(
        world.frame_objects.borrow()
            .iter()
            .enumerate()
            .map(|(index, organism)| (index, organism.borrow().state.bounds.get())),
    |token| Rc::clone(&world.frame_objects.borrow()[token]));
    coverage.compared_rows += 1;
    for (index, (name, bounds, envelope)) in tree.members.iter().enumerate() {
        let actual_ref = Rc::clone(&world.frame_objects.borrow()[index]);
        let actual = actual_ref.borrow();
        let id = match name.as_str() {
            "parent" => parent_id,
            "blocker" => blocker_id.unwrap(),
            other => panic!("Unknown tree member {other}"),
        };
        assert_eq!(actual.id, id);
        let actual_bounds = actual.state.bounds.get();
        if actual_bounds != *bounds {
            errors.push(format!(
                "{}\t{stage}\tT\tbounds\tactual={actual_bounds:?}\texpected={bounds:?}",
                case.name
            ));
        }
        let edges = [
            f64::from(actual_bounds.x),
            f64::from(actual_bounds.x) + f64::from(actual_bounds.width),
            f64::from(actual_bounds.y),
            f64::from(actual_bounds.y) + f64::from(actual_bounds.height),
        ];
        assert!(
            actual_bounds.width >= 0 && actual_bounds.height >= 0,
            "Birth fixtures use positive tree envelopes"
        );
        if edges.map(f64::to_bits) != *envelope {
            errors.push(format!("{}\t{stage}\tT\tenvelope differs", case.name));
        }
        coverage.compared_rows += 1;
    }
}

fn blocker_genome() -> Genome {
    let gene = Gene::new(12.0, 0.0, crate::ColorKind::Green.argb(), -1, [0; 36]).shared();
    let mut code = Genome::from_parts(
        &[gene],
        GenomeProperties {
            symmetry: 1,
            modifiers: GenomeModifiers {
                cream: 1,
                fallow: 1,
                spore: 1,
                adapt_spore: 1,
                ..GenomeModifiers::default()
            },
            ..GenomeProperties::default()
        },
    );
    code.set_generation(1);
    code.set_clade_id(Some("blocker".into()));
    code
}

fn run(case: BirthCase, coverage: &mut Coverage, errors: &mut Vec<String>) {
    let input = |stage: &str, key: &str| &case.inputs[&(stage.to_owned(), key.to_owned())];
    assert_eq!(input("setup", "parent_setup_seed").kind, "long");
    let mut parameters = parameters(&case.settings);
    parameters.spore_energy_consumption = real(&case.settings, "SPORE_ENERGY_CONSUMPTION");
    let mut world = World::with_parameters(
        WorldConfig {
            seed: input("setup", "parent_setup_seed").value.parse().unwrap(),
            width: integer(&case.settings, "WORLD_WIDTH"),
            height: integer(&case.settings, "WORLD_HEIGHT"),
            initial_attempts: 0,
        },
        parameters,
    )
    .unwrap();
    assert_eq!(
        (
            bits(world.oxygen.get()),
            bits(world.carbon_dioxide.get()),
            bits(world.methane.get())
        ),
        (
            case.settings["INITIAL_O2"].clone(),
            case.settings["INITIAL_CO2"].clone(),
            case.settings["INITIAL_CH4"].clone()
        )
    );
    let parent_id = world
        .paste_genome(
            explicit_genome(&case.stages["pasted"].genotypes["parent.genotype"]),
            [3500, 3500],
        )
        .unwrap()
        .unwrap();
    for (target, owner, marker) in [
        (
            "parent._infectedGeneticCode",
            "parent.infected",
            "explicit-virus-genome",
        ),
        (
            "parent._savedGeneticCode",
            "parent.saved",
            "explicit-saved-genome",
        ),
    ] {
        if let Some(value) = case.inputs.get(&("setup".into(), target.into())) {
            assert_eq!(
                (value.kind.as_str(), value.value.as_str()),
                ("GeneticCode", marker)
            );
            let code = Rc::new(RefCell::new(explicit_genome(
                &case.stages["pasted"].genotypes[owner],
            )));
            if owner == "parent.infected" {
                world.organisms.borrow()[0].borrow_mut().infected_genome = Some(code);
            } else {
                world.organisms.borrow()[0].borrow_mut().saved_genome = Some(code);
            }
        }
    }
    let original_infected = world.organisms.borrow()[0].borrow().infected_genome.clone();
    let initial_saved = world.organisms.borrow()[0].borrow().saved_genome.clone();
    let mut references = References::default();
    snapshot(
        &case,
        "pasted",
        &world,
        Child::Absent,
        None,
        original_infected.as_ref(),
        initial_saved.as_ref(),
        &world.random_snapshot(),
        false,
        &mut references,
        coverage,
        errors,
    );
    for ((stage, target), value) in &case.inputs {
        if stage == "parent-controls" {
            if target == "world.O2_after_paste" {
                assert_eq!(value.kind, "double");
                // This labelled fixture input sets oxygen before the birth operation under comparison
                world.oxygen.set(decoded(&value.value));
            } else {
                apply_control(&mut world.organisms.borrow()[0].borrow_mut(), target, value);
            }
        }
    }
    snapshot(
        &case,
        "prepared",
        &world,
        Child::Absent,
        None,
        original_infected.as_ref(),
        initial_saved.as_ref(),
        &world.random_snapshot(),
        false,
        &mut references,
        coverage,
        errors,
    );
    let blocker_id = if let Some(bounds) = case
        .inputs
        .get(&("blocking".into(), "blocker.bounds".into()))
    {
        assert_eq!(bounds.kind, "Rectangle");
        let id = world
            .paste_genome(blocker_genome(), [1000, 1000])
            .unwrap()
            .unwrap();
        world.organisms.borrow().last().unwrap().borrow().state.bounds.set(rect(&bounds.value));
        if let Some(preview) = case
            .inputs
            .get(&("blocking".into(), "unblocked_source_child.bounds".into()))
        {
            assert_eq!(preview.value, bounds.value);
        }
        Some(id)
    } else {
        None
    };
    rebuild_tree(
        &case, "initial", &mut world, parent_id, blocker_id, coverage, errors,
    );
    assert_eq!(input("birth", "birth_seed_reset").kind, "long");
    assert_eq!(
        input("birth", "birth_seed_reset")
            .value
            .parse::<i64>()
            .unwrap(),
        case.seed
    );
    *world.session.0.random.borrow_mut() = JavaRandom::new(case.seed);
    let attempts = case
        .inputs
        .keys()
        .filter(|(stage, key)| stage.starts_with("attempt-") && key == "first")
        .count();
    let mut failed = None;
    for number in 0..attempts {
        if number == 1 && case.operation == "retry" {
            assert_eq!(
                case.list_operations
                    .get(&("retry".into(), "remove_blocker".into()))
                    .map(String::as_str),
                Some("blocker")
            );
            world.organisms.borrow_mut()
                .retain(|organism| Some(organism.borrow().id) != blocker_id);
            rebuild_tree(
                &case, "retry", &mut world, parent_id, None, coverage, errors,
            );
        }
        let stage = format!("attempt-{number}");
        let first = input(&stage, "first");
        assert_eq!(first.kind, "boolean");
        // Peek the production constructor on a detached world; spawn_child consumes the live draw exactly once
        let peek = world.clone();
        let constructor = Organism::pasted_candidate(None, peek.initial_organism_state());
        snapshot(
            &case,
            &format!("{stage}-before"),
            &world,
            Child::Constructor(&constructor),
            failed.as_ref(),
            original_infected.as_ref(),
            initial_saved.as_ref(),
            &peek.random_snapshot(),
            false,
            &mut references,
            coverage,
            errors,
        );
        let result = world
            .spawn_child(parent_id, first.value.parse().unwrap())
            .unwrap();
        let child_ref = result.map(|id| {
            world.organisms.borrow()
                .iter()
                .find(|organism| organism.borrow().id == id)
                .cloned()
                .unwrap()
        });
        let child = child_ref.as_ref().map(|actor| actor.borrow());
        snapshot(
            &case,
            &format!("{stage}-after"),
            &world,
            child.as_deref().map_or(Child::Failed, Child::Placed),
            failed.as_ref(),
            original_infected.as_ref(),
            initial_saved.as_ref(),
            &world.random_snapshot(),
            result.is_some(),
            &mut references,
            coverage,
            errors,
        );
        let appended = case.list_operations.get(&(
            format!("{stage}-after"),
            "append_without_tree_rebuild".into(),
        ));
        assert_eq!(
            appended.map(String::as_str),
            result.map(|_| format!("child-{number}")).as_deref()
        );
        coverage.attempts += 1;
        if result.is_some() {
            coverage.successes += 1;
        } else {
            coverage.failures += 1;
        }
        if number == 0 && case.operation == "retry" {
            assert!(
                result.is_none(),
                "Retry fixtures begin with an actual blocked attempt"
            );
            failed = Some(Rc::clone(
                world.organisms.borrow()[0]
                    .borrow()
                    .saved_genome
                    .as_ref()
                    .expect("Retry candidate must remain observable through saved genotype"),
            ));
        }
    }
    let expected_stage_names: BTreeSet<_> = ["pasted".to_owned(), "prepared".to_owned()]
        .into_iter()
        .chain((0..attempts).flat_map(|number| {
            [
                format!("attempt-{number}-before"),
                format!("attempt-{number}-after"),
            ]
        }))
        .collect();
    assert_eq!(
        case.stages.keys().cloned().collect::<BTreeSet<_>>(),
        expected_stage_names
    );
    let expected_list_rows = case
        .stages
        .values()
        .filter(|stage| {
            stage
                .assertions
                .get("inherit_success")
                .is_some_and(|success| success == "true")
        })
        .count()
        + usize::from(case.operation == "retry");
    assert_eq!(case.list_operations.len(), expected_list_rows);
    assert_eq!(
        case.trees.len(),
        if case.operation == "retry" { 2 } else { 1 }
    );
    for (stage, target) in case.inputs.keys() {
        assert!(
            match stage.as_str() {
                "setup" => [
                    "parent_setup_seed",
                    "parent._infectedGeneticCode",
                    "parent._savedGeneticCode"
                ]
                .contains(&target.as_str()),
                "parent-controls" => true,
                "blocking" =>
                    ["blocker.bounds", "unblocked_source_child.bounds"].contains(&target.as_str()),
                "birth" => target == "birth_seed_reset",
                stage if stage.starts_with("attempt-") => target == "first",
                _ => false,
            },
            "Unused birth input {stage}:{target}"
        );
    }
    coverage.cases += 1;
}

#[test]
fn actual_child_construction_matches_java_birth_observations() {
    let expected = Coverage {
        cases: 63,
        attempts: 80,
        successes: 65,
        failures: 15,
        snapshots: 286,
        compared_rows: 78_991,
        failed_organism_rows: 870,
        failed_genome_rows: 258,
        failed_gene_rows: 1_440,
        failed_alias_rows: 105,
        population_rows: 286,
        metadata_rows: 10_211,
    };
    compare_fixture(
        "birth",
        include_str!("../tests/fixtures/birth.tsv"),
        false,
        92_161,
        expected,
    );
}

#[test]
fn actual_birth_spore_variants_match_full_java_organism_observations() {
    let expected = Coverage {
        cases: 80,
        attempts: 80,
        successes: 80,
        failures: 0,
        snapshots: 320,
        compared_rows: 129_160,
        population_rows: 320,
        metadata_rows: 12_888,
        ..Coverage::default()
    };
    compare_fixture(
        "birth-variants",
        include_str!("../tests/fixtures/birth_variants.tsv"),
        true,
        142_368,
        expected,
    );
}

fn compare_fixture(name: &str, text: &str, full: bool, rows: usize, expected: Coverage) {
    assert_eq!(text.lines().count(), rows);
    let mut coverage = Coverage::default();
    let cases = parse(text, full, &mut coverage);
    let mut errors = Vec::new();
    for case in cases {
        run(case, &mut coverage, &mut errors);
    }
    if coverage != expected {
        errors.push(format!(
            "Birth comparison scope changed: actual={coverage:?}; expected={expected:?}"
        ));
    }
    if !errors.is_empty() {
        let report = std::env::temp_dir().join(format!("{name}-oracle-mismatches.tsv"));
        std::fs::write(&report, errors.join("\n") + "\n").unwrap();
        panic!(
            "{} birth mismatches; report={}\n{}",
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
        "BIRTH_COMPARISON {name} {coverage:?}; failed candidates/population excluded; caller scheduling/statistics unexercised"
    );
}
