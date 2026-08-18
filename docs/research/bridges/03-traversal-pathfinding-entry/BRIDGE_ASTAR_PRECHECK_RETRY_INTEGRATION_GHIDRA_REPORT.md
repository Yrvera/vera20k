# Bridge A* Precheck / Retry Integration -- Ghidra Research Report

**Address(es):** `0x0042C900` (`AStar_pathfind_search`), `0x0042C290` (`Zone_precheck`), `0x0042CCD0` (`PathfinderClass__UpdateHierarchicalEdges`), `0x0042CF80` (`PathfinderClass__InvalidateZoneEdge`), `0x00429A90` (`AStar_main_loop`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `AStar_pathfind_search` integration around zone precheck, same-zone vs cross-zone initial failure, retry attempt budget, retry-local exclusion lifetime, hierarchy-disable behavior, and what `Zone_precheck` output feeds into cell A*.
**Non-Scope:** zone graph writer internals, exact graph emission order, full `Zone_Estimate_Slope_Cost`, full `MapClass__FloodFillReachableZones`, concrete stock-map route capture after bridge collapse, and Rust implementation.
**Confidence:** High for the scoped binary call/branch contract; Medium for Rust delta where exact full hierarchy data is not present yet.
**Active in YR:** Yes. Evidence: `FootClass__Run_AStar @ 0x004CBBA0` calls `AStar_pathfind_search`; `AStar_pathfind_search` calls `Zone_precheck` and `AStar_main_loop` without a TS-only gate.

## 0. Investigation Contract

Target question: How does live `AStar_pathfind_search` integrate `Zone_precheck`, hierarchy disable, retry-local edge exclusions, attempt caps, and the cell A* corridor after a bridge/pathing failure?

Non-goals: Do not re-investigate zone graph writer internals, bridge collapse state machines, full A* edge costs, or stock-map route outcome.

Evidence needed to mark COMPLETE: decompile `0x0042C900`, spot-check `0x0042C290`, `0x0042CCD0`, `0x0042CF80`, and `0x00429A90`; scan current Rust `zone_search.rs`; separate current stale-doc claims from verified binary facts.

Stop conditions: stop once the retry/precheck call contract is verified and Rust-facing handoff is concrete; defer graph writer and runtime route incidence.

## 1. Overview

`AStar_pathfind_search` is the retry wrapper around cell A*. It starts with hierarchy enabled, clears retry-local exclusion vectors once, optionally runs `Zone_precheck`, calls `AStar_main_loop`, and after failed hierarchical attempts appends Pathfinder-local undirected zone-edge exclusions before rerunning `Zone_precheck`.

For bridge-collapse parity, the important split is: global bridge/zone rebuild decides what graph exists, but failed cell A* attempts do not rebuild that graph. They add per-search edge exclusions and retry within the same `AStar_pathfind_search` call.

## 2. Class Layout / Key Offsets

| Owner | Offset | Meaning | Active in YR |
|---|---:|---|---|
| `PathfinderClass` | `+0x38` | hierarchy-valid flag; initialized to `1`, cleared by invalidation failure, copied into local hierarchy flag after retry update | Yes, `0x0042C909`, `0x0042CC85`, `0x0042CF94` |
| `PathfinderClass` | `+0x3C` | urgency / bridge-passability marker mode passed to cell A* helpers | Yes, set in `0x0042C900`, consumed by `0x00429A90` |
| `PathfinderClass` | `+0x40` | level-0 chosen-zone marker array used by cell A* when hierarchy is enabled | Yes, `0x00429D71`, `0x00429E94` |
| `PathfinderClass` | `+0x74 + level*0x18` | retry-local vector object for packed excluded zone edges | Yes, cleared at entry and appended by `0x0042CCD0`/`0x0042CF80` |
| `PathfinderClass` | `+0x78/+0x84 + level*0x18` | excluded-edge data pointer/count consumed by `Zone_precheck` | Yes, `0x0042C618..0x0042C664` |
| `PathfinderClass` | `+0xBC + level*1000` | stored chosen zone path from `Zone_precheck` | Yes, written by `0x0042C887..0x0042C8CE`, read by `0x0042CF80` |
| `PathfinderClass` | `+0xC74 + level*4` | stored chosen path count per hierarchy level | Yes, written by `Zone_precheck`, read at `0x0042CF8D` |

## 3. Core Logic

### 3.1 Entry setup clears retry-local exclusions once

Active in YR: Yes. Evidence: `AStar_pathfind_search @ 0x0042C900` is reached from `FootClass__Run_AStar @ 0x004CBBA0`.

At function entry, `AStar_pathfind_search` writes `Pathfinder+0x38 = 1`, calls `PathfinderClass__Reset`, then loops exactly three vector objects starting at `Pathfinder+0x74`, stepping by `0x18`, and calls each vector's clear slot. This is before start/destination zone lookup and before the first `Zone_precheck`.

Material detail: the retry path later calls `PathfinderClass__Reset` again, but does not clear these three vectors. Therefore exclusions are local to one `AStar_pathfind_search` call and persist across attempts inside that call.

### 3.2 Initial same-zone and cross-zone failures are intentionally different

Active in YR: Yes. Evidence: decompile `0x0042C900`, branches around `0x0042CB22..0x0042CB86`.

`AStar_pathfind_search` first compares the start and destination zone IDs returned by `MapClass__GetZoneID`.

If the zone IDs match and hierarchy is enabled, it calls `Zone_precheck`. If that precheck returns false, it logs the hierarchical failure string and clears the local hierarchy flag, then still calls `AStar_main_loop` without hierarchy.

If the zone IDs differ and hierarchy is enabled, it returns `0` immediately. There is no cell A* attempt and no retry-local edge update in this initial cross-zone hard-fail branch.

If hierarchy is already disabled, different zones are not a hard gate; the function may log the "A* without HS" warning and calls cell A*.

### 3.3 Attempt cap is total attempts, not "five retries"

Active in YR: Yes. Evidence: decompile expression in `0x0042C900`: `iStack_14 = (-(uint)(param_6 != -1) & 0xfffffffc) + 5`; retry counter increments before compare after failed A*.

When the caller passes the default search-limit argument `param_6 == -1`, the loop allows five total `AStar_main_loop` calls. When the caller passes any other value, the loop allows one total `AStar_main_loop` call. This is not an initial attempt plus five retries.

The normal `FootClass__Run_AStar` call passes `-1` for the scoped limit argument, so the default foot path uses the five-total-attempt mode. Active in YR: Yes; evidence `FootClass__Run_AStar @ 0x004CBBA0` decompile.

### 3.4 Failed hierarchical cell A* updates exclusions, resets search state, and reruns precheck

Active in YR: Yes. Evidence: `AStar_pathfind_search` post-call branch around `0x0042CC02..0x0042CCC0`; `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0`; `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80`.

After `AStar_main_loop` returns nonzero, `AStar_pathfind_search` returns success. After it returns zero:

1. If hierarchy is disabled, return zero.
2. Otherwise log regular failure for non-adjacent endpoints.
3. Increment attempt counter.
4. Call `PathfinderClass__UpdateHierarchicalEdges`.
5. Call `PathfinderClass__Reset`.
6. Copy `Pathfinder+0x38` back into the local hierarchy flag.
7. If the attempt cap is reached, return zero.
8. If hierarchy remains enabled, rerun `Zone_precheck`; if that fails, return zero; if it succeeds, loop back to cell A*.

`UpdateHierarchicalEdges` loops levels `0..2`, uses `ZoneMap__FloodFillReachableZones`, and appends packed sorted undirected zone-pair exclusions into Pathfinder-local vectors. `InvalidateZoneEdge` clears `+0x38` when the stored path length is less than two or the current zone is absent from the stored path.

### 3.5 Exclusions are retry-local edges, not zones and not global rebuilds

Active in YR: Yes. Evidence: `Zone_precheck @ 0x0042C290`, exclusion scan `0x0042C618..0x0042C664`; producer decompiles `0x0042CCD0`, `0x0042CF80`.

`Zone_precheck` canonicalizes each candidate edge as `(min(current_zone, neighbor_zone) << 16) | max(current_zone, neighbor_zone)` and scans the current level's exclusion vector. A match skips only that candidate edge.

No evidence in this slice shows `AStar_pathfind_search` rebuilding global zone maps between retry attempts. `UpdateHierarchicalEdges` reads global graph/cell-zone data and appends to Pathfinder-owned vectors. It does not mutate `DAT_0087F878`, `DAT_0087F858`, or bridge records.

### 3.6 What `Zone_precheck` feeds to cell A*

Active in YR: Yes. Evidence: `Zone_precheck` writes `+0xBC/+0xC74` and marker arrays; `AStar_main_loop @ 0x00429A90` reads `Pathfinder+0x40` around `0x00429D71`, candidate zone marker at `0x00429E94`, and hierarchy flag `param_7`.

`Zone_precheck` searches levels `2 -> 1 -> 0`, writes the chosen path per level, and marks the chosen zones in per-level marker arrays. Cell A* receives the hierarchy-enabled flag and uses the level-0 marker array as a pruning input while expanding cells.

Important nuance: this is not a Rust-style "expand the chosen zone corridor by one neighbor ring" behavior. The binary's input to cell A* is the stored/marked path from `Zone_precheck`, plus the existing A* cell predicates and the known `CellClass+0x122` occupancy-adjacent exception from prior reports. The exact `+0x122` interaction is already documented elsewhere and was not expanded in this slot.

## 4. INI Keys

No INI key is read directly by the scoped retry functions.

| Key / data | Effect in this slice | Active in YR |
|---|---|---|
| `MovementZone=` / `TechnoTypeClass+0x5B4` | Supplies the movement-zone row when the caller uses the default override; `Zone_precheck` uses it for passability matrix row. | Yes, read in `0x0042C900` before `Zone_precheck` |
| `ZonePassabilityMatrix` | Candidate zone graph edge passes only when matrix row/column value is `1`. | Yes, consumed by `0x0042C290` |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `FootClass__Run_AStar -> AStar_pathfind_search` | Normal foot pathfinding reaches the wrapper with default limit `-1`. | decompile `0x004CBBA0` | Yes |
| Initial same-zone hierarchy failure | Disables hierarchy and continues to cell A*. | `0x0042CB46..0x0042CB86` | Yes |
| Initial cross-zone hierarchy failure | Returns zero before cell A*. | `0x0042CB32..0x0042CB3F` | Yes |
| Failed hierarchical attempt | Calls `UpdateHierarchicalEdges`, then `Reset`, then may rerun `Zone_precheck`. | `0x0042CC79..0x0042CCB8` | Yes |
| Cell A* corridor input | Uses `Zone_precheck` marker/path output when hierarchy flag is enabled. | `0x0042C887..0x0042C8CE`, `0x00429E94..0x00429F04` | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current status vs binary slice |
|---|---|
| `src/sim/pathfinding/zone_search.rs:37` | `MAX_CORRIDOR_RETRIES = 5` matches the default total-attempt budget shape, but Rust does not expose the non-default one-attempt mode from `param_6 != -1`. |
| `src/sim/pathfinding/zone_search.rs:229..255` | Flat zoned pathing already has the same-zone fallback vs cross-zone abort split at the reduced reachability level. This matches the broad wrapper contract, but it is not full `Zone_precheck`. |
| `src/sim/pathfinding/zone_search.rs:325..356` | Rust retries by recomputing a corridor and excluding corridor edges; this is closer than stale whole-zone docs, but still not binary-shaped `UpdateHierarchicalEdges`/`FloodFillReachableZones` exclusion generation. |
| `src/sim/pathfinding/zone_search.rs:330` and `:687..694` | Rust expands the corridor by one neighbor ring before cell A*. No binary evidence in this slice supports that as parity behavior. |
| `src/sim/pathfinding/zone_search.rs:519..583` | Rust `find_zone_corridor` remains a single-level Dijkstra using centroid Manhattan cost, not three-level `Zone_precheck` with parent gating and zone-type/flag/slope cost. |
| `src/sim/pathfinding/zone_search.rs:406..459` | Layered pathing only does reachability precheck then full layered A*. It does not model the same retry/precheck/corridor loop. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AStar_pathfind_search` entry setup | verified | decompile `0x0042C900` | none |
| same-zone initial `Zone_precheck` failure | verified | `0x0042CB46..0x0042CB86` | none |
| cross-zone initial hierarchy hard fail | verified | `0x0042CB32..0x0042CB3F` | none |
| default/non-default attempt cap | verified | decompile expression in `0x0042C900`; caller `0x004CBBA0` | nonstandard caller incidence |
| retry update ordering | verified | `0x0042CC79 -> 0x0042CCD0 -> 0x0042A5B0 -> 0x0042CCB3` | none |
| exclusion consumption | verified | `0x0042C618..0x0042C664` | none for edge-vs-zone |
| `UpdateHierarchicalEdges` exclusion generation | touched-not-exhausted | decompile `0x0042CCD0`, `0x0042CF80` | full `FloodFillReachableZones` internals out-of-scope |
| cell A* use of zone path markers | touched-not-exhausted | decompile `0x00429A90` marker gate | full A* cost/path ordering out-of-scope |
| current Rust `zone_search.rs` | verified scan | file read and `rg` line refs | implementation not changed |
| stock map bridge-collapse route | deferred | no runtime trace in this slot | capture live graph/path trace |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is the slice live in standard YR? -> Yes, normal foot pathing reaches `FootClass__Run_AStar -> AStar_pathfind_search -> Zone_precheck/AStar_main_loop`.` (evidence: `0x004CBBA0`, `0x0042C900`; Active in YR: Yes)
- `[RESOLVED] OQ-2 - Are retry exclusions cleared per attempt? -> No, cleared once at search entry; retry `Reset` does not clear them.` (evidence: `0x0042C912..0x0042C925`, `0x0042CC80`; Active in YR: Yes)
- `[RESOLVED] OQ-3 - What is the default attempt cap? -> Five total A* attempts when `param_6 == -1`, one total attempt otherwise.` (evidence: `0x0042CB8B..0x0042CBA1` decompile; Active in YR: Yes)
- `[RESOLVED] OQ-4 - Does same-zone precheck failure abort? -> No, it disables hierarchy and still runs cell A*.` (evidence: `0x0042CB46..0x0042CB86`; Active in YR: Yes)
- `[RESOLVED] OQ-5 - Does cross-zone initial hierarchy failure run cell A*? -> No, it returns zero before A*.` (evidence: `0x0042CB32..0x0042CB3F`; Active in YR: Yes)
- `[RESOLVED] OQ-6 - What happens after failed hierarchical A*? -> update exclusions, reset, copy `+0x38`, cap-check, rerun `Zone_precheck` if still enabled.` (evidence: `0x0042CC79..0x0042CCB8`; Active in YR: Yes)
- `[RESOLVED] OQ-7 - Are exclusions whole zones? -> No, canonical undirected edge pairs.` (evidence: `0x0042C618..0x0042C664`; Active in YR: Yes)
- `[RESOLVED] OQ-8 - Does retry rebuild global zones? -> No evidence; verified writes are Pathfinder-local exclusion vectors.` (evidence: decompiles `0x0042CCD0`, `0x0042CF80`; Active in YR: Yes)
- `[RESOLVED] OQ-9 - What feeds cell A*? -> `Zone_precheck` path/marker arrays, especially level-0 marker array at `Pathfinder+0x40`, when hierarchy flag is true.` (evidence: `0x0042C887..0x0042C8CE`, `0x00429E94`; Active in YR: Yes)
- `[RESOLVED] OQ-10 - Does current Rust still ban whole zones? -> No, current `zone_search.rs` uses `BTreeSet<ZoneEdge>` and `exclude_corridor_edges`; older docs are stale.` (evidence: `src/sim/pathfinding/zone_search.rs:37`, `:325`, `:591`)
- `[RESOLVED] OQ-11 - Does current Rust implement full retry integration? -> No, it lacks binary `UpdateHierarchicalEdges`/`FloodFillReachableZones` exclusion generation and full three-level precheck.` (evidence: `zone_search.rs:325..356`, `:519..583`)
- `[DEFERRED] OQ-12 - Exact stock-map route after low bridge collapse.` (category: needs-runtime-debugger; reason: requires runtime graph/path capture; next-step-if-pursued: instrument one low-bridge map before/after collapse)
- `[DEFERRED] OQ-13 - Full `MapClass__FloodFillReachableZones` internals.` (category: out-of-scope; reason: this slot only needed retry caller contract; next-step-if-pursued: use `MAPCLASS_FLOODFILLREACHABLEZONES_005840C0_GHIDRA_REPORT.md` or verify separately)
- `[DEFERRED] OQ-14 - Full A* cell marker/cost parity.` (category: out-of-scope; reason: this slot only verified marker handoff; next-step-if-pursued: dedicated `AStar_main_loop` corridor gate trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Same-zone initial `Zone_precheck` failure disables hierarchy and still runs cell A*, but cross-zone initial failure with hierarchy enabled returns zero before A*. | `0x0042CB22..0x0042CB86`; Active in YR: Yes. | partially present for flat paths; layered pathing only has reachability precheck. | `src/sim/pathfinding/zone_search.rs::find_path_zoned_marker`, `find_layered_path_zoned_marker` | Preserve same-zone fallback and cross-zone abort when replacing reduced reachability with binary-style precheck. | Same-zone graph/precheck false negative still finds a direct cell path; cross-zone disconnected precheck returns `None` without invoking A*. Proposed test name: `zone_precheck_same_zone_failure_disables_hierarchy_cross_zone_aborts`. | Do not "simplify" all precheck failures into either unconditional abort or unconditional full A*. |
| Default retry budget is five total attempts; failed hierarchical attempts append search-local undirected edge exclusions, reset A* state, and rerun `Zone_precheck`. | `0x0042CB8B..0x0042CBA1`, `0x0042CC79..0x0042CCB8`; Active in YR: Yes. | partial: current Rust uses five corridor attempts and `ZoneEdge`, but derives exclusions from the whole failed corridor rather than `UpdateHierarchicalEdges`/flood-fill/path-edge invalidation. | `src/sim/pathfinding/zone_search.rs` retry state; future binary-style zone precheck adapter | Keep exclusions local to one search, do not clear them on retry reset, and generate/consume only canonical edge exclusions. | A graph with route A-B-C failing at cell A* excludes A-B for the next precheck but leaves B usable through D-B-C; cap is five total attempts. Proposed test name: `zoned_retry_keeps_edge_exclusions_across_five_total_attempts`. | Do not implement "five retries after the first"; do not rebuild `ZoneGrid` as the retry mechanism. |
| Cell A* receives `Zone_precheck` path/marker output, not a free one-ring-expanded corridor. | `0x0042C887..0x0042C8CE`, `0x00429E94..0x00429F04`; Active in YR: Yes. | mismatch: Rust expands corridors by one neighbor ring and uses single-level centroid corridor. | `src/sim/pathfinding/zone_search.rs::find_zone_corridor`, `expand_corridor`, cell A* corridor filter | Replace one-ring expansion with a binary-style hierarchy marker/corridor contract once hierarchy data exists. | Fine-level graph has an off-corridor neighbor that Rust's one-ring expansion allows; binary-style marker path rejects it unless separately allowed by cell predicates. Proposed test name: `zone_precheck_cell_astar_uses_marked_path_without_one_ring_expansion`. | Do not claim one-ring expansion is parity; keep it only as an explicitly labeled approximation until removed. |

### Negative Facts / Do Not Do

- Do not say current Rust still excludes whole zones. That was true of older docs, but current `zone_search.rs` uses `ZoneEdge` and `exclude_corridor_edges`. Active in YR comparison: N/A Rust scan; evidence `src/sim/pathfinding/zone_search.rs:41..56`, `:591..602`.
- Do not say binary retry rebuilds the zone graph. Active in YR: Yes; evidence `0x0042CC79 -> 0x0042CCD0`, producer writes Pathfinder-local vectors.
- Do not run cell A* on initial cross-zone hierarchy-enabled mismatch. Active in YR: Yes; evidence `0x0042CB32..0x0042CB3F`.
- Do not clear retry exclusions when `PathfinderClass__Reset` is called between attempts. Active in YR: Yes; entry vector clear is separate from retry reset.
- Do not model default retry budget as one initial attempt plus five retries. Active in YR: Yes; default is five total attempts.

### Stale Docs / Follow-up Docs

- `docs/research/ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`
  - Replace: "Current code excludes whole zones (`BTreeSet<ZoneId>`) after corridor failure"
  - With: "Current Rust now stores canonical `ZoneEdge` exclusions and excludes corridor edges after failed corridor A*. It remains non-parity because it derives exclusions from the failed Rust corridor, not from gamemd's `UpdateHierarchicalEdges` / `FloodFillReachableZones` / stored-zone-path invalidation contract."
- `docs/research/ZONE_REBUILD_ASTAR_RETRY_HELPERS_0056C510_0042C290_GHIDRA_REPORT.md`
  - Replace: "`zone_search.rs` stores `BTreeSet<ZoneId>` and extends it with every corridor zone after failure."
  - With: "`zone_search.rs` now stores `BTreeSet<ZoneEdge>` and excludes corridor edges, but still lacks the binary producer semantics that choose which edge(s) to append after failed hierarchical A*."
- `docs/research/PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`
  - Replace the integration table line: "1 attempt when `param_6 == -1`, 5 when `param_6 != -1`."
  - With: "5 total attempts when `param_6 == -1`, 1 total attempt when `param_6 != -1`."

## Sources

- Ghidra decompiled this slot: `0x0042C900`, `0x0042C290`, `0x0042CCD0`, `0x0042CF80`, `0x0042A5B0`, `0x004CBBA0`, `0x00429A90`.
- Prior docs referenced: `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`, `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md`.
- Rust scanned: `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_map.rs`.

Status: COMPLETE for the scoped retry/precheck integration contract.
