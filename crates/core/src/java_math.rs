/*
 * ====================================================
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 *
 * Developed at SunSoft, a Sun Microsystems, Inc. business.
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 *
 */
// Rust translation of fdlibm s_atan.c 1.3 and e_atan2.c 1.3, dated 1995-01-18
// Sources: https://netlib.org/fdlibm/s_atan.c and https://netlib.org/fdlibm/e_atan2.c
// Word access uses explicit bit conversion instead of the original C pointer aliasing

#[path = "java_math/pow3.rs"]
mod hotspot_pow3;
pub(crate) use hotspot_pow3::pow3;

// Match Java's negative quiet NaN only for fresh invalid elementary arithmetic
#[inline]
fn fresh_invalid_nan(result: f64, operand_is_nan: bool) -> f64 {
    if result.is_nan() && !operand_is_nan {
        f64::from_bits(0xfff8_0000_0000_0000)
    } else {
        result
    }
}

#[inline]
pub(crate) fn add(left: f64, right: f64) -> f64 {
    let result = left + right;
    fresh_invalid_nan(result, left.is_nan() || right.is_nan())
}

#[inline]
pub(crate) fn sub(left: f64, right: f64) -> f64 {
    let result = left - right;
    fresh_invalid_nan(result, left.is_nan() || right.is_nan())
}

#[inline]
pub(crate) fn mul(left: f64, right: f64) -> f64 {
    let result = left * right;
    fresh_invalid_nan(result, left.is_nan() || right.is_nan())
}

#[inline]
pub(crate) fn div(left: f64, right: f64) -> f64 {
    let result = left / right;
    fresh_invalid_nan(result, left.is_nan() || right.is_nan())
}

#[inline]
pub(crate) fn sqrt(value: f64) -> f64 {
    let result = value.sqrt();
    fresh_invalid_nan(result, value.is_nan())
}

const ATAN_HI: [f64; 4] = [
    f64::from_bits(0x3fddac670561bb4f),
    f64::from_bits(0x3fe921fb54442d18),
    f64::from_bits(0x3fef730bd281f69b),
    f64::from_bits(0x3ff921fb54442d18),
];
const ATAN_LO: [f64; 4] = [
    f64::from_bits(0x3c7a2b7f222f65e2),
    f64::from_bits(0x3c81a62633145c07),
    f64::from_bits(0x3c7007887af0cbbd),
    f64::from_bits(0x3c91a62633145c07),
];
const AT: [f64; 11] = [
    f64::from_bits(0x3fd555555555550d),
    f64::from_bits(0xbfc999999998ebc4),
    f64::from_bits(0x3fc24924920083ff),
    f64::from_bits(0xbfbc71c6fe231671),
    f64::from_bits(0x3fb745cdc54c206e),
    f64::from_bits(0xbfb3b0f2af749a6d),
    f64::from_bits(0x3fb10d66a0d03d51),
    f64::from_bits(0xbfadde2d52defd9a),
    f64::from_bits(0x3fa97b4b24760deb),
    f64::from_bits(0xbfa2b4442c6a6c2f),
    f64::from_bits(0x3f90ad3ae322da11),
];

fn high(value: f64) -> i32 {
    (value.to_bits() >> 32) as i32
}
fn low(value: f64) -> u32 {
    value.to_bits() as u32
}

pub(crate) fn atan(mut x: f64) -> f64 {
    let hx = high(x);
    let ix = hx & 0x7fff_ffff;
    if ix >= 0x4410_0000 {
        if ix > 0x7ff0_0000 || (ix == 0x7ff0_0000 && low(x) != 0) {
            return x + x;
        }
        return if hx > 0 {
            ATAN_HI[3] + ATAN_LO[3]
        } else {
            -ATAN_HI[3] - ATAN_LO[3]
        };
    }
    let id: i32;
    if ix < 0x3fdc_0000 {
        if ix < 0x3e20_0000 && 1.0e300 + x > 1.0 {
            return x;
        }
        id = -1;
    } else {
        x = x.abs();
        if ix < 0x3ff3_0000 {
            if ix < 0x3fe6_0000 {
                id = 0;
                x = (2.0 * x - 1.0) / (2.0 + x);
            } else {
                id = 1;
                x = (x - 1.0) / (x + 1.0);
            }
        } else if ix < 0x4003_8000 {
            id = 2;
            x = (x - 1.5) / (1.0 + 1.5 * x);
        } else {
            id = 3;
            x = -1.0 / x;
        }
    }
    let z = x * x;
    let w = z * z;
    let s1 = z * (AT[0] + w * (AT[2] + w * (AT[4] + w * (AT[6] + w * (AT[8] + w * AT[10])))));
    let s2 = w * (AT[1] + w * (AT[3] + w * (AT[5] + w * (AT[7] + w * AT[9]))));
    if id < 0 {
        x - x * (s1 + s2)
    } else {
        let z = ATAN_HI[id as usize] - ((x * (s1 + s2) - ATAN_LO[id as usize]) - x);
        if hx < 0 {
            -z
        } else {
            z
        }
    }
}

