# Scatter Dispatch System -- Complete Deep Dive (Ghidra Report)

## Overview

The scatter system has two distinct mechanisms:
1. **CellClass::Scatter_Objects (0x481670)** -- tells occupants of a cell to flee
2. **Per-class Scatter virtual (vtable+0x174)** -- the per-object handler that computes where to go

This report covers the full trigger -> dispatch -> per-class-handler flow.

---

## 1. CellClass::Scatter_Objects (0x481670)

### Signature

```c
void __thiscall CellClass::Scatter_Objects(
    CellClass* this,         // ECX
    CoordStruct* coord,      // param_2 -- direction hint (or NullCoord for any direction)
    int threat,              // param_3 -- passed through to each Scatter() call (typically 1)
    int force,               // param_4 -- if nonzero: skip eligibility checks, scatter everything
    char on_bridge           // param_5 -- 0 = ground occupants (cell+0xE4), 1 = bridge occupants (cell+0xE8)
);
```

### Full Pseudocode

```c
void CellClass::Scatter_Objects(CoordStruct* coord, int threat, int force, char on_bridge) {
    // Select occupant list based on bridge flag
    ObjectClass* first;
    if (on_bridge == 0)
        first = this->FirstObject;       // +0xE4
    else
        first = this->BridgeFirstObject; // +0xE8

    // PHASE 1: Pre-scan for ELITE occupants (only when force == 0)
    // Assembly: LEA ECX, [EAX + 0x150] before FUN_00750010 call
    // So it checks techno->Veterancy >= 2.0 (elite rank)
    bool has_elite = false;
    if (force == 0) {
        for (ObjectClass* obj = first; obj != NULL; obj = obj->NextObject) {
            ObjectClass* techno = FilterToTechno(obj);      // FUN_0040dd70
            if (techno != NULL && IsElite(techno + 0x150)) { // FUN_00750010
                has_elite = true;
                break;
            }
        }
    }

    // PHASE 2: Collect occupants into temp array (max 10)
    // Re-fetch first object (in case list changed)
    if (on_bridge == 0)
        first = this->FirstObject;
    else
        first = this->BridgeFirstObject;

    ObjectClass* objects[10];
    int count = 0;
    for (ObjectClass* obj = first; obj != NULL; obj = obj->NextObject) {
        if (count < 0 || DynamicVector_Resize(10, NULL)) {
            objects[count++] = obj;
        }
    }

    // PHASE 3: Dispatch scatter to each collected object
    for (int i = 0; i < count; i++) {
        ObjectClass* techno = FilterToTechno(objects[i]);  // FUN_0040dd70

        bool should_scatter = false;

        if (has_elite)                                     // elite unit in cell -> all scatter
            should_scatter = true;
        else if (force != 0)                               // forced scatter (e.g. crusher)
            should_scatter = true;
        else if (Rules->PlayerScatter)                     // g_RulesClass + 0x17ED
            should_scatter = true;
        else if (techno != NULL) {
            if (HasWeaponAbility(techno, ABILITY_SCATTER)) // ability index 3 (vet/elite check)
                should_scatter = true;
            else if (techno->Owner->CurrentIQ >= Rules->IQ_Scatter) // HouseClass+0x24C >= Rules+0x144C
                should_scatter = true;
        }

        if (should_scatter) {
            objects[i]->vt->Scatter(coord, threat, force);  // vtable + 0x174
        }
    }
}
```

### Key Logic Details

**The `has_elite` pre-scan**: When `force=0`, the function first scans occupants looking
for any **elite-rank** techno (veterancy >= 2.0 at TechnoClass+0x150). If found, ALL techno
occupants in the cell get their Scatter() called unconditionally. This means an elite unit
arriving in a cell causes everything else to flee regardless of IQ or abilities. The pre-scan
is skipped entirely when `force=1` (because force already scatters everyone).

