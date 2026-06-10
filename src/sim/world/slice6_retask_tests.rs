//! Slice 6 — verb API + dispatch-adoption integration tests.
//!
//! Two jobs:
//!   1. `replay_hash_stable_through_slice6` — the behavior-preserving gate. A
//!      scripted skirmish drives every retasking command site (Move / Stop /
//!      Attack / ForceAttack / ForceAttackCell / AttackMove) and asserts the
//!      end-of-run `state_hash()` equals the pre-slice baseline. The verb API
//!      writes `MissionCom` in parallel and `MissionCom` is unhashed in this
//!      slice, so a hash drift means a wrong `DockTeardown` subset or a dropped
//!      legacy-field clear — exactly what this gate exists to catch.
//!   2. The verb-write + retaliation-gate tripwires (added below the gate).

use super::*;
use crate::map::entities::{EntityCategory, MapEntity};
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::AttackTarget;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::components::OrderIntent;
use crate::sim::mission::MissionType;
use crate::sim::pathfinding::PathGrid;
use std::collections::BTreeMap;

fn slice6_rules() -> RuleSet {
    // Two attack-capable vehicles + an infantry; ranges short enough that no
    // auto-combat fires during the scripted window (commands drive everything,
    // keeping the RNG stream out of the picture).
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n0=E1\n\n\
         [VehicleTypes]\n0=MTNK\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=GACNST\n\n\
         [E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
         [GACNST]\nStrength=1000\nArmor=wood\nFoundation=4x3\n\n\
         [M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
         [SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n\n\
         [AP]\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n",
    );
    RuleSet::from_ini(&ini).expect("slice6 test rules should parse")
}

fn cmd_envelope(sim: &Simulation, owner: &str, execute_tick: u64, payload: Command) -> CommandEnvelope {
    let owner_id = sim
        .interner
        .get(owner)
        .unwrap_or_else(|| panic!("owner '{owner}' not interned"));
    CommandEnvelope::new(owner_id, execute_tick, payload)
}

fn unit(owner: &str, type_id: &str, cx: u16, cy: u16, cat: EntityCategory) -> MapEntity {
    MapEntity {
        owner: owner.to_string(),
        type_id: type_id.to_string(),
        health: 256,
        cell_x: cx,
        cell_y: cy,
        facing: 64,
        category: cat,
        sub_cell: 0,
        veterancy: 0,
        high: false,
    }
}

/// The pre-slice baseline. Captured from `dev` BEFORE the Slice-6 edits (run the
/// gate once, read the failure's `left:` value, paste it here). Slice 6 is
/// behavior-preserving, so this constant MUST NOT change for a Slice-6 *behavior*
/// reason. It DOES shift when a later slice adds a new field to the state hash:
/// the scripted scenario has no bunkers, so the value moved only because the
/// tank-bunker lifecycle state (`bunker_link`, `bunker_runtime`) now joins the
/// hash for every entity at its default — a hash-composition change, not a
/// behavior drift. Re-baselined for Slice 7b, then Slice 8 (MissionCom folded
/// into state_hash — every entity now contributes its default mission bytes;
/// composition change, not a behavior drift). Re-baselined for S3 idle→Guard:
/// idle machine-less Units hash mission Guard(5) instead of the legacy None
/// placeholder (hashed-representation fidelity fix; the retask behavior under
/// test is unchanged). Re-baselined for SC-2 (session identity — seed, map
/// name, theater, bounds, MP start table — folded into the hash; composition
/// change, not a behavior drift). Re-measured at the S3 × SC-2 merge (both
/// deltas combined; value from the merged tree's green run).
const SLICE6_BASELINE_HASH: u64 = 12517649216649727257;

#[test]
fn replay_hash_stable_through_slice6() {
    let rules = slice6_rules();
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    let mut sim = Simulation::new();
    // id 1: Americans MTNK (the unit we retask). id 2: enemy MTNK (Soviet, hostile
    // by default — no alliance entry). id 3: Americans E1 (second attacker).
    sim.spawn_from_map(
        &[
            unit("Americans", "MTNK", 3, 3, EntityCategory::Unit),
            unit("Soviet", "MTNK", 25, 3, EntityCategory::Unit),
            unit("Americans", "E1", 5, 5, EntityCategory::Infantry),
        ],
        Some(&rules),
        &heights,
    );

    // (execute_tick, command) — apply_due_commands fires each when self.session.tick+1 == tick.
    let script: &[(u64, Command)] = &[
        (
            1,
            Command::Move {
                entity_id: 1,
                target_rx: 10,
                target_ry: 10,
                queue: false,
                group_id: None,
            },
        ),
        (
            3,
            Command::AttackMove {
                entity_id: 1,
                target_rx: 15,
                target_ry: 3,
                queue: false,
            },
        ),
        (
            5,
            Command::ForceAttackCell {
                attacker_id: 1,
                target_rx: 18,
                target_ry: 3,
            },
        ),
        (
            7,
            Command::ForceAttack {
                attacker_id: 1,
                target_id: 2,
            },
        ),
        (
            9,
            Command::Attack {
                attacker_id: 3,
                target_id: 2,
            },
        ),
        (11, Command::Stop { entity_id: 1 }),
    ];

    for tick in 0..16u64 {
        let due: Vec<CommandEnvelope> = script
            .iter()
            .filter(|(t, _)| *t == tick + 1)
            .map(|(t, c)| cmd_envelope(&sim, "Americans", *t, c.clone()))
            .collect();
        let _ = sim.advance_tick(&due, Some(&rules), &heights, Some(&grid), None, 67);
    }

    let hash = sim.state_hash();
    assert_eq!(
        hash, SLICE6_BASELINE_HASH,
        "Slice 6 is behavior-preserving: the scripted-retask state hash must equal \
         the pre-slice baseline. A drift means a wrong DockTeardown subset or a \
         dropped legacy-field clear. (paste this `left` value into SLICE6_BASELINE_HASH)"
    );
}

#[test]
fn slice6_move_command_retasks_via_mission_substrate_and_clears_state() {
    // A Move command must route through the verb API: the mission substrate's
    // `current` becomes Move (proving the verb ran — checked BEFORE any tick-tail
    // shadow refresh) AND the legacy conflicting fields are cleared.
    let rules = slice6_rules();
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    let mut sim = Simulation::new();
    sim.spawn_from_map(
        &[unit("Americans", "MTNK", 3, 3, EntityCategory::Unit)],
        Some(&rules),
        &heights,
    );
    // Seed a conflicting prior order the Move must tear down.
    {
        let e = sim.substrate.entities.get_mut(1).expect("unit");
        e.attack_target = Some(AttackTarget::new(2));
        e.order_intent = Some(OrderIntent::Guard {
            anchor_rx: 3,
            anchor_ry: 3,
        });
    }

    let issued = sim.apply_command(
        "Americans",
        &Command::Move {
            entity_id: 1,
            target_rx: 10,
            target_ry: 10,
            queue: false,
            group_id: None,
        },
        Some(&rules),
        Some(&grid),
        &heights,
    );
    assert!(issued, "move command should issue");

    let e = sim.substrate.entities.get(1).expect("unit");
    assert_eq!(
        e.mission.current,
        MissionType::Move,
        "verb API committed Move to the mission substrate (pre-refresh)"
    );
    assert!(e.attack_target.is_none(), "Move tore down the attack target");
    assert!(e.order_intent.is_none(), "Move tore down the order intent");
}

#[test]
fn slice6_retaliation_still_suppressed_for_guarding_unit() {
    // The retaliation gate keeps the LITERAL `order_intent.is_some()` predicate
    // (a guarding unit must NOT retaliate). Tripwire: if the gate were swapped to
    // an `is_busy`-only check, the guard would retaliate and the first assert
    // would fail. The positive control (cleared intent) proves the suppression is
    // caused by `order_intent`, not by an absent valid weapon matchup.
    let rules = slice6_rules();
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut sim = Simulation::new();
    sim.spawn_from_map(
        &[
            unit("Americans", "MTNK", 3, 3, EntityCategory::Unit),
            unit("Soviet", "MTNK", 4, 3, EntityCategory::Unit),
        ],
        Some(&rules),
        &heights,
    );
    {
        let guard = sim.substrate.entities.get_mut(1).expect("guard");
        guard.last_attacker_id = Some(2);
        guard.order_intent = Some(OrderIntent::Guard {
            anchor_rx: 3,
            anchor_ry: 3,
        });
        assert!(guard.attack_target.is_none());
    }

    crate::sim::combat::tick_retaliation(&mut sim.substrate.entities, &rules, &sim.interner, &[1]);
    assert!(
        sim.substrate.entities.get(1).unwrap().attack_target.is_none(),
        "a guarding unit (order_intent = Guard) must NOT retaliate — the literal \
         order_intent gate suppresses it"
    );
    // last_attacker is preserved (the gate `continue`s before the clear).
    assert_eq!(
        sim.substrate.entities.get(1).unwrap().last_attacker_id,
        Some(2)
    );

    // Positive control: drop the order intent and the SAME unit now retaliates,
    // confirming the suppression above was due to `order_intent`, not the matchup.
    sim.substrate.entities.get_mut(1).unwrap().order_intent = None;
    crate::sim::combat::tick_retaliation(&mut sim.substrate.entities, &rules, &sim.interner, &[1]);
    assert!(
        sim.substrate.entities.get(1).unwrap().attack_target.is_some(),
        "with no order intent the unit retaliates (gate no longer suppresses)"
    );
}
