// SPDX-License-Identifier: GPL-2.0-or-later

use crate::object_stream::{
    Arena, ClassData, Document, FieldValue, Limits, Node, NodeId, Record, Value, class_schema,
};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphErrorKind {
    Root,
    Reference,
    Descriptor,
    Class,
    Slots,
    Field,
    Type,
    Length,
    ContainerData,
    Allocation,
    Budget,
    UnsupportedUtf16,
    UnsupportedColor,
    AliasedTreeList,
    TreeShape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GraphError {
    pub node: Option<NodeId>,
    pub field: Option<&'static str>,
    pub kind: GraphErrorKind,
}

impl GraphError {
    pub(crate) fn at(node: NodeId, field: Option<&'static str>, kind: GraphErrorKind) -> Self {
        Self {
            node: Some(node),
            field,
            kind,
        }
    }
}

/// Typed access to parsed fields; this borrows the existing bounded data arena
pub(crate) struct LegacyGraph<'a> {
    document: &'a Document,
    limits: Limits,
    work: Cell<usize>,
    allocated: Cell<usize>,
    array_elements: Cell<usize>,
}

impl<'a> LegacyGraph<'a> {
    pub(crate) fn new(document: &'a Document, limits: Limits) -> Result<Self, GraphError> {
        if document.arena.nodes.len() > limits.handles
            || document.records.len() > limits.records
            || document.consumed > limits.input_bytes
        {
            return Err(GraphError {
                node: None,
                field: None,
                kind: GraphErrorKind::Budget,
            });
        }
        Ok(Self {
            document,
            limits,
            work: Cell::new(0),
            allocated: Cell::new(0),
            array_elements: Cell::new(0),
        })
    }

