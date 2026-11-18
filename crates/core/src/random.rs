// SPDX-License-Identifier: GPL-2.0-or-later

use std::fmt;

/// The 48-bit generator and draw ordering used by java.util.Random
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaRandom {
    state: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RandomError;

impl fmt::Display for RandomError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Random bounds must be positive Java integers")
    }
}

impl std::error::Error for RandomError {}

impl JavaRandom {
    #[cfg(test)]
    pub(crate) fn state48(&self) -> u64 {
        self.state
    }

    const MULTIPLIER: u64 = 0x5deece66d;
    const MASK: u64 = (1 << 48) - 1;

    pub fn new(seed: i64) -> Self {
        Self {
            state: (seed as u64 ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    fn bits(&mut self, count: u32) -> u32 {
        self.state = self.state.wrapping_mul(Self::MULTIPLIER).wrapping_add(11) & Self::MASK;
        (self.state >> (48 - count)) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.bits(32) as i32
    }

    /// Reject nonpositive bounds without consuming the stream
    pub fn next_bounded(&mut self, bound: i32) -> Result<i32, RandomError> {
        if bound <= 0 {
            return Err(RandomError);
        }
        Ok(self.bounded(bound))
    }

    pub(crate) fn bounded(&mut self, bound: i32) -> i32 {
        debug_assert!(bound > 0);
        if bound & -bound == bound {
            return ((bound as i64 * self.bits(31) as i64) >> 31) as i32;
        }
        loop {
            let bits = self.bits(31) as i32;
            let value = bits % bound;
            if bits.wrapping_sub(value).wrapping_add(bound - 1) >= 0 {
                return value;
            }
        }
    }

    pub fn next_f64(&mut self) -> f64 {
        let high = self.bits(26) as u64;
        let low = self.bits(27) as u64;
        ((high << 27) + low) as f64 / (1_u64 << 53) as f64
    }

    pub fn next_bool(&mut self) -> bool {
        self.bits(1) != 0
    }

    pub fn next_i64(&mut self) -> i64 {
        // Java sign-extends both int draws before addition
        ((self.next_i32() as i64) << 32).wrapping_add(self.next_i32() as i64)
    }
}
