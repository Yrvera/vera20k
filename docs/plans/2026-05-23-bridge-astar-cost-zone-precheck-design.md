# Bridge A* Cost And Zone-Precheck Parity Design

## Goal

Make bridge-aware path choices match the verified YR A* facts for zone-precheck tie order, bridge edge costs, and direction-8 bypass behavior without claiming full stock-map route parity before runtime route logging exists.

## Architecture Context

Pathfinding lives entirely under `src/sim/pathfinding`, which keeps the change inside the deterministic simulation layer. `zone_search.rs` performs zone reachability/corridor prechecks before cell A*. `zone_build.rs` and `zone_map.rs` build and store adjacency lists. `core.rs` owns cell A*, including dual ground/bridge arrays, bridge height selection, entity soft-block cost, marker overlays, direction tie-breaks, and explicit tube jumps.

The current Rust architecture already has the right broad split:

- `ZoneGrid` and `ZoneAdjacency` provide movement-zone reachability and neighbor order.
- `find_zone_corridor` selects a coarse path through zone adjacency before corridor-restricted cell A*.
- `AStarOptions` carries optional per-search data such as `SearchMarkerOverlay`, `LayeredEntityBlockMap`, terrain costs, resolved terrain, and corridor restriction.
- `astar_search` computes normal compass edges and then separately handles direction-8 explicit tube edges.

The mismatch is narrower than a full pathfinding rewrite. `find_zone_corridor` currently uses tuple ordering `(f_cost, g_cost, ZoneId)`, so equal-cost routes can be chosen by ZoneId instead of adjacency insertion order. Cell A* already models marker x4 and tube marker bypass, but bridge flank cost is missing and needs tests that lock down ordering with marker/entity costs and final direction epsilon.

## Impact Analysis

The design touches these implementation surfaces:

- `src/sim/pathfinding/zone_search.rs`
  - Replace equal-cost `ZoneId` tie behavior in `find_zone_corridor`.
  - Preserve edge exclusions as undirected edge skips.
  - Keep or explicitly document the current one-ring corridor expansion as an approximation outside the scoped binary precheck fix.
- `src/sim/pathfinding/zone_build.rs`
  - Preserve final `ZoneAdjacency` insertion order.
  - Avoid relying on upstream sorted node adjacency where parity-facing order matters.
- `src/sim/pathfinding/zone_map.rs`
  - Keep `ZoneAdjacency::neighbors_of` as ordered slice access.
- `src/sim/pathfinding/core.rs`
  - Add bridge-layer flank multiplier for normal compass edges after marker x4 and before final direction tie-break.
  - Keep direction-8 tube expansion separate from normal compass edge cost.
  - Keep dual ground/bridge closed state.
- `src/sim/pathfinding/core_tests.rs`
  - Add marker/flank/epsilon/tube-bypass regression tests.
- `src/sim/pathfinding/zone_search_tests.rs`
  - Add insertion-order equal-cost zone route tests.

Risk areas:

- Deterministic path choice can change for any mover using zone precheck, especially after bridge collapse/repair.
- A bridge flank cost can alter route preference on bridge decks and bridgeheads.
- Over-tightening corridor behavior could break existing reachable-route tests because current Rust is still a partial hierarchy approximation.
- Direction-8 tube behavior must not receive normal marker/flank/epsilon costs as an accidental side effect of cost refactoring.

## Chosen Approach

Patch the existing pathfinding system in place for the implementation-safe binary facts.

This is deliberately not a full `Zone_precheck` clone. The current code still lacks all full binary hierarchy inputs: three-level path buffers, exact zone type base costs, slope cost, edge flag cost, and stock route logging. Approach A fixes the known active mismatches without inventing unresolved behavior.

The key design choices are:

1. Use stable insertion-order tie behavior in `find_zone_corridor`.
2. Keep edge exclusions as undirected edge exclusions, not zone bans.
3. Add bridge flank multiplier as a normal compass edge-cost decoration.
4. Keep marker x4 search-scoped and destination-based.
5. Keep direction epsilon final and additive.
6. Keep direction-8 tube expansion on a separate cost path.
7. Do not implement true closed-node reopen from the `1.009` finding.
8. Defer exact Carville route assertions and `FUN_0042B080` peer-object modeling.

## Tiny-Detail Ledger

