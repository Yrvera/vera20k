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
}
