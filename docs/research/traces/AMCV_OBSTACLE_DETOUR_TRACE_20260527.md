# AMCV Obstacle Detour Pathing Trace - 2026-05-27

## Scenario

Mechanic: AMCV obstacle detour pathing.

Concrete trace: Allied MCV (`AMCV`) receives a move order from cell `(40,40)` to `(48,40)` on flat ground with a small static direct-path blocker in the middle. The slot prompt did not give an exact blocker footprint; this trace fixes it as a single static wall/blocker cell at `(44,40)` so the run is numerically concrete.

Scope is limited to initial move command path setup, A*/zone precheck behavior, smoothing, selected waypoint cells, movement execution, and final arrival for this one scenario.

## Sources Checked

- `ini/rulesmd.ini:6969-7009`: active YR `[AMCV]` has `Speed=4`, `ROT=5`, `Crusher=yes`, `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}`, `MovementZone=Normal`.
- `docs/research/PATHFINDING_ASTAR_GHIDRA_REPORT.md`: verified `FootClass::Find_Path`, `AStar_pathfind_search @ 0x0042C900`, `AStar_main_loop @ 0x00429A90`, 9-direction expansion, zone precheck, retry loop, path queue format.
- `docs/research/PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md`: verified smoothing passes and DriveLocomotion speed handling.
- `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`: verified active YR destination/NavCom/DriveLocomotion arrival lifecycle.
- Ghidra MCP read-only spot-checks: `0042c900` and `00429a90` decompiled successfully and match the research reports. The pathfinding slice is active in standard YR through `FootClass`/`DriveLocomotion` callers; no TS-only gate was found for the cited A* or smoothing calls.

## Rust Code Surfaces

- Move info and speed dispatch: `src/sim/world/world_commands.rs:56-77`, `src/sim/world/world_commands.rs:248-270`.
- Move target setup: `src/sim/movement/movement_commands.rs:254-490`.
- Path wrapper and smoothing calls: `src/sim/movement/movement_path.rs:182-328`.
- A* implementation: `src/sim/pathfinding/core.rs:819-1360`, tie-break/neighbor order at `src/sim/pathfinding/core.rs:364-395`, heap ordering at `src/sim/pathfinding/core.rs:1975-2037`.
- Smoothing implementation: `src/sim/pathfinding/path_smooth.rs:86-161`, `src/sim/pathfinding/path_smooth.rs:258-294`, helpers at `src/sim/pathfinding/path_smooth.rs:349-494`.
- Movement tick and arrival removal: `src/sim/movement/movement_tick.rs:387-431`, `src/sim/movement/movement_tick.rs:578-691`, `src/sim/movement/movement_tick.rs:981-999`.

## Pipeline

`Move command` -> `resolve AMCV move info` -> `issue_move_command_with_layered` -> `resolve_requested_move_goal` -> `find_move_path_with_marker` -> layered A* (`astar_search`) -> smoothing/optimization -> `MovementTarget` -> per-tick rotation/drive-track/lepton movement -> cell-crossing/occupancy update -> `movement_target` cleared at arrival.

## Stage Results

### S1 - AMCV data

gamemd/YR data: `[AMCV] Speed=4`, `ROT=5`, `Crusher=yes`, Drive locomotor, `MovementZone=Normal`.

Rust data: `resolve_move_info` reads object rules, but applies `speed_mult = 3` when `deploys_into.is_some()`, so AMCV is dispatched at `Speed=12` equivalent before locomotor multiplier.

Verdict: **FAIL**. The AMCV cannot match movement execution timing in this scenario because the Rust mover is intentionally sped up 3x before path execution.

### S2 - Goal resolution

Scenario goal `(48,40)` is not the blocked cell. Rust `resolve_requested_move_goal` therefore keeps `(48,40)`.

gamemd: `FootClass::Find_Path` checks destination `Can_Enter_Cell` and has blocked-destination fallback for occupied/blocked destination cases. This scenario does not block the destination, so no fallback should be needed, but I did not run gamemd for this exact fixture.

Verdict: **UNCHECKED**.

### S3 - Zone precheck / hierarchy

gamemd: `AStar_pathfind_search @ 0x0042C900` always resolves source/destination zone IDs for this active YR path, calls `Zone_precheck` when hierarchical search remains enabled, and can reject or retry based on hierarchy state.

Rust: this player move path enters `issue_move_command_with_layered`, which calls `find_move_path_with_marker` with `zone_grid: None`; `find_layered_path_zoned_marker` therefore falls directly into `find_layered_path_marker` and runs A* without a zone precheck for this command.

Verdict: **FAIL**. Even if the concrete path is reachable, the mechanism skips an active gamemd gate and cannot prove identical retry/failure/order behavior.

### S4 - A* path cells

