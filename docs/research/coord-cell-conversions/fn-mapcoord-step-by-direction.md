# MapCoord_Step_By_Direction — Decode Doc

## Summary

`MapCoord_Step_By_Direction` (0x0042d490) advances a packed CellStruct one step
in a given direction. For directions 0–7 it reads the delta from a global lookup
table `g_DirectionOffsets` (8 entries × 4 bytes, each entry two 16-bit signed
deltas: X at byte 0, Y at byte 2) and adds the delta to the input cell. For the
special direction value 8 it performs a tube-traversal: it calls
`MapClass__Get_CellClass` to retrieve the CellClass for the input cell, reads the
tube index from `CellClass+0x116`, looks up the tube record via
`g_TubeArray[tube_idx]`, and returns the destination packed cell stored at
`tube_record+0x28`. If the cell has no tube (index == -1) it returns 0 (null cell).

Function body: 0x0042d490
(verified via `decompile_function 0x0042d490`)

## Active in YR

**Yes.** Called exclusively from `Path_smooth_single_segment` (0x0042b420), which
is the path-smoothing pass of the drive locomotor pathfinder. Drive locomotor
pathfinding runs during every unit-move order in all normal YR gameplay.
(verified via `get_function_callers 0x0042d490` and `get_xrefs_to 0x0042d490`)

## Decompilation excerpt

```c
// verified via decompile_function 0x0042d490
undefined4 * __fastcall MapCoord_Step_By_Direction(
    undefined4 *param_1,   // out: pointer to destination packed CellStruct
    short      *param_2,   // in:  source packed CellStruct (short*: [0]=X, [1]=Y)
    int         param_3)   // in:  direction 0–7, or 8 for tube traversal
{
  int iVar1;

  if (param_3 != 8) {
    // Direction 0–7: apply delta from g_DirectionOffsets[param_3]
    // Entry layout: short X_delta at offset 0, short Y_delta at offset +2
    param_3 = CONCAT22(
        *(short *)((int)&g_DirectionOffsets + param_3 * 4 + 2) + param_2[1],  // new Y
        *param_2 + *(short *)(&g_DirectionOffsets + param_3)                  // new X
    );
    *param_1 = param_3;
    return param_1;
  }

  // Direction 8: tube traversal
  iVar1 = MapClass__Get_CellClass(param_2);           // get CellClass* for source cell
  if (*(short *)(iVar1 + 0x116) != -1) {              // CellClass+0x116 = tube index (-1 = no tube)
    *param_1 = *(undefined4 *)(
        *(int *)(g_TubeArray + *(short *)(iVar1 + 0x116) * 4) + 0x28
    );                                                  // tube_record+0x28 = destination packed cell
    return param_1;
  }

  *param_1 = 0;   // no tube at this cell — return null/zero cell
  return param_1;
}
```

`param_2` is `short *` — Ghidra interprets the packed CellStruct as an array:
`param_2[0]` = cell_X (low 16 bits), `param_2[1]` = cell_Y (high 16 bits).
The output `*param_1` is a full 32-bit packed CellStruct in the same format.

## Behavioral analysis

### Direction encoding

`param_3` is a **direction integer 0–7** (or the sentinel 8). This is NOT:
- A **facing byte** (0–255, clockwise from north where 0x40=East). These look
  similar but index completely different lookup tables. Confusing them is a
  recurring bug class (see [[feedback-direction-bugs]] and CLAUDE.md).
- A **drive-track index** (0–127, indexes `g_DriveTrackData_Array`). `0x47`
  is a drive-track curve entry, NOT a direction step.

The direction integer convention used here is 0 = North, 1 = NE, 2 = East, ...
7 = NW (or an equivalent 8-point scheme). The exact mapping of integer values to
compass directions is encoded in `g_DirectionOffsets` — verified by the access
pattern but the precise per-entry delta values were not readable at decode time
(game was not running; `read_memory` of the table address returned all zeros).

**YELLOW** — see Unverified section for the per-entry delta values.

### g_DirectionOffsets table

- Global symbol at runtime address resolved via decompilation access pattern.
- 8 entries, each 4 bytes: `short X_delta` at entry offset 0, `short Y_delta` at
  entry offset 2.
- Access: `X_delta = *(short*)(&g_DirectionOffsets + param_3 * 4)`,
  `Y_delta = *(short*)(&g_DirectionOffsets + param_3 * 4 + 2)`.
- Result cell: `new_X = cell_X + X_delta`, `new_Y = cell_Y + Y_delta`.
- No bounds check inside this function — caller is responsible for validating the
  resulting cell is within map bounds.

(structure verified via `decompile_function 0x0042d490`; exact delta values
UNVERIFIED — `read_memory` at the table returned zeros because the game was not
running)

### Direction 8: tube traversal

When `param_3 == 8` the function does not use `g_DirectionOffsets`. Instead:

1. Calls `MapClass__Get_CellClass(param_2)` to retrieve the `CellClass*` for the
   source cell.
   (callee verified via `get_function_callees 0x0042d490`)
