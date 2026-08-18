# AI Difficulty System — Ghidra Research Report

**Confidence level:** HIGH — all offsets verified from decompiled binary (gamemd.exe).
Cross-referenced against CountryTypeClass field map (report 052), RulesClass parser
(report 105), HouseClass constructor/SetDifficulty (report 047), IQ parser (report 105),
and AI trigger system (report 009).

---

## 1. DifficultyClass Structure (0x50 = 80 bytes)

Parsed by `FUN_0066d270` at address `0x0066d270` (302 bytes). Each DifficultyClass
occupies 0x50 bytes and stores gameplay bias multipliers read from INI.

### DifficultyClass field layout

| Offset | Size | Type   | INI Key        | Default      |
|--------|------|--------|----------------|--------------|
| +0x00  | 8    | double | `FirePower`    | 1.0          |
| +0x08  | 8    | double | `Groundspeed`  | 1.0          |
| +0x10  | 8    | double | `Airspeed`     | 1.0          |
| +0x18  | 8    | double | `Armor`        | 1.0          |
| +0x20  | 8    | double | `ROF`          | 1.0          |
| +0x28  | 8    | double | `Cost`         | 1.0          |
| +0x30  | 8    | double | `BuildTime`    | 1.0          |
| +0x38  | 8    | double | `RepairDelay`  | 0.02         |
| +0x40  | 8    | double | `BuildDelay`   | 0.03         |
| +0x48  | 1    | bool   | `BuildSlowdown`| false        |
| +0x49  | 1    | bool   | `DestroyWalls` | true         |
| +0x4A  | 1    | bool   | `ContentScan`  | false        |

**Note:** The decompiler shows `param_2[6] = BuildTime` stored after `param_2[5] = Cost`
but before RepairDelay at `param_2[7]`. The `param_2` pointer is `double*`, so:
- `param_2[0]` = FirePower, `param_2[1]` = Groundspeed, `param_2[2]` = Airspeed
- `param_2[3]` = Armor, `param_2[4]` = ROF, `param_2[5]` = Cost
- `param_2[6]` = BuildTime, `param_2[7]` = RepairDelay, `param_2[8]` = BuildDelay
- Booleans packed at byte offsets: `param_2+9` = BuildSlowdown, `param_2+0x49` = DestroyWalls, `param_2+0x4A` = ContentScan

### Default values (IEEE-754 hex)

- 1.0 = `0x3FF00000_00000000`
- 0.02 (RepairDelay) = `0x3F947AE1_47AE147B`
- 0.03 (BuildDelay) = `0x3F9EB851_EB851EB8`

### Decompiled pseudocode: `DifficultyClass::Read(section_name, output, ini)`

```c
// FUN_0066d270 @ 0x0066d270
void DifficultyClass__Read(char* section_name, double* output, CCINIClass* ini) {
    if (!INI_SectionExists(ini, section_name))
        return;

    output[0] = ReadDouble(ini, section_name, "FirePower",    1.0);   // +0x00
    output[1] = ReadDouble(ini, section_name, "Groundspeed",  1.0);   // +0x08
    output[2] = ReadDouble(ini, section_name, "Airspeed",     1.0);   // +0x10
    output[3] = ReadDouble(ini, section_name, "Armor",        1.0);   // +0x18
    output[4] = ReadDouble(ini, section_name, "ROF",          1.0);   // +0x20
    output[5] = ReadDouble(ini, section_name, "Cost",         1.0);   // +0x28
    output[7] = ReadDouble(ini, section_name, "RepairDelay",  0.02);  // +0x38
    output[8] = ReadDouble(ini, section_name, "BuildDelay",   0.03);  // +0x40
    *(bool*)(output + 9)    = ReadBool(ini, section_name, "BuildSlowdown", false); // +0x48
    output[6] = ReadDouble(ini, section_name, "BuildTime",    1.0);   // +0x30
    *(bool*)((char*)output + 0x49) = ReadBool(ini, section_name, "DestroyWalls", true);
    *(bool*)((char*)output + 0x4A) = ReadBool(ini, section_name, "ContentScan", false);
}
```

---

## 2. RulesClass Difficulty Storage

### Three DifficultyClass slots in RulesClass

The `FUN_00668bf0` (RulesClass::ParseAllSections) at `0x00668BF0` calls the difficulty
parser three times:

