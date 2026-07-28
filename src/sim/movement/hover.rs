//! Hover locomotor horizontal throttle — the speed model of gamemd's
//! `HoverLocomotionClass::SpeedUpdate` (verified in
//! `docs/research/HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §3).
//!
//! The hover throttle is a `[0, 1]` fraction of the unit's base `Speed=`
//! (leptons/tick). Each tick it ramps toward a target derived from goal
//! proximity — `1.0` cruising, `0.5` on final approach — at accel/brake rates
//! set by the `HoverAcceleration` / `HoverBrake` keys (both TIMES in minutes;
//! `× 900` ticks/minute). A `HoverBoost` straightaway multiplier exists but the
//! post-boost clamp to `1.0` makes it a near-no-op at cruise (it only lifts the
//! `0.5` approach throttle to `0.75`), matching the binary.
//!
//! This module is the pure, verified horizontal-speed math (M2 phase 1). Wiring
//! it into a standalone hover movement path — the continuous cos/sin XY
//! integrator that replaces the drive-track ride these units borrow today — is
//! phase 2; the vertical bob/float controller is phase 3.
//!
//! ## Dependency rules
//! - Part of `sim/` — depends only on `util/fixed_math`, serde, std.
//! - `sim/` NEVER depends on `render/`, `ui/`, `sidebar/`, `audio/`, `net/`.

use crate::sim::movement::homing_movement::{atan2_bam, cos_bam, sin_bam};
use crate::util::fixed_math::{SIM_ONE, SIM_ZERO, SimFixed};

/// BAM offset between this repo's facing convention (0 = north, 0x4000 = east)
/// and the trig convention of `atan2_bam`/`cos_bam`/`sin_bam` (0 = +x east,
/// 0x4000 = +y south).
const FACING_TO_TRIG_BAM: u16 = 0x4000;

/// Turn-stall threshold: forward speed request drops to 0 while the body needs
/// to rotate MORE than 45° (gamemd literal `0x2000` of the 0x10000 circle,
/// strict greater-than) to reach the desired heading.
pub const HOVER_TURN_STALL_BAM: u16 = 0x2000;

/// Ticks per minute at the native 15 fps binary-frame rate (gamemd constant
/// `900.0` = 15 fps × 60 s, `double @0x007E27F8`). The minute-valued Hover time
/// keys divide by this to become per-tick rates.
pub const HOVER_TICKS_PER_MINUTE: i32 = 900;

/// Final-approach distance in leptons (~1 cell, gamemd literal `0xFF`): inside
/// this range of the goal the throttle request drops from `1.0` to `0.5`.
pub const HOVER_APPROACH_SLOWDOWN_LEPTONS: i32 = 0xFF;

/// Fallback `HoverAcceleration` when no RuleSet is available (stock 0.02 minutes).
pub const HOVER_ACCELERATION_DEFAULT_MINUTES: SimFixed = SimFixed::lit("0.02");
/// Fallback `HoverBrake` when no RuleSet is available (stock 0.03 minutes).
pub const HOVER_BRAKE_DEFAULT_MINUTES: SimFixed = SimFixed::lit("0.03");

/// Per-tick throttle step for a minute-valued rate key (`HoverAcceleration` or
/// `HoverBrake`): `1 / (rate_minutes × 900)`.
///
/// gamemd computes this in `double`; here it is the `SimFixed` reciprocal (the
/// faithful `+= 1/(rate×900)` structure). Because neither `0.02`/`0.03` nor their
/// reciprocals are exact in `I16F16`, the resulting ramp DURATION can differ from
/// the idealized `rate × 900` (18 / 27 ticks) by one tick at the clamp boundary;
/// that ±1-tick boundary is left to a live gamemd hover-speed trace to certify
/// rather than hand-tuned to the doc's idealized value (see the module tests).
pub fn hover_ramp_step(rate_minutes: SimFixed) -> SimFixed {
    let denom = rate_minutes * SimFixed::from_num(HOVER_TICKS_PER_MINUTE);
    if denom <= SIM_ZERO {
        // Degenerate (zero ramp time) → snap to target in a single tick.
        return SIM_ONE;
    }
    SIM_ONE / denom
}

