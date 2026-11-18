// SPDX-License-Identifier: GPL-2.0-or-later

//! Bounded data-only Java serialization graph; no class loading or serialization callbacks
//!
//! Node IDs preserve wire identity, including cycles and distinct equal values
//! Custom class data stays ordered and opaque; this module does not hydrate Java classes
//! Strings retain UTF-16 code units, including lone surrogates

use std::fmt;
use std::io::{self, BufRead, Read};
use std::mem::size_of;

#[path = "object_stream_schema.rs"]
mod schema;

const BASE_HANDLE: u32 = 0x7e0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassId(usize);

#[derive(Debug)]
pub struct FieldSchema {
    pub kind: u8,
    pub name: &'static str,
    pub signature: Option<&'static str>,
}

#[derive(Debug)]
pub struct ClassSchema {
    pub name: &'static str,
    pub uid: i64,
    pub flags: u8,
    pub superclass: Option<&'static str>,
    pub fields: &'static [FieldSchema],
}

pub fn class_by_name(name: &str) -> Option<ClassId> {
    schema::CLASSES.iter().position(|item| item.name == name).map(ClassId)
}

pub fn class_schema(class: ClassId) -> &'static ClassSchema {
    &schema::CLASSES[class.0]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaString {
    pub units: Vec<u16>,
    pub long: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Handle(NodeId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    Value(Value),
    Block { bytes: Vec<u8>, long: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldValue {
    Boolean(u8),
    Byte(u8),
    Char(u16),
    Short(i16),
    Int(i32),
    Long(i64),
    FloatBits(u32),
    DoubleBits(u64),
    Reference(Value),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassDescriptor {
    pub class: ClassId,
    pub signatures: Vec<Option<NodeId>>,
    pub annotations: Vec<Record>,
    pub superclass: Option<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassData {
    pub descriptor: NodeId,
    pub values: Vec<FieldValue>,
    pub custom: Vec<Record>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaObject {
    pub descriptor: NodeId,
    pub slots: Vec<ClassData>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArrayData {
    Ints(Vec<i32>),
    Doubles(Vec<u64>),
    Floats(Vec<u32>),
    References(Vec<Value>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaArray {
    pub descriptor: NodeId,
    pub data: ArrayData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Descriptor(ClassDescriptor),
    String(JavaString),
    Object(JavaObject),
    Array(JavaArray),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arena {
    pub nodes: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub arena: Arena,
    pub records: Vec<Record>,
    pub consumed: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub handles: usize,
    pub array_elements: usize,
    pub total_array_elements: usize,
    pub string_bytes: usize,
    pub string_units: usize,
    pub name_units: usize,
    pub records: usize,
    pub depth: usize,
    pub allocated_bytes: usize,
    pub work: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: 128 * 1024 * 1024,
            output_bytes: 128 * 1024 * 1024,
            handles: 2_000_000,
            array_elements: 1_000_000,
            total_array_elements: 16_000_000,
            string_bytes: 8 * 1024 * 1024,
            string_units: 4 * 1024 * 1024,
            name_units: 1024,
            records: 4_000_000,
            depth: 64,
            allocated_bytes: 512 * 1024 * 1024,
            work: 128_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitKind {
    Input, Output, Handles, Array, TotalArrays, String, Name, Records, Depth, Allocation, Work,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Header,
    Truncated { needed: usize },
    InvalidReference,
    ReferenceType,
    UnknownClass,
    UidMismatch,
    SchemaMismatch,
    UnsupportedToken(u8),
    IncompleteDescriptor,
    InvalidLength,
    MalformedUtf,
    InvalidGraph,
    Allocation,
    Poisoned,
    InvalidLimits,
    Io(io::ErrorKind),
    Limit(LimitKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamError {
    pub offset: usize,
    pub kind: ErrorKind,
}

impl fmt::Display for StreamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Java object stream error at byte {}: {:?}", self.offset, self.kind)
    }
}

impl std::error::Error for StreamError {}

type Result<T> = std::result::Result<T, StreamError>;

fn error(offset: usize, kind: ErrorKind) -> StreamError {
    StreamError { offset, kind }
}

#[derive(Clone)]
struct Budget {
    limits: Limits,
    allocated: usize,
    work: usize,
    records: usize,
    elements: usize,
}

impl Budget {
    fn new(limits: Limits) -> Result<Self> {
        if limits.depth > 128 || limits.handles > (u32::MAX - BASE_HANDLE) as usize {
            return Err(error(0, ErrorKind::InvalidLimits));
        }
        Ok(Self { limits, allocated: 0, work: 0, records: 0, elements: 0 })
    }

    fn add(counter: &mut usize, amount: usize, maximum: usize, kind: LimitKind, offset: usize) -> Result<()> {
        let next = counter.checked_add(amount).ok_or(error(offset, ErrorKind::Limit(kind)))?;
        if next > maximum { return Err(error(offset, ErrorKind::Limit(kind))); }
        *counter = next;
        Ok(())
    }

    fn work(&mut self, amount: usize, offset: usize) -> Result<()> {
        Self::add(&mut self.work, amount, self.limits.work, LimitKind::Work, offset)
    }

    fn record(&mut self, offset: usize) -> Result<()> {
        Self::add(&mut self.records, 1, self.limits.records, LimitKind::Records, offset)
    }

    fn elements(&mut self, amount: usize, offset: usize) -> Result<()> {
        if amount > self.limits.array_elements { return Err(error(offset, ErrorKind::Limit(LimitKind::Array))); }
        Self::add(&mut self.elements, amount, self.limits.total_array_elements, LimitKind::TotalArrays, offset)
    }

    fn depth(&self, depth: usize, offset: usize) -> Result<()> {
        if depth > self.limits.depth { return Err(error(offset, ErrorKind::Limit(LimitKind::Depth))); }
        Ok(())
    }

    fn allocation(&mut self, count: usize, width: usize, offset: usize) -> Result<()> {
        let bytes = count.checked_mul(width).ok_or(error(offset, ErrorKind::Limit(LimitKind::Allocation)))?;
        Self::add(&mut self.allocated, bytes, self.limits.allocated_bytes, LimitKind::Allocation, offset)
    }

    fn vector<T>(&mut self, count: usize, offset: usize) -> Result<Vec<T>> {
        self.allocation(count, size_of::<T>(), offset)?;
        let mut values = Vec::new();
        values.try_reserve_exact(count).map_err(|_| error(offset, ErrorKind::Allocation))?;
        Ok(values)
    }

    fn grow<T>(&mut self, values: &mut Vec<T>, maximum: usize, offset: usize) -> Result<()> {
        if values.len() >= maximum { return Err(error(offset, ErrorKind::Limit(LimitKind::Allocation))); }
        if values.len() == values.capacity() {
            let capacity = values.capacity().saturating_mul(2).max(8).min(maximum);
            self.allocation(capacity, size_of::<T>(), offset)?;
            values.try_reserve_exact(capacity - values.len()).map_err(|_| error(offset, ErrorKind::Allocation))?;
        }
        Ok(())
    }
}

fn decode_utf(bytes: &[u8], budget: &mut Budget, offset: usize) -> Result<Vec<u16>> {
    if bytes.len() > budget.limits.string_bytes { return Err(error(offset, ErrorKind::Limit(LimitKind::String))); }
    let mut units = budget.vector(bytes.len().min(budget.limits.string_units), offset)?;
    let mut cursor = 0;
    while cursor < bytes.len() {
        budget.work(1, offset + cursor)?;
        let first = bytes[cursor];
        cursor += 1;
        let unit = match first {
            0x00..=0x7f => u16::from(first),
            0xc0..=0xdf => {
                let second = *bytes.get(cursor).ok_or(error(offset + cursor, ErrorKind::MalformedUtf))?;
                if second & 0xc0 != 0x80 { return Err(error(offset + cursor, ErrorKind::MalformedUtf)); }
                cursor += 1;
                (u16::from(first & 31) << 6) | u16::from(second & 63)
            }
            0xe0..=0xef => {
                let second = *bytes.get(cursor).ok_or(error(offset + cursor, ErrorKind::MalformedUtf))?;
                let third = *bytes.get(cursor + 1).ok_or(error(offset + cursor, ErrorKind::MalformedUtf))?;
                if second & 0xc0 != 0x80 || third & 0xc0 != 0x80 { return Err(error(offset + cursor, ErrorKind::MalformedUtf)); }
                cursor += 2;
                (u16::from(first & 15) << 12) | (u16::from(second & 63) << 6) | u16::from(third & 63)
            }
            _ => return Err(error(offset + cursor - 1, ErrorKind::MalformedUtf)),
        };
        if units.len() == budget.limits.string_units { return Err(error(offset + cursor, ErrorKind::Limit(LimitKind::String))); }
        units.push(unit);
    }
    Ok(units)
}

impl Arena {
    pub fn node(&self, id: NodeId) -> Result<&Node> {
        self.nodes.get(id.0 as usize).ok_or(error(0, ErrorKind::InvalidReference))
    }

    pub fn descriptor(&self, id: NodeId) -> Result<&ClassDescriptor> {
        match self.node(id)? { Node::Descriptor(item) => Ok(item), _ => Err(error(0, ErrorKind::ReferenceType)) }
    }

    fn hierarchy(&self, descriptor: NodeId, budget: &mut Budget, offset: usize) -> Result<Vec<NodeId>> {
        let mut chain = budget.vector(4, offset)?;
        let mut next = Some(descriptor);
        while let Some(id) = next {
            budget.depth(chain.len(), offset)?;
            if chain.contains(&id) { return Err(error(offset, ErrorKind::InvalidGraph)); }
            let desc = self.descriptor(id)?;
            if chain.len() == chain.capacity() { budget.grow(&mut chain, budget.limits.depth + 1, offset)?; }
            chain.push(id);
            next = desc.superclass;
        }
        chain.reverse();
        Ok(chain)
    }

    fn accepts(&self, value: Value, signature: &str) -> Result<bool> {
        let Value::Handle(id) = value else { return Ok(true); };
        let node = self.node(id)?;
        if signature == "Ljava/lang/Object;" { return Ok(true); }
        if let Node::String(_) = node { return Ok(signature == "Ljava/lang/String;"); }
        let descriptor = match node {
            Node::Object(item) => item.descriptor,
            Node::Array(item) => item.descriptor,
            _ => return Ok(false),
        };
        let mut class = Some(class_schema(self.descriptor(descriptor)?.class));
        while let Some(info) = class {
            if signature.starts_with('[') {
                return Ok(info.name.bytes().map(|byte| if byte == b'.' { b'/' } else { byte }).eq(signature.bytes()));
            }
            if signature.strip_prefix('L').and_then(|s| s.strip_suffix(';')).is_some_and(|s| s.bytes().map(|byte| if byte == b'/' { b'.' } else { byte }).eq(info.name.bytes())) {
                return Ok(true);
            }
            if signature == "Ljava/util/List;" && ["java.util.ArrayList", "java.util.Collections$SynchronizedList"].contains(&info.name) { return Ok(true); }
            if signature == "Ljava/util/Collection;" && ["java.util.ArrayList", "java.util.Collections$SynchronizedList", "java.util.Collections$SynchronizedCollection"].contains(&info.name) { return Ok(true); }
            class = info.superclass.and_then(class_by_name).map(class_schema);
        }
        Ok(false)
    }
}

fn utf_length(units: &[u16], budget: &mut Budget, offset: usize) -> Result<usize> {
    if units.len() > budget.limits.string_units { return Err(error(offset, ErrorKind::Limit(LimitKind::String))); }
    budget.work(units.len(), offset)?;
    let mut length = 0usize;
    for unit in units {
        let count = if (1..=127).contains(unit) { 1 } else if *unit <= 0x7ff { 2 } else { 3 };
        length = length.checked_add(count).ok_or(error(offset, ErrorKind::InvalidLength))?;
    }
    if length > budget.limits.string_bytes { return Err(error(offset, ErrorKind::Limit(LimitKind::String))); }
    Ok(length)
}

fn validate_record(arena: &Arena, record: &Record, budget: &mut Budget) -> Result<()> {
    budget.record(0)?;
    match record {
        Record::Value(Value::Handle(id)) => { arena.node(*id)?; }
        Record::Value(Value::Null) => {}
        Record::Block { bytes, long } => {
            if bytes.len() > i32::MAX as usize || (!long && bytes.len() > 255) { return Err(error(0, ErrorKind::InvalidLength)); }
            if bytes.len() > budget.limits.output_bytes { return Err(error(0, ErrorKind::Limit(LimitKind::Output))); }
            budget.work(bytes.len(), 0)?;
        }
    }
    Ok(())
}

fn field_matches(arena: &Arena, value: &FieldValue, field: &FieldSchema) -> Result<bool> {
    Ok(match (field.kind, value) {
        (b'Z', FieldValue::Boolean(_)) | (b'B', FieldValue::Byte(_)) |
        (b'C', FieldValue::Char(_)) | (b'S', FieldValue::Short(_)) |
        (b'I', FieldValue::Int(_)) | (b'J', FieldValue::Long(_)) |
        (b'F', FieldValue::FloatBits(_)) | (b'D', FieldValue::DoubleBits(_)) => true,
        (b'L' | b'[', FieldValue::Reference(reference)) => arena.accepts(*reference, field.signature.unwrap())?,
        _ => false,
    })
}

fn validate_arena(arena: &Arena, budget: &mut Budget) -> Result<()> {
    if arena.nodes.len() > budget.limits.handles { return Err(error(0, ErrorKind::Limit(LimitKind::Handles))); }
    for node in &arena.nodes {
        budget.work(1, 0)?;
        match node {
            Node::Descriptor(descriptor) => {
                let info = class_schema(descriptor.class);
                if descriptor.signatures.len() != info.fields.len() { return Err(error(0, ErrorKind::InvalidGraph)); }
                let parent = descriptor.superclass.map(|id| arena.descriptor(id).map(|desc| class_schema(desc.class).name)).transpose()?;
                if parent != info.superclass { return Err(error(0, ErrorKind::InvalidGraph)); }
                for (field, signature) in info.fields.iter().zip(&descriptor.signatures) {
                    budget.work(1, 0)?;
                    match (field.signature, signature) {
                        (None, None) => {}
                        (Some(expected), Some(id)) => match arena.node(*id)? {
                            Node::String(value) if expected.encode_utf16().eq(value.units.iter().copied()) => {}
                            _ => return Err(error(0, ErrorKind::InvalidGraph)),
                        },
                        _ => return Err(error(0, ErrorKind::InvalidGraph)),
                    }
                }
                for record in &descriptor.annotations { validate_record(arena, record, budget)?; }
            }
            Node::String(value) => {
                let length = utf_length(&value.units, budget, 0)?;
                if !value.long && length > u16::MAX as usize { return Err(error(0, ErrorKind::InvalidLength)); }
            }
            Node::Object(object) => {
                let name = class_schema(arena.descriptor(object.descriptor)?.class).name;
                if name.starts_with('[') || ["java.lang.Number", "java.awt.color.ColorSpace", "org.locationtech.jts.index.strtree.AbstractNode", "org.locationtech.jts.index.strtree.AbstractSTRtree"].contains(&name) { return Err(error(0, ErrorKind::InvalidGraph)); }
                let hierarchy = arena.hierarchy(object.descriptor, budget, 0)?;
                if hierarchy.len() != object.slots.len() { return Err(error(0, ErrorKind::InvalidGraph)); }
                for (descriptor, slot) in hierarchy.iter().zip(&object.slots) {
                    if *descriptor != slot.descriptor { return Err(error(0, ErrorKind::InvalidGraph)); }
                    let info = class_schema(arena.descriptor(*descriptor)?.class);
                    if slot.values.len() != info.fields.len() || (info.flags & 1 == 0 && !slot.custom.is_empty()) { return Err(error(0, ErrorKind::InvalidGraph)); }
                    for (field, value) in info.fields.iter().zip(&slot.values) {
                        budget.work(1, 0)?;
                        if !field_matches(arena, value, field)? { return Err(error(0, ErrorKind::InvalidGraph)); }
                    }
                    for record in &slot.custom { validate_record(arena, record, budget)?; }
                }
            }
            Node::Array(array) => {
                let name = class_schema(arena.descriptor(array.descriptor)?.class).name;
                let count = match (&array.data, name) {
                    (ArrayData::Ints(values), "[I") => values.len(),
                    (ArrayData::Doubles(values), "[D") => values.len(),
                    (ArrayData::Floats(values), "[F") => values.len(),
                    (ArrayData::References(values), "[Lbiogenesis.Gene;" | "[Ljava.awt.Color;") => values.len(),
                    _ => return Err(error(0, ErrorKind::InvalidGraph)),
                };
                if count > i32::MAX as usize { return Err(error(0, ErrorKind::InvalidLength)); }
                budget.elements(count, 0)?;
                budget.work(count, 0)?;
                if let ArrayData::References(values) = &array.data {
                    let signature = if name == "[Lbiogenesis.Gene;" { "Lbiogenesis/Gene;" } else { "Ljava/awt/Color;" };
                    for value in values {
                        if !arena.accepts(*value, signature)? { return Err(error(0, ErrorKind::InvalidGraph)); }
                    }
                }
            }
        }
    }
    Ok(())
}

fn canonical_float(bits: u32) -> u32 {
    if bits & 0x7f800000 == 0x7f800000 && bits & 0x007fffff != 0 { 0x7fc00000 } else { bits }
}

fn canonical_double(bits: u64) -> u64 {
    if bits & 0x7ff0000000000000 == 0x7ff0000000000000 && bits & 0x000fffffffffffff != 0 { 0x7ff8000000000000 } else { bits }
}

pub struct StreamWriter<'a> {
    arena: &'a Arena,
    output: Vec<u8>,
    handles: Vec<Option<u32>>,
    active_descriptors: Vec<NodeId>,
    next_handle: u32,
    budget: Budget,
    poisoned: bool,
}

impl<'a> StreamWriter<'a> {
    pub fn new(arena: &'a Arena, limits: Limits) -> Result<Self> {
        Self::new_charged(arena, limits, 0, 0)
    }

    pub(crate) fn new_charged(arena: &'a Arena, limits: Limits, allocated: usize, work: usize) -> Result<Self> {
        let mut budget = Budget::new(limits)?;
        budget.allocation(allocated, 1, 0)?;
        budget.work(work, 0)?;
        validate_arena(arena, &mut budget)?;
        let mut handles = budget.vector(arena.nodes.len(), 0)?;
        handles.resize(arena.nodes.len(), None);
        let mut writer = Self { arena, output: Vec::new(), handles, active_descriptors: Vec::new(), next_handle: BASE_HANDLE, budget, poisoned: false };
        writer.bytes(&[0xac, 0xed, 0, 5])?;
        Ok(writer)
    }

    /// Bind a graph before assigning object handles, retaining the emitted stream and all budget charges
    pub(crate) fn bind_empty_arena<'b>(mut self, arena: &'b Arena) -> Result<StreamWriter<'b>> {
        let offset = self.position();
        if self.poisoned { return Err(error(offset, ErrorKind::Poisoned)); }
        if !self.arena.nodes.is_empty() || !self.handles.is_empty()
            || !self.active_descriptors.is_empty() || self.next_handle != BASE_HANDLE {
            return Err(error(offset, ErrorKind::InvalidGraph));
        }
        validate_arena(arena, &mut self.budget)?;
        let mut handles = self.budget.vector(arena.nodes.len(), offset)?;
        handles.resize(arena.nodes.len(), None);
        Ok(StreamWriter { arena, output: self.output, handles, active_descriptors: self.active_descriptors,
            next_handle: self.next_handle, budget: self.budget, poisoned: self.poisoned })
    }

    pub fn position(&self) -> usize { self.output.len() }
    pub(crate) fn output(&self) -> &[u8] { &self.output }
    pub(crate) fn usage(&self) -> (usize, usize) { (self.budget.allocated, self.budget.work) }
    pub(crate) fn charge_external(&mut self, allocated: usize, work: usize) -> Result<()> {
        let result = (|| {
            self.budget.allocation(allocated, 1, self.output.len())?;
            self.budget.work(work, self.output.len())
        })();
        if result.is_err() { self.poisoned = true; }
        result
    }


    fn reserve_output(&mut self, count: usize) -> Result<()> {
        let offset = self.output.len();
        self.budget.work(count.saturating_add(1), offset)?;
        let needed = offset.checked_add(count).ok_or(error(offset, ErrorKind::InvalidLength))?;
        if needed > self.budget.limits.output_bytes { return Err(error(offset, ErrorKind::Limit(LimitKind::Output))); }
        if needed > self.output.capacity() {
            let capacity = self.output.capacity().saturating_mul(2).max(needed).min(self.budget.limits.output_bytes);
            self.budget.allocation(capacity, 1, offset)?;
            self.output.try_reserve_exact(capacity - offset).map_err(|_| error(offset, ErrorKind::Allocation))?;
        }
        Ok(())
    }

    fn bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.reserve_output(bytes.len())?;
        self.output.extend_from_slice(bytes);
        Ok(())
    }

    fn utf_body(&mut self, units: &[u16]) -> Result<()> {
        for unit in units {
            if (1..=127).contains(unit) { self.bytes(&[*unit as u8])?; }
            else if *unit <= 0x7ff { self.bytes(&[0xc0 | (*unit >> 6) as u8, 0x80 | (*unit & 63) as u8])?; }
            else { self.bytes(&[0xe0 | (*unit >> 12) as u8, 0x80 | ((*unit >> 6) & 63) as u8, 0x80 | (*unit & 63) as u8])?; }
        }
        Ok(())
    }

    fn name(&mut self, name: &'static str) -> Result<()> {
        if name.len() > u16::MAX as usize || !name.is_ascii() { return Err(error(self.position(), ErrorKind::InvalidGraph)); }
        if name.len() > self.budget.limits.name_units { return Err(error(self.position(), ErrorKind::Limit(LimitKind::Name))); }
        self.bytes(&(name.len() as u16).to_be_bytes())?;
        self.bytes(name.as_bytes())
    }

    fn assign(&mut self, id: NodeId) -> Result<()> {
        let slot = self.handles.get_mut(id.0 as usize).ok_or(error(self.output.len(), ErrorKind::InvalidReference))?;
        if slot.is_some() { return Err(error(self.output.len(), ErrorKind::InvalidGraph)); }
        *slot = Some(self.next_handle);
        self.next_handle = self.next_handle.checked_add(1).ok_or(error(self.output.len(), ErrorKind::InvalidLength))?;
        Ok(())
    }

    fn descriptor(&mut self, id: Option<NodeId>, depth: usize) -> Result<()> {
        match id { None => self.bytes(&[0x70]), Some(value) => {
            self.arena.descriptor(value)?;
            if self.active_descriptors.contains(&value) { return Err(error(self.position(), ErrorKind::IncompleteDescriptor)); }
            self.value(Value::Handle(value), depth)
        } }
    }

    fn field(&mut self, value: &FieldValue, depth: usize) -> Result<()> {
        match value {
            FieldValue::Boolean(value) | FieldValue::Byte(value) => self.bytes(&[*value]),
            FieldValue::Char(value) => self.bytes(&value.to_be_bytes()),
            FieldValue::Short(value) => self.bytes(&value.to_be_bytes()),
            FieldValue::Int(value) => self.bytes(&value.to_be_bytes()),
            FieldValue::Long(value) => self.bytes(&value.to_be_bytes()),
            FieldValue::FloatBits(value) => self.bytes(&canonical_float(*value).to_be_bytes()),
            FieldValue::DoubleBits(value) => self.bytes(&canonical_double(*value).to_be_bytes()),
            FieldValue::Reference(value) => self.value(*value, depth),
        }
    }

    fn annotations(&mut self, records: &[Record], depth: usize) -> Result<()> {
        for record in records { self.record(record, depth)?; }
        self.bytes(&[0x78])
    }

    fn value(&mut self, value: Value, depth: usize) -> Result<()> {
        self.budget.depth(depth, self.position())?;
        let Value::Handle(id) = value else { return self.bytes(&[0x70]); };
        if let Some(handle) = *self.handles.get(id.0 as usize).ok_or(error(self.position(), ErrorKind::InvalidReference))? {
            self.bytes(&[0x71])?;
            return self.bytes(&handle.to_be_bytes());
        }
        let arena = self.arena;
        match arena.node(id)? {
            Node::Descriptor(descriptor) => {
                let info = class_schema(descriptor.class);
                self.bytes(&[0x72])?;
                self.name(info.name)?;
                self.bytes(&info.uid.to_be_bytes())?;
                self.assign(id)?;
                let offset = self.position();
                let maximum = self.budget.limits.depth + 1;
                self.budget.grow(&mut self.active_descriptors, maximum, offset)?;
                self.active_descriptors.push(id);
                self.bytes(&[info.flags])?;
                self.bytes(&(info.fields.len() as u16).to_be_bytes())?;
                for (field, signature) in info.fields.iter().zip(&descriptor.signatures) {
                    self.bytes(&[field.kind])?;
                    self.name(field.name)?;
                    if let Some(reference) = signature { self.value(Value::Handle(*reference), depth + 1)?; }
                }
                self.annotations(&descriptor.annotations, depth + 1)?;
                self.descriptor(descriptor.superclass, depth + 1)?;
                self.active_descriptors.pop();
            }
            Node::String(value) => {
                let count = utf_length(&value.units, &mut self.budget, self.output.len())?;
                self.assign(id)?;
                if value.long {
                    self.bytes(&[0x7c])?;
                    self.bytes(&(count as u64).to_be_bytes())?;
                } else {
                    self.bytes(&[0x74])?;
                    self.bytes(&(count as u16).to_be_bytes())?;
                }
                self.utf_body(&value.units)?;
            }
            Node::Object(object) => {
                self.bytes(&[0x73])?;
                self.descriptor(Some(object.descriptor), depth + 1)?;
                self.assign(id)?;
                for slot in &object.slots {
                    for field in &slot.values { self.field(field, depth + 1)?; }
                    if class_schema(arena.descriptor(slot.descriptor)?.class).flags & 1 != 0 { self.annotations(&slot.custom, depth + 1)?; }
                }
            }
            Node::Array(array) => {
                self.bytes(&[0x75])?;
                self.descriptor(Some(array.descriptor), depth + 1)?;
                self.assign(id)?;
                let count = match &array.data { ArrayData::Ints(values) => values.len(), ArrayData::Doubles(values) => values.len(), ArrayData::Floats(values) => values.len(), ArrayData::References(values) => values.len() };
                self.bytes(&(count as i32).to_be_bytes())?;
                match &array.data {
                    ArrayData::Ints(values) => for value in values { self.bytes(&value.to_be_bytes())?; },
                    ArrayData::Doubles(values) => for value in values { self.bytes(&canonical_double(*value).to_be_bytes())?; },
                    ArrayData::Floats(values) => for value in values { self.bytes(&canonical_float(*value).to_be_bytes())?; },
                    ArrayData::References(values) => for value in values { self.value(*value, depth + 1)?; },
                }
            }
        }
        Ok(())
    }

    fn record(&mut self, record: &Record, depth: usize) -> Result<()> {
        self.budget.depth(depth, self.position())?;
        self.budget.record(self.position())?;
        match record {
            Record::Value(value) => self.value(*value, depth),
            Record::Block { bytes, long } => {
                if bytes.len() > i32::MAX as usize || (!long && bytes.len() > 255) { return Err(error(self.position(), ErrorKind::InvalidLength)); }
                if *long { self.bytes(&[0x7a])?; self.bytes(&(bytes.len() as i32).to_be_bytes())?; }
                else { self.bytes(&[0x77, bytes.len() as u8])?; }
                self.bytes(bytes)
            }
        }
    }

    /// Append one record while retaining stream handles for later references
    pub fn write_record(&mut self, record: &Record) -> Result<()> {
        if self.poisoned { return Err(error(self.position(), ErrorKind::Poisoned)); }
        let result = validate_record(self.arena, record, &mut self.budget).and_then(|_| self.record(record, 0));
        if result.is_err() { self.poisoned = true; }
        result
    }

    /// Write one integer block after charging work and output storage, without a staging allocation
    pub(crate) fn write_i32_block(&mut self, values: &[i32]) -> Result<()> {
        if self.poisoned { return Err(error(self.position(), ErrorKind::Poisoned)); }
        let result = (|| {
            let length = values.len().checked_mul(4).ok_or(error(0, ErrorKind::InvalidLength))?;
            self.budget.record(0)?;
            if length > i32::MAX as usize { return Err(error(0, ErrorKind::InvalidLength)); }
            if length > self.budget.limits.output_bytes { return Err(error(0, ErrorKind::Limit(LimitKind::Output))); }
            self.budget.work(length, 0)?;
            self.budget.depth(0, self.position())?;
            self.budget.record(self.position())?;
            if length > 255 {
                self.bytes(&[0x7a])?;
                self.bytes(&(length as i32).to_be_bytes())?;
            } else {
                self.bytes(&[0x77, length as u8])?;
            }
            self.reserve_output(length)?;
            for value in values { self.output.extend_from_slice(&value.to_be_bytes()); }
            Ok(())
        })();
        if result.is_err() { self.poisoned = true; }
        result
    }

    pub fn finish(self) -> Result<Vec<u8>> {
        if self.poisoned { return Err(error(self.output.len(), ErrorKind::Poisoned)); }
        Ok(self.output)
    }
}

pub fn encode_document(document: &Document, limits: Limits) -> Result<Vec<u8>> {
    let mut writer = StreamWriter::new(&document.arena, limits)?;
    for record in &document.records { writer.write_record(record)?; }
    writer.finish()
}

/// Keep typed assembly allocation and work live within the original writer limits
pub(crate) fn encode_document_charged(
    document: &Document, limits: Limits, allocated: usize, work: usize,
) -> Result<Vec<u8>> {
    let mut writer = StreamWriter::new_charged(&document.arena, limits, allocated, work)?;
    for record in &document.records { writer.write_record(record)?; }
    writer.finish()
}

struct LimitedRead<'a> {
    source: &'a mut dyn Read,
    remaining: usize,
}

impl Read for LimitedRead<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = output.len().min(self.remaining);
        let read = self.source.read(&mut output[..count])?;
        self.remaining -= read;
        Ok(read)
    }
}

struct Buffered<'a> {
    source: LimitedRead<'a>,
    buffer: Vec<u8>,
    used: usize,
    end: usize,
}

impl BufRead for Buffered<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.used == self.end {
            self.used = 0;
            loop {
                match self.source.read(&mut self.buffer) {
                    Err(value) if value.kind() == io::ErrorKind::Interrupted => continue,
                    result => { self.end = result?; break; }
                }
            }
        }
        Ok(&self.buffer[self.used..self.end])
    }
    fn consume(&mut self, amount: usize) { self.used = (self.used + amount).min(self.end); }
}

impl Read for Buffered<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() { return Ok(0); }
        let input = self.fill_buf()?;
        let count = input.len().min(output.len());
        output[..count].copy_from_slice(&input[..count]);
        self.consume(count);
        Ok(count)
    }
}

