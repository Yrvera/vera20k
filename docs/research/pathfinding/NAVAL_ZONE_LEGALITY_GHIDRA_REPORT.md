# Naval/Water Zone Legality for Pathfinding — Ghidra Report

**Date:** 2026-04-04
**Confidence:** 95%+ (directly decompiled from gamemd.exe, cross-verified across multiple functions)
**Extends:** ZONE_PASSABILITY_VERIFIED.md, TODO_ZONE_FIDELITY_FIXES.md

## Executive Summary

The original game prevents ships from pathfinding through land using a **three-layer system**:

1. **Cell ZoneType classification** — Each cell is assigned one of 8 ZoneType values (0-7) by `CellClass::RecalcZoneType`. Water cells get ZoneType 4 (Water), land cells get ZoneType 0 (Ground), etc.

2. **Passability matrix check** — A 13x8 matrix at `0x82A594` indexed by `[MovementZone][ZoneType]`. Water (row 10) only passes on ZoneType 4 (Water). WaterBeach (row 11) passes on ZoneType 3 (Beach) and 4 (Water). All land ZoneTypes (0, 1, 2, 5, 6) are blocked for both.

3. **Zone ID equality check** — Connected regions of passable cells are flood-filled into zones per MovementZone. Before pathfinding, `MapClass::Can_Reach_Zone` compares zone IDs of source and destination. If they differ, the path request is rejected instantly. Ships in a lake cannot path to ships in the ocean because each water body is a separate zone.

The **root cause of the bug in our engine** is that `can_use_reduced_zone_precheck()` in `zone_search.rs` returns `false` for Water/WaterBeach movers, bypassing the zone pre-check entirely. Those movers then go straight to unrestricted A*, which correctly uses the passability matrix for individual cell checks but has no macro-level constraint preventing the search from exploring land cells (via the `is_water_surface_cell_passable` fallback on `cell.is_water`).

---

## 1. ZoneType Enum — The 8 Passability Matrix Columns

**Source:** `CellClass::RecalcZoneType` at `0x483C80`
**Storage:** `CellClass+0x4C` (4-byte int field)
**Confidence:** VERIFIED

The 8 ZoneType values determine the column index into the passability matrix:

| ZoneType | Value | Name           | Assignment Rule (from RecalcZoneType) |
|----------|-------|----------------|---------------------------------------|
| Ground   | 0     | Passable land  | Default fallthrough — most terrain ends here |
| Road     | 1     | Road overlay   | Cell has overlay with `IsCrate` flag (OverlayType+0x22D) (corrected 2026-05-29: was "IsRoad flag"; binary at 0x483C9A checks +0x22D which is ObjectTypeClass::Crushable=/IsCrate per CELLCLASS_STRUCT_GHIDRA_REPORT.md — RTTI_LABEL_DRIFT; verified via decompile_function 0x00483C80) |
| Wall     | 2     | Wall overlay   | Cell has overlay with `IsWall` flag (OverlayType+0x2A8) |
| Beach    | 3     | Beach terrain  | CellClass::LandType == 6 |
| Water    | 4     | Deep water     | CellClass::LandType == 2 |
| Building | 5     | Building/gate  | Building object on cell (with conditions) |
| Impassable | 6   | Blocked terrain | IsGate overlay (OverlayType+0x2B5), speed==0.0 overlay, impassable terrain (corrected 2026-05-29: was "Tiberium overlay"; binary checks +0x2B5=IsGate, not IsTiberium — INFERENCE_HARDENED; verified via decompile_function 0x00483C80) |
| OutOfBounds | 7  | Outside playfield | Cell not in playfield diamond |

**Key insight:** Rough terrain, Railroad terrain, Ice terrain, and road terrain (without road overlay) ALL fall through to ZoneType 0 (Ground). Only Water, Beach, overlays, buildings, and speed=0 terrain get non-zero ZoneTypes.

### RecalcZoneType Decision Tree (0x483C80)

