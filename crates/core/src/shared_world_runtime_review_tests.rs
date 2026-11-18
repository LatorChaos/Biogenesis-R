// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::history::{HistoryError, HistoryListRef, JavaDoubleRef};
use crate::legacy_graph::LegacyGraph;
use crate::object_stream::{
    Document, FieldValue, Limits, Node, NodeId, Record, StreamReader, Value,
    class_by_name, class_schema,
};
use crate::shared_world_owners::{ListAccessError, SharedListRef, SharedSlot};
use crate::statistics::tests::{Observation, observed_field, set_statistics_input};
use std::collections::{BTreeMap, HashMap};

const WORLD_CLASS: &str = "biogenesis.World";
const STATISTICS_CLASS: &str = "biogenesis.WorldStatistics";
const HISTORY_FIELDS: [&str; 6] = [
    "populationList", "deathList", "birthList", "oxygenList", "carbonDioxideList", "methaneList",
];
const CAPTURE_ROWS: &str = include_str!("../tests/fixtures/shared_world_owners/world_lists.tsv");

fn document(bytes: &[u8]) -> Document {
    let result = StreamReader::new(bytes, Limits::default()).unwrap().read_document().unwrap();
    assert_eq!(result.consumed, bytes.len());
    result
}

fn authored() -> Document {
    document(include_bytes!("../tests/fixtures/shared_world_owners/member_history-authored.ser"))
}

fn captured_after_event() -> Document {
    document(include_bytes!("../tests/fixtures/shared_world_owners/member_history-call_1.ser"))
}

fn caller() -> World {
    World::with_random(
        WorldConfig { initial_attempts: 0, ..WorldConfig::default() },
        WorldParameters::default(),
        JavaRandom::new(7224),
    ).unwrap()
}

fn open(caller: &World, input: &Document) -> World {
    let limits = Limits::default();
    caller.decode_legacy_document(input, limits, limits.allocated_bytes).unwrap()
}

fn node(value: Value) -> NodeId {
    let Value::Handle(id) = value else { panic!("Nonnull captured owner"); };
    id
}

fn roots(graph: &LegacyGraph<'_>) -> (NodeId, NodeId) {
    let world = graph.world_root().unwrap();
    let statistics = graph.required_reference(world, WORLD_CLASS, "worldStatistics").unwrap();
    (world, statistics)
}

pub(super) fn assert_captured_scalars(expected: &Document, world: &World) {
    let graph = LegacyGraph::new(expected, Limits::default()).unwrap();
    let (root, statistics) = roots(&graph);
    assert_eq!(world.dimensions(), (
        graph.int(root, WORLD_CLASS, "_width").unwrap(),
        graph.int(root, WORLD_CLASS, "_height").unwrap(),
    ));
    assert_eq!(world.next_id.get(), graph.int(root, WORLD_CLASS, "NEXT_ID").unwrap());
    assert_eq!(world.population(), graph.int(root, WORLD_CLASS, "_population").unwrap());
    assert_eq!(world.frame(), graph.int(root, WORLD_CLASS, "nFrames").unwrap());
    assert_eq!(world.corridors_exist.get(), graph.boolean(root, WORLD_CLASS, "_corridorexists").unwrap());
    for (field, actual) in [
        ("_O2", world.oxygen()), ("_CO2", world.carbon_dioxide()), ("_CH4", world.methane()),
    ] {
        assert_eq!(actual.to_bits(), graph.double_bits(root, WORLD_CLASS, field).unwrap(), "{field}");
    }
    let owner = world.statistics_ref();
    let actual = owner.borrow();
    let mut scalar_count = 0;
    let mut genome_count = 0;
    let schema = class_schema(class_by_name(STATISTICS_CLASS).unwrap());
    for field in schema.fields {
        let expected = graph.field(statistics, STATISTICS_CLASS, field.name).unwrap();
        match observed_field(&actual, field.name) {
            Observation::Int(value) => {
                assert_eq!(expected, FieldValue::Int(value), "{}", field.name);
                scalar_count += 1;
            }
            Observation::Long(value) => {
                assert_eq!(expected, FieldValue::Long(value), "{}", field.name);
                scalar_count += 1;
            }
            Observation::Double(value) => {
                assert_eq!(expected, FieldValue::DoubleBits(value.to_bits()), "{}", field.name);
                scalar_count += 1;
            }
            Observation::Genome(value) => {
                assert_eq!(expected, FieldValue::Reference(Value::Null), "{}", field.name);
                assert!(value.is_none(), "{}", field.name);
                genome_count += 1;
            }
            Observation::History(_) => {}
        }
    }
    assert_eq!((scalar_count, genome_count), (37, 9));
}

