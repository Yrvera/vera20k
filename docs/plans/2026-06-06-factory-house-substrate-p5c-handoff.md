# P5c Replay/Parity Acceptance Gate — SESSION HANDOFF

**Date:** 2026-06-06
**Branch:** `factory-house-substrate-p1p2`
**Status:** ✅ **P5c parts A + B implemented + green. Part C deferred (user decision).**

---

## TL;DR

P5b (the authority flip) was committed this session as `1db41ebf` (hunk-staged — the
concurrent miner session's files were left untouched). Then P5c (a.k.a. P9), the
replay/parity ACCEPTANCE GATE that ratifies the flip, was implemented: **A (replay
determinism) + B (economy conservation)**. Part **C (pre-flip-baseline observable
equivalence) was DEFERRED** by explicit user decision — the pre-flip charge path is
retired at the flip, and the one intended difference (the x0.9-free producer cadence)
is already documented + asserted by the producer's own tests, so a parent-commit
baseline capture is low marginal proof.

---

## What got DONE this session

1. **Committed P5b** (`1db41ebf`, 12 files). Staged ONLY the P5b hunks; left the
   concurrent miner session's 5 files (`cell_rect.rs`, `miner/*`, untracked
   `harvest_mission.rs`) unstaged. Full suite was green before commit (lib 3725/0/19).

2. **P5c A + B** — new internal test module `src/sim/production/production_replay_tests.rs`
   (wired into `src/sim/production/mod.rs` as `mod replay_tests`). Three gates, all
   driven by REAL production commands (`QueueProduction` / `TogglePauseProduction` /
   `CancelProductionByType`) through `advance_tick` + the shared replay harness
   (`ReplayLog` / `ReplayRunner`):
   - **(A) `factory_flip_replay_is_bit_identical_across_runs_and_playback`** — a recorded
     command stream (same-tick two-Begin across two owners, a second category, a FIFO
     tail, a pause+resume, a mid-build cancel) run live TWICE and replayed once through
     `ReplayRunner` yields a bit-identical per-tick `state_hash` timeline. The lockstep
     ratification: the newly-hashed Factory/Economy state machine adds no nondeterminism.
   - **(B) `economy_conservation_over_replay`** — over a refund-free replay, EVERY tick
     conserves `Σ(house.credits + economy.spent_credits) == Σ(initial)`; asserts the
     charge actually moved credits and ≥1 build completed + delivered (the full
     charge→complete→deliver cycle).
   - **(B′) `economy_conservation_through_cancel_refund`** — the C8/C15 partial-refund
     branch: a mid-build cancel refunds exactly the charged portion and nowhere else;
     `Σ(credits + spent) − cumulative_refunded == initial` at every tick, with the refund
     PARTIAL (0 < refunded < full cost — the retired `.rev()` full-refund DRIFT stays
     retired). `cumulative_refunded` is measured independently (every per-owner credit
     INCREASE is a refund; no deposits in the scenario).

   Scenario: `build_catalog_rules` (E1 Cost 200, MTNK Cost 900) + two funded owners,
   each with GACNST + GAPILE + GAWEAP + GAAIRC spawned so the units are Strict-mode
   buildable. Cross-sim replay invariant relies on `scenario()` being fully deterministic
   (interns rules→types→owners in a fixed order → identical interner state across sims →
   command `InternedId`s valid on playback).

**Already covered elsewhere (NOT duplicated in P5c):** the static v18 serde round-trip of
the registry (`snapshot_roundtrip_factory_registry`, production_shadow_tests.rs:375) and
post-load cache-rebuild determinism (`saveload_rebuild_is_deterministic`, snapshot.rs).

---

## Part C — DEFERRED (rationale)

C = pre-flip-baseline-vs-post-flip observable-output equivalence. Deferred because:
- The pre-flip charge path (legacy frames timer + x0.9 build-time family) is RETIRED at
  the flip — a true baseline needs a parent-commit (dc7a34d9 / P5a) fixture capture.
- The ONE intended difference is the x0.9-free producer cadence, which changes build
  TIMING — so any baseline comparison is timing-invariant-observables-only (delivered set,
  total spent, steady-state credits), not per-tick. That cross-check is largely subsumed
  by A (determinism) + B (conservation) + the producer's own x0.9-free tests.
- If revisited: check out P5a, record observable outputs (delivered-unit ticks, credit
  timeline, total spent) into a committed JSON fixture, then assert post-flip equivalence
  on the timing-invariant observables, explicitly allowing completion-tick to differ.

---

## Deferred follow-ups (unchanged from the P5b handoff)

- Non-fabricating `credits_entry_for_owner` getter (with/after the concurrent miner session).
- **P5d**: full `queues_by_owner` retirement into `Factory.queue` (own 18→19 bump).
- `active_producer_by_owner` removal (its own producer-focus retirement slice).
- Ship `ProductionCategory` (D2 follow-up; own hash-key change + version bump).
- `matching_factory_count_for_owner` full-store rescan retirement (P5d).

---

## Gotchas

- **Shared checkout:** the concurrent session still owns `miner/*` + `cell_rect.rs`. P5c
  touched ONLY `src/sim/production/production_replay_tests.rs` (new) + `mod.rs` (1 line) —
  no overlap.
- **`vera20k`** is the cargo package name. Scoped P5c run: `cargo test -p vera20k --lib replay_tests`.
- The P5c tests complete builds via the NATURAL per-step cadence (no `test_arm_ready`
  shortcut — a replay can only carry commands), so the conservation test runs ~600 ticks
  to guarantee a delivery.
