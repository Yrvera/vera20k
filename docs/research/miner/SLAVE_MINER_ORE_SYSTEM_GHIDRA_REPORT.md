# Slave Miner & Ore System — Ghidra Research Report

## 1. Slave Miner Deploy/Undeploy

### INI Key Parsing (TechnoTypeClass::ReadINI @ 0x00710000+)

| INI Key        | TechnoTypeClass Offset | Type       | Lookup Function |
|----------------|----------------------|------------|-----------------|
| `DeploysInto`  | 0x404 (`[0x101]`)    | BuildingTypeClass* | `FUN_004653c0` (BuildingTypeClass::Find) |
| `UndeploysInto`| 0x408 (`[0x102]`)    | UnitTypeClass*     | `FUN_007480d0` (UnitTypeClass::Find) |
| `DeployingAnim`| 0x6BC (`[0x1af]`)    | AnimTypeClass*     | `FindAnimType_by_name` |

**Confidence: HIGH** — directly from decompiled TechnoTypeClass::ReadINI at addresses 0x713270-0x7132cc.

### Deploy Mechanism (UnitClass::Deploy @ 0x007393c0)

The deploy sequence:

1. **Check CanDeploy** (vtable 0x314) — verifies deploy preconditions
2. **Face correct direction** — calls `Deploy_facing_calculator` (0x00465d70), rotates unit to match `DeployFacing` INI key before deploying
3. **Create BuildingClass** — `operator_new(0x720)` then `BuildingClass::Constructor` with the `DeploysInto` BuildingTypeClass
4. **Place building** — calls vtable 0xd8 (TryPlaceBuilding) at the unit's cell coordinates
5. **Transfer properties**:
   - Copies `UniqueID` from unit to building
   - Copies `Location_Z` (height)
   - Transfers health: `ObjectClass::GetHealthRatio(unit)` -> `Math::ftol` -> sets `building->Health`
   - Copies 5 dwords starting at field 0x1E0 (experience/veterancy data)
   - Transfers `field_0x1EC` and `field_0x1F0` (presumably rally point/linking data)
   - If unit has AttachedTag, transfers it to building (with refcount management)
