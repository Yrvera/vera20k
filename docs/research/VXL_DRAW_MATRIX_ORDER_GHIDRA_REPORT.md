# VXL Draw Matrix Order - Ghidra Slot 2 Report

Target question: In `gamemd.exe`, is vehicle VXL slope tilt applied before or after facing/camera/body transform, and is Rust `rotate_to_world * slope_mat * section_transform` composition-compatible with the simple no-body-rock path?

Non-goals: full body-rock pivot/shear reduction, slope sign mapping, cell sampling, Rust code edits.

Evidence needed to mark COMPLETE: `DriveLocomotionClass::Draw_Matrix` simple-path multiply order, slope-table construction order, later camera/view composition, and current Rust surface.

Stop conditions: read-only Ghidra only; write this report plus `.swarm-claims.md` only.

## Answer

For the simple path, native composition is not compatible with Rust's current `rotate_to_world * slope_mat * section_transform` if `rotate_to_world` contains body facing.

`gamemd.exe` builds the vehicle draw matrix as:

```text
draw_matrix = slope_matrix * facing_rotation
section_camera_matrix = camera_view * draw_matrix * section_or_hva_matrix
```

With column-vector semantics, this means a model point is first section/HVA transformed, then body/sub-facing rotated, then slope-tilted, then camera/view transformed.

Rust's current shape:

```text
combined = (camera_view * body_facing) * slope_matrix * section_transform
```

tilts in local/model space before body facing. That is the reversed slope-vs-facing relationship for the simple path, and matches the reported symptom class: a vehicle can lean as if the ramp direction is rotating with the unit body instead of remaining in world/cell slope space.

## Verified Facts

1. `DriveLocomotionClass::Draw_Matrix @ 0x004AFF60` selects the simple path when transition fraction is `1.0` and `abs(Techno+0x328)` / `abs(Techno+0x32C)` are both below `0.005` (`0x007E44E8`). Active in YR: Yes; this is the normal non-rocking vehicle VXL draw path.

2. In the simple path, `BuildFacingRotationMatrix @ 0x0055A730` builds the facing/sub-facing rotation, then `VXL_GetFacingMatrix @ 0x007559B0` copies `g_VXL_FacingMatrices + current_slope * 0x30`; the final multiply at `0x004B03CB..0x004B03DA` passes `EDX = slope_matrix` and stack arg `= facing_rotation` to `Locomotion_Matrix @ 0x005AF980`.

3. `Locomotion_Matrix @ 0x005AF980` computes `out = param_2 * param_3` using normal affine matrix multiplication. Evidence: decompile translation term is `A.rotation * B.translation + A.translation`, where `param_2` is A and `param_3` is B. Active in YR: Yes; many render paths call it.

4. `Matrix3x4_BuildFromRotateXAndFacing @ 0x005AE6F0` constructs each slope-table entry as identity -> `Matrix3x4_RotateZ(angle)` -> `Matrix_rotate_x_axis(tilt)` -> `Matrix3x4_RotateZ(-angle)`. `VXL_MasterLighting_Init @ 0x00754CB0` populates slope entries at `0x00B451B8..0x00B45488` with these compass+tilt pairs. Active in YR: Yes; init-time table used by draw.

5. `TechnoClass::Render @ 0x00706ED0` applies view/camera after the locomotor draw matrix: the call at `0x00706FCE` passes `EDX = 0x00887430` and stack arg `= section/draw matrix` to `Locomotion_Matrix`, then submits that result to `VXL_Submit_BoundingBox @ 0x007540F0`. Active in YR: Yes; body sections render through this loop.

## Implementation Handoff

- Verified behavior -> simple vehicle VXL transform is `camera_view * slope_matrix * facing_rotation * section_or_hva_matrix` -> Rust delta -> split body facing out of `rotate_to_world` and compose `camera_view * slope_mat * body_facing * section_transform` for the simple slope/no-rock path -> affected surface -> `src/render/vxl_raster.rs::prepare_limb_data` and GPU mirror of `LimbRenderData.combined` -> acceptance scenario -> Chrono Miner descending a one-cell ramp leans with the ramp/world slope, not upward relative to its body heading -> proposed test name -> `vxl_simple_slope_applies_after_body_facing_before_camera` -> risk -> changing cached atlas keys/visual snapshots for all sloped voxel vehicles.

- Verified behavior -> slope table is world/cell-slope orientation, not a local body-space tilt -> Rust delta -> keep slope direction independent from `params.facing` except via later camera projection -> affected surface -> `compute_slope_rotation` call ordering, not the table constants themselves -> acceptance scenario -> same slope type rendered at facings 0/64/128/192 preserves the same world ramp lean direction after camera projection -> proposed test name -> `vxl_slope_direction_does_not_rotate_with_vehicle_facing` -> risk -> existing sign tests may need expected-frame updates if they encoded the old local-space order.

## Negative Facts / Do Not Do

- Do not fix this symptom by flipping `tilt_rad` alone; the verified mismatch is matrix order, and a sign flip would only trade which facings look wrong. Evidence: `0x004B03DA` simple path multiplies slope table left of facing rotation.

- Do not fold slope into `rotate_to_world` while `rotate_to_world` still includes body facing. Evidence: native camera/view composition occurs later in `TechnoClass::Render @ 0x00706FCE`, while slope-vs-facing is already resolved inside `Draw_Matrix`.

- Do not implement stock slope interpolation as part of this matrix-order fix. Whether interpolation is active or expired, the requested simple no-body-rock endpoint uses direct `VXL_GetFacingMatrix` and the same slope-left-of-facing order.

- Do not treat `VXL_GetFacingMatrix(0)` as the normal flat camera matrix. In the simple path, slope 0 bypasses the table and leaves the slope slot identity before multiplying by facing rotation.

## Remaining Uncertainty

- Full body-rock path ordering includes additional pivot/shear and three multiplies at `0x004B0292`, `0x004B02A0`, and `0x004B03DA`; this report only proves the requested simple slope/no-body-rock path.

- Exact Rust camera basis may require matching `0x00887430` / rasterizer view conventions before visual tests can be pixel-exact; the slope-vs-facing order conclusion does not depend on that basis.

## Stale Doc Wording

- `docs/research/VXL_DRAW_MATRIX_GHIDRA_REPORT.md`: replace the uncertain Section 15/16 composition wording with: "For the simple no-body-rock path, `DriveLocomotionClass::Draw_Matrix` returns `slope_matrix * facing_rotation`; slope 0 uses identity for `slope_matrix`. `TechnoClass::Render` later applies `camera_view * draw_matrix * section_or_hva_matrix` before `VXL_Submit_BoundingBox`. Therefore slope is left of body/sub-facing and right of camera/view."

## Status

COMPLETE
