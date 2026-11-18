// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::shared_world_owners::{BoundableRef, BoundableValue, EnvelopeRef, SharedListRef, SharedSlot};
use crate::{IntRect, ListAccessError, SpatialAccessError, WorldError};
use std::path::PathBuf;

const CASES: [&str; 9] = [
    "actor-control", "foreign-member-world", "foreign-peer-world", "null-member-world",
    "history-envelope", "membership-envelope", "pending-envelope", "child-envelope", "cache-control",
];
const ENVELOPE: &str = "org.locationtech.jts.geom.Envelope";

fn input_path(case: usize) -> PathBuf {
    let root = crate::oracle_tests::fixture_path("crates/core").join("tests/fixtures");
    match case {
        0 => root.join("world_hydration/actor_closure-call_0.ser"),
        8 => root.join("shared_world_owners/child_history_cache-call_0.ser"),
        _ => root.join("world_owner_roles").join(format!("{}-authored.ser", CASES[case])),
    }
}
fn expected(case: usize, call: usize) -> Document {
    let changed = match (case, call) {
        (1 | 2, 3) => Some(3),
        (4, 3 | 4) => Some(3),
        (7, 4) => Some(4),
        (8, 3) => Some(3),
        _ => None,
    };
    let path = changed.map_or_else(|| input_path(case), |call| {
        crate::oracle_tests::fixture_path("crates/core").join("tests/fixtures/world_owner_roles")
            .join(format!("{}-call_{call}.ser", CASES[case]))
    });
    parse(&std::fs::read(path).unwrap())
}
fn loaded(case: usize) -> World {
    let limits = Limits::default();
    caller().load_legacy_file(&input_path(case), limits, limits.allocated_bytes).unwrap()
}
fn stored(world: &World) -> Document {
    let limits = Limits::default();
    world.encode_legacy_document(limits, limits.allocated_bytes).unwrap()
}
fn checkpoint(case: usize, stage: &str, expected: &Document, world: &World) {
    let actual = stored(world);
    assert!(compare(expected, &actual) > 0, "Complete persistent fields");
    let limits = Limits::default();
    let bytes = world.encode_legacy_file_bytes(limits, limits.allocated_bytes).unwrap();
    compare(expected, &parse(&bytes));
    if let Some(directory) = std::env::var_os("BGR_WORLD_OWNER_READBACK_OUTPUT") {
        let directory = PathBuf::from(directory);
        assert!(directory.is_absolute() && directory.is_dir());
        let path = directory.join(format!("{}-{stage}.bgw", CASES[case]));
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
        file.write_all(&bytes).unwrap();
        file.sync_all().unwrap();
    }
}
fn member(world: &World, index: usize) -> OrganismRef {
    world.organisms.shared_ref().unwrap().actor("_organisms", index).unwrap().unwrap()
}
fn population(world: &World) -> SharedListRef {
    world.statistics_ref().borrow().get_population_list().unwrap().shared_ref().clone()
}
fn tree_root(world: &World) -> BoundableRef {
    world.spatial.borrow().shared_ref().value().root.clone().unwrap()
}
fn children(node: &BoundableRef) -> SharedListRef {
    let BoundableValue::Node { children: Some(children), .. } = &*node.value() else { panic!("Node children"); };
    children.clone()
}
fn bounds(node: &BoundableRef) -> Option<EnvelopeRef> {
    match &*node.value() {
        BoundableValue::Item { bounds, .. } | BoundableValue::Node { bounds, .. } => bounds.clone(),
    }
}
fn all_bounds() -> IntRect { IntRect { x: 0, y: 0, width: 1200, height: 800 } }
fn envelope_error(error: SpatialAccessError) {
    assert!(matches!(error, SpatialAccessError::List { error: ListAccessError::WrongClass { actual: ENVELOPE, .. }, .. }));
}
fn peer(world: &World) -> OrganismRef {
    member(world, 0).borrow().body.size_rect.as_ref().unwrap().actor_target().unwrap()
}

