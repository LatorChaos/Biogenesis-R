// SPDX-License-Identifier: GPL-2.0-or-later

//! Constructor-free assembly for complete World owners and shared persistent targets

use super::{World, WorldState};
use super::root::StoredWorldParts;
use crate::legacy_genetics::GeneticsStager;
use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::legacy_history::HISTORY_FIELDS;
use crate::legacy_shared_world_owners::SharedOwnerStager;
use crate::legacy_organism::stage_actor_batch_in;
use crate::legacy_organism_arrays::PrimitiveStager;
#[cfg(test)]
use crate::legacy_spatial::{ActorTokens, decode_spatial};
use crate::spatial::SpatialTree;
use crate::object_stream::{Document, Limits, Node, NodeId, Value, class_schema};
use crate::organism::OrganismRef;
use crate::statistics::legacy::StatisticsStager;
use std::{mem::size_of, rc::Rc};

const WORLD: &str = "biogenesis.World";
const ACTOR: &str = "biogenesis.Organism";
const TREE: &str = "org.locationtech.jts.index.strtree.AbstractSTRtree";
const NODE: &str = "org.locationtech.jts.index.strtree.AbstractNode";
const ITEM: &str = "org.locationtech.jts.index.strtree.ItemBoundable";
const SYNCHRONIZED: &str = "java.util.Collections$SynchronizedList";
const COLLECTION: &str = "java.util.Collections$SynchronizedCollection";
const STATISTICS: &str = "biogenesis.WorldStatistics";

/// These source shapes need the separate nullable/shared-owner representation join
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnsupportedWorldShape {
    NullWorldReference,
    MembershipContainer,
    NullMember,
    MixedList,
    SharedListRoles,
    SharedTreeList,
    ActorWorld,
}

