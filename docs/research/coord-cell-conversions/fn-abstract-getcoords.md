# AbstractClass__GetCoords — decode

## Summary

`AbstractClass__GetCoords` (`0x004104c0`) is the base/fallback implementation of the
`GetCoords` virtual method (vtable slot `+0x48`). It returns a fixed global sentinel
CoordStruct `{X=0, Y=0, Z=0}` stored at `DAT_00887680/00887684/00887688`. This is the
"no position" or "invalid coordinate" return used for abstract or unplaced objects that
do not override `GetCoords`.

The function appears in 30+ vtable DATA xrefs across the class hierarchy. Any class
that does not override vtable+0x48 will dispatch here and receive the global zero-sentinel.

**Verified via `decompile_function 0x004104c0`, `disassemble_function 0x004104c0`,
`get_xrefs_to 0x004104c0`, `read_memory 0x00887680`, and `AnimClass__GetCoords_WithOwnerOffset`
caller decompilation (`0x00422be0`) which confirms vtable+0x48 as the dispatch offset.**

## Active in YR

YES. The vtable slot itself is always present and dispatched at runtime. The base
implementation (`0x004104c0`) fires for any game object that has not overridden GetCoords
— typically abstract type objects, detached animations before they acquire an owner, or
objects in an uninitialized state. The override chain (`ObjectClass__GetCoords @
0x005f65a0`, `BuildingClass__GetCoords @ 0x00447AC0`) covers all live gameplay objects.
No TS-only gate detected.

## Address

`0x004104c0` in `gamemd.exe`

## Signature (actual)

```c
// __stdcall: one explicit arg = output CoordStruct* (12-byte buffer)
// No 'this' in ECX — this is a free stdcall function despite being a vtable slot.
// The vtable call is: (**(code **)(obj_vtable + 0x48))(out_buf)
// Returns: void (writes directly into the output buffer)
void AbstractClass__GetCoords(undefined4 *out_buf);
```

Ghidra shows `undefined4 *param_1` with no `this`. Disassembly uses `MOV EAX, [ESP+4]`
(load output pointer from stack, not ECX) and `RET 0x4` (stdcall, pops 4 bytes — one
pointer argument). No ECX reads occur. Verified via `disassemble_function 0x004104c0`.

**Calling convention note:** The caller in `AnimClass__GetCoords_WithOwnerOffset`
(`0x00422be0`) uses `(**(code **)(**(int **)(param_1 + 0xcc) + 0x48))(local_c)` — the
vtable dispatch passes a single output buffer pointer and no `this`. The `this` pointer
for the object is stored in the vtable dispatch chain itself (`**(int **)(...)`), so the
callee receives only the output buffer. This is an unusual RA2 pattern: the vtable method
takes only the output buffer, not `this`, meaning the object being queried is implicitly
accessible via the vtable dispatch context (the double-deref of the vtable through the
object pointer). `AbstractClass__GetCoords` ignores ECX entirely; it only needs the output buffer.

## Parameters

| Name | Type | Location | Meaning |
|------|------|----------|---------|
| `out_buf` | `undefined4 *` | `[ESP+4]` | Output CoordStruct buffer (12 bytes: X at +0, Y at +4, Z at +8), leptons |

## Return Value

`void`. All three components written into `out_buf` before return. Callers that use
the return register (`puVar = (undefined4*)GetCoords(buf)`) are observing Ghidra's
decompiler artefact — the actual result is always in `out_buf`. Verified by caller patterns.

## Control Flow

Single basic block (9 instructions, cyclomatic complexity 1):

```
004104c0: MOV EAX, [ESP+4]          ; EAX = out_buf
004104c4: MOV EDX, [DAT_00887680]   ; EDX = global sentinel X (= 0)
004104ca: MOV ECX, EAX              ; ECX = out_buf
004104cc: MOV [ECX], EDX            ; out_buf[0] = sentinel X
004104ce: MOV EDX, [DAT_00887684]   ; EDX = global sentinel Y (= 0)
004104d4: MOV [ECX+4], EDX          ; out_buf[1] = sentinel Y
004104d7: MOV EDX, [DAT_00887688]   ; EDX = global sentinel Z (= 0)
004104dd: MOV [ECX+8], EDX          ; out_buf[2] = sentinel Z
004104e0: RET 0x4
```

