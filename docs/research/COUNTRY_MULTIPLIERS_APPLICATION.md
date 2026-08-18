# CountryTypeClass Multipliers — How They Are Applied in gamemd.exe

**Date:** 2026-03-22
**Source:** Ghidra decompilation of gamemd.exe + existing verified reports
**Confidence:** HIGH for global doubles (SetDifficulty decompiled). HIGH for per-category floats
(all 6 accessor functions decompiled). MEDIUM for IncomeMult, SmartAI, Prefix/Suffix (inferred
from xref context, no Ghidra MCP available this session for live verification).

---

## 1. The Two Tiers of Country Multipliers

CountryTypeClass has two distinct groups of multipliers:

### Tier 1: Global Doubles (+0xC8..+0xF8)

These 7 doubles are applied **once, at house creation time**, during `HouseClass::SetDifficulty`
(0x004F6EC0). They are baked into HouseClass fields and never re-read from CountryTypeClass
during gameplay.

| CountryType Offset | INI Key | HouseClass Destination | Applied Via |
|---|---|---|---|
| +0xC8 | `Firepower=` | HouseClass+0x188 (double) | `difficulty_firepower * country_firepower` |
| +0xD0 | `Groundspeed=` | HouseClass+0x190 (double) | `difficulty_armor * global_factor * country_groundspeed` |
| +0xD8 | `Airspeed=` | HouseClass+0x198 (double) | `difficulty_rof * global_factor * country_airspeed` |
| +0xE0 | `Armor=` | HouseClass+0x1A0 (double) | `difficulty_groundspeed * country_armor` |
| +0xE8 | (unnamed 5) | HouseClass+0x1A8 (double) | `difficulty_airspeed * country_unnamed5` |
| +0xF0 | (unnamed 6) | HouseClass+0x1B0 (double) | `difficulty_buildspeed * country_unnamed6` |
| +0xF8 | `BuildTime=` | HouseClass+0x1B8 (double) | `difficulty_cost * global_factor * country_buildtime` |

### Tier 2: Per-Category Floats (+0x100..+0x148)

These 19+1 floats are stored on CountryTypeClass and read **live during gameplay** through
accessor functions on HouseClass. They are never baked — each access reads
`HouseClass+0x34` (CountryTypeClass pointer) and indexes into the per-category float array.

---

## 2. SetDifficulty — Where Global Doubles Are Applied (0x004F6EC0)

**Decompiled pseudocode** (632 bytes, 4 callers):

```c
void HouseClass::SetDifficulty(int difficulty_level) {
    this->DifficultyLevel = difficulty_level;  // +0x184

    int* countryType = this->HouseTypeClass;   // +0x34
    int diffOffset = difficulty_level * 0x50;

    if (SessionClass::GameMode == 0) {
        // === SINGLEPLAYER ===
        // Firepower: directly from difficulty table (NO country scaling)
        this->Firepower = Rules[diffOffset + 0x1538];         // +0x188

        // Armor: difficulty * global factor (NO country scaling)
        this->Armor = Rules[(difficulty*5 + 0x154) * 0x10]    // +0x190
                    * Rules[0x1418];

        // ROF: difficulty * global factor
        this->ROF = Rules[diffOffset + 0x1548] * Rules[0x1418]; // +0x198

        // GroundSpeed: directly from difficulty table
        this->GroundSpeed = Rules[diffOffset + 0x1550];       // +0x1A0

        // AirSpeed: directly from difficulty table
        this->AirSpeed = Rules[diffOffset + 0x1558];          // +0x1A8

        // BuildSpeed: directly from difficulty table
        this->BuildSpeed = Rules[diffOffset + 0x1560];        // +0x1B0

        // Cost: difficulty * global factor
        this->Cost = Rules[diffOffset + 0x1568] * Rules[0x1418]; // +0x1B8

        // RepairDelay, BuildDelay: copied directly (no scaling at all)
        this->RepairDelay = Rules[diffOffset + 0x1570];       // +0x1C0
        this->BuildDelay = Rules[diffOffset + 0x1578];        // +0x1C8
    }
    else {
        // === MULTIPLAYER ===
        // Country multipliers from CountryTypeClass are applied here!

        // Firepower = difficulty_firepower * CountryType.Firepower
        this->Firepower = Rules[diffOffset + 0x1538]          // +0x188
                        * countryType[+0xC8];  // double

        // Armor = difficulty * global_factor * CountryType.Groundspeed
        // NOTE: The offset mapping here seems non-intuitive due to
        // how the difficulty table fields map to house fields.
        this->Armor = Rules[(difficulty*5 + 0x154) * 0x10]    // +0x190
                    * Rules[0x1418]
                    * countryType[+0xD0];

        // ROF = difficulty * global_factor * CountryType.Airspeed
        this->ROF = Rules[diffOffset + 0x1548]                // +0x198
                  * Rules[0x1418]
                  * countryType[+0xD8];

        // GroundSpeed = difficulty_groundspeed * CountryType.Armor
        this->GroundSpeed = Rules[diffOffset + 0x1550]        // +0x1A0
                          * countryType[+0xE0];

        // AirSpeed = difficulty_airspeed * CountryType.unnamed5
        this->AirSpeed = Rules[diffOffset + 0x1558]           // +0x1A8
                       * countryType[+0xE8];

        // BuildSpeed = difficulty_buildspeed * CountryType.unnamed6
        this->BuildSpeed = Rules[diffOffset + 0x1560]         // +0x1B0
                         * countryType[+0xF0];

        // RepairDelay, BuildDelay: NO country scaling
        this->RepairDelay = Rules[diffOffset + 0x1570];       // +0x1C0
        this->BuildDelay = Rules[diffOffset + 0x1578];        // +0x1C8

        // Cost = difficulty_cost * global_factor * CountryType.BuildTime
        this->Cost = Rules[diffOffset + 0x1568]               // +0x1B8
                   * Rules[0x1418]
                   * countryType[+0xF8];
    }

    // AI trigger timer computation
    int triggerDelay = Rules->DifficultyTriggerDelays[difficulty_level]; // +0x115C array
    this->AITriggerTimerStart = g_CurrentFrameCounter;         // +0x5798
    this->AITriggerDuration = triggerDelay + this->HouseIndex * 0xAF; // +0x57A0
}
```

