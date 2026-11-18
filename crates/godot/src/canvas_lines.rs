// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use godot::builtin::{Array, Color, PackedColorArray, PackedVector2Array, Rect2, Vector2};

#[godot_api(secondary)]
impl BgrSimulation {
    /// Prepare ordered opaque lines; translucent lines retain the canvas fallback
    #[func]
    fn prepare_crisp_batch(
        segments: PackedFloat64Array,
        colors: PackedInt32Array,
        alphas: PackedByteArray,
        zoom: f64,
        offset: Vector2,
    ) -> VarDictionary {
        let result = opaque_lines(segments.as_slice(), colors.as_slice(), alphas.as_slice(), zoom, offset);
        let (ok, eligible, error, points, colors) = match result {
            Ok(Some((points, colors))) => (true, true, "", points, colors),
            Ok(None) => (true, false, "", PackedVector2Array::new(), PackedColorArray::new()),
            Err(error) => (false, false, error, PackedVector2Array::new(), PackedColorArray::new()),
        };
        let mut output = VarDictionary::new();
        output.set("ok", ok);
        output.set("eligible", eligible);
        output.set("error", error);
        output.set("points", &points);
        output.set("colors", &colors);
        output
    }

    /// Prepare Smooth draw operations without changing their order or engine primitives
    #[func]
    fn prepare_smooth_batch(
        segments: PackedFloat64Array,
        colors: PackedInt32Array,
        alphas: PackedByteArray,
        zoom: f64,
        offset: Vector2,
        can_coalesce: bool,
    ) -> VarDictionary {
        let prepared = smooth_operations(segments.as_slice(), colors.as_slice(), alphas.as_slice(), zoom, offset, can_coalesce);
        smooth_packet(prepared)
    }

    /// Require an identity, clipped, material-free Canvas covering the supplied viewport
    #[func]
    fn prepare_smooth_viewport_batch(
        segments: PackedFloat64Array,
        colors: PackedInt32Array,
        alphas: PackedByteArray,
        zoom: f64,
        offset: Vector2,
        can_coalesce: bool,
        viewport_size: Vector2,
    ) -> VarDictionary {
        smooth_packet(smooth_operations_in_view(
            segments.as_slice(), colors.as_slice(), alphas.as_slice(), zoom, offset,
            can_coalesce, Some(viewport_size),
        ))
    }

    /// Pack the existing ordered Smooth operations for the identity GLES3 canvas
    #[func]
    fn prepare_smooth_aa_viewport_batch(
        segments: PackedFloat64Array,
        colors: PackedInt32Array,
        alphas: PackedByteArray,
        zoom: f64,
        offset: Vector2,
        can_coalesce: bool,
        viewport_size: Vector2,
    ) -> VarDictionary {
        let prepared = smooth_operations_in_view(
            segments.as_slice(), colors.as_slice(), alphas.as_slice(), zoom, offset,
            can_coalesce, Some(viewport_size),
        );
        if let Some(operations) = prepared.as_ref() {
            if let Some(packet) = aa_packet(operations) {
                return packet;
            }
        }
        smooth_packet(prepared)
    }
}

const AA_LINES_PER_CHUNK: usize = 2048;
const MAX_AA_PACKED_BYTES: usize = 256 * 1024 * 1024;
const AA_BYTES_PER_LINE: usize = 36 * (8 + 16) + 54 * 4;

// Godot 4.7.2 GLES3 TYPE_PRIMITIVE truncates colors to half precision
// Colors in this path are finite unit values with identity modulation
fn aa_half(value: f32) -> f32 {
    let bits = value.to_bits();
    if bits & 0x7f80_0000 <= 0x3800_0000 {
        0.0
    } else {
        f32::from_bits(bits & !0x1fff)
    }
}

struct AaChunk {
    points: Vec<Vector2>,
    colors: Vec<Color>,
    indices: Vec<i32>,
}

impl AaChunk {
    fn new(lines: usize) -> Option<Self> {
        let mut chunk = Self { points: Vec::new(), colors: Vec::new(), indices: Vec::new() };
        chunk.points.try_reserve_exact(lines.checked_mul(36)?).ok()?;
        chunk.colors.try_reserve_exact(lines.checked_mul(36)?).ok()?;
        chunk.indices.try_reserve_exact(lines.checked_mul(54)?).ok()?;
        Some(chunk)
    }

