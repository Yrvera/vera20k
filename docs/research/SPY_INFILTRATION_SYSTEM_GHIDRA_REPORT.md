# BuildingClass::OnSpyInfiltrate — Complete Ghidra Analysis

Function: `BuildingClass__OnSpyInfiltrate` at `0x004571E0`
Signature: `void __thiscall BuildingClass__OnSpyInfiltrate(BuildingClass* this)`
Size: `0x004571E0 - 0x004575A2` (962 bytes)

The function receives the spy's owner HouseClass* as a hidden stack parameter
(`in_stack_00000004` / EDI in asm). The building (`this` / EBP) is the infiltrated
building. The building's owner is at `this->Owner` (`+0x21C`).

All spy effects are **active in YR** — none are TS-gated.

---

## Early Exit

```
if (building->Owner == spy_owner)
    return;  // Can't spy on your own buildings
```

## Radar Event

Before any spy effect, if either the building owner or the spy owner is a human player,
a radar event (type 9 = spy) is created at the building's location. The `radarEventCreated`
flag tracks whether this succeeded.

---

## Decision Tree

The function reads `BuildingTypeClass*` from `building->Type` (offset `+0x520` on
BuildingClass). All field offsets below are direct byte offsets on BuildingTypeClass
(param_1 is `int` in the ReadINI function).

```
if (Type->Radar)                           → BRANCH 1: Radar Spy
else if (Type->Power > 0)                  → BRANCH 2: Power Plant Spy
else if (Type in Rules->BuildTech[])       → BRANCH 3: Tech Center Spy
else if (Type->SuperWeapon != -1)          → BRANCH 4: SuperWeapon Spy
else if (Type->Storage > 0)                → BRANCH 5: Refinery Spy (Money Steal)
else if (Type->Factory == UnitType)        → BRANCH 6: War Factory Spy
else if (Type->Factory == InfantryType)    → BRANCH 7: Barracks Spy
else                                       → No effect (falls through)
```

After the spy effect, the building is assigned mission Guard (vtable+0x124, arg 2).

---

## BRANCH 1: Radar Spy (Shroud Reset)

**Condition:** `BuildingTypeClass+0x16A4` != 0 (`Radar=yes` in INI)

**Effect:** Resets the victim's map shroud — all previously explored areas become
unexplored again. The victim loses map knowledge.

**Implementation:**
1. Calls `FUN_0050BD10(building->Owner)` which checks `HouseClass+0x577A`
   (LowPowerState). If the victim is NOT in low power, calls
   `MapClass::RestoreShroud(owner)`.
2. `RestoreShroud` (at `0x00577AB0`):
   - Sets `HouseArray[house_index]+0x241 = 0` (shroud visibility flag)
   - Calls `ParanoidRevealAll` then iterates all cells, clearing visibility bits
   - Calls `ParanoidUnrevealAll` to re-hide everything
   - Sets `g_PlayerPtr+0x240 = 0`
   - Refreshes radar display
3. If victim is in low power state, the spy has no effect (radar already offline).

**EVA voices:**
- Victim (if human): `EVA_RadarSabotaged` (at 0x8191E4)
- Attacker (if human + radar event): `EVA_BuildingInfRadarSabotaged` (at 0x8191C4)

**Field offsets:**
| Offset | On | Field | INI Key |
|--------|-----|-------|---------|
| +0x16A4 | BuildingTypeClass | Radar | `Radar=` (bool) |
| +0x577A | HouseClass | LowPowerState | (runtime) |

---

## BRANCH 2: Power Plant Spy (Power Blackout)

**Condition:** `BuildingTypeClass+0xEE0` > 0 (`Power=` value is positive)

**Effect:** Blacks out the victim's power for a duration, disabling all powered
buildings (radar, super weapons, etc.).

**Implementation:**
Calls `HouseClass::SpyPowerSabotage(building->Owner, duration)` at `0x0050BC90`.

```c
void HouseClass::SpyPowerSabotage(HouseClass* this, int duration) {
    this->PowerBlackedOut = 1;           // +0x5778
    this->BlackoutStartFrame = CurrentFrame;  // +0x2A4
    // +0x2A8 = (stack artifact, timer internal)
    this->BlackoutDuration = duration;    // +0x2AC
}
```

