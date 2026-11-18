// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::legacy_history::HistoryStager;
use crate::object_stream::{
    ArrayData, ClassDescriptor, Document, FieldValue, JavaArray, Limits, Node, NodeId, Record,
    StreamReader, class_by_name, class_schema,
};
use crate::statistics::tests::{Observation, observed_field};
use crate::{JavaRandom, SessionRef, World, WorldConfig, WorldParameters};

const BUDGET: usize = 8 * 1024 * 1024;
const WORLD: &[u8] = include_bytes!("../tests/fixtures/object_stream_codec/world.ser");
const STATISTICS: &[u8] = include_bytes!("../tests/fixtures/object_stream_codec/statistics.ser");
const JAR_STATISTICS: &[u8] = include_bytes!("../../../tests/fixtures/java/statistics.ser");
const SHARED: &[u8] = include_bytes!("../tests/fixtures/history_ownership/shared_99_history-initial.ser");
const NULLS: &[u8] = include_bytes!("../tests/fixtures/history_ownership/null_box_graph-initial.ser");
const NULL_LIST: &[u8] = include_bytes!("../tests/fixtures/history_ownership/late_methane_null-initial.ser");

fn decode(bytes: &[u8]) -> Document {
    let document = StreamReader::new(bytes, Limits::default()).unwrap().read_document().unwrap();
    assert_eq!(document.consumed, bytes.len());
    document
}

fn object(document: &Document, class: &str) -> NodeId {
    document.arena.nodes.iter().enumerate().find_map(|(index, node)| {
        let Node::Object(value) = node else { return None; };
        let Node::Descriptor(descriptor) = &document.arena.nodes[value.descriptor.0 as usize] else {
            panic!("Object descriptor required");
        };
        (class_schema(descriptor.class).name == class).then_some(NodeId(index as u32))
    }).expect("Recorded class required")
}

fn set(document: &mut Document, id: NodeId, name: &str, value: FieldValue) {
    let Node::Object(object) = &document.arena.nodes[id.0 as usize] else { panic!("Object required"); };
    let descriptor = object.descriptor;
    let Node::Descriptor(descriptor) = &document.arena.nodes[descriptor.0 as usize] else {
        panic!("Descriptor required");
    };
    let index = class_schema(descriptor.class).fields.iter().position(|field| field.name == name).unwrap();
    let Node::Object(object) = &mut document.arena.nodes[id.0 as usize] else { unreachable!(); };
    object.slots.last_mut().unwrap().values[index] = value;
}

fn duplicate(document: &mut Document, id: NodeId) -> NodeId {
    let copy = document.arena.nodes[id.0 as usize].clone();
    let id = NodeId(document.arena.nodes.len() as u32);
    document.arena.nodes.push(copy);
    id
}

fn compare_stored_fields(
    graph: &LegacyGraph<'_>,
    id: NodeId,
    statistics: &WorldStatistics,
    genetics: &mut GeneticsStager<'_, '_>,
    histories: &mut HistoryStager<'_, '_>,
) {
    let Node::Object(object) = graph.node(id).unwrap() else { panic!("Object required"); };
    let Node::Descriptor(descriptor) = graph.node(object.descriptor).unwrap() else {
        panic!("Descriptor required");
    };
    let schema = class_schema(descriptor.class);
    assert_eq!(schema.fields.len(), 52);
    for field in schema.fields {
        let expected = graph.field(id, CLASS, field.name).unwrap();
        match (expected, observed_field(statistics, field.name)) {
            (FieldValue::Int(expected), Observation::Int(actual)) => assert_eq!(actual, expected, "{}", field.name),
            (FieldValue::Long(expected), Observation::Long(actual)) => assert_eq!(actual, expected, "{}", field.name),
            (FieldValue::DoubleBits(expected), Observation::Double(actual)) => assert_eq!(actual.to_bits(), expected, "{}", field.name),
            (FieldValue::Reference(expected), Observation::Genome(actual)) => {
                let expected = genetics.genome(expected).unwrap();
                match (expected.as_ref(), actual) {
                    (Some(expected), Some(actual)) => assert!(Rc::ptr_eq(actual, expected), "{}", field.name),
                    (None, None) => {},
                    _ => panic!("Genotype null mismatch: {}", field.name),
                }
            },
            (FieldValue::Reference(expected), Observation::History(actual)) => {
                let expected = histories.list(expected).unwrap();
                match (expected.as_ref(), actual) {
                    (Some(expected), Some(actual)) => assert!(actual.same_instance(expected), "{}", field.name),
                    (None, None) => {},
                    _ => panic!("History null mismatch: {}", field.name),
                }
            },
            _ => panic!("Stored type mismatch: {}", field.name),
        }
    }
}

