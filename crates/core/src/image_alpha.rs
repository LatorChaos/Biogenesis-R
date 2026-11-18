/*
 * Copyright (c) 1998, 2025, Oracle and/or its affiliates. All rights reserved.
 * Copyright (c) 2007, 2022, Oracle and/or its affiliates. All rights reserved.
 * Copyright (c) 2007, 2025, Oracle and/or its affiliates. All rights reserved.
 * DO NOT ALTER OR REMOVE COPYRIGHT NOTICES OR THIS FILE HEADER.
 *
 * This code is free software; you can redistribute it and/or modify it
 * under the terms of the GNU General Public License version 2 only, as
 * published by the Free Software Foundation. Oracle designates this
 * particular file as subject to the "Classpath" exception as provided
 * by Oracle in the LICENSE file that accompanied this code.
 *
 * This code is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License
 * version 2 for more details (a copy is included in the LICENSE file that
 * accompanied this code).
 *
 * You should have received a copy of the GNU General Public License version
 * 2 along with this work; if not, write to the Free Software Foundation,
 * Inc., 51 Franklin St, Fifth Floor, Boston, MA 02110-1301 USA.
 *
 * Please contact Oracle, 500 Oracle Parkway, Redwood Shores, CA 94065 USA
 * or visit www.oracle.com if you need additional information or have any
 * questions.
 */
// SPDX-License-Identifier: GPL-2.0-only WITH Classpath-exception-2.0

//! Default non-antialiased Java2D color-shape path for one integer line
//!
//! The installed JDK21 bytecode owns float input, quarter normalization, square
//! caps and clipping subdivision; native ShapeSpanIterator owns pixel-center
//! inclusion, nonzero winding, float edge setup and 31-bit error stepping
//! See ALPHA_SOURCE_MAP.md for primary-source, binary and license custody

use super::{ImageLine, blend_pixel};

const MAX_VERTICES: usize = 24;
const ERROR_MASK: i32 = i32::MAX;
const ERROR_SCALE: f64 = i32::MAX as f64;

fn outcode(point: [f64; 2], clip: [f64; 4]) -> u8 {
    let mut code = 0;
    if point[1] < clip[0] {
        code |= 1;
    }
    if point[1] >= clip[1] {
        code |= 2;
    }
    if point[0] < clip[2] {
        code |= 4;
    }
    if point[0] >= clip[3] {
        code |= 8;
    }
    code
}

// Four clip boundaries produce at most five source line pieces
fn line_pieces(start: [f64; 2], end: [f64; 2], clip: [f64; 4]) -> ([[f64; 2]; 6], usize) {
    let mut points = [[0.0; 2]; 6];
    points[0] = start;
    let codes = outcode(start, clip) | outcode(end, clip);
    let delta = [end[0] - start[0], end[1] - start[1]];
    let mut roots = [0.0; 4];
    let mut count = 0;
    if codes != 0 && delta[0].abs() + delta[1].abs() > 100.0 {
        // CurveClipSplitter pads the already stroke-expanded clip by another quarter pixel
        for (bit, axis, boundary) in [
            (4, 0, clip[2] - 0.25),
            (8, 0, clip[3] + 0.25),
            (1, 1, clip[0] - 0.25),
            (2, 1, clip[1] + 0.25),
        ] {
            if codes & bit != 0 && delta[axis] != 0.0 {
                let root = -(start[axis] - boundary) / delta[axis];
                if (0.0..1.0).contains(&root) {
                    roots[count] = root;
                    count += 1;
                }
            }
        }
        roots[..count].sort_unstable_by(f64::total_cmp);
    }
    let mut current = start;
    let mut previous_t = 0.0;
    for (index, t) in roots[..count].iter().copied().enumerate() {
        let relative_t = (t - previous_t) / (1.0 - previous_t);
        current = [
            current[0] + relative_t * (end[0] - current[0]),
            current[1] + relative_t * (end[1] - current[1]),
        ];
        points[index + 1] = current;
        previous_t = t;
    }
    points[count + 1] = end;
    (points, count + 2)
}

fn outline(line: ImageLine, width: u32, height: u32) -> ([[f32; 2]; MAX_VERTICES], usize) {
    // PixelToShapeConverter constructs Line2D.Float before the double normalizer
    let normalized = |point: [i32; 2]| point.map(|n| (f64::from(n as f32) + 0.25).floor() + 0.25);
    let start = normalized(line.start);
    let end = normalized(line.end);
    normalized_outline(start, end, width, height, 0.5)
}

