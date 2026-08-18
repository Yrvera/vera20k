# BuildingClass — Master Ghidra Research Report

**Date:** 2026-04-16 (updated with extended investigation)
**Binary:** gamemd.exe
**Confidence:** HIGH (all primary findings verified from binary decompilation)
**Active in YR:** Yes — BuildingClass is the core building runtime class

**Companion reports (read these for full detail on each subsystem):**
- `BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md` — 300-slot vtable map
- `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` — 27-step per-tick pipeline
- `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` — Combat, garrison fire, charge mode
- `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` — Mission_Selling + MissionRepairAndProduce
- `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md` — Cloning, Grinding, Hospital, Armory, BioReactor, SecretLab, OrePurifier, FactoryPlant
- `BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md` — CloakGenerator, SensorArray, building cloaking
- `BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md` — Nuke silo state machine, Receive_Radio protocol
- `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md` — 7 spy effects in priority order
- `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` — 11-check immunity chain
- `BUILDING_UPGRADE_SYSTEM_GHIDRA_REPORT.md` — 3-slot upgrade lifecycle
- `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` — Per-building dock/exit paths
- `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` — 19-step ownership transfer
- `GARRISON_SYSTEM_GHIDRA_REPORT.md` — Occupant fire mechanics

## 1. Overview

BuildingClass is the runtime instance class for all buildings in gamemd.exe.
It inherits directly from TechnoClass (NOT through FootClass — buildings skip
FootClass in the hierarchy). Size is **0x720 bytes** (1824 decimal).

BuildingTypeClass is the type/template class for building definitions, loaded
from rules.ini/rulesmd.ini. Size is **0x1798 bytes** (6040 decimal).

Both sizes confirmed via `operator_new` calls in their respective constructors.

### Inheritance Chain

```
IUnknown (COM)
  └─ AbstractClass          vtable @ 0x007E1F50
       └─ ObjectClass       vtable @ 0x007EF060
            └─ MissionClass  vtable @ 0x007EDCC0
                 └─ RadioClass  vtable @ 0x007F0508
                      └─ TechnoClass  vtable @ 0x007F4960
                           └─ BuildingClass  vtable @ 0x007E3EBC
```

**No FootClass.** Infantry, vehicles, and aircraft inherit TechnoClass → FootClass → specific class. Buildings inherit TechnoClass → BuildingClass directly.

---

## 2. BuildingClass Instance Layout (+0x000 to +0x720)

Fields marked with ✓ are verified from decompilation. Fields marked with ? need further confirmation.

### Inherited Fields (TechnoClass, up to ~+0x500)

| Offset | Type | Field | Evidence |
|--------|------|-------|----------|
| +0x21C | ptr | Owner (HouseClass*) | OnSpyInfiltrate reads `this->Owner` |
| +0x504 | int | EMPLockRemaining | ✓ GoOnline checks `== 0` |

### BuildingClass-Specific Fields (+0x520 onward)

| Offset | Type | Field | Evidence |
|--------|------|-------|----------|
| +0x520 | ptr | Type (BuildingTypeClass*) | ✓ GetSuperWeaponIndex1, CanAcceptUpgrade |
| +0x524 | ptr | Factory (FactoryClass*) | ✓ Destructor references |
| +0x528-0x530 | | Temporal/chrono timer fields | IronCurtain writes here |
| +0x534 | int | DamagedState flag | ✓ GetCurrentFrame checks |
| +0x55C | ptr[21] | Anims[21] — AnimClass* array | ✓ 21 anim slot pointers, 0x54 bytes |
| +0x5B0 | — | **REMOVED**: prior docs claimed 21-byte ChargeFlags array; verification found NO function reads/writes this range. The "21" refers to the Anims array at +0x55C (21 DWORDs). | ✓ DEBUNKED 2026-04-16 |
| +0x5C8 | ptr[8] | Secondary anim/fire pointers | ✓ Destructor iterates 8 entries |
| +0x5E8 | | (internal upgrade base) | RemoveLastUpgrade indexes from here |
| +0x5EC | ptr | Upgrades[0] (BuildingTypeClass*) | ✓ GetPowerOutput iterates |
| +0x5F0 | ptr | Upgrades[1] | ✓ Ghidra struct |
| +0x5F4 | ptr | Upgrades[2] | ✓ Ghidra struct |
| +0x5FC | int | Cycling anim phase index | ✓ NukeReactor special types |
| +0x600 | ptr | **BuildingLightClass*** (spotlight — only if HasSpotlight, conditional) | ✓ Verified |
| +0x614 | ptr | **LightSourceClass*** (ambient light, on ALL buildings, destroyed in dtor) | ✓ Verified |
| +0x620 | int | Repair/production progress | ✓ MissionRepairAndProduce reads/writes |
| +0x624 | bool | Production step flag | Set to 1 when timer fires |
| +0x628 | int | CDTimer start frame | Set to g_CurrentFrameCounter |
| +0x62C | int | CDTimer auxiliary | |
| +0x630 | int | CDTimer rate | |
| +0x634 | int | CDTimer active flag (0/1) | |
| +0x638 | int | CDTimer step amount | Added to +0x620 each tick |
| +0x660 | byte | **HasPower** | ✓ GoOnline sets true, PowerCheck reads |
| +0x661 | byte | **IsOverpowered** (set by PowerCheck_Upgrade at 0x00450614) | ✓ Verified — NOT HasExtraPowerBonus |
| +0x662 | byte | (cleared at constructor, no verified consumer) | ? Unverified |
| +0x668 | byte | **HasExtraPowerBonus** (read in GetPowerOutput at 0x0044E7D5) | ✓ Verified |
| +0x669 | byte | **HasExtraPowerDrain** (read in GetPowerDrain at 0x0044E89F) | ✓ Verified |
| +0x114 | int | **TechnoClass CargoClass::NumPassengers** (inherited — bio-reactor uses this embedded CargoClass same as transport helos) | ✓ Verified — writes via CargoClass::AddPassenger |
| +0x66C | ptr | DynamicVector vtable (NOT absorb — see below) | ✓ Constructor initializes |
| +0x670 | ptr | DV Items pointer | ✓ PowerCheck_Upgrade reads |
| +0x674 | int | DV Capacity | |
| +0x679 | byte | DV OwnerFlag | |
| +0x67C | int | DV Count | ✓ Used by PowerCheck_Upgrade loop |
| +0x684 | ptr | Occupant DynamicVector vtable | ✓ Constructor sets |
| +0x688 | ptr | Occupant Items (InfantryClass* array) | ✓ AddGarrisonOccupant stores |
| +0x68C | int | Occupant Capacity | |
| +0x691 | byte | Occupant OwnerFlag | |
| +0x694 | int | Occupant Count | ✓ GetOccupantCount returns directly |
| +0x69C | int | **GarrisonFireIndex** (garrison round-robin) — verified | ✓ 2026-04-16 verification |
| +0x6C9 | byte | bool (construction-related) | ReadFromINI parse sets |
| +0x6CA | byte | bool | ReadFromINI sets |
| +0x6CB | byte | bool | ReadFromINI sets |
| +0x6D0-0x6D8 | | CDTimer (ProduceCash timer) | OnConstructionComplete sets |
| +0x6DC | byte | SellBuilding/NominalPower flag | |
| +0x6DD | byte | ConstructionComplete flag | ✓ OnConstructionComplete sets to 1 |
| +0x6DF | byte | ForceShield active flag | ✓ IronCurtain checks |
| +0x6EB | byte | CloakGenerator radius cache | Set to 0xFF on offline |
| +0x6EC | byte | CloakGenerator active flag | |
| +0x6F0 | int | Refinery ore level state | OnConstructionComplete calculates |
| +0x700 | short | (unknown) | |
| +0x702 | byte | **UpgradeLevel** (0-3) | ✓ CanAcceptUpgrade compares, AddUpgrade increments |

