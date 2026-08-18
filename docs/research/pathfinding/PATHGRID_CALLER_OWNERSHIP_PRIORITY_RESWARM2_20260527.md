# PathGrid Caller Ownership And Priority Map - Reswarm 2 Slot 5

Date: 2026-05-27
Slot: 5
Scope: current Rust caller ownership for pathgrid/cell-entry fixes related to the water/pier passability gap.
Status: COMPLETE

## Executive Verdict

The player-visible water/pier failure should be fixed by tightening a small set of movement-facing owners first, not by broadly changing every `PathGrid::is_walkable()` caller.

Current Rust already has `src/sim/pathfinding/cell_entry.rs`, but that module is not yet the native-shaped terrain legality boundary needed by the contract. Its terrain phase still checks ground cells with `path_grid.map_or(true, |g| g.is_walkable(nx, ny))` plus optional `TerrainCostGrid::cost_at() > 0`, with no `MovementZone`, reduced `ZoneType`, or resolved terrain matrix input. That means using it blindly today would preserve the same core water/pier drift for non-water ground movers.

Highest-risk owners for the water/pier gap:

1. `src/sim/pathfinding/core.rs`: A* goal/neighbor legality and `is_cell_passable_for_mover`.
2. `src/sim/movement/movement_path.rs` plus `path_smooth.rs`: command-time goal redirection and post-A* smoothing.
3. `src/sim/movement/movement_step.rs` / `movement_tick.rs`: runtime cell transition and drive-track chaining checks.
4. `src/sim/movement/scatter.rs` and `bump_crush.rs`: scatter candidate selection.
5. `src/sim/miner/miner_dock_sequence.rs` and `miner_system.rs`: Chrono Miner/refinery nearby-passable staging.
6. Production/refinery/edge/drop callers are real PathGrid users, but they should be patched after the shared evaluator exists unless a concrete repro shows them causing the observed water-driving symptom.

## Evidence Read

- Contract: `docs/contracts/2026-05-27-pathgrid-water-pier-cell-legality-implementation-contract.md`.
- Reswarm docs: `CAN_ENTER_CELL_WATER_PIER_LEGALITY_RESWARM_20260527.md`, `ASTAR_WATER_PIER_NEIGHBOR_LEGALITY_RESWARM_20260527.md`, `PATH_SMOOTHING_WATER_PIER_LEGALITY_RESWARM_20260527.md`, `HELPER_PASSABLE_CELL_CONTRACTS_RESWARM_20260527.md`, `PIER_BRIDGE_WATER_CLASSIFICATION_RESWARM_20260527.md`.
- Source search: `PathGrid::is_walkable`, `is_any_layer_walkable`, `nearest_walkable`, `is_cell_passable_for_mover`, current `cell_entry` APIs, movement, production, miner, and zone paths.
- Codegraph context: `PathGrid::is_walkable`, `is_cell_passable_for_mover`, `PathGrid`, miner, ore, smudge, and related symbols.

No Rust code was edited.

## Current Shared Boundary

| File / function | Current role | Classification | Evidence | Required ownership decision |
|---|---|---|---|---|
| `src/sim/pathfinding/cell_entry.rs::check_terrain_with_layers` | Intended cell-entry terrain phase. | REQUIRED_FIX | Lines 288-303 use `PathGrid::is_walkable` and optional terrain cost only for ground terrain. No `MovementZone` or resolved `ZoneType` input. | Make this, or a sibling API in `pathfinding`, the native-shaped cell-entry evaluator. It must accept MovementZone, resolved terrain, speed/cost inputs, layer context, and occupancy policy before movement callers migrate to it. |
| `src/sim/pathfinding/cell_entry.rs::classify_occupied_cell_with_layers` | Occupancy/result-code phase for runtime blockers. | DEFER until evaluator exists | Used by `movement_occupancy.rs`; occupancy classification is useful, but terrain legality is checked elsewhere before this phase. | Keep as phase 2. Do not broaden it to terrain legality until phase 1 has native-shaped inputs. |
| `src/sim/pathfinding/core.rs::is_cell_passable_for_mover` | Current shared Boolean pathfinding passability helper. | REQUIRED_FIX | Lines 1392-1415 return water-surface matrix only for water movers; all non-water movers fall back to `grid.is_walkable(x, y)`. | Replace fallback with native-shaped ground legality. This is the safest public compatibility hook for A*, production spawn, and other callers that already use it. |
| `src/sim/pathfinding/core.rs::PathGrid::from_resolved_terrain_with_bridges` | Builds coarse static grid. | DEFER until evaluator exists | Lines 1781-1785 and 1804-1835 intentionally keep water `ground_walkable` via `!cell.ground_walk_blocked || cell.is_water`. | Do not flip this globally first. Many compatibility paths were built around it. Treat `PathGrid` as coarse static geometry and enforce movement legality above it. |

