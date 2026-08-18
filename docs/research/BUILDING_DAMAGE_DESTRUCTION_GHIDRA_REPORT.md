# BuildingClass Damage Reception, Destruction & Immunity Systems — Ghidra Report

Confidence: HIGH (verified from binary decompilation, cross-referenced with existing reports)

---

## 1. Overview

Building damage flows through a three-level virtual dispatch chain:

```
BuildingClass::ReceiveDamage (0x00442230)
  → TechnoClass::ReceiveDamage (0x00701900)
    → ObjectClass::ReceiveDamage (0x005f5390)
```

Each level adds building-specific checks. This report focuses on the
**BuildingClass-specific** behavior — the TechnoClass and ObjectClass layers
are documented in `RECEIVE_DAMAGE_GHIDRA_REPORT.md` and
`IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md`.

---

## 2. BuildingClass::ReceiveDamage (0x00442230)

### Parameters

```c
int __thiscall BuildingClass::ReceiveDamage(
    int*             pDamage,       // +0x04: Pointer to damage amount (read/write)
    int              distance,      // +0x08: Distance from epicenter
    WarheadTypeClass* warhead,      // +0x0C: Warhead pointer
    TechnoClass*     attacker,      // +0x10: Source of damage (nullable)
    bool             ignoreDefenses // +0x14: Bypass all immunity checks
);
// Returns: DamageState enum (0=Unaffected, 1-3=condition, 4=Dead, 5=PostMortem)
```

### Phase 1: Self-Damage Guard

```c
if (this == attacker) {
    ObjectTypeClass* type = attacker->GetType();  // vtable+0x84
    if (type->field_0xCA0 == 0)  // DontScore or similar flag
        return 0;  // Building cannot damage itself unless flagged
}
```

### Phase 2: Pre-Damage State Recording

- Records current health ratio and animation frame (`GetCurrentFrame`)
- If attacker is non-null AND building is not cloaked (`vtable+0x80` returns false):
  - Updates owner house last-attacked timestamp: `Owner+0x54D8 = CurrentFrame`
  - Records attacker's AbstractType ID: `Owner+0x54DC = attacker->GetType()->ArrayIndex`
  - Calls `FUN_00708080(attacker)` — threat assessment / AI base defense response

### Phase 3: Building-Specific Immunity Checks

Two building-specific immunity checks occur BEFORE delegating to TechnoClass:

#### 3a. Wall Immunity (BuildingTypeClass+0x16BF)

```c
if (Type->field_0x16BF != 0 && ignoreDefenses == false) {
    // This is the "Wall" or "Gate" flag
    // Walls are immune to normal damage (only destroyed by specific warheads)
    return 0;
}
```

**INI Key:** This corresponds to a wall/gate-type building flag. Walls cannot receive
normal damage — they are only destroyed through `WallAbsoluteDestroyer` warhead checks
in `WarheadTypeClass::Detonate`, not through ReceiveDamage.

#### 3b. Insignificant + Gate Double-Check (BuildingTypeClass+0x16B6 AND +0x233)

```c
if (Type->field_0x16B6 != 0 && Type->Insignificant != 0) {
    // Both flags set: building is completely immune to damage
    return 0;
}
```

`+0x233` is `Insignificant` (TechnoTypeClass). `+0x16B6` is another building-specific
flag (likely `IsGate` or similar). When both are set, damage is blocked entirely.

### Phase 4: Dead-Already Check

```c
if (this->Health == 0) goto cleanup;  // Already dead, skip to cleanup
```

### Phase 5: Delegate to TechnoClass::ReceiveDamage

```c
int result = TechnoClass::ReceiveDamage(this, ...);
```

This call handles ALL the core immunity/damage/death logic documented in
`RECEIVE_DAMAGE_GHIDRA_REPORT.md`.

### Phase 6: Building Death Processing (result == 4, NowDead)

If the parent call returns 4 (NowDead), extensive building-specific cleanup occurs:

#### 6a. Docked Unit Handling (field_0x30C)

```c
if (this->field_0x30C != 0) {   // Docked unit
    // Damage the docked unit's movement accumulator
    docked->field_0xE8 *= _DAT_007e4460;  // Slow it down
}
```

This falls through to case 3 (ConditionRed handling):

#### 6b. Damage Sound (case 3, falls through from case 2)

```c
type = this->GetType();
if (type->field_0x538 == -1) {        // TechnoTypeClass+0x538
    // corrected 2026-05-28: was "!= -1"; binary shows "== -1" at case 3 in
    // BuildingClass__ReceiveDamage decompile. ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT
    VocClass::PlayAtCoord(0 /*sound index passed as 0 in thiscall*/, this->Location);
}
```

#### 6c. Foundation Debris Spawning (Warhead Sparky flag)

