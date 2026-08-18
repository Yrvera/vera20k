# VXL Draw Matrix & Slope/Tilt System -- Ghidra Report

## Overview

`DriveLocomotionClass::Draw_Matrix` (0x4AFF60) produces the final 4x3 transformation
matrix used to render VXL (voxel) units. It handles two fundamentally different paths:

1. **Simple path** (no body tilt): When `AngleRotatedSideways` and `AngleRotatedForwards`
   are both < 0.005 radians AND the slope transition timer is complete, use only the
   pre-built facing matrix from the global lookup table.

2. **Slope/tilt path**: When the unit has nonzero body tilt angles (rocking from movement,
   weapon recoil, or slope transitions), build a rotation matrix from sin/cos of the tilt
   angles, compose it with the facing matrix, then multiply with a slope rotation matrix.

The output is a 4x3 matrix (12 floats, row-major: 3 columns of 3-vectors + translation column).

---

## Matrix Layout: 4x3 Row-Major

All matrices in the VXL pipeline are stored as 12 floats in row-major order:

```
[  m[0]  m[1]  m[2]  m[3]  ]     [ col0.x  col1.x  col2.x  tx ]
[  m[4]  m[5]  m[6]  m[7]  ]  =  [ col0.y  col1.y  col2.y  ty ]
[  m[8]  m[9]  m[10] m[11] ]     [ col0.z  col1.z  col2.z  tz ]
```

- Columns 0-2 are rotation/scale (3x3 basis vectors)
- Column 3 is translation (tx, ty, tz)
- Size: 48 bytes (0x30) per matrix

---

## 1. VXL_GetFacingMatrix (0x7559B0)

```c
void __fastcall VXL_GetFacingMatrix(float *out_ECX, int slope_index_EDX)
```

Simple table lookup: copies 12 floats from the global facing matrix table.

**Address:** `g_VXL_FacingMatrices` @ **0xB45188**
**Entry size:** 0x30 (48 bytes, 12 floats)
**Index formula:** `base + slope_index * 0x30`

The table is populated at startup by `VXL_MasterLighting_Init` (0x755430). It contains
pre-computed rotation matrices for each of the slope orientations (0-7), built using
`Matrix3x4_BuildFromRotateXAndFacing` which composes a Z-rotation (facing) with an
X-rotation (camera tilt), effectively encoding both the isometric camera angle and
the slope tilt into a single pre-baked matrix.

---

## 2. VXL_InterpolatedFacing (0x755A40)

```c
float * __fastcall VXL_InterpolatedFacing(
    float *out_ECX, int prev_slope_EDX, int curr_slope, double fraction)
```

When `prev_slope != curr_slope` (unit is transitioning between slopes), this function
interpolates between the two facing matrices using **quaternion SLERP**.

**Quaternion table:** `DAT_00B43188`, 16 bytes per entry (4 floats: x, y, z, w)

**Logic:**
- If `prev_slope == curr_slope`: just copy the matrix from `g_VXL_FacingMatrices[curr_slope]`
- Otherwise: SLERP between quaternions at `[prev_slope]` and `[curr_slope]` by `fraction`,
  then convert the result quaternion back to a 4x3 matrix via `Quaternion_ToMatrix`

The quaternion table is initialized alongside the facing matrix table during
`VXL_MasterLighting_Init`, using `Quaternion_FromAxisAngle` (0x646480).

### Quaternion_Slerp (0x646590)

Standard SLERP implementation with three cases:
1. **Nearly identical** (dot product >= 1.0 - epsilon): linear interpolation
2. **Nearly opposite** (dot product <= -1.0 + epsilon): use perpendicular rotation
   via cos lookup as a fallback (avoids division by zero in the sin denominator)
3. **Normal case**: `result = q1 * sin((1-t)*omega)/sin(omega) + q2 * sin(t*omega)/sin(omega)`
   where `omega = acos(dot(q1, q2))`

