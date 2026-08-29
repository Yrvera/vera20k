use super::*;
use std::collections::HashMap;

use crate::map::actions::MapAction;
use crate::map::entities::EntityCategory;
use crate::map::events::MapEvent;
use crate::map::trigger_graph::build_trigger_graph;
use crate::map::trigger_program::TriggerProgram;
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

fn compile_program(body: &str) -> (crate::rules::ini_parser::IniFile, TriggerProgram) {
    use crate::map::{actions, events, tags, triggers};
    let ini = crate::rules::ini_parser::IniFile::from_str(body);
    let program = TriggerProgram::compile(
        &ini,
        &tags::parse_tags(&ini),
        &triggers::parse_triggers(&ini),
        &events::parse_events(&ini),
        &actions::parse_actions(&ini),
    )
    .expect("typed trigger program");
    (ini, program)
}

#[test]
fn native_executor_keeps_unowned_trigger_types_inert() {
    let (_, program) = compile_program(
        "[Triggers]\nT=Neutral,<none>,T,1,1,1,1,0\n\
         [Events]\nT=1,8,0,0\n\
         [Actions]\nT=1,28,0,7,0,0,0,0,A\n",
    );
    let mut sim = Simulation::new();
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan::default(),
        1,
        0,
        &mut sim.scenario_rng,
    );

    assert!(runtime.tags.is_empty());
    assert!(runtime.trigger_instances.is_empty());
    assert!(runtime
        .advance_native_poll(&program, &mut sim, None, None, &HashMap::new())
        .is_empty());
    assert!(runtime.globals_set.is_empty());
}

#[test]
fn false_event_one_and_events_49_50_do_not_create_persistence_or_owner() {
    let (_, program) = compile_program(
        "[Triggers]\nT=Neutral,<none>,T,1,1,1,1,0\n\
         [Tags]\nG=1,G,T\n\
         [Events]\nT=3,1,0,-1,49,0,0,50,0,0\n",
    );
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("OWNER");
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan {
            object_tag_types: vec![(10, 0), (11, 0)],
            ..Default::default()
        },
        1,
        0,
        &mut sim.scenario_rng,
    );
    let instance = runtime.tags[0].trigger_instances[0];

    for event_id in [49, 50] {
        let (fired, _) = runtime.dispatch_native_event(
            &program,
            &mut sim,
            None,
            None,
            &HashMap::new(),
            0,
            NativeTriggerEvent {
                event_id,
                object_id: Some(10),
                cell: None,
                raising_owner: Some(owner),
                data: 0,
                editor_mode: false,
            },
        );
        assert!(!fired);
    }
    assert_eq!(runtime.trigger_instances[instance as usize].satisfied_mask, 0);
    assert_eq!(
        runtime.trigger_types[0].event_last_raising_owners,
        vec![None, None, None]
    );
    assert_eq!(runtime.tags[0].attachment_count, 2);
}

#[test]
fn successful_event_one_owner_is_shared_across_distinct_tags() {
    let (_, program) = compile_program(
        "[Triggers]\nT=Neutral,<none>,T,1,1,1,1,0\n\
         [Tags]\nA=1,A,T\nB=1,B,T\n\
         [Events]\nT=1,1,0,-1\n",
    );
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("OWNER");
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan {
            object_tag_types: vec![(10, 0), (11, 0), (20, 1), (21, 1)],
            ..Default::default()
        },
        1,
        0,
        &mut sim.scenario_rng,
    );
    let event = |object_id, raising_owner| NativeTriggerEvent {
        event_id: 1,
        object_id: Some(object_id),
        cell: None,
        raising_owner,
        data: -1,
        editor_mode: false,
    };

    let _ = runtime.dispatch_native_event(
        &program,
        &mut sim,
        None,
        None,
        &HashMap::new(),
        0,
        event(10, Some(owner)),
    );
    let _ = runtime.dispatch_native_event(
        &program,
        &mut sim,
        None,
        None,
        &HashMap::new(),
        1,
        event(20, None),
    );

    let a_instance = runtime.tags[0].trigger_instances[0] as usize;
    let b_instance = runtime.tags[1].trigger_instances[0] as usize;
    assert_eq!(runtime.trigger_types[0].event_last_raising_owners, vec![Some(owner)]);
    assert_eq!(runtime.trigger_instances[a_instance].raising_house, Some(owner));
    assert_eq!(runtime.trigger_instances[b_instance].raising_house, Some(owner));
    assert_eq!(runtime.tags[0].attachment_count, 1);
    assert_eq!(runtime.tags[1].attachment_count, 1);
}

#[test]
fn variable_write_rearms_only_instances_referencing_that_exact_index() {
    let (_, program) = compile_program(
        "[Triggers]\nW=Neutral,<none>,W,1,1,1,1,0\n\
         M=Neutral,<none>,M,1,1,1,1,0\n\
         U=Neutral,<none>,U,1,1,1,1,0\n\
         [Tags]\nW_TAG=2,W,W\nM_TAG=2,M,M\nU_TAG=2,U,U\n\
         [Events]\nW=1,8,0,0\nM=2,27,0,3,51,0,7\nU=2,27,0,4,51,0,7\n\
         [Actions]\nW=1,28,0,3,0,0,0,0,A\n",
    );
    let mut sim = Simulation::new();
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan::default(),
        1,
        0,
        &mut sim.scenario_rng,
    );
    let mut expected = sim.scenario_rng.clone();
    let _ = expected.next_range_i32_inclusive(0, 7);

    let _ = runtime.advance_native_poll(&program, &mut sim, None, None, &HashMap::new());
    assert!(runtime.globals_set.contains(&3));
    assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
    let after_change = sim.scenario_rng.logical_state();
    let _ = runtime.advance_native_poll(&program, &mut sim, None, None, &HashMap::new());
    assert_eq!(sim.scenario_rng.logical_state(), after_change);
}

#[test]
fn repeat_zero_one_two_own_distinct_detach_and_expiry_paths() {
    let (_, program) = compile_program(
        "[Triggers]\nT0=Neutral,<none>,T0,1,1,1,1,0\n\
         T1=Neutral,<none>,T1,1,1,1,1,0\n\
         T2=Neutral,<none>,T2,1,1,1,1,0\n\
         [Tags]\nR0=0,R0,T0\nR1=1,R1,T1\nR2=2,R2,T2\n\
         [Events]\nT0=1,8,0,0\nT1=1,8,0,0\nT2=1,8,0,0\n\
         [Actions]\nT0=1,28,0,0,0,0,0,0,A\n\
         T1=1,28,0,1,0,0,0,0,A\n\
         T2=1,28,0,2,0,0,0,0,A\n",
    );
    let mut sim = Simulation::new();
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan {
            object_tag_types: vec![(10, 0), (20, 1), (21, 1), (30, 2)],
            ..Default::default()
        },
        1,
        0,
        &mut sim.scenario_rng,
    );
    let event = |object_id| NativeTriggerEvent {
        event_id: 13,
        object_id: Some(object_id),
        cell: None,
        raising_owner: None,
        data: 0,
        editor_mode: false,
    };

    assert!(runtime
        .dispatch_native_event(&program, &mut sim, None, None, &HashMap::new(), 0, event(10))
        .0);
    assert!(!runtime.tags[0].registered);
    assert!(!runtime.object_tags.contains_key(&10));
    assert!(runtime.globals_set.contains(&0));

    assert!(!runtime
        .dispatch_native_event(&program, &mut sim, None, None, &HashMap::new(), 1, event(20))
        .0);
    assert_eq!(runtime.tags[1].attachment_count, 1);
    assert!(!runtime.globals_set.contains(&1));
    assert!(runtime
        .dispatch_native_event(&program, &mut sim, None, None, &HashMap::new(), 1, event(21))
        .0);
    assert!(!runtime.tags[1].registered);
    assert!(runtime.globals_set.contains(&1));

    assert!(runtime
        .dispatch_native_event(&program, &mut sim, None, None, &HashMap::new(), 2, event(30))
        .0);
    assert!(runtime.tags[2].registered);
    assert_eq!(runtime.object_tags.get(&30), Some(&2));
    assert!(runtime.globals_set.contains(&2));
}

#[test]
fn polling_erase_skip_and_late_finalizer_stably_remap_every_registry() {
    let (_, program) = compile_program(
        "[Triggers]\nA=Neutral,<none>,A,1,1,1,1,0\n\
         B=Neutral,<none>,B,1,1,1,1,0\n\
         C=Neutral,<none>,C,1,1,1,1,0\n\
         [Tags]\nA_TAG=0,A,A\nB_TAG=0,B,B\nC_TAG=0,C,C\n\
         [Events]\nA=1,8,0,0\nB=1,8,0,0\nC=1,8,0,0\n",
    );
    let mut sim = Simulation::new();
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan::default(),
        1,
        0,
        &mut sim.scenario_rng,
    );

    let _ = runtime.advance_native_poll(&program, &mut sim, None, None, &HashMap::new());
    assert_eq!(runtime.polling_tags, vec![1]);
    assert_eq!(runtime.pending_tag_finalization, vec![0, 2]);
    assert!(!runtime.tags[0].registered);
    assert!(runtime.tags[1].registered);
    assert!(!runtime.tags[2].registered);

    runtime.finalize_pending_tags();
    assert_eq!(runtime.tags.len(), 1);
    assert_eq!(runtime.trigger_instances.len(), 1);
    assert_eq!(runtime.polling_tags, vec![0]);
    assert_eq!(runtime.tag_by_type, vec![None, Some(0), None]);
    assert_eq!(runtime.tags[0].trigger_instances, vec![0]);
    assert!(runtime.pending_tag_finalization.is_empty());
}

