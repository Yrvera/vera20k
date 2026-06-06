//! P5c — the Factory/House authority-flip replay/parity ACCEPTANCE GATE.
//!
//! The ratification of the P5b authority flip (the first hashed-state change).
//! Drives REAL production commands (`QueueProduction` / `TogglePauseProduction` /
//! `CancelProductionByType`) through `advance_tick` and the shared replay harness
//! (`ReplayLog` / `ReplayRunner`), and asserts:
//!
//!   - (A) DETERMINISM — a recorded production command stream, run live TWICE and
//!     replayed once through `ReplayRunner`, yields a bit-identical per-tick
//!     `state_hash` timeline. This is the lockstep ratification of the flip: the
//!     newly-hashed `Factory`/`Economy` state machine adds no nondeterminism.
//!   - (B) CONSERVATION (C15) — over a refund-free replay, EVERY tick conserves
//!     `Σ_owners(house.credits + economy.spent_credits) == Σ_owners(initial)`.
//!     The per-step charge moves credits out of the one wallet into `spent_credits`
//!     and nowhere else; no credit is created or destroyed by the charge machinery.
//!
//! Pre-flip-baseline observable equivalence (P5c part C) is intentionally DEFERRED:
//! the pre-flip charge path is retired at the flip, and the one intended difference
//! (the x0.9-free producer cadence) is already documented and asserted by the
//! producer's own tests. The determinism + conservation gates here are the
//! load-bearing ratification of the flip.
//!
//! Cross-sim replay invariant: a command carries owner/type as `InternedId`, and
//! `advance_tick` resolves those IDs against the sim's OWN interner. `scenario()`
//! is fully deterministic (interns rules → types → owners in a fixed order), so any
//! two sims it produces have identical interner state — an `InternedId` minted
//! against one is valid in the other. Both the recording sim and the replay sim are
//! built by `scenario()`, so the recorded `CommandEnvelope`s resolve correctly on
//! playback.

use std::collections::BTreeMap;

use super::tests::{build_catalog_rules, spawn_structure};
use super::{queue_view_for_owner, BuildQueueState, ProductionCategory};
use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::command::{Command, CommandEnvelope, QueueMode};
use crate::sim::house_state::HouseState;
use crate::sim::intern::InternedId;
use crate::sim::replay::{ReplayHeader, ReplayLog, ReplayRunner};
use crate::sim::world::Simulation;

const TICK_MS: u32 = 67;
const START_CREDITS: i32 = 50_000;

/// Two funded human owners (Americans, Alliance), each owning a Construction Yard
/// + Barracks + War Factory + Air Force HQ, so the [`build_catalog_rules`] units
/// (E1 / MTNK / ORCA) are Strict-mode buildable. No power plant is needed — none of
/// these structures drains power, so the producer runs at full rate.
///
/// Fully deterministic so the cross-sim replay invariant holds (see module docs).
fn scenario() -> (Simulation, RuleSet, BTreeMap<(u16, u16), u8>) {
    let mut sim = Simulation::new();
    let rules = build_catalog_rules();
    rules.intern_all_ids(&mut sim.interner);
    sim.resolve_type_handles(&rules);

    let owners = [("Americans", 0u8, 10u16), ("Alliance", 1u8, 30u16)];
    for (i, (owner, side, base_x)) in owners.iter().enumerate() {
        let oid = sim.interner.intern(owner);
        sim.houses
            .insert(oid, HouseState::new(oid, *side, None, true, START_CREDITS, 10));
        let sid = (i as u64) * 10 + 1;
        spawn_structure(&mut sim, sid, owner, "GACNST", *base_x, 10);
        spawn_structure(&mut sim, sid + 1, owner, "GAPILE", *base_x + 2, 10);
        spawn_structure(&mut sim, sid + 2, owner, "GAWEAP", *base_x + 4, 10);
        spawn_structure(&mut sim, sid + 3, owner, "GAAIRC", *base_x + 6, 10);
    }
    (sim, rules, BTreeMap::new())
}

fn env(owner: InternedId, tick: u64, payload: Command) -> CommandEnvelope {
    CommandEnvelope::new(owner, tick, payload)
}

