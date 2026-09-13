use super::*;
use crate::rules::ini_parser::IniFile;
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::sim::{
    command::Command,
    overlay_grid::OverlayGrid,
    pathfinding::{PathGrid, zone_map::ZoneGrid},
};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) fn fixture() -> (
    Simulation,
    RuleSet,
    crate::map::overlay_types::OverlayTypeRegistry,
) {
    let mut text = String::from(
        "[InfantryTypes]\n0=ENGINEER\n[AircraftTypes]\n0=HORNET\n[HORNET]\nLandable=yes\nSpeed=12\nSpeedType=Winged\nStrength=75\nLocomotor={4A582746-9839-11d1-B709-00A024DDAFD1}\n[BuildingTypes]\n0=CABHUT\n[ENGINEER]\nEngineer=yes\nSpeed=4\nSpeedType=Foot\nStrength=75\nLocomotor={4A582744-9839-11d1-B709-00A024DDAFD1}\n[CABHUT]\nBridgeRepairHut=yes\nFoundation=1x1\nStrength=200\n[CombatDamage]\nC4Warhead=SA\n[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n[OverlayTypes]\n",
    );
    for id in 0..=238 {
        text.push_str(&format!("{id}=O{id}\n"));
    }
    for id in 0..=238 {
        text.push_str(&format!("[O{id}]\nLand=Clear\nNoUseTileLandType=no\n"));
    }
    for land in crate::rules::terrain_rules::LandType::ALL.iter().take(9) {
        text.push_str(&format!(
            "[{}]\nFoot=100%\nTrack=100%\nWheel=100%\nBuildable=yes\n",
            land.section_name()
        ));
    }
    let ini = IniFile::from_str(&text);
    let rules = RuleSet::from_ini(&ini).unwrap();
    let registry = crate::map::overlay_types::OverlayTypeRegistry::from_ini(&ini, None);
    let mut terrain = ResolvedTerrainGrid::from_cells(
        33,
        33,
        (0..33)
            .flat_map(|y| {
                (0..33).map(move |x| {
                    crate::sim::world::lifecycle_tests::common_raw_terrain_cell(x, y, 0, false)
                })
            })
            .collect(),
    );
    crate::map::resolved_terrain::install_ordinary_repair_test_catalog(&mut terrain);
    terrain.test_set_high_bridge_set_starts(Some(100), Some(200));
    let terrain_rules = crate::rules::terrain_rules::TerrainRules::from_ini(&ini);
    let costs = terrain_rules
        .semantics_for_land_type(0)
        .unwrap()
        .speed_costs;
    for y in 0..33 {
        for x in 0..33 {
            let c = terrain.cell_mut(x, y).unwrap();
            c.speed_costs = costs.clone();
            c.base_speed_costs = costs.clone();
        }
    }
    let bounds =
        crate::map::playfield::PlayfieldBounds::from_normalized_local_size(16, 0, 0, 16, 16);
    let bridges =
        BridgeRuntimeState::from_resolved_terrain_with_map_size(&terrain, true, 300, (16, 16));
    let path = PathGrid::from_resolved_terrain(&terrain);
    let zones = ZoneGrid::build_with_native_map_context(
        &path,
        &BTreeMap::new(),
        &terrain,
        bridges.endpoint_records(),
        Some((16, 16)),
        Some(bounds),
    );
    assert!(zones.hierarchy_for(MovementZone::Normal).is_some());
    let mut sim = Simulation::with_seed(31);
    sim.resolve_type_handles(&rules);
    sim.playfield_bounds = Some(bounds);
    sim.session.map_width = 33;
    sim.session.map_height = 33;
    sim.overlay_grid = Some(OverlayGrid::new(33, 33));
    sim.bridge_state = Some(bridges);
    sim.zone_grid = Some(zones);
    sim.path_grid = Some(Arc::new(path));
    sim.install_resolved_terrain_for_new_map(terrain);
    sim.terrain_costs = crate::sim::pathfinding::terrain_cost::build_canonical_terrain_cost_grids(
        sim.resolved_terrain.as_ref().unwrap(),
    );
    for p in [(15, 15), (16, 15), (17, 14), (17, 15), (17, 16)] {
        assert!(crate::sim::cell_rect::cell_is_in_playfield_height_aware(
            (p.0, p.1),
            sim.playfield_bounds,
            sim.resolved_terrain.as_ref()
        ));
    }
    (sim, rules, registry)
}

fn broken_strip(sim: &mut Simulation) {
    for (x, y) in [(17, 14), (17, 15), (17, 16)] {
        let c = sim
            .resolved_terrain
            .as_mut()
            .unwrap()
            .cell_mut(x, y)
            .unwrap();
        c.bridge_facts.overlay_id = Some(231);
        sim.overlay_grid
            .as_mut()
            .unwrap()
            .write_bridge_overlay_identity(x, y, 231);
    }
}

#[test]
fn ordinary_engineer_command_repairs_from_completed_walk_in_object_turn() {
    command_repair_fixture(false, false);
}

