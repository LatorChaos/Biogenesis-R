// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use super::shared_world_runtime_review_tests::assert_captured_scalars;
use crate::history::HistoryListRef;
use crate::legacy_graph::LegacyGraph;
use crate::object_stream::{Document, Limits, NodeId, StreamReader, Value};
use crate::shared_world_owners::{BoundableRef, BoundableValue, SharedListRef, SharedSlot};
use std::collections::HashMap;

const W: &str = "biogenesis.World";
const S: &str = "biogenesis.WorldStatistics";
const T: &str = "org.locationtech.jts.index.strtree.AbstractSTRtree";
const N: &str = "org.locationtech.jts.index.strtree.AbstractNode";
const I: &str = "org.locationtech.jts.index.strtree.ItemBoundable";
const E: &str = "org.locationtech.jts.geom.Envelope";
const FIELDS: [&str; 6] = ["populationList", "deathList", "birthList", "oxygenList", "carbonDioxideList", "methaneList"];

fn fixture(name: &str) -> Document {
    let path = crate::oracle_tests::fixture_path("crates/core")
        .join("tests/fixtures/shared_world_owners").join(format!("{name}.ser"));
    let bytes = std::fs::read(path).unwrap();
    let result = StreamReader::new(&bytes, Limits::default()).unwrap().read_document().unwrap();
    assert_eq!(result.consumed, bytes.len());
    result
}
fn caller() -> World {
    World::with_random(WorldConfig { initial_attempts: 0, ..WorldConfig::default() },
        WorldParameters::default(), JavaRandom::new(7224)).unwrap()
}
fn open(caller: &World, name: &str) -> World {
    let path = crate::oracle_tests::fixture_path("crates/core")
        .join("tests/fixtures/shared_world_owners").join(format!("{name}.ser"));
    let owner = caller.handle();let session = caller.session();let random = caller.random_snapshot();
    let world = caller.load_legacy_file(&path, Limits::default(), Limits::default().allocated_bytes).unwrap();
    assert!(!world.handle().same_owner(&owner));assert!(world.session.same_owner(&session));
    assert!(caller.handle().same_owner(&owner));assert!(caller.session.same_owner(&session));
    assert_eq!(caller.random_snapshot(),random);
    world
}
fn population(world: &World) -> SharedListRef {
    world.statistics_ref().borrow().get_population_list().unwrap().shared_ref().clone()
}
fn root(world: &World) -> BoundableRef {
    world.spatial.borrow().shared_ref().value().root.clone().unwrap()
}
fn children(node: &BoundableRef) -> SharedListRef {
    let BoundableValue::Node { children: Some(value), .. } = &*node.value() else { panic!("Captured node children"); };
    value.clone()
}
fn first_child(node: &BoundableRef) -> BoundableRef {
    children(node).boundable("childBoundables", 0).unwrap().unwrap()
}
fn saved(world: &World) -> Document {
    world.encode_legacy_document(Limits::default(), Limits::default().allocated_bytes).unwrap()
}
fn event(world: &World) {
    world.statistics_ref().borrow_mut().event_time(7, 1000.0, 2000.0, 3000.0).unwrap();
}
fn all_bounds() -> IntRect { IntRect { x: 0, y: 0, width: 1200, height: 800 } }

