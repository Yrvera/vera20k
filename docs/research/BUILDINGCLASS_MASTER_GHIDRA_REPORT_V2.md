> **SUPERSEDED by `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md` (2026-04-24).**
> This file is kept as historical reference. See v3 Change Log for corrections.

---
name: BuildingClass Master Ghidra Research Report (v2)
description: Canonical reference for BuildingClass in gamemd.exe. Integrates 4 verification rounds plus MissionRepairAndProduce and Mission_Attack deep-dives. Supersedes BUILDINGCLASS_MASTER_GHIDRA_REPORT.md (v1, 2026-04-16).
type: reference
---

# BuildingClass — Master Ghidra Research Report v2

**Date:** 2026-04-19 (v2, integrates 4 verification rounds + 2 deep-dives)
**Supersedes:** `BUILDINGCLASS_MASTER_GHIDRA_REPORT.md` (v1, 2026-04-16)
**Binary:** gamemd.exe
**Confidence:** HIGH (all findings verified from direct decompilation)
**Active in YR:** Yes — BuildingClass is the core building runtime class

## Change Log from v1

| Round | Changes |
|---|---|
| R1 | +0x5B0 **re-confirmed** as 21-byte AnimStates (v1 claimed debunked — wrong); +0x524 confirmed FactoryClass*; +0x600 lifecycle traced; Soviet Engineer rule **corrected** (Factory==7/ConYard, not Soviet-side); MCV refund confirmed non-health-scaled; CloakGen UnInit is implicit; Type+0xE70 is single sound index |
| R2 | +0x6E3 identified as OwnershipChanged flag; Type+0x154B INI key = `HasSpotlight`; gap-gen state at +0x220 (NOT +0xBC); vtable slot 280 = StartCloaking; +0x4DC..+0x4F4 = TechnoClass SoundEvent fields |
| R3 | Kind enum complete (1=Unit, 2=Aircraft, 6=Building, 0xF=Infantry); GetDockCellForObject naval 3-cell search; ClearBibArea documented; FUN_0065ADC0 = `RadioClass::HasFreeSlot`; AI build queue at Owner+0x5704 mapped |
| MR&P | Mission_RepairAndProduce 7-mode dispatch + Bunker 6-state + Hospital/Armory 2-state + Repair Depot 3-state + Helipad radio cycle. "5-state gate machine" from v1 is **conceptual, not real** |
| MA+R4 | Mission_Attack direct-fire + charge-mode 3-state; 11-entry GetFireError jump table; URepairRate = Rules+0x16E8; Rules+0x16F8 hardcoded 1.0; DriveLocomotion CLSID = DAT_007E9AB0 |
| 2026-04-23 audit | Corrections: §4 slot 37 is `FUN_00452630`, NOT `CanAcceptUpgrade` (that's called directly, not virtual); §7 drain formula now notes `HasPower==false` zeroes drain; §17 Path A "no target" calls Queue_Mission(5 = Sleep/Retreat), NOT Guard; §17 Path B State 0 power check gated by `Type+0x1573 && Drain>0`, not universal; §19 SensorArray add/remove addresses (`0x00455820` / `0x004556D0`) and radius-asymmetry impact on Psychic Sensor; §20 FactoryPlant `GetAccumulatedBonus` switch cases corrected to 3/7/0x10/0x28 (TechnoTypeClass kind enum, distinct from BuildingClass `What_Am_I`) |
| 2026-04-23 pass 2 | §3 Factory= enum corrected (3=Aircraft, 7=Building, 0x10=Infantry, 0x28=Unit — was wrongly listed as 1/2/7/0x10); §4 vtable slot 254/255 split (254 @ 0x3F8 = GetWeapon; 255 @ 0x3FC = HasTurret); §12 anim slot role table rebuilt with binary-verified per-slot bindings and instance-side playback state fields (+0x0F8..+0x110, +0x534); §25 question #3 resolved (Type+0x16C5 = `TurretAnimIsVoxel`) |

## Companion / Detail Reports

Earlier reports remain accurate for subsystems not re-verified here:
- `BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md` — 300-slot vtable map
- `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` — 27-step per-tick pipeline
- `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` — v1 combat notes (partially superseded here)
- `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` — Mission_Selling
- `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md` — Cloning/Grinding/Hospital/Armory/BioReactor/SecretLab/OrePurifier/FactoryPlant
- `BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md` — CloakGenerator/SensorArray
- `BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md` — Nuke silo 5-state, Receive_Radio
- `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md` — 7 spy effects
- `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` — 11-check immunity
- `BUILDING_UPGRADE_SYSTEM_GHIDRA_REPORT.md` — 3-slot upgrades
- `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` — Per-building dock/exit
- `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` — 19-step ownership transfer
- `GARRISON_SYSTEM_GHIDRA_REPORT.md` — Garrison fire mechanics
- `BUILDINGCLASS_OPEN_QUESTIONS_VERIFICATION.md` — R1
- `BUILDINGCLASS_OPEN_QUESTIONS_VERIFICATION_R2.md` — R2
- `BUILDINGCLASS_OPEN_QUESTIONS_VERIFICATION_R3.md` — R3
- `BUILDINGCLASS_MISSION_REPAIR_AND_PRODUCE.md` — Mission_RepairAndProduce deep dive
- `BUILDINGCLASS_MISSION_ATTACK_AND_RESIDUALS.md` — Mission_Attack + residuals

---

## 1. Overview & Inheritance

BuildingClass is the runtime instance class for all buildings. Inherits
directly from **TechnoClass** — does NOT go through FootClass. Instance size
**0x720 bytes** (1824). BuildingTypeClass (template) size **0x1798 bytes** (6040).
Both sizes verified via `operator_new`.

```
IUnknown (COM)
  └─ AbstractClass      vtable @ 0x007E1F50
    └─ ObjectClass      vtable @ 0x007EF060
      └─ MissionClass   vtable @ 0x007EDCC0
        └─ RadioClass   vtable @ 0x007F0508
          └─ TechnoClass vtable @ 0x007F4960
            └─ BuildingClass vtable @ 0x007E3EBC
```

### Kind Enum (vtable slot 11 — `What_Am_I`)

Distinct from slot 8 (`AbstractClass::WhatAmI` which returns type-class
instance index). Slot 11 returns a constant class-kind tag:

| Value | Class | Constant source |
|---|---|---|
| 1 | `UnitClass` | `0x00746E20` — returns 1 |
| 2 | `AircraftClass` | (inferred from ExitObject case 2; not directly decompiled) |
| 6 | `BuildingClass` | `0x00459EC0` — returns 6 |
| 0xF | `InfantryClass` | `0x00523340` — returns 0xF |

Used by `ExitObject`, FactoryPlant bonus dispatch, and other polymorphic
branching.

---

## 2. BuildingClass Instance Layout (+0x000 to +0x720)

Legend: ✓ = verified from decompilation across multiple call sites.
Fields below +0x500 are TechnoClass-inherited.

### TechnoClass-inherited region (relevant subset)

| Offset | Type | Field | Evidence |
|---|---|---|---|
| +0x114 | int | `CargoClass::NumPassengers` (embedded, used for bio-reactor absorb count) | ✓ R1 |
| +0x21C | ptr | Owner (HouseClass*) | ✓ |
| +0x2B4 | ptr | Current target (TechnoClass*) | ✓ Mission_Attack |
| +0x4DC..+0x4EF | 20 bytes | **SoundEvent struct** (4 DWORDs + 4 pad) for looping sound | ✓ R2 |
| +0x4F0 | int | Sound loop handle #1 (-1 = none) | ✓ R2 |
| +0x4F4 | int | Sound loop handle #2 (-1 = none) | ✓ R2 |
| +0x504 | int | EMPLockRemaining | ✓ |

### BuildingClass-specific region (+0x520 onward)

| Offset | Type | Field | Evidence |
|---|---|---|---|
| +0x520 | ptr | **Type** (BuildingTypeClass*) | ✓ |
| +0x524 | ptr | **Factory** (FactoryClass*) — Cloning Vats auto-produce (NOT just destructor) | ✓ R1 |
| +0x528..+0x530 | | Temporal/chrono timer fields | ✓ |
| +0x534 | int | DamagedState flag | ✓ |
| +0x55C..+0x5AF | ptr[21] | **Anims[21]** array | ✓ |
| +0x5B0..+0x5C4 | byte[21] | **AnimStates[21] / ChargeFlags** — PoweredEffect active flag per slot | ✓ R1 |
| +0x5C8..+0x5E7 | ptr[8] | Secondary anim/fire pointers | ✓ |
| +0x5EC | ptr | Upgrades[0] (BuildingTypeClass*) | ✓ |
| +0x5F0 | ptr | Upgrades[1] | ✓ |
| +0x5F4 | ptr | Upgrades[2] | ✓ |
| +0x5FC | int | Cycling anim phase index (nuke reactor special) | ✓ |
| +0x600 | ptr | **BuildingLightClass*** (spotlight, if Type+0x154B `HasSpotlight=yes`; size 0xE8, ctor `0x00435820`, released via vtable+0xF8) | ✓ R2 |
| +0x614 | ptr | **LightSourceClass*** (ambient light, all buildings with Type+0xE30..0xE40 set; size 0x4C, ctor `0x00554760`) | ✓ |
| +0x618 | int | Wall orientation metadata (0x0/0x4/0x8/0xC) | ✓ Unlimbo |
| +0x620 | int | **Timer accumulator** (heal/repair/production progress) | ✓ MR&P |
| +0x624 | byte | "Timer fired this tick" flag | ✓ MR&P |
| +0x628 | int | CDTimer start frame | ✓ MR&P |
| +0x62C | int | CDTimer aux | ✓ MR&P |
| +0x630 | int | CDTimer rate | ✓ MR&P |
| +0x634 | int | CDTimer active flag (0 = paused) | ✓ MR&P |
| +0x638 | int | **Step amount** per CDTimer fire (added to +0x620) | ✓ MR&P |
| +0x660 | byte | **HasPower** | ✓ GoOnline/PowerCheck |
| +0x661 | byte | **IsOverpowered** | ✓ PowerCheck_Upgrade @0x00450614 |
| +0x662 | byte | (cleared at ctor; robot-tank online flag in UpdateGapAndSpecialEffects) | ✓ R1 |
| +0x664 | int | Misc reset flag (cleared in Mission_Attack no-target) — **NOT the garrison index** | ✓ R4 |
| +0x668 | byte | **HasExtraPowerBonus** | ✓ GetPowerOutput @0x0044E7D5 |
| +0x669 | byte | **HasExtraPowerDrain** | ✓ GetPowerDrain @0x0044E89F |
| +0x66C..+0x67F | | DynamicVector (upgrade iteration; NOT absorb) | ✓ |
| +0x684..+0x697 | | **Occupant DynamicVector** (garrison InfantryClass*) | ✓ |
| +0x694 | int | Occupant Count (GetOccupantCount reads directly) | ✓ |
| +0x69C | int | **GarrisonFireIndex** (round-robin) | ✓ R1 |
| +0x6C9..+0x6CB | byte | ReadFromINI init flags | ✓ |
| +0x6D0..+0x6D8 | | CDTimer (**ProduceCashTimer**) | ✓ OnConstructionComplete |
| +0x6DC | byte | SellBuilding/NominalPower flag | ✓ |
| +0x6DD | byte | ConstructionComplete flag | ✓ |
| +0x6DF | byte | ForceShield active flag | ✓ |
| +0x6E3 | byte | **OwnershipChanged / Captured flag** (set in ChangeOwner, reduces crew bounty; NOT bio-reactor) | ✓ R2 |
| +0x6EB | byte | CloakGenerator direction (0 / 1 / 0xFF=-1) | ✓ |
| +0x6EC | byte | CloakGenerator current radius | ✓ |
| +0x6ED | byte | Gap generator visual stage (0-16) | ✓ |
| +0x6F0 | int | Refinery ore level state | ✓ |
| +0x700 | short | (unknown) | — |
| +0x702 | byte | **UpgradeLevel** (0-3) | ✓ |
| +0x718 | int | **Bunker docking sub-state** (0-6, separate from +0xBC) | ✓ MR&P |

### Special/dual-state fields

| Offset | Type | Field | Evidence |
|---|---|---|---|
| +0xBC | int | MissionClass sub-state (used by Sell 3-state, Hospital/Armory 2-state, etc.) | ✓ |
| +0x220 | int | **GapGenerator state** (0=Inactive, 1=Expanding, 2=Active, 3=Contracting) | ✓ R2 |
| +0x2E4 | ptr | Docked unit pointer (Bunker, Repair Depot) | ✓ MR&P |
| +0x2FC | int | Occupant / radio slot counter | ✓ MR&P |
| +0x41A | byte | "Is player house" indicator (for EVA/sound) | ✓ MR&P |
| +0x57C | ptr | = Anims[8] (Repair Depot arm extended) | ✓ R4 |
| +0x588 | ptr | = Anims[11] (Repair Depot arm retracted) | ✓ R4 |
| +0x58C | ptr | = Anims[12] (Repair Depot secondary) | ✓ R4 |

---

## 3. BuildingTypeClass Layout (+0x000 to +0x1798)

### Core properties

| Offset | Type | INI Key | Default | Purpose |
|---|---|---|---|---|
| +0x0CCE | bool | `Naval=` | false | Naval building (TechnoTypeClass) |
| +0xCCD | bool | `Crewed=` | (varies) | Ejects crew on destruction |
| +0xCD1 | bool | TS-era (unused?) | false | — |
| +0xE88 | char[24] | `PowersUpBuilding=` | "" | Upgrade target name |
| +0xEB4 | bool | `Occupier=` (InfantryTypeClass) | — | Infantry-type flag (on INF, not BLD) |
| +0xEB8 | int | `Factory=` | 0 | **TechnoTypeClass kind enum** — values: `3` = AircraftType, `7` = BuildingType (ConYard target), `0x10` = InfantryType, `0x28` = UnitType. **Do NOT confuse with BuildingClass `What_Am_I`** values (1/2/6/0xF, see §1). Verified in `OnSpyInfiltrate` (0x28/0x10 cases), `GetSurvivorInfantryType` (7), `HouseClass::GetAccumulatedBonus` (all four). |
| +0xEC8 | int[3] | `ExitCoord=` | 0,0,0 | Lepton offset for exit |
| +0xED4 | ptr | (computed) | NULL | Foundation exit-cell table ptr |
| +0xEE0 | int | `Power=` | 0 | Power output |
| +0xEE4 | int | `Power=` (neg) | 0 | Power drain |
| +0xEE8 | int | `ExtraPower=` | 0 | Extra power bonus (bio-reactor) |
| +0xEEC | int | `ExtraPower=` (neg) | 0 | Extra power drain |
| +0xEF0 | int | `Foundation=` | — | Foundation enum |
| +0xF4C..+0x13D0 | [21×0x44] | (art.ini) | — | **PowerUp anim entries** (68 bytes each × 21 slots). Each entry has healthy (offset 0), damaged (+0x10), and three power flags (+0x40, +0x41, +0x42) |

### Upgrade / Production

| Offset | Type | INI Key | Default | Purpose |
|---|---|---|---|---|
| +0x14E0 | int | `Upgrades=` | 0 | Max upgrade slots (0-3) |
| +0x154B | **bool** | **`HasSpotlight=`** | false | ✓ R2 — gates +0x600 allocation |
| +0x1571 | bool | (wall-related flag?) | — | ✓ (seen in Unlimbo) |
| +0x1573 | bool | `Robot=?` | — | Power-drain flag (gates charge-mode power check) |
| +0x157B | bool | `CanBeOccupied=` | false | Infantry garrison |
| +0x157C | bool | `CanOccupyFire=` | false | Garrisoned infantry can fire |
| +0x1577 | bool | `CanC4=` | — | Infantry can place C4 (corrected from v1) |
| +0x1580 | int | `MaxNumberOccupants=` | 0 | Garrison capacity |
| +0x1584 | bool | `ShowOccupantPips=` | false | — |
| +0x1588..+0x1617 | | OccupantWeaponFireCoords | — | Fire port positions |
| +0x1618 | short[2] | `QueueingCell=` | 0,0 | Harvester queue cell (art.ini) |
| +0x16FC | int | `PowersUpToLevel=` | -1 | Target upgrade level |

### Boolean flags block (+0x16A4 to +0x16CD)

All from ReadINI at `0x0045FE50`:

| Offset | INI Key | Purpose |
|---|---|---|
| +0x16A4 | `Radar=` | Provides radar |
| +0x16A5 | `SpySat=` | Full map vision when powered |
| +0x16A6 | `ChargeAnim=` | Has charge animation |
| +0x16A7 | (internal) | Init to 0 |
| +0x16A8 | `SiloDamage=` | Ore destroyed on damage |
| +0x16A9 | `UnitRepair=` | **Repair Depot** |
| +0x16AA | `UnitReload=` | **Helipad** (ammo reload pad) |
| +0x16AB | `Bunker=` | **Battle Bunker** |
| +0x16AC | `Cloning=` | Cloning Vats |
| +0x16AD | `Grinding=` | Grinder |
| +0x16AE | `UnitAbsorb=` | Absorbs units |
| +0x16AF | `InfantryAbsorb=` | Absorbs infantry (Bio Reactor) |
| +0x16B0 | `SecretLab=` | Grants random tech |
| +0x16B1 | `DoubleThick=` | Double-thick wall |
| +0x16B3 | `DockUnload=` | Dock + unload (refinery) |
| +0x16B4 | `Recoilless=` | No barrel recoil |
| +0x16B6 | `BridgeRepairHut=` | Bridge repair (TS-only) |
| +0x16B7 | `Gate=` | Has opening gate |
| +0x16B8 | **(ChargeMode marker)** | **IsChargeMode** — Tesla Coil etc. |
| +0x16B9 | `ConstructionYard=` | ConYard |
| +0x16BA | `NukeSilo=` | Nuke silo |
| +0x16BB | `Refinery=` | Refinery |
| +0x16BC | `Weeder=` | Weeder |
| +0x16BD | `WeaponsFactory=` | Vehicle production |
| +0x16BE | `LaserFencePost=` | Fence post |
| +0x16BF | `LaserFence=` | Fence segment |
| +0x16C0 | `FirestormWall=` | TS-only, dormant |
| +0x16C1 | `Hospital=` | Heals infantry |
| +0x16C2 | `Armory=` | Promotes infantry |
| +0x16C3 | `EMPulseCannon=` | TS legacy |
| +0x16C4 | `TickTank=` | TS legacy |
| +0x16C5 | (unnamed) | Gates FIRE_FACING rotation branch (alongside HasTurret). Possibly `AllowTurretRotation`. ✓ R4 |
| +0x16C7 | `CloakGenerator=` | TS-legacy — no retail YR building sets this |
| +0x16C8 | `SensorArray=` | YR-active (Psychic Sensor, Spy Satellite) |
| +0x16C9 | `ICBMLauncher=` | ICBM launcher |
| +0x16CA | `Artillary=` | TS legacy |
| +0x16CB | `Helipad=` | Helicopter pad |
| +0x16CC | `OrePurifier=` | Ore purifier bonus |
| +0x16CD | `FactoryPlant=` | Cost reduction |

### Cost bonus floats (+0x16D0..+0x16E0)

| Offset | Type | INI Key | Purpose |
|---|---|---|---|
| +0x16D0 | float | `InfantryCostBonus=` | Infantry cost mult |
| +0x16D4 | float | `UnitsCostBonus=` | Vehicle cost mult |
| +0x16D8 | float | `AircraftCostBonus=` | Aircraft cost mult |
| +0x16DC | float | `BuildingsCostBonus=` | Building cost mult |
| +0x16E0 | float | `DefensesCostBonus=` | Defense cost mult |

### Barracks + misc (+0x16E4..+0x1707)

| Offset | Type | INI Key | Purpose |
|---|---|---|---|
| +0x16E4 | bool | `GDIBarracks=` | Allied barracks exit pattern (+1, +2) |
| +0x16E5 | bool | `NODBarracks=` | Soviet barracks exit pattern (+2, +2) |
| +0x16E6 | bool | `YuriBarracks=` | Yuri barracks exit pattern (+2, +1) |
| +0x16E8 | float | `ChargedAnimTime=` | Charge animation duration |
| +0x16EC | int | `DelayedFireDelay=` | Delayed fire delay ticks |
| +0x16F0 | int | `SuperWeapon=` | SW index (-1 = none) |
| +0x16F4 | int | `SuperWeapon2=` | Second SW |
| +0x16F8 | int | `GateStages=` | Gate animation frames |
| +0x1700 | bool | `DamagedDoor=` | — |
| +0x1701 | bool | `InvisibleInGame=` | — |
| +0x1702 | bool | `TerrainPalette=` | — |
| +0x1703 | bool | `PlaceAnywhere=` | No placement restrictions |
| +0x1704 | bool | `ExtraDamageStage=` | — |
| +0x1706 | bool | `IsBaseDefense=` | Base defense (AI queue special-case) |
| +0x1707 | byte | `CloakRadiusInCells=` | Default 0x14 (20); used by CloakGen + SensorArray remove |
| +0x1710 | int | `BarrelStartPitch=` | Barrel starting pitch |
| +0x1763 | bool | `IsThreatRatingNode=` | — |
| +0x1764 | bool | `PrimaryFireDualOffset=` | — |
| +0x1765 | bool | `ProtectWithWall=` | AI walls |
| +0x1766 | bool | `CanHideThings=` | Can hide units underneath |
| +0x1767 | bool | `CrateBeneath=` | Spawn crate on destroy |
| +0x1768 | bool | `LeaveRubble=` | Leaves rubble |
| +0x1769 | bool | `CrateBeneathIsMoney=` | Money crate |
| +0x1780 | int | `NumberOfDocks=` | Dock pads |
| +0x1788 | ptr | DockingOffset data | 12-byte lepton entries |

---

## 4. Vtable Summary (full 300 slots in `BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md`)

BuildingClass overrides ~95 of 300 slots. Key slots referenced across this
document:

| Slot | Offset | Purpose |
|---|---|---|
| 5 | 0x014 | `AbstractClass::Save` (serialization) |
| 6 | 0x018 | `AbstractClass::Load` |
| 8 | 0x020 | `AbstractClass::WhatAmI` (type-class ID, NOT kind) |
| 10 | 0x028 | `GetType` |
| 11 | 0x02C | **`What_Am_I` (kind: 1/2/6/0xF)** |
| 18 | 0x048 | `GetCoords` |
| 23 | 0x05C | `Update` (AI per-tick, `0x0043FB20`) |
| 37 | 0x094 | `BuildingClass__IsDeployable` `0x00452630` — checks whether the building can undeploy/redeploy (reads Type+0x157A and owner conditions); **NOT `CanAcceptUpgrade`** — that's `0x00452670`, called directly, not via vtable. (corrected 2026-05-28: was "upgrade/sell-related query"; binary shows `IsDeployable` via `get_function_by_address 0x00452630` + `decompile_function 0x00452630` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 41 | 0x0A4 | `GetTargetCoords` |
| 42 | 0x0A8 | `GetDockCoord` |
| 53 | 0x0D4 | `BuildingClass__Limbo` `0x00445880` — removes building from map, decrements all counters (OrePurifier/Helipad/etc.), releases +0x600 BuildingLight, calls `FUN_0050A490`. (corrected 2026-05-28: was "OnDestroyed"; Ghidra confirms `BuildingClass__Limbo` via `get_function_by_address 0x00445880` + `decompile_function 0x00445880` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 54 | 0x0D8 | `Unlimbo` `0x00440580` (place on map) |
| 55 | 0x0DC | `BuildingClass__Destroy` `0x0044EBF0` — full destruction/dealloc sequence (confirmed via `get_function_by_address 0x0044EBF0`) |
| 64 | 0x100 | `ExitObject` (6724 bytes) |
| 65 | 0x104 | Draw dispatcher |
| 69 | 0x114 | `DrawBody` |
| 91 | 0x16C | `ReceiveDamage` |
| 101 | 0x194 | `Receive_Radio` |
| 104 | 0x1A0 | `TogglePowerOrGate` |
| 122 | 0x1E8 | `Queue_Mission` |
| 123 | 0x1EC | `Commence` |
| 132 | 0x210 | **`Mission_Attack`** |
| 147 | 0x24C | **`Mission_RepairAndProduce`** |
| 148 | 0x250 | `Mission_Missile` (nuke) |
| 168 | 0x2A0 | `CanCloak` |
| 169 | 0x2A4 | `ShouldUncloak` |
| 195 | 0x30C | `GetSurvivorInfantryType` |
| 212 | 0x350 | `CanSellOrUndeploy` |
| 240 | 0x3C0 | **`GetFireError`** (returns 0-10 enum) |
| 242 | 0x3C8 | `ClearTarget / Set_ArchiveTarget(0)` |
| 243 | 0x3CC | **`Fire_At(target, weaponIdx)`** |
| 245 | 0x3D4 | `ChangeOwner` |
| 254 | 0x3F8 | `GetWeapon` (upgrade-aware weapon lookup, `0x004526F0`) |
| 255 | 0x3FC | `HasTurret` (`0x004527D0`) |
| 258 | 0x408 | `GetOccupantCount` |
| 260 | 0x410 | `UpdateGapGenerator_Tick` |
| 279 | 0x45C | `TechnoClass::StartUncloaking` (inherited) |
| 280 | 0x460 | `TechnoClass::StartCloaking` (inherited) |
| 293 | 0x494 | `RegisterOnRadar` |

---

## 5. Lifecycle

### Construction

1. **Constructor** `0x0043B740`: allocates 0x720, inits all fields. Sets +0x6E3 = 0 (captured flag), +0x5B0..+0x5C4 = 0 (AnimStates), +0x664 = 0, +0x660 = 1 (HasPower default).
2. **ReadFromINI** `0x0044F820`: parses INI data
3. **Unlimbo** `0x00440580` (~4300 bytes): places on map. Key steps:
   - Wall auto-extension
   - Upgrade attachment
   - `BuildingLightClass*` allocated at +0x600 if `Type+0x154B HasSpotlight=yes` (size 0xE8, ctor `0x00435820`)
   - `LightSourceClass*` allocated at +0x614 if Type+0xE30..+0xE40 ambient light set (size 0x4C, ctor `0x00554760`)
   - HouseClass registration (radar, sensor, gap, factory, dock, spysat lists)
   - Cell occupancy updates, bridge-adjacent passability
   - `CloakGenerator` count increment (Owner+0x56F8) if Type+0x16C7

### Per-Tick Update

`BuildingClass::Update` at `0x0043FB20`. 27-step pipeline detailed in
`BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`. Mission dispatch uses
`MissionClass::Mission_Dispatch` at `0x005B3060`.

### Destruction

1. `ReceiveDamage` (`0x00442230`) → `TechnoClass::ReceiveDamage` → `ObjectClass::ReceiveDamage`
2. `BuildingClass__Limbo` (`0x00445880`) — vtable slot 53. (corrected 2026-05-28: was "OnDestroyed"; Ghidra labels this `BuildingClass__Limbo` — ROOT_CAUSE: RTTI_LABEL_DRIFT — verified via `decompile_function 0x00445880`):
   - Release 8 secondary anim slots at +0x5C8
   - Decrement OrePurifier count (Owner+0x538C), Helipad count (Owner+0x2D4)
   - Release +0x600 BuildingLight via vtable+0xF8
   - Wall connection recalc
   - Sensor array / cloak generator deregister
   - `HouseClass::OnBuildingDestroyed` via `FUN_0050A490` — updates AI build queue (marks IsBaseDefense slot empty, or invalidates cell)
   - Screen invalidation, radar update, house recount
3. `SpawnSurvivors` (`0x00442D90`) + `EjectOccupants` (`0x004575B0`)

---

## 6. Damage & Immunity System (11-Check)

Full detail in `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md`.

**Checks (ordered):** self-damage guard → wall immunity → insignificant+bridge
repair hut → type-immune (same type + same owner) → IronCurtain → WarpingOut
→ Radiation+ImmuneToRadiation → PsychicDamage+ImmuneToPsionicWeapons →
Poison+ImmuneToPoison → AffectsAllies=no+allied → insignificant.

**Damage state thresholds:**
- ConditionYellow: health crosses below `Rules+0x1700` (typically 50%)
- ConditionRed: health crosses below `Rules+0x1708` (typically 25%)

`SetDamagedState` (`0x00451EE0`) swaps anim arrays; `CreateDamageFireAnims`
(`0x0043C0D0`) spawns fire overlays.

---

## 7. Power System

### Output Formula (health-scaled)

```
base = Type+0xEE0 (Power=)
    + (Type+0xEE8 if HasExtraPowerBonus)                  // bio-reactor
    + (Type+0xEE8 × docked_unit_count if UnitAbsorb/InfantryAbsorb)
    + sum(upgrade[i].Type+0xEE0 for i in 0..UpgradeLevel)

total_output = base × GetHealthRatio()   // ONLY if base > 0 AND HasPower
```

### Drain Formula (NOT health-scaled, but gated by HasPower)

Returns 0 if `HasPower == false` (e.g. low-power / spy blackout / offline) OR if
the "offline" virtual check (`vtable+0x1D4`) returns non-zero. Otherwise:

```
total_drain = Type+0xEE4
    + (Type+0xEEC if HasExtraPowerDrain)
    + sum(upgrade[i].Type+0xEE4 for i in 0..UpgradeLevel)
```

Implication: when a building is knocked offline it stops *consuming* power as
well as producing it. Code that treats drain as a constant nameplate value will
miscount house power balance during blackouts.

### Key Functions

- `GetPowerOutput` `0x0044E7B0`
- `GetPowerDrain` `0x0044E880`
- `PowerRatio` `0x004FCE30` (output/drain, clamped 0-1)
- `GoOnline` / `GoOffline` (TogglePower)
- Spy blackout: zeros PowerOutput for SpyPowerBlackout frames

---

## 8. Spy Infiltration (`0x004571E0`)

All 7 effects active in YR. Priority order (first match wins). Full detail
in `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md`.

1. **Same owner** → early return
2. `Radar=yes` → shroud reset via `MapClass::RestoreShroud` (skipped if victim low power)
3. `Power > 0` → power blackout for SpyPowerBlackout frames (default 1000)
4. **BuildTech list member** → tech steal (sets stolen flag by AIBasePlanningSide)
5. `SuperWeapon != -1` → SW charge reset via `OnSpyWeaponInfiltrate`
6. `Storage > 0` → money steal = `victim_balance × SpyMoneyStealPercent` (default 50%)
7. `Factory=UnitType` (Type+0xEB8==0x28) → sets SpiedWarFactory flag
8. `Factory=InfantryType` (Type+0xEB8==0x10) → sets SpiedBarracks flag

**Not handled**: Factory=BuildingType and Factory=AircraftType produce no
effect beyond trait qualifications.

---

## 9. Upgrade System (3-Slot)

Full detail in `BUILDING_UPGRADE_SYSTEM_GHIDRA_REPORT.md`.

### Storage
- +0x5EC / +0x5F0 / +0x5F4: Upgrades[0..2] (BuildingTypeClass*)
- +0x702: UpgradeLevel (byte, 0-3)

### Lifecycle
1. `CanAcceptUpgrade` `0x00452670`: owner match + PowersUpBuilding name + level cap
2. `Unlimbo` integrates upgrade building into parent
3. `AddUpgrade` `0x00451400`: full heal + level++, create PowerUp anim
4. `RemoveLastUpgrade` `0x00451690`: clear anims, decrement, null slot, recalc production

### Effects
- Power: additive via loop in GetPowerOutput/GetPowerDrain
- **Weapons: upgrade weapons CHECKED FIRST** (GetWeapon `0x004526F0`) — overrides host weapon
- Health: full heal on upgrade install
- Tech tree: RemoveLastUpgrade triggers `HouseClass::AI_ManageProduction`

---

## 10. Docking System

Full detail in `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md`.

### GetDockCoord (`0x00447B20`) — Base dock position

Per-type dispatch: Weeder → fixed offset (E+2, S+1); Refinery → center+128
leptons east; Bunker → angle-based 8-direction; Helipad/UnitRepair → uses
DockingOffset array via `RadioClass::FindDockSlot` (`0x0065AD90`); default →
center.

### GetDockCellForObject (`0x0044EFB0`) — Exit cell selection

**Full dispatch order (verified R3):**

| # | Condition | Cell(s) Tried |
|---|---|---|
| 1 | Type+0x16E4 GDIBarracks | origin + (+1, +2) |
| 2 | Type+0x16E5 NODBarracks | origin + (+2, +2) |
| 3 | Type+0x16E6 YuriBarracks | origin + (+2, +1) |
| 4 | Type+0xCCE Naval AND Type+0x16BD WF | **3 water cells**: (dock+1, +1), (dock+1, 0), (dock, +1) |
| 5 | Caller-provided fallback cell | that cell |
| 6 | Type+0xED4 null OR Type+0x16C1 Hospital | foundation perimeter scan |
| 7 | Type+0xED4 ExitList | {dx, dy} DWORD pairs until 0x7FFF, 0x7FFF sentinel |

Each candidate validated via `vtable[0x1AC]` (cell enterable by object);
first free wins; all-fail → returns `DAT_0089C818` (invalid sentinel).

### ExitObject (`0x00443C60`, 6724 bytes) — Production exit

Dispatches on **Kind** enum (What_Am_I = vtable slot 11):

- **Kind 1 (Unit)** / **Kind 0xF (Infantry)**: common tail with Hospital/Armory/WF
  precondition; calls `RadioClass::HasFreeSlot` (`0x0065ADC0`) to ensure
  radio bandwidth; dispatches by Type flag:
  - Refinery/Weeder: dock + unload (direction via g_DirectionOffsets + fixed offsets)
  - Barracks (GDI/NOD/Yuri variants): foundation-specific exit coord using Type+0xEC8/0xECC/0xED0
  - Non-WF non-infantry: general atan2-based direction math
  - WF vehicle: inline atan2 direction + foundation-edge step + Unlimbo at exit cell
- **Kind 2 (Aircraft)**: dedicated path. `HouseClass::AI_EconomyStateMachine(2)`, Owner+0x5658 cleared; uses ExitCoord or FindNearby; set facing via Random; Queue_Mission(MOVE)
- **Kind 6 (Building)**: dedicated path for building-from-building (Cloning Vats). Uses Owner build queue (§22); `BuildingTypeClass::CanBePlacedAt` returns 0/1/2 for invalid/retry/ok

**Cloning Vats hook** at `0x004449FB`: if Type.Factory==0x10 (Infantry
barracks) AND NOT itself a Cloning Vat (`Type+0x16AC==0`), iterate
`HouseClass+0xFC` (Cloning Vats list) and call each vat's `vtable[0x100]`
to spawn duplicate infantry.

### ClearBibArea (`0x00449540`) — WF bib scatter

Gated by `Type+0x16BD`. Scatters any unit blocking the bib via
`CellClass::Scatter_Objects` up to 8 iterations with `Pathfinding_update_continued`.

### Returns

- 0 = exit failed
- 1 = retry next tick
- 2 = exit successful

### "5-state gate machine" from v1 — not a real state machine

v1 section 10 claimed a 5-state gate machine (init → clear bib → drive out →
wait → close gate). **This is not a literal state machine.** It's the
*conceptual* combination of ExitObject + ClearBibArea +
Mission_RepairAndProduce (Repair Depot piggyback) + UnitClass locomotor +
rendered gate frames from Type+0x16F8 GateStages. No single `gate_state`
field exists.

---

## 11. Garrison System

Full detail in `GARRISON_SYSTEM_GHIDRA_REPORT.md`.

### Fields
- Type+0x157B `CanBeOccupied=`
- Type+0x157C `CanOccupyFire=`
- Type+0x1580 `MaxNumberOccupants=`
- BuildingClass+0x684 DynamicVector (Items +0x688, Count +0x694)
- **BuildingClass+0x69C: CurrentFireIdx (round-robin)** — not +0x664

### Fire Mechanics
- Weapon from occupant's InfantryTypeClass (OccupyWeapon +0xE04 / EliteOccupyWeapon +0xE20)
- Damage: `base × OccupyDamageMultiplier` (Rules+0xF40)
- ROF: `(baseROF / occupant_count) / OccupyROFMultiplier` (Rules+0xF44)
- Range: `OccupyWeaponRange` (Rules+0xF48) replaces weapon range entirely
- CurrentFireIdx increments after each shot

### Ownership Transfer
- `CheckAutoSellOrCivilian` (`0x00458200`) runs per tick
- Civilian-owned + occupied → transfer to first occupant's owner (1-tick delay)
- Reverts to civilian when last occupant leaves

### Bio-Reactor vs Garrison — SEPARATE systems

- **Bio-reactor**: embedded `CargoClass` at +0x114 (NumPassengers). Gated by
  `InfantryAbsorb=` / `UnitAbsorb=`. Entry via `CargoClass::AddPassenger`.
- **Garrison**: dedicated DynamicVector at +0x684. Gated by `CanBeOccupied=`
  + `InfantryTypeClass.Occupier`.
- The DynamicVector at +0x66C is UNRELATED — used by PowerCheck_Upgrade for
  upgrade iteration.

---

## 12. Animation System (21-Slot)

21 fixed anim slots stored as pointers in `Anims[]` at +0x55C. Type-side
entries at Type+0xF4C..+0x13D0 (21 × 0x44-byte entries).

### Slot roles (verified from `UpdateAnimation` @ `0x004509D0`)

Slots are NOT a clean one-role-per-range partition — a single slot serves
different purposes for different Type flags. Verified bindings:

| Slot | Instance field | Type-entry offsets | Role (binary-verified) |
|:---:|---|---|---|
| 0-2 | +0x55C..+0x564 | Type+0xF4C..+0xFD4 | Upgrade (PowerUp1/2/3) anims — see §9 |
| 3 | +0x568 | Type+0x1018/+0x1028/+0x1038 | Bio-Reactor **empty** (InfantryAbsorb, no passengers); also Refinery ore tier 0 |
| 4 | +0x56C | Type+0x105C/+0x106C/+0x107C | Bio-Reactor **with-cargo** (InfantryAbsorb, passengers>0); also Refinery ore tier 1 |
| 5 | +0x570 | Type+0x10A0/+0x10B0 | Refinery ore tier 2 |
| 6 | +0x574 | Type+0x10E4/+0x10F4 | Refinery ore tier 3 (full) |
| 8 | +0x57C | Type+0x116C/+0x117C | Repair Depot arm extended |
| 9 | +0x580 | — | Turret sprite facing (shadow-direction lookup) |
| 10 | +0x584 | Type+0x11F4/+0x1204 | Weeder / SiloDamage storage anim (4-tier via `StorageClass::GetTotalAmount`) |
| 11 | +0x588 | — | Repair Depot arm retracted |
| 12 | +0x58C | Type+0x127C/+0x128C | Repair Depot secondary |
| 14 | +0x594 | Type+0x1348/+0x1358 | SuperWeapon pre-charge (before `ChargedAnimTime`) |
| 16 | +0x59C | Type+0x13D0/+0x13E0 | SuperWeapon charged/ready |

Slots 7, 13, 15, 17-20 are not dispatched from `UpdateAnimation` directly —
they're driven from `GrandOpening`, `SetAnimSlotImage`, or other subsystems
(production/construction overlays, low-power idle). Companion doc
`BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md` has more detail per
building type.

**Instance-side playback state** (shared across all slots, not per-slot):
- +0x0F8 int — current anim frame
- +0x0FC byte — "stepped this tick" flag (set when CDTimer fires)
- +0x100 int — anim start frame (g_CurrentFrameCounter snapshot)
- +0x108 int — active frame count
- +0x10C int — cached frame count (0 = anim inactive)
- +0x110 int — rate (ticks/frame)
- +0x534 int — DamagedState flag; indexes damaged (+0x10) vs healthy (+0x00) within a PowerUp entry

### Per-entry flags (each 0x44-byte entry)

Each PowerUp entry has **4 power flags** at offsets within the entry:
- Flag A (Powered): anim stays alive on power-off, detach/reattach
- Flag B (PoweredLight): destroyed on power-off, recreated on power-on
- Flag C (PoweredEffect): tracked via `AnimStates[21]` at BuildingClass+0x5B0 (byte per slot, 0/1)
- Flag D (PoweredSpecial): triggers special anim on spy blackout

### AnimStates confirmed (R1)

`BuildingClass+0x5B0..+0x5C4` — 21-byte array, parallel to Anims[21]. Set to 1 in `OnPowerOn`, cleared to 0 in `OnPowerOff`, gated by Type+0xF8E flag per slot. **v1's "debunked" claim was wrong.**

### Damage-state transitions

`SetDamagedState` (`0x00451EE0`) at ConditionYellow swaps anim arrays
between undamaged (Type+0xF4C offset 0) and damaged (Type+0xF4C offset +0x10).

---

## 13. Wall System

Full detail in `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` wall section.

- LaserFencePost (+0x16BE): Connection point, 16-frame bitmask
- LaserFence (+0x16BF): Segment between posts
- Powered fences damage units with C4Warhead
- Functions: ConnectWalls, RecalculateWallConnections, ExtendWallInDirection,
  OnWallDestroyed, FindNearestFencePost

---

## 14. Gap Generator (4-State)

State at **`BuildingClass+0x220`** (DWORD, NOT +0xBC; corrected R2).

| State | Name | Behavior |
|---|---|---|
| 0 | Inactive | No shroud effect. If CanCloak: `TechnoClass::StartCloaking` (vtable+0x460) |
| 1 | Expanding | Grows 0 → 15 (+0x6ED increments). +0x80 redraw flag set on certain frames |
| 2 | Active | Full shroud; GapOverlayCount + GapShroudLevel maintained per-cell. If ShouldUncloak: `TechnoClass::StartUncloaking` (vtable+0x45C) |
| 3 | Contracting | +0x6ED decrements, shroud peeled. At 0: state=0, new ParticleSystem allocated if Type+0x764 set |

Translucency (slot+0x178 byte) synced to all 21 anim slots. Neighbor cascading
supported.

### Handler

`UpdateGapGenerator_Tick` at `0x00454DB0` (vtable slot 260).

### CloakGen (tick-down) separate 3-byte system

Parallel to gap-gen state but different fields:
- +0x6EB: direction (0 / 1 / 0xFF)
- +0x6EC: current radius
- Cleanup when direction<1 AND radius==0: set +0x6EB=0 and return. No dedicated UnInit.

---

## 15. TS-Legacy Fields (Dormant in YR)

| Offset | Field | Status in YR |
|---|---|---|
| +0x16B6 | BridgeRepairHut | TS-only, default false |
| +0x16C0 | FirestormWall | TS-only |
| +0x16C3 | EMPulseCannon | TS-only (Mission_Missile has dormant branch) |
| +0x16C4 | TickTank | TS-only |
| +0x16C7 | **CloakGenerator** | TS-only — **no retail YR building sets this flag** |
| +0x16CA | Artillary | TS-only |

Self-cloaking buildings (`Cloakable=yes`): code path exists, no retail YR
usage. `FogOfWar` (MultiplayerDialogSettings) defaults false in YR.

---

## 16. Per-Tick Update Pipeline (`0x0043FB20`)

~2650 bytes. 27-step pipeline. Full detail in
`BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`.

1. IsOperational check (vtable+0x350)
2. UpdateGapAndSpecialEffects on state change (`0x004549B0`)
3. Damage fire anims (ConditionYellow/Red crossings)
4. **ProduceCash (Oil Derrick)** — CDTimer at +0x6D0 counts `ProduceCashDelay` (Type+0x1560), grants `ProduceCashAmount` (Type+0x155C)
5. Gap generator tick (UpdateGapGenerator_Tick)
6. UpdateAnimation (`0x004509D0`) — frame timers, turret, garrison fire anims, radar, storage, SW staging
7. TechnoClass::AI_Update (`0x006F9E50`)
8. UpdateRepairAndPower (`0x00450630`)
9. Auto-production (Cloning Vats) — FUN_004500F0
10. ProcessDelayedFire (`0x004503F0`)
11. Destruction sequence (spawn survivors + Limbo at 0 HP)

---

## 17. Mission Handlers — Full Map

12 BuildingClass overrides. Dispatched via `MissionClass::Mission_Dispatch` at `0x005B3060`.

| Enum | Name | Vtable Slot | Addr | Size |
|---|---|---|---|---|
| 1 | Attack | 132 | `0x0044ACF0` | ~1174 |
| 5/6 | Retreat/Sleep | 135 | `0x004496B0` | ~902 |
| 8 | Guard | 133 | `0x0044B760` | ~26 |
| 10 | Return | 137 | `0x0044B770` | ~16 |
| 11 | Stop | 136 | `0x00449A40` | ~8 |
| 16 | Eaten/Rescue | 143 | `0x0044D880` | ? |
| 17 | Harmless | 133 | reuses Guard | — |
| 18 | Construction | 145 | `0x00449A50` | 355 |
| 19 | Selling | 146 | `0x00449C30` | 3989 |
| 20 | RepairAndProduce | 147 | `0x0044B780` | **4604** |
| 22 | Missile | 148 | `0x0044C980` | 3104 |
| 24 | Unload | 149 | `0x0044E440` | ? |

### Mission_Attack (0x0044ACF0) — Combat dispatcher

**Full detail in `BUILDINGCLASS_MISSION_ATTACK_AND_RESIDUALS.md`**

**Path A — Direct fire** (`Type+0x16B8` IsChargeMode = 0):
1. No target → clear target, Queue_Mission(**mission 5 = Sleep/Retreat**, **NOT** Guard — see §17 handler table: 5→Retreat/Sleep, 8→Guard), Commence
2. Has target → compute fire_error via `vtable[0x3C0]` (GetFireError)
3. fire_error == 2 (FIRE_FACING): if HasTurret AND `Type+0x16C5`: rotate, re-check
4. **11-entry jump table at `0x0044B728`**:

| fire_error | Handler | Shared | Enum | Behavior |
|:---:|:---:|:---:|:---|:---|
| 0 | `0x0044B2BC` | — | FIRE_OK | Fire via `vtable[0x3CC](target, 0)` — checks UpgradeLevel+Upgrades[0] for upgrade weapon override first |
| 1 | `0x0044B0DE` | 5,6,8 | FIRE_AMMO | Bail: clear target, reset idx |
| 2 | `0x0044B187` | — | FIRE_FACING | Rotate turret |
| 3 | `0x0044B1DE` | — | FIRE_REARM | Reload anim |
| 4 | `0x0044B14E` | 7 | FIRE_ROTATING | Wait |
| 5,6,8 | `0x0044B0DE` | 1 | FIRE_ILLEGAL/CANT/RANGE | Same bail |
| 7 | `0x0044B14E` | 4 | FIRE_MOVING | Wait (unreachable for buildings) |
| 9 | `0x0044B284` | — | FIRE_CLOAKED | Cloaked target handler |
| 10 | `0x0044B24F` | — | FIRE_BUSY | Busy handler |

**Path B — ChargeMode 3-state** (`Type+0x16B8` IsChargeMode = 1 — Tesla Coil, Prism Tower):

State at `+0xBC`:

- **State 0 (pre-charge)**: **Conditional power check** — only when `Type+0x1573` (Robot-style flag) is set AND `Type.Drain > 0`; in that case if `HouseClass::GetPowerRatio() < 1.0` the state advance is skipped (wait). For buildings without that flag (most charge-mode defenses: Tesla Coil, Prism Tower in stock YR), State 0 advances regardless of house power. Then validate target kind. If facing delta `< 0x2001` (~45°) → state=1; else `RateTimer::Set(target_facing)` to rotate
- **State 1 (charging/fire)**: Re-validate target visibility via `vtable[0x1D0]`. fire_error ∈ {5,6,8}: abort to state=0. fire_error == 0: fire both weapons `Fire_At(target, 0)` and `Fire_At(target, 1)`, state=0
- **State 2+ (cooldown)**: return `MissionClass::GetMissionTimerEntry() + Random(0, 2)` — jittered cooldown prevents lockstep fire

Facing tolerance `0x2001`: in 0-0xFFFF facing space = ~one compass direction
(8 directions × 0x2000 = 0x10000). Gates state 0→1 transition.

### Mission_RepairAndProduce (0x0044B780) — 7-Mode Dispatcher

**Full detail in `BUILDINGCLASS_MISSION_REPAIR_AND_PRODUCE.md`**

Dispatches on Type flag:

1. **Bunker** (Type+0x16AB) → FUN_00458E50 — **6-state docking machine at +0x718** (0: arrival, 1: dock slot search, 2: CDTimer + anim, 3: arrival check, 4: anim activation, 5: link + complete, 6: terminal)
2. **ConstructionYard** (Type+0x16B9) → 2-state (+0xBC): GrandOpening → idle monitor
3. **Hospital** (Type+0x16C1) → 2-state heal timer. Formula: **`Rules+0x16F0 IRepairRate × 900.0`** threshold. Response 0x21 (REPAIR_COMPLETE) → radar event + VoxEVA + VocPlay, eject, Queue_Mission(GUARD)
4. **Armory** (Type+0x16C2) → 2-state identical timer, uses `VeterancyStruct::SetVeteran/SetElite` instead of heal radio
5. **Repair Depot** (Type+0x16A9) → 3-state (+0xBC) with LocomotionClass piggyback:
   - State 0: `LocomotionClass::QueryInterface_IPiggyback` attach, radio 0x13, distance check
   - State 1: Drive-in phase, health check; if `Rules+0x16F8` (hardcoded 1.0) ≤ health: release; else: start repair anim
   - State 2: HP tick with `Rules+0x16E8 URepairRate × 1.0` threshold; radio 0x13 → 0x1C response determines retry/complete
6. **Helipad** (Type+0x16AA) → per-aircraft radio cycle (0x1D → 0x13 → 0x1F → 0x1C)
7. Default → return 0xF (15-frame re-check)

Accepted locomotors at Repair Depot: `CLSID_WalkLocomotion` and
`DriveLocomotion` (CLSID at `DAT_007E9AB0` = `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}`).

### Mission_Selling (0x00449C30) — 3-state with MCV undeploy

State at +0xBC:
- **State 0**: Init. If upgrades exist, refund LAST upgrade at full cost (not SellBack%), Queue_Mission(GUARD)
- **State 1**: Eject + animate. Plays GrandOpening(0) reverse anim
- **State 2**: Finish / undeploy. If `Type+0x408` UndeploysInto: attempt MCV undeploy

**Refund formula (corrected v1):** `Cost × Rules.SellBack (+0x145C, default 50%) + stored ore`. **NOT health-scaled** — sick building refunds same as full HP.

**MCV undeploy:**
- `operator_new(0x8E8)` + `UnitClass::Constructor(Type+0x408, Owner)`
- On alloc-fail (`0x0044A19E`): `vtable[0x2BC] (GetRefundValue)` + Add_Credits
- On placement-fail (`0x0044A16B`): cached `vtable[0x2BC]` result from pre-attempt, Add_Credits
- On success: MCV health = `floor(HealthRatio × UnitType.Strength)`, min 1. Inherits radar jam, gap-gen state, cloak shroud mask, **SoundEvent at +0x4DC..+0x4F4** (20-byte copy via MOVSD.REP + 2 DWORDs). All radio-linked units re-bound.

**Survivor count:** `GetSurvivorCount` (vtable+0x2D0 = `0x00451330`).
Formula: `clamp(Cost / SurvivorDivisor[side], 1, 5)`. **Bio-reactor doubles divisor.**
Zero on bridge or Crewed=no. If +0x6E3 (OwnershipChanged) != 0: divisor doubled again (halves).

**Survivor infantry type:** `GetSurvivorInfantryType` (vtable+0x30C = `0x0044EB10`):
- If +0x6E3 (OwnershipChanged) == 0 AND `Type+0xEB8 == 7` (Factory=BuildingType / **ConYard**): 25% Engineer (Rules+0xF70). **NOT "Soviet-side" — corrected R1**
- Otherwise falls through to `TechnoClass::GetSurvivorInfantryType` at `0x00707D20`: side (Owner->HouseType+0x1E8) → 0=Allied+0xF78 / 1=Soviet+0xF7C / 2=Third+0xF80, default Technician+0xF6C. 15% Technician override if Is_Weapon_Equipped.

### Mission_Missile (0x0044C980) — Nuke silo 5-state

Gated ONLY by `Type+0x16BA NukeSilo` flag. ICBMLauncher (+0x16C9) is a separate subsystem. State counter at +0xBC:
- State 0: `GrandOpening(2)`, create PSIWARN anim at target, → state 1
- State 1: Wait for +0x6DD != 0 (doors open), `GrandOpening(4)` → state 2
- State 2: Allocate BulletClass (NukeCarrier), release PSIWARN, fire bullet, create NUKETO anim → state 3 (returns 1)
- State 3: `GrandOpening(5)` (close doors) → state 4 (returns 6)
- State 4: `GrandOpening(5)` + Queue_Mission(GUARD) → returns 60

---

## 18. Receive_Radio Protocol (`0x0043C2D0`, slot 101)

9 messages handled; rest delegate to TechnoClass (`0x006F4AB0`) → RadioClass (`0x0065A820`).

| Msg | Name | Direction | BuildingClass Behavior |
|:---:|---|---|---|
| 0x03 | OVER_AND_OUT | any | GrandOpening reset + delegate |
| 0x08 | REQUEST_CLEARANCE | U→B | Near-range ROGER for UnitRepair/Bunker; WeaponsFactory → QUEUED (0x17) |
| 0x0B | DOCK_APPROACH | B→U | Queue_Mission(UNLOAD=0x14) |
| 0x0C | DOCK_ARRIVED | U→B | Queue_Mission(GUARD); if ConYard, rebuild ambient anim |
| 0x0D | — | — | Silent ROGER for WeaponsFactory |
| 0x0E | CAN_DOCK | U→B | Establish link, compute queue cell (+3,+1) for Refinery/Weeder, MOVE_TO_CELL + ENTER_DOCK + TIMING_SYNC |
| 0x0F | CAN_ENTER | U→B | Passenger/garrison entry — gated by UnitRepair/Bunker/UnitAbsorb/InfantryAbsorb/Grinding/Hospital/Armory/Helipad |
| 0x10 | RESERVE_DOCK | U→B | ROGER for harvester + same owner + idle |
| 0x15 | DOCK_NOW | U→B | Sets +0x6DD=1 + Queue_Mission(UNLOAD); Refinery sends sender ENTER |

**Repair (0x1C)** is TechnoClass-level. Response codes:
- `0x20` = INSUFFICIENT_FUNDS
- `0x21` = REPAIR_COMPLETE
- `0x01/10` = ROGER

Additional helipad radio (MR&P): `0x13` REQUEST_APPROACH, `0x1D` REFUEL_QUERY, `0x1F` RESERVE_DOCK.

---

## 19. CloakGenerator / SensorArray

Full detail in `BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md`.

### CloakGenerator (+0x16C7) — TS-Legacy

**No retail YR building sets this flag.** Do not prioritize for implementation.

- 3-byte state at +0x6EB/+0x6EC/+0x6ED (direction/radius/stage)
- Grows 1 cell radius per tick
- Uses `TechnoClass::UpdateCloakShroud` — increments GapOverlayCount (CellClass+0x134) + GapShroudLevel (+0x130)
- DOES NOT call Cloak() on units — just shrouds cells
- DoUncloak called on units when cells REMOVED (forces visibility recheck)
- Cleanup when radius 0: +0x6EB=0, early return (no dedicated UnInit)

### SensorArray (+0x16C8) — YR-Active

Used by Psychic Sensor and Spy Satellite.

- Uses CellClass+0x7C short-array-per-house counter
- `AddSensorArrayAt` (`0x00455820`, vtable slot 317 / offset 0x4F4): increment + DoUncloak on Units/Infantry/Aircraft in cell
- `RemoveSensorArrayAt` (`0x004556D0`, vtable slot 318 / offset 0x4F8): decrement across a possibly-different radius (see below)
- Radius fields (asymmetric — real bug in gamemd.exe): **AddSensorArrayAt reads Type+0x5F0 (`SensorsSight`, int)**; **RemoveSensorArrayAt reads Type+0x1707 (`CloakRadiusInCells`, byte, default 0x14 = 20)**. For retail YR Psychic Sensor (`SensorsSight=15`, no `CloakRadiusInCells` override), add zone = 15 cells but remove zone = 20 cells, so remove decrements ref-counts on cells that were never incremented. Rust impl should use the add radius for both paths to avoid the drift.

### Overlapping Fields — Reference Counted

- SensorCount[house] + DisguiseDetectCount[house] accumulate; visibility check `> 0`
- Gap: Overlay + Level double-counter — GapShroudLevel decrements only when GapOverlayCount hits 0. Overlapping gap generators coexist.

---

## 20. Special Buildings — Verified Mechanics

Full detail in `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md`.

### Cloning Vats (+0x16AC)
`ExitObject_Main` offset `0x004449FB`: when Barracks (Type.Factory==0x10) produces infantry AND the barracks is NOT a Cloning Vat (Type+0x16AC==0), iterate `HouseClass+0xFC` (Cloning list), call `vtable[0x100]` on each vat.

### Grinding (+0x16AD)
`Mission_Enter` (`0x005196A0`): `Add_Credits(unit->vtable[0x2BC]())` — that's `GetRefundValue` reading TechnoTypeClass.Soylent (+0x614). Passengers + mind-control slaves recursively refunded.

### Hospital (+0x16C1) / Armory (+0x16C2)
Mission_RepairAndProduce state 2. Threshold: `Rules.IRepairRate (+0x16F0) × 900.0` (constant at `DAT_007E27F8`). Hospital heals + ejects; Armory promotes + ejects.

### InfantryAbsorb / Bio Reactor (+0x16AF)
`GetPowerOutput` (`0x0044E7B0`): `power += Type.ExtraPower (+0xEE8) × NumPassengers (BuildingClass+0x114)` when InfantryAbsorb AND ExtraPower>0.

### SecretLab (+0x16B0)
Pool = Rules.SecretInfantry (+0xD00) + SecretUnits (+0xD1C) + SecretBuildings (+0xD38). Fisher-Yates sample per lab. Registry at `0x00442C40`; assignment at `0x0068C050`.

### OrePurifier (+0x16CC)
`DepositOreFromStorage` (`0x00522D50`): `bonus = NumOrePurifiers × Rules.PurifierBonus (+0xF3C) × amount`. Counter at HouseClass+0x538C. AI bonus at Rules+0x1324[difficulty].

### FactoryPlant (+0x16CD)
Per-building floats at Type+0x16D0..+0x16E0. `RecalcBonuses` (`0x0050BF60`): stacking multiply into HouseClass+0x5390..+0x53A0 (initialized to 1.0f). `GetAccumulatedBonus` (`0x0050BEB0`): applied at cost lookup, dispatches on `vtable+0x2C` (TechnoTypeClass kind enum — **different from BuildingClass `What_Am_I`** values in §1). Switch cases:
- `3` → Aircraft bonus (HouseClass+0x5398)
- `7` → BuildingType; sub-dispatches on `param_2[0x382] == 5` → Defense bonus (HouseClass+0x53A0), else Building bonus (HouseClass+0x539C)
- `0x10` → Infantry bonus (HouseClass+0x5390)
- `0x28` → Unit bonus (HouseClass+0x5394)
- default → constant from `_DAT_007E2AC8` (1.0f)

Do NOT match these cases against `What_Am_I` values (1/2/6/0xF) — wrong enum.

---

## 21. AI Build Queue at Owner+0x5704 — `DynamicVector<BuildOrder>`

**Full detail in `BUILDINGCLASS_OPEN_QUESTIONS_VERIFICATION_R3.md`**

### Vector layout

| Offset | Size | Purpose |
|---|---|---|
| Owner+0x5704 | 4 | vtable ptr |
| Owner+0x5708 | 4 | Items array ptr |
| Owner+0x570C | 4 | Capacity |
| Owner+0x5710 | 1 | IsAllocated (byte + pad to +0x5714) |
| Owner+0x5714 | 4 | Count |

### BuildOrder entry (16 bytes)

| Offset | Size | Purpose |
|---|---|---|
| +0x0 | 4 | BuildingType ID (matches `BuildingTypeClass+0xDF8`) |
| +0x4 | 4 | Packed cell coord (short x | short y << 16) |
| +0x8 | 4 | Unknown (reserved?) |
| +0xC | 4 | Unknown |

### Consumers

- `HouseClass::AI_Manage_Build_Queue` (`0x004FDD10`) — adds
- `HouseClass::AI_ChooseNextProduction` (`0x00506EF0`) — reads
- `BuildingClass::ExitObject_Main` — removes on spawn (or updates cell for IsBaseDefense)
- `FUN_0050A490` (OnBuildingDestroyed hook) — invalidates on destruction

---

## 22. Sound Event Subsystem (+0x4DC region)

`SoundEvent::SetLoopHandle` at `0x004060F0` (signature verified via 11 cross-refs from VocClass/AnimClass/RadarClass/etc.).

### BuildingClass layout (inherited from TechnoClass)

| Offset | Size | Purpose |
|---|---|---|
| +0x4DC..+0x4EB | 16 | SoundEvent struct (audio_handle, audio_ref, loop_data, vtable_sig) |
| +0x4EC..+0x4EF | 4 | Part of 5-DWORD MOVSD.REP copy block |
| +0x4F0 | 4 | Sound loop handle #1 (-1 = none) |
| +0x4F4 | 4 | Sound loop handle #2 (-1 = none) |

### Inheritance during MCV undeploy

`Mission_Selling` state 2 (`0x0044A0D4`): MOVSD.REP × 5 + 2 DWORDs, then `SoundEvent::SetLoopHandle(&src[+0x4DC], 0, 0)` to detach source, set +0x4F0/+0x4F4 = -1 on source. MCV inherits looping sound seamlessly.

---

## 23. RulesClass Repair Tuning

From `RulesClass::ReadGeneral` at `0x0066D530` (verified R4):

| Offset | Type | INI Key | Default | Purpose |
|---|---|---|---|---|
| Rules+0x16CC | int | `RepairStep` | 8 | HP per vehicle-repair tick (general) |
| Rules+0x16D0 | double | `RepairPercent` | 0.15 | Cost fraction of full rebuild |
| Rules+0x16D8 | int | `IRepairStep` | ? | HP per infantry-heal tick |
| Rules+0x16E0 | double | `RepairRate` | 0.016 min | Minutes between vehicle repair ticks |
| Rules+0x16E8 | double | **`URepairRate`** | ? | Minutes between Unit-in-Repair-Depot ticks |
| Rules+0x16F0 | double | **`IRepairRate`** | 0.001 min | Minutes between infantry-heal ticks (Hospital/Armory) |
| Rules+0x16F8 | double | (hardcoded 1.0) | 1.0 | "Full health" threshold |
| Rules+0x1700 | double | (constant) | — | ConditionYellow ratio |
| Rules+0x1708 | double | (constant) | — | ConditionRed ratio |
| `DAT_007E27F8` | double | (const) | 900.0 | Hospital/Armory/Repair Depot timer multiplier |

---

## 24. Current Rust Implementation Status

### Implemented
- Power system (generation, consumption, low-power, health scaling, spy blackout)
- Repair depot docking (state machine, FIFO queue, credit costs)
- Building placement validation (terrain, overlap, build area, foundation)
- Tech tree and prerequisites (including PrerequisiteOverride)
- Building sell with crew ejection and refunds (50% health-scaled in Rust — **should be non-health-scaled per binary**)
- Repair system (toggle, credit-based restoration)
- Production queues and factory matching
- Radar and SpySat functionality
- Building animation overlays (crane, one-shot, damage fires, garrison muzzle flash)
- Garrison occupancy tracking (flags parsed)

### Not Implemented or Partial
- Garrison fire logic (flags exist but targeting/firing not wired)
- Infantry/Unit absorption (InfantryAbsorb/UnitAbsorb)
- Upgrade system (fields exist, no installation/removal)
- Spy infiltration (only power blackout implemented)
- Wall/laser fence connectivity
- Gap generator logic (flag only)
- Sensor array / cloak generator field effects
- Building-specific ExitObject dispatch (barracks/WF/naval exit patterns)
- Superweapon activation
- Building capture (engineer) — must set `+0x6E3 = 1` after capture for correct survivor math
- Cloning vats, grinding, hospital, armory
- Mission_Attack charge-mode 3-state machine
- Mission_RepairAndProduce 7-mode dispatch

### Correctness Fixes Required (per this report)
- Sell refund should be non-health-scaled (Cost × SellBack + stored_ore). See §17 Mission_Selling.
- ConYard→Engineer bonus is 25% conditional on Factory==BuildingType AND +0x6E3==0 (not captured). See §17 Mission_Selling survivor type.
- Gap-gen state at +0x220 (NOT +0xBC). See §14.
- SensorArray add/remove should use same radius field (SensorsSight vs CloakRadiusInCells may mismatch). See §19.

---

## 25. Open Questions (Remaining)

Minor items deferred from the verification rounds:

1. **AircraftClass::What_Am_I** return value — inferred as 2 from ExitObject dispatch, not directly decompiled
2. **BuildOrder +0x8 and +0xC fields** — unknown purpose (likely priority/timestamp/state)
3. ~~**Type+0x16C5 flag** — gates FIRE_FACING rotation branch alongside HasTurret; likely `AllowTurretRotation` or similar~~ **RESOLVED 2026-04-23:** `TurretAnimIsVoxel=` (INI string at `0x0081960C`), read as `ReadBool` into `BuildingTypeClass+0x16C5` in `BuildingTypeClass_ReadINI_Water`. Gates the continuous `FacingClass::UpdateFacing` + `RateTimer` rotation path in Mission_Attack FIRE_FACING; SHP-sprite turrets skip that path and snap to discrete facings.
4. **Type+0x1573 flag** — power-drain requirement flag; exact name not traced
5. **Helipad radio 0x1D semantics** — REFUEL_QUERY by context, not directly confirmed
6. **+0x664 field** — cleared in Mission_Attack no-target; possibly "last fire tick reset" or secondary state (the primary GarrisonFireIndex is +0x69C)
7. ~~**+0x700 short** — init to 0x3E8 in ctor, consumer not isolated~~ **RESOLVED 2026-05-28**: `*(undefined2 *)(param_1 + 0x1C0) = 1000` in constructor (0x1C0×4=0x700) sets it to 0x3E8 = 1000. This is the `ProduceCashTimer` initial delay or a CDTimer countdown rate; consumer still not fully isolated. Confirmed via `decompile_function 0x0043B740`.
8. **SecretLab pick storage offset** (earlier +0x6F4 claim unverified)
9. **Rules+0x16E8 default value** (URepairRate in rules.ini — check default)
10. **Type+0x184C/+0x184D** — not yet surveyed
11. **Bunker +0x718 terminal state 6 cleanup path** (what clears back to 0?)

### Suggested next targets

- **UpdateAnimation (0x004509D0)** — 21-slot anim state machine; driven by power/damage/refinery/gattling branches. Core for rendering parity.
- **DrawBody (0x0043D290)** — body/turret/upgrade layering, power-down overlay, gate rendering using Type+0x16F8 GateStages.
- **Save/Load (slots 5/6)** — serialization format for snapshot work.

---

## Sources

### Functions decompiled/analyzed across rounds

Core:
- `0x0043B740` Constructor | `0x0043BCF0` Destructor | `0x0043FB20` Update
- `0x00440580` Unlimbo | `0x00445880` OnDestroyed | `0x0044EBF0` Limbo
- `0x00442230` ReceiveDamage | `0x00442D90` SpawnSurvivors | `0x00451EE0` SetDamagedState
- `0x0043CEA0` Draw dispatcher | `0x0043D290` DrawBody

Mission handlers:
- `0x0044ACF0` Mission_Attack | `0x0044B760` Mission_Guard
- `0x00449A50` Mission_Construction | `0x00449C30` Mission_Selling
- `0x0044B780` Mission_RepairAndProduce | `0x0044C980` Mission_Missile

Specialized:
- `0x00443C60` ExitObject_Main (6724 bytes) | `0x00449540` ClearBibArea
- `0x00447B20` GetDockCoord | `0x0044EFB0` GetDockCellForObject
- `0x00454DB0` UpdateGapGenerator_Tick | `0x004549B0` UpdateGapAndSpecialEffects
- `0x00458E50` Bunker docking state machine
- `0x004545D0` OnPowerOff | `0x004547C0` OnPowerOn
- `0x00451890` CreateAnimForSlot | `0x00451750` SetAnimSlotImage | `0x00451E40` ClearAnimSlot
- `0x0044EB10` GetSurvivorInfantryType | `0x00451330` GetSurvivorCount
- `0x004571E0` OnSpyInfiltrate | `0x00448260` ChangeOwner

Power:
- `0x0044E7B0` GetPowerOutput | `0x0044E880` GetPowerDrain
- `0x00452260` GoOnline | `0x00452360` GoOffline

Upgrade:
- `0x00452670` CanAcceptUpgrade | `0x00451400` AddUpgrade | `0x00451690` RemoveLastUpgrade | `0x004526F0` GetWeapon

INI parsing:
- `0x0045FE50` BuildingTypeClass::ReadINI | `0x004653C0` BuildingTypeClass ctor
- `0x0066D530` RulesClass::ReadGeneral | `0x006691E0` RulesClass::ReadAudioVisual

Helpers:
- `0x0065ADC0` RadioClass::HasFreeSlot | `0x004060F0` SoundEvent::SetLoopHandle
- `0x0050A490` OnBuildingDestroyed queue cleanup | `0x004500F0` Cloning Vats auto-produce

Vtable reads: BuildingClass vtable at `0x007E3EBC` — slots 11, 168, 169, 279, 280, 240, 243 directly inspected.

String lookups: "HasSpotlight" @ 0x81AEA0, "URepairRate" @ 0x83BDC4,
"IRepairStep" @ 0x83BDDC, "IRepairRate" @ 0x83BDB8, "RepairRate" @ 0x83BDD0,
"RepairStep" @ 0x83BDE8, "RepairPercent" @ 0x83BDF4.

CLSID lookup: DriveLocomotion at `0x007E9AB0` = `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}`.

Jump table: Mission_Attack GetFireError table at `0x0044B728` (11 DWORDs).

### INI files checked

- `ini/rulesmd.ini` — confirmed RepairPercent=15%, RepairRate=.016, RepairStep=8, IRepairRate=.001, RepairDelay=.02/.05
- `ini/artmd.ini` — referenced for PowerUp art entries