    fn quad(&mut self, vertices: [Vector2; 4], colors: [Color; 4]) {
        let base = self.points.len() as i32;
        self.points.extend(vertices);
        self.colors.extend(colors);
        self.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    // Adapted from Godot renderer_canvas_cull.cpp::canvas_item_add_line (MIT)
    // Copyright 2014-present Godot contributors; see GODOT-LICENSE.txt
    fn line(&mut self, from: Vector2, to: Vector2, color: Color) {
        let diff = from - to;
        let direction = Vector2::new(diff.y, -diff.x).normalized();
        let tangent = direction * 0.5 * 0.5;
        let (bl, br, el, er) = (from + tangent, from - tangent, to + tangent, to - tangent);
        let border = direction * (1.25 * 0.5);
        let border2 = diff.normalized() * (1.25 * 0.5);
        let color = Color::from_rgba(aa_half(color.r), aa_half(color.g), aa_half(color.b), aa_half(color.a));
        let transparent = Color::from_rgba(color.r, color.g, color.b, 0.0);
        let edge = [color, transparent, transparent, color];
        let corner = [color, transparent, transparent, transparent];
        self.quad([bl, br, er, el], [color; 4]);
        self.quad([bl, bl + border, el + border, el], edge);
        self.quad([br, br - border, er - border, er], edge);
        self.quad([bl, bl + border2, br + border2, br], edge);
        self.quad([el, el - border2, er - border2, er], edge);
        self.quad([bl, bl + border2, bl + border + border2, bl + border], corner);
        self.quad([br, br + border2, br - border + border2, br - border], corner);
        self.quad([el, el - border2, el + border - border2, el + border], corner);
        self.quad([er, er - border2, er - border - border2, er - border], corner);
    }
}

fn aa_size(operations: &[SmoothOperation]) -> Option<usize> {
    let mut count = 0usize;
    for operation in operations {
        if let SmoothOperation::Lines(points, colors) = operation {
            if colors.len().checked_mul(2)? != points.len()
                || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()
                    || p.x.abs() > MAX_CULL_COORDINATE || p.y.abs() > MAX_CULL_COORDINATE)
                || points.chunks_exact(2).any(|p| {
                    let length_squared = (p[0] - p[1]).length_squared();
                    !length_squared.is_finite() || length_squared <= 0.0
                })
                || colors.iter().any(|c| [c.r, c.g, c.b, c.a].iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)))
            {
                return None;
            }
            count = count.checked_add(colors.len())?;
        }
    }
    aa_bytes(count)
}

fn aa_bytes(lines: usize) -> Option<usize> {
    let bytes = lines.checked_mul(AA_BYTES_PER_LINE)?;
    (bytes <= MAX_AA_PACKED_BYTES).then_some(bytes)
}

fn aa_packet(operations: &[SmoothOperation]) -> Option<VarDictionary> {
    let packed_bytes = aa_size(operations)?;
    let mut commands = Vec::new();
    let mut points = Array::<PackedVector2Array>::new();
    let mut colors = Array::<PackedColorArray>::new();
    let mut indices = Array::<PackedInt32Array>::new();
    let mut point_rects = Array::<Rect2>::new();
    let mut point_colors = Vec::new();
    for operation in operations {
        match operation {
            SmoothOperation::Point(rect, color) => {
                commands.push(-(point_rects.len() as i32) - 1);
                point_rects.push(*rect);
                point_colors.push(*color);
            }
            SmoothOperation::Lines(line_points, line_colors) => {
                for start in (0..line_colors.len()).step_by(AA_LINES_PER_CHUNK) {
                    let end = (start + AA_LINES_PER_CHUNK).min(line_colors.len());
                    let mut chunk = AaChunk::new(end - start)?;
                    for index in start..end {
                        chunk.line(line_points[index * 2], line_points[index * 2 + 1], line_colors[index]);
                    }
                    commands.push(points.len() as i32);
                    points.push(&PackedVector2Array::from(chunk.points.as_slice()));
                    colors.push(&PackedColorArray::from(chunk.colors.as_slice()));
                    indices.push(&PackedInt32Array::from(chunk.indices.as_slice()));
                }
            }
        }
    }
    let mut output = VarDictionary::new();
    output.set("eligible", true);
    output.set("aa", true);
    output.set("aa_packed_bytes", packed_bytes as i64);
    output.set("commands", &PackedInt32Array::from(commands.as_slice()));
    output.set("aa_points", &points);
    output.set("aa_colors", &colors);
    output.set("aa_indices", &indices);
    output.set("point_rects", &point_rects);
    output.set("point_colors", &PackedColorArray::from(point_colors.as_slice()));
    Some(output)
}

