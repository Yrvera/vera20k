# AStar_pathfind_search 0x0042C900 Retry Semantics

Date: 2026-05-21

Target: `AStar_pathfind_search @ 0x0042C900`, retry loop, same-zone vs cross-zone precheck failure behavior, and where per-search edge exclusions are appended/consumed.

Investigation mode: exhaustive-slice.

Evidence note: the Ghidra MCP tools were not exposed in this session. Findings below are from direct Capstone disassembly of the retail `gamemd.exe` at `<ra2-install>/gamemd.exe`, cross-checked against existing Ghidra reports. No Ghidra mutation was possible or performed.

## Scope Boundaries

In scope:

- `AStar_pathfind_search @ 0x0042C900`.
- Retry loop call boundaries into `AStar_main_loop @ 0x00429A90`, `UpdateHierarchicalEdges @ 0x0042CCD0`, `PathfinderClass__Reset @ 0x0042A5B0`, and `Zone_precheck @ 0x0042C290`.
- Per-search edge-exclusion list clear, append, and consumption sites.

Out of scope:

- `Zone_precheck` internal pathfinding details beyond call-boundary and edge-exclusion consumption.
- Full semantics of `MapClass::FloodFillReachableZones @ 0x005840C0`.
- Blocked-destination fallback threshold and `Find_Nearby_Passable_Cell`.
- CellRect validators, Infantry `Can_Enter_Cell`, MovementZone parser mapping, and zone-edge byte+4 writer semantics.

## Open Questions Log

- [RESOLVED] Entry point - `0x0042C900` starts with `sub esp,0x20`, sets `Pathfinder+0x38=1`, calls Reset, and clears three vectors. Evidence: `0x0042C900..0x0042C927`.
- [RESOLVED] Retry count - max attempts are 5 only when arg5/max-depth is `-1`; otherwise max attempts are 1. Evidence: `0x0042CB8B..0x0042CBA1`.
- [RESOLVED] Retry gate - retries happen only while the hierarchy flag local remains true. Evidence: failed A* at `0x0042CC07..0x0042CC19`.
- [RESOLVED] Search reset - `PathfinderClass__Reset` is called before first attempt and after every failed hierarchical attempt. Evidence: `0x0042C90D`, `0x0042CC79..0x0042CC80`.
- [RESOLVED] Edge-list lifetime - the three exclusion vectors are cleared once at search entry, before retries. Evidence: `0x0042C912..0x0042C925`; no vector clear on the retry reset path at `0x0042CC79..0x0042CC80`.
- [RESOLVED] Same-zone initial precheck failure - same source/destination zone plus active hierarchy calls `Zone_precheck`; if it returns 0, a diagnostic is printed and hierarchy is disabled for the A* attempt. Evidence: `0x0042CB2A..0x0042CB86`.
- [RESOLVED] Cross-zone initial failure - different source/destination zones plus active hierarchy returns 0 immediately, before `Zone_precheck` or `AStar_main_loop`. Evidence: `0x0042CB2A..0x0042CB3F`.
- [RESOLVED] Cross-zone hierarchy disabled - if hierarchy flag is false, different zones do not hard-fail; the function logs "A* without HS" when coordinates differ and calls cell A*. Evidence: `0x0042CB8B..0x0042CBE6`.
- [RESOLVED] Post-retry precheck failure - after `UpdateHierarchicalEdges` and `Reset`, failure of `Zone_precheck` exits with current result 0. Evidence: `0x0042CCA1..0x0042CCC0`.
- [RESOLVED] Edge exclusion consumption - `Zone_precheck` packs `(min(cur_zone,next_zone)<<16)|max(...)`, scans the level vector backward, and skips the edge on match. Evidence: `0x0042C620..0x0042C666`.
- [RESOLVED] Edge exclusion append - `UpdateHierarchicalEdges` and helper `0x0042CF80` append sorted undirected pairs into the same per-level vectors. Evidence: `0x0042CE13..0x0042CE4E`, `0x0042D05D..0x0042D06D`, `0x0042D0FA..0x0042D13C`.
- [DEFERRED] Exact meaning of `MapClass::FloodFillReachableZones @ 0x005840C0` result - call-boundary only; internals out of scope.
- [DEFERRED] Exact semantic name of the local hierarchy-enable flag - bounded-cost-too-high for this slot; branch behavior is verified, name remains inferred.
- [DEFERRED] COM path at `0x0042CA4F..0x0042CABC` - out of scope; parent context already listed `TechnoType+0xD94` as a separate open question.

## Verified Binary Findings

### 1. Search Entry Initializes Per-Search State Once

