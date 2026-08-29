//! Bounded reciprocal-owner state and SQD Attach's crate-sensitive Ship seam.
//!
//! The full Parasite/SqdGrapple lifecycle belongs to the Ship mechanism.  This
//! module owns only the prerequisite proven by `ParasiteClass::Attach @
//! 0x0062A980`: Ship ForceTrack may synchronously mutate or UnInit the limboed
//! SQD, after which Attach still writes the victim backlink and then the
//! manager victim pointer against the deferred-lifetime tombstone.

use crate::sim::components::DriveCoord;
use crate::sim::crates::NativePickupReturn;
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::crate_callers::{
    MovementCrateCallsite, MovementCrateProbe, continue_after_pickup,
};

/// Minimal persistent owner-manager shape required by the crate prerequisite.
/// The later Ship/SQD mechanism extends this type with its timers/FSM/visual
/// state; the reciprocal identity remains owned here rather than migrating to
/// a second representation.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ParasiteManagerState {
    #[serde(default)]
    pub victim_id: Option<u64>,
    /// WarpAttach detach timer start written by Grinder at manager+0x2C.
    #[serde(default = "detached_timer_default")]
    pub detach_started_frame: i32,
    /// WarpAttach detach duration written as literal 0x32 by Grinder.
    #[serde(default)]
    pub detach_duration_frames: i32,
}

const fn detached_timer_default() -> i32 {
    -1
}

impl Default for ParasiteManagerState {
    fn default() -> Self {
        Self {
            victim_id: None,
            detach_started_frame: -1,
            detach_duration_frames: 0,
        }
    }
}

/// Install Ship `ForceTrack(-1, victim_xyz)` through its exact native entry.
///
/// This writes track facing `+0x54=-1`, track index `+0x58=0`, replaces the
/// destination, and returns the one exact `ShipForceTrack` probe.  The caller
/// must release all entity borrows before dispatching it synchronously.
pub(crate) fn begin_sqd_ship_force_track(
    entities: &mut EntityStore,
    attacker_id: u64,
    victim_id: u64,
) -> Option<MovementCrateProbe> {
    let requested = entities.get(victim_id).map(MovementCrateProbe::current_coord)?;
    let attacker = entities.get_mut(attacker_id)?;
    attacker.parasite_manager.as_ref()?;
    let ship = attacker.ship_locomotion.as_mut()?;

    ship.track_facing = -1;
    ship.track_index = 0;
    ship.destination = None;
    ship.destination = Some(requested);

    Some(MovementCrateProbe {
        callsite: MovementCrateCallsite::ShipForceTrack,
        requested,
        saved_current_speed_fraction: attacker.current_speed_fraction,
    })
}

