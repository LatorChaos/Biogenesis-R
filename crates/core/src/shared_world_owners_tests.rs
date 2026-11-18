// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::legacy_genetics::GeneticsStager;
use crate::legacy_organism::{stage_actor_batch_in, StagedActorBatch};
use crate::legacy_organism_arrays::PrimitiveStager;
use crate::legacy_shared_world_owners::SharedOwnerStager;
use crate::object_stream::{ArrayData, Document, JavaArray, Node, NodeId, Record, StreamReader, Value, class_schema};
use crate::organism::OrganismCloner;
use crate::{World, WorldConfig};

const WORLD: &str = "biogenesis.World";
const STATS: &str = "biogenesis.WorldStatistics";
const ABSTRACT_TREE: &str = "org.locationtech.jts.index.strtree.AbstractSTRtree";
const ABSTRACT_NODE: &str = "org.locationtech.jts.index.strtree.AbstractNode";
const WORLD_ROWS: &str = include_str!("../tests/fixtures/shared_world_owners/world_lists.tsv");
const SPATIAL_ROWS: &str = include_str!("../tests/fixtures/shared_world_owners/collection_spatial.tsv");
fn fixture(name: &str) -> Document {
    let bytes: &[u8] = match name {
        "member_history-authored" => include_bytes!("../tests/fixtures/shared_world_owners/member_history-authored.ser"),
        "member_history-call_1" => include_bytes!("../tests/fixtures/shared_world_owners/member_history-call_1.ser"),
        "tree_history-authored" => include_bytes!("../tests/fixtures/shared_world_owners/tree_history-authored.ser"),
        "tree_history-call_1" => include_bytes!("../tests/fixtures/shared_world_owners/tree_history-call_1.ser"),
        "member_wrapper_backing-authored" => include_bytes!("../tests/fixtures/shared_world_owners/member_wrapper_backing-authored.ser"),
        "member_wrapper_backing-call_0" => include_bytes!("../tests/fixtures/shared_world_owners/member_wrapper_backing-call_0.ser"),
        "pending_history_build-authored" => include_bytes!("../tests/fixtures/shared_world_owners/pending_history_build-authored.ser"),
        "pending_history_build-call_0" => include_bytes!("../tests/fixtures/shared_world_owners/pending_history_build-call_0.ser"),
        "pending_history_build-call_1" => include_bytes!("../tests/fixtures/shared_world_owners/pending_history_build-call_1.ser"),
        "pending_history_build-call_2" => include_bytes!("../tests/fixtures/shared_world_owners/pending_history_build-call_2.ser"),
        "pending_history_build-call_3" => include_bytes!("../tests/fixtures/shared_world_owners/pending_history_build-call_3.ser"),
        "child_history_cache-authored" => include_bytes!("../tests/fixtures/shared_world_owners/child_history_cache-authored.ser"),
        "child_history_cache-call_0" => include_bytes!("../tests/fixtures/shared_world_owners/child_history_cache-call_0.ser"),
        "child_history_cache-call_1" => include_bytes!("../tests/fixtures/shared_world_owners/child_history_cache-call_1.ser"),
        _ => panic!("Named immutable witness"),
    };
    StreamReader::new(bytes, Limits::default()).unwrap().read_document().unwrap()
}
fn limits(input: &Document) -> Limits { Limits { handles: input.arena.nodes.len() + 32, ..Limits::default() } }
fn nodes(input: &Document, class: &str) -> Vec<NodeId> {
    input.arena.nodes.iter().enumerate().filter_map(|(index, value)| {
        let Node::Object(object) = value else { return None };
        let Node::Descriptor(descriptor) = &input.arena.nodes[object.descriptor.0 as usize] else { panic!("Descriptor") };
        (class_schema(descriptor.class).name == class).then_some(NodeId(index as u32))
    }).collect()
}
fn handle(value: Value) -> NodeId { let Value::Handle(value) = value else { panic!("Reference") }; value }
fn field(input: &Document, id: NodeId, owner: &'static str, name: &'static str) -> Value { LegacyGraph::new(input, limits(input)).unwrap().reference(id, owner, name).unwrap() }
fn root_refs(input: &Document) -> (Value, Value, Value, Value) {
    let graph = LegacyGraph::new(input, limits(input)).unwrap(); let world = graph.world_root().unwrap();
    let statistics = graph.required_reference(world, WORLD, "worldStatistics").unwrap();
    (graph.reference(world, WORLD, "_organisms").unwrap(), graph.reference(statistics, STATS, "populationList").unwrap(), graph.reference(statistics, STATS, "deathList").unwrap(), graph.reference(world, WORLD, "colDetTree").unwrap())
}
fn worlds(input: &Document) -> Vec<(NodeId, WorldHandle)> {
    nodes(input, WORLD).into_iter().map(|id| {
        let world = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap(); (id, world.handle())
    }).collect()
}
fn stage(input: &Document, lists: &[Value], trees: &[Value]) -> OwnerRoots {
    let graph = LegacyGraph::new(input, limits(input)).unwrap(); let worlds = worlds(input);
    let mut genetics = GeneticsStager::new(&graph, graph.limits().allocated_bytes).unwrap();
    let mut primitive = PrimitiveStager::new(&graph, graph.limits().allocated_bytes).unwrap();
    let actor_nodes = nodes(input, "biogenesis.Organism");
    let actors: Option<StagedActorBatch> = if actor_nodes.is_empty() { None } else { Some(stage_actor_batch_in(&graph, &actor_nodes, &worlds, &mut genetics, &mut primitive).unwrap()) };
    let bindings = actors.as_ref().map_or(&[][..], |value| value.actors.as_slice());
    let mut stager = SharedOwnerStager::new(&graph, bindings, &worlds).unwrap();
    assert!(stager.uses_graph(&graph)); stager.roots(lists, trees).unwrap()
}
fn assert_source_exception(rows: &str, case: &str, call: &str, class: &str) {
    let mut selected = false; let mut found = false;
    for line in rows.lines() {
        let row: Vec<_> = line.split('\t').collect();
        if row[0] == "C" { selected = row[1] == case; }
        if selected && row[0] == "X" && row[1] == call { assert_eq!(row[2], class); found = true; }
    }
    assert!(found);
}