fn smooth_packet(prepared: Option<Vec<SmoothOperation>>) -> VarDictionary {
    let mut output = VarDictionary::new();
    let mut commands = Vec::new();
    let mut line_points = Array::<PackedVector2Array>::new();
    let mut line_colors = Array::<PackedColorArray>::new();
    let mut point_rects = Array::<Rect2>::new();
    let mut point_colors = Vec::new();
    output.set("eligible", prepared.is_some());
    if let Some(operations) = prepared {
        for operation in operations {
            match operation {
                SmoothOperation::Lines(points, colors) => {
                    commands.push(line_points.len() as i32);
                    line_points.push(&PackedVector2Array::from(points.as_slice()));
                    line_colors.push(&PackedColorArray::from(colors.as_slice()));
                }
                SmoothOperation::Point(rect, color) => {
                    commands.push(-(point_rects.len() as i32) - 1);
                    point_rects.push(rect);
                    point_colors.push(color);
                }
            }
        }
    }
    output.set("commands", &PackedInt32Array::from(commands.as_slice()));
    output.set("line_points", &line_points);
    output.set("line_colors", &line_colors);
    output.set("point_rects", &point_rects);
    output.set("point_colors", &PackedColorArray::from(point_colors.as_slice()));
    output
}

fn opaque_lines(
    segments: &[f64],
    colors: &[i32],
    alphas: &[u8],
    zoom: f64,
    offset: Vector2,
) -> Result<Option<(PackedVector2Array, PackedColorArray)>, &'static str> {
    validate_geometry(segments, colors, alphas, zoom, offset)?;
    if alphas.iter().any(|alpha| *alpha != 255) {
        return Ok(None);
    }
    let point_count = colors.len().checked_mul(2).ok_or("Crisp batch point count exceeds capacity")?;
    let mut points = PackedVector2Array::new();
    let mut packed_colors = PackedColorArray::new();
    points.resize(point_count);
    packed_colors.resize(colors.len());
    if points.len() != point_count || packed_colors.len() != colors.len() {
        return Err("Cannot allocate the crisp batch");
    }
    let point_values = points.as_mut_slice();
    let color_values = packed_colors.as_mut_slice();
    let offset_x = f64::from(offset.x);
    let offset_y = f64::from(offset.y);
    for (index, (coordinates, rgb)) in segments.chunks_exact(4).zip(colors).enumerate() {
        // Match GDScript's f64 affine calculation before Vector2 rounds its components
        let from = Vector2::new((coordinates[0] * zoom + offset_x) as _, (coordinates[1] * zoom + offset_y) as _);
        let to = Vector2::new((coordinates[2] * zoom + offset_x) as _, (coordinates[3] * zoom + offset_y) as _);
        if !from.x.is_finite() || !from.y.is_finite() || !to.x.is_finite() || !to.y.is_finite() {
            return Err("Crisp batch transformed geometry exceeds finite Vector2 range");
        }
        let pair = &mut point_values[index * 2..index * 2 + 2];
        if from == to {
            pair[0] = from - Vector2::new(0.5, 0.0);
            pair[1] = from + Vector2::new(0.5, 0.0);
        } else {
            pair[0] = from;
            pair[1] = to;
        }
        color_values[index] = Color::from_rgb(
            (f64::from((rgb >> 16) & 255) / 255.0) as f32,
            (f64::from((rgb >> 8) & 255) / 255.0) as f32,
            (f64::from(rgb & 255) / 255.0) as f32,
        );
    }
    Ok(Some((points, packed_colors)))
}

