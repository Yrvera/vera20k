# Helper Passable Cell Contracts Reswarm - 2026-05-27

**Slot:** 5  
**Target:** Generic helper/caller contracts for nearby passable cells around pathing: scatter candidate selection, miner/refinery staging/exit cells, nearest passable/goal redirection, and `Find_Nearby_Passable_Cell`-style functions.  
**Status:** PARTIAL. The main contract question is answered: active gamemd callers do not use a coarse boolean pathgrid; they use `Can_Enter_Cell` or `Find_Nearby_Passable_Cell` -> `CellRect::CheckPassability` with mover/caller-specific inputs. The remaining partial item is the exact 15-stack-argument row for the `FootClass::Find_Path` call to `0x0056DC20`, which Ghidra decompiler still collapses enough that it needs an assembly push walk before it should become a unique fixture.

## Target Question

Do helper/caller paths that choose nearby "passable" cells in active Yuri's Revenge use mover-specific `Can_Enter_Cell` / `CheckPassability` contracts, or do they use a coarse pathgrid-style boolean?

## Non-Goals

- Do not edit Rust code.
- Do not mutate Ghidra state.
- Do not re-open all A* internals or full bridge/tube pathing.
- Do not resolve every one of the ~47 `Find_Nearby_Passable_Cell` callers if their parameter class is already covered by existing caller-matrix docs.

## Evidence Needed To Mark COMPLETE

- Fresh read-only Ghidra evidence for `FootClass::Find_Nearby_Passable_Cell @ 0x0056DC20`.
- Fresh read-only Ghidra evidence for `CellRect::CheckPassability @ 0x0056E7C0` and `CellRect::CheckOccupancy @ 0x00586780`.
- Fresh read-only Ghidra evidence for `UnitClass::Scatter @ 0x00743A50` and at least one path goal redirection caller.
- Current Rust touchpoint scan for direct `PathGrid::is_walkable` helper callsites.

## Stop Conditions

- Stop after helper/caller contract and Rust deltas are clear.
- Record unresolved exact push rows as remaining uncertainty rather than inferring.
- Do not patch Rust.

## Verified Binary Findings

### 1. `Find_Nearby_Passable_Cell` is not a coarse walkability probe

Active in YR: Yes.

Fresh Ghidra decompile at `0x0056DC20` confirms the helper:

- normalizes zone id `0xFFFF` to `-1`;
- derives search radius from receiver fields `+0xF4 + +0xF8`, capped to `32`;
- scans square/Chebyshev perimeter rings in top/bottom rows, then left/right columns;
- collects up to `24` accepted candidates;
- calls `CellRect__CheckPassability(&candidate, width, height, speed_type, zone_id, movement_zone, -1, bridge_aware, reject_overlay)` for every candidate;
- optionally calls `TechnoClass__Is_Current_Cell_Obstacle_Free` when the caller enables the object-safety flag;
- optionally rejects structural bridge cells when the allow-bridge flag is zero;
- optionally calls `CellRect__CheckOccupancy(rect, -1)` when final occupancy is enabled;
- partitions direct vs indirect candidates through `FUN_006D6410` and chooses by `g_CurrentFrameCounter % count` for a null target, or nearest-to-target for a real target.

Evidence: `0x0056DC20` decompile; calls to `CellRect__CheckPassability` at `0x0056DE0E`, `0x0056E024`, `0x0056E265`, `0x0056E467`; candidate cap checks against `0x18`; selection block `0x0056E5B3..0x0056E79A`.

### 2. `CellRect::CheckPassability` is mover/caller-specific

Active in YR: Yes.

Fresh Ghidra decompile at `0x0056E7C0` confirms rectangle iteration over width/height and an overlay reject precheck before calling `CellClass__CheckCellPassability`. Existing validator reports decode the full stack signature:

```text
CheckPassability(top_left, width, height,
                 speed_type, required_zone_id, movement_zone,
                 required_height_or_level, bridge_aware_zone,
                 reject_any_overlay)
```

