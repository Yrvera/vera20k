---
name: HouseTypeClass (gamemd.exe)
date: 2026-04-21
related:
  - COUNTRY_SIDE_TYPE_CLASSES.md (2026-03-22) — extends and corrects offsets/defaults
  - COUNTRY_MULTIPLIERS_APPLICATION.md (2026-03-22) — extends (ROF/Cost names, veteran consumers)
  - SIDECLASS_GHIDRA_REPORT.md (2026-04-19) — cross-reference
  - HOUSECLASS_VERIFIED_FIELD_MAP.md (2026-03-26) — cross-reference
  - OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md (2026-03-22) — cross-reference
---

# HouseTypeClass — Ghidra Research Report

**Primary address:** `0x00511850` (ReadINI), `0x005113F0` (Constructor)
**Size:** `0x1B0` bytes (432)
**Confidence:** HIGH (all struct offsets, INI keys, consumers verified from live decompilation)
**Active in YR:** Yes — load-bearing. Every match selects one per player.

This report consolidates and corrects the prior CountryTypeClass / SideTypeClass reports. In
gamemd.exe the class is named `HouseTypeClass` in the RA2 source header (`D:\ra2mdpost\Country.CPP`)
but YRpp/ModEnc call it `HouseTypeClass` — it is the *type* template, not the per-match live
HouseClass instance. The two are linked by HouseClass `+0x34` → HouseTypeClass*. INI section
name and terminology here follow the binary strings: `[Countries]`, per-country `[Americans]`,
etc.

---

## 1. What existing reports already cover

- **`COUNTRY_SIDE_TYPE_CLASSES.md`** — full 0x1B0 struct, ReadINI (0x511850), Constructor (0x5113F0),
  FindByName (0x5117D0), FindOrCreate (0x512680), WriteINI (0x512170), `[Countries]` parser
  (0x6722F0), `[Sides]` linkage (0x672440), Owner bitmask pipeline, 14-country vanilla list.
- **`COUNTRY_MULTIPLIERS_APPLICATION.md`** — `SetDifficulty` baked doubles, the five per-category
  float accessors (`GetArmorBonus`/`GetCostBonus`/`GetSpeedBonus`/`GetBuildTimeBonus` +
  `GetAccumulatedBonus`), `IncomeMult` path, VeteranInfantry/Units/Aircraft storage.
- **`SIDECLASS_GHIDRA_REPORT.md`** — SideClass layout, `[Sides]` parser, per-country `Side=`
  override (corrects the earlier claim that the key is unused), `"Civilian"` runtime special case.
- **`HOUSECLASS_VERIFIED_FIELD_MAP.md`** — the HouseClass instance and the `+0x34` back-pointer.
- **`OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md`** — Owner / RequiredHouses / ForbiddenHouses
  bitmask construction, `CanBuild` flow, StolenTech, BuildLimit, AIBasePlanningSide.

**This report fills the remaining gaps** and corrects three specific errors in the prior docs.

---

## 2. Corrections to prior reports

### 2.1 The "unnamed 5" / "unnamed 6" doubles are `ROF` and `Cost`

Prior reports marked `HouseTypeClass+0xE8` and `+0xF0` as unnamed doubles with guessed INI keys.
Reading memory at the string literals in `HouseTypeClass::ReadINI` resolves them:

| Offset | INI key | String literal |
|-------:|---------|----------------|
| `+0xE8` | `ROF=` | `0x00825478` → bytes `52 4F 46 00` = `"ROF"` |
| `+0xF0` | `Cost=` | `0x00825470` → bytes `43 6F 73 74 00` = `"Cost"` |

These align with what `SetDifficulty` (`0x004F6EC0`) does in multiplayer: it multiplies
`Rules[diffOffset + 0x1548]` by `countryType[+0xE8]` → stores at `HouseClass+0x198` (ROF), and
`Rules[diffOffset + 0x1568]` by `countryType[+0xF0]` → stores at `HouseClass+0x1B8` (Cost).
`COUNTRY_MULTIPLIERS_APPLICATION.md` already documented the pipeline but named the fields by the
*HouseClass* destination rather than the CountryType INI key. The authoritative names are `ROF` and
`Cost` — both as INI keys and as the logical field names.

### 2.2 VeteranInfantry / VeteranUnits / VeteranAircraft DVC offsets

Prior report listed the three DVCs as starting at `+0x158`, `+0x178`, `+0x194` (sizes 24/24/16
bytes). Those are the **count** positions inside each DVC, not the DVC start. The verified
layout (from the Constructor writes at byte offsets `0x14C`, `0x168`, `0x184` for the DVC
vtable pointers, and confirmed by the consumers reading `data` one word after the vtable):

| DVC | vtable | data | capacity | owns | count | grow |
|-----|------:|------:|---------:|------:|------:|------:|
| VeteranInfantry | `+0x14C` | `+0x150` | `+0x154` | `+0x158` | **`+0x15C`** | `+0x160` |
| VeteranUnits    | `+0x168` | `+0x16C` | `+0x170` | `+0x174` | **`+0x178`** | `+0x17C` |
| VeteranAircraft | `+0x184` | `+0x188` | `+0x18C` | `+0x190` | **`+0x194`** | `+0x198` |

Total size from `+0x14C` to `+0x19C` = 0x50 bytes (3 × 0x18-byte DVCs). The overall
`0x1B0` struct size is unchanged.

### 2.3 `ParentCountry=` is a dead field in YR

Prior claim (implied by multiple reports): `ParentCountry` resolves the child country's side
if no direct `[Sides]` entry covers it, and cross-registers with the parent's child list.

**Verified:** The string literal `"ParentCountry"` at `0x0082549C` is referenced from exactly
**one** site — `HouseTypeClass::ReadINI` at `0x005118DA`, where it is read into the 25-byte
buffer at `+0x98`. No other function in the binary references offset `+0x98` of a
HouseTypeClass, and no function reads the string constant elsewhere. The side lookup at the
tail of `ReadINI` (`FUN_004756F0(this+0x24, "Side", default)`) reads the per-country `Side=`
key, not `ParentCountry=`.

**Conclusion:** `ParentCountry=` is parsed and stored, but **no runtime code consumes it**.
It is either a Tiberian Sun holdover (TS used it to chain side derivation) or a design-time
hook that was never wired. No INI data falls back through parent→child. Each country's
multipliers, prefix, color, veteran lists, and side are resolved from its own INI section in
isolation. `ParentCountry=` in vanilla YR sets no observable behavior.

