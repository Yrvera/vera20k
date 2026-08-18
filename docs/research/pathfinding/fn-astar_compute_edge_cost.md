# AStar_compute_edge_cost — Decode Doc
**Proposed Ghidra label:** AStar_compute_edge_cost (already labeled)

## Summary

Per-edge cost helper for the A\* main loop. Takes a source cell, destination cell, bridge-layer flag,
and base Can_Enter code, and returns a float edge cost. Called only by `AStar_main_loop @
0x00429A90` for compass directions 0–7; direction 8 (tube/bridge-crossing) bypasses this helper
entirely. The cost formula is layered multiplicatively: base table → code-2 entity prediction →
`0x40000` temporary marker × 4 → bridge flank multiplier. Caller then multiplies by
`PathfinderClass+0x04` and adds direction epsilon.

**Active in YR: Yes.** Single caller: `AStar_main_loop @ 0x00429A90` (verified via
`get_function_callers 0x00429830`). On the live ground/bridge movement path for all FootClass
units. No TS-legacy gate.

---

## Decompilation excerpt

Source: `decompile_function 0x00429830`

```c
float10 __thiscall
AStar_compute_edge_cost(int param_1,    // PathfinderClass* this
                        int *param_2,   // source CellClass* ptr-array (cell coords)
                        int *param_3,   // dest CellClass* ptr-array
                        char param_4,   // bridge_layer flag (non-zero = entering bridge)
                        float param_5)  // Can_Enter_Cell code (as float — actually an int cast)
{
    // 1. Load base cost from table indexed by Can_Enter code
    bool bVar10 = (param_5 == 2); // code 2 = moving friendly unit
    param_5 = g_AStar_EdgeCost_BaseTable[can_enter_code];  // 0x0081870C

    // 2. Code-2 entity prediction branch
    if (bVar10) {
        piVar9 = (param_4 == 0) ? dest->blocker_list_ground  // dest+0xE4
                                : dest->blocker_list_bridge;  // dest+0xE8
        if (pathfinder->urgency == 0) {  // PathfinderClass+0x3C
            // walk blocker's predicted path up to 10 steps
            // → param_5 = 1.0 (clears) or falls to 4.0 (jammed)
        }
        param_5 = 4.0;  // jam cost (if prediction didn't find clearing)
AStar_cost_predict_urgency_override:
        if (pathfinder->urgency == 2) param_5 = 1000.0;  // reroute-around
    }

    // 3. Temporary 0x40000 marker multiplier
    if (dest->flags & 0x40000)          // CellClass+0x140
        param_5 *= 4.0;                 // g_BridgeApproach_CostMult_4_0 @ 0x007E37BC

    // 4. Bridge flank multiplier (only when entering bridge layer AND Pathfinder+0x01)
    if (param_4 != 0 && pathfinder->bridge_flank_enable != 0) {  // Pathfinder+0x01
        // compute direction from dest - source cell coords
        // select flank table based on dest->flags & 0x800 (orientation)
        // read two flanking cells
        if (flank1->flags & 0x100) {            // structural bridge
            if (flank2->flags & 0x100)
                return param_5 * 2.0;           // both bridge
            return param_5 * 1.0;               // one bridge
        }
        return param_5 * 10.0;                  // non-bridge flank
    }

    return param_5;
}
// Caller does: step = returned_edge * *(PathfinderClass+0x04) + epsilon_table[dir]
```

---

## Behavioral analysis

### Exact cost formula for compass directions 0–7

1. `edge = g_AStar_EdgeCost_BaseTable[can_enter_code]` — load base from 8-float table at `0x0081870C`
2. If `can_enter_code == 2` → run code-2 entity prediction branch (see §3.2)
3. If `dest CellClass+0x140 & 0x40000` → `edge *= 4.0`
4. If entering bridge layer (`param_4 != 0`) and `PathfinderClass+0x01 != 0` → apply bridge flank multiplier
5. Return `edge`
6. Caller: `step = edge * *(PathfinderClass+0x04) + DirectionEpsilon[dir]`

All four multiplicative layers stack in this fixed order. Direction epsilon (§3.5) is additive and
outside all multipliers.

### Base cost table (`g_AStar_EdgeCost_BaseTable @ 0x0081870C`)

Verified via `read_memory 0x0081870C` (32 bytes = 8 × float):

| Code | Cost | Semantic |
|------|------|---------|
| 0 | `1.0` | Clear / OK |
| 1 | `1000.0` | Crushable unit |
| 2 | `1.0` (base) | Moving-friendly unit → code-2 branch overrides |
| 3 | `1.0` | Bridge ramp passable |
| 4 | `60.0` | Friendly wall |
| 5 | `20.0` | Enemy block |
| 6 | `8.0` | Friendly stationary |
| 7 | `10000.0` | Impassable (caller rejects before opening node) |