fn assert_captured_histories(expected: &Document, world: &World) {
    let graph = LegacyGraph::new(expected, Limits::default()).unwrap();
    let (_, statistics) = roots(&graph);
    let owner = world.statistics_ref();
    let actual = owner.borrow();
    let mut aliases = Vec::new();
    for field in HISTORY_FIELDS {
        let expected_list = node(graph.reference(statistics, STATISTICS_CLASS, field).unwrap());
        let expected_bits: Vec<_> = graph.array_list_compatible(expected_list).unwrap().iter().map(|value| {
            graph.double_bits(node(*value), "java.lang.Double", "value").unwrap()
        }).collect();
        let Observation::History(Some(list)) = observed_field(&actual, field) else {
            panic!("Captured nonnull history {field}");
        };
        let actual_bits: Vec<_> = list.numeric_prefix(field, 100).unwrap().iter().map(|value| value.to_bits()).collect();
        assert_eq!(actual_bits, expected_bits, "{field}");
        assert_eq!(list.len(), expected_bits.len(), "{field}");
        aliases.push((expected_list, list.identity()));
    }
    for left in &aliases {
        for right in &aliases {
            assert_eq!(left.0 == right.0, left.1 == right.1);
        }
    }
}

fn exported(world: &World) -> Document {
    let limits = Limits::default();
    world.encode_legacy_document(limits, limits.allocated_bytes).unwrap()
}

fn assert_exported_list_and_box_aliases(expected: &Document, actual: &Document) {
    let expected = LegacyGraph::new(expected, Limits::default()).unwrap();
    let actual = LegacyGraph::new(actual, Limits::default()).unwrap();
    let (expected_world, expected_statistics) = roots(&expected);
    let (actual_world, actual_statistics) = roots(&actual);
    let mut forward = HashMap::new();
    let mut reverse = HashMap::new();
    let mut bind = |left: NodeId, right: NodeId| {
        if let Some(previous) = forward.insert(left.0, right.0) { assert_eq!(previous, right.0); }
        if let Some(previous) = reverse.insert(right.0, left.0) { assert_eq!(previous, left.0); }
    };
    bind(
        node(expected.reference(expected_world, WORLD_CLASS, "_organisms").unwrap()),
        node(actual.reference(actual_world, WORLD_CLASS, "_organisms").unwrap()),
    );
    for field in HISTORY_FIELDS {
        let left = node(expected.reference(expected_statistics, STATISTICS_CLASS, field).unwrap());
        let right = node(actual.reference(actual_statistics, STATISTICS_CLASS, field).unwrap());
        bind(left, right);
        assert_eq!(expected.class_name(left).unwrap(), actual.class_name(right).unwrap());
        let left = expected.array_list_compatible(left).unwrap();
        let right = actual.array_list_compatible(right).unwrap();
        assert_eq!(left.len(), right.len());
        for (left, right) in left.iter().zip(right.iter()) {
            let (left, right) = (node(*left), node(*right));
            bind(left, right);
            assert_eq!(
                expected.double_bits(left, "java.lang.Double", "value").unwrap(),
                actual.double_bits(right, "java.lang.Double", "value").unwrap(),
            );
        }
    }
}

