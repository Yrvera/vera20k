# Pathfinder Failed A* Current Zone Source -- Ghidra Research Report

**Address(es):** `0x0042C900` (`AStar_pathfind_search`), `0x00429A90` (`AStar_main_loop`), `0x0042CCD0` (`PathfinderClass__UpdateHierarchicalEdges`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** identify exactly which cell/zone feeds `PathfinderClass__UpdateHierarchicalEdges` after a failed hierarchical A* attempt.  
**Non-Scope:** full `Zone_precheck`, full `ZoneMap__FloodFillReachableZones`, common-neighbor invalidation internals, A* edge costs, smoothing, path reconstruction, or Rust implementation changes.  
**Confidence:** High for the scoped field lifecycle and Rust handoff input.  
**Active in YR:** Yes. `AStar_pathfind_search` is the live standard pathfinding wrapper; `AStar_main_loop` and `UpdateHierarchicalEdges` have direct caller evidence from that function and no TS-only gate in the scoped path.

Target question: after `AStar_main_loop` returns 0 and `AStar_pathfind_search` calls `PathfinderClass__UpdateHierarchicalEdges`, does `UpdateHierarchicalEdges` use the start cell, current pathfinder cell, last expanded cell, destination-adjacent cell, or another stored field?

Non-goals: do not re-prove settled retry count, same-zone/cross-zone precheck behavior, exclusion vector packing, `FloodFillReachableZones` branch semantics, or `InvalidateZoneEdge` common-neighbor append details except where needed to identify the input cell.

Evidence needed to mark COMPLETE:

- Decompile plus assembly for the `Pathfinder+0x70` read in `UpdateHierarchicalEdges`.
- Decompile plus assembly for every scoped write to `Pathfinder+0x70` in `AStar_main_loop`.
- Caller/xref evidence that the live failed-A* path calls `UpdateHierarchicalEdges` after `AStar_main_loop` and before `Reset`.
- Reset evidence showing the field is not overwritten between failed A* and update.
- Rust-facing conclusion naming the exact cell state future retry producer must carry.

Stop conditions: stop once the `+0x70` lifecycle is resolved with address evidence, no Ghidra mutations are performed, no Rust/INI/in-repo docs are edited, this report is written, and the shared swarm claims row is updated.

## 1. Overview

`UpdateHierarchicalEdges` does not use a frontier node, last popped node, destination-adjacent cell, or an argument returned from A*. It reads `PathfinderClass+0x70`, a 4-byte cell coordinate maintained by `AStar_main_loop`.

That field starts as the A* start cell. During hierarchical A*, it is advanced only when a newly accepted neighbor cell enters the next level-0 zone in the stored `Zone_precheck` path. Therefore the failed-A* retry producer input is the furthest level-0 corridor-progress cell reached by generated/accepted neighbor nodes, falling back to the start cell if no next-zone crossing was accepted.

## 2. Class Layout / Key Offsets

| Owner | Offset | Type / shape | Purpose in this slice | Active in YR |
|---|---:|---|---|---|
| PathfinderClass | `+0x6C` | int | level-0 zone-path progress counter used by `AStar_main_loop`; initialized to 0 before the start node. | Yes; written at `0x00429BCD`, read at `0x0042A159`. |
| PathfinderClass | `+0x70` | `CellStruct` packed as two shorts / dword | retry-source cell consumed by `UpdateHierarchicalEdges`; initialized to start cell, later updated to accepted next-zone crossing cell. | Yes; writes at `0x00429BDA`, `0x0042A178`; read at `0x0042CCD8..0x0042CCE6`. |
| PathfinderClass | `+0xBC` | `u16[500]` level-0 stored zone path | `+0xBE + progress*2` is the next level-0 zone ID watched by A*. | Yes; `0x0042A162` reads `this + progress*2 + 0xBE`. |
| PathfinderClass | `+0xC74` | per-level stored path lengths | relevant to downstream invalidation after `+0x70` supplies current zone. | Yes; prior `0x0042CF80` report, not re-expanded here. |

## 3. Core Logic

### 3.1 `AStar_main_loop` seeds the retry-source cell from the A* start

At the start of normal A* setup, `AStar_main_loop` writes:

- `Pathfinder+0x6C = 0`
- `Pathfinder+0x70 = *(dword*)start_coord`
- then calls `AStar_create_node` for the start node.

Evidence:

- Decompile `AStar_main_loop @ 0x00429A90`: `*(undefined4 *)(param_1 + 0x6c) = 0; *(undefined4 *)(param_1 + 0x70) = *(undefined4 *)psVar1; piStack_48 = AStar_create_node(...)`.
- Assembly `0x00429BCD..0x00429BE6`: `MOV dword ptr [ESI + 0x6c],0x0`; `MOV ECX,dword ptr [EBP]`; `MOV dword ptr [ESI + 0x70],ECX`; `CALL 0x0042a460`.

Active in YR: Yes. `AStar_main_loop` has the sole direct caller `AStar_pathfind_search @ 0x0042C900`, and this setup is before ordinary search expansion with no TS-only gate.

### 3.2 Accepted neighbor nodes advance `+0x70` only when they enter the next stored level-0 path zone

For each candidate neighbor, `AStar_main_loop`:

1. Obtains the neighbor `CellStruct` pointer and stores `neighbor_cell + 0x24` in a stack slot.
2. Calls `ZoneMap__CellToZoneIndex` on that neighbor coordinate.
3. Reads the neighbor's level-0 zone from `DAT_0087F858[cell_index * 10]`.
4. Only after the candidate survives the gating and `AStar_create_node` path, it compares that neighbor level-0 zone against `*(u16 *)(Pathfinder + 0xBE + Pathfinder+0x6C*2)`.
5. If equal, it increments `Pathfinder+0x6C` and writes `Pathfinder+0x70 = *(dword*)neighbor_coord`.

Evidence:

- Decompile `AStar_main_loop @ 0x00429A90`: `psVar1 = (short *)(iVar16 + 0x24)` for the neighbor coordinate; `uVar14 = *(short *)(DAT_0087f858 + ZoneMap__CellToZoneIndex(psVar1) * 10)`; later after `AStar_create_node` and cost/closed-list writes, `if (uVar14 == *(ushort *)(param_1 + 0xbe + *(int *)(param_1 + 0x6c) * 2)) { *(int *)(param_1 + 0x6c) += 1; *(undefined4 *)(param_1 + 0x70) = *(undefined4 *)psVar1; }`.
- Assembly `0x00429E2B..0x00429E31`: `LEA ECX,[EBX + 0x24]`; `MOV dword ptr [ESP + 0x30],ECX`, proving the stored coordinate source is the neighbor cell's coordinate field.
- Assembly `0x00429E7F..0x00429EA0`: pushes that coordinate, calls `0x0056D3F0`, reads `DAT_0087F858`, and saves the level-0 zone in `[ESP+0x4C]`.
- Assembly `0x0042A159..0x0042A178`: reads `Pathfinder+0x6C`, loads `word ptr [ESI + EAX*2 + 0xBE]`, compares it to the neighbor zone in `EDX`, increments `+0x6C`, then writes `MOV dword ptr [ESI + 0x70],EAX` from the neighbor coordinate pointer.

Active in YR: Yes. This is inside the normal live A* expansion loop, and the hierarchy gate is the active standard pathfinding assist, not a TS-only feature.

### 3.3 `UpdateHierarchicalEdges` reads `+0x70` directly and converts that cell to current zones

`PathfinderClass__UpdateHierarchicalEdges` starts by converting `Pathfinder+0x70` to a zone-map index. It then reads the current zone ID for hierarchy levels 0, 1, and 2 from `DAT_0087F858 + cell_index*10 + level*2`, and uses `MapClass__Get_CellClass(Pathfinder+0x70)` as the flood-fill seed.

Evidence:

- Decompile `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0`: `iVar5 = ZoneMap__CellToZoneIndex(param_1 + 0x70)`; `uVar1 = *(ushort *)(iVar5 + iVar10 * 2)`; `MapClass__Get_CellClass(param_1 + 0x70)`.
- Assembly `0x0042CCD8..0x0042CCE6`: `LEA EAX,[EBX + 0x70]`; `PUSH EAX`; `CALL 0x0056d3f0`.

Active in YR: Yes. `get_function_callers` reports `AStar_pathfind_search @ 0x0042C900` as the caller of `0x0042CCD0`; the call is on the live failed-A* retry path.

### 3.4 The failed-A* caller does not pass a frontier/current-node argument

After `AStar_main_loop` returns 0 while hierarchy remains enabled, `AStar_pathfind_search` calls `UpdateHierarchicalEdges` with the Pathfinder/mover context and no A* result object. It then calls `PathfinderClass__Reset`.

Evidence:

- Decompile `AStar_pathfind_search @ 0x0042C900`: `iStack_18 = AStar_main_loop(...); if ((iStack_18 != 0) || ((char)param_8 == '\0')) break; ... PathfinderClass__UpdateHierarchicalEdges(piVar1); PathfinderClass__Reset();`.
- Assembly/caller evidence from prior and current Ghidra: direct call site `0x0042CC79 -> 0x0042CCD0`; `get_function_callers(0x0042CCD0)` returns only `AStar_pathfind_search @ 0x0042C900`.

Active in YR: Yes. This is the standard retry path reached after live `AStar_main_loop` failure.

### 3.5 `Reset` does not overwrite `+0x70` or `+0x6C` in this lifecycle

`PathfinderClass__Reset @ 0x0042A5B0` clears heap/open-list state, increments the closed-list stamp, and handles stamp wrap cleanup. The decompile contains no writes to `+0x6C` or `+0x70`. In the failed-A* retry order, `UpdateHierarchicalEdges` runs before `Reset`, so even if reset had touched the field it would not change the input to this update call.

Evidence:

- Decompile `PathfinderClass__Reset @ 0x0042A5B0`: writes include `+0x28`, heap/vector contents via `+0x14`, `+0x68`, and closed/cost arrays, with no `+0x6C` or `+0x70` write.
- Decompile `AStar_pathfind_search`: `UpdateHierarchicalEdges` precedes `PathfinderClass__Reset` on the retry path.

Active in YR: Yes. Reset is called at search entry and after failed hierarchical attempts in the live path.

## 4. INI Keys

No INI key is read by the scoped `+0x70` lifecycle.

| Key / data | Role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `MovementZone=` | Selects passability and zone row elsewhere; not the source of the failed-A* current cell. | `AStar_pathfind_search` reads movement-zone row before A*. | Yes, but not material to the target question. |
| Stored `Zone_precheck` level-0 path | Supplies the watched next zone at `Pathfinder+0xBE + progress*2`. | `0x0042A162`. | Yes. |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `AStar_pathfind_search -> AStar_main_loop` | A* attempt owns writes to `+0x70` during this slice. | `0x0042CC02` decompile call; `get_function_callers(0x00429A90)` returns `AStar_pathfind_search`. | Yes |
| `AStar_pathfind_search -> UpdateHierarchicalEdges` | Failed hierarchical attempt consumes the stored `+0x70` cell. | `0x0042CC79`; `get_function_callers(0x0042CCD0)` returns `AStar_pathfind_search`. | Yes |
| `UpdateHierarchicalEdges -> ZoneMap__CellToZoneIndex` | Converts `+0x70` cell into current level zones. | `0x0042CCD8..0x0042CCE6`. | Yes |
| `Reset` after update | Does not affect update's input cell; no `+0x70` write in reset. | `0x0042CC79` before `0x0042CC80`; reset decompile. | Yes |

## 6. Current Rust Implementation Status

Current Rust has consumer-side hierarchy/precheck state, but not the exact failed-A* context handoff:

- `src/sim/pathfinding/zone_hierarchy.rs` stores `ZonePrecheckResult.paths` and `ZonePrecheckExclusions`. The file comment explicitly says the exact failed-A* producer remains deferred.
- `src/sim/pathfinding/core.rs::astar_search` accepts a `hierarchy_gate`, but the scanned A* surface does not expose a binary-shaped "furthest accepted next-zone crossing cell" result for retry production.

Rust implication: the retry producer needs an A* attempt output/input state equivalent to `Pathfinder+0x70`: start cell initially, then updated when an accepted/generated neighbor enters the next stored level-0 path zone. `ZonePrecheckResult.paths` alone is insufficient because it does not identify which corridor-progress cell the failed cell A* actually reached.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AStar_main_loop` `+0x70` init | verified | decompile + assembly `0x00429BCD..0x00429BE6` | none |
| `AStar_main_loop` `+0x70` progress update | verified | decompile + assembly `0x00429E2B..0x00429EA0`, `0x0042A159..0x0042A178` | none |
| `UpdateHierarchicalEdges` read of `+0x70` | verified | decompile + assembly `0x0042CCD8..0x0042CCE6` | none |
| failed-A* retry call order | verified | decompile `0x0042C900`, caller evidence for `0x0042CCD0` | none |
| `PathfinderClass__Reset` interaction | verified | decompile `0x0042A5B0`; call order in `0x0042C900` | none |
| full flood-fill branch behavior | deferred | prior reports only | out-of-scope; slot 1/3 targets cover related details |
| full Rust patch design | deferred | code scan only | implementation phase |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] mode -- exhaustive-slice for the failed-A* current-zone source only.` (evidence: user scope)
- `[RESOLVED] target -- `UpdateHierarchicalEdges` uses `Pathfinder+0x70`, not a passed A* result.` (evidence: `0x0042CCD8..0x0042CCE6`)
- `[RESOLVED] init -- `AStar_main_loop` initializes `+0x70` from the start coordinate before the start node.` (evidence: `0x00429BCD..0x00429BE6`)
- `[RESOLVED] progress-counter -- `+0x6C` starts at 0 and indexes the next watched level-0 path zone at `+0xBE + progress*2`.` (evidence: `0x00429BCD`, `0x0042A159..0x0042A162`)
- `[RESOLVED] update-source -- the later `+0x70` write stores the accepted neighbor coordinate, not the current popped node.` (evidence: `0x00429E2B..0x00429E31`, `0x0042A176..0x0042A178`)
- `[RESOLVED] update-condition -- the later write occurs only when the neighbor level-0 zone equals the next stored path zone.` (evidence: `0x00429E7F..0x00429EA0`, `0x0042A162..0x0042A16C`)
- `[RESOLVED] update-order -- the later write is after candidate acceptance/node creation path, so rejected neighbors do not advance the retry-source cell.` (evidence: decompile `AStar_main_loop` around `AStar_create_node` and `0x0042A159..0x0042A178`)
- `[RESOLVED] reset -- retry `Reset` does not produce the update source and happens after `UpdateHierarchicalEdges`.` (evidence: `0x0042C900`, `0x0042A5B0`)
- `[RESOLVED] caller -- `UpdateHierarchicalEdges` is called from live `AStar_pathfind_search` after failed A*.` (evidence: `get_function_callers(0x0042CCD0)`, `0x0042CC79`)
- `[RESOLVED] TS legacy -- no TS-only gate was found on this path; it is standard YR pathfinding.` (evidence: `AStar_pathfind_search -> AStar_main_loop -> UpdateHierarchicalEdges` direct path)
- `[RESOLVED] INI -- no scoped INI key controls this cell source.` (evidence: decompiles of `0x00429A90`, `0x0042CCD0`)
- `[DEFERRED] all writes outside scoped functions -- exhaustive global write scan of `Pathfinder+0x70`.` (category: out-of-scope; reason: user bounded lifecycle to `AStar_pathfind_search`, `AStar_main_loop`, and update handoff; next-step-if-pursued: full class field audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Failed-A* retry uses `Pathfinder+0x70`: start cell initially, then furthest accepted next-level0-zone crossing cell. Active in YR: Yes. | `0x00429BDA`, `0x0042A178`, `0x0042CCD8..0x0042CCE6` | missing/unchecked; `astar_search` does not expose this exact retry-source cell | `src/sim/pathfinding/core.rs`, retry adapter near hierarchy-gated A* | Track a search-local `last_hierarchy_progress_cell` initialized to the A* start and update it only when an accepted neighbor enters the next level-0 zone in the precheck path. | A corridor path A-B-C where A* reaches a B cell then fails should retry from B's zone, not A's zone. | Do not use the last popped/open-list node or destination-adjacent cell. |
| If no accepted neighbor enters the next stored level-0 path zone, `+0x70` remains the start cell. Active in YR: Yes. | `0x00429BCD..0x00429BE6`; later write is conditional at `0x0042A16A..0x0042A178` | missing/unchecked | `src/sim/pathfinding/core.rs`, retry producer | Preserve start-cell fallback for failures inside the start zone. | Block all exits from the start zone; failed retry invalidation must seed from start zone. | Do not infer progress from precheck path length alone. |
| `UpdateHierarchicalEdges` converts the stored cell into current zones for all three levels. Active in YR: Yes. | `0x0042CCD8..0x0042CCFC` | partial; Rust has paths/markers/exclusions but needs the source cell/zone handoff | `src/sim/pathfinding/zone_hierarchy.rs` producer side | Retry producer should accept the tracked cell, look up its level 0/1/2 zones, then apply flood-fill/path-edge invalidation rules. | Multi-level hierarchy where level-0 progressed but higher-level zone is unchanged should still use all three zones derived from the same tracked cell. | Do not pass only `ZonePrecheckResult.paths` without the tracked current cell. |

Proposed Rust test names:

- `hierarchical_retry_uses_last_accepted_next_zone_crossing_cell`
- `hierarchical_retry_source_remains_start_when_no_zone_progress`
- `hierarchical_retry_updates_source_only_for_accepted_neighbors`
- `hierarchical_retry_source_cell_derives_all_three_current_zones`

## 10. Negative Facts / Do Not Do

- Do not pass the A* start cell unconditionally. Active in YR: Yes; `+0x70` is rewritten at `0x0042A178` after accepted next-zone progress.
- Do not pass the last expanded/popped A* node. Active in YR: Yes; the write source is the neighbor coordinate pointer saved at `0x00429E2B..0x00429E31`, not the current node pointer.
- Do not pass a destination-adjacent cell just because failure happened near the goal. Active in YR: Yes; the only later write is keyed to stored path-zone progress, not destination adjacency.
- Do not derive retry source from `ZonePrecheckResult.paths` alone. Active in YR: Yes; paths provide the watched zone IDs, but `+0x70` records which accepted cell actually reached the next watched zone.
- Do not expect `Reset` to compute or clear the retry-source cell. Active in YR: Yes; update occurs before reset, and reset does not write `+0x70`.

## 11. Remaining Uncertainty

- A full global audit of every `Pathfinder+0x70` write outside the scoped lifecycle was not performed; it is not needed for the failed-A* handoff because `AStar_main_loop` initializes the field before each attempt.
- Runtime frequency of each source case (no progress, one progress zone, multiple progress zones) requires instrumentation, but static binary evidence resolves the source selection.

## 12. Stale Docs / Follow-up Docs

No stale-doc replacement is required if prior docs merely say `UpdateHierarchicalEdges` uses "current zone" or "`Pathfinder+0x70`". If a reconciliation doc currently says to pass the A* start cell, last expanded cell, or `ZonePrecheckResult.paths` alone, replace it with:

> Failed hierarchical A* retry must use the `AStar_main_loop` progress cell equivalent to `PathfinderClass+0x70`: initialize it to the A* start cell, update it only when an accepted neighbor enters the next stored level-0 `Zone_precheck` path zone, and pass that cell to the retry-edge producer. The producer then derives current zones for levels 0..2 from that cell.

## Sources

- Ghidra decompile: `AStar_pathfind_search @ 0x0042C900`.
- Ghidra decompile: `AStar_main_loop @ 0x00429A90`.
- Ghidra decompile: `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0`.
- Ghidra decompile: `PathfinderClass__Reset @ 0x0042A5B0`.
- Ghidra assembly contexts: `0x00429BCD..0x00429BE6`, `0x00429E2B..0x00429EA0`, `0x0042A159..0x0042A178`, `0x0042CCD8..0x0042CCE6`.
- Ghidra caller evidence: `get_function_callers(0x00429A90)`, `get_function_callers(0x0042CCD0)`.
- Prior reports referenced: `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`, `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`.

Status: COMPLETE for the scoped failed-A* current-zone source.