#[test]
fn slave_master_admission_reaches_head_selection_in_the_same_object_turn() {
    use crate::map::entities::EntityCategory;
    use crate::sim::components::{DriveCoord, NavTargetRef};
    use crate::sim::movement::{ground_pose, locomotor::MovementLayer};
    use crate::sim::occupancy::{CellListInsertion, infantry_raw_occupation_mask};
    use crate::util::fixed_math::SimFixed;
    for later_blocker in [false, true] {
        let (mut sim, rules, registry) = fixture();
        let master = sim
            .spawn_object("CABHUT", "Americans", 16, 15, 0, &rules, &BTreeMap::new())
            .unwrap();
        let slave = sim
            .spawn_object("ENGINEER", "Americans", 15, 15, 0, &rules, &BTreeMap::new())
            .unwrap();
        // Supplied manager/deposit leg tests the real object-turn admission
        // continuation, not production slave AI or a stock hut manager.
        assert!(crate::sim::movement::issue_direct_move(
            &mut sim.substrate.entities,
            slave,
            (16, 15),
            SimFixed::from_num(150)
        ));
        sim.mission_assign_exact(
            slave,
            crate::sim::mission::MissionId::from_known(
                crate::rules::mission_data::MissionType::Move,
            ),
            sim.session.binary_frame,
        )
        .unwrap();
        let e = sim.substrate.entities.get_mut(slave).unwrap();
        e.slave_harvester = Some(crate::sim::slave_miner::SlaveHarvester::new(master, 4));
        e.navigation.nav_com = Some(NavTargetRef::Cell { rx: 16, ry: 15 });
        e.locomotor
            .as_mut()
            .unwrap()
            .set_walk_destination(Some(DriveCoord::cell(16, 15, 0)));
        // As for the existing building-enter producer, static path blocking
        // is bypassed for the admitted last leg; live objects still decide.
        e.movement_target.as_mut().unwrap().bypass_grid = true;
        let before = ground_pose::position_world_coord(&e.position);
        sim.production.slave_bindings.insert(master, vec![slave]);
        if later_blocker {
            let mut b = crate::sim::game_entity::GameEntity::test_default(
                100,
                "CABHUT",
                "Americans",
                16,
                15,
            );
            b.owner = sim.intern("Americans");
            b.type_ref = sim.intern("CABHUT");
            b.category = EntityCategory::Structure;
            sim.substrate.entities.insert(b);
            sim.substrate.occupancy.add(
                16,
                15,
                100,
                MovementLayer::Ground,
                None,
                CellListInsertion::AppendBuilding,
            );
        }
        let grid = sim.path_grid_snapshot();
        sim.advance_live_object_pass(Some(&rules), grid.as_deref(), Some(&registry));
        let e = sim.substrate.entities.get(slave).unwrap();
        let head = e.locomotor.as_ref().unwrap().step_head();
        if later_blocker {
            assert_eq!(head, None, "a later hard blocker is still reached");
            assert_eq!(ground_pose::position_world_coord(&e.position), before);
        } else {
            let head = head.expect("Clear CanEnter resumes fresh-head selection in this call");
            assert_eq!((head.x / 256, head.y / 256), (16, 15));
            assert_ne!(
                ground_pose::position_world_coord(&e.position),
                before,
                "the paid continuation also ran"
            );
            let mask = infantry_raw_occupation_mask(
                SimFixed::from_num(head.x % 256),
                SimFixed::from_num(head.y % 256),
            );
            assert_eq!(
                sim.substrate.raw_cell_occupation.ground_bits(16, 15) & mask,
                mask
            );
            assert_eq!(
                sim.substrate
                    .raw_cell_occupation
                    .ground_infantry_owner(16, 15),
                Some(e.owner())
            );
        }
    }
}

#[test]
fn engineer_repair_damages_landed_fly_occupant_before_its_later_turn() {
    //Supplied just-landed Fly state; native carrier/Hornet reachability is
    //proved separately. This exercises repair and the actual damage receiver.
    command_repair_fixture(true, false);
}

#[test]
fn engineer_repair_damages_landed_fly_with_known_non_waypoint_team_script() {
    command_repair_fixture(true, true);
}

