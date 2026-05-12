//! Integration tests for the engineer-bridge-repair flow + the C4-on-CABHUT
//! collapse path. Engineer entry repairs the bridge and consumes the
//! engineer; C4 on the hut leaves the hut at full HP and collapses the
//! bridge segment via the BridgeRepairHut branch in
//! `apply_c4_damage_to_building`.

use super::*;
use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::bridge_state::{
    AnchorSpan, Axis, BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState, DamageState, Direction,
};
use crate::sim::components::Health;
use crate::sim::game_entity::GameEntity;
use std::collections::BTreeMap;

fn bridge_repair_test_rules() -> RuleSet {
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n0=ENGI\n1=GHOST\n\n\
         [VehicleTypes]\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=CABHUT\n\n\
         [ENGI]\nStrength=75\nArmor=none\nSpeed=4\nPrimary=none\nEngineer=yes\n\n\
         [GHOST]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=none\nC4=yes\n\n\
         [CABHUT]\nStrength=200\nArmor=concrete\nFoundation=1x1\nBridgeRepairHut=yes\n\n\
         [AudioVisual]\nRepairBridgeSound=BridgeRepaired\n\n\
         [CombatDamage]\nC4Warhead=SA\n\n\
         [SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    RuleSet::from_ini(&ini).expect("bridge-repair test rules should parse")
}

fn build_sim() -> (Simulation, RuleSet, BTreeMap<(u16, u16), u8>) {
    let mut sim = Simulation::new();
    let mut rules = bridge_repair_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    (sim, rules, BTreeMap::new())
}

fn spawn_engineer(sim: &mut Simulation, rx: u16, ry: u16) -> u64 {
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("ENGI");
    let id = sim.next_stable_entity_id;
    sim.next_stable_entity_id += 1;
    let e = GameEntity::new(
        id,
        rx,
        ry,
        0,
        0,
        owner,
        Health {
            current: 75,
            max: 75,
        },
        ty,
        EntityCategory::Infantry,
        0,
        5,
        false,
    );
    sim.entities.insert(e);
    id
}

fn spawn_seal(sim: &mut Simulation, rx: u16, ry: u16) -> u64 {
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GHOST");
    let id = sim.next_stable_entity_id;
    sim.next_stable_entity_id += 1;
    let e = GameEntity::new(
        id,
        rx,
        ry,
        0,
        0,
        owner,
        Health {
            current: 125,
            max: 125,
        },
        ty,
        EntityCategory::Infantry,
        0,
        5,
        false,
    );
    sim.entities.insert(e);
    id
}

fn spawn_cabhut(sim: &mut Simulation, rx: u16, ry: u16) -> u64 {
    let owner = sim.interner.intern("Soviets");
    let ty = sim.interner.intern("CABHUT");
    let id = sim.next_stable_entity_id;
    sim.next_stable_entity_id += 1;
    let e = GameEntity::new(
        id,
        rx,
        ry,
        0,
        0,
        owner,
        Health {
            current: 200,
            max: 200,
        },
        ty,
        EntityCategory::Structure,
        0,
        5,
        false,
    );
    sim.entities.insert(e);
    id
}

const BRIDGE_CELLS: &[(u16, u16)] = &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)];

fn seed_destroyed_bridge(sim: &mut Simulation) {
    seed_bridge_with_state(sim, DamageState::Destroyed);
}

fn seed_bridge_with_state(sim: &mut Simulation, state: DamageState) {
    let mut bs = BridgeRuntimeState::default();
    let span = AnchorSpan {
        id: 1,
        anchor: (10, 10),
        cells: [
            Some((10, 10)),
            Some((10, 11)),
            Some((10, 12)),
            Some((10, 13)),
            Some((10, 9)),
            None,
        ],
        axis: Axis::NS,
        direction: Direction::S,
        damage_state: state,
        bridge_group_id: 1,
    };
    bs.test_seed_anchor_span(span);
    for &(rx, ry) in BRIDGE_CELLS {
        let role = if (rx, ry) == (10, 10) {
            BridgeCellRole::Anchor
        } else {
            BridgeCellRole::Body
        };
        bs.test_seed_cell(
            rx,
            ry,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: state,
                axis: Some(Axis::NS),
                role,
                anchor_span_id: Some(1),
                overlay_byte: 0,
                damaged_variant: false,
            },
        );
    }
    sim.bridge_state = Some(bs);
}

