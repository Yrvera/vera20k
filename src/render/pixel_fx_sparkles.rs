//! Per-frame water/ore sparkle render — observable parity with gamemd.exe's
//! DrawPixelFXSparkles. See ra2-rust-game-docs/PIXEL_FX_SPARKLES_GHIDRA_REPORT.md
//! for the full reverse-engineering and the design doc at
//! docs/plans/2026-05-18-pixel-fx-sparkle-design.md.
//!
//! Stateless / hash-derived: each visible water or ore cell, every frame,
//! hashes (cell_coord, cycle_index) to derive sub-pixel position, peak
//! colour noise, lerp speed, and timer-init for the current cycle, then
//! computes the sparkle's current RGB analytically. No per-cell persistent
//! state.
//!
//! ## Dependency rules
//! - Part of render/ — reads sim/ state through immutable references only.
//!   No writes to sim. No coupling to GPU types beyond SpriteInstance.

use crate::render::batch::SpriteInstance;

/// Per-species sparkle parameters mirroring gamemd's
/// g_PixelFXParams_Water (0x008367C8) and g_PixelFXParams_Ore (0x008367F0)
/// tables. Read directly from the binary; see report §5.2.
#[derive(Debug, Clone, Copy)]
struct SparkleParams {
    /// Dim endpoint of the lerp. Applied with weight (0x1000 - lerp).
    base_rgb: [u8; 3],
    /// Bright endpoint of the lerp. Applied with weight lerp; per cycle, each
    /// channel may be reduced by `0..(1 << color_noise_bits)`.
    peak_rgb: [u8; 3],
    /// Per-channel noise bit count subtracted from peak. 0 = no noise (ore).
    color_noise_bits: u8,
    /// Inclusive lower bound for the per-cell-per-cycle LerpSpeed (phase / ms).
    lerp_speed_min: u32,
    /// Inclusive upper bound for the per-cell-per-cycle LerpSpeed.
    lerp_speed_max: u32,
}

/// Water sparkle constants — verified by direct memory read at
/// gamemd.exe 0x008367C8. See report §5.2. (L1, L2, L3, L4)
const WATER: SparkleParams = SparkleParams {
    base_rgb: [40, 40, 80],
    peak_rgb: [158, 158, 224],
    color_noise_bits: 5,
    lerp_speed_min: 3,
    lerp_speed_max: 12,
};

/// Ore sparkle constants — verified by direct memory read at
/// gamemd.exe 0x008367F0. See report §5.2. (L6, L7, L8, L9)
const ORE: SparkleParams = SparkleParams {
    base_rgb: [176, 144, 0],
    peak_rgb: [255, 255, 240],
    color_noise_bits: 0,
    lerp_speed_min: 15,
    lerp_speed_max: 30,
};

/// Average cycle length for the stateless cycle-bucket approximation.
/// gamemd's per-cycle duration is (timer_init 0..4095 ms) + (active
/// 0x2000/lerp_speed ms). Avg ≈ 2048 + 430 = 2478 ms. Round to 2500 for
/// both species (coincidentally similar). See design doc §Cycle bucketing.
const WATER_CYCLE_BUCKET_MS: u64 = 2500;
const ORE_CYCLE_BUCKET_MS: u64 = 2500;

/// Splitmix64 — Vigna's PRNG, used here as a one-shot 64→64 bit hash.
/// Three operations: add, xor-shift-multiply (×2). Well-distributed; avalanche
/// quality is more than enough for "looks random per pixel."
#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Pack cell coordinates into a 64-bit key for hashing. Layout puts rx in
/// the high 16 bits of the upper 32, ry in the high 16 bits of the lower 32,
/// leaving the low 32 bits as a 0 sentinel that the caller can XOR with
/// cycle_index when mixing per-cycle entropy.
#[inline]
fn coord_key(rx: u16, ry: u16) -> u64 {
    ((rx as u64) << 32) | ((ry as u64) << 16)
}

