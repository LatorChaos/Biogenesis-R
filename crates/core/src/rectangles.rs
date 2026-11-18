// SPDX-License-Identifier: GPL-2.0-or-later

//! Mutable Rectangle values and nonowning Organism subtype associations

use crate::organism::{Organism, OrganismRef};
use crate::{GeometryError, IntRect};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::{Rc, Weak},
};

struct RectangleCell {
    value: Cell<IntRect>,
    actor_class: bool,
    actor: RefCell<Weak<RefCell<Organism>>>,
}

/// One authoritative Rectangle value; cloned handles retain object identity
#[derive(Clone)]
pub struct RectangleRef(Rc<RectangleCell>);

impl RectangleRef {
    pub fn new(value: IntRect) -> Self {
        Self::with_class(value, false)
    }
    pub(crate) fn actor(value: IntRect) -> Self {
        Self::with_class(value, true)
    }
    fn with_class(value: IntRect, actor_class: bool) -> Self {
        Self(Rc::new(RectangleCell {
            value: Cell::new(value),
            actor_class,
            actor: RefCell::new(Weak::new()),
        }))
    }
    pub fn get(&self) -> IntRect {
        self.0.value.get()
    }
    pub fn set(&self, value: IntRect) {
        self.0.value.set(value);
    }
    pub fn update(&self, update: impl FnOnce(&mut IntRect)) {
        let mut value = self.get();
        update(&mut value);
        self.set(value);
    }
    pub fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
    pub fn same_instance(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub fn class_name(&self) -> &'static str {
        if self.0.actor_class {
            "biogenesis.Organism"
        } else {
            "java.awt.Rectangle"
        }
    }
    pub(crate) fn is_actor(&self) -> bool {
        self.0.actor_class
    }
    pub(crate) fn allocation_bytes() -> usize {
        std::mem::size_of::<RectangleCell>() + 2 * std::mem::size_of::<usize>()
    }
    pub(crate) fn actor_target(&self) -> Option<OrganismRef> {
        self.0.actor.borrow().upgrade()
    }
    pub(crate) fn bind_actor(&self, actor: &OrganismRef) -> Result<(), GeometryError> {
        if !self.is_actor() {
            return Err(GeometryError::RectangleAssociation);
        }
        let mut association = self.0.actor.borrow_mut();
        if association
            .upgrade()
            .is_some_and(|previous| !Rc::ptr_eq(&previous, actor))
        {
            return Err(GeometryError::RectangleAssociation);
        }
        *association = Rc::downgrade(actor);
        Ok(())
    }
    pub(crate) fn copied(&self, owners: &mut HashMap<usize, Self>) -> Self {
        owners
            .entry(self.identity())
            .or_insert_with(|| Self::with_class(self.get(), self.is_actor()))
            .clone()
    }
    pub fn detached_copy(&self) -> Self {
        Self::with_class(self.get(), self.is_actor())
    }
}
impl Default for RectangleRef {
    fn default() -> Self {
        Self::new(IntRect::default())
    }
}
impl PartialEq for RectangleRef {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get() && self.is_actor() == other.is_actor()
    }
}
impl Eq for RectangleRef {}
impl fmt::Debug for RectangleRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RectangleRef")
            .field("class", &self.class_name())
            .field("value", &self.get())
            .finish()
    }
}

pub(crate) fn required_rectangle<'a>(
    value: Option<&'a RectangleRef>,
    field: &'static str,
) -> Result<&'a RectangleRef, GeometryError> {
    value.ok_or(GeometryError::MissingRectangle { field })
}

/// Complete imported actors remain World-owned while all Rectangle back-links stay weak
#[derive(Debug, PartialEq)]
pub(crate) struct RectangleActorContext {
    actors: Vec<OrganismRef>,
    retained_bytes: usize,
    maximum_bytes: usize,
}
impl RectangleActorContext {
    pub(crate) fn validated(
        actors: Vec<OrganismRef>,
        retained_bytes: usize,
        maximum_bytes: usize,
    ) -> Self {
        debug_assert!(retained_bytes <= maximum_bytes);
        Self {
            actors,
            retained_bytes,
            maximum_bytes,
        }
    }
    pub(crate) fn actors(&self) -> &[OrganismRef] {
        &self.actors
    }
    pub(crate) fn storage_capacity(&self) -> usize { self.actors.capacity() }
    pub(crate) fn maximum_bytes(&self) -> usize { self.maximum_bytes }
    pub(crate) fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
    pub(crate) fn include_staging_bytes(&mut self, bytes: usize, maximum: usize) {
        debug_assert!(bytes <= maximum);
        self.retained_bytes = bytes;
        self.maximum_bytes = maximum;
    }
    pub(crate) fn copied(&self, cloner: &mut crate::organism::OrganismCloner) -> Self {
        Self {
            actors: self.actors.iter().map(|actor| cloner.copy(actor)).collect(),
            retained_bytes: self.retained_bytes,
            maximum_bytes: self.maximum_bytes,
        }
    }
}
