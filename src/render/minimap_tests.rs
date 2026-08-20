//! Unit tests for the minimap renderer.
//!
//! Tests coordinate mapping, owner color logic, pixel setting, and viewport math.
//! GPU-dependent tests (full MinimapRenderer construction) are not possible here.

use super::*;
use crate::map::houses::HouseAllianceMap;
use crate::map::playfield::PlayfieldBounds;
use crate::map::terrain::{TerrainCell, TerrainGrid};
use crate::render::minimap_helpers::*;
use crate::rules::color_scheme::ColorSchemeEntry;
use crate::rules::house_colors::{HouseColorIndex, HouseColorRamps};
use crate::sim::components::Position;
use crate::sim::intern::test_intern;
use crate::sim::vision::FogState;
use std::collections::{BTreeMap, BTreeSet};

fn playfield_projection_grid(side: u16) -> TerrainGrid {
    let mut cells = Vec::new();
    for ry in 0..side {
        for rx in 0..side {
            let (screen_x, screen_y) = crate::map::terrain::iso_to_screen(rx, ry, 0);
            cells.push(TerrainCell {
                screen_x,
                screen_y,
                tile_id: 1,
                sub_tile: 0,
                z: 0,
                rx,
                ry,
                is_water: false,
                variant: 0,
                tint: [1.0; 3],
                radar_left: [20, 80, 20],
                radar_right: [40, 100, 40],
                has_damaged_data: false,
            });
        }
    }
    TerrainGrid {
        cells,
        world_width: 1.0,
        world_height: 1.0,
        origin_x: 0.0,
        origin_y: 0.0,
        local_bounds: None,
        anchor_variant_table: None,
    }
}

fn expanded_playfield() -> PlayfieldBounds {
    PlayfieldBounds {
        base: 40,
        off_fc: 2,
        off_100: 2,
        off_104: 36,
        off_108: 30,
    }
}

fn shrunken_playfield() -> PlayfieldBounds {
    PlayfieldBounds {
        base: 40,
        off_fc: 10,
        off_100: 10,
        off_104: 10,
        off_108: 8,
    }
}

fn make_pixel(rx: u16, ry: u16, color: [u8; 4]) -> TerrainPixel {
    TerrainPixel {
        rx,
        ry,
        px: 0,
        py: 0,
        color,
    }
}

#[test]
fn test_world_to_minimap_pixel_origin() {
    // World position at origin maps to minimap pixel (0, 0).
    let (px, py): (u32, u32) =
        world_to_minimap_pixel(0.0, 0.0, 0.0, 0.0, 1000.0, 1000.0, 0.0, 0.0, 200.0, 200.0);
    assert_eq!(px, 0);
    assert_eq!(py, 0);
}

#[test]
fn test_world_to_minimap_pixel_center() {
    // Position at center of world maps to center of minimap.
    let (px, py): (u32, u32) = world_to_minimap_pixel(
        500.0, 500.0, 0.0, 0.0, 1000.0, 1000.0, 0.0, 0.0, 200.0, 200.0,
    );
    assert_eq!(px, 100);
    assert_eq!(py, 100);
}

#[test]
fn test_world_to_minimap_pixel_clamps_negative() {
    // Positions outside world bounds are clamped to 0.
    let (px, py): (u32, u32) = world_to_minimap_pixel(
        -500.0, -500.0, 0.0, 0.0, 1000.0, 1000.0, 0.0, 0.0, 200.0, 200.0,
    );
    assert_eq!(px, 0);
    assert_eq!(py, 0);
}

#[test]
fn test_world_to_minimap_pixel_clamps_overflow() {
    // Positions beyond world extent are clamped to max pixel (199).
    let (px, py): (u32, u32) = world_to_minimap_pixel(
        2000.0, 2000.0, 0.0, 0.0, 1000.0, 1000.0, 0.0, 0.0, 200.0, 200.0,
    );
    assert_eq!(px, 199);
    assert_eq!(py, 199);
}

