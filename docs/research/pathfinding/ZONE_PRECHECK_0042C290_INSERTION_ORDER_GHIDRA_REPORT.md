# Zone_precheck 0x0042C290 Insertion Order -- Ghidra Research Report

**Address(es):** `0x0042C290` primary; callers `0x0042C900`, `0x0042D170`; heap helper `0x0042DCA0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** consumer-side `Zone_precheck` insertion-order/tie behavior: adjacency iteration order, heap insertion/replacement conditions, equal-cost behavior, zone-chain write order, edge exclusion handling, and whether any ZoneId sort/order participates in ties.  
**Non-Scope:** full A* cell loop, full zone graph construction, full `Zone_Estimate_Slope_Cost`, and runtime route capture on a named stock map.  
**Confidence:** High for scoped binary behavior; Medium for Rust delta because this report did not run tests or modify code.  
**Active in YR:** Yes. `AStar_pathfind_search @ 0x0042C900` calls `Zone_precheck` at `0x0042CB58` and `0x0042CCB3`; alternate path-quality helper `FUN_0042D170` calls it at `0x0042D222`.

## 0. Investigation Contract

Target question: does `Zone_precheck @ 0x0042C290` preserve adjacency insertion order for equal-cost zone paths, and what exact binary details should `zone_search.rs` reproduce?

Non-goals: do not re-investigate the whole dual-layer A* system, cell-level edge costs, low-bridge `TubeClass`, or the zone graph writer except where needed to explain the consumer's input order.

Evidence needed to mark COMPLETE: decompile plus assembly context for the adjacency scan, candidate replacement gate, heap insertion, heap pop/sift-down, path-chain write, caller reachability, and bridge/edge exclusion skip. Also verify that no ZoneId sort/order participates in the scoped tie decisions.

Stop conditions: stop after the above points are verified and Rust-facing deltas are identified; mark partial only if assembly cannot confirm equal-cost behavior or caller reachability.

## 1. Overview

`Zone_precheck` is a three-level hierarchical graph search over global zone graphs. It runs levels `2 -> 1 -> 0`, records the selected zone chain for each level into `PathfinderClass+0xBC + level*1000`, and returns false when any level cannot reach the destination zone.

For tie parity, the key finding is narrow: `Zone_precheck` scans each zone's adjacency array in stored order and uses strict lower-cost comparisons for replacement and heap movement. Equal-cost candidates do not replace earlier candidates and do not bubble ahead. No ZoneId sort or ZoneId tuple ordering appears in the consumer search.

## 2. Class Layout / Key Offsets

| Owner | Offset / address | Type / shape | Purpose | Active in YR |
|---|---:|---|---|---|
| PathfinderClass | `+0x28` | epoch/stamp | Marker value written into chosen/closed arrays. | Yes; read through `0x0042C300..0x0042C8F5`. |
| PathfinderClass | `+0x40/+0x44/+0x48` | `int*` | per-level chosen-path marker arrays for levels `0/1/2`. | Yes; current level uses `+0x40+level*4`, parent gate uses `+0x44+level*4`. |
| PathfinderClass | `+0x4C/+0x50/+0x54` | `int*` | per-level closed/visited marker arrays. | Yes; read at `0x0042C5CD`, written at `0x0042C71B`. |
| PathfinderClass | `+0x58/+0x5C/+0x60` | `float*` | per-level best accumulated cost by zone. | Yes; replacement compare at `0x0042C5DE..0x0042C5EA`. |
| PathfinderClass | `+0x64` | node pool | 16-byte nodes: parent index, zone id, cost, depth. | Yes; node write at `0x0042C66E..0x0042C690`, path walk at `0x0042C867..0x0042C8CE`. |
| PathfinderClass | `+0x68` | heap descriptor | 1-based min-heap of node pointers, compared by node `+8` float cost. | Yes; insert at `0x0042C693..0x0042C6F1`, pop/sift at `0x0042C740..0x0042C835`. |
| PathfinderClass | `+0x78 + level*0x18` | `u32*` | excluded undirected zone-edge keys. | Yes; consumer scan at `0x0042C635..0x0042C664`. |
| PathfinderClass | `+0x84 + level*0x18` | `int` | exclusion count for level. | Yes; loaded before neighbor loop at `0x0042C4D5`. |
| PathfinderClass | `+0xBC + level*1000` | `u16[500]` | selected zone chain, start-to-destination order. | Yes; written at `0x0042C887..0x0042C8CE`. |
| PathfinderClass | `+0xC74 + level*4` | `int` | selected chain length. | Yes; written at `0x0042C88B`; same-zone path writes `1`. |
| Global | `DAT_0087F858` | `u16[cell][5]` | per-cell hierarchical zone ids; current level at `cell*10 + level*2`. | Yes; reads at `0x0042C339`, `0x0042C35B`. |
| Global | `DAT_0087F878 + level*0x18` | graph header | per-level zone graph base. | Yes; current graph read at `0x0042C501`, `0x0042C547`. |
| Zone record | `+0x04` | edge pointer | adjacency array pointer for this zone. | Yes; loaded at `0x0042C507`. |
| Zone record | `+0x10` | edge count | number of adjacency entries. | Yes; loaded at `0x0042C50E`; loop decrements to zero. |
| Zone record | `+0x18` | `u16` | parent/coarser zone id for lower-level parent-path gate. | Yes; loaded at `0x0042C554`. |
| Zone record | `+0x1C` | `int` | reduced zone type, used for base cost and passability matrix column. | Yes; loaded at `0x0042C55C`. |
| Edge entry | `+0x00` | `u32` | neighbor zone id. | Yes; loaded at `0x0042C53E`. |
| Edge entry | `+0x04` low byte | flag byte | nonzero adds `0.001` to candidate cost. | Yes; loaded at `0x0042C540`, tested via decompile around `0x0042C59E..0x0042C5B4`. |

## 3. Core Logic

### 3.1 Level Order

The function initializes `level = 2` and decrements through `1` then `0`. Assembly around `0x0042C8D6..0x0042C8DD` performs `DEC ESI`, stores it back to the local level slot, jumps back while nonnegative, and returns `1` only after level `0` succeeds.

Active in YR: Yes. Evidence: `0x0042C300`, `0x0042C8D6..0x0042C8ED`; callers `0x0042CB58`, `0x0042CCB3`, `0x0042D222`.

### 3.2 Neighbor Iteration Order

For a popped node, the function obtains the current zone's adjacency pointer/count from the graph:

- `0x0042C501`: load graph header for the current level.
- `0x0042C507`: load edge pointer from zone record `+0x04`.
- `0x0042C50E`: load edge count from zone record `+0x10`.
- `0x0042C53E`: read `neighbor = *(edge+0)`.
- `0x0042C540`: read `edge_flag = byte(edge+4)`.
- `0x0042C726`: advance edge pointer by two dwords (`local_3c = local_3c + 2`) and decrement count.

There is no sort, binary search, or ZoneId-priority step between loading `edge+0` and evaluating the edge. The input adjacency array order is the expansion order.

Active in YR: Yes. Evidence: decompile `0x0042C290`; assembly context `0x0042C501..0x0042C540`, `0x0042C726`.

### 3.3 Candidate Cost and Replacement Gate

Candidate cost is accumulated graph cost, not centroid A*:

`candidate = parent_cost + ZoneBaseCost[neighbor_zone_type] + slope_cost + edge_flag_penalty`

Material order:

- `0x0042C55C`: load neighbor zone type from zone record `+0x1C`.
- `0x0042C56B..0x0042C59A`: optional `Zone_Estimate_Slope_Cost` path when a foot object has nonzero slope factor.
- `0x0042C5BB`: load `ZoneBaseCost` from `0x007E3794 + type*4`.
- `0x0042C5C2`: add parent node cost from node `+8`.
- `0x0042C5C9`: add integer slope contribution.
- `0x0042C5D0..0x0042C5D2`: add edge flag penalty and store candidate cost.

Replacement test:

- If closed marker for `neighbor` does not match the current epoch, the candidate can continue.
- If it already matches, assembly `0x0042C5DE..0x0042C5EA` compares existing best cost against the new candidate and jumps to skip unless the new candidate is strictly lower.

The floating compare sequence is `FLD existing`, `FCOMP candidate`, `FNSTSW AX`, `TEST AH,0x41`, `JNZ skip`. In this x87 pattern, equality falls into the skip case. Therefore equal-cost paths do not replace an existing node/parent.

Active in YR: Yes. Evidence: `0x0042C5CD..0x0042C5EA`; caller evidence above.

### 3.4 Parent-Gate, Passability, and Edge Exclusion Order

After cost replacement passes, the remaining filters run in this order:

1. Parent-path gate: level `2` bypasses it; levels `1/0` require the neighbor's parent zone to be marked on the next-coarser chosen path, unless neighbor zone type is `1`. Assembly `0x0042C5F0..0x0042C604`.
2. `ZonePassabilityMatrix[movementZone * 8 + neighbor_type] == 1`. Assembly `0x0042C60A..0x0042C612`.
3. Per-level edge exclusion scan. Assembly `0x0042C618..0x0042C664`.

The exclusion key is an undirected sorted pair: the code compares the two zone ids, places the smaller id in the high halfword and the larger in the low halfword, then linearly scans the exclusion vector backward from `count-1`. A match jumps to `0x0042C726` and skips only that edge.

Active in YR: Yes. Evidence: `0x0042C5F0..0x0042C664`; exclusion producer/consumer callers `0x0042CC79 -> 0x0042CF80 -> 0x0042CCB3`.

### 3.5 Heap Insertion and Equal-Cost Behavior

Accepted candidates are appended to the node pool in discovery order:

- `0x0042C66E..0x0042C690`: write parent index, neighbor zone id, candidate cost, and depth.
- `0x0042C693..0x0042C6A4`: compute heap insertion slot.
- `0x0042C6B3..0x0042C6BF`: compare parent heap node cost to new candidate cost.
- `0x0042C6BF`: if the parent is not strictly greater, jump to placement.
- `0x0042C6C1..0x0042C6D6`: bubble parent down only while parent cost is strictly greater.

Equal cost does not bubble the new node above the existing parent. There is no comparison on zone id, insertion counter stored explicitly, or graph coordinate. Stability comes from the combination of adjacency scan order, node-pool append order, and strict heap comparisons.

Active in YR: Yes. Evidence: decompile `0x0042C290`; assembly `0x0042C693..0x0042C6D8`.

### 3.6 Heap Pop / Sift-Down Tie Behavior

The first pop uses `MinHeap__SiftDown @ 0x0042DCA0`; the hot-path pop later in `Zone_precheck` contains equivalent inlined logic at `0x0042C740..0x0042C835`.

Verified helper behavior:

- Left child is selected over parent only if left cost is strictly lower.
- Right child is selected over the current candidate only if right cost is strictly lower.
- Equal child costs do not replace the current candidate.

The inlined pop uses the same strict comparisons:

- `0x0042C7AA..0x0042C7B7`: compare right child versus current candidate; equality does not select right.
- `0x0042C81A..0x0042C827`: repeated child comparison; equality does not change selected child.
- `0x0042C82D..0x0042C835`: exits when selected child equals current slot.

Active in YR: Yes. Evidence: decompile `0x0042DCA0`; assembly contexts `0x0042C7B0..0x0042C835`.

### 3.7 Destination and Zone-Chain Write Order

When the popped node's zone equals destination, the function first marks every node in the parent chain as chosen for the current level, then writes the chain buffer in start-to-destination order:

- `0x0042C867..0x0042C881`: walk from destination node backward through parent indices and stamp each zone in the chosen-path marker array.
- `0x0042C887..0x0042C88B`: write `depth + 1` to `PathfinderClass+0xC74+level*4`.
- `0x0042C897..0x0042C8BF`: start writing at `+0xBC + (depth + level*500)*2`, store current zone, decrement pointer by one word, and follow parent until start.
- `0x0042C8C1..0x0042C8CE`: write the start zone at `+0xBC + level*1000`.

The stored zone chain is therefore start first, destination last. If equal-cost alternatives exist, the chain is whichever destination node is popped first under the insertion-order heap behavior above.

Active in YR: Yes. Evidence: assembly `0x0042C867..0x0042C8CE`.

### 3.8 Negative Tie Fact: No ZoneId Sort

Within `Zone_precheck`, ZoneId participates in:

- adjacency index (`zone * 0x24`),
- exclusion pair canonicalization (`min/max` for edge identity only),
- array lookups for marker/cost/path storage,
- final chain writes.

It does not participate as a tie-break key for candidate replacement, heap insertion, heap pop, or path-chain choice. The only explicit ordering by ZoneId is the exclusion key canonicalization, which is used only to skip a matching edge and does not sort candidate neighbors or choose between equal paths.

Active in YR: Yes. Evidence: complete decompile of `0x0042C290` plus assembly contexts listed above.

## 4. INI Keys

`Zone_precheck` reads no INI key directly.

| Key / data | Binary source | Effect in this slice | Active in YR |
|---|---|---|---|
| `MovementZone=` | caller reads TechnoType movement zone and passes it as `param_4` | Selects the `ZonePassabilityMatrix` row used at `0x0042C60E`. | Yes. |
| `ZonePassabilityMatrix` | binary global `0x0082A594` | Candidate neighbor passes only when row/column value is exactly `1`. | Yes; compare at `0x0042C60E`. |
| `[General] DestroyableBridges=yes` | `ini/rulesmd.ini` | Makes bridge edge invalidation/collapse scenarios reachable in standard YR; not read by `Zone_precheck` itself. | Yes by default in YR rules. |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Initial hierarchical precheck | `AStar_pathfind_search` calls `Zone_precheck` before cell A* when hierarchical search is enabled. | call at `0x0042CB58`; failure handling `0x0042CB5D..0x0042CB8B`. | Yes. |
| Retry hierarchical precheck | After a failed cell A* under hierarchy, caller updates edge exclusions, resets, and calls `Zone_precheck` again. | `0x0042CC79..0x0042CCB8`; call at `0x0042CCB3`. | Yes. |
| Alternate distance/path-quality helper | `FUN_0042D170` calls `Zone_precheck` and returns `0x7fffffff` if it fails. | call at `0x0042D222`; failure branch `0x0042D227..0x0042D452`. | Yes. |
| Edge exclusion producer | `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80` appends packed undirected edge keys; `FUN_0042D830` is the append helper. | decompile `0x0042CF80`, `0x0042D830`. | Yes. |

## 6. Current Rust Implementation Status

| Surface | Status vs binary slice |
|---|---|
| `src/sim/pathfinding/zone_search.rs::find_zone_corridor` | The specific `ZoneId` tuple-tie mismatch is stale: current code uses `BinaryHeap<ZoneQueueEntry>` ordered by cost then discovery `sequence`, and replacement remains strict `new_cost < dist[...]`; it no longer uses `Reverse<(f_cost, g_cost, ZoneId)>` for ties (corrected 2026-06-01: was tuple ordering by `ZoneId`; source scan shows `ZoneQueueEntry` at `zone_search.rs:599..690`, while binary shows no ZoneId tie via `decompile_function 0x0042C290` and `get_assembly_context 0x0042C5DE,0x0042C6B3` - STALE). |
| `zone_search.rs::find_zone_corridor` | Legacy corridor cost still differs, but the stale part is `g+h`: current `find_zone_corridor` is Dijkstra over centroid-Manhattan edge cost with no destination heuristic; newer `zone_hierarchy.rs::zone_precheck_flat` uses zone base cost plus edge-flag cost, with slope still explicitly omitted in that flat slice (corrected 2026-06-01: was center Manhattan `g+h`; source scan shows `zone_search.rs:623..682` and `zone_hierarchy.rs:28,334..443`, while binary cost/no-heuristic is confirmed via `decompile_function 0x0042C290` and `get_assembly_context 0x0042C55C..0x0042C5D2` - STALE). |
| `zone_search.rs` corridor retry | Mixed: legacy corridor retry still has `ZoneEdge` exclusions and one-ring expansion after failure; `zone_hierarchy.rs::ZonePrecheckExclusions` now models the per-level undirected edge-key consumer and ordered producer append, but the real failed-A* producer/invalidation lifecycle is still deferred in the live `zone_precheck_flat` call path (corrected 2026-06-01: was only the legacy `ZoneEdge` approximation; source scan shows `zone_search.rs:427..458`, `zone_hierarchy.rs:240..279,419..443`, while binary edge-exclusion producer/consumer is confirmed via `decompile_function 0x0042C290,0x0042CF80,0x0042D830` - STALE). |
| `zone_search.rs::expand_corridor` | Still a mismatch for the legacy corridor fallback, but it is not part of the newer `zone_precheck_flat` path; the hierarchy path passes selected level-0 markers/path to `find_path_with_costs_hierarchy_marker_progress` instead of doing free one-ring expansion (corrected 2026-06-01: was framed as the current `Zone_precheck` parity path; source scan shows `zone_search.rs:287..304,431..434,794..797`, while binary no-expansion consumer is confirmed via `decompile_function 0x0042C290` and `get_assembly_context 0x0042C5F0..0x0042C664` - STALE). |
| `src/sim/pathfinding/zone_build.rs` | Mixed: the current `extract_adjacency` path preserves insertion-order unique adjacency for the final `ZoneAdjacency`, but `build_node_adjacency` still sorts/dedups node adjacency, which may matter if used as a parity-facing hierarchy input. |
| `src/sim/pathfinding/zone_map.rs::ZoneAdjacency::are_adjacent` | Current scan shows `contains`, which is compatible with insertion-order adjacency lists. |
| `src/sim/pathfinding/zone_hierarchy.rs` | Stale: this file now contains both the old `SuperZoneMap` reachability cache and a `ZoneHierarchy`/`zone_precheck_flat` consumer-side `Zone_precheck` scaffold with level order `2 -> 1 -> 0`, stable insertion-order queue ties, parent-path gate/type-1 exception, passability, per-level edge exclusions, and per-level path/marker output (corrected 2026-06-01: was "not a `Zone_precheck` tie surface"; source scan shows `zone_hierarchy.rs:97..279,302..460`, while binary behavior is confirmed via `decompile_function 0x0042C290` and `get_assembly_context 0x0042C5F0..0x0042C8CE` - STALE). |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Zone_precheck @ 0x0042C290` adjacency scan | verified | decompile plus assembly `0x0042C501..0x0042C540`, `0x0042C726` | none for consumer order |
| Candidate replacement strictness | verified | assembly `0x0042C5CD..0x0042C5EA` | none |
| Heap insertion strictness | verified | assembly `0x0042C693..0x0042C6D8` | none |
| Heap pop/sift-down strictness | verified | decompile `0x0042DCA0`; assembly `0x0042C7B0..0x0042C835` | none |
| Zone-chain write order | verified | assembly `0x0042C867..0x0042C8CE` | none |
| Edge exclusion handling | verified | assembly `0x0042C618..0x0042C664`; producer decompile `0x0042CF80`, `0x0042D830` | none for consumer contract |
| ZoneId sort/order in ties | verified negative | complete decompile `0x0042C290`; strict heap/cost assembly contexts | none |
| Caller reachability | verified | `0x0042CB58`, `0x0042CCB3`, `0x0042D222` | runtime frequency not measured |
| Full zone graph writer order | touched-not-exhausted | prior reports `BRIDGE_PATH_TIE_ORDER_AFTER_LOW_COLLAPSE...`, `BRIDGE_ZONE_INCREMENTAL_REFRESH...` | separate writer-side exhaustive audit if needed |
| Current Rust deltas | verified for scan | `rg` and file reads for `zone_search.rs`, `zone_build.rs`, `zone_map.rs`, `zone_hierarchy.rs` | no tests run; no code changed |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Entry point: is `0x0042C290` the live precheck? -> Yes, caller sites are `0x0042CB58`, `0x0042CCB3`, and `0x0042D222`.` (evidence: decompile/assembly caller contexts; Active in YR: Yes)
- `[RESOLVED] OQ-2 -- Does neighbor iteration preserve adjacency array order? -> Yes; the edge pointer is advanced linearly by 8 bytes and count is decremented, with no sort stage.` (evidence: `0x0042C501..0x0042C540`, `0x0042C726`; Active in YR: Yes)
- `[RESOLVED] OQ-3 -- Does equal candidate cost replace an existing candidate? -> No; the x87 compare jumps to skip on equality.` (evidence: `0x0042C5DE..0x0042C5EA`; Active in YR: Yes)
- `[RESOLVED] OQ-4 -- Does equal heap insertion bubble ahead of an existing parent? -> No; bubble-up occurs only while parent cost is strictly greater than new cost.` (evidence: `0x0042C6B3..0x0042C6D8`; Active in YR: Yes)
- `[RESOLVED] OQ-5 -- Does equal heap pop prefer lower ZoneId? -> No ZoneId compare exists; child selection uses strict lower float cost only.` (evidence: `0x0042DCA0`, `0x0042C7B0..0x0042C835`; Active in YR: Yes)
- `[RESOLVED] OQ-6 -- What order is the chosen chain written? -> Start-to-destination in `+0xBC + level*1000`, with length in `+0xC74 + level*4`.` (evidence: `0x0042C887..0x0042C8CE`; Active in YR: Yes)
- `[RESOLVED] OQ-7 -- Are exclusions zones or edges? -> Edges; sorted undirected packed pairs are scanned backward and only matching edge is skipped.` (evidence: `0x0042C620..0x0042C664`; Active in YR: Yes)
- `[RESOLVED] OQ-8 -- Does bridge-edge exclusion sort candidate paths? -> No; canonicalization is only for edge identity lookup after passability/parent gates.` (evidence: `0x0042C618..0x0042C664`; Active in YR: Yes)
- `[RESOLVED] OQ-9 -- Does `Zone_precheck` use centroid Manhattan or destination heuristic? -> No; candidate cost is accumulated zone-type/slope/edge-flag cost.` (evidence: `0x0042C55C..0x0042C5D2`; Active in YR: Yes)
- `[RESOLVED] OQ-10 -- Does lower-level search expand outside the parent chain? -> Generally no; levels 1/0 require the neighbor parent zone to be marked on the coarser path, except zone type `1`.` (evidence: `0x0042C5F0..0x0042C604`; Active in YR: Yes)
- `[RESOLVED] OQ-11 -- Does same-zone case run Dijkstra? -> No; it writes a one-zone path and continues to next level.` (evidence: decompile `0x0042C3B0..0x0042C42A`; Active in YR: Yes)
- `[RESOLVED] OQ-12 -- What happens if a level exhausts heap before destination? -> Immediate return `0`.` (evidence: `0x0042C8E2`; Active in YR: Yes)
- `[RESOLVED] OQ-13 -- Is any TS-only flag gate present in the scoped function? -> No TS/fog/special-mode gate appears; MovementZone matrix controls passability.` (evidence: complete decompile `0x0042C290`; Active in YR: Yes)
- `[RESOLVED] OQ-14 -- Current Rust tie risk? -> The original `BinaryHeap<Reverse<(f_cost, g_cost, ZoneId)>>` risk is stale: `find_zone_corridor` and `zone_precheck_flat` now use queue entries ordered by cost then discovery sequence, not ZoneId; remaining Rust risks are cost/integration/exclusion-producer fidelity, not this specific ZoneId tuple tie.` (corrected 2026-06-01: was tuple tie by `ZoneId`; source scan shows `zone_search.rs:599..690` and `zone_hierarchy.rs:308..443`, while binary no-ZoneId tie is confirmed via `decompile_function 0x0042C290` and `get_assembly_context 0x0042C5DE,0x0042C6B3,0x0042C7AA` - STALE; Active in YR: implementation delta)
- `[RESOLVED] OQ-15 -- Current Rust adjacency sort risk? -> final `ZoneAdjacency` access is insertion-order-capable, but `build_node_adjacency` still sorts/dedups an upstream node graph.` (evidence: `src/sim/pathfinding/zone_build.rs`; Active in YR: implementation delta)
- `[DEFERRED] OQ-16 -- Exact full writer-side adjacency emission order for all graph levels.` (category: out-of-scope; reason: this slot is consumer-side `Zone_precheck`; next-step-if-pursued: audit `ZoneMap__BuildZoneLevel` and incremental writer in a dedicated slot)
- `[DEFERRED] OQ-17 -- Full `Zone_Estimate_Slope_Cost` internals.` (category: out-of-scope; reason: only its contribution point affects this tie-order slice; next-step-if-pursued: dedicated slope-cost report)
- `[DEFERRED] OQ-18 -- Runtime frequency of exact equal-cost zone ties in stock maps.` (category: needs-runtime-debugger; reason: static binary proves tie rule but not incidence; next-step-if-pursued: instrument stock map path requests and log chosen zone chains)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Zone_precheck` scans adjacency arrays in stored order and equal-cost candidates keep the earlier parent. | `0x0042C501..0x0042C540`, `0x0042C5DE..0x0042C5EA`, `0x0042C6B3..0x0042C6D8` | The specific heap-tuple `ZoneId` tie mismatch is resolved in current `find_zone_corridor` and `zone_precheck_flat`; both use discovery `sequence` after cost and strict lower-cost replacement (corrected 2026-06-01: was `find_zone_corridor` tuple includes `ZoneId`; source scan shows `zone_search.rs:599..690`, `zone_hierarchy.rs:308..443`, while binary no-ZoneId tie is confirmed via `decompile_function 0x0042C290` and `get_assembly_context 0x0042C5DE,0x0042C6B3` - STALE). | `src/sim/pathfinding/zone_search.rs::find_zone_corridor`; `src/sim/pathfinding/zone_hierarchy.rs::zone_precheck_flat` | Keep stable insertion-order tie behavior: no ZoneId tie key for equal cost; replacement only on strictly lower cost. | Graph with start connected to two equal-cost neighbors where adjacency order is `[high_id, low_id]`; selected chain must start with `high_id`, not lower ZoneId. | Do not reintroduce tuple ordering `(cost, ZoneId)` or sorted adjacency as a proxy for binary heap behavior. |
| `Zone_precheck` cost is accumulated zone-type base + optional slope + optional `0.001` edge flag; no destination heuristic participates. | `0x0042C55C..0x0042C5D2` | Legacy `find_zone_corridor` still uses centroid-Manhattan Dijkstra, but the stale `g+h` claim is wrong; `zone_precheck_flat` now uses base zone cost plus edge-flag tiebreak cost, with slope contribution deferred/zero in the flat slice (corrected 2026-06-01: was Manhattan center distance and `g+h`; source scan shows `zone_search.rs:623..682`, `zone_hierarchy.rs:28,436..443`, while binary cost/no-heuristic is confirmed via `decompile_function 0x0042C290` and `get_assembly_context 0x0042C55C..0x0042C5D2` - STALE). | `src/sim/pathfinding/zone_search.rs::find_zone_corridor`; `src/sim/pathfinding/zone_hierarchy.rs::zone_precheck_flat`; zone metadata | Use binary cost inputs and prove the integer scale preserves binary float ordering; add the omitted slope contribution before claiming full parity for sloped movers. | Fixture where a geometrically shorter corridor has higher binary zone-type cost; chosen corridor must follow binary cost, not center distance. | Do not call centroid Manhattan corridor search binary-equivalent; do not forget the slope path for foot objects with nonzero slope factor. |
| Retry exclusions are undirected edge keys and skip only matching graph edges. | `0x0042C620..0x0042C664`; producer `0x0042CF80`, append helper `0x0042D830` | Consumer-side `ZonePrecheckExclusions` now exists with canonical edge keys and ordered producer append; the remaining delta is integration: live `zone_precheck_flat` is called with default exclusions, and the exact failed-A* producer/invalidation lifecycle remains deferred; legacy corridor retry still expands the allowed set (corrected 2026-06-01: was only `ZoneEdge` plus approximate corridor expansion; source scan shows `zone_search.rs:287..304,427..458`, `zone_hierarchy.rs:240..279,419..443`, while binary producer/consumer is confirmed via `decompile_function 0x0042C290,0x0042CF80,0x0042D830` - STALE). | `src/sim/pathfinding/zone_search.rs`; `src/sim/pathfinding/zone_hierarchy.rs`; retry state | Wire the binary failed-A* producer lifecycle before claiming retry parity; preserve per-edge, not per-zone, exclusion semantics. | Graph `A-B-C` plus `A-D-C`; exclude `A-B`; retry may still traverse `B` through another edge if graph permits, and should not globally ban zone `B`. | Do not turn a failed chain into a zone ban or expanded corridor unless separately verified. |
| Stored path chain is start-to-destination order in `+0xBC + level*1000`. | `0x0042C887..0x0042C8CE` | Stale: current `zone_precheck_flat` returns per-level start-to-destination `paths` and marker sets; the older "one corridor vec" claim now applies only to legacy fallback, not the hierarchy scaffold (corrected 2026-06-01: was unchecked/missing and "current Rust returns one corridor vec"; source scan shows `zone_hierarchy.rs:285..367,460..474` and `zone_search.rs:287..304`, while binary chain order is confirmed via `decompile_function 0x0042C290` and `get_assembly_context 0x0042C887..0x0042C8CE` - STALE). | `src/sim/pathfinding/zone_hierarchy.rs`; `src/sim/pathfinding/zone_search.rs` | Keep selected zone chain order stable because cell A* consumes the next-zone cursor from that order; verify stock route consumption against binary. | Multi-level fixture should expose chosen chain exactly as first-discovered path at each level. | Do not reverse chain order or reconstruct using a sorted predecessor map. |
| Lower levels are constrained by next-coarser chosen markers except neighbor zone type `1`. | `0x0042C5F0..0x0042C604` | Stale: `zone_precheck_flat` now implements parent-marked gating and the zone type `1` exception; remaining deltas are input/writer fidelity, integration gating, and slope/retry producer details, not a missing parent gate (corrected 2026-06-01: was "missing: current Rust is single-level plus one-ring expansion"; source scan shows `zone_hierarchy.rs:349..364,419..443,678..756`, while binary parent gate/type-1 exception is confirmed via `decompile_function 0x0042C290` and `get_assembly_context 0x0042C5F0..0x0042C604` - STALE). | `src/sim/pathfinding/zone_hierarchy.rs::zone_precheck_flat`; `src/sim/pathfinding/zone_search.rs` integration | Preserve parent-chain gating before claiming parity; ensure hierarchy input records match the binary writer. | Fine-level off-corridor edge should be pruned unless its type is `1`. | Do not approximate this with connected-component reachability alone. |