6. **Update targeting** — iterates all TechnoClass objects, redirecting any that targeted the unit to now target the building (unless it's a deploy-immune type)
7. **Remove unit** — calls vtable 0xF8 (RemoveFromMap) and vtable 0x3A0 (Destroy/Limbo)
8. **MCV special** — if `IsDeployable` (offset 0x16b9 in BuildingTypeClass), sets up base deployment (center-view, construction yard flags)

**Key finding**: The SlaveManager is NOT explicitly transferred in UnitClass::Deploy — it lives at TechnoClass offset 0x2D8 and the building gets its own SlaveManager via `FUN_006f3f40` during TechnoClass initialization. Both SMIN (unit) and YAREFN (building) have `Enslaves=SLAV` and `SlavesNumber=5` in their INI sections. The comment in rulesmd.ini says "Brain transplant will check to make sure extra one is not created."

**Confidence: HIGH** — decompiled from UnitClass::Deploy at 0x007393c0.

### Mission_Deploy_Building (@ 0x0073d630)

This is the *building*-style deploy handler for the Slave Miner once deployed. It handles the slave miner's ore dumping cycle as a building:

- **State 0**: Initial state, checks if unit has DeploysInto
- **State 1**: Rotating to face direction
- **State 2**: Deploy animation playing
- **State 3**: Ore dumping — reads from storage at `RulesClass+0x1528` (HarvesterDumpRate), calculates ore value per tick, calls `FUN_004f9610` to deposit credits to house
- **State 4**: Dump complete, transitions back to harvest

The ore value calculation during dumping (state 3) uses:
- `g_RulesClass_Instance + 0x1528` = HarvesterDumpRate (double)
- `FUN_006c9680(iVar3)` — reads storage amount for tiberium type `iVar3`
- `FUN_006c96b0(amount, iVar3)` — decrements storage for tiberium type
- `FUN_004f9610(amount, iVar3)` — deposits credits to house

**Confidence: MEDIUM-HIGH** — the decompilation is complex with many vtable calls, but the overall flow is clear.

---

## 2. Slave System (SlaveManagerClass)

### SlaveManagerClass Layout

| Offset | Field | Notes |
|--------|-------|-------|
| 0x00   | vtable pointer | Primary vtable at 0x007f31c8 |
| 0x04   | vtable secondary 1 | |
| 0x08   | vtable secondary 2 | |
| 0x0C   | vtable secondary 3 | |
| 0x24   | Owner (TechnoClass*) | The master (SMIN unit or YAREFN building) |
| 0x28   | ? | |
| 0x2C   | SlaveCount (int) | From `SlavesNumber` INI key |
| 0x30   | SlaveRegenRate (int) | From `SlaveRegenRate` INI key |
| 0x34   | SlaveReloadRate (int) | From `SlaveReloadRate` INI key |
| 0x38   | ? | |
| 0x3C   | SlaveControl* array ptr | DynamicVectorClass of SlaveControl structs |
| 0x48   | SlaveControl count (int) | Number of entries in array |
| 0x50   | Timer_Start (int) | Frame counter for rate timer |
| 0x54   | Timer_Data (int) | |
| 0x58   | Timer_Duration (int) | Default = 10 frames |
| 0x5C   | State (int) | Manager state: 0=Ready, 1=?, 2=Moving, 4=Freeze, 6=Relocating |
| 0x60   | StateTimer (int) | Frame counter for state changes |

### SlaveControl Struct (0x14 = 20 bytes)

| Offset | Field | Notes |
|--------|-------|-------|
| 0x00   | Slave (InfantryClass*) | Pointer to the slave infantry unit |
| 0x04   | State (int) | Slave state (see state machine below) |
| 0x08   | Timer_Start (int) | Frame counter |
| 0x0C   | Timer_Data (int) | |
| 0x10   | Timer_Duration (int) | Regen timer countdown |

**Confidence: HIGH** — from constructor at 0x006af1a0 and AI function at 0x006af6c0.

### INI Key Parsing

| INI Key          | TechnoTypeClass Offset | Notes |
|------------------|----------------------|-------|
| `Slaved`         | 0xD3E (bool)         | Marks a unit type as a slave |
| `Enslaves`       | 0xD40 (`[0x350]`)    | InfantryTypeClass* — the slave infantry type |
| `SlavesNumber`   | 0xD44 (`[0x351]`)    | int — number of slaves |
| `SlaveRegenRate`  | 0xD48 (`[0x352]`)    | int — frames before dead slave respawns |
| `SlaveReloadRate` | 0xD4C (`[0x353]`)    | int — frames between reload ticks |

These are parsed at addresses 0x714dc2-0x714e4f in TechnoTypeClass::ReadINI.

**Confidence: HIGH** — directly from decompilation.

### SlaveManager Creation (FUN_006f3f40 — TechnoClass Init)

At TechnoClass offset 0x2D8 (`param_1[0xb6]`), the SlaveManager is created if the type has `Enslaves` set (TechnoTypeClass offset 0xD40 != 0):

```c
if (*(int *)(typeClass + 0xd40) != 0) {
    pvVar2 = operator_new(100);  // sizeof(SlaveManagerClass) = 0x64 = 100
    SlaveManagerClass::Constructor(
        owner,                    // this TechnoClass
        *(typeClass + 0xd40),     // Enslaves type
        *(typeClass + 0xd44),     // SlavesNumber
        *(typeClass + 0xd48),     // SlaveRegenRate
        *(typeClass + 0xd4c)      // SlaveReloadRate
    );
    param_1[0xb6] = result;  // Store at TechnoClass+0x2D8
}
```

**Confidence: HIGH** — from decompiled FUN_006f3f40 at 0x006f3f40.

### SlaveManager Constructor (0x006af1a0)

The constructor:
1. Creates a DynamicVectorClass for SlaveControl entries
2. Loops `SlavesNumber` times:
   - Allocates SlaveControl (0x14 bytes via `operator_new`)
   - Gets the owner's HouseClass (vtable 0x3c)
   - Creates a new InfantryClass of the Enslaves type via HouseClass::CreateInfantry (vtable 0x8c)
   - Sets the slave's `field_0x2DC` to point back to the master (offset 0x2DC in InfantryClass)
   - Sets SlaveControl state to 0 (Ready)
   - Adds to the DynamicVectorClass
3. Registers in a global SlaveManager array

### SlaveManager Update (vtable[23] = 0x006af5f0)

Called from TechnoClass::AI_Update at offset 0x2D8:
```c
if (*(int **)&param_1->field_0x2D8 != NULL) {
    (**(code **)(**(int **)&param_1->field_0x2D8 + 0x5c))();
}
```

The update function:
1. Checks a rate timer (default 10 frames between updates)
2. If timer expired, calls:
   - `FUN_006af6c0()` — Slave AI state machine (per-slave processing)
   - `UnitClass__Mission_Deploy()` — Manager-level state (but actually SlaveManager::UpdateState)

### Slave AI State Machine (FUN_006af6c0 @ 0x006af6c0)

Each slave has a state tracked in `SlaveControl[1]` (offset 0x04). The states are:

| State | Name | Behavior |
|-------|------|----------|
| 0 | Ready | Slave is idle at master, ready to harvest |
| 1 | ScanForOre | Calls `TechnoClass::ScanForTiberium` (vtable 0x338) with range `RulesClass+0x1784` (TiberiumShortScan). If ore found, moves slave to ore cell (state 2). If no ore, moves slave back to master (state 4). |
| 2 | MovingToOre | Checks `FUN_00487df0` (cell land type == 5 = Tiberium). If arrived at tiberium cell, calls `FUN_00522d00` (set Mission_Harvest) -> state 3. If arrived but no tiberium, back to state 1. |
| 3 | Harvesting | Checks `FUN_00522d30` (health ratio >= 1.0 = fully loaded). If full, gets master's cell, moves slave back to master cell (state 4). Also checks `FUN_00522fc0` (current mission == Harvest). If not harvesting anymore, back to state 1. |
| 4 | ReturningToMaster | Slave is moving back. When it arrives at master's cell, checks if target has wandered too far (`RulesClass+0xDF8`). If at master: calls `FUN_00522d50` (deposit ore to master), `vtable 0xd4` (enter/dock), sets regen timer from `SlaveRegenRate` -> state 5. If too far from master, re-scan (state 1). |
| 5 | Regenerating | Waits for `SlaveRegenRate` timer to expire. When done, restores slave health to max (`type->Strength`) and sets state 0 (Ready). |
| 6 | Dead | Slave was killed. Waits for `SlaveReloadRate` timer, then calls `FUN_006af650` to respawn the slave. |

**Confidence: HIGH** — directly decompiled from FUN_006af6c0.

### Slave Death / Master Destroyed (FUN_006b0ae0 @ 0x006b0ae0)

When the master is destroyed:
1. Iterates all living slaves
2. For each slave with `IsAlive` and not in limbo:
   - Clears the slave's back-reference (offset 0x2DC = 0)
   - If no specific attacker: calls vtable 0x16c to kill the slave with `RulesClass+0xFA8` (death warhead — this creates the "InfantryElectrocuted" death animation visually, though the string isn't stored as a named constant)
   - If attacker exists: changes slave's owner to neutral/attacker house, sets mission to Guard, makes slave "free"
3. Plays a sound effect if any slaves survived (`RulesClass+0x234` anim index, via `FUN_007509e0`)
4. Clears the owner pointer

**Confidence: HIGH** — from decompiled FUN_006b0ae0.

### Slave Respawn (FUN_006af650 @ 0x006af650)

When a dead slave's `SlaveReloadRate` timer expires (state 6), `FUN_006af650` is called:

1. Gets the Enslaves InfantryTypeClass from SlaveManager offset 0x28
2. Gets the master's HouseClass (vtable 0x3c)
3. Creates a new InfantryClass via `HouseClass::CreateInfantry` (vtable 0x8c)
4. Calls vtable 0xd4 on the new slave (enter/dock with master)
5. Sets `slave+0x2DC` = master pointer (back-reference)
6. Resets SlaveControl: state=0, timer reset, duration=0

**Confidence: HIGH** — directly decompiled.

---

## 3. Ore Growth System

### Global Controls

| INI Key (Scenario) | ScenarioClass Offset | Notes |
|---------------------|---------------------|-------|
| `TiberiumGrowthEnabled` | 0x34a6 (bool) | Per-scenario toggle |
| `TiberiumGrows` (SpecialFlags) | bit 6 of SpecialFlags | Multiplayer game option |

| INI Key (General) | RulesClass Offset | Type | Notes |
|-------------------|-------------------|------|-------|
| `GrowthRate` | 0x1638 (double) | Rate multiplier for ore growth timer |

**Confidence: HIGH** — from RulesClass::ReadGeneral (0x670d69) and FUN_006b8b30.

### Per-Tiberium Type Growth Settings

From TiberiumClass::ReadINI (FUN_00721a90):

| INI Key | TiberiumClass Offset | Type | Notes |
|---------|---------------------|------|-------|
| `Growth` | 0xA8 | int | Growth interval in frames |
| `GrowthPercentage` | 0xB0 | double | Chance of growth per interval |
| `Spread` | 0x9C | int | Spread interval in frames |
| `SpreadPercentage` | 0xA0 | double | Chance of spread per interval |
| `Value` | 0xB8 | int | Credit value per bail |
| `Power` | 0xBC | int | Explosive power per bail |
| `Color` | 0xC0 | int | Display color index |
| `Image` | switch at 0x721c55 | int | 1=small ore, 2=gems, 3=vine, 4=type4 |
| `Debris` | 0xC4+ (DynVec) | AnimType* list | Crystal debris animations |

**Image overlay mapping** (from switch at 0x721c55):
- Image=1 (ore): overlay type from `DAT_00a83d84 + 0x198` — 12 frames each for density + 12 for visual variant
- Image=2 (gems): overlay type from `DAT_00a83d84 + 0x6C` — 12 frames
- Image=3 (vine): overlay type from `DAT_00a83d84 + 0x1FC`
- Image=4: overlay type from `DAT_00a83d84 + 0x24C`

The TiberiumClass also stores: `0xE0` = OverlayTypeClass* pointer, `0xE4` = NumGrowthFrames (=12), `0xE8` = NumSpreadFrames (=12), `0xEC` = NumDensityLevels (=8, but set to 12 for vine/type2)

**Confidence: HIGH** — from decompiled FUN_00721a90.

### Overlay-to-Tiberium Mapping (IsWallOverlay @ 0x005fdd00)

The function iterates all TiberiumClass instances (stored in global vector at `DAT_00b0f4ec`, count at `DAT_00b0f4f8`):

```c
for each TiberiumClass tib:
    overlayStart = tib->OverlayType->ArrayIndex   // tib+0xE0 -> +0x294
    numGrowth = tib->NumGrowthFrames               // tib+0xE8
    numSpread = tib->NumSpreadFrames                // tib+0xEC

    if (overlayIndex >= overlayStart && overlayIndex < overlayStart + numGrowth)
        return tib->UniqueIndex  // tib+0x98
    if (overlayIndex >= overlayStart + numGrowth && overlayIndex < overlayStart + numGrowth + numSpread)
        return tib->UniqueIndex
```

So each tiberium type owns a contiguous range of overlay indices:
- `[overlayStart .. overlayStart+numGrowth)` — growth frames (density levels)
- `[overlayStart+numGrowth .. overlayStart+numGrowth+numSpread)` — spread frames

**Confidence: HIGH** — directly from decompiled IsWallOverlay.

### Density and Overlay Data

The overlay data byte (CellClass offset 0x11E) represents the visual frame within the tiberium's overlay range. For ore (Image=1):
- 12 overlay types per tiberium type
- Each overlay type has multiple frames representing density
- The `OverlayData` byte selects which frame to display (0-11 typically)

When ore grows, the overlay data increments until it reaches the maximum density for that overlay type. When it spreads, it creates new ore cells in adjacent positions with low density.

**Confidence: MEDIUM** — inferred from overlay layout and naming conventions; the actual growth tick function was not directly decompiled.

---

## 4. Ore Value Calculation

### Per-Bail Value

From `[Tiberiums]` section in rulesmd.ini:
- **Ore (Riparius)**: `Value=25` credits per bail
- **Gems (Cruentus)**: `Value=50` credits per bail
- **Vinifera**: `Value=25`
- **Aboreus**: `Value=25`

The `Value` field is stored at TiberiumClass offset 0xB8 (int).

### Harvester Dump Cycle

Key RulesClass fields:

| INI Key | RulesClass Offset | Type | Default | Notes |
|---------|-------------------|------|---------|-------|
| `HarvesterDumpRate` | 0x1528 | double | | Multiplier for dump speed |
| `HarvesterLoadRate` | 0x1520 | int | | Frames per load tick |
| `PurifierBonus` | 0xF3C | float | | Multiplier for purifier-equipped refineries |

### Credit Deposit Flow (from Mission_Deploy_Building state 3)

The dump cycle in the slave miner building:

1. **Check dump timer**: `RulesClass+0x1528` (HarvesterDumpRate) * some factor determines dump interval
2. **Read storage**: `FUN_006c9680(storagePtr, tibType)` — reads `float` at `storagePtr + tibType * 4`
3. **Calculate value**: The storage amount is multiplied by `RulesClass+0xF3C` (PurifierBonus) and the tiberium's Value
4. **Deposit to house**:
   - `FUN_004f9610(amount, tibType)` — sets `HouseClass+0x54E8` and `HouseClass+0x30C` (account balance)
   - `FUN_004f9700(amount, tibType)` — loops depositing 1.0 per tiberium type until storage limit (`RulesClass+0x17D0`) is reached
5. **Decrement storage**: `FUN_006c96b0(storagePtr, amount, tibType)` — subtracts from storage

The storage system uses a per-tiberium-type float array (4 entries, one per tiberium type). Each float tracks how many bails of that type are stored.

### HouseClass Money Fields

| Offset | Field | Notes |
|--------|-------|-------|
| 0x30C | Balance/Credits | Running total, incremented by deposits |
| 0x54E8 | HarvestedAmount | Tracks total harvested for statistics |

**Confidence: MEDIUM-HIGH** — the overall flow is clear from decompilation, but some intermediate calculations are obscured by Ghidra's float handling.

### Slave Ore Deposit (FUN_00522d50 @ 0x00522d50)

When a slave returns to the master and deposits ore, `FUN_00522d50` is called with the master building:

```c
void DepositOreFromStorage(BuildingClass* master) {
    bool deposited = false;
    int tibType = StorageClass::FindNonEmpty();  // 0x006c9820 - returns first type with amount > 0

    while (tibType != -1) {
        int storageCapacity = master->Owner->field_0x538C;  // HouseClass silo capacity
        if (!master->Owner->IsHuman && g_GameMode != 0) {
            storageCapacity += AIDifficultyBonus[master->Owner->Difficulty];
            // bonus from RulesClass+0x1324 table indexed by difficulty
        }

        float currentAmount = StorageClass::GetAmount(tibType);     // 0x006c9680
        float creditValue = storageCapacity * PurifierBonus * currentAmount;

        float removed = StorageClass::RemoveAmount(currentAmount, tibType);  // 0x006c96b0
        if (removed > 0.0) {
            deposited = true;
            HouseClass::DepositOre(removed, tibType);      // 0x004f9610
            if (creditValue > 0.0) {
                HouseClass::DepositOre(creditValue, tibType);
            }
        }
        tibType = StorageClass::FindNonEmpty();
    }

    if (deposited) {
        master->vtable_0x468();  // trigger dump animation
    }
}
```

The `StorageClass::FindNonEmpty` (0x006c9820) iterates 4 float slots (one per tiberium type: ore, gems, vinifera, aboreus) and returns the first with amount > epsilon. Returns -1 if all empty.

**Confidence: HIGH** — directly decompiled.

---

## 5. CellClass Ore Fields

### CellClass Layout (328 bytes total, from Ghidra struct)

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0x24 (36) | 2 | MapCoord_X | Cell X coordinate |
| 0x26 (38) | 2 | MapCoord_Y | Cell Y coordinate |
| 0x38 (56) | 4 | IsoTileTypeIndex | Tile type, 0xFFFF = clear |
| 0x44 (68) | 4 | OverlayTypeIndex | -1 if no overlay; ore/gem overlay index if tiberium |
| 0xEC (236) | 4 | LandType | 0=Clear, 4=Road, 5=Tiberium, 9=Weeds, etc. |
| 0x11A (282) | 1 | Height | Cell height byte |
| 0x11B (283) | 1 | Level | Cell level byte |
| 0x11E (286) | 1 | OverlayData | Density/frame index for overlays (0-11 for tiberium) |
| 0x140 (320) | 4 | Flags | Bitfield (bit 8 = 0x100 checked in deploy, bit 10/11 for passability) |

### How the Game Distinguishes Ore from Gems

1. **OverlayTypeIndex** (offset 0x44) — stores the overlay type array index
2. **OverlayTypeClass** at `DAT_00a83d84 + overlayIndex * 4` has:
   - Offset 0x298: LandType (4=Road, 5=Tiberium for ore)
   - Offset 0x2A9: `IsTiberium` flag (bool)
   - Offset 0x2AC: another tiberium-related flag
3. **IsWallOverlay** function (0x005fdd00) maps overlay index -> TiberiumClass index (0=ore, 1=gems, 2=vine, 3=aboreus)
4. **CellClass::RecalcAttributes** (0x0047d2b0) reads the overlay type and sets `LandType` accordingly

The overlay index ranges are contiguous per tiberium type:
- Ore overlays: `overlayStart` to `overlayStart + 12` (growth) + 12 (spread) = 24 total
- Gem overlays: different `overlayStart`, 12 growth + 12 spread

### Overlay Data (Density)

The `OverlayData` byte at CellClass+0x11E represents density within the tiberium overlay:
- **0** = lowest density (just appeared)
- **11** = maximum density (fully grown)
- When ore grows, this value increments
- When ore is harvested, this value decrements (or the overlay is removed at 0)
- The visual frame displayed is directly tied to this value

The cell checks `FUN_00487df0` to determine if a cell has harvestable tiberium:
```c
bool IsOreCell(CellClass* cell) {
    return cell->LandType == 5;  // LandType_Tiberium
}
```

**Confidence: HIGH** — from CellClass struct, constructor, RecalcAttributes, and FUN_00487df0.

---

## 6. Key Global Addresses

| Address | Type | Description |
|---------|------|-------------|
| 0x00B0F4EC | int* | TiberiumClass array pointer (DynamicVectorClass data) |
| 0x00B0F4F8 | int | TiberiumClass array count |
| 0x00A83D84 | int* | OverlayTypeClass array pointer |
| 0x00B0B5B8 | int | Invalid cell coordinate sentinel (X) |
| 0x00B0B5BA | short | Invalid cell coordinate sentinel (Y) |

---

## 7. Key Function Addresses

| Address | Name | Description |
|---------|------|-------------|
| 0x007393C0 | UnitClass::Deploy | Unit-to-building transformation |
| 0x0073D630 | UnitClass::Mission_Deploy_Building | Slave miner dump/undeploy cycle |
| 0x006AF1A0 | SlaveManagerClass::Constructor | Creates slaves |
| 0x006AF5F0 | SlaveManagerClass::Update | Per-tick update (vtable[23]) |
| 0x006AF6C0 | SlaveManagerClass::SlaveAI | Per-slave state machine |
| 0x006B0AE0 | SlaveManagerClass::MasterDestroyed | Kills/frees slaves |
| 0x006B0300 | SlaveManagerClass::FindDeployCell | Finds cell for master deployment |
| 0x006B0DB0 | SlaveManagerClass::HandleRelocate | Handles master movement/relocation |
| 0x006B1020 | SlaveManagerClass::ShouldScanForOre | Checks if scan timer expired |
| 0x006F3F40 | TechnoClass::InitManagers | Creates SlaveManager/SpawnManager/etc |
| 0x006F9E50 | TechnoClass::AI_Update | Main AI loop (calls SlaveManager::Update) |
| 0x00744100 | UnitClass::ScanForTiberium | Harvester ore scanning |
| 0x005FDD00 | IsWallOverlay / GetTiberiumType | Maps overlay index to tiberium type |
| 0x00721A90 | TiberiumClass::ReadINI | Parses tiberium type properties |
| 0x004F9610 | HouseClass::DepositOre | Deposits harvested ore as credits |
| 0x004F9700 | HouseClass::DepositToStorage | Adds bails to storage system |
| 0x004F9950 | HouseClass::AddMoney | Direct credit addition |
| 0x006C9680 | StorageClass::GetAmount | Reads storage for tiberium type |
| 0x006C96B0 | StorageClass::RemoveAmount | Decrements storage |
| 0x006C9690 | StorageClass::AddAmount | Increments storage |
| 0x00487DF0 | CellClass::IsTiberiumCell | Returns LandType == 5 |
| 0x0047D2B0 | CellClass::RecalcAttributes | Recalculates land type from overlay |
| 0x00465D70 | Deploy_facing_calculator | Computes required facing for deploy |
