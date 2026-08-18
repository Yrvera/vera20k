# CountryTypeClass and SideTypeClass -- Deep Dive Research Report

**Date:** 2026-03-22
**Source:** Ghidra decompilation of gamemd.exe + existing reports + decompiled C files
**Confidence:** HIGH for struct layouts and field offsets (verified from decompiled constructor + ReadINI).
MEDIUM for some field semantics where names are inferred from context.

**NAMING NOTE (corrected 2026-05-29):** The INI section is `[Countries]` / `Country`, but the C++ class
in the binary is `HouseTypeClass` — not `CountryTypeClass`. Ghidra labels all functions at the addresses
below as `HouseTypeClass__*` (confirmed via `get_function_by_address` on each entry point). The name
`CountryTypeClass` does not appear in the binary's RTTI or labels. This document retains the INI-facing
name `CountryTypeClass` for readability, but every `CountryTypeClass::*` symbol maps to `HouseTypeClass::*`
in the binary (ROOT_CAUSE: RTTI_LABEL_DRIFT).

---

## Part 1: CountryTypeClass

### Overview

CountryTypeClass is the per-faction type record in YR. Each entry in `[Countries]` creates one
CountryTypeClass instance (size **0x1B0 = 432 bytes**). It stores all per-country INI keys:
stat multipliers, suffix/prefix for asset naming, veteran unit lists, color scheme, and a
parent-country reference.

### Key Addresses

| Symbol | Address | Purpose |
|--------|---------|---------|
| CountryTypeClass::Constructor | `0x005113f0` | 600 bytes, initializes all defaults |
| CountryTypeClass::ReadINI | `0x00511850` | 2325 bytes, full INI field parsing |
| CountryTypeClass::FindByName | `0x005117d0` | 116 bytes, 24 callers. Returns self-index or -1 | (corrected 2026-05-29: was 23 callers; `get_function_callers 0x005117d0` returned 24 entries — ROOT_CAUSE: INFERENCE_HARDENED)
| CountryTypeClass::FindOrCreate | `0x00512680` | 131 bytes, allocates 0x1B0 if not found | (binary label: `HouseTypeClass__FindOrAllocate`; confirmed via `get_function_by_address 0x00512680`)
| CountryTypeClass::WriteINI | `0x00512170` | 265 bytes, saves to stream |
| CountryTypeClass::CopyConstructor | `0x00511650` | 76 bytes |
| Global array pointer | `DAT_00a83c9c` | `int*` -- array of CountryTypeClass pointers |
| Global array count | `DAT_00a83ca8` | `int` -- number of registered countries |
| Global array capacity | `DAT_00a83ca0` | `int` -- allocated capacity |

### Struct Layout (0x1B0 bytes)

**IMPORTANT:** param_1 in the constructor is typed as `undefined4 *` (4-byte pointer), so
field indices like `param_1[0x2d]` are byte offset = `0x2d * 4 = 0xB4`. However, the ReadINI
function uses `int param_1` (byte offsets directly). The report below uses BYTE OFFSETS
consistently.

#### Base Class (AbstractTypeClass) -- offsets 0x00..0x97

| Offset | Size | Type | Field |
|--------|------|------|-------|
| +0x00 | 16 | ptr[4] | 4 vtable pointers (multi-inheritance) |
| +0x24 | 64 | char[64] | **ID** -- section name / primary identifier (e.g. "Americans") |
| +0x64 | 52 | char[52] | **UIName** -- display name / alternate name (e.g. from CSF string) |

