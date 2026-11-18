// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::legacy_organism::tests::{Identities, assert_actor};
use crate::object_stream::{ArrayData, FieldValue, Record, StreamReader, class_by_name};
use crate::organism::GenomeRef;
use crate::statistics::tests::{Observation, observed_field};
use crate::{JavaRandom, WorldConfig, WorldParameters};
use std::collections::HashMap;

#[path = "legacy_jts_streams.rs"]
mod jts_inputs;

fn document(bytes: &[u8]) -> Document {
    let input = StreamReader::new(bytes, Limits::default()).unwrap().read_document().unwrap();
    assert_eq!(input.consumed, bytes.len());
    input
}

fn caller() -> World {
    let world = World::with_random(
        WorldConfig { seed: -827364, initial_attempts: 0, ..WorldConfig::default() },
        WorldParameters::default(), JavaRandom::new(7224),
    ).unwrap();
    world.tick.set(17);
    world.session.0.random.borrow_mut().next_i64();
    world
}

fn open(caller: &World, input: &Document) -> World {
    let limits = Limits::default();
    caller.decode_legacy_document(input, limits, limits.allocated_bytes).unwrap()
}

fn handle(value: Value) -> NodeId {
    let Value::Handle(id) = value else { panic!("Nonnull source owner required"); };
    id
}

fn set(input: &mut Document, id: NodeId, owner: &str, name: &str, value: FieldValue) {
    let Node::Object(object) = &input.arena.nodes[id.0 as usize] else { panic!("Object required"); };
    let (slot, field) = object.slots.iter().enumerate().find_map(|(slot_index, slot)| {
        let Node::Descriptor(descriptor) = &input.arena.nodes[slot.descriptor.0 as usize] else { panic!("Descriptor required"); };
        let schema = class_schema(descriptor.class);
        if schema.name != owner { return None; }
        schema.fields.iter().position(|field| field.name == name).map(|field| (slot_index, field))
    }).unwrap();
    let Node::Object(object) = &mut input.arena.nodes[id.0 as usize] else { unreachable!() };
    object.slots[slot].values[field] = value;
}

fn compare_genetic_graph(
    source: &LegacyGraph<'_>, left: Value, target: &LegacyGraph<'_>, right: Value,
    forward: &mut HashMap<u32, u32>, reverse: &mut HashMap<u32, u32>,
) {
    let (left, right) = match (left, right) {
        (Value::Null, Value::Null) => return,
        (Value::Handle(left), Value::Handle(right)) => (left, right),
        _ => panic!("Genetic reference nullness changed"),
    };
    if let Some(previous) = forward.insert(left.0, right.0) { assert_eq!(previous, right.0); return; }
    assert!(reverse.insert(right.0, left.0).is_none(), "Distinct source owners were merged");
    match (source.node(left).unwrap(), target.node(right).unwrap()) {
        (Node::String(left), Node::String(right)) => assert_eq!(left.units, right.units),
        (Node::Array(left_array), Node::Array(right_array)) => {
            assert_eq!(source.class_name(left).unwrap(), target.class_name(right).unwrap());
            match (&left_array.data, &right_array.data) {
                (ArrayData::References(left), ArrayData::References(right)) => {
                    assert_eq!(left.len(), right.len());
                    for (left, right) in left.iter().zip(right) {
                        compare_genetic_graph(source, *left, target, *right, forward, reverse);
                    }
                }
                (left, right) => assert_eq!(left, right),
            }
        }
        (Node::Object(_), Node::Object(_)) => {
            assert_eq!(source.class_name(left).unwrap(), target.class_name(right).unwrap());
            for slot in source.slots(left).unwrap() {
                let Node::Descriptor(descriptor) = source.node(slot.descriptor).unwrap() else { panic!("Descriptor required"); };
                let schema = class_schema(descriptor.class);
                for field in schema.fields {
                    let expected = source.field(left, schema.name, field.name).unwrap();
                    let actual = target.field(right, schema.name, field.name).unwrap();
                    match (expected, actual) {
                        (FieldValue::Reference(left), FieldValue::Reference(right)) => {
                            compare_genetic_graph(source, left, target, right, forward, reverse);
                        }
                        (expected, actual) => assert_eq!(expected, actual, "{}::{}", schema.name, field.name),
                    }
                }
            }
        }
        _ => panic!("Genetic node kind changed"),
    }
}

