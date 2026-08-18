# Building Upgrade System - Ghidra Report

**Source:** gamemd.exe decompilation via Ghidra MCP  
**Confidence:** HIGH - all offsets verified from binary, cross-referenced with INI parsing  
**Active in YR:** YES - used by GAPOWR (Allied Power Plant, Upgrades=2) and YAPOWR (Yuri Bio Reactor, Upgrades=2)

## Overview

Buildings can receive upgrades when a "PowersUp" building is placed on top of them.
The upgrade system modifies power output/drain, weapons, and visual appearance (anims).
Maximum 3 upgrade slots per building (hardcoded). Vanilla YR uses max 2 (Upgrades=2).

---

## BuildingTypeClass Fields (upgrade target)

| Offset | Type | INI Key | Description |
|--------|------|---------|-------------|
| +0x14E0 | int | `Upgrades=` | Max number of upgrades this building accepts (0-3) |
| +0xEE0 | int | `Power=` | Base power output |
| +0xEE4 | int | `Power=` (negative) | Base power drain (stored as positive) |
| +0xEE8 | int | `ExtraPower=` | Extra power output bonus (when HasExtraPowerBonus set) |
| +0xEEC | int | `ExtraPower=` (negative) | Extra power drain (when negative ExtraPower given) |
| +0xF04 | BuildAnimEntry[varies] | (build anim data) | Build/active anim entries (0xC bytes each) |
| +0xF4C | PowerUpAnimEntry[3] | (art.ini) | PowerUp anim data array (0x44 bytes each, see below) |

### PowerUpAnimEntry Structure (0x44 = 68 bytes per slot)

Read from art.ini using the building's Image= name as section.

| Relative Offset | Type | INI Key (art.ini) |
|-----------------|------|-------------------|
| +0x00 | char[16] | `PowerUp%dAnim` |
| +0x10 | char[16] | `PowerUp%dDamagedAnim` |
| +0x20 | (16 bytes) | (unknown/padding) |
| +0x30 | int | `PowerUp%dLocXX` |
| +0x34 | int | `PowerUp%dLocYY` |
| +0x38 | int | `PowerUp%dLocZZ` |
| +0x3C | int | `PowerUp%dYSort` |
| +0x40 | byte | `PowerUp%dPowered` (flag checked in CreateAnimForSlot) |
| +0x41-0x43 | | (padding) |

Absolute offsets in BuildingTypeClass:
- Slot 1: 0xF4C - 0xF8F
- Slot 2: 0xF90 - 0xFD3
- Slot 3: 0xFD4 - 0x1017

## BuildingTypeClass Fields (upgrade building)

| Offset | Type | INI Key | Description |
|--------|------|---------|-------------|
| +0xE88 | char[24] | `PowersUpBuilding=` | Name of building type this upgrades (empty = not an upgrade) |
| +0x16FC | int | `PowersUpToLevel=` | Target upgrade level. -1 = incremental (add 1 level). 1-3 = set to specific level |
| +0x1F8 | char[32] | `Image=` | Image name (used as anim name for the upgrade) |
| +0xEE0 | int | `Power=` | Power bonus added to host building per upgrade |
| +0xEE4 | int | `Power=` (negative) | Power drain added to host building per upgrade |

## BuildingClass Fields (instance data)

| Offset | Type | Description |
|--------|------|-------------|
| +0x520 | ptr | `Type` - pointer to BuildingTypeClass |
| +0x55C | ptr[21] | `Anims[21]` - array of AnimClass pointers for all anim slots (0x15 entries) |
| +0x5EC | ptr | `Upgrades[0]` - BuildingTypeClass* of first upgrade |
| +0x5F0 | ptr | `Upgrades[1]` - BuildingTypeClass* of second upgrade |
| +0x5F4 | ptr | `Upgrades[2]` - BuildingTypeClass* of third upgrade |
| +0x5FC | int | Cycling anim phase index (used for special building types like NukeReactor) |
| +0x660 | byte | `HasPower` - whether building has power |
| +0x661 | byte | `HasExtraPowerBonus` - set by PowerCheck_Upgrade when bio-reactor has occupants |
| +0x662 | byte | `HasExtraPowerDrain` |
| +0x702 | byte | `UpgradeLevel` - current number of installed upgrades (0-3) |

