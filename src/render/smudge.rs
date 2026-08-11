//! Static decal rendering for the SmudgeGrid.
//!
//! Reads the per-cell SmudgeGrid + SmudgeTypeRegistry and produces SpriteInstance
//! buffers for the active smudges. Drawn between the terrain pass and the
//! entity pass so smudges sit on top of the ground but underneath units and
//! buildings.
//!
//! Smudges are static — no animation, no remap, no facing. Multi-cell
//! SmudgeType SHPs have a single composite frame. Native asks every occupied
//! footprint cell to draw frame 0, but each call recenters on the same origin;
//! repeated opaque draws are pixel-idempotent. Render therefore emits one
//! SpriteInstance for the footprint origin and skips non-origin cells.
//!
//! ## Dependency rules
//! - Part of render/ — depends on map/, rules/, sim/.
//! - Reads sim smudge state through immutable references; never mutates sim state.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::terrain::{TILE_HEIGHT, TILE_WIDTH, TilePlacement, iso_to_screen};
use crate::render::batch::SpriteInstance;
use crate::rules::smudge_type::SmudgeTypeRegistry;
use crate::sim::smudge_grid::SmudgeGrid;

/// Type alias for clarity at call sites.
///
/// Smudges share `SpriteInstance` with every other static decal in the engine —
/// they have no per-instance state beyond position, atlas UVs, and depth.
pub type SmudgeInstance = SpriteInstance;

/// Smudges are decals that sit between terrain (depth ≈ 1.0, far) and entities
/// (depth ≈ 0.0, near). Anything in this band draws on top of the ground and
/// underneath any sprite that passes the depth test.
const SMUDGE_DEPTH: f32 = 0.5;

