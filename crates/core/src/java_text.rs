// SPDX-License-Identifier: GPL-2.0-or-later

use std::{
    string::FromUtf16Error,
    sync::{Arc, OnceLock},
};

/// Immutable Java code units with allocation identity independent of value equality
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaText(Arc<[u16]>);

impl From<&str> for JavaText {
    fn from(value: &str) -> Self {
        Self::from_utf8(value)
    }
}

impl From<String> for JavaText {
    fn from(value: String) -> Self {
        Self::from_utf8(&value)
    }
}

impl JavaText {
    pub fn from_units(units: Vec<u16>) -> Self {
        Self(units.into())
    }

    pub fn from_utf8(value: &str) -> Self {
        Self::from_units(value.encode_utf16().collect())
    }

    pub fn units(&self) -> &[u16] {
        &self.0
    }

    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as *const u16 as usize
    }

    pub fn to_utf8(&self) -> Result<String, FromUtf16Error> {
        String::from_utf16(self.units())
    }

    /// Project for display without replacing or normalizing the stored units
    pub fn display_utf8_lossy(&self) -> String {
        String::from_utf16_lossy(self.units())
    }

    pub(crate) fn with_suffix(&self, suffix: &str) -> Self {
        let mut units = self.units().to_vec();
        units.extend(suffix.encode_utf16());
        Self::from_units(units)
    }
}

pub(crate) fn initial_positions() -> JavaText {
    static POSITIONS: OnceLock<JavaText> = OnceLock::new();
    POSITIONS.get_or_init(|| JavaText::from_utf8("01234567")).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projections_preserve_every_original_unit_and_identity() {
        for (units, valid) in [(vec![], true), (vec![0, 65], true), (vec![0xd83d, 0xde00], true), (vec![0xd800], false), (vec![0xdc00], false), (vec![0xd800, 43, 0, 0xdfff], false)] {
            let text = JavaText::from_units(units.clone());
            let alias = text.clone();
            assert_eq!(text.to_utf8().is_ok(), valid);
            let _display = text.display_utf8_lossy();
            assert_eq!(text.units(), units.as_slice());
            assert!(text.same_instance(&alias));
        }
        assert_eq!(JavaText::from_units(vec![0xd800]).display_utf8_lossy(), "\u{fffd}");
        let absent: Option<JavaText> = None;
        assert_ne!(absent, Some(JavaText::from_units(vec![])));
    }

    #[test]
    fn equal_allocations_stay_distinct_including_empty_strings() {
        for units in [vec![], vec![65, 0xd800]] {
            let first = JavaText::from_units(units.clone());
            let second = JavaText::from_units(units);
            assert_eq!(first, second);
            assert!(!first.same_instance(&second));
        }
    }

    #[test]
    fn source_literal_shares_one_owner_without_interning_equal_inputs() {
        let literal = initial_positions();
        assert!(literal.same_instance(&initial_positions()));
        let equal_input = JavaText::from_utf8("01234567");
        assert_eq!(literal, equal_input);
        assert!(!literal.same_instance(&equal_input));
    }
}
