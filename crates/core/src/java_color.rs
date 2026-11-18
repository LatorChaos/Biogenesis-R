// SPDX-License-Identifier: GPL-2.0-or-later

//! Immutable Color values and the shared references retained by the Java model

use std::sync::{Arc, OnceLock};

/// Raw float-array bits with reference identity independent of equal contents
#[derive(Clone, Debug)]
pub struct JavaFloatArray(Arc<Vec<u32>>);

impl JavaFloatArray {
    pub fn from_bits(bits: Vec<u32>) -> Self {
        Self(Arc::new(bits))
    }

    pub fn bits(&self) -> &[u32] {
        &self.0
    }

    pub(crate) fn storage_capacity(&self) -> usize { self.0.capacity() }

    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

/// The named profiles resolved by the installed JDK's ICC_Profile.readResolve
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum NamedIccProfile {
    Srgb,
    LinearRgb,
    Ciexyz,
    Pycc,
    Gray,
}

impl NamedIccProfile {
    pub(crate) const ALL: [Self; 5] = [
        Self::Srgb,
        Self::LinearRgb,
        Self::Ciexyz,
        Self::Pycc,
        Self::Gray,
    ];

    pub fn serialized_name(self) -> &'static str {
        match self {
            Self::Srgb => "CS_sRGB",
            Self::LinearRgb => "CS_LINEAR_RGB",
            Self::Ciexyz => "CS_CIEXYZ",
            Self::Pycc => "CS_PYCC",
            Self::Gray => "CS_GRAY",
        }
    }

    pub(crate) fn class_name(self) -> &'static str {
        match self {
            Self::Srgb | Self::LinearRgb => "java.awt.color.ICC_ProfileRGB",
            Self::Gray => "java.awt.color.ICC_ProfileGray",
            Self::Ciexyz | Self::Pycc => "java.awt.color.ICC_Profile",
        }
    }
}

/// Actual fixed profile identity, rather than interning arbitrary profile contents
#[derive(Clone, Debug)]
pub struct JavaIccProfile(Arc<NamedIccProfile>);

impl JavaIccProfile {
    pub fn builtin(kind: NamedIccProfile) -> Self {
        static PROFILES: OnceLock<[JavaIccProfile; 5]> = OnceLock::new();
        PROFILES
            .get_or_init(|| NamedIccProfile::ALL.map(|kind| Self(Arc::new(kind))))[kind as usize]
            .clone()
    }

    pub fn kind(&self) -> NamedIccProfile {
        *self.0
    }

    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

#[derive(Clone, Debug)]
pub(crate) struct IccSpaceData {
    pub num_components: i32,
    pub space_type: i32,
    pub need_scale_init: bool,
    pub diff_min_max: Option<JavaFloatArray>,
    pub inv_diff_min_max: Option<JavaFloatArray>,
    pub max_val: Option<JavaFloatArray>,
    pub min_val: Option<JavaFloatArray>,
    pub profile: Option<JavaIccProfile>,
}

/// A concrete ICC_ColorSpace snapshot without constructor-derived normalization
#[derive(Clone, Debug)]
pub struct JavaIccColorSpace(Arc<IccSpaceData>);

impl JavaIccColorSpace {
    pub(crate) fn from_parts(parts: IccSpaceData) -> Self {
        Self(Arc::new(parts))
    }

    pub(crate) fn parts(&self) -> &IccSpaceData {
        &self.0
    }

    pub fn num_components(&self) -> i32 {
        self.0.num_components
    }

    pub fn space_type(&self) -> i32 {
        self.0.space_type
    }

    pub fn need_scale_init(&self) -> bool {
        self.0.need_scale_init
    }

    pub fn diff_min_max(&self) -> Option<&JavaFloatArray> {
        self.0.diff_min_max.as_ref()
    }

    pub fn inv_diff_min_max(&self) -> Option<&JavaFloatArray> {
        self.0.inv_diff_min_max.as_ref()
    }

    pub fn max_val(&self) -> Option<&JavaFloatArray> {
        self.0.max_val.as_ref()
    }

    pub fn min_val(&self) -> Option<&JavaFloatArray> {
        self.0.min_val.as_ref()
    }

    pub fn profile(&self) -> Option<&JavaIccProfile> {
        self.0.profile.as_ref()
    }

    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ColorData {
    pub argb: u32,
    pub alpha_bits: u32,
    pub space: Option<JavaIccColorSpace>,
    pub rgb: Option<JavaFloatArray>,
    pub components: Option<JavaFloatArray>,
}

/// A Color owner preserves metadata while Java value equality compares full ARGB
#[derive(Clone, Debug)]
pub struct JavaColor(Arc<ColorData>);

impl JavaColor {
    /// Create a distinct integer-constructor Color, including its actual empty caches
    pub fn from_argb(argb: u32) -> Self {
        Self::from_parts(ColorData {
            argb,
            alpha_bits: 0,
            space: None,
            rgb: None,
            components: None,
        })
    }

    pub(crate) fn from_parts(parts: ColorData) -> Self {
        Self(Arc::new(parts))
    }

    pub(crate) fn parts(&self) -> &ColorData {
        &self.0
    }

    pub fn argb(&self) -> u32 {
        self.0.argb
    }

    pub fn alpha_bits(&self) -> u32 {
        self.0.alpha_bits
    }

    pub fn rgb_components(&self) -> Option<&JavaFloatArray> {
        self.0.rgb.as_ref()
    }

    pub fn components(&self) -> Option<&JavaFloatArray> {
        self.0.components.as_ref()
    }

    pub fn color_space(&self) -> Option<&JavaIccColorSpace> {
        self.0.space.as_ref()
    }

    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

impl PartialEq for JavaColor {
    fn eq(&self, other: &Self) -> bool {
        self.argb() == other.argb()
    }
}

impl Eq for JavaColor {}


/// One mutable Java Color[] allocation, independent of its holders and geometric segment count
#[derive(Clone, Debug)]
pub struct ColorArrayRef(std::rc::Rc<std::cell::RefCell<Vec<Option<JavaColor>>>>);

impl ColorArrayRef {
    pub fn from_slots(slots: Vec<Option<JavaColor>>) -> Self {
        Self(std::rc::Rc::new(std::cell::RefCell::new(slots)))
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    /// Clone only the immutable reference, then release the array borrow
    pub fn get(&self, index: usize) -> Option<Option<JavaColor>> {
        self.0.borrow().get(index).cloned()
    }

    pub fn replace(&self, index: usize, value: Option<JavaColor>) -> Result<(), usize> {
        let mut slots = self.0.borrow_mut();
        let length = slots.len();
        let slot = slots.get_mut(index).ok_or(length)?;
        *slot = value;
        Ok(())
    }

    pub fn same_instance(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn identity(&self) -> usize {
        std::rc::Rc::as_ptr(&self.0) as usize
    }

    /// A detached body value has a fresh mutable array and shared immutable Colors
    pub(crate) fn detached_copy(&self) -> Self {
        Self::from_slots(self.0.borrow().clone())
    }

    pub(crate) fn slots(&self) -> std::cell::Ref<'_, Vec<Option<JavaColor>>> {
        self.0.borrow()
    }
}

impl PartialEq for ColorArrayRef {
    fn eq(&self, other: &Self) -> bool {
        self.same_instance(other) || *self.0.borrow() == *other.0.borrow()
    }
}

impl Eq for ColorArrayRef {}

#[cfg(test)]
#[path = "java_color_tests.rs"]
mod tests;