## REQUIRED_FIX Callers

| Priority | File / function | Current call | Why it matters | Patch owner |
|---|---|---|---|---|
| P0 | `src/sim/pathfinding/core.rs::astar_search` | Goal check calls `is_cell_passable_for_mover` at lines 836-842; neighbor ground check calls it at lines 1142-1148. | A* is the first place a ground unit can route onto water when `PathGrid` says water is walkable. Contract requires gamemd-style `Can_Enter_Cell` legality for neighbors. | `sim/pathfinding`: update helper/evaluator and A* options so ground rows reject water even without a terrain cost grid. |
| P0 | `src/sim/pathfinding/core.rs::astar_search` | Missing cost grid defaults to `100` at lines 1216-1225; edge cost then uses terrain speed percentage at lines 1249-1257. | Without a cost grid, PathGrid-walkable water becomes uniformly legal for non-water movers. Cost source parity is also called out in the contract. | `sim/pathfinding`: separate zero-passability from route weighting. |
| P0 | `src/sim/pathfinding/zone_search.rs::can_use_reduced_zone_precheck` | Lines 62-75 allow only `None`, `Normal`, `Amphibious`, `Infantry`, `Fly`; rows such as `Crusher` bypass reduced-zone precheck and go straight to A*. | Crusher and other live ground rows can skip the matrix reachability gate that blocks water. | `sim/pathfinding`: broaden to verified MovementZone rows once evaluator/matrix rows are stable. |
| P0 | `src/sim/movement/movement_path.rs::is_move_goal_walkable` | Non-water movers return `grid.is_any_layer_walkable` at line 75. | A blocked or clicked goal near water/bridge can be accepted on coarse ground or bridge layer without mover-specific legality. | `sim/movement`: call evaluator for final goal acceptance. |
| P0 | `src/sim/movement/movement_path.rs::nearest_move_goal` | Non-water movers call `grid.nearest_walkable_any_layer` at lines 86-88. | This is a direct path to selecting water/pier-adjacent cells during command-time fallback. | `sim/movement`: replace with FNPC-shaped candidate scan once evaluator exists; preserve deterministic ring order until exact FNPC row is known. |
| P0 | `src/sim/movement/movement_commands.rs::issue_move_command_with_layered` | Calls `resolve_requested_move_goal(..., max_radius=10)` at lines 302-308 before pathing. | Every player-issued move command can hit the coarse fallback. The later debug logs at lines 428-439 are not causal. | `sim/movement`: route redirection through evaluator or temporarily disable non-water any-layer redirection for parity-critical paths. |
| P0 | `src/sim/movement/movement_path.rs::find_move_path_with_marker` | Layered smoothing closure checks `grid.is_walkable_on_layer` at lines 242-244; flat smoothing closure uses `grid.is_walkable` at lines 304-315 for non-water movers. | The reswarm verified gamemd smoothing revalidates candidate shortcut cells through mover `Can_Enter_Cell`, not a coarse grid. This can reintroduce water shortcuts after A* avoided them. | `sim/movement` and `sim/pathfinding/path_smooth`: pass evaluator-backed closures and later correct native reroute ordering. |
| P0 | `src/sim/pathfinding/path_smooth.rs::smooth_path` / `optimize_path` | Boolean closure contract at lines 85-86; diagonal flank checks at lines 127-140; reroute checks at lines 472-480. | The closure can only express coarse passability today, and the extra flank checks are a separate parity drift from the smoothing report. | `sim/pathfinding`: keep algorithm patch separate from caller migration. First feed a correct evaluator closure, then patch reroute order/flank behavior. |
| P0 | `src/sim/movement/movement_step.rs::advance_movement_step` | Non-water ground transition uses `target.bypass_grid || path_grid.map_or(true, |grid| grid.is_walkable(nx, ny))` at lines 474-499. | Even if path construction is fixed later, runtime transition must not allow stale paths or helper-issued paths to enter water for ground movers. | `sim/movement`: use the same terrain evaluator as A* for non-bypass ground transitions; preserve documented `bypass_grid` dock choreography. |
| P0 | `src/sim/movement/movement_tick.rs` drive-track chaining | Lines 844-850 check the following cell with `path_grid.map_or(true, |g| g.is_walkable(after.0, after.1))`. | Drive-track chaining can pre-approve a next cell using the same coarse water-walkable grid. | `sim/movement`: replace with evaluator-backed next-cell check using active layer and mover snapshot. |
| P1 | `src/sim/movement/scatter.rs::tick_idle_scatter` | Lines 121-150 choose random adjacent cell with `grid.is_walkable` plus simplified occupancy. | Native scatter uses `Can_Enter_Cell`/FNPC-shaped legality; this can scatter idle units onto water-adjacent cells. | `sim/movement`: after evaluator exists, make scatter candidate legality unit-specific. |
| P1 | `src/sim/movement/scatter.rs::find_passable_scatter_cell` | Lines 330-345 use `path_grid.is_walkable` plus simplified occupancy. | Building/cell scatter can choose the same bad water candidate. | `sim/movement`: route through evaluator or FNPC helper. |
| P1 | `src/sim/movement/bump_crush.rs::scatter_blocker` | Lines 640-667 select adjacent cell using `grid.is_walkable`. | Movement blocker scatter is on the live movement path and can issue a direct move onto water. | `sim/movement`: pass mover data into scatter candidate validation, or call a shared nearby-passable helper. |
| P1 | `src/sim/miner/miner_dock_sequence.rs::is_exit_cell_passable` and nearby helpers | Lines 268-287 gate FNPC-style candidates with `grid.is_walkable`; lines 303-392 build first/passable-ring selection from that predicate. | This is the Chrono Miner/refinery staging symptom surface identified by the first swarm. Native FNPC uses `CellRect::CheckPassability`, not pathgrid alone. | `sim/miner` with `sim/pathfinding`: replace predicate with FNPC-shaped validator; preserve candidate order and modulo selection. |
| P1 | `src/sim/miner/miner_system.rs::chrono_return_staging_cell_for_sid` | Lines 1212-1220 call `find_nearby_passable_cell_with_index` with no occupancy. | Far-return staging can choose a PathGrid-walkable water/pier-adjacent cell from refinery `QueueingCell`. | `sim/miner`: pass occupancy/resolved terrain/evaluator inputs; keep this after shared helper exists. |
| P1 | `src/sim/miner/miner_system.rs::issue_move_if_idle` and dock duplicate in `miner_dock_sequence.rs` | `issue_move_command` is called with no cost, resolved terrain, entity block map at `miner_system.rs` lines 1413-1415 and `miner_dock_sequence.rs` lines 1206-1208. | These calls enter the same command-time goal redirection/A* stack without terrain inputs. | `sim/miner`: after movement command evaluator is fixed, thread resolved terrain/cost inputs for miner calls where available. |
| P1 | `src/sim/production/production_refinery.rs::maybe_spawn_refinery_harvester` | Primary and fallback free harvester cells use `grid.is_walkable` at lines 50 and 115. | A refinery near water can spawn its free harvester into a PathGrid-walkable water cell. Not the same as driving on water, but same caller-pattern drift. | `sim/production`: patch after shared evaluator, or sooner if a refinery-spawn repro exists. |

