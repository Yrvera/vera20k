use super::*;
use std::collections::HashMap;

use crate::map::actions::MapAction;
use crate::map::entities::EntityCategory;
use crate::map::events::MapEvent;
use crate::map::trigger_graph::build_trigger_graph;
use crate::map::triggers::{MapTrigger, TriggerDifficulty};
use crate::map::variable_names::{LocalVariable, LocalVariableMap};
use crate::sim::game_entity::GameEntity;
use crate::sim::projectile::{
    ProjectileCollisionPolicy, ProjectileCoord, ProjectilePayload, ProjectileSpawn,
    ProjectileTarget, TargetExpiryPolicy,
};
use crate::sim::replay::{ReplayHeader, ReplayLog, ReplayRunner};
use crate::sim::snapshot::GameSnapshot;
use crate::sim::world::{MasterFrameTestRung, Simulation, TickLane, TriggerInputs};
use std::collections::BTreeMap;

fn flat_trigger_playfield_terrain(
    width: u16,
    height: u16,
) -> crate::map::resolved_terrain::ResolvedTerrainGrid {
    use crate::map::resolved_terrain::{ResolvedTerrainCell, zone_class};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};

    let prototype = ResolvedTerrainCell {
        rx: 0,
        ry: 0,
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
        accepts_smudge: false,
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
        zone_type: zone_class::GROUND,
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
        bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
        tube_index: None,
        radar_left: [0; 3],
        radar_right: [0; 3],
        has_damaged_data: false,
        bridgehead_anchor_class_at_load: None,
    };
    let cells = (0..height)
        .flat_map(|ry| {
            let prototype = prototype.clone();
            (0..width).map(move |rx| {
                let mut cell = prototype.clone();
                cell.rx = rx;
                cell.ry = ry;
                cell
            })
        })
        .collect();
    crate::map::resolved_terrain::ResolvedTerrainGrid::from_cells(width, height, cells)
}

fn make_trigger(
    id: &str,
    linked_trigger_id: Option<&str>,
    name: &str,
    enabled: bool,
    repeating: bool,
) -> MapTrigger {
    MapTrigger {
        id: id.to_string(),
        fields: vec![
            "Neutral".to_string(),
            linked_trigger_id.unwrap_or("<none>").to_string(),
            name.to_string(),
            if enabled { "1" } else { "0" }.to_string(),
            "1".to_string(),
            "1".to_string(),
            "1".to_string(),
            if repeating { "2" } else { "0" }.to_string(),
        ],
        owner: Some("Neutral".to_string()),
        linked_trigger_id: linked_trigger_id.map(|value| value.to_ascii_uppercase()),
        name: Some(name.to_string()),
        enabled,
        difficulty: TriggerDifficulty {
            easy: true,
            medium: true,
            hard: true,
        },
        repeating,
    }
}

fn spawn_type(sim: &mut Simulation, type_id: &str) -> u64 {
    let sid = sim.allocate_stable_id();
    let owner_id = sim.interner.intern("Neutral");
    let type_id_interned = sim.interner.intern(type_id);
    let ge = GameEntity::new_at_frame_zero_for_test(
        sid,
        0,
        0,
        0,
        0,
        owner_id,
        crate::sim::components::Health {
            current: 100,
            max: 100,
        },
        type_id_interned,
        EntityCategory::Unit,
        0,
        5,
        false,
    );
    sim.substrate.entities.insert(ge);
    sid
}

#[test]
fn change_visible_map_area_uses_native_parameter_indices_and_atoi() {
    let fields = vec![
        "991".to_string(),
        "-774".to_string(),
        "2cells".to_string(),
        "41".to_string(),
        "junk".to_string(),
        "+38tail".to_string(),
        "A".to_string(),
    ];
    assert_eq!(parse_visible_map_area(&fields), Some([2, 41, 0, 38]));
    assert_eq!(parse_visible_map_area(&fields[..5]), None);
}

