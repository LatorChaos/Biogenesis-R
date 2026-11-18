// SPDX-License-Identifier: GPL-2.0-or-later

//! Typed export of complete World owners and their shared persistent closure

use super::World;
use crate::history::{HistoryListRef, JavaDoubleRef};
use crate::shared_world_owners::{SharedListRef, SharedSlot};
use crate::legacy_graph::{GraphError, GraphErrorKind};
use crate::object_stream::*;
use crate::organism::{GenomeRef, Organism, OrganismRef};
use crate::shared_world_owners::{BoundableRef, BoundableValue, EnvelopeRef, TreeRef};
use crate::statistics::WorldStatistics;
use crate::{ColorArrayRef, GeneArrayRef, GeneRef, JavaColor, JavaFloatArray, JavaIccColorSpace,
    JavaIccProfile, JavaText, RectangleRef};
use std::{cell::RefCell, collections::HashMap, mem::size_of, rc::Rc};

const WORLD: &str = "biogenesis.World";
const ACTOR: &str = "biogenesis.Organism";
const RECTANGLE: &str = "java.awt.Rectangle";
const LIST: &str = "java.util.ArrayList";
const TREE: &str = "org.locationtech.jts.index.strtree.STRtree";
const TREE_BASE: &str = "org.locationtech.jts.index.strtree.AbstractSTRtree";
const NODE: &str = "org.locationtech.jts.index.strtree.STRtree$STRtreeNode";
const NODE_BASE: &str = "org.locationtech.jts.index.strtree.AbstractNode";
const ITEM: &str = "org.locationtech.jts.index.strtree.ItemBoundable";
const ENVELOPE: &str = "org.locationtech.jts.geom.Envelope";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnsupportedExportShape {
    ActorWorld,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorldExportError {
    Graph(GraphError),
    Stream(StreamError),
    Unsupported { field: &'static str, shape: UnsupportedExportShape },
}
impl From<GraphError> for WorldExportError {
    fn from(value: GraphError) -> Self { Self::Graph(value) }
}
impl From<StreamError> for WorldExportError {
    fn from(value: StreamError) -> Self { Self::Stream(value) }
}
pub(crate) type ExportResult<T> = Result<T, WorldExportError>;
fn failure(kind: GraphErrorKind) -> WorldExportError {
    GraphError { node: None, field: None, kind }.into()
}
fn product(a: usize, b: usize) -> ExportResult<usize> {
    a.checked_mul(b).ok_or(failure(GraphErrorKind::Budget))
}

// Allocation family and local index separate equal addresses from unrelated owners
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Key(u8, usize, usize);

struct Usage { allocated: usize, work: usize, elements: usize }
impl Usage {
    fn charge(&mut self, bytes: usize, limits: Limits) -> ExportResult<()> {
        self.allocated = self.allocated.checked_add(bytes).ok_or(failure(GraphErrorKind::Budget))?;
        if self.allocated > limits.allocated_bytes { return Err(failure(GraphErrorKind::Budget)); }
        Ok(())
    }
    fn visit(&mut self, count: usize, limits: Limits) -> ExportResult<()> {
        self.work = self.work.checked_add(count).ok_or(failure(GraphErrorKind::Budget))?;
        if self.work > limits.work { return Err(failure(GraphErrorKind::Budget)); }
        Ok(())
    }
    fn reserve<T>(&mut self, values: &mut Vec<T>, additional: usize, limits: Limits) -> ExportResult<()> {
        let needed = values.len().checked_add(additional).ok_or(failure(GraphErrorKind::Budget))?;
        if needed <= values.capacity() { return Ok(()); }
        let before = values.capacity();
        self.charge(product(needed - before, size_of::<T>())?, limits)?;
        values.try_reserve_exact(additional).map_err(|_| failure(GraphErrorKind::Allocation))?;
        self.charge(product(values.capacity() - needed, size_of::<T>())?, limits)
    }
}

/// One context owns all typed identities and all assembly accounting
pub(crate) struct WorldEmitter {
    document: Document,
    owners: HashMap<Key, NodeId>,
    descriptors: HashMap<&'static str, NodeId>,
    signatures: HashMap<&'static str, NodeId>,
    limits: Limits,
    usage: Usage,
    depth: usize,
}
impl WorldEmitter {
    fn new(mut limits: Limits, maximum: usize) -> Self {
        limits.allocated_bytes = limits.allocated_bytes.min(maximum);
        Self { document: Document { arena: Arena::default(), records: Vec::new(), consumed: 0 }, owners: HashMap::new(), descriptors: HashMap::new(),
            signatures: HashMap::new(), limits, usage: Usage { allocated: 0, work: 0, elements: 0 },
            depth: 0 }
    }
    fn vector<T>(&mut self, count: usize) -> ExportResult<Vec<T>> {
        let mut result = Vec::new();
        self.usage.reserve(&mut result, count, self.limits)?;
        Ok(result)
    }
    fn visit(&mut self, count: usize) -> ExportResult<()> { self.usage.visit(count, self.limits) }
    fn push(&mut self, value: Node) -> ExportResult<NodeId> {
        if self.document.arena.nodes.len() >= self.limits.handles { return Err(failure(GraphErrorKind::Budget)); }
        let id = NodeId(u32::try_from(self.document.arena.nodes.len()).map_err(|_| failure(GraphErrorKind::Budget))?);
        if self.document.arena.nodes.len() == self.document.arena.nodes.capacity() {
            let additional = self.document.arena.nodes.capacity().max(8)
                .min(self.limits.handles - self.document.arena.nodes.len());
            self.usage.reserve(&mut self.document.arena.nodes, additional, self.limits)?;
        }
        self.document.arena.nodes.push(value);
        Ok(id)
    }
    fn map_capacity<K: Eq + std::hash::Hash, V>(
        map: &mut HashMap<K, V>, usage: &mut Usage, limits: Limits,
    ) -> ExportResult<()> {
        if map.len() < map.capacity() { return Ok(()); }
        let old = map.capacity();
        let count = old.max(4);
        // HashMap control bytes and load-factor slack are charged conservatively before reserve
        usage.charge(product(product(count.checked_add(1).ok_or(failure(GraphErrorKind::Budget))?, size_of::<(K, V)>() + 16)?, 2)?, limits)?;
        map.try_reserve(count).map_err(|_| failure(GraphErrorKind::Allocation))?;
        Ok(())
    }
    fn reserve_owner(&mut self, key: Key) -> ExportResult<(NodeId, bool)> {
        self.visit(1)?;
        if let Some(id) = self.owners.get(&key) { return Ok((*id, false)); }
        Self::map_capacity(&mut self.owners, &mut self.usage, self.limits)?;
        let id = self.push(Node::String(JavaString { units: Vec::new(), long: false }))?;
        self.owners.insert(key, id);
        Ok((id, true))
    }
    fn enter(&mut self) -> ExportResult<()> {
        self.depth += 1;
        if self.depth > self.limits.depth || self.depth > 128 { return Err(failure(GraphErrorKind::Budget)); }
        Ok(())
    }
    fn text_units(&mut self, units: &[u16]) -> ExportResult<NodeId> {
        self.visit(units.len())?;
        let bytes = units.iter().try_fold(0usize, |n, unit| n.checked_add(
            if (1..=127).contains(unit) { 1 } else if *unit <= 0x7ff { 2 } else { 3 }
        ).ok_or(failure(GraphErrorKind::Budget)))?;
        if units.len() > self.limits.string_units || bytes > self.limits.string_bytes {
            return Err(failure(GraphErrorKind::Budget));
        }
        let mut copy = self.vector(units.len())?;
        copy.extend_from_slice(units);
        self.push(Node::String(JavaString { units: copy, long: bytes > u16::MAX as usize }))
    }
    fn signature(&mut self, text: &'static str) -> ExportResult<NodeId> {
        self.visit(1)?;
        if let Some(id) = self.signatures.get(text) { return Ok(*id); }
        Self::map_capacity(&mut self.signatures, &mut self.usage, self.limits)?;
        let mut units = self.vector(text.len())?;
        units.extend(text.encode_utf16());
        let id = self.text_units(&units)?;
        self.signatures.insert(text, id);
        Ok(id)
    }
    fn descriptor(&mut self, name: &'static str) -> ExportResult<NodeId> {
        self.visit(1)?;
        if let Some(id) = self.descriptors.get(name) { return Ok(*id); }
        let class = class_by_name(name).ok_or(failure(GraphErrorKind::Class))?;
        let schema = class_schema(class);
        if name.len() > self.limits.name_units || schema.fields.iter().any(|f| f.name.len() > self.limits.name_units) {
            return Err(failure(GraphErrorKind::Budget));
        }
        let parent = schema.superclass.map(|parent| self.descriptor(parent)).transpose()?;
        let mut signatures = self.vector(schema.fields.len())?;
        for field in schema.fields {
            self.visit(1)?;
            signatures.push(field.signature.map(|text| self.signature(text)).transpose()?);
        }
        Self::map_capacity(&mut self.descriptors, &mut self.usage, self.limits)?;
        let id = self.push(Node::Descriptor(ClassDescriptor { class, signatures,
            annotations: Vec::new(), superclass: parent }))?;
        self.descriptors.insert(name, id);
        Ok(id)
    }
    fn object<F>(&mut self, id: NodeId, name: &'static str, mut field: F) -> ExportResult<()>
    where F: FnMut(&mut Self, &'static str, &'static str) -> ExportResult<FieldValue> {
        self.enter()?;
        let descriptor = self.descriptor(name)?;
        let mut hierarchy = self.vector(4)?;
        let mut current = Some(descriptor);
        while let Some(id) = current {
            self.visit(1)?;
            self.usage.reserve(&mut hierarchy, 1, self.limits)?;
            hierarchy.push(id);
            let Node::Descriptor(value) = &self.document.arena.nodes[id.0 as usize] else { unreachable!() };
            current = value.superclass;
        }
        let mut slots = self.vector(hierarchy.len())?;
        for descriptor in hierarchy.into_iter().rev() {
            let Node::Descriptor(value) = &self.document.arena.nodes[descriptor.0 as usize] else { unreachable!() };
            let schema = class_schema(value.class);
            let mut values = self.vector(schema.fields.len())?;
            for f in schema.fields { self.visit(1)?; values.push(field(self, schema.name, f.name)?); }
            slots.push(ClassData { descriptor, values, custom: Vec::new() });
        }
        self.document.arena.nodes[id.0 as usize] = Node::Object(JavaObject { descriptor, slots });
        self.depth -= 1;
        Ok(())
    }
    fn elements(&mut self, count: usize) -> ExportResult<()> {
        self.usage.elements = self.usage.elements.checked_add(count).ok_or(failure(GraphErrorKind::Budget))?;
        if count > i32::MAX as usize || count > self.limits.array_elements || self.usage.elements > self.limits.total_array_elements {
            return Err(failure(GraphErrorKind::Budget));
        }
        self.visit(count)
    }
    fn array(&mut self, id: NodeId, name: &'static str, data: ArrayData) -> ExportResult<()> {
        let descriptor = self.descriptor(name)?;
        self.document.arena.nodes[id.0 as usize] = Node::Array(JavaArray { descriptor, data });
        Ok(())
    }
    fn list<F>(&mut self, key: Key, count: usize, mut element: F) -> ExportResult<Value>
    where F: FnMut(&mut Self, usize) -> ExportResult<Value> {
        let (id, first) = self.reserve_owner(key)?;
        if !first { return Ok(Value::Handle(id)); }
        self.enter()?;
        self.elements(count)?;
        let descriptor = self.descriptor(LIST)?;
        let mut custom = self.vector(count.checked_add(1).ok_or(failure(GraphErrorKind::Budget))?)?;
        let mut bytes = self.vector(4)?;
        bytes.extend_from_slice(&(count as i32).to_be_bytes());
        custom.push(Record::Block { bytes, long: false });
        for index in 0..count { custom.push(Record::Value(element(self, index)?)); }
        let mut values = self.vector(1)?; values.push(FieldValue::Int(count as i32));
        let mut slots = self.vector(1)?; slots.push(ClassData { descriptor, values, custom });
        self.document.arena.nodes[id.0 as usize] = Node::Object(JavaObject { descriptor, slots });
        self.depth -= 1;
        Ok(Value::Handle(id))
    }
    fn world(&mut self, world: &World) -> ExportResult<Value> {
        let (id, first) = self.reserve_owner(Key(b'W', world.association().identity().ok_or(failure(GraphErrorKind::Reference))?, 0))?;
        if !first { return Ok(Value::Handle(id)); }
        self.object(id, WORLD, |this, _, name| Ok(match name {
            "_CO2" => FieldValue::DoubleBits(world.carbon_dioxide.get().to_bits()),
            "_CH4" => FieldValue::DoubleBits(world.methane.get().to_bits()),
            "_O2" => FieldValue::DoubleBits(world.oxygen.get().to_bits()),
            "_width" => FieldValue::Int(world.config.get().width),
            "_height" => FieldValue::Int(world.config.get().height),
            "_population" => FieldValue::Int(world.population.get()),
            "nFrames" => FieldValue::Int(world.frame.get()),
            "NEXT_ID" => FieldValue::Int(world.next_id.get()),
            "_corridorexists" => FieldValue::Boolean(u8::from(world.corridors_exist.get())),
            "_organisms" => FieldValue::Reference(this.members(world)?),
            "colDetTree" => FieldValue::Reference(this.tree(world)?),
            "worldStatistics" => FieldValue::Reference(this.statistics(&world.statistics_ref())?),
            _ => return Err(failure(GraphErrorKind::Field)),
        }))?;
        Ok(Value::Handle(id))
    }
    fn members(&mut self, world: &World) -> ExportResult<Value> {
        self.shared_list(world.organisms.shared_ref())
    }
    fn shared_list(&mut self, owner: Option<&SharedListRef>) -> ExportResult<Value> {
        let Some(owner) = owner else { return Ok(Value::Null); };
        if owner.wire_class_name() == LIST {
            let values = owner.slots();
            return self.list(Key(b'Q', owner.backing_identity(), 0), values.len(), |this, index| match &values[index] {
                SharedSlot::Null => Ok(Value::Null),
                SharedSlot::Double(value) => this.boxed(value),
                SharedSlot::Actor(value) => this.actor(value),
                SharedSlot::Boundable(value) => this.boundable(value),
                SharedSlot::Envelope(value) => this.envelope(value),
            });
        }
        let (id, first) = self.reserve_owner(Key(b'Y', owner.identity(), 0))?;
        if first {
            let backing = owner.backing_ref();
            self.object(id, "java.util.Collections$SynchronizedList", |this, _, name| {
                Ok(FieldValue::Reference(match name {
                    "mutex" => Value::Handle(id),
                    "c" | "list" => this.shared_list(Some(&backing))?,
                    _ => return Err(failure(GraphErrorKind::Field)),
                }))
            })?;
        }
        Ok(Value::Handle(id))
    }
    fn statistics(&mut self, owner: &Rc<RefCell<WorldStatistics>>) -> ExportResult<Value> {
        let (id, first) = self.reserve_owner(Key(b'S', Rc::as_ptr(owner) as usize, 0))?;
        if first {
            let value = owner.borrow();
            self.object(id, "biogenesis.WorldStatistics", |this, _, field| value.legacy_export_field(this, field))?;
        }
        Ok(Value::Handle(id))
    }
    pub(crate) fn history(&mut self, owner: Option<&HistoryListRef>) -> ExportResult<Value> {
        self.shared_list(owner.map(HistoryListRef::shared_ref))
    }
    fn boxed(&mut self, value: &JavaDoubleRef) -> ExportResult<Value> {
        let (id, first) = self.reserve_owner(Key(b'B', value.identity(), 0))?;
        if first { self.object(id, "java.lang.Double", |_, _, field| match field {
            "value" => Ok(FieldValue::DoubleBits(value.bits())), _ => Err(failure(GraphErrorKind::Field)),
        })?; }
        Ok(Value::Handle(id))
    }
    pub(crate) fn genome(&mut self, value: Option<&GenomeRef>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let (id, first) = self.reserve_owner(Key(b'G', Rc::as_ptr(value) as usize, 0))?;
        if first {
            let value = value.borrow();
            self.object(id, "biogenesis.GeneticCode", |this, _, name| Ok(match name {
                "_genes" => FieldValue::Reference(this.genes(value.gene_array())?),
                "_cladeID" => FieldValue::Reference(this.text(value.clade_id())?),
                name => crate::legacy_genetics::code_scalar(&value, name).ok_or(failure(GraphErrorKind::Field))?,
            }))?;
        }
        Ok(Value::Handle(id))
    }
    fn genes(&mut self, value: Option<&GeneArrayRef>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let (id, first) = self.reserve_owner(Key(b'A', value.identity(), 0))?;
        if first {
            self.enter()?;
            self.elements(value.len())?;
            let mut references = self.vector(value.len())?;
            for value in value.slots().iter() { references.push(self.gene(value.as_ref())?); }
            self.array(id, "[Lbiogenesis.Gene;", ArrayData::References(references))?;
            self.depth -= 1;
        }
        Ok(Value::Handle(id))
    }
    fn gene(&mut self, value: Option<&GeneRef>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let (id, first) = self.reserve_owner(Key(b'g', Rc::as_ptr(value) as usize, 0))?;
        if first {
            let value = value.borrow();
            self.object(id, "biogenesis.Gene", |this, _, name| Ok(match name {
                "_length" => FieldValue::DoubleBits(value.length().to_bits()),
                "_theta" => FieldValue::DoubleBits(value.theta().to_bits()),
                "_branch" => FieldValue::Int(value.branch()),
                "_color" => FieldValue::Reference(this.color(value.color_handle())?),
                name => FieldValue::Int(value.reaction(crate::legacy_genetics::reaction_index(name)
                    .ok_or(failure(GraphErrorKind::Field))?)),
            }))?;
        }
        Ok(Value::Handle(id))
    }
    fn text(&mut self, value: Option<&JavaText>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let key = Key(b'T', value.identity(), 0);
        self.visit(1)?;
        if let Some(id) = self.owners.get(&key) { return Ok(Value::Handle(*id)); }
        Self::map_capacity(&mut self.owners, &mut self.usage, self.limits)?;
        let id = self.text_units(value.units())?;
        self.owners.insert(key, id);
        Ok(Value::Handle(id))
    }
    fn rectangle(&mut self, value: Option<&RectangleRef>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        if value.is_actor() {
            let actor = value.actor_target().ok_or(failure(GraphErrorKind::Reference))?;
            return self.actor(&actor);
        }
        let (id, first) = self.reserve_owner(Key(b'R', value.identity(), 0))?;
        if first {
            let value = value.get();
            self.object(id, RECTANGLE, |_, _, field| Ok(FieldValue::Int(match field {
                "x" => value.x, "y" => value.y, "width" => value.width, "height" => value.height,
                _ => return Err(failure(GraphErrorKind::Field)),
            })))?;
        }
        Ok(Value::Handle(id))
    }
    fn actor(&mut self, owner: &OrganismRef) -> ExportResult<Value> {
        let (id, first) = self.reserve_owner(Key(b'O', Rc::as_ptr(owner) as usize, 0))?;
        if first {
            let value = owner.borrow();
            if !value.state.bounds.actor_target().is_some_and(|target| Rc::ptr_eq(&target, owner)) {
                return Err(failure(GraphErrorKind::Reference));
            }
            self.object(id, ACTOR, |this, class, field| this.actor_field(&value, class, field))?;
        }
        Ok(Value::Handle(id))
    }
    fn actor_field(&mut self, value: &Organism, class: &'static str, field: &'static str) -> ExportResult<FieldValue> {
        if class == RECTANGLE {
            let bounds = value.state.bounds.get();
            return Ok(FieldValue::Int(match field {
                "x" => bounds.x, "y" => bounds.y, "width" => bounds.width, "height" => bounds.height,
                _ => return Err(failure(GraphErrorKind::Field)),
            }));
        }
        Ok(match field {
            "_startPointX" => FieldValue::Reference(self.ints(value.body.arrays.start_x.as_ref())?),
            "_startPointY" => FieldValue::Reference(self.ints(value.body.arrays.start_y.as_ref())?),
            "_endPointX" => FieldValue::Reference(self.ints(value.body.arrays.end_x.as_ref())?),
            "_endPointY" => FieldValue::Reference(self.ints(value.body.arrays.end_y.as_ref())?),
            "x1" => FieldValue::Reference(self.ints(value.body.arrays.x1.as_ref())?),
            "x2" => FieldValue::Reference(self.ints(value.body.arrays.x2.as_ref())?),
            "y1" => FieldValue::Reference(self.ints(value.body.arrays.y1.as_ref())?),
            "y2" => FieldValue::Reference(self.ints(value.body.arrays.y2.as_ref())?),
            "_segredReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[0].as_ref())?),
            "_seggreenReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[1].as_ref())?),
            "_segblueReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[2].as_ref())?),
            "_segplagueReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[3].as_ref())?),
            "_segscourgeReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[4].as_ref())?),
            "_segwhiteReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[5].as_ref())?),
            "_seggrayReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[6].as_ref())?),
            "_segsilverReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[7].as_ref())?),
            "_segdefaultReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[8].as_ref())?),
            "_segconsumerReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[9].as_ref())?),
            "_segplantReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[10].as_ref())?),
            "_segmagentaReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[11].as_ref())?),
            "_segpinkReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[12].as_ref())?),
            "_segcoralReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[13].as_ref())?),
            "_segorangeReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[14].as_ref())?),
            "_segbarkReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[15].as_ref())?),
            "_segvioletReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[16].as_ref())?),
            "_segvirusReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[17].as_ref())?),
            "_segmaroonReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[18].as_ref())?),
            "_segoliveReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[19].as_ref())?),
            "_segmintReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[20].as_ref())?),
            "_segcreamReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[21].as_ref())?),
            "_segspikeReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[22].as_ref())?),
            "_segfallowReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[23].as_ref())?),
            "_seglightblueReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[24].as_ref())?),
            "_segochreReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[25].as_ref())?),
            "_segskyReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[26].as_ref())?),
            "_seglilacReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[27].as_ref())?),
            "_segfireReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[28].as_ref())?),
            "_seglightbrownReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[29].as_ref())?),
            "_seggreenbrownReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[30].as_ref())?),
            "_segbrownReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[31].as_ref())?),
            "_segiceReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[32].as_ref())?),
            "_segbrokenReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[33].as_ref())?),
            "_segsickReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[34].as_ref())?),
            "_segfriendReaction" => FieldValue::Reference(self.ints(value.body.arrays.reactions[35].as_ref())?),
            "_m" => FieldValue::Reference(self.doubles(value.body.arrays.length.as_ref())?),
            "_m1" => FieldValue::Reference(self.doubles(value.body.arrays.start_radius.as_ref())?),
            "_m2" => FieldValue::Reference(self.doubles(value.body.arrays.end_radius.as_ref())?),
            "_mphoto" => FieldValue::Reference(self.doubles(value.body.arrays.photo.as_ref())?),
            "_I" => FieldValue::DoubleBits(value.body.inertia.to_bits()),
            "_ID" => FieldValue::Int(i32::try_from(value.id).map_err(|_| failure(GraphErrorKind::Field))?),
            "_age" => FieldValue::Int(value.state.age),
            "_allfrozen" => FieldValue::Boolean(u8::from(value.effects.all_frozen)),
            "_altruist" => FieldValue::Boolean(u8::from(value.effects.altruist)),
            "_antiviral" => FieldValue::Int(value.effects.antiviral),
            "_candodge" => FieldValue::Boolean(u8::from(value.effects.can_dodge)),
            "_canmove" => FieldValue::Boolean(u8::from(value.effects.can_move)),
            "_canreact" => FieldValue::Boolean(u8::from(value.effects.can_react)),
            "_centerX" => FieldValue::Int(value.state.integer_center[0]),
            "_centerY" => FieldValue::Int(value.state.integer_center[1]),
            "_clockwise" => FieldValue::Boolean(u8::from(value.effects.clockwise)),
            "_colonyPhotosynthesis" => FieldValue::DoubleBits(value.effects.colony_photosynthesis.to_bits()),
            "_color" => FieldValue::Reference(self.color(value.state.temporary_color.as_ref())?),
            "_createlavender" => FieldValue::Int(value.effects.create_lavender),
            "_crowded" => FieldValue::Boolean(u8::from(value.effects.crowded)),
            "_dCenterX" => FieldValue::DoubleBits(value.state.center[0].to_bits()),
            "_dCenterY" => FieldValue::DoubleBits(value.state.center[1].to_bits()),
            "_dodge" => FieldValue::Boolean(u8::from(value.effects.dodge)),
            "_earlyReproduceEnergy" => FieldValue::Int(value.state.reproduction.early_energy),
            "_energy" => FieldValue::DoubleBits(value.state.energy.to_bits()),
            "_fallowversion" => FieldValue::Int(value.effects.fallow_version),
            "_forestphoto" => FieldValue::DoubleBits(value.effects.forest_photo.to_bits()),
            "_framesColor" => FieldValue::Int(value.state.temporary_color_frames),
            "_generationbattle" => FieldValue::Boolean(u8::from(value.effects.generation_battle)),
            "_geneticCode" => FieldValue::Reference(self.genome(value.genome.as_ref())?),
            "_gold" => FieldValue::Int(value.effects.gold),
            "_growthRatio" => FieldValue::Int(value.body.growth_ratio),
            "_hasdodged" => FieldValue::Boolean(u8::from(value.effects.has_dodged)),
            "_haseyes" => FieldValue::Boolean(u8::from(value.body.eyes.enabled)),
            "_hasgoodvision" => FieldValue::Boolean(u8::from(value.body.eyes.good_vision)),
            "_healing" => FieldValue::Int(value.effects.healing),
            "_indigo" => FieldValue::Int(value.effects.indigo),
            "_infectedGeneticCode" => FieldValue::Reference(self.genome(value.infected_genome.as_ref())?),
            "_infectedReproduceEnergy" => FieldValue::Int(value.state.reproduction.infected_energy),
            "_isaconsumer" => FieldValue::Boolean(u8::from(value.effects.is_consumer)),
            "_isafungus" => FieldValue::Boolean(u8::from(value.effects.is_fungus)),
            "_isakiller" => FieldValue::Boolean(u8::from(value.effects.is_killer)),
            "_isaplant" => FieldValue::Boolean(u8::from(value.effects.is_plant)),
            "_isauburn" => FieldValue::Boolean(u8::from(value.effects.is_auburn)),
            "_isblond" => FieldValue::Boolean(u8::from(value.effects.is_blond)),
            "_iscoral" => FieldValue::Boolean(u8::from(value.effects.is_coral)),
            "_isenhanced" => FieldValue::Boolean(u8::from(value.effects.is_enhanced)),
            "_isfrozen" => FieldValue::Boolean(u8::from(value.effects.is_frozen)),
            "_isgray" => FieldValue::Boolean(u8::from(value.effects.is_gray)),
            "_isinfectious" => FieldValue::Boolean(u8::from(value.effects.is_infectious)),
            "_isinjured" => FieldValue::Boolean(u8::from(value.effects.is_injured)),
            "_islime" => FieldValue::Boolean(u8::from(value.effects.is_lime)),
            "_isonlyc4" => FieldValue::Boolean(u8::from(value.effects.is_only_c4)),
            "_isreproductive" => FieldValue::Boolean(u8::from(value.effects.is_reproductive)),
            "_issilver" => FieldValue::Boolean(u8::from(value.effects.is_silver)),
            "_isspike" => FieldValue::Boolean(u8::from(value.effects.is_spike)),
            "_jadefactor" => FieldValue::Int(value.effects.jade_factor),
            "_lastTheta" => FieldValue::DoubleBits(value.state.last_angle.to_bits()),
            "_lavender" => FieldValue::Int(value.effects.lavender),
            "_leafphoto" => FieldValue::DoubleBits(value.effects.leaf_photo.to_bits()),
            "_lengthfriend" => FieldValue::Int(value.effects.friend_length),
            "_lowersymmetric" => FieldValue::Int(value.effects.lower_symmetric),
            "_maintenance" => FieldValue::DoubleBits(value.effects.maintenance.to_bits()),
            "_mass" => FieldValue::DoubleBits(value.body.mass.to_bits()),
            "_max_age" => FieldValue::Int(value.state.reproduction.maximum_age),
            "_methanotrophy" => FieldValue::DoubleBits(value.effects.methanotrophy.to_bits()),
            "_nChildren" => FieldValue::Int(value.state.children),
            "_nPosition" => FieldValue::Reference(self.text(value.state.network_position.as_ref())?),
            "_nTotalChildren" => FieldValue::Int(value.state.total_children),
            "_nTotalInfected" => FieldValue::Int(value.state.total_infected),
            "_nTotalKills" => FieldValue::Int(value.state.total_kills),
            "_nVirusChildren" => FieldValue::Int(value.state.virus_children),
            "_parentID" => FieldValue::Int(value.state.parent_id),
            "_photosynthesis" => FieldValue::DoubleBits(value.effects.photosynthesis.to_bits()),
            "_plagueversion" => FieldValue::Int(value.effects.plague_version),
            "_reproduceEnergy" => FieldValue::Int(value.state.reproduction.energy),
            "_reproducelate" => FieldValue::Int(value.effects.reproduce_late),
            "_savedGeneticCode" => FieldValue::Reference(self.genome(value.saved_genome.as_ref())?),
            "_segColor" => FieldValue::Reference(self.colors(value.body.phenotype_colors.as_ref())?),
            "_segmentpretoucheffects" => FieldValue::Int(value.effects.segment_pre_touch_effects),
            "_segments" => FieldValue::Int(value.body.segment_count),
            "_siblingbattle" => FieldValue::Boolean(u8::from(value.effects.sibling_battle)),
            "_sizeRect" => FieldValue::Reference(self.rectangle(value.body.size_rect.as_ref())?),
            "_skyversion" => FieldValue::Int(value.effects.sky_version),
            "_spin" => FieldValue::Int(value.effects.spin),
            "_sporetime" => FieldValue::Int(value.effects.spore_time),
            "_sporeversion" => FieldValue::Int(value.effects.spore_version),
            "_summerinactivity" => FieldValue::Int(value.effects.summer_inactivity),
            "_theta" => FieldValue::DoubleBits(value.state.angle.to_bits()),
            "_thetafriend" => FieldValue::Int(value.effects.friend_theta),
            "_timeToReproduce" => FieldValue::Int(value.state.reproduction.countdown),
            "_timeToReproduceMax" => FieldValue::Int(value.state.reproduction.maximum_countdown),
            "_transfersenergy" => FieldValue::Boolean(u8::from(value.effects.transfers_energy)),
            "_updateEffects" => FieldValue::Int(value.body.update_effects),
            "_useextraeffects" => FieldValue::Boolean(u8::from(value.effects.use_extra_effects)),
            "_useframemovement" => FieldValue::Boolean(u8::from(value.effects.use_frame_movement)),
            "_usefriendeffects" => FieldValue::Int(value.effects.use_friend_effects),
            "_usepretoucheffects" => FieldValue::Boolean(u8::from(value.effects.use_pre_touch_effects)),
            "_world" => FieldValue::Reference(if value.state.world.is_present() {
                self.world(&value.state.world.resolve().map_err(|_| failure(GraphErrorKind::Reference))?)?
            } else { Value::Null }),
            "_yellowCounter" => FieldValue::Int(value.effects.yellow_count),
            "active" => FieldValue::Boolean(u8::from(value.state.active)),
            "alive" => FieldValue::Boolean(u8::from(value.state.alive)),
            "dtheta" => FieldValue::DoubleBits(value.state.angular_velocity.to_bits()),
            "dx" => FieldValue::DoubleBits(value.state.velocity[0].to_bits()),
            "dy" => FieldValue::DoubleBits(value.state.velocity[1].to_bits()),
            "hasGrown" => FieldValue::Int(value.state.has_grown),
            "hasMoved" => FieldValue::Boolean(u8::from(value.state.has_moved)),
            "lastFrame" => FieldValue::Reference(self.rectangle(value.state.last_frame.as_ref())?),
            _ => return Err(failure(GraphErrorKind::Field)),
        })
    }
    fn ints(&mut self, value: Option<&crate::IntArrayRef>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let (id, first) = self.reserve_owner(Key(b'i', value.identity(), 0))?;
        if first {
            let values = value.borrow_values(); self.elements(values.len())?;
            let mut copy = self.vector(values.len())?; copy.extend_from_slice(&values);
            self.array(id, "[I", ArrayData::Ints(copy))?;
        }
        Ok(Value::Handle(id))
    }
    fn doubles(&mut self, value: Option<&crate::DoubleArrayRef>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let (id, first) = self.reserve_owner(Key(b'd', value.identity(), 0))?;
        if first {
            let values = value.borrow_bits(); self.elements(values.len())?;
            let mut copy = self.vector(values.len())?; copy.extend_from_slice(&values);
            self.array(id, "[D", ArrayData::Doubles(copy))?;
        }
        Ok(Value::Handle(id))
    }
    fn floats(&mut self, value: Option<&JavaFloatArray>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let (id, first) = self.reserve_owner(Key(b'f', value.identity(), 0))?;
        if first {
            self.elements(value.bits().len())?;
            let mut copy = self.vector(value.bits().len())?; copy.extend_from_slice(value.bits());
            self.array(id, "[F", ArrayData::Floats(copy))?;
        }
        Ok(Value::Handle(id))
    }
    fn colors(&mut self, value: Option<&ColorArrayRef>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let (id, first) = self.reserve_owner(Key(b'a', value.identity(), 0))?;
        if first {
            self.enter()?;
            self.elements(value.len())?;
            let mut references = self.vector(value.len())?;
            for value in value.slots().iter() { references.push(self.color(value.as_ref())?); }
            self.array(id, "[Ljava.awt.Color;", ArrayData::References(references))?;
            self.depth -= 1;
        }
        Ok(Value::Handle(id))
    }
    fn color(&mut self, value: Option<&JavaColor>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let (id, first) = self.reserve_owner(Key(b'c', value.identity(), 0))?;
        if first {
            self.object(id, "java.awt.Color", |this, _, field| Ok(match field {
                "falpha" => FieldValue::FloatBits(value.alpha_bits()),
                "value" => FieldValue::Int(value.argb() as i32),
                "cs" => FieldValue::Reference(this.space(value.color_space())?),
                "frgbvalue" => FieldValue::Reference(this.floats(value.rgb_components())?),
                "fvalue" => FieldValue::Reference(this.floats(value.components())?),
                _ => return Err(failure(GraphErrorKind::Field)),
            }))?;
        }
        Ok(Value::Handle(id))
    }
    fn space(&mut self, value: Option<&JavaIccColorSpace>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let (id, first) = self.reserve_owner(Key(b's', value.identity(), 0))?;
        if first {
            self.object(id, "java.awt.color.ICC_ColorSpace", |this, _, field| Ok(match field {
                "numComponents" => FieldValue::Int(value.num_components()),
                "type" => FieldValue::Int(value.space_type()),
                "needScaleInit" => FieldValue::Boolean(u8::from(value.need_scale_init())),
                "diffMinMax" => FieldValue::Reference(this.floats(value.diff_min_max())?),
                "invDiffMinMax" => FieldValue::Reference(this.floats(value.inv_diff_min_max())?),
                "maxVal" => FieldValue::Reference(this.floats(value.max_val())?),
                "minVal" => FieldValue::Reference(this.floats(value.min_val())?),
                "thisProfile" => FieldValue::Reference(this.profile(value.profile())?),
                _ => return Err(failure(GraphErrorKind::Field)),
            }))?;
        }
        Ok(Value::Handle(id))
    }
    fn profile(&mut self, value: Option<&JavaIccProfile>) -> ExportResult<Value> {
        let Some(value) = value else { return Ok(Value::Null); };
        let (id, first) = self.reserve_owner(Key(b'p', value.identity(), 0))?;
        if first {
            self.object(id, value.kind().class_name(), |_, _, field| match field {
                "iccProfileSerializedDataVersion" => Ok(FieldValue::Int(1)),
                _ => Err(failure(GraphErrorKind::Field)),
            })?;
            let name = self.signature(value.kind().serialized_name())?;
            let mut custom = self.vector(2)?;
            custom.push(Record::Value(Value::Handle(name))); custom.push(Record::Value(Value::Null));
            let Node::Object(object) = &mut self.document.arena.nodes[id.0 as usize] else { unreachable!() };
            object.slots[0].custom = custom;
        }
        Ok(Value::Handle(id))
    }
    fn tree(&mut self, world: &World) -> ExportResult<Value> {
        self.tree_owner(world.spatial.borrow().shared_ref())
    }
    fn tree_owner(&mut self, owner: &TreeRef) -> ExportResult<Value> {
        let (id,first)=self.reserve_owner(Key(b'T',owner.identity(),0))?;
        if first {
            let tree=owner.value();
            self.object(id,TREE,|this,class,field| {
                if class!=TREE_BASE { return Err(failure(GraphErrorKind::Field)); }
                Ok(match field {
                    "built"=>FieldValue::Boolean(u8::from(tree.built)),
                    "nodeCapacity"=>FieldValue::Int(tree.node_capacity),
                    "itemBoundables"=>FieldValue::Reference(this.shared_list(tree.pending.as_ref())?),
                    "root"=>FieldValue::Reference(tree.root.as_ref().map(|root|this.boundable(root)).transpose()?.unwrap_or(Value::Null)),
                    _=>return Err(failure(GraphErrorKind::Field)),
                })
            })?;
        }
        Ok(Value::Handle(id))
    }
    fn shared_slot(&mut self, value: &SharedSlot) -> ExportResult<Value> {
        match value {
            SharedSlot::Null=>Ok(Value::Null),SharedSlot::Double(value)=>self.boxed(value),
            SharedSlot::Actor(value)=>self.actor(value),SharedSlot::Boundable(value)=>self.boundable(value),
            SharedSlot::Envelope(value)=>self.envelope(value),
        }
    }
    fn boundable(&mut self, owner: &BoundableRef) -> ExportResult<Value> {
        let (id,first)=self.reserve_owner(Key(b'b',owner.identity(),0))?;
        if first {
            match &*owner.value() {
                BoundableValue::Item { bounds,item }=>self.object(id,ITEM,|this,_,field| Ok(FieldValue::Reference(match field {
                    "bounds"=>bounds.as_ref().map(|bounds|this.envelope(bounds)).transpose()?.unwrap_or(Value::Null),
                    "item"=>this.shared_slot(item)?,
                    _=>return Err(failure(GraphErrorKind::Field)),
                })))?,
                BoundableValue::Node { level,bounds,children }=>self.object(id,NODE,|this,class,field| {
                    if class!=NODE_BASE { return Err(failure(GraphErrorKind::Field)); }
                    Ok(match field {
                        "level"=>FieldValue::Int(*level),
                        "bounds"=>FieldValue::Reference(bounds.as_ref().map(|bounds|this.envelope(bounds)).transpose()?.unwrap_or(Value::Null)),
                        "childBoundables"=>FieldValue::Reference(this.shared_list(children.as_ref())?),
                        _=>return Err(failure(GraphErrorKind::Field)),
                    })
                })?,
            }
        }
        Ok(Value::Handle(id))
    }
    fn envelope(&mut self, owner: &EnvelopeRef) -> ExportResult<Value> {
        let (id,first)=self.reserve_owner(Key(b'e',owner.identity(),0))?;
        if first {
            let value=owner.get();
            self.object(id,ENVELOPE,|_,_,field| Ok(FieldValue::DoubleBits(match field {
                "minx"=>value.min_x,"maxx"=>value.max_x,"miny"=>value.min_y,"maxy"=>value.max_y,
                _=>return Err(failure(GraphErrorKind::Field)),
            }.to_bits())))?;
        }
        Ok(Value::Handle(id))
    }
    fn finish(mut self, world: &World) -> ExportResult<(Document, Usage)> {
        if self.limits.records == 0 { return Err(failure(GraphErrorKind::Budget)); }
        let root = self.world(world)?;
        self.usage.reserve(&mut self.document.records, 1, self.limits)?;
        self.document.records.push(Record::Value(root));
        Ok((self.document, self.usage))
    }
}

impl World {
    /// Return one detached wire graph without retaining parsed input or mutating typed owners
    /// shortcut: null/mixed/cross-role list owners require the separate shared-owner migration
    pub(crate) fn encode_legacy_document(&self, limits: Limits, maximum: usize) -> ExportResult<Document> {
        WorldEmitter::new(limits, maximum).finish(self).map(|(document, _)| document)
    }

    /// Keep assembly storage and work charged while the existing codec writes output bytes
    pub(crate) fn encode_legacy_bytes(&self, mut limits: Limits, maximum: usize) -> ExportResult<Vec<u8>> {
        limits.allocated_bytes = limits.allocated_bytes.min(maximum);
        let (document, usage) = WorldEmitter::new(limits, maximum).finish(self)?;
        Ok(encode_document_charged(&document, limits, usage.allocated, usage.work)?)
    }
}

#[cfg(test)]
#[path = "legacy_world_export_tests.rs"]
pub(super) mod tests;
