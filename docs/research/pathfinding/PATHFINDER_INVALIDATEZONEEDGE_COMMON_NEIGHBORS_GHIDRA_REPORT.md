# Pathfinder InvalidateZoneEdge Common Neighbors -- Ghidra Research Report

**Address(es):** `0x0042CF80` (`PathfinderClass__InvalidateZoneEdge`), caller `0x0042CCD0`, retry caller `0x0042C900`, append helper `0x0042D830`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact append model inside `PathfinderClass__InvalidateZoneEdge`: endpoint order, direct-edge append, common-neighbor append, adjacency scan order, duplicate handling, hierarchy-valid clearing/reset interaction, and exclusion lifetime.  
**Non-Scope:** full `ZoneMap__FloodFillReachableZones`, full cell A* failure causes, full stock-map route capture, layered bridge/tube route parity, and Rust implementation.  
**Confidence:** High for scoped binary behavior; Medium for stock-route impact because no runtime route trace was taken.  
**Active in YR:** Yes. Standard foot pathfinding reaches `FootClass__Run_AStar @ 0x004CBBA0`, which calls `AStar_pathfind_search @ 0x0042C900` at `0x004CBC31`; failed hierarchical cell A* calls `UpdateHierarchicalEdges @ 0x0042CCD0` at `0x0042CC79`; nonzero flood-fill calls `InvalidateZoneEdge @ 0x0042CF80` at `0x0042CDAC`.

## 0. Investigation Contract

Target question: What exact per-level retry-local exclusions does `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80` append, in what order, and with what duplicate/lifetime semantics, so Rust can implement a deterministic retry-local exclusion producer?

Non-goals: Do not re-investigate the settled `UpdateHierarchicalEdges` level loop, the full `Zone_precheck` consumer, flood-fill internals, temp edge buckets, INI parsing, route-oracle captures, or any Rust patch.

Evidence needed to mark COMPLETE:

- Decompile and assembly evidence for path lookup, no-edge clearing, selected direct edge, direct append, common-neighbor loops, append helper behavior, and caller reset/lifetime.
- Caller evidence that the path is active in standard YR, not a TS-only branch.
- Current Rust scan sufficient to name affected surfaces and mismatch without editing Rust.
- Explicit duplicate/order findings, including adjacency loop direction.

Stop conditions:

- Stop once `0x0042CF80` append order and lifetime are proven from decompile plus assembly context.
- Defer stock-map route impact and branch incidence to runtime tracing.
- Write only this report and the shared claims file.

## 1. Overview

`InvalidateZoneEdge` handles the nonzero `FloodFillReachableZones` branch after a failed hierarchical cell A* attempt. It locates the current zone in the stored `Zone_precheck` path for the level, appends one direct path-edge exclusion, then appends additional exclusions from the earlier endpoint of that direct path edge to every common neighbor shared by the direct edge's endpoints.

The append model is list-like, not set-like. The binary appends records in encounter order, scans adjacency lists backward, and performs no duplicate suppression before writing to the Pathfinder-local exclusion vector.

## 2. Key Offsets / Data

| Owner | Offset / address | Meaning | Active in YR |
|---|---:|---|---|
| `PathfinderClass` | `+0x38` | hierarchy-valid flag; set at search entry, cleared by `InvalidateZoneEdge` if no path edge can be selected. | Yes; set at `0x0042C909`, cleared at `0x0042CF9A` / `0x0042CFD3` / `0x0042CFE6`, read after reset at `0x0042CC85`. |
| `PathfinderClass` | `+0x74 + level*0x18` | per-level exclusion vector object. | Yes; direct append uses `0x0042D069`; common-neighbor append uses `0x0042D101`. |
| `PathfinderClass` | `+0x78/+0x84 + level*0x18` | exclusion data pointer/count. | Yes; producer writes these, `Zone_precheck` consumes them in prior verified reports. |
| `PathfinderClass` | `+0xBC + level*1000` | stored selected `Zone_precheck` path, start-to-destination order. | Yes; path scan at `0x0042CFBA..0x0042CFD0`, edge loads at `0x0042D004..0x0042D031`. |
| `PathfinderClass` | `+0xC74 + level*4` | stored path length for the level. | Yes; read at `0x0042CF8D`. |
| global `DAT_0087F878 + level*0x18` | hierarchy graph for that level. | Yes; read at `0x0042D082`, zone record stride `0x24`. |
| zone record `+0x04/+0x10` | adjacency pointer/count. | Yes; endpoint adjacency count read at `0x0042D096` and `0x0042D0BA`, adjacency entries read at `0x0042D0A4..0x0042D0A7`, `0x0042D0D7..0x0042D0DE`. |
| append helper | `0x0042D830` | vector push with capacity growth; no duplicate scan. | Yes; direct call at `0x0042D06D`; common-neighbor append inlined at `0x0042D0FA..0x0042D13C`. |