#[test]
fn source_list_aliases_preserve_wrappers_compatibility_and_reached_failures() {
    let input = fixture("member_history-authored"); let (members, population, _, _) = root_refs(&input);
    let roots = stage(&input, &[members, population], &[]);
    let member = roots.lists[0].as_ref().unwrap(); let history = roots.lists[1].as_ref().unwrap();
    assert!(member.same_instance(history)); assert_eq!(member.len(), 0);
    let meter = OwnerBudget::new(Limits::default(), 1 << 20);
    history.append_history("populationList", 7.0, &meter).unwrap();
    assert!(matches!(member.actor("_organisms", 0), Err(ListAccessError::WrongClass { index: 0, actual: "java.lang.Double", .. })));
    let post = fixture("member_history-call_1"); let (_, post_population, _, _) = root_refs(&post);
    let expected = stage(&post, &[post_population], &[]);
    assert_eq!(history.numeric_prefix("populationList", 100).unwrap(), expected.lists[0].as_ref().unwrap().numeric_prefix("populationList", 100).unwrap());
    assert_source_exception(WORLD_ROWS, "member_history", "call_2", "java.lang.ClassCastException");

    let input = fixture("member_wrapper_backing-authored"); let (members, population, death, _) = root_refs(&input);
    let roots = stage(&input, &[members, population, death], &[]);
    let member = roots.lists[0].as_ref().unwrap(); let population = roots.lists[1].as_ref().unwrap(); let death = roots.lists[2].as_ref().unwrap();
    assert!(member.same_instance(death)); assert!(!member.same_instance(population)); assert!(member.shares_backing(population));
    assert_eq!(member.wire_class_name(), SYNC_LIST); assert_eq!(member.class_name(), "java.util.Collections$SynchronizedRandomAccessList");
    let graph = LegacyGraph::new(&input, limits(&input)).unwrap(); let world = graph.world_root().unwrap(); let stats = handle(field(&input, world, WORLD, "worldStatistics"));
    population.append_history("populationList", 7.0, &meter).unwrap();
    death.append_history("deathList", f64::from(graph.int(stats, STATS, "deathLastTime").unwrap()), &meter).unwrap();
    assert_eq!(member.len(), 3); assert!(member.actor("_organisms", 0).unwrap().is_some());
    assert!(matches!(member.actor("_organisms", 1), Err(ListAccessError::WrongClass { actual: "java.lang.Double", .. })));
    let post = fixture("member_wrapper_backing-call_0"); let (_, pop, _, _) = root_refs(&post); let expected = stage(&post, &[pop], &[]);
    for index in 1..3 { assert_eq!(population.number("populationList", index).unwrap().unwrap().bits(), expected.lists[0].as_ref().unwrap().number("populationList", index).unwrap().unwrap().bits()); }
    assert_source_exception(SPATIAL_ROWS, "member_wrapper_backing", "call_1", "java.lang.ClassCastException");

    let input = StreamReader::new(include_bytes!("../tests/fixtures/history_ownership/compatibility_integer-modified.ser"), Limits::default()).unwrap().read_document().unwrap();
    let stats = nodes(&input, STATS)[0]; let list = handle(field(&input, stats, STATS, "populationList"));
    let graph = LegacyGraph::new(&input, limits(&input)).unwrap();
    assert!(matches!(graph.array_list(list), Err(GraphError { kind: GraphErrorKind::ContainerData, .. })));
    assert_eq!(graph.array_list_compatible(list).unwrap().len(), 2);
    let mut stager = SharedOwnerStager::new(&graph, &[], &[]).unwrap(); let list = stager.list(Value::Handle(list)).unwrap().unwrap();
    let first = list.number("populationList", 0).unwrap().unwrap(); let second = list.number("populationList", 1).unwrap().unwrap();
    assert!(first.same_instance(&second)); assert_eq!(first.bits(), 0.75_f64.to_bits());
    let boxes = SharedListRef::from_slots((0..100).map(|_| SharedSlot::Double(first.clone())).collect());
    let no_space = OwnerBudget::new(Limits::default(), 0);
    assert_eq!(boxes.append_history("populationList", 9.0, &no_space), Err(ListAccessError::Capacity { field: "populationList" }));
    assert_eq!(boxes.len(), 99);
    let prefix = SharedListRef::from_slots((0..101).map(|index| if index == 100 { SharedSlot::Null } else { SharedSlot::Double(first.clone()) }).collect());
    assert_eq!(prefix.numeric_prefix("populationList", 100).unwrap().len(), 100);
    assert_eq!(prefix.numeric_prefix("populationList", 101), Err(ListAccessError::MissingElement { field: "populationList", index: 100 }));
}

