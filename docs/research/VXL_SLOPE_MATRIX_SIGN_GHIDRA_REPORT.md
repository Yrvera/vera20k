# VXL Slope Matrix Sign - Ghidra Report

Date: 2026-05-23
Target: populated gamemd VXL slope matrices for slope types 1-16, with emphasis on slope_type 4 South and 2 North.
Status: COMPLETE

## Target Question

Does gamemd's populated VXL slope matrix raise/lower the same model-space side that Rust currently raises in `src/render/vxl_raster.rs::compute_slope_rotation`, which uses:

```text
Rz(compass) * Rx(+tilt) * Rz(-compass)
```

## Non-Goals

- No code changes.
- No Ghidra mutations.
- No attempt to prove whether Rust's model `+Y` is world-south in every render path.
- No investigation of dynamic driving pitch/roll or dead slope interpolation behavior beyond sign relevance.

## Evidence Needed To Mark COMPLETE

- Verify the gamemd table population angles for slope_type 2 and 4.
- Verify the builder composition order and tilt sign.
- Verify the low-level `Rx` convention for a positive tilt on model axes.
- Compare against current Rust `compute_slope_rotation`.

## Stop Conditions

Stop after proving the sign convention for slope_type 2 and 4, and after checking whether any obvious stale-doc wording should be carried forward.

## Verified Binary Facts

1. Active in YR: Yes. `VXL_GetFacingMatrix` at `0x007559B0` copies `DAT_00b45188 + slope_type * 0x30` with no sign flip or remap: assembly context `0x007559B1..0x007559C7` computes `EDX + EDX*2`, shifts by 4, adds `0xB45188`, then copies 12 dwords.
2. Active in YR: Yes. `VXL_MasterLighting_Init` at `0x00754CB0` populates slope_type 2 at `DAT_00b451e8` from `Matrix3x4_BuildFromRotateXAndFacing(0x40490e56, fVar1)`, where `0x40490e56` is pi and `fVar1` is the edge tilt.
3. Active in YR: Yes. The same init populates slope_type 4 at `DAT_00b45248` from `Matrix3x4_BuildFromRotateXAndFacing(0, fVar1)`, so South uses compass 0 with the same positive edge-tilt argument.
4. Active in YR: Yes. `Matrix3x4_BuildFromRotateXAndFacing` at `0x005AE6F0` initializes identity, calls `Matrix3x4_RotateZ(param_2)`, then `Matrix_rotate_x_axis(param_3)`, then `Matrix3x4_RotateZ(-param_2)`. Assembly context `0x005AE6F0..0x005AE712` confirms identity setup; decompile confirms the three calls.
5. Active in YR: Yes. `Matrix_rotate_x_axis` at `0x005AEF60` with positive angle maps identity to rows `[1,0,0]`, `[0,cos,-sin]`, `[0,sin,cos]` after accounting for the swapped trig helper labels (`0x004CAD00` returns cos; `0x004CACB0` returns sin). Therefore a positive X tilt maps model `+Y` to positive `Z`.

## Sign Result

- Slope_type 4 / South: gamemd uses compass 0 and positive edge tilt. This reduces to `Rx(+tilt)`, so model `+Y` is raised (`z = +sin(tilt)`) and model `-Y` is lowered.
- Slope_type 2 / North: gamemd uses compass pi and positive edge tilt. In `Rz(pi) * Rx(+tilt) * Rz(-pi)`, model `+Y` is lowered (`z = -sin(tilt)`) and model `-Y` is raised.
- This matches current Rust's sign convention in `compute_slope_rotation`: positive `tilt_rad` is correct for matching gamemd's populated matrix side in model space.

## Implementation Handoff

- Verified behavior -> gamemd positive edge tilt raises model `+Y` for slope 4 and model `-Y` for slope 2 -> Rust delta -> do not negate `tilt_rad` in `compute_slope_rotation` for this reason -> affected surface -> `src/render/vxl_raster.rs::compute_slope_rotation` and its slope 2/4 tests -> acceptance scenario -> render/transform a two-point fixture on slope 4 and 2 and assert the raised model-space side matches gamemd -> proposed test name -> `test_vxl_slope_2_and_4_raise_gamemd_model_side` -> risk -> low if limited to matrix sign, but visual world-direction naming remains a separate check.
- Verified behavior -> `VXL_GetFacingMatrix` indexes `DAT_00b45188 + slope_type * 0x30` directly -> Rust delta -> keep the existing direct slope_type mapping for 1-16; do not add a slope 2/4 swap to fix a downhill visual symptom -> affected surface -> render VXL slope selection -> acceptance scenario -> slope_type 2 produces the opposite model-Y sign from slope_type 4 -> proposed test name -> `test_vxl_slope_2_is_opposite_of_slope_4` -> risk -> medium only if the upstream terrain-to-model-axis mapping is wrong.
- Verified behavior -> gamemd's populated matrices are pure rotations with no translation in the slope entries -> Rust delta -> keep slope handling as a rotation matrix, not a height offset correction -> affected surface -> VXL sprite bounds and limb transforms -> acceptance scenario -> flat/slope matrix does not add translation to origin -> proposed test name -> `test_vxl_slope_matrix_has_no_translation` -> risk -> low.

## Negative Facts / Do Not Do

- Do not negate `tilt_rad` merely because slope_type 4 is named South; binary `Rx(+tilt)` raises model `+Y` for the compass-0 entry at `DAT_00b45248`.
- Do not swap slope_type 2 and 4 to fix the Chrono Miner downhill symptom; the table entries are direct: type 2 is pi at `DAT_00b451e8`, type 4 is 0 at `DAT_00b45248`.
- Do not treat the `Sin_lookup` / `Cos_lookup` labels literally when deriving the matrix sign; `0x004CAD00` has the +0x800 sine-table phase and is the cosine value, while `0x004CACB0` is the sine value.
- Do not use the dormant interpolation quaternion table to infer compass direction; same-magnitude entries lose direction there, while the live direct lookup path reads `DAT_00b45188`.
- Do not apply translation/height offsets as a substitute for matrix sign; the builder writes a pure 3x3 rotation in a 3x4 container.

## Remaining Uncertainty

- The binary-side matrix sign is resolved, but the player-visible downhill symptom could still come from a separate mismatch in world compass naming, terrain slope_type assignment, model-axis convention before `section_transform`, or draw-order/projection composition.
- I did not verify a live screenshot or runtime capture of a Chrono Miner descending a specific map slope in this slot.

## Stale-Doc Wording

- `C:/Users/enok/Documents/ra2-rust-game-docs/VOXEL_SLOPE_TILT_SYSTEM.md`: the `Rust Engine Status` bullet `Slope tilt matrix computation and application during VXL rendering` is stale if the current Rust file is authoritative; suggested replacement: `Slope tilt matrix computation is implemented in src/render/vxl_raster.rs; remaining risk is visual validation of world-direction/model-axis mapping.`

## Conclusion

The answer to the target question is yes: for the populated gamemd slope matrices, Rust's current positive-tilt `Rz(compass) * Rx(tilt) * Rz(-compass)` raises/lowers the same model-space side for slope_type 4 South and slope_type 2 North. The Chrono Miner downhill symptom should not be addressed by blindly negating `tilt_rad`; investigate world-direction mapping or slope assignment next.
