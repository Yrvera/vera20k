# Path_smooth_single_segment — Decode Doc
**Address:** `0x0042B420`
**Proposed Ghidra label:** `Path_smooth_single_segment`
**Active in YR:** Yes — called exclusively by `Path_smooth_corners @ 0x0042B210` (in-scope), which is called from `AStar_main_loop`'s path post-processing pipeline.

## Summary

Attempts to smooth one segment of a reconstructed path by replacing a pair of corner turns with a straight diagonal step. Given a direction array and a current cell position, it:

1. Computes the "midpoint direction" between the two adjacent directions in the segment.
2. Checks each cell along the candidate diagonal: must be passable (via vtable call), must not have `CellClass+0x140 bit 0x40000` set (A* cost marker), and must have acceptable slope cost (via `MapClass__Get_Slope_Cost_At_Cell`).
3. If all checks pass: overwrites the path direction entries with the midpoint direction (the smoothed diagonal).
4. Returns the updated forward-step count.

If either adjacent direction is 8 (tube), smoothing is skipped entirely and all steps are walked as-is.

---

## Signature

```c
int Path_smooth_single_segment(
    int        *param_1,     // FootClass* (ECX via fastcall); vtable+0x1AC = passability check
    uint       *param_2,     // in/out: direction array pointer (ushort[] from reconstructed path)
    int         param_3,     // cost table pointer (passed to slope cost computation)
    int         param_4,     // forward step count (steps ahead from current position)
    int         param_5,     // backward step count (steps back / smoothing window size)
    undefined4 *param_6      // in/out: current MapCoord (packed short x, y)
)
// Returns: updated remaining step count
```

Verified via `decompile_function 0x0042B420`.

---

## Decompilation Key Excerpts

```c
// Midpoint direction computation
local_c = *param_2;             // start direction (at current position)
uVar1 = param_2[param_4];       // end direction (param_4 steps ahead)
local_2c = (int)(local_c + uVar1) >> 1;  // average direction
// Validate midpoint: must be adjacent (within 1) of both endpoints
if ((local_2c + 1 != uVar1) && (local_2c + 1 != local_c)) {
    local_2c = 0;  // not adjacent: no valid midpoint
}

// Tube bypass: if either endpoint direction is 8, skip smoothing
if ((local_c == 8) || (uVar1 == 8)) {
    // walk (param_4 + param_5) steps via MapCoord_Step_By_Direction
    *puVar6 = param_6;
    return iVar8;  // iVar8 = param_4 + param_5
}

// Advance position forward by (param_4 - param_5) steps using MapCoord_Add
// (handles tube direction 8 specially via MapClass__Get_CellClass + g_TubeArray)

// Slope speed factor for threshold check
fVar11 = FootClass__Get_Slope_Speed_Factor();

// Per-step smoothing check loop (2*param_5 steps):
iVar5 = (**(code **)(*param_1 + 0x1ac))(iVar4, local_2c, iVar8, 0, 1);
// iVar4 = CellClass*; local_2c = midpoint direction; iVar8 = current slope level
if (((iVar5 != 0) || ((*(uint *)(iVar4 + 0x140) & 0x40000) != 0)) ||
   (local_10 = MapClass__Get_Slope_Cost_At_Cell(&local_34, local_14),
   _g_Const_1_0 <= (double)local_10 * local_8)) {
    bVar2 = true;  // candidate cell fails check: abort smoothing
}
// If bVar2 set: abandon this segment (do not write midpoint direction)

// If check passes for all steps: write midpoint direction to direction array
for (; iVar8 != 0; iVar8--) {
    *puVar9 = local_2c;
    puVar9++;
}
```

Verified via `decompile_function 0x0042B420`.

---

## Behavioral Analysis

### Step 1 — Midpoint direction

The two adjacent directions in the segment are `local_c` (start = `param_2[0]`) and `uVar1` (end = `param_2[param_4]`). The midpoint is `(local_c + uVar1) >> 1` (arithmetic right shift = floor division).