fn stage(document: &Document) -> StatisticsRef {
    let graph = LegacyGraph::new(document, Limits::default()).unwrap();
    let mut genetics = GeneticsStager::new(&graph, BUDGET).unwrap();
    let mut histories = HistoryStager::new(&graph, BUDGET).unwrap();
    let mut stage = StatisticsStager::new(&graph, BUDGET).unwrap();
    let id = object(document, CLASS);
    let owner = stage.statistics(Value::Handle(id), &mut genetics, &mut histories).unwrap().unwrap();
    compare_stored_fields(&graph, id, &owner.borrow(), &mut genetics, &mut histories);
    let repeated = stage.statistics(Value::Handle(id), &mut genetics, &mut histories).unwrap().unwrap();
    assert!(Rc::ptr_eq(&owner, &repeated));
    assert!(stage.statistics(Value::Null, &mut genetics, &mut histories).unwrap().is_none());
    assert!(stage.allocated_bytes() < BUDGET);
    owner
}

#[test]
fn existing_source_and_jar_streams_assign_every_stored_statistics_slot() {
    for bytes in [WORLD, STATISTICS, JAR_STATISTICS, SHARED, NULLS, NULL_LIST] {
        let owner = {
            let document = decode(bytes);
            stage(&document)
        };
        assert_eq!(owner.borrow().retained_genomes().len(), 9);
    }
}

#[test]
fn raw_signed_scalars_and_ieee_payloads_survive_without_events_or_defaults() {
    let mut document = decode(STATISTICS);
    let id = object(&document, CLASS);
    let Node::Object(value) = &document.arena.nodes[id.0 as usize] else { unreachable!(); };
    let Node::Descriptor(descriptor) = &document.arena.nodes[value.descriptor.0 as usize] else { unreachable!(); };
    let schema = class_schema(descriptor.class);
    let bits = [0x7ff0_0000_0000_0042, 0x8000_0000_0000_0000, 0xfff8_1234_5678_9abc,
        0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000, 1];
    let mut doubles = 0;
    for (index, field) in schema.fields.iter().enumerate() {
        let offset = (index / 2) as i32;
        let value = match field.kind {
            b'I' => FieldValue::Int(if index % 2 == 0 { i32::MIN + offset } else { i32::MAX - offset }),
            b'J' => FieldValue::Long(if index % 2 == 0 { i64::MIN + i64::from(offset) } else { i64::MAX - i64::from(offset) }),
            b'D' => { let value = FieldValue::DoubleBits(bits[doubles]); doubles += 1; value },
            _ => continue,
        };
        set(&mut document, id, field.name, value);
    }
    set(&mut document, id, "time", FieldValue::Long(i64::MIN));
    set(&mut document, id, "populationSum", FieldValue::Long(i64::MAX));
    set(&mut document, id, "maxPopulation", FieldValue::Int(i32::MAX));
    set(&mut document, id, "minPopulation", FieldValue::Int(i32::MIN));
    let owner = stage(&document);
    drop(document);
    let statistics = owner.borrow();
    assert_eq!(statistics.time, i64::MIN);
    assert_eq!(statistics.population_sum, i64::MAX);
    assert_eq!((statistics.max_population, statistics.min_population), (i32::MAX, i32::MIN));
    assert_eq!(doubles, 6);
}