#[test]
fn expiring_reusable_tag_never_promotes_team_no_reuse_duplicate() {
    let (_, program) = compile_program(
        "[Triggers]\nT=Neutral,<none>,T,1,1,1,1,0\n\
         [Tags]\nG=0,G,T\n\
         [Events]\nT=1,8,0,0\n",
    );
    let mut sim = Simulation::new();
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan::default(),
        1,
        0,
        &mut sim.scenario_rng,
    );
    let team = runtime.materialize_team_tag(
        &program,
        0,
        1,
        sim.session.binary_frame,
        &mut sim.scenario_rng,
    );
    assert_eq!(team, 1);
    assert!(!runtime.tags[team as usize].reusable);

    assert!(runtime
        .dispatch_native_event(
            &program,
            &mut sim,
            None,
            None,
            &HashMap::new(),
            0,
            NativeTriggerEvent::polling(13, 0),
        )
        .0);
    assert_eq!(runtime.tag_by_type[0], None);
    runtime.finalize_pending_tags();
    assert_eq!(runtime.tags.len(), 1);
    assert!(!runtime.tags[0].reusable);
    assert_eq!(runtime.tag_by_type[0], None);
}

#[test]
fn action_22_springs_all_matching_instances_without_conditions_or_repeat_cleanup() {
    let (_, program) = compile_program(
        "[Triggers]\nT=Neutral,<none>,T,1,1,1,1,0\n\
         C=Neutral,<none>,C,1,1,1,1,0\n\
         [Tags]\nTARGET=0,Target,T\nCONTROLLER=2,Controller,C\n\
         [Events]\nT=1,1,0,-1\nC=1,8,0,0\n\
         [Actions]\nT=1,28,0,9,0,0,0,0,A\n\
         C=1,22,0,T,0,0,0,0,A\n",
    );
    let mut sim = Simulation::new();
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan {
            object_tag_types: vec![(10, 0)],
            ..Default::default()
        },
        1,
        0,
        &mut sim.scenario_rng,
    );
    let target_instance = runtime.tags[0].trigger_instances[0] as usize;

    let _ = runtime.advance_native_poll(&program, &mut sim, None, None, &HashMap::new());
    assert!(runtime.globals_set.contains(&9));
    assert!(!runtime.trigger_instances[target_instance].pending_delete);
    assert!(runtime.tags[0].registered);
    assert_eq!(runtime.object_tags.get(&10), Some(&0));
}

#[test]
fn actions_53_and_54_scan_pending_instances_with_exact_timer_and_rng_rules() {
    let (_, program) = compile_program(
        "[Triggers]\nT=Neutral,<none>,T,0,1,0,0,0\n\
         E=Neutral,<none>,E,1,1,1,1,0\n\
         D=Neutral,<none>,D,1,1,1,1,0\n\
         [Tags]\nENABLE=2,Enable,E\nDISABLE=2,Disable,D\nX=1,X,T\nY=1,Y,T\n\
         [Events]\nT=2,27,0,5,51,0,7\nE=1,8,0,0\nD=1,8,0,0\n\
         [Actions]\nE=1,53,0,T,0,0,0,0,A\n\
         D=1,54,0,T,0,0,0,0,A\n",
    );
    let mut sim = Simulation::new();
    sim.session.trigger_difficulty_raw = 0;
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan {
            object_tag_types: vec![(10, 2), (20, 3)],
            ..Default::default()
        },
        0,
        0,
        &mut sim.scenario_rng,
    );
    let targets = runtime
        .trigger_instances
        .iter()
        .enumerate()
        .filter_map(|(index, instance)| (instance.trigger_type_index == 0).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 2);
    runtime.trigger_instances[targets[0]].pending_delete = true;
    for &index in &targets {
        runtime.trigger_instances[index].satisfied_mask = u32::MAX;
        assert!(!runtime.trigger_instances[index].enabled);
    }
    let mut expected = sim.scenario_rng.clone();
    let _ = expected.next_range_i32_inclusive(0, 7);
    let _ = expected.next_range_i32_inclusive(0, 7);

    let _ = runtime.advance_native_poll(&program, &mut sim, None, None, &HashMap::new());
    assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
    for &index in &targets {
        let instance = &runtime.trigger_instances[index];
        assert!(!instance.enabled, "Action54 must run after Action53");
        assert_eq!(instance.satisfied_mask & 1, 0);
    }
    assert!(runtime.trigger_instances[targets[0]].pending_delete);
}

#[test]
fn fresh_materialization_preserves_reuse_and_all_three_native_orders() {
    let (_, program) = compile_program(
        "[Triggers]\n\
         T0=Neutral,T1,T0,1,1,1,1,0\n\
         T1=Neutral,<none>,T1,1,1,1,1,0\n\
         TB=Neutral,<none>,TB,1,1,1,1,0\n\
         TA=Neutral,<none>,TA,1,1,1,1,0\n\
         [Tags]\n\
         Z=0,Z,T0\n\
         A=0,A,TA\n\
         B=0,B,TB\n\
         C=0,C,T0\n\
         D=0,D,<none>\n\
         [Events]\n\
         T0=1,29,0,0\n\
         TB=1,13,0,3\n\
         TA=1,8,0,0\n",
    );
    let plan = TriggerAttachmentPlan {
        // B is first and the repeated Cell setter overwrites Tag+0x30.
        cell_tag_types: vec![(2, (4, 5)), (2, (6, 7))],
        // Z then C share TriggerType definitions but own distinct instances.
        object_tag_types: vec![(10, 0), (11, 0), (12, 3)],
    };
    let mut rng = SimRng::new(9);
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &plan,
        1,
        77,
        &mut rng,
    );

    assert_eq!(
        runtime
            .tags
            .iter()
            .map(|tag| program.tag_types[tag.tag_type_index as usize].id.as_str())
            .collect::<Vec<_>>(),
        vec!["B", "Z", "C", "A"]
    );
    assert_eq!(runtime.tags[0].attachment_count, 2);
    assert_eq!(runtime.tags[0].attached_cell, Some((6, 7)));
    assert_eq!(runtime.tags[1].attachment_count, 2);
    assert_eq!(runtime.tags[2].attachment_count, 1);
    assert_eq!(runtime.object_tags.get(&10), Some(&1));
    assert_eq!(runtime.object_tags.get(&11), Some(&1));
    assert_eq!(runtime.object_tags.get(&12), Some(&2));
    assert_eq!(
        runtime.tag_by_type,
        vec![Some(1), Some(3), Some(0), Some(2), None]
    );

    assert_eq!(
        runtime
            .trigger_instances
            .iter()
            .map(|instance| {
                program.trigger_types[instance.trigger_type_index as usize]
                    .id
                    .as_str()
            })
            .collect::<Vec<_>>(),
        vec!["TB", "T0", "T1", "T0", "T1", "TA"]
    );
    assert_eq!(runtime.tags[1].trigger_instances, vec![2, 1]);
    assert_eq!(runtime.trigger_instances[2].next, Some(1));
    assert_eq!(runtime.tags[2].trigger_instances, vec![4, 3]);
    assert_eq!(runtime.destroyed_event_tags, vec![3]);
    assert_eq!(runtime.polling_tags, vec![3, 0]);
    assert_eq!(runtime.proximity_event_tags, vec![3, 0]);
    assert_eq!(runtime.cell_tags.get(&(4, 5)), Some(&0));
    assert_eq!(runtime.cell_tags.get(&(6, 7)), Some(&0));
    assert_eq!(runtime.trigger_types[0].event_last_raising_owners.len(), 1);
    assert_eq!(runtime.trigger_instances[1].trigger_type_index, 0);
    assert_eq!(runtime.trigger_instances[3].trigger_type_index, 0);
    let shared_owner = InternedId::from_index(9);
    runtime.trigger_types[0].event_last_raising_owners[0] = Some(shared_owner);
    assert_eq!(
        runtime.trigger_types[runtime.trigger_instances[1].trigger_type_index as usize]
            .event_last_raising_owners[0],
        Some(shared_owner)
    );
    assert_eq!(
        runtime.trigger_types[runtime.trigger_instances[3].trigger_type_index as usize]
            .event_last_raising_owners[0],
        Some(shared_owner)
    );

    let team_tag = runtime.materialize_team_tag(&program, 0, 1, 77, &mut rng);
    assert_eq!(team_tag, 4);
    assert_eq!(
        runtime.tag_by_type[0],
        Some(1),
        "Team must not replace reuse owner"
    );
    assert_eq!(
        runtime.tags[team_tag as usize].trigger_instances,
        vec![7, 6]
    );
}

