//! FootClass-style navigation destination helpers.
//!
//! These helpers model the owner `NavCom` lifecycle separately from
//! `MovementTarget`, which remains the active path execution adapter.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::components::{
    DriveCoord, DriveLocomotionRuntime, NavTargetRef, ShipLocomotionRuntime,
};
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::mission::MissionType;
use crate::util::native_x87::NativeF64Bits;

const SHIP_STOP_TARGET_FRACTION: NativeF64Bits =
    NativeF64Bits::from_bits(0x3fd3_3333_4000_0000);

fn is_drive_locomotor(entity: &GameEntity) -> bool {
    entity
        .locomotor
        .as_ref()
        .is_some_and(|loco| matches!(loco.kind, LocomotorKind::Drive))
}

fn is_ship_locomotor(entity: &GameEntity) -> bool {
    entity
        .locomotor
        .as_ref()
        .is_some_and(|loco| matches!(loco.kind, LocomotorKind::Ship))
}

fn target_cell_coord(
    rx: u16,
    ry: u16,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
) -> DriveCoord {
    let z = resolved_terrain
        .and_then(|terrain| terrain.cell(rx, ry))
        .map(|cell| {
            if cell.has_bridge_deck {
                i32::from(cell.bridge_deck_level)
            } else {
                i32::from(cell.level)
            }
        })
        .unwrap_or(0);
    DriveCoord::cell(rx, ry, z)
}

pub(super) fn resolve_entity_nav_target_drive_coord(
    target: NavTargetRef,
    entities: &EntityStore,
) -> Option<DriveCoord> {
    match target {
        NavTargetRef::Entity { id } => entities.get(id).map(|entity| {
            let pos = &entity.position;
            DriveCoord {
                x: i32::from(pos.rx) * 256 + pos.sub_x.to_num::<i32>(),
                y: i32::from(pos.ry) * 256 + pos.sub_y.to_num::<i32>(),
                z: i32::from(pos.z),
            }
        }),
        NavTargetRef::Cell { .. } | NavTargetRef::Object { .. } | NavTargetRef::Building { .. } => {
            None
        }
    }
}

/// Owner non-null destination path for the Phase 1 normal cell-target slice.
pub(super) fn set_destination_internal_cell(
    entity: &mut GameEntity,
    target: (u16, u16),
    resolved_terrain: Option<&ResolvedTerrainGrid>,
) {
    entity.navigation.nav_com_aux = None;
    entity.navigation.nav_com = Some(NavTargetRef::cell(target.0, target.1));
    entity.navigation.pending_arrival_clear = false;

    if is_drive_locomotor(entity) {
        drive_set_destination(
            entity,
            target_cell_coord(target.0, target.1, resolved_terrain),
        );
    } else if is_ship_locomotor(entity) {
        ship_set_destination(
            entity,
            target_cell_coord(target.0, target.1, resolved_terrain),
        );
    }
}

/// Owner null destination path. Clears the owner and active Drive/Ship destination.
pub(super) fn set_destination_internal_null(entity: &mut GameEntity) {
    entity.navigation.nav_com_aux = None;
    entity.navigation.nav_com = None;
    entity.navigation.pending_arrival_clear = false;

    if is_drive_locomotor(entity) {
        drive_stop_moving(entity);
    } else if is_ship_locomotor(entity) {
        ship_stop_moving(entity);
    }
}

/// FootClass::Stop_Moving-equivalent owner clear: zeroes only the owner
/// destination pair (NavCom and its auxiliary slot), nothing else.
pub(super) fn foot_stop_moving(entity: &mut GameEntity) {
    entity.navigation.nav_com_aux = None;
    entity.navigation.nav_com = None;
}

/// Return the drive-track runtime to rest: no aim point, no active curve.
fn reset_drive_track_runtime(entity: &mut GameEntity) {
    if let Some(drive) = entity.drive_locomotion.as_mut() {
        drive.head_to = None;
        drive.track_valid = false;
        drive.track_index = -1;
        drive.point_index = 0;
    }
}

