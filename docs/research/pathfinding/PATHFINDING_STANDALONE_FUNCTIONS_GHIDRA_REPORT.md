# Pathfinding Standalone Functions — Ghidra Research Report

**Primary addresses:** `0x0047ca80`, `0x00481810`, `0x00429780`, `0x00429830`, `0x00483c80`, `0x0047d2b0`
**Confidence:** HIGH — all functions fully decompiled, data tables read from binary, call chains traced
**Active in YR:** YES, but not uniformly "every tick" — Pathfinding_update_continued,
Path_walk_directions_to_cell, and AStar_compute_edge_cost run every tick during active
unit movement/pathfinding; CellClass__ApplyLAT_and_SlopeFixup, CellClass::RecalcZoneType,
and CellClass::RecalcAttributes run only on terrain-mutation events (map load + runtime
terrain/overlay/building changes), not the per-tick sim loop (corrected 2026-07-12: header
overgeneralized "every tick" for all 6 functions; the doc's own per-function sections 2.1/2.5/2.6
already say "map load and runtime" / "every time terrain is modified" — confirmed via
`get_function_callers` on 0x0047ca80/0x00483c80/0x0047d2b0 showing terrain/overlay/building
call sites, none in the per-tick advance loop - OPERATOR_OR_ORDER_DRIFT)

## 1. Overview

This report covers the standalone pathfinding support functions that the A* search depends
on but which are NOT the A* algorithm itself. These functions handle:

1. **Cell zone type computation** — determining what kind of terrain each cell is for
   the passability matrix (CellClass::RecalcZoneType)
2. **Cell attribute recalculation** — updating LandType, SlopeIndex, and ZoneType when
   terrain changes (CellClass::RecalcAttributes)
3. **LAT tile auto-transition** — auto-selecting Rough/Sand/Green/Pave terrain blend
   variants + slope/ramp sub-variants based on cardinal neighbors (CellClass__ApplyLAT_and_SlopeFixup)
4. **Cell neighbor lookup** — the fundamental "get adjacent cell" utility
   (Pathfinding_update_continued)
5. **Path direction walking** — converting a direction array into a destination cell
   (Path_walk_directions_to_cell)
6. **A* edge cost evaluation** — computing traversal cost for the A* search
   (AStar_compute_edge_cost)

These work together: RecalcAttributes → CellClass__ApplyLAT_and_SlopeFixup → RecalcZoneType
establishes per-cell data. Then during A* search, AStar_compute_edge_cost reads that
data, and Pathfinding_update_continued navigates between cells.

See also: PATHFINDING_ASTAR_GHIDRA_REPORT.md (the A* algorithm itself),
ZONE_PASSABILITY_VERIFIED.md (passability matrix), TERRAIN_COST_FACTSHEET.md (speed tables).

---

## 2. Function Catalog

### 2.1 CellClass__ApplyLAT_and_SlopeFixup — 0x0047ca80

**Purpose:** Updates a cell's IsoTileTypeIndex (`cell+0x38`) based on which cardinal
neighbors share compatible LAT (Lookup Adjacent Tile) terrain types. This is the
LAT auto-transition system for terrain blending — Rough, Sand, Green, and Pave terrain
tile sets — plus a slope/ramp fixup pass. **Not** a bridge/tunnel/shoreline system.
(verified via `decompile_function 0x0047ca80` — function name `CellClass__ApplyLAT_and_SlopeFixup`,
uses LAT globals `g_ClearToRoughLat`, `g_ClearToSandLat`, `g_ClearToGreenLat`, `g_ClearToPaveLat`)

**Signature:** `bool __fastcall CellClass__ApplyLAT_and_SlopeFixup(int cell)`

**param_1 type:** `int` — direct byte offsets.

**Called from:**
- `CellClass::RecalcAttributes` (0x0047d2b0) — primary caller, during terrain updates
- `BuildingTypeClass::SetOwnerAndOccupy` (0x00543330) — when buildings are placed
- Two overlay-related functions (0x005a3ae0, 0x005a4280)

**Algorithm — Phase 1: Cardinal Neighbor Bitmask (4 passes)**

The function checks if `cell+0x38` (IsoTileTypeIndex) falls within specific tile type
ranges. Each range represents a LAT terrain tile set (Rough, Sand, Green, or Pave).
For each matching range, it:

1. Iterates 4 cardinal neighbors (N, E, S, W) by stepping direction index by 2:
   `uVar12 = uVar12 + 2 & 7` → visits directions 0, 2, 4, 6
2. Gets each neighbor via `MapClass::Get_CellClass`
3. Reads the neighbor's `IsoTileTypeIndex` (`neighbor+0x38`)
4. If the neighbor's tile type is NOT in the same range, sets a bit in a 4-bit bitmask
5. Result: `new_tile_index = range_base + bitmask` (selecting 1 of 16 sub-variants)
6. If all 4 neighbors match (bitmask == 0), sets tile to the range's "default" tile

This produces 16 possible tile connectivity variants per tile set (like Wang tiles):
- Bit 0: North neighbor doesn't match
- Bit 1: East neighbor doesn't match
- Bit 2: South neighbor doesn't match
- Bit 3: West neighbor doesn't match

The tile type ranges are stored in LAT globals: `g_ClearToRoughLat`, `g_ClearToSandLat`,
`g_ClearToGreenLat`, `g_ClearToPaveLat` (each a tile-type base index). Each has a range
of +0xf (16 variants). The base tiles are `g_RoughTile`, `g_SandTile`, `g_GreenTile`,
`g_PaveTile` respectively.