#### CountryTypeClass-specific fields

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| +0x98 | 25 | char[25] | `ParentCountry=` | same as own name | 24 chars + null, parent country name |
| +0xB4 | 4 | int | -- | (computed) | Self-index in global array (from constructor search) |
| +0xB8 | 4 | int | -- | (computed) | Self-index (secondary, from 2nd search loop) |
| +0xBC | 4 | int | `Side=` | -1 | Side index. Written by BOTH [Sides] registration (FUN_00672440) AND ReadINI (via FUN_004756f0 at end of parse). (corrected 2026-05-29: was "set by [Sides] parser, NOT by ReadINI"; `decompile_function 0x00511850` shows FUN_004756f0 writing to param_1+0xBC after veteran lists — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT) |
| +0xC0 | 4 | int | `Color=` | -1 | Color scheme index |
| +0xC8 | 8 | double | `Firepower=` | 1.0 | Firepower multiplier |
| +0xD0 | 8 | double | `Groundspeed=` | 1.0 | Ground speed multiplier |
| +0xD8 | 8 | double | `Airspeed=` | 1.0 | Air speed multiplier |
| +0xE0 | 8 | double | `Armor=` | 1.0 | Armor multiplier |
| +0xE8 | 8 | double | (unnamed 5) | 1.0 | Likely ROF or Cost global mult |
| +0xF0 | 8 | double | (unnamed 6) | 1.0 | Likely another global mult |
| +0xF8 | 8 | double | `BuildTime=` | 1.0 | Build time multiplier |
| +0x100 | 4 | float | `ArmorInfantryMult=` | 1.0 | Per-category armor multiplier |
| +0x104 | 4 | float | `ArmorUnitsMult=` | 1.0 | |
| +0x108 | 4 | float | `ArmorAircraftMult=` | 1.0 | |
| +0x10C | 4 | float | `ArmorBuildingsMult=` | 1.0 | |
| +0x110 | 4 | float | `ArmorDefensesMult=` | 1.0 | |
| +0x114 | 4 | float | `CostInfantryMult=` | 1.0 | Per-category cost multiplier |
| +0x118 | 4 | float | `CostUnitsMult=` | 1.0 | |
| +0x11C | 4 | float | `CostAircraftMult=` | 1.0 | |
| +0x120 | 4 | float | `CostBuildingsMult=` | 1.0 | |
| +0x124 | 4 | float | `CostDefensesMult=` | 1.0 | |
| +0x128 | 4 | float | `SpeedInfantryMult=` | 1.0 | Per-category speed multiplier |
| +0x12C | 4 | float | `SpeedUnitsMult=` | 1.0 | |
| +0x130 | 4 | float | `SpeedAircraftMult=` | 1.0 | |
| +0x134 | 4 | float | `BuildTimeInfantryMult=` | 1.0 | Per-category build time mult |
| +0x138 | 4 | float | `BuildTimeUnitsMult=` | 1.0 | |
| +0x13C | 4 | float | `BuildTimeAircraftMult=` | 1.0 | |
| +0x140 | 4 | float | `BuildTimeBuildingsMult=` | 1.0 | |
| +0x144 | 4 | float | `BuildTimeDefensesMult=` | 1.0 | |
| +0x148 | 4 | float | `IncomeMult=` | 1.0 | Income/harvesting multiplier |
| +0x14C | 12 | -- | (padding/alignment) | | |
| +0x158 | 24 | DVC | `VeteranInfantry=` | empty | DynamicVector of InfantryType ptrs |
| +0x178 | 24 | DVC | `VeteranUnits=` | empty | DynamicVector of UnitType ptrs |
| +0x194 | 12 | DVC | `VeteranAircraft=` | empty | DynamicVector of AircraftType ptrs | (corrected 2026-05-29: was 16 bytes; ReadINI writes to +0x194/+0x198/+0x19C = 3 × 4 = 12 bytes, placing Suffix= at +0x1A0 with no gap — ROOT_CAUSE: OFFSET_RETYPED_WRONG; via `decompile_function 0x00511850`)
| +0x1A0 | 5 | char[5] | `Suffix=` | "" | 4-char suffix (e.g. "Allied", "Soviet") |
| +0x1A4 | 1 | char | `Prefix=` | 'A' (0x41) | 1-char prefix for asset naming |
| +0x1A5 | 1 | bool | `Multiplay=` | false | Selectable in multiplayer lobby |
| +0x1A6 | 1 | bool | `MultiplayPassive=` | false | Passive in multiplayer (observer-like) |
| +0x1A7 | 1 | bool | `WallOwner=` | true | Can own walls |
| +0x1A8 | 1 | bool | `SmartAI=` | false | Uses enhanced AI logic |

### ReadINI Parse Order (FUN_00511850)

1. Calls `AbstractTypeClass::ReadINI()` (base class: reads UIName, Name)
2. `Suffix=` -> +0x1A0 (strncpy, max 4 chars)
3. `ParentCountry=` -> +0x98 (strncpy, max 24 chars)
4. `Color=` -> +0xC0 (color scheme index lookup)
5. `Prefix=` -> +0x1A4 (single char)
6. 7 doubles: `Firepower=`, `Groundspeed=`, `Airspeed=`, `Armor=`, (unnamed), (unnamed), `BuildTime=`
7. 4 bools: `Multiplay=`, `MultiplayPassive=`, `WallOwner=`, `SmartAI=`
8. 19 floats: `ArmorInfantryMult=` through `IncomeMult=`
9. `VeteranInfantry=` -> tokenize by comma, resolve each via InfantryTypeClass::FindOrCreate
10. `VeteranUnits=` -> tokenize by comma, resolve each via UnitTypeClass::FindOrCreate
11. `VeteranAircraft=` -> tokenize by comma, resolve each via AircraftTypeClass::FindOrCreate
12. Resolves `ParentCountry` -> side index at +0xBC (via FUN_004756f0)
13. Updates parent country's child list cross-reference

