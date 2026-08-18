# Cliff Objects — Ghidra Research Report

**Primary addresses:** Tile loading `0x00545714`, IsCliffTile `0x004863d0`, ZFudge `0x004daff0`, RecalcAttributes `0x0047d2b0`
**Confidence:** HIGH (all findings verified from decompilation)
**Active in YR:** Partially — see Section 14 for TS legacy analysis

## 1. Overview

Cliff objects in gamemd.exe are **terrain tile sets** defined in theater INI files, not standalone
game objects. The engine uses numeric tile set index ranges (not string matching) to classify
tiles as cliffs. Cliff systems span 7 interconnected subsystems: tile registration, tile
classification, variant randomization, neighbor fixup, destroyable cliff destruction, bullet
cliff blocking, rendering Z-fudge, and cliff-back impassability.

**IMPORTANT — Tiberian Sun legacy filtering:** Two cliff subsystems are **dormant in standard YR**
and should NOT be implemented: DestroyableCliffs (no theater defines them) and ZFudgeCliff
(no unit sets it to nonzero). See Section 14 for full analysis.

---

## 2. Tile Set Registration (Theater INI)

**Function:** Tile loading at `0x00545714` (mislabeled `CDFileClass__Constructor`)

The tile loader reads cliff-related tile set indices from the **theater-specific INI** file
(e.g., `temperatemd.ini`) `[General]` section. These are tile set indices, not tile IDs — they
get resolved to actual tile IDs during the tile loading loop.

| INI Key | Global Address | Range (tiles) | Default |
|---------|---------------|---------------|---------|
| `CliffSet` | `0x00aa1020` | `[CliffSet, CliffSet+40)` | -1 |
| `CliffRamps` | `0x00abbebc` | `[CliffRamps, CliffRamps+20)` | -1 |
| `WaterCliffs` | `0x00aa101c` | `[WaterCliffs, WaterCliffs+28)` | -1 |
| `DestroyableCliffs` | `0x00abc2c8` | `[DestroyableCliffs, DestroyableCliffs+2)` | -2 |

All four are read via `CCINIClass::ReadInt` with default -1 (or -2 for DestroyableCliffs).
A value of -1 means "this tileset doesn't exist in this theater."

### Related tile set globals (for context)

| INI Key | Global | Range | Notes |
|---------|--------|-------|-------|
| `BridgeSet` | `0x00aa0e28` | +16 tiles | Used in IsCliffTile checks |
| `WoodBridgeSet` | `0x00abad1c` | +16 tiles | Used in IsCliffTile checks |
| `WaterCaves` | `0x00abad24` | +4 tiles | Used in IsCliffTile checks |
| `WaterfallEast` | `0x00aa073c` | +4 tiles | Directional passability exceptions |
| `WaterfallWest` | `0x00abb110` | +4 tiles | Directional passability exceptions |
| `WaterfallNorth` | `0x00aa10a0` | +4 tiles | Directional passability exceptions |
| `WaterfallSouth` | `0x00aa1050` | +4 tiles | Directional passability exceptions |
| `SlopeSetPieces` | `0x00abc1f8` | — | Used as replacement when cliffs are destroyed |
| `HeightBase` | `0x00aa0744` | — | Base tile set for height transitions |

---

## 3. Tile Classification — IsCliffOrImpassableTile

**Function:** `FUN_004863d0`
**Signature:** `bool IsCliffOrImpassableTile(IsometricTileTypeClass *tile)`
**Key fields:** `tile+0x38` = tile set index, `tile+0x11a` = sub-tile slope direction

Returns **true** if the tile belongs to any of these impassable tile set ranges:

```
CliffSet         [CliffSet,         CliffSet + 0x28)        — 40 tiles
CliffRamps       [CliffRamps,       CliffRamps + 0x14)      — 20 tiles
WaterCliffs      [WaterCliffs,      WaterCliffs + 0x1C)     — 28 tiles
DestroyableCliffs[DestroyableCliffs, DestroyableCliffs + 2) — 2 tiles
BridgeSet        [BridgeSet,        BridgeSet + 0x10)       — 16 tiles
WoodBridgeSet    [WoodBridgeSet,    WoodBridgeSet + 0x10)   — 16 tiles
WaterCaves       [WaterCaves,       WaterCaves + 4)         — 4 tiles
WaterfallEast    [WaterfallEast,    WaterfallEast + 4)      — 4 tiles (with exceptions)
WaterfallWest    [WaterfallWest,    WaterfallWest + 4)      — 4 tiles (with exceptions)
WaterfallNorth   [WaterfallNorth,   WaterfallNorth + 4)     — 4 tiles (with exceptions)
WaterfallSouth   [WaterfallSouth,   WaterfallSouth + 4)     — 4 tiles (with exceptions)
```

