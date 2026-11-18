// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::object_stream::{StreamReader, encode_document};

const OWNED: usize = 16 * 1024 * 1024;
const ALIASES: &[u8] = include_bytes!("../tests/fixtures/color-stream/color-aliases/input.ser");
const SPACE_INPUT: &[u8] = include_bytes!("../tests/fixtures/color-stream/space-srgb/input.ser");

fn decode(bytes: &[u8]) -> Document {
    StreamReader::new(bytes, Limits::default()).unwrap().read_document().unwrap()
}

fn root(document: &Document) -> Value {
    let [Record::Value(root)] = document.records.as_slice() else { panic!("One captured root"); };
    *root
}

fn field_position(document: &Document, class: &str, field: &str) -> (NodeId, usize, usize) {
    for (node_index, node) in document.arena.nodes.iter().enumerate() {
        let Node::Object(object) = node else { continue };
        for (slot_index, slot) in object.slots.iter().enumerate() {
            let Node::Descriptor(descriptor) = &document.arena.nodes[slot.descriptor.0 as usize] else { continue };
            let schema = class_schema(descriptor.class);
            if schema.name == class {
                if let Some(index) = schema.fields.iter().position(|value| value.name == field) {
                    return (NodeId(node_index as u32), slot_index, index);
                }
            }
        }
    }
    panic!("Missing captured field")
}

fn set_field(document: &mut Document, class: &str, field: &str, value: FieldValue) {
    let (id, slot, field) = field_position(document, class, field);
    let Node::Object(object) = &mut document.arena.nodes[id.0 as usize] else { unreachable!() };
    object.slots[slot].values[field] = value;
}

#[test]
fn typed_owners_reemit_all_33_color_and_7_float_control_streams() {
    let directory = crate::oracle_tests::fixture_path("crates/core/tests/fixtures/color-stream");
    let mut color_count = 0;
    let mut float_count = 0;
    for row in include_str!("../tests/fixtures/color-stream/cases.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        let input = std::fs::read(directory.join(fields[0])).unwrap();
        let expected = std::fs::read(directory.join(fields[1])).unwrap();
        let document = decode(&input);
        let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
        let mut stage = ColorStager::new(&graph, OWNED).unwrap();
        let Value::Handle(id) = root(&document) else { panic!("Captured nonnull root"); };
        let emitted = match graph.class_name(id).unwrap() {
            COLOR => {
                color_count += 1;
                let value = stage.color(root(&document)).unwrap();
                emit_color_document(ColorRoot::Color(value.as_ref()), Limits::default(), OWNED).unwrap()
            }
            COLORS => {
                color_count += 1;
                let values = stage.colors(root(&document)).unwrap().unwrap();
                emit_color_document(ColorRoot::Colors(Some(&values)), Limits::default(), OWNED).unwrap()
            }
            FLOATS => {
                float_count += 1;
                let value = stage.floats(root(&document)).unwrap();
                if fields[0].starts_with("raw-float-array/") && !fields[0].ends_with("descriptor-control.ser") {
                    let column = if fields[0].ends_with("input.ser") { 1 } else { 2 };
                    let bits: Vec<u32> = include_str!("../tests/fixtures/color-stream/raw-float-bits.tsv")
                        .lines().map(|row| u32::from_str_radix(row.split('\t').nth(column).unwrap(), 16).unwrap()).collect();
                    assert_eq!(bits.len(), 12);
                    assert_eq!(value.as_ref().unwrap().bits(), bits);
                }
                emit_color_document(ColorRoot::Floats(value.as_ref()), Limits::default(), OWNED).unwrap()
            }
            _ => panic!("Unexpected captured root"),
        };
        assert_eq!(encode_document(&emitted, Limits::default()).unwrap(), expected, "{}", fields[0]);
    }
    assert_eq!((color_count, float_count), (33, 7));
}