```
1. If not in playfield → ZoneType 7 (OoB), RETURN
2. Check overlay (if OverlayTypeIndex != -1):
   a. If overlay IsCrate (OverlayType+0x22D) → ZoneType 1 (Road), RETURN
      (corrected 2026-05-29: was "IsRoad"; +0x22D is IsCrate/Crushable — RTTI_LABEL_DRIFT; decompile_function 0x00483C80)
   b. If overlay IsWall (OverlayType+0x2A8) → ZoneType 2 (Wall), RETURN
   c. If speed_table[overlay.LandType * 9] == 0.0 → ZoneType 6 (Impassable), RETURN
      (exact float equality to 0.0, not <= 0.01 — the <= 0.01 threshold only applies to step 4 below)
   d. If overlay IsGate (OverlayType+0x2B5) → ZoneType 6 (Impassable), RETURN
      (corrected 2026-05-29: was "IsTiberium"; binary at 0x483CC8 checks +0x2B5 IsGate — INFERENCE_HARDENED; decompile_function 0x00483C80)
   e. If overlay IsVeinholeMonster (OverlayType+0x2B4) → ZoneType 0 (Ground), RETURN
      (corrected 2026-05-29: MISSING step — binary has explicit goto default (ZoneType=0) when +0x2B4 is set; decompile_function 0x00483C80)
3. Check base LandType:
   a. LandType == 2 (Water) → ZoneType 4 (Water), RETURN
   b. LandType == 6 (Beach) → ZoneType 3 (Beach), RETURN
4. If SpeedType_LandType_Table[landType*9] <= 0.01 → ZoneType 6 (Impassable), RETURN
5. Check objects on cell (buildings, terrain objects) → ZoneType 5 or 6
6. Default → ZoneType 0 (Ground)
```

---

## 2. The Passability Matrix — 0x82A594

**Address:** `0x82A594` (416 bytes = 13 rows × 8 cols × 4 bytes per u32)
**Confidence:** VERIFIED — directly read from binary memory

Rows = MovementZone (0-12), Columns = ZoneType (0-7).
Values: 1 = passable, 2 = blocked, 3 = permanent sentinel (col 7 only).

```
                        Col0    Col1    Col2    Col3    Col4    Col5    Col6    Col7
                        Ground  Road    Wall    Beach   Water   Bldg    Impass  OoB
 0 Normal               OK      X       X       X       X       X       X       !!!
 1 Crusher              OK      OK      X       X       X       X       X       !!!
 2 Destroyer            OK      OK      OK      X       X       X       X       !!!
 3 AmphibDestroyer      OK      OK      OK      OK      OK      OK      X       !!!
 4 AmphibCrusher        OK      OK      X       OK      OK      X       X       !!!
 5 Amphibious           OK      X       X       OK      OK      X       X       !!!
 6 Subterranean         OK      OK      OK      X       X       X       OK      !!!
 7 Infantry             OK      X       X       X       X       OK      X       !!!
 8 InfantryDestroyer    OK      OK      OK      X       X       OK      X       !!!
 9 Fly                  OK      OK      OK      OK      OK      OK      OK      !!!
10 Water                X       X       X       X       OK      X       X       !!!
11 WaterBeach           X       X       X       OK      OK      X       X       !!!
12 CrusherAll           OK      OK      OK      X       X       X       X       !!!
```

**Critical for ships:**
- **Row 10 (Water):** ONLY passes on column 4 (Water). Every other terrain type blocks ships.
- **Row 11 (WaterBeach):** Passes on columns 3 (Beach) AND 4 (Water). Blocked everywhere else.
- Ships CANNOT traverse Ground (col 0), Road (col 1), Wall (col 2), Building (col 5), or Impassable (col 6).

---

## 3. MovementZone Enum — 13 Values

**INI parse table:** `0x81BA88` (pointer array, 13 entries)
**Parsed by:** `CCINIClass::ReadMovementZone` at `0x474E40`
**Stored at:** `TechnoTypeClass+0x5B4` (byte offset, = param_1[0x16D] with int* param_1)
**Confidence:** VERIFIED