**Scatter eligibility (when force=0)**: An occupant only receives its Scatter() call if at least
one of these is true:
- An elite-rank techno (veterancy >= 2.0) exists in the cell
- `[CombatDamage] PlayerScatter=yes` is set in rules
- The occupant has the SCATTER veteran/elite ability (checked via HasWeaponAbility(3))
- The occupant's owner house CurrentIQ >= `[IQ] Scatter=` threshold (AI only)

---

## 2. FUN_0040dd70 -- Type Filter (0x40DD70)

### Pseudocode

```c
ObjectClass* FilterToTechno(ObjectClass* obj) {
    if (obj == NULL) return NULL;

    int rtti = obj->vt->What_Am_I();  // vtable + 0x2C
    switch (rtti) {
        case 1:   // UnitClass (Vehicle)
        case 2:   // AircraftClass
        case 6:   // BuildingClass
        case 0xF: // InfantryClass
            return obj;
        default:
            return NULL;
    }
}
```

### RTTI Values

| RTTI | Class | Verified From |
|------|-------|---------------|
| 1 | UnitClass | `mov eax, 1; ret` at 0x746E20 |
| 2 | AircraftClass | `mov eax, 2; ret` at 0x41C180 |
| 6 | BuildingClass | `mov eax, 6; ret` at 0x459EC0 |
| 0xF (15) | InfantryClass | `mov eax, 0xF; ret` at 0x523340 |

This filters out non-Techno objects (terrain objects, overlays, smudges, etc.) that
can exist in a cell's occupant list but cannot scatter.

---

## 3. FUN_00750010 -- Veterancy Level Check (0x750010)

**CRITICAL**: Assembly at 0x4816AC shows `LEA ECX, [EAX + 0x150]` before calling
FUN_00750010. The parameter is NOT the object base -- it's `techno + 0x150` which
is the **Veterancy float field** (TechnoClass+0x150, confirmed by multiple docs).

### Pseudocode

```c
// Checks if techno is ELITE rank (veterancy >= 2.0f)
bool IsElite(TechnoClass* techno) {
    float vet = techno->Veterancy;  // TechnoClass + 0x150
    return vet >= 2.0f;             // _DAT_007e37b4 = 0x40000000 = 2.0f
}
```

### Related: Volume__IsNormal (0x74FF90)

```c
// Checks if techno is VETERAN rank (1.0 <= veterancy < 2.0)
bool IsVeteran(TechnoClass* techno) {
    float vet = techno->Veterancy;  // TechnoClass + 0x150
    return vet >= 1.0f && vet < 2.0f;  // 0x3F800000, 0x40000000
}
```

The veterancy classification used throughout the scatter system:
- `< 1.0` -- rookie (no vet abilities)
- `1.0 <= v < 2.0` -- veteran (has VeteranAbilities)
- `>= 2.0` -- elite (has VeteranAbilities + EliteAbilities)

These same functions are used in `HasWeaponAbility` to determine which ability
array (veteran vs elite) to check.

---

## 4. TechnoClass::HasWeaponAbility (0x70D0D0)

### Signature and Pseudocode

```c
bool __thiscall TechnoClass::HasWeaponAbility(TechnoClass* this, int ability_index) {
    // First check: must be at least veteran rank
    // (both IsVeteran and IsElite pass via techno+0x150 veterancy float)
    if (!IsVeteran(&this->Veterancy) && !IsElite(&this->Veterancy))
        return false;  // rookie units never have abilities

    // Get the TechnoTypeClass
    TechnoTypeClass* type = this->vt->GetTypeClass();  // vtable + 0x84

    // Veteran rank: check VeteranAbilities array
    if (IsVeteran(&this->Veterancy)) {
        if (type->VeteranAbilities[ability_index])      // TypeClass + 0x29C + index
            return true;
    }

    // Elite rank: check BOTH VeteranAbilities AND EliteAbilities
    if (IsElite(&this->Veterancy)) {
        if (type->VeteranAbilities[ability_index] ||    // TypeClass + 0x29C + index
            type->EliteAbilities[ability_index])        // TypeClass + 0x2AE + index
            return true;
    }

    return false;
}
```

Note: Elite units check both arrays because they inherit all veteran abilities plus
gain additional elite-only abilities.

