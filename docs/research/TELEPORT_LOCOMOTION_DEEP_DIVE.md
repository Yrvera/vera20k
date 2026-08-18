# TeleportLocomotionClass Deep Dive -- Ghidra Research Report

## Overview

Deep-dive addendum to `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`. This document covers
internals that were summarized or marked low-confidence in the original report: Process()
destination validation, infantry sub-cell handling, bridge checks, mission docking logic,
Update_Position mechanics, PostWarpValidation water death, kill credit via FUN_006B0AE0,
TimerCheck idle behavior (FUN_0070F770/FUN_00709480), and Mark_All_Occupation_Bits.

All addresses from gamemd.exe (YR 1.001). All decompilations are fresh from Ghidra MCP.

---

## 1. Constructor (0x00718000) -- Complete Field Layout Verified

### Decompiled Constructor

```c
TeleportLocomotionClass* Constructor(TeleportLocomotionClass* this) {
    LocomotionClass::Constructor(this);  // base class init

    // HeadToCoord (base+0x1C..0x24) = NullCoord
    this[7]  = g_NullCoord_X;   // +0x1C
    this[8]  = g_NullCoord_Y;   // +0x20
    this[9]  = g_NullCoord_Z;   // +0x24

    // DestCoord (base+0x28..0x30) = NullCoord
    this[10] = g_NullCoord_X;   // +0x28
    this[11] = g_NullCoord_Y;   // +0x2C
    this[12] = g_NullCoord_Z;   // +0x30

    *(byte*)(this + 0x34) = 0;  // IsMoving
    *(byte*)(this + 0x35) = 0;  // field_35 (checked by Is_Ok_To_End)
    *(byte*)(this + 0x36) = 0;  // field_36

    this[0xE] = 0;              // +0x38 = WarpPhase

    this[0xF] = g_CurrentFrameCounter;  // +0x3C = Timer.StartFrame
    this[0x11] = 0;             // +0x44 = Timer.Duration
    this[0x12] = 0;             // +0x48 = PiggybackedLocomotor (IPiggyback storage)

    // Install vtables
    this[0] = &TeleportLocomotionClass__IUnknown_vtable;     // 0x7F50CC
    this[1] = &TeleportLocomotionClass__ILocomotion_vtable;  // 0x7F5000
    this[6] = &TeleportLocomotionClass__IPiggyback_vtable;   // 0x7F4FDC

    return this;
}
```

### Complete Field Map (verified from constructor + all function cross-refs)

| Offset | Size | Field | Init | Verified From |
|--------|------|-------|------|---------------|
| +0x00 | 4 | IUnknown vtable | 0x7F50CC | Constructor |
| +0x04 | 4 | ILocomotion vtable | 0x7F5000 | Constructor |
| +0x08 | 4 | Refcount | 0 | LocomotionClass base |
| +0x0C | 4 | LinkedTo (FootClass*) | 0 | LocomotionClass base |
| +0x10 | 1 | Powered | 1 | LocomotionClass base |
| +0x11 | 1 | field_11 | 1 | LocomotionClass base |
| +0x14 | 4 | field_14 | 0 | LocomotionClass base |
| +0x18 | 4 | IPiggyback vtable | 0x7F4FDC | Constructor |
| +0x1C | 12 | HeadToCoord (X,Y,Z) | NullCoord | Constructor, HeadToCoord |
| +0x28 | 12 | DestCoord (X,Y,Z) | NullCoord | Constructor, Process |
| +0x34 | 1 | IsMoving | 0 | Is_Moving, HeadToCoord |
| +0x35 | 1 | field_35 | 0 | Is_Ok_To_End |
| +0x36 | 1 | field_36 | 0 | Stop_Moving |
| +0x38 | 4 | WarpPhase | 0 | StateMachineTick |
| +0x3C | 4 | Timer.StartFrame | CurrentFrame | TimerCheck, Phase 0 |
| +0x40 | 4 | Timer.field_4 | (uninit) | Vestigial CDTimerClass field |
| +0x44 | 4 | Timer.Duration | 0 | TimerCheck, Phase 0 |
| +0x48 | 4 | PiggybackedLocomotor | 0 | Begin_Piggyback, End_Piggyback |

**Total struct size: 0x4C (76 bytes).**

**Confidence: 98%** -- Every field verified from constructor assembly and cross-referenced
against all functions that read/write these offsets.

---

## 2. HeadToCoord (0x00718100) -- Destination Setup

### Full Decompiled Logic

HeadToCoord is ILocomotion vtable slot 17 (offset 0x44). Called by
`FootClass::Set_Destination_Internal` (0x4D94B0) when the active locomotor is
TeleportLocomotionClass.

**NOTE:** In this function, `param_1` is the ILocomotion interface pointer (base+0x04),
so field offsets are shifted by -4 relative to the struct base. E.g., `param_1+0x08` =
base+0x0C = LinkedTo.

```c
void HeadToCoord(ILocomotion* this, CoordStruct dest) {
    FootClass* techno = this->LinkedTo;  // ILoco+0x08 = base+0x0C

    // Guard 1: Is unit warping out?
    if (techno->vtable->IsWarpingOut())  // vtable+0x37C
        goto abort;

    // Guard 2: Is unit warping in?
    if (techno->vtable->IsWarpingIn())   // vtable+0x380
        goto abort;

    // Guard 3: Is unit deploying?
    if (techno->vtable->IsDeploying())   // vtable+0x1D4
        goto abort;

    // Guard 4: Is unit undeploying?
    if (techno->vtable->IsUndeploying()) // vtable+0x1D8
        goto abort;

    // All guards passed -- process the destination

    // If techno is Deployed (FootClass+0x1F8, byte at techno+0x9C*4 byte field):
    // Actually: *(char*)(techno[0x7E]) which is techno+0x1F8 as dword index
    // If techno byte flag is set AND unit is infantry:
    if ((char)(techno[0x7E]) != 0) {
        int abstractType = techno->vtable->WhatAmI();  // vtable+0x2C
        if (abstractType == 0xF) {  // Infantry
            // Scatter objects at destination cell
            CellClass* destCell = CellClass::Get_Cell_At(&dest);
            CellClass::Scatter_Objects(&g_NullCoord, 1, 1, 0);
        }
    }

    // Call Process to validate and resolve destination
    bool valid = TeleportLocomotionClass::Process(&dest);

    // Check if DestCoord was set to NullCoord (invalid destination)
    if (this->DestCoord == NullCoord) {
        // Destination invalid -- call SetOccupation(0, 1) and return
        techno->vtable->SetOccupation(0, 1);  // vtable+0x480
        return;
    }

    // Destination is valid -- arm the state machine
    this->IsMoving = 1;         // base+0x34 (via ILoco: +0x30)

    // Copy DestCoord -> HeadToCoord
    this->HeadToCoord.X = this->DestCoord.X;  // base+0x1C = base+0x28
    this->HeadToCoord.Y = this->DestCoord.Y;  // base+0x20 = base+0x2C
    this->HeadToCoord.Z = this->DestCoord.Z;  // base+0x24 = base+0x30
    return;

abort:
    // Unit is in a warp/deploy state -- clear NavCom target
    techno->DockBuilding (+0x5A4) = NULL;
    return;
}
```

