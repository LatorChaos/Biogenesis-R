// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::legacy_graph::LegacyGraph;
use crate::{JavaRandom, WorldConfig, WorldParameters};

#[path = "legacy_jts_streams.rs"]
mod jts_inputs;

fn parse(bytes: &[u8]) -> Document {
    StreamReader::new(bytes, Limits::default()).unwrap().read_document().unwrap()
}
fn caller() -> World {
    World::with_random(WorldConfig { initial_attempts: 0, ..WorldConfig::default() },
        WorldParameters::default(), JavaRandom::new(7224)).unwrap()
}
fn open(bytes: &[u8]) -> World {
    let source = parse(bytes);
    let limits = Limits::default();
    caller().decode_legacy_document(&source, limits, limits.allocated_bytes).unwrap()
}
fn root(document: &Document) -> Value {
    assert_eq!(document.records.len(), 1);
    let Record::Value(value) = document.records[0] else { panic!("World root"); };
    value
}

// The expected graph comes from independently retained Source bytes, not the export field writers
struct Compare<'a> {
    left: LegacyGraph<'a>,
    right: LegacyGraph<'a>,
    forward: HashMap<u32, u32>,
    reverse: HashMap<u32, u32>,
    fields: usize,
}
impl<'a> Compare<'a> {
    fn value(&mut self, left: Value, right: Value) {
        let (left, right) = match (left, right) {
            (Value::Null, Value::Null) => return,
            (Value::Handle(left), Value::Handle(right)) => (left, right),
            _ => panic!("Stored nullness changed"),
        };
        if let Some(previous) = self.forward.insert(left.0, right.0) { assert_eq!(previous, right.0); return; }
        assert!(self.reverse.insert(right.0, left.0).is_none(), "Distinct owners merged");
        match (self.left.node(left).unwrap().clone(), self.right.node(right).unwrap().clone()) {
            (Node::String(left), Node::String(right)) => assert_eq!(left.units, right.units),
            (Node::Array(left_array), Node::Array(right_array)) => {
                assert_eq!(self.left.class_name(left).unwrap(), self.right.class_name(right).unwrap());
                match (left_array.data, right_array.data) {
                    (ArrayData::References(left), ArrayData::References(right)) => {
                        assert_eq!(left.len(), right.len());
                        for (left, right) in left.into_iter().zip(right) { self.value(left, right); }
                    }
                    (left, right) => assert_eq!(left, right),
                }
            }
            (Node::Object(left_object), Node::Object(right_object)) => {
                assert_eq!(self.left.class_name(left).unwrap(), self.right.class_name(right).unwrap());
                assert_eq!(left_object.slots.len(), right_object.slots.len());
                for (left_slot, right_slot) in left_object.slots.iter().zip(&right_object.slots) {
                    let Node::Descriptor(desc) = self.left.node(left_slot.descriptor).unwrap() else { panic!("Descriptor"); };
                    let schema = class_schema(desc.class);
                    for field in schema.fields {
                        self.fields += 1;
                        let left = self.left.field(left, schema.name, field.name).unwrap();
                        let right = self.right.field(right, schema.name, field.name).unwrap();
                        match (left, right) {
                            (FieldValue::Reference(left), FieldValue::Reference(right)) => self.value(left, right),
                            (left, right) => assert_eq!(left, right, "{}::{}", schema.name, field.name),
                        }
                    }
                    assert_eq!(left_slot.custom.len(), right_slot.custom.len());
                    for (left, right) in left_slot.custom.iter().zip(&right_slot.custom) {
                        match (left, right) {
                            (Record::Value(left), Record::Value(right)) => self.value(*left, *right),
                            (left, right) => assert_eq!(left, right),
                        }
                    }
                }
            }
            _ => panic!("Stored node type changed"),
        }
    }
}
pub(in crate::world) fn compare(left: &Document, right: &Document) -> usize {
    let mut check = Compare { left: LegacyGraph::new(left, Limits::default()).unwrap(),
        right: LegacyGraph::new(right, Limits::default()).unwrap(),
        forward: HashMap::new(), reverse: HashMap::new(), fields: 0 };
    check.value(root(left), root(right));
    check.fields
}

