# Zone Passability System -- Verified from Binary

**Date:** 2026-03-23
**Confidence:** 95%+ (directly decompiled from gamemd.exe, cross-verified across multiple functions)

## Summary of Corrections

The existing research (TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md and others) contains
several errors about TechnoTypeClass+0x5B4, the passability matrix column meanings, and
enum orderings. This document provides verified corrections.

---

## 1. TechnoTypeClass+0x5B4 is MovementZone (NOT a computed ZoneSpeedCategory)

**Previous claim:** +0x5B4 is a "ZoneSpeedCategory" computed from SpeedType + MovementZone.

**Verified truth:** +0x5B4 **is** the MovementZone field, parsed directly from the INI
`MovementZone=` key in `TechnoTypeClass::ReadINI` at address `0x00716065`:

```c
// In TechnoTypeClass__ReadINI (param_1 is int*):
iVar4 = CCINIClass__ReadMovementZone();  // 0x00474e40
param_1[0x16d] = iVar4;                 // 0x16d * 4 = byte offset 0x5B4
```

The value is used **directly** as a row index into the 13x8 passability matrix at
0x82a594. No intermediate computation maps SpeedType+MovementZone into a combined
category. MovementZone (0-12) IS the row index.

**Evidence:**
- `AStar_pathfind_search` at 0x42c900 reads `*(uint*)(typeClass + 0x5b4)` and passes it
  directly to `MapClass__GetZoneID` and `Zone_precheck` as the zone layer.
- `ZoneMap__FloodFillReachableZones` at 0x5840c0 reads the same offset and uses it as
  `g_PassabilityMatrix[movementZone * 8 + cellZoneType]`.

The only "computed" zone category in the engine is `ZoneMap__FindBestCompatibleMovementZone`
(0x5889f0), which finds the best compatible passability row for a **team** of units with
different MovementZones. This is used by team pathfinding, not individual unit types.

## 2. TechnoTypeClass+0x67C is SpeedType (confirmed)

Parsed from `SpeedType=` INI key at address `0x007121e5`:
```c
iVar4 = CCINIClass__ReadSpeedType();   // 0x00476fc0
param_1[0x19f] = iVar4;               // 0x19f * 4 = byte offset 0x67C
```

## 3. TechnoTypeClass+0xC94 is IsTrain (confirmed)

**Previous claims varied:** Some docs said +0xE0C, others +0xC94.

**Verified:** IsTrain is at byte offset **0xC94** (param_1[0x325] with int* param_1).
Parsed from `IsTrain=` INI key at address `0x00712277`:

```c
// s_IsTrain_008444bc = "IsTrain"
uVar5 = CCINIClass__ReadBool();
*(char *)(param_1 + 0x325) = (char)uVar5;   // 0x325 * 4 = 0xC94
```

Cross-verified: `AStar_pathfind_search` checks `*(char*)(typeClass + 0xc94) == '\0'`
at 0x42cb0e to gate zone precheck behavior.

## 4. MovementZone Enum (13 values, table at 0x81ba88)

Parsed by `CCINIClass__ReadMovementZone` at 0x474e40 (string table lookup).

| Index | Name               | Passability Profile |
|-------|--------------------|---------------------|
|  0    | Normal             | Ground only |
|  1    | Crusher            | Ground + can crush fences |
|  2    | Destroyer          | Ground + can destroy walls |
|  3    | AmphibiousDestroyer| Land + water + beach, can destroy |
|  4    | AmphibiousCrusher  | Land + water + beach (no rough) |
|  5    | **Amphibious**     | Land + water (no rough) |
|  6    | Subterranean       | Ground + underground railroad |
|  7    | Infantry           | Ground + tiberium (foot only) |
|  8    | InfantryDestroyer  | Ground + tiberium + can destroy |
|  9    | Fly                | Everything except rock |
| 10    | Water              | Water only |
| 11    | WaterBeach         | Water + beach |
| 12    | CrusherAll         | Same as Destroyer |

**BUG in Rust code:** `src/rules/locomotor_type.rs` MovementZone enum is **missing
Amphibious (index 5)** and has a different variant ordering. This means:
- Only 12 variants instead of 13
- Numeric indices after position 4 are all shifted
- The `zone_layer_for_movement_zone()` function in passability.rs maps multiple
  MovementZones to shared rows, which is wrong -- each MovementZone IS its own row

## 5. SpeedType Enum (8 values, table at 0x81da58)

