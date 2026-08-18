# Zone_precheck 0x0042C290 Hierarchy Exclusions -- Ghidra Research Report

**Address(es):** `0x0042C290` (`Zone_precheck`), caller `0x0042C900`, alternate caller `0x0042D170`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `Zone_precheck` graph-search shape, edge costs/tie behavior visible in the binary, `ZonePassabilityMatrix` use, and consumption of `PathfinderClass+0x78/+0x84` per-search edge exclusions.  
**Non-Scope:** full cell-level A*, full `UpdateHierarchicalEdges`, full global zone-graph build, and full `Zone_Estimate_Slope_Cost` internals beyond the call contract used here.  
**Confidence:** High for scoped binary behavior; Medium for Rust delta because no code was modified or tests run.  
**Active in YR:** Yes. `FootClass__Run_AStar @ 0x004CBBA0` calls `AStar_pathfind_search @ 0x0042C900`, which calls `Zone_precheck` at `0x0042CB58` and `0x0042CCB3`; `FootClass__Find_Path @ 0x004D3920` also reaches `FUN_0042D170`, which calls `Zone_precheck` at `0x0042D222`.

## 1. Overview

`Zone_precheck` is the hierarchical zone-graph precheck used before and between hierarchical-assisted cell A* attempts. It searches three stored graph levels in order `2 -> 1 -> 0`, records the chosen zone chain for each level into `PathfinderClass+0xBC + level*1000`, and returns false if any level cannot connect start and destination zones.

The function consumes per-search edge exclusions already appended by `PathfinderClass__UpdateHierarchicalEdges` / `InvalidateZoneEdge`; those exclusions are sorted packed undirected zone pairs. It does not mutate global zone graphs.

## 2. Class Layout / Key Offsets

| Owner | Offset / address | Type / shape | Purpose | Active in YR |
|---|---:|---|---|---|
| PathfinderClass | `+0x28` | epoch/stamp | Written into marker arrays instead of clearing whole arrays. | Yes; read/written throughout `0x0042C300..0x0042C8ED`. |
| PathfinderClass | `+0x40/+0x44/+0x48` | `int*` | per-level chosen-path marker arrays for levels `0/1/2`. | Yes; current level `+0x40+level*4`, parent gate `+0x44+level*4`. |
| PathfinderClass | `+0x4C/+0x50/+0x54` | `int*` | per-level visited/closed marker arrays. | Yes; `0x0042C38F`, `0x0042C5CD`. |
| PathfinderClass | `+0x58/+0x5C/+0x60` | `float*` | per-level best accumulated cost by zone. | Yes; `0x0042C397`, `0x0042C5DE`, `0x0042C70F`. |
| PathfinderClass | `+0x64` | node pool | 16-byte nodes: parent index, zone id, cost, depth. | Yes; `0x0042C66E..0x0042C690`, `0x0042C879`. |
| PathfinderClass | `+0x68` | heap descriptor | 1-based min-heap of node pointers, compared by node `+8` float cost. | Yes; `0x0042C309`, `0x0042C693`, `0x0042C740`. |
| PathfinderClass | `+0x78 + level*0x18` | `u32*` | sorted packed excluded edge pairs, `(min << 16) | max`. | Yes; scanned at `0x0042C64F..0x0042C664`. |
| PathfinderClass | `+0x84 + level*0x18` | `int` | exclusion count for that level. | Yes; loaded at `0x0042C4D5`, used before edge-skip scan. |
| PathfinderClass | `+0xBC + level*1000` | `u16[500]` | stored chosen zone chain for each level. | Yes; written at `0x0042C887..0x0042C8CE`; consumed by invalidation docs. |
| PathfinderClass | `+0xC74 + level*4` | `int` | stored zone-chain count for each level. | Yes; written at `0x0042C88B`, same-zone path writes count 1. |
| Global | `DAT_0087F858` | `u16[cell][5]` | per-cell hierarchical zone ids; level id at `cell*10 + level*2`. | Yes; reads at `0x0042C339`, `0x0042C35B`. |
| Global | `DAT_0087F878 + level*0x18` | graph header | per-level zone graph pointer; zone records stride `0x24`. | Yes; read at `0x0042C501`, `0x0042C547`. |
| Zone record | `+0x04` | edge pointer | adjacency edge array. | Yes; `0x0042C507`. |
| Zone record | `+0x10` | edge count | adjacency count. | Yes; `0x0042C50E`. |
| Zone record | `+0x18` | `u16` | parent/coarser zone id used by lower-level parent gate. | Yes; `0x0042C554`. |
| Zone record | `+0x1C` | `int` | reduced zone type / matrix column / base-cost index. | Yes; `0x0042C55C`, `0x0042C60E`. |
| Edge entry | `+0x00` | `u32` | neighbor zone id. | Yes; `0x0042C53E`. |
| Edge entry | `+0x04` low byte | flag byte | if nonzero, add `0.001` to candidate edge cost. | Yes; `0x0042C540`, `0x0042C5A2..0x0042C5AE`. |

