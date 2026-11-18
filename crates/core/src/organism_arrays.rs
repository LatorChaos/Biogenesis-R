// SPDX-License-Identifier: GPL-2.0-or-later

//! The forty-eight independently nullable primitive arrays stored by Organism

use crate::GeometryError;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntArrayRef(Rc<RefCell<Vec<i32>>>);

impl IntArrayRef {
    pub fn from_values(values: Vec<i32>) -> Self {
        Self(Rc::new(RefCell::new(values)))
    }
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
    pub fn same_instance(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn borrow_values(&self) -> std::cell::Ref<'_, Vec<i32>> {
        self.0.borrow()
    }
    pub fn values(&self) -> Vec<i32> {
        self.0.borrow().clone()
    }
    pub fn get(&self, index: i32, field: &'static str) -> Result<i32, GeometryError> {
        let values = self.0.borrow();
        values
            .get(index as usize)
            .copied()
            .ok_or(GeometryError::PrimitiveIndex {
                field,
                index,
                length: values.len(),
            })
    }
    pub fn replace(
        &self,
        index: i32,
        value: i32,
        field: &'static str,
    ) -> Result<(), GeometryError> {
        let mut values = self.0.borrow_mut();
        let length = values.len();
        *values
            .get_mut(index as usize)
            .ok_or(GeometryError::PrimitiveIndex {
                field,
                index,
                length,
            })? = value;
        Ok(())
    }
    fn copied(&self, owners: &mut HashMap<usize, Self>) -> Self {
        owners
            .entry(self.identity())
            .or_insert_with(|| Self::from_values(self.values()))
            .clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoubleArrayRef(Rc<RefCell<Vec<u64>>>);

impl DoubleArrayRef {
    pub fn from_bits(values: Vec<u64>) -> Self {
        Self(Rc::new(RefCell::new(values)))
    }
    pub fn from_values(values: Vec<f64>) -> Self {
        Self::from_bits(values.into_iter().map(f64::to_bits).collect())
    }
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
    pub fn same_instance(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn borrow_bits(&self) -> std::cell::Ref<'_, Vec<u64>> {
        self.0.borrow()
    }
    pub fn bits(&self) -> Vec<u64> {
        self.0.borrow().clone()
    }
    pub fn get_bits(&self, index: i32, field: &'static str) -> Result<u64, GeometryError> {
        let values = self.0.borrow();
        values
            .get(index as usize)
            .copied()
            .ok_or(GeometryError::PrimitiveIndex {
                field,
                index,
                length: values.len(),
            })
    }
    pub fn get(&self, index: i32, field: &'static str) -> Result<f64, GeometryError> {
        Ok(f64::from_bits(self.get_bits(index, field)?))
    }
    pub fn replace_bits(
        &self,
        index: i32,
        value: u64,
        field: &'static str,
    ) -> Result<(), GeometryError> {
        let mut values = self.0.borrow_mut();
        let length = values.len();
        *values
            .get_mut(index as usize)
            .ok_or(GeometryError::PrimitiveIndex {
                field,
                index,
                length,
            })? = value;
        Ok(())
    }
    pub fn replace(
        &self,
        index: i32,
        value: f64,
        field: &'static str,
    ) -> Result<(), GeometryError> {
        self.replace_bits(index, value.to_bits(), field)
    }
    fn copied(&self, owners: &mut HashMap<usize, Self>) -> Self {
        owners
            .entry(self.identity())
            .or_insert_with(|| Self::from_bits(self.bits()))
            .clone()
    }
}

/// Channel order follows the source Mimic stores; create uses its separate store order
pub const REACTION_FIELDS: [&str; 36] = [
    "_segredReaction",
    "_seggreenReaction",
    "_segblueReaction",
    "_segplagueReaction",
    "_segscourgeReaction",
    "_segwhiteReaction",
    "_seggrayReaction",
    "_segsilverReaction",
    "_segdefaultReaction",
    "_segconsumerReaction",
    "_segplantReaction",
    "_segmagentaReaction",
    "_segpinkReaction",
    "_segcoralReaction",
    "_segorangeReaction",
    "_segbarkReaction",
    "_segvioletReaction",
    "_segvirusReaction",
    "_segmaroonReaction",
    "_segoliveReaction",
    "_segmintReaction",
    "_segcreamReaction",
    "_segspikeReaction",
    "_segfallowReaction",
    "_seglightblueReaction",
    "_segochreReaction",
    "_segskyReaction",
    "_seglilacReaction",
    "_segfireReaction",
    "_seglightbrownReaction",
    "_seggreenbrownReaction",
    "_segbrownReaction",
    "_segiceReaction",
    "_segbrokenReaction",
    "_segsickReaction",
    "_segfriendReaction",
];
pub(crate) const REACTION_STORE_ORDER: [usize; 36] = [
    0, 1, 2, 3, 4, 5, 6, 8, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 24, 25, 29, 31, 34, 26,
    27, 32, 33, 7, 28, 35, 30, 23, 9, 10,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntField {
    StartX,
    StartY,
    EndX,
    EndY,
    X1,
    Y1,
    X2,
    Y2,
    Reaction(usize),
}
impl IntField {
    pub fn name(self) -> &'static str {
        match self {
            Self::StartX => "_startPointX",
            Self::StartY => "_startPointY",
            Self::EndX => "_endPointX",
            Self::EndY => "_endPointY",
            Self::X1 => "x1",
            Self::Y1 => "y1",
            Self::X2 => "x2",
            Self::Y2 => "y2",
            Self::Reaction(channel) => REACTION_FIELDS[channel],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoubleField {
    Length,
    StartRadius,
    EndRadius,
    Photo,
}
impl DoubleField {
    pub fn name(self) -> &'static str {
        match self {
            Self::Length => "_m",
            Self::StartRadius => "_m1",
            Self::EndRadius => "_m2",
            Self::Photo => "_mphoto",
        }
    }
}

/// Each field holds its actual owner; no count or sibling field determines its length
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganismArrays {
    pub start_x: Option<IntArrayRef>,
    pub start_y: Option<IntArrayRef>,
    pub end_x: Option<IntArrayRef>,
    pub end_y: Option<IntArrayRef>,
    pub x1: Option<IntArrayRef>,
    pub y1: Option<IntArrayRef>,
    pub x2: Option<IntArrayRef>,
    pub y2: Option<IntArrayRef>,
    pub reactions: [Option<IntArrayRef>; 36],
    pub length: Option<DoubleArrayRef>,
    pub start_radius: Option<DoubleArrayRef>,
    pub end_radius: Option<DoubleArrayRef>,
    pub photo: Option<DoubleArrayRef>,
}
impl Default for OrganismArrays {
    fn default() -> Self {
        Self {
            start_x: None,
            start_y: None,
            end_x: None,
            end_y: None,
            x1: None,
            y1: None,
            x2: None,
            y2: None,
            reactions: std::array::from_fn(|_| None),
            length: None,
            start_radius: None,
            end_radius: None,
            photo: None,
        }
    }
}
impl OrganismArrays {
    pub fn int(&self, field: IntField) -> &Option<IntArrayRef> {
        match field {
            IntField::StartX => &self.start_x,
            IntField::StartY => &self.start_y,
            IntField::EndX => &self.end_x,
            IntField::EndY => &self.end_y,
            IntField::X1 => &self.x1,
            IntField::Y1 => &self.y1,
            IntField::X2 => &self.x2,
            IntField::Y2 => &self.y2,
            IntField::Reaction(channel) => &self.reactions[channel],
        }
    }
    pub fn int_mut(&mut self, field: IntField) -> &mut Option<IntArrayRef> {
        match field {
            IntField::StartX => &mut self.start_x,
            IntField::StartY => &mut self.start_y,
            IntField::EndX => &mut self.end_x,
            IntField::EndY => &mut self.end_y,
            IntField::X1 => &mut self.x1,
            IntField::Y1 => &mut self.y1,
            IntField::X2 => &mut self.x2,
            IntField::Y2 => &mut self.y2,
            IntField::Reaction(channel) => &mut self.reactions[channel],
        }
    }
    pub fn double(&self, field: DoubleField) -> &Option<DoubleArrayRef> {
        match field {
            DoubleField::Length => &self.length,
            DoubleField::StartRadius => &self.start_radius,
            DoubleField::EndRadius => &self.end_radius,
            DoubleField::Photo => &self.photo,
        }
    }
    pub fn double_mut(&mut self, field: DoubleField) -> &mut Option<DoubleArrayRef> {
        match field {
            DoubleField::Length => &mut self.length,
            DoubleField::StartRadius => &mut self.start_radius,
            DoubleField::EndRadius => &mut self.end_radius,
            DoubleField::Photo => &mut self.photo,
        }
    }
    pub fn read_int(&self, field: IntField, index: i32) -> Result<i32, GeometryError> {
        self.int(field)
            .as_ref()
            .ok_or(GeometryError::MissingPrimitiveArray {
                field: field.name(),
            })?
            .get(index, field.name())
    }
    pub fn read_double(&self, field: DoubleField, index: i32) -> Result<f64, GeometryError> {
        self.double(field)
            .as_ref()
            .ok_or(GeometryError::MissingPrimitiveArray {
                field: field.name(),
            })?
            .get(index, field.name())
    }
    /// Capture the destination owner before evaluating a Java assignment's RHS
    pub fn store_int(
        owner: Option<IntArrayRef>,
        field: IntField,
        index: i32,
        value: i32,
    ) -> Result<(), GeometryError> {
        owner
            .ok_or(GeometryError::MissingPrimitiveArray {
                field: field.name(),
            })?
            .replace(index, value, field.name())
    }
    pub fn store_double(
        owner: Option<DoubleArrayRef>,
        field: DoubleField,
        index: i32,
        value: f64,
    ) -> Result<(), GeometryError> {
        owner
            .ok_or(GeometryError::MissingPrimitiveArray {
                field: field.name(),
            })?
            .replace(index, value, field.name())
    }
    pub fn write_int(&self, field: IntField, index: i32, value: i32) -> Result<(), GeometryError> {
        Self::store_int(self.int(field).clone(), field, index, value)
    }
    pub fn write_double(
        &self,
        field: DoubleField,
        index: i32,
        value: f64,
    ) -> Result<(), GeometryError> {
        Self::store_double(self.double(field).clone(), field, index, value)
    }
    pub(crate) fn allocate_int(
        &mut self,
        field: IntField,
        count: usize,
    ) -> Result<(), GeometryError> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| GeometryError::Capacity)?;
        values.resize(count, 0);
        *self.int_mut(field) = Some(IntArrayRef::from_values(values));
        Ok(())
    }
    pub(crate) fn allocate_double(
        &mut self,
        field: DoubleField,
        count: usize,
    ) -> Result<(), GeometryError> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| GeometryError::Capacity)?;
        values.resize(count, 0);
        *self.double_mut(field) = Some(DoubleArrayRef::from_bits(values));
        Ok(())
    }
    pub(crate) fn copy_with_owners(
        &self,
        ints: &mut HashMap<usize, IntArrayRef>,
        doubles: &mut HashMap<usize, DoubleArrayRef>,
    ) -> Self {
        Self {
            start_x: self.start_x.as_ref().map(|v| v.copied(ints)),
            start_y: self.start_y.as_ref().map(|v| v.copied(ints)),
            end_x: self.end_x.as_ref().map(|v| v.copied(ints)),
            end_y: self.end_y.as_ref().map(|v| v.copied(ints)),
            x1: self.x1.as_ref().map(|v| v.copied(ints)),
            y1: self.y1.as_ref().map(|v| v.copied(ints)),
            x2: self.x2.as_ref().map(|v| v.copied(ints)),
            y2: self.y2.as_ref().map(|v| v.copied(ints)),
            reactions: std::array::from_fn(|i| self.reactions[i].as_ref().map(|v| v.copied(ints))),
            length: self.length.as_ref().map(|v| v.copied(doubles)),
            start_radius: self.start_radius.as_ref().map(|v| v.copied(doubles)),
            end_radius: self.end_radius.as_ref().map(|v| v.copied(doubles)),
            photo: self.photo.as_ref().map(|v| v.copied(doubles)),
        }
    }
}

