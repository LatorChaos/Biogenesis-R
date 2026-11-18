// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay captured Rectangle calls from authored constructors and ordered inputs

use super::death_oracle_tests::{apply_reference_input, statistics_schemas};
use super::phenotype_color_oracle_tests::{Identities, cases};
#[path = "rectangle_graph_fixture.rs"]
mod rectangle_graph_fixture;
use super::*;
use crate::organism::{GenomeRef, OrganismRef};
use crate::statistics::tests::{assert_observation, observed_field};
use crate::{ColorArrayRef, ColorKind, DoubleArrayRef, IntArrayRef, JavaColor, JavaText};
use std::{cell::RefCell, rc::Rc};

const SOURCE: &str = include_str!("../tests/fixtures/rectangle-ownership/rectangle_ownership.tsv");
const DIRECTORY: &str = "crates/core/tests/fixtures/rectangle-ownership/";
type Row<'a> = Vec<&'a str>;

#[derive(Debug, PartialEq)]
enum CallValue {
    Void,
    Boolean(bool),
}

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
    outcome: Option<Result<CallValue, WorldError>>,
    rectangles: BTreeMap<String, crate::RectangleRef>,
    observed_rectangles: Vec<crate::RectangleRef>,
    rectangle_observations: usize,
    graph_counts: [usize; 5],
    typed_graphs: usize,
    typed_rectangles: usize,
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
            clade: JavaText::from_utf8("rectangle_witness"),
            arguments: None,
            last_call: None,
            outcome: None,
            rectangles: BTreeMap::new(),
            observed_rectangles: Vec::new(),
            rectangle_observations: 0,
            graph_counts: [0; 5],
            typed_graphs: 0,
            typed_rectangles: 0,
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
        let bounds = actor.borrow().state.bounds.clone();
        let label = format!("actor_{}", row[1]);
        self.identities.bind(&label, ('R', bounds.identity()));
        assert!(self.rectangles.insert(label, bounds).is_none());
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

    fn rectangle_input(&mut self, row: &[&str]) {
        assert_eq!(row[2], "java.awt.Rectangle");
        let values: Vec<i32> = row[3..7]
            .iter()
            .map(|value| value.parse().unwrap())
            .collect();
        let cell = crate::RectangleRef::new(IntRect {
            x: values[0],
            y: values[1],
            width: values[2],
            height: values[3],
        });
        self.identities.bind(row[1], ('R', cell.identity()));
        assert!(self.rectangles.insert(row[1].into(), cell).is_none());
    }
    fn world_input(&mut self, row: &[&str]) {
        match &row[1..] {
            ["world:_organisms", "remove_identity", role] => {
                let target = &self.actors[*role];
                self.world.organisms.borrow_mut()
                    .retain(|actor| !Rc::ptr_eq(actor, target));
            }
            ["world:colDetTree", "STRtree", "fresh_empty"] => {
                self.world.frame_objects.borrow_mut().clear();
                *self.world.spatial.borrow_mut() = crate::spatial::SpatialTree::build([]);
            }
            _ => panic!("Unknown Rectangle World input"),
        }
    }
    fn input(&mut self, row: &[&str]) {
        let (target, kind, value) = (row[1], row[2], row[3]);
        let parts: Vec<_> = target.split(':').collect();
        match parts.as_slice() {
            ["actor", role, field @ ("_sizeRect" | "lastFrame")] => {
                assert_eq!(kind, "Rectangle");
                let reference = (value != "~").then(|| self.rectangles[value].clone());
                let mut actor = self.actors[*role].borrow_mut();
                if *field == "_sizeRect" {
                    actor.body.size_rect = reference;
                } else {
                    actor.state.last_frame = reference;
                }
            }
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
        let result = match row[2] {
            "symmetric" => actor
                .borrow_mut()
                .rebuild_body()
                .map(|_| CallValue::Void)
                .map_err(WorldError::Geometry),
            "move" => self
                .world
                .move_actor_frame(&actor, &mut actor.borrow_mut())
                .map(CallValue::Boolean),
            operation => panic!("Unknown captured Rectangle operation {operation}"),
        };
        self.last_call = Some((row[1].into(), row[2].into()));
        self.outcome = Some(result);
        self.calls += 1;
    }

    fn returned(&self, row: &[&str]) {
        let actual = self.outcome.as_ref().unwrap();
        match row[2] {
            "void" => assert_eq!(actual, &Ok(CallValue::Void)),
            "boolean" => assert_eq!(actual, &Ok(CallValue::Boolean(row[3].parse().unwrap()))),
            _ => panic!("Captured return kind"),
        }
    }
    fn outcome(&self, row: &[&str]) {
        let (stage, operation) = self.last_call.as_ref().unwrap();
        assert_eq!(stage, row[1]);
        let actual = self.outcome.as_ref().unwrap();
        if row[2] == "none" {
            assert!(actual.is_ok());
            return;
        }
        let expected = match self.name.as_str() {
            "rect_size_null" => GeometryError::MissingRectangle { field: "_sizeRect" },
            "rect_last_null" => GeometryError::MissingRectangle { field: "lastFrame" },
            "rect_after_set_prefix" => GeometryError::MissingPrimitiveArray { field: "_m1" },
            _ => panic!("Captured Rectangle failure"),
        };
        assert_eq!(actual, &Err(WorldError::Geometry(expected)));
        assert_eq!(row[2], "java.lang.NullPointerException");
        assert_eq!(row[3], "biogenesis.Organism");
        assert_eq!(row[4], operation);
        assert!(row[5].parse::<i32>().unwrap() >= -2);
    }
    fn rectangle_observation(
        &mut self,
        stage: &str,
        role: &str,
        row: &[&str],
        cell: Option<&crate::RectangleRef>,
    ) {
        match cell {
            None => assert_eq!(&row[4..], &["~", "~", "~", "~", "~", "~"]),
            Some(cell) => {
                assert_ne!(row[4], "~");
                let input_matches = self
                    .rectangles
                    .get(row[4])
                    .is_none_or(|input| input.same_instance(cell));
                if input_matches {
                    self.identities.bind(row[4], ('R', cell.identity()));
                } else {
                    self.errors.push(format!(
                        "{}\t{stage}\tRA\t{role}\t{}\towner mismatch",
                        self.name, row[3]
                    ));
                }
                assert_eq!(cell.class_name(), row[5]);
                assert_eq!(rectangle(cell.get()), row[6..10].join(","));
                self.observed_rectangles.push(cell.clone());
            }
        }
        self.rectangle_observations += 1;
    }
    fn typed_graph_snapshot(&mut self, rows: &[Row<'_>]) {
        use crate::legacy_graph::LegacyGraph;
        use crate::object_stream::{StreamReader, Value};
        let z = rows.iter().find(|row| row[0] == "Z").unwrap();
        let path = std::path::Path::new(z[2]);
        assert_eq!(path.components().count(), 1);
        let bytes = std::fs::read(crate::oracle_tests::fixture_path(DIRECTORY).join(path)).unwrap();
        assert_eq!(bytes.len(), z[3].parse::<usize>().unwrap());
        let mut document = StreamReader::new(&bytes, rectangle_limits())
            .unwrap()
            .read_document()
            .unwrap();
        assert_eq!(document.consumed, bytes.len());
        let graph = LegacyGraph::new(&document, rectangle_limits()).unwrap();
        let world = graph.world_root().unwrap();
        let list = graph
            .required_reference(world, "biogenesis.World", "_organisms")
            .unwrap();
        let (_, members) = graph.synchronized_list(list).unwrap();
        assert_eq!(members.len(), self.world.organisms.borrow().len());
        let mut bindings: Vec<_> = members
            .into_iter()
            .zip(self.world.organisms.borrow().iter())
            .map(|(node, actor)| {
                let Value::Handle(node) = node else {
                    panic!("Actor root");
                };
                (node, actor.clone())
            })
            .collect();
        let mut cursor = 0;
        while cursor < bindings.len() {
            let (node, actor) = bindings[cursor].clone();
            for field in ["_sizeRect", "lastFrame"] {
                let reference = graph.reference(node, "biogenesis.Organism", field).unwrap();
                let Value::Handle(target) = reference else {
                    continue;
                };
                if graph.class_name(target).unwrap() == "biogenesis.Organism" {
                    let cell = {
                        let actor = actor.borrow();
                        if field == "_sizeRect" {
                            actor.body.size_rect.clone()
                        } else {
                            actor.state.last_frame.clone()
                        }
                    }
                    .unwrap();
                    let actor = cell
                        .actor_target()
                        .expect("Actual full source actor association");
                    if let Some((_, previous)) = bindings.iter().find(|(id, _)| *id == target) {
                        assert!(Rc::ptr_eq(previous, &actor));
                    } else {
                        bindings.push((target, actor));
                    }
                }
            }
            cursor += 1;
        }
        assert_eq!(bindings.len(), 3);
        drop(graph);
        let (staged, mut wire) = crate::legacy_rectangles::stage_actor_rectangles(
            &mut document,
            rectangle_limits(),
            bindings,
            4 * 1024 * 1024,
        )
        .unwrap();
        let mut labels = BTreeMap::new();
        let mut reverse = BTreeMap::new();
        for row in rows.iter().filter(|row| row[0] == "RA") {
            let actor = &self.actors[row[2]];
            let state = staged
                .actors
                .iter()
                .find(|state| Rc::ptr_eq(&state.actor, actor))
                .unwrap();
            let cell = match row[3] {
                "this" => Some(&state.bounds),
                "_sizeRect" => state.size_rect.as_ref(),
                "lastFrame" => state.last_frame.as_ref(),
                _ => panic!("Rectangle field"),
            };
            match cell {
                None => assert_eq!(row[4], "~"),
                Some(cell) => {
                    assert_eq!(cell.class_name(), row[5]);
                    assert_eq!(rectangle(cell.get()), row[6..10].join(","));
                    if let Some(previous) = labels.insert(row[4], cell.identity()) {
                        assert_eq!(previous, cell.identity());
                    }
                    if let Some(previous) = reverse.insert(cell.identity(), row[4]) {
                        assert_eq!(previous, row[4]);
                    }
                    let source = wire
                        .rectangle_nodes()
                        .iter()
                        .find(|(_, value)| value.same_instance(cell))
                        .unwrap()
                        .0;
                    let graph = LegacyGraph::new(wire.document(), rectangle_limits()).unwrap();
                    assert_eq!(graph.class_name(source).unwrap(), row[5]);
                    if cell.is_actor() {
                        assert!(
                            Rc::ptr_eq(&cell.actor_target().unwrap(), &state.actor)
                                || row[3] != "this"
                        );
                    }
                }
            }
            self.typed_rectangles += 1;
        }
        wire.export_into(&staged.actors, rectangle_limits(), 1024 * 1024)
            .unwrap();
        assert_eq!(
            crate::object_stream::encode_document(wire.document(), rectangle_limits()).unwrap(),
            bytes
        );
        self.typed_graphs += 1;
    }
    fn snapshot(&mut self, rows: &[Row<'_>], case_rows: &[Row<'_>], verify_graphs: bool) {
        let stage = rows[0][1];
        assert_eq!(rows[0][0], "P");
        if self.world.rectangle_context.borrow().is_none() {
            let context = crate::legacy_rectangles::retain_actor_context(
                self.actors.values().cloned().collect(),
                rectangle_limits(),
                4 * 1024 * 1024,
            )
            .unwrap();
            *self.world.rectangle_context.borrow_mut() = Some(Rc::new(context));
        }
        for role in ["selector", "victim", "killer"] {
            let actor_ref = Rc::clone(&self.actors[role]);
            let actor = actor_ref.borrow();
            let expected: Fields = rows
                .iter()
                .filter(|r| r[0] == "O" && r[2] == role)
                .map(|r| (r[3].into(), r[4].into()))
                .collect();
            assert_eq!(
                expected.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                [
                    "_ID",
                    "hasGrown",
                    "lastFrame",
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
            let rects: Vec<_> = rows
                .iter()
                .filter(|row| row[0] == "RA" && row[2] == role)
                .collect();
            assert_eq!(rects.len(), 3);
            for row in rects {
                let value = match row[3] {
                    "this" => Some(actor.state.bounds.clone()),
                    "_sizeRect" => actor.body.size_rect.clone(),
                    "lastFrame" => actor.state.last_frame.clone(),
                    _ => panic!("Rectangle role"),
                };
                self.rectangle_observation(stage, role, row, value.as_ref());
            }
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
        drop(statistics);
        let rp = rows.iter().find(|r| r[0] == "RP").unwrap();
        let mut random = self.world.random_snapshot();
        assert_eq!(random.next_i64(), rp[2].parse::<i64>().unwrap());
        assert_eq!(random.bounded(10_000), rp[3].parse::<i32>().unwrap());
        assert_eq!(bits(random.next_f64()), rp[4]);
        let r = rows.iter().find(|row| row[0] == "R").unwrap();
        const HEADER: &str = "aced0005737200106a6176612e7574696c2e52616e646f6d363296344bf00a530300035a0014686176654e6578744e657874476175737369616e4400106e6578744e657874476175737369616e4a0004736565647870";
        assert_eq!(
            r[3],
            format!(
                "{HEADER}000000000000000000{:016x}78",
                self.world.random_snapshot().state48()
            )
        );
        if verify_graphs {
            self.typed_graph_snapshot(rows);
            let counts = rectangle_graph_fixture::verify_snapshot(case_rows, stage);
            for (sum, value) in self.graph_counts.iter_mut().zip(counts) {
                *sum += value;
            }
        }
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

fn replay(rows: &[Row<'_>], verify_graphs: bool) -> Replay {
    let mut replay = Replay::new(rows);
    let mut snapshot_start = None;
    for (index, row) in rows.iter().enumerate() {
        match row[0] {
            "GI" => replay.genome(row),
            "A" => replay.construct(row),
            "IR" => replay.rectangle_input(row),
            "IW" => replay.world_input(row),
            "IA" => replay.primitive_input(row),
            "IC" => replay.color_array_input(row),
            "I" => replay.input(row),
            "K" => replay.call(row),
            "X" => replay.outcome(row),
            "V" => replay.returned(row),
            "P" => assert!(snapshot_start.replace(index).is_none()),
            "Z" => {
                let start = snapshot_start.take().unwrap();
                replay.snapshot(&rows[start..=index], rows, verify_graphs);
            }
            "C" | "U" | "O" | "AR" | "AC" | "PA" | "Q" | "GM" | "G" | "N" | "B" | "W" | "M"
            | "S" | "R" | "RP" | "RA" | "E" => {}
            tag => panic!("Unknown array witness row {tag}"),
        }
    }
    assert!(snapshot_start.is_none());
    replay
}

fn rectangle_limits() -> crate::object_stream::Limits {
    crate::object_stream::Limits {
        input_bytes: 262_144,
        output_bytes: 262_144,
        handles: 8192,
        allocated_bytes: 16 * 1024 * 1024,
        work: 8_000_000,
        ..crate::object_stream::Limits::default()
    }
}

#[test]
fn actual_java_rectangle_calls_preserve_aliases_subtypes_prefixes_and_complete_graphs() {
    assert_eq!(SOURCE.lines().count(), 8068);
    let source = cases(SOURCE);
    assert_eq!(source.len(), 6);
    let mut totals = [0; 4];
    let mut graphs = [0; 5];
    let mut typed = [0; 2];
    for rows in source {
        let model = replay(&rows, true);
        assert!(model.errors.is_empty(), "{}", model.errors.join("\n"));
        for (sum, count) in totals.iter_mut().zip([
            model.calls,
            model.snapshots,
            model.primitive_observations,
            model.rectangle_observations,
        ]) {
            *sum += count;
        }
        for (sum, count) in graphs.iter_mut().zip(model.graph_counts) {
            *sum += count;
        }
        typed[0] += model.typed_graphs;
        typed[1] += model.typed_rectangles;
    }
    assert_eq!(totals, [8, 14, 2016, 126]);
    assert_eq!(graphs, [14, 42, 126, 2016, 56]);
    assert_eq!(typed, [14, 126]);
}

#[test]
fn rectangle_checker_rejects_equal_value_owner_change_with_all_inputs_unchanged() {
    let mut changed = false;
    let altered = SOURCE
        .lines()
        .map(|line| {
            if line.starts_with("RA\tinitial\tvictim\t_sizeRect\trect_shared\t") && !changed {
                changed = true;
                line.replacen("\trect_shared\t", "\trect_distinct\t", 1)
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(changed);
    let inputs = |source: &str| {
        source
            .lines()
            .filter(|line| {
                ["C", "U", "GI", "A", "IR", "I", "IW", "K"]
                    .contains(&line.split('\t').next().unwrap())
            })
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(inputs(SOURCE), inputs(&altered));
    assert_eq!(
        SOURCE
            .lines()
            .zip(altered.lines())
            .filter(|(a, b)| a != b)
            .count(),
        1
    );
    let rows = cases(&altered)
        .into_iter()
        .find(|rows| rows[0][1] == "rect_shared")
        .unwrap();
    let model = replay(&rows, false);
    assert_eq!(model.errors.len(), 1);
    assert_eq!(
        model.errors[0],
        "rect_shared\tinitial\tRA\tvictim\t_sizeRect\towner mismatch"
    );
}

fn peer_model_document() -> (
    Replay,
    crate::object_stream::Document,
    Vec<(crate::object_stream::NodeId, OrganismRef)>,
    Vec<u8>,
) {
    use crate::object_stream::{StreamReader, Value};
    let rows = cases(SOURCE)
        .into_iter()
        .find(|rows| rows[0][1] == "rect_peer")
        .unwrap();
    let model = replay(&rows, false);
    assert!(model.errors.is_empty());
    let bytes =
        std::fs::read(crate::oracle_tests::fixture_path(DIRECTORY).join("rect_peer-step_0.ser")).unwrap();
    let document = StreamReader::new(&bytes, rectangle_limits())
        .unwrap()
        .read_document()
        .unwrap();
    let graph = crate::legacy_graph::LegacyGraph::new(&document, rectangle_limits()).unwrap();
    let world = graph.world_root().unwrap();
    let list = graph
        .required_reference(world, "biogenesis.World", "_organisms")
        .unwrap();
    let (_, members) = graph.synchronized_list(list).unwrap();
    assert_eq!(members.len(), 2);
    let [Value::Handle(selector), Value::Handle(victim)] = members.as_slice() else {
        panic!("World members");
    };
    let killer = graph
        .required_reference(*victim, "biogenesis.Organism", "_sizeRect")
        .unwrap();
    graph.expect_class(killer, "biogenesis.Organism").unwrap();
    assert_eq!(
        graph.int(*victim, "biogenesis.Organism", "_ID").unwrap(),
        77
    );
    assert_eq!(graph.int(killer, "biogenesis.Organism", "_ID").unwrap(), 77);
    let bindings = vec![
        (*selector, model.actors["selector"].clone()),
        (*victim, model.actors["victim"].clone()),
        (killer, model.actors["killer"].clone()),
    ];
    drop(graph);
    (model, document, bindings, bytes)
}

#[test]
fn caller_bound_rectangle_stage_and_export_retain_the_complete_nonmember_actor() {
    use crate::legacy_graph::LegacyGraph;
    use crate::legacy_rectangles::stage_actor_rectangles;
    let (model, mut document, bindings, bytes) = peer_model_document();
    let weak = Rc::downgrade(&model.actors["killer"]);
    let killer = bindings[2].0;
    let victim = bindings[1].0;
    let (staged, mut wire) =
        stage_actor_rectangles(&mut document, rectangle_limits(), bindings, 4 * 1024 * 1024)
            .unwrap();
    assert_eq!(staged.context.actors().len(), 3);
    assert_eq!(staged.actors.len(), 3);
    assert!(
        staged.actors[1]
            .size_rect
            .as_ref()
            .unwrap()
            .same_instance(&staged.actors[2].bounds)
    );
    assert!(Rc::ptr_eq(
        &staged.actors[1]
            .size_rect
            .as_ref()
            .unwrap()
            .actor_target()
            .unwrap(),
        &model.actors["killer"]
    ));
    wire.export_into(&staged.actors, rectangle_limits(), 1024 * 1024)
        .unwrap();
    assert_eq!(
        crate::object_stream::encode_document(wire.document(), rectangle_limits()).unwrap(),
        bytes
    );
    let graph = LegacyGraph::new(wire.document(), rectangle_limits()).unwrap();
    assert_eq!(
        graph
            .required_reference(victim, "biogenesis.Organism", "_sizeRect")
            .unwrap(),
        killer
    );
    assert_eq!(graph.class_name(killer).unwrap(), "biogenesis.Organism");
    assert_eq!(graph.slots(killer).unwrap().len(), 2);
    drop(graph);
    drop(wire);
    drop(document);
    drop(model);
    let crate::legacy_rectangles::StagedRectangles { actors, context } = staged;
    drop(actors);
    assert!(weak.upgrade().is_some());
    drop(context);
    assert!(weak.upgrade().is_none());
}

fn replace_graph_reference(
    document: &mut crate::object_stream::Document,
    id: crate::object_stream::NodeId,
    field: &str,
    reference: crate::object_stream::Value,
) {
    use crate::object_stream::{FieldValue, Node, class_schema};
    let Node::Object(actor) = &document.arena.nodes[id.0 as usize] else {
        panic!("Actor object");
    };
    let (slot, index) = actor
        .slots
        .iter()
        .enumerate()
        .find_map(|(slot, value)| {
            let Node::Descriptor(descriptor) = &document.arena.nodes[value.descriptor.0 as usize]
            else {
                panic!("Descriptor");
            };
            let schema = class_schema(descriptor.class);
            (schema.name == "biogenesis.Organism").then(|| {
                (
                    slot,
                    schema
                        .fields
                        .iter()
                        .position(|value| value.name == field)
                        .unwrap(),
                )
            })
        })
        .unwrap();
    let Node::Object(actor) = &mut document.arena.nodes[id.0 as usize] else {
        unreachable!();
    };
    actor.slots[slot].values[index] = FieldValue::Reference(reference);
}

#[test]
fn rectangle_staging_rejects_missing_duplicate_and_inconsistent_bindings_without_actor_mutation() {
    use crate::legacy_graph::{GraphErrorKind, LegacyGraph};
    use crate::legacy_rectangles::stage_actor_rectangles;
    let (model, mut document, bindings, _) = peer_model_document();
    let before: Vec<_> = model
        .actors
        .values()
        .map(|actor| actor.borrow().snapshot())
        .collect();
    let identities: Vec<_> = model
        .actors
        .values()
        .map(|actor| actor.borrow().state.bounds.identity())
        .collect();
    assert!(
        matches!(stage_actor_rectangles(&mut document, rectangle_limits(), bindings[..2].to_vec(), 4 * 1024 * 1024), Err(error) if error.kind == GraphErrorKind::Reference)
    );
    let mut duplicate = bindings.clone();
    duplicate.push(bindings[0].clone());
    assert!(
        matches!(stage_actor_rectangles(&mut document, rectangle_limits(), duplicate, 4 * 1024 * 1024), Err(error) if error.kind == GraphErrorKind::Reference)
    );
    let mut duplicate = bindings.clone();
    duplicate[2].1 = bindings[1].1.clone();
    assert!(
        matches!(stage_actor_rectangles(&mut document, rectangle_limits(), duplicate, 4 * 1024 * 1024), Err(error) if error.kind == GraphErrorKind::Reference)
    );
    assert!(
        matches!(stage_actor_rectangles(&mut document, rectangle_limits(), Vec::new(), 4 * 1024 * 1024), Err(error) if error.kind == GraphErrorKind::Reference)
    );
    assert!(
        stage_actor_rectangles(&mut document, rectangle_limits(), bindings.clone(), 1).is_err()
    );
    let complete_unbound = Rc::new(RefCell::new(
        model.actors["killer"]
            .borrow()
            .copy_for_world(&mut crate::organism::GenomeCloner::default()),
    ));
    let mut unbound = bindings.clone();
    unbound[2].1 = complete_unbound;
    assert!(
        matches!(stage_actor_rectangles(&mut document, rectangle_limits(), unbound, 4 * 1024 * 1024), Err(error) if error.kind == GraphErrorKind::Reference)
    );
    let mut wrong_world = document.clone();
    replace_graph_reference(
        &mut wrong_world,
        bindings[2].0,
        "_world",
        crate::object_stream::Value::Null,
    );
    assert!(
        matches!(stage_actor_rectangles(&mut wrong_world, rectangle_limits(), bindings.clone(), 4 * 1024 * 1024), Err(error) if error.kind == GraphErrorKind::Reference)
    );
    let code = LegacyGraph::new(&document, rectangle_limits())
        .unwrap()
        .reference(bindings[1].0, "biogenesis.Organism", "_geneticCode")
        .unwrap();
    let mut unsupported = document.clone();
    replace_graph_reference(&mut unsupported, bindings[1].0, "_sizeRect", code);
    assert!(
        matches!(stage_actor_rectangles(&mut unsupported, rectangle_limits(), bindings.clone(), 4 * 1024 * 1024), Err(error) if error.kind == GraphErrorKind::Class)
    );
    assert_eq!(
        model
            .actors
            .values()
            .map(|actor| actor.borrow().snapshot())
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(
        model
            .actors
            .values()
            .map(|actor| actor.borrow().state.bounds.identity())
            .collect::<Vec<_>>(),
        identities
    );
    let bounds = model.actors["killer"].borrow().state.bounds.clone();
    let original = bounds.get();
    let sentinel = IntRect {
        x: -1,
        y: -2,
        width: -3,
        height: -4,
    };
    bounds.set(sentinel);
    assert!(
        matches!(stage_actor_rectangles(&mut document, rectangle_limits(), bindings, 4 * 1024 * 1024), Err(error) if error.kind == GraphErrorKind::Reference)
    );
    assert_eq!(bounds.get(), sentinel);
    bounds.set(original);
}

#[test]
fn rectangle_external_export_is_atomic_and_rejects_unbound_owners() {
    use crate::legacy_graph::GraphErrorKind;
    let (_, mut document, bindings, _) = peer_model_document();
    let before = document.clone();
    let (staged, mut wire) = crate::legacy_rectangles::stage_actor_rectangles(
        &mut document,
        rectangle_limits(),
        bindings,
        4 * 1024 * 1024,
    )
    .unwrap();
    assert!(
        matches!(wire.export_into(&staged.actors, rectangle_limits(), 0), Err(error) if error.kind == GraphErrorKind::Budget)
    );
    assert_eq!(wire.document(), &before);
    let mut malformed = staged.actors.clone();
    malformed[1].size_rect = Some(crate::RectangleRef::new(IntRect::default()));
    assert!(
        matches!(wire.export_into(&malformed, rectangle_limits(), 1024 * 1024), Err(error) if error.kind == GraphErrorKind::Reference)
    );
    assert_eq!(wire.document(), &before);
}

#[test]
fn world_rectangle_copies_detach_cells_and_drop_complete_weak_target_cycles() {
    let rows = cases(SOURCE)
        .into_iter()
        .find(|rows| rows[0][1] == "rect_peer")
        .unwrap();
    let (world, target) = {
        let model = replay(&rows, false);
        let target = Rc::downgrade(&model.actors["killer"]);
        (model.world, target)
    };
    assert_eq!(world.organisms.borrow().len(), 2);
    assert!(target.upgrade().is_some());
    let copied = world.clone();
    let copied_actor = copied.organisms.borrow()[1].clone();
    let copied_target = copied_actor
        .borrow()
        .body
        .size_rect
        .as_ref()
        .unwrap()
        .actor_target()
        .unwrap();
    assert!(!Rc::ptr_eq(&copied_target, &target.upgrade().unwrap()));
    let original = target.upgrade().unwrap().borrow().state.bounds.get();
    let sentinel = IntRect {
        x: -31,
        y: 37,
        width: 41,
        height: 43,
    };
    copied_target.borrow().state.bounds.set(sentinel);
    assert_eq!(copied_actor.borrow().body.size().unwrap(), sentinel);
    assert_eq!(
        target.upgrade().unwrap().borrow().state.bounds.get(),
        original
    );
    let snapshot = copied_actor.borrow().snapshot();
    snapshot
        .body
        .size_rect
        .as_ref()
        .unwrap()
        .set(IntRect::default());
    assert_eq!(copied_actor.borrow().body.size().unwrap(), sentinel);
    let cohort_len = copied.rectangle_context.borrow().as_ref().unwrap().actors().len();
    let mut extra = copied.organisms.borrow()[0]
        .borrow()
        .copy_for_world(&mut crate::organism::GenomeCloner::default());
    extra.id = 78;
    copied.add_organism(Rc::new(RefCell::new(extra)), None);
    assert_eq!(
        copied.rectangle_context.borrow().as_ref().unwrap().actors().len(),
        cohort_len
    );
    let cloned_weak = Rc::downgrade(&copied_target);
    drop(copied_target);
    drop(copied_actor);
    drop(copied);
    assert!(cloned_weak.upgrade().is_none());
    // Exercise a mutual source-reference cycle without creating a strong actor back-edge
    let actor = world.organisms.borrow()[1].clone();
    let peer = target.upgrade().unwrap();
    peer.borrow_mut().body.size_rect = Some(actor.borrow().state.bounds.clone());
    actor.borrow_mut().state.last_frame = Some(peer.borrow().state.bounds.clone());
    let mutual = world.clone();
    let copied_actor = mutual.organisms.borrow()[1].clone();
    let copied_peer = copied_actor
        .borrow()
        .body
        .size_rect
        .as_ref()
        .unwrap()
        .actor_target()
        .unwrap();
    assert!(
        copied_peer
            .borrow()
            .body
            .size_rect
            .as_ref()
            .unwrap()
            .same_instance(&copied_actor.borrow().state.bounds)
    );
    assert!(
        copied_actor
            .borrow()
            .state
            .last_frame
            .as_ref()
            .unwrap()
            .same_instance(&copied_peer.borrow().state.bounds)
    );
    drop(copied_actor);
    drop(copied_peer);
    drop(mutual);
    drop(peer);
    drop(actor);
    drop(world);
    assert!(target.upgrade().is_none());
}

#[test]
fn complete_actor_retention_charges_spare_capacity_shared_colors_and_work_before_publication() {
    use crate::legacy_graph::GraphErrorKind;
    use crate::legacy_rectangles::retain_actor_context;
    let rows = cases(SOURCE)
        .into_iter()
        .find(|rows| rows[0][1] == "rect_peer")
        .unwrap();
    let model = replay(&rows, false);
    let actors = || model.actors.values().cloned().collect::<Vec<_>>();
    let initial = retain_actor_context(actors(), rectangle_limits(), 4 * 1024 * 1024)
        .unwrap()
        .retained_bytes();
    assert!(
        matches!(retain_actor_context(actors(), rectangle_limits(), initial - 1), Err(error) if error.kind == GraphErrorKind::Budget)
    );
    let old_capacity = model.actors["selector"]
        .borrow()
        .body
        .arrays
        .int(crate::IntField::X1)
        .as_ref()
        .unwrap()
        .borrow_values()
        .capacity();
    let mut values = Vec::with_capacity(1024);
    values.push(1);
    model.actors["selector"].borrow_mut().body.arrays.x1 =
        Some(crate::IntArrayRef::from_values(values));
    let larger = retain_actor_context(actors(), rectangle_limits(), 4 * 1024 * 1024)
        .unwrap()
        .retained_bytes();
    assert!(larger >= initial + (1024 - old_capacity) * std::mem::size_of::<i32>());
    let mut values = Vec::with_capacity(1024);
    values.extend([0, 1, 2]);
    let shared = crate::JavaFloatArray::from_bits(values);
    let color = crate::JavaColor::from_parts(crate::java_color::ColorData {
        argb: 0xff123456,
        alpha_bits: 0,
        space: None,
        rgb: Some(shared.clone()),
        components: Some(shared),
    });
    model.actors["selector"].borrow_mut().state.temporary_color = Some(color);
    let shared_cost = retain_actor_context(actors(), rectangle_limits(), 4 * 1024 * 1024)
        .unwrap()
        .retained_bytes();
    let mut values = Vec::with_capacity(1024);
    values.extend([0, 1, 2]);
    let first = model.actors["selector"]
        .borrow()
        .state
        .temporary_color
        .as_ref()
        .unwrap()
        .rgb_components()
        .unwrap()
        .clone();
    let color = crate::JavaColor::from_parts(crate::java_color::ColorData {
        argb: 0xff123456,
        alpha_bits: 0,
        space: None,
        rgb: Some(first),
        components: Some(crate::JavaFloatArray::from_bits(values)),
    });
    model.actors["selector"].borrow_mut().state.temporary_color = Some(color);
    let distinct_cost = retain_actor_context(actors(), rectangle_limits(), 4 * 1024 * 1024)
        .unwrap()
        .retained_bytes();
    assert!(distinct_cost >= shared_cost + 1024 * std::mem::size_of::<u32>());
    let before = model.actors["selector"].borrow().snapshot();
    let mut limits = rectangle_limits();
    limits.work = 1;
    assert!(
        matches!(retain_actor_context(actors(), limits, 4 * 1024 * 1024), Err(error) if error.kind == GraphErrorKind::Budget)
    );
    assert_eq!(model.actors["selector"].borrow().snapshot(), before);
}

#[path = "dense_rectangle_tests.rs"]
mod dense_rectangle_tests;