Hex bytes: `0000803f 00007a44 0000803f 0000803f 00007042 0000a041 00000041 00401c46`

### Code-2 prediction branch

Entered only when `can_enter_code == 2`. Selects blocker list based on bridge-layer arg:
- `param_4 == 0` → `dest+0xE4` (ground-layer blocker list head)
- `param_4 != 0` → `dest+0xE8` (bridge-layer blocker list head)

Then checks `PathfinderClass+0x3C` (urgency):
- **urgency 0**: walks up to 10 hops of blocker's predicted path; if blocker will move clear →
  `edge = 1.0`; else → `edge = 4.0` (jammed)
- **urgency 1**: skip prediction, `edge = 4.0` (jammed)
- **urgency 2**: `edge = 4.0` then override to `edge = 1000.0` (reroute-around)

The bridge-layer selector uses an asymmetric height test inside the loop: uses `dest+0xE8`
(bridge list) when `CellClass+0x140 & 0x100` is set AND either the blocker has a height level
difference ≥ 3 from the destination cell OR the blocker is not bridge-flagged.

### Temporary `0x40000` marker multiply

After the code-2 branch joins, the helper reads `dest CellClass+0x140` and tests bit `0x40000`.
If set, `edge *= 4.0` (`g_BridgeApproach_CostMult_4_0 @ 0x007E37BC`).

Verified via `read_memory 0x007E37BC` → `00008040` = `4.0f`. ✓

This marker is **not** a static terrain/cliff flag — it is a temporary per-search destination cost
overlay written by `UpdateBridgePassability @ 0x0042ACF0` and cleared after the A\* search. It
stacks multiplicatively with all prior edge costs:

| Prior edge | After marker (×4) |
|-----------|------------------|
| code-0 clear `1.0` | `4.0` |
| code-2 jam `4.0` | `16.0` |
| code-2 urgency-2 `1000.0` | `4000.0` |
| code-5 enemy `20.0` | `80.0` |
| code-6 stationary `8.0` | `32.0` |

### Bridge flank multiplier

Gated by: `param_4 != 0` (entering bridge layer) AND `PathfinderClass+0x01 != 0`.

When active:
1. Direction computed from `(dest.cell_x - src.cell_x, dest.cell_y - src.cell_y)` → index into
   `DAT_007e3760` table
2. Orientation selector: `dest CellClass+0x140 & 0x800` → chooses `DAT_007e3710` (NS) or
   `DAT_007e3730` (EW) flank offset table
3. Two flank cells read via `dir` and `(dir - 4) & 7`
4. Flank structural test via `flank->CellClass+0x140 & 0x100`

| Flank condition | Multiplier |
|----------------|-----------|
| First flank not structural bridge | `10.0` (`_g_BridgeDiag_NonBridge_10_0 @ 0x007E37B8`) |
| First structural bridge, second not | `1.0` (`_DAT_007e2ac8`) |
| Both flanks structural bridge | `2.0` (`_g_BridgeDiag_BothSides_2_0 @ 0x007E37B4`) |

Constants verified via `read_memory 0x007E37B4` → `00000040 00002041 00008040` = 2.0, 10.0, 4.0. ✓

Combined marker + bridge example: marked code-2 jam on non-bridge-flank bridge entry =
`4.0 × 4.0 × 10.0 = 160.0` before caller epsilon.

### Direction epsilon (caller-side, not in helper)

The caller at `0x00429F8A..0x00429F9D`:
1. Calls `AStar_compute_edge_cost` → gets `edge`
2. Multiplies: `edge *= *(float*)(PathfinderClass+0x04)` (default `1.0f`, verified constructor write)
3. Adds: `edge += g_DirectionEpsilonTable[dir]` (`0x0081872C`)

Direction epsilon values (verified via `read_memory 0x0081872C`):
`[0.001, 0.005, 0.002, 0.006, 0.003, 0.007, 0.004, 0.008, 0.0]` for directions 0–8.

Epsilon is **not** scaled by marker, code-2, bridge flank, or `+0x04`. It is always the final
additive term.

### Direction 8 bypass

The caller at `0x00429F6B` checks: `CMP [ESP+0x18], 8; JZ 0x00429FA3`. When direction is 8 (tube
edge), it jumps past the `AStar_compute_edge_cost` call entirely. Direction-8 edges receive:
- No base table cost
- No code-2 prediction
- No `0x40000` marker multiply
- No bridge flank multiplier
- No `PathfinderClass+0x04` multiply
- No normal direction epsilon

---