## 3. Core Logic

### Level order and same-zone handling

The search initializes `level = 2` and decrements through `1` and `0` (`0x0042C300`, `0x0042C8D6..0x0042C8DD`). For each level it:

1. Clears the heap descriptor at `+0x68`.
2. Converts start and destination cells with `ZoneMap__CellToZoneIndex`.
3. Reads start and destination zone ids from `DAT_0087F858[cell*10 + level*2]`.
4. Marks both start and destination zones in the current level's path-marker array.
5. If zone ids match, writes a one-zone path to `+0xBC + level*1000`, writes count `1` to `+0xC74 + level*4`, and continues to the next finer level.

Active in YR: Yes; same function is live from standard foot pathing via `0x004CBBA0 -> 0x0042C900`.

### Graph search shape

When start and destination zones differ, `Zone_precheck` runs a heap-based accumulated-cost graph search over `DAT_0087F878 + level*0x18`. This is Dijkstra-like: no destination heuristic or zone-center Manhattan term is added in the binary path. The heap node cost is the accumulated candidate cost stored at node `+8`.

Candidate edge cost:

`new_cost = current_node.cost + ZoneBaseCost[target_zone_type] + slope_cost + edge_flag_penalty`

Material details:

- `ZoneBaseCost` is the 8-float table at `0x007E3794`; prior verified dump gives `{1,0,0,1,1,0,1,1}`.
- `target_zone_type` is `zone_record[neighbor].+0x1C`, the same reduced zone-type/matrix column used for passability.
- `slope_cost` is `0` unless `param_5` is non-null and `FootClass__Get_Slope_Speed_Factor(param_5) > 1e-5`; when enabled it calls `Zone_Estimate_Slope_Cost(Foot+0x21C, level, current_zone, neighbor_zone)`, converts via `Math__ftol`, multiplies by the slope factor, and adds it as an integer-derived cost (`0x0042C56B..0x0042C59A`).
- `edge_flag_penalty` is the double constant at `0x007E3818` (`~0.001000000047`, not exactly 0.001) when `byte(edge+4) != 0`, otherwise `0.0` (`0x0042C59E..0x0042C5B4`). <!-- corrected 2026-05-28: was "exactly `0.001`"; read_memory at 0x007E3818 gives 0x3F506248D2F1A9FC ≈ 0.001000000047 — OPERATOR_OR_ORDER_DRIFT (cosmetic) --> The byte is a zone-edge tiebreak flag, not proven to mean "bridge edge."

Active in YR: Yes. No TS-only flag gate appears in `Zone_precheck`; the function is on the normal pathfinding spine.

### Edge acceptance filters

A neighbor is inserted only if all of these pass, in this order:

1. It is unvisited this epoch, or the new cost is strictly lower than the stored best cost (`0x0042C5CD..0x0042C5EA`).
2. At level `2`, parent filtering is bypassed. At levels `1` and `0`, the neighbor's parent/coarser zone id at record `+0x18` must be marked in the next-coarser chosen-path marker array, unless the neighbor zone type at `+0x1C` is `1` (`0x0042C5F0..0x0042C604`).
3. `ZonePassabilityMatrix[movementZone * 8 + neighbor_zone_type] == 1` (`0x0042C60A..0x0042C612`).
4. If the current level exclusion count is nonzero, the sorted pair `(current_zone, neighbor_zone)` must not exist in the `+0x78/+0x84` exclusion array (`0x0042C618..0x0042C664`).

Active in YR: Yes; all branches are in the live function.

### Exclusion consumption

