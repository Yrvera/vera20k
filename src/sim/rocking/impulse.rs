//! Rocker impulse application (port of TechnoClass::ApplyRocker).
//!
//! Pure function over `(rocking, force, weight, dx, dy)` — no entity-store
//! access, no rules dependency. Multiple impulses in the same tick stack
//! additively before the per-axis velocity cap is enforced.

use crate::sim::components::RockingState;
use crate::sim::rocking::rocking_system::{FORCE_SATURATION, IMPULSE_VEL_CAP};
use crate::util::fixed_math::SimFixed;

/// Base force coefficient for the impulse-to-velocity conversion. L31's
/// distance attenuation `(0.04 − dist × 2.5e-5)` is deferred and approximated
/// here as the constant maximum 0.04 — documented parity drift.
const FORCE_COEFFICIENT: SimFixed = SimFixed::lit("0.04");

/// Below this scaled force, the impulse is dropped entirely. Prevents
/// heavy-Weight units from getting a visible-but-tiny twitch from every
/// glancing hit.
const TOO_WEAK_FLOOR: SimFixed = SimFixed::lit("0.01");

/// Forwards-axis dampener: when the no_dampen flag is false (the case at
/// both visible reference-engine call sites), the forwards component is
/// halved before being applied. Sideways is NOT halved — the asymmetry is
/// intentional.
const FORWARDS_DAMPENER: SimFixed = SimFixed::lit("0.5");

/// Default Weight used when the target's INI weight is missing or
/// non-positive. Matches the reference engine's TechnoTypeClass default.
const DEFAULT_WEIGHT: SimFixed = SimFixed::lit("2.0");

/// Apply a rocker impulse to a target unit.
///
/// Computes a direction-aware velocity pair from the source-to-target
/// vector and adds it to `rocking.vel_*`, clamping each axis to
/// `±IMPULSE_VEL_CAP`.
///
/// Parameters:
///   - `rocking`: target's rocking state (mutated in place).
///   - `force`: pre-saturated [0, 4.0] force magnitude from the source-side
///      computation (defensively re-clamped here).
///   - `weight`: target's Weight (default 2.0; retail range 0.5–5). Heavier
///      units rock less per equivalent force. L12c — observable parity:
///      MUST NOT be dropped.
///   - `dx`, `dy`: target_position − source_position in sim units. Only the
///      direction matters; magnitude cancels in the normalization.
///
/// Deferred parity drifts (both documented in the design ledger):
///   - L31: distance attenuation. Approximated here as constant 0.04
///      (the maximum).
///   - L32: rate-timer jitter on impulse direction. Dropped entirely.
pub fn apply_rocker_impulse(
    rocking: &mut RockingState,
    force: SimFixed,
    weight: SimFixed,
    dx: SimFixed,
    dy: SimFixed,
) {
    // Defensive: clamp force to [0, FORCE_SATURATION]. The source side
    // already saturates at 4.0; re-clamp here to catch any wiring error.
    let force = force.clamp(SimFixed::ZERO, FORCE_SATURATION);

    // Defensive: Weight=0 (malformed INI) would divide-by-zero. Treat any
    // non-positive weight as the engine default 2.0.
    let weight = if weight <= SimFixed::ZERO {
        DEFAULT_WEIGHT
    } else {
        weight
    };

    // Normalize the source-to-target direction. Scaling by max(|dx|, |dy|)
    // before squaring keeps the intermediates inside I16F16's ±32767 range
    // regardless of the unit the caller passes (cells, leptons, etc.).
    let max_abs = dx.abs().max(dy.abs());
    if max_abs == SimFixed::ZERO {
        // Source at target — no direction; abort.
        return;
    }
    let sdx = dx / max_abs;
    let sdy = dy / max_abs;
    let dist_sq = sdx * sdx + sdy * sdy;
    let dist = sqrt_approx(dist_sq);
    if dist == SimFixed::ZERO {
        return;
    }
    let nx = sdx / dist;
    let ny = sdy / dist;

    // L12c: 0.04 × force / Weight. Then per-axis cap at 0.05.
    let mut force_scaled = FORCE_COEFFICIENT * force / weight;
    if force_scaled > IMPULSE_VEL_CAP {
        force_scaled = IMPULSE_VEL_CAP;
    }
    // L11: too-weak gate.
    if force_scaled < TOO_WEAK_FLOOR {
        return;
    }

    // L12b: forwards halved, sideways unaltered.
    let vel_fwd = ny * force_scaled * FORWARDS_DAMPENER;
    let vel_side = -nx * force_scaled;

    // Additive stacking with per-axis clamp.
    rocking.vel_forwards =
        (rocking.vel_forwards + vel_fwd).clamp(-IMPULSE_VEL_CAP, IMPULSE_VEL_CAP);
    rocking.vel_sideways =
        (rocking.vel_sideways + vel_side).clamp(-IMPULSE_VEL_CAP, IMPULSE_VEL_CAP);
}

/// Newton-Raphson square root for `SimFixed`. Converges within 6 iterations
/// over the dist-squared range encountered here (always in [0, 2] after the
/// max-abs normalization in `apply_rocker_impulse`).
fn sqrt_approx(x: SimFixed) -> SimFixed {
    if x <= SimFixed::ZERO {
        return SimFixed::ZERO;
    }
    // Start from `x` and iterate. For x in [0, 2] the seed is good enough
    // to converge to ~DELTA precision in 6 steps.
    let mut s = x;
    let two = SimFixed::from_num(2);
    for _ in 0..6 {
        s = (s + x / s) / two;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_approx_known_values() {
        let eps = SimFixed::lit("0.001");
        // sqrt(1) = 1
        assert!((sqrt_approx(SimFixed::ONE) - SimFixed::ONE).abs() < eps);
        // sqrt(2) ≈ 1.4142
        assert!((sqrt_approx(SimFixed::from_num(2)) - SimFixed::lit("1.4142")).abs() < eps);
        // sqrt(0) = 0
        assert_eq!(sqrt_approx(SimFixed::ZERO), SimFixed::ZERO);
    }

    #[test]
    fn negative_or_zero_input_returns_zero() {
        assert_eq!(sqrt_approx(SimFixed::ZERO), SimFixed::ZERO);
        assert_eq!(sqrt_approx(SimFixed::from_num(-1)), SimFixed::ZERO);
    }
}