### Negative Facts / Do Not Do

- Do not sort `ZoneAdjacency` where the order can feed `Zone_precheck` tie behavior. Active in YR: Yes; evidence `0x0042C501..0x0042C540`, strict equality behavior at `0x0042C5DE..0x0042C6D8`.
- Do not let `ZoneId` decide equal-cost route ties. Active in YR: Yes; no ZoneId compare exists in candidate replacement or heap movement.
- Do not use `BinaryHeap<Reverse<(cost, ZoneId)>>` for a parity `Zone_precheck`. Active in YR: Yes; binary heap nodes compare only float cost and preserve insertion behavior on equality.
- Do not use centroid Manhattan, `g+h`, or destination heuristic for binary `Zone_precheck`. Active in YR: Yes; evidence `0x0042C55C..0x0042C5D2`.
- Do not model retry failure as banning whole zones. Active in YR: Yes; exclusion keys are undirected edges, evidence `0x0042C620..0x0042C664`.
- Do not treat `byte(edge+4) != 0` as a proven "bridge edge" label in this report. Active in YR: Conditional as a nonzero edge tiebreak flag; this slice only verifies its `0.001` cost effect.

### Stale Docs / Follow-up Docs

- `BRIDGE_PATH_TIE_ORDER_AFTER_LOW_COLLAPSE_GHIDRA_REPORT.md` remains directionally correct for `Zone_precheck` ties. This report strengthens it with focused assembly evidence.
- `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` remains valid for consumer behavior. This report narrows the implementation handoff around insertion order and the `ZoneId` tie hazard.

## Sources

- Ghidra decompiled: `0x0042C290`, `0x0042C900`, `0x0042D170`, `0x0042CF80`, `0x0042D830`, `0x0042DCA0`.
- Ghidra assembly contexts: `0x0042C501`, `0x0042C53E`, `0x0042C5CD`, `0x0042C5E1`, `0x0042C5EA`, `0x0042C5F0`, `0x0042C618`, `0x0042C6A4`, `0x0042C6B3`, `0x0042C6D8`, `0x0042C7B0`, `0x0042C810`, `0x0042C835`, `0x0042C865`, `0x0042C887`, `0x0042C8BF`, `0x0042C8D6`, `0x0042CB58`, `0x0042CCB3`, `0x0042D222`.
- Prior docs referenced: `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`, `BRIDGE_PATH_TIE_ORDER_AFTER_LOW_COLLAPSE_GHIDRA_REPORT.md`, `BRIDGE_ZONE_INCREMENTAL_REFRESH_FUN_00584550_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust surfaces scanned: `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_hierarchy.rs`.
