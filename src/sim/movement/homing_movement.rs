//! Homing missile flight — per-tick yaw correction toward a tracked target.
//!
//! Used for projectiles with `Ranged=yes`, such as AAHeatSeeker2 fired by
//! Guardian GI's MissileLauncher. Distinct from `rocket_movement.rs`, which
//! handles ballistic-arc projectiles (V3, dumb-fire) — keep them separate;
//! do not merge.
//!
//! ## State machine
//! Arming → Cruise → Detonation
//!         ↘ SelfDestruct (stall failsafe)
//!
//! ## Determinism
//! Sim-critical numeric fields use `SimFixed` for deterministic lockstep.
//! BAM angles are integer `u16` (wrapping arithmetic is exact).
//! Render-only `pitch` is `f32` and excluded from the state hash.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/entity_store, sim/game_entity.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::util::fixed_math::SimFixed;

/// Phase within the homing missile state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HomingPhase {
    /// Arming: per-tick decrement until ready to detonate on impact.
    Arming,
    /// Cruise: tracking target with sidewinder yaw + cruise altitude control.
    Cruise,
    /// Stall failsafe: target unreachable, detonate next tick.
    SelfDestruct,
    /// Impact: caller despawns this tick.
    Detonation,
}

/// State for an in-flight homing missile.
///
/// Sim-critical numeric fields use `SimFixed` for deterministic lockstep.
/// BAM angles are `u16` (wrapping integer arithmetic is exact).
/// Render-only `pitch` stays `f32`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HomingState {
    pub phase: HomingPhase,

    // Target tracking
    pub target_id: Option<u64>,
    pub last_known_rx: u16,
    pub last_known_ry: u16,

    // Flight kinematics
    pub yaw_bam: u16,
    pub pitch_bam: u16,
    pub speed: SimFixed,
    pub altitude: SimFixed,
    pub vz: SimFixed,

    // Per-projectile parameters from BulletType / WeaponType / Rules
    pub rot_ini: u16,
    pub missile_rot_var: SimFixed,
    pub floater: bool,
    pub very_high: bool,
    pub arm_ticks_remaining: u16,

    // Sidewinder phase + stall detection
    pub frame_counter: u32,
    pub stall_counter: u8,
    pub stall_ema: SimFixed,
    pub last_distance_to_target: SimFixed,

    /// Render-only pitch in radians. Excluded from the deterministic state
    /// hash — see the manual `Hash` impl below.
    #[serde(skip, default)]
    pub pitch: f32,
}

/// Precomputed cosine modulation table: `cos(2π * i / 15)` for `i` in `0..15`.
///
/// Replaces runtime cosine evaluation in the homing flight loop. Values are
/// stored as `SimFixed` literals (compile-time parsed) so the table is fully
/// deterministic — no f32 trig in the sim layer.
///
/// The 15-frame period is the "sidewinder" name's origin — the modulation
/// produces the characteristic oscillating flight curve.
const SIDEWINDER_TABLE: [SimFixed; 15] = [
    SimFixed::lit("1.0"),                  // cos(0)
    SimFixed::lit("0.91354545764260087"),  // cos(2π/15)
    SimFixed::lit("0.66913060635885821"),  // cos(4π/15)
    SimFixed::lit("0.30901699437494745"),  // cos(6π/15)
    SimFixed::lit("-0.10452846326765346"), // cos(8π/15)
    SimFixed::lit("-0.5"),                 // cos(10π/15)
    SimFixed::lit("-0.80901699437494745"), // cos(12π/15)
    SimFixed::lit("-0.97814760073380562"), // cos(14π/15)
    SimFixed::lit("-0.97814760073380562"), // cos(16π/15)
    SimFixed::lit("-0.80901699437494745"), // cos(18π/15)
    SimFixed::lit("-0.5"),                 // cos(20π/15)
    SimFixed::lit("-0.10452846326765346"), // cos(22π/15)
    SimFixed::lit("0.30901699437494745"),  // cos(24π/15)
    SimFixed::lit("0.66913060635885821"),  // cos(26π/15)
    SimFixed::lit("0.91354545764260087"),  // cos(28π/15)
];

/// Lookup the sidewinder cosine for the given frame counter.
pub(crate) fn sidewinder_cos(frame_counter: u32) -> SimFixed {
    SIDEWINDER_TABLE[(frame_counter % 15) as usize]
}

/// Inclusive ROT cap check: returns `true` when current yaw can snap directly
/// to target this tick (i.e. `|delta| <= cap`).
///
/// Inclusive comparison (`<=`) matches the original's IsWithinROT — equality
/// at the boundary snaps; off-by-one would over-rotate by one BAM step.
pub(crate) fn within_rot_bam(cur: u16, tgt: u16, cap: u16) -> bool {
    let delta_signed = (cur.wrapping_sub(tgt)) as i16;
    (delta_signed.unsigned_abs() as u16) <= cap
}