fn assert_world(input: &Document, world: &World) {
    let graph = LegacyGraph::new(input, Limits::default()).unwrap();
    let root = graph.world_root().unwrap();
    assert_eq!(world.dimensions(), (graph.int(root, WORLD, "_width").unwrap(), graph.int(root, WORLD, "_height").unwrap()));
    assert_eq!(world.next_id.get(), graph.int(root, WORLD, "NEXT_ID").unwrap());
    assert_eq!(world.population.get(), graph.int(root, WORLD, "_population").unwrap());
    assert_eq!(world.frame.get(), graph.int(root, WORLD, "nFrames").unwrap());
    assert_eq!(world.corridors_exist.get(), graph.boolean(root, WORLD, "_corridorexists").unwrap());
    for (field, actual) in [("_O2", world.oxygen.get()), ("_CO2", world.carbon_dioxide.get()), ("_CH4", world.methane.get())] {
        assert_eq!(actual.to_bits(), graph.double_bits(root, WORLD, field).unwrap());
    }
    assert!(world.incoming_corridors.borrow().is_empty() && world.outgoing_corridors.borrow().is_empty());
    let (_, expected_members) = members(&graph, root).unwrap();
    let mut tokens = ActorTokens::new(&graph, root);
    for source in &expected_members { tokens.token(&graph, handle(*source)).unwrap(); }
    let expected_spatial = decode_spatial(&graph, root, &mut tokens).unwrap();
    assert_eq!(*world.spatial.borrow(), expected_spatial);
    assert!(world.spatial.borrow().imported());
    let spatial = world.spatial.borrow().token_bindings();
    let spatial: Vec<_> = spatial.into_iter().map(|actor|actor.expect("Imported token retains its real graph owner")).collect();
    assert_eq!(tokens.nodes.len(), spatial.len());
    let mut identities = Identities::default();
    for (source, actual) in expected_members.iter().zip(world.organisms.borrow().iter()) {
        identities.bind(0, *source, Some(actual.borrow().state.bounds.identity()));
    }
    assert_eq!(expected_members.len(), world.organisms.borrow().len());
    let mut actor_nodes = HashMap::new();
    for (node, actor) in tokens.nodes.iter().zip(&spatial) { actor_nodes.insert(node.0, Rc::clone(actor)); }
    let mut pending: Vec<_> = actor_nodes.values().cloned().collect();
    while let Some(actor) = pending.pop() {
        let actor = actor.borrow();
        for field in ["_sizeRect", "lastFrame"] {
            let source_id = actor_nodes.iter().find(|(_, value)| value.borrow().state.bounds.same_instance(&actor.state.bounds)).map(|(id, _)| NodeId(*id)).unwrap();
            if let Value::Handle(node) = graph.reference(source_id, ACTOR, field).unwrap() {
                if graph.class_name(node).unwrap() == ACTOR && !actor_nodes.contains_key(&node.0) {
                    let rectangle = if field == "_sizeRect" { actor.body.size_rect.as_ref() } else { actor.state.last_frame.as_ref() }.unwrap();
                    let target = rectangle.actor_target().expect("Complete Rectangle actor target");
                    actor_nodes.insert(node.0, Rc::clone(&target));
                    pending.push(target);
                }
            }
        }
    }
    let mut genotype_sources = Vec::new();
    let mut genotypes = Vec::<Option<GenomeRef>>::new();
    let mut color_sources = Vec::new();
    let mut colors = Vec::new();
    for (node, actor) in &actor_nodes {
        let actor = actor.borrow();
        assert_actor(&graph, NodeId(*node), &actor, &mut identities);
        assert!(actor.state.world.resolve().unwrap().handle().same_owner(&world.handle()));
        for (field, actual) in [("_geneticCode", &actor.genome), ("_infectedGeneticCode", &actor.infected_genome), ("_savedGeneticCode", &actor.saved_genome)] {
            genotype_sources.push(graph.reference(NodeId(*node), ACTOR, field).unwrap());
            genotypes.push(actual.clone());
        }
        color_sources.push(graph.reference(NodeId(*node), ACTOR, "_color").unwrap());
        color_sources.push(graph.reference(NodeId(*node), ACTOR, "_segColor").unwrap());
        colors.push((actor.state.temporary_color.clone(), actor.body.phenotype_colors.clone()));
    }
    let statistics_node = graph.required_reference(root, WORLD, "worldStatistics").unwrap();
    let statistics = world.statistics_ref();
    let statistics = statistics.borrow();
    let schema = class_schema(class_by_name(STATISTICS).unwrap());
    assert_eq!(schema.fields.len(), 52);
    for field in schema.fields {
        let expected = graph.field(statistics_node, STATISTICS, field.name).unwrap();
        match (expected, observed_field(&statistics, field.name)) {
            (FieldValue::Int(expected), Observation::Int(actual)) => assert_eq!(expected, actual),
            (FieldValue::Long(expected), Observation::Long(actual)) => assert_eq!(expected, actual),
            (FieldValue::DoubleBits(expected), Observation::Double(actual)) => assert_eq!(expected, actual.to_bits()),
            (FieldValue::Reference(expected), Observation::Genome(actual)) => {
                identities.bind(3, expected, actual.map(|value| Rc::as_ptr(value) as usize));
                genotype_sources.push(expected);
                genotypes.push(actual.cloned());
            }
            (FieldValue::Reference(expected), Observation::History(actual)) => {
                identities.bind(8, expected, actual.map(|value| value.identity()));
                if let (Value::Handle(node), Some(actual)) = (expected, actual) {
                    let expected = graph.array_list_compatible(node).unwrap();
                    let actual = actual.elements();
                    assert_eq!(expected.len(), actual.len());
                    for (expected, actual) in expected.into_iter().zip(actual.iter()) {
                        identities.bind(9, expected, actual.as_ref().map(|value| value.identity()));
                        if let (Value::Handle(node), Some(actual)) = (expected, actual) {
                            assert_eq!(actual.bits(), graph.double_bits(node, "java.lang.Double", "value").unwrap());
                        }
                    }
                }
            }
            _ => panic!("Stored statistics type changed: {}", field.name),
        }
    }
    let color_roots: Vec<_> = colors.iter().flat_map(|(color, array)| [
        crate::legacy_color::ColorRoot::Color(color.as_ref()), crate::legacy_color::ColorRoot::Colors(array.as_ref()),
    ]).collect();
    let exported = crate::legacy_genetics::export_genome_color_roots(
        &genotypes.iter().map(Option::as_ref).collect::<Vec<_>>(), &color_roots,
        Limits::default(), Limits::default().allocated_bytes,
    ).unwrap();
    genotype_sources.extend(color_sources);
    let target = LegacyGraph::new(&exported, Limits::default()).unwrap();
    assert_eq!(genotype_sources.len(), exported.records.len());
    let mut forward = HashMap::new();
    let mut reverse = HashMap::new();
    for (expected, actual) in genotype_sources.into_iter().zip(&exported.records) {
        let Record::Value(actual) = actual else { panic!("Genotype root required"); };
        compare_genetic_graph(&graph, expected, &target, *actual, &mut forward, &mut reverse);
    }
    assert_eq!(class_schema(class_by_name(WORLD).unwrap()).fields.len(), 12);
    assert_eq!(class_schema(class_by_name("biogenesis.GeneticCode").unwrap()).fields.len(), 27);
    assert_eq!(class_schema(class_by_name("biogenesis.Gene").unwrap()).fields.len(), 40);
}

