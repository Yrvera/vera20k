# MapCoord_Set — Decode Doc
**Proposed Ghidra label:** `MapCoord_Set` (already labeled)
**Address:** `0x0042D470`

## Summary

`MapCoord_Set` at `0x0042D470` assigns x and y components separately to a
`short[2]` cell coordinate struct. It is a trivial setter: writes `param_2` to
`param_1[0]` (x) and `param_3` to `param_1[1]` (y). Body is 22 bytes, no callees.

Called by `Path_smooth_single_segment` (in-scope) and 26 additional callers across
map, locomotion, building-placement, and trigger subsystems.

## Active in YR

**Yes.** Called by `Path_smooth_single_segment @ 0x0042B420` (task #107, completed),
which is on the live `AStar_main_loop → Path_optimize_straight_segments` chain.
Verified via `get_function_callers 0x0042D470`.

## Callers

Verified via `get_function_callers 0x0042D470` — 27 total callers. In-scope:

| Caller | Address | Role |
|--------|---------|------|
| `Path_smooth_single_segment` | `0x0042B420` | Initialize cell coordinate during segment smoothing |

Notable out-of-scope callers: `FlyLocomotionClass__Process`, `MapClass__InitRevealSpiralTable`,
`TriggerAction__Execute`, bridge destruction/repair walkers, building placement.

## Callees

Verified via `get_function_callees 0x0042D470`: **none**. All operations inline.

## Signature

```c
void __thiscall MapCoord_Set(
    undefined2 *param_1,   // ECX: destination coordinate (short[2]: [0]=x, [1]=y)
    undefined2  param_2,   // new x value (short)
    undefined2  param_3    // new y value (short)
)
```

`__thiscall`: `param_1` is passed in ECX (the "this" destination).

## Full Algorithm

```c
void __thiscall MapCoord_Set(undefined2 *param_1, undefined2 param_2, undefined2 param_3)
{
    param_1[0] = param_2;   // x
    param_1[1] = param_3;   // y
}
```

Decompile verbatim (from `decompile_function 0x0042D470`):
```c
*param_1 = param_2;
param_1[1] = param_3;
```

## Coordinate frame

Cell-grid frame: `[0]` = x (column, east = +x), `[1]` = y (row, south = +y).
No bounds check or overflow protection — caller is responsible for valid values.

`MapCoord_Set` differs from `MapCoord_Add` (`0x0042D510`) in that it assigns
absolute values rather than accumulating a delta.

## Self-proof (3 claims verified)

**Claim 1:** In-scope caller `Path_smooth_single_segment @ 0x0042B420` confirmed.
Verified via `get_function_callers 0x0042D470` → 27 callers including
`Path_smooth_single_segment @ 0042b420`.

**Claim 2:** No callees — all inline.
Verified via `get_function_callees 0x0042D470` → "No callees found."

**Claim 3:** Operation is simple assignment: `param_1[0] = param_2; param_1[1] = param_3`.
Confirmed from `decompile_function 0x0042D470`:
```c
*param_1 = param_2;
param_1[1] = param_3;
```

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `Path_smooth_single_segment` | `0x0042B420` | task #107 (completed) |
| All other callers | various | locomotion/map/trigger subsystems |

## YELLOW — Unverified

- **`__thiscall` this-register convention**: verified from Ghidra signature
  (`void __thiscall`). Whether callers use `ECX` as a `MapCoord*` (struct pointer)
  or a raw `short*` was not traced at individual call sites in this session.
- **Relationship to MapCoord struct**: the function operates on `short[2]` directly;
  there may be a `MapCoord` typedef or struct in the Ghidra type system that aliases
  this layout, but that was not verified.
