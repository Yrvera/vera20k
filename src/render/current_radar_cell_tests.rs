use super::*;

use crate::map::bridge_facts::{
    Axis, BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts, BridgeheadAnchorClass,
};
use crate::map::playfield::PlayfieldBounds;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::map::terrain::{TerrainGrid, build_terrain_grid_from_resolved};
use crate::render::minimap::{MinimapCellRadarSource, MinimapOverlayDatum};
use crate::render::minimap_helpers::OverlayClassification;
use crate::render::minimap_projection::MinimapPlayfieldProjection;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::bridge_state::{
    BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState, DamageState,
};
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::runtime::SimRuntime;
use crate::sim::world::Simulation;

const SIDE: u16 = 64;
const BRIDGE_COLOR: [u8; 3] = [5, 6, 7];
const OVERLAY_COLOR: [u8; 3] = [11, 12, 13];
const BASE_COLOR: [u8; 3] = [10, 20, 30];

fn expanded_bounds() -> PlayfieldBounds {
    PlayfieldBounds {
        base: 40,
        off_fc: 2,
        off_100: 2,
        off_104: 36,
        off_108: 30,
    }
}

fn shrunken_bounds() -> PlayfieldBounds {
    PlayfieldBounds {
        base: 40,
        off_fc: 10,
        off_100: 10,
        off_104: 10,
        off_108: 8,
    }
}

fn fixture(static_bridge: Option<(u16, u16)>) -> (TerrainGrid, ResolvedTerrainGrid) {
    let cells = (0..SIDE)
        .flat_map(|ry| {
            (0..SIDE).map(move |rx| {
                let mut cell = flat_cell(rx, ry);
                if static_bridge == Some((rx, ry)) {
                    cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL;
                    cell.has_bridge_deck = true;
                }
                cell
            })
        })
        .collect();
    let resolved = ResolvedTerrainGrid::from_cells(SIDE, SIDE, cells);
    let grid = build_terrain_grid_from_resolved(&resolved, None, None);
    (grid, resolved)
}

fn central_cell(grid: &TerrainGrid, bounds: PlayfieldBounds) -> (u16, u16) {
    grid.cells
        .iter()
        .find(|cell| {
            bounds.contains_geometry_packed(i32::from(cell.rx), i32::from(cell.ry))
                && cell.rx > 8
                && cell.ry > 8
                && cell.rx < SIDE - 8
                && cell.ry < SIDE - 8
        })
        .map(|cell| (cell.rx, cell.ry))
        .expect("interior playfield cell")
}

fn live_runtime(
    resolved: ResolvedTerrainGrid,
    bridge_state: BridgeRuntimeState,
    overlay_grid: OverlayGrid,
) -> SimRuntime {
    let mut sim = Simulation::new();
    sim.resolved_terrain = Some(resolved);
    sim.bridge_state = Some(bridge_state);
    sim.overlay_grid = Some(overlay_grid);
    SimRuntime::from_simulation(sim)
}

fn bridge_state_at(cell: (u16, u16), intact: bool) -> BridgeRuntimeState {
    let mut state = BridgeRuntimeState::default();
    state.test_seed_cell(
        cell.0,
        cell.1,
        BridgeRuntimeCell {
            deck_present: intact,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: if intact {
                DamageState::Healthy { variant: 0 }
            } else {
                DamageState::Destroyed
            },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Body,
            anchor_span_id: Some(1),
            overlay_byte: if intact { 0xCD } else { 0xE7 },
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        },
    );
    state
}

fn presentation_overlay(cell: (u16, u16), overlay_id: u8, frame: u8) -> MinimapOverlayDatum {
    MinimapOverlayDatum {
        rx: cell.0,
        ry: cell.1,
        classification: OverlayClassification::Wall,
        source: MinimapCellRadarSource::Overlay {
            overlay_id,
            frame,
            is_tiberium: false,
            has_cell_anim: false,
            has_tiberium_type: false,
        },
    }
}

fn colors() -> HashMap<(u8, u8), [u8; 3]> {
    HashMap::from([((24, 0), BRIDGE_COLOR), ((10, 4), OVERLAY_COLOR)])
}

