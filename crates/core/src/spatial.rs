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
// Adapted from JTS STRtree/AbstractSTRtree/Envelope under the Eclipse Distribution License 1.0
// Packing and traversal were checked against the embedded JTS 1.15.0 bytecode and actual queries

use crate::IntRect;
use crate::organism::{Organism, OrganismRef};
use crate::shared_world_owners::{BoundableRef, BoundableValue, EnvelopeRef, SharedListRef, SharedSlot, TreeRef, TreeValue};
use std::{cell::{OnceCell, RefCell}, rc::{Rc, Weak}};

#[path = "spatial_state.rs"]
mod state;
pub(crate) use state::{Envelope, StoredBoundable, StoredTree, StoredTreeError};
#[path = "spatial_owners.rs"]
mod owners;
pub use owners::SpatialAccessError;

/// Live tree references share their actual lists, boundables and Envelope caches
#[derive(Debug)]
pub(crate) struct SpatialTree {
    owner: OnceCell<TreeRef>,
    // Weak ordinal bindings preserve legacy snapshot tokens without retaining removed items
    tokens: RefCell<Vec<Weak<RefCell<Organism>>>>,
    imported: bool,
}
impl SpatialTree {
    pub(crate) fn pending() -> Self { Self { owner: OnceCell::new(), tokens: RefCell::new(Vec::new()), imported: true } }
    pub(crate) fn empty() -> Self {
        Self::from_shared(TreeRef::new(TreeValue { built: false, node_capacity: 10, pending: Some(SharedListRef::from_slots(Vec::new())), root: None }), Vec::new(), false)
    }
    pub(crate) fn from_shared(owner: TreeRef, tokens: Vec<Weak<RefCell<Organism>>>, imported: bool) -> Self {
        let mut result = Self::pending(); result.imported = imported; result.initialize(owner, tokens); result
    }
    pub(crate) fn initialize(&self, owner: TreeRef, tokens: Vec<Weak<RefCell<Organism>>>) {
        assert!(self.owner.set(owner).is_ok(), "Spatial owner is initialized once before publication");
        self.tokens.replace(tokens);
    }
    pub(crate) fn shared_ref(&self) -> &TreeRef { self.owner.get().expect("Spatial owner is filled before publication") }
    pub(crate) fn token_bindings(&self) -> Vec<Option<OrganismRef>> { self.tokens.borrow().iter().map(Weak::upgrade).collect() }
    pub(crate) fn token_capacity(&self) -> usize { self.tokens.borrow().capacity() }
    pub(crate) fn imported(&self) -> bool { self.imported }
    pub(crate) fn token_actor(&self, token: usize) -> Option<OrganismRef> { self.tokens.borrow().get(token).and_then(Weak::upgrade) }
    pub(crate) fn insert_actor(&self, token: usize, actor: &OrganismRef, bounds: IntRect) -> Result<(), SpatialAccessError> {
        let envelope = Envelope::from_rect(bounds);
        let (built, pending) = { let value = self.shared_ref().value(); (value.built, value.pending.clone()) };
        if built { return Err(SpatialAccessError::Built); }
        if envelope.is_null() { return Ok(()); }
        let pending = pending.ok_or(SpatialAccessError::List { operation: "insert", error: crate::ListAccessError::MissingList { field: "itemBoundables" } })?;
        let needed = token.checked_add(1).ok_or(SpatialAccessError::Capacity { operation: "insert" })?;
        let mut tokens = self.tokens.borrow_mut();
        let additional = needed.saturating_sub(tokens.len());
        tokens.try_reserve_exact(additional).map_err(|_| SpatialAccessError::Capacity { operation: "insert" })?;
        pending.push_boundable("itemBoundables", BoundableRef::new(BoundableValue::Item { bounds: Some(EnvelopeRef::new(envelope)), item: SharedSlot::Actor(Rc::clone(actor)) }))
            .map_err(|error| SpatialAccessError::List { operation: "insert", error })?;
        if tokens.len() < needed { tokens.resize_with(needed, Weak::new); }
        tokens[token] = Rc::downgrade(actor);
        Ok(())
    }
    pub(crate) fn equal_with_actor(&self, other: &Self, actor_equal: impl FnMut(&OrganismRef, &OrganismRef) -> bool) -> bool {
        crate::shared_world_owners::OwnerEquality::new(actor_equal).tree(self.shared_ref(), other.shared_ref())
    }
    pub(crate) fn build_index(&self) -> Result<(), SpatialAccessError> { owners::build(self.shared_ref()) }
    pub(crate) fn query_slots(&self, bounds: IntRect) -> Result<Vec<SharedSlot>, SpatialAccessError> { owners::query(self.shared_ref(), Envelope::from_rect(bounds)) }

