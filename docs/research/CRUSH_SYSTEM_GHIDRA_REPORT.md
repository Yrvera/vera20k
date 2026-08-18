# Crush System — Ghidra Deep Dive Report

## Overview

The crush system allows heavy vehicles to destroy infantry and other crushable objects
by driving over them. The system involves INI-configurable flags on both crushers and
victims, cell-level occupancy iteration, distance checks, ally checks, and special
handling for deployed infantry and trains.

Confidence: HIGH (all offsets verified from live Ghidra decompilation of gamemd.exe).

---

## 1. INI Keys and Struct Offsets

### ObjectTypeClass (base class for all types)

| INI Key | Byte Offset | Type | Parsed In | Description |
|---------|-------------|------|-----------|-------------|
| `Crushable` | +0x22D | bool | `ObjectTypeClass::ReadINI` (0x5f9400) | Object can be crushed by regular crushers |
| `CrushSound` | +0x1F0 | int (VocClass index) | `ObjectTypeClass::ReadINI` (0x5f93a0) | Sound played when this object is crushed |

### TechnoTypeClass (inherits ObjectTypeClass)

| INI Key | Byte Offset | Type | Parsed In | Description |
|---------|-------------|------|-----------|-------------|
| `Crusher` | +0xD28 | bool | `TechnoTypeClass::ReadINI` (0x714cdb) | This unit can crush infantry |
| `OmniCrusher` | +0xD29 | bool | `TechnoTypeClass::ReadINI` (0x714cf0) | This unit can crush ANY non-OmniCrushResistant unit |
| `OmniCrushResistant` | +0xD2A | bool | `TechnoTypeClass::ReadINI` (0x714d11) | This unit cannot be crushed by OmniCrushers |
| `TiltsWhenCrushes` | +0xD2B | bool | `TechnoTypeClass::ReadINI` (0x7153e1) | Vehicle tilts forward visually when crushing |
| `AutoCrush` | +0xD2D | bool | `TechnoTypeClass::ReadINI` (0x714d32) | **Per-type flag** (separate from IQ level AutoCrush) |
| `IsTrain` | +0xC94 | bool | `TechnoTypeClass::ReadINI` (0x712284) | Train units crush allies too |

### InfantryTypeClass

| INI Key | Byte Offset | Type | Parsed In | Description |
|---------|-------------|------|-----------|-------------|
| `DeployedCrushable` | +0xEAC | bool | `InfantryTypeClass::ReadINI` (0x524627) | Infantry is crushable when deployed |
| *(unknown at +0xEC6)* | +0xEC6 | bool | `InfantryTypeClass::ReadINI` | Used in "pickup instead of crush" special case |

### BuildingTypeClass

| INI Key | Byte Offset | Type | Parsed In | Description |
|---------|-------------|------|-----------|-------------|
| `Gate` | +0x16B7 | bool | `BuildingTypeClass::ReadINI` (0x460083) | Gate buildings block enemy passage |

### RulesClass ([CombatDamage] section)

| INI Key | Byte Offset | Type | Parsed In | Description |
|---------|-------------|------|-----------|-------------|
| `CrushWarhead` | +0xFAC | WarheadTypeClass* | `RulesClass::ReadCombatDamage` (0x66c35d) | Warhead used for crush damage |

### RulesClass ([IQ] section)

| INI Key | Byte Offset | Type | Parsed In | Description |
|---------|-------------|------|-----------|-------------|
| `AutoCrush` | +0x1448 | int (IQ level) | `RulesClass::ReadIQ` (0x674240) | AI IQ level required for auto-crush behavior |

### Veteran/Elite Ability

The string `"CRUSHER"` is ability index **17 (0x11)** in the VeteranAbilities/EliteAbilities
array at 0x8463B8. Units with this ability gain crush capability at veteran/elite rank
even if their type doesn't have `Crusher=yes`.

Ability check: `TechnoClass::HasWeaponAbility(0x11)` at 0x70D0D0.
- Normal rank checks: `TechnoTypeClass + 0x29C + 0x11 = +0x2AD`
- Elite rank checks: `TechnoTypeClass + 0x2AE + 0x11 = +0x2BF`

### MovementZone Enum

`CrusherAll` is MovementZone enum value **12** (string at 0x81BAD0 in the enum table
at 0x81BA88). This is a pathfinding property, not directly a crush INI key.

---

## 2. TechnoClass::CanCrushCheck (0x5F6CD0)

```
bool __thiscall CanCrushCheck(TechnoClass* victim, TechnoClass* crusher)
```