fn command_repair_fixture(with_aircraft: bool, with_team: bool) {
    let (mut sim, rules, registry) = fixture();
    let hut = sim
        .spawn_object("CABHUT", "Soviets", 16, 15, 0, &rules, &BTreeMap::new())
        .unwrap();
    let engineer = sim
        .spawn_object("ENGINEER", "Americans", 15, 15, 0, &rules, &BTreeMap::new())
        .unwrap();
    assert!(sim.substrate.entities.get(engineer).is_some());
    assert_eq!(
        sim.substrate
            .entities
            .get(engineer)
            .unwrap()
            .locomotor
            .as_ref()
            .unwrap()
            .kind,
        crate::rules::locomotor_type::LocomotorKind::Walk
    );
    broken_strip(&mut sim);
    let grid = sim.path_grid_snapshot();
    assert!(sim.apply_command(
        "Americans",
        &Command::CaptureBuilding {
            engineer_id: engineer,
            target_building_id: hut
        },
        Some(&rules),
        grid.as_deref(),
        &BTreeMap::new()
    ));
    assert_eq!(
        sim.substrate
            .entities
            .get(engineer)
            .unwrap()
            .navigation
            .nav_com,
        Some(crate::sim::components::NavTargetRef::Building { id: hut }),
        "the real order publishes Foot NavCom before head selection"
    );
    let mut saw_boundary_before_completion = false;
    let mut changed = false;
    let mut trace = Vec::new();
    let mut aircraft = None;
    for _ in 0..100 {
        if with_aircraft
            && aircraft.is_none()
            && sim.substrate.entities.get(engineer).is_some_and(|e| {
                let Some(head) = e.locomotor.as_ref().and_then(|l| l.step_head()) else {
                    return false;
                };
                let xy = crate::sim::movement::ground_pose::position_world_xy(&e.position);
                let dx = head.x - xy[0];
                let dy = head.y - xy[1];
                dx * dx + dy * dy < 17 * 17
            })
        {
            let id = sim
                .spawn_object_at_height("HORNET", "Americans", 17, 15, 0, 0, &rules)
                .unwrap();
            assert!(id > engineer, "repair precedes this aircraft's Logic visit");
            assert!(sim.substrate.occupancy.get(17, 15).is_some_and(|list| {
                list.iter_layer(crate::sim::movement::locomotor::MovementLayer::Ground)
                    .any(|e| e.entity_id == id)
            }));
            if with_team {
                let script_id = sim.intern("REPAIR_TEAM_SCRIPT");
                let owner = sim.substrate.entities.get(id).unwrap().owner();
                sim.team_script_vm.register_script(
                    crate::sim::team_script_vm::TeamScriptDefinition {
                        id: script_id,
                        actions: vec![crate::sim::team_script_vm::TeamScriptAction {
                            action_id: 0,
                            argument: 0,
                        }],
                        source: crate::rules::team_ai_ini::TeamAiDefinitionSource::FixedAimd,
                    },
                );
                sim.team_script_vm.create_team(
                    owner,
                    script_id,
                    vec![id],
                    None,
                    sim.session.binary_frame as i32,
                );
                assert!(sim.team_script_vm.team_for_member(id).is_some());
            }
            aircraft = Some(id);
        }
        let grid = sim.path_grid_snapshot();
        let result = sim.advance_tick(
            &[],
            Some(&rules),
            &BTreeMap::new(),
            grid.as_deref(),
            Some(&registry),
            67,
        );
        changed |= result.bridge_state_changed;
        if let Some(e) = sim.substrate.entities.get(engineer) {
            trace.push(format!(
                "frame{} cell{},{} sub{},{} head{:?} mt{:?} mission{:?} alive{}",
                sim.session.binary_frame,
                e.position.rx,
                e.position.ry,
                e.position.sub_x,
                e.position.sub_y,
                e.locomotor.as_ref().unwrap().step_head(),
                e.movement_target,
                e.mission.current(),
                e.lifecycle.object_alive
            ));
            if e.position.rx == 16 && e.locomotor.as_ref().unwrap().step_head().is_some() {
                assert!(!changed, "crossing alone must not emit PerCell repair");
                if !saw_boundary_before_completion {
                    let mask = crate::sim::occupancy::infantry_raw_occupation_mask(
                        e.position.sub_x,
                        e.position.sub_y,
                    );
                    assert_ne!(
                        (if e.on_bridge {
                            sim.substrate
                                .raw_cell_occupation
                                .deck_bits(e.position.rx, e.position.ry)
                        } else {
                            sim.substrate
                                .raw_cell_occupation
                                .ground_bits(e.position.rx, e.position.ry)
                        }) & mask,
                        0,
                        "current XYZ is marked before a later object's repair query; marked={} enabled={} bridge={} raw={:x}/{:x} mask={mask:x} trace={trace:#?}",
                        e.lifecycle.cell_marked,
                        e.foot_occupation_enabled,
                        e.on_bridge,
                        sim.substrate
                            .raw_cell_occupation
                            .ground_bits(e.position.rx, e.position.ry),
                        sim.substrate
                            .raw_cell_occupation
                            .deck_bits(e.position.rx, e.position.ry)
                    );
                }
                saw_boundary_before_completion = true;
            }
        } else {
            break;
        }
    }
    assert!(saw_boundary_before_completion, "{trace:#?}");
    assert!(
        changed,
        "real object-turn repair must publish its outcome: {trace:#?}"
    );
    assert!(
        sim.substrate.entities.get(engineer).is_none(),
        "same-call Uninit followed by normal frame-tail deletion"
    );
    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(16, 15) & 0x1c,
        0
    );
    assert_eq!(
        sim.substrate
            .raw_cell_occupation
            .ground_infantry_owner(16, 15),
        None
    );
    for (x, y) in [(17, 14), (17, 15), (17, 16)] {
        assert!(
            (205..=208).contains(
                &sim.resolved_terrain
                    .as_ref()
                    .unwrap()
                    .cell(x, y)
                    .unwrap()
                    .bridge_facts
                    .overlay_id
                    .unwrap()
            )
        );
        assert_eq!(
            sim.path_grid().unwrap().cell(x, y).unwrap().ground_level,
            sim.resolved_terrain
                .as_ref()
                .unwrap()
                .cell(x, y)
                .unwrap()
                .level
        );
    }
    assert!(
        sim.zone_grid
            .as_mut()
            .unwrap()
            .base_topology_mut()
            .is_some()
    );
    assert!(sim.terrain_costs.contains_key(&SpeedType::Foot));
    if with_aircraft {
        let id = aircraft.expect("fixture must install the landed Fly before PerCell");
        assert!(
            sim.substrate
                .entities
                .get(id)
                .is_none_or(|e| !e.lifecycle.object_alive)
        );
        assert!(!sim.substrate.occupancy.contains_entity(17, 15, id));
    }
}

#[test]
fn walk_boundary_marks_current_xyz_without_replacing_head_or_consuming_path() {
    use crate::sim::components::DriveCoord;
    use crate::sim::movement::{ground_pose, locomotor::MovementLayer, walk_head};
    use crate::util::fixed_math::SimFixed;

    let (mut sim, rules, registry) = fixture();
    let id = sim
        .spawn_object("ENGINEER", "Americans", 15, 15, 0, &rules, &BTreeMap::new())
        .unwrap();
    let grid = sim.path_grid_snapshot();
    assert!(sim.apply_command(
        "Americans",
        &Command::Move {
            entity_id: id,
            target_rx: 17,
            target_ry: 15,
            queue: false,
            group_id: None,
        },
        Some(&rules),
        grid.as_deref(),
        &BTreeMap::new(),
    ));
    drop(grid);
    let head = DriveCoord {
        x: 16 * 256 + 192,
        y: 15 * 256 + 192,
        z: 0,
    };
    let (owner, current, queue, path_index) = {
        let e = sim.substrate.entities.get_mut(id).unwrap();
        e.position.sub_x = SimFixed::from_num(192);
        e.position.sub_y = SimFixed::from_num(64);
        e.locomotor.as_mut().unwrap().set_step_head(Some(head));
        (
            e.owner(),
            ground_pose::position_world_coord(&e.position),
            e.navigation.path_replay.clone(),
            e.movement_target.as_ref().unwrap().next_index,
        )
    };
    sim.substrate
        .raw_cell_occupation
        .clear_ground_infantry(15, 15, 0x1f);
    for coord in [current, head] {
        walk_head::raw_at(
            &mut sim.substrate.raw_cell_occupation,
            owner,
            coord,
            true,
            sim.resolved_terrain.as_ref(),
            None,
        );
    }
    let crossing = DriveCoord {
        x: 16 * 256 + 8,
        y: 15 * 256 + 64,
        z: 0,
    };
    sim.run_walk_boundary(id, crossing, Some(&rules), None, Some(&registry));
    let e = sim.substrate.entities.get(id).unwrap();
    assert_eq!(ground_pose::position_world_coord(&e.position), crossing);
    assert!(e.lifecycle.cell_marked && e.foot_occupation_enabled);
    assert_eq!(e.sub_cell, Some(0));
    assert_eq!(e.locomotor.as_ref().unwrap().step_head(), Some(head));
    assert_eq!(e.navigation.path_replay, queue);
    assert_eq!(e.movement_target.as_ref().unwrap().next_index, path_index);
    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(15, 15) & 0x1f,
        0
    );
    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(16, 15) & 0x1f,
        0x11
    );
    assert_eq!(
        sim.substrate
            .raw_cell_occupation
            .ground_infantry_owner(16, 15),
        Some(owner)
    );
    assert!(sim.substrate.occupancy.get(15, 15).is_none_or(|list| {
        !list
            .iter_layer(MovementLayer::Ground)
            .any(|entry| entry.entity_id == id)
    }));
    assert!(sim.substrate.occupancy.get(16, 15).is_some_and(|list| {
        list.iter_layer(MovementLayer::Ground)
            .any(|entry| entry.entity_id == id && entry.sub_cell == Some(0))
    }));
    assert!(sim.radar_events.is_empty());
}