### Waterfall slope-direction exceptions

Waterfall tiles have **directional passability** based on the sub-tile slope (`tile+0x11a`):

| Waterfall | Passable when slope direction is... |
|-----------|-------------------------------------|
| East | 0 (flat) or 4 (south) |
| West | 1 (west) or 3 (east) |
| North | 2 (north) or 3 (east) |
| South | 0 (flat) or 1 (west) |

Only the first and last tiles of each waterfall set (index 0 and 3) can be passable based
on slope. Middle tiles (indices 1-2) are always impassable.

### IsOnBridgeRamp — `0x00578d80`

Same logic as IsCliffOrImpassableTile but only checks: CliffSet (40), CliffRamps (20), and
the 4 waterfall directions (with slope exceptions). Does NOT check bridges, WaterCliffs,
DestroyableCliffs, or WaterCaves. Used to determine height transition ramps for movement.

---

## 4. Cliff Variant Randomizer

**Function:** `FUN_005a1350`
**Purpose:** Visual randomization so cliff faces don't look repetitive.

Takes a tile set index, subtracts `CliffSet`, and returns a randomized variant within the
same visual group. The 40 cliff tiles are organized into groups:

```
Offset from CliffSet → Behavior
4-6:     Group A — 3 variants, random(0-2) + 4
5:       Pair select from Group A — random(0-1)*2 + 4 → {4, 6}
6:       Pair select from Group A — random(0-1) + 4 → {4, 5}
8-10:    Group B — same pattern as A, base offset 8
11-13:   Group C — same pattern, base offset 11
14-16:   Group D — same pattern, base offset 14
22-24:   Group E — same pattern, base offset 22
28-29:   Mirror swap — 28→29, 29→28
34-36:   Group F — random selection
Default: Returns input unchanged (not a randomizable cliff)
```

Uses `Random::Next()` with `Math::ftol()` for variant selection.

---

## 5. Cliff Neighbor Fixup

**Function:** `FUN_005a17f0`
**Purpose:** Ensures adjacent cliff tiles have matching visual edges after randomization.

Algorithm:
1. Iterate all tiles in the current view
2. For each tile in `[CliffSet, CliffSet+0x28)`:
   - Check neighbor at direction offset 4 (SE) and direction offset 2 (NE)
   - If neighbor has the same tile set index but lower sub-tile slope value
   - Call the variant randomizer (`FUN_005a1350`) on the current tile
   - If randomization changes the tile, update the adjacent cell's rendered tile

---

## 6. Destroyable Cliff Destruction

**Function:** `FUN_00581140`
**Check function:** `FUN_00486900` — returns true if `tile_set_index == DestroyableCliffs` or
`tile_set_index == DestroyableCliffs + 1`

### DestroyableCliffs tile 0 (6-wide layout)

```
sub-tile layout: 6 columns × N rows
column = sub_tile_index % 6
row    = sub_tile_index / 6  (integer division, using multiply-by-magic-number trick: *-0x2AAAAAAB)
origin_x = tile_x - column
origin_y = tile_y + row
```

### DestroyableCliffs tile 1 (4-wide layout)

```
sub-tile layout: 4 columns × N rows
column = sub_tile_index & 3  (bitwise AND)
row    = sub_tile_index >> 2  (right shift)
origin_x = tile_x - column
origin_y = tile_y - row
```

### Destruction sequence

1. Create a new overlay object from the same tile template
2. Place it at the computed origin coordinates
3. Set tile to non-blocking: clear `0x81` flag, set `0x74` to destroyed mode
4. Call virtual method `0x124` (Destroy/Remove from map)
5. Free the tile object
6. Replace with **SlopeSetPieces** tiles (`DAT_00abc1f8` and `DAT_00abc1f8+1`)
7. For each cell in the destroyed cliff's footprint:
   - Reset zone connectivity (`zone_data[idx] = 0`)
   - Call `CellClass::RecalcAttributes` to update passability
   - Call `FUN_00584550` (rebuild zone connections) for cells with zero zone
   - Stop all targeting on affected cells
   - Mark radar minimap dirty