## Struct field accesses (frame-annotated)

| Offset | Expression | Frame | Notes |
|--------|-----------|-------|-------|
| `PathfinderClass+0x01` | `*(char*)(param_1+1)` | PathfinderClass instance | bridge-flank enable byte; constructor clears to 0 |
| `PathfinderClass+0x04` | `*(float*)(PathfinderClass+0x04)` | PathfinderClass instance (caller-side) | cost multiplier; constructor writes 1.0f @ `0x0042A6D0` |
| `PathfinderClass+0x3C` | `*(int*)(param_1+0x3C)` | PathfinderClass instance | urgency DWORD: 0=predict, 1=jam, 2=reroute-around |
| `CellClass+0x140` | `*(uint*)(dest+0x140)` | CellClass instance | flags field; bits `0x40000`, `0x100`, `0x800` used |
| `CellClass+0xE4` | `*(int**)(dest+0xE4)` | CellClass instance | ground-layer blocker list head |
| `CellClass+0xE8` | `*(int**)(dest+0xE8)` | CellClass instance | bridge-layer blocker list head |
| `TechnoClass+0x178` | `*(uint*)(blocker+0x178)` | TechnoClass/FootClass instance | next move-target cell index (int[0x178] = offset 0x5e0) |
| `TechnoClass+0x23×4` | `*(char*)(blocker+0x8C)` | TechnoClass instance | bridge-above flag used in prediction height test |

> Frame note: `param_3` (destination) and blocker list entries are CellClass/TechnoClass ptrs.
> All `+0x140` / `+0xE4` / `+0xE8` offsets are direct byte offsets (param type `int*` but used
> with explicit `(uint*)` cast, not ×4 indexing).

---

## Globals / Enums / INI

| Symbol | Address | Value | Role |
|--------|---------|-------|------|
| `g_AStar_EdgeCost_BaseTable` | `0x0081870C` | 8 × float | Base cost table indexed by Can_Enter code |
| `g_DirectionEpsilonTable` | `0x0081872C` | 9 × float | Caller-side direction tiebreak add |
| `g_BridgeApproach_CostMult_4_0` | `0x007E37BC` | `4.0f` | Temporary marker multiply constant |
| `_g_BridgeDiag_NonBridge_10_0` | `0x007E37B8` | `10.0f` | Bridge flank penalty: non-bridge first flank |
| `_g_BridgeDiag_BothSides_2_0` | `0x007E37B4` | `2.0f` | Bridge flank cost: both flanks structural bridge |
| `_DAT_007e2ac8` | `0x007E2AC8` | `1.0f` | Bridge flank cost: first bridge, second not |
| `DAT_007e3760` | `0x007E3760` | direction→index table | Maps `(dx,dy)` delta to direction index |
| `DAT_007e3710` | `0x007E3710` | NS flank offset table | Flank cell offsets for NS-oriented bridges |
| `DAT_007e3730` | `0x007E3730` | EW flank offset table | Flank cell offsets for EW-oriented bridges |

No INI keys read directly by this function. `BlockagePathDelay` (locomotor INI) indirectly affects
the urgency value written into `PathfinderClass+0x3C` by the upstream caller.

---

## Callees

| Function | Address | Role | Out-of-scope? |
|----------|---------|------|--------------|
| `MapClass__Get_CellClass` | `0x005657A0` | Gets CellClass* from packed cell index | Yes — cell-system (manifest excluded) |
| `RateTimer__Current` | `0x004C93D0` | Gets current timer tick for blocker prediction | Yes — utility (manifest excluded) |

---

## Out-of-scope refs

- `MapClass__Get_CellClass @ 0x005657A0` — cell-system utility, excluded per manifest.
- `RateTimer__Current @ 0x004C93D0` — runtime utility, excluded per manifest.
- `UpdateBridgePassability @ 0x0042ACF0` — writer of `0x40000` marker; in-scope (task #10).
- `AStar_main_loop @ 0x00429A90` — sole caller; in-scope (task #4).

---

## YELLOW — Unverified

- The exact setter path for `PathfinderClass+0x01` (bridge-flank enable): constructor clears to 0
  (`0x0042A6E2` per bridge docs), but the full lifecycle of who sets it non-zero is not confirmed
  within this decode session. Prior docs note it is not always set in standard YR.
- The exact layout and content of `DAT_007e3760` (direction→delta table), `DAT_007e3710` (NS
  flank offsets), and `DAT_007e3730` (EW flank offsets) — prior docs describe semantics but
  individual byte values not re-read in this session.
- Blocker list ordering when multiple blockers share the same cell at code-2 (affects which blocker
  prediction path is walked first); noted in Ghidra comment as VERA20k audit item.
