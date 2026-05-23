//! Simulation integration tests — exercises the full tick pipeline: entity spawning,
//! movement commands, combat, bridge traversal, ship pathfinding, deploy/undeploy,
//! and multi-system interactions.

use std::collections::BTreeMap;

use super::*;
use crate::map::entities::{EntityCategory, MapEntity};
use crate::map::houses::HouseAllianceMap;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::terrain;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::bridge_state::{BridgeDamageEvent, BridgeRuntimeState};
use crate::sim::combat::AttackTarget;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::components::MovementTarget;
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::pathfinding::PathGrid;
use crate::util::fixed_math::{SIM_ZERO, SimFixed};

fn make_test_entity(type_id: &str, category: EntityCategory) -> MapEntity {
    MapEntity {
        owner: "Americans".to_string(),
        type_id: type_id.to_string(),
        health: 256,
        cell_x: 30,
        cell_y: 40,
        facing: 64,
        category,
        sub_cell: 0,
        veterancy: 0,
        high: false,
    }
}

fn empty_heights() -> BTreeMap<(u16, u16), u8> {
    BTreeMap::new()
}

/// Create a CommandEnvelope with a string owner, interning it via the sim's interner.
fn cmd_envelope(
    sim: &Simulation,
    owner: &str,
    execute_tick: u64,
    payload: Command,
) -> CommandEnvelope {
    let owner_id = sim
        .interner
        .get(owner)
        .unwrap_or_else(|| panic!("owner '{}' not interned", owner));
    CommandEnvelope::new(owner_id, execute_tick, payload)
}

#[test]
fn despawn_entity_clears_live_radio_contacts() {
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    let htnk = sim.interner.intern("HTNK");
    let mtnk = sim.interner.intern("MTNK");
    let mut despawned = GameEntity::test_default(1, "HTNK", "Americans", 10, 10);
    let mut survivor = GameEntity::test_default(2, "MTNK", "Americans", 11, 10);

    despawned.owner = owner;
    despawned.type_ref = htnk;
    despawned.mark_live_contact_with(2);
    survivor.owner = owner;
    survivor.type_ref = mtnk;
    survivor.mark_live_contact_with(1);
    sim.entities.insert(despawned);
    sim.entities.insert(survivor);

    sim.despawn_entity(1);

    assert!(sim.entities.get(1).is_none());
    assert_eq!(
        sim.entities.get(2).unwrap().radio_contacts,
        Vec::<u64>::new()
    );
}

/// Create a water terrain grid (all cells are water, land_type=4) for ship tests.
fn water_terrain(width: u16, height: u16) -> ResolvedTerrainGrid {
    water_terrain_with_land_type(width, height, 4, false)
}

fn water_terrain_with_land_type(
    width: u16,
    height: u16,
    land_type: u8,
    is_cliff_like: bool,
) -> ResolvedTerrainGrid {
    let mut cells = Vec::new();
    for y in 0..height {
        for x in 0..width {
            cells.push(crate::map::resolved_terrain::ResolvedTerrainCell {
                rx: x,
                ry: y,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                is_wood_bridge_repair_tile: false,
                level: 0,
                filled_clear: false,
                tileset_index: Some(0),
                land_type,
                yr_cell_land_type: land_type,
                slope_type: 0,
                template_height: 0,
                render_offset_x: 0,
                render_offset_y: 0,
                terrain_class: crate::rules::terrain_rules::TerrainClass::Clear,
                speed_costs: crate::rules::terrain_rules::SpeedCostProfile::default(),
                is_water: true,
                is_cliff_like,
                is_cliff_redraw: false,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: false,
                terrain_object_blocks: false,
                overlay_blocks: false,
                zone_type: 4,
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
    ResolvedTerrainGrid::from_cells(width, height, cells)
}

fn single_bridge_cell(rx: u16, ry: u16, deck_level: u8) -> ResolvedTerrainGrid {
    let mut cells = Vec::new();
    for y in 0..=ry {
        for x in 0..=rx {
            cells.push(crate::map::resolved_terrain::ResolvedTerrainCell {
                rx: x,
                ry: y,
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
                terrain_class: crate::rules::terrain_rules::TerrainClass::Clear,
                speed_costs: crate::rules::terrain_rules::SpeedCostProfile::default(),
                is_water: false,
                is_cliff_like: false,
                is_cliff_redraw: false,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: false,
                terrain_object_blocks: false,
                overlay_blocks: false,
                zone_type: 0,
                base_ground_walk_blocked: false,
                base_build_blocked: false,
                base_land_type: 0,
                base_yr_cell_land_type: 0,
                base_terrain_class: Default::default(),
                base_speed_costs: Default::default(),
                build_blocked: false,
                has_bridge_deck: x == rx && y == ry,
                bridge_walkable: x == rx && y == ry,
                bridge_transition: x == rx && y == ry,
                bridge_deck_level: if x == rx && y == ry { deck_level } else { 0 },
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
    ResolvedTerrainGrid::from_cells(rx + 1, ry + 1, cells)
}

fn bridge_cell_with_ground_block(
    rx: u16,
    ry: u16,
    deck_level: u8,
    ground_walk_blocked: bool,
    level: u8,
) -> ResolvedTerrainGrid {
    let mut terrain = single_bridge_cell(rx, ry, deck_level);
    let idx = terrain.index(rx, ry).expect("bridge index");
    let cell = &mut terrain.cells[idx];
    cell.level = level;
    cell.ground_walk_blocked = ground_walk_blocked;
    cell.is_water = ground_walk_blocked;
    cell.base_build_blocked = ground_walk_blocked;
    cell.build_blocked = true;
    terrain
}

/// Build a 3-cell EW bridge strip centered at `(center_rx, ry)`, with the
/// `bridge_state` pre-classified so the orchestrator's HighDirect path
/// fires the walker (overlay 0xDC → final-stage collapse on all 3 cells).
///
/// All 3 bridge cells share the same `level`, `deck_level`, and
/// `ground_walk_blocked` flag. Caller mutates extras like `overlay_blocks`
/// / `terrain_object_blocks` / `is_cliff_like` on the returned terrain
/// before constructing the simulation if a specific fallout shape is
/// being asserted (mutate the center cell at `center_rx, ry`).
///
/// Constraints: `center_rx >= 1` (strip needs the west neighbor in-grid).
fn ew_high_bridge_strip_for_dispatch(
    center_rx: u16,
    ry: u16,
    deck_level: u8,
    ground_walk_blocked: bool,
    level: u8,
) -> (ResolvedTerrainGrid, BridgeRuntimeState) {
    use crate::map::resolved_terrain::ResolvedTerrainCell;
    use crate::sim::bridge_state::{Axis, BridgeCellRole, BridgeRuntimeCell, DamageState};
    assert!(center_rx >= 1, "EW strip needs west neighbor in-grid");

    let width = center_rx + 2; // 0..=(center_rx + 1)
    let height = ry + 1;
    let west = center_rx - 1;
    let east = center_rx + 1;

    let mut cells = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            let on_bridge = y == ry && x >= west && x <= east;
            cells.push(ResolvedTerrainCell {
                rx: x,
                ry: y,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                is_wood_bridge_repair_tile: false,
                level: if on_bridge { level } else { 0 },
                filled_clear: false,
                tileset_index: Some(0),
                land_type: 0,
                yr_cell_land_type: 0,
                slope_type: 0,
                template_height: 0,
                render_offset_x: 0,
                render_offset_y: 0,
                terrain_class: crate::rules::terrain_rules::TerrainClass::Clear,
                speed_costs: crate::rules::terrain_rules::SpeedCostProfile::default(),
                is_water: on_bridge && ground_walk_blocked,
                is_cliff_like: false,
                is_cliff_redraw: false,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: on_bridge && ground_walk_blocked,
                terrain_object_blocks: false,
                overlay_blocks: false,
                zone_type: 0,
                base_ground_walk_blocked: false,
                base_build_blocked: on_bridge && ground_walk_blocked,
                base_land_type: 0,
                base_yr_cell_land_type: 0,
                base_terrain_class: Default::default(),
                base_speed_costs: Default::default(),
                build_blocked: on_bridge,
                has_bridge_deck: on_bridge,
                bridge_walkable: on_bridge,
                bridge_transition: on_bridge,
                bridge_deck_level: if on_bridge { deck_level } else { 0 },
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
    let resolved = ResolvedTerrainGrid::from_cells(width, height, cells);

    // Build bridge state, then override the 3 deck cells with overlay 0xDC
    // (HIGH EW final-eligible). The HighDirect dispatcher path matches on
    // overlay alone (no Z-gate, no role check), so a single hit at any of
    // the 3 cells drives the walker to write 0xE8 / Destroyed across the
    // (this, west, east) triple.
    let mut state = BridgeRuntimeState::from_resolved_terrain(&resolved, true, 15);
    for x in west..=east {
        state.test_seed_cell(
            x,
            ry,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::EW),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: 0xDC,
                damaged_variant: false,
                bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
            },
        );
    }
    (resolved, state)
}

fn alliance_map(pairs: &[(&str, &[&str])]) -> HouseAllianceMap {
    let mut map = HouseAllianceMap::default();
    for &(owner, allies) in pairs {
        let mut set = std::collections::BTreeSet::new();
        for ally in allies {
            set.insert(ally.trim().to_ascii_uppercase());
        }
        map.insert(owner.trim().to_ascii_uppercase(), set);
    }
    map
}

fn combat_test_rules() -> RuleSet {
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n0=E1\n\n\
         [VehicleTypes]\n0=MTNK\n1=AMCV\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=GACNST\n\n\
         [E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
         [AMCV]\nStrength=450\nArmor=heavy\nSpeed=5\nPrimary=none\nDeploysInto=GACNST\n\n\
         [GACNST]\nStrength=1000\nArmor=wood\nFoundation=4x3\nUndeploysInto=AMCV\n\n\
         [M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
         [SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n\n\
         [AP]\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n",
    );
    RuleSet::from_ini(&ini).expect("combat test rules should parse")
}

fn naval_bridge_test_rules() -> RuleSet {
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n\n\
         [VehicleTypes]\n0=BOAT\n1=DRED\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n\n\
         [BOAT]\nStrength=300\nArmor=heavy\nSpeed=6\nMovementZone=Water\nSpeedType=Float\nNaval=yes\n\n\
         [DRED]\nStrength=600\nArmor=heavy\nSpeed=5\nMovementZone=Water\nSpeedType=Float\nNaval=yes\nTooBigToFitUnderBridge=yes\n",
    );
    RuleSet::from_ini(&ini).expect("naval bridge test rules should parse")
}

fn real_ship_test_rules() -> RuleSet {
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n\n\
         [VehicleTypes]\n0=DEST\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n\n\
         [DEST]\nStrength=600\nArmor=heavy\nSpeed=6\nROT=5\nNaval=yes\nLocomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C}\nMovementZone=Water\nSpeedType=Float\nTooBigToFitUnderBridge=yes\n",
    );
    RuleSet::from_ini(&ini).expect("real ship rules should parse")
}

fn teleport_command_test_rules() -> RuleSet {
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n\n\
         [VehicleTypes]\n0=CMIN\n1=CHRONO\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=GAREFN\n\n\
         [CMIN]\nStrength=400\nArmor=light\nSpeed=4\nHarvester=yes\nTeleporter=yes\nDock=GAREFN\n\n\
         [CHRONO]\nStrength=200\nArmor=light\nSpeed=5\nTeleporter=yes\n\n\
         [GAREFN]\nStrength=900\nArmor=wood\nFoundation=4x3\nRefinery=yes\n",
    );
    RuleSet::from_ini(&ini).expect("teleport command rules should parse")
}

#[test]
fn test_spawn_vehicle_has_voxel_marker() {
    let mut sim: Simulation = Simulation::new();
    let entities: Vec<MapEntity> = vec![make_test_entity("MTNK", EntityCategory::Unit)];
    let count: u32 = sim.spawn_from_map(&entities, None, &empty_heights());

    assert_eq!(count, 1);
    let voxel_count: usize = sim.entities.values().filter(|e| e.is_voxel).count();
    assert_eq!(voxel_count, 1, "Vehicle should have VoxelModel marker");
}

#[test]
fn test_spawn_infantry_has_sprite_marker() {
    let mut sim: Simulation = Simulation::new();
    let entities: Vec<MapEntity> = vec![make_test_entity("E1", EntityCategory::Infantry)];
    sim.spawn_from_map(&entities, None, &empty_heights());

    let sprite_count: usize = sim.entities.values().filter(|e| !e.is_voxel).count();
    assert_eq!(sprite_count, 1, "Infantry should have SpriteModel marker");
}

#[test]
fn test_spawn_sets_position_and_facing() {
    let mut sim: Simulation = Simulation::new();
    let entities: Vec<MapEntity> = vec![make_test_entity("HTNK", EntityCategory::Unit)];
    sim.spawn_from_map(&entities, None, &empty_heights());

    for e in sim.entities.values() {
        assert_eq!(e.position.rx, 30);
        assert_eq!(e.position.ry, 40);
        assert_eq!(e.facing, 64);
        assert_eq!(sim.interner.resolve(e.type_ref), "HTNK");
        // lepton_to_screen = CoordsToClient(cell_center) = (30*(30-40), 15*(30+40)+15) = (-300, 1065)
        assert!((e.position.screen_x - (-300.0)).abs() < 0.1);
        assert!((e.position.screen_y - 1065.0).abs() < 0.1);
    }
}

#[test]
fn test_spawn_from_map_high_unit_uses_bridge_layer_and_deck_level() {
    let mut sim = Simulation::new();
    let heights = empty_heights();
    let resolved = single_bridge_cell(5, 5, 3);
    let count = sim.spawn_from_map_with_resolved(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 5,
            cell_y: 5,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: true,
        }],
        Some(&combat_test_rules()),
        &heights,
        Some(&resolved),
    );

    assert_eq!(count, 1);
    let e = sim.entities.get(1).expect("spawned entity");
    assert_eq!(e.position.z, 3);
    let bridge = e.bridge_occupancy.as_ref().expect("bridge occupancy");
    assert_eq!(bridge.deck_level, 3);
    assert!(e.on_bridge);
    let loco = e.locomotor.as_ref().expect("loco");
    assert_eq!(loco.layer, MovementLayer::Bridge);
}

#[test]
fn test_spawn_from_map_high_without_bridge_falls_back_to_ground() {
    let mut sim = Simulation::new();
    let heights = BTreeMap::from([((5, 5), 1)]);
    let resolved = ResolvedTerrainGrid::from_cells(
        6,
        6,
        (0..6u16)
            .flat_map(|ry| {
                (0..6u16).map(
                    move |rx| crate::map::resolved_terrain::ResolvedTerrainCell {
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
                        terrain_class: crate::rules::terrain_rules::TerrainClass::Clear,
                        speed_costs: crate::rules::terrain_rules::SpeedCostProfile::default(),
                        is_water: false,
                        is_cliff_like: false,
                        is_cliff_redraw: false,
                        variant: 0,
                        is_rough: false,
                        is_road: false,
                        accepts_smudge: false,
                        has_ramp: false,
                        canonical_ramp: None,
                        ground_walk_blocked: false,
                        terrain_object_blocks: false,
                        overlay_blocks: false,
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
                    },
                )
            })
            .collect(),
    );
    sim.spawn_from_map_with_resolved(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 5,
            cell_y: 5,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: true,
        }],
        Some(&combat_test_rules()),
        &heights,
        Some(&resolved),
    );
    let e = sim.entities.get(1).expect("spawned entity");
    assert_eq!(e.position.z, 1);
    assert!(e.bridge_occupancy.is_none());
    assert!(!e.on_bridge);
    let loco = e.locomotor.as_ref().expect("loco");
    assert_eq!(loco.layer, MovementLayer::Ground);
}

#[test]
fn test_bridge_damage_rebuilds_path_grid() {
    // 3-cell EW strip at (1..=3, 0), all overlay 0xDC. HighDirect dispatcher
    // path fires the EW walker → all 3 cells transition to 0xE8 + Destroyed
    // → `is_bridge_walkable` returns false → rebuilt path grid says no bridge
    // layer at any of the 3 cells.
    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(2, 0, 2, false, 0);
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    // Build PathGrid before damage — all 3 cells walkable on bridge layer.
    let grid_before =
        PathGrid::from_resolved_terrain_with_bridges(&resolved, sim.bridge_state.as_ref());
    for x in 1..=3 {
        assert!(grid_before.is_walkable_on_layer(x, 0, MovementLayer::Bridge));
    }

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 2,
            ry: 0,
            damage: 20,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    // Rebuild PathGrid after damage — none of the 3 cells walkable on bridge.
    let grid_after = PathGrid::from_resolved_terrain_with_bridges(
        sim.resolved_terrain.as_ref().unwrap(),
        sim.bridge_state.as_ref(),
    );
    for x in 1..=3 {
        assert!(
            !grid_after.is_walkable_on_layer(x, 0, MovementLayer::Bridge),
            "cell ({x}, 0) should not be walkable on bridge layer after collapse"
        );
    }
}

/// End-to-end: a bridge collapse driven through the orchestrator must
/// signal `state_changed = true`, AND the PathGrid that the app would
/// rebuild post-tick (via PathGrid::from_resolved_terrain_with_bridges)
/// must show the collapsed cells as non-walkable on the bridge layer.
///
/// Ledger #1 (one-tick delay), #4 (ground revert), #9 (layer separation).
#[test]
fn test_bridge_collapse_signals_pathgrid_refresh() {
    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(2, 0, 2, false, 0);
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);

    let state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 2,
            ry: 0,
            damage: 20,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );
    assert!(
        state_changed,
        "orchestrator must signal state_changed=true on collapse"
    );

    // The PathGrid the app would build after this tick (via rebuild_
    // dynamic_path_grid → PathGrid::from_resolved_terrain_with_bridges):
    let post_tick_grid = PathGrid::from_resolved_terrain_with_bridges(
        sim.resolved_terrain.as_ref().unwrap(),
        sim.bridge_state.as_ref(),
    );
    for x in 1..=3 {
        assert!(
            !post_tick_grid.is_walkable_on_layer(x, 0, MovementLayer::Bridge),
            "cell ({x}, 0) must not be walkable on bridge layer after collapse"
        );
    }
}

