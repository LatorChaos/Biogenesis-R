// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-only

//! Cropped cached-phenotype images from Organism.java:19325-19334
//!
//! Raster arithmetic is expressed independently from the installed OpenJDK21 native
//! DrawLine behavior and the OpenJDK8u AlphaMath rounding contract
//! See the accompanying source map for binary custody and compatibility limits

use super::*;
use crate::RotatedSegment;

#[path = "image_alpha.rs"]
mod alpha;

/// An opaque black-background image in row-major RGB8 order
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganismImage {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

/// Image preparation fails before exposing a partial image or mutating the world
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganismImageError {
    InvalidHandle,
    NotAlive,
    InvalidDimensions { width: i32, height: i32 },
    CacheShape { segments: usize, cached: usize },
    SizeOverflow,
    BudgetExceeded { required: usize, budget: usize },
    Allocation,
    Color(crate::GeometryError),
}

impl fmt::Display for OrganismImageError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{self:?}")
    }
}

impl std::error::Error for OrganismImageError {}

#[derive(Clone, Copy)]
struct ImageLine {
    start: [i32; 2],
    end: [i32; 2],
    argb: u32,
}

struct ImagePlan {
    width: u32,
    height: u32,
    pixel_bytes: usize,
    lines: Vec<ImageLine>,
}

impl World {
    /// Read a current living member's cached phenotype without advancing simulation
    /// The budget covers RGB8 bytes and line-command storage, excluding allocator overhead
    pub fn organism_image(
        &self,
        handle: &OrganismHandle,
        budget_bytes: usize,
    ) -> Result<OrganismImage, OrganismImageError> {
        if !self.contains_handle(handle) {
            return Err(OrganismImageError::InvalidHandle);
        }
        let actor = handle.0.borrow();
        if !actor.state.alive {
            return Err(OrganismImageError::NotAlive);
        }
        image_from_organism(&actor, budget_bytes)
    }
}

// The original direct getImage method has no membership or alive guard
pub(super) fn image_from_organism(
    actor: &Organism,
    budget_bytes: usize,
) -> Result<OrganismImage, OrganismImageError> {
    let plan = ImagePlan::from_actor(actor, budget_bytes)?;
    let mut rgb = Vec::new();
    rgb.try_reserve_exact(plan.pixel_bytes)
        .map_err(|_| OrganismImageError::Allocation)?;
    rgb.resize(plan.pixel_bytes, 0);
    for line in plan.lines {
        raster_line(&mut rgb, plan.width, plan.height, line);
    }
    Ok(OrganismImage {
        width: plan.width,
        height: plan.height,
        rgb,
    })
}

impl ImagePlan {
    fn from_actor(actor: &Organism, budget: usize) -> Result<Self, OrganismImageError> {
        let bounds = actor.state.bounds.get();
        if bounds.width <= 0 || bounds.height <= 0 {
            return Err(OrganismImageError::InvalidDimensions {
                width: bounds.width,
                height: bounds.height,
            });
        }
        let segments = actor.body.forward_count();
        let width = bounds.width as u32;
        let height = bounds.height as u32;
        let pixel_bytes = (width as usize)
            .checked_mul(height as usize)
            .and_then(|size| size.checked_mul(3))
            .filter(|size| *size <= isize::MAX as usize)
            .ok_or(OrganismImageError::SizeOverflow)?;
        let command_bytes = segments
            .checked_mul(std::mem::size_of::<ImageLine>())
            .filter(|size| *size <= isize::MAX as usize)
            .ok_or(OrganismImageError::SizeOverflow)?;
        let required = pixel_bytes
            .checked_add(command_bytes)
            .ok_or(OrganismImageError::SizeOverflow)?;
        if required > budget {
            return Err(OrganismImageError::BudgetExceeded { required, budget });
        }
        let mut lines = Vec::new();
        lines
            .try_reserve_exact(segments)
            .map_err(|_| OrganismImageError::Allocation)?;
        let translate = |point: [i32; 2]| {
            [
                point[0]
                    .wrapping_sub(bounds.x)
                    .wrapping_add(actor.state.integer_center[0]),
                point[1]
                    .wrapping_sub(bounds.y)
                    .wrapping_add(actor.state.integer_center[1]),
            ]
        };
        let mut paint = 0xffff_ffff;
        for index in actor.body.reverse_indices() {
            let color = actor.body.color(index).map_err(OrganismImageError::Color)?;
            if let Some(color) = color {
                paint = color.argb();
            }
            let cached = actor.body.rotated_segment(index).map_err(OrganismImageError::Color)?;
            lines.push(ImageLine {
                start: translate(cached.start), end: translate(cached.end), argb: paint,
            });
        }
        Ok(Self {
            width,
            height,
            pixel_bytes,
            lines,
        })
    }
}

