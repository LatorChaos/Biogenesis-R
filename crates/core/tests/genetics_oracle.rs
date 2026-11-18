// SPDX-License-Identifier: GPL-2.0-or-later

use biogenesis_core::{
    DEFAULT_COLORS, Gene, GeneRef, GeneticsParams, Genome, GenomeModifiers, GenomeProperties,
    JavaRandom, REACTION_NAMES,
};
use std::{collections::BTreeMap, rc::Rc};

type Fields = BTreeMap<String, String>;

struct Case {
    name: String,
    operation: String,
    seed: i64,
    params: GeneticsParams,
    weights_seen: [bool; 46],
    genomes: BTreeMap<String, Fields>,
    genes: BTreeMap<String, BTreeMap<usize, Fields>>,
    assertions: Fields,
    suffix: Option<(i64, i32, u64)>,
    exception: bool,
}

fn signed(fields: &Fields, key: &str) -> i32 {
    fields[key].parse().unwrap()
}
fn flag(fields: &Fields, key: &str) -> bool {
    fields[key].parse().unwrap()
}
fn floating(fields: &Fields, key: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(&fields[key], 16).unwrap())
}
fn nullable(value: &str) -> Option<String> {
    if value == "~" {
        None
    } else {
        Some(value.into())
    }
}

fn observed_modifiers(fields: &Fields) -> GenomeModifiers {
    GenomeModifiers {
        cream: signed(fields, "_modifiescream"),
        fallow: signed(fields, "_modifiesfallow"),
        spore: signed(fields, "_modifiesspore"),
        adapt_spore: signed(fields, "_adaptspore"),
        plague: flag(fields, "_plague"),
        disperse_children: flag(fields, "_disperseChildren"),
        generation_battle: flag(fields, "_generationBattle"),
        sibling_battle: flag(fields, "_siblingBattle"),
        altruist: flag(fields, "_altruist"),
        familial: flag(fields, "_familial"),
        social: flag(fields, "_social"),
        peaceful: flag(fields, "_peaceful"),
        passive: flag(fields, "_passive"),
        clockwise: flag(fields, "_clockwise"),
        mimic_all: flag(fields, "_mimicall"),
        modifies_pink: flag(fields, "_modifiespink"),
        modifies_lilac: flag(fields, "_modifieslilac"),
        modifies_sky: flag(fields, "_modifiessky"),
        selfish: flag(fields, "_selfish"),
    }
}

fn observed_gene(fields: &Fields) -> Gene {
    let color = if fields["_color"] == "~" {
        None
    } else {
        Some(u32::from_str_radix(&fields["_color"], 16).unwrap())
    };
    let reactions =
        std::array::from_fn(|index| signed(fields, &format!("_{}reaction", REACTION_NAMES[index])));
    Gene::new(
        floating(fields, "_length"),
        floating(fields, "_theta"),
        color,
        signed(fields, "_branch"),
        reactions,
    )
}

fn input_genome(case: &Case, label: &str) -> Genome {
    let fields = &case.genomes[label];
    let mut genes: Vec<GeneRef> = Vec::new();
    if let Some(rows) = case.genes.get(label) {
        for (index, fields) in rows {
            assert_eq!(*index, genes.len());
            let alias = signed(fields, "_alias_index");
            genes.push(if alias >= 0 {
                genes[alias as usize].clone()
            } else {
                observed_gene(fields).shared()
            });
        }
    }
    let mut genome = Genome::from_parts(
        &genes,
        GenomeProperties {
            symmetry: signed(fields, "_symmetry"),
            mirror: signed(fields, "_mirror"),
            mutation_rate: signed(fields, "_mutationrate"),
            activity: signed(fields, "_activity"),
            modifiers: observed_modifiers(fields),
        },
    );
    genome.set_generation(signed(fields, "_generation"));
    genome.set_clade_id(nullable(&fields["_cladeID"]));
    genome.set_update_clade(flag(fields, "_updateClade"));
    genome
}

fn explicit_gene(index: i32) -> Gene {
    Gene::new(
        20.25 + index as f64,
        -0.375 - index as f64 / 8.0,
        Some(0x12000000 | ((index as u32 + 1) * 0x12345 & 0xffffff)),
        if index == 0 { -1 } else { index },
        std::array::from_fn(|reaction| 1000 + index * 100 + reaction as i32),
    )
}

