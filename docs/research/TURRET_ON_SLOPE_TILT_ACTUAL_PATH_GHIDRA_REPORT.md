# Turret-on-Slope Tilt: Actual Render Path — Ghidra Report

**Date:** 2026-05-19  
**Status:** COMPLETE  
**Confidence:** HIGH (content, identity, binding all verified from binary)

---

## Summary

The turret VXL inherits the body's slope-tilt matrix through the render call chain.
No separate "turret tilt" code exists. The same `camera × body_Draw_Matrix` composition
that orients the body VXL is passed directly into the turret draw path, and
`FUN_00707280` applies it as matrix A when composing the turret's VXL section frame.

**Hypothesis ranking:**
- **H1 (CONFIRMED):** Turret reuses body tilt matrix — emergent from matrix composition.
- **H2 (RULED OUT):** `GetFLH`/`GetRenderFLH` compute tilt inline. Both call vtable +0x2a8
  which returns an encoded facing integer, not a matrix. They produce FLH world positions, not tilt.
- **H3 (NOT FOUND):** No separate helper function for turret slope orientation. No such code path exists.

---

## Key Verified Facts (all verified from binary in this session)

1. **`DriveLocomotionClass::Draw_Matrix` is at ILocomotion vtable offset +0x24** (slot 9),
   not +0x14. The ILocomotion interface vtable for DriveLocomotionClass lives at `0x007e7eb0`.
   The primary C++ vtable lives at `0x007e7ec0` (16 bytes later). The xref `FROM 0x007e7ed4`
   for Draw_Matrix confirms it is slot +0x14 of the *primary* C++ vtable only.

2. **`UnitClass::DrawPips` calls `Draw_Matrix` at `0x0073b5cb`** via `[vtable + 0x24]` on the
   ILocomotion interface pointer stored at `[EBP + 0x674]`. Output goes into `[ESP + 0x108]`.
   Then `Locomotion_Matrix` at `0x0073b71c` composes: `[ESP+0x114] = camera × body_slope_matrix`.
   This composed matrix is `uVar12`.

3. **`uVar12` (`camera × body_slope_matrix`) is passed as param_7 to `FUN_004db0d0`**
   (call at `0x0073c5c4`), forwarded as param_7 to `VXL_turret_draw` (`0x00706BD0`),
   and arrives at `FUN_00707280` (`0x00707280`) as **param_4**.

4. **`FUN_00707280` first `Locomotion_Matrix` call** (`0x007072d3`, `Locomotion_Matrix @ 0x005AF980`):
   - ECX (output) = `[ESP + 0xbc]` = local result buffer
   - EDX (A matrix) = `[entry_ESP + 0x0c]` = **param_4** = `camera × body_slope_matrix`
   - B (stack) = VXL section frame data (48 bytes copied from VXL file)
   - Result: `local = (camera × body_slope_matrix) × vxl_section_frame`

5. **DriveLocomotionClass object layout** (verified from `Constructor @ 0x004AF540`):
   - `[obj + 0x00]`: IUnknown vtable ptr → `0x007e7ec0` (primary C++ vtable, starts with Is_Moving)
   - `[obj + 0x04]`: ILocomotion vtable ptr → `0x007e7eb0` (ILocomotion interface vtable)
   - `[obj + 0x18]`: IPiggyback vtable ptr
   - `[TechnoClass + 0x674]`: stores ILocomotion interface pointer = `obj + 0x04`

---

## ILocomotion Vtable Layout (`0x007e7eb0`)

Verified from `read_memory(0x007e7eb0, 80)`:

| Slot | Offset | Address    | Identified As                          |
|------|--------|------------|----------------------------------------|
|  0   | +0x00  | 0x004B4D90 | ILocomotion_QueryInterface             |
|  1   | +0x04  | 0x004B4DA0 | ILocomotion_AddRef                     |
|  2   | +0x08  | 0x004B4DB0 | ILocomotion_Release                    |
|  3   | +0x0c  | 0x0055A710 | (base class default, unidentified)     |
|  4   | +0x10  | 0x004AFB80 | Is_Moving                              |
|  5   | +0x14  | 0x004AFC90 | Destination                            |
|  6   | +0x18  | 0x004AFCC0 | Head_To_Coord                          |
|  7   | +0x1c  | 0x0055ABF0 | (base class default)                   |
|  8   | +0x20  | 0x0055ABE0 | (base class default)                   |
|  9   | +0x24  | 0x004AFF60 | **Draw_Matrix**                        |
| 10   | +0x28  | 0x004B0410 | Shadow_Matrix                          |
| 11   | +0x2c  | 0x0055ABD0 | Shadow_Point (base default, returns 0) |
| 12   | +0x30  | 0x0055A8C0 | Draw_Point (base default, returns screen Y) |
| 13   | +0x34  | 0x0055ABC0 | (base class default)                   |
| 14   | +0x38  | 0x004B4870 | In_Which_Layer                         |
| 15   | +0x3c  | 0x004B4880 | (unknown)                              |
| 16   | +0x40  | 0x004B0500 | **Process** (movement tick)            |
| 17   | +0x44  | 0x004AFD40 | Set_Destination                        |
| 18   | +0x48  | 0x004AFE00 | Stop_Moving                            |
| 19   | +0x4c  | 0x004B0EF0 | Do_Turn                                |

