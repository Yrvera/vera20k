//! Minimap helper types, color constants, and pixel-level utility functions.
//!
//! Extracted from minimap.rs for file-size limits. Contains color definitions,
//! overlay classification, coordinate mapping, and pixel buffer operations.
//!
//! ## Dependency rules
//! - Part of render/ — depends on map/terrain, map/houses, rules/house_colors, sim/vision.

use crate::map::houses::HouseColorMap;
use crate::rules::house_colors::{HouseColorIndex, HouseColorRamps, NO_REMAP};
use crate::sim::intern::InternedId;
use crate::sim::vision::FogState;

use super::native_radar_surface::{RADAR_APERTURE_HEIGHT, RADAR_APERTURE_WIDTH};

pub(super) const MINIMAP_WIDTH: u32 = RADAR_APERTURE_WIDTH;
pub(super) const MINIMAP_HEIGHT: u32 = RADAR_APERTURE_HEIGHT;

/// Margin from the screen edges in pixels.
pub(super) const MINIMAP_MARGIN: f32 = 10.0;

/// Depth value for minimap elements — always drawn in front of everything.
pub(super) const MINIMAP_DEPTH: f32 = 0.0;

/// Thickness of the viewport rectangle outline in pixels.
pub(super) const VIEWPORT_LINE_THICKNESS: f32 = 2.0;

// Fallback terrain colors when TMP radar colors are absent (RGBA).
/// Ground/land color: dark green.
const COLOR_LAND: [u8; 4] = [40, 120, 40, 255];
/// Water color: dark blue.
const COLOR_WATER: [u8; 4] = [30, 50, 120, 255];
/// Elevated terrain color: brown.
const COLOR_ELEVATED: [u8; 4] = [120, 90, 40, 255];
/// Shrouded / unexplored area color (pure black, matches original PutPixel(0)).
pub(super) const COLOR_SHROUD: [u8; 4] = [0, 0, 0, 255];

// Overlay minimap colors (RGBA) — drawn on top of terrain for map objects.
// Ore/gem radar colors vary by density (0-11). These are endpoint colors for
// linear interpolation, approximating the original's per-density tileset lookup.
const COLOR_ORE_LO: [u8; 4] = [130, 100, 20, 255];
const COLOR_ORE_HI: [u8; 4] = [240, 200, 40, 255];
const COLOR_GEM_LO: [u8; 4] = [100, 30, 130, 255];
const COLOR_GEM_HI: [u8; 4] = [220, 80, 255, 255];
/// Maximum density index for ore/gem overlays.
const MAX_DENSITY: f32 = 11.0;
/// Wall overlay color: olive grey (matches original 0xAA,0xAA,0x82).
const COLOR_WALL: [u8; 4] = [170, 170, 130, 255];
/// Bridge deck overlay color: brown.
const COLOR_BRIDGE: [u8; 4] = [140, 100, 50, 255];
/// Terrain/background object color from CellClass::GetRadarColor. Live
/// Building tracker pixels use their owner's remap color instead.
const COLOR_TERRAIN_OBJ: [u8; 4] = [200, 200, 160, 255];

/// Fog-of-war dimming factor for revealed (previously seen) cells.
/// Original engine uses SHR 1 = exact halving.

/// Terrain brightness multiplier for most theaters (SHR 1 = halving).
const TERRAIN_BRIGHTNESS_DEFAULT: f32 = 0.5;
/// Terrain brightness multiplier for URBAN theater (0.8 theater brightness × 0.5 halving).
const TERRAIN_BRIGHTNESS_URBAN: f32 = 0.4;

/// Classification of an overlay for minimap coloring.
///
/// Defined in render/ so that the minimap doesn't depend on map/overlay_types.
/// The caller (app layer) maps `OverlayTypeFlags` to this enum via a closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayClassification {
    Ore,
    Gem,
    Wall,
    Bridge,
    /// Trees, rocks, and other terrain objects.
    TerrainObject,
    /// Non-rendered overlay (unknown or not worth showing).
    Other,
}