/// End-of-track owner-navigation resolution for a mover whose path finished
/// this tick. Native contract (drive end-of-track block): when the track ends
/// at the owner destination on a live object, the stop is immediate — the
/// owner destination pair clears the same tick, the path head resets, and,
/// only when the current mission is Move, the arrival advance pops the queued
/// waypoint into a fresh destination. Dying/limbo objects skip the clear
/// entirely (only the ended track's aim point drops). A track that ends away
/// from the owner destination (or a non-cell owner target) keeps the
/// destination; the deferred process-entry pass repaths toward it next tick.
pub(super) fn finish_drive_navigation(
    entity: &mut GameEntity,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
) {
    if is_drive_locomotor(entity) && entity.navigation.nav_com.is_some() {
        if entity.dying {
            // Native liveness gate: no owner clear for a dying object; the
            // ended track still loses its aim point.
            if let Some(drive) = entity.drive_locomotion.as_mut() {
                drive.head_to = None;
            }
            return;
        }
        // Residual: the native arrival match also compares z within twice a
        // global height tolerance (bridge deck vs ground); NavTargetRef::Cell
        // carries no layer, so a same-cell bridge/ground mismatch reads as
        // arrived here.
        let arrived = matches!(
            entity.navigation.nav_com,
            Some(NavTargetRef::Cell { rx, ry })
                if rx == entity.position.rx && ry == entity.position.ry
        );
        if arrived {
            finish_drive_arrival(entity, resolved_terrain);
        } else {
            defer_drive_arrival_clear(entity);
        }
        return;
    }
    if is_ship_locomotor(entity) {
        // Ship's terminal Process_Movement retires the committed +0x3C head
        // before its no-destination/no-path Process tail calls owner
        // SetSpeedFraction(0). Mark the replay queue exhausted first so the
        // ordinary Ship null-destination path observes that same rest state.
        if let Some(ship) = entity.ship_locomotion.as_mut() {
            ship.head_to = None;
            ship.path.cursor = ship.path.directions.len().min(u16::MAX as usize) as u16;
        }
        set_destination_internal_null(entity);
        entity.navigation.nav_queue.clear();
        return;
    }
    // A soft Stop can clear the owner destination while an already-committed
    // Drive curve is still consuming. Its ordinary terminal Enter still clears
    // the head/valid/selector/cursor tuple even though NavCom is already null.
    if is_drive_locomotor(entity) {
        reset_drive_track_runtime(entity);
    }
    // Non-drive movers (and the remaining Drive owner state) keep the
    // pre-existing immediate cleanup.
    set_destination_internal_null(entity);
    entity.navigation.nav_queue.clear();
}

/// Same-tick arrival at the owner destination: clear the owner destination
/// pair immediately, return the drive runtime to rest, and — only under a
/// current Move mission (the native arrival gate) — advance the queued
/// waypoint into a fresh destination. The path toward the fresh destination
/// is built by the deferred process-entry pass at the top of the next
/// movement tick, matching the native next-process track build.
fn finish_drive_arrival(entity: &mut GameEntity, resolved_terrain: Option<&ResolvedTerrainGrid>) {
    foot_stop_moving(entity);
    entity.navigation.pending_arrival_clear = false;
    reset_drive_track_runtime(entity);
    // VERA-internal rest-state cleanup (speed clamp + drive destination
    // drop) — the same rest state the deferred clear used to reach one tick
    // later; the native drive-runtime equivalent is UNCHECKED.
    drive_stop_moving(entity);
    if entity.mission.effective().known() != Some(MissionType::Move) {
        return;
    }
    let Some(NavTargetRef::Cell { rx, ry }) = entity.navigation.nav_queue.first().copied() else {
        return;
    };
    entity.navigation.nav_queue.remove(0);
    set_destination_internal_cell(entity, (rx, ry), resolved_terrain);
    entity.navigation.pending_arrival_clear = true;
}

