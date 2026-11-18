// SPDX-License-Identifier: GPL-2.0-or-later

//! Caller-bound Rectangle staging; full actor and World hydration are separate boundaries

use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::object_stream::{Document, FieldValue, Limits, Node, NodeId, Value, class_schema};
use crate::organism::{GenomeRef, Organism, OrganismRef};
use crate::rectangles::RectangleActorContext;
use crate::{IntRect, JavaColor, JavaFloatArray, JavaText, RectangleRef};
use std::{cell::RefCell, collections::HashMap, hash::Hash, mem::size_of, rc::Rc};

const RECTANGLE: &str = "java.awt.Rectangle";
const ORGANISM: &str = "biogenesis.Organism";
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

fn staging_bytes(graph: &LegacyGraph<'_>, used: &mut usize, maximum: usize, bytes: usize) -> Result<(), GraphError> {
    let next = used.checked_add(bytes).ok_or(error(GraphErrorKind::Budget))?;
    if next > maximum { return Err(error(GraphErrorKind::Budget)); }
    graph.allocate(bytes)?;
    *used = next;
    Ok(())
}

fn reserve_index<K: Eq + Hash, V>(
    graph: &LegacyGraph<'_>, index: &mut HashMap<K, V>, used: &mut usize, maximum: usize,
) -> Result<(), GraphError> {
    if index.len() < index.capacity() { return Ok(()); }
    let before = index.capacity();
    let width = product(2, size_of::<(K, V)>())?;
    staging_bytes(graph, used, maximum, width)?;
    graph.charge(index.len())?;
    index.try_reserve(1).map_err(|_| error(GraphErrorKind::Allocation))?;
    staging_bytes(graph, used, maximum, product(index.capacity() - before - 1, width)?)
}