#[test]
fn trigger_action_40_normalizes_and_refreshes_authority_same_frame() {
    use crate::map::map_file::MapHeader;
    use crate::map::playfield::PlayfieldBounds;
    use crate::map::resolved_terrain::zone_class;
    use crate::rules::locomotor_type::MovementZone;
    use crate::sim::movement::locomotor::MovementLayer;
    use crate::sim::pathfinding::PathGrid;
    use crate::sim::pathfinding::zone_map::ZONE_INVALID;

    let triggers: TriggerMap = [(
        "AREA".to_string(),
        make_trigger("AREA", None, "Change visible map area", true, false),
    )]
    .into_iter()
    .collect();
    let events: EventMap = [(
        "AREA".to_string(),
        MapEvent {
            id: "AREA".to_string(),
            fields: vec![],
            conditions: vec![EventCondition {
                kind: 47,
                params: vec!["0".to_string(), "0".to_string()],
            }],
        },
    )]
    .into_iter()
    .collect();
    // Stock all06umd-shaped payload: 40,0,0,2,41,56,38,A. With params equal
    // to chunk[1..], ParamType/Param3 are the first two zeroes and LocalSize
    // is exactly params[2..=5].
    let actions: ActionMap = [(
        "AREA".to_string(),
        MapAction {
            id: "AREA".to_string(),
            fields: vec![],
            entries: vec![ActionEntry {
                kind: 40,
                params: ["0", "0", "2", "41", "56", "38", "A"]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            }],
        },
    )]
    .into_iter()
    .collect();
    let graph = build_trigger_graph(
        &HashMap::new(),
        &HashMap::new(),
        &triggers,
        &events,
        &actions,
    );
    let header = MapHeader {
        theater: "TEMPERATE".to_string(),
        fill: "Clear".to_string(),
        level: 0,
        width: 80,
        height: 58,
        local_left: 2,
        local_top: 4,
        local_width: 76,
        local_height: 48,
    };
    let mut sim = Simulation::new();
    sim.install_playfield_from_map_header(&header);
    let initial_bounds = sim.playfield_bounds.expect("initial playfield");
    let mut terrain = flat_trigger_playfield_terrain(100, 100);
    terrain.recalc_playfield_attributes(initial_bounds);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    sim.resolved_terrain = Some(terrain);
    sim.rebuild_zone_grid(&path_grid);
    sim.trigger_runtime = TriggerRuntime::from_map(&triggers, &HashMap::new());

    let before_zone = sim
        .zone_grid
        .as_ref()
        .and_then(|zones| zones.map_for(MovementZone::Normal))
        .expect("normal zone")
        .zone_at(50, 50, MovementLayer::Ground);
    assert_ne!(before_zone, ZONE_INVALID);
    assert!(
        !sim.resolved_terrain
            .as_ref()
            .unwrap()
            .cell(50, 50)
            .unwrap()
            .outside_playfield
    );

    let tick = sim.advance_master_frame(
        &[],
        None,
        &BTreeMap::new(),
        None,
        None,
        67,
        TickLane::Ordinary,
        Some(TriggerInputs {
            graph: &graph,
            triggers: &triggers,
            events: &events,
            actions: &actions,
            rules: None,
        }),
    );

    assert_eq!(
        sim.playfield_bounds,
        Some(PlayfieldBounds {
            base: 80,
            off_fc: 2,
            off_100: 41,
            off_104: 56,
            // Raw 38 clips to Size bottom 17, then the six-cell margin caps 11.
            off_108: 11,
        })
    );
    let terrain = sim.resolved_terrain.as_ref().unwrap();
    assert!(terrain.cell(50, 50).unwrap().outside_playfield);
    assert_eq!(terrain.cell(50, 50).unwrap().zone_type, zone_class::OUTSIDE);
    assert!(!terrain.cell(85, 85).unwrap().outside_playfield);
    assert_eq!(terrain.cell(85, 85).unwrap().zone_type, zone_class::GROUND);
    let zones = sim.zone_grid.as_ref().expect("zones rebuilt");
    let normal = zones.map_for(MovementZone::Normal).expect("normal zone");
    assert_eq!(normal.zone_at(50, 50, MovementLayer::Ground), ZONE_INVALID);
    assert_ne!(normal.zone_at(85, 85, MovementLayer::Ground), ZONE_INVALID);
    assert!(sim.radar_terrain_dirty_cells.is_empty());
    assert_eq!(sim.radar_terrain_dirty_generation, 0);
    assert_eq!(sim.playfield_revision, 1);
    assert_eq!(tick.state_hash, sim.state_hash());

    // FUN_006E21E0 rebuilds radar surfaces for every firing. A second writer
    // must not disappear behind the persistent per-cell dirty-list dedup gate.
    assert!(sim.change_visible_map_area([4, 40, 54, 12], None));
    assert_eq!(sim.playfield_revision, 2);
    assert!(sim.change_visible_map_area([4, 40, 54, 12], None));
    assert_eq!(sim.playfield_revision, 3);
    assert!(sim.radar_terrain_dirty_cells.is_empty());

    let bytes = GameSnapshot::save(&sim, 0, 0, "all06umd.map", 0);
    let restored = GameSnapshot::load(&bytes)
        .expect("action-40 snapshot loads")
        .sim;
    assert_eq!(restored.playfield_bounds, sim.playfield_bounds);
    assert_eq!(restored.playfield_size_height, sim.playfield_size_height);
    assert_eq!(restored.playfield_revision, sim.playfield_revision);
}

