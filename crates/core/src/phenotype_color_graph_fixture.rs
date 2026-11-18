// SPDX-License-Identifier: GPL-2.0-or-later

//! Verify captured graphs without hydrating or running their saved World state

use crate::legacy_color::ColorStager;
use crate::legacy_graph::LegacyGraph;
use crate::object_stream::{ArrayData, FieldValue, Limits, Node, NodeId, StreamReader, Value, encode_document};
use crate::{ColorArrayRef, JavaColor};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{DIRECTORY, SOURCE};

type Row = Vec<&'static str>;
const ORGANISM: &str = "biogenesis.Organism";
const WORLD: &str = "biogenesis.World";
const RECTANGLE: &str = "java.awt.Rectangle";

fn limits() -> Limits {
    Limits {
        input_bytes: 256 * 1024,
        output_bytes: 256 * 1024,
        handles: 8192,
        array_elements: 4096,
        total_array_elements: 65_536,
        string_bytes: 256 * 1024,
        string_units: 65_536,
        records: 8192,
        allocated_bytes: 16 * 1024 * 1024,
        work: 8_000_000,
        ..Limits::default()
    }
}

#[derive(Default)]
struct Owners {
    wire: BTreeMap<&'static str, Value>,
    colors: BTreeMap<&'static str, JavaColor>,
    arrays: BTreeMap<&'static str, ColorArrayRef>,
}

impl Owners {
    fn bind(&mut self, label: &'static str, value: Value) {
        if label == "~" {
            assert_eq!(value, Value::Null);
        } else if let Some(previous) = self.wire.get(label) {
            assert_eq!(*previous, value, "Owner {label}");
        } else {
            assert!(matches!(value, Value::Handle(_)), "Nonnull owner {label}");
            assert!(self.wire.values().all(|previous| *previous != value), "Distinct owner {label}");
            self.wire.insert(label, value);
        }
    }
}

fn one<'a>(rows: &[&'a Row], tag: &str, role: Option<&str>) -> &'a Row {
    let matches: Vec<_> = rows.iter().copied().filter(|row|
        row[0] == tag && role.is_none_or(|role| row[2] == role)
    ).collect();
    assert_eq!(matches.len(), 1, "One {tag} row for {role:?}");
    matches[0]
}

fn handle(value: Value) -> NodeId {
    let Value::Handle(id) = value else { panic!("Expected captured nonnull reference"); };
    id
}

fn color(
    graph: &LegacyGraph<'_>,
    stage: &mut ColorStager<'_, '_>,
    owners: &mut Owners,
    value: Value,
    label: &'static str,
    argb: &str,
) -> Option<JavaColor> {
    owners.bind(label, value);
    let actual = stage.color(value).unwrap();
    if label == "~" {
        assert_eq!(argb, "~");
        assert!(actual.is_none());
    } else {
        let actual = actual.as_ref().unwrap();
        let expected = u32::from_str_radix(argb, 16).unwrap();
        assert_eq!(graph.int(handle(value), "java.awt.Color", "value").unwrap() as u32, expected);
        assert_eq!(actual.argb(), expected);
        for (other_label, other) in &owners.colors {
            assert_eq!(actual.same_instance(other), *other_label == label, "Color owner {label}/{other_label}");
        }
        owners.colors.insert(label, actual.clone());
    }
    actual
}

fn color_text(
    graph: &LegacyGraph<'_>,
    stage: &mut ColorStager<'_, '_>,
    owners: &mut Owners,
    value: Value,
    expected: &'static str,
) {
    let (label, argb) = if expected == "~" { ("~", "~") } else {
        expected.split_once(':').unwrap()
    };
    let _ = color(graph, stage, owners, value, label, argb);
}