/// No-collapse tick must NOT signal state_changed. Empty event lists
/// (no bridge damage this tick) leave the path grid untouched — avoids
/// firing unnecessary refresh ticks.
#[test]
fn test_no_collapse_does_not_signal_refresh() {
    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(2, 0, 2, false, 0);
    sim.resolved_terrain = Some(resolved);
    sim.bridge_state = Some(bridge_state);

    let rules = combat_test_rules();

    let state_changed =
        crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(&mut sim, &rules, &[]);
    assert!(!state_changed, "empty events must not signal state_changed");
}

/// Regression for ledger #2 / #3: when a bridge body span collapses, every
/// cell that previously had `transition: true` must lose it. Otherwise A*
/// would still permit Ground→Bridge entry into the destroyed span.
///
/// The fixture seeds Body-role cells with `bridge_transition = true`
/// (mimicking bridgeheads from the PathCell projection's perspective).
/// Post-collapse, `from_resolved_terrain_with_bridges` gates `transition`
/// on `is_bridge_walkable`, so all 3 cells must drop the flag.
///
/// Guards against future per-cell-delta optimizations that might only
/// update the directly-destroyed cell and miss adjacent transition cells.
#[test]
fn test_bridge_collapse_clears_transition_flag() {
    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(2, 0, 2, false, 0);
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    // Snapshot cells with transition=true before damage.
    let grid_before =
        PathGrid::from_resolved_terrain_with_bridges(&resolved, sim.bridge_state.as_ref());
    let transition_cells_before: Vec<(u16, u16)> = (0..resolved.width())
        .flat_map(|x| (0..resolved.height()).map(move |y| (x, y)))
        .filter_map(|(x, y)| {
            let cell = grid_before.cell(x, y)?;
            cell.transition.then_some((x, y))
        })
        .collect();
    assert!(
        !transition_cells_before.is_empty(),
        "test fixture must have at least one transition cell"
    );

    // Damage event collapses the entire EW strip.
    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _ = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 2,
            ry: 0,
            damage: 20,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    let grid_after = PathGrid::from_resolved_terrain_with_bridges(
        sim.resolved_terrain.as_ref().unwrap(),
        sim.bridge_state.as_ref(),
    );
    for (x, y) in &transition_cells_before {
        let cell = grid_after.cell(*x, *y).expect("cell exists");
        assert!(
            !cell.transition,
            "cell ({x}, {y}) must lose transition flag after bridge collapse"
        );
    }
}

#[test]
fn test_destroyed_bridge_snaps_unit_to_ground_when_ground_exists() {
    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 3, false, 1);
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    sim.spawn_from_map_with_resolved(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 5,
            cell_y: 5,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: true,
        }],
        Some(&combat_test_rules()),
        &BTreeMap::from([((5, 5), 1)]),
        Some(&resolved),
    );

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 5,
            ry: 5,
            damage: 15,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    let e = sim.entities.get(1).expect("surviving bridge unit");
    assert_eq!(e.position.z, 1);
    assert!(e.bridge_occupancy.is_none());
    assert!(!e.on_bridge);
    let loco = e.locomotor.as_ref().expect("locomotor");
    assert_eq!(loco.layer, MovementLayer::Ground);
    assert!(e.movement_target.is_none());
}