Iterates all foundation cells. For each cell where `Warhead+0x14A` (`Sparky`) is set:
- Computes random count based on `FoundationHeight + FoundationWidth + 5`
- Spawns debris animations using `Rules+0xB78` array (3 types of debris):
  - Cases 1-5: Small debris (array index 0), random 1-3 loop count
  - Cases 6-8: Medium debris (array index 1), random 1-3 loop count  
  - Case 9: Large debris (array index 2), loop count 1

Each debris is an AnimClass with speed 0x600 and random facing.

#### 6d. Death Processing (case 4)

**Undock unit:**
```c
if (this->field_0x2E4 != 0) {   // Docked/attached unit
    BuildingClass::UndockUnit(this);
}
```

**Free mind-controlled units:**
```c
if (this->CaptureManager != 0) {
    CaptureManagerClass::FreeAll();
}
```

**Chrono warp handling:**
```c
if (this->field_0x2AC != 0) {   // Has chrono warp state
    BuildingClass::DeployUnit_ChronoWarp(1);  // Emergency undeploy
}
```

**Eject docked units** (iterates through all objects near the building):
For each docked unit within 0x100 leptons distance (or unconditionally if
`Type+0x16CB` is set — likely `Factory` flag):
- Applies damage: `unit->ReceiveDamage(unit->Strength * 10, ...)`
  with `C4Warhead` (Rules+0xFA8), force_damage=true, delayed mode
  
For units farther away:
- Sends radio message 0x17 (RADIO_OVER_OUT) to eject them
- Clears their target (`unit->field_0x500 = 0`)

**Sell building if Type+0x157B is set (label disputed):**
```c
if (Type->field_0x157B != 0) {   // exact INI label unresolved
    BuildingClass::SellBuilding(this);
}
```

Fresh damage-fire verification shows the same `BuildingType+0x157B` byte is also
used by `BuildingClass::Update @ 0x0043FB20` to choose the damage-fire threshold:
zero selects `ConditionYellow`, nonzero selects `ConditionRed`. Older docs label
this byte as `LeaveRubble` or `CanBeOccupied`; treat the INI label as unresolved
until the parser/field map is audited.

**Destroy light source:**
```c
if (this->LightSource != NULL) {
    FUN_00554a80(0);  // Destroy point light
}
```

**Call DestructionEffects:** `vtable+0x4EC` — this is `BuildingClass::DestructionEffects` (0x004415F0).
<!-- corrected 2026-05-28: was "vtable+0x4EC = BuildingClass::OnDestroyed (0x00445880)";
     read_memory(0x007E43A8) [vtable base 0x007E3EBC + 0x4EC] → 0x004415F0 =
     BuildingClass__DestructionEffects, verified via get_function_by_address.
     ROOT_CAUSE: RTTI_LABEL_DRIFT -->

**Handle Temporal/IronCurtain timer carryover:**
```c
// If building had remaining IC duration, re-occupy map
int remaining = this->field_0x530;  // IC duration
if (this->field_0x528 != -1) {      // IC start frame
    int elapsed = CurrentFrame - this->field_0x528;
    if (elapsed < remaining) remaining -= elapsed;
    else remaining = 0;
}
if (remaining > 0) {
    vtable+0xF8();  // Remove from map occupancy
    BuildingClass::Place_OccupyMap();  // Re-establish
}
```

---

## 3. Complete Immunity Check Summary

Immunity checks occur across all three class levels. Here is the complete ordered list:

### BuildingClass Level (0x00442230)

| # | Check | Condition | Result |
|---|-------|-----------|--------|
| 1 | Self-damage guard | `this == attacker && !attacker->Type->field_0xCA0` | return 0 |
| 2 | Wall immunity | `Type+0x16BF != 0 && !ignoreDefenses` | return 0 |
| 3 | Insignificant+Gate | `Type+0x16B6 != 0 && Type+0x233 != 0` | return 0 |
| 4 | Already dead | `Health == 0` | skip to cleanup |

### TechnoClass Level (0x00701900)

| # | Check | Condition | Result |
|---|-------|-----------|--------|
| 5 | Type-based multiplier | Applied to damage (Building mult at Rules+0x104) | modifies *pDamage |
| 6 | Veterancy modifier | Vet/Elite YOURFIRE_POW abilities | modifies *pDamage |
| 7 | Minimum damage floor | `*pDamage < 1` after modifiers | clamp to 1 |
| 8 | TypeImmune | `Type->TypeImmune && attacker same type + same owner` | return 0 |
| 9 | **IronCurtain** | `IsIronCurtainActive() && !ignoreDefenses && !healing` | *pDamage=0, return 0 |
| 10 | **WarpingOut** | `IsWarpingOut() && !ignoreDefenses` | *pDamage=0, return 0 |
| 11 | Ammo absorption | `Type+0x6B1 set` | reduces ammo instead |
| 12 | Bunker/ForceShield | `field_0x2E4 != 0` (garrisoned building) | complex logic |
| 13 | **Radiation immune** | `WH->Radiation && Type->ImmuneToRadiation` | return 0 |
| 14 | **PsychicDamage immune** | `WH->PsychicDamage && Type->ImmuneToPsionicWeapons` | return 0 |
| 15 | **Poison immune** | `WH->Poison && Type->ImmuneToPoison` | return 0 |
| 16 | **Allied friendly fire** | `!WH->AffectsAllies && attacker is allied` | return 0 |
| 17 | **Psychedelic/MC** | `WH->Psychedelic` + multiple sub-checks | return 1 (MC applied) |