### Called with ability_index = 3 (SCATTER)

When called from `CellClass::Scatter_Objects`, `ability_index = 3`.

The abilities array at TechnoTypeClass + 0x29C (veteran) and +0x2AE (elite) are
parsed from the INI keys `VeteranAbilities=` and `EliteAbilities=` as comma-separated
ability name lists.

### Ability Enum (from string table at 0x8463B8)

| Index | Name | Meaning |
|-------|------|---------|
| 0 | FASTER | Speed bonus |
| 1 | STRONGER | Armor bonus |
| 2 | FIREPOWER | Damage bonus |
| **3** | **SCATTER** | **Can scatter from threats** |
| 4 | ROF | Rate of fire bonus |
| 5+ | ... | Other abilities |

So `HasWeaponAbility(3)` checks: "Does this unit have SCATTER in its VeteranAbilities
or EliteAbilities, considering its current veterancy level?"

---

## 5. PlayerScatter Flag: g_RulesClass + 0x17ED

### INI Key

```ini
[CombatDamage]
PlayerScatter=no   ; default in rules(md).ini
```

### Parsing Location

Parsed in the massive `RulesClass::ReadCombatDamage` function at 0x66BBB0:

```c
// At offset 0x66CEE0 in the function:
bool val = CCINIClass::ReadBool("CombatDamage", "PlayerScatter", rules->PlayerScatter);
rules->PlayerScatter = val;  // stored at RulesClass + 0x17ED
```

The xref from `FUN_0066bbb0` at address 0x66CEDD confirms the string "PlayerScatter"
at 0x83ACF8 is read and stored at `param_1 + 0x17ED`.

### Effect in Scatter_Objects

When `PlayerScatter=yes`, the `g_RulesClass_Instance + 0x17ED` byte is nonzero, and
**every** techno in the cell gets its Scatter() called regardless of IQ level or
abilities. This makes player-controlled units auto-scatter from threats just like
AI units do.

Default is `no`, meaning player units only scatter if they have the SCATTER ability
at their veterancy level, or if their house IQ is high enough (only applies to AI).

### Nearby Flags (same function, consecutive fields)

| Offset | INI Key | Default | Purpose |
|--------|---------|---------|---------|
| 0x17E5 | TiberiumExplosive | | Tiberium chain explosions |
| 0x17EB | PlayerAutoCrush | | Player units auto-crush |
| 0x17EC | PlayerReturnFire | | Player units auto-retaliate |
| **0x17ED** | **PlayerScatter** | **no** | **Player units auto-scatter** |
| 0x17E9 | TreeTargeting | | Can target trees |

---

## 6. IQ Scatter Threshold: g_RulesClass + 0x144C

### INI Key

```ini
[IQ]
Scatter=2   ; IQ level needed for auto-scatter behavior
```

### Parsing Location

Parsed in `RulesClass::ReadIQ` at 0x674240:

```c
// At offset 0x674323 in the function:
int val = CCINIClass::ReadInt("IQ", "Scatter", rules->IQ_Scatter);
rules->IQ_Scatter = val;  // stored at RulesClass + 0x144C
```

### Effect in Scatter_Objects

The check `rules->IQ_Scatter <= owner->IQLevel` (where `owner->IQLevel` is at
`techno->Owner + 0x24C`) determines whether an AI house's units are smart enough
to scatter. Human players always have IQ 0, so this check only applies to AI.

The actual comparison from the decompilation:
```c
Rules->IQ_Scatter <= *(int*)(*(int*)(techno + 0x21C) + 0x24C)
                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                      techno->Owner (HouseClass*) -> CurrentIQ (int)
```
Verified: Techno+0x21C = Owner (HouseClass ptr), HouseClass+0x24C = CurrentIQ.
Human players have IQ=0, so `Scatter=2` means only AI with IQ >= 2 passes this check.

### MissionControlClass Scatter Flag

