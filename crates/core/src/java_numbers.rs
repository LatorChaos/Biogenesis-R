// SPDX-License-Identifier: GPL-2.0-or-later

// Integer.parseInt uses Character.digit(char, 10), including BMP decimal-digit blocks
pub(crate) fn java_int(text: &str) -> Option<i32> {
    const ZEROS: [u32; 37] = [
        0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66,
        0xde6, 0xe50, 0xed0, 0xf20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80, 0x1a90,
        0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0, 0xaa50, 0xabf0,
        0xff10,
    ];
    let (negative, digits) = if let Some(rest) = text.strip_prefix('-') {
        (true, rest)
    } else {
        (false, text.strip_prefix('+').unwrap_or(text))
    };
    if digits.is_empty() {
        return None;
    }
    let mut magnitude = 0i64;
    for ch in digits.chars() {
        let code = ch as u32;
        let zero = ZEROS
            .iter()
            .find(|zero| (**zero..**zero + 10).contains(&code))?;
        magnitude = magnitude
            .checked_mul(10)?
            .checked_add(i64::from(code - zero))?;
        if magnitude > i64::from(i32::MAX) + i64::from(negative) {
            return None;
        }
    }
    Some(if negative { -magnitude } else { magnitude } as i32)
}

pub(crate) fn java_double(text: &str) -> Option<f64> {
    let text = text.trim_matches(|ch| ch <= '\u{20}');
    let (negative, unsigned) = if let Some(value) = text.strip_prefix('-') {
        (true, value)
    } else {
        (false, text.strip_prefix('+').unwrap_or(text))
    };
    if unsigned == "NaN" {
        return Some(f64::NAN);
    }
    if unsigned == "Infinity" {
        return Some(if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    let number = unsigned
        .strip_suffix(['d', 'D', 'f', 'F'])
        .unwrap_or(unsigned);
    if number.starts_with("0x") || number.starts_with("0X") {
        return hex_double(number, negative);
    }
    let mut before = 0;
    let mut after = 0;
    let mut cursor = 0;
    let bytes = number.as_bytes();
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        before += 1;
        cursor += 1;
    }
    if bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            after += 1;
            cursor += 1;
        }
    }
    if before + after == 0 {
        return None;
    }
    if matches!(bytes.get(cursor), Some(b'e' | b'E')) {
        cursor += 1;
        if matches!(bytes.get(cursor), Some(b'+' | b'-')) {
            cursor += 1;
        }
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if start == cursor {
            return None;
        }
    }
    if cursor != bytes.len() {
        return None;
    }
    let signed = if negative {
        format!("-{number}")
    } else {
        number.to_owned()
    };
    signed.parse().ok()
}

