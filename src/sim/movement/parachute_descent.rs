//! Parachute descent — per-tick altitude integrator for paradropped infantry.
//!
//! Mirrors the gamemd descent block byte-exact:
//! - rate accumulates by `-1` per tick (integer DEC, not float)
//! - clamps to `Rules.ParachuteMaxFallRate` (default `-3`)
//! - Z integrates as `altitude += rate` per tick (integer leptons)
//! - first tick has `rate == 0` → no movement (3-tick ramp: 0,-1,-2,-3,-3,...)
//! - landing on `altitude <= 0` (inclusive bound)
//! - the infantry keeps its base locomotor and body sequence during descent
//!
//! Sibling of `droppod_movement` and follows the same shape: an `Option<State>`
//! field on `GameEntity`, a `begin_*` entry, a `tick_*` per-tick driver, and
//! cleanup when the object-level falling state lands.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/game_entity, sim/entity_store, sim/locomotor.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::entity_store::EntityStore;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, sim_to_f32};

/// Visual height offset per lepton of altitude (matches DropPod). Render-only f32.
const ALTITUDE_VISUAL_SCALE: f32 = 0.06;

/// Per-entity parachute descent state. Set by [`begin_parachute_descent`],
/// cleared on landing. This mirrors gamemd's object-level falling state:
/// normal paradropped infantry keep their base locomotor and body animation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParachuteDescentState {
    /// Descent rate in leptons/tick. Negative = falling.
    /// Starts at 0; decrements by 1 per tick; clamps to `Rules.ParachuteMaxFallRate`.
    pub rate: i32,
    /// Current altitude in leptons. Decreases by `rate` each tick.
    pub altitude: SimFixed,
}

/// Begin parachute descent for an entity. Returns `true` on success.
///
/// - Initializes state with `rate = 0` (the 3-tick ramp begins on the first tick).
///
/// The entity must already exist in the EntityStore. Caller is responsible
/// for positioning the entity at the desired horizontal coord; `drop_altitude`
/// controls the starting Z.
pub fn begin_parachute_descent(
    entities: &mut EntityStore,
    entity_id: u64,
    drop_altitude: SimFixed,
) -> bool {
    let Some(entity) = entities.get_mut(entity_id) else {
        return false;
    };

    entity.parachute_state = Some(ParachuteDescentState {
        rate: 0,
        altitude: drop_altitude,
    });

    entity.push_debug_event(
        0,
        DebugEventKind::SpecialMovementStart {
            kind: "Parachute".into(),
        },
    );
    true
}

