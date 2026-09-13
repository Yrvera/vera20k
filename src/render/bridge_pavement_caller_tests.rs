//! Perpendicular pavement branch is independent of state-byte/role writes.
use crate::map::bridge_rim_tiles::HighBridgeRimTiles;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::bridge_specs::update_ramp_perpendicular;
use crate::sim::bridge_state::{Axis, BridgeRuntimeState, DamageState, Phase};
use crate::sim::bridge_state::{BridgeCellRole, BridgeRuntimeCell, BridgeheadAnchorClass};

#[test]
fn pavement_raw_caller_gate_keeps_plain_and_structural_endpoint_art() {
    for initial_class in [
        BridgeheadAnchorClass::Variant0,
        BridgeheadAnchorClass::Variant1,
        BridgeheadAnchorClass::Damaged,
        BridgeheadAnchorClass::AboutToFall,
    ] {
        for is_high in [true, false] {
            for axis in [Axis::NS, Axis::EW] {
                for phase in [
                    Phase::DamageA,
                    Phase::DamageB,
                    Phase::CollapseA,
                    Phase::CollapseB,
                ] {
                    for role in [
                        None,
                        Some(BridgeCellRole::Anchor),
                        Some(BridgeCellRole::Bridgehead),
                    ] {
                        let side_a = matches!(phase, Phase::DamageA | Phase::CollapseA);
                        let relative = match (axis, side_a) {
                            (Axis::NS, true) => 3,
                            (Axis::NS, false) => 1,
                            (Axis::EW, true) => 7,
                            (Axis::EW, false) => 5,
                        };
                        let tile = if is_high { 100 } else { 200 } + relative - 1;
                        let mut terrain = endpoint_test_terrain();
                        terrain.test_set_high_bridge_set_starts(Some(100), Some(200));
                        terrain.test_set_high_bridge_rim_tiles(HighBridgeRimTiles::from_ini(100,
                        b"[General]\nBridgeTopLeft1=1\nBridgeTopLeft2=2\nBridgeBottomRight1=3\nBridgeBottomRight2=4\nBridgeTopRight1=5\nBridgeTopRight2=6\nBridgeBottomLeft1=7\nBridgeBottomLeft2=8\nBridgeMiddle1=9\nBridgeMiddle2=14\n"));
                        for (x, y) in [(4, 4), (4, 3), (5, 3)] {
                            let cell = terrain.cell_mut(x, y).unwrap();
                            cell.final_tile_index = tile;
                            cell.has_damaged_data = true;
                        }
                        let mut state = BridgeRuntimeState::default();
                        if let Some(role) = role {
                            state.test_seed_cell(
                                4,
                                4,
                                BridgeRuntimeCell {
                                    deck_present: false,
                                    destroyable: true,
                                    deck_level: 0,
                                    bridge_group_id: None,
                                    damage_state: DamageState::Damaged,
                                    axis: Some(axis),
                                    role,
                                    anchor_span_id: None,
                                    overlay_byte: 0xff,
                                    bridgehead_anchor_class: initial_class,
                                },
                            );
                        }
                        // Native576BA0 selects E/W for NS and S/N for EW.
                        let (dx, dy) = match (axis, side_a) {
                            (Axis::NS, true) => (1, 0),
                            (Axis::NS, false) => (-1, 0),
                            (Axis::EW, true) => (0, 1),
                            (Axis::EW, false) => (0, -1),
                        };
                        let outcome = update_ramp_perpendicular(
                            &mut state,
                            ((4 - dx) as u16, (4 - dy) as u16),
                            axis,
                            phase,
                            is_high,
                            &mut terrain,
                        );
                        assert_eq!(outcome.damaged_variant_cells, [(4, 4), (4, 3), (5, 3)]);
                        if role.is_some() {
                            assert_eq!(
                                state.cell(4, 4).unwrap().bridgehead_anchor_class,
                                initial_class,
                                "endpoint raw tile must not acquire a middle-class override"
                            );
                        }
                        let mut draw_grid = crate::map::terrain::build_terrain_grid_from_resolved(
                            &terrain, None, None,
                        );
                        draw_grid.cells.retain(|cell| (cell.rx, cell.ry) == (4, 4));
                        draw_grid.anchor_variant_table =
                            Some(crate::map::theater::BridgeAnchorVariantTable {
                                ns: [300, 301, 302, 303],
                                ew: [400, 401, 402, 403],
                            });
                        let cell = &draw_grid.cells[0];
                        let selected = std::cell::RefCell::new(Vec::new());
                        let uv = |tile, sub, variant| {
                            selected.borrow_mut().push((tile, sub, variant));
                            None
                        };
                        crate::render::terrain_instances::build_visible_instances(
                            &draw_grid,
                            None,
                            cell.screen_x,
                            cell.screen_y,
                            100.,
                            100.,
                            Some(&uv),
                            Some(&state),
                            Some(&terrain),
                        );
                        assert_eq!(
                            *selected.borrow(),
                            [(tile as u16, 0, 1)],
                            "actual sprite selection must keep the endpoint and use its damaged sibling"
                        );
                        for (x, y) in [(4, 4), (4, 3), (5, 3)] {
                            assert!(terrain.pavement_damaged_at(x, y));
                            assert_eq!(terrain.cell(x, y).unwrap().final_tile_index, tile);
                        }
                    }
                }
            }
        }
    }
}

