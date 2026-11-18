// SPDX-License-Identifier: GPL-2.0-or-later

//! Detached, bounded conversion for the captured Color closure

use crate::java_color::{ColorData, IccSpaceData};
use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::object_stream::{
    Arena, ArrayData, ClassData, ClassDescriptor, Document, FieldValue, JavaArray, JavaObject,
    JavaString, Limits, Node, NodeId, Record, Value, class_by_name, class_schema,
};
use crate::{ColorArrayRef, JavaColor, JavaFloatArray, JavaIccColorSpace, JavaIccProfile, NamedIccProfile};
use std::mem::size_of;

const COLOR: &str = "java.awt.Color";
const SPACE: &str = "java.awt.color.ICC_ColorSpace";
const SPACE_BASE: &str = "java.awt.color.ColorSpace";
const PROFILE: &str = "java.awt.color.ICC_Profile";
const FLOATS: &str = "[F";
const COLORS: &str = "[Ljava.awt.Color;";
const ARC_HEADER: usize = 2 * size_of::<usize>();

fn failure(node: Option<NodeId>, field: Option<&'static str>, kind: GraphErrorKind) -> GraphError {
    GraphError { node, field, kind }
}

fn field_error(mut error: GraphError, owner: NodeId, field: &'static str) -> GraphError {
    if error.node.is_none() { error.node = Some(owner); }
    if error.field.is_none() { error.field = Some(field); }
    error
}

fn product(count: usize, bytes: usize) -> Result<usize, GraphError> {
    count
        .checked_mul(bytes)
        .ok_or(failure(None, None, GraphErrorKind::Budget))
}

/// Additional requested allocation bytes, separate from the parsed arena's budget
struct ColorBudget {
    maximum: usize,
    used: usize,
}

impl ColorBudget {
    fn new(maximum: usize) -> Self {
        Self { maximum, used: 0 }
    }

    fn charge(&mut self, bytes: usize) -> Result<(), GraphError> {
        let next = self
            .used
            .checked_add(bytes)
            .ok_or(failure(None, None, GraphErrorKind::Budget))?;
        if next > self.maximum {
            return Err(failure(None, None, GraphErrorKind::Budget));
        }
        self.used = next;
        Ok(())
    }

    fn reserve<T>(&mut self, values: &mut Vec<T>, count: usize) -> Result<(), GraphError> {
        self.charge(product(count, size_of::<T>())?)?;
        values
            .try_reserve_exact(count)
            .map_err(|_| failure(None, None, GraphErrorKind::Allocation))
    }
}

#[derive(Clone)]
enum Staged {
    Color(JavaColor),
    Colors(ColorArrayRef),
    Floats(JavaFloatArray),
    Space(JavaIccColorSpace),
    Profile(JavaIccProfile),
}

/// This temporary table is dropped after typed holders are published
pub(crate) struct ColorStager<'graph, 'document> {
    graph: &'graph LegacyGraph<'document>,
    owners: Vec<Option<Staged>>,
    budget: ColorBudget,
    array_elements: usize,
    other_array_elements: usize,
    profile_catalog_charged: bool,
}

impl<'graph, 'document> ColorStager<'graph, 'document> {
    pub(crate) fn new(
        graph: &'graph LegacyGraph<'document>,
        additional_budget_bytes: usize,
    ) -> Result<Self, GraphError> {
        let mut budget = ColorBudget::new(additional_budget_bytes);
        let count = graph.arena().nodes.len();
        graph.charge(count)?;
        let mut owners = Vec::new();
        graph.allocate(product(count, size_of::<Option<Staged>>())?)?;
        budget.reserve(&mut owners, count)?;
        owners.resize_with(count, || None);
        Ok(Self {
            graph,
            owners,
            budget,
            array_elements: 0,
            other_array_elements: 0,
            profile_catalog_charged: false,
        })
    }