impl crate::Body {
    fn primitive_index(index: usize) -> Result<i32, GeometryError> {
        i32::try_from(index).map_err(|_| GeometryError::PrimitiveIndexDomain { index })
    }

    pub fn forward_count(&self) -> usize {
        self.segment_count.max(0) as usize
    }
    pub fn indices(&self) -> std::ops::Range<usize> {
        0..self.forward_count()
    }
    pub fn reverse_indices(
        &self,
    ) -> std::iter::Map<std::iter::Rev<std::ops::RangeInclusive<i32>>, fn(i32) -> usize> {
        (0..=self.segment_count.wrapping_sub(1))
            .rev()
            .map(|index| index as usize)
    }
    pub fn start(&self, index: usize, axis: usize) -> Result<i32, GeometryError> {
        self.arrays.read_int(
            if axis == 0 {
                IntField::StartX
            } else {
                IntField::StartY
            },
            Self::primitive_index(index)?,
        )
    }
    pub fn end(&self, index: usize, axis: usize) -> Result<i32, GeometryError> {
        self.arrays.read_int(
            if axis == 0 {
                IntField::EndX
            } else {
                IntField::EndY
            },
            Self::primitive_index(index)?,
        )
    }
    pub fn rotated_start(&self, index: usize, axis: usize) -> Result<i32, GeometryError> {
        self.arrays.read_int(
            if axis == 0 {
                IntField::X1
            } else {
                IntField::Y1
            },
            Self::primitive_index(index)?,
        )
    }
    pub fn rotated_end(&self, index: usize, axis: usize) -> Result<i32, GeometryError> {
        self.arrays.read_int(
            if axis == 0 {
                IntField::X2
            } else {
                IntField::Y2
            },
            Self::primitive_index(index)?,
        )
    }
    pub fn physical_length(&self, index: usize) -> Result<f64, GeometryError> {
        self.arrays
            .read_double(DoubleField::Length, Self::primitive_index(index)?)
    }
    pub fn start_radius(&self, index: usize) -> Result<f64, GeometryError> {
        self.arrays
            .read_double(DoubleField::StartRadius, Self::primitive_index(index)?)
    }
    pub fn end_radius(&self, index: usize) -> Result<f64, GeometryError> {
        self.arrays
            .read_double(DoubleField::EndRadius, Self::primitive_index(index)?)
    }
    pub fn photo(&self, index: usize) -> Result<f64, GeometryError> {
        self.arrays
            .read_double(DoubleField::Photo, Self::primitive_index(index)?)
    }
    pub fn reaction(&self, index: usize, channel: usize) -> Result<i32, GeometryError> {
        self.arrays
            .read_int(IntField::Reaction(channel), Self::primitive_index(index)?)
    }
    pub fn set_start(&self, index: usize, axis: usize, value: i32) -> Result<(), GeometryError> {
        self.arrays.write_int(
            if axis == 0 {
                IntField::StartX
            } else {
                IntField::StartY
            },
            Self::primitive_index(index)?,
            value,
        )
    }
    pub fn set_end(&self, index: usize, axis: usize, value: i32) -> Result<(), GeometryError> {
        self.arrays.write_int(
            if axis == 0 {
                IntField::EndX
            } else {
                IntField::EndY
            },
            Self::primitive_index(index)?,
            value,
        )
    }
    pub fn set_rotated_start(
        &self,
        index: usize,
        axis: usize,
        value: i32,
    ) -> Result<(), GeometryError> {
        self.arrays.write_int(
            if axis == 0 {
                IntField::X1
            } else {
                IntField::Y1
            },
            Self::primitive_index(index)?,
            value,
        )
    }
    pub fn set_rotated_end(
        &self,
        index: usize,
        axis: usize,
        value: i32,
    ) -> Result<(), GeometryError> {
        self.arrays.write_int(
            if axis == 0 {
                IntField::X2
            } else {
                IntField::Y2
            },
            Self::primitive_index(index)?,
            value,
        )
    }
    pub fn set_physical_length(&self, index: usize, value: f64) -> Result<(), GeometryError> {
        self.arrays
            .write_double(DoubleField::Length, Self::primitive_index(index)?, value)
    }
    pub fn set_photo(&self, index: usize, value: f64) -> Result<(), GeometryError> {
        self.arrays
            .write_double(DoubleField::Photo, Self::primitive_index(index)?, value)
    }
    pub fn set_reaction(
        &self,
        index: usize,
        channel: usize,
        value: i32,
    ) -> Result<(), GeometryError> {
        self.arrays.write_int(
            IntField::Reaction(channel),
            Self::primitive_index(index)?,
            value,
        )
    }
    /// A temporary inspection projection; mechanics access individual reached fields
    pub fn segment(&self, index: usize) -> Result<crate::Segment, GeometryError> {
        let mut reactions = [0; 36];
        for (channel, value) in reactions.iter_mut().enumerate() {
            *value = self.reaction(index, channel)?;
        }
        Ok(crate::Segment {
            start: [self.start(index, 0)?, self.start(index, 1)?],
            end: [self.end(index, 0)?, self.end(index, 1)?],
            reactions,
            physical_length: self.physical_length(index)?,
            start_radius: self.start_radius(index)?,
            end_radius: self.end_radius(index)?,
        })
    }
    pub fn segment_values(&self) -> Result<Vec<crate::Segment>, GeometryError> {
        self.indices().map(|i| self.segment(i)).collect()
    }
    pub fn rotated_segment(&self, index: usize) -> Result<crate::RotatedSegment, GeometryError> {
        Ok(crate::RotatedSegment {
            start: [self.rotated_start(index, 0)?, self.rotated_start(index, 1)?],
            end: [self.rotated_end(index, 0)?, self.rotated_end(index, 1)?],
        })
    }
    pub fn rotated_values(&self) -> Result<Vec<crate::RotatedSegment>, GeometryError> {
        self.indices().map(|i| self.rotated_segment(i)).collect()
    }
    pub fn photo_values(&self) -> Result<Vec<f64>, GeometryError> {
        self.indices().map(|i| self.photo(i)).collect()
    }
    pub fn reaction_values(&self, index: usize) -> Result<[i32; 36], GeometryError> {
        let mut result = [0; 36];
        for (c, value) in result.iter_mut().enumerate() {
            *value = self.reaction(index, c)?;
        }
        Ok(result)
    }
    pub fn set_reaction_values(
        &self,
        index: usize,
        values: [i32; 36],
    ) -> Result<(), GeometryError> {
        for (channel, value) in values.into_iter().enumerate() {
            self.set_reaction(index, channel, value)?;
        }
        Ok(())
    }
    pub fn set_rotated_segment(
        &self,
        index: usize,
        value: crate::RotatedSegment,
    ) -> Result<(), GeometryError> {
        self.set_rotated_start(index, 0, value.start[0])?;
        self.set_rotated_start(index, 1, value.start[1])?;
        self.set_rotated_end(index, 0, value.end[0])?;
        self.set_rotated_end(index, 1, value.end[1])
    }
}

