# MapClass__CellCoordToLinearIndex — Decode Doc

## Summary

`MapClass__CellCoordToLinearIndex` (0x0056d430) converts a packed CellStruct
(CONCAT22(cell_Y, cell_X)) into a flat linear array index using the formula:

```
linear = (MapClass+0xf8 + 1 + MapClass+0xf4) * cell_Y + cell_X
```

where `MapClass+0xf4` and `MapClass+0xf8` are width-related map dimension fields
that together define the row stride of the ZoneMap flat cell array. This is the
canonical gate between (x,y) cell coordinates and the flat zone-data array index.

Function body: 0x0056d430 (leaf function, no callees)
(verified via `decompile_function 0x0056d430` and `get_function_callees 0x0056d430`)

## Active in YR

**Yes.** The function has 5 named callers (14 total call sites) covering:
- `MapClass__RemoveBridgeZoneEdges` (0x00584e50) — bridge-zone cleanup, active
  every time a bridge collapses or is repaired in gameplay
- `ZoneMap__FloodFillReachableZones` (0x005840c0) — zone pathfinding reachability
  flood fill, active during zone initialization and bridge state changes
- `FUN_00582d70` (0x00582d70) — bridge/tube connectivity for zone graph
- `FUN_00584550` (0x00584550) — zone assignment of orphaned cells
- `FUN_00581140` (0x00581140) — bridge destruction handler with animation
(verified via `get_function_callers 0x0056d430` and `get_xrefs_to 0x0056d430`)

All callers are live in standard YR gameplay involving bridge mechanics and
pathfinding zone initialization — not gated behind any TS-only flag.

## Decompilation excerpt

```c
// verified via decompile_function 0x0056d430
int __thiscall MapClass__CellCoordToLinearIndex(int param_1, short *param_2)
{
  return (*(int *)(param_1 + 0xf8) + 1 + *(int *)(param_1 + 0xf4))
         * (int)param_2[1]   // cell_Y (high 16 bits of CellStruct)
         + (int)*param_2;    // cell_X (low 16 bits of CellStruct)
}
```

`param_1` is typed `int` (not `int *`), so all field offsets are direct byte
offsets per CLAUDE.md decompilation pitfall rule.

`param_2` is `short *` — Ghidra interprets the packed CellStruct as an array:
`param_2[0]` = cell_X (low 16-bit word), `param_2[1]` = cell_Y (high 16-bit word).

## Behavioral analysis

### Coordinate reference frame

