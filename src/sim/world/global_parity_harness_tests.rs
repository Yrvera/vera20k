//! Slice 8 — global lockstep parity harness.
//!
//! Records a deterministic multi-faction skirmish as a `ReplayLog` and re-runs it
//! through the SAME `ReplayRunner::run` path the live game uses, asserting (1)
//! every tick's replayed hash equals the recorded hash (intra-run determinism)
//! and (2) the final hash equals a committed baseline. This is the project-wide
//! desync tripwire for the whole mission/radio substrate migration.
//!
//! Coverage: two hostile houses; an Allied war factory + refinery + harvester
//! over a seeded ore patch (the harvester gets a `Miner` component at spawn and
//! the miner system acquires an ore target — that state folds into the hash);
//! tanks + infantry under scripted Move/AttackMove/Stop, with the two sides
//! closing to combat range (exercises mission retask, movement, targeting/
//! retaliation, and the RNG streams). The harvester carries the real
//! `Harvester`/`Dock`/`Storage` flags and the refinery `Refinery=yes`.
//!
//! Scope note: this is a determinism + baseline guard, not a miner-dock test.
//! Driving a harvester physically to ore and through the full refinery dock
//! handshake needs movement world-setup (terrain costs / resolved terrain) that
//! the dedicated miner-dock suite (`miner_tests.rs`) provides and owns; this
//! harness only guards that the miner system stays wired and deterministic.

use super::*;
use crate::map::entities::{EntityCategory, MapEntity};
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::miner::{ResourceNode, ResourceType};
use crate::sim::pathfinding::PathGrid;
use crate::sim::replay::{ReplayHeader, ReplayLog, ReplayRunner};
use std::collections::BTreeMap;

const HARNESS_SEED: u64 = 0xC0FFEE_1234;
const HARNESS_TICKS: u64 = 600;
const HARNESS_TICK_MS: u32 = 67;

/// Committed final-hash baseline. Captured from the first green run. Re-baselines
/// at most once per behavior-bearing change, with a one-line documented reason.
/// Baselined for Slice 8 (initial commit of the global parity harness).
const GLOBAL_HARNESS_FINAL_HASH: u64 = 669004916847079430;

