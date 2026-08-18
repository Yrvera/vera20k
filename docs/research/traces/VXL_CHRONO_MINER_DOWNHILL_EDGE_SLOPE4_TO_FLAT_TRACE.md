# VXL Chrono Miner Downhill Edge Slope4 To Flat Trace

Date: 2026-05-23
Scope: Chrono Miner descending from a south-facing edge ramp cell (`slope_type=4`) onto flat terrain (`slope_type=0`), at the occupied-cell change and the three visual transition frames around it.
Mode: `/trace-action` slot 1 of trace swarm.

## Verdict

PARTIAL.

The slope byte ingestion, slope direction table, and Rust transition/cache helpers are directionally aligned with the verified `gamemd.exe` DriveLocomotion path. The concrete Chrono Miner scenario is not proven correct because the current Rust entity spawn paths leave voxel units with `rocking: None`, so the implemented `RockingState.prev_slope/curr_slope/transition_ticks_remaining` path is not active for normal spawned CMIN entities. With `rocking: None`, render falls back to direct terrain-slope sampling and can snap from slope 4 to flat instead of showing gamemd's three transition frames.

## Pipeline

`TMP tile +0x2A ramp byte`
-> `ResolvedTerrainCell.slope_type`
-> `entity.position.rx/ry` occupied-cell lookup after movement
-> `RockingState.prev_slope/curr_slope/transition_ticks_remaining`
-> render slope state
-> transient VXL slope-blend cache key
-> `camera * slope_blend * body_facing * section`
-> sprite position/depth from `Position.screen_x/screen_y/z`.

## Stage Table

| Stage | gamemd output for slope 4 -> 0 | Rust output for the concrete current code | Verdict |
|---|---|---|---|
| CMIN locomotor binding | Retail CMIN uses `Locomotor={4A582747-...}` and the docs identify this as `TeleportLocomotionClass (piggybacks Drive)` for normal short-range driving. | Rules parse the teleport CLSID. Player-command code excludes harvesters from instant teleport, but no source path found that turns CMIN's active locomotor kind into Drive for ground drive-track behavior. `movement_step.rs` only uses drive tracks when `LocomotorKind::Drive`. | FAIL |
| TMP slope byte | `TMP_ReadSlopeType @ 0x005471B0` reads TMP cell `+0x2A`; slope 4 cell yields byte `4`, flat cell yields byte `0`. | `tmp_decode.rs:60` reads `data[offset + 42]`, and `resolved_terrain.rs:1261` copies it into `metadata.slope_type`. | PASS |
| Occupied cell used for slope sampling | `DriveLocomotionClass::Process @ 0x004B0500` calls owner `GetOccupiedCell`, reads `CellClass+0x11C`, active in standard YR. At the boundary, the sampled slope changes from `4` to `0`. | `rocking_system.rs:203-204` samples `terrain.cell(entity.position.rx, entity.position.ry)` after movement phase `world/mod.rs:1224`. Exact CMIN occupied-cell timing is not numerically proven because CMIN lacks Drive piggyback/drive-track parity. | UNCHECKED |
| Terrain height/Z visual placement | `CellClass.Level +0x11B` is distinct from slope byte `+0x11C`; visual placement also depends on object coordinates and terrain height. | `ResolvedTerrainGrid::build_height_map` uses `ResolvedTerrainCell.level`, and unit render consumes `Position.screen_x/screen_y/z`. No concrete map coordinate, level pair, or frame capture was computed for this trace. | UNCHECKED |
| `prev_slope` on boundary frame | `Process` writes old current slope into previous-slope cache. For this scenario: `prev_slope = 4`. | Helper would write `prev_slope=4` if `RockingState` existed, but `GameEntity::new` defaults `rocking: None`, and no normal spawn path found that initializes it for CMIN. | NOT-IMPLEMENTED |
| `curr_slope` on boundary frame | `Process` writes sampled flat slope into current-slope cache. For this scenario: `curr_slope = 0`. | Helper would write `curr_slope=0` if active; normal CMIN has no `RockingState`. | NOT-IMPLEMENTED |
| Transition duration/timer | `CDTimerClass::Start(3)` and transition total `3`; draw fractions are `(3-3)/3`, `(3-2)/3`, `(3-1)/3`, then stable flat. | `SLOPE_TRANSITION_TICKS` path sets duration `3`, but it is not active for normally spawned CMIN because `rocking` is absent. | NOT-IMPLEMENTED |
| Render phase | `Draw_Matrix` calls `VXL_InterpolatedFacing` while fraction `< 1.0`: phases `0/3`, `1/3`, `2/3`. | `slope_transition_phase_num` maps remaining `3,2,1` to phase numerators `0,1,2`, but `unit_render_slope_state` only uses this when `entity.rocking` exists. Concrete CMIN output is stable direct terrain slope, not transition. | FAIL |
| Transient cache key | Expected visual transition identity includes CMIN sprite, facing/layer/frame, `from_slope=4`, `to_slope=0`, and phase `0/1/2`. | `TransitionUnitSpriteKey` includes those fields and cache uses `phase_den=3`, but no key is emitted when `rocking=None`. | FAIL |
| Final slope matrix direction/order | Slope type `4` is South, compass `0°`; matrix builder is `Rz(compass) * Rx(edge_tilt) * Rz(-compass)`. Draw matrix composes slope before body facing. | `compute_slope_rotation(4)` uses compass `0.0`, and `prepare_limb_data` composes `camera_view * slope_mat * body_facing * section_transform`. Full float matrix equality was not computed against gamemd. | UNCHECKED |
| Can the visual appear to rise while descending? | Expected: at occupied-cell change, visual orientation blends from south edge ramp to flat over three frames; Z/position should make descent look continuous. | Yes, plausibly: because normal CMIN currently bypasses the transition cache and snaps directly to the current cell's terrain slope. Z continuity was not frame-captured, so the exact "rising" pixel cause remains unchecked. | FAIL |

