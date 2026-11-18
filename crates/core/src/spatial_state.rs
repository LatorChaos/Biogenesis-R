/*
 * Copyright (c) 2016 Vivid Solutions.
 * Copyright (c) 2007, Eclipse Foundation, Inc. and its licensors.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * Redistributions of source code must retain the above copyright notice, this
 * list of conditions and the following disclaimer.
 * Redistributions in binary form must reproduce the above copyright notice,
 * this list of conditions and the following disclaimer in the documentation
 * and/or other materials provided with the distribution.
 *
 * Neither the name of the Eclipse Foundation, Inc. nor the names of its
 * contributors may be used to endorse or promote products derived from this
 * software without specific prior written permission.
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR
 * ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
 * (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
 * ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
// SPDX-License-Identifier: BSD-3-Clause
// Stored packing and lazy cache semantics follow the bundled JTS 1.15.0 implementation

use crate::IntRect;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Envelope {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

impl Envelope {
    pub(crate) fn from_rect(bounds: IntRect) -> Self {
        let x = f64::from(bounds.x);
        let y = f64::from(bounds.y);
        let right = x + f64::from(bounds.width);
        let bottom = y + f64::from(bounds.height);
        Self {
            min_x: x.min(right),
            max_x: x.max(right),
            min_y: y.min(bottom),
            max_y: y.max(bottom),
        }
    }

    pub(super) fn is_null(self) -> bool {
        self.max_x < self.min_x
    }

    pub(super) fn intersects(self, other: Self) -> bool {
        !self.is_null()
            && !other.is_null()
            && !(other.min_x > self.max_x
                || other.max_x < self.min_x
                || other.min_y > self.max_y
                || other.max_y < self.min_y)
    }

    pub(super) fn include(&mut self, other: Self) {
        if other.is_null() {
            return;
        }
        if self.is_null() {
            *self = other;
            return;
        }
        if other.min_x < self.min_x {
            self.min_x = other.min_x;
        }
        if other.max_x > self.max_x {
            self.max_x = other.max_x;
        }
        if other.min_y < self.min_y {
            self.min_y = other.min_y;
        }
        if other.max_y > self.max_y {
            self.max_y = other.max_y;
        }
    }
}

/// Indices preserve boundable/envelope identity without sharing storage across World clones
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum StoredBoundable {
    Item {
        envelope: usize,
        token: usize,
    },
    Node {
        level: i32,
        envelope: Option<usize>,
        children: Vec<usize>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StoredTree {
    pub built: bool,
    pub node_capacity: i32,
    pub pending: Option<Vec<usize>>,
    pub root: Option<usize>,
    pub boundables: Vec<StoredBoundable>,
    pub envelopes: Vec<Envelope>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StoredTreeError {
    Capacity,
    Reference,
    Root,
    Pending,
    Cycle,
    Depth,
    QueryExpansion,
    EmptyChild,
    Allocation,
    NonfinitePacking,
}

impl StoredTree {
    pub(crate) fn empty(node_capacity: i32) -> Self {
        Self {
            built: false,
            node_capacity,
            pending: Some(Vec::new()),
            root: None,
            boundables: Vec::new(),
            envelopes: Vec::new(),
        }
    }

    pub(crate) fn insert(&mut self, token: usize, envelope: Envelope) {
        assert!(!self.built, "Cannot insert into a built STRtree");
        if envelope.is_null() {
            return;
        }
        let bounds = self.envelopes.len();
        self.envelopes.push(envelope);
        let item = self.boundables.len();
        self.boundables.push(StoredBoundable::Item {
            envelope: bounds,
            token,
        });
        self.pending
            .as_mut()
            .expect("Unbuilt tree retains pending items")
            .push(item);
    }

    pub(crate) fn validate(
        &self,
        maximum_depth: usize,
        maximum_expansion: usize,
    ) -> Result<(), StoredTreeError> {
        if self.node_capacity <= 1 {
            return Err(StoredTreeError::Capacity);
        }
        if self.built && self.root.is_none() {
            return Err(StoredTreeError::Root);
        }
        if !self.built && self.pending.is_none() {
            return Err(StoredTreeError::Pending);
        }
        let mut colors = Vec::new();
        let mut counts = Vec::new();
        let mut heights = Vec::new();
        colors
            .try_reserve_exact(self.boundables.len())
            .map_err(|_| StoredTreeError::Allocation)?;
        counts
            .try_reserve_exact(self.boundables.len())
            .map_err(|_| StoredTreeError::Allocation)?;
        heights
            .try_reserve_exact(self.boundables.len())
            .map_err(|_| StoredTreeError::Allocation)?;
        colors.resize(self.boundables.len(), 0_u8);
        counts.resize(self.boundables.len(), 0_usize);
        heights.resize(self.boundables.len(), 0_usize);
        fn visit(
            tree: &StoredTree,
            id: usize,
            depth: usize,
            limit: usize,
            expansion: usize,
            colors: &mut [u8],
            counts: &mut [usize],
            heights: &mut [usize],
        ) -> Result<usize, StoredTreeError> {
            if depth > limit {
                return Err(StoredTreeError::Depth);
            }
            let node = tree.boundables.get(id).ok_or(StoredTreeError::Reference)?;
            if colors[id] == 1 {
                return Err(StoredTreeError::Cycle);
            }
            if colors[id] == 2 {
                if depth.saturating_add(heights[id]) > limit {
                    return Err(StoredTreeError::Depth);
                }
                return Ok(counts[id]);
            }
            colors[id] = 1;
            let count = match node {
                StoredBoundable::Item { envelope, .. } => {
                    tree.envelopes
                        .get(*envelope)
                        .ok_or(StoredTreeError::Reference)?;
                    1
                }
                StoredBoundable::Node {
                    envelope, children, ..
                } => {
                    if let Some(envelope) = envelope {
                        tree.envelopes
                            .get(*envelope)
                            .ok_or(StoredTreeError::Reference)?;
                    }
                    let mut count = 0_usize;
                    for child in children {
                        if let Some(StoredBoundable::Node {
                            envelope: None,
                            children,
                            ..
                        }) = tree.boundables.get(*child)
                        {
                            if children.is_empty() {
                                return Err(StoredTreeError::EmptyChild);
                            }
                        }
                        count = count
                            .checked_add(visit(
                                tree,
                                *child,
                                depth + 1,
                                limit,
                                expansion,
                                colors,
                                counts,
                                heights,
                            )?)
                            .ok_or(StoredTreeError::QueryExpansion)?;
                        heights[id] = heights[id].max(heights[*child].saturating_add(1));
                        if count > expansion {
                            return Err(StoredTreeError::QueryExpansion);
                        }
                    }
                    count
                }
            };
            colors[id] = 2;
            counts[id] = count;
            Ok(count)
        }
        if let Some(pending) = &self.pending {
            if pending.len() > maximum_expansion {
                return Err(StoredTreeError::QueryExpansion);
            }
            for item in pending {
                if !matches!(
                    self.boundables.get(*item),
                    Some(StoredBoundable::Item { .. })
                ) {
                    return Err(StoredTreeError::Pending);
                }
                visit(
                    self,
                    *item,
                    0,
                    maximum_depth,
                    maximum_expansion,
                    &mut colors,
                    &mut counts,
                    &mut heights,
                )?;
                if !self.built {
                    if let StoredBoundable::Item { envelope, .. } = self.boundables[*item] {
                        let value = self.envelopes[envelope];
                        if [value.min_x, value.max_x, value.min_y, value.max_y]
                            .iter()
                            .any(|value| !value.is_finite())
                        {
                            return Err(StoredTreeError::NonfinitePacking);
                        }
                    }
                }
            }
        }
        if let Some(root) = self.root {
            if !matches!(
                self.boundables.get(root),
                Some(StoredBoundable::Node { .. })
            ) {
                return Err(StoredTreeError::Root);
            }
            visit(
                self,
                root,
                0,
                maximum_depth,
                maximum_expansion,
                &mut colors,
                &mut counts,
                &mut heights,
            )?;
        }
        Ok(())
    }

    fn bounds(&mut self, id: usize) -> Option<usize> {
        match &self.boundables[id] {
            StoredBoundable::Item { envelope, .. } => return Some(*envelope),
            StoredBoundable::Node {
                envelope: Some(envelope),
                ..
            } => return Some(*envelope),
            StoredBoundable::Node { .. } => {}
        }
        let count = match &self.boundables[id] {
            StoredBoundable::Node { children, .. } => children.len(),
            _ => unreachable!(),
        };
        let mut envelope: Option<Envelope> = None;
        for index in 0..count {
            let child = match &self.boundables[id] {
                StoredBoundable::Node { children, .. } => children[index],
                _ => unreachable!(),
            };
            let bounds = self
                .bounds(child)
                .expect("Validated nonempty child has bounds");
            let child = self.envelopes[bounds];
            if let Some(envelope) = &mut envelope {
                envelope.include(child);
            } else {
                envelope = Some(child);
            }
        }
        envelope.map(|envelope| {
            let index = self.envelopes.len();
            self.envelopes.push(envelope);
            if let StoredBoundable::Node { envelope, .. } = &mut self.boundables[id] {
                *envelope = Some(index);
            }
            index
        })
    }

    fn compare(&mut self, left: usize, right: usize, horizontal: bool) -> Ordering {
        let left = self
            .bounds(left)
            .expect("Packing contains only nonempty boundables");
        let right = self
            .bounds(right)
            .expect("Packing contains only nonempty boundables");
        let a = self.envelopes[left];
        let b = self.envelopes[right];
        let a = if horizontal {
            (a.min_x + a.max_x) / 2.0
        } else {
            (a.min_y + a.max_y) / 2.0
        };
        let b = if horizontal {
            (b.min_x + b.max_x) / 2.0
        } else {
            (b.min_y + b.max_y) / 2.0
        };
        if a > b {
            Ordering::Greater
        } else if a < b {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }

    fn parent_level(&mut self, mut children: Vec<usize>, level: i32) -> Vec<usize> {
        let capacity = self.node_capacity as usize;
        let minimum_leaves = children.len().div_ceil(capacity);
        let slices = (minimum_leaves as f64).sqrt().ceil() as usize;
        let slice_capacity = children.len().div_ceil(slices);
        children.sort_by(|a, b| self.compare(*a, *b, true));
        let mut input = children.into_iter();
        let mut parents = Vec::new();
        for _ in 0..slices {
            let mut slice: Vec<_> = input.by_ref().take(slice_capacity).collect();
            slice.sort_by(|a, b| self.compare(*a, *b, false));
            let mut slice = slice.into_iter();
            loop {
                let group: Vec<_> = slice.by_ref().take(capacity).collect();
                if group.is_empty() {
                    break;
                }
                let id = self.boundables.len();
                self.boundables.push(StoredBoundable::Node {
                    level,
                    envelope: None,
                    children: group,
                });
                parents.push(id);
            }
        }
        parents
    }

    pub(crate) fn build_index(&mut self) {
        if self.built {
            return;
        }
        let mut current = self
            .pending
            .take()
            .expect("Unbuilt tree retains pending items");
        let root = if current.is_empty() {
            let id = self.boundables.len();
            self.boundables.push(StoredBoundable::Node {
                level: 0,
                envelope: None,
                children: Vec::new(),
            });
            id
        } else {
            let mut level = 0_i32;
            loop {
                current = self.parent_level(current, level);
                if current.len() == 1 {
                    break current[0];
                }
                level = level.wrapping_add(1);
            }
        };
        self.root = Some(root);
        self.built = true;
    }

    fn query_node(&mut self, node: usize, search: Envelope, output: &mut Vec<usize>) {
        let count = match &self.boundables[node] {
            StoredBoundable::Node { children, .. } => children.len(),
            _ => unreachable!(),
        };
        for index in 0..count {
            let child = match &self.boundables[node] {
                StoredBoundable::Node { children, .. } => children[index],
                _ => unreachable!(),
            };
            let bounds = self
                .bounds(child)
                .expect("Validated query child has bounds");
            if !self.envelopes[bounds].intersects(search) {
                continue;
            }
            match self.boundables[child] {
                StoredBoundable::Item { token, .. } => output.push(token),
                StoredBoundable::Node { .. } => self.query_node(child, search, output),
            }
        }
    }

    pub(crate) fn query(&mut self, search: Envelope) -> Vec<usize> {
        self.build_index();
        let root = self.root.expect("Built tree retains root");
        let mut result = Vec::new();
        if matches!(&self.boundables[root], StoredBoundable::Node { children, .. } if children.is_empty())
        {
            return result;
        }
        let envelope = self.bounds(root).expect("Nonempty root has bounds");
        if self.envelopes[envelope].intersects(search) {
            self.query_node(root, search, &mut result);
        }
        result
    }

    #[cfg(test)]
    pub(crate) fn stored_envelopes(&self) -> Vec<(usize, [f64; 4])> {
        fn visit(tree: &StoredTree, id: usize, output: &mut Vec<(usize, [f64; 4])>) {
            match &tree.boundables[id] {
                StoredBoundable::Item { envelope, token } => {
                    let value = tree.envelopes[*envelope];
                    output.push((
                        *token,
                        [
                            value.min_x,
                            value.min_y,
                            value.max_x - value.min_x,
                            value.max_y - value.min_y,
                        ],
                    ));
                }
                StoredBoundable::Node { children, .. } => {
                    for child in children {
                        visit(tree, *child, output);
                    }
                }
            }
        }
        let mut output = Vec::new();
        if self.built {
            if let Some(root) = self.root {
                visit(self, root, &mut output);
            }
        } else if let Some(pending) = &self.pending {
            for item in pending {
                visit(self, *item, &mut output);
            }
        }
        output.sort_by_key(|(token, _)| *token);
        output
    }
}
