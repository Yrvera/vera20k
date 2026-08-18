# ObjectClass__GetRenderCoords — decode

## Summary

`ObjectClass__GetRenderCoords` (`0x0041be00`) is the default GetRenderCoords virtual
method. It is a thin wrapper that dispatches `GetCoords` via vtable+0x48 on `this` and
copies the result into the output buffer. For most game objects, GetRenderCoords and
GetCoords produce identical output — rendering uses the same coordinate as the sim
position. The building override (`BuildingClass__GetRenderCoords @ 0x00459ef0`) is the
notable exception: it shifts the render origin half a cell (128 leptons = 0x80) west and
north relative to the raw Location field, to align the building sprite with its map tile.

GetRenderCoords is at vtable slot **`+0xAC`** (byte offset from vtable pointer). It
appears in 20 vtable entries. `BuildingClass__GetRenderCoords` overrides it in one.

**Verified via `decompile_function 0x0041be00`, `disassemble_function 0x0041be00`,
`disassemble_function 0x00459ef0`, `decompile_function 0x00459ef0`,
`get_xrefs_to 0x0041be00`, `read_memory 0x007e2350`.**

## Active in YR

YES. `ObjectClass__GetRenderCoords` is in 20 vtables covering all mobile unit types
(aircraft, infantry, vehicle). `BuildingClass__GetRenderCoords` covers all buildings.
Both are dispatched every render frame. No TS-only gate detected.

## Addresses

- `ObjectClass__GetRenderCoords`: `0x0041be00`
- `BuildingClass__GetRenderCoords` (override): `0x00459ef0`

## Vtable Slot

**`vtable + 0xAC`** (byte offset from vtable pointer). Verified:
- AircraftClass vtable pointer base: `0x007e22a4` (slot 0 = `AircraftClass__Destructor`).
- `ObjectClass__GetRenderCoords @ 0x0041be00` is in the AircraftClass vtable at address
  `0x007e2350`. Offset = `0x007e2350 − 0x007e22a4 = 0xAC`.
- `read_memory 0x007e2350` returns `00 be 41 00` = `0x0041be00`. Confirmed.

## Signature — ObjectClass__GetRenderCoords

```c
// __fastcall: ECX = object pointer (this)
// One stdcall output arg: out_buf = pointer to 12-byte CoordStruct
// Returns: void (writes into out_buf)
// RET 0x4 = pops one 4-byte output-buffer arg
void __fastcall ObjectClass__GetRenderCoords(int *this, CoordStruct *out_buf);
```

Ghidra shows `__fastcall` with `int *param_1` (the object pointer in ECX). The explicit
stack argument is the output buffer. `RET 0x4` confirms one 4-byte arg is cleaned from
the stack. Verified via `disassemble_function 0x0041be00`.

## Signature — BuildingClass__GetRenderCoords

```c
// __thiscall: ECX = building object pointer (this)
// One stdcall output arg: out_buf = pointer to 12-byte CoordStruct
// Returns: void (writes into out_buf)
// RET 0x4 = pops one 4-byte output-buffer arg
void __thiscall BuildingClass__GetRenderCoords(int param_1, int *out_buf);
```

Ghidra shows `__thiscall` with `int param_1` (direct byte offsets). Verified via
`disassemble_function 0x00459ef0`.

## Parameters

| Name | Type | Location | Meaning |
|------|------|----------|---------|
| `this` (ObjectClass) | `int *` | ECX (fastcall) | Pointer to object — dereferenced to get vtable for GetCoords dispatch |
| `this` (BuildingClass) | `int` | ECX (thiscall) | Direct byte-offset pointer to building object |
| `out_buf` | `int *` / `undefined4 *` | `[ESP+4]` (after saves) | Destination 12-byte CoordStruct |

## Control Flow — ObjectClass__GetRenderCoords

Single basic block (18 instructions):

