// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Collision calculations from Organism.touchMove and ExLine2DDouble.getIntersection
//! Segment classification and clipping use independently written math, checked against actual AWT observations
//! No JDK implementation source or bytecode is translated here

use crate::java_math::{add, div, max, min, mul, sqrt, sub};
use crate::organism::Organism;
use crate::{IntRect, TOLERANCE};

#[derive(Clone, Copy, Debug, PartialEq)]
/// Reference restitution and per-frame translation/rotation caps, with rotation measured in radians
pub struct PhysicsParams {
    pub elasticity: f64,
    pub max_velocity: f64,
    pub max_rotation: f64,
}

impl Default for PhysicsParams {
    fn default() -> Self {
        Self {
            elasticity: 0.8,
            max_velocity: 5.0,
            max_rotation: std::f64::consts::PI / 16.0,
        }
    }
}

impl PhysicsParams {
    /// Validate settings without changing the primitive's raw arithmetic on controlled degenerate body inputs
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.elasticity.is_finite() || !(0.0..=1.0).contains(&self.elasticity) {
            return Err("Elasticity must be finite and between zero and one");
        }
        if !self.max_velocity.is_finite() || !self.max_rotation.is_finite() {
            return Err("Velocity and rotation limits must be finite");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Line {
    pub start: [f64; 2],
    pub end: [f64; 2],
}

impl Line {
    /// Source contact coordinates add integers before Line2D receives widened doubles
    pub(crate) fn from_offsets(start: [i32; 2], end: [i32; 2], center: [i32; 2]) -> Self {
        Self {
            start: std::array::from_fn(|axis| f64::from(start[axis].wrapping_add(center[axis]))),
            end: std::array::from_fn(|axis| f64::from(end[axis].wrapping_add(center[axis]))),
        }
    }
}

pub(crate) fn between(value: f64, lower: f64, upper: f64) -> f64 {
    max(min(upper, value), lower)
}

pub(crate) fn point_distance_squared(first: [f64; 2], second: [f64; 2]) -> f64 {
    let x = sub(first[0], second[0]);
    let y = sub(first[1], second[1]);
    add(mul(x, x), mul(y, y))
}

/// Retain the reference's signed determinant test rather than treating every nonzero determinant as an intersection
pub(crate) fn intersection(first: Line, second: Line) -> [f64; 2] {
    let determinant = add(
        mul(sub(second.start[0], second.end[0]), sub(first.end[1], first.start[1])),
        mul(sub(first.end[0], first.start[0]), sub(second.end[1], second.start[1])),
    );
    if determinant < TOLERANCE {
        return std::array::from_fn(|axis| {
            div(
                add(add(add(first.start[axis], first.end[axis]), second.start[axis]), second.end[axis]),
                4.0,
            )
        });
    }
    let fraction = div(
        add(
            mul(sub(second.start[0], first.start[0]), sub(first.end[1], first.start[1])),
            mul(sub(first.end[0], first.start[0]), sub(first.start[1], second.start[1])),
        ),
        determinant,
    );
    std::array::from_fn(|axis| {
        add(second.start[axis], mul(fraction, sub(second.end[axis], second.start[axis])))
    })
}

fn point_side(line: Line, point: [f64; 2]) -> i32 {
    let direction = [line.end[0] - line.start[0], line.end[1] - line.start[1]];
    let relative = [point[0] - line.start[0], point[1] - line.start[1]];
    let area = relative[0] * direction[1] - relative[1] * direction[0];
    let position = if area == 0.0 {
        let along = relative[0] * direction[0] + relative[1] * direction[1];
        if along > 0.0 {
            let beyond = (relative[0] - direction[0]) * direction[0]
                + (relative[1] - direction[1]) * direction[1];
            if beyond < 0.0 {
                0.0
            } else {
                beyond
            }
        } else {
            along
        }
    } else {
        area
    };
    if position < 0.0 {
        -1
    } else if position > 0.0 {
        1
    } else {
        0
    }
}

/// Signed-area separation includes endpoints, collinear projections, and AWT's degenerate point-line behavior
pub(crate) fn lines_intersect(first: Line, second: Line) -> bool {
    point_side(first, second.start) * point_side(first, second.end) <= 0
        && point_side(second, first.start) * point_side(second, first.end) <= 0
}

pub(crate) fn rectangles_intersect(first: IntRect, second: IntRect) -> bool {
    first.width > 0
        && first.height > 0
        && second.width > 0
        && second.height > 0
        && i64::from(first.x) < i64::from(second.x) + i64::from(second.width)
        && i64::from(first.x) + i64::from(first.width) > i64::from(second.x)
        && i64::from(first.y) < i64::from(second.y) + i64::from(second.height)
        && i64::from(first.y) + i64::from(first.height) > i64::from(second.y)
}

fn outside(point: [f64; 2], minimum: [f64; 2], maximum: [f64; 2]) -> u8 {
    let mut sides = 0;
    if point[0] < minimum[0] {
        sides |= 1;
    }
    if point[0] > maximum[0] {
        sides |= 2;
    }
    if point[1] < minimum[1] {
        sides |= 4;
    }
    if point[1] > maximum[1] {
        sides |= 8;
    }
    sides
}

/// Clip against positive-area closed rectangle bounds without normalizing negative dimensions
pub(crate) fn rectangle_intersects_line(bounds: IntRect, line: Line) -> bool {
    if bounds.width <= 0 || bounds.height <= 0 {
        return false;
    }
    let minimum = [f64::from(bounds.x), f64::from(bounds.y)];
    let maximum = [
        minimum[0] + f64::from(bounds.width),
        minimum[1] + f64::from(bounds.height),
    ];
    let fixed = line.end;
    let fixed_outside = outside(fixed, minimum, maximum);
    let mut clipped = line.start;
    if fixed_outside == 0 {
        return true;
    }
    loop {
        let sides = outside(clipped, minimum, maximum);
        if sides == 0 {
            return true;
        }
        if sides & fixed_outside != 0 {
            return false;
        }
        if sides & 3 != 0 {
            let edge = if sides & 2 != 0 {
                maximum[0]
            } else {
                minimum[0]
            };
            clipped[1] += (edge - clipped[0]) * (fixed[1] - clipped[1]) / (fixed[0] - clipped[0]);
            clipped[0] = edge;
        } else {
            let edge = if sides & 8 != 0 {
                maximum[1]
            } else {
                minimum[1]
            };
            clipped[0] += (edge - clipped[1]) * (fixed[0] - clipped[0]) / (fixed[1] - clipped[1]);
            clipped[1] = edge;
        }
    }
}

/// Apply the reference impulse to both bodies without moving centers or changing biological state
pub(crate) fn touch_move(
    actor: &mut Organism,
    peer: &mut Organism,
    point: [f64; 2],
    line: Line,
    this_organism: bool,
    params: &PhysicsParams,
) {
    let rapx = sub(point[0], actor.state.center[0]);
    let rapy = sub(point[1], actor.state.center[1]);
    let rbpx = sub(point[0], peer.state.center[0]);
    let rbpy = sub(point[1], peer.state.center[1]);
    let vapx = add(
        sub(actor.state.velocity[0], mul(actor.state.angular_velocity, rapy)),
        div(mul(f64::from(actor.state.has_grown), rapx), 10.0),
    );
    let vapy = add(
        add(actor.state.velocity[1], mul(actor.state.angular_velocity, rapx)),
        div(mul(f64::from(actor.state.has_grown), rapy), 10.0),
    );
    let vbpx = sub(peer.state.velocity[0], mul(peer.state.angular_velocity, rbpy));
    let vbpy = add(peer.state.velocity[1], mul(peer.state.angular_velocity, rbpx));
    let relative_x = sub(vapx, vbpx);
    let relative_y = sub(vapy, vbpy);
    let mut nx = sub(line.start[1], line.end[1]);
    let mut ny = sub(line.end[0], line.start[0]);
    let mut modulus = sqrt(add(mul(nx, nx), mul(ny, ny)));
    if modulus == 0.0 {
        modulus = 0.000000000000001;
    }
    nx = div(nx, modulus);
    ny = div(ny, modulus);
    let reference_center = if this_organism {
        peer.state.center
    } else {
        actor.state.center
    };
    let plus = point_distance_squared([add(point[0], nx), add(point[1], ny)], reference_center);
    let minus = point_distance_squared([sub(point[0], nx), sub(point[1], ny)], reference_center);
    if (this_organism && plus < minus) || (!this_organism && plus > minus) {
        nx = -nx;
        ny = -ny;
    }
    let actor_cross = sub(mul(rapx, ny), mul(rapy, nx));
    let peer_cross = sub(mul(rbpx, ny), mul(rbpy, nx));
    let impulse = div(
        mul(-add(1.0, params.elasticity), add(mul(relative_x, nx), mul(relative_y, ny))),
        add(
            add(
                add(div(1.0, actor.body.mass), div(1.0, peer.body.mass)),
                div(mul(actor_cross, actor_cross), actor.body.inertia),
            ),
            div(mul(peer_cross, peer_cross), peer.body.inertia),
        ),
    );
    actor.state.velocity[0] = between(
        add(actor.state.velocity[0], div(mul(impulse, nx), actor.body.mass)),
        -params.max_velocity,
        params.max_velocity,
    );
    actor.state.velocity[1] = between(
        add(actor.state.velocity[1], div(mul(impulse, ny), actor.body.mass)),
        -params.max_velocity,
        params.max_velocity,
    );
    peer.state.velocity[0] = between(
        sub(peer.state.velocity[0], div(mul(impulse, nx), peer.body.mass)),
        -params.max_velocity,
        params.max_velocity,
    );
    peer.state.velocity[1] = between(
        sub(peer.state.velocity[1], div(mul(impulse, ny), peer.body.mass)),
        -params.max_velocity,
        params.max_velocity,
    );
    actor.state.angular_velocity = between(
        add(actor.state.angular_velocity, div(mul(impulse, actor_cross), actor.body.inertia)),
        -params.max_rotation,
        params.max_rotation,
    );
    peer.state.angular_velocity = between(
        // The source uses ny twice here, although the impulse denominator uses nx in the second term
        sub(
            peer.state.angular_velocity,
            div(mul(impulse, sub(mul(rbpx, ny), mul(rbpy, ny))), peer.body.inertia),
        ),
        -params.max_rotation,
        params.max_rotation,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gene, Genome, GenomeProperties, OrganismState};
    use std::collections::BTreeMap;
    use std::{cell::RefCell, rc::Rc};

    fn number(value: &str) -> f64 {
        f64::from_bits(u64::from_str_radix(value, 16).unwrap())
    }

    fn bits(value: f64) -> String {
        format!("{:016x}", value.to_bits())
    }

    fn line_values(values: &[&str]) -> Line {
        Line {
            start: [number(values[0]), number(values[1])],
            end: [number(values[2]), number(values[3])],
        }
    }

    fn physical_value(actor: &Organism, field: &str) -> String {
        match field {
            "_mass" => bits(actor.body.mass),
            "_I" => bits(actor.body.inertia),
            "_dCenterX" => bits(actor.state.center[0]),
            "_dCenterY" => bits(actor.state.center[1]),
            "dx" => bits(actor.state.velocity[0]),
            "dy" => bits(actor.state.velocity[1]),
            "dtheta" => bits(actor.state.angular_velocity),
            "hasGrown" => actor.state.has_grown.to_string(),
            "_growthRatio" => actor.body.growth_ratio.to_string(),
            field => panic!("Unknown physical field {field}"),
        }
    }

    fn set_physical(actor: &mut Organism, field: &str, kind: &str, value: &str) {
        if kind == "int" {
            match field {
                "hasGrown" => actor.state.has_grown = value.parse().unwrap(),
                "_growthRatio" => actor.body.growth_ratio = value.parse().unwrap(),
                field => panic!("Unknown physical integer {field}"),
            }
            return;
        }
        assert_eq!(kind, "double");
        let value = number(value);
        match field {
            "_mass" => actor.body.mass = value,
            "_I" => actor.body.inertia = value,
            "_dCenterX" => actor.state.center[0] = value,
            "_dCenterY" => actor.state.center[1] = value,
            "dx" => actor.state.velocity[0] = value,
            "dy" => actor.state.velocity[1] = value,
            "dtheta" => actor.state.angular_velocity = value,
            field => panic!("Unknown physical double {field}"),
        }
    }

    fn compare(
        actual: impl ToString,
        expected: &str,
        case: &str,
        field: &str,
        errors: &mut Vec<String>,
    ) {
        let actual = actual.to_string();
        if actual != expected {
            errors.push(format!(
                "{case}\t{field}\tactual={actual}\texpected={expected}"
            ));
        }
    }

    fn run_observation(
        name: &str,
        operation: &str,
        rows: &[Vec<&str>],
        errors: &mut Vec<String>,
    ) -> usize {
        let mut lines = BTreeMap::new();
        let mut rectangles = BTreeMap::new();
        let mut parameters = PhysicsParams::default();
        let mut bodies = [actor([0.0; 2], [0.0; 2]), actor([0.0; 2], [0.0; 2])];
        let mut point = [0.0; 2];
        let mut this_organism = false;
        let mut applied = false;
        let mut deferred_constructor_suffixes = 0;
        let mut physical_rows = 0;
        for fields in rows {
            match fields[0] {
                "U" => match fields[1] {
                    "tol" => compare(bits(TOLERANCE), fields[2], name, "tol", errors),
                    "ELASTICITY" => parameters.elasticity = number(fields[2]),
                    "MAX_VEL" => parameters.max_velocity = number(fields[2]),
                    "MAX_ROT" => parameters.max_rotation = number(fields[2]),
                    field => panic!("Unknown physics parameter {field}"),
                },
                "X" => {
                    let values: Vec<i32> = fields[2..]
                        .iter()
                        .map(|value| value.parse().unwrap())
                        .collect();
                    lines.insert(
                        fields[1],
                        Line::from_offsets(
                            [values[0], values[1]],
                            [values[2], values[3]],
                            [values[4], values[5]],
                        ),
                    );
                }
                "L" => {
                    let supplied = line_values(&fields[2..]);
                    if let Some(actual) = lines.get(fields[1]) {
                        for (axis, value) in [
                            actual.start[0],
                            actual.start[1],
                            actual.end[0],
                            actual.end[1],
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            compare(bits(value), fields[axis + 2], name, fields[1], errors);
                        }
                    } else {
                        lines.insert(fields[1], supplied);
                    }
                }
                "Q" => {
                    rectangles.insert(
                        fields[1],
                        IntRect {
                            x: fields[2].parse().unwrap(),
                            y: fields[3].parse().unwrap(),
                            width: fields[4].parse().unwrap(),
                            height: fields[5].parse().unwrap(),
                        },
                    );
                }
                "P" => {
                    if fields[1] == "intersection" {
                        point = intersection(lines["a"], lines["b"]);
                        compare(bits(point[0]), fields[2], name, "intersection.x", errors);
                        compare(bits(point[1]), fields[3], name, "intersection.y", errors);
                    } else {
                        assert_eq!(fields[1], "contact");
                        point = [number(fields[2]), number(fields[3])];
                    }
                }
                "B" => {
                    let actual = match fields[1] {
                        "lines_intersect" => lines_intersect(lines["a"], lines["b"]),
                        "a_intersects_rect_a" => {
                            rectangle_intersects_line(rectangles["a"], lines["a"])
                        }
                        "b_intersects_rect_a" => {
                            rectangle_intersects_line(rectangles["a"], lines["b"])
                        }
                        "rectangles_intersect" => {
                            rectangles_intersect(rectangles["a"], rectangles["b"])
                        }
                        field => panic!("Unknown physics boolean {field}"),
                    };
                    compare(actual, fields[2], name, fields[1], errors);
                }
                "D" => {
                    let endpoint = match fields[1] {
                        "a.start" => lines["a"].start,
                        "a.end" => lines["a"].end,
                        "b.start" => lines["b"].start,
                        "b.end" => lines["b"].end,
                        field => panic!("Unknown distance endpoint {field}"),
                    };
                    compare(
                        bits(point_distance_squared(point, endpoint)),
                        fields[2],
                        name,
                        fields[1],
                        errors,
                    );
                }
                "I" => {
                    if fields[1] == "call" {
                        match fields[2] {
                            "constructor_seed" => {
                                assert_eq!(fields[3], "long");
                                let _: i64 = fields[4].parse().unwrap();
                            }
                            "thisOrganism" => {
                                assert_eq!(fields[3], "boolean");
                                this_organism = fields[4].parse().unwrap();
                            }
                            field => panic!("Unknown impulse call input {field}"),
                        }
                    } else {
                        let index = match fields[1] {
                            "a" => 0,
                            "b" => 1,
                            field => panic!("Unknown impulse owner {field}"),
                        };
                        set_physical(&mut bodies[index], fields[2], fields[3], fields[4]);
                    }
                }
                "O" => {
                    if fields[1] == "after" && !applied {
                        let [first, second] = &mut bodies;
                        touch_move(
                            first,
                            second,
                            point,
                            lines["impact"],
                            this_organism,
                            &parameters,
                        );
                        applied = true;
                    }
                    assert!(matches!(fields[1], "before" | "after"));
                    let index = match fields[2] {
                        "a" => 0,
                        "b" => 1,
                        field => panic!("Unknown impulse owner {field}"),
                    };
                    assert_eq!(
                        fields[4],
                        if matches!(fields[3], "hasGrown" | "_growthRatio") {
                            "int"
                        } else {
                            "double"
                        }
                    );
                    compare(
                        physical_value(&bodies[index], fields[3]),
                        fields[5],
                        name,
                        &format!("{} {} {}", fields[1], fields[2], fields[3]),
                        errors,
                    );
                    physical_rows += 1;
                }
                "R" => {
                    // These source suffixes observe bare Java constructors; this pure physics API receives no RNG
                    let _: i64 = fields[1].parse().unwrap();
                    let _: i32 = fields[2].parse().unwrap();
                    let _ = number(fields[3]);
                    deferred_constructor_suffixes += 1;
                }
                record => panic!("Unknown physics observation {record}"),
            }
        }
        if operation == "impulse" {
            assert!(applied);
            assert_eq!(physical_rows, 36);
        } else {
            assert!(matches!(operation, "geometry" | "offsets"));
            assert!(!applied);
            assert_eq!(physical_rows, 0);
        }
        deferred_constructor_suffixes
    }

    #[test]
    fn geometry_and_impulses_match_exact_java_observations() {
        let mut rows = Vec::new();
        let mut name = "";
        let mut operation = "";
        let mut cases = 0;
        let mut deferred_constructor_suffixes = 0;
        let mut errors = Vec::new();
        for line in include_str!("../tests/fixtures/physics.tsv").lines() {
            let fields: Vec<_> = line.split('\t').collect();
            match fields[0] {
                "C" => {
                    assert!(rows.is_empty());
                    name = fields[1];
                    operation = fields[2];
                }
                "E" => {
                    deferred_constructor_suffixes +=
                        run_observation(name, operation, &rows, &mut errors);
                    rows.clear();
                    cases += 1;
                }
                _ => rows.push(fields),
            }
        }
        assert!(rows.is_empty());
        assert_eq!(cases, 91);
        assert_eq!(deferred_constructor_suffixes, 48);
        if !errors.is_empty() {
            let report = std::env::temp_dir().join("physics-oracle-mismatches.tsv");
            std::fs::write(&report, errors.join("\n") + "\n").unwrap();
            panic!(
                "{} physics mismatches; report={}\n{}",
                errors.len(),
                report.display(),
                errors
                    .iter()
                    .take(40)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }

    fn actor(center: [f64; 2], velocity: [f64; 2]) -> Organism {
        let gene = Gene::new(12.0, 0.0, Some(0xff00ff00), 0, [0; 36]).shared();
        let code = Genome::from_parts(
            &[gene],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        let mut actor =
            Organism::unplaced(Rc::new(RefCell::new(code)), OrganismState::default()).unwrap();
        actor.state.center = center;
        actor.state.velocity = velocity;
        actor.body.mass = 1.0;
        actor.body.inertia = 1.0;
        actor
    }

    #[test]
    fn contact_square_rounding_matches_the_original_method_in_both_profiles() {
        let mut bodies = [actor([0.0; 2], [0.0; 2]), actor([0.0; 2], [0.0; 2])];
        let mut parameters = PhysicsParams::default();
        let mut point = [0.0; 2];
        let mut line = Line {
            start: [0.0; 2],
            end: [0.0; 2],
        };
        let mut this_organism = false;
        let mut controls = 0;
        for row in include_str!("../tests/fixtures/contact_square.inputs.tsv").lines() {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 4);
            match fields[0] {
                "a" | "b" => set_physical(
                    &mut bodies[usize::from(fields[0] == "b")],
                    fields[1],
                    fields[2],
                    fields[3],
                ),
                "u" => {
                    assert_eq!(fields[2], "double");
                    let value = number(fields[3]);
                    match fields[1] {
                        "ELASTICITY" => parameters.elasticity = value,
                        "MAX_VEL" => parameters.max_velocity = value,
                        "MAX_ROT" => parameters.max_rotation = value,
                        field => panic!("Unknown contact parameter {field}"),
                    }
                }
                "p" => {
                    assert_eq!(fields[2], "double");
                    let index = match fields[1] {
                        "x" => 0,
                        "y" => 1,
                        field => panic!("Unknown point field {field}"),
                    };
                    point[index] = number(fields[3]);
                }
                "l" => {
                    assert_eq!(fields[2], "double");
                    let value = number(fields[3]);
                    match fields[1] {
                        "x1" => line.start[0] = value,
                        "y1" => line.start[1] = value,
                        "x2" => line.end[0] = value,
                        "y2" => line.end[1] = value,
                        field => panic!("Unknown line field {field}"),
                    }
                }
                "flag" => {
                    assert_eq!(fields[1], "thisOrganism");
                    assert_eq!(fields[2], "boolean");
                    this_organism = fields[3].parse().unwrap();
                }
                field => panic!("Unknown contact input {field}"),
            }
            controls += 1;
        }
        assert_eq!(controls, 25);
        let [first, second] = &mut bodies;
        touch_move(first, second, point, line, this_organism, &parameters);
        let mut compared = 0;
        let mut errors = Vec::new();
        for row in include_str!("../tests/fixtures/contact_square.outputs.tsv").lines() {
            let fields: Vec<_> = row.split('\t').collect();
            match fields.as_slice() {
                ["O", owner, field, expected] => {
                    assert!(matches!(*owner, "a" | "b"));
                    compare(
                        physical_value(&bodies[usize::from(*owner == "b")], field),
                        expected,
                        "contact-square-rounding",
                        field,
                        &mut errors,
                    );
                    compared += 1;
                }
                ["R", "unchanged"] => {
                    // Original purity metadata; this physics API receives no RNG
                }
                _ => panic!("Unknown source contact observation {row}"),
            }
        }
        assert_eq!(compared, 6);
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }

    #[test]
    fn reference_intersection_quirks_and_wrapped_coordinates_remain_observable() {
        let horizontal = Line {
            start: [0.0, 0.0],
            end: [10.0, 0.0],
        };
        let vertical = Line {
            start: [2.0, -1.0],
            end: [2.0, 1.0],
        };
        assert!(lines_intersect(horizontal, vertical));
        assert_eq!(intersection(horizontal, vertical), [2.0, 0.0]);
        let reversed = Line {
            start: vertical.end,
            end: vertical.start,
        };
        assert!(lines_intersect(horizontal, reversed));
        assert_eq!(intersection(horizontal, reversed), [3.5, 0.0]);
        assert!(!lines_intersect(
            horizontal,
            Line {
                start: [11.0, 0.0],
                end: [12.0, 0.0]
            }
        ));
        assert!(rectangle_intersects_line(
            IntRect {
                x: 0,
                y: 0,
                width: 10,
                height: 10
            },
            horizontal
        ));
        assert!(!rectangle_intersects_line(
            IntRect {
                x: 10,
                y: 0,
                width: -10,
                height: 10
            },
            horizontal
        ));
        let wrapped = Line::from_offsets([i32::MAX, i32::MIN], [1, -1], [1, -1]);
        assert_eq!(wrapped.start, [f64::from(i32::MIN), f64::from(i32::MAX)]);
        assert_eq!(wrapped.end, [2.0, -2.0]);
        assert_eq!(point_distance_squared([1.0, 2.0], [4.0, 6.0]), 25.0);
    }

    #[test]
    fn head_on_impulse_preserves_momentum_and_ignores_peer_growth() {
        for peer_growth in [0, 1, -1, i32::MAX] {
            let mut first = actor([-1.0, 0.0], [1.0, 0.0]);
            let mut second = actor([1.0, 0.0], [-1.0, 0.0]);
            second.state.has_grown = peer_growth;
            let centers = [first.state.center, second.state.center];
            touch_move(
                &mut first,
                &mut second,
                [0.0, 0.0],
                Line {
                    start: [0.0, -1.0],
                    end: [0.0, 1.0],
                },
                true,
                &PhysicsParams::default(),
            );
            assert_eq!(first.state.velocity, [-0.8, 0.0]);
            assert_eq!(second.state.velocity, [0.8, 0.0]);
            assert_eq!(first.state.angular_velocity, 0.0);
            assert_eq!(second.state.angular_velocity, 0.0);
            assert_eq!([first.state.center, second.state.center], centers);
            assert_eq!(first.state.velocity[0] + second.state.velocity[0], 0.0);
        }
    }

    #[test]
    fn java_clamping_and_parameter_boundaries_do_not_silently_replace_nonfinite_arithmetic() {
        let nan = f64::from_bits(0x7ff8000000000042);
        assert_eq!(between(nan, -1.0, 1.0).to_bits(), nan.to_bits());
        assert_eq!(between(-0.0, -0.0, 0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(between(-0.0, 0.0, 0.0).to_bits(), 0);
        assert_eq!(between(f64::INFINITY, -1.0, 1.0), 1.0);
        assert_eq!(between(f64::NEG_INFINITY, -1.0, 1.0), -1.0);
        assert!(PhysicsParams::default().validate().is_ok());
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(PhysicsParams {
                max_velocity: value,
                ..PhysicsParams::default()
            }
            .validate()
            .is_err());
            assert!(PhysicsParams {
                max_rotation: value,
                ..PhysicsParams::default()
            }
            .validate()
            .is_err());
        }
        assert!(PhysicsParams {
            elasticity: 1.1,
            ..PhysicsParams::default()
        }
        .validate()
        .is_err());
        assert!(PhysicsParams {
            max_velocity: 0.0,
            max_rotation: 0.0,
            ..PhysicsParams::default()
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn persisted_hidden_limits_retain_finite_negative_values() {
        let parameters = PhysicsParams {
            max_velocity: -1.0,
            max_rotation: -0.125,
            ..PhysicsParams::default()
        };
        assert!(parameters.validate().is_ok());
        assert_eq!(
            between(0.0, -parameters.max_velocity, parameters.max_velocity),
            1.0
        );
        assert_eq!(
            between(0.0, -parameters.max_rotation, parameters.max_rotation),
            0.125
        );
    }
}