#[test]
fn shared_contexts_keep_null_repeated_and_distinct_equal_owners_after_document_drop() {
    let (first, alias, distinct, external_genome, external_history) = {
        let mut document = decode(STATISTICS);
        let id = object(&document, CLASS);
        let genome = object(&document, "biogenesis.GeneticCode");
        let distinct_genome = duplicate(&mut document, genome);
        let pattern = [Value::Handle(genome), Value::Handle(genome), Value::Handle(distinct_genome), Value::Null];
        let Node::Object(value) = &document.arena.nodes[id.0 as usize] else { unreachable!(); };
        let Node::Descriptor(descriptor) = &document.arena.nodes[value.descriptor.0 as usize] else { unreachable!(); };
        let schema = class_schema(descriptor.class);
        let mut count = 0;
        for field in schema.fields {
            if field.signature == Some("Lbiogenesis/GeneticCode;") {
                set(&mut document, id, field.name, FieldValue::Reference(pattern[count % pattern.len()]));
                count += 1;
            }
        }
        assert_eq!(count, 9);
        let shared_list = {
            let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
            graph.reference(id, CLASS, "populationList").unwrap()
        };
        let Value::Handle(shared_list) = shared_list else { panic!("Recorded history required"); };
        let distinct_list = duplicate(&mut document, shared_list);
        set(&mut document, id, "populationList", FieldValue::Reference(Value::Handle(shared_list)));
        set(&mut document, id, "deathList", FieldValue::Reference(Value::Handle(shared_list)));
        set(&mut document, id, "birthList", FieldValue::Reference(Value::Handle(distinct_list)));
        set(&mut document, id, "methaneList", FieldValue::Reference(Value::Null));
        let other = duplicate(&mut document, id);
        let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
        let mut genetics = GeneticsStager::new(&graph, BUDGET).unwrap();
        let mut histories = HistoryStager::new(&graph, BUDGET).unwrap();
        let external_genome = genetics.genome(Value::Handle(genome)).unwrap().unwrap();
        let external_history = histories.list(Value::Handle(shared_list)).unwrap().unwrap();
        let mut stage = StatisticsStager::new(&graph, BUDGET).unwrap();
        let first = stage.statistics(Value::Handle(id), &mut genetics, &mut histories).unwrap().unwrap();
        let alias = stage.statistics(Value::Handle(id), &mut genetics, &mut histories).unwrap().unwrap();
        let distinct = stage.statistics(Value::Handle(other), &mut genetics, &mut histories).unwrap().unwrap();
        compare_stored_fields(&graph, id, &first.borrow(), &mut genetics, &mut histories);
        (first, alias, distinct, external_genome, external_history)
    };
    assert!(Rc::ptr_eq(&first, &alias));
    assert!(!Rc::ptr_eq(&first, &distinct));
    let statistics = first.borrow();
    assert!(Rc::ptr_eq(statistics.alive_being_most_children.as_ref().unwrap(), &external_genome));
    assert!(Rc::ptr_eq(statistics.alive_being_most_children.as_ref().unwrap(), statistics.alive_being_most_infections.as_ref().unwrap()));
    let equal = statistics.alive_being_most_kills.as_ref().unwrap();
    assert!(!Rc::ptr_eq(equal, &external_genome));
    assert_eq!(*equal.borrow(), *external_genome.borrow());
    let distinct_generation = equal.borrow().generation();
    external_genome.borrow_mut().set_generation(i32::MIN);
    assert_eq!(statistics.alive_being_most_infections.as_ref().unwrap().borrow().generation(), i32::MIN);
    assert_eq!(equal.borrow().generation(), distinct_generation);
    assert!(statistics.being_most_children.is_none());
    assert!(statistics.population_list.as_ref().unwrap().same_instance(&external_history));
    assert!(statistics.death_list.as_ref().unwrap().same_instance(&external_history));
    let independent = statistics.birth_list.as_ref().unwrap();
    assert!(!independent.same_instance(&external_history));
    let bits = |history: &crate::history::HistoryListRef| {
        history.elements().iter().map(|value| value.as_ref().map(|value| value.bits())).collect::<Vec<_>>()
    };
    assert_eq!(bits(independent), bits(&external_history));
    assert!(statistics.methane_list.is_none());
    let before = independent.len();
    external_history.append("populationList", -0.0).unwrap();
    assert_eq!(statistics.death_list.as_ref().unwrap().elements().last().unwrap().as_ref().unwrap().bits(), (-0.0_f64).to_bits());
    assert_eq!(independent.len(), before);
    assert!(distinct.borrow().population_list.as_ref().unwrap().same_instance(&external_history));
}

