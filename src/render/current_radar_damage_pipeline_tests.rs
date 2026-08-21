use super::*;

use crate::map::bridge_facts::{Axis, BridgeCellFacts, BridgeheadAnchorClass};
use crate::map::playfield::PlayfieldBounds;
use crate::map::resolved_terrain::{
    RadarColorMetadata, ResolvedTerrainCell, ResolvedTerrainGrid,
};
use crate::map::terrain::build_terrain_grid_from_resolved;
use crate::render::minimap_projection::MinimapPlayfieldProjection;
use crate::render::radar_terrain_updates::{
    RadarTerrainUpdateLayers, apply_radar_terrain_dirty_cells,
};
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::bridge_state::{
    AnchorSpan, BridgeCellRole, BridgeDamageEvent, BridgeRuntimeCell, BridgeRuntimeState,
    DamageState, Direction,
};
use crate::sim::world::Simulation;

const SIDE: u16 = 64;
const CENTER: (u16, u16) = (25, 25);
const FLOOD: [(u16, u16); 3] = [(26, 25), (26, 24), (26, 23)];
const REPAIR_START: (u16, u16) = (26, 26);
const PRISTINE: [u8; 3] = [10, 20, 30];
const DAMAGED: [u8; 3] = [100, 50, 25];

fn bounds() -> PlayfieldBounds {
    PlayfieldBounds {
        base: 40,
        off_fc: 2,
        off_100: 2,
        off_104: 36,
        off_108: 30,
    }
}

fn cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
    let in_flood = FLOOD.contains(&(rx, ry));
    ResolvedTerrainCell {
        rx,
        ry,
        source_tile_index: if in_flood { 42 } else { 0 },
        source_sub_tile: 0,
        final_tile_index: if in_flood { 42 } else { 0 },
        final_sub_tile: 0,
        is_wood_bridge_repair_tile: false,
        level: 4,
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
        radar_right: [7, 8, 9],
        has_damaged_data: in_flood,
        bridgehead_anchor_class_at_load: None,
    }
}

fn bridge_cell(role: BridgeCellRole, span: Option<u16>, overlay_byte: u8) -> BridgeRuntimeCell {
    BridgeRuntimeCell {
        deck_present: true,
        destroyable: true,
        deck_level: 4,
        bridge_group_id: Some(1),
        damage_state: DamageState::Healthy { variant: 0 },
        axis: Some(Axis::NS),
        role,
        anchor_span_id: span,
        overlay_byte,
        damaged_variant: false,
        bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
    }
}

fn simulation_fixture() -> (Simulation, crate::map::terrain::TerrainGrid) {
    let cells = (0..SIDE)
        .flat_map(|ry| (0..SIDE).map(move |rx| cell(rx, ry)))
        .collect();
    let mut terrain = ResolvedTerrainGrid::from_cells(SIDE, SIDE, cells);
    for &(rx, ry) in &FLOOD {
        terrain.test_set_damaged_radar_metadata(
            rx,
            ry,
            RadarColorMetadata {
                left: [200, 100, 50],
                right: [9, 8, 7],
                valid: true,
            },
        );
    }
    let grid = build_terrain_grid_from_resolved(&terrain, None, None);
    let mut bridge_state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 1500);
    bridge_state.test_seed_cell(
        CENTER.0,
        CENTER.1,
        bridge_cell(BridgeCellRole::Anchor, Some(1), 0),
    );
    for &(rx, ry) in &FLOOD {
        bridge_state.test_seed_cell(rx, ry, bridge_cell(BridgeCellRole::Anchor, None, 0));
    }
    bridge_state.test_seed_anchor_span(AnchorSpan {
        id: 1,
        anchor: CENTER,
        cells: [Some(CENTER), None, None, None, None, None],
        axis: Axis::NS,
        direction: Direction::E,
        damage_state: DamageState::Healthy { variant: 0 },
        bridge_group_id: 1,
    });

    let mut sim = Simulation::new();
    sim.resolved_terrain = Some(terrain);
    sim.bridge_state = Some(bridge_state);
    (sim, grid)
}

fn projection(sim: &Simulation, grid: &crate::map::terrain::TerrainGrid) -> MinimapPlayfieldProjection {
    MinimapPlayfieldProjection::derive(
        grid,
        sim.resolved_terrain.as_ref(),
        &[],
        &HashMap::new(),
        "TEMPERATE",
        Some(bounds()),
        Some(CurrentRadarCellAuthority::new(
            sim.resolved_terrain.as_ref(),
            sim.bridge_state.as_ref(),
            sim.overlay_grid.as_ref(),
            None,
            None,
        )),
    )
}

