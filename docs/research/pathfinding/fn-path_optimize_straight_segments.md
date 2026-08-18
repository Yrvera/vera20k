# Path_optimize_straight_segments — Decode Doc
**Proposed Ghidra label:** Path_optimize_straight_segments

## Summary

`Path_optimize_straight_segments` at `0x0042B7F0` is the second post-A* smoothing
pass, applied immediately after `Path_smooth_corners`. It detects segments of the
direction array where the path "curves back" toward its starting point (Chebyshev
regression) and replaces them with straight-line cardinal+diagonal decompositions.

The function operates over a maximum window of 20 steps (`if (0x13 < iVar13) break`
from decompile). It tracks cumulative displacement from a segment anchor point. When
the Chebyshev distance from the anchor decreases (the path curves back), it calls
`Path_Find_Split_Anchor` to locate the inflection point and `Path_Reroute_Straight_Line`
to replace the segment. After all passes, it compacts the direction array by removing
`0xFFFFFFFE` (deleted) entries and updating the path length.

## Active in YR

**Yes.** Called unconditionally by `AStar_main_loop @ 0x00429A90` on every
successful A* result (verified via `get_function_callers 0x0042B7F0`). No gate flag;
active in all YR skirmishes.

## Callers

Verified via `get_function_callers 0x0042B7F0`:

| Caller | Address | Role |
|--------|---------|------|
| `AStar_main_loop` | `0x00429A90` | Calls as second smoothing pass after `Path_smooth_corners` |

## Callees

Verified via `get_function_callees 0x0042B7F0`:

| Callee | Address | Role |
|--------|---------|------|
| `Path_Find_Split_Anchor` | `0x0042BCA0` | Finds Chebyshev-peak inflection point in segment |
| `Path_Reroute_Straight_Line` | `0x0042BE20` | Cardinal+diagonal decomposer; validates each step |

Note: `MapClass__Get_CellClass` and slope check helpers are called from within
`Path_Reroute_Straight_Line`, not directly from this function.

## Decompilation analysis

Source: `decompile_function 0x0042B7F0`.

### Signature

```c
void __thiscall
Path_optimize_straight_segments(undefined4 param_1,    // FootClass * (for passability)
                                 undefined4 *param_2,   // path struct ptr (same as Pass 1)
                                 undefined4 param_3)    // FootClass * (passed to rerouter)
```

`param_2` layout (consistent with `Path_smooth_corners`):
- `param_2[0]` = current start position (MapCoord)
- `param_2[2]` = path length (step count; updated at end of compaction)
- `param_2[3]` = directions array base pointer
- `param_2[5]` = heights array base pointer

### Key variables

| Ghidra local | Role |
|-------------|------|
| `iVar13` | current step index (loop counter; breaks if `> 0x13 = 19`) |
| `local_80` | walking pointer into directions array |
| `local_7c` | cumulative displacement from segment anchor (short x, short y packed) |
| `local_78` | anchor-relative displacement reset target |
| `local_70` / `local_6c` | previous Chebyshev abs(x) / abs(y) from cumulative |
| `local_68` | peak Chebyshev distance seen in current segment |
| `local_64` | step index of current segment anchor |
| `local_5c` | step index saved for split-anchor call |
| `local_20` / `sStack_1e` | accumulated x/y displacement for current scan window |

### Core algorithm (pseudocode)

