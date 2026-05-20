//! Unit tests for the GI deploy-fire state machine (Slice B1).

#![cfg(test)]

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::AttackTarget;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::components::Health;
use crate::sim::deploy::{DEPLOY_DEFAULT_TICKS, DeployPhase, frames_to_ticks};
use crate::sim::game_entity::GameEntity;
use crate::sim::world::{SimSoundEvent, Simulation};

/// Test ruleset with E1 (DeployFire=yes, GIDeploy/GIUndeploy sounds) and E2
/// (no DeployFire). Mirrors the [InfantryTypes] / [General] / weapon section
/// scaffolding from the canonical fixture in `ruleset.rs::make_test_rules`.
fn make_rules_with_deploy() -> RuleSet {
    let text = "\
[InfantryTypes]
0=E1
1=E2

[General]
BuildSpeed=0.75
MultipleFactory=0.7
LowPowerPenaltyModifier=1.25
MinLowPowerProductionSpeed=0.4
MaxLowPowerProductionSpeed=0.85

[VehicleTypes]

[AircraftTypes]

[BuildingTypes]

[E1]
Name=GI
Cost=200
Strength=125
Armor=none
Speed=4
Primary=M60
DeployFire=yes
DeploySound=GIDeploy
UndeploySound=GIUndeploy
IFVMode=2

[E2]
Name=Conscript
Cost=100
Strength=100
Armor=none
Speed=4
Primary=INTL

[M60]
Damage=25
ROF=20
Range=5
Warhead=SA

[INTL]
Damage=20
ROF=20
Range=5
Warhead=SA

[SA]
Verses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%
CellSpread=0
";
    let ini: IniFile = IniFile::from_str(text);
    RuleSet::from_ini(&ini).expect("test ruleset parse")
}

/// Test ruleset where E1 has DeployFire=yes but no DeploySound/UndeploySound.
fn make_rules_no_sounds() -> RuleSet {
    let text = "\
[InfantryTypes]
0=E1

[General]
BuildSpeed=0.75
MultipleFactory=0.7
LowPowerPenaltyModifier=1.25
MinLowPowerProductionSpeed=0.4
MaxLowPowerProductionSpeed=0.85

[VehicleTypes]

[AircraftTypes]

[BuildingTypes]

[E1]
Name=GI
Cost=200
Strength=125
Armor=none
Speed=4
Primary=M60
DeployFire=yes

[M60]
Damage=25
ROF=20
Range=5
Warhead=SA

[SA]
Verses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%
CellSpread=0
";
    let ini: IniFile = IniFile::from_str(text);
    RuleSet::from_ini(&ini).expect("test ruleset parse")
}

fn spawn_infantry(sim: &mut Simulation, type_str: &str, owner: &str, rx: u16, ry: u16) -> u64 {
    let owner_id = sim.interner.intern(owner);
    let type_id = sim.interner.intern(type_str);
    let id = sim.next_stable_entity_id;
    sim.next_stable_entity_id += 1;
    let e = GameEntity::new(
        id,
        rx,
        ry,
        0,
        0,
        owner_id,
        Health {
            current: 125,
            max: 125,
        },
        type_id,
        EntityCategory::Infantry,
        0,
        5,
        false,
    );
    sim.entities.insert(e);
    id
}

/// Schedule one command for tick N+1 and run a single advance_tick.
fn dispatch(sim: &mut Simulation, _owner: &str, cmd: Command, rules: &RuleSet) {
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let owner_id = sim.interner.intern(_owner);
    let cmds = vec![CommandEnvelope::new(owner_id, sim.tick + 1, cmd)];
    sim.advance_tick(&cmds, Some(rules), &height_map, None, None, 22);
}

/// Apply a command directly via `apply_command` (no tick advance, no combat
/// or animation cleanup), returning whether the handler accepted it. This is
/// the cleanest signal for gate tests — gate fires → returns false; gate
/// passes → returns true (or fails downstream for unrelated reasons, e.g.
/// missing path_grid for Move). For deploy gate tests, the only thing the
/// gate cares about is that the early-return short-circuits the handler.
fn apply(sim: &mut Simulation, owner: &str, cmd: &Command, rules: &RuleSet) -> bool {
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    sim.apply_command(owner, cmd, Some(rules), None, &height_map)
}