```
0041be00: MOV EAX, [ECX]           ; EAX = vtable pointer of this
0041be02: SUB ESP, 0xC             ; allocate 12-byte local CoordStruct
0041be05: LEA EDX, [ESP]           ; EDX = pointer to local buf
0041be09: PUSH ESI
0041be0a: PUSH EDX                 ; push local buf as GetCoords output arg
0041be0b: CALL dword ptr [EAX+0x48]; dispatch GetCoords (vtable+0x48)
0041be0e: MOV ECX, EAX             ; ECX = result ptr (decompiler artefact — actual result in local buf)
0041be10: MOV EAX, [ESP+0x14]      ; EAX = out_buf (caller's output buffer)
0041be14: MOV EDX, EAX
0041be16: MOV ESI, [ECX]           ; copy X from local buf
0041be18: MOV [EDX], ESI
0041be1a: MOV ESI, [ECX+4]         ; copy Y
0041be1d: MOV [EDX+4], ESI
0041be20: POP ESI
0041be21: MOV ECX, [ECX+8]         ; copy Z
0041be24: MOV [EDX+8], ECX
0041be27: ADD ESP, 0xC             ; restore stack
0041be2a: RET 0x4
```

Verified via `disassemble_function 0x0041be00`.

The function calls `GetCoords` through the object's own vtable (`[EAX+0x48]`) with a
local 12-byte buffer, then copies the result into the caller's output buffer. This is
a pure delegate with no transformation — the render coordinate equals the sim coordinate
for non-building objects.

## Control Flow — BuildingClass__GetRenderCoords

Single basic block (15 instructions):

```
00459ef0: ADD ECX, 0x9C            ; ECX = &this->Location (X field)
00459ef6: PUSH ESI
00459ef7: MOV EDX, ECX             ; EDX = &Location
00459ef9: PUSH EDI
00459efa: MOV EAX, [EDX]           ; EAX = Location.X
00459efc: MOV ECX, [EDX+4]         ; ECX = Location.Y
00459eff: LEA ESI, [EAX - 0x80]    ; ESI = X - 128 leptons
00459f02: MOV EAX, [ESP+0xC]       ; EAX = out_buf
00459f06: ADD ECX, -0x80           ; ECX = Y - 128 leptons
00459f09: MOV EDX, [EDX+8]         ; EDX = Location.Z (unchanged)
00459f0c: MOV EDI, EAX
00459f0e: MOV [EDI], ESI           ; out_buf.X = Location.X - 0x80
00459f10: MOV [EDI+4], ECX         ; out_buf.Y = Location.Y - 0x80
00459f13: MOV [EDI+8], EDX         ; out_buf.Z = Location.Z
00459f16: POP EDI
00459f17: POP ESI
00459f18: RET 0x4
```

Verified via `disassemble_function 0x00459ef0`.

## Behavioral Analysis

### ObjectClass__GetRenderCoords

Pure delegate to `GetCoords` (vtable+0x48). Returns whatever the object's GetCoords
implementation returns:
- For mobile units using `ObjectClass__GetCoords @ 0x005f65a0`: returns Location
  frame `{ECX+0x9C, ECX+0xA0, ECX+0xA4}` (leptons, body center / NW corner).
- For objects using `AbstractClass__GetCoords @ 0x004104c0`: returns `{0, 0, 0}` sentinel.

**No position transformation is applied.** Render = sim position for non-buildings.

### BuildingClass__GetRenderCoords

Reads `Location` (`this+0x9C/0xA0/0xA4`) and shifts X and Y by **−128 leptons (−0x80)**
each. Z is passed through unchanged.

```
render_X = Location.X - 128   // = Location.X - 0.5 cells
render_Y = Location.Y - 128   // = Location.Y - 0.5 cells
render_Z = Location.Z          // unchanged
```

**Why −0x80?** The building `Location` field for buildings stores the NW corner of the
building footprint in leptons (same as the Location frame for mobile units, but anchored
to the NW corner cell). The building sprite is drawn from a render origin that is offset
half a cell (0.5 cells = 128 leptons) to the NW of `Location`, aligning the sprite's
reference point with the correct isometric tile position. This shift is **only for
rendering** — the sim position (`Location`) is unmodified.

**Concrete fixture (GAREFN 4×3 at NW cell (10,10)):**
- `Location.X` = 10 × 256 = 2560 leptons
- `Location.Y` = 10 × 256 = 2560 leptons
- `render_X` = 2560 − 128 = 2432 leptons = 9.5 cells (NW of cell 10)
- `render_Y` = 2560 − 128 = 2432 leptons = 9.5 cells (NW of cell 10)

This shifts the render origin 0.5 cells NW of the NW corner cell. The building sprite
is drawn from this point to encompass the full isometric foundation footprint.

## Struct Field Accesses

