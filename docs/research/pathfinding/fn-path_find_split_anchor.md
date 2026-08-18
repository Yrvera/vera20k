# Path_Find_Split_Anchor — Decode Doc
**Proposed Ghidra label:** `Path_Find_Split_Anchor` (already labeled)
**Address:** `0x0042BCA0`

## Summary

`Path_Find_Split_Anchor` at `0x0042BCA0` scans a window of the direction array
**backwards** (from high index toward low), computing cumulative Chebyshev displacement
from the window start. It finds the first **valley-then-new-peak** inflection point in
that displacement curve and returns it as a split anchor index and cell coordinate.

This is a sub-helper of `Path_optimize_straight_segments`: when a candidate straight
segment can't be rerouted in one pass, the optimizer calls this function to locate a
natural subdivision point — the furthest-away cell that precedes a local maximum —
then recurses on each half.

Returns via output parameters (no return value); no heap allocation, no callees.

## Active in YR

**Yes.** Sole caller is `Path_optimize_straight_segments @ 0x0042B7F0`, which is on
the live `AStar_main_loop → Path_optimize_straight_segments` chain (verified via
`get_function_callers 0x0042BCA0`).

## Callers

Verified via `get_function_callers 0x0042BCA0`:

| Caller | Address | Role |
|--------|---------|------|
| `Path_optimize_straight_segments` | `0x0042B7F0` | Segment subdivision: find split point for recursive reroute |

## Callees

Verified via `get_function_callees 0x0042BCA0`: **none**. All operations are inline
(direction table reads, accumulation, comparison).

## Signature

```c
void Path_Find_Split_Anchor(
    int       *param_1,   // direction array base pointer
    int        param_2,   // start index (high end of scan window)
    int        param_3,   // end index (low end of scan window, inclusive lower bound)
    int       *param_4,   // OUT: split anchor index
    undefined4 *param_5   // IN/OUT: accumulated cell coordinate (packed short: low=x, high=y)
)
```

Not a `__thiscall`. Free function — no `PathfinderClass` this-pointer.

`param_5` is both input (initial accumulated coordinate before the window) and output
(coordinate of the anchor cell after the anchor step).

## Full Algorithm (annotated)

```c
void Path_Find_Split_Anchor(int *param_1, int param_2, int param_3,
                             int *param_4, undefined4 *param_5)
{
    // local_c = current accumulated cell coord (packed short: low=x, high=y)
    local_c = *param_5;
    int dx_sum = 0;       // sVar7: cumulative x displacement from window start
    int dy_sum = 0;       // sVar3: cumulative y displacement from window start
    int local_18 = param_2; // scan cursor, decrements from param_2 to param_3
    bool past_peak = false;  // bVar1: set once we observe a distance decrease
    int max_dist = 0;     // local_8: max Chebyshev distance seen so far

    if (param_3 > param_2) {
        // Window is empty — fall through to default output
        goto done;
    }

    // Advance ptr to param_1[param_2]
    param_1 = (int *)((int)param_1 + param_2 * 4);

    do {
        if (*param_1 != -2) {   // -2 = deleted/compacted entry, skip
            // Reverse direction: rotate 180° to walk the segment backwards
            uint rev_dir = (*param_1 - 4u) & 7;

            // Accumulate displacement (backwards direction)
            short dx = g_DirectionOffsets[rev_dir].dx;   // stride-4 table, short at +0
            short dy = g_DirectionOffsets[rev_dir].dy;   // short at +2

            dx_sum += dx;
            dy_sum += dy;

            // Advance accumulated cell coordinate (still going backwards in path space)
            short cur_x = (short)local_c + dx;   // sVar6
            short cur_y = (short)(local_c >> 16) + dy;  // local_14._2_2_
            local_c = PACK(cur_y, cur_x);
            local_14 = local_c;

            // Chebyshev distance from window start
            int abs_dy = abs(dy_sum);
            int abs_dx = abs(dx_sum);
            int chebyshev = max(abs_dx, abs_dy);

            if (max_dist < chebyshev) {
                // New maximum distance
                max_dist = chebyshev;
                if (past_peak) {
                    // Valley followed by new peak → split anchor found
                    // Un-reverse direction (rotate another 180° = forward direction)
                    uint fwd_dir = (rev_dir - 4u) & 7;

                    // Output: index is one past current (path order = higher index)
                    *param_4 = local_18 + 1;

                    // Output coord: one step forward from current accumulated position
                    short anchor_x = cur_x + g_DirectionOffsets[fwd_dir].dx;
                    short anchor_y = cur_y + g_DirectionOffsets[fwd_dir].dy;
                    *param_5 = PACK(anchor_y, anchor_x);
                    return;
                }
            } else {
                // Distance stalled or decreased → we're past a local peak
                past_peak = true;
            }
        }
        local_18--;
        param_1--;   // step backwards through array
    } while (param_3 <= local_18);

done:
    // No inflection found — use end of window as anchor
    *param_4 = param_3;
    *param_5 = local_c;
}
```