### ObjectClass Level (0x005f5390)

| # | Check | Condition | Result |
|---|-------|-----------|--------|
| 18 | Dead/zero damage | `Health < 1 || *pDamage == 0` | return 0 |
| 19 | **Insignificant** | `!ignoreDefenses && Type->Insignificant (+0x233)` | return 0 |
| 20 | Armor/Verses | `FUN_00489180(damage, armor_index)` | modifies *pDamage |
| 21 | Building min-damage | Building type: damage clamped to >= 1 | ensures non-zero |

---

## 4. BuildingClass::Limbo (0x00445880) [formerly mislabeled OnDestroyed]

<!-- corrected 2026-05-28: was "BuildingClass::OnDestroyed (0x00445880)";
     binary shows get_function_by_address(0x00445880) → BuildingClass__Limbo.
     vtable+0x4EC (read_memory 0x007E43A8) → 0x004415F0 =
     BuildingClass__DestructionEffects, NOT Limbo.
     ROOT_CAUSE: RTTI_LABEL_DRIFT — wrong function name bound to correct address. -->

0x00445880 is `BuildingClass__Limbo` — called when a building is removed from the
map (limbo). The cleanup described in this section happens in Limbo, not in a
separate "OnDestroyed" function. The `vtable+0x4EC` slot points to
`BuildingClass__DestructionEffects` (0x004415F0), which is the actual virtual
on-death effects handler (anim cleanup, sound, SpawnSurvivors, debris).

`BuildingClass__Limbo` is called during map removal / death processing and handles
all the resource/counter cleanup documented below.

### Parameter

```c
void __thiscall BuildingClass::OnDestroyed(void);
```

### Detailed Steps

#### 4a. Destroy Building Animations (field_0x5C8, 8 slots)

```c
for (int i = 0; i < 8; i++) {
    if (this->AnimSlots[i] != NULL) {       // +0x5C8 + i*4
        AnimSlots[i]->Destroy();            // vtable+0xF8
        AnimSlots[i] = NULL;
    }
}
```

Cleans up all 8 building animation slots (idle anims, production anims, etc.).

#### 4b. Factory/Spy Counter Decrements

**Factory count (Type+0x16CC):**
```c
if (Type->field_0x16CC != 0 && this->ActuallyPlacedOnMap) {
    Owner->field_0x538C--;   // Factory building count
    if (Owner->field_0x538C < 0) Owner->field_0x538C = 0;
}
```

**Storage/Refinery capacity (Type+0x16CB, likely Factory flag):**
```c
if (Type->field_0x16CB != 0 && this->ActuallyPlacedOnMap) {
    Owner->field_0x2D4 -= Type->field_0x1780;  // Storage capacity
    if (Owner->field_0x2D4 < 0) Owner->field_0x2D4 = 0;
}
```

**Power output adjustment (Type+0x1564, Type+0x1568):**
```c
if (Type->PowerOutput != 0 && this->ActuallyPlacedOnMap) {
    Owner->field_0x164 -= Type->PowerOutput;     // +0x1564
    if (Owner->field_0x164 < 0) Owner->field_0x164 = 0;
}
if (Type->PowerDrain != 0 && this->ActuallyPlacedOnMap) {
    Owner->field_0x168 -= Type->PowerDrain;      // +0x1568
    if (Owner->field_0x168 < 0) Owner->field_0x168 = 0;
}
```

#### 4c. Wall/Fence Reconnection (Type+0x16BE)

```c
if (Type->field_0x16BE != 0) {  // IsWall/IsLaserFence flag
    BuildingClass::ConnectWalls(this, 0);  // Recalculate wall connections
}
```

#### 4d. Sensor Array / Detect Disguise Removal

```c
if (g_MapEditorMode == 0) {
    if (Type->field_0x16C8 != 0)   // HasSensorArray
        vtable+0x4F8(DAT_0089c818);  // RemoveSensorArrayAt
    if (Type->field_0xD31 != 0)    // DetectDisguise
        vtable+0x500(DAT_0089c818);  // RemoveDetectDisguiseAt
}
```

#### 4e. Power Update Notification

```c
if ((Type->PowerOutput == 0 || Type->field_0x5EC == 0) && !this->IsCloaked()) {
    FUN_004561f0();  // Trigger power grid recalculation
}
```

#### 4f. Check Upgrades for Super Weapon

```c
bool hasSpecialUpgrade = false;
for (int i = 0; i < 3; i++) {
    if (this->Upgrades[i] != NULL && Upgrades[i]->field_0x1763 != 0) {
        hasSpecialUpgrade = true;
        break;
    }
}
```