## DEFER Until Evaluator Exists

| File / function | Current call | Reason to defer |
|---|---|---|
| `src/sim/production/production_spawn.rs::spawn_cell_passable` | Water units call `is_cell_passable_for_mover` at lines 427-435; land units return `grid.is_walkable` at line 436. | `cell_available_for_spawn` rejects water when `resolved_terrain` is present at lines 396-405, so the common land-spawn water case is partially guarded. Fallback/no-terrain paths should migrate after the evaluator is ready. |
| `src/sim/production/production_spawn.rs::nearest_walkable_around` | Calls `spawn_cell_passable` at lines 331, 343, 357, 369. | Ownership should stay in production, but the predicate should be swapped rather than reimplementing cell legality locally. |
| `src/sim/production/production_placement.rs::cell_placeable` | Water-bound placement uses resolved terrain/matrix at lines 374-399; land fallback uses `grid.is_walkable` only when resolved terrain is missing at lines 401-415. | Building placement is not the observed unit-driving symptom and mostly has stronger resolved-terrain checks. Keep out of the first movement patch. |
| `src/sim/aircraft/drop_payload.rs::drop_next_payload` | Payload drop cell uses `path_grid.map_or(true, |g| g.is_walkable(...))` at line 170. | Can put passengers onto invalid terrain, but it is aircraft payload/drop behavior rather than ground pathing. Patch after the evaluator has a category/layer policy for dropped passengers. |
| `src/sim/world/edge_cell.rs::find_passable_at_edge` | Edge scanner filters by `path_grid.is_walkable` at lines 100 and 142. | Reinforcement/spawn edge selection is separate from path movement. It should get a caller-specific criterion once the evaluator exists. |
| `src/sim/miner/miner_system.rs::is_cell_path_clear_for_scan` | Ore scan uses `grid.is_walkable` at lines 306-324. | This filters ore candidate reachability, not final movement entry. It can stay behind the movement and FNPC fixes unless ore-on-water traces show a live symptom. |
| `src/sim/pathfinding/zone_build.rs::is_passable` | Non-water non-fly rows first reject `!path_grid.is_walkable` at lines 442-447, then consult resolved terrain/matrix at lines 449+. | This helper already has the correct high-level shape when resolved terrain is present. Revisit with matrix-row cleanup, but do not make it the first patch. |