There are 4 sequential LAT passes (Rough → Sand → Green → Pave). Later passes have
additional exclusion ranges — e.g., the Green pass excludes ShorePieces (+0x29 range)
and WaterBridge (+1 range); the Pave pass excludes MiscPaveTile (+0xD), Medians (+0xD),
and PavedRoads (+0x14).
(verified via `decompile_function 0x0047ca80`)

**Algorithm — Phase 2: Cliff/Slope Connectivity**

After the cardinal passes, if the cell's SlopeIndex (`cell+0x11c`) is 1-4 (representing
the 4 cliff slope orientations), the function checks 2 specific neighbor cells for each
orientation:

| SlopeIndex | Check neighbors in directions | Meaning |
|-----------|-------------------------------|---------|
| 1 | Directions from DAT_0089f6a0 and DAT_0089f690 | Cliff facing orientation 1 |
| 2 | Directions from g_DirectionOffsets[0] and DAT_0089f698 | Cliff facing orientation 2 |
| 3 | Directions from DAT_0089f690 and DAT_0089f6a0 | Cliff facing orientation 3 |
| 4 | Directions from DAT_0089f698 and g_DirectionOffsets[0] | Cliff facing orientation 4 |

(corrected 2026-07-12: rows 2 and 4 were `g_DirectionOffsets[1]`; the decompiled code reads
the symbol `g_DirectionOffsets` itself with no index arithmetic — `(short)g_DirectionOffsets`
for dx and `g_DirectionOffsets._2_2_` for dy — which is element 0 (North, dx=0,dy=-1), not
element 1. Confirmed the symbol's address (0089f688) equals element 0's address via the same
byte-for-byte access pattern used unindexed elsewhere in this function's own cardinal-neighbor
loop (`*(short*)(&g_DirectionOffsets + uVar12)` for uVar12=0) via `decompile_function 0x0047ca80`
and `list_globals` filter `g_DirectionOffsets` (address 0089f688) - OPERATOR_OR_ORDER_DRIFT)

For each neighbor, if `neighbor.SlopeIndex == 0` (flat), a bit is set. The resulting
2-bit value selects a cliff edge sub-variant from the appropriate tile set.

Note: `DAT_0089f6a0` (used by rows 1 and 3) is also labeled `g_refinery_unload_adjacent_lookup_dx`
in current Ghidra — a name from an unrelated refinery-unload caller reusing the same generic
direction-offset slot. The address (0089f6a0) matches this doc's claim; the label is caller-specific
pollution, not evidence the slot is refinery-specific (verified via `list_globals` filter
`refinery_unload_adjacent_lookup`).

**Final step:** If the tile type changed, calls `FUN_00544c80` which triggers a TMP
(tile) reload if needed — the visual tile image must be reloaded to match the new
connectivity variant.

**Returns:** `true` if the cell's IsoTileTypeIndex changed (needs visual refresh).

**Active in YR:** YES — essential for correct terrain LAT tile blending and slope/ramp
sub-variant selection. Called every time terrain is modified (map load and runtime).

---

### 2.2 Pathfinding_update_continued — 0x00481810

**Purpose:** Given a CellClass and a direction (0-7), compute and return the
neighboring CellClass. This is the fundamental "get adjacent cell" primitive used
throughout the engine.

**Signature:** `void __thiscall Pathfinding_update_continued(int cell, uint direction)`

**Note:** Declared `void` by Ghidra but effectively returns a `CellClass*` via EAX
(the return value of MapClass::Get_CellClass is left in EAX).

**param_1 type:** `int` — direct byte offsets.

**Logic:**
```
if direction < 8:
    cell_x = cell.MapCoord_X                         (short at cell+0x24, low 16 bits)
    cell_y = cell.MapCoord_Y                         (short at cell+0x24, high 16 bits)
    new_x  = cell_x + g_DirectionOffsets[direction].dx
    new_y  = cell_y + g_DirectionOffsets[direction].dy
    return MapClass::Get_CellClass(new_x, new_y)
else:
    return cell  (bridge crossing — caller handles separately)
```

**Direction offsets table (g_DirectionOffsets at 0x0089f688):**

Runtime-initialized. 8 entries of (short dx, short dy) = 4 bytes each. Standard values:

| Dir Index | dx | dy | Direction | CellArray offset |
|-----------|----|----|-----------|------------------|
| 0 | 0 | -1 | North (up in cell grid) | -512 |
| 1 | +1 | -1 | NE | -511 |
| 2 | +1 | 0 | East | +1 |
| 3 | +1 | +1 | SE | +513 |
| 4 | 0 | +1 | South (down in cell grid) | +512 |
| 5 | -1 | +1 | SW | +511 |
| 6 | -1 | 0 | West | -1 |
| 7 | -1 | -1 | NW | -513 |

The CellArray offsets match the neighbor table at `0x007e3774` used by AStar_main_loop
(map width = 512 cells = 0x200).

**Called from:** 47+ call sites including:
- `DriveLocomotionClass::Process_Movement` (0x4b2630)
- `FootClass::Find_Path` (0x4d3920)
- `MapClass::GetZoneID` (0x56d230) — bridge navigation
- `MapClass::ComputeBridgeZones` (0x56d6e0)
- `ZoneMap::FloodFillReachableZones` (0x5840c0)
- `UnitClass::Can_Enter_Cell` (0x73f0a0)
- `ShipLocomotionClass::Process_Movement` (0x6a1c80)
- `IsFogged` (0x5864a0), `IsShrouded` (0x586360)
- `WarheadTypeClass::Detonate` (0x4690b0)

**Active in YR:** YES — core utility function, used every tick by virtually every
movement and map system.

---

### 2.3 Path_walk_directions_to_cell — 0x00429780

**Purpose:** Walk an array of direction steps from a start cell to compute the
destination cell. This converts a path (stored as direction indices) into the
cell coordinate at the end of that path.

