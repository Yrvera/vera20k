# UnitClass::Mission_Harvest Deep Dive

**Function:** `UnitClass::Mission_Harvest` at `0x73E5E0`  
**param_1 type:** `int*` (UnitClass pointer, offsets are array indices, multiply by 4 for byte offset)  
**Active in YR:** YES. Core harvesting logic for War Miner (HARV) and Chrono Miner (CMIN).  
**Confidence:** HIGH (verified from binary, cross-referenced with INI)

---

## Table of Contents

1. [Overview and Entry Conditions](#1-overview-and-entry-conditions)
2. [State Machine (States 0-4)](#2-state-machine-states-0-4)
3. [War Miner vs Chrono Miner Differences](#3-war-miner-vs-chrono-miner-differences)
4. [Helper Functions](#4-helper-functions)
5. [Key Struct Offsets](#5-key-struct-offsets)
6. [INI Constants](#6-ini-constants)
7. [Timer/Rate System](#7-timerrate-system)

---

## 1. Overview and Entry Conditions

### Pre-State-Machine Checks (before the switch)

1. **Slave Miner check** (0x73E5E9-0x73E612):
   - If `UnitTypeClass+0x5ED != 0` (IsASlaveOfSomething) AND `UnitTypeClass+0x5EC != 0`
     AND `UnitClass+0x2D8 != 0` (has slave owner): calls `FUN_006b0db0` (slave miner
     harvest logic) and jumps to timer exit. This is **Slave Miner** handling -- completely
     separate path from War/Chrono Miner.

2. **Not a harvester at all** (0x73E617-0x73E637):
   - If `UnitTypeClass+0xE0E == 0` (Harvester=no) AND `UnitTypeClass+0xE0F == 0` (Weeder=no):
     returns `0x1C2` (450 ticks = ~30 seconds). This means "I'm not a harvester, go idle."

3. **No refineries available** (0x73E638-0x73E6CE):
   - Reads `UnitTypeClass+0x3F8` (count of Dock entries in the type's dock list).
   - If count == 0 AND unit is NOT player-controlled: queues Mission_Guard (5) and returns 1.
   - If count > 0: iterates through each dock type, calls
     `HouseClass::CountOwnedInstances(dockType->DeployedBuilding)` to see if the owner has
     at least one refinery of that type. If none found for any dock type: queues Mission_Guard
     and returns 1.

4. **BL register (cVar1/BL)** is loaded at 0x73E6DE:
   ```asm
   MOV BL, byte ptr [EAX + 0xcd4]   ; EAX = UnitTypeClass*
   ```
   This reads `TechnoTypeClass+0xCD4` = **Teleporter** flag.
   - `BL = 0` for War Miner (HARV, Teleporter=no)
   - `BL = 1` for Chrono Miner (CMIN, Teleporter=yes)
   This flag persists across all states and controls the key behavioral differences.

5. **State switch** at 0x73E6EA: `switch(param_1[0x2F])` = `switch(UnitClass+0xBC)` on
   values 0-4, with default falling through to timer exit.

---

## 2. State Machine (States 0-4)

### State 0: SCAN FOR ORE

**Purpose:** Find an ore patch and start moving toward it.

**Flow:**

1. **Chrono Miner with full storage check** (only if Weeder=no):
   - If `UnitTypeClass+0xE0F == 0` (not a Weeder):
     - Calls `vt+0x2B4` = `UnitClass::Get_Storage_Percentage()` (0x7414A0)
     - If result >= 1.0 (storage full): sets state = 2 (RETURN), returns 1
   - This check does NOT happen for Weeders

2. **Clear ghost cell** and set `UnitClass+0x6D2 = 0` (harvesting flag cleared)

3. **If target cell exists** (`UnitClass+0x218` = warp/target != 0):
   - Moves to the target cell and clears it

4. **Ore scanning -- the WAR MINER vs CHRONO MINER split:**

   **If Weeder=yes** (`UnitTypeClass+0xE0F != 0`):
   - Calls `FootClass::Search_For_Tiberium_Short_And_Move` at 0x4DDB90
   - Uses `TiberiumLongScan` radius (rules+0x177C), converted from leptons to cells
   - This function uses `Scan_For_Tiberium_NoZone` internally (no zone check)
   - Jumps to LAB_0073E879 (transition to State 1 if found)

   **If Teleporter=yes (Chrono Miner, BL=1):**
   - Gets piggybacked locomotion via IID_IPersistStream QueryInterface (0x818858)
   - Calls `vt+0xC` (GetClassID) on the piggybacked locomotion
   - Compares the CLSID against `CLSID_TeleportLocomotion` at 0x7E9A90
     (`{4A582747-9839-11d1-B709-00A024DDAFD1}`)
   - If CLSID matches AND unit has a destination (`UnitClass+0x5A4 != 0`):
     clears the destination (set to 0, with "force move" flag)
   - Calls `FootClass::Search_For_Tiberium_And_Move` at 0x4DCFE0
   - Uses `TiberiumLongScan` radius (rules+0x177C)
   - Passes `local_50` (which was set to 1 or 0) as the zone parameter

   **If Teleporter=no (War Miner, BL=0):**
   - Checks `UnitClass+0x674` (locomotion COM pointer)
   - If locomotion is null: asserts
   - Gets the piggybacked locomotion via QueryInterface
   - Compares CLSID against TeleportLocomotion
   - If it's a teleport locomotor AND has a destination: clears destination
   - Calls `FootClass::Search_For_Tiberium_And_Move` (0x4DCFE0)
   - Uses `TiberiumLongScan` radius (rules+0x177C)

5. **Transition to State 1** (at LAB_0073E879, if ore found):
   - Sets `UnitClass+0x6D2 = 1` (harvesting flag)
   - Initializes the RateTimer:
     - `param_1[0x43]` (UnitClass+0x10C) = 2 (timer value)
     - `param_1[0x40]` (UnitClass+0x100) = g_CurrentFrameCounter (start frame)
     - `param_1[0x41]` (UnitClass+0x104) = local_24
     - `param_1[0x42]` (UnitClass+0x108) = 2
   - Sets `param_1[0x3E]` (UnitClass+0xF8) = 0 (step counter reset)
   - Sets state = 1

6. **If no ore found AND no destination:**
   - If has a warp target: moves to warp target
   - If no warp target: sets state = 4 (NO ORE), sets `UnitClass+0x3D0 = 1` (first-time flag)
   - If Harvester=yes: sets `HouseClass+0x242 = 1` (harvest-related house flag)
   - Returns 0x69 (105 ticks)

7. **If no ore found BUT has a destination:**
   - Clears the first-time flag: `UnitClass+0x3D0 = 0`

---

### State 1: HARVEST ORE

**Purpose:** Extract ore from the current cell, one bale at a time.

**Flow:**

1. **Initialize timer if needed** (at 0x73E931):
   - If `param_1[0x43]` (timer value at UnitClass+0x10C) == 0:
     - Resets step counter (`UnitClass+0xF8 = 0`)
     - Reads `HarvesterLoadRate` from rules (rules+0x1520)
     - Sets timer to HarvesterLoadRate
     - Records current frame counter

2. **Wait for 9 steps** (at 0x73E96F):
   - If `param_1[0x3E]` (UnitClass+0xF8, step counter) < 9: returns 1 (keep waiting)
   - Each timer expiry increments the step counter
   - So the harvester waits for `9 * HarvesterLoadRate` frames between extraction ticks

3. **Call Harvest_Ore_Tick** (0x73D450):
   - If returns true (successfully harvested): returns 1 (stay in state 1)
   - If returns false (cell empty or storage full):

4. **After failed harvest tick:**
   - Clears `UnitClass+0x6D2 = 0`

   **If Harvester=yes (UnitTypeClass+0xE0E):**
   - Checks storage percentage via `vt+0x2B4`
   - If storage == 1.0 (exactly full):
     - Sets state = 2 (RETURN)
     - Searches for more ore nearby using `TiberiumShortScan` radius (rules+0x1778):
       - If Harvester=yes: uses zone-aware `vt+0x338` (FootClass::Scan_For_Tiberium)
       - If Harvester=no (Weeder): uses `FootClass::Scan_For_Tiberium_NoZone`
     - If ore found: sets ghost cell to that ore patch (so it remembers where to come back)
     - Returns 1
   - If storage < 1.0 (not full):
     - Searches for more ore using `TiberiumShortScan` radius
     - If no ore found AND no destination: sets ghost cell=0, state=2 (go return even if not full)
     - If ore found: stays in state 1 with harvesting flag set

   **If Harvester=no (Weeder):**
   - Similar logic but uses `Search_For_Tiberium_Short_And_Move` (no zone check)

---

### State 2: RETURN TO REFINERY

**Purpose:** Find a refinery and either drive or teleport to it.

**Flow:**

1. **Stop if already docked** (at 0x73EB2C):
   - If `UnitClass+0x5A4` (destination) != 0 AND Teleporter=yes (BL != 0):
     - Calls `vt+0x528` (Find_Docking_Bay) with `UnitTypeClass + 0x3E8` (1000 = dock list offset)
     - If found a bay: calls `FootClass::Stop_Moving` (clears destination)

2. **If destination exists:** jump to default timer exit

3. **Find a refinery** (first attempt, normal pathfinding):
   - Calls `vt+0x528` (Find_Docking_Bay) with `(UnitTypeClass + 0x3E8, 0, 0)`

4. **Distance check -- THE KEY WAR MINER vs CHRONO MINER DIFFERENCE:**

   **War Miner (BL=0, Teleporter=no):**
   - If refinery found:
     - Calculates 3D Euclidean distance between unit and refinery (in leptons)
     - Compares against `HarvesterTooFarDistance * 256` (rules+0xD78, default 5*256=1280 leptons)
     - If distance <= threshold: attempts to dock (calls `vt+0x278` = `RadioClass::Transmit_Radio(2, refinery)`)
     - If dock accepted (returns 1): sets state = 3 (DOCK/ENTER)

   **Chrono Miner (BL=1, Teleporter=yes):**
   - If refinery found:
     - Calculates 3D Euclidean distance
     - Compares against `ChronoHarvTooFarDistance * 256` (rules+0xD7C, default 50*256=12800 leptons)
     - If distance <= threshold: attempts to dock, same as War Miner

5. **If too far or no refinery (first attempt):**
   - Increments `g_MapEditorMode` (disables pathfinding constraints)
   - Calls `vt+0x528` again with `(UnitTypeClass + 0x3E8, 0, 1)` (param4=1 = any refinery)
   - Decrements `g_MapEditorMode`
   - If found a refinery on this second attempt:
     - Calculates distance again
     - If distance > 0x300 (768 leptons = 3 cells) OR Teleporter=yes (Chrono Miner):
       - Calculates a cell near the refinery entrance:
         - Gets refinery's coordinates, converts to cell
         - Adds `BuildingTypeClass+0x1618` (ExitX) and `BuildingTypeClass+0x161C` (ExitY) offsets
         - Calls `FootClass::Find_Nearby_Passable_Cell` to find a reachable cell near the exit
         - If found: sets destination to that cell via `vt+0x480` (TechnoClass::Set_Destination)
         - If not found: clears destination

     - If distance <= 768 AND NOT Teleporter: falls through to default timer exit

6. **If no refinery found at all:** falls through to default timer exit

**Key insight:** The distance thresholds differ dramatically:
- War Miner: 5 cells (1280 leptons) -- must be very close to dock directly
- Chrono Miner: 50 cells (12800 leptons) -- can teleport from far away to dock

---

### State 3: DOCK / ENTER REFINERY

**Purpose:** Hand off to Mission_Enter.

**Flow:**
- Calls `vt+0x1E8` = `MissionClass::Queue_Mission(7, 0)` -- Mission 7 = Mission_Enter
- Returns 1

This is trivially simple -- the actual docking/unloading is handled by Mission_Enter (0x73A340).

---

### State 4: NO ORE FOUND

**Purpose:** Handle the "no ore anywhere" situation.

**Flow:**

1. **If first-time flag set** (`UnitClass+0x3D0 != 0`):
   - Calls `vt+0x528` (Find_Docking_Bay) with `(rules+0x850, 0, 1)` --
     this searches for **Repair Bay** (rules+0x850 = RepairBay type list)
   - If repair bay found: queues Mission 0x14 (20 = Mission_Repair)
   - If no repair bay: queues Mission 0x0F (15 = Mission_Area_Guard)

2. **Check current cell for refinery/dock:**
   - Gets the building at the harvester's current cell
   - If there's a building AND it has `BuildingTypeClass+0x16BB != 0` (Refinery flag)
     OR `BuildingTypeClass+0x16BC != 0` (Dock flag):
     - Sets destination to a cell near that building's exit
     - (Essentially "move away from this refinery's entrance")

3. Queues `Mission_Guard` (5) and exits

---

## 3. War Miner vs Chrono Miner Differences

| Aspect | War Miner (HARV) | Chrono Miner (CMIN) |
|--------|-------------------|---------------------|
| `Teleporter` flag (TechnoType+0xCD4) | `false` | `true` |
| `Locomotor` | DriveLocomotionClass | TeleportLocomotionClass (piggybacked on DriveLocomotionClass) |
| State 0 scan radius | TiberiumLongScan (48 cells) | TiberiumLongScan (48 cells) |
| State 0 scan function | `Scan_For_Tiberium_And_Move` | `Scan_For_Tiberium_And_Move` |
| State 0 locomotion check | Checks if piggybacked loco is Teleport | Same check |
| State 2 distance threshold | `HarvesterTooFarDistance * 256` = 1280 leptons (5 cells) | `ChronoHarvTooFarDistance * 256` = 12800 leptons (50 cells) |
| State 2 far-refinery behavior | Drives toward refinery exit cell if > 3 cells | Always moves toward exit cell if too far |
| Storage | 40 bales | 20 bales |
| Has weapon | Yes (20mmRapid turret) | No |

### The locomotion CLSID comparison (State 0)

Both War Miner and Chrono Miner go through the same code path in State 0. The CLSID check
at 0x73E818-0x73E82A compares the piggybacked locomotion's CLSID against:

```
CLSID_TeleportLocomotion = {4A582747-9839-11d1-B709-00A024DDAFD1}
```
(stored at 0x7E9A90 in the binary)

For the Chrono Miner, the primary locomotion IS TeleportLocomotionClass (which piggybacks
DriveLocomotionClass). The QueryInterface for IID_IPersistStream on the locomotion gets the
piggybacked drive locomotion, and the CLSID check on THAT would NOT match TeleportLocomotion
since the piggybacked loco is Drive.

Actually, re-reading the code flow: the code calls `FUN_0045a050()` which dereferences the
locomotion pointer, then does `QueryInterface(IID_IPersistStream, &outPtr)` on it, then
calls `GetClassID` on the result. The IID_IPersistStream at 0x818858 =
`{0000010C-0000-0000-C000-000000000046}`.

The purpose of this check: if the unit's locomotion IS the teleport locomotion, and it
already has a destination set, clear the destination before scanning for ore. This prevents
a teleport-in-progress from conflicting with the ore search.

---

## 4. Helper Functions

### 4.1 FootClass::Scan_For_Tiberium (0x4DD0A0)

**param_1 type:** `int*` (FootClass pointer)  
**param_2:** `int` -- scan radius in cells  
**Returns:** CellStruct (packed X,Y in return value via stack)

**Algorithm: Diamond Spiral Scan**

1. Get unit's current coordinates, convert to cell (lepton >> 8)
2. Check current cell first:
   - If `CellClass+0xEC == 5` (contains tiberium overlay): return current cell immediately
3. Iterate rings from radius 1 to `param_2 - 1`:
   - For each ring at distance `r`, iterate from `inner_offset` to `r`:
     - Check 4 cells per iteration (one in each quadrant of the diamond):
       - `(centerX + col, centerY - r)` -- top
       - `(centerX + col, centerY + r)` -- bottom
       - `(centerX - r, centerY + col)` -- left
       - `(centerX + r, centerY + col)` -- right
     - For each cell: calls `FootClass::Is_Cell_Harvestable`
     - If harvestable: calls `CellClass::Get_Tiberium_Value` to get ore value
     - Tracks the **highest value** cell found so far
   - **Early termination:** if ANY harvestable cell is found in a ring, stops scanning
     further rings and returns the best cell found in that ring
   - `inner_offset` starts at -1 and decrements by 1 each ring (expanding the inner scan)

**Key detail:** The algorithm scans the entire ring before stopping, picking the
highest-value cell within the first ring that contains any ore. It does NOT simply
return the first ore cell found.

### 4.2 FootClass::Scan_For_Tiberium_NoZone (at footclass, similar to 0x4DD0A0)

Same diamond spiral but uses `FootClass::Is_Cell_Weedable` instead of
`FootClass::Is_Cell_Harvestable`. No zone connectivity check. Used for Weeders.

### 4.3 FootClass::Is_Cell_Harvestable (0x4DCE10 approx)

**Checks:**
1. Cell must be in playfield (`MapClass::Is_Cell_In_Playfield`)
2. If in campaign mode AND unit is player-controlled: cell must not be shrouded
3. Cell must be reachable via zone pathfinding (`MapClass::Can_Reach_Zone`)
4. Cell must have tiberium overlay (`CellClass+0xEC == 5`)
5. Cell must be enterable by the unit (`vt+0x1AC` = `Can_Enter_Cell` returns 0 = OK)

### 4.4 FootClass::Search_For_Tiberium_And_Move (0x4DCFE0)

**Wrapper function.** Calls `vt+0x338` (Scan_For_Tiberium) with the given radius and
zone parameter. If ore found and unit isn't at that cell already: moves unit to that cell
via `vt+0x480` (Set_Destination). Returns true if ore was found.

### 4.5 FootClass::Search_For_Tiberium_Short_And_Move (0x4DDB90)

**Wrapper function.** Calls `FootClass::Scan_For_Tiberium_NoZone` with the given radius.
Same move-to logic as above. Used for Weeders.

### 4.6 FootClass::Find_Docking_Bay (0x4DF040)

**param_1 type:** `int*` (FootClass pointer)  
**param_2:** dock type list pointer (e.g., UnitTypeClass+0x3E8 = Dock list)  
**param_3, param_4:** passed to `vt+0x52C` (building-specific search)

**Algorithm:**
1. Iterates through the dock type list (array of building type pointers at param_2+4,
   count at param_2+0x10)
2. For each dock type: calls `vt+0x52C` which searches all buildings of that type
3. Tracks the building with the **shortest distance** (param_2 output from the search)
4. Returns the best (closest) building found, or 0 if none

### 4.7 UnitClass::Harvest_Ore_Tick (0x73D450)

**param_1 type:** `int*` (UnitClass pointer)  
**Returns:** bool (1 = harvested successfully, 0 = failed/empty)

**Flow:**

1. Get current cell from unit coordinates
2. If unit has a destination (`UnitClass+0x5A4 != 0`): return 1 (still moving, not ready)

3. **If Harvester=yes (UnitTypeClass+0xE0E != 0):**
   - Check storage: if `Get_Storage_Percentage() >= 1.0`: reset timer, return 0 (full)
   - Check cell: if `CellClass+0xEC != 5` (no tiberium): reset timer, return 0 (empty)
   - **If Weeder=yes (UnitTypeClass+0xE0F != 0):**
     - Calls `FUN_00486E30` (weed-specific harvest)
     - Adds 1.0 of type 0 to storage: `StorageClass::AddAmount(1.0f, 0)`
     - Resets timer to `HarvesterLoadRate * 3`
     - Returns 1
   - **Normal ore (Weeder=no):**
     - Calls `FUN_00485010` to get the tiberium type index from the overlay
     - Gets `TechnoTypeClass+0x800` (Storage capacity)
     - Gets `StorageClass::GetTotalAmount()` -- current amount stored
     - Calculates `remaining = Storage - currentAmount`
     - If remaining <= epsilon: already full, return 0
     - Otherwise: `amountToExtract = ftol(remaining)` (integer floor of remaining space)
     - Calls `CellClass::Reduce_Tiberium(amountToExtract)` -- returns actual amount extracted
     - If extracted > 0: `StorageClass::AddAmount(extracted, tibType)`, resets timer to
       HarvesterLoadRate, returns 1
     - If extracted == 0: return 0

### 4.8 CellClass::Reduce_Tiberium (0x480A80)

**param_1 type:** `CellClass*` (Ghidra shows it as the `this` pointer)  
**param_2:** `uint` -- amount to remove

**Flow:**
1. Gets overlay data (which tiberium type is on this cell)
2. Reads `CellClass+0x11E` = current tiberium level (0-11, byte value)
3. If tiberium level is 11 (0x0B = maximum): calls `FUN_007235A0` (chain reaction / spread check)
4. If `param_2 < (currentLevel + 1)`:
   - Decrements level by param_2: `field_0x11E -= param_2`
   - Returns param_2 (full amount extracted)
5. If `param_2 >= (currentLevel + 1)`:
   - Removes ALL tiberium from cell:
     - Sets `OverlayTypeIndex = -1`
     - Sets `field_0x11E = 0`
     - Calls `CellClass::RecalcAttributes`
     - Marks radar dirty
     - Checks 8 neighboring cells for orphaned tiberium graphics
   - Returns `currentLevel` (the pre-existing amount, which was < requested)

**Tiberium value model:** Each cell has a tiberium level from 0-11. The "value" used by
`Get_Tiberium_Value` is `overlayType->Value * (level + 1)`.

### 4.9 StorageClass Functions

**StorageClass** is a simple 4-slot float array (one slot per tiberium type).

- `AddAmount(float amount, int type)` at labeled address:
  - `storage[type] += amount` -- direct float addition
- `GetAmount(int type)` at 0x6C9680:
  - Returns `storage[type]` as float
- `FindFirstNonEmpty()` at 0x6C9820:
  - Iterates slots 0-3, returns first index where `storage[i] > 0.0f`, or -1
- `GetTotalAmount()`: sums all 4 slots

### 4.10 FootClass::Stop_Moving (0x4DF0D0)

Clears `FootClass+0x5A0 = 0` and `FootClass+0x5A4 = 0` (head/next waypoint and destination).

---

## 5. Key Struct Offsets

### UnitClass (param_1 is int*, multiply indices by 4 for byte offsets)

| Offset (byte) | Index | Field | Description |
|---------------|-------|-------|-------------|
| 0x9C | 0x27 | Coords.X | Unit X coordinate (leptons) |
| 0xA0 | 0x28 | Coords.Y | Unit Y coordinate (leptons) |
| 0xA4 | 0x29 | Coords.Z | Unit Z coordinate (leptons) |
| 0xBC | 0x2F | HarvestState | State machine variable (0-4) |
| 0xF8 | 0x3E | StepCounter | Counts timer expirations in State 1 |
| 0x100 | 0x40 | RateTimer.StartFrame | Frame when timer was set |
| 0x104 | 0x41 | RateTimer.? | Timer field 2 |
| 0x108 | 0x42 | RateTimer.Value | Timer current value |
| 0x10C | 0x43 | RateTimer.Duration | Timer max/reload value |
| 0x218 | 0x86 | WarpTarget | Ghost/warp target cell (0 = none) |
| 0x21C | -- | HousePtr | Owner house pointer |
| 0x2D8 | 0xB6 | SlaveOwner | Slave owner pointer |
| 0x3D0 | 0xF4 | FirstTimeFlag | First-time-no-ore flag (byte) |
| 0x5A0 | 0x168 | HeadWaypoint | Current waypoint |
| 0x5A4 | 0x169 | Destination | Current destination target |
| 0x674 | 0x19D | Locomotion | Locomotion COM interface ptr |
| 0x6C4 | 0x1B1 | TypePtr | Pointer to UnitTypeClass |
| 0x6D2 | -- | IsHarvesting | 1 when actively harvesting (byte) |

### UnitTypeClass / TechnoTypeClass (accessed via iVar8 = TypePtr, byte offsets)

| Offset (byte) | Field | INI Key |
|---------------|-------|---------|
| 0x398 | PipCount | (set to 10 if Harvester/Weeder, else 15) |
| 0x3EC | DockList.Items | Array of dock type pointers |
| 0x3F8 | DockList.Count | Number of dock types |
| 0x5EC | IsASlaveOf | Related to slave mechanics |
| 0x5ED | SlaveFlag | Related to slave mechanics |
| 0x800 | Storage | `Storage=` (int, max bales) |
| 0xCD4 | Teleporter | `Teleporter=` (bool byte) |
| 0xE0E | Harvester | `Harvester=` (bool byte) |
| 0xE0F | Weeder | `Weeder=` (bool byte) |

### CellClass (byte offsets, direct)

| Offset | Field | Description |
|--------|-------|-------------|
| 0xEC | OverlayData | 5 = tiberium overlay present |
| 0x11E | TibLevel | Tiberium amount (0-11) |
| 0x11D | OverlayTypeIndex | Overlay type (-1 = none) |

### BuildingTypeClass (byte offsets from building's type pointer)

| Offset | Field | Description |
|--------|-------|-------------|
| 0x1618 | ExitX | Exit cell X offset (short) |
| 0x161C | ExitY | Exit cell Y offset (short) |
| 0x16BB | IsRefinery | Refinery flag (bool byte) |
| 0x16BC | IsDock | Dock flag (bool byte) |

---

## 6. INI Constants

### RulesClass Offsets

| Offset | INI Key | Default | Description |
|--------|---------|---------|-------------|
| 0xD78 | `HarvesterTooFarDistance` | 5 | Cells. War Miner dock threshold |
| 0xD7C | `ChronoHarvTooFarDistance` | 50 | Cells. Chrono Miner dock threshold |
| 0x1520 | `HarvesterLoadRate` | ? | Frames per timer tick in harvest state |
| 0x1778 | `TiberiumShortScan` | 6 | Cells. Nearby ore scan radius |
| 0x177C | `TiberiumLongScan` | 48 | Cells. Far ore scan radius |
| 0x1780 | `SlaveMinerShortScan` | ? | Cells. Slave miner nearby scan |
| 0x1784 | `SlaveMinerSlaveScan` | ? | Cells. Slave scan |
| 0x1788 | `SlaveMinerLongScan` | ? | Cells. Slave miner far scan |
| 0x178C | `SlaveMinerScanCorrection` | ? | Cells. Slave correction |
| 0x1790 | `SlaveMinerKickFrameDelay` | ? | Frames. Slave kick delay |

**Note:** TiberiumShortScan and TiberiumLongScan are read via `FUN_00474620` which reads
a lepton value (cell count * 256).

### Scan Radius Usage Summary

| Context | Radius Used | Value |
|---------|------------|-------|
| State 0 initial scan | TiberiumLongScan | 48 cells |
| State 1 re-scan after harvest (short) | TiberiumShortScan | 6 cells |
| State 1 re-scan after full harvest | TiberiumShortScan | 6 cells |
| State 0 Weeder scan | TiberiumLongScan | 48 cells |

---

## 7. Timer/Rate System

The harvest timer uses the shared StepTimer cluster at UnitClass offsets
`0xF8..0x110`:

```
+0x0F8 (0x3E): Counter        -- accumulated steps
+0x100 (0x40): StartFrame     -- frame when timer was last set
+0x104 (0x41): Companion      -- exact meaning outside this report
+0x108 (0x42): Duration       -- current frame duration
+0x10C (0x43): RepeatDuration -- recurring rate; zero triggers state-1 init
+0x110       : StepAmount     -- added to Counter on expiry; constructed as 1
```

### Harvest Timing

1. **State 0 -> State 1 transition:** The successful search-and-move path
   initializes counter `0`, start frame `F`, duration/repeat `2`, and substate
   `1` immediately; this is not a physical-arrival event.
2. **State 1 initialization when repeat is zero:** Reloads duration/repeat from
   `HarvesterLoadRate`.
3. **Step counting:** Each expiry adds `+0x110` (stock `1`) to counter
   `+0xF8`; the counter must be at least `9` when Mission_Harvest reads it.
4. **After every successful standard harvest tick:** `Harvest_Ore_Tick`
   explicitly resets counter `+0xF8=0`, start frame to current, and
   duration/repeat to `HarvesterLoadRate`.
5. **Weeder harvest:** Reloads with `HarvesterLoadRate * 3` (3x slower).

`TechnoClass::AI_Update` calls `Mission_Dispatch` at `0x006FA655` before the
timer maintenance at `0x006FABC4..0x006FAC22`. With stock
`HarvesterLoadRate=2`, the ninth increment is written after the mission call at
`F+18`; the mission first observes counter `9` and calls
`Harvest_Ore_Tick` at `F+19`. Successful standard extraction resets the
counter, so the next helper observation follows the same `F+19` sequence.

### Return Value

The function's return value is the delay before next call:
- Most paths: returns 1 (called again next frame)
- State 0 no-ore: returns 0x69 = 105 frames
- Not-a-harvester: returns 0x1C2 = 450 frames
- Default: `MissionTimerEntry * constant + Random(0,2)` -- the mission-specific timer

---

## 8. Slave Miner Path (FUN_006B0DB0)

This is NOT part of the War Miner / Chrono Miner flow but is called when
`UnitTypeClass+0x5ED` and `+0x5EC` are set (slave miner flags).

The function at 0x6B0DB0 handles the slave miner's harvest behavior differently:
- Checks if the slave's owner is a UnitClass (type check via vtable method 0x2C == 1)
- If owner has a deploy target (`owner+0x5A4` field): finds a passable cell near
  the deploy target and sends the slave there with Mission_Harvest (2)
- If owner is a BuildingClass (0x2C == 6): checks `owner+0x218` for a building,
  gets that building's coordinates, and sends the slave there
- Otherwise: sends the slave to Mission_Guard

This is entirely separate logic and should NOT be mixed with HARV/CMIN behavior.
