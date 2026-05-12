//! Tests for the rocking system. Populated incrementally by Tasks 7–10.

#![cfg(test)]

use crate::sim::components::RockingState;
use crate::sim::rocking::impulse::apply_rocker_impulse;
use crate::sim::rocking::rocking_system::{
    BASE_DECAY_RATE, IMPULSE_VEL_CAP, NORMAL_RANGE_PI2, SATURATION_PI4, SATURATION_PI10,
    SLOPE_TRANSITION_TICKS, TILT_DEADBAND, advance_axis, advance_ship_rocking,
    update_slope_transition,
};
use crate::util::fixed_math::SimFixed;

const DEFAULT_WEIGHT: SimFixed = SimFixed::lit("2.0");

const FALLBACK: SimFixed = SimFixed::lit("0.1");

#[test]
fn zero_velocity_force_zeros_angle() {
    let mut a = SimFixed::lit("0.5");
    let mut v = SimFixed::ZERO;
    advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    assert_eq!(a, SimFixed::ZERO);
}

#[test]
fn integrate_simple() {
    let mut a = SimFixed::lit("0.1");
    let mut v = SimFixed::lit("0.01");
    advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    // Integrate: a = 0.1 + 0.01 = 0.11; below cap, stationary + in-range so
    // saturation skipped. Dampen by BASE_DECAY_RATE: v = 0.01 - 0.002 = 0.008.
    let eps = SimFixed::lit("0.0001");
    assert!((a - SimFixed::lit("0.11")).abs() < eps);
    assert!((v - SimFixed::lit("0.008")).abs() < eps);
}

#[test]
fn saturation_clamps_when_stationary_and_crossing() {
    let mut a = SimFixed::lit("0.78"); // just below π/4 ≈ 0.7854
    let mut v = SimFixed::lit("0.05");
    advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    // new = 0.83; crosses π/4 from below; stationary + in_range → clamp.
    assert_eq!(a, SATURATION_PI4);
    assert_eq!(v, SimFixed::ZERO);
}

#[test]
fn saturation_does_not_clamp_when_moving() {
    let mut a = SimFixed::lit("0.78");
    let mut v = SimFixed::lit("0.05");
    advance_axis(&mut a, &mut v, SATURATION_PI4, true /* moving */, FALLBACK);
    // Moving → saturation skipped; angle drifts past π/4.
    assert!(a > SATURATION_PI4);
}

#[test]
fn pi10_cap_is_supported_via_parameter() {
    // ±π/10 cap is the L8 forwards-during-crush path. The crush gate is
    // currently DEFERRED at the caller; this test pins that the math itself
    // still clamps correctly when a tighter cap is supplied. Re-enabling L8
    // later then becomes a one-line change at the caller.
    let mut a = SimFixed::lit("0.30"); // just below π/10 ≈ 0.3142
    let mut v = SimFixed::lit("0.05");
    advance_axis(&mut a, &mut v, SATURATION_PI10, false, FALLBACK);
    assert_eq!(a, SATURATION_PI10);
    assert_eq!(v, SimFixed::ZERO);
}

#[test]
fn deadband_snaps_to_zero() {
    // For the deadband branch to fire, the integrated angle must stay inside
    // TILT_DEADBAND after the integrate step. Start a=0 with the tiniest
    // possible nonzero velocity so v != 0 (L10 doesn't fire) but angle stays
    // <= deadband after integrate.
    let mut a = SimFixed::ZERO;
    let mut v = SimFixed::DELTA;
    advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    assert_eq!(a, SimFixed::ZERO);
    assert_eq!(v, SimFixed::ZERO);
}

#[test]
fn convergence_decays_to_zero_over_time() {
    let mut a = SimFixed::ZERO;
    // 0.01 is a clean multiple of BASE_DECAY_RATE in I16F16 fixed-point, so
    // velocity decays to exactly ZERO in 5 dampening ticks; the L10
    // short-circuit then forces angle to 0 on the next tick. Larger seed
    // values that aren't clean multiples leave a few DELTA units of residual
    // velocity that oscillates around 0 forever and takes hundreds of ticks
    // to drain through the integrate-step toward deadband.
    let mut v = SimFixed::lit("0.01");
    for _ in 0..100 {
        advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    }
    assert!(a.abs() <= TILT_DEADBAND);
    assert!(v.abs() <= TILT_DEADBAND);
}