fn validate_geometry(
    segments: &[f64],
    colors: &[i32],
    alphas: &[u8],
    zoom: f64,
    offset: Vector2,
) -> Result<(), &'static str> {
    if colors.len().checked_mul(4) != Some(segments.len())
        || (!alphas.is_empty() && alphas.len() != colors.len())
    {
        return Err("Crisp batch arrays have inconsistent lengths");
    }
    if !zoom.is_finite() || zoom <= 0.0 || !offset.x.is_finite() || !offset.y.is_finite() {
        return Err("Crisp batch requires positive finite zoom and finite offsets");
    }
    if segments.iter().any(|value| !value.is_finite()) {
        return Err("Crisp batch contains nonfinite geometry");
    }
    Ok(())
}

const MAX_SMOOTH_SEGMENTS: usize = 1_048_576;

#[derive(Debug, PartialEq)]
enum SmoothOperation {
    Lines(Vec<Vector2>, Vec<Color>),
    Point(Rect2, Color),
}

fn flush_lines(operations: &mut Vec<SmoothOperation>, points: &mut Vec<Vector2>, colors: &mut Vec<Color>) {
    if !colors.is_empty() {
        operations.push(SmoothOperation::Lines(std::mem::take(points), std::mem::take(colors)));
    }
}

fn flush_point(operations: &mut Vec<SmoothOperation>, pending: &mut Option<(Vector2, Color)>) {
    if let Some((point, color)) = pending.take() {
        operations.push(SmoothOperation::Point(Rect2::new(point - Vector2::new(0.5, 0.5), Vector2::ONE), color));
    }
}

fn smooth_operations(
    segments: &[f64], colors: &[i32], alphas: &[u8], zoom: f64, offset: Vector2, can_coalesce: bool,
) -> Option<Vec<SmoothOperation>> {
    smooth_operations_in_view(segments, colors, alphas, zoom, offset, can_coalesce, None)
}

// Keep extreme legacy coordinates unculled rather than depending on large f32 geometry arithmetic
const MAX_CULL_COORDINATE: f32 = 1_048_576.0;

fn outside_smooth_viewport(from: Vector2, to: Vector2, viewport: Vector2) -> bool {
    if [from.x, from.y, to.x, to.y].iter().any(|v| !v.is_finite() || v.abs() > MAX_CULL_COORDINATE) {
        return false;
    }
    let (minimum, maximum, padding) = if from == to {
        let position = from - Vector2::new(0.5, 0.5);
        (position, position + Vector2::ONE, 1.0_f64)
    } else {
        // Half the 1px stroke plus a conservative 1px antialiasing margin
        (Vector2::new(from.x.min(to.x), from.y.min(to.y)),
            Vector2::new(from.x.max(to.x), from.y.max(to.y)), 1.5_f64)
    };
    f64::from(maximum.x) + padding < 0.0
        || f64::from(maximum.y) + padding < 0.0
        || f64::from(minimum.x) - padding > f64::from(viewport.x)
        || f64::from(minimum.y) - padding > f64::from(viewport.y)
}

fn smooth_operations_in_view(
    segments: &[f64], colors: &[i32], alphas: &[u8], zoom: f64, offset: Vector2,
    can_coalesce: bool, viewport: Option<Vector2>,
) -> Option<Vec<SmoothOperation>> {
    if viewport.is_some_and(|v| !can_coalesce || !v.x.is_finite() || !v.y.is_finite() || v.x <= 0.0 || v.y <= 0.0) {
        return None;
    }
    if colors.len() > MAX_SMOOTH_SEGMENTS || validate_geometry(segments, colors, alphas, zoom, offset).is_err() {
        return None;
    }
    let mut operations = Vec::new();
    let mut points = Vec::new();
    let mut line_colors = Vec::new();
    let mut pending = None;
    let offset_x = f64::from(offset.x);
    let offset_y = f64::from(offset.y);
    for (index, (coordinates, rgb)) in segments.chunks_exact(4).zip(colors).enumerate() {
        let from = Vector2::new((coordinates[0] * zoom + offset_x) as _, (coordinates[1] * zoom + offset_y) as _);
        let to = Vector2::new((coordinates[2] * zoom + offset_x) as _, (coordinates[3] * zoom + offset_y) as _);
        if !from.x.is_finite() || !from.y.is_finite() || !to.x.is_finite() || !to.y.is_finite() {
            return None;
        }
        let alpha = alphas.get(index).copied().unwrap_or(255);
        let color = Color::from_rgba(
            (f64::from((rgb >> 16) & 255) / 255.0) as f32,
            (f64::from((rgb >> 8) & 255) / 255.0) as f32,
            (f64::from(rgb & 255) / 255.0) as f32,
            (f64::from(alpha) / 255.0) as f32,
        );
        let invisible = viewport.is_some_and(|size| outside_smooth_viewport(from, to, size));
        if from == to {
            flush_lines(&mut operations, &mut points, &mut line_colors);
            if invisible {
                flush_point(&mut operations, &mut pending);
                continue;
            }
            if can_coalesce && color.a == 1.0 {
                if pending.as_ref().is_some_and(|(point, _)| *point != from) {
                    flush_point(&mut operations, &mut pending);
                }
                pending = Some((from, color));
            } else {
                flush_point(&mut operations, &mut pending);
                pending = Some((from, color));
                flush_point(&mut operations, &mut pending);
            }
        } else {
            flush_point(&mut operations, &mut pending);
            if invisible {
                continue;
            }
            points.extend_from_slice(&[from, to]);
            line_colors.push(color);
        }
    }
    flush_point(&mut operations, &mut pending);
    flush_lines(&mut operations, &mut points, &mut line_colors);
    Some(operations)
}