#[test]
fn diagonal_walk_relinks_the_first_actual_side_cell_before_reaching_its_head() {
    use crate::sim::movement::ground_pose;
    use crate::util::fixed_math::SimFixed;
    let (mut sim, rules, registry) = fixture();
    let id = sim
        .spawn_object("ENGINEER", "Americans", 15, 15, 0, &rules, &BTreeMap::new())
        .unwrap();
    {
        let e = sim.substrate.entities.get_mut(id).unwrap();
        e.position.sub_x = SimFixed::from_num(192);
        e.position.sub_y = SimFixed::from_num(64);
    }
    let grid = sim.path_grid_snapshot();
    assert!(sim.apply_command(
        "Americans",
        &Command::Move {
            entity_id: id,
            target_rx: 16,
            target_ry: 16,
            queue: false,
            group_id: None,
        },
        Some(&rules),
        grid.as_deref(),
        &BTreeMap::new()
    ));
    drop(grid);
    for _ in 0..100 {
        let grid = sim.path_grid_snapshot();
        sim.advance_tick(
            &[],
            Some(&rules),
            &BTreeMap::new(),
            grid.as_deref(),
            Some(&registry),
            67,
        );
        let e = sim.substrate.entities.get(id).unwrap();
        let xy = ground_pose::position_world_xy(&e.position);
        if xy[0] >= 16 * 256 {
            assert!(xy[1] < 16 * 256, "unequal offsets must cross X first");
            assert_eq!((e.position.rx, e.position.ry), (16, 15));
            let head = e.locomotor.as_ref().unwrap().step_head().unwrap();
            assert_eq!((head.x / 256, head.y / 256), (16, 16));
            assert!(sim.substrate.occupancy.contains_entity(16, 15, id));
            assert!(!sim.substrate.occupancy.contains_entity(15, 15, id));
            let mask = crate::sim::occupancy::infantry_raw_occupation_mask(
                e.position.sub_x,
                e.position.sub_y,
            );
            assert_ne!(
                sim.substrate.raw_cell_occupation.ground_bits(16, 15) & mask,
                0
            );
            return;
        }
    }
    panic!("diagonal walker never reached first side cell");
}

#[test]
fn refused_fresh_walk_head_restores_the_current_raw_occupation() {
    use crate::util::fixed_math::SimFixed;
    let (mut sim, rules, registry) = fixture();
    let id = sim
        .spawn_object("ENGINEER", "Americans", 15, 15, 0, &rules, &BTreeMap::new())
        .unwrap();
    let owner = {
        let e = sim.substrate.entities.get_mut(id).unwrap();
        e.position.sub_x = SimFixed::from_num(192);
        e.position.sub_y = SimFixed::from_num(64);
        e.owner()
    };
    sim.substrate
        .raw_cell_occupation
        .clear_ground_infantry(15, 15, 0x1f);
    sim.substrate
        .raw_cell_occupation
        .mark_ground_infantry(15, 15, 4, owner);
    // These are retained heads, with no listed infantry at the destination.
    sim.substrate
        .raw_cell_occupation
        .mark_ground_infantry(16, 15, 0x1c, owner);
    let grid = sim.path_grid_snapshot();
    assert!(sim.apply_command(
        "Americans",
        &Command::Move {
            entity_id: id,
            target_rx: 16,
            target_ry: 15,
            queue: false,
            group_id: None,
        },
        Some(&rules),
        grid.as_deref(),
        &BTreeMap::new()
    ));
    drop(grid);
    let grid = sim.path_grid_snapshot();
    sim.advance_tick(
        &[],
        Some(&rules),
        &BTreeMap::new(),
        grid.as_deref(),
        Some(&registry),
        67,
    );
    let e = sim.substrate.entities.get(id).unwrap();
    assert_eq!((e.position.rx, e.position.ry), (15, 15));
    assert_eq!(e.locomotor.as_ref().unwrap().step_head(), None);
    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(15, 15) & 0x1c,
        4
    );
    assert_eq!(
        sim.substrate
            .raw_cell_occupation
            .ground_infantry_owner(15, 15),
        Some(owner)
    );
    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(16, 15) & 0x1c,
        0x1c
    );
}