#[test]
fn detached_histories_preserve_box_nulls_raw_bits_and_aliases() {
    let (owner, external) = {
        let mut document = decode(SHARED);
        let boxes: Vec<_> = document.arena.nodes.iter().enumerate().filter_map(|(index, node)| {
            let Node::Object(object) = node else { return None; };
            let Node::Descriptor(descriptor) = &document.arena.nodes[object.descriptor.0 as usize] else { unreachable!(); };
            (class_schema(descriptor.class).name == "java.lang.Double").then_some(NodeId(index as u32))
        }).collect();
        for (index, id) in boxes.into_iter().enumerate() {
            let bits = if index < 2 || index % 2 == 0 { 0x7ff0_0000_0000_0042 } else { 0x8000_0000_0000_0000 };
            set(&mut document, id, "value", FieldValue::DoubleBits(bits));
        }
        let id = object(&document, CLASS);
        let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
        let mut genetics = GeneticsStager::new(&graph, BUDGET).unwrap();
        let mut histories = HistoryStager::new(&graph, BUDGET).unwrap();
        let external = histories.list(graph.reference(id, CLASS, "populationList").unwrap()).unwrap().unwrap();
        let owner = StatisticsStager::new(&graph, BUDGET).unwrap().statistics(Value::Handle(id), &mut genetics, &mut histories).unwrap().unwrap();
        (owner, external)
    };
    let statistics = owner.borrow();
    for name in crate::legacy_history::HISTORY_FIELDS {
        let Observation::History(Some(history)) = observed_field(&statistics, name) else { panic!("Shared list required"); };
        assert!(history.same_instance(&external));
    }
    let values = external.elements();
    assert!(values.iter().any(Option::is_none));
    assert!(values.iter().flatten().any(|value| value.bits() == 0x7ff0_0000_0000_0042));
    assert!(values.iter().flatten().any(|value| value.bits() == 0x8000_0000_0000_0000));
    let mut repeated = false;
    let mut distinct_equal = false;
    for (index, value) in values.iter().enumerate() {
        if let Some(value) = value {
            for other in values[..index].iter().flatten() {
                repeated |= other.same_instance(value);
                distinct_equal |= !other.same_instance(value) && other.bits() == value.bits();
            }
        }
    }
    assert!(repeated, "Frozen stream retains a repeated Double owner");
    assert!(distinct_equal, "Equal Double payloads retain distinct owners");
}

#[test]
fn separate_graph_contexts_fail_before_statistics_publication() {
    let document = decode(STATISTICS);
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    let other = LegacyGraph::new(&document, Limits::default()).unwrap();
    let mut genetics = GeneticsStager::new(&graph, BUDGET).unwrap();
    let mut foreign_genetics = GeneticsStager::new(&other, BUDGET).unwrap();
    let mut histories = HistoryStager::new(&graph, BUDGET).unwrap();
    let mut foreign_histories = HistoryStager::new(&other, BUDGET).unwrap();
    let id = object(&document, CLASS);
    let mut stage = StatisticsStager::new(&graph, BUDGET).unwrap();
    assert_eq!(stage.statistics(Value::Handle(id), &mut foreign_genetics, &mut histories).unwrap_err().kind, GraphErrorKind::Reference);
    assert_eq!(stage.statistics(Value::Handle(id), &mut genetics, &mut foreign_histories).unwrap_err().kind, GraphErrorKind::Reference);
    let owner = stage.statistics(Value::Handle(id), &mut genetics, &mut histories).unwrap().unwrap();
    compare_stored_fields(&graph, id, &owner.borrow(), &mut genetics, &mut histories);
}

