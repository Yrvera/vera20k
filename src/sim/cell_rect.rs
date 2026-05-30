//! Native-shaped CellRect passability and occupancy validators.
//!
//! These are read-only substrate facades over the existing Rust grids. They do
//! not collapse terrain, overlays, reservations, and object-list occupancy into
//! one store; the point is to expose the two distinct gamemd validator surfaces
//! while preserving the current Rust-native ownership split.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::passability;
use crate::sim::pathfinding::zone_map::{ZoneGrid, ZoneId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl CellRect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn single(rx: u16, ry: u16) -> Self {
        Self::new(rx as i32, ry as i32, 1, 1)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CellReservationGrid {
    masks: BTreeMap<(u16, u16), u32>,
}

impl CellReservationGrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reserve(&mut self, rx: u16, ry: u16, reservation_arg: i32) {
        let mask = reservation_mask(reservation_arg);
        if mask != 0 {
            *self.masks.entry((rx, ry)).or_default() |= mask;
        }
    }

    pub fn clear(&mut self, rx: u16, ry: u16, reservation_arg: i32) {
        let mask = reservation_mask(reservation_arg);
        if mask == 0 {
            return;
        }
        if let Some(bits) = self.masks.get_mut(&(rx, ry)) {
            *bits &= !mask;
            if *bits == 0 {
                self.masks.remove(&(rx, ry));
            }
        }
    }

    pub fn has_reservation(&self, rx: u16, ry: u16, reservation_arg: i32) -> bool {
        let mask = reservation_mask(reservation_arg);
        mask != 0
            && self
                .masks
                .get(&(rx, ry))
                .is_some_and(|bits| bits & mask != 0)
    }
}

pub struct CellRectPassabilityContext<'a> {
    pub rect: CellRect,
    pub speed_type: SpeedType,
    pub required_zone_id: Option<ZoneId>,
    pub movement_zone: MovementZone,
    pub required_height_or_level: Option<i16>,
    pub bridge_aware_zone: bool,
    pub reject_any_overlay: bool,
    pub path_grid: Option<&'a PathGrid>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub overlay_grid: Option<&'a OverlayGrid>,
    pub occupancy: Option<&'a OccupancyGrid>,
    pub zone_grid: Option<&'a ZoneGrid>,
}

pub struct CellRectOccupancyContext<'a> {
    pub rect: CellRect,
    pub reservation_arg: i32,
    pub reservations: Option<&'a CellReservationGrid>,
    pub occupancy: Option<&'a OccupancyGrid>,
    pub entities: Option<&'a EntityStore>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub overlay_grid: Option<&'a OverlayGrid>,
    pub map_size: Option<(u16, u16)>,
}

pub fn check_passability_rect(ctx: CellRectPassabilityContext<'_>) -> bool {
    if ctx.rect.width <= 0 || ctx.rect.height <= 0 {
        return true;
    }

    let mut x = 0;
    while x < ctx.rect.width {
        let mut y = 0;
        while y < ctx.rect.height {
            let cx = ctx.rect.x.saturating_add(x);
            let cy = ctx.rect.y.saturating_add(y);
            if !check_cell_passability(&ctx, cx, cy) {
                return false;
            }
            y += 1;
        }
        x += 1;
    }
    true
}

pub fn check_occupancy_rect(ctx: CellRectOccupancyContext<'_>) -> bool {
    let mask = reservation_mask(ctx.reservation_arg);

    if ctx.rect.width > 0 && ctx.rect.height > 0 {
        let mut x = 0;
        while x < ctx.rect.width {
            let mut y = 0;
            while y < ctx.rect.height {
                let cx = ctx.rect.x.saturating_add(x);
                let cy = ctx.rect.y.saturating_add(y);
                let Some((rx, ry)) = to_cell_coord(cx, cy) else {
                    return false;
                };

                if terrain_object_blocks(ctx.resolved_terrain, rx, ry) {
                    return false;
                }
                if mask != 0
                    && ctx.reservations.is_some_and(|reservations| {
                        reservations.has_reservation(rx, ry, ctx.reservation_arg)
                    })
                {
                    return false;
                }
                if overlay_present(ctx.overlay_grid, rx, ry) {
                    return false;
                }
                if ctx
                    .resolved_terrain
                    .and_then(|terrain| terrain.cell(rx, ry))
                    .is_some_and(|cell| {
                        cell.zone_type != zone_class::GROUND || cell.slope_type != 0
                    })
                {
                    return false;
                }
                if ground_building_present(ctx.occupancy, ctx.entities, rx, ry) {
                    return false;
                }

                y += 1;
            }
            x += 1;
        }
    }

    rect_in_playfield(
        ctx.rect,
        ctx.map_size
            .or_else(|| {
                ctx.resolved_terrain
                    .map(|terrain| (terrain.width(), terrain.height()))
            })
            .or_else(|| ctx.overlay_grid.map(|grid| (grid.width(), grid.height()))),
    )
}

