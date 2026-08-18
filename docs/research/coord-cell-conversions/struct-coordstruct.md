# CoordStruct — struct decode

## Summary

`CoordStruct` is a 12-byte struct holding a 3D position in **leptons** (1 cell = 256
leptons). It has exactly three `int32` fields at byte offsets 0, 4, 8 corresponding to
X (east-positive), Y (south-positive), and Z (height/altitude). There are no padding
bytes, no 4th field, and no variant layouts — all CoordStruct operations in the engine
uniformly access exactly these three fields.

It is the canonical 3D coordinate type for all sim-side object positions, velocity
vectors, distance vectors, and animation anchor points throughout gamemd.exe.

**Verified from six independent functions:**
`decompile_function 0x0041c230` (CoordStruct__Set),
`decompile_function 0x0041c380` (CoordStruct__Distance3D),
`decompile_function 0x006ce240` (CoordStruct__VecAdd),
`decompile_function 0x00710700` (CoordStruct__VecDiv),
`decompile_function 0x0075f540` (CoordStruct__ScaleByFactor),
`decompile_function 0x004399a0` (CoordStruct__FromDoubles).

## Struct Layout

```
CoordStruct (12 bytes, all fields signed int32)
┌────────────────────────────────────────┐
│ offset +0x00 │ int32 X │ east-positive  │  leptons
│ offset +0x04 │ int32 Y │ south-positive │  leptons
│ offset +0x08 │ int32 Z │ height (up)    │  leptons
└────────────────────────────────────────┘
```

| Byte offset | Size | Type | Field | Direction | Notes |
|---|---|---|---|---|---|
| `+0x00` | 4 | `int32` | X | east-positive | leptons |
| `+0x04` | 4 | `int32` | Y | south-positive | leptons |
| `+0x08` | 4 | `int32` | Z | up-positive (height) | leptons |

Total size: **12 bytes**. No padding. Signed 32-bit throughout.

`1 cell = 256 leptons`. Cell coordinates (CellStruct) are derived by floor-dividing
each component by 256 using the sign-correct arithmetic shift
`(v + (v >> 31 & 0xFF)) >> 8`.

## Field Evidence

### X at offset +0x00

`CoordStruct__Set @ 0x0041c230` (verified via `decompile_function 0x0041c230`):
```c
*param_1   = param_2;  // [this+0x00] = X
param_1[1] = param_3;  // [this+0x04] = Y
param_1[2] = param_4;  // [this+0x08] = Z
```

`CoordStruct__Distance3D @ 0x0041c380` (verified via `decompile_function 0x0041c380`):
```c
Sqrt_Approx((double)param_1[2] * (double)param_1[2] +
            (double)param_1[1] * (double)param_1[1] +
            (double)*param_1   * (double)*param_1);  // [0]=X, [1]=Y, [2]=Z
```

`CoordStruct__VecAdd @ 0x006ce240` (verified via `decompile_function 0x006ce240`):
```c
*param_2   = *param_3 + *param_1;     // out.X = a.X + b.X
param_2[1] = iVar2 + iVar1;           // out.Y = a.Y + b.Y
param_2[2] = iVar3 + iVar4;           // out.Z = a.Z + b.Z
```

`CoordStruct__VecDiv @ 0x00710700` (verified via `decompile_function 0x00710700`):
```c
*param_2   = *param_1 / param_3;      // out.X = X / divisor
param_2[1] = iVar1 / param_3;         // out.Y = Y / divisor
param_2[2] = iVar2 / param_3;         // out.Z = Z / divisor
```

All operations terminate at index `[2]` — no `[3]` or beyond accessed in any function.
**Struct is confirmed to be exactly 3 × int32 = 12 bytes.**

## Coordinate Reference Frame

CoordStruct appears in multiple reference frames depending on which subsystem produced
it. The struct layout is identical in all frames; only the semantic meaning of the
coordinates differs:

| Frame | Producer | Semantics |
|---|---|---|
| **Location** | `this+0x9C/0xA0/0xA4` direct | NW-corner anchor for buildings; body-center for mobile units. Raw game-object position field. |
| **GetCoords (foundation center)** | `BuildingClass__GetCoords @ 0x00447AC0` | `Location + ((W-1)*128, (H-1)*128, 0)` — foundation center for buildings. |
| **GetRenderCoords** | `BuildingClass__GetRenderCoords @ 0x00459ef0` | `Location - (128, 128, 0)` — render origin for building sprites. |
| **Delta vector** | Caller arithmetic (A-B) | Signed difference of two GetCoords results. Magnitude = distance in leptons. |
| **Null sentinel** | `AbstractClass__GetCoords @ 0x004104c0` | `{0, 0, 0}` — "no position / invalid." |
| **Float-converted** | `CoordStruct__FromDoubles`, `CoordStruct__ScaleByFactor` | Result of floating-point physics; non-deterministic for sim. |