fn scalar(graph: &LegacyGraph<'_>, actor: NodeId, owner: &'static str, field: &'static str) -> String {
    match graph.field(actor, owner, field).unwrap() {
        FieldValue::Boolean(value) => (value != 0).to_string(),
        FieldValue::Int(value) => value.to_string(),
        FieldValue::DoubleBits(value) => format!("{value:016x}"),
        FieldValue::Reference(Value::Null) => "~".to_owned(),
        FieldValue::Reference(Value::Handle(id)) => match graph.node(id).unwrap() {
            Node::Array(array) => match &array.data {
                ArrayData::Ints(values) => values.iter().map(i32::to_string).collect::<Vec<_>>().join(","),
                ArrayData::Doubles(values) => values.iter().map(|value| format!("{value:016x}")).collect::<Vec<_>>().join(","),
                _ => panic!("Unexpected captured primitive array {field}"),
            },
            Node::Object(_) => {
                graph.expect_class(id, RECTANGLE).unwrap();
                ["x", "y", "width", "height"].map(|field|
                    graph.int(id, RECTANGLE, field).unwrap().to_string()
                ).join(",")
            }
            _ => panic!("Unexpected captured field {field}"),
        },
        _ => panic!("Unexpected captured scalar {field}"),
    }
}

fn phenotype(
    graph: &LegacyGraph<'_>,
    stage: &mut ColorStager<'_, '_>,
    owners: &mut Owners,
    actor: NodeId,
    role: &str,
    rows: &[&Row],
) -> usize {
    let expected = one(rows, "AR", Some(role));
    let value = graph.reference(actor, ORGANISM, "_segColor").unwrap();
    owners.bind(expected[3], value);
    let staged = stage.colors(value).unwrap();
    let slots: Vec<_> = rows.iter().copied().filter(|row| row[0] == "AC" && row[2] == role).collect();
    let Some(array) = staged else {
        assert_eq!(expected[4], "~");
        assert!(slots.is_empty());
        return 0;
    };
    let length: usize = expected[4].parse().unwrap();
    assert_eq!(array.len(), length);
    assert_eq!(slots.len(), length);
    let Node::Array(raw) = graph.node(handle(value)).unwrap() else { panic!("Color array node"); };
    let ArrayData::References(values) = &raw.data else { panic!("Color reference slots"); };
    assert_eq!(values.len(), length);
    for (label, other) in &owners.arrays {
        assert_eq!(array.same_instance(other), *label == expected[3], "Array owner {role}/{label}");
    }
    owners.arrays.insert(expected[3], array.clone());
    for (index, raw_value) in values.iter().copied().enumerate() {
        let matching: Vec<_> = slots.iter().filter(|row| row[3].parse::<usize>().unwrap() == index).collect();
        assert_eq!(matching.len(), 1, "Color slot {role}/{index}");
        let row = matching[0];
        let expected_color = color(graph, stage, owners, raw_value, row[4], row[5]);
        let actual_color = array.get(index).unwrap();
        match (actual_color, expected_color) {
            (None, None) => {}
            (Some(actual), Some(expected)) => assert!(actual.same_instance(&expected)),
            _ => panic!("Nullable Color slot {role}/{index}"),
        }
    }
    length
}

