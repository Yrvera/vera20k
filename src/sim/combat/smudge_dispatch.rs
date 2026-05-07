//! Smudge spawn dispatcher — fired from combat tick at explosion emission and
//! at building destruction. Mirrors AnimClass::Start, BuildingClass::DestructionEffects,
//! and BuildingClass::SpawnSurvivors smudge logic from gamemd.exe.
//!
//! Dependency rules: depends on rules/, map/, sim/. Never render/ui/audio/net.

use std::sync::OnceLock;

use crate::sim::rng::SimRng;
use crate::sim::smudge_grid::SimCoord;

/// 256-entry unit-vector lookup table in Q16.16 fixed-point.
/// Each entry is `(sin(angle) * 65536, -cos(angle) * 65536)` rounded to i32,
/// where `angle = (i16(byte << 8) - 0x3FFF) * (-pi / 32768)`.
///
/// Built once at first use; deterministic across machines because it's
/// computed from constants and frozen as i32.
fn unit_vec_table() -> &'static [(i32, i32); 256] {
    static TABLE: OnceLock<[(i32, i32); 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = [(0i32, 0i32); 256];
        for b in 0u32..256 {
            let raw = ((b << 8) as i16) as i32 - 0x3FFF;
            let angle = raw as f64 * (-std::f64::consts::PI / 32768.0);
            let sin_q16 = (angle.sin() * 65536.0).round() as i32;
            let neg_cos_q16 = (-(angle.cos()) * 65536.0).round() as i32;
            t[b as usize] = (sin_q16, neg_cos_q16);
        }
        t
    })
}

/// Returns a (dx, dy) lepton offset at the given magnitude using one byte
/// of RNG state. Z is unaffected.
///
/// Mirrors `FUN_0049F420(magnitude, flag=0)` from gamemd.exe.
pub(crate) fn random_offset_at_radius(rng: &mut SimRng, magnitude_leptons: i32) -> (i32, i32) {
    let b: u8 = (rng.next_u32() & 0xFF) as u8;
    let (sin_q16, neg_cos_q16) = unit_vec_table()[b as usize];
    let dx = ((sin_q16 as i64) * (magnitude_leptons as i64)) >> 16;
    let dy = ((neg_cos_q16 as i64) * (magnitude_leptons as i64)) >> 16;
    (dx as i32, dy as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: i32, b: i32, tol: i32) -> bool { (a - b).abs() <= tol }

    #[test]
    fn unit_vec_table_byte_0_matches_reference() {
        // byte=0: raw = 0 - 0x3FFF = -16383; angle = -16383 * -pi/32768 ≈ 1.5708 (~pi/2)
        // sin(pi/2) ≈ 1.0, -cos(pi/2) ≈ 0.0
        let (sin_q16, neg_cos_q16) = unit_vec_table()[0];
        // sin*65536 ≈ 65536, -cos*65536 ≈ 0 (within rounding)
        assert!(approx_eq(sin_q16, 65536, 50), "sin_q16 = {}", sin_q16);
        assert!(approx_eq(neg_cos_q16, 0, 50), "neg_cos_q16 = {}", neg_cos_q16);
    }

    #[test]
    fn unit_vec_table_byte_64_quarter_turn() {
        // byte=64: (64<<8)=0x4000=16384; raw = 16384 - 0x3FFF = 1
        // angle ≈ -pi/32768 ≈ -0.0000958
        // sin(angle) ≈ -0.0000958, -cos(angle) ≈ -1.0
        let (sin_q16, neg_cos_q16) = unit_vec_table()[64];
        assert!(approx_eq(sin_q16, 0, 50), "sin_q16 = {}", sin_q16);
        assert!(approx_eq(neg_cos_q16, -65536, 50), "neg_cos_q16 = {}", neg_cos_q16);
    }

    #[test]
    fn random_offset_consumes_exactly_one_u32_advance() {
        // Two RNGs at the same seed: one advances via random_offset_at_radius,
        // the other advances via a single direct next_u32 call. After both
        // operations, internal state must match — confirming exactly one
        // RNG step was consumed.
        let mut rng_a = SimRng::new(42);
        let mut rng_b = SimRng::new(42);
        let _ = random_offset_at_radius(&mut rng_a, 0x80);
        let _ = rng_b.next_u32();
        assert_eq!(rng_a.state(), rng_b.state());
    }

    #[test]
    fn random_offset_per_axis_bounded() {
        let mut rng = SimRng::new(7);
        for _ in 0..100 {
            let (dx, dy) = random_offset_at_radius(&mut rng, 0x80);
            // Per-axis bound: each component is sin/cos*magnitude in Q16.16,
            // so |dx|, |dy| ≤ magnitude (+1 lepton tolerance for rounding).
            assert!(dx.abs() <= 0x80 + 1, "dx={} out of bound", dx);
            assert!(dy.abs() <= 0x80 + 1, "dy={} out of bound", dy);
        }
    }
}