// Compare captured owner partitions and raw fields, independent of the packing implementation
struct Owners<'a> {
    expected: LegacyGraph<'a>, actual: LegacyGraph<'a>,
    forward: HashMap<u32, u32>, reverse: HashMap<u32, u32>,
}
impl Owners<'_> {
    fn pair(&mut self, left: Value, right: Value) {
        if left == Value::Null && right == Value::Null { return; }
        let (Value::Handle(left), Value::Handle(right)) = (left, right) else { panic!("Captured nullable reference mismatch"); };
        if let Some(previous) = self.forward.get(&left.0) { assert_eq!(*previous, right.0); return; }
        assert!(self.reverse.insert(right.0, left.0).is_none(), "Distinct captured owners merged");
        self.forward.insert(left.0, right.0);
        let class = self.expected.class_name(left).unwrap();
        assert_eq!(class, self.actual.class_name(right).unwrap());
        match class {
            "java.util.ArrayList" => {
                let a = self.expected.array_list_compatible(left).unwrap();
                let b = self.actual.array_list_compatible(right).unwrap();
                assert_eq!(a.len(), b.len());
                for (a,b) in a.into_iter().zip(b) { self.pair(a,b); }
            }
            "java.util.Collections$SynchronizedList" => {
                self.field(left,right,class,"list");
                for field in ["c", "mutex"] { self.field(left,right,"java.util.Collections$SynchronizedCollection",field); }
            }
            "java.lang.Double" => assert_eq!(self.expected.double_bits(left,class,"value").unwrap(),self.actual.double_bits(right,class,"value").unwrap()),
            "org.locationtech.jts.index.strtree.STRtree" => {
                assert_eq!(self.expected.boolean(left,T,"built").unwrap(),self.actual.boolean(right,T,"built").unwrap());
                assert_eq!(self.expected.int(left,T,"nodeCapacity").unwrap(),self.actual.int(right,T,"nodeCapacity").unwrap());
                for field in ["itemBoundables", "root"] { self.field(left,right,T,field); }
            }
            "org.locationtech.jts.index.strtree.ItemBoundable" => for field in ["bounds", "item"] { self.field(left,right,I,field); },
            "org.locationtech.jts.index.strtree.STRtree$STRtreeNode" => {
                assert_eq!(self.expected.int(left,N,"level").unwrap(),self.actual.int(right,N,"level").unwrap());
                for field in ["bounds", "childBoundables"] { self.field(left,right,N,field); }
            }
            "org.locationtech.jts.geom.Envelope" => for field in ["minx", "maxx", "miny", "maxy"] {
                assert_eq!(self.expected.double_bits(left,E,field).unwrap(),self.actual.double_bits(right,E,field).unwrap());
            },
            "biogenesis.Organism" => {
                assert_eq!(self.expected.int(left,class,"_ID").unwrap(),self.actual.int(right,class,"_ID").unwrap());
                for field in ["x", "y", "width", "height"] {
                    assert_eq!(self.expected.int(left,"java.awt.Rectangle",field).unwrap(),self.actual.int(right,"java.awt.Rectangle",field).unwrap());
                }
                self.field(left,right,class,"_world");
            }
            "biogenesis.World" => {},
            _ => panic!("Unexpected captured owner class {class}"),
        }
    }
    fn field(&mut self, left: NodeId, right: NodeId, class: &'static str, field: &'static str) {
        self.pair(self.expected.reference(left,class,field).unwrap(),self.actual.reference(right,class,field).unwrap());
    }
}
pub(super) fn assert_captured_graph(expected: &Document, world: &World) {
    assert_captured_scalars(expected, world);
    let actual = saved(world);
    let mut owners = Owners { expected: LegacyGraph::new(expected,Limits::default()).unwrap(),
        actual: LegacyGraph::new(&actual,Limits::default()).unwrap(), forward: HashMap::new(), reverse: HashMap::new() };
    let left = owners.expected.world_root().unwrap(); let right = owners.actual.world_root().unwrap();
    owners.pair(Value::Handle(left),Value::Handle(right));
    owners.field(left,right,W,"_organisms");
    let ls = owners.expected.required_reference(left,W,"worldStatistics").unwrap();
    let rs = owners.actual.required_reference(right,W,"worldStatistics").unwrap();
    for field in FIELDS { owners.field(ls,rs,S,field); }
    owners.field(left,right,W,"colDetTree");
}