fn tick_n(sim: &mut Simulation, rules: &RuleSet, n: u32) {
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    for _ in 0..n {
        sim.advance_tick(&[], Some(rules), &height_map, None, None, 22);
    }
}

#[test]
fn deploy_phase_advances_to_deployed() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);

    dispatch(
        &mut sim,
        "Americans",
        Command::ToggleInfantryDeploy { entity_id: gi },
        &rules,
    );
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Deploying { .. })
    ));

    let n = DEPLOY_DEFAULT_TICKS as u32;
    tick_n(&mut sim, &rules, n);
    assert_eq!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Deployed)
    );
}

#[test]
fn undeploy_phase_clears_to_none() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    dispatch(
        &mut sim,
        "Americans",
        Command::ToggleInfantryDeploy { entity_id: gi },
        &rules,
    );
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Undeploying { .. })
    ));

    let n = DEPLOY_DEFAULT_TICKS as u32;
    tick_n(&mut sim, &rules, n);
    assert_eq!(sim.entities.get(gi).unwrap().deploy_state, None);
}

#[test]
fn mid_deploying_toggle_ignored() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state =
        Some(DeployPhase::Deploying { ticks_remaining: 3 });

    let sounds_before = sim.sound_events.len();
    dispatch(
        &mut sim,
        "Americans",
        Command::ToggleInfantryDeploy { entity_id: gi },
        &rules,
    );
    // Tick advance still runs; Deploying decremented from 3 → 2 (or to Deployed if already 1).
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Deploying { .. }) | Some(DeployPhase::Deployed)
    ));
    let new_deploy_undeploy_sounds = sim
        .sound_events
        .iter()
        .skip(sounds_before)
        .filter(|e| {
            matches!(
                e,
                SimSoundEvent::EntityDeployed { .. } | SimSoundEvent::EntityUndeployed { .. }
            )
        })
        .count();
    assert_eq!(new_deploy_undeploy_sounds, 0);
}

#[test]
fn mid_undeploying_toggle_ignored() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state =
        Some(DeployPhase::Undeploying { ticks_remaining: 3 });

    dispatch(
        &mut sim,
        "Americans",
        Command::ToggleInfantryDeploy { entity_id: gi },
        &rules,
    );
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Undeploying { .. }) | None
    ));
}

#[test]
fn move_silently_ignored_on_deployed() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    let applied = apply(
        &mut sim,
        "Americans",
        &Command::Move {
            entity_id: gi,
            target_rx: 30,
            target_ry: 30,
            queue: false,
            group_id: None,
        },
        &rules,
    );
    assert!(!applied, "gate must reject Move on deployed unit");
    let entity = sim.entities.get(gi).unwrap();
    assert!(entity.movement_target.is_none());
}

#[test]
fn move_silently_ignored_on_deploying() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state =
        Some(DeployPhase::Deploying { ticks_remaining: 5 });

    let applied = apply(
        &mut sim,
        "Americans",
        &Command::Move {
            entity_id: gi,
            target_rx: 30,
            target_ry: 30,
            queue: false,
            group_id: None,
        },
        &rules,
    );
    assert!(!applied, "gate must reject Move on Deploying unit");
}

#[test]
fn move_silently_ignored_on_undeploying() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state =
        Some(DeployPhase::Undeploying { ticks_remaining: 5 });

    let applied = apply(
        &mut sim,
        "Americans",
        &Command::Move {
            entity_id: gi,
            target_rx: 30,
            target_ry: 30,
            queue: false,
            group_id: None,
        },
        &rules,
    );
    assert!(!applied, "gate must reject Move on Undeploying unit");
}