#[test]
fn source_spatial_aliases_preserve_complete_targets_and_raw_cache_snapshots() {
    for name in ["tree_history-authored", "tree_history-call_1", "pending_history_build-authored", "pending_history_build-call_0", "pending_history_build-call_1", "pending_history_build-call_2", "pending_history_build-call_3", "child_history_cache-authored", "child_history_cache-call_0", "child_history_cache-call_1"] {
        let input = fixture(name); let (_, population, _, tree) = root_refs(&input);
        let roots = stage(&input, &[population], &[tree]); let population = roots.lists[0].as_ref().unwrap(); let tree = roots.trees[0].as_ref().unwrap();
        let graph = LegacyGraph::new(&input, limits(&input)).unwrap(); let wire_tree = handle(root_refs(&input).3);
        assert_eq!(tree.value().built, graph.boolean(wire_tree, ABSTRACT_TREE, "built").unwrap());
        assert_eq!(tree.value().node_capacity, graph.int(wire_tree, ABSTRACT_TREE, "nodeCapacity").unwrap());
        assert_eq!(tree.value().pending.is_none(), field(&input, wire_tree, ABSTRACT_TREE, "itemBoundables") == Value::Null);
        assert_eq!(tree.value().root.is_none(), field(&input, wire_tree, ABSTRACT_TREE, "root") == Value::Null);
        if name == "tree_history-authored" || name == "tree_history-call_1" || name == "pending_history_build-authored" || name == "pending_history_build-call_3" {
            assert!(tree.value().pending.as_ref().unwrap().same_instance(population));
        }
        if name == "pending_history_build-call_0" || name == "pending_history_build-call_1" || name == "pending_history_build-call_2" { assert!(tree.value().pending.is_none()); }
        if name == "pending_history_build-call_2" || name == "pending_history_build-call_3" {
            assert_eq!(population.len(), 5); assert!(matches!(population.boundable("itemBoundables", 4), Err(ListAccessError::WrongClass { actual: "java.lang.Double", .. })));
            assert_eq!(population.number("populationList", 4).unwrap().unwrap().value(), 7.0);
        }
        if name == "child_history_cache-call_1" {
            let node = tree.value().root.clone().unwrap();
            let BoundableValue::Node { bounds, children, .. } = &*node.value() else { panic!("Node") };
            assert!(bounds.is_none()); assert!(children.as_ref().unwrap().same_instance(population)); assert_eq!(population.len(), 3);
            assert!(matches!(population.boundable("childBoundables", 2), Err(ListAccessError::WrongClass { actual: "java.lang.Double", .. })));
        }
        if let Some(SharedSlot::Boundable(item)) = population.slots().first() {
            if let BoundableValue::Item { bounds: Some(bounds), item: SharedSlot::Actor(actor) } = &*item.value() {
                assert!(actor.borrow().state.bounds.actor_target().is_some_and(|target| Rc::ptr_eq(&target, actor)));
                let wire_item = handle(graph.array_list_compatible(handle(population_wire(&input))).unwrap()[0]);
                let wire_bounds = handle(field(&input, wire_item, ITEM, "bounds")); let actual = bounds.get();
                assert_eq!([actual.min_x.to_bits(), actual.max_x.to_bits(), actual.min_y.to_bits(), actual.max_y.to_bits()], [graph.double_bits(wire_bounds, "org.locationtech.jts.geom.Envelope", "minx").unwrap(), graph.double_bits(wire_bounds, "org.locationtech.jts.geom.Envelope", "maxx").unwrap(), graph.double_bits(wire_bounds, "org.locationtech.jts.geom.Envelope", "miny").unwrap(), graph.double_bits(wire_bounds, "org.locationtech.jts.geom.Envelope", "maxy").unwrap()]);
            }
        }
    }
    assert_source_exception(WORLD_ROWS, "tree_history", "call_2", "java.lang.ClassCastException");
    assert_source_exception(SPATIAL_ROWS, "pending_history_build", "call_3", "java.lang.ClassCastException");
    assert_source_exception(SPATIAL_ROWS, "child_history_cache", "call_2", "java.lang.ClassCastException");
}
fn population_wire(input: &Document) -> Value { root_refs(input).1 }