#[test]
fn captured_tree_history_runtime_keeps_failed_build_and_append_prefix() {
    let caller = caller(); let random = caller.random_snapshot();
    let world = open(&caller,"tree_history-authored");
    assert_captured_graph(&fixture("tree_history-authored"),&world);
    let original = population(&world);
    assert!(world.spatial.borrow().shared_ref().value().pending.as_ref().unwrap().same_instance(&original));
    world.statistics_ref().borrow_mut().event_time(7,17.0,19.0,23.0).unwrap();
    assert_captured_graph(&fixture("tree_history-call_1"),&world);
    assert!(matches!(world.spatial.borrow().query_slots(all_bounds()),Err(crate::SpatialAccessError::List { operation:"verticalSlices",error:crate::ListAccessError::WrongClass { actual:"java.lang.Double",.. } })));
    let spatial = world.spatial.borrow();let tree = spatial.shared_ref().value();
    assert!(!tree.built && tree.root.is_none());
    assert!(tree.pending.as_ref().unwrap().same_instance(&original));
    assert_eq!(original.number("populationList",0).unwrap().unwrap().bits(),7.0_f64.to_bits());
    drop(tree);drop(spatial);
    assert_captured_graph(&fixture("tree_history-call_1"),&world);
    assert_eq!(caller.random_snapshot(),random);assert!(world.session.same_owner(&caller.session));
}

#[test]
fn captured_pending_history_runtime_preserves_order_and_failed_reattachment() {
    let caller = caller();let random = caller.random_snapshot();
    let world = open(&caller,"pending_history_build-authored");
    assert_captured_graph(&fixture("pending_history_build-authored"),&world);
    let original = population(&world);
    world.spatial.borrow().build_index().unwrap();
    assert_captured_graph(&fixture("pending_history_build-call_0"),&world);
    let hits = world.spatial.borrow().query_slots(all_bounds()).unwrap();
    assert_eq!(hits.len(),4);
    let members = world.organisms.shared_ref().unwrap();
    for (slot,index) in hits.into_iter().zip([0,3,1,2]) {
        let SharedSlot::Actor(actor) = slot else { panic!("Captured query Actor"); };
        assert!(Rc::ptr_eq(&actor,&members.actor("_organisms",index).unwrap().unwrap()));
    }
    assert_captured_graph(&fixture("pending_history_build-call_1"),&world);
    event(&world);
    assert_captured_graph(&fixture("pending_history_build-call_2"),&world);
    let tree = world.spatial.borrow().shared_ref().clone();
    tree.set_built(false);tree.set_root(None);tree.replace_pending(Some(original.clone()));
    assert!(matches!(world.spatial.borrow().build_index(),Err(crate::SpatialAccessError::List { operation:"compare",error:crate::ListAccessError::WrongClass { actual:"java.lang.Double",.. } })));
    assert_captured_graph(&fixture("pending_history_build-call_3"),&world);
    assert_eq!(original.len(),5);assert_eq!(caller.random_snapshot(),random);
}

#[test]
fn captured_child_history_runtime_keeps_lazy_caches_and_reached_failure() {
    let caller = caller();let random = caller.random_snapshot();
    let world = open(&caller,"child_history_cache-authored");
    assert_captured_graph(&fixture("child_history_cache-authored"),&world);
    world.spatial.borrow().build_index().unwrap();
    assert_captured_graph(&fixture("child_history_cache-call_0"),&world);
    let root = root(&world);let population = children(&root);
    world.statistics_ref().borrow_mut().population_list=Some(HistoryListRef::from_shared(population.clone()));
    event(&world);
    assert_captured_graph(&fixture("child_history_cache-call_1"),&world);
    let first = first_child(&root);
    root.set_bounds(None);first.set_bounds(None);
    assert!(matches!(world.spatial.borrow().query_slots(all_bounds()),Err(crate::SpatialAccessError::List { operation:"computeBounds",error:crate::ListAccessError::WrongClass { actual:"java.lang.Double",.. } })));
    assert_captured_graph(&fixture("child_history_cache-call_1"),&world);
    assert_eq!(population.len(),3);assert_eq!(caller.random_snapshot(),random);
}

