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
/// S2 (dispatch-time mission authority) left this UNSHIFTED — verified empirically:
/// this scenario's movers are engaged or miners (never pure-Move scoped) on their
/// divergence ticks, so tail authority still wrote every hashed mission value. The
/// S2 hash delta is exercised by the arrival-tick tests in techno_ai.rs instead.
/// S3 facing flip (per-object pre-death barrel read) ALSO left this unshifted —
/// no Unit kill/retarget tick changes a barrel destination in this scenario.
/// Re-baselined ONCE for S3 idle→Guard: every idle machine-less Unit now hashes
/// mission Guard(5) instead of the legacy None placeholder (the gamemd idle
/// selector for ground vehicles) — a hashed-representation fidelity fix, not a
/// behavior drift; movement/combat outputs are byte-identical.
const GLOBAL_HARNESS_FINAL_HASH: u64 = 13100720271148196653;

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

/// S2 go/no-go measurement (not a gate): run the same realistic skirmish and sum the
/// per-tick dispatch churn — live non-miner Units whose dispatch family at the
/// gamemd-faithful host-time point (top-of-tick, post-command, pre-movement) differs
/// from the end-of-tick re-derivation. High churn means our phase-split structure
/// diverges from gamemd's per-object interleaving often enough that the S2 authority
/// flip must dispatch by the host-time value, not the tail projection. The number is
/// the deliverable — printed below; run with `--nocapture` to read it.
#[test]
fn dispatch_churn_measurement_over_global_skirmish() {
    let rules = harness_rules();
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    let script = harness_script();

    let mut sim = Simulation::with_seed(HARNESS_SEED);
    seed_scenario(&mut sim, &rules, &heights);

    let mut total_churn: u64 = 0;
    let mut ticks_with_churn: u32 = 0;
    let mut max_per_tick: u32 = 0;
    let mut iterations: u64 = 0;
    for tick in 0..HARNESS_TICKS {
        let due = due_commands(&sim, &script, tick);
        let result =
            sim.advance_tick(&due, Some(&rules), &heights, Some(&grid), None, HARNESS_TICK_MS);
        if result.dispatch_churn > 0 {
            ticks_with_churn += 1;
            total_churn += result.dispatch_churn as u64;
            max_per_tick = max_per_tick.max(result.dispatch_churn);
        }
        iterations += 1;
    }

    println!(
        "[S2 churn] {HARNESS_TICKS}-tick skirmish: total_unit_tick_churn={total_churn}, \
         ticks_with_churn={ticks_with_churn}/{HARNESS_TICKS}, max_per_tick={max_per_tick}"
    );
    // Guard only that the full span ran (a panic/early-return would void the number).
    assert_eq!(
        iterations, HARNESS_TICKS,
        "churn measurement must run the full skirmish span"
    );
}

const DENSE_SEED: u64 = 0x00BA771E_5EED;
const DENSE_TICKS: u64 = 300;
const DENSE_ROWS: u16 = 10;

/// S2 churn — DENSE arrival case: two facing tank columns (10 Allied vs 10 Soviet) both
/// ordered to converge on the same centre column, so a whole column reaches its
/// destination on the same tick and flips Move→Sleep together. Each Move is issued under
/// ITS OWN owner — the thin generic harness silently rejected one side's move as
/// non-owned, leaving only one real mover. This measures the *simultaneous* per-tick
/// churn the S2 authority flip must survive (a single-mover scenario understates it).
///
/// Scope note: this fixture exercises movement/arrival churn only — the tanks converge
/// but do not engage (no kills; pure-Move auto-acquisition does not fire here), so
/// combat-driven churn (Move→Attack on target acquisition) is NOT measured by this test.
/// Quantifying engagement churn needs a fixture that reliably forces combat (explicit
/// Attack orders + LOS/positioning); deferred to the S2 design phase.
/// Shared construction for the dense converging-battle fixture (20 tanks, two
/// facing columns converging on x=25; per-owner Move script due on tick 2).
/// Used by the churn measurement and the S2 position fingerprint below.
#[allow(clippy::type_complexity)]
fn dense_converging_setup() -> (
    Simulation,
    RuleSet,
    BTreeMap<(u16, u16), u8>,
    PathGrid,
    Vec<(u64, crate::sim::intern::InternedId, Command)>,
) {
    let rules = harness_rules();
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    let mut sim = Simulation::with_seed(DENSE_SEED);
    let mut roster: Vec<MapEntity> = Vec::new();
    for i in 0..DENSE_ROWS {
        roster.push(unit("Americans", "MTNK", 10, 5 + i, EntityCategory::Unit)); // ids 1..=10
    }
    for i in 0..DENSE_ROWS {
        roster.push(unit("Soviet", "MTNK", 40, 5 + i, EntityCategory::Unit)); // ids 11..=20
    }
    sim.spawn_from_map(&roster, Some(&rules), &heights);

    // Both columns converge on x=25, same row — they close together and arrive/stall
    // in formation. Each Move is under its OWN owner (the thin generic harness rejected
    // one side's move as non-owned, leaving a single real mover). Measures the
    // synchronized-arrival churn (a whole column flipping Move→Sleep on one tick).
    let allied = sim.interner.get("Americans").expect("Americans interned");
    let soviet = sim.interner.get("Soviet").expect("Soviet interned");
    let mut script: Vec<(u64, crate::sim::intern::InternedId, Command)> = Vec::new();
    for i in 0..DENSE_ROWS as u64 {
        let y = 5 + i as u16;
        script.push((2, allied, Command::Move { entity_id: 1 + i, target_rx: 25, target_ry: y, queue: false, group_id: None }));
        script.push((2, soviet, Command::Move { entity_id: 11 + i, target_rx: 25, target_ry: y, queue: false, group_id: None }));
    }
    (sim, rules, heights, grid, script)
}

