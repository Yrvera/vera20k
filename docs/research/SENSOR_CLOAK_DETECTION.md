# Sensor Detection vs Cloaking System — Ghidra Research Report

**Source:** Live Ghidra decompilation of `gamemd.exe`
**Confidence:** HIGH (verified from binary, all offsets cross-checked)

## Correction: TechnoTypeClass+0xC9A is NOT SensorsSight

The field at TechnoTypeClass byte offset 0xC9A is **`Invisible`** (bool), NOT `SensorsSight`.
Verified from TechnoTypeClass::ReadINI at 0x00714AAB:
```
INI key: "Invisible" (string at 0x00843944)
Storage: *(byte*)(TechnoTypeClass + 0xC9A)
```

Nearby fields in this region:
| Byte Offset | INI Key         | Type |
|-------------|-----------------|------|
| 0xC95       | IsDropship      | bool |
| 0xC96       | ToProtect       | bool |
| 0xC97       | Disableable     | bool |
| 0xC99       | DoubleOwned     | bool |
| 0xC9A       | **Invisible**   | bool |
| 0xC9B       | RadarVisible    | bool |
| 0xC9D       | Sensors         | bool |
| 0xC9E       | Nominal         | bool |
| 0xC9F       | DontScore       | bool |
| 0xCA1       | (used at 0x713388) | bool |
| 0xCA2       | TurretRecoil    | bool |

## 1. SensorsSight Field

**INI key:** `SensorsSight`
**String address:** 0x00843D50
**TechnoTypeClass offset:** 0x5F0 (int, byte offset; accessed as `param_1[0x17C]` when param_1 is `int*`)
**Read in:** TechnoTypeClass::ReadINI at 0x007142E8, via `CCINIClass::ReadInt`

This is an **int** specifying the sensor sight range in cells. When nonzero, the unit
projects a circular sensor field around itself that reveals cloaked enemy units.

### Where SensorsSight is used (all xrefs to byte pattern `8B xx F0 05 00 00`):

| Address    | Function                              | Purpose                              |
|------------|---------------------------------------|--------------------------------------|
| 0x004D7300 | FUN_004D7170 (Unit placement/deploy)  | If != 0, call AddSensorsAt           |
| 0x004DB362 | FUN_004DB260 (Unit removal/undeploy)  | If != 0, call RemoveSensorsAt        |
| 0x004DBEDD | FUN_004DBED0 (Unit movement tick)     | Remove old + add new sensors on move |
| 0x004DE7C0 | TechnoClass::AddSensorsAt             | Read range for cell iteration        |
| 0x004DE950 | TechnoClass::RemoveSensorsAt          | Read range for cell iteration        |
| 0x007142E1 | TechnoTypeClass::ReadINI              | Read default value for INI parsing   |

## 2. TechnoClass::GetVisualState (0x00703860) — Invisible, NOT Sensors

The function at 0x00703860 checks `Invisible` (TechnoTypeClass+0xC9A), NOT `SensorsSight`.
Return values represent visual/cloak state:
- 0 = fully visible
- 1-4 = cloaking animation stages
- 5 = fully cloaked/invisible

### Decompiled logic (simplified):

```c
int typeClass = this->GetType();   // vtable+0x84
bool invisible = *(char*)(typeClass + 0xC9A);  // Invisible flag
bool discovered = *(char*)((int)this + 0x41A); // IsDiscoveredByCurrentPlayer

if (invisible && discovered) {
    return 0;  // Invisible type but discovered -> show as visible
}

if (invisible && !discovered && !g_DAT_00a8ed6b) {
    return 5;  // Invisible type, not discovered -> fully hidden
}

// Cloakable unit handling (CloakState at this+0x220):
if (this->CloakState != 0 && !g_DAT_00a8ed6b) {
    int whatAmI = this->WhatAmI();  // vtable+0x2C
    if (whatAmI == 6) return 0;     // Buildings don't visually cloak this way

    if (this->CloakState == 2) {
        // Fully cloaked - check gap generators and fog of war
        // ... gap/fog checks determine if shown as 3 or 5
    } else {
        // Cloaking/uncloaking animation
        int progress = this->CloakProgress;  // +0x224
        if (progress < 0x40) return 1;
        if (progress < 0x80) return 2;
        if (progress < 0xC0) return 3;
        if (progress <= 0xFE) return 4;  // (corrected 2026-05-29: was `< 0xFE`; binary shows `return (0xfe < iVar3) + '\x04'` so threshold is <= 0xFE → 4, > 0xFE → 5 via decompile_function 0x00703860 — OPERATOR_OR_ORDER_DRIFT)
        return 5;
    }
}

return 0;  // Visible
```

**Key field:** `TechnoClass+0x41A` = per-player discovery flag. When a sensor covers a cell
containing an `Invisible` unit, this flag gets set, making the unit visible.

## 3. Sensor Detection System — Cell-Level Counters

The engine uses **per-house counters on each CellClass** to track sensor coverage:

### CellClass+0x7C: Sensor Count Array
```
short SensorCount[MaxHouses];  // at CellClass + 0x7C + houseIndex * 2
```

**Increment:** CellClass::IncrementSensorCount (0x00487150)
```c
void CellClass::IncrementSensorCount(int houseIndex) {
    this->SensorCount[houseIndex]++;  // *(short*)(this + 0x7C + houseIndex * 2)
}
```

**Decrement:** CellClass::DecrementSensorCount (0x00487160)

**Query:** CellClass::SensorCountForHouse (0x004870D0)
```c
bool CellClass::SensorCountForHouse(int houseIndex) {
    return this->SensorCount[houseIndex] > 0;
}
```

NOTE: This function was previously mislabeled as `CellClass__GapCountForHouse` in Ghidra.
It is the SENSOR count, not gap count. Gap generators use CellClass+0x130/0x134 (see
TechnoClass::UpdateCloakShroud at 0x006FB170).

### CellClass+0xAC: Disguise Detection Count Array
```
short DisguiseDetectCount[MaxHouses];  // at CellClass + 0xAC + houseIndex * 2
```

**Increment:** CellClass::IncrementDisguiseDetectCount (0x00487170)
**Decrement:** CellClass::DecrementDisguiseDetectCount (0x00487180)

## 4. TechnoClass::AddSensorsAt (0x004DE7B0) — vtable+0x4E8

Called when a unit with `SensorsSight > 0` is placed on the map. Iterates cells in a
circular radius and:
1. Increments the sensor count for the owner house on each cell
2. Forces `DoUncloak` (vtable+0x420 = 0x006F4EB0) on all Infantry, Aircraft, and Unit
   objects found in each covered cell
3. Marks enemy buildings in range as discovered

```c
void TechnoClass::AddSensorsAt(CellStruct coord) {
    int range = this->GetType()->SensorsSight;  // TypeClass + 0x5F0
    int houseIdx = this->Owner->ArrayIndex;     // TechnoClass+0x21C -> +0x30

    if (coord == NullCell) coord = this->GetCell();

    for (dy = -range; dy < range; dy++) {
        for (dx = -range; dx < range; dx++) {
            if (dx*dx + dy*dy < range*range) {  // circular range check
                CellClass* cell = Map.GetCellAt(coord + {dx, dy});
                cell->IncrementSensorCount(houseIdx);

                // Force uncloak on all units in this cell
                for (obj = cell->FirstObject; obj != NULL; obj = obj->NextObject) {
                    int type = obj->WhatAmI();
                    if (type == 1 || type == 0xF || type == 2) {  // Inf/Air/Unit
                        obj->DoUncloak();
                    }
                }

                // Discover enemy buildings
                BuildingClass* bld = cell->GetBuilding();
                if (bld && bld->Owner != g_PlayerPtr && bld->IsAlive()) {
                    bld->MarkDirty = true;
                }
            }
        }
    }
}
```

## 5. TechnoClass::RemoveSensorsAt (0x004DE940) — vtable+0x4EC

Reverse of AddSensorsAt. Decrements sensor counts and re-processes visibility:

```c
void TechnoClass::RemoveSensorsAt(CellStruct coord) {
    int range = this->GetType()->SensorsSight;
    int houseIdx = this->Owner->ArrayIndex;

    for each cell in circular range:
        if (cell->SensorCountForHouse(houseIdx)) {  // only if count was > 0
            cell->DecrementSensorCount(houseIdx);
            // Re-check visibility for all units in cell
            for (obj in cell) DoUncloak on Infantry/Aircraft/Unit
        }
}
```

## 6. Sensor Array Buildings

### BuildingTypeClass Fields
| Byte Offset | INI Key              | Type  | Purpose                          |
|-------------|----------------------|-------|----------------------------------|
| 0x16C7      | CloakGenerator       | bool  | Building generates cloak field   |
| 0x16C8      | SensorArray          | bool  | Building generates sensor field  |
| 0x1707      | CloakRadiusInCells   | byte  | Radius for cloak/sensor range    |
| 0x170C      | PsychicDetectionRadius| int  | Psychic detection range          |

### BuildingClass::GetSensorRange (0x004566B0)
Returns the effective sensor/cloak range for a building:
```c
int BuildingClass::GetSensorRange() {
    BuildingTypeClass* type = this->Type;
    int psychicRadius = type->PsychicDetectionRadius;  // +0x170C

    if (psychicRadius >= 1) return psychicRadius;

    if (type->GapGenerator) {  // +0xCD1
        if (this->IsPowered) return type->SuperGapRadiusInCells;  // +0xCD3
        else return type->GapRadiusInCells;  // +0xCD2
    }

    if (type->SensorArray || type->CloakGenerator) {  // +0x16C8 || +0x16C7
        return type->CloakRadiusInCells;  // +0x1707
    }

    // Fall through to sight range calculation...
}
```