/// Per-tick advance for all entities with `parachute_state`.
///
/// Wired into `World::advance_tick` Phase 2 immediately after
/// `tick_droppod_movement`.
///
/// Per-tick algorithm (mirrors gamemd's descent block):
/// 1. Integrate Z FIRST: `altitude += rate` (rate is negative; first tick rate=0 → no move)
/// 2. Landing check: `altitude <= 0` → mark for cleanup (altitude clamped to exactly 0)
/// 3. Rate update: `rate -= 1`, clamp to `parachute_max_fall_rate`
/// 4. Update `screen_y` for renderer (altitude offset, render-only f32)
///
/// Cleanup (per landed entity):
/// - clear `parachute_state`
pub fn tick_parachute_descent(
    entities: &mut EntityStore,
    tick_ms: u32,
    parachute_max_fall_rate: i32,
    sim_tick: u64,
) {
    if tick_ms == 0 {
        return;
    }

    let mut finished: Vec<u64> = Vec::new();

    let keys = entities.keys_sorted();
    for &id in &keys {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        let Some(ref mut state) = entity.parachute_state else {
            continue;
        };

        // Integrate Z FIRST. On the very first tick `rate == 0`, so altitude
        // doesn't change yet — that produces the 3-tick ramp 0,-1,-2,-3.
        state.altitude += SimFixed::from_num(state.rate);

        // Landing on `altitude <= 0` (inclusive). Clamp to exactly SIM_ZERO
        // so render position never shows the unit below ground for a frame.
        if state.altitude <= SIM_ZERO {
            state.altitude = SIM_ZERO;
            finished.push(id);
        } else {
            // Integer DEC, then clamp toward the more-negative bound.
            state.rate = (state.rate - 1).max(parachute_max_fall_rate);
        }

        // Render-side: update screen_y with altitude offset. Render-only f32
        // — does NOT feed back into sim state.
        let (sx, sy) = crate::util::lepton::lepton_to_screen(
            entity.position.rx,
            entity.position.ry,
            entity.position.sub_x,
            entity.position.sub_y,
            entity.position.z,
        );
        entity.position.screen_x = sx;
        entity.position.screen_y = sy - sim_to_f32(state.altitude) * ALTITUDE_VISUAL_SCALE;
    }

    // Cleanup landed entities: clear state BEFORE end_override (order matters
    // — anything that watches the override transition must see a coherent
    // (no descent state, restored locomotor) snapshot).
    for id in finished {
        if let Some(entity) = entities.get_mut(id) {
            entity.parachute_state = None;
            entity.push_debug_event(sim_tick as u32, DebugEventKind::SpecialMovementEnd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::sim::animation::{Animation, SequenceKind};
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::locomotor::{
        AirMovePhase, GroundMovePhase, LocomotorState, MovementLayer,
    };
    use crate::util::fixed_math::{SIM_ONE, SIM_ZERO};

    /// Mirrors the helper used in droppod_movement.rs tests.
    fn make_walk_loco() -> LocomotorState {
        LocomotorState {
            kind: LocomotorKind::Walk,
            primary_kind: Some(LocomotorKind::Walk),
            piggyback: None,
            layer: MovementLayer::Ground,
            phase: GroundMovePhase::Idle,
            air_phase: AirMovePhase::Landed,
            speed_multiplier: SIM_ONE,
            speed_fraction: SIM_ONE,
            fly_current_speed: SIM_ZERO,
            altitude: SIM_ZERO,
            target_altitude: SIM_ZERO,
            climb_rate: SIM_ZERO,
            jumpjet_speed: SIM_ZERO,
            jumpjet_wobbles: 0.0,
            jumpjet_accel: SIM_ZERO,
            jumpjet_current_speed: SIM_ZERO,
            jumpjet_deviation: 0,
            jumpjet_crash_speed: SIM_ZERO,
            jumpjet_turn_rate: 4,
            balloon_hover: false,
            hover_attack: false,
            speed_type: SpeedType::Foot,
            movement_zone: MovementZone::Normal,
            rot: 0,
            override_state: None,
            air_progress: SIM_ZERO,
            infantry_wobble_phase: 0.0,
            subcell_dest: None,
        }
    }

    fn drop_altitude_1200() -> SimFixed {
        SimFixed::from_num(1200)
    }

    /// Build an infantry entity with a Walk locomotor and a Stand animation,
    /// inserted into `entities`. Returns the entity id.
    fn insert_test_infantry(entities: &mut EntityStore, id: u64) -> u64 {
        let mut e = GameEntity::test_default(id, "E1", "Americans", 10, 10);
        e.locomotor = Some(make_walk_loco());
        e.animation = Some(Animation::new(SequenceKind::Stand));
        entities.insert(e);
        id
    }

    #[test]
    fn test_begin_attaches_state_and_keeps_locomotor_identity() {
        let mut entities = EntityStore::new();
        let id = insert_test_infantry(&mut entities, 1);

        assert!(begin_parachute_descent(
            &mut entities,
            id,
            drop_altitude_1200()
        ));

        let entity = entities.get(id).expect("should exist");
        let state = entity
            .parachute_state
            .as_ref()
            .expect("parachute state must be attached");
        assert_eq!(
            state.rate, 0,
            "rate must start at 0 (3-tick ramp begins next tick)"
        );
        assert_eq!(state.altitude, drop_altitude_1200());

        let loco = entity.locomotor.as_ref().expect("has loco");
        assert!(
            !loco.is_overridden(),
            "ordinary paradropped infantry keep their base locomotor"
        );
        assert_eq!(loco.kind, LocomotorKind::Walk);
        assert_eq!(
            loco.layer,
            MovementLayer::Ground,
            "object-level falling state must not rewrite locomotor layer"
        );
    }

    #[test]
    fn test_body_sequence_preserved_on_begin() {
        let mut entities = EntityStore::new();
        let id = insert_test_infantry(&mut entities, 1);

        begin_parachute_descent(&mut entities, id, drop_altitude_1200());

        let anim = entities
            .get(id)
            .expect("alive")
            .animation
            .as_ref()
            .expect("has anim");
        assert_eq!(
            anim.sequence,
            SequenceKind::Stand,
            "normal paradrops render the attached PARACH anim, not body Paradrop frames"
        );
    }

    #[test]
    fn test_begin_works_without_locomotor() {
        // Mirrors test_droppod_without_loco_still_works.
        let mut entities = EntityStore::new();
        let mut e = GameEntity::test_default(1, "E1", "Americans", 5, 5);
        // No locomotor.
        e.animation = Some(Animation::new(SequenceKind::Stand));
        entities.insert(e);

        assert!(begin_parachute_descent(
            &mut entities,
            1,
            drop_altitude_1200()
        ));

        let entity = entities.get(1).expect("alive");
        assert!(entity.parachute_state.is_some());
    }

    #[test]
    fn test_begin_returns_false_for_missing_entity() {
        let mut entities = EntityStore::new();
        assert!(!begin_parachute_descent(
            &mut entities,
            999,
            drop_altitude_1200()
        ));
    }

    // -------------------------------------------------------------------
    // Parity tests — verify the descent state machine matches gamemd
    // exactly. See JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md Round 4 §R4.7.
    // -------------------------------------------------------------------

    /// Default INI value per `[General] ParachuteMaxFallRate=-3`.
    const RULES_PARACHUTE_MAX_FALL_RATE: i32 = -3;
    /// Standard sim tick at 15 fps (~64ms). Specific value isn't sensitive —
    /// `tick_parachute_descent` only checks `tick_ms != 0` (pause guard).
    const TICK_MS_64: u32 = 64;

    /// Set up an entity at id=1, attach a parachute descent at the given altitude.
    /// Returns the id.
    fn setup_parachuting_entity(entities: &mut EntityStore, drop_altitude: SimFixed) -> u64 {
        let id = insert_test_infantry(entities, 1);
        begin_parachute_descent(entities, id, drop_altitude);
        id
    }

    #[test]
    fn test_3tick_rate_ramp() {
        // Rate sequence over 6 ticks must be exactly [0, -1, -2, -3, -3, -3].
        // Sample BEFORE each tick (= rate-in for that tick).
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, drop_altitude_1200());

        let mut observed: Vec<i32> = Vec::new();
        for _ in 0..6 {
            let entity = entities.get(id).expect("alive");
            let state = entity.parachute_state.as_ref().expect("descending");
            observed.push(state.rate);
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        }

        assert_eq!(
            observed,
            vec![0, -1, -2, -3, -3, -3],
            "3-tick ramp must be 0,-1,-2,-3,-3,-3 (NOT instant -3)"
        );
    }

    #[test]
    fn test_descent_distance_first_4_ticks() {
        // Total descent over the first N ticks (deltas vs initial):
        //   tick 1: 0  (rate was 0 at integration)
        //   tick 2: 1  (rate was -1 at integration)
        //   tick 3: 3  (rate was -2 at integration)
        //   tick 4: 6  (rate was -3 at integration)
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, drop_altitude_1200());

        let initial_altitude = entities
            .get(id)
            .unwrap()
            .parachute_state
            .as_ref()
            .unwrap()
            .altitude;

        let expected_deltas: [i32; 4] = [0, 1, 3, 6];
        for (i, expected_delta) in expected_deltas.iter().enumerate() {
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
            let altitude = entities
                .get(id)
                .unwrap()
                .parachute_state
                .as_ref()
                .unwrap()
                .altitude;
            let descent = initial_altitude - altitude;
            let expected = SimFixed::from_num(*expected_delta);
            assert_eq!(
                descent,
                expected,
                "after tick {} descent should be {} leptons (got {})",
                i + 1,
                expected_delta,
                descent
            );
        }
    }

    #[test]
    fn test_steady_state_rate() {
        // After enough ticks past the ramp, rate stays clamped at -3.
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, drop_altitude_1200());

        for _ in 0..10 {
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        }
        let rate = entities
            .get(id)
            .unwrap()
            .parachute_state
            .as_ref()
            .unwrap()
            .rate;
        assert_eq!(
            rate, RULES_PARACHUTE_MAX_FALL_RATE,
            "steady-state rate must equal ParachuteMaxFallRate"
        );
    }

    #[test]
    fn test_landing_inclusive_zero() {
        // Drop altitude = 6 leptons → tick 4 integrates altitude = 0 → landing
        // triggers (inclusive bound). `parachute_state` must be cleared.
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, SimFixed::from_num(6));

        for _ in 0..4 {
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        }

        let entity = entities.get(id).expect("alive");
        assert!(
            entity.parachute_state.is_none(),
            "landing at altitude == 0 must trigger cleanup"
        );
    }

    #[test]
    fn test_landing_clamps_to_zero_no_overshoot() {
        // Drop altitude = 5 leptons → tick 4 integrates altitude = -1 (would
        // overshoot below ground), must clamp to exactly SIM_ZERO before cleanup.
        // We can't observe altitude after cleanup; verify via screen_y offset.
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, SimFixed::from_num(5));

        for _ in 0..4 {
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        }

        let entity = entities.get(id).expect("alive");
        assert!(entity.parachute_state.is_none(), "must have landed");
        // Compute the no-altitude baseline screen_y the way the tick does.
        let (_sx, sy_baseline) = crate::util::lepton::lepton_to_screen(
            entity.position.rx,
            entity.position.ry,
            entity.position.sub_x,
            entity.position.sub_y,
            entity.position.z,
        );
        // The last tick's screen_y was computed with altitude = SIM_ZERO →
        // screen_y == sy - 0 == sy.
        assert!(
            (entity.position.screen_y - sy_baseline).abs() < 0.01,
            "screen_y must equal baseline (no altitude offset) after landing"
        );
    }

    #[test]
    fn test_clamp_at_max_fall_rate_default() {
        // Rate must never exceed (be more-negative than) ParachuteMaxFallRate.
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, drop_altitude_1200());

        for _ in 0..50 {
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
            if let Some(state) = entities.get(id).and_then(|e| e.parachute_state.as_ref()) {
                assert!(
                    state.rate >= RULES_PARACHUTE_MAX_FALL_RATE,
                    "rate {} must not exceed (more-negative than) max {}",
                    state.rate,
                    RULES_PARACHUTE_MAX_FALL_RATE
                );
            }
        }
    }

    #[test]
    fn test_clamp_with_custom_max_fall_rate() {
        // Mod-friendliness: a non-default `parachute_max_fall_rate` must be
        // respected. With max = -1, rate ramp is 0 → -1 → -1 → -1.
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, drop_altitude_1200());

        let custom_max: i32 = -1;
        let mut observed: Vec<i32> = Vec::new();
        for _ in 0..4 {
            let rate = entities
                .get(id)
                .unwrap()
                .parachute_state
                .as_ref()
                .unwrap()
                .rate;
            observed.push(rate);
            tick_parachute_descent(&mut entities, TICK_MS_64, custom_max, 0);
        }
        assert_eq!(
            observed,
            vec![0, -1, -1, -1],
            "with max=-1, rate must clamp at -1 after the first decrement"
        );
    }

    #[test]
    fn test_body_sequence_preserved_on_landing() {
        // Normal paradropped infantry do not switch to the body Paradrop
        // sequence, so landing should not rewrite the body animation either.
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, SimFixed::from_num(6));

        assert_eq!(
            entities
                .get(id)
                .unwrap()
                .animation
                .as_ref()
                .unwrap()
                .sequence,
            SequenceKind::Stand
        );

        for _ in 0..4 {
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        }

        let anim = entities.get(id).unwrap().animation.as_ref().unwrap();
        assert_eq!(
            anim.sequence,
            SequenceKind::Stand,
            "landing must preserve the unchanged body sequence"
        );
    }

    #[test]
    fn test_body_sequence_preserved_if_externally_changed() {
        // If some other system changed the sequence during descent (e.g., a
        // death anim took over), don't overwrite on landing.
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, SimFixed::from_num(6));

        // Mid-descent, externally change to Die1 (simulating shot down in air).
        for _ in 0..2 {
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        }
        entities
            .get_mut(id)
            .unwrap()
            .animation
            .as_mut()
            .unwrap()
            .switch_to(SequenceKind::Die1);

        // Continue ticking through landing.
        for _ in 0..4 {
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        }

        let anim = entities.get(id).unwrap().animation.as_ref().unwrap();
        assert_eq!(
            anim.sequence,
            SequenceKind::Die1,
            "must NOT overwrite Die1 with Stand on landing"
        );
    }

    #[test]
    fn test_locomotor_identity_preserved_through_landing() {
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, SimFixed::from_num(6));

        {
            let loco = entities.get(id).unwrap().locomotor.as_ref().unwrap();
            assert!(!loco.is_overridden());
            assert_eq!(loco.kind, LocomotorKind::Walk);
        }

        for _ in 0..4 {
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        }

        let loco = entities.get(id).unwrap().locomotor.as_ref().unwrap();
        assert!(
            !loco.is_overridden(),
            "object-level falling must not leave a locomotor override"
        );
        assert_eq!(
            loco.kind,
            LocomotorKind::Walk,
            "must preserve base Walk locomotor"
        );
    }

    #[test]
    fn test_works_without_animation() {
        // begin and tick must not panic when entity.animation is None.
        let mut entities = EntityStore::new();
        let mut e = GameEntity::test_default(1, "E1", "Americans", 5, 5);
        e.locomotor = Some(make_walk_loco());
        e.animation = None;
        entities.insert(e);

        assert!(begin_parachute_descent(
            &mut entities,
            1,
            SimFixed::from_num(6)
        ));
        for _ in 0..10 {
            tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        }
        let entity = entities.get(1).unwrap();
        assert!(
            entity.parachute_state.is_none(),
            "should land cleanly without an animation field"
        );
    }

    #[test]
    fn test_paused_tick_does_not_advance() {
        // tick_ms == 0 (paused) must not advance state.
        let mut entities = EntityStore::new();
        let id = setup_parachuting_entity(&mut entities, drop_altitude_1200());
        let initial_alt = entities
            .get(id)
            .unwrap()
            .parachute_state
            .as_ref()
            .unwrap()
            .altitude;
        let initial_rate = entities
            .get(id)
            .unwrap()
            .parachute_state
            .as_ref()
            .unwrap()
            .rate;

        tick_parachute_descent(&mut entities, 0, RULES_PARACHUTE_MAX_FALL_RATE, 0);

        let after = entities.get(id).unwrap().parachute_state.as_ref().unwrap();
        assert_eq!(
            after.altitude, initial_alt,
            "paused tick must not move altitude"
        );
        assert_eq!(after.rate, initial_rate, "paused tick must not update rate");
    }
}