The current live decompile shows the first three named parameters explicitly and stack use for the trailing caller flags. Sibling report `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md` already verifies that the callee path uses SpeedType/LandType speed checks, zone id via MovementZone, bridge/height/occupation fields, and overlay handling.

Evidence: fresh `0x0056E7C0` decompile; sibling validator report; `CellClass__CheckCellPassability @ 0x004834A0`.

### 3. `CellRect::CheckOccupancy` is separate from passability

Active in YR: Yes.

Fresh Ghidra decompile at `0x00586780` confirms `CheckOccupancy(rect, layer)`:

- skips `Cell+0xDC` reservation bits when layer is `-1`;
- scans the rectangle using fixed 512-wide cell indexing;
- rejects object/list blockers, reservation bits when enabled, `Cell+0x44`, `Cell+0x4C`, `Cell+0x11C`, and building lookup blockers;
- finishes with `MapClass__IsRectInPlayfield(rect, 1)`.

`Find_Nearby_Passable_Cell` calls this only when the caller enables final occupancy and always passes `-1`.

Evidence: fresh `0x00586780` decompile; FNPC calls to `CellRect__CheckOccupancy(..., 0xffffffff)`.

### 4. Unit scatter uses both FNPC and `Can_Enter_Cell`, depending on branch

Active in YR: Yes.

Fresh Ghidra decompile at `0x00743A50` confirms two relevant scatter contracts:

- Null-coordinate / random scatter branch calls `FootClass__Find_Nearby_Passable_Cell` with the unit's `Type+0x67C` SpeedType, zone `-1`, MovementZone-style argument `0`, current on-bridge flag, `1x1` rectangle, height check enabled, bridge cells allowed, and final occupancy disabled.
- Directional scatter branch scans the 8 neighboring cells, checks `MapClass__Is_Cell_In_Playfield`, then calls the unit vtable `+0x1AC` (`UnitClass::Can_Enter_Cell @ 0x0073F0A0` for vehicles) with the candidate `CellClass`, direction, and `CellClass::Get_Effective_Height`. Only return code `0` is accepted. It then runs height snap through `FUN_006D6410` and rejects structural bridge cells for the ideal candidate.

This means native scatter is mover-specific. It is not `PathGrid::is_walkable`.

Evidence: fresh `0x00743A50` decompile; fresh `0x0073F0A0` decompile; existing `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`.

### 5. `FootClass::Find_Path` destination fallback uses `Can_Enter_Cell`, FNPC, and zone scoring

Active in YR: Yes.

Fresh Ghidra decompile at `0x004D3920` confirms:

- the requested destination is converted to a `CellClass`;
- vtable `+0x1AC` is called before A*;
- return code `6` can trigger a fallback when distance exceeds close-enough and the unit is not naval;
- return code `7` can trigger a building-destination fallback when `Look_up_building_in_cell` succeeds;
- both fallback arms call `FootClass__Find_Nearby_Passable_Cell`;
- the code-6 fallback compares the returned candidate against null, measures candidate distance, and calls `PathfinderClass__EstimateZoneCost`; it accepts only when the estimated zone cost is `<= chebyshev_delta + 6`;
- accepted fallback calls vtable `+0x480` Set_Destination and substitutes the A* target.

The exact `0x0056DC20` stack argument row inside this large function remains unresolved by decompile alone. Still, the path is clearly not Rust's current generic nearest-walkable pre-command redirect.

Evidence: fresh `0x004D3920` decompile; sibling reports `FOOTCLASS_FIND_PATH_BLOCKED_DESTINATION_FALLBACK_GHIDRA_REPORT.md`, `FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`.

### 6. Chrono/standard miner far-return staging uses FNPC, not direct pathgrid

Active in YR: Yes for state-2 fallback when close refinery reservation/radio path does not fire.

