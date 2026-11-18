// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::{GeneRef, GeneticsError};
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

/// Fixed-length mutable Java Gene[] owner; cloning retains its actual identity
#[derive(Clone, Debug, PartialEq)]
pub struct GeneArrayRef(Rc<RefCell<Vec<Option<GeneRef>>>>);

impl GeneArrayRef {
    /// Retain supplied capacity and nullable slots without validating Gene values
    pub fn from_slots(slots: Vec<Option<GeneRef>>) -> Self {
        Self(Rc::new(RefCell::new(slots)))
    }
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn same_instance(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
    pub fn slots(&self) -> Ref<'_, [Option<GeneRef>]> {
        Ref::map(self.0.borrow(), |v| v.as_slice())
    }
    pub(crate) fn capacity(&self) -> usize {
        self.0.borrow().capacity()
    }
    pub fn get(&self, index: i32) -> Result<Option<GeneRef>, GeneticsError> {
        let slots = self.0.borrow();
        slots
            .get(index as usize)
            .cloned()
            .ok_or(GeneticsError::GeneIndex {
                index: i64::from(index),
                length: slots.len(),
            })
    }
    pub fn set(&self, index: i32, value: Option<GeneRef>) -> Result<(), GeneticsError> {
        let mut slots = self.0.borrow_mut();
        let length = slots.len();
        *slots
            .get_mut(index as usize)
            .ok_or(GeneticsError::GeneIndex {
                index: i64::from(index),
                length,
            })? = value;
        Ok(())
    }
    pub(crate) fn allocation_bytes(&self) -> Option<usize> {
        std::mem::size_of::<usize>()
            .checked_mul(2)?
            .checked_add(std::mem::size_of::<RefCell<Vec<Option<GeneRef>>>>())?
            .checked_add(
                self.capacity()
                    .checked_mul(std::mem::size_of::<Option<GeneRef>>())?,
            )
    }
}
