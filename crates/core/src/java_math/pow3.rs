/*
 * Copyright (c) 2016, 2021, Intel Corporation. All rights reserved.
 * Copyright (C) 2021, Tencent. All rights reserved.
 * Intel Math Library (LIBM) Source Code
 *
 * DO NOT ALTER OR REMOVE COPYRIGHT NOTICES OR THIS FILE HEADER.
 *
 * This code is free software; you can redistribute it and/or modify it
 * under the terms of the GNU General Public License version 2 only, as
 * published by the Free Software Foundation.
 *
 * This code is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
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
 *
 */
// SPDX-License-Identifier: GPL-2.0-only

//! Fixed exponent 3 following the pinned x86_64 HotSpot Intel LIBM rounding sequence
//! x86_64 retains SSE reciprocal; other targets use captured reference quantization
//! Remaining arithmetic requires round-to-nearest and gradual underflow

#[path = "tables.rs"]
mod tables;
use tables::*;

#[path = "reciprocal_bins.rs"]
mod reciprocal_bins;

#[derive(Clone, Copy, Debug)]
struct V([f64; 2]);
fn v(x: f64) -> V {
    V([x, 0.0])
}
fn load(words: &[u32], offset: usize) -> V {
    assert_eq!(offset % 4, 0);
    let i = offset / 4;
    V([
        f64::from_bits(words[i] as u64 | ((words[i + 1] as u64) << 32)),
        f64::from_bits(words[i + 2] as u64 | ((words[i + 3] as u64) << 32)),
    ])
}
fn sd_add(a: V, b: V) -> V {
    V([a.0[0] + b.0[0], a.0[1]])
}
fn sd_sub(a: V, b: V) -> V {
    V([a.0[0] - b.0[0], a.0[1]])
}
fn sd_mul(a: V, b: V) -> V {
    V([a.0[0] * b.0[0], a.0[1]])
}
fn pd_add(a: V, b: V) -> V {
    V([a.0[0] + b.0[0], a.0[1] + b.0[1]])
}
fn pd_mul(a: V, b: V) -> V {
    V([a.0[0] * b.0[0], a.0[1] * b.0[1]])
}
fn low(a: V) -> V {
    V([a.0[0], a.0[0]])
}
fn high(a: V) -> V {
    V([a.0[1], a.0[1]])
}
fn mask(a: V, bits: u64) -> V {
    V([f64::from_bits(a.0[0].to_bits() & bits), 0.0])
}
fn exp_adjust(a: V, bits: i32) -> V {
    V(a.0.map(|value| f64::from_bits(value.to_bits().wrapping_add((bits as u32 as u64) << 32))))
}
fn bsr(value: u32) -> i32 {
    assert_ne!(value, 0);
    31 - value.leading_zeros() as i32
}

#[cfg(target_arch = "x86_64")]
fn reciprocal(value: f32) -> f32 {
    // x86_64 guarantees SSE; this arithmetic intrinsic reproduces the source RCPPS operation
    // The intrinsic accesses no pointers or external memory
    unsafe {
        use std::arch::x86_64::{_mm_cvtss_f32, _mm_rcp_ps, _mm_set_ss};
        _mm_cvtss_f32(_mm_rcp_ps(_mm_set_ss(value)))
    }
}

fn quantized_reciprocal<const PORTABLE: bool>(packed: u32) -> u32 {
    #[cfg(target_arch = "x86_64")]
    if !PORTABLE {
        let approximation = reciprocal(f32::from_bits(packed));
        return (approximation * f32::from_bits(1069088768))
            .to_bits()
            .wrapping_add(8192);
    }
    reciprocal_bins::quantized(packed)
}

/// Compute the fixed positive odd exponent 3 with the pinned source rounding order
pub fn pow3(x: f64) -> f64 {
    pow3_impl::<false>(x)
}