/// Per HIGH §12.7 / §12.9: deck units snap to ground level on collapse —
/// no damage, no despawn, even when the ground below is unwalkable (water,
/// `is_water=true` + `ground_walk_blocked=true`). Vanilla has no drown
/// mechanism.
#[test]
fn test_destroyed_bridge_snaps_unit_to_ground_over_water_below() {
    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 3, true, 0);
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    sim.spawn_from_map_with_resolved(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 5,
            cell_y: 5,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: true,
        }],
        Some(&combat_test_rules()),
        &BTreeMap::new(),
        Some(&resolved),
    );

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 5,
            ry: 5,
            damage: 15,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    // DropIn correction: unit ALIVE, snapped to ground level=0, OnBridge
    // cleared, locomotor flipped to Ground/Idle.
    let e = sim
        .entities
        .get(1)
        .expect("deck unit must SURVIVE collapse over water");
    assert_eq!(
        e.health.current, e.health.max,
        "DropIn never harms — health stays at max"
    );
    assert_eq!(e.position.z, 0, "snapped to ground level");
    assert!(!e.on_bridge);
    assert!(e.bridge_occupancy.is_none());
    let loco = e.locomotor.as_ref().expect("locomotor");
    assert_eq!(loco.layer, MovementLayer::Ground);
    assert!(e.movement_target.is_none());
}

/// Same DropIn correction over an overlay-blocked ground cell.
#[test]
fn test_destroyed_bridge_snaps_unit_to_ground_over_overlay_blocked() {
    let mut sim = Simulation::new();
    let (mut resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 3, false, 0);
    let idx = resolved.index(5, 5).expect("bridge index");
    resolved.cells[idx].overlay_blocks = true;
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    sim.spawn_from_map_with_resolved(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 5,
            cell_y: 5,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: true,
        }],
        Some(&combat_test_rules()),
        &BTreeMap::new(),
        Some(&resolved),
    );

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 5,
            ry: 5,
            damage: 15,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    let e = sim
        .entities
        .get(1)
        .expect("deck unit must SURVIVE over overlay-blocked ground");
    assert_eq!(e.health.current, e.health.max, "DropIn never harms");
    assert_eq!(e.position.z, 0);
    assert!(!e.on_bridge);
    assert!(e.bridge_occupancy.is_none());
}

/// Same DropIn correction over a terrain-object-blocked ground cell.
#[test]
fn test_destroyed_bridge_snaps_unit_to_ground_over_terrain_object_blocked() {
    let mut sim = Simulation::new();
    let (mut resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 3, false, 0);
    let idx = resolved.index(5, 5).expect("bridge index");
    resolved.cells[idx].terrain_object_blocks = true;
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    sim.spawn_from_map_with_resolved(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 5,
            cell_y: 5,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: true,
        }],
        Some(&combat_test_rules()),
        &BTreeMap::new(),
        Some(&resolved),
    );

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 5,
            ry: 5,
            damage: 15,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    let e = sim
        .entities
        .get(1)
        .expect("deck unit must SURVIVE over terrain-object-blocked ground");
    assert_eq!(e.health.current, e.health.max, "DropIn never harms");
    assert_eq!(e.position.z, 0);
    assert!(!e.on_bridge);
    assert!(e.bridge_occupancy.is_none());
}

/// After collapse, the rebuilt path grid reverts the bridge cell to its
/// underlying ground walkability — a cliff-like cell stays unwalkable
/// (per `from_resolved_terrain_with_bridges`'s `is_cliff_like` branch).
/// Plus the DropIn correction: the deck unit still survives.
#[test]
fn test_destroyed_bridge_fallout_matches_rebuilt_ground_walkability() {
    let mut sim = Simulation::new();
    let (mut resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 3, false, 0);
    let idx = resolved.index(5, 5).expect("bridge index");
    resolved.cells[idx].is_cliff_like = true;
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    sim.spawn_from_map_with_resolved(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 5,
            cell_y: 5,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: true,
        }],
        Some(&combat_test_rules()),
        &BTreeMap::new(),
        Some(&resolved),
    );

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 5,
            ry: 5,
            damage: 15,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    let rebuilt_grid = PathGrid::from_resolved_terrain_with_bridges(
        sim.resolved_terrain.as_ref().expect("resolved terrain"),
        sim.bridge_state.as_ref(),
    );
    assert!(
        !rebuilt_grid.is_walkable_on_layer(5, 5, MovementLayer::Bridge),
        "destroyed bridge layer should be unwalkable"
    );
    assert!(
        !rebuilt_grid.is_walkable_on_layer(5, 5, MovementLayer::Ground),
        "destroyed cliff-like cell falls back to unwalkable underlying terrain"
    );
    // DropIn correction: the unit survived stranded at ground level even
    // though the underlying ground is cliff-like (vanilla never despawns).
    let e = sim.entities.get(1).expect("deck unit survives");
    assert_eq!(e.health.current, e.health.max, "DropIn never harms");
    assert!(!e.on_bridge);
}

/// Full-pipeline cascade: ground-layer entity at a destroyed bridge cell is
/// force-killed (health=0, dying=true) per HIGH §11.4 step 1 — mirrors the
/// binary's `BlowUpBridge` ground-occupant pass with C4Warhead semantics.
/// Bridge-deck entities go through DropIn (Step 2) and survive; this test
/// covers the parallel ground-layer path.
#[test]
fn test_bridge_collapse_kills_ground_unit_under_destroyed_cell() {
    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 3, false, 0);
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    // Spawn a GROUND unit at (5, 5) — same cell as the bridge above.
    // `high: false` → spawn places it on the ground layer; on_bridge=false.
    sim.spawn_from_map_with_resolved(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 5,
            cell_y: 5,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: false,
        }],
        Some(&combat_test_rules()),
        &BTreeMap::new(),
        Some(&resolved),
    );
    let id = sim
        .entities
        .iter_sorted()
        .next()
        .map(|(id, _)| id)
        .expect("ground unit spawned");
    assert!(!sim.entities.get(id).unwrap().on_bridge, "ground layer");

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _ = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 5,
            ry: 5,
            damage: 15,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    let e = sim
        .entities
        .get(id)
        .expect("ground unit still in EntityStore (kill is via dying flag)");
    assert_eq!(e.health.current, 0, "kill_ground_occupants_at zeroed HP");
    assert!(e.dying, "dying flag set for next combat-tick death effects");
    assert!(e.attack_target.is_none());
    assert!(e.movement_target.is_none());
}

/// Full-pipeline walker: a single Ion-Cannon hit at the center of a 3-cell
/// EW strip drives the HighDirect dispatcher → walker → all 3 cells of the
/// (this, west, east) triple get overlay 0xE8 + DamageState::Destroyed.
#[test]
fn test_bridge_walker_collapses_full_3_cell_strip_on_single_hit() {
    use crate::sim::bridge_state::DamageState;
    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 3, false, 0);
    sim.resolved_terrain = Some(resolved);
    sim.bridge_state = Some(bridge_state);

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _ = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 5,
            ry: 5,
            damage: 15,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    let bs = sim.bridge_state.as_ref().unwrap();
    for x in 4..=6 {
        let cell = bs.cell(x, 5).expect("bridge cell present");
        assert_eq!(
            cell.damage_state,
            DamageState::Destroyed,
            "cell ({x}, 5) must be destroyed by walker triple"
        );
        assert_eq!(
            cell.overlay_byte, 0xE8,
            "cell ({x}, 5) must hold the EW final-stage overlay"
        );
    }
}

/// Path mutual exclusion: a cell whose overlay has been TRANSITIONED out
/// of the raw body range (e.g. 0x6) routes to the state-machine path,
/// never to direct-overlay. The reverse (raw body overlay) routes to the
/// direct path. Verifies the dispatcher's overlay invariant prevents
/// double-firing on the same hit.
#[test]
fn test_bridge_dispatcher_state_machine_overlay_routes_to_high_sm_not_direct() {
    use crate::sim::bridge_state::{
        AnchorSpan, Axis, BridgeCellRole, BridgeRuntimeCell, DamageState, Direction, DispatchPath,
    };
    let mut sim = Simulation::new();
    let (resolved, mut bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 4, false, 0);
    // Override center cell to the post-transition state: overlay 0x6 (out
    // of the 0xCD..=0xE6 raw HIGH range), role=Anchor, damage_state=Damaged
    // (so a single hit Damaged→Destroyed). Anchor span carries only the
    // anchor itself so set_bridge_direction emits one BlowUpBridge action.
    bridge_state.test_seed_cell(
        5,
        5,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Damaged,
            axis: Some(Axis::EW),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x6,
            damaged_variant: false,
            bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
        },
    );
    bridge_state.test_seed_anchor_span(AnchorSpan {
        id: 1,
        anchor: (5, 5),
        cells: [Some((5, 5)), None, None, None, None, None],
        axis: Axis::EW,
        direction: Direction::S,
        damage_state: DamageState::Damaged,
        bridge_group_id: 1,
    });
    sim.resolved_terrain = Some(resolved);
    sim.bridge_state = Some(bridge_state);

    // Path classifier: HighSM matches, HighDirect does NOT, when the
    // overlay has been transitioned out of the raw body range.
    let bs = sim.bridge_state.as_ref().unwrap();
    let ctx = crate::sim::bridge_state::BridgeDamageContext {
        damage: 15,
        warhead_ref: crate::sim::intern::InternedId::default(),
        is_ion_cannon: true,
        bridge_strength: bs.bridge_strength(),
        impact_z: 0,
    };
    let terrain = sim.resolved_terrain.as_ref().unwrap();
    assert!(
        bs.path_matches_cell(DispatchPath::HighStateMachine, 5, 5, &ctx, terrain),
        "transitioned overlay routes to HighSM"
    );
    assert!(
        !bs.path_matches_cell(DispatchPath::HighDirect, 5, 5, &ctx, terrain),
        "transitioned overlay must NOT also match HighDirect"
    );

    // Conversely: a cell still in the raw body range routes to HighDirect
    // and NOT to HighSM. Re-seed (4, 5) with overlay 0xDC.
    let bs_mut = sim.bridge_state.as_mut().unwrap();
    bs_mut.test_seed_cell(
        4,
        5,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::EW),
            role: BridgeCellRole::Body,
            anchor_span_id: Some(1),
            overlay_byte: 0xDC,
            damaged_variant: false,
            bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
        },
    );
    let bs = sim.bridge_state.as_ref().unwrap();
    assert!(
        bs.path_matches_cell(DispatchPath::HighDirect, 4, 5, &ctx, terrain),
        "raw body overlay routes to HighDirect"
    );
    assert!(
        !bs.path_matches_cell(DispatchPath::HighStateMachine, 4, 5, &ctx, terrain),
        "raw body overlay must NOT also match HighSM"
    );
}

