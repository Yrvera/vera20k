# VXL Rhino Control Downhill Edge Slope4 To Flat Trace

Date: 2026-05-23

Scenario: Rhino Tank / Grizzly-style non-harvester ground vehicle descends from a south-facing edge ramp cell (`slope_type=4`) onto flat terrain (`slope_type=0`). Scope is one mechanic only: VXL ramp tilt while descending, used as the control case against the Chrono Miner report.

Status: PARTIAL. This trace used source/static evidence and read-only Ghidra decompilation. It did not run a live gamemd/Rust frame capture, so cache texture pixels and final 12-float output matrices are not marked PASS.

## Pipeline

Player move order -> DriveLocomotion movement tick -> occupied-cell slope tracker -> `prev_slope/curr_slope` + 3-frame timer -> render slope state -> transient VXL slope-blend cache -> VXL matrix composition -> screen sprite at `position.screen_x/screen_y/z`.

## Stage Verdicts

### Stage 1 - Control Unit Uses Standard DriveLocomotion

Rust: Voxel units are rendered by the generic `build_unit_instances` path in `src/app_instances/units.rs:158`; miner-specific branches only adjust dock depth and harvest overlays later in the same function.

gamemd: `rulesmd.ini` keeps `[HTNK]` on `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` at `ini/rulesmd.ini:7716`, the standard DriveLocomotion class. `artmd.ini:827-828` marks `[HTNK]` as `Voxel=yes`.

Verdict: PASS. The control tank uses the same ground DriveLocomotion slope tracker, not Chrono Miner teleport/harvester-specific locomotion.

### Stage 2 - TMP Slope Byte Source

Rust: TMP decode reads `ramp_type` from `data[offset + 42]` in `src/assets/tmp_decode.rs:60`, and terrain merge writes `metadata.slope_type = tile.ramp_type` in `src/map/resolved_terrain.rs:1261`.

gamemd: `TMP_ReadSlopeType @ 0x005471B0` reads `*(char *)(tmp_cell + 0x2a)` and returns 0 when the TMP cell pointer is null. This is active YR terrain decode, documented in `TMP_PER_TILE_HEIGHT_BYTE_GHIDRA_REPORT.md`.

Concrete scenario value: source ramp cell `slope_type=4`; destination flat cell `slope_type=0`.

Verdict: PASS. The source byte offset and untransformed slope byte flow match for values 0 and 4.

### Stage 3 - DriveLocomotion Slope Tracker Values

Rust: `update_slope_transition` writes `prev_slope = curr_slope`, `curr_slope = cell_slope`, `transition_ticks_remaining = 3` in `src/sim/rocking/rocking_system.rs:166-169`.

gamemd: `DriveLocomotionClass__Process @ 0x004B0500` reads `cell+0x11C`, compares it with locomotor `+0x18`, then writes `+0x1C = old +0x18`, `+0x18 = new cell slope`, starts `CDTimerClass__Start(3)`, and writes timer duration `+0x2C = 3`.

Concrete values when the change is observed: previous/current/duration = `4 -> 0 / 3`.

Verdict: PASS. The stored transition endpoints and duration match when the slope change is detected.

### Stage 4 - Occupied Cell Timing

Rust: world tick order runs ground movement first in `src/sim/world/mod.rs:1137-1158`, then slope tracking after all movement in `src/sim/world/mod.rs:1193-1203`. The tracker reads `terrain.cell(entity.position.rx, entity.position.ry)` in `src/sim/rocking/rocking_system.rs:202-205`, so it sees the post-movement occupied cell in the same Rust tick.

gamemd: `DriveLocomotionClass__Process @ 0x004B0500` reads the occupied cell and slope byte at the top of the function, before the later `DriveLocomotionClass__Process_Movement` call. Therefore, if `Process_Movement` crosses from slope 4 to flat later in that call, the slope tracker still holds the previous cell's slope until the next `Process` tick.

Concrete boundary tick:
- gamemd at crossing tick: slope tracker still observes `curr_slope=4`; no `4 -> 0` transition until next Process tick.
- Rust at crossing tick: movement has already updated `rx/ry`, so the slope tracker starts `4 -> 0` immediately.

Verdict: FAIL. Rust starts the visible slope blend one tick earlier than gamemd at the cell boundary.

Player-visible effect: on a downhill edge ramp, the body can begin flattening/tilting relative to the destination cell while gamemd would still draw the previous ramp tilt for that render frame. This can look like the unit is lifting or sliding against the ramp.

### Stage 5 - Height / Z Placement

Rust: ordinary ground cell transition updates `position.rx` and `position.ry` in `apply_cell_transition_remainder`, but does not assign `position.z` to the destination cell's ground level (`src/sim/movement/movement_step.rs:41-67`). The only visible `position.z` writes in the movement transition helpers are bridge/tube state updates, e.g. `src/sim/movement/movement_bridge.rs:120-128`. The main tick then refreshes screen coordinates using the current `position.z` in `src/sim/movement/movement_tick.rs:945`.