#[test]
fn spatial_cross_role_copy_export_reload_and_escaped_statistics_keep_complete_owners() {
    let caller = caller();let expected = fixture("child_history_cache-call_1");
    let world = open(&caller,"child_history_cache-call_1");
    let copied = world.clone();
    assert_captured_graph(&expected,&copied);
    assert!(!population(&world).same_instance(&population(&copied)));
    assert!(children(&root(&copied)).same_instance(&population(&copied)));
    assert!(!root(&world).identity().eq(&root(&copied).identity()));
    let a=first_child(&root(&world));let b=first_child(&root(&copied));
    let ab=match &*a.value() { BoundableValue::Node { bounds:Some(bounds),.. }=>bounds.clone(),_=>panic!("Captured child cache") };
    let bb=match &*b.value() { BoundableValue::Node { bounds:Some(bounds),.. }=>bounds.clone(),_=>panic!("Copied child cache") };
    assert_ne!(ab.identity(),bb.identity());
    assert_eq!(ab.get().min_x.to_bits(),bb.get().min_x.to_bits());
    event(&copied);assert_eq!(population(&copied).len(),4);assert_eq!(population(&world).len(),3);
    assert_captured_graph(&expected,&world);
    let wire=saved(&world);
    let reloaded=caller.decode_legacy_document(&wire,Limits::default(),Limits::default().allocated_bytes).unwrap();
    assert_captured_graph(&expected,&reloaded);
    assert!(children(&root(&reloaded)).same_instance(&population(&reloaded)));
    drop(reloaded);drop(copied);drop(a);drop(b);drop(ab);drop(bb);drop(wire);drop(expected);
    let actor=world.actor_roots().into_iter().next().unwrap();
    let weak=Rc::downgrade(&actor);let association=actor.borrow().state.world.clone();drop(actor);
    let handle=world.open_statistics();drop(world);
    assert!(weak.upgrade().is_some());assert!(association.resolve().is_ok());
    drop(handle);assert!(weak.upgrade().is_none());assert!(matches!(association.resolve(),Err(WorldError::ExpiredWorld)));
}


#[test]
fn removed_imported_actors_expire_with_the_same_empty_tree_and_world_alive() {
    let caller = caller();
    let world = open(&caller,"pending_history_build-authored");
    let tree = world.spatial.borrow().shared_ref().clone();
    let pending = tree.value().pending.clone().unwrap();
    let population = population(&world);
    assert!(pending.same_instance(&population));assert_eq!(pending.len(),4);
    assert!(!tree.value().built && tree.value().root.is_none());
    assert!(world.frame_objects.borrow().is_empty());
    assert!(world.incoming_corridors.borrow().is_empty());
    assert!(world.outgoing_corridors.borrow().is_empty());
    {
        let owner = world.statistics_ref();let statistics = owner.borrow();
        assert!(statistics.retained_genomes().into_iter().all(|value|value.is_none()));
        for history in statistics.history_refs().into_iter().skip(1) {
            assert_eq!(history.unwrap().len(),0);
        }
    }
    let weak = {
        let actors = world.actor_roots();
        let mut identities = std::collections::HashSet::new();
        let result: Vec<_> = actors.iter().filter(|actor|identities.insert(Rc::as_ptr(actor) as usize))
            .map(Rc::downgrade).collect();
        assert_eq!(result.len(),4);
        result
    };
    assert!(world.rectangle_context.borrow().is_some());
    world.organisms.clear().unwrap();
    population.clear();
    assert_eq!(pending.len(),0);assert_eq!(world.organisms.len().unwrap(),0);
    assert!(weak.iter().all(|actor|actor.upgrade().is_some()),"The explicit imported context still owns these targets");
    world.retire_imported_actors().unwrap();
    assert!(world.rectangle_context.borrow().is_none());
    assert!(weak.iter().all(|actor|actor.upgrade().is_none()),"Weak ordinal metadata must not retain removed targets");
    assert!(world.spatial.borrow().imported());
    assert_eq!(world.spatial.borrow().shared_ref().identity(),tree.identity());
    assert!(!tree.value().built && tree.value().root.is_none());
    assert!(tree.value().pending.as_ref().unwrap().same_instance(&pending));
    assert_eq!(pending.len(),0);
}