#[cfg(test)]
mod smooth_tests {
    use super::*;

    fn point(x: f32, color: Color) -> SmoothOperation {
        SmoothOperation::Point(Rect2::new(Vector2::new(x - 0.5, -0.5), Vector2::ONE), color)
    }

    #[test]
    fn smooth_preserves_interleaved_operations_and_last_opaque_point_color() {
        let segments = [
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            1.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            3.0, 0.0, 3.0, 0.0,
        ];
        let colors = [0xff0000, 0x00ff00, 0xffffff, 0xff0000, 0xff0000, 0x0000ff, 0x00ff00];
        let alphas = [255, 255, 255, 128, 255, 255, 255];
        let actual = smooth_operations(&segments, &colors, &alphas, 1.0, Vector2::ZERO, true).unwrap();
        assert_eq!(actual, vec![
            point(0.0, Color::GREEN),
            SmoothOperation::Lines(vec![Vector2::new(1.0, 0.0), Vector2::new(2.0, 0.0)], vec![Color::WHITE]),
            point(0.0, Color::from_rgba(1.0, 0.0, 0.0, (128.0_f64 / 255.0) as f32)),
            point(0.0, Color::BLUE), point(3.0, Color::GREEN),
        ]);
    }

    #[test]
    fn smooth_keeps_transparent_and_modulated_duplicate_points() {
        let segments = [0.0; 12];
        let colors = [0xff0000, 0x00ff00, 0x0000ff];
        let actual = smooth_operations(&segments, &colors, &[0, 255, 255], 1.0, Vector2::ZERO, false).unwrap();
        assert_eq!(actual, vec![point(0.0, Color::from_rgba(1.0, 0.0, 0.0, 0.0)),
                                point(0.0, Color::GREEN), point(0.0, Color::BLUE)]);
    }

    #[test]
    fn smooth_rounds_after_f64_affine_and_then_tests_point_equality() {
        let actual = smooth_operations(&[16_777_217.0, 0.0, 16_777_217.0, 0.0], &[0xffffff], &[],
                                       1.0, Vector2::new(1.0, 0.0), true).unwrap();
        let SmoothOperation::Point(rect, _) = &actual[0] else { panic!("Expected a point rectangle") };
        assert_eq!(rect.position.x.to_bits(), 16_777_218.0_f32.to_bits());
        let actual = smooth_operations(&[16_777_216.0, 0.0, 16_777_217.0, 0.0], &[0xffffff], &[],
                                       1.0, Vector2::ZERO, true).unwrap();
        assert!(matches!(actual[0], SmoothOperation::Point(_, _)));
    }

