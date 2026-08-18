# AStar Water/Pier Neighbor Legality Re-Swarm Slot 2

Date: 2026-05-27

Slot: 2

Target: exact active-YR `gamemd.exe` A* neighbor expansion legality for water/pier cells and how `MovementZone`, reduced `ZoneType`, `SpeedType`, and cost tables participate.

Required output path: `docs/research/pathfinding/ASTAR_WATER_PIER_NEIGHBOR_LEGALITY_RESWARM_20260527.md`

## Scope And Evidence Status

This slot could not perform fresh live Ghidra MCP reads because no Ghidra MCP tools were exposed in the session. `tool_search` only exposed Codegraph, GitHub, and Node tools. No Ghidra mutation was possible or performed.

Evidence below is therefore split as:

- `VERIFIED-FROM-PRIOR-GHIDRA-DOCS`: facts cited from existing Ghidra-backed reports and reports that themselves cite decompile, assembly, xrefs, or memory reads.
- `SOURCE-VERIFIED`: current Rust source read in this slot.
- `UNCHECKED-FRESH-MCP`: facts that should be spot-checked in a session with Ghidra MCP before being treated as newly reverified by this slot.

No Rust code, INI files, Ghidra state, or existing research docs were edited. This report is the only file written by this slot.

## Executive Summary

The docs agree on the core contract: active YR pathfinding does not decide water/pier legality from a single boolean `PathGrid::is_walkable()` equivalent.

For normal ground units such as a Chrono Miner, the important active-YR gates are:

1. `MovementZone` from `TechnoTypeClass+0x5B4` selects a row in the 13x8 `ZonePassabilityMatrix`.
2. `CellClass::RecalcZoneType @ 0x00483C80` writes reduced `ZoneType` to `CellClass+0x4C`; water is column 4.
3. `Zone_precheck @ 0x0042C290` and zone rebuild/readers use `matrix[MovementZone][ZoneType] == 1`; values 2 and 3 block.
4. `AStar_main_loop @ 0x00429A90` expands normal compass neighbors, gates by hierarchy marker state when enabled, calls the mover's `Can_Enter_Cell`, and only then computes edge cost from the `Can_Enter_Cell` result code.
5. `SpeedType` from `TechnoTypeClass+0x67C` is separate. It feeds the speed/land table zero-passability check in cell-entry/passability helpers, not the zone matrix row.
6. `AStar_compute_edge_cost @ 0x00429830` uses a fixed Can_Enter-code cost table, code-2 urgency logic, search marker x4, bridge flank multipliers, and direction epsilon. It does not read terrain-speed percentages as A* route weights.

Current Rust has multiple parity risks that can produce the visible "unit drives on water/outside pier" class of bug:

- `PathGrid::from_resolved_terrain_with_bridges` deliberately marks water as `ground_walkable`.
- non-water goal redirection and smoothing still use `PathGrid::is_walkable` / `is_any_layer_walkable`.
- `zone_search::can_use_reduced_zone_precheck` skips many live ground `MovementZone` rows, including `Crusher`, which is the likely Chrono Miner row.
- A* can run with `TerrainCostGrid` absent, where water stays walkable and receives uniform cost.
- Rust route cost uses `TerrainCostGrid` speed percentages, while gamemd A* route cost uses Can_Enter return-code cost.

Fresh MCP could still refine the exact pier branch, especially high/low bridge and pier-tile classification, but the A* legality/cost mismatch is real and implementation-relevant.

## VERIFIED Binary Findings

### 1. A* path entry is MovementZone based, not SpeedType based

Status: `VERIFIED-FROM-PRIOR-GHIDRA-DOCS`

`ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` verifies `ZonePassabilityMatrix` as `int[13][8]` at `0x0082A594`, rows keyed by `MovementZone` (`TechnoTypeClass+0x5B4`) and columns keyed by reduced `CellClass+0x4C` `ZoneType`. Only value `1` passes; `2` and `3` block.

Relevant active path:

- `FootClass::Find_Path @ 0x004D3920`
- `FootClass::Run_AStar @ 0x004CBBA0`
- `AStar_pathfind_search @ 0x0042C900`
- `Zone_precheck @ 0x0042C290`

The same report verifies `SpeedType` is stored separately at `TechnoTypeClass+0x67C` and is not a direct matrix row selector in the documented direct readers.

Active in standard YR: yes, per the pathfinding call chain above.

### 2. Water is reduced ZoneType column 4, and normal ground rows block it

Status: `VERIFIED-FROM-PRIOR-GHIDRA-DOCS`

`CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md` verifies `CellClass::RecalcZoneType @ 0x00483C80` writes:

- water `LandType == 2` -> reduced `ZoneType = 4`
- beach `LandType == 6` -> reduced `ZoneType = 3`
- out-of-playfield -> reduced `ZoneType = 7`

`ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` verifies the relevant rows:

- row 0 `Normal`: `[1,2,2,2,2,2,2,3]`
- row 1 `Crusher`: `[1,1,2,2,2,2,2,3]`
- row 10 `Water`: `[2,2,2,2,1,2,2,3]`
- row 11 `WaterBeach`: `[2,2,2,1,1,2,2,3]`

Therefore a `MovementZone::Crusher` or `MovementZone::Normal` ground unit does not have zone connectivity through water column 4.

Active in standard YR: yes.

### 3. `AStar_pathfind_search @ 0x0042C900` uses zone IDs and `Zone_precheck` before cell A*

Status: `VERIFIED-FROM-PRIOR-GHIDRA-DOCS`

`ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`, `ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`, and `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` agree:

- `AStar_pathfind_search` reads/uses the mover's `MovementZone` row when no explicit override applies.
- If start and destination zone IDs differ while hierarchy is enabled, the wrapper returns failure before `AStar_main_loop`.
- If same-zone `Zone_precheck` fails, hierarchy is disabled and cell A* may still run.
- Default retry budget is five total A* attempts for the default `-1` limit.
- Retry exclusions are per-search undirected zone-edge pairs, not whole-zone bans.

Active in standard YR: yes.

### 4. `AStar_main_loop @ 0x00429A90` expands neighbors through marker gate, layer selection, `Can_Enter_Cell`, then edge cost

Status: `VERIFIED-FROM-PRIOR-GHIDRA-DOCS`

`ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md` verifies:

- normal neighbor directions are `0..7`; direction `8` is a tube edge path.
- layer selection happens before selected closed-list lookup.
- ground and bridge closed/g-cost arrays are separate.
- `Can_Enter_Cell` and edge/tube cost work occur around `0x00429F37..0x00429FEA`.
- an impassable neighbor result can still trigger blocked-goal fallback when the neighbor is the destination and height delta is acceptable.

`ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md` verifies:

- `Zone_precheck` writes chosen-zone marker arrays.
- `AStar_main_loop` consumes the level-0 marker before calling `Can_Enter_Cell`.
- off-marker candidates with `CellClass+0x122 != 0` can still be allowed in hierarchical mode.

Active in standard YR: yes.

### 5. A* edge cost is Can_Enter-code based, not terrain-speed-percentage based

Status: `VERIFIED-FROM-PRIOR-GHIDRA-DOCS`

`ASTAR_COMPUTE_EDGE_COST_00429830_MARKER_STACKING_GHIDRA_REPORT.md` and `bridges/03-traversal-pathfinding-entry/ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md` verify:

- `AStar_compute_edge_cost @ 0x00429830` loads base cost from `0x0081870C[Can_Enter_Cell_code]`.
- base table values are `0:1.0`, `1:1000.0`, `2:1.0`, `3:1.0`, `4:60.0`, `5:20.0`, `6:8.0`, `7:10000.0`.
- code 2 is adjusted by `PathfinderClass+0x3C` urgency and blocker prediction.
- destination `CellClass+0x140 & 0x40000` multiplies the current edge by `4.0`.
- bridge flank cost can multiply by `10.0`, `1.0`, or `2.0`.
- direction epsilon is added after helper return.
- direction 8 bypasses this helper.

