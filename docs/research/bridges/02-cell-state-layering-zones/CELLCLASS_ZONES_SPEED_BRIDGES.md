# CellClass Gaps: Zone Connectivity, Terrain Speed Modifiers, Bridge Cells

**Date:** 2026-04-06
**Confidence:** HIGH (all decompiled from gamemd.exe via Ghidra MCP, cross-verified with existing docs)
**Active in YR:** Yes -- all systems documented here are live in standard YR skirmish
**Extends:** CELLCLASS_STRUCT_GHIDRA_REPORT.md, TERRAIN_COST_FACTSHEET.md, ZONE_PASSABILITY_VERIFIED.md, BRIDGE_SYSTEM.md

---

## 1. Zone Connectivity System

### 1.1 Architecture Overview

The zone system provides O(1) reachability checks ("can unit A reach cell B?") for pathfinding.
It uses a two-level indirection scheme:

```
Cell coord -> linear index -> zone cluster ID (per-cell) -> zone ID (per-MovementZone)
```

Two cells are reachable from each other (for a given MovementZone) iff their zone IDs match.

### 1.2 MapClass Zone Data Structures

| Offset  | Type          | Size             | Description |
|---------|---------------|------------------|-------------|
| +0x14   | ptr           | 4                | Zone connection graph (hash table, 256 buckets x 0x18 bytes each) |
| +0x18   | ptr[13]       | 52               | Zone ID arrays, one per MovementZone (ushort per cluster) |
| +0x4C   | int           | 4                | Total zone cluster count |
| +0x54   | ptr           | 4                | Bridge records array (16 bytes per bridge record) |
| +0x58   | int           | 4                | Bridge record capacity |
| +0x60   | int           | 4                | Bridge record count |
| +0x68   | ptr           | 4                | Per-cell zone data array (4 bytes per cell) |
| +0x6C   | int           | 4                | Total cell count |
| +0x70   | ptr           | 4                | Per-cell zone index array (10 bytes per cell = 5 shorts) |
| +0x90   | ptr[3]        | 12               | Hierarchical zone graphs for 3 speed categories (0x24 bytes per entry) |
| +0xF4   | int           | 4                | Map origin X |
| +0xF8   | int           | 4                | Map width |

**Confidence:** HIGH -- all offsets verified from multiple decompiled functions.

### 1.3 Per-Cell Zone Data (MapClass+0x68)

Each cell has a 4-byte entry:

| Byte | Type   | Purpose |
|------|--------|---------|
| 0    | byte   | ZoneType (0-7, same as CellClass+0x4C) |
| 1    | byte   | Height level (for flood-fill height checks) |
| 2-3  | ushort | Zone cluster ID (assigned during flood-fill) |

**Confidence:** HIGH -- verified from ZoneFloodFillScanLine (0x56CB90) which reads byte[0] as
zone type comparator, byte[1] as height for abs-diff checks, and writes cluster ID to bytes[2-3].

### 1.4 Per-Cell Zone Index (MapClass+0x70)

Each cell has 10 bytes (5 shorts), used by the hierarchical pathfinder:

| Short | Purpose |
|-------|---------|
| 0     | Zone node ID for speed category 0 (finest) |
| 1     | Zone node ID for speed category 1 (medium) |
| 2     | Zone node ID for speed category 2 (coarsest) |
| 3-4   | Possibly unused or bridge variants (~60% confidence) |

Used by `AddBridgeZoneEdges` (0x5851B0) which reads `*(short*)(zoneIndex + cellLinear * 10 + level * 2)`.

### 1.5 Zone Cluster Assignment Algorithm (UpdateBridgeZonesHelper, 0x56C510)

This is the main zone computation function. Called during map load and after bridge state changes.

**Algorithm (reconstructed from decompilation):**