/// Resume native Attach after Ship ForceTrack's synchronous crate callback.
///
/// Physical deletion cannot run between pickup and this continuation.  An
/// UnInit collector remains a stable-ID tombstone, which is deliberately
/// mutated here: ForceTrack first completes its own alive/limbo-dependent tail,
/// then Attach stores victim backlink followed by manager victim regardless of
/// the callback's lifetime outcome.
pub(crate) fn finish_sqd_attach_after_ship_force_track(
    entities: &mut EntityStore,
    attacker_id: u64,
    victim_id: u64,
    probe: MovementCrateProbe,
    pickup: NativePickupReturn,
) {
    debug_assert_eq!(probe.callsite, MovementCrateCallsite::ShipForceTrack);
    let attacker = entities
        .get_mut(attacker_id)
        .expect("Ship ForceTrack collector remains a deferred-lifetime tombstone");
    let _ = continue_after_pickup(attacker, probe, pickup);

    // `ParasiteClass::Attach`: victim +0x694 first ...
    entities
        .get_mut(victim_id)
        .expect("SQD Attach victim remains resolvable on its native stack")
        .parasite_attacker_id = Some(attacker_id);
    // ... then manager +0x28, even when pickup UnInit the limboed owner.
    entities
        .get_mut(attacker_id)
        .expect("SQD manager owner remains a deferred-lifetime tombstone")
        .parasite_manager
        .as_mut()
        .expect("admitted SQD Attach retains its manager through continuation")
        .victim_id = Some(victim_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::components::ShipLocomotionRuntime;
    use crate::sim::game_entity::GameEntity;
    use crate::util::fixed_math::SimFixed;
    use crate::util::native_x87::NativeF64Bits;

    fn pair() -> EntityStore {
        let mut entities = EntityStore::new();
        let mut attacker = GameEntity::test_default(1, "SQD", "Russians", 3, 4);
        attacker.lifecycle.in_limbo = true;
        attacker.parasite_manager = Some(ParasiteManagerState::default());
        attacker.ship_locomotion = Some(ShipLocomotionRuntime {
            destination: Some(DriveCoord::cell(30, 30, 1)),
            head_to: Some(DriveCoord::cell(31, 31, 2)),
            target_speed_fraction: NativeF64Bits::from_bits(0.25_f64.to_bits()),
            ..Default::default()
        });
        entities.insert(attacker);

        let mut victim = GameEntity::test_default(2, "DEST", "Americans", 8, 9);
        victim.position.sub_x = SimFixed::from_num(17);
        victim.position.sub_y = SimFixed::from_num(29);
        victim.position.exact_z_leptons = Some(333);
        entities.insert(victim);
        entities
    }

    #[test]
    fn sqd_ship_force_track_is_distinct_and_links_after_dead_limbo_callback() {
        let mut entities = pair();
        let probe = begin_sqd_ship_force_track(&mut entities, 1, 2).expect("Ship ForceTrack");
        assert_eq!(probe.callsite, MovementCrateCallsite::ShipForceTrack);
        assert_eq!(probe.requested, DriveCoord { x: 8 * 256 + 17, y: 9 * 256 + 29, z: 333 });
        let ship = entities.get(1).unwrap().ship_locomotion.as_ref().unwrap();
        assert_eq!(ship.track_facing, -1);
        assert_eq!(ship.track_index, 0);
        assert_eq!(ship.destination, Some(probe.requested));

        let attacker = entities.get_mut(1).unwrap();
        attacker.lifecycle.object_alive = false;
        attacker.dying = true;
        finish_sqd_attach_after_ship_force_track(
            &mut entities,
            1,
            2,
            probe,
            NativePickupReturn::One,
        );

        assert_eq!(
            entities.get(1).unwrap().ship_locomotion.as_ref().unwrap().destination,
            Some(probe.requested),
            "Explosion-like One/dead/limbo retains installed destination"
        );
        assert_eq!(entities.get(2).unwrap().parasite_attacker_id, Some(1));
        assert_eq!(
            entities.get(1).unwrap().parasite_manager.as_ref().unwrap().victim_id,
            Some(2)
        );
    }

    #[test]
    fn sqd_attach_force_track_callback_matrix_preserves_native_tail_and_links() {
        for (name, pickup, alive, limbo) in [
            ("event49-death", NativePickupReturn::Zero, false, true),
            ("explosion-death", NativePickupReturn::One, false, true),
            ("unit-success", NativePickupReturn::Zero, true, false),
            ("moved-unlimbo", NativePickupReturn::One, true, false),
        ] {
            let mut entities = pair();
            let probe = begin_sqd_ship_force_track(&mut entities, 1, 2).unwrap();
            let callback_destination = DriveCoord::cell(40, 41, 7);
            {
                let attacker = entities.get_mut(1).unwrap();
                attacker.lifecycle.object_alive = alive;
                attacker.lifecycle.in_limbo = limbo;
                attacker.position.rx = 20;
                attacker.position.ry = 21;
                attacker.position.sub_x = SimFixed::from_num(11);
                attacker.position.sub_y = SimFixed::from_num(13);
                attacker.ship_locomotion.as_mut().unwrap().destination = Some(callback_destination);
            }

            finish_sqd_attach_after_ship_force_track(&mut entities, 1, 2, probe, pickup);
            let attacker = entities.get(1).unwrap();
            let ship = attacker.ship_locomotion.as_ref().unwrap();
            let success = pickup == NativePickupReturn::One && !limbo;
            let expected_destination = if success || !alive {
                Some(callback_destination)
            } else {
                None
            };
            assert_eq!(ship.destination, expected_destination, "{name}: destination");
            if success {
                assert_eq!(
                    (attacker.position.rx, attacker.position.ry),
                    (8, 9),
                    "{name}: raw original request apply"
                );
                assert_eq!(ship.head_to, Some(probe.requested), "{name}: head-to");
                assert_eq!(
                    ship.target_speed_fraction,
                    NativeF64Bits::ONE,
                    "{name}: locomotor target qword"
                );
            } else {
                assert_eq!(
                    (attacker.position.rx, attacker.position.ry),
                    (20, 21),
                    "{name}: no raw step"
                );
            }
            assert_eq!(entities.get(2).unwrap().parasite_attacker_id, Some(1), "{name}");
            assert_eq!(attacker.parasite_manager.as_ref().unwrap().victim_id, Some(2), "{name}");
        }
    }

    #[test]
    fn sqd_reciprocal_ids_and_ship_track_facing_are_world_hash_authority() {
        fn hash(
            manager_victim: Option<u64>,
            victim_attacker: Option<u64>,
            track_facing: i32,
        ) -> u64 {
            let mut sim = crate::sim::world::Simulation::new();
            sim.substrate.entities = pair();
            let attacker = sim.substrate.entities.get_mut(1).unwrap();
            attacker.parasite_manager.as_mut().unwrap().victim_id = manager_victim;
            attacker.ship_locomotion.as_mut().unwrap().track_facing = track_facing;
            sim.substrate.entities.get_mut(2).unwrap().parasite_attacker_id = victim_attacker;
            sim.state_hash()
        }

        let baseline = hash(None, None, 0);
        assert_ne!(hash(Some(2), None, 0), baseline, "manager victim is hashed");
        assert_ne!(hash(None, Some(1), 0), baseline, "victim backlink is hashed");
        assert_ne!(hash(None, None, -1), baseline, "Ship +0x54 is hashed");
    }
}