/// Step current BAM angle toward target by at most `cap`; snap to target when
/// within `cap`. Picks the shortest-arc direction via wrapping `i16`
/// subtraction.
pub(crate) fn step_toward_bam_inclusive(cur: u16, tgt: u16, cap: u16) -> u16 {
    if within_rot_bam(cur, tgt, cap) {
        return tgt;
    }
    let delta_signed = (tgt.wrapping_sub(cur)) as i16;
    if delta_signed > 0 {
        cur.wrapping_add(cap)
    } else {
        cur.wrapping_sub(cap)
    }
}

/// Compute the BAM heading from a delta vector. Uses `f32` `atan2` internally;
/// the result is truncated to `u16` BAM.
///
/// Bounded jitter (≤±1 BAM) cannot flip the monotonic `<=` comparison in
/// `within_rot_bam` (cap is always ≫1 BAM), so the f32 use is lockstep-safe.
/// If lockstep desync ever surfaces here, replace with a SimFixed BAM table.
pub(crate) fn atan2_bam(dy: SimFixed, dx: SimFixed) -> u16 {
    use crate::util::fixed_math::sim_to_f32;
    let angle_rad = sim_to_f32(dy).atan2(sim_to_f32(dx));
    let bam_f = angle_rad * (32768.0 / std::f32::consts::PI);
    (bam_f as i32).rem_euclid(65536) as u16
}

impl std::hash::Hash for HomingState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.phase.hash(state);
        self.target_id.hash(state);
        self.last_known_rx.hash(state);
        self.last_known_ry.hash(state);
        self.yaw_bam.hash(state);
        self.pitch_bam.hash(state);
        self.speed.to_bits().hash(state);
        self.altitude.to_bits().hash(state);
        self.vz.to_bits().hash(state);
        self.rot_ini.hash(state);
        self.missile_rot_var.to_bits().hash(state);
        self.floater.hash(state);
        self.very_high.hash(state);
        self.arm_ticks_remaining.hash(state);
        self.frame_counter.hash(state);
        self.stall_counter.hash(state);
        self.stall_ema.to_bits().hash(state);
        self.last_distance_to_target.to_bits().hash(state);
        // `pitch: f32` intentionally omitted — render-only.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidewinder_table_min_max() {
        let max = SIDEWINDER_TABLE
            .iter()
            .copied()
            .fold(SimFixed::from_num(-2), SimFixed::max);
        let min = SIDEWINDER_TABLE
            .iter()
            .copied()
            .fold(SimFixed::from_num(2), SimFixed::min);
        assert!(max <= SimFixed::from_num(1));
        assert!(max >= SimFixed::lit("0.9"));
        assert!(min <= SimFixed::lit("-0.9"));
        assert!(min >= SimFixed::from_num(-1));
    }

    #[test]
    fn sidewinder_cos_wraps_at_15() {
        assert_eq!(sidewinder_cos(0), sidewinder_cos(15));
        assert_eq!(sidewinder_cos(7), sidewinder_cos(22));
        assert_eq!(sidewinder_cos(0), SimFixed::from_num(1));
    }

    #[test]
    fn within_rot_bam_inclusive_at_boundary() {
        // At exact ROT distance, snap (inclusive `<=`).
        assert!(within_rot_bam(0x0000, 0x0100, 0x0100));
        assert!(within_rot_bam(0x0100, 0x0000, 0x0100));
        // One past the cap — no snap.
        assert!(!within_rot_bam(0x0000, 0x0101, 0x0100));
    }

    #[test]
    fn step_toward_bam_inclusive_snaps_at_cap() {
        // Exactly at cap distance -> snap.
        assert_eq!(step_toward_bam_inclusive(0x0000, 0x0100, 0x0100), 0x0100);
    }

    #[test]
    fn step_toward_bam_inclusive_steps_outside_cap() {
        // Beyond cap -> step by cap toward target.
        assert_eq!(step_toward_bam_inclusive(0x0000, 0x0200, 0x0100), 0x0100);
        assert_eq!(step_toward_bam_inclusive(0x0000, 0xFE00, 0x0100), 0xFF00);
    }

    #[test]
    fn step_toward_bam_wraps_around() {
        // Shortest arc across the wrap (going CCW is closer).
        assert_eq!(step_toward_bam_inclusive(0x0000, 0xFF00, 0x0100), 0xFF00);
    }

    #[test]
    fn atan2_bam_cardinal_directions() {
        // +x -> 0 BAM; +y -> 0x4000 BAM (90°).
        let zero_x = atan2_bam(SimFixed::from_num(0), SimFixed::from_num(1));
        let pos_y = atan2_bam(SimFixed::from_num(1), SimFixed::from_num(0));
        assert!(
            zero_x < 8 || zero_x > 0xFFF8,
            "0 BAM ≈ +x (got 0x{:04X})",
            zero_x
        );
        assert!(
            (pos_y as i32 - 0x4000_i32).abs() < 8,
            "0x4000 BAM ≈ +y (got 0x{:04X})",
            pos_y
        );
    }
}