No INI terrain-speed percentage is read by the edge-cost helper.

Active in standard YR: yes for normal A* expansion; bridge branches are conditional on bridge/layer inputs.

### 6. SpeedType still matters, but as zero/nonzero passability in entry helpers, not as the zone row

Status: `VERIFIED-FROM-PRIOR-GHIDRA-DOCS`

`CELLRECT_CHECKPASSABILITY_0056E7C0_FULL_ARG_DECODE_GHIDRA_REPORT.md` verifies `CellClass::CheckCellPassability @ 0x004834A0` uses:

- `MovementZone` for zone ID lookup/row family.
- `SpeedType` for `g_SpeedType_LandType_Table[speed_type + LandType*9]`.
- exact `0.0` speed-table values reject non-bridge selected terrain paths.

`miner/traces/MINER_STUCK_TIBERIUM_PASSABILITY_BYPASS_TRACE.md` further verifies a live `UnitClass::Can_Enter_Cell @ 0x0073F0A0` speed-table check at `0x0073FAB5`: `table[LandType*9 + SpeedType] == 0.0 -> return 7`.

Inference for water/pier: for a ground `SpeedType::Wheel` or `Track` unit, water terrain with speed table zero should return impassable through the mover's cell-entry predicate even when a coarse boolean grid calls it walkable. Fresh Ghidra MCP should re-check the exact water branch in `UnitClass::Can_Enter_Cell @ 0x0073F0A0` if this becomes the final implementation contract.

Active in standard YR: yes for unit movement; water-specific unit branch is `UNCHECKED-FRESH-MCP` in this slot.

## Current Rust Touchpoints

Status: `SOURCE-VERIFIED`

### `src/sim/pathfinding/core.rs`

- `astar_search` goal passability uses `is_cell_passable_for_mover` at lines 836-844.
- normal neighbor passability for ground uses `is_cell_passable_for_mover` at lines 1133-1149.
- terrain cost blocks only if `TerrainCostGrid::cost_at` returns 0; when no cost grid is present, cost defaults to 100 at lines 1217-1225.
- route cost scales by terrain speed percentage at lines 1252-1257.
- height-change cost multiplies by `CLIFF_COST_MULTIPLIER` at lines 1259-1262.
- marker cost is applied after entity cost at line 1284.
- direction 8 tube edge is separate at lines 1320-1359.
- non-water `is_cell_passable_for_mover` falls back to `grid.is_walkable` at line 1415.
- water movers use `is_water_surface_cell_passable`, which has a permissive `cell.is_water -> true` fallback at lines 1374-1390.
- `PathGrid::from_resolved_terrain_with_bridges` sets water ground walkability via `!cell.ground_walk_blocked || cell.is_water` at line 1835.

### `src/sim/pathfinding/zone_search.rs`

- `can_use_reduced_zone_precheck` enables reduced zone precheck only for `None`, `Normal`, `Amphibious`, `Infantry`, and `Fly`, and returns false for all other rows at lines 62-75.
- `find_path_zoned_marker_inner` immediately falls back to `find_path_with_costs_marker` when reduced precheck is disabled at lines 227-240.

This means `MovementZone::Crusher`, `Destroyer`, `AmphibiousCrusher`, `CrusherAll`, `Water`, `WaterBeach`, and other live rows can bypass the MovementZone zone gate entirely in Rust.

### `src/sim/pathfinding/zone_build.rs`

- `MOVEMENT_CLASS_PASSABILITY` at lines 39-53 matches the verified binary rows, including Normal/Crusher blocking water and Water allowing only water.
- terrain-aware zone build uses `ResolvedTerrainCell.zone_type`, via `movement_class_for_cell` at lines 197-215.

This file is currently closer to the binary matrix contract than `passability.rs`.

### `src/sim/pathfinding/passability.rs`