| Index | Name                | INI Value            |
|-------|---------------------|----------------------|
|  0    | Normal              | `MovementZone=Normal` |
|  1    | Crusher             | `MovementZone=Crusher` |
|  2    | Destroyer           | `MovementZone=Destroyer` |
|  3    | AmphibiousDestroyer | `MovementZone=AmphibiousDestroyer` |
|  4    | AmphibiousCrusher   | `MovementZone=AmphibiousCrusher` |
|  5    | Amphibious          | `MovementZone=Amphibious` |
|  6    | Subterranean        | `MovementZone=Subterranean` |
|  7    | Infantry            | `MovementZone=Infantry` |
|  8    | InfantryDestroyer   | `MovementZone=InfantryDestroyer` |
|  9    | Fly                 | `MovementZone=Fly` |
| 10    | Water               | `MovementZone=Water` |
| 11    | WaterBeach          | `MovementZone=WaterBeach` |
| 12    | CrusherAll          | `MovementZone=CrusherAll` |

The MovementZone value IS the direct row index into the passability matrix. No intermediate mapping.

---

## 4. SpeedType Enum — 8 Values

**Name table:** `g_SpeedTypeNameTable` at `0x81DA58` (pointer array, 8 entries)
**Parsed by:** `SpeedType::FromName` at `0x48DFF0`
**Stored at:** `TechnoTypeClass+0x67C` (byte offset)
**Confidence:** VERIFIED

| Index | Name       | Binary ptr address |
|-------|------------|-------------------|
|  0    | Foot       | 0x81DBD4 |
|  1    | Track      | 0x81DBCC |
|  2    | Wheel      | 0x81DBC4 |
|  3    | Hover      | 0x81DBBC |
|  4    | Winged     | 0x81DBB4 |
|  5    | Float      | 0x81DBAC |
|  6    | Amphibious | 0x81BB18 |
|  7    | FloatBeach | 0x81DBA0 |

SpeedType controls speed multipliers per terrain (via `SpeedType_LandType_Table` at `0x89EA44`), NOT passability. MovementZone controls passability.

---

## 5. Zone Flood-Fill — How Water Zones are Computed

### Per-Cell Zone Assignment

Each cell stores:
- **ZoneType** at `CellClass+0x4C` — set by `RecalcZoneType` (0-7, determines passability column)
- **NodeIndex** — cell position in the zone grid array (linearized map coordinate)
- **Zone IDs** — stored in the ZoneMap arrays, one per MovementZone level

### Zone Levels

The zone map maintains **3 zone levels** (indexed 0, 1, 2), corresponding to hierarchical detail:
- Level 2 = coarsest (used first in `Zone_precheck`)
- Level 1 = medium
- Level 0 = finest

Each level has its own zone ID array. `ZoneMap::BuildZoneLevel` at `0x581F90` builds zones for each level using `FloodFillScanline` (0x5824A0).

### Flood-Fill Algorithm

`ZoneMap::FloodFillScanline` (0x5824A0) is a recursive scanline flood-fill:
1. Start at an unassigned passable cell.
2. Expand left and right along the scanline while cells have the same ZoneType and similar height (abs height diff <= 1 for left, <= 3 for right scan).
3. Assign the current zone ID to all cells in the span.
4. Recurse into the row above and below the span.
5. When encountering cells with DIFFERENT existing zone IDs, record zone adjacency edges.

**Key passability gate during flood-fill** (from `FloodFillReachableZones` at 0x5840C0):
```c
if (g_PassabilityMatrix[movementZone * 8 + cell->ZoneType] != 1) {
    // Cell is NOT passable for this movement zone — stop expansion
}
```

