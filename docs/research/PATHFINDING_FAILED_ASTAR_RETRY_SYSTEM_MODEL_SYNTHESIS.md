# Pathfinding Failed-A* Retry System Model Synthesis

**Date:** 2026-07-18  
**System:** flat hierarchical pathfinding retry  
**Mode:** model-synthesis  
**Status:** PARTIAL_READY — mechanism substrate is implementation-safe; production
activation is blocked on an exact runtime `Can_Enter_Cell` result surface.

## Scope

Included:

- The standard-YR `AStar_pathfind_search @ 0x0042C900` hierarchy-assisted failure
  loop.
- `Zone_precheck @ 0x0042C290` result reuse, search-local exclusions, the tracked
  hierarchy-progress cell, `UpdateHierarchicalEdges @ 0x0042CCD0`,
  `FloodFillReachableZones @ 0x005840C0`, and `InvalidateZoneEdge @ 0x0042CF80`.
- The eligible flat Rust hierarchy path in `zone_search.rs`, `zone_hierarchy.rs`,
  and `core.rs`.

Explicit non-scope:

- Layered/high-bridge entry, hierarchy construction, slope contribution, full A*
  edge-cost parity, blocked-destination fallback, COM locomotor pathing, and the
  complete hierarchy-enable predicate.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safety |
|---|---|---|---|---|---|
| Ordinary foot pathing reaches `0x0042C900` with default `-1` search limit. | `0x004CBBA0 -> 0x004CBC31`; retry reports | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Default limit permits five total A* attempts; non-default permits one. | `0x0042CB8B..0x0042CBA1`; live decompile `0x0042C900` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Exclusion lists clear once at outer search entry and survive retry `Reset`. | `0x0042C912..0x0042C925`, `0x0042CC79..0x0042CC80` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Retry exclusions are ordered, duplicate-preserving, per-level canonical undirected edge records. | `0x0042C620..0x0042C664`, `0x0042D830`, `0x0042D0FA..0x0042D13C` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| A failed hierarchical attempt uses the furthest accepted next-level-0-path-zone cell, or the start cell if no crossing was accepted. | `0x00429BCD..0x00429BE6`, `0x0042A159..0x0042A178`; live decompile `0x00429A90` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Retry update runs before Reset, then validity/budget is checked, then precheck is rerun when hierarchy remains valid. | live decompile `0x0042C900`; `0x0042CC79..0x0042CCB3` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| If invalidation clears hierarchy validity and budget remains, the next attempt runs without hierarchy; it is not an unconditional immediate return. | live decompile `0x0042C900`, retry tail | confirmed; older wording stale | high | conditional | IMPLEMENTATION_SAFE |
| The producer runs levels 0, 1, 2 from the same tracked progress cell. | `0x0042CCD8..0x0042CE93` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `0x005840C0` local bookkeeping executes when `Can_Enter_Cell == 0` **or** matrix value is not `1`. | live decompile `0x005840C0`; `0x00584271..0x00584286` | confirmed; older polarity contradicted | high | yes | IMPLEMENTATION_SAFE |
| Return `1` from `0x005840C0` selects stored-path invalidation; return `0` appends current-zone edges to graph neighbors absent from the local observed vector. | `0x005843D0..0x0058451B`, `0x0042CD85..0x0042CE4E` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Current Rust can supply exact runtime `Can_Enter_Cell` zero/nonzero values to this helper. | `cell_entry.rs:12-18,449-455`; path-search signature | contradicted | high | N/A | UNSAFE_FOR_IMPLEMENTATION |

## Current Model

At outer search entry, gamemd initializes hierarchy validity, resets scratch state,
and clears three per-level exclusion vectors exactly once. An eligible hierarchical
search runs `Zone_precheck`, which stores selected paths and level-0 marker state,
then runs cell A* with marker gating.

Each A* attempt seeds a hierarchy-progress cell from the start. It advances that
cell only after an accepted/generated neighbor enters the next selected level-0
path zone. On failure, `UpdateHierarchicalEdges` derives the current zone at all
three hierarchy levels from that one cell.

For each level, `FloodFillReachableZones` examines a `2`, `4`, or `8` cell block.
Its local bookkeeping branch is reached when the mover's `Can_Enter_Cell` result is
zero (YR code `Clear`) or the movement-zone matrix value is not `1`. A remaining
unvisited same-zone cell makes it return `1`; otherwise it returns `0` after
collecting graph neighbors absent from its locally observed different-zone list.

- Return `1`: invalidate one stored-path edge adjacent to the current zone, then
  append the verified common-neighbor exclusions.
- Return `0`: append current-zone-to-returned-neighbor exclusions in reverse helper
  vector order.

The producer appends to ordered per-search vectors without deduplication. Reset
clears A* scratch and advances marker epochs but preserves exclusions and selected
paths. When budget remains, a valid retry reruns `Zone_precheck`; an invalidated
hierarchy runs one following A* attempt without hierarchy. Default search performs
at most five total A* attempts.