Uses `Acos_lookup` (0x4CADB0) which is a table-based inverse cosine.

### Quaternion_ToMatrix (0x646980)

Standard quaternion-to-rotation-matrix conversion. Given q = (x, y, z, w):
```
m[0] = 1 - 2(y^2 + z^2)    m[1] = 2(xy - wz)        m[2] = 2(xz + wy)
m[4] = 2(xy + wz)           m[5] = 1 - 2(x^2 + z^2)  m[6] = 2(yz - wx)
m[8] = 2(xz - wy)           m[9] = 2(yz + wx)         m[10] = 1 - 2(x^2 + y^2)
```
Translation column (m[3], m[7], m[11]) is always 0.

---

## 3. BuildFacingRotationMatrix (0x55A730)

```c
void BuildFacingRotationMatrix(int loco, float *out_matrix, uint *turret_facing)
```

Builds a 2D rotation matrix from the **rate timer** (game tick interpolation for smooth
rotation between ticks). This handles sub-tick facing interpolation:

1. Calls `Matrix3x4_SetIdentity` on an internal matrix
2. Reads the rate timer from `techno + 0x388` via `RateTimer__Current` (0x4C93D0)
3. Extracts a 5-bit sub-facing: `((timer >> 10) + 1) >> 1 & 0x1F` (range 0-31)
4. Subtracts 8 (centering to range -8..+23)
5. Converts to angle: `sub_facing * (-PI/16)` where `-PI/16 = -11.25 degrees`
   (constant at 0x7E4408, loaded as double). Full 32 steps = 360 degrees.
6. Calls `Matrix3x4_RotateZ` with that angle
7. If `turret_facing` is not null and not -1, encodes the full facing as:
   `*turret_facing = (*turret_facing << 5) | sub_facing_bits`
8. Copies the result to the output matrix

The turret_facing encoding packs: `(body_facing * 64 + slope_index) * 32 + sub_facing`.

---

## 4. Matrix3x4_Copy (0x5AE610)

```c
void __thiscall Matrix3x4_Copy(float *dest, float *src)
```

Simple memcpy of 12 floats (48 bytes). Used extensively throughout the pipeline.

---

## 5. Matrix3x4_SetIdentity / ResetMatrix (0x5AE860)

```c
void __fastcall Matrix3x4_SetIdentity(float *mat)
```

Sets the 4x3 matrix to identity:
```
1  0  0  0
0  1  0  0
0  0  1  0
```
`0x3F800000` = 1.0f in the diagonal, all other entries = 0.

---

## 6. Locomotion_Matrix / Matrix3x4_Multiply (0x5AF980)

```c
void __fastcall Locomotion_Matrix(float *out, float *A, float *B)
```

Full 4x3 matrix multiplication: `out = A * B`

The 3x3 rotation part is multiplied normally. The translation column includes the
affine transform: `out.translation = A.rotation * B.translation + A.translation`

This is called at the end of Draw_Matrix to compose:
- **Slope path:** `result = facing_matrix * slope_tilt_matrix * facing_rotation_matrix`
- **Simple path:** `result = facing_matrix * facing_rotation_matrix`

---

## 7. Sin_lookup (0x4CAD00) / Cos_lookup (0x4CACB0)

Both use the **same** precomputed sine table at **0x84F084**.

**Table layout:**
- 8192 entries (0x2000) of 32-bit floats
- Covers one full period: `table[i] = sin(2*PI*i/8192)`
- Verified: `table[0] = 0.0`, `table[1] = 0.000766990`, etc.

**Sin_lookup (0x4CAD00):**
- Input: angle on FPU stack (the engine passes radians which are then scaled internally)
- Calls `Math__ftol` to convert to integer index
- Masks to 0-8191 range with wrap-around
- Returns `table[index + 0x800]` (offset by 2048 entries = 90-degree phase shift)

**Cos_lookup (0x4CACB0):**
- Same table, but without the +0x800 offset
- Returns `table[index]`

