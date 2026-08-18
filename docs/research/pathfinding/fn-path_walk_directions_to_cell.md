# Path_walk_directions_to_cell — Decode Doc
**Proposed Ghidra label:** Path_walk_directions_to_cell

## Summary

`Path_walk_directions_to_cell` at `0x00429780` walks a sequence of path direction
entries from a starting MapCoord to compute the destination cell. It is a utility
called by `FootClass__Run_AStar` to derive the endpoint of the current buffered path
and by `FUN_00582D70` (purpose unverified).

The function steps through `param_3` entries from `param_4` (the directions array).
For each entry:
- Direction 0–7: advances the current position by `g_DirectionOffsets[dir]`
- Direction 8: performs a tube/bridge jump via `g_TubeArray[CellClass.tube_index].exit_coord`

The resulting MapCoord is written to `*param_1` and returned.

## Active in YR

**Yes.** Called by `FootClass__Run_AStar @ 0x004CBBA0` (verified via
`get_function_callers 0x00429780`), which is on the live YR pathfinding chain
(`FootClass → Find_Path → Run_AStar`). Also called by `FUN_00582D70 @ 0x00582D70`
(YR presence not independently verified in this session).

## Callers

Verified via `get_function_callers 0x00429780`:

| Caller | Address | Role |
|--------|---------|------|
| `FootClass__Run_AStar` | `0x004CBBA0` | Main A* dispatch; uses result to determine path endpoint |
| `FUN_00582D70` | `0x00582D70` | Unknown caller (body: `0x00582D70`–`0x00583173`; not traced in this session) |

## Callees

Verified via `get_function_callees 0x00429780`:

| Callee | Address | Role |
|--------|---------|------|
| `MapClass__Get_CellClass` | `0x005657A0` | Looks up CellClass* from MapCoord (needed for tube index) |

## Decompilation analysis

Source: `decompile_function 0x00429780`.

### Signature

```c
undefined4 * __fastcall
Path_walk_directions_to_cell(
    undefined4 *param_1,    // OUT: destination MapCoord ptr (written and returned)
    undefined4 *param_2,    // IN:  starting MapCoord ptr
    int         param_3,    // IN:  number of steps to walk
    int        *param_4     // IN:  directions array base (int[param_3])
)
```

`__fastcall` convention: first two args in registers (ECX/EDX = param_1, param_2),
remaining on stack.

### Full decompile

```c
undefined4 * __fastcall
Path_walk_directions_to_cell(undefined4 *param_1, undefined4 *param_2,
                              int param_3, int *param_4)
{
    short sVar1;
    int iVar2;
    int *piVar3;
    int iVar4;
    undefined4 local_4;

    local_4 = *param_2;       // copy start position
    piVar3 = param_4;         // walking pointer into directions array
    iVar4 = param_3;          // step counter

    if (param_3 < 1) {
        *param_1 = local_4;   // zero steps: output = start
        return param_1;
    }

    do {
        iVar2 = *piVar3;      // current direction entry

        if (iVar2 == 8) {
            // Tube/bridge jump: look up exit coord via g_TubeArray
            iVar2 = MapClass__Get_CellClass(&local_4);
            if (*(short *)(iVar2 + 0x116) == -1) {
                param_3 = 0;  // no tube at this cell: position becomes (0,0)
            } else {
                param_3 = *(undefined4 *)(
                    *(int *)(g_TubeArray + *(short *)(iVar2 + 0x116) * 4) + 0x28
                );
            }
        } else {
            // Normal direction: advance position
            sVar1 = (short)local_4;            // current x (low word)
            local_4._2_2_ = (short)((uint)local_4 >> 0x10);  // current y (high word)
            // new_x = x + g_DirectionOffsets[iVar2].dx
            // new_y = y + g_DirectionOffsets[iVar2].dy
            param_4 = (int *)CONCAT22(
                *(short *)((int)&g_DirectionOffsets + iVar2 * 4 + 2) + local_4._2_2_,
                *(short *)(&g_DirectionOffsets + iVar2) + sVar1
            );
            param_3 = (int)param_4;  // reuses param_3 slot as temp
        }

        iVar4 = iVar4 - 1;
        local_4 = param_3;    // update current position
        piVar3 = piVar3 + 1;  // advance directions pointer
    } while (iVar4 != 0);

    *param_1 = param_3;       // write final position to output
    return param_1;
}
```

### MapCoord layout (inferred from decompile)

The position (`local_4`) is a packed `int`:
- Low word (`(short)local_4`): X coordinate (cell column)
- High word (`(short)(local_4 >> 0x10)`): Y coordinate (cell row)

`g_DirectionOffsets` entries are also 4 bytes each:
- `+0x00` (short): dx (added to x)
- `+0x02` (short): dy (added to y)

### Direction encoding

Consistent with `Path_smooth_corners` and `Path_optimize_straight_segments`:
- 0–7: eight compass directions, indexed via `g_DirectionOffsets` (same table)
- 8: bridge/tube jump — cell's `CellClass+0x116` (short `tube_index`) → `g_TubeArray`
- Any value with `== -1` sentinel in tube_index: position reset to (0,0)