## Implementation-Safe Facts

- Rust already has the correct ownership shape: search-local state in the path
  call, immutable hierarchy graphs, and no persistent `ZoneGrid` mutation.
- `ZonePrecheckExclusions` already combines canonical membership lookup with an
  ordered duplicate-preserving producer ledger (`zone_hierarchy.rs:240-282`).
- `ZonePrecheckResult` already retains all three selected paths and marker sets
  (`zone_hierarchy.rs:285-299`).
- `HierarchyProgressTracker` already initializes from the start and advances only
  on accepted next-path-zone neighbors (`core.rs:314-349,1306-1322`).
- The flat hierarchy branch already implements same-zone precheck fallback,
  cross-zone failure, and marker-gated A* (`zone_search.rs:278-330`).
- Safe new substrate: an injected-input pure producer plus an attempt outcome that
  preserves the progress cell on failure.

## Current Rust Delta

- `find_path_with_costs_hierarchy_marker_progress` returns `Option`; `?` discards
  the progress cell when A* fails (`core.rs:2361-2406`).
- The flat hierarchy branch constructs empty exclusions, performs one precheck and
  one A* attempt, and returns immediately (`zone_search.rs:278-330`).
- No `FloodFillReachableZones`-equivalent producer is wired.
- The compatibility branch retries five times but excludes every Rust corridor edge
  and uses a centroid-cost approximation; it is not the native producer
  (`zone_search.rs:427-466,620-709`).
- The current path-search interface lacks the mover, occupancy, alliance, and exact
  cell-entry context needed to reproduce the virtual `Can_Enter_Cell` call.

## Doc-Patch-Ready Facts

- Any report saying retry validity `+0x38 == 0` always returns immediately after
  update/reset is stale. Live `decompile_function 0x0042C900` shows that, when the
  attempt budget remains, the loop calls the next A* with hierarchy disabled.
- Any report saying `0x005840C0` floods on `Can_Enter_Cell != 0 && matrix == 1`
  has inverted the branch. Live `decompile_function 0x005840C0` confirms local
  bookkeeping on `result == 0 || matrix != 1`.
- Reports saying Rust lacks progress tracking are stale; the tracker now exists,
  although failure still discards its value.

## Stale Or Superseded Claims

- The narrow passability-polarity wording in
  `UPDATEHIERARCHICALEDGES_FAILED_ASTAR_EDGE_SELECTION_GHIDRA_REPORT.md` is
  superseded by `ZONEMAP_FLOODFILLREACHABLEZONES_RETRY_PRODUCER_GHIDRA_REPORT.md`
  and the 2026-07-18 live decompile.
- The immediate-return wording in older retry summaries is superseded by the
  2026-07-18 live `0x0042C900` retry-tail check.
- Older Rust-status tables predate `HierarchyProgressTracker` and the ordered
  exclusion ledger.

## Cross-Doc Conflicts

No unresolved mechanism conflict remains inside this bounded slice. Two conflicts
were resolved by live decompilation as listed above.

## Needs Re-Investigation

- Exact runtime adapter for the `0x005840C0` mover virtual `Can_Enter_Cell` call:
  current Rust's cell-entry classification explicitly retains approximation
  boundaries and the zoned path signature lacks the required mover context.
- The full hierarchy-enable predicate remains outside this conditional-path model.

Recommended command:

`/re-investigate 0x005840C0 Can_Enter_Cell zero/nonzero runtime adapter for flat standard-YR foot pathing`

## Do-Not-Implement Notes

- Do not rebuild or mutate persistent zone graphs during a retry.
- Do not exclude whole zones or every edge in the chosen path/corridor.
- Do not deduplicate producer appends or sort common neighbors.
- Do not use the last popped A* node as the retry source.
- Do not activate the producer with `PathGrid` walkability as a substitute for the
  mover's exact `Can_Enter_Cell` result.
- Do not expand this slice into layered bridges, slope costs, or hierarchy building.

## Source Ledger

- `docs/research/pathfinding/ASTAR_RETRY_RESET_EXCLUSION_LIFETIME_GHIDRA_REPORT.md`
- `docs/research/pathfinding/ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`
- `docs/research/pathfinding/PATHFINDER_FAILED_ASTAR_CURRENT_ZONE_SOURCE_GHIDRA_REPORT.md`
- `docs/research/pathfinding/PATHFINDER_INVALIDATEZONEEDGE_COMMON_NEIGHBORS_GHIDRA_REPORT.md`
- `docs/research/pathfinding/PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`
- `docs/research/ZONEMAP_FLOODFILLREACHABLEZONES_RETRY_PRODUCER_GHIDRA_REPORT.md`
- `docs/research/pathfinding/ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`
- Live read-only Ghidra decompiles on 2026-07-18: `0x00429A90`, `0x0042C900`,
  `0x005840C0`.
- Current Rust: `src/sim/pathfinding/zone_search.rs`, `zone_hierarchy.rs`,
  `core.rs`, and `cell_entry.rs`.