#### 4g. Special Building Type Handling

**Nuclear Reactor (Type == Rules+0x87C):**
Destroys ore/tiberium overlays in 8 adjacent cells when the reactor explodes.

**Bridge repair huts (Type == Rules+0x86C or +0x874):**
Triggers adjacent bridge section refresh (cells -1 and +3 from building position).

**Bridge sections (Type == Rules+0x870 or +0x878):**
Similar to repair huts, refreshes adjacent cells.

#### 4h. Cell Occupancy Cleanup

```c
// Decrement building occupancy counter for each foundation cell
if (Type->FoundationType == 0) {  // Type+0xE58 == 0
    int width = GetFoundationWidth() + 2;
    int height = GetFoundationHeight() + 2;
    for (int y = 0; y < height; y++) {
        for (int x = 0; x < width; x++) {
            CellClass* cell = MapClass::GetCellAt(baseX + x - 1, baseY + y - 1);
            cell->field_0x122--;   // Occupancy counter
        }
    }
}
```

#### 4i. Screen Invalidation

```c
if (g_Tactical != 0) {
    // Get bounding rect and dirty the screen area
    TacticalClass::DirtyScreenRect(rect);
}
```

#### 4j. Destroy Special Anim (field_0x600)

```c
if (this->field_0x600 != NULL) {
    this->field_0x600->Destroy();  // vtable+0xF8
}
```

#### 4k. House Notifications

```c
HouseClass::Recount(this);       // Update house building counts
Owner->field_0x1FC = 1;          // Flag: needs base recenter
HouseClass::Recalc_Base_Center();

if (Owner == g_PlayerPtr) {
    DAT_00880cf4 = 1;             // UI update flag
    FUN_004f42f0(0);              // Refresh sidebar/UI
}
```

#### 4l. Register Destruction

```c
FUN_0050a490(this);  // House::RegisterBuildingLoss or similar
```

#### 4m. Call Parent Destructor Chain

```c
FUN_006f6ac0();  // TechnoClass-level destruction
```

#### 4n. Super Weapon Upgrade Notification

```c
if (hasSpecialUpgrade) {
    FUN_00509130();  // Recalculate super weapon availability
}
```

#### 4o. Radar Update

```c
if (Type->RadarRange != 0) {  // Type+0xEB8
    UpdateRadar(RadarRange, Type->field_0xCCE, 0);
}
```

#### 4p. Laser Fence Frame Reset

```c
if (Type->field_0x16BE != 0 && g_MapEditorMode == 0) {
    this->LaserFenceFrame = 0;
}
```

---

## 5. BuildingClass::SpawnSurvivors (0x00442D90)

Called from within `BuildingClass::ReceiveDamage` case 4 (death), after OnDestroyed.

### Parameters

Uses building foundation cells from `vtable+0x108` (GetFoundationCells).

### Occupant Ejection (Garrisoned Infantry)

If building is garrisonable (`Type+0x16AE` or `Type+0x16AF` set — `CanBeOccupied`
or `CanOccupyFire`) and has occupants (`field_0x114 > 0`):

```c
for each occupant:
    if occupant is infantry:
        place at cell center (infantry sub-position)
    else:
        place at cell center directly
    
    if building was NOT destroyed by IC (field_0x6E0 == 0):
        if unit->Unlimbo(coords, facing) succeeds:
            mark as on map
            set scatter mission
            if enemy attacker: set retaliation target
        else:
            destroy the occupant
    else:
        // IC-destroyed building: just eject without unlimbo attempt
        unit->SetOwner(NULL)
        unit->Destroy()
```

### Crew/Survivor Spawning

For each foundation cell (iterated via foundation cell list ending at 0x7FFF sentinel):

```c
// Skip if owner is defeated (Owner->field_0x1F6)
if (!Owner->IsDefeated && survivorCount > 0) {
    int roll = Random(0, spawnChance);  // spawnChance = 2 normally, +6 if field_0x6E3
    if (roll == 1) {
        InfantryTypeClass* crewType = vtable+0x30C();  // GetCrewType
        if (crewType != NULL) {
            InfantryClass* survivor = new InfantryClass(crewType, Owner);
            
            // C4 veteran flag propagation
            if (this->field_0x6E9 && survivor->Type->field_0xC9E) {
                survivor->field_0x6D9 = 1;  // Mark as C4-carrying
            }
            
            // Place in cell
            coords = cellCenter + infantry subposition
            if (survivor->Unlimbo(coords, 0) succeeds) {
                survivorCount--;
                survivor->Health = Random(5, survivor->Type->Strength);
                survivor->SetMission(MISSION_SCATTER);
                
                // Set behavior based on attacker
                if (attacker == NULL || IsAlly(attacker)):
                    if IsPlayerControl(): Mission_Guard
                    else: Mission_Hunt(0xF)
                else:
                    survivor->SetMission(1);  // MISSION_ATTACK
                    survivor->Retaliate(attacker);
            } else {
                survivor->Destroy();
            }
        }
    }
}

// Debris spawning per cell (50/50 chance)
if (cell is passable) {
    if (Random(0, 99) < 50):
        SpawnDebris(...)      // Small debris
    else:
        Debris_Smoke(...)     // Smoke debris
}
```

