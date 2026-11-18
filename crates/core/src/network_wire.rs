// SPDX-License-Identifier: GPL-2.0-or-later

//! Data-only legacy network records; connection policy and publication belong to callers

use crate::legacy_genetics::{GeneticsStager, export_genomes};
use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::object_stream::{
    Arena, Document, ErrorKind as StreamErrorKind, Limits, NodeId, Record, StreamError,
    StreamReader, StreamWriter, Value,
};
use crate::Genome;
use std::{cell::RefCell, fmt, rc::Rc};

/// Request fields retain their signed wire values without connection-policy validation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Connect { version: i32, port: i32, netcode: i32 },
    Disconnect { netcode: i32 },
    SendCode { netcode: i32 },
    KeepAlive { netcode: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Stream(StreamErrorKind),
    UnknownRequest(i32),
    ExpectedPrimitive,
    PendingPrimitive,
    NullGeneticCode,
    WrongGeneticCode,
    InvalidGeneticCode { node: Option<NodeId>, field: Option<&'static str> },
    TrailingData,
    Budget,
    Allocation,
    Poisoned,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WireError {
    pub offset: usize,
    pub kind: ErrorKind,
}

impl fmt::Display for WireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Network wire error at byte {}: {:?}", self.offset, self.kind)
    }
}

impl std::error::Error for WireError {}

impl From<StreamError> for WireError {
    fn from(value: StreamError) -> Self {
        Self { offset: value.offset, kind: ErrorKind::Stream(value.kind) }
    }
}

type Result<T> = std::result::Result<T, WireError>;

fn error(offset: usize, kind: ErrorKind) -> WireError {
    WireError { offset, kind }
}

fn graph_error(value: GraphError) -> WireError {
    error(0, match value.kind {
        GraphErrorKind::Budget => ErrorKind::Budget,
        GraphErrorKind::Allocation => ErrorKind::Allocation,
        _ => ErrorKind::InvalidGeneticCode { node: value.node, field: value.field },
    })
}

/// One detached GeneticCode root, with its original wire graph and ownership identities
pub struct GeneticCodePayload {
    document: Document,
    limits: Limits,
}

impl GeneticCodePayload {
    pub fn from_document(document: Document, limits: Limits) -> Result<Self> {
        let root = match document.records.as_slice() {
            [Record::Value(Value::Null)] => return Err(error(0, ErrorKind::NullGeneticCode)),
            [Record::Value(Value::Handle(root))] => *root,
            _ => return Err(error(0, ErrorKind::TrailingData)),
        };
        let graph = LegacyGraph::new(&document, limits).map_err(graph_error)?;
        graph.expect_class(root, "biogenesis.GeneticCode").map_err(|value| {
            if matches!(value.kind, GraphErrorKind::Class | GraphErrorKind::Type) {
                error(0, ErrorKind::WrongGeneticCode)
            } else {
                graph_error(value)
            }
        })?;
        let mut limits = limits;
        limits.work = graph.remaining_work();
        limits.allocated_bytes = graph.remaining_allocation();
        Ok(Self { document, limits })
    }