## 3. Core Logic

### 3.1 Liveness and retry entry

Active in YR: Yes. Evidence: `FootClass__Run_AStar @ 0x004CBBA0` calls `AStar_pathfind_search @ 0x0042C900` (`0x004CBC31`). In `AStar_pathfind_search`, a failed hierarchical `AStar_main_loop` reaches `UpdateHierarchicalEdges @ 0x0042CCD0` (`0x0042CC79`), then `PathfinderClass__Reset @ 0x0042A5B0` (`0x0042CC80`), then rereads `Pathfinder+0x38` (`0x0042CC85`). `UpdateHierarchicalEdges` calls `InvalidateZoneEdge @ 0x0042CF80` on nonzero flood-fill (`0x0042CD80` call to flood-fill, `0x0042CD85..0x0042CD87` branch, `0x0042CDAC` call).

No TS-only option gate appears in this caller chain. The only gate for this slice is dynamic: hierarchy must be active and cell A* must fail while the flood-fill branch returns nonzero.

### 3.2 No-edge cases clear hierarchy-valid and append nothing

Active in YR: Yes. Evidence: `0x0042CF8D` reads path length, `0x0042CF94..0x0042CFA4` clears `+0x38` and returns when length is less than two. The path scan starts at `0x0042CFBA`; if `current_zone` is not found before `i == length`, `0x0042CFD3..0x0042CFDD` clears `+0x38` and returns. The secondary `EAX == -1` guard at `0x0042CFE0..0x0042CFF0` also clears and returns, though the normal scan reaches either found or absent first.

Material behavior: no direct edge and no common-neighbor exclusions are appended in these clearing cases. The caller's post-update `Reset` does not restore `+0x38`; `0x0042CC85..0x0042CC99` converts the cleared flag into disabled hierarchy for later retry flow.

### 3.3 Direct edge endpoint order

Active in YR: Yes. Evidence: decompile `0x0042CF80`; assembly `0x0042CFF3..0x0042D036`.

The stored path is start-to-destination. Let `i` be the index where `path[i] == current_zone` and `len` be `Pathfinder+0xC74[level]`.

- If `i == len - 1`, the direct edge is `(path[i - 1], path[i])`.
  Evidence: `0x0042CFF3..0x0042D014` decrements the length, detects the last index, loads `path[i]` from `+0xBC`, and loads `path[i-1]` from `+0xBA`.
- Otherwise, the direct edge is `(path[i], path[i + 1])`.
  Evidence: `0x0042D01B..0x0042D031` loads `path[i]` from `+0xBC` and `path[i+1]` from `+0xBE`.

The rest of the function keeps two roles:

- `early_endpoint`: the lower path index of the selected direct edge, i.e. `path[i-1]` for the last-element case or `path[i]` otherwise.
- `late_endpoint`: the higher path index of the selected direct edge, i.e. `path[i]` for the last-element case or `path[i+1]` otherwise.

The packed edge identity is canonicalized for storage: smaller zone id in the high halfword, larger zone id in the low halfword. Evidence: `0x0042D03A..0x0042D060` compares, swaps if needed, shifts by `0x10`, ORs, and stores the packed key.

### 3.4 Direct edge append comes first and uses the append helper