Separate from the IQ system, each mission type has its own `Scatter=` boolean parsed
from INI (e.g., `[Sleep] Scatter=no`, `[Attack] Scatter=no`). This is read by
`FUN_005b3760` (MissionControlClass::ReadINI) and stored at byte offset +9 in the
per-mission MissionControlClass entry. The per-class Scatter implementations
(InfantryClass, UnitClass) check this flag via `MissionClass::GetMissionTimerEntry()`
to determine if the current mission allows scattering.

---

## 7. Per-Class Scatter Handlers (vtable + 0x174)

### Vtable Verification

| Class | Vtable Base | +0x174 Value | Function |
|-------|-------------|-------------|----------|
| UnitClass | 0x7F5C70 | 0x00743A50 | `UnitClass::Scatter` |
| InfantryClass | 0x7EB058 | 0x0051D0D0 | `InfantryClass::Scatter` |

---

### 7a. UnitClass::Scatter (0x743A50)

```c
void __thiscall UnitClass::Scatter(
    UnitClass* this,
    CoordStruct* coord,   // direction hint (or NullCoord)
    uint param_3          // lower byte = force flag
) {
    // GATE 1: Check if alive
    if (!this->vt->IsAlive())   // vtable + 0x28C
        return;

    // GATE 2: Reject TeleportLocomotion
    // Query piggyback locomotor, check CLSID != TeleportLocomotion
    IID loco_id;
    this->Locomotor->GetClassID(&loco_id);
    if (loco_id == CLSID_TeleportLocomotion)
        return;  // teleporting units don't scatter

    // GATE 3: Check mission allows scatter
    MissionControlClass* mc = GetMissionTimerEntry(this->CurrentMission);
    if (mc->Scatter == false && force == false)
        return;

    // GATE 4: Scatter timer cooldown
    if (CDTimerClass::Remaining(this->ScatterTimer) != 0)
        return;  // still on cooldown from last scatter

    // GATE 5: Has destination and not forced
    if (this->NavTarget != NULL && force == false)
        return;  // already moving somewhere, don't interrupt

    // GATE 6: Deploying/unloading
    if (this->IsDeploying || this->field_6E1 || this->field_6E2)
        return;

    // GATE 7: Locomotor allows it
    if (!this->Locomotor->IsMovable())  // locomotor vtable + 0x60
        return;

    // === DESTINATION COMPUTATION ===

    if (coord == NullCoord) {
        // No direction hint: random scatter
        //   - If mission's Retaliate flag set AND is infantry: return (???)
        //   - If has nav target AND (not forced OR field_6AF set): return
        //   - If has target AND not forced: 75% chance to just return (random 1-4, return if != 1)

        // Compute facing from current position to coord
        int facing = atan2(coord.Y - pos.Y, pos.X - coord.X);
        int random_offset = Random(0, 2) - 1;
        int scatter_dir = random_offset + ((facing >> 12) + 1 >> 1);

        // Try 8 directions starting from scatter_dir
        CellStruct best_cell = {-1, -1};
        CellStruct fallback_cell = {-1, -1};
        for (int i = 0; i < 8; i++) {
            int dir = (i + scatter_dir) & 7;
            CellStruct candidate = current_cell + DirectionOffsets[dir];

            if (!MapClass::Is_Cell_In_Playfield(candidate))
                continue;

            int height = CellClass::Get_Effective_Height(candidate_cell, 0);
            int passability = this->vt->Can_Enter_Cell(candidate_cell, dir, height);
            if (passability != 0)
                continue;

            if (fallback_cell == NULL_CELL)
                fallback_cell = candidate;

            // Check Z-level compatibility
            // Compute leptons, check via FUN_006d6410 (subcell accuracy check)
            // If exact match and cell not bridge-occupied: this is ideal
            best_cell = candidate;
            break;
        }

        // Use best_cell if found, else fallback_cell
        CellStruct dest = (best_cell != NULL_CELL) ? best_cell : fallback_cell;
        if (dest == NULL_CELL)
            return;  // nowhere to go

        // Issue move mission
        this->vt->SetMission(MISSION_MOVE, 0);  // vtable + 0x1E8
    } else {
        // Has direction: use Find_Nearby_Passable_Cell
        CellStruct dest = FootClass::Find_Nearby_Passable_Cell(...);
        if (dest == NULL_CELL)
            return;
    }

    // Move to the scatter destination cell
    CellClass* dest_cell = MapClass::Get_CellClass(dest);
    this->vt->SetDestinationCell(dest_cell);  // vtable + 0x480
}
```

