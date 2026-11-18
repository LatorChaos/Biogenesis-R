// SPDX-License-Identifier: GPL-2.0-or-later

//! Shared list and complete spatial targets, independent of wire arenas and World publication
//! Callers charge allocations before invoking the data-only constructors

use crate::history::JavaDoubleRef;
use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::object_stream::Limits;
use crate::organism::OrganismRef;
use crate::spatial::Envelope;
use crate::WorldHandle;
use std::{cell::{Cell, Ref, RefCell}, collections::HashMap, fmt, mem::size_of, rc::Rc};

pub(crate) const ARRAY_LIST: &str = "java.util.ArrayList";
pub(crate) const SYNC_LIST: &str = "java.util.Collections$SynchronizedList";
pub(crate) const NODE: &str = "org.locationtech.jts.index.strtree.STRtree$STRtreeNode";
pub(crate) const ITEM: &str = "org.locationtech.jts.index.strtree.ItemBoundable";

fn budget_error() -> GraphError { GraphError { node: None, field: None, kind: GraphErrorKind::Budget } }
fn reference_error() -> GraphError { GraphError { node: None, field: None, kind: GraphErrorKind::Reference } }
fn product(count: usize, width: usize) -> Result<usize, GraphError> { count.checked_mul(width).ok_or_else(budget_error) }

/// The same operation counter is used for parsed and detached owner graphs
pub(crate) trait OwnerMeter {
    fn limits(&self) -> Limits;
    fn allocate(&self, bytes: usize) -> Result<(), GraphError>;
    fn visit(&self, count: usize) -> Result<(), GraphError>;
    fn array(&self, count: usize) -> Result<(), GraphError>;
}
impl OwnerMeter for LegacyGraph<'_> {
    fn limits(&self) -> Limits { LegacyGraph::limits(self) }
    fn allocate(&self, bytes: usize) -> Result<(), GraphError> { LegacyGraph::allocate(self, bytes) }
    fn visit(&self, count: usize) -> Result<(), GraphError> { self.charge(count) }
    fn array(&self, count: usize) -> Result<(), GraphError> { self.count_array(count) }
}
pub(crate) struct OwnerBudget { limits: Limits, bytes: Cell<usize>, work: Cell<usize>, elements: Cell<usize> }
impl OwnerBudget {
    pub(crate) fn new(mut limits: Limits, maximum: usize) -> Self {
        limits.allocated_bytes = limits.allocated_bytes.min(maximum);
        Self { limits, bytes: Cell::new(0), work: Cell::new(0), elements: Cell::new(0) }
    }
    pub(crate) fn allocated_bytes(&self) -> usize { self.bytes.get() }
}
impl OwnerMeter for OwnerBudget {
    fn limits(&self) -> Limits { self.limits }
    fn allocate(&self, bytes: usize) -> Result<(), GraphError> {
        let next = self.bytes.get().checked_add(bytes).ok_or_else(budget_error)?;
        if next > self.limits.allocated_bytes { return Err(budget_error()); }
        self.bytes.set(next); Ok(())
    }
    fn visit(&self, count: usize) -> Result<(), GraphError> {
        let next = self.work.get().checked_add(count).ok_or_else(budget_error)?;
        if next > self.limits.work { return Err(budget_error()); }
        self.work.set(next); Ok(())
    }
    fn array(&self, count: usize) -> Result<(), GraphError> {
        let next = self.elements.get().checked_add(count).ok_or_else(budget_error)?;
        if count > self.limits.array_elements || next > self.limits.total_array_elements { return Err(budget_error()); }
        self.visit(count)?; self.elements.set(next); Ok(())
    }
}
/// Live Clone retains its infallible allocation contract, independent of wire admission quotas
pub(crate) struct RuntimeCopyMeter(OwnerBudget);
impl RuntimeCopyMeter {
    pub(crate) fn new() -> Self {
        let limits = Limits {
            handles: usize::MAX, array_elements: usize::MAX, total_array_elements: usize::MAX,
            records: usize::MAX, depth: usize::MAX, allocated_bytes: usize::MAX, work: usize::MAX,
            ..Limits::default()
        };
        Self(OwnerBudget::new(limits, usize::MAX))
    }
}
impl OwnerMeter for RuntimeCopyMeter {
    fn limits(&self) -> Limits { self.0.limits() }
    fn allocate(&self, bytes: usize) -> Result<(), GraphError> { self.0.allocate(bytes) }
    fn visit(&self, count: usize) -> Result<(), GraphError> { self.0.visit(count) }
    fn array(&self, count: usize) -> Result<(), GraphError> { self.0.array(count) }
}
pub(crate) fn reserve<T>(meter: &impl OwnerMeter, vector: &mut Vec<T>, additional: usize) -> Result<(), GraphError> {
    let needed = vector.len().checked_add(additional).ok_or_else(budget_error)?;
    if needed > vector.capacity() {
        let before = vector.capacity();
        meter.allocate(product(needed - before, size_of::<T>())?)?;
        vector.try_reserve_exact(additional).map_err(|_| GraphError { node: None, field: None, kind: GraphErrorKind::Allocation })?;
        meter.allocate(product(vector.capacity() - needed, size_of::<T>())?)?;
    }
    Ok(())
}
pub(crate) fn reserve_map<K: Eq + std::hash::Hash, V>(meter: &impl OwnerMeter, map: &mut HashMap<K, V>) -> Result<(), GraphError> {
    if map.len() == map.capacity() {
        meter.visit(map.len())?;
        let before = map.capacity();
        let bound = before.checked_mul(2).and_then(|n| n.checked_add(4)).ok_or_else(budget_error)?;
        // Precharge the whole growth buffer, including conservative bucket/control storage
        let unit = 2 * size_of::<(K, V)>();
        meter.allocate(product(bound, unit)?)?;
        map.try_reserve(1).map_err(|_| GraphError { node: None, field: None, kind: GraphErrorKind::Allocation })?;
        if map.capacity() > bound { meter.allocate(product(map.capacity() - bound, unit)?)?; }
    }
    Ok(())
}