#[test]
fn existing_small_and_jts_worlds_restore_owned_fields_without_touching_the_old_root() {
    let caller = caller();
    let old_handle = caller.handle();
    let old_statistics = caller.open_statistics();
    let before = caller.clone();
    let random = caller.random_snapshot();
    let parameters = caller.parameters();
    for bytes in std::iter::once(include_bytes!("../tests/fixtures/object_stream_codec/world.ser").as_slice())
        .chain(jts_inputs::STREAMS.iter().map(|(_, bytes)| *bytes))
    {
        let input = document(bytes);
        let loaded = open(&caller, &input);
        assert_world(&input, &loaded);
        assert!(caller.session().same_owner(&loaded.session()));
        assert!(!loaded.handle().same_owner(&old_handle));
        assert_eq!(loaded.seed(), caller.seed());
        assert_eq!(loaded.config.get().initial_attempts, caller.config.get().initial_attempts);
        assert_eq!((loaded.tick(), loaded.paused()), (caller.tick(), caller.paused()));
        assert!(loaded.frame_objects.borrow().is_empty());
        drop(input);
        for actor in loaded.actor_roots() { assert!(actor.borrow().state.world.resolve().is_ok()); }
        assert_eq!(caller, before);
        assert_eq!(caller.open_statistics(), old_statistics);
        assert_eq!(caller.random_snapshot(), random);
        assert_eq!(caller.parameters(), parameters);
    }
}

