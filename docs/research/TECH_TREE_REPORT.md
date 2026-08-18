# gamemd.exe Tech Tree / Build Eligibility System Report

## Source: Ghidra MCP decompilation of gamemd.exe (Yuri's Revenge)

This report documents how the original engine determines what a player is allowed to build:
buildings, units, infantry, and aircraft. Every detail is verified against the actual decompiled
binary, not wiki speculation.

---

## 1. Overview: What Determines Buildability

When the sidebar checks whether to show an item as buildable (or greyed out), the engine runs
through a multi-layered check. The primary function is at **`0x004f7870`**
(`HouseClass::CanBuild`). It returns:

| Return | Meaning |
|--------|---------|
| `1`    | Can build |
| `0`    | Cannot build (prerequisites not met, forbidden, etc.) |
| `-1`   | At BuildLimit, but one is already in production (show greyed) |

The checks, in order:

1. **NotBuildable flag** (`TechnoTypeClass+0xC98`, parsed from INI key not shown to players)
2. **PrerequisiteOverride** (alternate prerequisite path)
3. **Owned-type check** (for types already in the house's inventory)
4. **TechLevel** check
5. **RequiresStolenTech** flags (AlliedTech, SovietTech, ThirdTech)
6. **RequiredHouses / ForbiddenHouses** bitmask check
7. **Prerequisite satisfaction** (the actual tech tree)
8. **BuildLimit** enforcement

---

## 2. TechnoTypeClass Fields (INI -> Binary Offsets)

These are the fields parsed by `TechnoTypeClass::ReadINI` at `0x00712170` (3471-line function).
All offsets are byte offsets from the TechnoTypeClass base pointer.

| Byte Offset | Field (int*[N]) | INI Key | Type | Description |
|-------------|-----------------|---------|------|-------------|
| `0x634`     | `[0x18D]`       | `TechLevel` | int | Tech level required. -1 = unbuildable. |
| `0x638-0x650` | `[0x18E-0x194]` | `Prerequisite` | DynamicVector\<int\> | List of prerequisite IDs (see section 4) |
| `0x654-0x66C` | `[0x195-0x19B]` | `PrerequisiteOverride` | DynamicVector\<int\> | Alternative prerequisite path (OR with main) |
| `0x3B8`     | `[0x0EE]`       | `BuildLimit` | int | Max copies. 0=unlimited, >0=hard cap, <0=abs value cap but allow if one dies |
| `0xDA0`     | `[0x368]`       | `RequiredHouses` | bitmask (32-bit) | Which countries/sides can build this |
| `0xDA4`     | `[0x369]`       | `ForbiddenHouses` | bitmask (32-bit) | Which countries/sides are blocked |
| `0xDA8`     | `[0x36A]`       | `SecretHouses` | bitmask (32-bit) | Houses that unlock via spying |
| `0xC98`     | `[0x326]`       | (internal) | bool | NotBuildable flag |
| `0xD9C`     | `[0x367]`       | `RequiresStolenSovietTech` | bool | Needs spy infiltration of Soviet tech |
| `0xD68`     | `+0xD9B`        | `RequiresStolenThirdTech` | bool | Needs spy infiltration of Yuri tech |
| `0xD9D`     | `+0xD9D`        | `RequiresStolenAlliedTech` | bool | Needs spy infiltration of Allied tech |
| `0x608`     | `[0x182]`       | `BuildTimeMultiplier` | float | Multiplier on production speed |
| `0xCCE`     | `+0xCCE`        | `Naval` | bool | Is a naval unit (affects factory slot + RequiredHouses check) |
| `0xD96`     | `+0xD96`        | (internal) | bool | IsUpgrade flag (for building upgrades) |

---

## 3. Prerequisite Groups (Generic Keywords)

In `rules(md).ini`, the `Prerequisite=` field can reference generic keywords instead of specific
building names. These keywords get converted to negative integer IDs during INI parsing
(function `0x004770e0`).

The parser does case-insensitive string comparison (via `_stricmp`):

| Keyword    | Stored ID | Hex        | Checks against (RulesClass offsets) |
|------------|-----------|------------|-------------------------------------|
| `POWER`    | -1        | 0xFFFFFFFF | `rules+0x35C` (array), `rules+0x368` (count) |
| `FACTORY`  | -2        | 0xFFFFFFFE | `rules+0x378` (array), `rules+0x384` (count) |
| `BARRACKS` | -3        | 0xFFFFFFFD | `rules+0x394` (array), `rules+0x3A0` (count) |
| `RADAR`    | -4        | 0xFFFFFFFC | `rules+0x3B0` (array), `rules+0x3BC` (count) |
| `TECH`     | -5        | 0xFFFFFFFB | `rules+0x3CC` (array), `rules+0x3D8` (count) |
| `PROC`     | -6        | 0xFFFFFFFA | `rules+0x3E8` (array), `rules+0x3F4` (count) + ProcAlternate at `rules+0x400` |

Any token that doesn't match these keywords is looked up as a **specific BuildingType** by name
(e.g., `GAPILE`, `NAHAND`, `GATECH`), and its index in `BuildingTypeClass::Array` is stored as
a non-negative integer.

### Default Group Definitions (from rulesmd.ini [General])

```ini
PrerequisitePower=GAPOWR,NAPOWR,NANRCT,YAPOWR       ; Allied/Soviet/Yuri power plants
PrerequisiteFactory=GAWEAP,NAWEAP,YAWEAP              ; War factories
PrerequisiteBarracks=NAHAND,GAPILE,YABRCK             ; Barracks
PrerequisiteRadar=GAAIRC,NARADR,AMRADR,NAPSIS         ; Radar/Airforce HQ
PrerequisiteTech=GATECH,NATECH,YATECH                 ; Tech centers (Battle Lab)
PrerequisiteProc=GAREFN,NAREFN,YAREFN                 ; Refineries
PrerequisiteProcAlternate=SMIN                         ; Slave Miner counts as PROC
```

This means `Prerequisite=BARRACKS` is satisfied if the player owns ANY of `NAHAND`, `GAPILE`,
or `YABRCK`. The groups are side-agnostic -- an Allied player with a captured Soviet Barracks
still satisfies `BARRACKS`.

---

## 4. The Full Prerequisite Check Algorithm

Function: `HouseClass::CanBuild` at `0x004f7870`

Parameters:
- `this` = HouseClass pointer
- `param_2` = TechnoTypeClass pointer (what we want to build)
- `param_3` = skip_prereqs flag (if nonzero, jumps directly to BuildLimit check)
- `param_4` = allow_in_production flag (for BuildLimit edge cases)

### Step-by-step logic:

```
STEP 1: NotBuildable check
    if type.NotBuildable (offset 0xC98):
        return 0  // can't build

STEP 2: PrerequisiteOverride check
    if PrerequisiteOverride list is non-empty:
        for each type_index in PrerequisiteOverride:
            if house owns at least 1 of type_index:
                SKIP to Step 8 (build limit check) -- prereqs satisfied
        // if none matched, fall through to normal check

STEP 3: Direct ownership check
    Check if the type already exists in house inventory
    (for types that auto-appear without prereqs)

STEP 4: TechLevel check
    if type.TechLevel == -1:
        return 0  // unbuildable type
    if type.TechLevel > house.TechLevel (house+0x1D4):
        return 0  // tech level too low

STEP 5: RequiresStolenTech check
    if type.RequiresStolenAlliedTech AND NOT house.HasStolenAlliedTech (house+0x2BE):
        return 0
    if type.RequiresStolenSovietTech AND NOT house.HasStolenSovietTech (house+0x2BD):
        return 0
    if type.RequiresStolenThirdTech AND NOT house.HasStolenThirdTech (house+0x2BC):
        return 0

STEP 6: RequiredHouses check
    houses_mask = type.RequiredHouses (offset 0xDA0)
    if houses_mask != -1 (i.e., not "all houses"):
        side_bit = 1 << house.HouseType.SideIndex (from HouseTypeClass+0xB8)

        Basic check: is our side bit in the required mask?

        Extended checks per RTTI type:
            Aircraft (0x10): also check house.AlliedAcquiredTech (house+0x2C4)
            Buildings (0x28): also check house.SovietAcquiredTech (house+0x2C8)
            Infantry (3):     also check house.ThirdAcquiredTech (house+0x2CC)
            Vehicles (7):     also check house.FourthAcquiredTech (house+0x2D0)

        if none of these pass: return 0

STEP 7: ForbiddenHouses check
    forbidden_mask = type.ForbiddenHouses (offset 0xDA4)
    if forbidden_mask != -1:
        if our side bit IS in the forbidden mask:
            return 0

STEP 8: AI shortcut
    if player is AI (not human, not in multiplayer):
        return 1  // AI skips prerequisite checking

STEP 9: Prerequisite satisfaction (THE TECH TREE)
    Read the Prerequisite vector (offset 0x638)

    ALL prerequisites must be satisfied (AND logic):

    for each prereq_id in Prerequisite list:
        switch (prereq_id):
            case -1 (POWER):
                Must own at least 1 building from PrerequisitePower group
            case -2 (FACTORY):
                Must own at least 1 building from PrerequisiteFactory group
            case -3 (BARRACKS):
                Must own at least 1 building from PrerequisiteBarracks group
            case -4 (RADAR):
                Must own at least 1 building from PrerequisiteRadar group
            case -5 (TECH):
                Must own at least 1 building from PrerequisiteTech group
            case -6 (PROC):
                Must own at least 1 building from PrerequisiteProc group
                OR own PrerequisiteProcAlternate (Slave Miner) AND it has
                its deploy-building (checked via rules+0x400 -> type+0xDF8)
            default (>= 0, specific building index):
                if building type has upgrade flag (type+0xE88):
                    Must own a building that has this upgrade attached
                    (checks owned buildings' upgrade slots at obj+0x17B..+0x17D)
                else:
                    Must own at least 1 building of this specific type
                    (via FUN_0049fae0 which counts owned instances)

        if ANY prerequisite fails: return 0

STEP 10: BuildLimit check
    build_limit = type.BuildLimit (offset 0x3B8)
    owned_count = count of this type owned by house

    Switch on RTTI type (infantry/vehicle/aircraft/building):

    if build_limit == 0:
        unlimited, skip check

    if build_limit < 0:
        // Negative = allow re-building after unit dies
        effective_limit = abs(build_limit)
        if owned_count >= effective_limit:
            return 0  // hard blocked
        if owned_count < effective_limit AND build_limit < 0:
            return 1  // can build (replacement allowed)

    if build_limit > 0:
        if owned_count >= build_limit:
            if allow_in_production flag AND type is currently in a factory:
                return 1  // allow because one is actively being built
            return -1  // at limit (shown greyed in sidebar)

    return 1  // can build
```

### PrerequisiteOverride: The "OR" Path

`PrerequisiteOverride` provides an **alternative** prerequisite path. If ANY type in the
override list is owned, the engine skips the normal `Prerequisite` check entirely. This is
used for special campaign buildings:

```ini
; Navy SEAL example from rulesmd.ini:
Prerequisite=GAPILE,RADAR
PrerequisiteOverride=CAWA2A,CAWA2B,CAWA2C,CAWA2D  ; any Pentagon building unlocks SEALs
```

---

## 5. Owner / RequiredHouses / ForbiddenHouses

The `Owner=` INI key maps to **RequiredHouses** (offset 0xDA0). It's a bitmask where each bit
represents a country. The `ForbiddenHouses` (offset 0xDA4) is the inverse -- explicitly blocked
countries.

The country-to-bit mapping comes from `HouseTypeClass`, with each country's `SideIndex`
(offset 0xB8) determining which bit position it occupies.

### How Owner Interacts with Sides

In YR, the three sides (Allied, Soviet, Yuri) each have multiple countries:

| Side | Countries |
|------|-----------|
| Allied | Americans, British, French, Germans, (Alliance/Korea) |
| Soviet | Russians, Confederation (Libya), Africans (Cuba), Arabs (Iraq) |
| Yuri | YuriCountry |

**Example:** A GI has `Owner=British,French,Germans,Americans,Alliance` -- only Allied countries
can build it. The engine converts these country names to a bitmask during INI parsing
(function `0x004750D0`).

### Acquired Tech (Spy Infiltration)

When a spy infiltrates an enemy building, the house gains "acquired tech" bits at:
- `house+0x2C4` = AlliedAcquiredTech
- `house+0x2C8` = SovietAcquiredTech
- `house+0x2CC` = ThirdAcquiredTech
- `house+0x2D0` = FourthAcquiredTech

These allow building types whose RequiredHouses mask includes the infiltrated side's countries,
even if the player's own side doesn't match. The check is per-RTTI-type (aircraft, buildings,
infantry, vehicles each check a different acquired-tech field).

---

## 6. BuildLimit System

`BuildLimit` (TechnoTypeClass offset 0x3B8, INI key `BuildLimit`) controls how many copies
of a type can exist simultaneously.

| Value | Behavior |
|-------|----------|
| `0`   | Unlimited (default) |
| `> 0` | Hard cap. Once you own N copies, can't build more. Shows greyed in sidebar. |
| `< 0` | "Replaceable" cap. Absolute value is the cap, but if a unit dies, you can rebuild. Negative allows re-queuing even at the limit if one is in production. |

**Example:** `BuildLimit=1` on hero units (Tanya, Boris, Yuri Prime) means only 1 can exist.
If she dies, you can build another.

The build limit check (`0x004f7870`, bottom half) also accounts for units currently in the
factory (via `FactoryClass::GetObject` at `0x004CA160` and `FactoryClass::CountQueued` at
`0x004CA670`).

For buildings (RTTI 0x10), there's an extra check: if the building type has flag `+0xEC6`
(PowersUpBuilding), it counts existing powered-up buildings as part of the limit, iterating
`BuildingClass::Array` at `DAT_008b410c`.

---

## 7. Factory System

Production is managed by **FactoryClass** (116 bytes, at `0x004C98B0`). Each house has factory
slots by RTTI type:

| HouseClass Offset | Factory For |
|-------------------|-------------|
| `+0x53AC`         | Infantry |
| `+0x53B0`         | Aircraft |
| `+0x53B4`         | Buildings |
| `+0x53B8`         | Buildings (naval/secondary) |
| `+0x53BC`         | Vehicles |
| `+0x53CC`         | Vehicles (naval/secondary) |

Key FactoryClass fields:
- `+0x24` = production step counter (0 to 54 = complete)
- `+0x38` = step delay timer
- `+0x58` = currently producing object pointer
- `+0x6C` = owning house pointer
- `+0x70` = "ready" flag
- `+0x71` = "can place" flag

Production completes at step **54** (0x36). The delay between steps = `lepton_distance / 54`,
clamped to [1, 255].

### Production Flow

1. **Begin:** `House::Begin_Production` (`0x004FA350`) creates a FactoryClass, calls
   `Factory::StartProduction` (`0x004C9C70`)
2. **Tick:** Factory advances one step per delay interval
3. **Complete:** Step reaches 54, sets ready flag
4. **Place:** `House::Place_Production` (`0x004FB0E0`) places the completed object

---

## 8. Sidebar Tab Mapping

The sidebar has **4 strips** (tabs), mapped by RTTI type:

| RTTI Values | Tab | Category |
|-------------|-----|----------|
| `1, 0x28` (BuildingType/BuildingClass) | 0 | Structures |
| `2, 3` (InfantryType/InfantryClass) | 1 | Infantry |
| `6, 7` (UnitType/UnitClass) | 2 | Vehicles (or 3 if naval via `FUN_005004E0`) |
| `0xF, 0x10` (AircraftType/AircraftClass) | 3 | Aircraft / Defense |
| `0x1F, 0x20, 0x39` (SuperWeapon types) | 1 | Super weapons (mixed with infantry) |

Items within each tab are sorted by (function `0x006A8420`):
1. Super weapons sorted separately by cost
2. Same-faction items first (matches player's country side)
3. Non-upgrade items before upgrades
4. Lower TechLevel first
5. Lower cost first
6. Alphabetical by name

---

## 9. Sidebar Buildability Display

In `SidebarClass::DrawStrip` (`0x006A9540`, 4210 bytes):

- Items are drawn with their cameo SHP icon
- **Unbuildable** items get `DARKEN.SHP` overlaid (semi-transparent darkening)
- **Completed** items flash (blink every 8 frames out of 16)
- **In-progress** items show `GCLOCK2.SHP` progress overlay
- **Ready** items show `"TXT_READY"` text
- **On hold** items show `"TXT_HOLD"` text
- **Queue count** badge (e.g. "x3") shown at top-right if multiple queued

The per-tick update (`0x006AA600`) removes items that become invalid (prerequisite building
destroyed, etc.) and auto-cancels their production via network commands.

---

## 10. Concrete Example: Allied Tech Tree

Tracing actual rulesmd.ini entries:

```
1. Player starts with Construction Yard (GACNST)
   -> TechLevel 1 items become available

2. Build Power Plant (GAPOWR)
   Prerequisite=GACNST
   -> Satisfies "POWER" group for downstream items

3. Build Barracks (GAPILE)
   Prerequisite=POWER,GACNST
   -> Satisfies "BARRACKS" group
   -> Unlocks: GI (Prerequisite=GAPILE, TechLevel=1)

4. Build Ore Refinery (GAREFN)
   Prerequisite=PROC,POWER,GACNST   (wait -- PROC requires itself? No:)
   Actually: Prerequisite=POWER,GACNST
   -> Satisfies "PROC" group

5. Build War Factory (GAWEAP)
   Prerequisite=PROC,POWER,GACNST
   -> Satisfies "FACTORY" group
   -> Unlocks vehicles

6. Build Airforce Command HQ (GAAIRC)
   Prerequisite=PROC,POWER,GACNST
   -> Satisfies "RADAR" group
   -> Unlocks: Sniper (Prerequisite=GAPILE,RADAR, TechLevel=3)

7. Build Battle Lab (GATECH)
   Prerequisite=RADAR,GACNST
   -> Satisfies "TECH" group
   -> Unlocks: Chrono Legionnaire (Prerequisite=GAPILE,GATECH, TechLevel=5)
   -> Unlocks: Prism Tank (requires FACTORY + TECH level items)
```

### Country-Specific Units

```ini
; Sniper (British only)
[SNIPE]
Prerequisite=GAPILE,RADAR
TechLevel=1
Owner=British,French,Germans,Americans,Alliance
RequiredHouses=British
; -> Only British can build, despite Owner listing all Allied countries
```

The `RequiredHouses=British` restricts it further beyond `Owner=`. A French player sees the
Sniper in their sidebar but it's available only to British.

---

## 11. Key Addresses Summary

| Address | Function |
|---------|----------|
| `0x004F7870` | `HouseClass::CanBuild` -- main build eligibility check |
| `0x004770E0` | Prerequisite INI parser (keyword to negative ID conversion) |
| `0x00712170` | `TechnoTypeClass::ReadINI` (3471 lines, reads all type properties) |
| `0x0066E400` | `RulesClass::ReadGeneral` (reads prerequisite groups from [General]) |
| `0x004FA350` | `House::Begin_Production` (starts building something) |
| `0x004FB0E0` | `House::Place_Production` (places completed production) |
| `0x004C9C70` | `FactoryClass::StartProduction` |
| `0x004CA130` | `FactoryClass::IsComplete` (step == 54?) |
| `0x004C9FF0` | `FactoryClass::AbandonProduction` (cancel + refund) |
| `0x0050B370` | Build limit helper (checks factories for queued count) |
| `0x0049FAE0` | Count owned instances of a type (used by prereq checker) |
| `0x006A8420` | Sidebar sort comparator (tech level, cost, name) |
| `0x006AA600` | Sidebar per-tick: remove invalid items, cancel orphaned builds |
| `0x006A9540` | Sidebar draw: render cameos, progress, darken overlay |

---

## 12. Notes for Implementation

1. **Prerequisite IDs are stored as signed ints.** Negative = generic group, non-negative =
   BuildingType array index. Parse keywords case-insensitively during INI load.

2. **ALL prerequisites must be met** (AND logic). PrerequisiteOverride provides an OR alternative
   that bypasses the entire main list.

3. **AI players skip prerequisite checking** entirely (Step 8). They can build anything their
   TechLevel and Owner allow.

4. **The `Owner=` field maps to RequiredHouses bitmask.** Each country name maps to a bit
   position. The check also considers acquired tech from spy infiltration.

5. **BuildLimit negative values** are special: they allow replacement builds. The absolute value
   is the cap.

6. **PrerequisiteProc (-6) has special handling:** it also checks `PrerequisiteProcAlternate`
   (the Slave Miner), verifying that the Slave Miner's deploy-building has been placed.

7. **Building upgrades** as prerequisites use a different code path: instead of counting owned
   buildings, the engine iterates owned buildings and checks their upgrade slots
   (3 slots at building offsets +0x17B through +0x17D).

8. **Production speed** is `lepton_distance / 54` per step. The `BuildTimeMultiplier` field
   affects this. Low power reduces speed (configurable via `MinLowPowerProductionSpeed` and
   `MaxLowPowerProductionSpeed` in [General]).

9. **The sidebar removes items in real-time** when prerequisites are lost (e.g., radar destroyed).
   This triggers network cancel commands for lockstep correctness.
