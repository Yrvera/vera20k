# Harvester Dock/Unload Sequence — Ghidra Deep Dive

**Date:** 2026-04-03  
**Binary:** gamemd.exe  
**Confidence:** HIGH (verified from binary decompilation)  
**Active in YR:** YES — core gameplay mechanic, fully active

---

## 1. Overview: The Full Dock/Unload Lifecycle

The harvester docking lifecycle involves these major phases:

1. **Harvest completion** — Harvester storage full (or no more ore nearby)
2. **Find refinery** — Radio command 0xE (CAN_DOCK) / search for nearest free refinery
3. **Link to refinery** — Radio command 0x2 (DOCK_LINK)
4. **Approach** — Navigate to the queue/dock cell
5. **Enter dock** — BuildingClass::EnterTransport (0x70FD70) — unit visually "enters"
6. **Unload loop** — BuildingClass::MissionRepairAndProduce handles the dump timer
7. **Undock** — BuildingClass::UndockUnit (0x4593A0) ejects the unit
8. **Resume harvest** — Unit returns to Mission_Harvest state 0

---

## 2. FootClass::Mission_Enter — UnitClass override (0x739EC0)

**Address:** 0x739EC0  
**param_1 type:** `TechnoClass*` (int* — offsets are x4)  
**Total lines:** 534  

This is the main entry mission handler. Key behaviors:

### State 2 (param `unaff_retaddr == 2`): Refinery docking path

When the unit is a harvester arriving at a refinery (RTTI == 6 = Building), it:

1. **Checks power** — If the building requires power (`TypeClass+0x410 != 0`) and the owner house lacks power surplus, the unit cancels and resets.

2. **Looks up the building at current cell** via `Look_up_building_in_cell()`.

3. **Grinder path** (RTTI == 9): If the destination is a Grinder:
   - Plays enter/scrap sounds (VocClass)
   - Calls `GetSellValue()` (vtable+0x2BC) and credits the owner
   - Unloads any passengers (recursively getting their sell values)
   - If the unit has a temporal/chrono attachment, credits that too
   - Destroys the unit via `ObjectClass::UnInit()` (vtable+0xF8)

4. **Transport enter path** (Type+0x16AE = `CanBeOccupied` set):
   - Sends radio 0xF (CAN_ENTER) to the building
   - If accepted (return 1): removes unit from map, clears bridge flag, frees mind control, adds as cargo via `CargoClass::AddPassenger`

5. **Refinery docking path** (IsRefinery check via `Type+0x16BD`):
   - If unit is near destination and locomotion is `DriveLocomotionClass`:
     - Queries piggybacked locomotion via COM interface `IID_IPiggyback`
     - Checks if inner locomotion is `CLSID_WalkLocomotion`
     - If walking + IsRefinery + no dock link yet → sets dock link (`param_1[0x84]` = building)
     - Calls `FUN_004d85d0(2)` — the state transition to docking approach
     - Sends radio 0x15 (DOCK_NOW) to the building
     - Stops the locomotion via ILocomotion::Power_Off (vtable+0x5C)

6. **Ore processing on dock cell**:
   - When at the exact dock cell, checks for ore overlay and destroys it (ore clearing animation plays)
   - This is the "harvester eating ore on the refinery pad" visual

### Queue cell navigation (radio 8 = REQUEST_DOCKING_CLEARANCE)

When the building returns 0x17 (QUEUED), the harvester:
- Retrieves the queue cell via `BuildingClass::GetDockCellForObject` (vtable+0x4D4, called indirectly)
- Sets destination to the queue cell
- Waits there until the refinery sends radio 0x7 (DOCKING_COMPLETE / proceed)

---

## 3. BuildingClass::EnterTransport (0x70FD70)

**Address:** 0x70FD70  
**param_1 type:** `int*` — offsets are x4  
**param_2:** unit pointer (int, raw address)

This is called when the harvester physically enters the refinery pad.

### What it does:

```
if (param_2 != 0) {
    // Verify unit is on the building's cell
    vtable->GetMapCell();  // vtable+0x1BC
    building = Look_up_building_in_cell();
    if (building == param_2) {
        // Link unit and building
        // (param_1 = building/this, param_2 = unit)
        *(unit + 0x1D0) = building;     // UnitClass.DockedBuilding = building
        *(building + 0x1CC) = unit;     // BuildingClass.DockedUnit = unit (param_1[0x73])

        // Mark house as needing sidebar update
        *(building->Owner + 0x5778) = 1;

        // Free any mind-controlled units on the building
        if (building+0x2BC != 0)  // CaptureManager
            CaptureManagerClass::FreeAll();

        // Create docking animation
        anim = AnimClass::Constructor(
            RulesClass+0x31C,   // [General] DockAnim (AnimType ptr)
            unit->GetCoords(),  // at unit position
            0, 1, 0x600, 0, 0  // delay=0, loop=1, rate=0x600
        );
        param_1[0x75] = anim;   // unit+0x1D4 = anim pointer
        if (anim != 0)
            AnimClass::SetOwnerObject(param_1);  // attach anim to unit

        // If unit has a temporal weapon, detach
        if (param_1 != 0 && (param_1.flags & 4) && param_1[0x175] != 0)
            FUN_006ea870(param_1, -1, 1);  // temporal detach
    }
}
```