```c
iVar13 = 0;     // step index
local_5c = 0;   // segment start index
local_64 = 0;   // anchor index
local_68 = 0;   // peak Chebyshev

while (iVar13 < path_length - 1) {
    if (iVar13 > 0x13) break;   // hard limit: 20 steps max

    dir = directions[iVar13];

    if (dir == 8) {
        // Tube jump: reset all drift state, advance past it
        iVar13++;
        local_64 = iVar13;
        local_5c = iVar13;
        reset all cumulative offsets to 0;
        continue;
    }
    if (dir == 0xFFFFFFFE) {
        // Skip deleted entries (already compacted out elsewhere)
        iVar13++;
        continue;
    }

    // Normal direction: update cumulative displacement
    d = dir & 7;
    new_cum_x = local_7c.x + g_DirectionOffsets[d].dx;
    new_cum_y = local_7c.y + g_DirectionOffsets[d].dy;

    abs_new_x = abs(new_cum_x);
    abs_new_y = abs(new_cum_y);

    if (abs_new_x < local_70 || abs_new_y < local_6c) {
        // Chebyshev regression: path curved back toward anchor
        if (local_20 == 0 && sStack_1e == 0) {
            // First regression event: reset anchor to current pos
            local_70 = 0; local_6c = 0; local_7c = 0;
            local_64 = iVar13;
            local_20 = cur_x;
        } else {
            // Have accumulated displacement to reroute
            local_5c = local_64;
            dx = cur_x - local_20;
            dy = cur_y - sStack_1e;
            local_78 = (dx, dy);
            local_70 = 0; local_6c = 0; local_7c = 0;
            local_68 = max(abs(dx), abs(dy));
            local_64 = iVar13;
            local_20 = cur_x;
        }
    } else {
        // Chebyshev grew or held: update running peak
        local_7c = (new_cum_x, new_cum_y);
        chebyshev = max(abs_new_x, abs_new_y);
        // Update cur_x/cur_y (current position)
        cur_x += g_DirectionOffsets[d].dx;
        cur_y += g_DirectionOffsets[d].dy;
        local_10 = cur_x; sStack_e = cur_y;

        if (chebyshev <= local_68) {
            // Path is converging on anchor — trigger reroute
            local_18 = local_8;   // save current x
            Path_Find_Split_Anchor(puVar4, iVar13, local_5c, &local_58, &local_18);
            // Compute remaining displacement from split anchor to current
            local_44 = (local_8 - local_18, sStack_6 - sStack_16);
            Path_Reroute_Straight_Line(
                puVar4 + local_58,
                (iVar13 - local_58) + 1,
                &local_18,          // split anchor position
                &local_44,          // displacement target
                param_3,            // FootClass*
                heights[local_58],  // height at split
                0                   // is_end_of_scan = false (mid-window)
            );
        }

        local_68 = chebyshev;
        iVar13++;
        local_70 = abs_new_x;
        local_6c = abs_new_y;
    }
}
```

### Final flush (end-of-window reroute)

After the main loop, if accumulated displacement is non-zero:

```c
if (local_20 != 0 || sStack_1e != 0) {
    dx = cur_x - local_20;
    dy = cur_y - sStack_1e;
    if (max(abs(dx), abs(dy)) < (iVar13 - local_64) - 1) {
        // Remaining segment has more steps than displacement needs — reroute
        Path_Find_Split_Anchor(puVar4, iVar13-1, local_64, &local_58, &local_18);
        Path_Reroute_Straight_Line(
            puVar4 + local_58,
            ((iVar13-1) - local_58) + 1,
            &local_18,
            &displacement,
            param_3,
            heights[local_58],
            1       // is_end_of_scan = true (end-of-window)
        );
    }
}
```

The `is_end_of_scan` flag (`1` vs `0`) controls the steep-slope tolerance inside
`Path_Reroute_Straight_Line`: mid-window calls (`0`) allow 0 steep cells;
end-of-window (`1`) allows up to 3 steep cells.

### Compaction pass

After all rerouting, the function scans the direction array and removes all
`0xFFFFFFFE` deleted entries, compacting in-place:

```c
iVar3 = 0;
for each entry in directions[0 .. path_length-1]:
    if entry != 0xFFFFFFFF (sentinel) and entry != 0xFFFFFFFE (deleted):
        directions[iVar3] = entry
        iVar3++
// Fill remainder with 0xFFFFFFFF
for i in iVar3 .. path_length:
    directions[i] = 0xFFFFFFFF
// Update stored path length
param_2[2] = iVar3 + 1
```

## Key invariants

**1. 20-step hard limit** (verified from `decompile_function 0x0042B7F0`):
```c
if (0x13 < iVar13) break;   // iVar13 > 19 → exit
```
The loop body never processes step index 20 or beyond.