// The private selector exercises the portable production path on the reference host
fn pow3_impl<const PORTABLE: bool>(x: f64) -> f64 {
    let sign = x.to_bits() & 0x8000_0000_0000_0000;
    let absolute = x.to_bits() & 0x7fff_ffff_ffff_ffff;
    if absolute >= 0x7ff0_0000_0000_0000 || absolute == 0 {
        return x + x;
    }
    // These broad exponent regions cannot approach a rounded finite/nonzero cube result
    if absolute < ((1023 - 359) as u64) << 52 {
        return f64::from_bits(sign);
    }
    if absolute >= ((1023 + 342) as u64) << 52 {
        return f64::from_bits(sign | 0x7ff0_0000_0000_0000);
    }
    let magnitude = finite::<PORTABLE>(f64::from_bits(absolute), sign != 0);
    f64::from_bits(magnitude.to_bits() | sign)
}

fn finite<const PORTABLE: bool>(x: f64, negative: bool) -> f64 {
    // Source lines807-863: reciprocal quantization and compensated reduced argument
    let bits = x.to_bits();
    let mut a = (bits >> 48) as i32;
    let exponent = ((a & 32752) - 16368) >> 4;
    // The negative source branch derives its mask after decrementing the exponent highword
    let mask_exponent = if negative { exponent - 1 } else { exponent };
    let split = bsr((mask_exponent.abs() + 1) as u32);
    let split_mask = (((-1_i32).wrapping_shl(split as u32)) as u32 as u64) << 32;
    let packed = (((bits | 0x3ff0_0000_0000_0000) >> 27) & 0xffff_ffff) as u32;
    // PSRLD shifts each32bit word separately
    let packed = packed >> 2;
    let quantized = quantized_reciprocal::<PORTABLE>(packed);
    let mut index = ((quantized & 16760832) >> 10) as i32 - 3648;
    assert!(index >= 0 && (index as usize) + 16 <= L_TBL_POW.len() * 4);
    let multiplier = f64::from_bits(((quantized as u64) << 29) & 0xffff_f800_0000_0000);
    let mut z = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | 0x77f0_0000_0000_0000);
    a -= 16367;
    let split_bits = if (a as u32) <= 1 {
        z.to_bits().wrapping_add(0x8000_0000)
    } else {
        z.to_bits()
    };
    let mut v5 = v(f64::from_bits(split_bits & split_mask));
    z -= v5.0[0];
    let k = (a - 1) >> 4;
    let mut v7 = v(k as f64);
    v5 = pd_mul(v5, v(multiplier));
    let mut v3 = v(z * multiplier);
    // Source lines864-903: ordered log polynomial and high/low table correction
    let mut v1 = load(&COEFF_POW, 0);
    v5 = sd_sub(v5, v(f64::from_bits(0x3ff7_2000_0000_0000)));
    let mut v4 = load(&COEFF_POW, 16);
    let region = bsr(k.unsigned_abs() + 1) * 16 + 512;
    v5 = V([v5.0[0], v3.0[0]]);
    let mut v6 = load(&COEFF_POW, 32);
    v3 = sd_add(v3, v5);
    v5 = pd_add(v5, load(&L_TBL_POW, index as usize));
    let mut v0 = load(&COEFF_POW, 48);
    let mut v2 = low(v3);
    v3 = sd_mul(v3, v3);
    v1 = pd_mul(v1, v2);
    v4 = pd_mul(v4, v2);
    v5 = sd_add(v5, v7);
    v2 = sd_mul(v2, v3);
    v6 = pd_add(v6, v1);
    v3 = sd_mul(v3, v3);
    v0 = pd_add(v0, v4);
    v1 = v(3.0);
    v7 = high(v5);
    v4 = v(f64::from_bits(3_f64.to_bits() & 0xffff_fff8_0000_0000));
    v6 = pd_mul(v6, v2);
    v3 = low(v3);
    v0 = pd_mul(v0, v2);
    v3 = pd_mul(v3, v6);
    if region >= 624 {
        // Source lines1054-1157: split scaling retains underflow correction before the final multiply
        v0 = sd_add(v0, v7);
        v3 = pd_add(v3, v0);
        v6 = v(f64::from_bits(0x42b8_0000_0000_0000));
        v0 = high(v3);
        v0 = sd_add(v0, v3);
        v3 = v5;
        v5 = sd_add(v5, v0);
        v3 = sd_sub(v3, v5);
        v7 = v5;
        v5 = mask(v5, 0xffff_ffff_f800_0000);
        v2 = v1;
        v4 = mask(v1, 0xffff_ffff_f800_0000);
        v7 = sd_sub(v7, v5);
        v0 = sd_add(v0, v3);
        v1 = sd_sub(v1, v4);
        v4 = sd_mul(v4, v5);
        v0 = sd_add(v0, v7);
        v2 = sd_mul(v2, v0);
        v7 = v6;
        v1 = sd_mul(v1, v5);
        v6 = sd_add(v6, v4);
        let rounded_bits = v6.0[0].to_bits() as u32;
        v6 = sd_sub(v6, v7);
        v2 = sd_add(v2, v1);
        v7 = load(&E_COEFF, 0);
        v3 = load(&E_COEFF, 16);
        v4 = sd_sub(v4, v6);
        let rounded_high = (v6.0[0].to_bits() >> 48) as i32;
        index = ((rounded_bits & 255) * 16) as i32;
        v5 = load(&T_EXP, index as usize);
        v2 = sd_add(v2, v4);
        let integral = (rounded_bits as i32) >> 8;
        let half = integral >> 1;
        let remainder = integral - half;
        v1 = v(f64::from_bits(0x3fe6_2e42_fefa_39ef));
        if (rounded_high & 32767) > 16529 {
            return if x < 1.0 { 0.0 } else { f64::INFINITY };
        }
        v0 = low(v2);
        v4 = low(v2);
        v0 = pd_mul(v0, v0);
        v7 = pd_mul(v7, v4);
        v1 = sd_mul(v1, v2);
        v0 = sd_mul(v0, v0);
        v5 = exp_adjust(v5, half.wrapping_shl(20));
        v3 = pd_add(v3, v7);
        v1 = sd_mul(v1, v5);
        v6 = high(v5);
        v0 = pd_mul(v0, v3);
        v1 = sd_add(v1, v6);
        v3 = high(v0);
        v0 = sd_mul(v0, v5);
        v3 = sd_mul(v3, v5);
        v4 = v(f64::from_bits(
            ((remainder.wrapping_mul(16).wrapping_add(16368)) as u16 as u64) << 48,
        ));
        v0 = sd_add(v0, v1);
        v0 = sd_add(v0, v3);
        v1 = v0;
        v0 = sd_add(v0, v5);
        v0 = sd_mul(v0, v4);
        if v0.0[0].is_subnormal() || v0.0[0] == 0.0 {
            // Source lines1567-1595: mask the normal factor before final subnormal-grid rounding
            let p5 = ((v5.0[0].to_bits() >> 48) as i32 & 32752) - 16368;
            let p4 = (v4.0[0].to_bits() >> 48) as i32 & 32752;
            let shifted = (p4 + p5) >> 4;
            let shift = -31 - shifted;
            if shift <= 20 {
                let mask = if shift <= 0 {
                    0xffff_ffff_0000_0000
                } else {
                    (((-1_i32).wrapping_shl(shift as u32)) as u32 as u64) << 32
                };
                let leading = f64::from_bits(v5.0[0].to_bits() & mask);
                let trailing = (v5.0[0] - leading) + v1.0[0];
                return leading * v4.0[0] + trailing * v4.0[0];
            }
        }
        return v0.0[0];
    }
    // Source lines904-956: compensated y*log2(x), exp table and ordered exp polynomial
    v6 = v(f64::from_bits(0x42b8_0000_0000_0000));
    v2 = v1;
    v1 = sd_sub(v1, v4);
    v4 = sd_mul(v4, v5);
    v0 = sd_add(v0, v7);
    v1 = sd_mul(v1, v5);
    v7 = v6;
    v6 = sd_add(v6, v4);
    v3 = pd_add(v3, v0);
    let rounded = v6.0[0].to_bits() as i32;
    v6 = sd_sub(v6, v7);
    v0 = high(v3);
    v4 = sd_sub(v4, v6);
    v0 = sd_add(v0, v3);
    v5 = load(&T_EXP, ((rounded as u32 & 255) * 16) as usize);
    v4 = sd_add(v4, v1);
    v2 = sd_mul(v2, v0);
    v7 = load(&E_COEFF, 0);
    v3 = load(&E_COEFF, 16);
    let adjust = rounded.wrapping_shl(12) & -1048576;
    v2 = sd_add(v2, v4);
    v1 = v(f64::from_bits(0x3fe6_2e42_fefa_39ef));
    v0 = low(v2);
    v4 = low(v2);
    v1 = sd_mul(v1, v2);
    v0 = pd_mul(v0, v0);
    v7 = pd_mul(v7, v4);
    v5 = exp_adjust(v5, adjust);
    v1 = sd_mul(v1, v5);
    v6 = high(v5);
    v0 = sd_mul(v0, v0);
    v3 = pd_add(v3, v7);
    v1 = sd_add(v1, v6);
    v0 = pd_mul(v0, v3);
    v3 = high(v0);
    v0 = sd_mul(v0, v5);
    v3 = sd_mul(v3, v5);
    v0 = sd_add(v0, v1);
    v0 = sd_add(v0, v3);
    v0 = sd_add(v0, v5);
    v0.0[0]
}