fn endpoint_test_terrain() -> ResolvedTerrainGrid {
    let mut cells = Vec::with_capacity(20 * 20);
    for ry in 0..20u16 {
        for rx in 0..20u16 {
            cells.push(ResolvedTerrainCell {
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
                height_in_pixels: 0,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                allows_tiberium: false,
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
                base_terrain_class: Default::default(),
                base_speed_costs: Default::default(),
                build_blocked: false,
                has_bridge_deck: false,
                bridge_walkable: false,
                bridge_transition: false,
                bridge_deck_level: 0,
                bridge_layer: None,
                bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                tube_index: None,
                radar_left: [0, 0, 0],
                radar_right: [0, 0, 0],
                has_damaged_data: false,
                bridgehead_anchor_class_at_load: None,
            });
        }
    }
    let mut terrain = ResolvedTerrainGrid::from_cells(20, 20, cells);
    terrain.test_set_high_bridge_rim_tiles(crate::map::bridge_rim_tiles::HighBridgeRimTiles::from_ini(
        0, b"[General]\nBridgeMiddle1=1\nBridgeMiddle2=1\nBridgeTopLeft1=11\nBridgeTopLeft2=12\nBridgeBottomRight1=13\nBridgeBottomRight2=14\nBridgeTopRight1=15\nBridgeTopRight2=16\nBridgeBottomLeft1=17\nBridgeBottomLeft2=18\n"));
    terrain
}