### ObjectClass__GetRenderCoords (`0x0041be00`)
| Access | Offset | Meaning |
|---|---|---|
| `[ECX]` | 0 | vtable pointer of the object |
| `[EAX+0x48]` | vtable+0x48 | GetCoords virtual method pointer |

### BuildingClass__GetRenderCoords (`0x00459ef0`)
| Offset from `this` | Size | Field | Semantics |
|---|---|---|---|
| `+0x9C` | 4 bytes (int32) | Location.X | Leptons, east-positive. From Location frame. |
| `+0xA0` | 4 bytes (int32) | Location.Y | Leptons, south-positive. |
| `+0xA4` | 4 bytes (int32) | Location.Z | Leptons, height. |

`param_1` is typed `int` → offsets are direct byte offsets. Verified by disassembly
`ADD ECX, 0x9C` followed by `[EDX]`, `[EDX+4]`, `[EDX+8]`.

## Callers

`get_function_callers` returns no direct callers for either function — both are called
exclusively through vtable dispatch (`[vtable + 0xAC]`). `get_xrefs_to 0x0041be00`
shows 20 DATA xrefs (vtable entries); `get_xrefs_to 0x00459ef0` shows 1 DATA xref
(BuildingClass vtable at `0x007e3f68`).

## Callees

- `ObjectClass__GetRenderCoords`: no named callees (the `CALL [EAX+0x48]` is an
  indirect dispatch through vtable; Ghidra does not resolve it as a named callee).
- `BuildingClass__GetRenderCoords`: no callees. Leaf function.

## Globals

None accessed directly by either function.

## INI Keys

None.

## Enums

None.

## Load-Bearing vs Internal

**Load-bearing:**
- The vtable slot offset `+0xAC` for GetRenderCoords is load-bearing.
- The `−0x80` (−128 lepton) shift in `BuildingClass__GetRenderCoords` is load-bearing
  for correct building sprite placement. Omitting it places building sprites 0.5 cells
  too far east-south; applying it to mobile units (incorrect) would displace their
  sprites 0.5 cells NW.
- The delegate pattern in `ObjectClass__GetRenderCoords` (render = sim position) means
  any object that incorrectly overrides GetRenderCoords with a transformed value will
  display at the wrong position.

**Internal:**
- The local 12-byte stack buffer used as an intermediate in `ObjectClass__GetRenderCoords`
  is an implementation detail.
- The register choices (ESI/EDI) in `BuildingClass__GetRenderCoords`.

## Out-of-Scope Refs

- `ObjectClass__GetCoords @ 0x005f65a0` — the GetCoords implementation dispatched by
  `ObjectClass__GetRenderCoords` for mobile units; task #12.
- `BuildingClass__GetCoords @ 0x00447AC0` — building GetCoords with foundation-center
  offset; task #15.
- `AbstractClass__GetCoords @ 0x004104c0` — base GetCoords sentinel; task #11 (completed).
- `BuildingClass__GetRenderCoords @ 0x00459ef0` — documented in this file; separate task
  #16 may cover it in more depth.

## Rust Equivalent

```rust
// vtable+0xAC: GetRenderCoords
// ObjectClass default: delegate to GetCoords
fn get_render_coords_default(obj: &dyn GameObj) -> CoordStruct {
    obj.get_coords()
}

// BuildingClass override: Location - (128, 128, 0) leptons
fn get_render_coords_building(building: &Building) -> CoordStruct {
    CoordStruct {
        x: building.location.x - 128,
        y: building.location.y - 128,
        z: building.location.z,
    }
}
```

## SELF-PROOF — 3 random load-bearing claims verified

1. **`CALL dword ptr [EAX+0x48]`** (vtable+0x48 GetCoords dispatch inside GetRenderCoords):
   confirmed from `disassemble_function 0x0041be00` — instruction at `0x0041be0b` is
   `CALL dword ptr [EAX + 0x48]`. Verified.
2. **`LEA ESI, [EAX - 0x80]`** (−128 lepton X shift in BuildingClass override):
   confirmed from `disassemble_function 0x00459ef0` — instruction at `0x00459eff` is
   `LEA ESI,[EAX + -0x80]`. Verified.
3. **vtable+0xAC = GetRenderCoords**: `read_memory 0x007e2350` returns `00 be 41 00`
   = `0x0041be00`. AircraftClass vtable base = `0x007e22a4`. Offset = `0x007e2350 −
   0x007e22a4 = 0xAC`. Verified.

## Unverified

None. All claims verified from live Ghidra in this session.
