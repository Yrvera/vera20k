# MapClass FloodFillReachableZones 0x005840C0 - Ghidra Research Report

**Address(es):** `0x005840C0` primary; direct caller `0x0042CCD0`; direct callee path `0x00481810`, `0x0056D430`, `0x00578460`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `MapClass::FloodFillReachableZones @ 0x005840C0` result semantics, matrix reader behavior, and its relationship to persistent zone rebuild versus A* retry-local edge exclusions.  
**Non-Scope:** full `Zone_precheck` graph search, full persistent `0x0056C510` rebuild algorithm, all `CellClass+0x4C` writer branches, locomotor-specific `Can_Enter_Cell` internals, and runtime frequency measurement.  
**Confidence:** High for scoped binary behavior; Medium for Rust delta because no code was changed or tested.  
**Active in YR:** Yes. The single direct caller is `AStar_pathfind_search -> PathfinderClass__UpdateHierarchicalEdges` on the normal foot pathfinding retry path.

## Target Question

What does `MapClass::FloodFillReachableZones @ 0x005840C0` return, what passability row/column does it read, and is it part of persistent zone rebuild or A* retry-local exclusion repair?

## Non-Goals

- Do not re-document all `AStar_pathfind_search` retry semantics already covered by `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`.
- Do not re-document `0x0056C510` persistent rebuild beyond the separation needed for this function.
- Do not infer every `Can_Enter_Cell` result code; this function only tests zero versus nonzero.
- Do not modify Rust, INI, or in-repo docs.

## Evidence Needed To Mark COMPLETE

- Decompile and assembly/caller evidence for `0x005840C0` and its direct caller. Complete: decompile `0x005840C0`, `0x0042CCD0`; direct CALL scan found only `0x0042CD80`.
- Return-value proof. Complete: assembly tails `0x00584512..0x0058451B` return `0`, `0x0058453F..0x00584548` return `1`; caller branches at `0x0042CD85..0x0042CDD8`.
- Matrix row/column proof. Complete: decompile `0x005840C0`, assembly `0x005841B4..0x005841C9`, `0x0058427B..0x00584286`.
- Bounds/window proof. Complete: decompile `0x005840C0`; assembly `0x00584357..0x0058444D`, `0x005843D0..0x0058441C`.
- Separation from persistent rebuild. Complete: direct caller is `0x0042CCD0`; zero-return path appends to Pathfinder local vectors, nonzero path calls `0x0042CF80`; no call to `0x0056C510`.

## Stop Conditions

- Stop at direct caller/callees needed to explain result and retry implications.
- Stop before locomotor-specific `Can_Enter_Cell` internals; only the zero/nonzero gate matters here.
- Stop before global hierarchy build adjacency ordering; sibling report covers emission order.
- Stop before runtime frequency claims; static evidence proves liveness and branch behavior, not common-case rate.

## 1. Overview

`0x005840C0` is a retry-local hierarchy repair helper. Starting from a current cell and hierarchy level, it flood-fills within one fixed-size wrapped block and tests whether any cell with the same hierarchy-zone id remains unreached. If such an unreached same-zone cell exists, it returns `1`; otherwise it returns `0` after collecting neighboring different-zone ids for the caller.

The result does not mean "path found" or "zone reachable." In the live caller, return `1` selects `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80`; return `0` makes `0x0042CCD0` append sorted undirected retry-local edge exclusions from the collected zone vector.

## 2. Class Layout / Key Offsets