This means water cells (ZoneType 4) get flood-filled into zones only for MovementZones where column 4 is passable (rows 3, 4, 5, 9, 10, 11 — i.e., amphibious, fly, water, waterbeach). Land cells (ZoneType 0) get flood-filled into zones only for MovementZones where column 0 is passable (rows 0-9, 12 — everything except Water and WaterBeach).

**Result:** For MovementZone 10 (Water), the flood fill ONLY expands through ZoneType 4 (Water) cells. Each isolated water body becomes its own zone ID. Land cells are never assigned to any Water zone.

---

## 6. Zone Pre-Check in Pathfinding

### MapClass::Can_Reach_Zone (0x56D100)

**Confidence:** VERIFIED

This is the **first** reachability gate. Simple zone ID equality check:

```c
bool Can_Reach_Zone(CellCoord* src, CellCoord* dst, int movementZone, ...) {
    if (movementZone == -1) return true;  // No zone constraint
    
    // Handle out-of-playfield edge cases
    if (!Is_Cell_In_Playfield(src) && cell_in_border) return true;
    if (src_in_playfield && !Is_Cell_In_Playfield(dst) && cell_in_border) return true;
    
    int zone_src = GetZoneID(src, movementZone, ...);
    int zone_dst = GetZoneID(dst, movementZone, ...);
    return zone_src == zone_dst;
}
```

This is called in `AStar_pathfind_search` (0x42C900) BEFORE any A* search begins:

```c
int AStar_pathfind_search(...) {
    // Get MovementZone from unit's type class
    int movementZone = *(int*)(typeClass + 0x5B4);
    
    // Get zone IDs for source and destination
    int zone_src = MapClass::GetZoneID(src_cell, movementZone, ...);
    int zone_dst = MapClass::GetZoneID(dst_cell, movementZone, ...);
    
    if (zone_src == zone_dst) {
        // Same zone — try hierarchical precheck then A*
        if (allowHS) {
            if (!Zone_precheck(...)) {
                // Hierarchical findpath failure
                allowHS = false;
            }
        }
    } else {
        // DIFFERENT zones — bail out immediately if HS was allowed
        if (allowHS) {
            return 0;  // ← INSTANT REJECTION
        }
    }
    
    // ... proceed with A* main loop ...
}
```

**For ships:** If a ship at a water cell (zone ID = X for Water zones) is ordered to move to a land cell (zone ID = INVALID or different zone ID for Water), `zone_src != zone_dst`, so the pathfind returns 0 immediately. No A* search is performed at all.

### MapClass::GetZoneID (0x56D230)

```c
uint GetZoneID(CellCoord* coord, int movementZone, bool checkBridge) {
    // Handle bridge cells (if onBridge flag set, may redirect to bridge zone)
    ...
    
    // Convert cell coordinate to linear index
    int linearIdx = (mapWidth + 1) * coord->y + coord->x;
    linearIdx = clamp(linearIdx, 0, totalCells - 1);
    
    // Look up zone ID from per-MovementZone array
    // MapClass+0x18 contains pointer array[3], each pointing to zone ID arrays
    // Node index is at *(ushort*)(nodeArray + linearIdx * 4 + 2)
    uint nodeIdx = *(ushort*)(zoneNodeArray + linearIdx * 4 + 2);
    return *(ushort*)(zoneIdArray[movementZone] + nodeIdx * 2);
}
```

**Critical detail:** The zone ID arrays are per-MovementZone. Each of the 13 MovementZones has its own zone ID table. Water cells will have valid zone IDs in the Water (10) and WaterBeach (11) tables but ZONE_INVALID (0) in the Normal (0) table. Land cells will have valid zone IDs in the Normal (0) table but ZONE_INVALID (0) in the Water (10) table.

---

## 7. Zone_precheck — Hierarchical Zone Pathfinding (0x42C290)

**Confidence:** 85% (complex function, key logic verified but some details unclear)

Zone_precheck is the **hierarchical pathfinder** that operates on the zone adjacency graph. It runs Dijkstra/A* on zones rather than cells, producing a coarse zone-to-zone corridor for the cell-level A* to follow.