fn raw_pair(projection: &MinimapPlayfieldProjection, cell: (u16, u16)) -> [[u8; 3]; 2] {
    let geometry = projection.native_radar_surface.expect("native surface");
    let raw = projection
        .native_radar_terrain
        .as_ref()
        .expect("native terrain")
        .raw_rgb();
    let (x, y) = geometry.cell_to_raw_pixel(cell);
    let index = (y * geometry.raw_size().0 + x) as usize;
    [raw[index], raw[index + 1]]
}

fn apply_queue(projection: &mut MinimapPlayfieldProjection, sim: &Simulation) {
    apply_radar_terrain_dirty_cells(
        RadarTerrainUpdateLayers {
            base_rgba: &mut projection.base_rgba,
            terrain_pixels: &projection.terrain_pixels,
            surface_pixels: &mut projection.surface_pixels,
            overlay_pixels: &mut projection.overlay_pixels,
            native_surface: projection.native_radar_surface,
            native_terrain: &mut projection.native_radar_terrain,
        },
        CurrentRadarCellAuthority::new(
            sim.resolved_terrain.as_ref(),
            sim.bridge_state.as_ref(),
            sim.overlay_grid.as_ref(),
            None,
            None,
        ),
        [0, 0, 0],
        &HashMap::new(),
        &sim.radar_terrain_dirty_cells,
    );
}

#[test]
fn gsi_04_01_bridge_damage_producer_queues_exact_flood_and_rearms_after_repair_ack() {
    let (mut sim, grid) = simulation_fixture();
    let mut radar = projection(&sim, &grid);
    assert_eq!(raw_pair(&radar, FLOOD[0]), [PRISTINE; 2]);

    let rules = RuleSet::from_ini(&IniFile::from_str("[General]\n"))
        .expect("minimal bridge damage rules");
    sim.resolve_type_handles(&rules);
    let collapsed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: CENTER.0,
            ry: CENTER.1,
            damage: 1501,
            warhead_ref: Default::default(),
            is_ion_cannon: false,
            impact_z: 4,
        }],
    );
    assert!(!collapsed, "first state-machine hit is absorbed");
    assert_eq!(sim.radar_terrain_dirty_cells, FLOOD);
    assert_eq!(sim.radar_terrain_dirty_generation, 1);
    apply_queue(&mut radar, &sim);
    for cell in FLOOD {
        assert_eq!(raw_pair(&radar, cell), [DAMAGED; 2]);
    }
    assert!(sim.acknowledge_radar_terrain_dirty(1));

    sim.bridge_state
        .as_mut()
        .unwrap()
        .cell_mut(FLOOD[0].0, FLOOD[0].1)
        .unwrap()
        .overlay_byte = 0xD1;
    sim.bridge_state.as_mut().unwrap().test_seed_cell(
        REPAIR_START.0,
        REPAIR_START.1,
        bridge_cell(BridgeCellRole::Anchor, None, 0xD1),
    );
    let repair = sim
        .bridge_state
        .as_mut()
        .unwrap()
        .repair_bridge_from_engineer_scan(
            &[FLOOD[0]],
            &mut sim.mapgen_rng,
            sim.resolved_terrain.as_ref().unwrap(),
        );
    assert_eq!(repair.radar_cells, FLOOD);
    sim.mark_radar_terrain_dirty_cells(repair.radar_cells);
    assert_eq!(sim.radar_terrain_dirty_generation, 2);
    assert_eq!(sim.radar_terrain_dirty_cells, FLOOD);
    apply_queue(&mut radar, &sim);
    for cell in FLOOD {
        assert_eq!(raw_pair(&radar, cell), [PRISTINE; 2]);
    }
    assert!(sim.acknowledge_radar_terrain_dirty(2));
    assert!(sim.radar_terrain_dirty_cells.is_empty());
}

#[test]
fn gsi_04_01_bridge_collapse_batches_variant_preorder_before_existing_dirty_cells() {
    let (mut sim, _grid) = simulation_fixture();
    let rules = RuleSet::from_ini(&IniFile::from_str("[General]\n"))
        .expect("minimal bridge damage rules");
    sim.resolve_type_handles(&rules);

    let collapsed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: CENTER.0,
            ry: CENTER.1,
            damage: 1,
            warhead_ref: Default::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    assert!(collapsed, "Ion retries the state machine through collapse");
    assert_eq!(sim.radar_terrain_dirty_generation, 1);
    assert_eq!(&sim.radar_terrain_dirty_cells[..FLOOD.len()], &FLOOD);
    assert!(sim.radar_terrain_dirty_cells.contains(&CENTER));
    let mut unique = sim.radar_terrain_dirty_cells.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), sim.radar_terrain_dirty_cells.len());
}