/// This tick's target throttle: `min(speed_mult × speed_request, 1.0)`.
///
/// The clamp is applied AFTER the boost multiply (verified `0x00515ED0`), so
/// `HoverBoost = 1.5` at `speed_request = 1.0` clamps back to `1.0` (no effect at
/// cruise) and only raises the `0.5` approach request to `0.75`.
pub fn hover_speed_target(speed_request: SimFixed, speed_mult: SimFixed) -> SimFixed {
    (speed_mult * speed_request).min(SIM_ONE)
}

/// Ramp `current` one tick toward `target`: accelerate up by `accel_step`
/// (clamped to `target`), brake down by `brake_step` (clamped to `0`, matching
/// gamemd's `max(0.0, …)` — braking does not clamp at `target`). Equal → hold.
pub fn hover_ramp_throttle(
    current: SimFixed,
    target: SimFixed,
    accel_step: SimFixed,
    brake_step: SimFixed,
) -> SimFixed {
    if current < target {
        (current + accel_step).min(target)
    } else if target < current {
        (current - brake_step).max(SIM_ZERO)
    } else {
        current
    }
}

/// Desired 16-bit body facing toward a waypoint at lepton delta `(dx, dy)`
/// (+x east, +y south). Deterministic integer atan2 (BAM LUT), converted from
/// the trig convention to the facing convention (0 = north).
pub fn hover_desired_facing16(dx_leptons: SimFixed, dy_leptons: SimFixed) -> u16 {
    atan2_bam(dy_leptons, dx_leptons).wrapping_add(FACING_TO_TRIG_BAM)
}

/// Unit movement vector for a 16-bit body facing: the hover XY step direction
/// is the hull heading itself (facing-lagged curves), not the path vector.
/// Feed this to `move_dir_{x,y}` with `move_dir_len = 1`.
pub fn hover_move_dir(facing16: u16) -> (SimFixed, SimFixed) {
    let trig: u16 = facing16.wrapping_sub(FACING_TO_TRIG_BAM);
    (cos_bam(trig), sin_bam(trig))
}

/// Whether the shortest arc from the current (animated) facing to the desired
/// facing exceeds the 45° turn-stall threshold (strict `>`, gamemd semantics).
pub fn hover_turning_hard(current16: u16, desired16: u16) -> bool {
    let diff: i16 = desired16.wrapping_sub(current16) as i16;
    diff.unsigned_abs() > HOVER_TURN_STALL_BAM
}

/// This tick's speed request: `0` while turning hard, `0.5` on the arrival
/// slow-in (within ~1 cell of the final goal) or the departure slow-out
/// (within ~1 cell of the path start — the step-start gate; per-PATH
/// interpretation, see module docs), else `1.0` cruise.
pub fn hover_speed_request(
    turning_hard: bool,
    dist_to_goal_leptons: SimFixed,
    dist_from_start_leptons: SimFixed,
) -> SimFixed {
    if turning_hard {
        return SIM_ZERO;
    }
    let near: SimFixed = SimFixed::from_num(HOVER_APPROACH_SLOWDOWN_LEPTONS);
    if dist_to_goal_leptons <= near || dist_from_start_leptons <= near {
        SimFixed::lit("0.5")
    } else {
        SIM_ONE
    }
}

/// One movement-tick throttle update: ramp the persisted throttle toward
/// `min(boost_mult × request, 1.0)` at the accel/brake minute rates.
/// `boost_mult` is `HoverBoost` when the next two queued path steps share a
/// direction (the straightaway condition), else `1.0`; the post-boost clamp
/// makes the boost a no-op at full cruise (verified).
pub fn hover_tick_throttle(
    throttle: SimFixed,
    request: SimFixed,
    boost_mult: SimFixed,
    accel_minutes: SimFixed,
    brake_minutes: SimFixed,
) -> SimFixed {
    hover_ramp_throttle(
        throttle,
        hover_speed_target(request, boost_mult),
        hover_ramp_step(accel_minutes),
        hover_ramp_step(brake_minutes),
    )
}

/// Bob period in ticks: `round(Kscale × HoverBob × 900)`, where `Kscale` is
/// `1.0` moving / `1.1` idle. Stock defaults → 36 ticks moving, 40 idle.
/// Rounded (not truncated): the original computes `0.04 × 900` in `double`
/// where it lands on 36 exactly; the I16F16 product falls a hair short, so
/// truncation would drift the period by one tick.
pub fn hover_bob_period_ticks(hover_bob_minutes: SimFixed, moving: bool) -> i64 {
    let kscale: SimFixed = if moving {
        SIM_ONE
    } else {
        SimFixed::lit("1.1")
    };
    let period: i64 = (kscale * hover_bob_minutes * SimFixed::from_num(HOVER_TICKS_PER_MINUTE))
        .round()
        .to_num::<i64>();
    period.max(1)
}