8. Create destruction animations:
   - For each cell in a 5×3 grid around the origin:
     - Spawn 2 random animations from 3 possible anim types
     - Random X offset: [-8, +8], random Y offset: [-12, +12]
     - Random animation start frame: [0, 2]
     - Duration flag: `0x600` (1536 frames)
9. Dirty the screen rectangle (with 0x78/0x3C pixel padding)
10. Call `MapClass::UpdateBridgeZonesHelper` to recalculate bridge zones

---

## 7. Bullet Cliff Blocking — SubjectToCliffs

**BulletTypeClass field:** offset `0x296` (bool)
**Read in:** `BulletTypeClass::ReadINI` at `0x0046bfeb`
**Used in:** `FUN_004CC360` (bullet cliff/wall collision check)

### Collision check algorithm (FUN_004CC360)

```
function BulletCliffWallCheck(bullet_pos, bullet_type, source_cell, dest_cell):
    mid_cell = GetCellAt(bullet_pos / 256)

    // --- Cliff check ---
    if bullet_type.SubjectToCliffs:
        src_height = source_cell.GetEffectiveHeight()
        mid_height = mid_cell.GetEffectiveHeight()
        if mid_height - src_height > 3:               // uphill cliff face
            dst_height = dest_cell.GetEffectiveHeight()
            if dst_height != mid_height AND dst_height - mid_height >= 0:
                return mid_cell  // BLOCKED by cliff

    // --- Wall check ---
    if bullet_type.SubjectToWalls:
        if mid_cell has wall overlay with IsWall flag:
            if mid_cell != dest_cell:                  // don't block at target
                if dest_cell.height >= source_cell.height:  // not firing downhill over wall
                    if Rules.AlliedWallTransparency AND wall owner is allied:
                        return NULL  // allied walls are transparent
                    return mid_cell  // BLOCKED by wall

    return NULL  // not blocked
```

**Key constants:**
- Height difference threshold for cliff blocking: **> 3 levels** (i.e., ≥ 4)
- `CellClass::GetEffectiveHeight()` = `cell.height_0x11B + (cell.flags_0x140 >> 7 & 1) * 4`
  (adds +4 if bridge overlay flag `0x80` is set)

### Related BulletTypeClass fields

| Offset | Type | INI Key | Purpose |
|--------|------|---------|---------|
| 0x295 | bool | `Floater` | Floats in air |
| 0x296 | bool | `SubjectToCliffs` | Blocked by cliff faces |
| 0x297 | bool | `SubjectToElevation` | Range bonus from height |
| 0x298 | bool | `SubjectToWalls` | Blocked by wall overlays |
| 0x299 | bool | `VeryHigh` | Very high trajectory |
| 0x29A | bool | `Shadow` | Casts shadow |
| 0x29B | bool | `Arcing` | Arcing trajectory |

---

## 8. Rendering Z-Fudge System

**Compositor function:** `FUN_004daff0` (TechnoClass Z-adjust)
**Purpose:** Prevent units from rendering through cliff/bridge/tunnel/column geometry by
adjusting their Z-buffer depth value.

### TechnoTypeClass ZFudge fields

| Offset | Field | INI Key | Default | Purpose |
|--------|-------|---------|---------|---------|
| 0xDC0 | `[0x370]` | `ZFudgeCliff` | 0 | Z bias when near cliff face |
| 0xDC4 | `[0x371]` | `ZFudgeColumn` | 0 | Z bias when near bridge support column |
| 0xDC8 | `[0x372]` | `ZFudgeTunnel` | 0 | Z bias when near tunnel entrance |
| 0xDCC | `[0x373]` | `ZFudgeBridge` | 0 | Z bias when on/under bridge |

Note: `param_1` is `int *` in TechnoTypeClass, so `param_1[0x370]` = byte offset `0x370 × 4 = 0xDC0`.

### Z-Fudge compositor algorithm (FUN_004daff0)

