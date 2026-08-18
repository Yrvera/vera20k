# ObjectClass__GetCoords (0x005f65a0) — Decode Doc

## Summary

`ObjectClass__GetCoords` at 0x005f65a0 copies three 32-bit lepton coordinate
fields (X at `this+0x9C`, Y at `this+0xA0`, Z at `this+0xA4`) into the caller-
supplied output `CoordStruct*`. It is the **real GetCoords implementation** used
by animation-related class hierarchies and is bound to vtable slot 0x48 in those
classes. It is a leaf function with no callees.

A second symbol named `ObjectClass__GetCoords` exists at 0x00410600, but its
body decompiles to a single call to `AbstractClass__Release()` — it is
**mislabeled** in Ghidra and is NOT a GetCoords implementation. The 27-vs-74
xref split reflects distinct vtable binding sets: 0x005f65a0 covers animation and
voxel-animation class vtables (19 DATA + 8 CODE xrefs); 0x00410600 covers the
majority of ObjectClass subclass vtables (74 DATA xrefs) but is almost certainly
another function incorrectly labeled by the RTTI labeler.

Function body: 0x005f65a0 – 0x005f65c0
(verified via `decompile_function 0x005f65a0`)

## Active in YR

**Yes.** Bound in 19 vtables covering animation and voxel-animation class
hierarchies. Called directly from `AnimClass__Constructor` (4 call sites) and
`VoxelAnimClass__Constructor` (2 call sites), and from
`AnimClass__GetCoords_WithOwnerOffset` (2 call sites). All of these callers
activate in normal YR gameplay whenever an animation plays.
(verified via `get_xrefs_to 0x005f65a0`)

## Decompilation excerpt

```c
// verified via decompile_function 0x005f65a0
void __thiscall ObjectClass__GetCoords(int param_1, undefined4 *param_2)
{
  // param_1 is int (direct byte offsets)
  // offsets 0x9C, 0xA0, 0xA4 = lepton X, Y, Z (Location frame)
  *param_2   = *(undefined4 *)(param_1 + 0x9c);  // X leptons
  param_2[1] = *(undefined4 *)(param_1 + 0xa0);  // Y leptons
  param_2[2] = *(undefined4 *)(param_1 + 0xa4);  // Z leptons
  return;
}
```

`param_1` is `int` — all field offsets are direct byte offsets (CLAUDE.md pitfall rule).
`param_2` is `undefined4*` — output CoordStruct with X at [0], Y at [1], Z at [2].

## Behavioral analysis

### Purpose

Copies the object's lepton-space 3D position (Location frame) into the output
CoordStruct. Identical in semantics to the standard ObjectClass GetCoords across
all subclasses: outputs (X, Y, Z) in leptons where 1 cell = 256 leptons.

### Coordinate reference frame

- **Input**: Frame #1 "Location" — `this+0x9C` (lepton X), `this+0xA0` (lepton Y),
  `this+0xA4` (lepton Z). For mobile objects: current position in leptons. For
  buildings: NW-corner cell origin in leptons.
- **Output**: CoordStruct (X at offset 0, Y at offset 4, Z at offset 8).
- **Rust canonical frame**: convert to cell-grid `(u16, u16)` via sign-correct
  arithmetic shift `(v + ((v >> 31) & 0xFF)) >> 8`.

### Z field offset: 0xA4 vs AnimTypeClass convention

This function reads Z from `this+0xA4`. This is the standard CoordStruct Z
slot (location altitude in leptons). The field at 0xA4 is one slot above the
Y field at 0xA0 in the Location layout (X=0x9C, Y=0xA0, Z=0xA4), consistent
across all ObjectClass subclasses that inherit the Location block.

### Vtable slot

Bound to vtable slot 0x48 in animation and voxel-animation class vtables.

Verification: DATA xref at 0x007e22ec (AircraftClass vtable, base 0x007e22a4).
Offset = 0x007e22ec − 0x007e22a4 = 0x48. `read_memory 0x007e22ec` → `a0 65 5f 00`
= 0x005f65a0. Confirmed.
(verified via `read_memory 0x007e22ec` and `get_xrefs_to 0x005f65a0`)

Additionally, `AnimClass__GetCoords_WithOwnerOffset` dispatches vtable slot 0x48
on the owner object: `(**(code **)(**(int **)(param_1 + 0xcc) + 0x48))(local_c)`,
confirming that 0x48 is the engine-wide GetCoords vtable slot.
(verified via `decompile_function 0x00422be0`)

### No callees

Leaf function. No internal calls.
(verified via `get_function_callees 0x005f65a0` → "No callees found")

### Contrast with 0x00410600 ("primary" label)

The task description refers to 0x00410600 as the "primary ObjectClass__GetCoords"
with 74 xrefs. Decompilation of 0x00410600 shows:

