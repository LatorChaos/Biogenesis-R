// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay captured primitive-array calls from authored constructors and ordered inputs

use super::death_oracle_tests::{apply_reference_input, statistics_schemas};
use super::phenotype_color_oracle_tests::{Identities, cases, phenotype_color_graph_fixture};
use super::*;
use crate::organism::{GenomeRef, OrganismRef};
use crate::statistics::tests::{assert_observation, observed_field};
use crate::{ColorArrayRef, ColorKind, DoubleArrayRef, IntArrayRef, JavaColor, JavaText};
use std::{cell::RefCell, rc::Rc};

const SOURCE: &str = include_str!("../tests/fixtures/organism-array/organism_array.tsv");
const DIRECTORY: &str = "crates/core/tests/fixtures/organism-array/";
type Row<'a> = Vec<&'a str>;

struct Replay {
    name: String,
    world: World,
    actors: BTreeMap<String, OrganismRef>,
    genomes: BTreeMap<String, GenomeRef>,
    colors: BTreeMap<String, JavaColor>,
    color_arrays: BTreeMap<String, ColorArrayRef>,
    ints: BTreeMap<String, IntArrayRef>,
    doubles: BTreeMap<String, DoubleArrayRef>,
    identities: Identities,
    clade: JavaText,
    arguments: Option<[usize; 2]>,
    last_call: Option<(String, String)>,
    outcome: Option<Result<(), WorldError>>,
    errors: Vec<String>,
    calls: usize,
    snapshots: usize,
    primitive_observations: usize,
}