/// One tick of the hover vertical controller — the damped-spring altitude
/// hold plus the visible cosine bob. Returns `(new_height, new_bob_offset)`.
///
/// `height` is the current altitude above ground in leptons (an integer value —
/// the visible height is truncated each tick like the original); `bob_offset`
/// is the spring's velocity-like state. The offset is pulled up while the unit
/// sits below `HoverHeight` (proportional lift, strongest near the ground, an
/// extra integer `Gravity/3` kick below `HoverHeight/4`), pulled down by
/// `Gravity` every tick, and damped by `HoverDampen`, settling the unit at
/// cruise. On top rides a `2·cos(phase)` wobble whose period comes from
/// `hover_bob_period_ticks`. When `powered` is false the lift term is skipped
/// (EMP'd / unpowered hover units sink). `climbing` (next cell's ground higher
/// than the current cell's) measures the height deficit against the uphill
/// slope, adding lift while ascending.
#[allow(clippy::too_many_arguments)]
pub fn hover_vertical_tick(
    height: SimFixed,
    bob_offset: SimFixed,
    binary_frame: u32,
    moving: bool,
    climbing: bool,
    powered: bool,
    hover_height: i32,
    hover_bob_minutes: SimFixed,
    hover_dampen: SimFixed,
    gravity: i32,
) -> (SimFixed, SimFixed) {
    let hover_h: SimFixed = SimFixed::from_num(hover_height.max(1));
    let heff: SimFixed = if moving && climbing {
        height - hover_h
    } else {
        height
    };

    // Visible height first, with LAST tick's offset (native update order):
    // ftol(2·cos(phase) + H + offset), floored at 0 (a floor hit also zeroes
    // the spring state).
    let period: i64 = hover_bob_period_ticks(hover_bob_minutes, moving);
    // Moving nudges the phase counter forward by 2 (the `counter + 2·b` term).
    let counter: i64 = binary_frame as i64 + if moving { 2 } else { 0 };
    let bam: u16 = (((counter % period) * 65536) / period) as u16;
    let wobble: SimFixed = cos_bam(bam) * SimFixed::from_num(2);

    let mut offset: SimFixed = bob_offset;
    let mut visible: SimFixed = (wobble + height + offset).floor();
    if visible < SIM_ZERO {
        offset = SIM_ZERO;
        visible = SIM_ZERO;
    }

    // Damped-spring update of the offset toward cruise.
    let g: SimFixed = SimFixed::from_num(gravity);
    if heff < hover_h {
        if powered {
            offset += ((hover_h * 2 - heff) / hover_h) * g;
        }
        if heff < hover_h / SimFixed::from_num(4) {
            // Integer division (the original uses a magic-constant idiv by 3).
            offset += SimFixed::from_num(gravity / 3);
        }
    }
    offset = (offset - g) * hover_dampen;

    (visible, offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Count ticks to ramp from `start` to `target` at the given step (the other
    /// step is irrelevant for a monotonic ramp). Caps to avoid a runaway loop.
    fn ticks_to_reach(start: SimFixed, target: SimFixed, step: SimFixed) -> u32 {
        let mut cur = start;
        for t in 1..=1000u32 {
            cur = hover_ramp_throttle(cur, target, step, step);
            if cur == target {
                return t;
            }
        }
        panic!("ramp did not converge");
    }

    #[test]
    fn accel_ramp_full_in_native_duration() {
        // HoverAcceleration=0.02 min × 900 = 18 ticks (doc §8.4). Fixed-point
        // reciprocal rounding may land on 18 or 19 at the clamp boundary; assert
        // the observed value and pin it as a regression ratchet. Certifying the
        // exact 18-vs-19 boundary needs a live gamemd hover-speed trace.
        let step = hover_ramp_step(SimFixed::from_num(0.02));
        let ticks = ticks_to_reach(SIM_ZERO, SIM_ONE, step);
        assert!(
            (18..=19).contains(&ticks),
            "accel ramp should take ~18 ticks (doc idealized), got {ticks}"
        );
    }

    #[test]
    fn brake_ramp_zero_in_native_duration() {
        // HoverBrake=0.03 min × 900 = 27 ticks (doc §8.4). Same ±1 boundary note.
        let step = hover_ramp_step(SimFixed::from_num(0.03));
        let ticks = ticks_to_reach(SIM_ONE, SIM_ZERO, step);
        assert!(
            (27..=28).contains(&ticks),
            "brake ramp should take ~27 ticks (doc idealized), got {ticks}"
        );
    }

    #[test]
    fn boost_clamped_to_one_at_cruise() {
        // The post-boost clamp: 1.5 × 1.0 → 1.0 (no cruise effect); 1.5 × 0.5 → 0.75.
        let boost = SimFixed::from_num(1.5);
        assert_eq!(hover_speed_target(SIM_ONE, boost), SIM_ONE);
        assert_eq!(
            hover_speed_target(SimFixed::from_num(0.5), boost),
            SimFixed::from_num(0.75)
        );
        // Without boost, the request passes through unchanged.
        assert_eq!(
            hover_speed_target(SimFixed::from_num(0.5), SIM_ONE),
            SimFixed::from_num(0.5)
        );
    }

    #[test]
    fn tick_throttle_cruise_ramps_up_and_approach_ramps_toward_half() {
        let accel = HOVER_ACCELERATION_DEFAULT_MINUTES;
        let brake = HOVER_BRAKE_DEFAULT_MINUTES;
        let far = SimFixed::from_num(1000); // > 255 leptons → cruise request 1.0
        let near = SimFixed::from_num(200); // ≤ 255 leptons → approach request 0.5
        let half = SimFixed::lit("0.5");

        // From rest, far from both endpoints: cruise request, one accel step.
        let req = hover_speed_request(false, far, far);
        assert_eq!(req, SIM_ONE);
        let t1 = hover_tick_throttle(SIM_ZERO, req, SIM_ONE, accel, brake);
        assert_eq!(t1, hover_ramp_step(accel));

        // At full cruise, near the goal: request 0.5, one brake step down.
        let req = hover_speed_request(false, near, far);
        assert_eq!(req, half);
        let t2 = hover_tick_throttle(SIM_ONE, req, SIM_ONE, accel, brake);
        assert_eq!(t2, SIM_ONE - hover_ramp_step(brake));

        // Departure slow-out: near the path START also requests 0.5.
        assert_eq!(hover_speed_request(false, far, near), half);

        // Already at the request: holds exactly.
        assert_eq!(hover_tick_throttle(half, half, SIM_ONE, accel, brake), half);

        // Boundary: exactly 255 leptons counts as approach (<=, gamemd 0xFF).
        let at_boundary = SimFixed::from_num(HOVER_APPROACH_SLOWDOWN_LEPTONS);
        assert_eq!(hover_speed_request(false, at_boundary, far), half);

        // Turn-stall wins over everything: request 0.
        assert_eq!(hover_speed_request(true, far, far), SIM_ZERO);

        // Boost lifts the 0.5 approach to 0.75 but is clamped away at cruise.
        let boost = SimFixed::lit("1.5");
        let t_approach = hover_tick_throttle(half, half, boost, accel, brake);
        assert!(
            t_approach > half,
            "boosted 0.5 request targets 0.75, so a 0.5 throttle accelerates"
        );
        let t_cruise = hover_tick_throttle(SIM_ONE, SIM_ONE, boost, accel, brake);
        assert_eq!(t_cruise, SIM_ONE, "boost is a clamp no-op at full cruise");
    }

    #[test]
    fn steering_cardinal_facings_and_move_dirs() {
        let cell = SimFixed::from_num(256);
        // Desired facing16 from waypoint deltas (facing: 0=N, 0x4000=E,
        // 0x8000=S, 0xC000=W; +x east, +y south).
        assert_eq!(hover_desired_facing16(SIM_ZERO, -cell), 0x0000); // north
        assert_eq!(hover_desired_facing16(cell, SIM_ZERO), 0x4000); // east
        assert_eq!(hover_desired_facing16(SIM_ZERO, cell), 0x8000); // south
        assert_eq!(hover_desired_facing16(-cell, SIM_ZERO), 0xC000); // west

        // Facing → unit movement vector.
        let (dx, dy) = hover_move_dir(0x4000); // east
        assert_eq!((dx, dy), (SIM_ONE, SIM_ZERO));
        let (dx, dy) = hover_move_dir(0x0000); // north
        assert_eq!((dx, dy), (SIM_ZERO, -SIM_ONE));
        let (dx, dy) = hover_move_dir(0x8000); // south
        assert_eq!((dx, dy), (SIM_ZERO, SIM_ONE));
        let (dx, dy) = hover_move_dir(0xC000); // west
        assert_eq!((dx, dy), (-SIM_ONE, SIM_ZERO));
    }

    #[test]
    fn turn_stall_boundary_is_strict_greater_than_45_degrees() {
        // gamemd: stall only when |diff| > 0x2000. Exactly 45° does NOT stall.
        assert!(!hover_turning_hard(0x0000, 0x2000));
        assert!(hover_turning_hard(0x0000, 0x2001));
        // Shortest-arc wrap: 0xF000 → 0x1000 is +0x2000 (45°), not stalled.
        assert!(!hover_turning_hard(0xF000, 0x1000));
        // 180° is the maximum arc — stalled.
        assert!(hover_turning_hard(0x0000, 0x8000));
    }

    #[test]
    fn bob_period_matches_native_durations() {
        // Stock HoverBob=0.04 minutes: 36 ticks moving (Kscale 1.0), 40 idle
        // (Kscale 1.1; 39.59 rounds to 40).
        let bob = SimFixed::from_num(0.04);
        assert_eq!(hover_bob_period_ticks(bob, true), 36);
        assert_eq!(hover_bob_period_ticks(bob, false), 40);
        // Degenerate zero period clamps to 1 (no division by zero).
        assert_eq!(hover_bob_period_ticks(SIM_ZERO, true), 1);
    }

    /// Run the vertical controller `n` ticks from the given state (stock
    /// values: HoverHeight=120, HoverBob=.04, HoverDampen=.4, Gravity=6).
    fn run_vertical(
        mut height: SimFixed,
        n: u32,
        moving: bool,
        powered: bool,
    ) -> (SimFixed, SimFixed) {
        let mut offset = SIM_ZERO;
        for frame in 0..n {
            let (h, o) = hover_vertical_tick(
                height,
                offset,
                frame,
                moving,
                false,
                powered,
                120,
                SimFixed::from_num(0.04),
                SimFixed::from_num(0.4),
                6,
            );
            height = h;
            offset = o;
        }
        (height, offset)
    }

    #[test]
    fn vertical_spring_lifts_from_ground_and_settles_near_hover_height() {
        // From the ground, powered: the spring lifts the unit and settles it
        // around HoverHeight (±the 2-lepton wobble and the coarse lift steps).
        let (h, _) = run_vertical(SIM_ZERO, 200, false, true);
        let h_i = h.to_num::<i32>();
        assert!(
            (100..=140).contains(&h_i),
            "settled near cruise altitude 120, got {h_i}"
        );
        // Never below ground at any point along the way (floor clamp) — spot
        // check by re-running and asserting per-tick.
        let mut height = SIM_ZERO;
        let mut offset = SIM_ZERO;
        for frame in 0..200u32 {
            let (nh, no) = hover_vertical_tick(
                height,
                offset,
                frame,
                false,
                false,
                true,
                120,
                SimFixed::from_num(0.04),
                SimFixed::from_num(0.4),
                6,
            );
            assert!(nh >= SIM_ZERO, "height clamped at ground");
            height = nh;
            offset = no;
        }
    }

    #[test]
    fn vertical_spring_sinks_when_unpowered() {
        // Cruise-height unit loses power: the lift term is skipped, so the
        // gravity/damping tail sinks it to the ground.
        let (h, _) = run_vertical(SimFixed::from_num(120), 150, false, false);
        assert!(
            h.to_num::<i32>() <= 2,
            "unpowered hover sinks to the ground, got {}",
            h.to_num::<i32>()
        );
    }

    #[test]
    fn ramp_holds_at_target_and_does_not_overshoot() {
        let step = hover_ramp_step(SimFixed::from_num(0.02));
        // At target → unchanged.
        assert_eq!(hover_ramp_throttle(SIM_ONE, SIM_ONE, step, step), SIM_ONE);
        // One step below target lands exactly on target (min clamp), not past it.
        let near = SIM_ONE - step / SimFixed::from_num(2);
        assert_eq!(hover_ramp_throttle(near, SIM_ONE, step, step), SIM_ONE);
    }
}