#[test]
fn signed_scalars_and_ieee_gases_are_not_startup_validated_or_normalized() {
    let caller = caller();
    let mut input = document(include_bytes!("../tests/fixtures/object_stream_codec/world.ser"));
    let root = LegacyGraph::new(&input, Limits::default()).unwrap().world_root().unwrap();
    for (field, value) in [("_width", i32::MIN), ("_height", -1), ("NEXT_ID", -3), ("_population", 17), ("nFrames", i32::MAX)] {
        set(&mut input, root, WORLD, field, FieldValue::Int(value));
    }
    for (field, bits) in [("_O2", 0x8000000000000000), ("_CO2", 0x7ff0000000000042), ("_CH4", 0xfff8123456789abc)] {
        set(&mut input, root, WORLD, field, FieldValue::DoubleBits(bits));
    }
    assert_world(&input, &open(&caller, &input));
}

#[test]
fn malformed_and_exhausted_imports_leave_the_old_root_and_session_unchanged() {
    let caller = caller();
    let before = caller.clone();
    let old_handle = caller.handle();
    let old_statistics = caller.open_statistics();
    let random = caller.random_snapshot();
    let input = document(jts_inputs::STREAMS[0].1);
    for limits in [Limits { allocated_bytes: 0, ..Limits::default() }, Limits { work: 1, ..Limits::default() },
        Limits { total_array_elements: 0, ..Limits::default() }, Limits { handles: 0, ..Limits::default() }]
    {
        assert!(matches!(caller.decode_legacy_document(&input, limits, limits.allocated_bytes), Err(WorldDecodeError::Graph(GraphError { kind: GraphErrorKind::Budget, .. }))));
    }
    let mut broken = input.clone();
    let root = LegacyGraph::new(&broken, Limits::default()).unwrap().world_root().unwrap();
    set(&mut broken, root, WORLD, "_width", FieldValue::Long(7));
    assert!(matches!(caller.decode_legacy_document(&broken, Limits::default(), usize::MAX), Err(WorldDecodeError::Graph(GraphError { kind: GraphErrorKind::Type, .. }))));
    let mut broken_descriptor = input.clone();
    let Node::Object(object) = &broken_descriptor.arena.nodes[root.0 as usize] else { unreachable!() };
    let descriptor_id = object.descriptor;
    let Node::Descriptor(descriptor) = &mut broken_descriptor.arena.nodes[descriptor_id.0 as usize] else { unreachable!() };
    descriptor.signatures.clear();
    assert!(matches!(caller.decode_legacy_document(&broken_descriptor, Limits::default(), usize::MAX), Err(WorldDecodeError::Graph(GraphError { kind: GraphErrorKind::Descriptor, .. }))));
    for field in ["colDetTree", "worldStatistics"] {
        let mut unsupported = input.clone();
        set(&mut unsupported, root, WORLD, field, FieldValue::Reference(Value::Null));
        assert!(matches!(caller.decode_legacy_document(&unsupported, Limits::default(), usize::MAX), Err(WorldDecodeError::Unsupported { shape: UnsupportedWorldShape::NullWorldReference, .. })));
    }
    let mut nullable_members = input.clone();
    set(&mut nullable_members, root, WORLD, "_organisms", FieldValue::Reference(Value::Null));
    let loaded = open(&caller, &nullable_members);
    assert!(loaded.organisms.shared_ref().is_none());
    assert!(matches!(loaded.find_organism_from_position([0, 0]), Err(crate::WorldError::List(crate::ListAccessError::MissingList { field: "_organisms" }))));
    assert_eq!(caller, before);
    assert!(caller.handle().same_owner(&old_handle));
    assert_eq!(caller.open_statistics(), old_statistics);
    assert_eq!(caller.random_snapshot(), random);
}