#[test]
fn attack_move_silently_ignored_on_deployed() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    let applied = apply(
        &mut sim,
        "Americans",
        &Command::AttackMove {
            entity_id: gi,
            target_rx: 30,
            target_ry: 30,
            queue: false,
        },
        &rules,
    );
    assert!(!applied, "gate must reject AttackMove on deployed unit");
    let entity = sim.entities.get(gi).unwrap();
    assert!(entity.movement_target.is_none());
    assert!(entity.order_intent.is_none());
}

#[test]
fn enter_transport_silently_ignored_on_deployed() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    let applied = apply(
        &mut sim,
        "Americans",
        &Command::EnterTransport {
            passenger_id: gi,
            transport_id: 9999,
        },
        &rules,
    );
    assert!(!applied, "gate must reject EnterTransport on deployed unit");
    assert!(matches!(
        sim.entities.get(gi).unwrap().passenger_role,
        crate::sim::passenger::PassengerRole::None
    ));
}

#[test]
fn move_works_after_undeploy_completes() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    // Apply ToggleInfantryDeploy directly — gate doesn't apply to it (it's
    // the toggle itself), and the handler returns true on Deployed → Undeploying.
    let toggled = apply(
        &mut sim,
        "Americans",
        &Command::ToggleInfantryDeploy { entity_id: gi },
        &rules,
    );
    assert!(toggled);
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Undeploying { .. })
    ));

    let n = DEPLOY_DEFAULT_TICKS as u32;
    tick_n(&mut sim, &rules, n);
    assert_eq!(sim.entities.get(gi).unwrap().deploy_state, None);

    // Now the gate must let Move through — it'll fail downstream for
    // unrelated reasons (no path_grid), but only AFTER the gate.
    // The test signal is: the gate doesn't fire (we don't get the gate's
    // early `false` return). Since Move with no path_grid also returns
    // `false` past the gate, we instead check via dock_state — Move
    // mutates `e.dock_state = None` BEFORE the path_grid check; if the
    // gate fired, dock_state would stay Some.
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    sim.entities.get_mut(gi).unwrap().dock_state =
        Some(crate::sim::docking::building_dock::DockState {
            dock_building_id: 9999,
            phase: crate::sim::docking::building_dock::DockPhase::Approach,
            service_timer: 0,
            no_funds_ticks: 0,
        });
    let _ = sim.apply_command(
        "Americans",
        &Command::Move {
            entity_id: gi,
            target_rx: 12,
            target_ry: 12,
            queue: false,
            group_id: None,
        },
        Some(&rules),
        None,
        &height_map,
    );
    assert!(
        sim.entities.get(gi).unwrap().dock_state.is_none(),
        "Move handler past the gate must clear dock_state after undeploy completes"
    );
}

#[test]
fn deploy_sound_emits_alongside_state_write() {
    // Regression lock for the emit-before-state-write reorder: both effects
    // (sound buffered + deploy_state = Deploying) must be observable after a
    // single ToggleInfantryDeploy command.
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 25, 30);

    assert!(sim.entities.get(gi).unwrap().deploy_state.is_none());
    let events_before = sim.sound_events.len();

    let applied = apply(
        &mut sim,
        "Americans",
        &Command::ToggleInfantryDeploy { entity_id: gi },
        &rules,
    );
    assert!(applied);

    let entity = sim.entities.get(gi).unwrap();
    assert!(matches!(
        entity.deploy_state,
        Some(DeployPhase::Deploying { .. })
    ));
    assert_eq!(sim.sound_events.len(), events_before + 1);
    assert!(matches!(
        sim.sound_events.last().unwrap(),
        SimSoundEvent::EntityDeployed { .. }
    ));
}

#[test]
fn deploy_sound_emitted_on_phase_entry() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 25, 30);

    dispatch(
        &mut sim,
        "Americans",
        Command::ToggleInfantryDeploy { entity_id: gi },
        &rules,
    );
    let evs: Vec<_> = sim
        .sound_events
        .iter()
        .filter_map(|e| match e {
            SimSoundEvent::EntityDeployed {
                deploy_sound_id,
                rx,
                ry,
            } => Some((*deploy_sound_id, *rx, *ry)),
            _ => None,
        })
        .collect();
    assert_eq!(evs.len(), 1);
    let (id, rx, ry) = evs[0];
    assert_eq!(sim.interner.resolve(id), "GIDeploy");
    assert_eq!((rx, ry), (25, 30));
}

