// SPDX-License-Identifier: GPL-2.0-or-later

use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::object_stream::{Arena, NodeId, Value};
use crate::spatial::{Envelope, SpatialTree, StoredBoundable, StoredTree, StoredTreeError};
use std::collections::{HashMap, HashSet};

const WORLD: &str = "biogenesis.World";
const TREE: &str = "org.locationtech.jts.index.strtree.AbstractSTRtree";
const NODE: &str = "org.locationtech.jts.index.strtree.AbstractNode";
const ITEM: &str = "org.locationtech.jts.index.strtree.ItemBoundable";

/// Source object-node identity, independent of the organism's wrapping numeric ID
pub(crate) struct ActorTokens<'a> {
    pub world: NodeId,
    pub nodes: Vec<NodeId>,
    by_node: HashMap<u32, usize>,
    arena: &'a Arena,
}

impl<'a> ActorTokens<'a> {
    pub(crate) fn new(graph: &LegacyGraph<'a>, world: NodeId) -> Self {
        Self {
            world,
            nodes: Vec::new(),
            by_node: HashMap::new(),
            arena: graph.arena(),
        }
    }

    pub(crate) fn token(
        &mut self,
        graph: &LegacyGraph<'a>,
        actor: NodeId,
    ) -> Result<usize, GraphError> {
        if !std::ptr::eq(self.arena, graph.arena()) {
            return Err(GraphError::at(actor, None, GraphErrorKind::Reference));
        }
        if let Some(token) = self.by_node.get(&actor.0) {
            return Ok(*token);
        }
        graph.expect_class(actor, "biogenesis.Organism")?;
        if graph.reference(actor, "biogenesis.Organism", "_world")? != Value::Handle(self.world) {
            return Err(GraphError::at(
                actor,
                Some("_world"),
                GraphErrorKind::Reference,
            ));
        }
        graph.allocate(std::mem::size_of::<NodeId>() + 2 * std::mem::size_of::<(u32, usize)>())?;
        self.nodes
            .try_reserve(1)
            .map_err(|_| GraphError::at(actor, None, GraphErrorKind::Allocation))?;
        self.by_node
            .try_reserve(1)
            .map_err(|_| GraphError::at(actor, None, GraphErrorKind::Allocation))?;
        let token = self.nodes.len();
        self.nodes.push(actor);
        self.by_node.insert(actor.0, token);
        Ok(token)
    }
}

pub(crate) struct DecodedSpatial<'a> {
    pub tree: SpatialTree,
    pub actors: ActorTokens<'a>,
    pub members: Vec<usize>,
}

struct Decoder<'g, 'a, 't> {
    graph: &'g LegacyGraph<'a>,
    actors: &'t mut ActorTokens<'a>,
    state: StoredTree,
    boundables: HashMap<u32, usize>,
    envelopes: HashMap<u32, usize>,
    active: HashSet<u32>,
    lists: HashMap<u32, Option<u32>>,
}

impl Decoder<'_, '_, '_> {
    fn envelope(&mut self, id: NodeId) -> Result<usize, GraphError> {
        if let Some(index) = self.envelopes.get(&id.0) {
            return Ok(*index);
        }
        self.graph
            .expect_class(id, "org.locationtech.jts.geom.Envelope")?;
        let value = Envelope {
            min_x: f64::from_bits(self.graph.double_bits(
                id,
                "org.locationtech.jts.geom.Envelope",
                "minx",
            )?),
            max_x: f64::from_bits(self.graph.double_bits(
                id,
                "org.locationtech.jts.geom.Envelope",
                "maxx",
            )?),
            min_y: f64::from_bits(self.graph.double_bits(
                id,
                "org.locationtech.jts.geom.Envelope",
                "miny",
            )?),
            max_y: f64::from_bits(self.graph.double_bits(
                id,
                "org.locationtech.jts.geom.Envelope",
                "maxy",
            )?),
        };
        self.graph
            .allocate(std::mem::size_of::<Envelope>() + 2 * std::mem::size_of::<(u32, usize)>())?;
        self.state
            .envelopes
            .try_reserve(1)
            .map_err(|_| GraphError::at(id, None, GraphErrorKind::Allocation))?;
        self.envelopes
            .try_reserve(1)
            .map_err(|_| GraphError::at(id, None, GraphErrorKind::Allocation))?;
        let index = self.state.envelopes.len();
        self.state.envelopes.push(value);
        self.envelopes.insert(id.0, index);
        Ok(index)
    }

    fn list(
        &mut self,
        id: NodeId,
        owner: Option<NodeId>,
        depth: usize,
    ) -> Result<Vec<usize>, GraphError> {
        let owner = owner.map(|value| value.0);
        if let Some(previous) = self.lists.get(&id.0) {
            if *previous != owner {
                return Err(GraphError::at(id, None, GraphErrorKind::AliasedTreeList));
            }
        } else {
            self.graph
                .allocate(2 * std::mem::size_of::<(u32, Option<u32>)>())?;
            self.lists
                .try_reserve(1)
                .map_err(|_| GraphError::at(id, None, GraphErrorKind::Allocation))?;
            self.lists.insert(id.0, owner);
        }
        let values = self.graph.array_list(id)?;
        self.graph.allocate(
            values
                .len()
                .checked_mul(std::mem::size_of::<usize>())
                .ok_or(GraphError::at(id, None, GraphErrorKind::Budget))?,
        )?;
        let mut children = Vec::new();
        children
            .try_reserve_exact(values.len())
            .map_err(|_| GraphError::at(id, None, GraphErrorKind::Allocation))?;
        for value in values {
            let Value::Handle(child) = value else {
                return Err(GraphError::at(id, None, GraphErrorKind::Reference));
            };
            children.push(self.boundable(child, depth)?);
        }
        Ok(children)
    }

    fn boundable(&mut self, id: NodeId, depth: usize) -> Result<usize, GraphError> {
        if depth > self.graph.limits().depth {
            return Err(GraphError::at(id, None, GraphErrorKind::Budget));
        }
        if self.active.contains(&id.0) {
            return Err(GraphError::at(id, None, GraphErrorKind::TreeShape));
        }
        if let Some(index) = self.boundables.get(&id.0) {
            return Ok(*index);
        }
        let class = self.graph.class_name(id)?;
        if !matches!(
            class,
            "org.locationtech.jts.index.strtree.STRtree$STRtreeNode" | ITEM
        ) {
            return Err(GraphError::at(id, None, GraphErrorKind::Class));
        }
        self.graph.allocate(
            std::mem::size_of::<StoredBoundable>() + 4 * std::mem::size_of::<(u32, usize)>(),
        )?;
        self.state
            .boundables
            .try_reserve(1)
            .map_err(|_| GraphError::at(id, None, GraphErrorKind::Allocation))?;
        self.boundables
            .try_reserve(1)
            .map_err(|_| GraphError::at(id, None, GraphErrorKind::Allocation))?;
        self.active
            .try_reserve(1)
            .map_err(|_| GraphError::at(id, None, GraphErrorKind::Allocation))?;
        let index = self.state.boundables.len();
        self.boundables.insert(id.0, index);
        self.active.insert(id.0);
        if class == ITEM {
            let envelope = self.graph.required_reference(id, ITEM, "bounds")?;
            let envelope = self.envelope(envelope)?;
            let actor = self.graph.required_reference(id, ITEM, "item")?;
            let token = self.actors.token(self.graph, actor)?;
            self.state
                .boundables
                .push(StoredBoundable::Item { envelope, token });
        } else {
            let level = self.graph.int(id, NODE, "level")?;
            let envelope = match self.graph.reference(id, NODE, "bounds")? {
                Value::Null => None,
                Value::Handle(envelope) => Some(self.envelope(envelope)?),
            };
            self.state.boundables.push(StoredBoundable::Node {
                level,
                envelope,
                children: Vec::new(),
            });
            let list = self.graph.required_reference(id, NODE, "childBoundables")?;
            let children = self.list(list, Some(id), depth + 1)?;
            if let StoredBoundable::Node {
                children: stored, ..
            } = &mut self.state.boundables[index]
            {
                *stored = children;
            }
        }
        self.active.remove(&id.0);
        Ok(index)
    }
}

