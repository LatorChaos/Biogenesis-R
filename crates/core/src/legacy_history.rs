// SPDX-License-Identifier: GPL-2.0-or-later

//! Detached bounded ArrayList and Double staging, separate from spatial list policy

use crate::history::{HistoryListRef, JavaDoubleRef};
use crate::shared_world_owners::{SharedListRef, SharedSlot};
use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::object_stream::{
    Arena, ClassData, ClassDescriptor, Document, FieldValue, JavaObject, Limits, Node, NodeId,
    Record, Value, class_by_name, class_schema,
};
use crate::statistics::WorldStatistics;
use std::mem::size_of;

pub(crate) const HISTORY_FIELDS: [&str; 6] = [
    "populationList",
    "deathList",
    "birthList",
    "oxygenList",
    "carbonDioxideList",
    "methaneList",
];

fn error(kind: GraphErrorKind) -> GraphError {
    GraphError {
        node: None,
        field: None,
        kind,
    }
}
fn product(a: usize, b: usize) -> Result<usize, GraphError> {
    a.checked_mul(b).ok_or(error(GraphErrorKind::Budget))
}

struct Budget {
    maximum: usize,
    used: usize,
}
impl Budget {
    fn charge(&mut self, bytes: usize) -> Result<(), GraphError> {
        let next = self
            .used
            .checked_add(bytes)
            .ok_or(error(GraphErrorKind::Budget))?;
        if next > self.maximum {
            return Err(error(GraphErrorKind::Budget));
        }
        self.used = next;
        Ok(())
    }
    fn reserve<T>(&mut self, values: &mut Vec<T>, count: usize) -> Result<(), GraphError> {
        self.charge(product(count, size_of::<T>())?)?;
        values
            .try_reserve_exact(count)
            .map_err(|_| error(GraphErrorKind::Allocation))
    }
}

