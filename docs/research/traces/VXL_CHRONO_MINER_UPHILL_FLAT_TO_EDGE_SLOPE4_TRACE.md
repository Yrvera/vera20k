# VXL Chrono Miner Uphill Flat To Edge Slope 4 Trace

**Date:** 2026-05-23  
**Slot:** trace-swarm slot 2  
**Scenario:** Chrono Miner in an active drive phase crosses from flat terrain
(`slope_type=0`) into a south-facing edge ramp cell (`slope_type=4`). Trace the
occupied-cell change and the three visual transition frames.

**Scope guard:** This report traces only the ramp-tilt/slope-transition visual
chain for the concrete `0 -> 4` uphill case. It assumes the Chrono Miner is in
the standard YR drive/piggyback movement phase where `DriveLocomotionClass` is
the active locomotor for the visible crossing. Teleport decision mechanics are
not re-traced here.

**PASS rule:** PASS is used only where both sides produce the same literal
integer/state output, or the same literal direction/order tuple. Exact pixel
output and exact float matrices are UNCHECKED unless both sides were numerically
computed.

## Pipeline

`TMP ramp_type +0x2A -> ResolvedTerrainCell.slope_type -> movement updates occupied cell -> RockingState prev/curr/timer -> render phase 0/1/2 -> transient VXL slope cache -> vxl_raster slope SLERP -> camera * slope * facing * section -> screen sprite at Position screen_x/screen_y/z`

## Source Evidence

- Stock CMIN is a voxel unit: `ini/artmd.ini:632`, `Voxel=yes` at
  `ini/artmd.ini:634`; unit rules are `[CMIN]` at `ini/rulesmd.ini:7351`.
- `TMP_ReadSlopeType` in `gamemd.exe` reads the per-tile slope byte from
  TMP cell `+0x2A`; `TMP_PER_TILE_HEIGHT_BYTE_GHIDRA_REPORT.md` names
  `CellClass+0x11C` as `SlopeIndex`.
- Rust reads the same byte as `ramp_type`: `src/assets/tmp_decode.rs:60`,
  then copies it to `ResolvedTerrainCell.slope_type`:
  `src/map/resolved_terrain.rs:1261`.
- `DriveLocomotionClass::Process` is active YR drive-locomotion code and
  samples owner occupied `CellClass+0x11C`, writes old current to previous,
  writes new byte to current, starts `CDTimerClass::Start(3)`, and stores
  transition total `3`.
- `CDTimerClass::Start` writes `g_CurrentFrameCounter` and duration `3`;
  `CDTimerClass::Remaining` returns `duration - elapsed` until zero.
- Rust slope tracking samples `terrain.cell(entity.position.rx, ry).slope_type`
  after movement phase, clamps aircraft to zero, and calls
  `update_slope_transition`: `src/sim/rocking/rocking_system.rs:203`,
  `src/sim/world/mod.rs:1192`.
- Rust render maps remaining ticks `3,2,1` to phase numbers `0,1,2`:
  `src/app_instances/units.rs:91`, then builds transition keys with
  `(type_id, facing, layer, frame, from_slope, to_slope, phase_num)`:
  `src/app_instances/units.rs:429`.
- `VXL_MasterLighting_Init` populates slope 4 with compass `0` and edge tilt;
  `Matrix3x4_BuildFromRotateXAndFacing` builds
  `Rz(compass) * Rx(+tilt) * Rz(-compass)`.
- Rust slope 4 uses `(0.0, EDGE_TILT_RAD)` and composes
  `camera_view * slope_mat * body_facing * section_transform`:
  `src/render/vxl_raster.rs:289`, `src/render/vxl_raster.rs:395`.

## Stage Table