/// Integration test: full apply_bridge_damage_events pipeline on a
/// state-machine path. Anchor cell with overlay 0x6 + Damaged →
/// body driver fires Damaged→Destroyed → endpoint deactivation cascade
/// runs via `refresh_bridge_zones_if_dirty`. Independently exercises the
/// HighSM path (Task 15 coverage focused on HighDirect).
#[test]
fn test_bridge_orchestrator_state_machine_path_collapses_anchor_and_deactivates_endpoint() {
    use crate::sim::bridge_state::{
        AnchorSpan, Axis, BridgeCellRole, BridgeRuntimeCell, DamageState, Direction,
    };
    let mut sim = Simulation::new();
    // Use the strip helper so resolved_terrain has a bridge group with
    // ground neighbors → endpoint records exist.
    let (resolved, mut bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 4, false, 0);
    // Override (5, 5) to the post-transition Damaged state-machine setup.
    bridge_state.test_seed_cell(
        5,
        5,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Damaged,
            axis: Some(Axis::EW),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x6,
            damaged_variant: false,
            bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
        },
    );
    bridge_state.test_seed_anchor_span(AnchorSpan {
        id: 1,
        anchor: (5, 5),
        cells: [Some((5, 5)), None, None, None, None, None],
        axis: Axis::EW,
        direction: Direction::S,
        damage_state: DamageState::Damaged,
        bridge_group_id: 1,
    });
    sim.resolved_terrain = Some(resolved);
    sim.bridge_state = Some(bridge_state);

    let pre_active: Vec<bool> = sim
        .bridge_state
        .as_ref()
        .unwrap()
        .endpoint_records()
        .iter()
        .map(|r| r.active)
        .collect();

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _ = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 5,
            ry: 5,
            damage: 15,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 0, // Z-gate window for level=0 is [-1, 1]
        }],
    );

    let bs = sim.bridge_state.as_ref().unwrap();
    assert_eq!(
        bs.cell(5, 5).unwrap().damage_state,
        DamageState::Destroyed,
        "body driver collapsed anchor on Damaged→Destroyed"
    );
    // Endpoint deactivation: at least one record was active pre-collapse
    // and any active record for group 1 is now inactive.
    if pre_active.iter().any(|&a| a) {
        let post_active: Vec<bool> = bs.endpoint_records().iter().map(|r| r.active).collect();
        assert!(
            post_active.iter().all(|&a| !a),
            "all group-1 endpoints must deactivate after collapse \
             (pre={pre_active:?}, post={post_active:?})"
        );
    }
}

/// Determinism: two independent simulations with identical seeds, identical
/// resolved terrain, identical bridge runtime state, and identical damage
/// events MUST produce the same state hash after running
/// `apply_bridge_damage_events`. Lockstep invariant — any divergence in
/// RNG draw order, iteration order, or non-deterministic sets desyncs
/// multiplayer.
#[test]
fn test_bridge_collapse_is_deterministic_under_replay() {
    fn run_one_collapse(seed: u64) -> u64 {
        let mut sim = Simulation::new();
        sim.rng = crate::sim::rng::SimRng::new(seed);
        let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 4, false, 0);
        sim.resolved_terrain = Some(resolved);
        sim.bridge_state = Some(bridge_state);

        let mut rules = combat_test_rules();
        rules.resolve_bridge_warheads(&mut sim.interner);
        let _ = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
            &mut sim,
            &rules,
            &[BridgeDamageEvent {
                rx: 5,
                ry: 5,
                damage: 100,
                warhead_ref: crate::sim::intern::InternedId::default(),
                is_ion_cannon: false, // exercises per-path RNG gate
                impact_z: 4,
            }],
        );
        sim.state_hash()
    }

    let h1 = run_one_collapse(0xCAFE_F00D);
    let h2 = run_one_collapse(0xCAFE_F00D);
    assert_eq!(
        h1, h2,
        "identical seed + inputs must produce identical post-collapse state hash"
    );
}

/// Replay determinism with bridge collapse + rim refresh. The new
/// `update_adjacent_bridges` step in the cascade introduces additional
/// `BridgeRuntimeCell` writes (`damaged_variant`, `damage_state` resets).
/// This test pins that those mutations are deterministic across two
/// identical-seed runs, so the new sim writes can never silently desync
/// lockstep.
#[test]
fn replay_determinism_with_bridge_collapse_and_rim_refresh() {
    fn run_one(seed: u64) -> u64 {
        let mut sim = Simulation::new();
        sim.rng = crate::sim::rng::SimRng::new(seed);
        let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 4, false, 0);
        sim.resolved_terrain = Some(resolved);
        sim.bridge_state = Some(bridge_state);

        let mut rules = combat_test_rules();
        rules.resolve_bridge_warheads(&mut sim.interner);
        let _ = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
            &mut sim,
            &rules,
            &[BridgeDamageEvent {
                rx: 5,
                ry: 5,
                damage: 100,
                warhead_ref: crate::sim::intern::InternedId::default(),
                is_ion_cannon: false,
                impact_z: 4,
            }],
        );
        sim.state_hash()
    }

    let h1 = run_one(0xFEED_BEEF);
    let h2 = run_one(0xFEED_BEEF);
    assert_eq!(
        h1, h2,
        "identical seed + inputs must produce identical state hash across the rim-refresh cascade"
    );
}

/// Snapshot regression: serialize the `BridgeRuntimeState` after a collapse
/// (overlay-byte progression + DamageState::Destroyed cells +
/// endpoint_records active flips), deserialize it, and assert the
/// post-restore state matches the pre-serialize state. Locks down the
/// snapshot contract across the orchestrator switchover.
#[test]
fn test_bridge_snapshot_roundtrip_preserves_state_after_collapse() {
    use crate::sim::bridge_state::DamageState;
    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 4, false, 0);
    sim.resolved_terrain = Some(resolved);
    sim.bridge_state = Some(bridge_state);

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _ = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 5,
            ry: 5,
            damage: 15,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    let pre = sim.bridge_state.as_ref().unwrap().clone();
    let json = serde_json::to_string(&pre).expect("serialize bridge_state");
    let restored: crate::sim::bridge_state::BridgeRuntimeState =
        serde_json::from_str(&json).expect("deserialize");

    // Compare every cell in the strip + bridge_strength + endpoint_records.
    for x in 4..=6 {
        let pre_cell = pre.cell(x, 5).expect("pre cell");
        let post_cell = restored.cell(x, 5).expect("restored cell");
        assert_eq!(pre_cell, post_cell, "cell ({x}, 5) round-trip");
        assert_eq!(post_cell.damage_state, DamageState::Destroyed);
        assert_eq!(post_cell.overlay_byte, 0xE8);
    }
    assert_eq!(pre.bridge_strength(), restored.bridge_strength());
    assert_eq!(
        pre.endpoint_records().len(),
        restored.endpoint_records().len()
    );
    for (a, b) in pre
        .endpoint_records()
        .iter()
        .zip(restored.endpoint_records())
    {
        assert_eq!(a.active, b.active, "endpoint record active flag round-trip");
        assert_eq!(
            a.bridge_kind, b.bridge_kind,
            "endpoint record kind round-trip"
        );
    }
}

/// RNG draw-count parity: per-event the dispatcher consumes RNG draws in
/// a fixed sequence. With non-IonCannon damage:
///   1. Per-path BridgeStrength gate fires once before the first matching
///      driver — `next_range_u32_inclusive(1, bridge_strength)`.
///   2. Driver dispatch (walker / state machine) does not draw RNG itself.
///   3. Cascade `spawn_bridge_debris` per destroyed cell consumes its
///      well-known sequence (covered by orchestrator unit tests).
///
/// This integration test pins step 1: with `is_ion_cannon=false`, the
/// orchestrator pulls exactly one BridgeStrength roll before falling
/// through to HighDirect (HighSM raw-overlay rejects, LowSM rejects,
/// LowDirect rejects). A parallel RNG primed with the same seed must
/// yield the same post-event state.
#[test]
fn test_bridge_dispatcher_consumes_one_path_gate_draw_per_non_ion_event() {
    let seed = 0xABCD_1234_u64;
    let mut sim = Simulation::new();
    sim.rng = crate::sim::rng::SimRng::new(seed);
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 4, false, 0);
    let bridge_strength = bridge_state.bridge_strength();
    sim.resolved_terrain = Some(resolved);
    sim.bridge_state = Some(bridge_state);

    // Predict: HighSM rejected on raw-overlay; LowSM rejected
    // (deck_level=4 vs want_high=false); LowDirect rejected (overlay not
    // in LOW range). HighDirect matches → one BridgeStrength gate roll
    // → walker (consumes no RNG) → cascade spawn_bridge_debris (consumes
    // the well-known per-cell sequence — but with both bridge_explosions
    // and metallic_debris empty in this fixture, the helper short-circuits
    // on the empty-lists check and draws no RNG).
    let mut predicted = crate::sim::rng::SimRng::new(seed);
    let _gate = predicted.next_range_u32_inclusive(1, bridge_strength as u32);

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    // High damage so the gate roll passes deterministically (any roll < 9999
    // succeeds when damage > roll).
    let _ = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 5,
            ry: 5,
            damage: 9999,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: false,
            impact_z: 4,
        }],
    );

    assert_eq!(
        sim.rng.state(),
        predicted.state(),
        "non-IonCannon hit must consume exactly one BridgeStrength gate roll"
    );
}

#[test]
fn test_water_mover_lookahead_does_not_attach_bridge_occupancy_under_bridge() {
    let rules = naval_bridge_test_rules();
    let mut sim = Simulation::new();
    let resolved = bridge_cell_with_ground_block(1, 0, 3, true, 0);
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(BridgeRuntimeState::from_resolved_terrain(
        &resolved, true, 15,
    ));
    let boat_id = sim
        .spawn_object("BOAT", "Americans", 0, 0, 64, &rules, &BTreeMap::new())
        .expect("spawn boat");
    let boat = sim.entities.get_mut(boat_id).expect("boat entity");
    boat.movement_target = Some(MovementTarget {
        path: vec![(0, 0), (1, 0)],
        path_layers: vec![MovementLayer::Ground, MovementLayer::Ground],
        next_index: 1,
        speed: SimFixed::from_num(256),
        current_speed: SimFixed::from_num(256),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });

    let path_grid = PathGrid::new(2, 1);
    let _ = sim.advance_tick(
        &[],
        Some(&rules),
        &BTreeMap::new(),
        Some(&path_grid),
        None,
        33,
    );

    let boat = sim.entities.get(boat_id).expect("boat still exists");
    assert!(
        boat.bridge_occupancy.is_none(),
        "Ship under a bridge should stay on the water layer"
    );
    assert_eq!(boat.position.z, 0);
}