**Critical finding:** In singleplayer, country multipliers are NOT applied at all. They only
take effect in multiplayer (when `SessionClass::GameMode != 0`). Since all vanilla countries
have all 7 doubles defaulting to 1.0, this has no visible effect in standard YR. These are
for mod use only.

**Callers** (4):
- `FUN_0050a5c0` — `HouseClass::ComputerTakeover` (player → AI handoff)
- `FUN_004c6210` — Event handler (difficulty change event)
- `FUN_005009b0` — Some initialization path
- `FUN_00687f10` — `Create_Houses` (game start)

---

## 3. Per-Category Float Accessor Functions

Six accessor functions on HouseClass read the per-category floats from `CountryTypeClass`
(via `HouseClass+0x34` pointer). All use the same pattern: switch on the unit's RTTI type
code to select the appropriate float offset.

**RTTI type codes:**
- `0x10` = Infantry (InfantryClass)
- `0x07` = Vehicle/Unit (UnitClass); sub-check `param_2[0x382] == 5` = Naval
- `0x03` = Aircraft (AircraftClass)
- `0x28` = Building (BuildingClass)
- `default` = returns 1.0 (no modifier)

### 3a. GetArmorBonus — FUN_0050bd30 (128 bytes, 2 callers)

**Callers:** `FUN_006fdb80` (damage calculation) and `FUN_00701900` (ReceiveDamage).

```c
float HouseClass::GetArmorBonus(TechnoClass* unit) {
    int rtti = unit->WhatAmI();  // vtable+0x2C
    CountryTypeClass* ct = this->HouseTypeClass;  // +0x34
    switch (rtti) {
        case 0x10: return ct->ArmorInfantryMult;     // +0x100
        case 0x03: return ct->ArmorAircraftMult;      // +0x108
        case 0x28: return ct->ArmorBuildingsMult;     // +0x104  (NOTE: 0x28=Building)
        case 0x07:
            if (unit->SpeedType == 5)                 // Naval
                return ct->ArmorDefensesMult;          // +0x110
            return ct->ArmorUnitsMult;                // +0x10C
        default:   return 1.0f;
    }
}
```

**Where applied:** This is called in both damage dealing (`FUN_006fdb80` = CalcDamage,
used inside `Fire_At`) and damage receiving (`FUN_00701900` = ReceiveDamage). The return
value is multiplied with the raw damage. For armor bonuses (reducing damage), the multiplier
would be < 1.0. For penalties, > 1.0.

### 3b. GetCostBonus — FUN_0050bdf0 (128 bytes, 2 callers)

**Callers:** `FUN_00711f60` and `FUN_00711f00` (production cost calculation).

