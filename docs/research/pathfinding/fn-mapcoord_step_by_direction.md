# MapCoord_Step_By_Direction — Decode Doc
**Proposed Ghidra label:** `MapCoord_Step_By_Direction` (already labeled)
**Address:** `0x0042D490`

## Summary

`MapCoord_Step_By_Direction` at `0x0042D490` advances a cell coordinate one step in a
given direction (0–7 compass, or 8 = tube/bridge jump) and stores the result in an
output parameter. Returns the output pointer as a passthrough.

- Directions 0–7: step via `g_DirectionOffsets[dir]` (dx/dy table).
- Direction 8: tube jump — looks up the cell's tube index at `CellClass+0x116`, then
  returns the tube exit coordinate from `g_TubeArray[tube_idx] + 0x28`.

Body is 28 bytes. Sole caller is `Path_smooth_single_segment`.

## Active in YR

**Yes.** Sole caller is `Path_smooth_single_segment @ 0x0042B420` (task #107,
completed), which is on the live `AStar_main_loop → Path_optimize_straight_segments`
chain. Verified via `get_function_callers 0x0042D490`.

## Callers

Verified via `get_function_callers 0x0042D490`:

| Caller | Address | Role |
|--------|---------|------|
| `Path_smooth_single_segment` | `0x0042B420` | Step accumulated cell coordinate during smoothing |

## Callees

Verified via `get_function_callees 0x0042D490`:

| Callee | Address | Role |
|--------|---------|------|
| `MapClass__Get_CellClass` | `0x005657A0` | Resolve current coord to `CellClass*` for tube lookup |

## Signature

```c
undefined4 * __fastcall MapCoord_Step_By_Direction(
    undefined4  *param_1,   // OUT: result coordinate (packed: low=x, high=y)
    short       *param_2,   // current coordinate (short[2]: [0]=x, [1]=y)
    int          param_3    // direction: 0-7 = compass, 8 = tube jump
)
```

`__fastcall`. Returns `param_1` (output pointer passthrough).

## Full Algorithm (annotated from decompile)

```c
undefined4 * __fastcall MapCoord_Step_By_Direction(
    undefined4 *param_1, short *param_2, int param_3)
{
    if (param_3 != 8) {
        // Compass direction: step via direction offset table
        short dx = *(short *)(&g_DirectionOffsets + param_3 * 4);      // dx at +0
        short dy = *(short *)((int)&g_DirectionOffsets + param_3*4+2);  // dy at +2
        short result_x = param_2[0] + dx;
        short result_y = param_2[1] + dy;
        *param_1 = CONCAT22(result_y, result_x);  // packed: low=x, high=y
        return param_1;
    }

    // Direction 8: tube/bridge jump
    CellClass *cell = MapClass__Get_CellClass(param_2);
    short tube_index = *(short *)(cell + 0x116);  // CellClass+0x116: tube index (-1 = none)

    if (tube_index != -1) {
        // Look up tube exit coordinate
        TubeClass *tube = *(TubeClass **)(g_TubeArray + tube_index * 4);
        *param_1 = *(undefined4 *)(tube + 0x28);  // TubeClass+0x28 = exit MapCoord
        return param_1;
    }

    // No tube found: output null coord
    *param_1 = 0;
    return param_1;
}
```

Decompile verbatim (from `decompile_function 0x0042D490`):
```c
if (param_3 != 8) {
    param_3 = CONCAT22(
        *(short *)((int)&g_DirectionOffsets + param_3 * 4 + 2) + param_2[1],
        *param_2 + *(short *)(&g_DirectionOffsets + param_3));
    *param_1 = param_3;
    return param_1;
}
iVar1 = MapClass__Get_CellClass(param_2);
if (*(short *)(iVar1 + 0x116) != -1) {
    *param_1 = *(undefined4 *)(*(int *)(g_TubeArray + *(short *)(iVar1 + 0x116) * 4) + 0x28);
    return param_1;
}
*param_1 = 0;
return param_1;
```

## Direction encoding

| Value | Meaning |
|-------|---------|
| 0 | N (north) |
| 1 | NE |
| 2 | E |
| 3 | SE |
| 4 | S |
| 5 | SW |
| 6 | W |
| 7 | NW |
| 8 | Tube/bridge jump — cell's tube entry at `CellClass+0x116` |

Same encoding used throughout the pathfinding subsystem. Verified consistent with
`Path_walk_directions_to_cell` (task #15 doc) which uses the same direction-8 tube
lookup pattern.

## CellClass and globals accessed

| Symbol | Offset/Address | Type | Meaning |
|--------|---------------|------|---------|
| `CellClass` | `+0x116` | `short` | Tube index (-1 = no tube at this cell) |
| `g_DirectionOffsets` | `0x0089F688` | `short[2][8]` | dx at +0, dy at +2, stride 4 |
| `g_TubeArray` | `0x008B413C` | `TubeClass*[]` | Array of tube object pointers |
| `TubeClass` | `+0x28` | `undefined4` | Exit cell coordinate (packed MapCoord) |

`g_DirectionOffsets @ 0x0089F688` and `g_TubeArray @ 0x008B413C` confirmed from
prior session assembly tracing (task #15 doc).

`CellClass+0x116` is newly observed in this session. It is the tube index — the same
field read in `Path_walk_directions_to_cell` (which used `*(short *)(iVar1 + 0x116)`
in an equivalent pattern per task #15 doc).

## Self-proof (3 claims verified)

**Claim 1:** Sole caller is `Path_smooth_single_segment @ 0x0042B420`.
Verified via `get_function_callers 0x0042D490` → exactly one caller returned.

**Claim 2:** Sole callee is `MapClass__Get_CellClass @ 0x005657A0`.
Verified via `get_function_callees 0x0042D490` → exactly one callee.

**Claim 3:** Direction 8 reads tube index from `CellClass+0x116`; compass directions
0-7 use `g_DirectionOffsets` with stride 4 (dx at +0, dy at +2).
Confirmed from `decompile_function 0x0042D490`:
- `if (param_3 != 8)` → direction gate
- `*(short *)(&g_DirectionOffsets + param_3)` → dx at offset 0 (stride 4 via `param_3*4` in dy line)
- `*(short *)(iVar1 + 0x116)` → tube index from CellClass+0x116

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `Path_smooth_single_segment` | `0x0042B420` | task #107 (completed) |
| `MapClass__Get_CellClass` | `0x005657A0` | map utility |
| `g_DirectionOffsets` | `0x0089F688` | global direction table |
| `g_TubeArray` | `0x008B413C` | global tube array |

## YELLOW — Unverified

- **`CellClass+0x116` name**: confirmed as tube index from the decompile pattern
  (`*(short *)(iVar1 + 0x116)`), and consistent with task #15 (`Path_walk_directions_to_cell`)
  which uses the same offset. The field's named identifier in the Ghidra struct was
  not verified via struct layout query in this session.
- **Null-coord fallback**: when `tube_index == -1` (no tube), `*param_1 = 0` — stores
  a null coordinate `(0, 0)`. The caller's behavior when receiving a null coord was
  not traced in this session.