Confidence: HIGH — single xref verified by direct string-address search.

---

## 3. Complete INI key map — exactly 38 keys

`HouseTypeClass::ReadINI` at `0x00511850` is a 2325-byte function. It reads the following
keys in this order, and **no others**. Any other per-country INI claim (`TauntFile=`,
`VoxFile=`, `FlagFile=`, `SideName=`, `RandomSelectionWeight=`, `AIBase…=`,
`DifficultyModifier=`, `VeteranBuildings=`, `VeteranTech=`) is **not** read and **not** stored
on HouseTypeClass. See §5 for how those concepts are implemented (or that they do not exist).

| # | INI key | Offset | Type | Default | Default source |
|--:|---------|-------:|------|---------|----------------|
| 1 | `UIName=` | `+0x64` | char[52] | — | AbstractTypeClass::ReadINI |
| 2 | `Name=` (alias `Suffix` in logic) | — | — | — | same base class |
| 3 | `Suffix=` | `+0x1A0` | char[5] | `""` | Constructor zeroes |
| 4 | `ParentCountry=` | `+0x98` | char[25] | `""` | Constructor zeroes; **DEAD — §2.3** |
| 5 | `Color=` | `+0xC0` | int (scheme index) | `-1` → clamped to `0` | `FUN_00474A90` re-lookup |
| 6 | `Prefix=` | `+0x1A4` | char | `'A'` (0x41) | Constructor |
| 7 | `Firepower=` | `+0xC8` | double | 1.0 | Constructor |
| 8 | `Groundspeed=` | `+0xD0` | double | 1.0 | Constructor |
| 9 | `Airspeed=` | `+0xD8` | double | 1.0 | Constructor |
| 10 | `Armor=` | `+0xE0` | double | 1.0 | Constructor |
| 11 | `ROF=` | `+0xE8` | double | 1.0 | Constructor |
| 12 | `Cost=` | `+0xF0` | double | 1.0 | Constructor |
| 13 | `BuildTime=` | `+0xF8` | double | 1.0 | Constructor |
| 14 | `Multiplay=` | `+0x1A5` | bool | false | Constructor |
| 15 | `MultiplayPassive=` | `+0x1A6` | bool | false | Constructor |
| 16 | `WallOwner=` | `+0x1A7` | bool | **true** | Constructor |
| 17 | `SmartAI=` | `+0x1A8` | bool | false | Constructor |
| 18 | `ArmorInfantryMult=` | `+0x100` | float | 1.0 | Constructor |
| 19 | `ArmorUnitsMult=` | `+0x104` | float | 1.0 | Constructor |
| 20 | `ArmorAircraftMult=` | `+0x108` | float | 1.0 | Constructor |
| 21 | `ArmorBuildingsMult=` | `+0x10C` | float | 1.0 | Constructor |
| 22 | `ArmorDefensesMult=` | `+0x110` | float | 1.0 | Constructor |
| 23 | `CostInfantryMult=` | `+0x114` | float | 1.0 | Constructor |
| 24 | `CostUnitsMult=` | `+0x118` | float | 1.0 | Constructor |
| 25 | `CostAircraftMult=` | `+0x11C` | float | 1.0 | Constructor |
| 26 | `CostBuildingsMult=` | `+0x120` | float | 1.0 | Constructor |
| 27 | `CostDefensesMult=` | `+0x124` | float | 1.0 | Constructor |
| 28 | `SpeedInfantryMult=` | `+0x128` | float | 1.0 | Constructor |
| 29 | `SpeedUnitsMult=` | `+0x12C` | float | 1.0 | Constructor |
| 30 | `SpeedAircraftMult=` | `+0x130` | float | 1.0 | Constructor |
| 31 | `BuildTimeInfantryMult=` | `+0x134` | float | 1.0 | Constructor |
| 32 | `BuildTimeUnitsMult=` | `+0x138` | float | 1.0 | Constructor |
| 33 | `BuildTimeAircraftMult=` | `+0x13C` | float | 1.0 | Constructor |
| 34 | `BuildTimeBuildingsMult=` | `+0x140` | float | 1.0 | Constructor |
| 35 | `BuildTimeDefensesMult=` | `+0x144` | float | 1.0 | Constructor |
| 36 | `IncomeMult=` | `+0x148` | float | 1.0 | Constructor |
| 37 | `VeteranInfantry=` | DVC `+0x14C` | DVC<InfantryType*> | empty | Constructor |
| 38 | `VeteranUnits=` | DVC `+0x168` | DVC<UnitType*> | empty | Constructor |
| 39 | `VeteranAircraft=` | DVC `+0x184` | DVC<AircraftType*> | empty | Constructor |
| 40 | `Side=` | `+0xBC` | int (side index) | value from `[Sides]` pass | `FUN_004756F0` after |

Count: 38 keys inside `ReadINI` (counting `Name=` and `UIName=` as one each from
AbstractTypeClass::ReadINI) plus `Side=` resolved at the tail, plus the Veteran DVC triple.
Keys 37/38/39 each parse a comma-separated list via `CRT__strtok` against delimiter
`","` at `DAT_00817F70`, resolving each token via `InfantryTypeClass::FindOrCreate` /
`UnitTypeClass::FindOrCreate` / `AircraftTypeClass::FindOrCreate`.

The `Side=` tail block at `+0xBC` also rewrites the parent SideClass's country-list DVC —
see `SIDECLASS_GHIDRA_REPORT.md` §5 for the add/remove compaction logic.

---

## 4. Constructor defaults, verified

`HouseTypeClass::Constructor` at `0x005113F0` takes `undefined4 *param_1` (treated as
`int[]`, so `param_1[k]` is byte offset `k*4`). All observed initializations (from direct
decompilation):

- Zero byte at `+0x98` (ParentCountry NUL-terminator).
- `+0xB4 = -1`, `+0xB8 = -1` (self-indices, filled after insertion at the end of the
  constructor by two scans of the global array for `*this`).
- `+0xBC = -1` (side index, overwritten by `[Sides]` parser).
- `+0xC0 = 0` (color index — prior docs said `-1`; the constructor writes `0`, which means
  the first color scheme in the array; but `ReadINI` re-resolves it via
  `FUN_00474A90(…, "Color", current_value)` which uses the **current value's scheme name** as
  the default string, so a missing `Color=` key keeps index 0, not −1).