**Calling convention:** `this` (ECX) = potential victim; stack arg = the crusher.
Returns true if the crusher CAN crush this victim.

### Block 1: OmniCrusher Check

```
if (crusher != NULL) {
    TechnoTypeClass* crusherType = crusher->GetTechnoType();    // vtable+0x84
    if (crusherType->OmniCrusher) {                              // +0xD29
        if (victim != NULL && (victim->flags & 0x01)) {          // bit 0 of flags at +0x14 = IsOnMap
            TechnoTypeClass* victimType = victim->GetTechnoType(); // vtable+0x84
            if (!victimType->OmniCrushResistant) {                // +0xD2A
                int rtti = victim->WhatAmI();                      // vtable+0x2C
                if (rtti != 6) {                                   // 6 = BuildingClass
                    if (!crusher_house->Is_Ally(victim)) {         // allies NOT crushed
                        if (!victim->IsBeingWarped()) {            // vtable+0x160
                            return true;  // CRUSHABLE
                        }
                    }
                }
            }
        }
    }
}
```

Rules:
- Crusher must have `OmniCrusher=yes` on its TechnoType
- Victim must NOT have `OmniCrushResistant=yes`
- Victim must NOT be a Building (RTTI 6)
- Crusher must NOT be allied with victim
- Victim must NOT be in a temporal/warp state (vtable+0x160 timer check at +0x18C/+0x194)

### Block 2: Regular Crushable Check

```
ObjectTypeClass* victimObjType = victim->GetObjectType();   // vtable+0x88
if (victimObjType->Crushable) {                              // +0x22D
    if (victim != NULL && (victim->flags & 0x01)) {
        if (*(byte*)(victim + 0x2A4) == 0) {                  // NOT deployed/prone
            if (!crusher_house->Is_Ally(victim)) {
                if (!victim->IsBeingWarped()) {
                    return true;  // CRUSHABLE
                }
            }
        }
    }
}
return false;
```

Rules:
- Victim's ObjectType must have `Crushable=yes`
- Victim must NOT be in deployed state (byte at instance +0x2A4 != 0)
- Crusher must NOT be allied with victim
- Victim must NOT be in a temporal/warp state

### Key vtable offsets (on AbstractClass/ObjectClass hierarchy)

| Vtable Offset | Function | Description |
|---------------|----------|-------------|
| +0x2C | `WhatAmI()` | Returns RTTI type ID |
| +0x84 | `GetTechnoType()` (trampoline) | Delegates to vtable+0x88 |
| +0x88 | `GetTechnoType()` (impl) | Returns TechnoTypeClass* (e.g. UnitClass+0x6C4, InfantryClass+0x6C0) |
| +0x160 | `IsBeingWarped()` | Timer-based check at +0x18C/+0x194; true = immune to crush |

### RTTI type IDs used

| ID | Class |
|----|-------|
| 1 | UnitClass |
| 6 | BuildingClass |
| 0xF (15) | InfantryClass |

---

## 3. MapClass::Check_Crushable_Obstacle (0x578AD0)

```
bool Check_Crushable_Obstacle(TechnoClass* mover, CellStruct* cell)
```

Returns true if the cell is passable (no blocking obstacle). Returns false if
a gate building blocks the path.

### Logic

```c
CellClass* cellObj = CellArray[cell->Y * 0x200 + cell->X];
ObjectClass* obj = cellObj->FirstObject;    // offset +0xE4

while (obj != NULL) {
    if (obj != mover) {
        if (obj->WhatAmI() == 6) {          // BuildingClass
            BuildingTypeClass* bldType = obj->house->BuildingType;  // obj[0x148]
            if (bldType->Gate) {             // +0x16B7
                if (mover_house->Is_Ally(obj)) {
                    // Allied gate - check garrison status
                    return CanPassGate(obj);  // FUN_00452540
                } else {
                    // Enemy gate blocks if it can be garrisoned
                    return !BuildingClass::CanGarrison(obj);
                }
            }
        }
    }
    obj = obj->NextObject;                   // +0x30 (obj[0xC] as int*)
}
return true;  // No blocking obstacle
```

Key detail: This function only checks **gate buildings**. Regular crushable objects
are handled in PerCellProcess, not here.

---

## 4. UnitClass::PerCellProcess (0x741700) — Crush Application

Called when a unit finishes entering a cell. This is where crush damage is actually applied.

### Entry Conditions

```c
void __thiscall PerCellProcess(UnitClass* this, CellStruct cell, char entering)
```