Fresh Ghidra decompile at `0x0073E5E0` confirms `UnitClass::Mission_Harvest` computes `refinery_anchor + BuildingType.QueueingCell`, then calls `FootClass__Find_Nearby_Passable_Cell(..., speed_type=2, zone=-1, movement_zone=0, bridge_aware=0, width=1, height=1, reject_overlay=0, height_check=0, object_safety=0, allow_bridge=1, target={0,0}, skip=0, final_occupancy=0)`. A null result clears destination; otherwise it converts the returned cell through `MapClass__Get_CellClass` and calls Set_Destination.

Evidence: fresh `0x0073E5E0` decompile; prior `miner/FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md`.

## Current Rust Touchpoints

### `miner_dock_sequence.rs`

- `src/sim/miner/miner_dock_sequence.rs:268` `is_exit_cell_passable` uses `grid.is_walkable(cx, cy)` plus optional `OccupancyGrid` ground emptiness.
- `src/sim/miner/miner_dock_sequence.rs:303` `find_nearby_passable_cell` returns the first passable cell, although comments acknowledge native candidate collection.
- `src/sim/miner/miner_dock_sequence.rs:353` `find_nearby_passable_cell_with_index` collects a ring and modulo-picks, but it still uses `PathGrid::is_walkable`, lacks `CheckPassability` SpeedType/MovementZone/rect flags, lacks direct/indirect split, and uses `EXIT_SEARCH_MAX_RADIUS`.

### `miner_system.rs`

- `src/sim/miner/miner_system.rs:1226` `chrono_return_staging_cell_for_sid` seeds from `QueueingCell` and calls the Rust nearby helper with `path_grid`, no occupancy, `EXIT_SEARCH_MAX_RADIUS`, and `sim.tick`.
- `src/sim/miner/miner_system.rs:1431` `issue_move_if_idle` calls `movement::issue_move_command` without resolved terrain or terrain costs.

### `movement_path.rs` and `movement_commands.rs`

- `src/sim/movement/movement_path.rs:60` `is_move_goal_walkable` checks water movers through `is_cell_passable_for_mover`, but non-water movers accept `grid.is_any_layer_walkable`.
- `src/sim/movement/movement_path.rs:78` `nearest_move_goal` uses `grid.nearest_walkable_any_layer` for non-water movers.
- `src/sim/movement/movement_commands.rs:302` redirects every requested move goal through `resolve_requested_move_goal(..., max_radius=10)` before A*, rather than decoding the native `Find_Path` destination-probe fallback.

### `scatter.rs` and `bump_crush.rs`

- `src/sim/movement/scatter.rs:134` idle scatter filters candidate cells with `grid.is_walkable`.
- `src/sim/movement/scatter.rs:310` `find_passable_scatter_cell` uses `PathGrid::is_walkable` plus simplified `OccupancyGrid` rules.
- `src/sim/movement/bump_crush.rs:655` `scatter_blocker` uses `PathGrid::is_walkable` for adjacent scatter candidates.

### `pathfinding/core.rs`

- `src/sim/pathfinding/core.rs:1392` `is_cell_passable_for_mover` explicitly says it is still a local path-grid legality gate, not native `Can_Enter_Cell`.
- `src/sim/pathfinding/core.rs:1835` builds `ground_walkable` as `!cell.ground_walk_blocked || cell.is_water` for ordinary non-bridge terrain. Any helper that treats this boolean as ground-unit cell-entry legality can accept water.

## DRIFT / UNCHECKED Findings

### D1 - DRIFT - Helper passability collapses native per-caller contracts into `PathGrid::is_walkable`

Native FNPC calls `CheckPassability` with SpeedType, zone id, MovementZone, bridge-aware flag, rect width/height, overlay rejection, height/safety flags, allow-bridge filtering, and optional `CheckOccupancy`. Current Rust helper sites use `PathGrid::is_walkable` plus simplified occupancy.

Affected surfaces: miner staging/exit, scatter, goal redirection, production/spawn-style future reuse.

### D2 - DRIFT - Non-water goal redirection can accept any walkable layer

Native `Find_Path` probes the destination with `Can_Enter_Cell` and uses FNPC/zone scoring only on specific return-code branches. Rust redirects up front with `is_any_layer_walkable` / `nearest_walkable_any_layer` for non-water movers.