**Survivor count** is determined by `vtable+0x2D0()` — returns the number of crew
to spawn. Modified by:
- `field_0x540 != 0`: reduces count by 1 (IC-related)
- `field_0x6E3 != 0`: adds 6 to spawn chance denominator (harder to spawn)

---

## 6. Damage State Transitions (ConditionYellow / ConditionRed)

### Thresholds (from ObjectClass::ReceiveDamage)

| State | Threshold | Rules Offset | Typical Value |
|-------|-----------|-------------|---------------|
| ConditionYellow | `Health < Strength * ConditionYellow` | Rules+0x1700 | 0.5 (50%) |
| ConditionRed | `Health < Strength * ConditionRed` | Rules+0x1708 | 0.25 (25%) |

### Building-Specific Transition Handling (in BuildingClass::ReceiveDamage)

After TechnoClass::ReceiveDamage returns, if damage was dealt (`result != 0`):

```c
double healthRatio = GetHealthRatio();
double yellowThreshold = Rules->ConditionYellow;  // +0x1700

if (this->IsDamaged != (healthRatio <= yellowThreshold)) {
    this->IsDamaged = (healthRatio <= yellowThreshold);
    
    // Update all 0x15 (21) building animation slots
    for (int slot = 0; slot < 0x594; slot += 0x44) {  // 0x594/0x44 = ~20.4 → 20 slots
        if (AnimSlot[slot] != NULL) {
            char* animName;
            if (healthRatio > yellowThreshold)
                animName = Type + 0xF4C + slot;  // Undamaged anim names
            else
                animName = Type + 0xF5C + slot;  // Damaged anim names
            
            if (animName != NULL && *animName != '\0') {
                BuildingClass::CreateAnimForSlot(this, slot);
            }
        }
    }
}
```

The building animation swap happens at the `ConditionYellow` threshold. The two
arrays at `Type+0xF4C` (undamaged) and `Type+0xF5C` (damaged) contain the SHP
animation names to use for each building anim slot.

### SetDamagedState Helper (0x00451EE0)

```c
void BuildingClass::SetDamagedState(bool isDamaged) {
    if (this->IsDamaged != isDamaged) {
        this->IsDamaged = isDamaged;
        // Same anim slot iteration as above
        for each of ~20 anim slots:
            swap to damaged/undamaged anim variant
    }
}
```

### Frame Change Detection

After anim state update, if the current frame differs from the pre-damage frame:
```c
int newFrame = GetCurrentFrame();
if (oldFrame != newFrame) {
    this->field_0x80 = 1;   // Mark visual state dirty
    // Repeat the IsDamaged check + anim swap (redundant but safe)
}
```

---

## 7. Building Auto-Repair

### Player-Initiated Repair

Buildings do NOT auto-repair by default. Repair is triggered by the player clicking
the repair wrench button, which sets the building mission to `MISSION_REPAIR` (0x13 = 19).

This is handled in `BuildingClass::MissionRepairAndProduce` (0x0044B780), which is
the combined repair + production mission handler.

### Repair Flow (MissionRepairAndProduce, IsRepairDepot = Type+0x16A9)

For buildings with `Type+0x16A9` (IsRepairDepot / Repairable):

**State 0 (init):** Transition to state 2, init repair timer, set repair animations.

**State 2 (active repair):**
```c
// Timer-based repair step
if (timer expired && field_0x634 != 0) {
    field_0x624 = 1;  // Repair tick active
    field_0x620 += field_0x638;  // Accumulate repair progress
    // Reset timer
}

// Check completion
double repairCost = Rules->RepairRate * multiplier;  // Rules+0x16E8
if (field_0x620 >= repairCost) {
    // Attempt to repair: check if target unit can dock
    int dockResult = vtable+0x274(0x13);  // CanAcceptDocking?
    if (dockResult == 1) {
        // Check unit health vs RepairPercent threshold
        double health = unit->GetHealthRatio();
        double threshold = Rules->RepairPercent;  // Rules+0x16F8
        
        if (health >= threshold && !Type->field_0xD24) {
            // Repair complete
            ...
        } else {
            // Continue repairing
            // EVA: "Repairing"
            ...
        }
    }
}
```

### Self-Healing (Veterancy-Based)

The `SelfHealing` TechnoTypeClass flag (`Type+0x1551` in the decompilation context,
parsed at TechnoTypeClass ReadINI) enables passive self-repair. This is NOT the same
as player-initiated repair.

Self-healing is checked during the tick update cycle, not in ReceiveDamage. Elite
units with `SelfHealing=yes` can slowly regenerate HP without player intervention.