#[test]
fn staged_aliases_charge_unique_payloads_once_and_outlive_the_input() {
    let retained = {
        let document = decode(ALIASES);
        let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
        let mut stage = ColorStager::new(&graph, OWNED).unwrap();
        let values = stage.colors(root(&document)).unwrap().unwrap();
        let slots = values.slots();
        assert!(slots[0].as_ref().unwrap().same_instance(slots[1].as_ref().unwrap()));
        assert_eq!(slots[0], slots[2]);
        assert!(!slots[0].as_ref().unwrap().same_instance(slots[2].as_ref().unwrap()));
        assert!(slots[3].is_none());
        assert!(slots[4].as_ref().unwrap().same_instance(slots[5].as_ref().unwrap()));
        assert!(!slots[4].as_ref().unwrap().same_instance(slots[6].as_ref().unwrap()));
        let float_color = slots[4].as_ref().unwrap();
        assert!(float_color.rgb_components().unwrap().same_instance(float_color.components().unwrap()));
        assert!(slots[7].as_ref().unwrap().same_instance(slots[9].as_ref().unwrap()));
        assert!(!slots[7].as_ref().unwrap().same_instance(slots[8].as_ref().unwrap()));
        let space = slots[7].as_ref().unwrap().color_space().unwrap();
        assert!(space.same_instance(slots[8].as_ref().unwrap().color_space().unwrap()));
        assert!(space.profile().unwrap().same_instance(&JavaIccProfile::builtin(space.profile().unwrap().kind())));
        let used = stage.allocated_bytes();
        let Value::Handle(id) = root(&document) else { unreachable!() };
        let Node::Array(array) = graph.node(id).unwrap() else { unreachable!() };
        let ArrayData::References(references) = &array.data else { unreachable!() };
        let again = stage.color(references[4]).unwrap().unwrap();
        assert!(again.same_instance(float_color));
        assert_eq!(stage.allocated_bytes(), used);
        drop(slots);
        values
    };
    let emitted = emit_color_document(ColorRoot::Colors(Some(&retained)), Limits::default(), OWNED).unwrap();
    assert!(!encode_document(&emitted, Limits::default()).unwrap().is_empty());
    let retained_slots = retained.slots();
    assert!(retained_slots[4].as_ref().unwrap().rgb_components().unwrap().same_instance(retained_slots[4].as_ref().unwrap().components().unwrap()));
}

#[test]
fn space_read_object_resolves_only_null_profile_and_preserves_raw_neighbors() {
    let mut document = decode(SPACE_INPUT);
    let (root_id, _, _) = field_position(&document, COLOR, "cs");
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    let cache = graph.reference(root_id, COLOR, "frgbvalue").unwrap();
    set_field(&mut document, COLOR, "falpha", FieldValue::FloatBits(0xffa0_0001));
    set_field(&mut document, SPACE_BASE, "numComponents", FieldValue::Int(i32::MIN));
    set_field(&mut document, SPACE_BASE, "type", FieldValue::Int(-17));
    set_field(&mut document, SPACE, "needScaleInit", FieldValue::Boolean(0));
    set_field(&mut document, SPACE, "diffMinMax", FieldValue::Reference(cache));
    set_field(&mut document, SPACE, "invDiffMinMax", FieldValue::Reference(cache));
    set_field(&mut document, SPACE, "maxVal", FieldValue::Reference(Value::Null));
    set_field(&mut document, SPACE, "minVal", FieldValue::Reference(Value::Null));
    set_field(&mut document, PROFILE, "iccProfileSerializedDataVersion", FieldValue::Int(i32::MIN));
    {
        let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
        let mut stage = ColorStager::new(&graph, OWNED).unwrap();
        let color = stage.color(root(&document)).unwrap().unwrap();
        assert!(color.color_space().unwrap().profile().unwrap()
            .same_instance(&JavaIccProfile::builtin(NamedIccProfile::Srgb)));
    }
    set_field(&mut document, SPACE, "thisProfile", FieldValue::Reference(Value::Null));
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    let mut stage = ColorStager::new(&graph, OWNED).unwrap();
    let color = stage.color(root(&document)).unwrap().unwrap();
    assert_eq!(color.alpha_bits(), 0xffa0_0001);
    let space = color.color_space().unwrap();
    assert_eq!(space.num_components(), i32::MIN);
    assert_eq!(space.space_type(), -17);
    assert!(!space.need_scale_init());
    assert!(space.max_val().is_none() && space.min_val().is_none());
    assert!(space.profile().unwrap().same_instance(&JavaIccProfile::builtin(NamedIccProfile::Srgb)));
    assert!(space.diff_min_max().unwrap().same_instance(space.inv_diff_min_max().unwrap()));
    assert!(space.diff_min_max().unwrap().same_instance(color.rgb_components().unwrap()));
    let emitted = emit_color_document(ColorRoot::Color(Some(&color)), Limits::default(), OWNED).unwrap();
    let serialized = encode_document(&emitted, Limits::default()).unwrap();
    let decoded = decode(&serialized);
    let graph = LegacyGraph::new(&decoded, Limits::default()).unwrap();
    let mut stage = ColorStager::new(&graph, OWNED).unwrap();
    let written = stage.color(root(&decoded)).unwrap().unwrap();
    assert_eq!(written.alpha_bits(), 0x7fc0_0000);
    assert!(written.color_space().unwrap().profile().unwrap()
        .same_instance(&JavaIccProfile::builtin(NamedIccProfile::Srgb)));
    assert_eq!(color.alpha_bits(), 0xffa0_0001);
    assert_eq!(written.color_space().unwrap().num_components(), i32::MIN);
}

