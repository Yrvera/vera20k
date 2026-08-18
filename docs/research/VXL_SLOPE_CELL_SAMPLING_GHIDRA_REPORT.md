# VXL Slope Cell Sampling During DriveLocomotion - Ghidra Report

**Date:** 2026-05-23
**Binary:** `gamemd.exe`
**Target question:** What `slope_type` feeds vehicle `Draw_Matrix` while crossing a ramp: current Techno cell, destination/next cell, locomotor cached slope, or transition field?
**Non-goals:** Full VXL matrix order, exact per-slope signs, Rust implementation.
**Evidence needed to mark COMPLETE:** Active DriveLocomotion tick source, `Draw_Matrix` consumer offsets, transition gate/timer write, and stock-YR interpolation liveness.
**Stop conditions:** Do not mutate Ghidra/code; stop after the scoped slope-source answer and Rust handoff.

## Short Answer

`DriveLocomotionClass::Process` samples the owning Techno/Object's current occupied cell at vtable `+0x1BC`, reads `CellClass+0x11C` as the slope byte, and caches it in the locomotor. `DriveLocomotionClass::Draw_Matrix` does not resample map cells directly; it consumes the locomotor's cached current/previous slope fields and the 3-frame transition timer.

Stock YR does use slope interpolation for drive locomotors. The prior "interpolation branch is unreachable because `+0x2C` is never set nonzero" wording is stale: the active `Process` path writes the transition total to `3` whenever the occupied-cell slope changes.

## Verified Facts

1. **Active slope sample is the current occupied cell, not destination/next.**
   `DriveLocomotionClass::Process @ 0x004B0500` calls the owning object vtable `+0x1BC` at `0x004B0510`, then reads `byte [EAX+0x11C]` at `0x004B051B`. `0x005F6960` is `ObjectClass::GetOccupiedCell`, and its assembly builds a cell lookup from object coordinates `+0x9C/+0xA0/+0xA4` before calling `CellClass__Get_Cell_At @ 0x00565730`. Active in YR: Yes; `Process` is in the DriveLocomotion vtable (`xref from 0x007E7EF0`).

2. **The sampled cell slope is cached into locomotor fields before draw.**
   If the sampled byte differs from cached current slope, `0x004B0523..0x004B0533` compares against `[EDI+0x1C]`, writes old current to `[EDI+0x20]`, and writes the new sampled slope to `[EDI+0x1C]`. With `ESI` as the ILocomotion view, these correspond to the fields read by `Draw_Matrix` as previous/current slope after the interface offset adjustment.

3. **Stock YR starts a 3-frame transition on slope change.**
   `0x004B052D` pushes `3`; `0x004B0536` calls `CDTimerClass__Start @ 0x0046B640`; `0x004B053B..0x004B0557` copies the timer tuple to the locomotor timer block and writes `3` to the transition total field (`MOV [EDX+0xC], EAX` with `EAX=3`). Active in YR: Yes; this is the first block of the standard DriveLocomotion `Process`.

4. **`Draw_Matrix` consumes cached slope fields and timer, not the map cell.**
   In `DriveLocomotionClass::Draw_Matrix @ 0x004AFF60`, the transition gate reads `[ESI+0x2C]` (`0x004AFF72`, `0x004B01B2`, `0x004B033B`); direct lookup uses `[ESI+0x18]` as the current slope (`0x004B0390..0x004B03A0` -> `VXL_GetFacingMatrix @ 0x007559B0`); interpolation passes current `[ESI+0x18]` and previous `[ESI+0x1C]` to `VXL_InterpolatedFacing @ 0x00755A40` (`0x004B0375..0x004B0389`).

5. **Interpolation is live in stock YR when the occupied-cell slope changes.**
   `Draw_Matrix` computes `t = (total - remaining) / total` from `+0x2C`, `+0x28`, `+0x20` (`0x004B01B2..0x004B01DE`, `0x004B0351..0x004B0371`) and calls `VXL_InterpolatedFacing` while `t < 1.0`; `Process` writes `+0x2C=3` on slope changes. This is standard YR drive-locomotor behavior, not a TS-only or dead branch.

## Implementation Handoff

- Current occupied-cell slope -> Rust should treat `entity.position.rx/ry` as the no-transition source only if that position is the current occupied cell after movement -> affected surface: `src/app_instances/units.rs` and the movement/rocking update that owns cached slope -> acceptance scenario: a vehicle whose current cell remains flat while its destination is a ramp still renders flat until the occupied cell changes -> proposed test `drive_vxl_slope_samples_current_cell_not_destination` -> risk: destination/next-cell sampling tilts vehicles one cell early.

- 3-frame cached slope transition -> Rust render should consume/use `RockingState.prev_slope`, `curr_slope`, and `transition_ticks_remaining` or an equivalent locomotor cache instead of snapping directly to `resolved_terrain.cell(entity.position)` every frame -> affected surface: `src/app_instances/units.rs`, `src/sim/rocking/rocking_system.rs`, VXL atlas/dynamic raster path -> acceptance scenario: first render tick after entering a ramp uses an interpolated previous-to-current slope matrix and reaches pure current slope after the 3-frame timer expires -> proposed test `drive_vxl_slope_change_blends_for_three_frames` -> risk: current Rust direct sampling visibly snaps on ramp boundaries.

- Direct path still uses cached current slope -> when no transition is active, render can use `curr_slope`/current occupied-cell slope and should not invent smoothing beyond the 3-frame binary timer -> affected surface: same render handoff -> acceptance scenario: stationary vehicle on a ramp renders using that cell's slope with no lingering previous slope after timer expiry -> proposed test `drive_vxl_slope_transition_expires_to_current_slope` -> risk: stale previous-slope blending causes wrong parked ramp tilt.

## Negative Facts / Do Not Do

- Do not sample `destination` (`DriveLocomotion +0x34`) or `head_to`/next waypoint (`+0x40`) for draw slope; the verified sample block uses owner `GetOccupiedCell` at `0x004B0510`, before any destination/track fields are consulted.
- Do not say stock YR never sets the transition gate nonzero; `0x004B053E..0x004B0557` writes `3` into the timer total on every detected occupied-cell slope change.
- Do not make interpolation a long/eased render flourish; the binary timer is exactly 3 frames from `CDTimerClass__Start(3)` at `0x004B052D..0x004B0536`.
- Do not wire `Draw_Matrix` to map terrain directly; its Ghidra body only reads locomotor cached slope/timer fields for slope matrix selection.

## Stale-Doc Wording

- `C:/Users/enok/Documents/ra2-rust-game-docs/VXL_INTERPOLATED_FACING_AND_SLOPE_TRANSITION_GHIDRA_REPORT.md`: replace claims equivalent to "No runtime writer sets locomotor+0x2C to non-zero; interpolation branch is unreachable in a normal YR game" with "DriveLocomotionClass::Process writes the slope transition total to 3 at `0x004B053E..0x004B0557` when the current occupied cell's `CellClass+0x11C` slope differs from the cached current slope; the `Draw_Matrix` interpolation branch is live for standard YR vehicles during the 3-frame slope transition."

## Remaining Uncertainty

- The exact visual fidelity required in Rust depends on whether the current pre-rendered atlas pipeline can represent intermediate SLERP matrices or needs a dynamic/expanded-cache path. The binary behavior is clear; the rendering implementation route is not decided here.

## Status

COMPLETE