fn snapshot(graph: &LegacyGraph<'_>, case: &[Row], name: &str, verify_pa: bool) -> (usize, usize) {
    let rows: Vec<_> = case.iter().filter(|row| row.len() > 1 && row[1] == name).collect();
    let world = graph.world_root().unwrap();
    let list = graph.required_reference(world, WORLD, "_organisms").unwrap();
    let (_, members) = graph.synchronized_list(list).unwrap();
    let roles: Vec<_> = one(&rows, "M", None)[2].split(',').collect();
    let placements: Vec<_> = case.iter().filter(|row| row[0] == "A").collect();
    assert_eq!(members.len(), roles.len());
    assert_eq!(members.len(), placements.len());
    assert_eq!(members.len(), 3);
    let mut actors = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for (role, value) in roles.iter().copied().zip(members) {
        let actor = handle(value);
        graph.expect_class(actor, ORGANISM).unwrap();
        assert!(actors.insert(role, actor).is_none());
        let id = graph.int(actor, ORGANISM, "_ID").unwrap();
        assert!(ids.insert(id));
        // Each fresh captured World assigns one ID per successful A placement
        let ordinal = placements.iter().position(|row| row[1] == role).unwrap();
        assert_eq!(id, ordinal as i32, "Captured member ID {role}");
        let bounds = one(&rows, "B", Some(role));
        assert_eq!(bounds.len(), 7);
        for (field, expected) in ["x", "y", "width", "height"].into_iter().zip(&bounds[3..]) {
            assert_eq!(graph.int(actor, RECTANGLE, field).unwrap().to_string(), *expected, "Member bounds {role}/{field}");
        }
    }
    assert_eq!(graph.int(world, WORLD, "NEXT_ID").unwrap(), placements.len() as i32);
    let mut owners = Owners::default();
    let mut stage = ColorStager::new(graph, 4 * 1024 * 1024).unwrap();
    let mut primitives = crate::legacy_organism_arrays::PrimitiveStager::new(graph, 4 * 1024 * 1024).unwrap();
    let mut int_owners = Vec::new();
    let mut double_owners = Vec::new();
    for (role, actor) in &actors {
        let genome = graph.reference(*actor, ORGANISM, "_geneticCode").unwrap();
        owners.bind(one(&rows, "G", Some(role))[3], genome);
    }
    let pa = rows.iter().filter(|row| row[0] == "PA").count();
    assert_eq!(pa, if verify_pa { actors.len() * 48 } else { 0 });
    let mut slots = 0;
    for (role, actor) in actors {
        let staged = primitives.organism(actor).unwrap();
        assert_eq!(staged.segment_count.to_string(), one_field(&rows, "O", role, "_segments"));
        for field in crate::organism_arrays::INT_FIELDS {
            let kind = if matches!(field, crate::IntField::Reaction(_)) { "Q" } else { "O" };
            let expected = one_field(&rows, kind, role, field.name());
            let raw = graph.reference(actor, ORGANISM, field.name()).unwrap();
            let actual = staged.arrays.int(field).as_ref();
            if verify_pa {
                primitive_metadata(&rows, &mut owners, role, field.name(), "int[]", raw, actual.map(|owner| owner.len()));
            }
            match (raw, actual) {
                (Value::Null, None) => assert_eq!(expected, "~"),
                (Value::Handle(id), Some(owner)) => {
                    assert_eq!(owner.values().iter().map(i32::to_string).collect::<Vec<_>>().join(","), expected);
                    for (other_id, other) in &int_owners { assert_eq!(owner.same_instance(other), id == *other_id); }
                    int_owners.push((id, owner.clone()));
                }
                _ => panic!("Primitive int nullness {role}/{}", field.name()),
            }
        }
        for field in crate::organism_arrays::DOUBLE_FIELDS {
            let expected = one_field(&rows, "O", role, field.name());
            let raw = graph.reference(actor, ORGANISM, field.name()).unwrap();
            let actual = staged.arrays.double(field).as_ref();
            if verify_pa {
                primitive_metadata(&rows, &mut owners, role, field.name(), "double[]", raw, actual.map(|owner| owner.len()));
            }
            match (raw, actual) {
                (Value::Null, None) => assert_eq!(expected, "~"),
                (Value::Handle(id), Some(owner)) => {
                    assert_eq!(owner.bits().iter().map(|bits| format!("{bits:016x}")).collect::<Vec<_>>().join(","), expected);
                    for (other_id, other) in &double_owners { assert_eq!(owner.same_instance(other), id == *other_id); }
                    double_owners.push((id, owner.clone()));
                }
                _ => panic!("Primitive double nullness {role}/{}", field.name()),
            }
        }
        slots += phenotype(graph, &mut stage, &mut owners, actor, role, &rows);
        for row in rows.iter().filter(|row| row[0] == "O" && row[2] == role) {
            match row[3] {
                "_segColor" => owners.bind(row[4], graph.reference(actor, ORGANISM, "_segColor").unwrap()),
                "_color" => color_text(graph, &mut stage, &mut owners, graph.reference(actor, ORGANISM, "_color").unwrap(), row[4]),
                "_infectedGeneticCode" | "_savedGeneticCode" => owners.bind(row[4], graph.reference(actor, ORGANISM, row[3]).unwrap()),
                field => assert_eq!(scalar(graph, actor, ORGANISM, field), row[4], "{role}/{field}"),
            }
        }
        for row in rows.iter().filter(|row| row[0] == "Q" && row[2] == role) {
            assert_eq!(scalar(graph, actor, ORGANISM, row[3]), row[4], "Reaction {role}/{}", row[3]);
        }
        let genome = graph.required_reference(actor, ORGANISM, "_geneticCode").unwrap();
        assert_eq!(graph.boolean(genome, "biogenesis.GeneticCode", "_mimicall").unwrap().to_string(), one(&rows, "GM", Some(role))[3]);
        let genes = graph.required_reference(genome, "biogenesis.GeneticCode", "_genes").unwrap();
        let Node::Array(genes) = graph.node(genes).unwrap() else { panic!("Gene array"); };
        let ArrayData::References(genes) = &genes.data else { panic!("Gene reference array"); };
        let expected: Vec<_> = rows.iter().filter(|row| row[0] == "N" && row[2] == role).collect();
        assert_eq!(genes.len(), expected.len());
        for (index, gene) in genes.iter().copied().enumerate() {
            let matches: Vec<_> = expected.iter().filter(|row| row[3].parse::<usize>().unwrap() == index).collect();
            assert_eq!(matches.len(), 1, "Gene {role}/{index}");
            let value = graph.reference(handle(gene), "biogenesis.Gene", "_color").unwrap();
            color_text(graph, &mut stage, &mut owners, value, matches[0][4]);
        }
    }
    for row in rows.iter().filter(|row| row[0] == "W") {
        assert_eq!(scalar(graph, world, WORLD, row[2]), row[3], "World {}", row[2]);
    }
    (slots, pa)
}

