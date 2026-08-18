# Direction 8 Tube-Step Reference Trace

**Scenario:** A path replay or coordinate-step helper starts on a cell whose `TubeClass` exit points to another cell; compare `gamemd.exe` direction `8` behavior against current Rust `ResolvedTerrainGrid::step_coord_by_direction` and related tube/path helpers.

**Scope lock:** One concrete mechanic only: direction-index `8` as a tube-step reference point, plus valid non-8 table stepping used by the same helper. Adjacent pathfinding marker/cost behavior and invalid direction values are recorded only as adjacent findings.

**Report path:** `docs/research/traces/DIRECTION8_TUBE_STEP_REFERENCE_TRACE.md`

## Verdict

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Evidence Inputs

- Existing verified report: `docs/research/CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md`
- Existing verified report: `docs/research/GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`
- Rust helper: `src/map/resolved_terrain.rs:298`
- Rust tube fact shape: `src/map/tube_facts.rs:30`
- Rust low-bridge tube movement consumer: `src/sim/movement/tube_movement.rs:57`

## Active YR Confirmation

The cited reports mark the relevant `gamemd.exe` functions active in standard YR:

- `Foundation_direction_table_init @ 0x0049F2F0`: active before `WinMain`; initializes `g_DirectionOffsets @ 0x0089F688`.
- `MapCoord_Step_By_Direction @ 0x0042D490`: active generic coordinate step helper.
- `Path_walk_directions_to_cell @ 0x00429780`: active path direction replay helper.
- `AStar_main_loop @ 0x00429A90`: active YR pathfinding; treats loop index `8` as the tube case.
- `PathfinderClass__UpdateBridgePassability @ 0x0042ACF0`: active under live A* bridge-marker replay; uses the same direction table and direction-8 tube jump.

No TS-only dormant path was used as evidence for this report.

## Pipeline

1. Trigger: helper receives `(coord, direction)`.
2. Branch: if `direction == 8`, resolve the current cell's tube index.
3. Tube destination: valid tube returns `TubeClass+0x28` / Rust `TubeFact.exit`.
4. Missing tube: no tube index returns packed coord `(0,0)`.
5. Non-8 valid direction: use canonical table `0..7`.
6. Consumer: path replay walks these steps sequentially.

## Stage Trace

### Stage 1 - Direction 8 With Valid Tube

- Rust site: `src/map/resolved_terrain.rs:298`
- Concrete input: `coord=(1,0)`, `direction=8`, current cell has `tube_index=0`, `TubeFact.exit=(2,0)`.
- Rust computation: line 299 takes the `direction == 8` branch; lines 301-302 call `tube_at_cell(1,0)` and return `tube.exit`; output `Some((2,0))`.
- Rust test fixture: `src/map/resolved_terrain.rs:1568`; assertion at line 1585 expects `Some((2,0))`.
- gamemd computation: `MapCoord_Step_By_Direction @ 0x0042D490` and `Path_walk_directions_to_cell @ 0x00429780` read `CellClass+0x116`, index `g_TubeArray`, and return `TubeClass+0x28`; for a tube whose `+0x28` is `(2,0)`, output is packed `(2,0)`.
- Timing/order: same helper call; current cell is read before tube array destination; no rounding, no clamp, signed tube index compare only against `-1`.
- Verdict: PASS. Both computed outputs are `(2,0)`.

### Stage 2 - Direction 8 Without Valid Tube

- Rust site: `src/map/resolved_terrain.rs:298`
- Concrete input: `coord=(0,0)`, `direction=8`, current cell has no `tube_index`.
- Rust computation: line 299 takes the `direction == 8` branch; `tube_at_cell(0,0)` returns `None`; line 302 `map_or((0,0), ...)` returns `Some((0,0))`.
- Rust test fixture: `src/map/resolved_terrain.rs:1590`; assertion at line 1593 expects `Some((0,0))`.
- gamemd computation: same helpers check `CellClass+0x116 == -1`; missing tube writes packed coord `0`, numerically `(0,0)`.
- Timing/order: same helper call; missing-tube branch does not consult canonical offsets; no rounding or bounds clamp.
- Verdict: PASS. Both computed outputs are `(0,0)`.

