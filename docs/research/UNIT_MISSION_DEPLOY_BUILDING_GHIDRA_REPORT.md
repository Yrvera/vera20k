# UnitClass::Mission_Deploy_Building -- Deep Dive

**Address:** `0x0073D630` -- `0x0073E5BD` (3966 bytes, 658 decompiled lines)
**Complexity:** 197 basic blocks, 1177 instructions, 139 calls, cyclomatic complexity 180
**Confidence:** HIGH -- fully decompiled and cross-referenced

## Overview

This single mega-function handles **three different systems**:

1. **MCV Deployment** -- converting an MCV unit into a Construction Yard building
2. **Harvester/Weeder Refinery Approach** -- driving toward and entering a refinery (undocked path)
3. **Harvester/Weeder Ore Dumping** -- per-bale credit transfer while docked (docked path)
4. **IsSimpleDeployer** -- units that deploy in-place (like siege choppers)

The function has TWO top-level branches based on `param_1[0xb9]` (DockedTo pointer):

```
if (param_1[0xb9] == 0) {
    // NOT DOCKED: Approach refinery OR enter it (states 0,1,3,4 in undocked switch)
} else {
    // DOCKED: Undock from building via FUN_004595c0
}
// Then falls to LAB_0073d672:
//   - If !Harvester && !Weeder: MCV deploy / IsSimpleDeployer
//   - If Harvester || Weeder: Ore dumping state machine (states 3, 4)
```

The MCV vs Harvester paths are distinguished by `UnitTypeClass::DeploysInto` at byte offset `+0x404` and the `Harvester`/`Weeder` flags at `+0xE0E`/`+0xE0F`.

---

## Key Struct Offsets (UnitClass, param_1 is `int *`)

| Index | Byte Offset | Field | Description |
|-------|-------------|-------|-------------|
| `param_1[0x24]` | 0x90 | **IsDeployed** | Bool: unit is in deployed state (byte, low bit) |
| `param_1[0x2d]` | 0xB4 | **CurrentMission** | Current mission ID (-1 = none, 0x10 = special) |
| `param_1[0x2e]` | 0xB8 | **MissionComplete** | Bool: mission done flag (byte, set in state 4) |
| `param_1[0x2f]` | 0xBC | **MissionState** | Sub-state within the current mission (the state machine variable) |
| `param_1[0x3e]` | 0xF8 | **AnimFrameCounter** | Current animation frame / dump timer counter |
| `param_1[0x40]` | 0x100 | **AnimStartFrame** | Frame counter at animation start |
| `param_1[0x41]` | 0x104 | **AnimRate** | Animation playback rate |
| `param_1[0x42]` | 0x108 | **AnimStepRate** | Steps per frame |
| `param_1[0x43]` | 0x10C | **AnimTotalFrames** | Total animation frames |
| `param_1[0x45]` | 0x114 | **DockDirection** | Dock approach direction index (from vtable+0x304) |
| `param_1[0x4c]` | 0x130 | **DeployAnim** | Pointer to current deploy animation (AnimClass*) |
| `param_1[0x4d]` | 0x134 | **DeployAnimPhase** | Set to 1 when deploy animation is playing (byte) |
| `param_1[0x87]` | 0x21C | **OwnerIndex** | Owner house/player index |
| `param_1[0xad]` | 0x2B4 | **Target** | Current target object |
| `param_1[0xb9]` | 0x2E4 | **DockedTo** | Pointer to the building this unit is docked at |
| `param_1[0x169]` | 0x5A4 | **IsSlaveMiner** | If nonzero, slave miner forced undock behavior |
| `param_1[0x175]` | 0x5D4 | **RadioLink** | Current radio/transport link |
| `param_1[0x178]` | 0x5E0 | **TargetDockBuilding** | Target building for docking (set to -1 to clear) |
| `param_1[0x19d]` | 0x674 | **Locomotor** | ILocomotion COM interface pointer |
| `param_1[0x1a3]` | 0x68C | **DeployToFire** | Flag: deploy is for firing (not building placement) |
| `param_1[0x1b1]` | 0x6C4 | **UnitTypeClass*** | Pointer to this unit's type class |
| `param_1[0x1b8]` | 0x6E0 | **DeployedFlag** | Boolean: unit is in deployed state (byte) |
| `param_1[0x1b9]` | 0x6E4 | **DumpBaleIndex** | Current bale being dumped (incremented per dump cycle) |
| `byte 0x6AF` | -- | **IsMoving** | Whether the unit is currently in motion |
| `byte 0x6D1` | -- | **DockingInitialized** | Set to 1 once docking animation/door has been opened |
| `byte 0x6E1` | -- | **DeployAnimForward** | Deploy animation playing forward |
| `byte 0x6E2` | -- | **DeployAnimReverse** | Deploy animation playing in reverse |

## Key UnitTypeClass Offsets (byte offsets from UnitTypeClass start)

