# MapClass__Get_Slope_Cost_At_Cell — Decode Doc
**Proposed Ghidra label:** `MapClass__Get_Slope_Cost_At_Cell` (already labeled)
**Address:** `0x0056BCD0`

## Summary

`MapClass__Get_Slope_Cost_At_Cell` at `0x0056BCD0` looks up a slope traversal cost
for a given cell and speed type. It reads two signed slope components from
`CellClass+0x24` (low word and high word), applies a signed floor-divide by 4 to
each to get table indices, and returns a value from a 2D cost table embedded in a
`MapClass`-derived struct at offset `+0x59F0`.

The return value is an `int` slope cost. Callers multiply it by a per-unit speed
factor and compare to 0.01 to decide if a cell is "too steep" for straight-line
rerouting.

No callees — all operations inline. Body is 109 bytes (`0x0056BCD0–0x0056BD3C`).

## Active in YR

**Yes.** Called by `Path_Reroute_Straight_Line @ 0x0042BE20` and
`Path_smooth_single_segment @ 0x0042B420`, both of which are on the live
`AStar_main_loop → Path_optimize_straight_segments` path-smoothing chain
(verified via `get_function_callers 0x0056BCD0`). Three additional callers
(`FUN_006EA0D0`, `FUN_006EEEA0`, `TeamClass__Find_Best_Target_Building`) are
out of scope.

## Callers

Verified via `get_function_callers 0x0056BCD0`:

| Caller | Address | In-scope? |
|--------|---------|-----------|
| `Path_Reroute_Straight_Line` | `0x0042BE20` | Yes — task #109 |
| `Path_smooth_single_segment` | `0x0042B420` | Yes — task #107 |
| `TeamClass__Find_Best_Target_Building` | `0x006EEBD0` | No — AI targeting |
| `FUN_006EA0D0` | `0x006EA0D0` | No — unknown |
| `FUN_006EEEA0` | `0x006EEEA0` | No — unknown |

## Callees

Verified via `get_function_callees 0x0056BCD0`: **none**. All operations inline.

## Signature

```c
undefined4 MapClass__Get_Slope_Cost_At_Cell(
    short *param_1,   // cell coordinate: *param_1 = x (short), param_1[1] = y (short)
    int    param_2    // pointer to speed-type cost table (MapClass* or sub-struct ptr)
)
```

Return value: `int` slope cost for the given cell and speed type.

## Full Algorithm (annotated from decompile)

```c
undefined4 MapClass__Get_Slope_Cost_At_Cell(short *param_1, int param_2)
{
    // Compute cell array index: y * 512 + x
    int index = param_1[1] * 0x200 + (int)*param_1;

    // Bounds check [0, 0x3FFFF = 512*512 - 1]
    CellClass *cell;
    if (index < 0 || index > 0x3FFFF ||
        (cell = g_CellArray_Base[index]) == NULL) {
        // Out-of-bounds: write coord to DAT_00ABDC74, use fallback cell
        DAT_00ABDC74 = *(int*)param_1;
        cell = &DAT_00ABDC50;
    }

    // Read slope field at CellClass+0x24 (32-bit value, two packed short components)
    int  slope_a = (short)(cell->field_0x24);         // low word
    int  slope_b = (short)(cell->field_0x24 >> 16);   // high word

    // Signed floor-divide each component by 4 to get table indices
    // Pattern: (x + (x >> 31 & 3)) >> 2  =  floor(x / 4) for signed integers
    int idx_a = (slope_a + (slope_a >> 31 & 3)) >> 2;
    int idx_b = (slope_b + (slope_b >> 31 & 3)) >> 2;

    // 2D lookup: table at param_2 + 0x59F0, stride 0x82 (130 columns) in units of 4 bytes
    return *(int*)(param_2 + 0x59F0 + (idx_a + idx_b * 0x82) * 4);
}
```

### CellClass+0x24 — slope field

`cell + 0x24` holds a 32-bit value interpreted as two signed 16-bit components:

| Half | Access | Meaning |
|------|--------|---------|
| Low 16 bits | `(short)(cell->field_0x24)` | Slope component A (x-axis slope?) |
| High 16 bits | `(short)(cell->field_0x24 >> 16)` | Slope component B (y-axis slope?) |