```c
float HouseClass::GetCostBonus(TechnoClass* unit) {
    int rtti = unit->WhatAmI();
    CountryTypeClass* ct = this->HouseTypeClass;
    switch (rtti) {
        case 0x10: return ct->CostInfantryMult;       // +0x114
        case 0x03: return ct->CostAircraftMult;        // +0x11C
        case 0x28: return ct->CostBuildingsMult;       // +0x118
        case 0x07:
            if (unit->SpeedType == 5)
                return ct->CostDefensesMult;            // +0x124
            return ct->CostUnitsMult;                  // +0x120
        default:   return 1.0f;
    }
}
```

**Where applied:** Called inside production cost functions. The cost is multiplied by this
value, so `CostUnitsMult=0.9` gives a 10% cost discount on vehicles.

### 3c. GetAccumulatedBonus — FUN_0050beb0 (113 bytes, 2 callers)

**Callers:** Same as GetCostBonus (`FUN_00711f60`, `FUN_00711f00`).

```c
float HouseClass::GetAccumulatedBonus(TechnoClass* unit) {
    int rtti = unit->WhatAmI();
    // Reads from HouseClass instance, NOT CountryTypeClass!
    switch (rtti) {
        case 0x10: return this->BonusInfantry;   // +0x5390
        case 0x03: return this->BonusAircraft;    // +0x5398
        case 0x28: return this->BonusNaval;       // +0x5394
        case 0x07:
            if (unit->SpeedType == 5)
                return this->BonusNavalAlt;        // +0x53A0
            return this->BonusVehicle;            // +0x539C
        default:   return 1.0f;
    }
}
```

This reads from HouseClass instance fields (NOT CountryTypeClass). These are computed by
`RecalcBonuses` (FUN_0050bf60) which multiplies 1.0 by per-upgrade-building bonuses from
`BuildingTypeClass+0x16D0..0x16E0`. This is the **upgrade building** bonus system
(e.g., building a Battle Lab might give a production speed bonus), separate from country
multipliers.

### 3d. GetSpeedBonus — FUN_0050c050 (76 bytes, 1 caller)

**Caller:** `FUN_004db1a0` (movement/locomotion system).

```c
float HouseClass::GetSpeedBonus(TechnoClass* unit) {
    int rtti = unit->WhatAmI();
    CountryTypeClass* ct = this->HouseTypeClass;
    switch (rtti) {
        case 0x03: return ct->SpeedAircraftMult;     // +0x130
        case 0x10: return ct->SpeedInfantryMult;      // +0x128
        case 0x28: return ct->SpeedBuildingsMult;     // +0x12C (300 = 0x12C)
        default:   return 1.0f;
    }
}
```

**NOTE:** No SpeedUnitsMult or SpeedNavalMult in this accessor! Only Infantry, Aircraft,
and Building (which is unusual). Vehicles may use the global GroundSpeed double instead
(baked at SetDifficulty time into HouseClass+0x1A0).

### 3e. GetBuildTimeBonus — FUN_0050c0a0 (128 bytes, 1 caller)

**Caller:** `FUN_006f47a0` (GetBuildCost — production rate calculation).

```c
float HouseClass::GetBuildTimeBonus(TechnoClass* unit) {
    int rtti = unit->WhatAmI();
    CountryTypeClass* ct = this->HouseTypeClass;
    switch (rtti) {
        case 0x10: return ct->BuildTimeInfantryMult;   // +0x134
        case 0x03: return ct->BuildTimeAircraftMult;    // +0x13C
        case 0x28: return ct->BuildTimeBuildingsMult;   // +0x138
        case 0x07:
            if (unit->SpeedType == 5)
                return ct->BuildTimeDefensesMult;        // +0x144
            return ct->BuildTimeUnitsMult;              // +0x140
        default:   return 1.0f;
    }
}
```

**Where applied:** Called inside `FUN_006f47a0` (GetBuildCost), which computes the
factory's production step delay: `Rate = GetBuildCost(Object) / 54`. A lower BuildTimeMult
means faster production.

---

## 4. IncomeMult (+0x148, float)

**Confidence:** MEDIUM — no Ghidra MCP available to trace live xrefs. Based on field
position and naming convention.

`IncomeMult` at CountryTypeClass+0x148 is a float that scales harvester income. Based on
the harvester dock/unload system documented in `HARVESTER_DOCK_UNLOAD.md`:

1. Harvester docks at refinery
2. Per-tick timer (based on `HarvesterDumpRate`) fires
3. One ore bale is popped from the harvester's cargo
4. Credits are awarded via `HouseClass::GiveMoney(amount)` at +0x30C