#[test]
fn graph_copy_preserves_aliases_and_detaches_mutable_lists_envelopes_and_actors() {
    let number = JavaDoubleRef::from_value(7.0);
    let runtime_list = SharedListRef::from_slots(vec![SharedSlot::Double(number.clone()), SharedSlot::Double(number)]);
    let tiny = OwnerBudget::new(Limits { array_elements: 1, ..Limits::default() }, 1 << 20);
    assert!(SharedOwnerCopier::new(&tiny, &[]).unwrap().list(&runtime_list).is_err());
    let runtime_meter = RuntimeCopyMeter::new();
    runtime_meter.array(Limits::default().array_elements + 1).unwrap();
    let mut runtime_copier = SharedOwnerCopier::new(&runtime_meter, &[]).unwrap();
    let detached = runtime_copier.list(&runtime_list).unwrap();
    assert_eq!(detached.numeric_prefix("populationList", 2).unwrap(), vec![7.0, 7.0]);
    assert!(!detached.same_instance(&runtime_list));
    assert!(detached.number("populationList", 0).unwrap().unwrap().same_instance(&detached.number("populationList", 1).unwrap().unwrap()));
    let input = fixture("pending_history_build-authored"); let (members, population, _, tree) = root_refs(&input);
    let original = stage(&input, &[members, population, population], &[tree]);
    let mut actor_copies = OrganismCloner::default();
    let actor_map: Vec<_> = original.retention.actors.iter().map(|actor| (Rc::clone(actor), actor_copies.copy(actor))).collect();
    let budget = OwnerBudget::new(Limits::default(), 16 << 20);
    let mut copier = SharedOwnerCopier::new(&budget, &actor_map).unwrap();
    let copied = copier.roots(&original, &original.retention.worlds).unwrap(); drop(copier); drop(actor_map); drop(actor_copies);
    let list = copied.lists[1].as_ref().unwrap(); assert!(list.same_instance(copied.lists[2].as_ref().unwrap()));
    assert!(!list.same_instance(original.lists[1].as_ref().unwrap()));
    assert!(copied.trees[0].as_ref().unwrap().value().pending.as_ref().unwrap().same_instance(list));
    let first = list.boundable("itemBoundables", 0).unwrap().unwrap(); let old_first = original.lists[1].as_ref().unwrap().boundable("itemBoundables", 0).unwrap().unwrap();
    let BoundableValue::Item { bounds: Some(bounds), item: SharedSlot::Actor(actor) } = &*first.value() else { panic!("Item") };
    let BoundableValue::Item { bounds: Some(old_bounds), item: SharedSlot::Actor(old_actor) } = &*old_first.value() else { panic!("Item") };
    assert!(!Rc::ptr_eq(actor, old_actor)); assert_eq!(actor.borrow().id, old_actor.borrow().id);
    let mut changed = bounds.get(); changed.min_x = -924.0; bounds.set(changed); assert_ne!(old_bounds.get().min_x, -924.0);
    let meter = OwnerBudget::new(Limits::default(), 1 << 20); list.append_history("populationList", 7.0, &meter).unwrap();
    assert_eq!(copied.lists[2].as_ref().unwrap().len(), 5); assert_eq!(original.lists[1].as_ref().unwrap().len(), 4);
    let wrapper_input = fixture("member_wrapper_backing-authored"); let (members, population, death, _) = root_refs(&wrapper_input); let wrapper = stage(&wrapper_input, &[members, population, death], &[]);
    let mut actors = OrganismCloner::default(); let map: Vec<_> = wrapper.retention.actors.iter().map(|actor| (Rc::clone(actor), actors.copy(actor))).collect();
    let mut copier = SharedOwnerCopier::new(&budget, &map).unwrap(); let detached = copier.roots(&wrapper, &wrapper.retention.worlds).unwrap();
    assert!(detached.lists[0].as_ref().unwrap().same_instance(detached.lists[2].as_ref().unwrap()));
    assert!(detached.lists[0].as_ref().unwrap().shares_backing(detached.lists[1].as_ref().unwrap()));
    assert!(!detached.lists[0].as_ref().unwrap().shares_backing(wrapper.lists[0].as_ref().unwrap()));
}