#[derive(Clone)]
pub(crate) enum SharedSlot { Null, Double(JavaDoubleRef), Actor(OrganismRef), Boundable(BoundableRef), Envelope(EnvelopeRef) }
impl SharedSlot {
    pub(crate) fn class_name(&self) -> &'static str {
        match self { Self::Null => "null", Self::Double(_) => "java.lang.Double", Self::Actor(_) => "biogenesis.Organism", Self::Boundable(value) => value.class_name(), Self::Envelope(_) => "org.locationtech.jts.geom.Envelope" }
    }
}
impl fmt::Debug for SharedSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.class_name()) }
}
#[derive(Clone)]
pub(crate) struct BackingList(Rc<RefCell<Vec<SharedSlot>>>);
pub(crate) struct SynchronizedList { backing: BackingList }
#[derive(Clone)]
pub(crate) enum SharedListRef { Array(BackingList), Synchronized(Rc<SynchronizedList>) }
impl fmt::Debug for SharedListRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_tuple(self.class_name()).field(&self.identity()).field(&self.len()).finish() }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListAccessError {
    MissingList { field: &'static str }, MissingElement { field: &'static str, index: usize },
    Index { field: &'static str, index: usize, length: usize },
    WrongClass { field: &'static str, index: usize, expected: &'static str, actual: &'static str },
    Capacity { field: &'static str },
}
impl fmt::Display for ListAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingList { field } => write!(f, "List {field} is null"),
            Self::MissingElement { field, index } => write!(f, "List {field} element {index} is null"),
            Self::Index { field, index, length } => write!(f, "List {field} index {index} exceeds length {length}"),
            Self::WrongClass { field, index, expected, actual } => write!(f, "List {field} element {index} requires {expected}, found {actual}"),
            Self::Capacity { field } => write!(f, "List {field} cannot grow"),
        }
    }
}
impl std::error::Error for ListAccessError {}
impl SharedListRef {
    pub(crate) fn from_slots(slots: Vec<SharedSlot>) -> Self { Self::Array(BackingList(Rc::new(RefCell::new(slots)))) }
    pub(crate) fn synchronized(backing: &Self) -> Result<Self, GraphError> {
        let Self::Array(backing) = backing else { return Err(reference_error()) };
        Ok(Self::Synchronized(Rc::new(SynchronizedList { backing: backing.clone() })))
    }
    fn backing(&self) -> &BackingList { match self { Self::Array(value) => value, Self::Synchronized(value) => &value.backing } }
    pub(crate) fn backing_ref(&self) -> Self { Self::Array(self.backing().clone()) }
    pub(crate) fn identity(&self) -> usize { match self { Self::Array(value) => Rc::as_ptr(&value.0) as usize, Self::Synchronized(value) => Rc::as_ptr(value) as usize } }
    pub(crate) fn backing_identity(&self) -> usize { Rc::as_ptr(&self.backing().0) as usize }
    pub(crate) fn class_name(&self) -> &'static str { match self { Self::Array(_) => ARRAY_LIST, Self::Synchronized(_) => "java.util.Collections$SynchronizedRandomAccessList" } }
    pub(crate) fn wire_class_name(&self) -> &'static str { match self { Self::Array(_) => ARRAY_LIST, Self::Synchronized(_) => SYNC_LIST } }
    pub(crate) fn same_instance(&self, other: &Self) -> bool { self.wire_class_name() == other.wire_class_name() && self.identity() == other.identity() }
    pub(crate) fn shares_backing(&self, other: &Self) -> bool { self.backing_identity() == other.backing_identity() }
    pub(crate) fn slots(&self) -> Ref<'_, Vec<SharedSlot>> { self.backing().0.borrow() }
    #[cfg(test)]
    pub(crate) fn slots_mut(&self) -> std::cell::RefMut<'_, Vec<SharedSlot>> { self.backing().0.borrow_mut() }
    pub(crate) fn len(&self) -> usize { self.backing().0.borrow().len() }
    pub(crate) fn slot(&self, field: &'static str, index: usize) -> Result<SharedSlot, ListAccessError> {
        let values = self.slots(); values.get(index).cloned().ok_or(ListAccessError::Index { field, index, length: values.len() })
    }
    pub(crate) fn actor(&self, field: &'static str, index: usize) -> Result<Option<OrganismRef>, ListAccessError> {
        match self.slot(field, index)? { SharedSlot::Null => Ok(None), SharedSlot::Actor(value) => Ok(Some(value)), value => Err(ListAccessError::WrongClass { field, index, expected: "biogenesis.Organism", actual: value.class_name() }) }
    }
    pub(crate) fn number(&self, field: &'static str, index: usize) -> Result<Option<JavaDoubleRef>, ListAccessError> {
        match self.slot(field, index)? { SharedSlot::Null => Ok(None), SharedSlot::Double(value) => Ok(Some(value)), value => Err(ListAccessError::WrongClass { field, index, expected: "java.lang.Double", actual: value.class_name() }) }
    }
    pub(crate) fn boundable(&self, field: &'static str, index: usize) -> Result<Option<BoundableRef>, ListAccessError> {
        match self.slot(field, index)? { SharedSlot::Null => Ok(None), SharedSlot::Boundable(value) => Ok(Some(value)), value => Err(ListAccessError::WrongClass { field, index, expected: "org.locationtech.jts.index.strtree.Boundable", actual: value.class_name() }) }
    }
    pub(crate) fn numeric_prefix(&self, field: &'static str, count: usize) -> Result<Vec<f64>, ListAccessError> {
        let mut result = Vec::new(); let length = self.len().min(count);
        result.try_reserve_exact(length).map_err(|_| ListAccessError::Capacity { field })?;
        for index in 0..length { result.push(self.number(field, index)?.ok_or(ListAccessError::MissingElement { field, index })?.value()); }
        Ok(result)
    }
    pub(crate) fn append_history(&self, field: &'static str, value: f64, meter: &impl OwnerMeter) -> Result<(), ListAccessError> {
        let mut slots = self.backing().0.borrow_mut();
        if slots.len() == 100 { slots.remove(0); }
        if slots.len() >= meter.limits().array_elements { return Err(ListAccessError::Capacity { field }); }
        meter.array(1).map_err(|_| ListAccessError::Capacity { field })?;
        meter.allocate(size_of::<u64>() + 2 * size_of::<usize>()).map_err(|_| ListAccessError::Capacity { field })?;
        reserve(meter, &mut *slots, 1).map_err(|_| ListAccessError::Capacity { field })?;
        slots.push(SharedSlot::Double(JavaDoubleRef::from_value(value))); Ok(())
    }
    pub(crate) fn append_runtime_number(&self, field: &'static str, value: f64) -> Result<(), ListAccessError> {
        let mut slots = self.backing().0.borrow_mut();
        if slots.len() == 100 { slots.remove(0); }
        slots.try_reserve(1).map_err(|_| ListAccessError::Capacity { field })?;
        slots.push(SharedSlot::Double(JavaDoubleRef::from_value(value)));
        Ok(())
    }
    pub(crate) fn push_actor(&self, field: &'static str, actor: OrganismRef) -> Result<(), ListAccessError> {
        let mut slots = self.backing().0.borrow_mut();
        slots.try_reserve(1).map_err(|_| ListAccessError::Capacity { field })?;
        slots.push(SharedSlot::Actor(actor));
        Ok(())
    }
    pub(crate) fn push_boundable(&self, field: &'static str, value: BoundableRef) -> Result<(), ListAccessError> {
        let mut slots = self.backing().0.borrow_mut();
        slots.try_reserve(1).map_err(|_| ListAccessError::Capacity { field })?;
        slots.push(SharedSlot::Boundable(value));
        Ok(())
    }
    pub(crate) fn clear(&self) { self.backing().0.borrow_mut().clear(); }
    pub(crate) fn remove(&self, field: &'static str, index: usize) -> Result<SharedSlot, ListAccessError> {
        let mut slots = self.backing().0.borrow_mut();
        if index >= slots.len() { return Err(ListAccessError::Index { field, index, length: slots.len() }); }
        Ok(slots.remove(index))
    }
    pub(crate) fn backing_allocation() -> usize { size_of::<RefCell<Vec<SharedSlot>>>() + 2 * size_of::<usize>() }
    pub(crate) fn wrapper_allocation() -> usize { size_of::<SynchronizedList>() + 2 * size_of::<usize>() }
}

