# Path_Reroute_Straight_Line — Decode Doc
**Address:** `0x0042BE20`
**Proposed Ghidra label:** `Path_Reroute_Straight_Line`
**Active in YR:** Yes — called exclusively by `Path_optimize_straight_segments @ 0x0042B7F0`, which is part of the post-A* path smoothing pipeline.

## Summary

Given a displacement `(dx, dy)` between two waypoints on a reconstructed path, computes a straight-line direction sequence to connect them: `diagonal_steps` (midpoint direction) + `straight_steps` (primary axis direction). Validates each candidate cell along the route for passability (via virtual vtable call) and slope cost threshold. Retries once with swapped directions (swap `local_34` ↔ `local_30`) on failure. Writes the direction sequence and step counts into output parameters on success.

Returns 1 (success) or 0 (failure).

---

## Signature

```c
uint Path_Reroute_Straight_Line(
    uint       *param_1,    // direction output buffer (written on success)
    int         param_2,    // total segment length (number of direction slots)
    undefined4 *param_3,    // in/out: current MapCoord (packed x, y)
    short      *param_4,    // delta vector (dx = *param_4, dy = param_4[1])
    int        *param_5,    // FootClass* (for vtable Can_Enter_Cell + slope factor)
    int         param_6,    // reference height for bridge ramp check
    char        param_7     // slope-strict mode: 0 = zero steep cells allowed; nonzero = up to 3
)
// Returns: 1 on success, 0 on failure
```

Verified via `decompile_function 0x0042BE20`.

---

## Decompilation Key Excerpts

```c
// Direction decomposition from (dx, dy)
local_34 = min(abs(dx), abs(dy));   // diagonal step count
local_30 = max(abs(dx), abs(dy)) - local_34;  // straight step count

// Slope speed factor threshold
fVar14 = FootClass__Get_Slope_Speed_Factor();
bVar4 = (fVar14 > _DAT_007e3810);   // enable slope checks if factor > threshold

// Per-cell validation loop:
iVar9 = (**(code **)(*piVar5 + 0x1ac))(iVar8, local_2c_dir, iVar7, 0, 1);
if (((iVar9 != 0) || ((*(uint *)(iVar8 + 0x140) & 0x40000) != 0)) ||
   (steep = MapClass__Get_Slope_Cost_At_Cell(&ref_coord, speed_type),
   _g_ImpassableSpeedThreshold <= (double)steep * fVar14)) {
    local_2c++;  // increment steep-cell failure count
}
// Failure gate: local_2c >= 4 (lenient) or local_2c >= 1 (strict, param_7==0)

// Retry: swap local_28 <-> local_24 (direction pair) and local_34 <-> local_30 (counts)
// Maximum 2 orderings total; returns 0 if both fail

// Success: write replacement directions + sentinel
for (i = 0; i < local_34; i++) param_1[i] = local_28;      // diagonal direction
for (i = 0; i < local_30; i++) param_1[local_34+i] = local_24; // straight direction
for (remainder) param_1[...] = 0xFFFFFFFE;                  // NOP sentinel padding
return 1;
```

Verified via `decompile_function 0x0042BE20`.

---

## Behavioral Analysis

### Step 1 — Direction encoding from (dx, dy)

Given delta `(dx = *param_4, dy = param_4[1])`, maps to two compass directions:

- `local_28` = primary (diagonal) direction based on `sign(dx)` and `sign(dy)` → one of NE/SE/SW/NW
- `local_24` = secondary (straight) direction based on whether `|dx| <= |dy|` → N/S or E/W

```
diagonal_steps (local_34) = min(|dx|, |dy|)
straight_steps (local_30) = max(|dx|, |dy|) - min(|dx|, |dy|)
total cells = local_34 + local_30
```

This is the standard Chebyshev decomposition for straight-line walking on an 8-connected grid.

### Step 2 — Slope speed factor

```c
fVar14 = FootClass__Get_Slope_Speed_Factor();
bVar4 = (fVar14 > _DAT_007e3810);  // enable slope checks if factor > threshold
iVar10 = piVar5[0x87];             // FootClass+0x21C = SpeedType
```