**IMPORTANT NOTE on naming:** The function labeled `Sin_lookup` in Ghidra has a +2048
entry phase shift, and `Cos_lookup` has no shift. Since `sin(x + PI/2) = cos(x)`:
- `Sin_lookup(angle)` mathematically returns **cos** of the input angle
- `Cos_lookup(angle)` mathematically returns **sin** of the input angle

The existing Ghidra names are preserved to avoid churn, but the EXISTING report
`docs/VOXEL_SLOPE_TILT_SYSTEM.md` has the correct identification:
`FUN_004cad00 = cos, FUN_004cacb0 = sin`. The names are counterintuitive but consistent
-- interpret them as "the function at that address" rather than what trig function it
computes.

There is also `Sin_Lookup_Table4096` (0x4CAD50) which uses a smaller 4096-entry table
at **0x85D0A4** -- used for different subsystems.

---

## 8. Matrix_shear_col3_by_col0/1/2 (0x5AE980, 0x5AE9B0, 0x5AE9E0)

These add a scaled basis column to the translation column:

```c
// Matrix_shear_col3_by_col0: translation += factor * column0
mat[3]  += factor * mat[0];
mat[7]  += factor * mat[4];
mat[11] += factor * mat[8];

// Matrix_shear_col3_by_col1: translation += factor * column1
mat[3]  += factor * mat[1];
mat[7]  += factor * mat[5];
mat[11] += factor * mat[9];

// Matrix_shear_col3_by_col2: translation += factor * column2
mat[3]  += factor * mat[2];
mat[7]  += factor * mat[6];
mat[11] += factor * mat[10];
```

In the slope path of Draw_Matrix, these are used to translate the model origin based on
the tilt -- effectively shifting the voxel model so it pivots around the correct ground
contact point rather than its center.

---

## 9. Matrix_rotate_x_axis (0x5AEF60) / Matrix_rotate_y_axis (0x5AF080)

Apply rotation around X or Y axis to an existing matrix (post-multiply by rotation).

**Matrix_rotate_x_axis:** Rotates columns 1 and 2 by the angle:
```
new_col1 = col2 * cos(angle) + col1 * sin(angle)
new_col2 = col2 * sin(angle) - col1 * cos(angle)
```

**Matrix_rotate_y_axis:** Rotates columns 0 and 2 by the angle:
```
new_col0 = col0 * sin(angle) - col2 * cos(angle)
new_col2 = col0 * cos(angle) + col2 * sin(angle)
```

In Draw_Matrix, these are called with `AngleRotatedSideways` (body roll, +0x328) and
`AngleRotatedForwards` (body pitch, +0x32C) respectively.

---

## 10. Matrix3x4_RotateZ / RotateMatrix2D (0x5AF1A0)

Rotates columns 0 and 1 around the Z axis:
```
new_col0 = col0 * sin(angle) + col1 * cos(angle)
new_col1 = col1 * sin(angle) - col0 * cos(angle)
```

Used in `BuildFacingRotationMatrix` for the sub-tick facing rotation and in
`Matrix3x4_BuildFromRotateXAndFacing` for camera-angle composition.

---

## 11. CellClass SlopeIndex (offset 0x11C = 284)

**Type:** `byte` (u8) on the CellClass struct.

**Source:** Read from the tile's section tailer during map load via `TMP_ReadSlopeType`
(0x5471B0). The value comes from byte offset 0x2A in the TMP tile sub-tile data.

**Values:** 0-7 (0 = flat, 1-7 = various slope orientations). These directly index into
the `g_VXL_FacingMatrices` table.

**Read path in Process:**
```c
iVar4 = techno->GetCell();  // vtable call at +0x1BC
bVar1 = *(byte*)(iVar4 + 0x11C);  // CellClass.SlopeIndex
```

---

## 12. Slope Transition Timer (DriveLocomotionClass fields)

