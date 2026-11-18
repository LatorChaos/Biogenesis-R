/*
 * Copyright (c) 1996, 2023, Oracle and/or its affiliates. All rights reserved.
 * DO NOT ALTER OR REMOVE COPYRIGHT NOTICES OR THIS FILE HEADER.
 *
 * This code is free software; you can redistribute it and/or modify it
 * under the terms of the GNU General Public License version 2 only, as
 * published by the Free Software Foundation.  Oracle designates this
 * particular file as subject to the "Classpath" exception as provided
 * by Oracle in the LICENSE file that accompanied this code.
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
 */
// SPDX-License-Identifier: GPL-2.0-only WITH Classpath-exception-2.0

use std::sync::OnceLock;

#[path = "number_profiles.rs"]
mod profiles;

pub(super) struct NumberFormat {
    zero: char,
    decimal: char,
    group: Option<char>,
    positive_prefix: &'static str,
    positive_suffix: &'static str,
    negative_prefix: &'static str,
    negative_suffix: &'static str,
    nan: &'static str,
    infinity: &'static str,
}

pub(super) fn for_tag(tag: &str) -> &'static NumberFormat {
    let mut candidate = tag;
    loop {
        if let Ok(index) = profiles::TAGS.binary_search_by_key(&candidate, |&(name, _)| name) {
            return &profiles::PROFILES[profiles::TAGS[index].1];
        }
        match candidate.rsplit_once('-') {
            Some((parent, _)) => candidate = parent,
            None => return &profiles::PROFILES[profiles::ROOT],
        }
    }
}

fn process_format() -> &'static NumberFormat {
    static FORMAT: OnceLock<&'static NumberFormat> = OnceLock::new();
    FORMAT.get_or_init(|| for_tag(&process_tag()))
}

pub(super) fn integer(value: i64) -> String {
    process_format().integer(value)
}

pub(super) fn real(value: f64) -> String {
    process_format().real(value)
}

impl NumberFormat {
    pub(super) fn integer(&self, value: i64) -> String {
        self.decorate(value.is_negative(), &value.unsigned_abs().to_string())
    }

    pub(super) fn real(&self, value: f64) -> String {
        if value.is_nan() {
            return self.nan.to_owned();
        }
        if value.is_infinite() {
            return self.affixes(value.is_sign_negative(), self.infinity);
        }
        let text = if value.abs() >= 288_230_376_151_711_744.0 {
            large_integer(value.abs())
        } else {
            format!("{:.1}", value.abs())
        };
        let text = text.strip_suffix(".0").unwrap_or(&text);
        self.decorate(value.is_sign_negative(), text)
    }

    fn affixes(&self, negative: bool, digits: &str) -> String {
        let (prefix, suffix) = if negative {
            (self.negative_prefix, self.negative_suffix)
        } else {
            (self.positive_prefix, self.positive_suffix)
        };
        format!("{prefix}{digits}{suffix}")
    }

    fn decorate(&self, negative: bool, text: &str) -> String {
        let (integer, fraction) = text.split_once('.').unwrap_or((text, ""));
        let mut digits = String::with_capacity(text.len() + text.len() / 3);
        for (index, byte) in integer.bytes().enumerate() {
            if index > 0 && (integer.len() - index) % 3 == 0 {
                if let Some(group) = self.group {
                    digits.push(group);
                }
            }
            digits.push(char::from_u32(self.zero as u32 + u32::from(byte - b'0')).unwrap());
        }
        if !fraction.is_empty() {
            digits.push(self.decimal);
            for byte in fraction.bytes() {
                digits.push(char::from_u32(self.zero as u32 + u32::from(byte - b'0')).unwrap());
            }
        }
        self.affixes(negative, &digits)
    }
}