enum Input<'a> {
    Slice(&'a [u8]),
    Stream { reader: Buffered<'a>, scratch: Vec<u8> },
}

impl Input<'_> {
    fn remaining(&self, offset: usize) -> Option<usize> {
        match self { Self::Slice(input) => Some(input.len() - offset), Self::Stream { .. } => None }
    }

    fn peek(&mut self, offset: usize) -> Result<Option<u8>> {
        match self {
            Self::Slice(input) => Ok(input.get(offset).copied()),
            Self::Stream { reader, .. } => {
                if reader.used == reader.end && reader.source.remaining == 0 {
                    // Strict document readers must distinguish exact-cap EOF from excess input
                    let mut sentinel = [0];
                    loop {
                        match reader.source.source.read(&mut sentinel) {
                            Ok(0) => return Ok(None),
                            Ok(_) => return Err(error(offset, ErrorKind::Limit(LimitKind::Input))),
                            Err(value) if value.kind() == io::ErrorKind::Interrupted => continue,
                            Err(value) => return Err(error(offset, ErrorKind::Io(value.kind()))),
                        }
                    }
                }
                Ok(reader.fill_buf().map_err(|value| error(offset, ErrorKind::Io(value.kind())))?.first().copied())
            }
        }
    }

    fn take<'b>(&'b mut self, count: usize, budget: &mut Budget, offset: &mut usize) -> Result<&'b [u8]> {
        budget.work(count.saturating_add(1), *offset)?;
        let end = offset.checked_add(count).ok_or(error(*offset, ErrorKind::InvalidLength))?;
        let bytes = match self {
            Self::Slice(input) => input.get(*offset..end)
                .ok_or(error(*offset, ErrorKind::Truncated { needed: count }))?,
            Self::Stream { reader, scratch } => {
                if end > budget.limits.input_bytes { return Err(error(*offset, ErrorKind::Limit(LimitKind::Input))); }
                if scratch.capacity() < count {
                    budget.allocation(count, 1, *offset)?;
                    scratch.try_reserve_exact(count.saturating_sub(scratch.len()))
                        .map_err(|_| error(*offset, ErrorKind::Allocation))?;
                }
                scratch.resize(count, 0);
                reader.read_exact(scratch).map_err(|value| error(*offset,
                    if value.kind() == io::ErrorKind::UnexpectedEof { ErrorKind::Truncated { needed: count } }
                    else { ErrorKind::Io(value.kind()) }))?;
                scratch.as_slice()
            }
        };
        *offset = end;
        Ok(bytes)
    }
}

