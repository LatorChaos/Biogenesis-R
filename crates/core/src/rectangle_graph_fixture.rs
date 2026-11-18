// SPDX-License-Identifier: GPL-2.0-or-later

//! Check captured Rectangle graphs without hydrating or executing their World state

use crate::legacy_graph::LegacyGraph;
use crate::object_stream::{
    ArrayData, FieldValue, Limits, Node, NodeId, StreamReader, Value, class_by_name, class_schema,
    encode_document,
};
use std::collections::BTreeMap;
use std::path::Path;

type Row<'a> = Vec<&'a str>;
const ORGANISM: &str = "biogenesis.Organism";
const WORLD: &str = "biogenesis.World";
const RECTANGLE: &str = "java.awt.Rectangle";
const STATISTICS: &str = "biogenesis.WorldStatistics";

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
struct Owners<'a>(BTreeMap<&'a str, Value>);

impl<'a> Owners<'a> {
    fn bind(&mut self, label: &'a str, value: Value) {
        if label == "~" {
            assert_eq!(value, Value::Null);
        } else if let Some(previous) = self.0.get(label) {
            assert_eq!(*previous, value, "Shared owner {label}");
        } else {
            assert!(matches!(value, Value::Handle(_)), "Nonnull owner {label}");
            assert!(
                self.0.values().all(|other| *other != value),
                "Distinct owner {label}"
            );
            self.0.insert(label, value);
        }
    }
}

fn one<'r, 's>(rows: &[&'r Row<'s>], tag: &str, role: Option<&str>) -> &'r Row<'s> {
    let found: Vec<_> = rows
        .iter()
        .copied()
        .filter(|row| row[0] == tag && role.is_none_or(|role| row[2] == role))
        .collect();
    assert_eq!(found.len(), 1, "One {tag} row for {role:?}");
    found[0]
}

fn handle(value: Value) -> NodeId {
    let Value::Handle(id) = value else {
        panic!("Nonnull captured reference");
    };
    id
}

