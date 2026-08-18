# UnitClass::PerCellProcess (0x739EC0) — Full Refinery Docking Choreography

**Date:** 2026-04-03 (post-2026-05-19 label/CLSID corrections applied)
**Binary:** gamemd.exe
**Confidence:** HIGH (verified from full 534-line decompilation)
**Active in YR:** YES — core gameplay mechanic

> **Label-drift note (2026-05-19 audit):** Ghidra's labeler now reports this function as
> `UnitClass__PerCellProcess` (vtable slot +0x18C per-cell hook), not `UnitClass__Mission_Enter`
> as earlier exports labelled it. The function body — refinery dock choreography — is
> unchanged; only the identity label drifted. `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md`
> independently documents the mislabel. Verified via `get_function_by_address 0x00739EC0`.
> All references to "Mission_Enter" in this doc below refer to *this function*, which is
> the per-cell handler whose body happens to drive refinery dock state. The actual
> `FootClass::Mission_Enter` (mission code 7) dispatch lives in the FootClass mission table
> at vtable slot +0x240 (FootClass's 0x004D9290), not here.
>
> **Correction 2026-05-24 - current stock refinery doc status**
>
> This report is historical and should not be used as the current mission-7
> timing or `0x15` source model. Current stock docking splits actual
> `FootClass::Mission_Enter @ 0x004D9290` retry timing from
> `UnitClass::PerCellProcess @ 0x00739EC0` cell-entry hooks. The accepted
> refinery move target is `NW+(3,1)`, `GetDockCoord` is `NW+(2,1)`, and
> `0x16` can send `0x15` without requiring PerCellProcess `GetDockCoord`
> equality. See
> `STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`.

---

## 1. Function Overview

**Address:** 0x739EC0
**Size:** 534 decompiled lines, body 0x739EC0–0x73B0AE
**param_1 type:** `TechnoClass*` (int — offsets are direct byte offsets)
**State variable:** `unaff_retaddr` — this is the mission state parameter, passed on the stack. Values observed: 0, 2.

Mission_Enter is the handler for Mission 7 (Enter). It manages entering ANY building: refineries, grinders, transports, repair pads, bunkers, hospitals, armories, and helipads. This document focuses on the **refinery path**.

### High-level flow:
```
State 0/initial → navigate toward destination building
State 2         → at/near building, attempt dock or enter
```

---

## 2. Entry and Deploy Check (lines 1–48)

```c
// Get the unit's current cell
cell = vtable->GetMapCell();  // vtable+0x1B8

// If state is 0 or 2, and unit is a simple deployer, try to deploy
if ((state == 2 || state == 0) && this->IsSimpleDeployer) {
    UnitClass__Deploy();
    if (!this->IsAlive) return;  // deployed and destroyed self
}
```

The `IsSimpleDeployer` flag is at instance offset 0x16c (relative to param_1[1]).

---

## 3. State 2: The Main Docking Logic

### 3.1 How the unit identifies what it's entering

The function does NOT check a single "building type" enum. Instead, it reads boolean flags on `BuildingTypeClass`:

| Flag Offset | INI Key | Purpose |
|-------------|---------|---------|
| `+0x16A9` | `UnitRepair` | Repair pad |
| `+0x16AA` | `UnitReload` | Reload pad |
| `+0x16AB` | `Bunker` | Bunker (garrison) |
| `+0x16AC` | `Cloning` | Cloning vat |
| `+0x16AD` | `Grinding` | Grinder |
| `+0x16AE` | `UnitAbsorb` | Bio-reactor (units) |
| `+0x16AF` | `InfantryAbsorb` | Bio-reactor (infantry) |
| `+0x16B3` | `DockUnload` | Refinery dock |
| `+0x16BB` | `Refinery` | Is refinery |
| `+0x16BC` | `Weeder` | Weed refinery |
| `+0x16BD` | `WeaponsFactory` | War factory |
| `+0x16C1` | `Hospital` | Hospital |
| `+0x16C2` | `Armory` | Armory |
| `+0x16CB` | `Helipad` | Helipad |

The checks are performed in this priority order:

1. **Grinder** (RTTI == 9, checked via vtable+0x184): immediate destruction
2. **Transport/UnitAbsorb** (Type+0x16AE): infantry/unit absorb enter
3. **Refinery approach** (locomotion piggybacking + WeaponsFactory flag)
4. **Harvester-specific behavior** (field_0x418 = Harvester flag)
5. **Transport enter** (same-cell overlap, radio 0xF check)
6. **Generic enter** for remaining types

### 3.2 Grinder Path (lines 62–143)

When the building RTTI is 9 (Grinder) AND the building matches the unit's dock link or target:

```c
// Play enter sounds (VocClass at Type+0x4CC, Type+0x520)
// Get sell value
credits = vtable->GetSellValue();  // vtable+0x2BC (offset 700)
HouseClass__Add_Credits(credits);

// Recursively sell all passengers
while (this->field_0x118 != 0) {  // passenger list
    passenger = FUN_004de710();  // get first passenger
    // Also sell passengers of passengers
    while (passenger->field_0x118 != 0) {
        inner = FUN_004de710();
        credits = inner->GetSellValue();
        HouseClass__Add_Credits(credits);
        inner->UnInit();  // vtable+0xF8
    }
    credits = passenger->GetSellValue();
    HouseClass__Add_Credits(credits);
    passenger->UnInit();
}

// Handle temporal/chrono weapon attachment
if (this->TemporalTarget != 0) {
    credits = temporal->GetSellValue();
    HouseClass__Add_Credits(credits);
    WarpAttachClass__Detach();
}

// Kill the unit
if (this->Type->Grinding) {  // Type+0x16AD
    VocClass__PlayAt(0);  // play grind sound
    if (building->field_0x568 != 0) {
        BuildingClass__ClearAnimSlot();
        BuildingClass__SetAnimSlotImage(10, ...);  // grinding animation
    }
}
this->UnInit();  // vtable+0xF8
return;
```

### 3.3 Transport/UnitAbsorb Enter (lines 145–172)

When the building has `UnitAbsorb=yes` (Type+0x16AE) and RTTI == 7:

```c
if (radio(0xF, building) == 1) {  // CAN_ENTER accepted
    SetGhostCell(0);              // remove from cell occupancy
    this->OnBridge = false;
    this->field_0xC4 = 0;
    FUN_0070de00(0);              // clear locomotion link
    FUN_0070ddd0(0);              // clear locomotion link
    if (this->MindControlledBy != NULL && this->MindControlledBy->CaptureManager != 0) {
        CaptureManagerClass__FreeUnit();
    }
    vtable->Disappear();          // vtable+0xD4 — remove from display
    CargoClass__AddPassenger();    // add to building cargo
    vtable->ClearMission();       // vtable+0x11C
    if (building->Type->SizeLimit > 0) {  // Type+0xEE8
        building->Owner->field_0x5778 = 1;  // mark sidebar dirty
    }
    return;
}
```

---

## 4. The Refinery Approach Path (lines 175–225)

This is the critical harvester-to-refinery docking code.

### 4.1 Locomotion Check and IPiggyback

**Trigger condition:** RTTI of destination == 6 (Building) AND destination is non-null AND the unit's current mission is 7 (Enter) or 0x19 (25).

```c
// Get unit's current coords (cell-aligned center)
unitCoords = vtable->GetCoords();
unitCell = Leptons_to_Cell(unitCoords);

// Get dock coordinates from building
dockCoords = building->vtable->GetDockCoord(&result, this);  // vtable+0xA8
dockCell = Leptons_to_Cell(dockCoords);

// Are we AT the dock cell?
if (unitCell == dockCell) {
    // Query locomotion for IPiggyback interface
    ILocomotion* loco = this->Locomotion;  // offset +0x674 (param_1[1].field_0x154)
    IPiggyback* piggy = NULL;

    if (loco == NULL) {
        Assert(E_POINTER);
    } else {
        HRESULT hr = loco->QueryInterface(IID_IPiggyback, &piggy);
        // IID_IPiggyback = {0000010C-0000-0000-C000-000000000046}
        if (FAILED(hr)) {
            piggy = NULL;
        }
    }

    // Get the CLSID of the piggybacked (inner) locomotion
    CLSID innerCLSID;
    piggy->GetClassID(&innerCLSID);  // IPiggyback vtable+0xC

    // Check if inner locomotion is WalkLocomotion
    if (innerCLSID == CLSID_WalkLocomotion) {
        // CLSID_WalkLocomotion = {4A582742-9839-11D1-B709-00A024DDAFD1}
        // CLSID_DriveLocomotion = {4A582741-9839-11D1-B709-00A024DDAFD1}

        if (building->Type->UnitRepair      // Type+0x16A9 — verified via decompile_function 0x00739EC0
            && this->DockLink == 0) {       // unit+0x5A4 (`param_1[1].field_0x84` = base 0x520 + 0x84)
            this->DockLink = building;      // set dock link
        }
    }

    // If this building IS our dock link
    if (building == this->DockLink) {
        FUN_004d85d0(2);  // state transition — clears approach state, updates ghost cell
        radio(0x15, building);  // vtable+0x274 — DOCK_NOW

        // Power off locomotion (stop moving)
        ILocomotion* loco = this->Locomotion;
        loco->Power_Off();  // ILocomotion vtable+0x5C

        // Release IPiggyback reference
        if (piggy != NULL) {
            piggy->Release();
        }
        return;
    }

    // Not our dock — release piggy ref
    if (piggy != NULL) {
        piggy->Release();
    }
}
```

**Key insight about piggybacking:**
- Chrono Miners normally use `TeleportLocomotionClass`
- When they need to physically drive to a refinery, `DriveLocomotionClass` is piggybacked OVER `TeleportLocomotionClass`
- The check here is: "is the inner (piggybacked) locomotion WalkLocomotion?" — this identifies that a DriveLocomotion has been piggybacked
- War Miners (HARV) use DriveLocomotion natively, so the inner is WalkLocomotion by default

### 4.2 The Harvester Above-Dock-Cell Check (lines 226–255)

When the harvester is on the cell just above the dock (cell Y-1):

```c
if (this->field_0x418 != 0  // IsHarvester instance flag
    && destination != NULL
    && destination->WhatAmI() == 6  // Building
    && this->GetMission() == 7) {   // Mission_Enter

    // Check cell at (current_X, current_Y - 1) — one cell above
    cellAbove = CONCAT22(currentCell.Y - 1, currentCell.X);
    MapClass__Get_CellClass(&cellAbove);
    buildingAtAbove = Look_up_building_in_cell();

    if (buildingAtAbove == destination) {
        result = radio(0x15, destination);  // try DOCK_NOW
        if (result != 1 && result != 5) {
            // Not accepted — navigate to dock cell
            vtable->Move_To(&dockCell, 1, 0);  // vtable+0x174
        }
    }
}
```

---

## 5. Queue Cell Navigation (lines 300–440)

This is the main "I'm a harvester approaching a refinery but not yet at the dock" handler.

### 5.1 Entry Conditions

```c
if (this->field_0x418 != 0  // IsHarvester
    && (GetMission() != 7 || (FUN_0040dd70() != 0 && GetDestination() != this->DockLink))
    && GetMission() != 0x10) {  // not Unloading
```

### 5.2 Check if at the dock link building

```c
dockLink = this->DockLink;  // param_1[1].field_0x84
bool atDockLink = (dockLink == NULL);

// Special case: if dock link is RTTI 0xB (11), check if we're on its cell
if (dockLink != NULL && dockLink->WhatAmI() == 0xB) {
    if (dockLink->MapCell == Leptons_to_Cell(this->Location)) {
        atDockLink = true;  // we're on the dock link's cell
    }
}
```

### 5.3 Radio 8 Response Handling

> **CORRECTED 2026-04-19** — earlier version of this section had the harvester /
> non-harvester branch bodies inverted, and labeled `field_0x2D8` as "PlayerOrdered"
> when it is actually `SlaveManager*`. See
> [MISSION_ENTER_REFINERY_DOCK_VERIFICATION_NOTES.md](MISSION_ENTER_REFINERY_DOCK_VERIFICATION_NOTES.md)
> for the verification trail. The pseudocode below reflects the actual binary.

```c
result = radio(8);  // REQUEST_DOCKING_CLEARANCE — vtable+0x274

if (result == 0x17) {  // QUEUED
    // Building says: "wait, I'm busy"

    if (dockLink == NULL || dockLink == GetCellBuilding()) {
        destination = GetDestination();
        if (destination != NULL && destination->WhatAmI() == 6) {
            destBuilding = destination;
        } else {
            destBuilding = NULL;
        }

        // Check if unit is Harvester or Weeder
        bool isHarvester = TypeClass->Harvester;   // TechnoTypeClass+0xE0E
        bool isWeeder    = TypeClass->Weeder;      // TechnoTypeClass+0xE0F

        if (!isHarvester && !isWeeder) {
            // === NON-harvester / non-weeder branch ===
            if (this->SlaveManager == NULL) {     // TechnoClass+0x2D8 (SlaveManagerClass*)
                if (!HouseClass__IsPlayerControl()) {
                    // AI-controlled non-harvester unit: pick wander cell if dest is
                    // a WeaponsFactory queue (e.g., a Rhino tank waiting to be sold/
                    // serviced at a War Factory)
                    if (destBuilding != NULL
                        && destBuilding->Type->WeaponsFactory) {  // BuildingType+0x16BD

                        queueCell = FUN_00500200(&result, this);  // random nearby cell
                        if (queueCell == INVALID_CELL) goto clearGhost;

                        // Navigate to wander cell, then enter Area Guard
                        SetMission(2, 0);                          // Mission_Move
                        cellPtr = MapClass__Get_CellClass(&queueCell);
                        SetDestination(cellPtr, 1);
                        QueueMission();
                        SetGhostCell(cellPtr);
                        SetMission(0xB, 0);                        // Mission_AreaGuard
                    }
                } else {
                    // Player-controlled non-harvester: hold position or use WarpTarget
                    if (field_0x218 == 0 || field_0x218 == dockLink) {  // WarpTarget
                        FootClass__Stop_Moving();
                        Move_To(&invalidCell, 1, 0);
                    } else {
                        SetDestination(field_0x218, 1);
                    }
                }
            } else {
                // Unit IS a Slave Master (e.g., Yuri Prime) — recall slaves first
                SlaveManagerClass__RecallAllSlaves(this->SlaveManager);
            }
        }
        else {
            // === HARVESTER or Weeder branch ===
            // Re-queue Mission_Harvest. The harvester FSM will re-find a refinery
            // (possibly the same one) and re-issue the dock request next tick.
            // NO wander, NO random cell — just poll-in-place via Mission_Harvest.
            SetMission(10, 1);    // Mission_Harvest, queued
        }
    }
    else if (dockLink != 0 && (dockLink->Flags & 1)) {
        // Dock link still valid — send radio 0xE (CAN_DOCK) to refresh
        radio(0xE, dockLink);
    }
}
else if (result != 10) {
    // Radio 8 returned something other than QUEUED or NEGATIVE
    // For harvesters/weeders with a dock link, navigate there
    if (field_0x218 == 0) {
        if (dockLink == 0) {
            Move_To(&invalidCell, 1, 0);
        }
    } else {
        SetMission(10, 0);
        SetDestination(field_0x218, 1);
        SetGhostCell(0);
    }
}
```

### 5.4 Arrival Detection and Abort

```c
// After queue logic: check if we've arrived at a building with no dock link
currentCell = Leptons_to_Cell(this->Location);
buildingHere = Look_up_building_in_cell(currentCell);
if (buildingHere != 0
    && this->DockLink == 0
    && this->Location_X_2 == 0  // param_1[1].Location_X
    && this->field_0x78 == 0) {
    Move_To(&invalidCell, 1, 1);  // abort — scatter
}
```

---

## 6. Radio Command Protocol — Building Side

### 6.1 Radio 0xE (CAN_DOCK) — BuildingClass::Receive_Radio, case 0xE

**Address:** 0x43C2D0, case 0xE

> **Correction 2026-05-21 - standard DockUnload no-slot path**
>
> The standard refinery `DockUnload` case is not guarded by a final
> `InDockQueue(sender)` hard reject. After the power / UnitRepair / Bunker
> gates, `Receive_Radio(0x0E)` can still compute the hardcoded receiver target
> `GetMapCell() + (3,1)`, send `0x12`, and return `1` without sending `0x18` /
> `0x16` if the unit is not already at the target cell. Ordinary occupied or
> no-free-contact DockUnload therefore is not simply `NEGATORY`.
>
> `QueueingCell=4,1` is not read by this receiver path; the target remains the
> hardcoded `NW+(3,1)` cell. Hard `10` here is reserved for the explicit gates
> such as no power or UnitRepair/Bunker rejection, not for the normal occupied
> DockUnload wait path.

**Full sequence:**

```c
case 0xE:
    TechnoClass__Receive_Radio(sender, 0xE, param);  // base call first
    
    // 1. Power check
    if (!this->HasPower) return 10;  // NEGATIVE
    
    // 2. Repair pad check (Type+0x16A9)
    if (Type->UnitRepair && InDockQueue(sender)) {
        if (radio(0x22, sender) == 10) return 10;  // already repairing
    }
    
    // 3. Bunker check (Type+0x16AB)
    if (Type->Bunker && !CanAutoDeployHere(sender)) return 10;
    
    // 4. For non-helipad, non-hospital buildings:
    if (!Type->Hospital && !Type->Armory) {
        if (!InDockQueue(sender) && HasFreeSlot()) {
            radio(2, sender);  // DOCK_LINK — establish link
        }
        
        if (InDockQueue(sender)) {
            // For refineries (DockUnload or Weeder):
            if (Type->DockUnload || Type->Weeder) {
                // Calculate QUEUE CELL
                cell = vtable->GetMapCell();  // building's top-left cell
                queueCell = (cell.X + 3, cell.Y + 1);  // offset (+3, +1) from top-left
                
                cellPtr = MapClass__Get_CellClass(&queueCell);
                *param_out = cellPtr;  // return queue cell to caller
                
                // Validate and signal
                result = radio(0x12, param_out, sender);  // tell unit to go to this cell
                if (result != 0x14) return 1;
                
                radio(0x18, sender);    // begin dock sequence
                result = radio(0x16, sender);  // timing sync
                if (result == 1) return 1;
                
                // If timing not ready, tell unit to scatter
                sender->Move_To(&invalidCell, 1, 1);
                return 1;
            }
            
            // For helipad:
            if (Type->Helipad) {
                *param_out = this;
                result = radio(0x12, param_out, sender);
                if (result != 0x14) return 1;
                radio(0x18);
                return 1;
            }
        }
        
        // Check if we can accept through dock iteration
        // (evicts non-viable units from dock queue)
        for (i = 0; i < dockCount; i++) {
            unit = GetDockedUnit(i);
            if (radio(0x22, unit) == 10) {
                radio(0x17);  // KICK from queue
            }
        }
        
        if (HasFreeSlot()) return 1;
        return 10;  // NEGATIVE — full
    }
    
    // For Hospital/Armory: direct cell-based docking
    cellPtr = CellClass__Get_Cell_At();
    *param_out = cellPtr;
    radio(0x12, param_out, sender);
    return 1;
```

### 6.2 Queue Cell Calculation

**The magic numbers `(+3, +1)`**: For a standard refinery (3x3 foundation at `artmd.ini`), the queue cell is:
```
building top-left cell = (X, Y)
queue cell = (X + 3, Y + 1)
```

This places the queue cell one cell to the right of the building's right edge, at the middle row. For the standard GAREFN/YAREFN with 3x3 foundation, this is the cell to the right of the refinery entrance.

Note: This is the HARDCODED queue cell for DockUnload/Weeder buildings. The `QueueingCell` INI key from artmd.ini is read at BuildingTypeClass offset but this hardcoded `(+3, +1)` is what's actually used in the radio 0xE handler.

### 6.3 Radio 0xF (CAN_ENTER) — BuildingClass handler

**Address:** 0x43C2D0, case 0xF

```c
case 0xF:
    TechnoClass__Receive_Radio(sender, 0xF, param);
    
    // Alliance check
    if (!HouseClass__Is_Ally(sender)) return 0;  // NO
    
    // Building status checks
    if (GetMission() == 0x12) return 10;  // Selling
    if (GetMission() == 0x13) return 10;  // Under Construction
    if (this->field_0x534 == 0) return 10;  // no cargo capacity
    
    // Map editor always allows
    if (g_MapEditorMode == 0 && !HasFreeSlot()) {
        // Must be a dockable type to proceed
        if (!Type->UnitAbsorb && !Type->InfantryAbsorb) return 10;
    }
    
    // Size check: sender size vs building capacity
    senderSize = sender->TypeClass->SizeWeight;  // +0x380 (double)
    buildingCap = this->TypeClass->SizeLimit;     // +0x5E0 (int)
    buildingMaxWeight = this->TypeClass->MaxWeight; // +0x388 (double)
    
    // UnitAbsorb/InfantryAbsorb path
    if (Type->UnitAbsorb || Type->InfantryAbsorb) {
        senderRTTI = sender->WhatAmI();
        if (senderRTTI == 1 && !Type->UnitAbsorb) return 10;  // unit but no UnitAbsorb
        if (senderRTTI == 0xF && !Type->InfantryAbsorb) return 10;
        
        // Mind control check
        if (sender->CaptureManager != NULL && FUN_004722c0()) return 10;
        
        // Capacity check
        if (this->PassengerCount + 1 <= buildingCap
            && senderSize <= buildingMaxWeight) {
            return 1;  // ACCEPTED
        }
    }
    
    // Grinder (Type+0x16AD)
    if (Type->Grinding) return 1;  // always accepts
    
    // Bunker (Type+0x16AB)
    if (Type->Bunker) {
        if (!CanAutoDeployHere(sender)) return 10;
        if (radio(0x23, sender) == 1) return 10;  // already occupied
        return 1;
    }
    
    // Repair pad (Type+0x16A9): unit must be RTTI 1 or 2
    if (Type->UnitRepair) {
        senderRTTI = sender->WhatAmI();
        if (senderRTTI != 1 && senderRTTI != 2) return 10;
        if (radio(0x23, sender) == 1) return 10;
        return 1;
    }
    
    // Helipad: unit must be RTTI 0xF (Aircraft)
    if ((Type->Hospital || Type->Armory) && sender->WhatAmI() == 0xF) {
        // Mind control check
        if (sender->CaptureManager != NULL && FUN_004722c0()) return 10;
        // Docking unit capacity check
        if (this->field_0x2FC != 0) return (this->field_0x2FC != 0) ? 10 : 3;
        return 10;
    }
    
    // Helipad check (Type+0x16CB)
    if (Type->Helipad) {
        return (sender->WhatAmI() == 2) ? 1 : 10;
    }
    
    // DockUnload/Refinery (Type+0x16B3): Harvester=yes required
    if (Type->DockUnload && sender->WhatAmI() == 1) {
        if (sender->TypeClass->Harvester) {  // +0xE0E
            if (g_MapEditorMode != 0) return 1;
            if (this->field_0x118 == 0) return 1;  // dock free
        }
    }
    
    // Weeder refinery (Type+0x16BC)
    if (Type->Weeder && sender->WhatAmI() == 1) {
        if (sender->TypeClass->Weeder) {  // +0xE0F
            if (g_MapEditorMode != 0) return 1;
            if (this->field_0x118 == 0) return 1;
        }
    }
    
    return 0;  // REJECTED
```

### 6.4 Radio 0x15 (DOCK_NOW) — BuildingClass handler

**Address:** 0x43C2D0, case 0x15

```c
case 0x15:
    if (GetMission() == 0x13) return 10;  // under construction
    
    // UnitAbsorb or InfantryAbsorb
    if (Type->UnitAbsorb) return 1;
    if (Type->InfantryAbsorb) return 1;
    
    // Repair pad, UnitReload, Hospital, Armory
    if (Type->UnitRepair || Type->UnitReload || Type->Hospital || Type->Armory) {
        this->field_0x6DD = 1;  // dock active flag
        SetMission(0x14, 0);    // Mission 0x14 = MissionRepairAndProduce
        sender->SetMission(0, 0);  // set unit to Mission_Stand
        return 1;
    }
    
    // Bunker
    if (Type->Bunker) {
        this->field_0x6DD = 1;
        SetMission(0x14, 0);
        return 1;
    }
    
    // DockUnload (Refinery)
    if (Type->DockUnload) {
        sender->SetMission(0x10, 0);  // Mission 0x10 = Mission_Unload (16)
        return 1;
    }
    
    // Fall through to base TechnoClass handler
```

**Critical finding:** When a refinery (DockUnload) receives DOCK_NOW, it sets the **unit's** mission to 0x10 (Unload), NOT the building's mission.

**Correction 2026-07-18:** The sentence "The building transitions to MissionRepairAndProduce separately when it receives the dock notification" is WRONG for the `DockUnload` branch. Live decompile of `case 0x15` at 0x0043C2D0 shows `field_0x6dd = 1` and `SetMission(0x14, 0)` (MissionRepairAndProduce) are emitted ONLY by the `UnitRepair (+0x16a9) / UnitReload (+0x16aa) / Hospital (+0x16c1) / Armory (+0x16c2)` branch and, separately, the `Bunker (+0x16ab)` branch — both of which `return` before the `DockUnload (+0x16b3)` branch is ever reached. The `DockUnload` branch body is exactly `sender->SetMission(0x10,0); return 1;` — no building-side mission change at all. Verified via `decompile_function 0x0043C2D0`. What actually drives the per-tick ore-dump loop for stock `DockUnload` buildings is UNVERIFIED in this session — see the historical-status banner at the top of this doc and `STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`. — INFERENCE_HARDENED.

### 6.5 Radio 0x15 (DOCK_NOW) — UnitClass handler

**Address:** 0x737430, case 0x15

```c
case 0x15:
    // Calculate dock distance
    int currentPassengers = this->TypeClass->SizeLimit;  // +0x5E0
    int actualPassengers = FUN_00473460();  // count passengers in cargo
    
    if (actualPassengers == currentPassengers) {
        // Full — set the dock cell animation timer
        FUN_004a5240(
            TypeClass->field_0x3C8,  // dock animation offset X
            TypeClass->field_0x3CC   // dock animation offset Y
        );
    }
    return 5;  // ALREADY_DOCKED response
```

### 6.6 Radio 8 (REQUEST_DOCKING_CLEARANCE) — BuildingClass handler

**Address:** 0x43C2D0, case 8

```c
case 8:
    // Distance check for repair/bunker buildings
    if (Type->UnitRepair || Type->Bunker) {
        distance = GetDistance(sender);
        if (distance < 0x180) {  // 384 leptons = 1.5 cells
            return 1;  // PROCEED — close enough
        }
    }
    
    TechnoClass__Receive_Radio(sender, 8, param);
    
    // For WeaponsFactory, UnitRepair, or Bunker: return QUEUED
    if (Type->WeaponsFactory || Type->UnitRepair || Type->Bunker) {
        return 0x17;  // QUEUED
    }
    
    return 1;
```

**Important (corrected 2026-07-18):** The gate is `WeaponsFactory (+0x16bd) || UnitRepair (+0x16a9) || Bunker (+0x16ab)` — refineries are NOT included by virtue of being refineries. Stock `GAREFN`/`NAREFN`/`YAREFN` in `ini/rulesmd.ini` set `DockUnload=yes` and `Refinery=yes` but do **not** set `WeaponsFactory=yes`; for these buildings the case-8 handler falls through to `return 1` (PROCEED), not 0x17 (QUEUED). The prior claim conflated "refinery" with the `WeaponsFactory` flag. Verified via `decompile_function 0x0043C2D0` (case 8: `if (Type[0x16bd]==0 && Type[0x16a9]==0 && Type[0x16ab]==0) return 1; return 0x17;`) plus `grep '^\[GAREFN\]' ini/rulesmd.ini` (no `WeaponsFactory=` key present). Only true war factories (which set `WeaponsFactory=yes`) plus repair pads and bunkers always get QUEUED; stock refineries do not. — INFERENCE_HARDENED. See `docs/research/AUDIT_LOG.md` 2026-07-10 RED entry.

---

## 7. The Physical "Enter Pad" Moment

### 7.1 What happens when radio 0x15 is accepted

From Mission_Enter (section 4.1 above):

1. Unit sends radio 0x15 (DOCK_NOW) to the building
2. **(corrected 2026-07-18)** Building receives it. For `UnitRepair`/`UnitReload`/`Hospital`/`Armory`/`Bunker` buildings ONLY, it sets `field_0x6DD = 1` and enters Mission 0x14 (MissionRepairAndProduce). For `DockUnload` (refinery) buildings, this step does NOT happen — the branch order in `case 0x15` (0x0043C2D0) returns before reaching the DockUnload check. Verified via `decompile_function 0x0043C2D0`. — INFERENCE_HARDENED
3. Building sets unit's mission to 0x10 (Unload) for refineries, or 0 (Stand) for repair pads
4. Unit calls `FUN_004d85d0(2)` — this is the dock state transition
5. Unit's locomotion is powered off via `ILocomotion::Power_Off()`

### 7.2 FootClass::PerCellProcess (0x4D85D0) — per-cell handler (used here for dock state cleanup)

> Originally titled "Dock State Transition" — that name described only this function's
> behaviour from the dock-callsite. The actual identity is `FootClass__PerCellProcess`
> (per-cell-crossing hook, vtable slot +0x18C). Verified via `get_function_by_address 0x004D85D0`
> returning `FootClass__PerCellProcess`. This dock-callsite happens to call it with `state=2`
> to clear the approach state and update the ghost cell as part of dock arrival.

**param_2 = 2:** "Entering dock" mode

```c
void FUN_004d85d0(FootClass* this, int state) {
    if (state == 2) {
        this->field_0x6B2 = 0;
        this->field_0x6B0 = 0;  // clear pathfinding flags
        
        // Update turret facing if vehicle has turret
        if (TypeClass->TurretCount > 0) {
            vtable->SetDesiredFacing(this->field_0x55C);
            vtable->SetCurrentFacing(this->body_facing);
        }
        
        // Update ghost cell (queue/dock occupancy tracking)
        if (this->GhostCell != (0,0)) {
            oldCell = GetMapCell();
            if (oldCell != GhostCell) {
                FUN_0070f6a0(GhostCell);     // remove from old ghost cell
                FUN_0070f670(GetMapCell());   // add to current cell
            }
            // Update adjacent cell counters
            for (dir = 0; dir < 8; dir++) {
                adjCell = GhostCell + DirectionOffset[dir];
                adjCell->CrowdCounter--;
            }
            GhostCell = GetMapCell();
            for (dir = 0; dir < 8; dir++) {
                adjCell = GhostCell + DirectionOffset[dir];
                adjCell->CrowdCounter++;
            }
        }
        
        // Scatter from crushable threats
        // ... (danger zone checking code)
    }
}
```

### 7.3 The unit does NOT disappear immediately

The unit stays visible on the dock pad. It does NOT call `Disappear()` or `UnInit()` at this point. The unit remains a visible game object on the dock cell while the refinery processes the unload.

For the **grinder**, the unit IS immediately destroyed (`UnInit()` called).
For **transport enter** (UnitAbsorb), the unit IS immediately hidden (`Disappear()` + `AddPassenger()`).
For **refineries**, the unit stays visible and transitions to the "unloading" visual using `UnloadingClass`.

---

## 8. Multi-Harvester Queue Handling

### 8.1 Queue Mechanics

The building maintains a dock queue via `DynamicVectorClass` at offsets `0xE4`/`0xE8` (array pointer / count).

**When a second harvester arrives:**

1. It sends radio 0xE (CAN_DOCK) to the building
2. Building checks if there's a free slot in the dock queue (via `FUN_0065adf0`)
3. If a slot is available: radio 0x2 (DOCK_LINK) establishes the link, building returns 1 with queue cell
4. If NO slot: returns 10 (NEGATIVE)
5. The harvester navigates to the **queue cell** (X+3, Y+1 from building top-left)
6. It waits there

### 8.2 Queue cell vs dock cell

- **Queue cell:** Hardcoded offset (+3, +1) from building top-left cell, calculated in radio 0xE — CONFIRMED 2026-07-18 via `decompile_function 0x0043C2D0` case 0xE: `CONCAT22(GetMapCell().Y + 1, GetMapCell().X + 3)`.
- **Dock cell:** From `BuildingTypeClass::DockingOffset` array (art.ini), converted to cells via `GetDockCoord` (vtable+0xA8) and `GetDockCellForObject` (0x44EFB0) — **corrected 2026-07-18: WRONG for stock Refinery-flagged buildings.** `BuildingClass::GetDockCoord` (0x00447B20) is an if/else-if chain gated on `Weeder (+0x16bc)` first, then `Refinery (+0x16bb)` second; only buildings that are neither Weeder nor Refinery (and pass further Bunker/Helipad/UnitRepair checks) reach the `DockingOffset` array logic. Stock `GAREFN`/`NAREFN`/`YAREFN` set `Refinery=yes`, so they short-circuit at the second branch — which calls `FUN_005F6C80` (itself just `GetCoords()`, vtable+0x48) and returns `X+0x80` on the building's own coordinate, never touching `DockingOffset`. Verified via `decompile_function 0x00447B20` and `decompile_function 0x005F6C80`. The exact resulting dock-cell formula for stock refineries is NOT re-derived in this pass (semantics of the `X+0x80`/no-Y-offset result need confirmation against a concrete cell fixture) — flagged UNVERIFIABLE, do not treat the DockingOffset-array claim as accurate for refineries.

The queue cell is where harvesters wait. The dock cell is where they physically unload.

### 8.3 Notification flow when dock becomes free

> **Correction 2026-07-18:** Steps 1-2 below assume `DockUnload` buildings run `MissionRepairAndProduce` and undock via `BuildingClass::UndockUnit`. Both premises are WRONG for the stock refinery path — see the §6.4 and §11 corrections. `get_function_callers(0x004593A0)` (`BuildingClass::UndockUnit`) returns exactly three callers: `BuildingClass__ReceiveDamage` (0x00442230), `BuildingClass__Sell` (0x00449c30), and `TemporalClass__Update` (0x0071a760) — none of which is a normal per-tick dump-completion path. `UndockUnit` is a destroy/sell/chrono-vortex eviction handler, not the routine unload-complete handoff. The actual routine completion mechanism for `DockUnload` is UNVERIFIED this session — INFERENCE_HARDENED.

1. Building finishes unloading (MissionRepairAndProduce state 1 → state 5/Guard) — **applies to UnitRepair/Bunker paths, not confirmed for DockUnload**
2. Building sends radio 7 (DOCKING_COMPLETE) to the docked unit via `UnitClass::Receive_Radio` case 7 — **not confirmed for DockUnload; not reached via UndockUnit on the normal path**
3. The undocked unit clears its destination and enters Guard mission
4. The building then sends radio 0x13 (IS_UNIT_LINKED) to the next queued harvester
5. If confirmed (return 1), sends radio 0x1C (REPAIR/DUMP_READY)
6. The waiting harvester gets kicked out of the queue and re-enters the approach sequence

### 8.4 Radio 0x17 (EVICT_FROM_QUEUE) — UnitClass handler

**Address:** 0x737430, case 0x17

```c
case 0x17:
    // Only for harvesters/weeders
    if ((TypeClass->Harvester || TypeClass->Weeder) && this->field_0x6D1 != 0) {
        this->field_0x6D1 = 0;  // clear "queued" flag
        Move_To(&invalidCell, 1, 0);  // scatter / find new refinery
        SetMission(10, 0);  // Mission_Harvest
        if (CanQueueMission()) QueueMission();
    }
```

---

## 9. Locomotor Piggyback During Approach

### 9.1 When Drive gets piggybacked over Teleport

For Chrono Miners (CMIN), the piggyback happens BEFORE Mission_Enter. In `Mission_Harvest` state 3 or `FootClass::Find_Nearest_Dock`, when the chrono miner needs to physically drive to a refinery:

1. The code queries the locomotion for `IID_IPiggyback`
2. Creates a new `DriveLocomotionClass` instance
3. Calls `IPiggyback::Piggyback(newDriveLoco)` to layer Drive on top of Teleport
4. The unit now moves using Drive (ground pathfinding) while Teleport is suspended

### 9.2 When it swaps back after undocking

After `BuildingClass::UndockUnit` (0x4593A0) ejects the harvester:

1. The building calls `radio(3)` (OVER_AND_OUT) to break the radio link
2. The unit returns to `Mission_Guard_Harvester` → `Mission_Harvest`
3. When `Mission_Harvest` begins, it detects the chrono miner flag and removes the Drive piggyback
4. The Teleport locomotion resumes as primary

### 9.3 COM Interface Details

```
IID_IPiggyback = {0000010C-0000-0000-C000-000000000046}

IPiggyback vtable:
  +0x00: QueryInterface
  +0x04: AddRef
  +0x08: Release
  +0x0C: GetClassID (of piggybacked locomotion)
  ...

CLSID_DriveLocomotion    = {4A582741-9839-11D1-B709-00A024DDAFD1}  // 0x7E9A30
CLSID_WalkLocomotion     = {4A582742-9839-11D1-B709-00A024DDAFD1}  // 0x7E9A40
CLSID_TeleportLocomotion = {4A582747-9839-11d1-B709-00A024DDAFD1}  // 0x7E9A90
```

Note: `CLSID_WalkLocomotion` at `0x7E9A40`, `CLSID_DriveLocomotion` at `0x7E9A30` differ only in
the first byte (0x42 vs 0x41). `CLSID_TeleportLocomotion` at `0x7E9A90` has first byte 0x47
(verified via `read_memory(0x7E9A90, 16)` = `47 27 58 4A ...` → first DWORD = `4A582747`). The
historical mislabel `{4A582790-…}` was an authoring typo; every other doc in the archive cites
the correct `4A582747` form. INI cross-check: stock `rulesmd.ini` Teleport locomotor GUID is
`{4A582747-9839-11d1-B709-00A024DDAFD1}` (used by Chrono Legionnaire and Chrono Miner).

---

## 10. End-of-Mission Checks (lines 450–534)

After all the dock/enter logic, Mission_Enter performs several cleanup checks:

### 10.1 Speed calculation (idle scan)
```c
if (dockLink == 0 && field_0x1C0 == -1 && !TypeClass->IsPassive) {
    speed = vtable->GetCurrentSpeed(1);  // vtable+0x318
    this->field_0x2EC = g_CurrentFrameCounter;
    this->field_0x2F0 = speed_param;
    this->field_0x2F4 = speed / 4;
}
```

### 10.2 Stuck detection — create explosion
```c
// If locomotion is NOT moving, and cell is impassable, and not on bridge, not sinking
if (!locomotion->IsMoving()
    && CanEnterCell(currentCell, ...) == 7  // impassable
    && !(this->OnBridge && cell has bridge)
    && !this->IsSinking) {
    
    if (GetMission() != 7) {  // not currently in Enter mission
        // Create explosion at unit position
        anim = new AnimClass(Warhead__SelectExplosionAnim(...), position, ...);
        // Apply damage
        vtable->ReceiveDamage(0, RulesClass+0xFA8, ...);  // self-destruct
        return;
    }
    Move_To(&invalidCell, 1, 1);  // scatter
}
```

### 10.3 Ore overlay destruction on dock cell
```c
// At the very end, when the unit is on the dock cell:
if ((TypeClass->field_0xD28 != 0 || HasWeaponAbility(0x11))
    && cell->OverlayType != -1) {
    
    overlayType = OverlayTypeClass[cell->OverlayType];
    if (overlayType->IsOre || (overlayType->IsWeed && TypeClass->SpeedType == 0xC)) {
        VocClass__PlayAt(0);  // play squish sound
        CellClass__DestroyOverlay(-1);  // remove ore overlay
        this->RockingForwardsPerFrame += constant;  // visual rock
    }
}
```

This is the "harvester eating the ore on the refinery pad" visual that plays when the harvester arrives.

---

## 11. Summary: Complete Refinery Dock Sequence

```
1. Harvester storage full → Mission_Harvest state 3 → Find_Nearest_Dock
2. Find_Nearest_Dock iterates owner's buildings, finds nearest Refinery
3. Sets destination to refinery, queues Mission_Enter (mission 7)

4. Mission_Enter starts, state 0:
   - Sends radio 8 (REQUEST_DOCKING_CLEARANCE)
   - Building returns 0x17 (QUEUED)
   - Building sends radio 0xE (CAN_DOCK) → returns queue cell (X+3, Y+1)
   - Harvester navigates to queue cell

5. At queue cell:
   - Harvester keeps sending radio 8 each tick
   - When building is free: radio 8 triggers building to send dock coords
   - Building sends radio 0x12 with dock cell location
   - Harvester navigates from queue cell to dock cell

6. At dock cell (state 2):
   - Mission_Enter checks: am I at the dock coords?
   - Queries IPiggyback to verify DriveLocomotion is active
   - Sets DockLink (unit+0x218 = building pointer)
   - Calls FUN_004d85d0(2) — dock state transition
   - Sends radio 0x15 (DOCK_NOW) to building
   - Powers off locomotion

7. Building receives DOCK_NOW:
   - Sets unit's mission to 0x10 (Unload)
   - **(corrected 2026-07-18)** Building does NOT enter Mission 0x14 (MissionRepairAndProduce) for `DockUnload` — that only happens for `UnitRepair`/`UnitReload`/`Hospital`/`Armory`/`Bunker`. Verified via `decompile_function 0x0043C2D0` case 0x15. The "state 0 → state 2 (dumping)" building state machine claim is therefore unconfirmed for the refinery path — INFERENCE_HARDENED.

8. Dump loop — **(corrected 2026-07-18) the "building MissionRepairAndProduce" framing below is unconfirmed for DockUnload per the step-7 correction; left as-is pending reinvestigation, do not treat as verified:**
   - Each HarvesterDumpRate interval: transfers one bail of ore to credits
   - Unit stays visible on pad, renders with UnloadingClass model
   - Checks radio 0x13 (IS_UNIT_LINKED) each tick to verify unit

9. Dump complete — **(corrected 2026-07-18) UNVERIFIED / likely WRONG, see §8.3 correction:**
   - Building sends radio 0x1C (DUMP_COMPLETE)
   - Building transitions to state 1 (undock)
   - ~~Calls BuildingClass::UndockUnit (0x4593A0)~~ — REFUTED: `get_function_callers(0x004593A0)` shows only `ReceiveDamage`/`Sell`/`TemporalClass::Update` call this function; it is not on the routine unload-complete path.
   - Unit receives radio 7 (DOCKING_COMPLETE) — not confirmed for the DockUnload path this session

10. Unit undocked:
    - Clears destination, target, dock link
    - Enters Guard mission → Mission_Guard_Harvester → Mission_Harvest
    - Cycle repeats
```

---

## 12. Key Struct Offsets Summary

### UnitClass instance offsets (from TechnoClass base):
| Offset | Name | Description |
|--------|------|-------------|
| `+0x218` | `WarpTarget` (a.k.a. ghost cell, sometimes called DockLink in older drafts) | Cell pointer used by harvesters to remember partially-harvested ore cell across docking trips |
| `+0x260` | `SightRange` | Vision range for fog updates |
| `+0x2D8` | `SlaveManager*` | `SlaveManagerClass*` — non-NULL on Slave Master units (e.g., Yuri Prime). See `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` line 185. **Earlier draft mislabeled this as "PlayerOrdered" — corrected 2026-04-19.** |
| `+0x2EC` | `LastSpeedFrame` | Frame counter for speed tracking |
| `+0x418` | `IsHarvester` | Instance-level harvester flag |

### FootClass offsets (param_1[1] in Ghidra):
| Relative Offset | Absolute | Name | Description |
|-----------------|----------|------|-------------|
| `+0x84` | unit+0x5A4 | `DockLink` | Building pointer for dock queue (FootClass+0x84 = unit base+0x520 + 0x84 = 0x5A4 absolute; disassembly accesses at `[EBP+0x5A4]` e.g. 0x00739F8E, 0x0073A4E9, 0x0073A954, 0x0073AAAB) |
| `+0x154` | | `Locomotion` | ILocomotion COM interface pointer |
| `+0x1A4` | | `TypeClass` | Pointer to UnitTypeClass |
| `+0x1B1` | | `MoveState` | Movement state byte |

### BuildingTypeClass flag offsets:
| Offset | INI Key | Type |
|--------|---------|------|
| `+0x5E0` | `SizeLimit` | int |
| `+0x388` | `SizeWeight` (max) | double |
| `+0xED4` | `OccupyList` | short* |
| `+0x16A9` | `UnitRepair` | bool |
| `+0x16AA` | `UnitReload` | bool |
| `+0x16AB` | `Bunker` | bool |
| `+0x16AD` | `Grinding` | bool |
| `+0x16AE` | `UnitAbsorb` | bool |
| `+0x16AF` | `InfantryAbsorb` | bool |
| `+0x16B3` | `DockUnload` | bool |
| `+0x16BB` | `Refinery` | bool |
| `+0x16BC` | `Weeder` | bool |
| `+0x16BD` | `WeaponsFactory` | bool |
| `+0x16C1` | `Hospital` | bool |
| `+0x16C2` | `Armory` | bool |
| `+0x16CB` | `Helipad` | bool |
| `+0x16E4` | `GDIBarracks` | bool (dock cell helper) |
| `+0x16E5` | `NODBarracks` | bool (dock cell helper) |
| `+0x16E6` | `YuriBarracks` | bool (dock cell helper) |
| `+0x1780` | `NumberOfDocks` | int |
| `+0x1788` | `DockingOffset[]` | 12 bytes each (x,y,z) |

### UnitTypeClass offsets:
| Offset | INI Key | Type |
|--------|---------|------|
| `+0x6B8` | `UnloadingClass` | UnitTypeClass* |
| `+0xE0E` | `Harvester` | bool |
| `+0xE0F` | `Weeder` | bool |
| `+0xE12` | `DeployToFire` | bool |
| `+0xE13` | `IsSimpleDeployer` | bool |

### BuildingClass instance offsets:
| Offset | Description |
|--------|-------------|
| `+0xBC` | State machine sub-state (0/1/2) |
| `+0xE4` | Dock queue array pointer |
| `+0xE8` | Dock queue count |
| `+0x118` | Docked unit pointer / passenger list |
| `+0x218` | Link to queued unit |
| `+0x534` | Cargo capacity remaining |
| `+0x620` | Dump progress counter |
| `+0x624` | Dump-tick-happened flag |
| `+0x628` | CDTimer start frame |
| `+0x62C` | CDTimer rate |
| `+0x630` | CDTimer step |
| `+0x634` | CDTimer enabled |
| `+0x638` | Dump step amount |
| `+0x6DD` | Dock active flag |

### Radio Command IDs:
| ID | Name | Direction | Purpose |
|----|------|-----------|---------|
| 0x02 | DOCK_LINK | Building→Unit | Establish persistent radio link |
| 0x03 | OVER_AND_OUT | Any→Any | Break radio link |
| 0x07 | DOCKING_COMPLETE | Building→Unit | You're done, exit |
| 0x08 | REQUEST_CLEARANCE | Unit→Building | Can I approach? |
| 0x0B | DOCK_APPROACH | Building→Unit | Set approach heading |
| 0x0C | DOCK_ARRIVED | Unit→Building | I'm at the dock cell |
| 0x0E | CAN_DOCK | Unit→Building | Full dock query + get queue cell |
| 0x0F | CAN_ENTER | Unit→Building | Can I enter as cargo? |
| 0x10 | RESERVE_DOCK | Unit→Building | Reserve a dock slot |
| 0x12 | MOVE_TO_CELL | Building→Unit | Navigate to this cell (via param_4) |
| 0x13 | IS_UNIT_LINKED | Building→Unit | Are you still linked? |
| 0x15 | DOCK_NOW | Unit→Building | I'm at the dock, begin sequence |
| 0x16 | TIMING_SYNC | Building→Unit | Sync timing for dock approach |
| 0x17 | EVICT_QUEUE | Building→Unit | Leave the dock queue |
| 0x18 | ENTER_DOCK | Building→Unit | Set DockedIn flag |
| 0x19 | LEAVE_DOCK | Building→Unit | Clear DockedIn flag |
| 0x1C | REPAIR_TICK | Building→Unit | One repair/dump tick complete |
| 0x22 | IS_REPAIRING | Building→Unit | Check if currently being repaired |
| 0x23 | IS_OCCUPIED | Building→Unit | Check if slot occupied |

### Radio Return Values:
| Value | Meaning |
|-------|---------|
| 0 | NO / REJECTED |
| 1 | YES / ACCEPTED |
| 5 | ALREADY_DOCKED |
| 10 | NEGATIVE / CANNOT |
| 0x0E | ZONE_MISMATCH |
| 0x14 | CELL_ACCEPTED |
| 0x17 | QUEUED |
| 0x20 | INSUFFICIENT_FUNDS |
| 0x21 | REPAIR_COMPLETE |

---

## Verification Audit — 2026-05-11

Audited against live `gamemd.exe` in Ghidra MCP. Confidence per claim noted as HIGH / MEDIUM / LOW. Findings reported as confirmed / corrected / unverifiable.

### Summary

The doc's macro-structure (radio protocol, state-2 dock-cell arrival branch, CLSID_WalkLocomotion piggyback detection, refinery-flag dispatch, 0x300 second-pass distance threshold, harvester re-Queue Mission_Harvest on QUEUED) is verified accurate in the binary at 0x00739EC0. The 2026-04-19 companion corrections (branch inversion in §5.3, SlaveManager at +0x2D8) are confirmed against the live decompile. However, several struct-offset claims in §12 reference fields by **labels that contradict the actual instructions** — most importantly the `DockLink` field, which the assembly accesses at unit+0x5A4, not at the +0x218 "WarpTarget" address the §12 table now leads with. The "1-3 tick jitter return" claim cannot be substantiated for Mission_Enter itself (the jitter return is in Mission_Harvest, not Mission_Enter).

### Per-claim audit

1. **Function address 0x00739EC0 = `UnitClass::Mission_Enter`** — confirmed HIGH. `get_function_by_address` returns `UnitClass__Mission_Enter` at 00739ec0, body 00739ec0–0073b0ae. No "Mission_Harvest" mislabel present.
2. **Companion-doc §5.3 branch correction (harvester branch issues `Set_Mission(10,1)`; non-harvester branch is the one with FUN_00500200 wander)** — confirmed HIGH. Disassembly at 0x0073A9AE–0x0073AAEF shows: `[EAX+0xE0E]` Harvester / `[EAX+0xE0F]` Weeder test, JNZ to 0x0073AAE6 which executes `PUSH 1 / PUSH 0xA / CALL [EDX+0x1E8]` (Set_Mission(10, queued=1)). The non-harvester fall-through at 0x0073A9CA tests `[EBP+0x2D8]` (SlaveManager) and only enters FUN_00500200 wander when SlaveManager==NULL AND `[EAX+0x16BD]` WeaponsFactory flag is set on destBuilding (0x0073A9FF). Bodies match the corrected pseudocode exactly.
3. **State count + state-byte location** — partially corrected MEDIUM. The doc says state values 0 and 2 entering Mission_Enter via `unaff_retaddr` — confirmed: the stack-arg at `[ESP+0x44]` (0x00739ED9) is compared to 2 and 0. However, the doc's §12 entry "BuildingClass +0xBC = State machine sub-state" applies to **BuildingClass**, not Mission_Enter's caller. UnitClass `MissionSubState` is at `+0xBC` too (seen at 0x0073E6D5 in Mission_Harvest: `MOV ECX, [EBP+0xBC]` then jump-table dispatch). State-byte location is HIGH-confidence correct, but the doc never explicitly says where Mission_Enter's state is stored — `unaff_retaddr` is a Ghidra mis-decompilation of a stack parameter; the **actual** state arg arrives via `__stdcall` push, not via `+0xBC`.
4. **Radio 8 (REQUEST_DOCKING_CLEARANCE) call site** — confirmed HIGH. 0x0073A939 `PUSH 0x8 / CALL [EDX+0x274]`. Return value compared to 0x17 at 0x0073A943. Doc characterization correct.
5. **Radio 0xE (CAN_DOCK) refresh call** — confirmed HIGH. 0x0073A97D `PUSH 0xE / PUSH ESI(dockLink) / CALL [EDX+0x278]`. Inside the "dock link still valid" branch (Flags & 1 test at 0x0073A96F). Matches §5.3.
6. **Radio 0x15 (DOCK_NOW) at dock-cell arrival** — confirmed HIGH. 0x0073A503 `PUSH 0x15 / CALL [EDX+0x274]` immediately after `FootClass::PerCellProcess(2)` at 0x0073A4F7–0x0073A4FB. Sequence matches §4.1.
7. **CLSID_WalkLocomotion = 0x7E9A40** — confirmed HIGH. 0x0073A4BC `MOV EDI, 0x7e9a40`; CMPSD against 4-DWORD GUID. Doc text states this address in §9.3.
8. **Locomotion at unit+0x674 (= FootClass+0x154)** — confirmed HIGH. 0x0073A43D `MOV EAX, [EBP+0x674]`, then QueryInterface for IPiggyback (`0x818858`). Doc §12 row "FootClass +0x154 = Locomotion" correct.
9. **DockLink field — §12 table is internally inconsistent** — corrected MEDIUM. Disassembly accesses DockLink exclusively at `[EBP+0x5A4]` (e.g., 0x00739F8E, 0x0073A4E9, 0x0073A954, 0x0073AAAB). The §12 "FootClass +0x84 = DockLink" row claims absolute address "unit+0x218+offset" — that's wrong arithmetic; param_1[1] = base + 0x520, so +0x84 inside = unit+0x5A4. Doc should either fix the "Absolute" column to read `unit+0x5A4` or drop that column entirely. The §12 TechnoClass row also lists `+0x218 = WarpTarget (a.k.a. ghost cell, sometimes called DockLink in older drafts)` — that "sometimes called DockLink" parenthetical is misleading; in this function +0x218 is the WarpTarget fallback, never the dock link.
10. **field_0x218 (WarpTarget/ghost cell) fallback** — confirmed HIGH. 0x0073AAA1 `MOV EAX, [EBP+0x218]`, used as the alternate destination when the unit doesn't have a current DockLink. Companion-doc Q1 resolution correct.
11. **field_0x418 (IsHarvester instance flag)** — confirmed HIGH. 0x0073A558 `MOV AL, byte ptr [EBP+0x418]` gates the harvester-above-dock-cell check and the queue-cell navigation block. §12 row correct.
12. **field_0x2D8 = SlaveManager pointer** — confirmed HIGH. 0x0073A9CA `MOV ECX, [EBP+0x2D8]` immediately followed by `CALL 0x006B0CC0` (`SlaveManagerClass__RecallAllSlaves`). The companion-doc Q3 correction is applied in §12 and matches the binary.
13. **WeaponsFactory flag at BuildingType+0x16BD** — confirmed HIGH. 0x0073A8F0, 0x0073A9FF: `MOV AL, byte ptr [ECX+0x16BD]` against the destination building's TypeClass. §3.1 table correct.
14. **Refinery flag at BuildingType+0x16BB** — confirmed HIGH. 0x0073AD37 `MOV AL, byte ptr [EDX+0x16BB]` in the end-of-mission `radio(3)` notification (refinery proximity check for Harvester=yes units).
15. **Weeder flag at BuildingType+0x16BC** — confirmed HIGH. 0x0073ADB3 `MOV AL, byte ptr [EDX+0x16BC]` mirrors the refinery path for Weeder=yes units.
16. **Harvester=yes flag at TechnoType+0xE0E, Weeder=yes at +0xE0F** — confirmed HIGH. 0x0073A9AE, 0x0073A9BC, and many other call sites. §12 table correct.
17. **Magic number: 0x300 second-pass distance threshold** — confirmed HIGH (in `UnitClass::Mission_Harvest`, not Mission_Enter). 0x0073ECD0 `CMP EAX, 0x300` after computing 3-D distance. Doc references this only in Section 11 prose; the 0x300 lives in Mission_Harvest state 3, not Mission_Enter.
18. **`HarvesterTooFarDistance * 0x100` claim** — unverifiable LOW from this function alone. The 0x300 in Mission_Harvest is a hardcoded literal compared post-FSQRT; it's not visibly scaled from a Rules field in Mission_Harvest's body around 0x0073ECC8–0x0073ECD0. The `* 0x100` cell-to-lepton scaling factor IS present elsewhere (RulesClass+0xD78 at 0x0073EC0E reads a Rules value and SHL by 8 = ×0x100 for a separate distance check). Marking this as MEDIUM: the scaling pattern exists but maps to RulesClass+0xD78/+0xD7C, not to a "HarvesterTooFarDistance" key the doc names without offset.
19. **Mission_Enter return value (1-3 tick jitter)** — corrected HIGH. **Mission_Enter does NOT compute a `Random(0,2) + base` jitter return.** Most return paths use `RET 0x4` without setting EAX, or set EAX=1 (e.g., 0x0073A2A8 region returns the result of a void-call chain). The 1-3 jitter pattern (`Random_Next(0,2) + (cellspeed * factor)`) lives in **`UnitClass::Mission_Harvest` epilogue at 0x0073EF77–0x0073EFA2**, not in Mission_Enter. The doc never explicitly claims Mission_Enter returns a jitter, but the prompt asked about it — verdict: jitter belongs to the Mission_Harvest handler that calls Mission_Enter via Queue_Mission, not to Mission_Enter itself.
    - **2026-05-21 label-scope note:** this audit item refers to `UnitClass::Mission_Enter @ 0x00739EC0` / older per-cell choreography labeling. The mission-table `FootClass::Mission_Enter @ 0x004D9290` spot-check returns `ftol(MissionTimerEntry[mission] + 0x10 * 900.0) + RandomRanged(0,2)` on its epilogue path. Do not generalize the older "no jitter" statement to `0x004D9290`.
20. **Transition to Guard on completion** — confirmed HIGH. 0x0073A7A8 `PUSH 0 / PUSH 5 / CALL [EDX+0x1E8]` sets Mission_Guard (5) on the unit when the dock attempt's radio 0xF returns ACCEPTED for transport-enter or fails at LAB_0073A796. Also 0x0073EF6D `PUSH 0 / PUSH 5 / CALL [EDX+0x1E8]` in Mission_Harvest epilogue. So completion-to-Guard is a real transition in this code; the building does **not** broadcast a `SetMission(Guard)` to the unit — the unit self-sets it.
21. **Caller flow: `UnitClass::Mission_Harvest @ 0x73E5E0` state 3 calls `Queue_Mission(Enter, false)`** — confirmed HIGH. 0x0073EE8F `PUSH 0x7 / CALL [EDX+0x1E8]` ([EDX+0x1E8] = Set_Mission/Queue_Mission vtable slot). The `PUSH 0` immediately before (0x0073EE8D) is the queued=0 flag, so this is `Set_Mission(Enter=7, queued=0)`, not Queue_Mission. **Minor correction:** Mission_Harvest state 3 calls `Set_Mission(7, 0)`, i.e., direct transition with queued=false. Functionally equivalent for state-machine intent (instantly move to Mission_Enter).
22. **State-byte write at instance+0xBC** — confirmed HIGH (UnitClass MissionSubState convention). 0x0073E714 `MOV dword ptr [EBP+0xBC], 0x2` in Mission_Harvest state 0→2 transition. Same byte slot referenced in §12.
23. **Hardcoded queue cell (X+3, Y+1) in BuildingClass radio 0xE** — not re-verified in this audit (covered by companion doc as confirmed against 0x43C2D0 case 0xE).
24. **Function at 0x004D85D0 = `FootClass::PerCellProcess`, not "dock state transition"** — confirmed HIGH per companion doc. Ghidra label is `FootClass__PerCellProcess`. §7.2 still uses the old name; that's a known open item from 2026-04-19.

### Stale labels / outdated references

- **None of the doc's address-to-function mappings are stale.** No pre-2026-04-06 `0x4D9290 = Mission_Harvest` mislabel survives — that address is correctly `FootClass::Mission_Enter` (see companion doc's mission-table). 0x73E5E0 is correctly `UnitClass::Mission_Harvest`.
- **§12 "FootClass +0x84 DockLink — Absolute: unit+0x218+offset"** is wrong arithmetic — should be `unit+0x5A4`. Recommend dropping the "Absolute" column or correcting it; the relative offset is fine.
- **§7.2 still titled "Dock State Transition"** — companion doc flagged this 2026-04-19 as open; remains stale. Should be "FootClass::PerCellProcess (general per-cell handler)".
- **§4.1 introduces `unaff_retaddr` as "the mission state parameter, passed on the stack"** — that's correct in effect but the variable name is a Ghidra decompiler artifact. Live disassembly shows the state comes in via stack at `[ESP+0x44]` on entry. Mostly cosmetic.

