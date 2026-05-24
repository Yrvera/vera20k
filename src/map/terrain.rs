//! Isometric terrain grid: coordinate math, viewport culling, and instance generation.
//!
//! Converts map cells (isometric rx/ry coordinates) to screen-space pixel positions
//! for rendering. Provides viewport culling to only draw visible tiles.
//!
//! ## Coordinate system
//! RA2 uses isometric coordinates where each cell is a diamond shape (60x30 pixels).
//! Screen position: `sx = (rx - ry) * 30`, `sy = (rx + ry) * 15 - z * 15`.
//!
//! ## Dependency rules
//! - Part of map/ — depends on map/map_file for MapFile/MapCell.

use std::collections::BTreeMap;

use crate::map::lighting::CellLightGrid;
use crate::map::map_file::{MapFile, MapHeader};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::render::batch::SpriteInstance;

/// Isometric tile diamond width in pixels (RA2 standard).
pub const TILE_WIDTH: f32 = 60.0;

/// Isometric tile diamond height in pixels (RA2 standard).
pub const TILE_HEIGHT: f32 = 30.0;

/// Pixels per elevation level. Each z-step raises the tile by this many pixels.
/// RA2: CellHeight = CellSizeY / 2 = 30 / 2 = 15. Confirmed by ra2_yr_map_terrain.md §1.6:
/// "Each height level shifts the cell up by 15 pixels on screen (RA2)."
/// (Note: Tiberian Sun uses 24/2 = 12 — do NOT use TS values here.)
pub const HEIGHT_STEP: f32 = 15.0;

/// Tactical screen-to-cell inverse scan cap from gamemd.exe `0x006D6590`.
pub const TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS: usize = 180;

/// Tactical bridge open-edge threshold. The binary uses strict `> 15`.
pub const TACTICAL_BRIDGE_EDGE_THRESHOLD_PX: f32 = 15.0;

/// Extra high-bridge height adjustment: four height levels at 15 px each.
pub const TACTICAL_BRIDGE_EXTRA_HEIGHT_PX: f32 = 60.0;

const DIR_NORTH: u8 = 0;
const DIR_EAST: u8 = 2;
const DIR_SOUTH: u8 = 4;
const DIR_WEST: u8 = 6;

/// Margin around viewport for culling (pixels). Tiles just outside the visible
/// area are still drawn to avoid pop-in during scrolling.
const CULL_MARGIN: f32 = 120.0;

/// Per-cell high-bridge metadata needed by the tactical inverse.
///
/// This deliberately carries structural/orientation facts instead of only deck
/// height because gamemd's cursor inverse branches on `CellClass+0x140` flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalBridgeCell {
    pub deck_z: u8,
    pub structural: bool,
    pub direction_zero: bool,
}

/// Inputs for the YR-shaped tactical screen-to-cell inverse.
#[derive(Debug, Clone, Copy)]
pub struct TacticalInverseContext<'a> {
    pub height_map: &'a BTreeMap<(u16, u16), u8>,
    pub bridge_cells: Option<&'a BTreeMap<(u16, u16), TacticalBridgeCell>>,
    pub viewport_offset_x: f32,
    pub viewport_offset_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TacticalInverseResult {
    Cell { rx: f32, ry: f32 },
    Fallback { rx: f32, ry: f32 },
}

/// Playable area bounds from map `[Map] LocalSize`, in our screen pixel space.
///
/// In RA2, cells outside LocalSize are hidden by permanent shroud/fog of war.
/// We use this to clip terrain rendering so out-of-bounds cells (which are often
/// tile_index=0 green grass filler) are not drawn.
///
/// LocalSize is in "cell unit" coordinates using the TS-scale pixel grid
/// (CellSizeX=48, CellSizeY=24). We convert those pixel coords to our engine's
/// coordinate system (CellSizeX=60, CellSizeY=30) which has the same isometric
/// axes but different scale and offset.
///
/// Conversion from TS-scale pixel space to our screen space:
///   our_x = ts_x * (60/48) - (Size.X - 1) * 30
///   our_y = ts_y * (30/24) + (Size.X + 1) * 15
///   (both scale factors are 1.25)
#[derive(Debug, Clone, Copy)]
pub struct LocalBounds {
    pub pixel_x: f32,
    pub pixel_y: f32,
    pub pixel_w: f32,
    pub pixel_h: f32,
}

/// InitialHeight constant — Y padding at top for elevation headroom.
const TS_INITIAL_HEIGHT: f32 = 3.0;

/// HeightAddition constant — extra rows below for tall terrain.
const TS_HEIGHT_ADDITION: f32 = 5.0;

/// TS-scale cell pixel dimensions (used for LocalSize conversion).
const TS_CELL_SIZE_X: f32 = 48.0;
const TS_CELL_SIZE_Y: f32 = 24.0;

/// Scale factor from TS-scale pixels to our pixels (60/48 = 30/24 = 1.25).
const TS_SCALE: f32 = TILE_WIDTH / TS_CELL_SIZE_X;

impl LocalBounds {
    /// Build from MapHeader LocalSize values.
    ///
    /// Converts the TS-scale LocalSize pixel rectangle to our engine's screen coordinates.
    /// Formula: x = local_left * 48, y = (local_top - 3) * 24
    /// Our offset: x -= (Size.X - 1) * 30, y += (Size.X + 1) * 15
    pub fn from_header(header: &MapHeader) -> Self {
        let size_x: f32 = header.width as f32;

        // TS-scale pixel coordinates (no baseline — we handle offset ourselves).
        let ts_x: f32 = header.local_left as f32 * TS_CELL_SIZE_X;
        let ts_y: f32 = (header.local_top as f32 - TS_INITIAL_HEIGHT) * TS_CELL_SIZE_Y;
        let ts_w: f32 = header.local_width as f32 * TS_CELL_SIZE_X;
        let ts_h: f32 = (header.local_height as f32 + TS_HEIGHT_ADDITION) * TS_CELL_SIZE_Y;

        // Convert to our coordinate system.
        let our_x: f32 = ts_x * TS_SCALE - (size_x - 1.0) * (TILE_WIDTH / 2.0);
        let our_y: f32 = ts_y * TS_SCALE + (size_x + 1.0) * (TILE_HEIGHT / 2.0);

        LocalBounds {
            pixel_x: our_x,
            pixel_y: our_y,
            pixel_w: ts_w * TS_SCALE,
            pixel_h: ts_h * TS_SCALE,
        }
    }