fn queue(owner: InternedId, type_id: InternedId, tick: u64) -> CommandEnvelope {
    env(
        owner,
        tick,
        Command::QueueProduction {
            owner,
            type_id,
            mode: QueueMode::Append,
        },
    )
}

/// Resolve the owner/type IDs the streams reference. All are already interned by
/// `scenario()`, so `.get()` never returns `None` and never mints a new ID.
fn ids(sim: &Simulation) -> (InternedId, InternedId, InternedId, InternedId) {
    (
        sim.interner.get("Americans").expect("Americans interned"),
        sim.interner.get("Alliance").expect("Alliance interned"),
        sim.interner.get("E1").expect("E1 interned"),
        sim.interner.get("MTNK").expect("MTNK interned"),
    )
}

/// A rich stream exercising the full hashed-state machinery: a same-tick two-Begin
/// from two owners, a second category (Vehicle) for one owner, a FIFO tail (a second
/// infantry queued behind the first), a mid-build pause + resume, and a mid-build
/// cancel (partial-refund active-abandon).
fn rich_command_stream(sim: &Simulation) -> Vec<CommandEnvelope> {
    let (am, al, e1, mtnk) = ids(sim);
    vec![
        // tick 1 — same-tick two-Begin (different owners) + a Vehicle for Americans.
        queue(am, e1, 1),
        queue(al, e1, 1),
        queue(am, mtnk, 1),
        // tick 2 — a second infantry behind the first (FIFO tail) for Americans.
        queue(am, e1, 2),
        // pause Americans' Vehicle build, then resume.
        env(
            am,
            8,
            Command::TogglePauseProduction {
                owner: am,
                category: ProductionCategory::Vehicle,
            },
        ),
        env(
            am,
            25,
            Command::TogglePauseProduction {
                owner: am,
                category: ProductionCategory::Vehicle,
            },
        ),
        // Alliance cancels its active infantry mid-build (partial refund).
        env(
            al,
            12,
            Command::CancelProductionByType {
                owner: al,
                type_id: e1,
            },
        ),
    ]
}

/// A refund-free stream (only Begins) for the conservation gate: with no cancels,
/// no refund ever fires, so `credits + spent_credits` is exactly conserved.
fn refund_free_stream(sim: &Simulation) -> Vec<CommandEnvelope> {
    let (am, al, e1, mtnk) = ids(sim);
    vec![
        queue(am, e1, 1),
        queue(al, e1, 1),
        queue(am, mtnk, 1),
        queue(al, mtnk, 2),
        queue(am, e1, 2), // a tail behind the first infantry
    ]
}

/// Drive `sim` for `ticks` ticks, dispatching due commands each tick, recording a
/// `ReplayLog` + the per-tick `state_hash` timeline.
fn record(
    sim: &mut Simulation,
    rules: &RuleSet,
    heights: &BTreeMap<(u16, u16), u8>,
    mut pending: Vec<CommandEnvelope>,
    ticks: u64,
) -> (Vec<u64>, ReplayLog) {
    let mut log = ReplayLog::new(ReplayHeader {
        version: 1,
        tick_hz: 15,
        seed: 0,
        map_name: "p5c_factory_replay".to_string(),
        rules_hash: 0,
    });
    let mut hashes = Vec::with_capacity(ticks as usize);
    for _ in 0..ticks {
        let execute_tick = sim.tick + 1;
        let mut due: Vec<CommandEnvelope> = Vec::new();
        pending.retain(|c| {
            if c.execute_tick <= execute_tick {
                due.push(c.clone());
                false
            } else {
                true
            }
        });
        let r = sim.advance_tick(&due, Some(rules), heights, None, None, TICK_MS);
        hashes.push(r.state_hash);
        log.record_tick(r.tick, due, r.state_hash);
    }
    (hashes, log)
}