```c
void ObjectClass__GetCoords(void) {
  AbstractClass__Release();
  return;
}
```

This is **not** a GetCoords implementation. The label was applied by the RTTI
labeler or a previous rename and is wrong. Its body of `AbstractClass__Release()`
indicates it is either a destructor stub, a release/cleanup function, or an
artifact of Ghidra's function boundary detection at that address.

The 74-xref count for 0x00410600 reflects how many vtable slots point to that
address — which may mean those slots contain a different function that happens to
share the Ghidra label, or the address range contains inlined code. This requires
a separate decode (task #12 covers the primary).

**For the Rust engine**: 0x005f65a0 is the authoritative GetCoords body for the
animation class hierarchy. The three-field copy (X/Y/Z from 0x9C/0xA0/0xA4) is
the canonical implementation.
(verified via `decompile_function 0x00410600` and `decompile_function 0x005f65a0`)

### AnimClass__GetCoords_WithOwnerOffset context

The most significant CODE caller (2 sites) is `AnimClass__GetCoords_WithOwnerOffset`
(0x00422be0). It calls 0x005f65a0 to get the animation's own coordinates, then
optionally adds the owner object's coordinates (fetched via vtable slot 0x48 on
`*(int**)(param_1+0xcc)` = the animation's owner pointer) if the owner is non-null.
This produces the animation's world position as `self_coords + owner_coords`.

The other callers (AnimClass__Constructor × 4, VoxelAnimClass__Constructor × 2)
call GetCoords during object construction — likely to read back the initial
position for initialization purposes.
(verified via `decompile_function 0x00422be0` and `get_xrefs_to 0x005f65a0`)

### INI keys / globals / enums

None. Pure field read, no INI, no globals, no enum comparisons.

## Struct field accesses

| Offset (bytes) | Size | Access | Semantics |
|---|---|---|---|
| `param_1 + 0x9C` | 4 (int) | read | Lepton X — Location frame |
| `param_1 + 0xA0` | 4 (int) | read | Lepton Y — Location frame |
| `param_1 + 0xA4` | 4 (int) | read | Lepton Z — altitude in leptons |

Param type is `int` (direct byte offsets).
(verified via `decompile_function 0x005f65a0`)

## Vtable binding summary

| Vtable DATA xref | Vtable slot (byte offset from base) | Bound address | Confirmed |
|---|---|---|---|
| 0x007e22ec (AircraftClass vtable) | 0x48 | 0x005f65a0 | YES — read_memory → a0 65 5f 00 |
| 0x007e3b18 | 0x48 (computed from base) | 0x005f65a0 | YES — read_memory → a0 65 5f 00 |
| 17 additional DATA xrefs | 0x48 (same slot) | 0x005f65a0 | Not individually read — consistent with slot-0x48 pattern |

(verified via `read_memory 0x007e22ec`, `read_memory 0x007e3b18`, `get_xrefs_to 0x005f65a0`)

## Callers / Lifecycle

| Caller | Address | Call sites | Context | Sim-side? |
|---|---|---|---|---|
| `AnimClass__Constructor` | ~0x004222xx | 4 | Anim construction — reads back initial position | Render |
| `VoxelAnimClass__Constructor` | ~0x00749xx | 2 | VoxelAnim construction | Render |
| `AnimClass__GetCoords_WithOwnerOffset` | 0x00422be0 | 2 | Animation world position = self + owner | Render |

8 total CODE xrefs plus 19 DATA (vtable) xrefs.
(verified via `get_xrefs_to 0x005f65a0`)

## Out-of-scope refs

- `AnimClass__GetCoords_WithOwnerOffset` (0x00422be0) internals — animation
  coordinate compositing; out of scope beyond the call-site analysis above
- `AnimClass__Constructor` and `VoxelAnimClass__Constructor` — construction logic;
  out of scope
- 0x00410600 ("ObjectClass__GetCoords" mislabel) — not a GetCoords; needs separate
  decode as part of task #12

## Unverified claims (YELLOW)

**UNVERIFIED**: The identity of the 17 vtable DATA xrefs beyond the two spot-
checked (0x007e22ec and 0x007e3b18). All 19 DATA xrefs are assumed to be at
vtable slot 0x48 based on the pattern, but only 2 were individually verified via
`read_memory`. The class names for each vtable (AircraftClass, AnimClass,
VoxelAnimClass, etc.) are inferred from the xref address ranges matching known
vtable regions — not confirmed by reading the vtable slot-0 destructor of each.

**UNVERIFIED**: Whether 0x00410600 is truly mislabeled vs. being a small inline
thunk that has legitimate GetCoords behavior obscured by Ghidra's decompilation.
The decompiled body showing only `AbstractClass__Release()` is consistent with a
wrong label, but a disassembly-level inspection was not performed in this session.