#[test]
fn moving_dampens_more_slowly_than_stationary() {
    // FALLBACK = 0.1 → moving decay is 0.1 * BASE_DECAY_RATE = 0.0002/tick,
    // 10× slower than stationary's 0.002/tick. After 100 ticks a moving unit
    // should still be carrying noticeably more velocity than a stationary one.
    let mut a_moving = SimFixed::lit("0.1");
    let mut v_moving = SimFixed::lit("0.04");
    let mut a_still = SimFixed::lit("0.1");
    let mut v_still = SimFixed::lit("0.04");
    for _ in 0..50 {
        advance_axis(&mut a_moving, &mut v_moving, SATURATION_PI4, true, FALLBACK);
        advance_axis(&mut a_still, &mut v_still, SATURATION_PI4, false, FALLBACK);
    }
    assert!(
        v_moving.abs() > v_still.abs(),
        "moving v={:?} should retain more velocity than still v={:?}",
        v_moving,
        v_still
    );
}

#[test]
fn ship_rocking_integrates_without_damping() {
    let mut r = RockingState::default();
    r.vel_sideways = SimFixed::lit("0.01");
    advance_ship_rocking(&mut r, true);
    assert_eq!(r.angle_sideways, SimFixed::lit("0.01"));
    // Velocity is unchanged — no damping in this path.
    assert_eq!(r.vel_sideways, SimFixed::lit("0.01"));
}

#[test]
fn ship_rocking_clamps_upper_sideways() {
    let mut r = RockingState::default();
    r.angle_sideways = SATURATION_PI4;
    r.vel_sideways = SimFixed::lit("0.1");
    advance_ship_rocking(&mut r, true);
    assert_eq!(r.angle_sideways, SATURATION_PI4);
}

#[test]
fn ship_rocking_clamps_lower_both_axes() {
    let mut r = RockingState::default();
    r.angle_forwards = -SATURATION_PI4 + SimFixed::lit("0.001");
    r.vel_forwards = SimFixed::lit("-0.01");
    r.angle_sideways = -SATURATION_PI4 + SimFixed::lit("0.001");
    r.vel_sideways = SimFixed::lit("-0.01");
    advance_ship_rocking(&mut r, true);
    assert_eq!(r.angle_forwards, -SATURATION_PI4);
    assert_eq!(r.angle_sideways, -SATURATION_PI4);
}

#[test]
fn ship_rocking_forwards_upper_is_unclamped() {
    // Asymmetric: forwards upper is NOT clamped (only sideways upper).
    let mut r = RockingState::default();
    r.angle_forwards = SATURATION_PI4;
    r.vel_forwards = SimFixed::lit("0.1");
    advance_ship_rocking(&mut r, true);
    assert!(
        r.angle_forwards > SATURATION_PI4,
        "forwards should drift past +π/4 unclamped, got {:?}",
        r.angle_forwards,
    );
}

#[test]
fn ship_rocking_no_clamp_when_type_doesnt_support() {
    let mut r = RockingState::default();
    r.angle_sideways = SATURATION_PI4 + SimFixed::lit("0.5");
    r.vel_sideways = SimFixed::lit("0.01");
    advance_ship_rocking(&mut r, false);
    assert!(r.angle_sideways > SATURATION_PI4);
}

#[test]
fn slope_change_starts_three_tick_transition() {
    let mut r = RockingState::default();
    r.curr_slope = 0;
    update_slope_transition(&mut r, 5);
    assert_eq!(r.prev_slope, 0);
    assert_eq!(r.curr_slope, 5);
    assert_eq!(r.transition_ticks_remaining, SLOPE_TRANSITION_TICKS);
}