```
function ComputeZFudge(techno):
    base_z = techno.GetBaseZAdjust()           // vtable+0x38
    type = techno.GetType()                    // vtable+0x84

    column_proximity = IsNearBridgeColumn()    // FUN_00703e70 → returns 0, 1, or 2
    tunnel_proximity = IsNearTunnel()          // FUN_00704000 → returns 0 or 1
    cliff_proximity  = IsNearCliff()           // FUN_00704240 → returns 0, 1, or 2
    on_bridge        = IsOnBridge()            // FUN_00703b10 → bool

    column_fudge = type.ZFudgeColumn * column_proximity
    tunnel_fudge = type.ZFudgeTunnel * tunnel_proximity
    cliff_fudge  = type.ZFudgeCliff  * cliff_proximity
    bridge_fudge = on_bridge ? type.ZFudgeBridge : 0

    // Take the MAXIMUM of all 4 fudge values
    max_fudge = max(column_fudge, tunnel_fudge, cliff_fudge, bridge_fudge)

    additional_z = ComputeAdditionalZ()        // FUN_00704350 — slope/ramp/gate Z
    return base_z + max_fudge + additional_z
```

### IsNearCliff — `FUN_00704240`

```
function IsNearCliff(techno):
    cell = GetCellAt(techno.GetCoords())
    if techno.IsInAir:
        return 0

    // Check cell one step in the "back" direction (DAT_0089f694 = isometric NW offset)
    neighbor1 = GetCellAt(cell + DAT_0089f694)
    if neighbor1 != NULL AND neighbor1.height - cell.height > 3:
        // One cell behind is a cliff: check two cells behind
        neighbor2 = GetCellAt(neighbor1 + DAT_0089f694)
        if neighbor2 != NULL AND neighbor2.height - cell.height > 3:
            return 1   // deeply behind cliff
        return 2       // one cell behind cliff (stronger fudge)

    return 0           // not near cliff
```

The proximity check looks **one direction** (isometric NW, using `DAT_0089f694` offset). It
returns 2 for "immediately behind cliff" and 1 for "two cells behind cliff." This is
multiplied by the per-unit `ZFudgeCliff` value to get the final Z offset.

### IsOnBridge — `FUN_00703b10`

Checks 5 neighbor cells (current + 4 cardinal directions) for bridge flag `0x100` at
`cell+0x140`, with directional filtering using flag `0x800`.

### IsNearBridgeColumn — `FUN_00703e70`

Checks 3 neighbor cells for bridge tile set indices in the range `[BridgeSet+7, BridgeSet+16]`
(bridge support column tiles). Returns count of matching neighbors (0, 1, or 2).

### ComputeAdditionalZ — `FUN_00704350`

Complex function that handles additional Z adjustments:
- Infantry inside transport vehicle with `Jumpjet` locomotor → -3
- Infantry on building with `SpawnsPads` → -14
- Cells with nonzero slope (`cell+0x11C != 0`): checks 3 facing-based neighbors for
  overlay "IsWall" flag at `overlay+0x2B5`, applies gate Z constant
- Facing-dependent Z using `RateTimer::Current()` to select directional neighbors
- Checks for `cell+0x140 & 0x10000` flag (special terrain marker)
- Reads slope type from neighbor cells via `FUN_00547150`

---

## 9. CliffBackImpassability

**RulesClass field:** offset `0x664` (1 byte, stored as `undefined1`)
**Read in:** `RulesClass::ReadGeneral` at `0x0066f1d9` via `CCINIClass::ReadInt`
**INI section:** `[General]`
**Default value:** typically `2` in standard YR

### Purpose

Controls whether cells at the base of cliffs are marked as impassable (LandType = Rock).
Prevents units from pathfinding into the "shadow" area behind cliff faces where they would
be invisible or stuck.

### Values

| Value | Behavior |
|-------|----------|
| `0` | Disabled — no cliff-back impassability |
| `1` | Enters the neighbor check code but does NOT change LandType (effectively disabled) |
| `2` | Enabled — marks cells behind cliffs as Rock (LandType 3 = impassable) |

### Algorithm (in CellClass::RecalcAttributes — `0x0047d2b0`)

The same check appears **3 times** in RecalcAttributes, covering different code paths
(overlay processing, empty tile fallback, final post-processing):