#[test]
fn legacy_world_export_retained_source_fields_topology_and_wire_bytes() {
    let extra = [
        include_bytes!("../tests/fixtures/object_stream_codec/world.ser").as_slice(),
        include_bytes!("../tests/fixtures/world_hydration/actor_closure-call_0.ser").as_slice(),
        include_bytes!("../tests/fixtures/world_hydration/read_frame-call_0.ser").as_slice(),
    ];
    for bytes in extra.into_iter().chain(jts_inputs::STREAMS.iter().map(|(_, bytes)| *bytes)) {
        let world = open(bytes);
        // open's source Document has dropped before the typed export begins
        let limits = Limits::default();
        let emitted = world.encode_legacy_document(limits, limits.allocated_bytes).unwrap();
        let source = parse(bytes);
        assert!(compare(&source, &emitted) >= 12 + 52);
        let output = world.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap();
        compare(&source, &parse(&output));
        assert_eq!(output, encode_document(&source, limits).unwrap(), "Source wire fidelity");
        assert_eq!(output, world.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap());
        let reopened = caller().decode_legacy_document(&parse(&output), limits, limits.allocated_bytes).unwrap();
        assert_eq!(output, reopened.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap());
    }
}

#[test]
fn legacy_world_export_mutation_aliases_raw_bits_and_no_retention_roots() {
    let world = open(include_bytes!("../tests/fixtures/world_hydration/actor_closure-call_0.ser"));
    let limits = Limits::default();
    let before_retention = world.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap();
    let retention_world = open(include_bytes!("../tests/fixtures/world_hydration/actor_closure-call_0.ser"));
    let retained = Rc::clone(&retention_world.organisms.borrow()[0]);
    retained.borrow_mut().state.world = world.association();
    world.frame_objects.borrow_mut().push(retained);
    assert_eq!(world.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap(), before_retention);
    let actor = Rc::clone(&world.organisms.borrow()[0]);
    let text = JavaText::from_units(vec![0xd800, 0, 0xdc00]);
    let array = crate::DoubleArrayRef::from_bits(vec![0x7ff0000000000001, (-0.0f64).to_bits()]);
    {
        let mut actor = actor.borrow_mut();
        actor.id = i64::from(i32::MIN);
        actor.state.network_position = Some(text.clone());
        actor.genome.as_ref().unwrap().borrow_mut().set_clade_text(Some(text));
        actor.body.arrays.length = Some(array.clone());
        actor.body.arrays.photo = Some(array);
        actor.state.last_frame = Some(actor.state.bounds.clone());
    }
    world.oxygen.set(f64::from_bits(0xfff0000000000000));
    world.frame.set(i32::MIN);
    let before_rng = world.random_snapshot();
    let before_stats = world.statistics_ref();
    let limits = Limits::default();
    let document = world.encode_legacy_document(limits, limits.allocated_bytes).unwrap();
    let graph = LegacyGraph::new(&document, limits).unwrap();
    let world_node = graph.world_root().unwrap();
    let wrapper = graph.required_reference(world_node, WORLD, "_organisms").unwrap();
    let (_, members) = graph.synchronized_list(wrapper).unwrap();
    let Value::Handle(node) = members[0] else { panic!("Actor"); };
    assert_eq!(graph.int(node, ACTOR, "_ID").unwrap(), i32::MIN);
    assert_eq!(graph.reference(node, ACTOR, "lastFrame").unwrap(), Value::Handle(node));
    assert_eq!(graph.reference(node, ACTOR, "_m").unwrap(), graph.reference(node, ACTOR, "_mphoto").unwrap());
    let genome = graph.required_reference(node, ACTOR, "_geneticCode").unwrap();
    assert_eq!(graph.reference(node, ACTOR, "_nPosition").unwrap(), graph.reference(genome, "biogenesis.GeneticCode", "_cladeID").unwrap());
    let raw = graph.required_reference(node, ACTOR, "_m").unwrap();
    let Node::Array(JavaArray { data: ArrayData::Doubles(bits), .. }) = graph.node(raw).unwrap() else { panic!("Array"); };
    assert_eq!(bits, &[0x7ff0000000000001, (-0.0f64).to_bits()]);
    let bytes = world.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap();
    let wire = parse(&bytes);
    let canonical = wire.arena.nodes.iter().find_map(|node| match node {
        Node::Array(JavaArray { data: ArrayData::Doubles(bits), .. }) if bits.len() == 2 && bits[1] == (-0.0f64).to_bits() => Some(bits),
        _ => None,
    }).unwrap();
    assert_eq!(canonical[0], 0x7ff8000000000000);
    assert_eq!(world.random_snapshot(), before_rng);
    assert!(Rc::ptr_eq(&before_stats, &world.statistics_ref()));
}