#[test]
fn null_mixed_and_foreign_owner_shapes_have_explicit_checkpoint_errors() {
    let caller = caller();
    let input = document(jts_inputs::STREAMS[0].1);
    let graph = LegacyGraph::new(&input, Limits::default()).unwrap();
    let root = graph.world_root().unwrap();
    let (list, values) = members(&graph, root).unwrap();
    let actor = handle(values[0]);
    let statistics = graph.required_reference(root, WORLD, "worldStatistics").unwrap();
    let mut mixed = input.clone();
    set(&mut mixed, statistics, STATISTICS, "populationList", FieldValue::Reference(Value::Handle(list)));
    let admitted = open(&caller, &mixed);
    let statistics_owner = admitted.statistics_ref();
    let statistics = statistics_owner.borrow();
    let history = statistics.get_population_list().unwrap();
    assert!(admitted.organisms.shared_ref().unwrap().shares_backing(history.shared_ref()));
    assert!(matches!(history.numeric_prefix("populationList", 100), Err(crate::HistoryError::WrongClass { index: 0, actual: "biogenesis.Organism", .. })));
    drop(statistics);
    let mut foreign = input.clone();
    set(&mut foreign, actor, ACTOR, "_world", FieldValue::Reference(Value::Null));
    let admitted = open(&caller, &foreign);
    assert!(!admitted.organisms.borrow()[0].borrow().state.world.is_present());
    set(&mut foreign, actor, ACTOR, "_world", FieldValue::Reference(Value::Handle(actor)));
    assert!(matches!(caller.decode_legacy_document(&foreign, Limits::default(), usize::MAX),
        Err(WorldDecodeError::Unsupported { shape: UnsupportedWorldShape::ActorWorld, .. })));
    let mut null_member = input.clone();
    let Node::Object(object) = &mut null_member.arena.nodes[list.0 as usize] else { unreachable!() };
    let record = object.slots[0].custom.iter_mut().find(|record| matches!(record, Record::Value(_))).unwrap();
    *record = Record::Value(Value::Null);
    let admitted = open(&caller, &null_member);
    assert!(matches!(admitted.find_organism_from_position([0, 0]), Err(crate::WorldError::List(crate::ListAccessError::MissingElement { field: "_organisms", index: 0 }))));
}

#[test]
fn tree_handoff_retires_root_retention_and_external_handle_leases_release_without_cycles() {
    let caller = caller();
    let mut witnessed_tree_only = false;
    for (name, bytes) in jts_inputs::STREAMS {
        if !name.starts_with("retained-") { continue; }
        let input = document(bytes);
        let loaded = open(&caller, &input);
        let direct = loaded.organisms.borrow();
        assert!(loaded.spatial.borrow().imported());
        let spatial = loaded.spatial.borrow().token_bindings();
        if !spatial.iter().flatten().any(|actor| !direct.iter().any(|member| Rc::ptr_eq(actor, member))) { continue; }
        drop(spatial);
        drop(direct);
        witnessed_tree_only = true;
        for escaped in [false, true] {
            let loaded = open(&caller, &input);
            let weak: Vec<_> = loaded.rectangle_context.borrow().as_ref().unwrap().actors().iter().map(Rc::downgrade).collect();
            let retained = escaped.then(|| loaded.handle());
            let old_oxygen = loaded.oxygen();
            loaded.add_oxygen(1.0);
            if let Some(handle) = &retained { assert_eq!(handle.oxygen(), old_oxygen + 1.0, "WorldHandle keeps live-state semantics"); }
            loaded.organisms.borrow_mut().clear();
            loaded.single_step().unwrap();
            assert!(loaded.rectangle_context.borrow().is_none());
            assert!(!loaded.spatial.borrow().imported());
            assert!(weak.iter().all(|actor| actor.upgrade().is_some() == escaped));
            drop(retained);
            assert!(weak.iter().all(|actor| actor.upgrade().is_none()));
        }
        break;
    }
    assert!(witnessed_tree_only, "Use an actual retained JTS actor fixture");
}

fn witness(name: &str) -> Document {
    let bytes: &[u8] = match name {
        "read_frame-call_0.ser" => include_bytes!("../tests/fixtures/world_hydration/read_frame-call_0.ser"),
        "read_frame-call_3.ser" => include_bytes!("../tests/fixtures/world_hydration/read_frame-call_3.ser"),
        "actor_closure-call_0.ser" => include_bytes!("../tests/fixtures/world_hydration/actor_closure-call_0.ser"),
        _ => panic!("Unknown retained witness"),
    };
    document(bytes)
}

