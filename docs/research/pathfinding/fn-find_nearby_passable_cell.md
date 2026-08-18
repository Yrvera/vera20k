# FootClass__Find_Nearby_Passable_Cell — Decode Doc
**Proposed Ghidra label:** FootClass__Find_Nearby_Passable_Cell (already labeled)

## Summary

`__thiscall` on `FootClass`. Given an origin cell, searches outward in expanding
diamond rings to collect up to 24 passable candidate cells, then selects the best one.
Used when a unit's destination is blocked or a rally/scatter point must be found: MCV
deploy, unit scatter, harvester repositioning, rally-point placement, chrono warp
landing, and many other systems.

**Active in YR: Yes.** Directly called from `FootClass__Find_Path @ 0x004D3920`,
`BuildingClass__ExitObject_Main @ 0x00443C60`, `UnitClass__Mission_Harvest @
0x0073E5E0`, and ~45 other callers (verified via `get_function_callers 0x0056DC20`).

---

## Signature

Source: `decompile_function 0x0056DC20`

```c
void __thiscall
FootClass__Find_Nearby_Passable_Cell(
    int    param_1,     // FootClass* this (ECX)
    undefined4 *param_2, // OUT: result cell (packed short x, short y)
    short *param_3,     // origin cell (short[2]: x=*param_3, y=param_3[1])
    undefined4 param_4, // SpeedType (locomotor speed category)
    int    param_5,     // zone ID (-1 = no zone filter; 0xFFFF → -1)
    undefined4 param_6, // locomotor type
    undefined4 param_7, // bridge-aware mode (bool)
    undefined4 param_8, // foundation width
    undefined4 param_9, // foundation height
    undefined4 param_10,// overlay check flag
    char   param_11,    // check height match (bool)
    char   param_12,    // check cell occupants (bool)
    char   param_13,    // reject bridge cells (bool; inverted: 0=reject, nonzero=allow)
    short *param_14,    // target cell for closest-candidate selection; (0,0) → random
    char   param_15,    // skip first quadrant (bool)
    char   param_16     // check occupancy rect (bool)
)
```

---

## Behavioral analysis

### Step 1 — Zone normalization and origin height

```c
if (param_5 == 0xffff) param_5 = -1;
```

Zone 0xFFFF is the sentinel for "no zone filter" and is normalized to -1 (verified in
decompile `0x0056DC20`: `if (param_5 == 0xffff) { param_5 = -1; }`).

Origin cell height (`local_1bc`) = `CellClass+0x11B` (signed byte). If bridge-aware
mode (`param_7 != 0`) AND the origin cell has `CellClass+0x140 & 0x100` set (bridge
structural flag), height is raised by +4. This prevents bridge-cell height from
causing false height-mismatch rejections.

### Step 2 — Search radius

```c
local_1c0 = this[+0xF4] + this[+0xF8];   // Speed + SightRange
if (local_1c0 > 0x20) local_1c0 = 0x20;  // cap at 32
```

`FootClass+0xF4` = cached Speed, `+0xF8` = cached SightRange. Hard cap 32 cells.
Verified in decompile: `local_1c0 = (undefined4 *)(*(int *)(param_1 + 0xf4) + *(int *)(param_1 + 0xf8))`.

### Step 3 — Diamond ring expansion (collect up to 24 candidates)

The function iterates ring radius `r` from 0 to `search_radius-1`. Each ring produces
cells in four quadrants (NE, NW, SW, SE faces of the diamond). The loop breaks early
when 24 candidates are found (`local_1d4 == 0x18`) or when `local_1d5` (found-a-direct
flag) is set and the current ring is complete.

Each candidate cell is tested against:
1. `TechnoClass__IsOnScreen` — must be visible on-screen.
2. `CellRect__CheckPassability(cell, width, height, speed, zone, loco, -1, bridge, overlay)` — terrain, zone, and overlay passability.
3. Height match (if `param_11`): `|candidate_height - origin_height| < 2`.
4. Occupant check (if `param_12`): `TechnoClass__Is_Current_Cell_Obstacle_Free`.
5. Bridge cell rejection (if `param_13 == 0`): `CellClass+0x140 & 0x100` must be clear.
6. Occupancy rect (if `param_16`): `CellRect__CheckOccupancy`.

Passing candidates are stored in `local_120[48]` (max 24 packed short-pairs).

