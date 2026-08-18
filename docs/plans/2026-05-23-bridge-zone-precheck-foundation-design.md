# Bridge Zone Precheck Foundation Design

## Goal

Build the first binary-style `Zone_precheck` foundation for flat/no-slope synthetic pathing, while explicitly deferring full high-bridge route parity, exact retry producer parity, slope context parity, and stock-route assertions.

## Architecture Context

Rust pathfinding currently uses a one-level approximation:

- `src/sim/pathfinding/zone_map.rs` owns `ZoneGrid`, one `ZoneMap` and one `ZoneAdjacency` per `MovementZone`, plus `SuperZoneMap` connected-component reachability.
- `src/sim/pathfinding/zone_build.rs` builds the one-level zone map, extracts adjacency, and injects bridge adjacency afterward.
- `src/sim/pathfinding/zone_hierarchy.rs` is currently only union-find reachability, not gamemd's three-level hierarchy.
- `src/sim/pathfinding/zone_search.rs` runs reduced reachability, `find_zone_corridor`, `expand_corridor`, and corridor-restricted A*.
- `src/sim/pathfinding/core.rs` receives `AStarOptions.corridor` as an allowed-zone set and filters candidate cells by `zone_map.zone_at(...)`.

The verified gamemd shape is different:

- `ZoneMap__BuildZoneLevel` builds three hierarchy levels in order `2 -> 1 -> 0`.
- Each level has zone records with parent/coarser zone id, reduced zone type, ordered edge records, and edge flag metadata.
- `Zone_precheck` searches those three levels, writes per-level selected paths, and stamps marker arrays.
- `AStar_main_loop` gates cell expansion with the level-0 marker array, not with Rust's one-ring expanded corridor.
- `AStar_main_loop` also allows off-marker cells when `CellClass+0x122 != 0`, the blocker-neighbor refcount exception.
- Exact retry-edge producer, exact slope-cost context, layered high-bridge integration, and stock Carville route oracle are separate follow-ups.

## Impact Analysis

Likely touched files:

- `src/sim/pathfinding/zone_map.rs`
- `src/sim/pathfinding/zone_build.rs`
- `src/sim/pathfinding/zone_hierarchy.rs`
- `src/sim/pathfinding/zone_search.rs`
- `src/sim/pathfinding/core.rs`
- `src/sim/pathfinding/zone_search_tests.rs`
- `src/sim/pathfinding/zone_map_tests.rs`
- `src/sim/pathfinding/core_tests.rs`

Risk areas:

- Shared pathfinding behavior and deterministic route choice.
- Replacing `expand_corridor()` too aggressively could make reachable paths fail unless the `+0x122` exception is modeled.
- The first slice must not silently claim high-bridge player-route parity because current layered pathing does not consume binary precheck markers yet.
- The first slice must not claim automatic retry parity because exact `UpdateHierarchicalEdges` producer semantics are deferred.

## Chosen Approach

Use **Foundation First v2**.

This means implementing the binary-style hierarchy/precheck data model and flat/no-slope synthetic precheck behavior first, with explicit labels around what it does and does not prove.

The first implementation slice should include:

- three-level hierarchy data structures;
- ordered edge records with flag byte;
- reduced zone type and parent/coarser zone fields;
- `Zone_precheck`-style search over levels `2 -> 1 -> 0`;
- per-level selected path storage and level-0 marker output;
- manual/preseeded per-level edge exclusions for tests;
- A* marker gating for flat ground paths;
- blocker-neighbor exception equivalent to `CellClass+0x122 != 0`;
- same-zone failure clears hierarchy while cross-zone failure aborts.

The first slice must defer:

- exact `UpdateHierarchicalEdges` / `FloodFillReachableZones` retry producer;
- exact `Zone_Estimate_Slope_Cost` context and sloped route parity;
- layered bridge path marker/retry integration;
- exact stock Carville route assertions;
- high-bridge player-visible route parity claims.

## Tiny-Detail Ledger

