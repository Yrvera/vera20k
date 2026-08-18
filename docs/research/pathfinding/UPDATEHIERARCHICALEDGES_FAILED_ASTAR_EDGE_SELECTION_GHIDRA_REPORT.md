# UpdateHierarchicalEdges Failed-A* Edge Selection -- Ghidra Research Report

**Address(es):** `0x0042C900` (`AStar_pathfind_search`), `0x0042CCD0` (`PathfinderClass__UpdateHierarchicalEdges`), `0x0042CF80` (`PathfinderClass__InvalidateZoneEdge`), `0x005840C0` (`ZoneMap__FloodFillReachableZones`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Which retry-local hierarchy edge exclusions are produced after a marker-gated `AStar_main_loop` failure, which inputs the producer reads, and whether Rust can implement the producer from `ZonePrecheckResult.paths` alone or needs A* failure/frontier context.  
**Non-Scope:** temp edge bucket writer order, `CellClass+0x122` writers, layered A*, slope, stock Carville route, explicit tube direction-8 behavior, and full locomotor-specific `Can_Enter_Cell` internals.  
**Confidence:** High for call order, liveness, inputs, selected edge identity, and Rust-facing conclusion; Medium for runtime frequency of each producer branch.  
**Active in YR:** Yes. `AStar_pathfind_search @ 0x0042C900` is the normal foot pathfinding wrapper and calls `UpdateHierarchicalEdges @ 0x0042CCD0` after a failed hierarchical `AStar_main_loop`.

## 0. Investigation Contract

Target question: Which selected-path edge(s) are invalidated after marker-gated A* failure, what inputs do `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0` and `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80` read, and can Rust implement the retry producer from `ZonePrecheckResult.paths` alone or does it need A* failure/frontier context?

Non-goals: Do not investigate temp edge buckets, `CellClass+0x122` writers, layered A*, slope, stock Carville route, explicit tube direction-8 behavior, or full locomotor-specific `Can_Enter_Cell` bodies.

Evidence needed to mark COMPLETE: verify failed `AStar_main_loop` handoff into `0x0042CCD0`; verify `0x0042CCD0` branch inputs and append outputs; verify `0x0042CF80` selected-path edge identity; verify whether producer reads any A* failure/frontier data; scan current Rust surfaces enough to name the implementation delta.

Stop conditions: stop once the failed-edge producer contract and Rust-facing input requirement are proven; defer runtime branch frequency and route-oracle behavior.

## 1. Overview

After a hierarchical cell A* attempt fails, gamemd does not pass the failed frontier, failed cell, or closed-list state to the retry producer. The caller invokes `PathfinderClass__UpdateHierarchicalEdges(this, mover)` with only the pathfinder object and mover object. The producer then recomputes retry exclusions from the pathfinder's stored start/current cell, per-level cell-zone map, local flood-fill reachability, stored `Zone_precheck` path arrays, and global hierarchy adjacency.

Therefore Rust cannot implement exact retry production from `ZonePrecheckResult.paths` alone. It also does not need A* frontier context for the scoped binary behavior. It needs a `FloodFillReachableZones`-equivalent producer surface plus retained per-level paths, level graphs, per-cell level zone ids, mover cell-entry/passability inputs, and search-local exclusion vectors.

## 2. Key Offsets / Inputs

| Owner | Offset / address | Meaning | Active in YR |
|---|---:|---|---|
| `AStar_pathfind_search` | `0x0042CC79` | Calls `UpdateHierarchicalEdges` after `AStar_main_loop` returns zero while hierarchy remains enabled. | Yes; decompile plus assembly context. |
| `PathfinderClass` | `+0x38` | Hierarchy-valid flag; `InvalidateZoneEdge` clears it when no actionable edge exists; caller rereads it after update/reset. | Yes; `0x0042CF94..0x0042CFA4`, `0x0042CC85..0x0042CC93`. |
| `PathfinderClass` | `+0x70` | Stored pathfinder cell used by `UpdateHierarchicalEdges` to find the current zone for every level. | Yes; read at `0x0042CCD8..0x0042CCE6`. |
| `PathfinderClass` | `+0x74 + level*0x18` | Per-level search-local edge-exclusion vector object. | Yes; append target in both producer branches. |
| `PathfinderClass` | `+0x78/+0x84 + level*0x18` | Exclusion data pointer/count consumed by later `Zone_precheck`. | Yes; producer writes these vector slots; consumer covered by prior reports. |
| `PathfinderClass` | `+0xBC + level*1000` | Stored selected `Zone_precheck` path for a level. | Yes; searched by `0x0042CFBA..0x0042CFE3`. |
| `PathfinderClass` | `+0xC74 + level*4` | Stored path length for a level. | Yes; read at `0x0042CF8D`. |
| `DAT_0087F858` | global per-cell level-zone ids | `UpdateHierarchicalEdges` reads the current cell's zone id for levels `0..2`. | Yes; `0x0042CCE6..0x0042CCFC`, `0x0042CD94..0x0042CDA7`. |
| `DAT_0087F878 + level*0x18` | global hierarchy graph | `InvalidateZoneEdge` reads endpoint adjacency lists for common-neighbor exclusions. | Yes; `0x0042D082..0x0042D13C`. |
| `0x005840C0` inputs | current cell, level, temp vector, mover | Local split detector / neighbor-zone collector used before choosing producer branch. | Yes; call at `0x0042CD80`. |

## 3. Core Logic

### 3.1 Failed A* handoff carries no frontier context

Active in YR: Yes. Evidence: `AStar_pathfind_search @ 0x0042C900` calls `AStar_main_loop`; when it returns zero and hierarchy remains enabled, the retry branch calls `0x0042CCD0`, then `PathfinderClass__Reset @ 0x0042A5B0`, then rereads `Pathfinder+0x38` before the next `Zone_precheck`. Assembly context: `0x0042CC79: CALL 0x0042CCD0`, `0x0042CC80: CALL 0x0042A5B0`, `0x0042CC85: MOV DL, byte ptr [EBP + 0x38]`.

The call site pushes the mover object and uses `ECX=PathfinderClass`; it does not pass an A* failed cell, frontier, closed list, or selected path edge. This is the load-bearing answer for Rust: exact producer parity needs retry-repair context, but not A* frontier output.

### 3.2 `UpdateHierarchicalEdges` loops levels `0..2` and branches through flood-fill

Active in YR: Yes. Evidence: `0x0042CCD0` decompile plus loop assembly `0x0042CE83..0x0042CE93`.

For each level:

1. Convert `Pathfinder+0x70` to a zone-map cell index.
2. Read `current_zone = DAT_0087F858[cell_index * 10 + level * 2]`.
3. Allocate/reset a temporary `u16` vector.
4. Call `ZoneMap__FloodFillReachableZones @ 0x005840C0` with current cell, level, temp vector, and mover.
5. If flood-fill returns nonzero, call `InvalidateZoneEdge(this, current_zone, level)`.
6. If flood-fill returns zero, iterate the temp vector backward and append `current_zone` to each collected different zone as sorted undirected exclusions.

This branch choice is not derivable from `ZonePrecheckResult.paths`. It depends on a level-block local flood using mover `Can_Enter_Cell`, the movement-zone passability matrix, and per-cell level-zone ids.

### 3.3 Zero-return flood-fill branch excludes current-zone-to-collected-neighbor edges

Active in YR: Yes. Evidence: caller branch at `0x0042CD85..0x0042CD87`; sort/pack around `0x0042CE02..0x0042CE11`; append write at `0x0042CE4E`.

When `0x005840C0` returns zero, `0x0042CCD0` does not inspect the stored `Zone_precheck` path. It iterates the helper's temp vector backward. For every `neighbor_zone != current_zone`, it stores `min(current_zone, neighbor_zone) << 16 | max(current_zone, neighbor_zone)` into the current level's Pathfinder-local exclusion vector.

Material detail: this can append multiple exclusions for one level and is not "the selected path edge." It is based on the helper-collected local different zones plus graph-neighbor supplement, not on A* frontier data.

### 3.4 Nonzero flood-fill branch invalidates one adjacent stored-path edge

Active in YR: Yes. Evidence: nonzero branch call `0x0042CDAC -> 0x0042CF80`; path length read `0x0042CF8D`; path scan `0x0042CFBA..0x0042CFE3`; direct path-edge append `0x0042D064..0x0042D06D`.

`InvalidateZoneEdge` reads the selected path array for the level. If `path_len < 2`, it clears `Pathfinder+0x38` and returns. If `current_zone` is absent from the path, it also clears `+0x38` and returns.

If `current_zone` is present:

- If it is the last path element, invalidate the previous edge `(path[i-1], path[i])`.
- Otherwise invalidate the next edge `(path[i], path[i+1])`.
- Sort endpoints before appending the packed undirected edge.

So `ZonePrecheckResult.paths` is necessary for this branch, but still not sufficient: the caller must know the current zone and must first know that the flood-fill branch selected `InvalidateZoneEdge`.

### 3.5 `InvalidateZoneEdge` also appends common-neighbor exclusions

Active in YR: Yes. Evidence: graph base read `0x0042D082`; adjacency nested scan and append path through `0x0042D0FA..0x0042D13C`.

After appending the direct adjacent path edge, `InvalidateZoneEdge` reads both endpoint zone records from `DAT_0087F878 + level*0x18`. For each neighbor common to both endpoint adjacency lists, it appends a sorted exclusion between the broken endpoint and that common neighbor, excluding self-pairs.

This requires hierarchy graph adjacency. `ZonePrecheckResult.paths` alone cannot produce these second-order exclusions.

### 3.6 `FloodFillReachableZones` input dependency

Active in YR: Yes. Evidence: decompile `0x005840C0`; caller at `0x0042CD80`.

The helper computes block size `1 << (level + 1)` for live levels `0..2`, flood-fills within that block, and accepts neighbor expansion only when:

- mover `Can_Enter_Cell(neighbor, dir, neighbor+0x11B, 0, 1)` returns nonzero;
- `ZonePassabilityMatrix[mover.MovementZone][neighbor.CellClass+0x4C] == 1`;
- the neighbor's hierarchy zone id at this level matches the seed zone;
- the masked block-local visited byte is clear.

If an in-playfield same-zone cell in the block remains unreached, it returns `1`. Otherwise it appends graph neighbors not locally seen and returns `0`. These inputs are dynamic pathing/cell-entry inputs, not `ZonePrecheckResult.paths` fields.

## 4. INI Keys

No INI key is read directly by `0x0042CCD0` or `0x0042CF80`.

| Data | Role | Evidence | Active in YR |
|---|---|---|---|
| `MovementZone=` / `TechnoTypeClass+0x5B4` | Row for flood-fill passability matrix via mover type. | `0x005840C0` decompile reads mover type `+0x5B4`. | Yes. |
| `ZonePassabilityMatrix` | Local flood expands only when matrix value is `1`. | `0x0058427B..0x00584286`; prior matrix reader reports. | Yes. |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Failed hierarchical A* | Calls `UpdateHierarchicalEdges`, resets, rereads hierarchy-valid flag, may rerun `Zone_precheck`. | `0x0042CC79`, `0x0042CC80`, `0x0042CC85..0x0042CC93`. | Yes. |
| Flood-fill split detector | `UpdateHierarchicalEdges` calls `0x005840C0` before deciding zero-return append vs stored-path invalidation. | `0x0042CD80`, `0x0042CD85..0x0042CDAC`. | Yes. |
| Stored path consumer | `InvalidateZoneEdge` reads `+0xBC/+0xC74` path arrays written by prior `Zone_precheck`. | `0x0042CF8D`, `0x0042CFBA..0x0042CFE3`. | Yes. |
| Graph adjacency consumer | `InvalidateZoneEdge` reads `DAT_0087F878` for common-neighbor exclusions. | `0x0042D082..0x0042D13C`. | Yes. |
| Rust current scaffold | `ZonePrecheckResult.paths` are retained; hierarchy branch currently does one precheck + marker A* with default empty exclusions; compatibility retry excludes corridor edges. | `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_search.rs`. | N/A Rust status. |

## 6. Current Rust Implementation Status

| Rust surface | Current status vs this slice |
|---|---|
| `src/sim/pathfinding/zone_hierarchy.rs` | `ZonePrecheckResult.paths` and per-level `ZonePrecheckExclusions` exist, which is necessary for the `InvalidateZoneEdge` branch and consumer tests. |
| `src/sim/pathfinding/zone_search.rs` hierarchy branch | Runs `zone_precheck_flat` with empty exclusions and then `find_path_with_costs_hierarchy_marker`; no verified failed-A* retry producer is wired. |
| `src/sim/pathfinding/zone_search.rs` compatibility branch | Uses five corridor attempts and `exclude_corridor_edges`, but this is not binary-shaped: it excludes every edge in the Rust corridor rather than using flood-fill split detection and path-edge/common-neighbor rules. |
| Needed Rust surface | A retry producer that receives current/start cell, mover/movement context, level graphs and cell-zone ids, dynamic cell-entry/passability inputs, retained precheck paths, and mutable per-search exclusions. It does not need A* frontier/closed-list output for this scoped binary behavior. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AStar_pathfind_search` failed-A* handoff | verified | decompile `0x0042C900`; assembly `0x0042CC79..0x0042CC93` | none |
| `UpdateHierarchicalEdges` level loop | verified | decompile `0x0042CCD0`; assembly `0x0042CE83..0x0042CE93` | none |
| zero-return append branch | verified | decompile `0x0042CCD0`; assembly `0x0042CE02..0x0042CE4E` | runtime frequency |
| nonzero branch to `InvalidateZoneEdge` | verified | `0x0042CDAC`; decompile `0x0042CF80` | runtime frequency |
| selected stored-path edge identity | verified | `0x0042CF8D`, `0x0042CFBA..0x0042D06D` | none |
| common-neighbor exclusions | verified | `0x0042D082..0x0042D13C` | route impact on stock maps |
| A* frontier/failure context use | verified absent for this call boundary | `0x0042CC79` call context plus `0x0042CCD0` inputs | none for scoped producer |
| `FloodFillReachableZones` internals | verified for producer inputs/result | decompile `0x005840C0`; prior flood-fill report | full locomotor virtual targets deferred |
| Current Rust surfaces | verified read-only | `rg`/file reads of `zone_hierarchy.rs`, `zone_search.rs`, `core.rs` | no implementation performed |

## 8. Open Questions -- Final State

- `[RESOLVED] mode -- Is this exhaustive-slice or coverage-map? -> Exhaustive-slice for the failed-A* retry producer input/edge-selection contract.` (evidence: bounded user target)
- `[RESOLVED] live-caller -- Is `UpdateHierarchicalEdges` active in standard YR? -> Yes, live failed hierarchical A* retry path calls it.` (evidence: `0x0042CC79`; Active in YR: Yes)
- `[RESOLVED] frontier -- Does the producer read A* frontier/failure-cell context? -> No evidence of any such input; call passes Pathfinder and mover only, and producer reads Pathfinder stored cell/zone state.` (evidence: `0x0042CC79` context, `0x0042CCD0` decompile; Active in YR: Yes)
- `[RESOLVED] paths-alone -- Can `ZonePrecheckResult.paths` alone implement exact producer? -> No; zero-return branch does not use paths, and nonzero branch also needs current zone plus graph adjacency.` (evidence: `0x0042CE02..0x0042CE4E`, `0x0042CFBA..0x0042D13C`; Active in YR: Yes)
- `[RESOLVED] selected-edge -- Which stored-path edge is invalidated? -> Previous edge if current zone is last path element; otherwise next edge.` (evidence: `0x0042CFBA..0x0042D06D`; Active in YR: Yes)
- `[RESOLVED] disable -- What if there is no path edge? -> Clear `Pathfinder+0x38`; caller uses that to disable hierarchy.` (evidence: `0x0042CF94..0x0042CFA4`, `0x0042CFE0..0x0042CFF0`, `0x0042CC85`; Active in YR: Yes)
- `[RESOLVED] multi-exclusion -- Can one failed attempt add multiple exclusions? -> Yes; zero-return branch can append multiple collected current-zone edges, and invalidation branch appends direct plus common-neighbor exclusions.` (evidence: `0x0042CE4E`, `0x0042D06D`, `0x0042D13C`; Active in YR: Yes)
- `[RESOLVED] mutation -- Does producer mutate global hierarchy graph? -> No observed writes; it appends Pathfinder-local vectors and reads global graph/cell-zone data.` (evidence: decompiles `0x0042CCD0`, `0x0042CF80`; Active in YR: Yes)
- `[RESOLVED] rust-delta -- Does current Rust already have enough data? -> It retains paths/exclusions but lacks the verified flood-fill producer and production context wiring.` (evidence: `zone_hierarchy.rs`, `zone_search.rs`; N/A Rust)
- `[DEFERRED] branch-frequency -- How often zero vs nonzero branch occurs in common maps.` (category: needs-runtime-debugger; reason: static evidence proves behavior, not incidence; next-step-if-pursued: instrument failed hierarchy retries)
- `[DEFERRED] full-can-enter -- Concrete locomotor `Can_Enter_Cell` behavior for every mover.` (category: out-of-scope; reason: this slice only needs zero/nonzero gate; next-step-if-pursued: locomotor cell-entry audit)
- `[DEFERRED] route-oracle -- Stock Carville/bridge exact route after retry.` (category: needs-runtime-debugger; reason: requires runtime path capture; next-step-if-pursued: trace stock map path logs)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Failed-A* retry producer does not consume A* frontier/failure-cell context; it recomputes from Pathfinder stored cell, level zone ids, mover context, flood-fill, retained paths, and graph adjacency. | `0x0042CC79..0x0042CC93`, `0x0042CCD0`, `0x005840C0`; Active in YR: Yes. | Rust need not expose A* frontier for this producer, but currently lacks a binary-shaped producer context. | `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_hierarchy.rs`, future retry producer helper. | Add a producer API fed by current/start cell, hierarchy level graphs/cell-zone ids, mover passability/cell-entry inputs, previous `ZonePrecheckResult`, and per-search exclusions. | A failed marker-gated A* can call producer without an A* frontier object and still add the same exclusions from deterministic context. Proposed test name: `update_hierarchical_edges_does_not_require_astar_frontier_context`. | Do not add a fake frontier dependency or derive exclusions from the failed Rust corridor. |
| `ZonePrecheckResult.paths` alone are insufficient: zero-return flood-fill branch ignores paths; nonzero branch also needs current zone and global graph adjacency for common-neighbor exclusions. | `0x0042CE02..0x0042CE4E`, `0x0042CFBA..0x0042D13C`; Active in YR: Yes. | Current `ZonePrecheckResult.paths` retention is necessary but not sufficient for Task 8. | `zone_hierarchy.rs::ZonePrecheckResult`, `ZonePrecheckExclusions`, producer helper. | Keep paths, but also provide level graph/cell-zone access and flood-fill results before adding production retry. | Producer fixture with identical selected path but different current-zone local flood result yields different exclusions. Proposed test name: `retry_producer_paths_alone_do_not_select_edges`. | Do not implement broad "ban selected path edges" from `paths` only. |
| Nonzero flood-fill branch invalidates one adjacent stored-path edge: previous if current zone is the last path element, otherwise next; then adds common-neighbor exclusions. | `0x0042CF8D`, `0x0042CFBA..0x0042D06D`, `0x0042D082..0x0042D13C`; Active in YR: Yes. | Missing exact branch producer. | `zone_hierarchy.rs` or dedicated producer module plus `zone_search.rs` retry loop. | Implement path-edge selection and common-neighbor append as per-level search-local undirected exclusions. | Path `[1,2,3]` with current `2` excludes `2-3`; current `3` excludes `2-3`; shared neighbor `4` adds endpoint/common-neighbor exclusion. Proposed test name: `retry_invalidate_zone_edge_selects_adjacent_path_edge_and_common_neighbors`. | Do not exclude every edge in the selected path or ban endpoint zones. |

## 10. Negative Facts / Do Not Do

- Do not implement the exact producer from `ZonePrecheckResult.paths` alone. Active in YR: Yes; evidence zero-return branch uses flood-fill vector at `0x0042CE02..0x0042CE4E`, and common-neighbor append needs graph adjacency at `0x0042D082..0x0042D13C`.
- Do not require A* frontier or closed-list output for this producer. Active in YR: Yes; evidence failed-A* handoff at `0x0042CC79` passes only Pathfinder/mover context into `0x0042CCD0`.
- Do not ban every edge in the selected path after failure. Active in YR: Yes; evidence `0x0042CF80` chooses only previous-or-next edge adjacent to current zone, then specific common-neighbor exclusions.
- Do not treat `FloodFillReachableZones` return `1` as route success. Active in YR: Yes; evidence nonzero branch calls `InvalidateZoneEdge` at `0x0042CDAC`.
- Do not rebuild or mutate the global `ZoneHierarchy`/`ZoneGrid` during retry. Active in YR: Yes; producer writes Pathfinder-local exclusion vectors and reads global zone data.

## 11. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/plans/2026-05-24-production-flat-bridge-zone-hierarchy-activation-design.md`
  - Replace: "Implement the flat retry producer in `zone_hierarchy.rs` or `zone_search.rs` using `ZonePrecheckResult.paths`."
  - With: "Implement the flat retry producer using retained `ZonePrecheckResult.paths` plus current/start cell, per-level cell-zone ids, hierarchy graph adjacency, mover passability/cell-entry inputs, and a `FloodFillReachableZones`-equivalent split detector. `paths` alone are insufficient, but no A* frontier context is required by gamemd's scoped producer."
- `C:/Users/enok/Documents/ra2-rust-game/docs/plans/2026-05-24-production-flat-bridge-zone-hierarchy-activation-plan.md`
  - Existing Task 8 gate is directionally correct. Tighten Step 5 wording to: "Use `ZonePrecheckResult.paths` only as one producer input; exact retry also requires current/start cell, per-level zone ids, hierarchy graph adjacency, mover passability/cell-entry inputs, and the `FloodFillReachableZones` split result. Do not require A* frontier context unless a future contradiction is found."

## Sources

- Ghidra decompiled/read this slot: `0x0042C900`, `0x0042CCD0`, `0x0042CF80`, `0x005840C0`.
- Ghidra assembly contexts: `0x0042CC79..0x0042CC93`, `0x0042CD80..0x0042CDAC`, `0x0042CE02..0x0042CE4E`, `0x0042CF8D`, `0x0042CFBA`, `0x0042D064..0x0042D06D`, `0x0042D082..0x0042D13C`.
- Prior docs referenced: `UPDATE_HIERARCHICAL_EDGES_RETRY_PRODUCER_SCOPE_GHIDRA_REPORT.md`, `BRIDGE_ASTAR_PRECHECK_RETRY_INTEGRATION_GHIDRA_REPORT.md`, `MAPCLASS_FLOODFILLREACHABLEZONES_005840C0_GHIDRA_REPORT.md`, `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`.
- Rust scanned read-only: `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core.rs`.

Status: COMPLETE.
