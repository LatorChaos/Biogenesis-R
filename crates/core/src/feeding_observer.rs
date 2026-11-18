// SPDX-License-Identifier: GPL-2.0-or-later

//! One presentation-only positive feeding stamp per frontend advance batch

use super::*;
use std::{cell::Cell, rc::Weak};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeedingSubject {
    pub id: i64,
    pub center: [f64; 2],
    pub bounds: IntRect,
}

impl FeedingSubject {
    fn from_actor(actor: &Organism) -> Self {
        Self {
            id: actor.id,
            center: actor.state.center,
            bounds: actor.state.bounds.get(),
        }
    }
}

/// Gross energy moved before respiration or decomposition costs, not net predator gain
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeedingStamp {
    pub batch: u64,
    pub tick: u64,
    pub predator: FeedingSubject,
    pub prey: FeedingSubject,
    pub gross_energy: f64,
}

#[derive(Debug)]
pub(super) struct FeedingState {
    owner: WorldAssociation,
    batch: Cell<u64>,
    stamp: Cell<Option<FeedingStamp>>,
}

/// The frontend owns this readout; the World retains only a weak observer
#[derive(Clone, Debug)]
pub struct FeedingReadout(Rc<FeedingState>);

impl FeedingReadout {
    pub fn is_current_for(&self, world: &World) -> bool {
        self.0.owner == world.association()
            && world
                .feeding_observer
                .borrow()
                .upgrade()
                .is_some_and(|current| Rc::ptr_eq(&current, &self.0))
    }

    /// Clear the previous stamp before a frontend batch, without changing biological state
    pub fn begin_batch(&self) {
        self.0.batch.set(self.0.batch.get().saturating_add(1));
        self.0.stamp.set(None);
    }

    pub fn snapshot(&self, world: &World) -> Option<FeedingStamp> {
        self.is_current_for(world)
            .then(|| self.0.stamp.get())
            .flatten()
    }
}

impl World {
    pub fn observe_feeding(&self) -> FeedingReadout {
        let state = Rc::new(FeedingState {
            owner: self.association(),
            batch: Cell::new(0),
            stamp: Cell::new(None),
        });
        self.feeding_observer.replace(Rc::downgrade(&state));
        FeedingReadout(state)
    }

    pub(super) fn invalidate_feeding(&self) {
        self.feeding_observer.replace(Weak::new());
    }

    pub(super) fn notify_feeding(&self, actor: &Organism, peer: &Organism, amount: f64) {
        if !actor.state.alive || !peer.state.alive || !(amount > 0.0) {
            return;
        }
        let Some(state) = self.feeding_observer.borrow().upgrade() else {
            return;
        };
        state.stamp.set(Some(FeedingStamp {
            batch: state.batch.get(),
            tick: self.tick(),
            predator: FeedingSubject::from_actor(actor),
            prey: FeedingSubject::from_actor(peer),
            gross_energy: amount,
        }));
    }
}
