// SPDX-License-Identifier: GPL-2.0-or-later

//! Bounded shared lists and complete spatial targets from one parsed document

use crate::history::JavaDoubleRef;
use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::object_stream::{Node, NodeId, Value, class_schema};
use crate::organism::OrganismRef;
use crate::shared_world_owners::{self as owners, ARRAY_LIST, SYNC_LIST, ITEM, NODE, BoundableRef, BoundableValue, EnvelopeRef, SharedListRef, SharedSlot, TreeRef, TreeValue, OwnerRoots};
use crate::spatial::Envelope;
use crate::WorldHandle;
use std::{mem::size_of, rc::Rc};

const ACTOR: &str = "biogenesis.Organism";
const WORLD: &str = "biogenesis.World";
const TREE: &str = "org.locationtech.jts.index.strtree.STRtree";
const ABSTRACT_TREE: &str = "org.locationtech.jts.index.strtree.AbstractSTRtree";
const ABSTRACT_NODE: &str = "org.locationtech.jts.index.strtree.AbstractNode";
const ENVELOPE: &str = "org.locationtech.jts.geom.Envelope";

#[derive(Clone)]
enum Cached { ValidatedDescriptor, List(SharedListRef), Double(JavaDoubleRef), Envelope(EnvelopeRef), Boundable(BoundableRef), Tree(TreeRef), Actor(OrganismRef), World(WorldHandle) }

