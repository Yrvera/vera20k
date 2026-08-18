# HouseClass — Complete Ghidra Analysis of gamemd.exe

Source: `D:\ra2mdpost\House.CPP`, `D:\ra2mdpost\Country.CPP`

HouseClass is the per-player state object. One instance per player slot (up to 8 in
multiplayer). Total size: **0x160B8 bytes (~90KB)**. Manages ownership, credits, production,
diplomacy, power, win/loss, difficulty, and AI (AI sections documented for reference only —
not implemented in this engine).

---

## Key Globals

| Address | Name | Purpose |
|---------|------|---------|
| DAT_00a8022c | HouseClass::Array | Pointer to array of all HouseClass pointers |
| DAT_00a80238 | HouseClass::Array.Count | Number of active houses |
| DAT_00a83d4c | PlayerPtr | Pointer to the local player's HouseClass |
| DAT_008871e0 | RulesClass | Singleton — game constants and balance data |
| DAT_00a8b238 | SessionClass::GameMode | 0=campaign, non-zero=multiplayer |
| DAT_00a8ef98 | InvalidCell | Sentinel "no cell" value (like None in Rust) |
| DAT_00b054d4 | ColorSchemeArray | Array of color scheme pointers |
| DAT_00a83c6c | BuildingTypeClass::Array | All building types |
| DAT_00a83ce4 | UnitTypeClass::Array | All unit types |
| DAT_00a8b21c | InfantryTypeClass::Array | All infantry types |
| DAT_00a8e34c | AircraftTypeClass::Array | All aircraft types |
| DAT_00a83e34 | FactoryClass::Array | All active factories |
| DAT_00a83e40 | FactoryClass::Array.Count | Number of factories |

---

## Complete HouseClass Field Map

### Identity & Control

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x00 | 4×4 | vtable pointers | Multiple inheritance: 4 vtable slots |
| +0x30 | 4 | HouseIndex | Player slot 0–31 |
| +0x34 | 4 | HouseTypeClass* | Pointer to CountryTypeClass (rules data) |
| +0x1d0 | 4 | IQ level | Current IQ (AI aggressiveness) |
| +0x1d4 | 4 | TechLevel | Parsed from map `[HouseName] TechLevel=` |
| +0x1e0 | 4 | StartingEdge | Map edge index for spawning |
| +0x1e4 | 4 | AI build state | 0/1/2 state machine (AI only) |
| +0x1e8 | 4 | SideIndex | 0=Allied, 1=Soviet, 2=Yuri |
| +0x1ec | 1 | IsHuman | Human-controlled flag |
| +0x1ed | 1 | PlayerControl | From map INI `PlayerControl=` |
| +0x1ee | 1 | AI active | Whether AI logic runs |
| +0x1ef | 1 | AI triggers active | Whether AI trigger evaluation runs |
| +0x1f3 | 1 | IQ threshold reached | Set when IQ >= MaxIQLevels |
| +0x15ff4 | 20 | PlayerName | 20-char player name string |
| +0x16009 | 32 | UIName | Localized display name |

### Flags & State

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x1f5 | 1 | IsDefeated | Set by MPlayer_Defeated() |
| +0x1f6 | 1 | FlagToWinPending | Waiting for borrowed time before scattering units |
| +0x1f7 | 1 | HasWon | Victory flag |
| +0x1f8 | 1 | HasLost | Defeat flag |
| +0x1fb | 1 | NeedsRebuild | AI flag for base rebuild |
| +0x1fc | 1 | ProductionChanged | Triggers AI_ManageProduction + AI_ResumeProduction |
| +0x246 | 1 | AnnounceReadyFlag | "Unit ready" announcement pending |
| +0x24b | 1 | SidebarUpdatePending | Triggers sidebar refresh |
| +0x24c | 4 | CurrentIQ | Current IQ level for AI gating |

### Economy

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x1dc | 4 | StartingCredits | INI `Credits=` value × 100 |
| +0x30c | 4 | AvailableCredits | Current balance (StartingCredits + difficulty bonus) |
| +0x2dc | 4 | TotalCreditsSpent | Cumulative spending tracker |
| +0x310 | 4 | TrackedTiberiumBalance | Tiberium/ore accounting |

**Credits internal scaling**: INI `Credits=10000` → internal value 1,000,000.
Difficulty bonus: Easy adds `RulesClass+0xdfc`, Hard adds `+0xe00` to starting credits.

### Power System

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x164 | 4 | PowerOutputUnits | Count of power-producing buildings |
| +0x168 | 4 | PowerDrainUnits | Count of power-consuming buildings |
| +0x53A4 | 4 | PowerOutputSum | Accumulated wattage from all owned power-producing buildings; zeroed and rebuilt each time AI_AssessPower runs (corrected 2026-07-18: the 2026-05-28 "correction" to +0x5384 was ITSELF WRONG — that session read the correct decimal byte offset 21412 from `get_struct_layout HouseClass` but mis-converted it to hex as 0x5384 instead of 0x53A4 (21412 decimal = 0x53A4, not 0x5384); independently re-verified this session via `get_struct_layout HouseClass` (field `PowerOutput` at byte 21412) plus fresh `decompile_function 0x508c30` (`HouseClass__AI_AssessPower`, uses typed `this->PowerOutput`), `decompile_function 0x4fce30` (`HouseClass__GetPowerRatio`), `decompile_function 0x508df0`, and `decompile_function 0x508f60` — all four independently resolve the same field to +0x53A4 — ROOT_CAUSE: OFFSET_RETYPED_WRONG (decimal-to-hex transcription error, not a struct-identity error)) |
| +0x53A8 | 4 | PowerDrainSum | Accumulated drain wattage from all owned power-consuming buildings; zeroed and rebuilt each time AI_AssessPower runs (corrected 2026-07-18: the 2026-05-28 "correction" to +0x5388 was ITSELF WRONG — same root cause as PowerOutputSum above: decimal byte offset 21416 mis-converted to hex 0x5388 instead of 0x53A8; independently re-verified this session via `get_struct_layout HouseClass` (field `PowerDrain` at byte 21416) and the same four decompiles cited above — ROOT_CAUSE: OFFSET_RETYPED_WRONG) |

Power ratio function (0x4fce30):
```
if power_used < power_total && power_total != 0:
    return power_used / power_total   (float ratio)
if power_used == 0:
    return 0.0
else:
    return 1.0  (full power or over-drain)
```

Power output/drain per building come from TypeClass offsets:
- `+0xE08` = PowerOutput
- `+0xEE0` = EnergyDrain

### Diplomacy / Alliances

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x1d8 | 4 | RadarShareBitfield | Which houses share radar |
| +0x5788 | 4 | AllianceBitfield | 32 bits, one per house slot |

**IsAlliedWith** (0x4f9a50, 89 callers — most-called alliance check):
```c
bool IsAlliedWith(HouseClass* this, HouseClass* other) {
    if (other == NULL) return false;
    if (other == this) return true;
    int idx = other->HouseIndex;  // +0x30
    if (idx == this->HouseIndex) return true;
    if (idx == -1) return false;
    return (this->AllianceBitfield & (1 << idx)) != 0;
}
```

**MakeAlly** (0x4f9b70): Sets bit in AllianceBitfield, plays "EVA_AllianceFormed",
clears targeting on all owned units that were targeting the new ally (vtable+0x3c8 with
arg 0). Sends radar event. In multiplayer also sets radar share bit at +0x1d8.

**BreakAlliance** (0x4f9f90): Clears bit reciprocally in both houses. Plays
"EVA_AllianceBroken", displays "TXT_AT_WAR".

### Owned Object Tracking

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x6c | 4 | OwnedObjectsArray | TechnoClass*[] pointer |
| +0x78 | 4 | OwnedObjectsCount | Number of owned objects |
| +0x144 | 4 | OwnedUpgradesArray | Upgrade building pointers |
| +0x150 | 4 | OwnedUpgradesCount | |
| +0x158 | 4 | SpySatCount | Number of SpySat uplinks owned |
| +0x15c | 4 | CloakDeviceCount | Number of gap generators owned |
| +0x170 | 4 | GarrisonStructuresArray | AI garrison tracking |
| +0x17c | 4 | GarrisonCount | |
| +0x2d8 | 4 | RobotControlCount | Reference-counted robot control centers |
| +0x2e8 | 4 | InfantryCount | Total owned infantry |
| +0x2f0 | 4 | BuildingCount | Total owned buildings |
| +0x2f4 | 4 | AircraftCount | Total owned aircraft |
| +0x2f8 | 4 | VehicleCount | Total owned vehicles |

When units are gained (`Added_To_Game`, 0x502a80) or lost (`Removed_From_Game`, 0x5025f0),
the corresponding counter is incremented/decremented and power/cost totals updated.

Robot tank system: when RobotControlCount transitions 1→0, all robot tanks are disabled
with EVA "EVA_RobotTanksOffline". When 0→1, re-enabled with "EVA_RobotTanksBackOnline".

### Production System

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x210 | 4×12 | FactorySlots | 12 max production slots |
| +0x20c | 4 | CurrentFactoryIndex | Active slot |
| +0x258 | 4 | ProductionQueueArray | FactoryClass*[] pointer |
| +0x264 | 4 | ProductionQueueCount | |
| +0x53ac | 4 | InfantryFactory* | Active infantry factory |
| +0x53b0 | 4 | AircraftFactory* | Active aircraft factory |
| +0x53b4 | 4 | UnitFactory* | Active unit factory |
| +0x53b8 | 4 | NavalFactory* | Active naval factory |
| +0x53bc | 4 | BuildingFactory* | Active building factory |
| +0x53cc | 4 | BuildingFactoryAlt* | Alt building factory (defense) |
| +0x5378–0x5388 | 4×5 | QueuedProductionCounts | Per-RTTI queued item counts |
| +0x5390–0x53a0 | float×5 | BuildSpeedBonuses | Infantry/Naval/Air/Vehicle/VehicleAlt |

**CanBuild** (0x4f7870, 407 lines decompiled):
- Takes TechnoTypeClass* and two flags
- Checks prerequisites from type+0x195 via negative index groups:
  - -1 = PrerequisitePower (power buildings)
  - -2 = PrerequisiteBarracks (infantry factories)
  - -3 = PrerequisiteRadar (radar buildings)
  - -4 = PrerequisiteTech (tech centers)
  - -5 = PrerequisiteProc (refineries)
  - -6 = PrerequisiteAny (any factory + ConYard)
- Checks house ownership masks at type+0x368/0x369
- Checks tech level
- BuildLimit logic (type+0xEE): switch by RTTI (3=Building, 7=Infantry, 0x10=Vehicle, 0x28=Aircraft)
  - Counts existing units of that type via FUN_0049fae0
  - If negative BuildLimit: treats as absolute cap
  - Returns: 1 (can build), 0 (missing prereqs), -1 (at build limit)

**Begin_Production** (0x4fa350): Allocates 0x74-byte FactoryClass, queues production.
**Place_Production** (0x4fb0e0): Places completed building or exits unit from factory.
**Abandon_Production** (0x4c9ff0): Refunds `(cost - amountSpent)`, clears chosen type.

### Win/Loss System

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x298 | 4 | WinLossStartFrame | Frame when win/loss triggered |
| +0x2a0 | 4 | BorrowedTimeFrames | Multiplayer grace period |

**MPlayer_Defeated** (0x4fc0b0, 1559 bytes):
- Sets +0x1f5 = 1 (defeated)
- Local player: disables UI, fades screen, plays "EVA_YouHaveLost"
- Opponents: logs "Opponent %s has been defeated", plays "EVA_PlayerDefeated"
- Checks if all remaining players are allied → triggers game over
- Handles observer mode, co-op, borrowed-time

**Flag_To_Win** (0x4fc9e0): Sets +0x1f7=1. Campaign: "EVA_MissionAccomplished".
Skirmish: "EVA_YouAreVictorious".

**Flag_To_Lose** (0x4fcbd0): Sets +0x1f8=1. Campaign: "EVA_MissionFailed".
Skirmish: "EVA_YouHaveLost".

### Difficulty System

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x184 | 4 | DifficultyLevel | 0=Easy, 1=Medium, 2=Hard |
| +0x188 | 8 | Firepower | double — difficulty multiplier |
| +0x190 | 8 | Armor | double |
| +0x198 | 8 | ROF | double (rate of fire) |
| +0x1a0 | 8 | GroundSpeed | double |
| +0x1a8 | 8 | AirSpeed | double |
| +0x1b0 | 8 | BuildSpeed | double |
| +0x1b8 | 8 | Cost | double |
| +0x1c0 | 8 | RepairDelay | double (copied directly, no scaling) |
| +0x1c8 | 8 | BuildDelay | double (copied directly, no scaling) |

**SetDifficulty** (0x4f6ec0):
- Reads 9 difficulty doubles from `RulesClass + difficulty * 0x50 + 0x1538` (Firepower,
  Armor, ROF, GroundSpeed, AirSpeed, BuildSpeed, Cost, RepairDelay, BuildDelay — corrected
  2026-07-18: was "7"; the doc's own field table two sections above already lists all 9,
  this bullet just undercounted)