#[derive(Clone)]
pub(crate) struct EnvelopeRef(Rc<Cell<Envelope>>);
impl EnvelopeRef {
    pub(crate) fn new(value: Envelope) -> Self { Self(Rc::new(Cell::new(value))) }
    pub(crate) fn get(&self) -> Envelope { self.0.get() }
    pub(crate) fn set(&self, value: Envelope) { self.0.set(value); }
    pub(crate) fn identity(&self) -> usize { Rc::as_ptr(&self.0) as usize }
    pub(crate) fn allocation() -> usize { size_of::<Cell<Envelope>>() + 2 * size_of::<usize>() }
}
impl fmt::Debug for EnvelopeRef { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_tuple("Envelope").field(&self.identity()).finish() } }
#[derive(Clone, Debug)]
pub(crate) enum BoundableValue {
    Item { bounds: Option<EnvelopeRef>, item: SharedSlot },
    Node { level: i32, bounds: Option<EnvelopeRef>, children: Option<SharedListRef> },
}
#[derive(Clone)]
pub(crate) struct BoundableRef(Rc<RefCell<BoundableValue>>);
impl BoundableRef {
    pub(crate) fn new(value: BoundableValue) -> Self { Self(Rc::new(RefCell::new(value))) }
    pub(crate) fn value(&self) -> Ref<'_, BoundableValue> { self.0.borrow() }
    pub(crate) fn identity(&self) -> usize { Rc::as_ptr(&self.0) as usize }
    pub(crate) fn class_name(&self) -> &'static str { match &*self.value() { BoundableValue::Item { .. } => ITEM, BoundableValue::Node { .. } => NODE } }
    pub(crate) fn set_bounds(&self, value: Option<EnvelopeRef>) { match &mut *self.0.borrow_mut() { BoundableValue::Item { bounds, .. } | BoundableValue::Node { bounds, .. } => *bounds = value } }
    pub(crate) fn set_children(&self, children: Option<SharedListRef>) {
        let mut value = self.0.borrow_mut();
        let BoundableValue::Node { children: target, .. } = &mut *value else { panic!("Only a node has child boundables"); };
        *target = children;
    }
    pub(crate) fn allocation() -> usize { size_of::<RefCell<BoundableValue>>() + 2 * size_of::<usize>() }
}
impl fmt::Debug for BoundableRef { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_tuple(self.class_name()).field(&self.identity()).finish() } }
#[derive(Clone, Debug)]
pub(crate) struct TreeValue { pub built: bool, pub node_capacity: i32, pub pending: Option<SharedListRef>, pub root: Option<BoundableRef> }
#[derive(Clone)]
pub(crate) struct TreeRef(Rc<RefCell<TreeValue>>);
impl TreeRef {
    pub(crate) fn new(value: TreeValue) -> Self { Self(Rc::new(RefCell::new(value))) }
    pub(crate) fn value(&self) -> Ref<'_, TreeValue> { self.0.borrow() }
    pub(crate) fn identity(&self) -> usize { Rc::as_ptr(&self.0) as usize }
    pub(crate) fn replace_pending(&self, value: Option<SharedListRef>) { self.0.borrow_mut().pending = value; }
    pub(crate) fn set_built(&self, built: bool) { self.0.borrow_mut().built = built; }
    pub(crate) fn set_root(&self, root: Option<BoundableRef>) { self.0.borrow_mut().root = root; }
    pub(crate) fn complete_build(&self, root: BoundableRef) {
        let mut value = self.0.borrow_mut();
        value.root = Some(root);
        value.pending = None;
        value.built = true;
    }
    pub(crate) fn allocation() -> usize { size_of::<RefCell<TreeValue>>() + 2 * size_of::<usize>() }
}
impl fmt::Debug for TreeRef { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_tuple("STRtree").field(&self.identity()).finish() } }