/// Isolated production draw/shader witness with untouched retail map/TMP data.
/// It does not compare gamemd frame pixels or certify full-scene composition.
/// Run with RA2_DIR and VERA20K_XMP34U4_MAP; optional VERA20K_PAVEMENT_PROBE_OUTPUT
/// saves the three readbacks. All native caller/state comparisons live in
/// tools/spatial_oracle/bridge_pavement.{py,json} and the live publisher test.
#[test]
#[ignore = "requires retail assets, VERA20K_XMP34U4_MAP and a GPU"]
fn retail_pavement_live_damage_changes_actual_terrain_pixels() {
    use crate::assets::asset_manager::AssetManager;
    use crate::map::terrain::{TerrainCell, TerrainGrid, TilePlacement};
    use crate::map::theater::{TileKey, load_theater, load_tile_images};
    use std::collections::HashSet;
    use std::path::PathBuf;

    let root = PathBuf::from(std::env::var_os("RA2_DIR").expect("retail root"));
    let map = std::env::var("VERA20K_XMP34U4_MAP").expect("retail xmp34u4 map");
    let mut scenario = crate::headless_scenario::load(&root, &map, 0x0B21_D6E5)
        .expect("normal scenario construction with stock rules/art/terrain");
    let (rx, ry) = (66, 102);
    let terrain = scenario.sim().resolved_terrain.as_ref().unwrap();
    let cell = terrain.cell(rx, ry).unwrap();
    let (tile_id, sub_tile) = terrain.presentation_tile(cell);
    assert_eq!((tile_id, sub_tile), (278, 7));
    assert!(!terrain.pavement_damaged_at(rx, ry));
    assert!(
        scenario
            .sim()
            .bridge_state
            .as_ref()
            .unwrap()
            .cell(rx, ry)
            .is_none()
    );
    let pristine = TileKey {
        tile_id,
        sub_tile,
        variant: 0,
    };
    let damaged = TileKey {
        variant: 1,
        ..pristine
    };
    let mut assets = AssetManager::new(&root).unwrap();
    let theater = load_theater(&mut assets, &scenario.map.header.theater).unwrap();
    let tiles = load_tile_images(
        &assets,
        &theater.lookup,
        &theater.iso_palette,
        &HashSet::from([pristine, damaged]),
    );
    let tile = tiles.get(&pristine).expect("retail pristine TMP");
    let broken = tiles.get(&damaged).expect("retail damaged TMP");
    assert_eq!(
        (tile.width, tile.height, tile.offset_x, tile.offset_y),
        (
            broken.width,
            broken.height,
            broken.offset_x,
            broken.offset_y
        )
    );
    let grid = TerrainGrid {
        cells: vec![TerrainCell {
            screen_x: -(tile.offset_x as f32),
            screen_y: -(tile.offset_y as f32),
            tile_id,
            sub_tile,
            z: cell.level,
            rx,
            ry,
            is_water: false,
            variant: 0,
            tint: [1.0; 3],
            radar_left: [0; 3],
            radar_right: [0; 3],
            has_damaged_data: true,
        }],
        world_width: tile.width as f32,
        world_height: tile.height as f32,
        origin_x: 0.0,
        origin_y: 0.0,
        local_bounds: None,
        anchor_variant_table: None,
    };
    let draw = |sim: &crate::sim::world::Simulation, expected: TileKey| {
        let selected = std::cell::RefCell::new(Vec::new());
        let uv = |tile_id, sub_tile, variant| {
            let key = TileKey {
                tile_id,
                sub_tile,
                variant,
            };
            selected.borrow_mut().push(key);
            tiles.get(&key).map(|image| TilePlacement {
                uv_origin: [0.0; 2],
                uv_size: [1.0; 2],
                pixel_size: [image.width as f32, image.height as f32],
                draw_offset: [image.offset_x as f32, image.offset_y as f32],
            })
        };
        let instances = super::build_visible_instances(
            &grid,
            None,
            0.0,
            0.0,
            tile.width as f32,
            tile.height as f32,
            Some(&uv),
            sim.bridge_state.as_ref(),
            sim.resolved_terrain.as_ref(),
        );
        assert_eq!(*selected.borrow(), [expected]);
        assert_eq!(instances.normal.len(), 1);
        crate::render::depth_gpu_tests::render_terrain_lighting_probe(
            tiles.get(&expected).unwrap(),
            instances.normal[0],
        )
    };
    let before = draw(scenario.sim(), pristine);
    let sim = &mut scenario.runtime.simulation;
    // The normal perpendicular caller reaches plain pavement without a
    // structural runtime entry. Native572330 uses this stock entry/coordinate.
    let outcome = update_ramp_perpendicular(
        sim.bridge_state.as_mut().unwrap(),
        (67, 102),
        Axis::NS,
        Phase::DamageB,
        true,
        sim.resolved_terrain.as_mut().unwrap(),
    );
    assert_eq!(outcome.damaged_variant_cells.len(), 15);
    let after = draw(sim, damaged);
    assert_eq!(before.len(), after.len());
    let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
    assert!(changed > 0, "live damage must reach production GPU pixels");
    for pixels in [&before, &after] {
        assert!(
            pixels.iter().any(|pixel| pixel[..3] != [0, 0, 0]),
            "nonblank output"
        );
    }
    // Explicit scalar clear tests the consumer in both directions. It does
    // not substitute for the separate, still-open engineer repair caller.
    assert_eq!(
        sim.resolved_terrain
            .as_mut()
            .unwrap()
            .apply_native_pavement((rx as i16, ry as i16), false)
            .len(),
        15
    );
    let restored = draw(sim, pristine);
    assert_eq!(restored, before);
    eprintln!(
        "retail pavement {tile_id}/{sub_tile}: GPU changed {changed}/{} pixels; explicit clear restores original pixels",
        before.len()
    );
    if let Some(out) = std::env::var_os("VERA20K_PAVEMENT_PROBE_OUTPUT") {
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        for (name, pixels) in [
            ("pristine.png", &before),
            ("damaged.png", &after),
            ("cleared.png", &restored),
        ] {
            let bytes: Vec<u8> = pixels
                .iter()
                .flat_map(|pixel| pixel.iter().copied())
                .collect();
            image::save_buffer(
                out.join(name),
                &bytes,
                tile.width,
                tile.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}