fn field_name(owner: &'static str, name: &str) -> &'static str {
    class_schema(class_by_name(owner).unwrap())
        .fields
        .iter()
        .find(|field| field.name == name)
        .unwrap()
        .name
}

fn reference(graph: &LegacyGraph<'_>, id: NodeId, owner: &'static str, name: &str) -> Value {
    graph.reference(id, owner, field_name(owner, name)).unwrap()
}

fn primitive(value: FieldValue) -> Option<String> {
    Some(match value {
        FieldValue::Boolean(value) => (value != 0).to_string(),
        FieldValue::Byte(value) => value.to_string(),
        FieldValue::Char(value) => value.to_string(),
        FieldValue::Short(value) => value.to_string(),
        FieldValue::Int(value) => value.to_string(),
        FieldValue::Long(value) => value.to_string(),
        FieldValue::FloatBits(value) => format!("{value:08x}"),
        FieldValue::DoubleBits(value) => format!("{value:016x}"),
        FieldValue::Reference(_) => return None,
    })
}

fn bounds(graph: &LegacyGraph<'_>, id: NodeId) -> [String; 4] {
    assert!(matches!(
        graph.class_name(id).unwrap(),
        RECTANGLE | ORGANISM
    ));
    ["x", "y", "width", "height"].map(|name| graph.int(id, RECTANGLE, name).unwrap().to_string())
}

fn compare_field<'a>(
    graph: &LegacyGraph<'_>,
    owners: &mut Owners<'a>,
    id: NodeId,
    owner: &'static str,
    name: &str,
    expected: &'a str,
) {
    let value = graph.field(id, owner, field_name(owner, name)).unwrap();
    if let Some(actual) = primitive(value) {
        assert_eq!(actual, expected, "{owner}/{name}");
        return;
    }
    let FieldValue::Reference(value) = value else {
        unreachable!()
    };
    let Value::Handle(id) = value else {
        assert_eq!(expected, "~", "Null {owner}/{name}");
        return;
    };
    let actual = match graph.node(id).unwrap() {
        Node::Array(array) => match &array.data {
            ArrayData::Ints(values) => values
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(","),
            ArrayData::Doubles(values) => values
                .iter()
                .map(|bits| format!("{bits:016x}"))
                .collect::<Vec<_>>()
                .join(","),
            _ => panic!("Unexpected captured array {owner}/{name}"),
        },
        Node::Object(_) => match graph.class_name(id).unwrap() {
            RECTANGLE | ORGANISM => bounds(graph, id).join(","),
            "biogenesis.GeneticCode" => {
                owners.bind(expected, value);
                return;
            }
            "java.util.ArrayList" => graph
                .array_list(id)
                .unwrap()
                .into_iter()
                .map(|value| match value {
                    Value::Null => "~".to_owned(),
                    Value::Handle(id) => {
                        graph.expect_class(id, "java.lang.Double").unwrap();
                        format!(
                            "{:016x}",
                            graph.double_bits(id, "java.lang.Double", "value").unwrap()
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
            _ => panic!("Unexpected captured object {owner}/{name}"),
        },
        _ => panic!("Unexpected captured field {owner}/{name}"),
    };
    assert_eq!(actual, expected, "{owner}/{name}");
}

fn color<'a>(
    graph: &LegacyGraph<'_>,
    owners: &mut Owners<'a>,
    value: Value,
    label: &'a str,
    argb: &str,
) {
    owners.bind(label, value);
    match value {
        Value::Null => assert_eq!(argb, "~"),
        Value::Handle(id) => {
            graph.expect_class(id, "java.awt.Color").unwrap();
            assert_eq!(
                graph.int(id, "java.awt.Color", "value").unwrap() as u32,
                u32::from_str_radix(argb, 16).unwrap()
            );
        }
    }
}

fn color_text<'a>(graph: &LegacyGraph<'_>, owners: &mut Owners<'a>, value: Value, text: &'a str) {
    let (label, argb) = if text == "~" {
        ("~", "~")
    } else {
        text.split_once(':').unwrap()
    };
    color(graph, owners, value, label, argb);
}

fn actors<'a>(
    graph: &LegacyGraph<'_>,
    case: &[Row<'a>],
    rows: &[&Row<'a>],
) -> BTreeMap<&'a str, NodeId> {
    let world = graph.world_root().unwrap();
    let (_, members) = graph
        .synchronized_list(handle(reference(graph, world, WORLD, "_organisms")))
        .unwrap();
    let placements: Vec<_> = case
        .iter()
        .filter(|row| row[0] == "A")
        .map(|row| row[1])
        .collect();
    assert_eq!(placements.len(), 3);
    let mut member_roles = placements.clone();
    for row in case
        .iter()
        .filter(|row| row[0] == "IW" && row[1] == "world:_organisms")
    {
        assert_eq!(row[2], "remove_identity");
        let index = member_roles
            .iter()
            .position(|role| *role == row[3])
            .unwrap();
        member_roles.remove(index);
    }
    assert_eq!(one(rows, "M", None)[2], member_roles.join(","));
    assert_eq!(members.len(), member_roles.len());
    let mut actors: BTreeMap<_, _> = member_roles
        .into_iter()
        .zip(members)
        .map(|(role, value)| (role, handle(value)))
        .collect();
    // Primary Rectangle assignments locate actors absent from the member list
    loop {
        let previous = actors.len();
        for row in case
            .iter()
            .filter(|row| row[0] == "I" && row[2] == "Rectangle")
        {
            let Some(target) = row[3].strip_prefix("actor_") else {
                continue;
            };
            let Some((source, field)) = row[1]
                .strip_prefix("actor:")
                .and_then(|key| key.split_once(':'))
            else {
                continue;
            };
            let Some(source) = actors.get(source).copied() else {
                continue;
            };
            let target_id = handle(reference(graph, source, ORGANISM, field));
            if let Some(previous) = actors.insert(target, target_id) {
                assert_eq!(previous, target_id);
            }
        }
        if previous == actors.len() {
            break;
        }
    }
    assert_eq!(actors.len(), placements.len());
    for (ordinal, role) in placements.into_iter().enumerate() {
        let id = actors[role];
        graph.expect_class(id, ORGANISM).unwrap();
        assert_eq!(
            reference(graph, id, ORGANISM, "_world"),
            Value::Handle(world)
        );
        let key = format!("actor:{role}:_ID");
        let expected = case
            .iter()
            .rev()
            .find(|row| row[0] == "I" && row[1] == key)
            .map(|row| {
                assert_eq!(row[2], "int");
                row[3].parse::<i32>().unwrap()
            })
            .unwrap_or(ordinal as i32);
        assert_eq!(
            graph.int(id, ORGANISM, "_ID").unwrap(),
            expected,
            "Explicit actor ID {role}"
        );
        assert_eq!(
            actors.values().filter(|other| **other == id).count(),
            1,
            "Distinct actor {role}"
        );
    }
    assert_eq!(
        graph
            .arena()
            .nodes
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                graph
                    .class_name(NodeId(u32::try_from(*index).unwrap()))
                    .ok()
                    == Some(ORGANISM)
            })
            .count(),
        3,
        "Complete serialized actors"
    );
    actors
}

