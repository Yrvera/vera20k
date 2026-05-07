//! Deterministic PRNG used by simulation systems.
//!
//! Single-stream xorshift64* implementation with explicit ownership.
//! This keeps replay and lockstep call order auditable.

/// Deterministic simulation RNG.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimRng {
    state: u64,
}

impl SimRng {
    /// Create a new RNG with the given seed.
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        };
        Self { state }
    }

    /// Current internal seed/state.
    pub fn state(&self) -> u64 {
        self.state
    }

    /// Advance and return next random u64.
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64* (Marsaglia/Vigna).
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Next random u32.
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Random integer in [0, max_exclusive). Returns 0 for max_exclusive=0.
    pub fn next_range_u32(&mut self, max_exclusive: u32) -> u32 {
        if max_exclusive == 0 {
            return 0;
        }
        // Modulo is acceptable here because gameplay logic currently doesn't rely on exact distribution.
        self.next_u32() % max_exclusive
    }

    /// Random integer in `[low, high]` inclusive on both ends.
    /// Returns `low` when `high <= low`. Saturating widen handles the
    /// pathological `high == u32::MAX, low == 0` case without overflow.
    ///
    /// Mirrors binary `Random__RandomRanged(low, high)` calling convention.
    pub fn next_range_u32_inclusive(&mut self, low: u32, high: u32) -> u32 {
        if high <= low {
            return low;
        }
        let span = (high as u64) - (low as u64) + 1;
        let span_u32 = span.min(u32::MAX as u64) as u32;
        low.saturating_add(self.next_range_u32(span_u32))
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
        // low == high → always returns low, no draw.
        assert_eq!(rng.next_range_u32_inclusive(7, 7), 7);
        // high < low → returns low.
        assert_eq!(rng.next_range_u32_inclusive(7, 3), 7);
    }
}