```c
// Line 2594-2596 of decompiled 105_006646c0_00679fa0.c
DifficultyClass__Read(&DAT_00818134);   // slot 0: unnamed default / "Easy"
DifficultyClass__Read("Normal");         // slot 1: [Normal] section
DifficultyClass__Read("Difficult");      // slot 2: [Difficult] section
```

**RulesClass difficulty array offsets** (each slot is 0x50 bytes):

| Difficulty Index | INI Section  | RulesClass Offset | Byte Range          |
|-----------------|--------------|-------------------|---------------------|
| 0 (Easy)        | unnamed/empty| +0x1538           | 0x1538 - 0x1587     |
| 1 (Normal)      | `[Normal]`   | +0x1588           | 0x1588 - 0x15D7     |
| 2 (Difficult)   | `[Difficult]`| +0x15D8           | 0x15D8 - 0x1627     |

**Formula:** `RulesClass + 0x1538 + (difficulty_index * 0x50)`

### GameSpeedBias

At `RulesClass+0x1418` (double), parsed from `[General]` `GameSpeedBias=`.
Used as an additional multiplier on Groundspeed and Airspeed when computing
per-house difficulty values. Default: 1.0.

---

## 3. HouseClass::SetDifficulty (per-house application)

**Function:** `FUN_004f6ec0` at `0x004F6EC0` (632 bytes)
**Called by:** 4 callers — `FUN_0050a5c0` (ComputerTakeover), `FUN_004c6210`
(credits restore), `FUN_005009b0` (house INI load), `FUN_00687f10` (create houses).

### HouseClass difficulty field map

| HouseClass Offset | Size | Purpose                       |
|-------------------|------|-------------------------------|
| +0x184            | 4    | Difficulty level index (0/1/2)|
| +0x188            | 8    | FirepowerBias (double)        |
| +0x190            | 8    | GroundspeedBias (double)      |
| +0x198            | 8    | AirspeedBias (double)         |
| +0x1A0            | 8    | ArmorBias (double)            |
| +0x1A8            | 8    | ROFBias (double)              |
| +0x1B0            | 8    | CostBias (double)             |
| +0x1B8            | 8    | BuildTimeBias (double)        |
| +0x1C0            | 8    | RepairDelayBias (double)      |
| +0x1C8            | 8    | BuildDelayBias (double)       |
| +0x57A0           | 4    | Score factor (int)            |

### Initialization (HouseClass constructor at 0x4F54A0)

All 7 double-precision difficulty multipliers are initialized to 1.0 (`0x3FF00000`)
in the constructor:

```c
// param_1 is undefined4*, so indices are dword-based (multiply by 4 for byte offset)
param_1[0x62] = 0;  param_1[0x63] = 0x3FF00000;  // +0x188 = FirepowerBias = 1.0
param_1[0x64] = 0;  param_1[0x65] = 0x3FF00000;  // +0x190 = GroundspeedBias = 1.0
param_1[0x66] = 0;  param_1[0x67] = 0x3FF00000;  // +0x198 = AirspeedBias = 1.0
param_1[0x68] = 0;  param_1[0x69] = 0x3FF00000;  // +0x1A0 = ArmorBias = 1.0
param_1[0x6A] = 0;  param_1[0x6B] = 0x3FF00000;  // +0x1A8 = ROFBias = 1.0
param_1[0x6C] = 0;  param_1[0x6D] = 0x3FF00000;  // +0x1B0 = CostBias = 1.0
param_1[0x6E] = 0;  param_1[0x6F] = 0x3FF00000;  // +0x1B8 = BuildTimeBias = 1.0
```

The constructor does not hardcode `DifficultyLevel`. At `0x004F5637..0x004F5648`
it reads `ScenarioClass+0x610` and copies that dword to `HouseClass+0x184`.
`ScenarioClass` reset `0x00683610` writes `Scenario+0x610 = 1`, so a house that is
not subsequently passed through `SetDifficulty` begins at **Normal (`1`)**.
`ScenarioClass::Create_Houses @ 0x00687F10` explicitly calls
`SetDifficulty(1)` for the multiplayer human house and later calls
`SetDifficulty` with each AI row's own difficulty value. Evidence: live decompile
`0x00683610`; live assembly `0x004F5637..0x004F5648` and
`0x00688122..0x00688132`, `0x006882B1..0x006882B9`.

### SetDifficulty logic (two modes)