### Key Observations

1. **Four guard conditions** prevent warping while the unit is already in a
   deploy/undeploy/warp transition.
2. Infantry scatter at dest cell only happens when a specific flag on the techno
   (offset 0x1F8 byte) is set -- this appears to be a "deployed" or "garrisoned" state byte.
3. Process() does the actual destination validation and writes DestCoord.
4. If Process leaves DestCoord as NullCoord, the movement is aborted with SetOccupation.
5. If valid, IsMoving=1 is set and HeadToCoord is armed -- next StateMachineTick Phase 0
   sees Is_Moving()=true and initiates the warp.

**Confidence: 95%** -- Fresh decompilation verified against assembly.

---

## 3. Process (0x00718B70) -- Destination Validation Deep Dive

This is the heart of destination validation. Called from HeadToCoord to resolve and validate
where the unit should warp to. Returns 1 if destination is valid, 0 if invalid.

**param_1** = TeleportLocomotionClass (base pointer, NOT ILocomotion offset).

### 3.1 Bridge Detection (cell+0x140 & 0x100)

```c
CellClass* destCell = CellClass::Get_Cell_At(&resolvedDest);
bool isOnBridge;

if ((destCell->Flags (+0x140) & 0x100) == 0) {
    isOnBridge = false;   // no bridge at destination
} else {
    // Cell has bridge flag -- check if unit is ABOVE the bridge
    int groundHeight = CellClass::GetGroundHeight(&resolvedDest);
    int unitZ = techno->Location.Z;  // techno+0xA4
    isOnBridge = (unitZ > groundHeight + g_BridgeZOffset * 3);
    // g_BridgeZOffset is at 0xB0EC38 (runtime value, 0 in .data)
    // If unit Z is above 3x bridge height offset, consider it on bridge
    // Otherwise treat as ground level
}
```

**CellClass+0x140 is the cell flags field.** Bit 0x100 (bit 8) = **HasBridge** flag.
This is a packed bitfield checked throughout the engine. When set, the cell has a bridge
overlay and special bridge-aware logic applies.

Bit 0x200 (bit 9) = **BridgeDestroyed** flag. Used in Update_Position to detect destroyed
bridges.

### 3.2 Non-Infantry Pathfinding Validation

For non-infantry units (WhatAmI() != 0xF) that are NOT ChronoInTransit:

```c
if (techno->ChronoInTransit (+0x27C) == 0) {
    TypeClass* type = techno->vtable->GetType();  // vtable+0x84
    int speedType = type->SpeedType (+0x5B4);

    // Convert dest to cell coordinates
    short cellX = (short)(dest.X + (dest.X >> 31 & 0xFF)) >> 8;
    short cellY = (short)(dest.Y + (dest.Y >> 31 & 0xFF)) >> 8;

    // Skip if destination is the "null cell" (0,0)
    if (cellX == DAT_00b0ebd8 && cellY == DAT_00b0ebda) {
        // Already at null cell, skip validation
    } else {
        // Step 1: Basic passability check
        CellClass* cell = MapClass::Get_CellClass(cellXY);
        int canEnter = techno->vtable->CanEnterCell(cell, -1, -1, 0, 1);
        // vtable+0x1AC: CanEnterCell(cell, fromDir, toDir, ?, ?)

        if (canEnter != 0) {
            // Cell is passable! Now validate zone connectivity

            // Step 2: Get zone ID at destination
            int zoneID = MapClass::GetZoneID(cellXY_shifted, speedType, isOnBridge);

            // Step 3: Call Pathfinding_validate_alternate
            TypeClass* type2 = techno->vtable->GetType();
            int speedType2 = type2->field_67C;  // secondary speed type
            short* result = Pathfinding_validate_alternate(
                &outCell,           // output: validated cell
                &cellXY,            // input: target cell
                speedType2,         // speed type for zone check
                zoneID,             // zone to match
                speedType,          // speed type
                isOnBridge,         // bridge flag
                1, 1, 0, 0, 0, 1,  // search params
                &cellBuffer, 0, 0
            );
            cellXY = *result;  // use validated cell
        }
    }

    // If cell is valid (not null cell):
    if (cellX != DAT_00b0ebd8 || cellY != DAT_00b0ebda) {
        // Snap to cell center
        this->DestCoord.X = cellX * 256 + 128;  // cell center X
        this->DestCoord.Y = cellY * 256 + 128;  // cell center Y
        this->DestCoord.Z = 0;
        this->DestCoord.Z = CellClass::GetGroundHeight(&this->DestCoord);

        // Set destination on techno
        techno->vtable->SetCoord(&this->DestCoord);  // vtable+0xF0
    }
}
```

**Key insight**: Non-infantry teleporters always snap to cell center (X*256+128, Y*256+128).
The Z coordinate is set to ground height at that cell. `Pathfinding_validate_alternate`
finds a nearby valid cell if the original destination is in an unreachable zone.

### 3.3 Pathfinding_validate_alternate (0x00????)

This is a large function (376 lines decompiled). It searches nearby cells for a valid
destination when the original cell is unreachable. Key behavior:

- Takes a target cell coordinate and a speed type
- Searches in expanding rings from the target
- For each candidate cell, checks zone connectivity using `MapClass::GetZoneID`
- Returns the first cell that matches the required movement zone
- If no valid cell found, returns the null cell coordinate

This ensures that chrono miners never warp onto impassable terrain or into disconnected
movement zones.

### 3.4 Infantry Sub-Cell Positioning

For infantry (WhatAmI() == 0xF), the logic is very different:

```c
// Infantry gets mission check first
int mission = techno->vtable->GetCurrentMission();  // vtable+0x184

// Missions 8, 9, 7, 25 (0x19) allow dock-at-destination check
if (mission == 8 || mission == 9 || mission == 7 || mission == 0x19) {
    // These are: Enter(7), Capture(8), Eaten(9), Patrol(25)
    // Check if techno has a DockBuilding target
    goto check_dock_building;
}
```