    fn charge(&mut self, bytes: usize) -> Result<(), GraphError> {
        self.budget.charge(bytes)?;
        self.graph.allocate(bytes)
    }
    fn reserve<T>(&mut self, values: &mut Vec<T>, count: usize) -> Result<(), GraphError> {
        self.charge(product(count, size_of::<T>())?)?;
        values.try_reserve_exact(count).map_err(|_| failure(None, None, GraphErrorKind::Allocation))?;
        self.charge(product(values.capacity() - count, size_of::<T>())?)
    }
    pub(crate) fn allocated_bytes(&self) -> usize {
        self.budget.used
    }
    pub(crate) fn restrict_budget(&mut self, maximum: usize) -> Result<(), GraphError> {
        if self.budget.used > maximum { return Err(failure(None, None, GraphErrorKind::Budget)); }
        self.budget.maximum = maximum;
        Ok(())
    }
    pub(crate) fn set_other_array_elements(&mut self, count: usize) -> Result<(), GraphError> {
        if self.array_elements.checked_add(count).is_none_or(|total| total > self.graph.limits().total_array_elements) {
            return Err(failure(None, None, GraphErrorKind::Budget));
        }
        self.other_array_elements = count;
        Ok(())
    }
    pub(crate) fn validate_descriptor(&self, id: NodeId) -> Result<(), GraphError> { self.descriptor(id) }


    fn descriptor(&self, id: NodeId) -> Result<(), GraphError> {
        self.graph.validate_descriptor(id)
    }

    fn object(&self, id: NodeId, name: &'static str) -> Result<(), GraphError> {
        self.graph.expect_class(id, name)?;
        for slot in self.graph.slots(id)? {
            self.descriptor(slot.descriptor)?;
            let Node::Descriptor(descriptor) = self.graph.node(slot.descriptor)? else {
                unreachable!()
            };
            if class_schema(descriptor.class).flags & 1 == 0 && !slot.custom.is_empty() {
                return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
            }
        }
        Ok(())
    }

    fn total_array(&mut self, count: usize, id: NodeId) -> Result<(), GraphError> {
        let total = self.array_elements.checked_add(count)
            .ok_or(GraphError::at(id, None, GraphErrorKind::Budget))?;
        if count > self.graph.limits().array_elements
            || total.checked_add(self.other_array_elements).is_none_or(|value| value > self.graph.limits().total_array_elements)
        {
            return Err(GraphError::at(id, None, GraphErrorKind::Budget));
        }
        self.graph.count_array(count)?;
        self.array_elements = total;
        Ok(())
    }

    pub(crate) fn color(&mut self, value: Value) -> Result<Option<JavaColor>, GraphError> {
        let Value::Handle(id) = value else { return Ok(None) };
        self.graph.node(id)?;
        if let Some(saved) = &self.owners[id.0 as usize] {
            return match saved {
                Staged::Color(value) => Ok(Some(value.clone())),
                _ => Err(GraphError::at(id, None, GraphErrorKind::Type)),
            };
        }
        self.object(id, COLOR)?;
        let argb = self.graph.int(id, COLOR, "value")? as u32;
        let alpha_bits = match self.graph.field(id, COLOR, "falpha")? {
            FieldValue::FloatBits(value) => value,
            _ => return Err(GraphError::at(id, Some("falpha"), GraphErrorKind::Type)),
        };
        let space = self.space(self.graph.reference(id, COLOR, "cs")?)
            .map_err(|error| field_error(error, id, "cs"))?;
        let rgb = self.floats(self.graph.reference(id, COLOR, "frgbvalue")?)
            .map_err(|error| field_error(error, id, "frgbvalue"))?;
        let components = self.floats(self.graph.reference(id, COLOR, "fvalue")?)
            .map_err(|error| field_error(error, id, "fvalue"))?;
        self.charge(size_of::<ColorData>() + ARC_HEADER)?;
        let color = JavaColor::from_parts(ColorData { argb, alpha_bits, space, rgb, components });
        self.owners[id.0 as usize] = Some(Staged::Color(color.clone()));
        Ok(Some(color))
    }