/// Build a `SpriteInstance` buffer for all visible smudges.
///
/// `atlas_lookup` resolves a (smudge_type_id, frame_offset) to a TilePlacement
/// (atlas UVs + pixel size + draw offset). The closure is borrowed because the
/// smudge atlas lives on the renderer side and uses interior types we don't
/// want to expose here.
pub fn build_visible_instances(
    grid: &SmudgeGrid,
    registry: &SmudgeTypeRegistry,
    resolved_terrain: &ResolvedTerrainGrid,
    atlas_lookup: &dyn Fn(u16, u8) -> Option<TilePlacement>,
    camera_x: f32,
    camera_y: f32,
    screen_w: f32,
    screen_h: f32,
) -> Vec<SmudgeInstance> {
    let mut instances: Vec<SmudgeInstance> = Vec::with_capacity(64);
    let view_right: f32 = camera_x + screen_w;
    let view_bottom: f32 = camera_y + screen_h;

    for (rx, ry, cell) in grid.iter_occupied() {
        let Some(type_id) = cell.type_id else {
            continue;
        };
        // Multi-cell smudge footprints are stored as W×H occupied cells, but
        // gamemd draws the SHP once at the footprint origin (per-cell
        // SmudgeTypeClass::Draw_It calls cancel back to the same screen
        // position with frame=0). Skipping non-origin cells produces
        // visually identical pixels and avoids redundant SpriteInstances.
        if cell.frame_offset != 0 {
            continue;
        }
        // Confirm the type still exists in the registry — defensive against
        // map/rules mismatches; an unknown id is silently skipped.
        if registry.get(type_id).is_none() {
            continue;
        }
        let Some(terrain_cell) = resolved_terrain.cell(rx, ry) else {
            continue;
        };
        let (sx, sy): (f32, f32) = iso_to_screen(rx, ry, terrain_cell.level);
        let placement: TilePlacement = match atlas_lookup(type_id, cell.frame_offset) {
            Some(p) => p,
            None => continue,
        };
        // iso_to_screen returns the NW corner of the cell's bounding box;
        // shift by half a tile to land on the cell center, then apply the
        // atlas entry's centered anchor (-pixel_w/2, -pixel_h/2). Mirrors
        // the overlay-render position math.
        let draw_x = sx + TILE_WIDTH / 2.0 + placement.draw_offset[0];
        let draw_y = sy + TILE_HEIGHT / 2.0 + placement.draw_offset[1];
        // Cull against the complete composite SHP bounds. Testing the origin
        // cell's diamond would hide a wide footprint while part of its art is
        // still inside the tactical viewport.
        if draw_x >= view_right || draw_x + placement.pixel_size[0] <= camera_x {
            continue;
        }
        if draw_y >= view_bottom || draw_y + placement.pixel_size[1] <= camera_y {
            continue;
        }
        instances.push(SmudgeInstance {
            position: [draw_x, draw_y],
            size: placement.pixel_size,
            uv_origin: placement.uv_origin,
            uv_size: placement.uv_size,
            depth: SMUDGE_DEPTH,
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
            ..Default::default()
        });
    }
    instances
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::bridge_facts::BridgeCellFacts;
    use crate::map::resolved_terrain::ResolvedTerrainCell;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::smudge_grid::SmudgeCell;
    use std::cell::RefCell;

    /// Closure-style atlas lookup that returns None for every atlas miss.
    fn never_lookup(_type_id: u16, _frame: u8) -> Option<TilePlacement> {
        None
    }

    fn empty_registry() -> SmudgeTypeRegistry {
        SmudgeTypeRegistry::default()
    }

    fn flat_terrain_cell(rx: u16, ry: u16, level: u8) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level,
            filled_clear: false,
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            height_in_pixels: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: true,
            allows_tiberium: false,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            terrain_object_occupation: None,
            overlay_blocks: false,
            overlay_zone_type: None,
            outside_playfield: false,
            zone_type: 0,
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

    fn flat_terrain(level: u8) -> ResolvedTerrainGrid {
        let mut cells = Vec::with_capacity(64);
        for ry in 0..8 {
            for rx in 0..8 {
                cells.push(flat_terrain_cell(rx, ry, level));
            }
        }
        ResolvedTerrainGrid::from_cells(8, 8, cells)
    }

    #[test]
    fn empty_grid_produces_empty_vec() {
        let grid = SmudgeGrid::new(8, 8);
        let registry = empty_registry();
        let terrain = flat_terrain(0);
        let v = build_visible_instances(
            &grid,
            &registry,
            &terrain,
            &never_lookup,
            0.0,
            0.0,
            800.0,
            600.0,
        );
        assert!(v.is_empty());
    }

    #[test]
    fn unknown_type_id_is_skipped() {
        let mut grid = SmudgeGrid::new(8, 8);
        grid.test_force_set(
            4,
            4,
            SmudgeCell {
                type_id: Some(99),
                footprint_origin: Some((4, 4)),
                frame_offset: 0,
            },
        );
        let registry = empty_registry();
        let terrain = flat_terrain(0);
        let v = build_visible_instances(
            &grid,
            &registry,
            &terrain,
            &never_lookup,
            0.0,
            0.0,
            800.0,
            600.0,
        );
        assert!(v.is_empty());
    }

    #[test]
    fn missing_atlas_entry_is_skipped() {
        // Registry has the type, but atlas lookup returns None — same as the
        // pre-atlas-registration state. Should produce no instances.
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR1\n[CR1]\nCrater=yes\nWidth=1\nHeight=1\n",
        )
        .unwrap();
        let registry = SmudgeTypeRegistry::from_rules_ini(&ini);
        let mut grid = SmudgeGrid::new(8, 8);
        let type_id = registry.find_by_name("CR1").unwrap();
        grid.test_force_set(
            4,
            4,
            SmudgeCell {
                type_id: Some(type_id),
                footprint_origin: Some((4, 4)),
                frame_offset: 0,
            },
        );
        let terrain = flat_terrain(0);
        let v = build_visible_instances(
            &grid,
            &registry,
            &terrain,
            &never_lookup,
            0.0,
            0.0,
            800.0,
            600.0,
        );
        assert!(v.is_empty());
    }

    #[test]
    fn visible_smudge_emits_instance_with_lookup() {
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR1\n[CR1]\nCrater=yes\nWidth=1\nHeight=1\n",
        )
        .unwrap();
        let registry = SmudgeTypeRegistry::from_rules_ini(&ini);
        let mut grid = SmudgeGrid::new(8, 8);
        let type_id = registry.find_by_name("CR1").unwrap();
        grid.test_force_set(
            4,
            4,
            SmudgeCell {
                type_id: Some(type_id),
                footprint_origin: Some((4, 4)),
                frame_offset: 0,
            },
        );
        let lookup = |_id: u16, _frame: u8| -> Option<TilePlacement> {
            Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [0.1, 0.1],
                pixel_size: [60.0, 30.0],
                draw_offset: [0.0, 0.0],
            })
        };
        let terrain = flat_terrain(0);
        let v =
            build_visible_instances(&grid, &registry, &terrain, &lookup, 0.0, 0.0, 800.0, 600.0);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].depth, SMUDGE_DEPTH);
    }

    #[test]
    fn gsi_04_11_elevated_multicell_smudge_draws_frame_zero_once_at_cell_level() {
        // 2x2 smudge: 4 cells occupied, frame_offsets 0..3. Only the
        // frame_offset==0 cell (footprint origin) should emit an instance.
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR2\n[CR2]\nCrater=yes\nWidth=2\nHeight=2\n",
        )
        .unwrap();
        let registry = SmudgeTypeRegistry::from_rules_ini(&ini);
        let mut grid = SmudgeGrid::new(8, 8);
        let type_id = registry.find_by_name("CR2").unwrap();
        // Manually seed all 4 cells of a 2x2 footprint at origin (3,3).
        for (dx, dy) in &[(0u16, 0u16), (1, 0), (0, 1), (1, 1)] {
            let frame_offset = (*dx as u8) + (*dy as u8) * 2;
            grid.test_force_set(
                3 + dx,
                3 + dy,
                SmudgeCell {
                    type_id: Some(type_id),
                    footprint_origin: Some((3, 3)),
                    frame_offset,
                },
            );
        }
        let lookup_calls = RefCell::new(Vec::new());
        let lookup = |id: u16, frame: u8| -> Option<TilePlacement> {
            lookup_calls.borrow_mut().push((id, frame));
            Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [0.1, 0.1],
                pixel_size: [120.0, 60.0],
                draw_offset: [0.0, 0.0],
            })
        };
        let terrain = flat_terrain(3);
        // Only the far side of the composite overlaps this narrow viewport;
        // culling the origin cell instead of the SHP bounds would be wrong.
        let v =
            build_visible_instances(&grid, &registry, &terrain, &lookup, 100.0, 80.0, 10.0, 10.0);
        assert_eq!(
            v.len(),
            1,
            "expected 1 SpriteInstance (origin cell only); got {}",
            v.len(),
        );
        assert_eq!(*lookup_calls.borrow(), vec![(type_id, 0)]);

        let (_, flat_y) = iso_to_screen(3, 3, 0);
        let (_, elevated_y) = iso_to_screen(3, 3, 3);
        assert_eq!(flat_y - elevated_y, 45.0, "each level shifts up 15px");
        assert_eq!(v[0].position[1], elevated_y + TILE_HEIGHT / 2.0);
    }
}
