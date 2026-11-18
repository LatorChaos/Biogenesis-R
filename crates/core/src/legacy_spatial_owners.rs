// SPDX-License-Identifier: GPL-2.0-or-later

//! Bound admitted live query expansion without building the tree or casting raw list slots

use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::shared_world_owners::{self as owners, BoundableRef, BoundableValue, SharedListRef, SharedSlot, TreeRef};
use std::collections::HashMap;

type Shape = (usize, usize);
fn failure(kind: GraphErrorKind) -> GraphError { GraphError { node: None, field: None, kind } }
fn node(value: &BoundableRef, depth: usize, graph: &LegacyGraph<'_>, seen: &mut HashMap<usize, Option<Shape>>) -> Result<Shape, GraphError> {
    graph.charge(1)?;
    if depth > graph.limits().depth { return Err(failure(GraphErrorKind::Budget)); }
    if let Some(cached) = seen.get(&value.identity()) {
        let (count,height) = cached.ok_or(failure(GraphErrorKind::TreeShape))?;
        if depth.checked_add(height).is_none_or(|n|n>graph.limits().depth) { return Err(failure(GraphErrorKind::Budget)); }
        return Ok((count,height));
    }
    owners::reserve_map(graph,seen)?;
    seen.insert(value.identity(),None);
    let shape = match &*value.value() {
        BoundableValue::Item { .. } => (1,0),
        BoundableValue::Node { children: Some(children),.. } => list(children,depth,graph,seen)?,
        BoundableValue::Node { children: None,.. } => (0,0),
    };
    seen.insert(value.identity(),Some(shape));
    Ok(shape)
}
fn list(value: &SharedListRef, depth: usize, graph: &LegacyGraph<'_>, seen: &mut HashMap<usize, Option<Shape>>) -> Result<Shape, GraphError> {
    let mut count=0_usize;let mut height=0_usize;
    for slot in value.slots().iter() {
        graph.charge(1)?;
        let (added,child_height)=if let SharedSlot::Boundable(value)=slot { node(value,depth.checked_add(1).ok_or(failure(GraphErrorKind::Budget))?,graph,seen)? } else { (1,0) };
        count=count.checked_add(added).ok_or(failure(GraphErrorKind::Budget))?;
        height=height.max(child_height.checked_add(1).ok_or(failure(GraphErrorKind::Budget))?);
        if count>graph.limits().array_elements { return Err(failure(GraphErrorKind::Budget)); }
    }
    Ok((count,height))
}
pub(crate) fn validate(graph: &LegacyGraph<'_>, tree: &TreeRef) -> Result<(),GraphError> {
    let mut seen=HashMap::new();
    let tree=tree.value();
    if let Some(pending)=&tree.pending { list(pending,0,graph,&mut seen)?; }
    if let Some(root)=&tree.root { node(root,0,graph,&mut seen)?; }
    Ok(())
}
