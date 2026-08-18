# Bridge Hierarchy Progress Cell Handoff Trace

**Scenario:** marker-gated A* starts in level-0 zone `1` with selected precheck path `[1, 2, 3]`, accepts a neighbor in zone `2`, then later fails. The failed retry source should be the tracked progress cell in zone `2`, not the start cell, last expanded node, or destination-adjacent cell.

**Trace date:** 2026-05-24  
**Scope:** one mechanic only: failed hierarchy retry source cell handoff.  
**Rust files read:** `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core_tests.rs`.  
**Gamemd evidence:** `PATHFINDER_FAILED_ASTAR_CURRENT_ZONE_SOURCE_GHIDRA_REPORT.md`, `ASTAR_RETRY_RESET_EXCLUSION_LIFETIME_GHIDRA_REPORT.md`, `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`.  
**Active in standard YR:** Yes, per the cited reports: `FootClass__Run_AStar -> AStar_pathfind_search -> AStar_main_loop -> UpdateHierarchicalEdges` is the normal live foot pathfinding spine, with no TS-only gate found in this slice.

## Pipeline

`zone_precheck_flat` selects `[1,2,3]` -> marker-gated cell A* receives the selected level-0 path -> progress tracker initializes to start -> accepted neighbor entering zone `2` advances progress -> A* later fails -> gamemd calls `UpdateHierarchicalEdges` using `Pathfinder+0x70`; Rust currently returns `None` before exposing a failed-attempt progress cell to a retry producer.

## Stage Verdicts

| Stage | Boundary value compared | gamemd output | Rust output | Verdict |
|---|---|---|---|---|
| 1. Selected level-0 path handoff | watched level-0 path | `[1, 2, 3]` stored in Pathfinder selected path | `zone_search.rs:295-304` passes `&result.paths[0]`; for this scenario that is `[1, 2, 3]` | PASS |
| 2. Progress initialization | retry-source/progress cell before expansion | `Pathfinder+0x6C = 0`; `Pathfinder+0x70 = start` | `HierarchyProgressTracker::new` sets `progress_index = 0`, `progress_cell = start` at `core.rs:257-262` | PASS |
| 3. Accepted zone-2 neighbor update point | update happens after candidate acceptance, not at raw neighbor probe | `AStar_main_loop` writes `+0x70` only after candidate acceptance/node creation, when neighbor zone equals next stored path zone | `core.rs:1071-1087` updates g/from, pushes the node, then calls `maybe_advance` | PASS |
| 4. Zone-2 progress value | progress after accepting neighbor `N2` in zone `2` | `+0x6C = 1`; `+0x70 = N2` | with `level0_path[1] == 2`, `maybe_advance(2, N2)` sets `progress_index = 1`, `progress_cell = N2` at `core.rs:265-270` | PASS |
| 5. Later failure before zone-3 acceptance | retry-source/progress cell at failed A* return | remains `N2`; rejected candidates and non-next-zone cells do not rewrite it | tracker would remain `N2` if no accepted zone `3` neighbor occurs; the exact failing fixture was not executed in this trace to avoid writes outside the report | UNCHECKED |
| 6. Failed retry source handoff | cell consumed by retry edge update | `UpdateHierarchicalEdges` reads `Pathfinder+0x70 = N2` and derives current zones for levels `0..2` | `find_path_with_costs_hierarchy_marker_progress` uses `astar_search(...)?` at `core.rs:2096-2119`; on failed A* it returns `None` before returning `progress_cell`, and `zone_search.rs:295-312` maps only successful results to a path. No failed-attempt retry producer consumes `N2`. | NOT-IMPLEMENTED |

## Player-Visible Findings

### NOT-IMPLEMENTED: failed hierarchy retry source is not handed off

When the first marker-gated cell A* reaches zone `2` and then fails, gamemd retries from the progressed current zone derived from the accepted zone-2 cell. Current Rust can track that cell inside A*, but the failed return path discards the tracker by propagating `None` from `astar_search`. `zone_search.rs` receives no `HierarchyMarkerPathResult` on failure and has no `UpdateHierarchicalEdges`-style retry producer.

Player-visible effect: in bridge hierarchy cases where the first selected `[1,2,3]` corridor reaches zone `2` but dead-ends, gamemd can invalidate the zone-2 edge context and retry. Rust can stop at the first failed marker-gated A* and report no path, so units may fail to route around a bridge/corridor blockage that retail can recover from.

Rust evidence:

- `core.rs:2096-2119`: `astar_search(...)?` exits before constructing `HierarchyMarkerPathResult` on failure.
- `core.rs:2120-2124`: `progress_cell` is returned only on success.
- `zone_search.rs:295-312`: hierarchy marker search maps successful result to `result.path`; no failed progress-cell branch.

Gamemd evidence:

- `PATHFINDER_FAILED_ASTAR_CURRENT_ZONE_SOURCE_GHIDRA_REPORT.md` sections 3.1-3.4: `Pathfinder+0x70` initializes to start, advances on accepted next-zone neighbor, and is read by `UpdateHierarchicalEdges`.
- `ASTAR_RETRY_RESET_EXCLUSION_LIFETIME_GHIDRA_REPORT.md` section 3: failed hierarchical A* order is `UpdateHierarchicalEdges -> Reset -> reread +0x38 -> Zone_precheck`.
- `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md` section 3: `UpdateHierarchicalEdges` derives current zones from `Pathfinder+0x70` for levels `0..2`.

## Existing Test Coverage

The new local tests cover:

- successful progress across next zones: `astar_hierarchy_progress_tracks_last_accepted_next_path_zone` at `core_tests.rs:2372-2400`;
- no-progress failure remains start: `astar_hierarchy_progress_remains_start_when_no_next_zone_accepted` at `core_tests.rs:2404-2432`.

They do not cover this exact scenario: progress reaches zone `2`, then A* fails, and the failed retry producer receives the zone-2 progress cell.

## Adjacent Findings

- The current public hierarchy branch is still a single attempt, not the full gamemd failed-A* retry lifecycle with persistent per-search edge exclusions.
- The success test advances through zone `4`, not a failed `[1,2,3]` handoff case; this is a coverage gap, not a separate mechanic traced here.

## Verdict Tally

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Status

COMPLETE