impl OverlayClassification {
    /// Get the radar color for this overlay, using density for ore/gem gradients.
    pub(super) fn color(self, density: u8) -> Option<[u8; 4]> {
        match self {
            Self::Ore => Some(lerp_color(COLOR_ORE_LO, COLOR_ORE_HI, density)),
            Self::Gem => Some(lerp_color(COLOR_GEM_LO, COLOR_GEM_HI, density)),
            Self::Wall => Some(COLOR_WALL),
            Self::Bridge => Some(COLOR_BRIDGE),
            Self::TerrainObject => Some(COLOR_TERRAIN_OBJ),
            Self::Other => None,
        }
    }
}

/// Linearly interpolate between two colors based on density (0..11).
fn lerp_color(lo: [u8; 4], hi: [u8; 4], density: u8) -> [u8; 4] {
    let t = (density as f32 / MAX_DENSITY).clamp(0.0, 1.0);
    [
        (lo[0] as f32 + (hi[0] as f32 - lo[0] as f32) * t) as u8,
        (lo[1] as f32 + (hi[1] as f32 - lo[1] as f32) * t) as u8,
        (lo[2] as f32 + (hi[2] as f32 - lo[2] as f32) * t) as u8,
        255,
    ]
}

/// An overlay pixel to stamp on the minimap (pre-computed at init time).
#[derive(Clone, Copy)]
pub(super) struct OverlayPixel {
    pub rx: u16,
    pub ry: u16,
    pub px: u32,
    pub py: u32,
    pub color: [u8; 4],
    pub classification: OverlayClassification,
}

/// A terrain pixel with pre-computed minimap position and color.
#[derive(Clone, Copy)]
pub(super) struct TerrainPixel {
    pub rx: u16,
    pub ry: u16,
    pub px: u32,
    pub py: u32,
    pub color: [u8; 4],
}

/// One generated-primary terrain pixel plus the exact cell gamemd queries for
/// shroud/fog when `RenderCellPixel @ 0x00655C50` refreshes that pixel.
#[derive(Clone, Copy)]
pub(super) struct RadarSurfacePixel {
    pub rx: u16,
    pub ry: u16,
    pub px: u32,
    pub py: u32,
    pub packed_rgb565: u16,
    pub color: [u8; 4],
}

/// Compute the minimap color for a terrain cell.
///
/// Uses per-tile radar colors from TMP files when available (RadarLeft/RadarRight
/// RGB triplets baked into each tile cell header). Falls back to hardcoded colors
/// when TMP data is absent (both triplets are [0,0,0]).
pub(super) fn radar_color_for_cell(
    cell: &crate::map::terrain::TerrainCell,
    terrain_brightness: f32,
) -> [u8; 4] {
    let (left, right) = radar_colors_for_cell(cell, terrain_brightness);
    [
        ((u16::from(left[0]) + u16::from(right[0])) / 2) as u8,
        ((u16::from(left[1]) + u16::from(right[1])) / 2) as u8,
        ((u16::from(left[2]) + u16::from(right[2])) / 2) as u8,
        255,
    ]
}

/// Preserve the two raw half-pixel colors consumed by
/// `FillTerrainColors @ 0x00654EA0`; averaging is only for legacy metadata.
pub(super) fn radar_colors_for_cell(
    cell: &crate::map::terrain::TerrainCell,
    terrain_brightness: f32,
) -> ([u8; 3], [u8; 3]) {
    let has_radar_colors: bool = cell.radar_left != [0, 0, 0] || cell.radar_right != [0, 0, 0];
    if has_radar_colors {
        let apply = |color: [u8; 3]| {
            color.map(|channel| {
                (channel as f32 * terrain_brightness).clamp(0.0, 255.0) as u8
            })
        };
        (apply(cell.radar_left), apply(cell.radar_right))
    } else if cell.is_water {
        let color = dim_color(COLOR_WATER, terrain_brightness);
        ([color[0], color[1], color[2]], [color[0], color[1], color[2]])
    } else if cell.z > 0 {
        let color = dim_color(COLOR_ELEVATED, terrain_brightness);
        ([color[0], color[1], color[2]], [color[0], color[1], color[2]])
    } else {
        let color = dim_color(COLOR_LAND, terrain_brightness);
        ([color[0], color[1], color[2]], [color[0], color[1], color[2]])
    }
}

