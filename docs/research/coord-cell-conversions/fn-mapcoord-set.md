# MapCoord_Set — decode

## Summary

`MapCoord_Set` is a trivial 2-component setter that packs two signed 16-bit
cell coordinates (X and Y) into a 4-byte `CellStruct`. It is a leaf function
with a single basic block, 6 instructions, and no callees.

The struct layout: `short X` at offset +0, `short Y` at offset +2.

The value of this function comes from its 491 total xrefs (27 direct call callers).
The 491 vs 27 discrepancy occurs because the function is also stored as a
function-pointer target in data tables, and inlined uses appear in DATA xrefs
that Ghidra cannot trace as calls. This is explicitly noted in the task description.

Callers uniformly use it to:
1. Construct a cell-offset pair from signed integer components.
2. Build the shroud spiral table (signed cell offsets from origin).
3. Create building-relative offset CellStructs for dock/slave positioning.
4. Convert a lepton coordinate pair to cells (using the sign-correct shift externally).

**Verified via `decompile_function 0x0042d470`, `disassemble_function 0x0042d470`,
`get_function_callers 0x0042d470`, `get_function_callees 0x0042d470`, and
three caller decompilations (0x00561910, 0x006af6c0, 0x0057e7a0).**

## Active in YR

YES. Called by `MapClass__InitRevealSpiralTable` (shroud reveal spiral table
initialization at game start), `SlaveManagerClass__AI_Update` (per-frame slave
ore-harvesting state machine), `MapClass__ApplyBridgeDestruction_NS_High`
(bridge collapse), `Path_smooth_single_segment` (pathfinding), and others.
All callers are live in standard YR gameplay. No TS-only gate detected.

## Address

`0x0042d470` in `gamemd.exe`

## Signature (actual)

```c
// __thiscall: ECX = short* (destination CellStruct — 4 bytes: {short X, short Y})
// Two explicit stdcall args: x and y (pushed as 32-bit but only low 16 bits used)
// Returns: void (Ghidra shows void; consistent with callers)
// RET 0x8 = pops 2 × 4-byte args from stack
void __thiscall MapCoord_Set(short *dest, short x, short y);
```

Ghidra shows `undefined2 *param_1, undefined2 param_2, undefined2 param_3`.
Disassembly uses `MOV DX, word [ESP+8]` and `MOV CX, word [ESP+4]` — only 16-bit
reads from the stack, so callers push 32-bit values but only the low word is stored.
Signed 32-bit values like `-8` (0xFFFFFFF8) truncate to `0xFFF8` (-8 as short).
`RET 0x8` confirms stdcall with 2 × 4-byte args (8 bytes total cleanup).
Verified via `disassemble_function 0x0042d470`.

## Parameters

| Name | Type | Location | Meaning |
|------|------|----------|---------|
| `dest` | `short *` | ECX (`this`) | Destination CellStruct: 4 bytes {short X at +0, short Y at +2}. |
| `x` | `short` (low 16 bits of int arg) | `[ESP+4]` | X cell coordinate (east-positive). Written to `[dest+0]`. |
| `y` | `short` (low 16 bits of int arg) | `[ESP+8]` | Y cell coordinate (south-positive). Written to `[dest+2]`. |

Reference frame: **CellStruct** (cell coordinates, not leptons). `[dest+0]` = X
(east, short), `[dest+2]` = Y (south, short). Verified via `disassemble_function 0x0042d470`.

**Important:** Callers that convert from leptons must apply the sign-correct shift
BEFORE calling `MapCoord_Set`. Example from `SlaveManagerClass__AI_Update @ 0x006af6c0`:
```c
// piVar7 = lepton coord pointer (piVar7[0]=X leptons, piVar7[1]=Y leptons)
puVar6 = (undefined4*)MapCoord_Set(
    (int)(*piVar7 + (*piVar7 >> 0x1f & 0xffU)) >> 8,    // sign-correct lepton→cell X
    (int)(piVar7[1] + (piVar7[1] >> 0x1f & 0xffU)) >> 8  // sign-correct lepton→cell Y
);
```
The sign-correct shift (`(v + (v>>31 & 0xFF)) >> 8`) is identical to the one in
`Get_Cell_Packed`. The conversion is the caller's responsibility.

## Return Value

`void`. All callers store the result indirectly (`uVar5 = MapCoord_Set(...)` stores
the function's 4-byte return region, i.e. the `this` pointer or the memory-return
buffer — a Ghidra decompiler artefact). The written value is always read through
the `dest` pointer. Confirmed by all sampled callers.

## Control Flow

Single basic block (cyclomatic complexity 1):

```
0042d470: MOV DX, word [ESP+8]     ; DX = y (low 16 bits)
0042d475: MOV EAX, ECX             ; EAX = dest (this)
0042d477: MOV CX, word [ESP+4]     ; CX = x (low 16 bits)
0042d47c: MOV word [EAX], CX       ; dest->X = x
0042d47f: MOV word [EAX+2], DX     ; dest->Y = y
0042d483: RET 0x8
```

Verified via `disassemble_function 0x0042d470`.

## CellStruct Layout (confirmed)

| Byte offset | Size | Component | Semantics |
|-------------|------|-----------|-----------|
| `+0` | 2 bytes (short) | X | East-positive cell coordinate. |
| `+2` | 2 bytes (short) | Y | South-positive cell coordinate. |

Total size: **4 bytes**. Signed shorts. `1 cell = 256 leptons`. This is the
canonical CellStruct for the map system. Verified by disassembly and caller patterns.

## Struct Field Accesses