    /// Check if a screen position is within the playable area.
    pub fn contains(&self, screen_x: f32, screen_y: f32) -> bool {
        screen_x >= self.pixel_x
            && screen_x < self.pixel_x + self.pixel_w
            && screen_y >= self.pixel_y
            && screen_y < self.pixel_y + self.pixel_h
    }
}

/// Per-tile rendering placement data returned by the UV lookup function.
/// Carries atlas UV coordinates, actual pixel size, and draw offset.
#[derive(Debug, Clone, Copy)]
pub struct TilePlacement {
    /// UV origin in the atlas texture (0.0..1.0).
    pub uv_origin: [f32; 2],
    /// UV extent in the atlas texture (0.0..1.0).
    pub uv_size: [f32; 2],
    /// Actual pixel dimensions of this tile (may differ from 60×30 for cliff/shore tiles).
    pub pixel_size: [f32; 2],
    /// Draw offset from the standard diamond origin (pixels).
    /// Negative values shift the sprite left/up to accommodate extra data regions.
    pub draw_offset: [f32; 2],
}

/// UV lookup function type: maps (tile_id, sub_tile) → optional placement data.
/// Returns None for tiles not in the atlas (empty template cells), which are skipped.
/// UV lookup: (tile_id, sub_tile, variant) → placement data.
pub type UvLookupFn<'a> = Option<&'a dyn Fn(u16, u8, u8) -> Option<TilePlacement>>;

/// A single terrain cell with pre-computed screen position.
#[derive(Debug, Clone)]
pub struct TerrainCell {
    /// Screen X position (top-left of diamond bounding box).
    pub screen_x: f32,
    /// Screen Y position (top-left of diamond bounding box).
    pub screen_y: f32,
    /// Tile index for atlas lookup (truncated from i32 to u16; -1 filtered out).
    pub tile_id: u16,
    /// Sub-tile index within the template.
    pub sub_tile: u8,
    /// Elevation level (0 = ground). Used for depth buffer computation.
    pub z: u8,
    /// Isometric cell X coordinate (preserved for lighting lookups).
    pub rx: u16,
    /// Isometric cell Y coordinate (preserved for lighting lookups).
    pub ry: u16,
    /// True when the resolved terrain classifies the cell as water.
    pub is_water: bool,
    /// FinalAlert2 cliff redraw flag — this tile is drawn a second time after
    /// entities so cliff face pixels occlude units behind them.
    pub is_cliff_redraw: bool,
    /// Tile visual variant index (FA2 bRNDImage): 0 = main tile, 1-4 = replacement a-d.
    pub variant: u8,
    /// RGB color tint from map lighting. [1,1,1] = full brightness (default).
    pub tint: [f32; 3],
    /// Per-tile radar minimap color (left half of isometric diamond), from TMP header.
    pub radar_left: [u8; 3],
    /// Per-tile radar minimap color (right half of isometric diamond), from TMP header.
    pub radar_right: [u8; 3],
    /// Mirrors `ResolvedTerrainCell.has_damaged_data` — true when this cell's
    /// TMP sub-tile carries a baked damaged-variant pixel set. Cached at
    /// `TerrainGrid` construction; treat as map-load-immutable. Drives the
    /// per-frame variant-override decision in `build_visible_instances`.
    pub has_damaged_data: bool,
}

/// Pre-computed terrain grid ready for rendering.
///
/// Cells are sorted by screen_y for correct back-to-front draw order.
/// World bounds are computed from all cell positions.
#[derive(Debug)]
pub struct TerrainGrid {
    /// All terrain cells, sorted by screen_y (draw order).
    pub cells: Vec<TerrainCell>,
    /// Total world width in pixels.
    pub world_width: f32,
    /// Total world height in pixels.
    pub world_height: f32,
    /// Minimum screen_x across all cells (world origin x).
    pub origin_x: f32,
    /// Minimum screen_y across all cells (world origin y).
    pub origin_y: f32,
    /// Playable area bounds (from LocalSize). Used to clip overlays/entities too.
    pub local_bounds: Option<LocalBounds>,
    /// Theater-derived bridge anchor variant tile_ids, threaded from
    /// TheaterData at map-load. None when theater lacks BridgeMiddle1/2
    /// keys — renderer override is then bypassed.
    pub anchor_variant_table: Option<crate::map::theater::BridgeAnchorVariantTable>,
}

/// Convert isometric cell coordinates to screen-space pixel position.
///
/// Returns the top-left corner of the tile's diamond bounding box.
/// The original engine passes cell CENTER coords to its coordinate transform
/// for tile positioning, placing the tile NW corner at the diamond center's
/// screen Y:
///   X = 30*(rx-ry) - 30
///   Y = 15*(rx+ry) + 15 - z*15
pub fn iso_to_screen(rx: u16, ry: u16, z: u8) -> (f32, f32) {
    let sx: f32 = (rx as f32 - ry as f32) * TILE_WIDTH / 2.0 - TILE_WIDTH / 2.0;
    let sy: f32 =
        (rx as f32 + ry as f32) * TILE_HEIGHT / 2.0 + TILE_HEIGHT / 2.0 - z as f32 * HEIGHT_STEP;
    (sx, sy)
}

/// Convert lepton-world coords to screen pixels with sub-cell precision.
///
/// 256 leptons = 1 cell. Returns the cell-center screen position so callers
/// can apply per-sprite anchor offsets without re-doing iso math.
///
///   X = (cell_x - cell_y) * TILE_WIDTH/2 + sub_offset_x
///   Y = (cell_x + cell_y) * TILE_HEIGHT/2 + TILE_HEIGHT/2 + sub_offset_y - z_lift
///
/// Negative coords use `div_euclid` / `rem_euclid` so a particle drifting
/// just outside the map's NW corner stays on the correct cell.
pub fn lepton_to_screen(coords: glam::IVec3) -> (f32, f32) {
    const LEPTONS_PER_CELL: i32 = 256;
    let cell_x = coords.x.div_euclid(LEPTONS_PER_CELL);
    let cell_y = coords.y.div_euclid(LEPTONS_PER_CELL);
    let sub_x = coords.x.rem_euclid(LEPTONS_PER_CELL) as f32;
    let sub_y = coords.y.rem_euclid(LEPTONS_PER_CELL) as f32;

    let cell_sx = (cell_x as f32 - cell_y as f32) * TILE_WIDTH / 2.0;
    let cell_sy = (cell_x as f32 + cell_y as f32) * TILE_HEIGHT / 2.0 + TILE_HEIGHT / 2.0;

    let sub_sx = (sub_x - sub_y) * (TILE_WIDTH / 2.0) / LEPTONS_PER_CELL as f32;
    let sub_sy = (sub_x + sub_y) * (TILE_HEIGHT / 2.0) / LEPTONS_PER_CELL as f32;

    let z_lift = coords.z as f32 / LEPTONS_PER_CELL as f32 * HEIGHT_STEP;

    (cell_sx + sub_sx, cell_sy + sub_sy - z_lift)
}