#[test]
fn source_read_frame_resumes_once_with_the_stored_sticky_flag_and_exact_source_gases() {
    let caller = caller();
    let random = caller.random_snapshot();
    let input = witness("read_frame-call_0.ser");
    let loaded = open(&caller, &input);
    assert_world(&input, &loaded);
    assert_eq!(loaded.frame(), 255);
    assert!(loaded.corridors_exist.get());
    drop(input);
    assert!(loaded.single_step().unwrap());
    assert_eq!(loaded.tick(), caller.tick() + 1);
    assert_eq!(loaded.random_snapshot(), random);
    let expected = witness("read_frame-call_3.ser");
    let graph = LegacyGraph::new(&expected, Limits::default()).unwrap();
    let root = graph.world_root().unwrap();
    assert_eq!(loaded.frame(), graph.int(root, WORLD, "nFrames").unwrap());
    for (field, actual) in [("_O2", loaded.oxygen.get()), ("_CO2", loaded.carbon_dioxide.get()), ("_CH4", loaded.methane.get())] {
        assert_eq!(actual.to_bits(), graph.double_bits(root, WORLD, field).unwrap());
    }
    assert!(loaded.paused());
}

#[test]
fn source_tree_only_and_rectangle_targets_survive_then_release_through_explicit_handle_leases() {
    let caller = caller();
    let input = witness("actor_closure-call_0.ser");
    let loaded = open(&caller, &input);
    assert_world(&input, &loaded);
    assert_eq!((loaded.next_id.get(), loaded.population()), (-3, 17));
    assert_eq!(loaded.organisms.borrow().len(), 2);
    assert!(Rc::ptr_eq(&loaded.organisms.borrow()[0], &loaded.organisms.borrow()[1]));
    let actors = loaded.actor_roots();
    let mut distinct = HashMap::new();
    for actor in actors { distinct.entry(Rc::as_ptr(&actor) as usize).or_insert(actor); }
    assert_eq!(distinct.len(), 3);
    assert!(distinct.values().all(|actor| actor.borrow().id == 77));
    let weak: Vec<_> = distinct.values().map(Rc::downgrade).collect();
    let escaped = crate::world::player_actions::OrganismHandle(Rc::clone(&loaded.organisms.borrow()[0]), loaded.handle());
    let old_tree_tokens = loaded.spatial.borrow().query(crate::IntRect { x: -1_000_000, y: -1_000_000, width: 2_000_000, height: 2_000_000 });
    loaded.frame_objects.replace(Vec::new());
    for token in old_tree_tokens { assert!(distinct.contains_key(&(Rc::as_ptr(&loaded.spatial_actor(token)) as usize))); }
    drop(distinct);
    drop(input);
    loaded.organisms.borrow_mut().clear();
    loaded.single_step().unwrap();
    assert!(!loaded.spatial.borrow().imported());
    assert!(loaded.rectangle_context.borrow().is_none());
    assert!(weak.iter().all(|actor| actor.upgrade().is_some()), "Escaped handle owns its explicit old context lease");
    drop(escaped);
    assert!(weak.iter().all(|actor| actor.upgrade().is_none()), "No obsolete actor remains rooted after lease release");
}