The function first checks:
1. **Bridge cell** (cell flags & 0x100): determines if on bridge or below
2. **Crush capability**: `this->UnitType->Crusher (+0xD28)` OR
   `TechnoClass::HasWeaponAbility(0x11)` (CRUSHER veteran ability)

If neither condition is met, crush processing is skipped entirely.

### Phase 1: Cell Scatter (entering == true)

When first entering a cell (`entering != 0`), instead of crushing, the function
calls `CellClass::Scatter_Objects` to push objects out of the way.
- Bridge cell: scatters from AltOccupants list (offset +0xE8), flag=1
- Normal cell: scatters from Occupants list (offset +0xE4), flag=0

### Phase 2: Crush Processing (entering == false)

When fully in the cell, iterates occupants and checks each for crushing:

```c
// Choose occupant list based on bridge status
ObjectClass* obj = bOnBridge ? cell->AltFirstObject : cell->FirstObject;

bool didCrush = false;

while (obj != NULL) {
    // 1. Check if crushable
    if (!obj->CanCrushCheck(this)) {
        next = obj->NextObject;
        continue;
    }

    // 2. Ally check with IsTrain override
    if (crusher_house->Is_Ally(obj)) {
        if (!this->UnitType->IsTrain) {    // +0xC94
            next = obj->NextObject;
            continue;  // Don't crush allies (unless train)
        }
    }

    // 3. Distance check
    Coords objCoords = obj->GetCoords();
    int distSq = DistanceSquared(this, objCoords);
    if (distSq > 0x3FFF) {                 // ~128 leptons max
        next = obj->NextObject;
        continue;
    }

    // 4. InLimbo/falling check
    if (*(byte*)(obj + 0x8D) != 0) {       // skip if in limbo
        next = obj->NextObject;
        continue;
    }

    // 5. Special case: Infantry pickup instead of crush
    if (obj->WhatAmI() == 0xF) {           // InfantryClass
        InfantryTypeClass* infType = obj->InfantryType;  // +0x6C0
        if (infType->field_0xEC6) {         // special absorbable flag
            if (obj->Transport == this) {   // obj+0x5A4 == crusher
                if (!this->UnitType->IsTrain) {
                    // ABSORB: pick up infantry instead of crushing
                    next = obj->NextObject;
                    this->Facing = obj->Facing;   // copy facing (+0x41A)
                    this->vtable[0xDC](0);         // stop current action
                    this->vtable[0x3D4](obj->House, 1);  // transfer ownership
                    obj->vtable[0xF8]();            // remove from game
                    continue;
                }
            }
        }
    }

    // 6. CRUSH DEATH — actual kill sequence
    next = obj->NextObject;
    didCrush = true;

    // Check if victim is UnitClass (RTTI 1) for special tilt flag
    if (obj->WhatAmI() == 1) {
        crushedUnit = true;  // used for tilt angle
    }

    // Copy crusher coords for anim/sound placement
    Coords crushCoords = this->Coords;   // +0x9C

    // Get victim's TechnoType for CrushSound
    TechnoTypeClass* victimType = obj->GetTechnoType();
    int crushSoundIdx = victimType->CrushSound;  // +0x1F0

    // Play crush sound at crush location
    VocClass::PlayAt(crushSoundIdx, &crushCoords, 0);

    // Free any mind-controlled units this victim was controlling
    obj->vtable[0x170]();      // FreeAllMindControlCaptures

    // Record the kill (passes crusher as killer for score/EVA)
    obj->vtable[0xE0](this);   // RecordKill(killerUnit)

    // Finalize death
    obj->vtable[0x124](0);     // MarkForDeletion
    obj->vtable[0xD4]();       // Destroy/UnInit
    obj->vtable[0xF8]();       // RemoveFromGame
}
```

### Phase 3: TiltsWhenCrushes Effect

After the crush loop, if any object was crushed:

```c
if (didCrush) {
    this->vtable[0x45C](0);    // Check TiltsWhenCrushes flag on type

    if (tiltsWhenCrushes && this->TiltAngle == 0.0f) {
        this->TiltAngle = -0.05f;   // tilt forward (offset +0x334)
    }
}
```

The tilt value `0xBD4CCCCD` as float = approximately **-0.05 radians** (about 2.9 degrees
forward tilt). This is a visual-only effect that makes the vehicle dip slightly when it
runs something over.

---

## 5. FUN_005f6560 — Distance Squared (0x5F6560)