pub(crate) struct SharedOwnerStager<'graph, 'document> {
    graph: &'graph LegacyGraph<'document>,
    values: Vec<Option<Cached>>,
    active: Vec<bool>,
    worlds: Vec<WorldHandle>,
    failed: Option<GraphError>,
}
impl<'graph, 'document> SharedOwnerStager<'graph, 'document> {
    pub(crate) fn uses_graph(&self, graph: &LegacyGraph<'_>) -> bool { std::ptr::eq(self.graph, graph) }
    pub(crate) fn new(
        graph: &'graph LegacyGraph<'document>,
        actors: &[(NodeId, OrganismRef)],
        worlds: &[(NodeId, WorldHandle)],
    ) -> Result<Self, GraphError> {
        let count = graph.arena().nodes.len();
        if actors.len() > graph.limits().handles || worlds.len() > graph.limits().handles { return Err(error(None, None, GraphErrorKind::Budget)); }
        let mut values = Vec::new(); let mut active = Vec::new(); let mut world_refs = Vec::new();
        owners::reserve(graph, &mut values, count)?; owners::reserve(graph, &mut active, count)?;
        graph.charge(count.checked_mul(2).ok_or(error(None, None, GraphErrorKind::Budget))?)?;
        values.resize_with(count, || None); active.resize(count, false);
        let mut result = Self { graph, values, active, worlds: Vec::new(), failed: None };
        let mut world_keys = Vec::new(); owners::reserve(graph, &mut world_keys, worlds.len())?;
        owners::reserve(graph, &mut world_refs, worlds.len())?;
        for (node, world) in worlds {
            result.object(*node, WORLD)?;
            if result.values[node.0 as usize].is_some() { return Err(error(Some(*node), None, GraphErrorKind::Reference)); }
            world_keys.push(world.alias().association().identity().ok_or(error(Some(*node), None, GraphErrorKind::Reference))?);
            result.values[node.0 as usize] = Some(Cached::World(world.clone()));
            world_refs.push(world.clone());
        }
        unique(graph, &mut world_keys)?;
        let mut actor_keys = Vec::new(); owners::reserve(graph, &mut actor_keys, actors.len())?;
        for (node, actor) in actors {
            result.object(*node, ACTOR)?;
            if result.values[node.0 as usize].is_some() { return Err(error(Some(*node), None, GraphErrorKind::Reference)); }
            let value = actor.borrow();
            let bounds = value.state.bounds.get();
            if value.id != i64::from(graph.int(*node, ACTOR, "_ID")?)
                || [bounds.x, bounds.y, bounds.width, bounds.height] != [graph.int(*node, "java.awt.Rectangle", "x")?, graph.int(*node, "java.awt.Rectangle", "y")?, graph.int(*node, "java.awt.Rectangle", "width")?, graph.int(*node, "java.awt.Rectangle", "height")?]
                || !value.state.bounds.actor_target().is_some_and(|target| Rc::ptr_eq(&target, actor)) {
                return Err(error(Some(*node), None, GraphErrorKind::Reference));
            }
            match graph.reference(*node, ACTOR, "_world")? {
                Value::Null if !value.state.world.is_present() => {},
                Value::Handle(world) => match result.values.get(world.0 as usize).and_then(Option::as_ref) {
                    Some(Cached::World(owner)) if value.state.world.identity() == owner.alias().association().identity() => {},
                    _ => return Err(error(Some(*node), Some("_world"), GraphErrorKind::Reference)),
                },
                _ => return Err(error(Some(*node), Some("_world"), GraphErrorKind::Reference)),
            }
            actor_keys.push(Rc::as_ptr(actor) as usize);
            result.values[node.0 as usize] = Some(Cached::Actor(Rc::clone(actor)));
        }
        unique(graph, &mut actor_keys)?;
        result.worlds = world_refs;
        Ok(result)
    }
    fn object(&mut self, id: NodeId, class: &'static str) -> Result<(), GraphError> {
        self.graph.expect_class(id, class)?;
        for slot in self.graph.slots(id)? {
            // Immutable descriptor nodes cannot also hold staged object values
            self.graph.charge(1)?;
            if !matches!(self.values[slot.descriptor.0 as usize], Some(Cached::ValidatedDescriptor)) {
                self.graph.validate_descriptor(slot.descriptor)?;
                self.values[slot.descriptor.0 as usize] = Some(Cached::ValidatedDescriptor);
            }
            let Node::Descriptor(descriptor) = self.graph.node(slot.descriptor)? else { unreachable!() };
            if class_schema(descriptor.class).name != ARRAY_LIST && !slot.custom.is_empty() { return Err(error(Some(id), None, GraphErrorKind::ContainerData)); }
        }
        Ok(())
    }
    fn begin(&mut self, id: NodeId, depth: usize) -> Result<Option<Cached>, GraphError> {
        self.graph.node(id)?;
        if depth > self.graph.limits().depth { return Err(error(Some(id), None, GraphErrorKind::Budget)); }
        if self.active[id.0 as usize] { return Err(error(Some(id), None, GraphErrorKind::TreeShape)); }
        if let Some(value) = &self.values[id.0 as usize] { return Ok(Some(value.clone())); }
        self.active[id.0 as usize] = true;
        Ok(None)
    }
    fn finish(&mut self, id: NodeId, value: Cached) { self.active[id.0 as usize] = false; self.values[id.0 as usize] = Some(value); }
    pub(crate) fn list(&mut self, value: Value) -> Result<Option<SharedListRef>, GraphError> {
        if let Some(error) = self.failed { return Err(error); }
        let result = self.list_at(value, 0);
        if let Err(error) = result { self.failed = Some(error); }
        result
    }
    fn list_at(&mut self, value: Value, depth: usize) -> Result<Option<SharedListRef>, GraphError> {
        self.graph.charge(1)?;
        let Value::Handle(id) = value else { return Ok(None) };
        if let Some(value) = self.begin(id, depth)? { return match value { Cached::List(value) => Ok(Some(value)), _ => Err(error(Some(id), None, GraphErrorKind::Class)) }; }
        let result = match self.graph.class_name(id)? {
            ARRAY_LIST => {
                self.object(id, ARRAY_LIST)?;
                let values = self.graph.array_list_compatible(id)?;
                self.graph.count_array(values.len())?;
                self.graph.allocate(SharedListRef::backing_allocation())?;
                let mut slots = Vec::new(); owners::reserve(self.graph, &mut slots, values.len())?;
                for value in values { slots.push(self.slot_at(value, depth + 1)?); }
                SharedListRef::from_slots(slots)
            }
            SYNC_LIST => {
                self.object(id, SYNC_LIST)?;
                let backing = self.graph.required_reference(id, SYNC_LIST, "list")?;
                if self.graph.reference(id, "java.util.Collections$SynchronizedCollection", "c")? != Value::Handle(backing)
                    || self.graph.reference(id, "java.util.Collections$SynchronizedCollection", "mutex")? != Value::Handle(id) {
                    return Err(error(Some(id), None, GraphErrorKind::ContainerData));
                }
                self.graph.expect_class(backing, ARRAY_LIST)?;
                let backing = self.list_at(Value::Handle(backing), depth + 1)?.ok_or(error(Some(id), None, GraphErrorKind::Reference))?;
                self.graph.allocate(SharedListRef::wrapper_allocation())?;
                SharedListRef::synchronized(&backing)?
            }
            _ => return Err(error(Some(id), None, GraphErrorKind::Class)),
        };
        self.finish(id, Cached::List(result.clone())); Ok(Some(result))
    }
    fn slot_at(&mut self, value: Value, depth: usize) -> Result<SharedSlot, GraphError> {
        self.graph.charge(1)?;
        let Value::Handle(id) = value else { return Ok(SharedSlot::Null) };
        match self.graph.class_name(id)? {
            ACTOR => match self.values.get(id.0 as usize).and_then(Option::as_ref) { Some(Cached::Actor(actor)) => Ok(SharedSlot::Actor(Rc::clone(actor))), _ => Err(error(Some(id), None, GraphErrorKind::Reference)) },
            "java.lang.Double" => {
                if let Some(value) = self.begin(id, depth)? { return match value { Cached::Double(value) => Ok(SharedSlot::Double(value)), _ => Err(error(Some(id), None, GraphErrorKind::Class)) }; }
                self.object(id, "java.lang.Double")?;
                let bits = self.graph.double_bits(id, "java.lang.Double", "value")?;
                self.graph.allocate(size_of::<u64>() + 2 * size_of::<usize>())?;
                let value = JavaDoubleRef::from_bits(bits); self.finish(id, Cached::Double(value.clone())); Ok(SharedSlot::Double(value))
            }
            ITEM | NODE => Ok(SharedSlot::Boundable(self.boundable_at(id, depth)?)),
            ENVELOPE => Ok(SharedSlot::Envelope(self.envelope_at(value, depth)?.ok_or(error(Some(id), None, GraphErrorKind::Reference))?)),
            _ => Err(error(Some(id), None, GraphErrorKind::Class)),
        }
    }
    fn envelope_at(&mut self, value: Value, depth: usize) -> Result<Option<EnvelopeRef>, GraphError> {
        self.graph.charge(1)?;
        let Value::Handle(id) = value else { return Ok(None) };
        if let Some(value) = self.begin(id, depth)? { return match value { Cached::Envelope(value) => Ok(Some(value)), _ => Err(error(Some(id), None, GraphErrorKind::Class)) }; }
        self.object(id, ENVELOPE)?;
        let value = Envelope { min_x: f64::from_bits(self.graph.double_bits(id, ENVELOPE, "minx")?), max_x: f64::from_bits(self.graph.double_bits(id, ENVELOPE, "maxx")?), min_y: f64::from_bits(self.graph.double_bits(id, ENVELOPE, "miny")?), max_y: f64::from_bits(self.graph.double_bits(id, ENVELOPE, "maxy")?) };
        self.graph.allocate(EnvelopeRef::allocation())?;
        let value = EnvelopeRef::new(value); self.finish(id, Cached::Envelope(value.clone())); Ok(Some(value))
    }
    fn boundable_at(&mut self, id: NodeId, depth: usize) -> Result<BoundableRef, GraphError> {
        self.graph.charge(1)?;
        if let Some(value) = self.begin(id, depth)? { return match value { Cached::Boundable(value) => Ok(value), _ => Err(error(Some(id), None, GraphErrorKind::Class)) }; }
        let value = match self.graph.class_name(id)? {
            ITEM => { self.object(id, ITEM)?; BoundableValue::Item { bounds: self.envelope_at(self.graph.reference(id, ITEM, "bounds")?, depth + 1)?, item: self.slot_at(self.graph.reference(id, ITEM, "item")?, depth + 1)? } }
            NODE => {
                self.object(id, NODE)?;
                let level = self.graph.int(id, ABSTRACT_NODE, "level")?;
                let bounds = self.envelope_at(self.graph.reference(id, ABSTRACT_NODE, "bounds")?, depth + 1)?;
                let children = self.graph.reference(id, ABSTRACT_NODE, "childBoundables")?;
                if let Value::Handle(children) = children { self.graph.expect_class(children, ARRAY_LIST)?; }
                BoundableValue::Node { level, bounds, children: self.list_at(children, depth + 1)? }
            }
            _ => return Err(error(Some(id), None, GraphErrorKind::Class)),
        };
        self.graph.allocate(BoundableRef::allocation())?;
        let value = BoundableRef::new(value); self.finish(id, Cached::Boundable(value.clone())); Ok(value)
    }
    pub(crate) fn tree(&mut self, value: Value) -> Result<Option<TreeRef>, GraphError> {
        if let Some(error) = self.failed { return Err(error); }
        let result = self.tree_inner(value);
        if let Err(error) = result { self.failed = Some(error); }
        result
    }
    fn tree_inner(&mut self, value: Value) -> Result<Option<TreeRef>, GraphError> {
        self.graph.charge(1)?;
        let Value::Handle(id) = value else { return Ok(None) };
        if let Some(value) = self.begin(id, 0)? { return match value { Cached::Tree(value) => Ok(Some(value)), _ => Err(error(Some(id), None, GraphErrorKind::Class)) }; }
        self.object(id, TREE)?;
        let pending = self.graph.reference(id, ABSTRACT_TREE, "itemBoundables")?;
        if let Value::Handle(pending) = pending { self.graph.expect_class(pending, ARRAY_LIST)?; }
        let root = match self.graph.reference(id, ABSTRACT_TREE, "root")? {
            Value::Null => None,
            Value::Handle(root) => { self.graph.expect_class(root, NODE)?; Some(self.boundable_at(root, 1)?) },
        };
        let value = TreeValue { built: self.graph.boolean(id, ABSTRACT_TREE, "built")?, node_capacity: self.graph.int(id, ABSTRACT_TREE, "nodeCapacity")?, pending: self.list_at(pending, 1)?, root };
        self.graph.allocate(TreeRef::allocation())?;
        let value = TreeRef::new(value); self.finish(id, Cached::Tree(value.clone())); Ok(Some(value))
    }
    pub(crate) fn roots(&mut self, lists: &[Value], trees: &[Value]) -> Result<OwnerRoots, GraphError> {
        if let Some(error) = self.failed { return Err(error); }
        let result = self.roots_inner(lists, trees);
        if let Err(error) = result { self.failed = Some(error); }
        result
    }
    fn roots_inner(&mut self, lists: &[Value], trees: &[Value]) -> Result<OwnerRoots, GraphError> {
        if lists.len().checked_add(trees.len()).is_none_or(|count| count > self.graph.limits().records) { return Err(error(None, None, GraphErrorKind::Budget)); }
        let mut list_roots = Vec::new(); let mut tree_roots = Vec::new();
        owners::reserve(self.graph, &mut list_roots, lists.len())?; owners::reserve(self.graph, &mut tree_roots, trees.len())?;
        for value in lists { list_roots.push(self.list(*value)?); }
        for value in trees { tree_roots.push(self.tree(*value)?); }
        OwnerRoots::new(list_roots, tree_roots, &self.worlds, self.graph)
    }
}
fn error(node: Option<NodeId>, field: Option<&'static str>, kind: GraphErrorKind) -> GraphError { GraphError { node, field, kind } }
fn unique(graph: &LegacyGraph<'_>, keys: &mut [usize]) -> Result<(), GraphError> {
    let work = keys.len().checked_mul((usize::BITS - keys.len().max(1).leading_zeros()) as usize + 1).ok_or(error(None, None, GraphErrorKind::Budget))?;
    graph.charge(work)?;
    keys.sort_unstable();
    if keys.windows(2).any(|pair| pair[0] == pair[1]) { return Err(error(None, None, GraphErrorKind::Reference)); }
    Ok(())
}