**Mission constants (verified from mission name table at 0x816CAC):**
- 7 = Mission_Enter (entering a building/transport)
- 8 = Mission_Capture (engineer capturing a building)
- 9 = Mission_Eaten (consumed by a unit, e.g., bio-reactor)
- 0x19 (25) = Mission_Patrol

When one of these missions is active:

```c
check_dock_building:
    bool isDocking = false;

    // Check DockBuilding pointer at techno+0x5A4
    BuildingClass* dock = techno->DockBuilding (+0x5A4);

    if (dock != NULL) {
        int dockType = dock->vtable->WhatAmI();  // vtable+0x2C

        // Type 1 = BuildingClass: check if dock cell matches dest cell
        if (dockType == 1) {
            short* dockCell = dock->vtable->GetMapCoords();  // vtable+0x1B8
            if (dockCell matches dest cell coords) {
                isDocking = true;
            }
        }

        // Type 2 = AircraftClass? Same check
        if (dockType == 2) {
            // same cell comparison
        }

        // Type 6 = another object type: Look up building in cell
        if (dockType == 6) {
            CellClass::Get_Cell_At(dest);
            BuildingClass* found = Look_up_building_in_cell();
            if (found == dock) {
                isDocking = true;
            }
        }
    }
```

After the dock check, infantry placement uses `CellClass::PlaceInfantryInCell`:

```c
// Get the destination cell
CellClass* cell = CellClass::Get_Cell_At(&destCoords);

// Place infantry in a free sub-cell
CoordStruct* placed = CellClass::PlaceInfantryInCell(
    &outCoord,      // output coordinates
    cell,           // target cell
    &destCoords,    // desired position
    isDocking,      // if true, allow docking-specific placement
    isOnBridge      // bridge awareness
);

// Update DestCoord with the placed position
this->DestCoord = *placed;
```

`PlaceInfantryInCell` (0x00480FA0) determines the sub-cell position:
1. Calculates which quadrant the destination coordinates fall in (using distance from
   cell center)
2. If distance < 0x3C (60 leptons) from center, uses sub-cell 0 (center)
3. Otherwise maps to quadrant: bit0 = (X > 0x80), bit1 = (Y > 0x80), +1 if nonzero
   -> yields sub-cells 0-4
4. Checks cell occupation bits at CellClass+0x124 (ground) or +0x128 (bridge):
   - Each sub-cell has a bit in the occupation byte
   - If target sub-cell is occupied, tries alternate sub-cells in priority order
   - Data table at 0x0081CC84 defines the priority order per quadrant
5. Returns the sub-cell's world coordinates (cell base + sub-cell offset from table
   at 0x0089E9F0)

### 3.5 Post-Placement Validation

After placing infantry, the code checks if the resulting cell is actually passable:

```c
CellClass* resultCell = MapClass::Get_CellClass(resultCellXY);
int canEnter = techno->vtable->CanEnterCell(resultCell, -1, -1, 0, 1);

if (canEnter != 0) {
    // Cannot enter -- invalidate destination
    this->DestCoord = NullCoord;
}
```

If the destination cell is blocked after infantry placement, DestCoord is set to NullCoord,
which causes HeadToCoord to abort the movement.

### 3.6 Alternate Cell Search for Infantry

When the destination is invalidated, the code checks if the infantry has a NavTarget
(`techno->NavTarget` at +0x5A4) and whether that target is a **Foot** (bit 2 = IsFoot
of `AbstractFlags @+0x14`, set by `FootClass::Constructor` at `0x004D34DD`):

```c
FootClass* navTarget = techno->NavTarget (+0x5A4);
if (navTarget != NULL && (*(byte*)(navTarget + 0x14) & 0x04) != 0) {
    // NavTarget is a Foot (Infantry/Unit/Aircraft, NOT a Building) -- try to find an adjacent cell
    TypeClass* type = techno->vtable->GetType();
    int speedType = type->SpeedType (+0x5B4);

    // Get NavTarget's map position
    CoordStruct* navPos = navTarget->vtable->GetCoords(&buf, 0);
    short navCellX = (short)(navPos->X >> 8);
    short navCellY = (short)(navPos->Y >> 8);

    // Get techno's map position
    CoordStruct* myPos = techno->vtable->GetCoords(&buf2, 0);
    short myCellX = (short)(myPos->X >> 8);
    short myCellY = (short)(myPos->Y >> 8);

    // Get cell properties for zone check
    CellClass* myCell = MapClass::Get_CellClass(myCellXY);
    uint bridgeFlag = (myCell->Flags (+0x140) >> 8) & 0x01000001; // bridge isolation
    byte onBridge = techno->vtable->IsOnBridge(0);  // vtable+0xBC

    int zoneID = MapClass::GetZoneID(navCellXY + 0x24, speedType, onBridge);

    TypeClass* type2 = techno->vtable->GetType();
    short* altCell = Pathfinding_validate_alternate(
        &outBuf, &navCellXY, type2->field_67C,
        zoneID, speedType, bridgeFlag, ...
    );

    if (*altCell is valid) {
        CellClass* altCellObj = MapClass::Get_CellClass(altCell);
        CoordStruct* altCoord = altCellObj->vtable->GetCenterCoords(&buf3, isDocking, ...);
        CellClass::PlaceInfantryInCell(&outCoord, altCoord, isDocking, ...);
        this->DestCoord = outCoord;
    }
}
```

This fallback ensures infantry can still reach a building even if the exact destination
cell is occupied -- it finds the nearest accessible cell adjacent to the NavTarget.

**Confidence: 85%** -- Complex function with heavy stack manipulation. Core flow verified
but infantry placement details have some decompilation artifacts.

---

## 4. Update_Position (0x00718260) -- How the Unit Teleports

### Two Modes of Operation

**param_5 (applyOccupancy)** controls the behavior:
- `param_5 == 0`: "Simple teleport" -- deals chrono damage to objects at destination,
  then teleports the unit there
- `param_5 != 0`: "Relative offset" -- reads ChronoDestCoord from TechnoClass+0x288/28C/290
  and applies it as the new position

### Mode 0: Destination Damage + Teleport