/// Headless fallback mapping when no authoritative generated surface exists.
///
/// Returns `(offset_x, offset_y, mapped_w, mapped_h)` — the sub-region of the
/// 140×108 aperture that the world maps to, centered with black margins on the
/// shorter axis.
pub(super) fn compute_aspect_fit(world_w: f32, world_h: f32) -> (f32, f32, f32, f32) {
    let width = MINIMAP_WIDTH as f32;
    let height = MINIMAP_HEIGHT as f32;
    let scale = (width / world_w).min(height / world_h);
    let mapped_w = (world_w * scale).min(width);
    let mapped_h = (world_h * scale).min(height);
    let offset_x = (width - mapped_w) * 0.5;
    let offset_y = (height - mapped_h) * 0.5;
    (offset_x, offset_y, mapped_w, mapped_h)
}

/// Map a world-space position to a minimap pixel coordinate.
///
/// Uses the aspect-fit sub-region `(map_off_x, map_off_y, map_w, map_h)` so
/// the fallback map is centered within the native aperture.
pub(super) fn world_to_minimap_pixel(
    world_x: f32,
    world_y: f32,
    origin_x: f32,
    origin_y: f32,
    world_w: f32,
    world_h: f32,
    map_off_x: f32,
    map_off_y: f32,
    map_w: f32,
    map_h: f32,
) -> (u32, u32) {
    let nx: f32 = (world_x - origin_x) / world_w;
    let ny: f32 = (world_y - origin_y) / world_h;
    let max_x: u32 = MINIMAP_WIDTH.saturating_sub(1);
    let max_y: u32 = MINIMAP_HEIGHT.saturating_sub(1);
    let px: u32 = (nx * map_w + map_off_x).clamp(0.0, max_x as f32) as u32;
    let py: u32 = (ny * map_h + map_off_y).clamp(0.0, max_y as f32) as u32;
    (px, py)
}

/// Set a pixel in an RGBA buffer. Bounds-checked; out-of-range writes are ignored.
pub(super) fn set_pixel(rgba: &mut [u8], width: u32, x: u32, y: u32, color: [u8; 4]) {
    let height = (rgba.len() / 4) as u32 / width.max(1);
    if x >= width || y >= height {
        return;
    }
    let offset: usize = ((y * width + x) * 4) as usize;
    if offset + 3 < rgba.len() {
        rgba[offset] = color[0];
        rgba[offset + 1] = color[1];
        rgba[offset + 2] = color[2];
        rgba[offset + 3] = color[3];
    }
}

/// Draw a line between two points using DDA. Bounds-safe via `set_pixel`.
pub(super) fn draw_line(
    rgba: &mut [u8],
    width: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 4],
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs());
    if steps == 0 {
        set_pixel(rgba, width, x0 as u32, y0 as u32, color);
        return;
    }
    let x_inc = dx as f32 / steps as f32;
    let y_inc = dy as f32 / steps as f32;
    let (mut x, mut y) = (x0 as f32, y0 as f32);
    for _ in 0..=steps {
        set_pixel(rgba, width, x.round() as u32, y.round() as u32, color);
        x += x_inc;
        y += y_inc;
    }
}

/// Map an owner name to a minimap dot color using house color data.
///
/// Looks up the owner's `[Colors]` entry index from the HouseColorMap, then uses
/// shade 0 of that scheme's ramp — the brightest band (palette index 16), which
/// is gamemd's radar color. Falls back to the default scheme for unknown owners.
pub(super) fn owner_dot_color(
    owner: &str,
    house_colors: &HouseColorMap,
    ramps: &HouseColorRamps,
) -> [u8; 4] {
    // Unknown owner → NO_REMAP, which ramp() resolves to the default scheme
    // (matching the producers' DEFAULT_SCHEME_ENTRY fallback), not entry 0.
    let index: HouseColorIndex = house_colors.get(owner).copied().unwrap_or(NO_REMAP);
    let c = ramps.ramp(index)[0];
    [c.r, c.g, c.b, 255]
}