pub(crate) trait HistorySource {
    fn uses_graph(&self, graph: &LegacyGraph<'_>) -> bool;
    fn list(&mut self, value: Value) -> Result<Option<HistoryListRef>, GraphError>;
}
impl HistorySource for HistoryStager<'_, '_> {
    fn uses_graph(&self, graph: &LegacyGraph<'_>) -> bool { HistoryStager::uses_graph(self, graph) }
    fn list(&mut self, value: Value) -> Result<Option<HistoryListRef>, GraphError> { HistoryStager::list(self, value) }
}
impl HistorySource for crate::legacy_shared_world_owners::SharedOwnerStager<'_, '_> {
    fn uses_graph(&self, graph: &LegacyGraph<'_>) -> bool { self.uses_graph(graph) }
    fn list(&mut self, value: Value) -> Result<Option<HistoryListRef>, GraphError> {
        Ok(crate::legacy_shared_world_owners::SharedOwnerStager::list(self, value)?.map(HistoryListRef::from_shared))
    }
}

/// One table spans all six histories; no borrowed Arena survives publication
pub(crate) struct HistoryStager<'graph, 'document> {
    graph: &'graph LegacyGraph<'document>,
    lists: Vec<Option<HistoryListRef>>,
    boxes: Vec<Option<JavaDoubleRef>>,
    budget: Budget,
    elements: usize,
}
impl<'graph, 'document> HistoryStager<'graph, 'document> {
    pub(crate) fn new(
        graph: &'graph LegacyGraph<'document>,
        additional_bytes: usize,
    ) -> Result<Self, GraphError> {
        let mut budget = Budget {
            maximum: additional_bytes.min(graph.limits().allocated_bytes),
            used: 0,
        };
        let count = graph.arena().nodes.len();
        graph.charge(product(count, 2)?)?;
        let mut lists = Vec::new();
        budget.reserve(&mut lists, count)?;
        lists.resize_with(count, || None);
        let mut boxes = Vec::new();
        budget.reserve(&mut boxes, count)?;
        boxes.resize_with(count, || None);
        graph.allocate(budget.used)?;
        Ok(Self {
            graph,
            lists,
            boxes,
            budget,
            elements: 0,
        })
    }
    pub(crate) fn uses_graph(&self, graph: &LegacyGraph<'_>) -> bool {
        std::ptr::eq(self.graph, graph)
    }
    pub(crate) fn allocated_bytes(&self) -> usize {
        self.budget.used
    }
    fn boxed(&mut self, value: Value) -> Result<Option<JavaDoubleRef>, GraphError> {
        let Value::Handle(id) = value else {
            return Ok(None);
        };
        self.graph.node(id)?;
        if let Some(value) = &self.boxes[id.0 as usize] {
            return Ok(Some(value.clone()));
        }
        self.graph.expect_class(id, "java.lang.Double")?;
        if self
            .graph
            .slots(id)?
            .iter()
            .any(|slot| !slot.custom.is_empty())
        {
            return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
        }
        let bits = self.graph.double_bits(id, "java.lang.Double", "value")?;
        let bytes = size_of::<u64>() + 2 * size_of::<usize>();
        self.budget.charge(bytes)?;
        self.graph.allocate(bytes)?;
        let value = JavaDoubleRef::from_bits(bits);
        self.boxes[id.0 as usize] = Some(value.clone());
        Ok(Some(value))
    }
    pub(crate) fn list(&mut self, value: Value) -> Result<Option<HistoryListRef>, GraphError> {
        let Value::Handle(id) = value else {
            return Ok(None);
        };
        self.graph.node(id)?;
        if let Some(value) = &self.lists[id.0 as usize] {
            return Ok(Some(value.clone()));
        }
        self.graph.expect_class(id, "java.util.ArrayList")?;
        let length = self.graph.int(id, "java.util.ArrayList", "size")?;
        if length < 0 || length as usize > self.graph.limits().array_elements {
            return Err(GraphError::at(id, Some("size"), GraphErrorKind::Length));
        }
        let length = length as usize;
        let total = self
            .elements
            .checked_add(length)
            .ok_or(error(GraphErrorKind::Budget))?;
        if total > self.graph.limits().total_array_elements {
            return Err(error(GraphErrorKind::Budget));
        }
        let [slot] = self.graph.slots(id)? else {
            return Err(GraphError::at(id, None, GraphErrorKind::Slots));
        };
        let (mut used, mut position) = (0_usize, 0_usize);
        while used < 4 {
            let Some(Record::Block { bytes, .. }) = slot.custom.get(position) else {
                return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
            };
            self.graph.charge(1)?;
            if bytes.len() > 4 - used && length != 0 {
                return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
            }
            used += bytes.len().min(4 - used);
            position += 1;
        }
        // JDK21 consumes this compatibility integer without using its value as capacity
        let end = position
            .checked_add(length)
            .ok_or(error(GraphErrorKind::Budget))?;
        let records = slot.custom.get(position..end).ok_or(GraphError::at(
            id,
            None,
            GraphErrorKind::ContainerData,
        ))?;
        self.graph.count_array(length)?;
        let before = self.budget.used;
        let mut values = Vec::new();
        self.budget.reserve(&mut values, length)?;
        self.budget.charge(
            SharedListRef::backing_allocation(),
        )?;
        self.graph.allocate(self.budget.used - before)?;
        for record in records {
            let Record::Value(value) = record else {
                return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
            };
            values.push(self.boxed(*value)?.map_or(SharedSlot::Null, SharedSlot::Double));
        }
        // ObjectInputStream discards remaining custom data after ArrayList.readObject returns
        let value = HistoryListRef::from_shared(SharedListRef::from_slots(values));
        self.lists[id.0 as usize] = Some(value.clone());
        self.elements = total;
        Ok(Some(value))
    }
    pub(crate) fn statistics(
        &mut self,
        id: NodeId,
    ) -> Result<[Option<HistoryListRef>; 6], GraphError> {
        self.graph.expect_class(id, "biogenesis.WorldStatistics")?;
        let mut values = Vec::new();
        self.budget.reserve(&mut values, 6)?;
        self.graph
            .allocate(product(6, size_of::<Option<HistoryListRef>>())?)?;
        for name in HISTORY_FIELDS {
            values.push(
                self.list(
                    self.graph
                        .reference(id, "biogenesis.WorldStatistics", name)?,
                )?,
            );
        }
        values.try_into().map_err(|_| error(GraphErrorKind::Field))
    }
    /// Stage every reference before replacing any field on the existing statistics owner
    pub(crate) fn restore_histories(
        &mut self,
        target: &mut WorldStatistics,
        id: NodeId,
    ) -> Result<(), GraphError> {
        let [population, death, birth, oxygen, carbon, methane] = self.statistics(id)?;
        target.population_list = population;
        target.death_list = death;
        target.birth_list = birth;
        target.oxygen_list = oxygen;
        target.carbon_dioxide_list = carbon;
        target.methane_list = methane;
        Ok(())
    }
}

/// Build only the requested history roots; the writer owns Java numeric normalization
pub(crate) fn export_history_lists(
    roots: &[Option<&HistoryListRef>],
    limits: Limits,
    additional_bytes: usize,
) -> Result<Document, GraphError> {
    if roots.len() > limits.records {
        return Err(error(GraphErrorKind::Budget));
    }
    let mut budget = Budget {
        maximum: additional_bytes.min(limits.allocated_bytes),
        used: 0,
    };
    let mut lists = Vec::new();
    budget.reserve(&mut lists, roots.len())?;
    for value in roots.iter().flatten() {
        lists.push((*value).clone());
    }
    lists.sort_unstable_by_key(HistoryListRef::identity);
    lists.dedup_by_key(|value| value.identity());
    let mut elements = 0_usize;
    for list in &lists {
        let length = list.len();
        elements = elements
            .checked_add(length)
            .ok_or(error(GraphErrorKind::Budget))?;
        if length > limits.array_elements
            || length > i32::MAX as usize
            || elements > limits.total_array_elements
        {
            return Err(error(GraphErrorKind::Budget));
        }
    }
    let work = roots
        .len()
        .checked_add(elements)
        .and_then(|v| v.checked_add(lists.len()))
        .and_then(|v| v.checked_mul(usize::BITS as usize + 8))
        .ok_or(error(GraphErrorKind::Budget))?;
    let custom_records = elements
        .checked_add(lists.len())
        .and_then(|v| v.checked_add(roots.len()))
        .ok_or(error(GraphErrorKind::Budget))?;
    if work > limits.work || custom_records > limits.records {
        return Err(error(GraphErrorKind::Budget));
    }
    let mut boxes = Vec::new();
    budget.reserve(&mut boxes, elements)?;
    for list in &lists {
        if list.shared_ref().wire_class_name() != "java.util.ArrayList" { return Err(error(GraphErrorKind::Class)); }
        for slot in list.shared_ref().slots().iter() {
            match slot {
                SharedSlot::Double(value) => boxes.push(value.clone()),
                SharedSlot::Null => {},
                _ => return Err(error(GraphErrorKind::Class)),
            }
        }
    }
    boxes.sort_unstable_by_key(JavaDoubleRef::identity);
    boxes.dedup_by_key(|value| value.identity());
    let descriptors = usize::from(!lists.is_empty()) + 2 * usize::from(!boxes.is_empty());
    let count = lists
        .len()
        .checked_add(boxes.len())
        .and_then(|v| v.checked_add(descriptors))
        .ok_or(error(GraphErrorKind::Budget))?;
    let depth = if !boxes.is_empty() {
        4
    } else if !lists.is_empty() {
        2
    } else {
        1
    };
    if count > limits.handles || count > u32::MAX as usize || limits.depth < depth {
        return Err(error(GraphErrorKind::Budget));
    }
    let mut arena = Arena::default();
    budget.reserve(&mut arena.nodes, count)?;
    let mut records = Vec::new();
    budget.reserve(&mut records, roots.len())?;
    let mut list_ids = Vec::new();
    budget.reserve(&mut list_ids, lists.len())?;
    let mut box_ids = Vec::new();
    budget.reserve(&mut box_ids, boxes.len())?;
    let mut number = None;
    let mut double = None;
    let mut array_list = None;
    for (name, enabled) in [
        ("java.lang.Number", !boxes.is_empty()),
        ("java.lang.Double", !boxes.is_empty()),
        ("java.util.ArrayList", !lists.is_empty()),
    ] {
        if !enabled {
            continue;
        }
        let id = NodeId(arena.nodes.len() as u32);
        let class = class_by_name(name).ok_or(error(GraphErrorKind::Class))?;
        let fields = class_schema(class).fields.len();
        let mut signatures = Vec::new();
        budget.reserve(&mut signatures, fields)?;
        signatures.resize(fields, None);
        arena.nodes.push(Node::Descriptor(ClassDescriptor {
            class,
            signatures,
            annotations: Vec::new(),
            superclass: if name == "java.lang.Double" {
                number
            } else {
                None
            },
        }));
        match name {
            "java.lang.Number" => number = Some(id),
            "java.lang.Double" => double = Some(id),
            _ => array_list = Some(id),
        }
    }
    for value in &boxes {
        let mut slots = Vec::new();
        budget.reserve(&mut slots, 2)?;
        let mut fields = Vec::new();
        budget.reserve(&mut fields, 1)?;
        fields.push(FieldValue::DoubleBits(value.bits()));
        slots.push(ClassData {
            descriptor: number.expect("Number descriptor"),
            values: Vec::new(),
            custom: Vec::new(),
        });
        slots.push(ClassData {
            descriptor: double.expect("Double descriptor"),
            values: fields,
            custom: Vec::new(),
        });
        let id = NodeId(arena.nodes.len() as u32);
        arena.nodes.push(Node::Object(JavaObject {
            descriptor: double.unwrap(),
            slots,
        }));
        box_ids.push(id);
    }
    for list in &lists {
        let values = list.shared_ref().slots();
        let mut fields = Vec::new();
        budget.reserve(&mut fields, 1)?;
        fields.push(FieldValue::Int(values.len() as i32));
        let mut custom = Vec::new();
        budget.reserve(
            &mut custom,
            values
                .len()
                .checked_add(1)
                .ok_or(error(GraphErrorKind::Budget))?,
        )?;
        let mut prefix = Vec::new();
        budget.reserve(&mut prefix, 4)?;
        prefix.extend_from_slice(&(values.len() as i32).to_be_bytes());
        custom.push(Record::Block {
            bytes: prefix,
            long: false,
        });
        for value in values.iter() {
            let value = match value {
                SharedSlot::Null => Value::Null,
                SharedSlot::Double(value) => Value::Handle(box_ids[boxes.binary_search_by_key(&value.identity(), JavaDoubleRef::identity).expect("Collected Double owner")]),
                _ => return Err(error(GraphErrorKind::Class)),
            };
            custom.push(Record::Value(value));
        }
        let mut slots = Vec::new();
        budget.reserve(&mut slots, 1)?;
        slots.push(ClassData {
            descriptor: array_list.expect("ArrayList descriptor"),
            values: fields,
            custom,
        });
        let id = NodeId(arena.nodes.len() as u32);
        arena.nodes.push(Node::Object(JavaObject {
            descriptor: array_list.unwrap(),
            slots,
        }));
        list_ids.push(id);
    }
    for root in roots {
        records.push(Record::Value(root.map_or(Value::Null, |value| {
            Value::Handle(
                list_ids[lists
                    .binary_search_by_key(&value.identity(), HistoryListRef::identity)
                    .expect("Collected history owner")],
            )
        })));
    }
    Ok(Document {
        arena,
        records,
        consumed: 0,
    })
}