#[test]
fn dispatch_churn_measurement_dense_converging_battle() {
    let (mut sim, rules, heights, grid, script) = dense_converging_setup();

    let mut hist = [0u32; 8]; // per-tick churn buckets; index = churn count clamped to 7
    let mut total_churn: u64 = 0;
    let mut ticks_with_churn: u32 = 0;
    let mut max_per_tick: u32 = 0;
    for tick in 0..DENSE_TICKS {
        let due: Vec<CommandEnvelope> = script
            .iter()
            .filter(|(t, _, _)| *t == tick + 1)
            .map(|(t, owner, c)| CommandEnvelope::new(*owner, *t, c.clone()))
            .collect();
        let result =
            sim.advance_tick(&due, Some(&rules), &heights, Some(&grid), None, HARNESS_TICK_MS);
        let c = result.dispatch_churn;
        if c > 0 {
            ticks_with_churn += 1;
            total_churn += c as u64;
            max_per_tick = max_per_tick.max(c);
        }
        hist[(c as usize).min(7)] += 1;
    }

    let survivors = sim
        .substrate
        .entities
        .iter_sorted()
        .filter(|(_, e)| e.category == EntityCategory::Unit && !e.dying)
        .count();
    println!(
        "[S2 churn DENSE] {DENSE_TICKS}-tick converging battle (20 tanks): \
         total_unit_tick_churn={total_churn}, ticks_with_churn={ticks_with_churn}/{DENSE_TICKS}, \
         max_per_tick={max_per_tick}, survivors={survivors}/20"
    );
    println!("[S2 churn DENSE] per-tick churn histogram [churn=0..=7+]: {hist:?}");
    assert!(
        total_churn >= 1,
        "a 20-tank converging battle must produce churn (arrivals + target acquisition)"
    );
}

/// S2 movement-neutrality tripwire: per-tick position fingerprint of the dense
/// converging scenario, captured PRE-flip (T2). The S2 dispatch flip changes
/// only `mission.current`/`tick_counter` write points — if this fingerprint
/// shifts, the flip moved someone: that is a bug, never a re-baseline.
const POSITION_FINGERPRINT: u64 = 12834935063109785345; // captured pre-flip (S2 T2)

#[test]
fn s2_dense_scenario_position_fingerprint_stable() {
    use std::hash::{Hash, Hasher};
    let (mut sim, rules, heights, grid, script) = dense_converging_setup();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for tick in 0..DENSE_TICKS {
        let due: Vec<CommandEnvelope> = script
            .iter()
            .filter(|(t, _, _)| *t == tick + 1)
            .map(|(t, owner, c)| CommandEnvelope::new(*owner, *t, c.clone()))
            .collect();
        let _ =
            sim.advance_tick(&due, Some(&rules), &heights, Some(&grid), None, HARNESS_TICK_MS);
        for (id, e) in sim.substrate.entities.iter_sorted() {
            (id, e.position.rx, e.position.ry, e.position.sub_x, e.position.sub_y).hash(&mut h);
        }
    }
    assert_eq!(
        h.finish(),
        POSITION_FINGERPRINT,
        "S2 must not change any position sequence (captured pre-flip in T2)"
    );
}