---

## Upgrade Lifecycle

### 1. CanUpgrade Check (FUN_00452670 @ 0x00452670)

Called to determine if an upgrade building can be placed on a target.

```
bool CanUpgrade(BuildingClass* target, BuildingTypeClass* upgradeType, HouseClass* owner)
{
    if (owner != target->Owner) return false;
    
    // strcmp: upgradeType->PowersUpBuilding must match target->Type->Name
    if (strcmp(upgradeType->PowersUpBuilding, target->Type->Name) != 0) return false;
    
    int level = upgradeType->PowersUpToLevel;
    if (level == -1) {
        // Incremental: check if target has room
        if (target->UpgradeLevel >= target->Type->Upgrades) return false;
    } else {
        // Specific level: must be 1-3
        if (level < 1 || level > 3) return false;
    }
    
    // Also fails if target already has max upgrades
    if (target->UpgradeLevel != 0) return false;  // Only when specific level
    
    return true;
}
```

### 2. Upgrade Installation (BuildingClass::Unlimbo, FUN_00440580 @ 0x00440580)

When an upgrade building is placed, `Unlimbo` handles the upgrade logic in the else
branch (when `PowersUpBuilding` is non-empty):

```
// 1. Find target building at placement cell
BuildingClass* target = FindBuildingInCell(cell);
if (target == NULL) return 0;

// 2. Verify it's a building (RTTI == 6)
if (target->GetRTTI() != RTTI_Building) return 0;

// 3. Same owner check
if (upgradeBuilding->Owner != target->Owner) return 0;

// 4. PowersUpBuilding name must match target type name
if (strcmp(upgradeType->PowersUpBuilding, target->Type->Name) != 0) return 0;

// 5. PowersUpToLevel check
int level = upgradeType->PowersUpToLevel;
if (level == -1) {
    // Incremental: check room
    if (target->UpgradeLevel >= target->Type->Upgrades) return 0;
} else {
    if (level < 1 || level > 3) return 0;
}

// 6. Can't upgrade a fully-upgraded building (for specific level)
if (target->UpgradeLevel != 0) return 0;  // for specific level mode

// 7. Mark house flags
target->Owner->NeedsRecalc1 = 1;
target->Owner->NeedsRecalc2 = 1;

// 8. Determine how many levels to add
int levelsToAdd = (level == -1) ? 1 : level;

// 9. Copy upgrade building's Image name into target's PowerUpNAnim slot
//    This overwrites the target's PowerUp anim name at the current upgrade slot
char* dest = target->Type + 0xF4C + target->UpgradeLevel * 0x44;
if (strcmp(dest, upgradeType->ImageName) != 0) {
    strncpy(dest, upgradeType->ImageName, 16);
}

// 10. Call AddUpgrade for each level
for (int i = 0; i < levelsToAdd; i++) {
    AddUpgrade(target);
}

// 11. Store upgrade type in the slot
target->Upgrades[target->UpgradeLevel - 1] = upgradeType;
// (Note: written AFTER AddUpgrade increments UpgradeLevel)
// Actual code: piVar8[UpgradeLevel + 0x17A] where piVar8 is int*
// When UpgradeLevel=1: writes to byte offset 0x5EC = Upgrades[0]

// 12. Resume production for owner
HouseClass::AI_ResumeProduction(target->Owner);
target->Owner->NeedsRecalc = 1;

// 13. If upgrade type has FactoryPlant flag, notify spy
if (upgradeType->IsFactoryPlant) {
    SpyNotify();
}

// 14. Destroy the upgrade building itself
upgradeBuilding->Destroy();

return 1;
```

**CRITICAL:** The upgrade building is destroyed after installation. It does NOT persist
as a physical building on the map. It becomes a pointer in the target's Upgrades[] array.