#[test]
fn timer_construction_spends_rng_before_explicit_difficulty_gate() {
    let (_, program) = compile_program(
        "[Triggers]\nT=Neutral,<none>,T,1,0,1,0,0\n\
         [Tags]\nG=0,G,T\n\
         [Events]\nT=3,13,0,5,51,0,7,13,0,9\n",
    );
    let plan = TriggerAttachmentPlan {
        object_tag_types: vec![(1, 0)],
        ..Default::default()
    };
    let mut rng = SimRng::new(0x1234);
    let mut expected = rng.clone();
    let _ = expected.next_range_i32_inclusive(0, 7);
    let runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &plan,
        0,
        0x8000_0001,
        &mut rng,
    );
    let instance = &runtime.trigger_instances[0];
    assert_eq!(rng.logical_state(), expected.logical_state());
    assert!(!instance.enabled, "raw Easy uses the explicit Easy flag");
    assert_eq!(instance.timer_start_frame, 0x8000_0001u32 as i32);
    // Runtime Event order is reverse textual order: 13(9), 51(7), 13(5).
    assert_eq!(instance.timer_duration, 75);
    assert_eq!(instance.opaque_timer_word, 0);

    for (raw, enabled) in [(1, true), (2, false), (-1, true), (3, true)] {
        let mut candidate_rng = SimRng::new(0x1234);
        let candidate = TriggerRuntime::materialize_fresh(
            &program,
            &LocalVariableMap::new(),
            &plan,
            raw,
            0,
            &mut candidate_rng,
        );
        assert_eq!(candidate.trigger_instances[0].enabled, enabled, "raw {raw}");
    }
}

#[test]
fn raw_celltag_plan_is_source_ordered_first_successful_and_object_order_is_spawned() {
    use crate::map::map_file::MapCell;
    use crate::map::rmg::{emit::empty_map_file, options::RmgOptions};

    let body = "[Triggers]\nTA=Neutral,<none>,TA,1,1,1,1,0\nTB=Neutral,<none>,TB,1,1,1,1,0\n\
                [Tags]\nA=0,A,TA\nB=0,B,TB\n\
                [CellTags]\n0001001=MISSING\n1001=A\n1002=B\n1003=A\n";
    let (ini, program) = compile_program(body);
    let mut map = empty_map_file(&RmgOptions::default(), 2, 2);
    map.ini = ini;
    map.cells = vec![
        MapCell {
            rx: 1,
            ry: 1,
            tile_index: 0,
            sub_tile: 0,
            z: 0,
        },
        MapCell {
            rx: 2,
            ry: 1,
            tile_index: 0,
            sub_tile: 0,
            z: 0,
        },
    ];
    let mut sim = Simulation::new();
    let later_id = spawn_type(&mut sim, "LATER");
    let earlier_id = spawn_type(&mut sim, "EARLIER");
    let tag_b = sim.interner.intern("B");
    let tag_a = sim.interner.intern("A");
    sim.substrate
        .entities
        .get_mut(later_id)
        .unwrap()
        .attached_tag_id = Some(tag_b);
    sim.substrate
        .entities
        .get_mut(earlier_id)
        .unwrap()
        .attached_tag_id = Some(tag_a);

    let plan = TriggerAttachmentPlan::from_loaded_map(&program, &map, &sim);
    assert_eq!(plan.cell_tag_types, vec![(0, (1, 1)), (1, (2, 1))]);
    assert_eq!(plan.object_tag_types, vec![(later_id, 1), (earlier_id, 0)]);
}

#[test]
fn mutable_native_runtime_state_hashes_but_opaque_residue_does_not() {
    use std::collections::hash_map::DefaultHasher;

    let (_, program) =
        compile_program("[Triggers]\nT=Neutral,<none>,T,1,1,1,1,0\n[Tags]\nG=0,G,T\n");
    let mut rng = SimRng::new(1);
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan {
            object_tag_types: vec![(1, 0)],
            ..Default::default()
        },
        1,
        0,
        &mut rng,
    );
    let hash = |runtime: &TriggerRuntime| {
        let mut hasher = DefaultHasher::new();
        runtime.hash_state(&mut hasher);
        hasher.finish()
    };
    let baseline = hash(&runtime);
    runtime.trigger_instances[0].opaque_timer_word = i32::MIN;
    assert_eq!(hash(&runtime), baseline);
    runtime.trigger_instances[0].satisfied_mask = 1;
    assert_ne!(hash(&runtime), baseline);
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
                waypoint_index: Some(0),
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
            program: None,
            graph: &graph,
            triggers: &triggers,
            events: &events,
            actions: &actions,
            waypoints: &HashMap::new(),
            rules: None,
            overlay_registry: None,
            local_player_owner: None,
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
                "J".to_string(),
            ],
            entries: vec![ActionEntry {
                kind: 112,
                // Runtime owns the reader-materialized +0x44 value below;
                // changing retained source text must not trigger re-decoding.
                params: vec![
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "ZZ".to_string(),
                ],
                waypoint_index: Some(9),
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
    let mut sim = Simulation::new();

    assert!(
        runtime
            .advance_at_frame(44, &graph, &triggers, &events, &actions, Some(&mut sim), None)
            .is_empty()
    );
    assert_eq!(
        runtime.advance_at_frame(
            45,
            &graph,
            &triggers,
            &events,
            &actions,
            Some(&mut sim),
            None,
        ),
        vec![TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump {
            target: [0, 30],
        })]
    );
    assert!(
        runtime
            .advance_at_frame(46, &graph, &triggers, &events, &actions, Some(&mut sim), None)
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
                    "J".to_string(),
                ],
                waypoint_index: Some(9),
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
            program: None,
            graph: &graph,
            triggers: &triggers,
            events: &events,
            actions: &actions,
            waypoints: &HashMap::new(),
            rules: None,
            overlay_registry: None,
            local_player_owner: None,
        }),
    );

    assert!(tick.frame_committed);
    assert_eq!(
        sim.take_master_frame_test_trace(),
        vec![
            MasterFrameTestRung::SessionCommands,
            MasterFrameTestRung::Triggers,
            MasterFrameTestRung::TeamScript,
            MasterFrameTestRung::LogicVector,
            MasterFrameTestRung::Houses,
            MasterFrameTestRung::FrameCommit,
            MasterFrameTestRung::PendingDelete,
        ]
    );
    assert_eq!(
        sim.drain_trigger_effects(),
        vec![TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump {
            target: [0, 30],
        })]
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
                waypoint_index: Some(0),
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
    let waypoints = HashMap::new();
    let trigger_inputs = TriggerInputs {
        program: None,
        graph: &graph,
        triggers: &triggers,
        events: &events,
        actions: &actions,
        waypoints: &waypoints,
        rules: None,
        overlay_registry: None,
        local_player_owner: None,
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
                    "D".to_string(),
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
                        waypoint_index: None,
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
                        waypoint_index: None,
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
                        waypoint_index: None,
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
                    "D".to_string(),
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
                        "D".to_string(),
                    ],
                    waypoint_index: Some(3),
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

    assert_eq!(
        runtime.advance_at_frame(
            15,
            &graph,
            &triggers,
            &events,
            &actions,
            Some(&mut sim),
            None,
        ),
        vec![TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump {
            target: [0, 30],
        })]
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
                    waypoint_index: Some(0),
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
                    "E".to_string(),
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
                        "E".to_string(),
                    ],
                    waypoint_index: Some(4),
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

    assert_eq!(
        runtime.advance_at_frame(
            15,
            &graph,
            &triggers,
            &events,
            &actions,
            Some(&mut sim),
            None,
        ),
        vec![TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump {
            target: [0, 30],
        })]
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
                    waypoint_index: None,
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
                    "I".to_string(),
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
                        "I".to_string(),
                    ],
                    waypoint_index: Some(8),
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
fn legacy_result_actions_have_no_inferred_owner_or_result_shortcut() {
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
                    waypoint_index: None,
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
                    waypoint_index: None,
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
        Vec::<TriggerEffect>::new()
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
                    waypoint_index: Some(0),
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
                    "G".to_string(),
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
                        "G".to_string(),
                    ],
                    waypoint_index: Some(6),
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
    let mut sim = Simulation::new();

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
        Vec::<TriggerEffect>::new()
    );
    assert!(runtime.locals_set.contains(&2));
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
        vec![TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump {
            target: [0, 30],
        })]
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
                    "L".to_string(),
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
                        "L".to_string(),
                    ],
                    waypoint_index: Some(11),
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
                    "M".to_string(),
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
                        "M".to_string(),
                    ],
                    waypoint_index: Some(12),
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
            TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump {
                target: [0, 30],
            }),
            TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump {
                target: [0, 30],
            }),
        ]
    );
}