When the SlopeIndex changes, a 3-frame countdown timer starts. During this time,
VXL_InterpolatedFacing is used to SLERP between the old and new slope matrices.

### DriveLocomotionClass field layout (byte offsets from `this`):

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| +0x18 | 4 | current_slope | Current SlopeIndex (int) |
| +0x1C | 4 | previous_slope | Previous SlopeIndex before transition |
| +0x20 | 4 | timer_start_frame | Frame counter when timer started (-1 = inactive) |
| +0x24 | 4 | timer_unknown | (possibly unused, set from uninitialized local) |
| +0x28 | 4 | timer_remaining | Frames remaining (CDTimerClass::Remaining) |
| +0x2C | 4 | timer_duration | Total timer duration (always 3) |

### Transition fraction computation:

```c
int duration = loco->timer_duration;       // +0x2C, always 3
int remaining = CDTimerClass__Remaining(); // reads +0x20, +0x28
double fraction = (double)(duration - remaining) / (double)duration;
// fraction goes from 0.0 to 1.0 over 3 frames
```

When `fraction >= 1.0` (timer expired), the transition is complete and the simple path
is used. When `fraction < 1.0`, `VXL_InterpolatedFacing(prev_slope, curr_slope, fraction)`
is called to get the interpolated matrix.

### CDTimerClass layout (3 ints):
- `[0]` = start frame (g_CurrentFrameCounter when started, -1 = inactive)
- `[1]` = (unused/padding)
- `[2]` = duration in frames

`Remaining = max(0, duration - (current_frame - start_frame))`

---

## 13. AngleRotatedSideways (+0x328) and AngleRotatedForwards (+0x32C)

These are `float` fields on **TechnoClass** (not the locomotor):
- **+0x328** = `AngleRotatedSideways` (body roll)
- **+0x32C** = `AngleRotatedForwards` (body pitch)

Updated by `TechnoClass::RockingUpdate` (0x70B570), which is a ~600-instruction function
implementing a spring-damper oscillation system. The rocking is driven by:

- `RockingSidewaysPerFrame` (+0x330) and `RockingForwardsPerFrame` (+0x334): per-frame
  velocity of the angle
- Damping toward zero with acceleration constant from `Rules->ShakeScreen` (+0x18B8)
- Clamped to approximately +/-0.7854 radians (45 degrees) for normal rocking
- Special handling for `IsSinking` (only pitch, with periodic reversal)
- Special handling for `field_0x425` (direct per-frame addition without damping)

### Epsilon check in Draw_Matrix:

```c
if (fraction >= 1.0
    && ABS(techno->AngleRotatedSideways) < 0.005   // ~0.29 degrees
    && ABS(techno->AngleRotatedForwards) < 0.005)
{
    // SIMPLE PATH: no body tilt needed
}
else
{
    // SLOPE/TILT PATH: build full rotation
}
```

The epsilon threshold is stored as a **double** at 0x7E44E8 = **0.005 radians**.

---

## 14. TechnoTypeClass Tilt Magnitude Fields

Draw_Matrix reads two **doubles** from the TechnoTypeClass (returned by vtable call +0x84):

| Offset | Type | Field | Computation |
|--------|------|-------|-------------|
| +0x360 | double | tilt_magnitude_X | `vxl_section.size_y * 0.5` |
| +0x368 | double | tilt_magnitude_Y | `vxl_section.size_x * 0.5` |

These represent half the VXL bounding box dimensions, used to scale the slope tilt into
world-space translation offsets. Computed at the end of `TechnoTypeClass::ReadINI` when
the unit type has VXL rendering (`param_1[0x2C] != 0`).

### Also on TechnoTypeClass (separate INI keys):

| Offset | Type | INI Key |
|--------|------|---------|
| +0x3A0 | double | RollAngle |
| +0x3A8 | double | PitchSpeed |
| +0x3B0 | double | PitchAngle |

