# Owner Bitmask, Tech/Prerequisite, and Build Eligibility System — gamemd.exe

**Date:** 2026-03-22
**Source:** Ghidra decompilation of gamemd.exe + verified research reports
**Scope:** RESEARCH ONLY — no code changes

---

## 1. Owner Bitmask Parsing

### The Parser: FUN_00475260

**Address:** `0x00475260`
**Called by:** `FUN_00500b40` (TechnoTypeClass::ReadINI uses `FUN_004750d0`, a sibling)
**Purpose:** Reads a comma-separated list of country names from INI and builds a 32-bit bitmask.

**Pseudocode:**
```c
uint32_t ReadOwnerBitmask(INIClass* ini, char* section, char* key, uint32_t default_value)
{
    char buffer[128];
    INI_GetString(ini, section, key, "", buffer, 128);  // FUN_00528a10

    if (buffer[0] == '\0')
        return default_value;

    uint32_t bitmask = 0;
    char* token = strtok(buffer, ",");   // FUN_007c9cc2, delimiter at DAT_00817f70

    while (token != NULL) {
        // FUN_0050c170: looks up country name in CountryTypeClass::Array
        // Returns the country's self-index (0-31) as a byte
        uint8_t country_index = CountryTypeClass_FindIndex(token);  // FUN_0050c170

        bitmask |= (1 << country_index);   // Set the bit for this country

        token = strtok(NULL, ",");
    }

    return bitmask;
}
```

### The Sibling Parser: FUN_004750d0

**Address:** `0x004750d0`
**Called by:** `FUN_00712170` (TechnoTypeClass::ReadINI)
**Purpose:** Same comma-tokenize pattern but uses `FUN_0048deb0` to convert each token to
a flag value (used for fields like `Prerequisite=` where tokens map to enum IDs rather than
bit positions).

### Country Index Resolver: FUN_0050c170

**Called by:** `FUN_00475260`
**Behavior:** Searches the global `HouseClass::Array` (at `g_HouseClass_Array`, count at
`g_HouseClass_Array_Count`) comparing the input string against each house's name at `+0x15FF4`.
Returns the house's iteration index (0-based). This is `HouseClass__FindByName`, not a
CountryTypeClass lookup. (corrected 2026-05-29: was described as CountryTypeClass_FindIndex
searching CountryTypeClass::Array at DAT_00a83c9c, reading names at +0x24/+0x64, returning
CountryTypeClass+0xB8 self-index; binary at 0x0050c170 shows HouseClass::Array iteration with
name read at +0x15FF4 via decompile_function 0x0050c170 — RTTI_LABEL_DRIFT)

### Country-to-Bit Mapping (vanilla rulesmd.ini)

The bit position comes from the country's index in the `[Countries]` section:

```
[Countries]
0=Americans     -> bit 0   (mask 0x001)
1=Alliance      -> bit 1   (mask 0x002)
2=French        -> bit 2   (mask 0x004)
3=Germans       -> bit 3   (mask 0x008)
4=British       -> bit 4   (mask 0x010)
5=Africans      -> bit 5   (mask 0x020)
6=Arabs         -> bit 6   (mask 0x040)
7=Confederation -> bit 7   (mask 0x080)
8=Russians      -> bit 8   (mask 0x100)
9=YuriCountry   -> bit 9   (mask 0x200)
```

**Maximum:** 32 countries. The bitmask uses `1 << (index & 0x1f)`, so any index beyond 31
wraps around. The 32-bit int is the hard limit.

### Where Stored on TechnoTypeClass

| Byte Offset | INI Key | Type | Default | Description |
|-------------|---------|------|---------|-------------|
| `+0x6CC` | `Owner=` | uint32 bitmask | (all bits set) | Which countries can build/own this |
| `+0xDA0` | `RequiredHouses=` | uint32 bitmask | `-1` (0xFFFFFFFF = all) | Further country restriction |
| `+0xDA4` | `ForbiddenHouses=` | uint32 bitmask | `-1` (0xFFFFFFFF = none forbidden) | Country exclusion |
| `+0xDA8` | `SecretHouses=` | uint32 bitmask | `-1` | Houses that unlock via spying |

