//! FootClass-style navigation destination helpers.
//!
//! These helpers model the owner `NavCom` lifecycle separately from
//! `MovementTarget`, which remains the active path execution adapter.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::components::{DriveCoord, DriveLocomotionRuntime, NavTargetRef};
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::mission::MissionType;
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
    // Non-drive movers (and drive movers with no owner destination) keep the
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::game_entity::GameEntity;

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