### Engineer Repair

`BuildingClass::EngineerRepair` (0x00701410) handles the special case of an engineer
entering a building to fully repair it to 100% health.

---

## 8. BuildingClass::CreateDamageFireAnims (0x0043C0D0)

Called from `BuildingClass::Update @ 0x0043FB20` when the cached damage-fire
state at `this+0x5E8` changes to damaged. The threshold is selected by
`BuildingType+0x157B`: zero uses `ConditionYellow`, nonzero uses `ConditionRed`.
Creates fire/smoke animations on the building's damage fire points.

### Logic

```c
int fireAnimCount = Rules->DamageFireTypes;  // Rules+0x2B0
if (fireAnimCount == 0) return;

int animIndex = Random(0, fireAnimCount - 1);

// Iterate damage fire points: offsets 0x15D8..0x1618 in BuildingTypeClass
// (8 byte per point: X_offset, Y_offset isometric pixel coords)
for (int offset = 0x15D8; offset < 0x1618; offset += 8) {
    // Check if point is valid (not NULL sentinel)
    if (Type->DamageFirePoint[offset] == NULL_COORD) return;
    
    // Check if anim slot already occupied
    if (this->DamageFireAnims[slot] != 0) return;
    
    // Convert isometric pixel offset to world coords
    CoordStruct worldPos = IsometricPixelToWorld(Type->DamageFirePoint[offset]);
    worldPos += this->GetRenderCoords();
    
    // Create fire animation
    AnimType* fireType = Rules->DamageFireTypes[animIndex];  // Rules+0x2A4 array
    AnimClass* anim = new AnimClass(fireType, worldPos, 0, 1, 0x600, 0, 0);
    
    if (anim != NULL) {
        this->DamageFireAnims[slot] = anim;
        
        // Calculate Z-priority based on foundation size
        int zPri = ((Type->DamageFireYOffset + (height + width) * -0xF) * 3 / 2) - 10;
        if (zPri > 0) zPri = 0;  // Clamp to non-positive
        anim->field_0x100 = zPri;
        
        // Randomize start frame
        int maxFrames = anim->Type->field_0x2C0;  // FrameCount
        if (maxFrames > 0)
            anim->CurrentFrame = Random(0, maxFrames - 1);
        
        animIndex = (animIndex + 1) % fireAnimCount;
    }
}
```

**Damage fire point array:** 8 points at BuildingTypeClass offsets 0x15D8-0x1618,
each 8 bytes (two shorts: X_iso, Y_iso). These are the isometric pixel positions
where damage fire animations appear on the building sprite.

**Rules->DamageFireTypes:** Array at Rules+0x2A4, count at Rules+0x2B0. These are
AnimTypeClass pointers for the fire animations.

---

## 9. BuildingClass::OnWallDestroyed (0x00453240)

Called when a wall segment is destroyed. Handles wall chain destruction logic.

### Logic

```c
CellClass* cell = MapClass::GetCellAt(coords);
BuildingClass* wallBuilding = LookUpBuildingInCell(cell);

if (wallBuilding && wallBuilding->Type->field_0x16BF != 0) {
    // This is a wall chain — trace along the wall direction
    uint direction = (CurrentFrame >> 12 + 1) >> 1 & 3;
    
    // Walk along connected walls in direction
    CellStruct next = coords + DirectionOffset[direction];
    while (next != end) {
        BuildingClass* nextWall = LookUpBuildingInCell(next);
        if (nextWall == NULL) break;
        
        if (nextWall->Type->field_0x16BE != 0) {
            // Is laser fence — adjust wall connections
            BuildingClass::AdjustWallConnections(direction | 4, 0);
            return;
        }
        if (nextWall->Type->field_0x16BF == 0) break;  // Not a wall
        
        // Continue tracing
        next += DirectionOffset[direction];
    }
    
    // No fence post found — destroy the chain end
    if (nextWall == NULL) {
        wallBuilding->Destroy();  // vtable+0xF8
    }
}
```

When a wall segment is destroyed, the engine traces along the wall direction to find
connected walls and laser fence posts, adjusting connections accordingly.

---

## 10. Key BuildingTypeClass Flags Referenced

