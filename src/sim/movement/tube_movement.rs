//! Low-bridge TubeClass movement state.
//!
//! This is separate from subterranean tunnel locomotion. Low-bridge tubes are
//! map-owned TubeClass facts. Explicit tubes may have a path buffer, while
//! automatic low-bridge shell tubes have zero steps and immediately finish at
//! their same-cell exit.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::tube_facts::{TubeFact, TubeId};
use crate::sim::components::Position;
use crate::sim::entity_store::EntityStore;
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

pub fn begin_low_bridge_tube_movement(
    tube_id: TubeId,
    tube: &TubeFact,
) -> Result<LowBridgeTubeMovementState, TubeBeginError> {
    Ok(LowBridgeTubeMovementState {
        tube_id,
        cursor: 0,
        entry: tube.entry,
        exit: tube.exit,
        phase: LowBridgeTubePhase::Traversing,
    })
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
        occupancy.move_entity(
            old.0,
            old.1,
            next.0,
            next.1,
            id,
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
    use crate::sim::components::{Health, MovementTarget};
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
    fn begin_accepts_zero_length_auto_shell() {
        let tube = TubeFact::auto_low_bridge((2, 2), 2);

        let state = begin_low_bridge_tube_movement(TubeId(0), &tube).unwrap();
        assert_eq!(state.tube_id, TubeId(0));
        assert_eq!(state.entry, (2, 2));
        assert_eq!(state.exit, (2, 2));
        assert_eq!(state.cursor, 0);
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
    fn zero_step_shell_tick_completes_same_cell_and_projects_low_bridge_state() {
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
        assert_eq!(entity.movement_target.as_ref().unwrap().next_index, 2);
        assert_eq!(
            (entity.position.rx, entity.position.ry, entity.position.z),
            (0, 0, 4)
        );
        assert!(entity.on_bridge);
        assert_eq!(entity.bridge_occupancy.unwrap().deck_level, 4);
        assert_eq!(occupancy.count_on_layer(0, 0, MovementLayer::Bridge), 1);
        assert_eq!(occupancy.count_on_layer(0, 0, MovementLayer::Ground), 0);
    }

    #[test]
    fn path_tube_step_starts_zero_step_shell_state() {
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

        assert_eq!(result, TubePathStepResult::Began);
        assert_eq!(state.unwrap().tube_id, TubeId(0));
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
}