### FindByName (FUN_005117d0) -- 23 callers

- Checks for `"<random>"` sentinel -> returns 0xFFFFFFFE (-2)
- Searches global array comparing both +0x64 (UIName) and +0x24 (ID)
- Returns the country's self-index (+0xB8) on match, or 0xFFFFFFFF (-1) if not found

### FindOrCreate (FUN_00512680)

- Checks for `"<none>"` and `"none"` sentinels -> returns 0 (corrected 2026-05-29: second sentinel was `"<all>"`; `read_memory 0x00817694` shows bytes `6e6f6e6500...` = "none" without angle brackets — ROOT_CAUSE: INFERENCE_HARDENED)
- Searches global array by name at +0x24
- If not found, allocates 0x1B0 bytes and calls constructor
- Called by [Countries] registration and [Sides] parser

---

## Part 2: SideTypeClass

### Overview

SideTypeClass is a simple grouping class that maps side names (GDI/Nod/ThirdSide/Civilian)
to lists of countries. Size **0xB4 = 180 bytes**. It inherits from AbstractTypeClass.

SideTypeClass does NOT have its own ReadINI function. It is populated entirely by the
`[Sides]` registration function (FUN_00672440).

### Key Addresses

| Symbol | Address | Purpose |
|--------|---------|---------|
| SideTypeClass::Constructor | `0x006a4550` | 192 bytes |
| SideTypeClass::FindByName | `0x006a46d0` | 61 bytes, 9 callers |
| [Sides] registration | `0x00672440` | 428 bytes, creates sides + links countries |
| Global array pointer | `DAT_008b4124` | `int*` -- array of SideTypeClass pointers |
| Global array count | `DAT_008b4130` | `int` -- number of registered sides |

### Struct Layout (0xB4 bytes)

| Offset | Size | Type | Purpose |
|--------|------|------|---------|
| +0x00 | 16 | ptr[4] | 4 vtable pointers |
| +0x24 | 64 | char[64] | Side name (e.g. "GDI", "Nod", "ThirdSide") |
| +0x98 | 4 | ptr | DVC vtable for country list |
| +0x9C | 4 | ptr | DVC data pointer -- array of country indices |
| +0xA0 | 4 | int | DVC capacity |
| +0xA4 | 1 | bool | DVC owns_memory flag |
| +0xA5 | 1 | bool | Unknown flag (initialized to 0) |
| +0xA8 | 4 | int | DVC element count -- number of countries in side |
| +0xAC | 4 | int | DVC grow amount (default 10) |
| +0xB0 | 4 | int | Unknown (initialized to 0) |

### [Sides] Registration (FUN_00672440)

The registration function at `0x00672440` does:

1. Iterates entries in `[Sides]` section (e.g., `GDI=British,French,...`)
2. For each side name, checks if a SideTypeClass already exists via FindByName
3. If not found, allocates 0xB4 bytes and calls constructor
4. Logs: `"Side %d: %s"`
5. Parses the comma-separated country list value
6. For each country name in the list:
   - Looks up the country's index via `CountryTypeClass::FindByName`
   - Sets the country's **side index** field at `CountryTypeClass+0xBC` to the current side's index
   - Logs: `"  %s"` (country name)

This is how countries get their Side= field populated -- it's NOT from the country's own
INI section, but from the [Sides] section mapping.

### Side Indices in Vanilla YR (rulesmd.ini)

| Index | Name | Countries |
|-------|------|-----------|
| 0 | GDI (Allied) | British, French, Germans, Americans, Alliance |
| 1 | Nod (Soviet) | Russians, Africans, Confederation, Arabs |
| 2 | ThirdSide (Yuri) | YuriCountry |
| 3 | Civilian | Neutral |

Note: The INI uses "GDI" and "Nod" as side names (legacy from Tiberian Sun).
In-game, these map to Allied and Soviet. The `Side=` key in each country section
(e.g., `Side=GDI` for Americans) is **separate** from the side index. The index
is derived from the `[Sides]` section order at parse time.

---

## Part 3: How Countries Connect to Game Start

### Flow: Lobby Selection -> HouseClass Creation