If `midpoint + 1 != uVar1 AND midpoint + 1 != local_c`: the midpoint is not adjacent to either endpoint (the turns are too far apart to form a valid diagonal). Set `local_2c = 0` in this case — but this value is still used in the smoothing loop as the replacement direction. `local_2c = 0` as a fallback direction (S) may produce incorrect output; the prior smoothness check would need to catch this.

### Step 2 — Tube bypass (direction 8)

If `local_c == 8` OR `uVar1 == 8`: tube transitions cannot be smoothed. Walk all `param_4 + param_5` steps via `MapCoord_Step_By_Direction` and return. No direction entries are modified.

### Step 3 — Forward advance

Advances the current position by `param_4 - param_5` steps along the direction array. Tube direction (8) is handled: calls `MapClass__Get_CellClass`, reads `CellClass+0x116` (tube index), looks up destination via `g_TubeArray[idx]+0x28`. Non-tube directions use `MapCoord_Add`.

### Step 4 — Passability / slope check loop

For each of the `2 * param_5` candidate cells along the smoothed diagonal (using midpoint direction `local_2c`):

1. **Passability vtable call**: `(**(code **)(*param_1 + 0x1AC))(cell*, midpoint_dir, slope_level, 0, 1)` — calls a virtual method on FootClass (vtable slot at offset `+0x1AC`) which checks whether the cell is passable for the midpoint direction. If returns nonzero → fail.

2. **A* cost marker flag**: if `CellClass+0x140 & 0x40000 != 0` → fail. This bit is set by `PathfinderClass__UpdateBridgePassability` to mark cells near bridge approaches.

3. **Slope cost threshold**: `MapClass__Get_Slope_Cost_At_Cell(&local_34, local_14)` — reads slope cost for the current position vs the start cell height. If `slope_cost * slope_speed_factor >= 1.0` → fail (too steep to smooth through).

Height tracking: after each step, reads `CellClass+0x11B` (Level); if `current_level - step_level == 4` AND `CellClass+0x140 & 0x100` (bridge): sets `iVar8 = step_level + 4` (bridge height adjustment).

If ANY step fails: set `bVar2 = true`, abandon smoothing for this segment.

### Step 5 — Apply smoothing OR walk as-is

If all `2 * param_5` checks passed (`bVar2 == false`):
- Writes `local_2c` (midpoint direction) to `param_4 - param_5` entries in the direction array.
- Updates position by walking the remaining `param_4 - param_5` steps using the (new) directions.
- Returns `param_4 - param_5`.

If any check failed (`bVar2 == true`):
- Abandons smoothing; falls through to walk the original `param_4` steps.
- Returns `param_4`.

### Tiberian Sun filter

No TS-only gate. Called from `Path_smooth_corners` which is called from `AStar_main_loop` — the live YR pathfinding pipeline.

---

## Struct Field Accesses

| Struct | Offset | Type | Meaning |
|---|---|---|---|
| `FootClass` | `vtable+0x1AC` | virtual fn | Passability check for candidate cell/direction; `(*this+0x1AC)(cell*, dir, level, 0, 1)` |
| `CellClass` | `+0x116` | `short` | Tube index (-1 = no tube) |
| `CellClass` | `+0x11B` | `signed byte` | Height level |
| `CellClass` | `+0x140` | `uint32` | Flags: bit `0x40000` = A* cost marker; bit `0x100` = bridge structural |

All offsets confirmed from decompile via `decompile_function 0x0042B420`.

---

## Globals

| Symbol | Role |
|---|---|
| `g_DirectionOffsets @ 0x0089F688` | Direction delta table; stride 4; dx at +0, dy at +2 (both signed shorts) |
| `g_TubeArray @ 0x008B413C` | Tube record array for direction=8 tube transitions |
| `_g_Const_1_0` | Slope cost threshold = 1.0 (float or double constant) |

---

## Callers

| Caller | Address | Notes |
|---|---|---|
| `Path_smooth_corners` | `0x0042B210` | Sole caller; part of the path post-processing pipeline |