fn normalized_outline(
    start: [f64; 2],
    end: [f64; 2],
    width: u32,
    height: u32,
    half_width: f64,
) -> ([[f32; 2]; MAX_VERTICES], usize) {
    // Default miter limit10 expands the half-width clip beyond square-cap padding
    let margin = 10.0 * half_width;
    let clip = [
        (-0.001 + 0.25) - margin,
        (f64::from(height) + 0.001 + 0.25) + margin,
        (-0.001 + 0.25) - margin,
        (f64::from(width) + 0.001 + 0.25) + margin,
    ];
    let mut polygon = [[0.0; 2]; MAX_VERTICES];
    if outcode(start, clip) & outcode(end, clip) != 0 {
        return (polygon, 0);
    }
    let (points, count) = line_pieces(start, end, clip);
    let mut pieces = [([0.0; 2], [0.0; 2], [0.0; 2]); 5];
    let mut kept = 0;
    for pair in points[..count].windows(2) {
        let [a, b] = [pair[0], pair[1]];
        if outcode(a, clip) & outcode(b, clip) != 0 {
            continue;
        }
        let mut dx = b[0] - a[0];
        let dy = b[1] - a[1];
        if dx == 0.0 && dy == 0.0 {
            if count > 2 {
                continue;
            }
            dx = 1.0;
        }
        let length = (dx * dx + dy * dy).sqrt();
        let normal = [dy * half_width / length, -(dx * half_width) / length];
        pieces[kept] = (a, b, normal);
        kept += 1;
    }
    if kept == 0 {
        return (polygon, 0);
    }
    let mut used = 0;
    let mut emit = |point: [f64; 2]| {
        polygon[used] = point.map(|n| n as f32);
        used += 1;
    };
    // Artificial clip pieces suppress join decoration but retain both offset endpoints
    for (a, b, n) in &pieces[..kept] {
        emit([a[0] + n[0], a[1] + n[1]]);
        emit([b[0] + n[0], b[1] + n[1]]);
    }
    let (_, last, n) = pieces[kept - 1];
    if outcode(last, clip) == 0 {
        emit([last[0] - n[1] + n[0], last[1] + n[0] + n[1]]);
        emit([last[0] - n[1] - n[0], last[1] + n[0] - n[1]]);
    }
    for (a, b, n) in pieces[..kept].iter().rev() {
        emit([b[0] - n[0], b[1] - n[1]]);
        emit([a[0] - n[0], a[1] - n[1]]);
    }
    let (first, _, n) = pieces[0];
    if outcode(first, clip) == 0 {
        emit([first[0] + n[1] - n[0], first[1] - n[0] - n[1]]);
        emit([first[0] + n[1] + n[0], first[1] - n[0] + n[1]]);
    }
    (polygon, used)
}

#[derive(Clone, Copy)]
struct Edge {
    first: i32,
    end: i32,
    x: i32,
    error: i32,
    bump_x: i32,
    bump_error: i32,
    winding: i32,
}

fn native_integer(n: f64) -> i32 {
    if n >= f64::from(i32::MIN) && n < f64::from(i32::MAX) + 1.0 {
        n as i32
    } else {
        i32::MIN
    }
}

impl Edge {
    fn new(mut a: [f32; 2], mut b: [f32; 2], width: u32, height: u32) -> Option<Self> {
        if a[1].max(b[1]) <= 0.0
            || a[1].min(b[1]) >= height as f32
            || a[0].min(b[0]) >= width as f32
        {
            return None;
        }
        if a[0].max(b[0]) <= 0.0 {
            // Left-only edges retain winding without an offscreen horizontal excursion
            a[0] = 0.0;
            b[0] = 0.0;
        }
        let winding = if a[1] > b[1] {
            std::mem::swap(&mut a, &mut b);
            -1
        } else {
            1
        };
        let original_first = native_integer(f64::from((a[1] - 0.5).ceil()));
        let last = native_integer(f64::from((b[1] - 0.5).ceil()));
        let first = original_first.max(0);
        let end = last.min(height as i32);
        if first >= end {
            return None;
        }
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let slope = dx / dy;
        let y_bump = (original_first as f32 + 0.5) - a[1];
        let crossing = a[0] + (y_bump * dx) / dy;
        let x = native_integer(f64::from((crossing - 0.5).ceil()));
        let error = native_integer(f64::from(crossing - (x as f32 - 0.5)) * ERROR_SCALE);
        let floor_slope = slope.floor();
        let bump_x = native_integer(f64::from(floor_slope));
        let bump_error = native_integer((f64::from(slope) - f64::from(floor_slope)) * ERROR_SCALE);
        let mut edge = Self {
            first,
            end,
            x,
            error,
            bump_x,
            bump_error,
            winding,
        };
        edge.advance(i64::from(first) - i64::from(original_first));
        Some(edge)
    }

