# P5b Authority Flip — SESSION HANDOFF (cold-start for next session)

**Date:** 2026-06-04 (end of session)
**Branch:** `factory-house-substrate-p1p2`
**Status:** ✅ **P5b implementation COMPLETE + full suite GREEN. NOTHING COMMITTED.**

---

## TL;DR — where we are

The Factory/House **authority flip (P5b)** is fully implemented and the **entire test suite
passes**. The work is **uncommitted on purpose** — last session ended at "report before
committing; commit only after I confirm." **Tomorrow's first decision: commit or keep going.**

Verify snapshot taken THIS session (re-run to confirm nothing drifted overnight from the
concurrent session):
- `cargo check -p vera20k` → clean (44 pre-existing warnings in render/app layers, **none** from P5b).
- `cargo test -p vera20k` → **`test result: ok. 3725 passed; 0 failed; 19 ignored`** (lib) + all
  integration binaries green (4/5/1/1/1/3/1 passed, 0 failed).
- `SNAPSHOT_VERSION == 18` (verified by `snapshot_version_is_18`).

---

## FIRST THING TOMORROW — the commit decision

The flip is the milestone (first hashed-state change in the program). To commit:

1. **Hunk-stage ONLY the P5b hunks.** `src/sim/world/mod.rs` is **co-edited by a concurrent
   session** — `git add -p` it; stage only: the `refresh_production_shadow` body (→reconcile),
   the deleted credits-mirror line + deleted `debug_assert_economy_shadow`, the Phase-7
   `step_all` block, and the `debug_assert_factory_invariants` repurpose.
2. **DO NOT stage the concurrent session's files** (they were already dirty when this work
   started, they are NOT ours): `src/sim/cell_rect.rs`, `src/sim/miner/miner_system.rs`,
   `src/sim/miner/miner_tests.rs`, `src/sim/miner/mod.rs`, and untracked
   `src/sim/miner/harvest_mission.rs`.
3. **`docs/` is gitignored/local-only** — no commit step for the design/plan/review/this-handoff docs.
4. Suggested commit subject (matches the P-series style):
   `sim/production: Factory/House economy substrate P5b — THE AUTHORITY FLIP (registry authoritative; per-step charge to real wallet; SNAPSHOT_VERSION 17→18; C1 fold)`

The P5b files that ARE ours to stage:
```
src/sim/economy.rs                  (serde on Economy)
src/sim/house_state.rs              (un-skip economy)
src/sim/production/factory.rs       (reconcile_from_queues, step_all, prepare_step_inputs, test helpers, reconcile front-always-active fix, remaining_balance_after→#[cfg(test)])
src/sim/production/production_queue.rs   (T8 + T10 — the bulk)
src/sim/production/production_tech.rs    (widened visibility; 3 legacy rate fns →#[cfg(test)])
src/sim/production/production_types.rs   (un-skip factory_shadow)
src/sim/snapshot.rs                 (17→18 + history comment)
src/sim/world/mod.rs                (HUNK-STAGE — co-edited)
src/sim/world/world_hash.rs         (hash fold)
src/sim/production/production_queue_tests.rs       (test fallout + no_upfront_charge_at_enqueue)
src/sim/production/production_placement_tests.rs   (2 test fallout fixes)
src/sim/world/production_shadow_tests.rs           (economy test repurpose + hang fix + 4 §D guards)
```

---

## What got DONE this session (T8 + T10 + fallout)

Design/plan/review (read these for the full rationale):
`docs/plans/2026-06-04-factory-house-substrate-p5b-{design,plan,plan-review}.md`,
`docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md`.

(M1/M2 — reconcile + serde/hash/17→18 — were already done+green when the session started.)

- **T8 (production_queue.rs):**
  - Retired the upfront `*credits_entry_for_owner(sim, owner) -= obj.cost;` in `enqueue_by_type`
    (KEPT the affordability gate — C20).
  - Rerouted `cancel_by_type_for_owner` + `cancel_last_for_owner` through a new
    `registry_cancel_active` helper → registry `cancel_one` against a `house.credits` shim.
    **Queued-tail removal → NO refund** (uncharged); **active-abandon → C8 PARTIAL refund**
    (`original_balance − balance`). Mirrors the outcome into `queues_by_owner` (QueuedRemoved →
    remove first tail idx≥1; AbandonedActive → pop front). Added `prune_empty_queues`.
  - `cancel_ready_by_type_for_owner` LEFT as-is (full refund is correct — ready buildings were
    fully charged).
- **T10 (production_queue.rs `tick_production_with_overlay_registry`):**
  - Completion now reads the **registry** `view(owner,cat).ready && object.is_some()` (not the
    frames timer). Removed `advance_queue_item`, the `progress_rate` computation, and the orphaned
    `RA2_QUEUE_FRAME_MS`.
  - **B2 ETA mirror:** `remaining_base_frames = total_base_frames * (54 − progress) / 54` each tick
    (sidebar ETA tracks the real build; the hashed field stays a derived mirror — no frozen ETA).
  - Spawn/placement geometry + `pop_completed_front` + the completion-path full-refund branches
    LEFT UNCHANGED. **Delivery re-arm is via the tick-tail reconcile** (pop advances the
    queue-of-record → reconcile SEEDs the next front), NOT an explicit `start_next_queued` call —
    this is the sanctioned "reconcile re-arms == binding start_next_queued at the hashed
    end-of-tick state."