Affected surfaces: `src/sim/movement/movement_path.rs`, `src/sim/movement/movement_commands.rs`.

### D3 - DRIFT - Miner far-return staging approximates FNPC

Native CMIN far-return uses FNPC with 1x1 passability, SpeedType `2`, zone `-1`, MovementZone `0`, allow-bridge `1`, no final occupancy, radius cap normally `32`, direct/indirect partition, and frame modulo. Rust uses `PathGrid::is_walkable`, no `CheckPassability`, `EXIT_SEARCH_MAX_RADIUS`, no direct/indirect split, and `sim.tick` as the modulo source.

Affected surfaces: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`.

### D4 - DRIFT - Scatter adjacent candidates are too coarse

Native Unit directional scatter accepts only `Can_Enter_Cell == 0` and then applies height-snap and bridge structural rejection for ideal candidates. Rust scatter helper checks static `PathGrid::is_walkable` and simplified `OccupancyGrid` availability.

Affected surfaces: `src/sim/movement/scatter.rs`, `src/sim/movement/bump_crush.rs`.

### D5 - UNCHECKED - Exact `FootClass::Find_Path -> FNPC` argument row

Fresh decompile proves the branch shape and zone-cost acceptance, but not every pushed argument. The caller matrix already lists `FootClass::Find_Path` as unresolved. Do not write a fixture that depends on the exact FNPC argument row until an assembly push walk verifies it.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| FNPC candidate acceptance runs `CellRect::CheckPassability` with caller SpeedType/MovementZone/rect/overlay/bridge flags, then optional `CheckOccupancy(rect,-1)` | Replace or wrap helper `PathGrid::is_walkable` use with a typed FNPC config and exact validator pair | `src/sim/miner/miner_dock_sequence.rs`, future shared `sim/pathfinding` helper | Same origin has a water cell marked `PathGrid` ground-walkable and a land cell accepted by SpeedType; ground unit chooses land, not water | `find_nearby_passable_uses_checkpassability_not_pathgrid_water` | High: directly matches pier/water symptom |
| Unit directional scatter checks `Can_Enter_Cell == 0` per adjacent candidate | Scatter helpers must call an equivalent cell-entry evaluator, not static walkability | `src/sim/movement/scatter.rs`, `src/sim/movement/bump_crush.rs` | Adjacent water/pier cell is static-walkable but `Can_Enter_Cell` returns 7 for a tank; scatter skips it | `unit_scatter_skips_candidate_can_enter_cell_rejects` | High: affects all ground units |
| `Find_Path` goal correction is return-code driven and zone-scored, not unconditional command-time nearest-walkable | Move generic `resolve_requested_move_goal(max_radius=10)` out of parity-critical path or gate it behind decoded native fallback conditions | `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_path.rs` | Blocked destination with nearby cell in unreachable zone is not redirected merely because it is geometrically nearest | `find_path_fallback_rejects_zone_unreachable_candidate` | High: broad movement behavior |
| CMIN far-return FNPC uses `QueueingCell`, speed type `2`, 1x1 rect, no final occupancy, allow-bridge, radius cap 32, direct candidate modulo | Miner staging helper must implement the real FNPC config, not `EXIT_SEARCH_MAX_RADIUS` + pathgrid | `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs` | Block rings 0..16 and leave a valid direct candidate on ring 17; native can find it, Rust should too | `chrono_far_return_find_nearby_radius_cap_32` | Medium-high |
| FNPC target null chooses by frame counter modulo from direct candidates if any | Preserve direct/indirect partition and modulo selection | shared FNPC helper, miner staging | Ring has one indirect and two direct candidates; null target picks from direct candidates only by frame modulo | `find_nearby_passable_prefers_direct_candidates_for_null_target` | Medium |

## Negative Facts / Do Not Do

- Do not treat `PathGrid::is_walkable` as native ground-unit `Can_Enter_Cell`; `PathGrid` currently marks water ground-walkable in some terrain builds, while native ground units reject water through speed/land and cell-entry logic.
- Do not model FNPC search radius as a caller-provided `max_radius`; native derives it from receiver fields and clamps at `32`.
- Do not merge `CheckPassability` and `CheckOccupancy`; FNPC always calls passability, and occupancy is a separate optional `CheckOccupancy(rect,-1)` path.
- Do not make `Cell+0xDC` reservations part of FNPC final occupancy; `-1` skips that mask.
- Do not globally reject structural bridge cells in FNPC; allow/reject is caller-specific and nonzero allows bridge cells past the separate FNPC bridge filter.
- Do not implement Rust's generic `nearest_walkable_any_layer(max_radius=10)` as if it were proven native `Find_Path` fallback behavior.

## Remaining Uncertainty

- Exact stack-argument row for the `FootClass::Find_Path @ 0x004D3920` call to `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20` still needs an assembly push walk.
- Exact Rust ownership for the future native-shaped validator API is an implementation design question, not solved in this research slot.
- `g_CurrentFrameCounter` vs Rust `sim.tick` equivalence was not proven in this slot.
- Full infantry `InfantryClass::Scatter @ 0x0051D0D0` was not freshly decompiled here; caller matrix says it uses the same FNPC parameter class, but a separate infantry subcell slot can verify infantry-specific deltas if needed.

## Shared Claims

- Active YR nearby-passable helper contracts are mover/caller-specific; a coarse `PathGrid` boolean is not a valid replacement.
- The likely pier/water drift mechanism is not a Chrono Miner-only state issue. Any Rust helper that accepts `PathGrid::is_walkable` as final ground passability can send ground units to cells native `Can_Enter_Cell` / `CheckPassability` would reject.
- `PathGrid` can remain a useful cached structural grid, but helper/caller contracts need a native-shaped legality layer above it.
- Goal redirection, scatter, and miner staging should be audited together because they currently share the same coarse passability shortcut.

## Stale Doc Notes

- `docs/research/pathfinding/fn-find_path.md` describes vtable `+0x1AC` as "GetLocomotorType" and labels return values `6=water` / `7=building occupied`. Fresh `UnitClass::Can_Enter_Cell @ 0x0073F0A0` evidence and sibling bridge reports identify `+0x1AC` as the cell-entry virtual returning 0..7 entry codes. Suggested replacement: "`Find_Path` probes the destination through vtable `+0x1AC` (`Can_Enter_Cell` for concrete foot classes). Return code `6` is a soft stationary allied/non-building block and return code `7` is hard impassable; `Find_Path` has special fallback branches for these destination-probe results."
- `docs/research/FOOTCLASS_FIND_PATH_BLOCKED_DESTINATION_FALLBACK_GHIDRA_REPORT.md` correctly treats the exact `Find_Path -> FNPC` stack row as unresolved. This report does not close that row; it only confirms the branch shape with live Ghidra.

## Sources

- Fresh Ghidra read-only decompile: `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`.
- Fresh Ghidra read-only decompile: `CellRect__CheckPassability @ 0x0056E7C0`.
- Fresh Ghidra read-only decompile: `CellRect__CheckOccupancy @ 0x00586780`.
- Fresh Ghidra read-only decompile: `UnitClass::Scatter @ 0x00743A50`.
- Fresh Ghidra read-only decompile: `FootClass::Find_Path @ 0x004D3920`.
- Fresh Ghidra read-only decompile: `UnitClass::Mission_Harvest @ 0x0073E5E0`.
- Fresh Ghidra read-only decompile: `UnitClass::Can_Enter_Cell @ 0x0073F0A0`.
- Existing docs read: `FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`, `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`, `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`, `SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md`, `FOOTCLASS_FIND_PATH_BLOCKED_DESTINATION_FALLBACK_GHIDRA_REPORT.md`, `miner/FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md`, `bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/movement/movement_path.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/scatter.rs`, `src/sim/movement/bump_crush.rs`, `src/sim/pathfinding/core.rs`.
