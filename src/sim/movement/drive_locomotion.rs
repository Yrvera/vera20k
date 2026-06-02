//! DriveLocomotion runtime helpers.
//!
//! This module owns Drive-specific state updates that should not leak into the
//! generic `MovementTarget` path. Detailed DriveTrack consumption remains in
//! `drive_track`; this file handles the Drive-local speed fraction scaffold.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::{LocomotorKind, SpeedType};
use crate::sim::components::{DriveCoord, DriveLocomotionRuntime, NavTargetRef};
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::pathfinding::terrain_speed::{self, TerrainSpeedConfig};
use crate::util::fixed_math::{SIM_ONE, SIM_ZERO, SimFixed};

const DRIVE_DESTINATION_BRAKE_FLOOR: SimFixed = SimFixed::lit("0.3");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DriveProcessOutcome {
    NotDrive,
    Processed,
    Waiting,
    Arrived,
    Blocked,
}

pub(crate) fn process_drive_locomotion_shell(entity: &GameEntity) -> DriveProcessOutcome {
    if entity.drive_locomotion.is_none() {
        return DriveProcessOutcome::NotDrive;
    }
    DriveProcessOutcome::Processed
}

pub(super) fn drive_requires_native_step(drive: &DriveLocomotionRuntime) -> bool {
    drive.active_tube.is_some() || !drive.path.directions.is_empty() || drive.residual_budget != 0
}

pub(super) fn refresh_drive_head_to_from_navcom(
    entity: &mut GameEntity,
    entities: &EntityStore,
) -> bool {
    let Some(target) = entity.navigation.nav_com else {
        return false;
    };
    let Some(coord) = super::navcom::resolve_entity_nav_target_drive_coord(target, entities) else {
        return false;
    };
    refresh_drive_head_to_coord(entity, coord)
}

pub(super) fn refresh_drive_head_to_coord(entity: &mut GameEntity, coord: DriveCoord) -> bool {
    let Some(drive) = entity.drive_locomotion.as_mut() else {
        return false;
    };
    if drive.head_to == Some(coord) {
        return false;
    }
    drive.head_to = Some(coord);
    true
}

pub(super) fn drive_entity_nav_targets(entities: &EntityStore) -> Vec<(u64, NavTargetRef)> {
    entities
        .keys_sorted()
        .into_iter()
        .filter_map(|id| {
            let entity = entities.get(id)?;
            let target = entity.navigation.nav_com?;
            matches!(target, NavTargetRef::Entity { .. }).then_some((id, target))
        })
        .collect()
}

/// Compute the Drive-local target speed fraction from currently modeled runtime
/// modifiers. This is the `DriveLocomotion` owner value; raw `Speed=` remains a
/// separate top-speed input.
pub(super) fn compute_drive_target_speed_fraction(
    speed_type: SpeedType,
    locomotor_kind: LocomotorKind,
    current_cell: (u16, u16),
    next_cell: (u16, u16),
    terrain: &ResolvedTerrainGrid,
    occupancy: &OccupancyGrid,
    config: &TerrainSpeedConfig,
) -> SimFixed {
    terrain_speed::compute_cell_speed_modifier(
        speed_type,
        locomotor_kind,
        current_cell,
        next_cell,
        terrain,
        occupancy,
        config,
    )
}