    pub(crate) fn colors(&mut self, value: Value) -> Result<Option<ColorArrayRef>, GraphError> {
        let Value::Handle(id) = value else { return Ok(None) };
        self.graph.node(id)?;
        if let Some(saved) = &self.owners[id.0 as usize] {
            return match saved {
                Staged::Colors(value) => Ok(Some(value.clone())),
                _ => Err(GraphError::at(id, None, GraphErrorKind::Type)),
            };
        }
        self.graph.expect_class(id, COLORS)?;
        let Node::Array(array) = self.graph.node(id)? else {
            return Err(GraphError::at(id, None, GraphErrorKind::Type));
        };
        self.descriptor(array.descriptor)?;
        let ArrayData::References(values) = &array.data else {
            return Err(GraphError::at(id, None, GraphErrorKind::Type));
        };
        self.total_array(values.len(), id)?;
        self.charge(size_of::<std::cell::RefCell<Vec<Option<JavaColor>>>>() + ARC_HEADER)?;
        let mut result = Vec::new();
        self.reserve(&mut result, values.len())?;
        for value in values {
            result.push(self.color(*value)?);
        }
        let array = ColorArrayRef::from_slots(result);
        self.owners[id.0 as usize] = Some(Staged::Colors(array.clone()));
        Ok(Some(array))
    }

    pub(crate) fn floats(&mut self, value: Value) -> Result<Option<JavaFloatArray>, GraphError> {
        let Value::Handle(id) = value else { return Ok(None) };
        self.graph.node(id)?;
        if let Some(saved) = &self.owners[id.0 as usize] {
            return match saved {
                Staged::Floats(value) => Ok(Some(value.clone())),
                _ => Err(GraphError::at(id, None, GraphErrorKind::Type)),
            };
        }
        self.graph.expect_class(id, FLOATS)?;
        let Node::Array(array) = self.graph.node(id)? else {
            return Err(GraphError::at(id, None, GraphErrorKind::Type));
        };
        self.descriptor(array.descriptor)?;
        let ArrayData::Floats(bits) = &array.data else {
            return Err(GraphError::at(id, None, GraphErrorKind::Type));
        };
        self.total_array(bits.len(), id)?;
        self.charge(size_of::<Vec<u32>>() + ARC_HEADER)?;
        let mut copied = Vec::new();
        self.reserve(&mut copied, bits.len())?;
        copied.extend_from_slice(bits);
        let result = JavaFloatArray::from_bits(copied);
        self.owners[id.0 as usize] = Some(Staged::Floats(result.clone()));
        Ok(Some(result))
    }

    fn space(&mut self, value: Value) -> Result<Option<JavaIccColorSpace>, GraphError> {
        let Value::Handle(id) = value else { return Ok(None) };
        self.graph.node(id)?;
        if let Some(saved) = &self.owners[id.0 as usize] {
            return match saved {
                Staged::Space(value) => Ok(Some(value.clone())),
                _ => Err(GraphError::at(id, None, GraphErrorKind::Type)),
            };
        }
        self.object(id, SPACE)?;
        let num_components = self.graph.int(id, SPACE_BASE, "numComponents")?;
        let space_type = self.graph.int(id, SPACE_BASE, "type")?;
        let need_scale_init = self.graph.boolean(id, SPACE, "needScaleInit")?;
        let diff_min_max = self.floats(self.graph.reference(id, SPACE, "diffMinMax")?)
            .map_err(|error| field_error(error, id, "diffMinMax"))?;
        let inv_diff_min_max = self.floats(self.graph.reference(id, SPACE, "invDiffMinMax")?)
            .map_err(|error| field_error(error, id, "invDiffMinMax"))?;
        let max_val = self.floats(self.graph.reference(id, SPACE, "maxVal")?)
            .map_err(|error| field_error(error, id, "maxVal"))?;
        let min_val = self.floats(self.graph.reference(id, SPACE, "minVal")?)
            .map_err(|error| field_error(error, id, "minVal"))?;
        let profile = self.profile(self.graph.reference(id, SPACE, "thisProfile")?)
            .map_err(|error| field_error(error, id, "thisProfile"))?;
        // ICC_ColorSpace.readObject replaces only a null profile with fixed sRGB
        let profile = Some(match profile {
            Some(profile) => profile,
            None => self.builtin_profile(NamedIccProfile::Srgb)?,
        });
        self.charge(size_of::<IccSpaceData>() + ARC_HEADER)?;
        let space = JavaIccColorSpace::from_parts(IccSpaceData {
            num_components, space_type, need_scale_init,
            diff_min_max, inv_diff_min_max, max_val, min_val, profile,
        });
        self.owners[id.0 as usize] = Some(Staged::Space(space.clone()));
        Ok(Some(space))
    }

