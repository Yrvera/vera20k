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
            .advance_at_frame(44, &graph, &triggers, &events, &actions, None)
            .is_empty()
    );
    assert_eq!(
        runtime.advance_at_frame(45, &graph, &triggers, &events, &actions, None),
        vec![TriggerEffect::CenterCameraAtWaypoint {
            waypoint: 9,
            immediate: true,
        }]
    );
    assert!(
        runtime
            .advance_at_frame(46, &graph, &triggers, &events, &actions, None)
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
        ReplayRunner::run_master_frame(
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
        runtime.advance_at_frame(15, &graph, &triggers, &events, &actions, None),
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
        runtime.advance_at_frame(15, &graph, &triggers, &events, &actions, None),
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
        runtime.advance_at_frame(15, &graph, &triggers, &events, &actions, None),
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
        runtime.advance_at_frame(15, &graph, &triggers, &events, &actions, None),
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
        runtime.advance_at_frame(0, &graph, &triggers, &events, &actions, None),
        Vec::<TriggerEffect>::new()
    );
    assert!(runtime.locals_set.contains(&2));
    assert_eq!(
        runtime.advance_at_frame(0, &graph, &triggers, &events, &actions, None),
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
        runtime.advance_at_frame(0, &graph, &triggers, &events, &actions, Some(&sim)),
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
