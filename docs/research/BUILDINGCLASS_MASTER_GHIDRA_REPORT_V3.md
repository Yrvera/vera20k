---
name: BuildingClass Master Ghidra Research Report (v3)
description: Canonical reference — integrates full-decode research plan Tasks 1-13. Supersedes v2.
type: reference
---

# BuildingClass — Master Ghidra Research Report v3

**Date:** 2026-04-24 (R4 close — full-decode plan Tasks 1-13 integrated)
**Supersedes:** `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` (2026-04-19 + 2026-04-23 audit)
**Binary:** gamemd.exe
**Confidence:** HIGH (all findings verified from direct decompilation)
**Active in YR:** Yes — BuildingClass is the core building runtime class

## v3 Change Log (what's new / corrected vs v2)

### Lifecycle — the big triangle correction (T5 + T10)

v2 had **three names wrong** on the destruction chain. v3 corrects:

| Address | v2 claim | v3 truth | Vtable slot |
|---|---|---|---|
| `0x00445880` | "OnDestroyed" | **`BuildingClass::Limbo`** — remove-from-map cleanup | 53 (`+0xD4`) |
| `0x0044EBF0` | "Limbo" | **`BuildingClass::Destroy`** — factory teardown | 55 (`+0xDC`) |
| `0x004415F0` | *(unlisted)* | **`BuildingClass::DestructionEffects`** — real HP=0 handler | **315** (`+0x4EC`, newly pinned) |

The real destruction chain (§5):

```
HP hits 0 → ReceiveDamage case 4
          → BuildingClass::DestructionEffects (0x004415F0, slot 315)
              ↓ (sets Health=0, spawns survivors, debris, anims, tiberium spill)
            BuildingClass::Limbo (0x00445880, slot 53)
              ↓ (removes from map, tears down subsystems, AI base-plan cleanup)
            BuildingClass::Destroy (0x0044EBF0, slot 55)
              ↓ (final deallocate)
```

Ghidra renames applied this pass: `BuildingClass__OnDestroyed @ 0x00445880` → `BuildingClass__Limbo`; `FUN_004415f0` → `BuildingClass__DestructionEffects`. Program saved.

### v2 §3 field-label corrections (T1)

| Offset | v2 label | v3 truth |
|---|---|---|
| `Type+0x1573` | "Robot=?" | **`Powered=`** — gates charge-mode power check, PoweredEffect anim linkage, OnPowerOn call in OnConstructionComplete |
| Building `+0x664` | "Misc reset flag" | **`FirePowerBonus`** — IronCurtain fire-power multiplier |
| `Type+0x184C/+0x184D` | "not yet surveyed on BuildingType" | **OUT OF BOUNDS** — these offsets belong to `RulesClass`, not BuildingTypeClass (ctor size is 0x1798). Rules+0x1848/+0x184C = `ElevationBonusCap` (double) |

### v2 §4 Vtable miscounts and mislabels

v2's §4 table had 10 errors; all corrected (see `BUILDINGCLASS_VTABLE_COMPLETE.md` §"Corrections flagged vs v2"):

1. Override count **101** (not 95) — v2 undercounted 6 slots in ranges 170-184, 282, 288-295
2. Total slot count **338** (not 300) — vtable extends to 0x544 before NULL terminator
3. Slot 5/6 swapped: slot 5 = **Load**, slot 6 = **Save** (IPersistStream order)
4. Slot 8 = **ScalarDeletingDestructor** (not "AbstractClass::WhatAmI")
5. Slot 9 = **Init_Managers** (not "SizeOf")
6. Slot 11 = **WhatAmI** returning 6 (not "stub")
7. Slot 12 = **SizeOf** returning 0x720 (not "stub")
8. Slot 13 = **Save_ChecksumFields** (not "PointerExpired")
9. Slot 292 = **809-byte override `0x00458A80`** (not a stub)
10. Slots 170/171/174/180/181/182/183/184/187 are all overrides (v2 labeled "various stubs/inherited")

Plus new slot identifications in the extension range 300-337:
- Slot 143 = **Mission_Hunt** (`0x0044D880`) — v2 had ambiguous naming
- Slot 145 = **Mission_Construction** (`0x00449A50`) — confirmed
- Slot 311 = **OnConstructionComplete** (`0x00445F80`) — newly pinned
- Slot 313 = **DrawBody_VXL** (`0x0043DA80`) — function boundary created in T7
- Slot 315 = **DestructionEffects** (`0x004415F0`) — newly named
- Slots 322/330 = **IPersistStream MI secondary-vtable markers** (not functions)

### v2 §24 Engineer-survivor rule correction (T10)

v2 claimed: "Soviet Engineer rule (Factory==7 / ConYard, not Soviet-side; corrected R1)".
The v2 correction was incomplete — the rule is **fully side-independent**. `GetSurvivorInfantryType @ 0x0044EB10` reads only `+0x6E3` and `Type+0xEB8`; no country or side check. Any uncaptured ConYard (`Type+0xEB8 == 7`) has the 25% Engineer survivor chance — Allied, Soviet, AND Yuri alike. Ref T10/T13 C2.

### v2 §25 Open Questions — 10 of 11 closed

Pre-resolved in T1 audit (6): #1, #4, #5, #6, #9, #11.
Closed in T13 (4): #2 (BuildOrder +0x8/+0xC = DEAD); #7 (+0x700 = DEAD); #8 (SecretLab pick at +0x6F4); #10 (Type+0x184C = OUT OF BOUNDS, Rules-side).
Remaining: #3 was already resolved in v2's 2026-04-23 audit pass.

§25 residuals promoted into structural sections §§1/2/3/17/18/19/20/23; v3 §25 keeps only the genuinely unresolved items (≤3).

### New sections §26-§31

- **§26** Complete Field Map (cross-link to `BUILDINGTYPECLASS_FIELDS.csv`, 344 rows)
- **§27** Constructor Defaults Reference (cross-link to `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`)
- **§28** Full Vtable 338 slots (cross-link to renamed `BUILDINGCLASS_VTABLE_COMPLETE.md`)
- **§29** Save/Load Format (cross-link to `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md`)
- **§30** Tech Tree / Prerequisites (cross-link to `BUILDINGCLASS_PREREQUISITES_GHIDRA_REPORT.md`)
- **§31** Rendering Pipeline (cross-link to `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md`)

## Companion / Detail Reports

Earlier reports remain accurate for subsystems not re-verified here:
- `BUILDINGCLASS_VTABLE_COMPLETE.md` — **full 338-slot vtable map** (renamed from `_FULL_300`)
- `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md` — T6 full decomp of `0x004509D0`
- `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md` — T7 rendering pipeline
- `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md` — T8 slot 5/6 serialization
- `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md` — T9 placement chain
- `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` — T10 destruction chain (attribution-clarification note prepended)
- `BUILDINGCLASS_PREREQUISITES_GHIDRA_REPORT.md` — T11 tech-tree
- `BUILDINGCLASS_MISSION_GUARD_AND_CONSTRUCTION.md` — T12 slots 133 + 145
- `BUILDINGCLASS_RESIDUAL_Q_R4.md` — T13 residuals batch
- `BUILDINGTYPECLASS_FIELDS.csv` — T2+T3+T4 full field map (344 rows)
- `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` — T3 ctor defaults
- `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` — 27-step per-tick pipeline
- `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` — v1 combat notes (partially superseded here)
- `BUILDINGCLASS_MISSION_ATTACK_AND_RESIDUALS.md` — charge-mode 3-state + jump table
- `BUILDINGCLASS_MISSION_REPAIR_AND_PRODUCE.md` — 7-mode dispatch deep dive
- `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` — Mission_Selling
- `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md` — Cloning/Grinding/Hospital/Armory/BioReactor/SecretLab/OrePurifier/FactoryPlant
- `BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md` — CloakGenerator/SensorArray
- `BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md` — Nuke silo 5-state, Receive_Radio
- `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md` — 7 spy effects
- `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` — 11-check immunity (note: §3b has offset mistakes corrected in T10)
- `BUILDING_UPGRADE_SYSTEM_GHIDRA_REPORT.md` — 3-slot upgrades
- `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` — Per-building dock/exit
- `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` — 19-step ownership transfer
- `GARRISON_SYSTEM_GHIDRA_REPORT.md` — Garrison fire mechanics

---

## 1. Overview & Inheritance

BuildingClass is the runtime instance class for all buildings. Inherits
directly from **TechnoClass** — does NOT go through FootClass. Instance size
**0x720 bytes** (1824, verified from `vtable[12] SizeOf` returning 0x720). BuildingTypeClass
(template) size **0x1798 bytes** (6040, verified via ctor max-touched-offset +
alignment).

```
IUnknown (COM)
  └─ AbstractClass      vtable @ 0x007E1F50
    └─ ObjectClass      vtable @ 0x007EF060
      └─ MissionClass   vtable @ 0x007EDCC0
        └─ RadioClass   vtable @ 0x007F0508
          └─ TechnoClass vtable @ 0x007F4960
            └─ BuildingClass vtable @ 0x007E3EBC (338 slots, NULL at slot 338)
```

BuildingTypeClass primary vtable at **`0x007E4570`** (verified in T3 ctor decomp at `0x0045E2CD`).

### Kind Enum (vtable slot 11 — `What_Am_I`)

Distinct from slot 8 (`AbstractClass::WhatAmI` which returns type-class
instance index). Slot 11 returns a constant class-kind tag:

| Value | Class | Constant source |
|---|---|---|
| 1 | `UnitClass` | `0x00746E20` — returns 1 |
| 2 | `AircraftClass` | **(confirmed T1)** — `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md:255` (resolves v2 §25 #1) |
| 6 | `BuildingClass` | `0x00459EC0` — returns 6 |
| 0xF | `InfantryClass` | `0x00523340` — returns 0xF |

Used by `ExitObject`, FactoryPlant bonus dispatch, and other polymorphic branching.

---

## 2. BuildingClass Instance Layout (+0x000 to +0x720)

Legend: ✓ = verified from decompilation across multiple call sites.
Fields below +0x500 are TechnoClass-inherited.

### TechnoClass-inherited region (relevant subset)

| Offset | Type | Field | Evidence |
|---|---|---|---|
| +0x114 | int | `CargoClass::NumPassengers` (embedded, used for bio-reactor absorb count) | ✓ R1 |
| +0x158 | int | Strength (current HP) | ✓ |
| +0x218 | ptr | **PrimaryRadioContact / ActiveMissionObject** (TechnoClass-inherited, RTTI-tag 0xB = RADIO when non-null). Read in `Sell`, `OnConstructionComplete`, `UpdateAnimation` phase H. ✓ T13 B2 | ✓ |
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
| +0x540 | ptr | **Bridge destruction damage source** (TechnoClass-inherited; Abstract-pointer, fixed up on Save/Load). Passed to `vtable[0x16C]` on HighBridge damage. ✓ T13 B12 | ✓ |
| +0x544/+0x548/+0x54C | ptr | Abstract-derived pointers, fixup-registered on Load. Purpose DEFERRED (T13 B12 partial) | DEFERRED |
| +0x55C..+0x5AF | ptr[21] | **Anims[21]** array | ✓ |
| +0x5B0..+0x5C4 | byte[21] | **AnimStates[21] / ChargeFlags** — PoweredEffect active flag per slot. **READ-ONLY in UpdateAnimation; written only by OnPowerOn/OnPowerOff** (T6 clarification). | ✓ R1, T6 |
| +0x5C8..+0x5E7 | ptr[8] | Secondary anim/fire pointers | ✓ |
| +0x5EC | ptr | Upgrades[0] (BuildingTypeClass*) | ✓ |
| +0x5F0 | ptr | Upgrades[1] | ✓ |
| +0x5F4 | ptr | Upgrades[2] | ✓ |
| +0x5FC | int | Cycling anim phase index (nuke reactor special) | ✓ |
| +0x600 | ptr | **BuildingLightClass*** (spotlight, if Type+0x154B `HasSpotlight=yes`; size 0xE8, ctor `0x00435820`, released via vtable+0xF8) | ✓ R2 |
| +0x614 | ptr | **LightSourceClass*** (ambient light, all buildings with Type+0xE30..0xE40 set; size 0x4C, ctor `0x00554760`). **Destroyed earlier than +0x600**: +0x614 teardown happens in ReceiveDamage case 4; +0x600 teardown in Limbo §3l (T10). | ✓ |
| +0x618 | int | **Wall orientation metadata** (domain {0, 4, 8, 0xC}) — wall-sprite-facing selector, picks 1 of 4 wall-cap sprites. ✓ Unlimbo (T9) | ✓ T9 |
| +0x620 | int | **Timer accumulator** (heal/repair/production progress). **NOT cleared in destruction path** — freed by destructor anyway (T10 §13b). | ✓ MR&P |
| +0x624 | byte | "Timer fired this tick" flag | ✓ MR&P |
| +0x628 | int | CDTimer start frame | ✓ MR&P |
| +0x62C | int | CDTimer aux | ✓ MR&P |
| +0x630 | int | CDTimer rate | ✓ MR&P |
| +0x634 | int | CDTimer active flag (0 = paused) | ✓ MR&P |
| +0x638 | int | **Step amount** per CDTimer fire (added to +0x620) | ✓ MR&P |
| +0x660 | byte | **HasPower** | ✓ GoOnline/PowerCheck |
| +0x661 | byte | **IsOverpowered** | ✓ PowerCheck_Upgrade @0x00450614 |
| +0x662 | byte | (cleared at ctor; robot-tank online flag in UpdateGapAndSpecialEffects) | ✓ R1 |
| +0x664 | int | **`FirePowerBonus`** (IronCurtain fire-power multiplier). **v2 called this "Misc reset flag" — wrong (T1 correction; resolves v2 §25 #6).** | ✓ T1 (UPDATE_AI_TICK:832,929) |
| +0x668 | byte | **HasExtraPowerBonus** | ✓ GetPowerOutput @0x0044E7D5 |
| +0x669 | byte | **HasExtraPowerDrain** | ✓ GetPowerDrain @0x0044E89F |
| +0x66C..+0x67F | | DynamicVector (upgrade iteration; NOT absorb) | ✓ |
| +0x684..+0x697 | | **Occupant DynamicVector** (garrison InfantryClass*) | ✓ |
| +0x694 | int | Occupant Count (GetOccupantCount reads directly) | ✓ |
| +0x69C | int | **GarrisonFireIndex** (round-robin) | ✓ R1 |
| +0x6C9..+0x6CB | byte | ReadFromINI init flags | ✓ |
| +0x6D0..+0x6D8 | | CDTimer (**ProduceCashTimer**) — initialized in OnConstructionComplete if `Type+0x1560 ProduceCashStartup != 0` | ✓ T13 B23 |
| +0x6DC | byte | SellBuilding/NominalPower flag | ✓ |
| +0x6DD | byte | ConstructionComplete flag — set by UpdateAnimation phase H when BState_Frame hits stage-end (esp. 0x17 for ConYard buildup) | ✓ T6 |
| +0x6DF | byte | ForceShield active flag | ✓ |
| +0x6E0 | byte | **IC-killed flag** — set in DestructionEffects §2n if building died to IronCurtain. Gates out survivor spawning (GetSurvivorCount returns 0) and gates "CrateBeneath" drop. ✓ T10 | ✓ T10 |
| +0x6E3 | byte | **OwnershipChanged / Captured flag** (set in ChangeOwner; reduces crew bounty; halves survivor count; blocks Engineer roll). Initialized to 0 in ctor. **NOT bio-reactor.** ✓ T10 B18 | ✓ R2 |
| +0x6E4 | byte | **ActuallyPlacedOnMap** — one-shot gate for OnConstructionComplete | ✓ T13 B23 |
| +0x6E7 | byte | **FoggedSnapshot flag** (TS-legacy) — set only by `CreateFoggedSnapshot @ 0x004D0EF0` under `SpecialFlags & 0x1000`. Gates off VXL pass in Draw dispatcher and gates out selection/hover. **Default OFF in YR; do not implement.** ✓ T13 B5 | ✓ T13 B5 |
| +0x6EB | byte | CloakGenerator direction (0 / 1 / 0xFF=-1) | ✓ |
| +0x6EC | byte | CloakGenerator current radius | ✓ |
| +0x6ED | byte | Gap generator visual stage (0-16); also read by UpdateAnimation phase B as owner-remap byte | ✓ |
| +0x6F0 | int | Refinery ore level state / Refinery previous-tier cache (written in UpdateAnimation phase F) | ✓ T6 |
| +0x6F4 | ptr | **SecretLab runtime pick** (TechnoTypeClass*) — the randomly-rolled secret tech. Read in `FUN_00459840` (called from `HouseClass::CanBuild`); written in `FUN_0068C050` (per-lab roll at game start); fixup-registered on Load. ✓ T13 A3 (resolves v2 §25 #8) | ✓ T13 A3 |
| +0x700 | short | **DEAD** — ctor writes 0x3E8; UpdateAnimation phase B overwrites with facing-helper return. Zero readers anywhere in the binary (T13 A2). Safe to omit from Rust port. (Resolves v2 §25 #7.) | ✓ T13 A2 |
| +0x702 | byte | **UpgradeLevel** (0-3) | ✓ |
| +0x718 | int | **Bunker docking sub-state** (0-6, separate from +0xBC). **Terminal-state cleanup** (v2 §25 #11) resolved in `BUNKER_SYSTEM_GHIDRA_REPORT.md:135`. State 6 → Queue_Mission(GUARD) clears back to 0 on next commence. ✓ T1 | ✓ T1 |

### Special/dual-state fields

| Offset | Type | Field | Evidence |
|---|---|---|---|
| +0xAC | byte | MissionClass current mission enum (read by Mission_Dispatch) | ✓ |
| +0xBC | int | MissionClass sub-state (used by Sell 3-state, Hospital/Armory 2-state, etc.) | ✓ |
| +0x220 | int | **GapGenerator state** (0=Inactive, 1=Expanding, 2=Active, 3=Contracting) — NOT at +0xBC | ✓ R2 |
| +0x294 | ptr | ChargeSource (BerserkColor tint gate — T7) | ✓ T7 |
| +0x2E4 | ptr | Docked unit pointer (Bunker, Repair Depot) | ✓ MR&P |
| +0x2FC | int | Occupant / radio slot counter | ✓ MR&P |
| +0x350 | 40 bytes | **Gate timer RateTimer struct** (T7) | ✓ T7 |
| +0x41A | byte | "Is player house" indicator (for EVA/sound) | ✓ MR&P |
| +0x57C | ptr | = Anims[8] (Repair Depot arm extended) | ✓ R4 |
| +0x588 | ptr | = Anims[11] (Repair Depot arm retracted) | ✓ R4 |
| +0x58C | ptr | = Anims[12] (Repair Depot secondary) | ✓ R4 |

---

## 3. BuildingTypeClass Layout (+0x000 to +0x1798)

**Authoritative complete field map:** see `BUILDINGTYPECLASS_FIELDS.csv` (344 rows, T2+T3+T4).
**Constructor defaults reference:** see `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` (T3, 514 lines).

### Two-section parse pattern ($SELF + $IMAGE)

T2/T3 discovered that BuildingTypeClass INI parsing happens in **two passes**:
- **$SELF section** (135 rows): keys read from `[BuildingType]` section in rulesmd.ini (gameplay)
- **$IMAGE section** (209 rows): keys read from the art.ini image section `[Image]` pointed to by `Image=` (visual/animation)

Total: 344 rows — 317 YR-active, 23 TS-legacy, 4 conditional. Not previously documented as a two-section pattern.

### Binary quirks flagged by T2

- **SuperAnim LOAD/STORE offset mismatch bug** (8 rows): compiled ReadINI writes one offset but the cached runtime field is read from a different offset. Harmless because nothing else reads those cached values, but surprising.
- **QueueingCell.min uninit bug** (T3): `QueueingCell=` (+0x1618, short[2]) uses `.max` (2nd short) but `.min` (1st short) is never initialized — reads whatever was at that offset. Benign in stock because `.min` is only cross-referenced against `.max`.

### Core properties (selected highlights — see CSV for full table)

| Offset | Type | INI Key | Default | Purpose |
|---|---|---|---|---|
| +0x0CCE | bool | `Naval=` | false | Naval building (TechnoTypeClass) |
| +0xCCD | bool | `Crewed=` | (varies) | Ejects crew on destruction |
| +0xCCE | bool | `Naval=` | false | — |
| +0xE88 | char[24] | `PowersUpBuilding=` | "" | Upgrade target name |
| +0xEB4 | bool | `Occupier=` (InfantryTypeClass) | — | Infantry-type flag (on INF, not BLD) |
| +0xEB8 | int | `Factory=` | 0 | **TechnoTypeClass kind enum** — values: `3` = AircraftType, `7` = BuildingType (ConYard target), `0x10` = InfantryType, `0x28` = UnitType. **Do NOT confuse with BuildingClass `What_Am_I`** values (1/2/6/0xF, see §1). Verified in `OnSpyInfiltrate` (0x28/0x10 cases), `GetSurvivorInfantryType` (7), `HouseClass::GetAccumulatedBonus` (all four). |
| +0xEC8 | int[3] | `ExitCoord=` | 0,0,0 | Lepton offset for exit |
| +0xED4 | ptr | (computed) | NULL | Foundation exit-cell table ptr |
| +0xEE0 | int | `Power=` | 0 | Power output |
| +0xEE4 | int | `Power=` (neg) | 0 | Power drain |
| +0xEE8 | int | `ExtraPower=` | 0 | Extra power bonus (bio-reactor; also repurposed as infantry-absorb capacity in UpdateAnim phase D) |
| +0xEEC | int | `ExtraPower=` (neg) | 0 | Extra power drain |
| +0xEF0 | int | `Foundation=` | — | Foundation enum |
| +0xF4C..+0x13D0 | [21×0x44] | (art.ini) | — | **PowerUp anim entries** — 21 slots × 68 bytes each. **T3 11-subfield decomp:** each entry has healthy (offset +0x00, SHP name char[24]), damaged (+0x10, SHP name char[24]), plus 3 power flags (+0x40 Powered, +0x41 PoweredLight, +0x42 PoweredEffect), frame rate, offset, PowerUp damage state, and small PowerUp bookkeeping. See `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` for the full entry template. |
| +0x14E4 / +0x14EC / +0x14FC / +0x1504 | char[24] each | (DEFERRED — T13 B6) | "" | **Construction-overlay SHP slots** — DrawBody construction branch picks 1 of 4 based on a state byte. Likely "BuildupShape" variants (healthy-buildup / healthy-complete / damaged-buildup / damaged-complete). **Not yet mapped to INI keys** — ~30 min follow-up via LoadVisualAssets string anchors. |
| +0x14E0 | int | `Upgrades=` | 0 | Max upgrade slots (0-3) |
| +0x1518 | ptr | (SHP cache) | NULL | **Primary BibShape** — NOT a damaged-only variant; set from `BibShape=` in artmd.ini (T13 B7 corrects v1 confusion) |
| +0x151C | byte | (flag) | 0 | BibShape-present flag (set when `BibShape=` was parsed) |
| +0x154B | **bool** | **`HasSpotlight=`** | false | ✓ R2 — gates +0x600 allocation |
| +0x1571 | bool | (wall-related flag?) | — | ✓ (seen in Unlimbo) |
| +0x1573 | **bool** | **`Powered=`** | — | **v2 called this "Robot=?" — wrong (T1 correction; resolves v2 §25 #4).** Gates charge-mode power check in Mission_Attack; gates PoweredEffect anim linkage in CreateAnimForSlot; triggers OnPowerOn call in OnConstructionComplete. |
| +0x157B | bool | `CanBeOccupied=` | false | Infantry garrison |
| +0x157C | bool | `CanOccupyFire=` | false | Garrisoned infantry can fire |
| +0x1577 | bool | `CanC4=` | — | Infantry can place C4 (corrected from v1) |
| +0x1580 | int | `MaxNumberOccupants=` | 0 | Garrison capacity |
| +0x1584 | bool | `ShowOccupantPips=` | false | — |
| +0x1588..+0x1617 | | OccupantWeaponFireCoords | — | Fire port positions |
| +0x1618 | short[2] | `QueueingCell=` | 0,0 | Harvester queue cell (art.ini). **`.min` uninit bug** — only `.max` is written (T3). |
| +0x16FC | int | `PowersUpToLevel=` | -1 | Target upgrade level |

### Prerequisite storage — on TechnoTypeClass, not BuildingTypeClass (T11)

v2 mentioned prerequisites without clarifying where they live. **They are inherited TechnoTypeClass fields:**
- `+0x638` — `Prerequisite=` (DynamicVectorClass<int>, 12 bytes)
- `+0x654` — `PrerequisiteOverride=` (DynamicVectorClass<int>, 12 bytes)

Stored as signed integers: non-negative = BuildingTypeClass array index; negative = categorical token (-1 POWER, -2 FACTORY, -3 BARRACKS, -4 RADAR, -5 TECH, -6 PROC). See `BUILDINGCLASS_PREREQUISITES_GHIDRA_REPORT.md` §2.

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
| +0x16C5 | `TurretAnimIsVoxel=` | Gates FIRE_FACING rotation branch (alongside HasTurret). Resolved R4. |
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
| +0x16E8 | float | `ChargedAnimTime=` | Charge animation duration — used in UpdateAnim phase G with tick-to-time multiplier 0.001111f (=1/900) at `0x007E44C0` and upper-bound gate 990.0f at `0x007E44C4` (block runs only when ChargedAnimTime ≤ 990.0). (corrected 2026-05-29: was "scaler 990.0f at 0x007E44C0 and min-charge cutoff 0.001111f at 0x007E44C4" — addresses were swapped; verified via read_memory 0x007E44C0→0x3A91A2B4≈0.001111f and 0x007E44C4→0x44778000≈990.0f, confirmed by decompile_function 0x004509D0 UpdateAnimation — OPERATOR_OR_ORDER_DRIFT) |
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
| +0x1707 | byte | `CloakRadiusInCells=` | Default 0x14 (20); used by CloakGen + **SensorArray REMOVE only** (asymmetry bug — add reads `Type+0x5F0 SensorsSight`) |
| +0x1710 | int | `BarrelStartPitch=` | Barrel starting pitch — **read by AnimClass::DrawIt slot 9, NOT by DrawBody** (T13 B10) |
| +0x1763 | bool | `IsThreatRatingNode=` | — |
| +0x1764 | bool | `PrimaryFireDualOffset=` | — |
| +0x1765 | bool | `ProtectWithWall=` | AI walls |
| +0x1766 | bool | `CanHideThings=` | Can hide units underneath |
| +0x1767 | bool | `CrateBeneath=` | Spawn crate on destroy — **only fires under IronCurtain carryover!** (T10 §7 — classic gamemd bug; must reproduce) |
| +0x1768 | bool | `LeaveRubble=` | **PARSED BUT DEAD** — no consumer anywhere in binary. Leftover from TS rubble system. Do NOT implement. (T10 §6a) |
| +0x1769 | bool | `CrateBeneathIsMoney=` | Money crate |
| +0x1780 | int | `NumberOfDocks=` | Dock pads |
| +0x1788 | ptr | DockingOffset data | 12-byte lepton entries |

---

## 4. Vtable Summary (338 slots — see §28 / `BUILDINGCLASS_VTABLE_COMPLETE.md`)

BuildingClass has a **338-slot vtable** at `0x007E3EBC` (v2 claimed 300 — undercount). 101 overrides (primary range 0..299); secondary-vtable MI markers at slots 322 and 330.

**Key slots referenced across this document:**

| Slot | Offset | Address | Purpose |
|---|---|---|---|
| 3 | 0x00C | 0x00459E80 | `BuildingClass::GetClassID` (GUID) |
| 5 | 0x014 | 0x00453E20 | `BuildingClass::Load` (IPersistStream::Load — v2 had slot 5/6 swapped) |
| 6 | 0x018 | 0x00454190 | `BuildingClass::Save` (IPersistStream::Save) |
| 8 | 0x020 | 0x00459F20 | `ScalarDeletingDestructor` (NOT AbstractClass::WhatAmI) |
| 9 | 0x024 | 0x00442C40 | `Init_Managers` |
| 10 | 0x028 | 0x0044E8F0 | `GetType` |
| 11 | 0x02C | 0x00459EC0 | **`What_Am_I` (kind: 1/2/6/0xF)** — returns 6 |
| 12 | 0x030 | 0x00459E70 | **`SizeOf`** — returns 0x720 |
| 13 | 0x034 | 0x00454260 | `Save_ChecksumFields` |
| 18 | 0x048 | 0x00447AC0 | `GetCoords` |
| 23 | 0x05C | 0x0043FB20 | `Update` (per-tick AI; only caller of UpdateAnimation) |
| 37 | 0x094 | 0x00452630 | **`IsDeployable`** (v2: wrongly "CanAcceptUpgrade") |
| 41 | 0x0A4 | 0x004500A0 | `GetTargetCoords` |
| 42 | 0x0A8 | 0x00447B20 | `GetDockCoord` |
| **53** | **0x0D4** | **0x00445880** | **`Limbo`** — remove-from-map cleanup (NOT "OnDestroyed"; renamed in Ghidra 2026-04-24) |
| 54 | 0x0D8 | 0x00440580 | `Unlimbo` (place on map) |
| **55** | **0x0DC** | **0x0044EBF0** | **`Destroy`** — factory teardown (v2 called this "Limbo" — wrong) |
| 64 | 0x100 | 0x00443C60 | `ExitObject` (6724 bytes) |
| 65 | 0x104 | 0x0043CEA0 | Draw dispatcher |
| 69 | 0x114 | 0x0043D290 | `DrawBody` (SHP pass) |
| 91 | 0x16C | 0x00442230 | `ReceiveDamage` |
| 101 | 0x194 | 0x0043C2D0 | `Receive_Radio` |
| 104 | 0x1A0 | 0x00447110 | `TogglePowerOrGate` |
| 122 | 0x1E8 | 0x005B35E0 | `Queue_Mission` (inherited) |
| 123 | 0x1EC | 0x005B3570 | `Commence` (inherited) |
| 132 | 0x210 | 0x0044ACF0 | **`Mission_Attack`** |
| 133 | 0x214 | 0x0044B760 | `Mission_Guard` (trivial stub: `MOV EAX, 0x1C2; RET`) |
| **143** | **0x23C** | **0x0044D880** | **`Mission_Hunt` / slave-deploy** (T12 resolved v2's ambiguity) |
| **145** | **0x244** | **0x00449A50** | **`Mission_Construction`** (2-state build-up anim driver) |
| 146 | 0x248 | 0x00449C30 | `Mission_Selling` |
| 147 | 0x24C | 0x0044B780 | `Mission_RepairAndProduce` |
| 148 | 0x250 | 0x0044C980 | `Mission_Missile` (nuke) |
| 168 | 0x2A0 | 0x00457770 | `CanCloak` |
| 169 | 0x2A4 | 0x004578C0 | `ShouldUncloak` |
| 195 | 0x30C | 0x0044EB10 | `GetSurvivorInfantryType` (Ghidra label "GetVoiceResponse" is wrong) |
| 212 | 0x350 | 0x004555D0 | `CanSellOrUndeploy` |
| 240 | 0x3C0 | 0x00447F10 | **`GetFireError`** (returns 0-10 enum) |
| 242 | 0x3C8 | 0x00443B90 | `ToggleGate` / `ClearTarget / Set_ArchiveTarget(0)` |
| 243 | 0x3CC | 0x006FDD50 | `Fire_At(target, weaponIdx)` (inherited) |
| 245 | 0x3D4 | 0x00448260 | `ChangeOwner` |
| 254 | 0x3F8 | 0x004526F0 | `GetWeapon` (upgrade-aware) |
| 255 | 0x3FC | 0x004527D0 | `HasTurret` |
| 258 | 0x408 | 0x004581F0 | `GetOccupantCount` |
| 260 | 0x410 | 0x00454DB0 | `UpdateGapGenerator_Tick` |
| 279 | 0x45C | 0x007036C0 | `TechnoClass::StartUncloaking` (inherited) |
| 280 | 0x460 | 0x00703770 | `TechnoClass::StartCloaking` (inherited) |
| 293 | 0x494 | 0x00456580 | `RegisterOnRadar` |
| **311** | **0x4DC** | **0x00445F80** | **`OnConstructionComplete`** (T13 B23 — fully decomped) |
| **313** | **0x4E4** | **0x0043DA80** | **`DrawBody_VXL`** — VXL/extras pass (function boundary created in T7) |
| **315** | **0x4EC** | **0x004415F0** | **`DestructionEffects`** — the real HP=0 handler (newly pinned) |
| 317 | 0x4F4 | 0x00455820 | `AddSensorArrayAt` |
| 318 | 0x4F8 | 0x004556D0 | `RemoveSensorArrayAt` |
| 322 | 0x508 | 0x007FC298 | **Secondary vtable marker (IPersistStream MI)** — not a function |
| 330 | 0x528 | 0x007FC390 | **Secondary vtable marker (MI)** — not a function |
| 338 | 0x548 | 0x00000000 | NULL terminator |

---

## 5. Lifecycle — Construction → Limbo → Destroy

### Construction

1. **Constructor** `0x0043B740`: allocates 0x720, inits all fields. Sets +0x6E3 = 0 (captured flag), +0x5B0..+0x5C4 = 0 (AnimStates), +0x664 = 0 (FirePowerBonus), +0x660 = 1 (HasPower default), +0x700 = 0x3E8 (dead field).
2. **ReadFromINI** `0x0044F820`: parses INI data
3. **Unlimbo** `0x00440580` (~4200 bytes — T9 full decomp in `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md`): places on map. Three branches: wall-extension, upgrade-attach, normal-placement. Key steps for normal path:
   - Cell occupancy: `(W+2)*(H+2)` box, `cell+0x122 += 1` per cell (byte counter, NOT a bitflag)
   - Wall orientation `+0x618` ∈ {0, 4, 8, 0xC}
   - Passability mask `cell+0xDC |= (1 << owner_idx)` across `(W + 2*AIBaseSpacing) × (H + 2*AIBaseSpacing)` box (Rules+0x1460 = `AIBaseSpacing`, default 1000 — T13 B13)
   - `BuildingLightClass*` allocated at +0x600 if `Type+0x154B HasSpotlight=yes` (size 0xE8, ctor `0x00435820`)
   - `LightSourceClass*` allocated at +0x614 if Type+0xE30..+0xE40 ambient light set (size 0x4C, ctor `0x00554760`)
   - HouseClass registration (radar, sensor, gap, factory, dock, spysat lists — 11 trait-list insertions)
   - CloakGenerator: Owner+0x56F8 = 1 if Type+0x16C7
   - Shroud reveal via TechnoClass::Unlimbo (not inline RevealAroundCell in normal branch)
   - NO sound (caller plays "ConstructionComplete" voc)
   - NO ClearBibArea (that's for ExitObject, not placement)
4. **Place_OccupyMap** `0x00441F60`: runs on the **first Update tick** (not from Unlimbo). Writes `cell.OverlayTypeIndex = 0xEF`, passability flags, zone rebuild (via PostDestructionWallCleanup). **This is where the 1-tick placement delay lives.** Also where `CrateBeneath=` fires — but ONLY on the IC-carryover path, not on normal placement (classic gamemd quirk).
5. **OnConstructionComplete** `0x00445F80` (vtable slot 311, T13 B23): one-shot post-animation handler. ProduceCash timer init, initial anim creation, Owner counter bumps (OrePurifier, InfantryGainSelfHeal, UnitsGainSelfHeal, FactoryPlant), SW indicator anim, LightSource allocation, ConnectWalls, AddSensorArrayAt, AddDetectDisguiseAt, rally point (naval WF), OnPowerOn (if Powered), ConYard free-MCV spawn, initial paratrooper launch. Sets `+0x6E4 = 1` to fencepost the one-shot.

### Per-Tick Update

`BuildingClass::Update` at `0x0043FB20`. 27-step pipeline detailed in
`BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`. Mission dispatch uses
`MissionClass::Mission_Dispatch` at `0x005B3060`. `UpdateAnimation @ 0x004509D0` is called once per tick unconditionally (T6 full decomp in `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`).

### Destruction — the three-function chain (CORRECTED from v2)

Old v2 wording used wrong function names. The actual chain:

```
ReceiveDamage (0x00442230) returns NowDead (4)
  └─ DestructionEffects (0x004415F0, slot 315)      ← the HP=0 event
        ├─ 8 anim-slots destroyed (+0x5C8..+0x5E4)
        ├─ GapGen/CloakGen radius cleanup, wall reconnect (for laser-fence posts)
        ├─ BuildingDieSound (default via `Rules[AudioVisual] BuildingDieSound`)
        ├─ Big-building (≥2×2) debris shower (50/50 metal vs smoke)
        ├─ Per-foundation-cell Type+0x73C death-anim spawn
        ├─ Nuclear-reactor 4-cell radioactive overlay (Type+0xD15)
        ├─ Tiberium/ore spill (StorageClass.Remove × 2, PlaceTiberium per iteration)
        ├─ Cost-refund stub (Rules+0x5C8 divisor — was hypothesized as `StorageSpillThreshold` but resolved as `ShakeScreen=` in T13 B17; the real divisor semantics remain unclear)
        ├─ Type+0x74C death-explosion AnimClass
        ├─ Particle system spawn (smoke plume, big buildings)
        ├─ Sets this->Health = 0
        ├─ Sets +0x6E0 = 1 if IC-killed
        └─ SpawnSurvivors + EMPPassengers
  └─ (ObjectClass::UnInit path) → eventually calls vtable[0xD4]
       └─ Limbo (0x00445880, slot 53)              ← remove-from-map cleanup
             ├─ Another 8-anim-slot walk (belt-and-braces)
             ├─ Owner counter decrements (OrePurifier/Helipad/SelfHeal grants)
             ├─ ConnectWalls(this, 0) for laser-fence posts
             ├─ RemoveSensorArrayAt + RemoveDetectDisguiseAt (gated)
             ├─ Power grid recalc
             ├─ Nuclear-reactor 8-cell ore destruction (if Type == Rules+0x87C)
             ├─ Bridge-repair-hut cell refresh
             ├─ Per-foundation `(W+2)*(H+2)` cell+0x122 DECREMENT (inverse of Unlimbo)
             ├─ Tactical screen rect dirtied
             ├─ BuildingLight (+0x600) teardown (LightSource +0x614 was torn down earlier)
             ├─ HouseClass::Recount (per-kind counters decrement)
             ├─ HouseClass::Recalc_Base_Center
             ├─ CleanBasePlanForLostBuilding (FUN_0050A490) — removes BasePlanNode
             ├─ TechnoClass::Limbo_Helper → ObjectClass::Conceal → +0x81 InLimbo = 1
             ├─ SetSuperWeaponDirty (if upgrades had super-weapon linkage)
             └─ RevalidateFactoryQueueForKind (misnamed "UpdateRadar" @0x00509140)
  └─ Destroy (0x0044EBF0, slot 55)                 ← final deallocate
        ├─ Aborts factory production, ejects queued units
        └─ ObjectClass::Destroy → operator_delete
```

### SpawnSurvivors (T10)

Side-based divisor (NOT difficulty): `clamp(Cost / SideSurvivorDivisor, 1, 5)`:
- Allied side: `Rules+0x14F8 AlliedSurvivorDivisor` (default 1000)
- Soviet side: `Rules+0x14FC SovietSurvivorDivisor` (default 1000)
- Yuri (Third): `Rules+0x1500 ThirdSurvivorDivisor` (default 1000)

Zero survivors if:
- `Type+0xCCD Crewed = no`
- `+0x6E0 IC-killed = 1`
- Owner defeated (`Owner+0x1F6`)
- On bridge (per v2)

**Engineer rule (T10 + T13 C2 — side-independent):**
- `+0x6E3 == 0` (not captured) AND `Type.Factory (+0xEB8) == 7` (WeaponsFactory/ConYard) AND `random 0..99 < 25`
- Yields Rules+0xF70 `Engineer=`
- **Fires for Allied, Soviet, and Yuri equally — v2 §24 "Soviet-only" framing is wrong.**

Otherwise → `TechnoClass::Crew_Type (0x00707D20)` → side-based default + 15% Technician override.

---

## 6. Damage & Immunity System (11-Check)

Full detail in `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` (note: §3b offset labels have mistakes; see T10 §3b corrections).

**Checks (ordered):** self-damage guard → wall immunity → insignificant+bridge
repair hut → type-immune (same type + same owner) → IronCurtain → WarpingOut
→ Radiation+ImmuneToRadiation → PsychicDamage+ImmuneToPsionicWeapons →
Poison+ImmuneToPoison → AffectsAllies=no+allied → insignificant.

**Damage state thresholds:**
- ConditionYellow: health crosses below `Rules+0x1700` (typically 50%)
- ConditionRed: health crosses below `Rules+0x1708` (typically 25%)

`SetDamagedState` (`0x00451EE0`) swaps anim arrays; `CreateDamageFireAnims`
(`0x0043C0D0`) spawns fire overlays.

### DestructionEffects events (T10 §2)

When HP reaches 0, `ReceiveDamage` case 4 invokes **DestructionEffects (`0x004415F0`, vtable slot 315)** — the pivotal kill-event handler:

| Step | Effect |
|---|---|
| Destroy 8 anim slots | +0x5C8..+0x5E4 nulled |
| GapGen/CloakGen cleanup | +0x210 / +0x16A4 rebuild |
| Wall reconnect (laser fence) | `ConnectWalls(this, 1)` |
| Death VOC | `VocClass::PlayAtCoord(0)` → Rules `BuildingDieSound` |
| Big-building debris (≥2×2) | 50/50 SpawnDebris vs Debris_Smoke |
| Per-cell death anims | `Type+0x730/0x73C` |
| Reactor fallout | `Type+0xD15` 4-cell radioactive overlay |
| Tiberium spill | StorageClass remove + PlaceTiberium |
| Type+0x74C death-explosion | Primary AnimClass spawn |
| Particle system | `Type+0x798/0x7A4` |
| `Health = 0` | explicit |
| `SpawnSurvivors` | garrison eject, crew spawn |
| `EMPPassengers` | cascade EMP to boarded infantry |

### Engineer-from-ConYard rule (T13 C2 correction)

**v2 §24 was wrong.** The 25% Engineer survivor rule is **side-independent**:
- `+0x6E3 == 0` (never captured)
- AND `Type.Factory (Type+0xEB8) == 7` (Factory=BuildingType, i.e. ConYard)
- AND `random 0..99 < 25`

No side/country check. Allied, Soviet, Yuri — all get the same 25% Engineer chance on their uncaptured ConYard dying.

### Post-death observables (T10 §5-§8)

- **Craters:** NOT from OnDestroyed/DestructionEffects — from the death anim(s) in `Type+0x74C` (AnimTypeClass.Crater=yes + ForceBigCraters=). No `Crater=` key on BuildingType.
- **Rubble tile:** NONE. `LeaveRubble=` (Type+0x1768) is PARSED BUT DEAD — zero consumers in binary. Do NOT implement.
- **EVA cue:** NONE from OnDestroyed. "Under attack" EVA fires from ReceiveDamage before death, not on death.
- **`CrateBeneath=`:** only fires on IC-carryover path (Place_OccupyMap runs on IC death, normal death skips it). Classic gamemd bug — must reproduce for parity.

See `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` for full destruction chain, including the attribution note that clarifies the doc describes 0x004415F0 (not 0x00445880).

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

### GetDockCellForObject (`0x0044EFB0`, vtable slot 309) — Exit cell selection

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

Gated by `Type+0x16BD WeaponsFactory=` (NOT `Bib=`). Scatters any unit blocking the bib via
`CellClass::Scatter_Objects` up to 8 iterations with `Pathfinding_update_continued`.
Called from ExitObject-style paths. NOT called from Unlimbo (T9 clarified).

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

**Full deep dive: `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`** (T6 — 1874 bytes decomped at `0x004509D0`).

21 fixed anim slots stored as pointers in `Anims[]` at +0x55C. Type-side
entries at Type+0xF4C..+0x13D0 (21 × 0x44-byte entries). **Entry format (T3 11-subfield decomp):** healthy SHP name (offset +0x00), damaged SHP name (+0x10), 3 power flags (+0x40/+0x41/+0x42), frame rate, offset coord, and smaller bookkeeping. See CTOR_DEFAULTS doc.

### UpdateAnimation responsibilities (T6)

1. **Production frame advance** (phase A) — steps `+0xF8` BState frame counter against CDTimer at `+0x100`
2. **6 type-gated mini state machines** driving 10 of 21 slots:
   - Phase C — UnitRepair (slots 8/B/C)
   - Phase D — InfantryAbsorb (slots 3/4)
   - Phase E — SiloDamage (slot 10)
   - Phase F — Refinery tier (slots 3-6)
   - Phase G — SuperWeapon charge (slots 14/15/17/19)
   - Phase H — BState terminal advance
3. **Shadow/facing/remap sync** (phases B, J, K) — all 21 slots touched

### AnimStates[21] ownership (+0x5B0..+0x5C4) — T6 clarification

**UpdateAnimation is READ-ONLY** on the 21-byte AnimStates array. Writes come only from:
- `OnPowerOff @ 0x004545D0` — clears bytes for slots whose Flag C (+0xF8E) is set
- `OnPowerOn @ 0x004547C0` — sets bytes for those slots

### Phase G magic numbers (ChargedAnimTime timing)

- **`_DAT_007E44C0 = 990.0f`** — float-to-tick conversion factor
- **`_DAT_007E44C4 = 0.001111111f` (= 1/900)** — minimum-charge cutoff
- `Type+0x16E8 ChargedAnimTime=` — from INI, seconds-ish (e.g. 900 for nuke)

Formula: `if (remaining_ticks × 990.0f < ChargedAnimTime) → post-charge indicator`.

### Tesla charge — NOT driven by UpdateAnimation (T6)

Tesla Coil charge-mode 3-state lives in **Mission_Attack** (per v2 §R4), NOT UpdateAnimation. Phase G only handles **superweapon** charge indicators (Chrono/IC/Nuke), not per-weapon chargers like Tesla.

### Slot roles (verified from `UpdateAnimation` @ `0x004509D0`)

Slots are NOT a clean one-role-per-range partition — a single slot serves
different purposes for different Type flags.

| Slot | Instance field | Type-entry offsets | Role (binary-verified) |
|:---:|---|---|---|
| 0-2 | +0x55C..+0x564 | Type+0xF4C..+0xFD4 | Upgrade (PowerUp1/2/3) anims — see §9 |
| 3 | +0x568 | Type+0x1018/+0x1028/+0x1038 | Bio-Reactor **empty**; Refinery ore tier 0 |
| 4 | +0x56C | Type+0x105C/+0x106C/+0x107C | Bio-Reactor **with-cargo**; Refinery ore tier 1 |
| 5 | +0x570 | Type+0x10A0/+0x10B0 | Refinery ore tier 2 |
| 6 | +0x574 | Type+0x10E4/+0x10F4 | Refinery ore tier 3 |
| 8 | +0x57C | Type+0x116C/+0x117C | Repair Depot arm extended |
| 9 | +0x580 | — | Turret sprite facing (shadow-direction lookup; BarrelStartPitch integrated via AnimClass::DrawIt) |
| 10 | +0x584 | Type+0x11F4/+0x1204 | Weeder / SiloDamage storage anim (4-tier) |
| 11 | +0x588 | — | Repair Depot arm retracted |
| 12 | +0x58C | Type+0x127C/+0x128C | Repair Depot secondary |
| 14 | +0x594 | Type+0x1348/+0x1358 | SuperWeapon pre-charge |
| 16 | +0x59C | Type+0x13D0/+0x13E0 | SuperWeapon charged/ready |

### Damage-state hysteresis

No hysteresis in UpdateAnimation — each branch picks healthy/damaged per-tick based on `GetHealthRatio() > Rules+0x1700 ConditionYellow`. Cross of the threshold cascades via `SetAnimSlotImage` re-imaging all 21 slots ("flash on damage" parity behavior).

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

State at **`BuildingClass+0x220`** (DWORD, NOT +0xBC; corrected R2, re-confirmed T13 C3).

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
| +0x1768 | `LeaveRubble=` | **Parsed but DEAD** — no consumer in binary (T10 §6a) |
| +0x6E7 | FoggedSnapshot flag | TS-legacy, SpecialFlags-gated (T13 B5) |

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

14 BuildingClass overrides (was 12 in v2 — add Mission_Guard and Mission_Construction from T12). Dispatched via `MissionClass::Mission_Dispatch` at `0x005B3060`.

| Enum | Name | Vtable Slot | Addr | Size |
|---|---|---|---|---|
| 1 | Attack | 132 | `0x0044ACF0` | ~1174 |
| 5/6 | Retreat/Sleep | 135 | `0x004496B0` | ~902 |
| **8** | **Guard** | **133** | **`0x0044B760`** | **2 instr (stub MOV EAX, 0x1C2)** — T12 |
| 10 | Return | 137 | `0x0044B770` | ~16 |
| 11 | Stop | 136 | `0x00449A40` | ~8 |
| 16 | Hunt/Rescue | 143 | `0x0044D880` | ? |
| 17 | Harmless | 133 | reuses Guard | — |
| **18** | **Construction** | **145** | **`0x00449A50`** | **354 (2-state anim driver)** — T12 |
| 19 | Selling | 146 | `0x00449C30` | 3989 |
| 20 | RepairAndProduce | 147 | `0x0044B780` | **4604** |
| 22 | Missile | 148 | `0x0044C980` | 3104 |
| 24 | Unload | 149 | `0x0044E440` | ? |

### Mission_Guard (T12)

Trivial stub — `MOV EAX, 0x1C2 (=450); RET`. Identical to `MissionClass::Mission_Default`. The slot exists only to satisfy the vtable override contract. No state, no transitions. 450-frame sleep timer (~30s at 15 fps). **Combat-capable defenses (Tesla Coil, Prism Tower, Sentry Gun, etc.) run Mission_Attack, NOT Mission_Guard.**

### Mission_Construction (T12)

2-state machine gated on `+0xBC`. Pure anim driver — no power gate, no HP scaling. After the "build up" anim completes, transitions back to GUARD. 354 bytes.

### Mission_Attack (0x0044ACF0) — Combat dispatcher

**Full detail in `BUILDINGCLASS_MISSION_ATTACK_AND_RESIDUALS.md`**

**Path A — Direct fire** (`Type+0x16B8` IsChargeMode = 0):
1. No target → clear target, Queue_Mission(**mission 5 = Sleep/Retreat**, **NOT** Guard — see §17 handler table: 5→Retreat/Sleep, 8→Guard), Commence
2. Has target → compute fire_error via `vtable[0x3C0]` (GetFireError)
3. fire_error == 2 (FIRE_FACING): if HasTurret AND `Type+0x16C5` (TurretAnimIsVoxel): rotate, re-check
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

- **State 0 (pre-charge)**: **Conditional power check** — only when `Type+0x1573 Powered=` is set AND `Type.Drain > 0`; in that case if `HouseClass::GetPowerRatio() < 1.0` the state advance is skipped (wait). For buildings without that flag (most charge-mode defenses: Tesla Coil, Prism Tower in stock YR), State 0 advances regardless of house power. Then validate target kind. If facing delta `< 0x2001` (~45°) → state=1; else `RateTimer::Set(target_facing)` to rotate
- **State 1 (charging/fire)**: Re-validate target visibility via `vtable[0x1D0]`. fire_error ∈ {5,6,8}: abort to state=0. fire_error == 0: fire both weapons `Fire_At(target, 0)` and `Fire_At(target, 1)`, state=0
- **State 2+ (cooldown)**: return `MissionClass::GetMissionTimerEntry() + Random(0, 2)` — jittered cooldown prevents lockstep fire

Facing tolerance `0x2001`: in 0-0xFFFF facing space = ~one compass direction
(8 directions × 0x2000 = 0x10000). Gates state 0→1 transition.

### Mission_RepairAndProduce (0x0044B780) — 7-Mode Dispatcher

**Full detail in `BUILDINGCLASS_MISSION_REPAIR_AND_PRODUCE.md`**

Dispatches on Type flag:

1. **Bunker** (Type+0x16AB) → FUN_00458E50 — **6-state docking machine at +0x718** (0: arrival, 1: dock slot search, 2: CDTimer + anim, 3: arrival check, 4: anim activation, 5: link + complete, 6: terminal — state 6 → Queue_Mission(GUARD) clears back to 0 on next commence; v2 §25 #11 resolved via T1)
2. **ConstructionYard** (Type+0x16B9) → 2-state (+0xBC): GrandOpening → idle monitor
3. **Hospital** (Type+0x16C1) → 2-state heal timer. Formula: **`Rules+0x16F0 IRepairRate × 900.0`** threshold. Response 0x21 (REPAIR_COMPLETE) → radar event + VoxEVA + VocPlay, eject, Queue_Mission(GUARD)
4. **Armory** (Type+0x16C2) → 2-state identical timer, uses `VeterancyStruct::SetVeteran/SetElite` instead of heal radio
5. **Repair Depot** (Type+0x16A9) → 3-state (+0xBC) with LocomotionClass piggyback:
   - State 0: `LocomotionClass::QueryInterface_IPiggyback` attach, radio 0x13, distance check
   - State 1: Drive-in phase, health check; if `Rules+0x16F8` (hardcoded 1.0) ≤ health: release; else: start repair anim
   - State 2: HP tick with `Rules+0x16E8 URepairRate (=0.016 default) × 1.0` threshold; radio 0x13 → 0x1C response determines retry/complete
6. **Helipad** (Type+0x16AA) → per-aircraft radio cycle (0x1D `REFUEL_QUERY` → 0x13 → 0x1F → 0x1C; v2 §25 #5 resolved via T1)
7. Default → return 0xF (15-frame re-check)

Accepted locomotors at Repair Depot: `CLSID_WalkLocomotion` and
`DriveLocomotion` (CLSID at `DAT_007E9AB0` = `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}`).

### Mission_Selling (0x00449C30) — 3-state with MCV undeploy

State at +0xBC:
- **State 0**: Init. If upgrades exist, refund LAST upgrade at full cost (not SellBack%), Queue_Mission(GUARD)
- **State 1**: Eject + animate. Plays GrandOpening(0) reverse anim
- **State 2**: Finish / undeploy. If `Type+0x408` UndeploysInto: attempt MCV undeploy

**Refund formula (T13 C1 confirmed):** `Cost × Rules.SellBack (+0x145C, default 50%) + stored ore`. **NOT health-scaled** — sick building refunds same as full HP.

**MCV undeploy:**
- `operator_new(0x8E8)` + `UnitClass::Constructor(Type+0x408, Owner)`
- On alloc-fail (`0x0044A19E`): `vtable[0x2BC] (GetRefundValue)` + Add_Credits
- On placement-fail (`0x0044A16B`): cached `vtable[0x2BC]` result from pre-attempt, Add_Credits
- On success: MCV health = `floor(HealthRatio × UnitType.Strength)`, min 1. Inherits radar jam, gap-gen state, cloak shroud mask, **SoundEvent at +0x4DC..+0x4F4** (20-byte copy via MOVSD.REP + 2 DWORDs). All radio-linked units re-bound.

**Survivor count:** `GetSurvivorCount` (vtable+0x2D0 = `0x00451330`).
Formula: `clamp(Cost / SurvivorDivisor[side], 1, 5)`. **Bio-reactor doubles divisor.**
Zero on bridge or Crewed=no. If +0x6E3 (OwnershipChanged) != 0: divisor doubled again (halves).

**Survivor infantry type:** `GetSurvivorInfantryType` (vtable+0x30C = `0x0044EB10`):
- If +0x6E3 (OwnershipChanged) == 0 AND `Type+0xEB8 == 7` (Factory=BuildingType / **ConYard**): 25% Engineer (Rules+0xF70). **Side-independent — v2 §24 "Soviet-side only" claim is WRONG (T10+T13 C2).**
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

Additional helipad radio (MR&P + T1): `0x13` REQUEST_APPROACH, **`0x1D` REFUEL_QUERY** (confirmed, resolves v2 §25 #5), `0x1F` RESERVE_DOCK.

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
- Radius fields (asymmetric — real bug in gamemd.exe): **AddSensorArrayAt reads Type+0x5F0 (`SensorsSight`, int)**; **RemoveSensorArrayAt reads Type+0x1707 (`CloakRadiusInCells`, byte, default 0x14 = 20)**. For retail YR Psychic Sensor (`SensorsSight=15`, no `CloakRadiusInCells` override), add zone = 15 cells but remove zone = 20 cells, so remove decrements ref-counts on cells that were never incremented. Rust impl should use the add radius for both paths to avoid the drift. (T13 C4)

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

- Pool = Rules.SecretInfantry (+0xD00) + SecretUnits (+0xD1C) + SecretBuildings (+0xD38). Fisher-Yates sample per lab.
- **Runtime pick storage: `BuildingClass+0x6F4`** (T13 A3 — resolves v2 §25 #8). Written at game start by `FUN_0068C050`; read in `FUN_00459840` (called from `HouseClass::CanBuild`); fixup-registered in `BuildingClass::Load` as Abstract-derived pointer.
- Type-level overrides: `Type+0xEA4 SecretInfantry=`, `Type+0xEA8 SecretUnit=`, `Type+0xEAC SecretBuilding=` — if non-zero, those override the random pick at +0x6F4.
- Registry at `0x00442C40`; assignment at `0x0068C050`.

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
| +0x8 | 4 | **DEAD** — written as 0 in producer; never read by any consumer (T13 A1 — resolves v2 §25 #2) |
| +0xC | 4 | **DEAD** — same verdict (T13 A1) |

Implication: treat as 8-byte (type, cell) for Rust — the last 8 bytes are padding only.

### Consumers

- `HouseClass::AI_Manage_Build_Queue` (`0x004FDD10`) — adds
- `HouseClass::AI_ChooseNextProduction` (`0x00506EF0`) — reads only +0x0 and +0x4
- `BuildingClass::ExitObject_Main` — removes on spawn (or updates cell for IsBaseDefense)
- `FUN_0050A490` (OnBuildingDestroyed hook) — invalidates on destruction; writes only +0x0 and +0x4

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

From `RulesClass::ReadGeneral` at `0x0066D530` (verified R4 + T13):

| Offset | Type | INI Key | Default | Purpose |
|---|---|---|---|---|
| Rules+0x1460 | int | `[AI] AIBaseSpacing` | **1000** | Passability OR-mask radius for base placement (T13 B13) |
| Rules+0x14F8 | int | `AlliedSurvivorDivisor` | 1000 | Side 0 survivor divisor |
| Rules+0x14FC | int | `SovietSurvivorDivisor` | 1000 | Side 1 |
| Rules+0x1500 | int | `ThirdSurvivorDivisor` | 1000 | Side 2 (Yuri) |
| Rules+0x16CC | int | `RepairStep` | 8 | HP per vehicle-repair tick (general) |
| Rules+0x16D0 | double | `RepairPercent` | 0.15 | Cost fraction of full rebuild |
| Rules+0x16D8 | int | `IRepairStep` | ? | HP per infantry-heal tick |
| Rules+0x16E0 | double | `RepairRate` | 0.016 min | Minutes between vehicle repair ticks |
| Rules+0x16E8 | double | **`URepairRate`** | **0.016 min** | Minutes between Unit-in-Repair-Depot ticks (T1 — resolves v2 §25 #9) |
| Rules+0x16F0 | double | **`IRepairRate`** | 0.001 min | Minutes between infantry-heal ticks (Hospital/Armory) |
| Rules+0x16F8 | double | (hardcoded 1.0) | 1.0 | "Full health" threshold |
| Rules+0x1700 | double | `ConditionYellow` | 0.5 | Damage state threshold |
| Rules+0x1708 | double | `ConditionRed` | 0.25 | Damage state threshold |
| Rules+0x1848..+0x184F | double | `[ElevationModel] ElevationBonusCap` | — | Elevation bonus cap (T13 A4 — NOT on BuildingTypeClass; resolves v2 §25 #10) |
| Rules+0x5C8 | int | `[AudioVisual] ShakeScreen` | 671 | Camera-shake magnitude (T13 B17 — T10's hypothesis of "refund divisor" was WRONG) |
| `DAT_007E27F8` | double | (const) | 900.0 | Hospital/Armory/Repair Depot timer multiplier |

---

## 24. Current Rust Implementation Status

### Implemented
- Power system (generation, consumption, low-power, health scaling, spy blackout)
- Repair depot docking (state machine, FIFO queue, credit costs)
- Building placement validation (terrain, overlap, build area, foundation)
- Tech tree and prerequisites (including PrerequisiteOverride)
- Building sell with crew ejection and refunds (50% health-scaled in Rust — **should be non-health-scaled per binary, T13 C1**)
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
- Sell refund should be non-health-scaled (Cost × SellBack + stored_ore). See §17 Mission_Selling. (T13 C1 confirmed.)
- ConYard→Engineer bonus is 25% conditional on Factory==BuildingType AND +0x6E3==0 (not captured) AND **side-independent** — not Soviet-only. (T10/T13 C2.)
- Gap-gen state at +0x220 (NOT +0xBC). See §14. (T13 C3 confirmed.)
- SensorArray add/remove should use same radius field (SensorsSight vs CloakRadiusInCells may mismatch). See §19. (T13 C4 confirmed as bug; use add radius for both in Rust.)
- CrateBeneath should only fire on IC-carryover death. See §6 / T10 §7.
- LeaveRubble= must remain a no-op (parsed only, no rubble tile). See §6 / T10 §6a.

---

## 25. Open Questions (Remaining)

Reduced to 3 residuals after T13's batch closure (21 resolved + 3 dead + 2 partial + 9 deferred with minimum-scope follow-up noted in `BUILDINGCLASS_RESIDUAL_Q_R4.md`).

1. **Save-size puzzle** — `AbstractClass::Save` emits `(4 bytes this_ptr, 6 bytes via vtable[12]=What_Am_I)` = 10 bytes per object on the IStream, but the full 0x720-byte struct is persisted via the OLE Structured Storage docfile substreams (per T8). The per-object raw memcpy shell in `OleSaveToStream`'s plumbing has not been step-traced. T13 B11 resolved the headline puzzle but not the exact bytes-per-stream. Needs live-debug `StgOpenStorage` trace if anyone wants bit-exact save format.

2. **Four construction-overlay SHP slots at Type+0x14E4 / +0x14EC / +0x14FC / +0x1504** — DrawBody construction-branch reads them via a state-byte selector (healthy-buildup / healthy-complete / damaged-buildup / damaged-complete hypothesis), but **no INI key** has been pinned. Stock YR ConYards use BState-driven BuildUp via Type+0xF04 instead, so these four slots likely only fire on non-ConYard construction overlays (gates, SW-specific buildups). ~30-min follow-up scope: trace `BuildingTypeClass::ReadINI @ 0x0045FE50` + `LoadVisualAssets @ 0x0045F230` for `ReadString` sites that store to these offsets.

3. **Abstract-pointer purposes at BuildingClass+0x544 / +0x548 / +0x54C** (T13 B12 partial) — registered with the Load fixup dictionary as Abstract-derived pointers but not read in any major mission handler. Load-time-only-reader sweep (~20 min) should close them. `+0x540` itself was resolved as bridge-damage source; SecretLab's `+0x6F4` was resolved in T13 A3.

### Suggested next research targets

- The three residuals above (all low priority; ≤30 min scope each)
- Finish the 11 HouseClass trait-list offsets (`+0x80/+0x98/+0x110/…`) ↔ BuildingType flag byte map (T13 B16, ~60 min)
- `FUN_00509140` (misnamed "UpdateRadar") full semantics for FactoryClass teardown (T13 B20)

---

## 26. Complete Field Map

**Authoritative source: `BUILDINGTYPECLASS_FIELDS.csv`** — 344 rows covering every Type-level field in BuildingTypeClass (base 0x1798 bytes).

### Row counts by section

| Section | Rows | Notes |
|---|---|---|
| $SELF (rulesmd.ini `[BuildingType]`) | 135 | Gameplay keys |
| $IMAGE (artmd.ini `[Image]`) | 209 | Visual/animation keys |
| **Total** | **344** | |

### YR activity breakdown

| Category | Rows |
|---|---|
| YR-active | 317 |
| TS-legacy / dormant in YR | 23 |
| Conditional (gated behind rare flags) | 4 |

### Notable binary quirks (T2)

- **SuperAnim LOAD/STORE offset mismatch bug** (8 rows): compiled ReadINI writes one byte offset but the cached runtime field is read from a different offset. Harmless (nothing consumes the drifted cache) but parity-surprising.
- **QueueingCell.min uninit bug** (T3): the `short` at `+0x1618` is never initialized; only `.max` at `+0x161A` is written. Benign because `.min` is only compared against `.max` in one internal use.
- **Two-section ReadINI parse pattern**: not previously documented. ReadINI_Water (SELF) and LoadVisualAssets (IMAGE) are distinct passes on different INI source files.
- **`LeaveRubble=` (+0x1768) parsed but never consumed** — zero xrefs outside ReadINI_Water. TS-legacy (T10 §6a).

### Prerequisite storage — NOT on BuildingTypeClass (T11)

Prerequisites live on the inherited **TechnoTypeClass** at `+0x638 Prerequisite=` and `+0x654 PrerequisiteOverride=`. See `BUILDINGCLASS_PREREQUISITES_GHIDRA_REPORT.md` §2.

### Construction-overlay open gap (T13 B6)

`+0x14E4 / +0x14EC / +0x14FC / +0x1504` — four char[24] SHP-name slots, ctor-orphaned (written to empty, no observed INI reader). See §25 #2.

---

## 27. Constructor Defaults Reference

**Authoritative source: `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`** — full decomp of `BuildingTypeClass::constructor @ 0x0045DD90` (1921 bytes).

### Summary

- 176 explicit field writes in ctor body
- 4 loops (PowerUp entry array at +0xF4C..+0x13D0, plus minor tables)
- Of 344 CSV fields, **338 are config-happy** (receive a ctor default AND an ReadINI read) — 6 are ctor-only or reader-only
- BuildingTypeClass vtable confirmed at **`0x007E4570`** (written at ctor step 3). Secondary MI vtables at `0x007E4554`, `0x007E454C`, `0x007E4544`.
- `operator_new(0xC)` at `0x0045e2B5` allocates the per-Type runtime-instance-list at `+0x1788` (12-byte DynamicVector, initial capacity 0)

### Important labeling note (T3)

v2 master mislabeled the ctor as `0x004653C0`. That address is **`BuildingTypeClass::FindOrAllocate`** — it calls `operator_new(0x1798)` then invokes the real ctor at `0x0045DD90` via `CALL 0045dd90`. v3 uses the correct address.

### PowerUp entry format (11 sub-fields)

Each of the 21 slot entries at `Type+0xF4C + N*0x44` has:
- `+0x00` char[24] healthy SHP name
- `+0x10` char[24] damaged SHP name
- `+0x40` byte Powered flag
- `+0x41` byte PoweredLight flag
- `+0x42` byte PoweredEffect flag
- `+0x24` int (offset coord / rate)
- plus smaller bookkeeping fields

See CTOR_DEFAULTS doc for bit-exact byte layout.

---

## 28. Full Vtable (338 Slots)

**Authoritative source: `BUILDINGCLASS_VTABLE_COMPLETE.md`** (renamed from `_FULL_300` and extended with slots 300-337 via T5/T10 reconciliation).

### Summary

| Metric | Value |
|---|---|
| Total slots | **338** |
| Primary function slots (0..321) | 322 |
| Secondary MI vtable markers (322, 330) | 2 |
| Tail continuation pointers (323-329, 331-337) | 14 |
| NULL terminator (slot 338) | 1 |
| Inherited from TechnoClass (addr match) | 199 (in 0..299 range) |
| **Overridden by BuildingClass (0..299)** | **101** |
| Pure-virtual stubs on BuildingClass | **0** — every TC pure-virtual has a concrete override |

### Secondary vtable markers (MI)

Slots 322 (`0x007FC298`) and 330 (`0x007FC390`) are **data-range pointers**, not function pointers. They are the multi-inheritance secondary vtable pointers for COM interfaces (most likely IPersistStream). Everything between and after them (slots 323-329, 331-337) is the MI-side helper method array.

### BuildingTypeClass vtable (NOT BuildingClass)

`BuildingTypeClass` has its own vtable at **`0x007E4570`** (T3 confirmed via ctor decomp). v2 did not cleanly split the two; v3 does.

---

## 29. Save/Load Format

**Authoritative source: `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md`** (T8).

### Summary

- Container: **OLE Structured Storage docfile** (`.SAV`). Uses `StgCreateDocfile`, `StgOpenStorage`, `OleSaveToStream`, `OleLoadFromStream` (OLE imports referenced at strings `0x0081086A`, `0x008108B0`, `0x008108C4`).
- Each top-level object is a separate IStream named by `StringFromCLSID(object_clsid)` prefixed with `AbstractClass::ID` (stored at `this+0x0C`).
- **BuildingClass CLSID:** `{0E272DC6-9C0F-11D1-B709-00A024DDAFD1}` (stored at `0x007E96A0`)

### Per-object persistence model

`AbstractClass::Save` (`0x00410320`) emits only 10 bytes per object to the IStream header (4-byte `old_this` + 6-byte RTTI tag via `vtable[12] = WhatAmI`). The full 0x720-byte struct body is written via a separate raw memcpy dump handled by the outer `OleSaveToStream` plumbing (T13 B11 resolved the v2 puzzle about "vtable[12] returns 6 but struct is 0x720 bytes").

### Pointer rehydration via swap-map (`DAT_00B0C110`)

On Load:
1. Read raw blob into freshly-allocated instance
2. Register `(old_this → new_this)` in the global pointer-fixup dictionary at `DAT_00B0C110`
3. Re-run `BuildingClass::Constructor` on populated memory (re-seats all 4 inherited vtables)
4. Register embedded pointer slots for fixup: `+0x148, +0x149, +0x150, +0x152, +0x153, +0x180, +0x1BD, +0x520, +0x524, +0x540, +0x548, +0x54C, +0x600, +0x6F4`, plus 21 Anims[], 3 Upgrades[], 8 secondary anim slots
5. Explicitly zero `+0x614 LightSourceClass*` (lazy-alloc on next tick)
6. Re-read two DynamicVectors at `+0x66C` (upgrade iter) and `+0x684` (garrison occupants)

After every object is loaded, a single global fixup pass walks the pointer-slot list and rewrites old→new via the swap-map.

### Slot-order correction vs v2

v2 §4 listed slot 5 as "AbstractClass::Save" and slot 6 as "AbstractClass::Load". That is swapped — by MS IPersistStream order, slot 5 = **Load** (`0x00453E20`), slot 6 = **Save** (`0x00454190`). (Also: Ghidra labels on `CaptureManagerClass__Save/Load` are swapped; trust the code, not the label.)

### Unlimbo is NOT on the Load path (T9 clarification)

Post-load rehydration uses the swap-map pointer fixup + lazy-cache rebuild on first tick; `BuildingClass::Unlimbo` is NOT re-invoked. A Rust port should have separate "restore from snapshot" and "place on map" paths.

---

## 30. Tech Tree / Prerequisites

**Authoritative source: `BUILDINGCLASS_PREREQUISITES_GHIDRA_REPORT.md`** (T11).

### Storage (on TechnoTypeClass, inherited by BuildingTypeClass)

- `TechnoTypeClass+0x638` — `Prerequisite=` (DynamicVectorClass<int>, 12 bytes)
- `TechnoTypeClass+0x654` — `PrerequisiteOverride=` (DynamicVectorClass<int>, 12 bytes)

Elements are **signed integers**:
- Non-negative → BuildingType array index (direct reference)
- `-1` POWER, `-2` FACTORY, `-3` BARRACKS, `-4` RADAR, `-5` TECH, `-6` PROC

### Evaluation: `HouseClass::CanBuild @ 0x004F7870`

- AND semantics on `Prerequisite=` (every entry must be satisfied)
- `PrerequisiteOverride=` short-circuits (owning any listed building bypasses AND chain)
- Per-house owned-count array at `HouseClass+0x64` (backed by `IndexClass`) — O(1) CountOwnedInstances lookup
- Returns: `1` (can build), `0` (cannot), `-1` (at BuildLimit but one is queued — sidebar greyed)

### Cross-side prerequisites

**No explicit ALLIED/SOVIET/THIRD keyword.** Cross-side works implicitly because `[General]` lists all three nationality-specific barracks under `PrerequisiteBarracks=` (same for War Factory, etc.). A captured enemy barracks satisfies `BARRACKS`.

### AI shortcut

AI build path skips the full prerequisite loop via a BasePlanNodeArray fast-path (checks only owned-count + tech-level gate).

---

## 31. Rendering Pipeline

**Authoritative source: `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md`** (T7).

### Summary

- `DrawBody @ 0x0043D290` (vtable slot 69, `+0x114`) — SHP pass (2021 bytes)
- `DrawBody_VXL @ 0x0043DA80` (vtable slot 313, `+0x4E4`) — VXL/extras pass. Function boundary created in T7.
- Dispatcher `0x0043CEA0` (vtable slot 65) — called twice per frame for VXL-bodied buildings (pass 0 = SHP, pass 1 = VXL), once otherwise. Gate: `+0x6E7 == 0` on pass 1 (skip VXL for fogged snapshots).

### Z-order within DrawBody (SHP pass)

| Step | What | Guard | Z layer |
|---|---|---|---|
| 1 | Primary body SHP (damaged-state aware) | SHP valid | 2 (normal body) |
| 2 | Damaged-only `BibShape=` SHP | `Type+0x1518 != 0 AND this+0x534 != 0` | 0 (behind body) |
| 3 | Construction-mission overlay (healthy or damaged) | Mission == CONSTRUCTION AND Type+0x14EC / +0x1504 set | 0 |

### Key observations (T7)

- **No body tint on power-down.** Low-power state does NOT recolor the body SHP; it shows via separate anim slots (LowPower = slot 19) rendered above the body.
- **Gate clamp order is load-bearing.** `Type+0x16F8 GateStages=` clamps per `+0x534 DamagedState × +0x6ED gate-frame`; changing the order breaks the closing-gate visual transition.
- **ForceShield (+0x6DF)** renders a different tint than **IronCurtain** — separate code paths. ForceShield uses a cyan palette; IronCurtain uses the classic gold.
- **BuildingLight (+0x600) and LightSource (+0x614) are NOT read in DrawBody.** Ambient light is applied via `TechnoClass::DrawSHP @ 0x00705E00`'s palette-table selection, not per-pixel in DrawBody. (T13 B8 confirmed negative.)
- **BarrelStartPitch** (`Type+0x1710`) is read by `AnimClass::DrawIt` on slot 9 (TurretAnim), NOT by DrawBody itself. (T13 B10.)

### Power-down visual

Power-off DOES render a dark-tinted anim at slot 19 (LowPower overlay), created by `OnPowerOff @ 0x004545D0`. Body itself stays untinted.

---

## Sources

### Functions decompiled/analyzed across rounds (cumulative through T13)

Core:
- `0x0043B740` Constructor | `0x0043BCF0` Destructor | `0x0043FB20` Update
- `0x00440580` Unlimbo | **`0x00445880` Limbo** (renamed from OnDestroyed) | **`0x0044EBF0` Destroy** (renamed from "Limbo")
- **`0x004415F0` DestructionEffects** (newly named, slot 315)
- **`0x00445F80` OnConstructionComplete** (slot 311, newly decomped)
- `0x00442230` ReceiveDamage | `0x00442D90` SpawnSurvivors | `0x00451EE0` SetDamagedState
- `0x0043CEA0` Draw dispatcher | `0x0043D290` DrawBody | **`0x0043DA80` DrawBody_VXL** (slot 313, new boundary)
- **`0x004509D0` UpdateAnimation** (T6 full decomp)

Mission handlers:
- `0x0044ACF0` Mission_Attack | **`0x0044B760` Mission_Guard** (T12) | **`0x00449A50` Mission_Construction** (T12) | `0x00449C30` Mission_Selling
- `0x0044B780` Mission_RepairAndProduce | `0x0044C980` Mission_Missile | `0x0044D880` Mission_Hunt (slot 143)

Save/Load (T8):
- **`0x00453E20` Load** (slot 5) | **`0x00454190` Save** (slot 6) | **`0x00454260` Save_ChecksumFields** (slot 13)
- `0x00410320` AbstractClass::Save | `0x00410380` AbstractClass::Load
- `0x0070BF50` TechnoClass::Load_IStream | `0x0070C250` TechnoClass::Save_IStream
- `0x0065AC40` ObjectClass::Save_IStream | `0x005F5E80` ObjectClass::Load_IStream

Vtable identity (T5):
- `0x00459E80` GetClassID (slot 3) | `0x00459EC0` WhatAmI (slot 11) | `0x00459E70` SizeOf (slot 12)
- `0x00459F20` ScalarDeletingDestructor (slot 8) | `0x00442C40` Init_Managers (slot 9)
- `0x00452630` IsDeployable (slot 37)

Specialized:
- `0x00443C60` ExitObject_Main (6724 bytes) | `0x00449540` ClearBibArea
- `0x00447B20` GetDockCoord | **`0x0044EFB0` GetDockCellForObject** (slot 309)
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

Prerequisites (T11):
- **`0x004F7870` HouseClass::CanBuild** | `0x00459840` SecretLab CanBuild helper
- `0x00712170` TechnoTypeClass::ReadINI (Prerequisite= parser at +0x638)

INI parsing:
- `0x0045FE50` BuildingTypeClass::ReadINI | **`0x0045DD90` BuildingTypeClass ctor** (correct) | **`0x004653C0` BuildingTypeClass::FindOrAllocate** (was wrongly cited as ctor in v2)
- `0x0066D530` RulesClass::ReadGeneral | `0x006691E0` RulesClass::ReadAudioVisual

AI queue:
- `0x004FDD10` HouseClass::AI_Manage_Build_Queue | `0x00506EF0` HouseClass::AI_ChooseNextProduction
- `0x0050A490` CleanBasePlanForLostBuilding (OnBuildingDestroyed hook, misnamed FUN_)
- `0x00509140` RevalidateFactoryQueueForKind (misnamed "UpdateRadar")

Helpers:
- `0x0065ADC0` RadioClass::HasFreeSlot | `0x004060F0` SoundEvent::SetLoopHandle
- `0x004500F0` Cloning Vats auto-produce
- `0x004D0EF0` CreateFoggedSnapshot (TS-legacy FoW, +0x6E7 writer)
- `0x00707D20` TechnoClass::Crew_Type

Vtable reads: BuildingClass vtable at `0x007E3EBC` — **338 slots** read to NULL terminator. BuildingTypeClass vtable at `0x007E4570`.

String lookups: "HasSpotlight" @ 0x81AEA0, "URepairRate" @ 0x83BDC4,
"IRepairStep" @ 0x83BDDC, "IRepairRate" @ 0x83BDB8, "RepairRate" @ 0x83BDD0,
"RepairStep" @ 0x83BDE8, "RepairPercent" @ 0x83BDF4, "AIBaseSpacing" (T13 B13).

CLSIDs:
- BuildingClass: `{0E272DC6-9C0F-11D1-B709-00A024DDAFD1}` at `0x007E96A0`
- DriveLocomotion: `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}` at `0x007E9AB0`

Jump tables: Mission_Attack GetFireError table at `0x0044B728` (11 DWORDs).

Ghidra renames applied this pass (T5/T10 reconciliation):
- `BuildingClass__OnDestroyed @ 0x00445880` → **`BuildingClass__Limbo`**
- `FUN_004415f0` → **`BuildingClass__DestructionEffects`**
- (Earlier renames preserved: T5 renamed `0x0044EBF0 "Limbo"` → `BuildingClass__Destroy`; slots 3/8/9/11/12/13/37 named via GetClassID/WhatAmI/SizeOf/etc.)

### INI files checked

- `ini/rulesmd.ini` — confirmed RepairPercent=15%, RepairRate=.016, RepairStep=8, IRepairRate=.001, **URepairRate=.016**, RepairDelay=.02/.05, AIBaseSpacing=1000
- `ini/artmd.ini` — referenced for PowerUp art entries and BibShape