#[test]
fn production_fresh_head_and_raw_history_match_original_walk_producer() {
    use crate::sim::components::DriveCoord;
    use crate::util::fixed_math::SimFixed;
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tools/spatial_oracle/walk_head_occupation.json"
    ))
    .unwrap();
    let cases = data["producer"].as_array().unwrap();
    assert_eq!(cases.len(), 4);
    for case in cases {
        let (mut sim, rules, registry) = fixture();
        let id = sim
            .spawn_object("ENGINEER", "Americans", 9, 10, 0, &rules, &BTreeMap::new())
            .unwrap();
        for x in [9, 10] {
            let c = sim
                .resolved_terrain
                .as_mut()
                .unwrap()
                .cell_mut(x, 10)
                .unwrap();
            c.level = 2;
            c.slope_type = 1;
        }
        sim.path_grid = Some(Arc::new(PathGrid::from_resolved_terrain(
            sim.resolved_terrain.as_ref().unwrap(),
        )));
        let owner = {
            let e = sim.substrate.entities.get_mut(id).unwrap();
            e.position.sub_x = SimFixed::from_num(192);
            e.position.sub_y = SimFixed::from_num(64);
            e.position.z = 2;
            e.position.exact_z_leptons = Some(260);
            e.owner()
        };
        sim.substrate
            .raw_cell_occupation
            .clear_ground_infantry(9, 10, 0x1f);
        sim.substrate
            .raw_cell_occupation
            .mark_ground_infantry(9, 10, 4, owner);
        let ground = case["input"]["ground"].as_u64().unwrap() as u8;
        sim.substrate
            .raw_cell_occupation
            .mark_ground_infantry(10, 10, ground, owner);
        let grid = sim.path_grid_snapshot();
        assert!(sim.apply_command(
            "Americans",
            &Command::Move {
                entity_id: id,
                target_rx: 10,
                target_ry: 10,
                queue: false,
                group_id: None,
            },
            Some(&rules),
            grid.as_deref(),
            &BTreeMap::new()
        ));
        drop(grid);
        sim.advance_tick(
            &[],
            Some(&rules),
            &BTreeMap::new(),
            None,
            Some(&registry),
            67,
        );
        let output = &case["output"];
        let expected = DriveCoord {
            x: output["head"][0].as_i64().unwrap() as i32,
            y: output["head"][1].as_i64().unwrap() as i32,
            z: output["head"][2].as_i64().unwrap() as i32,
        };
        let accepted = output["accepted"].as_bool().unwrap();
        assert_eq!(
            sim.substrate
                .entities
                .get(id)
                .unwrap()
                .locomotor
                .as_ref()
                .unwrap()
                .step_head(),
            accepted.then_some(expected),
            "{case}"
        );
        assert_eq!(
            sim.substrate.raw_cell_occupation.ground_bits(9, 10),
            output["current_ground"].as_u64().unwrap() as u8,
            "{case}"
        );
        assert_eq!(
            sim.substrate.raw_cell_occupation.ground_bits(10, 10),
            output["ground"].as_u64().unwrap() as u8,
            "{case}"
        );
        assert_eq!(
            sim.substrate
                .raw_cell_occupation
                .ground_infantry_owner(9, 10),
            (!accepted).then_some(owner),
            "{case}"
        );
    }
}

// The legacy global-repair tests below now start at an admitted completed
// Walk head. Actual command traversal is covered above; these cases isolate
// PerCell's publication and live Logic-vector continuation.
fn ready_repair_fixture(
    overlay: Option<u8>,
) -> (
    Simulation,
    RuleSet,
    crate::map::overlay_types::OverlayTypeRegistry,
    u64,
) {
    use crate::sim::bridge_state::{
        Axis, BridgeCellRole, BridgeRuntimeCell, BridgeheadAnchorClass, DamageState,
    };
    let (mut sim, rules, registry) = fixture();
    sim.mapgen_rng = Simulation::with_seed(0).mapgen_rng;
    let owner = sim.interner.intern("Americans");
    let mut house = crate::sim::house_state::HouseState::new(owner, 0, None, false, 0, 0);
    house.player_control = true;
    sim.houses.insert(owner, house);
    let hut = sim
        .spawn_object("CABHUT", "Soviets", 16, 15, 0, &rules, &BTreeMap::new())
        .unwrap();
    if let Some(overlay) = overlay {
        for (x, y) in [(17, 14), (17, 15), (17, 16)] {
            sim.resolved_terrain
                .as_mut()
                .unwrap()
                .cell_mut(x, y)
                .unwrap()
                .bridge_facts
                .overlay_id = Some(overlay);
            sim.overlay_grid
                .as_mut()
                .unwrap()
                .write_bridge_overlay_identity(x, y, overlay);
            sim.bridge_state.as_mut().unwrap().test_seed_cell(
                x,
                y,
                BridgeRuntimeCell {
                    deck_present: true,
                    destroyable: true,
                    deck_level: 0,
                    bridge_group_id: None,
                    damage_state: if overlay == 231 {
                        DamageState::Destroyed
                    } else {
                        DamageState::Healthy { variant: 0 }
                    },
                    axis: Some(Axis::NS),
                    role: BridgeCellRole::Body,
                    anchor_span_id: None,
                    overlay_byte: overlay,
                    bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
                },
            );
        }
    }
    (sim, rules, registry, hut)
}

fn ready_engineer(
    sim: &mut Simulation,
    rules: &RuleSet,
    registry: &crate::map::overlay_types::OverlayTypeRegistry,
    hut: u64,
) -> u64 {
    use crate::sim::components::{DriveCoord, NavTargetRef};
    let id = sim
        .spawn_object("ENGINEER", "Americans", 15, 15, 0, rules, &BTreeMap::new())
        .unwrap();
    assert!(crate::sim::movement::issue_direct_move(
        &mut sim.substrate.entities,
        id,
        (16, 15),
        crate::util::fixed_math::SimFixed::from_num(61)
    ));
    sim.mission_assign_exact(
        id,
        crate::sim::mission::MissionId::from_known(
            crate::rules::mission_data::MissionType::Capture,
        ),
        sim.session.binary_frame,
    )
    .unwrap();
    let e = sim.substrate.entities.get_mut(id).unwrap();
    e.navigation.nav_com = Some(NavTargetRef::Building { id: hut });
    e.capture_target = Some(hut);
    let head = DriveCoord::cell(16, 15, 0);
    e.locomotor
        .as_mut()
        .unwrap()
        .set_walk_destination(Some(head));
    sim.run_walk_boundary(id, head, Some(rules), None, Some(registry));
    sim.substrate
        .entities
        .get_mut(id)
        .unwrap()
        .locomotor
        .as_mut()
        .unwrap()
        .set_step_head(Some(head));
    id
}