```c
// Get destination cell
CellClass* destCell = CellClass::Get_Cell_At(&dest);
ObjectClass* objList;

// Select object list based on bridge presence
if (destCell->Flags & 0x100) {
    objList = destCell->BridgeOccupants (+0xE8);  // objects ON the bridge
} else {
    objList = destCell->FirstObject (+0xE4);       // ground-level objects
}

// Iterate all objects at destination cell
for (ObjectClass* obj = objList; obj != NULL; obj = obj->NextObject (+0x30)) {

    if (!obj->IsInAir() && obj->WhatAmI() == 0xF && techno->WhatAmI() == 0xF) {
        // BOTH are infantry and at same position:
        // Check if positions match exactly
        CoordStruct* objCoord = obj->vtable->GetCoords(&buf);
        if (objCoord->X == dest.X && objCoord->Y == dest.Y && objCoord->Z == dest.Z) {
            // Deal full damage to the other infantry
            TypeClass* theirType = obj->vtable->GetType();
            int damage = theirType->Strength (+0xA0);
            obj->vtable->TakeDamage(
                &damage, 0,
                Rules->ChronoWarpDamagePercent (+0xFA8),
                0, 1, 0, 0
            );
        }
    }
    else if (!obj->IsInAir()) {
        // Non-flying techno: check IsTechno flag
        if (obj != NULL && (*(byte*)(obj + 0x14) >> 2 & 1) != 0) {
            // IsTechno bit is set -- deal chrono damage
            TypeClass* theirType = obj->vtable->GetType();
            int damage = theirType->Strength;
            obj->vtable->TakeDamage(
                &damage, 0, Rules->ChronoWarpDamagePercent, 0, 1, 0, 0
            );
        } else {
            // Not a techno (terrain object, etc.) -- skip damage but flag param_5
            // applyOccupancy = true (force occupancy validation)
        }
    }
    else {
        // Flying object at destination:
        // Deal chrono damage to THIS unit (the teleporter)
        TypeClass* myType = techno->vtable->GetType();
        int damage = myType->Strength (+0xA0);
        techno->vtable->TakeDamage(
            &damage, 0, Rules->ChronoWarpDamagePercent, 0, 1, 0, 0
        );
    }
}

// Bridge integrity check
CellClass* destCell2 = CellClass::Get_Cell_At(&dest);
if ((destCell2->Flags & 0x100) != 0 && (destCell2->Flags & 0x200) == 0) {
    // Cell has bridge but bridge is NOT destroyed
    // Force occupancy check (param_5 = true)
    applyOccupancy = true;
}
```

**Important damage rules:**
- If dest has a flying unit: the TELEPORTER takes damage (telefragged by air unit)
- If dest has infantry and teleporter is infantry: the OTHER infantry takes damage
- If dest has any other techno: the OTHER techno takes damage
- Damage amount = target's full Strength, modified by Rules->ChronoWarpDamagePercent warhead

### Mode 1: ChronoSphere Relative Offset

When `param_5 != 0` (called from StateMachine Phase 2/3 during ChronoSphere warp):

```c
// Read ChronoDestCoord from TechnoClass
this->DestCoord.X = techno->ChronoDestCoord_X (+0x288);
this->DestCoord.Y = techno->ChronoDestCoord_Y (+0x28C);
this->DestCoord.Z = techno->ChronoDestCoord_Z (+0x290);

// Get ground height
this->DestCoord.Z = CellClass::GetGroundHeight(&this->DestCoord);

// Bridge detection and Z offset
CellClass* destCell = CellClass::Get_Cell_At(&this->DestCoord);
if ((destCell->Flags & 0x100) == 0 || techno->IsOnBridge (+0x8C)) {
    techno->IsOnBridge = 0;
} else {
    techno->IsOnBridge = 1;
    this->DestCoord.Z += g_BridgeZOffset_Teleport;  // global at 0xB0EC2C
}

// Place unit at destination
techno->vtable->SetCoord(&this->DestCoord);  // vtable+0xF0
```

### Return Value

```c
// After either mode, sync final position
if (this->DestCoord == NullCoord) {
    techno->vtable->SetCoord(&techno->Location);
    return 0;  // not arrived
}
techno->vtable->SetCoord(&this->DestCoord);
return 1;  // arrived
```

**The teleport is INSTANT** -- there is no interpolation. The unit jumps directly from
its current position to the destination coordinates in a single call.

The visual warp effect (fade out, sparkle, fade in) is purely cosmetic, handled by
animations and the BeingWarped rendering flag. The actual position update happens in one
step during Phase 2/3.

### Pathfinding in Mode 0

When `applyOccupancy` is forced true (by flag or bridge check), the code performs zone
validation using Pathfinding_validate_alternate:

```c
// Get speed type for zone check
TypeClass* type = techno->vtable->GetType();
int speedType = type->SpeedType (+0x5B4);
// Map speed type for zone lookup (9->0, 2->0, 3->5, others unchanged)

CellStruct destCellXY = CellStruct::FromCoord(dest);
int zoneID = MapClass::GetZoneID(currentCell + 0x24, speedType, isOnBridge);

short* validCell = Pathfinding_validate_alternate(
    &outBuf, &destCellXY, 1, zoneID, speedType, bridgeFlag,
    1, 1, 0, 0, 0, 1, &cellBuf, 0, 0
);

// Get validated cell center coords
CellClass* validCellObj = CellClass::Get_Cell_At(validCell);
CoordStruct* cellCenter = validCellObj->vtable->GetCenterCoords(&buf);

// Calculate relative offset: dest - validCellCenter + originalCellCenter
techno->ChronoDestCoord_X = cellCenter->X + (dest.X - originalCenter.X);
techno->ChronoDestCoord_Y = cellCenter->Y + (dest.Y - originalCenter.Y);
techno->ChronoDestCoord_Z = cellCenter->Z + (dest.Z - originalCenter.Z);
```

This adjusts the ChronoDestCoord when the original destination cell is unreachable,
preserving the sub-cell offset relative to the new valid cell.

**Confidence: 85%** -- Mode 1 is clear. Mode 0 damage logic has some decompilation
complexity but core paths verified.

---

## 5. PostWarpValidation (0x007187A0) -- Death on Water

### Step-by-Step Flow