These are used by `TechnoClass::RockingUpdate` and other systems, NOT by Draw_Matrix.

---

## 15. Complete Draw_Matrix Pipeline (Slope/Tilt Path)

When the unit has nonzero body tilt or an active slope transition:

### Step 1: Read tilt parameters
```
tilt_mag_x = TypeClass->tilt_magnitude_X;  // +0x360, double
tilt_mag_y = TypeClass->tilt_magnitude_Y;  // +0x368, double
pitch = techno->AngleRotatedForwards;       // +0x32C, float
roll  = techno->AngleRotatedSideways;       // +0x328, float
```

### Step 2: Compute shear offsets
```
sin_pitch = Sin_lookup(pitch)
cos_pitch = Cos_lookup(pitch)
sin_roll  = Sin_lookup(roll)
cos_roll  = Cos_lookup(roll)

combined_Z = ftol(|cos_roll| * tilt_mag_x + |cos_pitch| * tilt_mag_y)
partial_Y  = ftol(sin_pitch * tilt_mag_x)
remainder_Y = ftol(tilt_mag_x - partial_Y) // fractional leftover
partial_X  = ftol(sin_roll * tilt_mag_y)  // wait -- need to verify exact flow
...
```

The intermediate values are converted to integers via `Math__ftol`, then adjusted
by sign based on the sign of pitch and roll angles.

### Step 3: Build slope rotation matrix
```
Matrix A = identity;
Matrix_shear_col3_by_col2(A, combined_Z_offset);
Matrix_shear_col3_by_col0(A, x_offset);
Matrix_shear_col3_by_col1(A, y_offset);
Matrix_rotate_x_axis(A, roll);       // AngleRotatedSideways
Matrix_rotate_y_axis(A, pitch);      // AngleRotatedForwards
```

### Step 4: Force turret_facing to -1
Since the slope rotation replaces the standard facing encoding, the turret_facing
parameter is set to -1 to disable VXL-level facing selection.

### Step 5: Get facing rotation matrix
```
facing_rot = BuildFacingRotationMatrix(loco, turret_facing);
```

### Step 6: Get facing matrix (with interpolation)
```
if (timer_active && fraction < 1.0):
    facing_mat = VXL_InterpolatedFacing(prev_slope, curr_slope, fraction)
else:
    facing_mat = VXL_GetFacingMatrix(curr_slope)
```

### Step 7: Compose final matrix
```
result = Locomotion_Matrix(
    Locomotion_Matrix(facing_mat, slope_tilt_matrix),
    facing_rot_matrix
)
```

Actually the exact composition from disassembly at the end:
```
temp1 = Locomotion_Matrix(slope_tilt_matrix, facing_mat)       // 0x5AF980 first call
temp2 = Locomotion_Matrix(temp1, facing_rot_matrix)            // second call
result = Locomotion_Matrix(temp2, ???)                         // final call at 0x4B03DA
```

The final matrix multiplication at 0x4B03DA is the same for both paths and produces
the output written to `param_2`.

---

## 16. Simple Path (No Tilt)

When body tilt is negligible and the slope transition is complete:

1. Compute `facing_rot = BuildFacingRotationMatrix(loco, turret_facing)`
2. Copy facing_rot to the slope matrix slot (identity rotation, just the facing)
3. Get `facing_mat = VXL_GetFacingMatrix(curr_slope)` or interpolated
4. Compose: `result = Locomotion_Matrix(facing_mat, facing_rot)`

The turret_facing parameter encodes: `(body_facing * 64 + slope_index) * 32 + sub_facing`

---

## 17. VXL_MasterLighting_Init (0x755430) -- Facing Matrix Setup

This function initializes:
- `g_VXL_FacingMatrices` (0xB45188): 16+ pre-baked facing matrices at specific angles
- Quaternion table (0xB43188): corresponding quaternions for SLERP interpolation
- `g_VXL_ViewMatrices` (0xB43F40): 4 view matrices at different camera angles
- `g_VXL_NormalVectors` (0xB432D8): 256 pre-rotated normal vectors for lighting

