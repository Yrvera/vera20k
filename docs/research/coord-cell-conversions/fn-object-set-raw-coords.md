# ObjectClass::Set_Raw_Coords — decode doc

**Address:** `0x005f6940`
**Kind:** function-decode-v1
**Source:** decompile_function @ 0x005f6940

---

## Summary

`ObjectClass::Set_Raw_Coords` (0x005f6940) writes a 3-component CoordStruct directly
into the object's Location fields at ObjectClass+0x9C, +0xA0, +0xA4. It is the inverse
of `ObjectClass::GetCoords` (0x005f65a0) — a direct raw write to the Location frame
with no validation, snapping, or event notification.

No callees. No globals. Leaf function.

---

## Active in YR

**YES — called in constructor and special-coordinate contexts.**

4 direct callers via `get_function_callers 0x005f6940`:
- `BuildingLightClass__Constructor` @ `0x00435820` — sets initial position
- `IsometricTileClass__Constructor` @ `0x00543780` — tile position initialization
- `ParticleClass__Constructor` @ `0x0062b5e0` — particle spawn position
- `TechnoClass__Set_Coords_With_Cloak` @ `0x004db810` — sets coords with cloak handling

All active in normal YR gameplay.

---

## Signature

```c
// verified via decompile_function 0x005f6940
void __thiscall ObjectClass__Set_Raw_Coords(int param_1, undefined4 *param_2)
```

- `param_1` — `int` (direct byte offsets). `this` pointer to ObjectClass.
- `param_2` — `undefined4*`. Input CoordStruct: X at [0], Y at [1], Z at [2].

**`param_1` is `int`** — all field accesses are direct byte offsets (CLAUDE.md pitfall rule).

**Calling convention:** `__thiscall` — `param_1` in ECX; `param_2` on stack.

---

## Control Flow

```c
// verified via decompile_function 0x005f6940
void __thiscall ObjectClass__Set_Raw_Coords(int param_1, undefined4 *param_2) {
    *(undefined4 *)(param_1 + 0x9c) = *param_2;    // Location.X = param_2[0]
    *(undefined4 *)(param_1 + 0xa0) = param_2[1];  // Location.Y = param_2[1]
    *(undefined4 *)(param_1 + 0xa4) = param_2[2];  // Location.Z = param_2[2]
    return;
}
```

No branches, no guards. Unconditional 3-word write to Location fields.

---

## Struct Field Accesses

| Byte offset from `param_1` | Type | Field | Semantics |
|---|---|---|---|
| `+0x9C` | `int32` | Location.X | Written: input X leptons |
| `+0xA0` | `int32` | Location.Y | Written: input Y leptons |
| `+0xA4` | `int32` | Location.Z | Written: input Z leptons |

(verified via `decompile_function 0x005f6940`)

---

## Relationship to GetCoords

| Function | Address | Direction |
|---|---|---|
| `ObjectClass::GetCoords` | `0x005f65a0` | Location → output CoordStruct (read) |
| `ObjectClass::Set_Raw_Coords` | `0x005f6940` | Input CoordStruct → Location (write) |

They are exact inverses. Neither does validation or snapping.

---

## Globals / INI Keys

None.

---

## Callees

None. Leaf function.
(verified via `get_function_callees 0x005f6940`)

---

## Observable vs Internal

**Observable:** Direct writes to Location fields immediately affect the object's map position.
Used in constructors to set spawn location — wrong values displace the object at spawn.
`TechnoClass__Set_Coords_With_Cloak` is sim-side; wrong coords there produce visible
unit teleportation.

**Internal:** The raw write itself is invisible; only the resulting position is observable.

---

## "Raw" semantics

"Raw" means no change notification is issued and no snapping to cell grid occurs. It is
the primitive setter used when the caller already has the exact lepton coord to write.
Contrast with locomotor-mediated position updates which go through `Force_Track` or
`DriveLocomotion::Process_Movement`.

---

## Rust Equivalent

```rust
// from Location (ObjectClass+0x9C, leptons): direct write, no validation.
fn set_raw_coords(obj: &mut ObjectClass, coords: CoordStruct) {
    obj.location = coords;
}
```

---

## Unverified

None. All claims verified from live Ghidra in this session:
- `decompile_function 0x005f6940` — function body
- `get_function_callers 0x005f6940` — 4 callers
- `get_function_callees 0x005f6940` — no callees