| Offset | Field | Source |
|--------|-------|--------|
| `+0x398` | **NaturalMission** | Set to 0x0F (Guard) normally, 0x0A (Harvest) if Harvester/Weeder |
| `+0x404` | **DeploysInto** | BuildingTypeClass* (NULL if not MCV-type) |
| `+0x5E0` | **Storage** | Max storage capacity (int) -- checked at function entry |
| `+0x5E4` | **Enslaved** | Slave miner flag (bool) |
| `+0x6AC` | **CanPassiveAcquire** | Bool: used when Storage < 1 |
| `+0x6AD` | **NavalUnit** | Bool: checked for path-clear after undeploy |
| `+0x6BC` | **DeployingAnim** | AnimTypeClass* for deploy animation |
| `+0x805` | **DockQueueFlag** | Bool: controls dock queue behavior |
| `+0xE0E` | **Harvester** | Bool: `Harvester=yes` in INI |
| `+0xE0F` | **Weeder** | Bool: `Weeder=yes` in INI |
| `+0xE13` | **IsSimpleDeployer** | Bool: `IsSimpleDeployer=yes` in INI |
| `+0xEDC` | **DeployFacing** | Required facing for deployment |

## Key BuildingTypeClass Offsets

| Offset | Field | Source |
|--------|-------|--------|
| `+0x16B3` | **DockUnload** | Bool |
| `+0x16B9` | **ConstructionYard** | Bool |
| `+0x16BB` | **Refinery** | Bool -- controls dock door open/close animations |
| `+0x16BC` | **Weeder** | Bool (on BuildingTypeClass) |
| `+0x16BD` | **WeaponsFactory** | Bool |

## Key BuildingClass Offsets

| Offset | Field | Description |
|--------|-------|-------------|
| `+0x520` | **Type** | BuildingTypeClass* pointer |
| `+0x57C` | **DockDoorAnim** | Dock door animation pointer (non-null while playing) |
| `+0x584` | **SpecialAnim** | Special production animation pointer |
| `+0x2E4` | **DockedUnit** | Pointer to unit currently docked |
| `+0x718` | **DockQueueHead** | Dock queue tracking field |

---

## COMPLETE FUNCTION STRUCTURE

```
UnitClass::Mission_Deploy_Building(param_1):

    BRANCH 1: if (param_1[0xb9] == 0)    // NOT DOCKED
    |
    |   if (UnitTypeClass->Storage < 1):
    |       if (CanPassiveAcquire && !IsSimpleDeployer):
    |           // Set destination to own cell, try to find path
    |           return random(0,2) + 14
    |       goto LAB_0073d672
    |
    |   switch (param_1[0x2f]):    // UNDOCKED STATE MACHINE
    |       case 0: Approach Init
    |       case 1: Driving Into Refinery
    |       case 3: Find Exit Cell / Fully Enter Refinery
    |       case 4: Set Guard Mission
    |
    BRANCH 2: else                        // DOCKED (param_1[0xb9] != 0)
    |   Look up building in cell
    |   Call FUN_004595c0 (full undock + redirect)
    |
    LAB_0073d672:                         // COMMON CONTINUATION
    |
    |   if (!Harvester && !Weeder):
    |       if (DeploysInto == NULL):
    |           if (IsSimpleDeployer):
    |               // Play deploy/undeploy animation
    |               return 1 while animating, then Guard
    |           return 0x1C2 (450 ticks, fallback)
    |       else:
    |           // MCV DEPLOY STATE MACHINE (states 0, 1, 2)
    |           return timer
    |
    |   // HARVESTER/WEEDER ORE DUMPING PATH
    |   Check PathType::Has_Valid_Steps
    |   Check facing alignment
    |   if (byte 0x6D1 == 0): Initialize docking (open door, set state 3)
    |   else if (state == 3): Per-bale dump loop
    |   else if (state == 4): Undock and exit
```

---

## PATH 1: UNDOCKED HARVESTER APPROACH (param_1[0xb9] == 0, Storage > 0)

### Undocked State 0: Approach Init

1. **Check locomotor stopped:** Gets locomotor at `param_1[0x19d]`, calls `ILocomotion[4]` (Is_Moving). If still moving, returns 10.

2. **Check cell occupation:** Gets unit's current cell via `vtable+0x1B8`, then `MapClass::Get_CellClass()`. If `param_1[0x169]` (IsSlaveMiner) is 0 AND cell occupancy (`cell+0xEC`) == 2 (blocked):
   - Calls `FootClass__Find_Nearby_Passable_Cell` to find alternate cell
   - Moves unit via `vtable+0x480` (ScatterTo)
   - Returns 10

3. **Get dock direction:** `iVar3 = param_1[0x45]`, then calls `vtable+0x304` (GetDockDirection)

4. **If valid direction** (`iVar3 != 0` AND coordinates differ from current position):
   - Calls `FUN_0070dc60()` to verify approach path is clear
   - **Sets dump start index based on direction:**
     ```
     if (iVar3 == 1) param_1[0x1b9] = 0;  // Start from bale 0
     else             param_1[0x1b9] = 1;  // Start from bale 1
     ```
   - **Commands locomotor:** Converts facing to heading (`sVar13 << 0xd`), calls `ILocomotion::Set_Desired_Heading` (locomotor vtable+0x4C)
   - **Transitions:** `param_1[0x2f] = 1`, returns 1

