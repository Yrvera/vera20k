# BuildingClass::GetRenderCoords — decode doc

**Address:** `0x00459ef0`
**Kind:** function-decode-v1
**Source:** decompile_function @ 0x00459ef0

---

## Summary

`BuildingClass::GetRenderCoords` (0x00459ef0) returns the top-left (NW) corner
of the building's bounding box in leptons, shifted inward by half a cell (128 leptons)
on both X and Y axes. It reads Location+0x9C, +0xA0, +0xA4 (the NW-corner lepton
coords) and subtracts 0x80 (128 leptons) from both X and Y, leaving Z unchanged.

This differs from `BuildingClass::GetCoords` (0x00447ac0) which adds
`((width-1)*128, (height-1)*128)` to produce the geometric foundation center.
`GetRenderCoords` instead shifts to a different rendering reference point — 128 leptons
inward from the NW corner — used for sprite rendering alignment.

No callees. No globals. Leaf function.

---

## Active in YR

**YES — vtable slot 0xAC in BuildingClass vtable; called via vtable dispatch.**

No direct callers (verified via `get_function_callers 0x00459ef0`). All callers dispatch
through vtable.

Vtable binding confirmed: `BuildingClass vtable` base at `0x007e3ebc` (verified by decoder-1
via task #15 work). Byte offset 0xAC from base:
`0x007e3ebc + 0xAC` = `0x007e3f68`. Reading `0x007e3f68`: `f0 9e 45 00` = `0x00459ef0`.
(verified via `read_memory 0x007e3f68 length=4`)

---

## Signature

```c
// verified via decompile_function 0x00459ef0
void __thiscall BuildingClass__GetRenderCoords(int param_1, int *param_2)
```

- `param_1` — `int` (direct byte offsets). `this` pointer to BuildingClass.
- `param_2` — output `CoordStruct*` (3 × int32), filled by this function.

**Calling convention:** `__thiscall` — `param_1` is `this` in ECX.

---

## Control Flow

```c
// verified via decompile_function 0x00459ef0
void __thiscall BuildingClass__GetRenderCoords(int param_1, int *param_2) {
    int iVar1 = *(int *)(param_1 + 0xa0);   // Location.Y
    int iVar2 = *(int *)(param_1 + 0xa4);   // Location.Z
    *param_2    = *(int *)(param_1 + 0x9c) + -0x80;  // X - 128
    param_2[1]  = iVar1 + -0x80;                      // Y - 128
    param_2[2]  = iVar2;                               // Z unchanged
    return;
}
```

No branches, no guards. Unconditional 3-field read with constant offset applied to X and Y.

**`param_1` is `int`** — all field accesses are direct byte offsets (CLAUDE.md pitfall rule).

---

## Struct Field Accesses

| Byte offset from `param_1` | Type | Field | Semantics |
|---|---|---|---|
| `+0x9C` | `int32` | Location.X | NW-corner lepton X; X_out = Location.X - 128 |
| `+0xA0` | `int32` | Location.Y | NW-corner lepton Y; Y_out = Location.Y - 128 |
| `+0xA4` | `int32` | Location.Z | Ground height in leptons; Z_out = Location.Z (unchanged) |

(verified via `decompile_function 0x00459ef0`)

---

## Reference Frame

**Input:** Location frame (CLAUDE.md frame #1) — `ObjectClass+0x9C/A0/A4`.
For buildings: NW-corner cell in leptons.

**Output:** NW-corner minus half a cell on both X and Y axes.
- `X_out = Location.X - 128`  (128 = 0x80 = half cell)
- `Y_out = Location.Y - 128`
- `Z_out = Location.Z`

This is a render-alignment offset. It is NOT the geometric foundation center (that's
`BuildingClass::GetCoords` which adds `((w-1)*128, (h-1)*128)` instead). The `-128` on
both axes moves the reference point half a cell NW of the NW corner — consistent with
isometric sprite rendering where the sprite origin is centered differently than the game
coordinate.

---

## Comparison with BuildingClass::GetCoords

| Function | Address | Formula (X, Y) | Purpose |
|---|---|---|---|
| `BuildingClass::GetCoords` | `0x00447ac0` | `X + (w-1)*128, Y + (h-1)*128` | Foundation geometric center — sim/combat use |
| `BuildingClass::GetRenderCoords` | `0x00459ef0` | `X - 128, Y - 128` | Render alignment offset — sprite draw position |

`GetCoords` moves toward the foundation center; `GetRenderCoords` moves away from the
NW corner by half a cell in both axes for rendering purposes.

---

## Globals

None. The function reads only `this` fields.
(verified via `decompile_function 0x00459ef0`)

---

## INI Keys

None. The constant `0x80` (128 leptons = half cell) is hardcoded, not INI-driven.

---

## Callees

None. Leaf function.
(verified via `get_function_callees 0x00459ef0`)

---

## Observable vs Internal

**Observable:** Determines the pixel position at which a building sprite is rendered.
A wrong offset shifts the building sprite by 128 leptons (half a cell) on screen.

**Internal:** The subtraction of 0x80 is invisible to the player; only the resulting
screen position matters.

---

## Vtable binding verification

`BuildingClass vtable` base @ `0x007e3ebc`
(verified by decoder-1 via task #15 work; confirmed via team-lead spot-check).

Byte offset **0xAC** from `0x007e3ebc` = `0x007e3f68`.
`read_memory 0x007e3f68 length=4` → `f0 9e 45 00` = `0x00459ef0` — this function.
(verified via `read_memory 0x007e3f68` in this session)

Note: an earlier version of this doc incorrectly claimed vtable slot 0xD8. That was
wrong — `0x007e3ebc + 0xD8 = 0x007e3f94` holds `0x00440580` (BuildingClass::Unlimbo),
not GetRenderCoords. The correct slot is 0xAC, consistent with AircraftClass and all
other ObjectClass subclasses (polymorphic dispatch requires a fixed slot offset).

Compare: `BuildingClass::GetCoords @ 0x00447ac0` at vtable+0x4C (`0x007e3ebc + 0x4C = 0x007e3f08`).
(verified by decoder-1 in task #15; vtable+0x4C is the GetCoords slot for all ObjectClass subclasses)

---

## Rust Equivalent

```rust
// from Location (ObjectClass+0x9C, leptons): NW-corner lepton coords for buildings.
// Output: NW-corner - (128, 128, 0) leptons. Used for sprite render alignment.
fn get_render_coords(location: &CoordStruct) -> CoordStruct {
    CoordStruct {
        x: location.x - 128,
        y: location.y - 128,
        z: location.z,  // Z unchanged
    }
}
```

---

## Out-of-scope refs

| Symbol | Address | Reason deferred |
|---|---|---|
| `BuildingClass::GetCoords` | `0x00447ac0` | Foundation-center GetCoords — task #15. |
| `ObjectClass::GetRenderCoords` | `0x0041be00` | Base impl dispatches through vtable+0x48. Task #14. |

---

## Unverified

None. All claims verified from live Ghidra:
- `decompile_function 0x00459ef0` — function body
- `get_function_callers 0x00459ef0` — no direct callers
- `get_function_callees 0x00459ef0` — no callees
- `read_memory 0x007e3f68 length=4` → `f0 9e 45 00` = 0x00459ef0 — vtable slot 0xAC at BuildingClass vtable base 0x007e3ebc
