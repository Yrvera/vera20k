# VXL Chrono Miner Opposite Edge Slope 2 vs Slope 4 Trace

Date: 2026-05-23

Scope: Chrono Miner crossing north-facing edge ramp `slope_type=2` versus south-facing edge ramp `slope_type=4`, focused only on whether the VXL ramp tilt preserves opposite compass direction and avoids north/south visual swap.

Status: COMPLETE for static/binary direction chain. Runtime screenshot/pixel capture was not performed.

## Scenario

Concrete unit: `[CMIN]` Chrono Miner.

Retail/YR data:

- `rulesmd.ini [CMIN]` uses `Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}`.
- `docs/research/units/allied/CMIN.md` records that Chrono Miner uses `TeleportLocomotionClass`, but short-distance movement piggybacks Drive locomotion. This keeps Drive VXL slope drawing relevant for ordinary ramp crossing.
- `artmd.ini [CMIN]` has `Voxel=yes` and `Remapable=yes`, so the VXL tilt path is the visible body-render path.

Active YR check: Drive locomotion is active in standard YR vehicles and in Chrono Miner short-distance piggyback movement. The functions checked here are not the dormant Tunnel/TS tilt path.

## Pipeline

TMP tile cell `+0x2A` slope byte -> Rust `TmpTile.ramp_type` -> Rust `ResolvedTerrainCell.slope_type` -> rocking slope tracker `prev_slope/curr_slope/transition_ticks_remaining` -> unit render slope state -> stable atlas key or transient slope-blend key -> `compute_slope_rotation` / blend -> VXL limb matrix -> screen.

gamemd equivalent:

TMP tile cell `+0x2A` -> `CellClass+0x11C` -> `DriveLocomotionClass::Process` cached current/previous slope + 3-frame timer -> `DriveLocomotionClass::Draw_Matrix` -> `VXL_GetFacingMatrix` or `VXL_InterpolatedFacing` -> VXL raster.

## Stage Results

| Stage | Verdict | Evidence | Notes |
|---|---:|---|---|
| TMP slope byte mapping for 2 and 4 | PASS | gamemd `TMP_ReadSlopeType @ 0x005471B0`; Rust `src/assets/tmp_decode.rs:60`, `src/map/resolved_terrain.rs:1261` | gamemd reads tile cell byte `+0x2A`. Rust reads `data[offset + 42]` and stores it as `slope_type`. Values `2` and `4` remain exact. |
| Active slope source and timer | PASS | gamemd `DriveLocomotionClass::Process @ 0x004B0500`; Rust `src/sim/rocking/rocking_system.rs:165`, `:203`, `:223` | gamemd samples current occupied cell `CellClass+0x11C`, writes previous/current slope, and starts duration `3`. Rust samples current terrain cell and starts `SLOPE_TRANSITION_TICKS`. |
| gamemd slope constants for 2 and 4 | PASS | `VXL_MasterLighting_Init @ 0x00754CB0`, `Matrix3x4_BuildFromRotateXAndFacing @ 0x005AE6F0` | Slope 2 uses compass constant `0x40490e56` (~3.141499996), slope 4 uses `0`. Both use edge tilt. Builder order is `Rz(compass) -> Rx(tilt) -> Rz(-compass)`. |
| Rust slope constants for 2 and 4 | FAIL | `src/render/vxl_raster.rs:287`, `src/render/vxl_raster.rs:289` | Rust slope 4 uses `0.0`, matching gamemd. Rust slope 2 uses `std::f32::consts::PI` (`0x40490fdb`, ~3.141592741), not gamemd's `0x40490e56`. Direction is correct, but literal numerical equality fails. |
| Matrix sign/opposite direction | PASS | gamemd constants above; Rust `src/render/vxl_raster.rs:287`, `:289`, tests around `:922` | Slope 2 and slope 4 are opposite compass directions. For the local +Y axis, slope 4 lifts with positive Z while slope 2 lowers with negative Z. No north/south swap was found. |
| Matrix composition order | PASS | gamemd `DriveLocomotionClass::Draw_Matrix @ 0x004AFF60`; Rust `src/render/vxl_raster.rs:391`, `:395` | Rust composes `camera_view * slope_mat * body_facing * section_transform`, matching the active simple draw path shape: slope left of body facing and right of camera/view. |
| Render cache distinguishes slope 2 from 4 | PASS | gamemd `Draw_Matrix @ 0x004AFF60` cache pack uses `direction * 0x40 + current_slope`; Rust `src/render/unit_slope_transition_cache.rs:23`, `:28`, `:29`, `:30`; `src/app_instances/units.rs:433` | Stable and transient Rust keys include the slope IDs; slope 2 and slope 4 cannot alias in the cache. |
| Visible north/south screen result | UNCHECKED | No runtime capture in this slot | Static matrix sign says direction is not swapped. Pixel-level comparison against gamemd frames was not captured, so final visible equality is unchecked. |
| Mid-transition exact pixels | UNCHECKED | gamemd `VXL_InterpolatedFacing @ 0x00755A40`; Rust `src/render/vxl_raster.rs:312` | Both use quaternion interpolation conceptually, but this trace did not compute final gamemd and Rust raster pixels for phases 0/1/2. |

## Findings

### FAIL: Slope 2 compass constant is not byte-identical

Rust uses `std::f32::consts::PI` for slope 2 at `src/render/vxl_raster.rs:287`.

gamemd populates the north edge-ramp slope matrix with the float constant `0x40490e56` (~3.141499996) in `VXL_MasterLighting_Init @ 0x00754CB0`, then builds the matrix with `Matrix3x4_BuildFromRotateXAndFacing @ 0x005AE6F0`.

Player-visible risk: small non-gamemd cross-axis tilt leakage or subpixel body-position drift on north-facing edge ramps. This is not a north/south swap; it is an exact-numerics parity issue.

## Adjacent Findings

- `VXL_INTERPOLATED_FACING_AND_SLOPE_TRANSITION_GHIDRA_REPORT.md` still contains stale text claiming the drive interpolation branch is unreachable. `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md` supersedes that: `DriveLocomotionClass::Process` writes duration `3`, and `Draw_Matrix` consumes the transition in standard YR.
- This slot did not trace Chrono Miner long-distance teleport arrival or bridge Z interactions.

## Verdict Tally

PASS: 6

FAIL: 1

UNCHECKED: 2

NOT-IMPLEMENTED: 0

## Evidence Index

- Ghidra read-only: `TMP_ReadSlopeType @ 0x005471B0`
- Ghidra read-only: `DriveLocomotionClass::Process @ 0x004B0500`
- Ghidra read-only: `DriveLocomotionClass::Draw_Matrix @ 0x004AFF60`
- Ghidra read-only: `VXL_GetFacingMatrix @ 0x007559B0`
- Ghidra read-only: `VXL_InterpolatedFacing @ 0x00755A40`
- Ghidra read-only: `VXL_MasterLighting_Init @ 0x00754CB0`
- Ghidra read-only: `Matrix3x4_BuildFromRotateXAndFacing @ 0x005AE6F0`
- Rust source: `src/assets/tmp_decode.rs`
- Rust source: `src/map/resolved_terrain.rs`
- Rust source: `src/sim/rocking/rocking_system.rs`
- Rust source: `src/app_instances/units.rs`
- Rust source: `src/render/unit_slope_transition_cache.rs`
- Rust source: `src/render/vxl_raster.rs`