fn repair_frame(
    sim: &mut Simulation,
    rules: &RuleSet,
    registry: &crate::map::overlay_types::OverlayTypeRegistry,
) -> crate::sim::world::TickResult {
    sim.advance_tick(&[], Some(rules), &BTreeMap::new(), None, Some(registry), 67)
}

fn repair_sounds(sim: &Simulation) -> Vec<bool> {
    sim.sound_events
        .iter()
        .filter_map(|event| match event {
            crate::sim::world::SimSoundEvent::BridgeRepaired { eva_allowed, .. } => {
                Some(*eva_allowed)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn engineer_enters_cabhut_repairs_bridge() {
    let (mut sim, rules, registry, hut) = ready_repair_fixture(Some(231));
    let engineer = ready_engineer(&mut sim, &rules, &registry, hut);
    assert!(repair_frame(&mut sim, &rules, &registry).bridge_state_changed);
    assert!(sim.substrate.entities.get(engineer).is_none());
    assert_eq!(repair_sounds(&sim), [true]);
    for (x, y) in [(17, 14), (17, 15), (17, 16)] {
        let c = sim.bridge_state.as_ref().unwrap().cell(x, y).unwrap();
        assert_eq!(c.overlay_byte, 0xce, "Seed0 native MapGen variant");
        assert_eq!(
            c.damage_state,
            crate::sim::bridge_state::DamageState::Destroyed,
            "native stale damage byte"
        );
        assert!(sim.bridge_state.as_ref().unwrap().is_bridge_walkable(x, y));
    }
}

#[test]
fn engineer_at_intact_cabhut_emits_sound_no_mutation() {
    let (mut sim, rules, registry, hut) = ready_repair_fixture(Some(205));
    let engineer = ready_engineer(&mut sim, &rules, &registry, hut);
    assert!(!repair_frame(&mut sim, &rules, &registry).bridge_state_changed);
    assert!(sim.substrate.entities.get(engineer).is_none());
    assert_eq!(repair_sounds(&sim), [true]);
}

#[test]
fn engineer_far_from_bridge_at_cabhut_no_mutation() {
    let (mut sim, rules, registry, hut) = ready_repair_fixture(None);
    let engineer = ready_engineer(&mut sim, &rules, &registry, hut);
    assert!(!repair_frame(&mut sim, &rules, &registry).bridge_state_changed);
    assert!(sim.substrate.entities.get(engineer).is_none());
    assert_eq!(repair_sounds(&sim), [true]);
}

#[test]
fn bridge_repair_preserves_unrelated_foundation_before_next_reader() {
    let (mut sim, rules, registry, hut) = ready_repair_fixture(Some(231));
    let unrelated = sim
        .spawn_object("CABHUT", "Soviets", 13, 13, 0, &rules, &BTreeMap::new())
        .unwrap();
    let engineer = ready_engineer(&mut sim, &rules, &registry, hut);
    assert!(sim.rebuild_dynamic_navigation(&rules));
    let pinned = sim.path_grid_snapshot().unwrap();
    assert!(!pinned.is_walkable(13, 13));
    let gameplay = (sim.scenario_rng.state(), sim.main_rng.state());
    let mut mapgen = sim.mapgen_rng.clone();
    mapgen.next_range_u32_inclusive_scaled(0, 3);
    assert!(sim.run_completed_walk_step(
        engineer,
        crate::sim::components::DriveCoord::cell(16, 15, 0),
        Some(&rules),
        None,
        Some(&registry)
    ));
    assert!(
        !sim.substrate
            .entities
            .get(engineer)
            .unwrap()
            .lifecycle
            .object_alive,
        "Uninit before frame-tail drain"
    );
    assert!(sim.substrate.occupancy.contains_entity(13, 13, unrelated));
    assert!(!sim.path_grid().unwrap().is_walkable(13, 13));
    assert!(!pinned.is_walkable(13, 13));
    assert_eq!(gameplay, (sim.scenario_rng.state(), sim.main_rng.state()));
    assert_eq!(sim.mapgen_rng.state(), mapgen.state());
    for (x, y) in [(17, 14), (17, 15), (17, 16)] {
        assert!(sim.bridge_state.as_ref().unwrap().is_bridge_walkable(x, y));
    }
}

#[test]
fn consecutive_engineers_cancel_the_successors_hut_target_before_its_next_turn() {
    let (mut sim, rules, registry, hut) = ready_repair_fixture(Some(231));
    let a = ready_engineer(&mut sim, &rules, &registry, hut);
    let b = ready_engineer(&mut sim, &rules, &registry, hut);
    repair_frame(&mut sim, &rules, &registry);
    assert!(sim.substrate.entities.get(a).is_none());
    assert!(
        sim.substrate.entities.get(b).is_some(),
        "live vector removal skips its immediate successor"
    );
    let successor = sim.substrate.entities.get(b).unwrap();
    assert_eq!(successor.navigation.nav_com, None);
    assert!(successor.locomotor.as_ref().unwrap().step_head().is_some());
    assert_eq!(repair_sounds(&sim), [true]);
    assert_eq!(sim.radar_events.len(), 1);
    assert!(sim.radar_terrain_dirty_generation > 0);
    for p in [(17, 14), (17, 15), (17, 16)] {
        assert!(sim.radar_terrain_dirty_cells.contains(&p));
    }
    repair_frame(&mut sim, &rules, &registry);
    let successor = sim.substrate.entities.get(b).unwrap();
    assert!(successor.lifecycle.object_alive);
    assert_eq!(successor.navigation.nav_com, None);
    assert_eq!(successor.locomotor.as_ref().unwrap().step_head(), None);
    assert_eq!(repair_sounds(&sim), [true]);
}

#[test]
fn nonconsecutive_engineer_finishes_its_head_without_repeating_cancelled_repair() {
    let (mut sim, rules, registry, hut) = ready_repair_fixture(Some(231));
    let a = ready_engineer(&mut sim, &rules, &registry, hut);
    let separator = sim
        .spawn_object("ENGINEER", "Americans", 19, 15, 0, &rules, &BTreeMap::new())
        .unwrap();
    let b = ready_engineer(&mut sim, &rules, &registry, hut);
    repair_frame(&mut sim, &rules, &registry);
    assert!(sim.substrate.entities.get(a).is_none());
    assert!(sim.substrate.entities.get(separator).is_some());
    let successor = sim.substrate.entities.get(b).unwrap();
    assert!(successor.lifecycle.object_alive);
    assert_eq!(successor.navigation.nav_com, None);
    assert_eq!(successor.locomotor.as_ref().unwrap().step_head(), None);
    assert_eq!(repair_sounds(&sim), [true]);
    assert_eq!(sim.radar_events.len(), 1);
}

#[test]
fn repair_pointer_expiry_uses_descending_infantry_registry_and_preserves_paid_heads() {
    use crate::sim::components::NavTargetRef;
    let (mut sim, rules, registry, hut) = ready_repair_fixture(None);
    let a = ready_engineer(&mut sim, &rules, &registry, hut);
    let b = ready_engineer(&mut sim, &rules, &registry, hut);
    let c = ready_engineer(&mut sim, &rules, &registry, hut);
    let mut expected = sim.scenario_rng.clone();
    let mut delays = BTreeMap::new();
    let mut locomotors = BTreeMap::new();
    for id in [c, b, a] {
        delays.insert(id, expected.next_range_u32_inclusive(4, 8));
        let e = sim.substrate.entities.get_mut(id).unwrap();
        e.attack_target = Some(crate::sim::combat::AttackTarget::new(hut));
        e.passive_scan_timer.arm(sim.session.binary_frame, 30);
        e.navigation.nav_com_aux = Some(NavTargetRef::Cell { rx: 19, ry: 19 });
        e.navigation
            .nav_queue
            .push(NavTargetRef::Building { id: hut });
        e.navigation
            .nav_queue
            .push(NavTargetRef::Cell { rx: 16, ry: 15 });
        e.mark_live_contact_with(hut);
        locomotors.insert(id, serde_json::to_value(&e.locomotor).unwrap());
    }
    // Registry membership survives Limbo and is independent of Logic's list.
    sim.substrate
        .entities
        .get_mut(c)
        .unwrap()
        .lifecycle
        .in_limbo = true;
    // The hut itself is not an Infantry receiver even if it holds the pointer.
    sim.substrate
        .entities
        .get_mut(hut)
        .unwrap()
        .navigation
        .nav_com = Some(NavTargetRef::Building { id: hut });
    sim.expire_infantry_bridge_hut_targets(hut);
    assert_eq!(sim.scenario_rng.state(), expected.state());
    for id in [a, b, c] {
        let e = sim.substrate.entities.get(id).unwrap();
        assert_eq!(e.passive_scan_timer.duration, delays[&id]);
        assert!(e.attack_target.is_none());
        assert_eq!(e.navigation.nav_com, None);
        assert_eq!(e.navigation.nav_com_aux, None, "native clears the pair");
        assert_eq!(
            e.navigation.nav_queue,
            [NavTargetRef::Cell { rx: 16, ry: 15 }]
        );
        assert_eq!(serde_json::to_value(&e.locomotor).unwrap(), locomotors[&id]);
        assert!(
            e.has_live_contact_with(hut),
            "control0 keeps radio contacts"
        );
    }
    assert_eq!(
        sim.substrate.entities.get(hut).unwrap().navigation.nav_com,
        Some(NavTargetRef::Building { id: hut })
    );
}

#[test]
fn repair_pointer_expiry_keeps_sensor_and_occupier_exceptions_and_current_nav_gate() {
    use crate::sim::components::NavTargetRef;
    for exception in 0..3 {
        let (mut sim, rules, registry, hut) = ready_repair_fixture(None);
        let id = ready_engineer(&mut sim, &rules, &registry, hut);
        let owner = sim.substrate.entities.get(id).unwrap().owner();
        if exception == 0 {
            sim.fog.width = 33;
            sim.fog.height = 33;
            sim.fog.increment_sensor_at(owner, 16, 15);
        }
        let e = sim.substrate.entities.get_mut(id).unwrap();
        e.occupier = exception == 1;
        if exception == 2 {
            e.navigation.nav_com = Some(NavTargetRef::Cell { rx: 16, ry: 15 });
        }
        e.navigation.nav_com_aux = Some(NavTargetRef::Building { id: hut });
        let before = e.navigation.clone();
        sim.expire_infantry_bridge_hut_targets(hut);
        let e = sim.substrate.entities.get(id).unwrap();
        assert_eq!(
            e.navigation.nav_com, before.nav_com,
            "exception {exception}"
        );
        assert_eq!(
            e.navigation.nav_com_aux, before.nav_com_aux,
            "exception {exception}"
        );
    }
}

#[test]
fn engineer_adjacent_to_cabhut_enters_before_repairing_and_dirtying_minimap() {
    let (mut sim, rules, registry, hut) = ready_repair_fixture(Some(231));
    let id = sim
        .spawn_object("ENGINEER", "Americans", 15, 15, 0, &rules, &BTreeMap::new())
        .unwrap();
    let grid = sim.path_grid_snapshot();
    assert!(sim.apply_command(
        "Americans",
        &Command::CaptureBuilding {
            engineer_id: id,
            target_building_id: hut
        },
        Some(&rules),
        grid.as_deref(),
        &BTreeMap::new()
    ));
    drop(grid);
    assert!(!repair_frame(&mut sim, &rules, &registry).bridge_state_changed);
    assert!(sim.substrate.entities.get(id).is_some());
    assert!(repair_sounds(&sim).is_empty());
    assert!(sim.radar_terrain_dirty_cells.is_empty());
    let mut changed = false;
    for _ in 0..100 {
        changed |= repair_frame(&mut sim, &rules, &registry).bridge_state_changed;
        if sim.substrate.entities.get(id).is_none() {
            break;
        }
    }
    assert!(changed);
    assert!(sim.substrate.entities.get(id).is_none());
    assert_eq!(repair_sounds(&sim), [true]);
    assert!(sim.radar_terrain_dirty_generation > 0);
    for p in [(17, 14), (17, 15), (17, 16)] {
        assert!(sim.radar_terrain_dirty_cells.contains(&p));
    }
}

/// Native75ADA0/75ACB0 retain the head even after its cell was entered;
///75AEC0 consumes that head before consulting the replacement destination.
#[test]
fn walk_stop_and_retarget_finish_a_same_cell_committed_head() {
    use crate::sim::movement::ground_pose;
    use crate::sim::snapshot::GameSnapshot;
    for stop in [true, false] {
        let (mut sim, rules, registry) = fixture();
        let id = sim
            .spawn_object("ENGINEER", "Americans", 15, 15, 0, &rules, &BTreeMap::new())
            .unwrap();
        let grid = sim.path_grid_snapshot();
        assert!(sim.apply_command(
            "Americans",
            &Command::Move {
                entity_id: id,
                target_rx: 18,
                target_ry: 15,
                queue: false,
                group_id: None
            },
            Some(&rules),
            grid.as_deref(),
            &BTreeMap::new()
        ));
        drop(grid);
        let mut retained = None;
        for _ in 0..120 {
            repair_frame(&mut sim, &rules, &registry);
            let e = sim.substrate.entities.get(id).unwrap();
            if let Some(head) = e.locomotor.as_ref().unwrap().step_head()
                && (head.x / 256, head.y / 256)
                    == (i32::from(e.position.rx), i32::from(e.position.ry))
                && crate::util::native_x87::distance_3d_leptons(
                    {
                        let c = ground_pose::position_world_coord(&e.position);
                        [c.x, c.y, 0]
                    },
                    [head.x, head.y, 0],
                ) >= 17
            {
                retained = Some(head);
                break;
            }
        }
        let head = retained.expect("enter head cell before subcell completion");
        let before =
            ground_pose::position_world_coord(&sim.substrate.entities.get(id).unwrap().position);
        let grid = sim.path_grid_snapshot();
        let order = if stop {
            Command::Stop { entity_id: id }
        } else {
            Command::Move {
                entity_id: id,
                target_rx: 18,
                target_ry: 16,
                queue: false,
                group_id: None,
            }
        };
        assert!(sim.apply_command(
            "Americans",
            &order,
            Some(&rules),
            grid.as_deref(),
            &BTreeMap::new()
        ));
        drop(grid);
        let e = sim.substrate.entities.get(id).unwrap();
        assert_eq!(ground_pose::position_world_coord(&e.position), before);
        assert_eq!(e.locomotor.as_ref().unwrap().step_head(), Some(head));
        assert_eq!(e.movement_target.as_ref().unwrap().next_index, 0);
        assert_eq!(e.navigation.nav_com.is_none(), stop);
        // Both instance-owned XYZ values survive the actual snapshot envelope.
        let bytes = GameSnapshot::save(&sim, 0, 0, "walk retained order", 0);
        let mut replay = GameSnapshot::load(&bytes).unwrap().sim;
        replay.restore_after_snapshot_load().unwrap();
        replay.resolve_type_handles(&rules);
        replay.resolved_terrain = sim.resolved_terrain.clone();
        // Scenario reload applies native Seed(0); compare both continuations
        // under that established load rule, not uninterrupted RNG preservation.
        sim.scenario_rng = crate::sim::rng::SimRng::new(0);
        let mut retired = false;
        for _ in 0..240 {
            repair_frame(&mut sim, &rules, &registry);
            repair_frame(&mut replay, &rules, &registry);
            assert_eq!(sim.state_hash(), replay.state_hash());
            let e = sim.substrate.entities.get(id).unwrap();
            if e.locomotor.as_ref().unwrap().step_head() != Some(head) {
                retired = true;
            }
            if retired && e.movement_target.is_none() {
                break;
            }
        }
        let e = sim.substrate.entities.get(id).unwrap();
        assert!(retired && e.movement_target.is_none());
        assert!(e.locomotor.as_ref().unwrap().step_head().is_none());
        assert!(e.navigation.nav_com.is_none());
        if stop {
            assert_eq!(ground_pose::position_world_coord(&e.position), head);
        } else {
            assert_eq!((e.position.rx, e.position.ry), (18, 16));
        }
    }
}

#[test]
fn walk_completion_uses_retained_destination_and_exact_height_tolerance() {
    use crate::sim::components::{DriveCoord, NavTargetRef};
    //75BE6F reads live class fields after PerCell. A changed destination remains
    //authoritative even when the old A* adapter reports its last node complete.
    for (dest, survives) in [
        (
            DriveCoord {
                x: 17 * 256 + 128,
                y: 15 * 256 + 128,
                z: 0,
            },
            true,
        ),
        (
            DriveCoord {
                x: 16 * 256 + 128,
                y: 15 * 256 + 128,
                z: 208,
            },
            true,
        ),
        (
            DriveCoord {
                x: 16 * 256 + 128,
                y: 15 * 256 + 128,
                z: 207,
            },
            false,
        ),
    ] {
        let (mut sim, rules, registry) = fixture();
        let id = sim
            .spawn_object("ENGINEER", "Americans", 15, 15, 0, &rules, &BTreeMap::new())
            .unwrap();
        let head = DriveCoord {
            x: 16 * 256 + 192,
            y: 15 * 256 + 64,
            z: 0,
        };
        let e = sim.substrate.entities.get_mut(id).unwrap();
        e.navigation.nav_com = Some(NavTargetRef::cell(
            (dest.x / 256) as u16,
            (dest.y / 256) as u16,
        ));
        e.locomotor
            .as_mut()
            .unwrap()
            .set_walk_destination(Some(dest));
        e.locomotor.as_mut().unwrap().set_step_head(Some(head));
        sim.run_completed_walk_step(id, head, Some(&rules), None, Some(&registry));
        let e = sim.substrate.entities.get(id).unwrap();
        assert_eq!(e.navigation.nav_com.is_some(), survives);
        assert_eq!(
            e.locomotor.as_ref().unwrap().walk_destination().is_some(),
            survives
        );
    }
}