The exact semantic (dx/dy height difference in some fixed unit) is not established in
this session; it is consistent with a per-cell terrain-slope vector.

### Signed floor-divide by 4

The `(x + (x >> 31 & 3)) >> 2` pattern is a standard signed arithmetic floor-divide
by 4. For positive `x`: `x >> 2`. For negative `x`: rounds toward negative infinity
(same as Python floor division), not toward zero.

For slope values in the range `[−256..+255]`, the table index range is `[−64..+63]`
— exactly 128 values. Combined with the stride 0x82 = 130, the table is 130×130 =
16,900 entries of `int` (4 bytes each), totalling 67,600 bytes at `param_2+0x59F0`.

The stride of 130 (not 128) suggests either a sentinel row/column or the value range
is slightly wider than `[−64..+63]`.

### Lookup table location

The table base is `param_2 + 0x59F0`. `param_2` is passed by callers from the
`FootClass` speed-type field (`piVar5[0x87]` in `Path_Reroute_Straight_Line`, which
is `FootClass+0x21C` given `int*`-typed pointer). The table likely encodes
passability cost as a function of (speed_type, slope_a_index, slope_b_index).

Observed usage pattern (from `Path_Reroute_Straight_Line` decompile):
```c
int speed_type = piVar5[0x87];   // FootClass+0x21C
MapClass__Get_Slope_Cost_At_Cell(&coord, speed_type);
```

### Out-of-bounds handling

When the computed index falls outside `[0, 0x3FFFF]` or the cell pointer is null:
- Writes the raw coordinate to `DAT_00ABDC74`
- Uses the fallback cell `DAT_00ABDC50` (same out-of-bounds fallback used in
  `FootClass__Find_Nearby_Passable_Cell`)

## Self-proof (3 claims verified)

**Claim 1:** In-scope callers are `Path_Reroute_Straight_Line @ 0x0042BE20` and
`Path_smooth_single_segment @ 0x0042B420`.
Verified via `get_function_callers 0x0056BCD0` → 5 total callers; two are the
in-scope path-smoothing functions.

**Claim 2:** No callees — all inline.
Verified via `get_function_callees 0x0056BCD0` → "No callees found."

**Claim 3:** Cell index formula `y * 0x200 + x`, bounds `[0, 0x3FFFF]`, table
offset `+0x59F0`, stride `0x82`.
Confirmed from `decompile_function 0x0056BCD0`:
```c
iVar1 = param_1[1] * 0x200 + (int)*param_1;     // y*512 + x
(iVar1 < 0) || (0x3ffff < iVar1)                 // bounds check
param_2 + 0x59f0 + (idx_a + idx_b * 0x82) * 4   // table access
```

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `Path_Reroute_Straight_Line` | `0x0042BE20` | task #109 (completed) |
| `Path_smooth_single_segment` | `0x0042B420` | task #107 (completed) |
| `g_CellArray_Base` | runtime | Map cell array — global |
| `DAT_00ABDC74` | `0x00ABDC74` | Out-of-bounds coord fallback register |
| `DAT_00ABDC50` | `0x00ABDC50` | Fallback cell object |

## YELLOW — Unverified

- **`CellClass+0x24` semantic name**: the two slope components are verified to be
  packed at `+0x24` (confirmed from `puVar2 + 0x24` in decompile). Their semantic
  names (e.g., `slope_x`/`slope_y`, `ramp_height_dx`/`dy`) are not established in
  this session.
- **Table entry meaning**: the return value is used as a slope cost multiplied by
  `FootClass__Get_Slope_Speed_Factor` and compared to 0.01. Whether the table
  stores integer counts, fixed-point fractions, or raw height deltas is not traced
  in this session.
- **`param_2` identity**: assumed to be the speed-type value from `FootClass+0x21C`
  (as observed in `Path_Reroute_Straight_Line`). Whether all callers pass the same
  kind of parameter was not verified across all 5 callers.
- **Stride 130 vs expected 128**: the table stride of 0x82 = 130 exceeds the
  computed value-range of 128 (−64..+63). The 2 extra entries per row may be
  sentinels, alignment padding, or the slope range is actually −64..+65 for some
  cells. Not verified.