### 7b. InfantryClass::Scatter (0x51D0D0)

InfantryClass has a **different and more complex** Scatter implementation.

```c
void __thiscall InfantryClass::Scatter(
    InfantryClass* this,
    CoordStruct* coord,      // param_2 -- direction hint
    uint param_3,            // lower byte = force
    char param_4             // unknown, passed from higher caller
) {
    char force = (char)param_3;
    InfantryTypeClass* type = this->Type;  // param_1[0x1b0], byte offset 0x6C0

    // SPECIAL: Crawling sequences (0x1B-0x1E = prone/deploy anims)
    int seq = this->SequenceIndex;  // param_1[0x1b1], byte offset 0x6C4
    if ((seq == 0x1B || seq == 0x1C || seq == 0x1D || seq == 0x1E)
        && force != 0 && param_4 != 0)
    {
        // Force-interrupt prone animation
        this->vt->StopAnimation();  // vtable + 0x558
    }
    else {
        // Check if player-owned
        bool is_player = IsPlayerOwned(this);  // FUN_0050b730
        if (is_player) {
            // Player-owned infantry in crawling sequence: refuse to scatter
            if (seq == 0x1B || seq == 0x1C || seq == 0x1D || seq == 0x1E)
                return;
        }
    }

    // GATE 1: Locomotor must be ready
    if (this->Locomotor == NULL)
        Assert();
    if (this->Locomotor->IsBusy())  // locomotor vtable + 0x10
        force = false;  // downgrade to non-forced if locomotor is busy

    // GATE 2: Mission must allow scatter
    MissionControlClass* mc = GetMissionTimerEntry(this->CurrentMission);
    if (mc->Scatter == false && force == false)
        return;

    // GATE 3: Scatter flag in InfantryTypeClass
    if (type->Fearless == false && this->Target != NULL && force == false)
        return;  // type+0xEBF = "Fearless" flag

    // GATE 4: Per-mission scatter permission table
    // DAT_007eaf7c is a per-mission-index boolean table
    if (seq != -1 && seq != 0x1F) {
        if (g_MissionScatterTable[seq * 4] == false)
            return;  // this mission type does not allow scatter
    }

    // GATE 5: PlayerScatter / IQ / Ability check (same as dispatch)
    if (Rules->PlayerScatter == false) {
        bool has_scatter_ability = TechnoClass::HasWeaponAbility(this, ABILITY_SCATTER);
        if (!has_scatter_ability && force == false) {
            bool is_player = IsPlayerOwned(this);
            if (is_player) {
                if (force == false && this->NavTarget == NULL)
                    return;
                // fall through to scatter
            }
        }
    }
    else {
        // PlayerScatter is enabled
        if (force == false) {
            if (type->Fearless == false)  // type+0xEBF
                return;
        }
    }

    // === DESTINATION COMPUTATION ===

    if (coord == NullCoord) {
        // Random direction, use Find_Nearby_Passable_Cell with SpeedType
        CellStruct dest = FootClass::Find_Nearby_Passable_Cell(..., type->SpeedType, ...);
        if (dest == NULL_CELL)
            return;

        CellClass* dest_cell = MapClass::Get_CellClass(dest);
        this->vt->SetDestinationCell(dest_cell);  // vtable + 0x480
        this->Locomotor->Force_Move();             // locomotor vtable + 0x40
        return;
    }

    // Direction-based scatter (identical pattern to UnitClass):
    // Compute facing from coord, add random offset, try 8 directions
    int facing = atan2(coord.Y - pos.Y, pos.X - coord.X);
    int random_offset = Random(0, 4) - 2;  // NOTE: wider random range than UnitClass!
    int scatter_dir = random_offset + ((facing >> 12) + 1 >> 1) & 7;

    // 8-direction scan for passable cell
    CellStruct best_cell, fallback_cell;
    for (int i = 0; i <= 7; i++) {
        int dir = (i + scatter_dir) & 7;
        CellStruct candidate = current_cell + DirectionOffsets[dir];
        // ... Can_Enter_Cell check, Z-level check, subcell accuracy check ...
    }

    // Issue move mission and set destination
    this->vt->SetMission(MISSION_MOVE, 0);
    CellClass* dest_cell = MapClass::Get_CellClass(best_or_fallback);
    this->vt->SetDestinationCell(dest_cell, 1);  // the extra 1 = force re-enter
}
```

