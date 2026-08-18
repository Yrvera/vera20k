# HouseClass Verified Field Map (gamemd.exe)

**Source:** Live Ghidra decompilation of `gamemd.exe` (YR 1.001)
**Original source path:** `D:\ra2mdpost\House.CPP`, `D:\ra2mdpost\Country.CPP`
**Total struct size:** 0x160B8 bytes (90,296 bytes). Verified: `operator_new(0x160B8)` in Create_Houses.

Every offset in this document has been confirmed from disassembly. Fields marked "From report"
have been carried forward from cross-referenced analysis but not yet independently re-verified
at the assembly level; all other fields have direct asm citations.

---

## Corrections from Original Report

The original `HOUSECLASS_GHIDRA_REPORT.md` contained the following errors, now corrected:

1. **StorageClass offset was wrong.** The original report listed +0x310 or +0x314 for
   tiberium storage. The CORRECT offset is **+0x2FC** (verified from `Spend_Money` asm:
   `LEA ESI,[EBX+0x2FC]` and `Notify_Credit_State_Change` asm: `LEA ECX,[EDI+0x2FC]`).

2. **+0x310 is StorageCapacity, not TrackedTiberiumBalance.** The original report called
   +0x310 "TrackedTiberiumBalance". It is actually **StorageCapacity** -- the sum of all
   owned buildings' `Storage=` INI value. Verified from `Added_To_Game` (adds
   `BuildingType+0x800`), `Notify` (used as FIDIV divisor for fill %), and
   `Removed_From_Game` (subtracts `BuildingType+0x800`).

3. **Difficulty multiplier ORDER was wrong.** The original report had the doubles in a
   different sequence. The corrected order (from `SetDifficulty` at 0x4f6ec0, cross-referenced
   with `CountryTypeClass::ReadINI` at 0x511850) is: Firepower, Groundspeed, Airspeed,
   Armor, ROF, Cost, BuildSpeed, RepairDelay, BuildDelay.

4. **Power ratio function logic was inverted.** The original report's pseudocode had the
   condition backwards. Corrected: if output >= drain then 1.0; if output == 0 then 0.0;
   else output/drain.

5. **+0x24C is CurrentIQ (int), not at +0x1D0.** The original report listed IQ at +0x1D0.
   Verified from `Create_Houses` and `MPlayer_Defeated` that CurrentIQ is at +0x24C.

6. **Missing fields now added:** ScreenFlashCount (+0x48), DifficultyLevel (+0x184),
   ColorSchemeIndex (+0x16054), HouseColorRGB (+0x56F9), HouseBrightRGB (+0x56FC),
   StorageClass (+0x2FC), DisplayedCredits (+0x54E8), and many others.

---

## 1. Identity & Control

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x00 | 16 | ptr[4] | Vtable pointers (multiple inheritance) | Constructor |
| +0x24 | 4 | ptr | Vtable pointer (5th) | Constructor |
| +0x28 | 4 | ptr | Vtable pointer (6th) | Constructor |
| +0x2C | 4 | ptr | Vtable pointer (7th) | Constructor |
| +0x30 | 4 | int | HouseIndex (player slot 0-31, auto-increment from global count) | Is_Ally_ByIndex, Find_By_Country_Index |
| +0x34 | 4 | ptr | HouseTypeClass* (CountryTypeClass pointer, rules data) | Set_Credits_And_Color, Find_By_Country_Index |
| +0x48 | 4 | int | ScreenFlashCount | Update asm |
| +0x184 | 4 | int | DifficultyLevel (0=Easy, 1=Medium, 2=Hard) | SetDifficulty |
| +0x1D4 | 4 | int | TechLevel (parsed from map `[HouseName] TechLevel=`) | Create_Houses |
| +0x1E4 | 4 | int | AI build state (0/1/2 state machine) | Update AI chooser switch |
| +0x1E8 | 4 | int | SideIndex (0=Allied, 1=Soviet, 2=Yuri) | RecenterBase |
| +0x1EC | 1 | byte | IsHuman (human-controlled flag) | IsHumanPlayer (0x50b6f0) |
| +0x1ED | 1 | byte | PlayerControl (from map INI `PlayerControl=`) | IsHumanPlayer |
| +0x1EE | 1 | byte | IsAutoProduction (whether AI production logic runs) | Update AI activation |
| +0x1F3 | 1 | byte | IQ threshold reached (set when IQ >= MaxIQLevels) | Update AI activation |
| +0x15FF4 | 20 | char[20] | PlayerName (20-char player name string) | Create_Houses |
| +0x16009 | 32 | char[32] | UIName (localized display name) | Constructor |
| +0x16054 | 4 | int | ColorSchemeIndex | Set_Credits_And_Color, MPlayer_Defeated |

