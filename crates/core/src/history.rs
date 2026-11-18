// SPDX-License-Identifier: GPL-2.0-or-later

//! Numeric history role over shared nullable list slots

use std::{fmt, rc::Rc, collections::HashMap};
use crate::shared_world_owners::{SharedListRef, SharedSlot, ListAccessError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryError {
    MissingList { field: &'static str },
    MissingElement { field: &'static str, index: usize },
    Capacity { field: &'static str },
    WrongClass { field: &'static str, index: usize, expected: &'static str, actual: &'static str },
    Index { field: &'static str, index: usize, length: usize },
}
impl From<ListAccessError> for HistoryError {
    fn from(error: ListAccessError) -> Self {
        match error {
            ListAccessError::MissingList { field } => Self::MissingList { field },
            ListAccessError::MissingElement { field, index } => Self::MissingElement { field, index },
            ListAccessError::Capacity { field } => Self::Capacity { field },
            ListAccessError::WrongClass { field, index, expected, actual } => Self::WrongClass { field, index, expected, actual },
            ListAccessError::Index { field, index, length } => Self::Index { field, index, length },
        }
    }
}
impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::MissingList { field } => write!(f, "Statistics history {field} is null"),
            Self::MissingElement { field, index } => write!(f, "Statistics history {field} element {index} is null"),
            Self::Capacity { field } => write!(f, "Statistics history {field} cannot grow"),
            Self::WrongClass { field, index, expected, actual } => ListAccessError::WrongClass { field, index, expected, actual }.fmt(f),
            Self::Index { field, index, length } => ListAccessError::Index { field, index, length }.fmt(f),
        }
    }
}
impl std::error::Error for HistoryError {}

/// Identity is separate from raw IEEE payload equality
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JavaDoubleRef(Rc<u64>);

impl JavaDoubleRef {
    pub(crate) fn from_bits(bits: u64) -> Self {
        Self(Rc::new(bits))
    }
    pub(crate) fn from_value(value: f64) -> Self {
        Self::from_bits(value.to_bits())
    }
    pub(crate) fn bits(&self) -> u64 {
        *self.0
    }
    pub(crate) fn value(&self) -> f64 {
        f64::from_bits(self.bits())
    }
    pub(crate) fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
    pub(crate) fn same_instance(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HistoryListRef(SharedListRef);
impl HistoryListRef {
    pub(crate) fn from_shared(value: SharedListRef) -> Self { Self(value) }
    pub(crate) fn shared_ref(&self) -> &SharedListRef { &self.0 }
    pub(crate) fn empty() -> Self { Self(SharedListRef::from_slots(Vec::new())) }
    pub(crate) fn from_elements(values: Vec<Option<JavaDoubleRef>>) -> Self {
        let mut slots = Vec::with_capacity(values.capacity());
        slots.extend(values.into_iter().map(|value| value.map_or(SharedSlot::Null, SharedSlot::Double)));
        Self(SharedListRef::from_slots(slots))
    }
    pub(crate) fn from_values(values: Vec<f64>) -> Self { Self::from_elements(values.into_iter().map(|v| Some(JavaDoubleRef::from_value(v))).collect()) }
    #[cfg(test)]
    pub(crate) fn elements(&self) -> Vec<Option<JavaDoubleRef>> {
        self.0.slots().iter().map(|value| match value { SharedSlot::Null => None, SharedSlot::Double(value) => Some(value.clone()), _ => panic!("Numeric-only history helper") }).collect()
    }
    pub(crate) fn len(&self) -> usize { self.0.len() }
    pub(crate) fn is_empty(&self) -> bool { self.len() == 0 }
    pub(crate) fn identity(&self) -> usize { self.0.identity() }
    pub(crate) fn same_instance(&self, other: &Self) -> bool { self.0.same_instance(&other.0) }
    #[cfg(test)]
    pub(crate) fn numeric_slots(&self) -> Vec<Option<f64>> { self.elements().iter().map(|value| value.as_ref().map(JavaDoubleRef::value)).collect() }
    pub(crate) fn numeric_prefix(&self, field: &'static str, count: usize) -> Result<Vec<f64>, HistoryError> { self.0.numeric_prefix(field, count).map_err(HistoryError::from) }
    pub(crate) fn append(&self, field: &'static str, value: f64) -> Result<(), HistoryError> { self.0.append_runtime_number(field, value).map_err(HistoryError::from) }
    pub(crate) fn copied(&self, owners: &mut HashMap<usize, Self>) -> Self {
        owners.entry(self.identity()).or_insert_with(|| Self(SharedListRef::from_slots(self.0.slots().clone()))).clone()
    }
}
impl PartialEq for HistoryListRef {
    fn eq(&self, other: &Self) -> bool {
        let a = self.0.slots(); let b = other.0.slots();
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(a,b)| match (a,b) {
            (SharedSlot::Null, SharedSlot::Null) => true,
            (SharedSlot::Double(a), SharedSlot::Double(b)) => a.bits() == b.bits(),
            (SharedSlot::Actor(a), SharedSlot::Actor(b)) => Rc::ptr_eq(a,b),
            (SharedSlot::Boundable(a), SharedSlot::Boundable(b)) => a.identity() == b.identity(),
            (SharedSlot::Envelope(a), SharedSlot::Envelope(b)) => a.identity() == b.identity(),
            _ => false,
        })
    }
}