#[test]
fn techno_playfield_action_40_exact_recompute_and_mobile_reveal_callback() {
    use crate::map::map_file::MapHeader;
    use crate::map::playfield::PlayfieldBounds;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::sim::components::Health;
    use crate::sim::house_state::HouseState;

    let header = MapHeader {
        theater: "TEMPERATE".to_string(),
        fill: "Clear".to_string(),
        level: 0,
        width: 80,
        height: 58,
        local_left: 30,
        local_top: 20,
        local_width: 8,
        local_height: 6,
    };
    let mut sim = Simulation::new();
    sim.install_playfield_from_map_header(&header);
    let initial = sim.playfield_bounds.unwrap();
    let expanded = PlayfieldBounds::from_raw_local_size(80, 58, [2, 2, 76, 48]);
    let candidates: Vec<(u16, u16)> = (0u16..100)
        .flat_map(|ry| (0u16..100).map(move |rx| (rx, ry)))
        .filter(|&(rx, ry)| {
            expanded.contains_height_aware_packed(rx.into(), ry.into(), 0, 0)
                && !initial.contains_height_aware_packed(rx.into(), ry.into(), 0, 0)
        })
        .collect();
    let mobile_cell = candidates
        .get(candidates.len() / 2)
        .copied()
        .expect("expansion adds cells");
    let building_cell = candidates
        .iter()
        .copied()
        .max_by_key(|&(rx, ry)| {
            i32::from(rx).abs_diff(i32::from(mobile_cell.0))
                + i32::from(ry).abs_diff(i32::from(mobile_cell.1))
        })
        .expect("distant expansion cell");
    assert!(mobile_cell.0.abs_diff(building_cell.0) + mobile_cell.1.abs_diff(building_cell.1) > 10);
    let veteran_probe = [(3i32, 0i32), (-3, 0), (0, 3), (0, -3)]
        .into_iter()
        .find_map(|(dx, dy)| {
            let rx = i32::from(mobile_cell.0) + dx;
            let ry = i32::from(mobile_cell.1) + dy;
            (rx >= 0
                && ry >= 0
                && rx < 100
                && ry < 100
                && expanded.contains_height_aware_packed(rx, ry, 0, 0))
            .then_some((rx as u16, ry as u16))
        })
        .expect("effective veteran sight probe");
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[General]\nVeteranSight=3\nRevealByHeight=no\n",
    ))
    .expect("effective-sight action fixture");

    let owner = sim.interner.intern("Americans");
    sim.houses
        .insert(owner, HouseState::new(owner, 0, Some(owner), true, 0, 10));
    sim.fog.width = 100;
    sim.fog.height = 100;
    let mobile_type = sim.interner.intern("MTNK");
    let building_type = sim.interner.intern("GAPOWR");
    let mut mobile = GameEntity::new_at_frame_zero_for_test(
        1,
        mobile_cell.0,
        mobile_cell.1,
        0,
        0,
        owner,
        Health {
            current: 100,
            max: 100,
        },
        mobile_type,
        EntityCategory::Unit,
        100,
        1,
        true,
    );
    mobile.lifecycle.object_alive = true;
    mobile.lifecycle.in_limbo = false;
    let mut building = GameEntity::new_at_frame_zero_for_test(
        2,
        building_cell.0,
        building_cell.1,
        0,
        0,
        owner,
        Health {
            current: 100,
            max: 100,
        },
        building_type,
        EntityCategory::Structure,
        0,
        1,
        false,
    );
    building.lifecycle.object_alive = true;
    building.lifecycle.in_limbo = false;
    sim.substrate.entities.insert(mobile);
    sim.substrate.entities.insert(building);
    sim.resolved_terrain = Some(flat_trigger_playfield_terrain(100, 100));

    assert!(sim.change_visible_map_area([2, 2, 76, 48], Some(&rules)));
    assert!(sim.substrate.entities.get(1).unwrap().in_playfield);
    assert!(sim.substrate.entities.get(2).unwrap().in_playfield);
    assert!(
        sim.fog
            .is_cell_revealed(owner, mobile_cell.0, mobile_cell.1)
    );
    assert!(
        sim.fog
            .is_cell_revealed(owner, veteran_probe.0, veteran_probe.1),
        "0x0070ADC0 callback must use the shared veteran-adjusted sight, not raw VisionRange"
    );
    assert!(
        !sim.fog
            .is_cell_revealed(owner, building_cell.0, building_cell.1),
        "FUN_006E21E0 excludes BuildingClass from the false-to-true callback"
    );

    assert!(sim.change_visible_map_area([30, 20, 8, 6], Some(&rules)));
    assert!(!sim.substrate.entities.get(1).unwrap().in_playfield);
    assert!(!sim.substrate.entities.get(2).unwrap().in_playfield);
    assert!(sim.change_visible_map_area([2, 2, 76, 48], Some(&rules)));
    assert!(sim.substrate.entities.get(1).unwrap().in_playfield);
    assert!(sim.substrate.entities.get(2).unwrap().in_playfield);
}