The exclusion check canonicalizes zone ids before comparison:

- `min(current, neighbor)` becomes the high halfword.
- `max(current, neighbor)` becomes the low halfword.
- Packed key is `(min << 16) | max`.
- The array at `PathfinderClass+0x78 + level*0x18` is scanned backward from `count-1` to `0`.
- A match skips that edge and continues to the next adjacency entry.

The count is loaded for the current level before the pop/expand loop. `Zone_precheck` itself does not append exclusions; it only consumes the caller-maintained per-search list. `AStar_pathfind_search` clears the three exclusion vectors before the first attempt, while the retry path appends exclusions after cell A* failure and then calls `PathfinderClass__Reset` before calling `Zone_precheck` again.

Active in YR: Yes; direct evidence at `0x0042C4D5`, `0x0042C635..0x0042C664`, and caller sequence `0x0042CC79 -> 0x0042A5B0 -> 0x0042CCB3`.

### Heap and tie behavior

Heap insertion bubbles a new node up only while the parent node cost is greater than the new cost; equal costs do not displace the existing parent (`0x0042C6A4..0x0042C6D8`). Pop sift-down chooses a child only on strict lower cost; equal child costs preserve the current/left-biased existing heap order (`MinHeap__SiftDown @ 0x0042DCA0`, inlined block `0x0042C7C5..0x0042C835`).

Visible implication: tie-breaking is insertion/order dependent, not zone-id ordering. Rust using `(cost, ZoneId)` tuple ordering is not binary-equivalent for equal-cost candidate zones.

Active in YR: Yes; this is the standard heap path in live `Zone_precheck`, and the earlier "TS legacy heap branch" concern is refuted by the heap compare instructions.

### Success path output

When the destination zone is popped:

1. Walk the node parent chain and mark each chosen zone in the current level path-marker array.
2. Write `depth + 1` to `+0xC74 + level*4`.
3. Write the path zones into `+0xBC + level*1000` in start-to-destination order.
4. Continue to the next finer level; after level `0` succeeds, return `1`.

If the heap empties before reaching the destination at any level, return `0` immediately.

Active in YR: Yes; `0x0042C852..0x0042C8F5`.

## 4. INI Keys

`Zone_precheck` reads no INI key directly.

| Key / data | Binary field / data | Effect in this slice | Active in YR |
|---|---|---|---|
| `MovementZone=` | `TechnoTypeClass+0x5B4`, passed as `param_4` when caller uses `-1` override | Matrix row and graph-search passability row. | Yes; `AStar_pathfind_search` reads `+0x5B4` before `Zone_precheck`. |
| `ZonePassabilityMatrix` | `0x0082A594`, `int[13][8]` | Candidate graph edge passes only when value is exactly `1`. | Yes; direct compare at `0x0042C60E`. |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `FootClass__Run_AStar -> AStar_pathfind_search` | normal foot pathfinding reaches `Zone_precheck`. | `0x004CBBA0` decompile, `0x0042C900` xref from `0x004CBC31`. | Yes. |
| Initial precheck in `AStar_pathfind_search` | if start/dest MapClass zone ids match, precheck failure only logs and clears hierarchy; if they differ and hierarchy is enabled, the caller returns failure before cell A*. | `0x0042CB22..0x0042CB8B`. | Yes. |
| Retry precheck | after cell A* fails under hierarchy, caller appends exclusions via `0x0042CCD0`, resets search state, copies `+0x38`, and calls `Zone_precheck` again if hierarchy remains enabled. | `0x0042CC79..0x0042CCB8`. | Yes. |
| `PathfinderClass__EstimateZoneCost` (`FUN_0042D170`) alternate caller | uses `Zone_precheck`; if false, returns `0x7fffffff` as a failed/huge distance estimate. | call at `0x0042D222`, xrefs from `FootClass__Find_Path` and threat/patrol helpers. | Yes; direct `FootClass__Find_Path` call at `0x004D3C9C`. | <!-- corrected 2026-05-28: was `FUN_0042D170`; function is now labeled `PathfinderClass__EstimateZoneCost` in Ghidra via decompile_function 0x0042D170 — RTTI_LABEL_DRIFT -->
| Exclusion producer contract | `0x0042CCD0` / `0x0042CF80` append Pathfinder-local sorted packed undirected edge pairs; `Zone_precheck` consumes them as skip edges. | prior report plus consumer assembly `0x0042C635..0x0042C664`. | Yes. |