pub struct StreamReader<'a> {
    input: Input<'a>,
    offset: usize,
    arena: Arena,
    budget: Budget,
    active_descriptors: Vec<NodeId>,
    poisoned: bool,
}

impl<'a> StreamReader<'a> {
    pub fn new(input: &'a [u8], limits: Limits) -> Result<Self> {
        if input.len() > limits.input_bytes { return Err(error(0, ErrorKind::Limit(LimitKind::Input))); }
        let mut reader = Self { input: Input::Slice(input), offset: 0, arena: Arena::default(), budget: Budget::new(limits)?, active_descriptors: Vec::new(), poisoned: false };
        if reader.take(4)? != [0xac, 0xed, 0, 5] { return Err(error(0, ErrorKind::Header)); }
        Ok(reader)
    }

    /// Read incrementally through the same schema parser and cumulative budget
    /// Strict document EOF checks may consume one excess probe byte at the input cap
    /// One-record completion does not probe EOF; accepted logical input remains capped
    pub fn from_reader(input: &'a mut dyn Read, limits: Limits) -> Result<Self> {
        Self::from_reader_charged(input, limits, 0, 0)
    }

    pub(crate) fn from_reader_charged(input: &'a mut dyn Read, limits: Limits, allocated: usize, work: usize) -> Result<Self> {
        let mut budget = Budget::new(limits)?;
        budget.allocation(allocated, 1, 0)?;
        budget.work(work, 0)?;
        let capacity = limits.input_bytes.min(8192);
        let mut buffer = budget.vector(capacity, 0)?;
        buffer.resize(capacity, 0);
        let reader = Buffered { source: LimitedRead { source: input, remaining: limits.input_bytes }, buffer, used: 0, end: 0 };
        let mut result = Self { input: Input::Stream { reader, scratch: Vec::new() }, offset: 0,
            arena: Arena::default(), budget, active_descriptors: Vec::new(), poisoned: false };
        if result.take(4)? != [0xac, 0xed, 0, 5] { return Err(error(0, ErrorKind::Header)); }
        Ok(result)
    }