### Direction encoding: the 180° rotation trick

The direction array stores 0–7 compass directions (0=N, 1=NE, 2=E, … 7=NW).
To walk a path segment **backwards**, each forward direction `d` is reversed to
`(d - 4) & 7` (opposite direction), which maps 0↔4, 1↔5, 2↔6, 3↔7.

The output anchor cell needs to step **forward** by one: the forward direction is
recovered by applying the same `−4 & 7` transformation to the already-reversed
`rev_dir`, giving `(rev_dir - 4) & 7 = ((d - 4) - 4) & 7 = d & 7` — the original
forward direction.

### Packed coordinate layout

`local_c` (`*param_5`) stores a packed 32-bit coordinate:
- Low 16 bits = X (cell column)
- High 16 bits = Y (cell row)

The Ghidra decompile uses `CONCAT22(high, low)` and reads via `(short)local_c` (low
half) and `local_14._2_2_` (high half). This matches the `MapCoord` layout used
throughout the pathfinding subsystem.

### Chebyshev distance accumulation

```c
uVar5 = (int)sVar3 >> 0x1f;          // arithmetic sign extension
iVar4 = ((int)sVar3 ^ uVar5) - uVar5; // abs(dy_sum): CDQ/XOR/SUB pattern
uVar5 = (int)sVar7 >> 0x1f;
iVar2 = ((int)sVar7 ^ uVar5) - uVar5; // abs(dx_sum)
if (iVar2 <= iVar4) { iVar2 = iVar4; } // iVar2 = max(abs_dx, abs_dy) = Chebyshev
```

This is the same signed-abs pattern used in `PathfinderClass__EstimateZoneCost`
and `Path_optimize_straight_segments` — confirmed from `decompile_function 0x0042BCA0`.

### Valley-then-peak detection (inflection logic)

The scan looks for a point where:
1. Distance has previously decreased or stalled (`past_peak = true`), AND
2. Distance is now strictly greater than the previous maximum.

This means: the function finds a segment of the path that first curves away from the
straight line and then curves back — a natural waypoint for subdivision.

On exit without finding an inflection: the endpoint (`param_3`) is returned as the
anchor and `param_5` holds the accumulated coordinate at that endpoint.

### `g_DirectionOffsets` usage

Address `0x0089F688` (verified in prior session via assembly tracing). Stride 4:
- `+0`: `short dx` (x delta, signed)
- `+2`: `short dy` (y delta, signed)

Access pattern from decompile:
```c
*(short *)(&g_DirectionOffsets + uVar8)          // dx: base + dir*4 + 0
*(short *)((int)&g_DirectionOffsets + uVar8*4+2) // dy: base + dir*4 + 2
```

## Self-proof (3 claims verified)

**Claim 1:** Sole caller is `Path_optimize_straight_segments @ 0x0042B7F0`.
Verified via `get_function_callers 0x0042BCA0` → exactly one caller returned.

**Claim 2:** No callees — all inline.
Verified via `get_function_callees 0x0042BCA0` → "No callees found."

**Claim 3:** Backwards scan: `param_1` advanced by `param_2 * 4` then decremented;
direction reversed via `(dir - 4) & 7`; output index is `local_18 + 1`.
Confirmed from `decompile_function 0x0042BCA0`:
```c
param_1 = (int *)((int)param_1 + param_2 * 4);
uVar8 = *param_1 - 4U & 7;           // reverse direction
*param_4 = local_18 + 1;             // anchor = one past current scan position
local_18 = local_18 + -1;            // cursor decrements
param_1 = param_1 + -1;              // pointer decrements
```

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `Path_optimize_straight_segments` | `0x0042B7F0` | task #14 (completed) |
| `g_DirectionOffsets` | `0x0089F688` | map/global table — not pathfinding-class |

## YELLOW — Unverified

- **`param_5` input semantics**: the caller (`Path_optimize_straight_segments`)
  passes an accumulated cell coordinate built up across multiple iterations. The
  exact cell that `*param_5` represents on entry (start of window? end of window?
  running cursor?) was not traced through the caller's call site in this session.
  The doc treats it as "accumulated coordinate at the high end of the window" based
  on the decompile reading, but the caller trace was not done.
- **Scan direction upper-vs-lower bound**: the decompile uses `param_3 <= local_18`
  as the continue condition (scan while cursor >= param_3). The doc states param_2 is
  the high index and param_3 is the low bound. This is consistent with the pointer
  advancing to `param_1[param_2]` and decrementing, but the caller's argument order
  was not verified in this session.
- **`-2` deleted-entry sentinel**: `if (*param_1 != -2)` matches the `0xFFFFFFFE`
  compacted-entry value established in `Path_optimize_straight_segments` decode doc
  (task #14). Not re-verified from the binary in this session.
