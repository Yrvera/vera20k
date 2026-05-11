//! Parachute SHP animation lifecycle — spawn/despawn/advance per render frame.
//!
//! Polling-based lifecycle bound by the target entity's `parachute_state`,
//! not a fixed frame count: a chute exists while the entity is descending and
//! disappears on landing (parachute_state cleared) or death (entity gone).
//!
//! Render-only state; lives on AppState, not in Simulation.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::sim::components::ParachuteAnim;

/// Per-render-frame lifecycle pass.
///
/// Phases:
/// 1. **Despawn:** drop anims whose target entity is missing or whose
///    `parachute_state.is_none()`.
/// 2. **Spawn:** scan entities for any with `parachute_state.is_some()` not
///    yet tracked; push a new anim using `ParachuteRenderConfig`.
/// 3. **Advance:** accumulate `elapsed_ms`; advance `frame` per `rate_ms`;
///    wrap on `frame >= end_frame` to `loop_start`.
pub(crate) fn tick_parachute_anims(state: &mut AppState, dt_ms: u32) {
    let Some(rules) = state.rules.as_ref() else {
        state.parachute_anims.clear();
        return;
    };
    let Some(config) = rules.general.parachute_render.as_ref() else {
        state.parachute_anims.clear();
        return;
    };
    let Some(sim) = state.simulation.as_ref() else {
        state.parachute_anims.clear();
        return;
    };

    // Phase 1: despawn anims whose target is gone or has landed.
    state
        .parachute_anims
        .retain(|anim| match sim.entities.get(anim.target_id) {
            Some(entity) => entity.parachute_state.is_some(),
            None => false,
        });

    // Phase 2: spawn for any descending entity not yet tracked. Collect IDs
    // first so we don't borrow `state.parachute_anims` mutably mid-iteration.
    let new_targets: Vec<u64> = sim
        .entities
        .values()
        .filter(|e| e.parachute_state.is_some())
        .map(|e| e.stable_id)
        .filter(|sid| !state.parachute_anims.iter().any(|a| a.target_id == *sid))
        .collect();

    for target_id in new_targets {
        state.parachute_anims.push(ParachuteAnim {
            target_id,
            frame: 0,
            loop_start: config.loop_start,
            end_frame: config.end_frame,
            rate_ms: config.rate_ms,
            elapsed_ms: 0,
        });
    }

    // Phase 3: advance frames.
    for anim in &mut state.parachute_anims {
        if anim.rate_ms == 0 {
            continue;
        }
        anim.elapsed_ms = anim.elapsed_ms.saturating_add(dt_ms);
        while anim.elapsed_ms >= anim.rate_ms {
            anim.elapsed_ms -= anim.rate_ms;
            anim.frame = anim.frame.saturating_add(1);
            if anim.frame >= anim.end_frame {
                anim.frame = anim.loop_start;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sim::components::ParachuteAnim;

    fn make_anim(target: u64, frame: u16) -> ParachuteAnim {
        ParachuteAnim {
            target_id: target,
            frame,
            loop_start: 20,
            end_frame: 40,
            rate_ms: 133,
            elapsed_ms: 0,
        }
    }

    /// Mirror of phase 3 frame-advance math, exercised in isolation since the
    /// full `tick_parachute_anims` requires a `Simulation` setup. Spawn/despawn
    /// behavior is verified end-to-end in Task 9 (in-game).
    fn advance_frames(anim: &mut ParachuteAnim, dt_ms: u32) {
        if anim.rate_ms == 0 {
            return;
        }
        anim.elapsed_ms = anim.elapsed_ms.saturating_add(dt_ms);
        while anim.elapsed_ms >= anim.rate_ms {
            anim.elapsed_ms -= anim.rate_ms;
            anim.frame = anim.frame.saturating_add(1);
            if anim.frame >= anim.end_frame {
                anim.frame = anim.loop_start;
            }
        }
    }

    #[test]
    fn frame_advances_at_rate_ms_intervals() {
        let mut anim = make_anim(1, 0);
        advance_frames(&mut anim, 132);
        assert_eq!(anim.frame, 0, "below rate_ms threshold should not advance");
        advance_frames(&mut anim, 1); // total 133
        assert_eq!(anim.frame, 1);
    }

    #[test]
    fn frame_wraps_from_end_to_loop_start() {
        let mut anim = make_anim(1, 39); // last valid frame
        advance_frames(&mut anim, 133);
        assert_eq!(anim.frame, 20, "frame should wrap to loop_start (20)");
    }

    #[test]
    fn deploy_phase_plays_frames_0_through_19_once_then_loops_20_to_39() {
        let mut anim = make_anim(1, 0);
        for expected_frame in 1..=39 {
            advance_frames(&mut anim, 133);
            assert_eq!(anim.frame, expected_frame as u16);
        }
        // Next tick wraps to loop_start, NOT to 0.
        advance_frames(&mut anim, 133);
        assert_eq!(
            anim.frame, 20,
            "after frame 39, must wrap to 20 (loop_start), not 0"
        );
        for expected_frame in 21..=39 {
            advance_frames(&mut anim, 133);
            assert_eq!(anim.frame, expected_frame as u16);
        }
        advance_frames(&mut anim, 133);
        assert_eq!(anim.frame, 20, "second loop wraps again");
    }

    #[test]
    fn multiple_frames_per_tick_advance_correctly() {
        let mut anim = make_anim(1, 0);
        // 5 frames worth of dt (5 * 133 = 665ms).
        advance_frames(&mut anim, 665);
        assert_eq!(anim.frame, 5);
    }

    #[test]
    fn zero_rate_does_not_advance_or_panic() {
        let mut anim = ParachuteAnim {
            rate_ms: 0,
            ..make_anim(1, 0)
        };
        advance_frames(&mut anim, 1000);
        assert_eq!(anim.frame, 0, "zero rate must not advance");
    }
}