Key behavior:
1. Iterates through 3 zone levels (level 2 → 1 → 0, coarsest to finest).
2. At each level, gets zone IDs for source and destination.
3. If same zone at this level → done (trivially connected).
4. Otherwise, runs a priority-queue search on the zone adjacency graph.
5. Uses `g_PassabilityMatrix[movementZone * 8 + zoneType]` to gate which zone edges are traversable.
6. Records the zone corridor for A* to follow.

The function signature (reconstructed):
```c
bool Zone_precheck(
    PathfinderClass* this,
    CellCoord* src,
    CellCoord* dst,
    int movementZone,     // MovementZone enum value (0-12)
    TechnoClass* unit     // The unit being pathfound
);
```

**For ships:** Zone_precheck with MovementZone=10 (Water) will only find paths through zones that consist of Water cells (ZoneType 4). If source and destination are in different disconnected water bodies, no zone path is found and it returns false.

---

## 8. SpeedType_LandType_Table — Speed Multipliers (0x89EA44)

**Parsed by:** `RulesClass::ReadSpeedTypeLandTypeTable` at `0x674000`
**Confidence:** VERIFIED

This is a separate table from the passability matrix. It stores **speed multipliers** (0.0 to 1.0) for each SpeedType × LandType combination. Read from INI sections like `[Clear]`, `[Rough]`, etc. with keys `Foot=`, `Track=`, `Wheel=`, `Hover=`, `Float=`, `Amphibious=`, `Winged=`.

Each LandType section has 9 float entries (7 SpeedTypes + padding + Buildable flag).

When `RecalcZoneType` checks `speed[LandType * 9] <= 0.01`, it's reading the first SpeedType (Foot) speed for that LandType. If Foot speed is near-zero, the cell is classified as Impassable (ZoneType 6).

This table determines movement SPEED, not passability. A cell can be passable (matrix says OK) but have a low speed multiplier (slow movement).

---

## 9. How Ships are Confined — End-to-End Flow

### Ship unit example: Destroyer (Dreadnought, Aegis Cruiser, etc.)
- `MovementZone=Water` → index 10
- `SpeedType=Float` → index 5
- `Locomotor={ship CLSID}`

### When player orders ship to move to land cell:

1. **Can_Reach_Zone check:** Gets zone ID for ship's current cell using MovementZone=10 (Water). Gets zone ID for target land cell using MovementZone=10. Since land cells have no Water zone ID (they're ZONE_INVALID or in a different zone), `zone_src != zone_dst`. **Result: path rejected instantly, return 0.**

2. **No A* search performed.** The zone pre-check is an O(1) operation that prevents wasting CPU on impossible paths.

### When player orders ship to move to water cell in same body:

1. **Can_Reach_Zone:** Both cells have the same Water zone ID. Returns true.
2. **Zone_precheck:** Finds zone corridor (trivial if same zone at level 2).
3. **A* search:** Expands cells using `CellClass::CheckCellPassability` which calls `GetZoneID` to verify zone compatibility, then checks `SpeedType_LandType_Table` for traversal speed.

### When player orders ship to move to water cell in DIFFERENT body (e.g., across land):

1. **Can_Reach_Zone:** Zone IDs differ (each water body has its own zone ID under MovementZone=10). Returns false. **Path rejected instantly.**

---

## 10. Diagnosis of the Rust Engine Bug

### Current behavior (broken)
Ships can pathfind through land cells.

### Root cause analysis

**Problem 1: Zone precheck bypassed for water movers**

In `src/sim/pathfinding/zone_search.rs`, the function `can_use_reduced_zone_precheck()` returns `false` for all MovementZones except Normal, Amphibious, Infantry, and Fly. Water and WaterBeach movers skip the zone pre-check entirely and go directly to unrestricted A*:

```rust
fn can_use_reduced_zone_precheck(movement_zone: Option<MovementZone>) -> bool {
    match movement_zone {
        Some(MovementZone::Normal | MovementZone::Amphibious 
             | MovementZone::Infantry | MovementZone::Fly) => true,
        Some(_) => false,  // ← Water/WaterBeach skip zone precheck!
        ...
    }
}
```

When zone precheck is skipped, `find_path_zoned` calls `find_path_with_costs` directly, which runs unrestricted A* without any zone corridor constraint.

**Problem 2: Cell-level passability fallback is too permissive**

In `is_water_surface_cell_passable()`, there's a fallback:
```rust
if cell.is_water {
    return true;  // ← Treats any cell flagged as water surface as passable
}
```

Combined with bypassed zone precheck, the A* may find paths through cells that are technically water-surface flagged but border land, creating paths that traverse land areas.

**Problem 3: Only 6 zone categories instead of 13 per-MovementZone**

The original engine has 13 separate zone ID arrays — one per MovementZone. Our engine has 6 ZoneCategory maps that group multiple MovementZones into shared representatives:
- ZoneCategory::Water uses representative MovementZone::Water
- ZoneCategory::WaterBeach uses representative MovementZone::WaterBeach

This is structurally correct for water movers (Water/WaterBeach each get their own zone maps), but the zone precheck bypass (Problem 1) means these maps are never consulted.

### Fix approach (for implementation, not this document)

The minimal fix is to enable zone precheck for Water and WaterBeach movers by adding them to `can_use_reduced_zone_precheck()`. The zone maps for ZoneCategory::Water and ZoneCategory::WaterBeach already exist and are built with the correct passability checks — they just need to be used.

The zone maps correctly partition water bodies into separate zones because `zone_build` uses `is_passable_for_zone()` which checks `PASSABILITY_MATRIX[10][landType]`, and only LandType::Water (col 4) passes for MovementZone::Water. The problem is purely that the zone check is skipped in the pathfinding entry path.

---

## 11. Functions Referenced

| Address    | Name | Purpose |
|------------|------|---------|
| 0x42C290   | Zone_precheck | Hierarchical zone-level pathfinding |
| 0x42C900   | AStar_pathfind_search | Main pathfinding entry point |
| 0x474E40   | CCINIClass__ReadMovementZone | Parse MovementZone from INI |
| 0x476FC0   | CCINIClass__ReadSpeedType | Parse SpeedType from INI |
| 0x483C80   | CellClass__RecalcZoneType | Assign ZoneType (0-7) to cell |
| 0x4834A0   | CellClass__CheckCellPassability | Cell-level passability check during A* |
| 0x48DFF0   | SpeedType__FromName | Parse SpeedType enum from string |
| 0x48DF80   | MovementZone_From_Name | Parse MovementZone enum from string |
| 0x56CB90   | MapClass__ZoneFloodFillScanLine | Scanline flood-fill for ground zones |
| 0x56D100   | MapClass__Can_Reach_Zone | O(1) zone reachability check |
| 0x56D230   | MapClass__GetZoneID | Get zone ID for cell + MovementZone |
| 0x581F90   | ZoneMap__BuildZoneLevel | Build zones for one hierarchy level |
| 0x5824A0   | ZoneMap__FloodFillScanline | Zone flood-fill (hierarchical levels) |
| 0x5840C0   | ZoneMap__FloodFillReachableZones | Flood-fill reachable zones from cell |
| 0x5889F0   | ZoneMap__FindBestCompatibleMovementZone | Team pathfinding zone merger |
| 0x674000   | RulesClass__ReadSpeedTypeLandTypeTable | Parse speed multiplier tables |
| 0x82A594   | g_PassabilityMatrix | 13×8 passability matrix (data) |

---

## 12. TS Legacy Check

All functions documented here are actively called in standard YR skirmish gameplay:
- Zone flood-fill happens during map loading
- Zone precheck happens every time a unit pathfinds
- `Can_Reach_Zone` is called before every A* search
- No SpecialFlags or TS-only gates detected in the call paths

**Verdict:** This is all live YR code, not TS legacy.