#[cfg(test)]
mod tests {
    use super::{pow3, pow3_impl, quantized_reciprocal};

    #[test]
    fn actual_hotspot_normal_and_subnormal_rounding_are_distinct_contracts() {
        // Values come from actual Java observations, including two dense-growth operands
        for (input, expected) in [
            (0x4013_a2ce_ee1a_f054, 0x405d_9311_1bf9_f4c4),
            (0x4019_34a7_5c56_54bc, 0x406f_46ea_1f6c_eb77),
            (0x2a9f_ffff_ffff_fffe, 0x0007_ffff_ffff_fffe),
            (0x2aa0_0000_0000_0003, 0x0008_0000_0000_0005),
        ] {
            assert_eq!(pow3(f64::from_bits(input)).to_bits(), expected);
            assert_eq!(pow3_impl::<true>(f64::from_bits(input)).to_bits(), expected);
            assert_eq!(
                pow3_impl::<true>(f64::from_bits(input | (1 << 63))).to_bits(),
                expected | (1 << 63)
            );
            assert_eq!(
                pow3(f64::from_bits(input | (1 << 63))).to_bits(),
                expected | (1 << 63)
            );
        }
    }

    #[test]
    fn negative_argument_reduction_retains_its_source_split_mask() {
        // Frozen actual Math observation; the positive reduction's split mask loses this final bit
        assert_eq!(
            pow3(f64::from_bits(0xc024_264f_6095_13d1)).to_bits(),
            0xc08f_f4ec_ec0a_1d3a
        );
        assert_eq!(
            pow3_impl::<true>(f64::from_bits(0xc024_264f_6095_13d1)).to_bits(),
            0xc08f_f4ec_ec0a_1d3a
        );
    }