5. **If no valid direction:** Calls `vtable+0x1E8(5, 0)` = SetMission(Guard)

### Undocked State 1: Driving Into Refinery

1. **Check if unit stopped moving:**
   ```
   if (byte 0x6AF == 0) {  // Not moving
       param_1[0x2f] = 3;  // Jump to state 3
       return 1;
   }
   ```
2. If still moving: `break` (falls to common timer return)

### Undocked State 3: Find Exit Cell and Enter Refinery

This state handles the harvester physically entering the refinery. It:

1. **Checks** `param_1[0x1b9] < param_1[0x45]` -- if dump counter < dock direction (if there are more docking steps)

2. **Gets the refinery** via `FUN_004de710()` (dequeue from dock list)

3. **Calculates a facing offset** from `RateTimer::Current()`:
   ```
   local_50 = (*psVar11 + 0x7FFF);
   local_4c[0] = ((local_50 >> 12) + 1) >> 1;  // Normalized facing
   ```

4. **Iterates 8 directions** (do-while loop, `local_68 = 0..7`):
   - Computes candidate cell: `cell_x + g_DirectionOffsets[dir*2]`, `cell_y + g_DirectionOffsets[dir*2+1]`
   - Checks cell passability via building's `vtable[0x6B]` (two calls with different cells and heights via `CellClass::Get_Effective_Height`)
   - Checks for bridge presence (`cell+0x140 & 0x100`)

5. **If valid cell found:**
   - Increments `g_MapEditorMode` (disables map editor constraints temporarily)
   - If building type == 0x0F: Uses `FUN_004aca10` (infantry placement in cell)
   - Otherwise: Calls building's `vtable[0x21]` (FindPath) + `Pathfinding_validate_alternate`
   - Checks placement via building's `vtable[0x36]` with the calculated coordinates and facing
   - Decrements `g_MapEditorMode`

6. **On successful placement:**
   - If `UnitTypeClass+0x5E4` (Enslaved): calls `TechnoClass__ClearInOpenTransport (0x007104A0)` <!-- corrected 2026-05-28: was "FUN_007104a0(building, 0) (clears byte +0x82)"; binary shows TechnoClass__ClearInOpenTransport @ 0x007104A0 via search_functions — ROOT_CAUSE: RTTI_LABEL_DRIFT -->
   - If Enslaved AND owner mismatch: calls building's `vtable[0xF2]` (transfer ownership)
   - Clears building dock slot: `building[0x47] = NULL`
   - **Opens dock door:** Calls building `vtable[0x7A]` with param 2
   - **Moves unit into building:** Calls `vtable[0x120]` (ScatterTo with cell, 1)
   - If unit has radio link: calls `FUN_006ea500(building, 0)` (attach)
   - If type has sound (`TypeClass+0x56C != -1`): plays sound via `VocClass__PlayAtCoord (0x00750E20)` <!-- corrected 2026-05-28: was "If type has idle anim: spawns via FUN_00750e20"; binary shows VocClass__PlayAtCoord @ 0x00750E20 via search_functions; decompile confirms TypeClass+0x568 sound check not anim check — ROOT_CAUSE: RTTI_LABEL_DRIFT -->
   - Goes to common return

7. **If no valid cell in all 8 directions:**
   - Adds unit as passenger via `CargoClass__AddPassenger (0x004733A0)` <!-- corrected 2026-05-28: was "Requeues building via FUN_004733a0()"; binary shows CargoClass__AddPassenger @ 0x004733A0 via search_functions — ROOT_CAUSE: RTTI_LABEL_DRIFT -->
   - If `TypeClass+0x805`: calls `vtable+0x4D4` (force eject)
   - Calls building's `vtable[0x47]` (reset)

8. **If dump counter >= dock direction** (all steps done): `param_1[0x2f] = 4`

### Undocked State 4: Guard

```
vtable+0x1E8(5, 0)  // SetMission(Guard)
param_1[0x2e] = 1   // byte at 0xB8 = mission complete flag
```

---

## PATH 2: MCV DEPLOYMENT (DeploysInto != NULL, at LAB_0073d672)

Reached when `!Harvester && !Weeder && DeploysInto != NULL`.

### MCV State 0: Initiate Deploy

```
vtable+0x274(3)         // ScanForTargets
param_1[0x178] = -1     // Clear target dock building
param_1[0x2f] = 1       // Transition to state 1
```

### MCV State 1: Wait for Locomotor + Deploy