// DecimalFormat retains FloatingDecimal's integer and interval conversion rules
fn large_integer(value: f64) -> String {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023;
    if exponent <= 62 {
        let insignificant = (1_u64 << (exponent - 54)).ilog10();
        let unit = 10_u64.pow(insignificant);
        let integer = value as u64;
        let rounded = integer / unit + u64::from(integer % unit >= unit / 2);
        return format!("{rounded}{}", "0".repeat(insignificant as usize));
    }

    let fraction = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    let significant = 53 - fraction.trailing_zeros() as i32;
    let normalized = f64::from_bits((1023_u64 << 52) | (bits & ((1_u64 << 52) - 1)));
    let estimate = ((normalized - 1.5) * 0.289529654 + 0.176091259
        + f64::from(exponent) * 0.301029995663981).floor() as i32;
    let mut b2 = exponent - significant + 1;
    let mut s2 = estimate;
    let common = b2.min(s2);
    b2 -= common;
    s2 -= common;
    let mut m2 = exponent - 53 - common - i32::from(significant == 1);
    if m2 < 0 {
        b2 -= m2;
        s2 -= m2;
        m2 = 0;
    }
    let power_bits = if estimate + 1 < 27 {
        64 - 5_u64.pow((estimate + 1) as u32).leading_zeros() as i32
    } else {
        (estimate + 1) * 3
    };
    if significant + b2 < 64 && s2 + 1 + power_bits < 64 {
        let mut b = ((fraction >> fraction.trailing_zeros()) as i64) << b2;
        let s = (5_i64.pow(estimate as u32)) << s2;
        let mut m = 1_i64 << m2;
        let tens = s * 10;
        let mut digits = Vec::new();
        let mut decimal_at = estimate + 1;
        let (low, high) = loop {
            let digit = b / s;
            b = (b % s) * 10;
            m = m.wrapping_mul(10);
            let (low, high) = if m > 0 {
                (b < m, b.wrapping_add(m) > tens)
            } else {
                (true, true)
            };
            assert!((0..10).contains(&digit));
            if digits.is_empty() && digit == 0 && !high {
                decimal_at -= 1;
                continue;
            }
            digits.push(b'0' + digit as u8);
            if digits.len() >= 2 && (low || high) {
                break (low, high);
            }
            assert!(digits.len() < 20);
        };
        let difference = b.wrapping_shl(1).wrapping_sub(tens);
        if high && (!low || difference > 0 || (difference == 0 && digits.last().unwrap() & 1 != 0)) {
            let previous = digits.len();
            increment(&mut digits);
            decimal_at += i32::from(digits.len() > previous);
        }
        let mut result = String::from_utf8(digits).unwrap();
        result.push_str(&"0".repeat((decimal_at as usize).saturating_sub(result.len())));
        return result;
    }

    let integer = format!("{value:.0}");
    let margin_exponent = exponent - 53 - i32::from(significant == 1);
    let margin = format!("{:.0}", f64::from_bits(((margin_exponent + 1023) as u64) << 52));
    for count in 2..=integer.len() {
        let remainder = &integer[count..];
        let distance = complement(remainder);
        let low = decimal_cmp(remainder, &margin).is_lt();
        let high = !decimal_cmp(&distance, &margin).is_gt();
        if low || high {
            let mut digits = integer.as_bytes()[..count].to_vec();
            let middle = decimal_cmp(remainder, &distance);
            if high && (!low || middle.is_gt() || (middle.is_eq() && digits.last().unwrap() & 1 != 0)) {
                increment(&mut digits);
            }
            let mut result = String::from_utf8(digits).unwrap();
            result.push_str(&"0".repeat(integer.len() - count));
            return result;
        }
    }
    unreachable!("Exact integer remainder must enter the rounding interval")
}

fn increment(digits: &mut Vec<u8>) {
    for digit in digits.iter_mut().rev() {
        if *digit != b'9' {
            *digit += 1;
            return;
        }
        *digit = b'0';
    }
    digits.insert(0, b'1');
}

fn decimal_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let left = left.trim_start_matches('0');
    let right = right.trim_start_matches('0');
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

// Return 10^n minus an n-digit remainder without introducing a big-integer dependency
fn complement(remainder: &str) -> String {
    let mut digits = remainder.as_bytes().to_vec();
    let mut carry = 1;
    for digit in digits.iter_mut().rev() {
        let value = 9 - (*digit - b'0') + carry;
        *digit = b'0' + value % 10;
        carry = value / 10;
    }
    if carry != 0 {
        digits.insert(0, b'1');
    }
    String::from_utf8(digits).unwrap()
}

