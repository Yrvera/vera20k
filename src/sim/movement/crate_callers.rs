//! Typed suspension records and same-stack continuations for the thirteen
//! active-YR movement callers of `CrateClass::PickupDispatch`.
//!
//! A movement leaf records immutable native inputs while it owns the locomotor
//! borrow. `Simulation` then releases that borrow, dispatches pickup, re-fetches
//! the stable-ID/tombstone, and invokes the continuation below. This is not a
//! generic cell-change hook: callers without a verified native xref never
//! construct a probe.

use crate::sim::components::DriveCoord;
use crate::sim::crates::NativePickupReturn;
use crate::sim::game_entity::GameEntity;
use crate::rules::locomotor_type::LocomotorKind;
use super::locomotor::MovementLayer;
use crate::util::fixed_math::{SIM_ZERO, SimFixed};
use crate::util::native_x87::NativeF64Bits;

/// The exact live xref set to `CrateClass__PickupDispatch @ 0x00481A00`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MovementCrateCallsite {
    HoverMovement,
    JumpjetMovement,
    JumpjetDescend,
    DriveForceTrack,
    DriveProcessDriveTrack,
    DriveProcessMovementFirst,
    DriveProcessMovementFinal,
    ShipForceTrack,
    ShipProcessDriveTrack,
    ShipProcessMovementFirst,
    ShipProcessMovementFinal,
    TeleportArrival,
    WalkFindSubCellDest,
}

/// Pass-local suspension record. It is never serialized or hashed because a
/// native pickup call and its caller continuation are one synchronous stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MovementCrateProbe {
    pub(crate) callsite: MovementCrateCallsite,
    pub(crate) requested: DriveCoord,
    pub(crate) saved_current_speed_fraction: NativeF64Bits,
}

/// Stored ProcessDriveTrack tail while its synchronous pickup callback owns
/// the Simulation. `admitted` is set only by the One/unlimbo continuation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DriveTrackPickupResume {
    pub(crate) advance: super::drive_track::DriveTrackAdvance,
    pub(crate) admitted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessMovementPickupStage {
    First,
    Final,
}

