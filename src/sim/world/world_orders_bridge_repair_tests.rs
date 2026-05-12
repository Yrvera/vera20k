//! Integration tests for the engineer-bridge-repair flow + the C4-on-CABHUT
//! collapse path (the latter `#[ignore]`-gated pending the upstream Immune
//! fix tracked as `project_c4_bridge_hut_followup`).

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
        "[InfantryTypes]\n0=ENGI\n\n\
         [VehicleTypes]\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=CABHUT\n\n\
         [ENGI]\nStrength=75\nArmor=none\nSpeed=4\nPrimary=none\nEngineer=yes\n\n\
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

#[test]
#[ignore = "blocked on project_c4_bridge_hut_followup — upstream Immune gate"]
fn c4_on_cabhut_destroys_bridge_when_upstream_immune_lifted() {
    // SETUP NOTE: this test requires the upstream Immune-gate fix to land
    // before C4 placement on CABHUT is reachable. Until then, the C4 plant
    // path rejects the target before reaching apply_c4_damage_to_building.
    //
    // Test shape (when unblocked):
    //   1. Spawn SEAL/Tanya with c4_plant.target_building_id = CABHUT.
    //   2. Place a ground infantry on a bridge cell adjacent to the CABHUT.
    //   3. Place a tank ON the bridge deck (OnBridge=true).
    //   4. Run tick_c4_plants for c4_delay_ticks ticks.
    //   5. Assert: CABHUT entity still alive (hut survives the explosion).
    //   6. Assert: adjacent bridge cells now Destroyed.
    //   7. Assert: TickResult.bridge_state_changed == true (so the app
    //      rebuilds PathGrid after collapse).
    //   8. Assert: the ground infantry on a BlowUpBridge cell is dying or
    //      despawned (kill_ground_occupants_at fired).
    //   9. Assert: the deck tank is at ground level with OnBridge=false
    //      (drop_in_bridge_deck_entities fired).
    //  10. Assert: zone_grid was rebuilt (zones cascade fired, e.g., check
    //      via a known unreachability query that previously succeeded).
    let _placeholder = ();
}