### 3. AddUpgrade Function (FUN_00451400 @ 0x00451400)

Called by Unlimbo to increment upgrade level and create visual effects.

```
bool AddUpgrade(BuildingClass* building)
{
    // 1. Heal building to full health
    if (building->Health != building->Type->Strength) {
        building->Health = building->Type->Strength;
        if (building->IsDamaged) {
            building->IsDamaged = false;
            // Refresh all existing anim slots for undamaged state
            for (int i = 0; i < 0x15; i++) {
                if (building->Anims[i] != NULL && animName[i] != '\0') {
                    CreateAnimForSlot(building, i);
                }
            }
        }
        // Also refresh condition-based anims (damaged indicator, etc.)
        // ...
        building->NeedsRedraw = true;
    }
    
    // 2. Special path for NukeReactor (RulesClass+0x87C)
    if (building->Type == Rules->NukeReactor) {
        // Cycles through animation phases 0,1,2 (appends 'B','C','D' to type name)
        int phase = building->AnimPhase + 1;
        if (phase > 2) phase = 0;
        if (phase != building->AnimPhase && !building->Type->IsInvisible) {
            ClearAnimSlot(building, slotIndex);
            if (phase != -1) {
                // Build anim name: "TypeName_B" / "TypeName_C" / "TypeName_D"
                char animName[64];
                strcpy(animName, building->Type->ImageName);
                strcat(animName, "_");
                animName[len] = 'B' + phase;
                CreateAnimForSlot(building, animName);
                building->AnimPhase = phase;
            }
        }
        building->UpgradeLevel++;
        return true;
    }
    
    // 3. Normal upgrade path
    if (building->UpgradeLevel >= building->Type->Upgrades) {
        return false;  // Already at max
    }
    
    building->UpgradeLevel++;
    
    // 4. Create PowerUp anim for this upgrade level
    int slotIndex = building->UpgradeLevel - 1;
    double healthRatio = building->GetHealthRatio();
    char* animName;
    if (healthRatio > Rules->ConditionYellow) {
        animName = building->Type + slotIndex * 0x44 + 0xF4C;  // PowerUpNAnim
    } else {
        animName = building->Type + slotIndex * 0x44 + 0xF5C;  // PowerUpNDamagedAnim
    }
    
    if (animName != NULL && *animName != '\0') {
        CreateAnimForSlot(building, slotIndex, animName);
    }
    
    return true;
}
```

### 4. RemoveLastUpgrade (BuildingClass__RemoveLastUpgrade @ 0x00451690)

Called when:
- Building is sold (`BuildingClass::Sell` @ 0x00449C30)
- Occupants ejected (`BuildingClass::EjectOccupants` @ 0x004575B0)
- AI manages build queue cost overrun (`HouseClass::AI_Manage_Build_Queue` @ 0x004FDD10)

```
bool RemoveLastUpgrade(BuildingClass* building)
{
    if (building->UpgradeLevel == 0) return false;
    
    // Get the type of the last upgrade
    BuildingTypeClass* upgradeType = building->Upgrades[building->UpgradeLevel - 1];
    // (Read from: field_0x5E8 + UpgradeLevel * 4)
    
    bool hadPrereqs = false;
    if (upgradeType != NULL) {
        hadPrereqs = (upgradeType->TechLevel != -1);  // offset 0x16F0 in BTClass
    }
    
    if (upgradeType == NULL || !upgradeType->IsUpgrade || upgradeType->TechLevel == -1) {
        // Simple removal: clear one slot
        ClearAnimSlot(building, -2);  // -2 = clear all anims
        building->UpgradeLevel--;
        building->Upgrades[building->UpgradeLevel] = NULL;
        if (hadPrereqs) {
            HouseClass::AI_ManageProduction(building->Owner);
        }
    } else {
        // Full reset: clear everything
        ClearAnimSlot(building, -2);
        building->Upgrades[building->UpgradeLevel - 1] = NULL;
        building->UpgradeLevel = 0;
        building->AnimPhase = -1;
        if (hadPrereqs) {
            HouseClass::AI_ManageProduction(building->Owner);
            return true;
        }
    }
    return true;
}
```