**`param_2`** is the difficulty index (`0=Hard`, `1=Normal`, `2=Easy`). It is
written directly to `HouseClass+0x184`; do not invert it at consumers.

#### Single-player / Campaign mode (`DAT_00a8b238 == 0`):

Copies difficulty values directly from RulesClass, applying GameSpeedBias to
speed-related fields:

```c
void HouseClass__SetDifficulty(HouseClass* this, int diffIdx) {
    int old = this->DifficultyLevel;   // +0x184
    this->DifficultyLevel = diffIdx;

    int off = diffIdx * 0x50;
    DifficultyClass* diff = (DifficultyClass*)(Rules + 0x1538 + off);
    double speedBias = Rules->GameSpeedBias;  // Rules+0x1418

    this->FirepowerBias     = diff->FirePower;                         // direct copy
    this->GroundspeedBias   = diff->Groundspeed * speedBias;           // scaled
    this->AirspeedBias      = diff->Airspeed * speedBias;              // scaled
    this->ArmorBias         = diff->Armor;                             // direct copy
    this->ROFBias           = diff->ROF;                               // direct copy
    this->CostBias          = diff->Cost;                              // direct copy
    this->BuildTimeBias     = diff->BuildTime * speedBias;             // scaled
    this->RepairDelayBias   = diff->RepairDelay;                       // direct copy
    this->BuildDelayBias    = diff->BuildDelay;                        // direct copy
}
```

#### Multiplayer / Skirmish mode (`DAT_00a8b238 != 0`):

Additionally multiplies each bias by the corresponding **CountryTypeClass** multiplier
from the player's country/house type (`HouseClass+0x34 = HouseTypeClass*`):

```c
void HouseClass__SetDifficulty_MP(HouseClass* this, int diffIdx) {
    this->DifficultyLevel = diffIdx;
    CountryTypeClass* country = this->HouseType;  // +0x34
    int off = diffIdx * 0x50;
    DifficultyClass* diff = (DifficultyClass*)(Rules + 0x1538 + off);
    double speedBias = Rules->GameSpeedBias;  // Rules+0x1418

    this->FirepowerBias   = diff->FirePower   * country->Firepower;     // +0xC8
    this->GroundspeedBias = diff->Groundspeed * speedBias * country->Groundspeed; // +0xD0
    this->AirspeedBias    = diff->Airspeed    * speedBias * country->Airspeed;    // +0xD8
    this->ArmorBias       = diff->Armor       * country->Armor;         // +0xE0
    this->ROFBias         = diff->ROF         * country->ROF;           // +0xE8
    this->CostBias        = diff->Cost        * country->Cost;          // +0xF0
    this->BuildTimeBias   = diff->BuildTime   * speedBias * country->BuildTime;   // +0xF8
    this->RepairDelayBias = diff->RepairDelay;                          // NOT scaled by country
    this->BuildDelayBias  = diff->BuildDelay;                           // NOT scaled by country
}
```

**Key observation:** RepairDelay and BuildDelay are NOT multiplied by country multipliers
in multiplayer mode. Only the 7 main combat/economy biases get the country scaling.

### Score factor computation

After setting all biases, the function computes a score factor:

```c
// Rules+0x115C is a pointer to an int array indexed by difficulty
int baseFactor = *(int*)(*(int*)(Rules + 0x115C) + diffIdx * 4);
this->ScoreFactor = baseFactor + this->HouseIndex * 0xAF;  // +0x57A0
```

---

## 4. CountryTypeClass Multipliers

Parsed by `FUN_00511850` (CountryTypeClass::ReadINI). Each country defines per-faction
difficulty scaling. These multiply the DifficultyClass values in multiplayer.

### 7 global multipliers (doubles, used by SetDifficulty)

| CountryTypeClass Offset | INI Key        |
|-------------------------|----------------|
| +0xC8                   | `Firepower`    |
| +0xD0                   | `Groundspeed`  |
| +0xD8                   | `Airspeed`     |
| +0xE0                   | `Armor`        |
| +0xE8                   | (unnamed = ROF)|
| +0xF0                   | (unnamed = Cost)|
| +0xF8                   | `BuildTime`    |

All default to 1.0 (`0x3FF00000`).

### 15 per-category multipliers (floats, used by House bonus functions)

These are used by `House::GetBuildSpeedBonus`, `House::GetCostBonus`,
`House::GetArmorBonus` (at 0x50BD30, 0x50BDF0, 0x50C0A0), and similar per-unit-type
bonus functions. They switch on RTTI type:

- 0x10 = Infantry
- 0x28 = Aircraft
- 7 = Unit (with sub-check: naval type 5 gets separate slot)
- 3 = Building/Defense

| Offset | INI Key                    | Category    |
|--------|----------------------------|-------------|
| +0x100 | `ArmorInfantryMult`        | Armor       |
| +0x104 | `ArmorUnitsMult`           | Armor       |
| +0x108 | `ArmorAircraftMult`        | Armor       |
| +0x10C | `ArmorBuildingsMult`       | Armor       |
| +0x110 | `ArmorDefensesMult`        | Armor       |
| +0x114 | `CostInfantryMult`         | Cost        |
| +0x118 | `CostUnitsMult`            | Cost        |
| +0x11C | `CostAircraftMult`         | Cost        |
| +0x120 | `CostBuildingsMult`        | Cost        |
| +0x124 | `CostDefensesMult`         | Cost        |
| +0x128 | `SpeedInfantryMult`        | Speed       |
| +0x12C | `SpeedUnitsMult`           | Speed       |
| +0x130 | `SpeedAircraftMult`        | Speed       |
| +0x134 | `BuildTimeInfantryMult`    | BuildTime   |
| +0x138 | `BuildTimeUnitsMult`       | BuildTime   |
| +0x13C | `BuildTimeAircraftMult`    | BuildTime   |
| +0x140 | `BuildTimeBuildingsMult`   | BuildTime   |
| +0x144 | `BuildTimeDefensesMult`    | BuildTime   |
| +0x148 | `IncomeMult`               | Economy     |

---

## 5. AI IQ System

**Parser:** `FUN_00674240` at `0x00674240` (374 bytes)
**INI Section:** `[IQ]` (referenced via `PTR_DAT_007f0cdc`)

### RulesClass IQ field map

| RulesClass Offset | INI Key          | Purpose                                |
|-------------------|------------------|----------------------------------------|
| +0x1434           | `MaxIQLevels`    | Maximum IQ level (integer)             |
| +0x1438           | `SuperWeapons`   | IQ threshold for super weapon use      |
| +0x143C           | `Production`     | IQ threshold for production decisions  |
| +0x1440           | `GuardArea`      | IQ threshold for guard area commands   |
| +0x1444           | `RepairSell`     | IQ threshold for repair/sell decisions |
| +0x1448           | `AutoCrush`      | IQ threshold for auto-crush behavior   |
| +0x144C           | `Scatter`        | IQ threshold for scatter under fire    |
| +0x1450           | `ContentScan`    | IQ threshold for IFV content scanning  |
| +0x1454           | `Aircraft`       | IQ threshold for aircraft management   |
| +0x1458           | `Harvester`      | IQ threshold for harvester management  |
| +0x145C           | `SellBack`       | IQ threshold for selling back buildings|

### Decompiled pseudocode

```c
// FUN_00674240 @ 0x00674240
bool RulesClass__ReadIQ(RulesClass* this, CCINIClass* ini) {
    if (!INI_SectionExists(ini, "IQ"))
        return false;

    this->MaxIQLevels  = ReadInt(ini, "IQ", "MaxIQLevels",  this->MaxIQLevels);  // +0x1434
    this->IQSuperWeapons = ReadInt(ini, "IQ", "SuperWeapons", this->IQSuperWeapons); // +0x1438
    this->IQProduction   = ReadInt(ini, "IQ", "Production",   this->IQProduction);   // +0x143C
    this->IQGuardArea    = ReadInt(ini, "IQ", "GuardArea",    this->IQGuardArea);    // +0x1440
    this->IQRepairSell   = ReadInt(ini, "IQ", "RepairSell",   this->IQRepairSell);   // +0x1444
    this->IQAutoCrush    = ReadInt(ini, "IQ", "AutoCrush",    this->IQAutoCrush);    // +0x1448
    this->IQScatter      = ReadInt(ini, "IQ", "Scatter",      this->IQScatter);      // +0x144C
    this->IQContentScan  = ReadInt(ini, "IQ", "ContentScan",  this->IQContentScan);  // +0x1450
    this->IQAircraft     = ReadInt(ini, "IQ", "Aircraft",     this->IQAircraft);     // +0x1454
    this->IQHarvester    = ReadInt(ini, "IQ", "Harvester",    this->IQHarvester);    // +0x1458
    this->IQSellBack     = ReadInt(ini, "IQ", "SellBack",     this->IQSellBack);     // +0x145C
    return true;
}
```

