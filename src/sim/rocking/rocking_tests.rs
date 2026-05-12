//! Tests for the rocking system. Populated incrementally by Tasks 7–10.

#![cfg(test)]

use crate::sim::components::RockingState;
use crate::sim::rocking::rocking_system::{
    BASE_DECAY_RATE, NORMAL_RANGE_PI2, SATURATION_PI4, SATURATION_PI10, TILT_DEADBAND, advance_axis,
    advance_ship_rocking,
};
use crate::util::fixed_math::SimFixed;

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