---

## Upgrade Effects

### Power Output (BuildingClass::GetPowerOutput @ 0x0044E7B0)

```
int GetPowerOutput(BuildingClass* building)
{
    int power = building->Type->Power;       // +0xEE0
    
    if (building->IsUnderEMP()) return 0;
    
    // ExtraPowerBonus from bio-reactor passengers
    if (building->HasExtraPowerBonus) {       // +0x661
        power += building->Type->ExtraPower;  // +0xEE8
    }
    
    // UnitRepair/AircraftRepair dock bonus (per docked unit)
    if ((building->Type->UnitRepair || building->Type->AircraftRepair) 
        && building->Type->ExtraPower > 0 && building->DockedCount > 0) {
        power += building->Type->ExtraPower * building->DockedCount;
    }
    
    // UPGRADE POWER: each upgrade adds its type's Power value
    if (building->UpgradeLevel != 0) {
        for (int i = 0; i < 3; i++) {
            if (building->Upgrades[i] != NULL) {
                power += building->Upgrades[i]->Power;  // upgradeType+0xEE0
            }
        }
    }
    
    // Scale by health ratio if positive power
    if (power > 0 && building->HasPower) {
        power = (int)(power * building->GetHealthRatio());
    }
    
    return power;
}
```

### Power Drain (BuildingClass::GetPowerDrain @ 0x0044E880)

```
int GetPowerDrain(BuildingClass* building)
{
    if (building->IsUnderEMP() || !building->HasPower) return 0;
    
    int drain = building->Type->PowerDrain;     // +0xEE4
    
    if (building->HasExtraPowerDrain) {          // +0x662
        drain += building->Type->ExtraDrain;     // +0xEEC
    }
    
    // UPGRADE DRAIN: each upgrade adds its type's PowerDrain
    if (building->UpgradeLevel != 0) {
        for (int i = 0; i < 3; i++) {
            if (building->Upgrades[i] != NULL) {
                drain += building->Upgrades[i]->PowerDrain;  // upgradeType+0xEE4
            }
        }
    }
    
    return drain;
}
```

### Weapon Override (BuildingClass::GetWeapon @ 0x004526F0)

When a building has upgrades, the weapon system checks upgrade types first:

```
WeaponStruct* GetWeapon(BuildingClass* building, int weaponIndex)
{
    // Check upgrade types for weapons FIRST
    if (building->UpgradeLevel > 0) {
        for (int i = 0; i < building->UpgradeLevel; i++) {
            if (building->Upgrades[i] != NULL) {
                WeaponStruct* w = building->Upgrades[i]->GetWeaponStruct(weaponIndex);
                if (w->WeaponType != NULL) {
                    return w;  // Upgrade's weapon takes priority
                }
            }
        }
    }
    
    // Fall back to building's own weapons (or passenger weapons for garrisons)
    // ...
}
```

**Key insight:** Upgrade buildings can override the host's weapons by defining their own
`Primary=` / `Secondary=` weapon. The first upgrade with a non-null weapon wins.

`TechnoTypeClass::GetWeaponStruct` (FUN_007177c0): returns pointer to weapon data at
`typePtr + 0x898 + weaponIndex * 0x1C`.

### Animations

Upgrade anims are stored in the BuildingTypeClass PowerUpAnimEntry array (see layout above).
Each upgrade level gets its own anim slot with:
- Healthy/damaged variants
- Location offsets (LocXX, LocYY, LocZZ)
- Y-sort adjustment
- Powered flag (hides anim when building has no power)

The `CreateAnimForSlot` function (@ 0x00451890) creates an AnimClass instance, positions
it at the building's location plus the LocXX/LocYY/LocZZ offsets, and stores it in the
building's Anims[] array.

**Anim name override during placement:** When an upgrade is placed, the Unlimbo function
copies the upgrade building's `Image=` name into the target building's `PowerUpNAnim` slot,
overriding whatever was configured in art.ini. This allows different upgrade buildings to
produce different visual effects on the same base building.

