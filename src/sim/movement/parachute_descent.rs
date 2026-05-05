//! Parachute descent — per-tick altitude integrator for paradropped infantry.
//!
//! Mirrors the gamemd descent block byte-exact:
//! - rate accumulates by `-1` per tick (integer DEC, not float)
//! - clamps to `Rules.ParachuteMaxFallRate` (default `-3`)
//! - Z integrates as `altitude += rate` per tick (integer leptons)
//! - first tick has `rate == 0` → no movement (3-tick ramp: 0,-1,-2,-3,-3,...)
//! - landing on `altitude <= 0` (inclusive bound)
//! - body sequence set to `Paradrop` on attach, reset to `Stand` on landing
//!
//! Sibling of `droppod_movement` and follows the same shape: an `Option<State>`
//! field on `GameEntity`, a `begin_*` entry, a `tick_*` per-tick driver, and
//! cleanup via the locomotor override stack.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/game_entity, sim/entity_store, sim/locomotor.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::animation::SequenceKind;
use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::locomotor::OverrideKind;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, sim_to_f32};

/// Visual height offset per lepton of altitude (matches DropPod). Render-only f32.
const ALTITUDE_VISUAL_SCALE: f32 = 0.06;

/// Per-entity parachute descent state. Set by [`begin_parachute_descent`],
/// cleared on landing. While `Some`, the entity's locomotor is overridden
/// (`OverrideKind::Parachute`) so it does not occupy ground cells.
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
/// - Applies `OverrideKind::Parachute` to suppress the base locomotor
///   (entity does not occupy ground cells while descending).
/// - Initializes state with `rate = 0` (the 3-tick ramp begins on the first tick).
/// - Sets the body animation sequence to `Paradrop` (held until landing) — once,
///   at attach time. Per-tick re-set would freeze the frame counter.
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

    if let Some(ref mut loco) = entity.locomotor {
        loco.begin_override(OverrideKind::Parachute);
    }

    entity.parachute_state = Some(ParachuteDescentState {
        rate: 0,
        altitude: drop_altitude,
    });

    // Body sequence trigger: once at attach, never per-tick.
    if let Some(ref mut anim) = entity.animation {
        anim.switch_to(SequenceKind::Paradrop);
    }

    entity.push_debug_event(
        0,
        DebugEventKind::SpecialMovementStart {
            kind: "Parachute".into(),
        },
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::sim::animation::Animation;
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
    fn test_begin_attaches_state_and_overrides_locomotor() {
        let mut entities = EntityStore::new();
        let id = insert_test_infantry(&mut entities, 1);

        assert!(begin_parachute_descent(&mut entities, id, drop_altitude_1200()));

        let entity = entities.get(id).expect("should exist");
        let state = entity
            .parachute_state
            .as_ref()
            .expect("parachute state must be attached");
        assert_eq!(state.rate, 0, "rate must start at 0 (3-tick ramp begins next tick)");
        assert_eq!(state.altitude, drop_altitude_1200());

        let loco = entity.locomotor.as_ref().expect("has loco");
        assert!(loco.is_overridden(), "locomotor must be overridden during descent");
        assert_eq!(loco.kind, LocomotorKind::Parachute);
        assert_eq!(
            loco.layer,
            MovementLayer::Air,
            "Air layer means no ground occupancy is marked while descending"
        );
    }

    #[test]
    fn test_body_sequence_set_on_begin() {
        let mut entities = EntityStore::new();
        let id = insert_test_infantry(&mut entities, 1);

        begin_parachute_descent(&mut entities, id, drop_altitude_1200());

        let anim = entities
            .get(id)
            .expect("alive")
            .animation
            .as_ref()
            .expect("has anim");
        assert_eq!(anim.sequence, SequenceKind::Paradrop);
    }

    #[test]
    fn test_begin_works_without_locomotor() {
        // Mirrors test_droppod_without_loco_still_works.
        let mut entities = EntityStore::new();
        let mut e = GameEntity::test_default(1, "E1", "Americans", 5, 5);
        // No locomotor.
        e.animation = Some(Animation::new(SequenceKind::Stand));
        entities.insert(e);

        assert!(begin_parachute_descent(&mut entities, 1, drop_altitude_1200()));

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
}