fn assert_wrong_member(world: &World) {
    assert!(matches!(
        world.find_organism_from_position([100, 100]),
        Err(WorldError::List(ListAccessError::WrongClass {
            field: "_organisms", index: 0, expected: "biogenesis.Organism", actual: "java.lang.Double",
        })),
    ));
    assert!(matches!(world.pick_organism([100, 100]), Err(WorldError::List(
        ListAccessError::WrongClass { field: "_organisms", index: 0, actual: "java.lang.Double", .. },
    ))));
    assert!(matches!(world.inspect_at_position([100, 100]), Err(WorldError::List(
        ListAccessError::WrongClass { field: "_organisms", index: 0, actual: "java.lang.Double", .. },
    ))));
}

#[test]
fn captured_member_history_runtime_replays_all_fields_and_reached_cast() {
    let captured_case = CAPTURE_ROWS.split("C\tmember_history\t").nth(1).unwrap().split("E\tmember_history").next().unwrap();
    assert!(captured_case.contains("K\tcall_0\tread_input\n"));
    assert!(captured_case.contains("K\tcall_1\teventTime\n"));
    assert!(captured_case.contains("K\tcall_2\tfind_100_100\n"));
    assert!(captured_case.contains("X\tcall_2\tjava.lang.ClassCastException\tbiogenesis.World\tfindOrganismFromPosition\t149"));
    let caller = caller();
    let random = caller.session.random_snapshot();
    let input = authored();
    let world = open(&caller, &input);
    assert!(world.session.same_owner(&caller.session));
    assert_captured_scalars(&input, &world);
    assert_captured_histories(&input, &world);
    drop(input);
    world.statistics_ref().borrow_mut().event_time(7, 17.0, 19.0, 23.0).unwrap();
    let expected = captured_after_event();
    assert_captured_scalars(&expected, &world);
    assert_captured_histories(&expected, &world);
    assert_exported_list_and_box_aliases(&expected, &exported(&world));
    let before = exported(&world);
    assert_wrong_member(&world);
    assert_eq!(exported(&world), before);
    assert_eq!(caller.session.random_snapshot(), random);
    assert!(world.session.same_owner(&caller.session));
}

#[test]
fn copied_and_reloaded_member_history_preserves_aliases_and_detaches_mutability() {
    let caller = caller();
    let world = open(&caller, &captured_after_event());
    let copied = world.clone();
    assert!(!copied.session.same_owner(&world.session));
    assert_eq!(copied.session.random_snapshot(), world.session.random_snapshot());
    let original_history = world.statistics_ref().borrow().get_population_list().unwrap().clone();
    let copied_history = copied.statistics_ref().borrow().get_population_list().unwrap().clone();
    assert!(!original_history.same_instance(&copied_history));
    let expected = captured_after_event();
    assert_captured_scalars(&expected, &copied);
    assert_captured_histories(&expected, &copied);
    assert_exported_list_and_box_aliases(&expected, &exported(&copied));
    copied.statistics_ref().borrow_mut().event_time(7, 17.0, 19.0, 23.0).unwrap();
    assert_eq!(copied_history.len(), 2);
    assert_eq!(original_history.len(), 1);
    assert_wrong_member(&copied);
    let wire = exported(&world);
    let reloaded = open(&caller, &wire);
    assert_captured_scalars(&expected, &reloaded);
    assert_captured_histories(&expected, &reloaded);
    assert_exported_list_and_box_aliases(&expected, &exported(&reloaded));
    assert_wrong_member(&reloaded);
}

fn set_reference(input: &mut Document, id: NodeId, class: &str, field: &str, value: Value) {
    let Node::Object(object) = &input.arena.nodes[id.0 as usize] else { panic!("Object"); };
    let (slot, index) = object.slots.iter().enumerate().find_map(|(index, slot)| {
        let Node::Descriptor(descriptor) = &input.arena.nodes[slot.descriptor.0 as usize] else { panic!("Descriptor"); };
        let schema = class_schema(descriptor.class);
        (schema.name == class).then(|| schema.fields.iter().position(|value| value.name == field).map(|field| (index, field))).flatten()
    }).unwrap();
    let Node::Object(object) = &mut input.arena.nodes[id.0 as usize] else { unreachable!() };
    object.slots[slot].values[index] = FieldValue::Reference(value);
}