- **Reconcile correctness refinement (factory.rs):** the FRONT of a non-empty queue is ALWAYS the
  active build (drop the `has_object`/Queued gate). Hash-neutral for real flows (the live flow's
  `refresh_queue_states` means a front is never `Queued` at reconcile time); fixes direct-insert
  tests that seed `Queued` fronts.
- **production_tech.rs:** gated the retired legacy frames-rate family `#[cfg(test)]`
  (`effective_progress_rate_ppm_for_type` / `_for_category` / `matching_factory_time_multiplier_ppm`)
  — they have no production caller post-flip; only `matching_factory_bonus_is_category_specific`
  pins them.
- **Test fallout swept:**
  - Fixed an **infinite hang**: `queue_advances_only_after_delivery` looped `step until Completed`
    on a cloned `house.economy` that the retired credits-mirror left at 0 credits → never
    completes. Fix = fund the cloned oracle (`oracle.credits = 700`).
  - Repurposed `economy_shadow_tracks_legacy_credits` → `economy_shadow_does_not_mirror_credits`.
  - Adapted ~10 frames-driven integration/cancel tests to the registry model (reconcile + new
    `FactoryRegistry::test_arm_ready` / `test_factory_mut` helpers).
  - `#[ignore]`d 2 pure frames-rate tests (`low_power_and_factory_bonus_apply_per_owner_and_category`,
    `paused_queue_category_does_not_advance_while_other_category_does`) — retired mechanism; pause +
    rate coverage moved to §D + the producer.
  - Added **5 new §D charge-flip guards** (exercise `step_all` end-to-end via `advance_tick`):
    `no_upfront_charge_at_enqueue`, `single_wallet_charged_once_no_double_debit`,
    `cancel_one_partial_refund_to_house_credits`, `stall_on_no_funds_holds`,
    `factory_flip_determinism_over_scripted_commands`.

---

## Locked decisions honored (do NOT relitigate)

- **D1** drop `next_insertion_seq` + `seq_carry` (done in M2).
- **D2** Ship category deferred — naval collapses to Vehicle.
- **C1 ordering** folded into the one 17→18 bump; `step_all` at Phase-7 head before `run_late_region`.
- `active_producer_by_owner` KEPT hashed (§3.4 override).
- **Non-fabricating `credits_entry_for_owner` getter DEFERRED** — its signature change ripples into
  `miner_dock_sequence.rs` / `slave_miner.rs`, which the **concurrent session owns**. Left untouched.
- V2 corrections intact: no ×0.9; Aircraft/Infantry binding; `set_rate` takes the TOTAL; SpecialItem
  0/-1/Item distinct; purifier = building COUNT.

---

## NEXT SLICE(S) — pick up here

1. **P5c (a.k.a. P9) — the replay/parity ACCEPTANCE GATE.** The real ratification of the flip:
   replay a recorded command stream twice AND against a pre-flip baseline for bit-identical per-tick
   `state_hash`, plus `economy_conservation_over_replay` (C15), plus pre-flip-vs-post-flip
   observable-output equivalence (the x0.9-free producer correction is the ONE intended difference).
   Reuses the existing replay harness. `factory_flip_determinism_over_scripted_commands` is the
   near-term proxy that's already in place. **This is the next focused slice.**
2. **Deferred follow-ups (not blocking the commit):**
   - Non-fabricating `credits_entry_for_owner` getter (after/with the concurrent miner session).
   - **P5d**: full `queues_by_owner` retirement into `Factory.queue` (moves `enqueue_order` storage
     into the registry, erases the registry↔mirror redundancy + the U-FACTORYCOUNT registry-key-count
     limitation; own 18→19 bump).
   - `active_producer_by_owner` removal (its own producer-focus retirement slice).
   - Ship `ProductionCategory` (D2 follow-up; own hash-key change + version bump).
   - `matching_factory_count_for_owner` full-store rescan retirement (P5d, when the registry tracks
     building counts).

---

## Gotchas to remember tomorrow

- **Shared checkout:** the concurrent session edits `miner/*` + `cell_rect.rs` + `world/mod.rs`. If
  `cargo` errors point at files we didn't touch, it's their in-progress work — don't fix/revert/stash.
- **`cargo test` is slow + can hang** if a test loops-until-production-completes on an underfunded
  house (the retired mirror left cloned oracles at 0 credits). All known cases are fixed; if a NEW
  hang appears, look for `loop { ... advance_one_step/advance_tick ... }` with a 0-credit wallet.
  Run scoped first: `cargo test -p vera20k --lib production` (fast; covers everything P5b touched).
- **`vera20k`** is the cargo package name (a wrong `-p` exits 101 without running).