### Corrections to Prior Documentation

1. **+0x660** should be named **HasPower**, not "IsOnline" — all Ghidra references use `HasPower`
2. **+0x670** is NOT "InfantryAbsorb count" — it's the Items pointer of a DynamicVector
3. **+0x67C** is NOT "UpgradeCount" — it's the Count field of the DynamicVector at +0x66C
4. **+0x16BF** on BuildingTypeClass is **LaserFence**, not generic "Wall" (LaserFencePost is at +0x16BE)
5. **+0x5B0 ChargeFlags DEBUNKED** — no function touches this range as a 21-byte array. The "21" count comes from the Anims array at +0x55C (21 DWORDs). Remove this claim from all docs.
6. **+0x600 is BuildingLightClass*** (spotlight, conditional). +0x614 is the separate **LightSourceClass*** (ambient light, on all buildings). Prior docs conflated these.
7. **+0x69C IS correct** for GarrisonFireIndex. An intermediate agent incorrectly placed it at +0x664 — this was wrong. Verified via GetWeapon and TechnoClass::Fire_At.
8. **+0x114 is the bio-reactor occupant count scalar** (not a DynamicVector). GetPowerOutput reads this directly when UnitAbsorb/InfantryAbsorb is set.

---

## 3. BuildingTypeClass Layout (+0x000 to +0x1798)

### Core Building Properties

| Offset | Type | INI Key | Default | Purpose |
|--------|------|---------|---------|---------|
| +0x0CCE | bool | `Naval=` | false | Naval building (TechnoTypeClass level) |
| +0xE88 | char[24] | `PowersUpBuilding=` | "" | Name of building this upgrades |
| +0xEB8 | int | `Factory=` | 0 | Factory enum (0x10=Infantry, 0x28=Unit, etc.) |
| +0xEC8 | int[3] | `ExitCoord=` | 0,0,0 | Lepton offset for produced unit exit |
| +0xED4 | ptr | (computed) | NULL | Pointer into global foundation exit cell table |
| +0xEE0 | int | `Power=` | 0 | Power output (positive = generates) |
| +0xEE4 | int | `Power=` (neg) | 0 | Power drain (stored as positive) |
| +0xEE8 | int | `ExtraPower=` | 0 | Extra power bonus (bio-reactor occupants) |
| +0xEEC | int | `ExtraPower=` (neg) | 0 | Extra power drain |
| +0xEF0 | int | `Foundation=` | — | Foundation enum index |
| +0xF4C | [3×0x44] | (art.ini) | — | PowerUp anim entries (68 bytes each) |

### Upgrade/Production Fields

| Offset | Type | INI Key | Default | Purpose |
|--------|------|---------|---------|---------|
| +0x14E0 | int | `Upgrades=` | 0 | Max upgrade slots (0-3, vanilla uses max 2) |
| +0x1580 | int | `MaxNumberOccupants=` | 0 | Garrison capacity |
| +0x157B | bool | `CanBeOccupied=` | false | Infantry can garrison |
| +0x157C | bool | `CanOccupyFire=` | false | Garrisoned infantry can fire |
| +0x1584 | bool | `ShowOccupantPips=` | false | Show occupant pips |
| +0x1588 | | OccupantWeaponFireCoords array | — | Fire port positions for garrison |
| +0x1618 | short[2] | `QueueingCell=` | 0,0 | Harvester queue cell offset (art.ini) |
| +0x16FC | int | `PowersUpToLevel=` | -1 | Target upgrade level (-1 = incremental) |

### Boolean Flags Block (+0x16A4 to +0x16CD)

All verified from ReadINI at 0x0045FE50:

| Offset | INI Key | Purpose |
|--------|---------|---------|
| +0x16A4 | `Radar=` | Provides radar display |
| +0x16A5 | `SpySat=` | Full map vision when powered |
| +0x16A6 | `ChargeAnim=` | Has charge animation |
| +0x16A7 | (internal) | Initialized to 0 |
| +0x16A8 | `SiloDamage=` | Ore destroyed when damaged |
| +0x16A9 | `UnitRepair=` | Repair depot |
| +0x16AA | `UnitReload=` | Ammo reload pad |
| +0x16AB | `Bunker=` | Battle Bunker |
| +0x16AC | `Cloning=` | Cloning Vats |
| +0x16AD | `Grinding=` | Grinder (destroys units for money) |
| +0x16AE | `UnitAbsorb=` | Absorbs units |
| +0x16AF | `InfantryAbsorb=` | Absorbs infantry (Bio Reactor) |
| +0x16B0 | `SecretLab=` | Grants random tech |
| +0x16B1 | `DoubleThick=` | Double-thick wall |
| +0x16B3 | `DockUnload=` | Dock + unload (refinery flag) |
| +0x16B4 | `Recoilless=` | No barrel recoil |
| +0x16B6 | `BridgeRepairHut=` | Bridge repair |
| +0x16B7 | `Gate=` | Has opening gate |
| +0x16B9 | `ConstructionYard=` | Is a ConYard |
| +0x16BA | `NukeSilo=` | Nuke silo flag |
| +0x16BB | `Refinery=` | Refinery flag |
| +0x16BC | `Weeder=` | Weeder building |
| +0x16BD | `WeaponsFactory=` | Vehicle production |
| +0x16BE | `LaserFencePost=` | Laser fence post |
| +0x16BF | `LaserFence=` | Laser fence segment |
| +0x16C0 | `FirestormWall=` | TS firestorm wall (dormant in YR) |
| +0x16C1 | `Hospital=` | Heals infantry |
| +0x16C2 | `Armory=` | Promotes infantry |
| +0x16C3 | `EMPulseCannon=` | EMP cannon (TS legacy) |
| +0x16C4 | `TickTank=` | TS tick tank (dormant in YR) |
| +0x16C7 | `CloakGenerator=` | Provides cloaking field |
| +0x16C8 | `SensorArray=` | Provides sensor detection |
| +0x16C9 | `ICBMLauncher=` | ICBM launcher |
| +0x16CA | `Artillary=` | Artillery building |
| +0x16CB | `Helipad=` | Helicopter landing pad |
| +0x16CC | `OrePurifier=` | Ore purifier bonus |
| +0x16CD | `FactoryPlant=` | Factory plant cost reduction |

### Cost Bonus Floats (+0x16D0 to +0x16E0)

| Offset | Type | INI Key | Purpose |
|--------|------|---------|---------|
| +0x16D0 | float | `InfantryCostBonus=` | Infantry cost multiplier |
| +0x16D4 | float | `UnitsCostBonus=` | Vehicle cost multiplier |
| +0x16D8 | float | `AircraftCostBonus=` | Aircraft cost multiplier |
| +0x16DC | float | `BuildingsCostBonus=` | Building cost multiplier |
| +0x16E0 | float | `DefensesCostBonus=` | Defense cost multiplier |