#[test]
fn deploy_sound_suppressed_when_unset() {
    let rules = make_rules_no_sounds();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 25, 30);

    dispatch(
        &mut sim,
        "Americans",
        Command::ToggleInfantryDeploy { entity_id: gi },
        &rules,
    );
    let count = sim
        .sound_events
        .iter()
        .filter(|e| matches!(e, SimSoundEvent::EntityDeployed { .. }))
        .count();
    assert_eq!(count, 0);
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Deploying { .. })
    ));
}

#[test]
fn undeploy_sound_emitted_on_phase_entry() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 25, 30);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    dispatch(
        &mut sim,
        "Americans",
        Command::ToggleInfantryDeploy { entity_id: gi },
        &rules,
    );
    let count = sim
        .sound_events
        .iter()
        .filter(|e| matches!(e, SimSoundEvent::EntityUndeployed { .. }))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn non_deploy_fire_infantry_no_op() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let conscript = spawn_infantry(&mut sim, "E2", "Soviets", 10, 10);

    dispatch(
        &mut sim,
        "Soviets",
        Command::ToggleInfantryDeploy {
            entity_id: conscript,
        },
        &rules,
    );
    assert!(sim.entities.get(conscript).unwrap().deploy_state.is_none());
}

#[test]
fn hash_deterministic_through_full_cycle() {
    let rules = make_rules_with_deploy();
    let mut sim_a = Simulation::new();
    let mut sim_b = Simulation::new();
    let gi_a = spawn_infantry(&mut sim_a, "E1", "Americans", 10, 10);
    let gi_b = spawn_infantry(&mut sim_b, "E1", "Americans", 10, 10);
    assert_eq!(gi_a, gi_b);

    for _ in 0..3 {
        dispatch(
            &mut sim_a,
            "Americans",
            Command::ToggleInfantryDeploy { entity_id: gi_a },
            &rules,
        );
        dispatch(
            &mut sim_b,
            "Americans",
            Command::ToggleInfantryDeploy { entity_id: gi_b },
            &rules,
        );
        let n = DEPLOY_DEFAULT_TICKS as u32;
        for _ in 0..n {
            tick_n(&mut sim_a, &rules, 1);
            tick_n(&mut sim_b, &rules, 1);
            assert_eq!(sim_a.state_hash(), sim_b.state_hash());
        }
    }
}

#[test]
fn snapshot_round_trip_mid_deploying() {
    use crate::sim::snapshot::GameSnapshot;
    let _rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state =
        Some(DeployPhase::Deploying { ticks_remaining: 5 });

    let bytes = GameSnapshot::save(&sim, 0, 0, "test_map");
    let snap = GameSnapshot::load(&bytes).expect("load");
    assert_eq!(
        snap.sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Deploying { ticks_remaining: 5 })
    );
}