## Findings

### FAIL 1: CMIN normal-drive path is not DriveLocomotion-equivalent in Rust

Player-visible problem: Chrono Miner downhill motion can differ from gamemd before slope tilt is considered, because the stock CMIN locomotor is TeleportLocomotionClass that piggybacks DriveLocomotionClass for normal short-range driving, while Rust excludes harvesters from instant teleport but leaves the locomotor kind as `Teleport`.

Evidence:
- Retail CMIN: `rulesmd.ini:7351`, `rulesmd.ini:7398`, and `units/allied/CMIN.md:305`.
- Normal short-range drive piggyback: `TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md:574`.
- Rust excludes harvester teleport but does not switch to Drive: `src/sim/world/world_commands.rs:154-169`.
- Rust drive tracks only activate for `LocomotorKind::Drive`: `src/sim/movement/movement_step.rs:88-92`.

Impact on this trace: occupied-cell timing, facing, sub-cell motion, and visual Z timing may already be different for CMIN before the ramp-tilt render path runs.

### NOT-IMPLEMENTED 1: Normal CMIN entities do not get RockingState, so the slope transition cache is unreachable

Player-visible problem: the Chrono Miner can snap from ramp tilt to flat on the boundary frame instead of visually blending through gamemd's three transition frames.

Evidence:
- `GameEntity::new` defaults `rocking: None`: `src/sim/game_entity.rs:409`.
- No normal spawn assignment to `ge.rocking` was found in `src/sim/world/world_spawn.rs`.
- The transition writer exists at `src/sim/rocking/rocking_system.rs:165-169`, and the world tick calls it only for entities whose `rocking` is `Some`: `src/sim/rocking/rocking_system.rs:198-223`.
- The renderer also requires `entity.rocking` to enter transition mode: `src/app_instances/units.rs:98-123`.

Expected gamemd sequence for slope 4 -> 0:

| Rendered frame after occupied-cell change | gamemd previous | gamemd current | remaining | fraction |
|---|---:|---:|---:|---:|
| 0 | 4 | 0 | 3 | 0/3 |
| 1 | 4 | 0 | 2 | 1/3 |
| 2 | 4 | 0 | 1 | 2/3 |
| 3 | 0 | 0 | 0 | stable flat |

Current Rust concrete CMIN output: no `prev_slope`, no `curr_slope`, no remaining timer, no transient key; direct stable terrain slope is selected.

### FAIL 2: Transient cache key is implemented but not emitted for this scenario

Player-visible problem: the new VXL slope-blend cache cannot affect the Chrono Miner downhill case unless the render handoff sees a transition state.

Evidence:
- Cache key has the needed fields: `src/render/unit_slope_transition_cache.rs:23`.
- Render phase denominator is `3`: `src/render/unit_slope_transition_cache.rs:144`.
- `transition_key_for_unit` emits keys only for `UnitRenderSlopeState::Transition`: `src/app_instances/units.rs:429-436`.
- `UnitRenderSlopeState::Transition` requires `entity.rocking`: `src/app_instances/units.rs:98-123`.

## Adjacent Findings

- Terrain height/Z placement remains a possible contributor to the "looks like moving upward while descending" symptom. Rust uses `Position.screen_x/screen_y/z`; the trace did not compute a concrete map coordinate with gamemd and Rust pixel Y values, so this is not marked FAIL here.
- Slope matrix direction/order is structurally aligned for slope 4, but exact float equality with gamemd's table/LUT math was not computed. The relevant Rust code is `src/render/vxl_raster.rs:282-327` and `src/render/vxl_raster.rs:395`.
- CMIN's TeleportLocomotion piggyback behavior may deserve a separate trace. This slot only used it as a dependency for the concrete downhill ramp visual.

## Sources

- `docs/research/VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`
- `docs/research/VOXEL_SLOPE_TILT_SYSTEM.md`
- `docs/research/TMP_PER_TILE_HEIGHT_BYTE_GHIDRA_REPORT.md`
- `docs/research/TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md`
- `docs/research/units/allied/CMIN.md`
- Ghidra read-only decompilation this run: `DriveLocomotionClass__Process @ 004B0500`, `DriveLocomotionClass__Draw_Matrix @ 004AFF60`, `VXL_InterpolatedFacing @ 00755A40`, `Matrix3x4_BuildFromRotateXAndFacing @ 005AE6F0`, `VXL_MasterLighting_Init @ 00754CB0`.

## Tally

PASS: 1
FAIL: 4
UNCHECKED: 3
NOT-IMPLEMENTED: 3

## Status

PARTIAL: exact pixel/Z movement was not frame-captured, and full float matrix equality was not computed. The main implementation blocker for this concrete scenario is still clear: normal CMIN entities do not activate `RockingState`, so the transition render path is not reached.
