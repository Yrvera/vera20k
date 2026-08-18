# BuildingTypeClass__GetFoundationWidth — Decode Doc

## Summary

`BuildingTypeClass__GetFoundationWidth` (0x0045ec90) reads a foundation type
index from `BuildingTypeClass+0xef0` and returns the corresponding foundation
width in cells from the global table `g_FoundationWidthTable`. It is a leaf
function with a single field read and table lookup. The result is an integer
cell count (e.g., 4 for GAREFN, 3 for a 3-wide building). This is the
canonical source of building width used by `BuildingClass__GetCoords` to
compute the `(W−1) * 128` lepton center offset, and by 54 other call sites
across the entire building system.

Function body: 0x0045ec90
(verified via `decompile_function 0x0045ec90`)

## Active in YR

**Yes.** 54 call sites across core building systems including GetCoords,
DrawBody, Unlimbo, Sell, ExitObject_Main, ReceiveDamage, GetDockCellForObject,
DiskLaserClass__AI, TechnoClass__InRange, UnitClass__Deploy, and
SlaveManagerClass — all active in normal YR gameplay.
(verified via `get_xrefs_to 0x0045ec90`)

## Decompilation excerpt

```c
// verified via decompile_function 0x0045ec90
undefined4 __fastcall BuildingTypeClass__GetFoundationWidth(int param_1)
{
  return (&g_FoundationWidthTable)[*(int *)(param_1 + 0xef0)];
}
```

`param_1` is `int` (direct byte offsets — CLAUDE.md pitfall rule).
`param_1 + 0xef0` = `BuildingTypeClass` at byte offset 0xef0.
Return type `undefined4` = integer (width in cells).

## Behavioral analysis

### Input field

`*(int *)(param_1 + 0xef0)` — a 4-byte integer at `BuildingTypeClass+0xef0`.
This is the foundation type index: an integer that selects which predefined
foundation shape this building uses. Both `GetFoundationWidth` and
`GetFoundationHeight` read the same field at +0xef0, using it as an index into
separate width and height global tables.
(verified via `decompile_function 0x0045ec90` and `decompile_function 0x0045eca0`)

### Output: cells, not leptons

The return value is a foundation width in **cells** (integer). It is NOT in
leptons. Callers that need leptons apply the multiplication themselves:

In `BuildingClass__GetCoords`:
```c
iVar4 = BuildingTypeClass__GetFoundationWidth();
*param_2 = *(int *)(param_1 + 0x9c) + iVar4 * 0x80 + -0x80;
// = Location.X + width_cells * 128 - 128
// = Location.X + (width_cells - 1) * 128 leptons
```

`0x80` = 128 leptons = 0.5 cells. The formula `width * 128 - 128 = (width-1) * 128`
is the half-foundation-width offset that centers the coordinate.
(verified via `decompile_function 0x00447ac0`)

### g_FoundationWidthTable

A global array indexed by the foundation type index. The table values are
foundation widths in cells for each predefined foundation shape. Standard widths
in YR: 1, 2, 3, 4, 5, 6 cells (and more for large structures). The exact table
contents and address were not read in this session (game not running at decode
time).

### No callees — leaf function

The function is leaf: one field read, one table lookup, one return.
(no `get_function_callees` needed — body is a single expression)

### Concrete fixture

GAREFN (Allied Ore Refinery), 4×3 foundation:
- `BuildingTypeClass+0xef0` = foundation type index (e.g., index 3 for 4-wide)
- `g_FoundationWidthTable[3]` = 4 (cells)
- In GetCoords: `Location.X + 4 * 128 - 128 = Location.X + 384 leptons`
  = 1.5 cells east of NW corner = geometric center X of a 4-cell-wide building.

### INI key mapping

The foundation type index at +0xef0 is set from the `Foundation=` INI key in
`art(md).ini` for the building's art entry. Each predefined Foundation shape
(e.g., `Foundation=4x3`) maps to a specific index into the width/height tables.
The exact INI-to-index mapping is set during art INI parsing and was not traced
in this session.

## Struct field accesses

| Offset (bytes) | Size | Access | Semantics |
|---|---|---|---|
| `param_1 + 0xef0` | 4 (int) | read | Foundation type index → used to index g_FoundationWidthTable |

(verified via `decompile_function 0x0045ec90`)

## Callees

None. Leaf function — single field read and table lookup.

## Callers / Lifecycle

54 call sites across:

| Caller | Notable role |
|---|---|
| `BuildingClass__GetCoords` (0x00447ac0) | Foundation center X offset |
| `BuildingClass__GetHalfFoundationSize` (0x00458e00) | Half-size for placement |
| `BuildingClass__Unlimbo` (0x00440580) | Placement into map |
| `BuildingClass__ExitObject_Main` (0x00443c60) | Unit exit from building |
| `BuildingClass__ReceiveDamage` (0x00442230) | Damage area calculation |
| `BuildingClass__GetDockCellForObject` (0x0044efb0) | Dock cell lookup |
| `BuildingClass__SellBuilding` (0x00457de0) | Sell logic |
| `BuildingClass_DrawBody` (0x0043d290) | Rendering |
| `DiskLaserClass__AI` (0x004a7340) | Disk laser targeting |
| `TechnoClass__InRange` (0x006f7546) | Range check |
| `UnitClass__Deploy` (0x00739472) | Unit deploy into building |
| `SlaveManagerClass__AI_Update` (0x006af806) | Slave manager (Yuri slave) AI |
| + 42 additional call sites | Various building and AI systems |

(verified via `get_xrefs_to 0x0045ec90`)

## Out-of-scope refs

- `g_FoundationWidthTable` — global foundation shape table; initialization
  and layout is out of scope
- `BuildingTypeClass+0xef0` write site — where the foundation index is set
  during INI parsing; out of scope
- `art(md).ini` `Foundation=` INI key parsing — out of scope

## Unverified claims (YELLOW)

**UNVERIFIED**: The exact global address of `g_FoundationWidthTable`. Ghidra
references it by label (`&g_FoundationWidthTable`); the static VA was not read
via `read_memory` in this session.

**UNVERIFIED**: The exact mapping from `Foundation=NxM` INI values to the
foundation type index at +0xef0. The formula is consistent with the observed
behavior (GAREFN is 4-wide, formula produces 4), but the ReadINI path was not
traced.