- Zone adjacency is scanned in stored order; no sort before candidate evaluation. Source: `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`, `0x0042C501..0x0042C540`.
- Equal-cost zone candidates do not replace earlier candidates. Source: `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`, `0x0042C5DE..0x0042C5EA`.
- Zone heap movement uses strict lower-cost comparisons; equality does not bubble the new node ahead. Source: `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`, `0x0042C6B3..0x0042C6D8`.
- ZoneId is not a tie-break key. Source: `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`, negative tie fact.
- Exclusions are undirected edges, not whole-zone bans. Source: `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`, `0x0042C620..0x0042C664`.
- Binary `Zone_precheck` is accumulated-cost/Dijkstra-like, not centroid A*. Source: `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`, `0x0042C55C..0x0042C5D2`.
- Marker `0x40000` multiplies current edge cost by `4.0` after code-2/entity cost and before bridge flank. Source: `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`, `0x004299AA..0x004299C2`.
- Bridge flank multiplier is `10.0`, `1.0`, or `2.0`, keyed by destination orientation `0x800` and flank structural bit `0x100`. Source: `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`, `0x004299D2..0x00429A79`.
- Direction epsilon is final/additive after helper return; it is not multiplied by marker, flank, or entity costs. Source: `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`, `0x00429F8A..0x00429F9D`.
- Direction 8/tube bypasses the normal helper, marker, flank, and normal epsilon. Source: `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`, `0x00429F6B..0x00429FA3`.
- `1.009` is not true reopen logic; closed selected-layer nodes are not reinserted. Source: `ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md`, `0x00429EEC`, `0x00429FFB..0x0042A01E`.
- Exact Carville post-collapse route is unknown and must not be asserted yet. Source: `STOCK_LOW_BRIDGE_COLLAPSE_ROUTE_TRACE_GHIDRA_REPORT.md`.

## Design

### Components

#### Stable Zone Corridor Search

`find_zone_corridor` should stop using tuple ordering that lets `ZoneId` decide equal-cost ties. The replacement should preserve:

- lower accumulated cost wins;
- equal accumulated cost keeps the earlier discovered parent;
- neighbor iteration order comes directly from `ZoneAdjacency::neighbors_of`;
- stale heap entries are ignored by cost comparison, not by zone-id tie ordering.

The smallest design is to introduce an internal queue item with:

- `cost: i32`
- `sequence: u32` or equivalent insertion counter
- `zone: ZoneId`

Ordering should be by `cost` first and insertion sequence second. `ZoneId` is payload only. Replacement remains `new_cost < dist[neighbor]`, never `<=`.

The current `find_zone_corridor` still uses center Manhattan edge costs. This is not full binary `Zone_precheck`, but the tie fix removes the proven ZoneId drift. A comment should state that the cost model remains an approximation until zone type/slope/edge-flag cost is implemented.

#### Bridge Edge-Cost Decoration

`astar_search` should keep the existing normal compass edge pipeline but add one bridge flank multiplier stage:

1. base terrain/height/entity cost;
2. search marker x4 via `SearchMarkerOverlay`;
3. bridge flank multiplier when entering/using bridge layer and the branch is enabled for the searched context;
4. final `DIR_TIEBREAK[dir]` addition.

Because the exact `PathfinderClass+0x01` setter lifecycle remains deferred, the implementation should model this with a narrowly named option or helper that defaults to the current live bridge pathing expectation only after tests prove the intended call path. If the branch-enable byte cannot be mapped safely in the first patch, add the helper and tests first, then wire it in a second patch after confirming the caller contract.

The bridge flank helper should compute flanking cells from destination orientation and movement direction. It should inspect structural bridge state equivalent to binary `0x100`, using existing `PathCell` bridge facts rather than static terrain assumptions.

#### Direction-8 Tube Bypass

The existing direction-8 block in `astar_search` must remain separate. It should not call any shared helper that applies:

- entity/code helper costs;
- marker overlay x4;
- bridge flank multiplier;
- normal `DIR_TIEBREAK`.

Existing tests already cover marker bypass for explicit tubes. New tests should add flank bypass if the helper becomes shared.

#### Closed-List Guardrails

This patch should not implement ordinary A* reopen. The existing Rust immediate closed skip is not a perfect model of the binary `1.009` blocked-goal fallback path, but adding reopen would be worse. The design should add tests or comments that prevent future code from reopening closed selected-layer nodes based on lower `g`.

The more exact `1.009` blocked-goal behavior can be a later targeted patch only if a crafted test proves observable divergence.

### Interfaces / Contracts

No public app/render/UI API changes are needed.

Internal contracts:

- `ZoneAdjacency::neighbors_of(zone)` returns ordered neighbors and callers must treat order as parity-significant.
- `find_zone_corridor` returns a path whose equal-cost ties follow adjacency discovery order.
- `SearchMarkerOverlay` remains search-scoped and must not mutate `PathGrid`.
- Direction-8 tube edges remain a separate branch from normal compass edge computation.
- Bridge flank cost helper must operate on normal compass edges only.