#[test]
fn combat_fires_during_deployed_attack() {
    use crate::sim::animation::{
        Animation, LoopMode, SequenceDef, SequenceKind, SequenceSet, tick_animations,
    };

    let _rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);
    sim.entities.get_mut(gi).unwrap().attack_target = Some(AttackTarget {
        target: crate::sim::combat::TargetKind::Entity(9999),
        cooldown_ticks: 10,
        burst_remaining: 0,
        burst_delay_ticks: 0,
        pending_infantry_fire: Some(crate::sim::combat::PendingInfantryFire {
            sequence: SequenceKind::DeployedFire,
            fire_frame: 2,
        }),
    });
    sim.entities.get_mut(gi).unwrap().animation = Some(Animation::new(SequenceKind::Deployed));

    let mut sequences: BTreeMap<String, SequenceSet> = BTreeMap::new();
    let mut set = SequenceSet::new();
    set.insert(
        SequenceKind::Deployed,
        SequenceDef {
            start_frame: 0,
            frame_count: 1,
            facings: 8,
            facing_multiplier: 1,
            tick_ms: 200,
            loop_mode: LoopMode::Loop,
            clockwise_facings: false,
        },
    );
    set.insert(
        SequenceKind::DeployedFire,
        SequenceDef {
            start_frame: 8,
            frame_count: 6,
            facings: 8,
            facing_multiplier: 6,
            tick_ms: 80,
            loop_mode: LoopMode::TransitionTo(SequenceKind::Deployed),
            clockwise_facings: false,
        },
    );
    sequences.insert("E1".to_string(), set);

    let _ = tick_animations(&mut sim.entities, &sequences, 22, &sim.interner);
    assert_eq!(
        sim.entities
            .get(gi)
            .unwrap()
            .animation
            .as_ref()
            .unwrap()
            .sequence,
        SequenceKind::DeployedFire
    );
}

/// Test ruleset with GGI and a merged art entry that defines
/// GuardianGISequence with Deploy=300,15,0 and Undeploy=180,2,2.
fn make_rules_with_ggi_art() -> RuleSet {
    let rules_text = "\
[InfantryTypes]
0=GGI

[General]
BuildSpeed=0.75
MultipleFactory=0.7
LowPowerPenaltyModifier=1.25
MinLowPowerProductionSpeed=0.4
MaxLowPowerProductionSpeed=0.85

[VehicleTypes]

[AircraftTypes]

[BuildingTypes]

[GGI]
Name=Guardian GI
Cost=400
Strength=100
Armor=none
Speed=4
Primary=M60
DeployFire=yes
DeploySound=GuardianDeploy

[M60]
Damage=15
ROF=20
Range=4
Warhead=SA

[SA]
Verses=100%,80%,80%,50%,25%,25%,75%,50%,25%,100%,100%
CellSpread=0
";
    let rules_ini = IniFile::from_str(rules_text);
    let mut rules = RuleSet::from_ini(&rules_ini).expect("rules parse");
    let art_ini = IniFile::from_str(
        "[GGI]\n\
         Sequence=GuardianGISequence\n\
         \n\
         [GuardianGISequence]\n\
         Ready=0,1,1\n\
         Walk=8,6,6\n\
         Deploy=300,15,0\n\
         Undeploy=180,2,2\n\
         Deployed=315,1,1\n\
         DeployedFire=323,6,6\n",
    );
    let art = crate::rules::art_data::ArtRegistry::from_ini(&art_ini);
    rules.merge_art_data(&art);
    rules.art_registry = art;
    rules
}

#[test]
fn ggi_deploy_uses_art_frame_count() {
    // GGI's GuardianGISequence has Deploy=300,15,0 -> 15 frames.
    // 15 * 80 / 22 = 54 ticks (vs. the 55-tick fallback for sequence-less
    // infantry like E1). Uses apply() so we observe the raw command effect
    // without any deploy-tick decrement.
    let rules = make_rules_with_ggi_art();
    let mut sim = Simulation::new();
    let ggi = spawn_infantry(&mut sim, "GGI", "Americans", 10, 10);

    let applied = apply(
        &mut sim,
        "Americans",
        &Command::ToggleInfantryDeploy { entity_id: ggi },
        &rules,
    );
    assert!(applied);

    let entity = sim.entities.get(ggi).unwrap();
    match entity.deploy_state {
        Some(DeployPhase::Deploying { ticks_remaining }) => {
            assert_eq!(
                ticks_remaining, 54,
                "GGI deploy = 15 frames * 80 / 22 = 54 ticks"
            );
        }
        other => panic!("expected Deploying, got {:?}", other),
    }
}