    /// Assemble a detached graph with the existing bounded genetics exporter
    pub fn from_genome(genome: &Rc<RefCell<Genome>>, limits: Limits) -> Result<Self> {
        let document = export_genomes(&[Some(genome)], limits, limits.allocated_bytes)
            .map_err(graph_error)?;
        Self::from_document(document, limits)
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn root(&self) -> Value {
        let Record::Value(root) = self.document.records[0] else { unreachable!() };
        root
    }

    /// Hydrate detached owners without constructing a body, World, or random generator
    /// A reader-created payload retains only the allocation/work left after parsing
    pub fn into_genome(self) -> Result<Rc<RefCell<Genome>>> {
        self.hydrate()
    }

    /// Validate typed ownership within the remaining parse budget without retaining thread-local Rc owners
    /// Later consumer hydration is a separate operation under the same retained decode allowance
    pub(crate) fn validate(&self) -> Result<()> {
        self.hydrate().map(drop)
    }

    fn hydrate(&self) -> Result<Rc<RefCell<Genome>>> {
        let graph = LegacyGraph::new(&self.document, self.limits).map_err(graph_error)?;
        let mut stager = GeneticsStager::new(&graph, self.limits.allocated_bytes)
            .map_err(graph_error)?;
        stager.genome(self.root()).map_err(graph_error)?
            .ok_or(error(0, ErrorKind::NullGeneticCode))
    }
}

/// Cursor over one ObjectStream; errors poison it and payload extraction ends it
pub struct Reader<'a> {
    stream: Option<StreamReader<'a>>,
    block: Vec<u8>,
    used: usize,
    end: usize,
    input_length: Option<usize>,
    limits: Limits,
    poisoned: bool,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a [u8], limits: Limits) -> Result<Self> {
        let stream = StreamReader::new(input, limits)?;
        Ok(Self { stream: Some(stream), block: Vec::new(), used: 0, end: 4,
            input_length: Some(input.len()), limits, poisoned: false })
    }

    /// Read one command from an incremental source without requiring its EOF
    pub fn from_reader(input: &'a mut dyn std::io::Read, limits: Limits) -> Result<Self> {
        Self::from_reader_charged(input, limits, 0, 0)
    }

    pub(crate) fn from_reader_charged(input: &'a mut dyn std::io::Read, limits: Limits, allocated: usize, work: usize) -> Result<Self> {
        let stream = StreamReader::from_reader_charged(input, limits, allocated, work)?;
        Ok(Self { stream: Some(stream), block: Vec::new(), used: 0, end: 4,
            input_length: None, limits, poisoned: false })
    }

    pub(crate) fn usage(&self) -> (usize, usize) { self.stream.as_ref().unwrap().usage() }
    pub(crate) fn charge_external(&mut self, allocated: usize, work: usize) -> Result<()> {
        self.ready()?;
        let result = self.stream.as_mut().unwrap().charge_external(allocated, work).map_err(Into::into);
        if result.is_err() { self.poisoned = true; }
        result
    }

    /// Logical byte position, including record framing but excluding buffered primitive bytes
    pub fn position(&self) -> usize {
        self.stream.as_ref().map_or(self.end, StreamReader::position)
            - self.remaining_primitive_bytes()
    }

    pub fn remaining_primitive_bytes(&self) -> usize {
        self.block.len() - self.used
    }

    fn ready(&self) -> Result<()> {
        if self.poisoned { return Err(error(self.position(), ErrorKind::Poisoned)); }
        if self.stream.is_none() { return Err(error(self.position(), ErrorKind::Finished)); }
        Ok(())
    }

    fn primitive(&mut self) -> Result<i32> {
        self.ready()?;
        let mut bytes = [0; 4];
        for byte in &mut bytes {
            while self.used == self.block.len() {
                let record = self.stream.as_mut().unwrap().read_record()?;
                let Record::Block { bytes, .. } = record else {
                    return Err(error(self.position(), ErrorKind::ExpectedPrimitive));
                };
                self.block = bytes;
                self.used = 0;
            }
            *byte = self.block[self.used];
            self.used += 1;
        }
        Ok(i32::from_be_bytes(bytes))
    }

    /// Read an arbitrary reply integer, retaining unused block bytes for the next call
    pub fn read_i32(&mut self) -> Result<i32> {
        let result = self.primitive();
        if result.is_err() { self.poisoned = true; }
        result
    }

    /// Stop after the request fields, before any GeneticCode object record
    pub fn read_request(&mut self) -> Result<Request> {
        let result = (|| Ok(match self.read_i32()? {
            1 => Request::Connect { version: self.read_i32()?, port: self.read_i32()?, netcode: self.read_i32()? },
            5 => Request::Disconnect { netcode: self.read_i32()? },
            8 => Request::SendCode { netcode: self.read_i32()? },
            13 => Request::KeepAlive { netcode: self.read_i32()? },
            kind => return Err(error(self.position(), ErrorKind::UnknownRequest(kind))),
        }))();
        if result.is_err() { self.poisoned = true; }
        result
    }

    /// Consume the remaining single object root; no acknowledgement or publication occurs
    pub fn read_genetic_code(&mut self) -> Result<GeneticCodePayload> {
        let result = (|| {
            self.ready()?;
            if self.remaining_primitive_bytes() != 0 {
                return Err(error(self.position(), ErrorKind::PendingPrimitive));
            }
            let stream = self.stream.take().unwrap();
            self.end = stream.position();
            let (document, allocated, work) = if self.input_length.is_some() {
                stream.read_document_with_usage()?
            } else { stream.read_one_with_usage()? };
            self.end = document.consumed;
            let mut limits = self.limits;
            limits.allocated_bytes = limits.allocated_bytes.checked_sub(allocated)
                .ok_or(error(self.end, ErrorKind::Budget))?;
            limits.work = limits.work.checked_sub(work)
                .ok_or(error(self.end, ErrorKind::Budget))?;
            GeneticCodePayload::from_document(document, limits)
        })();
        if result.is_err() { self.poisoned = true; }
        result
    }

    pub fn finish(&mut self) -> Result<()> {
        let result = (|| {
            self.ready()?;
            if self.remaining_primitive_bytes() != 0 || self.input_length.is_some_and(|length| self.position() != length) {
                return Err(error(self.position(), ErrorKind::TrailingData));
            }
            self.end = self.position();
            self.stream = None;
            Ok(())
        })();
        if result.is_err() { self.poisoned = true; }
        result
    }
}