    #[test]
    fn smooth_refuses_new_domains_without_partial_operations() {
        assert_eq!(smooth_operations(&[], &[], &[], 1.0, Vector2::ZERO, true), Some(vec![]));
        for (segments, colors, alphas, zoom) in [
            (vec![0.0; 3], vec![0], vec![], 1.0),
            (vec![0.0; 4], vec![0], vec![0, 0], 1.0),
            (vec![0.0; 4], vec![0], vec![], 0.0),
            (vec![f64::NAN; 4], vec![0], vec![], 1.0),
            (vec![f64::MAX; 4], vec![0], vec![], 1.0),
        ] {
            assert!(smooth_operations(&segments, &colors, &alphas, zoom, Vector2::ZERO, true).is_none());
        }
        assert!(smooth_operations(&[], &vec![0; MAX_SMOOTH_SEGMENTS + 1], &[], 1.0, Vector2::ZERO, true).is_none());
    }
}

#[cfg(test)]
mod viewport_tests {
    use super::*;

    #[test]
    fn viewport_keeps_crossings_and_padding_contacts() {
        let size = Vector2::new(100.0, 100.0);
        assert!(!outside_smooth_viewport(Vector2::new(-20.0, 50.0), Vector2::new(120.0, 50.0), size));
        for x in [-1.5, 101.5] {
            assert!(!outside_smooth_viewport(Vector2::new(x, 20.0), Vector2::new(x, 80.0), size));
        }
        for x in [-1.6, 101.6] {
            assert!(outside_smooth_viewport(Vector2::new(x, 20.0), Vector2::new(x, 80.0), size));
        }
        assert!(!outside_smooth_viewport(Vector2::new(-1.5, 50.0), Vector2::new(-1.5, 50.0), size));
        assert!(outside_smooth_viewport(Vector2::new(-1.6, 50.0), Vector2::new(-1.6, 50.0), size));
    }

    #[test]
    fn viewport_preserves_visible_order_and_invisible_type_barriers() {
        let segments = [50.0,50.0,50.0,50.0, -10.0,10.0,-10.0,20.0,
            50.0,50.0,50.0,50.0, 10.0,10.0,20.0,20.0, -20.0,15.0,-20.0,25.0,
            30.0,30.0,40.0,40.0, -10.0,-10.0,-10.0,-10.0, 45.0,45.0,45.0,45.0];
        let actual = smooth_operations_in_view(&segments,
            &[0xff0000,0,0x0000ff,0x00ff00,0,0xffffff,0,0xff0000],
            &[255,255,255,255,255,255,255,128], 1.0, Vector2::ZERO, true,
            Some(Vector2::new(100.0,100.0))).unwrap();
        assert_eq!(actual, vec![
            SmoothOperation::Point(Rect2::new(Vector2::new(49.5,49.5),Vector2::ONE),Color::RED),
            SmoothOperation::Point(Rect2::new(Vector2::new(49.5,49.5),Vector2::ONE),Color::BLUE),
            SmoothOperation::Lines(vec![Vector2::new(10.0,10.0),Vector2::new(20.0,20.0),
                Vector2::new(30.0,30.0),Vector2::new(40.0,40.0)],vec![Color::GREEN,Color::WHITE]),
            SmoothOperation::Point(Rect2::new(Vector2::new(44.5,44.5),Vector2::ONE),
                Color::from_rgba(1.0,0.0,0.0,(128.0_f64/255.0)as f32)),
        ]);
    }

    #[test]
    fn viewport_retains_extreme_geometry_and_refuses_invalid_sizes() {
        let segments = [16_777_216.0,0.0,16_777_217.0,0.0];
        assert_eq!(smooth_operations_in_view(&segments,&[0xffffff],&[],1.0,Vector2::ZERO,true,
            Some(Vector2::new(100.0,100.0))),smooth_operations(&segments,&[0xffffff],&[],1.0,Vector2::ZERO,true));
        for size in [Vector2::ZERO,Vector2::new(-1.0,100.0),Vector2::new(f32::NAN,100.0)] {
            assert!(smooth_operations_in_view(&[],&[],&[],1.0,Vector2::ZERO,true,Some(size)).is_none());
        }
        assert!(smooth_operations_in_view(&[f64::MAX;4],&[0],&[],1.0,Vector2::ZERO,true,
            Some(Vector2::new(100.0,100.0))).is_none());
    }
    #[test]
    fn aa_half_matches_gles_truncation_and_flush() {
        for value in [0.0_f32, 1.0, 0.5, 128.0 / 255.0, 1.0 / 255.0] {
            assert_eq!(aa_half(value).to_bits(), value.to_bits() & !0x1fff);
        }
        assert_eq!(aa_half(f32::from_bits(0x3800_0000)), 0.0);
        assert_eq!(aa_half(f32::from_bits(0x3800_0001)), 0.0);
    }