Active in YR: Yes. Evidence: `0x0042D064..0x0042D06D` passes the packed direct edge to `FUN_0042D830` with `ECX = Pathfinder + 0x74 + level*0x18`. The call occurs before any graph adjacency reads; graph base `DAT_0087F878` is first loaded after the direct append at `0x0042D082`.

If the selected endpoints are equal, direct append is skipped. Evidence: `0x0042D048..0x0042D04E` compares the endpoint ids and jumps to graph scan when equal. This is an edge-case guard; normal `Zone_precheck` paths should not contain self-edges.

`FUN_0042D830` appends to the vector without searching existing entries. Evidence: decompile `0x0042D830`; assembly `0x0042D833..0x0042D86D` checks/grows capacity, increments count, and writes `*param_2` at `data[count]`. No loop compares existing packed keys.

### 3.5 Common-neighbor append is asymmetric: early endpoint to common neighbor

Active in YR: Yes. Evidence: decompile `0x0042CF80`; assembly `0x0042D072..0x0042D13C`.

After direct append, the function loads graph records for both direct endpoints:

- `late_record = graph + late_endpoint * 0x24`
- `early_record = graph + early_endpoint * 0x24`

Evidence: `0x0042D076..0x0042D092` computes the two record addresses after loading `DAT_0087F878 + level*0x18`.

The common-neighbor rule is:

1. Scan `late_endpoint` adjacency backward from `late_count - 1` to `0`.
2. Skip the direct `early_endpoint` neighbor.
3. For each remaining `candidate`, scan `early_endpoint` adjacency backward from `early_count - 1` to `0`.
4. When an `early_endpoint` adjacency entry equals `candidate`, append a packed sorted exclusion `(early_endpoint, candidate)`.

Evidence:

- Outer count initialized with `late_record+0x10`, decremented before first use at `0x0042D096..0x0042D09E`.
- Outer adjacency entry loaded from `late_record+0x04 + outer_index*8` at `0x0042D0A4..0x0042D0A7`.
- `candidate == early_endpoint` skips the whole inner loop at `0x0042D0AB..0x0042D0B0`.
- Inner count initialized from `early_record+0x10`, decremented before first use at `0x0042D0B6..0x0042D0C2`.
- Inner compare uses `early_record+0x04 + inner_index*8 == candidate` at `0x0042D0D7..0x0042D0E1`.
- Append key is canonicalized from `early_endpoint` and `candidate` at `0x0042D0E3..0x0042D0F8`, then written to the same level vector at `0x0042D0FA..0x0042D13C`.

This is not "append both endpoint-to-common edges." It appends only `early_endpoint -> common_neighbor` as an undirected packed edge. It is also not "append late endpoint to common neighbor."

### 3.6 Adjacency scan order and duplicate handling

Active in YR: Yes. Evidence: loop assembly `0x0042D096..0x0042D157` and append helper `0x0042D830`.

Order is deterministic from graph adjacency order but reversed:

- Direct selected path edge is appended first.
- Then outer `late_endpoint` adjacency is scanned from last stored edge to first stored edge.
- For each outer candidate, inner `early_endpoint` adjacency is scanned from last stored edge to first stored edge.
- The append happens immediately on every inner match.

There is no duplicate suppression:

- Existing exclusion entries are not scanned before direct append (`0x0042D06D` calls plain vector push).
- Existing exclusion entries are not scanned before common-neighbor append (`0x0042D0FA..0x0042D13C` inlines vector push).
- Duplicate adjacency entries would produce duplicate exclusion records.
- A duplicate already appended by a prior retry remains; a later retry can append the same packed key again.

`Zone_precheck` later scans exclusions backward and skips on first matching packed edge, so duplicates are redundant but binary-permitted. The scoped producer's list order and multiplicity are still part of the exact append model.

### 3.7 Reset interaction and exclusion lifetime

Active in YR: Yes. Evidence: `AStar_pathfind_search @ 0x0042C900`, `PathfinderClass__Reset @ 0x0042A5B0`, assembly `0x0042C909..0x0042C925`, `0x0042CC79..0x0042CCB3`.

At search entry:

1. `Pathfinder+0x38 = 1` (`0x0042C909`).
2. `PathfinderClass__Reset` runs (`0x0042C90D`).
3. The three exclusion vectors at `+0x74`, `+0x8C`, `+0xA4` are cleared by virtual call `vtable+0x0C` in a three-iteration loop (`0x0042C912..0x0042C925`).

On retry after failed hierarchical cell A*:

1. `UpdateHierarchicalEdges` appends exclusions (`0x0042CC79`).
2. `PathfinderClass__Reset` runs (`0x0042CC80`).
3. The caller reads `+0x38` (`0x0042CC85`) and, if still true, calls `Zone_precheck` again (`0x0042CCB3`).

`PathfinderClass__Reset` clears marker/heap/search scratch and increments the epoch, but its decompile contains no clears of the `+0x74/+0x8C/+0xA4` exclusion vectors. Therefore exclusions survive retry reset and remain local to the current `AStar_pathfind_search` call. They are cleared at the next search entry, not after each retry.

## 4. INI Keys

No INI key is read by `0x0042CF80` or the scoped append helper.

| Data | Role | Evidence | Active in YR |
|---|---|---|---|
| `MovementZone=` / `TechnoTypeClass+0x5B4` | Upstream pathfinding input; can affect whether the failed-A* retry path is reached through `Zone_precheck`/A*. | `AStar_pathfind_search` reads movement zone when caller passes `-1`; not read by `0x0042CF80`. | Yes, upstream only. |
| hierarchy graph `DAT_0087F878` | Direct source for common-neighbor adjacency. | `0x0042D082..0x0042D13C`. | Yes. |

## 5. Current Rust Implementation Status

| Rust surface | Status vs this slice |
|---|---|
| `src/sim/pathfinding/zone_hierarchy.rs:245` `ZonePrecheckExclusions` | Current type stores per-level exclusions in `BTreeSet<ZoneEdgeKey>`, which canonicalizes and deduplicates. That matches consumer edge identity but not producer append multiplicity/order. |
| `src/sim/pathfinding/zone_hierarchy.rs:269` `ZonePrecheckResult` | Retains per-level `paths`, which is required for direct-edge selection. |
| `src/sim/pathfinding/zone_search.rs:285..293` | Production hierarchy branch calls `zone_precheck_flat` with default empty exclusions and immediately runs marker-gated cell A*; no failed-A* retry producer is wired. |
| `src/sim/pathfinding/zone_search.rs:694` `exclude_corridor_edges` | Compatibility retry excludes every edge in the Rust corridor, which is not the binary producer. |
| Needed surface | A retry-local ordered vector/list producer that can append direct and common-neighbor exclusions from retained paths plus graph adjacency. The consumer may still use set-like lookup if tests separately preserve producer order/multiplicity where needed. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard YR pathfinding liveness | verified | `0x004CBBA0`, `0x004CBC31`, `0x0042C900`, `0x0042CC79`, `0x0042CDAC` | runtime incidence of nonzero flood-fill branch |
| Path length less than two | verified | `0x0042CF8D..0x0042CFA4` | none |
| Current zone absent from path | verified | `0x0042CFBA..0x0042CFDD` | none |
| Direct edge endpoint choice | verified | `0x0042CFF3..0x0042D036` | none |
| Direct edge append order | verified | `0x0042D064..0x0042D082` | none |
| Common-neighbor endpoint asymmetry | verified | `0x0042D072..0x0042D13C` | none |
| Adjacency scan direction | verified | `0x0042D096..0x0042D157` | none |
| Duplicate handling | verified | `0x0042D830`; inlined push `0x0042D0FA..0x0042D13C` | none |
| Reset/exclusion lifetime | verified | `0x0042C909..0x0042C925`, `0x0042CC79..0x0042CCB3`, `0x0042A5B0` | none |
| Current Rust surfaces | verified scan | Codegraph context; `zone_hierarchy.rs`, `zone_search.rs`, `core.rs` | implementation not performed |