### Key field offsets (BuildingClass::EnterTransport — param_1=building/this, param_2=unit):
| Offset | Field | Description |
|--------|-------|-------------|
| `building+0x1CC` (param_1[0x73]) | `DockedUnit` | Pointer to the unit currently on the dock pad |
| `unit+0x1D0` | `DockedBuilding` | Pointer to the building the unit is docked at |
| `unit+0x1D4` | `DockAnim` | Pointer to the docking animation instance (set on the unit) |
| `RulesClass+0x31C` | `DockAnim` | AnimType for the dock animation (from [General]) |

### Animation:
- The animation is `[General] DockAnim=` (RulesClass+0x31C)
- It's created at the unit's coordinates with rate 0x600
- Attached to the unit as owner object

---

## 4. Unload/Dump Sequence

### 4a. BuildingClass::MissionRepairAndProduce (0x44B780) — Refinery section

**Address:** 0x44B780  
**param_1 type:** `BuildingClass*` (has named struct fields)

The refinery docking/dumping is handled in the `IsRefinery` (Type+0x16B3) branch.

#### State machine (field_0xBC):

**State 0 (Init):**
- Sets `field_0xBC = 2`
- Resets `field_0x6DD = 0` (dirty flag)
- Resets `field_0x620 = 0` (dump progress counter)
- Starts the dump timer: `field_0x628 = CurrentFrame`, `field_0x634 = 1` (timer step)
- Decrements `field_0x2FC` (rearm counter if applicable)

**State 2 (Dumping):**
- Checks CDTimer (`field_0x628/0x62C/0x630`) for expiry
- If timer expired AND `field_0x634 != 0`:
  - Sets `field_0x624 = 1` (dump-tick-happened flag)
  - Increments dump progress: `field_0x620 += field_0x638` (step amount)
  - Resets timer for next tick
- If dump progress reaches threshold:
  - **Threshold formula:** `HarvesterDumpRate * 900.0 <= field_0x620`
  - Where `HarvesterDumpRate` = `RulesClass + 0x16E8` (double)
  - And 900.0 is at constant address 0x7E27F8
  - **NOTE:** The refinery branch uses offset `0x16E8`, not `0x1528`. Offset 0x1528 is the [General] `HarvesterDumpRate` read from ReadGeneral. These may be the same value copied elsewhere, or 0x16E8 may be a different rate. The 0x1528 is definitively `HarvesterDumpRate`.
- When dump is complete:
  - Sends radio 0x13 (IS_UNIT_LINKED) to confirm unit is still there
  - If unit confirmed (return 1):
    - Sends radio 0x1C (DUMP_COMPLETE)
    - If return 1: sets `field_0xBC = 1` (undock phase)
    - If return 0x20: ejects unit to passable cell
    - If return 0x21: plays EVA "insufficient funds" warning, still sets state 1

**State 1 (Undock/Exit):**
- Checks if locomotion path is complete (`PathType::Has_Valid_Steps() == false`)
- Clears dock/unload animations (slots 8, 11)
- Sets repair/active animations
- If `field_0x58C == 0`: sets mission to Guard (5), marks dirty (`field_0x6DD = 1`)

### 4b. BuildingClass::DepositOreFromStorage (0x522D50)

**Address:** 0x522D50  
**param_1 type:** `int*`

Called during the dump tick to actually transfer ore to credits.