/// Strong leases live outside list/boundable owners, avoiding actor-to-World ownership cycles
pub(crate) struct OwnerRetention { pub actors: Vec<OrganismRef>, pub worlds: Vec<WorldHandle> }
pub(crate) struct OwnedList { pub list: SharedListRef, retention: Rc<OwnerRetention> }
impl Clone for OwnedList { fn clone(&self) -> Self { Self { list: self.list.clone(), retention: Rc::clone(&self.retention) } } }
impl OwnedList { pub(crate) fn retained_actors(&self) -> usize { self.retention.actors.len() } }

pub(crate) struct OwnerRoots { pub lists: Vec<Option<SharedListRef>>, pub trees: Vec<Option<TreeRef>>, retention: Rc<OwnerRetention> }
impl OwnerRoots {
    pub(crate) fn new(lists: Vec<Option<SharedListRef>>, trees: Vec<Option<TreeRef>>, worlds: &[WorldHandle], meter: &impl OwnerMeter) -> Result<Self, GraphError> {
        let retention = retain_targets(&lists, &trees, worlds, meter)?;
        Ok(Self { lists, trees, retention })
    }
    pub(crate) fn refresh_retention(&mut self, worlds: &[WorldHandle], meter: &impl OwnerMeter) -> Result<(), GraphError> {
        let next = retain_targets(&self.lists, &self.trees, worlds, meter)?;
        self.retention = next; Ok(())
    }
    pub(crate) fn external_list(&self, index: usize, meter: &impl OwnerMeter) -> Result<Option<OwnedList>, GraphError> {
        let list = self.lists.get(index).ok_or_else(reference_error)?.clone();
        list.map(|list| {
            let retention = retain_targets(&[Some(list.clone())], &[], &self.retention.worlds, meter)?;
            Ok(OwnedList { list, retention })
        }).transpose()
    }
    pub(crate) fn retained_actors(&self) -> usize { self.retention.actors.len() }
}

