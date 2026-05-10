//! Static decal rendering for the SmudgeGrid.
//!
//! Reads the per-cell SmudgeGrid + SmudgeTypeRegistry and produces SpriteInstance
//! buffers for the active smudges. Drawn between the terrain pass and the
//! entity pass so smudges sit on top of the ground but underneath units and
//! buildings.
//!
//! Smudges are static — no animation, no remap, no facing. Multi-cell
//! SmudgeType SHPs have a single composite frame; render emits one
//! SpriteInstance per footprint origin cell. The `frame_offset` on each
//! SmudgeCell distinguishes the footprint origin (== 0) from non-origin
//! cells, so the loop here skips cells where `frame_offset != 0`.
//!
//! ## Dependency rules
//! - Part of render/ — depends on map/, rules/, sim/.
//! - Reads sim smudge state through immutable references; never mutates sim state.

use crate::map::terrain::{TILE_HEIGHT, TILE_WIDTH, TilePlacement, iso_to_screen};
use crate::render::batch::SpriteInstance;
use crate::rules::smudge_type::SmudgeTypeRegistry;
use crate::sim::smudge_grid::SmudgeGrid;

/// Type alias for clarity at call sites.
///
/// Smudges share `SpriteInstance` with every other static decal in the engine —
/// they have no per-instance state beyond position, atlas UVs, and depth.
pub type SmudgeInstance = SpriteInstance;

/// Half a tile width in screen pixels. Used to grow the visibility test rect
/// so wide smudge SHPs (W=2 footprint = 60px wide diamonds) don't pop in late.
const VIEW_PAD_X: f32 = 60.0;
/// Half a tile height in screen pixels — same idea on the vertical axis.
const VIEW_PAD_Y: f32 = 30.0;

/// Smudges are decals that sit between terrain (depth ≈ 1.0, far) and entities
/// (depth ≈ 0.0, near). Anything in this band draws on top of the ground and
/// underneath any sprite that passes the depth test.
const SMUDGE_DEPTH: f32 = 0.5;

/// Build a `SpriteInstance` buffer for all visible smudges.
///
/// `atlas_lookup` resolves a (smudge_type_id, frame_offset) to a TilePlacement
/// (atlas UVs + pixel size + draw offset). The closure is borrowed because the
/// smudge atlas (when it exists) lives on the renderer side and uses interior
/// types we don't want to expose here.
///
/// Until the smudge SHP atlas registration lands, callers will pass a closure
/// that always returns `None`. In that case this function returns an empty Vec
/// and no GPU work is dispatched.
pub fn build_visible_instances(
    grid: &SmudgeGrid,
    registry: &SmudgeTypeRegistry,
    atlas_lookup: &dyn Fn(u16, u8) -> Option<TilePlacement>,
    camera_x: f32,
    camera_y: f32,
    screen_w: f32,
    screen_h: f32,
) -> Vec<SmudgeInstance> {
    let mut instances: Vec<SmudgeInstance> = Vec::with_capacity(64);
    let view_left: f32 = camera_x - VIEW_PAD_X;
    let view_right: f32 = camera_x + screen_w + VIEW_PAD_X;
    let view_top: f32 = camera_y - VIEW_PAD_Y;
    let view_bottom: f32 = camera_y + screen_h + VIEW_PAD_Y;

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
        let (sx, sy): (f32, f32) = iso_to_screen(rx, ry, 0);
        // Cheap diamond-bbox cull. The placement may render outside this box
        // by a few pixels (draw_offset), but VIEW_PAD_X/Y already covers it.
        if sx > view_right || sx + VIEW_PAD_X < view_left {
            continue;
        }
        if sy > view_bottom || sy + VIEW_PAD_Y < view_top {
            continue;
        }
        let placement: TilePlacement = match atlas_lookup(type_id, cell.frame_offset) {
            Some(p) => p,
            None => continue,
        };
        // iso_to_screen returns the NW corner of the cell's bounding box;
        // shift by half a tile to land on the cell center, then apply the
        // atlas entry's centered anchor (-pixel_w/2, -pixel_h/2). Mirrors
        // the overlay-render position math.
        instances.push(SmudgeInstance {
            position: [
                sx + TILE_WIDTH / 2.0 + placement.draw_offset[0],
                sy + TILE_HEIGHT / 2.0 + placement.draw_offset[1],
            ],
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
    use crate::sim::smudge_grid::SmudgeCell;

    /// Closure-style atlas lookup that returns None for everything — mirrors
    /// the deferred-atlas state in which Task 14 ships.
    fn never_lookup(_type_id: u16, _frame: u8) -> Option<TilePlacement> {
        None
    }

    fn empty_registry() -> SmudgeTypeRegistry {
        SmudgeTypeRegistry::default()
    }

    #[test]
    fn empty_grid_produces_empty_vec() {
        let grid = SmudgeGrid::new(8, 8);
        let registry = empty_registry();
        let v = build_visible_instances(
            &grid,
            &registry,
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
        let v = build_visible_instances(
            &grid,
            &registry,
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
        let v = build_visible_instances(
            &grid,
            &registry,
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
        let v = build_visible_instances(&grid, &registry, &lookup, 0.0, 0.0, 800.0, 600.0);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].depth, SMUDGE_DEPTH);
    }

    #[test]
    fn skips_non_origin_footprint_cells() {
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
        let lookup = |_id: u16, _frame: u8| -> Option<TilePlacement> {
            Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [0.1, 0.1],
                pixel_size: [120.0, 60.0],
                draw_offset: [-60.0, -30.0],
            })
        };
        let v = build_visible_instances(&grid, &registry, &lookup, 0.0, 0.0, 800.0, 600.0);
        assert_eq!(
            v.len(),
            1,
            "expected 1 SpriteInstance (origin cell only); got {}",
            v.len(),
        );
    }
}