/// Convert screen-space pixel position back to isometric cell coordinates.
///
/// Inverse of `iso_to_screen`. Assumes z=0 (ground level). Returns floating-point
/// coordinates; caller should round to get the nearest cell.
///
/// `iso_to_screen` maps (rx,ry) to `((rx-ry)*30 - 30, (rx+ry)*15 + 15)`.
/// The tile center is at NW + (30, 15). To map tile centers back to integer
/// cell coords, shift by half tile before dividing.
///
/// Derivation (clicking at tile center):
///   tile_center_X = (rx-ry)*30,  tile_center_Y = (rx+ry)*15 + 30
///   col = center_X / 30 = screen_x / 30  (after shifting click left by 30)
///   row = (center_Y - 30) / 15 = (screen_y - 30) / 15
pub fn screen_to_iso(screen_x: f32, screen_y: f32) -> (f32, f32) {
    let half_w: f32 = TILE_WIDTH / 2.0;
    let col: f32 = screen_x / half_w;
    let row: f32 = (screen_y - TILE_HEIGHT) / (TILE_HEIGHT / 2.0);
    let rx: f32 = (col + row) / 2.0;
    let ry: f32 = (row - col) / 2.0;
    (rx, ry)
}

/// Convert tactical/client pixels to isometric cell coordinates using the
/// vertical scan shape verified from gamemd.exe `0x006D6590`.
pub fn screen_to_cell_tactical_inverse(
    screen_x: f32,
    screen_y: f32,
    context: TacticalInverseContext<'_>,
) -> TacticalInverseResult {
    let input_x = screen_x - context.viewport_offset_x;
    let input_y = screen_y - context.viewport_offset_y;
    let (fallback_rx, fallback_ry) = screen_to_iso(input_x, input_y);
    let mut scan_y = input_y + TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS as f32;

    for _ in 0..TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS {
        let (candidate_rx, candidate_ry) = screen_to_iso(input_x, scan_y);
        let Some((cell_rx, cell_ry)) = rounded_lookup_cell(candidate_rx, candidate_ry) else {
            scan_y -= 1.0;
            continue;
        };
        let terrain_z = context
            .height_map
            .get(&(cell_rx, cell_ry))
            .copied()
            .unwrap_or(0);
        let mut adjusted_scan_y = scan_y - terrain_z as f32 * HEIGHT_STEP;

        if let Some(bridge_result) = apply_tactical_bridge_inverse(
            input_x,
            input_y,
            scan_y,
            cell_rx,
            cell_ry,
            terrain_z,
            context,
            &mut adjusted_scan_y,
        ) {
            return bridge_result;
        }

        if adjusted_scan_y <= input_y {
            return TacticalInverseResult::Cell {
                rx: candidate_rx,
                ry: candidate_ry,
            };
        }
        scan_y -= 1.0;
    }

    TacticalInverseResult::Fallback {
        rx: fallback_rx,
        ry: fallback_ry,
    }
}

fn rounded_lookup_cell(rx: f32, ry: f32) -> Option<(u16, u16)> {
    if !rx.is_finite() || !ry.is_finite() || rx < 0.0 || ry < 0.0 {
        return None;
    }
    Some((rx.round() as u16, ry.round() as u16))
}

fn apply_tactical_bridge_inverse(
    input_x: f32,
    input_y: f32,
    scan_y: f32,
    cell_rx: u16,
    cell_ry: u16,
    terrain_z: u8,
    context: TacticalInverseContext<'_>,
    adjusted_scan_y: &mut f32,
) -> Option<TacticalInverseResult> {
    let bridge_cells = context.bridge_cells?;
    let bridge = bridge_cells.get(&(cell_rx, cell_ry)).copied()?;
    if !bridge.structural {
        return None;
    }

    let dir2 = tactical_bridge_neighbor(bridge_cells, cell_rx, cell_ry, DIR_EAST);
    let dir4 = tactical_bridge_neighbor(bridge_cells, cell_rx, cell_ry, DIR_SOUTH);
    let dir0 = bridge
        .direction_zero
        .then(|| tactical_bridge_neighbor(bridge_cells, cell_rx, cell_ry, DIR_NORTH))
        .flatten();
    let dir6 = (!bridge.direction_zero)
        .then(|| tactical_bridge_neighbor(bridge_cells, cell_rx, cell_ry, DIR_WEST))
        .flatten();

    let dir2_is_bridge = dir2.is_some_and(|b| b.structural);
    let dir4_is_bridge = dir4.is_some_and(|b| b.structural);
    let dir0_open_edge = bridge.direction_zero && !dir0.is_some_and(|b| b.structural);
    let dir6_open_edge = !bridge.direction_zero && !dir6.is_some_and(|b| b.structural);

    let dir2_height = tactical_neighbor_height(context.height_map, cell_rx, cell_ry, DIR_EAST);
    let dir4_height = tactical_neighbor_height(context.height_map, cell_rx, cell_ry, DIR_SOUTH);
    let terrain_z_i16 = terrain_z as i16;
    let direct_y = if bridge.direction_zero {
        !dir4_is_bridge
    } else {
        !dir4_is_bridge && (terrain_z_i16 - dir4_height as i16).abs() <= 1
    };
    let direct_x = if bridge.direction_zero {
        !dir2_is_bridge && (terrain_z_i16 - dir2_height as i16).abs() <= 1
    } else {
        !dir2_is_bridge
    };

    let (bridge_ref_x, bridge_ref_y) = iso_to_screen(cell_rx, cell_ry, bridge.deck_z);
    if bridge_ref_y <= input_y {
        if direct_y {
            return Some(TacticalInverseResult::Cell {
                rx: cell_rx as f32,
                ry: cell_ry.saturating_add(1) as f32,
            });
        }
        if direct_x {
            return Some(TacticalInverseResult::Cell {
                rx: cell_rx.saturating_add(1) as f32,
                ry: cell_ry as f32,
            });
        }
    }

    let input_x_delta = input_x - bridge_ref_x;
    let input_y_delta = input_y - bridge_ref_y;
    let apply_extra_bridge_lift = if dir0_open_edge {
        input_y_delta - input_x_delta / 2.0 > TACTICAL_BRIDGE_EDGE_THRESHOLD_PX
    } else if dir6_open_edge {
        input_y_delta + input_x_delta / 2.0 > TACTICAL_BRIDGE_EDGE_THRESHOLD_PX
    } else {
        true
    };
    if apply_extra_bridge_lift {
        *adjusted_scan_y =
            scan_y - terrain_z as f32 * HEIGHT_STEP - TACTICAL_BRIDGE_EXTRA_HEIGHT_PX;
    }
    None
}