#[test]
fn time_trigger_can_center_camera_at_waypoint() {
    let triggers: TriggerMap = [(
        "TRIG_A".to_string(),
        make_trigger("TRIG_A", None, "Timer A", true, false),
    )]
    .into_iter()
    .collect();
    let events: EventMap = [(
        "TRIG_A".to_string(),
        MapEvent {
            id: "TRIG_A".to_string(),
            fields: vec![
                "1".to_string(),
                "47".to_string(),
                "3".to_string(),
                "0".to_string(),
            ],
            conditions: vec![EventCondition {
                kind: 47,
                params: vec!["3".to_string(), "0".to_string()],
            }],
        },
    )]
    .into_iter()
    .collect();
    let actions: ActionMap = [(
        "TRIG_A".to_string(),
        MapAction {
            id: "TRIG_A".to_string(),
            fields: vec![
                "1".to_string(),
                "112".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "9".to_string(),
            ],
            entries: vec![ActionEntry {
                kind: 112,
                params: vec![
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "9".to_string(),
                ],
            }],
        },
    )]
    .into_iter()
    .collect();
    let graph = build_trigger_graph(
        &HashMap::new(),
        &HashMap::new(),
        &triggers,
        &events,
        &actions,
    );
    let mut runtime = TriggerRuntime::from_map(&triggers, &HashMap::new());

    assert!(
        runtime
            .advance_at_frame(44, &graph, &triggers, &events, &actions, None, None)
            .is_empty()
    );
    assert_eq!(
        runtime.advance_at_frame(45, &graph, &triggers, &events, &actions, None, None),
        vec![TriggerEffect::CenterCameraAtWaypoint {
            waypoint: 9,
            immediate: true,
        }]
    );
    assert!(
        runtime
            .advance_at_frame(46, &graph, &triggers, &events, &actions, None, None)
            .is_empty()
    );
}

#[test]
fn master_frame_polls_triggers_before_logic_houses_commit_and_delete() {
    let triggers: TriggerMap = [(
        "TRIG_A".to_string(),
        make_trigger("TRIG_A", None, "Frame Zero", true, false),
    )]
    .into_iter()
    .collect();
    let events: EventMap = [(
        "TRIG_A".to_string(),
        MapEvent {
            id: "TRIG_A".to_string(),
            fields: vec![],
            conditions: vec![EventCondition {
                kind: 47,
                params: vec!["0".to_string(), "0".to_string()],
            }],
        },
    )]
    .into_iter()
    .collect();
    let actions: ActionMap = [(
        "TRIG_A".to_string(),
        MapAction {
            id: "TRIG_A".to_string(),
            fields: vec![],
            entries: vec![ActionEntry {
                kind: 112,
                params: vec![
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "9".to_string(),
                ],
            }],
        },
    )]
    .into_iter()
    .collect();
    let graph = build_trigger_graph(
        &HashMap::new(),
        &HashMap::new(),
        &triggers,
        &events,
        &actions,
    );
    let mut sim = Simulation::new();
    sim.trigger_runtime = TriggerRuntime::from_map(&triggers, &HashMap::new());

    let tick = sim.advance_master_frame(
        &[],
        None,
        &BTreeMap::new(),
        None,
        None,
        67,
        TickLane::Ordinary,
        Some(TriggerInputs {
            graph: &graph,
            triggers: &triggers,
            events: &events,
            actions: &actions,
            rules: None,
        }),
    );

    assert!(tick.frame_committed);
    assert_eq!(
        sim.take_master_frame_test_trace(),
        vec![
            MasterFrameTestRung::SessionCommands,
            MasterFrameTestRung::Triggers,
            MasterFrameTestRung::LogicVector,
            MasterFrameTestRung::Houses,
            MasterFrameTestRung::TeamScript,
            MasterFrameTestRung::FrameCommit,
            MasterFrameTestRung::PendingDelete,
        ]
    );
    assert_eq!(
        sim.drain_trigger_effects(),
        vec![TriggerEffect::CenterCameraAtWaypoint {
            waypoint: 9,
            immediate: true,
        }]
    );
}

