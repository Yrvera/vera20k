# PathGrid Cell-Entry Evaluator Design

## Goal

Close the water/pier movement parity gap by making mover-specific cell-entry legality a shared `sim/pathfinding` evaluator above `PathGrid`, then migrating movement-facing callers to that evaluator in safe phases.

## Architecture Context

`PathGrid` currently stores coarse geometry and layer metadata: ground walkability, bridge walkability, transitions, and height/layer facts. It is useful for candidate generation, but it is not a faithful active-YR `Can_Enter_Cell` or `CellRect::CheckPassability` oracle.

The existing `src/sim/pathfinding/cell_entry.rs` module already owns `CellEntryResult` and a two-phase model:

- phase 1: terrain/basic occupancy without `EntityStore`;
- phase 2: blocker classification with `EntityStore`.

That is the right architectural home, but its current terrain phase still checks ground cells with `PathGrid::is_walkable` plus optional `TerrainCostGrid`, without `MovementZone`, resolved `ZoneType`, or enough bridge/tube/land-type context.

Current high-risk callers are:

- A* goal/neighbor legality in `src/sim/pathfinding/core.rs`;
- runtime movement transition and drive-track chaining in `src/sim/movement/movement_step.rs` and `movement_tick.rs`;
- move-goal redirection and smoothing in `src/sim/movement/movement_path.rs` and `src/sim/pathfinding/path_smooth.rs`;
- scatter and bump/crush helpers;
- miner/refinery staging helpers.

`sim/` remains the boundary. The evaluator must not depend on render, UI, audio, sidebar, or net modules.

## Impact Analysis

The design touches shared movement, so the main risk is divergence between path construction and path execution. If A* rejects water but runtime movement still uses `PathGrid`, stale/helper paths can still enter invalid cells. If runtime movement rejects cells A* accepted, units may churn or get stuck. For that reason, A* legality and runtime movement transition should land together.

The matrix/precheck work has a separate hazard: Rust currently has two matrix sources. `zone_build.rs::MOVEMENT_CLASS_PASSABILITY` matches the verified binary rows, while `passability.rs::PASSABILITY_MATRIX` is stale/wrong in several rows and is used by `zone_hierarchy.rs`. The matrix must be unified before broadening reduced-zone precheck to all valid rows.

Existing `bypass_grid` behavior for dock/refinery choreography must remain explicit. It is not a general movement legality shortcut.

## Chosen Approach

Use Approach C: a native-shaped evaluator above `PathGrid`.

`PathGrid` remains a coarse structural graph. A new or extended evaluator in `cell_entry.rs` decides mover-specific terrain/cell legality using explicit inputs:

- mover `MovementZone`;
- mover `SpeedType`;
- source and target cell coordinates;
- target `LandType` and reduced `ZoneType` from `ResolvedTerrainGrid`;
- current movement layer and requested target layer;
- bridge/tube flags, levels, ramp/slope bytes, and direction, including direction `8`;
- optional terrain-cost table only as a speed/land source or compatibility input, not as final legality;
- caller mode: A*, runtime transition, smoothing, scatter, or spawn-style helper; FNPC/check-passability uses a wrapper policy around this slice;
- occupancy/list-layer policy where the caller needs it.

This design deliberately keeps three native questions separate:

- cell-entry legality: `Can_Enter_Cell` / `CellClass::CheckCellPassability` style terrain, bridge/tube, speed/land, layer, locomotor, and occupancy-list decisions;
- reduced-zone reachability: A* hierarchy precheck using `MovementZone x reduced ZoneType`;
- nearby-passable selection: FNPC ring collection and caller policy around `CellRect::CheckPassability`.

`MovementZone x ZoneType` must not be used as a generic runtime or smoothing substitute for `Can_Enter_Cell`. It belongs to reduced-zone reachability and FNPC zone constraints. Runtime cell-entry, smoothing, and adjacent scatter need the cell-entry terrain slice first; route planning may additionally use the reduced-zone matrix before A*.

