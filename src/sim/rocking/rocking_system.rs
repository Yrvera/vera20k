//! Per-tick spring-damper + slope-transition advance.
//!
//! All angles and angular velocities are in radians, stored as `SimFixed`
//! (I16F16, ~1.5e-5 precision). Constants here are extracted from the
//! reference engine; do not change without binary verification.

use crate::sim::entity_store::EntityStore;
use crate::util::fixed_math::SimFixed;

/// Tilt-renderer deadband. Both angles below this snap to zero and the unit
/// renders via the static atlas path.
pub const TILT_DEADBAND: SimFixed = SimFixed::lit("0.00002");

/// Saturation cap for body roll/pitch (±π/4 ≈ 0.7854 rad).
pub const SATURATION_PI4: SimFixed = SimFixed::lit("0.7853982");

/// Tighter forwards-only saturation cap (±π/10 ≈ 0.3142 rad) used when a
/// Crusher vehicle is mid-crush of a building. The `TechnoClass+0x6B5`-style
/// gate that selects this cap is DEFERRED until building-crushing lands; the
/// constant is defined so re-enabling that path later is a one-line change.
pub const SATURATION_PI10: SimFixed = SimFixed::lit("0.3141593");

/// "Out of normal range" threshold (±π/2). Above this, dampening pushes back
/// inward at the base rate regardless of `is_moving`.
pub const NORMAL_RANGE_PI2: SimFixed = SimFixed::lit("1.5707963");

/// Base damping rate (rad/tick). Stationary units use this directly; moving
/// units scale it by `fallback_coefficient` (default 0.1 → 0.0002 rad/tick).
pub const BASE_DECAY_RATE: SimFixed = SimFixed::lit("0.002");

/// Snap-back rate for the velocity-fighting-itself sub-branch (rad/tick).
/// Used only in the out-of-normal-range path.
pub const SNAP_BACK_RATE: SimFixed = SimFixed::lit("0.005");

/// Per-axis velocity cap applied at impulse-receive time (rad/tick).
pub const IMPULSE_VEL_CAP: SimFixed = SimFixed::lit("0.05");

/// Slope-transition duration in sim ticks (hard-coded constant).
pub const SLOPE_TRANSITION_TICKS: u8 = 3;

/// Saturation cap on rocker impulse force from area-damage (clamped to 4.0
/// before the per-axis velocity gate). Bounds defended both at the source
/// and inside `apply_rocker_impulse` against any wiring error.
pub const FORCE_SATURATION: SimFixed = SimFixed::lit("4");

/// Minimum force for the Apply_area_damage 3×3 cell impulse loop to fire at
/// all (after `FORCE_SATURATION` clamp). Below this floor, no impulses are
/// applied to any target in the radius.
pub const APPLY_AREA_FORCE_FLOOR: SimFixed = SimFixed::lit("0.3");

/// Stub entry point — implemented in Task 11.
pub fn tick(_entities: &mut EntityStore) {
    // Implemented in Task 11.
}