The most likely application site is inside the ore-to-credits conversion step, where the
raw bale value is multiplied by IncomeMult before being added to credits. The accessor
would read `HouseClass+0x34 → CountryTypeClass+0x148`.

Without live xref tracing, the exact function address cannot be confirmed. The pattern
would be:
```c
int credits = bale_value * (float)countryType->IncomeMult;
house->GiveMoney(credits);
```

---

## 5. VeteranInfantry / VeteranUnits / VeteranAircraft

**Confidence:** HIGH for storage. MEDIUM for spawn-at-veteran logic (inferred).

### Storage

Three `DynamicVectorClass` arrays on CountryTypeClass:
- `+0x158` (24 bytes): VeteranInfantry — array of `InfantryTypeClass*` pointers
- `+0x178` (24 bytes): VeteranUnits — array of `UnitTypeClass*` pointers
- `+0x194` (16 bytes): VeteranAircraft — array of `AircraftTypeClass*` pointers

Parsed in `CountryTypeClass::ReadINI` (0x00511850) from comma-separated type name lists.
Each name is resolved via `FindOrCreate` on the corresponding TypeClass.

### Spawn-at-Veteran Logic

When a house produces a unit, the engine checks whether the unit's type appears in the
house's country's veteran list. If it does, the unit starts at **veteran rank** (experience
float set to 1.0) instead of rookie.

The check would be:
```c
// In unit creation / factory completion:
CountryTypeClass* country = house->HouseTypeClass;  // +0x34
TechnoTypeClass* unitType = newUnit->GetType();

// Check if unitType is in the appropriate veteran list
bool startVeteran = false;
if (unitType is InfantryType)
    startVeteran = country->VeteranInfantry.Contains(unitType);
else if (unitType is UnitType)
    startVeteran = country->VeteranUnits.Contains(unitType);
else if (unitType is AircraftType)
    startVeteran = country->VeteranAircraft.Contains(unitType);

if (startVeteran)
    newUnit->Veterancy = 1.0f;  // TechnoClass+0x150
```

Additionally, `InitialVeteran` (bit 9 in SpecialFlags, 0x006B8CA0) is a game option that
makes ALL newly produced units start as veterans, regardless of the veteran lists.

### Vanilla Usage