| Offset | Likely INI Key | Used In | Description |
|--------|---------------|---------|-------------|
| +0x233 | Insignificant | ReceiveDamage, ObjectClass | Immune to damage |
| +0x16AE | CanBeOccupied | SpawnSurvivors | Building is garrisonable |
| +0x16AF | (related garrison) | SpawnSurvivors | Garrison fire capability |
| +0x16B6 | (Gate/special) | ReceiveDamage | Combined with Insignificant for immunity |
| +0x16BE | IsLaserFence/Wall | OnDestroyed, OnWallDestroyed | Wall connection management |
| +0x16BF | Wall | ReceiveDamage | Wall damage immunity |
| +0x16C1 | Weeder/Grinder | MissionRepairAndProduce | Factory-type production |
| +0x16C2 | (related factory) | MissionRepairAndProduce | Another factory variant |
| +0x16C8 | HasSensorArray | OnDestroyed | Sensor removal on death |
| +0x16CB | Factory | SpawnSurvivors, OnDestroyed | Factory building flag |
| +0x16CC | (secondary factory) | OnDestroyed | Factory count tracking |
| +0xD31 | DetectDisguise | OnDestroyed | Disguise detection removal |
| +0xEB8 | RadarRange | OnDestroyed | Radar range (int) |
| +0xF4C | (anim names undamaged) | ReceiveDamage | Anim name array (undamaged) |
| +0xF5C | (anim names damaged) | ReceiveDamage | Anim name array (damaged) |
| +0x157B | label unresolved | ReceiveDamage case 4; Update damage-fire threshold selector | Sell on destruction when set; also selects ConditionRed instead of ConditionYellow for damage fires |
| +0x1564 | PowerOutput (int) | OnDestroyed | Power grid adjustment |
| +0x1568 | PowerDrain (int) | OnDestroyed | Power grid adjustment |
| +0x1780 | (storage amount) | OnDestroyed | Tiberium storage capacity |

---

## 11. Key BuildingClass Instance Fields Referenced

| Offset | Type | Field | Used In |
|--------|------|-------|---------|
| +0x6C | int | Health | All damage functions |
| +0x80 | byte | VisualDirty | ReceiveDamage (frame change) |
| +0x81 | byte | (limbo flag) | OnDestroyed |
| +0x8C | byte | OwnerIndex? | SpawnSurvivors |
| +0x90 | byte | IsAlive | ReceiveDamage (alive check) |
| +0x30C | ptr | DockedUnit | ReceiveDamage case 2/4 |
| +0x520 | ptr | Type (BuildingTypeClass*) | Everywhere |
| +0x528 | int | IC_StartFrame (building) | ReceiveDamage, IronCurtain |
| +0x52C | int | IC_TimerPad | IronCurtain |
| +0x530 | int | IC_Duration (building) | ReceiveDamage |
| +0x53C | int | LastAttackerTypeID | ReceiveDamage |
| +0x540 | int | IC_Counter | IronCurtain, SpawnSurvivors |
| +0x5C8 | ptr[8] | AnimSlots | OnDestroyed |
| +0x600 | ptr | SpecialAnim | OnDestroyed |
| +0x620 | int | RepairProgress | MissionRepairAndProduce |
| +0x628 | int | RepairTimerStart | MissionRepairAndProduce |
| +0x62C | int | RepairTimerPad | MissionRepairAndProduce |
| +0x630 | int | RepairTimerDuration | MissionRepairAndProduce |
| +0x634 | int | RepairActive | MissionRepairAndProduce |
| +0x638 | int | RepairStep | MissionRepairAndProduce |
| +0x6DD | byte | ProductionDone | MissionRepairAndProduce |
| +0x6DF | byte | DelayKillActive | IronCurtain, ReceiveDamage |
| +0x6E0 | byte | DestroyedByIC | SpawnSurvivors |
| +0x6E3 | byte | (survivor modifier) | SpawnSurvivors |
| +0x6E9 | byte | C4VeteranFlag | SpawnSurvivors |
| +IsDamaged | bool | ConditionYellow state | ReceiveDamage, SetDamagedState |
| +LightSource | ptr | Point light | ReceiveDamage case 4 |

---

## 12. Retaliation Logic (Building-Specific)

Buildings follow the same retaliation path as other TechnoClass objects
(documented in RECEIVE_DAMAGE_GHIDRA_REPORT.md Phase 11), with these
building-specific additions in BuildingClass::ReceiveDamage:

**Attack notification (if attacker exists and damage dealt):**
```c
if (Type->field_0x232 == 0 && !this->IsCloaked()) {
    HouseClass::NotifyUnderAttack(this);
}
```

**Auto-return fire check:**
```c
// Only retaliates if:
// - Not in mission 0x13 (REPAIR)
// - Attacker is enemy (not allied)
// - Primary weapon exists and is not anti-air-only (weapon->Type+0x2A4 == 0)
// - No existing target OR target is already the attacker
// - For human players: only if Rules+0x17EC is 0 (not passive mode)
//   OR if auto-base-defense is enabled

if (conditions met) {
    CDTimerClass timer;
    if (timer expired && vtable+0x350 returns true) {
        // Set retaliation delay with random component
        RateTimer::Set(Random::Next() << 8);
    }
} else {
    vtable+0x3C8(attacker);  // Assign target for AI
}
```

---

## 13. Complete Damage Pipeline Summary