1. Checks locomotor: `ILocomotion[4]` (Is_Moving)
2. If stopped: Calls `UnitClass::Deploy()` at `0x007393C0` <!-- corrected 2026-05-28: was 0x00739390 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->
3. If deploy succeeds (`param_1[0x24]` set):
   - If `param_1[0x1a3]` (DeployToFire) clear:
     - `HouseClass__IsPlayerControl (0x0050B730)` checks if house is player-controlled <!-- corrected 2026-05-28: was "FUN_0050b730() checks placement validity"; binary confirms HouseClass__IsPlayerControl @ 0x0050B730 via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT -->
     - If NOT player-controlled AND multiplayer: SetMission(0x0F = Guard)
     - If player-controlled (or single-player): SetMission(0x05 = Guard)
   - If DeployToFire set: `param_1[0x2f] = 2` (transition to state 2)
4. If locomotor still moving but deploy state + `param_1[0x2d] != -1` and `!= 0x10`:
   - Calls `vtable+0x1EC` (QueueMission)

### MCV State 2: Deploy To Fire

1. If `param_1[0x1a3]` (DeployToFire) clear:
   - `vtable+0x484(0, 1)` (ForceScatter)
   - `vtable+0x1EC` (QueueMission)
2. If DeployToFire set:
   - Calls `UnitClass::Deploy()` again
   - If deploy fails AND `param_1[0x169]` (IsSlaveMiner): clears DeployToFire flag

### UnitClass::Deploy() (0x007393C0) -- MCV to Building Conversion
<!-- corrected 2026-05-28: was 0x00739390; binary shows entry at 0x007393C0 via get_function_by_address — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->

1. Checks `vtable+0x314` (CanDeploy) -- returns 0 if cannot
2. Verifies locomotor is not moving via `ILocomotion[4]`
3. Gets DeploysInto type from `UnitTypeClass+0x404`
4. Calls `BuildingTypeClass::CanBePlacedAt` to verify placement
5. Gets required facing from `Deploy_facing_calculator()` (reads `UnitTypeClass+0xEDC`)
6. **If not at correct facing:**
   - Sets desired heading via `ILocomotion::Set_Desired_Heading` (locomotor vtable+0x4C)
   - Returns 1 (still turning)
7. **When at correct facing:**
   - Allocates `BuildingClass` (0x720 bytes)
   - Calls `BuildingClass::Constructor` with the DeploysInto type and owner
   - Sets building mission to 0x12 (Construction)
   - Places building via `vtable+0xD8` (PlaceAtCoords)
   - Iterates all TechnoClass objects: redirects any unit targeting this MCV to target the new building (or NULL if building has ConstructionYard=yes and target is aircraft on helipad)
   - Transfers health, veterancy, etc.
   - **Destroys the MCV unit**
   - Returns 1

---

## PATH 3: SIMPLE DEPLOYER (IsSimpleDeployer=yes, at LAB_0073d672)

Reached when `!Harvester && !Weeder && DeploysInto == NULL && IsSimpleDeployer`.

Calls one of two functions based on `param_1[0x1b8]` (deployed flag):

### FUN_00739AC0 -- Deploy Forward Animation

1. Checks `UnitTypeClass+0xE13` (IsSimpleDeployer)
2. If has ammo (`vtable+0x1C8` > 0) and `TypeClass+0x6AD` clear: returns early
3. If `param_1[0x4D]` (DeployAnimPhase) is 0 AND has ammo: sets `param_1[0x4D] = 1`
4. If `byte 0x6E1` (forward anim) active AND `param_1[0x4C]` (AnimPointer) exists:
   - Checks if anim reached last frame; if so: sets `param_1[0x1B8] = 1` (deployed), clears forward flag
5. If not yet started:
   - If no DeployingAnim (`UnitTypeClass+0x6BC == 0`): immediately set deployed
   - Else: Creates AnimClass from DeployingAnim, sets frame counters, starts forward animation
6. If type has idle anim (`TypeClass+0x56C != -1`): spawns idle anim at location

### FUN_00739CD0 -- Deploy Reverse Animation (Undeploy)

1. If deployed (`param_1[0x1B8]` set):
2. If `byte 0x6E2` (reverse anim) active AND `param_1[0x4C]` exists:
   - Checks if anim reached second-to-last frame; if so: sets `param_1[0x1B8] = 0` (undeployed), clears reverse flag
   - If `TypeClass+0x6AD` (NavalUnit): calls `vtable+0x480` (scatter to nearby cell)
3. Else: Creates AnimClass from DeployingAnim with reverse=1 flag, starts reverse animation
4. If type has active anim (`TypeClass+0x570 != -1`): spawns anim at location

After animation completes (both `byte 0x6E1`, `byte 0x6E2`, and `param_1[0x4D]` are all 0):
- SetMission(5 = Guard)
- QueueMission

Returns `thunk_FUN_005b2ef0()` = **0x1C2 (450 ticks)** as fallback.

---

## PATH 4: HARVESTER ORE DUMPING (at LAB_0073d672, Harvester || Weeder)

This is reached for Harvester/Weeder units after the undocked/docked branch has executed.

### Pre-Check: Path Validity