At `0x0042C909`, `AStar_pathfind_search` writes `Pathfinder+0x38 = 1` before the first `Reset` call at `0x0042C90D`.

Immediately after Reset, it clears exactly three vector-like objects starting at `Pathfinder+0x74`, stepping by `0x18`, by loading each vtable and calling slot `+0x0C`: `0x0042C912..0x0042C925`.

Active in YR: Yes. Evidence: `FootClass::Run_AStar` is an established live caller in `PATHFINDING_ASTAR_GHIDRA_REPORT.md` and the disassembled function has no TS/fog/SpecialFlags gate before this setup.

### 2. Retry Count Is Attempts, Not Extra Retries

The expression at `0x0042CB8B..0x0042CBA1` computes:

- arg at stack slot `+0x44 == -1` -> `5`
- any other value -> `1`

This value is stored as the loop limit at local `[esp+0x1C]`. The failure path increments the attempt counter at `0x0042CC71..0x0042CC79`, then compares `attempts_done >= limit` at `0x0042CC91..0x0042CC97`.

Therefore the default path runs at most 5 total `AStar_main_loop` calls, not one initial try plus five retries. A specified max-depth/search-limit parameter disables retry and allows one total call.

Active in YR: Yes. Evidence: retry expression is inside the live `AStar_pathfind_search` body and reaches `AStar_main_loop @ 0x00429A90` at `0x0042CC02`.

### 3. Retry Requires Hierarchy To Remain Enabled

After `AStar_main_loop`, the function tests the result at `0x0042CC07..0x0042CC0D`. If the result is nonzero, it returns success.

If the result is zero, it tests the local hierarchy flag at `0x0042CC13..0x0042CC19`. If the flag is false, it returns the zero result immediately. Only when the flag is true does it call `UpdateHierarchicalEdges @ 0x0042CCD0`, then `Reset @ 0x0042A5B0`, then possibly `Zone_precheck` again.

Active in YR: Yes. Evidence: no TS-only gate on this branch; it is the ordinary failure path after a live A* call.

### 4. Same-Zone And Cross-Zone Initial Failures Differ

At `0x0042CB22..0x0042CB34`, the function compares the start and destination zone IDs.

If zones differ and the hierarchy flag is true, it returns 0 immediately at `0x0042CB36..0x0042CB3F`. This is a cross-zone hard failure before cell A*.

If zones are the same and the hierarchy flag is true, it calls `Zone_precheck @ 0x0042C290` at `0x0042CB46..0x0042CB58`. If precheck fails, it prints the string at `0x00818820` ("Hierarchical findpath failure...") and clears the local hierarchy flag at `0x0042CB86`; then it continues to cell A* without hierarchy.

If the hierarchy flag is false, the zone comparison is not a hard gate. The function can print the string at `0x008187F0` ("Warning. A* without HS...") and still call cell A*.

Active in YR: Yes. Evidence: all branches are in the live function body after ordinary source/destination zone lookup and before `AStar_main_loop`.

### 5. Post-Update Precheck Failure Aborts The Retry Loop

On a failed hierarchical A* attempt, the function calls:

- `UpdateHierarchicalEdges @ 0x0042CCD0` at `0x0042CC79`
- `PathfinderClass__Reset @ 0x0042A5B0` at `0x0042CC80`

It then checks `Pathfinder+0x38` at `0x0042CC85..0x0042CC97`. If that flag is false, or if the attempt limit is reached, it exits with zero.

If another attempt is still allowed and `Pathfinder+0x38` remains true, it reruns `Zone_precheck` at `0x0042CCA1..0x0042CCB8`. A nonzero precheck result jumps back to the A* attempt at `0x0042CBA5`; a zero precheck result falls through to return zero at `0x0042CCC0`.

Active in YR: Yes. Evidence: direct failure path from live `AStar_main_loop`.

### 6. Exclusion Vectors Are Per Search But Persist Across Retries

The three per-level vectors are cleared once at search entry (`0x0042C912..0x0042C925`). The retry reset path calls `Reset` (`0x0042CC80`) but does not clear these vectors. Therefore exclusions appended by failed attempts remain visible to the next `Zone_precheck` in the same `AStar_pathfind_search` call.

The next search call clears them again at entry, so they are per-search, not persistent world state.

Active in YR: Yes. Evidence: live entry clear and live retry path; no TS gate.

### 7. `Zone_precheck` Consumes Undirected Edge Exclusions, Not Whole-Zone Bans

Inside `Zone_precheck`, after ordinary edge filters pass, the code at `0x0042C620..0x0042C666` checks the per-level vector:

- reads current and neighbor zone IDs,
- sorts them by numeric value,
- packs them as `(min << 16) | max`,
- reads the level vector count from `Pathfinder + level*0x18 + 0x84`,
- reads the vector data pointer from `Pathfinder + level*0x18 + 0x78`,
- scans backward,
- and skips only that edge when a packed pair matches.

This is not a zone ban. The same zone may still be considered through other edges.

Active in YR: Yes. Evidence: direct `Zone_precheck` disassembly on the live precheck path.

### 8. `UpdateHierarchicalEdges` Appends Sorted Edge Pairs

`UpdateHierarchicalEdges @ 0x0042CCD0` loops over three levels (`ebp=0..2`, `0x0042CE83..0x0042CE93`). The loop pointer starts at `Pathfinder + 0x7c` (not `+0x74`) and steps by `+0x18` per level — i.e., `piVar11 = param_1 + 0x7c; piVar11 += 6` (corrected 2026-05-29: was "vector base `Pathfinder + 0x74 + level*0x18`"; binary shows loop pointer at `+0x7c`, while the struct base `+0x74` is where the vtable field lives and where the main entry clear-loop and `InvalidateZoneEdge` anchor; ROOT_CAUSE: MISLEADING; via decompile_function 0x0042CCD0 + decompile_function 0x0042CF80). The consumed count/data offsets still align with `Zone_precheck`'s `+0x84/+0x78` because `piVar11[2]` = `+0x7c+8 = +0x84` (count) and `piVar11[-1]` = `+0x7c-4 = +0x78` (data pointer).

Append sites:

- `0x0042CE13..0x0042CE4E`: direct append of a sorted pair from the level's local zone list into the level vector.
- `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80` helper (corrected 2026-05-29: was unnamed "helper `0x0042CF80`"; Ghidra labels it `PathfinderClass__InvalidateZoneEdge` — confirmed via get_function_by_address 0x0042CF80; single caller confirmed via get_function_callers 0x0042CF80 — ROOT_CAUSE: STALE), called only from `0x0042CDAC`, validates that the supplied zone appears in the stored `Zone_precheck` path for that level. If path length is <=1 or the zone is absent, it clears `Pathfinder+0x38` and returns (`0x0042CF8D..0x0042CFEA`).
- `0x0042D05D..0x0042D06D`: appends the sorted pair between the matched path zone and the adjacent path zone chosen by index (`previous` when the matched node is last, otherwise `next`).
- `0x0042D0FA..0x0042D13C`: appends additional sorted pairs discovered from adjacency scans around that path edge.

Active in YR: Yes. Evidence: `UpdateHierarchicalEdges` is called directly from the live retry path at `0x0042CC79`; helper `0x0042CF80` has a single direct call at `0x0042CDAC`.

## Current Rust Implication

`src/sim/pathfinding/zone_search.rs` currently models retry as corridor retry with whole-zone exclusions and then unrestricted fallback. That is not the verified gamemd contract for this slice:

- gamemd appends undirected edge exclusions, not whole-zone exclusions;
- exclusions persist across the default five total attempts of one search call;
- same-zone initial hierarchy failure falls back to non-hierarchical A*;
- cross-zone initial hierarchy failure returns 0 when hierarchy is enabled;
- there is no unconditional unrestricted fallback after exhausting default hierarchical retries.

## Implementation Handoff

1. Verified behavior: default search allows at most five total A* attempts, with edge exclusions accumulating across retries and a `Zone_precheck` rerun before each retry. Rust delta: replace whole-zone retry exclusion with per-search undirected edge exclusions and a total-attempt limit of 5 only for the `-1`/default limit mode. Affected surface: `src/sim/pathfinding/zone_search.rs` and any zone-precheck adapter. Acceptance scenario: first corridor edge fails, appends that edge, second precheck avoids only that edge and can still use the same zones via another edge. Proposed test name: `zone_search_retries_append_edge_exclusion_not_zone_ban`. Risk: high player visibility on destroyed bridges and narrow multi-zone corridors.

2. Verified behavior: same-zone initial `Zone_precheck` failure logs and disables hierarchy, then still runs cell A*; cross-zone initial failure with hierarchy enabled returns 0 before cell A*. Rust delta: split same-zone fallback from cross-zone hard failure instead of treating all precheck failures alike. Affected surface: `src/sim/pathfinding/zone_search.rs`. Acceptance scenario: same-zone graph inconsistency still finds a direct cell path, but cross-zone disconnected graph returns `None` without invoking A*. Proposed test name: `zone_search_same_zone_precheck_failure_falls_back_cross_zone_aborts`. Risk: medium; same-zone false negatives otherwise make units refuse reachable local moves.