```
BuildingClass::ReceiveDamage (0x00442230)
│
├─ [1] Self-damage guard
├─ [2] Record pre-damage state (health, frame)
├─ [3] Threat assessment (AI defense response)
├─ [4] Wall immunity check → return 0
├─ [5] Insignificant+Gate check → return 0
├─ [6] Dead-already check → skip to cleanup
│
├─ TechnoClass::ReceiveDamage (0x00701900)
│  ├─ Type-based damage multiplier
│  ├─ Veterancy modifier
│  ├─ TypeImmune check → return 0
│  ├─ IronCurtain check → spark + return 0
│  ├─ WarpingOut check → return 0
│  ├─ Ammo absorption
│  ├─ Bunker/ForceShield logic
│  ├─ Warhead immunity checks (Radiation/Psychic/Poison/Allied)
│  ├─ Psychedelic/MindControl → return 1
│  │
│  ├─ ObjectClass::ReceiveDamage (0x005f5390)
│  │  ├─ Insignificant check → return 0
│  │  ├─ Armor/Verses damage calculation
│  │  ├─ Health -= Damage
│  │  ├─ State transitions (Yellow=2, Red=3, Dead=4)
│  │  ├─ Trigger events (0x26-0x2C)
│  │  └─ If dead: RegisterDestruction + MarkForDeath
│  │
│  ├─ Score tracking
│  ├─ CausesDelayKill building delay-kill
│  ├─ Death cleanup (MC, passengers, debris, EVA)
│  ├─ Damage particles (smoke)
│  └─ Retaliation / scatter
│
├─ [7] Building death processing (result == 4):
│  ├─ Undock units
│  ├─ Free mind-controlled
│  ├─ Chrono warp emergency undeploy
│  ├─ Eject/damage docked units
│  ├─ Type+0x157B set (label unresolved) -> SellBuilding
│  ├─ Destroy light source
│  ├─ BuildingClass::DestructionEffects via vtable+0x4EC (0x004415F0) [corrected 2026-05-28]
│  │  │  (0x00445880 = BuildingClass::Limbo, called separately on map removal)
│  │  ├─ Destroy 8 anim slots
│  │  ├─ Factory count decrement
│  │  ├─ Storage capacity decrement
│  │  ├─ Power grid recalculation
│  │  ├─ Wall connection update
│  │  ├─ Sensor/disguise detect removal
│  │  ├─ Special building handling (reactor, bridge)
│  │  ├─ Cell occupancy cleanup
│  │  ├─ Screen invalidation
│  │  ├─ House recount + base recenter
│  │  ├─ UI update (sidebar)
│  │  ├─ Register building loss
│  │  ├─ Super weapon recalc (if upgraded)
│  │  └─ Radar update
│  └─ IC timer carryover → re-occupy map
│
├─ [8] BuildingClass::SpawnSurvivors (0x00442D90)
│  ├─ Eject garrisoned occupants
│  ├─ Spawn crew infantry (random per cell)
│  └─ Spawn debris per foundation cell
│
├─ [9] ConditionYellow/Red anim swap
├─ [10] Attack notification + retaliation
└─ [11] Frame change → mark visual dirty

Return DamageState
```

---

## 14. Function Address Summary

| Address | Function | Purpose |
|---------|----------|---------|
| 0x00442230 | BuildingClass::ReceiveDamage | Building damage entry point |
| 0x00445880 | BuildingClass::Limbo | Map-removal cleanup (formerly mislabeled OnDestroyed; corrected 2026-05-28 via get_function_by_address) |
| 0x004415F0 | BuildingClass::DestructionEffects | vtable+0x4EC on-death effects: anim slots, sound, SpawnSurvivors, debris (corrected 2026-05-28 via read_memory(0x007E43A8)) |
| 0x00442D90 | BuildingClass::SpawnSurvivors | Crew/occupant ejection |
| 0x00451EE0 | BuildingClass::SetDamagedState | Anim swap on condition change |
| 0x0043C0D0 | BuildingClass::CreateDamageFireAnims | Fire anims on damaged buildings |
| 0x00453240 | BuildingClass::OnWallDestroyed | Wall chain destruction |
| 0x00457C90 | BuildingClass::IronCurtain | Building IC override (resets timer) |
| 0x004575B0 | BuildingClass::EjectOccupants | Remove upgrades + sell |
| 0x0044B780 | BuildingClass::MissionRepairAndProduce | Repair + production mission |
| 0x00441F60 | BuildingClass::Place_OccupyMap | Map cell occupation |
| 0x00701900 | TechnoClass::ReceiveDamage | Core damage pipeline |
| 0x005F5390 | ObjectClass::ReceiveDamage | HP deduction + state transitions |
| 0x0041BF40 | TechnoClass::IsIronCurtainActive | IC timer check |
| 0x0070E2B0 | TechnoClass::IronCurtain | Apply IC effect |
| 0x0070C5B0 | TechnoClass::IsWarpingOut | Chrono warp check |
| 0x00489180 | Armor/Verses calculation | Damage * Verses multiplier |
| 0x0048A620 | Play spark/flash anim | IC/ForceShield visual feedback |
| 0x004FF980 | HouseClass::Recount | Update building counts |
| 0x00701410 | BuildingClass::EngineerRepair | Full repair by engineer |