## 6. Current Rust Implementation Status

| Surface | Status vs binary slice |
|---|---|
| `src/sim/pathfinding/zone_search.rs::find_path_zoned` | Uses a single-level zone corridor approximation; binary uses three hierarchy levels `2 -> 1 -> 0` with parent-corridor filtering. |
| `src/sim/pathfinding/zone_search.rs::find_zone_corridor` | Uses center Manhattan edge cost plus heuristic (`g+h`); binary uses accumulated target-zone-type base cost + optional slope + optional `~0.001` edge-flag penalty, with no destination heuristic. |
| `zone_search.rs` retry exclusion state | Current code excludes whole zones (`BTreeSet<ZoneId>`) after corridor failure; binary excludes undirected zone edges via packed pairs. |
| `zone_search.rs` corridor expansion | Current code expands the chosen corridor by one neighbor ring before cell A*; no such expansion was found in `Zone_precheck`. |
| `can_use_reduced_zone_precheck` | Current Rust only hard-gates selected movement zones; binary accepts whichever `MovementZone` row the caller passes and applies matrix value `== 1`. |
| `ZoneGrid`/`ZoneMap` | Single graph per movement zone with super-zone cache; binary consumes three global hierarchy graphs independent of MovementZone, and uses MovementZone only as matrix row. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Zone_precheck @ 0x0042C290` main loop | verified | decompile + assembly contexts `0x0042C300..0x0042C8F5` | none for scoped behavior |
| Level order `2 -> 1 -> 0` | verified | `0x0042C300`, `0x0042C8D6..0x0042C8DD` | none |
| Same-zone path write | verified | decompile `0x0042C3B0..0x0042C42A` | none |
| Graph record and edge fields consumed | verified | `0x0042C501..0x0042C55C` | writer-side full build out-of-scope |
| Edge weight formula | verified | `0x0042C56B..0x0042C5D2`; prior constant dumps for `0x007E3794/0x007E3818` | exact semantic label for every zone type column belongs to CellClass zone-type work |
| Parent-corridor gate | verified | `0x0042C5F0..0x0042C604` | none for consumer contract |
| `ZonePassabilityMatrix` gate | verified | `0x0042C60A..0x0042C612`; matrix report | none |
| Exclusion consumption | verified | `0x0042C618..0x0042C664`; producer report | none |
| Heap tie behavior | verified | `0x0042C6A4..0x0042C6D8`, `0x0042DCA0`, `0x0042C7C5..0x0042C835` | exact original insertion order depends on graph build order, out-of-scope |
| `AStar_pathfind_search` integration | verified | decompile `0x0042C900`, calls at `0x0042CB58`, `0x0042CCB3` | none |
| `FUN_0042D170` integration | touched-not-exhausted | decompile `0x0042D170`, xrefs | full nearby-cell fallback not covered |
| Current Rust `zone_search.rs` | verified for scan | Codegraph + file read | implementation not changed |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is this an exhaustive slice or coverage map? -> exhaustive-slice for `0x0042C290` consumer behavior and direct integration.` (evidence: user scope)
- `[RESOLVED] OQ-2 -- Is `Zone_precheck` live in standard YR? -> Yes, via `FootClass__Run_AStar -> AStar_pathfind_search` and `FootClass__Find_Path -> FUN_0042D170`.` (evidence: `0x004CBBA0`, `0x0042C900`, `0x004D3C9C`, `0x0042D222`)
- `[RESOLVED] OQ-3 -- Does it search one or three levels? -> three levels, order `2,1,0`.` (evidence: `0x0042C300`, `0x0042C8D6..0x0042C8DD`)
- `[RESOLVED] OQ-4 -- Does it use a heuristic/centroid Manhattan cost? -> no heuristic seen; heap key is accumulated node cost.` (evidence: `0x0042C5BB..0x0042C5D2`, `0x0042C689`, `0x0042C6B3`)
- `[RESOLVED] OQ-5 -- What are edge weights? -> target zone-type base cost + accumulated cost + optional slope + optional `0.001` edge-flag penalty.` (evidence: `0x0042C55C`, `0x0042C56B..0x0042C5D2`)
- `[RESOLVED] OQ-6 -- How is `ZonePassabilityMatrix` used? -> row is movement zone, column is neighbor zone type, only value `1` passes.` (evidence: `0x0042C60A..0x0042C612`; `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-7 -- Are exclusions zones or edges? -> edges, sorted undirected packed pairs.` (evidence: `0x0042C620..0x0042C664`)
- `[RESOLVED] OQ-8 -- Where are exclusions stored? -> `PathfinderClass+0x78/+0x84 + level*0x18`.` (evidence: `0x0042C645..0x0042C653`)
- `[RESOLVED] OQ-9 -- Does `Zone_precheck` append exclusions? -> no, it consumes them; appends are in retry update/invalidation functions.` (evidence: no writes to `+0x78/+0x84` in `0x0042C290`; producer report)
- `[RESOLVED] OQ-10 -- What happens on same-zone per level? -> writes count-one path and continues.` (evidence: `0x0042C3B0..0x0042C42A`)
- `[RESOLVED] OQ-11 -- How are lower levels constrained? -> level 1/0 candidate parent must be on next-coarser path, except zone type `1`; level 2 has no parent gate.` (evidence: `0x0042C5F0..0x0042C604`)
- `[RESOLVED] OQ-12 -- How are ties handled? -> strict lower-cost comparisons; equal costs preserve heap/insertion order, not zone-id order.` (evidence: `0x0042C6A4..0x0042C6D8`, `0x0042DCA0`)
- `[RESOLVED] OQ-13 -- What does failure return? -> returns zero immediately if any level exhausts without destination; `AStar_pathfind_search` treats same-zone/cross-zone failures differently.` (evidence: `0x0042C8E2`, `0x0042CB22..0x0042CB8B`)
- `[RESOLVED] OQ-14 -- What Rust surface is affected? -> `src/sim/pathfinding/zone_search.rs`, plus zone graph data shape if exact hierarchy is implemented.` (evidence: Codegraph/file scan)
- `[DEFERRED] OQ-15 -- Exact writer-side ordering of adjacency arrays, which influences equal-cost tie order.` (category: out-of-scope; reason: this slot is consumer-side `Zone_precheck`; next-step-if-pursued: audit global hierarchy build order and final edge emission ordering)
- `[DEFERRED] OQ-16 -- Full `Zone_Estimate_Slope_Cost` parity.` (category: out-of-scope; reason: only its cost contribution contract is needed for this slot; next-step-if-pursued: dedicated slope-cost investigation)
- `[DEFERRED] OQ-17 -- Runtime frequency of `FUN_0042D170` path-quality caller in common skirmish actions.` (category: needs-runtime-debugger; reason: static xrefs prove reachability but not frequency; next-step-if-pursued: instrument FootClass find-path code 6/7 branches)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Zone_precheck` excludes undirected edges, not zones, by scanning packed sorted pairs at `+0x78/+0x84` per level. | `0x0042C620..0x0042C664`; producer report `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md` | mismatch: `zone_search.rs` excludes whole corridor zones in `BTreeSet<ZoneId>`. | `src/sim/pathfinding/zone_search.rs` retry/corridor state. | Store/search per-level canonical edge exclusions and skip only matching graph edges during zone search. | Build graph `A-B-C` plus `A-D-C`; exclude `A-B`; search still uses `A-D-C` and does not ban zone `B` globally. Proposed test name: `zoned_retry_excludes_edges_not_whole_zones`. | Do not convert a failed corridor into zone bans; that can remove valid alternate paths through the same zone. |
| Binary graph search is three-level `2 -> 1 -> 0`; lower levels are constrained by the next-coarser chosen path, except zone type `1`; level 2 is unconstrained. | `0x0042C300`, `0x0042C5F0..0x0042C604`, `0x0042C887..0x0042C8DD` | missing: Rust uses one adjacency graph plus one-ring corridor expansion. | `src/sim/pathfinding/zone_search.rs`, future hierarchical zone data in `zone_map/zone_build`. | Implement exact stored per-level paths before applying cell-A* corridor pruning; do not expand by a free neighbor ring unless separately verified. | Multi-level graph where a fine-level off-corridor edge is tempting but parent is not on level-2 path; precheck rejects it unless target type is `1`. Proposed test name: `zone_precheck_prunes_fine_edges_outside_parent_corridor`. | Do not approximate parent gating with connected-component reachability or arbitrary corridor widening. |
| Edge cost is target zone-type base cost `{1,0,0,1,1,0,1,1}` plus optional slope and `0.001` edge-flag penalty; no centroid Manhattan heuristic participates in binary precheck. | `0x0042C55C`, `0x0042C5BB..0x0042C5D2`; prior constant dumps | mismatch: `find_zone_corridor` uses Manhattan center distance and pushes `g+h`. | `src/sim/pathfinding/zone_search.rs::find_zone_corridor`; zone record metadata. | Use binary zone type/flag/slope cost inputs and heap accumulated cost; tie behavior should preserve insertion order on equal costs. | Equal-center-distance graph where a zero-cost type-1 branch competes with type-0 branch should pick the binary lower-cost path, not shortest centroid path. Proposed test name: `zone_precheck_uses_zone_type_cost_not_centroid_distance`. | Do not use `BinaryHeap<(cost, ZoneId)>` tie ordering as a stand-in for gamemd's heap order. |

### Negative Facts / Do Not Do

- Do not exclude whole zones after a failed corridor. Evidence: consumer skip key is packed `(min(current, next) << 16) | max(current, next)` at `0x0042C635..0x0042C664`; Active in YR: Yes.
- Do not use centroid Manhattan or `g+h` A* as the binary `Zone_precheck` graph search. Evidence: candidate cost comes from `0x007E3794[target_zone_type] + parent cost + slope + flag`, with heap compares on node `+8`; Active in YR: Yes.
- Do not collapse the three hierarchy levels into one when claiming parity. Evidence: loop is exactly `2 -> 1 -> 0`, and lower levels use next-coarser path markers; Active in YR: Yes.
- Do not treat `edge+4` low byte as proven "bridge edge." Evidence: `BRIDGE_ZONE_EDGE_FLAGS_GHIDRA_REPORT.md` verified repaired bridge edges write zero; `Zone_precheck` only treats nonzero as `0.001` tiebreak input; Active in YR: Yes.
- Do not gate `Zone_precheck` to a hand-picked subset of MovementZones for binary parity. Evidence: caller passes `MovementZone` row and `Zone_precheck` applies matrix `==1` for that row; Active in YR: Yes.

### Remaining Uncertainty

- Exact adjacency array writer order remains out-of-scope; it affects equal-cost insertion-order ties.
- Full `Zone_Estimate_Slope_Cost` internals were not re-documented here; only the call/added-cost contract was verified.
- Static xrefs prove alternate caller reachability, but not frequency in ordinary skirmish play.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
  - Replace: "`edge.flags_low_byte != 0` / bridge_edge / `1 if bridge-edge`"
  - With: "`byte(edge+4) != 0` is a zone-edge tiebreak flag that adds `0.001`; it is not proven to mean bridge edge, and repaired bridge-added edges are verified to write zero."
- `C:/Users/enok/Documents/ra2-rust-game-docs/PATHFINDING_ASTAR_GHIDRA_REPORT.md`
  - Replace any `Zone_precheck` wording that says edge cost is based on LandType or centroid/manhattan distance with: "edge cost is accumulated `ZoneBaseCost[neighbor reduced zone type] + optional slope + optional edge-flag 0.001`; the same reduced zone type indexes `ZonePassabilityMatrix`."

## Sources

- Ghidra decompiled: `0x0042C290`, `0x0042C900`, `0x0042D170`, `0x0042CF80`, `0x0042D830`, `0x0042DCA0`, `0x004DC760`, `0x00585F40`, `0x004CBBA0`, `0x004D3920`.
- Ghidra xrefs / assembly contexts: `0x0042CB58`, `0x0042CCB3`, `0x0042D222`, `0x0042C4EB`, `0x0042C52C`, `0x0042C56B`, `0x0042C5BB`, `0x0042C60E`, `0x0042C635`, `0x0042C6A0`, `0x0042C865`.
- Prior reports: `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`, `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, `BRIDGE_ZONE_EDGE_FLAGS_GHIDRA_REPORT.md`, `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`, `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`.
- Rust scan: Codegraph context and file reads for `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_search_tests.rs`.