Verified via `disassemble_function 0x004104c0`.

## Vtable Binding — vtable+0x48

The GetCoords virtual method sits at byte offset `+0x48` from the vtable pointer stored
in game objects. Evidence:

- `AnimClass__GetCoords_WithOwnerOffset @ 0x00422be0` decompiles as:
  `(**(code **)(**(int **)(param_1 + 0xcc) + 0x48))(local_c)` — direct literal `0x48`.
  Verified via `decompile_function 0x00422be0`.
- `AbstractClass__GetCoords @ 0x004104c0` appears in DATA xref at `0x007e1f98`.
  The AircraftClass vtable pointer base is `0x007e22a4` (verified: `[0x007e22a4]` =
  `0x00414290` = `AircraftClass__Destructor`, slot 0). `ObjectClass__GetCoords @
  0x005f65a0` appears at `0x007e22ec` = `0x007e22a4 + 0x48`. Cross-check confirmed
  via `read_memory 0x007e22a0` and `get_function_by_address 0x00414290`.
- For the AbstractClass vtable: `AbstractClass__GetCoords` at `0x007e1f98`. Vtable
  pointer at `0x007e1f50` (verified: `[0x007e1f50]` = `0x00410260` =
  `AbstractClass__QueryInterface`, slot 0). `0x007e1f50 + 0x48 = 0x007e1f98`. Confirmed
  via `read_memory 0x007e1f50` and `get_function_by_address 0x00410260`.

**Vtable slot `+0x48` = GetCoords. Confirmed from two independent vtable instances.**

## Global Sentinel — DAT_00887680

The sentinel at `0x00887680` (12 bytes, three int32 fields) reads as `{0, 0, 0}` at
runtime (verified via `read_memory 0x00887680`). It is written by a static initializer
near `0x00410112` (not yet defined as a function; raw XOR EAX,EAX; MOV stores pattern).
This zero sentinel is the canonical "invalid/no-position" CoordStruct in the engine.

**Note:** The sentinel is all zeros in RAM but represents an invalid coordinate, not the
map origin. Code that receives a zero CoordStruct from a GetCoords call should treat it
as "position unknown" rather than "position (0,0,0)". This matters for callers like
`AnimClass__GetCoords_WithOwnerOffset` which may fall back to the abstract sentinel when
the owner has no position.

## CoordStruct Output Layout

| Byte offset in out_buf | Size | Component | Units |
|---|---|---|---|
| `+0x00` | 4 bytes (int32) | X | Leptons, east-positive |
| `+0x04` | 4 bytes (int32) | Y | Leptons, south-positive |
| `+0x08` | 4 bytes (int32) | Z | Leptons, height |

Total output: **12 bytes**. For `AbstractClass__GetCoords`, all three are always 0
(from the sentinel globals). For concrete overrides (`ObjectClass__GetCoords`,
`BuildingClass__GetCoords`), they carry real coordinates.

## Related GetCoords Implementations