- 7 doubles at `+0xC8/+0xD0/+0xD8/+0xE0/+0xE8/+0xF0/+0xF8` = `1.0` (bytes `00 00 00 00 00 00
  F0 3F`, written as two `int` halves).
- 19 floats at `+0x100..+0x148` = `1.0f` (bytes `00 00 80 3F`).
- Three DVC initializations with `owns_memory=1`, `grow=10` at `+0x14C`, `+0x168`, `+0x184`.
- `+0x1A4 = 'A'` (Prefix default).
- `+0x1A5 = 0` (Multiplay=false), `+0x1A6 = 0` (MultiplayPassive=false),
  `+0x1A7 = 1` (**WallOwner=true** — this is the only boolean with a non-zero default),
  `+0x1A8 = 0` (SmartAI=false).
- Vtable + 3 secondary vtables installed.
- `AbstractClass::AssignUniqueID` called.
- **Self-append to global array** `DAT_00A83C9C` (data ptr), `DAT_00A83CA8` (count),
  `DAT_00A83CA0` (capacity). On success the count is incremented and the index is written to
  both `+0xB4` and `+0xB8` by scanning the array for `*this`.

Gotcha: the constructor signature `undefined4 *` means `param_1[k]` is byte offset
`k*4`. The writes like `param_1[0x26] = 0` set a byte at offset `0x26*4=0x98`, but the
`*(undefined1 *)(param_1 + 0x69)` writes are **byte** stores at `param_1` cast up — those
resolve to byte offset `0x69*4 = 0x1A4` (Prefix), not `0x69`. This is the same trap called
out in `CLAUDE.md`: int-pointer vs byte-pointer arithmetic in Ghidra decompilation.

---

## 5. Fields the user asked about that DO NOT EXIST on HouseTypeClass

Exhaustive string search of gamemd.exe confirms these INI keys are **not present** as
per-country parsable keys:

| Asked-about key | Status | Where the concept actually lives |
|-----------------|--------|----------------------------------|
| `TauntFile=` | ❌ not in binary | Taunts are **hardcoded** by country index — see §5.1 |
| `VoxFile=` / `VoiceSet=` / `EVAFile=` | ❌ not in binary | EVA voice set is picked by house **SideIndex** (0/1/2) via hardcoded tables in VoxClass; not a per-country field. The per-country `Side=` key decides it |
| `FlagFile=` | ❌ not in binary | Flag animation is the global constant `FLAGFLY.SHP` (string `0x008458F8`, used by `UnitClass::DrawExtras` for all houses) |
| `SideName=` | ❌ not in binary | `Side=` INI key on the country resolves to a SideClass by name via `FUN_004756F0`; there is no separate `SideName=` key |
| `RandomSelectionWeight=` | ❌ not in binary | `ProcessRandomAssignments` (`0x0069B8C0`) picks countries **uniformly** over `Random(0, 9)`; no weight field is consulted |
| `AIBaseSpacing=` | ✅ but global | Single float on RulesClass (`[General] AIBaseSpacing=`). There is no per-country override |
| `AIBasePlanningSide=` | ✅ but per-type | On TechnoTypeClass (`+0x6D0`). Per-*type*, not per-country; see `OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` §8 |
| `DifficultyModifier=` | ❌ not in binary | Only global `[Easy]`/`[Normal]`/`[Difficult]` sections on RulesClass; no per-country overlay |
| `VeteranBuildings=` | ❌ not in binary | Only `VeteranInfantry=`/`VeteranUnits=`/`VeteranAircraft=` — no building equivalent |
| `SecretUnits=` *(per country)* | ❌ not per-country | Exists only in `[General]` (global pool — §5.3) |

Every one of these was verified by a `search_strings` hit count of 0 or 1, and in the
"1" cases the hit's xref goes to `RulesClass::ReadGeneral` or a `BuildingTypeClass::ReadINI`,
not to `HouseTypeClass::ReadINI`.

### 5.1 Taunts: hardcoded country-index switch (`SpeechSystem::PlayTaunt` at `0x00752B70`)

The taunt file name is selected by a **hardcoded switch on `(param >> 4) & 0xF`** — the
country slot 0–9:

| Country index | Taunt file template | Matches vanilla `[Countries]` entry |
|:-------------:|---------------------|-------------------------------------|
| 0 | `taunts\tauam%02i.wav` | Americans |
| 1 | `taunts\tauko%02i.wav` | Alliance (Korea — `ko`) |
| 2 | `taunts\taufr%02i.wav` | French |
| 3 | `taunts\tauge%02i.wav` | Germans |
| 4 | `taunts\taubr%02i.wav` | British |
| 5 | `taunts\tauli%02i.wav` | Africans (Libya — `li`) |
| 6 | `taunts\tauir%02i.wav` | Arabs (Iraq — `ir`) |
| 7 | `taunts\taucu%02i.wav` | Confederation (Cuba — `cu`) |
| 8 | `taunts\tauru%02i.wav` | Russians |
| 9 | `taunts\tauyu%02i.wav` | YuriCountry |

**Load-bearing gotcha:** the 2-character prefixes are **wired to the first 10 entries of
`[Countries]` by index**. Reordering `[Countries]` in a mod breaks all taunt audio silently
— the switch uses the slot, not the country's name or Prefix/Suffix field. This is a major
determinism constraint for Rust: the engine must preserve `[Countries]` insertion order
exactly as `rulesmd.ini` specifies it (indices 0–9), or else Americans will taunt in
Korean and vice versa. `taubr` for British is index 4 even though British is alphabetically
after Arabs; the order comes from the file, not sorting.

The low nibble `bVar1 = param & 0xF` is checked against `1..8` — so each country has taunts
1..8 (8 taunt lines per faction). Value 0 or 9+ returns 0 silently.

### 5.2 SecretUnits / SecretBuildings: global pool, not per-country

The keys exist only in `[General]`:

```ini
[General]
SecretInfantry=SNIPE,TERROR,DESO,YURI
SecretUnits=TNKD,TTNK,DTRUCK
SecretBuildings=GTGCAN
```

Read by `RulesClass::ReadGeneral` (`0x0066F...`). The per-type gate is
`TechnoTypeClass+0xDA8 = SecretHouses=` (bitmask, written by `TechnoTypeClass::ReadINI` at
`0x00714543`). The selection mechanism is:

1. A house builds (or captures) a Secret Lab (a building that has `SecretLab=yes` at
   `BuildingTypeClass+…`).
2. At construction, the engine picks **one** item from each of the global
   `SecretInfantry`/`SecretUnits`/`SecretBuildings` lists that is allowed for this house
   (bit-AND `SecretHouses` against `1 << country_index`).
3. That type becomes buildable (a separate unlock flag on the house tracks it).

**This is distinct from the RequiredHouses per-country gating.** Vanilla YR gates
country-specific units via **Owner=/RequiredHouses=/ForbiddenHouses= on the TechnoType**
(e.g. `SNIPE` has `RequiredHouses=British`, `DESO` has `RequiredHouses=Arabs`). The Secret
Lab mechanism is a separate, random-selection reward that also uses the country bitmask
bit. Tank Destroyer (TNKD) is in the `SecretUnits` pool — it is not wired to Germans via
per-country Prerequisites on HouseTypeClass. The `RequiredHouses=` bit-filter and the
`SecretHouses=` bit-filter use the same country-index encoding defined in §8.

There is **no per-country `SecretUnits=`** field on HouseTypeClass.

### 5.3 Flag draw: hardcoded global `FLAGFLY.SHP`

`UnitClass::DrawExtras` at `0x0073D3E7` loads the single global string
`0x008458F8 = "FLAGFLY.SHP"` for all factions. The per-house tint is applied through the
house color scheme (index at HouseClass `+0x16054`), not through a per-country flag asset
path. No `FlagFile=` INI key exists.

### 5.4 EVA / Voice: per-side, not per-country

EVA lines are selected by the house's **SideIndex** (0 = Allied, 1 = Soviet, 2 = Yuri)
via hardcoded voice tables in VoxClass. There is no `VoxFile=` or `EVAFile=` per-country
field. The per-country `Side=` key indirectly chooses the EVA set by selecting the
SideClass index.

---

## 6. VeteranInfantry / VeteranUnits / VeteranAircraft — spawn-at-veteran consumers (verified)

Prior doc had the storage right and the spawn behavior **inferred**. Both have now been
verified from live decompilation:

### 6.1 VeteranInfantry — `InfantryClass::InitFromType` at `0x00517CC0`

```c
void InfantryClass::InitFromType(InfantryClass* self) {
    TechnoClass::Init_Managers();
    if (self->Owner /* +0x21C */) HouseClass::Add_Tracking(self);
    if (self->Type /* +0x6C0 */) {
        if (self->Owner) {
            HouseTypeClass* ct = self->Owner->HouseType;  // +0x34
            int count = ct->VeteranInfantry_count;         // +0x15C
            InfantryType** data = ct->VeteranInfantry_data;// +0x150
            for (int i = 0; i < count; i++) {
                if (data[i] == self->Type) {
                    VeterancyStruct::SetVeteran(1);        // self-member 0x150 area
                    break;
                }
            }
            // Stolen-tech fallback path:
            if (self->Owner->StolenThirdTech /* +0x2BF */
                && self->Type->field_0xC8E /* some flag */) {
                VeterancyStruct::SetVeteran(1);
            }
        }
        // ... copies cell/facing/strength caps from type ...
    }
}
```

### 6.2 VeteranUnits — `UnitClass::Constructor` at `0x007353C0`

```c
// After locomotor COM init, inside the `if (Type != NULL)` block:
if (self->Owner /* +0x21C */) {
    HouseTypeClass* ct = self->Owner->HouseType;   // +0x34
    int count = ct->VeteranUnits_count;             // +0x178
    UnitType** data = ct->VeteranUnits_data;        // +0x16C
    for (int i = 0; i < count; i++) {
        if (data[i] == self->Type) {
            VeterancyStruct::SetVeteran(1);
            break;
        }
    }
    // Stolen-tech/flag fallback:
    if (self->Owner->field_0x2C0           // see §6.4 — flag not fully identified
        && !self->Type->Naval /* +0xCCE */
        && self->Type->field_0xC8E) {
        VeterancyStruct::SetVeteran(1);
    }
}
```

### 6.3 VeteranAircraft — `AircraftClass::InitFromType` at `0x00413F80`

```c
if (self->Type /* +0x6C4 */ && self->Owner /* +0x21C */) {
    HouseTypeClass* ct = self->Owner->HouseType;    // +0x34
    int count = ct->VeteranAircraft_count;           // +0x194
    AircraftType** data = ct->VeteranAircraft_data;  // +0x188
    for (int i = 0; i < count; i++) {
        if (data[i] == self->Type) {
            VeterancyStruct::SetVeteran(1);
            break;
        }
    }
    // ... copies weapon slots, facing, strength ...
}
```

All three paths are identical in shape: linear scan of the DVC, pointer-equality against
the TechnoType pointer, early-exit on first match, `SetVeteran(1)` (rank 1 = Veteran — not
Elite). Units not in the list spawn at Rookie. The `InitialVeteran` game option
(SpecialFlags bit) is a separate global override — not related to these lists.

### 6.4 Secondary veteran-grant paths

Both InfantryClass and UnitClass have a **second** veteran check using a HouseClass flag
(`+0x2BF`/`+0x2C0`) and a TechnoType flag (`+0xC8E`). The `+0x2BD..+0x2BF` bytes on HouseClass
are documented as stolen-tech flags in `OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` §4
(`StolenAlliedTech`/`StolenSovietTech`/`StolenThirdTech`). The byte at `+0x2C0` has been
documented elsewhere as a power-sabotage flag, but its read here for veteran eligibility
suggests either (a) it is also reused for a "this side's tech stolen" bit, or (b) the prior
doc is off by one. **Open question** — see §12.

`TechnoTypeClass+0xC8E` is a bool — likely "veteran-eligible"; identification is also an
open question, but the gate is not complicated: the path only fires when the house has the
corresponding stolen-tech flag **and** the type opts in. Vanilla rulesmd.ini only lists
`VeteranUnits=DVDTANK` / `VeteranInfantry=DVDPLT` (commented out in most cases) — this
secondary path is engine machinery that vanilla rarely uses.

---

## 7. Load order — when `[Countries]` and HouseTypeClass::ReadINI run

The master rules loader is `FUN_00668BF0` (the "Full Init" rules-load path). Verified sequence:

```
1.  FUN_0066D3A0           — [Colors] / palette init
2.  [ColorAdd]             — ColorAdd RGB offsets
3.  [Countries]            — FUN_006722F0: list-parse + FindOrCreate each name
4.  [Sides]                — FUN_00672440: allocate SideClass per entry, write HouseType+0xBC
5.  [OverlayTypes]
6.  [SuperWeaponTypes]
7.  [Warheads]
8.  [SmudgeTypes]
9.  [TerrainTypes]
10. FUN_00672660           — [BuildingTypes] register
11. FUN_00672360           — [VehicleTypes] register
12. FUN_006723D0           — [AircraftTypes] register
13. FUN_00672280           — [InfantryTypes] register
14. FUN_006728B0           — [Animations]
15. FUN_00672920           — [VoxelAnims]
16. FUN_00672A00           — [Particles]
17. FUN_00672A70           — [ParticleSystems]
18. RulesClass::ReadJumpjetControls
19. RulesClass::ReadMultiplayerDialogSettings
20. FUN_00672AE0
21. FUN_00673E80
22. RulesClass::ReadSpeedTypeLandTypeTable
23. RulesClass::ReadIQ
24. RulesClass::ReadGeneral        ← [General]  (SecretUnits, SecretBuildings, etc.)
25. **`FUN_00679A10`** — **THE per-type ReadINI dispatcher** (see §7.1 below)
26. FUN_0066D270 × 3               ← [Normal], [Difficult], difficulty tier doubles
27. RulesClass::ReadCrateRules
28. RulesClass::ReadCombatDamage
29. RulesClass::ReadRadiation
30. FUN_0066D150 / FUN_0066D1F0    ← map-control / waypoints
31. RulesClass::ReadAudioVisual
32. RulesClass::ReadSpecialWeapons
33. TiberiumClass::ReadINI_All
34. FUN_00674650(0 or 1)           ← type-specific ReadINI dispatcher (vtable)
```

Key observations:

- **`[Countries]` registration happens at step 3, before `[General]` (step 24).** So when
  `HouseTypeClass::FindOrCreate` is called for each country name, no `[General]` default has
  yet been parsed. This is fine — country defaults come from the constructor (`1.0`, `'A'`, etc.),
  not `[General]`.
- **`[Sides]` links countries immediately (step 4).** Each country's `+0xBC` side index is
  written by `FUN_00672440` before any ReadINI runs.
- **Per-country vtable-dispatched `ReadINI` runs in step 25** (`FUN_00679A10`), *after* all
  type registrations and `[General]` parsing are complete. The `Side=` tail in `ReadINI`
  therefore re-overrides the `[Sides]`-assigned index if the country's own section
  specifies one (see `SIDECLASS_GHIDRA_REPORT.md` §5; in vanilla both agree).
- **The `[Countries]` list always has its order determined by the numeric prefixes in the
  file** (`0=Americans`, `1=Alliance`, …). The parser trusts the prefix; gaps (like the
  blank lines in the vanilla file) are ignored.

The only xref to `HouseTypeClass::ReadINI` (address `0x00511850`) is the vtable slot at
`0x007EABBC`, which is `vtable__HouseTypeClass[+100]` — the `AbstractTypeClass::ReadINI`
virtual slot. `FUN_00679A10` dispatches through that slot for every registered
HouseTypeClass.

### 7.1 `FUN_00679A10` — the master ReadINI dispatcher

Verified by decompilation. Its structure is a fixed sequence of vtable-dispatched
`ReadINI(INIClass*)` calls, by type category, each iterating a global array:

```c
// In ReadINI order inside FUN_00679A10 — param_1 = rulesmd CCINIClass*:

for (i = 0; i < country_count;     i++) HouseType[i]    ->ReadINI(ini);  // DAT_00A83C9C
for (i = 0; i < warhead_count;     i++) Warhead[i]      ->ReadINI(ini);  // DAT_00A8E334
for (i = 0; i < animtype_count;    i++) AnimType[i]     ->ReadINI(&artmd); // art ini
for (i = 0; i < building_count;    i++) BuildingType[i] ->ReadINI(ini);
for (i = 0; i < infantry_count;    i++) InfantryType[i] ->ReadINI(ini);
for (i = 0; i < unit_count;        i++) UnitType[i]     ->ReadINI(ini);
for (i = 0; i < aircraft_count;    i++) AircraftType[i] ->ReadINI(ini);
... (VoxelAnim, Overlay, Particle, ParticleSystem, Terrain, Smudge, SuperWeapon, ...)
// Then a follow-up pass:
for (i = 0; i < voxelanim_count;   i++) FUN_007729F0();       // post-process
for (i = 0; i < building_count;    i++) FUN_00465CB0();       // post-process
... (Tiberiums, Movies, missions iter)
```

Two consequences for country parsing:

1. **`HouseTypeClass::ReadINI` runs *first* in the dispatcher order**, before BuildingType
   / UnitType / InfantryType / AircraftType ReadINIs. This means when a country's
   `VeteranUnits=MTNK` is parsed, `MTNK`'s own `[MTNK]` section has NOT yet been
   ReadINI'd — only its shell exists (registered by `FUN_00672360`). The token
   `UnitTypeClass::FindOrCreate` call resolves to the registered shell pointer, which
   later gets its fields filled in. By the time the sim reads
   `HouseTypeClass.VeteranUnits[i]` on spawn, the type is fully initialized.

2. **Country ReadINI runs AFTER `[General]`** (`RulesClass::ReadGeneral` is step 24,
   immediately before `FUN_00679A10` at step 25). Any global default parsed in `[General]`
   is available to country sections — though in practice the country's constructor already
   set every default to `1.0`, so this rarely matters.

This closes the open question from the first-pass report. The vtable dispatcher pattern is
the same one used by every other rules type — there is no country-specific code path for
invoking ReadINI.

---

## 8. Country identity at runtime — INDEX, not name

Every RA2 country has **three** identifiers on HouseTypeClass:

| Field | Offset | Meaning | Used for |
|-------|-------:|---------|----------|
| `ID` / `Name` | `+0x24` | 64-byte section-name string (e.g. `"Americans"`) | Initial INI lookup; `FindByName`; `WriteINI` |
| `UIName` | `+0x64` | 52-byte display name (CSF key, e.g. `"Name:Americans"`) | UI rendering; also compared in `FindByName` |
| **self-index** | `+0xB8` | int — position in the global array `DAT_00A83C9C` | **All runtime checks** |

Runtime code uses the **self-index exclusively**. Every one of the following is encoded by
index, not name:

- **Owner / RequiredHouses / ForbiddenHouses / SecretHouses bitmasks** on every
  TechnoTypeClass: `(1 << country_index)` — see `OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` §1.
- **Taunt file selection** — hardcoded switch on index 0..9 (§5.1).
- **Alliance bitfield** on HouseClass `+0x5788` — same bit-per-slot encoding.
- **`CountryListIndex` save-format** — the country is referenced in serialized state by its
  array index (or by its name string, re-resolved on load; the two must stay 1:1 because the
  bitmasks are not remapped).

**Consequence for lockstep:** All clients must register `[Countries]` in identical order.
YR does not rename, renumber, or reindex countries after registration. The 32-bit bitmask
space allows up to 32 countries (`index & 0x1F`); the vanilla file uses 14 (see §9).

**Consequence for save/load:** HouseClass stores a raw pointer to HouseTypeClass at `+0x34`.
On save, that pointer is not serializable; either the name or the index must be persisted.
`HouseTypeClass::WriteINI` (`0x00512170`) uses the **section name** as the key — so the
persistent identifier is the name string. On re-load, `FindByName` resolves it back to a
pointer, and the index is reconstructed from the array position. If the `[Countries]` INI
is reordered between save and load, the indices shift but the names still resolve — which
means bitmasks stored elsewhere would be **mis-interpreted** (Americans' bit 0 becoming
whichever country is now at index 0). In practice rulesmd.ini is frozen between save and
load, so this is not a live concern, but it is a landmine for modding.

**Implementation note for Rust:** mirror this discipline. Use a single `CountryIdx(u8)` as
the in-memory identity, derived from the `[Countries]` insertion order. Keep the name only
for INI lookup and display. Do not sort, renumber, or re-key the list at any point.

---

## 9. Vanilla [Countries] list (14 entries) and what is playable

```ini
[Countries]
0=Americans       ; Multiplay=yes    SideIndex=0 Allied     Prefix='G' Color=Gold
1=Alliance        ; Multiplay=yes    SideIndex=0 Allied     Prefix='G' Color=Gold
2=French          ; Multiplay=yes    SideIndex=0 Allied     Prefix='G' Color=Gold
3=Germans         ; Multiplay=yes    SideIndex=0 Allied     Prefix='G' Color=Gold
4=British         ; Multiplay=yes    SideIndex=0 Allied     Prefix='G' Color=Gold

5=Africans        ; Multiplay=yes    SideIndex=1 Soviet     Prefix='B' Color=DarkRed
6=Arabs           ; Multiplay=yes    SideIndex=1 Soviet     Prefix='B' Color=DarkRed
7=Confederation   ; Multiplay=yes    SideIndex=1 Soviet     Prefix='B' Color=DarkRed
8=Russians        ; Multiplay=yes    SideIndex=1 Soviet     Prefix='B' Color=DarkRed

9=YuriCountry     ; Multiplay=yes    SideIndex=2 Yuri       Prefix='B' Color=DarkRed

10=GDI            ; *** NOT Multiplay — Tiberian Sun holdover, dormant in YR ***
11=Nod            ; *** NOT Multiplay — Tiberian Sun holdover, dormant in YR ***

12=Neutral        ; MultiplayPassive=true  SideIndex=3 Civilian  Prefix='C' Color=Grey
13=Special        ; MultiplayPassive=true  SideIndex=4 Mutant    Prefix='J' Color=Grey
```

Playable countries: **indices 0–9** (`Multiplay=yes`). Indices 10–11 are TS legacy — they
exist so the engine can load TS campaign content but have no YR skirmish role. Indices 12
(Neutral) and 13 (Special) are the civilian/map-entity owners.

In vanilla rulesmd.ini, every playable country has every multiplier = 1.0 (nearly all
multiplier keys are commented out with `;`). The seven global doubles and nineteen
per-category floats are all 1.0. Countries differ **only** in `Side=` (and thus `SideIndex`,
EVA set, sidebar chrome), plus taunt prefix (by index) and the nominal `UIName`/`Name`/
`Prefix`/`Suffix`/`Color` strings. Per-country unit differentiation comes from the
TechnoTypeClass `RequiredHouses=` bitmask, NOT from HouseTypeClass.

**Implication for parity:** a correct Rust implementation of HouseTypeClass for vanilla can
ship with all multipliers hardcoded to `1.0` and still produce bit-for-bit identical
combat. The moment someone loads a mod that sets any multiplier, the full pipeline has to
work. Given the low implementation cost, stub all 26 multipliers as INI-parsed fields from
day one; the consumer sites are already documented.

---

## 10. Neutral and Special house creation

`ScenarioClass::Create_Houses` at `0x00687F10` (see `HOUSE_CREATION_COLOR_SYSTEM.md` §3)
creates the Neutral and Special houses as the final two HouseClass instances, **after** all
human and AI slots. For each:

```c
HouseClass* h = new (operator_new(0x160B8)) HouseClass(
    CountryTypeClass_FindByName("Neutral"));   // §3 global array lookup
h->ColorSchemeIndex = ColorScheme_FindByName("Neutral");
House::InitColor(h);
// NO Set_Credits_And_Color, NO ComputeRemap, NO SetDifficulty
```

So Neutral and Special are **not** special-cased in the class — they are regular
HouseTypeClass entries at indices 12 and 13 in vanilla, found by name lookup. What makes
them behave as "civilian" is:

- `MultiplayPassive=true` (`+0x1A6`) — excluded from defeat detection, alliance wins.
- `Side=Civilian` (index 3) — triggers the `HouseClass::Is_Enemy` MP civilian rule
  (`SIDECLASS_GHIDRA_REPORT.md` §6.1: civilian is never hostile in MP).
- Color scheme "Neutral" / "Special" (grey), found via a separate
  `ColorScheme_FindByName`-with-type-filter path (color schemes with `TypeFlag==1` are
  reserved for these; see `HOUSE_CREATION_COLOR_SYSTEM.md` §5).

**There is no shipped "default" country entry separate from `[Countries]`.** Every house —
including Neutral, Special, and even campaign "bad guys" — resolves its HouseTypeClass
through the same `FindOrCreate` → `FindByName` path.

**Defensive case:** if `[Countries]` is missing a `Neutral` or `Special` entry, the
Create_Houses lookup returns `-1`, `DAT_00A83C9C[-1]` is UB, and the game crashes. All
shipped rulesmd.ini variants therefore include both — they are load-bearing.