```
cVar2 = PathType__Has_Valid_Steps();
if (cVar2 == 0) {
    // No valid path steps -- abort docking
    vtable+0x484(0, 1)           // ForceScatter
    byte 0x6D1 = 0               // Clear docking initialized
    ILocomotion::Is_Moving_Now() // locomotor vtable+0x10
    if (moving) vtable+0x500()   // ForceStop
    if (vtable+0x200())          // ShouldScatter
        vtable+0x1EC()           // QueueMission
    return 1;
}
```

### Facing Alignment Check

```
puVar10 = RateTimer::Current();
if (((*puVar10 >> 7) + 1 & 0x1FE) != 0x80) {
    // Not at correct facing
    if (byte 0x6AF == 0) {  // Not moving
        // Set heading to 0x4000 (due south)
        ILocomotion::Set_Desired_Heading(locomotor, 0x4000)
    }
    return 5;
}
```

### Dock Initialization (byte 0x6D1 == 0 -> first entry)

```
param_1[0x3E] = 0;             // Reset dump timer
byte 0x6D1 = 1;                // Mark docking initialized
param_1[0x43] = 1;             // Total anim frames = 1
param_1[0x40] = g_CurrentFrameCounter;
param_1[0x42] = 1;             // Step rate
```

**For Harvesters (UnitTypeClass+0xE0E set):**
```
// Find refinery building in adjacent cell via DAT_0089f6a0 offset
psVar11 = vtable+0x1B8();  // GetCoords
cell = (psVar11[0] + DAT_0089f6a0.x, psVar11[1] + DAT_0089f6a0.y)
building = Look_up_building_in_cell(MapClass::Get_CellClass(cell));
if (building != NULL) {
    healthRatio = ObjectClass::GetHealthRatio(building);
    isDamaged = healthRatio <= RulesClass+0x1700;
    BuildingClass__SetAnimSlotImage(7, isDamaged, 0);  // OPEN DOCK DOOR (slot 7)
}
```

**Transition:** `param_1[0x2f] = 3` (enter dump state)

### Dump State 3: Per-Bale Credit Transfer

Each tick while in state 3:

1. **Find refinery** in adjacent cell (same pattern as above)

2. **Check dump rate timer:**
   ```
   // RulesClass+0x1528 = HarvesterDumpRate (double)
   // DAT_007e27f8 = 900.0 (constant)
   if (HarvesterDumpRate * 900.0 <= param_1[0x3E]) {
       // Time to dump a bale
   ```

3. **Close intermediate dock door:**
   ```
   building->vtable+0x468()  // CloseDockDoor (vtable slot 0x11A)
   ```

4. **Show production anim (if no special anim running):**
   ```
   if (building->field_0x584 == 0) {
       BuildingClass__SetAnimSlotImage(10, isDamaged, 0, 0);
   }
   ```

5. **Find first non-empty ore slot:**
   ```
   iVar3 = StorageClass__FindFirstNonEmpty();  // Returns 0-3, or -1 if empty
   ```
   StorageClass is a float[4] array, one slot per ore type.

6. **Calculate purifier bonus:**
   ```
   iVar9 = building->vtable+0x3C()  // GetOwnerHouse
   purifier_storage = HouseClass+0x538C
   if (!is_AI && multiplayer) {
       purifier_storage += difficulty_bonus[HouseClass+0x184]
   }
   ```

7. **Get ore amount and calculate credit value:**
   ```
   amount = StorageClass::GetAmount(ore_index)  // Float value
   purifier_bonus = (float)purifier_storage * RulesClass+0xF3C * amount
   ```

8. **Remove ore from storage and deposit credits:**
   ```
   actual_removed = StorageClass::Remove(amount, ore_index)
   if (actual_removed > 0.0) {
       if (UnitTypeClass+0xE0F == 0) {
           // HARVESTER PATH
           HouseClass__Add_Tiberium_Credits(actual_removed, ore_index)  // corrected 2026-05-28: was HouseClass__DepositOreCredits
           if (purifier_bonus > 0.0) {
               HouseClass__Add_Tiberium_Credits(purifier_bonus, ore_index)
           }
       } else {
           // WEEDER PATH
           amount_int = Math__ftol(actual_removed)
           HouseClass__Add_Tiberium_To_Storage(amount_int, ore_index)  // corrected 2026-05-28: was HouseClass__DepositWeedCredits
       }
       param_1[0x3E] = 0  // Reset dump timer
       goto LAB_0073e539  // Continue dumping
   }
   ```

9. **When storage is empty (FindFirstNonEmpty returns -1):**
   ```
   // Close dock door animation
   if (building->Type[0x16BB] != 0) {  // Refinery flag
       BuildingClass__SetAnimSlotImage(8, isDamaged, 0, 0)  // CLOSE DOOR (slot 8)
   }
   param_1[0x2f] = 4  // Transition to undock state
   if (building->field_0x584 != 0) {
       BuildingClass__ClearAnimSlot(building)  // Clear special anim
   }
   ```

10. **Force-undock check for slave miners:**
    ```
    if (param_1[0x169] != 0 && param_1[0x2d] != -1 && param_1[0x2d] != 10) {
        // Same door-close and state transition
        if (building->Type[0x16BB]) SetAnimSlotImage(8, ...)
        param_1[0x2f] = 4
        if (building->field_0x584) ClearAnimSlot(building)
    }
    ```