/// (P5d derived-state) An underfunded mid-build factory (on_hold) renders as Building in
/// the sidebar build queue, NOT "On Hold"/NoFunds — the pre-P5d front never surfaced NoFunds
/// during a stall (it stayed Building; on_hold is internal). The sibling of the blocked-exit
/// Done case: the derived view state must reproduce the exact observed label set
/// {Building, Paused, Done, Queued}.
#[test]
fn derived_view_state_stays_building_on_underfunded_stall() {
    let (mut sim, rules, _heights) = scenario();
    let (am, _, e1, _) = ids(&sim);
    // Arm an E1 build directly, then simulate a mid-build underfunded stall.
    sim.production
        .factory_shadow
        .enqueue(am, ProductionCategory::Infantry, e1, 1, 100, 200);
    {
        let f = sim
            .production
            .factory_shadow
            .test_factory_mut(am, ProductionCategory::Infantry)
            .expect("infantry factory armed");
        f.progress = 10;
        f.on_hold = true; // underfunded stall: not paused, not complete
    }
    let view = queue_view_for_owner(&sim, &rules, "Americans");
    assert_eq!(view.len(), 1, "the single armed build projects to one view item");
    assert_eq!(
        view[0].state,
        BuildQueueState::Building,
        "an on_hold (underfunded) stall renders Building, never NoFunds"
    );
}

fn delivered_unit_count(sim: &Simulation) -> usize {
    sim.substrate
        .entities
        .values()
        .filter(|e| matches!(e.category, EntityCategory::Unit | EntityCategory::Infantry))
        .count()
}

/// (A) DETERMINISM — the lockstep ratification of the flip. A recorded production
/// command stream run live twice AND replayed through `ReplayRunner` yields a
/// bit-identical per-tick `state_hash` timeline.
#[test]
fn factory_flip_replay_is_bit_identical_across_runs_and_playback() {
    const TICKS: u64 = 120;

    // Run 1 — live record.
    let (mut s1, rules, heights) = scenario();
    let cmds = rich_command_stream(&s1);
    let (timeline_live, log) = record(&mut s1, &rules, &heights, cmds.clone(), TICKS);

    // Run 2 — live record again (pure repeatability).
    let (mut s2, rules2, heights2) = scenario();
    let (timeline_live2, _) = record(&mut s2, &rules2, &heights2, cmds, TICKS);
    assert_eq!(
        timeline_live, timeline_live2,
        "two live runs of the same command stream must produce an identical per-tick hash timeline"
    );

    // Run 3 — replay the recorded log through the shared ReplayRunner.
    let (mut s3, rules3, heights3) = scenario();
    let timeline_playback =
        ReplayRunner::run(&mut s3, &log, Some(&rules3), &heights3, None, TICK_MS);
    assert_eq!(
        timeline_live, timeline_playback,
        "replay playback must reproduce the live hash timeline bit-for-bit"
    );

    // The gate must be exercising hashed state, not asserting on a no-op.
    let distinct: std::collections::BTreeSet<u64> = timeline_live.iter().copied().collect();
    assert!(
        distinct.len() > 1,
        "the command stream must actually move state_hash over the run (it did not)"
    );
}

/// (B) CONSERVATION (C15) — over a refund-free replay, every tick conserves the
/// global money pool: `Σ(house.credits + economy.spent_credits) == Σ(initial)`.
/// The per-step charge is the only mover of credits; nothing is created or lost.
#[test]
fn economy_conservation_over_replay() {
    const TICKS: u64 = 600;

    let (mut sim, rules, heights) = scenario();
    let (am, al, _, _) = ids(&sim);
    let pending = refund_free_stream(&sim);

    let initial: i64 = [am, al]
        .iter()
        .map(|o| sim.houses[o].credits as i64)
        .sum();

    let mut pending = pending;
    let mut any_spent = false;
    for _ in 0..TICKS {
        let execute_tick = sim.tick + 1;
        let mut due: Vec<CommandEnvelope> = Vec::new();
        pending.retain(|c| {
            if c.execute_tick <= execute_tick {
                due.push(c.clone());
                false
            } else {
                true
            }
        });
        sim.advance_tick(&due, Some(&rules), &heights, None, None, TICK_MS);

        let total: i64 = [am, al]
            .iter()
            .map(|o| {
                let h = &sim.houses[o];
                h.credits as i64 + h.economy.spent_credits as i64
            })
            .sum();
        assert_eq!(
            total, initial,
            "tick {}: credits+spent_credits must equal the initial pool (no money created/destroyed)",
            sim.tick
        );
        if [am, al]
            .iter()
            .any(|o| sim.houses[o].economy.spent_credits > 0)
        {
            any_spent = true;
        }
    }

    assert!(
        any_spent,
        "the per-step charge must actually have moved credits over the replay"
    );
    assert!(
        delivered_unit_count(&sim) >= 1,
        "at least one build must complete and deliver over the replay (the full charge cycle)"
    );
}