fn explicit_wrapper(genes: &[GeneRef]) -> Genome {
    let mut genome = Genome::from_parts(
        genes,
        GenomeProperties {
            symmetry: 4,
            mirror: 1,
            mutation_rate: 0,
            activity: 2,
            modifiers: GenomeModifiers {
                cream: 7,
                fallow: 4,
                spore: 12,
                adapt_spore: 70,
                plague: true,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: false,
                altruist: true,
                familial: false,
                social: true,
                peaceful: false,
                passive: true,
                clockwise: false,
                mimic_all: true,
                modifies_pink: false,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: true,
            },
        },
    );
    genome.set_generation(41);
    genome.set_clade_id(Some("root+3-5>8<9".into()));
    genome.set_update_clade(true);
    genome
}

fn edit_gene(gene: &mut Gene, all: bool) {
    gene.set_length(if all {
        f64::from_bits(0x7ff8000000000042)
    } else {
        -1234.5
    });
    if all {
        gene.set_theta(-0.0);
        gene.set_color(None);
        gene.set_branch(i32::MIN);
    }
    for index in 0..if all { 36 } else { 1 } {
        gene.set_reaction(index, if all { i32::MIN + index as i32 } else { -777 });
    }
}

fn compare(case: &Case, label: &str, actual: &Genome, parent: Option<&Genome>) {
    let expected = &case.genomes[label];
    assert_eq!(expected.len(), 26, "{} metadata schema", case.name);
    let context = format!("{} {label}", case.name);
    assert_eq!(
        actual.symmetry(),
        signed(expected, "_symmetry"),
        "{context}"
    );
    assert_eq!(actual.mirror(), signed(expected, "_mirror"), "{context}");
    assert_eq!(
        actual.mutation_rate(),
        signed(expected, "_mutationrate"),
        "{context}"
    );
    assert_eq!(
        actual.activity(),
        signed(expected, "_activity"),
        "{context}"
    );
    assert_eq!(
        actual.generation(),
        signed(expected, "_generation"),
        "{context}"
    );
    assert_eq!(
        actual.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(),
        nullable(&expected["_cladeID"]).as_deref(),
        "{context}"
    );
    assert_eq!(
        actual.update_clade(),
        flag(expected, "_updateClade"),
        "{context}"
    );
    assert_eq!(
        actual.modifiers(),
        observed_modifiers(expected),
        "{context}"
    );
    let rows = case.genes.get(label);
    assert_eq!(
        actual.genes().unwrap().len(),
        rows.map_or(0, BTreeMap::len),
        "{context}"
    );
    if let Some(rows) = rows {
        for (index, fields) in rows {
            assert_eq!(fields.len(), 42, "{context} gene schema");
            let gene_handle = actual.gene_ref(*index).unwrap();
            let gene = gene_handle.borrow();
            let expected_gene = observed_gene(fields);
            assert_eq!(
                gene.length().to_bits(),
                expected_gene.length().to_bits(),
                "{context} gene {index} length"
            );
            assert_eq!(
                gene.theta().to_bits(),
                expected_gene.theta().to_bits(),
                "{context} gene {index} theta"
            );
            assert_eq!(
                gene.color(),
                expected_gene.color(),
                "{context} gene {index} color"
            );
            assert_eq!(
                gene.branch(),
                expected_gene.branch(),
                "{context} gene {index} branch"
            );
            assert_eq!(
                gene.reactions(),
                expected_gene.reactions(),
                "{context} gene {index} reactions"
            );
            let shared = parent
                .and_then(|parent| {
                    parent
                        .genes().unwrap()
                        .iter()
                        .position(|candidate| Rc::ptr_eq(candidate, &actual.genes().unwrap()[*index]))
                })
                .map_or(-1, |position| position as i32);
            let alias = actual.genes().unwrap()[..*index]
                .iter()
                .position(|candidate| Rc::ptr_eq(candidate, &actual.genes().unwrap()[*index]))
                .map_or(-1, |position| position as i32);
            assert_eq!(
                shared,
                signed(fields, "_shared_parent_index"),
                "{context} gene {index} parent sharing"
            );
            assert_eq!(
                alias,
                signed(fields, "_alias_index"),
                "{context} gene {index} slot sharing"
            );
        }
    }
}