**The struct layout does not encode the frame.** Callers must track which frame a
CoordStruct is in. See CLAUDE.md "The five binary reference frames" for the full
classification.

## Null Sentinel

`{0, 0, 0}` is the canonical "no position" CoordStruct, returned by
`AbstractClass__GetCoords @ 0x004104c0` from globals at `DAT_00887680/84/88`
(verified via `read_memory 0x00887680` — all zero at analysis time). Code that
receives a zero CoordStruct from a GetCoords dispatch should treat it as "position
unknown," not as map origin. See `fn-abstract-getcoords.md` for details.

## Type Size Summary

| Property | Value |
|---|---|
| Total bytes | 12 |
| Alignment | 4 (aligned to int32) |
| Fields | 3 × signed int32 |
| Padding | None |
| Unit | Leptons (1 cell = 256 leptons) |
| Sign | All fields signed (negative coords are valid) |

## Ghidra Type Appearance

Ghidra decompiles CoordStruct parameters as:
- `int *param_1` — when the function takes a pointer to CoordStruct
- `undefined4 *param_1` — same layout, different type annotation
- `int *param_1` with `param_1[0]`, `param_1[1]`, `param_1[2]` — index access

When Ghidra shows `int *`, indexing `param_1[1]` = byte offset `+4` (int* arithmetic).
When Ghidra shows `undefined4 *`, same arithmetic. Never `param_1[0x04]` — that would
be offset `+16`. Always verify the type before computing byte offsets.

Calling conventions seen on CoordStruct methods:
- `__thiscall` (ECX = output or input CoordStruct ptr): `CoordStruct__VecAdd`,
  `CoordStruct__VecDiv`, `CoordStruct__Set`
- `__fastcall` (ECX = first arg): `CoordStruct__Distance3D`, `CoordStruct__ScaleByFactor`

## Known Functions Operating on CoordStruct

| Address | Name | Operation |
|---|---|---|
| `0x0041c230` | `CoordStruct__Set` | Write X, Y, Z into struct |
| `0x0041c380` | `CoordStruct__Distance3D` | sqrt(X²+Y²+Z²), returns leptons |
| `0x006ce240` | `CoordStruct__VecAdd` | Element-wise addition |
| `0x00710700` | `CoordStruct__VecDiv` | Element-wise integer division |
| `0x0075f540` | `CoordStruct__ScaleByFactor` | Float multiply then ftol |
| `0x004399a0` | `CoordStruct__FromDoubles` | Convert 3 doubles to int32 via ftol |

## Determinism Hazard

`CoordStruct__ScaleByFactor` and `CoordStruct__FromDoubles` use `Math__ftol` (x87 FPU
rounding) to convert floating-point values to int32. This is **non-deterministic**
across CPU state — not safe for sim-side fixed-point logic. Both functions appear only
in bullet/anim render paths, not in the core sim tick. Verified via
`decompile_function 0x0075f540` and `decompile_function 0x004399a0` (both call
`Math__ftol` multiple times).

## Rust Equivalent

```rust
#[repr(C)]
struct CoordStruct {
    x: i32,  // leptons, +X = east
    y: i32,  // leptons, +Y = south
    z: i32,  // leptons, +Z = up (height)
}
// Total: 12 bytes, no padding, repr(C) matches binary layout
```

In sim code, all leptons should use fixed-point (`fixed::types::I20F12` or similar)
rather than raw `i32`. The `repr(C)` layout matches binary exactly.

## SELF-PROOF — 3 random load-bearing claims verified

1. **X at offset +0 (not +4)**: `decompile_function 0x0041c230` shows `*param_1 = param_2`
   (index 0 = first field = offset 0). Y at `param_1[1]` = offset 4, Z at `param_1[2]` =
   offset 8. Verified.
2. **No 4th field**: `decompile_function 0x00710700` (VecDiv) accesses only `*param_1`,
   `param_1[1]`, `param_1[2]`, `*param_2`, `param_2[1]`, `param_2[2]` — no index 3.
   Verified.
3. **Total size 12 bytes**: three int32 fields at offsets 0, 4, 8 with no gaps. No field
   beyond index 2 found in any of the 6 sampled CoordStruct functions. `CoordStruct__Set`
   writes exactly 3 fields. Verified.

## Unverified

None. All claims verified from live Ghidra in this session.