### How IQ interacts with difficulty

The IQ system is **independent** from the difficulty multiplier system. IQ thresholds
control which AI behaviors are enabled (a higher IQ level means more sophisticated AI),
while difficulty biases scale the raw numerical values (damage, speed, cost, etc.).

**AI IQ level** is stored at `HouseClass+0x184` (the difficulty level field). The AI
trigger evaluation function (`FUN_0041e720` at `0x0041E720`) checks it as condition
type 3:

```
condition type 3 = IQ level check: compares the house's difficulty index against a
threshold to determine if the AI trigger should fire.
```

AI triggers also have per-difficulty enable flags:
- `+0xD0` / `+0xD2` / `+0xD3` / `+0xD4` — enable bits for Easy/Normal/Hard

The scenario/trigger system reads the global difficulty at `ScenarioClass+0x60C`
(`DAT_00a8b230+0x60C`) where:
- 0 = Easy
- 1 = Normal (Medium)
- 2 = Hard

This is separate from the per-house difficulty index and is used for campaign trigger
filtering. Triggers can be selectively enabled per difficulty via byte flags at
`TriggerClass+0x9C` (Easy), `+0x9D` (Normal), `+0x9E` (Hard).

---

## 6. Campaign-Specific Difficulty Settings

### OptionsClass difficulty storage

The `OptionsClass` (loaded from `SUN.INI / RA2MD.INI`) stores:

| OptionsClass Offset | INI Key           | Range   | Purpose                |
|---------------------|-------------------|---------|------------------------|
| +0x04               | `Difficulty`      | 0-4     | Skirmish/MP difficulty  |
| +0x08               | `CampDifficulty`  | 0-2     | Campaign difficulty     |

### CampaignMoneyDelta

Parsed from `[General]` section, stored in RulesClass:

| RulesClass Offset | INI Key                  | Purpose                          |
|-------------------|--------------------------|----------------------------------|
| +0xDFC            | `CampaignMoneyDeltaEasy` | Money bonus for Easy campaign    |
| +0xE00            | `CampaignMoneyDeltaHard` | Money penalty for Hard campaign  |

These are integer values (read with `FUN_005276d0` = ReadInt). They adjust starting
money in campaign missions based on the campaign difficulty setting.

### Other campaign-adjacent difficulty parameters from [General]

| RulesClass Offset | INI Key                          |
|-------------------|----------------------------------|
| +0xDF8            | `ApproachTargetResetMultiplier`  |
| +0xE04            | `GuardAreaTargetingDelay`        |
| +0xE08            | `NormalTargetingDelay`           |
| +0xE0C            | `AINavalYardAdjacency`           |

### CompEasyBonus

Parsed from `[AI]` section:

| RulesClass Offset | INI Key        | Type | Purpose                          |
|-------------------|----------------|------|----------------------------------|
| +0x17E3           | `CompEasyBonus`| bool | Extra bonus for AI on Easy       |
| +0x17E0           | `Paranoid`     | bool | AI attacks all players equally   |

When `CompEasyBonus` is true and the player count is > 1 in multiplayer, the AI
gets additional advantages (noted in `FUN_00687F10` — ScenarioClass::Create_Houses).

---

## 7. Difficulty Flow: Lobby to Gameplay

### Multiplayer/Skirmish flow

1. **Lobby setup:** Player selects AI difficulty via slider. Stored in per-slot
   array `DAT_00a8b27c[8]` — AI difficulty index per player slot. Values: 0=Hard,
   1=Normal, 2=Easy. This is the same index convention stored at
   `HouseClass+0x184`, not an inverted scenario convention.

2. **Game initialization:** `FUN_00687F10` (ScenarioClass::Create_Houses) iterates
   AI slots. For each valid AI, creates a HouseClass and calls `SetDifficulty`
   with the lobby difficulty index.

3. **SetDifficulty** reads from `RulesClass+0x1538 + (index * 0x50)` and multiplies
   by `CountryTypeClass` multipliers for multiplayer. Stores final values in
   `HouseClass+0x188..+0x1CC`.

### Campaign flow

1. **Options:** `CampDifficulty` (0-2) stored in SUN.INI.

