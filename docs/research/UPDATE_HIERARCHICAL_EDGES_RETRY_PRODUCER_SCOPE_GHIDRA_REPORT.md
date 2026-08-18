# UpdateHierarchicalEdges Retry Producer Scope -- Ghidra Research Report

**Address(es):** `0x0042CCD0` (`PathfinderClass__UpdateHierarchicalEdges`), `0x0042CF80` (`PathfinderClass__InvalidateZoneEdge`), `0x005840C0` (`ZoneMap__FloodFillReachableZones`), `0x0042C900` (`AStar_pathfind_search`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** whether exact retry-edge producer semantics are required in the first Foundation First implementation slice, or can be deferred while implementing hierarchy data, `Zone_precheck`, and marker handoff.
**Non-Scope:** full `Zone_precheck` consumer algorithm, full `AStar_main_loop` marker/cost semantics, exact stock-map route capture, and full repeated bridge collapse/repair lifecycle.
**Confidence:** High for the scope decision; High for producer call/append behavior; Medium for how often each producer branch is hit in normal play because that needs runtime tracing.
**Active in YR:** Yes. `AStar_pathfind_search @ 0x0042C900` reaches `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0` after a failed hierarchical `AStar_main_loop`, and this path is the normal foot pathfinding retry path.

## 0. Investigation Contract

Target question: Are exact `PathfinderClass__UpdateHierarchicalEdges` / `ZoneMap__FloodFillReachableZones` retry-edge producer semantics required in the first Foundation First implementation slice, or can they be deferred while adding hierarchy data, `Zone_precheck`, and marker handoff?

Non-goals: Do not re-investigate the full `Zone_precheck` consumer, full cell A* path ordering, full bridge collapse zone rebuild, or stock-map Carville route outcome.

Evidence needed to mark COMPLETE:

- Verify the live caller ordering from failed `AStar_main_loop` into `UpdateHierarchicalEdges`.
- Verify what edge(s) the producer appends and where they are stored.
- Verify whether producer input depends on `Zone_precheck` stored path/marker arrays.
- Verify whether synthetic first-slice tests can use manual/preseeded edge exclusions without claiming retry parity.
- Verify current Rust retry shape enough to name the deferred delta.

Stop conditions:

- Stop once producer-vs-consumer boundary is clear enough to answer the implementation-scope question.
- Defer runtime incidence and stock route winner because those require debugger/path logging.
- Do not write Rust, INI, or in-repo docs.

## 1. Overview

`UpdateHierarchicalEdges` is not needed to build or consume the first binary-style `Zone_precheck` path. It only runs after a hierarchy-assisted cell A* attempt has already failed. Its job is to append Pathfinder-local per-level edge exclusions, then the caller reruns `Zone_precheck`.

Therefore the first Foundation First slice can implement hierarchy records, ordered edge metadata, `Zone_precheck` search, edge-exclusion consumption, and marker/path handoff using manual or preseeded exclusions in tests. Exact automatic retry-edge production can be deferred, but the first slice must not claim retry parity after failed cell A*, bridge-collapse detour retry parity, or stock-map route parity.

## 2. Class Layout / Key Offsets

| Owner | Offset | Type / shape | Purpose | Active in YR |
|---|---:|---|---|---|
| `PathfinderClass` | `+0x38` | byte | hierarchy-valid flag; cleared when no path edge can be invalidated | Yes; written by `0x0042CF94..0x0042CFA4`, read by caller at `0x0042CC85..0x0042CC93` |
| `PathfinderClass` | `+0x70` | cell coordinate | current/start cell used by `UpdateHierarchicalEdges` to find per-level current zones | Yes; read by `0x0042CCD8..0x0042CCE6` |
| `PathfinderClass` | `+0x74 + level*0x18` | vector object | per-level retry-local exclusion vector metadata | Yes; append target at `0x0042D069` and direct append path around `0x0042CE13..0x0042CE4E` |
| `PathfinderClass` | `+0x78/+0x84 + level*0x18` | data pointer/count | packed edge exclusions consumed later by `Zone_precheck` | Yes; produced here and consumed by `Zone_precheck` edge-skip scan |
| `PathfinderClass` | `+0xBC + level*1000` | `u16[500]` style path storage | stored chosen zone chain written by prior `Zone_precheck` | Yes; searched by `InvalidateZoneEdge @ 0x0042CFBA..0x0042CFE3` |
| `PathfinderClass` | `+0xC74 + level*4` | int | chosen path length per hierarchy level | Yes; read by `0x0042CF8D` |
| global zone map | `DAT_0087F858` | per-cell level zone ids | current zone lookup for levels `0..2` | Yes; read by `0x0042CCE6..0x0042CCFC` |
| global hierarchy graph | `DAT_0087F878 + level*0x18` | zone records | read-only source for common-neighbor exclusions | Yes; read by `0x0042D082..0x0042D13C` |

## 3. Core Logic

### 3.1 Retry producer runs only after failed hierarchical cell A*

Active in YR: Yes. Evidence: `AStar_pathfind_search @ 0x0042C900` calls `AStar_main_loop`; if result is zero and hierarchy remains enabled, the retry branch calls `0x0042CCD0` at `0x0042CC79`, then `PathfinderClass__Reset @ 0x0042A5B0` at `0x0042CC80`, then reads `+0x38` before attempting another `Zone_precheck`.

This is after, not before, `Zone_precheck` and cell A*. The first precheck/marker handoff does not require producer output unless the implementation is also claiming automatic retry after failed cell A*.

### 3.2 `UpdateHierarchicalEdges` loops exactly three hierarchy levels

Active in YR: Yes. Evidence: decompile `0x0042CCD0`; assembly loop increments the level register at `0x0042CE83`, advances the vector slot by `0x18` at `0x0042CE84`, compares with `3` at `0x0042CE87`, and jumps back while less-than at `0x0042CE93`.

For each level, it reads the current zone from `DAT_0087F858 + cell_index*10 + level*2`, calls `ZoneMap__FloodFillReachableZones`, then either appends temp-vector zones as exclusions or calls `InvalidateZoneEdge`.

### 3.3 Zero-return flood-fill branch appends all returned neighbor zones as exclusions

Active in YR: Yes. Evidence: `ZoneMap__FloodFillReachableZones @ 0x005840C0` is called at `0x0042CD80`; if the returned byte is zero, `UpdateHierarchicalEdges` iterates the temp vector backward. For each `neighbor_zone != current_zone`, it sorts the pair and appends `min << 16 | max` into the level's Pathfinder-local exclusion vector. Assembly shows sort/pack around `0x0042CE02..0x0042CE11` and append write at `0x0042CE4E`.

This branch does not depend on the stored `Zone_precheck` path array. It depends on the current cell zone, local flood-fill result, and temp vector. Runtime frequency of this branch in common maps is not proven by static analysis.

### 3.4 Nonzero flood-fill branch uses stored `Zone_precheck` path arrays

Active in YR: Yes. Evidence: nonzero return from `0x005840C0` reaches call `0x0042CDAC -> 0x0042CF80`. `InvalidateZoneEdge` reads path length at `+0xC74 + level*4` (`0x0042CF8D`), searches the stored path at `+0xBC + level*1000` (`0x0042CFBA..0x0042CFE3`), and clears `+0x38` if path length is less than two or the current zone is absent (`0x0042CF94..0x0042CFA4`, `0x0042CFE0..0x0042CFF0`).

If the zone is found, it invalidates one adjacent edge in the stored path: previous edge when the zone is the last path element, otherwise next edge. It appends the packed sorted edge via `FUN_0042D830` at `0x0042D064..0x0042D06D`.

This means exact producer parity does depend on the stored path arrays created by the previous `Zone_precheck`, but only for retry-edge production. It is not a prerequisite for implementing the first `Zone_precheck` consumer and marker handoff.

### 3.5 `InvalidateZoneEdge` also appends common-neighbor exclusions

Active in YR: Yes. Evidence: after direct path-edge append, `InvalidateZoneEdge` reads both endpoint zone records from `DAT_0087F878 + level*0x18`, scans the first endpoint's adjacency list and the second endpoint's adjacency list, and when a common neighbor is found it appends a sorted exclusion between the broken endpoint and that common neighbor. Assembly append path writes to the level exclusion vector at `0x0042D0FA..0x0042D13C`.

This second-order exclusion logic is one reason exact producer semantics should be a separate follow-up. A first-slice manual-exclusion test can prove consumer behavior, but it does not prove which exclusions gamemd would generate after a failed cell search.

### 3.6 Exclusion vectors are Pathfinder-local and consumed by `Zone_precheck`

Active in YR: Yes. Evidence: producer writes to `PathfinderClass + 0x74/+0x78/+0x84 + level*0x18`; no writes to global `DAT_0087F878` or `DAT_0087F858` were observed in `0x0042CCD0`/`0x0042CF80`. `AStar_pathfind_search` clears these vectors once at search entry, not on retry reset. `Zone_precheck` consumes them as edge-skip vectors during its next run.

This supports implementing a `ZonePrecheckInput { excluded_edges_by_level }`-style surface before implementing the binary producer.

## 4. INI Keys

No INI key is read directly by `0x0042CCD0`, `0x0042CF80`, or the scoped append helper.

| Key / data | Role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `MovementZone=` / `TechnoTypeClass+0x5B4` | Mover row used by `ZoneMap__FloodFillReachableZones` through the mover object | `0x005840C0` decompile calls mover type reader and reads `+0x5B4` | Yes |
| `g_PassabilityMatrix` | Local flood fill rejects neighbor cells unless matrix value is `1` | `0x005840C0` decompile checks `matrix[movementZone * 8 + cell_zone_type] != 1` | Yes |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Failed hierarchical A* retry | `AStar_pathfind_search` calls `UpdateHierarchicalEdges`, then `Reset`, then may rerun `Zone_precheck` | `0x0042CC79`, `0x0042CC80`, `0x0042CCB3` | Yes |
| Producer direct fallback | `UpdateHierarchicalEdges` calls `FloodFillReachableZones` and branches on its return | `0x0042CD80..0x0042CD87` | Yes |
| Producer path-array branch | Nonzero flood-fill calls `InvalidateZoneEdge` | `0x0042CDAC` | Yes |
| Consumer connection | `Zone_precheck` skips candidate graph edges matching packed pairs in the current level exclusion vector | prior verified consumer `0x0042C618..0x0042C664`; producer vector offsets match | Yes |
| Current Rust approximation | Rust already has `ZoneEdge` exclusions but derives them from failed corridor windows | `src/sim/pathfinding/zone_search.rs` `find_path_zoned_marker`, `exclude_corridor_edges` | N/A Rust status |

## 6. Current Rust Implementation Status

| Rust surface | Current status vs this producer slice |
|---|---|
| `src/sim/pathfinding/zone_search.rs` header | Correctly labels current behavior as an approximation with Dijkstra corridor, corridor A*, and per-edge exclusions. |
| `ZoneEdge` | Canonical undirected pair shape is already compatible with the consumer/producers' sorted pair concept. |
| `find_path_zoned_marker` retry loop | Retries up to five total attempts, but exclusion source is failed Rust corridor windows, not `UpdateHierarchicalEdges` / flood-fill / stored-path invalidation. |
| `find_zone_corridor` | Consumes exclusions in graph search, but still single-level centroid/corridor approximation. |
| `exclude_corridor_edges` | Suitable as an approximation only. It excludes every edge in the failed Rust corridor, while gamemd may append flood-fill returned edges, one stored-path adjacent edge, and common-neighbor edges. |
| future Foundation First surface | Needs explicit per-level preseeded/manual exclusion input to test `Zone_precheck` consumer behavior before exact producer parity exists. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AStar_pathfind_search` retry call order | verified | decompile `0x0042C900`; assembly `0x0042CC79`, `0x0042CC80` | none for scope decision |
| `UpdateHierarchicalEdges` level loop | verified | decompile `0x0042CCD0`; assembly `0x0042CE83..0x0042CE93` | none |
| zero-return flood-fill append branch | verified | decompile `0x0042CCD0`; assembly `0x0042CE02..0x0042CE4E` | runtime frequency |
| nonzero flood-fill branch to `InvalidateZoneEdge` | verified | `0x0042CDAC`; decompile `0x0042CF80` | runtime frequency |
| stored path dependency | verified | `0x0042CF8D`, `0x0042CFBA..0x0042CFE3` | none for scope decision |
| common-neighbor exclusion append | verified | `0x0042D082..0x0042D13C` | exact route impact on stock maps |
| `FloodFillReachableZones` internals | touched-not-exhausted | decompile `0x005840C0` | full cell-neighbor edge cases out-of-scope |
| `Zone_precheck` consumer | touched-not-exhausted | prior consumer reports and offset match | full algorithm out-of-scope |
| Rust `zone_search.rs` | verified scan | file read; current `ZoneEdge`, retry loop, `exclude_corridor_edges` | implementation not changed |
| stock bridge-collapse route | deferred | no runtime route trace in this slice | use runtime logger/debugger |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] mode -- Is this an exhaustive slice or coverage map? -> Exhaustive slice for producer-scope decision, not full pathfinding.` (evidence: user-scoped target)
- `[RESOLVED] entry-ccd0 -- Is `UpdateHierarchicalEdges` live in YR? -> Yes, direct call after failed hierarchical A* in `AStar_pathfind_search`.` (evidence: `0x0042CC79`; Active in YR: Yes)
- `[RESOLVED] entry-cf80 -- Is `InvalidateZoneEdge` live in YR? -> Yes, called by `UpdateHierarchicalEdges` on nonzero flood-fill result.` (evidence: `0x0042CDAC`; Active in YR: Yes)
- `[RESOLVED] first-slice-need -- Is producer output required before the first `Zone_precheck`/marker handoff can run? -> No; producer runs only after failed cell A* and before retry precheck.` (evidence: `0x0042C900` retry ordering; Active in YR: Yes)
- `[RESOLVED] manual-exclusions -- Can synthetic first-slice tests use manual exclusions? -> Yes, if they are explicitly consumer tests and do not claim producer parity.` (evidence: producer writes Pathfinder-local exclusion vectors consumed by later `Zone_precheck`)
- `[RESOLVED] path-array-dependency -- Does exact producer depend on prior `Zone_precheck` output? -> Yes in the nonzero flood-fill branch via stored per-level path arrays.` (evidence: `0x0042CF8D`, `0x0042CFBA..0x0042CFE3`; Active in YR: Yes)
- `[RESOLVED] edge-count -- Does producer append only one possible edge? -> No; zero-return branch can append all flood-fill returned neighbor zones, and invalidation branch appends one path edge plus common-neighbor edges.` (evidence: `0x0042CE02..0x0042CE4E`, `0x0042D064..0x0042D13C`; Active in YR: Yes)
- `[RESOLVED] global-mutation -- Does producer mutate global zone graphs? -> No observed writes; it appends Pathfinder-local exclusions.` (evidence: decompile `0x0042CCD0`, `0x0042CF80`; Active in YR: Yes)
- `[RESOLVED] rust-edge-shape -- Does Rust still use whole-zone exclusions? -> No, current Rust has canonical `ZoneEdge`, but the producer source is still approximate.` (evidence: `src/sim/pathfinding/zone_search.rs`; N/A Rust status)
- `[RESOLVED] acceptance-boundary -- What must first-slice tests not claim? -> Automatic failed-A* retry edge selection, flood-fill branch behavior, stock-map route parity, or exact bridge-collapse detour parity.` (evidence: unresolved runtime incidence and producer deferral)
- `[DEFERRED] flood-fill-frequency -- How often does each producer branch trigger in common YR play?` (category: needs-runtime-debugger; reason: static evidence proves behavior but not incidence; next-step-if-pursued: instrument failed hierarchical A* attempts)
- `[DEFERRED] full-flood-fill -- Full `ZoneMap__FloodFillReachableZones` neighbor and edge cases.` (category: out-of-scope; reason: only producer scope decision was needed; next-step-if-pursued: dedicated exhaustive slice for `0x005840C0`)
- `[DEFERRED] astar-marker-details -- Full `AStar_main_loop` marker/cost parity.` (category: out-of-scope; reason: separate swarm slot; next-step-if-pursued: marker gate investigation)
- `[DEFERRED] stock-route -- Exact Carville or other stock-map post-collapse route.` (category: needs-runtime-debugger; reason: requires runtime zone/path logging; next-step-if-pursued: run stock fixture trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Exact retry-edge production runs after failed hierarchical cell A*, not before initial `Zone_precheck`; it can be deferred from the first Foundation First slice. | `0x0042CC79 -> 0x0042CCD0 -> 0x0042A5B0 -> 0x0042CCB3`; Active in YR: Yes. | current Rust retry source is approximate, but first precheck foundation does not need exact producer yet | future `src/sim/pathfinding/zone_search.rs` precheck adapter and retry state | Implement precheck consumer with explicit per-level exclusion input; tests may seed exclusions manually. | `zone_precheck_manual_exclusion_skips_only_matching_edge` verifies consumer skip behavior without invoking failed-A* retry production. | Do not label manual-exclusion tests as `UpdateHierarchicalEdges` parity. |
| Exact producer depends on previous `Zone_precheck` stored path arrays in the nonzero flood-fill branch. | `0x0042CF8D`, `0x0042CFBA..0x0042CFE3`; Active in YR: Yes. | missing: Rust has no binary stored path arrays/markers yet | hierarchy/precheck result data model; later retry producer | Store chosen per-level path/markers in a shape that a later producer can inspect, even if automatic producer is deferred. | `zone_precheck_result_retains_per_level_path_for_later_retry_update` verifies selected level paths are retained. | Do not throw away path chain data after building only an allowed-zone set. |
| Producer can append multiple exclusions, not just the failed corridor edge: flood-fill returned neighbor zones, one stored-path adjacent edge, and common-neighbor edges. | `0x0042CE02..0x0042CE4E`, `0x0042D064..0x0042D13C`; Active in YR: Yes. | mismatch: Rust `exclude_corridor_edges` excludes every edge in the approximated corridor | later exact retry producer in `zone_search.rs` or a dedicated producer module | Defer automatic retry parity until `FloodFillReachableZones` and path-edge/common-neighbor exclusion rules are implemented. | `update_hierarchical_edges_appends_path_edge_and_common_neighbor_edges` should be a later test, not a Foundation First gate. | Do not claim bridge-collapse retry-route parity from a precheck-only patch. |

## 10. Negative Facts / Do Not Do

- Do not block Foundation First hierarchy/`Zone_precheck`/marker handoff on exact `UpdateHierarchicalEdges` producer semantics. Active in YR: Yes; evidence producer is post-failed-A* retry path at `0x0042CC79`.
- Do not claim automatic retry parity if tests use manual/preseeded exclusions. Active in YR: Yes; evidence exact producer appends branch-dependent exclusions at `0x0042CE4E`, `0x0042D06D`, and `0x0042D13C`.
- Do not reduce producer semantics to "exclude the failed corridor edge." Active in YR: Yes; evidence it can append all returned neighbor zones or path edge plus common-neighbor edges.
- Do not discard `Zone_precheck` selected path arrays in the first implementation if retry producer parity is planned later. Active in YR: Yes; `InvalidateZoneEdge` reads `+0xBC/+0xC74`.
- Do not represent `UpdateHierarchicalEdges` as a persistent zone-graph rebuild. Active in YR: Yes; producer writes Pathfinder-local vectors, not global graph data.

## 11. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_ASTAR_PRECHECK_RETRY_INTEGRATION_GHIDRA_REPORT.md`
  - Add: "Exact `UpdateHierarchicalEdges` producer parity can be deferred from the first hierarchy/precheck/marker-handoff implementation if tests use manual per-level edge exclusions and avoid claims about automatic failed-A* retry edge selection."
- `C:/Users/enok/Documents/ra2-rust-game-docs/PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`
  - Add: "For implementation staging, this producer is required for retry parity but not for initial `Zone_precheck` consumer parity. However, the first precheck result should preserve per-level selected path arrays because `InvalidateZoneEdge` later reads them."

## Sources

- Ghidra decompiled/read this slot: `0x0042C900`, `0x0042CCD0`, `0x0042CF80`, `0x005840C0`, `0x0042D170`, `0x0042D830`.
- Ghidra assembly contexts: `0x0042CC79`, `0x0042CD80`, `0x0042CDAC`, `0x0042CE02..0x0042CE4E`, `0x0042CE83..0x0042CE93`, `0x0042D064..0x0042D06D`, `0x0042D0FA..0x0042D13C`.
- Prior docs referenced: `BRIDGE_ASTAR_PRECHECK_RETRY_INTEGRATION_GHIDRA_REPORT.md`, `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`.
- Rust scanned read-only: `src/sim/pathfinding/zone_search.rs`.

Status: COMPLETE.