## 7. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- Is this a bounded exhaustive slice? -> Yes, only `0x0042CF80` append model and lifetime are claimed.` (evidence: user scope)
- `[RESOLVED] OQ-2 -- Is `InvalidateZoneEdge` active in standard YR? -> Yes, live foot pathfinding reaches failed hierarchical retry and nonzero flood-fill branch.` (evidence: `0x004CBC31`, `0x0042CC79`, `0x0042CDAC`; Active in YR: Yes)
- `[RESOLVED] OQ-3 -- What happens if no path edge can be selected? -> Clear `Pathfinder+0x38`, append nothing, return.` (evidence: `0x0042CF94..0x0042CFA4`, `0x0042CFD3..0x0042CFF0`; Active in YR: Yes)
- `[RESOLVED] OQ-4 -- Which direct edge is selected? -> Previous edge if current zone is last in path, otherwise next edge.` (evidence: `0x0042CFF3..0x0042D031`; Active in YR: Yes)
- `[RESOLVED] OQ-5 -- Is direct append before common-neighbor append? -> Yes, direct append calls `0x0042D830` before graph adjacency is loaded.` (evidence: `0x0042D064..0x0042D082`; Active in YR: Yes)
- `[RESOLVED] OQ-6 -- Which endpoint is used for common-neighbor exclusions? -> The earlier path-order endpoint of the selected direct edge is paired with each common neighbor.` (evidence: `0x0042D072..0x0042D13C`; Active in YR: Yes)
- `[RESOLVED] OQ-7 -- What is adjacency scan order? -> Backward over late endpoint adjacency, and backward over early endpoint adjacency for each candidate.` (evidence: `0x0042D096..0x0042D157`; Active in YR: Yes)
- `[RESOLVED] OQ-8 -- Are duplicates suppressed? -> No; append helper and inlined common append do not search existing entries.` (evidence: `0x0042D830`, `0x0042D0FA..0x0042D13C`; Active in YR: Yes)
- `[RESOLVED] OQ-9 -- Does retry `Reset` clear exclusions? -> No; search entry clears vectors, retry reset preserves them.` (evidence: `0x0042C912..0x0042C925`, `0x0042CC79..0x0042CCB3`, `0x0042A5B0`; Active in YR: Yes)
- `[RESOLVED] OQ-10 -- Can current Rust produce this exactly today? -> No; it has retained paths and set-like exclusions but no producer and no ordered duplicate-preserving append surface.` (evidence: `zone_hierarchy.rs:245`, `zone_hierarchy.rs:269`, `zone_search.rs:285..293`; Active in YR: N/A Rust)
- `[DEFERRED] OQ-11 -- How often common-neighbor appends change stock-map routes?` (category: needs-runtime-debugger; reason: static binary proves append model, not route incidence; next-step-if-pursued: instrument failed hierarchy retries and logged exclusions)
- `[DEFERRED] OQ-12 -- Full flood-fill return-condition edge cases.` (category: out-of-scope; reason: another swarm slot owns flood-fill producer; next-step-if-pursued: audit `0x005840C0`)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Direct retry exclusion is selected from the stored path: previous edge if current zone is last, otherwise next edge, then appended before any common-neighbor exclusions. | `0x0042CFF3..0x0042D06D`; Active in YR: Yes. | Missing producer; `ZonePrecheckResult.paths` exists but is not used for retry production. | `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_search.rs`, future retry producer. | Build direct exclusion from retained per-level path and current zone, preserving append-before-common order. | Path `[1,2,3]`: current `2` appends `2-3`; current `3` appends `2-3`; no other direct path edges append. Proposed test name: `invalidate_zone_edge_appends_selected_direct_edge_first`. | Do not exclude every edge in the selected path or the failed Rust corridor. |
| Common-neighbor exclusions scan the later endpoint's adjacency backward, test against the earlier endpoint's adjacency backward, and append `(early_endpoint, common)` for each match. | `0x0042D072..0x0042D13C`; Active in YR: Yes. | Missing producer; current compatibility retry has no common-neighbor append. | Hierarchy graph adjacency and retry-local exclusion producer. | Use graph adjacency order directly and append only early-endpoint-to-common-neighbor packed edges. | Direct path edge `2-3`, late endpoint `3` adjacency stored `[2,4,5]`, early endpoint `2` adjacency `[3,5,4]` appends common exclusions in late reverse order, with inner reverse matches. Proposed test name: `invalidate_zone_edge_appends_common_neighbors_in_binary_reverse_scan_order`. | Do not append both endpoint-to-common edges, do not sort common neighbors, and do not use ZoneId order. |
| Exclusion vectors are ordered retry-local append lists cleared at search entry, preserved by retry `Reset`, and not deduplicated by the producer. | `0x0042C912..0x0042C925`, `0x0042CC79..0x0042CCB3`, `0x0042D830`; Active in YR: Yes. | `ZonePrecheckExclusions` is a `BTreeSet`, adequate for consumer lookup but not for exact producer order/multiplicity. | `ZonePrecheckExclusions` or a new producer-side ordered list feeding the existing consumer. | Preserve append list semantics through the retry loop; clear only at path-search entry. | Repeated retry appending the same edge twice leaves two producer records until the next search entry; consumer still skips the edge. Proposed test name: `retry_exclusions_preserve_duplicate_appends_until_new_search`. | Do not clear exclusions in `Reset`; do not make producer success depend on set insertion returning true. |