- **Input**: packed CellStruct in "Get_Cell_Packed (NW cell)" frame (Frame #2)
  — same layout as ObjectClass__Get_Cell_Packed output: `CONCAT22(cell_Y, cell_X)`.
- **Output**: flat integer index into the ZoneMap's 1D cell data array.
- **Rust canonical frame**: cell-grid `(u16, u16)` +X east, +Y south. Pass as
  `cell_x = packed & 0xFFFF`, `cell_y = packed >> 16`.

### Row stride formula

```
stride = MapClass+0xf8 + 1 + MapClass+0xf4
linear = stride * cell_Y + cell_X
```

This is equivalent to `cell_Y * total_width + cell_X` for a rectangular flat
array where `total_width = f8 + 1 + f4`.

### Semantics of 0xf4 and 0xf8

These two fields together define the flat-array row stride. Evidence from callers:

From `ZoneMap__BuildZoneLevel` (0x00581f90, verified via `decompile_function 0x00581f90`):
```c
uStack_40 = DAT_0087f8dc + 1 + DAT_0087f8e0;
```
This local variable `uStack_40` is used as the row stride for iterating the flat
cell data table — when a per-row counter `local_64` reaches `uStack_40`, it resets
and increments the row counter. This confirms the `f8 + 1 + f4` sum is the total
cells per row (row stride), stored also as globals `DAT_0087f8dc` (matching field
0xf8) and `DAT_0087f8e0` (matching field 0xf4).

From `MapClass__ComputeBridgeZones` (0x0056d6e0, verified via `decompile_function 0x0056d6e0`):
```c
iVar8 = *(int *)(param_1 + 0xf4);
// playfield boundary check using diamond coordinates:
// (X+Y <= f4) || (f4 <= X-Y) || (f4 <= Y-X) || (X+Y > f4 + f8*2)
```
Here `f4` is used as a boundary value in the isometric diamond playfield check —
it represents the left/top margin of the playfield in isometric (X+Y) coordinate
space. Field `f8` is used as the width half-span.

**Conclusion**: 
- `MapClass+0xf4` = left/edge margin (the cell offset from the array origin to
  the playfield edge, used as both a margin value and part of the row stride)
- `MapClass+0xf8` = interior playfield span
- Row stride = `f8 + 1 + f4` = total cells in one row of the flat array

The exact semantic labels for 0xf4 and 0xf8 (e.g., "LocalSize" vs "MapSize" in
RA2 terminology) are UNVERIFIED — see the YELLOW section below. The formula and
row-stride role ARE verified.

### No callees

Leaf function — no calls to other functions.
(verified via `get_function_callees 0x0056d430` → "No callees found")

### Bounds clamping in callers

Every caller immediately clamps the result:
```c
if (iVar < 0) iVar = 0;
else if (*(int *)(param_1 + 0x6c) <= iVar) iVar = *(int *)(param_1 + 0x6c) - 1;
```
Where `MapClass+0x6c` is the total zone-table count (separate from the flat cell
array size). This is a defensive clamp at the caller, not inside this function.

### INI keys / globals / enums

No INI keys or enum comparisons. The fields 0xf4 and 0xf8 appear to be set during
map load from the `[Map]` section dimensions. No global reads inside this function.

## Struct field accesses

| Offset (bytes) | Size | Access | Semantics |
|---|---|---|---|
| `param_1 + 0xf4` | 4 (int) | read | Left/edge margin — contributes to row stride |
| `param_1 + 0xf8` | 4 (int) | read | Playfield span — contributes to row stride |
| `*param_2` (offset 0) | 2 (short) | read | cell_X — low word of packed CellStruct |
| `param_2[1]` (offset 2) | 2 (short) | read | cell_Y — high word of packed CellStruct |

Param type is `int` for param_1 (direct byte offsets). Param_2 is `short *`.
(verified via `decompile_function 0x0056d430`)

## Callers / Lifecycle

| Caller | Address | Call sites | Context |
|---|---|---|---|
| `FUN_00582d70` | 0x00582d70 | 6 call sites | Bridge/tube connectivity for zone-graph |
| `FUN_00584550` | 0x00584550 | 2 call sites | Zone orphan assignment |
| `MapClass__RemoveBridgeZoneEdges` | 0x00584e50 | 2 call sites | Bridge zone cleanup |
| `FUN_00581140` | 0x00581140 | 2 call sites | Bridge destruction + animation handler |
| `ZoneMap__FloodFillReachableZones` | 0x005840c0 | 2 call sites | Zone reachability flood fill |

14 total call sites, all within the zone/pathfinding subsystem.
(verified via `get_xrefs_to 0x0056d430`)

## Out-of-scope refs

- `MapClass+0x6c` — total zone-table count, used for bounds clamping in callers; out of scope
- `MapClass+0x70` — pointer to zone data table, used as array base in callers; out of scope
- `ZoneMap__FloodFillReachableZones` internals — zone fill logic; out of scope
- `MapClass__ComputeBridgeZones` internals — bridge record building; out of scope
- `ZoneMap__BuildZoneLevel` — zone level builder using this function's output; out of scope
- `DAT_0087f8dc`, `DAT_0087f8e0` — global mirrors of field 0xf8/0xf4 seen in BuildZoneLevel

## Unverified claims (YELLOW)

**UNVERIFIED**: The exact semantic labels for `MapClass+0xf4` and `MapClass+0xf8`.
The formula and their role as row-stride contributors are verified. The specific
naming (e.g., RA2 modding terminology "LocalSize" for map playfield dimensions vs.
array margin) is not verified from binary — would require finding where these fields
are written during map/scenario load. The playfield-check usage in ComputeBridgeZones
(where f4 is the isometric boundary threshold) differs from the row-stride role here,
suggesting f4 may encode a value that serves dual purposes: as a coordinate boundary
AND as a row-margin count. This dual use is observed but not fully decoded.