```
function CheckCliffBackImpassability(cell):
    if RulesClass.CliffBackImpassability == 0:
        return  // disabled

    // Check 6 isometric neighbors
    neighbors = [
        (cell.Y - 1, cell.X),      // N
        (cell.Y,     cell.X - 1),   // W
        (cell.Y + 2, cell.X + 2),   // SE
        (cell.Y + 1, cell.X + 1),   // S
        (cell.Y + 1, cell.X - 1),   // SW
        (cell.Y - 1, cell.X + 1),   // NE
    ]

    is_behind_cliff = false
    for each neighbor in neighbors:
        if neighbor.height >= cell.height + 4:
            is_behind_cliff = true
            break

    if is_behind_cliff AND CliffBackImpassability == 2:
        // Final check: only override certain land types
        if cell.LandType in {Clear(0), Water(2), Beach(6), Ice(8)}:
            cell.LandType = Rock (3)  // IMPASSABLE
```

**Key constant:** height difference threshold = **4 levels** (same as bridge height offset).

The 6 neighbors form the isometric adjacency ring. If ANY neighbor is ≥4 levels above the
current cell, the cell is considered "behind a cliff." Only LandTypes that are normally
passable (Clear, Water, Beach, Ice) get overridden — Rock, Road, Wall, etc. are left alone.

---

## 10. INI Keys Summary

### Theater INI `[General]` (tile set indices)

| Key | Type | Default | Purpose |
|-----|------|---------|---------|
| `CliffSet` | int | -1 | First tile set index for cliff tiles (40 tiles) |
| `CliffRamps` | int | -1 | First tile set index for cliff ramp tiles (20 tiles) |
| `WaterCliffs` | int | -1 | First tile set index for water cliff tiles (28 tiles) |
| `DestroyableCliffs` | int | -2 | First tile set index for destroyable cliff tiles (2 tiles) |

### Rules(md).ini `[General]`

| Key | Type | Default | Offset | Purpose |
|-----|------|---------|--------|---------|
| `CliffBackImpassability` | int (byte) | 2 | RulesClass+0x664 | Cliff-back cell impassability mode |

### Rules(md).ini `[BulletTypes]`

| Key | Type | Default | Offset | Purpose |
|-----|------|---------|--------|---------|
| `SubjectToCliffs` | bool | false | BulletTypeClass+0x296 | Bullet blocked by cliff faces |
| `SubjectToElevation` | bool | false | BulletTypeClass+0x297 | Range bonus from height advantage |
| `SubjectToWalls` | bool | false | BulletTypeClass+0x298 | Bullet blocked by wall overlays |

### Rules(md).ini `[TechnoTypes]`

| Key | Type | Default | Offset | Purpose |
|-----|------|---------|--------|---------|
| `ZFudgeCliff` | int | 0 | TechnoTypeClass+0xDC0 | Z-depth bias near cliffs |
| `ZFudgeColumn` | int | 0 | TechnoTypeClass+0xDC4 | Z-depth bias near bridge columns |
| `ZFudgeTunnel` | int | 0 | TechnoTypeClass+0xDC8 | Z-depth bias near tunnels |
| `ZFudgeBridge` | int | 0 | TechnoTypeClass+0xDCC | Z-depth bias on/under bridges |

---

## 11. Integration Points

### Who calls these functions

| Function | Called by | When |
|----------|----------|------|
| `IsCliffOrImpassableTile` (0x004863d0) | Passability checks, movement | On pathfinding queries |
| `IsOnBridgeRamp` (0x00578d80) | Bridge/ramp logic | Movement height transitions |
| `Cliff Variant Randomizer` (0x005a1350) | Map loading, neighbor fixup | Map init and editor |
| `Destroyable Cliff Destruction` (0x00581140) | Damage/destruction system | When cliff takes lethal damage |
| `Bullet Cliff Check` (0x004CC360) | `BulletClass::AI` bounce check | Every bullet tick |
| `Z-Fudge Compositor` (0x004daff0) | TechnoClass draw pipeline | Every frame per visible unit |
| `CliffBackImpassability` (in 0x0047d2b0) | CellClass::RecalcAttributes | On cell attribute recalc |

---

## 12. Current Rust Implementation Status

### Implemented

- **Cliff classification:** String-based heuristic in `theater.rs:214` (`name.contains("cliff")`)
  and `resolved_terrain.rs:940` (`"cliff" || "rock" || "shore"` for `is_cliff_like`).
  **Divergence:** gamemd uses numeric tile set index ranges, not string matching. The string
  approach conflates rocks/shore with actual cliff tile sets.