### TS-legacy filter

- **`BuildingType+0x16BF = LaserFence`** and **`+0x16C0 = FirestormWall`** — TS Firestorm legacy. Referenced by `FootClass::PerCellProcess` (called at the end of Mission_Enter, 0x0073B0A0). Doc §12 doesn't list these flags, but the function does call into PerCellProcess which runs the laser-fence damage path. Conditional in YR (only active when LaserFence buildings are present on the map). Companion doc Q4 resolution is correct.
- **§6.5 "DOCK_NOW UnitClass handler at 0x737430"** — not re-decompiled in this audit. The doc's pseudocode references "set the dock cell animation timer" using `FUN_004a5240` with TypeClass offsets 0x3C8/0x3CC. These offsets are not visibly used in Mission_Enter itself; can't confirm whether they're TS holdovers vs live YR. Marking unverifiable LOW.
- **§3.2 Grinder path (RTTI==9 destruction with recursive passenger sell + Grinding flag at 0x16AD)** — confirmed live in YR (Grinder is a YR-specific Soviet building, GRIND). Not TS legacy.
- **§3.3 UnitAbsorb/InfantryAbsorb enter (Bio Reactor, Industrial Plant)** — confirmed live in YR. Not TS legacy.
- **The "RTTI == 0xB" branch in §5.2** (dockLink RTTI check for "warp-attach" entity) — unusual but verified at 0x0073A83D `CALL [EDX+0x2C] / CMP EAX, 0xB`. RTTI 0xB is `WarpClass`/Chrono — live in YR (used by Chrono Legionnaire and Chrono Miner teleport). Not TS legacy.

### Doc health verdict

**NEEDS-MINOR-PATCHES.** The doc's core technical content (radio protocol, dispatch logic, state-2 dock-cell arrival, harvester branch behavior post 2026-04-19 correction) is verified accurate against the live binary. The patches needed are cosmetic / nomenclature:

1. Fix §12 FootClass table "Absolute" column for DockLink (should read `unit+0x5A4`, not `unit+0x218+offset`).
2. Rename §7.2 from "Dock State Transition" to "FootClass::PerCellProcess" (already flagged 2026-04-19 as open).
3. Optional: add a one-line note that Mission_Enter itself does not return a Random-jitter delay — the jitter is in Mission_Harvest's epilogue and applies when Set_Mission(Enter) is followed by the next Mission_Harvest tick.
4. Optional: add LaserFence/FirestormWall flags (`+0x16BF`, `+0x16C0`) to the §12 BuildingType table with a "TS Firestorm legacy, conditional in YR" note, since `FootClass::PerCellProcess` (called at the end of Mission_Enter) reads them.

Nothing in this doc blocks downstream Rust Mission_Enter abstraction work.