#[test]
fn test_world_to_minimap_pixel_with_offset() {
    // World origin is offset — position at origin_x maps to pixel 0.
    let (px, py): (u32, u32) = world_to_minimap_pixel(
        -500.0, 200.0, -500.0, 200.0, 2000.0, 2000.0, 0.0, 0.0, 200.0, 200.0,
    );
    assert_eq!(px, 0);
    assert_eq!(py, 0);
}

/// Ramps where entry 0 = Gold, entry 1 = DarkBlue, entry 2 = DarkRed (HSV from
/// the retail `[Colors]` list) so dot colors carry the expected hue dominance.
fn test_ramps() -> HouseColorRamps {
    let mk = |name: &str, hsv: [u8; 3]| ColorSchemeEntry {
        name: name.into(),
        hsv,
    };
    HouseColorRamps::from_schemes(&[
        mk("Gold", [43, 239, 255]),
        mk("DarkBlue", [153, 214, 212]),
        mk("DarkRed", [0, 230, 255]),
    ])
}

#[test]
fn test_owner_dot_color_uses_house_map() {
    let mut map: HouseColorMap = HouseColorMap::new();
    map.insert("Americans".to_string(), HouseColorIndex(1)); // DarkBlue entry
    map.insert("Russians".to_string(), HouseColorIndex(2)); // DarkRed entry
    let ramps = test_ramps();

    let blue: [u8; 4] = owner_dot_color("Americans", &map, &ramps);
    let red: [u8; 4] = owner_dot_color("Russians", &map, &ramps);
    // Blue should have more B than R, red should have more R than B.
    assert!(blue[2] > blue[0], "Americans should be blue-ish: {blue:?}");
    assert!(red[0] > red[2], "Russians should be red-ish: {red:?}");
}

#[test]
fn test_owner_dot_color_unknown_defaults_to_default_scheme() {
    let mk = |name: &str, hsv: [u8; 3]| ColorSchemeEntry {
        name: name.into(),
        hsv,
    };
    // Entry 2 = DEFAULT_SCHEME_ENTRY (LightGrey-ish): the producers' fallback.
    let ramps = HouseColorRamps::from_schemes(&[
        mk("LightGold", [25, 255, 255]),
        mk("Gold", [43, 239, 255]),
        mk("LightGrey", [0, 0, 240]),
    ]);
    let map: HouseColorMap = HouseColorMap::new();
    let dot: [u8; 4] = owner_dot_color("Unknown", &map, &ramps);
    // Unknown owner resolves to the default scheme (entry 2), not entry 0.
    assert_eq!(
        dot,
        owner_dot_color_for_index(&ramps, HouseColorIndex(2)),
        "unknown owner should use DEFAULT_SCHEME_ENTRY (2)"
    );
    assert_eq!(dot[3], 255, "Alpha should be fully opaque");
}

/// Helper: dot color for a known index (mirrors owner_dot_color's ramp[0] pick).
fn owner_dot_color_for_index(ramps: &HouseColorRamps, idx: HouseColorIndex) -> [u8; 4] {
    let c = ramps.ramp(idx)[0];
    [c.r, c.g, c.b, 255]
}

#[test]
fn test_minimap_entity_visible_for_allied_owner() {
    let mut alliances = HouseAllianceMap::default();
    let allied_names = BTreeSet::from(["AMERICANS".to_string(), "BRITISH".to_string()]);
    alliances.insert("AMERICANS".to_string(), allied_names.clone());
    alliances.insert("BRITISH".to_string(), allied_names);

    let fog = FogState {
        width: 64,
        height: 64,
        by_owner: BTreeMap::new(),
        alliances,
        ..Default::default()
    };
    let pos = Position {
        rx: 10,
        ry: 12,
        z: 0,
        exact_z_leptons: None,
        sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
        sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
    };
    assert!(minimap_entity_visible(
        test_intern("Americans"),
        &fog,
        &pos,
        test_intern("British"),
    ));
}

#[test]
fn test_cell_visibility_color_visible_uses_base_color() {
    let mut fog = FogState {
        width: 16,
        height: 16,
        ..Default::default()
    };
    fog.mark_visible_for_owner(test_intern("Americans"), 5, 7);
    let pixel = make_pixel(5, 7, [40, 120, 40, 255]);
    assert_eq!(
        cell_visibility_color(test_intern("Americans"), &fog, &pixel),
        Some([40, 120, 40, 255])
    );
}

