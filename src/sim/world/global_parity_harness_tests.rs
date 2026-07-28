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
/// AT-8: ticks at which the per-stream RNG cursors are compared record-vs-replay
/// (after the tick at this index executes).
const STREAM_CHECKPOINT_TICKS: &[u64] = &[149, 299, 449, 599];

/// AT-8 proper: ABSOLUTE committed per-stream fingerprints at the final
/// checkpoint (tick 599). Record-vs-replay equality alone cannot catch a
/// deterministic cross-stream misroute — both passes run the same code, so a
/// misrouted draw appears identically in both. Only committed values detect
/// it, and when a legitimate change shifts the total hash, these localize
/// WHICH stream moved. Same re-baseline ceremony as GLOBAL_HARNESS_FINAL_HASH
/// (one documented re-baseline per behavior-bearing change; paste the failing
/// `left` values).
/// Baselined at SC-2 review hardening. scenario == main here: this scripted
/// scenario consumes ZERO draws from either gameplay stream (they stay at the
/// identical post-seed state), and MapGen holds the fresh native Seed(0)
/// fingerprint — so ANY future draw in this scenario shifts exactly one
/// component loudly.
/// Re-baselined after MapGen was split from the scenario seed. The new MapGen
/// value was identical in two focused runs with pristine fresh Seed(0) MapGen.
/// This remains a Rust regression ratchet, not a gamemd parity reference.
/// Re-baselined with the tube-gate fix (off-tube non-adjacent path steps are
/// no longer killed as failed tube traversals): the harness harvester's
/// sharp-turn outbound legs now execute instead of dying on their issue tick,
/// so it reaches ore and Reduce_Tiberium's growth reseeding consumes scenario
/// draws this fixture never reached before. Streams 1 and 2 are unchanged and
/// the total hash moved with stream 0 — a behavior-bearing shift, not a
/// misroute.
/// Re-baselined with the native Mission_Harvest per-path dispatch delays:
/// the return/idle/still-driving handler exits now draw the native
/// RandomRanged(0,2) Rate-epilogue jitter on the scenario stream, so this
/// fixture's harvester consumes scenario draws on every non-productive
/// dispatch. Streams 1 and 2 are unchanged and the total hash moved with
/// stream 0 — a behavior-bearing shift, not a misroute.
const FINAL_STREAM_STATES: (u64, u64, u64) = (
    4301199653360695687,
    4175722561206807420,
    2082941527059030371,
);

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
/// Re-baselined for SC-2: session identity (seed, map name, theater, bounds,
/// MP start table, slot->house) folded into the state hash — every absolute
/// hash shifts once by composition; the tick-by-tick rec-vs-replay equality
/// and the per-stream cursor pins prove no behavioral movement.
/// Re-measured at the S3 × SC-2 merge (both deltas combined; value from the
/// merged tree's green run — neither side's pre-merge value can be correct).
/// Re-baselined for S4b: the hashed `damage_particle_live_until` `+0x308`-
/// equivalent field folds an extra 0 per entity — a composition shift, NOT a
/// behavior drift. Proven: with the fold line disabled this baseline held its
/// prior value (so S4b moved zero RNG and changed no committed scenario), and
/// the tick-by-tick rec-vs-replay equality below still passes.
/// W1 (mission-cadence: G5/G6/L20 + L9/L10 RandomRanged(0,2) draws) left this
/// UNSHIFTED — verified empirically (this baseline + the per-stream cursor pins
/// held their values, and the harness runs deterministically 2×). The harness
/// harvester (id 3) acquires an ore target but never completes the refinery dock
/// handshake — the coverage tripwire below only asserts ore-target acquisition,
/// and the dock/unload cadence paths that carry the new draws are never reached
/// in this scenario. The W1 cadence + RNG-draw determinism is covered by the
/// dedicated miner-dock suite (accepted_face_sync_handoff_draws_one_scenario_rng,
/// state_four_exit_draws_and_applies_resume_jitter, et al.) instead.
/// Re-baselined after MapGen became an independent fresh Seed(0) stream. The
/// value was identical in two focused runs; this is a Rust regression ratchet,
/// not a gamemd parity reference.
/// The later aircraft-RTB rationale was invalid: this fixture contains no
/// aircraft. A pristine `fafc0ba5` run reproduced the preceding
/// `7340892273004731329` baseline, proving the committed replacement was captured
/// from a contaminated worktree.
/// Re-baselined for lockstep hash completeness: body-facing presence,
/// damage-fire state/animation IDs, locomotor hover/altitude state, per-house
/// difficulty, Spark state, and `AnimStore` now join the hash. A current-tree
/// legacy-schema probe reproduced `7340892273004731329` exactly; record/replay
/// tick equality and the absolute RNG pins also remained unchanged. This shift
/// is therefore composition-only and remains a Rust regression ratchet, not
/// gamemd parity evidence.
/// Re-baselined for snapshot/hash schema v28: independent lifecycle axes,
/// lifecycle bookkeeping, and ordered pending deletion now join the hash. The
/// current-tree legacy-schema probe reproduces the prior value, record/replay
/// equality remains exact, and all three absolute RNG pins are unchanged.
/// Re-baselined after the reviewed outbound and far-return miner Drive
/// authority changes (`932fc5e8`, `3ff8f43c`). Parent/child isolation proved
/// that each change moved this fixture's hashed navigation/Drive state while
/// record/replay equality and all three absolute RNG pins stayed unchanged.
/// This is a behavior-bearing Rust regression ratchet, not gamemd parity
/// evidence.
/// Re-baselined for the Mission authority flip: MissionCom is verb-owned
/// (commands queue via the event-execute shape; the object-AI host ticks
/// `+0xC4` for every live category and promotes queued missions
/// Ready→Commence; the per-tick legacy projection is deleted). Every hashed
/// mission field changes value — including under the legacy pre-v29
/// composition, which folds the reduced mission subset — so all three
/// constants shift together while record/replay equality and all three
/// absolute RNG pins stay unchanged (the verbs draw nothing).
/// Re-baselined for the Harvest handler absorption (A1): the miner FSM now
/// dispatches from the per-object AI host BEFORE Phase-1 ground movement (the
/// native handler→locomotion order) instead of the late production phase, the
/// FSM cursor moved from the hashed miner block into
/// `MissionCom.handler_state`, and every miner's dispatch timer advances per
/// dispatch (post-handler epilogue write). This shifts the harness harvester's
/// hashed mission/miner state under every schema composition — including the
/// legacy reconstructions — so all three constants move together. All three
/// absolute RNG stream pins and record/replay tick equality held unchanged
/// (this scenario's miner never docks, so no draw moved).
/// Re-baselined with the native same-tick drive-arrival owner clear: the
/// harness harvester's NavCom now clears on the arrival tick itself (no
/// deferred pass), so its dispatch resumes one tick earlier — a
/// behavior-bearing timing shift in hashed navigation/mission/position
/// state. All three absolute RNG stream pins held their values (the arrival
/// clear draws nothing), and record/replay tick equality still holds.
/// Re-baselined with the native Mission_Harvest per-path dispatch delays:
/// a behavior-bearing shift (dispatch cadence + scenario-stream draws), so
/// the legacy-schema probes move together with the live hash.
const GLOBAL_HARNESS_PRE_LIFECYCLE_V28_HASH: u64 = 4844629824678724271;
const GLOBAL_HARNESS_PRE_MISSION_V29_HASH: u64 = 2401965642562130820;
// Snapshot/hash schema v29 originally added the exact Mission/readiness state.
// Its schema shift was composition-only; the later behavior-bearing Drive,
// authority-flip, and Harvest-absorption re-baselines are documented above.
// All remain Rust regression ratchets, not gamemd evidence.
/// Re-baselined with the tube-gate fix (see FINAL_STREAM_STATES): the
/// harvester's outbound Drive moves now survive their issue tick, changing
/// positions, mission/miner state, and scenario-stream consumption in this
/// fixture. Both schema probes shift with it (movement diverges from early
/// ticks). Record/replay tick equality still holds.
/// Re-baselined with the native Mission_Harvest per-path dispatch delays
/// (see FINAL_STREAM_STATES): the harvester's dispatch cadence and
/// scenario-stream consumption changed in this fixture, shifting positions
/// and timers from the first return leg on. Record/replay tick equality
/// still holds.
const GLOBAL_HARNESS_FINAL_HASH: u64 = 0xADCD_2D1C_ABFF_D48D;

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
        (
            2,
            Command::Move {
                entity_id: 4,
                target_rx: 24,
                target_ry: 8,
                queue: false,
                group_id: None,
            },
        ),
        (
            40,
            Command::AttackMove {
                entity_id: 4,
                target_rx: 38,
                target_ry: 8,
                queue: false,
            },
        ),
        (
            120,
            Command::Move {
                entity_id: 6,
                target_rx: 28,
                target_ry: 10,
                queue: false,
                group_id: None,
            },
        ),
        (300, Command::Stop { entity_id: 4 }),
        (
            320,
            Command::Move {
                entity_id: 4,
                target_rx: 8,
                target_ry: 8,
                queue: false,
                group_id: None,
            },
        ),
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
    // AT-8 stream pins: per-stream cursor fingerprints captured at checkpoint
    // ticks during record, re-asserted in replay. Total-hash equality can mask
    // a draw routed to the wrong stream when a compensating error exists;
    // per-stream checkpoints catch misrouting directly.
    let mut recorded_streams: Vec<(u64, u64, u64, u64)> = Vec::new();
    for tick in 0..HARNESS_TICKS {
        let due = due_commands(&rec, &script, tick);
        let result = rec.advance_tick(
            &due,
            Some(&rules),
            &heights,
            Some(&grid),
            None,
            HARNESS_TICK_MS,
        );
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
        if STREAM_CHECKPOINT_TICKS.contains(&tick) {
            recorded_streams.push((
                tick,
                rec.scenario_rng.state(),
                rec.main_rng.state(),
                rec.mapgen_rng.state(),
            ));
        }
    }
    assert!(
        miner_engaged,
        "the miner system must engage the harvester (acquire an ore target) — \
         else miner-component creation or the SearchOre path regressed"
    );

    // ---- Replay pass: fresh sim, real ReplayRunner::run, assert tick-by-tick.
    // The replay is fed through the SAME ReplayRunner::run path the live game
    // uses, chunked at the stream checkpoints so the per-stream cursors can be
    // pinned between chunks (chunking preserves the exact advance_tick call
    // sequence; ReplayRunner::run is a plain fold over entries). ----
    let mut rep = Simulation::with_seed(HARNESS_SEED);
    seed_scenario(&mut rep, &rules, &heights);
    let mut replayed: Vec<u64> = Vec::with_capacity(log.ticks.len());
    let mut replayed_streams: Vec<(u64, u64, u64, u64)> = Vec::new();
    let mut chunk_start = 0usize;
    for &checkpoint in STREAM_CHECKPOINT_TICKS {
        let chunk_end = (checkpoint as usize + 1).min(log.ticks.len());
        let chunk = ReplayLog {
            header: log.header.clone(),
            ticks: log.ticks[chunk_start..chunk_end].to_vec(),
        };
        replayed.extend(ReplayRunner::run(
            &mut rep,
            &chunk,
            Some(&rules),
            &heights,
            Some(&grid),
            HARNESS_TICK_MS,
        ));
        replayed_streams.push((
            checkpoint,
            rep.scenario_rng.state(),
            rep.main_rng.state(),
            rep.mapgen_rng.state(),
        ));
        chunk_start = chunk_end;
    }
    if chunk_start < log.ticks.len() {
        let tail = ReplayLog {
            header: log.header.clone(),
            ticks: log.ticks[chunk_start..].to_vec(),
        };
        replayed.extend(ReplayRunner::run(
            &mut rep,
            &tail,
            Some(&rules),
            &heights,
            Some(&grid),
            HARNESS_TICK_MS,
        ));
    }
    assert_eq!(
        recorded_streams, replayed_streams,
        "per-stream cursor consistency: a nondeterminism moved streams between record and replay"
    );
    let (_, final_scen, final_main, final_mapgen) =
        *recorded_streams.last().expect("final checkpoint recorded");
    let final_hash = *replayed.last().expect("at least one tick recorded");
    println!(
        "[global parity] final_hash={final_hash:016X} \
         streams={final_scen:016X},{final_main:016X},{final_mapgen:016X}"
    );
    assert_eq!(
        (final_scen, final_main, final_mapgen),
        FINAL_STREAM_STATES,
        "AT-8 absolute per-stream pin at tick 599: a stream's committed \
         fingerprint moved. If a real behavior change shifted it, re-baseline \
         ONCE with a one-line documented reason (paste this `left` tuple into \
         FINAL_STREAM_STATES); the shifted component tells you WHICH stream \
         consumed differently — a lone shift in one stream with an unchanged \
         total-hash baseline is a misroute, never a re-baseline."
    );

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

    assert_eq!(
        rep.state_hash_before_lifecycle_v28_and_mission_v29(),
        GLOBAL_HARNESS_PRE_LIFECYCLE_V28_HASH,
        "pre-v28/pre-v29 schema probe must reproduce the historical baseline"
    );
    assert_eq!(
        rep.state_hash_without_mission_v29(),
        GLOBAL_HARNESS_PRE_MISSION_V29_HASH,
        "v29 provenance probe must reproduce the prior live v28 baseline; otherwise this is behavior drift"
    );
    assert_eq!(
        final_hash, GLOBAL_HARNESS_FINAL_HASH,
        "committed global-harness baseline drifted. Do not copy the observed value: \
         first prove whether behavior, RNG routing, or intentional hash composition \
         changed, and document reproducible baseline provenance"
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
        script.push((
            2,
            allied,
            Command::Move {
                entity_id: 1 + i,
                target_rx: 25,
                target_ry: y,
                queue: false,
                group_id: None,
            },
        ));
        script.push((
            2,
            soviet,
            Command::Move {
                entity_id: 11 + i,
                target_rx: 25,
                target_ry: y,
                queue: false,
                group_id: None,
            },
        ));
    }
    (sim, rules, heights, grid, script)
}

/// S2 movement-neutrality tripwire: per-tick position fingerprint of the dense
/// converging scenario, captured PRE-flip (T2). The S2 dispatch flip changes
/// only `mission.current`/`tick_counter` write points — if this fingerprint
/// shifts, the flip moved someone: that is a bug, never a re-baseline.
/// Re-baselined ONCE after the flip validation closed, for the tube-gate fix:
/// off-tube non-adjacent path steps (sharp-turn fallback bumps) are no longer
/// killed on their issue tick, so movers that previously froze now drive —
/// an intended movement-behavior change, not dispatch-order drift.
const POSITION_FINGERPRINT: u64 = 18354164349101625193;

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
        let _ = sim.advance_tick(
            &due,
            Some(&rules),
            &heights,
            Some(&grid),
            None,
            HARNESS_TICK_MS,
        );
        for (id, e) in sim.substrate.entities.iter_sorted() {
            (
                id,
                e.position.rx,
                e.position.ry,
                e.position.sub_x,
                e.position.sub_y,
            )
                .hash(&mut h);
        }
    }
    assert_eq!(
        h.finish(),
        POSITION_FINGERPRINT,
        "S2 must not change any position sequence (captured pre-flip in T2)"
    );
}