- In singleplayer: copies directly, with NO `RulesClass+0x1418` multiply
  (**Firepower, GroundSpeed, AirSpeed, BuildSpeed**), or multiplies by the global
  `RulesClass+0x1418` factor (**Armor, ROF, Cost**) (corrected 2026-07-18: this doc
  previously had ROF and the {GroundSpeed, AirSpeed, BuildSpeed} group EXACTLY SWAPPED
  between the two categories — independently re-verified via `decompile_function
  0x4f6ec0`, which shows ROF at `param_1+0x198` multiplied by `RulesClass+0x1418` in the
  singleplayer branch, while GroundSpeed/AirSpeed/BuildSpeed at `+0x1a0/+0x1a8/+0x1b0` are
  raw 8-byte copies with no such multiply — ROOT_CAUSE: INFERENCE_HARDENED, the grouping
  was asserted without checking each field's arithmetic individually)
- In multiplayer: additionally multiplies each by the HouseTypeClass's per-country
  modifier at +0xC8..+0xF8 (e.g., Korea's Firepower × difficulty Firepower); confirmed via
  the same decompile that Firepower/GroundSpeed/AirSpeed/BuildSpeed gain ONLY the
  per-country factor in multiplayer (still no +0x1418 term), while Armor/ROF/Cost gain
  both the +0x1418 factor AND the per-country factor
- RepairDelay and BuildDelay copied without country scaling (confirmed unchanged in both
  singleplayer and multiplayer branches via the same decompile)

### Timers

All timers use the pattern: `[start_frame, unused, duration]` where the timer has
expired when `current_frame - start_frame >= duration`. Start frame of -1 means timer
is inactive.

| Offset | Timer | Purpose |
|--------|-------|---------|
| +0x2a4–0x2ac | Speech timer | When to recheck power/superweapons |
| +0x2b0–0x2b8 | Announcement timer | When to recheck notifications |
| +0x280–0x288 | Generic timer | |
| +0x5634–0x563c | AI strategy timer | When to run AI_Building_Strategy |
| +0x5798–0x57a0 | AI trigger timer | Duration from RulesClass difficulty table |
| +0x57a4–0x57ac | Announce ready timer | "Unit ready" cooldown (0x5a = 90 frames) |
| +0x57bc–0x57c4 | Low power warning timer | |
| +0x57d4–0x57dc | Money warning timer | |

### Rally Points & Base Center

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x53dc | 4 | RallyPointObject | Object pointer for rally target |
| +0x53e0 | 4 | RallyPointCell | CellStruct (2 shorts) |
| +0x5490 | 4 | BaseCenterCell | Primary base center |
| +0x5494 | 4 | AltBaseCenterCell | Alternate center |
| +0x5498 | 4 | BaseSpreadRadius | Minimum 0x200 |
| +0x54d8 | 4 | BuildCooldownTimer | |
| +0x54f0 | 4 | PrimaryRallyCell | |
| +0x54f4 | 4 | SecondaryRallyCell | With timeout at +0x54fc |
| +0x54fc | 4 | SecondaryRallyFrame | -100 (0xffffff9c) = expired |

### House Color

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x16054 | 4 | ColorSchemeIndex | Index into DAT_00b054d4 scheme array |
| +0x56f9 | 3 | HouseColorRGB | Extracted from palette scheme |
| +0x56fc | 3 | HouseBrightRGB | Normalized "bright" remap color |

**InitColor** (0x50b840): Reads scheme from `DAT_00b054d4[ColorSchemeIndex]`.
Extracts RGB from palette entry using bit shift tables at
`DAT_008a0dd0`–`DAT_008a0de4`. Forces WHITE (index 5) if color scheme missing,
with debug message: "Forcing House %s [%s] to color WHITE".

**ComputeRemap** (0x50ba00): Reads +0x56f9 RGB, normalizes vector to length 255
via NormalizeRGBVector (0x50b920), stores bright version at +0x56fc.

### Threat & Enemy Tracking

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x5600 | 4 | EnemyHouseIndex | Primary threat (-1 = none) |
| +0x5608 | 4 | GrudgeListPtr | 8-byte entries: [house_ptr, score] |
| +0x5614 | 4 | GrudgeListCount | |
| +0x57e4 | 4×0x4204 | ThreatMapGrid | ~130×130 cell threat values |
| +0x1609C | float | AIInfantryRatio | Default 0.33 |
| +0x160A0 | float | AIVehicleRatio | Default 0.33 |
| +0x160A4 | float | AIAircraftRatio | Default 0.33 |
| +0x160A8 | 4 | TrackedAircraftValue | Enemy tracking |
| +0x160AC | 4 | TrackedInfantryValue | Enemy tracking |
| +0x160B0 | 4 | TrackedGeneralValue | Enemy tracking |
| +0x160B4 | 4 | AICostTolerance | |

### Build Queue & Base Plan

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x564c | 4 | ChosenBuildingType | -1 = none |
| +0x5650 | 4 | ChosenUnitType | -1 = none |
| +0x5654 | 4 | ChosenAircraftType | -1 = none |
| +0x5658 | 4 | ChosenInfantryType | -1 = none |
| +0x565c | 4 | RatioAITriggerTeam | From map INI |
| +0x5660 | 4 | RatioTeamAircraft | Default 75 (0x4b) |
| +0x5664 | 4 | RatioTeamInfantry | Default 75 |
| +0x5668 | 4 | RatioTeamUnits | Default 75 |
| +0x5704 | 4 | BasePlanVtable | |
| +0x5708 | 4 | BasePlanNodeArray | 16 bytes per node |
| +0x570c | 4 | BasePlanCapacity | |
| +0x5714 | 4 | BasePlanNodeCount | |
| +0x5718 | 4 | BasePlanGrowIncrement | |
| +0x5750 | 4 | BasePlanCenterCell | |
| +0x5754–0x5770 | 4×8 | BaseBoundingRect | Visible + expanded area |

Base plan node format (16 bytes): `[TypeIndex:4, CellStruct:4, Flags:4, Reserved:4]`

### Miscellaneous

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x48 | 4 | ScreenFlashCount | |
| +0x5774 | 4 | SelfPointer | Debug? Stores `this` |
| +0x5778 | 1 | SpeechPending (Ghidra: RecheckPower) | Cleared by `AI_AssessPower` (0x508c30) each time power totals are rebuilt |
| +0x5779 | 1 | AnnouncementPending (Ghidra: RecheckRadar) | Triggers the radar/tacmap gate (0x508df0) + SpySat shroud scan (0x508f60) below — NOT a superweapon check (corrected 2026-07-18: see corrected CheckSuperweaponReady/CheckLowPower sections — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| +0x577a | 1 | LowPowerState (SpySatActive) | Ghidra struct DB names this field `SpySatActive`, not a generic low-power flag; toggled by `HouseClass__CheckLowPower` (0x508f60) around a SpySat-gated shroud blackout/restore, not general power state (corrected 2026-07-18: verified via `get_struct_layout HouseClass` field `SpySatActive` at byte 22394=0x577A, and `decompile_function 0x508f60` which reads/writes `this->SpySatActive` exclusively — see corrected CheckLowPower section below — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| +0x577b | 1 | HasOffensiveUnits | |
| +0x577c | 1 | EdgeDirection | 0=N, 1=E, 2=S, 3=W |

---

## FactoryClass (0x74 = 116 bytes)

One FactoryClass per active production slot. Created by Begin_Production, destroyed
when production completes or is abandoned.

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x24 | 4 | Progress | 0 → 0x36 (54) = complete |
| +0x28 | 4 | TechnoTypeClass* | Type being produced |
| +0x2c | 4 | StartFrame | When production began |
| +0x38 | 4 | StepTimer | Accumulated cost/time |
| +0x44 | 4 | QueueArrayPtr | Secondary queue |
| +0x48 | 4 | QueueCapacity | |
| +0x50 | 4 | QueueCount | |
| +0x58 | 4 | ProducedObject* | TechnoClass* when complete |
| +0x5d | 1 | ActiveFlag | |
| +0x60 | 4 | CreditsSpentSoFar | For refund calculation |
| +0x68 | 4 | AlternateProductionID | -1 = none |
| +0x6c | 4 | OwnerHouse* | |
| +0x70 | 1 | IsBusy | |

**IsComplete** (0x4ca130): Returns true when Progress == 0x36 AND (ProducedObject != NULL
OR AlternateProductionID != -1).

**AbandonProduction** (0x4c9ff0): Refunds `cost - CreditsSpentSoFar` to owner house.
Clears chosen type indices on owner (+0x5650/5654/5658/564c = -1). Destroys produced
object. Debug: "Abandoning production of %s".

**StartProduction** (0x4c9c70): If queue at capacity and build-limited, plays screen
flash for human player and returns false. Otherwise appends to queue or starts
production with fresh timer values. For AI houses, marks buildings with +0x6ca flag.

---

## CountryTypeClass (HouseTypeClass) — 0x1B0 bytes

Per-faction type data parsed from `rulesmd.ini` sections like `[Americans]`, `[Soviet]`.
Constructor at 0x5113f0, INI reader at 0x511850.

### Identity

| Offset | Type | INI Key / Purpose |
|--------|------|-------------------|
| +0x24 | char[] | Section name (country name) |
| +0x64 | char[] | Alternate name (offset 100) |
| +0x98 | char[25] | ParentCountry= |
| +0xb8 | int | Self-index in global country array |
| +0xbc | int | Parent country index (-1 = none) |
| +0xc0 | int | Color= (scheme index) |

### Stat Multipliers (doubles, default 1.0)

Applied per-country in multiplayer via SetDifficulty.

| Offset | Type | INI Key |
|--------|------|---------|
| +0xc8 | double | Firepower= |
| +0xd0 | double | Groundspeed= |
| +0xd8 | double | Airspeed= |
| +0xe0 | double | Armor= |
| +0xe8 | double | (unnamed multiplier 5 — likely ROF) |
| +0xf0 | double | (unnamed multiplier 6 — likely Cost) |
| +0xf8 | double | BuildTime= |

### Per-Category Multipliers (floats, default 1.0f)

Fine-grained multipliers applied to specific object categories.

| Offset | Type | INI Key |
|--------|------|---------|
| +0x100 | float | ArmorInfantryMult= |
| +0x104 | float | ArmorUnitsMult= |
| +0x108 | float | ArmorAircraftMult= |
| +0x10c | float | ArmorBuildingsMult= |
| +0x110 | float | ArmorDefensesMult= |
| +0x114 | float | CostInfantryMult= |
| +0x118 | float | CostUnitsMult= |
| +0x11c | float | CostAircraftMult= |
| +0x120 | float | CostBuildingsMult= |
| +0x124 | float | CostDefensesMult= |
| +0x128 | float | SpeedInfantryMult= |
| +0x12c | float | SpeedUnitsMult= |
| +0x130 | float | SpeedAircraftMult= |
| +0x134 | float | BuildTimeInfantryMult= |
| +0x138 | float | BuildTimeUnitsMult= |
| +0x13c | float | BuildTimeAircraftMult= |
| +0x140 | float | BuildTimeBuildingsMult= |
| +0x144 | float | BuildTimeDefensesMult= |
| +0x148 | float | IncomeMult= |

### Veteran Unit Lists

| Offset | Type | INI Key |
|--------|------|---------|
| +0x158 | DVC | VeteranInfantry= (type pointer list) |
| +0x178 | DVC | VeteranUnits= (type pointer list) |
| +0x194 | DVC | VeteranAircraft= (type pointer list) |

### Flags

| Offset | Type | INI Key |
|--------|------|---------|
| +0x1a0 | char[5] | Suffix= (4-char theater suffix) |
| +0x1a4 | char | Prefix= (1-char asset prefix, default 'A') |
| +0x1a5 | bool | Multiplay= (available in skirmish/MP) |
| +0x1a6 | bool | MultiplayPassive= (passive in MP, no AI) |
| +0x1a7 | bool | WallOwner= |
| +0x1a8 | bool | SmartAI= |

**FindByName** (0x5117d0, 23 callers): Searches global country array. Handles
`"<random>"` as special sentinel (returns 0xFFFFFFFE).

---

## HouseClass::Update() — Per-Frame Tick

Address: 0x004f8440, 3879 bytes, 678 lines decompiled.

### Execution order each frame:

1. **Timer decay**: Speech timer (+0x2a4) and announcement timer (+0x2b0)
2. **Power assessment**: If speech pending → `AI_AssessPower()` (sums attack/defense)
3. **Radar/tacmap gate + SpySat shroud check**: If announcement pending (+0x5779) →
   `HouseClass__CheckSuperweaponReady` (0x508df0, Ghidra label — actually a radar/tactical-map
   availability gate, not a superweapon check) + `HouseClass__CheckLowPower` (0x508f60, Ghidra
   label — actually a SpySat-flagged-building shroud blackout/restore scan, not a general
   low-power check) (corrected 2026-07-18: both function names are Ghidra RTTI label drift;
   see the corrected "CheckSuperweaponReady"/"CheckLowPower" sections below — ROOT_CAUSE:
   RTTI_LABEL_DRIFT; verified via `decompile_function 0x508df0` and `decompile_function 0x508f60`)
4. **Threat decay**: Every 100 frames, decrement grudge scores in +0x5608 array
5. **AI activation**: If non-human and IQ >= MaxIQLevels, activate AI flags
6. **Win processing**: HasWon (+0x1f7) → wait borrowed time → end game
7. **Loss processing**: HasLost (+0x1f8) → wait borrowed time → end game
8. **FlagToWin processing**: +0x1f6 → wait borrowed time → scatter all units
9. **Power clamping**: Clamp +0x53A4 (PowerOutputSum) and +0x53A8 (PowerDrainSum) to >= 0 (corrected 2026-07-18: the 2026-05-28 "correction" to +0x5384/+0x5388 was itself wrong — see the Power System table above for the full root-cause explanation; re-confirmed this session via `get_struct_layout HouseClass` and `decompile_function 0x508c30`/`0x4fce30` — ROOT_CAUSE: OFFSET_RETYPED_WRONG)
10. **Rally point movement**: Every 15 frames, move units to rally cell (+0x53e0)
11. **AI trigger evaluation**: On timer expiry, process AI trigger teams (AI only)
12. **Local player checks**: Building count, money warnings, low-power EVA
13. **Multiplayer defeat detection**: Count ConYards + units, trigger defeat if zero
14. **AI building strategy**: On timer expiry → `AI_Building_Strategy()` (AI only)
15. **AI production selection**: Every 8 frames, 3-state chooser (AI only)
16. **Production tick**: Iterate all factories → `FactoryClass::Update()` per queue
17. **Production management**: If changed flag → `AI_ManageProduction()` + `AI_ResumeProduction()`
18. **Cooldown timer**: Announce-ready timer reset (90 frames = 0x5a)

### Key frame-rate patterns:
- Every frame: timers, win/loss, production tick
- Every 8 frames: AI production decisions
- Every 15 frames: rally point processing
- Every 100 frames: threat decay

---

## House::Read_Scenario_INI — Per-House Properties from Map

Address: 0x500b40. Reads per-house properties from the map's INI section.

**INI keys parsed** (in order):
1. `TechLevel` → +0x1d4 (default from RulesClass+0x1254)
2. `Credits` → +0x1dc (multiplied by 100 for internal storage)
3. `PlayerControl` → +0x1ed
4. `UIName` → +0x16009 (localized via string table)
5. `RatioAITriggerTeam` → +0x565c
6. `RatioTeamAircraft` → +0x5660 (default 75)
7. `RatioTeamInfantry` → +0x5664 (default 75)
8. `RatioTeamUnits` → +0x5668 (default 75)
9. Side index → +0x1e8 (from HouseTypeClass+0xbc)
10. `Color` → +0x16054 (forces WHITE=5 if invalid, with debug warning)
11. `Allies` → read as bitmask, calls MakeAlly for each set bit

**Color initialization**: Reads palette scheme from `DAT_00b054d4[ColorSchemeIndex]`,
extracts RGB using bit shift tables, computes sqrt of R²+G²+B² for normalization,
stores base color at +0x56f9 and bright remap at +0x56fc.

**Starting credits with difficulty bonus**:
- PlayerControl=yes in singleplayer:
  - Easy (DAT_00a8eb64=0): credits += `RulesClass+0xdfc`
  - Hard (DAT_00a8eb64=2): credits += `RulesClass+0xe00`
  - Medium: no bonus
- Credits clamped to >= 0

---

## Key Named Functions

| Address | Name | Size | Callers | Purpose |
|---------|------|------|---------|---------|
| 0x4f5190 | Constructor (simple) | — | — | Lightweight init, no spawn data |
| 0x4f54a0 | Constructor (full) | 4512 | 3 | Full init with difficulty, registers in 5 global arrays |
| 0x4f6ec0 | SetDifficulty | 636 | 4 | Copies/scales 7 difficulty doubles from RulesClass |
| 0x4f7140 | Destructor | — | — | Tears down everything, removes from all arrays |
| 0x4f7870 | CanBuild | 2804 | 13 | Full prerequisite + build limit check |
| 0x4f8440 | Update | 3879 | — | Per-frame tick (see above) |
| 0x4f93e0 | NotifyUnderAttack | — | — | EVA voice + radar ping |
| 0x4f9790 | Spend_Money | — | — | Subtract credits, sell objects if overdraft |
| 0x4f9950 | Add_Credits | — | 7 | Simple `credits += amount` |
| 0x4f9a50 | IsAlliedWith | — | 89 | Alliance bitfield check |
| 0x4f9b70 | MakeAlly | 1009 | — | Form alliance + EVA + clear targeting |
| 0x4f9f90 | BreakAlliance | — | — | Break alliance reciprocally + EVA |
| 0x4fa350 | Begin_Production | 1222 | 2 | Start building something |
| 0x4fb0e0 | Place_Production | 1426 | 1 | Place completed building / exit unit |
| 0x4fb920 | Destroy_All_Owned | — | — | On defeat: destroy all teams/triggers/house |
| 0x4fc0b0 | MPlayer_Defeated | 1559 | — | Multiplayer defeat handler |
| 0x4fc9e0 | Flag_To_Win | — | — | Victory handler |
| 0x4fcbd0 | Flag_To_Lose | — | — | Defeat handler |
| 0x4fce30 | Get_Power_Ratio | — | 9 | power_used / power_total as float |
| 0x4ff550 | Remove_Tracking | — | — | Decrement per-type counters on unit loss |
| 0x4ff700 | Add_Tracking | — | — | Increment per-type counters on unit gain |
| 0x500b40 | Read_Scenario_INI | — | — | Load per-house properties from map INI |
| 0x501160 | Write_INI | — | — | Save houses to scenario |
| 0x502d30 | Find_By_Country_Index | — | 25 | Lookup house by country type index |
| 0x50b370 | CheckBuildLimit | — | — | Enforce BuildLimit= from rules |
| 0x50b6f0 | IsHumanPlayer | — | 78 | Most-called house query in the engine |
| 0x50b730 | IsPlayerControl | — | 52 | Second most-called house query |
| 0x50b840 | InitColor | — | — | Color scheme from palette |
| 0x50bf60 | RecalcBonuses | — | — | Recompute 5-category build speed bonuses |
| 0x50c210 | RecenterBase | — | — | Find ConYard, set base center |
| 0x509400 | AI_BuildThreatMap | — | 3 | Zero + rebuild 0x4204-cell threat grid |
| 0x509700 | AI_EconomyStateMachine | — | — | 4-state FSM for AI aggressiveness |
| 0x4fd500 | AI_Building_Strategy | 3879 | — | Top-level AI strategy tick |
| 0x4fd9a0 | AI_Check_Build_Need | 819 | — | Does AI urgently need to build? |
| 0x4fdd10 | AI_Manage_Build_Queue | 1736 | — | Core build queue manager |
| 0x4fe3e0 | AI_Choose_Building | 1653 | — | Select building to construct |
| 0x4fea60 | AI_Choose_Unit | 1147 | — | Select unit type to build |
| 0x4feee0 | AI_Choose_Aircraft | — | — | Select aircraft type to build |
| 0x4ff210 | AI_Choose_Infantry | — | — | Select infantry type to build |
| 0x5098f0 | AI_DispatchProduction | — | — | Route production to handlers |
| 0x509cd0 | AI_GroundRallyPoint | — | — | Resolve rally for ground units |
| 0x509e00 | AI_NavalRallyPoint | — | — | Resolve rally for naval units |
| 0x509f60 | AI_FindInfantryTarget | — | — | Find building with most enemy infantry |
| 0x50a150 | AI_FindAirTarget | — | — | Find building target for air units |
| 0x50af10 | AI_ManageProduction | — | 5 | Start/stop builds based on conditions |
| 0x50b1d0 | AI_ResumeProduction | — | — | Re-evaluate idle queues |
| 0x5054b0 | AI_RecalcBuildOptions | 2755 | — | Full "what can I build?" recalc |
| 0x5082c0 | AI_ScanBasePerimeter | — | — | Walk base edges, place defense nodes |
| 0x506ef0 | AI_ChooseNextProduction | 3216 | — | Master production decision with threat maps |
| 0x508150 | AI_UpdateEnemyThreatRatios | — | — | Compute infantry/vehicle/aircraft ratios |
| 0x50cbf0 | AI_FindBestRallyTarget | 1406 | — | Complex rally point selection |
| 0x50d170 | AI_FindTeamTarget | — | — | Get team's current target cell |
| 0x50a5c0 | ComputerTakeover | 2375 | — | Convert human to AI control |
| 0x50d290 | TransferUnitsTo | — | — | Transfer ownership of all units |

---

## AI Subsystems (Complete)

These systems drive AI player behavior. Documented here for completeness and
future reference even though AI is not currently implemented in this engine.

### AI Economy State Machine (0x509700) — 4-state FSM

Controls AI aggressiveness based on credit balance. Only runs for non-human
houses in multiplayer.

```
State 0 (Normal):
    credits = wallet->GetBalance()
    if credits < RulesClass+0x1300 (AIPoorCreditThreshold):
        → State 1 (if RTTI==6) or State 2

State 1 (Low credits, no units):
    if credits >= threshold: → State 0
    if RTTI==6: → State 2

State 2 (Low credits, check military):
    if credits >= threshold: → State 0
    Check if house owns both building types (RulesClass+0x904 array)
    AND unit types (RulesClass+0x93c array)
    If both exist AND attack_power > defense_power:
        50% random chance to skip (stay aggressive)
    If RTTI != 6: → State 1

State 3 (Under attack):
    if credits > threshold: → State 0
```

The state value at +0x1e4 controls which AI choosers run in the Update loop.

### AI Threat Map (0x509400) — AI_BuildThreatMap

Zeros the 0x4204-entry threat grid at +0x57e4, then builds it from all
enemy units:

```
for each TechnoClass in global array (backwards):
    skip dead, inactive, or zero-sight-range objects
    if type is Aircraft(0xf), Building(1), or Infantry(2):
        cell_index = CellToMapIndex(unit.MapCell)
        skip if same house
        skip if allied (check alliance bitfield)
        // Accumulate into 9-cell neighborhood:
        for i in 0..9:
            neighbor = cell_index + OFFSET_TABLE[i]  // DAT_008243c8
            threat[neighbor] += sight_range >> SHIFT_TABLE[i]  // DAT_008243ec
            clamp to >= 0
    else (other types like Vehicle):
        cell_index = CellToMapIndex(unit.Building.MapCell)
        // Same 9-cell neighborhood accumulation
```

The offset table (DAT_008243c8) contains 9 entries for the cell itself plus
8 neighbors. The shift table (DAT_008243ec) creates distance decay: center
cell gets full value, adjacent cells get value >> 1 or >> 2.

### AI Building Strategy (0x4fd500) — Top-level AI strategy tick

Called on AI strategy timer expiry. Returns delay until next tick (random 0x6a–0x70 frames).

**Phase 1: Enemy Selection**
- If no current enemy (+0x5600 == -1) in multiplayer:
  - Get own base center position
  - Iterate all houses, find nearest non-allied, non-defeated, non-MultiplayPassive
  - Distance = sqrt(dx² + dy² + dz²) using 3D coords
  - Set enemy to closest → Update_Threat_Score (+0x5600)
- If current enemy is defeated: remove from grudge list, clear enemy

**Phase 2: Urgency Assessment**
- Urgency state at +0x250:
  - State 0 (normal): if wallet balance < 25 → State 1
  - State 1 (low money): if balance > 24 → State 0
  - State 3 (under attack): if cooldown (+0x54d8 + 900) expired → State 0
  - If current frame < cooldown + 900 → State 3

**Phase 3: Dispatch by urgency** (multiplayer only)
- Priority 4 (highest): Urgency 0 + has construction task → dispatch AI wallet
- Priority 1: AI_Check_Build_Need → AI_Manage_Build_Queue

**Phase 4: Production dispatch**
- Calls AI_DispatchProduction (0x5098f0)

### AI Check Build Need (0x4fd9a0) — Does AI need to build?

Returns 0 (human player, early exit) or 1 (AI needs buildings). Checks:
1. If house can afford tech (CanAffordToBuild)
2. If CloakDevice count < 1: find radar building from PrerequisiteRadar list
3. Look for construction yard in PrerequisiteConYard or backup list
4. If already building chosen type in factory: check cost vs wallet balance
5. Check existing buildings against needed factories

### AI Manage Build Queue (0x4fdd10, 289 lines)

Core AI build queue processing. Iterates base plan entries backwards:
1. For each entry: try to place building via FUN_0042e820
2. If placement invalid: sell via FUN_0042e780, then Sell_Building_At_Cell
3. If building ready and has upgrades (TypeClass+0x16b7): apply upgrade
4. Handle construction yard placement as special case
5. Uses memmove to shift 16-byte base plan entries when removing

### AI Choose Building (0x4fe3e0, 268 lines)

Selects which building the AI should construct next:

1. If already have a choice (+0x564c != -1): return
2. Get first base plan entry via FUN_0042eb20
3. **Naval skip**: If entry is naval type (+0xcce) and house not in naval mode,
   remove from queue and call AI_ScanBasePerimeter (0x5082c0)
4. **Special index -3**: Trigger base perimeter rescan
5. **ConYard entries (-1 or matching ConYard type)**: Random chance
   (from difficulty table at RulesClass+0xdd8) to try naval base planning
   (FUN_0050c340). Otherwise use AI_Choose_Next_Production (0x506ef0)
6. **Cost check**: If `defense_power + building_cost > attack_power - AI_cost_tolerance`:
   skip expensive buildings. Exception: ConYard types bypass cost check.
7. **No offensive units**: If HasOffensiveUnits (+0x577b) is false, insert
   a side-appropriate base defense building into the base plan queue
8. **Wall placement**: Check foundation cells for valid wall placement
9. Store chosen type index at +0x564c

### AI Choose Unit (0x4fea60, 191 lines)

Selects which unit type the AI should build:

1. If already chosen (+0x5650 != -1): return
2. **Early game detection**: Check SpySat count vs difficulty-scaled threshold.
   If below threshold AND no radar building exists AND matching robot type available:
   set +0x5650 to robot tank type and return
3. **Need counting**: For each cell in global building list:
   - Get occupants, count enemy units by type (RTTI 0x28)
   - Store counts in local_4b0[100] array
   - Track closest distance in local_320[100]
4. **Subtract in-production**: For each factory building, if producing matching
   type, decrement the need count
5. **Scoring**: For each UnitType that passes CanBuild:
   - Get cost via GetCost(this_house)
   - If wallet can afford: track as candidate
   - Pick highest-need type, or nearest if tied
6. **Random selection**: With probability from difficulty table
   (RulesClass+0x13f4), pick the highest-need type. Otherwise random pick
   from all candidates with max need score.
7. Store at +0x5650

AI_Choose_Aircraft (0x4feee0) and AI_Choose_Infantry (0x4ff210) follow the
same pattern but use aircraft/infantry type arrays respectively.

### AI Dispatch Production (0x5098f0, 106 lines)

Routes production to the correct handler based on production category:

```
for each production queue entry:
    if entry is active (+0x6f flag):
        category = entry->TypeClass->ProductionCategory (+0xb4)
        switch(category):
            case 0:  // Base building
                resolve rally point, dispatch to task force manager
            case 2:  // Naval
                AI_NavalRallyPoint (0x509e00)
            case 5, 6, 8, 0xb:  // Ground units
                AI_GroundRallyPoint (0x509cd0)
            case 7:  // Air units
                AI_FindAirTarget (0x50a150)
            case 9:  // Infantry
                AI_FindInfantryTarget (0x509f60)
            case 10: // Special (with timeout)
                check secondary rally + timeout before dispatching
```

### AI Ground Rally Point (0x509cd0)

Resolves where to send ground units:
1. If enemy exists (+0x5600 != -1): get enemy house pointer
2. If primary rally (+0x54f0) is set: use it directly
3. If rally mode == 1: use ally base center (or own if no ally),
   then pathfind nearby with FUN_0056dc20 (5×5 area search), offset +2,+2
4. Other rally modes: use FUN_0050d170 (FindTeamTarget)
5. Send to task force manager at +0x254

### AI Find Infantry Target (0x509f60, 107 lines)

Searches for buildings surrounded by enemy infantry:
1. For each building in global array (backwards):
   - Walk foundation cells (DAT_00abd490 offset table)
   - For each cell: get occupant via FUN_0047ec40
   - Count non-allied infantry (check alliance bitfield)
   - Track building with highest surrounding enemy count
2. Validate pathability via FUN_00578460
3. Return best target cell

### AI Find Air Target (0x50a150, 106 lines)

Similar to infantry targeting but for air units:
1. Check FUN_0053b400 (has_airfield). If no airfield, skip
2. For each building: walk foundation cells
3. Check for targets with tiberium flag (byte & 4)
4. Use FUN_0053c450 for air-specific targeting validation
5. Return best target cell

### AI Manage Production (0x50af10, 102 lines)

Iterates all production queues. For each active queue with a factory:
1. Find matching factory building in BuildingClass::Array
2. Check if factory has the production queue assigned (+0x5ec slots)
3. Check power ratio — if underpowered, skip production start
4. If factory exists and funded (FUN_006cc2a0): start build (FUN_006cb4d0)
5. If can't start: try alternative (FUN_006cb7b0)
6. For local player: update sidebar
7. Set ProductionChanged flag (+0x1fc = 1)

### AI Recalculate Build Options (0x5054b0, 419 lines)

The massive "what can I build?" recalculator. Called when tech changes:

1. Clear existing base plan
2. Get house side index from CountryTypeClass+0xbc
3. Iterate ALL BuildingTypes (DAT_00a83c6c):
   - Filter by house bitmask (+0x6cc & side bit)
   - Filter by Side (+0x6d0 must match or be -1)
   - Filter by TechLevel (+0x634 <= house TechLevel)
   - Filter by RequiredHouses/ForbiddenHouses (+0xda0/+0xda4)
   - Skip stolen tech types in non-multiplayer
   - Collect into buildable list
4. Insert in priority order:
   - Power buildings first (PrerequisitePower list)
   - Then barracks/factories (PrerequisiteBarracks)
   - Then remaining by prerequisite chains
   - GAPLUG is special-cased out of ordering
   - Extra copies of Radar type based on difficulty
5. Fill base plan queue at +0x5708 with 16-byte entries

### AI Scan Base Perimeter (0x5082c0, 390 lines)

Walks the 4 edges of the expanded base rectangle (+0x5764–0x5770):
1. For each direction: follow cells using direction table DAT_0089f688
2. Evaluate terrain per cell:
   - Land type (+0xec): values 2, 3, 8 = unbuildable
   - Height difference (+0x11b): must be < 3 cells
   - Passability: FUN_00578460
3. Collect "good" runs of 5+ consecutive buildable cells
4. Place defense nodes at run midpoints
5. Fill two DynamicVectors: primary defenses + secondary defenses

### AI Choose Next Production (0x506ef0, 522 lines)

The master AI production decision function. For a given queue slot:
1. Allocate 3 threat maps (infantry/vehicle/aircraft) sized to map dimensions
2. Iterate all enemy objects (+0x6c array):
   - Fill threat maps via FUN_00506d50 (6-cell radius, distance-weighted)
3. Bucket enemy objects by compass quadrant (4 directions from base center)
4. Select most-threatened direction
5. Pick buildable type using weighted ratios:
   - infantry_ratio (+0x1609c)
   - vehicle_ratio (+0x160a0)
   - aircraft_ratio (+0x160a4)
6. If primary enemy exists (+0x5600), bias ratios using enemy tracked values

### AI Update Enemy Threat Ratios (0x508150)

Reads primary enemy's tracked unit values:
```
general  = enemy+0x160b0  (vehicle/general value)
infantry = enemy+0x160ac
aircraft = enemy+0x160a8

// Add noise: ±random range + 3000 base
general  += Random(-noise, +noise) + 3000
infantry += Random(-noise, +noise) + 3000
aircraft += Random(-noise, +noise) + 3000

total = general + infantry + aircraft
if total > 0:
    infantry_ratio = infantry / total  → +0x1609c
    vehicle_ratio  = general / total   → +0x160a0
    aircraft_ratio = aircraft / total  → +0x160a4
else:
    all ratios = 0.33  (0x3ea8f5c3 as float)
```

### AI Production Categories (dispatch enum)

| Value | Category | Handler |
|-------|----------|---------|
| 0 | Base building | Rally point → task force manager |
| 2 | Naval | AI_NavalRallyPoint |
| 5, 6, 8, 0xb | Ground units | AI_GroundRallyPoint |
| 7 | Air units | AI_FindAirTarget |
| 9 | Infantry | AI_FindInfantryTarget |
| 10 | Special | Timeout-gated dispatch |

---

## Decompiled Function Details

### IsHumanPlayer (0x50b6f0) — 78 callers, most-called house query

```c
bool IsHumanPlayer(HouseClass* this) {
    if (SessionClass::GameMode != 0) {  // multiplayer
        return this == PlayerPtr;        // compare against local player
    }
    // singleplayer: check both flags
    return this->IsHuman || this->PlayerControl;
}
```

### IsPlayerControl (0x50b730) — 52 callers

Only matches IsHumanPlayer's logic in singleplayer (returns 1 if either `IsHuman`+0x1ec
or `PlayerControl`+0x1ed is set, 0 otherwise — same OR condition). In multiplayer the two
functions diverge: IsHumanPlayer compares `this == g_PlayerPtr` (is this THE local
player), while IsPlayerControl simply returns the raw `IsHuman`+0x1ec byte with no
`g_PlayerPtr` comparison at all (corrected 2026-07-18: doc previously said "same logic...
but returns raw flag byte", which undersells a real behavioral difference in the
multiplayer branch, not just a bool-vs-byte return-type difference — verified via
`decompile_function 0x50b730` — ROOT_CAUSE: INFERENCE_HARDENED / MISLEADING).

### Add_Credits (0x4f9950) — 7 callers

```c
void Add_Credits(HouseClass* this, int amount) {
    this->AvailableCredits += amount;  // +0x30c
}
```

### Spend_Money (0x4f9790) — Full overdraft handling

```c
void Spend_Money(HouseClass* this, int amount) {
    if (this->AvailableCredits < amount) {
        int shortfall = amount - this->AvailableCredits;
        this->AvailableCredits = 0;
        // If shortfall remains and tiberium stored > 0:
        // Iterate owned objects, drain stored tiberium from each
        // until shortfall is covered. Uses FUN_006c9820 (drain from storage),
        // FUN_006c96b0 (consume 1.0 unit at a time), ftol for conversion.
        // Credits recovered from tiberium are added back to spending total.
    } else {
        this->AvailableCredits -= amount;
    }
    // Notify owned objects of credit state change via FUN_004f9970
    this->TotalCreditsSpent += amount;
}
```

The tiberium drain loop processes objects one-by-one from the owned array (+0x6c),
draining stored ore/tiberium worth from each refinery/storage until the shortfall
is covered. This is why selling refineries with ore in them gives money back.

### Notify_Owned_Of_State_Change (0x4f9970)

Called after credit changes. Compares old vs new credit state. If state crossed
a threshold, iterates all owned objects and calls `Object::EnterIdleMode` (vtable+0x124
with arg 2) on objects whose TypeClass has flag at +0x16a8 set. This triggers powered
buildings to react to power/credit changes.

### Record_Last_Built (0x4fb6b0)

Records the type ID of most recently built object by RTTI:
- RTTI 6 (Unit/Vehicle): stored at +0x26c
- RTTI 0xf (Aircraft): stored at +0x270
- RTTI 1 (Building): stored at +0x274
- RTTI 2 (Infantry): stored at +0x278

Also sets ProductionChanged flag (+0x1fc = 1) and AnnounceReady (+0x246 = 1).
If the type has a custom announce delay (TypeClass+0x14d != -1), uses that;
otherwise uses defaults from RulesClass (+0x178 for buildings, +0x17c for aircraft,
+0x180 for infantry).

### AI_AssessPower (0x508c30)

Per-house power assessment, called when SpeechPending flag is set:

1. Records old power balance state (power_used vs power_total ratio against threshold)
2. Clears SpeechPending flag (+0x5778 = 0)
3. Zeros electrical power sums (+0x53A4 PowerOutputSum, +0x53A8 PowerDrainSum) (corrected 2026-07-18: the 2026-05-28 "correction" to +0x5384/+0x5388 was itself wrong; re-verified this session via `decompile_function 0x508c30` (uses typed `this->PowerOutput`/`this->PowerDrain`) and `get_struct_layout HouseClass` — ROOT_CAUSE: OFFSET_RETYPED_WRONG)
4. Iterates all owned objects:
   - Skips dead objects (byte +0x81 != 0) and inactive (byte +0x74 == 0)
   - For human players in singleplayer, skips objects without +0x41b flag
   - Calls BuildingClass__GetPowerOutput to get wattage, adds to +0x53A4 (corrected 2026-07-18: was +0x5384, itself a mis-correction of the original +0x53a4)
   - Calls BuildingClass__GetPowerDrain to get drain, adds to +0x53A8 (corrected 2026-07-18: was +0x5388, itself a mis-correction of the original +0x53a8)
   - Calls FUN_0070fec0 to check offensive capability → sets HasOffensiveUnits (+0x577b)
5. If power balance changed, calls AI_ManageProduction to adjust queues
6. Sets AnnouncementPending (+0x5779 = 1)

### CheckSuperweaponReady (0x508df0) — MISLABELED, actually a radar/tacmap availability gate

**Corrected 2026-07-18** (INDEPENDENTLY re-derived this session via `decompile_function
0x508df0`, before consulting the sibling `POWER_SYSTEM_GHIDRA_REPORT.md` correction that
flagged the same address on the same day — both reads agree): this function has **no
superweapon reference anywhere in its body**. `this->field_0x16a4` at the RTTI check is a
`TypeClass`-level flag tested only as part of a power-ratio-gated building scan, and the
function's actual output is decided by comparing its own local result against
`RadarClass__IsTacticalMapAvailable()` and, on mismatch, calling `FUN_00656df0(cVar2)` —
the same setter whose debug string is `"Radar/TacticalMap availability is %s"` (see the
"SetRadarState" note later in this doc). ROOT_CAUSE: RTTI_LABEL_DRIFT (Ghidra's stored
label is stale/wrong, not a doc error carried from a bad source).

Actual behavior (local player only, `this != g_PlayerPtr` → early return):
- Clears RecheckRadar flag (+0x5779). Waits for the announcement timer (+0x2b0/+0x2b8)
  to expire (same decay pattern as other HouseClass timers).
- If a ScenarioClass byte (`g_ScenarioClass_Instance+0x34a4`) is clear: checks the power
  ratio (`this->PowerOutput` +0x53A4 vs `this->PowerDrain` +0x53A8, same formula as
  `GetPowerRatio`); if power is sufficient, scans owned objects for one with the
  `TypeClass+0x16a4` flag set, alive, active, and whose `vtable+0x1d4` (IsActive/CanFire)
  call fails — if found, the gate result is "false" (unavailable).
- If the ScenarioClass byte is set: gate result is forced "true" (available) unconditionally.
- Compares the gate result against `RadarClass__IsTacticalMapAvailable()`; on mismatch,
  calls `FUN_00656df0` to toggle radar/tacmap state.
- See "Radar Blackout from Low Power (CheckPoweredRadar, 0x508df0)" further down in this
  doc for the fuller pseudocode walkthrough (that section already used the correct
  `RadarClass`-based framing; this section previously did not — the two were never
  reconciled until this pass).
- UNVERIFIABLE (not resolved this session): the exact INI key backing `TypeClass+0x16a4`.
  The doc elsewhere guesses both "HasRadar" (line ~2008) and "Powered=yes" (line ~2163) for
  this same offset — those two guesses conflict with each other and neither was confirmed
  against a `BuildingTypeClass::ReadINI` decompile this session. Left flagged, not resolved.

### CheckLowPower (0x508f60) — MISLABELED, actually a SpySat-building shroud handler

**Corrected 2026-07-18** (independently re-derived via `decompile_function 0x508f60`,
corroborating the sibling `POWER_SYSTEM_GHIDRA_REPORT.md`'s 2026-07-18 finding that
`TypeClass+0x16a5` is the `SpySat` INI flag, verified there via `BuildingClass::ReadINI`
push of the string `"SpySat"` at 0x0045ff72): this function has **zero references to
`PowerOutput`/`PowerDrain`**. It scans owned buildings for the first one with
`TypeClass+0x16a5` set (the `SpySat` flag) that is alive/online/not-selling; if that
building's `vtable+0x1d4` (IsActive) call returns false, it calls
`MapClass__BlackoutShroud(this)` and sets `this->SpySatActive` (+0x577a, previously
mislabeled `LowPowerState` in this doc's field table — see correction above) true; if the
scan finds no such offline building, it calls `MapClass__RestoreShroud(this)` and clears
`SpySatActive`. Both branches play a fixed sound cue (`VocClass__PlayAtPos`) only for the
local player. ROOT_CAUSE: RTTI_LABEL_DRIFT.

This is a **SpySat-specific** shroud toggle (does the house's SpySat uplink currently have
power to function), not a general "buildings losing power" notifier. See "CheckLowPower —
Powered Buildings Going Offline (0x508f60)" further down in this doc for the fuller
pseudocode (that section's mechanics are consistent with this correction; only this
earlier section had the stale framing).

### RecalcBonuses (0x50bf60) — Build speed bonus from upgrade buildings

Resets 5 float bonuses at +0x5390..+0x53a0 to 1.0f. Then iterates owned
upgrade buildings (array at +0x144, count at +0x150):

```
For each upgrade building:
    TypeClass = building->ObjectType  (+0x520)
    bonuses[0] *= TypeClass->InfantryBonus    (+0x16d0)
    bonuses[1] *= TypeClass->NavalBonus       (+0x16d4)
    bonuses[2] *= TypeClass->AircraftBonus    (+0x16d8)
    bonuses[3] *= TypeClass->VehicleBonus     (+0x16dc)
    bonuses[4] *= TypeClass->VehicleAltBonus  (+0x16e0)
```

These multiply into production speed. Multiple upgrade buildings stack multiplicatively.

### GetBuildSpeedBonus (0x50bd30) — Per-country speed modifier

Reads from HouseTypeClass (+0x34) based on RTTI type:
- RTTI 3 (Aircraft): HouseTypeClass+0x108
- RTTI 7 (Vehicle): +0x10c, or +0x110 if naval (param_2+0x382 == 5)
- RTTI 0x10 (Infantry): +0x100
- RTTI 0x28 (Naval): +0x104

These are the `SpeedInfantryMult`, `SpeedUnitsMult`, etc. from CountryTypeClass.

### GetCostBonus (0x50bdf0) — Per-country cost modifier

Same switch pattern but reading CostMult fields:
- RTTI 3: HouseTypeClass+0x11c (CostAircraftMult)
- RTTI 7: +0x120 (CostBuildingsMult), or +0x124 (CostDefensesMult) if naval
- RTTI 0x10: +0x114 (CostInfantryMult)
- RTTI 0x28: +0x118 (CostUnitsMult)

### GetAccumulatedBonus (0x50beb0) — Combined upgrade building bonus

Reads from the instance bonuses (+0x5390..+0x53a0) computed by RecalcBonuses:
- RTTI 3: +0x5398 (Aircraft)
- RTTI 7: +0x539c (Vehicle), or +0x53a0 (VehicleAlt) if naval
- RTTI 0x10: +0x5390 (Infantry)
- RTTI 0x28: +0x5394 (Naval)

### GetArmorBonus (0x50c0a0) — Per-country build time modifier

Despite the name in report 050, this actually reads BuildTimeMult fields:
- RTTI 3: HouseTypeClass+0x13c (BuildTimeAircraftMult)
- RTTI 7: +0x140 (BuildTimeBuildingsMult), or +0x144 (BuildTimeDefensesMult) if naval
- RTTI 0x10: +0x134 (BuildTimeInfantryMult)
- RTTI 0x28: +0x138 (BuildTimeUnitsMult)

### GetRepairBonus (0x50c050) — Per-country speed modifier (simplified)

Only handles RTTI 3 (Aircraft: +0x130), 0x10 (Infantry: +0x128), 0x28 (Naval: +0x12c).
Reads SpeedAircraftMult, SpeedInfantryMult, SpeedUnitsMult from HouseTypeClass.

### CheckBuildLimit (0x50b370) — Full build limit enforcement

Switch on RTTI to select the correct factory pointer:
- RTTI 1/0x28 (Building): +0x53b4 (land) or +0x53b8 (naval, if +0xCCE flag set)
- RTTI 2/3 (Infantry): +0x53ac
- RTTI 6/7 (Vehicle): +0x53bc
- RTTI 0xf/0x10 (Aircraft): +0x53b0

If factory exists, also counts queued items via FUN_004ca670.

Then switch on RTTI again for actual limit check:
- **RTTI 3 (special building)**: If type has +0xe0d flag, sums costs of ALL super
  weapons (iterates RulesClass+0xb5c array), counts owned + queued. If total <
  SuperWeaponBuildLimit (+0x2d4), returns 0 (allowed).
- **RTTI 3/7/0x28 (standard)**: Reads BuildLimit from type+0xEE. If negative, uses
  absolute value. Counts owned via FUN_0049fae0 + queued. Returns 1 (at limit) or 0.
- **RTTI 0x10 (Infantry)**: Same but also counts infantry in occupied buildings
  (iterates BuildingClass::Array checking type match via +0x338).

### Removed_From_Game (0x5025f0) — Unit lost accounting

When a unit is lost/destroyed:
1. Decrements SpySat count (+0x158) if type has SpySat flag (+0x5ec)
2. Decrements CloakDevice count (+0x15c) if type has Cloakable flag (+0x5ed)
3. Removes from selection via FUN_006a5f20
4. **Robot tank handling**: If the lost unit is a Robot Control building and
   RobotControlCount (+0x2d8) drops to 0, iterates ALL techno objects, finds
   robot tanks matching the type, calls FUN_0070fc90 (deactivate). Plays
   "EVA_RobotTanksOffline" for human players.
5. Subtracts unit cost from tracked totals by RTTI:
   - Infantry (RTTI 1): If not d96+d58 flagged → -=0x160AC (infantry value),
     else → -=0x160B0 (general value)
   - Vehicle (RTTI 2): → -=0x160B0
   - Building (RTTI 6): Removes from type tracker, handles power drain subtraction
     (+0x310), queues sell-back credits
   - Aircraft (RTTI 0xf): If not d96 flagged → -=0x160A8 (aircraft value),
     else → -=0x160B0

### Added_To_Game (0x502a80) — Unit gained accounting

Mirror of Removed_From_Game. When a unit is created/captured:
1. Increments SpySat/CloakDevice counts
2. Adds unit cost to tracked totals by RTTI (same offsets, but adding)
3. For buildings (RTTI 6):
   - If type has wall flag (+0x16bd) and not naval (+0xcce): increments +0x160
   - Calls Update_Power_And_EVA (0x5018c0) with defense power value
   - Adds building's power output (TypeClass+0x800) to +0x310
   - Adds to base node tracker via FUN_006c9740

### FindByName (0x50c170) — Lookup house by player name

Iterates all houses (DAT_00a8022c array). For each, copies +0x15ff4 (player name)
to local buffer, then does byte-by-byte strcmp against input. Returns house index
or -1 if not found.

### Find_By_Country_Index (0x502d30) — 25 callers

```c
HouseClass* Find_By_Country_Index(int country_index) {
    for (int i = 0; i < HouseCount; i++) {
        if (HouseArray[i]->HouseTypeClass->SelfIndex == country_index)
            return HouseArray[i];
    }
    return NULL;
}
```

### NotifyUnderAttack (0x4f93e0) — EVA voice + radar ping

Triggers EVA notifications when units are attacked:
1. Checks if attacker is in fog of war via FUN_0065fa70 (cell visibility)
2. Converts position: leptons to cell via `>> 8` shift
3. For ore miners under attack (type+0x148→refinery with +0x5ec SpySat and +0x408
   Robot flag): plays "EVA_OreMinerUnderAttack"
4. For local player's units: plays "EVA_OurBaseIsUnderAttack"
5. For allied units: plays "EVA_OurAllyIsUnderAttack"
6. Calls FUN_00750920 (radar ping at attacked location with intensity 1.0f)
7. Triggers screen flash via FUN_006e53a0 for each screen flash count (+0x48)

### Sell_Building_At_Cell (0x4fce80)

Validates cell has a building, checks human ownership, finds the BuildingClass,
calls its sell method (vtable+0x84). Clears cell references (+0x44 = -1, +0x50 = -1,
+0x11e = 0). Recalculates cell attributes, overlays, and occupancy.

### Power Query Functions (0x50d9c0–0x50d9f0)

```c
bool HasPowerOutput(HouseClass* this)  { return this->PowerOutputUnits > 0; }  // +0x164
bool HasPowerDrain(HouseClass* this)   { return this->PowerDrainUnits > 0; }   // +0x168
int  GetTotalPowerOutput(HouseClass* this) { return Rules->PowerPerUnit * this->PowerOutputUnits; }
int  GetTotalPowerDrain(HouseClass* this)  { return Rules->DrainPerUnit * this->PowerDrainUnits; }
```

RulesClass offsets: PowerPerUnit = +0x34, DrainPerUnit = +0x3c.

---

## RulesClass Key Offsets (House-Related)

| Offset | INI Key | Purpose |
|--------|---------|---------|
| +0x34 | (computed) | PowerPerUnit — multiplier for power output |
| +0x3c | (computed) | DrainPerUnit — multiplier for power drain |
| +0xdfc | CampaignMoneyDeltaEasy | Credits bonus for Easy difficulty |
| +0xe00 | CampaignMoneyDeltaHard | Credits bonus for Hard difficulty |
| +0x1254 | (default) | Default TechLevel |
| +0x1418 | (computed) | Difficulty scaling factor for Armor/ROF/Cost only (corrected 2026-07-18: NOT a speed factor — `decompile_function 0x4f6ec0` shows GroundSpeed/AirSpeed/BuildSpeed are copied without this multiply in singleplayer; see SetDifficulty section above — ROOT_CAUSE: INFERENCE_HARDENED) |
| +0x143c | MaxIQLevels | IQ threshold for AI activation |
| +0x1538 | (table) | Difficulty table base: 3 entries × 0x50 bytes each |
| +0x8b0 | PrerequisiteConYard | Construction yard type list |
| +0x8c8 | PrerequisitePower | Power plant prerequisite group |
| +0x8e4 | PrerequisiteRadar | Radar building prerequisite group |
| +0x900 | PrerequisiteBarracks | Infantry factory prerequisite group |
| +0x91c | PrerequisiteAny | "Any factory" prerequisite group |
| +0x934 | (list) | Additional prerequisite group |
| +0x938 | (list) | GDI/Nod/Yuri base building lists |
| +0xa34 | PrerequisiteProc | Refinery prerequisite group |
| +0xb24 | (list) | Side-indexed unit type lists |
| +0xb5c | SuperWeaponTypes | SuperWeapon type array |
| +0x115c | (table) | AI tick rate per difficulty level |

---

## RTTI Type Constants (used in switch statements throughout HouseClass)

| Value | Type | Description |
|-------|------|-------------|
| 1 | BuildingClass | Buildings/structures |
| 2 | InfantryClass | Infantry units |
| 3 | AircraftTypeClass | Aircraft type (for CanBuild/CheckBuildLimit) |
| 5 | (subtype) | Naval variant (checked via param+0x382 == 5 or +0xCCE flag) |
| 6 | UnitClass | Vehicles/units |
| 7 | InfantryTypeClass | Infantry type (for build options) |
| 0xf | AircraftClass | Aircraft units |
| 0x10 | UnitTypeClass | Vehicle type (for build options) |
| 0x13 | (special) | Excluded type — superweapons skip this |
| 0x28 | (subtype) | Naval/building variant |

Note: The RTTI values differ between the *instance* type (1, 2, 6, 0xf) and the
*type class* type (3, 7, 0x10, 0x28). Some switches handle both conventions.

---

## Alliance System — Full Decompiled Detail

### Is_Ally_ByIndex (0x4f9a10) — Check by house index

```c
bool Is_Ally_ByIndex(HouseClass* this, int house_index) {
    if (house_index == this->HouseIndex) return true;   // +0x30
    if (house_index == -1) return false;
    return (this->AllianceBitfield & (1 << house_index)) != 0;  // +0x5788
}
```

### Is_Ally_ByObject (0x4f9a90) — Check by game object

Gets the object's owner house via vtable+0x3c (GetOwner), then checks alliance.
33+ callers — used for targeting/weapon logic.

### Is_Ally_ByObject_WithFlag (0x4f9af0) — Check with validity

Same as above but requires `(object+0x14) & 2` flag set first (object must be
in valid state before testing alliance).

### MakeAlly (0x4f9b70, 129 lines) — Form alliance

Full verified flow:
1. Check if target is already an enemy via FUN_00501540 (Is_Enemy)
2. Clear UI display state via FUN_004adee0
3. **Set alliance bit**: `this->AllianceBitfield |= (1 << target->HouseIndex)`
4. Rebuild threat map via FUN_00509400
5. Clear grudge score for target in grudge list
6. If target was primary enemy: clear enemy index (+0x5600 = -1)
7. **Multiplayer radar share**: `this->RadarShareBitfield |= (1 << target->HouseIndex)` (+0x1d8)
8. If multiplayer gamemode != 4 AND not observer: recalculate alliances
9. **Clear targeting**: Iterate all game objects. For each owned unit targeting
   the new ally, clear target via vtable+0x3c8(0)
10. For human player in multiplayer: display message with ally's UIName and play
    EVA speech (StringTable line 0xd1d = "TXT_HAS_ALLIED")
11. Play EVA via FUN_00752700(-1) and update UI via FUN_004f42f0(0)

### BreakAlliance (0x4f9f90, 73 lines) — Break alliance

Full verified flow:
1. Skip if target is MultiplayPassive (+0x1a6) or self is MultiplayPassive
2. Clear UI state
3. Add to grudge list via FUN_00504790(1, target)
4. Verify was allied before clearing
5. **Clear alliance bit**: `this->AllianceBitfield &= ~(1 << target->HouseIndex)`
6. **Clear radar share**: `this->RadarShareBitfield &= ~(1 << target->HouseIndex)`
7. Rebuild threat map
8. **Reciprocal clear**: If target had us as ally, clear their bit too:
   `target->AllianceBitfield &= ~(1 << this->HouseIndex)`
   Also clear their radar share bit
9. For human player: display "TXT_AT_WAR" message (StringTable line 0xd98)
10. Play EVA_AllianceBroken and screen flash

---

## Win/Loss System — Full Decompiled Detail

### Flag_To_Win (0x4fc9e0) — Victory

Verified flow:
1. Only triggers if none of HasWon/FlagToWinPending/HasLost are set
2. Sets HasWon (+0x1f7 = 1)
3. **Borrowed time calculation** (multiplayer, gamemode != 5):
   - Get remaining time from win timer
   - Clamp minimum to `SessionClass::MaxAhead` (DAT_00a8b550)
   - Round up to next 10-frame boundary: `((frame + 9 + remaining) / 10) * 10 - frame`
   - Store at +0x298 (start frame) and +0x2a0 (borrowed time)
4. Debug log: `"Frame %d, BorrowedTime = %d"`
5. Campaign mode: calls FUN_00684240 (campaign complete), shows message
   (StringTable 0x1607 = "TXT_SCENARIO_WON"), plays EVA via FUN_006d4db0
6. Multiplayer: shows message (StringTable 0x160a = "TXT_VICTORIOUS"),
   plays EVA

### Flag_To_Lose (0x4fcbd0) — Defeat

Mirror of Flag_To_Win:
1. Clears HasWon (+0x1f7 = 0), checks FlagToWinPending
2. Sets HasLost (+0x1f8 = 1)
3. Same borrowed time calculation
4. Campaign: StringTable 0x163e = "TXT_SCENARIO_LOST", plays EVA via FUN_006d4db0(1)
5. Multiplayer: StringTable 0x1641 = "TXT_LOST", plays EVA via FUN_006d4db0(3)
6. Special: if local player == observer (DAT_00ac1198), skips EVA

### MPlayer_Defeated (0x4fc0b0, 246 lines) — Full multiplayer defeat

Verified comprehensive flow:
1. Sets IsDefeated (+0x1f5 = 1)
2. If house has max IQ: recalculate alliances for AI houses
3. Clear rally point and all owned buildings' rally references
4. If multiplayer flag 0x800: destroy all owned buildings
5. **Local player defeated**:
   - Call FUN_00431410 (disable input)
   - If screen recording: stop via FUN_006d1660
   - Disable map rendering, production, sidebar
   - Show "EVA_PlayerDefeated" message with UIName
6. **Opponent defeated**:
   - Log "MPlayer_Defeated: frame %d, house %d"
   - Show "TXT_OPPONENT_DEFEATED" message with UIName and color
7. **Check for game completion**:
   - Count alive non-defeated non-MultiplayPassive houses
   - Count human players among alive houses
   - If all remaining are allied → game over
   - "Saw game completion due to player defeat"
8. **Determine win/loss for local player**:
   - If local not defeated AND all remaining allied: Flag_To_Win
   - Else: Flag_To_Lose

### Flag_To_Win_Check (0x4fc980)

```c
char Flag_To_Win_Check(HouseClass* this) {
    if (this->HasWon == 0) {
        if (this->FlagToWinPending == 0 && this->HasLost == 0) {
            this->FlagToWinPending = 1;
            this->WinLossStartFrame = g_CurrentFrameCounter;
            this->BorrowedTimeFrames = 0;
        }
    }
    return this->FlagToWinPending;
}
```

### Destroy_All_Owned (0x4fb920, 44 lines) — On defeat

Iterates all TechnoClass objects, destroys units owned by this house
(vtable+0x20 with arg 1 = force delete). Also destroys triggers from
DAT_00a8eaec whose associated object matches. Finally destroys the house itself.

### ScatterAllUnits (0x4fc6d0) — Scatter on win pending

Iterates all TechnoClass objects. For each unit owned by this house (or matching
house via FUN_0070f820), if not busy (no active order at +0xb0, or not retreating
via FUN_00472330): call scatter (vtable+0x16c) with home position and rules
scatter range (RulesClass+0xfa8).

---

## Additional Verified Functions

### Set_Credits_And_Color (0x4fce00)

```c
void Set_Credits_And_Color(HouseClass* this, int color, int unused, int credits) {
    this->StartingCredits = credits;     // +0x1dc
    this->AvailableCredits = credits;    // +0x30c
    this->HouseTypeClass->Color = color; // HouseTypeClass+0xc0
    this->ColorSchemeIndex = color;      // +0x16054
}
```

### SetTimerValues (0x4f9610)

```c
float SetTimerValues(HouseClass* this, float param) {
    this->field_54e8 = ftol(param);  // +0x54e8
    this->AvailableCredits = ftol(param);  // +0x30c
    return param;
}
```

### GetPowerDrain (0x4f69d0)

```c
int GetPowerDrain(HouseClass* this) {
    int stored = FUN_006c9650();  // get wallet stored value
    int ftol_result = ftol(stored);
    return this->field_2ec - ftol_result;  // +0x2ec
}
```

### GetEfficiency (0x4f6e70)

```c
float GetEfficiency(HouseClass* this) {
    int stored = FUN_006c9650();
    int balance = ftol(stored);
    if (balance == 0) return 0.0f;
    float current = FUN_006c9650();
    return current / (float)this->TrackedTiberiumBalance;  // +0x310
}
```

### GetPrimaryFactoryBuilding (0x4fbd80)

Returns the TypeClass for the chosen production type by RTTI:
```c
TypeClass* GetPrimaryFactoryBuilding(HouseClass* this, int rtti_type) {
    switch(rtti_type) {
        case 1: case 0x28:  // Unit
            if (this->ChosenUnitType != -1)
                return UnitTypeArray[this->ChosenUnitType];
        case 2: case 3:    // Infantry
            if (this->ChosenInfantryType != -1)
                return InfantryTypeArray[this->ChosenInfantryType];
        case 6: case 7:    // Building
            if (this->ChosenBuildingType != -1)
                return BuildingTypeArray[this->ChosenBuildingType];
        case 0xf: case 0x10:  // Aircraft
            if (this->ChosenAircraftType != -1)
                return AircraftTypeArray[this->ChosenAircraftType];
    }
    return NULL;
}
```

### Begin_Building_Placement (0x4fb840)

Sets up placement cursor for a produced building. Only for local player:
```c
bool Begin_Building_Placement(HouseClass* this, TechnoClass* factory, BuildingClass* building) {
    if (this != PlayerPtr || DAT_00880990 != 0) return false;
    // Clear sidebar placement state
    FUN_0048dc90();  // Reset placement
    FUN_004ac8c0(0); // Clear overlay
    FUN_004ac660(0); // Clear animation
    // Store references
    DAT_00880990 = building->TypeClass;  // building type being placed
    DAT_0088098c = building;             // building being placed
    DAT_00880994 = this->HouseIndex;     // owner
    // Get placement image and show in sidebar
    image = building->vtable->GetPlacementImage(1);
    FUN_004a8bf0(image);
    coords = factory->vtable->GetCoords();
    FUN_004a91b0(coords);
    factory->vtable->EnterIdleMode(2);
    return true;
}
```

### Clear_Rally_Point (0x4fbe40)

Validates rally point object/cell, clears +0x53dc (object) and optionally
+0x53e0 (cell). For buildings: validates via FUN_00740e20. For cells: validates
via FUN_00568300 and FUN_00483460.

### Set_Rally_Point_Cell (0x4fbf60)

Validates cell via FUN_00568300, clears old rally, pathfinds nearby passable cell
via FUN_0056dc20 (1×1 search), validates with house ownership check
(CountryTypeClass+0xb8), stores at +0x53dc.

### Update_Threat_Score (0x504790) — Grudge list management

```c
void Update_Threat_Score(HouseClass* this, int score, HouseClass* target) {
    // Add score to matching entries in grudge list
    for (int i = 0; i < this->GrudgeListCount; i++) {
        if (this->GrudgeList[i].house == target)
            this->GrudgeList[i].score += score;
    }
    // Find highest-scoring non-allied enemy as primary threat
    int max_score = 0;
    HouseClass* best = NULL;
    for (int i = 0; i < this->GrudgeListCount; i++) {
        HouseClass* h = this->GrudgeList[i].house;
        if (this->GrudgeList[i].score > max_score
            && !h->IsDefeated
            && !IsAllied(this, h)) {
            max_score = this->GrudgeList[i].score;
            best = h;
        }
    }
    this->EnemyHouseIndex = best ? best->HouseIndex : -1;
}
```

### Mark_Threat_Source (0x504860)

```c
void Mark_Threat_Source(HouseClass* this, int target_house) {
    for (int i = 0; i < this->field_562c; i++) {
        if (this->field_5620[i * 2] == target_house)
            this->field_5620[i * 2 + 1] = 1;  // mark as confirmed
    }
}
```

### TransferUnitsTo (0x50d290)

```c
void TransferUnitsTo(HouseClass* this, HouseClass* source) {
    for (int i = source->OwnedCount - 1; i >= 0; i--) {
        TechnoClass* unit = source->OwnedArray[i];
        unit->vtable->SetOwner(this, 0);  // vtable+0x3d4
        unit->field_2e0 = source;  // old owner reference
        unit->field_2cc = this;    // new owner pointer
    }
}
```

### ReclaimUnitsFrom (0x50d2d0)

```c
void ReclaimUnitsFrom(HouseClass* this, HouseClass* target) {
    for (int i = this->OwnedCount - 1; i >= 0; i--) {
        TechnoClass* unit = this->OwnedArray[i];
        if (unit->field_2e0 == target) {
            unit->vtable->SetOwner(target, 0);
            unit->field_2e0 = 0;
            unit->field_2cc = 0;
        }
    }
}
```

### Adjust_Threat (0x4fa2e0) — Threat map adjustment

```c
void Adjust_Threat(HouseClass* this, int cell_index, int amount) {
    bool positive = amount >= 0;
    int abs_amount = positive ? amount : -amount;
    for (int i = 0; i < 9; i++) {
        int neighbor = OFFSET_TABLE[i] + cell_index;
        int value = abs_amount >> SHIFT_TABLE[i];
        if (positive)
            this->ThreatMap[neighbor] += value;
        else
            this->ThreatMap[neighbor] -= value;
        if (this->ThreatMap[neighbor] < 0)
            this->ThreatMap[neighbor] = 0;
    }
}
```

The 9-cell offset table (DAT_008243c8) and shift table (DAT_008243ec) are the
same tables used by AI_BuildThreatMap. This function is called to incrementally
update the threat map when individual units move.

### Reveal_Tech (0x4fae50)

Handles revealing technology to houses when a tech building is captured. Reads
upgrade info from the production queue entry (+0xee/+0xf0). Updates sidebar
buildable list via FUN_006cb920 for all houses. Calls Check_Spy_Reveal for each.

### Check_Spy_Reveal (0x4faf00)

When a spy infiltrates a building, checks distance from spy to target base center.
If within Rules->SpyInfiltrateRange (+0xee4) and random check passes
(probability from difficulty table at Rules+0xec8), stores spy's cell location
at +0x54f4 and current frame at +0x54fc. This enables the "spy reveal" secondary
rally point mechanic.

### ComputerTakeover (0x50a5c0, 327 lines) — Human to AI conversion

Verified full flow:
1. Clear human flags: +0x1ec=0, +0x1ed=0
2. Set IQ to MaxIQLevels from rules (+0x24c)
3. Copy "Computer" string to player name (+0x15ff4)
4. Load "TXT_COMPUTER" localized name (StringTable line 0x35b0)
5. Abandon all active production: iterate factories, suspend + abandon
6. Set difficulty: `FUN_004f6ec0(2 - current_difficulty)` — inverts difficulty
7. Find first alive ConYard among owned buildings
8. Set base center (+0x5490) to ConYard cell position
9. Recalculate alliances and build options
10. Set AI flags: +0x1ee=1, +0x1f2=1, +0x1f3=1
11. Rebuild base plan entries for all owned buildings with cell positions
12. Walk waypoints: sell buildings at cells with valid wall placements

---

## Verified Field Corrections

After decompilation review, the following field map corrections are confirmed:

- **+0x2ec**: Not "total credits spent secondary" — it's a separate accounting field
  used by GetPowerDrain (subtracted from wallet balance to get net drain)
- **+0x54e8**: Timer/speed value set by SetTimerValues alongside +0x30c
- **+0x54ec**: Rally point mode (0, 1, or other — controls rally resolution strategy)
- **+0x2e0/+0x2cc on TechnoClass**: Old owner and new owner pointers used during
  TransferUnitsTo/ReclaimUnitsFrom (at TechnoClass offsets, not HouseClass)
- **+0x5620/+0x562c**: Secondary threat tracking array (separate from grudge list
  at +0x5608/+0x5614). 8-byte entries: [house_ptr, confirmed_flag]
- **+0x60**: Production queue sub-array count (in owned upgrades context)
- **+0x241**: Per-house "opponent defeated notification shown" flag

---

## CanBuild Prerequisite Groups — Verified RulesClass Offsets

The negative prerequisite indices in CanBuild (0x4f7870) map to these exact
RulesClass array offsets (verified from decompiled lines 251–407):

| Index | Group | Array Ptr Offset | Count Offset | Purpose |
|-------|-------|-----------------|--------------|---------|
| -1 | PrerequisitePower | +0x35c | +0x368 | Power plants |
| -2 | PrerequisiteBarracks | +0x378 | +0x384 (900) | Infantry factories |
| -3 | PrerequisiteRadar | +0x394 | +0x3a0 | Radar buildings |
| -4 | PrerequisiteTech | +0x3b0 | +0x3bc | Tech centers |
| -5 | PrerequisiteProc | +0x3cc | +0x3d8 | Refineries |
| -6 | PrerequisiteAny | +0x3e8 (array) + 0x400 (ConYard) | +0x3f4 | Any factory + ConYard |

Each group is checked by iterating the array and calling FUN_0049fae0 (count
owned of type). If any entry has count > 0, the prerequisite is met. For group
-6, also checks the dedicated ConYard pointer at +0x400.

### CanBuild — Full Prerequisite Flow (verified)

Phase 1 (skip if param_3 != 0): Build limit check by RTTI type
Phase 2: Check type exclusion flag (type+0x326)
Phase 3: Get prerequisite list from type+0x195 via FUN_004779e0
Phase 4: For each prerequisite in list, check if house owns at least one
Phase 5: Check prerequisite override list at house+0x120 via FUN_00459840
Phase 6: If type has prerequisite group (type+0x18d != -1):
  - Parse prerequisite group from type+0x18e via FUN_004779e0
  - Check TechLevel: type+0x634 <= house TechLevel
  - **Stolen tech checks**:
    - type+0xd9d (StolenAlliedTech): needs house+0x2be flag
    - type+0x367 (StolenSovietTech): needs house+0x2bd flag
    - type+0xd9b (StolenThirdTech): needs house+0x2bc flag
  - **RequiredHouses** (type+0x368): bitmask check against CountryTypeClass+0xb8
    Also checks per-RTTI alliance masks at house+0x2c4/+0x2c8/+0x2cc/+0x2d0
  - **ForbiddenHouses** (type+0x369): bitmask exclusion check
  - **Ares-style stolen tech**: In non-multiplayer, if type has stolen tech
    prereq (type+0x5bc != -1), checks if type is in the "safe" list
    (RulesClass+0x920, count at +0x92c). If not safe, checks the production
    queue entry at type+0x5bc for the stolen flag (+0xe7).
  - **Negative prerequisite groups**: -1 to -6 as documented above
  - **Positive prerequisites**: Direct BuildingTypeClass lookup from
    DAT_00a83c6c. If type has placeable flag (+0xe88), searches owned
    buildings for matching upgrade slots (+0x17b array, 3 entries).

### CanBuild — RequiredHouses Extended Masks

The RequiredHouses check at type+0x368 has extended alliance masks that
allow side-specific ownership checking:
- house+0x2c4: mask for RTTI 0x10 (UnitType)
- house+0x2c8: mask for RTTI 0x28 (Naval/building variant)
- house+0x2cc: mask for RTTI 3 (AircraftType)
- house+0x2d0: mask for RTTI 7 (InfantryType)

---

## Additional Verified Functions

### Is_Enemy (0x501540) — Full enemy detection

```c
bool Is_Enemy(HouseClass* this, HouseClass* target) {
    if (target == 0) {
        // Special: check if ANY non-allied non-defeated house exists
        int civilianSide = FUN_006a46d0();
        if (target->CountryTypeClass->SelfIndex == civilianSide && multiplayer)
            return false;  // civilians not enemies
        if (DAT_00a8e7ac != 0) return true;
        if (this->IsDefeated) return false;
        // Count alive houses, count allied houses
        int alive = 0, allied = 0;
        for each house h:
            if (!h->IsDefeated && !h->MultiplayPassive) {
                alive++;
                if (IsAllied(this, h)) allied++;
            }
        return alive != allied + 1;  // true if non-allied houses exist
    }
    // Normal: not self, not same index, not allied
    if (target == this) return false;
    if (target->HouseIndex == this->HouseIndex) return false;
    if (IsAllied(this, target)) return false;
    // Fall through to "any enemy exists" check above
}
```

### Recalculate_Alliances (0x501640) — AI alliance management

Only runs if cooperative mode or rules flag allows. For each alive AI house:
- Sets +0x24a flag (alliance readiness)
- For each OTHER alive house:
  - If other is AI: MakeAlly (all AI houses ally with each other)
  - If other is human: BreakAlliance (AI houses are enemies of humans)

### Recalc_Base_Center (0x4fd150, 160 lines) — Weighted average

1. Clears base center (+0x5490) and all per-zone arrays
2. Iterates all owned objects with health > 0:
   - Gets cost/1000 + 1 as weight
   - Accumulates weighted X and Y coordinates
3. Divides accumulated coords by total weight → cell position
4. Pathfinds nearby valid cell via FUN_0056dc20 (1×1 area)
5. Stores at +0x5490
6. If 2+ buildings: computes base spread as max distance from center
   to any owned building, then assigns per-zone threat values

### Find_Building_Of_Type (0x4fd060)

Iterates owned objects, for each alive building: looks up its TypeClass
index in BuildingTypeClass::Array. If matches param_2, optionally filters
by zone (via FUN_004ffb20 if param_3 != -1). Returns building pointer.

### Find_Nearest_Ally_Building (0x500300, 86 lines)

Searches a grid of foundation offsets (DAT_00abd490) around given coords.
For each cell: gets building occupant, checks if allied via bitmask,
computes 3D euclidean distance, tracks closest. Returns closest allied
building pointer or null.

### DetermineEdge (0x50db00, 129 lines) — Map edge detection

Finds closest map edge for the house:
1. Locates primary building (Vehicle type RTTI 7 with deploy flag +0x3d3)
   or first ConYard (flag +0x16b9), or first alive building
2. Gets building's cell coordinates
3. Computes distance to 4 map edge midpoints
4. Returns closest edge (0=N, 1=E, 2=S, 3=W) → stored at +0x577c

### GetOppositeEdge (0x50dac0) — Simple lookup

```c
int GetOppositeEdge(HouseClass* this) {
    switch(this->EdgeDirection) {  // +0x577c
        case 0: return 2;  // N → S
        case 1: return 3;  // E → W
        case 3: return 1;  // W → E
        default: return 0; // S → N (or invalid)
    }
}
```

### Spend_Credits_Loop (0x4f9700) — Tiberium drain helper

Drains tiberium 1.0 unit at a time from the wallet object until either
param_1 iterations are done or wallet balance at RulesClass+0x17d0 is
exhausted.

### Get_Credit_Fraction (0x4f9750) — Wallet ratio

Returns ratio of current wallet balance to max capacity
(RulesClass+0x17d0). Returns 0.0 if wallet is empty.

### Place_Production (0x4fb0e0, 206 lines) — Verified flow

1. Resolve factory pointer by RTTI type (same switch as Begin_Production)
2. Verify factory IsComplete (progress == 0x36)
3. Get produced object from factory
4. **Building placement**: If placement cell is valid (not sentinel):
   - Create via vtable+0x190 (CreateBuildingAtCell)
   - Call vtable+0x278(2, producer) to associate
   - Try placement via vtable+0xd8 (TryPlaceAt)
   - On failure: play "EVA_CannotDeployHere", restore sidebar placement state
   - On success: for vehicles, deploy at exit point (vtable+0x2c check for RTTI 6)
5. **Factory exit (non-building)**: Call vtable to exit unit from factory
6. Call Record_Last_Built to track and set ProductionChanged flag
7. Handle naval waypoint rally (type+0xE74)

### Begin_Production (0x4fa350, 213 lines) — Verified flow

1. Get building power output from TypeClass+0xe08 (only for buildings RTTI 6/7)
2. Set dirty flag via FUN_005007a0
3. Look up TypeClass, verify CanBuild passes (vtable+0x94)
4. If CanBuild fails with param_4: try with relaxed mode (param 1,0,1)
5. **Factory slot selection** (same RTTI switch pattern):
   - Building: +0x53b4 (land) or +0x53b8 (naval)
   - Infantry: +0x53ac
   - Vehicle: +0x53bc or +0x53cc (defense category, power==5)
   - Aircraft: +0x53b0
6. If no factory: allocate new one (operator_new(0x74) → FUN_004c98b0 constructor)
7. If factory already building different type and RTTI==7: reject
   "Request to Begin_Production of '%s' was rejected"
8. Store factory pointer back to house slot
9. Call FactoryClass__StartProduction
10. On failure: extensive debug logging (frame, queue count, object RTTI)

---

## Electrical Power System — Complete Verified Analysis

The engine tracks TWO completely separate "power" systems in HouseClass. Confusingly,
both are called "power" in different contexts:

1. **Electrical Power** — power plants vs building drain (the sidebar power bar)
2. **Military Power** — attack strength vs defense strength (AI decision-making)

### Electrical Power: Fields and Computation

**HouseClass fields:**
- `+0x164` — Power output unit count (number of power-producing buildings)
- `+0x168` — Power drain unit count (number of power-consuming buildings)

**RulesClass constants:**
- `+0x34` — PowerPerUnit (integer multiplier, from `[General]` Power=)
- `+0x3c` — DrainPerUnit (integer multiplier, from `[General]` Drain=)

**Total output/drain calculation (verified at 0x50d9e0, 0x50d9f0):**
```c
int GetPowerOutput(HouseClass* this) {
    return RulesClass->PowerPerUnit * this->power_output_units;  // +0x34 * +0x164
}
int GetPowerDrain(HouseClass* this) {
    return RulesClass->DrainPerUnit * this->power_drain_units;   // +0x3c * +0x168
}
```

**HasPower / HasDrain (verified at 0x50d9c0, 0x50d9d0):**
```c
bool HasPower(HouseClass* this)  { return this->field_0x164 > 0; }
bool HasDrain(HouseClass* this)  { return this->field_0x168 > 0; }
```

### Per-Building Power Contribution (0x44e7b0, 0x44e880)

Each building contributes to power via `GetAttackPower` and `GetDefensePower`:

**GetAttackPower (0x44e7b0) — power output per building:**
```c
int GetAttackPower(BuildingClass* bld) {
    int power = bld->TypeClass->Power;           // TypeClass+0xee0
    if (IsInLimbo() || !IsAlive()) return 0;

    if (bld->HasUpgrade)                          // field 0x19a
        power += bld->TypeClass->ExtraPower;      // TypeClass+0xee8

    // Garrisonable buildings: each occupant adds ExtraPower
    if ((TypeClass->CanBeOccupied || TypeClass->Occupiable)
        && ExtraPower > 0 && occupant_count > 0)
        power += ExtraPower * occupant_count;     // bld+0x45 = occupant count

    // Upgrade slot contributions (3 slots at bld+0x17b)
    if (bld->HasGarrison) {                       // +0x702 flag
        for (int i = 0; i < 3; i++)
            if (upgrade_slot[i]) power += upgrade_slot[i]->Power;
    }

    // Health scaling: damaged buildings produce less
    if (power > 0 && IsAlive())
        return power * (health / max_health);     // float truncated to int
    return 0;
}
```

**GetDefensePower (0x44e880) — power drain per building:**
```c
int GetDefensePower(BuildingClass* bld) {
    if (IsInLimbo() || !IsAlive()) return 0;

    int drain = bld->TypeClass->PowerDrain;       // TypeClass+0xee4
    if (bld->field_0x669)                          // extra drain flag
        drain += bld->TypeClass->ExtraDrain;       // TypeClass+0xeec

    // Upgrade slot drain contributions (3 slots)
    if (bld->HasGarrison) {
        for (int i = 0; i < 3; i++)
            if (upgrade_slot[i]) drain += upgrade_slot[i]->PowerDrain;
    }
    return drain;
}
```

**Key insight**: Power output is health-scaled but drain is NOT. A half-health power plant
produces half power, but a half-health building still drains full power. This is why
damaged bases brown out.

### Power Assessment Per-Frame (AI_AssessPower, 0x508c30)

Called every frame when `+0x5778` (PowerDirty) flag is set:

```c
void AI_AssessPower(HouseClass* this) {
    int old_output = this->field_0x53A4;   // previous power output sum (corrected 2026-07-18: doc's 2026-05-28 fix to 0x5384 was itself wrong — 21412 decimal = 0x53A4, not 0x5384; re-verified via AI_AssessPower decompile 0x508c30 and get_struct_layout HouseClass — ROOT_CAUSE: OFFSET_RETYPED_WRONG)
    int old_drain  = this->field_0x53A8;   // previous power drain sum (corrected 2026-07-18: was 0x5388, same root cause)

    // Determine if was previously in low-power state
    bool was_low_power;
    if (old_output >= old_drain || old_drain == 0)
        was_low_power = false;
    else if (old_output == 0)
        was_low_power = true;
    else
        was_low_power = ((double)old_output / (double)old_drain) < 1.0;

    // Reset and recompute
    this->field_0x577a_has_eva_warned = false;   // +0x5778 cleared
    this->field_0x53A4 = 0;  // zero output sum (corrected 2026-07-18: was field_0x5384)
    this->field_0x53A8 = 0;  // zero drain sum (corrected 2026-07-18: was field_0x5388)

    // Sum all owned buildings
    for each building in owned_buildings:
        if (building is alive && online && !powered_off):
            // Skip human-only display buildings in skirmish
            if (IsHumanPlayer && !building->DrawOnMap && !IsMultiplayer)
                continue;
            this->field_0x53A4 += GetAttackPower(building);  // corrected 2026-07-18: was field_0x5384
            this->field_0x53A8 += GetDefensePower(building);  // corrected 2026-07-18: was field_0x5388
            if (IsPowerPlant(building) && GetAttackPower > 0)
                has_active_power_plant = true;

    this->field_0x577b = has_active_power_plant;

    // Apply spy power blackout timer
    if (blackout_timer at +0x2a4 is active && remaining > 0)
        this->field_0x53A4 = 0;  // force zero output during blackout! (corrected 2026-07-18: was field_0x5384)

    // Recompute and compare
    bool is_low_power = (new_output < new_drain && new_drain != 0
                          && (new_output == 0 || ratio < 1.0));

    if (was_low_power != is_low_power)
        AI_ManageProduction();  // trigger sidebar rebuild
    this->field_0x5779 = true;  // mark assessed
}
```

### Spy Power Sabotage (0x50bc90)

When a spy infiltrates a power plant:
```c
void SpyPowerSabotage(HouseClass* this, int delay) {
    this->field_0x5778 = true;           // mark power dirty
    this->field_0x2a4 = current_frame;   // blackout timer start
    this->field_0x2ac = delay;           // duration from RulesClass+0xd64 (SpyPowerBlackout)
}
```

During `AI_AssessPower`, if the blackout timer is active, `field_0x53A4` (output) is
forced to zero — causing immediate low power state regardless of actual plant output.
(corrected 2026-07-18: was field_0x5384, itself a wrong 2026-05-28 mis-correction of the
original field_0x53a4 — see the Power System table root-cause note above)
The EVA plays `EVA_PowerSabotaged` (string at 0x8191b0).

### Power-Off Toggle (BuildingClass, 0x4571e0)

When ownership changes or building is toggled off:
- If TypeClass has `Power > 0` (TypeClass+0xee0): calls `SpyPowerSabotage` with
  `RulesClass+0xd64` delay to trigger power reassessment
- If TypeClass has `Powered=yes` (TypeClass+0x16a4): calls `FUN_0050bd10` which
  triggers shroud reset via `FUN_00577ab0` (blackout the radar)

### EVA_LowPower Trigger in HouseClass__Update

In the local player section of Update (around address 0x4f8c00):
```c
// Only for the local player (DAT_00a83d4c == this)
int output = this->field_0x53A4;  // corrected 2026-07-18: was 0x5384 (a wrong 2026-05-28 mis-correction)
int drain  = this->field_0x53A8;  // corrected 2026-07-18: was 0x5388

if (output >= drain || drain == 0 || (output != 0 && ratio >= 1.0)) {
    DAT_00a8f040 = 0;  // clear low power warning
} else {
    // Check if player owns any ConYard-class building
    int conyard_type = Rules->BuildConst[0];  // RulesClass+0x8b0
    if (OwnedOf(conyard_type) > 0 || OwnedOf(conyard_type+1) > 0
        || OwnedOf(conyard_type+2) > 0) {
        if (DAT_00a8f040 == 0) {
            // First frame of low power: play EVA + show message
            PlayEVA("EVA_LowPower");                    // string at 0x82473c
            ShowMessage(STR_ID_0x949, house_color);     // "Insufficient power"
            DAT_00a8f040 = 1;
        }
        // Set timer for periodic re-warning
    }
}
```

### Production Speed: LowPowerPenaltyModifier and MultipleFactory

**RulesClass offsets (verified from assembly at 0x66eb9f-0x66ebce):**
- `+0x578` — **LowPowerPenaltyModifier** (float, default ~0.3) — from `[General]` LowPowerPenaltyModifier=
- `+0x57c` — **MultipleFactory** (float, default ~0.3) — from `[General]` MultipleFactory=

**GetBuildStepTime (0x6f47a0) — production speed formula:**

```c
int GetBuildStepTime(BuildingClass* factory) {
    TypeClass* type = factory->GetType();
    HouseClass* owner = factory->Owner;

    // 1. Get country cost bonus multiplier
    float cost_bonus = owner->GetCostBonus(type);   // FUN_0050c0a0

    // 2. Base step time from type cost
    int base_time = type->GetCost() * cost_bonus;

    // 3. Get power ratio (0.0 to 1.0)
    float power_ratio = owner->GetPowerRatio();      // FUN_004fce30

    // 4. Get number of same-type factories
    int factory_count = owner->GetFactoryCount(rtti); // from +0x5378/537c/5380/5384/5388

    // 5. Apply MultipleFactory bonus (RulesClass+0x57c)
    // Each additional factory divides the time further
    if (RulesClass->MultipleFactory > 0.0) {
        for (int i = 0; i < factory_count - 1; i++) {
            base_time *= power_ratio;  // power ratio penalty stacks per factory
        }
    }

    // 6. For buildings with TypeClass+0x1571 flag: apply BuildSpeed modifier
    if (rtti == 6 && type->field_0x1571)
        return base_time * BuildSpeed;  // RulesClass+0x1748

    return base_time;
}
```

**Factory count tracking (GetFactoryCount, 0x500910):**
- `+0x5378` — Infantry factory count (RTTI 2/3)
- `+0x537c` — Aircraft factory count (RTTI 0xf/0x10)
- `+0x5380` — Infantry(alt) factory count (RTTI 1/0x28, no naval flag)
- `+0x5384` — Vehicle factory count (RTTI 6/7)
- `+0x5388` — Infantry(naval) factory count (RTTI 1/0x28, naval flag)

### PowerClass::Draw — Sidebar Power Bar (0x63fb20)

**PowerClass fields (within the PowerClass object, NOT HouseClass):**
- `+0x151c` — partial pip count (drawn as frame 4 from POWER.SHP)
- `+0x152c` — green pip count (frame 1 — surplus power)
- `+0x1530` — yellow pip count (frame 2 — balanced)
- `+0x1534` — red pip count (frame 3 — deficit/drain)
- `+0x150c` — dirty flag (triggers redraw)

**Rendering:**
- Total bar height = `(DAT_00b0b504 + 3) / 3` pips (3 pixels per pip)
- Blank pips drawn first (frame 0 or 5 depending on game state)
- Then partial → green → yellow → red, bottom-to-top
- Asset: `POWER.SHP` with 6 frames (0=blank, 1=green, 2=yellow, 3=red, 4=partial, 5=alt-blank)

### Radar Blackout from Low Power (CheckPoweredRadar, 0x508df0)

Called every frame from HouseClass__Update when `+0x5779` is set. **Local player only.**

```c
void CheckPoweredRadar(HouseClass* this) {
    this->field_0x5779 = false;
    if (this != g_LocalPlayer) return;

    // Check blackout timer at +0x2b0/+0x2b8
    if (blackout_timer_active && remaining > 0) {
        goto check_radar;
    }

    // Check if NOT in Fog-of-War mode (DAT_00a8b230 + 0x34a4) — SIBLING CONTRADICTION
    // (not independently re-derived this session, flagged only): POWER_SYSTEM_GHIDRA_REPORT.md's
    // 2026-07-18 pass identifies this same ScenarioClass+0x34a4 byte as the `FreeRadar=` map
    // `[Basic]` INI key, which FORCES radar available=true when set — opposite polarity from
    // "Fog-of-War mode" framing here. This session's own decompile of 0x508df0 confirms the
    // control-flow shape (byte!=0 skips the power/building scan and forces the gate to
    // "available") is consistent with that reading, but the exact ScenarioClass field name/INI
    // key was not independently re-verified here — left for a dedicated pass, do not treat as
    // resolved.
    if (!fog_of_war_disabled) {
        // Check power ratio
        int output = this->field_0x53A4;  // corrected 2026-07-18: was 0x5384 (a wrong 2026-05-28 mis-correction)
        int drain  = this->field_0x53A8;  // corrected 2026-07-18: was 0x5388
        if (output >= drain || drain == 0 || (output != 0 && ratio >= 1.0)) {
            // Power is fine — scan for working radar building
            for each building in owned_buildings:
                if (building.IsAlive && building.TypeClass->HasRadar  // +0x16a4
                    && !building.InLimbo && building.IsOnline
                    && building.Mission != SELLING && building.SinkState != SELLING):
                    // Check if building has power to operate
                    if (building.Anim == null && !building.vtable_0x1d4())
                        goto set_radar_offline;  // powered building can't function
                    break;  // found working radar → power ok
        }
    } else {
set_radar_offline:
        should_disable_radar = true;
    }

check_radar:
    // FUN_00656de0 = GetRadarState() → reads field +0x14d8 of RadarClass
    // FUN_00656df0 = SetRadarState(bool) → writes +0x14d8, triggers radar toggle
    bool current_state = GetRadarState();
    if (current_state != should_disable_radar) {
        SetRadarState(should_disable_radar);  // toggles radar minimap on/off
    }
}
```

**SetRadarState (0x656df0)** writes to `RadarClass+0x14d8` and then:
- If radar mode == 1: calls `FUN_00656be0` (toggle radar display)
- Otherwise: calls `FUN_00656cb0` (full radar redraw)
- Debug log: `"Radar/TacticalMap availability is %s"` with "on"/"off"

**Key insight**: Radar goes offline when ANY `Powered=yes` building fails the
`vtable+0x1d4` (IsActive) check. So losing ALL radar buildings to low power
kills the minimap, but having even ONE functional one keeps it alive.

### CheckLowPower — Powered Buildings Going Offline (0x508f60)

Called every frame when `+0x5779` is set. Iterates ALL owned buildings:

```c
void CheckLowPower(HouseClass* this) {
    this->field_0x5779 = false;

    for each building in owned_buildings:
        // Only check buildings with NeedsPower flag (TypeClass+0x16a5)
        if (building.TypeClass->NeedsPower == false) continue;
        if (building.InLimbo || !building.IsOnline) continue;

        // Skip human-only display buildings for AI
        if (!IsHumanPlayer && !building.DrawOnMap) continue;

        // Skip buildings being sold/sinking
        if (building.Mission == SELLING || building.SinkState == SELLING) continue;

        // THE KEY CHECK: vtable+0x1d4 = IsActive/IsPoweredOn
        bool is_active = building.vtable_0x1d4();

        if (!is_active) {
            // Found a powered building that can't function!
            if (this->field_0x577a) return;  // already handled

            // BLACKOUT: reset shroud for this house
            FUN_00577d90(this);           // ResetShroud — fog everything
            this->field_0x577a = true;    // mark as blacked out

            if (this == g_LocalPlayer)
                PlaySFX(0x3f800000, 0);   // play power-down sound
            return;
        }
        break;  // found a working powered building → stop checking

    // If we got here without finding an offline powered building:
    if (this->field_0x577a) {
        // Was blacked out, now restored!
        FUN_00577ab0(this);               // RestoreShroud — reveal everything
        this->field_0x577a = false;

        if (this == g_LocalPlayer)
            PlaySFX(0x3f800000, 0);       // play power-up sound
    }
}
```

**Critical behavior**: The function checks buildings in ORDER. It stops at the FIRST
`NeedsPower` building it finds. If that one building is offline → blackout. If it's
online → everything's fine. This means the first NeedsPower building in the array
determines the entire house's powered state.

### Shroud Reset vs Restore (0x577ab0, 0x577d90)

**FUN_00577ab0 — BlackoutShroud (power OFF):**
- Sets `DAT_00a8022c[house_index] + 0x241 = 0` (shroud visible flag)
- Resets the map iterator state
- For each cell in the map: clears visibility bits (0xE7 mask), sets state = 1, clears flags
- Sets `g_LocalPlayer+0x240 = 0` (house can't see map)
- Calls shroud refresh and tactical map redraw

**FUN_00577d90 — RestoreShroud (power ON):**
- Sets `DAT_00a8022c[house_index] + 0x241 = 1` (shroud visible flag)
- Resets the map iterator state
- For each cell: calls `FUN_004aa050` (RevealCell) to restore vision
- Special handling for co-op mode: skips cells outside siege bounds
- Sets `g_LocalPlayer+0x240 = 1`

### Building Health State Function (0x750030, NOT power-related)

This function is called on individual buildings and returns a **damage state** based
on health ratio, NOT power ratio:

```c
int GetDamageState(float* health_ratio) {
    if (*health_ratio >= 2.0f) return 0;  // full health (green)
    if (*health_ratio >= 1.0f) return 1;  // damaged (yellow)
    return 2;                              // critical (red)
}
```

**Thresholds** (verified from memory):
- `DAT_007e37b4` = **2.0f** (0x40000000) — above this = full
- `DAT_007e2ac8` = **1.0f** (0x3F800000) — above this = damaged

Note: the input is NOT a 0-1 ratio. It's `health_ratio * ConditionYellow` where
ConditionYellow is from rules.ini. So if ConditionYellow = 0.5 and health = 75%,
the input is 1.5 → state 1 (yellow). The function returns:
- **0** — healthy: no damage fire anims
- **1** — yellow: play LowPower/damaged idle anims
- **2** — red: play SuperLowPower/heavy damage anims, POWEROFF.SHP overlay

This drives the idle animation selection from `art.ini`:
- State 0: `IdleAnim`, `SpecialAnim*`
- State 1: `LowPower`, `LowPowerPowered`, `LowPowerDamaged`
- State 2: `SuperLowPower`, `SuperLowPowerPowered`, `SuperLowPowerDamaged`

The `POWEROFF.SHP` file (string at 0x819410) is loaded and drawn as an overlay
on buildings that are in the power-off state.

### Power Toggle (TogglePower from BuildingTypeClass)

From `BuildingTypeClass_ReadINI` (string `TogglePower` at 0x81ab68):
- `TypeClass+0x1760` — TogglePower flag (parsed from art.ini)
- `TypeClass+0x1761` — related secondary toggle flag

When a building with `TogglePower=yes` is manually toggled off by the player:
- Its power contribution (GetAttackPower) drops to zero
- Its drain contribution (GetDefensePower) drops to zero
- The `+0x5778` dirty flag is set on the owning house
- Next frame's `AI_AssessPower` recomputes the totals

The `NoTogglePower` flag (string at 0x81be7c) prevents the player from toggling
specific buildings.

### Summary: How Power Flows Through the System

```
BuildingTypeClass (from rules.ini):
    Power=100        → TypeClass+0xee0 (base power output)
    PowerDrain=50    → TypeClass+0xee4 (base power drain)
    ExtraPower=25    → TypeClass+0xee8 (upgrade/garrison bonus)
    (unresolved)     → TypeClass+0x16a4 (gates the CheckSuperweaponReady/radar-gate scan;
                        doc guesses conflict — "HasRadar" elsewhere in this doc vs
                        "Powered=yes" here — neither confirmed against ReadINI this
                        session, corrected 2026-07-18 to UNVERIFIABLE rather than asserted)
    SpySat=yes       → TypeClass+0x16a5 (corrected 2026-07-18: was "NeedsPower=yes,
                        radar-critical power need" — sibling POWER_SYSTEM_GHIDRA_REPORT.md's
                        2026-07-18 pass verified this is the `SpySat` INI flag via
                        `BuildingClass::ReadINI` string push at 0x0045ff72; this session's own
                        `decompile_function 0x508f60` independently corroborates — that
                        function reads exactly this flag and drives `this->SpySatActive`,
                        with zero reference to power state — ROOT_CAUSE: RTTI_LABEL_DRIFT)
         │
         ▼
Per-Building (each frame):
    GetAttackPower() = (Power + upgrades + garrisons) × health_ratio
    GetDefensePower() = PowerDrain + upgrades (NOT health-scaled!)
         │
         ▼
HouseClass (summed across all buildings):
    +0x53A4 = Σ GetAttackPower()     (total output — corrected 2026-07-18: was +0x5384, a wrong 2026-05-28 mis-correction of the original +0x53a4)
    +0x53A8 = Σ GetDefensePower()    (total drain  — corrected 2026-07-18: was +0x5388, same root cause)
    +0x164  = count of power units   → × RulesClass+0x34 = gross output
    +0x168  = count of drain units   → × RulesClass+0x3c = gross drain
         │
         ├─→ GetPowerRatio() = output/drain (0.0 to 1.0, or 1.0 if ok)
         │       │
         │       ├─→ Production speed (GetBuildStepTime)
         │       │     × LowPowerPenaltyModifier (Rules+0x578)
         │       │     + MultipleFactory bonus (Rules+0x57c)
         │       │
         │       ├─→ AI economy state machine (attack vs defend)
         │       │
         │       └─→ EVA_LowPower + sidebar message (local player)
         │
         ├─→ CheckPoweredRadar() → radar minimap on/off
         │
         ├─→ CheckLowPower() → shroud blackout/restore
         │
         └─→ PowerClass::Draw() → sidebar power bar rendering
              (green/yellow/red pips from POWER.SHP)
```

### Electrical Power Sums (Output/Drain — actual struct fields)

**HouseClass fields (corrected 2026-07-18: the 2026-05-28 pass mislabeled these as
Military Power at +0x53a4/+0x53a8, then "corrected" to +0x5384/+0x5388 — that second
correction was ALSO wrong, a decimal-to-hex transcription slip; the true offsets, verified
independently this session via `get_struct_layout HouseClass` plus four separate function
decompiles, are +0x53A4/+0x53A8):**
- `+0x53A4` — PowerOutputSum (Σ building power output wattage, struct field `PowerOutput`)
- `+0x53A8` — PowerDrainSum  (Σ building power drain wattage, struct field `PowerDrain`)

These are rebuilt by `AI_AssessPower` (0x508c30) by calling
`BuildingClass__GetPowerOutput` and `BuildingClass__GetPowerDrain` on every owned building. The ratio
`output/drain` is returned by `GetPowerRatio` (0x4fce30):

```c
float10 GetPowerRatio(HouseClass* this) {
    int output = this->PowerOutput;  // +0x53A4 (corrected 2026-07-18: doc's 2026-05-28 fix to +0x5384 was itself wrong; independently re-verified via decompile_function 0x4fce30 and get_struct_layout HouseClass this session)
    int drain  = this->PowerDrain;   // +0x53A8 (corrected 2026-07-18: was +0x5388, same root cause as PowerOutput above)
    if (output >= drain || drain == 0) return 1.0;
    if (output == 0)                   return 0.0;
    return (float10)output / (float10)drain;
}
```

The threshold constant `DAT_007e1718` = **1.0** (double 0x3FF0000000000000).
It's not a magic number — it's literally "are you at full strength or not."

This ratio drives:
- Production speed penalty (via `GetBuildStepTime`)
- AI economy state transitions (State 2 checks ratio before attacking)
- Low power EVA warnings and sidebar message in the local player Update loop
- `AI_ManageProduction` trigger when ratio crosses the 1.0 boundary

---

## Verified Struct Size and Key Layout Regions

**Total size: 0x160B8 bytes (90,296 bytes)** — confirmed from `operator_new(0x160b8)` at 0x5009b0.

### Constructor Field Regions (from 0x4f54a0)

The constructor reveals the exact layout through initialization order:

**DynamicVectorClass arrays** (12 arrays initialized with 10-slot capacity each):
- `+0x38..+0x68` (offsets 0x0E..0x1A as dword indices) — 12 tracking arrays
  Each is a DynamicVectorClass with vtable, data pointer, count, capacity, grow flag

**Difficulty doubles** (7 × 8 bytes, initialized to 1.0 = 0x3FF0000000000000):
- `+0x18C` (index 0x63) — FirepowerMultiplier (default 1.0)
- `+0x194` (index 0x65) — GroundspeedMultiplier
- `+0x19C` (index 0x67) — AirspeedMultiplier
- `+0x1A4` (index 0x69) — ArmorMultiplier
- `+0x1AC` (index 0x6B) — ROFMultiplier
- `+0x1B4` (index 0x6D) — CostMultiplier
- `+0x1BC` (index 0x6F) — BuildTimeMultiplier

**Boolean flags** (packed bytes at +0x1EC..+0x1FC):
- `+0x1EC` — IsHumanPlayer (bool)
- `+0x1ED` — IsPlayerControl (bool, for coop)
- `+0x1EE` — IsAutoProduction
- `+0x1EF` — Unknown
- `+0x1F0` — Unknown
- `+0x1F1` — Unknown
- `+0x1F3` — HasBeenSpied
- `+0x1F4` — Unknown
- `+0x1F5` — IsDefeated
- `+0x1F6` — IsLosing
- `+0x1F7` — IsWinning
- `+0x1F8` — IsLost
- `+0x1F9..+0x1FB` — Unknown flags
- `+0x1FC` — ProductionChanged (triggers sidebar rebuild)

**Name string** at `+0x15FF4` — 20 chars (strncpy with size 0x14), null-terminated

---

## Superweapon System (per-house)

### SuperClass Instance (128 bytes, 0x80)

Each SuperClass instance is constructed at 0x6caf90:

```c
struct SuperClass {  // size = 0x80
    void* vtable;            // +0x00  PTR_FUN_007f3fe8
    // AbstractClass base
    int field_0x24;          // +0x24  unknown
    int type_pointer;        // +0x28  → SuperWeaponTypeClass*
    int owner_house;         // +0x2C  → HouseClass* (param_3 in ctor)
    int creation_frame;      // +0x30  = g_CurrentFrameCounter at construction
    int charge_timer;        // +0x34  timer for recharge
    int field_0x38;          // +0x38
    int field_0x3C;          // +0x3C
    // Timer at +0x40
    byte is_enabled;         // +0x60  (param field)
    byte field_0x62;         // +0x62  = DAT_00b0c000
    byte is_granted;         // +0x6D  charged/granted flag
    byte is_one_time;        // +0x6E  one-shot flag
    byte is_ready;           // +0x6F  ready to fire
    byte field_0x70;         // +0x70  suspended flag
    int field_0x78;          // +0x78  = -1 init
    int field_0x7C;          // +0x7C  readiness state (0/1/2)
    int field_0x7E;          // +0x7E  = -1 init
};
```

### AnimStage (0x6cbee0) — Sidebar charge progress:
```c
int SuperClass__AnimStage(SuperClass* this) {
    if (!this->is_granted)     return 0;        // not charging
    if (!this->type->ShowTimer && this->is_ready) return 0x36; // 54 = fully charged
    int stage = ftol(timer_ratio);               // 0..52
    if (stage > 0x34) return 0x35;               // cap at 53
    return stage;
}
```

### NameReadiness (0x6cc2b0) — Status text:
```c
char* SuperClass__NameReadiness(SuperClass* this) {
    if (this->field_0x70)      return "TXT_READY";      // 0x3b6
    if (!this->type->ShowTimer) {
        if (this->is_ready)    return "TXT_READY";      // 0x3b0
    } else {
        switch (this->field_0x7C) {
            case 0: return "TXT_HOLD";                   // 0x397
            case 1: return "TXT_CHARGING";               // 0x39a
            case 2: return "TXT_READY";                  // 0x39d
        }
    }
    return NULL;
}
```

### House → Superweapon Array

In the HouseClass constructor, superweapons are created in a loop:
```c
// At constructor offset ~0x600:
for (int i = 0; i < g_SuperWeaponTypeClass_Count; i++) {
    SuperClass* sw = new SuperClass(SuperWeaponTypes[i], this_house);
    house->superweapon_array.Add(sw);  // DynVectorClass at +0x254..+0x264
}
```

The superweapon array is stored as a DynamicVectorClass at HouseClass+0x254.

---

## Spy Infiltration Effects

### Stolen Tech Flags (from CanBuild prerequisite check, 0x4f7870)

Three stolen tech flags checked during prerequisite resolution:
- `RequiresStolenAlliedTech` (string at 0x843bc4)
- `RequiresStolenSovietTech` (string at 0x843be0)
- `RequiresStolenThirdTech` (string at 0x843bfc)

These are per-TypeClass flags. When a spy infiltrates an enemy's tech building (e.g. Battle Lab),
the infiltrating house gains the corresponding `StolenTech` flag at:
- `+0x2BD` — StolenAlliedTech (byte, set to 1)
- `+0x2BE` — StolenSovietTech (byte, set to 1)
- `+0x2BF` — StolenThirdTech (byte, set to 1)

These are set in the building capture handler (FUN_004571e0 at 0x4572a0) based on
`TypeClass+0x6d0` (Side index: 0=Allied, 1=Soviet, 2=Third).

### Spy Money Steal

- `SpyMoneyStealPercent` at RulesClass+0xd68 (float, e.g. 0.5 = steal 50%)
- When spy infiltrates a refinery: `stolen_amount = victim->credits * SpyMoneyStealPercent`
- Credits are transferred from victim to spy's owner

### Spy Radar Reset

When spy infiltrates a radar building:
- Sets `+0x2C0` flag on victim house
- Sets `ProductionChanged = true` (+0x1FC)
- Triggers sidebar rebuild: `DAT_00884b8e = 1`

---

## Threat Tracking System

### Threat Scores (DynamicVectorClass at +0x5608)

Each entry is 8 bytes: `{ HouseClass* enemy, int threat_score }`.

**UpdateThreat (0x504790)**:
```c
void UpdateThreat(HouseClass* this, int delta, int enemy_house) {
    // Add delta to matching enemy's threat score
    for each entry in threat_array (+0x5608):
        if (entry.house == enemy_house)
            entry.score += delta;

    // Find highest-threat non-allied enemy
    int max_score = 0;
    HouseClass* biggest_threat = NULL;
    for each entry in threat_array:
        if (entry.score > max_score
            && !entry.house->IsDefeated
            && !IsAlliedWith(entry.house)):
            max_score = entry.score;
            biggest_threat = entry.house;

    if (biggest_threat)
        this->field_0x5600 = biggest_threat->HouseIndex;
}
```

`+0x5600` — **CurrentEnemy** (index into house array) — the house this player
considers its primary threat. Used for:
- AI attack target selection
- Base defense orientation
- Production priority (counter-build)

---

## Edge/Direction System (0x4ffb20)

Determines which map edge a coordinate is relative to the base center:

```c
int GetEdgeDirection(HouseClass* this, CoordStruct* coord) {
    // Get base center (primary or secondary)
    CoordStruct base;
    if (this->field_0x5494 != INVALID)
        base = this->field_0x5494;
    else
        base = this->field_0x5490;

    // Distance check against +0x5498 (base radius)
    int dist = Distance(coord, &base);
    if (dist <= this->field_0x5498) return 0;  // inside base
    if (this->field_0x5498 * 4 < dist) return -1;  // too far

    // Calculate angle from base to coord
    float angle = atan2(base.Y - coord->Y, coord->X - base.X);
    int dir = (ftol(angle) >> 7 + 1) >> 1;  // convert to 0-255 facing

    // Map to edge index:
    if (dir > 0x20 && dir < 0xE0) return 1;  // NORTH
    if (dir > 0xA0 && dir < 0x60) return 3;  // SOUTH
    if (dir > 0x20 || dir > 0x5F) return 4;  // EAST
    return 2;                                  // WEST
}
```

**Edge indices**: 0=Inside, 1=North, 2=West, 3=South, 4=East, -1=TooFar

Used for: AI base expansion direction, attack approach vectors, and spawn placement.

---

## Credit Counter Display Refresh (0x4f9970)

When the displayed credit amount changes (after spending or earning):

```c
void RefreshCreditDisplayBuildings(HouseClass* this, int old_total, int new_total) {
    int old_display = ftol(wallet_display_value);
    int new_display = ftol(wallet_display_value);

    if (old_display != new_display) {
        // Find all buildings with DisplayCredits flag (TypeClass+0x16a8)
        for each building in owned_buildings:
            if (building.IsAlive && !building.InLimbo
                && building.TypeClass->DisplayCreditsCounter)
                building->vtable_0x124(2);  // trigger redraw
    }
}
```

This is why refineries show the credit counter — they have `TypeClass+0x16a8` set.

---

## Production Dirty Flags (0x5007a0)

When production state changes, per-RTTI dirty flags are set:

```c
void SetProductionDirty(HouseClass* this, int rtti, bool is_naval, int category, byte value) {
    switch (rtti) {
        case 2: case 3:   this->field_0x53d0 = value; break;  // Infantry
        case 6: case 7:
            if (category == 5) this->field_0x53d8 = value;     // Defense
            else               this->field_0x53d4 = value;     // Vehicle
            break;
        case 0xf: case 0x10: this->field_0x53d1 = value; break; // Aircraft
        case 1: case 0x28:
            if (is_naval)      this->field_0x53d3 = value;     // Naval building
            else               this->field_0x53d2 = value;     // Land building
            break;
    }
}
```

These flags at `+0x53d0..+0x53d8` track which production categories need sidebar refresh.

---

## IQ System (RulesClass, from [IQ] section)

Parsed at 0x674240 from `[IQ]` section in rules(md).ini. Each value is an integer
threshold — the AI can perform a behavior when its IQ level >= the threshold.

**RulesClass offsets (all int32):**
- `+0x1434` — **MaxIQLevels** (default from INI, typically 5)
- `+0x1438` — **SuperWeapons** — IQ level required to use superweapons
- `+0x143C` — **Production** — IQ level to auto-produce units
- `+0x1440` — **GuardArea** — IQ level to use guard-area command
- `+0x1444` — **RepairSell** — IQ level to repair/sell buildings
- `+0x1448` — **AutoCrush** — IQ level to auto-crush infantry with vehicles
- `+0x144C` — **Scatter** — IQ level to scatter units under fire
- `+0x1450` — **ContentScan** — IQ level to scan garrison contents
- `+0x1454` — **Aircraft** — IQ level to build/use aircraft
- `+0x1458` — **Harvester** — IQ level to manage harvesters
- `+0x145C` — **SellBack** — IQ level to sell back buildings for money

HouseClass stores the current IQ level at `+0x24C` (read from scenario INI).
The check pattern throughout AI code is: `if (house->IQLevel >= Rules->IQ_Threshold)`.

---

## SetDifficulty (0x4f6ec0) — Verified Exact Formula

**MAJOR CORRECTION 2026-07-18**: this entire section previously asserted a field-to-offset
mapping that directly CONTRADICTS both this doc's own earlier "Difficulty System" table
(the correct one) and a fresh independent decompile of 0x4f6ec0 taken this session. The
two tables disagreed with each other inside the same document and were never reconciled.
This session's `decompile_function 0x4f6ec0` is unambiguous — each HouseClass write
follows immediately after its own RulesClass table read with a literal destination offset
in the pointer arithmetic, leaving no room for interpretation. ROOT_CAUSE:
INFERENCE_HARDENED (this section's mapping was asserted, not derived from the decompile
it claims to be "verified" from).

This function loads 9 difficulty doubles from RulesClass and optionally multiplies them
by country-specific modifiers.

**Parameters**: `param_1` = HouseClass, `param_2` = difficulty index (0-4)

**Difficulty table in RulesClass**: 0x50 (80) bytes per difficulty level, starting at +0x1538.
Structure per level (corrected 2026-07-18 — field order was previously wrong; re-derived
from the exact destination-offset arithmetic in `decompile_function 0x4f6ec0`, e.g. Armor's
source read uses index arithmetic `(level*5+0x154)*0x10` which algebraically resolves to
`RulesClass + level*0x50 + 0x1540`, i.e. table-relative +0x08, not +0x18):

```
RulesClass + 0x1538 + (level * 0x50):
    +0x00 (double) = Firepower
    +0x08 (double) = Armor
    +0x10 (double) = ROF
    +0x18 (double) = GroundSpeed
    +0x20 (double) = AirSpeed
    +0x28 (double) = BuildSpeed
    +0x30 (double) = Cost
    +0x38 (double) = RepairDelay (copied directly, no scaling)
    +0x40 (double) = BuildDelay (copied directly, no scaling)
```

**Singleplayer**: Firepower/GroundSpeed/AirSpeed/BuildSpeed/RepairDelay/BuildDelay are
DIRECT COPIES (no `RulesClass+0x1418` multiply); Armor/ROF/Cost ARE multiplied by
`RulesClass+0x1418` (corrected 2026-07-18 — this session's decompile shows the exact
opposite grouping from what this section previously claimed):
```c
house->Firepower    = table->Firepower;
house->Armor        = table->Armor * Rules->field_0x1418;
house->ROF          = table->ROF * Rules->field_0x1418;
house->GroundSpeed  = table->GroundSpeed;
house->AirSpeed     = table->AirSpeed;
house->BuildSpeed   = table->BuildSpeed;
house->Cost         = table->Cost * Rules->field_0x1418;
house->RepairDelay  = table->RepairDelay;
house->BuildDelay   = table->BuildDelay;
```

**Multiplayer**: Firepower/GroundSpeed/AirSpeed/BuildSpeed gain ONLY the per-country
factor (still no `+0x1418` term); Armor/ROF/Cost gain BOTH the `+0x1418` factor AND the
per-country factor; RepairDelay/BuildDelay are unaffected (no country scaling in either
mode):
```c
house->Firepower    = table->Firepower    * country->FirepowerMult;   // +0xC8
house->Armor        = table->Armor * Rules->field_0x1418 * country->ArmorMult;         // +0xD0
house->ROF          = table->ROF * Rules->field_0x1418 * country->ROFMult;             // +0xD8
house->GroundSpeed  = table->GroundSpeed  * country->GroundSpeedMult; // +0xE0
house->AirSpeed     = table->AirSpeed     * country->AirSpeedMult;    // +0xE8
house->BuildSpeed   = table->BuildSpeed   * country->BuildSpeedMult;  // +0xF0
house->Cost         = table->Cost * Rules->field_0x1418 * country->CostMult;           // +0xF8
```
(Country-modifier offsets +0xC8..+0xF8 are carried over from the doc's earlier
CountryTypeClass table and were not independently re-verified against 0x4f6ec0's
multiplayer branch field-by-field this session beyond confirming which HouseClass fields
get a third multiply term vs. which get only two — the exact HouseTypeClass source offset
for each was not re-read this pass; flagged UNVERIFIABLE for the specific +0xC8..+0xF8
assignment, though the CountryTypeClass section elsewhere in this doc lists the same
offsets independently.)

**Output stored at HouseClass offsets (corrected 2026-07-18 to match the destination
offsets literally written in `decompile_function 0x4f6ec0`; matches the doc's own earlier
"Difficulty System" table, which was already correct):**
- `+0x184` — Difficulty index (int, 0-4)
- `+0x188` — Firepower (double)
- `+0x190` — Armor (double)
- `+0x198` — ROF (double)
- `+0x1A0` — GroundSpeed (double)
- `+0x1A8` — AirSpeed (double)
- `+0x1B0` — BuildSpeed (double)
- `+0x1B8` — Cost (double)
- `+0x1C0` — RepairDelay (double, not country-scaled)
- `+0x1C8` — BuildDelay (double, not country-scaled)

**Timer reset after difficulty change:**
```c
house->timer_0x5798 = g_CurrentFrameCounter;
house->timer_0x57A0 = Rules->SomethingPerDifficulty[difficulty] + house->HouseIndex * 175;
```
The `* 175` offset staggers AI timers across houses to prevent synchronized AI actions.

---

## House INI Read (0x500b40) — Scenario Loading

Called for each house during scenario initialization. Reads from the house's named
section in the scenario INI (e.g. `[Americans]`, `[Russians]`).

**Fields loaded (verified offsets):**

| INI Key | HouseClass Offset | Type | Notes |
|---------|------------------|------|-------|
| `TechLevel` | +0x1D4 | int | Default from ScenarioClass+0x1254 |
| `Credits` | +0x1DC | int | Multiplied by 100 (stored as hundredths) |
| `PlayerControl` | +0x1ED | byte | Co-op player control flag |
| `UIName` | +0x16009 | char[32] | Display name override |
| `RatioAITriggerTeam` | +0x565C | int | AI trigger team ratio |
| `RatioTeamAircraft` | +0x5660 | int | Default 75 (0x4B) |
| `RatioTeamInfantry` | +0x5664 | int | Default 75 |
| `RatioTeamUnits` | +0x5668 | int | Default 75 |
| `IQ` | +0x1D0, +0x24C | int | Capped at Rules->MaxIQLevels |
| `Edge` | +0x1E0 | int | Spawn edge (-1=none) |
| `Color` | +0x16054 | int | Color scheme index |
| `Allies` | → +0x5788 | bitmask | Parsed by FUN_00475260 |

**Credits with campaign difficulty bonus:**
```c
if (IsPlayerControl && GameMode == Singleplayer) {
    if (difficulty == Easy)
        credits += Rules->CampaignMoneyDeltaEasy;   // RulesClass+0xDFC
    else if (difficulty == Hard)
        credits += Rules->CampaignMoneyDeltaHard;    // RulesClass+0xE00
}
house->field_0x30C = max(credits, 0);  // clamp to non-negative
```

**Color extraction from palette:**
```c
// Get color scheme from global array
ColorScheme* scheme = g_ColorSchemes[house->ColorIndex];
// Extract RGB from palette entry
ushort pixel = palette[scheme->RemapIndex];
// Convert 16-bit to RGB bytes using bit shifts from global masks
house->ColorR = (pixel >> RedShift) << RedScale;     // +0x56F9
house->ColorG = (pixel >> GreenShift) << GreenScale;  // +0x56FA
house->ColorB = (pixel >> BlueShift) << BlueScale;    // +0x56FB

// Normalize to 0-255 brightness
float magnitude = sqrt(R² + G² + B²);
if (magnitude == 0) {
    BrightR = BrightG = BrightB = 255.0;  // white fallback
} else {
    BrightR = clamp((R * 240.0) / magnitude, 0, 255);
    BrightG = clamp((G * 240.0) / magnitude, 0, 255);
    BrightB = clamp((B * 240.0) / magnitude, 0, 255);
}
```

The `240.0` constant (DAT_007e5f78) normalizes to slightly below max brightness.

**Alliance loading from INI:**
```c
uint allies_bitmask = ReadAlliesBitmask(section, "Allies", 0);
// Allies= is a comma-separated list of house names
// Each name is looked up by FUN_0050c170 (FindHouseByName)
// Returns -1 if not found, otherwise house array index
for each token in allies_string:
    int idx = FindHouseByName(token);   // iterates DAT_00a8022c array
    bitmask |= (1 << idx);

// Then apply via MakeAlly
for each house in global_array:
    if (bitmask & (1 << house->HouseIndex)):
        MakeAlly(this, house);
```

---

## OwnedOf Counting System (IndexClass, 0x49f9b0)

The HouseClass constructor creates 12 IndexClass arrays for tracking owned unit
counts by type. Each IndexClass is 20 bytes:

```c
struct IndexClass {  // 20 bytes
    void* vtable;     // +0x00 = PTR_FUN_007e5c54
    int*  data;       // +0x04 = pointer to int array
    int   capacity;   // +0x08
    byte  can_grow;   // +0x0C
    byte  pad[3];
    int   total;      // +0x10 = total count across all types
};
```

**Key operations:**

**OwnedOf (0x49fae0)** — Get count of owned units of a specific type:
```c
int OwnedOf(IndexClass* this, int type_index) {
    if (type_index >= this->capacity) {
        if (!Grow(type_index + 10)) return 0;
        // Zero-fill new entries
    }
    return this->data[type_index];
}
```

**IncrementOwned (0x49fa00)** — Add one unit of a type:
```c
int IncrementOwned(IndexClass* this, int type_index) {
    if (type_index >= this->capacity) {
        if (!Grow(type_index + 10)) return 0;
    }
    this->data[type_index]++;
    this->total++;           // +0x10 tracks grand total
    return this->data[type_index];
}
```

**GetTotal (0x49fb60)** — Get total owned across all types:
```c
int GetTotal(IndexClass* this) {
    return this->total;  // +0x10
}
```

The 12 IndexClass arrays in HouseClass track:
- Infantry owned (by InfantryTypeClass index)
- Vehicles owned (by UnitTypeClass index)
- Aircraft owned (by AircraftTypeClass index)
- Buildings owned (by BuildingTypeClass index)
- Plus built/killed/lost variants for score tracking

---

## StorageClass — Tiberium/Ore Storage (4 floats)

Used in the credit spending system. Each refinery has a StorageClass with 4 float
slots (for the 4 ore types). Operations verified at 0x6c9650, 0x6c9740, 0x6c9820:

**GetTotal (0x6c9650)** — Sum all 4 storage slots:
```c
float GetTotal(StorageClass* this) {
    float total = 0;
    for (int i = 0; i < 4; i++)
        total += this->slots[i];
    return total;
}
```

**AddStorage (0x6c9740)** — Add amounts from another storage:
```c
void AddStorage(StorageClass* dst, StorageClass* out, StorageClass* src) {
    for (int i = 0; i < 4; i++)
        dst->slots[i] += src->slots[i];
    memcpy(out, dst, 4 * sizeof(float));
}
```

**FindNonEmpty (0x6c9820)** — Find first non-empty slot:
```c
int FindNonEmpty(StorageClass* this) {
    for (int i = 0; i < 4; i++)
        if (this->slots[i] > 0.0f) return i;
    return -1;
}
```

Used in SpendMoney (0x4f9790) for the overdraft tiberium drain:
when spending exceeds cash, the engine iterates each refinery's storage
and drains ore slot-by-slot to cover the deficit.

---

## MakeAlly (0x4f9b70) — Alliance with Targeting Cleanup

Full verified flow:

```c
void MakeAlly(HouseClass* this, HouseClass* target, bool announce) {
    if (!CanAlly(target)) return;

    // Set alliance bit
    this->AllianceBitmask |= (1 << target->HouseIndex);  // +0x5788

    // Rebuild threat map
    RecalcThreatMap();  // FUN_00509400

    // Reset threat score for new ally
    for each entry in this->ThreatArray (+0x5608):
        if (entry.house == target):
            UpdateThreat(-entry.score, target);  // zero it out
            break;

    // Clear existing targeting against new ally
    for each owned techno in global array:
        if (techno.Owner == this && techno.Target.Owner == target)
            techno->ClearTarget();
        if (techno.Owner == target && techno.Target.Owner == this)
            techno->ClearTarget();

    // EVA announcement for local player
    if (IsMultiplayer && !IsCampaign && IsHumanPlayer) {
        ShowMessage(STR_ALLIANCE_FORMED, this_name, target_name);
        if (announce && IsLocalPlayer) PlayEVA_Sound();
    }

    FUN_004adcd0();  // release lock
}
```

**BreakAlly (0x4f9f90)** — same but clears the bit, recalculates threat, and
also clears the reciprocal alliance if it existed (mutual break).

---

## Global House Array

- `DAT_00a8022c` — `HouseClass** g_HouseArray` (pointer to array of HouseClass pointers)
- `DAT_00a80238` — `int g_HouseCount` (number of active houses)
- `DAT_00a83d4c` — `HouseClass* g_LocalPlayer` (the human player's house)
- `DAT_00ac1198` — `HouseClass* g_NeutralHouse` (neutral/civilian house)
- `DAT_00a8b238` — `int g_SessionType` (0=SP, 1-4=MP modes, 5=observer)

House lookup by name (0x50c170):
```c
int FindHouseByName(char* name) {
    for (int i = 0; i < g_HouseCount; i++) {
        if (strcmp(g_HouseArray[i]->Name, name) == 0)  // +0x15FF4
            return i;
    }
    return -1;
}
```

House lookup by country index (0x502d30):
```c
HouseClass* FindHouseByCountry(int country_index) {
    for (int i = 0; i < g_HouseCount; i++) {
        if (g_HouseArray[i]->Country->SideIndex == country_index)  // Country+0xB8
            return g_HouseArray[i];
    }
    return NULL;
}
```

---

## Building Placement Validation System

Building placement involves a chain of functions that check cell passability,
foundation occupancy, overlay conflicts, adjacency to owned buildings, and
zone restrictions. All verified from Ghidra decompilation.

### Foundation Data Format

Every BuildingTypeClass has a foundation cell list obtained via `vtable+0x90`
(GetFoundationData). For MCV deploy, an alternate list is at `TypeClass+0xED4`.

**Format**: Array of `short[2]` pairs (x_offset, y_offset), terminated by sentinel `(0x7FFF, 0x7FFF)`.

Example for a 3×2 building:
```
{0,0}, {1,0}, {2,0}, {0,1}, {1,1}, {2,1}, {0x7FFF, 0x7FFF}
```

### CanBePlacedAt (0x45ee70) — Main Placement Validator

Called to determine if a building type can be placed at a given cell coordinate.
Returns: 0=OK, 1=Partial (some cells occupied by friendly), 2=Blocked.

```c
int CanBePlacedAt(BuildingTypeClass* type, CellStruct* cell) {
    if (*cell == sentinel) return 0;  // invalid cell

    short* foundation = type->GetFoundationData(1);  // vtable+0x90
    bool has_friendly_occupant = false;

    for each (dx, dy) in foundation until (0x7FFF, 0x7FFF):
        CellStruct test = { cell->X + dx, cell->Y + dy };

        // 1. Bounds check
        if (!IsValidCell(&test)) continue;  // FUN_00568300

        // 2. Get cell object
        CellClass* c = GetCellAt(&test);

        // 3. Zone/overlay check — cell+0x44 (overlay index)
        if (c->OverlayType != -1) {
            // Only WallTower (Rules->WallTower at +0x87C) allowed on zone 2
            if (type != Rules->WallTower) return 2;  // BLOCKED
            if (c->OverlayType != 2) return 2;        // wrong zone
        }

        // 4. Check occupant — cell+0xE4 (pointer to occupying object)
        TechnoClass* occupant = c->Occupant;
        if (occupant == NULL) continue;  // empty cell, OK

        // 5. Overlay object blocks placement
        int rtti = occupant->WhatAmI();  // vtable+0x2C
        if (rtti == 0x24) return 2;      // overlay object = BLOCKED

        // 6. If occupant is a building (bit 0 of flags at +0x14)
        if (!(occupant->flags & 1)) continue;

        int occ_type = occupant->WhatAmI();
        if (occ_type == 6) {
            // Wall building — check wall adjacency
            if (!CheckWallAdjacency(type, cell)) return 2;
        } else {
            // Non-wall building — must be allied
            if (!IsAlliedWith(occupant->Owner)) return 2;

            // Must have passable flag (bit 2)
            if (!(occupant->flags >> 2 & 1)) return 2;

            has_friendly_occupant = true;

            // If building has exit cell (occupant+0x169):
            // verify exit cell matches building location
            if (occupant->ExitCell != 0) {
                CellStruct bld_cell = GetCellAt(occupant->coords);
                if (bld_cell != occupant->ExitCell) {
                    // Mark cell for preview overlay
                    MarkCellForPlacement(cell, 1, 1, 0);
                }
            }
        }

    if (has_friendly_occupant) return 1;  // partially occupied by ally
    return 0;  // all clear
}
```

### CellClass__CheckCellPassability (0x4834a0 / verified decompilation)

The low-level single-cell check. Parameters:
```c
bool CheckCellPassability(
    CellClass* cell,
    int speed_type,        // 0-8 locomotion type, 4=fly (always passes)
    bool ignore_infantry,  // if true, mask out infantry bits (& 0xE0)
    bool ignore_units,     // if true, mask out unit bits (& 0x5F)
    int required_zone,     // -1=any, else must match cell zone
    int required_land,     // movement type for speed lookup
    int height_check,      // -1=any, else check cell height
    bool bridge_flag       // bridge passability override
)
```

**Check order:**
1. Speed type 4 (Fly) → always return true (aircraft ignore ground)
2. Zone check: `FUN_0056d230(cell+0x24, land, bridge)` must match required_zone
3. Height check: compare `cell+0x11B` against height_check param, with bridge variant (+4)
4. Passability bits at `cell+0x124` (normal) or `cell+0x128` (bridge):
   - Bits are masked by ignore_infantry/ignore_units params
   - If ANY unmasked bit is set → cell is BLOCKED (return false)
5. Overlay/object check: if `cell+0x44` (overlay type) is valid AND object at
   `DAT_00a83d84[overlay_index]` has flag `+0x2A8` set (impassable overlay):
   - Only allow speed types 2, 3, 8, 12, or types 1/4 if amphibious
   - Otherwise → BLOCKED
6. Speed-landtype table lookup: `g_SpeedType_LandType_Table[speed_type + land_type * 9]`
   - If speed == 0.0 and not on bridge → BLOCKED

**Key CellClass offsets used:**
- `+0x24` — Cell coordinates (CellStruct)
- `+0x44` — Overlay type index (-1 = none)
- `+0x78` — Owner bitmask (1 bit per house)
- `+0xE4` — Occupant pointer (TechnoClass*)
- `+0xEC` — Land type (int, 0-8)
- `+0x11B` — Height level (byte)
- `+0x124` — Passability bits (byte, normal ground)
- `+0x128` — Passability bits (byte, bridge surface)
- `+0x140` — Cell flags (bit 0x100 = bridge, bit 0x10 = placement pending, bit 0x400000 = impassable)

### CanDeployAtLocation (0x459ca0) — MCV Deploy Check

Checks if an MCV can deploy at its current location:

```c
bool CanDeployAtLocation(BuildingClass* mcv) {
    short* foundation = mcv->TypeClass->DeployFoundation;  // TypeClass+0xED4
    CellStruct base = mcv->GetCell();

    if (foundation == NULL) return false;

    while (*foundation != 0x7FFF || foundation[1] != 0x7FFF) {
        CellStruct test = { base.X + foundation[0], base.Y + foundation[1] };
        foundation += 2;

        if (!IsValidCell(&test)) continue;

        CellClass* cell = GetCellAt(&test);
        if (cell->Occupant != NULL) continue;  // cell must be EMPTY

        // Full passability check with all params set to "any"
        if (CheckCellPassability(cell, 0, 0, -1, 0, -1, 1))
            return true;  // at least ONE cell works
    }
    return false;
}
```

**Key difference from building placement**: MCV deploy requires cells to be
completely EMPTY (`cell->Occupant == NULL`), not just passable.

### PlaceOnFoundation (0x457aa0) — Actually Place Building

After validation passes, this function registers the building on the map:

```c
void PlaceOnFoundation(BuildingClass* bld, int param2, int param3) {
    short* foundation = bld->TypeClass->GetFoundationData(0);  // vtable+0x90
    CellStruct base = bld->GetCell();

    // Create OccupantList (0x78 bytes) via FUN_004d0ef0
    operator_new(0x78);

    for each (dx, dy) in foundation until (0x7FFF, 0x7FFF):
        CellStruct target = { base.X + dx, base.Y + dy };
        CellClass* cell = GetCellAt(&target);

        // Get or create occupant list at cell+0x28
        DynamicVectorClass* list = cell->OccupantList;
        if (list == NULL) {
            // Allocate new occupant list (0x18 bytes)
            list = new DynamicVectorClass();  // vtable = PTR_FUN_007e44f4
            list->grow_step = 1;
            cell->OccupantList = list;
        }

        // Add building reference to the cell's occupant list
        list->Add(bld);
}
```

### Wall Adjacency Check (0x452670)

For wall-type buildings (RTTI == 6), placement is only valid if:

```c
bool CheckWallAdjacency(BuildingTypeClass* type, CellStruct* cell) {
    if (type->Owner != candidate_cell_owner) return false;

    // Check if wall name matches existing structure
    if (strcmp(type->ININame, existing_building->TypeClass->ININame) != 0) {
        // Different wall types: check BuildLimit
        int limit = type->BuildLimit;       // TypeClass+0x16FC
        if (limit == -1) {
            // Unlimited: check current garrison count vs MaxOccupants
            int current = existing->GarrisonCount;  // +0x702
            int max = existing->TypeClass->MaxOccupants;  // +0x14E0
            return current < max;
        }
        if (limit >= 1 && limit <= 3) {
            return existing->GarrisonCount == 0;
        }
    }
    return false;
}
```

### Adjacency Distance Check

The `Adjacent=` value from `art.ini` (stored in BuildingTypeClass) defines how many
cells away from existing friendly buildings a new building can be placed. This is
checked during the sidebar cursor rendering — not in the placement functions themselves.

The actual adjacency is enforced by the sidebar placement preview system, which
iterates cells around existing owned buildings within the `Adjacent` radius and
marks them as valid placement zones. Cells outside this radius show as red in
the placement cursor.

### Summary: Building Placement Validation Flow

```
Player clicks cell in placement mode
         │
         ▼
Place_Production (0x4fb0e0)
    │
    ├─ Get factory → verify IsComplete (progress == 0x36)
    │
    ├─ Convert cell to world coords (cell * 256 + 128)
    │
    ├─ Call vtable+0xD8 (TryPlaceAt) on produced building
    │       │
    │       ├─ CanBePlacedAt (0x45ee70)
    │       │     │
    │       │     ├─ For each foundation cell:
    │       │     │     ├─ IsValidCell (bounds check)
    │       │     │     ├─ Zone check (cell+0x44 overlay type)
    │       │     │     ├─ Occupant check (cell+0xE4)
    │       │     │     │    ├─ NULL → OK
    │       │     │     │    ├─ RTTI 0x24 (overlay) → BLOCKED
    │       │     │     │    ├─ RTTI 6 (wall) → CheckWallAdjacency
    │       │     │     │    └─ Other → IsAlliedWith + passable flag
    │       │     │     └─ Exit cell verification
    │       │     │
    │       │     └─ Return 0=OK, 1=Partial, 2=Blocked
    │       │
    │       └─ If OK: PlaceOnFoundation (0x457aa0)
    │             └─ Register building in each cell's occupant list
    │
    ├─ On failure: EVA_CannotDeployHere + restore sidebar state
    │
    └─ On success:
          ├─ Record last built type
          ├─ Set ProductionChanged flag
          └─ Handle factory exit (for vehicles from war factory)
```

---

## SuperClass — Per-House Superweapon Instances (0x80 bytes)

**CORRECTION**: The field at +0x254..+0x268 previously labeled as "ProductionQueueArray"
is actually the **SuperWeapons DynamicVector**. The constructor at 0x4f54a0 creates one
SuperClass per SuperWeaponType and stores them in this DVC. HandlePowerTransition
(0x50af10) iterates this array and calls SuperClass::Suspend/Deactivate on elements.

### SuperWeapons DVC on HouseClass

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x254 | 4 | DVC vtable | DynamicVector<SuperClass*> |
| +0x258 | 4 | SuperWeaponsArray | Pointer to SuperClass*[] |
| +0x25C | 4 | SuperWeaponsCapacity | |
| +0x260 | 4 | SuperWeaponsOwns | owns_memory flag + padding |
| +0x264 | 4 | SuperWeaponsCount | Number of superweapons |
| +0x268 | 4 | SuperWeaponsGrow | Grow increment (10) |

Created in the constructor by iterating `DAT_00a8e340` (global SuperWeaponTypeClass count),
allocating 0x80 bytes per SuperClass, and calling `SuperClass::Constructor(type_ptr, house_ptr)`.

### SuperClass Field Layout (0x80 bytes)

| Offset | Size | Type | Purpose |
|--------|------|------|---------|
| +0x00 | 16 | ptr[4] | 4 vtable pointers (INoticeSink + multi-inherit) |
| +0x24 | 4 | int | TimerOverride — custom recharge time (-1 = use TypeClass default) |
| +0x28 | 4 | ptr | SuperWeaponTypeClass* — the type definition |
| +0x2C | 4 | int | Unknown (init 0) |
| +0x30 | 4 | int | ChargeStartFrame — when charging began (-1 = inactive) |
| +0x34 | 4 | int | Timer data (frame snapshot) |
| +0x38 | 4 | int | RemainingFrames — frames left until ready |
| +0x3C | 4 | ptr | Related object ptr |
| +0x40 | 1 | byte | Flag (init 0) |
| +0x48 | 4 | int | Unknown (init 0) |
| +0x4C | 4 | int | Unknown (init 0) |
| +0x50 | 4 | int | ReadyCountdown — sound countdown (-1 = done) |
| +0x54 | 4 | int | Unknown |
| +0x60 | 1 | byte | IsActivated — production started |
| +0x62 | 4 | -- | CRC/hash from DAT_00b0c000 |
| +0x68 | 4 | ptr | AssociatedBuilding — building that grants this SW |
| +0x6C | 1 | byte | Unknown flag |
| +0x6D | 1 | byte | IsEnabled — master on/off toggle |
| +0x6E | 1 | byte | Unknown flag |
| +0x6F | 1 | byte | IsReady — fully charged and available to fire |
| +0x70 | 1 | byte | IsSuspended — charging paused (e.g., low power) |
| +0x74 | 4 | int | LastProgressFrame — frame of last charge step |
| +0x78 | 4 | int | LastAnimStage — cached sidebar pip stage (-1 = unset) |
| +0x7C | 4 | int | OneTimeState — for OneTime SWs: 0=Charging, 1=Ready, 2=Active |

### SuperClass Key Functions

| Address | Name | Purpose |
|---------|------|---------|
| 0x6caec0 | Constructor (no params) | Basic field init |
| 0x6caf90 | Constructor (no params, alt) | Same basic init with global array registration |
| 0x6cb120 | Constructor (params) | Full init with SuperWeaponTypeClass ptr + HouseClass ptr |
| 0x6cb4d0 | Suspend | Pause/resume charging timer; saves remaining frames on suspend |
| 0x6cb7b0 | Deactivate | Fully disable SW, remove from global active list |
| 0x6cbca0 | AI_Ready | Check if timer expired; handle OneTime 3-state (Charging→Ready→Active) |
| 0x6cbee0 | AnimStage | Returns 0–54 progress (54 = ready); used for sidebar pip display |
| 0x6cc080 | AI_Charging | Start/continue charging; plays EVA at charge start |
| 0x6cc2b0 | NameReadiness | Returns localized state string for UI display |
| 0x6cc390 | Launch | Massive (955 lines) — dispatches superweapon effect by type enum |

### SuperWeaponTypeClass Key Offsets (from SuperClass usage)

| Offset | Type | Purpose |
|--------|------|---------|
| +0xB0 | int | DefaultRechargeTime — frames to fully charge |
| +0xB4 | int | Type enum — 0=Nuke, 1=IronCurtain, 2=ChronoSphere, etc. (switch in Launch) |
| +0xE5 | byte | IsOneTime — one-shot superweapon (3-state lifecycle) |
| +0xF5 | byte | Unknown flag (affects suspend behavior) |

### AnimStage Values (sidebar pip progress)

```c
int AnimStage(SuperClass* this) {
    if (!this->IsEnabled) return 0;
    if (!this->TypeClass->IsOneTime && this->IsReady) return 0x36;  // 54 = fully ready
    int progress = ftol(elapsed / duration * 54);
    return min(progress, 0x35);  // cap at 53 while charging
}
```

The sidebar uses these values to draw a charging progress indicator. 0x36 (54) is the
"complete" stage that triggers the "superweapon ready" cameo highlight.

### NameReadiness State Strings

```c
if (IsSuspended)     return "TXT_OFFLINE";       // StringTable 0x3B6
if (IsReady)         return "TXT_READY";          // StringTable 0x3B0
if (IsOneTime) {
    if (state == 0)  return "TXT_CHARGING";       // StringTable 0x397
    if (state == 1)  return "TXT_READY";          // StringTable 0x39A
    if (state == 2)  return "TXT_ACTIVE";         // StringTable 0x39D
}
```

### HandlePowerTransition (0x50af10) — Power↔Superweapon Link

This function connects the electrical power system to superweapon availability.
Iterates the SuperWeapons DVC at +0x258:

```c
void HandlePowerTransition(HouseClass* this) {
    for (int i = 0; i < this->SuperWeaponsCount; i++) {
        SuperClass* sw = this->SuperWeaponsArray[i];
        if (!sw->IsEnabled || !sw->IsActivated) continue;
        if (this->IsDefeated) {
            // Deactivate all on defeat
            SuperClass::Deactivate(sw);
            continue;
        }

        // Find matching factory building that owns this superweapon
        bool has_factory = false;
        bool factory_powered = false;
        for each building in BuildingClass::Array:
            if (building->Owner == this) {
                // Check 3 production slots at building+0x5EC
                for (int slot = 0; slot < 3; slot++) {
                    int sw_ptr = building->ProductionSlots[slot];
                    if (sw_ptr->SlotA == i || sw_ptr->SlotB == i) {
                        has_factory = true;
                        factory_powered = building->IsPowered;  // +0x660
                    }
                }
                // Also check building's primary/secondary SW slots
                if (building->PrimarySW == i || building->SecondarySW == i) {
                    has_factory = true;
                    factory_powered = building->IsPowered;
                }
            }

        // Check power ratio
        int output = this->PowerOutputUnits;
        int drain = this->PowerDrainUnits;
        bool has_power = (output >= drain) || (drain == 0)
                         || (output > 0 && (double)output/(double)drain >= 1.0);
        if (!has_power) factory_powered = false;

        // Apply state change
        if (has_factory && !this->IsDefeated) {
            if (!factory_powered) {
                SuperClass::Suspend(sw, 1);  // suspend (pause timer)
            } else {
                SuperClass::Suspend(sw, 0);  // resume
            }
        } else {
            SuperClass::Deactivate(sw);      // fully disable
        }

        // Update sidebar if local player
        if (this == PlayerPtr) {
            SidebarClass::Refresh(tab);
            this->ProductionChanged = 1;
        }
    }
}
```

**Key behavior**: When power drops below demand, all superweapons are suspended
(their charge timers pause but don't reset). When power is restored, they resume
charging from where they left off. If the building that grants the superweapon is
destroyed, the superweapon is fully deactivated.

---

## Field Map Corrections & Additions

### HasPowerSurplus (0x50E1B0) — Actually "HasRobotControl"

The function named `HasPowerSurplus` returns `this->field_0x2D8 > 0`. This is the
**RobotControlCount** field, confirmed by:
- `RobotTanksOffline` (0x50E0E0) decrements +0x2D8 and triggers at 0 transition
- `RobotTanksBackOnline` (0x50E010) increments +0x2D8 and triggers at 1 transition
- Callers: `TeleportLocomotionClass::PostWarpValidation` and `UnitClass::Mission_Enter`
  — both check robot control availability, not power surplus

The function should be understood as "HasRobotControl" — returns true when at least
one Robot Control Center building is owned.

### Constructor Init Defaults (from full 0x4f54a0 decompilation)

Additional field initializations confirmed from the constructor:

| Offset | Init Value | Meaning |
|--------|-----------|---------|
| +0x1D8 | 0, then `OR (1 << houseIndex)` | RadarShareBitfield — self-bit always set (you share radar with yourself) |
| +0x56F9 | (0, 0, 0) | HouseColorRGB — black until InitColor sets it |
| +0x56FC | (0xFF, 0xFF, 0xFF) | HouseBrightRGB — white until ComputeRemap sets it |
| +0x1E8 | copies CountryTypeClass+0xBC | SideIndex — only 0, 1, 2 recognized; else stays -1 |
| +0x5390..+0x53A0 | 0x3F800000 (1.0f) each | 5 build speed bonuses (infantry/naval/air/vehicle/vehicleAlt) |
| +0x1609C..+0x160A4 | 0x3EA8F5C3 (0.33f) each | AI infantry/vehicle/aircraft ratios |
| +0x5788 | 0 | AllianceBitfield — no allies initially |
| +0x57E4 | zeroed (0x4204 dwords) | ThreatMapGrid — full zero at start |
| +0x54FC | 0xFFFFFF9C (-100) | SecondaryRallyFrame — expired sentinel |

### DynamicVector Arrays in HouseClass (Complete Map from Constructor)

The constructor creates 14 DynamicVector instances. Each DVC is 0x18 bytes
(vtable + data_ptr + capacity + owns_memory + count + grow_amount):

| DVC Start | Data Ptr | Count | Purpose |
|-----------|----------|-------|---------|
| +0x38 | +0x3C | +0x48 | Unknown DVC (special FUN_00510640 type) |
| +0x50 | +0x54 | +0x60 | Unknown DVC |
| +0x68 | +0x6C | +0x78 | **OwnedObjects** (TechnoClass*[]) |
| +0x80 | +0x84 | +0x90 | Unknown DVC |
| +0x98 | +0x9C | +0xA8 | Unknown DVC |
| +0xB0 | +0xB4 | +0xC0 | Unknown DVC |
| +0xC8 | +0xCC | +0xD8 | Unknown DVC |
| +0xE0 | +0xE4 | +0xF0 | Unknown DVC |
| +0xF8 | +0xFC | +0x108 | Unknown DVC |
| +0x110 | +0x114 | +0x120 | Unknown DVC |
| +0x128 | +0x12C | +0x138 | Unknown DVC |
| +0x140 | +0x144 | +0x150 | **OwnedUpgrades** (BuildingClass*[]) |
| +0x16C | +0x170 | +0x17C | **GarrisonStructures** (AI garrison tracking) |
| +0x254 | +0x258 | +0x264 | **SuperWeapons** (SuperClass*[]) |

There are also 2 DVCs for grudge/threat tracking:
| +0x5604 | +0x5608 | +0x5614 | **GrudgeList** (8-byte entries: [house_ptr, score]) |
| +0x561C | +0x5620 | +0x562C | **ThreatSourceList** (8-byte entries: [house_ptr, flag]) |

And 2 per-house relationship DVCs initialized in the constructor loop:
| +0x5604 | +0x5608 | +0x5614 | GrudgeList (per other house) |
| +0x561C | +0x5620 | +0x562C | ThreatSourceList (per other house) |

### DepositOreCredits (0x4F9610)

```c
float DepositOreCredits(HouseClass* this, float amount) {
    this->field_54E8 = ftol(amount);   // +0x54E8 = last deposit amount
    this->AvailableCredits = ftol(amount);  // +0x30C = set (not add!) credits
    return amount;
}
```

Note: This function SETS credits, it does not ADD. Called from specific ore processing
contexts where the running total is passed in.

### DepositWeedCredits (0x4F9700)

```c
void DepositWeedCredits(HouseClass* this, int iterations, int ore_type) {
    while (iterations > 0 && StorageClass::GetTotal() < RulesClass->MaxStorage) {
        StorageClass::AddAmount(1.0f, ore_type);
        iterations--;
    }
}
```

Deposits ore/weed into storage one unit at a time. Stops when storage capacity
(RulesClass+0x17D0 = MaxStorage) is reached. This is the refinery unload loop.

### UpdateSiloDisplays (0x4F9970)

Compares old vs new ore storage levels. If changed, iterates all owned buildings
with the silo flag (TypeClass+0x16A8) and calls `EnterIdleMode(2)` via vtable+0x124
to update their visual state (silo fullness animation frames).

---

## Ghidra Reports Used

- `047_menu_button_house_class.md` — Constructor, destructor, difficulty, CanBuild stub
- `048_house_class_alliance_production_ai.md` — Alliance, production, defeat/victory, AI strategy
- `049_house_class_tracking_ai.md` — Unit tracking, INI load/save, base defense placement
- `050_house_class_ai_core.md` — AI brain, base planning, build system, player identity
- `051_house_class_continued.md` — Garrison, rally points, robot tanks, power queries
- `052_dynamic_vector_destructors_country_type_ini.md` — CountryTypeClass INI loading