**Signature:** `CellStruct* __fastcall Path_walk_directions_to_cell(CellStruct* out, CellStruct* start, int step_count, int* directions)`

**Logic:**
```
current_cell = *start
for i in 0..step_count:
    direction = directions[i]
    if direction == 8:  // tube/tunnel crossing
        cell = MapClass::Get_CellClass(current_cell)
        if cell.tube_index (cell+0x116) == -1:
            current_cell = {0, 0}  // invalid
        else:
            current_cell = *(g_TubeArray[cell.tube_index] + 0x28)
    else:
        current_cell.x += g_DirectionOffsets[direction].dx
        current_cell.y += g_DirectionOffsets[direction].dy
*out = current_cell
return out
```

**Key detail — direction 8 (tube/tunnel crossing):**
- Reads `cell+0x116` (short) — tube index (NOT bridge partner index)
- If -1: no tube partner, returns null cell
- Otherwise: looks up destination via `*(g_TubeArray + cell[0x116]*4) + 0x28`
- `g_TubeArray` is a `TubeClass*` array; `+0x28` is the tube destination cell coord
- This represents a "teleport" through a tunnel/tube, not a bridge-deck crossing
(verified via `decompile_function 0x00429780` — `g_TubeArray` name confirmed, not `g_BridgeArray`)

**Called from:**
- `FootClass::Run_AStar` (0x4cbba0) — walks current path_queue to find the start
  cell for the next A* search. This is how path continuation works: the unit's
  existing partial path is replayed to find where the unit will be when the current
  path runs out.
- `FUN_00582d70` (0x582d70) — bridge/tunnel path resolution for zone map updates

**Active in YR:** YES — called during every A* pathfinding operation.

---

### 2.4 AStar_compute_edge_cost — 0x00429830

**Purpose:** Compute the cost of traversing from one cell to a neighbor during A*
search. This is the g-cost component for each edge.

**Signature:** `float10 __thiscall AStar_compute_edge_cost(int pathfinder_ctx, int* src_cell_ptr, int* dest_cell_ptr, char bridge_flag, float zone_type_as_int)`

**param_1 type:** `int` (this pointer) — the PathfinderClass context.

**Note:** The 5th parameter (zone_type) is typed as `float` by Ghidra but is actually
an integer (0-7 zone type). The comparison `param_5 == 2.8026e-45` is a bit-pattern
comparison equivalent to `(int)param_5 == 2`.

**A* Base Cost Table (0x0081870c) — 8 entries, indexed by Can_Enter_Cell result code:**

**CORRECTION:** This table is indexed by the return value of `Can_Enter_Cell` (vtable+0x1ac),
NOT by ZoneType. The vtable call returns a passability code (0-7) for the destination cell.

| Index | Can_Enter_Cell code | Cost | Notes |
|-------|---------------------|------|-------|
| 0 | Clear (freely passable) | 1.0 | All normal terrain, including roads |
| 1 | Crushable obstacle | 1000.0 | Strongly avoid crushing |
| 2 | Temporary block (moving friendly) | 1.0 base → overridden to 4.0 | See below |
| 3 | Bridge ramp | 1.0 | Normal traversal |
| 4 | Occupied by stationary friendly | 60.0 | Expensive — try to path around |
| 5 | Occupied by enemy | 20.0 | Moderate cost — may want to fight through |
| 6 | Cliff / steep terrain | 8.0 | Prefer avoiding but not impassable |
| 7 | Impassable | 10000.0 | Effectively infinite |

**Temporary-block special case (Can_Enter_Cell == 2):**

When the destination cell has a moving friendly unit (TemporaryBlock), `pathfinder_ctx+0x3c`
(urgency) is checked FIRST and gates whether the prediction walk runs at all — it is not just
a post-hoc override:

- **If urgency (`pathfinder_ctx+0x3c`) is 1 or 2:** the prediction walk (steps 1-5 below) never
  runs — cost goes straight to 4.0, then to 1000.0 if urgency==2 (destroyer mode).
- **If urgency == 0:**
  1. Gets the object list from the destination cell (`cell+0xE4` ground or `cell+0xE8` bridge)
  2. If that list is empty (no blocker present), **cost stays at the table base value (1.0)**
     — it does NOT escalate to 4.0.
  3. Otherwise checks the first object's activity flag (`object+20 byte, bit 2`); if unset, the
     walk stops immediately and cost becomes 4.0.
  4. Reads the blocking unit's path_queue direction (`object[0x178]` = FootClass+0x5E0) — used
     only when the unit's current speed (`object+0x578`, a double) is 0.0; if that cached
     direction is `-1` (none), **cost stays at 1.0**, same as the empty-list case (no
     escalation). Otherwise (nonzero speed) derives facing from `RateTimer::Current`.
  5. Follows the blocking unit's predicted path ahead, cell by cell, up to 10 cells, checking
     bridge/height compatibility at each hop.
  6. After the loop ends (10 hops reached, or the activity-flag check stopped it early):
     cost = 4.0.