Duration comes from `RulesClass+0xD64` = `SpyPowerBlackout` INI key.
Default: **1000 frames** (~66.7 seconds at 15fps).

**EVA voices:**
- Victim (if human + radar event): `EVA_PowerSabotaged` (at 0x8191B0)
- Attacker (if human + radar event): `EVA_BuildingInfiltrated` (at 0x819198) +
  `EVA_EnemyBasePoweredDown` (at 0x81917C)

**Field offsets:**
| Offset | On | Field | INI Key |
|--------|-----|-------|---------|
| +0xEE0 | BuildingTypeClass | Power | `Power=` (int) |
| +0x5778 | HouseClass | PowerBlackedOut | (runtime) |
| +0x2A4 | HouseClass | BlackoutStartFrame | (runtime, timer) |
| +0x2AC | HouseClass | BlackoutDuration | (runtime, timer) |
| +0xD64 | RulesClass | SpyPowerBlackout | `SpyPowerBlackout=` (int, frames) |

---

## BRANCH 3: Tech Center Spy (Stolen Tech)

**Condition:** `BuildingTypeClass*` matches an entry in `RulesClass->BuildTech[]` array,
AND the building is NOT a power plant (Power <= 0).

The BuildTech list is at `RulesClass+0x920` (data pointer) / `+0x92C` (count).
Default: `BuildTech=NATECH,GATECH,YATECH` (YR).

**Effect:** Sets a "stolen tech" flag on the spy's owner house based on the
tech building's `AIBasePlanningSide`:

| AIBasePlanningSide | Flag Set | HouseClass Offset | Meaning |
|-------------------|----------|-------------------|---------|
| 0 (Allied) | StolenAlliedTech | +0x2BE | Units requiring `RequiresStolenAlliedTech=yes` become available |
| 1 (Soviet) | StolenSovietTech | +0x2BD | Units requiring `RequiresStolenSovietTech=yes` become available |
| else (Third/Yuri) | StolenThirdTech | +0x2BC | Units requiring `RequiresStolenThirdTech=yes` become available |

Also sets `HouseClass+0x1FC` (ProductionChanged) = 1, which triggers prerequisite
recalculation so newly available units appear in the sidebar.

**AIBasePlanningSide** is at `TechnoTypeClass+0x6D0` (`AIBasePlanningSide=` INI key).
Default values: GATECH=0 (Allied), NATECH=1 (Soviet), YATECH=2 (Yuri).

**EVA voices:**
- Victim (if human + radar event): `EVA_TechnologyStolen` (at 0x819138)
- Attacker (if human + radar event): `EVA_BuildingInfiltrated` (at 0x819198) +
  `EVA_NewTechnologyAcquired` (at 0x81911C)

**Field offsets:**
| Offset | On | Field | INI Key |
|--------|-----|-------|---------|
| +0x6D0 | TechnoTypeClass | AIBasePlanningSide | `AIBasePlanningSide=` (int) |
| +0x920 | RulesClass | BuildTech.Data | `BuildTech=` (list) |
| +0x92C | RulesClass | BuildTech.Count | (runtime) |
| +0x2BC | HouseClass | StolenThirdTech | (runtime flag) |
| +0x2BD | HouseClass | StolenSovietTech | (runtime flag) |
| +0x2BE | HouseClass | StolenAlliedTech | (runtime flag) |
| +0x1FC | HouseClass | ProductionChanged | (runtime flag) |

---

## BRANCH 4: SuperWeapon Spy (Reset Charge Timer)

**Condition:** `BuildingTypeClass+0x16F0` != -1 (`SuperWeapon=` index is set),
AND the building is NOT a power plant, AND it is NOT in the BuildTech list.

**Effect:** Resets the superweapon's recharge timer, forcing it to start charging
from scratch.

**Implementation:**
1. Gets the superweapon index from `BuildingTypeClass+0x16F0`
2. Looks up the SuperClass instance: `building->Owner->SuperWeaponsArray[index]`
   (`HouseClass+0x258` is the SuperClass* array)
3. Calls `OnSpyWeaponInfiltrate(superClass)` at `0x006CE0B0`