#[test]
fn original_nine_owner_roles_replay_complete_graphs_and_reached_prefixes() {
    let limits = Limits::default();
    let mut calls = 0;
    let mut snapshots = 0;
    let mut errors = 0;
    for case in 0..CASES.len() {
        let caller = caller();
        let owner = caller.handle();
        let session = caller.session();
        let random = caller.random_snapshot();
        let parameters = caller.parameters();
        let mut world = caller.load_legacy_file(&input_path(case), limits, limits.allocated_bytes).unwrap();
        if (1..8).contains(&case) {
            // Authored-memory NaN payloads are distinct from the captured serialized graph
            checkpoint(case, "authored", &expected(case, 0), &world);
            snapshots += 1;
        }
        checkpoint(case, "call_0", &expected(case, 0), &world);
        calls += 1; snapshots += 1;
        assert!(!world.handle().same_owner(&owner));
        assert!(world.session().same_owner(&session));
        let previous = world.handle();
        let rewritten = world.encode_legacy_file_bytes(limits, limits.allocated_bytes).unwrap();
        world = caller.decode_legacy_document(&parse(&rewritten), limits, limits.allocated_bytes).unwrap();
        assert!(!world.handle().same_owner(&previous));
        checkpoint(case, "call_1", &expected(case, 1), &world);
        calls += 1; snapshots += 1;
        // Core admission already supplies the caller Session and empty transient corridors
        assert!(world.incoming_corridors.borrow().is_empty() && world.outgoing_corridors.borrow().is_empty());
        assert!(world.session().same_owner(&session));
        assert_eq!(world.tick(), caller.tick());
        checkpoint(case, "call_2", &expected(case, 2), &world);
        calls += 1; snapshots += 1;
        match case {
            0 => assert!(Rc::ptr_eq(&world.find_organism_from_position([100, 100]).unwrap().unwrap(), &member(&world, 0))),
            1 | 2 => {
                let actor = if case == 1 { member(&world, 0) } else { peer(&world) };
                assert!(actor.borrow_mut().use_energy(1.0).unwrap());
                let foreign = actor.borrow().state.world.resolve().unwrap();
                assert!(!foreign.handle().same_owner(&world.handle()));
                assert_eq!((actor.borrow().state.energy, foreign.oxygen(), foreign.carbon_dioxide()), (9.0, 36.0, 74.0));
                assert_eq!((world.oxygen(), world.carbon_dioxide(), world.methane()), (11.0, 23.0, 41.0));
            },
            3 => {
                let actor = member(&world, 0);
                assert!(matches!(actor.borrow_mut().use_energy(1.0), Err(WorldError::MissingWorld)));
                assert_eq!(actor.borrow().state.energy, 10.0);
                errors += 1;
            },
            4 => world.statistics_ref().borrow_mut().event_time(7, 1000.0, 2000.0, 3000.0).unwrap(),
            5 => assert!(Rc::ptr_eq(&world.find_organism_from_position([100, 100]).unwrap().unwrap(), &member(&world, 0))),
            6 => { envelope_error(world.spatial.borrow().build_index().unwrap_err()); errors += 1; },
            7 => { envelope_error(world.spatial.borrow().query_slots(all_bounds()).unwrap_err()); errors += 1; },
            8 => {
                let hits = world.spatial.borrow().query_slots(all_bounds()).unwrap();
                assert_eq!(hits.len(), 4);
                for (slot, index) in hits.iter().zip([0, 3, 1, 2]) {
                    let SharedSlot::Actor(actor) = slot else { panic!("Captured Actor return"); };
                    assert!(Rc::ptr_eq(actor, &member(&world, index)));
                }
            },
            _ => unreachable!(),
        }
        checkpoint(case, "call_3", &expected(case, 3), &world);
        calls += 1; snapshots += 1;
        if (4..8).contains(&case) {
            match case {
                4 => assert!(matches!(population(&world).numeric_prefix("populationList", 100), Err(ListAccessError::WrongClass { index: 1, actual: ENVELOPE, .. }))),
                5 => assert!(matches!(world.find_organism_from_position([1100, 700]), Err(WorldError::List(ListAccessError::WrongClass { index: 1, actual: ENVELOPE, .. })))),
                6 => envelope_error(world.spatial.borrow().query_slots(all_bounds()).unwrap_err()),
                7 => {
                    let root = tree_root(&world);
                    let first = children(&root).boundable("childBoundables", 0).unwrap().unwrap();
                    root.set_bounds(None); first.set_bounds(None);
                    envelope_error(world.spatial.borrow().query_slots(all_bounds()).unwrap_err());
                    assert!(bounds(&root).is_none());
                    assert!(bounds(&first).is_some());
                },
                _ => unreachable!(),
            }
            errors += 1;
            checkpoint(case, "call_4", &expected(case, 4), &world);
            calls += 1; snapshots += 1;
        }
        assert!(caller.handle().same_owner(&owner));
        assert!(world.session().same_owner(&session));
        assert_eq!(caller.random_snapshot(), random);
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(caller.parameters(), parameters);
        assert_eq!(world.parameters(), parameters);
    }
    assert_eq!((calls, snapshots, errors), (40, 47, 7));
}