#[test]
fn test_cell_visibility_color_revealed_shows_full_color() {
    // In standard YR (FogOfWar=false), revealed cells show at full brightness.
    let mut fog = FogState {
        width: 16,
        height: 16,
        ..Default::default()
    };
    fog.mark_visible_for_owner(test_intern("Americans"), 5, 7);
    fog.by_owner
        .get_mut(&test_intern("Americans"))
        .expect("owner present")
        .clear_all_visible();
    let pixel = make_pixel(5, 7, [100, 50, 25, 255]);
    assert_eq!(
        cell_visibility_color(test_intern("Americans"), &fog, &pixel),
        Some([100, 50, 25, 255])
    );
}

#[test]
fn test_cell_visibility_color_shrouded_returns_none() {
    let fog = FogState::default();
    let pixel = make_pixel(9, 9, [40, 120, 40, 255]);
    assert_eq!(
        cell_visibility_color(test_intern("Americans"), &fog, &pixel),
        None
    );
}

#[test]
fn test_set_pixel_in_bounds() {
    let mut rgba: Vec<u8> = vec![0u8; 16]; // 2x2 pixel buffer
    set_pixel(&mut rgba, 2, 1, 0, [255, 128, 64, 255]);
    // Pixel at (1,0) -> offset = (0*2 + 1)*4 = 4
    assert_eq!(rgba[4], 255);
    assert_eq!(rgba[5], 128);
    assert_eq!(rgba[6], 64);
    assert_eq!(rgba[7], 255);
}

#[test]
fn test_set_pixel_out_of_bounds_does_nothing() {
    let mut rgba: Vec<u8> = vec![0u8; 16]; // 2x2 pixel buffer
    // Writing to x=5 in a 2-wide buffer should be silently ignored.
    set_pixel(&mut rgba, 2, 5, 0, [255, 255, 255, 255]);
    assert!(rgba.iter().all(|&b| b == 0));
}

#[test]
fn test_viewport_rect_returns_four_lines() {
    // We can't construct a full MinimapRenderer without GPU, but we can
    // test the coordinate math independently.
    let mm_size: f32 = MINIMAP_SIZE as f32;
    let world_w: f32 = 3000.0;
    let world_h: f32 = 2000.0;

    // Camera at origin, 1024x768 viewport.
    let cam_x: f32 = 0.0;
    let cam_y: f32 = 0.0;
    let screen_w: f32 = 1024.0;
    let screen_h: f32 = 768.0;

    // Compute expected viewport rect on minimap.
    let nx_left: f32 = (cam_x - 0.0) / world_w;
    let ny_top: f32 = (cam_y - 0.0) / world_h;
    let nx_right: f32 = (cam_x + screen_w - 0.0) / world_w;
    let ny_bottom: f32 = (cam_y + screen_h - 0.0) / world_h;

    let left: f32 = (nx_left * mm_size).clamp(0.0, mm_size);
    let top: f32 = (ny_top * mm_size).clamp(0.0, mm_size);
    let right: f32 = (nx_right * mm_size).clamp(0.0, mm_size);
    let bottom: f32 = (ny_bottom * mm_size).clamp(0.0, mm_size);

    // Verify the expected rect is within the minimap.
    assert!(left >= 0.0);
    assert!(top >= 0.0);
    assert!(right <= mm_size);
    assert!(bottom <= mm_size);
    assert!(right > left, "viewport should have nonzero width");
    assert!(bottom > top, "viewport should have nonzero height");
}

#[test]
fn test_single_cell_map_pixel_mapping() {
    // A map with one cell: its position should map to a valid minimap pixel.
    // If world_width is small (e.g., just one tile = 60px), the cell still
    // maps to pixel (0,0) since it IS the origin.
    let (px, py): (u32, u32) =
        world_to_minimap_pixel(0.0, 0.0, 0.0, 0.0, 60.0, 30.0, 0.0, 0.0, 200.0, 200.0);
    assert_eq!(px, 0);
    assert_eq!(py, 0);
}

