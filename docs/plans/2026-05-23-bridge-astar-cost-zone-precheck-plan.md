# Bridge A* Cost And Zone-Precheck Parity - Implementation Plan

> **For Codex:** Execute this plan task-by-task. Keep patches small and run the focused tests after each phase.

**Goal:** Fix the implementation-safe bridge A* parity mismatches: zone-precheck equal-cost tie order, bridge flank edge costs, marker/flank/epsilon ordering, direction-8 bypass guardrails, and no-reopen guardrails.

**Architecture:** Patch existing `sim/pathfinding` surfaces in place. `zone_search.rs` gets stable strict-cost queue behavior without `ZoneId` ties. `core.rs` gets bridge flank helper/tests first; live normal-edge wiring happens only if the branch-enable lifecycle and probe geometry clear the evidence gate. Tests in `zone_search_tests.rs` and `core_tests.rs` pin the verified binary details. No render/UI/audio changes.

**Design Doc:** [docs/plans/2026-05-23-bridge-astar-cost-zone-precheck-design.md](2026-05-23-bridge-astar-cost-zone-precheck-design.md)

---

## Grounding Summary

- **Synthesis:** [BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md](docs/research/BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md) marks this patch bundle `IMPLEMENTATION_SAFE` for synthetic zone/cell pathing parity. It explicitly defers exact stock Carville route assertions and `FUN_0042B080` object-scan modeling.
- **Zone-precheck evidence:** `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md` verifies adjacency scan order, strict lower-cost replacement, strict heap comparisons, no `ZoneId` tie key, and undirected edge exclusions.
- **Bridge edge-cost evidence:** `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md` verifies marker x4 placement, bridge flank multipliers `10.0 / 1.0 / 2.0`, final additive direction epsilon, and direction-8 bypass.
- **Closed-list evidence:** `ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md` verifies `1.009` is not true reopen behavior; selected-layer closed nodes are not reinserted.
- **Repo status:** `SearchMarkerOverlay`, code-2 cost, dual ground/bridge arrays, and explicit tube bypass already exist in `core.rs`. Missing or risky surfaces are zone tie ordering and bridge flank multiplier.

## Key Technical Decisions

- **Use an explicit stable queue item in `find_zone_corridor`.** Do not use tuple ordering that includes `ZoneId`. Ordering is cost first, insertion sequence second; `ZoneId` is payload only.
- **Replacement stays strict.** Update `dist` and `prev` only when `new_cost < dist[neighbor]`, never on equality.
- **Keep current centroid edge cost for now.** This plan fixes tie/order drift but does not claim full binary `Zone_precheck` cost parity. Add a comment naming the remaining approximation.
- **Do not remove the existing one-ring corridor expansion in this patch.** It is known approximation debt, but changing it together with queue behavior would widen the blast radius.
- **Bridge flank cost is a normal compass edge decoration, but live wiring is gated.** Apply it after marker x4 and before `DIR_TIEBREAK` only after the branch-enable lifecycle and flank probe geometry are verified enough for the Rust call path. Until then, land helper/tests only.
- **Direction 8 remains separate.** It must not call any helper that applies marker, flank, entity-helper costs, or normal direction epsilon.
- **No true reopen.** Do not add standard A* reopen behavior. Add guardrail tests/comments if a natural refactor touches closed handling.

## Open Questions

### Resolved During Planning

- **Should exact Carville route be asserted now?** No. Fixture is known, exact route is not.
- **Should `FUN_0042B080` be implemented now?** No. It belongs to bridge passability marker peer-object modeling, not this first A* cost/tie patch.
- **Should full three-level `Zone_precheck` be implemented now?** No. Full zone type/slope/edge-flag cost and selected-chain hierarchy remain separate work.

### Deferred To Implementation

- **Out-of-map flank lookup behavior.** Keep first tests in-bounds. Do not invent edge-of-map semantics unless a Ghidra or runtime check confirms them.

### Required Before Live Flank Wiring