| Owner | Offset / address | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `MapClass` singleton | `0x0087F7E8` | `this` passed to `0x005840C0` by caller. | `0x0042CD7B..0x0042CD80` sets `ECX=0x87F7E8` before CALL. | Yes |
| `MapClass` | `+0x6C` | cell-count upper bound for clamping linear indices. | `0x005840C0` decompile; index clamps before `+0x70` reads. | Yes |
| `MapClass` | `+0x70` / `DAT_0087F858` | per-cell hierarchy zone ids, five `u16` entries per cell; this helper reads `level 0..2` slots. | `0x005840C0` decompile reads `*(u16 *)(+0x70 + (level + cell*5)*2)`. | Yes |
| `CellClass` | `+0x24/+0x26` | cell coordinate used as flood seed and window origin. | `0x005840C0` decompile; `0x0042CD75` supplies current CellClass. | Yes |
| `CellClass` | `+0x4C` | reduced zone type column for `ZonePassabilityMatrix`. | `0x0058427B..0x00584286`. | Yes |
| `CellClass` | `+0x11B` | level/height byte passed to vtable `Can_Enter_Cell`. | `0x00584261..0x00584271`. | Yes |
| `TechnoTypeClass` | `+0x5B4` | `MovementZone` row for matrix lookup. | `0x005841B4..0x005841C9`; caller passes mover object. | Yes |
| Global | `0x0082A594` | `int[13][8] ZonePassabilityMatrix`; only value `1` passes. | `0x005841C9` row base; `0x00584282..0x00584286` compare `== 1`. | Yes |
| Temp byte grid | `DAT_00ABDE48` | visited bitmap sized `(1 << (level+1))` square, addressed with coordinate masks. | `0x005840C0` decompile and start assembly. | Yes |
| Temp cell stack | `DAT_00ABDA68` | local flood stack of `CellClass*`. | `0x005841ED..0x005841FF`. | Yes |
| Caller vector | stack vector passed by `0x0042CCD0` | receives different adjacent zone ids plus graph neighbors not locally seen. | `0x0042CD58..0x0042CD80`, `0x005844AF..0x005844E7`. | Yes |

## 3. Core Logic

### Inputs and window size

The direct caller pushes, in order, mover object, temp `u16` vector, hierarchy level, current cell coordinate pointer, then pushes the current `CellClass*` result and calls with `ECX=0x87F7E8`. Evidence: `0x0042CD54..0x0042CD80`.

The helper computes `block_size = 1 << (level + 1)` and clears a byte visited grid at `DAT_00ABDE48`. Because the live caller loops levels `0..2`, block sizes are `2`, `4`, and `8`. Active in YR: Yes; evidence: `0x0042CE83..0x0042CE93` caller loop and `0x005840C0` start decompile.

The seed cell's linear index is clamped to `[0, MapClass+0x6C - 1]`, then the seed hierarchy zone id is read from `MapClass+0x70 + (level + index*5)*2`. Clamping happens before both seed and neighbor hierarchy reads. Active in YR: Yes.

### Flood expansion

The function pushes the seed cell into `DAT_00ABDA68` and marks visited at:

`((seed.x & (block_size - 1)) * 8) + (seed.y & (block_size - 1))`

The stride is hardcoded `8`, not `block_size`. This is safe for live levels because the maximum block size is `8`. Active in YR: Yes.

For each popped cell it visits 8 neighbors through `Pathfinding_update_continued @ 0x00481810`, which adds `g_DirectionOffsets[dir]` and returns `MapClass::Get_CellClass` for that neighbor. Evidence: `0x00584215..0x00584216`, decompile `0x00481810`. Active in YR: Yes.

Each neighbor is accepted into the local flood only when all of these hold:

1. `Can_Enter_Cell(neighbor, dir, neighbor+0x11B, 0, 1)` returns nonzero.
2. `ZonePassabilityMatrix[mover.MovementZone][neighbor.CellClass+0x4C] == 1`.
3. The neighbor hierarchy zone id at the current level equals the seed zone id.
4. The masked visited byte is still zero.

The branch is inverted in decompile as "if CanEnter failed OR matrix is not 1, do not expand." Assembly shows the pass case at `0x00584277..0x00584286`: `TEST EAX,EAX; JZ ...; CMP matrix,1; JZ 0x00584339`. Active in YR: Yes.

If a neighbor has a different nonzero hierarchy zone id, it is added once to the caller vector. The helper remembers the last different zone in `uVar4` as a fast duplicate check, but still scans the existing vector backward before append. Zone id `0` is never appended from this neighbor path. Active in YR: Yes.

### Return value semantics

After the flood, the helper scans every coordinate in the same level block window. For each candidate:

- It constructs a coordinate by combining the seed origin with masked block offsets.
- It calls `MapClass::Is_Cell_In_Playfield(coord, 1)`.
- If in playfield, it computes a linear index with `(MapClass+0xF8 + 1 + MapClass+0xF4) * y + x`, clamps it to `[0, cell_count-1]`, and reads the same level hierarchy zone id.
- If that zone id equals the seed zone id and the corresponding visited byte is still zero, it returns `1`.

Evidence: `0x00584357..0x0058444D`; exact success tail `0x0058453F..0x00584548` sets `AL=1` and returns. Active in YR: Yes.

If no unreached same-zone in-playfield cell is found, the helper iterates the global hierarchy graph record for the seed zone at this level and appends any graph neighbor zone id that was not already in the locally collected different-zone vector. Then it returns `0`. Evidence: graph scan `0x00584467..0x005844F1`; zero tail `0x00584512..0x0058451B`. Active in YR: Yes.

### Caller interpretation

`PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0` is the only direct CALL-site found by local PE direct-call scan: `0x0042CD80`.

At `0x0042CD85..0x0042CD87`, caller tests `AL`. If nonzero, it gets the current zone id from `DAT_0087F858` and calls `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80` at `0x0042CDAC`. If zero, it walks the helper's vector backward and appends sorted packed pairs between the current zone and each collected different zone into Pathfinder local exclusion vectors. Evidence: decompile `0x0042CCD0`, assembly `0x0042CD80..0x0042CDAC`.

Therefore:

- Return `1` = same hierarchy-zone block has split under mover-constrained flood; invalidate an edge adjacent to the current zone in the stored precheck path.
- Return `0` = no same-zone split found in this block; append retry-local exclusions between current zone and collected adjacent graph/different zones.

## 4. Matrix Reader Behavior

`0x005840C0` reads the same `ZonePassabilityMatrix` contract as the other pathfinding zone readers:

- Row is `MovementZone` from the mover's type `+0x5B4`, not `SpeedType`.
- Column is reduced `CellClass+0x4C`, not raw terrain `LandType`.
- Only integer value `1` passes; values `2` and `3` reject the neighbor from local flood expansion.

