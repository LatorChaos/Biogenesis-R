// SPDX-License-Identifier: GPL-2.0-or-later

//! Bounded detached primitive-array staging and batched export

use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::object_stream::{
    Arena, ArrayData, ClassDescriptor, Document, JavaArray, Limits, Node, NodeId, Record, Value,
    class_by_name,
};
use crate::{DoubleArrayRef, IntArrayRef, OrganismArrays};
use std::mem::size_of;

fn error(kind: GraphErrorKind) -> GraphError {
    GraphError {
        node: None,
        field: None,
        kind,
    }
}

struct ArrayBudget {
    maximum: usize,
    used: usize,
}
impl ArrayBudget {
    fn charge(&mut self, bytes: usize) -> Result<(), GraphError> {
        let used = self
            .used
            .checked_add(bytes)
            .ok_or(error(GraphErrorKind::Budget))?;
        if used > self.maximum {
            return Err(error(GraphErrorKind::Budget));
        }
        self.used = used;
        Ok(())
    }
    fn reserve<T>(&mut self, vector: &mut Vec<T>, count: usize) -> Result<(), GraphError> {
        self.charge(
            count
                .checked_mul(size_of::<T>())
                .ok_or(error(GraphErrorKind::Budget))?,
        )?;
        vector
            .try_reserve_exact(count)
            .map_err(|_| error(GraphErrorKind::Allocation))
    }
}

/// No borrowed arena or cached Segment survives this detached boundary
pub(crate) struct StagedPrimitiveState {
    pub segment_count: i32,
    pub arrays: OrganismArrays,
}