## TEST_ONLY / Currently Unwired

| File / function | Current state | Classification | Notes |
|---|---|---|---|
| `src/sim/movement/group_destination.rs::distribute_group_destinations` | Uses `grid.is_any_layer_walkable` in `find_next_vehicle_cell` and `find_next_infantry_cell` at lines 102-143. `rg` found no production caller outside its own tests and module export. | TEST_ONLY | If this gets wired into command handling, it becomes REQUIRED_FIX because group spread would allocate water/bridge-layer cells without mover legality. Today it should not block the first patch. |
| `src/sim/pathfinding/core.rs::PathGrid::nearest_walkable` / `nearest_walkable_any_layer` | Definitions at lines 1627-1680. The movement goal redirect uses `nearest_walkable_any_layer`; no independent live use found for `nearest_walkable`. | TEST_ONLY for `nearest_walkable`; REQUIRED_FIX through `movement_path` for `nearest_walkable_any_layer`. | Do not delete now. Treat as coarse helpers; new movement code should stop using them for final passability. |
| `src/sim/movement/movement_commands.rs` debug path logging | Lines 428-439 call `grid.is_walkable` only to format a warning/log path. | TEST_ONLY / diagnostic | No gameplay decision. Keep or update after behavior fixes to avoid misleading logs. |
| `src/sim/pathfinding/core_tests.rs` and other `*_tests.rs` | Many tests assert water is PathGrid-walkable. | TEST_ONLY | These tests should be rewritten around the distinction: `PathGrid` may be coarse, but native evaluator must reject water for ground movers. |

## OUT_OF_SCOPE For Water/Pier Movement Fix

| File / function | Current call | Why out of scope |
|---|---|---|
| `src/sim/ore_growth.rs::can_germinate` | Uses `grid.is_walkable` at lines 514-532. | This is resource germination, not unit movement. It may be a separate terrain parity issue because the comment says water should not germinate, while PathGrid makes water walkable. Do not mix it into the movement patch. |
| `src/sim/combat/smudge_dispatch.rs::try_dispatch_building_survivor_smudges` | Uses `path_grid.is_walkable` at lines 274-289. | Visual smudge dispatch on destroyed building cells. Not cell-entry or pathfinding. |
| `src/sim/production/production_sell.rs::garrison_infantry_can_enter_cell` | Calls `check_terrain(..., None, None, &sim.occupancy)` at lines 317-326. | It does not use PathGrid directly and is about garrison/sell exit. It may still need exact infantry exit research later, but it is not the water/pier pathgrid caller set. |
| `src/sim/production/production_placement.rs::cell_placeable` common resolved-terrain path | Uses build-blocked/overlay/bridge/slope facts at lines 401-410, not PathGrid. | Building placement has its own terrain contract. Only the no-resolved-terrain fallback is deferred above. |
| `src/sim/world/edge_cell.rs::find_paradrop_carrier_edge_cell` | Uses unchecked edge scans by design at lines 59-79. | Comment says the paradrop carrier spawner bypasses ordinary ground passability. Do not "fix" this via ground cell-entry logic. |