Bridge-aware path check (`param_7 == 0`): after storing, calls `FUN_006D6410`
(height-corrected cell snap) on the candidate's lepton center `(x*0x100+0x80,
y*0x100+0x80, 0)`. If the snapped cell differs from the candidate cell, it is an
"indirect" candidate and `local_1d5` is NOT set. Only "direct" candidates set
`local_1d5`.

### Step 4 — Selection: direct vs indirect partition

After the collection loop, passing candidates are re-partitioned:
- **Direct** (`local_c0[]`): snapped cell == candidate cell.
- **Indirect** (`local_60[]`): snapped cell != candidate cell.

### Step 5 — Final selection

If `param_14` == null cell `{0, 0}` (checked by comparing with `DAT_00ABD480`):
→ **random selection** using `g_CurrentFrameCounter % count`.
  - If any direct candidates: pick from indirect array (off-by-0x18 index: `local_60[frame % direct_count - 0x18]`).
  - Else: pick from indirect array: `local_60[frame % indirect_count]`.

If `param_14` is a real target cell:
→ **closest-to-target selection**: Euclidean distance via `Sqrt_Approx`; picks the
  candidate (from direct pool if available, else indirect) with smallest distance.

If no candidates found: output is `DAT_00ABD480` (null cell sentinel).

---

## Key struct field accesses

| Owner | Offset | Type | Meaning | Verified |
|-------|--------|------|---------|---------|
| `FootClass` (this) | `+0xF4` | int | cached Speed | decompile `param_1+0xf4` |
| `FootClass` (this) | `+0xF8` | int | cached SightRange | decompile `param_1+0xf8` |
| `CellClass` | `+0x11B` | signed byte | cell height level | decompile `puVar5[0x11b]` |
| `CellClass` | `+0x140` | uint flags | bit 0x100 = bridge structural; bit 0x200 = bridge orientation | decompile `*(uint*)(puVar5+0x140) & 0x100` |
| Cell array | `g_CellArray_Base + index*4` | CellClass** | cell lookup by packed index `y*0x200 + x` | decompile direct access pattern |

> Frame note: `param_1` is `int` (direct byte offsets).

---

## Globals

| Symbol | Address | Role |
|--------|---------|------|
| `g_CellArray_Base` | (runtime) | Cell pointer array; index = `y*0x200+x`; valid range `0..0x3FFFF` |
| `DAT_00ABD480` | `0x00ABD480` | Null cell sentinel {0,0}; used as output when no candidates found |
| `DAT_00ABDC74` / `DAT_00ABDC50` | `0x00ABDC74/50` | Out-of-bounds cell fallback (verified decompile: written when index out of `0..0x3FFFF`) |
| `g_CurrentFrameCounter` | (runtime) | Current simulation frame; used for random candidate selection |

---

## Callers (key subset)

Verified via `get_function_callers 0x0056DC20` — 47 total callers. Key pathfinding-chain callers:

| Caller | Address | Context |
|--------|---------|---------|
| `FootClass__Find_Path` | `0x004D3920` | Called when primary destination is blocked |
| `FUN_00500200` | `0x00500200` | Wrapper: reads zone, randomizes start quadrant, calls this |
| `BuildingClass__ExitObject_Main` | `0x00443C60` | Find exit cell for spawned unit |
| `UnitClass__Mission_Harvest` | `0x0073E5E0` | Harvester repositioning when ore field blocked |

---

## Callees (in-scope)

| Function | Address | Role | Scope |
|----------|---------|------|-------|
| `TechnoClass__IsOnScreen` | — | Must be on-screen to be considered | Out-of-scope |
| `CellRect__CheckPassability` | — | Full passability check: terrain, zone, overlay | Out-of-scope |
| `TechnoClass__Is_Current_Cell_Obstacle_Free` | — | Occupant safety check | Out-of-scope |
| `CellRect__CheckOccupancy` | — | Occupancy rect check | Out-of-scope |
| `FUN_006D6410` | `0x006D6410` | Height-corrected cell snap (slope-aware) | Out-of-scope — cell-system |
| `Sqrt_Approx` | — | Euclidean distance for closest candidate | Out-of-scope — utility |

---

## Cross-reference

Existing exhaustive analysis: `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md` (586 lines;
covers all 16 parameters in detail, full search pattern, exact ring geometry,
caller-parameter matrix). This decode doc cross-checks and confirms all key claims
from that report against the live decompile.

Discrepancy note: the existing report §2 states `param_13 != 0` means "allow bridge
cells." The live decompile confirms: `if (param_13 != '\\0' || ((*(uint *)(puVar5 + 0x140) & 0x100) == 0))` — i.e., `param_13 nonzero` bypasses the bridge-cell check (allow
bridge), `param_13 == 0` rejects bridge cells. Logic is inverted from the parameter
name "reject bridge cells." This matches the existing report. No disparity.

---

## Self-Proof (3 Claims Verified This Session)

1. **47 callers total including `FootClass__Find_Path @ 0x004D3920`** — confirmed via `get_function_callers 0x0056DC20` which returned 47 named callers including `FootClass__Find_Path @ 004d3920`, `BuildingClass__ReleaseDockedHarvester @ 004595c0`, `ChronoSphere__WarpUnitsAtCell @ 0065ec30`, and `UnitClass__Mission_Harvest @ 0073e5e0`.

2. **Search radius: `FootClass+0xF4 + FootClass+0xF8`, capped at 32 (`0x20`)** — confirmed from fresh decompile: `local_1c0 = (undefined4 *)(*(int *)(param_1 + 0xf4) + *(int *)(param_1 + 0xf8)); if (0x20 < (int)local_1c0) { local_1c0 = (undefined4 *)0x20; }`. Verified via `decompile_function 0x0056DC20`.

3. **Max 24 candidates (`local_1d4 == 0x18`) with `Sqrt_Approx` used for distance selection** — confirmed from fresh decompile: early-exit condition `if (local_1d4 == 0x18) goto LAB_0056e5b3` appears 4 times (once per diamond segment), and `fVar15 = (float10)Sqrt_Approx((double)local_1cc)` in target-present selection. Cross-verified: `Sqrt_Approx @ 004cac40` appears in `get_function_callees 0x0056DC20`.

---

## YELLOW — Unverified

- The random-selection index arithmetic `local_60[frame % direct_count - 0x18]` accesses
  the indirect array (offset -0x18 = -24 = one full array length below the direct array
  start). This looks like a Ghidra decompile alias: `local_c0` and `local_60` are two
  separate 24-entry arrays on the stack. The -0x18 index is likely the decompile's
  representation of wrapping from `local_c0[]` back into `local_60[]`. Functional
  meaning is clear; exact array aliasing needs disassembly confirmation if implementation
  requires byte-exact fidelity.
- `FUN_00501AC0` called at the top of `FUN_00500200` with `uVar5` (random 1–4): purpose
  not traced in this session (out of scope for the Find_Nearby_Passable_Cell doc itself;
  belongs in task #114 decode-fn_500200).