fn retain_targets(lists: &[Option<SharedListRef>], trees: &[Option<TreeRef>], worlds: &[WorldHandle], meter: &impl OwnerMeter) -> Result<Rc<OwnerRetention>, GraphError> {
    let mut pending = Vec::new();
    for list in lists.iter().flatten() { reserve(meter, &mut pending, 1)?; pending.push(Walk::List(list.clone())); }
    for tree in trees.iter().flatten() { reserve(meter, &mut pending, 1)?; pending.push(Walk::Tree(tree.clone())); }
    let mut visited = HashMap::new(); let mut actors = Vec::new(); let mut retained_worlds = Vec::new();
    while let Some(value) = pending.pop() {
        meter.visit(1)?;
        let key = value.key();
        if visited.contains_key(&key) { continue; }
        if visited.len() >= meter.limits().handles { return Err(budget_error()); }
        reserve_map(meter, &mut visited)?; visited.insert(key, ());
        let mut push = |value| -> Result<(), GraphError> { reserve(meter, &mut pending, 1)?; pending.push(value); Ok(()) };
        match value {
            Walk::List(list) => { for slot in list.slots().iter() { meter.visit(1)?; match slot { SharedSlot::Actor(actor) => push(Walk::Actor(Rc::clone(actor)))?, SharedSlot::Boundable(node) => push(Walk::Boundable(node.clone()))?, _ => {} } } }
            Walk::Tree(tree) => { let tree = tree.value(); if let Some(list) = &tree.pending { push(Walk::List(list.clone()))?; } if let Some(node) = &tree.root { push(Walk::Boundable(node.clone()))?; } }
            Walk::Boundable(node) => match &*node.value() {
                BoundableValue::Item { item: SharedSlot::Actor(actor), .. } => push(Walk::Actor(Rc::clone(actor)))?,
                BoundableValue::Item { item: SharedSlot::Boundable(node), .. } => push(Walk::Boundable(node.clone()))?,
                BoundableValue::Node { children: Some(list), .. } => push(Walk::List(list.clone()))?, _ => {},
            },
            Walk::Actor(actor) => {
                let value = actor.borrow();
                for rect in [Some(&value.state.bounds), value.body.size_rect.as_ref(), value.state.last_frame.as_ref()].into_iter().flatten() {
                    meter.visit(1)?;
                    if rect.is_actor() { push(Walk::Actor(rect.actor_target().ok_or_else(reference_error)?))?; }
                }
                if let Some(identity) = value.state.world.identity() {
                    meter.visit(worlds.len())?;
                    let owner = worlds.iter().find(|world| world.alias().association().identity() == Some(identity)).ok_or_else(reference_error)?;
                    if !retained_worlds.iter().any(|previous: &WorldHandle| previous.same_owner(owner)) { reserve(meter, &mut retained_worlds, 1)?; retained_worlds.push(owner.clone()); }
                }
                drop(value); reserve(meter, &mut actors, 1)?; actors.push(actor);
            }
        }
    }
    meter.allocate(size_of::<OwnerRetention>() + 2 * size_of::<usize>())?;
    Ok(Rc::new(OwnerRetention { actors, worlds: retained_worlds }))
}
enum Walk { List(SharedListRef), Tree(TreeRef), Boundable(BoundableRef), Actor(OrganismRef) }
impl Walk { fn key(&self) -> (u8, usize) { match self { Self::List(value) => (0, value.backing_identity()), Self::Tree(value) => (1, value.identity()), Self::Boundable(value) => (2, value.identity()), Self::Actor(value) => (3, Rc::as_ptr(value) as usize) } } }


#[derive(Clone)]
enum CopyValue { List(SharedListRef), Envelope(EnvelopeRef), Boundable(BoundableRef), Tree(TreeRef) }
struct CopyEntry { _source: CopyValue, target: CopyValue }