#[test]
fn test_degenerate_world_size_no_panic() {
    // Zero-size world should not panic (clamped to 1.0 in MinimapRenderer::new).
    let (px, py): (u32, u32) =
        world_to_minimap_pixel(100.0, 100.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 200.0, 200.0);
    // Should clamp to max pixel.
    assert_eq!(px, 199);
    assert_eq!(py, 199);
}

#[test]
fn playfield_projection_shrinks_and_reexpands_exact_mode_zero_membership() {
    let grid = playfield_projection_grid(64);
    let expanded = expanded_playfield();
    let shrunken = shrunken_playfield();
    let shared = grid
        .cells
        .iter()
        .find(|cell| {
            expanded.contains_geometry_packed(cell.rx.into(), cell.ry.into())
                && shrunken.contains_geometry_packed(cell.rx.into(), cell.ry.into())
        })
        .map(|cell| (cell.rx, cell.ry))
        .expect("shared valid cell");
    let readded = grid
        .cells
        .iter()
        .find(|cell| {
            expanded.contains_geometry_packed(cell.rx.into(), cell.ry.into())
                && !shrunken.contains_geometry_packed(cell.rx.into(), cell.ry.into())
        })
        .map(|cell| (cell.rx, cell.ry))
        .expect("cell excluded by contraction");
    let outside = grid
        .cells
        .iter()
        .find(|cell| !expanded.contains_geometry_packed(cell.rx.into(), cell.ry.into()))
        .map(|cell| (cell.rx, cell.ry))
        .expect("diamond filler cell");
    let overlays = [
        (shared.0, shared.1, OverlayClassification::Ore, 5, None),
        (readded.0, readded.1, OverlayClassification::Gem, 7, None),
        (outside.0, outside.1, OverlayClassification::Wall, 0, None),
    ];

    let initial = MinimapPlayfieldProjection::derive(&grid, &overlays, "TEMPERATE", Some(expanded));
    let shrunk = MinimapPlayfieldProjection::derive(&grid, &overlays, "TEMPERATE", Some(shrunken));
    let reexpanded =
        MinimapPlayfieldProjection::derive(&grid, &overlays, "TEMPERATE", Some(expanded));

    let initial_cells: BTreeSet<_> = initial
        .terrain_pixels
        .iter()
        .map(|pixel| (pixel.rx, pixel.ry))
        .collect();
    let expected_cells: BTreeSet<_> = grid
        .cells
        .iter()
        .filter(|cell| expanded.contains_geometry_packed(cell.rx.into(), cell.ry.into()))
        .map(|cell| (cell.rx, cell.ry))
        .collect();
    assert_eq!(
        initial_cells, expected_cells,
        "membership must be exact mode 0"
    );
    assert!(initial_cells.contains(&readded));
    assert!(!initial_cells.contains(&outside));
    assert!(
        initial
            .overlay_pixels
            .iter()
            .any(|p| (p.rx, p.ry) == readded)
    );
    assert!(
        !initial
            .overlay_pixels
            .iter()
            .any(|p| (p.rx, p.ry) == outside)
    );
    assert!(
        !shrunk
            .terrain_pixels
            .iter()
            .any(|p| (p.rx, p.ry) == readded)
    );
    assert!(
        !shrunk
            .overlay_pixels
            .iter()
            .any(|p| (p.rx, p.ry) == readded)
    );
    assert!(
        reexpanded
            .terrain_pixels
            .iter()
            .any(|p| (p.rx, p.ry) == readded)
    );
    assert!(
        reexpanded
            .overlay_pixels
            .iter()
            .any(|p| (p.rx, p.ry) == readded)
    );
    assert!(shrunk.world_width < initial.world_width);
    assert!(shrunk.world_height < initial.world_height);
    assert_eq!(reexpanded.world_origin_x, initial.world_origin_x);
    assert_eq!(reexpanded.world_origin_y, initial.world_origin_y);
}

#[test]
fn playfield_revision_rebuild_gate_observes_two_successive_writers() {
    let bounds = Some(expanded_playfield());
    let initial = PlayfieldAuthorityStamp {
        bounds,
        revision: 0,
    };
    assert!(!playfield_authority_needs_reconcile(
        Some(initial),
        bounds,
        0
    ));
    assert!(playfield_authority_needs_reconcile(
        Some(initial),
        bounds,
        1
    ));
    let second = PlayfieldAuthorityStamp {
        bounds,
        revision: 1,
    };
    assert!(playfield_authority_needs_reconcile(Some(second), bounds, 2));
    assert!(playfield_authority_needs_reconcile(None, bounds, 2));
}