### Stage 3 - Non-8 Canonical Direction Table

- Rust site: `src/map/resolved_terrain.rs:1153`
- Concrete input: valid directions `0..7`.
- Rust computation: `direction_offset` returns `0:(0,-1)`, `1:(1,-1)`, `2:(1,0)`, `3:(1,1)`, `4:(0,1)`, `5:(-1,1)`, `6:(-1,0)`, `7:(-1,-1)`.
- gamemd computation: `g_DirectionOffsets @ 0x0089F688` is initialized to the same signed pairs by `0x0049F2F0`.
- Timing/order: table lookup is immediate in the helper; signed short offsets are added to current packed X/Y; no rounding.
- Verdict: PASS for valid `0..7` table values. Literal table pairs match.

### Stage 4 - Path Replay `[2, 8]`

- Rust site: `src/map/resolved_terrain.rs:314`
- Concrete input: start `(0,0)`, directions `[2,8]`, cell `(1,0)` has `tube_index=0`, `TubeFact.exit=(2,0)`.
- Rust computation: first step `2` uses `(1,0)` and returns `(1,0)`; second step `8` returns tube exit `(2,0)`; final output `Some((2,0))`.
- Rust test fixture: assertion at `src/map/resolved_terrain.rs:1586`.
- gamemd computation: `Path_walk_directions_to_cell @ 0x00429780` applies non-8 table step for `2`, then direction-8 tube branch to `TubeClass+0x28`; output `(2,0)`.
- Timing/order: sequential replay in buffer order; each step reads the post-previous-step coordinate.
- Verdict: PASS. Both computed final outputs are `(2,0)`.

### Stage 5 - Runtime Low-Bridge Tube Movement Path Steps

- Rust site: `src/sim/movement/tube_movement.rs:129`
- Concrete input: active `LowBridgeTubeMovementState`, next `tube.path_steps[cursor]`.
- Rust computation: line 133 falls back to `8` only if the indexed path step is absent; line 124 exits early when `cursor >= tube.path_len()`, so valid in-bounds ticks use authored `0..7` steps and call `step_coord_by_direction` at line 135.
- gamemd computation for this exact runtime movement timing was not recomputed in this trace; existing reports verify the shared step contract but not every movement-state tick transition here.
- Verdict: UNCHECKED. Shared helper contract is verified; full movement tick equality was not computed for both engines.

## Failures

None in the scoped coordinate-step/helper scenario.

## Not Implemented

None in the scoped coordinate-step/helper scenario.

## Adjacent Findings

- Invalid non-8 directions are outside this scenario: Rust `direction_offset` masks with `direction & 7` at `src/map/resolved_terrain.rs:1154`, while the verified binary generic helper indexes directly for non-8 values. Do not count wrapping `9..=255` as parity without a caller-side sanitizer.
- Rust `[Tubes]` parsing accepts authored path steps only in `0..=7` at `src/map/tubes.rs:68`, which is consistent with existing research that `8` is an internal replay/tube sentinel, not a normal map-authored path step.
- Rust A* has a separate explicit tube edge at `src/sim/pathfinding/core.rs:891`, with cost `STEP_COST * path_len + TUBE_DIR_TIEBREAK`; that is not the same as `0x0042ACF0` bridge-marker replay, which only marks the destination. This report did not trace marker cost behavior.
- `ResolvedTerrainGrid::tube` safely returns `None` for an out-of-range positive `TubeId` at `src/map/resolved_terrain.rs:289`; the binary consumers checked here only compare the signed cell tube index to `-1` before indexing `g_TubeArray`. Valid fixture data avoids this safety-boundary difference.

## Player-Visible Impact

The scoped helper matches for valid path replay: a unit/path helper using direction `8` from a tube cell resolves to the same destination cell as `gamemd.exe`, and a missing tube resolves to origin `(0,0)` rather than an adjacent ninth direction. No player-visible mismatch was found in this concrete coordinate-step scenario.

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

None.

## Status

COMPLETE