/// Wire parsing remains object_stream's responsibility, before this typed boundary
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorldDecodeError {
    Graph(GraphError),
    Unsupported { node: NodeId, field: &'static str, shape: UnsupportedWorldShape },
}

impl From<GraphError> for WorldDecodeError {
    fn from(error: GraphError) -> Self { Self::Graph(error) }
}

fn failure(kind: GraphErrorKind) -> GraphError { GraphError { node: None, field: None, kind } }
fn unsupported(node: NodeId, field: &'static str, shape: UnsupportedWorldShape) -> WorldDecodeError {
    WorldDecodeError::Unsupported { node, field, shape }
}
fn reserve<T>(graph: &LegacyGraph<'_>, values: &mut Vec<T>, count: usize) -> Result<(), GraphError> {
    let required = values.len().checked_add(count).ok_or(failure(GraphErrorKind::Budget))?;
    if required <= values.capacity() { return Ok(()); }
    let before = values.capacity();
    graph.allocate((required - before).checked_mul(size_of::<T>()).ok_or(failure(GraphErrorKind::Budget))?)?;
    values.try_reserve_exact(count).map_err(|_| failure(GraphErrorKind::Allocation))?;
    graph.allocate((values.capacity() - required).checked_mul(size_of::<T>()).ok_or(failure(GraphErrorKind::Budget))?)
}
fn required(graph: &LegacyGraph<'_>, root: NodeId, field: &'static str) -> Result<NodeId, WorldDecodeError> {
    match graph.reference(root, WORLD, field)? {
        Value::Handle(node) => Ok(node),
        Value::Null => Err(unsupported(root, field, UnsupportedWorldShape::NullWorldReference)),
    }
}

fn membership_backing(graph: &LegacyGraph<'_>, root: NodeId) -> Result<Value, WorldDecodeError> {
    backing(graph, graph.reference(root, WORLD, "_organisms")?)
}
fn backing(graph: &LegacyGraph<'_>, value: Value) -> Result<Value, WorldDecodeError> {
    let Value::Handle(id) = value else { return Ok(Value::Null); };
    match graph.class_name(id)? {
        "java.util.ArrayList" => Ok(value),
        SYNCHRONIZED => Ok(Value::Handle(graph.required_reference(id, SYNCHRONIZED, "list")?)),
        _ => Err(unsupported(id, "list", UnsupportedWorldShape::MembershipContainer)),
    }
}

#[cfg(test)]
fn members(graph: &LegacyGraph<'_>, root: NodeId) -> Result<(NodeId, Vec<Value>), WorldDecodeError> {
    let Value::Handle(list) = membership_backing(graph, root)? else { return Err(unsupported(root, "_organisms", UnsupportedWorldShape::NullWorldReference)); };
    Ok((list, graph.array_list_compatible(list)?))
}

/// Admit the modeled owner roles before allocating any World or invoking typed holders
fn preflight(graph: &LegacyGraph<'_>) -> Result<(Vec<NodeId>, Vec<NodeId>), WorldDecodeError> {
    let mut worlds = Vec::new();
    let mut actors = Vec::new();
    for (index, node) in graph.arena().nodes.iter().enumerate() {
        match node {
            Node::Descriptor(_) => graph.validate_descriptor(NodeId(index as u32))?,
            Node::Object(object) => {
                for slot in graph.slots(NodeId(index as u32))? {
                    let Node::Descriptor(descriptor) = graph.node(slot.descriptor)? else {
                        return Err(GraphError::at(slot.descriptor, None, GraphErrorKind::Descriptor).into());
                    };
                    if class_schema(descriptor.class).flags & 1 == 0 && !slot.custom.is_empty() {
                        return Err(GraphError::at(NodeId(index as u32), None, GraphErrorKind::ContainerData).into());
                    }
                }
                let Node::Descriptor(descriptor) = graph.node(object.descriptor)? else {
                    return Err(GraphError::at(object.descriptor, None, GraphErrorKind::Descriptor).into());
                };
                let id = NodeId(index as u32);
                match class_schema(descriptor.class).name {
                    WORLD => { reserve(graph, &mut worlds, 1)?; worlds.push(id); },
                    ACTOR => {
                        if let Value::Handle(owner) = graph.reference(id, ACTOR, "_world")? {
                            if graph.class_name(owner)? != WORLD {
                                return Err(unsupported(id, "_world", UnsupportedWorldShape::ActorWorld));
                            }
                        }
                        reserve(graph, &mut actors, 1)?; actors.push(id);
                    },
                    _ => {},
                }
            }
            _ => {}
        }
    }
    for world in &worlds { preflight_world(graph, *world)?; }
    Ok((worlds, actors))
}

fn preflight_world(graph: &LegacyGraph<'_>, root: NodeId) -> Result<(), WorldDecodeError> {
    let statistics = required(graph, root, "worldStatistics")?;
    let tree = required(graph, root, "colDetTree")?;
    let mut lists = [Value::Null; 7];
    lists[0] = membership_backing(graph, root)?;
    graph.expect_class(statistics, STATISTICS)?;
    for (slot, field) in lists[1..].iter_mut().zip(HISTORY_FIELDS) { *slot = backing(graph, graph.reference(statistics, STATISTICS, field)?)?; }
    for list in lists {
        let Value::Handle(list) = list else { continue; };
        {
            for value in graph.array_list_compatible(list)? {
                if let Value::Handle(value) = value {
                    match graph.class_name(value)? {
                        ACTOR | "java.lang.Double" | ITEM | "org.locationtech.jts.index.strtree.STRtree$STRtreeNode" | "org.locationtech.jts.geom.Envelope" => {},
                        _ => return Err(unsupported(list, "list", UnsupportedWorldShape::MixedList)),
                    }
                }
            }
        }
    }
    graph.expect_class(tree, "org.locationtech.jts.index.strtree.STRtree")?;
    Ok(())
}

impl World {
    /// Return a new root in this session; failure does not modify any caller-owned state
    pub(crate) fn decode_legacy_document(
        &self, document: &Document, mut limits: Limits, maximum: usize,
    ) -> Result<Self, WorldDecodeError> {
        limits.allocated_bytes = limits.allocated_bytes.min(maximum);
        let graph = LegacyGraph::new(document, limits)?;
        let root = graph.world_root()?;
        let (world_nodes, actor_nodes) = preflight(&graph)?;
        let mut genetics = GeneticsStager::new(&graph, limits.allocated_bytes)?;
        let mut primitive = PrimitiveStager::new(&graph, limits.allocated_bytes)?;
        let mut statistics = StatisticsStager::new(&graph, limits.allocated_bytes)?;
        let mut worlds = Vec::new();
        reserve(&graph, &mut worlds, world_nodes.len())?;
        for node in world_nodes {
            genetics.object(node, WORLD)?;
            graph.allocate(size_of::<WorldState>() + 2 * size_of::<usize>())?;
            let world = self.stage_legacy_root(StoredWorldParts {
                width: graph.int(node, WORLD, "_width")?,
                height: graph.int(node, WORLD, "_height")?,
                next_id: graph.int(node, WORLD, "NEXT_ID")?,
                population: graph.int(node, WORLD, "_population")?,
                frame: graph.int(node, WORLD, "nFrames")?,
                corridors_exist: graph.boolean(node, WORLD, "_corridorexists")?,
                oxygen: f64::from_bits(graph.double_bits(node, WORLD, "_O2")?),
                carbon_dioxide: f64::from_bits(graph.double_bits(node, WORLD, "_CO2")?),
                methane: f64::from_bits(graph.double_bits(node, WORLD, "_CH4")?),
                spatial: SpatialTree::pending(),
            });
            worlds.push((node, world.handle()));
        }
        let batch = stage_actor_batch_in(&graph, &actor_nodes, &worlds, &mut genetics, &mut primitive)?;
        let mut shared = SharedOwnerStager::new(&graph, &batch.actors, &worlds)?;
        graph.allocate(size_of::<crate::rectangles::RectangleActorContext>() + 2 * size_of::<usize>())?;
        let context = Rc::new(batch.context);
        for (node, world) in &worlds {
            let organisms = shared.list(graph.reference(*node, WORLD, "_organisms")?)?;
            let statistics = statistics.statistics(graph.reference(*node, WORLD, "worldStatistics")?, &mut genetics, &mut shared)?
                .ok_or(GraphError::at(*node, Some("worldStatistics"), GraphErrorKind::Reference))?;
            let tree = shared.tree(graph.reference(*node, WORLD, "colDetTree")?)?
                .ok_or(GraphError::at(*node, Some("colDetTree"), GraphErrorKind::Reference))?;
            crate::legacy_spatial_owners::validate(&graph, &tree)?;
            let lists = organisms.as_ref().map_or(&[][..], std::slice::from_ref);
            let actors = crate::shared_world_owners::referenced_actors_in(lists, std::slice::from_ref(&tree), &graph)?;
            let mut tokens = Vec::new();
            reserve(&graph, &mut tokens, actors.len())?;
            tokens.extend(actors.iter().map(Rc::downgrade));
            world.alias().fill_legacy_references(organisms, statistics, tree, tokens, Rc::clone(&context));
        }
        let mut result = worlds.iter().find(|(node, _)| *node == root)
            .ok_or(GraphError::at(root, None, GraphErrorKind::Reference))?.1.alias();
        if worlds.len() > 1 {
            result.retain_worlds_metered(&batch.worlds, limits, limits.allocated_bytes, Some((&graph, batch.actors.len())))?;
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "legacy_world_tests.rs"]
mod tests;