// Native DrawLine reduces non-axis endpoints to signed30-bit coordinates before
// Bresenham setup; preserve that floating preclip before the bounded integer sampler
fn preclip(mut start: [i32; 2], mut end: [i32; 2]) -> ([i32; 2], [i32; 2]) {
    const LOW: f64 = -536_870_912.0;
    const HIGH: f64 = 536_870_911.0;
    if start[0] == end[0]
        || start[1] == end[1]
        || start
            .into_iter()
            .chain(end)
            .all(|n| (-536_870_912..=536_870_911).contains(&n))
    {
        return (start, end);
    }
    let original_start = start.map(f64::from);
    let original_end = end.map(f64::from);
    let delta = [
        original_end[0] - original_start[0],
        original_end[1] - original_start[1],
    ];
    let mut a = original_start;
    let mut b = original_end;
    for axis in [1, 0] {
        let clipped = a[axis].clamp(LOW, HIGH);
        if clipped != a[axis] {
            let other = 1 - axis;
            a[other] = original_start[other]
                + (clipped - original_start[axis]) * delta[other] / delta[axis];
            a[axis] = clipped;
        }
    }
    for axis in [0, 1] {
        let clipped = b[axis].clamp(LOW, HIGH);
        if clipped != b[axis] {
            let other = 1 - axis;
            b[other] =
                original_end[other] + (clipped - original_end[axis]) * delta[other] / delta[axis];
            b[axis] = clipped;
        }
    }
    // Installed x86_64 DrawLine uses cvttsd2si, whose out-of-range result is INT_MIN
    let truncate = |n: f64| {
        if n >= f64::from(i32::MIN) && n < f64::from(i32::MAX) + 1.0 {
            n as i32
        } else {
            i32::MIN
        }
    };
    start = a.map(truncate);
    end = b.map(truncate);
    (start, end)
}

fn raster_line(rgb: &mut [u8], width: u32, height: u32, line: ImageLine) {
    let alpha = line.argb >> 24;
    if alpha == 0 {
        return;
    }
    if alpha < 255 {
        alpha::raster_alpha_line(rgb, width, height, line);
        return;
    }
    let (start, end) = preclip(line.start, line.end);
    let delta = [
        i64::from(end[0]) - i64::from(start[0]),
        i64::from(end[1]) - i64::from(start[1]),
    ];
    let distance = delta.map(i64::abs);
    let major = usize::from(distance[1] > distance[0]);
    let minor = 1 - major;
    let length = distance[major];
    let extent = i64::from(if major == 0 { width } else { height });
    let origin = i64::from(start[major]);
    let direction = if delta[major] < 0 { -1 } else { 1 };
    let (first, last) = if direction > 0 {
        ((-origin).max(0), (extent - 1 - origin).min(length))
    } else {
        ((origin - extent + 1).max(0), origin.min(length))
    };
    // Iteration is bounded by the image's major extent, never offscreen endpoint distance
    for step in first..=last {
        let offset = if length == 0 {
            0
        } else {
            (i128::from(step) * i128::from(distance[minor]) * 2 + i128::from(length))
                / (i128::from(length) * 2)
        } as i64;
        let mut point = [0_i64; 2];
        point[major] = origin + direction * step;
        point[minor] = i64::from(start[minor]) + if delta[minor] < 0 { -offset } else { offset };
        if point[0] < 0
            || point[0] >= i64::from(width)
            || point[1] < 0
            || point[1] >= i64::from(height)
        {
            continue;
        }
        let index = (point[1] as usize * width as usize + point[0] as usize) * 3;
        blend_pixel(&mut rgb[index..index + 3], line.argb);
    }
}

pub(super) fn raster_preview_segment(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    segment: RotatedSegment,
    argb: u32,
    translation: [i32; 2],
    scale: f64,
) {
    let mut line = ImageLine {
        start: segment.start,
        end: segment.end,
        argb,
    };
    if argb >> 24 == 0 {
        return;
    }
    if argb >> 24 < 255 {
        alpha::raster_preview_alpha_line(rgb, width, height, line, translation, scale);
        return;
    }
    let transform = |point: [i32; 2]| {
        std::array::from_fn(|axis| {
            if scale == 1.0 {
                point[axis].wrapping_add(translation[axis])
            } else {
                // Installed transformed-thin dispatch floors doubles before JVM d2i
                let scaled = f64::from(point[axis]) * scale;
                (scaled + f64::from(translation[axis])).floor() as i32
            }
        })
    };
    line.start = transform(line.start);
    line.end = transform(line.end);
    raster_line(rgb, width, height, line);
}

fn blend_pixel(rgb: &mut [u8], argb: u32) {
    let alpha = argb >> 24;
    for (channel, shift) in [16, 8, 0].into_iter().enumerate() {
        let source = (argb >> shift) & 255;
        let destination = u32::from(rgb[channel]);
        rgb[channel] =
            (multiply_byte(source, alpha) + multiply_byte(destination, 255 - alpha)) as u8;
    }
}