### Prerequisites / Tech Tree

When an upgrade is removed (RemoveLastUpgrade), if the upgrade type has a valid TechLevel
(offset 0x16F0 != -1), the function calls `HouseClass::AI_ManageProduction` to recalculate
what the house can build. This means upgrades can affect prerequisites/tech availability.

The Unlimbo function also calls `HouseClass::AI_ResumeProduction` after installation,
which re-evaluates the production queue.

---

## ClearAnimSlot (@ 0x00451E40)

```
void ClearAnimSlot(BuildingClass* building, int slotIndex)
{
    if (slotIndex == -2) {
        // Clear ALL 21 anim slots
        for (int i = 0; i < 0x15; i++) {
            if (building->Anims[i] != NULL) {
                AnimClass* anim = building->Anims[i];
                building->Anims[i] = NULL;
                anim->Destroy();
            }
        }
    } else {
        // Clear specific slot
        if (building->Anims[slotIndex] != NULL) {
            AnimClass* anim = building->Anims[slotIndex];
            building->Anims[slotIndex] = NULL;
            anim->Destroy();
        }
    }
}
```

---

## PowerCheck_Upgrade (@ 0x00450590) - Bio Reactor Passenger Power Bonus

**NOTE:** Despite the name, this function is specifically about the bio-reactor's
infantry passenger power bonus, NOT the building upgrade system. It uses a different
set of fields:

- +0x670: DynamicVector pointer (passenger array)
- +0x67C: DynamicVector count (number of passengers)
- +0x661: HasExtraPowerBonus flag (output)

The function iterates passengers, removes invalid ones (dead/escaped), and sets
the `HasExtraPowerBonus` flag based on whether there are valid passengers AND the
house has sufficient power ratio.

No callers found via xrefs - likely called through vtable dispatch or inlined.

---

## Summary of Key Addresses

| Address | Function |
|---------|----------|
| 0x00440580 | BuildingClass::Unlimbo (includes upgrade placement logic) |
| 0x00451400 | AddUpgrade (FUN_00451400) - increments level, creates anim |
| 0x00451690 | BuildingClass::RemoveLastUpgrade |
| 0x004526F0 | BuildingClass::GetWeapon (checks upgrade weapons first) |
| 0x00452670 | CanUpgrade check (FUN_00452670) |
| 0x0044E7B0 | BuildingClass::GetPowerOutput (sums upgrade power) |
| 0x0044E880 | BuildingClass::GetPowerDrain (sums upgrade drain) |
| 0x00451890 | BuildingClass::CreateAnimForSlot |
| 0x00451E40 | BuildingClass::ClearAnimSlot |
| 0x00450590 | BuildingClass::PowerCheck_Upgrade (bio-reactor passengers, NOT upgrades) |
| 0x00447780 | BuildingClass::GrandOpening (uses upgrade level for anim selection) |

---

## Implementation Notes

1. **Upgrades[] stores BuildingTypeClass pointers** - not indices, not objects. Each slot
   is a direct pointer to the upgrade's type class.

2. **Max 3 slots hardcoded** - the Upgrades[3] array and the `< 3` check in PowerCheck
   and the `> 3` validation in CanUpgrade all enforce this.

3. **PowersUpToLevel=-1 is the common case** (incremental). Positive values (1-3) set
   a specific level, which is unusual and mainly for mods.

4. **The upgrade building is destroyed** after installation. It only exists as a pointer
   in the target's Upgrades[] array.

5. **Health is fully restored** when an upgrade is installed (AddUpgrade heals to max).

6. **Upgrade weapons take priority** over the building's own weapons. This is how weapon
   upgrades work (e.g., giving a power plant a weapon through an upgrade building).

7. **Power is additive** - each upgrade's Power= value is added to the host's base power.
   Power drain works the same way.

8. **Anim name override** - the upgrade building's Image= name replaces the PowerUpNAnim
   string at the current slot, allowing dynamic visual customization.