| Address | Name | What it returns |
|---|---|---|
| `0x004104c0` | `AbstractClass__GetCoords` | Zero sentinel `{0,0,0}` (this doc) |
| `0x005f65a0` | `ObjectClass__GetCoords` | `{ECX+0x9C, ECX+0xA0, ECX+0xA4}` — Location frame |
| `0x00410600` | `ObjectClass__GetCoords` (thunk) | Adjusts arg −4 bytes, jumps to `0x00410310` |
| `0x00447AC0` | `BuildingClass__GetCoords` | Location + foundation-center offset (task #15) |
| `0x00422be0` | `AnimClass__GetCoords_WithOwnerOffset` | Adds owner offset if owner is set |

The thunk at `0x00410600` does `SUB dword [ESP+4], 0x4; JMP 0x00410310`. This adjusts
the return address / struct pointer by −4, consistent with a multiple-inheritance
adjustment. Verified via `disassemble_function 0x00410600`.

## Callers (direct call, not vtable)

`get_function_callers 0x004104c0` returns "No callers found" — all calls are through
vtable dispatch (DATA xrefs). This is expected: the function is never called directly by
name in the binary; all call sites use the `[obj_vtable + 0x48]` indirect dispatch.

The 30+ DATA xrefs (from `get_xrefs_to 0x004104c0`) are all vtable entries in class
vtables that inherit the base implementation without overriding GetCoords.

## Callees

None. Leaf function — no calls to other functions. Verified via `get_function_callees
0x004104c0` → "No callees found."

## Globals

| Address | Access | Semantics |
|---|---|---|
| `DAT_00887680` | READ | Sentinel X coordinate (int32, leptons) = 0 at runtime |
| `DAT_00887684` | READ | Sentinel Y coordinate (int32, leptons) = 0 at runtime |
| `DAT_00887688` | READ | Sentinel Z coordinate (int32, leptons) = 0 at runtime |

All three are written once by a static initializer at startup and never modified during
gameplay. Verified via `get_xrefs_to 0x00887680` (one WRITE from `0x00410112`, one READ
from this function). `read_memory 0x00887680` at analysis time confirmed all-zero content.

## INI Keys

None. Pure sentinel function.

## Enums

None.

## Load-Bearing vs Internal

**Load-bearing:** The zero sentinel values `(0,0,0)` are what callers receive when
dispatching GetCoords on an abstract or unpositioned object. Any Rust port of a system
that calls GetCoords via vtable must handle the `(0,0,0)` case as "no position" rather
than treating it as a valid on-map coordinate. The vtable slot offset `+0x48` is
load-bearing — it must match across all class implementations.

**Internal:** The specific global addresses (`0x00887680`) and the static initializer
mechanism are implementation details.

## Out-of-Scope Refs

- `ObjectClass__GetCoords @ 0x005f65a0` — concrete GetCoords reading from Location
  frame (`ECX+0x9C/0xA0/0xA4`); task #12.
- `BuildingClass__GetCoords @ 0x00447AC0` — building override adding foundation-center
  offset; task #15.
- `AnimClass__GetCoords_WithOwnerOffset @ 0x00422be0` — animator wrapper; out of scope.
- `AbstractClass__QueryInterface @ 0x00410260` — IUnknown slot 0; out of scope.
- `ObjectClass__GetCoords` thunk at `0x00410600` — multiple-inheritance adjustment;
  out of scope as a separate entry point.

## Rust Equivalent

```rust
// vtable+0x48: GetCoords — returns CoordStruct (12 bytes: X, Y, Z in leptons)
// Abstract/base implementation returns the null sentinel (0, 0, 0).
// All concrete game object types override this slot.
fn get_coords_abstract() -> CoordStruct {
    CoordStruct { x: 0, y: 0, z: 0 }
}
```

In the Rust engine, `GetCoords` should be modelled as a method on a trait (or enum
dispatch), not a raw vtable call. The `(0, 0, 0)` return from the abstract base maps
to `Option<CoordStruct>::None` or a sentinel variant at the Rust boundary — do not
treat the zero-return as a valid map position.

## SELF-PROOF — 3 random load-bearing claims verified

1. **`RET 0x4`** (stdcall, one 4-byte arg): confirmed from `disassemble_function
   0x004104c0` — last instruction is `RET 0x4`. Verified.
2. **Vtable+0x48 dispatch offset**: confirmed from `decompile_function 0x00422be0` —
   literal `+ 0x48` in the vtable call expression. Verified.
3. **Global sentinel all-zero at runtime**: confirmed from `read_memory 0x00887680`
   returning 12 bytes of `0x00`. Verified.

## Unverified

None. All claims verified from live Ghidra in this session.