### Bitmask Write-back: FUN_004752e0

Inverse of reading — iterates `HouseClass::Array` (at `DAT_00a8022c`), for each house
whose index bit is set in the bitmask, appends the house name (at offset `+0x15FF4`) to
a comma-separated string. This is used for save/INI write operations.

**Confidence: HIGH** — The tokenization + bit-shift pattern is clearly visible in the
decompiled function at 0x00475260. The house-name resolver at 0x0050c170 confirmed
via decompile_function 0x0050c170 — it is HouseClass__FindByName, not CountryTypeClass::FindByName.

---

## 2. TechLevel System

### TechLevel on Types

- **TechnoTypeClass+0x634** — `TechLevel=` (int, default -1)
- Value of -1 means the type is unbuildable by players
- Parsed as a simple integer from INI

### TechLevel on Houses

- **HouseClass+0x1D4** — Current tech level for the house
- **Source:** Parsed from the map's per-house INI section via `Read_Scenario_INI` at
  `0x00500B40` using key `TechLevel=`
- **Default:** Comes from `RulesClass+0x1254` (the `[General] TechLevel=` default)

### How TechLevel Advances

The house's TechLevel is set at map load time from the map INI. In standard multiplayer/
skirmish, all houses start with TechLevel=10 (max), meaning TechLevel on types only
serves as a sort key for the sidebar (lower TechLevel items appear first).

In **campaign missions**, the map author sets TechLevel per house to restrict what the
player can build. Triggers can modify TechLevel mid-mission.

### TechLevel Check in CanBuild (0x4F7870)

```c
// Step 4: TechLevel check
if (type->TechLevel == -1)
    return 0;  // unbuildable type (e.g., civilian objects)

if (type->TechLevel > house->TechLevel)
    return 0;  // house hasn't reached this tech tier yet
```

**AI exception:** AI players (Step 8) skip prerequisite checking entirely but still
must pass TechLevel and Owner checks.

**Confidence: HIGH** — House+0x1D4 confirmed from Read_Scenario_INI parser, type+0x634
confirmed from TechnoTypeClass::ReadINI.

---

## 3. Prerequisite System

### Prerequisite Parsing: FUN_004770E0

**Address:** `0x004770E0`
**Purpose:** Converts comma-separated prerequisite strings from INI into a list of integer
IDs, where negative IDs represent generic keyword groups and non-negative IDs represent
specific BuildingType array indices.

**Keyword Resolution (case-insensitive via `_stricmp`):**

| Keyword | Stored ID | Hex | RulesClass Array | RulesClass Count |
|---------|-----------|-----|------------------|------------------|
| `POWER` | -1 | 0xFFFFFFFF | +0x35C | +0x368 |
| `FACTORY` | -2 | 0xFFFFFFFE | +0x378 | +0x384 |
| `BARRACKS` | -3 | 0xFFFFFFFD | +0x394 | +0x3A0 |
| `RADAR` | -4 | 0xFFFFFFFC | +0x3B0 | +0x3BC |
| `TECH` | -5 | 0xFFFFFFFB | +0x3CC | +0x3D8 |
| `PROC` | -6 | 0xFFFFFFFA | +0x3E8 | +0x3F4 |

Tokens that don't match any keyword are looked up by name in `BuildingTypeClass::Array`
and stored as their non-negative array index.

### Default Prerequisite Groups (from `[General]` in rulesmd.ini)

Parsed by `RulesClass::ReadGeneral` at `0x0066E400`:

```ini
PrerequisitePower=GAPOWR,NAPOWR,NANRCT,YAPOWR
PrerequisiteFactory=GAWEAP,NAWEAP,YAWEAP
PrerequisiteBarracks=NAHAND,GAPILE,YABRCK
PrerequisiteRadar=GAAIRC,NARADR,AMRADR,NAPSIS
PrerequisiteTech=GATECH,NATECH,YATECH
PrerequisiteProc=GAREFN,NAREFN,YAREFN
PrerequisiteProcAlternate=SMIN
```