```c
void BuildingClass::DepositOreFromStorage(int* param_1) {
    bool anyDeposited = false;
    int slotIndex = StorageClass::FindFirstNonEmptySlot();
    
    while (slotIndex != -1) {
        int owner = param_1[0x87];  // Owner HouseClass*
        int storageFacilities = *(owner + 0x538C);  // number of refineries
        
        // AI bonus: if not human player and in multiplayer
        if (!IsHumanPlayer(owner) && g_GameMode != 0) {
            storageFacilities += AIDifficultyBonuses[owner->DifficultyIndex];
            // RulesClass+0x1324 = AI bonus storage array
        }
        
        float oreAmount = StorageClass::GetAmount(slotIndex);
        
        // Purifier bonus calculation
        float purifierBonus = (float)storageFacilities 
                            * *(float*)(RulesClass + 0xF3C)  // PurifierBonus multiplier
                            * oreAmount;
        
        // Remove ore from storage
        float removed = StorageClass::RemoveAmount(oreAmount, slotIndex);
        if (removed > 0.0) {
            anyDeposited = true;
            HouseClass::Add_Tiberium_Credits(removed, slotIndex);
            if (purifierBonus > 0.0) {
                HouseClass::Add_Tiberium_Credits(purifierBonus, slotIndex);
            }
        }
        
        slotIndex = StorageClass::FindFirstNonEmptySlot();
    }
    
    if (anyDeposited) {
        vtable->SiloUpdate();  // vtable+0x468
    }
}
```

### 4c. StorageClass::RemoveAmount (0x6C96B0)

**Address:** 0x6C96B0  
**param_1 type:** `int` (byte offset)

```c
void StorageClass::RemoveAmount(int this, float amount, int slotIndex) {
    float* slot = (float*)(this + slotIndex * 4);
    if (*slot < amount) {
        *slot = *slot - *slot;  // Clamp to 0 (removes all)
    } else {
        *slot = *slot - amount;
    }
}
```

StorageClass is a simple float array indexed by ore type (0=Riparius, 1=Cruentus, 2=Vinifera, 3=Aboreus in TS terms; YR only uses indices 0 and 1 typically).

### 4d. HouseClass::Add_Tiberium_Credits (0x4F9610)

**Address:** 0x4F9610  
**param_1 type:** `int` (byte offset)

```c
float10 HouseClass::Add_Tiberium_Credits(int this, float amount) {
    int credits = Math::ftol(amount);  // truncate to int
    *(this + 0x54E8) = credits;   // HarvestedCreditsThisFrame
    credits = Math::ftol(amount);
    *(this + 0x30C) = credits;    // TotalHarvestedCredits (cumulative)
    return amount;
}
```

Note: The decompilation looks odd — it appears to just set two fields. The actual credit accumulation likely happens via the caller or through `HouseClass::Add_Credits` which is a separate function.

---

## 5. UnloadingClass Swap Mechanism

### 5a. INI Reading (TechnoTypeClass::ReadINI)

**Address of read:** 0x7146E8 (within TechnoTypeClass::ReadINI)  
**INI key:** `UnloadingClass`  
**Stored at:** TechnoTypeClass + 0x6B8 (pointer to UnitTypeClass)

The code:
```asm
PUSH 0x843af8      ; "UnloadingClass"
PUSH EBX           ; section name
MOV ECX,ESI        ; INIClass*
CALL ReadString    ; 0x00528a10
TEST EAX,EAX
JZ skip
LEA ECX,[ESP+0x5c]
CALL UnitTypeClass::FindByName  ; 0x007480d0
skip:
MOV [EBP+0x6B8],EAX  ; store result
```

**Key INI examples:**
- `[HARV]` → `UnloadingClass=HORV`
- `[CMIN]` → `UnloadingClass=CMON`
- `[SLAVE]` → `UnloadingClass=SCHD` (YR only)

### 5b. When does the swap happen?

The swap is **NOT** done by directly changing the unit's type pointer. Instead, the game uses the `UnloadingClass` value at **TechnoTypeClass+0x6B8** to determine which **visual model** (SHP/VXL) to render when the unit is in the "unloading" state.

The mechanism works through the rendering pipeline:
- The unit's `TypeClass` pointer stays the same (e.g., HARV)
- During rendering, when the unit is docked (`DockedBuilding != 0` and in unload state), the renderer checks `TypeClass->UnloadingClass` (offset 0x6B8)
- If non-null, it uses the UnloadingClass's image/voxel instead of the normal one
- This is why HORV/CMON are defined as separate UnitTypes in rules.ini but are never directly buildable — they only exist as visual overlays

### 5c. Timing of swap

- **Swap TO unloading model:** When unit enters the dock (after `BuildingClass::EnterTransport` completes and the unit starts the dump sequence)
- **Swap BACK to normal model:** When `BuildingClass::UndockUnit` is called and the unit exits

The actual visual swap is driven by the `field_0x624` (dump-tick flag) being set in `MissionRepairAndProduce` state 2 — the renderer checks whether the building has an active dump cycle.

---

## 6. Dock Exit / Undock: BuildingClass::UndockUnit (0x4593A0)

**Address:** 0x4593A0  
**param_1 type:** `int*` — offsets are x4