If slope checks are disabled (flat terrain type or below threshold), the steep-cell counter stays 0 and never triggers rejection.

### Step 3 — Per-cell passability check loop

For each candidate cell (total: `local_34 + local_30` cells):

1. **Passability vtable call** at `FootClass vtable+0x1AC`: `(*this->vtable[0x1AC])(cell*, dir, level, 0, 1)`. Nonzero = impassable.
2. **A* cost marker flag**: `CellClass+0x140 & 0x40000 != 0` → reject.
3. **Slope cost threshold**: `MapClass__Get_Slope_Cost_At_Cell() * slope_speed_factor >= _g_ImpassableSpeedThreshold` → increment steep counter `local_2c`.

Height tracking: same bridge-ramp `+4` height jump logic as `Path_smooth_single_segment` — reads `CellClass+0x11B` (height level); if `current_level - step_level == 4` AND `CellClass+0x140 & 0x100` (bridge), adjusts height by +4.

### Step 4 — Strictness flag (param_7)

| `param_7` value | Threshold | Meaning |
|---|---|---|
| 0 (strict) | `local_2c >= 1` | Any single steep cell aborts the route |
| nonzero (lenient) | `local_2c >= 4` | Up to 3 steep cells tolerated |

Verified from decompile: the conditional is `(param_7 == '\0') && (local_2c >= 1)` for strict rejection, and `local_2c >= 4` for lenient.

### Step 5 — Retry with swapped direction order

On first-pass failure: swaps `local_28 ↔ local_24` (diagonal ↔ straight directions) and `local_34 ↔ local_30` (their step counts), then reruns the validation loop. This tests leading with the straight direction instead of the diagonal direction. Only one retry; counter `local_10` caps at 2 orderings total.

### Step 6 — Output on success

Writes the direction sequence into `param_1`:
- `local_34` entries of `local_28` (diagonal direction)
- `local_30` entries of `local_24` (straight direction)
- Remaining slots (`param_2 - local_34 - local_30`) filled with sentinel `0xFFFFFFFE`

Returns 1.

### Step 7 — Output on failure

Returns 0. `param_1` may be in a partially-written state; caller must check return value before consuming.

### Tiberian Sun filter

No TS-only gate. Called from `Path_optimize_straight_segments` which is part of the live YR path-smoothing pipeline.

---

## Struct Field Accesses

| Struct | Offset | Type | Meaning |
|---|---|---|---|
| `FootClass` (`param_5`) | `vtable+0x1AC` | virtual fn | Passability check: `(*this+0x1AC)(cell*, dir, level, 0, 1)` |
| `FootClass` (`param_5`) | `+0x21C` (`[0x87]*4`) | `int` | SpeedType; passed to slope cost lookup |
| `CellClass` | `+0x11B` | `signed byte` | Height level |
| `CellClass` | `+0x140` | `uint32` | Flags: bit `0x40000` = A* cost marker (reject); bit `0x100` = bridge structural |

All offsets confirmed from decompile via `decompile_function 0x0042BE20`. `CellClass+0x140` and `FootClass vtable+0x1AC` are cross-consistent with `fn-path_smooth_single_segment.md`.

---

## Globals

| Symbol | Role |
|---|---|
| `g_DirectionOffsets @ 0x0089F688` | Direction delta table; stride 4; dx at +0, dy at +2 (signed shorts) |
| `_DAT_007e3810 @ 0x007E3810` | Slope-enable threshold; `slope_speed_factor > this` enables slope checks |
| `_g_ImpassableSpeedThreshold` | Steep-cell cost threshold; `slope_cost * factor >= this` = steep cell |

---

## Callers

| Caller | Address | Notes |
|---|---|---|
| `Path_optimize_straight_segments` | `0x0042B7F0` | Sole caller; tests each candidate straight segment |

Verified via `get_function_callers 0x0042BE20`.

---

## Callees