fn projection<'a>(
    grid: &TerrainGrid,
    runtime: &'a SimRuntime,
    presentation: &[MinimapOverlayDatum],
    bounds: PlayfieldBounds,
    colors: &HashMap<(u8, u8), [u8; 3]>,
) -> MinimapPlayfieldProjection {
    MinimapPlayfieldProjection::derive(
        grid,
        runtime.simulation.resolved_terrain.as_ref(),
        presentation,
        colors,
        "TEMPERATE",
        Some(bounds),
        Some(CurrentRadarCellAuthority::from_runtime(runtime)),
    )
}

fn raw_pair(
    projection: &MinimapPlayfieldProjection,
    cell: (u16, u16),
) -> [[u8; 3]; 2] {
    let geometry = projection.native_radar_surface.expect("native radar surface");
    let raw = projection
        .native_radar_terrain
        .as_ref()
        .expect("native radar terrain")
        .raw_rgb();
    let (x, y) = geometry.cell_to_raw_pixel(cell);
    assert!(x >= 0 && x + 1 < geometry.raw_size().0);
    assert!(y >= 0 && y < geometry.raw_size().1);
    let index = (y * geometry.raw_size().0 + x) as usize;
    [raw[index], raw[index + 1]]
}

#[test]
fn gsi_04_01_load_intact_bridge_discards_abandoned_destroyed_pixels() {
    let (grid, mut resolved) = fixture(None);
    let cell = central_cell(&grid, expanded_bounds());
    resolved.cell_mut(cell.0, cell.1).unwrap().bridge_facts.raw_flags =
        BRIDGE_FLAG_STRUCTURAL;
    let overlay = OverlayGrid::new(SIDE, SIDE);
    let abandoned = live_runtime(
        resolved.clone(),
        bridge_state_at(cell, false),
        overlay.clone(),
    );
    let restored = live_runtime(resolved, bridge_state_at(cell, true), overlay);
    assert!(restored.simulation.radar_terrain_dirty_cells.is_empty());

    let colors = colors();
    let stale_destroyed = [presentation_overlay(cell, 239, 0)];
    assert_eq!(
        raw_pair(
            &projection(&grid, &abandoned, &stale_destroyed, expanded_bounds(), &colors),
            cell,
        ),
        [BASE_COLOR; 2],
        "abandoned destroyed timeline used its current fallen bridge state",
    );
    assert_eq!(
        raw_pair(
            &projection(&grid, &restored, &stale_destroyed, expanded_bounds(), &colors),
            cell,
        ),
        [BRIDGE_COLOR; 2],
        "first restored primary surface comes from saved intact bridge state",
    );
}

#[test]
fn gsi_04_01_load_destroyed_bridge_discards_abandoned_repair_pixels() {
    let (grid, mut resolved) = fixture(None);
    let cell = central_cell(&grid, expanded_bounds());
    resolved.cell_mut(cell.0, cell.1).unwrap().bridge_facts.raw_flags =
        BRIDGE_FLAG_STRUCTURAL;
    let overlay = OverlayGrid::new(SIDE, SIDE);
    let abandoned = live_runtime(
        resolved.clone(),
        bridge_state_at(cell, true),
        overlay.clone(),
    );
    let restored = live_runtime(resolved, bridge_state_at(cell, false), overlay);
    assert!(restored.simulation.radar_terrain_dirty_cells.is_empty());

    let colors = colors();
    let stale_repaired = [presentation_overlay(cell, 0xCD, 1)];
    assert_eq!(
        raw_pair(
            &projection(&grid, &abandoned, &stale_repaired, expanded_bounds(), &colors),
            cell,
        ),
        [BRIDGE_COLOR; 2],
    );
    let restored_projection =
        projection(&grid, &restored, &stale_repaired, expanded_bounds(), &colors);
    assert_eq!(raw_pair(&restored_projection, cell), [BASE_COLOR; 2]);
    assert!(
        restored_projection
            .overlay_pixels
            .iter()
            .all(|pixel| (pixel.rx, pixel.ry) != cell),
        "static bridge facts and abandoned repaired overlay cannot revive the saved collapse",
    );
}