fn run_waypoint_action(
    kind: i32,
    token: Option<&str>,
    trigger_country: Option<&str>,
    sim: &mut Simulation,
    waypoints: &HashMap<u32, crate::map::waypoints::Waypoint>,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> Vec<TriggerEffect> {
    let mut trigger = make_trigger("WAYPOINT_ACTION", None, "Waypoint action", true, false);
    trigger.owner = trigger_country.map(str::to_string);
    trigger.fields[0] = trigger_country.unwrap_or("").to_string();
    let triggers: TriggerMap = [(trigger.id.clone(), trigger)].into_iter().collect();
    let events: EventMap = [(
        "WAYPOINT_ACTION".to_string(),
        MapEvent {
            id: "WAYPOINT_ACTION".to_string(),
            fields: Vec::new(),
            conditions: vec![EventCondition {
                kind: 47,
                params: vec!["0".to_string()],
            }],
        },
    )]
    .into_iter()
    .collect();
    let mut params = vec!["0".to_string(); 6];
    if let Some(token) = token {
        params.push(token.to_string());
    }
    let actions: ActionMap = [(
        "WAYPOINT_ACTION".to_string(),
        MapAction {
            id: "WAYPOINT_ACTION".to_string(),
            fields: Vec::new(),
            entries: vec![ActionEntry {
                kind,
                params,
                waypoint_index: crate::map::actions::read_waypoint_token(token),
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
    runtime.advance_at_frame_with_waypoints(
        0,
        &graph,
        &triggers,
        &events,
        &actions,
        Some(sim),
        rules,
        waypoints,
    )
}

fn waypoint_action_rules() -> crate::rules::ruleset::RuleSet {
    crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
        "[Countries]\n0=Americans\n1=Russians\n\
         [Americans]\nName=United States\n\
         [Russians]\nName=Russian Federation\n",
    ))
    .expect("waypoint-action country registry parses")
}

fn register_waypoint_action_house(
    sim: &mut Simulation,
    name: &str,
    country: Option<&str>,
    alternate_base_center: (u16, u16),
) -> crate::sim::intern::InternedId {
    let owner = sim.interner.intern(name);
    let country = country.map(|country| sim.interner.intern(country));
    let mut house = crate::sim::house_state::HouseState::new(owner, 0, country, false, 0, 10);
    house.base_center = Some((40, 50));
    house.alternate_base_center = alternate_base_center;
    sim.houses.insert(owner, house);
    sim.session.house_order.push(owner);
    owner
}

#[test]
fn camera_actions_use_alpha_tokens_and_ctor_zero_not_decimal_indices() {
    for (kind, token, waypoint) in [(48, "NZ", 389), (112, "AA", 26)] {
        let mut sim = Simulation::new();
        let cell = (10 + waypoint as u16 % 20, 20 + waypoint as u16 % 20);
        let waypoints = [(
            waypoint,
            crate::map::waypoints::Waypoint {
                index: waypoint,
                rx: cell.0,
                ry: cell.1,
            },
        )]
        .into_iter()
        .collect();
        let world_x = i32::from(cell.0) * 256 + 128;
        let world_y = i32::from(cell.1) * 256 + 128;
        let (screen_x, screen_y) =
            crate::util::lepton::absolute_leptons_to_screen(world_x, world_y, 0);
        let target = [screen_x as i32, screen_y as i32];
        let expected = if kind == 48 {
            TriggerEffect::TacticalCamera(TacticalCameraCommand::Glide {
                target,
                speed: crate::util::native_x87::NativeF32Bits::from_bits(0x3ac4_9ba6),
            })
        } else {
            TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump { target })
        };
        assert_eq!(
            run_waypoint_action(kind, Some(token), None, &mut sim, &waypoints, None),
            vec![expected]
        );
    }

    for token in [None, Some("")] {
        let mut sim = Simulation::new();
        assert_eq!(
            run_waypoint_action(112, token, None, &mut sim, &HashMap::new(), None),
            vec![TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump {
                target: [0, 30],
            })]
        );
    }
    for token in [Some("15"), Some("   ")] {
        let mut sim = Simulation::new();
        assert!(run_waypoint_action(112, token, None, &mut sim, &HashMap::new(), None).is_empty());
    }
}

fn run_native_camera_action(
    kind: i32,
    selector: i32,
    token: &str,
    sim: &mut Simulation,
    waypoints: &HashMap<u32, crate::map::waypoints::Waypoint>,
) -> Vec<TriggerEffect> {
    let body = format!(
        "[Triggers]\nT=Neutral,<none>,Camera,1,1,1,1,0\n\
         [Tags]\nG=2,G,T\n\
         [Events]\nT=1,8,0,0\n\
         [Actions]\nT=1,{kind},0,{selector},91,-7,33,55,{token}\n",
    );
    let (_, program) = compile_program(&body);
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan::default(),
        1,
        sim.session.binary_frame,
        &mut sim.scenario_rng,
    );
    runtime.advance_native_poll(&program, sim, None, None, waypoints)
}

#[test]
fn native_camera_actions_share_exact_flat_slope_and_both_bridge_flag_targets() {
    use crate::map::bridge_facts::{BRIDGE_FLAG_DESTROYED_OR_RAMP, BRIDGE_FLAG_STRUCTURAL};

    let cell = (2_u16, 3_u16);
    let waypoints = [(
        1,
        crate::map::waypoints::Waypoint {
            index: 1,
            rx: cell.0,
            ry: cell.1,
        },
    )]
    .into_iter()
    .collect();
    for (level, slope, flags) in [
        (0_u8, 0_u8, 0_u32),
        (2, 1, 0),
        (1, 0, BRIDGE_FLAG_STRUCTURAL),
        (1, 0, BRIDGE_FLAG_DESTROYED_OR_RAMP),
    ] {
        for kind in [48, 112] {
            let mut sim = Simulation::new();
            let mut terrain = flat_trigger_playfield_terrain(8, 8);
            let target_cell = terrain.cell_mut(cell.0, cell.1).unwrap();
            target_cell.level = level;
            target_cell.slope_type = slope;
            target_cell.bridge_facts.raw_flags = flags;
            sim.install_resolved_terrain_for_new_map(terrain);

            let world_x = i32::from(cell.0) * 256 + 128;
            let world_y = i32::from(cell.1) * 256 + 128;
            let mut world_z = crate::util::lepton::ground_height_leptons(
                level, slope, world_x, world_y,
            )
            .unwrap();
            if flags & 0x500 != 0 {
                world_z += 416;
            }
            let (screen_x, screen_y) = crate::util::lepton::absolute_leptons_to_screen(
                world_x, world_y, world_z,
            );
            let target = [screen_x as i32, screen_y as i32];
            let expected = if kind == 48 {
                TriggerEffect::TacticalCamera(TacticalCameraCommand::Glide {
                    target,
                    speed: crate::util::native_x87::NativeF32Bits::from_bits(0x3bf5_c28f),
                })
            } else {
                TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump { target })
            };
            assert_eq!(
                run_native_camera_action(kind, 2, "B", &mut sim, &waypoints),
                vec![expected],
                "kind={kind} level={level} slope={slope} flags={flags:#x}",
            );
        }
    }
}

#[test]
fn native_camera_valid_missing_slot_is_not_a_noop_and_typed_invalids_fail_closed() {
    let mut sim = Simulation::new();
    assert_eq!(
        run_native_camera_action(112, 0, "Z", &mut sim, &HashMap::new()),
        vec![TriggerEffect::TacticalCamera(TacticalCameraCommand::Jump {
            target: [0, 30],
        })]
    );

    for row in [
        "T=1,48,0,5,91,-7,33,55,A",
        "T=1,112,0,0,91,-7,33,55,15",
    ] {
        let ini = crate::rules::ini_parser::IniFile::from_str(&format!(
            "[Triggers]\nT=Neutral,<none>,Camera,1,1,1,1,0\n\
             [Tags]\nG=2,G,T\n\
             [Events]\nT=1,8,0,0\n\
             [Actions]\n{row}\n",
        ));
        assert!(
            TriggerProgram::compile(
                &ini,
                &crate::map::tags::parse_tags(&ini),
                &crate::map::triggers::parse_triggers(&ini),
                &crate::map::events::parse_events(&ini),
                &crate::map::actions::parse_actions(&ini),
            )
            .is_err(),
            "invalid typed row must be rejected: {row}",
        );
    }
}