**2. Direction 8 resets all drift state** (verified from decompile):
```c
if (local_24 == 8) {
    local_78 = 0; local_4c = 0; local_48 = 0;
    local_68 = 0; local_70 = 0; local_6c = 0;
    local_7c = 0; local_20 = 0; sStack_1e = 0;
    local_64 = iVar13;
    local_5c = iVar13;
}
```
Bridge/tube jumps are never smoothed; they restart the drift tracking from the exit
coordinate.

**3. `is_end_of_scan` parameter distinction** (verified from decompile):
- Mid-window call: `Path_Reroute_Straight_Line(..., 0)` — 7th argument = 0
- End-of-window call: `Path_Reroute_Straight_Line(..., 1)` — 7th argument = 1

This is `param_7` in `Path_Reroute_Straight_Line @ 0x0042BE20`; it controls
whether 0 or 3 steep cells are tolerated during segment validation.

## Self-proof (3 claims re-verified)

**Claim 1:** Sole caller is `AStar_main_loop @ 0x00429A90`.
Verified via `get_function_callers 0x0042B7F0` → `AStar_main_loop @ 00429a90` only.

**Claim 2:** Two callees only: `Path_Find_Split_Anchor @ 0x0042BCA0` and
`Path_Reroute_Straight_Line @ 0x0042BE20`.
Verified via `get_function_callees 0x0042B7F0` → exactly those two functions.

**Claim 3:** Slope-check enable gate constant at `0x007E3810` = 1e-5 (double).
Verified via `read_memory 0x007E3810 8` → `f1 68 e3 88 b5 f8 e4 3e` = IEEE-754
double = 1e-5. (This gate lives inside `Path_Reroute_Straight_Line`; confirmed
address is consistent with prior report §8.4.)

## Globals referenced

| Global | Address / Symbol | Role |
|--------|-----------------|------|
| `g_DirectionOffsets` | inline symbol | 4-byte per-direction dx/dy table (stride 4 per direction) |
| `0x007E3810` | constant | Slope-check enable gate = 1e-5 (in `Path_Reroute_Straight_Line`) |
| `0x007E3808` | constant | Pass-2 steep threshold = 0.01 (in `Path_Reroute_Straight_Line`) |

## Control flow summary

```
Path_optimize_straight_segments(foot, path_struct, foot2)
├── Load directions[], heights[], path_length, start_pos from path_struct
├── For iVar13 in [0 .. path_length-1], break if iVar13 > 19:
│   ├── dir == 8 → reset all drift state, advance
│   ├── dir == 0xFFFFFFFE → skip (deleted)
│   └── Normal dir:
│       ├── Update cumulative displacement
│       ├── If Chebyshev regression (path curves back):
│       │   └── Reset anchor or record displacement for reroute
│       └── If Chebyshev converges (≤ peak):
│           ├── Path_Find_Split_Anchor(...)
│           └── Path_Reroute_Straight_Line(..., is_end=0)
├── Final flush if non-zero displacement remains:
│   ├── Path_Find_Split_Anchor(...)
│   └── Path_Reroute_Straight_Line(..., is_end=1)
└── Compaction: remove 0xFFFFFFFE entries, fill tail with 0xFFFFFFFF
    └── Update param_2[2] = compacted length + 1
```

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `AStar_main_loop` | `0x00429A90` | task #4 (completed) |
| `Path_Find_Split_Anchor` | `0x0042BCA0` | task #108 (pending) |
| `Path_Reroute_Straight_Line` | `0x0042BE20` | task #109 (pending) |
| `Path_smooth_corners` | `0x0042B210` | task #13 (completed) |

## YELLOW — Unverified

- `local_20` / `sStack_1e` exact semantic: these track accumulated displacement for
  the scan window but their precise role in the anchor-reset vs reroute decision
  path is inferred from the Chebyshev comparisons. The variable names come from
  Ghidra auto-naming; full semantic confirmed by cross-reading with prior report §8.7.
- `g_DirectionOffsets` exact base address: used as a symbol in the decompile but
  the absolute address was not independently read via `read_memory` in this session.
  The 4-byte stride (`uVar9 * 4 + 2` for dy offset) is confirmed from decompile.
- `param_2[4]` (word offset 4): not accessed by this function; layout between
  `[3]` (directions) and `[5]` (heights) is not independently verified.