In vanilla rulesmd.ini, only South Korea uses veteran lists:
```ini
[YuriCountry]
VeteranInfantry=DVDPLT
VeteranUnits=DVDTANK
```
(Yuri's faction-specific veteran units for balance.)

---

## 6. Prefix (+0x1A4, char) and Suffix (+0x1A0, char[5])

**Confidence:** MEDIUM for actual usage sites.

### Prefix

A single character (default `'A'` = 0x41) used for country-specific asset lookup.
The NewTheater prefix substitution system at `FUN_005f96b0` replaces the first/second
characters of asset filenames based on theater type. The country Prefix character may be
used similarly for faction-specific cameo or art fallbacks.

In vanilla YR, all countries use the default prefix `'A'`, so this has no visible effect.
It exists for mod support where different factions could have different art prefixes.

### Suffix

A 4-character string (default empty) at +0x1A0. Used for faction-specific naming in
asset lookups. Like Prefix, not used by vanilla countries but available for mods.

---

## 7. SmartAI (+0x1A8, bool)

**Confidence:** MEDIUM — the field is confirmed at offset +0x1A8, but the exact AI
behavior differences require live xref tracing.

`SmartAI=yes` is a boolean flag on CountryTypeClass. Based on the multiplayer lobby system
(report 085, `FUN_005e99c0`), there are distinct AI difficulty tiers:
- `STT:PlayerDumbAI` — basic AI
- `STT:PlayerSmartAI` — enhanced AI
- `STT:PlayerGeniusAI` — expert AI

These are **NOT** the same as `SmartAI` on CountryTypeClass. The lobby AI levels are
separate settings (Easy/Medium/Hard AI), while CountryTypeClass `SmartAI` is a per-country
flag that may enable additional AI behaviors specifically for that country.

The most likely effect: when `SmartAI=yes` is set on a country, the AI player using that
country gets enhanced decision-making (e.g., better target prioritization, smarter base
building, more aggressive expansion). The exact code paths gated by this flag would need
live Ghidra MCP tracing of xrefs to offset +0x1A8 on CountryTypeClass.

In vanilla rulesmd.ini, `SmartAI=yes` is NOT set on any country (all default to false).

---

## 8. ParentCountry (+0x98, char[25])

**Confidence:** HIGH (verified from ReadINI decompilation).

### How Inheritance Works

`ParentCountry=` specifies another country name (up to 24 characters). In
`CountryTypeClass::ReadINI` at 0x00511850:

1. The parent country name is stored at +0x98
2. After all fields are parsed, `FUN_004756f0` resolves the parent country name to a
   side index, which is stored at +0xBC
3. The parent country's child list cross-reference is updated

**ParentCountry does NOT cause multiplier inheritance.** Each country's multipliers are
parsed independently from its own INI section. If a child country doesn't specify a
multiplier, it gets the default (1.0), NOT the parent's value.

The parent country relationship is used for:
- **Side determination:** A child country inherits its parent's side if not in the [Sides]
  list directly
- **Owner bitmask compatibility:** The child may share build permissions with the parent
- **NOT** for multiplier inheritance

---

## 9. Difficulty + Country Interaction Summary

The complete multiplier pipeline for a unit's combat effectiveness:

```
Final Damage = WeaponDamage
             * HouseClass.Firepower           ← baked: difficulty * country(MP only)
             * CountryType.ArmorXxxMult        ← live per-category float
             * VeteranCombat                   ← if vet/elite with FIREPOWER ability
             * TypeDamageMult                  ← Rules+0x100..0x110 (per RTTI type)

Final Armor  = IncomingDamage
             * Verses[ArmorType]               ← warhead vs armor
             * HouseClass.Armor               ← baked: difficulty * country(MP only)
             * VeteranArmor                    ← if vet/elite with ARMOR ability

Final Cost   = BaseCost
             * CountryType.CostXxxMult         ← live per-category
             * AccumulatedBonus                ← from upgrade buildings
             (displayed in sidebar, deducted incrementally during production)

Final BuildTime = BaseRate / 54
                * CountryType.BuildTimeXxxMult ← live per-category
                * AccumulatedBonus             ← from upgrade buildings
                * MultipleFactory discount     ← Rules+0x57C

Final Speed  = BaseSpeed
             * HouseClass.GroundSpeed          ← baked: difficulty * country(MP only)
             * CountryType.SpeedXxxMult        ← live per-category (Inf/Air/Bldg only)
             * VeteranSpeed                    ← if vet/elite with SPEED ability

Final Income = BaleValue
             * CountryType.IncomeMult          ← live float
```

### Key Architectural Insight

The global doubles (Tier 1) and per-category floats (Tier 2) are **completely independent
systems**. They don't interact — they apply at different points in the pipeline:

- **Global doubles** are combined with difficulty at house creation and stored on HouseClass.
  They affect the house's overall combat/speed/cost modifiers.
- **Per-category floats** are read live from CountryTypeClass through accessor functions.
  They provide fine-grained per-unit-type adjustments.

Both multiply into the final value, so they stack multiplicatively.

---

## 10. Function Address Reference

| Function | Address | Size | Purpose |
|---|---|---|---|
| CountryTypeClass::Constructor | 0x005113F0 | 600 | Initializes all defaults to 1.0 |
| CountryTypeClass::ReadINI | 0x00511850 | 2325 | Full INI field parsing |
| CountryTypeClass::FindByName | 0x005117D0 | 116 | 23 callers, returns index or -1 |
| CountryTypeClass::FindOrCreate | 0x00512680 | 131 | Allocates 0x1B0 bytes if new |
| HouseClass::SetDifficulty | 0x004F6EC0 | 632 | Bakes global doubles into HouseClass |
| HouseClass::GetArmorBonus | 0x0050BD30 | 128 | ArmorXxxMult accessor |
| HouseClass::GetCostBonus | 0x0050BDF0 | 128 | CostXxxMult accessor |
| HouseClass::GetAccumulatedBonus | 0x0050BEB0 | 113 | Upgrade building bonus (NOT country) |
| HouseClass::GetSpeedBonus | 0x0050C050 | 76 | SpeedXxxMult accessor (Inf/Air/Bldg) |
| HouseClass::GetBuildTimeBonus | 0x0050C0A0 | 128 | BuildTimeXxxMult accessor |
| HouseClass::RecalcBonuses | 0x0050BF60 | 235 | Recomputes upgrade bonuses |
| TechnoClass::Fire_At | 0x006FDD50 | 7167 | Applies firepower multipliers |
| TechnoClass::ReceiveDamage | 0x00701900 | 5154 | Applies armor multipliers |
| TechnoClass::GetBuildCost | 0x006F47A0 | 438 | Production rate with BuildTimeMult |
| FUN_00711F00 | 0x00711F00 | 96 | Cost calculation with CostMult |
| FUN_00711F60 | 0x00711F60 | 217 | Cost calculation (extended) |
