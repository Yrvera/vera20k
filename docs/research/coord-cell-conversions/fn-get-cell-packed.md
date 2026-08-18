# ObjectClass__Get_Cell_Packed — Decode Doc

## Summary

`ObjectClass__Get_Cell_Packed` (0x0041bea0) converts an object's lepton-space
X/Y coordinates (from fields at offsets 0x9C and 0xA0) into a packed 32-bit
cell index using the sign-correct arithmetic shift `(v + (v >> 31 & 0xFF)) >> 8`.
The result is `CONCAT22(cell_Y, cell_X)` — Y in the high 16 bits, X in the low.
This is the canonical lepton-to-cell-index gate (Frame #2 of the 5 CLAUDE.md
coordinate reference frames).

Function body: 0x0041bea0 – 0x0041bedc
(verified via `get_function_by_address 0x0041bea0`)

## Active in YR

**Yes.** The function is bound in 22 vtables (DATA xrefs at 0x007E245C,
0x007E4074, 0x007E489C, 0x007E8E4C, 0x007EB210, 0x007EDE78, 0x007EF218,
0x007EF58C, 0x007EFB0C, 0x007EFD54, 0x007F06C0, 0x007F34B4, 0x007F4B18,
0x007F53E4, 0x007F5E28, 0x007F64D0, 0x007F6860, 0x007F6DAC, 0x007E350C,
0x007E3C88, 0x007EC410, plus one COMPUTED_CALL in UnitClass__Constructor 0x00735876).
(verified via `get_xrefs_to 0x0041bea0`)

The function is dispatched via vtable slot 0x1B8 during UnitClass construction
(`(**(code **)(*param_1 + 0x1b8))(&piStack_4, 0)` in UnitClass__Constructor 0x00735780),
a code path exercised every time a unit is created in a normal YR skirmish.
(verified via `decompile_function 0x00735780`)

## Decompilation excerpt

```c
// verified via decompile_function 0x0041bea0
void __thiscall ObjectClass__Get_Cell_Packed(int param_1, undefined4 *param_2)
{
  undefined4 local_4;

  // from Location (leptons): offset 0x9c = lepton X, 0xa0 = lepton Y
  // sign-correct arithmetic shift: (v + (v >> 31 & 0xFF)) >> 8
  local_4 = CONCAT22(
    (short)(*(int *)(param_1 + 0xa0) + (*(int *)(param_1 + 0xa0) >> 0x1f & 0xffU) >> 8),
    (short)(*(int *)(param_1 + 0x9c) + (*(int *)(param_1 + 0x9c) >> 0x1f & 0xffU) >> 8)
  );
  *param_2 = local_4;
  return;
}
```

`param_1` is typed `int` (not `int *`), so all field offsets are direct byte
offsets per the CLAUDE.md decompilation pitfall rule.

## Behavioral analysis

### Coordinate reference frame

- **Input frame**: Frame #1 "Location" — leptons stored at `(object)+0x9C` (X)
  and `(object)+0xA0` (Y). 1 cell = 256 leptons.
- **Output frame**: Frame #2 "Get_Cell_Packed (NW cell)" — a packed 32-bit
  cell index with cell_X in bits 0–15 and cell_Y in bits 16–31.
- **Rust canonical frame**: cell-grid `(u16, u16)` with +X = east, +Y = south.
  Unpack as `x = (result & 0xFFFF) as u16`, `y = (result >> 16) as u16`.

### The sign-correct arithmetic shift

Standard C right-shift on signed integers is implementation-defined for negative
values. gamemd.exe uses the explicit floor-correction pattern:

```
cell = (v + (v >> 31 & 0xFF)) >> 8
```

For positive leptons this reduces to `v >> 8` (= `v / 256`).
For negative leptons the `v >> 31 & 0xFF` term adds 255 before shifting, which
implements floor division rather than truncation toward zero. This keeps cell
indices correct when coordinates cross the map origin.

Equivalent Rust:
```rust
fn lepton_to_cell(v: i32) -> i16 {
    ((v + ((v >> 31) & 0xFF)) >> 8) as i16
}
```

### Pack layout

`CONCAT22(cell_Y, cell_X)` in Ghidra means: a 32-bit value where cell_Y
occupies the high 16 bits and cell_X occupies the low 16 bits. Reading back:

```rust
fn unpack_cell(packed: u32) -> (u16, u16) {
    let x = (packed & 0x0000_FFFF) as u16;
    let y = (packed >> 16) as u16;
    (x, y)
}
```

### No callees

The function is leaf — no calls to other functions.
(verified via `get_function_callees 0x0041bea0` → "No callees found")

### INI keys / globals / enums

None. The function reads only `param_1+0x9C` and `param_1+0xA0` (the
Location lepton fields) and writes to the output pointer. No globals, no INI
keys, no enum comparisons.

## Struct field accesses

| Offset (bytes) | Size | Access | Semantics |
|---|---|---|---|
| `param_1 + 0x9C` | 4 (int) | read | Lepton X coordinate — from Location frame |
| `param_1 + 0xA0` | 4 (int) | read | Lepton Y coordinate — from Location frame |

Param type is `int` (direct byte offsets confirmed by Ghidra signature).
(verified via `decompile_function 0x0041bea0`)

## Vtable binding

| Vtable DATA xref | Slot byte offset from vtable base | Content |
|---|---|---|
| 0x007E245C | 0x1B8 | 0x0041bea0 (little-endian: a0 be 41 00) |

Verified: `read_memory 0x007E245C length=4` → `a0 be 41 00` = 0x0041bea0.

The vtable at 0x007E22A4 has slot 0 = AircraftClass__Destructor (0x00414290),
confirmed via `get_function_by_address 0x00414290`. Slot 0x1B8 = 0x007E22A4 +
0x1B8 = 0x007E245C → 0x0041bea0. This is the AircraftClass vtable slot binding.

The function also appears in 21 additional vtable DATA slots across the
0x007E2000–0x007F6E00 range (all ObjectClass subclass vtables), consistent
with a base class default implementation inherited unchanged by all subclasses.

UnitClass__Constructor uses `(**(code **)(*param_1 + 0x1b8))` to dispatch this
function at runtime, confirming the vtable slot 0x1B8 binding is the live
call site used during gameplay.
(verified via `decompile_function 0x00735780`)

## Callers / Lifecycle

| Caller | Address | Call type | Context |
|---|---|---|---|
| UnitClass__Constructor | 0x00735780 (call site 0x00735876) | COMPUTED_CALL via vtable+0x1B8 | Called during rally-point cell update when the unit exits via Limbo |

Only 1 direct/computed caller identified by Ghidra. The function is also
reachable via any caller that dispatches vtable slot 0x1B8 on any ObjectClass
subclass pointer, which is the primary consumer pattern across the engine.

## Out-of-scope refs

The following were observed but are not decoded here:

- `HouseClass__Set_Rally_Point_Cell` — called with the result of this function
  in UnitClass__Constructor; rally point logic is out of scope.
- `FootClass__Limbo` — gates the Get_Cell_Packed call in the constructor; limbo
  logic is out of scope.
- The 21 additional vtable slots in other subclass vtables — each subclass may
  override; only the ObjectClass base implementation is decoded here.

## Unverified claims (YELLOW)

None. All offsets, addresses, and vtable slots in this document have been
directly verified via Ghidra MCP calls cited inline above.