    #[test]
    fn odd_fixed_exponent_preserves_ieee_signs_and_classes() {
        for bits in [0, 1 << 63, 0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000] {
            assert_eq!(pow3(f64::from_bits(bits)).to_bits(), bits);
            assert_eq!(pow3_impl::<true>(f64::from_bits(bits)).to_bits(), bits);
        }
        for bits in [0x7ff0_0000_0000_0042, 0xfff8_0000_0000_0042] {
            assert!(pow3(f64::from_bits(bits)).is_nan());
            assert!(pow3_impl::<true>(f64::from_bits(bits)).is_nan());
        }
        for value in [1.0, 2.0, 0.5, f64::MIN_POSITIVE, f64::MAX] {
            assert_eq!(pow3(-value).to_bits(), pow3(value).to_bits() | (1 << 63));
        }
    }

    #[test]
    fn near_one_table_reduction_is_monotone_across_its_boundary() {
        let mut previous = pow3(f64::from_bits(0x3fef_ffff_ffff_f000));
        for bits in 0x3fef_ffff_ffff_f001..=0x3ff0_0000_0000_1000 {
            let current = pow3(f64::from_bits(bits));
            assert!(current >= previous);
            assert_eq!(
                pow3_impl::<true>(f64::from_bits(bits)).to_bits(),
                current.to_bits()
            );
            previous = current;
        }
    }