```c
void PostWarpValidation(TeleportLoco* this, CoordStruct dest) {
    FootClass* techno = this->LinkedTo;  // base+0x0C

    // ---- Step 1: Damage flying objects at destination ----
    CellClass* destCell = CellClass::Get_Cell_At(&dest);
    for (ObjectClass* obj = destCell->FirstObject (+0xE4);
         obj != NULL;
         obj = obj->NextObject (+0x30))
    {
        if (obj->IsInAir()) {  // vtable+0x160
            TypeClass* myType = techno->vtable->GetType();
            int damage = myType->Strength (+0xA0);
            techno->vtable->TakeDamage(
                &damage, 0, Rules->ChronoWarpDamagePercent (+0xFA8), 0, 1, 0);
        }
    }

    // ---- Step 2: Chronoshiftable bridge check ----
    TypeClass* type = techno->vtable->GetType();
    if (type->Chronoshiftable (+0xCCE)) {
        // If Chronoshiftable, check for bridge at destination
        CellClass* destCell2 = CellClass::Get_Cell_At(&dest);
        if (destCell2->Flags & 0x100) {
            // Bridge present -- additional handling (details unclear)
        }
    }

    // ---- Step 3: Naval unit power validation ----
    bool hasNavalPower = false;
    TypeClass* type2 = techno->vtable->GetType();

    if (type2->SpeedType_Secondary (+0x67C) == 3) {  // SPEED_FLOAT (naval)
        hasNavalPower = true;
        if (type2->NeedsEngineer (+0x410)) {
            HouseClass* house = techno->Owner (+0x21C);
            if (!HouseClass::HasPowerSurplus(house)) {
                hasNavalPower = false;  // no power = no naval protection
            }
        }
    }

    // ---- Step 4: WATER DEATH CHECK ----
    CellStruct destCellXY = CellStruct::FromCoord(dest);
    CellClass* cellObj = MapClass::Get_CellClass(&destCellXY);

    if (cellObj->LandType (+0xEC) == 2  /* WATER */  && !hasNavalPower) {
        // Destination is water and unit has no naval capability!

        // Exception 1: Chronoshiftable units survive
        if (type->Chronoshiftable (+0xCCE))
            goto passability_check;

        // Exception 2: Infantry can never land on water via teleport
        // (infantry are caught by abstractType != 0xF check)
        if (techno->vtable->WhatAmI() == 0xF)
            goto passability_check;

        // Exception 3: Check if there's a bridge over the water
        CellClass* bridgeCheck = CellClass::Get_Cell_At(&dest);
        if (bridgeCheck->Flags & 0x100) {
            // Bridge exists -- unit survives on bridge
            goto passability_check;
        }

        // Exception 4: Check if land type is actually Road (1)
        // This handles cells that are "water" visually but have road overlay
        CellClass* landCheck = CellClass::Get_Cell_At(&dest);
        if (landCheck->LandType (+0xEC) == 1  /* ROAD/CLEAR */) {
            goto passability_check;
        }

        // ============ UNIT DIES ON WATER ============

        // Set the falling/self-destruct flag
        techno->ShouldSelfDestruct (+0x3CD) = 1;

        // Call KillSelf via vtable
        techno->vtable->KillSelf();  // vtable+0x3A0

        // Handle linked building (ChronoSphere source)
        if (techno->LinkedBuilding (+0x2D8) != 0) {
            // Release passengers/cargo via FUN_006B0AE0
            FUN_006B0AE0(
                techno->ChronoSourceBuilding (+0x428),
                techno->ChronoSourceHouse (+0x42C)
            );

            // Destroy the linked anim
            AnimClass* anim = techno->LinkedBuilding (+0x2D8);
            if (anim != NULL) {
                anim->vtable->Remove(1);  // vtable+0x20
            }
            techno->LinkedBuilding = 0;
        }

        // Try to scatter or destroy based on targets
        if (techno->TargetA (+0x428) != 0) {
            techno->vtable->ScatterFrom(techno->TargetA);  // vtable+0xE0
        } else if (techno->TargetB (+0x42C) != 0) {
            techno->vtable->ScatterFromB(techno->TargetB);  // vtable+0xE4
        }
        return;  // unit is dead
    }

passability_check:
    // ---- Step 5: General passability check ----
    CellStruct cellXY2 = CellStruct::FromCoord(dest);
    CellClass* passCell = MapClass::Get_CellClass(&cellXY2);
    int canEnter = techno->vtable->CanEnterCell(passCell, -1, -1, 0, 1);

    if (canEnter == 7  /* MOVE_NO */  || /* blocked */) {
        // Check for bridge overlay at original position
        CellStruct origCellXY = CellStruct::FromCoord(techno->OriginalLocation);
        CellClass* origCell = MapClass::Get_CellClass(&origCellXY);
        bool hasBridge = CellClass::HasBridgeOverlay(origCell);

        if (!hasBridge || techno->vtable->WhatAmI() == 0xF) {
            // No bridge or infantry: deal chrono damage to self (not lethal)
            TypeClass* myType = techno->vtable->GetType();
            int damage = myType->Strength;
            techno->vtable->TakeDamage(
                &damage, 0, Rules->ChronoWarpDamagePercent, 0, 1, 0, 0);
        } else {
            // On bridge and blocked by impassable terrain:
            // ============ UNIT DIES ON BRIDGE ============
            techno->ShouldSelfDestruct (+0x3CD) = 1;
            techno->vtable->KillSelf();

            // Same linked building cleanup as water death
            if (techno->LinkedBuilding (+0x2D8) != 0) {
                FUN_006B0AE0(
                    techno->ChronoSourceBuilding (+0x428),
                    techno->ChronoSourceHouse (+0x42C)
                );
                AnimClass* anim = techno->LinkedBuilding;
                if (anim) anim->vtable->Remove(1);
                techno->LinkedBuilding = 0;
            }
            return;
        }
    }
}
```

### CellClass+0xEC -- LandType Field

**CellClass+0xEC is the LandType enum field (int/byte).** The LandType enum values:

| Value | Name | Notes |
|-------|------|-------|
| 0 | Clear | Default passable terrain |
| 1 | Road | Paved surfaces, passable |
| 2 | **Water** | Triggers death for non-naval units |
| 3 | Rock | Impassable rocky terrain |
| 4 | Wall | Building walls |
| 5 | Tiberium | Ore/gem fields |
| 6 | Beach | Shoreline |
| 7 | Rough | Rough ground |
| 8 | Ice | Frozen water |
| 9 | Railroad | Train tracks |
| 10 | Tunnel | Underground passage |
| 11 | Weeds | Weed terrain |

The water death check specifically tests for `LandType == 2` (Water).

### +0x3CD -- ShouldSelfDestruct Flag

**TechnoClass+0x3CD** is a byte flag. When set to 1:
- Signals that the unit should play a death animation and be removed
- Set by PostWarpValidation when the unit warps onto invalid terrain
- The vtable+0x3A0 call (KillSelf) triggers the actual death sequence
- Used as a "falling into water" or "crushed by chrono" visual

### Kill Credit via FUN_006B0AE0

**FUN_006B0AE0** (at 0x006B0AE0) handles releasing cargo/passengers from a
building/transport and assigning kill credit.