- **Cliff redraw rendering:** Dual-pass system in `terrain.rs` and `draw_passes.rs`. Cells with
  height difference ≥4 from neighbors get `is_cliff_redraw = true` and are rendered twice
  (normal pass + zdepth pass with Less compare). This matches the concept but the height
  detection algorithm differs from gamemd's approach.

- **Passability:** `terrain_cost.rs` treats `is_cliff_like` cells as `COST_BLOCKED` for all
  ground SpeedTypes. `passability.rs` maps TMP terrain type to `LandType::Rock` for cliff bytes.

- **INI parsing:** `SubjectToCliffs` parsed in `projectile_type.rs:57`. `ZFudgeBridge` parsed
  in `object_type.rs:323`. `TooBigToFitUnderBridge` parsed in `object_type.rs:325`.

### NOT Implemented (that SHOULD be — active in YR)

- **Numeric tile set index classification** — the engine should use CliffSet/CliffRamps/
  WaterCliffs indices from theater INI, not string matching
- **CliffBackImpassability** — not implemented; cells behind cliffs are not marked impassable
- **ZFudgeColumn/ZFudgeTunnel** — not parsed or applied to rendering (60+ units use these)
- **Bullet cliff/wall collision** — SubjectToCliffs parsed but collision check not implemented

### NOT Implemented (and SHOULD NOT be — TS legacy / dormant in YR)

- **DestroyableCliffs** — no YR theater defines them, default -2 = disabled. TS legacy.
- **ZFudgeCliff** — no YR unit sets it to nonzero. System exists but dormant.
- **Cliff variant randomization** — visual only, low priority, called during map load
- **Cliff neighbor fixup** — visual only, low priority

---

## 13. Open Questions

1. **DAT_0089f694 direction offset:** Used in IsNearCliff (FUN_00704240). This is one of the
   8 direction offset pairs at `0x0089f688`. Need to confirm which isometric direction it
   corresponds to (likely NW based on cliff face orientation). **Confidence: MEDIUM**

2. **CliffBackImpassability value 1:** The code checks `!= 0` to enter the neighbor scan but
   only `== 2` to apply LandType change. Value 1 would scan neighbors but never mark cells
   as Rock. Is this intentional (debug mode?) or is 1 simply unused? **Confidence: LOW**

3. **Cliff variant randomizer seed:** Uses `Random::Next()` which is the global non-deterministic
   random, not the sim random. This confirms it's visual-only (not synced in multiplayer).
   **Confidence: HIGH**

4. **Theater INI parsing:** We don't currently parse cliff tile set indices from theater INI
   `[General]` section. Need to verify the exact section format and which theaters have which
   cliff sets. **Confidence: N/A — implementation gap**

---

## 14. Tiberian Sun Legacy Analysis

**Method:** Cross-referenced Ghidra decompilation with actual standard YR rulesmd.ini values
(CNCNet YR client package) to determine which systems are live vs dormant.

### DORMANT — Do NOT implement

| Subsystem | Evidence | Verdict |
|-----------|----------|---------|
| **DestroyableCliffs** | Default is -2. NOT defined in any standard YR theater INI. Tile index -2 never matches any real tile, so `FUN_00486900` always returns false, and `FUN_00581140` (destruction) is never called. | **TS LEGACY — SKIP** |
| **ZFudgeCliff** | **No unit in standard YR rulesmd.ini sets ZFudgeCliff to nonzero.** Default is 0. The proximity check `FUN_00704240` runs and returns 0/1/2, but the result is multiplied by 0 for all units. | **DORMANT — SKIP** |
| **FUN_005a1e10** (cliff fixup wrapper) | Has zero callers (Ghidra xref: "No references found"). Dead code. | **DEAD CODE — SKIP** |

### ACTIVE — Should implement