```c
int DistanceSquared(TechnoClass* a, Coords* b) {
    Coords* aPos = a->GetCoords();
    int dx = aPos->X - b->X;
    int dy = aPos->Y - b->Y;
    return dx*dx + dy*dy;
}
```

The threshold is `> 0x3FFF` (16383). Since `128^2 = 16384`, this means objects must be
within approximately **128 leptons** (half a cell width) to be crushed. This prevents
crushing objects on the far edge of a cell.

---

## 6. Complete Crush Decision Flowchart

```
Unit enters cell
  |
  +-- Does unit have Crusher=yes OR veteran CRUSHER ability?
  |     NO --> exit (no crush processing)
  |     YES --> continue
  |
  +-- Is unit still entering cell? (entering==true)
  |     YES --> scatter objects in cell, exit
  |     NO --> continue to crush checks
  |
  For each occupant in cell:
  |
  +-- CanCrushCheck(victim, crusher):
  |   |
  |   +-- Block 1: crusher has OmniCrusher?
  |   |     YES --> victim NOT OmniCrushResistant?
  |   |               YES --> victim NOT a building?
  |   |                         YES --> NOT allies? NOT warped? --> CRUSHABLE
  |   |
  |   +-- Block 2: victim has Crushable?
  |         YES --> victim NOT deployed (byte +0x2A4)?
  |                   YES --> NOT allies? NOT warped? --> CRUSHABLE
  |
  +-- CanCrushCheck returned false? --> skip
  |
  +-- Is victim an ally AND crusher NOT IsTrain? --> skip (allies protected)
  |
  +-- Distance > 128 leptons? --> skip
  |
  +-- Victim in limbo (byte +0x8D)? --> skip
  |
  +-- Special infantry pickup case? --> absorb instead of crush
  |
  +-- CRUSH: play sound, free mind control, record kill, destroy victim
```

---

## 7. Key Behavioral Notes

1. **OmniCrusher vs Regular Crusher**: OmniCrush ignores `Crushable=no` but respects
   `OmniCrushResistant=yes`. Regular crush only works on `Crushable=yes` objects.

2. **Buildings are immune to OmniCrush**: Even with OmniCrusher, buildings (RTTI 6)
   cannot be crushed. The WhatAmI check explicitly blocks this.

3. **Allies are never crushed** (by default). The `HouseClass::Is_Ally` check prevents
   friendly-fire crushing. Exception: **`IsTrain=yes`** units crush everything including allies.

4. **Deployed infantry**: The `*(byte*)(victim + 0x2A4)` check in the regular Crushable
   path means deployed GIs (byte at +0x2A4 set when deployed) are NOT crushable by regular
   crushers. This is separate from `DeployedCrushable` which handles the case at
   InfantryTypeClass+0xEAC.

5. **Temporal immunity**: Objects being chrono'd/warped (vtable+0x160 returns true) are
   immune to all crushing.

6. **CRUSHER veteran ability**: Units can gain crush capability through the
   `VeteranAbilities=CRUSHER` or `EliteAbilities=CRUSHER` INI key, even without
   `Crusher=yes` on their type.

7. **CrushWarhead**: Stored at `RulesClass+0xFAC`, parsed from `[CombatDamage]CrushWarhead`.
   Used by the RecordKill/ReceiveDamage calls during crush death, not directly in PerCellProcess.

8. **Distance gate**: Objects must be within 128 leptons (~half cell) to be crushed.
   This prevents crushing objects at the far edge of a large cell.

9. **TiltsWhenCrushes**: A cosmetic effect that tilts the crusher forward by ~0.05 radians
   when it crushes something. The tilt value is stored at instance offset +0x334.

10. **AutoCrush IQ level**: At `RulesClass+0x1448`, parsed from `[IQ]AutoCrush`. This is
    the AI IQ threshold for automatic crush-seeking behavior. The per-type `AutoCrush`
    boolean at TechnoTypeClass+0xD2D is a separate flag.

---

## 8. Cross-references / Callers of CanCrushCheck

| Address | Caller | Context |
|---------|--------|---------|
| 0x7417C1 | `UnitClass::PerCellProcess` | Actual crush application |
| 0x741569/656 | `FUN_007414E0` | Pre-crush check during movement |
| 0x4B1911 | `DriveLocomotionClass::Process_Drive_Track` | Drive track crush check |
| 0x6A0F9D | `ShipLocomotionClass::Process_Drive_Track` | Ship crush check |
| 0x743927 | `FUN_007438F0` | Additional crush check |
| 0x73FB4A/FD17/FE7F | `UnitClass::Can_Enter_Cell` | Pathfinding: can we crush to enter? |