/// Preserve fdlibm rounding before Java narrows rotated endpoints to integers
pub(crate) fn atan2(y: f64, x: f64) -> f64 {
    const TINY: f64 = 1.0e-300;
    const PI_O_4: f64 = f64::from_bits(0x3fe921fb54442d18);
    const PI_O_2: f64 = f64::from_bits(0x3ff921fb54442d18);
    const PI: f64 = f64::from_bits(0x400921fb54442d18);
    const PI_LO: f64 = f64::from_bits(0x3ca1a62633145c07);
    let hx = high(x);
    let ix = hx & 0x7fff_ffff;
    let lx = low(x);
    let hy = high(y);
    let iy = hy & 0x7fff_ffff;
    let ly = low(y);
    if ((ix as u32 | ((lx | lx.wrapping_neg()) >> 31)) > 0x7ff0_0000)
        || ((iy as u32 | ((ly | ly.wrapping_neg()) >> 31)) > 0x7ff0_0000)
    {
        return x + y;
    }
    if (hx.wrapping_sub(0x3ff0_0000) as u32 | lx) == 0 {
        return atan(y);
    }
    let m = ((hy >> 31) & 1) | ((hx >> 30) & 2);
    if (iy as u32 | ly) == 0 {
        return match m {
            0 | 1 => y,
            2 => PI + TINY,
            _ => -PI - TINY,
        };
    }
    if (ix as u32 | lx) == 0 {
        return if hy < 0 {
            -PI_O_2 - TINY
        } else {
            PI_O_2 + TINY
        };
    }
    if ix == 0x7ff0_0000 {
        return if iy == 0x7ff0_0000 {
            match m {
                0 => PI_O_4 + TINY,
                1 => -PI_O_4 - TINY,
                2 => 3.0 * PI_O_4 + TINY,
                _ => -3.0 * PI_O_4 - TINY,
            }
        } else {
            match m {
                0 => 0.0,
                1 => -0.0,
                2 => PI + TINY,
                _ => -PI - TINY,
            }
        };
    }
    if iy == 0x7ff0_0000 {
        return if hy < 0 {
            -PI_O_2 - TINY
        } else {
            PI_O_2 + TINY
        };
    }
    let k = (iy - ix) >> 20;
    let z = if k > 60 {
        PI_O_2 + 0.5 * PI_LO
    } else if hx < 0 && k < -60 {
        0.0
    } else {
        atan((y / x).abs())
    };
    match m {
        0 => z,
        1 => f64::from_bits(z.to_bits() ^ (1_u64 << 63)),
        2 => PI - (z - PI_LO),
        _ => (z - PI_LO) - PI,
    }
}
/// Java Math.min propagates NaN and selects negative zero when either argument is negative zero
pub(crate) fn min(left: f64, right: f64) -> f64 {
    if left.is_nan() {
        left
    } else if left == 0.0 && right == 0.0 {
        f64::from_bits(left.to_bits() | right.to_bits())
    } else if left <= right {
        left
    } else {
        right
    }
}

pub(crate) fn max(left: f64, right: f64) -> f64 {
    -min(-left, -right)
}

/// Preserve Math.round(value / 10)'s integer division and implicit float conversion
pub(crate) fn round_integer_tenth(value: i32) -> i32 {
    ((value / 10) as f32) as i32
}

#[cfg(test)]
mod tests {
    #[test]
    fn elementary_fresh_invalid_results_match_java_nan_bits() {
        use std::hint::black_box;

        fn check(operation: fn(f64, f64) -> f64, cases: &[(f64, f64)]) {
            for &(left, right) in cases {
                assert_eq!(
                    operation(black_box(left), black_box(right)).to_bits(),
                    0xfff8_0000_0000_0000
                );
            }
        }
        check(
            super::add,
            &[(f64::INFINITY, f64::NEG_INFINITY), (f64::NEG_INFINITY, f64::INFINITY)],
        );
        check(
            super::sub,
            &[(f64::INFINITY, f64::INFINITY), (f64::NEG_INFINITY, f64::NEG_INFINITY)],
        );
        for zero in [0.0, -0.0] {
            for infinity in [f64::INFINITY, f64::NEG_INFINITY] {
                check(super::mul, &[(zero, infinity), (infinity, zero)]);
            }
        }
        for left in [0.0, -0.0] {
            for right in [0.0, -0.0] {
                check(super::div, &[(left, right)]);
            }
        }
        for left in [f64::INFINITY, f64::NEG_INFINITY] {
            for right in [f64::INFINITY, f64::NEG_INFINITY] {
                check(super::div, &[(left, right)]);
            }
        }
        for value in [-1.0, f64::NEG_INFINITY] {
            assert_eq!(super::sqrt(black_box(value)).to_bits(), 0xfff8_0000_0000_0000);
        }
    }