| Callee | Address | Role | Scope |
|---|---|---|---|
| `FootClass__Get_Slope_Speed_Factor` | `0x004DC760` | Slope speed factor for this FootClass | Out-of-scope: locomotor/slope |
| `MapClass__Get_CellClass` | `0x005657A0` | CellClass* lookup | Out-of-scope: cell-system |
| `MapClass__Get_Slope_Cost_At_Cell` | `0x0056BCD0` | Slope cost for a cell | In-scope (Task #110) |

Verified via `get_function_callees 0x0042BE20`.

---

## 3-Axis Confidence

| Finding | Content | Identity | Binding |
|---|---|---|---|
| Sole caller: `Path_optimize_straight_segments` | HIGH | HIGH | HIGH — `get_function_callers` |
| Direction decomposition: diagonal = min, straight = max - min | HIGH — directly in decompile | HIGH | HIGH |
| Strictness flag `param_7`: threshold 1 (strict) vs 4 (lenient) | HIGH — decompile shows explicit `(param_7 == '\0') && (local_2c >= 1)` vs `local_2c >= 4` | HIGH | HIGH |
| Retry with swapped directions | HIGH — decompile shows swap + loop re-entry, counter `local_10` caps at 2 | HIGH | HIGH |
| `FootClass vtable+0x1AC` passability call | HIGH — decompile shows `(**(code **)(*piVar5 + 0x1ac))(...)` | MEDIUM — vtable slot identity not independently confirmed | MEDIUM |
| `CellClass+0x140 bit 0x40000` = A* cost marker | HIGH — consistent with `fn-path_smooth_single_segment.md` | HIGH | HIGH |
| `0xFFFFFFFE` sentinel written to remaining slots | HIGH — decompile shows `*puVar13 = 0xfffffffe` in padding loop | MEDIUM — caller consumption of sentinel not verified | MEDIUM |

---

## Self-Proof (3 Claims Verified This Session)

1. **Sole caller is `Path_optimize_straight_segments @ 0x0042B7F0`** — confirmed via `get_function_callers 0x0042BE20` returning exactly one result: `Path_optimize_straight_segments @ 0042b7f0`. Verified via `get_function_callers 0x0042BE20`.

2. **Direction decomposition: diagonal_steps = min(|dx|, |dy|), straight_steps = max(|dx|, |dy|) - min(|dx|, |dy|)** — confirmed from decompile: `local_34` is assigned the minimum of the two absolute deltas; `local_30` is assigned the difference. This is the standard Chebyshev diagonal decomposition. Verified via `decompile_function 0x0042BE20`.

3. **Retry: swaps diagonal/straight direction pair and their counts on first-pass failure, then re-runs the per-cell check loop; only one retry attempted** — confirmed from decompile: after the first walk fails, the code swaps `local_28 ↔ local_24` and `local_34 ↔ local_30`, increments counter `local_10`, and re-enters the validation loop. Counter `local_10 > 1` causes the function to return `local_10 & 0xffffff00 = 0`. Verified via `decompile_function 0x0042BE20`.

---

## YELLOW (Unverified)

| Item | Why unverified | How to verify |
|---|---|---|
| `_DAT_007e3810 @ 0x007E3810` exact value | Address inferred from decompile symbol; not read via `read_memory` this session | `read_memory 0x007E3810` (4 bytes) |
| `_g_ImpassableSpeedThreshold` address | Referenced as threshold constant but address not extracted from decompile | Disassembly at the comparison site to read constant address |
| `FootClass vtable+0x1AC` target function | Virtual dispatch; exact target not confirmed | `read_memory` at vtable base + 0x1AC |
| `0xFFFFFFFE` sentinel consumption | Written to `param_1` padding slots; whether `Path_optimize_straight_segments` skips these is not independently verified | Decompile `Path_optimize_straight_segments @ 0x0042B7F0` |

---

## Companion Docs

- `fn-path_smooth_single_segment.md` — shares `FootClass vtable+0x1AC` passability check and `CellClass+0x140 bit 0x40000` cost marker pattern
- `fn-mapclass_get_slope_cost_at_cell.md` (Task #110) — callee that computes the slope cost checked here
