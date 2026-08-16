//! Visible terrain sprite-instance construction for the base terrain pass.
//!
//! Render-owned (F05): culling, UV lookup, lighting, fog, and bridge-aware
//! instance emission over the immutable `map::terrain` grid. Map keeps
//! parsing and static projection; the app render pass is the only caller.

use crate::map::lighting::CellLightGrid;
use crate::map::terrain::{
    HEIGHT_STEP, TILE_HEIGHT, TILE_WIDTH, TerrainCell, TerrainGrid, TilePlacement, UvLookupFn,
};
use crate::render::batch::SpriteInstance;

/// Cull margin in screen pixels beyond the viewport on every side.
const CULL_MARGIN: f32 = 120.0;


/// Visible ordinary terrain instances drawn in the base terrain pass.
pub struct TerrainInstances {
    /// Normal terrain — drawn in the base terrain pass.
    pub normal: Vec<SpriteInstance>,
}

fn visible_cell_slice(grid: &TerrainGrid, view_top: f32, view_bottom: f32) -> &[TerrainCell] {
    let start = grid
        .cells
        .partition_point(|cell| cell.screen_y + TILE_HEIGHT < view_top);
    let end = grid
        .cells
        .partition_point(|cell| cell.screen_y <= view_bottom);
    &grid.cells[start..end]
}

/// Generate SpriteInstance data for all tiles visible in the current viewport.
///
/// Single-layer rendering: each cell draws exactly one tile. LAT transition
/// tiles are fully opaque inside the diamond shape (confirmed via diagnostics),
/// so no base clear-ground layer is needed. Missing tiles are skipped —
/// the caller's UV lookup should provide fallbacks if desired.
pub fn build_visible_instances(
    grid: &TerrainGrid,
    lighting_grid: Option<&CellLightGrid>,
    camera_x: f32,
    camera_y: f32,
    screen_width: f32,
    screen_height: f32,
    uv_fn: UvLookupFn<'_>,
    fog: Option<(
        crate::sim::intern::InternedId,
        &crate::sim::vision::FogState,
    )>,
    bridge_state: Option<&crate::sim::bridge_state::BridgeRuntimeState>,
) -> TerrainInstances {
    let view_left: f32 = camera_x - CULL_MARGIN;
    let view_right: f32 = camera_x + screen_width + CULL_MARGIN;
    let view_top: f32 = camera_y - CULL_MARGIN;
    let view_bottom: f32 = camera_y + screen_height + CULL_MARGIN;

    let mut instances = TerrainInstances {
        normal: Vec::with_capacity(grid.cells.len() / 2),
    };

    for cell in visible_cell_slice(grid, view_top, view_bottom) {
        // AABB visibility test against viewport.
        let right: f32 = cell.screen_x + TILE_WIDTH;
        let bottom: f32 = cell.screen_y + TILE_HEIGHT;

        if right < view_left || cell.screen_x > view_right {
            continue;
        }
        if bottom < view_top || cell.screen_y > view_bottom {
            continue;
        }

        // Skip fully shrouded cells — matches gamemd which doesn't render terrain
        // for unexplored cells at all (ZBuffer cleared to 0xFFFF prevents drawing).
        if let Some((owner, fog_state)) = fog {
            if !fog_state.is_cell_revealed(owner, cell.rx, cell.ry) {
                continue;
            }
        }

        // Depth: reconstruct elevation-free iso row, then normalize.
        // Lower screen_y → larger depth (drawn behind). Elevation bias ensures
        // elevated tiles draw in front of same-row ground tiles.
        let signed_z = f32::from(cell.z as i8);
        let iso_row: f32 = cell.screen_y + signed_z * HEIGHT_STEP;
        let normalized: f32 = ((iso_row - grid.origin_y) / grid.world_height).clamp(0.0, 1.0);
        let z_bias: f32 = signed_z * 0.0001;
        let depth: f32 = (1.0 - normalized - z_bias).clamp(0.001, 0.999);

        // Bridge cells with baked damaged-variant TMP data ignore the
        // map-load PRNG variant and instead route to the per-frame
        // damaged_variant bool from the sim's BridgeRuntimeState. Variant 1
        // is the damaged baked art; variant 0 is the pristine art.
        //
        // Native ordering (verified from the tile draw entry point): the engine
        // tests the tile's chain length FIRST — `total_file_count < 2` pins the
        // variant to 0 and skips the damaged check entirely — and only then asks
        // whether the sub-tile has damaged data. Every other case agrees with the
        // branch below, so the single divergent input is a tile that advertises
        // damaged data while owning exactly one TMP file: gamemd draws variant 0,
        // VERA asks the atlas for variant 1 and the exact-key lookup drops the
        // cell. Reachability in stock data is UNCHECKED — damaged art ships as a
        // sibling file, so such a tile should not exist. Closing it needs the
        // chain length at draw time, which this struct does not carry.
        let damaged_variant_swap: u8 = if cell.has_damaged_data {
            bridge_state
                .and_then(|bs| bs.cell(cell.rx, cell.ry))
                .map(|bc| bc.damaged_variant as u8)
                .unwrap_or(0)
        } else {
            cell.variant
        };

        // Bridge anchor tile_id override. Fires when sim reports a
        // non-Variant0 bridgehead_anchor_class AND the theater carries
        // the variant table. Swaps the cell's tile_id for the variant's
        // tile_id; sub_tile is preserved (the reference engine only
        // rewrites the tile-class field). When the override fires, the
        // FA2 sibling-TMP slot is reset to 0 — the variant tile_ids ARE
        // the damage progression, no further a/b/c/d swap.
        let anchor_override = grid.anchor_variant_table.and_then(|table| {
            let bc = bridge_state?.cell(cell.rx, cell.ry)?;
            let axis = bc.axis?;
            table.tile_id_for(axis, bc.bridgehead_anchor_class)
        });

        let (effective_tile_id, effective_variant) = match anchor_override {
            Some(tid) => (tid, 0u8),
            None => (cell.tile_id, damaged_variant_swap),
        };

        let placement: Option<TilePlacement> = match &uv_fn {
            Some(f) => f(effective_tile_id, cell.sub_tile, effective_variant),
            None => Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [1.0, 1.0],
                pixel_size: [TILE_WIDTH, TILE_HEIGHT],
                draw_offset: [0.0, 0.0],
            }),
        };

        if let Some(p) = placement {
            let tint = lighting_grid
                .map(|lights| lights.terrain_tile_tint_at((cell.rx, cell.ry)))
                .unwrap_or(cell.tint);
            let inst = SpriteInstance {
                position: [
                    cell.screen_x + p.draw_offset[0],
                    cell.screen_y + p.draw_offset[1],
                ],
                size: p.pixel_size,
                uv_origin: p.uv_origin,
                uv_size: p.uv_size,
                depth,
                tint,
                alpha: 1.0,
                ..Default::default()
            };
            instances.normal.push(inst);
        }
    }

    instances
}