fn run_case(case: Case) {
    let mut random = JavaRandom::new(case.seed);
    if case.exception {
        let failed = match case.operation.as_str() {
            "gene-random" => Gene::random_with(&mut random, &case.params.color_weights).is_err(),
            "genome-random" => Genome::random_with(&mut random, &case.params).is_err(),
            operation => panic!("Unspecified invalid-input operation {operation}"),
        };
        assert!(failed, "{} malformed boundary", case.name);
        assert_eq!(
            random,
            JavaRandom::new(case.seed),
            "Explicit Rust validation must precede random draws"
        );
        return;
    }
    let parent = case
        .genomes
        .contains_key("input")
        .then(|| input_genome(&case, "input"));
    if let Some(parent) = &parent {
        compare(&case, "input", parent, None);
    }
    let mut assertions = BTreeMap::<String, String>::new();
    let mut explicit_alias: Option<GeneRef> = None;
    let child = match case.operation.as_str() {
        "gene-blank" => explicit_wrapper(&[Gene::default().shared()]),
        "gene-explicit" => explicit_wrapper(&[explicit_gene(0).shared()]),
        "gene-random" => {
            explicit_wrapper(&[Gene::random_with(&mut random, &case.params.color_weights)
                .unwrap()
                .shared()])
        }
        "gene-color" => {
            let input = parent.as_ref().unwrap();
            input
                .gene_ref(0).unwrap().borrow_mut()
                .randomize_color(&mut random, &case.params.color_weights)
                .unwrap();
            explicit_wrapper(&input.genes().unwrap())
        }
        "gene-setters" => {
            let input = parent.as_ref().unwrap();
            edit_gene(&mut input.gene_ref(0).unwrap().borrow_mut(), true);
            assertions.insert("edit_all_fields".into(), "true".into());
            explicit_wrapper(&input.genes().unwrap())
        }
        "gene-clone" => {
            let input = parent.as_ref().unwrap();
            let gene = input.gene_ref(0).unwrap().borrow().clone().shared();
            assertions.insert(
                "clone_gene_distinct".into(),
                (!Rc::ptr_eq(&gene, &input.genes().unwrap()[0])).to_string(),
            );
            assertions.insert(
                "clone_color_shared".into(),
                (gene.borrow().color() == input.gene_ref(0).unwrap().borrow().color()).to_string(),
            );
            explicit_wrapper(&[gene])
        }
        "genome-empty" => explicit_wrapper(&[]),
        "genome-explicit" => {
            let input = parent.as_ref().unwrap();
            let mut supplied = input.genes().unwrap().to_vec();
            let shared = supplied[0].clone();
            let mut output = Genome::from_parts(&supplied, input.properties());
            assert_eq!(output.generation(), 0);
            assert_eq!(output.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), None);
            assert!(!output.update_clade());
            output.set_generation(input.generation());
            output.set_clade_text(input.clade_id().cloned());
            output.set_update_clade(input.update_clade());
            assertions.insert(
                "explicit_array_distinct".into(),
                (!input.gene_array().unwrap().same_instance(output.gene_array().unwrap())).to_string(),
            );
            supplied[0] = explicit_gene(9).shared();
            assertions.insert(
                "list_replacement_detached".into(),
                Rc::ptr_eq(&output.genes().unwrap()[0], &shared).to_string(),
            );
            explicit_alias = Some(shared);
            output
        }
        "genome-random" => Genome::random_with(&mut random, &case.params).unwrap(),
        "genome-clone" => {
            let input = parent.as_ref().unwrap();
            let output = input.java_clone().unwrap();
            assertions.insert(
                "clone_array_distinct".into(),
                (!input.gene_array().unwrap().same_instance(output.gene_array().unwrap())).to_string(),
            );
            assertions.insert(
                "clone_repeated_slots_distinct".into(),
                (!Rc::ptr_eq(&output.genes().unwrap()[0], &output.genes().unwrap()[1])).to_string(),
            );
            output
        }
        "inherit" => {
            Genome::inherit_with(parent.as_ref().unwrap(), &mut random, &case.params).unwrap()
        }
        operation => panic!("Unknown fixture operation {operation}"),
    };
    compare(&case, "output", &child, parent.as_ref());
    if let Some(shared) = explicit_alias {
        assert_eq!(
            case.assertions["edit_supplied_gene_length"],
            format!("{:016x}", (-1234.5_f64).to_bits())
        );
        assert_eq!(case.assertions["edit_supplied_gene_redreaction"], "-777");
        edit_gene(&mut shared.borrow_mut(), false);
        assertions.insert(
            "supplied_gene_edit_shared".into(),
            (child.gene_ref(1).unwrap().borrow().length().to_bits() == shared.borrow().length().to_bits()).to_string(),
        );
        compare(&case, "output_after", &child, parent.as_ref());
        assertions.insert(
            "edit_supplied_gene_length".into(),
            case.assertions["edit_supplied_gene_length"].clone(),
        );
        assertions.insert("edit_supplied_gene_redreaction".into(), "-777".into());
    }
    if case.assertions.contains_key("edit_child_gene0_length") {
        assert_eq!(
            case.assertions["edit_child_gene0_length"],
            format!("{:016x}", (-1234.5_f64).to_bits())
        );
        assert_eq!(case.assertions["edit_child_gene0_redreaction"], "-777");
        edit_gene(&mut child.gene_ref(0).unwrap().borrow_mut(), false);
        compare(&case, "output_after", &child, parent.as_ref());
        assertions.insert(
            "edit_child_gene0_length".into(),
            case.assertions["edit_child_gene0_length"].clone(),
        );
        assertions.insert("edit_child_gene0_redreaction".into(), "-777".into());
    }
    if case.genomes.contains_key("input_after") {
        compare(&case, "input_after", parent.as_ref().unwrap(), None);
    }
    assert_eq!(
        assertions, case.assertions,
        "{} semantic assertions",
        case.name
    );
    let suffix = case.suffix.unwrap();
    assert_eq!(
        random.next_i64(),
        suffix.0,
        "{} following nextLong",
        case.name
    );
    assert_eq!(
        random.next_bounded(10000).unwrap(),
        suffix.1,
        "{} following nextInt",
        case.name
    );
    assert_eq!(
        random.next_f64().to_bits(),
        suffix.2,
        "{} following nextDouble",
        case.name
    );
}