gamemd: terrain height is active in standard YR. `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md` shows A* start/destination heights read from `CellClass.Level`; `TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md` states ObjectClass `Coords.Z` is maintained as `cell_level * LevelHeight (104) + bridge offset` for ground units. `COORDINATE_SYSTEM_GAMEMD.md` verifies `LevelHeight=104`.

Concrete downhill consequence: for a real descending slope edge where destination ground level is one height lower, gamemd's object Z changes to the destination cell height. Rust currently keeps the old non-bridge `position.z` unless another path writes it.

Verdict: FAIL. Normal ground elevation transitions are not updating Rust unit Z to the destination ground level.

Player-visible effect: after descending, the tank can render one terrain level too high (`15px` in Rust screen-space), which directly matches the "moving upward while driving downhill" symptom.

### Stage 6 - Render Phase Fractions

Rust: `slope_transition_phase_num` maps remaining ticks `3,2,1` to phase numbers `0,1,2` in `src/app_instances/units.rs:91-95`. `VxlSlopeTransitionCache` renders with `phase_den=3` in `src/render/unit_slope_transition_cache.rs:140-144`; the rasterizer computes `t = phase_num / phase_den` in `src/render/vxl_raster.rs:312-314`.

gamemd: `DriveLocomotionClass__Draw_Matrix @ 0x004AFF60` computes `fraction = (duration - remaining) / duration`; with duration 3, the visible transition fractions are `0/3`, `1/3`, `2/3`, then stable at `3/3`.

Concrete values after transition start: Rust fractions `0/3, 1/3, 2/3`; gamemd fractions `0/3, 1/3, 2/3`.

Verdict: PASS for phase numbers once the transition has started. This does not cancel the one-tick early start in Stage 4.

### Stage 7 - Render Cache Key

Rust: transient key includes `type_id`, `facing`, `layer`, `frame`, `from_slope`, `to_slope`, and `phase_num` in `src/render/unit_slope_transition_cache.rs:23-30`; app code fills it from the render slope state in `src/app_instances/units.rs:429-444`.

gamemd: `DriveLocomotionClass__Draw_Matrix @ 0x004AFF60` packs the stable cache-facing value as `direction * 0x40 + current_slope` in the simple path. I did not compute the exact cache identity used for interpolated transient matrices or compare it to Rust's transient cache key.

Verdict: UNCHECKED. Rust has enough identity to avoid obvious reuse collisions, but this stage lacks literal gamemd/Rust cache-key equality.

### Stage 8 - Final Matrix Order / Direction

Rust: slope rotation for type 4 is compass `0.0` south with edge tilt in `src/render/vxl_raster.rs:282-307`; blended slope matrices are generated by quaternion slerp in `src/render/vxl_raster.rs:312-326`; final limb matrix is `camera_view * slope_mat * body_facing * section_transform` in `src/render/vxl_raster.rs:347-395`.

gamemd: `VXL_MasterLighting_Init @ 0x00754CB0` populates slope 4 at `DAT_00b45248` with `Matrix3x4_BuildFromRotateXAndFacing(0, EDGE)`. `Matrix3x4_BuildFromRotateXAndFacing @ 0x005AE6F0` applies `RotateZ(compass)`, `RotateX(tilt)`, `RotateZ(-compass)`. `VXL_InterpolatedFacing @ 0x00755A40` uses quaternion SLERP between slope entries.

Verdict: UNCHECKED under strict trace-action rules. Direction constants and structural order match the verified binary model, but I did not compute and compare the final 12 floats for Rhino slope `4 -> 0` against gamemd.

### Stage 9 - Chrono Miner-Specific Code Implication

Rust: the slope state and transition cache are generic voxel unit rendering. Miner branches in `src/app_instances/units.rs` cover dock depth and harvest overlay; they do not choose the slope endpoints. The control unit path reaches the same slope render state without `MinerState`.

gamemd: `[HTNK]` uses standard DriveLocomotion, while Chrono Miner uses TeleportLocomotion with drive piggybacking in separate miner docs. This control trace's active binary references are DriveLocomotion-only.

Verdict: PASS. The two identified mismatches are generic ground movement/render timing and ground Z placement, not Chrono Miner-only code.

## Failures

1. Occupied cell timing is one tick early in Rust. Rust samples slope after movement; gamemd samples before movement in `DriveLocomotionClass::Process`. A downhill crossing can start visual flattening one render frame too soon.

2. Normal ground elevation is not written to `position.z` on non-bridge cell transitions. For a real downhill level change, Rust can keep the previous elevation and render the tank too high after descending.

## Not Implemented

None in this slot. The failures are implemented paths with mismatching timing/state updates, not missing systems.

## Adjacent Findings

- The exact mid-transition pixels are still unproven because Rust uses `glam::Quat::slerp` while gamemd uses its own quaternion table and `Quaternion_Slerp`.
- Slope types 17-20 are outside this concrete scenario.
- Bridge/tube Z updates are adjacent and already have separate code paths; not traced here.

## Verdict Tally

PASS: 5 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

