// SPDX-License-Identifier: GPL-2.0-or-later
//! Primary recipes use C/U/I/IU inputs; all graph and RNG rows remain expectations
use super::phenotype_color_oracle_tests::{Identities, cases};
use super::*;
use crate::organism::{GenomeRef, OrganismRef};
use crate::{ColorKind, GeneArrayRef, GeneticsError, JavaText};
use std::rc::Rc;

#[path = "gene_array_graph_fixture.rs"]
mod graph_fixture;
const SOURCE: &str = include_str!("../tests/fixtures/gene_array/gene_array.tsv");
const DIRECTORY: &str = "crates/core/tests/fixtures/gene_array/";
type Row<'a> = Vec<&'a str>;
const RANDOM_HEADER: &str = "aced0005737200106a6176612e7574696c2e52616e646f6d363296344bf00a530300035a0014686176654e6578744e657874476175737369616e4400106e6578744e657874476175737369616e4a0004736565647870";
fn random_wire(random: &JavaRandom) -> String {
    format!(
        "{RANDOM_HEADER}000000000000000000{:016x}78",
        random.state48()
    )
}
fn fixed(clade: &JavaText) -> GenomeRef {
    let mut code = Genome::from_parts(
        &[Gene::new_with_color(12.0, 0.0, ColorKind::Green.color_ref(), 0, [0; 36]).shared()],
        GenomeProperties {
            symmetry: 1,
            mirror: 0,
            mutation_rate: 0,
            activity: 0,
            modifiers: crate::GenomeModifiers {
                cream: 1,
                fallow: 1,
                spore: 1,
                adapt_spore: 1,
                ..Default::default()
            },
        },
    );
    code.set_generation(41);
    code.set_clade_text(Some(clade.clone()));
    Rc::new(RefCell::new(code))
}
#[derive(Default)]
enum Returned {
    #[default]
    Null,
    Count(usize),
    Gene(crate::GeneRef),
    Code(GenomeRef),
}
struct Replay {
    world: World,
    actor: OrganismRef,
    parent: GenomeRef,
    peer: Option<GenomeRef>,
    original: crate::GeneRef,
    replacement: crate::GeneRef,
    returned: Returned,
}
impl Replay {
    fn new(rows: &[Row<'_>]) -> Self {
        let settings = rows
            .iter()
            .filter(|r| r[0] == "U")
            .map(|r| (r[1].to_string(), r[3].to_string()))
            .collect();
        let world = empty_reference_world(&settings, rows[0][3].parse().unwrap());
        let clade = JavaText::from_utf8("world_publication_witness");
        let parent = fixed(&clade);
        let actor = Rc::new(RefCell::new(
            world
                .construct_pasted_shared(Rc::clone(&parent), [100, 100])
                .unwrap()
                .unwrap(),
        ));
        actor.borrow().state.bounds.bind_actor(&actor).unwrap();
        world.organisms.borrow_mut().push(Rc::clone(&actor));
        actor.borrow_mut().saved_genome = Some(fixed(&clade));
        let original = parent.borrow().gene_ref(0).unwrap();
        let replacement =
            Gene::new_with_color(7.0, 0.25, ColorKind::Red.color_ref(), 0, [0; 36]).shared();
        let input = rows.iter().find(|r| r[0] == "I").unwrap();
        let mut peer = None;
        match input[1] {
            "parent_genes_null" | "clone_parent_genes_null_destination_sentinel" => {
                parent.borrow_mut().set_gene_array(None)
            }
            "parent_genes_one_null_slot" => parent
                .borrow_mut()
                .set_gene_array(Some(GeneArrayRef::from_slots(vec![None]))),
            "clone_parent_repeated_Gene_then_null_destination_sentinel" => parent
                .borrow_mut()
                .set_gene_array(Some(GeneArrayRef::from_slots(vec![
                    Some(Rc::clone(&original)),
                    Some(Rc::clone(&original)),
                    None,
                ]))),
            "inherit_parent_null_array_rate97_child_clamp0_destination_sentinel" => {
                let mut code = parent.borrow_mut();
                code.set_gene_array(None);
                let mut p = code.properties();
                p.mutation_rate = 97;
                code.set_properties(p);
            }
            "inherit_parent_repeated_Gene_null_middle_rate97_child_clamp0" => {
                let mut code = parent.borrow_mut();
                code.set_gene_array(Some(GeneArrayRef::from_slots(vec![
                    Some(Rc::clone(&original)),
                    None,
                    Some(Rc::clone(&original)),
                ])));
                let mut p = code.properties();
                p.mutation_rate = 97;
                code.set_properties(p);
            }
            "inherit_same_null_slot_parent_rate10000_child_clamps5000_then10000_fixed_seed" => {
                let mut code = parent.borrow_mut();
                code.set_gene_array(Some(GeneArrayRef::from_slots(vec![None])));
                let mut p = code.properties();
                p.mutation_rate = 10000;
                code.set_properties(p);
            }
            "two_holders_share_two_G_original_slots_G_replacement_len7_theta025_red" => {
                let shared = GeneArrayRef::from_slots(vec![
                    Some(Rc::clone(&original)),
                    Some(Rc::clone(&original)),
                ]);
                parent.borrow_mut().set_gene_array(Some(shared.clone()));
                let other = fixed(&clade);
                other.borrow_mut().set_gene_array(Some(shared));
                actor.borrow_mut().infected_genome = Some(Rc::clone(&other));
                peer = Some(other);
            }
            name => panic!("Unknown authored input {name}"),
        }
        Self {
            world,
            actor,
            parent,
            peer,
            original,
            replacement,
            returned: Returned::Null,
        }
    }
    fn call(
        &mut self,
        rows: &[Row<'_>],
        stage: &str,
        operation: &str,
    ) -> Result<(), GeneticsError> {
        for row in rows.iter().filter(|r| r[0] == "IU" && r[1] == stage) {
            match row[2] {
                "Utils.META_MUTATION_RATE" => {
                    self.world.session.0.parameters.borrow_mut().genetics.meta_mutation_rate = row[4].parse().unwrap()
                }
                "Utils.MIN_MUTATION_RATE" => {
                    self.world.session.0.parameters.borrow_mut().genetics.min_mutation_rate = row[4].parse().unwrap()
                }
                "Utils.MAX_MUTATION_RATE" => {
                    self.world.session.0.parameters.borrow_mut().genetics.max_mutation_rate = row[4].parse().unwrap()
                }
                "Utils.random_seed" => *self.world.session.0.random.borrow_mut() = JavaRandom::new(row[4].parse().unwrap()),
                "A_shared[1]" => {
                    assert_eq!(&row[3..], ["Gene", "G_replacement"]);
                    self.parent
                        .borrow()
                        .gene_array()
                        .unwrap()
                        .set(1, Some(Rc::clone(&self.replacement)))?;
                }
                "parent._genes" => {
                    assert_eq!(&row[3..], ["Gene[]", "A_replacement[G_original]"]);
                    self.parent
                        .borrow_mut()
                        .set_gene_array(Some(GeneArrayRef::from_slots(vec![Some(Rc::clone(
                            &self.original,
                        ))])));
                }
                name => panic!("Unknown authored per-call input {name}"),
            }
        }
        self.returned = Returned::Null;
        let before = rows.iter().find(|r| r[0] == "RB" && r[1] == stage).unwrap();
        assert_eq!(random_wire(&self.world.random_snapshot()), before[3]);
        self.returned = match operation {
            "parent_getNGenes" | "parent_getNGenes_after_replace" => {
                Returned::Count(self.parent.borrow().gene_count()?)
            }
            "parent_getGene_minus1" => self.slot(-1)?,
            "parent_getGene_zero" => self.slot(0)?,
            "parent_getGene_upper1" => self.slot(1)?,
            "parent_getGene_min" => self.slot(i32::MIN)?,
            "parent_getGene_max" => self.slot(i32::MAX)?,
            "peer_getGene1_after_store" | "peer_getGene1" => self
                .peer
                .as_ref()
                .unwrap()
                .borrow()
                .gene_slot(1)?
                .map_or(Returned::Null, Returned::Gene),
            "clone_parent" => {
                let code = Rc::new(RefCell::new(self.parent.borrow().java_clone()?));
                self.actor.borrow_mut().saved_genome = Some(Rc::clone(&code));
                Returned::Code(code)
            }
            "inherit_zero" | "inherit_mixed" | "inherit_all" => {
                let code = Rc::new(RefCell::new(Genome::inherit_with(
                    &self.parent.borrow(),
                    &mut *self.world.session.0.random.borrow_mut(),
                    &self.world.parameters().genetics,
                )?));
                self.actor.borrow_mut().saved_genome = Some(Rc::clone(&code));
                Returned::Code(code)
            }
            name => panic!("Unknown target operation {name}"),
        };
        Ok(())
    }
    fn slot(&self, index: i32) -> Result<Returned, GeneticsError> {
        Ok(self
            .parent
            .borrow()
            .gene_slot(index)?
            .map_or(Returned::Null, Returned::Gene))
    }
    fn snapshot(&self, rows: &[Row<'_>], stage: &str) -> Result<(), String> {
        let rows: Vec<_> = rows
            .iter()
            .filter(|r| r.len() > 1 && r[1] == stage)
            .collect();
        let mut ids = Identities::default();
        let world_label = rows.iter().find(|r| r[0] == "P").unwrap()[2];
        let member_list = field(&rows, world_label, "_organisms");
        let actor_label = element(&rows, member_list, 0);
        let actor = self.actor.borrow();
        for (name, code) in [
            ("_geneticCode", actor.genome.as_ref()),
            ("_infectedGeneticCode", actor.infected_genome.as_ref()),
            ("_savedGeneticCode", actor.saved_genome.as_ref()),
        ] {
            compare_code(&rows, &mut ids, field(&rows, actor_label, name), code)?;
        }
        let observed = organism_fields(&View::from(&*actor));
        for (name, value) in &observed {
            if ["_geneticCode", "_infectedGeneticCode", "_savedGeneticCode"]
                .contains(&name.as_str())
            {
                continue;
            }
            // The source frontend is transient and has no engine-independent live owner
            if name == "_visibleWorld" {
                continue;
            }
            let expected = if name == "bounds" {
                rectangle_fields(&rows, actor_label)
            } else if name == "_world" {
                equal("actor World", world_label, field(&rows, actor_label, name))?;
                "present".into()
            } else {
                projected(&rows, name, field(&rows, actor_label, name))
            };
            equal(name, value, &expected)?;
        }
        for (name, value) in world_fields(&self.world) {
            let expected = if name == "organism_count" {
                rows.iter()
                    .find(|r| r[0] == "L" && r[2] == member_list)
                    .unwrap()[3]
            } else {
                field(&rows, world_label, &name)
            };
            equal(&name, &value, expected)?;
        }
        let statistics_label = field(&rows, world_label, "worldStatistics");
        let statistics_owner = self.world.statistics_ref();
        let statistics = statistics_owner.borrow();
        let (types, _) = super::death_oracle_tests::statistics_schemas();
        let known_codes = std::collections::BTreeMap::new();
        for (name, kind) in &types {
            let expected = field(&rows, statistics_label, name);
            let history;
            if kind == "List" {
                let length = rows
                    .iter()
                    .find(|r| r[0] == "L" && r[2] == expected)
                    .unwrap()[3]
                    .parse::<usize>()
                    .unwrap();
                history = (0..length)
                    .map(|i| data(&rows, element(&rows, expected, i)))
                    .collect::<Vec<_>>()
                    .join(",");
                crate::statistics::tests::assert_observation(
                    crate::statistics::tests::observed_field(&statistics, name),
                    kind,
                    &history,
                    &known_codes,
                    name,
                );
            } else {
                crate::statistics::tests::assert_observation(
                    crate::statistics::tests::observed_field(&statistics, name),
                    kind,
                    expected,
                    &known_codes,
                    name,
                );
            }
        }
        drop(statistics);
        for field_kind in crate::organism_arrays::INT_FIELDS {
            let label = field(&rows, actor_label, field_kind.name());
            match actor.body.arrays.int(field_kind) {
                None => equal(field_kind.name(), "~", label)?,
                Some(array) => {
                    ids.bind(label, ('I', array.identity()));
                    let values = array.borrow_values();
                    compare_array(
                        &rows,
                        label,
                        "int",
                        &values.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    )?;
                }
            }
        }
        for field_kind in crate::organism_arrays::DOUBLE_FIELDS {
            let label = field(&rows, actor_label, field_kind.name());
            match actor.body.arrays.double(field_kind) {
                None => equal(field_kind.name(), "~", label)?,
                Some(array) => {
                    ids.bind(label, ('D', array.identity()));
                    let values = array.borrow_bits();
                    compare_array(
                        &rows,
                        label,
                        "double",
                        &values
                            .iter()
                            .map(|v| format!("{v:016x}"))
                            .collect::<Vec<_>>(),
                    )?;
                }
            }
        }
        let color_label = field(&rows, actor_label, "_segColor");
        ids.array(color_label, actor.body.phenotype_colors.as_ref());
        if let Some(array) = &actor.body.phenotype_colors {
            for (index, color) in array.slots().iter().enumerate() {
                let owner = element(&rows, color_label, index);
                match color {
                    None => equal("null cached Color", "~", owner)?,
                    Some(color) => {
                        ids.bind(owner, ('C', color.identity()));
                        equal(
                            "cached Color",
                            &format!("{:08x}", color.argb()),
                            data(&rows, owner),
                        )?;
                    }
                }
            }
        }
        if let Some(color) = &actor.state.temporary_color {
            let owner = field(&rows, actor_label, "_color");
            ids.bind(owner, ('C', color.identity()));
            equal(
                "temporary Color",
                &format!("{:08x}", color.argb()),
                data(&rows, owner),
            )?;
        }
        let v = rows.iter().find(|r| r[0] == "V").unwrap();
        match &self.returned {
            Returned::Null => {
                equal("return type", "null_or_void", v[2])?;
                equal("return", "~", v[3])?;
            }
            Returned::Count(count) => {
                equal("return type", "java.lang.Integer", v[2])?;
                equal("return count", &count.to_string(), v[3])?;
            }
            Returned::Gene(gene) => {
                equal("return type", "biogenesis.Gene", v[2])?;
                compare_gene(&rows, &mut ids, v[3], gene)?;
            }
            Returned::Code(code) => {
                equal("return type", "biogenesis.GeneticCode", v[2])?;
                compare_code(&rows, &mut ids, v[3], Some(code))?;
            }
        }
        let r = rows.iter().find(|r| r[0] == "R").unwrap();
        equal("Random", &random_wire(&self.world.random_snapshot()), r[3])?;
        let rp = rows.iter().find(|r| r[0] == "RP").unwrap();
        let mut peek = self.world.random_snapshot();
        equal("RP long", &peek.next_i64().to_string(), rp[2])?;
        equal("RP bound", &peek.bounded(10000).to_string(), rp[3])?;
        equal("RP double", &bits(peek.next_f64()), rp[4])?;
        Ok(())
    }
}
fn equal(name: &str, actual: &str, expected: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{name}: actual={actual} expected={expected}"))
    }
}
fn field<'a>(rows: &[&Row<'a>], owner: &str, name: &str) -> &'a str {
    rows.iter()
        .find(|r| r[0] == "F" && r[2] == owner && r[4] == name)
        .unwrap()[7]
}
fn element<'a>(rows: &[&Row<'a>], owner: &str, index: usize) -> &'a str {
    rows.iter()
        .find(|r| r[0] == "N" && r[2] == owner && r[3].parse::<usize>().unwrap() == index)
        .unwrap()[4]
}
fn data<'a>(rows: &[&Row<'a>], owner: &str) -> &'a str {
    rows.iter().find(|r| r[0] == "D" && r[2] == owner).unwrap()[4]
}
fn rectangle_fields(rows: &[&Row<'_>], owner: &str) -> String {
    ["x", "y", "width", "height"]
        .map(|name| field(rows, owner, name))
        .join(",")
}
fn projected(rows: &[&Row<'_>], name: &str, value: &str) -> String {
    if value == "~" || !value.starts_with('o') {
        return value.into();
    }
    if ["_sizeRect", "lastFrame"].contains(&name) {
        return rectangle_fields(rows, value);
    }
    if name == "_nPosition" {
        let hex = data(rows, value);
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        return String::from_utf8(bytes).unwrap();
    }
    if name == "_color" {
        return data(rows, value).into();
    }
    let array = rows
        .iter()
        .find(|r| r[0] == "A" && r[2] == value)
        .expect("Mapped primitive or Color array");
    (0..array[4].parse::<usize>().unwrap())
        .map(|index| {
            let v = element(rows, value, index);
            if name == "_segColor" && v != "~" {
                data(rows, v)
            } else {
                v
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}
fn compare_array(
    rows: &[&Row<'_>],
    label: &str,
    component: &str,
    actual: &[String],
) -> Result<(), String> {
    let a = rows.iter().find(|r| r[0] == "A" && r[2] == label).unwrap();
    equal("array component", component, a[3])?;
    equal("array length", &actual.len().to_string(), a[4])?;
    for (index, value) in actual.iter().enumerate() {
        equal("array value", value, element(rows, label, index))?;
    }
    Ok(())
}
fn compare_code(
    rows: &[&Row<'_>],
    ids: &mut Identities,
    label: &str,
    code: Option<&GenomeRef>,
) -> Result<(), String> {
    let Some(code) = code else {
        return equal("null genome", "~", label);
    };
    ids.bind(label, ('c', Rc::as_ptr(code) as usize));
    let code = code.borrow();
    for (name, value) in genome_fields(&code) {
        if name == "_cladeID" {
            let owner = field(rows, label, &name);
            match code.clade_id() {
                None => equal("clade", "~", owner)?,
                Some(text) => {
                    ids.bind(owner, ('t', text.units().as_ptr() as usize));
                    let hex = text
                        .to_utf8()
                        .unwrap()
                        .as_bytes()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>();
                    equal("clade bytes", &hex, data(rows, owner))?;
                }
            }
        } else {
            equal(&name, &value, field(rows, label, &name))?;
        }
    }
    let array_label = field(rows, label, "_genes");
    let Some(array) = code.gene_array() else {
        return equal("null Gene[]", "~", array_label);
    };
    ids.bind(array_label, ('j', array.identity()));
    let a = rows
        .iter()
        .find(|r| r[0] == "A" && r[2] == array_label)
        .unwrap();
    equal("Gene[] class", "biogenesis.Gene", a[3])?;
    equal("Gene[] length", &array.len().to_string(), a[4])?;
    for (index, gene) in array.slots().iter().enumerate() {
        let owner = element(rows, array_label, index);
        match gene {
            None => equal("null Gene", "~", owner)?,
            Some(gene) => compare_gene(rows, ids, owner, gene)?,
        }
    }
    Ok(())
}
fn compare_gene(
    rows: &[&Row<'_>],
    ids: &mut Identities,
    label: &str,
    gene: &crate::GeneRef,
) -> Result<(), String> {
    ids.bind(label, ('g', Rc::as_ptr(gene) as usize));
    let gene = gene.borrow();
    for (name, value) in gene_fields(&gene) {
        if name == "_color" {
            let owner = field(rows, label, &name);
            match gene.color_handle() {
                None => equal("null color", "~", owner)?,
                Some(color) => {
                    ids.bind(owner, ('C', color.identity()));
                    equal(
                        "Color ARGB",
                        &format!("{:08x}", color.argb()),
                        data(rows, owner),
                    )?;
                }
            }
        } else {
            equal(&name, &value, field(rows, label, &name))?;
        }
    }
    Ok(())
}
fn replay(source: &str, graphs: bool) -> Result<(usize, usize, usize), String> {
    let (mut count, mut calls_count, mut snapshots) = (0, 0, 0);
    for rows in cases(source) {
        let mut replay = Replay::new(&rows);
        replay.snapshot(&rows, "authored")?;
        snapshots += 1;
        if graphs {
            graph_fixture::verify(&rows, "authored");
        }
        for k in rows.iter().filter(|r| r[0] == "K") {
            let result = replay.call(&rows, k[1], k[2]);
            let x = rows.iter().find(|r| r[0] == "X" && r[1] == k[1]).unwrap();
            match (result, x[2]) {
                (Ok(()), "none") => {}
                (Err(GeneticsError::NullGeneArray), "java.lang.NullPointerException") => {}
                (
                    Err(GeneticsError::NullGene { index: 0 | 2 }),
                    "java.lang.NullPointerException",
                ) => {}
                (
                    Err(GeneticsError::GeneIndex { .. }),
                    "java.lang.ArrayIndexOutOfBoundsException",
                ) => {}
                (actual, expected) => panic!(
                    "{} {} actual={actual:?} expected={expected}",
                    rows[0][1], k[1]
                ),
            }
            replay.snapshot(&rows, k[1])?;
            if graphs {
                graph_fixture::verify(&rows, k[1]);
            }
            calls_count += 1;
            snapshots += 1;
        }
        count += 1;
    }
    Ok((count, calls_count, snapshots))
}
#[test]
fn actual_java_nullable_gene_arrays_match_inputs_failures_aliases_and_rng() {
    assert_eq!(replay(SOURCE, true).unwrap(), (8, 21, 29));
}
#[test]
fn nullable_gene_replay_rejects_an_output_only_bit_change() {
    let position = SOURCE
        .find("_length\tdouble\tfalse\t4028000000000000")
        .unwrap();
    let mut changed = SOURCE.to_string();
    let value = position + "_length\tdouble\tfalse\t".len();
    changed.replace_range(value..value + 16, "4028000000000001");
    let original_inputs: Vec<_> = SOURCE
        .lines()
        .filter(|r| {
            ["C\t", "U\t", "I\t", "IU\t", "K\t"]
                .iter()
                .any(|p| r.starts_with(p))
        })
        .collect();
    let changed_inputs: Vec<_> = changed
        .lines()
        .filter(|r| {
            ["C\t", "U\t", "I\t", "IU\t", "K\t"]
                .iter()
                .any(|p| r.starts_with(p))
        })
        .collect();
    assert_eq!(original_inputs, changed_inputs);
    let error = replay(&changed, false).unwrap_err();
    assert!(error.starts_with("_length: actual="), "{error}");
}

#[test]
fn nullable_gene_raw_create_preserves_actual_allocation_and_store_prefix() {
    let source = include_str!("../tests/fixtures/gene_array/raw_genes.tsv");
    let rows = cases(source).remove(0);
    let settings = rows
        .iter()
        .filter(|r| r[0] == "U")
        .map(|r| (r[1].to_string(), r[3].to_string()))
        .collect();
    let world = empty_reference_world(&settings, 827364);
    let clade = JavaText::from_utf8("world_publication_witness");
    let parent = fixed(&clade);
    let peer = fixed(&clade);
    let saved = fixed(&clade);
    let first = Rc::new(RefCell::new(
        world
            .construct_pasted_shared(Rc::clone(&parent), [100, 100])
            .unwrap()
            .unwrap(),
    ));
    first.borrow().state.bounds.bind_actor(&first).unwrap();
    let second = Rc::new(RefCell::new(
        world
            .construct_pasted_shared(Rc::clone(&peer), [400, 400])
            .unwrap()
            .unwrap(),
    ));
    second.borrow().state.bounds.bind_actor(&second).unwrap();
    let original = parent.borrow().gene_ref(0).unwrap();
    let shared = GeneArrayRef::from_slots(vec![
        Some(Rc::clone(&original)),
        Some(Rc::clone(&original)),
        None,
    ]);
    parent.borrow_mut().set_gene_array(Some(shared.clone()));
    peer.borrow_mut().set_gene_array(Some(shared));
    saved.borrow_mut().set_gene_array(None);
    {
        let mut actor = first.borrow_mut();
        actor.infected_genome = Some(Rc::clone(&peer));
        actor.saved_genome = Some(saved);
        actor.body.segment_count = -1;
        actor.body.arrays.start_radius = None;
        actor.body.arrays.x1 = Some(crate::IntArrayRef::from_values(vec![11]));
    }
    world.organisms.borrow_mut().extend([Rc::clone(&first), second]);
    let mut replay = Replay {
        world,
        actor: first,
        parent,
        peer: Some(peer),
        original,
        replacement: Gene::default().shared(),
        returned: Returned::Null,
    };
    replay.snapshot(&rows, "authored").unwrap();
    let count = replay.parent.borrow().gene_count().unwrap();
    assert_eq!(count, 3);
    replay.returned = Returned::Count(count);
    replay.snapshot(&rows, "call_1").unwrap();
    assert_eq!(
        replay
            .actor
            .borrow()
            .saved_genome
            .as_ref()
            .unwrap()
            .borrow()
            .gene_count(),
        Err(GeneticsError::NullGeneArray)
    );
    replay.returned = Returned::Null;
    replay.snapshot(&rows, "call_2").unwrap();
    let previous: Vec<_> = crate::organism_arrays::INT_FIELDS
        .iter()
        .filter(|f| !matches!(f, crate::IntField::Reaction(_)))
        .map(|f| (*f, replay.actor.borrow().body.arrays.int(*f).clone()))
        .collect();
    let previous_double: Vec<_> = crate::organism_arrays::DOUBLE_FIELDS
        .iter()
        .map(|f| (*f, replay.actor.borrow().body.arrays.double(*f).clone()))
        .collect();
    let result = replay.actor.borrow_mut().create_for_paste();
    assert_eq!(
        result,
        Err(GeometryError::Genetics(GeneticsError::NullGene {
            index: 2
        }))
    );
    assert_eq!(replay.actor.borrow().body.segment_count, 3);
    for (field, old) in previous {
        let actor = replay.actor.borrow();
        let new = actor.body.arrays.int(field);
        assert!(
            old.as_ref()
                .zip(new.as_ref())
                .is_some_and(|(a, b)| a.same_instance(b))
        );
    }
    for (field, old) in previous_double {
        let actor = replay.actor.borrow();
        let new = actor.body.arrays.double(field);
        match (old, new) {
            (Some(a), Some(b)) => assert!(a.same_instance(b)),
            (None, None) => {}
            _ => panic!("Old geometry array changed"),
        }
    }
    replay.snapshot(&rows, "call_3").unwrap();
    for stage in ["authored", "call_0", "call_1", "call_2", "call_3"] {
        graph_fixture::verify(&rows, stage);
    }
}

#[test]
fn revive_nonpositive_segments_preserve_reset_prefix_before_late_genotype_failure() {
    for segments in [-1, 0, 1] {
        for missing_primary in [false, true] {
            let rows = cases(SOURCE).remove(0);
            let replay = Replay::new(&rows);
            if missing_primary {
                replay.actor.borrow_mut().genome = None;
            }
            {
                let mut actor = replay.actor.borrow_mut();
                actor.body.segment_count = segments;
                actor.body.phenotype_colors = None;
                actor.state.alive = false;
                actor.state.active = false;
                actor.state.has_moved = false;
                actor.state.age = 73;
                actor.effects.is_plant = true;
                actor.effects.is_consumer = true;
                actor.effects.has_dodged = false;
                actor.effects.is_injured = true;
            }
            let before_population = replay.world.population();
            let before_random = replay.world.random_snapshot().state48();
            let handle = super::OrganismHandle(Rc::clone(&replay.actor), replay.world.handle());
            let expected = if missing_primary {
                GeometryError::MissingGenome
            } else {
                GeometryError::Genetics(GeneticsError::NullGeneArray)
            };
            assert_eq!(
                replay
                    .world
                    .act_on_organism(&handle, super::OrganismAction::Revive),
                Err(WorldError::Geometry(expected))
            );
            let actor = replay.actor.borrow();
            if segments <= 0 {
                assert!(actor.state.alive && actor.state.active && actor.state.has_moved);
                assert_eq!(actor.state.age, 1);
                assert!(
                    !actor.effects.is_plant
                        && !actor.effects.is_consumer
                        && !actor.effects.is_injured
                );
                assert!(actor.effects.has_dodged);
                assert_eq!(actor.body.update_effects, 2);
            } else {
                assert!(!actor.state.alive && !actor.state.active && !actor.state.has_moved);
                assert_eq!(actor.state.age, 73);
                assert!(
                    actor.effects.is_plant && actor.effects.is_consumer && actor.effects.is_injured
                );
            }
            assert!(actor.body.phenotype_colors.is_none());
            assert_eq!(replay.world.population(), before_population);
            assert_eq!(replay.world.random_snapshot().state48(), before_random);
        }
    }
}
