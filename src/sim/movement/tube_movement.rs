//! Low-bridge TubeClass movement state.
//!
//! This is separate from subterranean tunnel locomotion. Low-bridge tubes are
//! map-owned TubeClass facts; units may begin visible tube movement only when
//! the tube has a real path buffer. Automatic low-bridge shell tubes are valid
//! cell facts but have zero steps and are rejected here.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::tube_facts::{TubeFact, TubeId};
use crate::sim::components::Position;
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::facing_from_delta;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::{CellListInsertion, OccupancyGrid};
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
        if tube.path_len() == 0 {
            debug_assert!(
                false,
                "zero-length low-bridge tube movement state should never start"
            );
            entity.low_bridge_tube_state = None;
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

        move_entity_to_cell(entity, occupancy, id, old, next);
        state.cursor = state.cursor.saturating_add(1);

        if state.cursor as usize >= tube.path_len() || next == state.exit {
            let old = (entity.position.rx, entity.position.ry);
            move_entity_to_cell(entity, occupancy, id, old, state.exit);
            entity.low_bridge_tube_state = None;
            if let Some(target) = &mut entity.movement_target {
                if target.next_index < target.path.len()
                    && target.path[target.next_index] == state.exit
                {
                    target.next_index += 1;
                }
            }
        } else {
            entity.low_bridge_tube_state = Some(state);
        }
    }
}

fn move_entity_to_cell(
    entity: &mut crate::sim::game_entity::GameEntity,
    occupancy: &mut OccupancyGrid,
    id: u64,
    old: (u16, u16),
    next: (u16, u16),
) {
    if old == next {
        return;
    }
    entity.facing = facing_from_delta(next.0 as i32 - old.0 as i32, next.1 as i32 - old.1 as i32);
    let layer = entity.movement_layer_or_ground();
    occupancy.move_entity(
        old.0,
        old.1,
        next.0,
        next.1,
        id,
        layer,
        entity.sub_cell,
        CellListInsertion::from_category(entity.category),
    );
    entity.position.rx = next.0;
    entity.position.ry = next.1;
    entity.position.sub_x = CELL_CENTER_LEPTON;
    entity.position.sub_y = CELL_CENTER_LEPTON;
    entity.position.refresh_screen_coords();
    if let Some(loco) = &mut entity.locomotor {
        loco.layer = match layer {
            MovementLayer::Air | MovementLayer::Underground => MovementLayer::Ground,
            other => other,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::{
        ResolvedTerrainCell, ResolvedTerrainGrid, YR_CELL_LAND_TUNNEL,
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

    #[test]
    fn begin_rejects_zero_length_auto_shell() {
        let tube = TubeFact::auto_low_bridge((2, 2), 2);

        assert_eq!(
            begin_low_bridge_tube_movement(TubeId(0), &tube),
            Err(TubeBeginError::ZeroLengthTube)
        );
    }

    #[test]
    fn explicit_tube_tick_advances_and_clears_state() {
        let mut cells = vec![cell(0, 0), cell(1, 0), cell(2, 0)];
        cells[0].yr_cell_land_type = YR_CELL_LAND_TUNNEL;
        cells[0].tube_index = Some(TubeId(0));
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
    }

    #[test]
    fn path_tube_step_rejects_zero_step_shell_without_starting_state() {
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
}