#[test]
fn gsi_04_01_load_live_overlay_absence_beats_stale_presentation_tombstone() {
    let (grid, resolved) = fixture(None);
    let cell = central_cell(&grid, expanded_bounds());
    let mut prior_overlay = OverlayGrid::new(SIDE, SIDE);
    prior_overlay.place_overlay(cell.0, cell.1, 10, 4);
    let prior = live_runtime(
        resolved.clone(),
        BridgeRuntimeState::default(),
        prior_overlay,
    );
    let restored = live_runtime(
        resolved,
        BridgeRuntimeState::default(),
        OverlayGrid::new(SIDE, SIDE),
    );
    let stale = [presentation_overlay(cell, 10, 4)];
    let colors = colors();

    assert_eq!(
        raw_pair(
            &projection(&grid, &prior, &stale, expanded_bounds(), &colors),
            cell,
        ),
        [OVERLAY_COLOR; 2],
    );
    let restored_projection = projection(&grid, &restored, &stale, expanded_bounds(), &colors);
    assert_eq!(raw_pair(&restored_projection, cell), [BASE_COLOR; 2]);
    assert!(
        restored_projection
            .overlay_pixels
            .iter()
            .all(|pixel| (pixel.rx, pixel.ry) != cell),
        "full rebuild enumerates restored OverlayGrid, not retained presentation entries",
    );
}

#[test]
fn gsi_04_01_load_restored_local_size_builds_geometry_before_live_cell_pixels() {
    let (grid, resolved) = fixture(None);
    let cell = central_cell(&grid, shrunken_bounds());
    let mut overlay = OverlayGrid::new(SIDE, SIDE);
    overlay.place_overlay(cell.0, cell.1, 10, 4);
    let runtime = live_runtime(resolved, BridgeRuntimeState::default(), overlay);
    let colors = colors();

    let abandoned = projection(&grid, &runtime, &[], expanded_bounds(), &colors);
    let restored = projection(&grid, &runtime, &[], shrunken_bounds(), &colors);
    let abandoned_geometry = abandoned.native_radar_surface.expect("old geometry");
    let restored_geometry = restored.native_radar_surface.expect("restored geometry");
    assert_ne!(abandoned_geometry.raw_size(), restored_geometry.raw_size());
    assert_ne!(
        abandoned_geometry.cell_to_raw_pixel(cell),
        restored_geometry.cell_to_raw_pixel(cell),
    );
    assert_eq!(raw_pair(&restored, cell), [OVERLAY_COLOR; 2]);
    assert!(
        restored
            .overlay_pixels
            .iter()
            .any(|pixel| (pixel.rx, pixel.ry) == cell),
        "restored current source is projected through restored LocalSize",
    );
}

#[test]
fn gsi_04_01_full_and_incremental_paths_share_current_cell_source_precedence() {
    let (grid, mut resolved) = fixture(None);
    let cell = central_cell(&grid, expanded_bounds());
    resolved
        .cell_mut(cell.0, cell.1)
        .unwrap()
        .terrain_object_occupation = Some(9);
    let mut overlay = OverlayGrid::new(SIDE, SIDE);
    overlay.place_overlay(cell.0, cell.1, 10, 4);
    let runtime = live_runtime(resolved, bridge_state_at(cell, true), overlay);
    let authority = CurrentRadarCellAuthority::from_runtime(&runtime);
    let colors = colors();
    assert_eq!(
        authority.source(cell.0, cell.1, BRIDGE_COLOR, &colors),
        Some(([200, 200, 160], OverlayClassification::TerrainObject)),
    );
    assert_eq!(
        raw_pair(
            &projection(&grid, &runtime, &[], expanded_bounds(), &colors),
            cell,
        ),
        [[200, 200, 160]; 2],
    );
}

fn flat_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
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
        radar_left: [20, 40, 60],
        radar_right: [90, 90, 90],
        has_damaged_data: false,
        bridgehead_anchor_class_at_load: None,
    }
}
