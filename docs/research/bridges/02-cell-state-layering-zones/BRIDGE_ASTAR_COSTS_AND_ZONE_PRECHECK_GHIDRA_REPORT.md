# Bridge A* Costs & Zone Precheck — Ghidra Research Report

**Phase:** Phase 1 of approved plan `docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`
**Plan items covered:** #4 (Zone_precheck), #5 (AStar_compute_edge_cost), #6 (cost constants 0x7E37B4/B8/BC + cross-refs)
**Companion doc:** `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md` (items #1, #2, #3, #7, #8, #9)
**Date:** 2026-05-13
**Active in YR:** **Yes** — both functions reached every standard skirmish A* call.

> Every claim cites a Ghidra address + decompilation excerpt or `read_memory` byte dump.
> Confidence axes: **C**=content / **I**=identity / **B**=binding.

---

## 1. Overview

Two systems are paired here because they share data structures (PathfinderClass arrays) and constants region (`0x007E37B0..0x007E37BF`):

1. **`AStar_compute_edge_cost (0x429830)`** — per-edge cost computation called from the inner A* loop. Reads the Can_Enter_Cell return code, applies a base-cost lookup, multiplies by bridge-related multipliers, and for diagonal moves looks up two flanking cells to choose a bridge-corner cost.

2. **`Zone_precheck (0x42C290)`** — pre-A* hierarchical Dijkstra. Runs at 3 zone-graph levels (2 → 1 → 0) and returns 0 if any level reports the start/dest zones as disconnected. On success, marks all zones on the chosen level-0 path so main A* can skip cells whose zone isn't on the path.

The cost constants at `0x7E37B4` (=2.0), `0x7E37B8` (=10.0), `0x7E37BC` (=4.0) drive **diagonal-bridge cost and `0x40000`-flag multiplier**. One of the three (10.0) is **shared with damage code** — naming it as pathfinding-exclusive would mislead.

---

## 2. The cost constants region (verified)

Reading 32 bytes at `0x007E37B0`:

```
0x007E37B0: 00 00 80 3f   → float 1.0     (g_AStar_Cost_OnePointZero ??? — see §5)
0x007E37B4: 00 00 00 40   → float 2.0     (g_BridgeDiag_BothSides)
0x007E37B8: 00 00 20 41   → float 10.0    (g_BridgeDiag_NonBridge — SHARED with damage code)
0x007E37BC: 00 00 80 40   → float 4.0     (g_BridgeApproach_CostMult)
0x007E37C0: be 9f 1a 2f dd 24 f0 3f   → double 1.009 (tie-break epsilon; documented in companion doc)
```

Reading 8 bytes at `0x007E2AC8`:
```
0x007E2AC8: 00 00 80 3f   → float 1.0     (g_BridgeDiag_OneSide — the "one flanker on bridge" case)
```

Confidence: C=HIGH (raw memory + xref), I=HIGH (Ghidra labels for two of three), B=HIGH (xref site count manually verified — see §3).

### 2.1 Cross-use audit per constant

| Address | Value | Xrefs (`get_xrefs_to`) | Pathfinding-exclusive? |
|---------|-------|-------------------------|------------------------|
| **0x7E37BC** | 4.0 | **AStar_compute_edge_cost @ 0x4299BC [only]** | **YES — safe to rename `g_BridgeApproach_CostMult_4_0`** |
| 0x7E37B8 | 10.0 | Apply_area_damage @ 0x489D61, 0x489D79, 0x489D92; WarheadTypeClass__Detonate @ 0x469929, 0x469933, 0x469945; AStar_compute_edge_cost @ 0x429A52 | **NO — shared with damage. Renaming would mislead.** |
| 0x7E37B4 | 2.0 | VeterancyClass__IsVeteran @ 0x74FFA1; VeterancyClass__IsElite @ 0x750012; AStar_compute_edge_cost @ 0x429A6F; Volume__GetCategory @ 0x750032 | **NO — shared with veterancy/audio.** |
| 0x7E2AC8 | 1.0 | very widely used — 65+ readers including Quaternion_Slerp, FactoryClass__GetBuildStepTime, HouseClass__GetCostBonus, AStar_compute_edge_cost @ 0x4299F1, and many more | **NO — generic 1.0f literal pool.** |

**Important.** The compiler interned float literals — `1.0`, `2.0`, `10.0` were emitted once each in `.rdata` and every site reads the same address. Naming them `BridgeXxx` in Ghidra would be misleading for non-pathfinding sites. The user's plan called this out: **0x7E37BC (4.0) is the only one truly unique to pathfinding**. 2.0 and 10.0 must be named generically (e.g. `g_Float_2_0_pool` and `g_Float_10_0_pool`), with comment annotations at each pathfinding-relevant read site.

---

## 3. AStar_compute_edge_cost (0x429830) — full decomp + branch enumeration

### 3.1 Function signature (verified)

`__thiscall AStar_compute_edge_cost(int param_1, int *param_2, int *param_3, char param_4, float param_5)`

| Param | Type | Meaning |
|-------|------|---------|
| `param_1` | PathfinderClass* | this |
| `param_2` | CellClass* | source cell |
| `param_3` | CellClass* | destination cell (the one we want to enter) |
| `param_4` | char | **`(char)pathfinder_layer == 0`** — i.e. `1 if entering BRIDGE layer, 0 if GROUND` (inverted polarity vs main loop) |
| `param_5` | float | **the Can_Enter_Cell return CODE (0-7), reinterpret_cast-as-float** |

**`param_5` gotcha**: Ghidra's decompiler tags `param_5` as `float`, but the actual call site passes a **signed int** (Can_Enter_Cell return). The check `param_5 == 2.8026e-45` is really `param_5_as_int == 2` (because IEEE 754 denormal representation of int 2 is `0x00000002` ≈ 2.8e-45 as a subnormal float). Then `param_5 = *(float *)(&g_AStar_EdgeCost_BaseTable + (int)param_5 * 4)` immediately loads the float from the base table, discarding the int-vs-float ambiguity.

### 3.2 Base cost table at `0x0081870C` (`g_AStar_EdgeCost_BaseTable`)

Reading 32 bytes at `0x0081870C`:

| Index | Hex bytes | Float value | Can_Enter_Cell code |
|-------|-----------|-------------|---------------------|
| [0] | `00 00 80 3F` | **1.0** | OK (cell empty / walkable) |
| [1] | `00 00 7A 44` | **1000.0** | Crushable |
| [2] | `00 00 80 3F` | **1.0** | TemporaryBlock (friendly moving — may be overridden, see §3.3) |
| [3] | `00 00 80 3F` | **1.0** | BridgeRamp / passable special |
| [4] | `00 00 70 42` | **60.0** | FriendlyWall |
| [5] | `00 00 A0 41` | **20.0** | EnemyBlock |
| [6] | `00 00 00 41` | **8.0** | FriendlyStationary |
| [7] | `00 40 1C 46` | **10000.0** | Impassable |

The bytes at `0x0081872C..0x0081874B` are a separate table (per-direction-cost addend) used by main A*; the EdgeCost table is exactly 8 floats at 0x81870C. See §6 of companion doc for the per-direction table.

**Confidence**: C=HIGH (memory dump), I=HIGH (table base label confirmed by main-loop reference at `0x429F96`), B=HIGH (single read site in AStar_compute_edge_cost).

### 3.3 Code-2 special handling — friendly-moving-unit prediction

When Can_Enter_Cell returned code 2 (FriendlyMoving — a unit is currently in the cell but moving), the function **predicts whether the blocker will clear** by walking the blocker's planned path up to 10 cells:

```c
// Decompilation (sanitized):
if (param_5_as_int == 2) {
  blocker_list = (param_4 == 0)
              ? dest_cell.+0xE4    // ground occupancy list
              : dest_cell.+0xE8;   // bridge occupancy list
  iVar7 = 0;
  if (pathfinder.+0x3C == 0) {
    do {
      if (blocker_list == NULL) goto CLEAR_PREDICTION;       // blocker dispersed → cost 1.0
      if ((blocker_list.+5_bits >> 2) & 1 == 0) break;        // not in "moving" sub-state → cost 4.0
      if (blocker_list.velocity_double == 0.0) {              // not actually moving
        next_dir = blocker_list.+0x178;                        // first queued direction
        if (next_dir == 0xFFFFFFFF) goto CLEAR_PREDICTION;     // no planned move → cost 1.0
      } else {
        next_dir = (RateTimer__Current() >> 0xc + 1 >> 1) & 7; // pseudo-random direction
      }
      coord = blocker_list.coord;
      neighbor_coord = coord + g_DirectionOffsets[next_dir];   // step in next_dir
      next_cell = MapClass.Get_CellClass(neighbor_coord);

      // Layer decision for tracing the blocker's path:
      if (!(next_cell.flags & 0x100) ||
          (blocker.+0x8C == 0 &&
           prev_cell.Level - next_cell.Level < 3)) {     // ← ASYMMETRIC threshold!
        blocker_list = next_cell.+0xE4;
      } else {
        blocker_list = next_cell.+0xE8;
      }
      iVar7++;
    } while (iVar7 < 10);
  }
  param_5 = 4.0;                              // fell through 10 iters → 4.0 ("jammed")
CLEAR_PREDICTION:
  // base 1.0 retained (table[2] = 1.0)
  if (pathfinder.+0x3C == 2) param_5 = 1000.0;  // URGENT mode → reroute around
}
```

**Verified exit costs** for code-2:

| Exit path | Cost |
|-----------|------|
| Blocker dispersed (list became NULL) | **1.0** (base, no change) |
| Blocker not moving + no queued direction | **1.0** |
| Blocker inactive (move-state bit clear) | **4.0** |
| 10 iterations completed (jammed traffic) | **4.0** |
| `pathfinder.+0x3C == 2` (URGENT — explicit reroute request) | **1000.0** (overrides everything) |

The `< 3` threshold on the inner layer-decision is the **asymmetric height-diff trap** — only checks `prev.Level - cur.Level`, never the reverse. So:
- If the blocker is **descending** by less than 3 height-units, stay on ground.
- If the blocker is **ascending** by any amount, the test is on the negative difference which is always < 3 → also stay on ground.

This means **the blocker prediction can only follow a unit down off a bridge, never up onto one**. Subtle but important for parity. Documented as one of the four divergent thresholds in the companion doc §4.1.

### 3.4 The `0x40000` BridgeApproach multiplier — straightforward

```c
if (dest_cell.flags & 0x40000) param_5 *= g_BridgeApproach_CostMult_4_0;   // 4.0
```

This is the **only** read of `0x7E37BC` in the entire binary. Single-purpose constant.

**Player-observable effect:** when bit 0x40000 is set (transient, during a peer's A* search — see companion §5), cells near other moving units cost 4× more, so the new A* search routes around them.

### 3.5 Diagonal-bridge cost computation (the meat)

Triggered when:
- `param_4 != 0` (entering BRIDGE layer)
- AND `pathfinder.+1 != 0` (PathfinderClass byte at offset 1 enables this — gated for ground-A* only)

```c
if (param_4 != 0 && pathfinder.+1 != 0) {
  // Look up direction code from (dy, dx)
  dy = dest.coord.y - src.coord.y;        // expected ∈ {-1, 0, +1}
  dx = dest.coord.x - src.coord.x;        // expected ∈ {-1, 0, +1}
  int dir = DirEncodeTable[dy*3 + dx];    // table at 0x7E3760-base
  
  // Choose orientation table based on bridge orientation (bit 0x800 of cell.flags)
  if (!(dest.flags & 0x800)) {
    flank_offset_1 = OffsetTable_NS[dir];       // 0x7E3710 table
    flank_offset_2 = OffsetTable_NS[(dir - 4) & 7];
  } else {
    flank_offset_1 = OffsetTable_EW[dir];       // 0x7E3730 table
    flank_offset_2 = OffsetTable_EW[(dir - 4) & 7];
  }
  
  flank_1_cell = dest_cell_array_ptr[flank_offset_1];
  flank_2_cell = dest_cell_array_ptr[flank_offset_2];
  
  if (flank_1_cell.flags & 0x100) {
    multiplier = g_BridgeDiag_OneSide;          // 1.0 (at 0x7E2AC8)
    if (flank_2_cell.flags & 0x100)
      multiplier = g_BridgeDiag_BothSides;       // 2.0 (at 0x7E37B4)
    return base_cost * multiplier;
  }
  return base_cost * g_BridgeDiag_NonBridge;     // 10.0 (at 0x7E37B8)
}
return base_cost;
```

### 3.6 Direction-encoder and orientation offset tables (verified)

**`(dy, dx) → dir` table** at `0x007E3750..0x007E3770` (9 ints, indexed by `(dy*3 + dx) * 4`):

| dy\dx | -1 | 0 | 1 |
|-------|----|----|----|
| -1 | 7 (NW) | 0 (N) | 1 (NE) |
| 0  | 6 (W) | -1 (self/invalid) | 2 (E) |
| 1  | 5 (SW) | 4 (S) | 3 (SE) |

**NS-orientation flank table** at `0x007E3710..0x007E372F` (8 ints, indexed by direction code):

| Dir | Code | Flank offset (int units) |
|-----|------|--------------------------|
| N | 0 | -2 |
| NE | 1 | -2 |
| E | 2 | 0 |
| SE | 3 | +1 |
| S | 4 | +1 |
| SW | 5 | +1 |
| W | 6 | 0 |
| NW | 7 | -2 |

**EW-orientation flank table** at `0x007E3730..0x007E374F` (8 ints):

| Dir | Code | Flank offset (cell-pointer-array units, map_width=512) |
|-----|------|--------------------------------------------------------|
| N | 0 | 0 |
| NE | 1 | -1024 (= -2 rows) |
| E | 2 | -1024 |
| SE | 3 | -1024 |
| S | 4 | 0 |
| SW | 5 | +512 (= +1 row) |
| W | 6 | +512 |
| NW | 7 | +512 |

These offsets are **in int-pointer-array units** (multiplied by 4 implicit when added to a `int *` pointer). The cell-pointer array is row-major with stride `MAP_WIDTH = 512` (verified via `[ESI+0x140]` accesses and `IMUL EAX, [0x89c2dc]` which loads MAP_WIDTH from the runtime-initialised global).

The second offset (`(dir - 4) & 7`) flips to the opposite direction — so the two flanks are 180°-opposite. Specifically, for direction code 2 (E), `flank_1 = 0`, `flank_2 = OffsetTable[(2-4)&7] = OffsetTable[6] = 0`. Both flanks are the same cell — that's the **degenerate cardinal case**. For diagonal directions (1, 3, 5, 7), the flanks are different.

**Why only diagonals matter:** for cardinal directions (N/E/S/W), both flank lookups return the same cell, and the diagonal-bridge cost path simplifies to just checking if that single cell is on a bridge. The `2.0` multiplier never fires for cardinal moves because the second test (`flank_2.flags & 0x100`) is identical to the first.

For diagonal moves (NE/SE/SW/NW), the two flanks are the cells you'd "pass through" on the diagonal. The cost table:

| Both flanks on bridge? | Multiplier | Meaning |
|-----------------------|------------|---------|
| YES | **2.0** (`g_BridgeDiag_BothSides`) | Clean diagonal across a bridge-deck section |
| First only | **1.0** (`g_BridgeDiag_OneSide`) | Diagonal-corner approach onto bridge — no penalty |
| Neither / first not | **10.0** (`g_BridgeDiag_NonBridge`) | Diagonal-cutting onto bridge from off-bridge — heavy penalty |

**Player-observable effect:** units will only step diagonally onto a bridge cell if **the cell they cut through** is itself a bridge cell. Otherwise the 10× penalty steers them to the bridgehead and they step on cardinally. This is what produces the "units go to the bridgehead first" behaviour rather than diagonal shortcuts.

The cell.flags **bit `0x800` (NS bridge orientation)** is verified here as the gate that swaps between the two offset tables — already documented in `BRIDGE_SYSTEM.md` but reconfirmed here at this exact code site.

---

## 4. Zone_precheck (0x42C290) — 3-tier hierarchical Dijkstra

### 4.1 Function signature & parameters

`uint __thiscall Zone_precheck(int param_1, undefined4 param_2, undefined4 param_3, int param_4, int param_5)`

| Param | Type | Meaning |
|-------|------|---------|
| `param_1` | PathfinderClass* | this |
| `param_2` | CellCoord (packed) | start cell |
| `param_3` | CellCoord (packed) | dest cell |
| `param_4` | int | MovementZone (used as index into g_PassabilityMatrix) |
| `param_5` | FootClass* | unit (nullable — if null, slope-cost estimation disabled) |

Returns **0 if unreachable** (any of the 3 levels reports disconnected), **1 if reachable** at all levels.

### 4.2 Slope-cost estimation gate

```c
if (param_5 == 0) {
  slope_param = 0;
  bVar11 = false;
} else {
  float slope_factor = FootClass.Get_Slope_Speed_Factor();
  slope_param = FootClass.+0x21C;                    // locomotor cap?
  bVar11 = true;
  if (slope_factor <= _DAT_007e3810) bVar11 = false; // threshold
}
```

`_DAT_007e3810` is a **double at 0x007E3810** = bytes `f1 68 e3 88 b5 f8 e4 3e` = `0x3EE4F8B588E368F1` = **1.0e-5**.

So slope-cost estimation activates whenever `Get_Slope_Speed_Factor() > 1e-5`, which is essentially "any non-zero". The threshold exists only to skip dead-zero values; for any real unit it's always on. **Confidence**: C=HIGH (memory verified), I=MEDIUM (purpose inferred from context), B=HIGH (single read site).

`FootClass.+0x21C` is passed to `Zone_Estimate_Slope_Cost` — likely the **locomotor speed type** or a slope-modifier index. Needs follow-up.

### 4.3 Top-level loop: 3 levels, 2 → 1 → 0

```c
local_38 = 2;
do {
  // Clear heap, do Dijkstra at level local_38
  // ...
  local_38--;
} while (local_38 >= 0);
return 1;   // success
```

At each level:
1. Clear `pathfinder.+0x68` heap.
2. Get start/dest **zone IDs** at level local_38 from `g_ZoneInfoTable[cell_idx].zones[local_38]` (table base `DAT_0087F858`, 10 bytes/entry, `+local_38*2` is a u16).
3. Load the level-specific arrays:
   - `iVar3 = pathfinder.+(0x40 + local_38*4)` — zone-on-path marker array
   - `iVar15 = pathfinder.+(0x4C + local_38*4)` — zone-closed marker array
   - `iVar4 = pathfinder.+(0x58 + local_38*4)` — zone f-cost array
   - `local_2c = pathfinder.+(0x44 + local_38*4)` for levels 0/1 ; **`local_2c = 0` at level 2** (different semantics)
4. Mark start and dest zones as "interesting" in the level's zone-on-path array.
5. If `start_zone == dest_zone`: same-zone case → record length 1, continue to next level.
6. Else: run Dijkstra (heap-based, expanding zones via adjacency at `g_ZoneGraph[level] @ DAT_0087F878 + local_38*0x18`, each zone has 0x24-byte block).
7. On reaching dest zone, walk parent chain in node pool, mark each zone on path as "on-path" in iVar3.

### 4.4 Zone graph layout (verified via Zone_precheck reads)

`g_ZoneGraph_Level0 @ 0x0087F878` (24 bytes per level, so `+0x18 = 0x0087F890` is level 1, `+0x30 = 0x0087F8A8` is level 2).

Each zone block within a level is **0x24 bytes**:

| Offset | Type | Field |
|--------|------|-------|
| `+0x04` | ptr | adjacency-edge array |
| `+0x10` | u32 | adjacency-edge count |
| `+0x18` | u16 | "representative" cell zone id (level-1 parent) |
| `+0x1C` | i32 | LandType-or-class index (`iVar18` in decomp) |

Each adjacency edge is **8 bytes**:

| Offset | Field |
|--------|-------|
| `+0` | u32 | neighbour zone id |
| `+4` | u32 | edge flags (low byte = `1 if bridge-edge`) |

So adjacency walk: `for (uVar25 in zone_block.edges) { neighbor_id = uVar25.id; bridge_edge = uVar25.flags & 0xFF; ... }`.

### 4.5 Per-edge cost computation in Dijkstra

```c
float slope_cost = bVar11 ? float(Math__ftol(Zone_Estimate_Slope_Cost(slope_param, level, cur_zone, next_zone))) : 0;
float bridge_penalty = (edge.flags_low_byte != 0) ? _DAT_007e3818 : 0.0;
float neighbor_cost = g_ZoneBaseCostByLandType[next_zone.land_type] + parent_node.fcost + slope_cost + bridge_penalty;
```

**`_DAT_007e3818`** at 0x007E3818 = bytes `fc a9 f1 d2 4d 62 50 3f` = `0x3F50624DD2F1A9FC` = **0.001** (exact IEEE 754 representation of 1.0e-3). Bridge edge penalty in zone precheck — essentially a tiebreak, not a real discouragement.

**`g_ZoneBaseCostByLandType`** at `0x007E3794` (8 floats):

| Index | Hex bytes | Float | Land type (inferred) |
|-------|-----------|-------|----------------------|
| [0] | `00 00 80 3F` | 1.0 | Clear |
| [1] | `00 00 00 00` | 0.0 | Road |
| [2] | `00 00 00 00` | 0.0 | (uncategorised — 0.0) |
| [3] | `00 00 80 3F` | 1.0 | Rough |
| [4] | `00 00 80 3F` | 1.0 | Beach |
| [5] | `00 00 00 00` | 0.0 | (uncategorised — 0.0) |
| [6] | `00 00 80 3F` | 1.0 | Tiberium / Ore |
| [7] | `00 00 80 3F` | 1.0 | Tunnel / Rock |

(Land type mapping inferred from `passability.rs` semantics; confidence MEDIUM for the exact label per index. The values themselves are HIGH confidence.)

**Note**: indices [1], [2], [5] all read 0.0 → zones with those land types have **zero cost** in zone precheck. This makes Roads "free" in the Dijkstra. (Reasonable since the actual A* will apply detail cost; the precheck just decides reachability + zone ordering.)

### 4.6 Three filter conditions for accepting an edge

After computing `neighbor_cost`, the edge is accepted only if all three hold:

```c
if (   ((iVar15[next_zone] != epoch) || (neighbor_cost < iVar4[next_zone]))   // not yet closed, or strictly better
    && (local_38 == 2                                                              // at level 2, no per-cell zone check
        || (local_2c[next_zone.parent_zone] == epoch                                // parent zone is open at higher level
            || next_zone.land_type == 1))                                            // OR special land type 1
    && (g_PassabilityMatrix[movement_zone * 8 + next_zone.land_type] == 1) )      // basic passability
{
  // ... accept ...
}
```

**Verified semantics:**

1. **Closed-list test with f-cost beat**: standard Dijkstra (`closed[next] != epoch || new_cost < closed[next].cost`).
2. **Hierarchical link gate**: at levels 0 and 1, only expand into zones whose **level-(N+1) parent** was on the higher level's path. Level 2 has no parent, so always pass. **Special case**: if `next_zone.land_type == 1`, override and allow expansion regardless. Land type 1 = (likely) **Road** — roads bypass hierarchical pruning. This is a key parity detail.
3. **Passability matrix**: `g_PassabilityMatrix[movement_zone * 8 + land_type]` must equal 1.

The hierarchical gate `local_2c[next_zone.+0x18] == epoch` reads the higher-level's per-cell-zone marker (set when that level's Dijkstra marked a zone as on-path). If the parent zone wasn't on the higher-level path, this zone is pruned. This is the **hierarchical cascade**.

### 4.7 Bridge-edge exclusion list (per level)

After the three filters pass, **one more check**:

```c
if (pathfinder.+(0x84 + level*0x18) != 0) {     // bridge_edge_count[level] != 0
  // Pack (min(cur, next), max(cur, next)) and linear-search pathfinder.+(0x78 + level*0x18)
  // If found in the bridge-edge exclusion list → SKIP this edge
}
```

This is the **broken-bridge exclusion**: when a bridge is destroyed, its zone-graph edges are added to the `pathfinder.+(0x78 + lvl*0x18)` array. Subsequent Dijkstras will refuse to use those edges. The data is keyed by sorted (min, max) zone id pair, packed as `(min << 16) | max`.

**The pair is searched linearly from the end backward** (`piVar22 + iVar18 * 4; do { ... iVar18--; } while (-1 < iVar18)`). Linear search — could be slow if many bridges destroyed, but normally O(few).

### 4.8 Heap & node pool

- Heap descriptor at `pathfinder.+0x68` — same 5-int layout as main A* heap.
- Node pool at `pathfinder.+0x64` (=100 decimal) — 16 bytes per node (parent_offset_within_pool, zone_id, f_cost (float), depth).
- Heap sift-up at the insert site; sift-down via call to `MinHeap__SiftDown @ 0x42DCA0`. (Also one inlined sift-down at `LAB_0042c7c5` — but that's just the same algorithm inlined for the pop hot path, NOT TS-legacy.)

### 4.9 Path-marking when destination reached

When Dijkstra pops a node whose zone == dest zone:

```c
// Walk parent chain in node pool, marking each zone on path
piVar13 = current_node;
while (piVar13.parent != -1) {
  iVar3[piVar13.zone_id] = epoch;             // mark zone as ON-PATH at this level
  piVar13 = node_pool + piVar13.parent * 16;
}
iVar3[piVar13.zone_id] = epoch;               // mark start zone too

// Record path zone count
pathfinder.+(0xC74 + level*4) = depth + 1;

// Record path zones in pathfinder.+0xBC + level*1000 (reverse order)
puVar24 = (u16 *)(pathfinder + 0xBC + (depth + level*500) * 2);
for (i = depth; i > 0; i--) {
  *puVar24-- = (u16) piVar13.zone_id;
  piVar13 = node_pool + piVar13.parent * 16;
}
pathfinder.+(0xBC + level*1000) = (u16) start_zone;
```

So `pathfinder.+0xBC..` is the path-zones array, indexed by `level*500` words plus path step. Buffer slots: 500 words = 1000 bytes per level. Three levels = 1500 bytes total.

Main A* later reads `pathfinder.+0xBE` (which is `0xBC + 2`, i.e. the first zone at level 0 *after* the start zone) and uses it as the "next expected zone" cursor. The cursor advances when A* expands a cell whose zone id matches the cursor (see companion §2 `+0x6C`).

### 4.10 Same-zone short-circuit at level 0

```c
if (start_zone == dest_zone && local_38 == 0) {
  // Treat as a 1-step path: write a single node at node_pool[0]
  node_pool[0].depth = 0;
  node_pool[0].zone_id = start_zone;
}
pathfinder.+(0xBC + level*1000) = (u16) start_zone;
pathfinder.+(0xC74 + level*4) = 1;
```

Same-zone is a trivial success — no Dijkstra needed. Only special-cased at level 0; higher levels just fall through the regular path.

### 4.11 Return semantics

- **Returns 0** if at any level Dijkstra fails to reach the dest zone (heap empty without finding it). Whole precheck fails.
- **Returns 1** if all 3 levels succeed.

A return of 0 from `Zone_precheck` means **the destination is unreachable at the current zone graph** (broken bridge, walled off area, etc.). `AStar_pathfind_search` treats this as a hard failure for cross-zone searches.

### 4.12 Same-zone case in AStar_pathfind_search interaction

For same-zone path requests, `AStar_pathfind_search` calls `Zone_precheck` and on failure **logs "Hierarchical findpath failure" but continues with `param_8` (hierarchy hint) cleared**. So same-zone always tries the A* loop even if the hierarchy says "no". Cross-zone failures, in contrast, return 0 immediately. See companion doc §8.

---

## 5. The 1.0 at 0x7E37B0 — what is it?

Reading `0x7E37B0..0x7E37B3`: `00 00 80 3F` = float 1.0. This is NOT referenced from compute_edge_cost directly, but it sits just before the cost-multiplier triplet (2.0, 10.0, 4.0). May be:
- An adjacent literal not used in this triple but used elsewhere
- The base "no-multiplier" 1.0 for a different path's cost lookup

`get_xrefs_to(0x007E37B0)` was not run; deferred as low priority since the value 1.0 is generic and likely a shared literal. Plan item #6 didn't specifically ask about 0x7E37B0; flagged as **Open Question** for completeness.

---

## 6. Active in YR — per function and per constant

| Item | Active in YR? | Evidence |
|------|---------------|----------|
| `AStar_compute_edge_cost` | Yes | Caller: `AStar_main_loop` only. Reachable every pathfind. |
| `Zone_precheck` | Yes | Callers: `AStar_pathfind_search` AND `FUN_0042D170`. Latter is the helper used by `FootClass::Find_Path`'s code-6/7 nearby-cell fallback. Both reachable in skirmish. |
| `g_BridgeApproach_CostMult @ 0x7E37BC` (4.0) | Yes | Read every pathfind where any cell has bit 0x40000 set. UpdateBridgePassability sets the bit during every A* with `pathfinder.+0x3C != 0`. |
| `g_BridgeDiag_NonBridge @ 0x7E37B8` (10.0) — pathfinding read | Yes | The diagonal-bridge cost branch is gated by `param_4 != 0 && pathfinder.+1 != 0`. The latter (`+1` byte) is set by AStar_pathfind_search for ground-A* runs — verified via the `param_8` chain. So this fires on every cross-bridge diagonal entry. |
| `g_BridgeDiag_BothSides @ 0x7E37B4` (2.0) | Yes | Same gate as above. |
| `g_BridgeDiag_OneSide @ 0x7E2AC8` (1.0) | Yes | Same gate. |
| `_DAT_007e3810` (1e-5 slope threshold) | Yes | Read in Zone_precheck every call. Effectively always-true. |
| `_DAT_007e3818` (0.001 bridge edge penalty) | Yes | Read in Zone_precheck every bridge-edge consideration. |

None of these are gated behind TS-only flags (`SpecialFlags`, fog, subterranean). All live in standard YR.

---

## 7. Refuted prior-audit claims (carried over from plan §2)

### 7.1 "Zone_precheck contains TS-legacy heap-sift branch"

**Refuted.** The block at `LAB_0042c7c5` is just an inlined min-heap sift-down. Standard binary heap code, identical algorithm to the main A* heap. The variable `iVar22 != 1` is the heap-position iteration cursor — not a TS-related flag. Verified by reading the assembly and confirming each `FCOMP float ptr [N+0x8]` is the f-cost compare common to all heap ops.

### 7.2 Cost constants are pathfinding-only

**Refuted for 2.0 and 10.0** (see §2.1 xref table). 4.0 is pathfinding-exclusive. The others share with damage code (10.0) and veterancy/audio code (2.0). Implementers should NOT rename Ghidra labels for 2.0/10.0 as pathfinding-specific.

---

## 8. The "what does pathfinder.+1 do?" mystery

The diagonal-bridge cost path is gated by `*(char *)(param_1 + 1) != 0`. So PathfinderClass has a single-byte flag at offset +1 that enables/disables the diagonal-bridge cost branch. Two questions:

1. **Where is it set?** Likely in `AStar_pathfind_search` based on locomotor type — i.e. "this locomotor cares about bridge costs". Search would reveal it.
2. **Why disable it?** Possibly for air units / locomotor kind 2 (which already ignores bridge layer at the height-selection stage). But the gate to enter `compute_edge_cost` is `param_4 != 0` (entering bridge layer), and air units already get GROUND-layer source/dest via the kind==2 branch in main loop. So `+1 == 0` would be redundant... unless there's another locomotor that uses bridge layer but **shouldn't pay the diagonal cost**.

Flagged as **Open Question**.

---

## 9. Open Questions

1. **Where is `pathfinder.+1` set** and what conditions clear it? Affects whether the diagonal-bridge cost branch fires.
2. **`FootClass.+0x21C`** semantic — locomotor slope-cost parameter passed to `Zone_Estimate_Slope_Cost`.
3. **`Zone_Estimate_Slope_Cost (0x585F40)`** full decomp — Phase 4 will cover under zone-system items.
4. **`MinHeap__SiftDown (0x42DCA0)`** body — confirm it's the standard textbook sift-down with no surprises. Low priority.
5. **g_ZoneGraph block offsets `+0x00`, `+0x08`, `+0x0C`, `+0x14`, `+0x20`** — only +0x04, +0x10, +0x18, +0x1C documented here. Phase 4 should fill the rest.
6. **`g_ZoneBaseCostByLandType`** — confirm the index-to-LandType mapping (currently inferred from convention).
7. **The 1.0 at 0x7E37B0** — xref check deferred.

---

## 10. Current Rust Implementation Status

| Binary feature | Rust file | Status |
|----------------|-----------|--------|
| 8-element base cost table (0x81870C) | [src/sim/pathfinding/cell_entry.rs](../../ra2-rust-game/src/sim/pathfinding/cell_entry.rs) | Implemented (`Code-{0..7}` codes). Cost magnitudes may differ. Worth direct comparison: gamemd uses 1.0/1000/1.0/1.0/60.0/20.0/8.0/10000.0; Rust may not match. |
| Code-2 friendly-moving prediction (10-step blocker walk) | partial | Rust has soft-block via `path_blocked_counter`, but doesn't predict blocker's planned-path. Player-visible effect on multi-unit traffic. |
| Code-2 urgency=2 → 1000.0 reroute | none | Missing — Rust doesn't have a 3-level urgency for A*. |
| Diagonal-bridge cost (2.0 / 1.0 / 10.0) | none | **Critical missing**. Without this, units take diagonal shortcuts onto bridges that gamemd would discourage. Player-visible. |
| BridgeApproach 4.0 multiplier (0x40000 flag) | none | Missing (depends on UpdateBridgePassability). |
| Zone_precheck 3-tier Dijkstra | partial | `zone_search.rs` has hierarchical Dijkstra but the 3-level cascade + parent-zone-must-be-on-higher-path gate isn't replicated exactly. |
| Bridge-edge exclusion list | partial | `bridge_orchestrator.rs` triggers zone rebuilds, but the "exclude these edges" mechanism is implicit (rebuilt graph just doesn't have the edges). Equivalent in observable outcome; design difference fine. |
| Slope cost estimation in precheck | partial | `Zone_Estimate_Slope_Cost` not separately ported; Rust uses inline slope multipliers. |
| 0.001 bridge edge penalty | none | Likely doesn't matter due to magnitude. |

(Severity assessment deferred to Phase 7 synthesis.)

---

## 11. Sources

**Ghidra functions decompiled:**
- `AStar_compute_edge_cost` @ 0x00429830 (body to 0x00429A8E, ~607 bytes)
- `Zone_precheck` @ 0x0042C290 (body to 0x0042C8F7, ~1639 bytes)

**Memory reads:**
- 0x007E37B0..0x007E37CF (cost constants region: 1.0, 2.0, 10.0, 4.0, then tie-break double)
- 0x007E2AC8 (1.0 — g_BridgeDiag_OneSide)
- 0x0081870C..0x0081872B (8-entry cost base table)
- 0x007E3794..0x007E37B3 (8-entry land-type cost table)
- 0x007E3810..0x007E381F (slope threshold 1e-5 and bridge penalty 0.001)
- 0x007E3710..0x007E376F (3 tables: NS flank offsets, EW flank offsets, dy*3+dx direction encoder)
- 0x007E1748 (heap initial priority — 0.0)
- 0x007E3774 (g_CellNeighborOffsets_8Dir — 8 cell-array offsets)

**Cross-reference checks (binding evidence):**
- 0x007E37BC: 1 xref [AStar_compute_edge_cost] — **truly pathfinding-exclusive**
- 0x007E37B8: 7 xrefs across 3 functions [Apply_area_damage ×3, WarheadTypeClass__Detonate ×3, AStar_compute_edge_cost ×1] — **shared with damage**
- 0x007E37B4: 4 xrefs [VeterancyClass×2, AStar_compute_edge_cost, Volume__GetCategory] — **shared**
- 0x007E2AC8: 65+ xrefs (generic 1.0f literal pool)

**Callers:**
- AStar_compute_edge_cost ← AStar_main_loop [only]
- Zone_precheck ← AStar_pathfind_search, FUN_0042D170 [both reachable in YR skirmish]

**Callees:**
- AStar_compute_edge_cost → MapClass__Get_CellClass, RateTimer__Current
- Zone_precheck → FootClass::Get_Slope_Speed_Factor, Math::ftol, MinHeap::SiftDown, ZoneMap::CellToZoneIndex, Zone_Estimate_Slope_Cost

**Companion doc:**
- `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md` (the A* spine + dual closed-list this report's constants drive)