#[test]
fn legacy_world_export_independent_limits_and_bad_references_do_not_publish() {
    let world = open(include_bytes!("../tests/fixtures/world_hydration/actor_closure-call_0.ser"));
    let limits = Limits::default();
    let baseline = world.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap();
    for limited in [Limits { work: 0, ..limits }, Limits { handles: 1, ..limits },
        Limits { depth: 1, ..limits }, Limits { records: 0, ..limits },
        Limits { allocated_bytes: 1, ..limits }, Limits { output_bytes: 1, ..limits }] {
        assert!(world.encode_legacy_bytes(limited, limited.allocated_bytes).is_err());
    }
    let (document, usage) = WorldEmitter::new(limits, limits.allocated_bytes).finish(&world).unwrap();
    assert!(encode_document(&document, limits).is_ok());
    assert!(matches!(encode_document_charged(&document, Limits { work: usage.work, ..limits }, usage.allocated, usage.work),
        Err(StreamError { kind: ErrorKind::Limit(LimitKind::Work), .. })));
    assert!(matches!(encode_document_charged(&document, Limits { allocated_bytes: usage.allocated, ..limits }, usage.allocated, usage.work),
        Err(StreamError { kind: ErrorKind::Limit(LimitKind::Allocation), .. })));
    assert_eq!(world.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap(), baseline);
    let actor = Rc::clone(&world.organisms.borrow()[0]);
    let previous = actor.borrow().state.world.clone();
    actor.borrow_mut().state.world = Default::default();
    let nullable = world.encode_legacy_document(limits, limits.allocated_bytes).unwrap();
    let reopened = caller().decode_legacy_document(&nullable, limits, limits.allocated_bytes).unwrap();
    assert!(!reopened.organisms.borrow()[0].borrow().state.world.is_present());
    actor.borrow_mut().state.world = previous.clone();
    let expired = caller();
    actor.borrow_mut().state.world = expired.association();
    drop(expired);
    assert!(matches!(world.encode_legacy_document(limits, limits.allocated_bytes),
        Err(WorldExportError::Graph(GraphError { kind: GraphErrorKind::Reference, .. }))));
    actor.borrow_mut().state.world = previous;
    let last_frame = actor.borrow().state.last_frame.clone();
    actor.borrow_mut().state.last_frame = Some(RectangleRef::actor(crate::IntRect::default()));
    assert!(matches!(world.encode_legacy_document(limits, limits.allocated_bytes),
        Err(WorldExportError::Graph(GraphError { kind: GraphErrorKind::Reference, .. }))));
    actor.borrow_mut().state.last_frame = last_frame;
    actor.borrow_mut().id = i64::from(i32::MAX) + 1;
    assert!(matches!(world.encode_legacy_document(limits, limits.allocated_bytes),
        Err(WorldExportError::Graph(GraphError { kind: GraphErrorKind::Field, .. }))));
}

#[test]
fn legacy_world_export_shared_history_boxes_and_distinct_equal_lists() {
    let world = open(include_bytes!("../tests/fixtures/world_hydration/actor_closure-call_0.ser"));
    let number = JavaDoubleRef::from_bits((-0.0f64).to_bits());
    let equal = JavaDoubleRef::from_bits((-0.0f64).to_bits());
    let history = HistoryListRef::from_elements(vec![Some(number.clone()), None, Some(number), Some(equal)]);
    let distinct = HistoryListRef::from_elements(history.elements().clone());
    {
        let statistics = world.statistics_ref();
        let mut statistics = statistics.borrow_mut();
        statistics.population_list = Some(history.clone());
        statistics.death_list = Some(history);
        statistics.birth_list = Some(distinct);
    }
    let limits = Limits::default();
    let document = world.encode_legacy_document(limits, limits.allocated_bytes).unwrap();
    let graph = LegacyGraph::new(&document, limits).unwrap();
    let statistics = graph.required_reference(graph.world_root().unwrap(), WORLD, "worldStatistics").unwrap();
    let population = graph.required_reference(statistics, "biogenesis.WorldStatistics", "populationList").unwrap();
    assert_eq!(graph.reference(statistics, "biogenesis.WorldStatistics", "deathList").unwrap(), Value::Handle(population));
    assert_ne!(graph.reference(statistics, "biogenesis.WorldStatistics", "birthList").unwrap(), Value::Handle(population));
    let slots = graph.array_list(population).unwrap();
    assert_eq!(slots[0], slots[2]);
    assert_eq!(slots[1], Value::Null);
    assert_ne!(slots[0], slots[3]);
    let bytes = world.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap();
    compare(&document, &parse(&bytes));
}