fn distinct_history_with_spatial_alias(clone_items: bool) -> Document {
    let mut input = document(include_bytes!("../tests/fixtures/shared_world_owners/pending_history_build-authored.ser"));
    let (statistics, original) = {
        let graph = LegacyGraph::new(&input, Limits::default()).unwrap();
        let (_, statistics) = roots(&graph);
        (statistics, node(graph.reference(statistics, STATISTICS_CLASS, "populationList").unwrap()))
    };
    let mut list = input.arena.nodes[original.0 as usize].clone();
    let Node::Object(object) = &mut list else { panic!("Captured ArrayList"); };
    for record in object.slots.iter_mut().flat_map(|slot| slot.custom.iter_mut()) {
        let Record::Value(Value::Handle(item)) = record else { continue; };
        if clone_items {
            let value = input.arena.nodes[item.0 as usize].clone();
            *item = NodeId(input.arena.nodes.len() as u32);
            input.arena.nodes.push(value);
        }
    }
    let separate = NodeId(input.arena.nodes.len() as u32);
    input.arena.nodes.push(list);
    set_reference(&mut input, statistics, STATISTICS_CLASS, "populationList", Value::Handle(separate));
    assert_ne!(separate, original);
    let graph = LegacyGraph::new(&input, Limits::default()).unwrap();
    let world = graph.world_root().unwrap();
    let tree = graph.required_reference(world, WORLD_CLASS, "colDetTree").unwrap();
    assert_eq!(graph.required_reference(tree, "org.locationtech.jts.index.strtree.AbstractSTRtree", "itemBoundables").unwrap(), original);
    let tree_items = graph.array_list_compatible(original).unwrap();
    let history_items = graph.array_list_compatible(separate).unwrap();
    assert!(!tree_items.is_empty());
    assert_eq!(tree_items.len(), history_items.len());
    for (tree_item, history_item) in tree_items.iter().zip(history_items.iter()) {
        let (tree_item, history_item) = (node(*tree_item), node(*history_item));
        assert_eq!(tree_item == history_item, !clone_items);
        let class = "org.locationtech.jts.index.strtree.ItemBoundable";
        assert_eq!(graph.class_name(history_item).unwrap(), class);
        assert_eq!(graph.reference(tree_item, class, "bounds").unwrap(), graph.reference(history_item, class, "bounds").unwrap());
    }
    input
}