/// Actor copies come from the caller's complete graph map, never from numeric organism IDs
pub(crate) struct SharedOwnerCopier<'meter, M: OwnerMeter> {
    meter: &'meter M,
    actors: HashMap<usize, (OrganismRef, OrganismRef)>,
    values: HashMap<(u8, usize), CopyEntry>,
    active: HashMap<(u8, usize), ()>,
    failed: Option<GraphError>,
}
impl<'meter, M: OwnerMeter> SharedOwnerCopier<'meter, M> {
    pub(crate) fn new(meter: &'meter M, actors: &[(OrganismRef, OrganismRef)]) -> Result<Self, GraphError> {
        if actors.len() > meter.limits().handles { return Err(budget_error()); }
        let mut result = Self { meter, actors: HashMap::new(), values: HashMap::new(), active: HashMap::new(), failed: None };
        let mut target_keys = Vec::new(); reserve(meter, &mut target_keys, actors.len())?;
        for (source, target) in actors {
            meter.visit(1)?;
            let key = Rc::as_ptr(source) as usize;
            if Rc::ptr_eq(source, target) { return Err(reference_error()); }
            if result.actors.contains_key(&key) { return Err(reference_error()); }
            if !target.borrow().state.bounds.actor_target().is_some_and(|owner| Rc::ptr_eq(&owner, target)) { return Err(reference_error()); }
            reserve_map(meter, &mut result.actors)?;
            result.actors.insert(key, (Rc::clone(source), Rc::clone(target)));
            target_keys.push(Rc::as_ptr(target) as usize);
        }
        meter.visit(target_keys.len().checked_mul((usize::BITS - target_keys.len().max(1).leading_zeros()) as usize + 1).ok_or_else(budget_error)?)?;
        target_keys.sort_unstable();
        if target_keys.windows(2).any(|pair| pair[0] == pair[1]) { return Err(reference_error()); }
        Ok(result)
    }
    fn begin(&mut self, key: (u8, usize), depth: usize) -> Result<Option<CopyValue>, GraphError> {
        self.meter.visit(1)?;
        if depth > self.meter.limits().depth { return Err(budget_error()); }
        if self.active.contains_key(&key) { return Err(GraphError { node: None, field: None, kind: GraphErrorKind::TreeShape }); }
        if let Some(value) = self.values.get(&key) { return Ok(Some(value.target.clone())); }
        if self.values.len().checked_add(self.active.len()).and_then(|n| n.checked_add(self.actors.len())).is_none_or(|n| n >= self.meter.limits().handles) { return Err(budget_error()); }
        reserve_map(self.meter, &mut self.active)?; self.active.insert(key, ()); Ok(None)
    }
    fn finish(&mut self, key: (u8, usize), source: CopyValue, target: CopyValue) -> Result<(), GraphError> {
        reserve_map(self.meter, &mut self.values)?;
        self.values.insert(key, CopyEntry { _source: source, target }); self.active.remove(&key); Ok(())
    }
    fn slot(&mut self, value: &SharedSlot, depth: usize) -> Result<SharedSlot, GraphError> {
        self.meter.visit(1)?;
        match value {
            SharedSlot::Null => Ok(SharedSlot::Null),
            SharedSlot::Double(value) => Ok(SharedSlot::Double(value.clone())),
            SharedSlot::Actor(value) => self.actors.get(&(Rc::as_ptr(value) as usize)).map(|(_, target)| SharedSlot::Actor(Rc::clone(target))).ok_or_else(reference_error),
            SharedSlot::Boundable(value) => Ok(SharedSlot::Boundable(self.boundable(value, depth)?)),
            SharedSlot::Envelope(value) => Ok(SharedSlot::Envelope(self.envelope(value, depth)?)),
        }
    }
    pub(crate) fn list(&mut self, source: &SharedListRef) -> Result<SharedListRef, GraphError> {
        if let Some(error) = self.failed { return Err(error); }
        let result = self.list_at(source, 0); if let Err(error) = result { self.failed = Some(error); } result
    }
    fn list_at(&mut self, source: &SharedListRef, depth: usize) -> Result<SharedListRef, GraphError> {
        let key = (if matches!(source, SharedListRef::Array(_)) { 0 } else { 1 }, source.identity());
        if let Some(value) = self.begin(key, depth)? { let CopyValue::List(value) = value else { unreachable!() }; return Ok(value); }
        let target = match source {
            SharedListRef::Array(_) => {
                let values = source.slots(); self.meter.array(values.len())?;
                self.meter.allocate(SharedListRef::backing_allocation())?;
                let mut copied = Vec::new(); reserve(self.meter, &mut copied, values.len())?;
                for value in values.iter() { copied.push(self.slot(value, depth + 1)?); }
                SharedListRef::from_slots(copied)
            }
            SharedListRef::Synchronized(_) => {
                let backing = self.list_at(&source.backing_ref(), depth + 1)?;
                self.meter.allocate(SharedListRef::wrapper_allocation())?;
                SharedListRef::synchronized(&backing)?
            }
        };
        self.finish(key, CopyValue::List(source.clone()), CopyValue::List(target.clone()))?; Ok(target)
    }
    fn envelope(&mut self, source: &EnvelopeRef, depth: usize) -> Result<EnvelopeRef, GraphError> {
        let key = (2, source.identity());
        if let Some(value) = self.begin(key, depth)? { let CopyValue::Envelope(value) = value else { unreachable!() }; return Ok(value); }
        self.meter.allocate(EnvelopeRef::allocation())?;
        let target = EnvelopeRef::new(source.get());
        self.finish(key, CopyValue::Envelope(source.clone()), CopyValue::Envelope(target.clone()))?; Ok(target)
    }
    fn boundable(&mut self, source: &BoundableRef, depth: usize) -> Result<BoundableRef, GraphError> {
        let key = (3, source.identity());
        if let Some(value) = self.begin(key, depth)? { let CopyValue::Boundable(value) = value else { unreachable!() }; return Ok(value); }
        let value = match &*source.value() {
            BoundableValue::Item { bounds, item } => BoundableValue::Item { bounds: bounds.as_ref().map(|value| self.envelope(value, depth + 1)).transpose()?, item: self.slot(item, depth + 1)? },
            BoundableValue::Node { level, bounds, children } => BoundableValue::Node { level: *level, bounds: bounds.as_ref().map(|value| self.envelope(value, depth + 1)).transpose()?, children: children.as_ref().map(|value| self.list_at(value, depth + 1)).transpose()? },
        };
        self.meter.allocate(BoundableRef::allocation())?;
        let target = BoundableRef::new(value);
        self.finish(key, CopyValue::Boundable(source.clone()), CopyValue::Boundable(target.clone()))?; Ok(target)
    }
    pub(crate) fn tree(&mut self, source: &TreeRef) -> Result<TreeRef, GraphError> {
        if let Some(error) = self.failed { return Err(error); }
        let result = self.tree_inner(source); if let Err(error) = result { self.failed = Some(error); } result
    }
    fn tree_inner(&mut self, source: &TreeRef) -> Result<TreeRef, GraphError> {
        let key = (4, source.identity());
        if let Some(value) = self.begin(key, 0)? { let CopyValue::Tree(value) = value else { unreachable!() }; return Ok(value); }
        let value = source.value();
        let copied = TreeValue { built: value.built, node_capacity: value.node_capacity, pending: value.pending.as_ref().map(|value| self.list_at(value, 1)).transpose()?, root: value.root.as_ref().map(|value| self.boundable(value, 1)).transpose()? };
        self.meter.allocate(TreeRef::allocation())?;
        let target = TreeRef::new(copied);
        self.finish(key, CopyValue::Tree(source.clone()), CopyValue::Tree(target.clone()))?; Ok(target)
    }
    pub(crate) fn roots(&mut self, source: &OwnerRoots, worlds: &[WorldHandle]) -> Result<OwnerRoots, GraphError> {
        if let Some(error) = self.failed { return Err(error); }
        let result = self.roots_inner(source, worlds); if let Err(error) = result { self.failed = Some(error); } result
    }
    fn roots_inner(&mut self, source: &OwnerRoots, worlds: &[WorldHandle]) -> Result<OwnerRoots, GraphError> {
        if source.lists.len().checked_add(source.trees.len()).is_none_or(|n| n > self.meter.limits().records) { return Err(budget_error()); }
        let mut lists = Vec::new(); let mut trees = Vec::new();
        reserve(self.meter, &mut lists, source.lists.len())?; reserve(self.meter, &mut trees, source.trees.len())?;
        for list in &source.lists { lists.push(list.as_ref().map(|value| self.list(value)).transpose()?); }
        for tree in &source.trees { trees.push(tree.as_ref().map(|value| self.tree(value)).transpose()?); }
        OwnerRoots::new(lists, trees, worlds, self.meter)
    }
}