3. Verified behavior: exhausting default hierarchical attempts returns failure; it does not perform a final unrestricted A* fallback unless hierarchy was disabled earlier by the same-zone initial failure path. Rust delta: remove or gate the unconditional final unrestricted fallback currently documented in `zone_search.rs`. Affected surface: `src/sim/pathfinding/zone_search.rs`. Acceptance scenario: a cross-zone route whose hierarchical retry attempts all fail returns `None` after five total attempts, while a same-zone initial precheck failure can still call unrestricted A*. Proposed test name: `zone_search_no_unrestricted_fallback_after_hierarchical_retry_exhaustion`. Risk: medium; unrestricted fallback can route through graph-disallowed bridge or blocked-zone states that gamemd refuses.

## Negative Facts / Do Not Do

- Do not model retry exclusions as whole-zone bans. Evidence: `Zone_precheck` packs and searches only sorted pairs at `0x0042C620..0x0042C666`.
- Do not clear edge exclusions on every `Reset`. Evidence: vectors are cleared at entry `0x0042C912..0x0042C925`; retry `Reset` at `0x0042CC80` does not clear them.
- Do not implement default retry as "initial attempt plus five retries." Evidence: limit computation at `0x0042CB8B..0x0042CBA1` and failure counter compare at `0x0042CC71..0x0042CC97` yield five total attempts.
- Do not run cell A* for cross-zone hierarchy-enabled zone mismatch. Evidence: different zones plus hierarchy flag returns 0 at `0x0042CB32..0x0042CB3F`.
- Do not add an unconditional unrestricted final fallback after hierarchical retries. Evidence: when retry limit is reached or post-update `Zone_precheck` fails, control falls to return current zero result at `0x0042CCC0..0x0042CCCB`.

## Remaining Uncertainty

- Exact semantic name of the local hierarchy-enable flag remains inferred; its branch behavior is verified.
- Exact internals of `MapClass::FloodFillReachableZones @ 0x005840C0` remain out of scope.
- The COM path gated by `TechnoType+0xD94` at `0x0042CA4F..0x0042CABC` was not resolved in this slice.

## Stale-Doc Replacement Wording

### `docs/research/PATHFINDING_ASTAR_GHIDRA_REPORT.md`

Replace the simplified AStar_pathfind_search retry wording with:

> `AStar_pathfind_search @ 0x0042C900` clears the three per-search edge-exclusion vectors once at search entry, then runs at most five total `AStar_main_loop` attempts when the caller passes the default `-1` search limit, or one total attempt otherwise. A failed hierarchical attempt calls `UpdateHierarchicalEdges @ 0x0042CCD0`, appends per-level sorted undirected zone-edge exclusions, calls `PathfinderClass__Reset`, and reruns `Zone_precheck`; the exclusion vectors are not cleared by this retry reset. Same-zone initial `Zone_precheck` failure disables hierarchy and still runs cell A*, but cross-zone mismatch with hierarchy enabled returns 0 before cell A*. Exhausting hierarchical retries returns failure; there is no unconditional final unrestricted A* fallback.

### `docs/research/PATHFINDERCLASS_GHIDRA_REPORT.md`

Replace retry mechanism lines that say "max retries: 5" or imply `UpdateHierarchicalEdges` invalidates whole zones with:

> The default retry budget is five total A* attempts, not five retries after the first attempt. Retry state is per-search: the three edge-exclusion vectors at `Pathfinder+0x74/+0x8C/+0xA4` are cleared once at entry and survive `Reset()` between attempts. `UpdateHierarchicalEdges` appends sorted undirected zone-edge pairs, and `Zone_precheck` skips only matching edges via the `+0x78/+0x84` vector data/count for each level.

### `docs/research/BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`

Add to the `Zone_precheck`/AStar interaction section:

> The edge exclusions consumed by `Zone_precheck` are appended by the `AStar_pathfind_search` retry path (`UpdateHierarchicalEdges @ 0x0042CCD0` and helper `0x0042CF80`) and persist only within the current search call. They are not global bridge-edge removals and not whole-zone exclusions.

## Coverage Ledger

- `AStar_pathfind_search @ 0x0042C900`: covered for setup, precheck branching, retry loop, retry count, and return behavior.
- `Zone_precheck @ 0x0042C290`: covered only for edge-exclusion consumption and call-boundary behavior.
- `UpdateHierarchicalEdges @ 0x0042CCD0`: covered only for per-level loop and append sites.
- `0x0042CF80` helper: covered for search-valid failure conditions and append sites.
- `MapClass::FloodFillReachableZones @ 0x005840C0`: deferred, call-boundary only.

Status: COMPLETE for the scoped retry semantics and edge-exclusion append/consume contract.