#[test]
fn runtime_null_membership_and_late_history_failures_keep_distinct_prefixes() {
    let caller = caller();
    let mut input = captured_after_event();
    let (root, list) = {
        let graph = LegacyGraph::new(&input, Limits::default()).unwrap();
        let (root, _) = roots(&graph);
        (root, node(graph.reference(root, WORLD_CLASS, "_organisms").unwrap()))
    };
    let Node::Object(list) = &mut input.arena.nodes[list.0 as usize] else { panic!("ArrayList"); };
    let element = list.slots.iter_mut().flat_map(|slot| slot.custom.iter_mut()).find(|record| matches!(record, Record::Value(_))).unwrap();
    *element = Record::Value(Value::Null);
    let world = open(&caller, &input);
    assert!(matches!(world.find_organism_from_position([100, 100]), Err(WorldError::List(
        ListAccessError::MissingElement { field: "_organisms", index: 0 },
    ))));
    set_reference(&mut input, root, WORLD_CLASS, "_organisms", Value::Null);
    let world = open(&caller, &input);
    assert!(matches!(world.find_organism_from_position([100, 100]), Err(WorldError::List(
        ListAccessError::MissingList { field: "_organisms" },
    ))));

    // This seeded case checks Source ordering; it is not an additional captured Java call
    let world = open(&caller, &authored());
    let statistics = world.statistics_ref();
    let mut statistics = statistics.borrow_mut();
    set_statistics_input(&mut statistics, "birthLastTime", "int", "3", &BTreeMap::new());
    set_statistics_input(&mut statistics, "deathLastTime", "int", "5", &BTreeMap::new());
    statistics.methane_list = None;
    assert_eq!(statistics.event_time(7, 17.0, 19.0, 23.0), Err(HistoryError::MissingList { field: "methaneList" }));
    for (field, expected) in HISTORY_FIELDS.into_iter().zip([7.0, 5.0, 3.0, 17.0, 19.0, 23.0]).take(5) {
        let Observation::History(Some(list)) = observed_field(&statistics, field) else { panic!("Prefix history"); };
        assert_eq!(list.numeric_prefix(field, 100).unwrap(), vec![expected]);
    }
    assert!(matches!(observed_field(&statistics, "time"), Observation::Long(1)));
    assert!(matches!(observed_field(&statistics, "populationSum"), Observation::Long(7)));
    assert!(matches!(observed_field(&statistics, "birthLastTime"), Observation::Int(3)));
    assert!(matches!(observed_field(&statistics, "deathLastTime"), Observation::Int(5)));
    drop(statistics);

    let world = open(&caller, &authored());
    let statistics = world.statistics_ref();
    let mut statistics = statistics.borrow_mut();
    set_statistics_input(&mut statistics, "birthLastTime", "int", "3", &BTreeMap::new());
    set_statistics_input(&mut statistics, "deathLastTime", "int", "5", &BTreeMap::new());
    let shared = statistics.population_list.as_ref().unwrap().clone();
    statistics.death_list = Some(shared.clone());
    statistics.birth_list = Some(shared.clone());
    statistics.oxygen_list = Some(shared.clone());
    statistics.carbon_dioxide_list = Some(shared.clone());
    statistics.methane_list = Some(shared.clone());
    statistics.event_time(7, 17.0, 19.0, 23.0).unwrap();
    assert_eq!(shared.numeric_prefix("populationList", 100).unwrap(), vec![7.0, 5.0, 3.0, 17.0, 19.0, 23.0]);
    assert!(matches!(observed_field(&statistics, "birthLastTime"), Observation::Int(0)));
    assert!(matches!(observed_field(&statistics, "deathLastTime"), Observation::Int(0)));
    drop(statistics);

    let donor_input = document(include_bytes!("../tests/fixtures/shared_world_owners/member_wrapper_backing-authored.ser"));
    let donor = open(&caller, &donor_input);
    let actor = Rc::clone(&donor.actor_roots()[0]);
    let mut slots: Vec<_> = (0..100).map(|_| SharedSlot::Double(JavaDoubleRef::from_value(1.0))).collect();
    slots.push(SharedSlot::Actor(actor));
    let history = HistoryListRef::from_shared(SharedListRef::from_slots(slots));
    let world = open(&caller, &authored());
    world.statistics_ref().borrow_mut().population_list = Some(history);
    let view = world.refresh_statistics(&world.open_statistics()).unwrap();
    assert_eq!(view.histories.population_list.len(), 101);
    assert_eq!(view.histories.population_list[100], crate::HistorySlotSnapshot::OtherClass("biogenesis.Organism"));
    assert!(view.histories.population_list[..100].iter().all(|value| {
        value == &crate::HistorySlotSnapshot::DoubleBits(1.0_f64.to_bits())
    }));
    assert_eq!(view.plot_series[2].points.len(), 100);

    // Admitted ItemBoundable aliases retain their distinct Item and shared Envelope identities
    for clone_items in [false, true] {
        let input = distinct_history_with_spatial_alias(clone_items);
        let admitted = open(&caller, &input);
        let population = admitted.statistics_ref().borrow().get_population_list().unwrap().shared_ref().clone();
        let spatial = admitted.spatial.borrow();
        let pending = spatial.shared_ref().value().pending.clone().unwrap();
        assert!(!pending.same_instance(&population));
        assert_eq!(pending.len(), population.len());
        for index in 0..pending.len() {
            let left = pending.boundable("itemBoundables", index).unwrap().unwrap();
            let right = population.boundable("populationList", index).unwrap().unwrap();
            assert_eq!(left.identity() == right.identity(), !clone_items);
            let bounds = |item: &crate::shared_world_owners::BoundableRef| {
                let crate::shared_world_owners::BoundableValue::Item { bounds: Some(bounds), .. } = &*item.value() else { panic!("Captured Item bounds"); };
                bounds.clone()
            };
            assert_eq!(bounds(&left).identity(), bounds(&right).identity());
        }
        assert!(matches!(population.numeric_prefix("populationList", 100), Err(ListAccessError::WrongClass {
            index: 0, actual: "org.locationtech.jts.index.strtree.ItemBoundable", ..
        })));
        drop(spatial);
        super::live_spatial_review_tests::assert_captured_graph(&input, &admitted);
        let copied = admitted.clone();
        super::live_spatial_review_tests::assert_captured_graph(&input, &copied);
    }

    let mut direct_envelope = distinct_history_with_spatial_alias(false);
    let (list, envelope) = {
        let graph = LegacyGraph::new(&direct_envelope, Limits::default()).unwrap();
        let (_, statistics) = roots(&graph);
        let list = node(graph.reference(statistics, STATISTICS_CLASS, "populationList").unwrap());
        let item = node(graph.array_list_compatible(list).unwrap()[0]);
        (list, graph.required_reference(item, "org.locationtech.jts.index.strtree.ItemBoundable", "bounds").unwrap())
    };
    let Node::Object(list) = &mut direct_envelope.arena.nodes[list.0 as usize] else { panic!("ArrayList"); };
    let first = list.slots.iter_mut().flat_map(|slot| slot.custom.iter_mut()).find(|record| matches!(record, Record::Value(_))).unwrap();
    *first = Record::Value(Value::Handle(envelope));
    let admitted = open(&caller, &direct_envelope);
    let history = admitted.statistics_ref().borrow().get_population_list().unwrap().shared_ref().clone();
    assert!(matches!(history.numeric_prefix("populationList", 100), Err(ListAccessError::WrongClass {
        index: 0, actual: "org.locationtech.jts.geom.Envelope", ..
    })));
}