#[test]
fn action_137_replays_all_three_retail_waypoint_fixtures() {
    for (token, index, cell) in [
        ("P", 15, (93, 106)),
        ("NZ", 389, (122, 135)),
        ("AA", 26, (105, 194)),
    ] {
        let mut sim = Simulation::new();
        let rules = waypoint_action_rules();
        let house = register_waypoint_action_house(&mut sim, "HouseA", Some("Americans"), (0, 0));
        let waypoints = [(
            index,
            crate::map::waypoints::Waypoint {
                index,
                rx: cell.0,
                ry: cell.1,
            },
        )]
        .into_iter()
        .collect();

        assert!(
            run_waypoint_action(
                137,
                Some(token),
                Some("americans"),
                &mut sim,
                &waypoints,
                Some(&rules),
            )
            .is_empty()
        );
        assert_eq!(sim.houses[&house].alternate_base_center, cell);
        assert_eq!(sim.houses[&house].base_center, Some((40, 50)));
    }
}

#[test]
fn action_137_uses_first_registration_order_country_match_only() {
    let mut sim = Simulation::new();
    let rules = waypoint_action_rules();
    let first = register_waypoint_action_house(&mut sim, "First", Some("Americans"), (1, 2));
    let second = register_waypoint_action_house(&mut sim, "Second", Some("AMERICANS"), (3, 4));
    let waypoints = [(
        15,
        crate::map::waypoints::Waypoint {
            index: 15,
            rx: 93,
            ry: 106,
        },
    )]
    .into_iter()
    .collect();

    run_waypoint_action(
        137,
        Some("P"),
        Some("americans"),
        &mut sim,
        &waypoints,
        Some(&rules),
    );

    assert_eq!(sim.houses[&first].alternate_base_center, (93, 106));
    assert_eq!(sim.houses[&second].alternate_base_center, (3, 4));
    assert_eq!(sim.houses[&first].base_center, Some((40, 50)));
    assert_eq!(sim.houses[&second].base_center, Some((40, 50)));
}

#[test]
fn action_137_canonicalizes_house_type_alias_and_none_owner() {
    let rules = waypoint_action_rules();
    let waypoints = [(
        15,
        crate::map::waypoints::Waypoint {
            index: 15,
            rx: 93,
            ry: 106,
        },
    )]
    .into_iter()
    .collect();

    let mut alias_sim = Simulation::new();
    let alias_house =
        register_waypoint_action_house(&mut alias_sim, "AliasHouse", Some("Americans"), (0, 0));
    run_waypoint_action(
        137,
        Some("P"),
        Some("united states"),
        &mut alias_sim,
        &waypoints,
        Some(&rules),
    );
    assert_eq!(
        alias_sim.houses[&alias_house].alternate_base_center,
        (93, 106)
    );

    let mut none_sim = Simulation::new();
    let russian =
        register_waypoint_action_house(&mut none_sim, "RussianHouse", Some("Russians"), (7, 8));
    let first_type =
        register_waypoint_action_house(&mut none_sim, "AmericanHouse", Some("Americans"), (0, 0));
    run_waypoint_action(
        137,
        Some("P"),
        Some("<none>"),
        &mut none_sim,
        &waypoints,
        Some(&rules),
    );
    assert_eq!(none_sim.houses[&russian].alternate_base_center, (7, 8));
    assert_eq!(
        none_sim.houses[&first_type].alternate_base_center,
        (93, 106)
    );
}

#[test]
fn action_137_rejects_only_exact_packed_zero_cell() {
    let rules = waypoint_action_rules();
    for cell in [(0, 17), (23, 0)] {
        let mut sim = Simulation::new();
        let house =
            register_waypoint_action_house(&mut sim, "AxisHouse", Some("Americans"), (9, 9));
        let waypoints = [(
            15,
            crate::map::waypoints::Waypoint {
                index: 15,
                rx: cell.0,
                ry: cell.1,
            },
        )]
        .into_iter()
        .collect();
        run_waypoint_action(
            137,
            Some("P"),
            Some("Americans"),
            &mut sim,
            &waypoints,
            Some(&rules),
        );
        assert_eq!(sim.houses[&house].alternate_base_center, cell);
    }
}

#[test]
fn action_137_writes_signed_waypoint_halves_as_raw_nonzero_cell() {
    let rules = waypoint_action_rules();
    let mut sim = Simulation::new();
    let house =
        register_waypoint_action_house(&mut sim, "SignedWaypointHouse", Some("Americans"), (9, 9));
    let waypoints = crate::map::waypoints::parse_waypoints(
        &crate::rules::ini_parser::IniFile::from_str("[Waypoints]\n15=-1001\n"),
    );

    run_waypoint_action(
        137,
        Some("P"),
        Some("Americans"),
        &mut sim,
        &waypoints,
        Some(&rules),
    );

    assert_eq!(
        sim.houses[&house].alternate_base_center,
        (u16::MAX, u16::MAX),
        "signed -1 quotient/remainder halves are nonzero and must be written"
    );
    assert_eq!(sim.houses[&house].base_center, Some((40, 50)));
}

#[test]
fn action_137_invalid_resolution_and_waypoints_do_not_mutate_either_base_cell() {
    fn assert_no_write(
        trigger_country: Option<&str>,
        house_country: Option<&str>,
        token: Option<&str>,
        waypoints: HashMap<u32, crate::map::waypoints::Waypoint>,
        register: bool,
        with_rules: bool,
    ) {
        let mut sim = Simulation::new();
        let rules = waypoint_action_rules();
        let house = register
            .then(|| register_waypoint_action_house(&mut sim, "Americans", house_country, (7, 8)));
        run_waypoint_action(
            137,
            token,
            trigger_country,
            &mut sim,
            &waypoints,
            with_rules.then_some(&rules),
        );
        if let Some(house) = house {
            assert_eq!(sim.houses[&house].alternate_base_center, (7, 8));
            assert_eq!(sim.houses[&house].base_center, Some((40, 50)));
        }
    }

    let valid = || {
        [(
            15,
            crate::map::waypoints::Waypoint {
                index: 15,
                rx: 93,
                ry: 106,
            },
        )]
        .into_iter()
        .collect()
    };
    assert_no_write(None, Some("Americans"), Some("P"), valid(), true, true);
    assert_no_write(
        Some("Americans"),
        Some("Americans"),
        Some("P"),
        valid(),
        true,
        false,
    );
    assert_no_write(Some("Americans"), None, Some("P"), valid(), true, true);
    assert_no_write(
        Some("Americans"),
        Some("Americans"),
        Some("P"),
        valid(),
        false,
        true,
    );
    assert_no_write(
        Some("Americans"),
        Some("Americans"),
        Some("15"),
        valid(),
        true,
        true,
    );
    assert_no_write(
        Some("Americans"),
        Some("Americans"),
        Some("P"),
        HashMap::new(),
        true,
        true,
    );
    assert_no_write(
        Some("Americans"),
        Some("Americans"),
        Some("P"),
        [(
            15,
            crate::map::waypoints::Waypoint {
                index: 15,
                rx: 0,
                ry: 0,
            },
        )]
        .into_iter()
        .collect(),
        true,
        true,
    );
    // Matching the House name is insufficient: only HouseState.country owns
    // the native Spring/Find_By_Country_Index resolution.
    assert_no_write(
        Some("Americans"),
        Some("Russians"),
        Some("P"),
        valid(),
        true,
        true,
    );
}

#[test]
fn action_138_clears_only_first_country_match_without_token_or_rng_use() {
    let mut sim = Simulation::new();
    let rules = waypoint_action_rules();
    let first = register_waypoint_action_house(&mut sim, "First", Some("Americans"), (93, 106));
    let second = register_waypoint_action_house(&mut sim, "Second", Some("Americans"), (122, 135));
    let rng_before = sim.scenario_rng.state();

    run_waypoint_action(
        138,
        Some("present but ignored"),
        Some("AMERICANS"),
        &mut sim,
        &HashMap::new(),
        Some(&rules),
    );

    assert_eq!(sim.houses[&first].alternate_base_center, (0, 0));
    assert_eq!(sim.houses[&second].alternate_base_center, (122, 135));
    assert_eq!(sim.houses[&first].base_center, Some((40, 50)));
    assert_eq!(sim.houses[&second].base_center, Some((40, 50)));
    assert_eq!(sim.scenario_rng.state(), rng_before);

    let mut no_match = Simulation::new();
    let house =
        register_waypoint_action_house(&mut no_match, "OnlyHouse", Some("Russians"), (105, 194));
    run_waypoint_action(
        138,
        None,
        Some("Americans"),
        &mut no_match,
        &HashMap::new(),
        Some(&rules),
    );
    assert_eq!(no_match.houses[&house].alternate_base_center, (105, 194));
}

struct NativeResultFixture {
    program: TriggerProgram,
    runtime: TriggerRuntime,
    simulation: Simulation,
    russian: crate::sim::intern::InternedId,
    neutral: crate::sim::intern::InternedId,
    local: crate::sim::intern::InternedId,
}

impl NativeResultFixture {
    fn new(actions: &[(i32, i32)]) -> Self {
        Self::with_trigger_owner(actions, "Neutral")
    }

