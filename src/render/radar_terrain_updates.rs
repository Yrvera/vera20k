//! Current-cell updates for retained native radar terrain layers.
//!
//! This is the GPU-free body used by `MinimapRenderer` after the simulation's
//! terrain-dirty generation changes. Full reconstruction and these updates
//! both consume `CurrentRadarCellAuthority`.

use std::collections::HashMap;

use super::current_radar_cell::CurrentRadarCellAuthority;
use super::minimap_helpers::{
    MINIMAP_WIDTH, OverlayPixel, RadarSurfacePixel, TerrainPixel, set_pixel,
};
use super::minimap_projection::{aperture_pixel, rasterize_native_terrain};
use super::native_radar_surface::NativeRadarSurfaceGeometry;
use super::native_radar_terrain::NativeRadarTerrainSurface;

pub(super) struct RadarTerrainUpdateLayers<'a> {
    pub base_rgba: &'a mut Vec<u8>,
    pub terrain_pixels: &'a [TerrainPixel],
    pub surface_pixels: &'a mut Vec<RadarSurfacePixel>,
    pub overlay_pixels: &'a mut Vec<OverlayPixel>,
    pub native_surface: Option<NativeRadarSurfaceGeometry>,
    pub native_terrain: &'a mut Option<NativeRadarTerrainSurface>,
}

/// Re-run the current CellClass radar source for one acknowledged dirty batch.
///
/// gamemd-derived: `CellClass::GetRadarColor @ 0x0047C060` is called by the
/// terrain-dirty consumer `RadarClass::ClearBackground @ 0x00655250`; its source
/// priority is TerrainClass, current structural-high flag, current overlay,
/// then selected TMP.
pub(super) fn apply_radar_terrain_dirty_cells(
    layers: RadarTerrainUpdateLayers<'_>,
    authority: CurrentRadarCellAuthority<'_>,
    structural_bridge_color: [u8; 3],
    overlay_radar_colors: &HashMap<(u8, u8), [u8; 3]>,
    cells: &[(u16, u16)],
) {
    let mut native_changes = Vec::new();
    for &(rx, ry) in cells {
        let Some(terrain_pixel) = layers
            .terrain_pixels
            .iter()
            .find(|pixel| pixel.rx == rx && pixel.ry == ry)
            .copied()
        else {
            continue;
        };
        if let Some((x, y)) = aperture_pixel(
            layers.native_surface,
            (terrain_pixel.px, terrain_pixel.py),
        ) {
            set_pixel(
                layers.base_rgba,
                MINIMAP_WIDTH,
                x,
                y,
                terrain_pixel.color,
            );
        }

        let source = authority.source(
            rx,
            ry,
            structural_bridge_color,
            overlay_radar_colors,
        );
        if let Some((native_color, classification)) = source {
            let color = [native_color[0], native_color[1], native_color[2], 255];
            native_changes.push(((rx, ry), Some(native_color)));
            if let Some(existing) = layers
                .overlay_pixels
                .iter_mut()
                .find(|pixel| pixel.rx == rx && pixel.ry == ry)
            {
                existing.px = terrain_pixel.px;
                existing.py = terrain_pixel.py;
                existing.color = color;
                existing.classification = classification;
            } else {
                layers.overlay_pixels.push(OverlayPixel {
                    rx,
                    ry,
                    px: terrain_pixel.px,
                    py: terrain_pixel.py,
                    color,
                    classification,
                });
            }
        } else {
            native_changes.push(((rx, ry), None));
            layers
                .overlay_pixels
                .retain(|pixel| !(pixel.rx == rx && pixel.ry == ry));
        }
    }
    if let Some(surface) = layers.native_terrain
        && surface.set_cell_overrides(native_changes)
    {
        let (rgba, pixels) = rasterize_native_terrain(surface);
        *layers.base_rgba = rgba;
        *layers.surface_pixels = pixels;
    }
}