2. **Scenario load:** `FUN_00689020` reads `DAT_00a8eb64` (campaign difficulty) and
   uses it as the human player's difficulty index. Money adjusted by
   `CampaignMoneyDeltaEasy`/`CampaignMoneyDeltaHard`.

3. **AI players in campaigns** get their difficulty set independently via trigger
   actions (trigger action types 0x47, 0x48 write float values to
   `RulesClass+0x1670` and `+0x1668`).

---

## 8. Where Difficulty Biases Are Applied

The difficulty multipliers stored in HouseClass are consumed by the gameplay systems:

### Firepower (HouseClass+0x188)
- Applied when computing weapon damage dealt by units owned by this house.
- Multiplied into the damage calculation in combat resolution functions.

### Groundspeed (HouseClass+0x190) and Airspeed (HouseClass+0x198)
- Applied to unit movement speed calculations.
- Combined with `GameSpeedBias` from `[General]`.

### Armor (HouseClass+0x1A0)
- Applied when computing damage received by units owned by this house.
- Acts as a damage reduction multiplier.

### ROF (HouseClass+0x1A8)
- Applied to rate-of-fire calculations (lower = faster firing).
- Consumed by `TechnoClass::GetROF` and related functions.

### Cost (HouseClass+0x1B0)
- Applied to production cost calculations.
- Affects both money deduction and production time.

### BuildTime (HouseClass+0x1B8)
- Applied to building/unit production duration.
- Combined with `GameSpeedBias`.

### Per-unit-type bonuses (HouseClass+0x5390..+0x53A0)

Additionally, the `House::RecalcBonuses` function (`FUN_0050BF60`) accumulates
per-category multipliers from owned upgrade buildings (offsets +0x16D0..+0x16E0 in
BuildingTypeClass). These are stored as 5 floats at HouseClass+0x5390:

| Offset  | Category        |
|---------|-----------------|
| +0x5390 | InfantryBonus   |
| +0x5394 | NavalBonus      |
| +0x5398 | AircraftBonus   |
| +0x539C | VehicleBonus    |
| +0x53A0 | VehicleAltBonus |

The bonus functions (`GetBuildSpeedBonus`, `GetCostBonus`, `GetArmorBonus`,
`GetRepairBonus`) switch on unit RTTI type to select the appropriate country
multiplier from CountryTypeClass.

---

## 9. Adaptive AI Difficulty (AITriggerTypeClass)

Separate from the static difficulty system, the AI trigger system has **adaptive
difficulty tuning** via two functions:

### FUN_0041FD60 — AI difficulty increase (191 bytes)

```c
void AITriggerType__IncreaseWeight(AITriggerTypeClass* this) {
    int totalAttempts = this->TotalAttempts;  // +0x108
    double adjustment = 0.0;
    if (totalAttempts > 0) {
        double hitRatio = (double)this->Successes / (double)totalAttempts;  // +0x104
        adjustment = (hitRatio - THRESHOLD) * totalAttempts;
        if (adjustment < 0.0) adjustment = 0.0;
    }
    double newWeight = Rules->AITriggerBaseRate + this->Weight + adjustment;
    // Rules+0xC0 = base rate for increase
    this->Weight = newWeight;                  // +0xB8

    // Clamp to [min, max]
    if (newWeight < this->MinWeight) this->Weight = this->MinWeight;  // +0xC0
    if (newWeight > this->MaxWeight) this->Weight = this->MaxWeight;  // +0xC8

    this->Successes++;   // +0x104
    this->TotalAttempts++;  // +0x108
}
```

### FUN_0041FE20 — AI difficulty decrease (187 bytes)

Same structure but uses `Rules+0xD0` (decay multiplier) and `Rules+0xC8` (base rate
for decrease). Ensures the adjustment is <= 0 (only decreases weight). Does NOT
increment Successes, only TotalAttempts.

### AITriggerTypeClass field map (relevant offsets)

| Offset | Type   | Purpose                                |
|--------|--------|----------------------------------------|
| +0x98  | int    | Condition type (-1 to 7)               |
| +0xA0  | int    | Side restriction (0=any, 1=specific)   |
| +0xB0  | int    | Tech level requirement                 |
| +0xB8  | double | Current weight (adaptive)              |
| +0xC0  | double | Minimum weight                         |
| +0xC8  | double | Maximum weight                         |
| +0xD0  | byte   | Enable for difficulty 0 (Easy)         |
| +0xD2  | byte   | Enable for difficulty 1 (Normal)       |
| +0xD3  | byte   | Enable for difficulty 2 (Hard)         |
| +0xD4  | byte   | Enable for difficulty 3 (extra?)       |
| +0xDC  | ptr    | Primary weapon type                    |
| +0xE0  | ptr    | Secondary weapon type                  |
| +0x104 | int    | Success count                          |
| +0x108 | int    | Total attempt count                    |

