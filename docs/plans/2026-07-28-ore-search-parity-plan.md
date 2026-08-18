# Ore Search Parity Implementation Plan

> **For Claude:** Execute task-by-task.

**Goal:** Match native Mission_Harvest state-0 search behavior: no whole-map fallback beyond the bounded TiberiumLongScan ring scan, and a miner already standing on its found ore cell enters harvesting the same dispatch.

**Architecture:** Confined to `handle_search_ore` in `src/sim/miner/miner_system.rs` (delete the `pick_best_resource_node` fallback; add the own-cell direct-to-Harvest entry) plus tests. No new state, no RNG, no snapshot bump. `pick_best_resource_node` stays in `production/` (its unit tests remain; other future consumers possible).

**Design Doc:** none (approved inline 2026-07-28: backlog item 1, "ok do ore search parity").

## Grounding Summary

- `docs/research/miner/TIBERIUM_SEARCH_FAMILY_GHIDRA_REPORT.md` (2026-07-28, spot-checked this session): `0x004dd0a0` scans square/Chebyshev rings 1..radius-1 (radius-EXCLUSIVE) after an unfiltered own-cell fast path; gamemd has **no fallback** beyond that — search miss goes straight to the no-ore paths. `0x004dcfe0`'s boolean return = "already standing on the found cell, no move needed"; state 0 enters state 1 the same dispatch either way (driving happens inside state 1's NavCom branch).
- `docs/scans/trace-swarm-20260728/mission-harvest-cadence.md` §3 state 0: found → status 1, return 1.
- Current Rust: `handle_search_ore` (miner_system.rs:462-551) — the fallback block at :532-541 is the deletion target; the own-cell case currently detours through `MoveToOre` (one extra per-frame dispatch) whose arrival branch arms `harvest_timer` with `harvest_tick_interval + 1` (miner_system.rs:634-641).
- Only production consumer of `pick_best_resource_node` is the deleted block; direct unit tests of the helper (miner_tests.rs:2529, :2563) keep it referenced.
- Rust `search_local_ore` ring loop is `for ring in 1..radius_i` — already radius-exclusive, matching native; ring-0 fast path already unfiltered. No changes there (comment fix only: "diamond" → Chebyshev square, per the report).

## Key Technical Decisions

- Delete the global fallback outright (no config gate): **high** — native has none (report §, spot-checked epilogue); parity bar says match gamemd.
- Own-cell target → `Harvest` directly, arming `harvest_timer(interval+1)` exactly like the MoveToOre arrival branch: **high** — native state 0→1 same dispatch; the +1 mirrors the verified mission-before-timer observation already encoded at the arrival site.
- Apply the own-cell entry to BOTH the archive-consumption path and the scan path: **high** — native's archive drive and scan share state-1 entry.

## Open Questions

- Deferred: whether the global determinism harness's harvester found its ore via the fallback (beyond 48 cells). If so, the `miner_engaged` assert and pins shift — re-baseline with documentation (Task 3). Detectable only by running.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/miner/miner_system.rs` | handle_search_ore changes + search_local_ore doc comment fix |
| Modify | `src/sim/miner/miner_tests.rs` | 2 new tests; adjust fallout |
| Maybe | `src/sim/world/global_parity_harness_tests.rs` | re-baseline if the scenario used the fallback |

## Sim Checklist

- [x] No floats; no new hashed state; no RNG; no new deps; no tick-order change.

## Risk Areas / Player-Experience Items

| Class | Item | Verification |
|---|---|---|
| MILESTONE | Miner must idle (WaitNoOre) when ore exists only beyond the scan ring — retail behavior | `miner_no_fallback_beyond_scan_radius_matches_gamemd` |
| COMPOUNDING | Same-dispatch harvest entry on own-cell ore (1-frame delta per acquisition) | `miner_standing_on_ore_harvests_same_dispatch` |
| RESIDUAL | Radius-exclusive ring already matches; only the "diamond" comment is wrong | comment fix, no behavior |

## Tasks

### Task 1: handle_search_ore changes
- Delete the `pick_best_resource_node` block (:532-541) and its `use` (:31); the no-ore WaitNoOre arm follows the scan miss directly. Update the preceding comment to cite the no-fallback contract.
- In both the archive path (:495-503) and scan-hit path (:519-530): when the chosen cell == `(snap.rx, snap.ry)`, set `target_ore_cell`, `snap.state = MinerState::Harvest`, and arm `snap.miner.harvest_timer.arm(sim.session.binary_frame, u32::from(config.harvest_tick_interval) + 1)` (mirror of the MoveToOre arrival branch, same +1 rationale comment); else `MoveToOre` as today.
- Fix `search_local_ore`'s doc comment: ring is square/Chebyshev (report), not "diamond"; note radius-exclusive.
- Verify: `cargo check -p vera20k --lib`.

### Task 2: Tests
```rust
/// gamemd has no whole-map fallback: ore beyond the TiberiumLongScan ring
/// (radius-EXCLUSIVE, Chebyshev) leaves the miner idle in WaitNoOre.
#[test]
fn miner_no_fallback_beyond_scan_radius_matches_gamemd() {
    // miner at (5,5); ore at Chebyshev distance exactly long_scan_radius
    // (48) → NOT found (exclusive bound) → WaitNoOre.
    // config.long_scan_radius = 48 default; place ore at (53, 5+48=53)? use (5+48, 5).
}

/// Native state 0 enters harvesting the same dispatch when the miner already
/// stands on the found ore cell — no MoveToOre detour.
#[test]
fn miner_standing_on_ore_harvests_same_dispatch() {
    // ore at the miner's own cell; one tick → state == Harvest and
    // harvest_timer armed (not due for interval+1 frames).
}
```
Concrete bodies use the existing `spawn_miner`/`ResourceNode` insertion idioms (see `miner_tests` ore-seeding tests, e.g. :2526 area) and `tick_miners_n`. Verify both PASS.

### Task 3: Full suite + harness
- `cargo test -p vera20k --lib`; fix fallout: tests that placed ore beyond 48 Chebyshev of the miner now need nearer ore or a WaitNoOre expectation (prefer the fixture change unless the test's purpose was the fallback itself — those get rewritten to the native contract, citing the report).
- If the global harness pins shift (fallback was load-bearing in that scenario): re-baseline all moved constants with a documented reason referencing this plan; record record/replay equality still holding.
- Record the literal `test result:` line.

### Task 4: Commit + live verify
- Commit: `miner: ore-search parity (no whole-map fallback; same-dispatch own-cell harvest entry)`.
- Rebuild the bin, run `RA2_QUICKPLAY=minerloop.map`, confirm the full loop in logs/ra2.log (miner 1), kill the instance.

## Sources & References

- docs/research/miner/TIBERIUM_SEARCH_FAMILY_GHIDRA_REPORT.md; docs/scans/trace-swarm-20260728/mission-harvest-cadence.md §3
- Addresses: 0x004dd0a0 (scan), 0x004dcfe0 (wrapper), shared false epilogue 0x004dd08c — here, not in code comments.
- Code: src/sim/miner/miner_system.rs:31,462-551,634-641; src/sim/production (pick_best_resource_node stays).
- Prior commits: f57be00f, 8c74f28f (same branch).