pub(super) fn verify_snapshot(case_rows: &[Row<'_>], stage: &str) -> [usize; 5] {
    let rows: Vec<_> = case_rows
        .iter()
        .filter(|row| row.len() > 1 && row[1] == stage)
        .collect();
    let capture = one(&rows, "Z", None);
    let file = Path::new(capture[2]);
    assert_eq!(file.components().count(), 1);
    assert_eq!(file.extension().unwrap(), "ser");
    let path = crate::oracle_tests::fixture_path(super::DIRECTORY).join(file);
    let metadata = std::fs::symlink_metadata(&path).unwrap();
    assert!(metadata.is_file() && metadata.len() <= limits().input_bytes as u64);
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(bytes.len(), capture[3].parse::<usize>().unwrap());
    let document = StreamReader::new(&bytes, limits())
        .unwrap()
        .read_document()
        .unwrap();
    assert_eq!(document.consumed, bytes.len());
    let graph = LegacyGraph::new(&document, limits()).unwrap();
    let actors = actors(&graph, case_rows, &rows);
    let mut owners = Owners::default();
    let mut rectangle_count = 0;
    let mut primitive_count = 0;
    let mut color_count = 0;
    for (role, actor) in &actors {
        owners.bind(
            one(&rows, "G", Some(role))[3],
            reference(&graph, *actor, ORGANISM, "_geneticCode"),
        );
    }
    for (role, actor) in &actors {
        let observed_bounds = one(&rows, "B", Some(role));
        assert_eq!(bounds(&graph, *actor).as_slice(), &observed_bounds[3..]);
        let rectangle_rows: Vec<_> = rows
            .iter()
            .filter(|row| row[0] == "RA" && row[2] == *role)
            .collect();
        assert_eq!(rectangle_rows.len(), 3);
        for row in rectangle_rows {
            let value = if row[3] == "this" {
                Value::Handle(*actor)
            } else {
                reference(&graph, *actor, ORGANISM, row[3])
            };
            owners.bind(row[4], value);
            match value {
                Value::Null => assert_eq!(&row[5..], &["~"; 5]),
                Value::Handle(id) => {
                    assert_eq!(graph.class_name(id).unwrap(), row[5]);
                    assert_eq!(bounds(&graph, id).as_slice(), &row[6..]);
                }
            }
            rectangle_count += 1;
        }
        let primitive_rows: Vec<_> = rows
            .iter()
            .filter(|row| row[0] == "PA" && row[2] == *role)
            .collect();
        assert_eq!(primitive_rows.len(), 48);
        for row in primitive_rows {
            let value = reference(&graph, *actor, ORGANISM, row[3]);
            owners.bind(row[5], value);
            match value {
                Value::Null => assert_eq!(row[6], "~"),
                Value::Handle(id) => {
                    let Node::Array(array) = graph.node(id).unwrap() else {
                        panic!("Primitive array");
                    };
                    let (kind, length) = match &array.data {
                        ArrayData::Ints(values) => ("int[]", values.len()),
                        ArrayData::Doubles(values) => ("double[]", values.len()),
                        _ => panic!("Captured primitive array type"),
                    };
                    assert_eq!(row[4], kind);
                    assert_eq!(row[6].parse::<usize>().unwrap(), length);
                }
            }
            primitive_count += 1;
        }
        let array_row = one(&rows, "AR", Some(role));
        let colors = reference(&graph, *actor, ORGANISM, "_segColor");
        owners.bind(array_row[3], colors);
        let color_rows: Vec<_> = rows
            .iter()
            .filter(|row| row[0] == "AC" && row[2] == *role)
            .collect();
        match colors {
            Value::Null => {
                assert_eq!(array_row[4], "~");
                assert!(color_rows.is_empty());
            }
            Value::Handle(id) => {
                let Node::Array(array) = graph.node(id).unwrap() else {
                    panic!("Color array");
                };
                let ArrayData::References(values) = &array.data else {
                    panic!("Color reference slots");
                };
                assert_eq!(values.len(), array_row[4].parse::<usize>().unwrap());
                assert_eq!(values.len(), color_rows.len());
                for (index, value) in values.iter().copied().enumerate() {
                    let selected: Vec<_> = color_rows
                        .iter()
                        .filter(|row| row[3].parse::<usize>().unwrap() == index)
                        .collect();
                    assert_eq!(selected.len(), 1);
                    color(&graph, &mut owners, value, selected[0][4], selected[0][5]);
                    color_count += 1;
                }
            }
        }
        for tag in ["O", "Q"] {
            let fields: Vec<_> = rows
                .iter()
                .filter(|row| row[0] == tag && row[2] == *role)
                .collect();
            assert_eq!(fields.len(), if tag == "O" { 53 } else { 36 });
            for row in fields {
                match row[3] {
                    "_segColor" => owners.bind(row[4], colors),
                    "_color" => color_text(
                        &graph,
                        &mut owners,
                        reference(&graph, *actor, ORGANISM, "_color"),
                        row[4],
                    ),
                    _ => compare_field(&graph, &mut owners, *actor, ORGANISM, row[3], row[4]),
                }
            }
        }
        let genome = handle(reference(&graph, *actor, ORGANISM, "_geneticCode"));
        assert_eq!(
            graph
                .boolean(genome, "biogenesis.GeneticCode", "_mimicall")
                .unwrap()
                .to_string(),
            one(&rows, "GM", Some(role))[3]
        );
        let gene_array = handle(reference(
            &graph,
            genome,
            "biogenesis.GeneticCode",
            "_genes",
        ));
        let Node::Array(gene_array) = graph.node(gene_array).unwrap() else {
            panic!("Gene array");
        };
        let ArrayData::References(genes) = &gene_array.data else {
            panic!("Gene reference slots");
        };
        let gene_rows: Vec<_> = rows
            .iter()
            .filter(|row| row[0] == "N" && row[2] == *role)
            .collect();
        assert_eq!(genes.len(), gene_rows.len());
        for (index, gene) in genes.iter().copied().enumerate() {
            let selected: Vec<_> = gene_rows
                .iter()
                .filter(|row| row[3].parse::<usize>().unwrap() == index)
                .collect();
            assert_eq!(selected.len(), 1);
            color_text(
                &graph,
                &mut owners,
                reference(&graph, handle(gene), "biogenesis.Gene", "_color"),
                selected[0][4],
            );
        }
    }
    let world = graph.world_root().unwrap();
    let statistics = handle(reference(&graph, world, WORLD, "worldStatistics"));
    for (tag, id, owner, count) in [("W", world, WORLD, 8), ("S", statistics, STATISTICS, 52)] {
        let fields: Vec<_> = rows.iter().filter(|row| row[0] == tag).collect();
        assert_eq!(fields.len(), count);
        for row in fields {
            compare_field(&graph, &mut owners, id, owner, row[2], row[3]);
        }
    }
    assert_eq!(
        encode_document(&document, limits()).unwrap(),
        bytes,
        "Complete captured wire roundtrip {}",
        capture[2]
    );
    [
        1,
        actors.len(),
        rectangle_count,
        primitive_count,
        color_count,
    ]
}