### Key Differences: InfantryClass vs UnitClass Scatter

| Aspect | UnitClass | InfantryClass |
|--------|-----------|---------------|
| Crawling sequence check | No | Yes (refuses scatter during prone/deploy sequences 0x1B-0x1E) |
| Player-owned special handling | No | Yes (extra checks for player infantry) |
| Locomotor busy check | No explicit check | Downgrades force to false if locomotor is busy |
| Fearless type flag | Not checked | Checked at type+0xEBF |
| Mission scatter table | Uses MissionControlClass.Scatter only | Also checks g_MissionScatterTable at 0x7EAF7C |
| PlayerScatter interaction | Not checked (handled in dispatch) | Independently re-checks PlayerScatter and ability |
| Teleport locomotor reject | Yes (explicit CLSID check) | No (uses generic locomotor busy check) |
| Random offset range | Random(0,2)-1 = [-1,0,1] | Random(0,4)-2 = [-2,-1,0,1,2] |
| Scatter timer cooldown | Yes (CDTimerClass check) | No explicit timer |
| After scatter | SetDestinationCell only | SetDestinationCell + Locomotor::Force_Move |

---

## 8. Complete Scatter Dispatch Chain

### Pseudocode: Full Flow

```
TRIGGER (one of):
  - DriveLocomotionClass::Process_Movement -- Can_Enter_Cell returns 6 (friendly blocked)
  - DriveLocomotionClass::Process_Drive_Track -- mid-track CEC returns 6
  - ShipLocomotionClass::Process_Movement -- naval equivalent
  - WalkLocomotionClass::ProcessMovement -- infantry CEC returns 6
  - UnitClass::PerCellProcess -- crusher entering occupied cell
  - BuildingTypeClass::CanBePlacedAt -- building placement clearing
  - FUN_00449540 -- factory exit clearing (weapons factory)
  - TeleportLocomotionClass::HeadToCoord -- chrono movement
  - InfantryClass::Mission_Enter -- entering transport/building
  - FUN_005206b0 -- unknown context (infantry-related)
  - FUN_00514f70 -- unknown context

    |
    v

CellClass::Scatter_Objects(cell, coord, threat, force, on_bridge)
    |
    +-- Select occupant list: ground (cell+0xE4) or bridge (cell+0xE8)
    |
    +-- [if force==0] Pre-scan: any elite-rank techno (Veterancy >= 2.0)?
    |       -> sets has_elite flag
    |
    +-- Collect up to 10 occupants into temp array
    |
    +-- For each occupant:
    |       |
    |       +-- FilterToTechno (FUN_0040dd70):
    |       |       Is RTTI in {1=Unit, 2=Aircraft, 6=Building, 0xF=Infantry}?
    |       |       If not: skip this occupant
    |       |
    |       +-- Check scatter eligibility:
    |       |       has_elite?     -> scatter (elite in cell = everyone flees)
    |       |       force != 0?    -> scatter
    |       |       PlayerScatter? -> scatter
    |       |       HasAbility(SCATTER) at vet/elite rank? -> scatter
    |       |       Owner IQ >= IQ.Scatter? -> scatter
    |       |       None of above? -> SKIP this occupant
    |       |
    |       +-- Call occupant->vt->Scatter(coord, threat, force)
    |               |
    |               +-- [UnitClass::Scatter @ 0x743A50]
    |               |       Gate: IsAlive, not TeleportLoco, mission allows,
    |               |             scatter cooldown timer, not deploying
    |               |       Compute: 8-direction scan from facing/random
    |               |       Result: SetMission(MOVE) + SetDestinationCell
    |               |
    |               +-- [InfantryClass::Scatter @ 0x51D0D0]
    |                       Gate: not in prone sequence (unless forced),
    |                             locomotor not busy, mission allows,
    |                             Fearless flag, PlayerScatter/IQ/Ability
    |                       Compute: 8-direction scan (wider random range)
    |                       Result: SetMission(MOVE) + SetDestinationCell + Force_Move
```