#[test]
fn test_too_big_ship_can_move_under_bridge_route() {
    let rules = naval_bridge_test_rules();
    let mut sim = Simulation::new();
    // Build a 2x1 water terrain where cell (1,0) has a bridge deck.
    // Water movers need land_type=4 (Water) for passability.
    let mut resolved = water_terrain(2, 1);
    let idx = resolved.index(1, 0).expect("bridge cell index");
    resolved.cells[idx].has_bridge_deck = true;
    resolved.cells[idx].bridge_walkable = true;
    resolved.cells[idx].bridge_transition = true;
    resolved.cells[idx].bridge_deck_level = 3;
    resolved.cells[idx].ground_walk_blocked = true;
    resolved.cells[idx].build_blocked = true;
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(BridgeRuntimeState::from_resolved_terrain(
        &resolved, true, 15,
    ));
    let ship_id = sim
        .spawn_object("DRED", "Americans", 0, 0, 64, &rules, &BTreeMap::new())
        .expect("spawn dreadnought");
    let ship = sim.entities.get_mut(ship_id).expect("ship entity");
    ship.movement_target = Some(MovementTarget {
        path: vec![(0, 0), (1, 0)],
        path_layers: vec![MovementLayer::Ground, MovementLayer::Ground],
        next_index: 1,
        speed: SimFixed::from_num(256),
        current_speed: SimFixed::from_num(256),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });

    // Use tick_ms=1000 so the ship crosses the cell boundary in 1 tick
    // (speed=256 * dt=1.0 = 256 leptons = 1 cell).
    let path_grid = PathGrid::new(2, 1);
    let _ = sim.advance_tick(
        &[],
        Some(&rules),
        &BTreeMap::new(),
        Some(&path_grid),
        None,
        1000,
    );

    let ship = sim.entities.get(ship_id).expect("ship still exists");
    assert!(
        ship.movement_target.is_none(),
        "Naval ships should finish a direct move under bridge structural cells in the experimental behavior"
    );
    assert_eq!((ship.position.rx, ship.position.ry), (1, 0));
}

#[test]
fn test_ship_turn_path_completes_without_drive_track_stall() {
    let rules = naval_bridge_test_rules();
    let mut sim = Simulation::new();
    // Water movers need resolved_terrain with water cells (land_type=4) for
    // the passability check in is_cell_passable_for_mover.
    sim.resolved_terrain = Some(water_terrain(3, 3));
    let boat_id = sim
        .spawn_object("BOAT", "Americans", 0, 0, 64, &rules, &BTreeMap::new())
        .expect("spawn boat");
    let boat = sim.entities.get_mut(boat_id).expect("boat entity");
    boat.movement_target = Some(MovementTarget {
        path: vec![(0, 0), (1, 0), (1, 1)],
        path_layers: vec![
            MovementLayer::Ground,
            MovementLayer::Ground,
            MovementLayer::Ground,
        ],
        next_index: 1,
        speed: SimFixed::from_num(1024),
        current_speed: SimFixed::from_num(1024),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });

    let path_grid = PathGrid::new(3, 3);
    for _ in 0..10 {
        let _ = sim.advance_tick(
            &[],
            Some(&rules),
            &BTreeMap::new(),
            Some(&path_grid),
            None,
            100,
        );
    }

    let boat = sim.entities.get(boat_id).expect("boat still exists");
    assert_eq!(
        (boat.position.rx, boat.position.ry),
        (1, 1),
        "ship should finish a simple turn path instead of stalling in place"
    );
    assert!(
        boat.movement_target.is_none(),
        "ship movement should complete after reaching the goal"
    );
}

#[test]
fn test_real_ship_locomotor_move_command_crosses_water_cells() {
    let rules = real_ship_test_rules();
    let mut sim = Simulation::new();
    let terrain = water_terrain(4, 4);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    sim.resolved_terrain = Some(terrain.clone());

    sim.terrain_costs.insert(
        crate::rules::locomotor_type::SpeedType::Float,
        crate::sim::pathfinding::terrain_cost::TerrainCostGrid::from_resolved_terrain(
            &terrain,
            crate::rules::locomotor_type::SpeedType::Float,
        ),
    );

    let ship_id = sim
        .spawn_object("DEST", "Americans", 0, 0, 64, &rules, &BTreeMap::new())
        .expect("spawn destroyer");
    let cmd = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::Move {
            entity_id: ship_id,
            target_rx: 3,
            target_ry: 1,
            queue: false,
            group_id: None,
        },
    );

    let _ = sim.advance_tick(
        &[cmd],
        Some(&rules),
        &BTreeMap::new(),
        Some(&path_grid),
        None,
        100,
    );
    for _ in 0..80 {
        let _ = sim.advance_tick(
            &[],
            Some(&rules),
            &BTreeMap::new(),
            Some(&path_grid),
            None,
            100,
        );
    }

    let ship = sim.entities.get(ship_id).expect("ship still exists");
    assert_eq!(
        (ship.position.rx, ship.position.ry),
        (3, 1),
        "real Ship locomotor should complete a simple move command over water"
    );
    assert!(
        ship.movement_target.is_none(),
        "real Ship locomotor should finish its move command"
    );
}

#[test]
fn test_real_ship_locomotor_crosses_water_surface_cells_with_non_water_land_type() {
    let rules = real_ship_test_rules();
    let mut sim = Simulation::new();
    // Real maps contain water-surface tiles that keep is_water=true while carrying
    // shoreline/coast land_type values. Ships should still navigate them.
    let terrain = water_terrain_with_land_type(4, 4, 7, false);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    sim.resolved_terrain = Some(terrain.clone());

    sim.terrain_costs.insert(
        crate::rules::locomotor_type::SpeedType::Float,
        crate::sim::pathfinding::terrain_cost::TerrainCostGrid::from_resolved_terrain(
            &terrain,
            crate::rules::locomotor_type::SpeedType::Float,
        ),
    );

    let ship_id = sim
        .spawn_object("DEST", "Americans", 0, 0, 64, &rules, &BTreeMap::new())
        .expect("spawn destroyer");
    let cmd = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::Move {
            entity_id: ship_id,
            target_rx: 3,
            target_ry: 1,
            queue: false,
            group_id: None,
        },
    );

    let _ = sim.advance_tick(
        &[cmd],
        Some(&rules),
        &BTreeMap::new(),
        Some(&path_grid),
        None,
        100,
    );
    for _ in 0..80 {
        let _ = sim.advance_tick(
            &[],
            Some(&rules),
            &BTreeMap::new(),
            Some(&path_grid),
            None,
            100,
        );
    }

    let ship = sim.entities.get(ship_id).expect("ship still exists");
    assert_eq!(
        (ship.position.rx, ship.position.ry),
        (3, 1),
        "real Ship locomotor should treat water-surface cells as navigable even when land_type is not the pure water column"
    );
    assert!(
        ship.movement_target.is_none(),
        "real Ship locomotor should finish its move command on water-surface cells"
    );
}

#[test]
fn test_real_ship_move_command_can_path_under_bridge_when_too_big() {
    let rules = real_ship_test_rules();
    let mut sim = Simulation::new();
    let mut terrain = water_terrain(5, 3);
    let bridge_idx = terrain.index(2, 1).expect("bridge cell index");
    terrain.cells[bridge_idx].bridge_deck_level = 1;
    terrain.cells[bridge_idx].bridge_walkable = true;
    terrain.cells[bridge_idx].bridge_transition = true;
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    sim.resolved_terrain = Some(terrain.clone());

    sim.terrain_costs.insert(
        crate::rules::locomotor_type::SpeedType::Float,
        crate::sim::pathfinding::terrain_cost::TerrainCostGrid::from_resolved_terrain(
            &terrain,
            crate::rules::locomotor_type::SpeedType::Float,
        ),
    );

    let ship_id = sim
        .spawn_object("DEST", "Americans", 0, 1, 64, &rules, &BTreeMap::new())
        .expect("spawn destroyer");
    let cmd = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::Move {
            entity_id: ship_id,
            target_rx: 4,
            target_ry: 1,
            queue: false,
            group_id: None,
        },
    );

    let _ = sim.advance_tick(
        &[cmd],
        Some(&rules),
        &BTreeMap::new(),
        Some(&path_grid),
        None,
        100,
    );
    let initial_path = sim
        .entities
        .get(ship_id)
        .and_then(|ship| ship.movement_target.as_ref())
        .map(|mt| mt.path.clone())
        .expect("ship should have an initial path");
    for _ in 0..120 {
        let _ = sim.advance_tick(
            &[],
            Some(&rules),
            &BTreeMap::new(),
            Some(&path_grid),
            None,
            100,
        );
    }

    let ship = sim.entities.get(ship_id).expect("ship still exists");
    assert_eq!(
        (ship.position.rx, ship.position.ry),
        (4, 1),
        "Naval ships should still complete move commands when the straight route passes under a bridge"
    );
    assert!(
        initial_path.contains(&(2, 1)),
        "planned path should be allowed to include under-bridge structural cells for naval movers"
    );
}

#[test]
fn test_spawn_multiple_entities() {
    let mut sim: Simulation = Simulation::new();
    let entities: Vec<MapEntity> = vec![
        make_test_entity("MTNK", EntityCategory::Unit),
        make_test_entity("HTNK", EntityCategory::Unit),
        make_test_entity("E1", EntityCategory::Infantry),
        make_test_entity("GAPOWR", EntityCategory::Structure),
    ];
    let count: u32 = sim.spawn_from_map(&entities, None, &empty_heights());
    assert_eq!(count, 4);

    let total: usize = sim.entities.values().count();
    assert_eq!(total, 4);
}

#[test]
fn test_empty_entities_spawns_nothing() {
    let mut sim: Simulation = Simulation::new();
    let count: u32 = sim.spawn_from_map(&[], None, &empty_heights());
    assert_eq!(count, 0);
    assert_eq!(sim.entities.values().count(), 0);
}

#[test]
fn test_stable_ids_are_assigned() {
    let mut sim: Simulation = Simulation::new();
    let entities: Vec<MapEntity> = vec![
        make_test_entity("MTNK", EntityCategory::Unit),
        make_test_entity("E1", EntityCategory::Infantry),
    ];
    sim.spawn_from_map(&entities, None, &empty_heights());

    let mut ids: Vec<u64> = sim.entities.values().map(|e| e.stable_id).collect();
    ids.sort_unstable();
    assert_eq!(ids, vec![1, 2]);
}

#[test]
fn test_select_command_applies_snapshot_selection() {
    let mut sim: Simulation = Simulation::new();
    sim.spawn_from_map(
        &[
            make_test_entity("MTNK", EntityCategory::Unit),
            make_test_entity("E1", EntityCategory::Infantry),
        ],
        None,
        &empty_heights(),
    );

    let select = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::Select {
            entity_ids: vec![2],
            additive: false,
        },
    );
    let _ = sim.advance_tick(&[select], None, &empty_heights(), None, None, 33);

    assert!(!sim.entities.get(1).is_some_and(|e| e.selected));
    assert!(sim.entities.get(2).is_some_and(|e| e.selected));
}

#[test]
fn test_select_command_replaces_previous_selection() {
    let mut sim: Simulation = Simulation::new();
    sim.spawn_from_map(
        &[
            make_test_entity("MTNK", EntityCategory::Unit),
            make_test_entity("E1", EntityCategory::Infantry),
        ],
        None,
        &empty_heights(),
    );

    let cmd1 = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::Select {
            entity_ids: vec![1],
            additive: false,
        },
    );
    let _ = sim.advance_tick(&[cmd1], None, &empty_heights(), None, None, 33);

    let cmd2 = cmd_envelope(
        &sim,
        "Americans",
        2,
        Command::Select {
            entity_ids: vec![2],
            additive: true,
        },
    );
    let _ = sim.advance_tick(&[cmd2], None, &empty_heights(), None, None, 33);

    assert!(!sim.entities.get(1).is_some_and(|e| e.selected));
    assert!(sim.entities.get(2).is_some_and(|e| e.selected));
}