```c
void BuildingClass::UndockUnit(int* param_1) {
    int* unit = (int*)param_1[0xB9];  // building+0x2E4 = DockedUnit
    
    if (unit != NULL) {
        int rtti = vtable->WhatAmI(unit);  // vtable+0x2C
        if (rtti == 1) {  // UnitClass
            // Stop the unit's locomotion
            ILocomotion* loco = unit[0x19D];  // unit+0x674 = locomotion
            if (loco == 0) Assert(E_POINTER);
            loco->Stop();  // ILocomotion vtable+0x58
            
            // Get building's center coords
            int* coords = vtable->GetCoords(param_1, &stack);  // vtable+0x48
            int x = coords[0];
            int y = coords[1];
            int z = coords[2];
            
            // Head_To: move unit to exit position
            if (unit[0x19D] == 0) Assert(E_POINTER);
            loco->Head_To(
                0x47,           // facing = 0x47 (71 = ~east-southeast)
                x - 0x80,       // x offset = -128 leptons
                y + 0x80,       // y offset = +128 leptons  
                z               // same z
            );
            
            // Set speed to 1.0
            vtable->SetSpeedPercent(unit, 1.0);  // vtable+0x544, value = 0x3FF00000 (1.0 double)
            
            // Clear dock links
            unit[0xB9] = 0;        // unit+0x2E4 = 0
            param_1[0xB9] = 0;     // building+0x2E4 = 0
            
            // Notify production system
            vtable->RadioCommand(param_1, 3);  // vtable+0x274, command 3 (RADIO_OVER_AND_OUT)
        }
    }
}
```

### Key findings:
| Detail | Value |
|--------|-------|
| **Exit facing** | 0x47 (71 in decimal, ~east-southeast, roughly 100 degrees) |
| **Exit offset** | (-0x80, +0x80, 0) = (-128, +128, 0) leptons from building center |
| **Exit speed** | 1.0 (full speed) |
| **Dock link field** | `+0x2E4` (param_1[0xB9]) on both building and unit — set to 0 on undock |
| **Radio sent** | Command 3 (RADIO_OVER_AND_OUT) — notifies production system |

### When does Mission_Harvest resume?

After undocking, the unit receives radio command 7 (DOCKING_COMPLETE) via `UnitClass::Receive_Radio` case 7:
```c
case 7:  // DOCKING_COMPLETE
    FootClass::Receive_Radio(sender, 7, param);
    SetDestination(NULL, 1);     // vtable+0x480
    SetTarget(NULL);             // vtable+0x3C8
    SetMission(GUARD, 0);       // vtable+0x1E8, mission 0 = Guard
    FUN_004da1c0();             // clear locomotion state
    
    if (!IsHarvester || GetDestination() == 0) {
        SendRadio(DOCK_LINK, sender);   // vtable+0x278, cmd 2
        SendRadio(0x18, ...);           // request new dock
    }
    return 1;
```

The unit then enters `UnitClass::Mission_Guard_Harvester` (0x740810) which:
- After `HarvestInterval` frames, calls `ScanForTiberium` and transitions to Mission_Harvest
- Mission_Harvest state 0 restarts the harvest cycle

---

## 7. DockingOffset and QueueingCell

### 7a. DockingOffset reading (BuildingTypeClass::ReadINI at 0x4649B7)

**Address:** 0x4649B7  
**INI key:** `DockingOffset%d` (format string at 0x8194B4)  
**Related:** `NumberOfDocks` at 0x8194C4

The reading loop:
```c
// Read NumberOfDocks
int numDocks = ReadInt("NumberOfDocks", existing_value);
// stored at BuildingTypeClass + 0x1780

// Initialize new dock slots to (0, 0, 0)
for (int i = oldCount; i < numDocks; i++) {
    dockArray[i] = {0, 0, 0};  // 3 ints = 12 bytes per dock
}

// Read each DockingOffset
for (int i = 0; i < numDocks; i++) {
    char key[256];
    sprintf(key, "DockingOffset%d", i);
    
    // Read as 3-int coordinate (x, y, z) — from art.ini
    CoordStruct coords = Read3Int(artSection, key, dockArray[i]);
    dockArray[i] = coords;  // 12 bytes each
}
```

**Storage:**
- `BuildingTypeClass + 0x1780` = NumberOfDocks (int)
- `BuildingTypeClass + 0x1788` = pointer to dock offset array
- Each entry is 12 bytes (3 x int32): `{x, y, z}` in leptons
- These values come from **art.ini**, not rules.ini

### 7b. BuildingClass::GetDockCellForObject (0x44EFB0)

**Address:** 0x44EFB0  
**param_1 type:** `int*` — offsets are x4