---

## 10. Summary: Complete Data Flow

```
  rulesmd.ini                          SUN.INI
  ┌─────────────────┐                 ┌───────────────┐
  │ [Easy] (unnamed)│                 │ Difficulty=0-4│
  │ [Normal]        │                 │ CampDiff=0-2  │
  │ [Difficult]     │                 └───────┬───────┘
  │ [IQ]            │                         │
  │ [General]       │                         ▼
  │   GameSpeedBias │               OptionsClass+0x04,+0x08
  │   CampMoney*    │
  └────────┬────────┘
           │
           ▼
  RulesClass (DAT_008871e0)
  ┌────────────────────────────────┐
  │ +0x1418: GameSpeedBias         │
  │ +0x1434: IQ fields (11 ints)  │
  │ +0x1538: DifficultyClass[0]   │ ← Easy    (0x50 bytes)
  │ +0x1588: DifficultyClass[1]   │ ← Normal  (0x50 bytes)
  │ +0x15D8: DifficultyClass[2]   │ ← Hard    (0x50 bytes)
  │ +0xDFC:  CampaignMoneyDeltaEasy│
  │ +0xE00:  CampaignMoneyDeltaHard│
  │ +0x17E3: CompEasyBonus         │
  └────────────┬───────────────────┘
               │
     SetDifficulty(diffIdx)
               │
               ▼
  HouseClass (per player)
  ┌────────────────────────────────┐
  │ +0x184: DifficultyLevel (0/1/2)│
  │ +0x188: FirepowerBias          │ ← diff[i].FirePower * country.Firepower
  │ +0x190: GroundspeedBias        │ ← diff[i].Groundspeed * speedBias * country.Groundspeed
  │ +0x198: AirspeedBias           │ ← diff[i].Airspeed * speedBias * country.Airspeed
  │ +0x1A0: ArmorBias              │ ← diff[i].Armor * country.Armor
  │ +0x1A8: ROFBias                │ ← diff[i].ROF * country.ROF
  │ +0x1B0: CostBias               │ ← diff[i].Cost * country.Cost
  │ +0x1B8: BuildTimeBias          │ ← diff[i].BuildTime * speedBias * country.BuildTime
  │ +0x1C0: RepairDelayBias        │ ← diff[i].RepairDelay (no country mult)
  │ +0x1C8: BuildDelayBias         │ ← diff[i].BuildDelay (no country mult)
  │ +0x57A0: ScoreFactor           │
  └────────────────────────────────┘
               │
               ▼
    Consumed by combat, movement,
    production, and AI systems
```

---

## Addresses Reference

| Address    | Function                              |
|------------|---------------------------------------|
| 0x0066D270 | DifficultyClass::Read (parser)        |
| 0x00668BF0 | RulesClass::ParseAllSections (caller)  |
| 0x00674240 | RulesClass::ReadIQ                    |
| 0x004F54A0 | HouseClass constructor (full)          |
| 0x004F6EC0 | HouseClass::SetDifficulty              |
| 0x00683610 | ScenarioClass reset (default difficulty `1`) |
| 0x0050BD30 | House::GetBuildSpeedBonus              |
| 0x0050BDF0 | House::GetCostBonus                    |
| 0x0050BEB0 | House::GetAccumulatedBonus             |
| 0x0050BF60 | House::RecalcBonuses                   |
| 0x0050C050 | House::GetRepairBonus                  |
| 0x0050C0A0 | House::GetArmorBonus                   |
| 0x0041FD60 | AITriggerType::IncreaseWeight          |
| 0x0041FE20 | AITriggerType::DecreaseWeight          |
| 0x0041E720 | AITriggerType::Evaluate                |
| 0x00687F10 | ScenarioClass::Create_Houses           |
| 0x00665650 | RulesClass default values init         |
| 0x0050A5C0 | House::ComputerTakeover                |
| 0x00705D70 | TechnoClass::GetHouseDifficultyMult    |