/// Immutable ProcessMovement selection inputs retained across an Event-49
/// callback. In particular, callback movement/repath may not replace the
/// original curve endpoint or its descriptor-gated first candidate.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProcessMovementPickupResume {
    pub(crate) stage: ProcessMovementPickupStage,
    pub(crate) plan: super::drive_track::DriveTrackPlan,
    pub(crate) shared_kind: LocomotorKind,
    pub(crate) origin_layer: MovementLayer,
    pub(crate) endpoint: (i16, i16),
    pub(crate) endpoint_layer: MovementLayer,
    pub(crate) endpoint_coord: DriveCoord,
    /// Original RawTrack handoff retained across the synchronous callback.
    /// Callback repath/retarget must not make the final native occupation call
    /// derive a different track role.
    pub(crate) handoff_occupation: Option<crate::sim::components::DriveOccupationFootprint>,
    pub(crate) saved_current_speed_fraction: NativeF64Bits,
    pub(crate) admitted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AirPickupStage {
    JumpjetMovement,
    JumpjetDescend,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AirPickupResume {
    pub(crate) stage: AirPickupStage,
    pub(crate) final_goal: Option<(u16, u16)>,
    pub(crate) arrived: bool,
    pub(crate) install_post_callback_current: bool,
    pub(crate) admitted: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TeleportPickupResume {
    pub(crate) being_warped_ticks: u32,
    pub(crate) admitted: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum GroundCrossingPickupStage {
    Hover {
        direction: (i8, i8),
        next_layer: MovementLayer,
    },
    Walk {
        committed_cell: (u16, u16),
        active_layer: MovementLayer,
        pending_bridge_update: super::movement_bridge::BridgeStateUpdate,
        projected_on_bridge: bool,
        runtime_bridge_transition:
            super::movement_bridge::RuntimeBridgeTransitionState,
    },
}

/// Exact continuation state for Hover's pre-CanEnter pickup and Walk's
/// post-reservation FindSubCellDest pickup. These two xrefs live inside the
/// cell-crossing leaf and must not replay advancement or occupancy on resume.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GroundCrossingPickupResume {
    pub(super) stage: GroundCrossingPickupStage,
    pub(super) admitted: bool,
}

impl MovementCrateProbe {
    pub(crate) fn current_coord(entity: &GameEntity) -> DriveCoord {
        DriveCoord {
            x: i32::from(entity.position.rx) * 256 + entity.position.sub_x.to_num::<i32>(),
            y: i32::from(entity.position.ry) * 256 + entity.position.sub_y.to_num::<i32>(),
            z: entity
                .position
                .exact_z_leptons
                .unwrap_or_else(|| i32::from(entity.position.z)),
        }
    }

    pub(crate) fn at_entity(callsite: MovementCrateCallsite, entity: &GameEntity) -> Self {
        Self {
            callsite,
            requested: Self::current_coord(entity),
            saved_current_speed_fraction: entity.current_speed_fraction,
        }
    }

    pub(crate) fn cell(self) -> Option<(u16, u16)> {
        let rx = u16::try_from(self.requested.x.div_euclid(256)).ok()?;
        let ry = u16::try_from(self.requested.y.div_euclid(256)).ok()?;
        Some((rx, ry))
    }
}

fn clear_drive_destination(entity: &mut GameEntity, ship: bool) {
    if ship {
        if let Some(runtime) = entity.ship_locomotion.as_mut() {
            runtime.destination = None;
        }
    } else if let Some(runtime) = entity.drive_locomotion.as_mut() {
        runtime.destination = None;
        runtime.track_valid = false;
    }
}

fn raw_apply_requested(entity: &mut GameEntity, requested: DriveCoord) {
    let rx = requested.x.div_euclid(256);
    let ry = requested.y.div_euclid(256);
    if let (Ok(rx), Ok(ry)) = (u16::try_from(rx), u16::try_from(ry)) {
        entity.position.rx = rx;
        entity.position.ry = ry;
        entity.position.sub_x = SimFixed::from_num(requested.x.rem_euclid(256));
        entity.position.sub_y = SimFixed::from_num(requested.y.rem_euclid(256));
        entity.position.exact_z_leptons = Some(requested.z);
    }
}

/// Apply the caller tail after synchronous pickup and stable-ID re-fetch.
///
/// The bool is caller control flow, never a crate-consumed flag.
pub(crate) fn continue_after_pickup(
    entity: &mut GameEntity,
    probe: MovementCrateProbe,
    pickup: NativePickupReturn,
) -> bool {
    let alive = entity.lifecycle.object_alive;
    let limbo = entity.lifecycle.in_limbo;
    match probe.callsite {
        MovementCrateCallsite::DriveForceTrack | MovementCrateCallsite::ShipForceTrack => {
            let ship = probe.callsite == MovementCrateCallsite::ShipForceTrack;
            if pickup == NativePickupReturn::One && !limbo {
                // Success performs raw owner/head/target writes even on a dead
                // tombstone, but does not reinstall a callback destination.
                raw_apply_requested(entity, probe.requested);
                if ship {
                    if let Some(runtime) = entity.ship_locomotion.as_mut() {
                        runtime.head_to = Some(probe.requested);
                        runtime.target_speed_fraction = NativeF64Bits::ONE;
                    }
                } else if let Some(runtime) = entity.drive_locomotion.as_mut() {
                    runtime.head_to = Some(probe.requested);
                    runtime.target_speed_fraction = NativeF64Bits::ONE;
                }
                true
            } else {
                if alive {
                    clear_drive_destination(entity, ship);
                }
                false
            }
        }
        MovementCrateCallsite::DriveProcessDriveTrack
        | MovementCrateCallsite::ShipProcessDriveTrack => {
            let ship = probe.callsite == MovementCrateCallsite::ShipProcessDriveTrack;
            if pickup == NativePickupReturn::One && !limbo {
                crate::sim::movement::drive_locomotion::set_entity_current_speed_fraction(
                    entity,
                    probe.saved_current_speed_fraction,
                );
                if let Some(resume) = entity.pending_drive_track_crate_resume.as_mut() {
                    resume.admitted = true;
                }
                if !alive {
                    raw_apply_requested(entity, probe.requested);
                    entity.pending_drive_track_crate_resume = None;
                }
                true
            } else {
                if alive {
                    clear_drive_destination(entity, ship);
                }
                entity.pending_drive_track_crate_resume = None;
                false
            }
        }
        MovementCrateCallsite::DriveProcessMovementFirst
        | MovementCrateCallsite::ShipProcessMovementFirst => {
            // The first/candidate call owns no destination install/clear. Its
            // Zero/unlimbo result is coerced to native CanEnter result seven.
            if alive && (pickup == NativePickupReturn::One || limbo) {
                if let Some(resume) = entity.pending_process_movement_crate_resume.as_mut() {
                    resume.admitted = true;
                }
                true
            } else {
                // Zero/unlimbo is native CanEnter result 7. The rejection owns
                // common finalization: discard the local curve/selector, clear
                // any callback destination on a live collector, and stop.
                if alive {
                    entity.drive_track = None;
                    let ship = probe.callsite
                        == MovementCrateCallsite::ShipProcessMovementFirst;
                    clear_drive_destination(entity, ship);
                    if ship {
                        if let Some(runtime) = entity.ship_locomotion.as_mut() {
                            runtime.track_index = -1;
                        }
                    } else if let Some(runtime) = entity.drive_locomotion.as_mut() {
                        runtime.track_index = -1;
                        runtime.point_index = 0;
                        runtime.track_valid = false;
                    }
                    entity.current_speed_fraction = NativeF64Bits::POSITIVE_ZERO;
                    if let Some(target) = entity.movement_target.as_mut() {
                        target.current_speed = SIM_ZERO;
                    }
                }
                entity.pending_process_movement_crate_resume = None;
                false
            }
        }
        MovementCrateCallsite::DriveProcessMovementFinal
        | MovementCrateCallsite::ShipProcessMovementFinal => {
            let ship = probe.callsite == MovementCrateCallsite::ShipProcessMovementFinal;
            if pickup == NativePickupReturn::One && !limbo {
                // The post-pickup call at Drive 0x004B4705 / Ship 0x006A3D34
                // is Apply_Track_Occupation_Mode, not Object::SetCoords.  Keep
                // the immutable endpoint in the staged resume and let the
                // Simulation owner apply the vehicle occupation mark after
                // this stable-ID re-fetch.  Moving Position here teleports a
                // fresh curve to its head and leaves OccupancyGrid behind.
                if let Some(resume) = entity.pending_process_movement_crate_resume.as_mut() {
                    resume.admitted = true;
                }
            } else {
                if alive {
                    clear_drive_destination(entity, ship);
                    // `MovementTarget::final_goal` is Rust's segmented-path
                    // projection of the live locomotor destination.  Native
                    // clears that destination only while the collector is
                    // alive; leaving this projection installed would let the
                    // next Rust tick synthesize a fresh segment after the
                    // final pickup rejected the curve.
                    if let Some(target) = entity.movement_target.as_mut() {
                        target.final_goal = None;
                    }
                }
                // Drive ProcessMovement @ 0x004B46E6 and Ship
                // ProcessMovement @ 0x006A3D15 both discard the selected
                // curve and Foot path head for Zero/unlimbo or One/limbo,
                // including dead tombstones.  These are separate from the
                // alive-only destination clear above.
                entity.drive_track = None;
                if ship {
                    if let Some(runtime) = entity.ship_locomotion.as_mut() {
                        runtime.track_index = -1;
                    }
                } else if let Some(runtime) = entity.drive_locomotion.as_mut() {
                    runtime.track_index = -1;
                    runtime.track_valid = false;
                }
                entity.current_speed_fraction = NativeF64Bits::POSITIVE_ZERO;
                if let Some(target) = entity.movement_target.as_mut() {
                    target.path.clear();
                    target.path_layers.clear();
                    target.next_index = 0;
                    target.current_speed = SIM_ZERO;
                }
                entity.pending_process_movement_crate_resume = None;
            }
            false
        }
        MovementCrateCallsite::HoverMovement => {
            let may_enter = if pickup == NativePickupReturn::Zero && !limbo {
                if alive && entity.object_is_falling_down == 0 {
                    if let Some(target) = entity.movement_target.as_mut() {
                        // Rust's active path vector is the Foot+0x5E0 owner.
                        // Keep the wrapper alive only so the synchronous
                        // continuation can return through the normal epilogue.
                        target.path.clear();
                        target.path_layers.clear();
                        target.next_index = 0;
                    }
                    if let Some(locomotor) = entity.locomotor.as_mut() {
                        locomotor.hover_destination = None;
                        locomotor.hover_throttle = SIM_ZERO;
                        locomotor.hover_speed_request = SIM_ZERO;
                    }
                    entity.current_speed_fraction = NativeF64Bits::POSITIVE_ZERO;
                }
                false
            } else {
                alive && !limbo && entity.object_is_falling_down == 0
            };
            if let Some(resume) = entity.pending_ground_crossing_crate_resume.as_mut() {
                resume.admitted = may_enter;
            }
            may_enter
        }
        MovementCrateCallsite::JumpjetMovement => {
            if pickup == NativePickupReturn::One && !limbo {
                if let Some(locomotor) = entity.locomotor.as_mut() {
                    locomotor.jumpjet_destination = Some(probe.requested);
                }
                if let Some(resume) = entity.pending_air_crate_resume.as_mut() {
                    resume.admitted = true;
                    resume.install_post_callback_current = false;
                }
            } else {
                if let Some(locomotor) = entity.locomotor.as_mut() {
                    locomotor.jumpjet_destination = None;
                }
                if alive {
                    if let Some(resume) = entity.pending_air_crate_resume.as_mut() {
                        resume.admitted = true;
                        resume.install_post_callback_current = true;
                    }
                } else {
                    entity.pending_air_crate_resume = None;
                }
            }
            alive
        }
        MovementCrateCallsite::JumpjetDescend => {
            // Pickup return, alive, and limbo are ignored by this native caller.
            entity.movement_target = None;
            entity.jumpjet_post_landing_restored = true;
            entity.jumpjet_recovery_landing_armed = false;
            entity.jumpjet_falling_crash_requested = false;
            if let Some(locomotor) = entity.locomotor.as_mut() {
                locomotor.jumpjet_current_speed = SIM_ZERO;
                locomotor.speed_fraction = SIM_ZERO;
            }
            entity.pending_air_crate_resume = None;
            true
        }
        MovementCrateCallsite::WalkFindSubCellDest => {
            let mut native_return = true;
            if pickup == NativePickupReturn::Zero && !limbo {
                if let Some(locomotor) = entity.locomotor.as_mut() {
                    locomotor.subcell_dest = None;
                }
                native_return = false;
                if alive {
                    let current = (entity.position.sub_x, entity.position.sub_y);
                    if let Some(locomotor) = entity.locomotor.as_mut() {
                        locomotor.subcell_dest = Some(current);
                    }
                }
            } else if entity
                .locomotor
                .as_ref()
                .is_none_or(|locomotor| locomotor.subcell_dest.is_none())
            {
                let current = (entity.position.sub_x, entity.position.sub_y);
                if let Some(locomotor) = entity.locomotor.as_mut() {
                    locomotor.subcell_dest = Some(current);
                }
                native_return = false;
            }
            if let Some(resume) = entity.pending_ground_crossing_crate_resume.as_mut() {
                resume.admitted = alive || pickup != NativePickupReturn::Zero || limbo;
            }
            native_return
        }
        MovementCrateCallsite::TeleportArrival => {
            // Pickup result/alive/limbo are ignored; the adapter emits the
            // animation from post-callback XYZ after this raw stop.
            entity.movement_target = None;
            entity.current_speed_fraction = NativeF64Bits::POSITIVE_ZERO;
            if let Some(resume) = entity.pending_teleport_crate_resume.as_mut() {
                resume.admitted = true;
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::components::{DriveLocomotionRuntime, ShipLocomotionRuntime};
    use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};

    fn entity() -> GameEntity {
        let mut entity = GameEntity::test_default(1, "TANK", "House", 1, 1);
        entity.lifecycle.in_limbo = false;
        entity
    }

    fn jumpjet_entity() -> GameEntity {
        let mut entity = entity();
        let mut locomotor = LocomotorState::for_test_kind(LocomotorKind::Jumpjet);
        locomotor.layer = MovementLayer::Air;
        entity.locomotor = Some(locomotor);
        entity.pending_air_crate_resume = Some(AirPickupResume {
            stage: AirPickupStage::JumpjetMovement,
            final_goal: None,
            arrived: false,
            install_post_callback_current: false,
            admitted: false,
        });
        entity
    }

    fn crossing_resume(stage: GroundCrossingPickupStage) -> GroundCrossingPickupResume {
        GroundCrossingPickupResume {
            stage,
            admitted: false,
        }
    }

    #[test]
    fn force_track_one_unlimbo_raw_writes_dead_tombstone_but_preserves_retarget() {
        let mut entity = entity();
        entity.lifecycle.object_alive = false;
        entity.drive_locomotion = Some(DriveLocomotionRuntime::default());
        entity.drive_locomotion.as_mut().unwrap().destination = Some(DriveCoord::cell(9, 9, 0));
        let probe = MovementCrateProbe {
            callsite: MovementCrateCallsite::DriveForceTrack,
            requested: DriveCoord::cell(4, 5, 7),
            saved_current_speed_fraction: NativeF64Bits::from_bits(0x3fd8_0000_0000_0000),
        };
        assert!(continue_after_pickup(&mut entity, probe, NativePickupReturn::One));
        let drive = entity.drive_locomotion.as_ref().unwrap();
        assert_eq!(drive.destination, Some(DriveCoord::cell(9, 9, 0)));
        assert_eq!(drive.head_to, Some(probe.requested));
        assert_eq!(drive.target_speed_fraction, NativeF64Bits::ONE);
        assert_eq!((entity.position.rx, entity.position.ry), (4, 5));
    }

    #[test]
    fn process_track_restores_saved_fraction_but_keeps_speed_crate_operand() {
        let mut entity = entity();
        entity.ship_locomotion = Some(ShipLocomotionRuntime::default());
        entity.speed_crate_multiplier = NativeF64Bits::from_bits(0x3ff3_3333_3333_3333);
        entity.current_speed_fraction = NativeF64Bits::ONE;
        let probe = MovementCrateProbe {
            callsite: MovementCrateCallsite::ShipProcessDriveTrack,
            requested: DriveCoord::cell(2, 3, 0),
            saved_current_speed_fraction: NativeF64Bits::from_bits(0x3fd8_0000_0000_0000),
        };
        assert!(continue_after_pickup(&mut entity, probe, NativePickupReturn::One));
        assert_eq!(entity.current_speed_fraction, probe.saved_current_speed_fraction);
        assert_eq!(
            entity.speed_crate_multiplier,
            NativeF64Bits::from_bits(0x3ff3_3333_3333_3333)
        );
    }

    #[test]
    fn final_rejection_matrix_clears_track_and_path_but_only_live_destination() {
        let callback_destination = DriveCoord::cell(8, 8, 0);
        for (ship, callsite) in [
            (false, MovementCrateCallsite::DriveProcessMovementFinal),
            (true, MovementCrateCallsite::ShipProcessMovementFinal),
        ] {
            for (pickup, limbo) in [
                (NativePickupReturn::Zero, false),
                (NativePickupReturn::One, true),
            ] {
                for alive in [false, true] {
                    let mut entity = entity();
                    entity.lifecycle.object_alive = alive;
                    entity.lifecycle.in_limbo = limbo;
                    entity.current_speed_fraction = NativeF64Bits::ONE;
                    entity.drive_track = Some(
                        crate::sim::movement::drive_track::begin_drive_track(1, 0, 1, 0, 0x40)
                            .expect("fixture curve"),
                    );
                    let mut target = crate::sim::components::MovementTarget::default();
                    target.path = vec![(1, 1), (2, 1), (3, 1)];
                    target.path_layers = vec![MovementLayer::Ground; 3];
                    target.next_index = 2;
                    target.final_goal = Some((9, 9));
                    target.current_speed = SimFixed::from_num(17);
                    entity.movement_target = Some(target);
                    if ship {
                        entity.ship_locomotion = Some(ShipLocomotionRuntime {
                            destination: Some(callback_destination),
                            track_index: 4,
                            ..Default::default()
                        });
                    } else {
                        entity.drive_locomotion = Some(DriveLocomotionRuntime {
                            destination: Some(callback_destination),
                            track_index: 4,
                            track_valid: true,
                            ..Default::default()
                        });
                    }

                    let probe = MovementCrateProbe::at_entity(callsite, &entity);
                    assert!(!continue_after_pickup(&mut entity, probe, pickup));

                    let destination = if ship {
                        let runtime = entity.ship_locomotion.as_ref().unwrap();
                        assert_eq!(runtime.track_index, -1);
                        runtime.destination
                    } else {
                        let runtime = entity.drive_locomotion.as_ref().unwrap();
                        assert_eq!(runtime.track_index, -1);
                        assert!(!runtime.track_valid);
                        runtime.destination
                    };
                    assert_eq!(
                        destination,
                        if alive { None } else { Some(callback_destination) },
                        "only a live {callsite:?} collector clears its destination"
                    );
                    assert!(entity.drive_track.is_none());
                    assert_eq!(
                        entity.current_speed_fraction,
                        NativeF64Bits::POSITIVE_ZERO
                    );
                    let target = entity.movement_target.as_ref().unwrap();
                    assert!(target.path.is_empty());
                    assert!(target.path_layers.is_empty());
                    assert_eq!(target.next_index, 0);
                    assert_eq!(target.current_speed, SIM_ZERO);
                    assert_eq!(
                        target.final_goal,
                        if alive { None } else { Some((9, 9)) },
                        "the segmented destination follows the native alive gate"
                    );
                }
            }
        }
    }

    #[test]
    fn jumpjet_move_dead_one_raw_writes_original_destination_and_resumes_tail() {
        let mut entity = jumpjet_entity();
        entity.lifecycle.object_alive = false;
        entity.position.rx = 20;
        entity.position.ry = 21;
        let probe = MovementCrateProbe {
            callsite: MovementCrateCallsite::JumpjetMovement,
            requested: DriveCoord::cell(4, 5, 7),
            saved_current_speed_fraction: NativeF64Bits::ONE,
        };
        assert!(!continue_after_pickup(
            &mut entity,
            probe,
            NativePickupReturn::One,
        ));
        assert_eq!(
            entity.locomotor.as_ref().unwrap().jumpjet_destination,
            Some(probe.requested),
            "dead/unlimbo still receives the original raw destination write"
        );
        assert!(entity.pending_air_crate_resume.as_ref().unwrap().admitted);

        let mut entities = crate::sim::entity_store::EntityStore::new();
        entities.insert(entity);
        let _ = super::super::air_movement::resume_air_movement_crate_tail(
            &mut entities,
            &[1],
            0,
        );
        let entity = entities.get(1).unwrap();
        assert!(entity.pending_air_crate_resume.is_none());
        assert_eq!(
            entity.locomotor.as_ref().unwrap().jumpjet_destination,
            Some(probe.requested)
        );
    }

    #[test]
    fn jumpjet_move_zero_or_limbo_alive_installs_post_callback_current_xyz() {
        for (pickup, limbo) in [
            (NativePickupReturn::Zero, false),
            (NativePickupReturn::One, true),
        ] {
            let mut entity = jumpjet_entity();
            entity.lifecycle.in_limbo = limbo;
            entity.position.rx = 30;
            entity.position.ry = 31;
            entity.position.sub_x = SimFixed::from_num(17);
            entity.position.sub_y = SimFixed::from_num(29);
            entity.position.exact_z_leptons = Some(333);
            let probe = MovementCrateProbe {
                callsite: MovementCrateCallsite::JumpjetMovement,
                requested: DriveCoord::cell(4, 5, 7),
                saved_current_speed_fraction: NativeF64Bits::ONE,
            };
            assert!(continue_after_pickup(&mut entity, probe, pickup));
            assert!(entity.locomotor.as_ref().unwrap().jumpjet_destination.is_none());

            let mut entities = crate::sim::entity_store::EntityStore::new();
            entities.insert(entity);
            let _ = super::super::air_movement::resume_air_movement_crate_tail(
                &mut entities,
                &[1],
                0,
            );
            let entity = entities.get(1).unwrap();
            assert_eq!(
                entity.locomotor.as_ref().unwrap().jumpjet_destination,
                Some(DriveCoord {
                    x: 30 * 256 + 17,
                    y: 31 * 256 + 29,
                    z: 333,
                })
            );
        }
    }

    #[test]
    fn jumpjet_descend_ignores_return_alive_limbo_and_runs_all_postwrites() {
        for pickup in [NativePickupReturn::Zero, NativePickupReturn::One] {
            for alive in [false, true] {
                for limbo in [false, true] {
                    let mut entity = jumpjet_entity();
                    entity.lifecycle.object_alive = alive;
                    entity.lifecycle.in_limbo = limbo;
                    entity.jumpjet_falling_crash_requested = true;
                    entity.jumpjet_recovery_landing_armed = true;
                    entity.movement_target = Some(Default::default());
                    if let Some(locomotor) = entity.locomotor.as_mut() {
                        locomotor.jumpjet_current_speed = SimFixed::from_num(9);
                        locomotor.speed_fraction = SimFixed::from_num(1);
                    }
                    let probe = MovementCrateProbe {
                        callsite: MovementCrateCallsite::JumpjetDescend,
                        requested: DriveCoord::cell(4, 5, 7),
                        saved_current_speed_fraction: NativeF64Bits::ONE,
                    };
                    assert!(continue_after_pickup(&mut entity, probe, pickup));
                    assert!(entity.movement_target.is_none());
                    assert!(entity.jumpjet_post_landing_restored);
                    assert!(!entity.jumpjet_falling_crash_requested);
                    assert!(!entity.jumpjet_recovery_landing_armed);
                    let locomotor = entity.locomotor.as_ref().unwrap();
                    assert_eq!(locomotor.jumpjet_current_speed, SIM_ZERO);
                    assert_eq!(locomotor.speed_fraction, SIM_ZERO);
                    assert!(entity.pending_air_crate_resume.is_none());
                }
            }
        }
    }

    #[test]
    fn hover_return_alive_limbo_falling_matrix_preserves_or_stops_exact_state() {
        let probe = MovementCrateProbe {
            callsite: MovementCrateCallsite::HoverMovement,
            requested: DriveCoord::cell(2, 1, 0),
            saved_current_speed_fraction: NativeF64Bits::ONE,
        };
        for (pickup, alive, limbo, falling, admitted, clears) in [
            (NativePickupReturn::One, true, false, 0, true, false),
            (NativePickupReturn::One, false, false, 0, false, false),
            (NativePickupReturn::One, true, true, 0, false, false),
            (NativePickupReturn::One, true, false, 1, false, false),
            (NativePickupReturn::Zero, true, false, 0, false, true),
            (NativePickupReturn::Zero, false, false, 0, false, false),
            (NativePickupReturn::Zero, true, true, 0, false, false),
        ] {
            let mut entity = entity();
            entity.lifecycle.object_alive = alive;
            entity.lifecycle.in_limbo = limbo;
            entity.object_is_falling_down = falling;
            let mut locomotor = LocomotorState::for_test_kind(LocomotorKind::Hover);
            let callback_retarget = DriveCoord::cell(9, 9, 7);
            locomotor.hover_destination = Some(callback_retarget);
            locomotor.hover_throttle = SimFixed::from_num(1);
            locomotor.hover_speed_request = SimFixed::from_num(1);
            entity.locomotor = Some(locomotor);
            entity.current_speed_fraction = NativeF64Bits::ONE;
            let mut target = crate::sim::components::MovementTarget::default();
            target.path = vec![(1, 1), (2, 1)];
            target.path_layers = vec![MovementLayer::Ground; 2];
            target.next_index = 1;
            entity.movement_target = Some(target);
            entity.pending_ground_crossing_crate_resume = Some(crossing_resume(
                GroundCrossingPickupStage::Hover {
                    direction: (1, 0),
                    next_layer: MovementLayer::Ground,
                },
            ));

            assert_eq!(continue_after_pickup(&mut entity, probe, pickup), admitted);
            assert_eq!(
                entity
                    .pending_ground_crossing_crate_resume
                    .as_ref()
                    .unwrap()
                    .admitted,
                admitted
            );
            let locomotor = entity.locomotor.as_ref().unwrap();
            assert_eq!(
                locomotor.hover_destination,
                if clears { None } else { Some(callback_retarget) }
            );
            if clears {
                assert!(entity.movement_target.as_ref().unwrap().path.is_empty());
                assert_eq!(locomotor.hover_throttle, SIM_ZERO);
                assert_eq!(locomotor.hover_speed_request, SIM_ZERO);
                assert_eq!(entity.current_speed_fraction, NativeF64Bits::POSITIVE_ZERO);
            }
        }
    }

    #[test]
    fn walk_return_alive_limbo_matrix_runs_native_reservation_branches() {
        let probe = MovementCrateProbe {
            callsite: MovementCrateCallsite::WalkFindSubCellDest,
            requested: DriveCoord::cell(2, 1, 0),
            saved_current_speed_fraction: NativeF64Bits::ONE,
        };
        for (pickup, alive, limbo, initial_dest, expected_return, expected_dest, admitted) in [
            (
                NativePickupReturn::Zero,
                false,
                false,
                Some((SimFixed::from_num(1), SimFixed::from_num(2))),
                false,
                None,
                false,
            ),
            (
                NativePickupReturn::Zero,
                true,
                false,
                Some((SimFixed::from_num(1), SimFixed::from_num(2))),
                false,
                Some((SimFixed::from_num(17), SimFixed::from_num(29))),
                true,
            ),
            (
                NativePickupReturn::Zero,
                false,
                true,
                Some((SimFixed::from_num(1), SimFixed::from_num(2))),
                true,
                Some((SimFixed::from_num(1), SimFixed::from_num(2))),
                true,
            ),
            (
                NativePickupReturn::One,
                false,
                false,
                None,
                false,
                Some((SimFixed::from_num(17), SimFixed::from_num(29))),
                true,
            ),
        ] {
            let mut entity = entity();
            entity.lifecycle.object_alive = alive;
            entity.lifecycle.in_limbo = limbo;
            entity.position.sub_x = SimFixed::from_num(17);
            entity.position.sub_y = SimFixed::from_num(29);
            let mut locomotor = LocomotorState::for_test_kind(LocomotorKind::Walk);
            locomotor.subcell_dest = initial_dest;
            entity.locomotor = Some(locomotor);
            entity.pending_ground_crossing_crate_resume = Some(crossing_resume(
                GroundCrossingPickupStage::Walk {
                    committed_cell: (2, 1),
                    active_layer: MovementLayer::Ground,
                    pending_bridge_update:
                        super::super::movement_bridge::BridgeStateUpdate::Unchanged,
                    projected_on_bridge: false,
                    runtime_bridge_transition: Default::default(),
                },
            ));

            assert_eq!(continue_after_pickup(&mut entity, probe, pickup), expected_return);
            assert_eq!(entity.locomotor.as_ref().unwrap().subcell_dest, expected_dest);
            assert_eq!(
                entity
                    .pending_ground_crossing_crate_resume
                    .as_ref()
                    .unwrap()
                    .admitted,
                admitted
            );
        }
    }
}
