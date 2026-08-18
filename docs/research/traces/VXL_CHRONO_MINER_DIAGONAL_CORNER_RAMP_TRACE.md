# VXL Chrono Miner Diagonal/Corner Ramp Trace

Date: 2026-05-23
Slot: 4
Scenario: Chrono Miner crosses one diagonal/corner ramp using slope_type 5 as the concrete checked case; slope_types 7 and 8 plus alias families 9-12 and 13-16 are checked only for direction drift.
Status: COMPLETE

## Scope Lock

This trace covers one visual mechanic: ground VXL ramp tilt direction for a Chrono Miner on diagonal/corner ramp slope bytes. It does not trace pathfinding, terrain height interpolation, bridge ramps, miner refinery behavior, or live screenshot parity.

## Evidence Sources

- Live read-only Ghidra decompile: `TMP_ReadSlopeType @ 0x005471B0`, `DriveLocomotionClass::Process @ 0x004B0500`, `DriveLocomotionClass::Draw_Matrix @ 0x004AFF60`, `VXL_GetFacingMatrix @ 0x007559B0`, `VXL_InterpolatedFacing @ 0x00755A40`, `VXL_MasterLighting_Init @ 0x00754CB0`, `Matrix3x4_BuildFromRotateXAndFacing @ 0x005AE6F0`.
- Existing verified reports: `VOXEL_SLOPE_TILT_SYSTEM.md`, `VXL_DRAW_MATRIX_ORDER_GHIDRA_REPORT.md`, `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`.
- Rust source: `src/assets/tmp_decode.rs`, `src/map/resolved_terrain.rs`, `src/sim/rocking/rocking_system.rs`, `src/app_instances/units.rs`, `src/render/unit_slope_transition_cache.rs`, `src/render/vxl_raster.rs`.

## Pipeline

TMP tile byte `+0x2A` -> resolved terrain `slope_type` -> Drive/Rocking current-cell slope cache -> 3-frame render transition state -> transient VXL slope-blend cache -> raster slope matrix -> screen sprite.

## Stage Results

| Stage | Result | Evidence |
|---|---:|---|
| Corner ramp slope byte | PASS | gamemd `TMP_ReadSlopeType @ 0x005471B0` returns tile cell byte `+0x2A`; Rust reads `data[offset + 42]` in `src/assets/tmp_decode.rs:60` and copies it to `metadata.slope_type` in `src/map/resolved_terrain.rs:1261`. For concrete slope_type 5, both outputs are numeric 5. |
| Active YR slope cache source | PASS | gamemd `DriveLocomotionClass::Process @ 0x004B0500` reads occupied `CellClass+0x11C`, writes previous/current slope and starts a 3-frame timer. Rust updates rocking after movement in `src/sim/world/mod.rs:1193` and reads `terrain.cell(entity.position.rx, entity.position.ry)` in `src/sim/rocking/rocking_system.rs:205`. |
| gamemd corner compass/tilt family | PASS | `VXL_MasterLighting_Init @ 0x00754CB0` populates slope 5 with compass `0x407b51ec` / 225 deg and corner tilt, slope 7 with `0x3f490e56` / 45 deg and corner tilt, slope 8 with `0x40afec8b` / 315 deg and corner tilt. This path is active because `Draw_Matrix` calls `VXL_GetFacingMatrix` / `VXL_InterpolatedFacing`. |
| Rust corner compass constants | FAIL | Rust uses rounded decimal f32 literals: slope 5 `3.9270` -> `0x407b53f8`, slope 7 `0.7854` -> `0x3f490ff9`, slope 8 `5.4978` -> `0x40afedfa`. These are not literally equal to gamemd `0x407b51ec`, `0x3f490e56`, `0x40afec8b`. See `src/render/vxl_raster.rs:291`, `:293`, `:294`. |
| Alias direction grouping | PASS | gamemd repeats 5-8 directions for 9-12 with corner tilt and repeats the same directions for 13-16 with edge tilt. Rust repeats the same groups in `src/render/vxl_raster.rs:296` through `:304`. Exact compass literal drift from the previous stage still applies. |
| Transition cache key coverage | UNCHECKED | Rust key includes type, facing, layer, frame, from slope, to slope, and phase at `src/render/unit_slope_transition_cache.rs:23`; render lookup uses it from `src/app_instances/units.rs:429`. No live cache lookup was captured against gamemd for this concrete Chrono Miner ramp crossing, so this cannot be marked PASS. |
| Matrix order | PASS | gamemd simple path returns `slope_matrix * facing_rotation`, then render applies camera/view. Rust composes `camera_view * slope_mat * body_facing * section_transform` in `src/render/vxl_raster.rs:395`. Direction is world/cell-oriented rather than rotating with vehicle facing. |
| Hidden diagonal axis/sign dependency | UNCHECKED | No sign swap or quadrant drift was found in the static chain. However no frame capture compared final Chrono Miner pixels on slope 5/7/8 against gamemd, so hidden projection/model-axis issues remain unchecked. |

## Findings

### FAIL - Rounded corner compass constants

Rust's corner compass literals are close but not bit-identical to gamemd. The concrete slope_type 5 case differs by about `0.00012493` radians; slope 7 differs by about `0.00002497`; slope 8 differs by about `0.00017500`. Under the trace-swarm rule requiring literal numerical equality, this is a FAIL.

Player-visible risk is probably small, but not zero: diagonal/corner ramp VXL tilt can be a fraction of a pixel off, and aliases inherit the same tiny direction drift. This is not the large "tilts uphill/downhill" class of bug; that class was the matrix-order/sampling chain, which is directionally correct here.

## Adjacent Findings

- Exact mid-transition matrix parity is still unchecked because Rust uses `glam::Quat::slerp` from matrices, while gamemd uses its quaternion table plus `Quaternion_Slerp`.
- The transient cache likely has the right distinguishing fields, but this trace did not capture a live Chrono Miner crossing slope_type 5 with both engines.
- Slope types 17-20 are outside this concrete scenario; Rust clamps them to identity for VXL render.

## Verdict Tally

PASS: 5 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