| Index | Name       |
|-------|------------|
|  0    | Foot       |
|  1    | Track      |
|  2    | Wheel      |
|  3    | **Hover**  |
|  4    | **Winged** |
|  5    | **Float**  |
|  6    | **Amphibious** |
|  7    | **FloatBeach** |

**BUG in Rust code:** `src/rules/locomotor_type.rs` SpeedType enum has a different order:
Foot, Track, Wheel, Float, Amphibious, Winged, FloatBeach, Hover. The numeric values
don't match the binary (e.g., Hover is 3 in binary but 7 in Rust). This affects any code
that converts to/from raw integer indices for the speed/landtype table.

## 6. Passability Matrix at 0x82a594 (416 bytes = 13 rows x 8 cols x 4 bytes)

Values: 1 = passable, 2 = impassable, 3 = impassable sentinel (out-of-bounds/rock cells)
**[Corrected 2026-04-06: Value 3 is NOT "always-passable" — it is impassable like 2. Only value 1 passes the `== 1` check in zone logic. Value 3 is a sentinel distinguishing permanent OoB/rock from regular blocked terrain (value 2).]**

### Column Meanings (Zone Types from CellClass__RecalcZoneType at 0x483c80)

| Col | Zone Type         | Assigned When |
|-----|-------------------|---------------|
|  0  | Ground/Clear      | Default passable land (after checking overlays/objects) |
|  1  | Crushable overlay/object | Overlay/object inherited `ObjectTypeClass+0x22D Crushable=` flag is true; `Crate=` and road art are not read here |
|  2  | Wall              | Cell has wall overlay (OverlayType::IsWall at +0x2A8) |
**[Corrected 2026-05-21: Column 1 is overlay/object `Crushable=` at inherited ObjectTypeClass+0x22D, not `Crate=`, `IsRoad`, or visible road art. Column 2 is Wall (IsWall), not "Water (overlay)" — verified from RecalcZoneType at 0x483c80.]**
|  3  | Beach             | CellClass::LandType == 6 (Beach) |
|  4  | Water (deep)      | CellClass::LandType == 2 (Water) |
|  5  | Building          | TerrainType building occupies cell |
|  6  | Impassable        | Tiberium overlay (speed=0), blocking overlays, various |
|  7  | Out of bounds     | Cell outside playfield; always value 3 in matrix |

NOTE: These are NOT the same as the 12-value LandType enum (Clear, Road, Water, Rock,
Wall, Tiberium, Beach, Rough, Ice, Railroad, Tunnel, Weeds). The 8 zone types are a
reduced classification used specifically by the zone system.

### Full Matrix

```
                    Col0  Col1  Col2  Col3  Col4  Col5  Col6  Col7
                    Grnd  Crsh  Wall  Bch   Wtr   Bldg  Imps  OoB
 0 Normal            1     2     2     2     2     2     2     3
 1 Crusher           1     1     2     2     2     2     2     3
 2 Destroyer         1     1     1     2     2     2     2     3
 3 AmphibDestroyer   1     1     1     1     1     1     2     3
 4 AmphibCrusher     1     1     2     1     1     2     2     3
 5 Amphibious        1     2     2     1     1     2     2     3
 6 Subterranean      1     1     1     2     2     2     1     3
 7 Infantry          1     2     2     2     2     1     2     3
 8 InfantryDestroyer 1     1     1     2     2     1     2     3
 9 Fly               1     1     1     1     1     1     1     3
10 Water             2     2     2     2     1     2     2     3
11 WaterBeach        2     2     2     1     1     2     2     3
12 CrusherAll        1     1     1     2     2     2     2     3
```

### Notable Observations

- **Normal (0)**: Only passes on bare ground. Cannot traverse roads, water, or any special terrain.
- **Crusher (1)**: Ground + roads. The "crushing" ability is handled by gameplay code, not the zone system.
- **Subterranean (6)**: Unique profile -- passes ground, road, wall, AND the "impassable" column (railroad/tunnel).
- **Infantry (7)**: Ground + buildings (column 5). Infantry can enter garrisons.
- **Fly (9)**: Passes everything except rock/OoB sentinel.
- **CrusherAll (12)**: Same profile as Destroyer (2). The difference is in crush behavior, not zone passability.
- Column 7 is always 3 (sentinel) -- used for out-of-bounds cells.

## 7. ZoneMap__FindBestCompatibleMovementZone (0x5889f0)