#[test]
fn malformed_slots_and_references_preserve_existing_roots_session_and_statistics() {
    let session = SessionRef::new(WorldParameters::default(), JavaRandom::new(73245));
    let old = World::empty_in_session(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, session.clone());
    let old_handle = old.handle();
    let old_statistics = old.statistics_ref();
    let random = session.random_snapshot();
    let parameters = session.parameters();
    for case in 0..10 {
        let mut document = decode(STATISTICS);
        let id = object(&document, CLASS);
        let invalid = duplicate(&mut document, id);
        let bad = NodeId(u32::MAX);
        let request = match case {
            0 => Value::Handle(object(&document, "biogenesis.GeneticCode")),
            1 => Value::Handle(bad),
            2 => { set(&mut document, invalid, "lastInfectedBeing", FieldValue::Reference(Value::Handle(bad))); Value::Handle(invalid) },
            3 => { set(&mut document, invalid, "methaneList", FieldValue::Reference(Value::Handle(id))); Value::Handle(invalid) },
            4 => { set(&mut document, invalid, "time", FieldValue::Int(7)); Value::Handle(invalid) },
            5 => {
                let Node::Object(value) = &mut document.arena.nodes[invalid.0 as usize] else { unreachable!(); };
                value.slots[0].custom.push(Record::Value(Value::Null));
                Value::Handle(invalid)
            },
            6 => {
                let Node::Object(value) = &mut document.arena.nodes[invalid.0 as usize] else { unreachable!(); };
                value.slots[0].values.pop();
                Value::Handle(invalid)
            },
            7 => {
                let Node::Object(value) = &document.arena.nodes[invalid.0 as usize] else { unreachable!(); };
                let descriptor = value.descriptor;
                let descriptor = duplicate(&mut document, descriptor);
                let Node::Descriptor(value) = &mut document.arena.nodes[descriptor.0 as usize] else { unreachable!(); };
                value.signatures.pop();
                let Node::Object(value) = &mut document.arena.nodes[invalid.0 as usize] else { unreachable!(); };
                value.descriptor = descriptor;
                value.slots[0].descriptor = descriptor;
                Value::Handle(invalid)
            },
            8 => { set(&mut document, invalid, "lastInfectedBeing", FieldValue::Reference(Value::Handle(id))); Value::Handle(invalid) },
            9 => { set(&mut document, invalid, "populationList", FieldValue::Reference(Value::Handle(bad))); Value::Handle(invalid) },
            _ => unreachable!(),
        };
        let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
        let mut genetics = GeneticsStager::new(&graph, BUDGET).unwrap();
        let mut histories = HistoryStager::new(&graph, BUDGET).unwrap();
        let mut stage = StatisticsStager::new(&graph, BUDGET).unwrap();
        let previous = stage.statistics(Value::Handle(id), &mut genetics, &mut histories).unwrap().unwrap();
        let before = previous.borrow().time;
        assert!(stage.statistics(request, &mut genetics, &mut histories).is_err(), "case {case}");
        let retained = stage.statistics(Value::Handle(id), &mut genetics, &mut histories).unwrap().unwrap();
        assert!(Rc::ptr_eq(&previous, &retained));
        assert_eq!(retained.borrow().time, before);
        assert_eq!(old.handle(), old_handle);
        assert!(Rc::ptr_eq(&old.statistics_ref(), &old_statistics));
        assert!(old.session().same_owner(&session));
        assert_eq!(session.random_snapshot(), random);
        assert_eq!(session.parameters(), parameters);
    }
}