/// Get the terrain color for a pixel based on fog-of-war visibility.
///
/// Visible cells show at full brightness, revealed (previously seen) cells
/// Standard YR (FogOfWar=false): explored cells at full brightness, shrouded = None.
pub(super) fn cell_visibility_color(
    local_owner: InternedId,
    fog: &FogState,
    pixel: &TerrainPixel,
) -> Option<[u8; 4]> {
    // Hostile gap generator: terrain renders black like unexplored shroud.
    // Checked before fog so black wins over half-bright on overlap (native order).
    if fog.is_cell_gap_covered(local_owner, pixel.rx, pixel.ry) {
        return Some(COLOR_SHROUD);
    }
    // Friendly (own/allied) gap generator: half-bright fog over the terrain.
    if fog.is_cell_gap_fog(local_owner, pixel.rx, pixel.ry) {
        return Some(dim_color(pixel.color, 0.5));
    }
    if fog.is_cell_revealed(local_owner, pixel.rx, pixel.ry) {
        Some(pixel.color)
    } else {
        None
    }
}

/// Dim an RGBA color by a brightness factor (0.0 = black, 1.0 = original).
pub(super) fn dim_color(color: [u8; 4], factor: f32) -> [u8; 4] {
    [
        (color[0] as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (color[1] as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (color[2] as f32 * factor).round().clamp(0.0, 255.0) as u8,
        color[3],
    ]
}

/// Get the terrain brightness multiplier for a theater.
///
/// URBAN gets 0.4 (0.8 theater brightness × 0.5 halving), all others get 0.5.
pub(super) fn terrain_brightness_for_theater(theater_name: &str) -> f32 {
    if theater_name.eq_ignore_ascii_case("URBAN") {
        TERRAIN_BRIGHTNESS_URBAN
    } else {
        TERRAIN_BRIGHTNESS_DEFAULT
    }
}

/// Resolve a gamemd foundation name into minimap footprint dimensions.
pub(super) fn parse_foundation_size(foundation: &str) -> (u32, u32) {
    let (w, h) = crate::rules::foundation::foundation_dimensions(foundation);
    (u32::from(w), u32::from(h))
}

/// Check if an entity should be visible on the minimap (test helper).
#[cfg(test)]
pub(super) fn minimap_entity_visible(
    local_owner: InternedId,
    fog: &FogState,
    pos: &crate::sim::components::Position,
    owner: InternedId,
) -> bool {
    let interner = crate::sim::intern::test_interner();
    fog.is_friendly_id(local_owner, owner, &interner)
        || fog.is_cell_revealed(local_owner, pos.rx, pos.ry)
}

/// Default minimap screen rectangle (bottom-left corner with margin).
pub fn default_minimap_rect(screen_h: f32) -> (f32, f32, f32, f32) {
    let mm_w = MINIMAP_WIDTH as f32;
    let mm_h = MINIMAP_HEIGHT as f32;
    let mm_x = MINIMAP_MARGIN;
    let mm_y = screen_h - mm_h - MINIMAP_MARGIN;
    (mm_x, mm_y, mm_w, mm_h)
}

pub(super) fn surface_visibility_color(
    local_owner: InternedId,
    fog: &FogState,
    pixel: &RadarSurfacePixel,
) -> Option<[u8; 4]> {
    if fog.is_cell_gap_covered(local_owner, pixel.rx, pixel.ry) {
        return Some(COLOR_SHROUD);
    }
    if fog.is_cell_gap_fog(local_owner, pixel.rx, pixel.ry) {
        return Some(super::native_radar_terrain::half_bright_rgb565(
            pixel.packed_rgb565,
        ));
    }
    fog.is_cell_revealed(local_owner, pixel.rx, pixel.ry)
        .then_some(pixel.color)
}
