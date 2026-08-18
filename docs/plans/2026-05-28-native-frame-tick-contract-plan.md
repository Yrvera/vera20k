# Native Frame / Tick Contract — Implementation Plan

**Status: COMPLETE (2026-05-28).** This plan was written retroactively to
document the implementation of Approach A from
`2026-05-28-native-frame-tick-contract-design.md`. All tasks below were
executed and verified; each records the exact edit made and its verification.

## Source of truth

- Design: `2026-05-28-native-frame-tick-contract-design.md` (Approach A).
- Research (verified GREEN this session): `FRAME_COUNTER_PREINCREMENT_VS_RUST_BINARY_FRAME_GHIDRA_REPORT.md`.
- Contract: `binary_frame` is committed LATE in `advance_tick` so consumers see
  the pre-increment frame `N` during the tick, mirroring `Main_Tick`'s guarded
  `g_CurrentFrameCounter` increment after `Network_ServiceLoop`.

## Task 1 — Document the contract on the `binary_frame` field — DONE

**File:** `src/sim/world/mod.rs` (field decl, ~line 278).

Replaced the field doc-comment to state the late-commit / pre-increment-visible
contract and the stored-start CDTimer read pattern (capture `binary_frame`,
later compute `binary_frame.saturating_sub(start)`; never read as the next
frame).

**Verify:** `cargo check --lib` (passed, exit 0).

## Task 2 — Remove the clock advance from the top of `advance_tick` — DONE

**File:** `src/sim/world/mod.rs`, top of `advance_tick` (~line 1223).

Deleted the early `total_sim_ms += tick_ms; binary_frame = ((total_sim_ms * 15)
/ 1000)` block. Kept `let execute_tick = self.tick.saturating_add(1);` in place
(command scheduling below filters on it). Left a comment pointing to the late
commit.

```rust
// The synthetic 15 Hz binary-frame counter is committed LATE (end of
// this fn, beside self.tick) so consumers see the pre-increment frame
// during the tick. execute_tick stays here: command scheduling below
// filters on it.
let execute_tick = self.tick.saturating_add(1);
```

## Task 3 — Add the late commit beside `self.tick = execute_tick` — DONE

**File:** `src/sim/world/mod.rs`, end of `advance_tick` (~line 1882).

Inserted the relocated clock advance immediately before `self.tick =
execute_tick;` (and therefore before `state_hash()`):

```rust
// Native frame / tick contract: commit the synthetic 15 Hz frame LATE,
// after all phase work — mirrors Main_Tick's guarded g_CurrentFrameCounter
// increment after Network_ServiceLoop. During the tick, binary_frame held
// the previous tick's committed value (the pre-increment frame N), so
// stored-start CDTimer consumers captured N, not N+1. Drift-free: every
// binary-frame boundary is exactly when total_sim_ms crosses a multiple
// of 1000/15 ≈ 66.67ms.
self.total_sim_ms = self.total_sim_ms.saturating_add(tick_ms as u64);
self.binary_frame = ((self.total_sim_ms * 15) / 1000) as u32;
self.tick = execute_tick;
let state_hash = self.state_hash();
```

**Effect:** during a tick, `binary_frame` holds the value committed at the end
of the previous tick (= pre-increment `N`). Post-tick value is unchanged
(`= f(K·dt)`), so existing derivation tests and external/render readers are
unaffected. Only *captured* start-frames shift on boundary ticks (the parity
correction).

## Task 4 — FacingClass acceptance tests — DONE

**File:** `src/sim/movement/facing_class.rs` (test module, additive).

Added two pure-unit tests:

- `set_and_check_same_frame_yields_zero_elapsed` — acceptance: timer start +
  check in the same update → `elapsed == 0` (animated sits at start, still
  rotating). Uses `set(12800, 100)` (12800/1280 = exactly 10 frames) then
  `current(100)`/`is_rotating(100)`.
- `retarget_captures_supplied_frame_and_progresses_relative_to_it` —
  acceptance: facing-retarget boundary. `set(12800, 100)` then check
  `current(105) == 6400`, retarget `set(0, 105)` captures `start_frame == 105`
  and `prev == 6400`, animating relative to the supplied frame.

**Verify:** both passed (`cargo test --lib set_and_check_same_frame`,
`cargo test --lib retarget_captures_supplied`).

## Task 5 — Discriminating late-commit test via `advance_tick` — DONE

**File:** `src/sim/world/world_tests.rs` (additive: `gate_test_rules()` helper +
test).

Added `binary_frame_committed_late_gate_captures_pre_increment_frame`: spawns a
`Gate=yes` building, seeds its `building_gate` runtime into
`BuildingGateMissionState::Setup` / `BuildingGatePhase::ClosedStable`, then runs
one 67 ms `advance_tick` (crosses the 0→1 binary-frame boundary). Asserts:

- `sim.binary_frame == 1` post-tick (committed late), and
- the gate's `transition_last_frame == 0` (the Phase-1 consumer captured the
  pre-increment frame, not the post-increment 1).

This is the modulo-gate-boundary acceptance proof via a real consumer: it fails
under top-commit (would capture 1) and passes under late-commit. Helper
`gate_test_rules()` mirrors the inline-INI pattern of `combat_test_rules()`.

**Verify:** passed under late-commit; confirmed FAIL under a temporary
top-commit revert (the discriminating check).

## Task 6 — Regression + causation verification — DONE

- Targeted modules all green: `binary_frame` (3), `facing` (95 incl. 2 new),
  `gate` (59), `turret` (15), `miner` (137).
- Full lib suite with the change: 3262 passed, 10 failed (movement ×4, ai ×1,
  ore ×1, production ×4).
- **Causation toggle:** reverted only the `mod.rs` change to top-commit and
  re-ran the full suite → 3261 passed, 11 failed = the same 10 pre-existing
  failures PLUS the new gate test (which correctly fails under top-commit). The
  10 pre-existing failures are byte-identical with and without the change ⇒
  zero regressions. They are the movement/ore/production artifacts the user
  flagged as pre-existing ("leave them").

## Out of scope (named DRIFT)

- 45 Hz (`sim.tick`) vs 15 Hz (`binary_frame`) rate mismatch — separate
  wall-clock-pace roadmap concern.
- No production absolute-frame modulo gate added (bridge-shroud `% 0x78` is
  unimplemented; contract makes it correct-by-default when built).
- No reclassification of `sim.tick`-gated systems (e.g. `sim.tick % 90`
  production retry) — flagged in the design's classification, not converted.

## Determinism

Single commit point; `binary_frame` derived from the deterministic
`total_sim_ms` accumulator; no unordered collections in the path; both
`binary_frame` and `total_sim_ms` remain in the state hash. Lockstep-safe.