These are side-agnostic: an Allied player with a captured Soviet Barracks satisfies
`BARRACKS`.

### TechnoTypeClass Prerequisite Fields

| Byte Offset | INI Key | Type |
|-------------|---------|------|
| `+0x638..0x650` | `Prerequisite=` | DynamicVector<int> (list of prerequisite IDs) |
| `+0x654..0x66C` | `PrerequisiteOverride=` | DynamicVector<int> (alternative path) |

### OR Groups in Prerequisite=

The `|` (pipe) separator in INI (e.g., `GAWEAP|NAWEAP|YAWEAP`) is NOT parsed by the
engine as a special OR operator within a single prerequisite entry. Instead, the generic
keyword system provides the OR semantics: writing `Prerequisite=FACTORY` means "own any
of GAWEAP, NAWEAP, or YAWEAP."

For explicit OR between specific buildings, the engine provides
`PrerequisiteOverride=`. If ANY building in the override list is owned, the entire
`Prerequisite=` check is bypassed.

### PrerequisiteOverride: The "OR" Path

`PrerequisiteOverride` provides an alternative prerequisite path. If the owner has ANY
building from this list, the engine **skips** the normal `Prerequisite` check entirely:

```ini
; Navy SEAL example from rulesmd.ini:
Prerequisite=GAPILE,RADAR
PrerequisiteOverride=CAWA2A,CAWA2B,CAWA2C,CAWA2D  ; any Pentagon unlocks SEALs
```

### Full Prerequisite Check in HouseClass::CanBuild (0x4F7870)

**ALL prerequisites must be satisfied (AND logic):**

```
for each prereq_id in type.Prerequisite:
    switch (prereq_id):
        case -1 (POWER):
            Must own >= 1 building from PrerequisitePower group
        case -2 (FACTORY):
            Must own >= 1 building from PrerequisiteFactory group
        case -3 (BARRACKS):
            Must own >= 1 building from PrerequisiteBarracks group
        case -4 (RADAR):
            Must own >= 1 building from PrerequisiteRadar group
        case -5 (TECH):
            Must own >= 1 building from PrerequisiteTech group
        case -6 (PROC):
            Must own >= 1 building from PrerequisiteProc group
            OR own PrerequisiteProcAlternate (Slave Miner) AND its
            deploy-building exists (checked via rules+0x400 -> type+0xDF8)
        default (>= 0, specific building index):
            if building type has upgrade flag (type+0xE88):
                Must own a building with this upgrade in slots (+0x17B..+0x17D)
            else:
                Must own >= 1 building of this specific type
                (counted via FUN_0049FAE0)

    if ANY prerequisite fails: return 0
```

The function that counts owned instances of a type is `FUN_0049FAE0`. It iterates
owned buildings and counts matches against the BuildingType array index.

**Confidence: HIGH** — Full CanBuild function decompiled at 0x4F7870 (2804 bytes,
13 callers). All prerequisite group offsets verified from RulesClass::ReadGeneral.

---

## 4. Stolen Tech System

### Per-Type Requirements (TechnoTypeClass)

| Byte Offset | INI Key | Type |
|-------------|---------|------|
| `+0xD9D` | `RequiresStolenAlliedTech=` | bool |
| `+0xD9C` | `RequiresStolenSovietTech=` | bool |
| `+0xD9B` | `RequiresStolenThirdTech=` | bool |

### Per-House Stolen Tech Flags (HouseClass)

| Byte Offset | Field | Default |
|-------------|-------|---------|
| `+0x2BE` | StolenAlliedTech | 0 (false) |
| `+0x2BD` | StolenSovietTech | 0 (false) |
| `+0x2BC` | StolenThirdTech | 0 (false) |