impl crate::Body {
    pub fn set_rotated_values(&mut self, values: &[crate::RotatedSegment]) {
        self.arrays.x1 = Some(IntArrayRef::from_values(
            values.iter().map(|s| s.start[0]).collect(),
        ));
        self.arrays.y1 = Some(IntArrayRef::from_values(
            values.iter().map(|s| s.start[1]).collect(),
        ));
        self.arrays.x2 = Some(IntArrayRef::from_values(
            values.iter().map(|s| s.end[0]).collect(),
        ));
        self.arrays.y2 = Some(IntArrayRef::from_values(
            values.iter().map(|s| s.end[1]).collect(),
        ));
    }
}

pub(crate) const INT_FIELDS: [IntField; 44] = [
    IntField::StartX,
    IntField::StartY,
    IntField::EndX,
    IntField::EndY,
    IntField::Reaction(0),
    IntField::Reaction(1),
    IntField::Reaction(2),
    IntField::Reaction(3),
    IntField::Reaction(4),
    IntField::Reaction(5),
    IntField::Reaction(6),
    IntField::Reaction(7),
    IntField::Reaction(8),
    IntField::Reaction(9),
    IntField::Reaction(10),
    IntField::Reaction(11),
    IntField::Reaction(12),
    IntField::Reaction(13),
    IntField::Reaction(14),
    IntField::Reaction(15),
    IntField::Reaction(16),
    IntField::Reaction(17),
    IntField::Reaction(18),
    IntField::Reaction(19),
    IntField::Reaction(20),
    IntField::Reaction(21),
    IntField::Reaction(22),
    IntField::Reaction(23),
    IntField::Reaction(24),
    IntField::Reaction(25),
    IntField::Reaction(26),
    IntField::Reaction(27),
    IntField::Reaction(28),
    IntField::Reaction(29),
    IntField::Reaction(30),
    IntField::Reaction(31),
    IntField::Reaction(32),
    IntField::Reaction(33),
    IntField::Reaction(34),
    IntField::Reaction(35),
    IntField::X1,
    IntField::Y1,
    IntField::X2,
    IntField::Y2,
];
pub(crate) const DOUBLE_FIELDS: [DoubleField; 4] = [
    DoubleField::Length,
    DoubleField::StartRadius,
    DoubleField::EndRadius,
    DoubleField::Photo,
];