    fn with_trigger_owner(actions: &[(i32, i32)], trigger_owner: &str) -> Self {
        let chunks = actions
            .iter()
            .map(|(kind, operand)| format!("{kind},0,{operand},91,-7,33,55,A"))
            .collect::<Vec<_>>()
            .join(",");
        let body = format!(
            "[Triggers]\nT={trigger_owner},<none>,Result,1,1,1,1,0\n\
             [Tags]\nG=2,G,T\n\
             [Events]\nT=1,8,0,0\n\
             [Actions]\nT={},{}\n",
            actions.len(),
            chunks,
        );
        let (_, program) = compile_program(&body);
        let mut simulation = Simulation::new();
        simulation.session.binary_frame = 123;
        let russian = simulation.interner.intern("Russians");
        let neutral = simulation.interner.intern("Neutral");
        let local = simulation.interner.intern("Americans");
        for (owner, human) in [(russian, true), (neutral, false), (local, true)] {
            simulation.houses.insert(
                owner,
                crate::sim::house_state::HouseState::new(owner, 0, None, human, 0, 10),
            );
            simulation.session.house_order.push(owner);
        }
        let runtime = TriggerRuntime::materialize_fresh(
            &program,
            &LocalVariableMap::new(),
            &TriggerAttachmentPlan::default(),
            1,
            simulation.session.binary_frame,
            &mut simulation.scenario_rng,
        );
        Self {
            program,
            runtime,
            simulation,
            russian,
            neutral,
            local,
        }
    }

    fn poll(&mut self, pinned: Option<crate::sim::intern::InternedId>) -> Vec<TriggerEffect> {
        self.runtime.advance_native_poll_for_client(
            &self.program,
            &mut self.simulation,
            None,
            None,
            &HashMap::new(),
            pinned,
        )
    }

    fn notices(&self) -> Vec<(crate::sim::intern::InternedId, crate::sim::house_state::HouseOutcomeKind)> {
        self.simulation
            .sound_events
            .iter()
            .filter_map(|event| match event {
                crate::sim::world::SimSoundEvent::MatchOutcome { owner, kind } => {
                    Some((*owner, *kind))
                }
                _ => None,
            })
            .collect()
    }
}

#[test]
fn native_result_actions_use_only_the_pinned_local_house_and_ignore_operands() {
    let mut fixture = NativeResultFixture::new(&[(67, i32::MAX)]);
    assert!(fixture.poll(Some(fixture.local)).is_empty());

    assert!(fixture.simulation.houses[&fixture.local].has_won);
    assert!(!fixture.simulation.houses[&fixture.local].has_lost);
    assert!(!fixture.simulation.houses[&fixture.russian].has_won);
    assert!(!fixture.simulation.houses[&fixture.neutral].has_won);
    assert_eq!(fixture.simulation.houses[&fixture.local].result_timer_start, 0);
    assert_eq!(fixture.simulation.houses[&fixture.local].result_timer_duration, 0);
    assert_eq!(
        fixture.notices(),
        vec![(
            fixture.local,
            crate::sim::house_state::HouseOutcomeKind::Victory,
        )]
    );

    let mut no_pin = NativeResultFixture::new(&[(68, i32::MIN)]);
    assert!(no_pin.poll(None).is_empty());
    assert!(!no_pin.simulation.houses[&no_pin.local].has_won);
    assert!(!no_pin.simulation.houses[&no_pin.local].has_lost);
    assert!(no_pin.notices().is_empty());
}

#[test]
fn native_result_actions_emit_notices_only_for_accepted_67_68_transitions() {
    for (action, kind) in [
        (67, crate::sim::house_state::HouseOutcomeKind::Victory),
        (68, crate::sim::house_state::HouseOutcomeKind::Defeat),
    ] {
        let mut fixture = NativeResultFixture::new(&[(action, 77)]);
        let _ = fixture.poll(Some(fixture.local));
        let _ = fixture.poll(Some(fixture.local));
        assert_eq!(fixture.notices(), vec![(fixture.local, kind)]);
    }

    let mut end = NativeResultFixture::new(&[(69, -12345)]);
    let _ = end.poll(Some(end.local));
    assert!(end.simulation.houses[&end.local].has_won);
    assert_eq!(end.simulation.houses[&end.local].result_timer_start, 123);
    assert_eq!(end.simulation.houses[&end.local].result_timer_duration, 27);
    assert!(end.notices().is_empty(), "Action69 has no direct result notice");
}

#[test]
fn native_result_actions_preserve_pending_gates_and_action_68_win_clear() {
    let mut win = NativeResultFixture::new(&[(67, 0)]);
    win.simulation.houses.get_mut(&win.local).unwrap().result_pending = true;
    let _ = win.poll(Some(win.local));
    assert!(!win.simulation.houses[&win.local].has_won);
    assert!(win.notices().is_empty());

    let mut lose = NativeResultFixture::new(&[(68, 0)]);
    {
        let house = lose.simulation.houses.get_mut(&lose.local).unwrap();
        house.result_pending = true;
        house.has_won = true;
        house.result_timer_start = 17;
        house.result_timer_duration = 41;
    }
    let _ = lose.poll(Some(lose.local));
    let house = &lose.simulation.houses[&lose.local];
    assert!(!house.has_won);
    assert!(!house.has_lost);
    assert_eq!((house.result_timer_start, house.result_timer_duration), (17, 41));
    assert!(lose.notices().is_empty());

    let mut end = NativeResultFixture::new(&[(69, 0)]);
    {
        let house = end.simulation.houses.get_mut(&end.local).unwrap();
        house.result_pending = true;
        house.result_timer_start = 29;
        house.result_timer_duration = 53;
    }
    let _ = end.poll(Some(end.local));
    let house = &end.simulation.houses[&end.local];
    assert!(!house.has_won);
    assert!(!house.has_lost);
    assert_eq!((house.result_timer_start, house.result_timer_duration), (29, 53));
    assert!(end.notices().is_empty());
}

#[test]
fn native_result_action_order_preserves_shared_timer_and_textual_stack_results() {
    let mut win_then_lose = NativeResultFixture::new(&[(67, 1), (68, 2)]);
    let _ = win_then_lose.poll(Some(win_then_lose.local));
    let house = &win_then_lose.simulation.houses[&win_then_lose.local];
    assert!(!house.has_won && house.has_lost);
    assert_eq!((house.result_timer_start, house.result_timer_duration), (0, 0));
    assert_eq!(win_then_lose.notices().len(), 2);

    let mut lose_then_win = NativeResultFixture::new(&[(68, 3), (67, 4)]);
    let _ = lose_then_win.poll(Some(lose_then_win.local));
    let house = &lose_then_win.simulation.houses[&lose_then_win.local];
    assert!(!house.has_won && house.has_lost);
    assert_eq!(lose_then_win.notices().len(), 1);

    for terminal in [67, 68] {
        let mut repair = NativeResultFixture::new(&[(terminal, 5), (69, 6)]);
        repair.simulation.houses.get_mut(&repair.local).unwrap().result_timer_start = -1;
        let _ = repair.poll(Some(repair.local));
        assert_eq!(repair.simulation.houses[&repair.local].result_timer_start, 123);
        assert_eq!(repair.notices().len(), 1);
    }

    let mut armed_then_lose = NativeResultFixture::new(&[(69, 7), (68, 8)]);
    let _ = armed_then_lose.poll(Some(armed_then_lose.local));
    let house = &armed_then_lose.simulation.houses[&armed_then_lose.local];
    assert!(!house.has_won && house.has_lost);
    assert_eq!((house.result_timer_start, house.result_timer_duration), (123, 27));
    assert_eq!(armed_then_lose.notices().len(), 1);

    let mut armed_then_skip_win = NativeResultFixture::new(&[(69, 9), (67, 10)]);
    let _ = armed_then_skip_win.poll(Some(armed_then_skip_win.local));
    let house = &armed_then_skip_win.simulation.houses[&armed_then_skip_win.local];
    assert!(house.has_won && !house.has_lost);
    assert_eq!((house.result_timer_start, house.result_timer_duration), (123, 27));
    assert!(armed_then_skip_win.notices().is_empty());
}

fn action_119_rules() -> crate::rules::ruleset::RuleSet {
    crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
        "[Countries]\n0=Americans\n1=Alliance\n2=French\n3=Germans\n4=British\n5=Africans\n6=Arabs\n7=Koreans\n8=Neutral\n9=YuriCountry\n\
         [Americans]\nName=Americans\n\
         [Alliance]\nName=Alliance\n\
         [French]\nName=French\n\
         [Germans]\nName=Germans\n\
         [British]\nName=British\n\
         [Africans]\nName=Africans\n\
         [Arabs]\nName=Arabs\n\
         [Koreans]\nName=Koreans\n\
         [Neutral]\nName=Civilian\nMultiplayPassive=yes\n\
         [YuriCountry]\nName=YuriCountry\n\
         [InfantryTypes]\n\
         [VehicleTypes]\n0=TARGET\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [TARGET]\nStrength=100\nArmor=none\n\
         [Warheads]\n0=SWEEPC4\n\
         [SWEEPC4]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [CombatDamage]\nC4Warhead=SWEEPC4\n",
    ))
    .expect("Action 119 country/C4 rules")
}

