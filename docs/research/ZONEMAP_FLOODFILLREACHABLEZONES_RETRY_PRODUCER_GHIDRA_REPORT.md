# ZoneMap FloodFillReachableZones Retry Producer - Ghidra Research Report

**Address(es):** `0x005840C0` (`ZoneMap__FloodFillReachableZones`), direct caller `0x0042CCD0` (`PathfinderClass__UpdateHierarchicalEdges`), nonzero-result consumer `0x0042CF80` (`PathfinderClass__InvalidateZoneEdge`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact `0x005840C0` retry-producer semantics needed by `PathfinderClass__UpdateHierarchicalEdges`: return polarity, output vector contents, movement-zone/passability gates, block bounds, visited mask, and neighbor-zone collection.  
**Non-Scope:** full A* retry loop, full `Zone_precheck`, full `InvalidateZoneEdge` common-neighbor behavior beyond caller polarity, persistent zone rebuild, stock-map route capture, and concrete locomotor-specific `Can_Enter_Cell` internals.  
**Confidence:** High for branch polarity, vector contents, bounds, and caller handoff; Medium for naming/intent because prior prose and the function name are misleading relative to the raw branch polarity.  
**Active in YR:** Yes. `AStar_pathfind_search @ 0x0042C900` reaches `UpdateHierarchicalEdges @ 0x0042CCD0` after failed hierarchical cell A*, and `0x0042CCD0` is the verified direct caller of `0x005840C0` in this slice.

## 0. Investigation Contract

Target question: Re-check `ZoneMap__FloodFillReachableZones @ 0x005840C0` for exact retry-producer semantics used by `PathfinderClass__UpdateHierarchicalEdges`: return value polarity, collected zone contents, passability/movement-zone gates, block bounds, visited mask, and how neighbor zones are collected.

Non-goals: Do not re-investigate unrelated pathfinder systems, full cell A*, full `Zone_precheck`, full `InvalidateZoneEdge` common-neighbor semantics, persistent zone rebuild, stock-map route winners, or locomotor-specific `Can_Enter_Cell` bodies.

Evidence needed to mark COMPLETE: direct Ghidra decompile and assembly context for `0x005840C0`; direct caller branch proof in `0x0042CCD0`; return-tail proof for `0` and `1`; branch proof for `Can_Enter_Cell` and `ZonePassabilityMatrix` polarity; proof of caller-vector contents and local duplicate filtering; Rust-facing scan sufficient to name the missing producer surface.

Stop conditions: stop once the exact helper contract and caller consumption are proven; stop before implementing Rust or editing in-repo docs; stop before route-frequency/runtime incidence claims; stop before full virtual-target audit of `Can_Enter_Cell`.

## 1. Overview

`ZoneMap__FloodFillReachableZones @ 0x005840C0` is a retry-local helper for `PathfinderClass__UpdateHierarchicalEdges`, not a persistent zone rebuild. It performs a block-local flood from the pathfinder current cell at one hierarchy level, then either reports a split inside the current hierarchy zone or returns graph-neighbor zones absent from the local flood observation.

The load-bearing correction from this re-check is the gate polarity. The local flood bookkeeping is reached when `Can_Enter_Cell` returns zero or the movement-zone matrix value is not `1`; when `Can_Enter_Cell` is nonzero and `ZonePassabilityMatrix[MovementZone][CellClass+0x4C] == 1`, execution jumps past same-zone push and different-zone local collection for that neighbor. Active in YR: Yes; evidence `0x00584271..0x00584286`.

## 2. Key Offsets / Data

| Owner | Offset / address | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `MapClass` singleton | `0x0087F7E8` | `this` for `0x005840C0` in caller. | `0x0042CD7B..0x0042CD80` | Yes |
| `MapClass+0x6C` | int | cell-count upper bound; linear zone-index reads clamp into `[0, count-1]`. | decompile `0x005840C0`; `0x005843D0..0x005843F9` | Yes |
| `MapClass+0x70` / `DAT_0087F858` | `u16[cell][5]` | per-cell hierarchy-zone ids; level slot read as `(level + cell_index * 5) * 2`. | decompile `0x005840C0`; caller `0x0042CD94..0x0042CDAC` | Yes |
| `CellClass+0x24/+0x26` | cell coordinate | seed and neighbor coordinates for masks and playfield/index checks. | `0x00584357..0x0058444D` | Yes |
| `CellClass+0x4C` | int | reduced zone-type column for matrix lookup. | `0x0058427B..0x00584286` | Yes |
| `CellClass+0x11B` | byte | height/level byte passed to mover `Can_Enter_Cell`. | `0x00584261..0x00584271` | Yes |
| `TechnoTypeClass+0x5B4` | int | `MovementZone` row; obtained through mover vtable `+0x84`. | `0x005841B4..0x005841C9` | Yes |
| `0x0082A594` | `int[13][8]` | `ZonePassabilityMatrix`; this helper compares selected entry to `1`. | `0x005841C6..0x005841C9`, `0x00584282` | Yes |
| `DAT_00ABDE48` | byte grid | visited mask; addressed as `(x & mask) * 8 + (y & mask)`. | seed mark in decompile; neighbor mark `0x005842AB` | Yes |
| `DAT_00ABDA68` | cell pointer stack | local flood stack; directions `0..7` are expanded for each popped cell. | `0x005841ED..0x00584216` | Yes |
| caller vector | `param_4` to `0x005840C0` | receives graph-neighbor zone ids not present in the local observed-different-zone vector. | `0x00584467..0x0058451B` | Yes |

## 3. Core Logic

### 3.1 Direct caller and level loop

Active in YR: Yes. `UpdateHierarchicalEdges @ 0x0042CCD0` loops exactly levels `0..2`: `INC EBP`, `ADD ESI,0x18`, `CMP EBP,0x3`, `JL 0x0042CD02` at `0x0042CE83..0x0042CE93`. For each level it calls `0x005840C0` at `0x0042CD80`, tests `AL` at `0x0042CD85`, jumps to zero-return handling at `0x0042CDD8` when `AL == 0`, and calls `InvalidateZoneEdge @ 0x0042CF80` at `0x0042CDAC` when `AL != 0`.

### 3.2 Block size, visited mask, and stack

Active in YR: Yes. The helper computes `block_size = 1 << (level + 1)`. With live levels `0..2`, the only block sizes are `2`, `4`, and `8`.

The visited mask uses fixed row stride `8`, not `block_size`: seed and neighbor visited bytes are addressed as `(x & (block_size - 1)) * 8 + (y & (block_size - 1))` into `DAT_00ABDE48`. The seed cell pointer is pushed to `DAT_00ABDA68`, marked visited, then the helper pops stack entries and expands directions `0..7` through `Pathfinding_update_continued @ 0x00481810`. Evidence: `0x0058418A..0x00584216`, mark/push `0x00584293..0x005842B8`.

### 3.3 Movement-zone and passability gate polarity

Active in YR: Yes. The row is `MovementZone`, not `SpeedType`: the helper calls mover vtable `+0x84`, reads `+0x5B4`, shifts by `0x20`, and adds matrix base `0x82A594` at `0x005841B4..0x005841C9`.

For each neighbor, it calls mover vtable `+0x1AC` as `Can_Enter_Cell(neighbor, direction, neighbor.CellClass+0x11B, 0, 1)` at `0x00584261..0x00584271`. The exact branch is:

- `TEST EAX,EAX; JZ 0x0058428C`: if `Can_Enter_Cell` returns zero, go to local flood bookkeeping.
- if nonzero, read `CellClass+0x4C` and compare `matrix[row][column]` to `1`.
- `CMP ...,0x1; JZ 0x00584339`: if the matrix entry is `1`, skip local flood bookkeeping and continue to the next direction.
- therefore same-zone push / different-zone local collection runs only when `Can_Enter_Cell == 0` or `matrix != 1`.

This corrects prior prose that described the local flood as accepting only `Can_Enter_Cell != 0 && matrix == 1`. The raw assembly proves the inverse bookkeeping condition for this helper. Do not infer a global `Can_Enter_Cell` meaning from this report; only this callsite polarity is claimed.

### 3.4 Local observed-different-zone vector

Active in YR: Yes. In the local bookkeeping branch:

- If neighbor zone id equals seed/current zone and the visited byte is clear, the helper marks that block-local byte and pushes the neighbor cell pointer to the local stack (`0x0058428C..0x005842B8`).
- If neighbor zone id differs from the seed zone, equals neither the last observed different zone nor zero, it scans a local `u16` vector backward for duplicates before appending (`0x005842C2..0x00584335`).
- Zone id `0` is skipped by `TEST BX,BX; JZ 0x00584339` at `0x005842C9..0x005842CC`.

The local different-zone vector is not itself the caller output. It is used later as the "locally observed" filter when selecting which graph neighbors to return to `UpdateHierarchicalEdges`.

### 3.5 Return `1`: same-zone block split found

Active in YR: Yes. After the stack flood completes, the helper scans the whole `block_size x block_size` local window. Candidate cells are playfield-gated by `MapClass__Is_Cell_In_Playfield(coord, 1)` at `0x005843AF..0x005843C8`. In-playfield candidates compute a linear index using map dimensions, clamp to `[0, MapClass+0x6C-1]`, and read the same hierarchy level zone id (`0x005843D0..0x00584410`).

If a candidate has the seed zone id and its visited byte is still zero, `JZ 0x0058451E` reaches the return-`1` tail `MOV AL,0x1; RET 0x10` at `0x0058453F..0x00584548`.

Caller interpretation: `AL != 0` makes `UpdateHierarchicalEdges` call `InvalidateZoneEdge(current_zone, level)` at `0x0042CDAC`. Return `1` is not route success. It is the same-zone-split branch for retry-local stored-path invalidation.

### 3.6 Return `0`: caller vector contains graph neighbors absent from local observed vector

Active in YR: Yes. If no unvisited same-zone cell is found, the helper reads the seed zone's hierarchy graph record for the current level and iterates the graph edge list backward (`0x00584467..0x00584485`). For each graph neighbor, it scans the local observed-different-zone vector backward (`0x00584499..0x005844AA`). Only when the graph neighbor is absent from that local vector does the helper append it to the caller vector (`0x005844AF..0x005844DB`). It then returns `0` at `0x00584512..0x0058451B`.

Caller interpretation: `AL == 0` makes `UpdateHierarchicalEdges` iterate the caller vector backward and append sorted undirected exclusions between the current zone and each returned graph-neighbor zone (`0x0042CDD8..0x0042CE4E`). It skips self-pairs at `0x0042CE02..0x0042CE04`, sorts endpoints at `0x0042CE06..0x0042CE11`, and writes packed `min << 16 | max` at `0x0042CE4E`.

This means the zero-return output is not "all adjacent different zones". It is graph neighbors of the current/seed zone that were not observed in the local different-zone vector during the flood.

## 4. INI Keys / Defaults

No INI key is read directly by `0x005840C0`.

| Key / data | Binary field | Effect | Evidence | Active in YR |
|---|---|---|---|---|
| `MovementZone=` | `TechnoTypeClass+0x5B4` | selects the 13-row matrix row used by this helper. | parser and matrix reader reports; direct read `0x005841B4..0x005841C9` | Yes |
| `SpeedType=` | `TechnoTypeClass+0x67C` | not used as the matrix row in this helper. | no read in `0x005840C0`; matrix reader report | Yes for parser, No for this helper's matrix row |
| `ZonePassabilityMatrix` | `0x0082A594` | value `1` is the branch value that skips local flood bookkeeping when `Can_Enter_Cell` is nonzero. | `0x00584282..0x00584286` | Yes |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Failed hierarchical A* retry | `AStar_pathfind_search` calls `UpdateHierarchicalEdges`, then `Reset`, then may rerun `Zone_precheck`. | prior retry reports; caller chain into `0x0042CD80` | Yes |
| Direct `0x005840C0` caller | `UpdateHierarchicalEdges` calls the helper once per level `0..2`. | `0x0042CD80`, `0x0042CE83..0x0042CE93` | Yes |
| Nonzero result | caller calls `InvalidateZoneEdge(current_zone, level)`. | `0x0042CD85..0x0042CDAC` | Yes |
| Zero result | caller appends current-zone to returned graph-neighbor exclusions. | `0x0042CDD8..0x0042CE4E` | Yes |
| Persistent rebuild separation | helper writes temp visited/stack/vector state and caller-local Pathfinder exclusions; no global graph rebuild call. | decompile `0x005840C0`, `0x0042CCD0` | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current status vs this slice |
|---|---|
| `src/sim/pathfinding/zone_hierarchy.rs` | Has `ZonePrecheckResult.paths`, `marked`, and per-level `ZonePrecheckExclusions`; comments still say exact failed-A* producer is deferred. Necessary consumer scaffolding exists, producer does not. |
| `src/sim/pathfinding/zone_search.rs` | The hierarchy branch runs one `zone_precheck_flat` with default empty exclusions and marker-gated A*. The compatibility retry loop excludes Rust corridor edges, not this block-local flood and graph-neighbor-missing producer. |
| `src/sim/pathfinding/core.rs` | Has `HierarchyGate` and `BlockerNeighborCounts` for marker gate; no retry-local `FloodFillReachableZones` equivalent. |
| Needed Rust surface | A producer that, per level, can read current cell level-zone id, level graph adjacency, level cell-zone ids, mover movement-zone/matrix data, and a `Can_Enter_Cell`-equivalent result, then either call stored-path invalidation on return `1` or append only current-zone to returned graph-neighbor exclusions on return `0`. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x005840C0` primary body | verified | decompile plus assembly contexts `0x005841B4..0x00584548` | none for scoped helper |
| direct caller `0x0042CCD0` | verified | decompile and assembly `0x0042CD80..0x0042CE93` | no full retry-loop restatement |
| return `1` polarity | verified | `0x00584418..0x00584548`, caller `0x0042CD85..0x0042CDAC` | runtime frequency |
| return `0` polarity | verified | `0x00584467..0x0058451B`, caller `0x0042CDD8..0x0042CE4E` | runtime frequency |
| matrix / `Can_Enter_Cell` branch polarity | verified | `0x00584271..0x00584286` | virtual target meanings deferred |
| local observed-different-zone vector | verified | `0x005842C2..0x00584335` | semantic name only inferred |
| graph-neighbor output vector | verified | `0x00584467..0x005844DB` | none |
| block size and visited mask | verified | decompile; assembly `0x0058418A..0x005842B8` | none |
| current Rust scan | verified read-only | Codegraph plus targeted reads of `zone_hierarchy.rs`, `zone_search.rs`, `core.rs` | no implementation performed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is this exhaustive-slice or coverage-map? -> Exhaustive-slice for `0x005840C0` retry-producer semantics only.` (evidence: user target and bounded direct caller)
- `[RESOLVED] OQ-2 - Is `0x005840C0` active in YR? -> Yes, direct retry caller `0x0042CCD0` is reached from normal failed hierarchical A* retry.` (evidence: `0x0042CD80`; prior retry call-chain reports)
- `[RESOLVED] OQ-3 - What are live levels and block sizes? -> Caller loops levels `0..2`, giving block sizes `2,4,8`.` (evidence: `0x0042CE83..0x0042CE93`; `0x005840C0` shift)
- `[RESOLVED] OQ-4 - What is return `1`? -> An in-playfield same-zone cell inside the block remains unvisited after the local flood; caller invalidates a stored path edge.` (evidence: `0x00584418..0x00584548`, `0x0042CD85..0x0042CDAC`)
- `[RESOLVED] OQ-5 - What is return `0`? -> No same-zone split found; helper returns graph neighbors absent from local observed-different-zone vector; caller excludes current-zone to those returned zones.` (evidence: `0x00584467..0x0058451B`, `0x0042CDD8..0x0042CE4E`)
- `[RESOLVED] OQ-6 - Which matrix row/column? -> row is `MovementZone` from `TechnoType+0x5B4`; column is `CellClass+0x4C`.` (evidence: `0x005841B4..0x005841C9`, `0x0058427B..0x00584282`)
- `[RESOLVED] OQ-7 - What is the branch polarity for passability? -> local bookkeeping runs on `Can_Enter_Cell == 0 || matrix != 1`; `Can_Enter_Cell != 0 && matrix == 1` skips it.` (evidence: `0x00584271..0x00584286`)
- `[RESOLVED] OQ-8 - Is zone id `0` locally collected? -> No, the local different-zone path tests zero and skips it.` (evidence: `0x005842C9..0x005842CC`)
- `[RESOLVED] OQ-9 - Does this mutate global hierarchy data? -> No observed global graph mutation; caller appends Pathfinder-local exclusions or calls `InvalidateZoneEdge`.` (evidence: `0x005840C0`, `0x0042CCD0`)
- `[RESOLVED] OQ-10 - Does Rust already have this producer? -> No; current Rust has consumer/exclusion scaffolding and marker gate, but not this exact producer.` (evidence: `zone_hierarchy.rs`, `zone_search.rs`, `core.rs`)
- `[DEFERRED] OQ-11 - What does each concrete locomotor return at `Can_Enter_Cell` vtable `+0x1AC`?` (category: out-of-scope; reason: this slice proves callsite polarity, not virtual target internals; next-step-if-pursued: locomotor-specific `Can_Enter_Cell` audit)
- `[DEFERRED] OQ-12 - Runtime frequency of return `0` vs return `1` in normal skirmish.` (category: needs-runtime-debugger; reason: static proof does not measure incidence; next-step-if-pursued: instrument failed hierarchy retries)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| `0x005840C0` return `1` means same-level same-zone block split found; `UpdateHierarchicalEdges` then calls stored-path `InvalidateZoneEdge`, not zero-return edge list append. | `0x00584418..0x00584548`; caller `0x0042CD85..0x0042CDAC`; Active in YR: Yes. | missing exact producer | `zone_search.rs`, `zone_hierarchy.rs`, future producer helper | Add a per-level split detector before choosing stored-path invalidation vs returned-neighbor exclusions. | A level-0 block has same-zone cells where one remains unvisited by the helper; retry invalidates adjacent stored path edge instead of excluding current-zone graph neighbors. | `zone_retry_split_result_invalidates_stored_path_edge` | Do not treat return `1` as success or persistent zone rebuild. |
| `0x005840C0` return `0` outputs graph neighbors of current zone that were absent from the local observed-different-zone vector; caller excludes only current-zone to those returned graph neighbors. | `0x00584467..0x0058451B`; caller append `0x0042CDD8..0x0042CE4E`; Active in YR: Yes. | Rust derives retry exclusions from failed corridor edges | retry producer and `ZonePrecheckExclusions` feed | Produce canonical undirected per-level exclusions from current zone to returned graph neighbors, not from whole corridor or whole zones. | Graph neighbors B and C exist; local observed vector contains B only; return `0` appends only current-C, and route may still use unrelated B-D. | `zone_retry_zero_result_excludes_unobserved_graph_neighbors_only` | Do not append all locally observed different zones or ban zones. |
| Local flood bookkeeping is gated by `Can_Enter_Cell == 0 || matrix != 1`; the `Can_Enter_Cell != 0 && matrix == 1` case skips same-zone push/different-zone local collection. | `0x00584271..0x00584286`; Active in YR: Yes. | no equivalent producer; risk if built from ordinary passable-neighbor flood | producer cell-entry/matrix adapter | Model this helper's exact branch polarity separately from normal path expansion semantics. | A neighbor with matrix value `1` and nonzero cell-entry result does not mark visited or enter local observed-different-zone set; a matrix-blocked or zero-return neighbor does. | `zone_retry_flood_bookkeeping_uses_inverse_passability_branch` | Do not reuse normal A* "can step" predicate as the flood expansion predicate without this polarity. |

## 10. Negative Facts / Do Not Do

- Do not describe the helper output on return `0` as "all locally collected different zones." Active in YR: Yes; local observed zones are used as a filter, and the caller vector receives graph neighbors absent from that local vector (`0x00584467..0x005844DB`).
- Do not state that same-zone flood push requires `Can_Enter_Cell != 0 && matrix == 1`. Active in YR: Yes; assembly shows that pair jumps to `0x00584339`, skipping local bookkeeping (`0x00584271..0x00584286`).
- Do not treat return `1` as route success. Active in YR: Yes; caller invokes `InvalidateZoneEdge` on nonzero result (`0x0042CD85..0x0042CDAC`).
- Do not append zone id `0` from the local different-zone path. Active in YR: Yes; `TEST BX,BX; JZ` skips it (`0x005842C9..0x005842CC`).
- Do not rebuild or mutate the global zone graph as the retry analogue. Active in YR: Yes; this path feeds Pathfinder-local retry exclusions/stored-path invalidation, not persistent `MapClass` rebuild.

## 11. Remaining Uncertainty

- Concrete `Can_Enter_Cell` return-code meanings for each locomotor remain outside this report. The callsite branch polarity is verified, but virtual target semantics are not generalized.
- Runtime frequency of the return `0` and return `1` branches in common YR situations needs debugger or replay instrumentation.
- The function's name suggests reachable-zone semantics, but the verified branch polarity is best documented as exact control flow rather than renamed intent.

## 12. Stale Docs / Follow-up Wording

- Replace prior wording in `MAPCLASS_FLOODFILLREACHABLEZONES_005840C0_GHIDRA_REPORT.md` that says neighbor expansion is accepted only when `Can_Enter_Cell` is nonzero and matrix value is `1` with:
  > In `ZoneMap__FloodFillReachableZones @ 0x005840C0`, local flood bookkeeping runs when `Can_Enter_Cell` returns zero or `ZonePassabilityMatrix[MovementZone][CellClass+0x4C] != 1`. The `Can_Enter_Cell != 0 && matrix == 1` case jumps to the next direction without marking same-zone visited state or adding a local different-zone observation.
- Replace prior wording that says return `0` appends collected different/graph neighbor zones with:
  > On return `0`, the helper appends to the caller vector only graph neighbors of the seed/current zone that were absent from the local observed-different-zone vector. `UpdateHierarchicalEdges` then excludes current-zone to those returned graph neighbors.
- For `UPDATEHIERARCHICALEDGES_FAILED_ASTAR_EDGE_SELECTION_GHIDRA_REPORT.md`, tighten any statement that "`FloodFillReachableZones` returns collected different zones" to:
  > The zero-return caller vector contains graph-neighbor zones not locally observed by `0x005840C0`; it is not the raw local different-zone vector.

## Sources

- Ghidra decompiled this slot: `0x005840C0`, `0x0042CCD0`, `0x0042CF80`, `0x00481810`.
- Ghidra assembly contexts: `0x005841B4..0x005841C9`, `0x00584271..0x00584286`, `0x0058428C..0x00584335`, `0x00584357..0x00584548`, `0x00584467..0x0058451B`, `0x0042CD80..0x0042CE4E`, `0x0042CE83..0x0042CE93`, `0x0042CF8D..0x0042D13C`.
- Prior docs checked: `MAPCLASS_FLOODFILLREACHABLEZONES_005840C0_GHIDRA_REPORT.md`, `UPDATEHIERARCHICALEDGES_FAILED_ASTAR_EDGE_SELECTION_GHIDRA_REPORT.md`, `UPDATE_HIERARCHICAL_EDGES_RETRY_PRODUCER_SCOPE_GHIDRA_REPORT.md`, `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`.
- Rust scanned read-only: `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core.rs`.

Status: COMPLETE.