pub(crate) fn decode_spatial<'a>(
    graph: &LegacyGraph<'a>,
    world: NodeId,
    actors: &mut ActorTokens<'a>,
) -> Result<SpatialTree, GraphError> {
    graph.expect_class(world, WORLD)?;
    if actors.world != world || !std::ptr::eq(actors.arena, graph.arena()) {
        return Err(GraphError::at(world, None, GraphErrorKind::Reference));
    }
    let tree = graph.required_reference(world, WORLD, "colDetTree")?;
    graph.expect_class(tree, "org.locationtech.jts.index.strtree.STRtree")?;
    let capacity = graph.int(tree, TREE, "nodeCapacity")?;
    let built = graph.boolean(tree, TREE, "built")?;
    let mut decoder = Decoder {
        graph,
        actors,
        state: StoredTree::empty(capacity),
        boundables: HashMap::new(),
        envelopes: HashMap::new(),
        active: HashSet::new(),
        lists: HashMap::new(),
    };
    decoder.state.built = built;
    decoder.state.pending = match graph.reference(tree, TREE, "itemBoundables")? {
        Value::Null => None,
        Value::Handle(list) => Some(decoder.list(list, None, 0)?),
    };
    decoder.state.root = match graph.reference(tree, TREE, "root")? {
        Value::Null => None,
        Value::Handle(root) => Some(decoder.boundable(root, 0)?),
    };
    SpatialTree::from_stored(
        decoder.state,
        graph.limits().depth,
        graph.limits().array_elements,
    )
    .map_err(|error| {
        GraphError::at(
            tree,
            Some(match error {
                StoredTreeError::Capacity => "nodeCapacity",
                StoredTreeError::Pending | StoredTreeError::NonfinitePacking => "itemBoundables",
                _ => "root",
            }),
            match error {
                StoredTreeError::Allocation => GraphErrorKind::Allocation,
                StoredTreeError::Depth | StoredTreeError::QueryExpansion => GraphErrorKind::Budget,
                _ => GraphErrorKind::TreeShape,
            },
        )
    })
}

pub(crate) fn decode_world_spatial<'a>(
    graph: &LegacyGraph<'a>,
) -> Result<DecodedSpatial<'a>, GraphError> {
    let world = graph.world_root()?;
    let list = graph.required_reference(world, WORLD, "_organisms")?;
    let (_, members) = graph.synchronized_list(list)?;
    let mut actors = ActorTokens::new(graph, world);
    let mut tokens = Vec::new();
    graph.allocate(
        members
            .len()
            .checked_mul(std::mem::size_of::<usize>())
            .ok_or(GraphError::at(
                world,
                Some("_organisms"),
                GraphErrorKind::Budget,
            ))?,
    )?;
    tokens
        .try_reserve_exact(members.len())
        .map_err(|_| GraphError::at(world, None, GraphErrorKind::Allocation))?;
    for value in members {
        let Value::Handle(actor) = value else {
            return Err(GraphError::at(
                world,
                Some("_organisms"),
                GraphErrorKind::Reference,
            ));
        };
        tokens.push(actors.token(graph, actor)?);
    }
    let tree = decode_spatial(graph, world, &mut actors)?;
    Ok(DecodedSpatial {
        tree,
        actors,
        members: tokens,
    })
}