#[test]
fn logical_root_retirement_and_external_lists_keep_complete_targets_without_cycles() {
    let input = fixture("pending_history_build-authored"); let (_, population, _, tree) = root_refs(&input);
    let mut roots = stage(&input, &[population], &[tree]); drop(input);
    assert_eq!(roots.retained_actors(), 4);
    let weak = Rc::downgrade(&roots.retention.actors[0]);
    let budget = OwnerBudget::new(Limits::default(), 4 << 20);
    let external = roots.external_list(0, &budget).unwrap().unwrap(); assert_eq!(external.retained_actors(), 4);
    let worlds = roots.retention.worlds.clone();
    roots.lists[0] = None; roots.trees[0] = None; roots.refresh_retention(&worlds, &budget).unwrap();
    assert_eq!(roots.retained_actors(), 0); drop(worlds); drop(roots);
    let actor = weak.upgrade().unwrap(); assert!(actor.borrow().state.world.resolve().is_ok()); drop(actor);
    let item = external.list.boundable("itemBoundables", 0).unwrap().unwrap();
    {
        let value = item.value();
        let BoundableValue::Item { item: SharedSlot::Actor(actor), .. } = &*value else { panic!("Item") };
        assert!(actor.borrow().state.world.resolve().is_ok());
    }
    drop(item);
    drop(external); assert!(weak.upgrade().is_none());
}