fn action_119_program(operand: i32) -> TriggerProgram {
    compile_program(&format!(
        "[Triggers]\nT=Neutral,<none>,DestroyHouse,1,1,1,1,0\n\
         [Tags]\nG=2,G,T\n\
         [Events]\nT=1,8,0,0\n\
         [Actions]\nT=1,119,0,{operand},0,0,0,0,A\n",
    ))
    .1
}

fn register_action_119_house(
    sim: &mut Simulation,
    name: &str,
    country: &str,
) -> InternedId {
    let owner = sim.interner.intern(name);
    let country = sim.interner.intern(country);
    sim.houses.insert(
        owner,
        crate::sim::house_state::HouseState::new(owner, 0, Some(country), false, 0, 10),
    );
    sim.session.house_order.push(owner);
    owner
}

fn execute_action_119_once(
    operand: i32,
    with_rules: bool,
    resolve_handles: bool,
) -> (NativeActionResult, Simulation, InternedId) {
    let rules = action_119_rules();
    let program = action_119_program(operand);
    let mut simulation = Simulation::new();
    let first = register_action_119_house(&mut simulation, "FirstAmericans", "Americans");
    let _second = register_action_119_house(&mut simulation, "SecondAmericans", "Americans");
    if resolve_handles {
        simulation.resolve_type_handles(&rules);
    }
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan::default(),
        1,
        0,
        &mut simulation.scenario_rng,
    );
    let action = program.trigger_types[0].actions[0].clone();
    let result = {
        let mut transaction = TriggerTransaction {
            runtime: &mut runtime,
            program: &program,
            simulation: &mut simulation,
            rules: with_rules.then_some(&rules),
            overlay_registry: None,
            waypoints: &HashMap::new(),
            local_player_owner: None,
            effects: Vec::new(),
        };
        transaction.execute_action(0, 0, &action)
    };
    (result, simulation, first)
}

#[test]
fn action_119_resolver_covers_retail_operands_raising_house_and_slots_a_through_h() {
    let rules = action_119_rules();
    let program = action_119_program(0);
    let mut sim = Simulation::new();
    let mut by_country = BTreeMap::new();
    for (index, name) in [
        (0, "Americans"),
        (1, "Alliance"),
        (4, "British"),
        (6, "Arabs"),
        (9, "YuriCountry"),
    ] {
        by_country.insert(index, register_action_119_house(&mut sim, name, name));
    }
    let duplicate_american = register_action_119_house(&mut sim, "SecondAmericans", "Americans");
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan::default(),
        1,
        0,
        &mut sim.scenario_rng,
    );
    let instance = runtime.trigger_instances.get_mut(0).expect("live Trigger instance");
    let raising = by_country[&6];
    instance.raising_house = Some(raising);

    for operand in [9, 1, 4, 6, 9, 0, 1] {
        assert_eq!(
            resolve_action_119_house(&sim, &rules, Some(operand), Some(instance)),
            Some(by_country[&operand]),
            "mounted-retail Action119 country operand {operand}",
        );
    }
    assert_ne!(by_country[&0], duplicate_american);
    assert_eq!(
        resolve_action_119_house(&sim, &rules, Some(0), Some(instance)),
        Some(by_country[&0]),
        "ordinary country lookup returns the first House registration"
    );
    assert_eq!(
        resolve_action_119_house(&sim, &rules, Some(0x2325), Some(instance)),
        Some(raising)
    );

    for slot in 0_u32..8 {
        let house = *by_country
            .values()
            .nth(slot as usize % by_country.len())
            .unwrap();
        sim.session.start_slot_houses.insert(slot, house);
        assert_eq!(
            resolve_action_119_house(
                &sim,
                &rules,
                Some(0x117B + slot as i32),
                Some(instance),
            ),
            Some(house),
            "Player slot {slot}"
        );
    }
    assert_eq!(resolve_action_119_house(&sim, &rules, Some(-1), Some(instance)), None);
    assert_eq!(resolve_action_119_house(&sim, &rules, Some(3), Some(instance)), None);
    assert_eq!(resolve_action_119_house(&sim, &rules, Some(0), None), None);
}

#[test]
fn action_119_boolean_result_requires_trigger_house_rules_and_resolved_c4() {
    let (result, _, _) = execute_action_119_once(0, true, true);
    assert_eq!(result, NativeActionResult::True, "resolved empty sweep succeeds");
    let (result, _, _) = execute_action_119_once(0, false, true);
    assert_eq!(result, NativeActionResult::False, "missing Rules fails");
    let (result, simulation, _) = execute_action_119_once(0, true, false);
    assert_eq!(result, NativeActionResult::True, "resolved empty House returns true");
    assert!(
        simulation.rule_handles.is_some(),
        "direct callers idempotently install the configured C4 authority"
    );
    let (result, _, _) = execute_action_119_once(77, true, true);
    assert_eq!(result, NativeActionResult::False, "missing House fails");

    let rules = action_119_rules();
    let program = action_119_program(0);
    let mut simulation = Simulation::new();
    register_action_119_house(&mut simulation, "FirstAmericans", "Americans");
    simulation.resolve_type_handles(&rules);
    let mut runtime = TriggerRuntime::materialize_fresh(
        &program,
        &LocalVariableMap::new(),
        &TriggerAttachmentPlan::default(),
        1,
        0,
        &mut simulation.scenario_rng,
    );
    let action = program.trigger_types[0].actions[0].clone();
    let mut transaction = TriggerTransaction {
        runtime: &mut runtime,
        program: &program,
        simulation: &mut simulation,
        rules: Some(&rules),
        overlay_registry: None,
        waypoints: &HashMap::new(),
        local_player_owner: None,
        effects: Vec::new(),
    };
    assert_eq!(
        transaction.execute_action(u32::MAX, 0, &action),
        NativeActionResult::False,
        "missing live Trigger pointer fails before operand resolution"
    );
}

#[test]
fn action_119_production_poll_destroys_the_first_matching_house_only() {
    let rules = action_119_rules();
    for operand in [9, 1, 4, 6, 9, 0, 1] {
        let program = action_119_program(operand);
        let mut sim = Simulation::new();
        let target_country = rules
            .country_name(crate::rules::ruleset::CountryIdx(operand as u16))
            .expect("retail country index");
        let target = register_action_119_house(&mut sim, "TargetHouse", target_country);
        let other = register_action_119_house(&mut sim, "OtherHouse", "French");
        sim.resolve_type_handles(&rules);
        let target_id = sim
            .spawn_object("TARGET", "TargetHouse", 2, 2, 0, &rules, &BTreeMap::new())
            .unwrap();
        let other_id = sim
            .spawn_object("TARGET", "OtherHouse", 3, 2, 0, &rules, &BTreeMap::new())
            .unwrap();
        let mut runtime = TriggerRuntime::materialize_fresh(
            &program,
            &LocalVariableMap::new(),
            &TriggerAttachmentPlan::default(),
            1,
            0,
            &mut sim.scenario_rng,
        );

        assert!(runtime
            .advance_native_poll(&program, &mut sim, Some(&rules), None, &HashMap::new())
            .is_empty());

        assert!(sim.substrate.entities.get(target_id).is_some_and(|entity| entity.dying));
        assert!(sim.substrate.entities.get(other_id).is_some_and(|entity| !entity.dying));
        assert_eq!(sim.houses[&target].owned_unit_count, 0);
        assert_eq!(sim.houses[&other].owned_unit_count, 1);
    }
}

#[test]
fn action_119_transaction_uses_instance_raising_house_and_all_start_slots() {
    let rules = action_119_rules();
    for (operand, slot) in std::iter::once((0x2325, None))
        .chain((0_u32..8).map(|slot| (0x117B + slot as i32, Some(slot))))
    {
        let program = action_119_program(operand);
        let mut sim = Simulation::new();
        let target = register_action_119_house(&mut sim, "TargetHouse", "Americans");
        if let Some(slot) = slot {
            sim.session.start_slot_houses.insert(slot, target);
        }
        sim.resolve_type_handles(&rules);
        let target_id = sim
            .spawn_object("TARGET", "TargetHouse", 2, 2, 0, &rules, &BTreeMap::new())
            .unwrap();
        let mut runtime = TriggerRuntime::materialize_fresh(
            &program,
            &LocalVariableMap::new(),
            &TriggerAttachmentPlan::default(),
            1,
            0,
            &mut sim.scenario_rng,
        );
        if slot.is_none() {
            runtime.trigger_instances[0].raising_house = Some(target);
        }

        let _ = runtime.advance_native_poll(
            &program,
            &mut sim,
            Some(&rules),
            None,
            &HashMap::new(),
        );

        assert!(
            sim.substrate.entities.get(target_id).is_some_and(|entity| entity.dying),
            "operand={operand:#x} slot={slot:?}",
        );
    }
}