2. Reads `*(short*)(CellClass* + 0x116)` — the cell's tube index. Value -1 means
   no tube is present.
3. If a tube exists: reads `*(int*)(g_TubeArray + tube_index * 4)` to get the
   tube record pointer, then reads the destination packed cell from offset +0x28
   inside the tube record.
4. If no tube: writes 0 to `*param_1` (null/zero cell) and returns.

`CellClass+0x116` is a 16-bit signed field. Its semantics are tube index (into
`g_TubeArray`) for the tube overlay on this cell. Value -1 is the "no tube"
sentinel.

### Caller context: Path_smooth_single_segment

`Path_smooth_single_segment` (0x0042b420) is the only caller, with 2 call sites.
It is the path-smoothing pass applied to a sequence of waypoints generated by the
drive locomotor pathfinder. The function calls `MapCoord_Step_By_Direction` to
compute one-step neighbours in order to validate or reroute path segments.

The caller selects direction 0–7 from path-segment data and passes it directly to
`param_3`. Direction 8 (tube) is also a valid input from the caller when the path
crosses a tube cell.

(verified via `decompile_function 0x0042b420` and `get_xrefs_to 0x0042d490`)

### Coordinate reference frame

- **Input**: `param_2` is a packed CellStruct — same frame as
  `ObjectClass__Get_Cell_Packed` output: `CONCAT22(cell_Y, cell_X)`, high 16 bits
  = cell_Y, low 16 bits = cell_X. Rust canonical frame: `(u16, u16)` (+X east,
  +Y south).
- **Output**: `*param_1` is a packed CellStruct in the same format.
- The delta from `g_DirectionOffsets` is in cells (not leptons). This is a
  cell-space step, not a lepton-space displacement.

### No bounds checking

The function does not validate that the resulting cell lies within the map. All
bounds enforcement is the caller's responsibility.

### INI keys / enums

No INI key reads. No enum comparisons beyond the `param_3 != 8` sentinel check.

## Struct field accesses

| Object | Offset (bytes) | Size | Access | Semantics |
|---|---|---|---|---|
| `param_2` (source packed cell) | 0x00 (low word) | 2 (short) | read | cell_X |
| `param_2` (source packed cell) | 0x02 (high word) | 2 (short) | read | cell_Y |
| `g_DirectionOffsets` entry | param_3 * 4 + 0 | 2 (short) | read | X delta (cells) |
| `g_DirectionOffsets` entry | param_3 * 4 + 2 | 2 (short) | read | Y delta (cells) |
| `CellClass` (from Get_CellClass) | +0x116 | 2 (short) | read | Tube index; -1 = no tube |
| `g_TubeArray[tube_idx]` pointer | +0x00 | 4 (int*) | read | Pointer to tube record |
| Tube record | +0x28 | 4 (undefined4) | read | Destination packed CellStruct |
| `*param_1` (output) | 0x00 | 4 (undefined4) | write | Result packed CellStruct |

(verified via `decompile_function 0x0042d490`)

## Callees

| Callee | Address | Purpose |
|---|---|---|
| `MapClass__Get_CellClass` | 0x005657a0 | Returns CellClass* for a packed cell — tube case only |

(verified via `get_function_callees 0x0042d490`)

## Callers / Lifecycle

| Caller | Address | Call sites | Context | Sim-side? |
|---|---|---|---|---|
| `Path_smooth_single_segment` | 0x0042b420 | 2 | Drive locomotor path smoothing | YES |

Only one caller with two call sites. No additional xrefs found.
(verified via `get_function_callers 0x0042d490` and `get_xrefs_to 0x0042d490`)

## Out-of-scope refs

- `g_TubeArray` — tube record array; full struct layout of tube records is out of scope
- `CellClass+0x116` tube index field — the tube system as a whole is out of scope
- `MapClass__Get_CellClass` (0x005657a0) internals — cell lookup; out of scope
- `Path_smooth_single_segment` full logic — pathfinder internals; out of scope
- Tube connectivity (`[Tubes]` INI section or map overlays) — out of scope

## Unverified claims (YELLOW)

**UNVERIFIED**: The exact per-entry delta values in `g_DirectionOffsets` (which
integer direction maps to which (X_delta, Y_delta) pair). The table structure
(8 entries × 4 bytes, short X then short Y) is verified from the decompilation
access pattern. The per-entry values were not readable because `read_memory` at
the table's runtime address returned all zeros (game was not running during this
decode session). Typical convention: 0=N=(0,-1), 1=NE=(+1,-1), 2=E=(+1,0),
3=SE=(+1,+1), 4=S=(0,+1), 5=SW=(-1,+1), 6=W=(-1,0), 7=NW=(-1,-1) — but this
is inferred from common isometric conventions, NOT read from the binary.

**UNVERIFIED**: The address of `g_DirectionOffsets` at the Ghidra static address.
The decompilation references it by name/label; the exact static VA was not
inspected with `get_xrefs_to` or `list_globals` in this session.