11. **If refinery not found:** Sets mission to Harvest(10, 1), returns timer

### Dump State 4: Finish / Undock

**Two paths based on Weeder flag:**

#### Harvester Undock (Weeder=no):

1. **Find refinery building** in adjacent cell

2. **Wait for dock door animation to finish:**
   ```
   if (building != NULL
       && building->Type[0x16BB]   // Refinery flag
       && building+0x57C != 0) {   // Dock door anim still playing
       return 1;  // Wait
   }
   ```

3. **Clear docking flag:** `byte 0x6D1 = 0`

4. **Normal harvester exit** (`param_1[0x169] == 0` OR `param_1[0x2d] == -1` OR `== 10`):
   ```
   vtable+0x1E8(10, 0)  // SetMission(Harvest)
   if (vtable+0x200()) {  // ShouldScatter
       if (PathType__Has_Valid_Steps()) {
           vtable+0x274(3)  // ScanForTargets
       }
       vtable+0x1EC()  // QueueMission
   }
   ```

5. **Slave miner exit** (forced undock):
   ```
   // Check locomotor via ILocomotion[4]
   if (moving) vtable+0x500()  // ForceStop
   if (vtable+0x200()) vtable+0x1EC()  // QueueMission
   ```

#### Weeder Undock (Weeder=yes):

1. **Immediately clear** `byte 0x6D1 = 0` -- **NO dock door wait**

2. Same normal vs slave miner exit logic as Harvester

**Key difference:** Weeders skip the dock door animation wait in state 4.

---

## DOCKED BRANCH (param_1[0xb9] != 0)

When the unit is docked, the function runs before LAB_0073d672:

```
vtable+0x1BC()                           // GetCenterCoords
building = Look_up_building_in_cell()    // Find building
if (building != NULL) {
    vtable+0x1BC()                       // Re-get center coords
    Look_up_building_in_cell()           // Re-find building
    FUN_004595c0(building)               // FULL UNDOCK + REDIRECT
}
// Falls through to LAB_0073d672
```

### BuildingClass__ReleaseDockedHarvester (0x004595C0) -- Release Docked Harvester
<!-- corrected 2026-05-28: was "FUN_004595c0: Full Undock + Redirect"; binary Ghidra name is BuildingClass__ReleaseDockedHarvester (not a generic redirect helper — note also per plate comment, not the normal stock CMIN/HARV unload path) via get_function_by_address 0x004595C0 — ROOT_CAUSE: RTTI_LABEL_DRIFT -->

```c
void FUN_004595c0(BuildingClass* building) {
    // 1. Clear animation slots 10 (0x0A) and 11 (0x0B)
    BuildingClass__ClearAnimSlot(building, 10);
    BuildingClass__ClearAnimSlot(building, 11);

    // 2. Spawn smoke animation if RulesClass+0x244 != -1
    if (RulesClass+0x244 != -1) {
        AnimClass__SpawnAtCoord(building->Location);
    }

    // 3. Create dock visual anims (slots 12, 13)
    double health = ObjectClass::GetHealthRatio(building);
    // Slot 12: Normal=Type+0x127C, Damaged=Type+0x128C
    // Slot 13: Normal=Type+0x12C0, Damaged=Type+0x12D0

    // 4. Get docked unit
    int* unit = building->DockedUnit;  // +0x2E4
    if (unit == NULL) {
        building->field_0x718 = 0;
        SetMission(building, 5);  // Guard
        return;
    }

    if (unit->GetRTTI() == 1) {  // RTTI_Unit
        unit[0xB9] = 0;  // Clear unit dock link

        ILocomotion::Stop(unit->Locomotor);

        int* exitCoords = building->GetExitCoords();  // vtable+0x48

        // Command unit to drive SE with facing 0x47
        ILocomotion::Head_To(
            unit->Locomotor,
            0x47,                    // Facing ~100 degrees (SE)
            exitCoords[0] - 0x80,    // X: -128 leptons
            exitCoords[1] + 0x80,    // Y: +128 leptons
            exitCoords[2]            // Z: same
        );

        // Set speed to 1.0
        vtable+0x544(unit, 0, 0x3FF00000);  // 1.0 as IEEE754 double

        // Find a scatter cell: unit cell + (-1, +1)
        // Calls FindPath + Pathfinding_validate_alternate
        // Then vtable+0x480 (ScatterTo)

        // Set unit mission to 2 (Move)
        vtable+0x1E8(unit, 2, 0);

        // Clear building dock state
        building->DockedUnit = 0;    // +0x2E4
        building->field_0x718 = 0;

        SetMission(building, 5);     // Guard
        vtable+0x274(building, 3);   // ScanForTargets
    }
}
```

### BuildingClass::UndockUnit (0x004593A0) -- Simpler Version

This is a simpler undock that doesn't do the pathfinding/scatter:

```c
void BuildingClass__UndockUnit(int* building) {
    int* unit = building[0xB9];  // DockedUnit
    if (unit && unit->GetRTTI() == 1) {
        ILocomotion::Stop(unit->Locomotor);
        int* exit = building->GetExitCoords();
        ILocomotion::Head_To(unit->Locomotor, 0x47,
            exit[0] - 0x80, exit[1] + 0x80, exit[2]);
        unit->SetSpeed(1.0);
        unit[0xB9] = 0;
        building[0xB9] = 0;
        vtable+0x274(building, 3);
    }
}
```

**Exit facing: 0x47** in RA2's 256-value facing system = ~100 degrees = **southeast**.
**Exit offset: (-0x80, +0x80)** = (-128, +128) leptons from building exit point.

---

## DOCK DOOR ANIMATION SYSTEM

### Animation Slots Used

| Slot | Purpose | When Set | When Cleared |
|------|---------|----------|--------------|
| **7** | Dock door **OPEN** | Dock init (byte 0x6D1: 0->1), Harvester only | -- |
| **8** | Dock door **CLOSE** | All bales dumped, Refinery flag checked | -- |
| **10** | Production/active anim | During per-bale dump (if field_0x584 == 0) | FUN_004595c0 teardown |
| **11** | Secondary production anim | -- | FUN_004595c0 teardown |
| **12** | Dock idle visual | FUN_004595c0 (from Type+0x127C/0x128C) | -- |
| **13** | Dock idle visual | FUN_004595c0 (from Type+0x12C0/0x12D0) | -- |

### BuildingClass::SetAnimSlotImage (0x00451750)
<!-- corrected 2026-05-28: was 0x00459960; binary shows BuildingClass__SetAnimSlotImage @ 0x00451750 via search_functions — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->

Selects animation name from BuildingTypeClass based on slot, health, and param:

```c
void SetAnimSlotImage(BuildingClass* this, int slot, bool isDamaged, bool isSpecial) {
    char* animName;
    if (!isDamaged) {
        animName = this->Type + slot * 0x44 + (isSpecial ? 0xF6C : 0xF4C);
    } else {
        animName = this->Type + slot * 0x44 + 0xF5C;
    }
    if (animName && *animName) {
        BuildingClass__CreateAnimForSlot(this);
    }
}
```

**Stride: 0x44 (68) bytes per slot** in BuildingTypeClass, starting at `+0xF4C`.

### BuildingClass::ClearAnimSlot (0x00451E40)
<!-- corrected 2026-05-28: was 0x004592B0; binary shows BuildingClass__ClearAnimSlot @ 0x00451E40 via search_functions — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->

```c
void ClearAnimSlot(BuildingClass* this, int slot) {
    if (slot == -2) {
        // Clear ALL 21 slots
        for (int i = 0; i < 21; i++) { ... }
    } else {
        if (this->Anims[slot]) {
            this->Anims[slot]->Destroy(1);
            this->Anims[slot] = NULL;
        }
    }
}
```

---

## WEEDER vs HARVESTER DIFFERENCES

| Aspect | Harvester | Weeder |
|--------|-----------|--------|
| **Credit function** | `HouseClass__Add_Tiberium_Credits` (0x004F9610) <!-- corrected 2026-05-28: was 0x006C8F10 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT --> | `HouseClass__Add_Tiberium_To_Storage` (0x004F9700) <!-- corrected 2026-05-28: was 0x006C9040 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT --> |
| **Deposit mechanism** | Direct integer credit set at HouseClass+0x54E8 and +0x30C | Loop: adds 1.0 per iteration to storage, capped by RulesClass+0x17D0 |
| **Purifier bonus** | Yes: `purifier * RulesClass+0xF3C * amount` | **No purifier bonus** |
| **Amount type** | Float (fractional ore) | Integer (Math__ftol truncation) |
| **Dock door wait** | Waits for `building+0x57C` to clear | **Skips door wait entirely** |
| **Ore type** | Uses StorageClass float[4] array | Same storage, different deposit |

### HouseClass::Add_Tiberium_Credits (0x004F9610)
<!-- corrected 2026-05-28: was named "HouseClass::DepositOreCredits (0x006C8F10)"; binary shows HouseClass__Add_Tiberium_Credits @ 0x004F9610 (0x006C8F10 does not exist) via search_functions + decompile_function 0x004F9610 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->

```c
float10 Add_Tiberium_Credits(HouseClass* this, float amount) {
    this->field_0x54E8 = Math__ftol(amount);   // Ore income display
    this->field_0x30C = Math__ftol(amount);     // Actual credit balance
    return amount;
}
```

### HouseClass::Add_Tiberium_To_Storage (0x004F9700)
<!-- corrected 2026-05-28: was named "HouseClass::DepositWeedCredits (0x006C9040)"; binary shows HouseClass__Add_Tiberium_To_Storage @ 0x004F9700 (0x006C9040 does not exist) via search_functions + decompile_function 0x004F9700 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->