#[test]
fn ggi_deploy_decrements_on_command_tick() {
    // Full advance_tick path: ToggleInfantryDeploy writes the art-derived
    // 15-frame countdown, then tick_deploy_state runs later in the same tick.
    let rules = make_rules_with_ggi_art();
    let mut sim = Simulation::new();
    let ggi = spawn_infantry(&mut sim, "GGI", "Americans", 10, 10);
    let deploy_ticks = frames_to_ticks(15);

    dispatch(
        &mut sim,
        "Americans",
        Command::ToggleInfantryDeploy { entity_id: ggi },
        &rules,
    );

    assert_eq!(
        sim.entities.get(ggi).unwrap().deploy_state,
        Some(DeployPhase::Deploying {
            ticks_remaining: deploy_ticks - 1
        })
    );
    tick_n(&mut sim, &rules, (deploy_ticks - 2) as u32);
    assert_eq!(
        sim.entities.get(ggi).unwrap().deploy_state,
        Some(DeployPhase::Deploying { ticks_remaining: 1 })
    );
    tick_n(&mut sim, &rules, 1);
    assert_eq!(
        sim.entities.get(ggi).unwrap().deploy_state,
        Some(DeployPhase::Deployed)
    );
}

#[test]
fn ggi_undeploy_uses_art_frame_count() {
    // GuardianGISequence Undeploy=180,2,2 -> 2 frames -> 7 ticks.
    let rules = make_rules_with_ggi_art();
    let mut sim = Simulation::new();
    let ggi = spawn_infantry(&mut sim, "GGI", "Americans", 10, 10);
    sim.entities.get_mut(ggi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    let applied = apply(
        &mut sim,
        "Americans",
        &Command::ToggleInfantryDeploy { entity_id: ggi },
        &rules,
    );
    assert!(applied);

    let entity = sim.entities.get(ggi).unwrap();
    match entity.deploy_state {
        Some(DeployPhase::Undeploying { ticks_remaining }) => {
            assert_eq!(
                ticks_remaining, 7,
                "GGI undeploy = 2 frames * 80 / 22 = 7 ticks"
            );
        }
        other => panic!("expected Undeploying, got {:?}", other),
    }
}

#[test]
fn ggi_undeploy_decrements_on_command_tick() {
    // GuardianGISequence Undeploy=180,2,2 -> 2 frames. The current sim-local
    // countdown decrements once in the same tick that accepts the command.
    let rules = make_rules_with_ggi_art();
    let mut sim = Simulation::new();
    let ggi = spawn_infantry(&mut sim, "GGI", "Americans", 10, 10);
    sim.entities.get_mut(ggi).unwrap().deploy_state = Some(DeployPhase::Deployed);
    let undeploy_ticks = frames_to_ticks(2);

    dispatch(
        &mut sim,
        "Americans",
        Command::ToggleInfantryDeploy { entity_id: ggi },
        &rules,
    );

    assert_eq!(
        sim.entities.get(ggi).unwrap().deploy_state,
        Some(DeployPhase::Undeploying {
            ticks_remaining: undeploy_ticks - 1
        })
    );
    tick_n(&mut sim, &rules, (undeploy_ticks - 2) as u32);
    assert_eq!(
        sim.entities.get(ggi).unwrap().deploy_state,
        Some(DeployPhase::Undeploying { ticks_remaining: 1 })
    );
    tick_n(&mut sim, &rules, 1);
    assert_eq!(sim.entities.get(ggi).unwrap().deploy_state, None);
}

#[test]
fn sequence_less_infantry_falls_back_to_default_ticks() {
    // E1 has no art Sequence= -> compute_anim_ticks falls back to
    // DEPLOY_DEFAULT_TICKS=55. Distinguishes the GGI 54-tick path from the
    // baseline.
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);

    let applied = apply(
        &mut sim,
        "Americans",
        &Command::ToggleInfantryDeploy { entity_id: gi },
        &rules,
    );
    assert!(applied);
    let entity = sim.entities.get(gi).unwrap();
    match entity.deploy_state {
        Some(DeployPhase::Deploying { ticks_remaining }) => {
            assert_eq!(ticks_remaining, DEPLOY_DEFAULT_TICKS);
        }
        other => panic!("expected Deploying, got {:?}", other),
    }
}