#[test]
fn test_select_command_deduplicates_and_sorts_ids() {
    let mut sim: Simulation = Simulation::new();
    sim.spawn_from_map(
        &[
            make_test_entity("MTNK", EntityCategory::Unit),
            make_test_entity("E1", EntityCategory::Infantry),
        ],
        None,
        &empty_heights(),
    );

    let select = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::Select {
            entity_ids: vec![2, 2, 1],
            additive: false,
        },
    );
    let _ = sim.advance_tick(&[select], None, &empty_heights(), None, None, 33);

    assert!(sim.entities.get(1).is_some_and(|e| e.selected));
    assert!(sim.entities.get(2).is_some_and(|e| e.selected));
}

#[test]
fn test_deploy_mcv_replaces_vehicle_with_conyard() {
    let mut sim = Simulation::new();
    let rules = combat_test_rules();
    let heights = empty_heights();
    let mcv = sim
        .spawn_object("AMCV", "Americans", 20, 22, 64, &rules, &heights)
        .expect("spawn MCV");
    if let Some(e) = sim.entities.get_mut(mcv) {
        e.selected = true;
    }

    let cmd = cmd_envelope(&sim, "Americans", 1, Command::DeployMcv { entity_id: mcv });
    let _ = sim.advance_tick(&[cmd], Some(&rules), &heights, None, None, 33);

    assert!(sim.entities.get(mcv).is_none(), "MCV should be removed");
    let gacnst_id = sim
        .interner
        .get("GACNST")
        .expect("GACNST should be interned");
    assert!(
        sim.entities
            .values()
            .any(|e| e.type_ref == gacnst_id && e.position.rx == 19 && e.position.ry == 21),
        "Construction yard should spawn at gamemd's deploy foundation origin"
    );
}

#[test]
fn test_execute_tick_delay_blocks_early_execution() {
    let mut sim: Simulation = Simulation::new();
    sim.spawn_from_map(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 2,
            cell_y: 2,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: false,
        }],
        None,
        &empty_heights(),
    );
    let grid = PathGrid::new(32, 32);
    let delayed = cmd_envelope(
        &sim,
        "Americans",
        3,
        Command::Move {
            entity_id: 1,
            target_rx: 8,
            target_ry: 2,

            queue: false,
            group_id: None,
        },
    );

    let _ = sim.advance_tick(
        &[delayed.clone()],
        None,
        &empty_heights(),
        Some(&grid),
        None,
        33,
    );
    assert!(
        sim.entities
            .get(1)
            .and_then(|e| e.movement_target.as_ref())
            .is_none()
    );

    let _ = sim.advance_tick(
        &[delayed.clone()],
        None,
        &empty_heights(),
        Some(&grid),
        None,
        33,
    );
    assert!(
        sim.entities
            .get(1)
            .and_then(|e| e.movement_target.as_ref())
            .is_none()
    );

    let _ = sim.advance_tick(&[delayed], None, &empty_heights(), Some(&grid), None, 33);
    assert!(
        sim.entities
            .get(1)
            .and_then(|e| e.movement_target.as_ref())
            .is_some()
    );
}

#[test]
fn test_move_queue_command_appends_waypoint() {
    let mut sim: Simulation = Simulation::new();
    sim.spawn_from_map(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 2,
            cell_y: 2,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: false,
        }],
        None,
        &empty_heights(),
    );
    let grid = PathGrid::new(32, 32);
    let commands = vec![
        cmd_envelope(
            &sim,
            "Americans",
            1,
            Command::Move {
                entity_id: 1,
                target_rx: 8,
                target_ry: 2,
                queue: false,
                group_id: None,
            },
        ),
        cmd_envelope(
            &sim,
            "Americans",
            1,
            Command::Move {
                entity_id: 1,
                target_rx: 12,
                target_ry: 2,
                queue: true,
                group_id: None,
            },
        ),
    ];
    let _ = sim.advance_tick(&commands, None, &empty_heights(), Some(&grid), None, 33);

    let ge = sim
        .entities
        .get(1)
        .expect("entity 1 should exist in EntityStore");
    let movement = ge
        .movement_target
        .as_ref()
        .expect("movement target should be set");
    assert_eq!(movement.path.last().copied(), Some((12, 2)));
}

#[test]
fn test_stop_command_clears_move_and_attack_intent() {
    let mut sim: Simulation = Simulation::new();
    sim.spawn_from_map(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 4,
            cell_y: 4,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: false,
        }],
        None,
        &empty_heights(),
    );

    if let Some(e) = sim.entities.get_mut(1) {
        e.movement_target = Some(MovementTarget {
            path: vec![(4, 4), (5, 4)],
            path_layers: vec![MovementLayer::Ground; 2],
            next_index: 1,
            speed: SimFixed::from_num(1024),
            move_dir_x: SimFixed::from_num(256),
            move_dir_y: SIM_ZERO,
            move_dir_len: SimFixed::from_num(256),
            ..Default::default()
        });
        e.attack_target = Some(AttackTarget::new(1));
    }

    let cmd = cmd_envelope(&sim, "Americans", 1, Command::Stop { entity_id: 1 });
    let _ = sim.advance_tick(&[cmd], None, &empty_heights(), None, None, 33);
    assert!(
        sim.entities.get(1).unwrap().movement_target.is_none(),
        "movement target should be cleared by Stop"
    );
    assert!(
        sim.entities.get(1).unwrap().attack_target.is_none(),
        "AttackTarget should be cleared by Stop command"
    );
}

#[test]
fn test_move_command_rejects_non_owned_entity() {
    let mut sim: Simulation = Simulation::new();
    sim.spawn_from_map(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 2,
            cell_y: 2,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: false,
        }],
        None,
        &empty_heights(),
    );
    let grid = PathGrid::new(32, 32);
    sim.interner.intern("Russians"); // Ensure "Russians" is in sim's interner for cmd_envelope lookup.
    let cmd = cmd_envelope(
        &sim,
        "Russians",
        1,
        Command::Move {
            entity_id: 1,
            target_rx: 8,
            target_ry: 2,

            queue: false,
            group_id: None,
        },
    );

    let _ = sim.advance_tick(&[cmd], None, &empty_heights(), Some(&grid), None, 33);
    assert!(
        sim.entities
            .get(1)
            .is_some_and(|e| e.movement_target.is_none())
    );
}

#[test]
fn test_move_command_chrono_miner_uses_ground_path() {
    let rules = teleport_command_test_rules();
    let mut sim: Simulation = Simulation::new();
    let heights = empty_heights();
    let entity = sim
        .spawn_object("CMIN", "Americans", 2, 2, 64, &rules, &heights)
        .expect("spawn chrono miner");
    let grid = PathGrid::new(32, 32);
    let cmd = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::Move {
            entity_id: entity,
            target_rx: 8,
            target_ry: 2,
            queue: false,
            group_id: None,
        },
    );

    let _ = sim.advance_tick(&[cmd], Some(&rules), &heights, Some(&grid), None, 33);
    assert!(
        sim.entities
            .get(entity)
            .and_then(|e| e.movement_target.as_ref())
            .is_some(),
        "Chrono Miner should path like a ground unit on normal move orders"
    );
    assert!(
        sim.entities
            .get(entity)
            .and_then(|e| e.teleport_state.as_ref())
            .is_none(),
        "Chrono Miner should not enter teleport movement on a normal move order"
    );
}

#[test]
fn test_move_command_non_harvester_teleporter_uses_teleport() {
    let rules = teleport_command_test_rules();
    let mut sim: Simulation = Simulation::new();
    let heights = empty_heights();
    let entity = sim
        .spawn_object("CHRONO", "Americans", 2, 2, 64, &rules, &heights)
        .expect("spawn teleporter");
    let grid = PathGrid::new(32, 32);
    let cmd = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::Move {
            entity_id: entity,
            target_rx: 8,
            target_ry: 2,
            queue: false,
            group_id: None,
        },
    );

    let _ = sim.advance_tick(&[cmd], Some(&rules), &heights, Some(&grid), None, 33);
    assert!(
        sim.entities
            .get(entity)
            .and_then(|e| e.teleport_state.as_ref())
            .is_some(),
        "Non-harvester teleporters should still use teleport movement"
    );
    assert!(
        sim.entities
            .get(entity)
            .is_some_and(|e| e.movement_target.is_none()),
        "Teleport movement should not attach a ground MovementTarget"
    );
}

#[test]
fn test_attack_move_command_chrono_miner_uses_ground_path() {
    let rules = teleport_command_test_rules();
    let mut sim: Simulation = Simulation::new();
    let heights = empty_heights();
    let entity = sim
        .spawn_object("CMIN", "Americans", 2, 2, 64, &rules, &heights)
        .expect("spawn chrono miner");
    let grid = PathGrid::new(32, 32);
    let cmd = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::AttackMove {
            entity_id: entity,
            target_rx: 8,
            target_ry: 2,
            queue: false,
        },
    );

    let _ = sim.advance_tick(&[cmd], Some(&rules), &heights, Some(&grid), None, 33);
    assert!(
        sim.entities
            .get(entity)
            .and_then(|e| e.movement_target.as_ref())
            .is_some(),
        "Chrono Miner should path on attack-move instead of teleporting"
    );
    assert!(
        sim.entities
            .get(entity)
            .is_some_and(|e| e.order_intent.is_some()),
        "Attack-move should still set order intent"
    );
    assert!(
        sim.entities
            .get(entity)
            .and_then(|e| e.teleport_state.as_ref())
            .is_none(),
        "Chrono Miner should not enter teleport movement on attack-move"
    );
}

#[test]
fn test_attack_command_rejects_friendly_target() {
    let mut sim: Simulation = Simulation::new();
    sim.house_alliances = alliance_map(&[
        ("Americans", &["Americans", "British"]),
        ("British", &["Americans", "British"]),
    ]);
    sim.spawn_from_map(
        &[
            MapEntity {
                owner: "Americans".to_string(),
                type_id: "MTNK".to_string(),
                health: 256,
                cell_x: 2,
                cell_y: 2,
                facing: 64,
                category: EntityCategory::Unit,
                sub_cell: 0,
                veterancy: 0,
                high: false,
            },
            MapEntity {
                owner: "British".to_string(),
                type_id: "E1".to_string(),
                health: 256,
                cell_x: 4,
                cell_y: 2,
                facing: 64,
                category: EntityCategory::Infantry,
                sub_cell: 0,
                veterancy: 0,
                high: false,
            },
        ],
        None,
        &empty_heights(),
    );
    let cmd = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::Attack {
            attacker_id: 1,
            target_id: 2,
        },
    );

    let _ = sim.advance_tick(&[cmd], None, &empty_heights(), None, None, 33);
    assert!(
        sim.entities.get(1).unwrap().attack_target.is_none(),
        "Attack on same-owner target should not issue"
    );
}

