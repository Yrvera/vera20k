# Path_smooth_corners — Decode Doc
**Proposed Ghidra label:** Path_smooth_corners

## Summary

`Path_smooth_corners` at `0x0042B210` is the first of two post-A* smoothing passes
applied immediately after `AStar_reconstruct_path` builds the raw direction array.
Its purpose is to replace 90-degree zigzag pairs with diagonal shortcuts.

It iterates a direction array (array of `int`, one entry per path step), tracking
runs of identical directions. When it detects a 90-degree turn (direction delta of
exactly ±2 mod 8) where the anchor direction is diagonal (odd), it calls
`Path_smooth_single_segment` to attempt replacement. Bridge/tube direction 8 steps
are never smoothed — they update the current position via the tube lookup table
and reset zigzag state.

## Active in YR

**Yes.** Called unconditionally by `AStar_main_loop @ 0x00429A90` on every
successful A* result (verified via `get_function_callers 0x0042B210`). No gate flag;
active in all YR skirmishes.

## Callers

Verified via `get_function_callers 0x0042B210`:

| Caller | Address | Role |
|--------|---------|------|
| `AStar_main_loop` | `0x00429A90` | Calls immediately after `AStar_reconstruct_path` |

Call sequence in `AStar_main_loop` (from verified decompile of `0x00429A90`):
```
AStar_reconstruct_path(piStack_48, param_5)        → raw direction array
Path_smooth_corners(result, piVar2)                → Pass 1: zigzag removal
Path_optimize_straight_segments(result, piVar2)    → Pass 2: drift correction
```

## Callees

Verified via `get_function_callees 0x0042B210`:

| Callee | Address | Role |
|--------|---------|------|
| `MapClass__Get_CellClass` | `0x005657A0` | Look up cell for direction-8 tube exit |
| `Path_smooth_single_segment` | `0x0042B420` | Per-zigzag smoothing attempt |

## Decompilation analysis

Source: `decompile_function 0x0042B210`.

### Signature

```c
void __thiscall
Path_smooth_corners(undefined4 param_1,    // FootClass * (for passability)
                    undefined4 *param_2,   // path struct ptr
                    undefined4 param_3)    // FootClass * (passed to smooth_single)
```

`param_2` is a struct with at least 6 fields. From the decompile:
- `param_2[5]` = heights array base pointer (`local_8`)
- `param_2[3]` = directions array base pointer (`iVar1`)
- `param_2[2]` = path length (step count, used as loop bound via `iVar5 = param_2[2] - 1`)
- `param_2[0]` = current position (MapCoord, `local_2c`)

### Core algorithm

The function maintains:
- `uVar7` = current anchor direction (initialized to `0xFFFFFFFF` = "no anchor")
- `iVar8` = run length of the current anchor direction
- `iVar9` = write index (how far the output pointer has advanced)
- `local_20` = index of the start of the current zigzag
- `local_28` = length of the zigzag tail
- `local_14` = the zigzag direction (second direction)
- `bVar3` = in-zigzag flag

**Main loop (pseudocode):**