Verified via `get_function_callers 0x0042B420`.

---

## Callees

| Callee | Address | Role | Scope |
|---|---|---|---|
| `MapCoord_Set` | `0x0042D470` | Set MapCoord to (0,0) on failed tube lookup | In-scope (Task #112) |
| `MapCoord_Add` | `0x0042D510` | Add direction delta to MapCoord | In-scope (Task #111) |
| `MapCoord_Step_By_Direction` | `0x0042D490` | Step MapCoord by direction index | In-scope (Task #113) |
| `MapClass__Get_Slope_Cost_At_Cell` | `0x0056BCD0` | Slope cost for a cell relative to reference | In-scope (Task #110) |
| `MapClass__Get_CellClass` | `0x005657A0` | CellClass* lookup | Out-of-scope: cell-system |
| `FootClass__Get_Slope_Speed_Factor` | `0x004DC760` | Slope speed factor for this FootClass | Out-of-scope: locomotor/slope |

Verified via `get_function_callees 0x0042B420`.

---

## 3-Axis Confidence

| Finding | Content | Identity | Binding |
|---|---|---|---|
| Sole caller: `Path_smooth_corners` | HIGH | HIGH | HIGH — `get_function_callers` |
| Midpoint direction formula: `(dir_a + dir_b) >> 1` | HIGH — directly in decompile | HIGH | HIGH |
| Tube bypass on direction 8 | HIGH — decompile shows explicit `local_c == 8 || uVar1 == 8` | HIGH | HIGH |
| Passability check: `vtable+0x1AC` virtual call | HIGH — decompile shows `(**(code **)(*param_1 + 0x1ac))(...)` | MEDIUM — vtable slot identity not independently confirmed | MEDIUM |
| `CellClass+0x140 bit 0x40000` = A* cost marker check | HIGH — confirmed in decompile; consistent with `fn-pathfinder_update_bridge_pass.md` | HIGH | HIGH |
| Slope cost threshold = 1.0 | HIGH — `_g_Const_1_0 <= slope_cost * factor` | MEDIUM — `_g_Const_1_0` address not verified in this session | MEDIUM |

---

## Self-Proof (3 Claims Verified This Session)

1. **Sole caller is `Path_smooth_corners @ 0x0042B210`** — confirmed via `get_function_callers 0x0042B420` returning exactly one result.

2. **Tube bypass: direction == 8 causes the function to skip smoothing and walk all steps** — confirmed from decompile: `if ((local_c == 8) || (uVar1 == 8)) { ... return iVar8; }` where `iVar8 = param_4 + param_5`. The entire smoothing decision tree is bypassed. Verified via `decompile_function 0x0042B420`.

3. **`CellClass+0x140 bit 0x40000` is checked as a rejection condition during the candidate walk** — confirmed from decompile: `(*(uint *)(iVar4 + 0x140) & 0x40000) != 0` causes `bVar2 = true` (abort smoothing). Cross-confirmed with `fn-pathfinder_update_bridge_pass.md` which documents this bit as the bridge-approach cost marker. Verified via `decompile_function 0x0042B420`.

---

## YELLOW (Unverified)

| Item | Why unverified | How to verify |
|---|---|---|
| `FootClass vtable+0x1AC` target function | Virtual dispatch; exact target not read this session | `read_memory` at vtable base + 0x1AC |
| `_g_Const_1_0` address | Referenced as a global constant (1.0) but address not extracted from decompile | Check disassembly at the `_g_Const_1_0` comparison site |
| `local_2c = 0` fallback behavior | When midpoint is not adjacent, `local_2c = 0` (direction South) is used — unclear if this produces a correct path or is a dead branch in practice | Trace `Path_smooth_corners` to understand what direction pairs are passed |
| Exact parameter semantics of `param_3` (cost table) | Passed to `MapClass__Get_Slope_Cost_At_Cell` but how it's used as a cost table is in Task #110 | Decode Task #110: `mapclass_get_slope_cost_at_cell` |