| Stage | Output compared | gamemd output | Rust output | Verdict |
|---|---:|---:|---:|---|
| Active drive precondition | locomotor path | DriveLocomotion slope path is active only during drive/piggyback movement | Current render slope path is locomotor-agnostic once the entity is moving on terrain | UNCHECKED |
| Occupied cell used for slope sampling | sampled cell source | current occupied cell, not destination/next cell | `entity.position.rx/ry` after movement phase | PASS |
| Entered cell slope byte | sampled slope | `CellClass+0x11C = 4` | `ResolvedTerrainCell.slope_type = 4` | PASS |
| Terrain height/Z visual placement | screen anchor/Z | `CellClass.Level * 15` visual lift path; concrete map level not computed | `Position.z * 15` in `lepton_to_screen`; concrete map level not computed | UNCHECKED |
| Previous slope on change | `prev_slope` | old current slope `0` | `rocking.prev_slope = old curr_slope = 0` | PASS |
| Current slope on change | `curr_slope` | new sampled slope `4` | `rocking.curr_slope = cell_slope = 4` | PASS |
| Transition duration | total ticks | `3` | `SLOPE_TRANSITION_TICKS = 3` | PASS |
| Transition remaining sequence | remaining around visible frames | `3,2,1,0` if drawn on start frame and subsequent ticks | `3,2,1,0` under same render-after-tick ordering | PASS |
| Render phase/fraction | three visible blend values | `(3-r)/3 = 0, 1/3, 2/3` while `r=3,2,1`; stable at `1` when `r=0` | `phase_num=0,1,2`, `phase_den=3`; stable slope 4 at `r=0` | PASS |
| Transient cache key | cache identity | native key path packs current slope with facing/cache slot; no transient from/to/phase sprite cache | key includes from `0`, to `4`, phase `0/1/2`, type/facing/layer/frame | UNCHECKED |
| Final slope direction/order | directional tuple and order | slope 4 = compass `0`, positive edge tilt, slope left of body facing | slope 4 = compass `0`, positive edge tilt, `camera * slope * body_facing * section` | PASS |
| Exact final matrix floats | 12 float matrix values | not dumped for this scenario/frame | not dumped for this scenario/frame | UNCHECKED |
| Uphill/downhill symmetry | compared reverse case | not computed in this slot | not computed in this slot | UNCHECKED |

## Frame Values For The Concrete 0 -> 4 Case

Assuming render observes the same frame after the slope-change tick:

| Frame | gamemd remaining | gamemd fraction | Rust remaining | Rust phase | Rust blend |
|---:|---:|---:|---:|---:|---|
| T | 3 | 0/3 | 3 | 0 | from `0` to `4`, `t=0` |
| T+1 | 2 | 1/3 | 2 | 1 | from `0` to `4`, `t=1/3` |
| T+2 | 1 | 2/3 | 1 | 2 | from `0` to `4`, `t=2/3` |
| T+3 | 0 | stable | 0 | none | stable slope `4` |

## Findings

No directional inversion was found for this exact `0 -> 4` uphill ramp-tilt
chain. The integer state path, transition duration, render phase numbering, and
slope direction/order match the verified drive-locomotion behavior.

The remaining unchecked risk is visual, not directional: exact terrain Z anchor,
exact transient cache output, exact float matrix entries, and the reverse
downhill comparison were not numerically captured against gamemd in this slot.

## Adjacent Findings

- The scenario is only valid for Chrono Miner while it is actually in the
  drive/piggyback path. Stock CMIN's primary locomotor is TeleportLocomotion,
  with DriveLocomotion involved during the drive phase. This report does not
  re-open the teleport-vs-drive decision.
- Rust's transient cache is an implementation device, not a gamemd object. It is
  directionally complete because it keys `from_slope`, `to_slope`, and phase,
  but exact sprite equality was not proven.
- If the player still sees "moving upward while driving downhill", the next most
  likely dependency is screen/Z placement timing or movement cell-boundary
  timing, not slope-type 4 direction.

## Verdict Tally

PASS: 8 | FAIL: 0 | UNCHECKED: 5 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