pub(crate) struct PrimitiveStager<'graph, 'document> {
    graph: &'graph LegacyGraph<'document>,
    ints: Vec<Option<IntArrayRef>>,
    doubles: Vec<Option<DoubleArrayRef>>,
    budget: ArrayBudget,
    elements: usize,
}
impl<'graph, 'document> PrimitiveStager<'graph, 'document> {
    pub(crate) fn new(
        graph: &'graph LegacyGraph<'document>,
        additional_bytes: usize,
    ) -> Result<Self, GraphError> {
        let mut budget = ArrayBudget {
            maximum: additional_bytes,
            used: 0,
        };
        let count = graph.arena().nodes.len();
        graph.allocate(count.checked_mul(
            size_of::<Option<IntArrayRef>>() + size_of::<Option<DoubleArrayRef>>()
        ).ok_or(error(GraphErrorKind::Budget))?)?;
        graph.charge(count.checked_mul(2).ok_or(error(GraphErrorKind::Budget))?)?;
        let mut ints = Vec::new();
        budget.reserve(&mut ints, count)?;
        ints.resize_with(count, || None);
        let mut doubles = Vec::new();
        budget.reserve(&mut doubles, count)?;
        doubles.resize_with(count, || None);
        Ok(Self {
            graph,
            ints,
            doubles,
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
    fn array(&self, id: NodeId, class: &'static str) -> Result<&'document JavaArray, GraphError> {
        self.graph.expect_class(id, class)?;
        let Node::Array(array) = self.graph.node(id)? else {
            return Err(GraphError::at(id, None, GraphErrorKind::Type));
        };
        let Node::Descriptor(descriptor) = self.graph.node(array.descriptor)? else {
            return Err(GraphError::at(id, None, GraphErrorKind::Descriptor));
        };
        if !descriptor.signatures.is_empty() || descriptor.superclass.is_some() {
            return Err(GraphError::at(id, None, GraphErrorKind::Descriptor));
        }
        Ok(array)
    }
    fn count(&mut self, id: NodeId, count: usize) -> Result<(), GraphError> {
        let total = self.elements.checked_add(count).ok_or(GraphError::at(
            id,
            None,
            GraphErrorKind::Budget,
        ))?;
        if count > self.graph.limits().array_elements
            || count > i32::MAX as usize
            || total > self.graph.limits().total_array_elements
        {
            return Err(GraphError::at(id, None, GraphErrorKind::Budget));
        }
        self.graph.count_array(count)?;
        self.elements = total;
        Ok(())
    }
    pub(crate) fn ints(&mut self, value: Value) -> Result<Option<IntArrayRef>, GraphError> {
        let Value::Handle(id) = value else {
            return Ok(None);
        };
        self.graph.node(id)?;
        if let Some(owner) = &self.ints[id.0 as usize] {
            return Ok(Some(owner.clone()));
        }
        let ArrayData::Ints(values) = &self.array(id, "[I")?.data else {
            return Err(GraphError::at(id, None, GraphErrorKind::Type));
        };
        self.count(id, values.len())?;
        self.graph.allocate(values.len().checked_mul(size_of::<i32>())
            .and_then(|n| n.checked_add(size_of::<std::cell::RefCell<Vec<i32>>>() + 2 * size_of::<usize>()))
            .ok_or(error(GraphErrorKind::Budget))?)?;
        self.budget
            .charge(size_of::<std::cell::RefCell<Vec<i32>>>() + 2 * size_of::<usize>())?;
        let mut copied = Vec::new();
        self.budget.reserve(&mut copied, values.len())?;
        copied.extend_from_slice(values);
        let owner = IntArrayRef::from_values(copied);
        self.ints[id.0 as usize] = Some(owner.clone());
        Ok(Some(owner))
    }
    pub(crate) fn doubles(&mut self, value: Value) -> Result<Option<DoubleArrayRef>, GraphError> {
        let Value::Handle(id) = value else {
            return Ok(None);
        };
        self.graph.node(id)?;
        if let Some(owner) = &self.doubles[id.0 as usize] {
            return Ok(Some(owner.clone()));
        }
        let ArrayData::Doubles(values) = &self.array(id, "[D")?.data else {
            return Err(GraphError::at(id, None, GraphErrorKind::Type));
        };
        self.count(id, values.len())?;
        self.graph.allocate(values.len().checked_mul(size_of::<u64>())
            .and_then(|n| n.checked_add(size_of::<std::cell::RefCell<Vec<u64>>>() + 2 * size_of::<usize>()))
            .ok_or(error(GraphErrorKind::Budget))?)?;
        self.budget
            .charge(size_of::<std::cell::RefCell<Vec<u64>>>() + 2 * size_of::<usize>())?;
        let mut copied = Vec::new();
        self.budget.reserve(&mut copied, values.len())?;
        copied.extend_from_slice(values);
        let owner = DoubleArrayRef::from_bits(copied);
        self.doubles[id.0 as usize] = Some(owner.clone());
        Ok(Some(owner))
    }
    pub(crate) fn organism(&mut self, id: NodeId) -> Result<StagedPrimitiveState, GraphError> {
        const ORGANISM: &str = "biogenesis.Organism";
        self.graph.expect_class(id, ORGANISM)?;
        let segment_count = self.graph.int(id, ORGANISM, "_segments")?;
        let mut arrays = OrganismArrays::default();
        for field in crate::organism_arrays::INT_FIELDS {
            let value = self.graph.reference(id, ORGANISM, field.name())?;
            *arrays.int_mut(field) = self.ints(value)?;
        }
        for field in crate::organism_arrays::DOUBLE_FIELDS {
            let value = self.graph.reference(id, ORGANISM, field.name())?;
            *arrays.double_mut(field) = self.doubles(value)?;
        }
        Ok(StagedPrimitiveState {
            segment_count,
            arrays,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) enum PrimitiveRoot<'a> {
    Int(Option<&'a IntArrayRef>),
    Double(Option<&'a DoubleArrayRef>),
}

/// All roots share one identity context; failure publishes no partial Document
pub(crate) fn export_primitive_arrays(
    roots: &[PrimitiveRoot<'_>],
    limits: Limits,
    additional_bytes: usize,
) -> Result<Document, GraphError> {
    if roots.len() > limits.records || limits.depth < 2 {
        return Err(error(GraphErrorKind::Budget));
    }
    let work = roots
        .len()
        .checked_mul(usize::BITS as usize + 3)
        .ok_or(error(GraphErrorKind::Budget))?;
    if work > limits.work {
        return Err(error(GraphErrorKind::Budget));
    }
    let mut budget = ArrayBudget {
        maximum: additional_bytes,
        used: 0,
    };
    let mut ints = Vec::new();
    let mut doubles = Vec::new();
    budget.reserve(&mut ints, roots.len())?;
    budget.reserve(&mut doubles, roots.len())?;
    for root in roots {
        match root {
            PrimitiveRoot::Int(Some(value)) => ints.push((*value).clone()),
            PrimitiveRoot::Double(Some(value)) => doubles.push((*value).clone()),
            _ => {}
        }
    }
    ints.sort_unstable_by_key(IntArrayRef::identity);
    ints.dedup_by_key(|v| v.identity());
    doubles.sort_unstable_by_key(DoubleArrayRef::identity);
    doubles.dedup_by_key(|v| v.identity());
    let node_count = ints
        .len()
        .checked_add(doubles.len())
        .and_then(|v| {
            v.checked_add(usize::from(!ints.is_empty()) + usize::from(!doubles.is_empty()))
        })
        .ok_or(error(GraphErrorKind::Budget))?;
    if node_count > limits.handles || node_count > u32::MAX as usize {
        return Err(error(GraphErrorKind::Budget));
    }
    let mut arena = Arena::default();
    budget.reserve(&mut arena.nodes, node_count)?;
    let mut records = Vec::new();
    budget.reserve(&mut records, roots.len())?;
    let mut int_ids = Vec::new();
    budget.reserve(&mut int_ids, ints.len())?;
    let mut double_ids = Vec::new();
    budget.reserve(&mut double_ids, doubles.len())?;
    let mut elements = 0_usize;
    for (class, int_kind) in [("[I", true), ("[D", false)] {
        let count = if int_kind { ints.len() } else { doubles.len() };
        if count == 0 {
            continue;
        }
        let descriptor = NodeId(arena.nodes.len() as u32);
        arena.nodes.push(Node::Descriptor(ClassDescriptor {
            class: class_by_name(class).ok_or(error(GraphErrorKind::Class))?,
            signatures: Vec::new(),
            annotations: Vec::new(),
            superclass: None,
        }));
        for index in 0..count {
            let length = if int_kind {
                ints[index].len()
            } else {
                doubles[index].len()
            };
            elements = elements
                .checked_add(length)
                .ok_or(error(GraphErrorKind::Budget))?;
            if length > limits.array_elements
                || length > i32::MAX as usize
                || elements > limits.total_array_elements
                || work.checked_add(elements).is_none_or(|v| v > limits.work)
            {
                return Err(error(GraphErrorKind::Budget));
            }
            let id = NodeId(arena.nodes.len() as u32);
            let data = if int_kind {
                let mut values = Vec::new();
                budget.reserve(&mut values, length)?;
                values.extend_from_slice(&ints[index].borrow_values());
                int_ids.push(id);
                ArrayData::Ints(values)
            } else {
                let mut values = Vec::new();
                budget.reserve(&mut values, length)?;
                values.extend_from_slice(&doubles[index].borrow_bits());
                double_ids.push(id);
                ArrayData::Doubles(values)
            };
            arena
                .nodes
                .push(Node::Array(JavaArray { descriptor, data }));
        }
    }
    for root in roots {
        let value = match *root {
            PrimitiveRoot::Int(Some(value)) => Value::Handle(
                int_ids[ints
                    .binary_search_by_key(&value.identity(), IntArrayRef::identity)
                    .expect("Collected int owner")],
            ),
            PrimitiveRoot::Double(Some(value)) => Value::Handle(
                double_ids[doubles
                    .binary_search_by_key(&value.identity(), DoubleArrayRef::identity)
                    .expect("Collected double owner")],
            ),
            _ => Value::Null,
        };
        records.push(Record::Value(value));
    }
    Ok(Document {
        arena,
        records,
        consumed: 0,
    })
}