impl Replay {
    fn new(rows: &[Row<'_>]) -> Self {
        let settings: Fields = rows
            .iter()
            .filter(|r| r[0] == "U")
            .map(|r| (r[1].into(), r[3].into()))
            .collect();
        assert_eq!(settings.len(), 147);
        let world = empty_reference_world(&settings, rows[0][3].parse().unwrap());
        let mut colors = BTreeMap::new();
        let mut identities = Identities::default();
        for (label, kind) in [
            ("fixed_green", ColorKind::Green),
            ("fixed_red", ColorKind::Red),
            ("fixed_dark", ColorKind::Dark),
            ("fixed_eye", ColorKind::Eye),
            ("fixed_brown", ColorKind::Brown),
            ("fixed_vision", ColorKind::Vision),
        ] {
            let color = kind.color_ref().unwrap();
            identities.color(&format!("{label}:{:08x}", color.argb()), Some(&color));
            colors.insert(label.into(), color);
        }
        Self {
            name: rows[0][1].into(),
            world,
            actors: BTreeMap::new(),
            genomes: BTreeMap::new(),
            colors,
            color_arrays: BTreeMap::new(),
            ints: BTreeMap::new(),
            doubles: BTreeMap::new(),
            identities,
            clade: JavaText::from_utf8("array_witness"),
            arguments: None,
            last_call: None,
            outcome: None,
            errors: Vec::new(),
            calls: 0,
            snapshots: 0,
            primitive_observations: 0,
        }
    }

    fn genome(&mut self, row: &[&str]) {
        assert_eq!(row.len(), 3);
        let count: usize = row[2].parse().unwrap();
        assert!(count <= 3);
        let genes: Vec<_> = (0..count)
            .map(|_| {
                Gene::new_with_color(12.0, 0.0, ColorKind::Green.color_ref(), 0, [0; 36]).shared()
            })
            .collect();
        let mut genome = Genome::from_parts(
            &genes,
            GenomeProperties {
                symmetry: 1,
                mirror: 0,
                mutation_rate: 0,
                activity: 0,
                modifiers: GenomeModifiers {
                    cream: 1,
                    fallow: 1,
                    spore: 1,
                    adapt_spore: 1,
                    plague: false,
                    disperse_children: false,
                    generation_battle: false,
                    sibling_battle: false,
                    altruist: false,
                    familial: false,
                    social: false,
                    peaceful: false,
                    passive: false,
                    clockwise: false,
                    mimic_all: false,
                    modifies_pink: false,
                    modifies_lilac: false,
                    modifies_sky: false,
                    selfish: false,
                },
            },
        );
        genome.set_generation(41);
        genome.set_clade_text(Some(self.clade.clone()));
        genome.set_update_clade(false);
        assert!(
            self.genomes
                .insert(row[1].into(), Rc::new(RefCell::new(genome)))
                .is_none()
        );
    }

    fn construct(&mut self, row: &[&str]) {
        let genome = Rc::clone(&self.genomes[row[2]]);
        let actor = self
            .world
            .construct_pasted_shared(genome, [row[3].parse().unwrap(), row[4].parse().unwrap()])
            .unwrap()
            .unwrap();
        let actor = Rc::new(RefCell::new(actor));
        self.world.add_organism(Rc::clone(&actor), None);
        assert!(self.actors.insert(row[1].into(), actor).is_none());
    }

    fn input_color(&self, value: &str) -> Option<JavaColor> {
        if value == "~" {
            return None;
        }
        let (label, argb) = value.split_once(':').unwrap();
        let color = self.colors[label].clone();
        assert_eq!(color.argb(), u32::from_str_radix(argb, 16).unwrap());
        Some(color)
    }

    fn primitive_input(&mut self, row: &[&str]) {
        let length: usize = row[3].parse().unwrap();
        assert!(length <= 16);
        let values: Vec<_> = if row[4].is_empty() {
            Vec::new()
        } else {
            row[4].split(',').collect()
        };
        assert_eq!(values.len(), length);
        match row[2] {
            "int[]" => {
                let owner =
                    IntArrayRef::from_values(values.iter().map(|v| v.parse().unwrap()).collect());
                self.identities.ints(row[1], Some(&owner));
                assert!(self.ints.insert(row[1].into(), owner).is_none());
            }
            "double[]" => {
                let owner = DoubleArrayRef::from_bits(
                    values
                        .iter()
                        .map(|v| u64::from_str_radix(v, 16).unwrap())
                        .collect(),
                );
                self.identities.doubles(row[1], Some(&owner));
                assert!(self.doubles.insert(row[1].into(), owner).is_none());
            }
            kind => panic!("Unknown primitive input {kind}"),
        }
    }

    fn color_array_input(&mut self, row: &[&str]) {
        assert_eq!(row[2], "Color[]");
        let length: usize = row[3].parse().unwrap();
        assert!(length <= 16);
        let slots = if row[4].is_empty() {
            Vec::new()
        } else {
            row[4]
                .split(',')
                .map(|value| self.input_color(value))
                .collect()
        };
        let owner = ColorArrayRef::from_slots(slots);
        assert_eq!(owner.len(), length);
        self.identities.array(row[1], Some(&owner));
        assert!(self.color_arrays.insert(row[1].into(), owner).is_none());
    }

    fn input(&mut self, row: &[&str]) {
        let (target, kind, value) = (row[1], row[2], row[3]);
        let parts: Vec<_> = target.split(':').collect();
        match parts.as_slice() {
            ["actor", role, name] if kind == "int[]" => {
                let field = crate::organism_arrays::INT_FIELDS
                    .into_iter()
                    .find(|field| field.name() == *name)
                    .expect("Declared int field");
                let owner = (value != "~").then(|| self.ints[value].clone());
                *self.actors[*role].borrow_mut().body.arrays.int_mut(field) = owner;
            }
            ["actor", role, name] if kind == "double[]" => {
                let field = crate::organism_arrays::DOUBLE_FIELDS
                    .into_iter()
                    .find(|field| field.name() == *name)
                    .expect("Declared double field");
                let owner = (value != "~").then(|| self.doubles[value].clone());
                *self.actors[*role]
                    .borrow_mut()
                    .body
                    .arrays
                    .double_mut(field) = owner;
            }
            ["actor", role, "_segColor"] => {
                assert_eq!(kind, "Color[]");
                self.actors[*role].borrow_mut().body.phenotype_colors =
                    (value != "~").then(|| self.color_arrays[value].clone());
            }
            ["actor", role, "_geneticCode"] => {
                assert_eq!(kind, "GeneticCode");
                self.actors[*role].borrow_mut().genome = Some(Rc::clone(&self.genomes[value]));
            }
            ["actor", role, "_segments"] => {
                assert_eq!(kind, "int");
                self.actors[*role].borrow_mut().body.segment_count = value.parse().unwrap();
            }
            ["actor", role, "_theta"] => {
                assert_eq!(kind, "double");
                self.actors[*role].borrow_mut().state.angle = decoded(value);
            }
            ["actor", role, "_lastTheta"] => {
                assert_eq!(kind, "double");
                self.actors[*role].borrow_mut().state.last_angle = decoded(value);
            }
            ["gene", label, index, "_redreaction"] => {
                assert_eq!(kind, "int");
                self.genomes[*label].borrow().genes().unwrap()[index.parse::<usize>().unwrap()]
                    .borrow_mut()
                    .set_reaction(0, value.parse().unwrap());
            }
            ["gene", label, index, "_color"] => {
                assert_eq!(kind, "Color");
                let color = self.input_color(value);
                self.genomes[*label].borrow().genes().unwrap()[index.parse::<usize>().unwrap()]
                    .borrow_mut()
                    .set_color_handle(color);
            }
            ["genome", label, "_mimicall"] => {
                assert_eq!(kind, "boolean");
                let mut genome = self.genomes[*label].borrow_mut();
                let mut properties = genome.properties();
                properties.modifiers.mimic_all = value.parse().unwrap();
                genome.set_properties(properties);
            }
            ["pretouch_args" | "reaction_args"] => {
                assert_eq!(kind, "String");
                let args: Vec<_> = value.split(',').collect();
                assert_eq!(args[0], "killer");
                self.arguments = Some([args[1].parse().unwrap(), args[2].parse().unwrap()]);
            }
            _ => apply_reference_input(
                &self.name,
                &BTreeMap::new(),
                target,
                kind,
                value,
                &mut self.world,
                &self.actors,
                &mut self.genomes,
                &mut self.errors,
            ),
        }
    }

    fn call(&mut self, row: &[&str]) {
        assert_eq!(row[3], "victim");
        let actor = Rc::clone(&self.actors[row[3]]);
        let peer = Rc::clone(&self.actors["killer"]);
        let result = match row[2] {
            "create" => {
                let genome = Rc::clone(actor.borrow().genome.as_ref().unwrap());
                let result = actor.borrow_mut().body.create_arrays(&genome.borrow());
                result.map_err(WorldError::Geometry)
            }
            "calculateBounds_false" | "calculateBounds_true" => {
                crate::movement::calculate_bounds(
                    &mut actor.borrow_mut(),
                    row[2] == "calculateBounds_true",
                )
                .map_err(WorldError::Geometry)
            }
            operation => {
                let [segment, peer_segment] = self.arguments.expect("Declared call arguments");
                match operation {
                    "pretouchEffects" => self.world.pre_touch_effects(
                        &mut actor.borrow_mut(),
                        &mut peer.borrow_mut(),
                        segment,
                        peer_segment,
                    ),
                    "touchReaction" => self.world.touch_reaction(
                        &mut actor.borrow_mut(),
                        &peer.borrow(),
                        segment,
                        peer_segment,
                    ),
                    "touchfriendReaction" => self.world.touch_friend_reaction(
                        &mut actor.borrow_mut(),
                        &peer.borrow(),
                        segment,
                        peer_segment,
                    ),
                    _ => panic!("Unknown captured operation {operation}"),
                }
            }
        };
        self.last_call = Some((row[1].into(), row[2].into()));
        self.outcome = Some(result);
        self.calls += 1;
    }

    fn outcome(&self, row: &[&str]) {
        use crate::GeometryError::*;
        let (stage, operation) = self.last_call.as_ref().expect("Call precedes X");
        assert_eq!(stage, row[1]);
        let result = self.outcome.as_ref().unwrap();
        if row[2] == "none" {
            assert_eq!(row.len(), 3);
            assert!(matches!(
                (self.name.as_str(), operation.as_str()),
                ("array_bounds_bypass_prefix", "calculateBounds_false")
                    | (
                        "array_signed_count",
                        "touchReaction" | "touchfriendReaction"
                    )
            ));
            assert_eq!(result, &Ok(()), "{} {stage}", self.name);
            return;
        }
        let expected = match (self.name.as_str(), operation.as_str()) {
            ("array_create_prefix", "create") => MissingColor { gene: 1 },
            ("array_bounds_bypass_prefix", "calculateBounds_true") => {
                MissingPrimitiveArray { field: "_m2" }
            }
            ("array_mimic_channel_alias", "pretouchEffects") => MissingPrimitiveArray {
                field: "_segblueReaction",
            },
            ("array_signed_count", "touchReaction" | "touchfriendReaction") => ColorIndex {
                segment: i32::MAX as usize,
                length: 1,
            },
            _ => panic!("Unexpected captured exception"),
        };
        assert_eq!(
            result,
            &Err(WorldError::Geometry(expected)),
            "{} {stage}",
            self.name
        );
        let class = if matches!(expected, ColorIndex { .. }) {
            "java.lang.ArrayIndexOutOfBoundsException"
        } else {
            "java.lang.NullPointerException"
        };
        assert_eq!(row[2], class);
        assert_eq!(row[3], "biogenesis.Organism");
        assert_eq!(row[4], operation.split('_').next().unwrap());
        assert!(row[5].parse::<u32>().unwrap() > 0);
    }
    fn snapshot(&mut self, rows: &[Row<'_>]) {
        let stage = rows[0][1];
        assert_eq!(rows[0][0], "P");
        for role in ["selector", "victim", "killer"] {
            let actor = self.actors[role].borrow();
            let expected: Fields = rows
                .iter()
                .filter(|r| r[0] == "O" && r[2] == role)
                .map(|r| (r[3].into(), r[4].into()))
                .collect();
            assert_eq!(
                expected.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                [
                    "_dCenterX",
                    "_dCenterY",
                    "_lastTheta",
                    "_theta",
                    "_I",
                    "_age",
                    "_canmove",
                    "_canreact",
                    "_centerX",
                    "_centerY",
                    "_colonyPhotosynthesis",
                    "_color",
                    "_endPointX",
                    "_endPointY",
                    "_energy",
                    "_forestphoto",
                    "_framesColor",
                    "_growthRatio",
                    "_haseyes",
                    "_hasgoodvision",
                    "_infectedGeneticCode",
                    "_m",
                    "_m1",
                    "_m2",
                    "_mass",
                    "_methanotrophy",
                    "_mphoto",
                    "_nTotalKills",
                    "_photosynthesis",
                    "_savedGeneticCode",
                    "_segColor",
                    "_segmentpretoucheffects",
                    "_segments",
                    "_sizeRect",
                    "_startPointX",
                    "_startPointY",
                    "_updateEffects",
                    "_useframemovement",
                    "_usefriendeffects",
                    "_usepretoucheffects",
                    "active",
                    "alive",
                    "dtheta",
                    "dx",
                    "dy",
                    "hasMoved",
                    "x1",
                    "x2",
                    "y1",
                    "y2"
                ]
                .into_iter()
                .collect()
            );
            let ordinary: Fields = expected
                .iter()
                .filter(|(key, _)| {
                    ![
                        "_color",
                        "_segColor",
                        "_infectedGeneticCode",
                        "_savedGeneticCode",
                    ]
                    .contains(&key.as_str())
                })
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            let all = organism_fields(&View::from(&*actor));
            let actual: Fields = ordinary
                .keys()
                .map(|key| (key.clone(), all[key].clone()))
                .collect();
            compare_fields(&self.name, stage, "O", &actual, &ordinary, &mut self.errors);
            self.identities
                .color(&expected["_color"], actor.state.temporary_color.as_ref());
            self.identities
                .array(&expected["_segColor"], actor.body.phenotype_colors.as_ref());
            for (field, actual) in [
                ("_infectedGeneticCode", &actor.infected_genome),
                ("_savedGeneticCode", &actor.saved_genome),
            ] {
                compare_genome_reference(actual.as_ref(), &expected[field], &self.genomes);
            }
            let q: Vec<_> = rows
                .iter()
                .filter(|r| r[0] == "Q" && r[2] == role)
                .collect();
            assert_eq!(q.len(), 36);
            let q_fields: BTreeSet<_> = q.iter().map(|row| row[3]).collect();
            assert_eq!(
                q_fields,
                crate::organism_arrays::REACTION_FIELDS
                    .into_iter()
                    .collect()
            );
            for row in q {
                let field = crate::organism_arrays::INT_FIELDS
                    .into_iter()
                    .find(|field| field.name() == row[3])
                    .unwrap();
                assert_eq!(int_array(&View::from(&*actor), field), row[4]);
            }
            let pa: Vec<_> = rows
                .iter()
                .filter(|r| r[0] == "PA" && r[2] == role)
                .collect();
            assert_eq!(pa.len(), 48);
            assert_eq!(
                pa.iter().map(|row| row[3]).collect::<BTreeSet<_>>().len(),
                48
            );
            for row in pa {
                if row[4] == "int[]" {
                    let field = crate::organism_arrays::INT_FIELDS
                        .into_iter()
                        .find(|field| field.name() == row[3])
                        .unwrap();
                    let owner = actor.body.arrays.int(field).as_ref();
                    self.identities.ints(row[5], owner);
                    if let Some(owner) = owner {
                        assert_eq!(owner.len(), row[6].parse::<usize>().unwrap());
                    } else {
                        assert_eq!(row[6], "~");
                    }
                    let kind = if matches!(field, crate::IntField::Reaction(_)) {
                        "Q"
                    } else {
                        "O"
                    };
                    assert_eq!(
                        rows.iter()
                            .filter(|payload| payload[0] == kind
                                && payload[2] == role
                                && payload[3] == row[3])
                            .count(),
                        1
                    );
                } else {
                    assert_eq!(row[4], "double[]");
                    let field = crate::organism_arrays::DOUBLE_FIELDS
                        .into_iter()
                        .find(|field| field.name() == row[3])
                        .unwrap();
                    let owner = actor.body.arrays.double(field).as_ref();
                    self.identities.doubles(row[5], owner);
                    if let Some(owner) = owner {
                        assert_eq!(owner.len(), row[6].parse::<usize>().unwrap());
                    } else {
                        assert_eq!(row[6], "~");
                    }
                    assert_eq!(
                        rows.iter()
                            .filter(|payload| payload[0] == "O"
                                && payload[2] == role
                                && payload[3] == row[3])
                            .count(),
                        1
                    );
                }
                self.primitive_observations += 1;
            }
            let genome = actor.genome.as_ref().unwrap().borrow();
            let gene_rows: Vec<_> = rows
                .iter()
                .filter(|r| r[0] == "N" && r[2] == role)
                .collect();
            assert_eq!(gene_rows.len(), genome.genes().unwrap().len());
            for row in gene_rows {
                self.identities
                    .color(row[4], genome.gene_ref(row[3].parse().unwrap()).unwrap().borrow().color_handle());
            }
            let array_row = rows.iter().find(|r| r[0] == "AR" && r[2] == role).unwrap();
            self.identities
                .array(array_row[3], actor.body.phenotype_colors.as_ref());
            let slots: Vec<_> = rows
                .iter()
                .filter(|r| r[0] == "AC" && r[2] == role)
                .collect();
            if let Some(array) = &actor.body.phenotype_colors {
                assert_eq!(array.len(), array_row[4].parse::<usize>().unwrap());
                assert_eq!(slots.len(), array.len());
                for row in slots {
                    let value = array.get(row[3].parse().unwrap()).unwrap();
                    let expected = if row[4] == "~" {
                        "~".into()
                    } else {
                        format!("{}:{}", row[4], row[5])
                    };
                    self.identities.color(&expected, value.as_ref());
                }
            } else {
                assert_eq!(array_row[4], "~");
                assert!(slots.is_empty());
            }
            let g = rows.iter().find(|r| r[0] == "G" && r[2] == role).unwrap();
            assert!(Rc::ptr_eq(
                actor.genome.as_ref().unwrap(),
                &self.genomes[g[3]]
            ));
            let gm = rows.iter().find(|r| r[0] == "GM" && r[2] == role).unwrap();
            assert_eq!(genome.modifiers().mimic_all, gm[3].parse::<bool>().unwrap());
            let b = rows.iter().find(|r| r[0] == "B" && r[2] == role).unwrap();
            assert_eq!(rectangle(actor.state.bounds.get()), b[3..7].join(","));
        }
        let expected: Fields = rows
            .iter()
            .filter(|r| r[0] == "W")
            .map(|r| (r[2].into(), r[3].into()))
            .collect();
        assert_eq!(expected.len(), 8);
        let mut all = world_fields(&self.world);
        all.insert("nFrames".into(), self.world.frame.get().to_string());
        let actual = expected
            .keys()
            .map(|key| (key.clone(), all[key].clone()))
            .collect();
        compare_fields(&self.name, stage, "W", &actual, &expected, &mut self.errors);
        let member = rows.iter().find(|r| r[0] == "M").unwrap();
        let actual = self
            .world.organisms.borrow()
            .iter()
            .map(|reference| {
                self.actors
                    .iter()
                    .find_map(|(role, actor)| Rc::ptr_eq(reference, actor).then_some(role.as_str()))
                    .unwrap()
            })
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(actual, member[2]);
        let (types, _) = statistics_schemas();
        let statistics_owner = self.world.statistics_ref();
        let statistics = statistics_owner.borrow();
        let s: Vec<_> = rows.iter().filter(|r| r[0] == "S").collect();
        assert_eq!(s.len(), 52);
        for row in s {
            assert_observation(
                observed_field(&statistics, row[2]),
                &types[row[2]],
                row[3],
                &self.genomes,
                &format!("{} {stage} {}", self.name, row[2]),
            );
        }
        let rp = rows.iter().find(|r| r[0] == "RP").unwrap();
        let mut random = self.world.random_snapshot();
        assert_eq!(random.next_i64(), rp[2].parse::<i64>().unwrap());
        assert_eq!(random.bounded(10_000), rp[3].parse::<i32>().unwrap());
        assert_eq!(bits(random.next_f64()), rp[4]);
        // R retains the Java serialized-Random provenance; RP checks the model RNG without consuming it
        let r = rows.iter().find(|r| r[0] == "R").unwrap();
        assert_eq!(
            r[2],
            "55b2a1d6ca0c49501893ce8d7eefb5efb1718defdab6f19beea2e3e5e7dc682e"
        );
        self.snapshots += 1;
    }
}

fn compare_genome_reference(
    actual: Option<&GenomeRef>,
    expected: &str,
    genomes: &BTreeMap<String, GenomeRef>,
) {
    if expected == "~" {
        assert!(actual.is_none());
    } else {
        assert!(Rc::ptr_eq(actual.unwrap(), &genomes[expected]));
    }
}

fn replay(rows: &[Row<'_>]) -> Replay {
    let mut replay = Replay::new(rows);
    let mut snapshot_start = None;
    for (index, row) in rows.iter().enumerate() {
        match row[0] {
            "GI" => replay.genome(row),
            "A" => replay.construct(row),
            "IA" => replay.primitive_input(row),
            "IC" => replay.color_array_input(row),
            "I" => replay.input(row),
            "K" => replay.call(row),
            "X" => replay.outcome(row),
            "P" => assert!(snapshot_start.replace(index).is_none()),
            "Z" => {
                let start = snapshot_start.take().unwrap();
                replay.snapshot(&rows[start..=index]);
            }
            "C" | "U" | "O" | "AR" | "AC" | "PA" | "Q" | "GM" | "G" | "N" | "B" | "W" | "M"
            | "S" | "R" | "RP" | "E" => {}
            tag => panic!("Unknown array witness row {tag}"),
        }
    }
    assert!(snapshot_start.is_none());
    replay
}

#[test]
fn actual_java_primitive_array_calls_preserve_ordered_prefixes_aliases_count_and_rng() {
    let cases = cases(SOURCE);
    assert_eq!(SOURCE.lines().count(), 6531);
    assert_eq!(cases.len(), 4);
    let mut totals = [0; 3];
    for rows in cases {
        let result = replay(&rows);
        assert!(result.errors.is_empty(), "{}", result.errors.join("\n"));
        totals[0] += result.calls;
        totals[1] += result.snapshots;
        totals[2] += result.primitive_observations;
    }
    assert_eq!(totals, [8, 12, 1728]);
    phenotype_color_graph_fixture::verify_captured_graph_fixtures(SOURCE, DIRECTORY, 12, 41, 1728);
}

#[test]
fn primitive_array_checker_rejects_changed_output_energy_without_changing_inputs() {
    let mut replaced = false;
    let changed = SOURCE
        .lines()
        .map(|line| {
            if !replaced && line.starts_with("O\tinitial\tvictim\t_energy\t") {
                replaced = true;
                let (prefix, value) = line.rsplit_once('\t').unwrap();
                assert_eq!(value.len(), 16);
                format!(
                    "{prefix}\t{:016x}",
                    u64::from_str_radix(value, 16).unwrap() ^ 1
                )
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(replaced);
    let inputs = |source: &str| {
        source
            .lines()
            .filter(|line| {
                ["C", "U", "GI", "A", "IA", "IC", "I", "K"]
                    .contains(&line.split('\t').next().unwrap())
            })
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(inputs(SOURCE), inputs(&changed));
    assert_eq!(
        SOURCE
            .lines()
            .zip(changed.lines())
            .filter(|(a, b)| a != b)
            .count(),
        1
    );
    let result = replay(&cases(&changed).remove(0));
    assert_eq!(result.errors.len(), 1, "{}", result.errors.join("\n"));
    assert!(result.errors[0].starts_with("array_create_prefix\tinitial\tO\t_energy\t"));
}