```c
void Add_Tiberium_To_Storage(int amount, int ore_index) {
    while (amount > 0) {
        int maxStorage = RulesClass+0x17D0;
        float10 current = StorageClass__GetTotalAmount();
        if (current >= maxStorage) break;        // Storage full
        StorageClass__AddAmount(1.0f, ore_index);
        amount--;
    }
}
```

---

## VTABLE METHOD REFERENCE

### TechnoClass/UnitClass Virtual Methods (offsets from vtable base)

| Offset | Slot | Method | Description |
|--------|------|--------|-------------|
| +0x2C | 0x0B | GetRTTI | Returns type ID (1=Unit, 6=Building) |
| +0x3C | 0x0F | GetOwnerHouse | Returns HouseClass* |
| +0x48 | 0x12 | GetExitCoords | Returns CoordStruct* |
| +0x84 | 0x21 | GetTypeClass | Returns TypeClass* |
| +0xC4 | 0x31 | IsSelected | Returns bool |
| +0xD8 | 0x36 | PlaceAtCoords | Place object at coordinates |
| +0x124 | 0x49 | MarkForRedraw | Force visual update |
| +0x1B8 | 0x6E | GetCoords | Returns current position |
| +0x1BC | 0x6F | GetCenterCoords | Returns center position |
| +0x1C8 | 0x72 | GetAmmoCount | Returns ammo count |
| +0x1E4 | 0x79 | GetOwnerIndex | Returns owner index |
| +0x1E8 | 0x7A | SetMission | (mission_id, param) |
| +0x1EC | 0x7B | QueueMission | Queue next mission |
| +0x200 | 0x80 | ShouldScatter | Returns bool |
| +0x274 | 0x9D | ScanForTargets | (mode) |
| +0x2C0 | 0xB0 | GetSize | Returns unit size |
| +0x304 | 0xC1 | GetDockDirection | Returns dock facing |
| +0x314 | 0xC5 | CanDeploy | Returns bool |
| +0x3C0 | 0xF0 | FindPath | Pathfinding |
| +0x3C8 | 0xF2 | SetDestination | Set target cell |
| +0x3CC | 0xF3 | SetAltDestination | Alternate destination |
| +0x468 | 0x11A | CloseDockDoor | Close refinery door |
| +0x470 | 0x11C | UpdateGatherState | Update gather status |
| +0x480 | 0x120 | ScatterTo | Scatter to cell |
| +0x484 | 0x121 | ForceScatter | Force scatter |
| +0x4D4 | 0x135 | ForceEject | Force eject docked unit |
| +0x4D8 | 0x136 | NotifyDockEmpty | Notify dock empty |
| +0x500 | 0x140 | ForceStop | Stop movement |
| +0x544 | 0x151 | SetSpeed | Set move speed (double) |

### ILocomotion COM Interface (on param_1[0x19d])

| Offset | Method | Description |
|--------|--------|-------------|
| +0x04 | Is_Moving | Locomotor active? |
| +0x10 | Is_Moving_Now | Currently in motion? |
| +0x4C | Set_Desired_Heading | Set facing target |
| +0x58 | Stop | Halt movement |
| +0x70 | Head_To | Drive to coords with facing |
| +0x80 | Is_Turning | Currently turning? |
| +0x9C | Lock/Unlock | Lock movement |

---

## GLOBAL DATA REFERENCES

| Address/Name | Value | Description |
|--------------|-------|-------------|
| `g_RulesClass_Instance + 0x0244` | AnimTypeClass* | Smoke animation for undock |
| `g_RulesClass_Instance + 0x0F3C` | float | OrePurifierBonus multiplier |
| `g_RulesClass_Instance + 0x1324` | int* | Difficulty bonus array |
| `g_RulesClass_Instance + 0x1528` | double | **HarvesterDumpRate** |
| `g_RulesClass_Instance + 0x1700` | double | ConditionYellow threshold |
| `g_RulesClass_Instance + 0x17D0` | int | MaxWeedStorage |
| `DAT_007e27f8` | 900.0 (double) | Dump rate time multiplier constant |
| `FLOAT_007e1748` | 0.0f | Float epsilon (ore empty threshold) |
| `g_DirectionOffsets (0x89F680)` | short[16] | 8 direction X,Y offset pairs |
| `DAT_0089f6a0` | short[2] | Refinery adjacent cell offset |
| `DAT_00a8e3a8` | int[] | Mission timer table (indexed by mission ID) |
| `g_CurrentFrameCounter` | int | Global tick counter |
| `g_GameMode` | int | 0 = single player, nonzero = multiplayer |

## RETURN VALUES

| Value | Meaning |
|-------|---------|
| **1** | Call again next tick (active transitions, animations) |
| **5** | Short delay (facing alignment, approach driving) |
| **10** | Moderate delay (locomotor wait, cell blocked) |
| **random(0,2) + 14** | Guard mode for passive acquire |
| **timer + random(0,2)** | Variable from mission timer table |
| **0x1C2 (450)** | Fallback for non-harvester/non-deployer units |