    pub(crate) fn arena(&self) -> &'a Arena {
        &self.document.arena
    }

    pub(crate) fn limits(&self) -> Limits {
        self.limits
    }

    pub(crate) fn allocated_bytes(&self) -> usize { self.allocated.get() }
    pub(crate) fn remaining_allocation(&self) -> usize { self.limits.allocated_bytes - self.allocated.get() }
    pub(crate) fn remaining_work(&self) -> usize { self.limits.work - self.work.get() }

    pub(crate) fn count_array(&self, count: usize) -> Result<(), GraphError> {
        let total = self.array_elements.get().checked_add(count)
            .ok_or(GraphError { node: None, field: None, kind: GraphErrorKind::Budget })?;
        if count > self.limits.array_elements || total > self.limits.total_array_elements {
            return Err(GraphError { node: None, field: None, kind: GraphErrorKind::Budget });
        }
        self.charge(count)?;
        self.array_elements.set(total);
        Ok(())
    }

    pub(crate) fn allocate(&self, bytes: usize) -> Result<(), GraphError> {
        let next = self.allocated.get().checked_add(bytes).ok_or(GraphError {
            node: None,
            field: None,
            kind: GraphErrorKind::Budget,
        })?;
        if next > self.limits.allocated_bytes {
            return Err(GraphError {
                node: None,
                field: None,
                kind: GraphErrorKind::Budget,
            });
        }
        self.allocated.set(next);
        Ok(())
    }

    pub(crate) fn charge(&self, count: usize) -> Result<(), GraphError> {
        let next = self.work.get().checked_add(count).ok_or(GraphError {
            node: None,
            field: None,
            kind: GraphErrorKind::Budget,
        })?;
        if next > self.limits.work {
            return Err(GraphError {
                node: None,
                field: None,
                kind: GraphErrorKind::Budget,
            });
        }
        self.work.set(next);
        Ok(())
    }

    pub(crate) fn node(&self, id: NodeId) -> Result<&'a Node, GraphError> {
        self.charge(1)?;
        self.document
            .arena
            .nodes
            .get(id.0 as usize)
            .ok_or(GraphError::at(id, None, GraphErrorKind::Reference))
    }

    pub(crate) fn class_name(&self, id: NodeId) -> Result<&'static str, GraphError> {
        let descriptor = match self.node(id)? {
            Node::Object(value) => value.descriptor,
            Node::Array(value) => value.descriptor,
            _ => return Err(GraphError::at(id, None, GraphErrorKind::Class)),
        };
        match self.node(descriptor)? {
            Node::Descriptor(value) => Ok(class_schema(value.class).name),
            _ => Err(GraphError::at(id, None, GraphErrorKind::Descriptor)),
        }
    }

    pub(crate) fn expect_class(&self, id: NodeId, name: &'static str) -> Result<(), GraphError> {
        if self.class_name(id)? != name {
            return Err(GraphError::at(id, None, GraphErrorKind::Class));
        }
        Ok(())
    }

    pub(crate) fn world_root(&self) -> Result<NodeId, GraphError> {
        let [Record::Value(Value::Handle(root))] = self.document.records.as_slice() else {
            return Err(GraphError {
                node: None,
                field: None,
                kind: GraphErrorKind::Root,
            });
        };
        self.expect_class(*root, "biogenesis.World")?;
        Ok(*root)
    }

    pub(crate) fn validate_descriptor(&self, id: NodeId) -> Result<(), GraphError> {
        let Node::Descriptor(descriptor) = self.node(id)? else {
            return Err(GraphError::at(id, None, GraphErrorKind::Descriptor));
        };
        let schema = class_schema(descriptor.class);
        if descriptor.signatures.len() != schema.fields.len() {
            return Err(GraphError::at(id, None, GraphErrorKind::Descriptor));
        }
        for (field, actual) in schema.fields.iter().zip(&descriptor.signatures) {
            match (field.signature, actual) {
                (None, None) => {}
                (Some(expected), Some(id)) => {
                    if !self.string_units(*id)?.iter().copied().eq(expected.encode_utf16()) {
                        return Err(GraphError::at(*id, Some(field.name), GraphErrorKind::Descriptor));
                    }
                }
                _ => return Err(GraphError::at(id, Some(field.name), GraphErrorKind::Descriptor)),
            }
        }
        let parent = match descriptor.superclass {
            Some(parent) => {
                let Node::Descriptor(value) = self.node(parent)? else {
                    return Err(GraphError::at(id, None, GraphErrorKind::Descriptor));
                };
                Some(class_schema(value.class).name)
            }
            None => None,
        };
        if parent != schema.superclass {
            return Err(GraphError::at(id, None, GraphErrorKind::Descriptor));
        }
        Ok(())
    }


    pub(crate) fn slots(&self, id: NodeId) -> Result<&'a [ClassData], GraphError> {
        let Node::Object(object) = self.node(id)? else {
            return Err(GraphError::at(id, None, GraphErrorKind::Class));
        };
        let mut descriptor = Some(object.descriptor);
        let mut position = object.slots.len();
        let mut traversed = 0;
        while let Some(current) = descriptor {
            traversed += 1;
            if traversed > self.limits.depth || position == 0 {
                return Err(GraphError::at(id, None, GraphErrorKind::Slots));
            }
            position -= 1;
            let Node::Descriptor(value) = self.node(current)? else {
                return Err(GraphError::at(id, None, GraphErrorKind::Descriptor));
            };
            let parent_name = match value.superclass {
                Some(parent) => match self.node(parent)? {
                    Node::Descriptor(parent) => Some(class_schema(parent.class).name),
                    _ => return Err(GraphError::at(id, None, GraphErrorKind::Descriptor)),
                },
                None => None,
            };
            if parent_name != class_schema(value.class).superclass {
                return Err(GraphError::at(id, None, GraphErrorKind::Slots));
            }
            if object.slots[position].descriptor != current
                || object.slots[position].values.len() != class_schema(value.class).fields.len()
            {
                return Err(GraphError::at(id, None, GraphErrorKind::Slots));
            }
            descriptor = value.superclass;
        }
        if position != 0 {
            return Err(GraphError::at(id, None, GraphErrorKind::Slots));
        }
        Ok(&object.slots)
    }

    pub(crate) fn field(
        &self,
        id: NodeId,
        owner: &'static str,
        name: &'static str,
    ) -> Result<FieldValue, GraphError> {
        for slot in self.slots(id)? {
            let Node::Descriptor(descriptor) = self.node(slot.descriptor)? else {
                unreachable!()
            };
            let schema = class_schema(descriptor.class);
            if schema.name != owner {
                continue;
            }
            let index = schema
                .fields
                .iter()
                .position(|field| field.name == name)
                .ok_or(GraphError::at(id, Some(name), GraphErrorKind::Field))?;
            return Ok(slot.values[index]);
        }
        Err(GraphError::at(id, Some(name), GraphErrorKind::Field))
    }

    pub(crate) fn reference(
        &self,
        id: NodeId,
        owner: &'static str,
        name: &'static str,
    ) -> Result<Value, GraphError> {
        match self.field(id, owner, name)? {
            FieldValue::Reference(value) => Ok(value),
            _ => Err(GraphError::at(id, Some(name), GraphErrorKind::Type)),
        }
    }

    pub(crate) fn required_reference(
        &self,
        id: NodeId,
        owner: &'static str,
        name: &'static str,
    ) -> Result<NodeId, GraphError> {
        match self.reference(id, owner, name)? {
            Value::Handle(value) => {
                self.node(value)?;
                Ok(value)
            }
            Value::Null => Err(GraphError::at(id, Some(name), GraphErrorKind::Reference)),
        }
    }

    pub(crate) fn int(
        &self,
        id: NodeId,
        owner: &'static str,
        name: &'static str,
    ) -> Result<i32, GraphError> {
        match self.field(id, owner, name)? {
            FieldValue::Int(value) => Ok(value),
            _ => Err(GraphError::at(id, Some(name), GraphErrorKind::Type)),
        }
    }

    pub(crate) fn long(
        &self,
        id: NodeId,
        owner: &'static str,
        name: &'static str,
    ) -> Result<i64, GraphError> {
        match self.field(id, owner, name)? {
            FieldValue::Long(value) => Ok(value),
            _ => Err(GraphError::at(id, Some(name), GraphErrorKind::Type)),
        }
    }

    pub(crate) fn double_bits(
        &self,
        id: NodeId,
        owner: &'static str,
        name: &'static str,
    ) -> Result<u64, GraphError> {
        match self.field(id, owner, name)? {
            FieldValue::DoubleBits(value) => Ok(value),
            _ => Err(GraphError::at(id, Some(name), GraphErrorKind::Type)),
        }
    }

    pub(crate) fn boolean(
        &self,
        id: NodeId,
        owner: &'static str,
        name: &'static str,
    ) -> Result<bool, GraphError> {
        match self.field(id, owner, name)? {
            FieldValue::Boolean(value) => Ok(value != 0),
            _ => Err(GraphError::at(id, Some(name), GraphErrorKind::Type)),
        }
    }

    pub(crate) fn string_units(&self, id: NodeId) -> Result<&'a [u16], GraphError> {
        match self.node(id)? {
            Node::String(value) => {
                if value.units.len() > self.limits.string_units {
                    return Err(GraphError::at(id, None, GraphErrorKind::Budget));
                }
                self.charge(value.units.len())?;
                let mut bytes = 0_usize;
                for unit in &value.units {
                    let width = if *unit == 0 {
                        2
                    } else if *unit < 0x80 {
                        1
                    } else if *unit < 0x800 {
                        2
                    } else {
                        3
                    };
                    bytes = bytes.checked_add(width).ok_or(GraphError::at(
                        id,
                        None,
                        GraphErrorKind::Budget,
                    ))?;
                    if bytes > self.limits.string_bytes {
                        return Err(GraphError::at(id, None, GraphErrorKind::Budget));
                    }
                }
                Ok(&value.units)
            }
            _ => Err(GraphError::at(id, None, GraphErrorKind::Type)),
        }
    }

    pub(crate) fn string(&self, id: NodeId) -> Result<String, GraphError> {
        let units = self.string_units(id)?;
        let mut result = String::new();
        let reserve: usize = units
            .iter()
            .map(|unit| {
                if *unit == 0 {
                    2
                } else if *unit < 0x80 {
                    1
                } else if *unit < 0x800 {
                    2
                } else {
                    3
                }
            })
            .sum();
        self.allocate(reserve)?;
        result
            .try_reserve_exact(reserve)
            .map_err(|_| GraphError::at(id, None, GraphErrorKind::Allocation))?;
        for value in char::decode_utf16(units.iter().copied()) {
            result.push(
                value.map_err(|_| GraphError::at(id, None, GraphErrorKind::UnsupportedUtf16))?,
            );
        }
        Ok(result)
    }

    pub(crate) fn array_list(&self, id: NodeId) -> Result<Vec<Value>, GraphError> {
        self.read_array_list(id, true)
    }

    /// ArrayList.readObject consumes its compatibility integer without interpreting it
    pub(crate) fn array_list_compatible(&self, id: NodeId) -> Result<Vec<Value>, GraphError> {
        self.read_array_list(id, false)
    }

    fn read_array_list(&self, id: NodeId, strict_compatibility: bool) -> Result<Vec<Value>, GraphError> {
        self.expect_class(id, "java.util.ArrayList")?;
        let length = self.int(id, "java.util.ArrayList", "size")?;
        if length < 0 || length as usize > self.limits.array_elements {
            return Err(GraphError::at(id, Some("size"), GraphErrorKind::Length));
        }
        let slots = self.slots(id)?;
        let [slot] = slots else {
            return Err(GraphError::at(id, None, GraphErrorKind::Slots));
        };
        let mut prefix = [0_u8; 4];
        let mut used = 0;
        let mut position = 0;
        while used < 4 {
            let Some(Record::Block { bytes, .. }) = slot.custom.get(position) else {
                return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
            };
            if bytes.is_empty() || bytes.len() > 4 - used {
                return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
            }
            prefix[used..used + bytes.len()].copy_from_slice(bytes);
            used += bytes.len();
            position += 1;
        }
        if (strict_compatibility && i32::from_be_bytes(prefix) != length)
            || slot.custom.len() - position != length as usize {
            return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
        }
        self.charge(length as usize)?;
        self.allocate(
            (length as usize)
                .checked_mul(std::mem::size_of::<Value>())
                .ok_or(GraphError::at(id, None, GraphErrorKind::Budget))?,
        )?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(length as usize)
            .map_err(|_| GraphError::at(id, None, GraphErrorKind::Allocation))?;
        for record in &slot.custom[position..] {
            let Record::Value(value) = record else {
                return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
            };
            if let Value::Handle(value) = value {
                self.node(*value)?;
            }
            result.push(*value);
        }
        Ok(result)
    }

    pub(crate) fn synchronized_list(&self, id: NodeId) -> Result<(NodeId, Vec<Value>), GraphError> {
        self.expect_class(id, "java.util.Collections$SynchronizedList")?;
        let list = self.required_reference(id, "java.util.Collections$SynchronizedList", "list")?;
        if self.reference(id, "java.util.Collections$SynchronizedCollection", "c")?
            != Value::Handle(list)
            || self.reference(id, "java.util.Collections$SynchronizedCollection", "mutex")?
                != Value::Handle(id)
        {
            return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
        }
        Ok((list, self.array_list(list)?))
    }
}