#[test]
fn test_attack_move_auto_acquires_enemy() {
    let rules = combat_test_rules();
    let mut sim: Simulation = Simulation::new();
    sim.spawn_from_map(
        &[
            MapEntity {
                owner: "Americans".to_string(),
                type_id: "MTNK".to_string(),
                health: 256,
                cell_x: 2,
                cell_y: 2,
                facing: 64,
                category: EntityCategory::Unit,
                sub_cell: 0,
                veterancy: 0,
                high: false,
            },
            MapEntity {
                owner: "Russians".to_string(),
                type_id: "E1".to_string(),
                health: 256,
                cell_x: 4,
                cell_y: 2,
                facing: 64,
                category: EntityCategory::Infantry,
                sub_cell: 0,
                veterancy: 0,
                high: false,
            },
        ],
        None,
        &empty_heights(),
    );
    let grid = PathGrid::new(32, 32);
    let cmd = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::AttackMove {
            entity_id: 1,
            target_rx: 8,
            target_ry: 2,
            queue: false,
        },
    );

    let _ = sim.advance_tick(
        &[cmd],
        Some(&rules),
        &empty_heights(),
        Some(&grid),
        None,
        100,
    );
    let attack = sim
        .entities
        .get(1)
        .unwrap()
        .attack_target
        .as_ref()
        .expect("attack-move should acquire target");
    assert!(matches!(
        attack.target,
        crate::sim::combat::TargetKind::Entity(2)
    ));
    assert!(sim.entities.get(1).unwrap().order_intent.is_some());
}

#[test]
fn test_attack_move_resumes_after_kill() {
    let rules = combat_test_rules();
    let mut sim: Simulation = Simulation::new();
    sim.spawn_from_map(
        &[
            MapEntity {
                owner: "Americans".to_string(),
                type_id: "MTNK".to_string(),
                health: 256,
                cell_x: 2,
                cell_y: 2,
                facing: 64,
                category: EntityCategory::Unit,
                sub_cell: 0,
                veterancy: 0,
                high: false,
            },
            MapEntity {
                owner: "Russians".to_string(),
                type_id: "E1".to_string(),
                health: 256,
                cell_x: 4,
                cell_y: 2,
                facing: 64,
                category: EntityCategory::Infantry,
                sub_cell: 0,
                veterancy: 0,
                high: false,
            },
        ],
        None,
        &empty_heights(),
    );
    if let Some(e) = sim.entities.get_mut(2) {
        e.health.current = 50;
        e.health.max = 50;
    }
    let grid = PathGrid::new(32, 32);
    let cmd = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::AttackMove {
            entity_id: 1,
            target_rx: 8,
            target_ry: 2,
            queue: false,
        },
    );

    let _ = sim.advance_tick(
        &[cmd],
        Some(&rules),
        &empty_heights(),
        Some(&grid),
        None,
        100,
    );
    assert!(
        sim.entities.get(1).unwrap().attack_target.is_none(),
        "target should die and attack should clear"
    );
    let ge = sim.entities.get(1).expect("entity 1 should exist");
    let movement = ge
        .movement_target
        .as_ref()
        .expect("attack-move should resume movement after kill");
    assert_eq!(movement.path.last().copied(), Some((8, 2)));
}

#[test]
fn test_guard_returns_to_anchor_when_displaced() {
    let rules = combat_test_rules();
    let mut sim: Simulation = Simulation::new();
    sim.spawn_from_map(
        &[MapEntity {
            owner: "Americans".to_string(),
            type_id: "MTNK".to_string(),
            health: 256,
            cell_x: 2,
            cell_y: 2,
            facing: 64,
            category: EntityCategory::Unit,
            sub_cell: 0,
            veterancy: 0,
            high: false,
        }],
        None,
        &empty_heights(),
    );
    let guard_cmd = cmd_envelope(
        &sim,
        "Americans",
        1,
        Command::Guard {
            entity_id: 1,
            target_id: None,
        },
    );
    let grid = PathGrid::new(32, 32);
    let _ = sim.advance_tick(
        &[guard_cmd],
        Some(&rules),
        &empty_heights(),
        Some(&grid),
        None,
        100,
    );

    if let Some(e) = sim.entities.get_mut(1) {
        e.position.rx = 5;
        e.position.ry = 2;
        let (sx, sy) = terrain::iso_to_screen(5, 2, e.position.z);
        e.position.screen_x = sx;
        e.position.screen_y = sy;
        e.movement_target = None;
        e.attack_target = None;
    }

    let _ = sim.advance_tick(&[], Some(&rules), &empty_heights(), Some(&grid), None, 100);
    let ge = sim.entities.get(1).expect("entity 1 should exist");
    let movement = ge
        .movement_target
        .as_ref()
        .expect("guard should re-path back to its anchor");
    assert_eq!(movement.path.last().copied(), Some((2, 2)));
}

#[test]
fn test_fog_revealed_persists_after_unit_moves_away() {
    let mut sim = Simulation::new();
    let (sx, sy) = terrain::iso_to_screen(1, 1, 0);
    use crate::sim::game_entity::GameEntity;
    let americans_id = sim.interner.intern("Americans");
    let e1_id = sim.interner.intern("E1");
    let ge = GameEntity::new(
        1,
        1,
        1,
        0,
        0,
        americans_id,
        crate::sim::components::Health {
            current: 100,
            max: 100,
        },
        e1_id,
        EntityCategory::Infantry,
        0,
        0,
        false,
    );
    sim.entities.insert(ge);

    let grid = PathGrid::new(8, 8);
    let americans = sim.interner.get("Americans").expect("Americans interned");
    let _ = sim.advance_tick(&[], None, &empty_heights(), Some(&grid), None, 33);
    assert!(sim.fog.is_cell_visible(americans, 1, 1));
    assert!(sim.fog.is_cell_revealed(americans, 1, 1));

    let _ = (sx, sy); // suppress unused warning
    if let Some(e) = sim.entities.get_mut(1) {
        e.position.rx = 2;
        e.position.ry = 1;
        let (nx, ny) = terrain::iso_to_screen(2, 1, 0);
        e.position.screen_x = nx;
        e.position.screen_y = ny;
    }
    let _ = sim.advance_tick(&[], None, &empty_heights(), Some(&grid), None, 33);
    assert!(!sim.fog.is_cell_visible(americans, 1, 1));
    assert!(sim.fog.is_cell_revealed(americans, 1, 1));
    assert!(sim.fog.is_cell_visible(americans, 2, 1));
}

#[test]
fn test_undeploy_conyard_spawns_mcv() {
    let mut sim = Simulation::new();
    let rules = combat_test_rules();
    let heights = empty_heights();

    // First deploy an MCV to get a ConYard.
    let mcv = sim
        .spawn_object("AMCV", "Americans", 20, 22, 64, &rules, &heights)
        .expect("spawn MCV");
    if let Some(e) = sim.entities.get_mut(mcv) {
        e.selected = true;
    }
    let deploy_cmd = cmd_envelope(&sim, "Americans", 1, Command::DeployMcv { entity_id: mcv });
    let _ = sim.advance_tick(&[deploy_cmd], Some(&rules), &heights, None, None, 33);

    // Find the ConYard that was spawned.
    let yard_id: u64 = sim
        .entities
        .values()
        .find(|e| sim.interner.resolve(e.type_ref) == "GACNST")
        .map(|e| e.stable_id)
        .expect("ConYard should exist after deploy");

    // Clear building_up so we can undeploy (can't undeploy during construction).
    if let Some(e) = sim.entities.get_mut(yard_id) {
        e.building_up = None;
        e.selected = true;
    }

    // Undeploy the ConYard — starts a 30-tick reverse build-up animation.
    let undeploy_cmd = cmd_envelope(
        &sim,
        "Americans",
        2,
        Command::UndeployBuilding { entity_id: yard_id },
    );
    let _ = sim.advance_tick(&[undeploy_cmd], Some(&rules), &heights, None, None, 33);

    // ConYard should still exist but have building_down set.
    assert!(
        sim.entities.get(yard_id).is_some(),
        "ConYard should still exist during undeploy animation"
    );
    assert!(
        sim.entities.get(yard_id).unwrap().building_down.is_some(),
        "ConYard should have building_down component"
    );

    // Advance through the 30-tick undeploy animation.
    for _tick in 3..33 {
        let _ = sim.advance_tick(&[], Some(&rules), &heights, None, None, 33);
    }

    // ConYard should be gone after animation completes.
    assert!(
        sim.entities.get(yard_id).is_none(),
        "ConYard should be removed after undeploy animation"
    );

    // MCV should be spawned at center of old foundation (4x3 → center offset 2,1).
    let amcv_id = sim.interner.get("AMCV").expect("AMCV should be interned");
    let mcvs: Vec<(u16, u16, bool)> = sim
        .entities
        .values()
        .filter(|e| e.type_ref == amcv_id)
        .map(|e| (e.position.rx, e.position.ry, e.selected))
        .collect();
    assert_eq!(mcvs.len(), 1, "Exactly one MCV should exist after undeploy");
    let (rx, ry, selected) = &mcvs[0];
    // Origin was (19, 21) from deploy, foundation 4x3, center = (19+2, 21+1) = (21, 22).
    assert_eq!(*rx, 21, "MCV should spawn at foundation center X");
    assert_eq!(*ry, 22, "MCV should spawn at foundation center Y");
    assert!(*selected, "MCV should inherit selection from ConYard");
}