| Subsystem | Evidence | Priority |
|-----------|----------|----------|
| **CliffBackImpassability** | `CliffBackImpassability=2` in standard YR rulesmd.ini. Code in `CellClass::RecalcAttributes` runs on every cell recalc. Marks cells behind ≥4-level cliffs as Rock (impassable). | **HIGH** — affects pathfinding correctness |
| **SubjectToCliffs** (bullet blocking) | Many bullet types in YR set `SubjectToCliffs=yes`: Cannon, InvisibleLow, InvisibleMedium, etc. Collision check `FUN_004CC360` is called every bullet tick. | **HIGH** — affects combat behavior |
| **ZFudgeColumn** | 60+ units in standard YR set `ZFudgeColumn=7..12`. Prevents units rendering through bridge support columns. | **MEDIUM** — rendering correctness |
| **ZFudgeTunnel** | 60+ units in standard YR set `ZFudgeTunnel=13..15`. Prevents units rendering through tunnel entrances. | **MEDIUM** — rendering correctness |
| **ZFudgeBridge** | 3 units (CMIN, HARV, SMIN) set `ZFudgeBridge=7`. Already partially parsed (`object_type.rs`). | **LOW** — only 3 harvesters affected |
| **Cliff tile set identification** | CliffSet, CliffRamps, WaterCliffs are defined in all YR theater INIs. Maps contain cliff tiles. Required for correct passability checks. | **HIGH** — correctness |
| **Cliff variant randomizer** | Called during `ScenarioClass::Read_Scenario` (map loading) via `FUN_00598960`. Uses non-sim random (visual-only, no lockstep impact). | **LOW** — cosmetic only |
| **Cliff neighbor fixup** | Also called during map loading, ensures cliff edge matching. Visual only. | **LOW** — cosmetic only |

### UNCERTAIN — Verify before implementing

| Subsystem | Question | How to verify |
|-----------|----------|---------------|
| **Waterfall directional passability** | Do standard YR theater INIs define WaterfallEast/West/North/South? If all are -1, the passability exceptions are dormant. | Parse YR theater INIs from MIX archives |
| **IsOnBridgeRamp cliff check** | This function checks CliffSet and CliffRamps ranges. Active if any YR map uses cliff ramp tiles near bridges. | Check standard YR multiplayer maps |

### Key principle for implementation

The ZFudge compositor (`FUN_004daff0`) should be implemented, but **only wire up
ZFudgeColumn, ZFudgeTunnel, and ZFudgeBridge**. Do NOT implement `IsNearCliff`
(`FUN_00704240`) or parse `ZFudgeCliff` — it's dead weight in YR.

Similarly, the tile set identification should use `CliffSet`, `CliffRamps`, and `WaterCliffs`
ranges but should NOT implement `DestroyableCliffs` checks or the destruction system.

---

## Sources

### Ghidra addresses decompiled
- `0x00545714` — Tile set loading (CliffSet, CliffRamps, WaterCliffs, DestroyableCliffs read)
- `0x004863d0` — IsCliffOrImpassableTile
- `0x00578d80` — IsOnBridgeRamp
- `0x005a1350` — Cliff variant randomizer
- `0x005a17f0` — Cliff neighbor fixup
- `0x00581140` — Destroyable cliff destruction
- `0x00486900` — IsDestroyableCliff check
- `0x004CC360` — Bullet cliff/wall collision
- `0x004daff0` — ZFudge compositor
- `0x00704240` — IsNearCliff (cliff proximity for ZFudge)
- `0x00703e70` — IsNearBridgeColumn
- `0x00704000` — IsNearTunnel
- `0x00703b10` — IsOnBridge
- `0x00704350` — ComputeAdditionalZ
- `0x0047d2b0` — CellClass::RecalcAttributes (CliffBackImpassability consumer)
- `0x00487d50` — CellClass::GetEffectiveHeight
- `0x0066f1d9` — RulesClass::ReadGeneral (CliffBackImpassability read)
- `0x0046bfeb` — BulletTypeClass::ReadINI (SubjectToCliffs read)
- `0x00715423` — TechnoTypeClass::ReadINI (ZFudgeCliff read)

### INI files checked
- `ini/rulesmd.ini`, `ini/rules.ini` — SubjectToCliffs, ZFudge values
- `ini/artmd.ini`, `ini/art.ini` — no cliff-specific keys found
- Theater INIs (temperatemd.ini etc.) — not present in repo, values read from MIX archives

### Existing docs referenced
- `BULLET_CLASS_AI_GHIDRA_REPORT.md` — SubjectToCliffs field offset verification
- `TERRAIN_COST_FACTSHEET.md` — LandType enum, passability matrix
- `BRIDGE_SYSTEM.md` — Bridge flags (0x80, 0x100), height conventions
- `COORDINATE_ATOMS_AUDIT.md` — Height-to-pixel conversion constants
- `ZONE_PASSABILITY_VERIFIED.md` — MovementZone/SpeedType passability matrix
- `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` — Height difference thresholds
- `VOXEL_SLOPE_TILT_SYSTEM.md` — Slope type values