/// Ping-pong lerp between base (dim) and peak (bright) colors.
///
/// Phase is the position within a cycle, domain [0, 0x2000). The cycle is
/// symmetric: phase [0, 0x1000) rises from base to peak; phase [0x1000,
/// 0x2000) falls from peak back to base. (L13, L14, L16)
///
/// Per-channel formula (L15):
///     current = (base * (0x1000 - lerp) + peak * lerp) >> 12
///
/// where `lerp = phase & 0xFFF`, optionally flipped if bit 0x1000 is set.
#[inline]
fn ping_pong_lerp(phase: u32, base: [u8; 3], peak: [u8; 3]) -> [u8; 3] {
    let mut lerp = phase & 0xFFF;
    if (phase & 0x1000) != 0 {
        lerp = 0x1000 - lerp;
    }
    let inv = 0x1000 - lerp;
    let blend = |b: u8, p: u8| -> u8 {
        // (base * inv + peak * lerp) >> 12. Use u32 to avoid overflow
        // (255 * 0x1000 = 1,044,480, fits in u32 easily).
        (((b as u32) * inv + (p as u32) * lerp) >> 12) as u8
    };
    [blend(base[0], peak[0]), blend(base[1], peak[1]), blend(base[2], peak[2])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_constants_match_report() {
        // Lock L1, L2, L3, L4 against the report. Any tuning would have to
        // change both the report (and the underlying binary memory!) and
        // this test in tandem.
        assert_eq!(WATER.base_rgb, [40, 40, 80]);
        assert_eq!(WATER.peak_rgb, [158, 158, 224]);
        assert_eq!(WATER.color_noise_bits, 5);
        assert_eq!(WATER.lerp_speed_min, 3);
        assert_eq!(WATER.lerp_speed_max, 12);
    }

    #[test]
    fn ore_constants_match_report() {
        // Lock L6, L7, L8, L9 against the report.
        assert_eq!(ORE.base_rgb, [176, 144, 0]);
        assert_eq!(ORE.peak_rgb, [255, 255, 240]);
        assert_eq!(ORE.color_noise_bits, 0);
        assert_eq!(ORE.lerp_speed_min, 15);
        assert_eq!(ORE.lerp_speed_max, 30);
    }

    #[test]
    fn cycle_buckets_are_positive_and_documented() {
        // Sanity: buckets must be non-zero (division by zero in cycle math).
        // The actual 2500ms value is an approximation choice; this test
        // documents that we picked it deliberately.
        assert_eq!(WATER_CYCLE_BUCKET_MS, 2500);
        assert_eq!(ORE_CYCLE_BUCKET_MS, 2500);
    }

    #[test]
    fn splitmix64_is_deterministic() {
        // Same input always yields same output (necessary for replay
        // determinism). Spot-check a handful of inputs.
        assert_eq!(splitmix64(0), splitmix64(0));
        assert_eq!(splitmix64(0xDEAD_BEEF), splitmix64(0xDEAD_BEEF));
        assert_eq!(splitmix64(u64::MAX), splitmix64(u64::MAX));
    }

    #[test]
    fn splitmix64_distributes_low_bits() {
        // For 1000 consecutive inputs, the low byte of the output should
        // span at least 200 distinct values out of 256. Catches a hash
        // that's stuck on a small subset.
        let mut seen = std::collections::HashSet::new();
        for i in 0u64..1000 {
            seen.insert(splitmix64(i) & 0xFF);
        }
        assert!(seen.len() >= 200, "splitmix64 low-byte spread too small: {}", seen.len());
    }

    #[test]
    fn coord_key_is_injective_for_typical_map() {
        // Two adjacent cells must produce different keys (else the cell
        // offset would not break beat-sync per L26).
        assert_ne!(coord_key(10, 10), coord_key(11, 10));
        assert_ne!(coord_key(10, 10), coord_key(10, 11));
        assert_ne!(coord_key(0, 0), coord_key(0, 1));
    }

    #[test]
    fn coord_key_with_cycle_xor_breaks_per_cycle() {
        // Hashing (coord_key XOR cycle_index) — different cycles must yield
        // different splitmix64 outputs for the same cell (else L24
        // re-randomization would not happen).
        let key = coord_key(50, 50);
        let s0 = splitmix64(key ^ 0);
        let s1 = splitmix64(key ^ 1);
        let s2 = splitmix64(key ^ 2);
        assert_ne!(s0, s1);
        assert_ne!(s1, s2);
        assert_ne!(s0, s2);
    }

    #[test]
    fn lerp_at_phase_0_is_base() {
        // L16: phase 0 → base color (sparkle just spawned, dim).
        // L23: cells START dim each cycle.
        let result = ping_pong_lerp(0, [40, 40, 80], [158, 158, 224]);
        assert_eq!(result, [40, 40, 80]);
    }

    #[test]
    fn lerp_at_phase_0x1000_is_peak() {
        // L16: phase 0x1000 → peak color (sparkle at brightest).
        let result = ping_pong_lerp(0x1000, [40, 40, 80], [158, 158, 224]);
        assert_eq!(result, [158, 158, 224]);
    }

    #[test]
    fn lerp_at_phase_0x1FFF_is_near_base() {
        // L16: phase 0x1FFF → near base (one step before re-init).
        // With lerp = 0xFFF flipped via bit 0x1000, the inv weight is 0xFFF
        // and the lerp weight is 1 — overwhelmingly base.
        let result = ping_pong_lerp(0x1FFF, [40, 40, 80], [158, 158, 224]);
        // (40 * 0xFFF + 158 * 1) >> 12 = (163800 + 158) >> 12 = 164158 >> 12 = 40
        assert_eq!(result, [40, 40, 80]);
    }

    #[test]
    fn lerp_ping_pong_symmetry() {
        // L14: phase (0x1000 - x) and (0x1000 + x) must yield same color
        // for any x in 1..0x1000. This is the ping-pong invariant.
        let base = [40, 40, 80];
        let peak = [158, 158, 224];
        for x in [1u32, 100, 0x400, 0x800, 0xFFF] {
            let rising = ping_pong_lerp(0x1000 - x, base, peak);
            let falling = ping_pong_lerp(0x1000 + x, base, peak);
            assert_eq!(rising, falling, "asymmetry at x={:#x}", x);
        }
    }

    #[test]
    fn lerp_monotonic_rise_first_half() {
        // Phase 0 → 0x1000 should produce monotonically rising R channel
        // (since peak.R > base.R for water). Catches a flipped formula.
        let base = [40, 40, 80];
        let peak = [158, 158, 224];
        let mut prev_r = 0u8;
        for phase in (0..=0x1000).step_by(0x100) {
            let rgb = ping_pong_lerp(phase, base, peak);
            assert!(rgb[0] >= prev_r, "R not monotonic at phase {:#x}: {} < {}", phase, rgb[0], prev_r);
            prev_r = rgb[0];
        }
    }
}