/// Update Drive target/current speed fractions before budget consumption.
///
/// Gamemd keeps the target fraction on DriveLocomotion and the applied/current
/// fraction on the owner through `SetSpeedFraction`. Rust stores both in the
/// runtime for now, but `current_speed_fraction` is the movement authority.
pub(super) fn update_drive_speed_fraction(
    drive: &mut DriveLocomotionRuntime,
    target_fraction: SimFixed,
    accelerates: bool,
    raw_speed_per_frame: SimFixed,
    accel_factor: SimFixed,
    decel_factor: SimFixed,
    slowdown_distance: SimFixed,
    distance_to_goal: SimFixed,
) {
    drive.target_speed_fraction = target_fraction.clamp(SIM_ZERO, SIM_ONE);
    if !accelerates {
        drive.current_speed_fraction = drive.target_speed_fraction;
        return;
    }

    let target = drive.target_speed_fraction;
    let mut current = drive.current_speed_fraction.clamp(SIM_ZERO, SIM_ONE);
    if slowdown_distance > SIM_ZERO && distance_to_goal < slowdown_distance {
        current -= raw_speed_per_frame * decel_factor;
        if current < DRIVE_DESTINATION_BRAKE_FLOOR {
            current = DRIVE_DESTINATION_BRAKE_FLOOR;
        }
    } else if current < target {
        current += accel_factor;
        if current > target {
            current = target;
        }
    } else if target < current {
        current -= raw_speed_per_frame * decel_factor;
        if current < target {
            current = target;
        }
    }
    drive.current_speed_fraction = current.clamp(SIM_ZERO, SIM_ONE);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::util::fixed_math::{SIM_HALF, SIM_ONE, SIM_ZERO};

    fn terrain_cell(rx: u16, ry: u16, speed_costs: SpeedCostProfile) -> ResolvedTerrainCell {
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
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs,
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
            is_cliff_redraw: false,
            variant: 0,
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

    #[test]
    fn drive_target_speed_fraction_uses_terrain_modifier() {
        let terrain = ResolvedTerrainGrid::from_cells(
            2,
            1,
            vec![
                terrain_cell(0, 0, SpeedCostProfile::default()),
                terrain_cell(
                    1,
                    0,
                    SpeedCostProfile {
                        track: Some(50),
                        ..Default::default()
                    },
                ),
            ],
        );

        let fraction = compute_drive_target_speed_fraction(
            SpeedType::Track,
            LocomotorKind::Drive,
            (0, 0),
            (1, 0),
            &terrain,
            &OccupancyGrid::new(),
            &TerrainSpeedConfig::default(),
        );

        assert_eq!(fraction, SIM_HALF);
    }

    #[test]
    fn accelerates_false_assigns_current_fraction_directly() {
        let mut drive = DriveLocomotionRuntime {
            current_speed_fraction: SIM_ZERO,
            ..Default::default()
        };

        update_drive_speed_fraction(
            &mut drive,
            SIM_HALF,
            false,
            SimFixed::from_num(10),
            SimFixed::lit("0.03"),
            SimFixed::lit("0.002"),
            SimFixed::from_num(500),
            SimFixed::from_num(1000),
        );

        assert_eq!(drive.target_speed_fraction, SIM_HALF);
        assert_eq!(drive.current_speed_fraction, SIM_HALF);
    }

    #[test]
    fn accelerates_true_ramps_current_fraction_upward() {
        let mut drive = DriveLocomotionRuntime {
            current_speed_fraction: SIM_ZERO,
            ..Default::default()
        };

        update_drive_speed_fraction(
            &mut drive,
            SIM_ONE,
            true,
            SimFixed::from_num(10),
            SimFixed::lit("0.03"),
            SimFixed::lit("0.002"),
            SimFixed::from_num(500),
            SimFixed::from_num(1000),
        );

        assert_eq!(drive.target_speed_fraction, SIM_ONE);
        assert_eq!(drive.current_speed_fraction, SimFixed::lit("0.03"));
    }

    #[test]
    fn accelerates_true_brakes_by_raw_speed_scaled_decel_with_floor() {
        let mut drive = DriveLocomotionRuntime {
            current_speed_fraction: SIM_HALF,
            ..Default::default()
        };

        update_drive_speed_fraction(
            &mut drive,
            SIM_ONE,
            true,
            SimFixed::from_num(10),
            SimFixed::lit("0.03"),
            SimFixed::lit("0.002"),
            SimFixed::from_num(500),
            SimFixed::from_num(499),
        );

        assert_eq!(
            drive.current_speed_fraction,
            SIM_HALF - SimFixed::from_num(10) * SimFixed::lit("0.002")
        );
    }

    #[test]
    fn accelerates_true_braking_uses_strict_slowdown_distance() {
        let mut drive = DriveLocomotionRuntime {
            current_speed_fraction: SIM_HALF,
            ..Default::default()
        };

        update_drive_speed_fraction(
            &mut drive,
            SIM_ONE,
            true,
            SimFixed::from_num(10),
            SimFixed::lit("0.03"),
            SimFixed::lit("0.002"),
            SimFixed::from_num(500),
            SimFixed::from_num(500),
        );

        assert_eq!(drive.current_speed_fraction, SimFixed::lit("0.53"));
    }
}