#[test]
#[ignore = "requires the configured active retail RA2/YR install"]
fn active_retail_action_119_rows_are_extracted_and_executed_through_typed_transactions() {
    let default_root = std::path::PathBuf::from(
        "C:/Users/enok/Documents/Command and Conquer Red Alert II",
    );
    let root = std::env::var_os("RA2_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or(default_root);
    let assets = crate::assets::asset_manager::AssetManager::new(&root)
        .unwrap_or_else(|error| panic!("open active retail install {}: {error}", root.display()));
    let expected = [
        ("all01umd.map", vec![("08AE3E3C", 9), ("0611BABC", 1)]),
        ("all03umd.map", vec![("06B0CCCC", 4)]),
        ("sov01umd.map", vec![("096879AC", 6)]),
        (
            "sov06lmd.map",
            vec![("0782720C", 9), ("09A0EC1C", 0), ("09A0C36C", 1)],
        ),
    ];
    let rules = action_119_rules();
    let mut extracted_total = 0;

    for (map_name, expected_rows) in expected {
        let bytes = assets
            .get(map_name)
            .unwrap_or_else(|| panic!("active archive stack does not contain {map_name}"));
        let ini = crate::rules::ini_parser::IniFile::from_bytes(&bytes)
            .unwrap_or_else(|error| panic!("parse {map_name}: {error}"));
        let actions = crate::map::actions::parse_actions(&ini);
        let rows = actions
            .iter()
            .flat_map(|(trigger_id, row)| {
                row.entries
                    .iter()
                    .filter(|entry| entry.kind == 119)
                    .map(move |entry| (trigger_id.as_str(), entry))
            })
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), expected_rows.len(), "{map_name} Action119 census");

        for (expected_trigger, expected_operand) in expected_rows {
            let (_, extracted) = rows
                .iter()
                .find(|(trigger_id, _)| trigger_id.eq_ignore_ascii_case(expected_trigger))
                .unwrap_or_else(|| panic!("{map_name} missing trigger {expected_trigger}"));
            assert_eq!(extracted.params[0], "0", "{map_name} ParamType");
            let operand = extracted.params[1]
                .parse::<i32>()
                .expect("retail Action119 numeric operand");
            assert_eq!(operand, expected_operand, "{map_name} {expected_trigger}");
            eprintln!(
                "{map_name}: trigger={expected_trigger} action=119 params={:?}",
                extracted.params,
            );

            let body = format!(
                "[Triggers]\nT=Neutral,<none>,Retail destroy,1,1,1,1,0\n\
                 [Tags]\nG=2,G,T\n\
                 [Events]\nT=1,8,0,0\n\
                 [Actions]\nT=1,119,{}\n",
                extracted.params.join(","),
            );
            let (_, program) = compile_program(&body);
            let mut sim = Simulation::new();
            let country = rules
                .country_name(crate::rules::ruleset::CountryIdx(operand as u16))
                .expect("retail country index");
            register_action_119_house(&mut sim, "TargetHouse", country);
            sim.resolve_type_handles(&rules);
            let target_id = sim
                .spawn_object("TARGET", "TargetHouse", 2, 2, 0, &rules, &BTreeMap::new())
                .unwrap();
            let mut runtime = TriggerRuntime::materialize_fresh(
                &program,
                &LocalVariableMap::new(),
                &TriggerAttachmentPlan::default(),
                1,
                0,
                &mut sim.scenario_rng,
            );
            assert!(runtime
                .advance_native_poll(&program, &mut sim, Some(&rules), None, &HashMap::new())
                .is_empty());
            assert!(sim.substrate.entities.get(target_id).is_some_and(|entity| entity.dying));
            extracted_total += 1;
        }
    }
    assert_eq!(extracted_total, 7);
}

#[test]
fn result_action_68_census_fixture_preserves_pinned_owner_semantics() {
    // Hermetic mirror of the separately gated active-install census below.
    let installed = [("all01umd.map", 68), ("all04dmd.map", 68)];
    assert_eq!(installed.len(), 2);
    assert!(installed.iter().all(|(_, action)| *action == 68));

    for (map_name, action) in installed {
        let mut fixture = NativeResultFixture::with_trigger_owner(&[(action, 123_456)], "Americans");
        fixture.simulation.session.map_name = map_name.to_string();
        let pinned = fixture.simulation.interner.intern("PinnedCampaignClient");
        fixture.simulation.houses.insert(
            pinned,
            crate::sim::house_state::HouseState::new(pinned, 0, None, true, 0, 10),
        );
        fixture.simulation.session.house_order.push(pinned);

        let _ = fixture.poll(Some(pinned));

        assert!(fixture.simulation.houses[&pinned].has_lost, "{map_name}");
        assert!(!fixture.simulation.houses[&fixture.local].has_lost, "{map_name}");
        assert!(!fixture.simulation.houses[&fixture.russian].has_lost, "{map_name}");
        assert_eq!(
            fixture.notices(),
            vec![(pinned, crate::sim::house_state::HouseOutcomeKind::Defeat)],
            "{map_name}",
        );
    }
}

#[test]
#[ignore = "requires the configured active retail RA2/YR install"]
fn active_retail_result_rows_are_extracted_and_executed_through_typed_transactions() {
    let default_root = std::path::PathBuf::from(
        "C:/Users/enok/Documents/Command and Conquer Red Alert II",
    );
    let root = std::env::var_os("RA2_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or(default_root);
    let assets = crate::assets::asset_manager::AssetManager::new(&root)
        .unwrap_or_else(|error| panic!("open active retail install {}: {error}", root.display()));

    for map_name in ["all01umd.map", "all04dmd.map"] {
        let bytes = assets
            .get(map_name)
            .unwrap_or_else(|| panic!("active archive stack does not contain {map_name}"));
        let ini = crate::rules::ini_parser::IniFile::from_bytes(&bytes)
            .unwrap_or_else(|error| panic!("parse {map_name}: {error}"));
        let triggers = crate::map::triggers::parse_triggers(&ini);
        let actions = crate::map::actions::parse_actions(&ini);
        let result_rows = actions
            .iter()
            .flat_map(|(trigger_id, row)| {
                row.entries
                    .iter()
                    .filter(|entry| matches!(entry.kind, 67 | 68 | 69))
                    .map(move |entry| (trigger_id, entry))
            })
            .collect::<Vec<_>>();
        assert_eq!(result_rows.len(), 1, "{map_name} result-row census");
        let (trigger_id, extracted) = result_rows[0];
        assert_eq!(extracted.kind, 68, "{map_name} active result action");
        assert_eq!(
            extracted.params,
            ["0", "0", "0", "0", "0", "0", "A"],
            "{map_name} exact retail Action68 operands",
        );
        let authored_owner = triggers
            .get(trigger_id)
            .and_then(|trigger| trigger.owner.as_deref())
            .unwrap_or_else(|| panic!("{map_name} {trigger_id} has no authored owner"));
        assert!(
            authored_owner.eq_ignore_ascii_case("Americans"),
            "{map_name} {trigger_id} owner={authored_owner}",
        );
        eprintln!(
            "{map_name}: trigger={trigger_id} owner={authored_owner} action={} params={:?}",
            extracted.kind,
            extracted.params,
        );

        // Feed the literal extracted chunk back through typed compilation and
        // the production per-Tag transaction. The pinned client is deliberately
        // neither the authored Americans House nor a preferred first human.
        let extracted_chunk = format!("{},{}", extracted.kind, extracted.params.join(","));
        let body = format!(
            "[Triggers]\nT={authored_owner},<none>,Retail result,1,1,1,1,0\n\
             [Tags]\nG=2,G,T\n\
             [Events]\nT=1,8,0,0\n\
             [Actions]\nT=1,{extracted_chunk}\n",
        );
        let (_, program) = compile_program(&body);
        let mut simulation = Simulation::new();
        simulation.session.map_name = map_name.to_string();
        let preferred = simulation.interner.intern("PreferredFirstHuman");
        let authored = simulation.interner.intern(authored_owner);
        let pinned = simulation.interner.intern("PinnedCampaignClient");
        for owner in [preferred, authored, pinned] {
            simulation.houses.insert(
                owner,
                crate::sim::house_state::HouseState::new(owner, 0, None, true, 0, 10),
            );
            simulation.session.house_order.push(owner);
        }
        let mut runtime = TriggerRuntime::materialize_fresh(
            &program,
            &LocalVariableMap::new(),
            &TriggerAttachmentPlan::default(),
            1,
            simulation.session.binary_frame,
            &mut simulation.scenario_rng,
        );
        assert!(
            runtime
                .advance_native_poll_for_client(
                    &program,
                    &mut simulation,
                    None,
                    None,
                    &HashMap::new(),
                    Some(pinned),
                )
                .is_empty()
        );
        assert!(simulation.houses[&pinned].has_lost, "{map_name}");
        assert!(!simulation.houses[&preferred].has_lost, "{map_name}");
        assert!(!simulation.houses[&authored].has_lost, "{map_name}");
    }
}