The exact direction-to-compass mapping (N=0 vs S=0) is not independently verified
in this session; it matches whatever convention `g_DirectionOffsets` encodes — see
`fn-path_smooth_corners.md` YELLOW section for the N=0 vs S=0 dispute.

### Early exit (zero steps)

If `param_3 < 1` on entry, the function immediately writes `*param_2` to `*param_1`
and returns — the output equals the input start position unchanged.

### Tube jump behavior

When direction == 8:
1. `MapClass__Get_CellClass(&local_4)` → `CellClass*`
2. Read `CellClass+0x116` (short) = `tube_index`
3. If `tube_index == -1`: position set to 0 (defensive fallback; path state broken)
4. Else: position = `g_TubeArray[tube_index]->field_0x28` (exit MapCoord)

`g_TubeArray` base pointer is at `0x008B413C` (consistent with `fn-path_smooth_corners.md`).

## Self-proof (3 claims re-verified)

**Claim 1:** `FootClass__Run_AStar @ 0x004CBBA0` is a caller.
Verified via `get_function_callers 0x00429780` → result includes
`FootClass__Run_AStar @ 004cbba0`.

**Claim 2:** `MapClass__Get_CellClass @ 0x005657A0` is the sole callee.
Verified via `get_function_callees 0x00429780` → result is exactly
`MapClass__Get_CellClass @ 005657a0`.

**Claim 3:** `FUN_00582D70` is a named function at `0x00582D70` with body ending at
`0x00583173`. Verified via `get_function_by_address 0x00582D70` → confirmed entry
and body range.

## Globals referenced

| Global | Address / Symbol | Role |
|--------|-----------------|------|
| `g_DirectionOffsets` | `0x0089F688` | 4-byte dx/dy per direction; stride = 4 (dx at +0, dy at +2, both signed shorts). Address confirmed from assembly: `MOV DX,word ptr [ECX*0x4 + 0x89f688]` (x-delta), `MOV AX,word ptr [ECX*0x4 + 0x89f68a]` (y-delta). Verified via `get_assembly_context 0x004297E1`. |
| `g_TubeArray` | `0x008B413C` | Pointer array; `[tube_index]` → TubeClass*; `+0x28` = exit MapCoord. Assembly: `MOV ECX,dword ptr [0x008b413c]`. Verified via `get_assembly_context 0x004297E1`. |
| MapClass singleton | `0x0087F7E8` | Passed as ECX to `MapClass__Get_CellClass` during tube lookup. Assembly: `MOV ECX,0x87f7e8`. Verified via `get_assembly_context 0x004297E1`. |

## CellClass fields accessed

| Offset | Type | Name | Notes |
|--------|------|------|-------|
| `+0x116` | `short` | `tube_index` | Index into `g_TubeArray`; -1 = no tube |

## Control flow summary

```
Path_walk_directions_to_cell(out, start, n_steps, directions[])
├── pos = *start
├── If n_steps < 1 → *out = pos; return
├── For i in [0 .. n_steps-1]:
│   ├── dir = directions[i]
│   ├── If dir == 8:
│   │   ├── cell = MapClass__Get_CellClass(pos)
│   │   ├── tube_idx = cell[+0x116]
│   │   ├── If tube_idx == -1 → pos = (0,0)
│   │   └── Else → pos = g_TubeArray[tube_idx].exit_coord (+0x28)
│   └── Else (dir 0..7):
│       └── pos += g_DirectionOffsets[dir]  (dx short, dy short)
└── *out = pos; return out
```

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `FootClass__Run_AStar` | `0x004CBBA0` | task #2 (completed) |
| `FUN_00582D70` | `0x00582D70` | Unknown; not in pathfinding task list |
| `MapClass__Get_CellClass` | `0x005657A0` | Map utility; out of pathfinding scope |

## YELLOW — Unverified

- `FUN_00582D70` role: the function at `0x00582D70` is a non-trivial caller (body is
  ~1KB) but was not decompiled in this session. Its call to
  `Path_walk_directions_to_cell` was verified via callers list; the purpose and
  YR-activity of `FUN_00582D70` itself was not traced. Per manifest it is a
  "ZoneMap build helper; phase0-drop; zone-system territory."
- `g_DirectionOffsets` exact runtime delta values: the base address `0x0089F688`
  is confirmed from assembly (verified via `get_assembly_context 0x004297E1`), and the
  stride/layout is confirmed. However the table is runtime-populated (reads as zero
  in static Ghidra memory), so the actual (dx,dy) pairs per direction index were not
  read this session. To verify: find xrefs to `0x0089F688` with WRITE access and
  decompile the initializer.
- Direction 8 fallback `pos = 0`: when `tube_index == -1`, the decompile sets
  `param_3 = 0`, making `local_4 = 0` (MapCoord (0,0)). This is consistent with
  the defensive fallback described in `fn-path_smooth_corners.md §8.3` but was not
  separately traced to determine if this ever triggers in practice.