```c
while (iVar9 + iVar8 < path_length) {
    if (bVar3) {
        // Inside a zigzag: extend or flush
        if (directions[iVar9 + iVar8 + local_20] == local_14) {
            local_28++;    // extend zigzag run
        } else {
            // Flush: try smoothing
            iVar5 = Path_smooth_single_segment(
                param_3,
                &directions[iVar9],   // anchor run start
                &heights[iVar9],
                iVar8,                // anchor run length
                local_28,             // zigzag run length
                &local_24             // current pos (updated)
            );
            iVar9 += iVar5;
            iVar8 = 1;
            bVar3 = false;
            // Re-read the new current direction after smoothing
            uVar7 = directions[iVar9] & 7;
        }
    } else {
        uVar2 = directions[iVar9 + iVar8];   // next step direction
        uVar6 = (uVar2 - uVar7) & 7;         // direction delta mod 8

        if (uVar2 == uVar7) {
            iVar8++;   // same direction: extend run
        } else if ((uVar6 == 2 || uVar6 == 6)    // ±90-degree turn
                && uVar7 != 0xFFFFFFFF            // anchor is valid
                && uVar7 != 8 && uVar2 != 8) {   // neither is tube-dir
            // 90-degree turn on valid diagonal anchor: start zigzag
            bVar3 = true;
            local_28 = 1;
            local_20 = iVar9 + iVar8;
            local_14 = uVar2;
        } else {
            // Other direction change: reset anchor
            iVar8 = 1;
            uVar7 = uVar2;
            if ((uVar2 & 1) == 0) {
                uVar7 = 0xFFFFFFFF;   // cardinal → blank anchor
            }
            local_24 = local_2c;
            iVar9 = iVar9 + iVar8;
        }

        // Position update
        if (uVar2 == 8) {
            // Tube/bridge jump: look up exit coordinate
            iVar4 = MapClass__Get_CellClass(&local_2c);
            if (*(short *)(iVar4 + 0x116) == -1) {
                local_2c = 0;   // defensive fallback
            } else {
                local_2c = *(undefined4 *)(
                    *(int *)(g_TubeArray + *(short *)(iVar4 + 0x116) * 4) + 0x28
                );
            }
        } else {
            // Normal step: advance position by g_DirectionOffsets[uVar2]
            local_2c.x += g_DirectionOffsets[uVar2].dx;
            local_2c.y += g_DirectionOffsets[uVar2].dy;
        }
    }
}
// Final flush if still in zigzag at end of array
if (bVar3) {
    Path_smooth_single_segment(param_3, &directions[iVar9], &heights[iVar9],
                               iVar8, local_28, &local_24);
}
```

### Key invariants verified from decompile

**1. Only diagonal anchors trigger zigzag detection** (verified from `decompile_function 0x0042B210`):

```c
// In the "else" (direction change) branch:
uVar7 = uVar2;
if ((uVar2 & 1) == 0) {   // even = cardinal direction
    uVar7 = 0xffffffff;   // reset anchor — cardinal never anchors a zigzag
}
```

Cardinal directions (0/2/4/6 = N/E/S/W equivalent in whatever encoding) clear the
anchor to `0xFFFFFFFF`. Only odd (diagonal) directions persist as anchor candidates.

**2. Direction 8 is never a zigzag anchor or zigzag tail** (verified from decompile):

```c
} else if ((((uVar6 == 2) || (uVar6 == 6)) && (uVar7 != 0xffffffff)) &&
            ((uVar7 != 8 && (uVar2 != 8)))) {
    bVar3 = true;
```

Both `uVar7 != 8` (anchor not tube) and `uVar2 != 8` (current not tube) are required
to enter zigzag mode. Direction 8 never participates in smoothing.

**3. Tube exit coordinate lookup** uses `CellClass+0x116` (short tube_index) and
`g_TubeArray` pointer (verified from decompile): `*(int *)(g_TubeArray + tube_index * 4) + 0x28`
gives the exit MapCoord. `g_TubeArray` is the tube array; base pointer at `0x008B413C`
(consistent with cross-reference in `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md`).

**4. Direction offsets table** `g_DirectionOffsets` is used inline for position tracking:
```c
CONCAT22(*(short *)((int)&g_DirectionOffsets + uVar2 * 4 + 2) + local_2c._2_2_,
         *(short *)(&g_DirectionOffsets + uVar2) + (short)local_2c);
```
Each entry is 4 bytes: `short dx` at offset 0, `short dy` at offset 2. Table stride = 4 bytes
per direction.

## Self-proof (3 claims re-verified)

**Claim 1:** `Path_smooth_single_segment` at `0x0042B420` is the sole non-trivial callee.
Verified via `get_function_callees 0x0042B210` → result includes exactly
`Path_smooth_single_segment @ 0042b420` and `MapClass__Get_CellClass @ 005657a0`. No other callees.

**Claim 2:** `Path_smooth_single_segment` function boundaries confirmed via
`get_function_by_address 0x0042B420` → `Function: Path_smooth_single_segment at 0042b420,
Body: 0042b420 - 0042b7ea`. Label is correctly placed.