This function finds a passable cell adjacent to the building for the unit to dock.

Algorithm:
1. Gets the building's top-left cell via `GetMapCell()` (vtable+0x1B8)
2. Checks three "preferred" dock positions (checked via `BuildingTypeClass` flags at offsets 0x16E4, 0x16E5, 0x16E6):
   - Position 1: `(cell_x + 1, cell_y + 2)` — if flag 0x16E4 set
   - Position 2: `(cell_x + 2, cell_y + 2)` — if flag 0x16E5 set
   - Position 3: `(cell_x + 2, cell_y + 1)` — if flag 0x16E6 set
   - Each is checked with `CanEnterCell(cell, -1, -1, 0, 1)` — the last `1` means "ignore movability"
3. For IsRefinery buildings (`Type+0xCCE` and `Type+0x16BD`):
   - Gets the dock coordinate via `GetDockCoord` (vtable+0xA8)
   - Converts to cell and tries: `(dock_cell_x + 1, dock_cell_y + 1)`, `(dock_cell_x + 1, dock_cell_y)`, `(dock_cell_x, dock_cell_y + 1)`
4. If a `param_3` fallback cell is provided, tries that
5. Falls back to iterating around the building's foundation perimeter:
   - If building has an explicit OccupyList (offset 0xED4), iterates that list
   - Otherwise, scans the edges: bottom row, top row, right column, left column

### 7c. Multiple harvesters wanting the same refinery

The game handles this through the radio protocol:

1. **Radio 0xE (CAN_DOCK):** Harvester asks "can I dock here?"
   - Building checks: powered? Allied? Already has a docked unit?
   - If building has a dock queue slot available: returns 1 (OK) with dock info
   - If no slot: returns 10 (NEGATIVE)

2. **Radio 0x15 (DOCK_NOW):** Harvester signals it's ready to dock
   - Building responds by entering Mission_Unload (0x14) and setting the unit's mission to Stand(0)
   - Returns 5 (ALREADY_DOCKED) if already processing

3. **Queue handling:** When a building returns 0x17 (QUEUED) to radio 8:
   - The harvester navigates to the queue cell (found via `GetDockCellForObject`)
   - Waits there until the building finishes with the current harvester
   - The building then sends radio 7 to the waiting harvester

4. **In Mission_Enter:** If the building already has someone docked, the harvester checks `radio 0x13` (IS_UNIT_LINKED) and if not linked, navigates to the queue cell or finds another refinery.

---

## 8. Radio Command Protocol for Docking

### Radio 0x2 (DOCK_LINK)

In `BuildingClass::Receive_Radio` case 0xE, after checking docking is valid:
```c
// If not already in dock queue
if (!DynamicVectorClass::Contains(unit) && CanDock()) {
    SendRadio(2, unit);  // DOCK_LINK — establish radio link
    DynamicVectorClass::Contains();  // verify added
}
```

This establishes a persistent radio link between the harvester and the refinery. The link is stored in `TechnoClass::RadioContact` and allows ongoing communication.

### Radio 0xE (CAN_DOCK) — BuildingClass handler

**Address:** 0x43C2D0, case 0xE

Full checks in order:
1. Call base `TechnoClass::Receive_Radio` first
2. If building has no power (`HasPower == false`): return 10 (NEGATIVE)
3. If building is a repair bay (`Type+0x16A9`) and unit is in dock queue, check radio 0x22 — if returns 10, reject
4. If building is a cloning vat (`Type+0x16AB`) and unit can't auto-deploy: return 10
5. For non-helipad, non-cloning buildings:
   - If unit is not already in dock queue and a slot is available: call radio 0x2 (DOCK_LINK)
   - For refineries (`Type+0x16B3` or `Type+0x16BC`):
     - Get building map cell + offset (3, 1) as queue cell
     - Store queue cell in `param_4` (output parameter)
     - Call radio 0x12 to validate the cell
     - Call radio 0x18 to signal docking start
     - Call radio 0x16 to check timing
   - For other dockable buildings: return appropriate result

### Radio 0xF (CAN_ENTER) — BuildingClass handler

**Address:** 0x43C2D0, case 0xF