- comments describe the binary row/column concept, but `PASSABILITY_MATRIX` at lines 115-143 does not match the verified binary rows from `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`.
- example mismatches: Normal row column 2 and column 6 are `1` in Rust but verified binary has `2`; Fly row column 7 is `1` in Rust but verified binary has `3`; Subterranean row column 5 and 7 differ from verified rows.
- `zone_layer_for_speed_type` still maps `SpeedType` to a matrix row at lines 145-160; existing docs explicitly warn this is not a direct binary reader behavior.

For water specifically, rows 10 and 11 still preserve the main water/beach pattern, but the file remains risky for any fallback path that claims exact binary matrix parity.

### `src/sim/movement/movement_path.rs`

- non-water goal acceptance uses `grid.is_any_layer_walkable` at lines 60-76.
- non-water nearest-goal redirection uses `grid.nearest_walkable_any_layer` at lines 78-88.
- layered path smoothing uses `grid.is_walkable_on_layer` at lines 242-264.
- flat smoothing for non-water movers uses `grid.is_walkable` at lines 304-325.

Because `PathGrid` marks water as ground-walkable, these goal/smoothing paths can reintroduce water/pier shortcuts even if A* with a cost grid avoided them.

## DRIFT / UNCHECKED Findings

### D1 - `PathGrid` water is ground-walkable for non-water movers

Verdict: `DRIFT`

Evidence:

- Binary: Normal and Crusher rows block reduced `ZoneType=4` water in `ZonePassabilityMatrix`; `Can_Enter_Cell` speed-table zero also blocks incompatible terrain.
- Rust: `PathGrid::from_resolved_terrain_with_bridges` sets `ground_walkable = !cell.ground_walk_blocked || cell.is_water` for non-bridge, non-cliff cells.

Player-visible risk: ground units can select, redirect toward, smooth through, or path through water/pier-adjacent cells if a later caller uses `PathGrid` without the exact `MovementZone`/`SpeedType` legality context.

Likely pier symptom contribution: high.

### D2 - Reduced-zone precheck skips live ground rows, including likely Chrono Miner `Crusher`

Verdict: `DRIFT`

Evidence:

- Binary: `Zone_precheck` accepts the `MovementZone` row passed by `AStar_pathfind_search`; verified matrix has 13 rows and direct readers key by `MovementZone`.
- Rust: `can_use_reduced_zone_precheck` returns false for all rows except Normal, Amphibious, Infantry, Fly, and `None`.

For a Chrono Miner using `MovementZone::Crusher`, Rust bypasses the row-1 matrix that blocks water and goes directly to cell A*. If `TerrainCostGrid` is absent or a later smoother/goal helper uses `PathGrid`, water can become legal in practice.

Likely pier symptom contribution: high.

### D3 - Rust A* can run with no `TerrainCostGrid`, making water uniformly traversable

Verdict: `DRIFT`

Evidence:

- Binary: normal A* calls `Can_Enter_Cell`; edge cost is based on the `Can_Enter_Cell` code and code 7 is rejected except blocked-goal fallback.
- Rust: in `astar_search`, missing `options.terrain_costs` yields terrain cost 100 for non-water movers.

This is a hard bug for any caller that supplies `PathGrid` but no cost grid. Since `PathGrid` water is ground-walkable, water is then not rejected.

Likely pier symptom contribution: medium to high, depending on caller.

### D4 - Rust A* route cost uses terrain speed percentages; gamemd A* edge cost does not

Verdict: `DRIFT`

Evidence:

- Binary: `AStar_compute_edge_cost @ 0x00429830` reads `0x0081870C[Can_Enter_Cell_code]`, code-2 urgency, marker/flank multipliers, and epsilon. No INI terrain speed percentage is read in the helper.
- Rust: `step_cost = base_cost * 100 / terrain_cost` when terrain cost is not 100.

This is not necessarily the direct water-on-pier cause, because zero cost still blocks, but it is an exact-mechanism A* parity drift and can change route choices near beaches, rough terrain, roads, ore, and bridge approaches.

Likely pier symptom contribution: medium.

### D5 - Goal redirection and smoothing can bypass mover-specific legality