#[test]
fn refresh_vision_heights_copies_path_cell_ground_levels() {
    use crate::map::resolved_terrain::ResolvedTerrainCell;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};

    // Build a 3x3 terrain with one elevated cell at (1,1), level=4.
    let width: u16 = 3;
    let height: u16 = 3;
    let mut cells = Vec::with_capacity((width as usize) * (height as usize));
    for y in 0..height {
        for x in 0..width {
            let level: u8 = if x == 1 && y == 1 { 4 } else { 0 };
            cells.push(ResolvedTerrainCell {
                rx: x,
                ry: y,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                is_wood_bridge_repair_tile: false,
                level,
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
                is_cliff_redraw: false,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: false,
                terrain_object_blocks: false,
                overlay_blocks: false,
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
    let terrain = ResolvedTerrainGrid::from_cells(width, height, cells);
    let grid = PathGrid::from_resolved_terrain(&terrain);

    let mut sim = Simulation::new();
    assert!(
        sim.vision_height_grid.is_none(),
        "fresh sim should have no height grid"
    );

    sim.refresh_vision_heights(&grid);

    let heights = sim
        .vision_height_grid
        .as_ref()
        .expect("refresh_vision_heights must populate the grid");
    assert_eq!(heights.len(), (width as usize) * (height as usize));

    // Index = ry * width + rx; the elevated cell at (1,1) must report level 4.
    let idx = (1usize) * (width as usize) + 1usize;
    assert_eq!(heights[idx], 4, "elevated cell should report level 4");

    // Flat cells stay at 0.
    assert_eq!(heights[0], 0, "(0,0) is flat");
    assert_eq!(
        heights[(width as usize) * (height as usize) - 1],
        0,
        "(2,2) is flat"
    );
}

/// Phase D Task 16: render integration regression — the body builder must
/// query the bridge atlas with the SHP frame derived from the cell's
/// **post-tick** `damage_state` (NOT a stale overlay byte). If a future
/// refactor accidentally has the body builder read from the wrong source
/// (e.g. legacy `OverlayGrid`), bridges would visually stay healthy after
/// they collapse — and nothing else would catch it.
///
/// Approach: seed an EW Damaged cell directly via `test_seed_cell`, run
/// the inner builder against a mock `BridgeAtlasLookup` that only returns
/// `Some` for the EXPECTED `(name, frame)` pair (`BRIDGE1`, frame 15 - the
/// EW Damaged SHP frame). If the builder queried with anything else, the
/// mock returns `None`, no `SpriteInstance` is emitted, and the assertion
/// fires.
#[test]
fn bridge_body_builder_queries_atlas_with_post_tick_state_byte_frame() {
    use crate::app_instances::bridges::build_bridge_body_instances_inner;
    use crate::map::lighting::CellLightGrid;
    use crate::render::bridge_atlas::BridgeAtlasLookup;
    use crate::render::overlay_atlas::OverlaySpriteEntry;
    use crate::sim::bridge_state::{
        Axis, BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState, DamageState,
    };

    struct MockAtlas {
        expected_name: String,
        expected_frame: u8,
        entry: OverlaySpriteEntry,
        queries: std::cell::RefCell<Vec<(String, u8)>>,
    }
    impl BridgeAtlasLookup for MockAtlas {
        fn body_entry(&self, name: &str, frame: u8) -> Option<&OverlaySpriteEntry> {
            self.queries.borrow_mut().push((name.to_string(), frame));
            if name == self.expected_name && frame == self.expected_frame {
                Some(&self.entry)
            } else {
                None
            }
        }
    }

    // Build a 3-cell EW bridge strip via the existing fixture. Cells at
    // (4,5), (5,5), (6,5) are seeded Healthy with overlay 0xDC, axis EW.
    let (_resolved, mut bridge_state) = ew_high_bridge_strip_for_dispatch(5, 5, 4, false, 0);

    // Force the center cell to Damaged directly — Task 16 is about the
    // sim → render bridge, not about which damage path produced Damaged.
    bridge_state.test_seed_cell(
        5,
        5,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Damaged,
            axis: Some(Axis::EW),
            role: BridgeCellRole::Body,
            anchor_span_id: Some(1),
            overlay_byte: 0xDC,
            damaged_variant: false,
            bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
        },
    );

    // Mock atlas accepts only ("BRIDGE1", frame 15) - the EW Damaged SHP frame.
    // Per BRIDGE_RENDERING_GHIDRA_REPORT.md §12, EW body is SHP frames 0..=8
    // (axis_base=0); Damaged adds local offset 6.
    let mock = MockAtlas {
        expected_name: "BRIDGE1".to_string(),
        expected_frame: 15,
        entry: OverlaySpriteEntry {
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            pixel_size: [60.0, 30.0],
            offset_x: 0.0,
            offset_y: 0.0,
        },
        queries: std::cell::RefCell::new(Vec::new()),
    };

    // Map every overlay byte the walker may have written to "BRIDGE1" so
    // `is_high_bridge_body_name` accepts it.
    let mut overlay_names: BTreeMap<u8, String> = BTreeMap::new();
    for byte in 0xCDu8..=0xE8u8 {
        overlay_names.insert(byte, "BRIDGE1".to_string());
    }

    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let lighting_grid = CellLightGrid::new();

    // Camera centered on cell (5, 5) so view culling doesn't reject it.
    let (cam_target_x, cam_target_y) = crate::map::terrain::iso_to_screen(5, 5, 4);

    let mut out: Vec<crate::render::batch::SpriteInstance> = Vec::new();
    build_bridge_body_instances_inner(
        &bridge_state,
        &mock,
        &overlay_names,
        &height_map,
        &lighting_grid,
        /* origin_y */ 0.0,
        /* world_height */ 1.0,
        /* cam_x */ cam_target_x - 400.0,
        /* cam_y */ cam_target_y - 300.0,
        /* sw */ 800.0,
        /* sh */ 600.0,
        &mut out,
    );

    let queries = mock.queries.borrow();
    assert!(
        !queries.is_empty(),
        "body builder must query the atlas at least once for the seeded Damaged cell"
    );
    let queried_for_55 = queries
        .iter()
        .any(|(name, frame)| name == "BRIDGE1" && *frame == 15);
    assert!(
        queried_for_55,
        "body builder must query atlas with (\"BRIDGE1\", 15) - the EW Damaged SHP frame; \
         actual queries: {:?}",
        *queries
    );
    assert!(
        !out.is_empty(),
        "expected at least one SpriteInstance for the Damaged EW bridge cell"
    );
}

// --- G7 bridgehead registration: cross-rebuild + A* invariants ---

/// 5x1 high-bridge fixture with realistic bridgehead semantics:
/// ground(h=4) → bridgehead → body(water, deck=4) → bridgehead → ground(h=4).
/// Used by the two G7 invariant tests below.
fn make_realistic_bridgehead_terrain() -> ResolvedTerrainGrid {
    use crate::map::resolved_terrain::ResolvedTerrainCell;
    let cells = vec![
        ResolvedTerrainCell {
            level: 4,
            ..bridgehead_base_cell(0, 0)
        },
        ResolvedTerrainCell {
            bridge_walkable: true,
            bridge_transition: true,
            bridge_deck_level: 4,
            has_bridge_deck: true,
            ..bridgehead_base_cell(1, 0)
        },
        ResolvedTerrainCell {
            ground_walk_blocked: true,
            build_blocked: true,
            base_build_blocked: true,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: Default::default(),
            base_speed_costs: Default::default(),
            bridge_walkable: true,
            bridge_transition: true,
            bridge_deck_level: 4,
            has_bridge_deck: true,
            is_water: true,
            ..bridgehead_base_cell(2, 0)
        },
        ResolvedTerrainCell {
            bridge_walkable: true,
            bridge_transition: true,
            bridge_deck_level: 4,
            has_bridge_deck: true,
            ..bridgehead_base_cell(3, 0)
        },
        ResolvedTerrainCell {
            level: 4,
            ..bridgehead_base_cell(4, 0)
        },
    ];
    ResolvedTerrainGrid::from_cells(5, 1, cells)
}

fn bridgehead_base_cell(rx: u16, ry: u16) -> crate::map::resolved_terrain::ResolvedTerrainCell {
    use crate::map::resolved_terrain::ResolvedTerrainCell;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
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
        render_offset_x: 0,
        render_offset_y: 0,
        terrain_class: TerrainClass::Clear,
        speed_costs: SpeedCostProfile::default(),
        is_water: false,
        is_cliff_like: false,
        is_cliff_redraw: false,
        variant: 0,
        is_rough: false,
        is_road: false,
        accepts_smudge: false,
        has_ramp: false,
        canonical_ramp: None,
        ground_walk_blocked: false,
        terrain_object_blocks: false,
        overlay_blocks: false,
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
    }
}

#[test]
fn test_bridgehead_walkability_invariant_across_non_bridge_rebuild_triggers() {
    // In production app_sim_tick fires `rebuild_dynamic_path_grid` on each of
    // `destroyed_structure | ownership_changed | spawned_entities` events.
    // The rebuild is just `PathGrid::from_resolved_terrain_with_bridges(...)`.
    // Calling it N times models N rebuild triggers; bridgehead walkability
    // must hold across every rebuild.
    let mut sim = Simulation::new();
    let terrain = make_realistic_bridgehead_terrain();
    sim.resolved_terrain = Some(terrain.clone());
    sim.bridge_state = Some(BridgeRuntimeState::from_resolved_terrain(
        &terrain, true, 10,
    ));

    for trigger_idx in 0..3 {
        let grid = PathGrid::from_resolved_terrain_with_bridges(
            sim.resolved_terrain.as_ref().unwrap(),
            sim.bridge_state.as_ref(),
        );
        for rx in [1u16, 3] {
            let pc = grid
                .cell(rx, 0)
                .expect("bridgehead cell exists in path grid");
            assert!(
                pc.bridge_walkable,
                "bridgehead ({rx},0) lost bridge_walkable on rebuild #{trigger_idx}"
            );
            assert!(
                pc.transition,
                "bridgehead ({rx},0) lost transition on rebuild #{trigger_idx}"
            );
        }
    }
}

#[test]
fn test_layered_astar_can_traverse_bridge_after_unrelated_rebuild() {
    // Build a sim with the realistic bridgehead fixture. Find an A* layered
    // path Ground(0,0) → Bridge(1,0)..(3,0) → Ground(4,0). Then rebuild the
    // PathGrid (simulating an unrelated event like a building dying somewhere
    // off-bridge) and re-find the same path. PRE-G7 this would fail on the
    // second find: rebuild flips bridgehead bridge_walkable false → A* can't
    // enter the bridge layer. POST-G7 both finds succeed.
    let mut sim = Simulation::new();
    let terrain = make_realistic_bridgehead_terrain();
    sim.resolved_terrain = Some(terrain.clone());
    sim.bridge_state = Some(BridgeRuntimeState::from_resolved_terrain(
        &terrain, true, 10,
    ));

    let grid_initial = PathGrid::from_resolved_terrain_with_bridges(
        sim.resolved_terrain.as_ref().unwrap(),
        sim.bridge_state.as_ref(),
    );
    let path_initial = crate::sim::pathfinding::find_layered_path(
        &grid_initial,
        None,
        None,
        (0, 0),
        MovementLayer::Ground,
        (4, 0),
        None,
        None,
        None,
        0,
        false,
    );
    assert!(
        path_initial.is_some(),
        "intact bridge must allow Ground→Bridge→Ground A* path"
    );

    // Simulate the rebuild_dynamic_path_grid path that fires on every
    // unrelated structure death / unit spawn / ownership change.
    let grid_after_rebuild = PathGrid::from_resolved_terrain_with_bridges(
        sim.resolved_terrain.as_ref().unwrap(),
        sim.bridge_state.as_ref(),
    );
    let path_after_rebuild = crate::sim::pathfinding::find_layered_path(
        &grid_after_rebuild,
        None,
        None,
        (0, 0),
        MovementLayer::Ground,
        (4, 0),
        None,
        None,
        None,
        0,
        false,
    );
    assert!(
        path_after_rebuild.is_some(),
        "A* path must still exist after an unrelated rebuild (G7: bridgeheads \
         must keep bridge_walkable across PathGrid refresh)"
    );
}