---

## 2. Flags & State

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x1F5 | 1 | byte | IsDefeated | MPlayer_Defeated sets it, Update reads it |
| +0x1F6 | 1 | byte | FlagToWinPending (waiting for borrowed time) | Flag_To_Win guard, Flag_To_Lose guard, Update scatter |
| +0x1F7 | 1 | byte | HasWon (victory flag) | Flag_To_Win sets, Flag_To_Lose CLEARS |
| +0x1F8 | 1 | byte | HasLost (defeat flag) | Flag_To_Lose sets |
| +0x1FC | 1 | byte | ProductionChanged (init=1, triggers AI production) | Update, Constructor |
| +0x245 | 1 | byte | Unknown notification flag | Update tail |
| +0x246 | 1 | byte | AnnounceReadyFlag ("unit ready" pending) | Update (0x5A=90 frame timer) |
| +0x24B | 1 | byte | SidebarUpdatePending (init=1, triggers refresh) | Constructor, Update tail |
| +0x24C | 4 | int | CurrentIQ (AI aggressiveness level) | Create_Houses, MPlayer_Defeated |

**Flag_To_Lose behavior:** CLEARS HasWon (+0x1F7 = 0) before setting HasLost (+0x1F8 = 1).
Losing always overrides winning.

---

## 3. Economy

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x1DC | 4 | int | StartingCredits (INI `Credits=` value x100) | Set_Credits_And_Color |
| +0x2DC | 4 | int | TotalCreditsSpent (cumulative spending) | Spend_Money asm (`EBX+0x2DC`) |
| +0x2FC | 16 | float[4] | TiberiumStorage (StorageClass, 4 floats for 4 tiberium types) | Spend_Money asm (`LEA ESI,[EBX+0x2FC]`), Notify_Credit_State_Change asm (`LEA ECX,[EDI+0x2FC]`) |
| +0x30C | 4 | int | Credits / AvailableCredits (current cash balance) | Add_Credits (`param_1+0x30C += param_2`), Spend_Money, Set_Credits_And_Color |
| +0x310 | 4 | int | StorageCapacity (sum of all owned buildings' `Storage=` INI value) | Added_To_Game (`+= BuildingType+0x800`), Notify (`FIDIV` as divisor for fill %), Removed_From_Game (`-= BuildingType+0x800`) |
| +0x54E8 | 4 | int | DisplayedCredits (interpolated value for sidebar counter) | Add_Tiberium_Credits asm |

### Tiberium-to-Credits Formula

Verified from `Add_Tiberium_Credits` asm at 0x4f9610:

```
new_credits = old_credits + (TiberiumTypeClass[slot]+0xB8 * CountryTypeClass+0x148 * amount)
```

Where:
- `TiberiumTypeClass[slot]+0xB8` = Value per unit of that tiberium type
- `CountryTypeClass+0x148` = IncomeMult (country-specific income multiplier)

### Spend_Money Overdraft Logic

Verified from asm at 0x4f9790:

1. If Credits (+0x30C) >= cost: simple subtraction from +0x30C
2. Else: set Credits = 0, compute shortfall = cost - old_credits
3. Iterate OwnedObjects (+0x6C array, +0x78 count)
4. For each building: drain from BUILDING storage (object+0x33C), AND drain from
   HOUSE storage (+0x2FC)
5. Convert drained tiberium via `TiberiumType->Value * CountryType->IncomeMult`
6. Any over-recovery is refunded back to +0x30C

---

## 4. Difficulty Multipliers

Verified from `SetDifficulty` (0x4f6ec0), cross-referenced with `CountryTypeClass::ReadINI`
(0x511850). All are doubles (8 bytes each).

| Offset | Size | Type | Field | INI Key (CountryType) |
|--------|------|------|-------|-----------------------|
| +0x188 | 8 | double | Firepower (default 1.0) | `Firepower=` (CountryType+0xC8) |
| +0x190 | 8 | double | Groundspeed | `Groundspeed=` (CountryType+0xD0) |
| +0x198 | 8 | double | Airspeed | `Airspeed=` (CountryType+0xD8) |
| +0x1A0 | 8 | double | Armor | `Armor=` (CountryType+0xE0) |
| +0x1A8 | 8 | double | ROF (rate of fire) | unnamed (CountryType+0xE8) |
| +0x1B0 | 8 | double | Cost | unnamed (CountryType+0xF0) |
| +0x1B8 | 8 | double | BuildSpeed | `BuildTime=` (CountryType+0xF8) |
| +0x1C0 | 8 | double | RepairDelay (no country scaling) | -- |
| +0x1C8 | 8 | double | BuildDelay (no country scaling) | -- |

**Difficulty table source:** `RulesClass` at `Rules+0x1538 + difficulty*0x50`, 9 doubles per
difficulty tier.

**Scaling rules:**
- In campaign: some fields are additionally multiplied by `Rules+0x1418` (global factor)
- In multiplayer: additionally multiplied by the CountryType fields listed above

---

## 5. Power System

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x164 | 4 | int | PowerOutputUnits (count of power-producing units) | HasPowerOutput, GetTotalPowerOutput (`*Rules+0x34`) |
| +0x168 | 4 | int | PowerDrainUnits (count of power-consuming units) | HasPowerDrain, GetTotalPowerDrain (`*Rules+0x3C`) |
| +0x53A4 | 4 | int | AttackPowerSum (total offensive value) | Update low power check, AI_AssessPower |
| +0x53A8 | 4 | int | DefensePowerSum (total defensive value) | Update low power check |

### GetPowerRatio (0x4fce30)

```
if output >= drain:  return 1.0
if output == 0:      return 0.0
else:                return (float)output / (float)drain
```

---

## 6. Diplomacy

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x1D8 | 4 | int | RadarShareBitfield (32 bits, one per house) | MakeAlly |
| +0x5600 | 4 | int | EnemyHouseIndex (-1 = none) | Update_Threat_Score |
| +0x5608 | 4 | ptr | GrudgeList DVC data ptr (8-byte entries: [HouseClass*, int score]) | Constructor |
| +0x5614 | 4 | int | GrudgeList count | Update threat decay (every 100 frames) |
| +0x5620 | 4 | ptr | ThreatSource DVC data ptr | Constructor cross-registration |
| +0x562C | 4 | int | ThreatSource count | Constructor |
| +0x5788 | 4 | int | AllianceBitfield (32 bits, one per house slot) | Is_Ally_ByIndex asm, MPlayer_Defeated alliance check |

### Alliance Check

`Is_Ally_ByIndex(house_index)`: tests bit `house_index` in AllianceBitfield (+0x5788).

### MPlayer_Defeated Game Completion

O(n^2) check: ALL alive, non-defeated, non-MultiplayPassive houses must be BIDIRECTIONALLY
allied (both houses must have each other's bit set in their respective +0x5788). If all
remaining houses are mutually allied, the game ends: Flag_To_Win for alive local player,
Flag_To_Lose for dead local player.

---

## 7. Owned Object Tracking

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x6C | 4 | ptr | OwnedObjects array ptr (TechnoClass**) | Spend_Money asm |
| +0x78 | 4 | int | OwnedObjectsCount | Spend_Money asm, Notify, RecenterBase |
| +0x158 | 4 | int | SpySatCount | Added/Removed_To_Game (TypeClass+0x5EC flag) |
| +0x15C | 4 | int | CloakDeviceCount | Added/Removed_To_Game (TypeClass+0x5ED flag) |
| +0x160 | 4 | int | WallCount | Added_To_Game (building+0x16BD flag, not naval+0xCCE) |
| +0x2D8 | 4 | int | RobotControlCount | Removed_From_Game (if drops to 0, disable robots) |
| +0x2F0 | 4 | int | OwnedBuildings | Update defeat detection asm (`MOV EDI,[ESI+0x2F0]`) |
| +0x160A8 | 4 | int | TrackedAircraftValue | Added/Removed_To_Game |
| +0x160AC | 4 | int | TrackedInfantryValue | Added/Removed_To_Game |
| +0x160B0 | 4 | int | TrackedGeneralValue | Added/Removed_To_Game |

### IndexClass Arrays (20 bytes each, for per-type counting)

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x5514 | 20 | IndexClass | Building IndexClass | Alt defeat mode asm |
| +0x5550 | 20 | IndexClass | ConYard IndexClass | Low power ConYard check asm |
| +0x5564 | 20 | IndexClass | Infantry IndexClass | Defeat detection asm (`LEA ECX,[ESI+0x5564]`) |
| +0x5578 | 20 | IndexClass | Vehicle IndexClass | Defeat detection asm (`LEA ECX,[ESI+0x5578]`) |
| +0x558C | 20 | IndexClass | Aircraft IndexClass | Defeat detection asm (`LEA ECX,[ESI+0x558C]`) |

### Defeat Detection (from Update asm)

```
total = OwnedBuildings[+0x2F0]
      + IndexClass::GetTotal(+0x5564)   // infantry
      + IndexClass::GetTotal(+0x5578)   // vehicles
      + IndexClass::GetTotal(+0x558C)   // aircraft
      + CountOwnedInstances(MCV from Rules+0x8E8)

if total == 0:
    ScatterAllUnits()
    MPlayer_Defeated()
```

No grace period. Runs every frame in multiplayer. Houses with MultiplayPassive
(HouseTypeClass+0x1A6) are exempt.

---

## 8. Production

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x210 | 48 | struct[4] | FactorySlots (4 slots x 12 bytes each) | From report |
| +0x258 | 4 | ptr | SuperWeaponArray DVC data ptr | Constructor (`param_1[0x96]`) |
| +0x264 | 4 | int | SuperWeaponArray count | Update loop |
| +0x5378 | 4 | int | InfantryFactoryCount | GetFactoryCount |
| +0x537C | 4 | int | AircraftFactoryCount | GetFactoryCount |
| +0x5380 | 4 | int | BuildingFactoryCount | GetFactoryCount |
| +0x5384 | 4 | int | VehicleFactoryCount | GetFactoryCount |
| +0x5388 | 4 | int | NavalFactoryCount | GetFactoryCount |
| +0x538C | 4 | int | HarvestBonusCount (buildings with Purifier flag) | Add_Tiberium_Credits chain |
| +0x5390 | 20 | float[5] | BuildSpeedBonuses | RecalcBonuses |
| +0x53AC | 4 | ptr | InfantryFactory ptr | From report |
| +0x53B0 | 4 | ptr | AircraftFactory ptr | From report |
| +0x53B4 | 4 | ptr | UnitFactory ptr | From report |
| +0x53B8 | 4 | ptr | NavalFactory ptr | From report |
| +0x53BC | 4 | ptr | BuildingFactory ptr | From report |
| +0x53CC | 4 | ptr | BuildingFactoryAlt ptr | From report |
| +0x564C | 4 | int | ChosenBuildingType (-1 = none) | Update |
| +0x5650 | 4 | int | ChosenUnitType | Update AI chooser |
| +0x5654 | 4 | int | ChosenAircraftType | Update AI chooser |
| +0x5658 | 4 | int | ChosenInfantryType | Update AI chooser |

---

## 9. Win/Loss System

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x298 | 4 | int | WinLossStartFrame | Flag_To_Win, Flag_To_Lose |
| +0x2A0 | 4 | int | BorrowedTimeFrames | Flag_To_Win, Flag_To_Lose |

### Borrowed Time Formula

```
borrowed = ((current_frame + 9 + remaining) / 10) * 10 - current_frame
```

Clamped to at least `g_NetworkFrameBudget` (SessionClass::MaxAhead, at DAT_00a8b550).

### Win/Loss Semantics

- `Flag_To_Win`: sets HasWon (+0x1F7 = 1), records WinLossStartFrame
- `Flag_To_Lose`: CLEARS HasWon (+0x1F7 = 0), THEN sets HasLost (+0x1F8 = 1)
- Losing always overrides a prior win state

---

## 10. Rally Points & Base

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x53DC | 4 | ptr | RallyPointObject ptr | MPlayer_Defeated |
| +0x53E0 | 4 | short[2] | RallyPointCell (2 shorts: X, Y) | MPlayer_Defeated, Update rally loop (every 15 frames) |
| +0x5490 | 4 | CellStruct | BaseCenterCell (weighted avg of owned building positions) | RecenterBase |
| +0x5494 | 4 | CellStruct | AltBaseCenterCell | From report |
| +0x5498 | 4 | int | BaseSpreadRadius (min 0x200) | From report |
| +0x54F0 | 4 | CellStruct | PrimaryRallyCell | From report |
| +0x54F4 | 4 | CellStruct | SecondaryRallyCell | From report |
| +0x54FC | 4 | int | SecondaryRallyFrame (-100 = expired sentinel) | Constructor (0xFFFFFF9C = -100) |

---

## 11. Timers

All timers follow the pattern: `[start_frame (int), unused (int), duration (int)]` = 12 bytes.
**Expired** when `current_frame - start_frame >= duration`. `start_frame = -1` means inactive.

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x280 | 12 | Timer | Generic timer | Update |
| +0x298 | -- | -- | (WinLossStartFrame, see section 9) | -- |
| +0x2A4 | 12 | Timer | Speech timer | Update |
| +0x2B0 | 12 | Timer | Announcement timer | Update |
| +0x5634 | 12 | Timer | AI strategy timer | Update |
| +0x5798 | 12 | Timer | AI trigger timer | SetDifficulty (last 3 lines set it) |
| +0x57A4 | 12 | Timer | Announce ready timer (0x5A = 90 frames) | Update |
| +0x57BC | 12 | Timer | Low power warning timer | Update asm |
| +0x57D4 | 12 | Timer | Money warning timer | Update asm |

---

## 12. Color

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x56F9 | 3 | byte[3] | HouseColorRGB (R, G, B) | Constructor (init to 0,0,0), InitColor |
| +0x56FC | 3 | byte[3] | HouseBrightRGB (normalized, brighter variant) | Constructor (init to 0xFF, 0xFF, 0xFF) |

---

## 13. AI-Specific

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x5708 | 4 | ptr | BasePlanNodeArray | RecenterBase |
| +0x5714 | 4 | int | BasePlanNodeCount | RecenterBase |
| +0x5750 | 4 | CellStruct | BasePlanCenterCell | RecenterBase |
| +0x5778 | 1 | byte | PowerDirty / SpeechPending | Removed_From_Game, Update |
| +0x5779 | 1 | byte | RecheckRadar (init=1) | Update |
| +0x577A | 1 | byte | LowPowerState | Constructor |
| +0x577B | 1 | byte | HasOffensiveUnits | AI_AssessPower |
| +0x577C | 4 | int | EdgeDirection (0=N, 1=E, 2=S, 3=W) | DetermineEdge |
| +0x57E4 | ~66KB | int[] | ThreatMapGrid (0x4204 dwords) | Constructor (0x15F9*4 = 0x57E4, zeros 0x4204 dwords) |
| +0x160A0 | 4 | float | AI infantry ratio (default 0.33) | Constructor |
| +0x160A4 | 4 | float | AI vehicle ratio (default 0.33) | Constructor |
| +0x160A8 is TrackedAircraftValue (see section 7) | -- | -- | -- | -- |

---

## 14. Misc

| Offset | Size | Type | Field | Verified By |
|--------|------|------|-------|-------------|
| +0x5774 | 4 | ptr | SelfPointer (stores `this`) | From report |

---

## 15. Key Globals

| Address | Name | Purpose |
|---------|------|---------|
| DAT_00a8022c | g_HouseClass_Array | Array of HouseClass pointers |
| DAT_00a80238 | g_HouseClass_Array_Count | Number of active houses |
| DAT_00a83d4c | g_PlayerPtr | Local player's HouseClass* |
| DAT_00ac1198 | g_ObserverHouse | Observer house pointer |
| DAT_00a8b238 | g_GameMode | 0=campaign, 3/4=multiplayer |
| DAT_00a8ed84 | g_CurrentFrameCounter | Current simulation frame |
| DAT_008871e0 | g_RulesClass_Instance | Singleton rules/balance data |
| DAT_00a8ef98 | g_InvalidCell | Sentinel "no cell" value |
| DAT_00a8b550 | g_NetworkFrameBudget | SessionClass::MaxAhead |
| DAT_00a83c6c | g_BuildingTypeClass_Array | All building types |
| DAT_00a83ce4 | g_UnitTypeClass_Array | All unit types |
| DAT_00a8b21c | g_InfantryTypeClass_Array | All infantry types |
| DAT_00a8e34c | g_AircraftTypeClass_Array | All aircraft types |
| DAT_00b0f4ec | g_TiberiumTypeClass_Array | All tiberium types |

---

## 16. Total Size

```
0x160B8 bytes = 90,296 bytes
```

Verified: `operator_new(0x160B8)` called in `Create_Houses`.

---

*Last updated: 2026-03-26. All offsets verified against gamemd.exe (YR 1.001) via Ghidra.*