Evidence: `0x005841B4..0x005841C9` calls mover vtable `+0x84`, reads type `+0x5B4`, multiplies row by `0x20`, and adds `0x82A594`; `0x0058427B..0x00584286` compares matrix column against `1`.

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Direct caller | Only direct CALL to `0x005840C0` is `0x0042CD80` in `0x0042CCD0`. | local PE direct-call scan; assembly context. | Yes |
| Retry path | `AStar_pathfind_search` calls `0x0042CCD0` after failed hierarchical-assisted `AStar_main_loop`, then resets and may rerun `Zone_precheck`. | `0x0042CC79..0x0042CCB8`; sibling retry report. | Yes |
| Persistent rebuild separation | This helper is not `0x0056C510` and does not rebuild `MapClass+0x18`, `+0x68`, `+0x70`, or `DAT_0087F878`. | decompile writes only temp grids/vector and caller Pathfinder vectors. | Yes |
| Neighbor cell entry | Uses mover vtable `+0x1AC` with args `(neighbor, dir, neighbor level byte, 0, 1)` and only tests zero/nonzero. | `0x00584261..0x00584277`. | Yes |
| Graph-neighbor supplement | Zero-return path appends hierarchy graph neighbors absent from local different-zone list. | `0x00584467..0x005844F1`. | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Observed current behavior | Delta versus this slice |
|---|---|---|
| `src/sim/pathfinding/zone_search.rs:283` | Maintains `excluded_edges: BTreeSet<ZoneEdge>` and excludes corridor edges after failed corridor A*. | Already closer to edge exclusions than old whole-zone wording, but still lacks this binary split detector that chooses between path-edge invalidation and current-zone-to-adjacent-zone exclusions. |
| `src/sim/pathfinding/zone_search.rs:289` | Expands corridor by one ring before cell A*. | No equivalent expansion was found in `0x005840C0`; binary uses block-local flood plus graph neighbor supplementation for retry repair. |
| `src/sim/pathfinding/zone_map.rs:217` | Builds `MovementZone::all_ground()` maps. | Persistent rebuild question is separate; `0x005840C0` is retry-local and cannot justify persistent row omissions or additions. |
| `src/sim/world/mod.rs:674` | `Simulation::rebuild_zone_grid` updates persistent zone connectivity from `PathGrid`. | Correct surface for persistent map changes, not the analogue of `0x005840C0` or `0x0042CCD0`. |
| `src/app_sim_tick.rs:775` | Rebuilds dynamic path grid and calls `sim.rebuild_zone_grid`. | App tick rebuild is persistent state; A* retry repair happens inside one search call in gamemd. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x005840C0` primary body | verified | decompile plus assembly contexts `0x005840C0`, `0x00584271`, `0x00584357`, `0x00584467`, return tails | none for scoped behavior |
| Return `1` branch | verified | `0x005843D0..0x0058441C` test; `0x0058453F..0x00584548` return tail; caller branch `0x0042CD85..0x0042CDAC` | runtime frequency |
| Return `0` branch | verified | graph supplement `0x00584467..0x005844F1`; zero tail `0x00584512..0x0058451B`; caller vector append decompile | none |
| Matrix row/column/value | verified | `0x005841B4..0x005841C9`, `0x0058427B..0x00584286`; matrix reader report | none |
| Direct caller inventory | verified | local PE direct-call scan found `0x0042CD80` only | indirect/control-flow exotic entries not proven |
| `Pathfinding_update_continued @ 0x00481810` | verified | decompile; 8-direction offset callee | exact direction ordering not restated |
| `MapClass__CellCoordToLinearIndex @ 0x0056D430` | verified | decompile | none |
| `MapClass__Is_Cell_In_Playfield @ 0x00578460` | touched-not-exhausted | decompile read enough to identify playfield gate | full playfield formula not in scope |
| `Can_Enter_Cell` virtual target | deferred | callsite only | locomotor-specific code requires separate target |
| Persistent rebuild `0x0056C510` | touched-not-exhausted | prior Ghidra reports; no direct call from this helper/caller | full rebuild details separate |
| Rust surfaces | verified-read-only | targeted line reads in `zone_search.rs`, `zone_map.rs`, `world/mod.rs`, `app_sim_tick.rs` | no implementation/test run |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is the investigation exhaustive-slice or coverage-map? -> Exhaustive-slice for `0x005840C0` plus direct caller/callees needed for semantics.` (evidence: user scope; decompile set)
- `[RESOLVED] OQ-2 - Is `0x005840C0` live in YR? -> Yes, direct caller `0x0042CCD0` is reached from live A* retry path.` (evidence: `0x0042CC79`, `0x0042CD80`)
- `[RESOLVED] OQ-3 - What is the direct caller set? -> Local direct CALL scan found only `0x0042CD80`.` (evidence: PE scan of retail `gamemd.exe`)
- `[RESOLVED] OQ-4 - What does return `1` mean? -> An in-playfield cell with same hierarchy zone id in the level block was not reached by the local flood.` (evidence: `0x00584357..0x00584548`)
- `[RESOLVED] OQ-5 - What does return `0` mean? -> No unreached same-zone cell was found; caller should use collected different/graph neighbor zones for edge exclusions.` (evidence: `0x00584467..0x0058451B`, `0x0042CDD8..0x0042CE5C`)
- `[RESOLVED] OQ-6 - Does it read `ZonePassabilityMatrix`? -> Yes, mover MovementZone row and `CellClass+0x4C` column; only `==1` passes.` (evidence: `0x005841B4..0x00584286`)
- `[RESOLVED] OQ-7 - Does it use `SpeedType` as row? -> No; row is TechnoType `+0x5B4` MovementZone.` (evidence: `0x005841B4..0x005841C9`; matrix reader report)
- `[RESOLVED] OQ-8 - What are the level bounds? -> Caller invokes exactly levels `0..2`, yielding block sizes `2,4,8`.` (evidence: `0x0042CE83..0x0042CE93`, `0x005840C0` start)
- `[RESOLVED] OQ-9 - How are out-of-range linear indices handled? -> Linear index reads clamp to `[0, MapClass+0x6C-1]`.` (evidence: `0x005840C0` decompile; `0x005843D0..0x005843F9`)
- `[RESOLVED] OQ-10 - Does zone id `0` get appended? -> Different-zone neighbor path explicitly requires nonzero zone id before append.` (evidence: `0x005840C0` decompile)
- `[RESOLVED] OQ-11 - Does it rebuild persistent zones? -> No; it writes temp state and feeds caller Pathfinder local exclusions/path invalidation.` (evidence: `0x0042CD80..0x0042CE5C`; no `0x0056C510` call)
- `[RESOLVED] OQ-12 - Rust persistent rebuild surface? -> `rebuild_dynamic_path_grid` calls `Simulation::rebuild_zone_grid`; this is separate persistent state.` (evidence: `src/app_sim_tick.rs:775`, `src/sim/world/mod.rs:674`)
- `[RESOLVED] OQ-13 - Rust retry surface? -> `zone_search.rs` has per-search `excluded_edges`, but not this exact block-local flood split detector.` (evidence: `src/sim/pathfinding/zone_search.rs:283`)
- `[DEFERRED] OQ-14 - Which concrete locomotor `Can_Enter_Cell` implementations are most important for this helper?` (category: out-of-scope; reason: caller/callee contract only needs zero/nonzero; next-step-if-pursued: locomotor-specific cell-entry audit)
- `[DEFERRED] OQ-15 - How often does return `1` trigger in ordinary skirmish?` (category: needs-runtime-debugger; reason: static binary proves behavior, not frequency; next-step-if-pursued: instrument failed hierarchical A* retries on bridge/dense-blocker maps)
- `[DEFERRED] OQ-16 - Are there computed/indirect entries to `0x005840C0`?` (category: bounded-cost-too-high; reason: direct-call scan found the live callsite and no vtable evidence; next-step-if-pursued: full Ghidra xref table if exposed)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Return `1` from `0x005840C0` means same hierarchy-zone cells inside the level block are not all reachable under mover `Can_Enter_Cell` plus matrix `==1`; caller invalidates a stored path edge, not a global zone. | `0x00584357..0x00584548`; caller `0x0042CD85..0x0042CDAC`. Active in YR: Yes. | missing/unchecked: Rust retry has edge exclusions but no block-local same-zone split detector choosing `InvalidateZoneEdge` behavior. | `src/sim/pathfinding/zone_search.rs`; future hierarchy/precheck adapter. | If exact hierarchy retry is implemented, first test block-local same-zone reachability; only the split case should invalidate the stored path-chain edge. | A level block contains two same-zone islands separated by a local dynamic blocker; failed A* retry invalidates the path-chain edge rather than excluding every adjacent current-zone edge. Proposed test name: `zone_retry_split_block_invalidates_stored_path_edge`. | Do not interpret return `1` as path success or persistent zone rebuild. |
| Return `0` means no unreached same-zone cell was found; the helper-supplied different/graph neighbor zones become current-zone undirected retry exclusions. | `0x00584467..0x0058451B`; `0x0042CDD8..0x0042CE5C`. Active in YR: Yes. | partial: Rust can exclude edges, but currently derives them from corridor failure rather than this collected zone list. | `src/sim/pathfinding/zone_search.rs::find_zone_corridor` retry state. | Exclude only canonical current-zone-to-collected-neighbor edges for the current search and preserve other routes through the same zones. | Local flood sees adjacent zones B and D from current zone A; retry excludes A-B and A-D only, while unrelated B-C remains usable. Proposed test name: `zone_retry_zero_result_excludes_collected_current_zone_edges_only`. | Do not rebuild `ZoneGrid` or ban whole zones. |
| Matrix gate inside the flood uses mover `MovementZone` row and reduced `CellClass+0x4C` column, accepting only value `1`; `Can_Enter_Cell==0` also blocks flood expansion. | `0x005841B4..0x00584286`, callsite `0x00584261..0x00584277`. Active in YR: Yes. | potential mismatch if Rust uses speed/terrain buckets or ignores cell-entry legality in retry repair. | `src/sim/pathfinding/passability.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_search.rs`. | The retry-local flood/repair layer must use the same binary-facing matrix and mover cell-entry legality as cell A*. | A Water mover and a Normal mover see different reachable sets in the same block because row 10 and row 0 differ; a blocked `Can_Enter_Cell` neighbor is not flooded even if matrix row permits it. Proposed test name: `zone_retry_flood_uses_movement_zone_matrix_and_cell_entry_gate`. | Do not use `SpeedType` or raw `LandType` as the matrix row/column. |