#[test]
fn complete_genetics_matches_independent_java_fixture() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/genetics.tsv");
    let text = std::fs::read_to_string(path).expect("Independent genetics fixture must exist");
    let mut current: Option<Case> = None;
    let mut cases = 0;
    for line in text.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields[0] == "C" {
            assert!(current.is_none());
            current = Some(Case {
                name: fields[1].into(),
                operation: fields[2].into(),
                seed: fields[3].parse().unwrap(),
                params: GeneticsParams {
                    initial_complexity: fields[4].parse().unwrap(),
                    min_mutation_rate: fields[5].parse().unwrap(),
                    max_mutation_rate: fields[6].parse().unwrap(),
                    meta_mutation_rate: fields[7].parse().unwrap(),
                    clade_complexity: fields[8].parse().unwrap(),
                    color_weights: [0; 46],
                },
                weights_seen: [false; 46],
                genomes: BTreeMap::new(),
                genes: BTreeMap::new(),
                assertions: BTreeMap::new(),
                suffix: None,
                exception: false,
            });
            continue;
        }
        if fields[0] == "E" {
            let case = current.take().unwrap();
            assert!(
                case.weights_seen.iter().all(|seen| *seen),
                "{} missing color settings",
                case.name
            );
            assert!(case.suffix.is_some(), "{} missing RNG suffix", case.name);
            run_case(case);
            cases += 1;
            continue;
        }
        let case = current.as_mut().unwrap();
        match fields[0] {
            "W" => {
                let index: usize = fields[1].parse().unwrap();
                assert!(!std::mem::replace(&mut case.weights_seen[index], true));
                assert_eq!(DEFAULT_COLORS[index].0, fields[2]);
                case.params.color_weights[index] = fields[3].parse().unwrap();
            }
            "G" => {
                assert!(
                    case.genomes
                        .entry(fields[1].into())
                        .or_default()
                        .insert(fields[2].into(), fields[3].into())
                        .is_none()
                );
            }
            "N" => {
                assert!(
                    case.genes
                        .entry(fields[1].into())
                        .or_default()
                        .entry(fields[2].parse().unwrap())
                        .or_default()
                        .insert(fields[3].into(), fields[4].into())
                        .is_none()
                );
            }
            "A" => {
                assert!(
                    case.assertions
                        .insert(fields[1].into(), fields[2].into())
                        .is_none()
                );
            }
            "R" => {
                assert!(
                    case.suffix
                        .replace((
                            fields[1].parse().unwrap(),
                            fields[2].parse().unwrap(),
                            u64::from_str_radix(fields[3], 16).unwrap()
                        ))
                        .is_none()
                );
            }
            "X" => case.exception = true,
            record => panic!("Unknown genetics fixture record {record}"),
        }
    }
    assert!(current.is_none());
    assert_eq!(
        cases, 126,
        "The adopted genetics fixture must remain complete"
    );
}