/// (B') CONSERVATION THROUGH THE PARTIAL-REFUND BRANCH (C8/C15) — the cancel of a
/// mid-build active object refunds exactly the already-charged portion
/// (`original_balance − balance`) back to the one wallet and nowhere else. The
/// global pool is conserved once refunds are accounted for:
/// `Σ(credits + spent_credits) − cumulative_refunded == initial` at every tick.
///
/// `cumulative_refunded` is measured independently of the engine — every per-owner
/// credit INCREASE is a refund (this scenario has no deposits/income), so summing
/// the positive per-owner deltas reconstructs total refunded without reading factory
/// internals. The cancelling owner builds ONLY the cancelled item, so on the cancel
/// tick its sole credit movement is the refund (the build is removed before the
/// Phase-7 charge sweep), keeping the per-owner delta a clean refund signal.
#[test]
fn economy_conservation_through_cancel_refund() {
    const TICKS: u64 = 300;
    const CANCEL_TICK: u64 = 8;

    let (mut sim, rules, heights) = scenario();
    let (am, al, e1, mtnk) = ids(&sim);

    // Americans: ONLY an MTNK (Cost 900 -> guaranteed mid-build at the cancel tick),
    // cancelled mid-build. Alliance: an E1 + an MTNK that run uninterrupted.
    let mut pending = vec![
        queue(am, mtnk, 1),
        queue(al, e1, 1),
        queue(al, mtnk, 2),
        env(
            am,
            CANCEL_TICK,
            Command::CancelProductionByType {
                owner: am,
                type_id: mtnk,
            },
        ),
    ];

    let initial: i64 = [am, al].iter().map(|o| sim.houses[o].credits as i64).sum();
    let mtnk_cost = sim
        .object_type(mtnk, &rules)
        .map(|o| o.cost.max(0))
        .expect("MTNK has a cost") as i64;

    let mut prev: BTreeMap<InternedId, i32> =
        [am, al].iter().map(|&o| (o, sim.houses[&o].credits)).collect();
    let mut cumulative_refunded: i64 = 0;

    for _ in 0..TICKS {
        let execute_tick = sim.tick + 1;
        let mut due: Vec<CommandEnvelope> = Vec::new();
        pending.retain(|c| {
            if c.execute_tick <= execute_tick {
                due.push(c.clone());
                false
            } else {
                true
            }
        });
        sim.advance_tick(&due, Some(&rules), &heights, None, None, TICK_MS);

        // Every per-owner credit INCREASE is a refund (no deposits in this scenario).
        for &o in &[am, al] {
            let now = sim.houses[&o].credits;
            let delta = now - *prev.get(&o).unwrap();
            if delta > 0 {
                cumulative_refunded += delta as i64;
            }
            prev.insert(o, now);
        }

        let pool: i64 = [am, al]
            .iter()
            .map(|o| {
                let h = &sim.houses[o];
                h.credits as i64 + h.economy.spent_credits as i64
            })
            .sum();
        assert_eq!(
            pool - cumulative_refunded,
            initial,
            "tick {}: pool minus refunds must equal the initial pool (conservation through cancel)",
            sim.tick
        );
    }

    // The refund fired and was PARTIAL (mid-build), not the full cost — the C8 fix
    // (the legacy `.rev()` full refund is the retired DRIFT).
    assert!(
        cumulative_refunded > 0 && cumulative_refunded < mtnk_cost,
        "the cancel must refund the partial charged portion (0 < {} < {})",
        cumulative_refunded,
        mtnk_cost
    );
    // Americans' MTNK left the queue-of-record on cancel (the registry factory is gone —
    // P5d: the queue-of-record lives in the registry, pruned when idle).
    assert!(
        sim.production
            .factory_shadow
            .view(am, ProductionCategory::Vehicle)
            .is_none(),
        "the cancelled active build left the Americans Vehicle factory"
    );
}