1. **Rules loading** (`FUN_00668bf0`):
   - Iterates `[Countries]` -- calls `CountryTypeClass::FindOrCreate` for each entry
   - Calls `[Sides]` registration (`FUN_00672440`) -- links countries to sides
   - Then iterates CountryTypeClass array calling vtable+100 (ReadINI) on each

2. **Multiplayer init** (address UNVERIFIED — `FUN_006980c0` is actually `CDFileClass__Constructor` per `get_function_by_address 0x006980c0`; the real multiplayer init address is not confirmed in this session):
   - Reads player slot settings: `SideEx=` (country index), `Color=`, team assignment

3. **Create_Houses** (`FUN_00687f10`, 1134 bytes, at `0x00687f10`):
   - Sorts players by priority
   - For each human player:
     - Allocates HouseClass (0x160B8 = ~90KB!)
     - Calls `FUN_004f54a0` (HouseClass constructor)
     - Stores CountryTypeClass pointer at HouseClass+0x34
     - Calls `FUN_004fce00` (`HouseClass__Set_Credits_And_Color` per `get_function_by_address 0x004fce00`) — sets initial credits and color, not full production/economy init (corrected 2026-05-29: was "init production/economy"; ROOT_CAUSE: INFERENCE_HARDENED)
   - For each AI player:
     - Same process with `"Computer"` name, AI flags
   - Creates special houses: "Neutral" and "Special"

4. **HouseClass key country fields:**
   - `+0x30`: house index in global array
   - `+0x34`: **CountryTypeClass pointer** -- the house's faction identity
   - `+0x1EC`: IsHuman flag (1=human, 0=AI)
   - `+0x1ED`: IsLocalPlayer flag
   - `+0x5788`: Alliance bitfield

### Flow: BaseUnit Selection

The BaseUnit for MCV deployment uses the Owner bitmask system:

1. `BaseUnit=` in `[General]` is a comma-separated list of vehicle types stored on RuleSet
   at offset +0xB30 as a DynamicVector.