### Building Placement (FUN_00445F80)
When a SensorArray building is placed on the map:
```c
if (buildingType->SensorArray) {           // +0x16C8
    this->AddSensorArrayAt(NullCell);       // vtable+0x4F4
}
if (buildingType->DetectDisguise) {         // TechnoTypeClass+0xD31
    this->AddDetectDisguiseAt(NullCell);    // vtable+0x4FC
}
```

### BuildingClass::AddSensorArrayAt (0x00455820) — vtable+0x4F4
Uses **TechnoTypeClass::SensorsSight** (0x5F0) as range. Same circular iteration as
TechnoClass::AddSensorsAt but includes a power check (vtable+0x350).

### BuildingClass::RemoveSensorArrayAt (0x004556D0) — vtable+0x4F8
Uses **BuildingTypeClass::CloakRadiusInCells** (0x1707) as range. Decrements sensor
counts and re-processes unit visibility.

## 7. DetectDisguise vs DetectDisguiseRange

### TechnoTypeClass Fields
| Byte Offset | INI Key              | Type | Purpose                              |
|-------------|----------------------|------|--------------------------------------|
| 0xD31       | DetectDisguise       | bool | Unit can detect disguised enemies     |
| 0x5F4       | DetectDisguiseRange  | int  | Range (cells) for disguise detection  |

**INI string addresses:**
- "DetectDisguise" at 0x00843C78 (xref: ReadINI at 0x0071443F)
- "DetectDisguiseRange" at 0x00843D3C (xref: ReadINI at 0x00714302)

### Disguise Detection Cell Counter
Separate from sensor detection. Uses CellClass+0xAC per-house array:

**BuildingClass::AddDetectDisguiseAt (0x00455A80) — vtable+0x4FC:**
- Range: TechnoTypeClass::DetectDisguiseRange (0x5F4)
- Increments CellClass+0xAC per-house disguise detect count
- Does NOT call DoUncloak (disguise detection doesn't decloak, it reveals the true identity)

**BuildingClass::RemoveDetectDisguiseAt (0x00455980) — vtable+0x500:**
- Range: TechnoTypeClass::DetectDisguiseRange (0x5F4)
- Decrements CellClass+0xAC per-house disguise detect count

### No "DetectCloak" INI Key
There is NO string "DetectCloak" or "DetectCloaked" in gamemd.exe. Cloaked unit detection
is handled entirely by the `SensorsSight` and `SensorArray` systems described above.

## 8. TechnoClass::DoUncloak (0x006F4EB0) — vtable+0x420

Called on each unit when sensor coverage changes. Determines if the unit should uncloak:

```c
void TechnoClass::DoUncloak() {
    CellStruct myCell = this->GetCell();

    // If unit is cloaked (state 2), enemy player, and cell has sensor coverage
    if (this->CloakState == 2 && g_PlayerPtr && this->Owner != g_PlayerPtr) {
        if (!CellClass::SensorCountForHouse(g_PlayerPtr->ArrayIndex)) {
            this->MarkForRedraw();  // vtable+0x150
        }
    }

    // If cell is visible to owner and unit should uncloak
    if (cell->IsVisibleToHouse(this->Owner->ArrayIndex) && this->ShouldUncloak()) {
        this->StartUncloaking();  // vtable+0x460
        // Notify units targeting this one
    }
}
```

## 9. Summary: Three Separate Detection Systems

| System           | INI Keys                     | Cell Array Offset | Effect on Target           |
|------------------|------------------------------|-------------------|----------------------------|
| **Sensor Sight** | SensorsSight (int range)     | CellClass+0x7C    | Forces DoUncloak           |
| **Sensor Array** | SensorArray (bool) + CloakRadiusInCells | CellClass+0x7C | Forces DoUncloak     |
| **Disguise Detect** | DetectDisguise (bool) + DetectDisguiseRange (int) | CellClass+0xAC | Reveals true identity |

All three use circular range checks (`dx*dx + dy*dy < range*range`) and per-house
short counters on CellClass. Sensor sight and sensor arrays share the same cell counter
(+0x7C), while disguise detection has its own (+0xAC).

## 10. Key Vtable Offsets (TechnoClass/BuildingClass)

| Vtable Offset | Function                          | Address    |
|---------------|-----------------------------------|------------|
| +0x420        | TechnoClass::DoUncloak            | 0x006F4EB0 |
| +0x4E8        | TechnoClass::AddSensorsAt         | 0x004DE7B0 |
| +0x4EC        | TechnoClass::RemoveSensorsAt      | 0x004DE940 |
| +0x4F4        | BuildingClass::AddSensorArrayAt   | 0x00455820 |
| +0x4F8        | BuildingClass::RemoveSensorArrayAt| 0x004556D0 |
| +0x4FC        | BuildingClass::AddDetectDisguiseAt| 0x00455A80 |
| +0x500        | BuildingClass::RemoveDetectDisguiseAt | 0x00455980 |
