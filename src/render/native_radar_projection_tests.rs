use super::*;
use crate::map::bridge_facts::BridgeCellFacts;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::map::terrain::{TerrainGrid, build_terrain_grid_from_resolved};
use crate::render::minimap_helpers::OverlayClassification;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::util::native_x87::{NativeF32Bits, NativeF64Bits, X87Chop53, X87Value};
use std::collections::{BTreeMap, HashMap};

const SIZE_WIDTH: u16 = 80;
const SIZE_HEIGHT: u16 = 60;
const GRID_SIDE: u16 = 201;

fn wide_bounds() -> PlayfieldBounds {
    PlayfieldBounds::from_normalized_local_size(80, 2, 2, 73, 54)
}

fn shrunken_bounds() -> PlayfieldBounds {
    PlayfieldBounds::from_normalized_local_size(80, 10, 10, 50, 40)
}

fn fixture() -> (TerrainGrid, ResolvedTerrainGrid) {
    let special = BTreeMap::from([
        // raw_x=-1: native clipping retains only this cell's right half.
        ((12, 88), ([20, 30, 40], [100, 120, 140])),
        // raw_x=raw_w-1: native clipping retains only the left half.
        ((85, 15), ([160, 180, 200], [30, 40, 50])),
        // raw_y=109 is the final source row. Active CW 0x0E7F truncates its
        // generated center to y=104; the row still must reach the sampler.
        ((97, 97), ([200, 20, 20], [20, 200, 20])),
    ]);
    let cells = (0..GRID_SIDE)
        .flat_map(|ry| {
            let special = &special;
            (0..GRID_SIDE).map(move |rx| {
                let (left, right) = special
                    .get(&(rx, ry))
                    .copied()
                    .unwrap_or(([2, 2, 2], [2, 2, 2]));
                flat_cell(rx, ry, left, right)
            })
        })
        .collect();
    let allocated = (0..GRID_SIDE)
        .flat_map(|ry| {
            (0..GRID_SIDE)
                .filter_map(move |rx| native_size_diamond_cell(rx, ry).then_some((rx, ry)))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        allocated.len(),
        SIZE_HEIGHT as usize * (2 * SIZE_WIDTH as usize - 1)
    );
    let mut resolved = ResolvedTerrainGrid::from_cells(GRID_SIDE, GRID_SIDE, cells);
    resolved.test_set_native_allocated_cells(&allocated);
    let grid = build_terrain_grid_from_resolved(&resolved, None, None);
    assert_eq!(grid.cells.len(), allocated.len());
    (grid, resolved)
}

fn native_size_diamond_cell(rx: u16, ry: u16) -> bool {
    let x = i32::from(rx);
    let y = i32::from(ry);
    let width = i32::from(SIZE_WIDTH);
    let height = i32::from(SIZE_HEIGHT);
    width < x + y && x - y < width && y - x < width && x + y <= width + 2 * height
}

#[test]
fn projection_raw_fill_keeps_clipped_edges_and_final_source_row() {
    let (grid, resolved) = fixture();
    let projection = MinimapPlayfieldProjection::derive(
        &grid,
        Some(&resolved),
        &[],
        &HashMap::new(),
        "TEMPERATE",
        Some(wide_bounds()),
    );
    let geometry = projection.native_radar_surface.expect("native surface");
    assert_eq!(geometry.raw_size(), (146, 110));
    assert_eq!(geometry.generated_size(), (140, 105));

    let surface = projection
        .native_radar_terrain
        .as_ref()
        .expect("native terrain");
    let raw = surface.raw_rgb();
    assert_eq!(raw[15 * 146], [10, 15, 20], "left clip keeps right");
    assert_eq!(raw[15 * 146 + 145], [80, 90, 100], "right clip keeps left");
    assert_eq!(geometry.cell_to_raw_pixel((97, 97)), (75, 109));
    assert_eq!(geometry.cell_to_surface_pixel((97, 97)).1, 104);
    assert_eq!(
        &raw[109 * 146 + 75..109 * 146 + 77],
        &[[100, 10, 10], [100, 10, 10]]
    );

    // `Math__ftol @ 0x007C5F00` runs under live CW 0x0E7F (truncate).
    // Therefore the critic's proposed 109 -> 105 boundary is excluded: in
    // this 146x110 fixture every cell whose raw footprint intersects the
    // source has an in-bounds generated center. Raw-fill admission remains
    // independent of that incidental fact, exactly like 0x00654EA0.
    for cell in &grid.cells {
        let origin = geometry.cell_to_raw_pixel((cell.rx, cell.ry));
        let raw_intersects = origin.1 >= 0
            && origin.1 < geometry.raw_size().1
            && origin.0 < geometry.raw_size().0
            && origin.0 + 1 >= 0;
        if raw_intersects {
            let center = geometry.cell_to_surface_pixel((cell.rx, cell.ry));
            assert!(
                center.0 >= 0
                    && center.0 < geometry.generated_size().0
                    && center.1 >= 0
                    && center.1 < geometry.generated_size().1,
                "raw-intersecting cell ({},{}) projected outside at {center:?}",
                cell.rx,
                cell.ry,
            );
        }
    }

    let expected_raw = reference_raw(&grid, geometry);
    assert_eq!(raw, expected_raw);
    assert_eq!(
        surface.generated_rgb565(),
        reference_generate(
            &expected_raw,
            geometry.raw_size(),
            geometry.generated_size()
        )
    );
}

#[test]
fn action40_projection_rebuild_regenerates_every_pixel_without_stale_raw_data() {
    let (grid, resolved) = fixture();
    let initial = MinimapPlayfieldProjection::derive(
        &grid,
        Some(&resolved),
        &[],
        &HashMap::new(),
        "TEMPERATE",
        Some(wide_bounds()),
    );
    let rebuilt = MinimapPlayfieldProjection::derive(
        &grid,
        Some(&resolved),
        &[],
        &HashMap::new(),
        "TEMPERATE",
        Some(shrunken_bounds()),
    );
    let initial_geometry = initial.native_radar_surface.expect("initial surface");
    let rebuilt_geometry = rebuilt.native_radar_surface.expect("rebuilt surface");
    assert_eq!(initial_geometry.raw_size(), (146, 110));
    assert_eq!(rebuilt_geometry.raw_size(), (100, 82));

    let initial = initial.native_radar_terrain.expect("initial terrain");
    let rebuilt = rebuilt.native_radar_terrain.expect("rebuilt terrain");
    assert_ne!(initial.raw_rgb(), rebuilt.raw_rgb());
    let expected = reference_raw(&grid, rebuilt_geometry);
    assert_eq!(rebuilt.raw_rgb(), expected);
    assert_eq!(
        rebuilt.generated_rgb565(),
        reference_generate(
            &expected,
            rebuilt_geometry.raw_size(),
            rebuilt_geometry.generated_size(),
        )
    );
}

#[test]
fn cell_get_radar_color_precedence_feeds_raw_surface_before_weighted_generation() {
    let (mut grid, mut resolved) = fixture();
    resolved
        .cell_mut(50, 50)
        .expect("terrain object cell")
        .terrain_object_occupation = Some(0);
    resolved
        .cell_mut(50, 51)
        .expect("structural bridge cell")
        .bridge_facts
        .raw_flags = crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
    resolved.test_set_radar_color_valid(51, 50, false);
    resolved.test_set_radar_color_valid(52, 50, true);
    let black = grid
        .cells
        .iter_mut()
        .find(|cell| (cell.rx, cell.ry) == (52, 50))
        .expect("valid black TMP cell");
    black.radar_left = [0; 3];
    black.radar_right = [99; 3];
    let overlays = [
        MinimapOverlayDatum {
            rx: 50,
            ry: 50,
            classification: OverlayClassification::Wall,
            source: MinimapCellRadarSource::Overlay {
                overlay_id: 10,
                frame: 4,
                is_tiberium: false,
                has_tiberium_type: false,
            },
        },
        MinimapOverlayDatum {
            rx: 50,
            ry: 51,
            classification: OverlayClassification::Wall,
            source: MinimapCellRadarSource::Overlay {
                overlay_id: 10,
                frame: 4,
                is_tiberium: false,
                has_tiberium_type: false,
            },
        },
        MinimapOverlayDatum {
            rx: 97,
            ry: 97,
            classification: OverlayClassification::Wall,
            source: MinimapCellRadarSource::Overlay {
                overlay_id: 10,
                frame: 4,
                is_tiberium: false,
                has_tiberium_type: false,
            },
        },
    ];
    let colors = HashMap::from([
        ((24, 0), [5, 6, 7]),
        ((10, 4), [11, 12, 13]),
    ]);
    let projection = MinimapPlayfieldProjection::derive(
        &grid,
        Some(&resolved),
        &overlays,
        &colors,
        "TEMPERATE",
        Some(wide_bounds()),
    );
    let geometry = projection.native_radar_surface.expect("surface");
    let raw = projection
        .native_radar_terrain
        .expect("terrain")
        .raw_rgb()
        .to_vec();
    let pair = |cell| {
        let (x, y) = geometry.cell_to_raw_pixel(cell);
        [raw[(y * 146 + x) as usize], raw[(y * 146 + x + 1) as usize]]
    };

    assert_eq!(pair((50, 50)), [[200, 200, 160]; 2], "TerrainClass wins");
    assert_eq!(pair((50, 51)), [[5, 6, 7]; 2], "flag 0x100 uses BRIDGE1 f0");
    assert_eq!(pair((97, 97)), [[11, 12, 13]; 2], "ordinary overlay wins TMP");
    assert_eq!(pair((51, 50)), [[60, 60, 60]; 2], "missing subimage fallback");
    assert_eq!(pair((52, 50)), [[0, 0, 0]; 2], "valid black remains black");
}

fn reference_raw(grid: &TerrainGrid, geometry: NativeRadarSurfaceGeometry) -> Vec<[u8; 3]> {
    let size = geometry.raw_size();
    let mut raw = vec![[0; 3]; (size.0 * size.1) as usize];
    for cell in &grid.cells {
        let (left, right) = radar_colors_for_cell(cell, true, 1.0);
        let origin = geometry.cell_to_raw_pixel((cell.rx, cell.ry));
        for (dx, color) in [(0, left), (1, right)] {
            let x = origin.0 + dx;
            let y = origin.1;
            if x >= 0 && x < size.0 && y >= 0 && y < size.1 {
                raw[(y * size.0 + x) as usize] = color;
            }
        }
    }
    raw
}

/// Independent rectangle-overlap translation of the active loop at
/// `0x00654BE3..0x00654DBC`, retained here so the projection test compares
/// every generated pixel rather than trusting the production sampler.
fn reference_generate(raw: &[[u8; 3]], raw_size: (i32, i32), out: (i32, i32)) -> Vec<u16> {
    let load = |value: f32| X87Chop53::load_f32(NativeF32Bits::from_bits(value.to_bits())).unwrap();
    let store = |value| f32::from_bits(X87Chop53::store_f32(value).unwrap().bits());
    let ftol = |value| X87Chop53::ftol_i64(value).unwrap() as i32;
    let x_step =
        store(X87Chop53::div(X87Chop53::load_i32(raw_size.0), X87Chop53::load_i32(out.0)).unwrap());
    let y_step_extended =
        X87Chop53::div(X87Chop53::load_i32(raw_size.1), X87Chop53::load_i32(out.1)).unwrap();
    let y_step = store(y_step_extended);
    let normalization = store(
        X87Chop53::div(
            X87Chop53::load_f64(NativeF64Bits::ONE).unwrap(),
            X87Chop53::mul(y_step_extended, load(x_step)),
        )
        .unwrap(),
    );
    let mut result = Vec::with_capacity((out.0 * out.1) as usize);
    let mut y0 = 0.0f32;
    for _ in 0..out.1 {
        let y1_extended = X87Chop53::add(load(y0), load(y_step));
        let y1 = store(y1_extended);
        let first_y = ftol(load(y0));
        let last_y = (ftol(y1_extended) + 1).min(raw_size.1);
        let mut x0 = 0.0f32;
        for _ in 0..out.0 {
            let x1_extended = X87Chop53::add(load(x0), load(x_step));
            let x1 = store(x1_extended);
            let first_x = ftol(load(x0));
            let last_x = (ftol(x1_extended) + 1).min(raw_size.0);
            let mut accum = [X87Chop53::load_i32(0); 3];
            for sy in first_y..last_y {
                let wy = overlap(load(y0), y1_extended, sy);
                for sx in first_x..last_x {
                    let wx = overlap(load(x0), load(x1), sx);
                    let weight = X87Chop53::mul(X87Chop53::mul(wx, wy), load(normalization));
                    let sample = raw[(sy * raw_size.0 + sx) as usize];
                    for channel in 0..3 {
                        accum[channel] = X87Chop53::add(
                            accum[channel],
                            X87Chop53::mul(X87Chop53::load_i32(sample[channel] as i32), weight),
                        );
                    }
                }
            }
            let convert = |value| {
                ftol(X87Chop53::add(
                    value,
                    X87Chop53::load_f64(NativeF64Bits::HALF).unwrap(),
                ))
                .min(255) as u8
            };
            result.push(super::super::native_radar_terrain::pack_rgb565(
                convert(accum[0]),
                convert(accum[1]),
                convert(accum[2]),
            ));
            x0 = x1;
        }
        y0 = y1;
    }
    result
}

fn overlap(start: X87Value, end: X87Value, pixel: i32) -> X87Value {
    let pixel_start = X87Chop53::load_i32(pixel);
    let pixel_end = X87Chop53::load_i32(pixel + 1);
    let low = if X87Chop53::compare(start, pixel_start)
        == crate::util::native_x87::X87Ordering::Greater
    {
        start
    } else {
        pixel_start
    };
    let high = if X87Chop53::compare(end, pixel_end) == crate::util::native_x87::X87Ordering::Less {
        end
    } else {
        pixel_end
    };
    X87Chop53::sub(high, low)
}

fn flat_cell(rx: u16, ry: u16, radar_left: [u8; 3], radar_right: [u8; 3]) -> ResolvedTerrainCell {
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
        radar_left,
        radar_right,
        has_damaged_data: false,
        bridgehead_anchor_class_at_load: None,
    }
}