Checks in order:
1. Call base `TechnoClass::Receive_Radio`
2. If not allied: return 0 (NO)
3. If building mission is 0x12 or 0x13 (Selling/Under Construction): return 10
4. If `field_0x534 == 0`: return 10 (no cargo capacity)
5. If not in map editor mode AND no free dock slots AND building is not UnitRepair/Helipad/Cloning: return 10
6. Check unit size vs. building capacity (`SizeLimit` at Type+0x5E0, `SizeWeight` at Type+0x388)
7. For IsGrinder (`Type+0x16AD`): return 1 (always accept)
8. For IsCloning (`Type+0x16AB`): check CanAutoDeployHere + radio 0x23
9. For repair pad (`Type+0x16A9`): unit must be RTTI 1 or 2 (Unit or Infantry)
10. For helipad (`Type+0x16C1/0x16C2`): unit must be RTTI 0xF (Aircraft)
11. For refinery (`Type+0x16CB`): unit must be RTTI 2 (Unit) — returns 1 if Unit, 10 otherwise

---

## 9. Free Harvester Spawn: BuildingClass::OnConstructionComplete (0x445F80)

**Address:** 0x445F80  
**param_1 type:** `BuildingClass*` (has named struct fields)

### FreeUnit spawning:

The relevant section (near offset ~0x446CF0 in the function):

```c
// Check: FreeUnit type exists, not in map editor, not captured, not reconstructing
if (Type+0xEA0 != 0 && !g_MapEditorMode && param_2 == 0 && !DAT_00a8ed6b) {
    // Check: player-controlled buildings have construction count check
    if (IsPlayerControl && field_0x300 != 0) {
        int built = TypeClass->GetBuildCount();
        if (field_0x300 <= built) goto skip_freeunit;
    }
    
    // Get building coords and calculate spawn position
    int* buildCoords = GetCoords();  // vtable+0x48
    short spawnCellX = (buildCoords[0] >> 8) + DAT_0089f698.x;
    short spawnCellY = (buildCoords[1] >> 8) + DAT_0089f698.y;
    
    // Create the unit
    UnitClass* unit = new UnitClass(Type+0xEA0, Owner);
    if (unit == NULL) {
        // Refund the cost
        int cost = UnitTypeClass::GetBuildCost(Type+0xEA0, Owner, 1);
        HouseClass::Add_Credits(cost);
        goto skip_freeunit;
    }
    
    // Try to place at spawn position
    int spawnX = spawnCellX * 256 + 128;  // center of cell
    int spawnY = spawnCellY * 256 + 128;
    bool placed = unit->Unlimbo({spawnX, spawnY, 0}, 0xC0);  // facing 0xC0 = 192 = south
    
    if (!placed) {
        // Find nearby passable cell (try multiple strategies)
        CellStruct nearCell = Find_Nearby_Passable_Cell(
            buildingCoords, 2, zoneID, speedType,
            false, 1, 1, 1, 0, 0, &zero
        );
        
        if (nearCell != INVALID_CELL) {
            spawnX = nearCell.x * 256 + 128;
            spawnY = nearCell.y * 256 + 128;
            placed = unit->Unlimbo({spawnX, spawnY, 0}, 0xA0);  // facing 0xA0 = 160 = south-southwest
        }
        
        if (!placed) {
            // Try again without strict zone matching
            nearCell = Find_Nearby_Passable_Cell(
                buildingCoords, 2, zoneID, speedType,
                false, 1, 0, 1, 0, 0, &zero  // less strict
            );
            if (nearCell != INVALID_CELL) {
                placed = unit->Unlimbo({...}, 0xA0);
            }
        }
        
        if (!placed) {
            // Complete failure — refund and destroy
            int cost = GetBuildCost(Owner, 1);
            HouseClass::Add_Credits(cost);
            unit->UnInit(1);
            goto skip_freeunit;
        }
    }
    
    // Successfully placed — start harvesting
    unit->SetMission(HARVEST, 0);   // mission 10 = Harvest
    unit->QueueMission();           // vtable+0x1EC
}
```

### Key findings:
| Detail | Value |
|--------|-------|
| **FreeUnit TypeClass** | `BuildingTypeClass + 0xEA0` (pointer to UnitTypeClass) |
| **Initial facing** | 0xC0 (192) = south; fallback 0xA0 (160) = south-southwest |
| **Spawn cell** | Building center cell + offset from `DAT_0089f698` |
| **Initial mission** | Mission 10 (Harvest) — immediately starts harvesting |
| **Failure handling** | If no passable cell found, refunds the unit cost to owner |
| **Search radius** | 2 cells from building center |

---

## 10. HarvesterLoadRate and HarvesterDumpRate

### HarvesterLoadRate
- **INI key:** `[General] HarvesterLoadRate=`
- **RulesClass offset:** 0x1520 (int)
- **Default:** 2
- **Usage:** In `UnitClass::Harvest_Ore_Tick` (0x73D450), this is the number of **frames** per timer step during ore gathering:
  ```c
  param_1[0x43] = RulesClass+0x1520;  // timer duration
  param_1[0x42] = RulesClass+0x1520;  // timer max
  ```
  The harvester animation plays 9 steps (frames 0-8), and each step takes HarvesterLoadRate frames. So one bale of ore takes `9 * HarvesterLoadRate` frames to gather.