### Facing matrix angles (radians -> degrees):

The matrices are built with `Matrix3x4_BuildFromRotateXAndFacing` which does:
```
identity -> RotateZ(facing_angle) -> RotateX(camera_tilt) -> RotateZ(-facing_angle)
```

This creates the isometric projection rotation at each facing direction.

**Stored angles (from hex float constants):**

| Hex | Radians | Degrees | Direction |
|-----|---------|---------|-----------|
| 0x3F490E56 | 0.7854 | 45 | NE |
| 0x4016CAC1 | 2.3561 | 135 | NW |
| 0x407B51EC | 3.9269 | 225 | SW |
| 0x40AFEC8B | 5.4976 | 315 | SE |
| 0x3FC90E56 | 1.5708 | 90 | N |
| 0x40490E56 | 3.1415 | 180 | W |
| 0x4096CAC1 | 4.7123 | 270 | S |
| 0x00000000 | 0.0000 | 0 | E |

The matrices at indices 1-4 use one camera tilt (from `_DAT_00B44310`), indices 5-8
use a second tilt, and so on with different tilt/rotation combinations.

### Special fixed matrices at end (0xB450B8 - 0xB45178):

The last few entries in the range below the main table are set to identity with
specific axis flips:
- Index at 0xB450B8: identity with `-1` in specific diagonal entries (mirror matrices)
- These handle slope orientations where the surface normal is inverted

---

## 18. Helper Function Summary

| Address | Name | Signature | Purpose |
|---------|------|-----------|---------|
| 0x7559B0 | VXL_GetFacingMatrix | (out, slope_index) | Lookup pre-built facing matrix |
| 0x755A40 | VXL_InterpolatedFacing | (out, prev, curr, frac) | SLERP between slope matrices |
| 0x55A730 | BuildFacingRotationMatrix | (loco, out, facing) | Sub-tick facing rotation |
| 0x5AE610 | Matrix3x4_Copy | (dst, src) | Copy 12 floats |
| 0x5AE860 | Matrix3x4_SetIdentity | (mat) | Set to identity |
| 0x5AE980 | Matrix_shear_col3_by_col0 | (mat, factor) | Translate along col0 |
| 0x5AE9B0 | Matrix_shear_col3_by_col1 | (mat, factor) | Translate along col1 |
| 0x5AE9E0 | Matrix_shear_col3_by_col2 | (mat, factor) | Translate along col2 |
| 0x5AEF60 | Matrix_rotate_x_axis | (mat, angle) | Rotate around X |
| 0x5AF080 | Matrix_rotate_y_axis | (mat, angle) | Rotate around Y |
| 0x5AF1A0 | Matrix3x4_RotateZ | (mat, angle) | Rotate around Z |
| 0x5AF980 | Locomotion_Matrix | (out, A, B) | Matrix multiply A*B |
| 0x5AFB80 | Matrix3x4_TransformPoint | (out, mat, point) | Transform 3D point |
| 0x5AE6F0 | Matrix3x4_BuildFromRotateXAndFacing | (mat, facing, tilt) | Camera-angle matrix |
| 0x5AE750 | Matrix3x4_BuildAxisAngleRotation | (mat, axis, angle) | Axis-angle rotation |
| 0x4CAD00 | Sin_lookup | (angle on FPU) | Sin from 8192-entry table |
| 0x4CACB0 | Cos_lookup | (angle on FPU) | Cos from same table |
| 0x4CADB0 | Acos_lookup | (value on FPU) | Acos from table |
| 0x646590 | Quaternion_Slerp | (out, q1, q2, t) | Spherical linear interp |
| 0x646980 | Quaternion_ToMatrix | (out, quat) | Quaternion to 4x3 matrix |
| 0x646480 | Quaternion_FromAxisAngle | (out, axis, angle) | Axis-angle to quaternion |
| 0x645C50 | Quaternion_Set | (q, x, y, z, w) | Set quaternion components |
| 0x645D20 | Quaternion_CopyAndStore | (dst1, dst2, src) | Copy quaternion |
| 0x70B570 | TechnoClass__RockingUpdate | (this) | Spring-damper body oscillation |
| 0x4AFB40 | DriveLocomotionClass__Force_New_Slope | (loco, slope) | Set slope without transition |
| 0x4B04D0 | DriveLocomotionClass__Update_Facing_From_Type | (loco) | Read SlopeIndex from cell |
| 0x7C5F00 | Math__ftol | () | Float-to-long from FPU |

