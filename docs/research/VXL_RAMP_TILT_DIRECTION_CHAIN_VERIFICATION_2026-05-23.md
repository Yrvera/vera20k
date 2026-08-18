# VXL Ramp Tilt Direction Chain Verification - 2026-05-23

## Scope

Bounded verification of whether the current Rust VXL ramp-tilt path is directionally correct after the slope-transition rendering work.

This is a coverage-map audit, not a full new locomotion investigation. It checks the player-visible direction dependencies:

1. TMP ramp byte source.
2. Cell slope byte / resolved terrain propagation.
3. Drive locomotor previous/current slope tracking.
4. Render transition phase selection.
5. VXL slope matrix compass constants.
6. Final matrix order relative to unit body facing.

Out of scope: live screenshot capture against gamemd.exe, ship-specific matrix behavior, exact bitwise floating-point equivalence of every intermediate SLERP value.

## Binary Evidence

### TMP slope byte source

`TMP_ReadSlopeType @ 0x005471B0` reads the tile-cell slope byte from `tile_cell + 0x2A` and returns 0 if the tile pointer is absent.

Relevant decompile evidence:

```c
return (int)*(char *)(piVar1[param_2 % (piVar1[1] * *piVar1) + 4] + 0x2a);
```

### Drive locomotor samples the current occupied cell

`DriveLocomotionClass::Process @ 0x004B0500` obtains the owner's current cell through vtable slot `+0x1BC`, reads `CellClass+0x11C`, and compares it with locomotor `+0x18`.

When the byte changes:

```c
piVar2[7] = piVar2[6];       // previous slope, locomotor +0x1C
piVar2[6] = (uint)bVar1;     // current slope, locomotor +0x18
CDTimerClass__Start(3);
piVar2[0xb] = 3;             // transition duration, locomotor +0x2C
```

This confirms the transition direction is old cell slope -> new current cell slope. It is not based on destination heading or vehicle facing.

### Draw path uses direct or interpolated slope matrix

`DriveLocomotionClass::Draw_Matrix @ 0x004AFF60` computes transition fraction from the duration/remaining timer fields. If the transition is complete, it uses `VXL_GetFacingMatrix(current_slope)`. If still in transition, it uses `VXL_InterpolatedFacing(previous_slope, current_slope, fraction)`; the decompiler hides some fastcall/hidden arguments, but the called function and caller field reads match the existing slope-transition docs.

`VXL_GetFacingMatrix @ 0x007559B0` indexes directly:

```c
puVar2 = (undefined4 *)(&g_VXL_FacingMatrices + param_2 * 0x30);
```

There is no slope-direction remap, clamp, or facing-dependent adjustment in this helper.

`VXL_InterpolatedFacing @ 0x00755A40` uses quaternion SLERP when the two slope indices differ and copies the matrix table entry when they are the same.

### Matrix construction and direction constants

`Matrix3x4_BuildFromRotateXAndFacing @ 0x005AE6F0` builds:

```c
Matrix3x4_RotateZ(facing);
Matrix_rotate_x_axis(tilt);
Matrix3x4_RotateZ(-facing);
```

So the slope matrix is:

```text
Rz(compass) * Rx(tilt) * Rz(-compass)
```

`VXL_MasterLighting_Init @ 0x00754CB0` populates the active slope matrix table with the following slope entries:

| Slope | Compass | Tilt family |
| --- | ---: | --- |
| 1 | 4.7124 rad / 270 deg | edge |
| 2 | pi / 180 deg | edge |
| 3 | pi/2 / 90 deg | edge |
| 4 | 0 deg | edge |
| 5, 9 | 3.9270 rad / 225 deg | corner |
| 6, 10 | 2.3562 rad / 135 deg | corner |
| 7, 11 | 0.7854 rad / 45 deg | corner |
| 8, 12 | 5.4978 rad / 315 deg | corner |
| 13 | 3.9270 rad / 225 deg | edge |
| 14 | 2.3562 rad / 135 deg | edge |
| 15 | 0.7854 rad / 45 deg | edge |
| 16 | 5.4978 rad / 315 deg | edge |

## Rust Evidence

### TMP and terrain propagation

`src/assets/tmp_decode.rs` reads:

```rust
let ramp_type: u8 = data[offset + 42];
```

`src/map/resolved_terrain.rs` copies it directly:

```rust
metadata.slope_type = tile.ramp_type;
```

No directional remap occurs between the TMP byte and `ResolvedTerrainCell.slope_type`.

### Rocking / slope-transition tracker

`src/sim/rocking/rocking_system.rs` samples `terrain.cell(entity.position.rx, entity.position.ry).slope_type` after movement in `World::tick` phase 2.5.

When the slope changes, Rust mirrors the binary state transition:

```rust
rocking.prev_slope = rocking.curr_slope;
rocking.curr_slope = cell_slope;
rocking.transition_ticks_remaining = SLOPE_TRANSITION_TICKS;
```

Aircraft force slope 0, matching the non-ground behavior boundary.

### Render phase and endpoints

`src/app_instances/units.rs` maps transition countdown to three render phases:

```rust
1..=3 => Some(3 - remaining)
```

So the three cached visual frames are:

| Remaining | Phase numerator | Visual endpoint |
| ---: | ---: | --- |
| 3 | 0 | old slope |
| 2 | 1 | intermediate |
| 1 | 2 | intermediate closer to new slope |
| 0 | stable | new slope |

This matches the binary's `(duration - remaining) / duration` shape.

### Slope directions and matrix order

`src/render/vxl_raster.rs` uses the same compass constants as `VXL_MasterLighting_Init`, with the same slope index mapping.

The final Rust limb matrix is:

```rust
camera_view * slope_mat * body_facing * section_transform
```

This is the important direction dependency: the slope matrix remains world/cell oriented and does not rotate with the unit body facing. That prevents a vehicle from tilting "forward" relative to its body when the original engine would tilt it relative to the map slope.

## Conclusion

No directional inversion or directional remap bug was found in the current Rust ramp-tilt chain.

The current implementation is directionally aligned with gamemd.exe for ground VXL slope tilt:

- TMP `ramp_type` byte is read from the same `+0x2A` source.
- The byte is preserved as terrain `slope_type`.
- The transition goes from previous occupied-cell slope to current occupied-cell slope.
- The render path uses those previous/current slope IDs.
- The slope compass constants match the binary table.
- The slope matrix is applied before body facing, so ramp direction stays map-oriented.

## Remaining Risk

The remaining uncertainty is not direction. It is exact visual parity of the intermediate SLERP frames:

- Rust uses `glam::Quat::slerp` derived from the two computed matrices.
- gamemd.exe uses its precomputed quaternion table plus `Quaternion_Slerp`.
- Endpoints and direction are correct, but mid-transition pixels may still need a gamemd/Rust screenshot comparison if exact interpolation cadence is the target.

Recommended next verification: capture the same vehicle descending and ascending the same ramp in gamemd.exe and Rust, then compare the three transition frames around the cell slope change.