#[test]
fn clone_discovers_foreign_worlds_from_the_escaped_import_lease_and_its_rectangle_closure() {
    let caller = caller();
    let input = witness("actor_closure-call_0.ser");
    let loaded = open(&caller, &input);
    let leased_actor = Rc::clone(&loaded.organisms.borrow()[0]);
    let escaped = loaded.handle().alias();
    loaded.organisms.borrow_mut().clear();
    loaded.single_step().unwrap();
    assert!(loaded.actor_roots().is_empty());
    assert!(loaded.rectangle_context.borrow().is_none());
    assert!(!escaped.actor_roots().is_empty());

    let foreign = World::with_session(
        WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, loaded.session(),
    ).unwrap();
    foreign.oxygen.set(12345.0);
    let rectangle_owner = World::with_session(
        WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, loaded.session(),
    ).unwrap();
    let genome = crate::Genome::from_parts(
        &[crate::Gene::new(12.0, 0.0, crate::ColorKind::Green.argb(), -1, [0; 36]).shared()],
        crate::GenomeProperties { symmetry: 1, ..crate::GenomeProperties::default() },
    );
    rectangle_owner.paste_genome(genome, [1000, 1000]).unwrap().unwrap();
    rectangle_owner.oxygen.set(54321.0);
    let rectangle_actor = Rc::clone(&rectangle_owner.organisms.borrow()[0]);
    {
        let mut actor = leased_actor.borrow_mut();
        actor.state.world = foreign.association();
        actor.body.size_rect = Some(rectangle_actor.borrow().state.bounds.clone());
    }
    let originals = [loaded.association(), foreign.association(), rectangle_owner.association()];
    let foreign_random = foreign.random_snapshot();
    let copied = escaped.clone();
    let copied_actor = copied.actor_roots().into_iter().find(|actor| {
        actor.borrow().state.world.resolve().unwrap().oxygen() == 12345.0
    }).unwrap();
    assert!(!Rc::ptr_eq(&copied_actor, &leased_actor));
    let copied_foreign = copied_actor.borrow().state.world.resolve().unwrap();
    assert!(!copied_foreign.handle().same_owner(&foreign.handle()));
    assert!(copied_foreign.session().same_owner(&copied.session()));
    let copied_target = copied_actor.borrow().body.size_rect.as_ref().unwrap().actor_target().unwrap();
    assert!(!Rc::ptr_eq(&copied_target, &rectangle_actor));
    let copied_rectangle_owner = copied_target.borrow().state.world.resolve().unwrap();
    assert!(!copied_rectangle_owner.handle().same_owner(&rectangle_owner.handle()));
    assert_eq!(copied_rectangle_owner.oxygen(), 54321.0);
    assert!(copied_rectangle_owner.session().same_owner(&copied.session()));
    copied_foreign.add_oxygen(1.0);
    copied_rectangle_owner.add_oxygen(2.0);
    assert_eq!(foreign.oxygen(), 12345.0);
    assert_eq!(rectangle_owner.oxygen(), 54321.0);
    assert_eq!(foreign.random_snapshot(), foreign_random);
    let copied_associations = [copied_foreign.association(), copied_rectangle_owner.association()];
    drop(copied_foreign);
    drop(copied_rectangle_owner);
    drop(leased_actor);
    drop(rectangle_actor);
    drop(loaded);
    drop(escaped);
    drop(foreign);
    drop(rectangle_owner);
    drop(input);
    assert!(originals.iter().all(|association| association.resolve().is_err()));
    assert_eq!(copied_actor.borrow().state.world.resolve().unwrap().oxygen(), 12346.0);
    assert_eq!(copied_target.borrow().state.world.resolve().unwrap().oxygen(), 54323.0);
    drop(copied_target);
    drop(copied_actor);
    drop(copied);
    assert!(copied_associations.iter().all(|association| association.resolve().is_err()), "Copied foreign Worlds have no retention cycle");
}

#[test]
fn declared_child_list_rejects_wrappers_before_publication_and_defers_null() {
    let caller=caller();let before=caller.clone();let owner=caller.handle();
    let session=caller.session();let random=caller.random_snapshot();
    for null_child in [false,true] {
        let mut input=document(include_bytes!("../tests/fixtures/shared_world_owners/child_history_cache-call_0.ser"));
        let (node,wrapper)={
            let graph=LegacyGraph::new(&input,Limits::default()).unwrap();
            let world=graph.world_root().unwrap();
            let tree=graph.required_reference(world,WORLD,"colDetTree").unwrap();
            (graph.required_reference(tree,"org.locationtech.jts.index.strtree.AbstractSTRtree","root").unwrap(),
                graph.required_reference(world,WORLD,"_organisms").unwrap())
        };
        set(&mut input,node,"org.locationtech.jts.index.strtree.AbstractNode","childBoundables",
            FieldValue::Reference(if null_child { Value::Null } else { Value::Handle(wrapper) }));
        let result=caller.decode_legacy_document(&input,Limits::default(),Limits::default().allocated_bytes);
        if null_child {
            let loaded=result.unwrap();
            assert!(loaded.session().same_owner(&session));
            let root=loaded.spatial.borrow().shared_ref().value().root.clone().unwrap();
            assert!(matches!(&*root.value(),crate::shared_world_owners::BoundableValue::Node { children:None,.. }));
            assert!(matches!(loaded.spatial.borrow().query_slots(crate::IntRect { x:0,y:0,width:1200,height:800 }),
                Err(crate::SpatialAccessError::List { error:crate::ListAccessError::MissingList { field:"childBoundables" },.. })));
        } else {
            assert!(matches!(result,Err(WorldDecodeError::Graph(GraphError { node:Some(node),kind:GraphErrorKind::Class,.. })) if node==wrapper));
        }
        assert_eq!(caller,before);assert!(caller.handle().same_owner(&owner));
        assert!(caller.session().same_owner(&session));assert_eq!(caller.random_snapshot(),random);
    }
}
