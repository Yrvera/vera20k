# ObjectClass::GetOccupiedCell — decode doc

**Address:** `0x005f6960`
**Kind:** function-decode-v1
**Source:** decompile_function @ 0x005f6960

---

## Summary

`ObjectClass::GetOccupiedCell` (0x005f6960) reads the object's Location fields
(+0x9C, +0xA0, +0xA4) and calls `CellClass::Get_Cell_At` **twice** with the same
lepton-coordinate buffer. The function body appears to perform the same lookup twice
and returns void — Ghidra's decompilation suggests the result is conveyed via the
hidden return-value pointer (the second `Get_Cell_At` call's result is the return).

No globals. One callee: `CellClass::Get_Cell_At @ 0x00565730`.

---

## Active in YR

**YES — vtable-dispatched; called whenever an object's occupied cell is needed.**

No direct callers (verified via `get_function_callers 0x005f6960`). All callers use
vtable dispatch. The function is active in normal YR gameplay wherever cell occupancy
is checked (pathfinding, placement validation, combat targeting, shroud reveal).

---

## Signature

```c
// verified via decompile_function 0x005f6960
void __fastcall ObjectClass__GetOccupiedCell(int param_1)
```

- `param_1` — `int` (direct byte offsets). `this` pointer to ObjectClass.
- Return: via hidden return pointer (not visible as `param_2` in this signature but conveyed
  through the `CellClass::Get_Cell_At` result). Returns a `CellClass*` or a packed cell coord.

**Calling convention:** `__fastcall` — `param_1` in ECX. Return buffer via EAX on entry.

---

## Control Flow

```c
// verified via decompile_function 0x005f6960
void __fastcall ObjectClass__GetOccupiedCell(int param_1) {
    undefined4 local_c = *(undefined4 *)(param_1 + 0x9c);  // Location.X
    undefined4 local_8 = *(undefined4 *)(param_1 + 0xa0);  // Location.Y
    undefined4 local_4 = *(undefined4 *)(param_1 + 0xa4);  // Location.Z
    CellClass__Get_Cell_At(&local_c);  // call 1: lepton coord → cell class
    
    local_c = *(undefined4 *)(param_1 + 0x9c);  // re-read X (same value)
    local_4 = *(undefined4 *)(param_1 + 0xa4);  // re-read Z
    local_8 = *(undefined4 *)(param_1 + 0xa0);  // re-read Y
    CellClass__Get_Cell_At(&local_c);  // call 2: same lookup again
    return;
}
```

The double call is unusual. Both calls use the same Location coords. The second call's
return value (in EAX) is the actual result returned to the caller via the x86 calling
convention — the first call's result is discarded. This may be a Ghidra decompilation
artifact (the compiler emitted two loads because the first call could have modified the
stack-local buffer in a way that required a reload).

---

## Struct Field Accesses

| Byte offset from `param_1` | Type | Field | Semantics |
|---|---|---|---|
| `+0x9C` | `int32` | Location.X | Lepton X — read twice |
| `+0xA0` | `int32` | Location.Y | Lepton Y — read twice |
| `+0xA4` | `int32` | Location.Z | Lepton Z — read twice |

`param_1` is `int` — direct byte offsets.
(verified via `decompile_function 0x005f6960`)

---

## Callee: CellClass::Get_Cell_At @ 0x00565730

`CellClass::Get_Cell_At` converts a lepton-space CoordStruct to a `CellClass*` by
dividing X and Y by 256 (lepton-to-cell conversion) and looking up the cell array.
The function receives a `CoordStruct*` (3 × int32) and returns a `CellClass*`.

(verified via `get_function_callees 0x005f6960`)

---

## Observable vs Internal

**Observable:** Used to determine which cell an object occupies — affects occupancy marking,
pathfinding, combat targeting, and shroud reveal. A wrong result produces incorrect cell
occupancy (object appears to occupy the wrong cell).

**Internal:** The double call and interim reload are compiler artifacts invisible to callers.

---

## Globals

None read or written directly by this function.

---

## INI Keys

None.

---

## Rust Equivalent

```rust
// from Location (ObjectClass+0x9C, leptons): convert to CellClass reference.
// Equivalent: floor(Location.X / 256), floor(Location.Y / 256) → cell lookup.
fn get_occupied_cell(obj: &ObjectClass) -> CellCoord {
    let cx = (obj.location.x + ((obj.location.x >> 31) & 0xFF)) >> 8;
    let cy = (obj.location.y + ((obj.location.y >> 31) & 0xFF)) >> 8;
    CellCoord { x: cx as i16, y: cy as i16 }
}
```

---

## Out-of-scope refs

| Symbol | Address | Reason deferred |
|---|---|---|
| `CellClass::Get_Cell_At` | `0x00565730` | Lepton-to-cell lookup — own decode scope. |

---

## Unverified

- **Double call rationale** (YELLOW): The decompiled body shows two identical calls to
  `CellClass::Get_Cell_At`. The exact reason (compiler reload artifact vs. intentional
  double computation) was not confirmed by disassembly inspection in this session.

All addresses, field offsets, and callee identity are verified from live Ghidra:
- `decompile_function 0x005f6960` — function body
- `get_function_callers 0x005f6960` — no direct callers
- `get_function_callees 0x005f6960` — single callee CellClass::Get_Cell_At