#[test]
fn master_frame_save_load_continues_trigger_projectile_and_delete_state() {
    let triggers: TriggerMap = [(
        "TRIG_A".to_string(),
        make_trigger("TRIG_A", None, "Set Global", true, false),
    )]
    .into_iter()
    .collect();
    let events: EventMap = [(
        "TRIG_A".to_string(),
        MapEvent {
            id: "TRIG_A".to_string(),
            fields: vec![],
            conditions: vec![EventCondition {
                kind: 47,
                params: vec!["0".to_string(), "0".to_string()],
            }],
        },
    )]
    .into_iter()
    .collect();
    let actions: ActionMap = [(
        "TRIG_A".to_string(),
        MapAction {
            id: "TRIG_A".to_string(),
            fields: vec![],
            entries: vec![ActionEntry {
                kind: 28,
                params: vec!["13".to_string()],
            }],
        },
    )]
    .into_iter()
    .collect();
    let graph = build_trigger_graph(
        &HashMap::new(),
        &HashMap::new(),
        &triggers,
        &events,
        &actions,
    );
    let height_map = BTreeMap::new();
    let trigger_inputs = TriggerInputs {
        graph: &graph,
        triggers: &triggers,
        events: &events,
        actions: &actions,
        rules: None,
    };

    let mut original = Simulation::new();
    original.trigger_runtime = TriggerRuntime::from_map(&triggers, &HashMap::new());
    original.advance_master_frame(
        &[],
        None,
        &height_map,
        None,
        None,
        67,
        TickLane::Ordinary,
        Some(trigger_inputs),
    );
    assert!(original.trigger_runtime.globals_set.contains(&13));

    let projectile_id = original.allocate_stable_id();
    original.admit_projectile(
        projectile_id,
        ProjectileSpawn {
            source_id: crate::sim::combat::RAD_NO_ATTACKER,
            origin: ProjectileCoord::new(0, 0, 0),
            target: ProjectileTarget::Cell { rx: 4, ry: 0 },
            initial_target_position: ProjectileCoord::new(1024, 0, 0),
            payload: ProjectilePayload {
                base_damage: 40,
                warhead: crate::sim::intern::InternedId::from_index(0),
                weapon: crate::sim::intern::InternedId::from_index(0),
                owner: crate::sim::intern::InternedId::from_index(0),
            },
            speed_leptons_per_frame: 64,
            velocity: crate::sim::projectile::ProjectileVelocity::new(64, 0, 0),
            trajectory: crate::sim::projectile::ProjectileTrajectory::Straight,
            guidance: None,
            visual: crate::sim::projectile::ProjectileVisualState::new(0, 0, 0),
            arm_frames: 0,
            fuse_frames: None,
            ranged_fuse: false,
            tracks_target: false,
            target_expiry: TargetExpiryPolicy::Expire,
            collision: ProjectileCollisionPolicy::NONE,
        },
    );
    let deleted_id = spawn_type(&mut original, "GAPOWR");
    original.uninit(deleted_id);
    assert_eq!(original.projectiles.len(), 1);
    assert!(original.substrate.pending_delete.contains(&deleted_id));

    // Native in-scenario load restarts Scenario RNG from Seed0. Continue both
    // branches from that cursor while testing unrelated trigger/lifecycle state.
    original.scenario_rng = crate::sim::rng::SimRng::new(0);
    let hash_before_save = original.state_hash();
    let bytes = GameSnapshot::save(&original, 0, 0, "test_map", 0);
    let mut restored = GameSnapshot::load(&bytes).expect("snapshot loads").sim;
    restored
        .restore_after_snapshot_load()
        .expect("snapshot references and Logic membership restore");
    assert_eq!(restored.state_hash(), hash_before_save);
    assert!(restored.trigger_runtime.globals_set.contains(&13));
    assert_eq!(restored.projectiles.len(), 1);
    assert!(restored.substrate.pending_delete.contains(&deleted_id));

    let original_tick = original.advance_master_frame(
        &[],
        None,
        &height_map,
        None,
        None,
        67,
        TickLane::Ordinary,
        Some(trigger_inputs),
    );
    let mut replay_log = ReplayLog::new(ReplayHeader {
        version: 1,
        tick_hz: 15,
        seed: original.session.seed,
        map_name: original.session.map_name.clone(),
        rules_hash: 0,
    });
    replay_log.record_tick(original_tick.tick, Vec::new(), original_tick.state_hash);
    let restored_tick = restored.advance_master_frame(
        &[],
        None,
        &height_map,
        None,
        None,
        67,
        TickLane::Ordinary,
        Some(trigger_inputs),
    );

    assert!(original_tick.frame_committed);
    assert!(restored_tick.frame_committed);
    assert_eq!(original.state_hash(), restored.state_hash());
    assert_eq!(original.projectiles.len(), 1);
    assert!(original.substrate.pending_delete.is_empty());

    let mut replayed = GameSnapshot::load(&bytes).expect("snapshot reloads").sim;
    replayed
        .restore_after_snapshot_load()
        .expect("replay snapshot references and Logic membership restore");
    assert_eq!(
        ReplayRunner::run_fixture_master_frame(
            &mut replayed,
            &replay_log,
            None,
            &height_map,
            None,
            None,
            67,
            Some(trigger_inputs),
        ),
        vec![original_tick.state_hash],
    );
}

#[test]
fn trigger_runtime_latches_participate_in_state_hash() {
    let mut sim = Simulation::new();
    let before = sim.state_hash();

    sim.trigger_runtime.globals_set.insert(13);

    assert_ne!(sim.state_hash(), before);
}

#[test]
fn elapsed_time_uses_signed_current_frame_divided_by_fifteen() {
    let runtime = TriggerRuntime::default();
    let one_second = EventCondition {
        kind: 47,
        params: vec!["1".to_string(), "0".to_string()],
    };
    let zero_seconds = EventCondition {
        kind: 47,
        params: vec!["0".to_string(), "0".to_string()],
    };

    assert!(runtime.evaluate_event(&zero_seconds, 0, None));
    assert!(!runtime.evaluate_event(&one_second, 14, None));
    assert!(runtime.evaluate_event(&one_second, 15, None));
    assert!(
        !runtime.evaluate_event(&one_second, 0x8000_0000, None),
        "the native frame counter is divided as a signed 32-bit value"
    );
}