    fn builtin_profile(&mut self, kind: NamedIccProfile) -> Result<JavaIccProfile, GraphError> {
        if !self.profile_catalog_charged {
            self.charge(product(5, size_of::<NamedIccProfile>() + ARC_HEADER)?)?;
            self.profile_catalog_charged = true;
        }
        Ok(JavaIccProfile::builtin(kind))
    }

    fn profile(&mut self, value: Value) -> Result<Option<JavaIccProfile>, GraphError> {
        let Value::Handle(id) = value else { return Ok(None) };
        self.graph.node(id)?;
        if let Some(saved) = &self.owners[id.0 as usize] {
            return match saved {
                Staged::Profile(value) => Ok(Some(value.clone())),
                _ => Err(GraphError::at(id, None, GraphErrorKind::Type)),
            };
        }
        let class = self.graph.class_name(id)?;
        if ![PROFILE, "java.awt.color.ICC_ProfileRGB", "java.awt.color.ICC_ProfileGray"].contains(&class) {
            return Err(GraphError::at(id, None, GraphErrorKind::Class));
        }
        self.object(id, class)?;
        // defaultReadObject reads this field before readResolve replaces the profile
        self.graph.int(id, PROFILE, "iccProfileSerializedDataVersion")?;
        let slot = self.graph.slots(id)?.iter().find(|slot| {
            matches!(self.graph.arena().nodes.get(slot.descriptor.0 as usize),
                Some(Node::Descriptor(descriptor)) if class_schema(descriptor.class).name == PROFILE)
        }).ok_or(GraphError::at(id, None, GraphErrorKind::Slots))?;
        let [Record::Value(name), Record::Value(data)] = slot.custom.as_slice() else {
            return Err(GraphError::at(id, Some("custom"), GraphErrorKind::ContainerData));
        };
        if let Value::Handle(data) = data {
            let kind = if self.graph.class_name(*data)? == "[B" {
                GraphErrorKind::UnsupportedColor
            } else {
                GraphErrorKind::Type
            };
            return Err(GraphError::at(id, Some("profileData"), kind));
        }
        let Value::Handle(name) = name else {
            return Err(GraphError::at(id, Some("profileName"), GraphErrorKind::ContainerData));
        };
        let units = self.graph.string_units(*name)?;
        let kind = NamedIccProfile::ALL.into_iter().find(|kind|
            units.iter().copied().eq(kind.serialized_name().encode_utf16())
        ).ok_or(GraphError::at(id, Some("profileName"), GraphErrorKind::ContainerData))?;
        let profile = self.builtin_profile(kind)?;
        self.owners[id.0 as usize] = Some(Staged::Profile(profile.clone()));
        Ok(Some(profile))
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ColorRoot<'a> {
    Color(Option<&'a JavaColor>),
    Colors(Option<&'a ColorArrayRef>),
    Floats(Option<&'a JavaFloatArray>),
}

/// Collect only typed handles, then sort identities before allocating copied payloads
struct ColorEmitter {
    arena: Arena,
    limits: Limits,
    budget: ColorBudget,
    arrays: Vec<ColorArrayRef>,
    array_ids: Vec<NodeId>,
    colors: Vec<JavaColor>,
    floats: Vec<JavaFloatArray>,
    spaces: Vec<JavaIccColorSpace>,
    profiles: Vec<JavaIccProfile>,
    color_ids: Vec<NodeId>,
    float_ids: Vec<NodeId>,
    space_ids: Vec<NodeId>,
    profile_ids: Vec<NodeId>,
    descriptors: Vec<(&'static str, NodeId)>,
    signatures: Vec<(&'static str, NodeId)>,
    elements: usize,
}

impl ColorEmitter {
    fn new(roots: &[ColorRoot<'_>], limits: Limits, maximum: usize) -> Result<Self, GraphError> {
        let mut counts = [0_usize; 5];
        fn count_color(color: &JavaColor, counts: &mut [usize; 5]) -> Result<(), GraphError> {
            counts[0] = counts[0].checked_add(1).ok_or(failure(None, None, GraphErrorKind::Budget))?;
            counts[1] = counts[1].checked_add(
                usize::from(color.rgb_components().is_some()) + usize::from(color.components().is_some()),
            ).ok_or(failure(None, None, GraphErrorKind::Budget))?;
            if let Some(space) = color.color_space() {
                counts[2] = counts[2].checked_add(1).ok_or(failure(None, None, GraphErrorKind::Budget))?;
                counts[3] = counts[3].checked_add(usize::from(space.profile().is_some()))
                    .ok_or(failure(None, None, GraphErrorKind::Budget))?;
                counts[1] = counts[1].checked_add(
                    [space.diff_min_max(), space.inv_diff_min_max(), space.max_val(), space.min_val()]
                        .into_iter().filter(Option::is_some).count(),
                ).ok_or(failure(None, None, GraphErrorKind::Budget))?;
            }
            Ok(())
        }
        for root in roots {
            match *root {
                ColorRoot::Color(value) => { if let Some(value) = value { count_color(value, &mut counts)?; } }
                ColorRoot::Colors(value) => {
                    if let Some(array) = value {
                        if array.len() > limits.array_elements {
                            return Err(failure(None, None, GraphErrorKind::Budget));
                        }
                        counts[4] = counts[4].checked_add(1).ok_or(failure(None, None, GraphErrorKind::Budget))?;
                        for value in array.slots().iter().flatten() { count_color(value, &mut counts)?; }
                    }
                }
                ColorRoot::Floats(value) => {
                    counts[1] = counts[1].checked_add(usize::from(value.is_some())).ok_or(failure(None, None, GraphErrorKind::Budget))?;
                }
            }
        }
        if roots.len() > limits.records || limits.depth == 0 {
            return Err(failure(None, None, GraphErrorKind::Budget));
        }
        let work = counts.iter().try_fold(1_usize, |sum, value| sum.checked_add(*value))
            .and_then(|sum| sum.checked_mul(usize::BITS as usize))
            .ok_or(failure(None, None, GraphErrorKind::Budget))?;
        if work > limits.work {
            return Err(failure(None, None, GraphErrorKind::Budget));
        }
        let mut result = Self {
            arena: Arena::default(), limits, budget: ColorBudget::new(maximum),
            arrays: Vec::new(), array_ids: Vec::new(),
            colors: Vec::new(), floats: Vec::new(), spaces: Vec::new(), profiles: Vec::new(),
            color_ids: Vec::new(), float_ids: Vec::new(), space_ids: Vec::new(), profile_ids: Vec::new(),
            descriptors: Vec::new(), signatures: Vec::new(), elements: 0,
        };
        result.budget.reserve(&mut result.arrays, counts[4])?;
        result.budget.reserve(&mut result.colors, counts[0])?;
        result.budget.reserve(&mut result.floats, counts[1])?;
        result.budget.reserve(&mut result.spaces, counts[2])?;
        result.budget.reserve(&mut result.profiles, counts[3])?;
        for root in roots {
            match *root {
                ColorRoot::Color(value) => { if let Some(value) = value { result.collect(value); } }
                ColorRoot::Colors(value) => {
                    if let Some(array) = value {
                        result.arrays.push(array.clone());
                        for value in array.slots().iter().flatten() { result.collect(value); }
                    }
                }
                ColorRoot::Floats(value) => { if let Some(value) = value { result.floats.push(value.clone()); } }
            }
        }
        result.arrays.sort_unstable_by_key(ColorArrayRef::identity);
        result.arrays.dedup_by_key(|value| value.identity());
        result.colors.sort_unstable_by_key(JavaColor::identity);
        result.colors.dedup_by_key(|value| value.identity());
        result.floats.sort_unstable_by_key(JavaFloatArray::identity);
        result.floats.dedup_by_key(|value| value.identity());
        result.spaces.sort_unstable_by_key(JavaIccColorSpace::identity);
        result.spaces.dedup_by_key(|value| value.identity());
        result.profiles.sort_unstable_by_key(JavaIccProfile::identity);
        result.profiles.dedup_by_key(|value| value.identity());
        result.budget.reserve(&mut result.array_ids, result.arrays.len())?;
        result.budget.reserve(&mut result.color_ids, result.colors.len())?;
        result.budget.reserve(&mut result.float_ids, result.floats.len())?;
        result.budget.reserve(&mut result.space_ids, result.spaces.len())?;
        result.budget.reserve(&mut result.profile_ids, result.profiles.len())?;
        result.budget.reserve(&mut result.descriptors, 8)?;
        result.budget.reserve(&mut result.signatures, 3)?;
        let owners = result.colors.len().checked_add(result.arrays.len()).and_then(|n| n.checked_add(result.floats.len()))
            .and_then(|n| n.checked_add(result.spaces.len()))
            .and_then(|n| n.checked_add(result.profiles.len()))
            .and_then(|n| n.checked_add(17))
            .ok_or(failure(None, None, GraphErrorKind::Budget))?;
        result.budget.reserve(&mut result.arena.nodes, owners)?;
        Ok(result)
    }

    fn collect(&mut self, color: &JavaColor) {
        self.colors.push(color.clone());
        if let Some(value) = color.rgb_components() { self.floats.push(value.clone()); }
        if let Some(value) = color.components() { self.floats.push(value.clone()); }
        if let Some(space) = color.color_space() {
            self.spaces.push(space.clone());
            for value in [space.diff_min_max(), space.inv_diff_min_max(), space.max_val(), space.min_val()].into_iter().flatten() {
                self.floats.push(value.clone());
            }
            if let Some(profile) = space.profile() { self.profiles.push(profile.clone()); }
        }
    }

    fn push(&mut self, node: Node) -> Result<NodeId, GraphError> {
        if self.arena.nodes.len() >= self.limits.handles || self.arena.nodes.len() >= self.arena.nodes.capacity() {
            return Err(failure(None, None, GraphErrorKind::Budget));
        }
        let id = u32::try_from(self.arena.nodes.len())
            .map_err(|_| failure(None, None, GraphErrorKind::Budget))?;
        self.arena.nodes.push(node);
        Ok(NodeId(id))
    }

    fn string(&mut self, text: &'static str) -> Result<NodeId, GraphError> {
        let count = text.encode_utf16().count();
        if count > self.limits.string_units || text.len() > self.limits.string_bytes {
            return Err(failure(None, None, GraphErrorKind::Budget));
        }
        let mut units = Vec::new();
        self.budget.reserve(&mut units, count)?;
        units.extend(text.encode_utf16());
        self.push(Node::String(JavaString { units, long: false }))
    }

    fn descriptor(&mut self, name: &'static str) -> Result<NodeId, GraphError> {
        if let Some((_, id)) = self.descriptors.iter().find(|(saved, _)| *saved == name) { return Ok(*id); }
        let class = class_by_name(name).ok_or(failure(None, None, GraphErrorKind::Class))?;
        let schema = class_schema(class);
        if name.encode_utf16().count() > self.limits.name_units
            || schema.fields.iter().any(|field| field.name.encode_utf16().count() > self.limits.name_units)
        {
            return Err(failure(None, None, GraphErrorKind::Budget));
        }
        let superclass = match schema.superclass {
            Some(parent) => Some(self.descriptor(parent)?),
            None => None,
        };
        let mut signatures = Vec::new();
        self.budget.reserve(&mut signatures, schema.fields.len())?;
        for field in schema.fields {
            let value = match field.signature {
                None => None,
                Some(text) => {
                    let existing = self.signatures.iter().find(|(saved, _)| *saved == text).map(|(_, id)| *id);
                    Some(match existing {
                        Some(id) => id,
                        None => {
                            let id = self.string(text)?;
                            self.signatures.push((text, id));
                            id
                        }
                    })
                }
            };
            signatures.push(value);
        }
        let id = self.push(Node::Descriptor(ClassDescriptor {
            class, signatures, annotations: Vec::new(), superclass,
        }))?;
        self.descriptors.push((name, id));
        Ok(id)
    }

    fn object(&mut self, descriptor: NodeId, slots: Vec<ClassData>) -> Result<NodeId, GraphError> {
        self.push(Node::Object(JavaObject { descriptor, slots }))
    }

    fn object_vectors(&mut self, slots: usize, fields: usize, custom: usize) -> Result<(), GraphError> {
        self.budget.charge(product(slots, size_of::<ClassData>())?)?;
        self.budget.charge(product(fields, size_of::<FieldValue>())?)?;
        self.budget.charge(product(custom, size_of::<Record>())?)
    }

    fn array_count(&mut self, count: usize) -> Result<(), GraphError> {
        let total = self.elements.checked_add(count).ok_or(failure(None, None, GraphErrorKind::Budget))?;
        if count > self.limits.array_elements || total > self.limits.total_array_elements {
            return Err(failure(None, None, GraphErrorKind::Budget));
        }
        self.elements = total;
        Ok(())
    }

    fn float_value(&self, value: Option<&JavaFloatArray>) -> Value {
        value.map_or(Value::Null, |value| {
            let index = self.floats.binary_search_by_key(&value.identity(), JavaFloatArray::identity).expect("Collected float owner");
            Value::Handle(self.float_ids[index])
        })
    }

    fn space_value(&self, value: Option<&JavaIccColorSpace>) -> Value {
        value.map_or(Value::Null, |value| {
            let index = self.spaces.binary_search_by_key(&value.identity(), JavaIccColorSpace::identity).expect("Collected space owner");
            Value::Handle(self.space_ids[index])
        })
    }

    fn profile_value(&self, value: Option<&JavaIccProfile>) -> Value {
        value.map_or(Value::Null, |value| {
            let index = self.profiles.binary_search_by_key(&value.identity(), JavaIccProfile::identity).expect("Collected profile owner");
            Value::Handle(self.profile_ids[index])
        })
    }

    fn array_value(&self, value: Option<&ColorArrayRef>) -> Value {
        value.map_or(Value::Null, |value| {
            let index = self.arrays.binary_search_by_key(&value.identity(), ColorArrayRef::identity)
                .expect("Collected Color array owner");
            Value::Handle(self.array_ids[index])
        })
    }

    fn color_value(&self, value: Option<&JavaColor>) -> Value {
        value.map_or(Value::Null, |value| {
            let index = self.colors.binary_search_by_key(&value.identity(), JavaColor::identity).expect("Collected Color owner");
            Value::Handle(self.color_ids[index])
        })
    }

    fn build(&mut self) -> Result<(), GraphError> {
        for index in 0..self.floats.len() {
            let value = self.floats[index].clone();
            self.array_count(value.bits().len())?;
            let mut bits = Vec::new();
            self.budget.reserve(&mut bits, value.bits().len())?;
            bits.extend_from_slice(value.bits());
            let descriptor = self.descriptor(FLOATS)?;
            let id = self.push(Node::Array(JavaArray { descriptor, data: ArrayData::Floats(bits) }))?;
            self.float_ids.push(id);
        }
        for index in 0..self.profiles.len() {
            let value = self.profiles[index].clone();
            let class = value.kind().class_name();
            let descriptor = self.descriptor(class)?;
            let base = self.descriptor(PROFILE)?;
            let name = self.string(value.kind().serialized_name())?;
            self.object_vectors(if class == PROFILE { 1 } else { 2 }, 1, 2)?;
            let mut slots = Vec::new();
            slots.try_reserve_exact(if class == PROFILE { 1 } else { 2 })
                .map_err(|_| failure(None, None, GraphErrorKind::Allocation))?;
            slots.push(ClassData {
                descriptor: base, values: vec![FieldValue::Int(1)],
                custom: vec![Record::Value(Value::Handle(name)), Record::Value(Value::Null)],
            });
            if class != PROFILE {
                slots.push(ClassData { descriptor, values: Vec::new(), custom: Vec::new() });
            }
            let id = self.object(descriptor, slots)?;
            self.profile_ids.push(id);
        }
        for index in 0..self.spaces.len() {
            let value = self.spaces[index].clone();
            let data = value.parts();
            let descriptor = self.descriptor(SPACE)?;
            let base = self.descriptor(SPACE_BASE)?;
            self.object_vectors(2, 8, 0)?;
            let slots = vec![
                ClassData { descriptor: base, values: vec![
                    FieldValue::Int(data.num_components), FieldValue::Int(data.space_type),
                ], custom: Vec::new() },
                ClassData { descriptor, values: vec![
                    FieldValue::Boolean(u8::from(data.need_scale_init)),
                    FieldValue::Reference(self.float_value(data.diff_min_max.as_ref())),
                    FieldValue::Reference(self.float_value(data.inv_diff_min_max.as_ref())),
                    FieldValue::Reference(self.float_value(data.max_val.as_ref())),
                    FieldValue::Reference(self.float_value(data.min_val.as_ref())),
                    FieldValue::Reference(self.profile_value(data.profile.as_ref())),
                ], custom: Vec::new() },
            ];
            let id = self.object(descriptor, slots)?;
            self.space_ids.push(id);
        }
        for index in 0..self.colors.len() {
            let value = self.colors[index].clone();
            let data = value.parts();
            let descriptor = self.descriptor(COLOR)?;
            self.object_vectors(1, 5, 0)?;
            let slots = vec![ClassData { descriptor, values: vec![
                FieldValue::FloatBits(data.alpha_bits), FieldValue::Int(data.argb as i32),
                FieldValue::Reference(self.space_value(data.space.as_ref())),
                FieldValue::Reference(self.float_value(data.rgb.as_ref())),
                FieldValue::Reference(self.float_value(data.components.as_ref())),
            ], custom: Vec::new() }];
            let id = self.object(descriptor, slots)?;
            self.color_ids.push(id);
        }
        for index in 0..self.arrays.len() {
            let array = self.arrays[index].clone();
            self.array_count(array.len())?;
            let descriptor = self.descriptor(COLORS)?;
            let mut references = Vec::new();
            self.budget.reserve(&mut references, array.len())?;
            for value in array.slots().iter() {
                references.push(self.color_value(value.as_ref()));
            }
            let id = self.push(Node::Array(JavaArray { descriptor, data: ArrayData::References(references) }))?;
            self.array_ids.push(id);
        }

        Ok(())
    }
}

/// Build detached typed holders without publishing a partial graph
pub(crate) fn emit_color_document(
    root: ColorRoot<'_>,
    limits: Limits,
    additional_budget_bytes: usize,
) -> Result<Document, GraphError> {
    emit_color_documents(std::slice::from_ref(&root), limits, additional_budget_bytes)
}

/// Batch related Gene, phenotype and temporary holders through one owner map
pub(crate) fn emit_color_documents(
    roots: &[ColorRoot<'_>],
    limits: Limits,
    additional_budget_bytes: usize,
) -> Result<Document, GraphError> {
    Ok(emit_color_documents_charged(roots, limits, additional_budget_bytes)?.0)
}

/// Return the bounded peak allocation ledger when another typed closure shares this batch
pub(crate) fn emit_color_documents_charged(
    roots: &[ColorRoot<'_>], limits: Limits, additional_budget_bytes: usize,
) -> Result<(Document, usize), GraphError> {
    let mut emitter = ColorEmitter::new(roots, limits, additional_budget_bytes)?;
    emitter.build()?;
    let mut records = Vec::new();
    emitter.budget.reserve(&mut records, roots.len())?;
    for root in roots {
        let value = match *root {
            ColorRoot::Color(value) => emitter.color_value(value),
            ColorRoot::Floats(value) => emitter.float_value(value),
            ColorRoot::Colors(value) => emitter.array_value(value),
        };
        records.push(Record::Value(value));
    }
    Ok((Document { arena: emitter.arena, records, consumed: 0 }, emitter.budget.used))
}

#[cfg(test)]
#[path = "legacy_color_tests.rs"]
mod tests;