The evaluator should return a small structured result rather than only `bool`, for example:

```text
TerrainEntryResult
- Clear
- HardBlocked
- NeedsOccupancy
- LayerMismatch
- TubeRequired
- BridgeRejected
- LocomotorRejected
```

Full `Can_Enter_Cell` occupant code priority remains phase 2 and should not be collapsed into the terrain evaluator.

## Tiny-Detail Ledger

- `PathGrid::is_walkable` is not active-YR cell-entry legality. Source: `PATHGRID_CALLER_OWNERSHIP_PRIORITY_RESWARM2_20260527.md`.
- Bare water rejection for ordinary ground uses `SpeedType x LandType == 0`, indexed as `land_type * 9 + speed_type`. Source: `CAN_ENTER_CELL_MINIMAL_WATER_PIER_EVALUATOR_RESWARM2_20260527.md`.
- Reduced-zone reachability uses `MovementZone x ZoneType`, not SpeedType. Source: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`.
- `Zone_precheck` has no MovementZone row whitelist; valid rows reaching A* use the matrix row directly. Source: `MOVEMENTZONE_PRECHECK_ROWS_RESWARM2_20260527.md`.
- Bridge/tube/locomotor gates happen before object-list traversal in `UnitClass::Can_Enter_Cell`. Source: `CAN_ENTER_CELL_MINIMAL_WATER_PIER_EVALUATOR_RESWARM2_20260527.md`.
- Direction `8` is a tube transition and must not be smoothed or redirected as an ordinary adjacent step. Source: `CAN_ENTER_CELL_MINIMAL_WATER_PIER_EVALUATOR_RESWARM2_20260527.md`.
- High bridge legality depends on source/target level, bridge flags, ramp byte, and requested/effective bridge level. Source: `CAN_ENTER_CELL_MINIMAL_WATER_PIER_EVALUATOR_RESWARM2_20260527.md`.
- Speed/land zero rejection happens after selected object-list traversal in `UnitClass::Can_Enter_Cell`; a terrain-only helper must not claim full result-code parity. Source: `CAN_ENTER_CELL_MINIMAL_WATER_PIER_EVALUATOR_RESWARM2_20260527.md`.
- `CellRect::CheckPassability` is boolean and caller-configured; it is not the same as full `Can_Enter_Cell`. Source: `HELPER_PASSABLE_CELL_CONTRACTS_RESWARM_20260527.md`.
- `Find_Path -> FNPC` uses requested destination as search seed but current unit cell as target ranking point. Source: `FIND_PATH_FNPC_ARGUMENT_ROW_RESWARM2_20260527.md`.
- `Find_Path -> FNPC` fallback uses SpeedType, mapped MovementZone, current zone id, bridge context, `1x1`, height check enabled, `allow_bridge=1`, and `final_occupancy=0`. Source: `FIND_PATH_FNPC_ARGUMENT_ROW_RESWARM2_20260527.md`.
- WaterBridge `wbrdge` TMP terrain byte is `14` / Rough, not water, beach, or tunnel. Source: `WATERBRIDGE_TMP_TERRAIN_BYTES_RESWARM2_20260527.md`.
- `bypass_grid` remains an explicit choreography exception and must not become generic passability. Source: `PATHGRID_CALLER_OWNERSHIP_PRIORITY_RESWARM2_20260527.md`.

## Design

### Components

`cell_entry.rs`

- Own the new terrain/cell-entry evaluator API.
- Keep existing `CellEntryResult` and phase-2 occupancy classification.
- Add a richer phase-1 terrain result that can represent hard block, clear terrain, and deferred occupancy.

`passability.rs` / `zone_build.rs`

- Consolidate the verified 13x8 `MovementZone x ZoneType` matrix into one canonical source.
- Remove exact movement-zone legality paths that depend on `SpeedType` as the matrix row.

`core.rs`

- Replace non-water fallback in `is_cell_passable_for_mover` with evaluator-backed legality.
- Use evaluator for A* goal and neighbor checks.
- Keep `PathGrid` as candidate/layer geometry.

`movement_step.rs` / `movement_tick.rs`

- Use the same evaluator for runtime ground/bridge transition checks and drive-track chaining.
- Preserve `bypass_grid` as an explicit exception.

`movement_path.rs` / `path_smooth.rs`

- Use evaluator-backed closures for smoothing/reroute candidate checks.
- Later align reroute ordering and remove/gate unverified diagonal flank checks.

`scatter.rs`, `bump_crush.rs`, `miner_*`

- Migrate after the shared evaluator is established.
- Use caller policies matching native scatter/FNPC behavior.

### Interfaces / Contracts

Add a context object shaped around native inputs:

```text
CellEntryTerrainContext
- mover_category
- movement_zone
- speed_type
- source_cell
- target_cell
- direction
- current_layer
- requested_layer
- requested_bridge_level
- effective_bridge_level
- selected_list_mode
- locomotor_passability
- path_grid
- resolved_terrain
- terrain_costs
- occupancy
- bypass_grid
- mode
```

`mode` should distinguish at least:

- `AStarNeighbor`;
- `RuntimeTransition`;
- `Smoothing`;
- `Scatter`;
- `SpawnLike`.

FNPC / `FindNearbyPassable` should be a separate wrapper policy, not just a mode bit on this terrain context. It needs its own caller configuration:

```text
NearbyPassableContext
- origin_seed
- target_ranking_cell
- required_zone_id
- mapped_movement_zone
- speed_type
- bridge_aware
- rect_width
- rect_height
- reject_overlay
- height_check
- object_safety
- allow_bridge_cells
- final_occupancy
- direct_indirect_selection
```

That wrapper should call the cell-entry / `CheckPassability` slice for each candidate, but it also owns ring order, candidate caps, zone matching, target ranking, frame modulo behavior, and caller-specific final occupancy.

The first implementation phase should support the water/pier-critical subset:

- non-water ground rejects true water by SpeedType/LandType zero;
- WaterBridge Rough is accepted as ground where other blockers allow it;
- bridge/tube direction/layer checks are not bypassed by any-layer walkability;
- direction `8` is not ordinary adjacency;
- locomotor passability is represented as an explicit requested/not-requested input, even if the first ordinary-ground tests use the neutral result;
- high-bridge status-code parity that depends on mutable requested/effective bridge-level output remains bounded unless the first patch supplies those exact inputs.

### Data Flow

1. `PathGrid` provides candidate geometry and layer metadata.
2. Caller builds `CellEntryTerrainContext` from mover snapshot, resolved terrain, path layer, and source/target cells.
3. Evaluator checks coarse bounds/geometry, bridge/tube/layer facts, optional locomotor hard-block result, SpeedType/LandType zero legality, selected list/layer behavior, and optional mode-specific checks.
4. Caller either rejects candidate, accepts clear terrain, or proceeds to occupancy/result-code classification.
5. Runtime movement and A* use the same evaluator path for equivalent terrain questions.
6. A* reduced-zone precheck uses the canonical `MovementZone x ZoneType` matrix separately before cell A* where the native hierarchy path applies.
7. FNPC-style helpers use `NearbyPassableContext` to collect and rank candidates, then delegate candidate passability to the shared terrain/check-passability slice.

### Error Handling

Missing required terrain data should not silently make water legal. For parity-critical movement calls:

- missing `ResolvedTerrainGrid` should return blocked or a clearly logged diagnostic unless the caller is a known compatibility/test path;
- missing `PathGrid` may remain permissive only for legacy tests or explicit bypass paths;
- `bypass_grid` must be named and scoped at the call site.

### Testing Strategy

Core evaluator tests:

- ground mover rejects true water even when `PathGrid` says walkable;
- runtime/smoothing cell-entry tests do not use `MovementZone x ZoneType` as the only water rejection mechanism;
- WaterBridge TMP/Rough cell is not treated as water;
- shore/beach follows SpeedType/LandType, not `is_water`;
- bridge deck legality is layer/flag/level based;
- direction `8` requires tube data.

Integration tests:

- A* cannot route Crusher/Normal through true water without terrain costs;
- runtime transition rejects a stale path into water;
- move-goal redirection does not choose water for a ground mover;
- smoothing does not shortcut across water;
- scatter and miner staging skip water candidates once migrated.

Matrix tests:

- one canonical 13x8 matrix matches verified `0x0082A594` rows;
- reduced-zone precheck is enabled for valid rows, including `Crusher`, `Destroyer`, `CrusherAll`, `Water`, and `WaterBeach`;
- SpeedType is not used as matrix row.

FNPC policy tests:

- destination seed and target ranking cell can differ;
- mapped MovementZone and required current-zone id are explicit inputs;
- `allow_bridge_cells=1` does not globally accept bare water;
- `final_occupancy=0` does not accidentally enable final `CheckOccupancy`.

## Architectural Decisions

- Keep `PathGrid` coarse. This avoids breaking hover/naval/bridge compatibility paths before caller-specific legality exists.
- Extend `cell_entry.rs` instead of creating unrelated helper modules. The module already owns result codes and borrow-safe terrain/occupancy separation.
- Do not implement the full object-list `Can_Enter_Cell` tree in the first phase. The water/pier gap needs the terrain/layer slice; full occupant priority remains a later parity phase.
- Do not fold reduced-zone reachability into runtime cell-entry. The matrix is a route-planning/hierarchy and zone-constraint mechanism, while `Can_Enter_Cell` uses speed/land plus bridge/tube/layer and caller-specific checks for this bug class.
- Do not make FNPC just another boolean passability call. Its caller row owns seed, target ranking, candidate cap, zone id, bridge allowance, final occupancy, and direct/indirect selection.
- Patch A* and runtime movement transition together to prevent path/execution disagreement.
- Defer production/spawn/edge/drop callers until the movement-facing path is stable.

## Alternatives Considered

### Direct Caller Patches

Rejected. It would spread separate passability approximations into A*, smoothing, scatter, miner, and runtime movement, recreating the drift.

### Global `PathGrid` Semantics Change

Rejected as first fix. Native legality is mover-specific, layer-sensitive, and caller-configured. Making water globally blocked in `PathGrid` would be too blunt and could break naval, amphibious, hover, bridge, and compatibility paths.

### Native-Shaped Evaluator Above `PathGrid`

Chosen. It matches the existing module boundary, preserves caller-specific policy, and lets `PathGrid` remain a structural candidate graph.

## Phased Implementation

1. Add evaluator API and tests in `cell_entry.rs`.
2. Wire A* and runtime movement transition together.
3. Unify the verified matrix and widen reduced-zone precheck.
4. Replace or gate move-goal redirection with a native-shaped destination probe and FNPC wrapper policy.
5. Replace smoothing/reroute candidate legality, then align smoothing algorithm details.
6. Replace scatter and miner staging helpers.
7. Clean up production/refinery/edge/drop callers.
8. Update diagnostics and tests that currently equate `PathGrid` walkability with passability.

## Open Follow-Ups

- Concrete repro-cell trace is still useful for confirming the visible map symptom, but it no longer blocks the shared evaluator architecture.
- Full object-list `Can_Enter_Cell` priority remains a later parity contract.
- Exact land-type-10 tile-set branch semantic names remain unresolved outside the immediate water/pier gap.
- Exact mutable bridge-level output semantics remain a parity follow-up unless the first implementation threads the full requested/effective bridge-level state.
- FNPC exact caller rows other than the verified `Find_Path` fallback and miner staging rows remain caller-specific follow-ups.