```
Phase 1: Cleanup
  - Clear all 256 hash buckets in the zone connection graph (MapClass+0x14)
  - Free all 13 zone ID arrays (MapClass+0x18..0x48)
  - Clear all per-cell cluster IDs to 0 (in MapClass+0x68 array)

Phase 2: Flood-fill cluster assignment
  cluster_id = 1   // 0 is reserved as "unassigned"
  largest_cluster_id = 0xFFFF  // tracks which cluster is largest
  largest_cluster_size = -1

  For each cell in cell_data array:
    if cell.zoneType == 7 (OoB) OR cell.cluster_id != 0:
      skip (already assigned or out of bounds)

    size = ZoneFloodFillScanLine(cell, cluster_id)
    // Record zone type for this cluster (stored in separate array)
    // Track which cluster is the largest
    cluster_id++

  Store total cluster count at MapClass+0x4C

Phase 3: Bridge zone edges
  For each bridge record (MapClass+0x54, 16 bytes each):
    if bridge is active (byte+8 != 0):
      Get cluster IDs at each endpoint
      If endpoints have different cluster IDs:
        Add edge to zone connection graph (hash by lower 4 bits of each cluster)
        Edge stored as packed uint32: (smaller_id << 16) | larger_id

Phase 4: Build adjacency lists
  Count edges per cluster from the zone connection graph
  Allocate per-cluster neighbor arrays
  Populate neighbor arrays from graph edges

Phase 5: Per-MovementZone zone ID assignment (13 iterations)
  For each of the 13 MovementZone rows (0-12):
    Allocate a ushort array (one entry per cluster)
    For each cluster:
      initial_value = (g_PassabilityMatrix[movementZone * 8 + cluster_zoneType] != 1) ? 1 : 0
      // 0 = passable (needs zone ID), 1 = blocked (pre-assigned as blocked)

    zone_id = 2  // 2+ are real zone IDs; 0 = unassigned sentinel; 1 = blocked sentinel
    For each unassigned passable cluster (value == 0):
      BFS/flood-fill through neighbor list:
        Only expand to neighbors with SAME passability value in the matrix
        Assign zone_id to all reachable clusters
      zone_id++

    Set cluster 0 to 0xFFFF (sentinel)
    Store array at MapClass+0x18 + movementZone * 4

Phase 6: Cleanup
  Free temporary arrays
  Return largest_cluster_id
```

**Key insight:** The zone connection graph uses a hash table with 256 buckets (0x18 bytes each).
Each bucket is a dynamic vector of edges. The hash key is `(cluster_a & 0xF) << 4 | (cluster_b & 0xF)`.
Each edge is stored as two uint32s: the packed cluster pair and a copy of itself.

**Confidence:** HIGH -- fully decompiled, algorithm matches the described behavior in
TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md but with corrected details.

### 1.6 Zone Flood-Fill (ZoneFloodFillScanLine, 0x56CB90)

Recursive scanline flood-fill operating on the per-cell zone data array (MapClass+0x68).