## Negative Facts / Do Not Do

- Do not treat `0x005840C0` as a persistent zone rebuild. Active in YR: Yes; evidence: only direct call is from `0x0042CCD0`, and caller writes Pathfinder-local vectors / calls `0x0042CF80`.
- Do not treat return `1` as "route exists." Active in YR: Yes; evidence: return `1` selects edge invalidation at `0x0042CDAC`.
- Do not treat return `0` as "unreachable, fail immediately." Active in YR: Yes; evidence: return `0` path appends retry-local exclusions for another precheck/search attempt.
- Do not append zone id `0` from the different-neighbor path. Active in YR: Yes; evidence: branch requires `uVar3 != current_zone` and `uVar3 != 0`.
- Do not use `SpeedType` as the passability matrix row or raw `LandType` as the column. Active in YR: Yes; evidence: `+0x5B4` and `CellClass+0x4C` direct reads.
- Do not call `rebuild_zone_grid` as the Rust analogue of this helper. Active in YR: Yes; evidence: binary retry helper does not call `0x0056C510` or mutate global hierarchy graph.

## Remaining Uncertainty

- Runtime frequency of return `1` versus return `0` in common YR situations needs debugger/instrumented replay evidence.
- Concrete virtual `Can_Enter_Cell` targets remain outside this report; only the zero/nonzero contract is verified here.
- Direct CALL inventory is verified by PE scan, but exotic computed jumps were not exhaustively proven absent.
- Rust delta should be rechecked after ongoing pathfinding changes settle; this report made no code edits.