(corrected 2026-05-29: was +0x2BD=Allied, +0x2BE=Soviet, +0x2BF=Third; binary
OnSpyInfiltrate 0x4571E0 shows: +0x6D0==0 → puVar3[0x2be], +0x6D0==1 → puVar3[0x2bd],
+0x6D0==2 → puVar3[700]=puVar3[0x2bc] via decompile_function 0x004571E0 — STRUCT_FAMILY_CASCADE)

### How Spy Infiltration Sets Stolen Tech

**Function:** `BuildingClass::OnSpyInfiltrate` at `0x004571E0` (~965 bytes)

When a spy enters an enemy building:

1. If `TypeClass+0xEE0` >= 1, routes directly to `HouseClass__SpyPowerSabotage`
   (bypassing tech stealing). Tech stealing proceeds only when `TypeClass+0xEE0 < 1`.
   (corrected 2026-05-29: was "if > 0 proceeds with tech stealing"; binary shows the
   branch is `if (*(int *)(puVar1 + 0xee0) < 1)` to enter the tech-steal block, and the
   else-branch calls SpyPowerSabotage via decompile_function 0x004571E0 — OPERATOR_OR_ORDER_DRIFT)
2. Iterates the "stolen tech buildings" list at `RulesClass+0x920` (array of building
   type pointers, count at `+0x92C`) to find the matching entry.
3. Based on the **infiltrated building's side** (`TypeClass+0x6D0`, the
   `AIBasePlanningSide` field which maps 0=Allied, 1=Soviet, 2=Third):
   - Allied building infiltrated → sets `spy_owner_house+0x2BE` (StolenAlliedTech) = 1
   - Soviet building infiltrated → sets `spy_owner_house+0x2BD` (StolenSovietTech) = 1
   - Third/Yuri building infiltrated → sets `spy_owner_house+0x2BC` (StolenThirdTech) = 1
   (corrected 2026-05-29: was 0x2BD=Allied, 0x2BE=Soviet, 0x2BF=Third; binary shows
   puVar3[0x2be] for side==0, puVar3[0x2bd] for side==1, puVar3[700]=0x2bc for side==2
   via decompile_function 0x004571E0 — STRUCT_FAMILY_CASCADE)
4. Sets `spy_owner_house+0x1FC` (ProductionChanged) = 1 to trigger sidebar rebuild.

### Other Spy Infiltration Effects

The same `OnSpyInfiltrate` function handles multiple effects based on building type flags:

| TypeClass Flag | Effect |
|----------------|--------|
| `+0x16A4` (ResetRadar) | Calls radar sabotage — EVA: "EVA_RadarSabotaged" |
| `+0xEB8 == 0x28` | Power sabotage — sets `house+0x2C0`, EVA: "EVA_EnemyBasePoweredDown" |
| `+0xEB8 == 0x10` | Radar sabotage — sets `house+0x2BF` |
| `+0x800 > 0` (CashBounty) | Cash steal — EVA: "EVA_CashStolen" |
| `+0xEE0 >= 1` | Routes to `HouseClass__SpyPowerSabotage` directly, bypassing tech/cash/weapon branches. (corrected 2026-05-29: was "+0xEE0 > 0 (StolenTechIndex) → Tech steal"; binary shows this field routes to SpyPowerSabotage, not tech steal, via decompile_function 0x004571E0 — OPERATOR_OR_ORDER_DRIFT) |
| `+0x16F0 != -1` (InfiltrateWeapon) | Special weapon activation |

### Acquired Tech (Per-RTTI Alliance Masks)

Separate from the 3 stolen tech bools, there are 4 "acquired tech" bitmasks on HouseClass
that provide per-RTTI-type cross-side building:

| House Offset | Applies to RTTI | Name |
|---|---|---|
| `+0x2C4` | 0x10 (Aircraft) | AlliedAcquiredTech |
| `+0x2C8` | 0x28 (Buildings) | SovietAcquiredTech |
| `+0x2CC` | 3 (Infantry) | ThirdAcquiredTech |
| `+0x2D0` | 7 (Vehicles) | FourthAcquiredTech |