fn check_cell_passability(ctx: &CellRectPassabilityContext<'_>, x: i32, y: i32) -> bool {
    let Some((rx, ry)) = to_cell_coord(x, y) else {
        return false;
    };

    if ctx.reject_any_overlay && overlay_present(ctx.overlay_grid, rx, ry) {
        return false;
    }

    if ctx.speed_type == SpeedType::Winged {
        return true;
    }

    let terrain_cell = ctx
        .resolved_terrain
        .and_then(|terrain| terrain.cell(rx, ry));
    let path_cell = ctx.path_grid.and_then(|grid| grid.cell(rx, ry));
    if terrain_cell.is_none() && path_cell.is_none() {
        return false;
    }

    if let Some(required_zone) = ctx.required_zone_id {
        let Some(zone_grid) = ctx.zone_grid else {
            return false;
        };
        let Some(zone_map) = zone_grid.map_for(ctx.movement_zone) else {
            return false;
        };
        let layer = if ctx.bridge_aware_zone {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        if zone_map.zone_at(rx, ry, layer) != required_zone {
            return false;
        }
    }

    let base_level = path_cell
        .map(|cell| cell.signed_level())
        .or_else(|| terrain_cell.map(|cell| cell.level as i8 as i16))
        .unwrap_or(0);
    let structural_bridge = path_cell.is_some_and(|cell| cell.has_structural_bridge())
        || terrain_cell.is_some_and(|cell| cell.bridge_facts.has_structural_bridge());

    let selected_bridge_layer = match ctx.required_height_or_level {
        Some(required) if required == base_level => {
            if structural_bridge && !ctx.bridge_aware_zone {
                return false;
            }
            false
        }
        Some(required) => {
            if !structural_bridge || required != base_level.saturating_add(4) {
                return false;
            }
            true
        }
        None => structural_bridge,
    };

    let occupation_layer = if selected_bridge_layer {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    };
    if ctx
        .occupancy
        .is_some_and(|occupancy| occupancy.count_on_layer(rx, ry, occupation_layer) > 0)
    {
        return false;
    }

    if selected_bridge_layer {
        return true;
    }

    terrain_cell.map_or_else(
        || ctx.path_grid.map_or(true, |grid| grid.is_walkable(rx, ry)),
        |cell| speed_type_allows_cell(cell, ctx.speed_type, ctx.movement_zone),
    )
}

fn speed_type_allows_cell(
    cell: &ResolvedTerrainCell,
    speed_type: SpeedType,
    movement_zone: MovementZone,
) -> bool {
    if cell.zone_type == zone_class::WALL {
        return matches!(
            movement_zone,
            MovementZone::Destroyer
                | MovementZone::AmphibiousDestroyer
                | MovementZone::InfantryDestroyer
                | MovementZone::CrusherAll
        );
    }
    if let Some(cost) = cell.speed_costs.cost_for_speed_type(speed_type) {
        return cost > 0;
    }
    passability::is_passable_for_speed_type(cell.land_type, speed_type)
}

fn terrain_object_blocks(resolved_terrain: Option<&ResolvedTerrainGrid>, rx: u16, ry: u16) -> bool {
    resolved_terrain
        .and_then(|terrain| terrain.cell(rx, ry))
        .is_some_and(|cell| cell.terrain_object_blocks)
}

fn overlay_present(overlay_grid: Option<&OverlayGrid>, rx: u16, ry: u16) -> bool {
    overlay_grid
        .map(|grid| grid.cell(rx, ry).overlay_id.is_some())
        .unwrap_or(false)
}

fn ground_building_present(
    occupancy: Option<&OccupancyGrid>,
    entities: Option<&EntityStore>,
    rx: u16,
    ry: u16,
) -> bool {
    let (Some(occupancy), Some(entities)) = (occupancy, entities) else {
        return false;
    };
    occupancy.get(rx, ry).is_some_and(|cell| {
        cell.iter_layer(MovementLayer::Ground).any(|occupant| {
            entities
                .get(occupant.entity_id)
                .is_some_and(|entity| entity.category == EntityCategory::Structure)
        })
    })
}

fn rect_in_playfield(rect: CellRect, map_size: Option<(u16, u16)>) -> bool {
    let Some((width, height)) = map_size else {
        return true;
    };
    let max_x = rect.x.saturating_add(rect.width).saturating_sub(1);
    let max_y = rect.y.saturating_add(rect.height).saturating_sub(1);
    [
        (rect.x, rect.y),
        (max_x, rect.y),
        (rect.x, max_y),
        (max_x, max_y),
    ]
    .into_iter()
    .all(|(x, y)| x >= 0 && y >= 0 && x < i32::from(width) && y < i32::from(height))
}

fn reservation_mask(reservation_arg: i32) -> u32 {
    if reservation_arg == -1 {
        0
    } else {
        1u32 << ((reservation_arg as u32) & 0x1F)
    }
}

fn to_cell_coord(x: i32, y: i32) -> Option<(u16, u16)> {
    if x < 0 || y < 0 || x > i32::from(u16::MAX) || y > i32::from(u16::MAX) {
        return None;
    }
    Some((x as u16, y as u16))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::map::bridge_facts::{BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::occupancy::CellListInsertion;
    use crate::sim::pathfinding::zone_map::ZoneGrid;

    fn terrain_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
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
            speed_costs: SpeedCostProfile::default(),
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
            zone_type: zone_class::GROUND,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: TerrainClass::Clear,
            base_speed_costs: SpeedCostProfile::default(),
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn flat_terrain(width: u16, height: u16) -> ResolvedTerrainGrid {
        let cells = (0..height)
            .flat_map(|ry| (0..width).map(move |rx| terrain_cell(rx, ry)))
            .collect();
        ResolvedTerrainGrid::from_cells(width, height, cells)
    }

    #[test]
    fn cellrect_occupancy_minus_one_skips_reservation_but_rejects_cell_blockers() {
        let mut terrain = flat_terrain(3, 1);
        terrain.cells[1].slope_type = 2;
        let mut reservations = CellReservationGrid::new();
        reservations.reserve(0, 0, 3);

        let clear_reserved = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: -1,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
        };
        assert!(check_occupancy_rect(clear_reserved));

        let sloped = CellRectOccupancyContext {
            rect: CellRect::single(1, 0),
            reservation_arg: -1,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
        };
        assert!(!check_occupancy_rect(sloped));
    }

    #[test]
    fn cellrect_occupancy_house_reservation_blocks_same_house_only() {
        let terrain = flat_terrain(2, 1);
        let mut reservations = CellReservationGrid::new();
        reservations.reserve(0, 0, 5);

        let same_house = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: 5,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
        };
        assert!(!check_occupancy_rect(same_house));

        let other_house = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: 6,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
        };
        assert!(check_occupancy_rect(other_house));

        let skipped = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: -1,
            reservations: Some(&reservations),
            occupancy: None,
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
        };
        assert!(check_occupancy_rect(skipped));
    }

    #[test]
    fn cellrect_passability_uses_movement_zone_zone_id_and_speed_type_separately() {
        let mut terrain = flat_terrain(2, 1);
        terrain.cells[0].speed_costs = SpeedCostProfile {
            track: Some(100),
            foot: Some(0),
            ..Default::default()
        };
        let path_grid = PathGrid::from_resolved_terrain(&terrain);
        let zone_grid = ZoneGrid::build_with_terrain(
            &path_grid,
            &BTreeMap::new(),
            Some(&terrain),
            &[],
            terrain.width(),
            terrain.height(),
        );
        let zone_id =
            zone_grid
                .map_for(MovementZone::Normal)
                .unwrap()
                .zone_at(0, 0, MovementLayer::Ground);

        let wrong_zone = CellRectPassabilityContext {
            rect: CellRect::single(0, 0),
            speed_type: SpeedType::Track,
            required_zone_id: Some(zone_id.saturating_add(1)),
            movement_zone: MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: false,
            reject_any_overlay: false,
            path_grid: Some(&path_grid),
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            occupancy: None,
            zone_grid: Some(&zone_grid),
        };
        assert!(!check_passability_rect(wrong_zone));

        let foot_speed_blocked = CellRectPassabilityContext {
            rect: CellRect::single(0, 0),
            speed_type: SpeedType::Foot,
            required_zone_id: Some(zone_id),
            movement_zone: MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: false,
            reject_any_overlay: false,
            path_grid: Some(&path_grid),
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            occupancy: None,
            zone_grid: Some(&zone_grid),
        };
        assert!(!check_passability_rect(foot_speed_blocked));

        let track_passes = CellRectPassabilityContext {
            rect: CellRect::single(0, 0),
            speed_type: SpeedType::Track,
            required_zone_id: Some(zone_id),
            movement_zone: MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: false,
            reject_any_overlay: false,
            path_grid: Some(&path_grid),
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            occupancy: None,
            zone_grid: Some(&zone_grid),
        };
        assert!(check_passability_rect(track_passes));
    }

    #[test]
    fn cellrect_passability_bridge_bits_are_not_occupancy_rect_blockers() {
        let mut terrain = flat_terrain(1, 1);
        terrain.cells[0].bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL;
        terrain.cells[0].has_bridge_deck = true;
        terrain.cells[0].bridge_walkable = true;
        terrain.cells[0].bridge_deck_level = 4;
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            0,
            0,
            10,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let passability = CellRectPassabilityContext {
            rect: CellRect::single(0, 0),
            speed_type: SpeedType::Track,
            required_zone_id: None,
            movement_zone: MovementZone::Normal,
            required_height_or_level: None,
            bridge_aware_zone: true,
            reject_any_overlay: false,
            path_grid: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            occupancy: Some(&occupancy),
            zone_grid: None,
        };
        assert!(!check_passability_rect(passability));

        let occupancy_rect = CellRectOccupancyContext {
            rect: CellRect::single(0, 0),
            reservation_arg: -1,
            reservations: None,
            occupancy: Some(&occupancy),
            entities: None,
            resolved_terrain: Some(&terrain),
            overlay_grid: None,
            map_size: None,
        };
        assert!(check_occupancy_rect(occupancy_rect));
    }
}