## Stale Docs / Follow-up Docs

- `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md` deferred this function. Add/replace the deferred note with:
  > `MapClass::FloodFillReachableZones @ 0x005840C0` is the `0x0042CCD0` retry-local split detector. Return `1` means an in-playfield cell with the same hierarchy zone id in the level block was not reached by the local mover-constrained flood, so the caller invokes `PathfinderClass__InvalidateZoneEdge`. Return `0` means no same-zone split was found; the collected different/graph neighbor zones are appended as current-zone retry-local edge exclusions. The helper reads `ZonePassabilityMatrix[MovementZone][CellClass+0x4C] == 1` and is not a persistent zone rebuild.
- `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md` is mostly correct; add this precision:
  > The nonzero `FloodFillReachableZones` branch is not a generic "reachable" result. It specifically reports a same-hierarchy-zone cell inside the level block that remains unvisited after the local cell-entry/matrix-gated flood.

## Sources

- Ghidra decompile: `0x005840C0`, `0x0042CCD0`, `0x0042CF80`, `0x0042D830`, `0x0042C290`, `0x00481810`, `0x0056D430`, `0x00578460`.
- Ghidra assembly contexts: `0x0042CD54..0x0042CD80`, `0x0042CD85..0x0042CDAC`, `0x005840C0`, `0x005841B4..0x005841C9`, `0x00584215..0x00584286`, `0x00584357..0x0058444D`, `0x00584467..0x00584548`.
- Local direct-call scan of retail `gamemd.exe`: target `0x005840C0`, hit `0x0042CD80` only.
- Prior docs checked: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`, `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`, `ZONE_REBUILD_ASTAR_RETRY_HELPERS_0056C510_0042C290_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`.

Status: COMPLETE for the scoped exhaustive slice.