fn step(sim: &mut Simulation, rules: &RuleSet, heights: &BTreeMap<(u16, u16), u8>) -> TickResult {
    let due = sim.take_due_commands();
    sim.advance_tick(&due, Some(rules), heights, None, None, 67)
}

#[test]
fn engineer_enters_cabhut_repairs_bridge() {
    let (mut sim, rules, heights) = build_sim();
    // Engineer adjacent (Chebyshev-1) to a CABHUT at (9, 10). Bridge cells
    // sit at (10, 9..=13) — the engineer's 5×5 scan covers all of them.
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer = spawn_engineer(&mut sim, 10, 10);
    sim.entities.get_mut(engineer).unwrap().capture_target = Some(cabhut);
    seed_destroyed_bridge(&mut sim);

    let result = step(&mut sim, &rules, &heights);

    assert!(
        result.bridge_state_changed,
        "TickResult.bridge_state_changed must be set on repair"
    );
    assert!(
        sim.entities.get(engineer).is_none(),
        "engineer must be despawned after repair"
    );

    let bs = sim.bridge_state.as_ref().unwrap();
    for &(rx, ry) in BRIDGE_CELLS {
        match bs.cell(rx, ry).unwrap().damage_state {
            DamageState::Healthy { variant } => assert!(
                variant <= 3,
                "cell ({rx},{ry}) variant={variant} — must be 0..=3 (healthy)"
            ),
            other => panic!("cell ({rx},{ry}) = {other:?} (expected Healthy)"),
        }
        assert!(bs.is_bridge_walkable(rx, ry));
    }
    assert!(
        sim.sound_events
            .iter()
            .any(|e| matches!(e, SimSoundEvent::BridgeRepaired { .. })),
        "BridgeRepaired sound event must be emitted"
    );
}

#[test]
fn engineer_at_intact_cabhut_emits_sound_no_mutation() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer = spawn_engineer(&mut sim, 10, 10);
    sim.entities.get_mut(engineer).unwrap().capture_target = Some(cabhut);
    seed_bridge_with_state(&mut sim, DamageState::Healthy { variant: 0 });

    let result = step(&mut sim, &rules, &heights);

    assert!(
        sim.entities.get(engineer).is_none(),
        "engineer still consumed even when bridge is intact"
    );
    assert!(
        sim.sound_events
            .iter()
            .any(|e| matches!(e, SimSoundEvent::BridgeRepaired { .. })),
        "sound event always fires on trigger"
    );
    assert!(
        !result.bridge_state_changed,
        "intact bridge: no zone rebuild signal"
    );
}

#[test]
fn two_engineers_both_repair_same_tick() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer_a = spawn_engineer(&mut sim, 10, 10);
    let engineer_b = spawn_engineer(&mut sim, 10, 11);
    sim.entities.get_mut(engineer_a).unwrap().capture_target = Some(cabhut);
    sim.entities.get_mut(engineer_b).unwrap().capture_target = Some(cabhut);
    seed_destroyed_bridge(&mut sim);

    step(&mut sim, &rules, &heights);

    assert!(sim.entities.get(engineer_a).is_none());
    assert!(sim.entities.get(engineer_b).is_none());
    let repair_events = sim
        .sound_events
        .iter()
        .filter(|e| matches!(e, SimSoundEvent::BridgeRepaired { .. }))
        .count();
    assert_eq!(repair_events, 2, "both engineers emit a sound event");

    let bs = sim.bridge_state.as_ref().unwrap();
    for &(rx, ry) in BRIDGE_CELLS {
        assert!(matches!(
            bs.cell(rx, ry).unwrap().damage_state,
            DamageState::Healthy { .. }
        ));
    }
}