**Claim 3:** Steep-slope threshold for Pass 2 (`Path_optimize_straight_segments`)
is 0.01 (double). Verified via `read_memory 0x007e3808 8` →
`7b 14 ae 47 e1 7a 84 3f` = IEEE-754 LE double = 0.01 exactly. (Relevant to
`Path_smooth_corners` context: Pass 1 uses threshold 1.0 from a compiler global,
not this address — see `Path_smooth_single_segment` task #107.)

## Direction encoding

The path direction array uses integers 0–8:
- 0–7: eight compass directions (exact encoding: N=0 per A* neighbor table at
  `0x007E3774`, where Dir 0 = offset -512 = cell Y-1 = North — consistent with
  `g_DirectionOffsets` stride pattern)
- 8: bridge/tube cell-to-cell jump (special; position updated via `g_TubeArray`)
- -1 (`0xFFFFFFFF`): end sentinel
- -2 (`0xFFFFFFFE`): deleted/skip marker (used by Pass 2 compaction)

**Note:** Prior report §2 WARNING (2026-04-06) correctly disputes "S=0" — the A*
expansion table at `0x007E3774` uses N=0. This doc treats N=0 as authoritative.

## Path struct layout (param_2)

Inferred from decompile of `Path_smooth_corners`:

| Word offset | Byte offset | Type | Name | Notes |
|-------------|-------------|------|------|-------|
| `[0]` | `+0x00` | `MapCoord` | `current_pos` | Current position (updated as loop walks path) |
| `[2]` | `+0x08` | `int` | `path_length` | Total step count |
| `[3]` | `+0x0C` | `int*` | `directions` | Base pointer to direction int array |
| `[5]` | `+0x14` | `int*` | `heights` | Base pointer to parallel height array |

This layout is observed from `param_2[0]`, `param_2[2]`, `param_2[3]`, `param_2[5]`
accesses in the decompile. Other fields may exist but are not accessed here.

## CellClass fields accessed

| Offset | Type | Name | Notes |
|--------|------|------|-------|
| `+0x116` | `short` | `tube_index` | Index into `g_TubeArray`; -1 if no tube |

## Globals referenced

| Global | Address | Role |
|--------|---------|------|
| `g_DirectionOffsets` | inlined symbol reference | 4-byte per-direction dx/dy table |
| `g_TubeArray` | `0x008B413C` | Pointer array; `g_TubeArray[i]` = TubeClass*; `+0x28` = exit MapCoord |

## Control flow summary

```
Path_smooth_corners(foot, path_struct, foot2)
├── Load directions[], heights[], path_length, current_pos from path_struct
├── For each step in [0 .. path_length-1]:
│   ├── If in-zigzag (bVar3):
│   │   ├── Next dir == zigzag_dir → extend zigzag run
│   │   └── Next dir != zigzag_dir → flush:
│   │       └── Path_smooth_single_segment(foot2, &dir[anchor], &height[anchor],
│   │                                      anchor_len, zigzag_len, &cur_pos)
│   └── Not in-zigzag:
│       ├── Same dir → extend anchor run
│       ├── ±90° turn, both non-8, anchor is diagonal → enter zigzag mode
│       └── Other → reset anchor (cardinal blanks anchor to 0xFFFFFFFF)
│       ├── If dir == 8 → update cur_pos via TubeArray lookup
│       └── else → advance cur_pos by g_DirectionOffsets[dir]
└── Final flush if still in-zigzag at path end
```

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `AStar_main_loop` | `0x00429A90` | task #4 (completed) |
| `Path_smooth_single_segment` | `0x0042B420` | task #107 (pending) |
| `Path_optimize_straight_segments` | `0x0042B7F0` | task #14 (pending) |
| `MapClass__Get_CellClass` | `0x005657A0` | map utility; out of pathfinding scope |

## YELLOW — Unverified

- `g_DirectionOffsets` exact address: the decompile references it as `&g_DirectionOffsets`
  but the symbol address was not read via `read_memory` in this session. The stride (4 bytes,
  short dx at +0, short dy at +2) is confirmed from the decompile arithmetic. Absolute
  address was not independently verified here (prior report cites `0x818760` but the warning
  in §2 disputes that table's role as dx/dy offsets — see `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md` §2 WARNING).
- `param_2` struct layout at `[1]` and `[4]`: not accessed by `Path_smooth_corners`;
  layout inferred only from accessed offsets.
- `param_1` vs `param_3` distinction: both carry `FootClass*` pointers but the function
  passes `param_3` (not `param_1`) to `Path_smooth_single_segment`. The distinction between
  the two FootClass pointers is not clarified by this function alone.