2. `FUN_00505310` (House::Find_First_Owned_Type):
   - Gets country index from `CountryTypeClass::FindByName` (called with the house's country name)
   - For each type in the BaseUnit list, checks: `type.Owner_bitmask & (1 << country_index)`
   - Returns the first matching type
3. This means the MCV type is determined by the `Owner=` field on each vehicle type.
   Typically, `Owner=Americans,Alliance,French,...,Russians,...` covers all playable factions.

### Flow: Sidebar / Build Options Filtering

`FUN_005051e0` (House::Find_First_Buildable_From_List) checks 4 conditions:

1. **Owner bitmask** (TechnoType+0x6CC): `(1 << country_index) & Owner != 0`
   - Country index comes from `CountryTypeClass.self_index` (+0xB8)
   - The bitmask is built by parsing `Owner=` as comma-separated country names,
     converting each to an index via `CountryTypeClass::FindByName`, then `1 << index`

2. **RequiredHouses bitmask** (TechnoType+0xDA0): If not -1 (default),
   `(1 << country_index) & RequiredHouses != 0` must be true
   - This restricts the unit to specific countries (e.g., `RequiredHouses=Americans` means
     only Americans can build it, even if Owner= includes their side)

3. **ForbiddenHouses bitmask** (TechnoType+0xDA4): If not -1 (default),
   `(1 << country_index) & ForbiddenHouses == 0` must be true
   - Explicitly blocks specific countries

4. **Side index** (TechnoType+0x6D0): If not -1,
   `type.AIBasePlanningSide == CountryType.side_index`
   - AI-only filter for base planning (not used for human sidebar)

### Owner Bitmask System

The Owner=, RequiredHouses=, ForbiddenHouses= fields are **bitmasks** where each bit
position corresponds to a country's index in the global `[Countries]` array:

```
[Countries]
0=Americans    -> bit 0  (0x001)
1=Alliance     -> bit 1  (0x002)
2=French       -> bit 2  (0x004)
3=Germans      -> bit 3  (0x008)
4=British      -> bit 4  (0x010)
5=Africans     -> bit 5  (0x020)
6=Arabs        -> bit 6  (0x040)
7=Confederation -> bit 7 (0x080)
8=Russians     -> bit 8  (0x100)
9=YuriCountry  -> bit 9  (0x200)
...
```

Parser: `FUN_00475260` reads comma-separated names, calls `FUN_0050c170` for each
(returns 0-31 index), then ORs `1 << index` into the bitmask.

Maximum: 32 countries (limited by 32-bit bitmask, `1 << (index & 0x1f)`).

---

## Part 4: [Countries] Section Parser

### Registration Function (FUN_006722f0, at 0x006722F0)

- **Size:** 106 bytes
- **Called by:** 3 functions (FUN_00686b20, FUN_006980c0, FUN_006ae2c0)
- **Behavior:**
  1. Calls `FUN_00526960(PTR_s_Countries)` to get the entry count in `[Countries]`
  2. For each entry (index 0..count-1):
     - Reads the entry value (country name) via `FUN_00526cc0` + `FUN_00528a10`
     - Calls `FUN_00512680` (CountryTypeClass::FindOrCreate) to register it
  3. Returns `count > 0`

### Parse sequence in master loader (FUN_00668bf0)

```
1. [Colors] + [ColorAdd]
2. [Countries]  <-- registers CountryTypeClass objects
3. [Sides]      <-- links countries to sides, sets CountryType.side_index
4. [OverlayTypes]
5. [SuperWeaponTypes]
6. [Warheads]
7. [SmudgeTypes]
8. [TerrainTypes]
9. [BuildingTypes], [VehicleTypes], [AircraftTypes], [InfantryTypes]
10. [Animations], [VoxelAnims], [Particles], [ParticleSystems]
... (many more sections)
```

After registration, each CountryTypeClass's ReadINI is called through the vtable
(vtable+100 = offset 0x64 in vtable, which is AbstractTypeClass::ReadINI's virtual slot).

### Maximum Country Count

Limited to **32** by the Owner bitmask system (`1 << (index & 0x1f)`). The bitmask
is stored as a 32-bit unsigned int. Vanilla YR uses 14 countries (indices 0-13).

---

## Part 5: Fields NOT on CountryTypeClass

The following fields were asked about but are NOT on CountryTypeClass:

### ParaDrop.Types, ParaDrop.Num, ParaDrop.Aircraft

These are per-SIDE fields on **RulesetClass** (`[General]` section), not per-country:
- `AmerParaDropInf=` -> RuleSet+0xC04
- `AmerParaDropNum=` -> RuleSet+0xC1C
- `AllyParaDropInf=` -> RuleSet+0xC3C
- `AllyParaDropNum=` -> RuleSet+0xC54
- `SovParaDropInf=` -> RuleSet+0xC74
- `SovParaDropNum=` -> RuleSet+0xC8C
- `YuriParaDropInf=` -> RuleSet+0xCAC
- `YuriParaDropNum=` -> RuleSet+0xCC4

These use hardcoded per-side key names (Amer/Ally/Sov/Yuri), not a generic per-country
override system.

### Side=

The `Side=` key that appears in country INI sections (e.g., `Side=GDI` under `[Americans]`)
is **NOT parsed by CountryTypeClass::ReadINI**. The side index at CountryTypeClass+0xBC
is set by the `[Sides]` registration function, which reads the [Sides] section values
(e.g., `GDI=British,French,...`) and assigns each country its side index.

The `Side=` key in individual country sections appears to be a legacy/unused field. However, note
that `HouseTypeClass::ReadINI` (FUN_00511850) DOES write to `+0xBC` at the end of parsing via
`FUN_004756f0` — both the [Sides] registration and ReadINI write the side index there (corrected
2026-05-29; verified via `decompile_function 0x00511850`).

---

## Relevance to ra2-rust-game

### Current State

- `src/rules/ruleset.rs` -- Does NOT parse [Countries] or [Sides]
- `src/rules/object_type.rs` -- Owner/RequiredHouses/ForbiddenHouses are stored as `Vec<String>`
  (name lists), not as bitmasks
- `src/map/houses.rs` -- Parses per-map [Houses] section with Country/Side/Color
- `src/sim/production_tech.rs` -- Checks Owner by string matching against country name
  (correct behavior, just not using bitmask optimization)

### What Needs Implementation

1. **CountryType struct** in `rules/` with all fields from the struct layout above
2. **SideType struct** in `rules/` mapping side names to country lists
3. **[Countries] and [Sides] parsing** in `RuleSet::load()`
4. **Country multipliers** applied during HouseClass creation (Firepower, Armor, etc.)
5. **VeteranInfantry/Units/Aircraft** lists for spawn-at-veteran-rank feature
6. **Multiplay flag** for lobby filtering (which countries appear in the dropdown)

### What Works Already

- Owner/RequiredHouses/ForbiddenHouses matching via string comparison is functionally correct
  for gameplay (just slower than bitmask, but irrelevant for performance)
- Map house parsing correctly reads Country= and Side= from [Houses]
- Color scheme assignment works through house_colors module
