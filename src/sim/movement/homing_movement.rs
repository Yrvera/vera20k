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