fn harness_rules() -> RuleSet {
    // Multi-faction vehicles + infantry + buildings (war factory, refinery) plus a
    // real harvester (Harvester/Dock/Storage) and a real refinery (Refinery=yes)
    // so the miner dock path is reachable. Short weapon ranges keep combat to the
    // scripted engagements, keeping the scenario deterministic.
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n0=E1\n\n\
         [VehicleTypes]\n0=MTNK\n1=HARV\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=GAWEAP\n1=GAREFN\n\n\
         [E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
         [HARV]\nStrength=600\nArmor=heavy\nSpeed=5\nHarvester=yes\nStorage=28\nDock=GAREFN\n\n\
         [GAWEAP]\nStrength=1000\nArmor=wood\nFoundation=4x3\n\n\
         [GAREFN]\nStrength=1000\nArmor=wood\nRefinery=yes\nFoundation=3x3\n\n\
         [M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
         [SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n\n\
         [AP]\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n",
    );
    RuleSet::from_ini(&ini).expect("harness rules should parse")
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

/// Build the recorded scenario into `sim`. Spawn order fixes stable ids
/// 1..=7 (war factory, refinery, harvester, Allied tank, Allied infantry,
/// Soviet tank, Soviet infantry).
fn seed_scenario(sim: &mut Simulation, rules: &RuleSet, heights: &BTreeMap<(u16, u16), u8>) {
    sim.spawn_from_map(
        &[
            unit("Americans", "GAWEAP", 3, 3, EntityCategory::Structure), // 1
            unit("Americans", "GAREFN", 3, 10, EntityCategory::Structure), // 2
            unit("Americans", "HARV", 8, 12, EntityCategory::Unit),       // 3
            unit("Americans", "MTNK", 10, 8, EntityCategory::Unit),       // 4
            unit("Americans", "E1", 11, 9, EntityCategory::Infantry),     // 5
            unit("Soviet", "MTNK", 40, 8, EntityCategory::Unit),          // 6
            unit("Soviet", "E1", 41, 9, EntityCategory::Infantry),        // 7
        ],
        Some(rules),
        heights,
    );
    // Seed an ore patch near the harvester so it harvests, then returns to the
    // refinery and engages the dock handshake (populating dock_reservations).
    for (rx, ry) in [(12, 13), (13, 13), (12, 14), (13, 14)] {
        sim.production.resource_nodes.insert(
            (rx, ry),
            ResourceNode {
                resource_type: ResourceType::Ore,
                remaining: 5000,
            },
        );
    }
}

/// Scripted commands keyed by `execute_tick` (fires when tick+1 == execute_tick).
fn harness_script() -> Vec<(u64, Command)> {
    vec![
        (2, Command::Move { entity_id: 4, target_rx: 24, target_ry: 8, queue: false, group_id: None }),
        (40, Command::AttackMove { entity_id: 4, target_rx: 38, target_ry: 8, queue: false }),
        (120, Command::Move { entity_id: 6, target_rx: 28, target_ry: 10, queue: false, group_id: None }),
        (300, Command::Stop { entity_id: 4 }),
        (320, Command::Move { entity_id: 4, target_rx: 8, target_ry: 8, queue: false, group_id: None }),
    ]
}

/// Owner of every scripted command (all are issued by the Allied player).
fn due_commands(sim: &Simulation, script: &[(u64, Command)], tick: u64) -> Vec<CommandEnvelope> {
    let owner = sim.interner.get("Americans").expect("Americans interned");
    script
        .iter()
        .filter(|(t, _)| *t == tick + 1)
        .map(|(t, c)| CommandEnvelope::new(owner, *t, c.clone()))
        .collect()
}

#[test]
fn global_skirmish_replay_is_deterministic_and_baseline_stable() {
    let rules = harness_rules();
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    let script = harness_script();

    // ---- Record pass: build a ReplayLog through the live advance_tick path. ----
    let mut rec = Simulation::with_seed(HARNESS_SEED);
    seed_scenario(&mut rec, &rules, &heights);
    let mut log = ReplayLog::new(ReplayHeader {
        version: 1,
        tick_hz: 15,
        seed: HARNESS_SEED,
        map_name: "global_parity_harness".to_string(),
        rules_hash: 0,
    });
    // Coverage tripwire: the harvester (id 3) must be picked up by the miner
    // system — it acquires an ore target via the SearchOre path. (Physical
    // movement to ore and the full dock handshake need movement world-setup
    // beyond this generic harness; the dedicated miner-dock suite owns that
    // coverage. This guards that miner-component creation + the acquisition
    // path stay wired and contribute to the hash.)
    let mut miner_engaged = false;
    for tick in 0..HARNESS_TICKS {
        let due = due_commands(&rec, &script, tick);
        let result = rec.advance_tick(&due, Some(&rules), &heights, Some(&grid), None, HARNESS_TICK_MS);
        if rec
            .substrate
            .entities
            .get(3)
            .and_then(|h| h.miner.as_ref())
            .is_some_and(|m| m.target_ore_cell.is_some())
        {
            miner_engaged = true;
        }
        log.record_tick(tick, due, result.state_hash);
    }
    assert!(
        miner_engaged,
        "the miner system must engage the harvester (acquire an ore target) — \
         else miner-component creation or the SearchOre path regressed"
    );

    // ---- Replay pass: fresh sim, real ReplayRunner::run, assert tick-by-tick. ----
    let mut rep = Simulation::with_seed(HARNESS_SEED);
    seed_scenario(&mut rep, &rules, &heights);
    let replayed =
        ReplayRunner::run(&mut rep, &log, Some(&rules), &heights, Some(&grid), HARNESS_TICK_MS);

    assert_eq!(
        replayed.len(),
        log.ticks.len(),
        "replay tick count must match record"
    );
    for (i, h) in replayed.iter().enumerate() {
        assert_eq!(
            *h, log.ticks[i].state_hash,
            "intra-run determinism: replay tick {i} hash must equal the recorded hash"
        );
    }

    let final_hash = *replayed.last().expect("at least one tick recorded");
    assert_eq!(
        final_hash, GLOBAL_HARNESS_FINAL_HASH,
        "committed global-harness baseline. If this shifts for a real behavior \
         reason, re-baseline once with a one-line documented reason. (paste this \
         `left` value into GLOBAL_HARNESS_FINAL_HASH)"
    );
}
