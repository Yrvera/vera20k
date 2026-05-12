//! Per-tick spring-damper + slope-transition advance.
//!
//! All angles and angular velocities are in radians, stored as `SimFixed`
//! (I16F16, ~1.5e-5 precision). Constants here are extracted from the
//! reference engine; do not change without binary verification.

use crate::sim::components::RockingState;
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

/// Advance one rocking axis (sideways OR forwards) by one tick.
///
/// Step order (matches reference engine RockingUpdate):
///   1. Zero-velocity short-circuit — if velocity == 0, force angle to 0.
///   2. Integrate angle += velocity.
///   3. Saturation clamp — only when stationary AND in normal range AND
///      the integration step crossed the cap this tick. Moving units drift
///      past the cap; out-of-range angles fall through to step 4.
///   4. Dampening — moving units decay at `fallback * BASE_DECAY_RATE`;
///      stationary units at `BASE_DECAY_RATE`. Out-of-range velocity is
///      pushed in the same direction as the velocity sign (which is what
///      produces the wide-amplitude run-away that L30 self-destruct catches).
///   5. Deadband snap — `|angle| <= TILT_DEADBAND` clears both fields.
///
/// `cap` selects ±π/4 (default) or ±π/10 (forwards during vehicle-vs-building
/// crush, currently DEFERRED).
pub(crate) fn advance_axis(
    angle: &mut SimFixed,
    velocity: &mut SimFixed,
    cap: SimFixed,
    is_moving: bool,
    fallback: SimFixed,
) {
    // L10: strict velocity == 0 → angle force-zero, skip integration.
    if *velocity == SimFixed::ZERO {
        *angle = SimFixed::ZERO;
        return;
    }

    // L2: integrate.
    let prev = *angle;
    let new_angle = prev + *velocity;
    *angle = new_angle;

    let in_range = angle.abs() <= NORMAL_RANGE_PI2;

    // L7: saturation fires only when stationary, in normal range, and crossing
    // the cap this tick. Both clamp the angle and zero the velocity.
    if !is_moving && in_range {
        if new_angle > cap && prev < cap {
            *angle = cap;
            *velocity = SimFixed::ZERO;
        } else if new_angle < -cap && prev > -cap {
            *angle = -cap;
            *velocity = SimFixed::ZERO;
        }
    }

    // L3 / L4 / L5: dampening.
    //   - in_range, moving:    velocity decays by fallback * BASE_DECAY_RATE
    //   - in_range, stationary: velocity decays by BASE_DECAY_RATE
    //   - out_of_range:        velocity is pushed in its own direction by
    //                          BASE_DECAY_RATE (subtract -BASE_DECAY_RATE = +)
    //                          — the runaway path the L30 self-destruct catches.
    let decay = if is_moving {
        fallback * BASE_DECAY_RATE
    } else {
        BASE_DECAY_RATE
    };
    if *velocity > SimFixed::ZERO {
        *velocity -= if in_range { decay } else { -BASE_DECAY_RATE };
    } else if *velocity < SimFixed::ZERO {
        *velocity += if in_range { decay } else { -BASE_DECAY_RATE };
    }

    // L9: deadband snap clears both angle and velocity in the same tick.
    if angle.abs() <= TILT_DEADBAND {
        *angle = SimFixed::ZERO;
        *velocity = SimFixed::ZERO;
    }
}

/// Advance the ship-rocking path: integrate without damping, asymmetric
/// one-sided clamp. Used for EMP wobble and naval continuous rocking.
///
/// `type_supports_ship_rocking` is the per-type allow flag — when false,
/// integration still runs but no clamping is applied (matches the
/// non-clamping branch in the reference engine for types that "shouldn't"
/// ship-rock but had the flag set externally).
pub(crate) fn advance_ship_rocking(rocking: &mut RockingState, type_supports_ship_rocking: bool) {
    rocking.angle_forwards += rocking.vel_forwards;
    rocking.angle_sideways += rocking.vel_sideways;
    if !type_supports_ship_rocking {
        return;
    }
    // Lower clamps to -π/4 on both axes.
    if rocking.angle_forwards < -SATURATION_PI4 {
        rocking.angle_forwards = -SATURATION_PI4;
    }
    if rocking.angle_sideways < -SATURATION_PI4 {
        rocking.angle_sideways = -SATURATION_PI4;
    }
    // Asymmetric upper clamp — sideways only, forwards is allowed to drift
    // positive without clamping (matches the reference engine).
    if rocking.angle_sideways >= SATURATION_PI4 {
        rocking.angle_sideways = SATURATION_PI4;
    }
}

/// Update the slope-transition state for one entity.
///
/// If `cell_slope` differs from the tracked `curr_slope`, start (or
/// restart) a fresh 3-tick transition: the old `curr_slope` becomes the
/// new `prev_slope`, and the counter resets to `SLOPE_TRANSITION_TICKS`.
/// Otherwise, decrement the counter (saturating at 0).
///
/// The render side reads `prev_slope`/`curr_slope`/`transition_ticks_remaining`
/// to SLERP between the two slope matrices.
pub(crate) fn update_slope_transition(rocking: &mut RockingState, cell_slope: u8) {
    if cell_slope != rocking.curr_slope {
        rocking.prev_slope = rocking.curr_slope;
        rocking.curr_slope = cell_slope;
        rocking.transition_ticks_remaining = SLOPE_TRANSITION_TICKS;
    } else if rocking.transition_ticks_remaining > 0 {
        rocking.transition_ticks_remaining -= 1;
    }
}

/// Stub entry point — implemented in Task 11.
pub fn tick(_entities: &mut EntityStore) {
    // Implemented in Task 11.
}