// OpenJDK reads LC_CTYPE after one atomic LC_ALL initialization on GNU/Linux
#[cfg(all(target_os = "linux", target_env = "gnu"))]
pub(super) fn process_tag() -> String {
    use std::ffi::{CStr, c_char, c_int, c_void};
    unsafe extern "C" {
        fn newlocale(mask: c_int, locale: *const c_char, base: *mut c_void) -> *mut c_void;
        fn nl_langinfo_l(item: c_int, locale: *mut c_void) -> *mut c_char;
        fn freelocale(locale: *mut c_void);
    }
    // A private locale object preserves Godot's process locale and numeric parsing
    let name = unsafe {
        let locale = newlocale(0x1fbf, c"".as_ptr(), std::ptr::null_mut());
        if locale.is_null() {
            return "en-US".to_owned();
        }
        let pointer = nl_langinfo_l(0xffff, locale);
        assert!(!pointer.is_null(), "GNU locale name must be present");
        let name = CStr::from_ptr(pointer).to_string_lossy().into_owned();
        freelocale(locale);
        name
    };
    posix_tag(&name)
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
pub(super) fn process_tag() -> String {
    // shortcut: GNU/Linux is qualified; verify locale initialization before qualifying another platform
    posix_tag(&std::env::var("LC_ALL").ok().filter(|name| !name.is_empty())
        .or_else(|| std::env::var("LC_CTYPE").ok().filter(|name| !name.is_empty()))
        .or_else(|| std::env::var("LANG").ok()).unwrap_or_else(|| "C".to_owned()))
}

pub(super) fn posix_tag(name: &str) -> String {
    if name == "C" || name == "POSIX" {
        return "en-US".to_owned();
    }
    let (name, modifier) = name.split_once('@').unwrap_or((name, ""));
    let name = name.split('.').next().unwrap();
    let name = profiles::ALIASES.iter().find_map(|&(from, to)| (from == name).then_some(to)).unwrap_or(name);
    let (language, region) = name.split_once('_').unwrap_or((name, ""));
    let language = profiles::LANGUAGES.iter().find_map(|&(from, to)| (from == language).then_some(to)).unwrap_or(language);
    let language = match language {
        "iw" => "he", "in" => "id", "ji" => "yi",
        _ => language,
    };
    let region = match region { "RN" => "US", "YU" => "CS", _ => region };
    if language == "no" && region == "NO" && modifier == "nynorsk" {
        return "nn-NO".to_owned();
    }
    let script = match modifier {
        "cyrillic" | "Cyrl" => "Cyrl", "devanagari" | "Deva" => "Deva",
        "iqtelif" | "latin" | "Latn" => "Latn",
        "Arab" | "Ethi" | "Hans" | "Hant" | "Sund" | "Syrc" | "Tfng" => modifier,
        _ => "",
    };
    [language, script, region].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_number_profiles_preserve_integer_and_double_output() {
        let mut cases = 0;
        for line in include_str!("../tests/fixtures/number_profiles.tsv").lines() {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 4);
            let profile = for_tag(fields[0]);
            let actual = match fields[1] {
                "integer" => profile.integer(fields[2].parse().unwrap()),
                "double" => profile.real(f64::from_bits(u64::from_str_radix(fields[2], 16).unwrap())),
                _ => panic!("Unexpected captured number kind"),
            };
            assert_eq!(actual, fields[3], "{} {} {}", fields[0], fields[1], fields[2]);
            cases += 1;
        }
        assert_eq!(cases, 1519);
    }

    #[test]
    fn posix_names_preserve_source_aliases_and_ignore_encoding() {
        for (name, expected) in [
            ("C", "en-US"), ("C.UTF-8", "en"), ("POSIX", "en-US"),
            ("de_DE.UTF-8", "de-DE"), ("de", "de-DE"),
            ("pt", "pt-PT"), ("sr_RS.UTF-8@latin", "sr-Latn-RS"),
            ("sr_YU.UTF-8@cyrillic", "sr-Cyrl-CS"),
            ("no_NO.UTF-8@nynorsk", "nn-NO"), ("iw_IL.UTF-8", "he-IL"),
            ("german", "de-DE"), ("de_DE.UTF-8@euro", "de-DE"),
        ] {
            assert_eq!(posix_tag(name), expected);
        }
    }
}