#[test]
fn foreign_owner_copy_and_external_handles_retain_then_release_complete_closure() {
    for case in [1, 2] {
        let world = loaded(case);
        let actor = if case == 1 { member(&world, 0) } else { peer(&world) };
        let foreign = actor.borrow().state.world.resolve().unwrap();
        let association = foreign.association();
        let weak = Rc::downgrade(&actor);
        let copied = world.clone();
        compare(&expected(case, 0), &stored(&copied));
        let copied_actor = if case == 1 { member(&copied, 0) } else { peer(&copied) };
        let copied_foreign = copied_actor.borrow().state.world.resolve().unwrap();
        assert!(!Rc::ptr_eq(&actor, &copied_actor));
        assert!(!foreign.handle().same_owner(&copied_foreign.handle()));
        assert!(copied_foreign.session().same_owner(&copied.session()));
        assert!(!copied.session().same_owner(&world.session()));
        assert!(copied_actor.borrow_mut().use_energy(1.0).unwrap());
        compare(&expected(case, 3), &stored(&copied));
        if std::env::var_os("BGR_WORLD_OWNER_READBACK_OUTPUT").is_some() {
            checkpoint(case, "copy-call_3", &expected(case, 3), &copied);
        }
        compare(&expected(case, 0), &stored(&world));
        let copied_association = copied_foreign.association();
        let snapshot = world.handle();
        let statistics = world.open_statistics();
        drop(actor); drop(foreign); drop(world);
        assert!(weak.upgrade().is_some());
        assert_eq!(association.resolve().unwrap().oxygen(), 37.0);
        drop(snapshot);
        assert!(association.resolve().is_ok());
        drop(statistics);
        assert!(weak.upgrade().is_none());
        assert!(matches!(association.resolve(), Err(WorldError::ExpiredWorld)));
        assert_eq!(copied_association.resolve().unwrap().oxygen(), 36.0);
        drop(copied_actor); drop(copied_foreign); drop(copied);
        assert!(matches!(copied_association.resolve(), Err(WorldError::ExpiredWorld)));
    }
}

#[test]
fn envelope_copy_keeps_cross_role_aliases_and_detaches_mutable_raw_values() {
    let world = loaded(7);
    let copied = world.clone();
    let check = |world: &World| {
        let root = tree_root(world);
        let list = children(&root);
        let SharedSlot::Envelope(child) = list.slot("childBoundables", 2).unwrap() else { panic!("Envelope child"); };
        let SharedSlot::Envelope(history) = population(world).slot("populationList", 1).unwrap() else { panic!("Envelope history"); };
        assert_eq!(child.identity(), history.identity());
        assert_eq!(child.identity(), bounds(&root).unwrap().identity());
        child
    };
    let original = check(&world); let detached = check(&copied);
    assert_ne!(original.identity(), detached.identity());
    let value = original.get();
    assert_eq!([value.min_x.to_bits(), value.max_x.to_bits(), value.min_y.to_bits(), value.max_y.to_bits()],
        [0x8000000000000000, 0, 0x7ff8000000000000, 0x7ff0000000000000]);
    compare(&expected(7, 0), &stored(&copied));
    if std::env::var_os("BGR_WORLD_OWNER_READBACK_OUTPUT").is_some() {
        checkpoint(7, "copy-call_0", &expected(7, 0), &copied);
    }
    let mut changed = detached.get(); changed.min_x = 123.0; detached.set(changed);
    assert_eq!(check(&copied).get().min_x, 123.0);
    assert_eq!(original.get().min_x.to_bits(), 0x8000000000000000);
    compare(&expected(7, 0), &stored(&world));

    let equal = EnvelopeRef::new(original.get());
    let list = SharedListRef::from_slots(vec![SharedSlot::Envelope(original.clone()),
        SharedSlot::Envelope(original.clone()), SharedSlot::Envelope(equal)]);
    let budget = crate::shared_world_owners::OwnerBudget::new(Limits::default(), 1 << 20);
    let detached = crate::shared_world_owners::SharedOwnerCopier::new(&budget, &[]).unwrap().list(&list).unwrap();
    let identity = |index| match detached.slot("test", index).unwrap() {
        SharedSlot::Envelope(value) => value.identity(), _ => panic!("Envelope copy"),
    };
    assert_eq!(identity(0), identity(1));
    assert_ne!(identity(0), identity(2));
    assert_ne!(identity(0), original.identity());
}