These are bitmasks (like Owner). When set, they allow building types from the infiltrated
side even if the player's own side doesn't match the RequiredHouses mask.

### CanBuild Stolen Tech Check (Step 5)

```c
// Step 5: RequiresStolenTech check
if (type->RequiresStolenAlliedTech && !house->StolenAlliedTech)
    return 0;
if (type->RequiresStolenSovietTech && !house->StolenSovietTech)
    return 0;
if (type->RequiresStolenThirdTech && !house->StolenThirdTech)
    return 0;
```

**Confidence: HIGH** — Spy infiltration handler at 0x4571E0 confirmed with EVA string
references. Stolen tech offsets on HouseClass corrected from CanBuild at 0x4F7870 and
OnSpyInfiltrate at 0x4571E0: Allied=0x2BE, Soviet=0x2BD, Third=0x2BC.

---

## 5. BuildLimit System

### TechnoTypeClass Field

| Byte Offset | INI Key | Type | Default |
|-------------|---------|------|---------|
| `+0x3B8` | `BuildLimit=` | int | 0 |

### Semantics

| Value | Behavior |
|-------|----------|
| `0` | Unlimited (default) |
| `> 0` | Hard cap — once you own N copies, can't build more. Shows greyed in sidebar. |
| `< 0` | "Replaceable" cap — abs(value) is the cap, but if a unit dies, you can rebuild. Negative allows re-queuing even at the limit if one is in production. |

### CanBuild BuildLimit Check (Step 10)

```c
// Step 10 — BuildLimit check
int build_limit = type->BuildLimit;  // +0x3B8
if (build_limit == 0)
    goto unlimited;  // skip check

// Count owned instances — switch by RTTI type
int owned_count;
switch (rtti_type) {
    case INFANTRY:  owned_count = count_owned_infantry(house, type); break;
    case VEHICLE:   owned_count = count_owned_vehicles(house, type); break;
    case AIRCRAFT:  owned_count = count_owned_aircraft(house, type); break;
    case BUILDING:
        owned_count = count_owned_buildings(house, type);
        // Extra: if building has PowersUpBuilding (+0xEC6), also count
        // buildings with this type as an upgrade (checks upgrade slots
        // at obj+0x17B..+0x17D)
        break;
}

if (build_limit < 0) {
    int effective_limit = abs(build_limit);
    if (owned_count >= effective_limit)
        return 0;  // hard blocked
    return 1;  // can build (replacement allowed)
}

if (build_limit > 0) {
    if (owned_count >= build_limit) {
        // Check if one is currently in a factory
        if (allow_in_production_flag && type_is_in_factory(house, type))
            return 1;  // allow because one is actively being built
        return -1;  // at limit (shown greyed in sidebar)
    }
}

return 1;  // can build
```

The factory check uses `FactoryClass::GetObject` (`0x4CA160`) and
`FactoryClass::CountQueued` (`0x4CA670`) to determine if a copy is currently in
production.

**Confidence: HIGH** — BuildLimit logic fully decompiled from CanBuild at 0x4F7870.

---

## 6. Naval/Aircraft Factory Requirements

### How Naval Units Need a Shipyard

Naval units do NOT use a separate prerequisite keyword like "SHIPYARD". Instead, the
naval system works through the **factory classification system**:

1. Each TechnoTypeClass has a `Naval=` flag at `+0xCCE` (bool).
2. Each HouseClass has **separate factory pointers** for naval vs non-naval:
   - `+0x53BC` — VehicleFactory (non-naval)
   - `+0x53CC` — NavalFactory (naval, `Naval=yes` units)
   - `+0x53B4` — BuildingFactory (non-naval buildings)
   - `+0x53B8` — NavalBuildFactory (naval buildings)
3. When `HouseClass::Begin_Production` (`0x4FA350`) is called, it selects the factory
   pointer based on the type's Naval flag.
4. If the house has no naval factory building (e.g., GAYARD/NAYARD/YAYARD), the
   NavalFactory pointer at `+0x53CC` remains NULL, and production cannot start.