### Data Flow

Zone path flow:

1. `ZoneGrid` is built from `PathGrid`, terrain costs, resolved terrain, and bridge endpoint records.
2. `ZoneAdjacency` preserves discovered neighbor order.
3. `find_path_zoned_marker` invokes `find_zone_corridor`.
4. `find_zone_corridor` uses stable strict-cost queue behavior.
5. The selected corridor is expanded by current Rust compatibility logic before cell A*.
6. Cell A* searches within allowed zones.

Cell edge-cost flow:

1. `astar_search` expands normal directions `0..=7`.
2. It decides ground/bridge layer from current height and neighbor bridge state.
3. It applies passability, terrain, height, and entity costs.
4. It applies marker x4 for marked destination cells.
5. It applies bridge flank multiplier for bridge-layer normal compass edges when enabled.
6. It adds final direction tie-break.
7. Direction 8 explicit tube expansion runs separately and bypasses the normal cost pipeline.

### Error Handling

No new error types are needed. Invalid flanking coordinates should behave as non-structural bridge flank cells for cost purposes only if that matches the binary flank-cell lookup bounds. If bounds behavior is not already proven, tests should cover in-bounds fixtures first and leave edge-of-map flank behavior as a documented follow-up.

### Testing Strategy

Zone tests:

- Equal-cost corridor with adjacency order `[high_zone, low_zone]` must choose `high_zone` first.
- Reversing adjacency order should reverse the chosen equal-cost corridor.
- Excluding one undirected edge should skip only that edge, not ban either endpoint zone.
- A stale-entry case should not replace equal-cost existing parent.

Core A* tests:

- Marker x4 still stacks after code-2/entity cost.
- Bridge flank helper returns `10x`, `1x`, and `2x` cases.
- Marker x4 and bridge flank multiply before final direction tie-break.
- Direction epsilon remains additive and unscaled.
- Direction-8 explicit tube destination does not receive marker or bridge flank cost.
- Existing bridge/ground dual-layer path tests still pass.

Verification commands:

- `cargo test zone_search --lib`
- `cargo test bridge --lib`
- `cargo test marker --lib`
- `cargo test tube --lib`
- `cargo check`

## Architectural Decisions

- Keep the patch inside existing `sim/pathfinding` modules. This follows the current architecture and avoids hidden dependencies on render/UI/audio.
- Prefer an internal stable queue item over tuple heap ordering. This is a small local abstraction that exists only to prevent accidental `ZoneId` tie behavior.
- Do not introduce a full binary `Zone_precheck` hierarchy yet. That would require unresolved cost and runtime-route evidence.
- Keep marker overlay as search-scoped data, matching the existing Rust model and binary lifecycle.
- Keep direction-8 bypass physically separate in code, because sharing a generic edge-cost helper is the main way this parity fact could regress.

Tech debt intentionally retained:

- `find_zone_corridor` still uses centroid Manhattan edge costs rather than binary zone type/slope/edge-flag cost.
- The current one-ring corridor expansion remains a compatibility approximation.
- Exact `1.009` blocked-goal fallback is not modeled.
- Exact Carville post-collapse route is not asserted.
- `FUN_0042B080` peer-object scan is deferred.

## Alternatives Considered

### Separate Binary Precheck Path

Add a new `binary_zone_precheck` beside the current corridor code. This would isolate parity work cleanly, but it duplicates search behavior before the full hierarchy inputs are known. It also risks routing only some callers through the parity path and leaving confusing split behavior.

Rejected for now because the implementation-safe findings can be fixed locally without a parallel system.

### Full Three-Level Binary Hierarchy

Model `Zone_precheck` levels `2 -> 1 -> 0`, selected chain buffers, parent gates, zone type base costs, slope costs, edge flag cost, and per-level exclusions.

Rejected for this patch because the synthesis still marks full cost model and stock Carville route validation as needing further investigation. Building the hierarchy now would look complete while still guessing at important inputs.

### Keep Current Zone Search And Only Add Bridge Flank Cost

This would improve bridge-deck local A* choices but leave the known equal-cost zone tie mismatch untouched. Since bridge collapse/repair makes zone detours player-visible, cutting the zone tie fix would preserve a confirmed parity hole.

Rejected because it fails the parity ledger.

## Handoff

Recommended next step: write an implementation plan for Approach A, then patch in two small phases:

1. Zone-precheck tie/order tests and stable queue behavior.
2. Bridge flank cost helper/tests and direction-8 bypass guardrails.

Do not add exact Carville route expectations until runtime route logging exists.
