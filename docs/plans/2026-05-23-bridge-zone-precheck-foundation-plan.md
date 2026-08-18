# Bridge Zone Precheck Foundation - Implementation Plan

> **For Codex:** Execute this plan task-by-task. Keep patches small. Do not claim high-bridge route parity, automatic retry parity, sloped route parity, or stock Carville route parity from this plan.

**Goal:** Implement the first binary-style `Zone_precheck` foundation for flat/no-slope synthetic pathing: hierarchy data, ordered edge metadata, precheck search, selected path/marker output, manual exclusions, and A* level-0 marker gating with the blocker-neighbor exception.

**Design Doc:** [docs/plans/2026-05-23-bridge-zone-precheck-foundation-design.md](2026-05-23-bridge-zone-precheck-foundation-design.md)

---

## Grounding Summary

- `BRIDGE_ZONE_PRECHECK_HIERARCHY_WRITER_ORDER_GHIDRA_REPORT.md`: hierarchy is built/searched `2 -> 1 -> 0`; zone records need parent/type fields; final adjacency order is writer/insertion order; bridge/tube edges append after scanline edges and are zero-flagged.
- `BRIDGE_ZONE_EDGE_FLAG_WRITER_SEMANTICS_GHIDRA_REPORT.md`: final edge `byte(edge+4) != 0` adds `0.001`; this is not a bridge-edge flag.
- `ZONE_ESTIMATE_SLOPE_COST_PARITY_GHIDRA_REPORT.md`: slope cost is live, but level-0/flat/no-mover synthetic tests can defer it.
- `ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`: cell A* reads level-0 marker state and allows off-marker cells when `CellClass+0x122 != 0`.
- `UPDATE_HIERARCHICAL_EDGES_RETRY_PRODUCER_SCOPE_GHIDRA_REPORT.md`: exact retry producer can be deferred, but `ZonePrecheckResult` must retain per-level selected paths for later producer parity.
- `LAYERED_BRIDGE_PATH_ENTRY_FOUNDATION_SCOPE_GHIDRA_REPORT.md`: flat foundation may go first only as a scoped data-model/synthetic-test step; layered high-bridge route parity remains deferred.

## Non-Goals