/// Track/path execution finished away from the owner destination (or the
/// owner target is not a plain cell): the owner keeps its destination, and
/// the deferred pass at the top of the next movement tick rebuilds a path
/// toward it — the drive locomotor's process-entry fallback. Arrivals AT the
/// owner destination never come through here; they clear immediately via
/// [`finish_drive_arrival`].
pub(super) fn defer_drive_arrival_clear(entity: &mut GameEntity) -> bool {
    if !is_drive_locomotor(entity) || entity.navigation.nav_com.is_none() {
        return false;
    }
    entity.navigation.pending_arrival_clear = true;
    reset_drive_track_runtime(entity);
    true
}

pub(super) fn process_pending_empty_drive_arrivals_in_order(
    entities: &mut EntityStore,
    ids: &[u64],
) {
    for &id in ids {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        if !entity.navigation.pending_arrival_clear {
            continue;
        }
        if entity.movement_target.is_some() || entity.drive_track.is_some() {
            continue;
        }
        if entity.navigation.nav_queue.is_empty() {
            set_destination_internal_null(entity);
        }
    }
}

fn drive_set_destination(entity: &mut GameEntity, destination: DriveCoord) {
    let drive = entity
        .drive_locomotion
        .get_or_insert_with(DriveLocomotionRuntime::default);
    drive.destination = Some(destination);
    drive.head_to = Some(destination);
}

fn drive_stop_moving(entity: &mut GameEntity) {
    let drive = entity
        .drive_locomotion
        .get_or_insert_with(DriveLocomotionRuntime::default);
    drive.destination = None;
    // Drive rest state. The gamemd Drive `Process` tail drives the applied speed
    // fraction to exactly 0.0 once the drive destination coord, the head-to
    // coord and the owner path head are all empty and the fraction is still
    // above zero — there is no rest clamp on this path. The 0.3 an earlier
    // revision used here is the destination-brake FLOOR (a different branch),
    // and the 0.2 it was modelled on belongs to the unrelated bump/rock clamp
    // inside `Process_Drive_Track`. Every `Accelerates=true` departure therefore
    // ramps up from zero; in stock YR that set is the Ore Miner and both MCVs,
    // which omit `Accelerates=` and take the constructor default.
    if drive.head_to.is_none() {
        if f64::from_bits(entity.current_speed_fraction.bits()) > 0.0 {
            entity.current_speed_fraction = NativeF64Bits::POSITIVE_ZERO;
        }
        drive.owner_current_speed = 0;
    }
}

fn ship_set_destination(entity: &mut GameEntity, destination: DriveCoord) {
    let ship = entity
        .ship_locomotion
        .get_or_insert_with(ShipLocomotionRuntime::default);
    // Ship's Move_To slot writes only +0x30. The committed +0x3C head is
    // selected later by Process_Movement from the owner's path.
    ship.destination = Some(destination);
}