    #[test]
    #[ignore = "Requires the accepted complete reciprocal witness and three retained Java Math fixtures"]
    fn portable_quantization_and_java_math_reference_are_exact() {
        use std::{
            fs::File,
            io::{BufRead, BufReader, Read},
        };

        const DOMAIN: u32 = 1 << 23;
        let path = std::env::var_os("BGR_RECIPROCAL_BINS_WITNESS")
            .expect("BGR_RECIPROCAL_BINS_WITNESS must name the accepted dense witness");
        let file = File::open(path).expect("Open accepted dense witness");
        assert_eq!(file.metadata().unwrap().len(), u64::from(DOMAIN) * 2);
        let mut input = BufReader::new(file);
        for physical_slot in 0..DOMAIN {
            let mut word = [0_u8; 2];
            input.read_exact(&mut word).unwrap();
            let bin = u16::from_le_bytes(word);
            assert!(bin <= 512, "Invalid witness bin at {physical_slot}");
            let packed = 0x3f80_0000 | physical_slot;
            let actual = quantized_reciprocal::<true>(packed);
            let recorded = 0x3f39_0000 + (u32::from(bin) << 14);
            let actual_index = ((actual & 16760832) >> 10) as i32 - 3648;
            let actual_multiplier = ((actual as u64) << 29) & 0xffff_f800_0000_0000;
            let recorded_index = i32::from(bin) * 16;
            let recorded_multiplier = (u64::from(recorded) << 29) & 0xffff_f800_0000_0000;
            assert_eq!(
                actual_index, recorded_index,
                "Witness index at {physical_slot}"
            );
            assert_eq!(
                actual_multiplier, recorded_multiplier,
                "Witness multiplier at {physical_slot}"
            );
            #[cfg(target_arch = "x86_64")]
            {
                let original = quantized_reciprocal::<false>(packed);
                let original_index = ((original & 16760832) >> 10) as i32 - 3648;
                let original_multiplier = ((original as u64) << 29) & 0xffff_f800_0000_0000;
                assert_eq!(
                    actual_index, original_index,
                    "Original index at {physical_slot}"
                );
                assert_eq!(
                    actual_multiplier, original_multiplier,
                    "Original multiplier at {physical_slot}"
                );
            }
        }
        let mut extra = [0_u8; 1];
        assert_eq!(input.read(&mut extra).unwrap(), 0);
        let mut rows = 0_usize;
        for (key, expected_rows) in [
            ("BGR_POW3_REFERENCE_1", 15202),
            ("BGR_POW3_REFERENCE_2", 152),
            ("BGR_POW3_REFERENCE_3", 346218),
        ] {
            let path = std::env::var_os(key)
                .unwrap_or_else(|| panic!("{key} must name the retained Java fixture"));
            let mut file_rows = 0_usize;
            for line in BufReader::new(File::open(path).unwrap()).lines() {
                let line = line.unwrap();
                let fields: Vec<_> = line.split('\t').collect();
                assert_eq!(fields.len(), 6);
                assert_eq!(fields[0], "P");
                let bits = u64::from_str_radix(fields[2], 16).unwrap();
                let expected_bits = u64::from_str_radix(fields[3], 16).unwrap();
                let expected = f64::from_bits(expected_bits);
                let portable = pow3_impl::<true>(f64::from_bits(bits));
                if expected.is_nan() {
                    assert!(portable.is_nan(), "{key} portable NaN row {file_rows}");
                } else {
                    assert_eq!(
                        portable.to_bits(),
                        expected_bits,
                        "{key} portable row {file_rows}"
                    );
                }
                let native = pow3(f64::from_bits(bits));
                if expected.is_nan() {
                    assert!(native.is_nan(), "{key} native NaN row {file_rows}");
                } else {
                    assert_eq!(
                        native.to_bits(),
                        expected_bits,
                        "{key} native row {file_rows}"
                    );
                }
                file_rows += 1;
            }
            assert_eq!(file_rows, expected_rows, "{key} complete row count");
            rows += file_rows;
        }
        assert_eq!(rows, 361572);
        let original_pairs = if cfg!(target_arch = "x86_64") {
            DOMAIN
        } else {
            0
        };
        println!(
            "PI5_PORTABLE_QUANTIZATION_PASS inputs={DOMAIN} consumer_comparisons={} original_x86_pairs={original_pairs} java_math_rows={rows} raw_bytes={}",
            u64::from(DOMAIN) * 2,
            u64::from(DOMAIN) * 2
        );
    }
}