#[test]
fn multi_world_limits_wrong_classes_and_fresh_retry_preserve_caller_state() {
    let caller = caller();
    let source = expected(2, 0);
    let limits = Limits { handles: source.arena.nodes.len() + 64, allocated_bytes: 16 * 1024 * 1024, ..Limits::default() };
    let owner = caller.handle(); let session = caller.session(); let random = caller.random_snapshot();
    let parameters = caller.parameters();
    let before = stored(&caller);
    let mut low = 0; let mut high = limits.allocated_bytes;
    assert!(caller.decode_legacy_document(&source, limits, high).is_ok());
    while low < high {
        let middle = low + (high - low) / 2;
        if caller.decode_legacy_document(&source, limits, middle).is_ok() { high = middle; } else { low = middle + 1; }
    }
    assert!(low > 0);
    assert!(caller.decode_legacy_document(&source, limits, low - 1).is_err());
    compare(&source, &stored(&caller.decode_legacy_document(&source, limits, low).unwrap()));
    let mut low_work = 0; let mut high_work = limits.work;
    while low_work < high_work {
        let middle = low_work + (high_work - low_work) / 2;
        let limited = Limits { work: middle, ..limits };
        if caller.decode_legacy_document(&source, limited, limits.allocated_bytes).is_ok() {
            high_work = middle;
        } else { low_work = middle + 1; }
    }
    assert!(low_work > 0);
    assert!(caller.decode_legacy_document(&source, Limits { work: low_work - 1, ..limits }, limits.allocated_bytes).is_err());
    compare(&source, &stored(&caller.decode_legacy_document(&source, Limits { work: low_work, ..limits }, limits.allocated_bytes).unwrap()));
    for limited in [Limits { work: 0, ..limits }, Limits { handles: source.arena.nodes.len() - 1, ..limits },
        Limits { depth: 1, ..limits }, Limits { array_elements: 0, ..limits }] {
        assert!(caller.decode_legacy_document(&source, limited, limited.allocated_bytes).is_err());
    }
    let mut wrong = source.clone();
    let (actor, foreign) = {
        let graph = LegacyGraph::new(&wrong, limits).unwrap();
        let root = graph.world_root().unwrap();
        let list = graph.required_reference(root, "biogenesis.World", "_organisms").unwrap();
        let backing = if graph.class_name(list).unwrap() == "java.util.Collections$SynchronizedList" {
            graph.required_reference(list, "java.util.Collections$SynchronizedList", "list").unwrap()
        } else { list };
        let carrier = graph.array_list_compatible(backing).unwrap()[0];
        let Value::Handle(carrier) = carrier else { panic!("Carrier"); };
        let actor = graph.required_reference(carrier, "biogenesis.Organism", "_sizeRect").unwrap();
        let foreign = graph.required_reference(actor, "biogenesis.Organism", "_world").unwrap();
        (actor, foreign)
    };
    fn set(document: &mut Document, node: NodeId, class: &str, field: &str, target: NodeId) {
        let (slot, index) = {
            let Node::Object(object) = &document.arena.nodes[node.0 as usize] else { panic!("Object"); };
            object.slots.iter().enumerate().find_map(|(slot, value)| {
                let Node::Descriptor(descriptor) = &document.arena.nodes[value.descriptor.0 as usize] else { panic!("Descriptor"); };
                let schema = class_schema(descriptor.class);
                (schema.name == class).then(|| (slot, schema.fields.iter().position(|value| value.name == field).unwrap()))
            }).unwrap()
        };
        let Node::Object(object) = &mut document.arena.nodes[node.0 as usize] else { unreachable!(); };
        object.slots[slot].values[index] = FieldValue::Reference(Value::Handle(target));
    }
    set(&mut wrong, foreign, "biogenesis.World", "worldStatistics", actor);
    assert!(caller.decode_legacy_document(&wrong, limits, limits.allocated_bytes).is_err());
    let mut wrong_owner = source.clone();
    set(&mut wrong_owner, actor, "biogenesis.Organism", "_world", actor);
    assert!(caller.decode_legacy_document(&wrong_owner, limits, limits.allocated_bytes).is_err());
    compare(&source, &stored(&caller.decode_legacy_document(&source, limits, limits.allocated_bytes).unwrap()));
    compare(&before, &stored(&caller));
    assert!(caller.handle().same_owner(&owner)); assert!(caller.session().same_owner(&session));
    assert_eq!(caller.random_snapshot(), random);
    assert_eq!(caller.parameters(), parameters);

    // The captured two-World shape is not a runtime World-count limit
    let mut world = loaded(1);
    let foreign = member(&world, 0).borrow().state.world.resolve().unwrap();
    let third = World::with_session(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, world.session()).unwrap();
    member(&foreign, 0).borrow_mut().state.world = third.association();
    let owners = [world.handle(), foreign.handle(), third.handle()];
    world.retain_worlds(&owners, limits, limits.allocated_bytes).unwrap();
    let wire = stored(&world);
    let graph = LegacyGraph::new(&wire, limits).unwrap();
    let worlds = wire.arena.nodes.iter().enumerate().filter(|(index, node)| {
        matches!(node, Node::Object(_)) && graph.class_name(NodeId(*index as u32)).unwrap() == "biogenesis.World"
    }).count();
    assert_eq!(worlds, 3);
    let reopened = caller.decode_legacy_document(&wire, limits, limits.allocated_bytes).unwrap();
    compare(&wire, &stored(&reopened));
}