The prerequisite for a naval unit is typically `Prerequisite=FACTORY,GAYARD` or similar —
the shipyard is a direct building prerequisite, not a generic group.

### How Aircraft Need an Airfield/Helipad

Aircraft use the same mechanism:

1. `AircraftTypeClass+0xE0D` — `AirportBound=` (bool). When true, the aircraft requires
   a helipad/airfield pad to land on.
2. The prerequisite is expressed through `Prerequisite=` listing the airfield building
   (e.g., `Prerequisite=GAAIRC` for Allied aircraft).
3. The house's AircraftFactory pointer at `+0x53B0` is set when a building with
   `Factory=AircraftType` is owned.
4. `Helipad=` (`BuildingTypeClass+0x16B8` / `+0x16CB`) marks buildings as aircraft docks.

**Key insight:** There is no magic "naval" or "aircraft" prerequisite group. The engine
uses two parallel systems:
- **Prerequisites** (the tech tree) determine if the item appears buildable in the sidebar
- **Factory pointers** (the production system) determine which physical factory building
  produces the unit — separate slots for infantry, vehicles, naval vehicles, aircraft,
  buildings, and naval buildings

**Confidence: HIGH** — Factory pointer offsets confirmed from 6+ functions in
HouseClass. Naval flag at +0xCCE confirmed from TechnoTypeClass::ReadINI.

---

## 7. The Spawnable Flag (TechnoTypeClass+0x6D5)

### INI Key

`AllowedToStartInMultiplayer=` (parsed in TechnoTypeClass::ReadINI at `0x00712170`)

### Storage

`TechnoTypeClass+0x6D5` — 1 byte (bool)

### Purpose

The Spawnable flag controls **exclusively** whether a type is eligible for the random
starting unit pool at game start. It has **no effect** on any other system (production,
sidebar, AI, etc.).

### How It's Used: Generate_Random_Units (0x006886B0)

At game start, the engine builds a pool of valid starting unit types. For each infantry
and vehicle type in the global arrays:

```c
if (type->Spawnable                          // +0x6D5 != 0
    && type->TechLevel <= house->TechLevel   // +0x634 <= house+0x1D4
    && (type->HouseMask & houseMask) != 0)   // +0x6CC & (1 << country_index)
{
    // Type is eligible for the starting unit pool
    candidateList.Add(type);
}
```

The starting unit budget is calculated as:
```
avgCost = total_cost_of_all_spawnable_types / count_of_spawnable_types
totalBudget = avgCost * unitCount
```

Where `unitCount` comes from the multiplayer dialog setting (minus 1 if Bases=yes, since
the MCV counts as one unit).

Units are placed with a 2/3 infantry, 1/3 vehicle split from the filtered pool.

### What Spawnable Does NOT Do

- Does NOT affect the sidebar or production system
- Does NOT affect AI build decisions
- Does NOT affect campaign/trigger unit creation
- Is NOT related to spawn-missile or spawn-aircraft systems

MCVs (AMCV/SMCV/YMCV) have `AllowedToStartInMultiplayer=no` — they are handled
separately via the `BaseUnit=` list in `[General]`.

**Confidence: HIGH** — Traced from Generate_Random_Units at 0x6886B0. The field at
+0x6D5 is only referenced in this starting unit generation context.

---

## 8. AIBasePlanningSide (TechnoTypeClass+0x6D0)

### INI Key

`AIBasePlanningSide=` (int, default -1)

### Storage

`TechnoTypeClass+0x6D0` — 4 bytes (int)

### Purpose

This field maps a type to a specific side for the AI's base planning system. Values:
- `-1` = not side-restricted (default)
- `0` = Allied (GDI)
- `1` = Soviet (Nod)
- `2` = Third/Yuri (ThirdSide)

### How the AI Uses It

**AI_RecalcBuildOptions** (`0x005054B0`, 2755 bytes) iterates ALL BuildingTypes and
filters them for the AI's buildable list:

```c
// For each BuildingType in the global array:
if ((type->Owner & (1 << country_index)) == 0)
    continue;  // Wrong side via Owner bitmask (+0x6CC)

if (type->AIBasePlanningSide != -1
    && type->AIBasePlanningSide != house->CountryTypeClass->SideIndex)
    continue;  // Wrong side for AI base planning (+0x6D0 vs CountryType+0xBC)

if (type->TechLevel > house->TechLevel)
    continue;  // Too high tech level

// Check RequiredHouses/ForbiddenHouses...
// Add to buildable list
```

### Also Used in Spy Infiltration

The `AIBasePlanningSide` field at `+0x6D0` is also read by the spy infiltration handler
(`BuildingClass::OnSpyInfiltrate` at `0x4571E0`) to determine which stolen tech flag
to set:
- `+0x6D0 == 0` → set StolenAlliedTech
- `+0x6D0 == 1` → set StolenSovietTech
- `+0x6D0 == 2` → set StolenThirdTech

### Not Used for Human Players

The `AIBasePlanningSide` check is ONLY in the AI build options recalculator. Human
players' sidebar uses `Owner=`, `RequiredHouses=`, and `ForbiddenHouses=` bitmask checks
in `HouseClass::CanBuild` without referencing `AIBasePlanningSide`.

**Confidence: HIGH** — Verified from AI_RecalcBuildOptions at 0x5054B0 and spy
infiltration handler at 0x4571E0.

---

## 9. Complete HouseClass::CanBuild Flow (0x4F7870)

### Parameters

```c
int HouseClass::CanBuild(
    TechnoTypeClass* type,   // what we want to build
    int skip_prereqs,        // if nonzero, jump to BuildLimit check only
    int allow_in_production  // for BuildLimit edge cases
)
```

### Return Values

| Return | Meaning |
|--------|---------|
| `1` | Can build |
| `0` | Cannot build (prerequisites not met, forbidden, etc.) |
| `-1` | At BuildLimit, but one is in production (show greyed) |

### Step-by-Step Check Order

```
STEP 1: NotBuildable check
    if type+0xC98 (NotBuildable flag) is set:
        return 0

STEP 2: PrerequisiteOverride check
    if PrerequisiteOverride list (+0x654) is non-empty:
        for each type_index in PrerequisiteOverride:
            if house owns >= 1 of this building type:
                SKIP to Step 10 (BuildLimit check)
        // none matched, fall through

STEP 3: Direct ownership check (for auto-appearing types)

STEP 4: TechLevel check
    if type.TechLevel (+0x634) == -1: return 0
    if type.TechLevel > house.TechLevel (+0x1D4): return 0

STEP 5: RequiresStolenTech check
    if type.RequiresStolenAlliedTech (+0xD9D) AND NOT house+0x2BE: return 0
    if type.RequiresStolenSovietTech (+0xD9C) AND NOT house+0x2BD: return 0
    if type.RequiresStolenThirdTech  (+0xD9B) AND NOT house+0x2BC: return 0
    (corrected 2026-05-29: ThirdTech was house+0x2BF; binary CanBuild 0x4F7870 shows
    field_0x2bc for RequiresStolenThirdTech (+0xD9B) via decompile_function 0x004F7870 — STRUCT_FAMILY_CASCADE)

STEP 6: RequiredHouses check
    mask = type.RequiredHouses (+0xDA0)
    if mask != -1 (not "all houses"):
        side_bit = 1 << house.CountryType.SelfIndex (+0xB8)

        // Primary check: is our country bit in the required mask?
        ok = (mask & side_bit) != 0

        // Extended checks per RTTI type (acquired tech from spying):
        if !ok:
            switch(rtti):
                Aircraft (0x10): ok |= (house+0x2C4 & mask) != 0
                Buildings (0x28): ok |= (house+0x2C8 & mask) != 0
                Infantry (3):    ok |= (house+0x2CC & mask) != 0
                Vehicles (7):    ok |= (house+0x2D0 & mask) != 0

        if !ok: return 0

STEP 7: ForbiddenHouses check
    forbidden = type.ForbiddenHouses (+0xDA4)
    if forbidden != -1:
        if (forbidden & side_bit) != 0: return 0

STEP 8: AI shortcut
    if house is AI (not human, not multiplayer):
        return 1  // AI skips prerequisite checking

STEP 9: Prerequisite satisfaction (THE TECH TREE)
    Read Prerequisite vector (+0x638)

    for each prereq_id:
        Negative IDs → generic group check (POWER/FACTORY/BARRACKS/RADAR/TECH/PROC)
        Non-negative IDs → specific building ownership check
        Upgrade buildings → check upgrade slots on owned buildings

    if ANY prerequisite fails: return 0

STEP 10: BuildLimit check
    (see Section 5 above)
```