- **`PathfinderClass+0x01` flank-branch lifecycle.** Before applying bridge flank multipliers to live Rust A* calls, verify when this byte is enabled for standard YR path searches. If this cannot be verified in the implementation turn, implement only the helper and tests, and leave runtime wiring as a blocked follow-up.
- **Flank probe geometry.** Before implementing the helper, transcribe or spot-check the destination-orientation table and two flank probe offsets from `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md` / Ghidra. Do not infer the flank cells from visual intuition.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/pathfinding/zone_search.rs` | Stable strict-cost queue for `find_zone_corridor`; comments for remaining cost-model approximation |
| Modify | `src/sim/pathfinding/zone_search_tests.rs` | Equal-cost adjacency-order regressions; exclusion guardrails |
| Modify | `src/sim/pathfinding/core.rs` | Bridge flank multiplier helper and normal compass edge-cost insertion point |
| Modify | `src/sim/pathfinding/core_tests.rs` | Flank multiplier tests, marker/flank/epsilon ordering, direction-8 bypass guardrails |
| Read/check only | `src/sim/pathfinding/zone_build.rs` | Confirm no new sort/dedup is introduced into final `ZoneAdjacency` |
| Read/check only | `src/sim/pathfinding/zone_map.rs` | Confirm `neighbors_of` remains insertion-order slice access |

## Interface Changes

Expected minimal interface impact:

- `find_zone_corridor` signature stays unchanged.
- `ZoneAdjacency` public shape stays unchanged.
- `SearchMarkerOverlay` stays unchanged.
- Prefer keeping any bridge flank helper private to `core.rs`.
- Do not add a runtime `AStarOptions` flank-enable switch unless `PathfinderClass+0x01` lifecycle is verified. If lifecycle is not verified, any option must remain private/test-only or the helper stays unwired.

No new persistent sim state. No state-hash changes expected.

## Sim Checklist

- [x] No render/UI/sidebar/audio/net dependencies.
- [x] No persistent state added unless a flank enable option is added as per-search scratch.
- [x] Deterministic ordering is explicit: insertion sequence is stable and local to one search.
- [x] No floating point in sim path costs. Use integer multipliers matching the existing `STEP_COST` scale.
- [x] `BTreeMap` / `BinaryHeap` tie behavior considered. Do not depend on default tuple ordering where parity cares.
- [x] Tick ordering unchanged. This is path request behavior only.

## Risk Areas

- **Zone path choice changes:** Some existing tests may assume the old `ZoneId` tie. Update only when the binary ledger proves the old expectation was wrong.
- **Corridor approximation remains:** Because this plan keeps one-ring expansion and centroid costs, do not label resulting stock detours as exact route parity.
- **Bridge flank helper geometry:** Wrong flank-coordinate mapping can make bridge diagonals too cheap or too expensive. Keep fixtures small and in-bounds.
- **Direction-8 regression:** Refactoring edge cost into a shared helper can accidentally apply marker/flank costs to tube jumps. Tests must pin bypass.
- **Closed-list temptation:** The `1.009` finding can look like reopen logic. It is not. Do not add heap reinsertion for closed selected-layer cells.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 1-2 | Equal-cost zone ties preserve adjacency order | Units can choose different bridge detours after collapse/repair if equal-cost corridor ties fall to `ZoneId` | `zone_search_tests` fixture with `[high_id, low_id]` adjacency |
| 2 | Edge exclusions skip edges, not zones | Retry behavior after a failed corridor must not over-prune valid detours | Existing and retained exclusion test |
| 4-7 | Bridge flank multiplier | Bridge deck diagonal/side choices depend on cost penalty, not hard blocking | Helper-level tests first; path-level tests only after enable lifecycle is verified |
| 5-7 | Marker/flank/epsilon ordering | Small ordering mistakes alter tie outcomes and route preference | Cost-ordering unit test; live path test only after enable lifecycle is verified |
| 7 | Direction-8 bypass | Low bridge/tube jumps must not receive normal compass edge decorations | Existing marker bypass test plus flank bypass guard |
| 7 | No true reopen | Prevents a future "better A*" refactor from diverging from YR | Test/comment guard around closed handling |

---

## Tasks

### Task 1: Add failing zone-precheck tie-order tests

**Why:** Establish the player-visible mismatch before touching search code. Current tuple heap ordering can choose lower `ZoneId` even when adjacency order says otherwise.

**Files:**
- Modify: `src/sim/pathfinding/zone_search_tests.rs`

**Steps:**

1. Add a synthetic `ZoneMap` helper where start zone `1` has two equal-cost routes to goal zone `5`.
2. Use adjacency order `1 -> [3, 2]`, where zone `3` has the higher id but is discovered first.
3. Give zones `2` and `3` identical/equal-distance centers so current cost and heuristic ties are exact.
4. Assert `find_zone_corridor(..., 1, 5, empty_exclusions)` returns `[1, 3, 5]`.
5. Add the mirrored test with adjacency `1 -> [2, 3]` and assert `[1, 2, 5]`.
6. Keep or extend the existing undirected edge exclusion tests.

**Expected before Task 2:** first test should fail under current tuple heap ordering if the equal-cost setup is correct.

**Run:**

```powershell
cargo test zone_search --lib
```

### Task 2: Replace `ZoneId` tuple tie behavior with stable strict-cost queue

**Why:** This is the direct parity fix for `Zone_precheck` insertion-order ties.

**Files:**
- Modify: `src/sim/pathfinding/zone_search.rs`

**Steps:**

1. Add a private queue item type near `find_zone_corridor`, for example:
   - `cost: i32`
   - `sequence: u32`
   - `zone: ZoneId`
2. Implement `Ord`/`PartialOrd` so the `BinaryHeap` behaves as a min-heap by:
   - lower `cost` first;
   - lower `sequence` first for equal cost;
   - no `ZoneId` comparison for tie choice.
3. Replace `BinaryHeap<Reverse<(i32, i32, ZoneId)>>` with the new queue item.
4. Keep `new_cost < dist[neighbor]`; equality must not replace `prev[neighbor]`.
5. Increment sequence only when pushing an accepted candidate.
6. Update comments: current edge cost is still center-distance approximation; tie/order behavior is now binary-shaped.

**Run:**

```powershell
cargo test zone_search --lib
cargo test zone --lib
```

**Acceptance:**

- New adjacency-order tests pass.
- Existing edge-exclusion tests still pass.
- No sorted/ZoneId tie behavior remains in `find_zone_corridor`.

### Task 3: Decide whether to remove the destination heuristic from zone corridor ordering

**Why:** Binary `Zone_precheck` is Dijkstra-like and does not use a destination heuristic, but removing Rust's heuristic is a broader behavior change than the equal-cost tie fix. Handle it as an explicit decision with its own test coverage.

**Files:**
- Modify: `src/sim/pathfinding/zone_search.rs`
- Modify: `src/sim/pathfinding/zone_search_tests.rs`

**Steps:**

1. Add a test where the current centroid `g+h` ordering and accumulated-cost-only ordering choose different first corridors.
2. If the test reflects a verified binary ledger item, remove the destination heuristic from heap ordering and keep accumulated cost only.
3. If the fixture cannot be grounded without full zone type/slope/edge-flag costs, leave the heuristic unchanged for this patch and document it as remaining approximation debt.
4. Do not combine this with any full hierarchy rewrite.

**Run:**

```powershell
cargo test zone_search --lib
```

**Acceptance:**

- Either accumulated-cost-only ordering is covered by a test and implemented, or the plan explicitly carries the current heuristic as deferred approximation debt.

### Task 4: Verify flank enable lifecycle and flank probe geometry

**Why:** Runtime bridge flank cost is active in the helper, but the branch also depends on `PathfinderClass+0x01`. Wiring it unconditionally could introduce path drift. The helper geometry also must use binary probe offsets, not guessed visual flanks.

**Files:**
- Read: `docs/research/ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`
- Optional Ghidra spot-check: `AStar_compute_edge_cost @ 0x00429830`
- Optional Ghidra spot-check: `PathfinderClass` setter/xrefs for `+0x01`

**Steps:**

1. Extract the destination-orientation table and two flank probe offset rules from the report or a fresh Ghidra spot-check.
2. Confirm which Rust `PathCell` facts correspond to destination orientation `0x800` and structural bridge bit `0x100`.
3. Verify when `PathfinderClass+0x01` is enabled for standard YR path searches.
4. If lifecycle is verified, proceed to live wiring in Task 6.
5. If lifecycle is not verified, proceed with helper/tests only and mark live wiring blocked.

**Acceptance:**

- The implementation notes identify the exact flank coordinate mapping.
- The plan has a clear yes/no decision for live runtime wiring.

### Task 5: Add bridge flank cost helper tests

**Why:** Lock down the numeric bridge edge behavior before wiring it into A*.

**Files:**
- Modify: `src/sim/pathfinding/core_tests.rs`
- Possibly expose helper as `pub(crate)` or keep tests in same module access path if available.

**Steps:**

1. Add a small in-bounds bridge-deck fixture with a candidate destination, movement direction, and two flank cells.
2. Test first flank not structural bridge returns multiplier `10`.
3. Test first flank structural and second not structural returns multiplier `1`.
4. Test both flanks structural returns multiplier `2`.
5. Test orientation selection uses destination bridge orientation, not source orientation, using the verified table from Task 4.

**Notes:**

- Use existing `PathGrid`/`PathCell` test builders where possible.
- Do not test edge-of-map flank lookup yet.
- Do not hard-block diagonals; multiplier only changes cost.

**Expected before Task 6:** helper tests fail or do not compile until helper exists.

### Task 6: Implement bridge flank multiplier helper and wire only if evidence allows

**Why:** Rust currently has marker x4 and entity/code costs but lacks the binary bridge flank multiplier. This can change bridge-deck route preference.

**Files:**
- Modify: `src/sim/pathfinding/core.rs`

**Steps:**

1. Add named constants:
   - `BRIDGE_FLANK_MISSING_MULTIPLIER: i32 = 10`
   - `BRIDGE_FLANK_ONE_MULTIPLIER: i32 = 1`
   - `BRIDGE_FLANK_BOTH_MULTIPLIER: i32 = 2`
2. Add a private helper that returns the bridge flank multiplier for a normal compass edge using the verified Task 4 probe mapping.
3. Gate helper use to normal directions `0..=7` and bridge-layer edge context.
4. If `PathfinderClass+0x01` lifecycle is verified, insert helper after `apply_search_marker_cost(...)` and before `DIR_TIEBREAK[dir_index]`.
5. If lifecycle is not verified, do not wire the helper into live A*. Leave a TODO with the blocked evidence item and keep the helper covered by unit tests.
6. Ensure direction-8 tube branch does not call this helper.

**Run:**

```powershell
cargo test bridge --lib
cargo test marker --lib
cargo test tube --lib
```

**Acceptance:**

- Flank helper tests pass.
- Existing marker overlay tests still pass.
- Existing bridge layer tests still pass.
- If live wiring is skipped, the final handoff explicitly says bridge flank runtime parity remains blocked on `PathfinderClass+0x01` lifecycle.

### Task 7: Add cost-ordering and direction-8 bypass guardrails

**Why:** The highest regression risk is a future refactor that makes costs look cleaner but changes the binary order.

**Files:**
- Modify: `src/sim/pathfinding/core_tests.rs`
- Possibly modify comments in `src/sim/pathfinding/core.rs`

**Steps:**

1. Add a direct helper-level cost-ordering test:
   - code-2 jam cost;
   - marker x4;
   - bridge flank multiplier;
   - final `DIR_TIEBREAK`;
   - assert the tie-break value is not multiplied.
2. If live flank wiring landed, extend the existing `astar_marker_overlay_does_not_apply_to_direction8_tube_edge` test or add a sibling test proving bridge flank cost also does not apply to direction 8. If wiring is blocked, add a helper-level assertion and keep the existing tube marker bypass test unchanged.
3. Add a comment near the closed-list skip in `astar_search` referencing the `1.009` finding:
   - current Rust immediate skip is not full binary fallback;
   - do not implement true closed-node reopen.
4. If practical, add a narrow unit test that a closed selected-layer cell is not reinserted by a later equal/lower route. If this requires too much fixture work, leave it as a documented follow-up in this plan and code comment.

**Run:**

```powershell
cargo test marker --lib
cargo test tube --lib
cargo test bridge --lib
```

### Task 8: Focused integration check across zone + bridge pathfinding

**Why:** Zone tie changes and bridge cost changes are individually small but both affect path choice.

**Files:**
- Modify tests only if a regression exposes an incorrect old expectation.

**Steps:**

1. Run focused suites:

```powershell
cargo test zone_search --lib
cargo test zone --lib
cargo test bridge --lib
cargo test marker --lib
cargo test tube --lib
```

2. Run broader pathfinding tests:

```powershell
cargo test pathfinding --lib
```

3. Run final check:

```powershell
cargo check
```

**Acceptance:**

- All focused tests pass.
- `cargo check` passes.
- Any changed path expectation is justified against the tiny-detail ledger, not convenience.

### Task 9: Update local docs only if implementation exposes a new verified fact

**Why:** AGENTS.md requires newly confirmed binary facts to be preserved. This patch should not need new Ghidra facts, but implementation may expose stale wording in repo docs.

**Files:**
- Optional: `docs/plans/2026-05-23-bridge-astar-cost-zone-precheck-plan.md`
- Optional: docs under `docs/research/` only if a new binary fact is confirmed, not for implementation notes.

**Steps:**

1. If code comments reveal stale wording such as "`1.009` reopen", update the relevant local doc or leave a targeted TODO.
2. Do not patch research docs for mere Rust implementation choices.
3. Do not claim exact Carville route parity.

## Final Verification Bundle

Run this before handing off the implementation:

```powershell
cargo test zone_search --lib
cargo test zone --lib
cargo test bridge --lib
cargo test marker --lib
cargo test tube --lib
cargo test pathfinding --lib
cargo check
```

## Do Not Do In This Plan

- Do not assert the exact Carville post-collapse route.
- Do not implement `FUN_0042B080`.
- Do not build the full three-level binary `Zone_precheck`.
- Do not turn bridge flank penalties into diagonal hard-blocking.
- Do not apply marker/flank/entity costs to direction-8 tube edges.
- Do not introduce standard A* closed-node reopen behavior.
- Do not add render/UI/audio dependencies.
