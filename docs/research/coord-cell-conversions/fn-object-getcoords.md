# ObjectClass::GetCoords — decode doc

**Address:** `0x005f65a0`
**Kind:** function-decode-v1
**Source:** decompile_function @ 0x005f65a0

---

## Summary

`ObjectClass::GetCoords` (0x005f65a0) copies the three `int32` Location fields at
ObjectClass+0x9C, +0xA0, and +0xA4 into a caller-supplied CoordStruct output buffer.
It is the base-class implementation of the `GetCoords` vtable slot (vtable+0x48). No
arithmetic, no branching — a pure 3-word copy from the object's Location field. Subclasses
(notably `BuildingClass`) override this slot to apply geometry adjustments; for non-building
objects (bullets, infantry, vehicles, overlays, terrain, animations) this base implementation
is used directly.

---

## Active in YR

**YES — used throughout normal YR skirmish play via vtable dispatch.**

Direct callers (3):
- `AnimClass__Constructor` @ `0x00421ea0` — anim spawn position queries
- `AnimClass__GetCoords_WithOwnerOffset` @ `0x00422be0` — anim coord + owner offset
- `VoxelAnimClass__Constructor` @ `0x007493b0` — voxel anim spawn coord

The low direct-caller count is expected: most consumers call via vtable+0x48 dispatch
(`(**(code **)(*param + 0x48))(buf)`) and those calls do NOT appear as direct callers of
`0x005f65a0`. The vtable binding is confirmed: vtable+0x48 in `vtable__ObjectClass`
resolves to `0x005f65a0`.

(verified via `get_function_callers 0x005f65a0`, `decompile_function 0x00422be0`,
`read_memory 0x007ef060 length=200` showing slot at byte-offset 0x48)

---

## Vtable binding verification

`vtable__ObjectClass` is at `0x007ef060` (verified via `list_globals` filter=vtable__ObjectClass).

Reading 200 bytes from `0x007ef060`, byte offset 0x48 (4-byte slot, little-endian):
`a0 65 5f 00` = `0x005f65a0` = this function.

`AnimClass__GetCoords_WithOwnerOffset` shows the dispatch pattern explicitly:
```c
(**(code **)(**(int **)(param_1 + 0xcc) + 0x48))(local_c)
```
— vtable+0x48 invoked on an owner object to retrieve its coords.
(verified via `decompile_function 0x00422be0`, `read_memory 0x007ef060`)

---

## Signature

```c
// verified via decompile_function 0x005f65a0
void __thiscall ObjectClass__GetCoords(int param_1, undefined4 *param_2)
```

- `param_1` — `int` (direct byte offsets, NOT pointer-scaled). `this` pointer to ObjectClass.
- `param_2` — output `CoordStruct*`, a caller-provided 3-int buffer. Filled with X, Y, Z.

**Calling convention:** `__thiscall` — `param_1` is `this` in ECX; `param_2` pushed on stack.

**Return value:** void. Result is written through `param_2`. Callers read from `param_2`
after the call; the function itself does not return the pointer.

---

## Control Flow

```c
// verified via decompile_function 0x005f65a0
void __thiscall ObjectClass__GetCoords(int param_1, undefined4 *param_2) {
    *param_2    = *(undefined4 *)(param_1 + 0x9c);  // X leptons
    param_2[1]  = *(undefined4 *)(param_1 + 0xa0);  // Y leptons
    param_2[2]  = *(undefined4 *)(param_1 + 0xa4);  // Z leptons
    return;
}
```

No branches, no guards, no validation. Unconditional 3-word read-and-copy.