fn tactical_bridge_neighbor(
    bridge_cells: &BTreeMap<(u16, u16), TacticalBridgeCell>,
    rx: u16,
    ry: u16,
    direction: u8,
) -> Option<TacticalBridgeCell> {
    let (nx, ny) = tactical_cardinal_neighbor(rx, ry, direction)?;
    bridge_cells.get(&(nx, ny)).copied()
}

fn tactical_neighbor_height(
    height_map: &BTreeMap<(u16, u16), u8>,
    rx: u16,
    ry: u16,
    direction: u8,
) -> u8 {
    tactical_cardinal_neighbor(rx, ry, direction)
        .and_then(|pos| height_map.get(&pos).copied())
        .unwrap_or(0)
}

fn tactical_cardinal_neighbor(rx: u16, ry: u16, direction: u8) -> Option<(u16, u16)> {
    let (dx, dy) = match direction {
        DIR_NORTH => (0_i32, -1_i32),
        DIR_EAST => (1, 0),
        DIR_SOUTH => (0, 1),
        DIR_WEST => (-1, 0),
        _ => return None,
    };
    let nx = rx as i32 + dx;
    let ny = ry as i32 + dy;
    if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
        return None;
    }
    Some((nx as u16, ny as u16))
}

/// Convert screen-space pixel position to isometric cell, accounting for terrain elevation.
///
/// Iteratively refines the cell guess by looking up the actual terrain height at each
/// candidate cell and re-solving with the corrected screen Y. This fixes the z=0
/// assumption in `screen_to_iso` which causes clicks on elevated terrain to resolve
/// to the wrong cell.
///
/// Converges in 1-3 iterations on typical RA2 terrain gradients.
pub fn screen_to_iso_with_height(
    screen_x: f32,
    screen_y: f32,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> (f32, f32) {
    screen_to_iso_with_height_and_bridges(screen_x, screen_y, height_map, None)
}

/// Convert screen coordinates to isometric cell coordinates, accounting for
/// terrain height and optionally bridge deck height.
///
/// When `bridge_height_map` is provided and the resolved cell has a bridge deck,
/// the bridge deck elevation is used instead of the ground elevation. This makes
/// clicks on high bridge surfaces resolve to the correct cell.
pub fn screen_to_iso_with_height_and_bridges(
    screen_x: f32,
    screen_y: f32,
    height_map: &BTreeMap<(u16, u16), u8>,
    bridge_height_map: Option<&BTreeMap<(u16, u16), u8>>,
) -> (f32, f32) {
    // First pass: resolve using ground height (existing behavior).
    let (mut rx, mut ry) = screen_to_iso(screen_x, screen_y);
    for _ in 0..3 {
        let cell_rx: u16 = rx.round().max(0.0) as u16;
        let cell_ry: u16 = ry.round().max(0.0) as u16;
        let z: u8 = height_map.get(&(cell_rx, cell_ry)).copied().unwrap_or(0);
        if z == 0 {
            break;
        }
        let corrected_y: f32 = screen_y + z as f32 * HEIGHT_STEP;
        let (new_rx, new_ry) = screen_to_iso(screen_x, corrected_y);
        if (new_rx - rx).abs() < 0.01 && (new_ry - ry).abs() < 0.01 {
            break;
        }
        rx = new_rx;
        ry = new_ry;
    }

    // Second pass: bridge deck click resolution. The bridge surface is elevated
    // (deck_level = ground + 4), so the ground resolution above can shift the
    // result by up to ~3 cells away from the actual bridge cell. We search a
    // neighborhood around the ground-resolved cell for any bridge entries and
    // test each at its deck height. The closest match wins.
    if let Some(bridge_map) = bridge_height_map {
        let cell_rx: u16 = rx.round().max(0.0) as u16;
        let cell_ry: u16 = ry.round().max(0.0) as u16;
        let mut best: Option<(f32, f32)> = None;
        let mut best_dist: f32 = f32::MAX;
        for dy in -3i32..=3 {
            for dx in -3i32..=3 {
                let bx_i: i32 = cell_rx as i32 + dx;
                let by_i: i32 = cell_ry as i32 + dy;
                if bx_i < 0 || by_i < 0 {
                    continue;
                }
                let bx: u16 = bx_i as u16;
                let by: u16 = by_i as u16;
                if let Some(&bridge_z) = bridge_map.get(&(bx, by)) {
                    let corrected_y: f32 = screen_y + bridge_z as f32 * HEIGHT_STEP;
                    let (new_rx, new_ry) = screen_to_iso(screen_x, corrected_y);
                    let dist: f32 = (new_rx - bx as f32).abs() + (new_ry - by as f32).abs();
                    if dist < 0.7 && dist < best_dist {
                        best = Some((new_rx, new_ry));
                        best_dist = dist;
                    }
                }
            }
        }
        if let Some((brx, bry)) = best {
            rx = brx;
            ry = bry;
        }
    }

    (rx, ry)
}

/// Build a TerrainGrid from a parsed map file.
///
/// Converts all map cells to screen coordinates, computes world bounds,
/// and sorts by screen_y for correct draw order. Cells outside the
/// LocalSize playable area are clipped (they are filler tiles hidden
/// by shroud in the original RA2 engine).
pub fn build_terrain_grid(map: &MapFile, local_bounds: Option<LocalBounds>) -> TerrainGrid {
    let mut cells: Vec<TerrainCell> = Vec::with_capacity(map.cells.len());
    let mut min_x: f32 = f32::MAX;
    let mut min_y: f32 = f32::MAX;
    let mut max_x: f32 = f32::MIN;
    let mut max_y: f32 = f32::MIN;
    let mut clipped: u32 = 0;

    for cell in &map.cells {
        // Skip true "no tile" entries: -1 (0xFFFFFFFF).
        // Some maps use 0x0000FFFF as "clear ground" (legacy 16-bit sentinel).
        // Treat that as tile 0 so we don't render black holes in otherwise valid cells.
        if cell.tile_index < 0 {
            continue;
        }
        let tile_id: u16 = if cell.tile_index == 0xFFFF {
            0
        } else {
            cell.tile_index as u16
        };

        let (sx, sy): (f32, f32) = iso_to_screen(cell.rx, cell.ry, cell.z);

        // Clip cells outside the playable area (LocalSize bounds).
        // In RA2, these border cells are hidden under permanent shroud.
        if let Some(ref bounds) = local_bounds {
            if !bounds.contains(sx, sy) {
                clipped += 1;
                continue;
            }
        }

        cells.push(TerrainCell {
            screen_x: sx,
            screen_y: sy,
            tile_id,
            sub_tile: cell.sub_tile,
            z: cell.z,
            rx: cell.rx,
            ry: cell.ry,
            is_water: tile_id == 0,
            is_cliff_redraw: false,
            variant: 0,
            tint: [1.0, 1.0, 1.0],
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
        });

        min_x = min_x.min(sx);
        min_y = min_y.min(sy);
        max_x = max_x.max(sx + TILE_WIDTH);
        max_y = max_y.max(sy + TILE_HEIGHT);
    }

    if clipped > 0 {
        log::info!(
            "LocalSize clip: {} cells kept, {} clipped (outside playable area)",
            cells.len(),
            clipped,
        );
    }

    // Sort by screen_y for back-to-front draw order.
    cells.sort_by(|a, b| {
        a.screen_y
            .partial_cmp(&b.screen_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    TerrainGrid {
        cells,
        world_width: max_x - min_x,
        world_height: max_y - min_y,
        origin_x: min_x,
        origin_y: min_y,
        local_bounds,
        anchor_variant_table: None,
    }
}

/// Build a TerrainGrid from the resolved terrain stage.
///
/// Unlike `build_terrain_grid()`, this consumes the final LAT-adjusted tile
/// choice and retains water classification from resolved terrain metadata.
pub fn build_terrain_grid_from_resolved(
    resolved: &ResolvedTerrainGrid,
    local_bounds: Option<LocalBounds>,
    anchor_variant_table: Option<crate::map::theater::BridgeAnchorVariantTable>,
) -> TerrainGrid {
    let mut cells: Vec<TerrainCell> = Vec::with_capacity(resolved.cells.len());
    let mut min_x: f32 = f32::MAX;
    let mut min_y: f32 = f32::MAX;
    let mut max_x: f32 = f32::MIN;
    let mut max_y: f32 = f32::MIN;
    let mut clipped: u32 = 0;

    for cell in resolved.iter() {
        if cell.final_tile_index < 0 {
            continue;
        }
        let tile_id = if cell.final_tile_index == 0xFFFF {
            0
        } else {
            cell.final_tile_index as u16
        };
        let (sx, sy) = iso_to_screen(cell.rx, cell.ry, cell.level);
        if let Some(ref bounds) = local_bounds {
            if !bounds.contains(sx, sy) {
                clipped += 1;
                continue;
            }
        }
        cells.push(TerrainCell {
            screen_x: sx,
            screen_y: sy,
            tile_id,
            sub_tile: cell.final_sub_tile,
            z: cell.level,
            rx: cell.rx,
            ry: cell.ry,
            is_water: cell.is_water,
            is_cliff_redraw: cell.is_cliff_redraw,
            variant: cell.variant,
            tint: [1.0, 1.0, 1.0],
            radar_left: cell.radar_left,
            radar_right: cell.radar_right,
            has_damaged_data: cell.has_damaged_data,
        });
        min_x = min_x.min(sx);
        min_y = min_y.min(sy);
        max_x = max_x.max(sx + TILE_WIDTH);
        max_y = max_y.max(sy + TILE_HEIGHT);
    }

    if clipped > 0 {
        log::info!(
            "LocalSize clip: {} resolved cells kept, {} clipped (outside playable area)",
            cells.len(),
            clipped,
        );
    }

    cells.sort_by(|a, b| {
        a.screen_y
            .partial_cmp(&b.screen_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    TerrainGrid {
        cells,
        world_width: max_x - min_x,
        world_height: max_y - min_y,
        origin_x: min_x,
        origin_y: min_y,
        local_bounds,
        anchor_variant_table,
    }
}

/// Terrain instance sets: normal terrain drawn behind entities, and cliff-redraw
/// terrain drawn after entities so cliff face pixels occlude units behind them.
/// The cliff-redraw set contains copies of flagged tiles with a depth bias that
/// places them in front of entities in the depth buffer.
pub struct TerrainInstances {
    /// Normal terrain — drawn in the first pass (behind entities).
    pub normal: Vec<SpriteInstance>,
    /// Cliff-redraw terrain — drawn after entities (cliff occlusion pass).
    pub cliff_redraw: Vec<SpriteInstance>,
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
        cliff_redraw: Vec::new(),
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
        let iso_row: f32 = cell.screen_y + cell.z as f32 * HEIGHT_STEP;
        let normalized: f32 = ((iso_row - grid.origin_y) / grid.world_height).clamp(0.0, 1.0);
        let z_bias: f32 = cell.z as f32 * 0.0001;
        let depth: f32 = (1.0 - normalized - z_bias).clamp(0.001, 0.999);

        // Bridge cells with baked damaged-variant TMP data ignore the FA2
        // map-load PRNG variant and instead route to the per-frame
        // damaged_variant bool from the sim's BridgeRuntimeState. Variant 1
        // is the damaged baked art; variant 0 is the pristine art.
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
            // Cliff-redraw: same tile redrawn AFTER sprites using zdepth shader
            // with Less compare. Only cliff face pixels (z_sample > 0) pass the
            // test — flat ground pixels have equal depth and fail Less, preserving
            // sprites near cliff edges.
            if cell.is_cliff_redraw {
                instances.cliff_redraw.push(inst);
            }
        }
    }

    instances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_to_screen_origin() {
        // (0,0,0) → X = 0-30 = -30, Y = 0+15 = 15
        let (sx, sy): (f32, f32) = iso_to_screen(0, 0, 0);
        assert!((sx - (-30.0)).abs() < f32::EPSILON);
        assert!((sy - 15.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_iso_to_screen_positive() {
        // rx=10, ry=0, z=0 → sx = 300-30 = 270, sy = 150+15 = 165
        let (sx, sy): (f32, f32) = iso_to_screen(10, 0, 0);
        assert!((sx - 270.0).abs() < f32::EPSILON);
        assert!((sy - 165.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_iso_to_screen_diagonal() {
        // rx=5, ry=5, z=0 → sx = 0-30 = -30, sy = 150+15 = 165
        let (sx, sy): (f32, f32) = iso_to_screen(5, 5, 0);
        assert!((sx - (-30.0)).abs() < f32::EPSILON);
        assert!((sy - 165.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_iso_to_screen_elevation() {
        // rx=0, ry=0, z=2 → sx = -30, sy = 15 - 30 = -15
        let (sx, sy): (f32, f32) = iso_to_screen(0, 0, 2);
        assert!((sx - (-30.0)).abs() < f32::EPSILON);
        assert!((sy - (-15.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn screen_to_cell_tactical_inverse_uses_vertical_height_scan() {
        let mut height_map = BTreeMap::new();
        height_map.insert((10, 5), 4);
        let result = screen_to_cell_tactical_inverse(
            150.0,
            180.0,
            TacticalInverseContext {
                height_map: &height_map,
                bridge_cells: None,
                viewport_offset_x: 0.0,
                viewport_offset_y: 0.0,
            },
        );

        assert_eq!(result, TacticalInverseResult::Cell { rx: 9.5, ry: 4.5 });
    }

    #[test]
    fn screen_to_cell_tactical_inverse_returns_initial_fallback_on_scan_cap() {
        let height_map = BTreeMap::new();
        let result = screen_to_cell_tactical_inverse(
            -500.0,
            -500.0,
            TacticalInverseContext {
                height_map: &height_map,
                bridge_cells: None,
                viewport_offset_x: 0.0,
                viewport_offset_y: 0.0,
            },
        );

        let (rx, ry) = screen_to_iso(-500.0, -500.0);
        assert_eq!(result, TacticalInverseResult::Fallback { rx, ry });
    }

    #[test]
    fn tactical_cardinal_neighbor_rejects_direction_eight() {
        assert_eq!(tactical_cardinal_neighbor(10, 10, 8), None);
    }

    #[test]
    fn tactical_bridge_edge_threshold_is_strict() {
        let height_map = BTreeMap::from([((0, 0), 0)]);
        let mut bridge_cells = BTreeMap::new();
        bridge_cells.insert(
            (0, 0),
            TacticalBridgeCell {
                deck_z: 0,
                structural: true,
                direction_zero: true,
            },
        );
        bridge_cells.insert(
            (1, 0),
            TacticalBridgeCell {
                deck_z: 0,
                structural: true,
                direction_zero: true,
            },
        );
        bridge_cells.insert(
            (0, 1),
            TacticalBridgeCell {
                deck_z: 0,
                structural: true,
                direction_zero: true,
            },
        );

        let mut adjusted = 90.0;
        assert_eq!(
            apply_tactical_bridge_inverse(
                -30.0,
                30.0,
                90.0,
                0,
                0,
                0,
                TacticalInverseContext {
                    height_map: &height_map,
                    bridge_cells: Some(&bridge_cells),
                    viewport_offset_x: 0.0,
                    viewport_offset_y: 0.0,
                },
                &mut adjusted,
            ),
            None
        );
        assert_eq!(adjusted, 90.0);

        let mut adjusted = 90.0;
        assert_eq!(
            apply_tactical_bridge_inverse(
                -30.0,
                31.0,
                90.0,
                0,
                0,
                0,
                TacticalInverseContext {
                    height_map: &height_map,
                    bridge_cells: Some(&bridge_cells),
                    viewport_offset_x: 0.0,
                    viewport_offset_y: 0.0,
                },
                &mut adjusted,
            ),
            None
        );
        assert_eq!(adjusted, 30.0);
    }

    #[test]
    fn test_local_bounds_from_header_dustbowl() {
        // Dustbowl: Size=70x76, LocalSize=2,8,65,62
        let header = MapHeader {
            theater: "TEMPERATE".to_string(),
            width: 70,
            height: 76,
            local_left: 2,
            local_top: 8,
            local_width: 65,
            local_height: 62,
        };
        let bounds: LocalBounds = LocalBounds::from_header(&header);
        // TS-scale pixel rect: x=2*48=96, y=(8-3)*24=120, w=65*48=3120, h=(62+5)*24=1608
        // Our coords: x = 96*1.25 - 69*30 = -1950, y = 120*1.25 + 71*15 = 1215
        // w = 3120*1.25 = 3900, h = 1608*1.25 = 2010
        assert!((bounds.pixel_x - (-1950.0)).abs() < 1.0);
        assert!((bounds.pixel_y - 1215.0).abs() < 1.0);
        assert!((bounds.pixel_w - 3900.0).abs() < 1.0);
        assert!((bounds.pixel_h - 2010.0).abs() < 1.0);
    }

    #[test]
    fn test_local_bounds_contains() {
        let bounds = LocalBounds {
            pixel_x: -1950.0,
            pixel_y: 1215.0,
            pixel_w: 3900.0,
            pixel_h: 2010.0,
        };
        assert!(bounds.contains(-1950.0, 1215.0)); // top-left (inclusive)
        assert!(bounds.contains(0.0, 2000.0)); // center
        assert!(!bounds.contains(-1951.0, 1215.0)); // just left
        assert!(!bounds.contains(-1950.0, 1214.0)); // just above
        assert!(!bounds.contains(1950.0, 1215.0)); // at right edge (exclusive)
        assert!(!bounds.contains(-1950.0, 3225.0)); // at bottom edge (exclusive)
    }

    #[test]
    fn test_build_visible_instances_culling() {
        // Create a small grid manually.
        let grid: TerrainGrid = TerrainGrid {
            cells: vec![
                TerrainCell {
                    screen_x: 0.0,
                    screen_y: 0.0,
                    tile_id: 0,
                    sub_tile: 0,
                    z: 0,
                    rx: 1,
                    ry: 0,
                    is_water: false,
                    is_cliff_redraw: false,
                    variant: 0,
                    tint: [1.0, 1.0, 1.0],
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                },
                TerrainCell {
                    screen_x: 5000.0,
                    screen_y: 5000.0,
                    tile_id: 0,
                    sub_tile: 0,
                    z: 0,
                    rx: 100,
                    ry: 100,
                    is_water: false,
                    is_cliff_redraw: false,
                    variant: 0,
                    tint: [1.0, 1.0, 1.0],
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                },
            ],
            world_width: 5060.0,
            world_height: 5030.0,
            origin_x: 0.0,
            origin_y: 0.0,
            local_bounds: None,
            anchor_variant_table: None,
        };

        // Camera at origin, 1024x768 viewport — only first cell should be visible.
        let result: TerrainInstances =
            build_visible_instances(&grid, None, 0.0, 0.0, 1024.0, 768.0, None, None, None);
        assert_eq!(result.normal.len(), 1);
        assert_eq!(result.cliff_redraw.len(), 0);
    }

    #[test]
    fn terrain_tile_instances_consume_per_cell_lighting() {
        let grid: TerrainGrid = TerrainGrid {
            cells: vec![
                TerrainCell {
                    screen_x: 0.0,
                    screen_y: 0.0,
                    tile_id: 0,
                    sub_tile: 0,
                    z: 0,
                    rx: 1,
                    ry: 0,
                    is_water: false,
                    is_cliff_redraw: false,
                    variant: 0,
                    tint: [1.0, 1.0, 1.0],
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                },
                TerrainCell {
                    screen_x: 60.0,
                    screen_y: 0.0,
                    tile_id: 0,
                    sub_tile: 0,
                    z: 4,
                    rx: 2,
                    ry: 0,
                    is_water: false,
                    is_cliff_redraw: false,
                    variant: 0,
                    tint: [1.0, 1.0, 1.0],
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                },
            ],
            world_width: 120.0,
            world_height: 30.0,
            origin_x: 0.0,
            origin_y: 0.0,
            local_bounds: None,
            anchor_variant_table: None,
        };
        let lights = crate::map::lighting::build_cell_light_grid_from_heights(
            [((1, 0), 0), ((2, 0), 4)],
            &crate::map::lighting::LightingConfig::default(),
        );

        let result = build_visible_instances(
            &grid,
            Some(&lights),
            0.0,
            0.0,
            1024.0,
            768.0,
            None,
            None,
            None,
        );

        assert_eq!(result.normal.len(), 2);
        assert!((result.normal[0].tint[0] - 0.95).abs() < 0.001);
        assert!((result.normal[1].tint[0] - 0.982).abs() < 0.001);
    }

    fn override_test_grid(
        anchor_variant_table: Option<crate::map::theater::BridgeAnchorVariantTable>,
    ) -> TerrainGrid {
        TerrainGrid {
            cells: vec![TerrainCell {
                screen_x: 0.0,
                screen_y: 0.0,
                tile_id: 100,
                sub_tile: 0,
                z: 0,
                rx: 0,
                ry: 0,
                is_water: false,
                is_cliff_redraw: false,
                variant: 0,
                tint: [1.0; 3],
                radar_left: [0; 3],
                radar_right: [0; 3],
                has_damaged_data: false,
            }],
            world_width: TILE_WIDTH,
            world_height: TILE_HEIGHT,
            origin_x: 0.0,
            origin_y: 0.0,
            local_bounds: None,
            anchor_variant_table,
        }
    }

    fn override_test_bridge_state(
        axis: Option<crate::sim::bridge_state::Axis>,
        class: crate::sim::bridge_state::BridgeheadAnchorClass,
    ) -> crate::sim::bridge_state::BridgeRuntimeState {
        use crate::sim::bridge_state::{
            BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState, DamageState,
        };
        let mut bs = BridgeRuntimeState::default();
        bs.test_seed_cell(
            0,
            0,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis,
                role: BridgeCellRole::Anchor,
                anchor_span_id: Some(1),
                overlay_byte: 0,
                damaged_variant: false,
                bridgehead_anchor_class: class,
            },
        );
        bs
    }

    #[test]
    fn override_fires_when_class_is_aboutto_fall_with_table() {
        use crate::map::theater::BridgeAnchorVariantTable;
        use crate::sim::bridge_state::{Axis, BridgeheadAnchorClass};

        let table = BridgeAnchorVariantTable {
            ns: [200, 201, 202, 203],
            ew: [300, 301, 302, 303],
        };
        let grid = override_test_grid(Some(table));
        let bs = override_test_bridge_state(Some(Axis::NS), BridgeheadAnchorClass::AboutToFall);

        let captured: std::cell::RefCell<Option<(u16, u8, u8)>> = std::cell::RefCell::new(None);
        let lookup = |tid: u16, sub: u8, var: u8| -> Option<TilePlacement> {
            *captured.borrow_mut() = Some((tid, sub, var));
            Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [1.0, 1.0],
                pixel_size: [TILE_WIDTH, TILE_HEIGHT],
                draw_offset: [0.0, 0.0],
            })
        };
        let uv_fn: UvLookupFn = Some(&lookup);

        let _ =
            build_visible_instances(&grid, None, 0.0, 0.0, 1024.0, 768.0, uv_fn, None, Some(&bs));
        let (tid, sub, var) = captured.borrow().expect("uv_fn was called");
        // Override fired: tile_id = NS AboutToFall slot = 203.
        assert_eq!(tid, 203);
        // Sub-tile preserved.
        assert_eq!(sub, 0);
        // FA2 sibling-TMP slot reset to 0 on variant tiles.
        assert_eq!(var, 0);
    }

    #[test]
    fn override_bypassed_when_class_is_variant0() {
        use crate::map::theater::BridgeAnchorVariantTable;
        use crate::sim::bridge_state::{Axis, BridgeheadAnchorClass};

        let table = BridgeAnchorVariantTable {
            ns: [200, 201, 202, 203],
            ew: [300, 301, 302, 303],
        };
        let grid = override_test_grid(Some(table));
        let bs = override_test_bridge_state(Some(Axis::NS), BridgeheadAnchorClass::Variant0);

        let captured: std::cell::RefCell<Option<(u16, u8, u8)>> = std::cell::RefCell::new(None);
        let lookup = |tid: u16, sub: u8, var: u8| -> Option<TilePlacement> {
            *captured.borrow_mut() = Some((tid, sub, var));
            Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [1.0, 1.0],
                pixel_size: [TILE_WIDTH, TILE_HEIGHT],
                draw_offset: [0.0, 0.0],
            })
        };
        let uv_fn: UvLookupFn = Some(&lookup);

        let _ =
            build_visible_instances(&grid, None, 0.0, 0.0, 1024.0, 768.0, uv_fn, None, Some(&bs));
        let (tid, _sub, _var) = captured.borrow().expect("uv_fn was called");
        // Override bypassed: native tile_id retained.
        assert_eq!(tid, 100);
    }

    #[test]
    fn override_bypassed_when_table_is_none() {
        use crate::sim::bridge_state::{Axis, BridgeheadAnchorClass};

        let grid = override_test_grid(None);
        let bs = override_test_bridge_state(Some(Axis::NS), BridgeheadAnchorClass::AboutToFall);

        let captured: std::cell::RefCell<Option<(u16, u8, u8)>> = std::cell::RefCell::new(None);
        let lookup = |tid: u16, sub: u8, var: u8| -> Option<TilePlacement> {
            *captured.borrow_mut() = Some((tid, sub, var));
            Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [1.0, 1.0],
                pixel_size: [TILE_WIDTH, TILE_HEIGHT],
                draw_offset: [0.0, 0.0],
            })
        };
        let uv_fn: UvLookupFn = Some(&lookup);

        let _ =
            build_visible_instances(&grid, None, 0.0, 0.0, 1024.0, 768.0, uv_fn, None, Some(&bs));
        let (tid, _sub, _var) = captured.borrow().expect("uv_fn was called");
        // Override bypassed (no table): native tile_id retained.
        assert_eq!(tid, 100);
    }

    #[test]
    fn test_screen_to_iso_with_height_flat_terrain() {
        // On flat terrain (z=0 everywhere), result matches plain screen_to_iso.
        let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
        let (rx, ry) = screen_to_iso_with_height(300.0, 150.0, &height_map);
        let (rx0, ry0) = screen_to_iso(300.0, 150.0);
        assert!((rx - rx0).abs() < 0.01);
        assert!((ry - ry0).abs() < 0.01);
    }

    #[test]
    fn test_screen_to_iso_with_height_elevated() {
        // Cell (10, 5) at z=4:
        //   iso_to_screen = ((10-5)*30-30, (10+5)*15+15-4*15) = (120, 165)
        //   Tile center = (120+30, 165+15) = (150, 180)
        let mut height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
        for rx in 8..=12 {
            for ry in 3..=7 {
                height_map.insert((rx, ry), 4);
            }
        }
        let (rx, ry) = screen_to_iso_with_height(150.0, 180.0, &height_map);
        assert!((rx - 10.0).abs() < 0.6, "rx={rx}, expected ~10");
        assert!((ry - 5.0).abs() < 0.6, "ry={ry}, expected ~5");
    }

    #[test]
    fn test_screen_to_iso_with_height_convergence() {
        // Verify the function converges and doesn't overshoot on steep terrain.
        let mut height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
        // A ridge: cells with ry < 10 are at z=6, ry >= 10 at z=0.
        for rx in 0..30 {
            for ry in 0..10 {
                height_map.insert((rx, ry), 6);
            }
        }
        // Click on the elevated part: cell (15, 5) at z=6.
        // iso_to_screen = ((15-5)*30-30, (15+5)*15+15-6*15) = (270, 225)
        // Center: (270+30, 225+15) = (300, 240).
        let (rx, ry) = screen_to_iso_with_height(300.0, 240.0, &height_map);
        assert!((rx - 15.0).abs() < 0.6, "rx={rx}, expected ~15");
        assert!((ry - 5.0).abs() < 0.6, "ry={ry}, expected ~5");
    }

    #[test]
    fn lepton_to_screen_zero_matches_iso_origin() {
        let (sx, sy) = lepton_to_screen(glam::IVec3::ZERO);
        assert_eq!(sx, 0.0);
        assert_eq!(sy, TILE_HEIGHT / 2.0);
    }

    #[test]
    fn lepton_to_screen_integer_cell_lands_at_iso_center() {
        // 4 cells east, 2 cells south = (4*256, 2*256, 0).
        let (sx, sy) = lepton_to_screen(glam::IVec3::new(4 * 256, 2 * 256, 0));
        assert_eq!(sx, (4.0 - 2.0) * TILE_WIDTH / 2.0);
        assert_eq!(sy, (4.0 + 2.0) * TILE_HEIGHT / 2.0 + TILE_HEIGHT / 2.0);
    }

    #[test]
    fn lepton_to_screen_sub_cell_offset_is_iso_subdivided() {
        // Sub-cell offset of (128, 0) — half a cell east in lepton coords.
        let (sx, sy) = lepton_to_screen(glam::IVec3::new(128, 0, 0));
        assert!((sx - (128.0 - 0.0) * (TILE_WIDTH / 2.0) / 256.0).abs() < 1e-3);
        assert!((sy - (TILE_HEIGHT / 2.0 + (128.0) * (TILE_HEIGHT / 2.0) / 256.0)).abs() < 1e-3);
    }

    #[test]
    fn lepton_to_screen_negative_coords_use_euclidean_rounding() {
        // A particle at -50 leptons (west of origin) should land just west of cell 0.
        // div_euclid(-50, 256) = -1, rem_euclid = 206. Screen X = -30 + 206*30/256 ≈ -5.86.
        let (sx, _sy) = lepton_to_screen(glam::IVec3::new(-50, 0, 0));
        assert!(sx < 0.0, "sx={sx}");
        assert!(sx > -10.0, "sx={sx}");
    }

    #[test]
    fn lepton_to_screen_z_lift_uses_height_step() {
        // Z = 256 leptons = 1 cell of altitude → screen Y lifted by HEIGHT_STEP.
        let (_, sy_low) = lepton_to_screen(glam::IVec3::ZERO);
        let (_, sy_high) = lepton_to_screen(glam::IVec3::new(0, 0, 256));
        assert!((sy_low - sy_high - HEIGHT_STEP).abs() < 1e-3);
    }
}