fn hex_double(text: &str, negative: bool) -> Option<f64> {
    let body = &text[2..];
    let exponent_at = body.find(['p', 'P'])?;
    let mantissa = &body[..exponent_at];
    let exponent = &body[exponent_at + 1..];
    let (minus, exponent) = if let Some(value) = exponent.strip_prefix('-') {
        (true, value)
    } else {
        (false, exponent.strip_prefix('+').unwrap_or(exponent))
    };
    if exponent.is_empty() || !exponent.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut power = 0i128;
    for byte in exponent.bytes() {
        power = power
            .saturating_mul(10)
            .saturating_add(i128::from(byte - b'0'));
    }
    if minus {
        power = -power;
    }
    let mut digits = Vec::new();
    let mut point = false;
    let mut fractional = 0i128;
    for ch in mantissa.chars() {
        if ch == '.' && !point {
            point = true;
            continue;
        }
        let digit = ch.to_digit(16).filter(|_| ch.is_ascii_hexdigit())? as u8;
        digits.push(digit);
        if point {
            fractional += 1;
        }
    }
    if digits.is_empty() {
        return None;
    }
    let sign = u64::from(negative) << 63;
    let first = match digits.iter().position(|digit| *digit != 0) {
        Some(first) => first,
        None => return Some(f64::from_bits(sign)),
    };
    let digits = &digits[first..];
    let length = digits.len() as i128 * 4 - i128::from(digits[0].leading_zeros() - 4);
    // Cancel the mantissa scale before testing the binary64 exponent range
    let scale = power.saturating_sub(4 * fractional);
    let mut top = (length - 1).saturating_add(scale);
    if top > 1023 {
        return Some(f64::from_bits(sign | 0x7ff0_0000_0000_0000));
    }
    if top < -1075 {
        return Some(f64::from_bits(sign));
    }
    let bit = |index: i128| -> bool {
        index >= 0
            && index < length
            && (digits[digits.len() - 1 - (index / 4) as usize] >> (index % 4) as u32) & 1 != 0
    };
    let shift = if top >= -1022 {
        length - 53
    } else {
        -scale - 1074
    };
    let mut significand = 0u64;
    for index in (shift.max(0)..length).rev() {
        significand = (significand << 1) | u64::from(bit(index));
    }
    if shift < 0 {
        significand <<= -shift as u32;
    }
    if shift > 0 {
        let guard = bit(shift - 1);
        let sticky = (0..(shift - 1).min(length)).any(bit);
        if guard && (sticky || significand & 1 != 0) {
            significand += 1;
        }
    }
    if top < -1022 {
        return Some(f64::from_bits(sign | significand));
    }
    if significand == 1 << 53 {
        significand >>= 1;
        top += 1;
    }
    if top > 1023 {
        return Some(f64::from_bits(sign | 0x7ff0_0000_0000_0000));
    }
    Some(f64::from_bits(
        sign | ((top + 1023) as u64) << 52 | (significand - (1 << 52)),
    ))
}

pub(crate) fn double_text(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-Infinity"
        } else {
            "Infinity"
        }
        .to_owned();
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .to_owned();
    }
    if (0.001..10_000_000.0).contains(&value.abs()) {
        let mut text = value.to_string();
        if !text.contains('.') {
            text.push_str(".0");
        }
        return text;
    }
    let text = format!("{value:e}");
    let (coefficient, exponent) = text.split_once('e').unwrap();
    let coefficient = if coefficient.contains('.') {
        coefficient.to_owned()
    } else {
        format!("{coefficient}.0")
    };
    format!("{coefficient}E{}", exponent.parse::<i32>().unwrap())
}

#[cfg(test)]
mod tests {
    use super::java_double;

    #[test]
    fn hexadecimal_scale_cancels_before_ieee_range_checks() {
        let controls: Vec<_> = include_str!("../tests/fixtures/java_hex_cases.tsv")
            .lines()
            .collect();
        let observed: Vec<_> = include_str!("../tests/fixtures/java_hex_jdk.tsv")
            .lines()
            .collect();
        assert_eq!(controls.len(), 12);
        assert_eq!(observed.len(), controls.len());
        let mut differences = Vec::new();
        for (control, observed) in controls.iter().zip(observed) {
            let control: Vec<_> = control.split('\t').collect();
            let observed: Vec<_> = observed.split('\t').collect();
            assert_eq!(control.len(), 6);
            assert_eq!(observed.len(), 5);
            assert_eq!(observed[0], "D");
            assert_eq!(observed[1], control[0]);
            let zeros = "0".repeat(control[2].parse().unwrap());
            let mantissa = match control[1] {
                "fraction" => format!("0.{zeros}{}", control[3]),
                "integer" => format!("{}{zeros}", control[3]),
                _ => panic!("Unknown controlled mantissa"),
            };
            let sign = if control[5].parse::<bool>().unwrap() {
                "-"
            } else {
                ""
            };
            let input = format!("{sign}0x{mantissa}p{}", control[4]);
            assert_eq!(input.len(), observed[2].parse::<usize>().unwrap());
            let expected = u64::from_str_radix(observed[4], 16).unwrap();
            let actual = java_double(&input).unwrap().to_bits();
            if actual != expected {
                differences.push(format!("{}:{actual:016x}!={expected:016x}", control[0]));
            }
        }
        assert!(
            differences.is_empty(),
            "Actual JDK21 differences: {differences:?}"
        );
    }
}