#[test]
fn global_actions_can_enable_and_force_followup_trigger() {
    let triggers: TriggerMap = [
        (
            "TRIG_A".to_string(),
            make_trigger("TRIG_A", None, "Set Global", true, false),
        ),
        (
            "TRIG_B".to_string(),
            make_trigger("TRIG_B", None, "Camera", false, false),
        ),
    ]
    .into_iter()
    .collect();
    let events: EventMap = [
        (
            "TRIG_A".to_string(),
            MapEvent {
                id: "TRIG_A".to_string(),
                fields: vec![
                    "1".to_string(),
                    "47".to_string(),
                    "1".to_string(),
                    "0".to_string(),
                ],
                conditions: vec![EventCondition {
                    kind: 47,
                    params: vec!["1".to_string(), "0".to_string()],
                }],
            },
        ),
        (
            "TRIG_B".to_string(),
            MapEvent {
                id: "TRIG_B".to_string(),
                fields: vec![
                    "1".to_string(),
                    "27".to_string(),
                    "7".to_string(),
                    "0".to_string(),
                ],
                conditions: vec![EventCondition {
                    kind: 27,
                    params: vec!["7".to_string(), "0".to_string()],
                }],
            },
        ),
    ]
    .into_iter()
    .collect();
    let actions: ActionMap = [
        (
            "TRIG_A".to_string(),
            MapAction {
                id: "TRIG_A".to_string(),
                fields: vec![
                    "3".to_string(),
                    "28".to_string(),
                    "7".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "53".to_string(),
                    "TRIG_B".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "22".to_string(),
                    "TRIG_B".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                ],
                entries: vec![
                    ActionEntry {
                        kind: 28,
                        params: vec![
                            "7".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                        ],
                    },
                    ActionEntry {
                        kind: 53,
                        params: vec![
                            "TRIG_B".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                        ],
                    },
                    ActionEntry {
                        kind: 22,
                        params: vec![
                            "TRIG_B".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                        ],
                    },
                ],
            },
        ),
        (
            "TRIG_B".to_string(),
            MapAction {
                id: "TRIG_B".to_string(),
                fields: vec![
                    "1".to_string(),
                    "112".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "3".to_string(),
                ],
                entries: vec![ActionEntry {
                    kind: 112,
                    params: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "3".to_string(),
                    ],
                }],
            },
        ),
    ]
    .into_iter()
    .collect();
    let graph = build_trigger_graph(
        &HashMap::new(),
        &HashMap::new(),
        &triggers,
        &events,
        &actions,
    );
    let mut runtime = TriggerRuntime::from_map(&triggers, &HashMap::new());

    assert_eq!(
        runtime.advance_at_frame(15, &graph, &triggers, &events, &actions, None, None),
        vec![TriggerEffect::CenterCameraAtWaypoint {
            waypoint: 3,
            immediate: true,
        }]
    );
}

#[test]
fn linked_trigger_field_queues_followup_trigger() {
    let triggers: TriggerMap = [
        (
            "TRIG_A".to_string(),
            make_trigger("TRIG_A", Some("TRIG_B"), "Primary", true, false),
        ),
        (
            "TRIG_B".to_string(),
            make_trigger("TRIG_B", None, "Followup", true, false),
        ),
    ]
    .into_iter()
    .collect();
    let events: EventMap = [
        (
            "TRIG_A".to_string(),
            MapEvent {
                id: "TRIG_A".to_string(),
                fields: vec![
                    "1".to_string(),
                    "47".to_string(),
                    "1".to_string(),
                    "0".to_string(),
                ],
                conditions: vec![EventCondition {
                    kind: 47,
                    params: vec!["1".to_string(), "0".to_string()],
                }],
            },
        ),
        (
            "TRIG_B".to_string(),
            MapEvent {
                id: "TRIG_B".to_string(),
                fields: vec![
                    "1".to_string(),
                    "28".to_string(),
                    "9".to_string(),
                    "0".to_string(),
                ],
                conditions: vec![EventCondition {
                    kind: 28,
                    params: vec!["9".to_string(), "0".to_string()],
                }],
            },
        ),
    ]
    .into_iter()
    .collect();
    let actions: ActionMap = [
        (
            "TRIG_A".to_string(),
            MapAction {
                id: "TRIG_A".to_string(),
                fields: vec![
                    "1".to_string(),
                    "28".to_string(),
                    "5".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                ],
                entries: vec![ActionEntry {
                    kind: 28,
                    params: vec![
                        "5".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                    ],
                }],
            },
        ),
        (
            "TRIG_B".to_string(),
            MapAction {
                id: "TRIG_B".to_string(),
                fields: vec![
                    "1".to_string(),
                    "112".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "4".to_string(),
                ],
                entries: vec![ActionEntry {
                    kind: 112,
                    params: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "4".to_string(),
                    ],
                }],
            },
        ),
    ]
    .into_iter()
    .collect();
    let graph = build_trigger_graph(
        &HashMap::new(),
        &HashMap::new(),
        &triggers,
        &events,
        &actions,
    );
    let mut runtime = TriggerRuntime::from_map(&triggers, &HashMap::new());

    assert_eq!(
        runtime.advance_at_frame(15, &graph, &triggers, &events, &actions, None, None),
        vec![TriggerEffect::CenterCameraAtWaypoint {
            waypoint: 4,
            immediate: true,
        }]
    );
}