## 9. Negative Facts / Do Not Do

- Do not implement common-neighbor append as "late endpoint to common neighbor." Active in YR: Yes; evidence append canonicalizes `early_endpoint` with candidate at `0x0042D0E3..0x0042D13C`.
- Do not append both endpoint-common pairs for a shared neighbor. Active in YR: Yes; only one append path exists and uses `early_endpoint`. Evidence: `0x0042D0E3..0x0042D13C`.
- Do not sort common neighbors by `ZoneId`. Active in YR: Yes; scan is reverse adjacency order. Evidence: `0x0042D096..0x0042D157`.
- Do not deduplicate producer appends or rely on `BTreeSet::insert == false` to suppress binary-visible append attempts. Active in YR: Yes; `0x0042D830` and common append write without duplicate search.
- Do not clear retry exclusions inside `PathfinderClass__Reset`. Active in YR: Yes; search entry clears vectors at `0x0042C912..0x0042C925`, retry reset at `0x0042CC80` preserves them.

## 10. Remaining Uncertainty

- Runtime frequency and player-visible route impact of common-neighbor appends remain unmeasured.
- This report does not prove the exact `FloodFillReachableZones` nonzero branch conditions; it only proves what happens after that branch calls `InvalidateZoneEdge`.
- This report does not decide whether Rust's consumer can remain set-backed while the producer keeps an ordered duplicate-preserving trace; that is an implementation design choice.

## 11. Stale Docs / Follow-up Wording

- Any implementation doc that says "use `ZonePrecheckExclusions` as a set for exact retry producer parity" should be replaced with: "The consumer may check canonical edge membership, but the `InvalidateZoneEdge` producer appends to an ordered retry-local vector with no duplicate suppression; exact producer tests must verify append order and duplicate-preserving lifetime."
- Any implementation doc that says "append common-neighbor exclusions for both endpoints" should be replaced with: "After the direct path-edge exclusion, `InvalidateZoneEdge` scans the later endpoint's adjacency backward and appends `(earlier path-edge endpoint, common_neighbor)` for each neighbor also found in the earlier endpoint's adjacency, also scanned backward."

## Sources

- Ghidra decompiled/read: `0x0042CF80`, `0x0042CCD0`, `0x0042C900`, `0x0042A5B0`, `0x0042D830`, `0x004CBBA0`, `0x004D3920`.
- Ghidra assembly contexts: `0x004CBC31`, `0x0042C909..0x0042C925`, `0x0042CC79..0x0042CCB3`, `0x0042CD80..0x0042CDAC`, `0x0042CF8D..0x0042D13C`, `0x0042D830..0x0042D86D`.
- Prior docs checked: `UPDATEHIERARCHICALEDGES_FAILED_ASTAR_EDGE_SELECTION_GHIDRA_REPORT.md`, `UPDATE_HIERARCHICAL_EDGES_RETRY_PRODUCER_SCOPE_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`.
- Rust scanned read-only: `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core.rs`.

Status: COMPLETE.