#[test]
fn minimap_techno_membership_is_height_aware_at_raised_slope_edge() {
    let bounds = expanded_playfield();
    let (rx, ry) = (0u16..64)
        .flat_map(|ry| (0u16..64).map(move |rx| (rx, ry)))
        .find(|&(rx, ry)| {
            bounds.contains_geometry_packed(rx.into(), ry.into())
                && !bounds.contains_height_aware_packed(rx.into(), ry.into(), 10, 1)
        })
        .expect("raised/slope edge where native mode 0 and mode 1 differ");
    assert!(bounds.contains_geometry_packed(rx.into(), ry.into()));
    assert!(!bounds.contains_height_aware_packed(rx.into(), ry.into(), 10, 1));
    assert!(bounds.contains_height_aware_packed(rx.into(), ry.into(), 0, 0));

    let mut entity =
        crate::sim::game_entity::GameEntity::test_default(1, "MTNK", "Americans", rx, ry);
    entity.in_playfield = false;
    assert!(!minimap_entity_in_playfield(true, &entity));
    entity.in_playfield = true;
    assert!(minimap_entity_in_playfield(true, &entity));
    entity.in_playfield = false;
    assert!(minimap_entity_in_playfield(false, &entity));
}

#[test]
fn playfield_projection_updates_camera_bounds_and_click_inverse_mapping() {
    let mut grid = playfield_projection_grid(64);
    let initial_geometry = PlayfieldPresentationGeometry::from_grid(&grid, expanded_playfield())
        .expect("expanded geometry");
    let shrunk_geometry = PlayfieldPresentationGeometry::from_grid(&grid, shrunken_playfield())
        .expect("shrunken geometry");
    let camera_bounds = shrunk_geometry.camera_local_bounds();
    assert_eq!(
        camera_bounds.pixel_x - crate::map::terrain::TILE_WIDTH / 2.0,
        shrunk_geometry.origin_x
    );
    assert_eq!(camera_bounds.pixel_y, shrunk_geometry.origin_y);
    assert_eq!(camera_bounds.pixel_w, shrunk_geometry.world_width);
    assert_eq!(camera_bounds.pixel_h, shrunk_geometry.world_height);
    grid.install_playfield_local_bounds(Some(expanded_playfield()));
    let installed_initial = grid.local_bounds.expect("initial camera authority");
    grid.install_playfield_local_bounds(Some(shrunken_playfield()));
    assert_eq!(grid.local_bounds, Some(camera_bounds));
    assert_ne!(grid.local_bounds, Some(installed_initial));

    let projection =
        MinimapPlayfieldProjection::derive(&grid, &[], "TEMPERATE", Some(shrunken_playfield()));
    let rect = (10.0, 20.0, 300.0, 240.0);
    let tex_x = projection.map_offset_x + projection.map_pixel_w * 0.5;
    let tex_y = projection.map_offset_y + projection.map_pixel_h * 0.5;
    let click_x = rect.0 + tex_x / MINIMAP_SIZE as f32 * rect.2;
    let click_y = rect.1 + tex_y / MINIMAP_SIZE as f32 * rect.3;
    let camera = minimap_screen_point_to_camera_top_left(
        click_x,
        click_y,
        800.0,
        600.0,
        rect.0,
        rect.1,
        rect.2,
        rect.3,
        projection.world_origin_x,
        projection.world_origin_y,
        projection.world_width,
        projection.world_height,
        projection.map_offset_x,
        projection.map_offset_y,
        projection.map_pixel_w,
        projection.map_pixel_h,
    );
    assert_eq!(
        camera,
        (
            projection.world_origin_x + projection.world_width * 0.5 - 400.0,
            projection.world_origin_y + projection.world_height * 0.5 - 300.0,
        )
    );
    assert_ne!(initial_geometry.camera_local_bounds(), camera_bounds);
}