---

## 11. Field → runtime consumer map (final table)

For every HouseTypeClass INI-parsed field, where it is consumed:

| Field | Consumer | Address | Path |
|-------|----------|---------|------|
| `UIName`, `Name` | Sidebar, CSF lookup, `FindByName` | multiple | AbstractTypeClass base |
| `ParentCountry` | **nowhere** | — | Dead field (§2.3) |
| `Side` | `SideClass::FindOrCreate_FromKey` | `0x004756F0` | Writes `+0xBC`; used by Is_Enemy, EVA, sidebar theme |
| `Color` | `InitColor`, sidebar draw | `0x0050B840` | Palette scheme lookup |
| `Prefix` | `FUN_005F96B0` (NewTheater substitution) | — | Theater asset-name mangling; vanilla default `'A'` disables it |
| `Suffix` | Art lookup fallback | — | Mostly vestigial in vanilla (not referenced in core draw paths) |
| `Multiplay` | Lobby dropdown filter | `FUN_005E9...` (lobby UI) | Only Multiplay=yes countries appear |
| `MultiplayPassive` | Defeat detection | `MPlayer_Defeated` in Update | Passive houses excluded |
| `WallOwner` | Wall ownership on capture | `BuildingClass::ChangeOwner` | Default true; TS-era flag, still read |
| `SmartAI` | AI branch selection | multiple | Per-country AI behaviour flag |
| `Firepower` | **HouseClass::SetDifficulty** (MP only) | `0x004F6EC0` | Bakes into HouseClass+0x188 × difficulty |
| `Groundspeed` | `SetDifficulty` (MP) | same | HouseClass+0x190 |
| `Airspeed` | `SetDifficulty` (MP) | same | HouseClass+0x198 |
| `Armor` | `SetDifficulty` (MP) | same | HouseClass+0x1A0 |
| `ROF` | `SetDifficulty` (MP) | same | HouseClass+0x1A8 |
| `Cost` | `SetDifficulty` (MP) | same | HouseClass+0x1B0 (yes — prior docs swapped `Cost` and `BuildTime` labels) |
| `BuildTime` | `SetDifficulty` (MP) | same | HouseClass+0x1B8 |
| `ArmorInfantryMult` | `HouseClass::GetArmorBonus` | `0x0050BD30` | RTTI=0x10 (infantry) path |
| `ArmorUnitsMult` | `GetArmorBonus` | same | RTTI=0x07 non-naval |
| `ArmorAircraftMult` | `GetArmorBonus` | same | RTTI=0x03 |
| `ArmorBuildingsMult` | `GetArmorBonus` | same | RTTI=0x28 |
| `ArmorDefensesMult` | `GetArmorBonus` | same | RTTI=0x07 naval |
| `CostInfantryMult` .. `CostDefensesMult` | `HouseClass::GetCostBonus` | `0x0050BDF0` | Applied in `GetBuildCost` (`0x006F47A0`) |
| `SpeedInfantryMult` | `HouseClass::GetSpeedBonus` | `0x0050C050` | Infantry locomotion |
| `SpeedAircraftMult` | `GetSpeedBonus` | same | Aircraft locomotion |
| `SpeedBuildingsMult` | `GetSpeedBonus` | same | Only Infantry/Aircraft/Building cased; vehicles use global Groundspeed (§2 prior doc) |
| `BuildTimeInfantryMult` .. `BuildTimeDefensesMult` | `HouseClass::GetBuildTimeBonus` | `0x0050C0A0` | Factory step-rate divisor |
| `IncomeMult` | `HouseClass::Add_Tiberium_Credits` | `0x004F9610` | `credits += tibVal * IncomeMult * amount` |
| `VeteranInfantry` | `InfantryClass::InitFromType` | `0x00517CC0` | Linear scan → SetVeteran(1) |
| `VeteranUnits` | `UnitClass::Constructor` | `0x007353C0` | Same |
| `VeteranAircraft` | `AircraftClass::InitFromType` | `0x00413F80` | Same |

---

## 12. Open questions (MEDIUM or lower confidence)

1. ~~`HouseClass+0x2C0` / `+0x2BF` in the veteran fallback~~ **Resolved — see
   `HOUSECLASS_STOLEN_TECH_AUDIT.md`.** The five flags at `+0x2BC..+0x2C0` are:
   `+0x2BC = StolenThirdTech`, `+0x2BD = StolenSovietTech`, `+0x2BE = StolenAlliedTech`,
   `+0x2BF = InfantryVeteranBonus` (set when the house spies a Barracks —
   `BuildingType.Factory = InfantryType`), `+0x2C0 = VehicleVeteranBonus` (set when
   the house spies a War Factory — `BuildingType.Factory = UnitType`). The veteran
   fallback paths read `+0x2BF` / `+0x2C0` (not stolen-tech flags) and gate against
   `TechnoTypeClass.Trainable=` at `+0xC8E`. This is vanilla YR's documented
   "spy-on-factory → produced units spawn Veteran" mechanic. The prior
   `OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` doc had the stolen-tech offsets
   off-by-one (listed them at `+0x2BD/+0x2BE/+0x2BF`).

2. ~~`TechnoTypeClass+0xC8E`~~ **Resolved — it is `Trainable=`** (bool, default yes).
   String `"Trainable"` at `0x00843974`, xref to TechnoTypeClass::ReadINI at `0x007149E4`.
   See `HOUSECLASS_STOLEN_TECH_AUDIT.md` §6. Vanilla disables it on Engineers, Spies,
   Chrono Legionnaires, most civilians, dogs, and some support units.

3. ~~Per-country `ReadINI` dispatcher address.~~ **Resolved (spot-check pass):** the
   dispatcher is `FUN_00679A10`, called from the rules-load master at step 25 (immediately
   after `RulesClass::ReadGeneral`). See §7.1.

4. **Campaign per-map HouseClass `TechLevel=`, `PlayerControl=` etc. path for HouseTypeClass
   fields** — the prior `HOUSE_CREATION_COLOR_SYSTEM.md` covers `Read_Scenario_INI` at
   `0x00500B40`, which reads scenario-level INI onto HouseClass (not HouseTypeClass). There
   is no equivalent map-level override of HouseTypeClass fields; the type is shared by all
   houses of the same country across campaign and skirmish.

---

## 13. Implementation status vs Rust engine (as of 2026-04-21)

From the prior reports and `src/` scans:

- `src/rules/ruleset.rs` — does **not** parse `[Countries]` or `[Sides]`. Country type
  records are not modeled at all.
- `src/rules/object_type.rs` — Owner / RequiredHouses / ForbiddenHouses are stored as
  `Vec<String>` and matched by string comparison. This works for correctness but does not
  mirror the 32-bit bitmask semantics, and will not preserve the hardcoded country-index
  dependencies (taunts, save-format bitmask ordering).
- `src/map/houses.rs` — parses per-map `[Houses]` section (Country/Side/Color) but does
  not link to a shared HouseTypeClass.
- `src/sim/house_state.rs` — `side_index: u8` hardcoded to 0/1/2 from string match
  (`"allied"→0`, `"soviet"→1`, `"yuri"→2`). Civilian (3) and Mutant (4) not modeled.

What is missing for parity (scoped narrowly):

1. **`CountryType` record + `[Countries]` parser**. 14 vanilla entries, 0x1B0 fields
   per entry. Insertion order is load-bearing.
2. **`SideType` record + `[Sides]` parser**. 5 entries; `Civilian` is hardcoded-lookup
   by name at 6 runtime sites.
3. **CountryIdx(u8)** newtype threaded through the 32-bit Owner/RequiredHouses/
   ForbiddenHouses/SecretHouses bitmasks. String lookups become index-resolution.
4. **`SetDifficulty` baking of the seven global doubles** on house creation. In vanilla,
   every value is 1.0; in mods, the product matters.
5. **Five per-category float accessors** (`GetArmorBonus` etc.) called from damage,
   production, and movement. In vanilla, all read 1.0.
6. **Veteran DVC scans** on unit/infantry/aircraft spawn. Vanilla YuriCountry lists one
   DVDPLT/DVDTANK each (some are commented out).
7. **Taunt dispatch**. 10 hardcoded 2-char prefixes × 8 taunt indices.
8. **Neutral and Special** HouseClass creation after skirmish players.

Because vanilla multipliers are all 1.0, the lowest-fidelity parity path is (1) + (2) +
(3) + (7) + (8). Adding (4)–(6) is zero-cost in vanilla and mod-ready. (7) is the only
system with observable behavior that breaks if `[Countries]` order drifts.

---

## Sources

**Ghidra addresses decompiled this pass:**

- `0x00511850` — HouseTypeClass::ReadINI (full parse, all 38 keys)
- `0x005113F0` — HouseTypeClass::Constructor (full; default field values)
- `0x00752B70` / `0x00752C11` — SpeechSystem::PlayTaunt (hardcoded country-index switch)
- `0x00517CC0` — InfantryClass::InitFromType (VeteranInfantry consumer)
- `0x00517A50` / `0x00517D90` — InfantryClass::Constructor + destructor
- `0x007353C0` — UnitClass::Constructor (VeteranUnits consumer)
- `0x00413F80` — AircraftClass::InitFromType (VeteranAircraft consumer)
- `0x00413D20` — AircraftClass::Constructor
- `0x006F6CA0` — TechnoClass::Unlimbo (ruled out as veteran setter)
- `0x006722F0` — `[Countries]` list parser
- `0x00668BF0` — rules master loader (load-order table in §7)
- `0x006723D0` — `[AircraftTypes]` registration (reference for dispatcher shape)
- `0x00672360` — `[VehicleTypes]` registration
- `0x00672660` — `[BuildingTypes]` registration
- `0x00679A10` — **per-type ReadINI dispatcher** (spot-check pass, §7.1)
- `0x004571E0` — BuildingClass::OnSpyInfiltrate (spot-check: StolenTech offset correction)
- `0x0050C170` — HouseClass::FindByName (spot-check: Ghidra label clarification — prior doc misattributed as "CountryTypeClass::FindIndex")

**Memory reads:**

- `0x00825470` → `"Cost\0"` ; `0x00825478` → `"ROF\0"` (confirms the two "unnamed" doubles)
- `0x0082549C` → `"ParentCountry"` ; xref: only `0x005118DA` (ReadINI) — dead field

**String searches:**

- `"SecretUnits"` (`0x0083C730`) — single xref `RulesClass::ReadGeneral` at `0x0066FA87`
- `"SecretBuildings"` (`0x0083C720`) — single xref to `ReadGeneral` at `0x0066FA54`
- `"SecretLab"` / `"SecretUnit"` / `"SecretBuilding"` / `"SecretInfantry"` — xrefs to
  `BuildingTypeClass::ReadINI` at `0x0046XXXX`
- `"SecretHouses"` (`0x00843BA4`) — single xref to `TechnoTypeClass::ReadINI` at `0x00714543`
- `"VoxFile"`, `"FlagFile"`, `"TauntFile"`, `"SideName"`, `"RandomSelectionWeight"`,
  `"AIBaseSpacing"` as per-country keys, `"VeteranBuildings"`, `"DifficultyModifier"`
  — **no string matches found**
- `"AIBasePlanningSide"` (`0x00843980`) — per-TechnoType (see Owner Bitmask doc §8)
- `"FLAGFLY.SHP"` (`0x008458F8`) — single xref `UnitClass::DrawExtras` at `0x0073D3E7`
- `"taunts\\tau??%02i.wav"` — 10 literals at `0x00846698..0x00846770`, all xref-only from
  `SpeechSystem::PlayTaunt`

**Docs cross-referenced:**

- `COUNTRY_SIDE_TYPE_CLASSES.md` (2026-03-22)
- `COUNTRY_MULTIPLIERS_APPLICATION.md` (2026-03-22)
- `SIDECLASS_GHIDRA_REPORT.md` (2026-04-19)
- `HOUSECLASS_VERIFIED_FIELD_MAP.md` (2026-03-26)
- `HOUSECLASS_CONSTRUCTOR_DETAILED.md` (2026-03-26)
- `HOUSE_CREATION_COLOR_SYSTEM.md` (2026-03-26)
- `LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md` (2026-03-26)
- `OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` (2026-03-22)

**INI verified against:** `ini/rulesmd.ini` — `[Countries]`, `[Americans]`, `[Germans]`,
`[YuriCountry]`, `[Neutral]`, `[Special]`, `[SNIPE]`, `[DESO]`, `[General]` (SecretUnits /
SecretBuildings / SecretInfantry lines). Per `CLAUDE.md`, used in-repo INI only — not
`ra2nextevolution/`.