// Logical payload accounting includes Rc/Arc counters, actual Vec capacities and bounded validation storage
// This is a staging allocation bound rather than an allocator or RSS measurement
pub(crate) struct Retention {
    used: usize,
    maximum: usize,
    work: usize,
    work_limit: usize,
    owner_limit: usize,
    seen_index: HashMap<(u8, usize), ()>,
}
impl Retention {
    pub(crate) fn new(maximum: usize, limits: Limits) -> Result<Self, GraphError> {
        let value = Self {
            used: 0,
            maximum,
            work: 0,
            work_limit: limits.work,
            owner_limit: limits.handles,
            seen_index: HashMap::new(),
        };
        Ok(value)
    }
    pub(crate) fn bytes(&mut self, bytes: usize) -> Result<(), GraphError> {
        self.used = self
            .used
            .checked_add(bytes)
            .ok_or(error(GraphErrorKind::Budget))?;
        if self.used > self.maximum {
            return Err(error(GraphErrorKind::Budget));
        }
        Ok(())
    }
    pub(crate) fn visit(&mut self, count: usize) -> Result<(), GraphError> {
        self.work = self
            .work
            .checked_add(count)
            .ok_or(error(GraphErrorKind::Budget))?;
        if self.work > self.work_limit {
            return Err(error(GraphErrorKind::Budget));
        }
        Ok(())
    }
    pub(crate) fn first(&mut self, kind: u8, identity: usize) -> Result<bool, GraphError> {
        self.visit(1)?;
        if self.seen_index.contains_key(&(kind, identity)) {
            return Ok(false);
        }
        if self.seen_index.len() >= self.owner_limit {
            return Err(error(GraphErrorKind::Budget));
        }
        if self.seen_index.len() == self.seen_index.capacity() {
            let before = self.seen_index.capacity();
            let width = product(2, size_of::<((u8, usize), ())>())?;
            self.bytes(width)?;
            self.visit(self.seen_index.len())?;
            self.seen_index.try_reserve(1).map_err(|_| error(GraphErrorKind::Allocation))?;
            self.bytes(product(self.seen_index.capacity() - before - 1, width)?)?;
        }
        self.seen_index.insert((kind, identity), ());
        Ok(true)
    }
    pub(crate) fn allocation<T>(&mut self) -> Result<(), GraphError> {
        self.bytes(
            size_of::<T>()
                .checked_add(2 * size_of::<usize>())
                .ok_or(error(GraphErrorKind::Budget))?,
        )
    }
    pub(crate) fn text(&mut self, text: Option<&JavaText>) -> Result<(), GraphError> {
        self.visit(1)?;
        if let Some(text) = text {
            if self.first(b'T', text.units().as_ptr() as usize)? {
                self.bytes(
                    (2 * size_of::<usize>())
                        .checked_add(product(text.units().len(), size_of::<u16>())?)
                        .ok_or(error(GraphErrorKind::Budget))?,
                )?;
            }
        }
        Ok(())
    }
    fn floats(&mut self, value: Option<&JavaFloatArray>) -> Result<(), GraphError> {
        self.visit(1)?;
        if let Some(value) = value {
            if self.first(b'F', value.identity())? {
                self.allocation::<Vec<u32>>()?;
                self.bytes(product(value.storage_capacity(), size_of::<u32>())?)?;
            }
        }
        Ok(())
    }
    fn color(&mut self, value: Option<&JavaColor>) -> Result<(), GraphError> {
        self.visit(1)?;
        let Some(value) = value else {
            return Ok(());
        };
        if !self.first(b'C', value.identity())? {
            return Ok(());
        }
        self.allocation::<crate::java_color::ColorData>()?;
        self.floats(value.rgb_components())?;
        self.floats(value.components())?;
        if let Some(space) = value.color_space() {
            if self.first(b'S', space.identity())? {
                self.allocation::<crate::java_color::IccSpaceData>()?;
                let parts = space.parts();
                for value in [
                    &parts.diff_min_max,
                    &parts.inv_diff_min_max,
                    &parts.max_val,
                    &parts.min_val,
                ] {
                    self.floats(value.as_ref())?;
                }
                if let Some(profile) = &parts.profile {
                    if self.first(b'P', profile.identity())? {
                        self.allocation::<crate::NamedIccProfile>()?;
                    }
                }
            }
        }
        Ok(())
    }
    pub(crate) fn genome(&mut self, value: Option<&GenomeRef>) -> Result<(), GraphError> {
        self.visit(1)?;
        let Some(value) = value else {
            return Ok(());
        };
        if !self.first(b'G', Rc::as_ptr(value) as usize)? {
            return Ok(());
        }
        self.allocation::<RefCell<crate::Genome>>()?;
        let genome = value.borrow();
        self.text(genome.clade_id())?;
        self.visit(1)?;
        if let Some(array) = genome.gene_array() {
            if self.first(b'J', array.identity())? {
                self.bytes(array.allocation_bytes().ok_or(error(GraphErrorKind::Budget))?)?;
                for gene in array.slots().iter() {
                    self.visit(1)?;
                    if let Some(gene) = gene {
                        if self.first(b'N', Rc::as_ptr(gene) as usize)? {
                            self.allocation::<RefCell<crate::Gene>>()?;
                            self.color(gene.borrow().color_handle())?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn rectangle(&mut self, value: Option<&RectangleRef>) -> Result<(), GraphError> {
        self.visit(1)?;
        if let Some(value) = value {
            if self.first(b'R', value.identity())? {
                self.bytes(RectangleRef::allocation_bytes())?;
            }
        }
        Ok(())
    }
    pub(crate) fn retained_actor(&mut self, value: &OrganismRef) -> Result<bool, GraphError> {
        self.visit(1)?;
        if self.seen_index.contains_key(&(b'A', Rc::as_ptr(value) as usize)) { return Ok(false); }
        self.actor(value)?;
        Ok(true)
    }

    pub(crate) fn actor(&mut self, value: &OrganismRef) -> Result<(), GraphError> {
        if !self.first(b'A', Rc::as_ptr(value) as usize)? {
            return Err(error(GraphErrorKind::Reference));
        }
        self.allocation::<RefCell<Organism>>()?;
        let actor = value.borrow();
        self.text(actor.state.network_position.as_ref())?;
        self.genome(actor.genome.as_ref())?;
        self.genome(actor.infected_genome.as_ref())?;
        self.genome(actor.saved_genome.as_ref())?;
        self.color(actor.state.temporary_color.as_ref())?;
        for value in [
            Some(&actor.state.bounds),
            actor.body.size_rect.as_ref(),
            actor.state.last_frame.as_ref(),
        ] {
            self.rectangle(value)?;
        }
        for field in crate::organism_arrays::INT_FIELDS {
            self.visit(1)?;
            if let Some(array) = actor.body.arrays.int(field) {
                if self.first(b'I', array.identity())? {
                    self.allocation::<RefCell<Vec<i32>>>()?;
                    self.bytes(product(array.borrow_values().capacity(), size_of::<i32>())?)?;
                }
            }
        }
        for field in crate::organism_arrays::DOUBLE_FIELDS {
            self.visit(1)?;
            if let Some(array) = actor.body.arrays.double(field) {
                if self.first(b'D', array.identity())? {
                    self.allocation::<RefCell<Vec<u64>>>()?;
                    self.bytes(product(array.borrow_bits().capacity(), size_of::<u64>())?)?;
                }
            }
        }
        if let Some(array) = &actor.body.phenotype_colors {
            if self.first(b'L', array.identity())? {
                self.allocation::<RefCell<Vec<Option<JavaColor>>>>()?;
                let slots = array.slots();
                self.bytes(product(slots.capacity(), size_of::<Option<JavaColor>>())?)?;
                for color in slots.iter() {
                    self.color(color.as_ref())?;
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn retain_worlds_with_work(
    worlds: &Vec<Rc<crate::world::WorldState>>,
    root: &Rc<crate::world::WorldState>,
    limits: Limits,
    maximum: usize,
    mut retained: Vec<OrganismRef>,
) -> Result<(Vec<OrganismRef>, usize), GraphError> {
    if worlds.len() > limits.handles { return Err(error(GraphErrorKind::Budget)); }
    let mut budget = Retention::new(maximum, limits)?;
    budget.first(b'K', Rc::as_ptr(root) as usize)?;
    budget.bytes(crate::world::WorldState::cohort_allocation_bytes())?;
    budget.bytes(product(worlds.capacity(), size_of::<Rc<crate::world::WorldState>>())?)?;
    budget.bytes(product(retained.capacity(), size_of::<OrganismRef>())?)?;
    for (index, world) in worlds.iter().enumerate() {
        budget.visit(index.checked_add(1).ok_or(error(GraphErrorKind::Budget))?)?;
        if worlds[..index].iter().any(|earlier| Rc::ptr_eq(earlier, world)) {
            return Err(error(GraphErrorKind::Reference));
        }
        world.charge_retained(&mut budget, worlds, &mut retained)?;
    }
    budget.visit(worlds.len())?;
    if !worlds.iter().any(|world| Rc::ptr_eq(world, root)) {
        return Err(error(GraphErrorKind::Reference));
    }
    Ok((retained, budget.work))
}


/// The caller owns complete actors; this checks and charges their retained backing without modifying any actor
pub(crate) fn retain_actor_context(
    actors: Vec<OrganismRef>, limits: Limits, maximum: usize,
) -> Result<RectangleActorContext, GraphError> {
    retain_actor_context_with_work(actors, limits, maximum).0
}

fn retain_actor_context_with_work(
    actors: Vec<OrganismRef>,
    limits: Limits,
    maximum: usize,
) -> (Result<RectangleActorContext, GraphError>, usize, usize) {
    if actors.len() > limits.handles {
        return (Err(error(GraphErrorKind::Budget)), 0, 0);
    }
    let mut budget = match Retention::new(maximum, limits) {
        Ok(budget) => budget,
        Err(error) => return (Err(error), maximum, 0),
    };
    let result = (|| -> Result<(), GraphError> {
        budget.bytes(size_of::<RectangleActorContext>())?;
        budget.bytes(product(actors.capacity(), size_of::<OrganismRef>())?)?;
        for actor in &actors {
            budget.actor(actor)?;
        }
        for actor in &actors {
            budget.visit(1)?;
            let value = actor.borrow();
            if !value
                .state
                .bounds
                .actor_target()
                .is_some_and(|target| Rc::ptr_eq(&target, actor))
            {
                return Err(error(GraphErrorKind::Reference));
            }
            for cell in [
                Some(&value.state.bounds),
                value.body.size_rect.as_ref(),
                value.state.last_frame.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                if cell.is_actor() {
                    let target = cell
                        .actor_target()
                        .ok_or(error(GraphErrorKind::Reference))?;
                    budget.visit(1)?;
                    if !budget.seen_index.contains_key(&(b'A', Rc::as_ptr(&target) as usize)) {
                        return Err(error(GraphErrorKind::Reference));
                    }
                }
            }
        }
        Ok(())
    })();
    let used = budget.used.min(maximum);
    let work = budget.work.min(limits.work);
    (result.map(|()| RectangleActorContext::validated(actors, used, maximum)), used, work)
}

#[derive(Clone)]
pub(crate) struct StagedActorRectangles {
    pub actor: OrganismRef,
    pub bounds: RectangleRef,
    pub size_rect: Option<RectangleRef>,
    pub last_frame: Option<RectangleRef>,
}
pub(crate) struct StagedRectangles {
    pub actors: Vec<StagedActorRectangles>,
    pub context: RectangleActorContext,
}

/// Wire identities belong only to one explicit staging/export Document, never to runtime actor owners
pub(crate) struct RectangleDocumentBindings<'document> {
    document: &'document mut Document,
    actors: Vec<(NodeId, OrganismRef)>,
    rectangles: Vec<(NodeId, RectangleRef)>,
}
fn rectangle_value(graph: &LegacyGraph<'_>, id: NodeId) -> Result<IntRect, GraphError> {
    Ok(IntRect {
        x: graph.int(id, RECTANGLE, "x")?,
        y: graph.int(id, RECTANGLE, "y")?,
        width: graph.int(id, RECTANGLE, "width")?,
        height: graph.int(id, RECTANGLE, "height")?,
    })
}

pub(crate) fn stage_actor_rectangles<'document>(
    document: &'document mut Document,
    limits: Limits,
    bindings: Vec<(NodeId, OrganismRef)>,
    maximum: usize,
) -> Result<(StagedRectangles, RectangleDocumentBindings<'document>), GraphError> {
    let graph = LegacyGraph::new(document, limits)?;
    let world = graph.world_root()?;
    if bindings.len() > limits.handles { return Err(error(GraphErrorKind::Budget)); }
    let initial = product(bindings.capacity(), size_of::<(NodeId, OrganismRef)>())?
        .checked_add(size_of::<StagedRectangles>())
        .and_then(|value| value.checked_add(size_of::<RectangleDocumentBindings<'_>>()))
        .ok_or(error(GraphErrorKind::Budget))?;
    if initial > maximum { return Err(error(GraphErrorKind::Budget)); }
    let members = graph.required_reference(world, "biogenesis.World", "_organisms")?;
    let (_, members) = graph.synchronized_list(members)?;
    for member in members {
        graph.charge(
            bindings
                .len()
                .checked_add(1)
                .ok_or(error(GraphErrorKind::Budget))?,
        )?;
        let Value::Handle(member) = member else {
            return Err(error(GraphErrorKind::Reference));
        };
        if !bindings.iter().any(|(id, _)| *id == member) {
            return Err(GraphError::at(member, None, GraphErrorKind::Reference));
        }
    }
    for (id, _) in &bindings {
        if graph.reference(*id, ORGANISM, "_world")? != Value::Handle(world) {
            return Err(GraphError::at(*id, Some("_world"), GraphErrorKind::Reference));
        }
    }
    let (staged, rectangles) = stage_actor_rectangles_in(&graph, &bindings, maximum)?;
    drop(graph);
    Ok((staged, RectangleDocumentBindings { document, actors: bindings, rectangles }))
}

/// Join already admitted actors without resetting the document's cumulative budget
pub(crate) fn stage_actor_rectangles_in(
    graph: &LegacyGraph<'_>,
    bindings: &Vec<(NodeId, OrganismRef)>,
    maximum: usize,
) -> Result<(StagedRectangles, Vec<(NodeId, RectangleRef)>), GraphError> {
    let limits = graph.limits();
    if bindings.len() > limits.handles { return Err(error(GraphErrorKind::Budget)); }
    let mut used = product(bindings.capacity(), size_of::<(NodeId, OrganismRef)>())?
        .checked_add(size_of::<StagedRectangles>())
        .and_then(|value| value.checked_add(size_of::<RectangleDocumentBindings<'_>>()))
        .ok_or(error(GraphErrorKind::Budget))?;
    if used > maximum {
        return Err(error(GraphErrorKind::Budget));
    }
    graph.allocate(used)?;
    let mut by_node = HashMap::new();
    let mut actor_owners = HashMap::new();
    let mut bounds_owners = HashMap::new();
    for (index, (id, actor)) in bindings.iter().enumerate() {
        graph.charge(1)?;
        graph.expect_class(*id, ORGANISM)?;
        let value = actor.borrow();
        if value.state.bounds.get() != rectangle_value(&graph, *id)?
            || value.id != i64::from(graph.int(*id, ORGANISM, "_ID")?)
        {
            return Err(GraphError::at(*id, None, GraphErrorKind::Reference));
        }
        let actor_identity = Rc::as_ptr(actor) as usize;
        let bounds_identity = value.state.bounds.identity();
        graph.charge(3)?;
        if by_node.contains_key(&id.0) || actor_owners.contains_key(&actor_identity)
            || bounds_owners.contains_key(&bounds_identity)
        {
            return Err(GraphError::at(*id, None, GraphErrorKind::Reference));
        }
        reserve_index(graph, &mut by_node, &mut used, maximum)?;
        reserve_index(graph, &mut actor_owners, &mut used, maximum)?;
        reserve_index(graph, &mut bounds_owners, &mut used, maximum)?;
        by_node.insert(id.0, index);
        actor_owners.insert(actor_identity, ());
        bounds_owners.insert(bounds_identity, ());
    }
    drop((actor_owners, bounds_owners));
    let mut actors = Vec::new();
    used = used
        .checked_add(product(bindings.len(), size_of::<OrganismRef>())?)
        .ok_or(error(GraphErrorKind::Budget))?;
    if used > maximum {
        return Err(error(GraphErrorKind::Budget));
    }
    graph.allocate(product(bindings.len(), size_of::<OrganismRef>())?)?;
    actors
        .try_reserve_exact(bindings.len())
        .map_err(|_| error(GraphErrorKind::Allocation))?;
    actors.extend(bindings.iter().map(|(_, actor)| actor.clone()));
    let remaining = maximum
        .checked_sub(used)
        .ok_or(error(GraphErrorKind::Budget))?;
    let retention_limits = Limits { work: graph.remaining_work(), ..limits };
    let (context, charged, work) = retain_actor_context_with_work(
        actors, retention_limits, remaining.min(graph.remaining_allocation()),
    );
    graph.charge(work)?;
    graph.allocate(charged)?;
    let mut context = context?;
    used = used
        .checked_add(context.retained_bytes())
        .ok_or(error(GraphErrorKind::Budget))?;
    let mut rectangles = Vec::<(NodeId, RectangleRef)>::new();
    let mut states = Vec::new();
    let per_actor = size_of::<StagedActorRectangles>() + size_of::<(NodeId, RectangleRef)>();
    used = used
        .checked_add(product(bindings.len(), per_actor)?)
        .ok_or(error(GraphErrorKind::Budget))?;
    if used > maximum {
        return Err(error(GraphErrorKind::Budget));
    }
    graph.allocate(product(bindings.len(), per_actor)?)?;
    rectangles
        .try_reserve_exact(bindings.len())
        .map_err(|_| error(GraphErrorKind::Allocation))?;
    states
        .try_reserve_exact(bindings.len())
        .map_err(|_| error(GraphErrorKind::Allocation))?;
    let extra = product(
        rectangles.capacity() - bindings.len(),
        size_of::<(NodeId, RectangleRef)>(),
    )?
    .checked_add(product(
        states.capacity() - bindings.len(),
        size_of::<StagedActorRectangles>(),
    )?)
    .ok_or(error(GraphErrorKind::Budget))?;
    used = used
        .checked_add(extra)
        .ok_or(error(GraphErrorKind::Budget))?;
    if used > maximum {
        return Err(error(GraphErrorKind::Budget));
    }
    graph.allocate(extra)?;
    for (id, actor) in bindings {
        rectangles.push((*id, actor.borrow().state.bounds.clone()));
    }
    for (id, actor) in bindings {
        let mut read = |field| -> Result<Option<RectangleRef>, GraphError> {
            let Value::Handle(target) = graph.reference(*id, ORGANISM, field)? else {
                return Ok(None);
            };
            graph.charge(1)?;
            if let Some(index) = by_node.get(&target.0) {
                return Ok(Some(rectangles[*index].1.clone()));
            }
            match graph.class_name(target)? {
                RECTANGLE => {
                    let grows = rectangles.len() == rectangles.capacity();
                    let cost = RectangleRef::allocation_bytes()
                        .checked_add(if grows {
                            size_of::<(NodeId, RectangleRef)>()
                        } else {
                            0
                        })
                        .ok_or(error(GraphErrorKind::Budget))?;
                    used = used
                        .checked_add(cost)
                        .ok_or(error(GraphErrorKind::Budget))?;
                    if used > maximum {
                        return Err(error(GraphErrorKind::Budget));
                    }
                    graph.allocate(cost)?;
                    if grows {
                        let before = rectangles.capacity();
                        rectangles
                            .try_reserve_exact(1)
                            .map_err(|_| error(GraphErrorKind::Allocation))?;
                        let extra = product(
                            rectangles.capacity() - before - 1,
                            size_of::<(NodeId, RectangleRef)>(),
                        )?;
                        used = used
                            .checked_add(extra)
                            .ok_or(error(GraphErrorKind::Budget))?;
                        if used > maximum {
                            return Err(error(GraphErrorKind::Budget));
                        }
                        graph.allocate(extra)?;
                    }
                    reserve_index(graph, &mut by_node, &mut used, maximum)?;
                    let cell = RectangleRef::new(rectangle_value(&graph, target)?);
                    by_node.insert(target.0, rectangles.len());
                    rectangles.push((target, cell.clone()));
                    Ok(Some(cell))
                }
                ORGANISM => Err(GraphError::at(target, None, GraphErrorKind::Reference)),
                _ => Err(GraphError::at(target, None, GraphErrorKind::Class)),
            }
        };
        let size_rect = read("_sizeRect")?;
        let last_frame = read("lastFrame")?;
        states.push(StagedActorRectangles {
            actor: actor.clone(),
            bounds: actor.borrow().state.bounds.clone(),
            size_rect,
            last_frame,
        });
    }
    context.include_staging_bytes(used, maximum);
    Ok((StagedRectangles { actors: states, context }, rectangles))
}

#[derive(Clone, Copy)]
struct Edit {
    node: NodeId,
    slot: usize,
    field: usize,
    value: FieldValue,
}
fn field_position(
    graph: &LegacyGraph<'_>,
    node: NodeId,
    owner: &'static str,
    field: &'static str,
) -> Result<(usize, usize), GraphError> {
    for (position, slot) in graph.slots(node)?.iter().enumerate() {
        let Node::Descriptor(descriptor) = graph.node(slot.descriptor)? else {
            return Err(GraphError::at(
                node,
                Some(field),
                GraphErrorKind::Descriptor,
            ));
        };
        let schema = class_schema(descriptor.class);
        if schema.name == owner {
            let index = schema
                .fields
                .iter()
                .position(|value| value.name == field)
                .ok_or(GraphError::at(node, Some(field), GraphErrorKind::Field))?;
            return Ok((position, index));
        }
    }
    Err(GraphError::at(node, Some(field), GraphErrorKind::Field))
}
impl RectangleDocumentBindings<'_> {
    pub(crate) fn document(&self) -> &Document {
        &*self.document
    }
    pub(crate) fn rectangle_nodes(&self) -> &[(NodeId, RectangleRef)] {
        &self.rectangles
    }
    fn reference(
        &self,
        graph: &LegacyGraph<'_>,
        cell: Option<&RectangleRef>,
    ) -> Result<Value, GraphError> {
        graph.charge(1)?;
        let Some(cell) = cell else {
            return Ok(Value::Null);
        };
        graph.charge(self.rectangles.len())?;
        let (node, _) = self
            .rectangles
            .iter()
            .find(|(_, value)| value.same_instance(cell))
            .ok_or(error(GraphErrorKind::Reference))?;
        if cell.is_actor() {
            graph.charge(self.actors.len())?;
            let actor = cell
                .actor_target()
                .ok_or(error(GraphErrorKind::Reference))?;
            if !self
                .actors
                .iter()
                .any(|(id, value)| id == node && Rc::ptr_eq(value, &actor))
            {
                return Err(error(GraphErrorKind::Reference));
            }
        }
        Ok(Value::Handle(*node))
    }
    /// Update only supplied Rectangle fields; caller-owned actor payloads and all other slots remain untouched
    pub(crate) fn export_into(
        &mut self,
        states: &[StagedActorRectangles],
        limits: Limits,
        maximum: usize,
    ) -> Result<(), GraphError> {
        let graph = LegacyGraph::new(&*self.document, limits)?;
        let count = self
            .rectangles
            .len()
            .checked_mul(4)
            .and_then(|value| value.checked_add(states.len().checked_mul(2)?))
            .ok_or(error(GraphErrorKind::Budget))?;
        let bytes = product(count, size_of::<Edit>())?;
        if bytes > maximum {
            return Err(error(GraphErrorKind::Budget));
        }
        graph.allocate(bytes)?;
        graph.charge(count)?;
        let mut edits = Vec::new();
        edits
            .try_reserve_exact(count)
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        let actual = product(edits.capacity(), size_of::<Edit>())?;
        if actual > maximum {
            return Err(error(GraphErrorKind::Budget));
        }
        graph.allocate(actual - bytes)?;
        for (node, cell) in &self.rectangles {
            graph.expect_class(*node, cell.class_name())?;
            if self.reference(&graph, Some(cell))? != Value::Handle(*node) {
                return Err(error(GraphErrorKind::Reference));
            }
            let bounds = cell.get();
            for (field, value) in [
                ("x", bounds.x),
                ("y", bounds.y),
                ("width", bounds.width),
                ("height", bounds.height),
            ] {
                let (slot, index) = field_position(&graph, *node, RECTANGLE, field)?;
                if !matches!(graph.field(*node, RECTANGLE, field)?, FieldValue::Int(_)) {
                    return Err(error(GraphErrorKind::Type));
                }
                edits.push(Edit {
                    node: *node,
                    slot,
                    field: index,
                    value: FieldValue::Int(value),
                });
            }
        }
        for (index, state) in states.iter().enumerate() {
            graph.charge(
                index
                    .checked_add(self.actors.len())
                    .and_then(|value| value.checked_add(1))
                    .ok_or(error(GraphErrorKind::Budget))?,
            )?;
            if states[..index]
                .iter()
                .any(|other| Rc::ptr_eq(&state.actor, &other.actor))
            {
                return Err(error(GraphErrorKind::Reference));
            }
            let (node, _) = self
                .actors
                .iter()
                .find(|(_, actor)| Rc::ptr_eq(actor, &state.actor))
                .ok_or(error(GraphErrorKind::Reference))?;
            if self.reference(&graph, Some(&state.bounds))? != Value::Handle(*node) {
                return Err(error(GraphErrorKind::Reference));
            }
            for (field, value) in [
                ("_sizeRect", state.size_rect.as_ref()),
                ("lastFrame", state.last_frame.as_ref()),
            ] {
                let (slot, index) = field_position(&graph, *node, ORGANISM, field)?;
                if !matches!(
                    graph.field(*node, ORGANISM, field)?,
                    FieldValue::Reference(_)
                ) {
                    return Err(error(GraphErrorKind::Type));
                }
                edits.push(Edit {
                    node: *node,
                    slot,
                    field: index,
                    value: FieldValue::Reference(self.reference(&graph, value)?),
                });
            }
        }
        drop(graph);
        for edit in edits {
            let Node::Object(object) = &mut self.document.arena.nodes[edit.node.0 as usize] else {
                unreachable!("Validated object slot");
            };
            object.slots[edit.slot].values[edit.field] = edit.value;
        }
        Ok(())
    }
}