This function is used for **team pathfinding** to find the most permissive MovementZone
row that is compatible with all unit types in the team.

```c
int FindBestCompatibleMovementZone(int movZone1, int movZone2) {
    int bestRow = -1, bestScore = 0;
    for (int candidate = 0; candidate < 13; candidate++) {
        bool valid = true;
        int score = 0;
        for (int col = 0; col < 8; col++) {
            int cv = matrix[candidate][col];
            int v1 = matrix[movZone1][col];
            int v2 = matrix[movZone2][col];
            // Disqualify if candidate passes terrain that either input blocks
            if ((v1 == 2 || v2 == 2) && cv == 1) valid = false;
            // Count columns where all three agree as passable
            if (v1 == cv && v2 == cv && cv == 1) score++;
        }
        if (valid && score > bestScore) {
            bestScore = score;
            bestRow = candidate;
        }
    }
    return bestRow;
}
```

Called from `TeamTypeClass__ComputeZoneCategory` (0x6f1fa0) during scenario init.

## 8. Sin/Cos Lookup Table Naming

**Table at 0x84F084:** 8192-entry sine table covering a full 2*pi cycle.
- table[0] = 0.0 = sin(0)
- table[2048] = 1.0 = sin(pi/2)
- table[4096] ~= 0.0 = sin(pi)
- table[6144] = -1.0 = sin(3*pi/2)

**Sin_lookup (0x4CAD00):** Adds 2048 to the index before lookup.
- Sin_lookup(0) = table[2048] = 1.0 = **cos(0)**
- **This function returns COSINE despite its name.**

**Cos_lookup (0x4CACB0):** No index shift.
- Cos_lookup(0) = table[0] = 0.0 = **sin(0)**
- **This function returns SINE despite its name.**

**Confirmed via Matrix3x4_RotateZ (0x5af1a0):**
Standard Z-rotation: `new_row = [m00*cos + m01*sin, -m00*sin + m01*cos]`
Code uses: `S = Sin_lookup(angle)` for cosine term, `C = Cos_lookup(angle)` for sine term.
The naming follows the RA2 engine convention (likely 0=North facing), not standard
math convention (0=East facing).

## 9. MovementZone Side Effect in ReadINI

After parsing MovementZone, the engine also computes:
```c
*(bool*)(param_1 + 0x34b) = (movementZone == 6);  // byte offset 0xD2C
```
This flags Subterranean types (MovementZone index 6) with a special bool.

## Functions Labeled in This Session

| Address    | Name |
|------------|------|
| 0x00474e40 | CCINIClass__ReadMovementZone |
| 0x00476fc0 | CCINIClass__ReadSpeedType |
| 0x00483c80 | CellClass__RecalcZoneType |
| 0x005889f0 | ZoneMap__FindBestCompatibleMovementZone |
| 0x0056cb90 | MapClass__ZoneFloodFillScanLine |
| 0x006f1fa0 | TeamTypeClass__ComputeZoneCategory |
| 0x006f2040 | TeamTypeClass__RecomputeAllZoneCategories |

## Rust Code Issues Found (DO NOT FIX -- research only)

1. **MovementZone enum missing Amphibious:** 12 variants instead of 13. All indices after
   position 4 are shifted relative to the binary.
2. **MovementZone enum wrong order:** Even ignoring Amphibious, the order doesn't match
   (e.g., AmphibiousCrusher before AmphibiousDestroyer in Rust, reversed in binary).
3. **SpeedType enum wrong order:** Binary order is Foot,Track,Wheel,Hover,Winged,Float,
   Amphibious,FloatBeach. Rust order is Foot,Track,Wheel,Float,Amphibious,Winged,FloatBeach,Hover.
4. **zone_layer_for_movement_zone() is architecturally wrong:** Each MovementZone IS its
   own row (0-12). There is no separate "zone layer" mapping -- the function should be
   removed and MovementZone used directly as the index.
5. **LandType column labels partially wrong:** The 8 columns are zone types assigned by
   CellClass::RecalcZoneType, not a direct mapping from the 12-value LandType enum.
6. **Passability values 2/3 semantics:** Both 2 and 3 mean "not passable" in zone logic
   (only value 1 passes the `== 1` check). Value 3 only appears in column 7 (OoB/rock).
   The practical distinction is that 2 = blocked terrain, 3 = permanent sentinel.
   The Rust PASS_BLOCKED/PASS_IMPASSABLE naming is acceptable.