#[test]
fn aggregate_byte_work_and_depth_limits_reject_without_a_completed_owner() {
    let document = decode(STATISTICS);
    let id = object(&document, CLASS);
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    assert!(matches!(StatisticsStager::new(&graph, 0), Err(error) if error.kind == GraphErrorKind::Budget));
    for (depth, exhaust_bytes, exhaust_work) in [(false, true, false), (false, false, true), (true, false, false)] {
        let mut limits = Limits::default();
        if depth { limits.depth = 0; }
        let graph = LegacyGraph::new(&document, limits).unwrap();
        let mut genetics = GeneticsStager::new(&graph, BUDGET).unwrap();
        let mut histories = HistoryStager::new(&graph, BUDGET).unwrap();
        let mut stage = StatisticsStager::new(&graph, BUDGET).unwrap();
        if exhaust_bytes { graph.allocate(graph.remaining_allocation()).unwrap(); }
        if exhaust_work { graph.charge(graph.remaining_work()).unwrap(); }
        let error = stage.statistics(Value::Handle(id), &mut genetics, &mut histories).unwrap_err();
        if depth { assert_eq!(error.kind, GraphErrorKind::Slots); }
        else { assert_eq!(error.kind, GraphErrorKind::Budget); }
    }
}

#[test]
fn primitive_and_complete_statistics_staging_share_the_aggregate_element_limit() {
    let mut document = decode(SHARED);
    let statistics_id = object(&document, CLASS);
    let descriptor = NodeId(document.arena.nodes.len() as u32);
    document.arena.nodes.push(Node::Descriptor(ClassDescriptor {
        class: class_by_name("[I").unwrap(), signatures: Vec::new(), annotations: Vec::new(), superclass: None,
    }));
    let array = NodeId(document.arena.nodes.len() as u32);
    document.arena.nodes.push(Node::Array(JavaArray { descriptor, data: ArrayData::Ints(vec![i32::MIN]) }));
    for (total, primitive_first) in [(100, false), (100, true), (99, false), (99, true)] {
        let mut limits = Limits::default();
        limits.total_array_elements = total;
        let graph = LegacyGraph::new(&document, limits).unwrap();
        let mut genetics = GeneticsStager::new(&graph, BUDGET).unwrap();
        let mut histories = HistoryStager::new(&graph, BUDGET).unwrap();
        let mut primitive = crate::legacy_organism_arrays::PrimitiveStager::new(&graph, BUDGET).unwrap();
        let mut statistics = StatisticsStager::new(&graph, BUDGET).unwrap();
        if primitive_first {
            let array = primitive.ints(Value::Handle(array)).unwrap().unwrap();
            let result = statistics.statistics(Value::Handle(statistics_id), &mut genetics, &mut histories);
            if total == 99 {
                assert_eq!(result.unwrap_err().kind, GraphErrorKind::Budget);
                continue;
            }
            let first = result.unwrap().unwrap();
            let repeated = statistics.statistics(Value::Handle(statistics_id), &mut genetics, &mut histories).unwrap().unwrap();
            assert!(Rc::ptr_eq(&first, &repeated));
            assert_eq!(array.len(), 1);
        } else {
            let first = statistics.statistics(Value::Handle(statistics_id), &mut genetics, &mut histories).unwrap().unwrap();
            let repeated = statistics.statistics(Value::Handle(statistics_id), &mut genetics, &mut histories).unwrap().unwrap();
            assert!(Rc::ptr_eq(&first, &repeated));
            assert_eq!(first.borrow().population_list.as_ref().unwrap().len(), 99);
            let result = primitive.ints(Value::Handle(array));
            if total == 99 { assert_eq!(result.unwrap_err().kind, GraphErrorKind::Budget); }
            else { assert_eq!(result.unwrap().unwrap().len(), 1); }
        }
    }
}