### HarvesterDumpRate
- **INI key:** `[General] HarvesterDumpRate=`
- **RulesClass offset:** 0x1528 (double, read at 0x670CD4)
- **Default:** 0.016 (minutes per bale)
- **Usage:** In `BuildingClass::MissionRepairAndProduce`, the refinery checks:
  ```c
  if (HarvesterDumpRate * 900.0 <= dump_progress) {
      // dump complete — undock unit
  }
  ```
  Where 900.0 = 60 seconds * 15 fps = frames per minute.
  
  So with default 0.016: `0.016 * 900 = 14.4 frames per bale`.

  The `dump_progress` (field_0x620) is incremented by `field_0x638` (step amount, typically 1) each frame when the CDTimer expires.

**NOTE:** The refinery branch in MissionRepairAndProduce actually uses `RulesClass + 0x16E8` in the comparison, not 0x1528. Both are doubles read from [General]. Without finding the exact INI key for 0x16E8, it's possible this is a copy of HarvesterDumpRate or a related but different value. The code at 0x1528 is definitively HarvesterDumpRate from the disassembly at 0x670CD4.

---

## 11. Harvest Ore Tick: UnitClass::Harvest_Ore_Tick (0x73D450)

**Address:** 0x73D450  
**param_1 type:** `int*`

This handles one tick of ore gathering:

```c
uint Harvest_Ore_Tick(int* param_1) {
    int cellAt = CellClass::Get_Cell_At(param_1->Location);
    
    if (param_1[0x169] != 0)  // destination set
        return 1;
    
    // Check: is harvester, not full, cell has ore
    if (!(TypeClass+0xE0E) || GetStorageRatio() >= 1.0 || cell->LandType != 5) {
        // Reset timer — no ore here or full
        param_1[0x3E] = 0;  // step counter reset
        param_1[0x43] = 0;  // timer = 0
        return 0;
    }
    
    // Weed eater special case
    if (TypeClass+0xE0F) {
        FUN_00486e30();  // special weed reduction
        StorageClass::AddAmount(1.0, 0);  // add 1 bale of type 0
        // Timer = HarvesterLoadRate * 3
        param_1[0x43] = RulesClass+0x1520 * 3;
        return 1;
    }
    
    // Normal ore harvesting
    int oreType = FUN_00485010();  // determine ore type from cell
    int capacity = TypeClass+0x800;  // Storage= from rules.ini
    float currentOre = StorageClass::GetTotalAmount();
    float remaining = (float)capacity - currentOre;
    int balesNeeded = ftol(remaining);  // truncate
    
    int harvested = CellClass::Reduce_Tiberium(balesNeeded);
    if (harvested > 0) {
        StorageClass::AddAmount((float)harvested, oreType);
        // Set timer for next harvest step
        param_1[0x43] = RulesClass+0x1520;  // HarvesterLoadRate
        return 1;
    }
    
    return 0;
}
```

---

## 12. Mission_Harvest State Machine (0x73E5E0)

**Address:** 0x73E5E0  
**param_1 type:** `int*`

This is the main harvester state machine with 5 states:

| State | Name | Description |
|-------|------|-------------|
| 0 | **FindOre** | Search for tiberium field, navigate there |
| 1 | **Harvesting** | Actually gathering ore (calls Harvest_Ore_Tick) |
| 2 | **FindRefinery** | Storage full — find nearest refinery and navigate |
| 3 | **EnterDock** | Switch to Mission_Enter (mission 7) |
| 4 | **PostDock** | After docking, decide next action |

### State 0 (FindOre):
- If weed eater (`TypeClass+0xE0F`): calls `Search_For_Tiberium_Short_And_Move`
- If chrono miner (`TypeClass+0xE0E`): checks if locomotion is TeleportLocomotion
  - If teleport + IsRefinery target + no dock link → set dock link
- Normal harvesters: call `FootClass::Search_For_Tiberium_And_Move`
- Sets timer to HarvesterLoadRate, step counter to 0

### State 1 (Harvesting):
- Waits 9 steps (param_1[0x3E] < 9): return 1 (wait)
- Calls `Harvest_Ore_Tick()`
- If no more ore harvested:
  - If storage full → go to state 2
  - Otherwise search for more ore nearby → stay in state 1 or go to state 2

### State 2 (FindRefinery):
- Calls `FindBestObject` (vtable+0x528) with `TypeClass+1000` as search radius
- **Distance check:** 
  - Normal harvester: distance <= `HarvesterTooFarDistance` (RulesClass+0xD78) * 256
  - Chrono harvester: distance <= `ChronoHarvTooFarDistance` (RulesClass+0xD7C) * 256