fn multiply_byte(value: u32, alpha: u32) -> u32 {
    (value * alpha + 127) / 255
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gene, GenomeProperties};

    fn member() -> (World, OrganismHandle) {
        let world = World::new(WorldConfig {
            seed: 827364,
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        let genome = Genome::from_parts(
            &[Gene::new(12.0, 0.0, Some(0xff00ff00), 0, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        world.paste_genome(genome, [100, 100]).unwrap().unwrap();
        let bounds = world.organisms.borrow()[0].borrow().state.bounds.get();
        let handle = world.pick_organism([bounds.x, bounds.y]).unwrap().unwrap();
        (world, handle)
    }

    #[test]
    fn public_image_requires_actual_living_membership_and_retains_removed_identity() {
        let (world, handle) = member();
        let original = world.clone();
        let copied = world.clone();
        assert_eq!(
            copied.organism_image(&handle, usize::MAX),
            Err(OrganismImageError::InvalidHandle)
        );
        assert!(world.organism_image(&handle, usize::MAX).is_ok());
        assert_eq!(world, original);
        handle.0.borrow_mut().state.alive = false;
        assert_eq!(
            world.organism_image(&handle, usize::MAX),
            Err(OrganismImageError::NotAlive)
        );
        handle.0.borrow_mut().state.alive = true;
        world.rebuild_spatial();
        world.organisms.borrow_mut().clear();
        assert!(Rc::ptr_eq(&world.frame_objects.borrow()[0], &handle.0));
        assert_eq!(
            world.organism_image(&handle, usize::MAX),
            Err(OrganismImageError::InvalidHandle)
        );
    }

    #[test]
    fn image_preflight_counts_both_allocations_without_a_producer_side_limit() {
        let (world, handle) = member();
        {
            let mut actor = handle.0.borrow_mut();
            actor.state.bounds.set(IntRect {
                x: 0,
                y: 0,
                width: 65,
                height: 2,
            });
            actor.state.integer_center = [0, 0];
            { let value = [0, 0]; actor.body.set_rotated_start(0, 0, value[0]).unwrap(); actor.body.set_rotated_start(0, 1, value[1]).unwrap(); };
            { let value = [64, 0]; actor.body.set_rotated_end(0, 0, value[0]).unwrap(); actor.body.set_rotated_end(0, 1, value[1]).unwrap(); };
        }
        let required = 65 * 2 * 3 + std::mem::size_of::<ImageLine>();
        assert_eq!(
            world.organism_image(&handle, required - 1),
            Err(OrganismImageError::BudgetExceeded {
                required,
                budget: required - 1
            })
        );
        let image = world.organism_image(&handle, required).unwrap();
        assert_eq!((image.width, image.height, image.rgb.len()), (65, 2, 390));
        assert!(
            image.rgb[..195]
                .chunks_exact(3)
                .all(|pixel| pixel == [0, 255, 0])
        );
        assert!(image.rgb[195..].iter().all(|byte| *byte == 0));
        handle.0.borrow().state.bounds.update(|bounds| bounds.width = 0);
        assert_eq!(
            world.organism_image(&handle, usize::MAX),
            Err(OrganismImageError::InvalidDimensions {
                width: 0,
                height: 2
            })
        );
        handle.0.borrow().state.bounds.update(|bounds| bounds.width = 1);
        let empty = crate::IntArrayRef::from_values(vec![]);
        handle.0.borrow_mut().body.arrays.x1 = Some(empty.clone());
        let before = handle.0.borrow().snapshot();
        let random = world.random_snapshot();
        let other_owners = {
            let actor = handle.0.borrow();
            [actor.body.arrays.y1.clone().unwrap(), actor.body.arrays.x2.clone().unwrap(), actor.body.arrays.y2.clone().unwrap()]
        };
        assert_eq!(
            world.organism_image(&handle, usize::MAX),
            Err(OrganismImageError::Color(crate::GeometryError::PrimitiveIndex {
                field: "x1", index: 0, length: 0
            }))
        );
        let actor = handle.0.borrow();
        assert_eq!(actor.snapshot(), before);
        assert_eq!(world.random_snapshot(), random);
        assert!(actor.body.arrays.x1.as_ref().unwrap().same_instance(&empty));
        for (actual, expected) in [actor.body.arrays.y1.as_ref().unwrap(), actor.body.arrays.x2.as_ref().unwrap(), actor.body.arrays.y2.as_ref().unwrap()].into_iter().zip(&other_owners) {
            assert!(actual.same_instance(expected));
        }
    }

    #[test]
    fn extreme_offscreen_axis_line_visits_only_the_image_extent() {
        let mut rgb = [0; 9];
        raster_line(
            &mut rgb,
            3,
            1,
            ImageLine {
                start: [i32::MIN, 0],
                end: [i32::MAX, 0],
                argb: 0xff123456,
            },
        );
        assert_eq!(rgb, [0x12, 0x34, 0x56, 0x12, 0x34, 0x56, 0x12, 0x34, 0x56]);
    }
}