#[test]
fn forced_trigger_with_unmet_conditions_does_not_fire() {
    let triggers: TriggerMap = [
        (
            "TRIG_A".to_string(),
            make_trigger("TRIG_A", None, "Force", true, false),
        ),
        (
            "TRIG_B".to_string(),
            make_trigger("TRIG_B", None, "Blocked", true, false),
        ),
    ]
    .into_iter()
    .collect();
    let events: EventMap = [
        (
            "TRIG_A".to_string(),
            MapEvent {
                id: "TRIG_A".to_string(),
                fields: vec![
                    "1".to_string(),
                    "47".to_string(),
                    "1".to_string(),
                    "0".to_string(),
                ],
                conditions: vec![EventCondition {
                    kind: 47,
                    params: vec!["1".to_string(), "0".to_string()],
                }],
            },
        ),
        (
            "TRIG_B".to_string(),
            MapEvent {
                id: "TRIG_B".to_string(),
                fields: vec![
                    "1".to_string(),
                    "27".to_string(),
                    "99".to_string(),
                    "0".to_string(),
                ],
                conditions: vec![EventCondition {
                    kind: 27,
                    params: vec!["99".to_string(), "0".to_string()],
                }],
            },
        ),
    ]
    .into_iter()
    .collect();
    let actions: ActionMap = [
        (
            "TRIG_A".to_string(),
            MapAction {
                id: "TRIG_A".to_string(),
                fields: vec![
                    "1".to_string(),
                    "22".to_string(),
                    "TRIG_B".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                ],
                entries: vec![ActionEntry {
                    kind: 22,
                    params: vec![
                        "TRIG_B".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                    ],
                }],
            },
        ),
        (
            "TRIG_B".to_string(),
            MapAction {
                id: "TRIG_B".to_string(),
                fields: vec![
                    "1".to_string(),
                    "112".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "8".to_string(),
                ],
                entries: vec![ActionEntry {
                    kind: 112,
                    params: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "8".to_string(),
                    ],
                }],
            },
        ),
    ]
    .into_iter()
    .collect();
    let graph = build_trigger_graph(
        &HashMap::new(),
        &HashMap::new(),
        &triggers,
        &events,
        &actions,
    );
    let mut runtime = TriggerRuntime::from_map(&triggers, &HashMap::new());

    assert_eq!(
        runtime.advance_at_frame(15, &graph, &triggers, &events, &actions, None, None),
        Vec::<TriggerEffect>::new()
    );
}

#[test]
fn mission_announce_then_force_end_emits_result_effects() {
    let triggers: TriggerMap = [(
        "TRIG_A".to_string(),
        make_trigger("TRIG_A", None, "End Mission", true, false),
    )]
    .into_iter()
    .collect();
    let events: EventMap = [(
        "TRIG_A".to_string(),
        MapEvent {
            id: "TRIG_A".to_string(),
            fields: vec![
                "1".to_string(),
                "47".to_string(),
                "1".to_string(),
                "0".to_string(),
            ],
            conditions: vec![EventCondition {
                kind: 47,
                params: vec!["1".to_string(), "0".to_string()],
            }],
        },
    )]
    .into_iter()
    .collect();
    let actions: ActionMap = [(
        "TRIG_A".to_string(),
        MapAction {
            id: "TRIG_A".to_string(),
            fields: vec![
                "2".to_string(),
                "67".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "69".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
            ],
            entries: vec![
                ActionEntry {
                    kind: 67,
                    params: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                    ],
                },
                ActionEntry {
                    kind: 69,
                    params: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                    ],
                },
            ],
        },
    )]
    .into_iter()
    .collect();
    let graph = build_trigger_graph(
        &HashMap::new(),
        &HashMap::new(),
        &triggers,
        &events,
        &actions,
    );
    let mut runtime = TriggerRuntime::from_map(&triggers, &HashMap::new());

    assert_eq!(
        runtime.advance_at_frame(15, &graph, &triggers, &events, &actions, None, None),
        vec![
            TriggerEffect::MissionAnnouncement {
                text: "Mission Accomplished".to_string(),
            },
            TriggerEffect::MissionResult {
                title: "Mission Accomplished".to_string(),
                detail: "The scenario ended after a victory announcement.".to_string(),
            },
        ]
    );
}