fn ship_stop_moving(entity: &mut GameEntity) {
    let ship = entity
        .ship_locomotion
        .get_or_insert_with(ShipLocomotionRuntime::default);
    // Ship Stop_Moving clamps the class-owned target fraction, then clears
    // only +0x30. A committed head may continue to its track endpoint.
    if f64::from_bits(ship.target_speed_fraction.bits())
        > f64::from_bits(SHIP_STOP_TARGET_FRACTION.bits())
    {
        ship.target_speed_fraction = SHIP_STOP_TARGET_FRACTION;
    }
    ship.destination = None;

    // FootClass::Stop_Moving clears the owner path sentinel. With no committed
    // Ship head there is therefore no segment left to consume: retire Rust's
    // path-replay adapter and reproduce the Process-tail SetSpeedFraction(0).
    // A non-null head is the sole case that preserves the committed segment.
    if ship.head_to.is_none() {
        ship.path.cursor = ship.path.directions.len().min(u16::MAX as usize) as u16;
        if f64::from_bits(entity.current_speed_fraction.bits()) > 0.0 {
            entity.current_speed_fraction = NativeF64Bits::POSITIVE_ZERO;
        }
        ship.owner_current_speed = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::locomotor::LocomotorState;

    fn native_f64(value: f64) -> NativeF64Bits {
        NativeF64Bits::from_bits(value.to_bits())
    }

    #[test]
    fn gsi_13_06_ship_destination_and_stop_stay_on_locomotor_runtime() {
        let mut entity = GameEntity::test_default(1, "DLPH", "Americans", 3, 3);
        entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Ship));

        set_destination_internal_cell(&mut entity, (4, 3), None);
        entity.current_speed_fraction = native_f64(0.5);
        let ship = entity.ship_locomotion.as_mut().expect("Ship runtime");
        assert_eq!(ship.destination, Some(DriveCoord::cell(4, 3, 0)));
        assert_eq!(
            ship.head_to, None,
            "Move_To does not invent a committed head"
        );
        ship.target_speed_fraction = NativeF64Bits::ONE;
        ship.owner_current_speed = 10;
        ship.path.directions = vec![64, 64];
        ship.path.cursor = 0;

        set_destination_internal_null(&mut entity);
        let ship = entity.ship_locomotion.as_ref().expect("Ship runtime");
        assert_eq!(ship.destination, None);
        assert_eq!(ship.target_speed_fraction, SHIP_STOP_TARGET_FRACTION);
        assert_eq!(ship.path.cursor, 2);
        assert_eq!(entity.current_speed_fraction, NativeF64Bits::POSITIVE_ZERO);
        assert_eq!(ship.owner_current_speed, 0);
    }

    #[test]
    fn gsi_13_06_ship_stop_preserves_committed_head_and_owner_speed() {
        let mut entity = GameEntity::test_default(1, "DLPH", "Americans", 3, 3);
        entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Ship));
        entity.navigation.nav_com = Some(NavTargetRef::cell(5, 3));
        entity.current_speed_fraction = native_f64(0.5);
        entity.ship_locomotion = Some(ShipLocomotionRuntime {
            destination: Some(DriveCoord::cell(5, 3, 0)),
            head_to: Some(DriveCoord::cell(4, 3, 0)),
            path: crate::sim::components::DrivePathQueue {
                directions: vec![64, 64],
                cursor: 1,
                ..Default::default()
            },
            track_facing: 0,
            track_index: -1,
            target_speed_fraction: NativeF64Bits::ONE,
            owner_current_speed: 10,
        });

        set_destination_internal_null(&mut entity);

        let ship = entity.ship_locomotion.as_ref().expect("Ship runtime");
        assert_eq!(ship.destination, None);
        assert_eq!(ship.head_to, Some(DriveCoord::cell(4, 3, 0)));
        assert_eq!(ship.target_speed_fraction, SHIP_STOP_TARGET_FRACTION);
        assert_eq!(entity.current_speed_fraction, native_f64(0.5));
        assert_eq!(ship.owner_current_speed, 10);

        let ship = entity.ship_locomotion.as_mut().expect("Ship runtime");
        ship.destination = Some(DriveCoord::cell(5, 3, 0));
        ship.target_speed_fraction = native_f64(0.2);
        set_destination_internal_null(&mut entity);
        assert_eq!(
            entity
                .ship_locomotion
                .as_ref()
                .expect("Ship runtime")
                .target_speed_fraction,
            native_f64(0.2),
            "Stop stores min(previous target, 0.3)"
        );
    }

    #[test]
    fn gsi_13_06_ship_final_arrival_retires_head_and_owner_speed() {
        let mut entity = GameEntity::test_default(1, "DLPH", "Americans", 4, 3);
        entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Ship));
        entity.navigation.nav_com = Some(NavTargetRef::cell(4, 3));
        entity.current_speed_fraction = native_f64(0.5);
        entity.ship_locomotion = Some(ShipLocomotionRuntime {
            destination: Some(DriveCoord::cell(4, 3, 0)),
            head_to: Some(DriveCoord::cell(4, 3, 0)),
            path: crate::sim::components::DrivePathQueue {
                directions: vec![64],
                cursor: 0,
                ..Default::default()
            },
            track_facing: 0,
            track_index: -1,
            target_speed_fraction: NativeF64Bits::ONE,
            owner_current_speed: 10,
        });

        finish_drive_navigation(&mut entity, None);

        let ship = entity.ship_locomotion.as_ref().expect("Ship runtime");
        assert_eq!(ship.destination, None);
        assert_eq!(ship.head_to, None);
        assert_eq!(ship.path.cursor, 1);
        assert_eq!(entity.current_speed_fraction, NativeF64Bits::POSITIVE_ZERO);
        assert_eq!(ship.owner_current_speed, 0);
    }

    fn resting_drive_miner() -> GameEntity {
        let mut entity = GameEntity::test_default(1, "HARV", "Americans", 3, 3);
        entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
        entity.drive_locomotion = Some(DriveLocomotionRuntime {
            destination: Some(DriveCoord::cell(3, 3, 0)),
            ..Default::default()
        });
        entity.current_speed_fraction = NativeF64Bits::ONE;
        entity
    }

    /// GSI-06.11 G1: the gamemd Drive `Process` tail drives the applied speed
    /// fraction to exactly 0.0 at rest. There is no 0.3 clamp on this path, so
    /// every `Accelerates=true` departure — the Ore Miner and both MCVs in stock
    /// YR — ramps up from zero rather than launching at 30% speed.
    #[test]
    fn gsi_06_11_drive_rest_speed_fraction_returns_to_zero_not_a_stop_clamp() {
        let mut entity = resting_drive_miner();

        set_destination_internal_null(&mut entity);

        let drive = entity.drive_locomotion.as_ref().expect("drive state");
        assert_eq!(entity.current_speed_fraction, NativeF64Bits::POSITIVE_ZERO);
        assert_eq!(drive.destination, None);
    }

    /// The reset is gated, not unconditional: gamemd requires the head-to coord
    /// to be empty too, so a mover still committed to a head keeps its fraction.
    #[test]
    fn gsi_06_11_drive_rest_reset_requires_an_empty_head_to() {
        let mut entity = resting_drive_miner();
        entity
            .drive_locomotion
            .as_mut()
            .expect("drive state")
            .head_to = Some(DriveCoord::cell(4, 3, 0));
        entity.current_speed_fraction = native_f64(0.5);

        set_destination_internal_null(&mut entity);

        assert_eq!(
            entity.current_speed_fraction,
            native_f64(0.5)
        );
    }

    #[test]
    fn resolve_nav_target_drive_coord_tracks_moving_entity() {
        let mut entities = EntityStore::new();
        entities.insert(GameEntity::test_default(2, "MTNK", "Allies", 3, 4));

        let first =
            resolve_entity_nav_target_drive_coord(NavTargetRef::Entity { id: 2 }, &entities)
                .unwrap();
        entities.get_mut(2).unwrap().position.rx += 1;
        let second =
            resolve_entity_nav_target_drive_coord(NavTargetRef::Entity { id: 2 }, &entities)
                .unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn resolve_nav_target_drive_coord_does_not_reaim_cell_targets() {
        let entities = EntityStore::new();

        assert_eq!(
            resolve_entity_nav_target_drive_coord(NavTargetRef::Cell { rx: 12, ry: 34 }, &entities),
            None
        );
    }

    #[test]
    fn resolve_nav_target_drive_coord_does_not_guess_building_anchor() {
        let entities = EntityStore::new();

        assert_eq!(
            resolve_entity_nav_target_drive_coord(NavTargetRef::Building { id: 7 }, &entities),
            None
        );
    }
}