**Per-cell data layout used by flood-fill:**
- Byte 0: ZoneType (must match seed cell's ZoneType to expand)
- Byte 1: Height level (abs height diff checked against threshold)
- Bytes 2-3: Cluster ID (written with current cluster_id; checked for 0 = unassigned)

**Algorithm:**

```
1. Start at seed cell. Record its ZoneType.

2. Scan LEFT:
   While neighbor has same ZoneType AND abs(height_diff) <= 1:
     Assign cluster_id to cell
     Move left

3. Check left boundary:
   If existing cluster at boundary has a different cluster_id:
     AND height diff <= 1 (or ZoneType == 6 "Impassable"):
       Add edge between old cluster and new cluster in zone graph

4. Scan RIGHT from seed:
   While neighbor has same ZoneType AND abs(height_diff) <= 3:   // NOTE: more lenient!
     Assign cluster_id to cell
     Move right

5. Check right boundary (same edge-recording logic as step 3)

6. Compute span width. Record cells advanced from seed in *param_4.

7. Recurse into rows above and below the span:
   For each cell in the adjacent row within the span:
     If cell is unassigned (cluster_id == 0):
       If same ZoneType AND abs(height_diff from connecting cell) <= 1:
         Recurse with ZoneFloodFillScanLine
     Else if cell has different cluster_id:
       If height diff <= 1 (or ZoneType == 6):
         Record edge between clusters

   Return total cells assigned (accumulated across recursions)
```

**Height threshold asymmetry (CONFIRMED):**
- Left scan: abs(height_diff) <= 1
- Right scan: abs(height_diff) <= 3 (more lenient)
- Recursive scans (row above/below): abs(height_diff) <= 1

The rightward leniency likely handles bridge approaches or steep ramps where the right-hand
neighbor may have a larger height jump but should still be in the same zone.

**ZoneType == 6 (Impassable) special case:** When the seed cell's ZoneType is 6, height
checks at boundaries are bypassed (edges always added). This ensures all impassable zones
are connected even across height discontinuities.

**Confidence:** HIGH -- fully decompiled, matches TODO_ZONE_FIDELITY_FIXES.md Finding 7.

### 1.7 Zone Lookup (GetZoneID, 0x56D230)

```c
uint GetZoneID(MapClass* this, CellCoord* coord, int movementZone, char checkBridge) {
    if (checkBridge != 0) {
        // Get cell at coord
        cell = GetCellAt(coord);

        if (cell->Flags & 0x100) {  // Bridge structural cell
            // Look up bridge record for this cell
            bridgeIdx = FindBridgeRecord(coord, 1, 0);
            if (bridgeIdx == -1) return 0xFFFFFFFF;  // No bridge record

            bridgeRecord = MapClass+0x54 + bridgeIdx * 16;

            if (bridgeRecord.active == 0) {
                // Bridge is destroyed -- walk off the bridge to find ground zone
                // Walk in the direction perpendicular to bridge orientation
                // until we find a non-bridge cell
                while (cell->Flags & 0x100) {
                    // Binary: (-(uint)(sVar1 != sVar2) & 0xFFFFFFFE) + 4
                    //   sVar1 != sVar2 (different X = horizontal bridge) -> direction 2
                    //   sVar1 == sVar2 (same X    = vertical bridge)    -> direction 4
                    direction = (endpoint1.X != endpoint2.X) ? 2 : 4;
                    cell = GetNeighborCell(direction);
                }
                // If the ground cell under the bridge is a valid bridge tile
                // and not Rock (LandType != 3), use the other bridge endpoint
                if ((IsBridge(cell) || IsWoodBridge(cell)) && cell.LandType != 3) {
                    coord = &bridgeRecord.endpoint2;  // Use far endpoint
                }
            }
        }
    }

    // Standard two-level lookup
    linearIdx = (mapWidth + 1 + mapOriginX) * coord->Y + coord->X;
    linearIdx = clamp(linearIdx, 0, totalCells - 1);

    clusterID = *(ushort*)(cellData + linearIdx * 4 + 2);  // bytes 2-3
    return *(ushort*)(zoneIdArrays[movementZone] + clusterID * 2);
}
```

**Bridge zone handling:** When a unit is on a bridge cell and `checkBridge` is true, the function
first checks if the bridge is active. If the bridge is destroyed, it walks perpendicular to the
bridge until it finds a non-bridge cell, then looks up the zone from there. This prevents destroyed
bridge cells from returning stale zone IDs.

**Confidence:** HIGH -- fully decompiled.

### 1.8 Zone Reachability (Can_Reach_Zone, 0x56D100)

Simple zone ID equality check:
```c
bool Can_Reach_Zone(CellCoord* src, CellCoord* dst, int movementZone, ...) {
    if (movementZone == -1) return true;
    zone_src = GetZoneID(src, movementZone, ...);
    zone_dst = GetZoneID(dst, movementZone, ...);
    return zone_src == zone_dst;
}
```

No graph traversal. Just equality. **Confirmed:** two cells in different connected components
of the same passability type will have different zone IDs and fail this check.

### 1.9 Zone Recalculation Triggers

Zones are recomputed by calling `UpdateBridgeZonesHelper` (full recompute) when:

1. **Map initialization** -- called during scenario loading
2. **Bridge destruction** -- `ProcessBridgeDamageStateMachine_Low/High` calls
   `InvalidateBridgeZones` then `UpdateBridgeZonesHelper` if the invalidation changed anything
3. **Bridge repair** -- same pattern via `ValidateBridgeZones` + `UpdateBridgeZonesHelper`
4. **Building placed/sold** -- triggers `RecalcAttributes` on affected cells which updates
   ZoneType, but does NOT trigger full zone recomputation. The zone cluster IDs remain stale
   until a bridge event triggers a full recompute. (This means placing a building on a chokepoint
   does NOT immediately update zone connectivity -- a known limitation/behavior of the original.)

**InvalidateBridgeZones (0x56DAE0):**
- Finds all bridge records matching the given cell coordinate (within distance 3)
- For each active bridge record: removes zone edges, marks bridge as inactive
- Returns true if any bridge was actually deactivated

**ValidateBridgeZones (0x56DB70):**
- Finds all bridge records matching the given cell coordinate
- For each inactive bridge record: marks as active, adds zone edges
- Calls `Can_Reach_Zone` to check if the bridge actually connects two different zones
- Returns true if the bridge created a NEW connection

**Confidence:** HIGH -- decompiled and cross-verified.

### 1.10 Hierarchical Zone Pathfinding (Zone_precheck, 0x42C290)

The hierarchical pathfinder uses 3 zone levels (coarsest=2 to finest=0), stored at
MapClass+0x90 as three separate zone graph structures.

`AddBridgeZoneEdges` (0x5851B0) adds edges to all 3 hierarchical zone graphs simultaneously:
- For each of the 3 zone levels (loop variable += 2 for each level, iterating bytes in the 10-byte zone index)
- Gets zone node IDs at each bridge endpoint
- Adds bidirectional edges between the endpoint nodes in that level's graph

Each zone graph entry is 0x24 (36) bytes, containing a dynamic vector of neighbors.

**Confidence:** 85% -- complex function, key logic verified but some structural details unclear.

---

## 2. Terrain Speed Modifiers

This section consolidates and extends TERRAIN_COST_FACTSHEET.md.

### 2.1 Speed Table (g_SpeedType_LandType_Table, 0x89EA40)

**Size:** 432 bytes = 12 LandTypes x 9 entries x 4 bytes/float
**Populated by:** `RulesClass::ReadSpeedTypeLandTypeTable` at 0x674000 from INI sections

Index formula: `speed = table[SpeedType + LandType * 9]`

Each row has 9 entries: 7 SpeedTypes + 1 padding + 1 Buildable flag.

### 2.2 Full Speed Table (from rulesmd.ini, stored as 0.0-1.0 floats)

| LandType    | Foot | Track | Wheel | Hover | Winged | Float | Amph | FltBch | Buildable |
|-------------|------|-------|-------|-------|--------|-------|------|--------|-----------|
| Clear (0)   | 1.0  | 1.0   | 1.0   | 0.5   | 1.0    | 0.0   | 0.8  | 0.0    | 1.0       |
| Road (1)    | 1.0  | 1.0   | 1.0   | 0.75  | 1.0    | 0.0   | 1.0  | 0.0    | 1.0       |
| Water (2)   | 0.0  | 0.0   | 0.0   | 1.0   | 1.0    | 1.0   | 1.0  | 1.0    | 0.0       |
| Rock (3)    | 0.0  | 0.0   | 0.0   | 0.0   | 1.0    | 0.0   | 0.0  | 0.0    | 0.0       |
| Wall (4)    | 0.0  | 0.0   | 0.0   | 0.0   | 1.0    | 0.0   | 0.0  | 0.0    | 0.0       |
| Tiberium (5)| 0.9  | 0.7   | 0.5   | 0.5   | 1.0    | 0.0   | 0.5  | 0.0    | 0.0       |
| Beach (6)   | 0.0  | 0.0   | 0.0   | 0.75  | 1.0    | 0.0   | 0.6  | 1.0    | 0.0       |
| Rough (7)   | 1.0  | 1.0   | 1.0   | 0.5   | 1.0    | 0.0   | 0.8  | 0.0    | 1.0       |
| Ice (8)     | 0.5  | 0.8   | 0.5   | 1.0   | 1.0    | 0.0   | 0.5  | 0.0    | 0.0       |
| Railroad (9)| 0.9  | 1.0   | 0.5   | 1.0   | 1.0    | 0.0   | 0.5  | 0.0    | 0.0       |
| Tunnel (10) | 1.0  | 1.0   | 1.0   | 1.0   | 1.0    | 0.0   | 1.0  | 0.0    | 0.0       |
| Weeds (11)  | 0.5  | 0.7   | 0.5   | 1.0   | 1.0    | 0.0   | 0.5  | 0.0    | 0.0       |

**Note:** Winged (column 4) is hardcoded to 1.0 for ALL LandTypes at 0x674000.
Never read from INI.

### 2.3 Speed Computation in DriveLocomotionClass::Process_Movement (0x4B2630)

```
1. BASE_SPEED = SpeedTable[unit.SpeedType + cell.LandType * 9]
2. Clamp: if BASE_SPEED > 1.0 -> BASE_SPEED = 1.0
3. Fallback: if BASE_SPEED == 0.0 -> BASE_SPEED = 0.5
   (emergency minimum to prevent units stuck on impassable terrain)

4. SLOPE MODIFIER (only for ground units):
   Going UPHILL:
     Track:  speed *= RulesClass+0x768  (SlopeClimb for tracked)
     Other:  speed *= RulesClass+0x778  (SlopeClimb for others)
   Going DOWNHILL:
     Track:  speed *= RulesClass+0x770  (SlopeDescend for tracked)
     Other:  speed *= RulesClass+0x780  (SlopeDescend for others)

5. HEALTH PENALTY:
   if healthRatio <= RulesClass+0x1700:
     speed *= DamageSpeedMultiplier

6. FORMATION:
   if unit is in formation with speed < 0x40:
     store raw speed
```

### 2.4 Special Terrain Cases

**Tiberium/Ore cells:** Use LandType 5 (Tiberium) which has non-zero speeds for most ground
units but is 0 for Float and FloatBeach. Speed ranges from 0.5 (Wheel, Hover, Amph) to 0.9 (Foot).

**Building foundations:** When a building is placed, `RecalcZoneType` sets the cell's ZoneType
to 5 (Building) or 6 (Impassable). The LandType at CellClass+0xEC is NOT changed by building
placement. The speed table check in `CheckCellPassability` uses the cell's LandType, but since
the occupation bits already block entry, the speed value is academic for occupied cells.

**Walls:** OverlayType with `IsWall` flag. `RecalcZoneType` assigns ZoneType 2 (Wall).
In `CheckCellPassability`, if the unit's MovementZone can crush walls, the LandType is
temporarily treated as 0 (Clear) for the speed table lookup.

**Bridge cells:** When on a bridge, the unit uses the bridge deck's effective terrain type
(usually Clear). The `on_bridge` flag at FootClass+0x8C adds +4 to the height level used for
slope calculations, effectively skipping the underlying water terrain.

### 2.5 Pathfinding Cost vs Movement Speed

**Critical:** The A* pathfinder does NOT use the TerrainSpeedTable for step costs.
All passable cells have equal A* cost (modulo bridge and diagonal penalties).
The pathfinder finds the **shortest-distance** path, not the fastest path.

The TerrainSpeedTable only affects **runtime movement speed** -- how fast a unit traverses
a cell it has already committed to moving through.

Bridge cost penalties in pathfinding:
- Flag 0x40000 (AlteredPassability): cost *= 4.0 (at constant 0x7E37BC)
- Bridge diagonal: cost *= 2.0 (at constant 0x7E37B4)
- Non-bridge diagonal with bridge neighbor: cost *= 10.0 (at constant 0x7E37B8)

### 2.6 RecalcZoneType Speed Threshold

In `RecalcZoneType` (0x483C80), the impassable threshold is:
```
if speed_table[LandType * 9 + 0] <= 0.01 -> ZoneType = 6 (Impassable)
```
The 0.01 threshold (from double at 0x7E3808) means only terrain with <= 1% speed is
classified as impassable. Rock (0.0) and Wall (0.0) are impassable. Tiberium (0.9), Rough
(1.0), Railroad (0.9) all pass and default to ZoneType 0 (Ground).

**Note:** The overlay check uses exact `== 0.0` comparison (step 2c in RecalcZoneType),
while the terrain check uses `<= 0.01` (step 4).

---

## 3. Bridge Cell Special Cases

### 3.1 Bridge Overlay IDs

Bridges use two separate overlay index ranges depending on bridge type (low vs high):

**Low bridges (wooden/concrete):**
- NS direction: overlay indices 0x4A-0x52 (74-82) -- body cells
- EW direction: overlay indices 0x53-0x5F (83-95) -- body cells  
- Shared endpoint: overlay index 0x64 (100)
- Full range for detection: 0x4A-0x65 (74-101)

**High bridges (reinforced concrete):**
- NS direction: overlay indices 0xCD-0xD5 (205-213) -- body cells
- EW direction: overlay indices 0xD6-0xDE (214-222) -- body cells
- Shared endpoint: overlay index 0xE8 (232)
- NS additional: 0xE7 (231)
- Full range for detection: 0xCD-0xE8 (205-232)

**Bridge tile types (IsoTileTypeIndex):**
- `BridgeSet` (global at DAT_00AA0E28): Start of concrete bridge tiles
- `WoodBridgeSet` (global at DAT_00ABAD1C): Start of wooden bridge tiles

`CellClass::IsBridge` checks `IsoTileTypeIndex in [BridgeSet, BridgeSet+16)`.
`CellClass::IsWoodBridge` checks `IsoTileTypeIndex in [WoodBridgeSet, WoodBridgeSet+16)`.

**Confidence:** HIGH -- overlay ranges extracted from DestroyBridge_Low (0x57BAA0) and
DestroyBridge_High (0x57CCF0).

### 3.2 Bridge Records (MapClass+0x54)

Each bridge record is 16 bytes (0x10):

| Offset | Size | Type    | Purpose |
|--------|------|---------|---------|
| +0x00  | 4    | CellCoord | Endpoint 1 (packed X:Y) |
| +0x04  | 4    | CellCoord | Endpoint 2 (packed X:Y) |
| +0x08  | 1    | byte    | Active flag (1 = bridge intact, 0 = destroyed) |
| +0x09  | 3    | bytes   | Flags/padding |
| +0x0C  | 4    | int     | Bridge type (0 = high bridge, 1 = low bridge/tube) |

**FindBridgeRecord (0x56DA10):**
- Searches bridge records starting from `param_4` (start index)
- Matches if the given cell coordinate falls within `param_3` cells of the bridge line
- For NS bridges: checks Y range, computes X distance
- For EW bridges: checks X range, computes Y distance
- Returns record index or -1 if not found

**ComputeBridgeZones (0x56D6E0):**
- Called during map initialization
- Iterates all cells looking for bridge tiles
- For each bridge tile: walks along the bridge direction to find the far endpoint
- Checks height matching using tables at 0x82A734 and 0x82A7B4
- Creates a bridge record with both endpoints, active=1, and bridge type
- Also handles low bridges (tube cells): checks perpendicular neighbors, computes tube endpoints

**Confidence:** HIGH -- decompiled both functions.

### 3.3 Bridge Damage State Machine (CellClass+0x11E)

> **NS/EW labels in this section are physically correct.** Verified 2026-05-13
> by extracting `bridge.tem` frames 0 and 9 (via `extract-bridge-frames` bin).
> Frame 0 (state byte range 0..8) renders as a NW→SE screen-space diagonal =
> world east-west axis = **physically EW**. Frame 9 (state byte range 9..0x11)
> renders as NE→SW = world north-south axis = **physically NS**. Trust the
> byte-range / axis labels in this doc.
>
> Note: gamemd's *function names* in the `UpdateRamp_*` and
> `ApplyBridgeDestruction_*` families are inverted relative to the axis they
> operate on (functions named `_NS_*` actually transition the physically-EW
> state range 0..8, and `_EW_*` the physically-NS range 9..0x11) — the binary
> mis-names its own functions. Do not trust binary function names for axis
> attribution; trust the byte-range labels.

The `OverlayData` field at CellClass+0x11E serves double duty for bridge cells:
it encodes both direction and damage state.

**State values for low bridges (ProcessBridgeDamageStateMachine_Low at 0x571490):**

| State | Direction | Meaning | Transition |
|-------|-----------|---------|------------|
| 0-5   | NS        | Healthy/initial | -> 6 (first damage hit) |
| 6     | NS        | Damaged phase 1 | -> collapse sequence |
| 7     | NS        | Collapse A (one ramp) | -> clear bridge |
| 8     | NS        | Collapse B (other ramp) | -> clear bridge |
| 9-14  | EW        | Healthy/initial | -> 15 (first damage hit) |
| 15    | EW        | Damaged phase 1 | -> collapse sequence |
| 16    | EW        | Collapse B (one ramp) | -> clear bridge |
| 17    | EW        | Collapse A (other ramp) | -> clear bridge |

**Damage progression:**
1. **First hit (states 0-5 or 9-14):** Set state to 6 (NS) or 15 (EW). Update ramp tiles
   to show damage (UpdateRamp_*_Damage_Low functions). Bridge remains passable.

2. **Second hit (state 6 or 15):** Begin collapse sequence. Update ramps to collapsed state.
   Call `SetBridgeDirection_NWSE(0 or 6, 0)` to clear bridge flags. Reset state to 0.
   Set overlay to -1 (remove overlay). Call `UpdateAdjacentBridges` and
   `InvalidateBridgeZones` + `UpdateBridgeZonesHelper` to recompute zones.

3. **Bridgehead destruction (state corresponding to +3):** The bridgehead (entry point) is
   handled separately. When destroyed: calls `BlowUpBridge` on 3 adjacent cells, sets the
   collapsed bridge overlay, updates ramps, invalidates zones, and forces a full zone recompute.

**BridgeStrength:** Read from `[CombatDamage] BridgeStrength` in rules.ini.
Stored at `RulesClass + 0x1740`. This is the HP value for bridge segments.

**Confidence:** HIGH -- full state machine decompiled.

### 3.4 Bridge Destruction Sequence

When a bridge is destroyed (via `DestroyBridge_Low` or `DestroyBridge_High`):

1. **Direction detection:** Check overlay index to determine NS vs EW bridge
2. **Walk to bridge start:** Move in the reverse bridge direction to find the starting cell
3. **Call DestroyBridgeWalker:** Walk along the bridge, destroying each segment
4. **Per-cell destruction (BlowUpBridge, 0x47DD70):**
   - Damage all objects on ground layer (FirstObject list) with C4Warhead
   - Kill all objects on bridge layer (AltObject list) via vtable call +0xEC
   - Add cell coordinates to a pending destruction list
   - Randomly spawn bridge explosion animations (BridgeExplosions and BridgeDestruction
     animation lists from rules.ini)
5. **Post-destruction:**
   - Set collapsed overlay using `SetOverlayAndPropagate`
   - Update ramp tiles to collapsed state
   - Update adjacent bridges
   - Call `InvalidateBridgeZones` to remove bridge zone edges
   - If invalidation changed anything, call `UpdateBridgeZonesHelper` for full zone recompute
   - Invalidate pathfinder caches

### 3.5 Bridge Repair Sequence

`RepairBridge_Low` (0x57F200) / `RepairBridge_High` (0x57F440):

1. **Direction detection:** Same overlay range checks as destruction
2. **Walk to bridge start:** Same walking logic
3. **Call RepairBridgeWalker:** Walk along the bridge, repairing each segment
4. **Post-repair:**
   - Set healthy bridge overlay
   - Update ramp tiles to intact state
   - Call `ValidateBridgeZones` to add bridge zone edges
   - If validation connected new zones, call `UpdateBridgeZonesHelper`

**EVA notification:** "EVA_BridgeRepaired" (string at 0x825538) is played.
**Sound:** `RepairBridgeSound` (string at 0x83A7FC) is played.

### 3.6 Dual-Layer Cell System for Bridges

Bridge cells maintain two parallel tracking systems:

**Object lists:**
- `CellClass+0xE4` (FirstObject): Ground-level object linked list
- `CellClass+0xE8` (AltObject): Bridge-level object linked list

**Occupation bits:**
- `CellClass+0x124` (OccupationFlags): Ground-level occupation
- `CellClass+0x128` (AltOccupationFlags): Bridge-level occupation

**Height determination:** When a unit enters a bridge cell, the pathfinder and locomotor
use height comparison to decide which layer:
- `abs(unit_height - cell_ground_height) <= 1`: Ground layer (passing under bridge)
- `abs(unit_height - (cell_ground_height + 4)) <= 1`: Bridge layer (on bridge)

The A* pathfinder maintains dual closed lists for bridge cells:
- `PathfinderClass+0x18/+0x24`: Ground-level visited/costs
- `PathfinderClass+0x1C/+0x20`: Bridge-level visited/costs

### 3.7 Bridge Cell Flags (CellClass+0x140, bits 7-21)

| Bit | Mask     | Name               | Purpose |
|-----|----------|--------------------|---------|
| 7   | 0x0080   | HasBridgeOverlay   | Body cell -- GetEffectiveHeight adds +4 |
| 8   | 0x0100   | BridgeStructural   | Primary pathfinding/movement flag |
| 9   | 0x0200   | Bridgehead         | Entry/exit point for bridge |
| 10  | 0x0400   | BridgeRail         | Ramp cell / guard post |
| 11  | 0x0800   | BridgeOrientation  | 0=N-S, 1=E-W |
| 12  | 0x1000   | BridgeDirectionBit | Direction sub-type |
| 13  | 0x2000   | BridgePavement     | Sub-tile variant selector |
| 18  | 0x40000  | AlteredPassability | A* cost x4 multiplier |
| 20  | 0x100000 | BridgeZone_NS      | Zone marker for NS bridges |
| 21  | 0x200000 | BridgeZone_EW      | Zone marker for EW bridges |

### 3.8 Bridge-Specific CellClass Fields

| Offset | Field          | Bridge Usage |
|--------|----------------|--------------|
| 0x2C   | BridgeAnchorPtr | Pointer to bridge anchor cell (set by SetBridgeDirection_NESW) |
| 0x11A  | Height/SubType | For bridge body cells: bridge sub-type / orientation byte |
| 0x11B  | Level          | Ground height level (bridge deck = Level + 4) |
| 0x11E  | OverlayData    | Bridge damage state machine (0-17 for low bridges) |

---

## 4. New Struct Fields Discovered

### CellClass Fields (confirmed or clarified)

| Offset | Type   | Name            | Evidence | Confidence |
|--------|--------|-----------------|----------|------------|
| 0x4C   | int    | ZoneType        | RecalcZoneType writes; used as passability column | HIGH |
| 0x11E  | byte   | OverlayData / BridgeDamageState | Dual-purpose: ore amount for tiberium, damage state for bridges | HIGH |
| 0x2C   | ptr    | BridgeAnchorPtr | SetBridgeDirection_NESW; points to anchor cell | HIGH |

### MapClass Fields (confirmed)

| Offset | Type     | Name                | Evidence | Confidence |
|--------|----------|---------------------|----------|------------|
| 0x14   | ptr      | ZoneGraph           | UpdateBridgeZonesHelper clears buckets | HIGH |
| 0x18   | ptr[13]  | ZoneIdArrays        | Per-MovementZone, allocated in UpdateBridgeZonesHelper | HIGH |
| 0x4C   | int      | ZoneClusterCount    | Written at end of flood-fill phase | HIGH |
| 0x54   | ptr      | BridgeRecords       | 16-byte records, searched by FindBridgeRecord | HIGH |
| 0x58   | int      | BridgeRecordCapacity| Dynamic vector capacity | HIGH |
| 0x60   | int      | BridgeRecordCount   | Incremented in ComputeBridgeZones | HIGH |
| 0x68   | ptr      | CellZoneData        | 4 bytes/cell: zoneType, height, clusterID | HIGH |
| 0x6C   | int      | TotalCellCount      | Used as array bound | HIGH |
| 0x70   | ptr      | CellZoneIndex       | 10 bytes/cell (5 shorts), hierarchical zone node IDs | HIGH |
| 0x90   | ptr[3]   | HierarchicalZoneGraphs | 0x24 bytes/entry, used by Zone_precheck | MED |
| 0xF4   | int      | MapOriginX          | Used in linear index computation | HIGH |
| 0xF8   | int      | MapWidth            | Used in linear index computation | HIGH |

### RulesClass Fields (bridge-related)

| Offset | Type   | INI Key           | Purpose | Confidence |
|--------|--------|-------------------|---------|------------|
| 0x1740 | int    | BridgeStrength    | HP of bridge segments | HIGH |
| 0x17CC | int    | CollapseChance    | % chance of bridge collapse | HIGH |
| 0x0FA8 | ptr    | C4Warhead         | Warhead used when bridges blow up objects | HIGH |

---

## 5. Function Address Summary

### Zone System

| Address    | Name | Purpose |
|------------|------|---------|
| 0x0056C510 | MapClass__UpdateBridgeZonesHelper | Full zone recomputation |
| 0x0056CB90 | MapClass__ZoneFloodFillScanLine | Recursive scanline flood-fill |
| 0x0056D100 | MapClass__Can_Reach_Zone | O(1) zone equality check |
| 0x0056D230 | MapClass__GetZoneID | Two-level zone ID lookup |
| 0x0056D3F0 | ZoneMap__CellToZoneIndex | Cell coord to linear index |
| 0x0056D430 | MapClass__CellCoordToLinearIndex | Same purpose, slightly different |
| 0x0056D6E0 | MapClass__ComputeBridgeZones | Scan map for bridges, create records |
| 0x0056DA10 | MapClass__FindBridgeRecord | Search bridge records by coord |
| 0x0056DAE0 | MapClass__InvalidateBridgeZones | Remove bridge zone edges |
| 0x0056DB70 | MapClass__ValidateBridgeZones | Add bridge zone edges |
| 0x005840C0 | ZoneMap__FloodFillReachableZones | Flood-fill for hierarchical levels |
| 0x005851B0 | MapClass__AddBridgeZoneEdges | Add edges to 3 hierarchical graphs |
| 0x00584E50 | MapClass__RemoveBridgeZoneEdges | Remove edges from hierarchical graphs |
| 0x005889F0 | ZoneMap__FindBestCompatibleMovementZone | Team pathfinding zone merger |
| 0x0042C290 | Zone_precheck | Hierarchical zone-level pathfinding |
| 0x0082A594 | g_PassabilityMatrix | 13x8 passability matrix (data) |

### Speed/Terrain

| Address    | Name | Purpose |
|------------|------|---------|
| 0x0089EA40 | g_SpeedType_LandType_Table | 12x9 float speed table |
| 0x00674000 | RulesClass__ReadSpeedTypeLandTypeTable | Parse speed table from INI |
| 0x004B2630 | DriveLocomotionClass__Process_Movement | Speed computation with slope/health (function entry; 0x4B3C80 was an interior address) |
| 0x00483C80 | CellClass__RecalcZoneType | Assign ZoneType (0-7) to cell |
| 0x0047D2B0 | CellClass__RecalcAttributes | Full cell attribute recomputation |
| 0x004834A0 | CellClass__CheckCellPassability | Cell-level passability check |

### Bridge System

| Address    | Name | Purpose |
|------------|------|---------|
| 0x0047DD70 | CellClass__BlowUpBridge | Per-cell bridge destruction |
| 0x0047E040 | CellClass__SetBridgeDirection_NESW | Set bridge cell flags |
| 0x0047E470 | CellClass__SetBridgeDirection_NWSE | Set bridge cell flags (alt dir) |
| 0x0057BAA0 | DestroyBridge_Low | Low bridge destruction entry point |
| 0x0057CCF0 | DestroyBridge_High | High bridge destruction entry point |
| 0x00571490 | ProcessBridgeDamageStateMachine_Low | Bridge damage state machine |
| 0x00576BA0 | ProcessBridgeDamageStateMachine_High | Bridge damage state machine (high) |
| 0x0057F200 | MapClass__RepairBridge_Low | Low bridge repair entry point |
| 0x0057F440 | MapClass__RepairBridge_High | High bridge repair entry point |
| 0x00486750 | CellClass__IsBridge | Check if tile is concrete bridge |
| 0x00486770 | CellClass__IsWoodBridge | Check if tile is wooden bridge |
| 0x00484AB0 | CellClass__IsLowBridgeCell | TubeIndex >= 0 AND LandType == 10 |
| 0x00487D50 | CellClass__GetEffectiveHeight | Level + (Flags & 0x80 ? 4 : 0) |
| 0x0042ACF0 | PathfinderClass__UpdateBridgePassability | Toggle AlteredPassability flag |

---

## 6. TS-Only Features Identified

**None.** All systems documented in this report are live in standard YR skirmish:
- Zone system runs during every map load and bridge event
- Speed table is read from INI and used every movement tick
- Bridge damage/repair is active during gameplay
- No SpecialFlags gates detected in any call path

---

## 7. Cross-Reference to Existing Reports

| Report | Relationship |
|--------|-------------|
| CELLCLASS_STRUCT_GHIDRA_REPORT.md | Base struct layout -- this doc extends zone/bridge field details |
| TERRAIN_COST_FACTSHEET.md | Speed table and pathfinding costs -- this doc adds slope/health modifiers |
| ZONE_PASSABILITY_VERIFIED.md | Passability matrix and enum corrections -- this doc adds zone algorithm |
| TODO_ZONE_FIDELITY_FIXES.md | Known Rust implementation gaps -- this doc provides reference for fixes |
| BRIDGE_SYSTEM.md | Bridge flags, height arithmetic, dual occupancy -- this doc adds damage/repair |
| BRIDGE_RENDERING_GHIDRA_REPORT.md | Bridge rendering pipeline -- complementary (rendering vs gameplay) |
| NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md | Ship zone confinement -- this doc provides zone algorithm details |
| TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md | Early zone research -- this doc corrects/extends many details |