    pub fn position(&self) -> usize { self.offset }
    pub(crate) fn usage(&self) -> (usize, usize) { (self.budget.allocated, self.budget.work) }
    pub(crate) fn charge_external(&mut self, allocated: usize, work: usize) -> Result<()> {
        let result = (|| {
            self.budget.allocation(allocated, 1, self.offset)?;
            self.budget.work(work, self.offset)
        })();
        if result.is_err() { self.poisoned = true; }
        result
    }

    pub fn arena(&self) -> Result<&Arena> {
        if self.poisoned { return Err(error(self.offset, ErrorKind::Poisoned)); }
        Ok(&self.arena)
    }

    pub fn into_arena(self) -> Result<Arena> {
        if self.poisoned { return Err(error(self.offset, ErrorKind::Poisoned)); }
        Ok(self.arena)
    }

    fn take(&mut self, count: usize) -> Result<&[u8]> {
        self.input.take(count, &mut self.budget, &mut self.offset)
    }

    fn byte(&mut self) -> Result<u8> { Ok(self.take(1)?[0]) }
    fn u16(&mut self) -> Result<u16> { Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap())) }
    fn u32(&mut self) -> Result<u32> { Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap())) }
    fn u64(&mut self) -> Result<u64> { Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap())) }

    fn name(&mut self) -> Result<Vec<u16>> {
        let count = usize::from(self.u16()?);
        if count > self.budget.limits.name_units.saturating_mul(3) { return Err(error(self.offset, ErrorKind::Limit(LimitKind::Name))); }
        let start = self.offset;
        let bytes = self.input.take(count, &mut self.budget, &mut self.offset)?;
        let units = decode_utf(bytes, &mut self.budget, start)?;
        if units.len() > self.budget.limits.name_units { return Err(error(start, ErrorKind::Limit(LimitKind::Name))); }
        Ok(units)
    }

    fn insert(&mut self, node: Node) -> Result<NodeId> {
        if self.arena.nodes.len() >= self.budget.limits.handles { return Err(error(self.offset, ErrorKind::Limit(LimitKind::Handles))); }
        self.budget.grow(&mut self.arena.nodes, self.budget.limits.handles, self.offset)?;
        let id = NodeId(self.arena.nodes.len() as u32);
        self.arena.nodes.push(node);
        Ok(id)
    }

    fn reference(&mut self) -> Result<NodeId> {
        let wire = self.u32()?;
        let id = wire.checked_sub(BASE_HANDLE).map(NodeId).ok_or(error(self.offset - 4, ErrorKind::InvalidReference))?;
        self.arena.node(id).map_err(|_| error(self.offset - 4, ErrorKind::InvalidReference))?;
        Ok(id)
    }

    fn descriptor(&mut self, depth: usize) -> Result<Option<NodeId>> {
        self.budget.depth(depth, self.offset)?;
        let token = self.byte()?;
        if token == 0x70 { return Ok(None); }
        if token == 0x71 {
            let id = self.reference()?;
            self.arena.descriptor(id)?;
            if self.active_descriptors.contains(&id) { return Err(error(self.offset, ErrorKind::IncompleteDescriptor)); }
            return Ok(Some(id));
        }
        if token != 0x72 { return Err(error(self.offset - 1, ErrorKind::UnsupportedToken(token))); }
        self.new_descriptor(depth).map(Some)
    }

    fn new_descriptor(&mut self, depth: usize) -> Result<NodeId> {
        let name = self.name()?;
        let uid = self.u64()? as i64;
        let class = schema::CLASSES.iter().position(|item| item.name.encode_utf16().eq(name.iter().copied())).map(ClassId).ok_or(error(self.offset, ErrorKind::UnknownClass))?;
        let info = class_schema(class);
        if uid != info.uid { return Err(error(self.offset - 8, ErrorKind::UidMismatch)); }
        if self.byte()? != info.flags { return Err(error(self.offset - 1, ErrorKind::SchemaMismatch)); }
        if usize::from(self.u16()?) != info.fields.len() { return Err(error(self.offset - 2, ErrorKind::SchemaMismatch)); }
        let id = self.insert(Node::Descriptor(ClassDescriptor { class, signatures: Vec::new(), annotations: Vec::new(), superclass: None }))?;
        self.budget.grow(&mut self.active_descriptors, self.budget.limits.depth + 1, self.offset)?;
        self.active_descriptors.push(id);
        let mut signatures = self.budget.vector(info.fields.len(), self.offset)?;
        for field in info.fields {
            if self.byte()? != field.kind || !field.name.encode_utf16().eq(self.name()?) { return Err(error(self.offset, ErrorKind::SchemaMismatch)); }
            let signature = if let Some(expected) = field.signature {
                let Value::Handle(reference) = self.value(depth + 1)? else { return Err(error(self.offset, ErrorKind::SchemaMismatch)); };
                match self.arena.node(reference)? {
                    Node::String(value) if expected.encode_utf16().eq(value.units.iter().copied()) => Some(reference),
                    _ => return Err(error(self.offset, ErrorKind::SchemaMismatch)),
                }
            } else { None };
            signatures.push(signature);
        }
        let annotations = self.annotations(depth + 1)?;
        let superclass = self.descriptor(depth + 1)?;
        let parent = superclass.map(|value| self.arena.descriptor(value).map(|item| class_schema(item.class).name)).transpose()?;
        if parent != info.superclass { return Err(error(self.offset, ErrorKind::SchemaMismatch)); }
        self.arena.nodes[id.0 as usize] = Node::Descriptor(ClassDescriptor { class, signatures, annotations, superclass });
        self.active_descriptors.pop();
        Ok(id)
    }

    fn annotations(&mut self, depth: usize) -> Result<Vec<Record>> {
        let mut records = Vec::new();
        loop {
            let token = self.input.peek(self.offset)?.ok_or(error(self.offset, ErrorKind::Truncated { needed: 1 }))?;
            if token == 0x78 { self.take(1)?; return Ok(records); }
            let record = self.record(depth)?;
            self.budget.grow(&mut records, self.budget.limits.records, self.offset)?;
            records.push(record);
        }
    }

    fn field(&mut self, field: &FieldSchema, depth: usize) -> Result<FieldValue> {
        Ok(match field.kind {
            b'Z' => FieldValue::Boolean(self.byte()?), b'B' => FieldValue::Byte(self.byte()?),
            b'C' => FieldValue::Char(self.u16()?), b'S' => FieldValue::Short(self.u16()? as i16),
            b'I' => FieldValue::Int(self.u32()? as i32), b'J' => FieldValue::Long(self.u64()? as i64),
            b'F' => FieldValue::FloatBits(self.u32()?), b'D' => FieldValue::DoubleBits(self.u64()?),
            b'L' | b'[' => {
                let value = self.value(depth)?;
                if !self.arena.accepts(value, field.signature.unwrap())? { return Err(error(self.offset, ErrorKind::ReferenceType)); }
                FieldValue::Reference(value)
            }
            _ => return Err(error(self.offset, ErrorKind::SchemaMismatch)),
        })
    }

    fn object(&mut self, depth: usize) -> Result<NodeId> {
        let descriptor = self.descriptor(depth + 1)?.ok_or(error(self.offset, ErrorKind::SchemaMismatch))?;
        let name = class_schema(self.arena.descriptor(descriptor)?.class).name;
        if name.starts_with('[') || ["java.lang.Number", "java.awt.color.ColorSpace", "org.locationtech.jts.index.strtree.AbstractNode", "org.locationtech.jts.index.strtree.AbstractSTRtree"].contains(&name) { return Err(error(self.offset, ErrorKind::SchemaMismatch)); }
        let hierarchy = self.arena.hierarchy(descriptor, &mut self.budget, self.offset)?;
        if hierarchy.iter().any(|id| self.active_descriptors.contains(id)) { return Err(error(self.offset, ErrorKind::IncompleteDescriptor)); }
        let id = self.insert(Node::Object(JavaObject { descriptor, slots: Vec::new() }))?;
        let mut slots = self.budget.vector(hierarchy.len(), self.offset)?;
        for class_descriptor in hierarchy {
            let info = class_schema(self.arena.descriptor(class_descriptor)?.class);
            let mut values = self.budget.vector(info.fields.len(), self.offset)?;
            for field in info.fields { values.push(self.field(field, depth + 1)?); }
            let custom = if info.flags & 1 != 0 { self.annotations(depth + 1)? } else { Vec::new() };
            slots.push(ClassData { descriptor: class_descriptor, values, custom });
        }
        self.arena.nodes[id.0 as usize] = Node::Object(JavaObject { descriptor, slots });
        Ok(id)
    }

    fn array(&mut self, depth: usize) -> Result<NodeId> {
        let descriptor = self.descriptor(depth + 1)?.ok_or(error(self.offset, ErrorKind::SchemaMismatch))?;
        let name = class_schema(self.arena.descriptor(descriptor)?.class).name;
        let count = self.u32()? as i32;
        let count = usize::try_from(count).map_err(|_| error(self.offset - 4, ErrorKind::InvalidLength))?;
        self.budget.elements(count, self.offset)?;
        let empty = match name {
            "[I" => ArrayData::Ints(Vec::new()), "[D" => ArrayData::Doubles(Vec::new()), "[F" => ArrayData::Floats(Vec::new()),
            "[Lbiogenesis.Gene;" | "[Ljava.awt.Color;" => ArrayData::References(Vec::new()),
            _ => return Err(error(self.offset, ErrorKind::SchemaMismatch)),
        };
        let width = match name { "[I" | "[F" => 4, "[D" => 8, _ => 1 };
        let minimum = count.checked_mul(width).ok_or(error(self.offset, ErrorKind::InvalidLength))?;
        if self.input.remaining(self.offset).is_some_and(|remaining| minimum > remaining) { return Err(error(self.offset, ErrorKind::Truncated { needed: minimum })); }
        if self.input.remaining(self.offset).is_none() && minimum > self.budget.limits.input_bytes.saturating_sub(self.offset) {
            return Err(error(self.offset, ErrorKind::Limit(LimitKind::Input)));
        }
        let id = self.insert(Node::Array(JavaArray { descriptor, data: empty }))?;
        let data = match name {
            "[I" => { let mut values = self.budget.vector(count, self.offset)?; for _ in 0..count { values.push(self.u32()? as i32); } ArrayData::Ints(values) }
            "[D" => { let mut values = self.budget.vector(count, self.offset)?; for _ in 0..count { values.push(self.u64()?); } ArrayData::Doubles(values) }
            "[F" => { let mut values = self.budget.vector(count, self.offset)?; for _ in 0..count { values.push(self.u32()?); } ArrayData::Floats(values) }
            _ => {
                let mut values = self.budget.vector(count, self.offset)?;
                let signature = if name == "[Lbiogenesis.Gene;" { "Lbiogenesis/Gene;" } else { "Ljava/awt/Color;" };
                for _ in 0..count {
                    let value = self.value(depth + 1)?;
                    if !self.arena.accepts(value, signature)? { return Err(error(self.offset, ErrorKind::ReferenceType)); }
                    values.push(value);
                }
                ArrayData::References(values)
            }
        };
        self.arena.nodes[id.0 as usize] = Node::Array(JavaArray { descriptor, data });
        Ok(id)
    }

    fn value(&mut self, depth: usize) -> Result<Value> {
        self.budget.depth(depth, self.offset)?;
        let token = self.byte()?;
        let id = match token {
            0x70 => return Ok(Value::Null), 0x71 => return self.reference().map(Value::Handle),
            0x72 => self.new_descriptor(depth)?, 0x73 => self.object(depth)?, 0x75 => self.array(depth)?,
            0x74 | 0x7c => {
                let count = if token == 0x74 { usize::from(self.u16()?) } else {
                    let value = self.u64()? as i64;
                    usize::try_from(value).map_err(|_| error(self.offset - 8, ErrorKind::InvalidLength))?
                };
                if count > self.budget.limits.string_bytes { return Err(error(self.offset, ErrorKind::Limit(LimitKind::String))); }
                let id = self.insert(Node::String(JavaString { units: Vec::new(), long: token == 0x7c }))?;
                let start = self.offset;
                let bytes = self.input.take(count, &mut self.budget, &mut self.offset)?;
                let units = decode_utf(bytes, &mut self.budget, start)?;
                self.arena.nodes[id.0 as usize] = Node::String(JavaString { units, long: token == 0x7c });
                id
            }
            _ => return Err(error(self.offset - 1, ErrorKind::UnsupportedToken(token))),
        };
        Ok(Value::Handle(id))
    }

    fn record(&mut self, depth: usize) -> Result<Record> {
        self.budget.depth(depth, self.offset)?;
        self.budget.record(self.offset)?;
        let token = self.input.peek(self.offset)?.ok_or(error(self.offset, ErrorKind::Truncated { needed: 1 }))?;
        if token == 0x77 || token == 0x7a {
            self.byte()?;
            let count = if token == 0x77 { usize::from(self.byte()?) } else { usize::try_from(self.u32()? as i32).map_err(|_| error(self.offset - 4, ErrorKind::InvalidLength))? };
            let bytes = self.input.take(count, &mut self.budget, &mut self.offset)?;
            let mut owned = self.budget.vector(count, self.offset)?;
            owned.extend_from_slice(bytes);
            return Ok(Record::Block { bytes: owned, long: token == 0x7a });
        }
        self.value(depth).map(Record::Value)
    }

    /// Read one record without requiring end-of-stream or consuming the following record
    pub fn read_record(&mut self) -> Result<Record> {
        if self.poisoned { return Err(error(self.offset, ErrorKind::Poisoned)); }
        let result = self.record(0);
        if result.is_err() { self.poisoned = true; }
        result
    }

    pub fn read_document(self) -> Result<Document> {
        self.read_document_with_usage().map(|(document, _, _)| document)
    }

    /// Consume one complete record without probing for EOF or parsing a following record
    pub(crate) fn read_one_with_usage(mut self) -> Result<(Document, usize, usize)> {
        let record = self.read_record()?;
        let mut records = Vec::new();
        self.budget.grow(&mut records, self.budget.limits.records, self.offset)?;
        records.push(record);
        Ok((Document { arena: self.arena, records, consumed: self.offset },
            self.budget.allocated, self.budget.work))
    }

    /// Return cumulative codec allocation and work for callers composing several bounded phases
    pub(crate) fn read_document_with_usage(mut self) -> Result<(Document, usize, usize)> {
        let mut records = Vec::new();
        while self.input.peek(self.offset)?.is_some() {
            let record = self.read_record()?;
            self.budget.grow(&mut records, self.budget.limits.records, self.offset)?;
            records.push(record);
        }
        Ok((Document { arena: self.arena, records, consumed: self.offset },
            self.budget.allocated, self.budget.work))
    }
}

#[cfg(test)]
#[path = "object_stream_tests.rs"]
mod tests;