**OnSpyWeaponInfiltrate** (SuperClass at `0x006CE0B0`):
```c
void OnSpyWeaponInfiltrate(SuperClass* this) {
    // Remove any active charge animation
    if (this->ChargeAnim != NULL) {              // +0x68
        this->ChargeAnim->IsActive = 0;          // anim+0x195
        this->ChargeAnim = NULL;                  // +0x68 = 0
        // Remove from global SuperClass tracking array
        int idx = SuperArray->FindIndex(this);
        if (idx != -1) SuperArray->RemoveIndex(idx);
    }
    // Clear the "ready/charged" flag
    if (this->IsCharged) {                        // +0x6C
        int idx = SuperArray->FindIndex(this);
        if (idx != -1) SuperArray->RemoveIndex(idx);
        this->IsCharged = 0;                      // +0x6C = 0
    }
    
    // Reset recharge timer
    int rechargeTime = this->CustomRechargeTime;  // +0x24
    this->IsOneShotFired = 0;                     // +0x6F = 0
    this->OldReadyFrame = this->RechargeStartFrame;  // save old +0x30
    this->CameoChargeFrame = -1;                  // +0x78 = -1
    
    if (rechargeTime == -1)
        rechargeTime = this->Type->RechargeTime;  // +0x28 -> +0xB0
    
    this->RechargeStartFrame = CurrentFrame;       // +0x30
    this->RechargeDuration = rechargeTime;         // +0x38
    
    // Handle edge case: if timer was previously stopped (-1),
    // adjust remaining duration
    if (oldStartFrame == -1 && newStartFrame != -1) {
        elapsed = CurrentFrame - newStartFrame;
        if (elapsed < rechargeDuration) {
            this->RechargeStartFrame = -1;
            this->RechargeDuration -= elapsed;
        } else {
            this->RechargeDuration = 0;
            this->RechargeStartFrame = -1;
        }
    }
}
```

**EVA voices:**
- Victim: (none specific — no power/radar message)
- Attacker (if human + radar event): `EVA_BuildingInfiltrated` (at 0x819198)

**Field offsets:**
| Offset | On | Field | INI Key |
|--------|-----|-------|---------|
| +0x16F0 | BuildingTypeClass | SuperWeapon | `SuperWeapon=` (index, -1=none) |
| +0x258 | HouseClass | SuperWeaponsArray | (runtime, SuperClass*[]) |
| +0x24 | SuperClass | CustomRechargeTime | (runtime) |
| +0x28 | SuperClass | Type (SuperWeaponTypeClass*) | (runtime) |
| +0x30 | SuperClass | RechargeStartFrame | (runtime, timer) |
| +0x38 | SuperClass | RechargeDuration | (runtime, timer) |
| +0x68 | SuperClass | ChargeAnim | (runtime, AnimClass*) |
| +0x6C | SuperClass | IsCharged | (runtime, bool) |
| +0x6F | SuperClass | IsOneShotFired | (runtime, bool) |
| +0x78 | SuperClass | CameoChargeFrame | (runtime) |
| +0xB0 | SuperWeaponTypeClass | RechargeTime | `RechargeTime=` (int) |

---

## BRANCH 5: Refinery Spy (Money Steal)

**Condition:** `TechnoTypeClass+0x800` > 0 (`Storage=` value is positive),
AND building has no SuperWeapon, AND it's not in BuildTech list, AND Power <= 0.

**Effect:** Steals a percentage of the victim's total credits and transfers them
to the spy's owner.

**Formula:**
```
victim_balance = GetTotalBalance(building->Owner)  // virtual call via +0x24
stolen_amount = (int)(victim_balance * SpyMoneyStealPercent)
HouseClass::Spend_Money(building->Owner, stolen_amount)  // victim loses
HouseClass::Add_Credits(spy_owner, stolen_amount)         // attacker gains
```

