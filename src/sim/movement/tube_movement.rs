//! Low-bridge TubeClass movement state.
//!
//! This is separate from subterranean tunnel locomotion. Low-bridge tubes are
//! map-owned TubeClass facts. Explicit tubes may have a path buffer, while
//! automatic low-bridge shell tubes have zero steps and immediately finish at
//! their same-cell exit.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::tube_facts::{TubeFact, TubeId};
use crate::sim::components::{DriveCoord, DriveTubePayload, Position};
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::facing_from_delta;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::{CellListInsertion, OccupancyGrid};
use crate::sim::pathfinding::{PathCell, PathGrid};
use crate::util::lepton::CELL_CENTER_LEPTON;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LowBridgeTubePhase {
    Traversing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LowBridgeTubeMovementState {
    pub tube_id: TubeId,
    pub cursor: u8,
    pub entry: (u16, u16),
    pub exit: (u16, u16),
    pub phase: LowBridgeTubePhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TubeBeginError {
    ZeroLengthTube,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TubePathStepResult {
    NotTubeStep,
    Began,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitTubeAdvance {
    Partial,
    AdvancedStep,
    ReachedFinal,
    BlockedFinal,
}

pub fn begin_low_bridge_tube_movement(
    tube_id: TubeId,
    tube: &TubeFact,
) -> Result<LowBridgeTubeMovementState, TubeBeginError> {
    if tube.path_len() == 0 {
        return Err(TubeBeginError::ZeroLengthTube);
    }
    Ok(LowBridgeTubeMovementState {
        tube_id,
        cursor: 0,
        entry: tube.entry,
        exit: tube.exit,
        phase: LowBridgeTubePhase::Traversing,
    })
}

pub fn begin_drive_tube_traversal(
    tube_id: TubeId,
    tube: &TubeFact,
    entry_ground: i32,
    exit_ground: i32,
) -> Result<DriveTubePayload, TubeBeginError> {
    let path_len = tube.path_len();
    if path_len == 0 {
        return Err(TubeBeginError::ZeroLengthTube);
    }
    let z_step = (exit_ground - entry_ground) / path_len as i32;
    let mut cell = tube.entry;
    let mut path_buffer = Vec::with_capacity(path_len);
    for &step in tube.path_steps() {
        if let Some(&(dx, dy)) = crate::util::direction::DIRECTION_DELTAS.get(step as usize) {
            let next_x = (i32::from(cell.0) + dx).clamp(0, i32::from(u16::MAX)) as u16;
            let next_y = (i32::from(cell.1) + dy).clamp(0, i32::from(u16::MAX)) as u16;
            cell = (next_x, next_y);
        }
        path_buffer.push(DriveCoord::cell(cell.0, cell.1, 0));
    }
    Ok(DriveTubePayload {
        tube_index: Some(tube_id.0),
        cursor: 0,
        destination: path_buffer.first().copied(),
        path_buffer,
        z_accumulator: entry_ground + z_step,
        z_step,
    })
}

pub fn tick_unit_tube_payload(
    payload: &mut DriveTubePayload,
    position: &mut Position,
    budget: i32,
    _tube: &TubeFact,
) -> UnitTubeAdvance {
    let Some(destination) = payload.destination else {
        return UnitTubeAdvance::ReachedFinal;
    };
    let budget = budget.max(0);
    let cur_x = i32::from(position.rx) * 256 + position.sub_x.to_num::<i32>();
    let cur_y = i32::from(position.ry) * 256 + position.sub_y.to_num::<i32>();
    let dx = destination.x - cur_x;
    let dy = destination.y - cur_y;
    let dist_sq = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
    let dist = crate::util::fixed_math::isqrt_i64(dist_sq) as i32;
    if dist > budget && dist > 0 {
        let step_x = i64::from(dx) * i64::from(budget) / i64::from(dist);
        let step_y = i64::from(dy) * i64::from(budget) / i64::from(dist);
        set_position_from_drive_coord(position, cur_x + step_x as i32, cur_y + step_y as i32);
        return UnitTubeAdvance::Partial;
    }

    set_position_from_drive_coord(position, destination.x, destination.y);
    position.z = payload.z_accumulator.clamp(0, i32::from(u8::MAX)) as u8;
    payload.cursor = payload.cursor.saturating_add(1);
    payload.z_accumulator = payload.z_accumulator.saturating_add(payload.z_step);
    if usize::from(payload.cursor) >= payload.path_buffer.len() {
        payload.destination = None;
        UnitTubeAdvance::ReachedFinal
    } else {
        payload.destination = payload
            .path_buffer
            .get(usize::from(payload.cursor))
            .copied();
        UnitTubeAdvance::AdvancedStep
    }
}

pub fn finish_unit_tube_movement(
    entity: &mut GameEntity,
    tube: &TubeFact,
    exit_ground_blocked: bool,
) -> UnitTubeAdvance {
    if exit_ground_blocked {
        return UnitTubeAdvance::BlockedFinal;
    }
    let Some(drive) = entity.drive_locomotion.as_mut() else {
        return UnitTubeAdvance::ReachedFinal;
    };
    let z = drive
        .active_tube
        .as_ref()
        .map_or(i32::from(entity.position.z), |payload| {
            payload.z_accumulator
        })
        .clamp(0, i32::from(u8::MAX)) as u8;
    entity.position.rx = tube.exit.0;
    entity.position.ry = tube.exit.1;
    entity.position.sub_x = CELL_CENTER_LEPTON;
    entity.position.sub_y = CELL_CENTER_LEPTON;
    entity.position.z = z;
    entity.position.refresh_screen_coords();
    drive.active_tube = None;
    UnitTubeAdvance::ReachedFinal
}

fn set_position_from_drive_coord(position: &mut Position, x: i32, y: i32) {
    let rx = x.div_euclid(256).clamp(0, i32::from(u16::MAX)) as u16;
    let ry = y.div_euclid(256).clamp(0, i32::from(u16::MAX)) as u16;
    position.rx = rx;
    position.ry = ry;
    position.sub_x = crate::util::fixed_math::SimFixed::from_num(x.rem_euclid(256));
    position.sub_y = crate::util::fixed_math::SimFixed::from_num(y.rem_euclid(256));
    position.refresh_screen_coords();
}

pub fn try_begin_path_tube_step(
    tube_state: &mut Option<LowBridgeTubeMovementState>,
    target: &mut crate::sim::components::MovementTarget,
    position: &Position,
    terrain: Option<&ResolvedTerrainGrid>,
) -> TubePathStepResult {
    if target.next_index >= target.path.len() {
        return TubePathStepResult::NotTubeStep;
    }
    let current = (position.rx, position.ry);
    let next = target.path[target.next_index];
    let dx = next.0 as i32 - current.0 as i32;
    let dy = next.1 as i32 - current.1 as i32;
    if dx.abs() <= 1 && dy.abs() <= 1 && (dx != 0 || dy != 0) {
        return TubePathStepResult::NotTubeStep;
    }

    let Some(terrain) = terrain else {
        return TubePathStepResult::Blocked;
    };
    let Some(tube_id) = terrain
        .cell(current.0, current.1)
        .and_then(|cell| cell.tube_index)
    else {
        return TubePathStepResult::Blocked;
    };
    let Some(tube) = terrain.tube(tube_id) else {
        return TubePathStepResult::Blocked;
    };
    if tube.exit != next {
        return TubePathStepResult::Blocked;
    }

    match begin_low_bridge_tube_movement(tube_id, tube) {
        Ok(state) => {
            *tube_state = Some(state);
            TubePathStepResult::Began
        }
        Err(TubeBeginError::ZeroLengthTube) => TubePathStepResult::Blocked,
    }
}

pub fn tick_low_bridge_tube_movement(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    terrain: &ResolvedTerrainGrid,
) {
    let keys = entities.keys_sorted();
    if !keys.iter().any(|&id| {
        entities
            .get(id)
            .is_some_and(|entity| entity.low_bridge_tube_state.is_some())
    }) {
        return;
    }
    let path_grid = PathGrid::from_resolved_terrain(terrain);
    for &id in &keys {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        let Some(mut state) = entity.low_bridge_tube_state else {
            continue;
        };
        let Some(tube) = terrain.tube(state.tube_id) else {
            entity.low_bridge_tube_state = None;
            continue;
        };
        if state.cursor as usize >= tube.path_len() {
            finish_tube_movement(entity, occupancy, id, state, &path_grid);
            continue;
        }

        let step = tube
            .path_steps
            .get(state.cursor as usize)
            .copied()
            .unwrap_or(8);
        let old = (entity.position.rx, entity.position.ry);
        let Some(next) = terrain.step_coord_by_direction(old, step) else {
            entity.low_bridge_tube_state = None;
            continue;
        };

        move_entity_to_cell(entity, occupancy, id, old, next, &path_grid);
        state.cursor = state.cursor.saturating_add(1);

        if state.cursor as usize >= tube.path_len() || next == state.exit {
            finish_tube_movement(entity, occupancy, id, state, &path_grid);
        } else {
            entity.low_bridge_tube_state = Some(state);
        }
    }
}

fn finish_tube_movement(
    entity: &mut crate::sim::game_entity::GameEntity,
    occupancy: &mut OccupancyGrid,
    id: u64,
    state: LowBridgeTubeMovementState,
    path_grid: &PathGrid,
) {
    let old = (entity.position.rx, entity.position.ry);
    move_entity_to_cell(entity, occupancy, id, old, state.exit, path_grid);
    entity.low_bridge_tube_state = None;
    if let Some(target) = &mut entity.movement_target {
        if target.next_index < target.path.len() && target.path[target.next_index] == state.exit {
            target.next_index += 1;
        }
    }
}

fn move_entity_to_cell(
    entity: &mut crate::sim::game_entity::GameEntity,
    occupancy: &mut OccupancyGrid,
    id: u64,
    old: (u16, u16),
    next: (u16, u16),
    path_grid: &PathGrid,
) {
    if old != next {
        entity.facing =
            facing_from_delta(next.0 as i32 - old.0 as i32, next.1 as i32 - old.1 as i32);
    }

    let old_occupancy_layer = if entity.on_bridge {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    };
    let active_layer = infer_tube_landing_layer(entity.on_bridge, path_grid.cell(next.0, next.1));
    let bridge_update =
        resolve_tube_landing_bridge_state(entity, old, next, active_layer, path_grid);
    let new_on_bridge =
        super::movement_bridge::projected_on_bridge(entity.on_bridge, bridge_update);
    let new_occupancy_layer = if new_on_bridge {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    };

    if old != next || old_occupancy_layer != new_occupancy_layer {
        // GATE A2 verified two-layer order: remove on the OLD object-list layer,
        // insert on the NEW one (they differ when a tube landing flips on_bridge).
        occupancy.move_entity_layered(
            old.0,
            old.1,
            next.0,
            next.1,
            id,
            old_occupancy_layer,
            new_occupancy_layer,
            entity.sub_cell,
            CellListInsertion::from_category(entity.category),
        );
    }
    entity.position.rx = next.0;
    entity.position.ry = next.1;
    entity.position.sub_x = CELL_CENTER_LEPTON;
    entity.position.sub_y = CELL_CENTER_LEPTON;
    super::movement_bridge::apply_pending_bridge_render_state(
        &mut entity.locomotor,
        &mut entity.bridge_occupancy,
        &mut entity.on_bridge,
        active_layer,
        bridge_update,
        id,
    );
    entity.position.refresh_screen_coords();
}

fn infer_tube_landing_layer(on_bridge: bool, dst_cell: Option<&PathCell>) -> MovementLayer {
    let Some(dst_cell) = dst_cell else {
        return MovementLayer::Ground;
    };
    if dst_cell.bridge_walkable
        && (on_bridge || dst_cell.has_structural_bridge() || dst_cell.is_low_bridge_tube_cell())
    {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    }
}

fn resolve_tube_landing_bridge_state(
    entity: &mut crate::sim::game_entity::GameEntity,
    old: (u16, u16),
    next: (u16, u16),
    active_layer: MovementLayer,
    path_grid: &PathGrid,
) -> super::movement_bridge::BridgeStateUpdate {
    let bridge_update = super::movement_bridge::resolve_cell_transition_bridge_state(
        &mut entity.position,
        Some(path_grid),
        old,
        next,
        active_layer,
    );
    if !matches!(
        bridge_update,
        super::movement_bridge::BridgeStateUpdate::Unchanged
    ) {
        return bridge_update;
    }

    let Some(dst_cell) = path_grid.cell(next.0, next.1) else {
        return bridge_update;
    };

    if dst_cell.bridge_walkable
        && (dst_cell.has_structural_bridge() || dst_cell.is_low_bridge_tube_cell())
    {
        let deck_level = dst_cell
            .bridge_deck_level_if_any()
            .unwrap_or(dst_cell.ground_level);
        entity.position.z = deck_level;
        return super::movement_bridge::BridgeStateUpdate::Set(deck_level);
    }
    if !dst_cell.bridge_walkable {
        entity.position.z = dst_cell.ground_level;
        return super::movement_bridge::BridgeStateUpdate::Clear;
    }

    bridge_update
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::{
        BridgeDirection, BridgeLayer, ResolvedTerrainCell, ResolvedTerrainGrid, YR_CELL_LAND_TUNNEL,
    };
    use crate::map::tube_facts::TubeSource;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::components::{DriveTubePayload, Health, MovementTarget};
    use crate::sim::game_entity::GameEntity;
    use crate::sim::intern::test_intern;

    fn cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: None,
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            is_cliff_redraw: false,
            variant: 0,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            overlay_blocks: false,
            zone_type: 0,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: Default::default(),
            base_speed_costs: Default::default(),
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn low_bridge_tube_cell(
        rx: u16,
        ry: u16,
        tube_id: TubeId,
        deck_level: u8,
    ) -> ResolvedTerrainCell {
        let mut c = cell(rx, ry);
        c.level = 0;
        c.yr_cell_land_type = YR_CELL_LAND_TUNNEL;
        c.has_bridge_deck = true;
        c.bridge_walkable = true;
        c.bridge_deck_level = deck_level;
        c.bridge_layer = Some(BridgeLayer {
            overlay_id: 0x4a,
            overlay_name: "LOBRDG01".to_string(),
            deck_level,
            direction: BridgeDirection::Low,
        });
        c.tube_index = Some(tube_id);
        c
    }

    #[test]
    fn direction8_rejects_zero_step_tube_shell() {
        let tube = TubeFact::auto_low_bridge((2, 2), 2);

        assert_eq!(
            begin_low_bridge_tube_movement(TubeId(0), &tube).unwrap_err(),
            TubeBeginError::ZeroLengthTube
        );
    }

    #[test]
    fn explicit_tube_tick_advances_and_clears_state() {
        let mut cells = vec![cell(0, 0), cell(1, 0), cell(2, 0)];
        cells[0].yr_cell_land_type = YR_CELL_LAND_TUNNEL;
        cells[0].tube_index = Some(TubeId(0));
        cells[2].has_bridge_deck = true;
        cells[2].bridge_walkable = true;
        cells[2].bridge_deck_level = 4;
        let tube = TubeFact {
            entry: (0, 0),
            exit: (2, 0),
            direction: 2,
            path_steps: vec![2, 2],
            source: TubeSource::ExplicitMap,
        };
        let terrain = ResolvedTerrainGrid::from_cells_with_tubes(3, 1, cells, vec![tube]);
        let mut entities = EntityStore::new();
        let mut entity = GameEntity::new(
            1,
            0,
            0,
            0,
            0,
            test_intern("Allies"),
            Health {
                current: 100,
                max: 100,
            },
            test_intern("MTNK"),
            crate::map::entities::EntityCategory::Unit,
            0,
            5,
            true,
        );
        entity.low_bridge_tube_state =
            begin_low_bridge_tube_movement(TubeId(0), terrain.tube(TubeId(0)).unwrap()).ok();
        entity.movement_target = Some(MovementTarget {
            path: vec![(0, 0), (2, 0)],
            path_layers: vec![MovementLayer::Ground, MovementLayer::Ground],
            next_index: 1,
            ..MovementTarget::default()
        });
        entities.insert(entity);
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            0,
            0,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        tick_low_bridge_tube_movement(&mut entities, &mut occupancy, &terrain);
        assert_eq!(entities.get(1).unwrap().position.rx, 1);
        assert!(entities.get(1).unwrap().low_bridge_tube_state.is_some());

        tick_low_bridge_tube_movement(&mut entities, &mut occupancy, &terrain);
        let entity = entities.get(1).unwrap();
        assert_eq!((entity.position.rx, entity.position.ry), (2, 0));
        assert!(entity.low_bridge_tube_state.is_none());
        assert_eq!(entity.movement_target.as_ref().unwrap().next_index, 2);
        assert_eq!(entity.position.z, 4);
        assert!(entity.on_bridge);
        assert_eq!(entity.bridge_occupancy.unwrap().deck_level, 4);
        assert_eq!(occupancy.count_on_layer(2, 0, MovementLayer::Bridge), 1);
        assert_eq!(occupancy.count_on_layer(2, 0, MovementLayer::Ground), 0);
    }

    #[test]
    fn zero_step_shell_tick_does_not_start_active_tube_state() {
        let cells = vec![low_bridge_tube_cell(0, 0, TubeId(0), 4)];
        let terrain = ResolvedTerrainGrid::from_cells_with_tubes(
            1,
            1,
            cells,
            vec![TubeFact::auto_low_bridge((0, 0), 2)],
        );
        let mut entities = EntityStore::new();
        let mut entity = GameEntity::new(
            1,
            0,
            0,
            0,
            0,
            test_intern("Allies"),
            Health {
                current: 100,
                max: 100,
            },
            test_intern("MTNK"),
            crate::map::entities::EntityCategory::Unit,
            0,
            5,
            true,
        );
        entity.low_bridge_tube_state =
            begin_low_bridge_tube_movement(TubeId(0), terrain.tube(TubeId(0)).unwrap()).ok();
        entity.movement_target = Some(MovementTarget {
            path: vec![(0, 0), (0, 0)],
            path_layers: vec![MovementLayer::Ground, MovementLayer::Ground],
            next_index: 1,
            ..MovementTarget::default()
        });
        entities.insert(entity);
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            0,
            0,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        tick_low_bridge_tube_movement(&mut entities, &mut occupancy, &terrain);

        let entity = entities.get(1).unwrap();
        assert!(entity.low_bridge_tube_state.is_none());
        assert_eq!(
            (entity.position.rx, entity.position.ry, entity.position.z),
            (0, 0, 0)
        );
        assert!(!entity.on_bridge);
        assert_eq!(occupancy.count_on_layer(0, 0, MovementLayer::Ground), 1);
    }

    #[test]
    fn path_tube_step_blocks_zero_step_shell_state() {
        let mut cells = vec![cell(0, 0)];
        cells[0].yr_cell_land_type = YR_CELL_LAND_TUNNEL;
        cells[0].tube_index = Some(TubeId(0));
        let terrain = ResolvedTerrainGrid::from_cells_with_tubes(
            1,
            1,
            cells,
            vec![TubeFact::auto_low_bridge((0, 0), 2)],
        );
        let position = Position {
            rx: 0,
            ry: 0,
            z: 0,
            sub_x: CELL_CENTER_LEPTON,
            sub_y: CELL_CENTER_LEPTON,
            screen_x: 0.0,
            screen_y: 0.0,
        };
        let mut target = MovementTarget {
            path: vec![(0, 0), (0, 0)],
            path_layers: vec![MovementLayer::Ground, MovementLayer::Ground],
            next_index: 1,
            ..MovementTarget::default()
        };
        let mut state = None;

        let result = try_begin_path_tube_step(&mut state, &mut target, &position, Some(&terrain));

        assert_eq!(result, TubePathStepResult::Blocked);
        assert!(state.is_none());
    }

    #[test]
    fn path_tube_step_starts_explicit_non_adjacent_tube() {
        let mut cells = vec![cell(0, 0), cell(1, 0), cell(2, 0)];
        cells[0].yr_cell_land_type = YR_CELL_LAND_TUNNEL;
        cells[0].tube_index = Some(TubeId(0));
        let terrain = ResolvedTerrainGrid::from_cells_with_tubes(
            3,
            1,
            cells,
            vec![TubeFact::explicit((0, 0), (2, 0), 2, vec![2, 2])],
        );
        let position = Position {
            rx: 0,
            ry: 0,
            z: 0,
            sub_x: CELL_CENTER_LEPTON,
            sub_y: CELL_CENTER_LEPTON,
            screen_x: 0.0,
            screen_y: 0.0,
        };
        let mut target = MovementTarget {
            path: vec![(0, 0), (2, 0)],
            path_layers: vec![MovementLayer::Ground, MovementLayer::Ground],
            next_index: 1,
            ..MovementTarget::default()
        };
        let mut state = None;

        let result = try_begin_path_tube_step(&mut state, &mut target, &position, Some(&terrain));

        assert_eq!(result, TubePathStepResult::Began);
        assert_eq!(state.unwrap().tube_id, TubeId(0));
    }

    #[test]
    fn direction8_seeds_z_step_with_signed_truncation() {
        let tube = TubeFact::explicit((0, 0), (3, 0), 2, vec![2, 2, 2]);

        let payload = begin_drive_tube_traversal(TubeId(0), &tube, 1, 8).unwrap();

        assert_eq!(payload.z_step, 2);
        assert_eq!(payload.z_accumulator, 3);
        assert_eq!(payload.path_buffer.len(), 3);
    }

    #[test]
    fn unit_tube_partial_budget_does_not_increment_cursor() {
        let tube = TubeFact::explicit((0, 0), (1, 0), 2, vec![2]);
        let mut payload = begin_drive_tube_traversal(TubeId(0), &tube, 0, 0).unwrap();
        let mut position = Position {
            rx: 0,
            ry: 0,
            z: 0,
            sub_x: CELL_CENTER_LEPTON,
            sub_y: CELL_CENTER_LEPTON,
            screen_x: 0.0,
            screen_y: 0.0,
        };

        let result = tick_unit_tube_payload(&mut payload, &mut position, 4, &tube);

        assert_eq!(result, UnitTubeAdvance::Partial);
        assert_eq!(payload.cursor, 0);
    }

    #[test]
    fn unit_tube_reaches_one_step_per_tick() {
        let tube = TubeFact::explicit((0, 0), (2, 0), 2, vec![2, 2]);
        let mut payload = begin_drive_tube_traversal(TubeId(0), &tube, 0, 0).unwrap();
        let mut position = Position {
            rx: 0,
            ry: 0,
            z: 0,
            sub_x: CELL_CENTER_LEPTON,
            sub_y: CELL_CENTER_LEPTON,
            screen_x: 0.0,
            screen_y: 0.0,
        };

        let result = tick_unit_tube_payload(&mut payload, &mut position, 512, &tube);

        assert_eq!(result, UnitTubeAdvance::AdvancedStep);
        assert_eq!(payload.cursor, 1);
    }

    #[test]
    fn unit_tube_final_empty_ground_list_keeps_accumulated_z() {
        let tube = TubeFact::explicit((0, 0), (1, 0), 2, vec![2]);
        let mut entity = GameEntity::new(
            1,
            0,
            0,
            0,
            0,
            test_intern("Allies"),
            Health {
                current: 100,
                max: 100,
            },
            test_intern("MTNK"),
            crate::map::entities::EntityCategory::Unit,
            0,
            5,
            true,
        );
        entity.drive_locomotion = Some(crate::sim::components::DriveLocomotionRuntime {
            active_tube: Some(DriveTubePayload {
                z_accumulator: 7,
                ..Default::default()
            }),
            ..Default::default()
        });

        let result = finish_unit_tube_movement(&mut entity, &tube, false);

        assert_eq!(result, UnitTubeAdvance::ReachedFinal);
        assert_eq!(entity.position.z, 7);
        assert!(
            entity
                .drive_locomotion
                .as_ref()
                .unwrap()
                .active_tube
                .is_none()
        );
    }

    #[test]
    fn unit_tube_final_blocked_ground_list_keeps_active_tube() {
        let tube = TubeFact::explicit((0, 0), (1, 0), 2, vec![2]);
        let mut entity = GameEntity::new(
            1,
            0,
            0,
            0,
            0,
            test_intern("Allies"),
            Health {
                current: 100,
                max: 100,
            },
            test_intern("MTNK"),
            crate::map::entities::EntityCategory::Unit,
            0,
            5,
            true,
        );
        entity.drive_locomotion = Some(crate::sim::components::DriveLocomotionRuntime {
            active_tube: Some(DriveTubePayload::default()),
            ..Default::default()
        });

        let result = finish_unit_tube_movement(&mut entity, &tube, true);

        assert_eq!(result, UnitTubeAdvance::BlockedFinal);
        assert!(
            entity
                .drive_locomotion
                .as_ref()
                .unwrap()
                .active_tube
                .is_some()
        );
    }
}