pub(super) fn verify_graph_fixtures() {
    verify_captured_graph_fixtures(SOURCE, DIRECTORY, 27, 114, 0);
}

pub(in crate::world::organism_oracle_tests) fn verify_captured_graph_fixtures(
    source: &'static str,
    directory: &str,
    expected_streams: usize,
    expected_slots: usize,
    expected_pa: usize,
) {
    let mut files = BTreeSet::new();
    let mut slots = 0;
    let mut pa = 0;
    for case in super::cases(source) {
        for row in case.iter().filter(|row| row[0] == "Z") {
            let path = Path::new(row[2]);
            assert_eq!(path.components().count(), 1);
            assert_eq!(path.extension().unwrap(), "ser");
            assert!(files.insert(row[2]), "Unique snapshot filename");
            let path = crate::oracle_tests::fixture_path(directory).join(path);
            let metadata = std::fs::metadata(&path).unwrap();
            assert!(metadata.is_file() && metadata.len() <= limits().input_bytes as u64);
            let bytes = std::fs::read(path).unwrap();
            assert_eq!(bytes.len(), row[3].parse::<usize>().unwrap());
            let document = StreamReader::new(&bytes, limits()).unwrap().read_document().unwrap();
            assert_eq!(document.consumed, bytes.len(), "Complete captured stream {}", row[2]);
            let graph = LegacyGraph::new(&document, limits()).unwrap();
            let (snapshot_slots, snapshot_pa) = snapshot(&graph, &case, row[1], expected_pa != 0);
            slots += snapshot_slots;
            pa += snapshot_pa;
            assert_eq!(encode_document(&document, limits()).unwrap(), bytes, "Captured wire roundtrip {}", row[2]);
        }
    }
    assert_eq!(files.len(), expected_streams);
    assert_eq!(slots, expected_slots);
    assert_eq!(pa, expected_pa);
}


fn one_field<'a>(rows: &[&'a Row], kind: &str, role: &str, field: &str) -> &'a str {
    let matching: Vec<_> = rows.iter().filter(|row| row[0] == kind && row[2] == role && row[3] == field).collect();
    assert_eq!(matching.len(), 1, "Captured primitive field {role}/{field}");
    matching[0][4]
}


fn primitive_metadata(
    rows: &[&Row],
    owners: &mut Owners,
    role: &str,
    field: &str,
    kind: &str,
    value: Value,
    length: Option<usize>,
) {
    let matching: Vec<_> = rows.iter().copied().filter(|row|
        row[0] == "PA" && row[2] == role && row[3] == field
    ).collect();
    assert_eq!(matching.len(), 1, "Primitive metadata {role}/{field}");
    let row = matching[0];
    assert_eq!(row.len(), 7);
    assert_eq!(row[4], kind, "Primitive type {role}/{field}");
    owners.bind(row[5], value);
    match length {
        Some(length) => assert_eq!(row[6].parse::<usize>().unwrap(), length, "Primitive length {role}/{field}"),
        None => assert_eq!(row[6], "~"),
    }
}