- Hierarchy level order is `2 -> 1 -> 0`. Source: `BRIDGE_ZONE_PRECHECK_HIERARCHY_WRITER_ORDER_GHIDRA_REPORT.md`, `0x005671F7..0x00567218`.
- Level block sizes are `1 << (level + 1)`: level 2 is 8x8, level 1 is 4x4, level 0 is 2x2. Source: `BRIDGE_ZONE_PRECHECK_HIERARCHY_WRITER_ORDER_GHIDRA_REPORT.md`.
- Lower levels are parent-gated by the chosen coarser path; reduced zone type `1` bypasses the parent gate. Source: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `0x0042C5F0..0x0042C604`.
- Zone records carry parent/coarser id at `+0x18` and reduced zone type at `+0x1C`. Source: `BRIDGE_ZONE_PRECHECK_HIERARCHY_WRITER_ORDER_GHIDRA_REPORT.md`; consumer reads `0x0042C554`, `0x0042C55C`.
- Final edge records are ordered arrays, not sorted by `ZoneId`. Source: `BRIDGE_ZONE_PRECHECK_HIERARCHY_WRITER_ORDER_GHIDRA_REPORT.md`, `0x00582395..0x00582480`.
- Final edge record shape is neighbor id plus flag dword; `byte(edge+4) != 0` adds `0.001` to precheck cost. Source: `BRIDGE_ZONE_EDGE_FLAG_WRITER_SEMANTICS_GHIDRA_REPORT.md`, `0x0042C540`, `0x0042C59E..0x0042C5AE`.
- Bridge/tube full-build and repaired-bridge direct edges are zero-flagged. Source: `BRIDGE_ZONE_EDGE_FLAG_WRITER_SEMANTICS_GHIDRA_REPORT.md`, `FUN_00582D70`, `MapClass__AddBridgeZoneEdges @ 0x005851B0`.
- First-slice flat/no-slope tests may defer slope if they use level 0, no mover, or a factor at/below the threshold. Source: `ZONE_ESTIMATE_SLOPE_COST_PARITY_GHIDRA_REPORT.md`, `0x00585F47..0x00585F4B`, `0x0042C2BA..0x0042C2FB`.
- `Zone_precheck` writes selected chains/counts at `Pathfinder+0xBC + level*1000` and `+0xC74 + level*4`. Source: `ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`, `0x0042C887..0x0042C8CE`.
- `AStar_main_loop` gates candidates by level-0 marker array. Source: `ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`, `0x00429E85..0x00429EA7`.
- Off-marker cells with `CellClass+0x122 != 0` are allowed; off-marker cells with `+0x122 == 0` are skipped only when hierarchy is enabled. Source: `ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`, `0x00429EB1..0x00429EC1`.
- `+0x122` is a blocker-neighbor refcount exception, not fog, water, bridge state, shroud, or ore-neighbor state. Source: `ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`; `CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md`.
- Same-zone initial precheck failure clears hierarchy and still runs cell A*; cross-zone hierarchy failure returns before cell A*. Source: `BRIDGE_ASTAR_PRECHECK_RETRY_INTEGRATION_GHIDRA_REPORT.md`, `0x0042CB22..0x0042CB86`.
- Default foot pathing uses five total attempts, not one plus five retries. Source: `BRIDGE_ASTAR_PRECHECK_RETRY_INTEGRATION_GHIDRA_REPORT.md`, `0x0042CB8B..0x0042CBA1`.
- Exact retry producer can be deferred from the first slice, but the precheck result must retain per-level paths because `InvalidateZoneEdge` later reads them. Source: `UPDATE_HIERARCHICAL_EDGES_RETRY_PRODUCER_SCOPE_GHIDRA_REPORT.md`, `0x0042CF8D`, `0x0042CFBA..0x0042CFE3`.
- Flat foundation must not claim high-bridge player-route parity until layered pathing consumes binary-style precheck markers/retry output. Source: `LAYERED_BRIDGE_PATH_ENTRY_FOUNDATION_SCOPE_GHIDRA_REPORT.md`.
- Carville waypoint `1=(79,50)` to `0=(49,87)` after CABHUT `(57,49)` / starter `(60,52):0x11380` is trace-ready only, not an exact route oracle. Source: `STOCK_LOW_BRIDGE_ROUTE_FIXTURE_READINESS_GHIDRA_REPORT.md`.

## Design

### Components

#### Zone Hierarchy Data

Add a hierarchy representation separate from the existing `SuperZoneMap` reachability cache.

Proposed types:

```rust
pub(crate) struct ZoneHierarchy {
    pub levels: [ZoneLevelGraph; 3],
}

pub(crate) struct ZoneLevelGraph {
    pub zones: Vec<ZoneRecord>,
    pub cell_zone_ids: Vec<ZoneId>,
    pub width: u16,
    pub height: u16,
}

pub(crate) struct ZoneRecord {
    pub parent: ZoneId,
    pub zone_type: u8,
    pub edges: Vec<ZoneEdgeRecord>,
}

pub(crate) struct ZoneEdgeRecord {
    pub neighbor: ZoneId,
    pub flag: u8,
}
```

Index `0` remains the sentinel zone. Real zones remain `1..`.

`ZoneGrid` should keep the current one-level `ZoneMap`/`ZoneAdjacency` surfaces for existing callers while adding optional hierarchy data for parity precheck:

```rust
hierarchy: BTreeMap<MovementZone, ZoneHierarchy>
```

The first implementation can build synthetic hierarchy fixtures directly in tests before a full production hierarchy builder exists. Production builder work can then fill the same types.

#### Hierarchy Builder Foundation

The production builder should eventually follow gamemd writer order:

1. Build level 2, then level 1, then level 0.
2. Assign zone ids by row-major first discovery within aligned blocks.
3. Preserve parent/coarser ids in lower levels.
4. Preserve final adjacency order.
5. Store edge flags.
6. Append bridge/tube edges after scanline edges and zero-flag them.

For Foundation First, implementation can start with a helper that constructs `ZoneHierarchy` from explicit test graphs. That lets precheck/search behavior land with high-confidence tests before fully reproducing writer order in map load.

The design should avoid using `BTreeSet` or sorted neighbor lists in any final parity graph. Sorting is acceptable only in non-parity helper caches that do not feed `Zone_precheck` route choice.

#### Zone Precheck Search

Add a new function instead of mutating `find_zone_corridor` into a half-compatible shape:

```rust
pub(crate) fn zone_precheck_flat(
    hierarchy: &ZoneHierarchy,
    start_cell: (u16, u16),
    goal_cell: (u16, u16),
    movement_zone: MovementZone,
    exclusions: &ZonePrecheckExclusions,
) -> ZonePrecheckOutcome
```

The exact signature can vary, but the output must include more than an allowed-zone set:

```rust
pub(crate) enum ZonePrecheckOutcome {
    Passed(ZonePrecheckResult),
    Failed,
}

pub(crate) struct ZonePrecheckResult {
    pub paths: [Vec<ZoneId>; 3],
    pub marked: [BTreeSet<ZoneId>; 3],
}
```

`ZonePrecheckResult` must retain the per-level selected paths because the later retry producer reads them. It must also expose level-0 markers for A* gating.

Search behavior:

- Iterate levels `2 -> 1 -> 0`.
- If start and goal zones match for a level, write a one-zone path.
- Otherwise run Dijkstra-like accumulated-cost graph search.
- Candidate cost in first slice: `current_cost + ZoneBaseCost[target_zone_type] + edge_flag_cost`.
- Slope contribution is zero in first-slice flat/no-mover tests.
- Equal-cost candidates do not replace earlier candidates.
- Heap ties preserve insertion/scan order and never use `ZoneId`.
- Levels 1 and 0 must reject candidates whose parent is not in the next-coarser marked set, except when `target.zone_type == 1`.
- Exclusions are per-level canonical undirected edge pairs for manual/preseeded tests.

Use fixed-point or integer-scaled costs for deterministic sim behavior. Because the only fractional first-slice cost is `0.001`, represent cost in milli-units:

- zone base cost `1.0` -> `1000`;
- zero base cost -> `0`;
- edge flag penalty `0.001` -> `1`.

This avoids `f32`/`f64` in sim logic and keeps equality behavior explicit.

#### A* Marker Gate

Replace the parity path's `expand_corridor()` handoff with a marker-aware filter in `core.rs`.

Do not make it strict chosen-zone-only. The filter must be:

```text
if hierarchy is enabled:
    candidate_zone = level0_zone(candidate_cell)
    if candidate_zone is marked:
        allow
    else if blocker_neighbor_refcount(candidate_cell) != 0:
        allow
    else:
        skip
```

The `+0x122` equivalent should be modeled as explicit search input, not buried inside terrain passability or `Can_Enter_Cell`:

```rust
pub hierarchy_gate: Option<HierarchyGate<'a>>

pub struct HierarchyGate<'a> {
    pub level0_zones: &'a ZoneLevelGraph,
    pub marked_level0: &'a BTreeSet<ZoneId>,
    pub blocker_neighbor_counts: Option<&'a BlockerNeighborCounts>,
}
```

If production blocker-neighbor counts are not available yet, tests can pass a synthetic count grid. The first production call path may keep the current corridor fallback until the count surface is ready, but the design target for removing `expand_corridor()` requires the exception.

#### Retry Exclusions

First slice supports manual/preseeded per-level exclusions:

```rust
pub(crate) struct ZonePrecheckExclusions {
    pub per_level: [BTreeSet<ZoneEdgeKey>; 3],
}
```

This is enough to test consumer behavior:

- excluded edge is skipped;
- endpoints are not banned as zones;
- exclusions persist through a single synthetic precheck retry scenario.

Do not implement or claim exact `UpdateHierarchicalEdges` producer parity in this slice. The precheck result must preserve paths so that later producer implementation can consume them.

#### Layered Pathing Boundary

Foundation First does not wire `find_layered_path_zoned_marker` to the new hierarchy gate yet.

The design must add comments/tests preventing overclaiming:

- flat/no-slope synthetic precheck can use `find_path_zoned_marker` or direct `core.rs` tests;
- layered high-bridge route parity remains deferred;
- low bridge records remain ground/tube/zone graph connectivity, not high-bridge redirect records.

### Interfaces / Contracts

Internal contracts:

- `ZoneAdjacency` remains available for current compatibility path.
- `ZoneHierarchy` is the parity-precheck graph, distinct from `SuperZoneMap`.
- `ZonePrecheckResult` stores per-level paths and markers.
- `HierarchyGate` filters cell A* candidates using level-0 markers plus blocker-neighbor exception.
- `ZonePrecheckExclusions` stores per-level undirected edge exclusions.
- `expand_corridor()` remains an approximation path until the parity gate is fully wired.

No render/UI/audio APIs change. `sim/` layering remains intact.

### Data Flow

Foundation test flow:

1. Construct a synthetic `ZoneHierarchy`.
2. Call `zone_precheck_flat` with start/goal cells and optional manual exclusions.
3. Assert per-level paths and marked sets.
4. Feed `ZonePrecheckResult.marked[0]` into `AStarOptions.hierarchy_gate`.
5. Run A* against a synthetic grid.
6. Assert marked cells are allowed, unmarked cells are rejected, and `+0x122` synthetic exception allows the intended off-marker cell.

Future production flow:

1. `ZoneGrid::build_with_terrain` builds current one-level data and the new hierarchy.
2. `find_path_zoned_marker` calls `zone_precheck_flat`.
3. Same-zone precheck failure clears hierarchy and calls A* without `HierarchyGate`.
4. Cross-zone precheck failure returns `None`.
5. Passed precheck calls A* with `HierarchyGate`.
6. Later retry producer can use `ZonePrecheckResult.paths`.

### Error Handling

Invalid zone ids, out-of-range cells, or missing hierarchy data should fall back to current compatibility behavior only where the current code already falls back today. Tests for exact parity functions should return `Failed` rather than silently using the old corridor path.

This avoids hiding missing hierarchy data behind accidental old behavior.

### Testing Strategy

Precheck tests:

- `zone_precheck_searches_levels_2_1_0`
- `zone_precheck_parent_gate_prunes_off_corridor_child_edges`
- `zone_precheck_parent_gate_allows_type_1_exception`
- `zone_precheck_edge_flag_adds_tiny_tiebreak_cost`
- `zone_precheck_bridge_edges_are_zero_flagged`
- `zone_precheck_manual_exclusion_skips_only_matching_edge`
- `zone_precheck_result_retains_per_level_path_for_later_retry_update`

A* marker-gate tests:

- `astar_hierarchy_rejects_unmarked_one_ring_zone_without_blocker_exception`
- `astar_hierarchy_allows_off_marker_cell_with_blocker_neighbor_count`
- `zone_precheck_same_zone_failure_clears_hierarchy_before_astar`

Boundary tests:

- `zone_get_high_bridge_redirect_ignores_low_bridge_records`
- `zone_precheck_flat_foundation_does_not_claim_layered_bridge_route_parity`

Suggested verification commands:

- `cargo test -q zone_precheck --lib`
- `cargo test -q hierarchy --lib`
- `cargo test -q astar_hierarchy --lib`
- `cargo test -q zone_search --lib`
- `cargo check -q`

## Architectural Decisions

- Add a new hierarchy/precheck path instead of further mutating `find_zone_corridor`. The current function is explicitly an approximation and is still useful as compatibility behavior while the hierarchy foundation is staged.
- Store per-level selected paths, not only marker sets. This is required for later retry producer parity.
- Model `+0x122` as explicit search-side blocker-neighbor data. It should not be folded into terrain, fog, water, bridge, or passability state.
- Use integer milli-costs for `0.001` edge flag cost to preserve deterministic sim math.
- Keep layered pathing out of the first implementation claim. The data structures should be reusable by layered pathing later, but the first slice proves flat/no-slope synthetic behavior only.
- Keep exact retry producer out of the first implementation claim. Manual/preseeded exclusions can prove consumer behavior now.

Tech debt intentionally introduced:

- Production hierarchy builder may initially lag the synthetic test graph builder.
- Exact slope cost remains absent.
- Exact `UpdateHierarchicalEdges` producer remains absent.
- `find_layered_path_zoned_marker` remains approximate.
- Stock Carville route remains trace-only.

## Alternatives Considered

### Approach A: Data Model Only

This would add `ZoneHierarchy` and edge metadata without wiring A* marker gating.

Rejected as the recommended first implementation because it would not close the important `expand_corridor()` approximation, and it would not force the `+0x122` exception into the design.

### Approach B: Foundation First v2

This is the chosen approach. It implements hierarchy/precheck data, flat/no-slope precheck, marker output, manual exclusions, and A* marker gating with the blocker-neighbor exception. It defers producer/slope/layered/stock-route parity without blocking the foundation.

### Approach C: Full Route Parity Now

This would implement exact retry producer, slope context, layered pathing, high-bridge route parity, and stock Carville logging in one push.

Rejected because it depends on unresolved runtime traces and would carry too much regression risk across shared pathfinding.
