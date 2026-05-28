//! FootClass-style navigation destination helpers.
//!
//! These helpers model the owner `NavCom` lifecycle separately from
//! `MovementTarget`, which remains the active path execution adapter.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::components::{DriveCoord, DriveLocomotionRuntime, NavTargetRef};
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::util::fixed_math::SimFixed;

const DRIVE_STOP_SPEED_CLAMP: SimFixed = SimFixed::lit("0.3");

fn is_drive_locomotor(entity: &GameEntity) -> bool {
    entity
        .locomotor
        .as_ref()
        .is_some_and(|loco| matches!(loco.kind, LocomotorKind::Drive))
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
    }
}

/// Owner null destination path. Clears owner destination before Drive clear-navigation.
pub(super) fn set_destination_internal_null(entity: &mut GameEntity) {
    entity.navigation.nav_com_aux = None;
    entity.navigation.nav_com = None;
    entity.navigation.pending_arrival_clear = false;

    if is_drive_locomotor(entity) {
        drive_stop_moving(entity);
    }
}

/// FootClass::Stop_Moving-equivalent owner clear used by queued arrival.
pub(super) fn foot_stop_moving(entity: &mut GameEntity) {
    entity.navigation.nav_com_aux = None;
    entity.navigation.nav_com = None;
}

/// Track/path execution finished at the owner destination, but gamemd clears
/// owner NavCom on the next no-active-track Drive process pass.
pub(super) fn defer_drive_arrival_clear(entity: &mut GameEntity) -> bool {
    if !is_drive_locomotor(entity) || entity.navigation.nav_com.is_none() {
        return false;
    }
    entity.navigation.pending_arrival_clear = true;
    if let Some(drive) = entity.drive_locomotion.as_mut() {
        drive.head_to = None;
        drive.track_valid = false;
        drive.track_index = -1;
        drive.point_index = 0;
    }
    true
}

pub(super) fn process_pending_empty_drive_arrivals(entities: &mut EntityStore) {
    let ids = entities.keys_sorted();
    for &id in &ids {
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
    if drive.current_speed_fraction > DRIVE_STOP_SPEED_CLAMP {
        drive.current_speed_fraction = DRIVE_STOP_SPEED_CLAMP;
    }
    drive.destination = None;
}