#[test]
fn legacy_world_export_color_genetics_float_and_profile_identity_join() {
    let world = open(include_bytes!("../tests/fixtures/world_hydration/actor_closure-call_0.ser"));
    let actor = Rc::clone(&world.organisms.borrow()[0]);
    let floats = JavaFloatArray::from_bits(vec![0x80000000, 0x3f800000]);
    let space = JavaIccColorSpace::from_parts(crate::java_color::IccSpaceData {
        num_components: -7, space_type: 5, need_scale_init: true,
        diff_min_max: Some(floats.clone()), inv_diff_min_max: Some(floats.clone()),
        max_val: None, min_val: Some(floats.clone()),
        profile: Some(JavaIccProfile::builtin(crate::NamedIccProfile::Srgb)),
    });
    let color = JavaColor::from_parts(crate::java_color::ColorData {
        argb: 0x81234567, alpha_bits: 0x80000000, space: Some(space),
        rgb: Some(floats.clone()), components: Some(floats),
    });
    let equal = JavaColor::from_argb(color.argb());
    {
        let mut actor = actor.borrow_mut();
        actor.state.temporary_color = Some(color.clone());
        actor.body.phenotype_colors = Some(ColorArrayRef::from_slots(vec![Some(color.clone()), Some(color.clone()), Some(equal)]));
        actor.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow_mut().set_color_handle(Some(color));
    }
    let limits = Limits::default();
    let document = world.encode_legacy_document(limits, limits.allocated_bytes).unwrap();
    let graph = LegacyGraph::new(&document, limits).unwrap();
    let root = graph.world_root().unwrap();
    let members = graph.required_reference(root, WORLD, "_organisms").unwrap();
    let Value::Handle(actor) = graph.synchronized_list(members).unwrap().1[0] else { panic!("Actor"); };
    let color = graph.required_reference(actor, ACTOR, "_color").unwrap();
    let colors = graph.required_reference(actor, ACTOR, "_segColor").unwrap();
    let Node::Array(JavaArray { data: ArrayData::References(slots), .. }) = graph.node(colors).unwrap() else { panic!("Color array"); };
    assert_eq!(slots[0], Value::Handle(color)); assert_eq!(slots[0], slots[1]); assert_ne!(slots[0], slots[2]);
    let code = graph.required_reference(actor, ACTOR, "_geneticCode").unwrap();
    let genes = graph.required_reference(code, "biogenesis.GeneticCode", "_genes").unwrap();
    let Node::Array(JavaArray { data: ArrayData::References(genes), .. }) = graph.node(genes).unwrap() else { panic!("Genes"); };
    let Value::Handle(gene) = genes[0] else { panic!("Gene"); };
    assert_eq!(graph.reference(gene, "biogenesis.Gene", "_color").unwrap(), Value::Handle(color));
    let floats = graph.reference(color, "java.awt.Color", "frgbvalue").unwrap();
    assert_eq!(floats, graph.reference(color, "java.awt.Color", "fvalue").unwrap());
    let space = graph.required_reference(color, "java.awt.Color", "cs").unwrap();
    assert_eq!(graph.int(space, "java.awt.color.ColorSpace", "numComponents").unwrap(), -7);
    for field in ["diffMinMax", "invDiffMinMax", "minVal"] {
        assert_eq!(graph.reference(space, "java.awt.color.ICC_ColorSpace", field).unwrap(), floats);
    }
    let profile = graph.required_reference(space, "java.awt.color.ICC_ColorSpace", "thisProfile").unwrap();
    assert_eq!(graph.class_name(profile).unwrap(), "java.awt.color.ICC_ProfileRGB");
    assert_eq!(graph.int(profile, "java.awt.color.ICC_Profile", "iccProfileSerializedDataVersion").unwrap(), 1);
    compare(&document, &parse(&world.encode_legacy_bytes(limits, limits.allocated_bytes).unwrap()));
}

#[path = "world_owner_roles_tests.rs"]
mod owner_roles;