**Note on param_1 type:** `param_1` is `int` — so `param_1 + 0x9c` is a **direct byte
offset** of 0x9C. This is the Location frame (CLAUDE.md frame #1). Verified: `int` type
means byte offsets, not × 4.

---

## Struct Field Accesses

| Byte offset from `param_1` | Type | Field | Semantics |
|---|---|---|---|
| `+0x9C` | `int32` | Location.X | Lepton X coordinate |
| `+0xA0` | `int32` | Location.Y | Lepton Y coordinate |
| `+0xA4` | `int32` | Location.Z | Lepton Z coordinate (height) |

These are the **Location frame** fields (CLAUDE.md frame #1):
- For buildings: NW-corner cell in leptons (see `BuildingClass::GetCoords` for center correction).
- For mobile units: body center in leptons.
- 1 cell = 256 leptons; sign-correct arithmetic applies when converting to cell index.

(verified via `decompile_function 0x005f65a0`)

---

## Reference Frame

**Location frame (CLAUDE.md frame #1):** `ObjectClass + 0x9C` / `+0xA0` / `+0xA4`.

- For buildings: NW-corner cell origin in leptons. Verified via `BuildingClass::GetCoords`
  (0x00447AC0) which adds `((w-1)*128, (h-1)*128)` to compute foundation center.
- For mobile objects (infantry, vehicles, bullets, anims): body center in leptons.
- Output frame of this function: **Location frame** — same reference, just copied out.

**This is NOT the same as GetCoords (foundation center) frame for buildings.** Use
`BuildingClass::GetCoords` (vtable override) when calling through the vtable on a building.
`ObjectClass::GetCoords` is the base impl that buildings do NOT use at vtable+0x48.

---

## Globals

None. The function reads no global state — only `this` fields.
(verified via `decompile_function 0x005f65a0`)

---

## INI Keys

None.

---

## Enum Values

None.

---

## Callees

None. (verified: function body has no call instructions)

---

## Observable vs Internal

**Observable:** Any consumer that uses the returned coords to position a unit on screen,
determine firing range, or update locomotor state will produce player-visible effects.
Wrong X/Y values displace the unit or shift splash damage radii.

**Internal:** The field copy itself is not observable — only its downstream consumers
(locomotors, combat, rendering) make it visible.

---

## Caller Pattern Analysis

### Pattern A — Anim spawn coord + owner offset

```c
// from AnimClass__GetCoords_WithOwnerOffset @ 0x00422be0
// verified via decompile_function 0x00422be0
if (param_1[0xcc] != 0) {
    piVar5 = (int *)ObjectClass__GetCoords(local_18);         // this object's coords
    piVar6 = (int *)(**(code **)(**(int **)(param_1 + 0xcc) + 0x48))(local_c); // owner coords via vtable
    *unaff_retaddr    = *piVar6 + *piVar5;   // add offset
    unaff_retaddr[1]  = piVar6[1] + piVar5[1];
    unaff_retaddr[2]  = piVar5[2] + piVar6[2];
    return;
}
// else: just copy this object's coords
puVar7 = (undefined4 *)ObjectClass__GetCoords(local_c);
*param_2 = *puVar7; param_2[1] = puVar7[1]; param_2[2] = puVar7[2];
```

`ObjectClass::GetCoords` called on `this` (the anim object itself). Owner's coords are
retrieved via vtable+0x48 dispatch on a different object. This shows the function is used
both directly (for the local object) and indirectly (for referenced objects).

### Pattern B — VoxelAnim constructor spawn point

`VoxelAnimClass__Constructor` @ `0x007493b0` calls `ObjectClass__GetCoords` directly
during construction to read the initial spawn coord — same pattern as anim constructor.

---

## Note: mislabeled function at 0x00410600

`search_functions` returns two matches for "ObjectClass__GetCoords":
- `0x005f65a0` — the real implementation (this decode)
- `0x00410600` — a mislabeled function; decompilation shows it calls `AbstractClass__Release()`
  and returns. This is NOT a GetCoords implementation. It has no callers. The label is wrong.

(verified via `decompile_function 0x00410600`)

---

## AbstractClass::GetCoords @ 0x004104c0

```c
// verified via decompile_function 0x004104c0
void AbstractClass__GetCoords(undefined4 *param_1) {
    *param_1    = DAT_00887680;
    param_1[1]  = DAT_00887684;
    param_1[2]  = DAT_00887688;
    return;
}
```

`AbstractClass::GetCoords` returns three fixed global values — this is a null/fallback
implementation that returns a sentinel coordinate. `ObjectClass::GetCoords` at `0x005f65a0`
overrides it with the real Location-field read. No callers found for
`AbstractClass::GetCoords` directly — it exists as the root of the override chain.

The globals `DAT_00887680 / 84 / 88` are likely the null-coord sentinel
(`DAT_0089a178/7c/80` referenced elsewhere, or a separate set). Out of scope for this decode.

---

## Rust Equivalent

```rust
// from Location (ObjectClass+0x9C, leptons): X/Y/Z as i32.
// Reference frame: Location frame — NW corner for buildings, body center for mobile units.
fn get_coords(location: &CoordStruct) -> CoordStruct {
    *location  // direct copy; no arithmetic
}
```

In Rust the Location field would be a `CoordStruct` member of the entity struct.
`GetCoords` for base objects is a trivial copy. `BuildingClass` overrides this by adding
`((w-1)*128, (h-1)*128)` to center on the foundation.

---

## Out-of-scope refs

| Symbol | Address | Reason deferred |
|---|---|---|
| `BuildingClass::GetCoords` | `0x00447AC0` | Override for buildings; adds `(w-1)*128, (h-1)*128`. Task #15 decode-fn-building-getcoords. |
| `AnimClass::GetCoords_WithOwnerOffset` | `0x00422be0` | Anim-specific override adding owner offset. |
| `DAT_00887680/84/88` | globals | Null/fallback CoordStruct sentinel used by AbstractClass::GetCoords. |
| vtable+0x48 dispatcher pattern | — | All callers that use vtable dispatch to call this; not individually traced. |

---

## Unverified

None. All claims verified from live Ghidra decompilation in this session:
- `decompile_function 0x005f65a0` — main function body
- `decompile_function 0x00410600` — confirmed mislabel (calls AbstractClass__Release)
- `decompile_function 0x004104c0` — AbstractClass::GetCoords fallback
- `decompile_function 0x00422be0` — AnimClass::GetCoords_WithOwnerOffset (caller pattern A)
- `get_function_callers 0x005f65a0` — 3 direct callers
- `list_globals` filter=vtable__ObjectClass — vtable base address 0x007ef060
- `read_memory 0x007ef060 length=200` — confirmed vtable+0x48 = 0x005f65a0