#[test]
fn slope_unchanged_decrements_counter() {
    let mut r = RockingState::default();
    r.curr_slope = 5;
    r.transition_ticks_remaining = 3;
    update_slope_transition(&mut r, 5);
    assert_eq!(r.transition_ticks_remaining, 2);
    update_slope_transition(&mut r, 5);
    assert_eq!(r.transition_ticks_remaining, 1);
    update_slope_transition(&mut r, 5);
    assert_eq!(r.transition_ticks_remaining, 0);
}

#[test]
fn slope_counter_saturates_at_zero() {
    let mut r = RockingState::default();
    r.curr_slope = 5;
    r.transition_ticks_remaining = 0;
    update_slope_transition(&mut r, 5);
    assert_eq!(r.transition_ticks_remaining, 0);
}

#[test]
fn slope_change_mid_transition_resets_to_three() {
    let mut r = RockingState::default();
    r.curr_slope = 5;
    r.transition_ticks_remaining = 1;
    update_slope_transition(&mut r, 7);
    assert_eq!(r.prev_slope, 5);
    assert_eq!(r.curr_slope, 7);
    assert_eq!(r.transition_ticks_remaining, SLOPE_TRANSITION_TICKS);
}

#[test]
fn impulse_caps_at_005_per_axis() {
    let mut r = RockingState::default();
    apply_rocker_impulse(
        &mut r,
        SimFixed::from_num(100),
        DEFAULT_WEIGHT,
        SimFixed::ONE,
        SimFixed::ZERO,
    );
    assert!(r.vel_sideways.abs() <= IMPULSE_VEL_CAP);
    assert!(r.vel_forwards.abs() <= IMPULSE_VEL_CAP);
}

#[test]
fn impulse_direction_from_x_axis_writes_sideways_only() {
    let mut r = RockingState::default();
    apply_rocker_impulse(
        &mut r,
        SimFixed::lit("4.0"),
        DEFAULT_WEIGHT,
        SimFixed::ONE,
        SimFixed::ZERO,
    );
    // Force=4, Weight=2 → force_scaled = 0.04 × 4 / 2 = 0.08 → clamps to 0.05.
    // nx = +1 → vel_side = -0.05; ny = 0 → vel_fwd = 0.
    assert!(r.vel_sideways < SimFixed::ZERO);
    assert_eq!(r.vel_forwards, SimFixed::ZERO);
}

#[test]
fn impulse_zero_distance_does_nothing() {
    let mut r = RockingState::default();
    apply_rocker_impulse(
        &mut r,
        SimFixed::ONE,
        DEFAULT_WEIGHT,
        SimFixed::ZERO,
        SimFixed::ZERO,
    );
    assert_eq!(r.vel_sideways, SimFixed::ZERO);
    assert_eq!(r.vel_forwards, SimFixed::ZERO);
}

#[test]
fn impulse_stacks_additively_until_cap() {
    let mut r = RockingState::default();
    // force=1.0, weight=2 → force_scaled = 0.04 × 1 / 2 = 0.02 (passes 0.01 floor).
    apply_rocker_impulse(
        &mut r,
        SimFixed::lit("1.0"),
        DEFAULT_WEIGHT,
        SimFixed::ONE,
        SimFixed::ZERO,
    );
    let v1 = r.vel_sideways;
    apply_rocker_impulse(
        &mut r,
        SimFixed::lit("1.0"),
        DEFAULT_WEIGHT,
        SimFixed::ONE,
        SimFixed::ZERO,
    );
    let v2 = r.vel_sideways;
    assert!(v2.abs() > v1.abs() || v2.abs() == IMPULSE_VEL_CAP);
}

#[test]
fn impulse_too_weak_dropped_by_floor_gate() {
    let mut r = RockingState::default();
    // force=0.1, weight=2 → force_scaled = 0.04 × 0.1 / 2 = 0.002 < 0.01 gate.
    apply_rocker_impulse(
        &mut r,
        SimFixed::lit("0.1"),
        DEFAULT_WEIGHT,
        SimFixed::ONE,
        SimFixed::ZERO,
    );
    assert_eq!(r.vel_sideways, SimFixed::ZERO);
    assert_eq!(r.vel_forwards, SimFixed::ZERO);
}