---

## 9. INI Configuration Summary

### [CombatDamage] section (RulesClass fields)

| Key | Offset | Type | Default | Effect |
|-----|--------|------|---------|--------|
| PlayerScatter | +0x17ED | bool | no | If yes, all player units scatter regardless of IQ |
| PlayerReturnFire | +0x17EC | bool | | Related: player auto-retaliation |
| PlayerAutoCrush | +0x17EB | bool | | Related: player auto-crush |

### [IQ] section (RulesClass fields)

| Key | Offset | Type | Default | Effect |
|-----|--------|------|---------|--------|
| Scatter | +0x144C | int | 2 | Minimum IQ level for auto-scatter. Human = 0, so humans never pass this check. AI difficulty sets IQ level. |

### Per-Mission [MissionName] sections (MissionControlClass)

| Key | Offset in entry | Type | Example |
|-----|-----------------|------|---------|
| Scatter | +9 | bool | `[Sleep] Scatter=no`, `[Attack] Scatter=no` |
| Retaliate | +8 | bool | Related: mission-level retaliation flag |

Missions with `Scatter=no`: Sleep, Sticky, Attack, Retreat, and several others.
This prevents units in those mission states from scattering even when fired upon.

### VeteranAbilities / EliteAbilities

```ini
[UnitType]
VeteranAbilities=SCATTER,FIREPOWER  ; parsed into bool array at TypeClass+0x29C
EliteAbilities=SCATTER,STRONGER     ; parsed into bool array at TypeClass+0x2AE
```

The SCATTER ability (index 3) allows that unit to scatter when it reaches
veteran/elite rank, even if the owner's IQ would normally be too low.

---

## 10. Confidence Notes

| Finding | Confidence | Method |
|---------|-----------|--------|
| CellClass::Scatter_Objects full logic | 95% | Direct decompilation at 0x481670 |
| FilterToTechno types 1,2,6,0xF | 99% | Direct decompilation + verified What_Am_I for all 4 classes |
| Veterancy thresholds 1.0f/2.0f at +0x150 | 99% | Assembly `LEA ECX,[EAX+0x150]` + float constants 0x3F800000, 0x40000000 |
| HasWeaponAbility(3) = SCATTER ability | 99% | Traced ability string table at 0x8463B8, index 3 = "SCATTER" at 0x820BA0 |
| PlayerScatter at Rules+0x17ED | 99% | String xref "PlayerScatter" -> ReadBool -> store at param_1+0x17ED |
| IQ Scatter at Rules+0x144C | 99% | String xref "Scatter" in [IQ] ReadINI -> ReadInt -> store at param_1+0x144C |
| UnitClass::Scatter vtable+0x174 | 99% | Read 4 bytes at UnitClass_vtable+0x174 = 0x743A50, named function |
| InfantryClass::Scatter vtable+0x174 | 99% | Read 4 bytes at InfantryClass_vtable+0x174 = 0x51D0D0, named function |
| UnitClass::Scatter full logic | 80% | Decompilation somewhat garbled by optimizer; core flow clear but edge cases uncertain |
| InfantryClass::Scatter full logic | 85% | Large function, several gates verified but some field semantics inferred |
| MissionControlClass.Scatter at +9 | 95% | ReadBool with "Scatter" string writes to entry+9 in FUN_005b3760 |