    #[test]
    fn aa_geometry_preserves_nine_quad_diagonals_and_feather_colors() {
        let mut chunk = AaChunk::new(1).unwrap();
        chunk.line(Vector2::new(3.0, 5.0), Vector2::new(4.0, 5.0), Color::WHITE);
        assert_eq!((chunk.points.len(), chunk.colors.len(), chunk.indices.len()), (36, 36, 54));
        assert_eq!(&chunk.points[..4], &[Vector2::new(3.0, 5.25), Vector2::new(3.0, 4.75), Vector2::new(4.0, 4.75), Vector2::new(4.0, 5.25)]);
        for (quad, indices) in chunk.indices.chunks_exact(6).enumerate() {
            let base = quad as i32 * 4;
            assert_eq!(indices, &[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        assert_eq!(&chunk.colors[..4], &[Color::WHITE; 4]);
        assert_eq!(chunk.colors[4].a, 1.0);
        assert_eq!(chunk.colors[5].a, 0.0);
        assert_eq!(chunk.colors[6].a, 0.0);
        assert_eq!(chunk.colors[7].a, 1.0);
    }

    #[test]
    fn aa_checked_byte_bound_and_chunk_capacity_refuse_overflow() {
        let maximum = MAX_AA_PACKED_BYTES / AA_BYTES_PER_LINE;
        assert_eq!(aa_bytes(maximum), Some(maximum * AA_BYTES_PER_LINE));
        assert_eq!(aa_bytes(maximum + 1), None);
        assert_eq!(aa_bytes(usize::MAX), None);
        assert!(AaChunk::new(usize::MAX).is_none());
        let chunk = AaChunk::new(AA_LINES_PER_CHUNK).unwrap();
        assert!(chunk.points.capacity() >= AA_LINES_PER_CHUNK * 36);
    }

    #[test]
    fn aa_domain_refusal_preserves_original_operations() {
        let cases = [
            (vec![Vector2::ZERO], vec![Color::WHITE]),
            (vec![Vector2::ZERO, Vector2::new(2_000_000.0, 0.0)], vec![Color::WHITE]),
            (vec![Vector2::ZERO, Vector2::new(f32::NAN, 1.0)], vec![Color::WHITE]),
            (vec![Vector2::ZERO, Vector2::new(1.0e-30, 0.0)], vec![Color::WHITE]),
            (vec![Vector2::ZERO, Vector2::ONE], vec![Color::from_rgba(1.0, 0.0, 0.0, -0.1)]),
        ];
        for (points, colors) in cases {
            let operations = vec![SmoothOperation::Lines(points, colors)];
            let before = format!("{operations:?}");
            assert_eq!(aa_size(&operations), None);
            assert_eq!(format!("{operations:?}"), before);
        }
    }

    #[test]
    fn aa_points_and_line_color_order_remain_separate() {
        let operations = vec![SmoothOperation::Point(Rect2::new(Vector2::new(1.0, 0.0), Vector2::ONE), Color::BLUE), SmoothOperation::Lines(
            vec![Vector2::ZERO, Vector2::ONE, Vector2::ONE, Vector2::new(2.0, 3.0)],
            vec![Color::RED, Color::GREEN]), SmoothOperation::Point(Rect2::new(Vector2::new(2.0, 0.0), Vector2::ONE), Color::BLUE)];
        assert_eq!(aa_size(&operations), Some(AA_BYTES_PER_LINE * 2));
        let mut chunk = AaChunk::new(2).unwrap();
        chunk.line(Vector2::ZERO, Vector2::ONE, Color::RED);
        chunk.line(Vector2::ONE, Vector2::new(2.0, 3.0), Color::GREEN);
        assert_eq!(chunk.colors[0], Color::RED);
        assert_eq!(chunk.colors[36], Color::GREEN);
        assert!(matches!(operations[0], SmoothOperation::Point(..)));
        assert!(matches!(operations[2], SmoothOperation::Point(..)));
    }

}