- Do not implement exact `UpdateHierarchicalEdges` / `FloodFillReachableZones` producer semantics.
- Do not implement exact `Zone_Estimate_Slope_Cost` or `Foot+0x21C` slope context.
- Do not wire layered high-bridge pathing to the new hierarchy path yet.
- Do not assert exact Carville route, route direction, zone ids, or final path cells.
- Do not replace low-bridge records with high-bridge redirect semantics.
- Do not remove the legacy corridor approximation for all callers until the hierarchy gate has the `+0x122` exception surface.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/pathfinding/zone_hierarchy.rs` | Add binary-style hierarchy graph/precheck types and synthetic constructors/tests |
| Modify | `src/sim/pathfinding/zone_map.rs` | Store optional per-`MovementZone` hierarchy; expose hierarchy accessors |
| Modify | `src/sim/pathfinding/zone_search.rs` | Add flat/no-slope `zone_precheck` search and bridge to flat path wrapper when safe |
| Modify | `src/sim/pathfinding/core.rs` | Add hierarchy marker gate option with blocker-neighbor exception |
| Modify | `src/sim/pathfinding/zone_search_tests.rs` | Precheck search, path/marker, manual exclusion tests |
| Modify | `src/sim/pathfinding/core_tests.rs` | A* marker gate and blocker-neighbor exception tests |
| Read/check | `src/sim/pathfinding/zone_build.rs` | Preserve bridge zero-flag facts and avoid sorted final parity graph assumptions |
| Read/check | `src/sim/pathfinding/zone_map_tests.rs` | Keep high-vs-low bridge redirect distinction tests passing |

## Interface Decisions

- Keep current `ZoneMap` / `ZoneAdjacency` compatibility path intact.
- Add a new hierarchy/precheck surface rather than mutating `find_zone_corridor` into a partial clone.
- Represent precheck costs in integer milli-units:
  - base `1.0` => `1000`
  - base `0.0` => `0`
  - edge flag `0.001` => `1`
- Precheck output must store both:
  - per-level selected paths;
  - per-level marked sets.
- Manual exclusions are per-level undirected edge keys.
- A* marker gate must be explicit search input, not terrain/passability state.
- Blocker-neighbor refcount equivalent should be testable with synthetic data before production wiring.
- `Zone_precheck` candidate acceptance must include the movement-zone passability matrix check. Type `1` only bypasses the lower-level parent gate; it must not bypass passability, exclusions, or normal candidate validity.
- Production marker-gated A* must not treat a missing blocker-neighbor count surface as "all cells have count 0". If counts are unavailable outside synthetic tests, keep the compatibility path until the count surface is wired.
- When a hierarchy is present and the caller elects to use it, hierarchy precheck is authoritative for same-zone/cross-zone failure handling. The old reduced `SuperZoneMap` reachability check must not abort first.

## Sim Checklist

- [x] Stay inside `sim/pathfinding`.
- [x] No render/UI/sidebar/audio/net dependency.
- [x] No floating point in sim precheck cost logic.
- [x] Stable deterministic tie order via insertion sequence, not `ZoneId`.
- [x] No persistent world state required for synthetic first slice.
- [x] Explicitly separate compatibility corridor path from parity hierarchy path.

## Risks

- **Over-pruning A*:** strict chosen-zone-only marker gating is wrong. Include blocker-neighbor exception.
- **Overclaiming parity:** flat foundation is not high-bridge player-route parity until layered marker/retry integration exists.
- **Wrong retry semantics:** manual exclusion tests prove consumer behavior only, not automatic failed-A* producer behavior.
- **Wrong edge flags:** bridge edges are zero-flagged; do not add `0.001` to bridge adjacency just because it is a bridge.
- **Sorting drift:** final parity graph edge order must be insertion order, not `BTreeSet`/`sort_unstable`.

---

## Tasks

### Task 1: Add hierarchy data types

**Why:** Current `zone_hierarchy.rs` is only union-find reachability. The parity precheck needs three-level graph records.

**Files:**
- Modify: `src/sim/pathfinding/zone_hierarchy.rs`

**Steps:**

1. Keep existing `SuperZoneMap` behavior intact.
2. Add new hierarchy types:
   - `ZoneHierarchy`
   - `ZoneLevelGraph`
   - `ZoneRecord`
   - `ZoneEdgeRecord`
   - `ZoneEdgeKey`
   - `ZonePrecheckExclusions`
   - `ZonePrecheckResult`
   - `ZonePrecheckOutcome`
3. Use `ZoneId` from `zone_map.rs`.
4. Keep zone `0` as sentinel.
5. Add synthetic test-only constructors or small helpers for explicit graph fixtures.
6. Document that this is gamemd hierarchy data, while `SuperZoneMap` remains a reachability cache.

**Acceptance:**

- Existing `zone_hierarchy` tests still pass.
- New types compile without changing existing runtime callers.

**Run:**

```powershell
cargo test -q zone_hierarchy --lib
```

### Task 2: Add flat/no-slope precheck search over synthetic hierarchy

**Why:** Prove binary-style `Zone_precheck` search behavior before wiring pathfinding.

**Files:**
- Modify: `src/sim/pathfinding/zone_hierarchy.rs`
- Modify: `src/sim/pathfinding/zone_search_tests.rs` or add tests under `zone_hierarchy.rs`

**Steps:**

1. Add `zone_precheck_flat` or equivalent function.
2. Search levels in order `2 -> 1 -> 0`.
3. For same start/goal zone at a level, write a one-zone path and mark it.
4. For different zones, run Dijkstra-like accumulated-cost search.
5. Before accepting a neighbor candidate, verify that the active `MovementZone` can enter the candidate zone type through the same passability-matrix semantics used by gamemd's zone precheck.
6. Use integer milli-costs:
   - `ZONE_BASE_COSTS = [1000, 0, 0, 1000, 1000, 0, 1000, 1000]`
   - flagged edge adds `1`
   - slope contribution `0` in this slice.
7. Treat invalid/out-of-range zone types as non-candidates. Do not index base-cost or passability tables blindly and do not default unknown types to passable.
8. Preserve strict replacement: update only on `new_cost < old_cost`.
9. Preserve insertion-order ties with an explicit sequence counter.
10. Never use `ZoneId` as a tie-breaker.
11. Return `ZonePrecheckResult` with `[Vec<ZoneId>; 3]` and marked sets.

**Tests:**

- `zone_precheck_searches_levels_2_1_0`
- `zone_precheck_equal_cost_keeps_edge_insertion_order`
- `zone_precheck_edge_flag_adds_tiny_tiebreak_cost`
- `zone_precheck_result_retains_per_level_path_for_later_retry_update`
- `zone_precheck_rejects_zone_type_blocked_by_movement_zone_matrix`
- `zone_precheck_rejects_invalid_zone_type`

**Run:**

```powershell
cargo test -q zone_precheck --lib
```

### Task 3: Implement parent-gate and type-1 exception

**Why:** The key difference from a one-level corridor is lower-level pruning by the chosen coarser path.

**Files:**
- Modify: `src/sim/pathfinding/zone_hierarchy.rs`
- Modify: tests in same module or `zone_search_tests.rs`

**Steps:**

1. On levels `1` and `0`, candidate neighbor is accepted only if:
   - neighbor parent is marked in the next coarser level; or
   - neighbor `zone_type == 1`.
2. Level `2` bypasses parent gate.
3. Make gate order match the verified ledger: cost replacement, parent gate, passability/type handling, then exclusions where practical.
4. Keep type-1 bypass narrow: it bypasses parent gate, not passability, exclusions, or normal candidate validity.

**Tests:**

- `zone_precheck_parent_gate_prunes_off_corridor_child_edges`
- `zone_precheck_parent_gate_allows_type_1_exception`
- `zone_precheck_type_1_exception_still_obeys_passability_matrix`

**Run:**

```powershell
cargo test -q zone_precheck --lib
```

### Task 4: Add manual/preseeded exclusion consumption

**Why:** First slice can defer exact retry producer, but consumer exclusion behavior must be correct.

**Files:**
- Modify: `src/sim/pathfinding/zone_hierarchy.rs`
- Modify: tests

**Steps:**

1. Add per-level `ZonePrecheckExclusions`.
2. Store edge keys as canonical undirected `(min, max)` pairs for retry/precheck consumer semantics.
3. During neighbor scan, skip only the matching edge at the current level.
4. Do not ban either endpoint zone.
5. Keep exclusions as input to precheck. Do not mutate the graph.

**Tests:**

- `zone_precheck_manual_exclusion_skips_only_matching_edge`
- `zone_precheck_manual_exclusion_does_not_ban_endpoint_zone`

**Do Not Claim:**

- automatic failed-A* retry producer parity.

**Run:**

```powershell
cargo test -q zone_precheck --lib
```

### Task 5: Add hierarchy access to `ZoneGrid` without changing runtime behavior

**Why:** The new hierarchy must be attachable to existing pathfinding infrastructure while leaving compatibility paths untouched.

**Files:**
- Modify: `src/sim/pathfinding/zone_map.rs`

**Steps:**

1. Add optional hierarchy storage:
   - `hierarchy: BTreeMap<MovementZone, ZoneHierarchy>`
2. Add accessor:
   - `hierarchy_for(&self, mz: MovementZone) -> Option<&ZoneHierarchy>`
3. Keep `build` / `build_with_terrain` initially inserting no production hierarchy if the builder is not ready.
4. Add a crate-private constructor or test helper only if needed to assemble `ZoneGrid` with synthetic hierarchy.
5. Ensure current `can_reach`, `map_for`, `adjacency_for`, and super-zone behavior is unchanged.
6. If any mutable zone-map/adjacency update API can run while hierarchy data exists, clear or invalidate the hierarchy for that `MovementZone`. A stale hierarchy must never survive an incremental pathing or bridge update.

**Tests:**

- Existing `zone_map` and `zone_search` tests must pass unchanged.

**Run:**

```powershell
cargo test -q zone_map --lib
cargo test -q zone_search --lib
```

### Task 6: Add A* hierarchy marker gate option

**Why:** `core.rs` currently filters by allowed corridor zones. Binary A* uses level-0 markers plus the blocker-neighbor exception.

**Files:**
- Modify: `src/sim/pathfinding/core.rs`
- Modify: `src/sim/pathfinding/core_tests.rs`

**Steps:**

1. Add `HierarchyGate` or equivalent to `AStarOptions`.
2. Include:
   - level-0 zone lookup data;
   - marked level-0 zone set;
   - optional blocker-neighbor count grid.
3. Add a small `BlockerNeighborCounts` type or pass a simple cell-indexed `u8` slice with width/height metadata.
4. In normal candidate expansion, before terrain cost / virtual enter checks, apply:
   - if candidate zone is marked: allow;
   - else if blocker-neighbor count is nonzero: allow;
   - else skip.
5. Synthetic tests may pass an explicit count grid with all zeros. Production marker-gated calls must provide a real blocker-neighbor count surface; otherwise keep the compatibility corridor path.
6. Do not enable the production hierarchy-gated path for explicit-tube scenarios in this slice unless the direction-8 marker-gate behavior is verified. Keep existing explicit-tube behavior under the compatibility path and document the tube marker-gate question as deferred.
7. Keep existing `corridor` option intact for compatibility.

**Tests:**

- `astar_hierarchy_rejects_unmarked_one_ring_zone_without_blocker_exception`
- `astar_hierarchy_allows_off_marker_cell_with_blocker_neighbor_count`
- Existing corridor tests still pass.

**Run:**

```powershell
cargo test -q astar_hierarchy --lib
cargo test -q core --lib
```

### Task 7: Wire flat pathing through precheck foundation behind a narrow path

**Why:** The foundation must exercise the new precheck/marker handoff in flat pathing without disrupting layered bridge pathing.

**Files:**
- Modify: `src/sim/pathfinding/zone_search.rs`
- Modify: tests

**Steps:**

1. In `find_path_zoned_marker`, check for an eligible `zg.hierarchy_for(mz)` before the old reduced `zg.can_reach` cross-zone abort.
2. If hierarchy exists:
   - compute start/goal level zones from hierarchy;
   - run `zone_precheck_flat`.
3. Preserve failure behavior:
   - if start/goal are the same zone in the hierarchy source used by `zone_precheck_flat` and precheck fails, clear hierarchy and run ordinary A*;
   - if cross-zone hierarchy precheck fails, return `None`.
4. On success, call `find_path_with_costs_marker` or an equivalent A* entry with `HierarchyGate`, not `expand_corridor`.
5. Only enter the production `HierarchyGate` path when the blocker-neighbor count surface is available and the request is not an explicit-tube scenario whose direction-8 marker-gate behavior remains deferred. If either condition is not met, keep legacy `find_zone_corridor + expand_corridor` behavior rather than silently over-pruning open off-marker cells or inventing tube gating.
6. Keep legacy `find_zone_corridor + expand_corridor` behavior when no hierarchy exists.
7. Run the old reduced `zg.can_reach` / `SuperZoneMap` precheck only on the compatibility path, not before an eligible hierarchy precheck.
8. Do not change `find_layered_path_zoned_marker` in this task except comments documenting deferred parity.

**Tests:**

- `zone_precheck_same_zone_failure_clears_hierarchy_before_astar`
- `zone_precheck_flat_foundation_rejects_off_parent_child_path`
- `zone_precheck_hierarchy_path_bypasses_reduced_superzone_abort`
- Existing `zoned_path_*` tests pass through compatibility path.

**Run:**

```powershell
cargo test -q zone_precheck --lib
cargo test -q zone_search --lib
```

### Task 8: Preserve bridge record distinctions and zero-flag bridge edges

**Why:** The hierarchy must not regress previously fixed high-vs-low bridge record semantics.

**Files:**
- Modify/read: `src/sim/pathfinding/zone_build.rs`
- Modify/read: `src/sim/pathfinding/zone_map_tests.rs`
- Modify tests if needed

**Steps:**

1. Keep current high-only bridge redirect behavior.
2. Keep low records available to all-active adjacency where already verified.
3. In hierarchy edge fixtures/tests, represent bridge/tube edges with `flag = 0`.
4. Add a synthetic hierarchy test for bridge zero-flag behavior.
5. Do not implement full production hierarchy bridge injection unless the previous tasks are stable.

**Tests:**

- `zone_precheck_bridge_edges_are_zero_flagged`
- `zone_get_high_bridge_redirect_ignores_low_bridge_records`

**Run:**

```powershell
cargo test -q bridge_redirect --lib
cargo test -q zone_precheck --lib
```

### Task 9: Add explicit non-claim guardrails

**Why:** This patch is easy to overstate. The code should make the remaining work visible.

**Files:**
- Modify: `src/sim/pathfinding/zone_search.rs`
- Modify: `src/sim/pathfinding/zone_hierarchy.rs`
- Possibly modify: `src/sim/pathfinding/zone_search_tests.rs`

**Steps:**

1. Add comments near `find_layered_path_zoned_marker`:
   - layered marker/retry integration remains deferred;
   - flat hierarchy foundation does not prove high-bridge route parity.
2. Add comments near manual exclusion handling:
   - producer parity deferred;
   - exclusions are preseeded/manual in current tests.
3. Add comments near slope:
   - flat/no-mover tests use zero slope; exact sloped route parity deferred.
4. Add a test or doc comment ensuring stock Carville route is not asserted by this patch.

**Run:**

```powershell
cargo test -q zone_precheck --lib
cargo check -q
```

### Task 10: Full focused verification

**Why:** Shared pathfinding changes need more than one test target.

**Run:**

```powershell
cargo test -q zone_precheck --lib
cargo test -q zone_hierarchy --lib
cargo test -q zone_search --lib
cargo test -q zone_map --lib
cargo test -q core --lib
cargo check -q
```

If `core --lib` is too broad/noisy, run the named new tests plus the existing pathfinding test groups and report any unrelated failures.

## Acceptance Criteria

- New hierarchy/precheck types exist without breaking existing zone reachability.
- Synthetic flat/no-slope precheck searches levels `2 -> 1 -> 0`.
- Candidate acceptance includes movement-zone passability matrix behavior; type `1` bypasses only the parent gate.
- Invalid/out-of-range zone types fail closed as non-candidates.
- Parent gating and type-1 exception are tested.
- Ordered edge records and edge flag cost are tested.
- Bridge/tube edge flag zero behavior is tested in hierarchy fixtures.
- Manual exclusions skip only matching edges and do not ban endpoint zones.
- `ZonePrecheckResult` retains per-level selected paths and marker sets.
- A* marker gate rejects unmarked one-ring zones unless blocker-neighbor count is nonzero.
- Production marker-gated A* is not enabled without a real blocker-neighbor count surface.
- Production hierarchy-gated A* is not enabled for explicit-tube direction-8 scenarios until that marker-gate behavior is verified.
- Same-zone precheck failure uses the hierarchy zone source to clear hierarchy and still run A*; cross-zone failure aborts.
- Eligible hierarchy precheck is not preempted by the old reduced `SuperZoneMap` reachability abort.
- Optional hierarchy data cannot remain stale across mutable zone-map/adjacency updates.
- Existing compatibility path remains available when no hierarchy is present.
- Layered bridge pathing remains explicitly deferred and unclaimed.

## Follow-Up Queue After This Plan

1. Implement exact `UpdateHierarchicalEdges` / `FloodFillReachableZones` retry producer.
2. Build production hierarchy from real `ResolvedTerrainGrid` and bridge records using gamemd writer order.
3. Wire layered high-bridge pathing to the hierarchy marker gate.
4. Implement exact `Zone_Estimate_Slope_Cost` context if a sloped route mismatch is targeted.
5. Run `gamemd_carville_low_bridge_post_collapse_route_trace` and only then add stock route oracle tests.