## Architecture Boundaries

- All required changes stay inside `sim/`, primarily `sim/pathfinding`, `sim/movement`, `sim/miner`, and `sim/production`.
- `sim/` must not depend on `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`. Nothing in this caller map requires those layers.
- `cell_entry.rs` already depends on `map`, `rules`, `sim/entity_store`, `sim/movement`, and `sim/occupancy`. A native terrain evaluator should avoid taking the whole `Simulation`; pass explicit `ResolvedTerrainGrid`, `PathGrid`, `TerrainCostGrid`, `MovementZone`, `SpeedType` if needed, layer context, and occupancy policy.
- Preserve deterministic ordering: A*, scatter neighbor order, FNPC ring order, modulo source, and `EntityStore` iteration must not be rewritten as part of passability cleanup.
- Do not globally change `PathGrid` water construction first. That would have broad unknown effects on hover/amphibious/naval compatibility paths and zone rebuilds. The safer boundary is "coarse grid plus exact evaluator."

## Safe Patch Ordering

1. **Evaluator foundation**
   - Extend or wrap `cell_entry.rs` with a terrain-only evaluator that can answer "would this mover enter this cell/layer?" using `MovementZone`, resolved `ZoneType`/land facts, bridge/tube facts, `PathGrid` static blockers, terrain cost zero checks, and occupancy policy.
   - Add narrow tests where `PathGrid` says water is walkable but Normal/Crusher/Infantry ground entry rejects it.

2. **A* plus runtime transition in one patch**
   - Update `is_cell_passable_for_mover` and A* ground neighbor/goal checks.
   - Update `movement_step.rs` runtime ground transition check to the same evaluator.
   - Reason: if only A* changes, stale/helper paths may still enter water; if only runtime changes, units may generate paths they immediately reject and churn.

3. **Reduced-zone precheck**
   - Broaden `zone_search::can_use_reduced_zone_precheck` to verified live rows such as `Crusher`.
   - Keep water/naval rows guarded until the water-surface semantics are verified enough.

4. **Goal redirection**
   - Replace `is_any_layer_walkable` and `nearest_walkable_any_layer` for non-water move-goal fallback with evaluator-backed candidate checks.
   - Preserve current ring order until the unresolved `Find_Path -> FNPC` push row is decoded.

5. **Smoothing**
   - Feed evaluator-backed walkability closures into flat and layered smoothing.
   - Then separately patch `path_smooth.rs` reroute ordering and remove/gate unverified diagonal flank checks.

6. **Scatter and miner helpers**
   - Move `scatter.rs`, `bump_crush.rs`, and miner FNPC-style helpers onto the shared evaluator.
   - Keep `bypass_grid` and dock choreography exceptions explicit and tested.

7. **Production/refinery/edge/drop cleanup**
   - Patch free refinery harvester spawn, production spawn fallback, edge-cell ground spawns, and aircraft passenger drops with caller-specific policies.
   - Do this after the movement path is stable to avoid broad refactor churn.

8. **Tests and diagnostics**
   - Update tests that currently assert "water is PathGrid-walkable" so they also assert the evaluator rejects water for ground movers.
   - Update debug logs that print `grid.is_walkable` as "goal_walkable" if that becomes misleading after evaluator migration.

## Implementation-Risk Notes

- `bypass_grid` is a real current exception for choreographed harvester dock movement. It should not be removed casually; `movement_occupancy.rs` lines 114-123 also special-case it to avoid dock oscillation.
- `MovementLayer::Bridge` cannot be collapsed into ground passability. Bridge object-list and occupancy-bits layer splits are already modeled through `CanEnterLayerContext`.
- Production spawn and edge-cell selection need category-specific policies; do not route every caller through a "ground unit" assumption.
- Existing `cell_entry.rs` result codes are useful, but the current terrain phase is not yet a proof of gamemd `Can_Enter_Cell` terrain legality.

## Bottom Line

For the water/pier bug, fix ownership in this order: `cell_entry` evaluator boundary, A* plus runtime transition, reduced-zone precheck, move-goal redirection, smoothing, scatter, miner staging. Defer production/edge/drop and out-of-scope systems until the shared evaluator exists. Do not globally rewrite `PathGrid` construction as the first step.
