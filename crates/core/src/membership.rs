// SPDX-License-Identifier: GPL-2.0-or-later

//! Nullable membership role over the same mutable backing used by saved histories

use crate::organism::OrganismRef;
use crate::shared_world_owners::{ListAccessError, SharedListRef, SharedSlot};
use std::{cell::OnceCell, rc::Rc};

#[derive(Debug, Default)]
pub(crate) struct Membership { reference: OnceCell<Option<SharedListRef>> }
impl Membership {
    pub(crate) fn pending() -> Self { Self::default() }
    pub(crate) fn new(reference: Option<SharedListRef>) -> Self {
        let value = Self::pending();
        value.initialize(reference);
        value
    }
    pub(crate) fn empty(capacity: usize) -> Self {
        let backing = SharedListRef::from_slots(Vec::with_capacity(capacity));
        Self::new(Some(SharedListRef::synchronized(&backing).expect("A fresh ArrayList has a synchronized wrapper")))
    }
    pub(crate) fn initialize(&self, reference: Option<SharedListRef>) {
        assert!(self.reference.set(reference).is_ok(), "Membership reference is initialized once before publication");
    }
    pub(crate) fn shared_ref(&self) -> Option<&SharedListRef> {
        self.reference.get().expect("World membership is filled before publication").as_ref()
    }
    pub(crate) fn required(&self) -> Result<&SharedListRef, ListAccessError> {
        self.shared_ref().ok_or(ListAccessError::MissingList { field: "_organisms" })
    }
    pub(crate) fn len(&self) -> Result<usize, ListAccessError> { Ok(self.required()?.len()) }
    pub(crate) fn actor(&self, index: usize) -> Result<OrganismRef, ListAccessError> {
        self.required()?.actor("_organisms", index)?.ok_or(ListAccessError::MissingElement { field: "_organisms", index })
    }
    pub(crate) fn push(&self, actor: OrganismRef) -> Result<(), ListAccessError> { self.required()?.push_actor("_organisms", actor) }
    pub(crate) fn clear(&self) -> Result<(), ListAccessError> { self.required()?.clear(); Ok(()) }
    pub(crate) fn remove(&self, index: usize) -> Result<(), ListAccessError> { self.required()?.remove("_organisms", index)?; Ok(()) }
    pub(crate) fn contains(&self, actor: &OrganismRef) -> bool {
        self.shared_ref().is_some_and(|list| list.slots().iter().any(|slot| matches!(slot, SharedSlot::Actor(value) if Rc::ptr_eq(value, actor))))
    }
    /// Reachability inspects targets without applying the membership role cast
    pub(crate) fn actors(&self) -> Vec<OrganismRef> {
        self.shared_ref().map_or_else(Vec::new, |list| list.slots().iter().filter_map(|slot| {
            if let SharedSlot::Actor(actor) = slot { Some(Rc::clone(actor)) } else { None }
        }).collect())
    }

    #[cfg(test)]
    pub(crate) fn borrow(&self) -> ActorView<'_> { ActorView(self.required().unwrap().slots()) }
    #[cfg(test)]
    pub(crate) fn borrow_mut(&self) -> ActorEdit<'_> { ActorEdit(self.required().unwrap().slots_mut()) }
}

// Existing actor-only fixture helpers borrow the actual slots; production reads use reached role checks
#[cfg(test)]
pub(crate) struct ActorView<'a>(std::cell::Ref<'a, Vec<SharedSlot>>);
#[cfg(test)]
fn actor(slot: &SharedSlot) -> &OrganismRef { let SharedSlot::Actor(actor) = slot else { panic!("Actor-only test fixture"); }; actor }
#[cfg(test)]
impl ActorView<'_> {
    pub(crate) fn len(&self) -> usize { self.0.len() }
    pub(crate) fn is_empty(&self) -> bool { self.0.is_empty() }
    pub(crate) fn capacity(&self) -> usize { self.0.capacity() }
    pub(crate) fn iter(&self) -> impl DoubleEndedIterator<Item = &OrganismRef> + ExactSizeIterator { self.0.iter().map(actor) }
    pub(crate) fn last(&self) -> Option<&OrganismRef> { self.0.last().map(actor) }
    pub(crate) fn first(&self) -> Option<&OrganismRef> { self.0.first().map(actor) }
    pub(crate) fn clone(&self) -> Vec<OrganismRef> { self.iter().cloned().collect() }
}
#[cfg(test)]
impl std::ops::Index<usize> for ActorView<'_> { type Output = OrganismRef; fn index(&self, index: usize) -> &Self::Output { actor(&self.0[index]) } }
#[cfg(test)]
pub(crate) struct ActorEdit<'a>(std::cell::RefMut<'a, Vec<SharedSlot>>);
#[cfg(test)]
impl ActorEdit<'_> {
    pub(crate) fn push(&mut self, value: OrganismRef) { self.0.push(SharedSlot::Actor(value)); }
    pub(crate) fn clear(&mut self) { self.0.clear(); }
    pub(crate) fn remove(&mut self, index: usize) -> OrganismRef { let SharedSlot::Actor(value) = self.0.remove(index) else { panic!("Actor-only test fixture"); }; value }
    pub(crate) fn pop(&mut self) -> Option<OrganismRef> { self.0.pop().map(|slot| { let SharedSlot::Actor(value) = slot else { panic!("Actor-only test fixture"); }; value }) }
    pub(crate) fn extend(&mut self, values: impl IntoIterator<Item = OrganismRef>) { self.0.extend(values.into_iter().map(SharedSlot::Actor)); }
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&OrganismRef) -> bool) { self.0.retain(|slot| keep(actor(slot))); }
}