#[cfg(test)]
#[path = "shared_world_owners_tests.rs"]
mod tests;

/// Discover actual Actor targets without applying a consumer's list-role cast
pub(crate) fn referenced_actors(lists: &[SharedListRef], trees: &[TreeRef]) -> Vec<OrganismRef> {
    referenced_actors_in(lists, trees, &RuntimeCopyMeter::new()).expect("Runtime owner traversal cannot exceed codec quotas")
}
pub(crate) fn referenced_actors_in(lists: &[SharedListRef], trees: &[TreeRef], meter: &impl OwnerMeter) -> Result<Vec<OrganismRef>, GraphError> {
    fn push(pending: &mut Vec<Walk>, value: Walk, meter: &impl OwnerMeter) -> Result<(), GraphError> {
        reserve(meter, pending, 1)?; pending.push(value); Ok(())
    }
    let mut pending = Vec::new();
    for tree in trees.iter().rev() { push(&mut pending, Walk::Tree(tree.clone()), meter)?; }
    for list in lists.iter().rev() { push(&mut pending, Walk::List(list.clone()), meter)?; }
    let mut seen = HashMap::new();
    let mut actors = Vec::new();
    while let Some(value) = pending.pop() {
        meter.visit(1)?;
        if seen.contains_key(&value.key()) { continue; }
        reserve_map(meter, &mut seen)?; seen.insert(value.key(), ());
        match value {
            Walk::Actor(actor) => { reserve(meter, &mut actors, 1)?; actors.push(actor); },
            Walk::List(list) => for slot in list.slots().iter().rev() {
                match slot { SharedSlot::Actor(actor) => push(&mut pending, Walk::Actor(Rc::clone(actor)), meter)?, SharedSlot::Boundable(value) => push(&mut pending, Walk::Boundable(value.clone()), meter)?, _ => {} }
            },
            Walk::Tree(tree) => {
                let tree = tree.value();
                if let Some(root) = &tree.root { push(&mut pending, Walk::Boundable(root.clone()), meter)?; }
                if let Some(pending_list) = &tree.pending { push(&mut pending, Walk::List(pending_list.clone()), meter)?; }
            },
            Walk::Boundable(value) => match &*value.value() {
                BoundableValue::Item { item, .. } => match item { SharedSlot::Actor(actor) => push(&mut pending, Walk::Actor(Rc::clone(actor)), meter)?, SharedSlot::Boundable(value) => push(&mut pending, Walk::Boundable(value.clone()), meter)?, _ => {} },
                BoundableValue::Node { children: Some(children), .. } => push(&mut pending, Walk::List(children.clone()), meter)?,
                _ => {},
            },
        }
    }
    Ok(actors)
}