    #[test]
    fn elementary_defined_results_preserve_bits() {
        use std::hint::black_box;

        fn check(operation: fn(f64, f64) -> f64, cases: &[(f64, f64, f64)]) {
            for &(left, right, expected) in cases {
                assert_eq!(
                    operation(black_box(left), black_box(right)).to_bits(),
                    expected.to_bits()
                );
            }
        }
        check(super::add, &[
            (1.5, 2.25, 3.75), (-0.0, -0.0, -0.0), (0.0, -0.0, 0.0),
            (f64::INFINITY, 2.0, f64::INFINITY), (f64::NEG_INFINITY, -2.0, f64::NEG_INFINITY),
        ]);
        check(super::sub, &[
            (5.5, 2.0, 3.5), (-0.0, 0.0, -0.0), (0.0, -0.0, 0.0),
            (f64::INFINITY, 2.0, f64::INFINITY), (2.0, f64::INFINITY, f64::NEG_INFINITY),
        ]);
        check(super::mul, &[
            (-3.0, 2.0, -6.0), (-0.0, 2.0, -0.0), (-0.0, -2.0, 0.0),
            (f64::NEG_INFINITY, 2.0, f64::NEG_INFINITY), (f64::INFINITY, -2.0, f64::NEG_INFINITY),
        ]);
        check(super::div, &[
            (7.0, 2.0, 3.5), (-0.0, 2.0, -0.0), (0.0, -2.0, -0.0),
            (-1.0, 0.0, f64::NEG_INFINITY), (1.0, -0.0, f64::NEG_INFINITY),
            (1.0, f64::INFINITY, 0.0), (-1.0, f64::INFINITY, -0.0),
            (f64::NEG_INFINITY, -2.0, f64::INFINITY),
        ]);
        for (value, expected) in [
            (4.0, 2.0_f64), (0.0, 0.0), (-0.0, -0.0), (f64::INFINITY, f64::INFINITY),
        ] {
            assert_eq!(super::sqrt(black_box(value)).to_bits(), expected.to_bits());
        }
    }

    #[test]
    fn elementary_existing_nan_results_match_native_operations() {
        use std::hint::black_box;

        let operations: [(fn(f64, f64) -> f64, fn(f64, f64) -> f64); 4] = [
            (super::add, |left, right| left + right),
            (super::sub, |left, right| left - right),
            (super::mul, |left, right| left * right),
            (super::div, |left, right| left / right),
        ];
        for bits in [
            0x7ff8_0000_0000_0042, 0xfff8_0000_0000_0042,
            0x7ff0_0000_0000_0042, 0xfff0_0000_0000_0042,
        ] {
            let nan = f64::from_bits(bits);
            for (operation, native) in operations {
                for (left, right) in [(nan, 2.0), (2.0, nan)] {
                    let expected = native(black_box(left), black_box(right));
                    let actual = operation(black_box(left), black_box(right));
                    assert!(actual.is_nan());
                    assert_eq!(actual.to_bits(), expected.to_bits());
                    assert_ne!(actual.to_bits() & (1_u64 << 51), 0);
                }
            }
            let expected = black_box(nan).sqrt();
            let actual = super::sqrt(black_box(nan));
            assert!(actual.is_nan());
            assert_eq!(actual.to_bits(), expected.to_bits());
            assert_ne!(actual.to_bits() & (1_u64 << 51), 0);
        }
    }

    #[test]
    fn rounded_integer_tenths_match_actual_java_overload_observations() {
        let fixture = include_str!("../tests/fixtures/thresholds.tsv");
        assert_eq!(fixture.lines().count(), 20);
        let mut count = 0;
        for line in fixture.lines() {
            let row: Vec<_> = line.split('\t').collect();
            if row[0] == "R" {
                assert_eq!(row.len(), 3);
                assert_eq!(
                    super::round_integer_tenth(row[1].parse().unwrap()),
                    row[2].parse::<i32>().unwrap()
                );
                count += 1;
            }
        }
        assert_eq!(count, 11);
    }

    #[test]
    fn minimum_preserves_nan_and_negative_zero() {
        for (left, right) in [(0.0, -0.0), (-0.0, 0.0), (-0.0, -0.0)] {
            assert_eq!(super::min(left, right).to_bits(), (-0.0_f64).to_bits());
        }
        assert_eq!(super::min(0.0, 0.0).to_bits(), 0);
        assert!(super::min(f64::NAN, 1.0).is_nan());
        assert!(super::min(1.0, f64::NAN).is_nan());
        assert_eq!(super::max(-0.0, 0.0).to_bits(), 0);
        assert_eq!(super::max(0.0, -0.0).to_bits(), 0);
        assert!(super::max(f64::NAN, 1.0).is_nan());
        assert!(super::max(1.0, f64::NAN).is_nan());
        assert_eq!(
            super::min(f64::NEG_INFINITY, f64::INFINITY),
            f64::NEG_INFINITY
        );
    }
}