    #[cfg(test)]
    pub(crate) fn build(items: impl IntoIterator<Item = (usize, IntRect)>) -> Self {
        let mut parts = StoredTree::empty(10);
        for (token, bounds) in items { parts.insert(token, Envelope::from_rect(bounds)); }
        Self::from_stored(parts, usize::MAX, usize::MAX).unwrap()
    }
    #[cfg(test)]
    pub(crate) fn build_actor_entries(items: impl IntoIterator<Item = (usize, IntRect)>, mut resolve: impl FnMut(usize) -> OrganismRef) -> Self {
        let result = Self::empty();
        for (token, bounds) in items { result.insert_actor(token, &resolve(token), bounds).unwrap(); }
        result
    }
    #[cfg(test)]
    pub(crate) fn query(&self, bounds: IntRect) -> Vec<usize> {
        self.query_slots(bounds).unwrap().into_iter().map(|item| self.snapshot_token(&item)).collect()
    }
    #[cfg(test)]
    fn snapshot_token(&self, item: &SharedSlot) -> usize {
        match item {
            SharedSlot::Double(value) => value.bits() as usize,
            SharedSlot::Actor(actor) => self.tokens.borrow().iter().position(|value| value.ptr_eq(&Rc::downgrade(actor))).expect("A snapshot Actor has its original ordinal"),
            _ => panic!("A numeric-token fixture or admitted Actor item is required"),
        }
    }
}

#[cfg(test)]
#[path = "spatial_snapshot.rs"]
mod snapshot;

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle(fields: &[&str]) -> IntRect {
        IntRect {
            x: fields[0].parse().unwrap(),
            y: fields[1].parse().unwrap(),
            width: fields[2].parse().unwrap(),
            height: fields[3].parse().unwrap(),
        }
    }

    fn check_envelope(bounds: IntRect, expected: &[&str]) {
        let envelope = Envelope::from_rect(bounds);
        for (actual, expected) in [
            envelope.min_x,
            envelope.max_x,
            envelope.min_y,
            envelope.max_y,
        ]
        .into_iter()
        .zip(expected)
        {
            assert_eq!(actual.to_bits(), u64::from_str_radix(expected, 16).unwrap());
        }
    }

    #[test]
    fn frozen_spatial_queries_match_bundled_jts_order_and_envelopes() {
        let mut items = Vec::new();
        let mut tree = None;
        let mut name = "";
        let mut cases = 0;
        let mut queries = 0;
        for line in include_str!("../tests/fixtures/spatial.tsv").lines() {
            let fields: Vec<_> = line.split('\t').collect();
            match fields[0] {
                "M" => assert_eq!(&fields[1..], &["JTSVersion", "1.15.0"]),
                "C" => {
                    assert!(items.is_empty());
                    assert!(tree.is_none());
                    name = fields[1];
                }
                "I" => {
                    assert!(tree.is_none());
                    let bounds = rectangle(&fields[2..6]);
                    check_envelope(bounds, &fields[6..10]);
                    items.push((fields[1].parse().unwrap(), bounds));
                }
                "Q" => {
                    let tree =
                        tree.get_or_insert_with(|| SpatialTree::build(items.iter().copied()));
                    let bounds = rectangle(&fields[2..6]);
                    check_envelope(bounds, &fields[6..10]);
                    let expected: Vec<usize> = if fields[10] == "~" {
                        Vec::new()
                    } else {
                        fields[10]
                            .split(',')
                            .map(|value| value.parse().unwrap())
                            .collect()
                    };
                    let actual = tree.query(bounds);
                    assert_eq!(actual, expected, "{name} {}", fields[1]);
                    assert_eq!(
                        tree.query(bounds),
                        actual,
                        "Repeated queries must remain immutable"
                    );
                    queries += 1;
                }
                "E" => {
                    assert!(tree.is_some());
                    tree = None;
                    items.clear();
                    cases += 1;
                }
                record => panic!("Unknown spatial observation {record}"),
            }
        }
        assert!(tree.is_none());
        assert!(items.is_empty());
        assert_eq!((cases, queries), (113, 3740));
    }

    #[test]
    fn tree_owns_frozen_envelopes_and_keeps_duplicate_tokens() {
        let mut supplied = [(
            7,
            IntRect {
                x: 10,
                y: 20,
                width: 0,
                height: 0,
            },
        ); 2];
        let tree = SpatialTree::build(supplied);
        supplied[0].1.x = 100;
        assert_eq!(
            tree.query(IntRect {
                x: 10,
                y: 20,
                width: 0,
                height: 0
            }),
            [7, 7]
        );
        assert!(tree.query(supplied[0].1).is_empty());
        assert_eq!(tree.clone(), tree);
    }
}

#[cfg(test)]
#[path = "spatial_persistence_tests.rs"]
mod persistence_tests;