Rust output computed from current A* neighbor order, uniform `STEP_COST=1000`, direction tie-breaks, Euclidean heuristic, and a flat grid with only `(44,40)` blocked:

`[(40,40), (41,40), (42,40), (43,40), (44,39), (45,40), (46,40), (47,40), (48,40)]`

Rust selected the north detour through `(44,39)`. Final Rust path cost for the raw path is `8023` under the current integer cost/tiebreak model.

gamemd: A* expands 8 compass directions plus direction 8, calls `Can_Enter_Cell` on each candidate, applies edge cost/tiebreak values, and reconstructs a direction array before smoothing. I did not run gamemd with this exact `(40,40)->(48,40)` plus `(44,40)` fixture, so the exact gamemd waypoint cells are not known.

Verdict: **UNCHECKED**.

### S5 - Smoothing / optimization

Rust smoothing leaves the computed path unchanged:

`[(40,40), (41,40), (42,40), (43,40), (44,39), (45,40), (46,40), (47,40), (48,40)]`

gamemd: on successful A*, `AStar_main_loop @ 0x00429A90` calls `AStar_reconstruct_path`, `Path_smooth_corners`, then `Path_optimize_straight_segments` unconditionally. The research report verifies slope/cliff/Can_Enter_Cell validation and two-ordering straight reroute behavior. Exact gamemd smoothed output for this fixture was not computed.

Verdict: **UNCHECKED**.

### S6 - Movement execution and arrival

Rust movement attaches `MovementTarget` with `next_index=1`, path layers aligned to path cells, `final_goal=(48,40)`, then movement tick rotates vehicles before movement, advances drive-track/lepton state, crosses cells, updates occupancy, and clears the target once `next_index >= path.len()` and current cell equals `final_goal`.

gamemd DriveLocomotion arrival is active and clears NavCom through `Set_Destination(NULL,1)` / `Stop_Moving` when the unit reaches the destination cell. Exact tick count, track cadence, facing cadence, and arrival frame for this fixture were not computed.

Verdict: **FAIL** for timing because Rust dispatches AMCV at 3x stock speed. **UNCHECKED** for exact facing/track/arrival-cell clearing sequence.

## Findings

1. **FAIL - AMCV speed is multiplied by 3 before movement.** Rust `src/sim/world/world_commands.rs:72-75` applies a development speed multiplier to any deployable unit, so stock `[AMCV] Speed=4` becomes an effective `12` before command dispatch. Player-visible result: the AMCV traverses the detour and arrives too soon even if path cells match.

2. **FAIL - Initial player move pathing skips gamemd's active zone precheck when `zone_grid` is absent.** Rust `src/sim/movement/movement_commands.rs:394-417` builds `PathfindingContext { zone_grid: None }`; `src/sim/pathfinding/zone_search.rs:547-578` only runs the reachability precheck when a zone grid exists. gamemd `AStar_pathfind_search @ 0x0042C900` resolves zones and runs/consults `Zone_precheck` for active standard YR pathfinding.

3. **FAIL - Initial player move pathing does not reproduce gamemd's default five-attempt hierarchy retry path for this command surface.** gamemd `AStar_pathfind_search @ 0x0042C900` sets the default attempt cap to five when `max_search_depth == -1` and updates hierarchical edges between failures. This exact Rust command path falls through to a single A* call when no zone grid is supplied.

4. **UNCHECKED - Exact gamemd detour cells were not computed.** Rust's computed path detours north through `(44,39)`, but no live gamemd fixture/log was run for the same `(40,40)->(48,40)` and `(44,40)` blocker setup, so this cannot be marked PASS.

5. **UNCHECKED - Exact gamemd smoothing result was not computed.** Rust smoothing leaves the detour unchanged, but gamemd applies verified post-A* smoothing passes with Can_Enter_Cell, cliff, and slope validation. The exact resulting direction array/cell list for this fixture remains unknown.

## Adjacent Findings

- `MoveInfo::mover_is_crusher` is derived from `omni_crusher` or Crusher movement zones, not the `[AMCV] Crusher=yes` key. That affects dynamic unit-block/crush semantics, but this trace used a static wall/blocker and did not trace crush-on-path.
- The prompt did not define whether the blocker is an overlay wall, terrain block, building footprint, or entity. This report fixes the path computation as a static non-walkable `PathGrid` blocker at `(44,40)`. Other blocker representations can change both Rust and gamemd pathing.

## Verdict Tally

PASS: 0
FAIL: 3
UNCHECKED: 4
NOT-IMPLEMENTED: 0

## Status

COMPLETE for the requested read-only slot trace with one fixed blocker assumption. Exact gamemd waypoint/tick equality remains UNCHECKED because no live gamemd fixture output was produced for this coordinate/blocker setup.