pub(crate) struct OwnerEquality<F> { actors: F, seen: std::collections::HashSet<(u8, usize, usize)> }
impl<F: FnMut(&OrganismRef, &OrganismRef) -> bool> OwnerEquality<F> {
    pub(crate) fn new(actors: F) -> Self { Self { actors, seen: std::collections::HashSet::new() } }
    fn envelope(&self, a: Option<&EnvelopeRef>, b: Option<&EnvelopeRef>) -> bool {
        match (a,b) {
            (None,None) => true,
            (Some(a),Some(b)) => { let a=a.get(); let b=b.get(); [a.min_x.to_bits(),a.max_x.to_bits(),a.min_y.to_bits(),a.max_y.to_bits()] == [b.min_x.to_bits(),b.max_x.to_bits(),b.min_y.to_bits(),b.max_y.to_bits()] },
            _ => false,
        }
    }
    fn slot(&mut self, a: &SharedSlot, b: &SharedSlot) -> bool {
        match (a,b) {
            (SharedSlot::Null,SharedSlot::Null) => true,
            (SharedSlot::Double(a),SharedSlot::Double(b)) => a.bits()==b.bits(),
            (SharedSlot::Actor(a),SharedSlot::Actor(b)) => (self.actors)(a,b),
            (SharedSlot::Boundable(a),SharedSlot::Boundable(b)) => self.boundable(Some(a),Some(b)),
            (SharedSlot::Envelope(a),SharedSlot::Envelope(b)) => self.envelope(Some(a),Some(b)),
            _ => false,
        }
    }
    pub(crate) fn list(&mut self, a: Option<&SharedListRef>, b: Option<&SharedListRef>) -> bool {
        let (a,b)=match(a,b) { (None,None)=>return true,(Some(a),Some(b))=>(a,b),_=>return false };
        if a.wire_class_name()!=b.wire_class_name() { return false; }
        if !self.seen.insert((b'L',a.identity(),b.identity())) { return true; }
        let a=a.slots();let b=b.slots();a.len()==b.len()&&a.iter().zip(b.iter()).all(|(a,b)|self.slot(a,b))
    }
    pub(crate) fn boundable(&mut self, a: Option<&BoundableRef>, b: Option<&BoundableRef>) -> bool {
        let (a,b)=match(a,b) { (None,None)=>return true,(Some(a),Some(b))=>(a,b),_=>return false };
        if !self.seen.insert((b'B',a.identity(),b.identity())) { return true; }
        match(&*a.value(),&*b.value()) {
            (BoundableValue::Item { bounds:a,item:x },BoundableValue::Item { bounds:b,item:y })=>self.envelope(a.as_ref(),b.as_ref())&&self.slot(x,y),
            (BoundableValue::Node { level:a,bounds:x,children:c },BoundableValue::Node { level:b,bounds:y,children:d })=>a==b&&self.envelope(x.as_ref(),y.as_ref())&&self.list(c.as_ref(),d.as_ref()),
            _=>false,
        }
    }
    pub(crate) fn tree(&mut self, a: &TreeRef, b: &TreeRef) -> bool {
        let a=a.value();let b=b.value();
        a.built==b.built&&a.node_capacity==b.node_capacity&&self.list(a.pending.as_ref(),b.pending.as_ref())&&self.boundable(a.root.as_ref(),b.root.as_ref())
    }
}