```c
void FUN_006B0AE0(int this, int sourceBuilding, int sourceHouse) {
    // 'this' appears to be a cargo container or building occupant list

    if (this->OccupantCount (+0x24) == 0) return;

    // Find the house that owns the source building
    int sourceHouseIndex = FUN_006A46D0(sourceBuilding);
    // FUN_006A46D0 iterates g_HouseClass_Array to find which house has
    // a type matching the source building's type GUID

    HouseClass* killerHouse = NULL;
    for (int i = 0; i < g_HouseClass_Array_Count; i++) {
        if (g_HouseClass_Array[i]->Type->GUID (+0xBC) == sourceHouseIndex) {
            killerHouse = g_HouseClass_Array[i];
            break;
        }
    }

    // Iterate occupants in reverse order
    for (int i = this->OccupantCount (+0x48) - 1; i >= 0; i--) {
        TechnoClass* occupant = this->OccupantList[i];
        if (occupant == NULL || !g_GameActive) continue;

        occupant->field_2DC = 0;  // clear some state

        if (occupant->IsAlive (+0x81)) {
            // Occupant is in limbo -- deal damage from source building
            occupant->vtable->ScatterFrom(sourceBuilding);  // vtable+0xE0
            occupant->vtable->Remove();  // vtable+0xF8
            continue;
        }

        if (sourceBuilding == 0) {
            // No source building -- use sourceHouse for credit
            if (sourceHouse == 0 && killerHouse == 0) {
                // No credit possible -- just deal damage to self
                int damage = occupant->Strength;
                occupant->vtable->TakeDamage(
                    &damage, 0, Rules->ChronoWarpDamagePercent, 0, 0, 0, 0);
                continue;
            }
            // Use house from sourceHouse or killerHouse
        }

        // Assign kill credit to the source house
        HouseClass* creditHouse = (sourceBuilding != 0)
            ? sourceBuilding->OwnerHouse (+0x21C)
            : killerHouse;

        occupant->vtable->SetKiller(creditHouse, 1);     // vtable+0x3D4
        occupant->vtable->SetKillerWeapon();              // vtable+0x3D0
        occupant->vtable->RecordKill(1);                  // vtable+0x388
    }

    // Spawn explosion anim if applicable
    if (g_GameActive && firstKilled != NULL && !g_MapEditorMode) {
        if (Rules->ChronoKillExplosion (+0x234) != -1) {
            AnimClass::SpawnAtCoord(0);
        }
    }

    this->OccupantCount = 0;
}
```

### ChronoSourceBuilding (+0x428) and ChronoSourceHouse (+0x42C)

These TechnoClass fields store the source of a ChronoSphere warp:

- **+0x428 (ChronoSourceBuilding)**: Pointer to the ChronoSphere building that initiated
  the warp. Used for kill credit -- if a chrono-warped unit dies on water, the kill is
  credited to the ChronoSphere's owner.

- **+0x42C (ChronoSourceHouse)**: Pointer to the HouseClass that owns the ChronoSphere.
  Fallback for kill credit when the building pointer is invalid.

These are set by the ChronoSphere superweapon handler (0x65EC30) and cleared:
- In Phase 5 after post-warp validation completes
- In End_Piggyback when the locomotor is released
- Both are always 0 for self-teleporting units (chrono miners, chrono legionnaires)

**Confidence: 90%** -- PostWarpValidation flow verified. FUN_006B0AE0 is complex with
heavy iteration but core kill-credit logic is clear.

---

## 6. TimerCheck (0x00719BF0) -- Detailed Analysis

### Full Decompiled Logic

```c
void TimerCheck(TeleportLocomotionClass* this) {
    // this = IUnknown interface pointer (base+0x00)
    // Timer is at base+0x3C/0x40/0x44
    // WarpPhase is at base+0x38

    int duration = this->Timer.Duration;  // +0x44
    if (this->Timer.StartFrame != -1) {   // +0x3C
        int elapsed = g_CurrentFrameCounter - this->Timer.StartFrame;
        if (elapsed >= duration) {
            goto timer_expired;
        }
        duration = duration - elapsed;  // remaining time
    }
    if (duration != 0) {
        return;  // timer still counting down
    }

timer_expired:
    // Timer has expired!

    // Step 1: Clear BeingWarped flag on the techno
    FootClass* techno = this->LinkedTo;  // +0x0C
    techno->BeingWarped (+0x271) = 0;

    // Step 2: Post-timer idle behavior
    // Check TechnoClass+0x2B4
    if (techno->field_2B4 (+0x2B4) == 0) {
        // No active target/action -- do idle behavior
        FUN_0070F770(techno);   // Set random idle timer
        bool scattered = FUN_00709480(techno);  // Try to scatter/find target
        if (!scattered) {
            // Couldn't scatter -- just update occupation
            techno->vtable->SetOccupation(0, 1);  // vtable+0x484
        }
    }

    // Step 3: Advance phase
    if (this->WarpPhase (+0x38) > 0) {
        this->WarpPhase++;
    }
}
```

### TechnoClass+0x2B4 -- What Is This?

Based on analysis of all contexts where +0x2B4 is checked:

**TechnoClass+0x2B4 appears to be a queued action/target pointer.** When non-zero, the
unit has a pending action (e.g., a garrison it's entering, a building to capture, etc.)
and should not idle after the chrono timer expires. When zero, the unit is truly idle
and should scatter or find something to do.

The existing report labels this "Garrison" which is plausible -- it may specifically be
the garrison target pointer. However, it could also be a more general "PendingAction"
field. Without finding the exact write sites, I'll maintain moderate confidence.

**Confidence: 75%** for exact field identity.

### FUN_0070F770 -- Idle Wander Timer

```c
void FUN_0070F770(FootClass* techno) {
    // Reads/writes a CDTimerClass at techno+0x180/0x184/0x188
    // This is an "idle wander" or "AI scan" timer

    int duration = techno->IdleTimer.Duration;  // +0x188
    if (techno->IdleTimer.StartFrame != -1) {   // +0x180
        int elapsed = g_CurrentFrameCounter - techno->IdleTimer.StartFrame;
        if (elapsed >= duration) {
            return;  // timer already expired, don't reset
        }
        duration = duration - elapsed;
    }

    // Only reset if remaining time > 10 frames
    if (duration > 10 && !g_MapEditorMode) {
        int newDuration = Random::RandomRanged(4, 8);  // 4-8 frame random delay
        techno->IdleTimer.StartFrame = g_CurrentFrameCounter;
        techno->IdleTimer.field_4 = <stack value>;
        techno->IdleTimer.Duration = newDuration;
    }
}
```

This function sets a short random delay (4-8 frames) before the unit's next AI scan.
It prevents units from immediately finding new targets after finishing a chrono warp.

### FUN_00709480 -- Scatter/Resume Check

```c
bool FUN_00709480(FootClass* techno) {
    // First check: can this unit scatter?
    bool canScatter = FUN_00709290(techno);
    if (!canScatter) {
        return false;
    }

    // Unit can scatter -- attempt to scatter
    int prevTarget = techno->field_2B4;  // +0x2B4 (current target)
    techno->field_4FC = g_CurrentFrameCounter;  // timestamp

    CoordStruct* coords = techno->vtable->GetCoords(&buf, 1);
    bool scattered = techno->vtable->TryScatter(coords);  // vtable+0x39C

    // If scattered AND target changed, mark as having new action
    if (scattered && techno->field_2B4 != prevTarget) {
        techno->field_50C = 1;
        return true;
    }

    return false;
}
```

### FUN_00709290 -- Scatter Eligibility Check

```c
bool FUN_00709290(FootClass* techno) {
    // Complex eligibility check with multiple conditions:

    // Path 1: Already has a target in field_2B4 AND type is compatible
    if (techno->field_2B4 == 0 && !FUN_0050B730()
        && (techno->IsTechno flag)
        && techno->NavTarget (+0x5A4) != 0)
    {
        int navTypeClass = techno->NavTarget->TypeClass;
        if (!navTypeClass->field_AF && navTypeClass->field_AD
            && techno->AbstractType == 2)  // UnitClass
        {
            return true;
        }
    }

    // Path 2: General scatter check
    if (!FUN_007091D0(techno)) return false;

    // Path 3: Unit is a unit (type 2) with deployable type
    if (techno->AbstractType == 2) {
        TypeClass* type = techno->vtable->GetType();
        if (type->AutoDeploy (+0xD6A)) {
            // Check if at own building and not garrisoned
            if (techno->IsTechno && techno->vtable->GetBuilding() == techno->NavTarget) {
                if (techno->field_514 == 0) return true;
                // Additional garrison count check
            }
        }
        // More building-specific checks
    }

    // Path 4: WanderAllowed check
    TypeClass* type = techno->vtable->GetType();
    if (!type->WanderAllowed (+0x6AF)) {
        // Check if aircraft in dock
        if (techno->AbstractType != 5) return false;
        // Aircraft-specific dock check
    }

    return true;
}
```

**Confidence: 80%** -- FUN_0070F770 is straightforward. FUN_00709480 and FUN_00709290
are complex with many branches but the core scatter/idle logic is clear.

---

## 7. Mark_All_Occupation_Bits (0x007192C0) -- Occupation Update

### Decompiled Assembly

This function is surprisingly simple -- it's a thin wrapper:

```asm
007192c0: MOV EDX, [ESP+0x4]     ; EDX = param_2 (ILocomotion this)
007192c4: MOV EAX, [ESP+0x8]     ; EAX = param_3 (some value)
007192c8: LEA ECX, [ESP+0x8]     ; ECX = &param_3
007192cc: MOV [ESP+0x8], EAX     ; store param_3 at stack
007192d0: PUSH ECX               ; push &param_3
007192d1: MOV ECX, [EDX+0x8]     ; ECX = ILocomotion+0x08 = LinkedTo (FootClass*)
007192d4: ADD ECX, 0x388         ; ECX = FootClass + 0x388
007192da: CALL 0x004C9220        ; RateTimer::Set
007192df: RET 0x8
```

**What it actually does:**

Mark_All_Occupation_Bits calls `RateTimer::Set` on a RateTimer located at
**FootClass+0x388**. It passes the parameter as the new timer value.

`RateTimer::Set` (0x004C9220) is a timer-with-rate class that interpolates between values.
It stores:
- Current value (short at +0x00)
- Target value (short at +0x04)
- Rate of change (short at +0x14)
- CDTimerClass at +0x08 (start frame, field_4, duration)

When Set is called with a new value that differs from the current:
1. It computes the interpolation distance: `|new - current| / rate`
2. Sets the timer duration to that distance
3. Starts counting down

**FootClass+0x388** is a heading/facing timer -- this function updates the unit's facing
direction after teleportation. The "occupation bits" name is misleading; it actually
triggers the smooth facing rotation to the new heading after the unit warps to its
destination.

**Confidence: 95%** -- Assembly is trivial and fully understood. The RateTimer::Set
decompilation confirms the interpolation behavior.

---

## 8. ILocomotion VTable Layout (0x7F5000)

Complete vtable for TeleportLocomotionClass's ILocomotion interface:

| Slot | Offset | Address | Function | Notes |
|------|--------|---------|----------|-------|
| 0 | 0x00 | 0x71A160 | QueryInterface | Thunk to base+0x00 |
| 1 | 0x04 | 0x71A170 | AddRef | Thunk |
| 2 | 0x08 | 0x71A180 | Release | Thunk |
| 3 | 0x0C | 0x55A710 | Link_To_Object | LocomotionClass base |
| 4 | 0x10 | 0x718080 | **Is_Moving** | Returns base+0x34 == 1 |
| 5 | 0x14 | 0x7180A0 | **Destination** | Returns HeadToCoord or Location |
| 6 | 0x18 | 0x55ACA0 | Head_To_Coord (base) | LocomotionClass default |
| 7 | 0x1C | 0x55ABF0 | (base) | LocomotionClass |
| 8 | 0x20 | 0x55ABE0 | (base) | LocomotionClass |
| 9 | 0x24 | 0x55A730 | (base) | LocomotionClass |
| 10 | 0x28 | 0x55A7D0 | (base) | LocomotionClass |
| 11 | 0x2C | 0x55ABD0 | (base) | LocomotionClass |
| 12 | 0x30 | 0x55A8C0 | (base) | LocomotionClass |
| 13 | 0x34 | 0x55ABC0 | (base) | LocomotionClass |
| 14 | 0x38 | 0x55ABA0 | (base) | LocomotionClass |
| 15 | 0x3C | 0x55ABB0 | (base) | LocomotionClass |
| 16 | 0x40 | 0x7192F0 | **StateMachineTick** | Main state machine (phases 0-7) |
| 17 | 0x44 | 0x718100 | **HeadToCoord** | Validates dest, arms state machine |
| 18 | 0x48 | 0x718260 | **Update_Position** | Called from vtable dispatch |
| 19 | 0x4C | 0x7192C0 | **Mark_All_Occupation_Bits** | Facing timer update |
| 20 | 0x50 | 0x55AC20 | (base) | LocomotionClass |
| 21 | 0x54 | 0x55AB90 | (base) | LocomotionClass |
| 22 | 0x58 | 0x55A8F0 | (base) | LocomotionClass |
| 23 | 0x5C | 0x55A910 | (base) | LocomotionClass |
| 24 | 0x60 | 0x55A930 | (base) | LocomotionClass |
| 25 | 0x64 | 0x55A940 | (base) | LocomotionClass |
| 26 | 0x68 | 0x55AB70 | (base) | LocomotionClass |
| 27 | 0x6C | 0x55AB80 | (base) | LocomotionClass |
| 28 | 0x70 | 0x55AC10 | (base) | LocomotionClass |
| 29 | 0x74 | 0x719E20 | Returns 2 | Layer type = GROUND(2) |
| 30 | 0x78 | 0x55AC00 | (base) | LocomotionClass |
| 31 | 0x7C | 0x55ACE0 | (base) | LocomotionClass |

Slots 0-3, 6-15, 20-28, 30-31 are inherited from LocomotionClass (0x55xxxx addresses).
Slots 4, 5, 16-19, 29 are overridden by TeleportLocomotionClass (0x71xxxx addresses).

---

## 9. IUnknown VTable Layout (0x7F50CC)

| Slot | Offset | Address | Function |
|------|--------|---------|----------|
| 0 | 0x00 | 0x719E30 | QueryInterface |
| 1 | 0x04 | 0x71A0E0 | AddRef |
| 2 | 0x08 | 0x71A0F0 | Release |
| 3 | 0x0C | 0x719C60 | (LocomotionClass method) |
| 4 | 0x10 | 0x4B4C30 | (inherited) |
| 5 | 0x14 | 0x719CA0 | (LocomotionClass method) |
| 6 | 0x18 | 0x719D40 | (LocomotionClass method) |
| 7 | 0x1C | 0x55AB40 | (inherited) |
| 8 | 0x20 | 0x71A130 | (LocomotionClass method) |
| 9 | 0x24 | 0x71A120 | (LocomotionClass method) |
| **10** | **0x28** | **0x719BF0** | **TimerCheck** |
| 11 | 0x2C | 0x718090 | (LocomotionClass method) |

IUnknown vtable slot 10 (offset 0x28) = **TimerCheck** at 0x719BF0.
This is called by Phases 1 and 6 to wait for timer expiry.

---

## 10. IPiggyback VTable Layout (0x7F4FDC)

| Slot | Offset | Address | Function |
|------|--------|---------|----------|
| 0 | 0x00 | 0x71A190 | QueryInterface |
| 1 | 0x04 | 0x71A1A0 | AddRef |
| 2 | 0x08 | 0x71A1B0 | Release |
| 3 | 0x0C | 0x719E90 | **Begin_Piggyback** |
| 4 | 0x10 | 0x719EE0 | **End_Piggyback** |
| 5 | 0x14 | 0x719F30 | **Is_Ok_To_End** |
| 6 | 0x18 | 0x719F80 | Piggyback_CLSID |
| 7 | 0x1C | 0x71A100 | Is_Piggybacking |

### Is_Ok_To_End (0x719F30) -- When Can Piggyback End?

Returns true only when ALL conditions are met:
1. `Is_Moving()` returns false (IsMoving == 0)
2. PiggybackedLocomotor is not NULL (base+0x48 != 0)
3. field_35 (base+0x35) == 0
4. techno->ChronoInTransit (+0x27C) == 0
5. WarpPhase (base+0x38) == 0
6. techno->field_6AD == 0 (not deploying)

This ensures the locomotor swap only happens when the unit is completely idle and not
in any warp or deploy transition.

---

## 11. RulesClass Chrono Fields -- Verified Offsets

Decoded from assembly at 0x0066FAD6 in RulesClass::ReadGeneral. Pattern: MOV default
from [ESI+offset], PUSH string_addr, CALL ReadInt, MOV result to [ESI+offset].

| Offset | INI Key | Type | Default | Notes |
|--------|---------|------|---------|-------|
| +0xBEC | ChronoDelay | int | ? | Post-warp immobilization duration (frames) |
| +0xBF0 | ChronoReinfDelay | int | ? | ChronoSphere reinforcement delay |
| +0xBF4 | ChronoDistanceFactor | int | ? | delay = distance / this value |
| +0xBF8 | ChronoTrigger | bool | ? | Enable distance-based delay calculation |
| +0xBFC | ChronoMinimumDelay | int | ? | Floor for warp timer duration |
| +0xC00 | ChronoRangeMinimum | int | ? | Force minimum delay below this distance |

**String addresses verified:**
- 0x83C714 = "ChronoDelay"
- 0x83C700 = "ChronoReinfDelay"
- 0x83C6E8 = "ChronoDistanceFactor"
- 0x83C6D8 = "ChronoTrigger"
- 0x83C6C4 = "ChronoMinimumDelay"
- 0x83C6B0 = "ChronoRangeMinimum"

**Confidence: 99%** -- Directly decoded from binary instruction bytes.

---

## 12. CellClass Key Offsets Used by Teleport

| Offset | Type | Field | Used In |
|--------|------|-------|---------|
| +0xE4 | ptr | FirstObject | Ground-level object list |
| +0xE8 | ptr | BridgeOccupants | Objects on bridge surface |
| +0xEC | int | **LandType** | 2=Water triggers death |
| +0x124 | byte | OccupationBits (ground) | Infantry sub-cell occupation |
| +0x128 | byte | OccupationBits (bridge) | Infantry sub-cell on bridge |
| +0x140 | int | **Flags** | Bit 0x100=HasBridge, 0x200=BridgeDestroyed |

---

## Summary of New Findings vs Original Report

1. **HeadToCoord guards**: Four distinct guard conditions (warp-out, warp-in, deploy,
   undeploy) prevent concurrent transitions. Not documented in original.

2. **Process infantry path**: Full sub-cell placement logic with 4 mission checks
   (Enter=7, Capture=8, Eaten=9, Patrol=25) for dock-at-destination validation.

3. **Process alternate cell search**: When infantry placement fails, searches adjacent
   cells via NavTarget's Pathfinding_validate_alternate. Preserves sub-cell offset.

4. **Update_Position damage rules**: Complete object iteration with distinct damage
   paths for flying units (teleporter takes damage), infantry vs infantry (other dies),
   and general techno (other dies).

5. **PostWarpValidation exception chain**: Four exceptions before water death:
   Chronoshiftable, Infantry, HasBridge, LandType==Road. All verified.

6. **FUN_006B0AE0 kill credit**: Full passenger release logic with house lookup for
   kill credit assignment. Credits kills to ChronoSphere owner's house.

7. **Mark_All_Occupation_Bits**: Actually a RateTimer::Set wrapper on FootClass+0x388
   (facing timer), not cell occupation. Name is misleading.

8. **FUN_0070F770**: Random 4-8 frame idle delay timer at FootClass+0x180.

9. **FUN_00709480**: Scatter/resume check that attempts to give the unit a new order
   after chrono timer expires. Calls FUN_00709290 for eligibility.

10. **TechnoClass+0x2B4**: Likely a queued garrison/action target. When zero, unit is
    truly idle after warp.