### Barracks Type Flags and Misc

| Offset | Type | INI Key | Purpose |
|--------|------|---------|---------|
| +0x16E4 | bool | `GDIBarracks=` | Allied barracks exit cell pattern |
| +0x16E5 | bool | `NODBarracks=` | Soviet barracks exit cell pattern |
| +0x16E6 | bool | `YuriBarracks=` | Yuri barracks exit cell pattern |
| +0x16E8 | float | `ChargedAnimTime=` | Charge animation duration |
| +0x16EC | int | `DelayedFireDelay=` | Delayed fire delay ticks |
| +0x16F0 | int | `SuperWeapon=` | SuperWeapon index (-1 = none) |
| +0x16F4 | int | `SuperWeapon2=` | Second superweapon index (-1 = none) |
| +0x16F8 | int | `GateStages=` | Number of gate animation stages |
| +0x1700 | bool | `DamagedDoor=` | Door damaged flag |
| +0x1701 | bool | `InvisibleInGame=` | Invisible in game |
| +0x1702 | bool | `TerrainPalette=` | Uses terrain palette |
| +0x1703 | bool | `PlaceAnywhere=` | No placement restrictions |
| +0x1704 | bool | `ExtraDamageStage=` | Extra damage stage |
| +0x1706 | bool | `IsBaseDefense=` | Base defense building |
| +0x1707 | byte | `CloakRadiusInCells=` (also used by RemoveSensorArrayAt) — default 0x14 (20 cells), signed byte | ✓ Verified |
| +0x1710 | int | `BarrelStartPitch=` | Barrel starting pitch angle |
| +0x1763 | bool | `IsThreatRatingNode=` | Threat rating node |
| +0x1764 | bool | `PrimaryFireDualOffset=` | Dual fire offset |
| +0x1765 | bool | `ProtectWithWall=` | AI walls around this |
| +0x1766 | bool | `CanHideThings=` | Can hide units underneath |
| +0x1767 | bool | `CrateBeneath=` | Spawns crate when destroyed |
| +0x1768 | bool | `LeaveRubble=` | Leaves rubble on destruction |
| +0x1769 | bool | `CrateBeneathIsMoney=` | Crate is money type |
| +0x1780 | int | `NumberOfDocks=` | Number of dock pads |
| +0x1788 | ptr | DockingOffset data | Array of 12-byte lepton offset entries |

---

## 4. Vtable (0x007E3EBC) — 300 Slots

BuildingClass overrides ~95 of 300 vtable slots. Key overrides:

| Slot | Offset | Address | Method |
|------|--------|---------|--------|
| 3 | 0x00C | 0x00459E80 | Init (stub) |
| 8 | 0x020 | 0x00459F20 | WhatAmI |
| 10 | 0x028 | 0x0044E8F0 | GetType |
| 18 | 0x048 | 0x00447AC0 | GetCoords |
| 23 | 0x05C | **0x0043FB20** | **Update (AI per-tick)** |
| 37 | 0x094 | 0x00452630 | IsDeployable (corrected 2026-05-28: was listed as CanAcceptUpgrade; binary shows 0x00452630=IsDeployable via get_function_by_address; CanAcceptUpgrade is at 0x00452670, called directly not via vtable — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 41 | 0x0A4 | 0x004500A0 | GetTargetCoords |
| 42 | 0x0A8 | 0x00447B20 | GetDockCoord |
| 53 | 0x0D4 | **0x00445880** | **Limbo (remove from map)** (corrected 2026-05-28: was named "OnDestroyed"; binary shows BuildingClass__Limbo via get_function_by_address + decompile — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 54 | 0x0D8 | **0x00440580** | **Unlimbo (place on map)** |
| 55 | 0x0DC | 0x0044EBF0 | Destroy (corrected 2026-05-28: was "Limbo/Destroy"; binary shows BuildingClass__Destroy via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 64 | 0x100 | **0x00443C60** | **ExitObject (6724 bytes!)** |
| 65 | 0x104 | 0x0043CEA0 | Draw dispatcher |
| 69 | 0x114 | 0x0043D290 | DrawBody |
| 91 | 0x16C | **0x00442230** | **ReceiveDamage** |

Full 300-slot vtable map available in `BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md`.

---

## 5. Lifecycle

### Construction

1. **Constructor** (0x0043B740): Allocates 0x720 bytes, initializes all fields
2. **ReadFromINI** (0x0044F820): Parses INI data, sets up type pointer
3. **Unlimbo** (0x00440580, ~4300 bytes): Places building on map
   - Wall auto-extension
   - Upgrade attachment (finds target building, copies anim name, calls AddUpgrade)
   - Light source creation (BuildingLight)
   - HouseClass array registration (radar, sensor, gap, factory, dock, spysat lists)
   - Cell occupancy updates (marks foundation cells as occupied)
   - Bridge-adjacent passability setup

### Per-Tick Update (AI)

**Update** at 0x0043FB20 handles the per-tick logic. Building missions dispatch through
`MissionClass::Mission_Dispatch` at 0x005B3060.

### Mission Handlers (12 overrides)

| Mission | Enum | Address | Size | Purpose |
|---------|------|---------|------|---------|
| Attack | 1 | 0x0044ACF0 | ~1,174B | Turret targeting, garrison fire, ChargeMode superweapons |
| Retreat | 5 | 0x004496B0 | ~902B | Selling sequence (triggers sell animation) |
| Guard | 8 | 0x0044B760 | ~26B | Default idle state (stub, defers to TechnoClass) |
| Return | 10 | 0x0044B770 | ~16B | Return to base (stub) |
| Stop | 11 | 0x00449A40 | ~8B | Stop current activity (stub) |
| Construction | 18 | 0x00449A50 | **355B** (was ~434B) | Build-up animation playback |
| Selling | 19 | 0x00449C30 | ~3,989B | Full sell sequence: anim → refund → survivor eject → cleanup |
| RepairAndProduce | 20 | 0x0044B780 | ~4,604B | Timer-based repair + production dispatch |
| Missile | 22 | 0x0044C980 | ~3,104B | Nuke silo 5-state machine |
| Unload | 24 | 0x0044E440 | ? | Unload units |
| Eaten/Rescue | 16 | 0x0044D880 | ? | Grinder/rescue |

**Mission_Dispatch aliases**: case 6 (Sleep) → Retreat handler, case 17 (Harvest) → Guard handler (both share slot for buildings).

### Destruction

1. **ReceiveDamage** (0x00442230) → TechnoClass::ReceiveDamage → ObjectClass::ReceiveDamage
2. **Limbo** (0x00445880, vtable slot 53) removes building from map: (corrected 2026-05-28: was called "OnDestroyed"; binary shows BuildingClass__Limbo via decompile at 0x00445880 — ROOT_CAUSE: RTTI_LABEL_DRIFT)
   - 8+ secondary anim slots (field_0x5c8 range)
   - OrePurifier/Helipad/Storage counters on HouseClass
   - Wall connections (LaserFencePost)
   - Sensor arrays / gap shroud removal
   - Cell occupancy removal (foundation pass-through counter)
   - Screen invalidation
   - House recount, radar update
3. **SpawnSurvivors** (0x00442D90) ejects crew infantry
4. **EjectOccupants** (0x004575B0) ejects garrison infantry

---

## 6. Damage & Immunity System

### Damage Pipeline

```
BuildingClass::ReceiveDamage (0x00442230)
  → Building-specific immunity checks
  → TechnoClass::ReceiveDamage (0x00701900)
    → ObjectClass::ReceiveDamage (0x005F5390)
```

### Immunity Checks (ordered)

1. **Self-damage guard** — building cannot damage itself unless flagged
2. **Wall immunity** (BuildingTypeClass+0x16BF) — walls immune unless ignoreDefenses
3. **Insignificant + BridgeRepairHut** (+0x16B6 AND +0x233) — combined immunity
4. **TypeImmune** — same type + same owner = immune
5. **IronCurtain** — timed invulnerability, spark anim (type 1 for IC, type 6 for ForceShield)
6. **WarpingOut** — chrono warp in progress = immune
7. **Radiation + ImmuneToRadiation** (WH+0x177 vs Type+0xD37)
8. **PsychicDamage + ImmuneToPsionicWeapons** (WH+0x178 vs Type+0xD36)
9. **Poison + ImmuneToPoison** (WH+0x156 vs Type+0xD3B)
10. **AffectsAllies=no + allied target** (WH+0x179)
11. **Insignificant** (ObjectClass level, Type+0x233)

### Damage State Transitions

- **ConditionYellow**: health crosses below `Rules+0x1700` (typically 50%)
- **ConditionRed**: health crosses below `Rules+0x1708` (typically 25%)
- **SetDamagedState** (0x00451EE0): Swaps between undamaged/damaged anim arrays
- **CreateDamageFireAnims** (0x0043C0D0): Spawns fire/smoke overlays

### Repair

- Buildings do **NOT** auto-repair. Player-initiated via MISSION_REPAIR (0x0044B780)
- Timer-based: accumulates progress against `Rules->RepairRate`
- Engineer repair (0x00701410): separate instant-full-repair path
- SelfHealing (veterancy): separate TechnoTypeClass flag, checked during tick

---

## 7. Power System

### Output Formula (4 components, health-scaled)

```
base_output = Type+0xEE0
+ (Type+0xEE8 if HasExtraPowerBonus)           // bio-reactor occupants
+ (Type+0xEE8 × docked_unit_count if Absorber) // grinders
+ sum(upgrade[i].Type+0xEE0 for i in 0..UpgradeLevel)

total_output = base_output × GetHealthRatio()   // ONLY if base > 0 AND HasPower
```

### Drain Formula (NOT health-scaled)

```
total_drain = Type+0xEE4
+ (Type+0xEEC if HasExtraPowerDrain)
+ sum(upgrade[i].Type+0xEE4 for i in 0..UpgradeLevel)
```

### Key Functions

- **GetPowerOutput** (0x0044E7B0)
- **GetPowerDrain** (0x0044E880)
- **PowerRatio** (0x004FCE30): output/drain, clamped 0.0-1.0
- **GoOnline** / **GoOffline**: Player toggle via TogglePower=yes
- **Spy blackout**: Zeroes PowerOutput for SpyPowerBlackout frames

---

## 8. Spy Infiltration (0x004571E0)

All 7 effects are active in YR (no TS gating). Priority order — first match wins:

| # | Condition | Effect | Formula/Details |
|---|-----------|--------|-----------------|
| 1 | Same owner | Early return | No effect |
| 2 | `Radar=yes` (+0x16A4) | Shroud reset | MapClass::RestoreShroud. Skipped if victim in low power |
| 3 | `Power > 0` (+0xEE0) | Power blackout | Duration = SpyPowerBlackout frames (default 1000 ≈ 67s) |
| 4 | In BuildTech list (Rules+0x920) | Tech steal | Sets StolenTech flag by AIBasePlanningSide (0=Allied→+0x2BE, 1=Soviet→+0x2BD, else→+0x2BC). Triggers prereq recalc |
| 5 | `SuperWeapon != -1` (+0x16F0) | SW timer reset | Resets charge timer via OnSpyWeaponInfiltrate (0x006CE0B0) |
| 6 | `Storage > 0` (TechnoType+0x800) | Money steal | `stolen = (int)(victim_balance × SpyMoneyStealPercent)`. Default 50% (Rules+0xD68) |
| 7 | `Factory=UnitType` (+0xEB8==0x28) | War Factory spy | Sets HouseClass+0x2C0 (SpiedWarFactory) |
| 8 | `Factory=InfantryType` (+0xEB8==0x10) | Barracks spy | Sets HouseClass+0x2BF (SpiedBarracks) |

**Notable:** Factory=BuildingType and Factory=AircraftType are NOT handled — spying on ConYard or Airfield with no other qualifying trait produces no effect.

---

## 9. Upgrade System

### Storage

3 upgrade slots stored as BuildingTypeClass pointers:
- +0x5EC: Upgrades[0]
- +0x5F0: Upgrades[1]
- +0x5F4: Upgrades[2]
- +0x702: UpgradeLevel (byte, 0-3)

### Lifecycle

1. **CanAcceptUpgrade** (0x00452670): Validates owner match, PowersUpBuilding name, level capacity
2. **Unlimbo** (0x00440580): Finds target at cell, copies Image name into PowerUpNAnim slot, calls AddUpgrade, destroys upgrade building
3. **AddUpgrade** (0x00451400): Heals to full, increments UpgradeLevel, creates PowerUp anim
4. **RemoveLastUpgrade** (0x00451690): Clears anims, decrements level, nulls slot, triggers production recalc

### Effects

- **Power**: Each upgrade's Power= additively summed in GetPowerOutput/GetPowerDrain
- **Weapons**: GetWeapon (0x004526F0) checks upgrade types FIRST — if any upgrade has a weapon, it overrides host building's weapon
- **Animations**: PowerUp1/2/3Anim from art.ini, with healthy/damaged variants and position offsets
- **Health**: Building fully healed when upgrade installed
- **Tech tree**: RemoveLastUpgrade triggers HouseClass::AI_ManageProduction recalc

---

## 10. Docking System

### GetDockCoord (0x447B20) — Where units dock

Dispatch order:
1. **Weeder** (+0x16BC): Fixed offset (2 cells east, 1 cell south)
2. **Refinery** (+0x16BB): Building center + 128 leptons east
3. **Bunker** (+0x16AB): Angle-based 8-direction offset from center
4. **Helipad/UnitRepair** (+0x16CB/+0x16A9): Uses DockingOffset array. Multi-dock uses RadioClass::FindDockSlot (0x65AD90)
5. **Default**: Building center

### GetDockCellForObject (0x44EFB0) — Where units exit

1. **Barracks**: Hardcoded per type — GDI (+1,+2), NOD (+2,+2), Yuri (+2,+1)
2. **Naval Yard** (Naval + WeaponsFactory): Tries 3 adjacent water cells
3. **Fallback cell**: If provided and valid
4. **ExitList**: Foundation exit cell table at 0x89D368 ({short dx, short dy} pairs)
5. **Hospital/NULL ExitList**: Foundation perimeter scan (bottom → top → right → left)

### ExitObject (0x443C60) — Production exit

6724 bytes, dispatches on RTTI type:
- **Aircraft**: Dock at helipad, launch anim
- **Infantry**: Scatter from barracks exit cell
- **Vehicles**: 5-state gate machine (init → clear bib → drive out with locomotor piggyback → wait → close gate)
- **Naval**: Skip gate, special water cell finder

### QueueingCell

Harvesters that can't dock wait at `building_cell + QueueingCell` (from art.ini). Default (0,0).

---

## 11. Garrison System

### Fields

- BuildingTypeClass+0x157B: CanBeOccupied (bool)
- BuildingTypeClass+0x157C: CanOccupyFire (bool)
- BuildingTypeClass+0x1580: MaxNumberOccupants (int)
- BuildingClass+0x684: Occupant DynamicVector (Items at +0x688, Count at +0x694)
- BuildingClass+0x69C: CurrentFireIdx (round-robin index)

### Fire Mechanics

- Weapon comes from occupant's InfantryTypeClass (OccupyWeapon +0xE04 / EliteOccupyWeapon +0xE20)
- **Damage**: base_damage × OccupyDamageMultiplier (RulesClass+0xF40)
- **ROF**: (baseROF / occupant_count) / OccupyROFMultiplier (RulesClass+0xF44)
- **Range**: OccupyWeaponRange (RulesClass+0xF48) replaces weapon range entirely
- Round-robin: CurrentFireIdx incremented after each shot

### Ownership Transfer

- CheckAutoSellOrCivilian (0x00458200) runs per-tick
- Detects "occupied but civilian-owned" → transfers to first occupant's owner
- **1-tick delay** between infantry entry and ownership transfer
- Reverts to civilian when last occupant leaves

---

## 12. Animation System (21-Slot)

21 fixed anim slots, each a pointer in Anims[] at +0x55C:

| Slots | Purpose |
|-------|---------|
| 0-2 | Upgrade anims (PowerUp1/2/3) |
| 3-6 | Activity anims (refinery arm, crane) |
| 7-8 | Production anims (pre/active) |
| 9-13 | Turret/special anims |
| 14-17 | Superweapon charge anims |
| 18-20 | Idle/low-power anims |

### Power Flags (4 per slot)

- **Flag A (Powered)**: Anim stays alive on power-off but detached/reattached
- **Flag B (PoweredLight)**: Anim destroyed on power-off, recreated on power-on
- **Flag C (PoweredEffect)**: Tracks charge state via +0x5B0 array
- **Flag D (PoweredSpecial)**: Triggers special anim on spy blackout

### Damage State Transitions

SetDamagedState (0x00451EE0) at ConditionYellow threshold swaps between undamaged/damaged anim arrays from BuildingTypeClass+0xF4C and +0xF5C.

---

## 13. Wall System

- **LaserFencePost** (+0x16BE): Connection point, 16-frame bitmask for variants
- **LaserFence** (+0x16BF): Fence segment between posts
- Powered fences damage units with C4Warhead
- Functions: ConnectWalls, RecalculateWallConnections, ExtendWallInDirection, OnWallDestroyed, FindNearestFencePost

---

## 14. Gap Generator

4-state machine:
- **0 = Inactive**: No shroud effect
- **1 = Expanding**: Grows from 0 to 15 frames
- **2 = Active**: Full shroud applied per-cell in circular pattern using CloakRadiusInCells
- **3 = Contracting**: Shrinks back

Translucency synced to all 21 anim slots. Neighbor cascading: expanding gap triggers nearby gaps.

---

## 15. TS-Legacy Fields (Present but Dormant in YR)

| Offset | Field | Status |
|--------|-------|--------|
| +0x16B6 | BridgeRepairHut | TS-only, default false in YR |
| +0x16BA | NukeSilo | YR-active (NAMISL only) |
| +0x16C0 | FirestormWall | TS-only, no YR usage |
| +0x16C3 | EMPulseCannon | TS-only (Mission_Missile has dormant branch) |
| +0x16C4 | TickTank | TS-only |
| +0x16C7 | CloakGenerator | **YR-ACTIVE** — gap generator runtime flag; UpdateGapGenerator_Tick and UpdateGapAndSpecialEffects gate on this field (corrected 2026-05-28: was "TS-only"; binary shows active use in YR gap generator tick — ROOT_CAUSE: TS_LEGACY_AS_YR) |
| +0x16CA | Artillary | TS-only, default false |

**Self-cloaking buildings** (`Cloakable=yes`): code path exists but no retail YR building uses it.

---

## 16. Per-Tick Update Pipeline (0x0043FB20)

The BuildingClass::Update function (~2,650 bytes) runs every tick. 27-step pipeline:

1. **Power state tracking** — `IsOperational` (vtable+0x350) checks HasPower, EMP, health, power ratio
2. **UpdateGapAndSpecialEffects** (0x004549B0) on state change — robot tank online/offline, cloak generator expand/contract, sensor array toggle, mind control release, chrono warp abort
3. **Damage fire anims** — `CreateDamageFireAnims` (0x0043C0D0) at ConditionYellow/Red crossings, 8 slots at +0x5C8
4. **ProduceCash (Oil Derrick)** — Timer at +0x6D0/+0x6D8 counts down using `ProduceCashDelay` (Type+0x1560), grants `ProduceCashAmount` (Type+0x155C) when expired
5. **Gap generator tick** — `UpdateGapGenerator_Tick` (0x00454DB0) 4-state machine with +0x6ED radius (0..15)
6. **UpdateAnimation** (0x004509D0) — frame timers, turret facing, garrison fire anims, radar dish, ore storage, SW charge staging
7. **TechnoClass::AI_Update** (0x006F9E50) — mission dispatch, target validation, self-healing, EMP countdown, capture manager, cloak visibility, veterancy, retaliation
8. **UpdateRepairAndPower** (0x00450630) — AI repair decisions, per-tick HP/cost, anim updates on health thresholds
9. **Auto-production** (FUN_004500F0) — Cloning Vats manages auto-produce
10. **ProcessDelayedFire** (0x004503F0) — handles DelayedFireDelay timer
11. **Destruction sequence** — At 0 HP, waits death timer, calls SpawnSurvivors + Limbo

See `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` for the full 27-step breakdown.

---

## 17. Mission Handlers — Verified Details

### Mission_Attack (0x0044ACF0)

**Two completely separate paths** based on `IsChargeMode` (BuildingTypeClass+0x16B8):
- **Charge mode** (Tesla Coil): 3-state charge machine, facing tolerance ~45° (0x2001 in 0-0xFFFF system)
- **Direct fire**: 11-entry jump table on GetFireError results. Turret gets two rotation chances per tick — if FIRE_FACING, rotate by ROT (+0x71C) and re-check

**Garrison fire round-robin:** Index at **BuildingClass+0x69C** (GarrisonFireIndex — verified correct). GetWeapon returns current occupant's OccupyWeapon/EliteOccupyWeapon. Index increments in `TechnoClass::Fire_At` after each shot.

**Gattling decay:** Buildings lose gattling stage via `UpdateGattlingStage` (0x0070E000) when target invalid.

**DelayedFireDelay** (+0x16EC) is handled by `ProcessDelayedFire` (0x004503F0) in the per-tick loop, NOT in Mission_Attack itself.

### Mission_Selling (0x00449C30) — 3-State Machine

State at BuildingClass+0xBC:
- **State 0**: Init — if upgrades exist, early-return pops LAST upgrade at **FULL COST refund** (not SellBack%), calls `RemoveLastUpgrade`, queues GUARD. User must sell N times to strip N upgrades.
- **State 1**: Eject + animate — ejects occupants (units via passenger list pop + `PlaceInfantryInCell`, bunker garrison via `SellBuilding` at 0x00457DE0), plays `GrandOpening(0)` reverse construction
- **State 2**: Finish — waits for +0x6DD flag, does cleanup

**Refund formula (CORRECTED):** `Cost × Rules.SellBack (+0x145C, default 50%) + stored ore`. **NO health scaling** — sick building refunds same as full HP. Prior docs said health-scaled — this is wrong for sell; health-scaling only applies to the MCV undeploy HP calculation.

**MCV undeploy:** Type+0x408 (`UndeploysInto=`) non-null triggers `UnitClass` constructor. MCV health = `floor(HealthRatio × UnitType.Strength)` floored at 1. Inherits radar jam, gap-gen state, cloak shroud mask, sound loops. All radio-linked units re-bound.

**Survivor count:** `GetSurvivorCount` (vtable+0x2D0 at 0x00451330). Formula: `clamp(Cost / SurvivorDivisor[side], 1, 5)`. Bio-reactor doubles divisor. Zero on bridge or if `Crewed=no`. Divisors at Rules+0x14F8/+0x14FC/+0x1500 (Allied/Soviet/Third).

**Survivor infantry type:** `GetSurvivorInfantryType` (vtable+0x30C at 0x0044EB10). 25% Engineer chance if Soviet-side AND not bio-reactor. Otherwise AlliedCrew/SovietCrew/ThirdCrew by side. 15% Technician override if `vtable+0x2AC` (Is_Weapon_Equipped). Rules offsets: 0xF6C Technician, 0xF70 Engineer, 0xF78 AlliedCrew, 0xF7C SovietCrew, 0xF80 ThirdCrew.

**On-bridge quirk:** Buildings on bridges DELETE their garrison instead of ejecting.

### Mission_Missile (0x0044C980) — 5-State Nuke Silo

**Gated ONLY by NukeSilo flag (+0x16BA).** ICBMLauncher (+0x16C9) is NOT tested here — that's a separate subsystem. EMPulseCannon (+0x16C3) is dormant TS legacy.

State counter at BuildingClass+0xBC:
- **State 0**: `GrandOpening(2)`, clear +0x6DD, create PSIWARN anim at target cell (from Owner+0x5784), store ptr at +0x54C → state 1
- **State 1**: Wait for +0x6DD ≠ 0 (doors open), `GrandOpening(4)` → state 2
- **State 2**: Lookup SW via +0x5F8, allocate BulletClass (NukeCarrier), release PSIWARN, compute vertical velocity (sin/cos π/2), fire bullet, create NUKETO anim at silo (from Rules+0x98) → state 3 (returns 1)
- **State 3**: `GrandOpening(5)` (close doors) → state 4 (returns 6)
- **State 4**: `GrandOpening(5)` + `Queue_Mission(GUARD)` → returns 60

**SW-charge interaction:** `Mission_Missile` does NOT touch +0x6D0 (that's ProduceCashTimer). SuperClass::Launch writes target to HouseClass+0x5784, SW index to building+0x5F8, then dispatches Mission 22.

---

## 18. Receive_Radio Protocol (0x0043C2D0, vtable slot 101)

BuildingClass handles 9 message cases, delegates the rest to TechnoClass::Receive_Radio (0x006F4AB0), which delegates to RadioClass::Receive_Radio (0x0065A820).

| Msg | Name | Direction | BuildingClass Behavior |
|-----|------|-----------|------------------------|
| 0x03 | OVER_AND_OUT | any | GrandOpening reset + delegate |
| 0x08 | REQUEST_CLEARANCE | U→B | Near-range ROGER for UnitRepair/Bunker; WeaponsFactory returns QUEUED (0x17) |
| 0x0B | DOCK_APPROACH | B→U | Queue_Mission(UNLOAD=0x14) |
| 0x0C | DOCK_ARRIVED | U→B | Queue_Mission(GUARD); if ConYard, rebuild ambient anim |
| 0x0D | — | — | Silent ROGER for WeaponsFactory |
| 0x0E | CAN_DOCK | U→B | Establishes link, computes queue cell (+3,+1) for Refinery/Weeder, MOVE_TO_CELL + ENTER_DOCK + TIMING_SYNC |
| 0x0F | CAN_ENTER | U→B | Passenger/garrison entry — gated by UnitRepair/Bunker/UnitAbsorb/InfantryAbsorb/Grinding/Hospital/Armory/Helipad |
| 0x10 | RESERVE_DOCK | U→B | ROGER only for harvester + same owner + idle |
| 0x15 | DOCK_NOW | U→B | Sets +0x6DD=1 + Queue_Mission(UNLOAD); for Refinery sends sender ENTER |

**Repair (0x1C) is TechnoClass-level.** Computes money/tick via vtable+0xB0, HP/tick via vtable+0xB4. Returns INSUFFICIENT_FUNDS (0x20) / REPAIR_COMPLETE (0x21) / ROGER (0x01).

---

## 19. CloakGenerator / SensorArray Mechanics

### CloakGenerator (+0x16C7) — YR-Active Gap Generator flag (corrected 2026-05-28)

**CORRECTION:** Prior doc said "TS-legacy dormant, no retail YR building sets this flag." This is WRONG. (corrected 2026-05-28: binary shows UpdateGapGenerator_Tick at 0x00454DB0 and UpdateGapAndSpecialEffects at 0x004549B0 gate gap shroud logic on BuildingTypeClass+0x16C7; Mission_Sell at 0x00449C30 also checks Type+0x16C7 for gap generator shutdown. The separate `GapGenerator=` INI key (TechnoTypeClass+0xCD1) controls a different but related path. The shroud expand/contract behavior gated by +0x16C7 is ACTIVE in YR. The INI key `CloakGenerator=` (TS-era name) maps to +0x16C7 and is the runtime cloakgen/gap flag — ROOT_CAUSE: TS_LEGACY_AS_YR mislabeled dormant.)

**Important:** `GapGenerator=yes` (INI key) maps to TechnoTypeClass+0xCD1 (verified: xref from 0x00713f99 in TechnoTypeClass__ReadINI). The field at BuildingTypeClass+0x16C7 is the internal `CloakGenerator` flag used by the gap generator runtime tick — these two fields coexist. GAGAP sets both.

- Expansion/contraction driven by 3 BuildingClass bytes: +0x6EB (direction: 1/-1/0), +0x6EC (current radius), +0x6ED (visual stage 0-16)
- Grows **one cell radius per tick** (not instant)
- Uses `TechnoClass::UpdateCloakShroud` — increments GapOverlayCount (CellClass+0x134) and GapShroudLevel (+0x130) on covered cells
- Does NOT call `Cloak()` on units — only shrouds cells
- `DoUncloak` called on units only when cells are REMOVED (forces visibility recheck)

### SensorArray (+0x16C8) — YR-Active

Used by Psychic Sensor and Spy Satellite.

- Uses CellClass+0x7C short-array-per-house counter
- **AddSensorArrayAt**: increments counter + calls `DoUncloak` on Units/Infantry/Aircraft in cell
- No subterranean-specific logic (subterranean is TS-only)

### AddSensorArrayAt vs RemoveSensorArrayAt — Radius Fields

- `AddSensorArrayAt` reads **Type+0x5F0 (SensorsSight, int)**
- `RemoveSensorArrayAt` reads **Type+0x1707 (CloakRadiusInCells, byte)**
- These are **two different fields**. The constructor initializes `Type+0x1707 = 0x14 (20)` as the default, so the remove is not a total no-op — but if SensorsSight differs from CloakRadiusInCells the radii won't match and counters may leak at the boundary.
- **Verification note:** The "sensor counter leak on destruction" claim from an intermediate agent was based on the assumption `Type+0x1707 = 0`. Actual default is **0x14 (20)** per constructor. If the INI does NOT set CloakRadiusInCells, the 20-cell default is used on remove. If SensorsSight is set to a different value (e.g., 15), there's a mismatch — remove clears 20-cell radius while add populated only 15-cell. Reverse case (Sensor adds 20, removes 20) works cleanly if both default.
- **For Rust implementation:** Use the same field (SensorsSight) consistently for both add and remove to avoid any mismatch.

### Overlapping Fields — Reference Counted

- SensorCount[house] and DisguiseDetectCount[house] accumulate; visibility check is `> 0`
- GapOverlayCount + GapShroudLevel use double-counter: only when GapOverlayCount hits 0 does GapShroudLevel decrement
- Overlapping gap generators coexist correctly

---

## 20. Special Buildings — Verified Mechanics

### Cloning Vats (+0x16AC)

In `ExitObject_Main` at offset 0x004449FB: when a **Barracks** (Type+0xEB8==0x10) produces infantry AND the barracks itself is NOT a Cloning Vat (Type[0x16AC]==0), iterate `HouseClass+0xFC` (Cloning list), call `vtable+0x100` on each vat to spawn a duplicate infantry.

### Grinding (+0x16AD)

In `Mission_Enter` at 0x005196A0: `Add_Credits(unit->vtable[0x2BC]())`. That's `GetRefundValue` which reads **TechnoTypeClass.Soylent at +0x614**. Passengers and mind-control slaves refunded recursively.

### Hospital (+0x16C1) / Armory (+0x16C2)

Both handled in `Mission_RepairAndProduce` (0x0044B780) with identical timer:
- `field_0x638` accumulates into `field_0x620`
- Triggers when `field_0x620 >= Rules.IRepairRate (Rules+0x16F0) × 900.0`
- Constant stored at DAT_007E27F8
- Hospital: heals + ejects occupant
- Armory: calls SetVeteran or SetElite on occupant, then ejects
- **No dedicated INI key** — both use Rules.IRepairRate

### InfantryAbsorb / Bio Reactor (+0x16AF)

In `GetPowerOutput` (0x0044E7B0): `power += Type.ExtraPower (+0xEE8) × NumPassengers (BuildingClass+0x114)` when InfantryAbsorb AND ExtraPower>0.

**VERIFIED:** Bio-reactor and garrison are SEPARATE systems:
- **Bio-reactor**: Uses the embedded `CargoClass` at BuildingClass+0x114 (inherited from TechnoClass) — same mechanism as transport helo passengers. Entry happens via `CargoClass::AddPassenger`. Gated by `InfantryAbsorb=` / `UnitAbsorb=` (Type+0x16AE/0x16AF).
- **Garrison**: Uses dedicated `DynamicVector<InfantryClass*>` at BuildingClass+0x684..+0x69C. Gated by `CanBeOccupied=` (Type+0x157B) + `InfantryTypeClass.Occupier` (+0xEB4).
- The DynamicVector at +0x66C-+0x67C is UNRELATED to either (used by PowerCheck_Upgrade for upgrade iteration).

### SecretLab (+0x16B0)

- Pool concatenated from Rules.SecretInfantry (+0xD00), SecretUnits (+0xD1C), SecretBuildings (+0xD38)
- Fisher-Yates sample per lab
- Registry via 0x00442C40; assignment at 0x0068C050
- **Note:** The storage offset for the chosen TechnoTypeClass* on BuildingClass is not fully verified — earlier claim of +0x6F4 could not be confirmed (field is zeroed at constructor, no writer site located). Storage may be elsewhere or inferred from lab's Type pointer.

### OrePurifier (+0x16CC)

In `DepositOreFromStorage` (0x00522D50): `bonus = NumOrePurifiers × Rules.PurifierBonus (+0xF3C) × amount`.
- Counter at HouseClass+0x538C
- AI gets extra from Rules+0x1324[difficulty]

### FactoryPlant (+0x16CD)

- Per-building floats at Type+0x16D0..+0x16E0 (Infantry/Units/Aircraft/Buildings/Defenses CostBonus)
- `RecalcBonuses` at 0x0050BF60: multiply stacking into HouseClass+0x5390..+0x53A0
- `GetAccumulatedBonus` at 0x0050BEB0: applied at cost lookup, dispatches on RTTI kind (vtable+0x2C: 0x10 Infantry, 0x28 Unit, 0x03 Aircraft, 0x07 Building with Type[0x382]==5 → Defense)

---

## 21. BuildingClass Field Corrections (After 2 Verification Rounds)

Verified 2026-04-16 against direct Ghidra decompilation from multiple call sites. Supersedes all earlier claims.

| Offset | Final Verdict | Evidence |
|--------|---------------|----------|
| +0x114 | **CargoClass::NumPassengers** (embedded CargoClass, inherited from TechnoClass) — bio-reactor reads this via GetPowerOutput and Receive_Radio | ✓ HIGH — writes via CargoClass::AddPassenger (14+ sites) |
| +0x520 | **Type (BuildingTypeClass\*)** | ✓ HIGH — verified in 5+ functions |
| +0x524 | **Factory (FactoryClass\*)** — EPHEMERAL (null when not producing) | ✓ HIGH — destructor releases via vtable+0x20 |
| +0x5B0 | **NO CHARGE FLAG ARRAY** — prior claim debunked | ✓ HIGH — no function accesses this range |
| +0x600 | **BuildingLightClass\*** (conditional spotlight on Type+0x154B) | ✓ HIGH |
| +0x614 | **LightSourceClass\*** (ambient light, conditional on Type+0xE30..+0xE40) | ✓ HIGH — destroyed in dtor |
| +0x660 | **HasPower** | ✓ HIGH |
| +0x661 | **IsOverpowered** (NOT HasExtraPowerBonus as earlier claimed) | ✓ HIGH — PowerCheck_Upgrade sets at 0x00450614 |
| +0x662 | Cleared at ctor, no consumer verified | ? UNVERIFIED |
| +0x668 | **HasExtraPowerBonus** (NOT a "copy" — this is the real field) | ✓ HIGH — GetPowerOutput reads at 0x0044E7D5 |
| +0x669 | **HasExtraPowerDrain** (NOT a "copy" — this is the real field) | ✓ HIGH — GetPowerDrain reads at 0x0044E89F |
| +0x66C-+0x67C | DynamicVector (purpose unclear; NOT bio-reactor) | ✓ HIGH |
| +0x694 | **Occupant Count** (DynamicVector.Count field) | ✓ HIGH — GetOccupantCount, AddGarrisonOccupant, PointerExpired |
| +0x69C | **GarrisonFireIndex** (garrison round-robin, wraps via `%= count`) | ✓ HIGH — GetWeapon, Fire_At, constructor init |
| +0x6F4 | Storage location of SecretLab pick — NOT confirmed | ✗ LOW |

**BuildingTypeClass corrections:**

| Offset | Final Verdict | Evidence |
|--------|---------------|----------|
| +0x16A9 | **UnitRepair** (Service Depot flag) | ✓ HIGH — ReadINI string "UnitRepair" |
| +0x16C7 | **CloakGenerator** flag (NOT SensorArray) — gates UpdateGapGenerator_Tick | ✓ HIGH |
| +0x16C8 | **SensorArray** flag | ✓ HIGH |
| +0x1577 | **CanC4** (infantry can place C4 on this) | ✓ HIGH — not +0x16A9 |
| +0x1707 | `CloakRadiusInCells` byte, **default 0x14 (20)** — constructor initializes | ✓ HIGH — verified in BuildingTypeClass ctor |

**Ghidra labels to fix:**
- `BuildingClass__UpdateGarrisonFire` at 0x0043E7B0 is **mislabeled** — actually draws the factory queue preview (calls FactoryClass::GetObject + CC_Draw_Shape). Recommended rename: `DrawFactoryQueuePreview`.

**Debunked claims (remove from all docs):**
- +0x5B0 21-byte ChargeFlags array — does not exist; the "21" came from the Anims array at +0x55C (21 DWORDs)
- CloakGenerator active in YR — no retail YR building sets the +0x16C7 flag; system is TS-legacy
- +0x66C DynamicVector as "absorb/occupant tracking" — actually unrelated to absorb (occupants are at +0x684-+0x694; bio-reactor count is scalar at +0x114)

---

## 22. Current Rust Implementation Status

### Implemented
- Power system (generation, consumption, low-power, health scaling, spy blackout)
- Repair depot docking (state machine, FIFO queue, credit costs)
- Building placement validation (terrain, overlap, build area, foundation)
- Tech tree and prerequisites (including PrerequisiteOverride)
- Building sell with crew ejection and refunds (50% health-scaled)
- Repair system (toggle, credit-based restoration)
- Production queues and factory matching
- Radar and SpySat functionality
- Building animation overlays (crane, one-shot, damage fires, garrison muzzle flash)
- Garrison occupancy tracking (flags parsed)

### Not Implemented or Partial
- Garrison fire logic (flags exist but targeting/firing not wired)
- Infantry/Unit absorption (InfantryAbsorb/UnitAbsorb flags exist, no mechanics)
- Upgrade system (fields exist, no installation/removal logic)
- Spy infiltration (only power blackout implemented)
- Wall/laser fence connectivity
- Gap generator logic (flag only)
- Sensor array / cloak generator field effects
- Building-specific ExitObject dispatch (barracks/WF/naval exit patterns)
- Superweapon activation
- Building capture (engineer)
- Cloning vats, grinding, hospital, armory

---

## 23. Open Questions (Remaining)

Resolved in this round:
- ✓ SensorRange/CloakRadiusInCells: Two fields used asymmetrically. Default of CloakRadiusInCells is 0x14 (20), NOT 0 as an intermediate agent claimed. Mismatch only occurs when SensorsSight is explicitly set to a different value.
- ✓ Absorb DV at +0x66C: **Not absorb-related** — occupant count for bio-reactor uses scalar +0x114 (verified in GetPowerOutput).
- ✓ GarrisonFireIndex: **+0x69C is correct** (verified). An intermediate agent's claim of +0x664 was wrong.
- ✓ +0x5B0 ChargeFlags: **Does not exist** (verified). No function accesses this range. The "21" came from the Anims array at +0x55C.
- ✓ +0x524: **Confirmed FactoryClass\*** (destructor releases via vtable+0x20).
- ✓ +0x600 / +0x614: Two separate light pointers — spotlight (conditional) vs ambient (all buildings).

Still open:
1. **ChargeFlags at +0x5B0**: Still not verified from any decompiled function. Needs targeted investigation of PoweredEffect anim logic.
2. **+0x524 Factory pointer**: Referenced in destructor — likely the FactoryClass* when this building IS a factory. Needs confirmation from production code path.
3. **+0x600 BuildingLight pointer**: Lifecycle (creation/destruction) not fully traced.
4. **Soviet Engineer enum value** in GetSurvivorInfantryType (appears to be field_0xEB8 == 7): needs verification.
5. **MCV unlimbo-fail refund path**: the FPU trace when MCV placement fails during sell was not fully followed.
6. **CloakGen tick-down final cleanup**: what triggers the final UnInit call after the radius retracts.
7. **Type+0xE70 sound**: whether this is a single index or a list/array of sound entries.

---

## Sources

### Ghidra Functions Decompiled (this investigation)
- 0x004571E0 — OnSpyInfiltrate (962 bytes)
- 0x0050BC90 — HouseClass::SpyPowerSabotage
- 0x00577AB0 — MapClass::RestoreShroud
- 0x006CE0B0 — OnSpyWeaponInfiltrate
- 0x00442230 — ReceiveDamage
- 0x00445880 — OnDestroyed
- 0x00442D90 — SpawnSurvivors
- 0x00451EE0 — SetDamagedState
- 0x0043C0D0 — CreateDamageFireAnims
- 0x00453240 — OnWallDestroyed
- 0x00457C90 — IronCurtain
- 0x004575B0 — EjectOccupants
- 0x0044B780 — MissionRepairAndProduce
- 0x00701900 — TechnoClass::ReceiveDamage
- 0x005F5390 — ObjectClass::ReceiveDamage
- 0x00452670 — CanAcceptUpgrade
- 0x00440580 — Unlimbo
- 0x00451400 — AddUpgrade
- 0x00451690 — RemoveLastUpgrade
- 0x004526F0 — GetWeapon (upgrade override)
- 0x00443C60 — ExitObject_Main (6724 bytes)
- 0x00447B20 — GetDockCoord
- 0x0044EFB0 — GetDockCellForObject
- 0x00449540 — ClearBibArea
- 0x0065AD90 — RadioClass::FindDockSlot
- 0x004FB0E0 — HouseClass::Place_Production
- 0x0043B740 — Constructor
- 0x0043BCF0 — Destructor
- 0x0043FB20 — Update (AI per-tick)
- 0x0043D290 — DrawBody
- 0x0043CEA0 — Draw dispatcher
- 0x0044EBF0 — Limbo/Destroy
- 0x0044ACF0 — Mission_Attack
- 0x0044B760 — Mission_Guard
- 0x004496B0 — Mission_Retreat
- 0x00449A50 — Mission_Construction
- 0x00449C30 — Mission_Selling
- 0x0044C980 — Mission_Missile
- 0x0045FE50 — BuildingTypeClass::ReadINI
- 0x004653C0 — BuildingTypeClass constructor

### Existing Reports Referenced
- `BUILDING_SYSTEMS_GHIDRA_REPORT.md`
- `BUILDING_ANIM_STATE_MACHINE.md`
- `GARRISON_SYSTEM_GHIDRA_REPORT.md`
- `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md`
- `FACTORYCLASS_PRODUCTION_DEEP_DIVE.md`
- `POWER_SYSTEM_GHIDRA_REPORT.md`

### INI Files Checked
- `ini/rulesmd.ini`
- `ini/artmd.ini`