(corrected 2026-07-18: was "After following: cost = 4.0" stated as an unconditional outcome
with destroyer-mode (urgency==2) as the only branch; binary shows two additional cost-stays-1.0
exits — empty blocker list, or a lone blocker with zero speed and no cached path direction — and
shows the walk is skipped entirely (not merely result-overridden) whenever urgency != 0, not
only when urgency == 2. A sibling doc (`ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`, audited
2026-06-01) had already logged this as "blocker-clears 1.0; jam 4.0; urgency-2 1000.0" but the
correction was never propagated into this doc's own 2026-06-01/2026-07-12 passes. Verified via
`decompile_function 0x00429830` this session — the `AStar_cost_predict_blocker_clears` label
sits inside the `urgency == 0` branch and jumps past the `param_5 = 4.0` assignment straight to
the urgency-override check; urgency != 0 skips the containing `do {...} while` loop's `if`
entirely - ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

This is a **friendly-unit-path-prediction** mechanism, NOT road-following. It traces where
the blocking unit is heading so the pathfinder can assess whether to wait or path around.

**There is NO road-following mechanism.** Roads return Can_Enter_Cell code 0 (Clear)
with cost 1.0 — same as any open terrain. Units do not prefer roads in A*.

**Cliff ramp multiplier:**

If `dest_cell.Flags (cell+0x140) & 0x40000` (cliff ramp flag) is set:
```
cost *= 4.0    (constant at DAT_007e37bc = 0x40800000)
```

**Bridge diagonal cost modifiers:**

Only applied when `bridge_flag != 0` AND `pathfinder_ctx+0x01 != 0` (bridge-aware mode).
For diagonal moves across bridges, the function checks two flanking cells:

1. Computes direction index from cell delta: `dir = table_007e3760[(dy*3 + dx)]`
2. **Table selection** — chosen by `dest_cell+0x140 & 0x800` (undocumented selector):
   - If `dest_cell.Flags & 0x800 == 0`: use non-bridge table `0x007e3710`
     → cell offsets {-2,-2,0,1,1,1,0,-2}
   - If `dest_cell.Flags & 0x800 != 0`: use bridge table `0x007e3730`
     → cell offsets {0,-1024,-1024,-1024,0,512,512,512}
   (verified via `decompile_function 0x00429830` — `if ((*(uint *)(iVar6 + 0x140) & 0x800) == 0)`)
3. Checks if flanking cell has bridge flag (`cell+0x140 & 0x100`):

| Flanking cells with bridge | Multiplier | Constant address |
|----------------------------|------------|-----------------|
| Both sides have bridge | × 2.0 | DAT_007e37b4 = 0x40000000 |
| One side has bridge | × 1.0 | DAT_007e2ac8 = 0x3f800000 |
| Neither side (non-bridge diagonal) | × 10.0 | DAT_007e37b8 = 0x41200000 |

This penalizes diagonal movement that would cut across bridge geometry incorrectly.

**Total edge cost formula (in AStar_main_loop):**
```
raw_cost = AStar_compute_edge_cost(...)
total_g_cost = raw_cost * pathfinder_ctx.multiplier(+0x04) + direction_tiebreaker[direction]
```

**Direction tie-breaker offsets (0x0081872c) — 9 entries:**

| Dir | Offset | Dir | Offset |
|-----|--------|-----|--------|
| 0 (N) | ≈0.001 | 4 (S) | ≈0.003 |
| 1 (NE) | ≈0.005 | 5 (SW) | ≈0.007 |
| 2 (E) | ≈0.002 | 6 (W) | ≈0.004 |
| 3 (SE) | ≈0.006 | 7 (NW) | ≈0.008 |
| 8 (bridge) | 0.0 | | |

These tiny values (0.001–0.008) break ties when two paths have equal cost. Cardinal
directions have lower offsets than diagonals, producing a slight cardinal preference
when costs are otherwise identical. This prevents random path oscillation.

**Called from:** `AStar_main_loop` (0x00429a90) — exclusively. Called once per non-bridge
neighbor expansion (8 times per cell maximum).

**Active in YR:** YES — core A* cost function.

---

### 2.5 CellClass::RecalcZoneType — 0x00483c80

**Purpose:** Assigns the zone type (`cell+0x4C`) based on overlay, terrain, and
occupants. The zone type is the column index into the passability matrix (at 0x82a594).

**Signature:** `void __thiscall CellClass::RecalcZoneType(CellClass* this)`

**Zone type assignment priority (first match wins):**

| Priority | Condition | Zone | Value |
|----------|-----------|------|-------|
| 1 | Cell outside playfield | Out-of-bounds | 7 |
| 2 | Overlay has inherited `Crushable=` flag (`overlay+0x22D`) | Zone type 1 | 1 |
| 3 | Overlay has `Wall=` flag (`overlay+0x2A8`) | Wall | 2 |
| 4 | Overlay Wheel speed == 0.0 (terrain table entry at `0x0089ea48 + LandType*36`) | Impassable | 6 |
| 5 | Overlay `IsARock=` (`overlay+0x2B5`) | Impassable | 6 |
| 6 | Overlay `IsRubble=` (`overlay+0x2B4`) — forces Clear, skips all remaining checks below | Clear | 0 |
| 7 | LandType == 2 (Water) | Deep water | 4 |
| 8 | LandType == 6 (Beach) | Beach | 3 |
| 9 | Wheel speed for LandType <= 0.01 (double constant at `0x007e3808`) | Impassable | 6 |
| 10 | TerrainClass object (RTTI type 0x24) with type fields `+0x2A8`/`+0x2AC` compared to 7 | Wall/Building | 2 or 5 |
| 11 | Building on cell (type 6): see note below — outcome depends which sub-branch matches | Impassable, or silently overwritten to 0 | 6 or 0 |
| 12 | Default, or object-list scan exhausted with no qualifying early return | Clear | 0 |

(corrected 2026-07-12: row 6 (`IsRubble=` at `overlay+0x2B4`) was entirely absent from this
table — the binary checks it right after `IsARock=` and, if set, jumps straight to the
Clear/0 exit, skipping the LandType/Wheel-speed/object-list checks below. Row 9's threshold
was unstated "threshold"; confirmed 0.01 via `read_memory 0x007e3808` (double 0x3F847AE147AE147B).
Row 11 wording weakened — see note below the table. Verified via `decompile_function 0x00483c80`
and `disassemble_function 0x00483c80` - all STRUCT_FAMILY_CASCADE, the priority chain has more
branches/exits than previously enumerated)

Rows 2-5 and 8-9 above were corrected 2026-06-01: prior text named `+0x22D` as IsCrate, `+0x2A8` as IsWater, `+0x2B5` as IsRailroad, used Foot-speed wording, and described the type-0x24 object branch as a building water-flag branch; binary shows inherited ObjectTypeClass `Crushable=` at `+0x22D`, OverlayTypeClass `Wall=` at `+0x2A8`, `IsARock=` at `+0x2B5`, RecalcZoneType reads the Wheel column at `0x0089ea48`, and the type-0x24 branch compares type fields `+0x2A8/+0x2AC` to 7 via `decompile_function 0x00483c80`, `decompile_function 0x005f92d0`, `decompile_function 0x005fe770`, and `decompile_function 0x00674000` - INFERENCE_HARDENED / RTTI_LABEL_DRIFT.

**Key lookups:**
- Wheel speed: `terrain_speed_table[LandType * 9 + Wheel]` at `DAT_0089ea48 + LandType * 36`
  (`DAT_0089ea48` is the Wheel column, not Foot; corrected 2026-06-01: was `SpeedType 0 = Foot, column 0`; binary shows RecalcZoneType reads `0x0089ea48`, and RulesClass__ReadSpeedTypeLandTypeTable writes the `Wheel=` key to that slot via `decompile_function 0x00483c80` and `decompile_function 0x00674000` - INFERENCE_HARDENED)
- Overlay data: `DAT_00a83d84[overlay_index]` → OverlayTypeClass pointer
  - `+0x22D`: inherited `Crushable=` (byte) — **not** IsRoad and not IsCrate; zone result is value 1
    (corrected 2026-06-01: was `IsCrate`; binary shows ObjectTypeClass__ReadINI writes `Crushable=` to `+0x22D`, then RecalcZoneType tests that byte via `decompile_function 0x005f92d0` and `decompile_function 0x00483c80` - RTTI_LABEL_DRIFT)
  - `+0x298`: LandType (int)
  - `+0x2A8`: `Wall=` (byte) (corrected 2026-06-01: was `IsWater`; binary shows OverlayTypeClass__ReadINI writes `Wall=` to `+0x2A8`, and RecalcZoneType maps it to value 2 via `decompile_function 0x005fe770` and `decompile_function 0x00483c80` - INFERENCE_HARDENED)
  - `+0x2AC`: `NoUseTileLandType=` (byte) (corrected 2026-06-01: was unknown; binary shows the INI key write via `decompile_function 0x005fe770` - STALE)
  - `+0x2B4`: `IsRubble=` (byte) (corrected 2026-06-01: was unknown; binary shows the INI key write via `decompile_function 0x005fe770` - STALE)
  - `+0x2B5`: `IsARock=` (byte) (corrected 2026-06-01: was `IsRailroad`; binary shows the INI key write via `decompile_function 0x005fe770`, and RecalcZoneType maps it to value 6 via `decompile_function 0x00483c80` - INFERENCE_HARDENED)

**Building zone assignment detail:**
- Object RTTI type 0x24 = TerrainClass: compares the TerrainType fields at `+0x2A8`
  and `+0x2AC` against 7 to choose zone type 2 vs 5 (corrected 2026-06-01: was
  "Building RTTI type 0x24" and "naval buildings/water zone"; binary shows the type-0x24
  branch reads `object+0xC8` then compares `[type+0x2A8]` and `[type+0x2AC]` to 7 via
  `decompile_function 0x00483c80`, and `TerrainClass__What_Am_I` returns 0x24 via
  `decompile_function 0x0071d300` - INFERENCE_HARDENED)
- Building RTTI type 6 = BuildingClass: checks wall/gate flags for impassability, but the
  two sub-branches behave differently. Reading `object+0x520` then its byte `+0x16c0`:
  - If that byte is **nonzero**: reads `object+0x21c`'s byte `+0x1fa`; if nonzero, sets
    ZoneType=6 **and returns immediately** (locked in).
  - If that byte is **zero**: reads byte `+0x16bf` off the same `object+0x520` pointer and
    the value at `object+0x618`; if `+0x16bf` is nonzero and `object+0x618` is neither 0xc
    nor 8, it sets ZoneType=6 but does **not** return — the object-list loop continues to
    the next object (`object+0x30`). If no later object in the list triggers an early
    return, the function falls through to the unconditional `ZoneType=0` at the end (the
    same exit `IsRubble=` jumps to). This sub-branch's ZoneType=6 is transient and gets
    silently overwritten to Clear(0) unless another object on the cell independently forces
    a return. (new finding 2026-07-12, root cause STRUCT_FAMILY_CASCADE: confirmed via
    `decompile_function 0x00483c80` and `disassemble_function 0x00483c80` — instruction
    `00483dca: MOV dword ptr [EDI+0x4c],EBP` has no following RET, falls through to
    `00483dcd` (loop continue) and, on loop exhaustion, `00483dd4: MOV dword ptr [EDI+0x4c],0x0`
    unconditionally overwrites it. Field names for `+0x520/+0x16c0/+0x16bf/+0x618/+0x21c/+0x1fa`
    are not resolved — offsets only, no struct identity claimed)

**Called from:** `CellClass::RecalcAttributes` (0x0047d2b0) — always called after
LandType and overlay updates. Result stored in `cell+0x4C`.

**Active in YR:** YES — called every time cell terrain changes.

---

### 2.6 CellClass::RecalcAttributes — 0x0047d2b0

**Purpose:** Master cell attribute recalculation. Called whenever a cell's terrain,
overlay, or tile type changes. Updates LandType, SlopeIndex, IsoTileTypeIndex, and
triggers zone type recalculation.

**Signature:** `void __thiscall CellClass::RecalcAttributes(CellClass* this)`

**Key CellClass field map (verified from this function):**

| Offset | Type | Field | Source |
|--------|------|-------|--------|
| +0x24 | packed (short x, short y) | MapCoord | Cell grid position |
| +0x38 | int | IsoTileTypeIndex | Index into IsometricTileType array |
| +0x4C | int | ZoneType | Zone for passability matrix (0-7) |
| +0xE4 | int* | FirstObject | Ground-level object linked list |
| +0xE8 | int* | AltObject | Bridge-level object linked list |
| +0xEC | int | LandType | Terrain type (0-11) |
| +0x116 | short | TubeIndex | TubeClass index for direction-8 tube/tunnel path steps (-1 = none); corrected 2026-06-01: was `BridgePartnerIndex`, but binary reads this field as a `g_TubeArray` index in `Path_walk_directions_to_cell` and bounds/constructs it against `g_TubeCount` in `CellClass::RecalcAttributes` via `decompile_function 0x00429780` and `decompile_function 0x0047d2b0` - INFERENCE_HARDENED |
| +0x11A | byte | IsoSubTileIndex | Linear sub-tile index within the cell's TMP template (`row*template_width+col`), written by `MapClass__ApplyBridgeTile` (`puVar10[0x11a] = (char)(width*row+col)`). Ghidra's CellClass struct labels this offset `Height`, but that label is drift — do not read it as a height value. Consumers `TMP_ReadSlopeType`/`FUN_00544c20`/`FUN_00547150` use it to index TMP sub-tile records (consumer roles UNVERIFIED). (corrected 2026-07-12 by swarm parent spot-check: was `Height`/"TMP tile-height sub-data" sourced only from `get_struct_layout CellClass`; binary writer verified via `decompile_function 0x0057B440` — RTTI_LABEL_DRIFT) |
| +0x11B | char | Level | Cell height level |
| +0x11C | byte | SlopeIndex | Cliff/slope type (0 = flat, 1-4 = orientations) |
| +0x11D | char | field_0x11D | Computed from tile height data |
| +0x11E | short | field_0x11E | Overlay sub-state |
| +0x124 | uint | OccupationFlags_Ground | Ground occupation bitmask |
| +0x128 | uint | OccupationFlags_Bridge | Bridge occupation bitmask |
| +0x140 | uint | Flags | Cell flags bitfield |

**Cell Flags bitfield (cell+0x140):**
- `0x00000100` — Bridge flag (cell is a bridge deck)
- `0x00010000` — Cliff neighbor marker
- `0x00020000` — Has attached animation
- `0x00040000` — Cliff ramp flag (affects A* cost ×4)

**RecalcAttributes flow:**

**Phase 1 — Overlay handling:**
If cell has an overlay (`OverlayTypeIndex != -1`):
- Reads overlay's LandType from `OverlayTypeClass+0x298`
- Sets `cell.LandType` = overlay LandType
- If LandType is Wall (4), Railroad (9), or overlay has special flag:
  - Re-reads SlopeIndex from TMP data
  - If slope != 0 and overlay flag `+0x2A9` set: clears overlay (overlays can't sit on slopes)
  - Applies CliffBackImpassability check (see below)
  - Calls `CellClass__ApplyLAT_and_SlopeFixup()` to update tile connectivity
  - Calls `CellClass::RecalcZoneType()` to recompute zone
  - Returns early

**Phase 2 — Tile type handling:**
For cells without special overlays:
- Validates IsoTileTypeIndex is within bounds
- Reads tile data via `FUN_00544c20` (TMP tile validator)
- Re-reads SlopeIndex from TMP data
- Calls `CellClass__ApplyLAT_and_SlopeFixup()` if slope is present
- Determines LandType from tile data (`FUN_00544be0`)
- Handles overlay interactions (walls on slopes get cleared)
- **Tunnel detection:** If LandType == 10 (Tunnel) and cell has valid bridge data,
  creates a TubeClass for underground pathfinding
- Attaches tile animations if the tile type has them
- Sets cliff neighbor markers (flag 0x10000) on surrounding cells

**Phase 3 — CliffBackImpassability:**
Controlled by `RulesClass+0x664` (the `CliffBackImpassability=` INI key, 0/1/2):
1. Checks 6 neighbor cells (N-1, W-1, SE+2, S+1, SW, NE offsets from cell)
2. For each neighbor: `if neighbor.Level < cell.Level + 4` → continue checking
3. If ANY neighbor has level ≥ cell.Level + 4 → cliff detected
4. When `CliffBackImpassability == 2`:
   - Sets LandType to 3 (Rock) if current LandType is Clear(0), Water(2), Beach(6), or Ice(8)
   - Rock is impassable for most ground units

**Cliff neighbor offsets checked (relative to cell coords):**
```
(0, -1), (-1, 0), (+2, +2), (+1, +1), (-1, +1), (+1, -1)
```
These probe all 6 directions that could indicate a cliff face. Height difference ≥ 4
levels triggers the cliff-back marking. (corrected 2026-07-12: first two entries were listed
as `(-1,0), (0,-1)`; the binary's short-circuit AND chain checks `(0,-1)` (North) first, then
`(-1,0)` (West) — confirmed via `decompile_function 0x0047d2b0`, `CONCAT22(MapCoord_Y + -1, X)`
evaluated before `CONCAT22(MapCoord_Y, X + -1)`. All 6 reads are side-effect-free lookups
ANDed together, so this ordering does not change which cells get flagged as cliff-back —
OPERATOR_OR_ORDER_DRIFT, transcription only, not a behavioral difference)

**Phase 4 — Final updates:**
- Calls `CellClass::RecalcZoneType()`
- Updates zone map data arrays:
  - Zone map entry: stores Level and ZoneType (`cell+0x4C`)
  - Zone info entry: stores Level

**Called from:** Map loading, terrain modification, overlay changes, building placement.

**Active in YR:** YES — called during map init and every terrain change.

---

## 3. Data Tables Summary

### Neighbor Offset Table (0x007e3774) — CellArray Index Offsets

Used by `AStar_main_loop` for fast neighbor lookup. Map width = 512.

| Direction | Offset | Cell delta |
|-----------|--------|------------|
| 0 (N) | -512 | (0, -1) |
| 1 (NE) | -511 | (+1, -1) |
| 2 (E) | +1 | (+1, 0) |
| 3 (SE) | +513 | (+1, +1) |
| 4 (S) | +512 | (0, +1) |
| 5 (SW) | +511 | (-1, +1) |
| 6 (W) | -1 | (-1, 0) |
| 7 (NW) | -513 | (-1, -1) |

### A* Cost Table (0x0081870c) — Indexed by Can_Enter_Cell Result Code

| Code | Meaning | Cost |
|------|---------|------|
| 0 | Clear | 1.0 |
| 1 | Crushable | 1000.0 |
| 2 | TemporaryBlock (moving friendly) | 1.0 → 4.0 |
| 3 | BridgeRamp | 1.0 |
| 4 | OccupiedFriendly | 60.0 |
| 5 | OccupiedEnemy | 20.0 |
| 6 | Cliff | 8.0 |
| 7 | Impassable | 10000.0 |

### Direction Tie-Breaker Offsets (0x0081872c) — Added to g-cost

Tiny epsilon values (0.001–0.008) that break ties. Cardinals < diagonals.

### Bridge Diagonal Cost Multipliers

| Constant | Address | Value | When applied |
|----------|---------|-------|--------------|
| Bridge one side | 0x007e2ac8 | 1.0 | One flanking cell is bridge |
| Bridge both sides | 0x007e37b4 | 2.0 | Both flanking cells are bridge |
| Non-bridge diagonal | 0x007e37b8 | 10.0 | Neither flanking cell is bridge |
| Cliff ramp | 0x007e37bc | 4.0 | Cell has cliff ramp flag |

---

## 4. INI Keys

| Key | Section | Type | Default | Effect |
|-----|---------|------|---------|--------|
| CliffBackImpassability | [General] | int (0/1/2) | 2 | Controls cliff-back terrain marking: 0=off, 1=partial, 2=full (sets LandType=Rock) |
| PathDelay | [General] | float (minutes) | 0.01 | Cooldown between path searches |

The `CliffBackImpassability` key maps to `RulesClass+0x664`. When set to 2 (YR default),
cells behind cliffs (height difference ≥ 4) get LandType changed to Rock (3), making them
impassable for most ground units. This prevents units from pathfinding into visually
hidden areas behind cliff faces.

---

## 5. Integration Points

### Call flow during terrain changes:
```
terrain modified → CellClass::RecalcAttributes
  ├─ Update LandType from overlay/TMP data
  ├─ CellClass__ApplyLAT_and_SlopeFixup  (LAT terrain blend + slope/ramp sub-variants)
  ├─ CellClass::RecalcZoneType    (zone type for passability matrix)
  └─ Update zone map arrays
```

### Call flow during A* search:
```
AStar_main_loop
  ├─ For each of 9 neighbors (8 directions + bridge):
  │   ├─ Get neighbor cell (via CellArray offset table at 0x007e3774)
  │   ├─ Check zone map stamp (dual ground/bridge closed sets)
  │   ├─ vtable+0x1ac call → get zone type / passability code
  │   ├─ AStar_compute_edge_cost → base cost + modifiers
  │   ├─ Total: cost * multiplier + direction_tiebreaker
  │   └─ AStar_create_node → push to open set min-heap
  └─ Pop minimum f-cost node from heap, repeat
```

### Path continuation:
```
FootClass::Run_AStar
  ├─ Path_walk_directions_to_cell(path_queue) → find current path endpoint
  └─ AStar_pathfind_search(endpoint) → search from there to destination
```

### Zone map queries (using Pathfinding_update_continued):
```
MapClass::GetZoneID(cell, zone_category, bridge_aware)
  ├─ If bridge cell: follow bridge via Pathfinding_update_continued
  │   to find the ground cell underneath
  └─ Look up zone ID from zone map arrays
```

---

## 6. Current Rust Implementation Status

### Well-implemented:
- **Zone categories** (`src/sim/pathfinding/passability.rs`): 13 MovementZone rows × 8 zone
  type columns in passability matrix — matches binary
- **Zone flood fill** (`src/sim/pathfinding/zone_build.rs`): ground + bridge layers
- **A* core** (`src/sim/pathfinding/core.rs`): octile heuristic, 24-step segments,
  MAX_SEARCH_NODES=65527, layered bridge-aware search
- **Path smoothing** (`src/sim/pathfinding/path_smooth.rs`): zigzag removal + drift correction
- **Cell entry checks** (`src/sim/pathfinding/cell_entry.rs`): 8-code result enum

### Gaps / differences from binary:
- **A* cost table**: Rust uses CARDINAL_COST=10, DIAGONAL_COST=14 (integer octile). Binary
  uses float costs from a Can_Enter_Cell result code table (1.0 for clear, 60.0 for
  occupied-by-friendly, 1000.0 for crushable, etc.). The binary's A* weighs cell
  OCCUPATION state, not terrain type. Our A* ignores occupation costs during search.
- **Friendly-unit-path-prediction**: Not implemented. When Can_Enter_Cell returns 2
  (TemporaryBlock — a friendly is moving through), the binary follows the blocking unit's
  path_queue ahead up to 10 cells and assigns cost 4.0. This helps units decide whether
  to wait or repath around moving friendlies.
- **Direction tie-breakers**: Not implemented. Binary adds tiny epsilon values (0.001-0.008)
  per direction to break cost ties deterministically.
- **Bridge diagonal cost modifiers**: Not found in Rust. Binary multiplies diagonal bridge
  crossing costs by 1.0/2.0/10.0 depending on flanking cell bridge flags.
- **Cliff ramp cost multiplier**: Not found in Rust. Binary multiplies cliff ramp cells
  by 4.0 in A* cost.
- **CliffBackImpassability**: Partially implemented. `src/sim/pathfinding/` does not appear
  to check for cliff-back terrain marking (LandType→Rock when behind cliffs).
- **CellClass__ApplyLAT_and_SlopeFixup** (LAT terrain auto-transition + slope fixup): Not needed in Rust — we load
  pre-authored maps from WAE, tiles are already resolved. Only needed for runtime terrain
  modification (bridge destruction/construction).
- **RecalcZoneType logic**: The Rust zone assignment in `zone_build.rs` uses passability
  matrix checks rather than the binary's priority-based overlay/terrain/occupant cascade.
  This may produce slightly different zone assignments for edge cases (buildings, railroads).

---

## 7. Open Questions

1. **Temporary-block object chain**: The old "road-following" interpretation is resolved:
   `AStar_compute_edge_cost` enters this object-list scan only when `Can_Enter_Cell` returned
   code 2, selects the destination cell's `cell+0xE4` or `cell+0xE8` list by bridge layer,
   and predicts a moving blocking unit's future cells from activity/facing/path fields before
   assigning the temporary-block cost. It is not a road overlay/object chain. (corrected
   2026-06-01: was "Road-following object chain"; binary shows the `param_5 == 2` gate,
   `cell+0xE4/+0xE8` list selection, up-to-10-cell prediction loop, and final `param_5 = 4.0`
   via `decompile_function 0x00429830` - INFERENCE_HARDENED)

2. **Pathfinder context field +0x3c**: Value 0 enables the temporary-block prediction path
   for `Can_Enter_Cell == 2`; value 2 overrides that temporary-block cost to 1000.0.
   What other values exist, and what sets this field, still needs tracing. (corrected
   2026-06-01: was "normal ground mode enables road-following" and "destroyer mode ignores
   road preference"; binary shows both uses are inside the code-2 temporary-block branch via
   `decompile_function 0x00429830` - INFERENCE_HARDENED)

3. **Bridge flanking table at 0x007e3730**: The large offset values (-1024, 512) suggest
   these index into the CellArray at 2-row distances. Need to verify the exact geometry
   of bridge flanking checks. **Confidence: MEDIUM** — values decoded but spatial meaning
   not fully verified.

4. **Exact ZoneType-to-passability mapping**: CellClass::RecalcZoneType produces values
   0-7 which index the A* cost table. But the passability matrix has 8 columns. Need to
   verify the ZoneType values exactly match the passability matrix column ordering.
   **Confidence: HIGH** — the 8 values match, but the exact column names in the matrix
   should be cross-checked.

5. **Cliff ramp flag (0x40000) origin**: What sets this flag on `cell+0x140`? Likely set
   during map loading or RecalcAttributes. Not traced to source. **Confidence: LOW**.

---

## Sources

### Ghidra addresses decompiled:
- `0x0047ca80` — CellClass__ApplyLAT_and_SlopeFixup (LAT terrain auto-transition + slope fixup)
- `0x00481810` — Pathfinding_update_continued (get neighbor cell)
- `0x00429780` — Path_walk_directions_to_cell (walk direction array)
- `0x00429830` — AStar_compute_edge_cost (A* edge cost)
- `0x00429a90` — AStar_main_loop (core A* search, 427 lines)
- `0x0042a460` — AStar_create_node (node allocation + heuristic)
- `0x00483c80` — CellClass::RecalcZoneType (zone assignment)
- `0x0047d2b0` — CellClass::RecalcAttributes (master cell recalc)
- `0x004cbba0` — FootClass::Run_AStar (A* wrapper)
- `0x0042d510` — FUN_0042d510 (cell coordinate addition helper)
- `0x00544c80` — FUN_00544c80 (TMP tile reload trigger)
- `0x005657a0` — MapClass::Get_CellClass
- `0x00582d70` — Bridge/tunnel path resolution function

### Data tables read:
- `0x0081870c` — A* base cost table (8 floats)
- `0x0081872c` — Direction tie-breaker offsets (9 floats)
- `0x007e37b4` — Bridge cost multiplier: both sides = 2.0
- `0x007e37b8` — Non-bridge diagonal multiplier: 10.0
- `0x007e37bc` — Cliff ramp multiplier: 4.0
- `0x007e2ac8` — Bridge one-side multiplier: 1.0
- `0x007e3710` — Bridge flanking cell offsets (non-bridge)
- `0x007e3730` — Bridge flanking cell offsets (bridge)
- `0x007e3760` — Direction index from cell delta
- `0x007e3774` — CellArray neighbor offset table (8 directions)
- `0x0089f688` — g_DirectionOffsets (8 × {short dx, short dy})

### Related documents referenced:
- `PATHFINDING_ASTAR_GHIDRA_REPORT.md` — A* algorithm, call hierarchy, path queue format
- `TERRAIN_COST_FACTSHEET.md` — SpeedType×LandType movement speed table
- `ZONE_PASSABILITY_VERIFIED.md` — passability matrix (13×8), MovementZone enum
- `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md` — zone map structure
- `CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md` — occupation flags
- `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md` — path smoothing passes
- `DRIVE_TRACK_SYSTEM.md` — vehicle turn curves