fn replace_first_slot(input: &mut Document, list: NodeId, value: Value) {
    let Node::Object(object) = &mut input.arena.nodes[list.0 as usize] else { panic!("List") };
    let slot = object.slots.last_mut().unwrap(); let record = slot.custom.iter_mut().find(|record| matches!(record, Record::Value(_))).unwrap();
    *record = Record::Value(value);
}
#[test]
fn independent_malformed_and_resource_cases_preserve_root_state_and_original_errors() {
    let input = fixture("tree_history-authored"); let (_, population, _, tree) = root_refs(&input);
    let graph = LegacyGraph::new(&input, limits(&input)).unwrap(); let mut stage = SharedOwnerStager::new(&graph, &[], &[]).unwrap();
    assert!(stage.list(Value::Null).unwrap().is_none()); assert!(stage.tree(Value::Null).unwrap().is_none());
    let roots = stage.roots(&[population], &[tree]).unwrap(); assert_eq!(roots.retained_actors(), 0);
    let graph = LegacyGraph::new(&input, limits(&input)).unwrap(); graph.allocate(graph.remaining_allocation() - 1).unwrap();
    assert!(matches!(SharedOwnerStager::new(&graph, &[], &[]), Err(GraphError { kind: GraphErrorKind::Budget, .. })));

    let mut bad = fixture("member_history-call_1"); let list = handle(population_wire(&bad));
    let string = bad.arena.nodes.iter().position(|node| matches!(node, Node::String(_))).unwrap(); replace_first_slot(&mut bad, list, Value::Handle(NodeId(string as u32)));
    let graph = LegacyGraph::new(&bad, limits(&bad)).unwrap(); let mut stage = SharedOwnerStager::new(&graph, &[], &[]).unwrap(); let first = stage.list(Value::Handle(list)).err().unwrap();
    assert_eq!(first.kind, GraphErrorKind::Class); assert_eq!(stage.list(Value::Handle(list)).err().unwrap(), first);

    let mut cycle = fixture("child_history_cache-call_1"); let tree = handle(root_refs(&cycle).3); let node = handle(field(&cycle, tree, ABSTRACT_TREE, "root")); let list = handle(field(&cycle, node, ABSTRACT_NODE, "childBoundables"));
    replace_first_slot(&mut cycle, list, Value::Handle(node));
    let graph = LegacyGraph::new(&cycle, limits(&cycle)).unwrap(); let mut stage = SharedOwnerStager::new(&graph, &[], &[]).unwrap();
    assert!(matches!(stage.tree(Value::Handle(tree)), Err(GraphError { kind: GraphErrorKind::TreeShape, .. })));

    let mut bad = fixture("member_history-authored"); let list = handle(population_wire(&bad));
    let Node::Object(object) = &bad.arena.nodes[list.0 as usize] else { unreachable!() }; let descriptor = object.descriptor;
    let Node::Descriptor(value) = &mut bad.arena.nodes[descriptor.0 as usize] else { unreachable!() }; value.signatures.clear();
    let graph = LegacyGraph::new(&bad, limits(&bad)).unwrap(); let mut stage = SharedOwnerStager::new(&graph, &[], &[]).unwrap();
    assert!(matches!(stage.list(Value::Handle(list)), Err(GraphError { kind: GraphErrorKind::Descriptor, .. })));

    let actor_input = fixture("member_wrapper_backing-authored"); let members = root_refs(&actor_input).0;
    let old = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap(); let before = old.clone(); let rng = old.random_snapshot();
    let graph = LegacyGraph::new(&actor_input, limits(&actor_input)).unwrap();
    let world_binding = [(nodes(&actor_input, WORLD)[0], old.handle())];
    let mut stage = SharedOwnerStager::new(&graph, &[], &world_binding).unwrap();
    assert!(matches!(stage.list(members), Err(GraphError { kind: GraphErrorKind::Reference, .. })));
    assert_eq!(old, before); assert_eq!(old.random_snapshot(), rng);

    let mut input = StreamReader::new(include_bytes!("../tests/fixtures/history_ownership/shared_99_history-initial.ser"), Limits::default()).unwrap().read_document().unwrap();
    let class = crate::object_stream::class_by_name("[D").unwrap(); let descriptor = NodeId(input.arena.nodes.len() as u32);
    input.arena.nodes.push(Node::Descriptor(crate::object_stream::ClassDescriptor { class, signatures: Vec::new(), annotations: Vec::new(), superclass: None }));
    let array = NodeId(input.arena.nodes.len() as u32); input.arena.nodes.push(Node::Array(JavaArray { descriptor, data: ArrayData::Doubles(vec![0]) }));
    let stats = nodes(&input, STATS)[0]; let population = field(&input, stats, STATS, "populationList");
    for total in [100, 99] {
        let mut quota = limits(&input); quota.total_array_elements = total;
        let graph = LegacyGraph::new(&input, quota).unwrap(); let mut primitive = PrimitiveStager::new(&graph, quota.allocated_bytes).unwrap(); primitive.doubles(Value::Handle(array)).unwrap();
        let mut stage = SharedOwnerStager::new(&graph, &[], &[]).unwrap();
        if total == 100 { assert_eq!(stage.list(population).unwrap().unwrap().len(), 99); }
        else { assert!(matches!(stage.list(population), Err(GraphError { kind: GraphErrorKind::Budget, .. }))); }
    }
}