#[test]
fn engineer_far_from_bridge_at_cabhut_no_mutation() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer = spawn_engineer(&mut sim, 10, 10);
    sim.entities.get_mut(engineer).unwrap().capture_target = Some(cabhut);
    // Empty bridge state — scan finds nothing.
    sim.bridge_state = Some(BridgeRuntimeState::default());

    let result = step(&mut sim, &rules, &heights);

    assert!(sim.entities.get(engineer).is_none());
    assert!(
        sim.sound_events
            .iter()
            .any(|e| matches!(e, SimSoundEvent::BridgeRepaired { .. })),
        "sound emitted; no bridge to mutate"
    );
    assert!(!result.bridge_state_changed);
}

/// SEAL with `c4_plant` set, adjacent to a healthy CABHUT, must:
///   - claim the plant on its first tick (adjacency-1),
///   - leave the hut at full HP across the entire C4Delay window,
///   - on timer expiry, route through the BridgeRepairHut branch in
///     `apply_c4_damage_to_building` so the bridge collapses while the
///     hut survives,
///   - propagate `bridge_state_changed` to TickResult so the app rebuilds
///     PathGrid.
///
/// Cascading-subsystem coverage (ground-occupant kill on BlowUpBridge
/// cells, deck-tank drop, zone_grid rebuild) is intentionally NOT
/// asserted here — those are owned by the bridge cascade tests proper.
/// This test only asserts the C4-on-CABHUT integration points.
#[test]
fn c4_on_cabhut_collapses_bridge_and_hut_survives() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let cabhut_max_hp = sim.entities.get(cabhut).unwrap().health.current;
    let seal = spawn_seal(&mut sim, 10, 10); // Chebyshev-1 adjacent
    sim.entities.get_mut(seal).unwrap().c4_plant =
        Some(crate::sim::components::C4PlantState {
            target_building_id: cabhut,
        });
    seed_bridge_with_state(&mut sim, DamageState::Healthy { variant: 0 });

    // First tick: tick_c4_plants Phase 1 sees adjacency and claims.
    step(&mut sim, &rules, &heights);
    assert!(
        sim.entities
            .get(cabhut)
            .and_then(|b| b.pending_c4_detonation)
            .is_some(),
        "plant must be claimed once SEAL is adjacent to CABHUT"
    );
    let plant_start = sim
        .entities
        .get(cabhut)
        .unwrap()
        .pending_c4_detonation
        .unwrap()
        .plant_start_tick;

    // Throughout the C4Delay window: hut HP must stay at max — the
    // BridgeRepairHut branch never damages the hut, even before the timer
    // fires. Bridge stays Healthy until detonation, then flips.
    let delay = rules.c4_delay_ticks as u64;
    let mut bridge_state_changed_seen = false;
    for _ in 0..(delay + 1) {
        let result = step(&mut sim, &rules, &heights);
        bridge_state_changed_seen |= result.bridge_state_changed;
        // Hut HP invariant — hold across every tick of the window.
        let cur = sim.entities.get(cabhut).unwrap().health.current;
        assert_eq!(
            cur, cabhut_max_hp,
            "hut HP must stay at max during C4Delay (plant_start={plant_start}, sim.tick={})",
            sim.tick
        );
    }

    // After detonation: hut alive, bridge segment Destroyed,
    // bridge_state_changed propagated at least once.
    let hut = sim
        .entities
        .get(cabhut)
        .expect("hut entity must survive the explosion");
    assert_eq!(
        hut.health.current, cabhut_max_hp,
        "hut HP unchanged: BridgeRepairHut branch must skip damage"
    );
    assert!(!hut.dying, "hut must not be marked dying");

    // Anchor cell must reach Destroyed via the body_cell_advance_state
    // walker invoked by `dispatch_bridge_collapse_from_hut`. Body cells
    // along the span propagate via the cascade machinery owned by the
    // bridge_orchestrator tests — this test does not re-assert that here.
    let bs = sim.bridge_state.as_ref().unwrap();
    let anchor = bs.cell(10, 10).unwrap();
    assert!(
        matches!(anchor.damage_state, DamageState::Destroyed),
        "anchor cell (10,10) must be Destroyed after C4 cascade, got {:?}",
        anchor.damage_state
    );

    assert!(
        bridge_state_changed_seen,
        "TickResult.bridge_state_changed must fire at least once so the app rebuilds PathGrid"
    );
}