---

## 10. Key Addresses Summary

| Address | Function | Purpose |
|---------|----------|---------|
| `0x00475260` | ReadOwnerBitmask | Parse Owner=/RequiredHouses=/ForbiddenHouses= from INI |
| `0x004750D0` | ReadFlagBitmask | Parse comma-separated flag lists (for Prerequisite=) |
| `0x004770E0` | ParsePrerequisite | Convert prerequisite keywords to negative IDs |
| `0x0050C170` | CountryTypeClass::FindIndex | Country name → bit index (0-31) |
| `0x005117D0` | CountryTypeClass::FindByName | Country name → CountryTypeClass* |
| `0x00512680` | CountryTypeClass::FindOrCreate | Find or allocate new country |
| `0x00672440` | [Sides] Registration | Links countries to sides, sets SideIndex |
| `0x004F7870` | HouseClass::CanBuild | Full build eligibility check (2804 bytes) |
| `0x0066E400` | RulesClass::ReadGeneral | Reads prerequisite groups from [General] |
| `0x00712170` | TechnoTypeClass::ReadINI | Master INI parser (16,337 bytes) |
| `0x004571E0` | BuildingClass::OnSpyInfiltrate | Spy infiltration handler (~965 bytes) |
| `0x0049FAE0` | CountOwnedOfType | Count owned buildings of a specific type |
| `0x0050B370` | CheckBuildLimit | Build limit enforcement helper |
| `0x006886B0` | Generate_Random_Units | Starting unit generation (uses Spawnable) |
| `0x005054B0` | AI_RecalcBuildOptions | AI buildable list (uses AIBasePlanningSide) |
| `0x004FA350` | HouseClass::Begin_Production | Start producing (selects factory by Naval flag) |
| `0x004FB0E0` | HouseClass::Place_Production | Place completed production |

---

## Confidence Summary

| Topic | Confidence | Basis |
|-------|-----------|-------|
| Owner bitmask parsing (FUN_00475260) | HIGH | Decompiled with clear tokenize+shift pattern |
| Country index mapping (0-31 bits) | HIGH | CountryTypeClass::FindByName has 23 callers |
| TechLevel storage and check | HIGH | Confirmed from ReadScenarioINI + CanBuild |
| Prerequisite groups (6 keywords) | HIGH | String constants + RulesClass offsets verified |
| PrerequisiteOverride OR logic | HIGH | Clear skip-to-BuildLimit in CanBuild decompilation |
| Stolen tech (3 bools + spy handler) | HIGH | EVA string refs confirm spy infiltration handler |
| Acquired tech (4 per-RTTI masks) | HIGH | Verified from CanBuild RequiredHouses extended check |
| BuildLimit semantics (0/+/-) | HIGH | Full decompilation of CanBuild bottom half |
| Naval factory pointer separation | HIGH | 6+ functions use the same factory slot switch |
| Spawnable flag purpose | HIGH | Only referenced in Generate_Random_Units |
| AIBasePlanningSide purpose | HIGH | Referenced in AI_RecalcBuildOptions + spy handler |
| RTTI mapping for acquired tech | MEDIUM | RTTI values 0x10/0x28/3/7 match expected but naming could differ |
| StolenTechIndex field at TypeClass+0xEE0 | MEDIUM | Decompiled from spy handler but write site not fully traced |