#[test]
fn cached_descriptors_keep_instance_and_binding_validation() {
    let mut input = fixture("member_wrapper_backing-authored");
    let first = nodes(&input, "biogenesis.Organism")[0];
    let second = NodeId(input.arena.nodes.len() as u32);
    input.arena.nodes.push(input.arena.nodes[first.0 as usize].clone());
    let world_bindings = worlds(&input);
    let graph = LegacyGraph::new(&input, limits(&input)).unwrap();
    let mut genetics = GeneticsStager::new(&graph, graph.limits().allocated_bytes).unwrap();
    let mut primitive = PrimitiveStager::new(&graph, graph.limits().allocated_bytes).unwrap();
    let batch = stage_actor_batch_in(&graph, &[first, second], &world_bindings, &mut genetics, &mut primitive).unwrap();
    assert!(SharedOwnerStager::new(&graph, &batch.actors, &world_bindings).is_ok());

    let mut bad = input.clone();
    let Node::Object(actor) = &mut bad.arena.nodes[second.0 as usize] else { unreachable!() };
    actor.slots.pop();
    let bad_graph = LegacyGraph::new(&bad, limits(&bad)).unwrap();
    assert!(matches!(SharedOwnerStager::new(&bad_graph, &batch.actors, &world_bindings),
        Err(GraphError { kind: GraphErrorKind::Slots, .. })));

    let mut bad = input.clone();
    let Node::Object(actor) = &mut bad.arena.nodes[second.0 as usize] else { unreachable!() };
    actor.slots.last_mut().unwrap().custom.push(Record::Block { bytes: vec![0], long: false });
    let bad_graph = LegacyGraph::new(&bad, limits(&bad)).unwrap();
    assert!(matches!(SharedOwnerStager::new(&bad_graph, &batch.actors, &world_bindings),
        Err(GraphError { kind: GraphErrorKind::ContainerData, .. })));

    let mut bad = input.clone();
    let Node::Object(actor) = &bad.arena.nodes[second.0 as usize] else { unreachable!() };
    let original_descriptor = actor.descriptor;
    let mut descriptor = bad.arena.nodes[original_descriptor.0 as usize].clone();
    let Node::Descriptor(value) = &mut descriptor else { unreachable!() };
    value.signatures.clear();
    let malformed_descriptor = NodeId(bad.arena.nodes.len() as u32);
    bad.arena.nodes.push(descriptor);
    let Node::Object(actor) = &mut bad.arena.nodes[second.0 as usize] else { unreachable!() };
    actor.descriptor = malformed_descriptor;
    for slot in &mut actor.slots {
        if slot.descriptor == original_descriptor { slot.descriptor = malformed_descriptor; }
    }
    let bad_graph = LegacyGraph::new(&bad, limits(&bad)).unwrap();
    assert!(matches!(SharedOwnerStager::new(&bad_graph, &batch.actors, &world_bindings),
        Err(GraphError { node: Some(node), kind: GraphErrorKind::Descriptor, .. }) if node == malformed_descriptor));

    let original_id = batch.actors[1].1.borrow().id;
    batch.actors[1].1.borrow_mut().id = original_id.wrapping_add(1);
    assert!(matches!(SharedOwnerStager::new(&graph, &batch.actors, &world_bindings),
        Err(GraphError { node: Some(node), kind: GraphErrorKind::Reference, .. }) if node == second));
    batch.actors[1].1.borrow_mut().id = original_id;
}