/// Writer for one caller-owned arena and ObjectStream handle-table lifetime
pub struct Writer<'a> {
    stream: StreamWriter<'a>,
    arena: &'a Arena,
    poisoned: bool,
}

impl<'a> Writer<'a> {
    pub fn new(arena: &'a Arena, limits: Limits) -> Result<Self> {
        Ok(Self { stream: StreamWriter::new(arena, limits)?, arena, poisoned: false })
    }

    pub(crate) fn new_charged(arena: &'a Arena, limits: Limits, allocated: usize, work: usize) -> Result<Self> {
        Ok(Self { stream: StreamWriter::new_charged(arena, limits, allocated, work)?, arena, poisoned: false })
    }

    /// Bind a delayed payload after primitive headers without restarting the ObjectStream or its budget
    pub(crate) fn bind_empty_arena<'b>(self, arena: &'b Arena) -> Result<Writer<'b>> {
        if self.poisoned { return Err(error(self.position(), ErrorKind::Poisoned)); }
        Ok(Writer { stream: self.stream.bind_empty_arena(arena)?, arena, poisoned: false })
    }

    pub(crate) fn output(&self) -> &[u8] { self.stream.output() }
    pub(crate) fn usage(&self) -> (usize, usize) { self.stream.usage() }
    pub(crate) fn charge_external(&mut self, allocated: usize, work: usize) -> Result<()> {
        let result = self.stream.charge_external(allocated, work).map_err(Into::into);
        if result.is_err() { self.poisoned = true; }
        result
    }

    pub fn position(&self) -> usize {
        self.stream.position()
    }

    /// Each call emits one primitive block, matching an original flush boundary
    pub fn write_i32s(&mut self, values: &[i32]) -> Result<()> {
        if self.poisoned { return Err(error(self.position(), ErrorKind::Poisoned)); }
        let result = self.stream.write_i32_block(values).map_err(Into::into);
        if result.is_err() { self.poisoned = true; }
        result
    }

    pub fn write_i32(&mut self, value: i32) -> Result<()> {
        self.write_i32s(&[value])
    }

    pub fn write_request(&mut self, request: Request) -> Result<()> {
        match request {
            Request::Connect { version, port, netcode } => self.write_i32s(&[1, version, port, netcode]),
            Request::Disconnect { netcode } => self.write_i32s(&[5, netcode]),
            Request::SendCode { netcode } => self.write_i32s(&[8, netcode]),
            Request::KeepAlive { netcode } => self.write_i32s(&[13, netcode]),
        }
    }

    /// Write a GeneticCode root from this writer's arena after the caller's handshake decision
    pub fn write_genetic_code(&mut self, root: Value) -> Result<()> {
        let result = (|| {
            if self.poisoned { return Err(error(self.position(), ErrorKind::Poisoned)); }
            let Value::Handle(id) = root else {
                return Err(error(self.position(), ErrorKind::NullGeneticCode));
            };
            let crate::object_stream::Node::Object(object) = self.arena.node(id)? else {
                return Err(error(self.position(), ErrorKind::WrongGeneticCode));
            };
            let descriptor = self.arena.descriptor(object.descriptor)?;
            if crate::object_stream::class_schema(descriptor.class).name != "biogenesis.GeneticCode" {
                return Err(error(self.position(), ErrorKind::WrongGeneticCode));
            }
            self.stream.write_record(&Record::Value(root))?;
            Ok(())
        })();
        if result.is_err() { self.poisoned = true; }
        result
    }

    pub fn finish(self) -> Result<Vec<u8>> {
        if self.poisoned { return Err(error(self.position(), ErrorKind::Poisoned)); }
        self.stream.finish().map_err(Into::into)
    }
}

#[cfg(test)]
#[path = "network_wire_tests.rs"]
mod tests;