`SpyMoneyStealPercent` is at `RulesClass+0xD68` (float).
Default: **0.5** (50% of victim's current balance).

`GetTotalBalance` is called via `(*(vtable*)(*(Owner+0x24)))->method_0x18(Owner+0x24)`.
This returns the victim's complete available funds (cash + ore in storage).

**Spend_Money** (at `0x004F9790`): Deducts from `+0x30C` (AvailableCredits) first,
then if insufficient, pulls from ore storage in refineries.

**Add_Credits** (at `0x004F9950`): Simply adds to `+0x30C` (AvailableCredits).

**EVA voices:**
- Victim (if human + radar event): `EVA_CashStolen` (at 0x81916C)
- Attacker (if human + radar event): `EVA_BuildingInfCashStolen` (at 0x819150)

**Field offsets:**
| Offset | On | Field | INI Key |
|--------|-----|-------|---------|
| +0x800 | TechnoTypeClass | Storage | `Storage=` (int) |
| +0xD68 | RulesClass | SpyMoneyStealPercent | `SpyMoneyStealPercent=` (float) |
| +0x30C | HouseClass | AvailableCredits | (runtime) |

---

## BRANCH 6: War Factory Spy (Unlock Units)

**Condition:** `BuildingTypeClass+0xEB8` == 0x28 (`Factory=UnitType` in INI),
AND building has no Storage, no SuperWeapon, not in BuildTech, Power <= 0.

RTTI value 0x28 = `UnitType` in the RTTIType enum table at `0x00816EE0`.

**Effect:** Sets a "spied war factory" flag on the spy's owner house, AND triggers
sidebar recalculation.

```c
spy_owner->SpiedWarFactory = 1;       // HouseClass+0x2C0
spy_owner->ProductionChanged = 1;     // HouseClass+0x1FC
if (spy_owner->IsHumanPlayer())
    g_SidebarNeedsRepaint = 1;        // global at 0x00884B8E
```

**EVA voices:**
- Victim (if human + radar event): `EVA_TechnologyStolen` (at 0x819138)
- Attacker (if human + radar event): `EVA_BuildingInfiltrated` (at 0x819198) +
  `EVA_NewTechnologyAcquired` (at 0x81911C)

**Field offsets:**
| Offset | On | Field | INI Key |
|--------|-----|-------|---------|
| +0xEB8 | BuildingTypeClass | Factory | `Factory=` (RTTIType enum) |
| +0x2C0 | HouseClass | SpiedWarFactory | (runtime flag) |
| +0x1FC | HouseClass | ProductionChanged | (runtime flag) |
| 0x884B8E | Global | SidebarNeedsRepaint | (runtime flag) |

---

## BRANCH 7: Barracks Spy (Unlock Infantry)

**Condition:** `BuildingTypeClass+0xEB8` == 0x10 (`Factory=InfantryType` in INI),
AND building has no Storage, no SuperWeapon, not in BuildTech, Power <= 0.

RTTI value 0x10 = `InfantryType` in the RTTIType enum table at `0x00816EE0`.

**Effect:** Sets a "spied barracks" flag on the spy's owner house, AND triggers
sidebar recalculation. Identical logic to War Factory spy but different flag.

```c
spy_owner->SpiedBarracks = 1;         // HouseClass+0x2BF
spy_owner->ProductionChanged = 1;     // HouseClass+0x1FC
if (spy_owner->IsHumanPlayer())
    g_SidebarNeedsRepaint = 1;        // global at 0x00884B8E
```

**EVA voices:**
- Victim (if human + radar event): `EVA_TechnologyStolen` (at 0x819138)
- Attacker (if human + radar event): `EVA_BuildingInfiltrated` (at 0x819198) +
  `EVA_NewTechnologyAcquired` (at 0x81911C)

**Field offsets:**
| Offset | On | Field | INI Key |
|--------|-----|-------|---------|
| +0xEB8 | BuildingTypeClass | Factory | `Factory=` (RTTIType enum) |
| +0x2BF | HouseClass | SpiedBarracks | (runtime flag) |
| +0x1FC | HouseClass | ProductionChanged | (runtime flag) |
| 0x884B8E | Global | SidebarNeedsRepaint | (runtime flag) |

---

## Branches NOT Taken: Factory=BuildingType, Factory=AircraftType

Buildings with `Factory=BuildingType` (RTTI 0x28? No — actually BuildingType in this
context is a different RTTI value) or `Factory=AircraftType` are not matched by either
the `0x28` or `0x10` checks. The only factory types that trigger spy effects are
`UnitType` (0x28) and `InfantryType` (0x10).

**Correction on RTTI values:** The `Factory=` field stores RTTIType enum values:
- `InfantryType` → 0x10 (16)
- `UnitType` → 0x28 (40)
- `BuildingType` and `AircraftType` → other values, not handled by spy code

So spying on a Construction Yard (`Factory=BuildingType`) or Airfield
(`Factory=AircraftType`) with no other qualifying traits produces **no spy effect**
(falls through to the end with just the Guard mission assignment).

---

## Complete Priority Order Summary

The checks are evaluated in this strict order. The FIRST match wins:

| Priority | Condition | Effect | Active in YR? |
|----------|-----------|--------|---------------|
| 1 | Same owner | No effect (early return) | Yes |
| 2 | `Radar=yes` | Reset victim's shroud | Yes |
| 3 | `Power > 0` | Power blackout for SpyPowerBlackout frames | Yes |
| 4 | In BuildTech list | Grant StolenXxxTech flag based on side | Yes |
| 5 | `SuperWeapon != -1` | Reset superweapon charge timer | Yes |
| 6 | `Storage > 0` | Steal SpyMoneyStealPercent of victim's money | Yes |
| 7 | `Factory=UnitType` | Set SpiedWarFactory flag + sidebar update | Yes |
| 8 | `Factory=InfantryType` | Set SpiedBarracks flag + sidebar update | Yes |
| 9 | None of the above | No gameplay effect | — |

---

## Helper Functions Decompiled

| Address | Name | Purpose |
|---------|------|---------|
| 0x004571E0 | BuildingClass::OnSpyInfiltrate | Main spy dispatch |
| 0x0050BC90 | HouseClass::SpyPowerSabotage | Set power blackout timer |
| 0x0050BD10 | FUN_0050BD10 | Wrapper: check LowPowerState then RestoreShroud |
| 0x00577AB0 | MapClass::RestoreShroud | Reset all map visibility for a house |
| 0x006CE0B0 | OnSpyWeaponInfiltrate | Reset superweapon charge timer |
| 0x004F9790 | HouseClass::Spend_Money | Deduct credits from house |
| 0x004F9950 | HouseClass::Add_Credits | Add credits to house |
| 0x0050B6F0 | HouseClass::IsHumanPlayer | Check if house is human-controlled |
| 0x0065FA70 | CreateRadarEvent | Create radar minimap ping |
| 0x00752700 | VoxClass::PlayEVA | Play EVA voice line |
| 0x0040DCE0 | FUN_0040DCE0 | RTTIType enum string-to-int lookup |
| 0x0045ADD0 | DynamicVectorClass::Remove_Index | Remove element from dynamic array |

---

## RulesClass Fields Used

| Offset | Type | INI Key | Default | Purpose |
|--------|------|---------|---------|---------|
| +0x920 | ptr | BuildTech= | NATECH,GATECH,YATECH | Tech buildings list (data ptr) |
| +0x92C | int | (count) | 3 | Tech buildings list count |
| +0xD64 | int | SpyPowerBlackout= | 1000 | Power blackout duration in frames |
| +0xD68 | float | SpyMoneyStealPercent= | 0.5 | Fraction of money stolen |

---

## HouseClass Flags Set by Spy

| Offset | Size | Name | Set By |
|--------|------|------|--------|
| +0x1FC | 1 | ProductionChanged | Tech center, barracks, war factory spy |
| +0x2BC | 1 | StolenThirdTech | Tech center spy (Side >= 2) |
| +0x2BD | 1 | StolenSovietTech | Tech center spy (Side == 1) |
| +0x2BE | 1 | StolenAlliedTech | Tech center spy (Side == 0) |
| +0x2BF | 1 | SpiedBarracks | Barracks spy |
| +0x2C0 | 1 | SpiedWarFactory | War factory spy |
| +0x5778 | 1 | PowerBlackedOut | Power plant spy |
| +0x2A4 | 4 | BlackoutStartFrame | Power plant spy (timer) |
| +0x2AC | 4 | BlackoutDuration | Power plant spy (timer) |

---

## Confidence Level

**HIGH** — All logic traced directly from `gamemd.exe` decompilation at `0x004571E0`.
Field offset mappings confirmed by cross-referencing INI ReadINI functions
(`BuildingTypeClass_ReadINI_Water` at `0x0045F560`, `TechnoTypeClass__ReadINI` at
`0x00712170`, `RulesClass__ReadGeneral`). RTTI enum values verified against the
lookup table at `0x00816EE0`. EVA voice strings verified by reading memory at each
address. All 7 spy effects are active in standard YR skirmish with no TS gating.