Verdict: `DRIFT`

Evidence:

- Binary: A* neighbor legality is via hierarchy marker plus `Can_Enter_Cell` and layer/bridge handling; path smoothing reports also cite vtable `Can_Enter_Cell` checks and marker checks.
- Rust: `movement_path.rs` goal acceptance and smoothing use `PathGrid::is_any_layer_walkable`, `is_walkable_on_layer`, or `is_walkable` for non-water movers.

Because `PathGrid` marks water as ground-walkable, smoothing can shorten a legal land route across water/pier-adjacent cells after A* did more restrictive work.

Likely pier symptom contribution: high if the observed unit path looks like a shortcut after initial path selection.

### D6 - `passability.rs` matrix is not the verified binary matrix

Verdict: `DRIFT`

Evidence:

- Binary: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` gives the verified rows.
- Rust: `passability.rs::PASSABILITY_MATRIX` differs from verified rows for multiple columns and still exposes SpeedType-to-row mapping.

Water rows mostly preserve the obvious water/beach shape, so this is not the top pier-specific cause. It remains a blocker for exact `MovementZone`/`ZoneType` parity anywhere the compatibility helper is used instead of `zone_build.rs`'s matrix.

Likely pier symptom contribution: low to medium for this specific symptom, high for broader pathing parity.

### D7 - Exact pier tile classification is still not freshly verified by this slot

Verdict: `UNCHECKED-FRESH-MCP`

Evidence:

- Existing docs verify water, beach, high bridge, low bridge/tube, and bridge structural flags in adjacent systems.
- This slot did not fresh-decompile a specific pier tile path or a concrete repro map cell.

Open branch: a "pier" visual tile could be water, beach, low bridge/tube, bridge deck, bridgehead, or ordinary ground depending on TMP/overlay/theater metadata. The exact patch point may differ between a true water cell, a bridge deck over water, and a low bridge/tube cell.

Likely pier symptom contribution: unknown until a repro cell is classified.

## Implementation Handoff

Do not fix this by only changing one visible symptom helper. The correct implementation direction is to split static topology from mover-specific cell-entry legality.

Required deltas:

1. Replace non-water use of `PathGrid::is_walkable` as path legality with a binary-shaped cell-entry predicate.
   - Inputs: `MovementZone`, `SpeedType`, reduced `ZoneType`, final LandType/speed-table value, bridge layer/height, occupation/object layer context, structural bridge flags, and current goal exception.
   - Affected surfaces: `src/sim/pathfinding/core.rs`, `src/sim/movement/movement_path.rs`, helper callers like scatter/miner staging.

2. Enable `Zone_precheck` semantics for every live `MovementZone` row unless a specific binary path proves a row bypasses it.
   - Affected surface: `src/sim/pathfinding/zone_search.rs::can_use_reduced_zone_precheck`.
   - Acceptance: `MovementZone::Crusher` start/goal separated by water returns no path before unrestricted A*; same-zone precheck failure still follows the verified same-zone fallback behavior.

3. Preserve the verified binary matrix in one canonical place.
   - Affected surfaces: `src/sim/pathfinding/passability.rs`, `src/sim/pathfinding/zone_build.rs`.
   - Acceptance: Normal/Crusher cannot pass water column 4; Fly blocks sentinel column 7; only value `1` passes.

4. Stop treating terrain speed percentages as A* route weights unless a separate verified A* reader proves a speed-weighted path cost.
   - Affected surface: `src/sim/pathfinding/core.rs` step-cost construction and `TerrainCostGrid` role.
   - Acceptance: nonzero rough/road/ore speed values affect movement timing/entry legality where verified, not A* edge preference, while exact zero still rejects terrain in the appropriate Can_Enter path.

5. Make smoothing and goal redirection call the same mover-specific legality predicate used by A*.
   - Affected surface: `src/sim/movement/movement_path.rs`.
   - Acceptance: a non-water ground unit cannot get a smoothed segment through a water cell that A* would have rejected.

6. Add a repro-classification trace for the actual pier map/cell.
   - Inputs to log: map name, cell coord, final tile/subtile, `ResolvedTerrainCell.zone_type`, `is_water`, `land_type`, `yr_cell_land_type`, `ground_walk_blocked`, `has_bridge_deck`, `bridge_walkable`, `bridge_transition`, `tube_index`, `PathGrid.ground_walkable`, `TerrainCostGrid` for the mover `SpeedType`, and movement `MovementZone`.
   - This determines whether the visible bug is true water walkability, bridge/pier classification, smoothing, or goal redirection.

## Acceptance Tests To Add Later

- `astar_crusher_movement_zone_blocks_water_even_when_pathgrid_water_walkable`
- `zone_precheck_runs_for_crusher_and_blocks_disconnected_water_goal`
- `astar_without_cost_grid_does_not_make_water_legal_for_ground_mover`
- `flat_smoothing_uses_mover_legality_not_pathgrid_walkable`
- `move_goal_redirection_does_not_choose_water_for_normal_or_crusher_mover`
- `passability_matrix_binary_rows_match_0x0082A594`
- `astar_edge_cost_does_not_weight_nonzero_speed_percentages`
- `pier_repro_cell_classification_matches_gamemd_zone_and_bridge_layer`

## Open Questions

1. Fresh Ghidra MCP should re-check the exact water/SpeedType branch inside `UnitClass::Can_Enter_Cell @ 0x0073F0A0` for ground vehicle movement, especially whether water returns code 7 only via speed-table zero or an earlier water/naval branch.
2. Fresh Ghidra MCP should re-check whether any standard-YR `MovementZone` rows intentionally bypass hierarchy in `AStar_pathfind_search`; current docs imply the row is accepted generally.
3. The exact pier repro cell must be classified against retail map/TMP/overlay data. Without a concrete map coordinate, "pier" remains ambiguous.
4. Bridge layer entry around high bridge over water may involve `CheckBridgeTraversal @ 0x004D9C60`; this slot used existing bridge A* docs but did not re-audit the traversal function.
5. Low bridge/tube cells use direction-8 semantics and can bypass the normal edge-cost helper. A pier built from low bridge/tube pieces needs a dedicated trace if the repro lands there.

## Shared Claims

- slot-2 claim: active-YR A* water legality is `MovementZone`/reduced `ZoneType` plus `Can_Enter_Cell`, not `PathGrid::is_walkable`.
- slot-2 claim: `MovementZone::Crusher` must not bypass reduced-zone reachability by default; Rust currently does.
- slot-2 claim: `SpeedType` must not be used as the zone-matrix row; it belongs to speed/land entry checks and movement timing/cost where separately verified.
- slot-2 claim: `AStar_compute_edge_cost @ 0x00429830` is Can_Enter-code based and does not use terrain speed percentages as route weights.
- slot-2 claim: the exact pier visual symptom still needs a concrete cell classification trace before choosing between PathGrid construction, goal redirection, smoothing, or bridge classification as the immediate patch point.

## Sources Read

- `docs/research/ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md`
- `docs/research/CELLRECT_CHECKPASSABILITY_0056E7C0_FULL_ARG_DECODE_GHIDRA_REPORT.md`
- `docs/research/NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md`
- `docs/research/ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`
- `docs/research/ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md`
- `docs/research/ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`
- `docs/research/ASTAR_COMPUTE_EDGE_COST_00429830_MARKER_STACKING_GHIDRA_REPORT.md`
- `docs/research/bridges/03-traversal-pathfinding-entry/ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`
- `docs/research/bridges/02-cell-state-layering-zones/BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
- `docs/research/miner/traces/MINER_STUCK_TIBERIUM_PASSABILITY_BYPASS_TRACE.md`
- `src/sim/pathfinding/core.rs`
- `src/sim/pathfinding/zone_search.rs`
- `src/sim/pathfinding/zone_build.rs`
- `src/sim/pathfinding/passability.rs`
- `src/sim/pathfinding/terrain_cost.rs`
- `src/sim/movement/movement_path.rs`

