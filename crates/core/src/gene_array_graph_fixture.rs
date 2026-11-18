// SPDX-License-Identifier: GPL-2.0-or-later
//! Inspect complete captured wire graphs without hydrating their World state
use super::{DIRECTORY, Row};
use crate::legacy_genetics::{GeneticsStager, export_genomes};
use crate::legacy_graph::LegacyGraph;
use crate::object_stream::{
    ArrayData, FieldValue, Limits, Node, NodeId, Record, StreamReader, Value, encode_document,
};
use std::collections::BTreeMap;
fn limits() -> Limits {
    Limits {
        input_bytes: 262144,
        output_bytes: 262144,
        handles: 4096,
        array_elements: 4096,
        total_array_elements: 65536,
        records: 8192,
        allocated_bytes: 16 << 20,
        work: 8_000_000,
        ..Limits::default()
    }
}
fn bind(owners: &mut BTreeMap<String, NodeId>, label: &str, value: Value) {
    if label == "~" {
        assert_eq!(value, Value::Null);
        return;
    }
    let Value::Handle(id) = value else {
        panic!("Nonnull captured reference {label}")
    };
    if let Some(previous) = owners.get(label) {
        assert_eq!(*previous, id);
    } else {
        assert!(
            owners
                .iter()
                .all(|(other, previous)| other == label || *previous != id),
            "Distinct wire owners collapsed"
        );
        owners.insert(label.into(), id);
    }
}
fn scalar(actual: FieldValue, expected: &str) {
    match actual {
        FieldValue::Boolean(v) => assert_eq!((v != 0).to_string(), expected),
        FieldValue::Int(v) => assert_eq!(v.to_string(), expected),
        FieldValue::Long(v) => assert_eq!(v.to_string(), expected),
        FieldValue::DoubleBits(v) => assert_eq!(format!("{v:016x}"), expected),
        FieldValue::FloatBits(v) => assert_eq!(format!("{v:08x}"), expected),
        _ => panic!("Unexpected persistent scalar"),
    }
}
pub(super) fn verify(case: &[Row<'_>], stage: &str) {
    let rows: Vec<_> = case
        .iter()
        .filter(|r| r.len() > 1 && r[1] == stage)
        .collect();
    let z = rows.iter().find(|r| r[0] == "Z").unwrap();
    let bytes = std::fs::read(crate::oracle_tests::fixture_path(DIRECTORY).join(z[2])).unwrap();
    assert_eq!(bytes.len(), z[3].parse::<usize>().unwrap());
    let document = StreamReader::new(&bytes, limits())
        .unwrap()
        .read_document()
        .unwrap();
    assert_eq!(document.consumed, bytes.len());
    assert_eq!(encode_document(&document, limits()).unwrap(), bytes);
    let graph = LegacyGraph::new(&document, limits()).unwrap();
    let root = graph.world_root().unwrap();
    let p = rows.iter().find(|r| r[0] == "P").unwrap();
    let mut owners = BTreeMap::from([(p[2].to_string(), root)]);
    loop {
        let previous = owners.len();
        for row in &rows {
            if row[0] == "F" && row[6] == "false" {
                let Some(&owner) = owners.get(row[2]) else {
                    continue;
                };
                // Declaring classes come only from the pinned static stream schema
                let class = crate::object_stream::class_by_name(row[3]).unwrap();
                let schema = crate::object_stream::class_schema(class);
                let field = schema.fields.iter().find(|f| f.name == row[4]).unwrap();
                let actual = graph.field(owner, schema.name, field.name).unwrap();
                if let FieldValue::Reference(value) = actual {
                    bind(&mut owners, row[7], value);
                } else {
                    scalar(actual, row[7]);
                }
            } else if row[0] == "N" {
                let Some(&owner) = owners.get(row[2]) else {
                    continue;
                };
                let index = row[3].parse::<usize>().unwrap();
                match graph.node(owner).unwrap() {
                    Node::Array(array) => match &array.data {
                        ArrayData::References(values) => bind(&mut owners, row[4], values[index]),
                        ArrayData::Ints(values) => assert_eq!(values[index].to_string(), row[4]),
                        ArrayData::Doubles(values) => {
                            assert_eq!(format!("{:016x}", values[index]), row[4])
                        }
                        ArrayData::Floats(values) => {
                            assert_eq!(format!("{:08x}", values[index]), row[4])
                        }
                    },
                    _ => {
                        let class = graph.class_name(owner).unwrap();
                        let values = if class == "java.util.Collections$SynchronizedList" {
                            graph.synchronized_list(owner).unwrap().1
                        } else {
                            graph.array_list(owner).unwrap()
                        };
                        bind(&mut owners, row[4], values[index]);
                    }
                }
            }
        }
        if owners.len() == previous {
            break;
        }
    }
    for row in &rows {
        let Some(&id) = row.get(2).and_then(|label| owners.get(*label)) else {
            continue;
        };
        match row[0] {
            "O" => {
                let class = graph.class_name(id);
                if row[3] == "java.lang.String" {
                    assert!(matches!(graph.node(id).unwrap(), Node::String(_)));
                } else if row[3] == "java.util.Collections$SynchronizedRandomAccessList" {
                    assert_eq!(class.unwrap(), "java.util.Collections$SynchronizedList");
                } else {
                    assert_eq!(class.unwrap(), row[3]);
                }
            }
            "D" => match row[3] {
                "String" => {
                    let units = graph.string_units(id).unwrap();
                    let text = String::from_utf16(units).unwrap();
                    assert_eq!(
                        text.as_bytes()
                            .iter()
                            .map(|v| format!("{v:02x}"))
                            .collect::<String>(),
                        row[4]
                    );
                }
                "ColorARGB" => assert_eq!(
                    format!(
                        "{:08x}",
                        graph.int(id, "java.awt.Color", "value").unwrap() as u32
                    ),
                    row[4]
                ),
                _ => panic!("Unexpected captured immutable payload"),
            },
            "A" => {
                let Node::Array(array) = graph.node(id).unwrap() else {
                    panic!("Expected wire array")
                };
                let len = match &array.data {
                    ArrayData::References(v) => v.len(),
                    ArrayData::Ints(v) => v.len(),
                    ArrayData::Doubles(v) => v.len(),
                    ArrayData::Floats(v) => v.len(),
                };
                assert_eq!(len, row[4].parse::<usize>().unwrap());
            }
            "L" => {
                let values =
                    if graph.class_name(id).unwrap() == "java.util.Collections$SynchronizedList" {
                        graph.synchronized_list(id).unwrap().1
                    } else {
                        graph.array_list(id).unwrap()
                    };
                assert_eq!(values.len(), row[3].parse::<usize>().unwrap());
            }
            _ => {}
        }
    }
    let code_ids: Vec<_> = rows
        .iter()
        .filter(|r| r[0] == "O" && r[3] == "biogenesis.GeneticCode")
        .map(|r| owners[r[2]])
        .collect();
    let mut stager = GeneticsStager::new(&graph, 8 << 20).unwrap();
    let root_values: Vec<_> = code_ids.iter().map(|id| Value::Handle(*id)).collect();
    let codes: Vec<_> = stager
        .genomes(&root_values)
        .unwrap()
        .into_iter()
        .map(Option::unwrap)
        .collect();
    let roots: Vec<_> = codes.iter().map(Some).collect();
    let exported = export_genomes(&roots, limits(), 8 << 20).unwrap();
    let encoded = encode_document(&exported, limits()).unwrap();
    let read = StreamReader::new(&encoded, limits())
        .unwrap()
        .read_document()
        .unwrap();
    let second_graph = LegacyGraph::new(&read, limits()).unwrap();
    let mut second_stager = GeneticsStager::new(&second_graph, 8 << 20).unwrap();
    let second: Vec<_> = read
        .records
        .iter()
        .map(|record| {
            let Record::Value(value) = record else {
                panic!("GeneticCode root")
            };
            second_stager.genome(*value).unwrap().unwrap()
        })
        .collect();
    assert_eq!(codes.len(), second.len());
    for (i, left) in codes.iter().enumerate() {
        assert_eq!(*left.borrow(), *second[i].borrow());
        for (j, right) in codes.iter().enumerate() {
            assert_eq!(
                std::rc::Rc::ptr_eq(left, right),
                std::rc::Rc::ptr_eq(&second[i], &second[j])
            );
            match (
                left.borrow().gene_array(),
                right.borrow().gene_array(),
                second[i].borrow().gene_array(),
                second[j].borrow().gene_array(),
            ) {
                (Some(a), Some(b), Some(c), Some(d)) => {
                    assert_eq!(a.same_instance(b), c.same_instance(d));
                    for (x, slot) in a.slots().iter().enumerate() {
                        for (y, other) in b.slots().iter().enumerate() {
                            let copied = c.get(x as i32).unwrap();
                            let copied_other = d.get(y as i32).unwrap();
                            match (slot, other, copied, copied_other) {
                                (Some(v), Some(w), Some(v2), Some(w2)) => assert_eq!(
                                    std::rc::Rc::ptr_eq(v, w),
                                    std::rc::Rc::ptr_eq(&v2, &w2)
                                ),
                                (None, _, None, _) | (_, None, _, None) => {}
                                _ => panic!("Gene null topology"),
                            }
                        }
                    }
                }
                (None, _, None, _) | (_, None, _, None) => {}
                _ => panic!("Array null topology"),
            }
        }
    }
}