#[test]
fn escaped_statistics_keeps_complete_owners_then_releases_without_self_cycle() {
    let caller = caller();
    let input = document(include_bytes!("../tests/fixtures/shared_world_owners/member_wrapper_backing-authored.ser"));
    let world = open(&caller, &input);
    let (actor, association) = {
        let actors = world.actor_roots();
        assert!(!actors.is_empty());
        let association = actors[0].borrow().state.world.clone();
        (Rc::downgrade(&actors[0]), association)
    };
    let statistics = world.open_statistics();
    drop(input);
    drop(world);
    {
        let actor = actor.upgrade().expect("Escaped statistics retains complete actors");
        let owner = actor.borrow().state.world.resolve().expect("Complete associated World remains live");
        let expected = association.resolve().unwrap();
        assert!(owner.handle().same_owner(&expected.handle()));
        assert_eq!(owner.session.random_snapshot(), caller.session.random_snapshot());
    }
    drop(statistics);
    assert!(actor.upgrade().is_none());
    assert!(matches!(association.resolve(), Err(WorldError::ExpiredWorld)));

    let unescaped = open(&caller, &authored());
    let association = unescaped.association();
    drop(unescaped);
    assert!(matches!(association.resolve(), Err(WorldError::ExpiredWorld)));
}

#[test]
fn uncaptured_frame_boundary_appends_shared_history_before_next_frame_cast_failure() {
    let caller = caller();
    let world = open(&caller, &authored());
    let random = world.session.random_snapshot();
    world.frame.set(255);
    let tick = world.tick();
    world.resume();
    assert!(world.step().unwrap());
    assert_eq!((world.tick(), world.frame(), world.time()), (tick + 1, 0, 1));
    assert_eq!((world.oxygen(), world.carbon_dioxide(), world.methane()), (1000.0, 499501.0, 1499.0));
    let statistics = world.statistics_ref();
    let statistics = statistics.borrow();
    for (field, expected) in HISTORY_FIELDS.into_iter().zip([0.0, 0.0, 0.0, 1000.0, 499501.0, 1499.0]) {
        let Observation::History(Some(list)) = observed_field(&statistics, field) else { panic!("Frame history"); };
        assert_eq!(list.numeric_prefix(field, 100).unwrap(), vec![expected]);
    }
    drop(statistics);
    assert!(matches!(world.step(), Err(WorldError::List(ListAccessError::WrongClass {
        field: "_organisms", index: 0, actual: "java.lang.Double", ..
    }))));
    assert_eq!((world.tick(), world.frame(), world.time()), (tick + 1, 0, 1));
    assert_eq!((world.oxygen(), world.carbon_dioxide(), world.methane()), (1000.0, 499501.0, 1499.0));
    assert!(world.paused());
    assert_eq!(world.session.random_snapshot(), random);

    // Source inserts each preceding actor before the later membership cast or null dereference
    let input = document(include_bytes!("../tests/fixtures/shared_world_owners/member_wrapper_backing-authored.ser"));
    let graph = LegacyGraph::new(&input, Limits::default()).unwrap();
    let actor_node = input.arena.nodes.iter().enumerate().find_map(|(index, value)| {
        let Node::Object(_) = value else { return None; };
        let id = NodeId(index as u32);
        (graph.class_name(id).unwrap() == "biogenesis.Organism").then_some(id)
    }).unwrap();
    let x = graph.int(actor_node, "java.awt.Rectangle", "x").unwrap();
    let y = graph.int(actor_node, "java.awt.Rectangle", "y").unwrap();
    let width = graph.int(actor_node, "java.awt.Rectangle", "width").unwrap();
    let height = graph.int(actor_node, "java.awt.Rectangle", "height").unwrap();
    assert!(width > 0 && height > 0);
    let rectangle = [f64::from(x), f64::from(y), f64::from(width), f64::from(height)];
    let envelope = [f64::from(x), f64::from(x) + f64::from(width), f64::from(y), f64::from(y) + f64::from(height)];
    for null_slot in [false, true] {
        let world = open(&caller, &input);
        assert_eq!(world.organisms.len().unwrap(), 1);
        let first = world.organisms.actor(0).unwrap();
        world.organisms.shared_ref().unwrap().slots_mut().push(if null_slot {
            SharedSlot::Null
        } else {
            SharedSlot::Double(JavaDoubleRef::from_value(7.0))
        });
        let clock = (world.tick(), world.frame(), world.time());
        let gases = [world.oxygen().to_bits(), world.carbon_dioxide().to_bits(), world.methane().to_bits()];
        let random = world.session.random_snapshot();
        world.resume();
        let result = world.step();
        if null_slot {
            assert!(matches!(result, Err(WorldError::List(ListAccessError::MissingElement {
                field: "_organisms", index: 1,
            }))));
        } else {
            assert!(matches!(result, Err(WorldError::List(ListAccessError::WrongClass {
                field: "_organisms", index: 1, actual: "java.lang.Double", ..
            }))));
        }
        assert_eq!((world.tick(), world.frame(), world.time()), clock);
        assert_eq!([world.oxygen().to_bits(), world.carbon_dioxide().to_bits(), world.methane().to_bits()], gases);
        assert_eq!(world.session.random_snapshot(), random);
        assert!(world.paused());
        let frame = world.frame_objects.borrow();
        assert_eq!(frame.len(), 1);
        assert!(Rc::ptr_eq(&frame[0], &first));
        let spatial = world.spatial.borrow();
        assert_eq!(spatial.stored_envelopes(), vec![(0, rectangle)]);
        let tree = spatial.stored_state();
        assert!(!tree.built && tree.root.is_none());
        assert_eq!(tree.pending.as_deref(), Some(&[0][..]));
        assert_eq!((tree.boundables.len(), tree.envelopes.len()), (1, 1));
        let actual = tree.envelopes[0];
        assert_eq!([actual.min_x, actual.max_x, actual.min_y, actual.max_y], envelope);
    }
}