#[test]
fn typed_staging_and_emission_enforce_separate_budgets_and_field_errors() {
    let document = decode(ALIASES);
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    assert!(matches!(ColorStager::new(&graph, 0), Err(GraphError { kind: GraphErrorKind::Budget, .. })));
    let mut stage = ColorStager::new(&graph, OWNED).unwrap();
    let values = stage.colors(root(&document)).unwrap().unwrap();
    assert_eq!(emit_color_document(ColorRoot::Colors(Some(&values)), Limits::default(), 0).unwrap_err().kind, GraphErrorKind::Budget);
    assert_eq!(emit_color_document(ColorRoot::Colors(Some(&values)), Limits { array_elements: 9, ..Limits::default() }, OWNED).unwrap_err().kind, GraphErrorKind::Budget);

    let mut invalid = decode(SPACE_INPUT);
    let root_value = root(&invalid);
    set_field(&mut invalid, COLOR, "cs", FieldValue::Reference(root_value));
    let graph = LegacyGraph::new(&invalid, Limits::default()).unwrap();
    let mut stage = ColorStager::new(&graph, OWNED).unwrap();
    let error = stage.color(root(&invalid)).unwrap_err();
    assert_eq!(error.kind, GraphErrorKind::Class);
    assert_eq!(error.field, Some("cs"));

    let mut invalid = decode(SPACE_INPUT);
    let (id, _, _) = field_position(&invalid, PROFILE, "iccProfileSerializedDataVersion");
    let Node::Object(profile) = &mut invalid.arena.nodes[id.0 as usize] else { unreachable!() };
    profile.slots[0].custom.clear();
    let graph = LegacyGraph::new(&invalid, Limits::default()).unwrap();
    let mut stage = ColorStager::new(&graph, OWNED).unwrap();
    let error = stage.color(root(&invalid)).unwrap_err();
    assert_eq!(error.kind, GraphErrorKind::ContainerData);
    assert_eq!(error.node, Some(id));
    assert_eq!(error.field, Some("custom"));
}


#[test]
fn array_staging_and_batched_export_preserve_mutable_alias_topology() {
    let retained = {
        let document = decode(ALIASES);
        let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
        let mut stage = ColorStager::new(&graph, OWNED).unwrap();
        let first = stage.colors(root(&document)).unwrap().unwrap();
        let used = stage.allocated_bytes();
        let again = stage.colors(root(&document)).unwrap().unwrap();
        assert!(first.same_instance(&again));
        assert_eq!(used, stage.allocated_bytes());
        first
    };
    let gene_color = retained.get(4).unwrap().unwrap();
    let distinct = ColorArrayRef::from_slots(vec![Some(gene_color.clone()), None]);
    let emitted = emit_color_documents(&[
        ColorRoot::Colors(Some(&retained)),
        ColorRoot::Colors(Some(&retained)),
        ColorRoot::Colors(Some(&distinct)),
        ColorRoot::Color(Some(&gene_color)),
        ColorRoot::Colors(None),
    ], Limits::default(), OWNED).unwrap();
    let values: Vec<_> = emitted.records.iter().map(|record| {
        let Record::Value(value) = record else { panic!("Value root"); };
        *value
    }).collect();
    assert_eq!(values[0], values[1]);
    assert_ne!(values[0], values[2]);
    assert_eq!(values[4], Value::Null);
    let graph = LegacyGraph::new(&emitted, Limits::default()).unwrap();
    let mut stage = ColorStager::new(&graph, OWNED).unwrap();
    let first = stage.colors(values[0]).unwrap().unwrap();
    let again = stage.colors(values[1]).unwrap().unwrap();
    let second = stage.colors(values[2]).unwrap().unwrap();
    let gene = stage.color(values[3]).unwrap().unwrap();
    assert!(stage.colors(values[4]).unwrap().is_none());
    assert!(first.same_instance(&again));
    assert!(!first.same_instance(&second));
    assert!(first.get(4).unwrap().unwrap().same_instance(&gene));
    assert!(second.get(0).unwrap().unwrap().same_instance(&gene));
    assert!(gene.rgb_components().unwrap().same_instance(gene.components().unwrap()));
    first.replace(4, None).unwrap();
    assert!(again.get(4).unwrap().is_none());
    assert!(second.get(0).unwrap().unwrap().same_instance(&gene));
}
