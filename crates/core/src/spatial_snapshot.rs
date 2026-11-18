// SPDX-License-Identifier: GPL-2.0-or-later

//! Detached token DTOs keep the existing spatial fixture assertions independent of live owners

use super::*;
use crate::history::JavaDoubleRef;
use std::collections::HashMap;

impl SpatialTree {
    pub(crate) fn from_stored(parts: StoredTree, depth: usize, expansion: usize) -> Result<Self, StoredTreeError> {
        parts.validate(depth, expansion)?;
        let envelopes: Vec<_> = parts.envelopes.iter().copied().map(EnvelopeRef::new).collect();
        fn copied(id: usize, parts: &StoredTree, envelopes: &[EnvelopeRef], cache: &mut [Option<BoundableRef>]) -> BoundableRef {
            if let Some(value) = &cache[id] { return value.clone(); }
            let value = match &parts.boundables[id] {
                StoredBoundable::Item { envelope, token } => BoundableValue::Item {
                    bounds: Some(envelopes[*envelope].clone()), item: SharedSlot::Double(JavaDoubleRef::from_bits(*token as u64)),
                },
                StoredBoundable::Node { level, envelope, children } => BoundableValue::Node {
                    level: *level, bounds: envelope.map(|id| envelopes[id].clone()),
                    children: Some(SharedListRef::from_slots(children.iter().map(|id| SharedSlot::Boundable(copied(*id, parts, envelopes, cache))).collect())),
                },
            };
            let value = BoundableRef::new(value); cache[id] = Some(value.clone()); value
        }
        let mut cache = vec![None; parts.boundables.len()];
        let pending = parts.pending.as_ref().map(|ids| SharedListRef::from_slots(ids.iter().map(|id| SharedSlot::Boundable(copied(*id, &parts, &envelopes, &mut cache))).collect()));
        let root = parts.root.map(|id| copied(id, &parts, &envelopes, &mut cache));
        Ok(Self::from_shared(TreeRef::new(TreeValue { built: parts.built, node_capacity: parts.node_capacity, pending, root }), Vec::new(), false))
    }
    pub(crate) fn stored_state(&self) -> StoredTree {
        struct Snapshot<'a> {
            owner: &'a SpatialTree, parts: StoredTree,
            boundables: HashMap<usize, usize>, envelopes: HashMap<usize, usize>,
        }
        impl Snapshot<'_> {
            fn envelope(&mut self, value: &EnvelopeRef) -> usize {
                if let Some(id) = self.envelopes.get(&value.identity()) { return *id; }
                let id = self.parts.envelopes.len(); self.envelopes.insert(value.identity(), id); self.parts.envelopes.push(value.get()); id
            }
            fn list(&mut self, value: &SharedListRef) -> Vec<usize> {
                value.slots().iter().map(|value| {
                    let SharedSlot::Boundable(value) = value else { panic!("A structural DTO cannot cast a mixed spatial list"); };
                    self.boundable(value)
                }).collect()
            }
            fn boundable(&mut self, value: &BoundableRef) -> usize {
                if let Some(id) = self.boundables.get(&value.identity()) { return *id; }
                let id = self.parts.boundables.len(); self.boundables.insert(value.identity(), id);
                self.parts.boundables.push(StoredBoundable::Node { level: 0, envelope: None, children: Vec::new() });
                let copied = match &*value.value() {
                    BoundableValue::Item { bounds, item } => StoredBoundable::Item {
                        envelope: self.envelope(bounds.as_ref().expect("A structural item fixture has bounds")),
                        token: self.owner.snapshot_token(item),
                    },
                    BoundableValue::Node { level, bounds, children } => StoredBoundable::Node {
                        level: *level, envelope: bounds.as_ref().map(|value| self.envelope(value)),
                        children: self.list(children.as_ref().expect("A structural node fixture has children")),
                    },
                };
                self.parts.boundables[id] = copied; id
            }
        }
        let value = self.shared_ref().value();
        let mut snapshot = Snapshot { owner: self, parts: StoredTree::empty(value.node_capacity), boundables: HashMap::new(), envelopes: HashMap::new() };
        snapshot.parts.built = value.built;
        snapshot.parts.pending = value.pending.as_ref().map(|value| snapshot.list(value));
        snapshot.parts.root = value.root.as_ref().map(|value| snapshot.boundable(value));
        snapshot.parts
    }
    pub(crate) fn stored_envelopes(&self) -> Vec<(usize, [f64; 4])> { self.stored_state().stored_envelopes() }
}
impl PartialEq for SpatialTree { fn eq(&self, other: &Self) -> bool { self.stored_state() == other.stored_state() } }
impl PartialEq<StoredTree> for SpatialTree { fn eq(&self, other: &StoredTree) -> bool { self.stored_state() == *other } }

impl Clone for SpatialTree {
    fn clone(&self) -> Self { Self::from_stored(self.stored_state(),usize::MAX,usize::MAX).expect("A typed token fixture snapshot remains valid") }
}