- If refinery found and in range:
  - Sends radio 2 (DOCK_LINK) to refinery
  - If accepted → go to state 3
- If no refinery found/too far:
  - Searches with `g_MapEditorMode` override (wider search)
  - Finds a passable cell near the refinery using `Find_Nearby_Passable_Cell`
  - Uses the building's `QueueingCell` offset: `Type+0x1618` (x) and `Type+0x161C` (y)
  - Navigates there

### State 3 (EnterDock):
- Simply sets mission to 7 (Enter): `SetMission(ENTER, 0)`
- Mission_Enter then handles the actual docking sequence

### State 4 (PostDock):
- If harvester was "told" to go somewhere (`field_0x3D0 != 0`):
  - Searches for a war factory via `FindBestObject(RulesClass+0x850, 0, 1)`
  - If found: set mission 0x14 (Unload at war factory)
  - If not: set mission 0xF (Hunt)
- Checks if standing on a refinery and if so, moves to queue cell via `FUN_00703590`
- Sets mission to Guard (5)

---

## Summary of Key Offsets

### BuildingClass offsets (byte offsets):
| Offset | Field | Description |
|--------|-------|-------------|
| 0x1CC | DockedUnit | Currently docked unit pointer (set in EnterTransport, param_1[0x73]) |
| 0x2E4 | DockedUnit (alt) | Used in UndockUnit (param_1[0xB9]) — separate dock-link slot |
| 0xBC | DockPhase | State machine phase (0/1/2) |
| 0x620 | DumpProgress | Cumulative dump counter |
| 0x624 | DumpTickFlag | Set to 1 when a dump tick occurs |
| 0x628 | TimerStartFrame | Frame when timer started |
| 0x62C | TimerData | Timer auxiliary data |
| 0x630 | TimerDuration | Current timer duration |
| 0x634 | TimerStep | Step amount per timer tick |
| 0x638 | DumpStep | Amount added to DumpProgress per tick |
| 0x6DD | DirtyFlag | Sidebar needs refresh |

### UnitClass offsets (byte offsets):
| Offset | Field | Description |
|--------|-------|-------------|
| 0x1D0 | DockedBuilding | Building this unit is docked at (set in EnterTransport) |
| 0x1D4 | DockAnim | Current docking animation |
| 0x2E4 | DockedBuilding (alt) | Used in UndockUnit — separate dock-link slot |
| 0x674 | Locomotion | ILocomotion COM pointer |
| 0x6B2 | IsHarvesting | Currently gathering ore flag |
| 0x6D2 | HasOreToGather | Found ore to harvest |

### TechnoTypeClass offsets (byte offsets):
| Offset | Field | Description |
|--------|-------|-------------|
| 0x6B8 | UnloadingClass | UnitTypeClass pointer for visual swap during unloading |
| 0x6BC | DeployingAnim | AnimType for deploying animation |
| 0x800 | Storage | Max storage capacity |
| 0xE0E | Harvester | Is a harvester (bool) |
| 0xE0F | Weeder | Is a weed eater (bool) |

### BuildingTypeClass offsets (byte offsets):
| Offset | Field | Description |
|--------|-------|-------------|
| 0x16B3 | IsRefinery | Building accepts ore |
| 0x16BB | IsRefinery (storage) | Building has ore storage |
| 0x16BC | IsSlaveMiner | Is a slave miner refinery |
| 0x16BD | IsRefinery (dock) | Building has docking pads |
| 0x1618 | QueueingCellX | X offset for queue cell |
| 0x161C | QueueingCellY | Y offset for queue cell |
| 0x1780 | NumberOfDocks | Number of docking pads |
| 0x1788 | DockOffsets | Pointer to array of 3-int dock coordinates |
| 0xEA0 | FreeUnit | UnitTypeClass for free unit on construction |

### RulesClass offsets (byte offsets):
| Offset | Field | Description |
|--------|-------|-------------|
| 0x31C | DockAnim | AnimType for dock animation |
| 0xD78 | HarvesterTooFarDistance | Max cell distance for normal harvesters |
| 0xD7C | ChronoHarvTooFarDistance | Max cell distance for chrono miners |
| 0xF3C | PurifierBonus | Float multiplier for Purifier bonus |
| 0x1520 | HarvesterLoadRate | Frames per step during ore gathering (int) |
| 0x1528 | HarvesterDumpRate | Minutes per bale during unloading (double) |
| 0x1790 | HarvestInterval | Frames between harvest scans |