---

## 19. Global Data Addresses

| Address | Name | Size | Description |
|---------|------|------|-------------|
| 0xB45188 | g_VXL_FacingMatrices | ~0x600 | Pre-baked 4x3 matrices per slope index |
| 0xB43188 | (quaternion table) | 16*N | Quaternions for SLERP interpolation |
| 0xB43F40 | g_VXL_ViewMatrices | 4*48 | View matrices for different camera angles |
| 0xB432D8 | g_VXL_NormalVectors | 256*12 | Pre-rotated lighting normals |
| 0x84F084 | (sin/cos table) | 8192*4 | Sin lookup: table[i] = sin(2*PI*i/8192) |
| 0x85D0A4 | (sin table 4096) | 4096*4 | Alternate sin table (smaller) |
| 0xA8ED84 | g_CurrentFrameCounter | 4 | Current game frame |
| 0x7E1718 | (constant) | 8 | double 1.0 |
| 0x7E1748 | (constant) | 4 | float 0.0 |
| 0x7E44E8 | (constant) | 8 | double 0.005 (tilt epsilon, ~0.29 degrees) |
| 0x7E5168 | (constant) | 4 | float 0.5 |
| 0x7F6954 | (constant) | 4 | float 2*PI/256 = 0.02454 (per-facing angle) |

---

## Confidence Levels

- **Verified from binary (high confidence):**
  - All function signatures and decompilations
  - Matrix layout (4x3, 12 floats, row-major)
  - Sin/cos lookup table structure and values
  - CellClass.SlopeIndex at byte offset 0x11C (284)
  - TechnoClass.AngleRotatedSideways at +0x328, AngleRotatedForwards at +0x32C
  - DriveLocomotionClass slope timer fields (+0x18 through +0x2C)
  - Epsilon threshold = 0.005 radians (double at 0x7E44E8)
  - Facing angles: 45, 90, 135, 180, 225, 270, 315, 0 degrees

- **Inferred with moderate confidence:**
  - TechnoTypeClass +0x360/+0x368 are VXL bounding box half-sizes (from init code pattern)
  - The exact composition order in the slope path (3 sequential matrix multiplies)
  - The precise mapping of slope indices 0-7 to terrain orientations

- **Needs further verification:**
  - The full set of slope index values and their geometric meanings (0=flat, 1-7=?)
    (see `docs/VOXEL_SLOPE_TILT_SYSTEM.md` for prior research on slope types 0-20)
  - How the special mirror matrices at 0xB450B8-0xB45178 are indexed
  - The camera tilt angles stored at runtime addresses 0xB43F08 and 0xB44310
  - **Sin_lookup vs Cos_lookup naming:** The Ghidra labels may be swapped from their
    mathematical meaning. `Sin_lookup` (0x4CAD00) uses a +2048 entry phase shift
    (sin(x+PI/2) = cos(x)), while `Cos_lookup` (0x4CACB0) has no shift. The prior
    report `docs/VOXEL_SLOPE_TILT_SYSTEM.md` identifies them as `4cad00=cos, 4cacb0=sin`
    which matches the mathematical analysis. The Ghidra labels are kept as-is to avoid
    confusion with existing code references.