The primary C++ vtable at `0x007e7ec0` is this same block offset by +0x10 (starts at slot 4 = Is_Moving).

---

## Full Render Path: Body + Turret on Slope

### Body render path (`TechnoClass::Render @ 0x00706ED0`):
```
Draw_Matrix(locomotor, body_matrix_out, facing_param)    // [ILoco vtable +0x24]
  → body_matrix = slope_rotation × facing_rotation       // (or with dynamic tilt)
Locomotion_Matrix(camera × body_matrix)                  // per-section
  → submit to VXL_Submit_Billboard / VXL_Sort_Rasterize
```

### Turret render path (per `UnitClass::DrawPips @ 0x0073B500`):
```
[0x0073b5cb] Draw_Matrix(locomotor, body_matrix_out, facing_param)
                // ILocomotion vtable slot +0x24 = 0x004AFF60
                // writes slope × facing composed matrix to [ESP+0x108]

[0x0073b70e] FUN_00754be0(camera_buffer)
                // copies static camera matrix (DAT_00b44318) to [ESP+0xe4]

[0x0073b71c] Locomotion_Matrix(out=[ESP+0x114], A=camera, B=body_matrix)
                // uVar12 = camera × body_slope_matrix

[0x0073c5c4] FUN_004db0d0(EBX+0xb0, ..., uVar12, ...)
                // param_7 = uVar12

[0x004db190] VXL_turret_draw(..., param_6=auStack_c, param_7=uVar12, ...)
                // auStack_c = output of ILoco vtable +0x30 = Draw_Point = {0, screen_Y_adj}
                // NOT a matrix; used for screen position, not tilt

[0x00706bd0 → 0x00707280] FUN_00707280(this, ..., param_4=uVar12, param_5=auStack_c, ...)
                // First Locomotion_Matrix (at 0x007072d3):
                //   A = param_4 = uVar12 = (camera × body_slope_matrix)
                //   B = VXL turret section frame
                //   out = (camera × body_slope_matrix) × turret_vxl_frame

                // Second Locomotion_Matrix (at 0x0070736c):
                //   A = DAT_00887430 (camera matrix again — for billboard orientation)
                //   B = result of first composition
                //   → submit to rasterizer
```

### Why this produces correct turret tilt

`Draw_Matrix` encodes the vehicle's current slope (from cell slope index lookup into `DAT_00b45188`)
combined with facing rotation. This slope component is present in `body_slope_matrix`.
`uVar12 = camera × body_slope_matrix` carries both camera orientation AND slope tilt.
When FUN_00707280 applies `uVar12 × turret_vxl_frame`, the turret section inherits the
same slope orientation as the body. No separate "turret tilt" calculation is needed —
it's structurally emergent from the shared matrix chain.

---

## Previous Misidentification: FUN_00729B40

`FUN_00729B40` (formerly suspected as "turret tilt") is confirmed TS-dormant:
- Only xref: DATA at `0x007F5A48` = TunnelLocomotionClass vtable slot 9
- Zero code callers in normal YR execution
- Correct identity: `TunnelLocomotionClass::Draw_Matrix` (TS-only locomotor)

---

## Rust Implementation Note

The turret tilt is NOT a separate compute step. The implementation follows naturally:

```rust
// body render:
let body_matrix = loco.draw_matrix(facing, sub_facing);
let final_body = camera_matrix * body_matrix * section_frame;

// turret render (same body_matrix reused):
let composed = camera_matrix * body_matrix;  // uVar12 equivalent
let final_turret = composed * turret_section_frame;
```

The key: pass `camera × body_Draw_Matrix` (not just `body_Draw_Matrix`) to the turret draw function.