#[test]
fn local_variables_seed_and_gate_followup_triggers() {
    let triggers: TriggerMap = [
        (
            "TRIG_A".to_string(),
            make_trigger("TRIG_A", None, "Flip Local", true, false),
        ),
        (
            "TRIG_B".to_string(),
            make_trigger("TRIG_B", None, "Uses Local", true, false),
        ),
    ]
    .into_iter()
    .collect();
    let events: EventMap = [
        (
            "TRIG_A".to_string(),
            MapEvent {
                id: "TRIG_A".to_string(),
                fields: vec![
                    "1".to_string(),
                    "37".to_string(),
                    "2".to_string(),
                    "0".to_string(),
                ],
                conditions: vec![EventCondition {
                    kind: 37,
                    params: vec!["2".to_string(), "0".to_string()],
                }],
            },
        ),
        (
            "TRIG_B".to_string(),
            MapEvent {
                id: "TRIG_B".to_string(),
                fields: vec![
                    "1".to_string(),
                    "36".to_string(),
                    "2".to_string(),
                    "0".to_string(),
                ],
                conditions: vec![EventCondition {
                    kind: 36,
                    params: vec!["2".to_string(), "0".to_string()],
                }],
            },
        ),
    ]
    .into_iter()
    .collect();
    let actions: ActionMap = [
        (
            "TRIG_A".to_string(),
            MapAction {
                id: "TRIG_A".to_string(),
                fields: vec![
                    "1".to_string(),
                    "56".to_string(),
                    "2".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                ],
                entries: vec![ActionEntry {
                    kind: 56,
                    params: vec![
                        "2".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                    ],
                }],
            },
        ),
        (
            "TRIG_B".to_string(),
            MapAction {
                id: "TRIG_B".to_string(),
                fields: vec![
                    "1".to_string(),
                    "112".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "6".to_string(),
                ],
                entries: vec![ActionEntry {
                    kind: 112,
                    params: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "6".to_string(),
                    ],
                }],
            },
        ),
    ]
    .into_iter()
    .collect();
    let local_variables: LocalVariableMap = [(
        2,
        LocalVariable {
            index: 2,
            name: "BridgeDone".to_string(),
            initially_set: false,
        },
    )]
    .into_iter()
    .collect();
    let graph = build_trigger_graph(
        &HashMap::new(),
        &HashMap::new(),
        &triggers,
        &events,
        &actions,
    );
    let mut runtime = TriggerRuntime::from_map(&triggers, &local_variables);

    assert_eq!(
        runtime.advance_at_frame(0, &graph, &triggers, &events, &actions, None, None),
        Vec::<TriggerEffect>::new()
    );
    assert!(runtime.locals_set.contains(&2));
    assert_eq!(
        runtime.advance_at_frame(0, &graph, &triggers, &events, &actions, None, None),
        vec![TriggerEffect::CenterCameraAtWaypoint {
            waypoint: 6,
            immediate: true,
        }]
    );
}

#[test]
fn techtype_exists_and_not_exists_query_simulation_world() {
    let triggers: TriggerMap = [
        (
            "TRIG_A".to_string(),
            make_trigger("TRIG_A", None, "Need Two Power Plants", true, false),
        ),
        (
            "TRIG_B".to_string(),
            make_trigger("TRIG_B", None, "No Radar", true, false),
        ),
    ]
    .into_iter()
    .collect();
    let events: EventMap = [
        (
            "TRIG_A".to_string(),
            MapEvent {
                id: "TRIG_A".to_string(),
                fields: vec![
                    "1".to_string(),
                    "60".to_string(),
                    "2".to_string(),
                    "GAPOWR".to_string(),
                ],
                conditions: vec![EventCondition {
                    kind: 60,
                    params: vec!["2".to_string(), "GAPOWR".to_string()],
                }],
            },
        ),
        (
            "TRIG_B".to_string(),
            MapEvent {
                id: "TRIG_B".to_string(),
                fields: vec![
                    "1".to_string(),
                    "61".to_string(),
                    "0".to_string(),
                    "GAAIRC".to_string(),
                ],
                conditions: vec![EventCondition {
                    kind: 61,
                    params: vec!["0".to_string(), "GAAIRC".to_string()],
                }],
            },
        ),
    ]
    .into_iter()
    .collect();
    let actions: ActionMap = [
        (
            "TRIG_A".to_string(),
            MapAction {
                id: "TRIG_A".to_string(),
                fields: vec![
                    "1".to_string(),
                    "112".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "11".to_string(),
                ],
                entries: vec![ActionEntry {
                    kind: 112,
                    params: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "11".to_string(),
                    ],
                }],
            },
        ),
        (
            "TRIG_B".to_string(),
            MapAction {
                id: "TRIG_B".to_string(),
                fields: vec![
                    "1".to_string(),
                    "112".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "12".to_string(),
                ],
                entries: vec![ActionEntry {
                    kind: 112,
                    params: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "12".to_string(),
                    ],
                }],
            },
        ),
    ]
    .into_iter()
    .collect();
    let graph = build_trigger_graph(
        &HashMap::new(),
        &HashMap::new(),
        &triggers,
        &events,
        &actions,
    );
    let mut runtime = TriggerRuntime::from_map(&triggers, &HashMap::new());
    let mut sim = Simulation::new();
    spawn_type(&mut sim, "GAPOWR");
    spawn_type(&mut sim, "GAPOWR");

    assert_eq!(
        runtime.advance_at_frame(
            0,
            &graph,
            &triggers,
            &events,
            &actions,
            Some(&mut sim),
            None,
        ),
        vec![
            TriggerEffect::CenterCameraAtWaypoint {
                waypoint: 11,
                immediate: true,
            },
            TriggerEffect::CenterCameraAtWaypoint {
                waypoint: 12,
                immediate: true,
            },
        ]
    );
}