    fn advance(&mut self, rows: i64) {
        if rows == 0 {
            return;
        }
        if rows == 1 {
            let sum = self.error.wrapping_add(self.bump_error) as u32;
            self.x = self
                .x
                .wrapping_add(self.bump_x)
                .wrapping_add((sum >> 31) as i32);
            self.error = (sum & ERROR_MASK as u32) as i32;
        } else {
            let error = i128::from(self.error) + i128::from(rows) * i128::from(self.bump_error);
            self.x = self
                .x
                .wrapping_add((rows as i32).wrapping_mul(self.bump_x))
                .wrapping_add((error >> 31) as i32);
            self.error = (error & i128::from(ERROR_MASK)) as i32;
        }
    }
}

pub(super) fn raster_alpha_line(rgb: &mut [u8], width: u32, height: u32, line: ImageLine) {
    let (polygon, count) = outline(line, width, height);
    fill_outline(rgb, width, height, line.argb, polygon, count);
}

pub(super) fn raster_preview_alpha_line(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    line: ImageLine,
    translation: [i32; 2],
    scale: f64,
) {
    let normalize = |point: [i32; 2]| {
        std::array::from_fn(|axis| {
            let input = f64::from(point[axis] as f32);
            let scaled = input * scale;
            let transformed = scaled + f64::from(translation[axis]);
            (transformed + 0.25).floor() + 0.25
        })
    };
    // Retain installed userSpaceLineWidth arithmetic before uniform Stroker scaling
    let first_square = scale * scale + 0.0;
    let second_square = 0.0 + scale * scale;
    let mixed = 2.0 * (scale * 0.0 + 0.0 * scale);
    let difference = first_square - second_square;
    let discriminant = (mixed * mixed + difference * difference).sqrt();
    let user_scale = ((first_square + second_square + discriminant) / 2.0).sqrt();
    let user_width = 1.0 / user_scale;
    let device_width = user_width * (scale * scale + 0.0).sqrt();
    let (polygon, count) = normalized_outline(
        normalize(line.start),
        normalize(line.end),
        width,
        height,
        device_width / 2.0,
    );
    fill_outline(rgb, width, height, line.argb, polygon, count);
}

fn fill_outline(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    argb: u32,
    polygon: [[f32; 2]; MAX_VERTICES],
    count: usize,
) {
    if count == 0 {
        return;
    }
    let mut edges = [None; MAX_VERTICES];
    for index in 0..count {
        edges[index] = Edge::new(polygon[index], polygon[(index + 1) % count], width, height);
    }
    let first = edges.iter().flatten().map(|e| e.first).min().unwrap_or(0);
    let end = edges.iter().flatten().map(|e| e.end).max().unwrap_or(0);
    // At most image-height scanlines, with fixed stack arrays and clipped pixel spans
    for y in first..end {
        let mut crossings = [(0_i32, 0_i32); MAX_VERTICES];
        let mut active = 0;
        for edge in edges.iter_mut().flatten() {
            if y >= edge.first && y < edge.end {
                crossings[active] = (edge.x, edge.winding);
                active += 1;
                edge.advance(1);
            }
        }
        // Equal-X ordering affects only zero-width spans; unstable sorting avoids heap scratch
        crossings[..active].sort_unstable_by_key(|pair| pair.0);
        let mut winding = 0;
        let mut left = 0;
        let mut fill = |left: i32, right: i32| {
            for x in left.clamp(0, width as i32)..right.clamp(0, width as i32) {
                let index = (y as usize * width as usize + x as usize) * 3;
                blend_pixel(&mut rgb[index..index + 3], argb);
            }
        };
        for (x, direction) in &crossings[..active] {
            if winding != 0 {
                fill(left, *x);
            }
            winding += direction;
            left = *x;
        }
        if winding != 0 {
            fill(left, width as i32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_alpha_diagonal_is_clipped_before_float_outline_conversion() {
        let mut rgb = [0; 27];
        raster_alpha_line(
            &mut rgb,
            3,
            3,
            ImageLine {
                start: [i32::MIN, i32::MIN],
                end: [i32::MAX, i32::MAX],
                argb: 0x8000ff00,
            },
        );
        for y in 0..3 {
            for x in 0..3 {
                let start = (y * 3 + x) * 3;
                let expected = if x == y { [0, 128, 0] } else { [0; 3] };
                assert_eq!(rgb[start..start + 3], expected);
            }
        }
    }
}