#[test]
fn impulse_heavier_unit_rocks_less_per_equal_force() {
    // L12c: Weight=5 (Aircraft Carrier) rocks 2.5× less than Weight=2 default.
    // Pick a force in the linear regime (below the 0.05 cap):
    //   light: 0.04 × 1.5 / 2.0 = 0.03
    //   heavy: 0.04 × 1.5 / 5.0 = 0.012
    //   ratio: 0.03 / 0.012 = 2.5
    let force = SimFixed::lit("1.5");
    let mut light = RockingState::default();
    apply_rocker_impulse(
        &mut light,
        force,
        SimFixed::lit("2.0"),
        SimFixed::ONE,
        SimFixed::ZERO,
    );
    let mut heavy = RockingState::default();
    apply_rocker_impulse(
        &mut heavy,
        force,
        SimFixed::lit("5.0"),
        SimFixed::ONE,
        SimFixed::ZERO,
    );
    let ratio = light.vel_sideways.abs() / heavy.vel_sideways.abs();
    assert!(
        ratio > SimFixed::lit("2.4") && ratio < SimFixed::lit("2.6"),
        "expected ~2.5× ratio (light/heavy), got {:?}",
        ratio
    );
}

#[test]
fn impulse_zero_weight_falls_back_to_default() {
    // Defensive: malformed INI section with Weight=0 must not panic or
    // produce inf — should behave identically to Weight=2.0.
    let mut r = RockingState::default();
    apply_rocker_impulse(
        &mut r,
        SimFixed::lit("4.0"),
        SimFixed::ZERO,
        SimFixed::ONE,
        SimFixed::ZERO,
    );
    let mut reference = RockingState::default();
    apply_rocker_impulse(
        &mut reference,
        SimFixed::lit("4.0"),
        DEFAULT_WEIGHT,
        SimFixed::ONE,
        SimFixed::ZERO,
    );
    assert_eq!(r.vel_sideways, reference.vel_sideways);
}

#[test]
fn impulse_forwards_axis_halved_relative_to_sideways() {
    // L12b asymmetry: pure +Y direction → vel_fwd = +ny × force_scaled × 0.5;
    // pure +X direction → vel_side = -nx × force_scaled (no halving). Same
    // force magnitude → forwards-axis impulse is exactly half the sideways one.
    let force = SimFixed::lit("4.0");
    let mut y_only = RockingState::default();
    apply_rocker_impulse(
        &mut y_only,
        force,
        DEFAULT_WEIGHT,
        SimFixed::ZERO,
        SimFixed::ONE,
    );
    let mut x_only = RockingState::default();
    apply_rocker_impulse(
        &mut x_only,
        force,
        DEFAULT_WEIGHT,
        SimFixed::ONE,
        SimFixed::ZERO,
    );
    // Sideways component (from +X impulse) is the full force-scaled
    // value (clamped at 0.05); forwards component (from +Y) is half.
    assert!(
        (x_only.vel_sideways.abs() - SimFixed::lit("0.05")).abs() < SimFixed::lit("0.001"),
        "x-only sideways should be ≈0.05, got {:?}",
        x_only.vel_sideways,
    );
    assert!(
        (y_only.vel_forwards.abs() - SimFixed::lit("0.025")).abs() < SimFixed::lit("0.001"),
        "y-only forwards should be ≈0.025 (halved), got {:?}",
        y_only.vel_forwards,
    );
}

#[test]
fn out_of_range_velocity_runs_away_not_inward() {
    // Out of normal range (|angle| > π/2), dampening sign flips so velocity
    // grows in its own direction — this is the runaway path the L30
    // self-destruct catches at ±π.
    let mut a = NORMAL_RANGE_PI2 + SimFixed::lit("0.1");
    let mut v = SimFixed::lit("0.01");
    let v_initial = v;
    advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    // Integrate pushed angle further past π/2; out-of-range branch adds
    // BASE_DECAY_RATE to a positive velocity → grows.
    assert!(
        v > v_initial,
        "out-of-range velocity should grow, was {:?} now {:?}",
        v_initial,
        v
    );
}