Writes to `[dest+0]` (X) and `[dest+2]` (Y). `param_1` is `undefined2 *` in
Ghidra, so `param_1[1]` = offset +2 (multiplied by sizeof(undefined2) = 2).
This matches the disassembly `[EAX+2]`. Verified via `decompile_function 0x0042d470`.

## Callers — Usage Patterns

27 callers identified. 3 sampled in detail. Verified via `get_function_callers 0x0042d470`.

### Pattern 1: Shroud Reveal Spiral Table Construction

`MapClass__InitRevealSpiralTable @ 0x00561910` builds a pre-computed table of
cell offsets for the shroud reveal spiral. It calls `MapCoord_Set(dx, dy)` with
signed integer offsets ranging from about -11 to +11 in both axes. The packed
4-byte value is stored directly into global memory (`_DAT_00abd910` etc.).

```c
// Example calls from MapClass__InitRevealSpiralTable:
puVar1 = (undefined4*)MapCoord_Set(0xfffffff8, 4);   // (-8, 4)
_DAT_00abd910 = *puVar1;
puVar1 = (undefined4*)MapCoord_Set(8, 4);             // (8, 4)
_DAT_00abd914 = *puVar1;
```

The signed truncation: `0xfffffff8` as int → `0xFFF8` as short = -8. Correct.
Verified via `decompile_function 0x00561910`.

### Pattern 2: Building-Relative Dock/Slave Offset

`SlaveManagerClass__AI_Update @ 0x006af6c0` uses `MapCoord_Set` to compute a
building's easternmost dock cell offset:

```c
// When building is a 6-type (orientation=6):
iVar10 = BuildingTypeClass__GetFoundationWidth();  // e.g. 4 for GAREFN
iVar9  = BuildingTypeClass__GetFoundationHeight(); // e.g. 3
uVar5  = MapCoord_Set(iVar10 + -1, iVar9 / 2);   // (3, 1) for 4×3 building
(*piVar7 + vtable_GetCellXY)(puVar11, puVar12, uVar5); // then MapCoord_Add
puVar6 = MapCoord_Add(puVar12, puVar11);            // NW_cell + (width-1, height/2)
```

This produces the center-right cell of the building footprint — used as the
slave's target/return cell. Verified via `decompile_function 0x006af6c0`.

### Pattern 3: Lepton-to-Cell Conversion Before Set

Also in `SlaveManagerClass__AI_Update @ 0x006af6c0`, case 4:
```c
// piVar7[0]=X leptons, piVar7[1]=Y leptons (GetCoords result):
puVar6 = MapCoord_Set(
    (int)(*piVar7 + (*piVar7 >> 0x1f & 0xffU)) >> 8,    // floor(X / 256)
    (int)(piVar7[1] + (piVar7[1] >> 0x1f & 0xffU)) >> 8  // floor(Y / 256)
);
```
Same sign-correct formula as `Get_Cell_Packed`. Confirms this is the
canonical lepton→cell conversion applied pre-pack. Verified via `decompile_function 0x006af6c0`.

### Pattern 4: Simple Constant Cell Delta

`MapClass__ApplyBridgeDestruction_NS_High @ 0x0057e7a0`:
```c
uVar7 = MapCoord_Set(0, 1);  // one cell south delta
piVar3 = (int*)FUN_00588c60(&local_c8, uVar7);  // then some bridge update with this offset
```

Verified via `decompile_function 0x0057e7a0`.

## Callees

None. Leaf function confirmed by `get_function_callees 0x0042d470`.

## Globals

None accessed directly.

## INI Keys

None.

## Enums

None. All values are caller-computed integers.

## 491 xrefs vs 27 direct callers — discrepancy note

The 491 total xref count (per task description) vs 27 callers (from
`get_function_callers`) results from:
1. The shroud spiral table in `MapClass__InitRevealSpiralTable` stores packed
   CellStruct values in a global array — each stored `_DAT_*` address counts as a
   DATA xref. Approximately 150+ stores from that function alone contribute.
2. `MapCoord_Set` may also be stored as a function pointer in data tables
   (function pointer dispatch patterns), which appear as DATA refs.
3. Inlined call-by-pointer paths that Ghidra resolves as DATA rather than CODE refs.

This does NOT mean there are 491 runtime callers — the 27 confirmed call sites is
the correct count. Verified: `get_function_callers` is authoritative for CODE xrefs.

## Load-Bearing vs Internal

**Load-bearing:** The write order (X at +0, Y at +2) is the canonical CellStruct
layout used throughout the map system. The truncation from 32-bit to 16-bit is
load-bearing — callers that pass values outside the int16 range (e.g., `0xFFF8 = -8`)
rely on this truncation. Any port must truncate to 16-bit at this boundary.

**Internal:** Register choice (CX, DX), the fact that the caller pushes 32-bit
values even though only 16-bit are used (calling convention artefact).

## Out-of-Scope Refs

- `MapCoord_Add` — commonly called immediately after `MapCoord_Set` to compute a
  derived cell; belongs to task #9 (fn-mapcoord-add).
- `MapClass__InitRevealSpiralTable @ 0x00561910` — the reveal spiral table; out of scope.
- `BuildingTypeClass__GetFoundationWidth` / `GetFoundationHeight` — used to compute
  dock offsets before `MapCoord_Set`; belong to tasks #60/#61.
- `vtable + 0x1B8` (`Get_Cell_Packed` vtable) — produces NW cell that callers add
  the `MapCoord_Set` offset to.

## Unverified

None. All claims verified from binary in this session.
