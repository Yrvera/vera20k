//! Deterministic PRNG used by simulation systems.
//!
//! Mirrors gamemd.exe `Random::RandomRanged`/scenario RNG shape for
//! gameplay-visible randomness: one auditable stream, 250-word XOR-lag state,
//! inclusive sorted ranged draws, and rejection sampling.

use std::hash::{Hash, Hasher};

const RNG_TABLE_LEN: usize = 250;
const RNG_INDEX_B_SEED: i32 = 0x67;
const INIT_TABLE_1: [u32; 4] = [0xBAA9_6887, 0x1E17_D32C, 0x03BC_DC3C, 0x0F33_D1B2];
const INIT_TABLE_2: [u32; 4] = [0x4B0F_3B58, 0xE874_F0C3, 0x6955_C5A6, 0x55A7_CA46];

/// Deterministic simulation RNG.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimRng {
    disabled: u8,
    index_a: i32,
    index_b: i32,
    state: Vec<u32>,
}

impl SimRng {
    /// Create a new RNG with the given seed.
    pub fn new(seed: u64) -> Self {
        let mut rng = Self {
            disabled: 0,
            index_a: 0,
            index_b: RNG_INDEX_B_SEED,
            state: vec![0; RNG_TABLE_LEN],
        };
        rng.reseed(seed as u32);
        rng
    }

    /// Compact deterministic fingerprint of the full internal state.
    ///
    /// This is for tests and debug comparisons. Use `hash_state` when feeding
    /// the authoritative world hash so every field contributes directly.
    pub fn state(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let mut mix = |value: u64| {
            for byte in value.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
            }
        };
        mix(u64::from(self.disabled));
        mix(self.index_a as u32 as u64);
        mix(self.index_b as u32 as u64);
        for &word in &self.state {
            mix(u64::from(word));
        }
        hash
    }

    /// Hash the complete RNG state for deterministic replay/desync checks.
    pub fn hash_state(&self, hasher: &mut impl Hasher) {
        self.disabled.hash(hasher);
        self.index_a.hash(hasher);
        self.index_b.hash(hasher);
        self.state.hash(hasher);
    }

    fn reseed(&mut self, seed: u32) {
        self.index_a = 0;
        self.index_b = RNG_INDEX_B_SEED;

        for (entry_index, slot) in self.state.iter_mut().enumerate() {
            let mut prev = seed;
            let mut mixed = entry_index as u32;
            for i in 0..INIT_TABLE_1.len() {
                let input = INIT_TABLE_1[i] ^ mixed;
                let high = (input as i32) >> 16;
                let low = input & 0xFFFF;
                let square_mix =
                    (!high.wrapping_mul(high)).wrapping_add((low as i32).wrapping_mul(low as i32));
                let rotated = ((square_mix >> 16) as u32) | ((square_mix as u32) << 16);
                let cross = (high as u32).wrapping_mul(low);
                let next = (rotated ^ INIT_TABLE_2[i]).wrapping_add(cross) ^ prev;
                prev = mixed;
                mixed = next;
            }
            *slot = mixed;
        }

        self.disabled = 0;
    }

    /// Advance and return next random u64.
    pub fn next_u64(&mut self) -> u64 {
        let lo = u64::from(self.next_u32());
        let hi = u64::from(self.next_u32());
        lo | (hi << 32)
    }

    /// Next random u32.
    pub fn next_u32(&mut self) -> u32 {
        if self.disabled != 0 {
            return 0;
        }

        let a = self.index_a as usize;
        let b = self.index_b as usize;
        let value = self.state[a] ^ self.state[b];
        self.state[a] = value;

        self.index_a += 1;
        if self.index_a >= RNG_TABLE_LEN as i32 {
            self.index_a = 0;
        }
        self.index_b += 1;
        if self.index_b >= RNG_TABLE_LEN as i32 {
            self.index_b = 0;
        }

        value
    }

    /// Random integer in [0, max_exclusive). Returns 0 for max_exclusive=0.
    pub fn next_range_u32(&mut self, max_exclusive: u32) -> u32 {
        if max_exclusive == 0 {
            return 0;
        }
        self.next_range_u32_inclusive(0, max_exclusive - 1)
    }

    /// Random integer in `[low, high]` inclusive on both ends.
    /// Sorts reversed bounds and consumes no draw when the bounds are equal.
    /// Mirrors binary `Random__RandomRanged(low, high)` for ordinary spans.
    pub fn next_range_u32_inclusive(&mut self, low: u32, high: u32) -> u32 {
        let (lo, hi) = if low <= high {
            (low, high)
        } else {
            (high, low)
        };
        if lo == hi {
            return lo;
        }

        let span = hi.wrapping_sub(lo);
        if span >= 0x7FFF_FFFF {
            return lo.wrapping_add(0x8000_0000);
        }

        let mask = span.next_power_of_two().wrapping_sub(1);
        loop {
            let sample = self.next_u32() & mask;
            if sample <= span {
                return lo.wrapping_add(sample);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SimRng;

    #[test]
    fn test_rng_repeatable_sequence() {
        let mut a = SimRng::new(12345);
        let mut b = SimRng::new(12345);
        for _ in 0..128 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn test_rng_range_bounds() {
        let mut rng = SimRng::new(1);
        for _ in 0..256 {
            let v = rng.next_range_u32(7);
            assert!(v < 7);
        }
    }

    #[test]
    fn test_inclusive_range_bounds() {
        let mut rng = SimRng::new(42);
        for _ in 0..256 {
            let v = rng.next_range_u32_inclusive(1, 5);
            assert!((1..=5).contains(&v));
        }
    }

    #[test]
    fn test_inclusive_range_degenerate() {
        let mut rng = SimRng::new(1);
        let before = rng.state();
        assert_eq!(rng.next_range_u32_inclusive(7, 7), 7);
        assert_eq!(rng.state(), before);

        let before = rng.state();
        let value = rng.next_range_u32_inclusive(7, 3);
        assert!((3..=7).contains(&value));
        assert_ne!(rng.state(), before);
    }

    #[test]
    fn test_exclusive_single_choice_consumes_no_draw() {
        let mut rng = SimRng::new(99);
        let before = rng.state();
        assert_eq!(rng.next_range_u32(1), 0);
        assert_eq!(rng.state(), before);
    }

    #[test]
    fn test_gamemd_raw_sequence_seed_one() {
        let mut rng = SimRng::new(1);
        assert_eq!(rng.next_u32(), 0x78B7_6ED5);
        assert_eq!(rng.next_u32(), 0x275D_74AE);
        assert_eq!(rng.next_u32(), 0xDA63_B931);
    }
}
