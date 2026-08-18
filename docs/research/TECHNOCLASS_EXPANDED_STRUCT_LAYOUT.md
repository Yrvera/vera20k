# TechnoClass Expanded Struct Layout — Ghidra Research Report

**Functions decompiled (30+):** Constructor (0x006f2b40), AI_Update (0x006f9e50, all 625 lines),
Save (0x0070c270), Init_Managers (0x006f3f40), ReceiveDamage (0x00701900, 682 lines),
CloakingTick (0x006fb740), CanAutoCloak (0x006fbdc0), SelectWeaponAgainst (0x006f3330),
GetFireError (0x006fc0b0, 412 lines), ApplyTemporalDamage (0x0070e000),
ScaleByTemporalVisualPhase (0x0070e380), ModifyCloakDrawFlags (0x0070ed80),
UpdateGapVisual (0x0070e920), UpdateReveal (0x0070af50),
Retaliate_And_Scan (0x00709820), RecordKill (0x00702d40), OnDeployBegin (0x0070fc90),
OnUndeployComplete (0x0070fbe0), StopAllTargeting (0x0070d4a0),
HasStealthAbility (0x0070c5a0), IsUnderEMP (0x0070efd0), FreeAllMindControlCaptures (0x00710460),
Passive_Target_Acquire (0x00709480), Fire_At (0x006fdd50, all 919 lines),
Set_ArchiveTarget (0x006fcdb0), GetFLH (0x006f3ad0), Draw (0x00706640),
Receive_Radio (0x006f4ab0), PerformDeploy (0x00710000), What_Action_OnObject (0x006ffec0),
StartCloaking (0x00703770), StartUncloaking (0x007036c0), DoUncloak (0x006f4eb0),
ShouldUncloak (0x006fbc90), GetVisualState (0x00703860), Calculate_Threat_Score (0x0070cd10),
ProcessCloakMode (0x005f5850), IdleActionTick (0x0070ccf0),
StorageClass__Constructor (0x006c95e0), StorageClass__GetTotalAsInt (0x006c9600),
FacingClass constructors (0x004c91e0, 0x004c91c0)

**Confidence:** High (struct verified from Ghidra struct definition + 20+ decompiled functions)
**Active in YR:** Yes — all fields documented here are used in standard YR skirmish
**Research date:** 2026-04-01

## 1. Overview

TechnoClass is the core gameplay class for all interactive game objects in gamemd.exe.
It inherits from RadioClass (which inherits from MissionClass → ObjectClass → AbstractClass).
All units, infantry, aircraft, and buildings derive from TechnoClass (via FootClass for mobile
units, directly for BuildingClass).

**Total struct size:** 1312 bytes (0x520) — confirmed from Ghidra struct definition.
Subclasses extend beyond 0x520 (FootClass adds fields from ~0x520, BuildingClass much larger).

## 2. Inheritance Chain

```
AbstractClass (0x00-0x24)
  → ObjectClass (0x24-0xAB)
    → MissionClass (adds mission state)
      → RadioClass (adds radio communication)
        → TechnoClass (0xF0-0x51F, own fields)
          → FootClass → UnitClass / InfantryClass / AircraftClass
          → BuildingClass
```

## 3. Ghidra Struct Fields (Authoritative — from Ghidra's Type Manager)

These offsets come directly from Ghidra's struct definition for TechnoClass and are definitive.

| Byte Offset | Dec | Type | Ghidra Name | Confidence |
|-------------|-----|------|-------------|------------|
| 0x000 | 0 | ptr | vtable | HIGH |
| 0x06C | 108 | int | Health | HIGH |
| 0x08C | 140 | bool | OnBridge | HIGH |
| 0x090 | 144 | bool | IsAlive | HIGH |
| 0x09C | 156 | int | Location_X | HIGH |
| 0x0A0 | 160 | int | Location_Y | HIGH |
| 0x0A4 | 164 | int | Location_Z | HIGH |
| 0x138 | 312 | int | CurrentWeaponNumber | HIGH |
| 0x140 | 320 | int | CurrentGattlingStage | HIGH |
| 0x144 | 324 | int | GattlingValue | HIGH |
| 0x150 | 336 | int | Veterancy | HIGH |
| 0x158 | 344 | double | ArmorMultiplier | HIGH |
| 0x160 | 352 | double | FirepowerMultiplier | HIGH |
| 0x21C | 540 | ptr | Owner (HouseClass*) | HIGH |
| 0x220 | 544 | int | CloakState | HIGH |
| 0x254 | 596 | int | LastSightCoords_X | HIGH |
| 0x258 | 600 | int | LastSightCoords_Y | HIGH |
| 0x25C | 604 | int | LastSightCoords_Z | HIGH |
| 0x288 | 648 | int | ChronoDestCoords_X | HIGH |
| 0x28C | 652 | int | ChronoDestCoords_Y | HIGH |
| 0x290 | 656 | int | ChronoDestCoords_Z | HIGH |
| 0x2AC | 684 | ptr | LocomotorTarget | HIGH |
| 0x2B4 | 692 | ptr | Target (ArchiveTarget) | HIGH |
| 0x2BC | 700 | ptr | CaptureManager | HIGH |
| 0x2C0 | 704 | ptr | MindControlledBy | HIGH |
| 0x2FC | 764 | int | Ammo | HIGH |
| 0x324 | 804 | ptr | Wave (RadBeam/Sonic) | HIGH |
| 0x328 | 808 | float | AngleRotatedSideways | HIGH |
| 0x32C | 812 | float | AngleRotatedForwards | HIGH |
| 0x330 | 816 | float | RockingSidewaysPerFrame | HIGH |
| 0x334 | 820 | float | RockingForwardsPerFrame | HIGH |
| 0x3B8 | 952 | int | CurrentBurstIndex | HIGH |
| 0x3CD | 973 | bool | IsSinking | HIGH |
| 0x3D2 | 978 | bool | Cloakable | HIGH |
| 0x3D3 | 979 | bool | IsPrimaryFactory | HIGH |
| 0x504 | 1284 | int | EMPLockRemaining | HIGH |
| 0x518 | 1304 | ptr | Disguise (TechnoTypeClass*) | HIGH |
| 0x51C | 1308 | ptr | DisguisedAsHouse (HouseClass*) | HIGH |

## 4. Corrections to Previous Report (TECHNOCLASS_STRUCT_LAYOUT.md)

The following fields were incorrectly identified in the prior report:

| Offset | Old Name | Correct Name | Evidence |
|--------|----------|--------------|----------|
| 0x138 | ForcedWeaponIndex | **CurrentWeaponNumber** | Ghidra struct + SelectWeaponAgainst: returned for gattling units when != -1 |
| 0x140 | CurrentWeaponIndex | **CurrentGattlingStage** | Ghidra struct + SelectWeaponAgainst: `iVar4 * 2` for weapon pair index |
| 0x144 | Unknown (init 0) | **GattlingValue** | Ghidra struct + ApplyTemporalDamage: accumulated gattling charge, also used as temporal HP |
| 0x150 | Unknown (embedded object) | **Veterancy** (int) | Ghidra struct + Volume__IsNormal reads float at this address |
| 0x158-0x15F | float/double (init 1.0) | **ArmorMultiplier** (double) | Ghidra struct, init 0x3FF00000 = 1.0 high dword |
| 0x160-0x167 | float/double (init 1.0) | **FirepowerMultiplier** (double) | Ghidra struct, init 0x3FF00000 = 1.0 high dword |
| 0x254-0x25C | ChronoDestX/Y/Z | **LastSightCoords_X/Y/Z** | Ghidra struct + UpdateReveal writes current position here for sight tracking |
| 0x2AC | IsMoving or similar | **LocomotorTarget** (ptr) | Ghidra struct + GetFireError compares against target |
| 0x2FC | Unknown (init -1) | **Ammo** (int) | Ghidra struct + ReceiveDamage modifies for IFV ammo |
| 0x3CD | IsRocking flag | **IsSinking** (bool) | Ghidra struct + GetFireError prevents firing when set |

## 5. Newly Identified Fields (from decompilation cross-reference)

### 5a. Timer Fields (CDTimerClass — 12 bytes each: StartFrame, mid, Duration)

CDTimerClass pattern: `StartFrame` at +0 (int, -1 = inactive), unknown at +4, `Duration` at +8.
Remaining time = max(0, Duration - (g_CurrentFrameCounter - StartFrame)).

| Start Offset | Duration Offset | Name | Init Duration | Evidence | Confidence |
|-------------|-----------------|------|---------------|----------|------------|
| 0x168 | 0x170 | **GenericTimer1** | 0 | Save function serializes remaining time | MED |
| 0x174 | 0x17C | **DamageResponseTimer** | 0 | ReceiveDamage: 0x174=frame, 0x178=distance, 0x17C=RetaliationDelay from rules | HIGH |
| 0x180 | 0x188 | **RetaliationScanTimer** | 45 (0x2D) | Retaliate_And_Scan: set with randomized delay from RulesClass+0xE04/0xE08 | HIGH |
| 0x18C | 0x194 | **IronCurtainTimer** | 0 | ReceiveDamage checks invulnerability via vtable+0x160 | HIGH |
| 0x198 | 0x1A0 | **TemporalVisualTimer** | 0 | ScaleByTemporalVisualPhase: reads 0x198/0x1A0, switch on 0x1A4 | HIGH |
| 0x1A8 | 0x1B0 | **IdleActionTimer** | 0 | AI_Update: timing for idle animations | MED |
| 0x1B4 | 0x1BC | **GapGeneratorTimer** | 0 | UpdateGapVisual: state machine uses this for phase transitions | HIGH |
| 0x1E0 | 0x1E8 | **DisguiseFleeTimer** | 0 | ReceiveDamage: set when CanDisguise && !PermaDisguise unit takes damage | HIGH |
| 0x1EC | 0x1F4 | **CloakDetectTimer** | 0 | ModifyCloakDrawFlags: controls shimmer visibility for enemies | HIGH |
| 0x1FC | 0x204 | **CloakDelayTimer** | 0 | Save function + CanAutoCloak: must expire before re-cloaking | HIGH |
| 0x22C | 0x234 | **CloakTimer** | 0 | StartCloaking: StartFrame=g_CurrentFrameCounter (param_1[0x8b]×4=0x22C), Duration=TechnoTypeClass+0x310 (CloakingSpeed) (param_1[0x8d]×4=0x234). Times the cloak *animation* progression, not state machine transitions. corrected 2026-05-28: was "CloakStateTimer"; Section A1 (deeper analysis) correctly names it CloakTimer — ROOT_CAUSE: INFERENCE_HARDENED | HIGH |
| 0x240 | 0x248 | **CloakCooldownTimer** | 0 | CanAutoCloak: must expire before auto-cloaking starts | HIGH |
| 0x2EC | 0x2F4 | **FireRateTimer** | 0 | Fire_At: weapon ROF countdown | HIGH |
| 0x3BC | 0x3C4 | **MuzzleFlashTimer** | 0 | Fire_At: trail/muzzle flash duration | HIGH |
| 0x4FC | — | **LastScanFrame** (single int) | g_CurrentFrame | Passive_Target_Acquire: last frame target scan ran | HIGH |

### 5b. State / Counter Fields

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| 0x070 | int | **VisualHealth** | (from Health) | AI_Update: slowly catches up to actual Health for health bar animation | HIGH |
| 0x0F0 | int | **ThreatCategory** | 0 | AI_Update: set from RulesClass+0xBE8 on volume change | MED |
| 0x0F8 | int | **AnimFrameAccum** | 0 | AI_Update: animation accumulator, incremented by 0x110 | MED |
| 0x0FC | byte | **AnimActive** | 0 | AI_Update: set 1 when animation timer fires, 0 when stopped | MED |
| 0x100 | int | **AnimTimer.Start** | g_CurrentFrame | AI_Update: animation rate timer start | MED |
| 0x108 | int | **AnimTimer.Duration** | 0 | AI_Update: animation rate timer duration | MED |
| 0x10C | int | **AnimRateValue** | 0 | AI_Update: if non-zero, triggers animation advance | MED |
| 0x110 | int | **AnimStepSize** | 1 | AI_Update: added to 0xF8 each animation tick | MED |
| 0x120 | int | **LastFireFrame** | -100 (0xFFFFFF9C) | Constructor: intentionally old frame = "never fired" | HIGH |
| 0x124 | int | **TurretAnimFrame** | -1 | AI_Update: computed from fire timer ratio for turret barrel anim | HIGH |
| 0x13C | int | **DamageVolumeCategory** | -1 | AI_Update: tracks volume/damage category, triggers EVA when changes | HIGH |
| 0x178 | int | **LastDamageDistance** | 0 | ReceiveDamage: distance from last damage source | HIGH |
| 0x1A4 | int | **TemporalVisualPhase** | 10 | ScaleByTemporalVisualPhase: switch 1-9 for warp effects, 10=none | HIGH |
| 0x1C0 | int | **GapVisualState** | 0 | UpdateGapVisual: state machine (0=off, 1-4=animation phases) | HIGH |
| 0x1C4 | int | **IronCurtainType** | 0 | ReceiveDamage: 0=IronCurtain, 1=ForceShield (different spark anim) | HIGH |
| 0x1DC | int | **CloakShimmerFrame** | 0 | ModifyCloakDrawFlags: frame offset for visual shimmer cycling | HIGH |
| 0x218 | int | **WarpState** | 0 | SetGhostCell (0x0070c610): visual warp state for chrono rendering | HIGH |
| 0x250 | byte | **HasInitialReveal** | 0 | UpdateReveal: set 1 after first sight reveal calculation | HIGH |
| 0x260 | int | **LastSightRange** | 0 | UpdateReveal: sight range used for last reveal | HIGH |
| 0x298 | byte | **IsMindControlVictim** | 0 | ReceiveDamage: set 1 when MC warhead hits, triggers MC behavior | HIGH |
| 0x29C | int | **MindControlCountdown** | 0 | AI_Update: decremented each tick, clears MC on reaching 0 | HIGH |
| 0x2A0 | int | **SpreadAttackIndex** | 0 | Fire_At: cycled 0-7 for spread weapons | HIGH |
| 0x2E4 | int | **GarrisonBuildingPtr** | 0 | ReceiveDamage/GetFireError: building this unit is garrisoned in | HIGH |
| 0x300 | int | **FireAnimCount** | 0 | Fire_At/AI_Update: related to weapon fire animation management | MED |
| 0x420 | byte | **LastHeightTier** | 0 | UpdateReveal: last Z-tier for sight recalculation on elevation change | MED |
| 0x431 | byte | **WasJustAttacked** | 0 | AI_Update: cleared to 0 at start of each AI tick | MED |
| 0x432 | byte | **SuicideFlag** | 0 | Fire_At: set 1 after self-destruct weapon fires | MED |
| 0x43C | int | **BarrelRotationIndex** | 0 | Fire_At: cycled for multi-barrel weapons | HIGH |
| 0x50C | byte | **IsNewTarget** | 0 | Passive_Target_Acquire: set 1 when target changes | HIGH |

### 5c. Pointer Fields (Manager Classes & Links)

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| 0x11C | ptr | **LinkedTechno** | vtable ptr | GetFireError: can't fire at this linked entity | MED |
| 0x12C | ptr | **AttachedBeamAnim** | 0 | AI_Update: destroyed under certain conditions | MED |
| 0x14C | ptr | **TechnoTypeClass*** | param | Constructor: set from constructor argument | HIGH |
| 0x1CC | ptr | **DrainTarget** | 0 | AI_Update: money transfer link — unit being drained/controlled | HIGH |
| 0x1D0 | ptr | **DrainedBy** | 0 | AI_Update: bidirectional partner of 0x1CC | HIGH |
| 0x1D4 | ptr | **DrainAnim** | 0 | AI_Update/ReceiveDamage: visual anim for drain link | HIGH |
| 0x274 | ptr | **TemporalClass*** | 0 | Init_Managers: created when weapon has Temporal=yes | HIGH |
| 0x294 | ptr | **AirstrikeClass*** | 0 | Init_Managers: created when TechnoType has AirstrikeTeam | HIGH |
| 0x2BC | ptr | **CaptureManagerClass*** | 0 | Init_Managers: created when weapon has MindControl=yes | HIGH |
| 0x2C0 | ptr | **MindControlledBy** | 0 | Ghidra struct: pointer to controlling TechnoClass | HIGH |
| 0x2C4 | byte | **IsMindControlled** | 0 | MIND_CONTROL report: flag set on capture | HIGH |
| 0x2C8 | ptr | **MCRingAnim** | 0 | MIND_CONTROL report: ring visual on victim | HIGH |
| 0x2D0 | ptr | **SpawnManagerClass*** | 0 | Init_Managers: created when TechnoType has SpawnsNumber | HIGH |
| 0x2D8 | ptr | **SlaveManagerClass*** | 0 | Init_Managers: created when TechnoType has Enslaves | HIGH |
| 0x304 | ptr | **TemporalTargetAnim** | 0 | ReceiveDamage: destroyed on death | MED |
| 0x308 | ptr | **DamageParticleSystem1** | 0 | AI_Update: created for damage smoke/fire particles | HIGH |
| 0x310 | ptr | **DamageParticleSystem2** | 0 | AI_Update: destroyed when health recovers above threshold | HIGH |

### 5d. Byte Flag Block (0x3CE-0x3D5)

These bytes are serialized by the Save function (0x0070c270) and represent persistent state flags:

| Offset | Name | Init | Evidence | Confidence |
|--------|------|------|----------|------------|
| 0x3CD | IsSinking | 0 | Ghidra struct | HIGH |
| 0x3CE | Unknown | 0 | Constructor only | LOW |
| 0x3CF | Repairable | 0 | RECEIVE_DAMAGE report: AI defense flag | MED |
| 0x3D0 | IsInOre | 0 | Constructor only, saved | LOW |
| 0x3D1 | WasAttacked | 0 | RECEIVE_DAMAGE report: damage tracking flag | MED |
| 0x3D2 | Cloakable | 0 | Ghidra struct + HasStealthAbility getter (0x0070c5a0) | HIGH |
| 0x3D3 | IsPrimaryFactory | 0 | Ghidra struct | HIGH |
| 0x3D4 | Unknown | 0 | Constructor only, saved | LOW |
| 0x3D5 | HasSight | 0 | UpdateReveal: gates all sight reveal processing | HIGH |

### 5e. Byte Flag Block (0x418-0x427)

| Offset | Name | Init | Evidence | Confidence |
|--------|------|------|----------|------------|
| 0x418 | CanCloak | 0 | CloakingTick: gates cloaking behavior | HIGH |
| 0x419 | Unknown | 0 | Constructor only, saved | LOW |
| 0x41A | IsControlledByPlayer | 0 | Init_Managers: set 1 if Owner == g_PlayerPtr | HIGH |
| 0x41B | IsControlledByHuman | 0 | Select: checked for selection permission | HIGH |
| 0x41C | Unknown | 0 | Save: conditionally saved for single-player | LOW |
| 0x41D | Unknown | 0 | Constructor, saved | LOW |
| 0x41E | FlashCountdown | 0 | AI_Update: decremented each tick when non-zero | MED |
| 0x41F | Unknown | 0 | Constructor, saved as int | LOW |
| 0x420 | LastHeightTier | 0 | UpdateReveal: Z-tier for sight | MED |
| 0x421 | CanRock | 1 | RockingUpdate: enables rocking | HIGH |
| 0x422 | Unknown | 1 | Constructor | LOW |
| 0x425 | IsShipRocking | 0 | RockingUpdate: ship-type velocity-based rocking | HIGH |

### 5f. Weapon Burst Rate Data (0x3D8-0x414)

Two parallel 28-byte structures for primary and secondary weapon burst timing.
Initialized from TechnoTypeClass fields in Init_Managers.

**Primary Weapon (0x3D8-0x3F4):**

| Offset | Name | TechnoType Source | Init | Confidence |
|--------|------|-------------------|------|------------|
| 0x3D8 | PrimaryBurst.Total | +0xCA4 | 2 | HIGH |
| 0x3DC | PrimaryBurst.Count | +0xCA8 | 1 | HIGH |
| 0x3E0 | PrimaryBurst.Current | +0xCAC | 1 | HIGH |
| 0x3E4 | PrimaryBurst.Divisor | +0xCB0 | 1 | HIGH |
| 0x3E8 | PrimaryBurst.Ratio | computed | 0 | HIGH |
| 0x3EC | PrimaryBurst.Unknown | — | 0 | LOW |
| 0x3F0 | PrimaryBurst.Active | — | 0 | HIGH |
| 0x3F4 | PrimaryBurst.MaxDivisor | — | 0 | HIGH |

**Secondary Weapon (0x3F8-0x414):**

| Offset | Name | TechnoType Source | Init | Confidence |
|--------|------|-------------------|------|------------|
| 0x3F8 | SecondaryBurst.Total | +0xCB8 | 2 | HIGH |
| 0x3FC | SecondaryBurst.Count | +0xCBC | 1 | HIGH |
| 0x400 | SecondaryBurst.Current | +0xCC0 | 1 | HIGH |
| 0x404 | SecondaryBurst.Divisor | +0xCC4 | 1 | HIGH |
| 0x408 | SecondaryBurst.Ratio | computed | 0 | HIGH |
| 0x40C | SecondaryBurst.Unknown | — | 0 | LOW |
| 0x410 | SecondaryBurst.Active | — | 0 | HIGH |
| 0x414 | SecondaryBurst.MaxDivisor | — | 0 | HIGH |

### 5g. Embedded Objects (DynamicVectorClass instances)

Three embedded DynamicVectorClass instances at:
- **0x440** (vtable PTR_FUN_007e4e78): FlashTimer or similar — size ~24 bytes, Capacity init 10
- **0x458** (vtable PTR_FUN_007e91ec): Target tracking list 1 — size ~24 bytes, Capacity init 10
- **0x470** (vtable PTR_FUN_007e91ec): Target tracking list 2 — size ~24 bytes, Capacity init 10

Related serialized fields:
- 0x444: Data pointer for first list
- 0x45C: Associated object pointer array
- 0x468: Count for first list
- 0x474: Data pointer for second list
- 0x480: Count for second list

### 5h. Disguise System (from Init_Managers)

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| 0x1D8 | byte | IsPermaDisguised | 0 | Init_Managers: set 1 for CanDisguise+PermaDisguise types | HIGH |
| 0x518 | ptr | Disguise | 0 | Ghidra struct + Init_Managers: TechnoTypeClass* to appear as | HIGH |
| 0x51C | ptr | DisguisedAsHouse | 0 | Ghidra struct + Init_Managers: HouseClass* owner of disguise | HIGH |

### 5i. Chrono/Warp Fields (verified from TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md)

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| 0x218 | int | WarpState | 0 | SetGhostCell writes here | HIGH |
| 0x254 | int | LastSightCoords_X | NullCoord | Ghidra struct (NOT ChronoSource!) | HIGH |
| 0x258 | int | LastSightCoords_Y | NullCoord | Ghidra struct | HIGH |
| 0x25C | int | LastSightCoords_Z | NullCoord | Ghidra struct | HIGH |
| 0x268 | byte | Unknown chrono flag | 0 | Constructor | LOW |
| 0x269 | byte | Unknown chrono flag | 0 | Constructor | LOW |
| 0x270 | byte | WarpingOut | 0 | IsWarpingOut (0x0070c5b0) | HIGH |
| 0x271 | byte | BeingWarped | 0 | IsBeingWarped (0x0070c5c0) | HIGH |
| 0x272 | byte | Unknown | 0 | Constructor | LOW |
| 0x27C | byte | ChronoInTransit | 0 | Constructor comment | HIGH |
| 0x280 | int | PendingWarpPhase | 0 | Constructor comment: set to 3 by ChronoSphere handler | HIGH |
| 0x284 | int | ChronoLockDuration | 0 | Constructor comment | HIGH |
| 0x288 | int | ChronoDestCoords_X | NullCoord | Ghidra struct | HIGH |
| 0x28C | int | ChronoDestCoords_Y | NullCoord | Ghidra struct | HIGH |
| 0x290 | int | ChronoDestCoords_Z | NullCoord | Ghidra struct | HIGH |

## 6. Veterancy System

The veterancy float lives at offset +0x150 (336 dec). The Ghidra struct labels it `int` but
Volume__IsNormal (0x0074ff90) and FUN_00750010 read it as `float*`:

```
Volume__IsNormal: returns (threshold1 <= *this_float) && (*this_float < threshold2)
FUN_00750010:     returns (elite_threshold <= *this_float)
```

**Thresholds** (from globals):
- Veteran: `_DAT_007e2ac8` <= value < `_DAT_007e37b4`
- Elite: value >= `_DAT_007e37b4`

The float is accumulated by RecordKill (0x00702d40) and scaled by TechnoTypeClass Cost.
HasWeaponAbility (0x0070d0d0) checks VeteranAbilities at TechnoTypeClass+0x29C+index
and EliteAbilities at +0x2AE+index.

**Active in YR:** Yes, core gameplay mechanic.

## 7. Cloaking System

**Fields:**
- 0x220 CloakState: enum (0=Uncloaked, 1=Cloaking, 2=Cloaked, 3=Uncloaking)
- 0x224 CloakProgress: animation counter
- 0x3D2 Cloakable: bool (runtime flag, can be granted by veteran ability)
- 0x418 CanCloak: bool (another cloaking permission flag)
- 0x1DC CloakShimmerFrame: frame offset for visual cycling
- 0x1EC/0x1F4 CloakDetectTimer: CDTimerClass — shimmer visibility for enemies
- 0x240/0x248 CloakCooldownTimer: CDTimerClass — must expire before auto-cloak

**Key functions:**
- CloakingTick (0x006fb740): state machine for cloak/uncloak transitions
- CanAutoCloak (0x006fbdc0): permission check — requires no fire timer, no target, cooldown expired
- ModifyCloakDrawFlags (0x0070ed80): visual shimmer effect using frame cycling

**Active in YR:** Yes, used by Mirage Tank, Spy, cloaked units.

## 8. Iron Curtain / Force Shield

**Fields:**
- 0x18C/0x194 IronCurtainTimer: CDTimerClass — duration of invulnerability
- 0x1C4 IronCurtainType: int (0=IronCurtain, 1=ForceShield — determines spark anim type)

**Logic in ReceiveDamage:**
```
if (IsInvulnerable()) {  // vtable+0x160
    if (IronCurtainType == 1) spark = 6;  // ForceShield spark
    else spark = 1;                        // IronCurtain spark
    PlaySparkAnim(Location, spark);
    *damage = 0;
    return 0;  // No damage taken
}
```

**Active in YR:** Yes, IronCurtain superweapon and ForceShield (Yuri faction).

## 9. EMP System

**Fields:**
- 0x504 EMPLockRemaining: int — countdown timer, decremented 1 per tick in AI_Update

**IsUnderEMP (0x0070efd0):** `return 0 < this->EMPLockRemaining;`

When EMP expires (reaches 0):
- Buildings: calls RestoreOnlineEffects
- Mobile units: restarts locomotor, clears EMP anim

**Active in YR:** Yes, Robot Tank EMP weapon, Lightning Storm side effect.

## 10. Mind Control System

Two parallel systems:

**CaptureManagerClass (weapon-level, multi-target):**
- 0x2BC CaptureManager: ptr — manages all units captured by THIS unit's MC weapon
- 0x2C0 MindControlledBy: ptr — who captured THIS unit
- 0x2C4 IsMindControlled: byte flag
- 0x2C8 MCRingAnim: ptr — ring visual on victim

**Direct Link (instance-level, bidirectional):**
- 0x1CC DrainTarget: ptr — the unit linked to by this one (money drain / upkeep)
- 0x1D0 DrainedBy: ptr — the unit linking to this one
- 0x1D4 DrainAnim: ptr — visual for the link

The direct link pair handles periodic money transfer (AI_Update checks TechnoTypeClass+0x5ED
flag, transfers credits at RulesClass+0x314 interval).

**Active in YR:** Yes, Yuri Prime, Yuri Clone, Psychic Dominator.

## 11. Gattling Weapon System

**Fields:**
- 0x140 CurrentGattlingStage: int — current weapon tier (0-based)
- 0x144 GattlingValue: int — accumulated charge, compared against thresholds

**In SelectWeaponAgainst:**
```
if (IsGattling) {  // TechnoTypeClass+0xCD5
    stage = this->CurrentGattlingStage;
    if (TargetIsAirborne && WeaponHasAA) return stage * 2 + 1;
    return stage * 2;
}
```

The gattling system uses paired weapons: stage N uses weapon N*2 (ground) or N*2+1 (air).

**Dual use in ApplyTemporalDamage:** GattlingValue at 0x144 is also decremented by temporal
weapon damage, and CurrentGattlingStage at 0x140 tracks the "erasure phase" when both reach 0
the unit is fully erased. This is a shared field — temporal damage resets gattling state.

**Active in YR:** Yes, Gattling Tank, Gattling Cannon.

## 12. Current Rust Implementation Status

**Implemented:**
- Health, Owner, Location, Facing, Veterancy (as u16 levels, not float XP)
- Passengers/Transport, Aircraft Ammo, Slave Miner
- Movement (locomotor, teleport, tunnel, rocket, droppod, drive tracks)
- Combat targeting (attack_target, last_attacker_id)
- Turret facing, building construction/deconstruction
- Garrison system with fire index

**NOT implemented (from Ghidra findings):**
- Cloaking state machine and all cloak fields
- Iron Curtain / Force Shield timer and invulnerability
- EMP system (EMPLockRemaining timer)
- Mind Control (CaptureManager, direct links, ownership transfer)
- Gattling weapon stage/value
- Temporal weapon damage/erasure
- Drain link system (0x1CC/0x1D0)
- Disguise system
- Gap generator visual state machine
- Armor/Firepower multipliers (doubles at 0x158/0x160)
- Damage particle systems
- Spread attack cycling
- Ship/vehicle rocking physics
- Visual health bar animation smoothing

## 13. Open Questions

1. **0x168 timer:** Purpose unknown. Serialized by Save but not clearly used in decompiled functions.
   Could be a generic rearm or reload timer.

2. **0x1A8 timer:** Likely related to idle actions or fidget animations but not confirmed.

3. **0x11C pointer:** Some kind of linked entity. GetFireError prevents firing at it. Could be
   a docking partner or radio contact. Needs trace from RadioClass.

4. **0x12C pointer:** Destroyed in AI_Update under conditions related to a building check +
   IsGarrisonable flag. Possibly a garrison-related beam or anim.

5. **0x1CC/0x1D0 precise semantics:** Labeled as "drain" based on money transfer code, but
   could also be a more general "link" system. The exact relationship to CaptureManagerClass
   (0x2BC) needs clarification — are these separate MC implementations or different aspects
   of the same system?

6. **Byte flags 0x3CE, 0x3D0, 0x3D4, 0x419, 0x41C, 0x41D, 0x41F, 0x422:** Still
   unidentified. All initialized to 0, all serialized by Save.

7. **0x4F0/0x4F4:** Initialized to -1. Used in AI_Update for sound playback — possibly
   VoiceIndex or SoundEvent queue. Set to -1 after played.

8. **Veterancy type ambiguity:** Ghidra struct says `int` at 0x150 but Volume__IsNormal
   reads it as `float`. The field is likely a `float` that Ghidra mis-typed as `int`
   due to C++ union-like usage (comparing float thresholds but sometimes cast to int for XP math).

## Sources

**Ghidra functions decompiled (20+):**
TechnoClass__Constructor (0x006f2b40), TechnoClass__AI_Update (0x006f9e50),
TechnoClass__Save (0x0070c270), TechnoClass__Init_Managers (0x006f3f40),
TechnoClass__ReceiveDamage (0x00701900), TechnoClass__CloakingTick (0x006fb740),
TechnoClass__CanAutoCloak (0x006fbdc0), TechnoClass__SelectWeaponAgainst (0x006f3330),
TechnoClass__GetFireError (0x006fc0b0), TechnoClass__ApplyTemporalDamage (0x0070e000),
TechnoClass__ScaleByTemporalVisualPhase (0x0070e380), TechnoClass__ModifyCloakDrawFlags (0x0070ed80),
TechnoClass__UpdateGapVisual (0x0070e920), TechnoClass__UpdateReveal (0x0070af50),
TechnoClass__Retaliate_And_Scan (0x00709820), TechnoClass__RecordKill (0x00702d40),
TechnoClass__OnDeployBegin (0x0070fc90), TechnoClass__OnUndeployComplete (0x0070fbe0),
TechnoClass__StopAllTargeting (0x0070d4a0), TechnoClass__HasStealthAbility (0x0070c5a0),
TechnoClass__IsUnderEMP (0x0070efd0), TechnoClass__FreeAllMindControlCaptures (0x00710460),
TechnoClass__Passive_Target_Acquire (0x00709480), Volume__IsNormal (0x0074ff90),
FUN_00750010 (elite check)

**Cross-referenced docs:**
TECHNOCLASS_STRUCT_LAYOUT.md, TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md,
TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md, CLOAKING_INTERACTIONS_REPORT.md,
MIND_CONTROL_GHIDRA_REPORT.md, VETERANCY_SYSTEM_GHIDRA_REPORT.md,
RADIATION_EMP_GHIDRA_REPORT.md, RECEIVE_DAMAGE_GHIDRA_REPORT.md

**Ghidra struct definition:** TechnoClass (1312 bytes, 36 named fields)

---

## ADDENDUM: Deep Dive Findings (2026-04-01, session 2)

### A1. Cloaking System — Complete Field Map

From StartCloaking (0x00703770), StartUncloaking (0x007036c0), CloakingTick, GetVisualState:

| Offset | Type | Name | Evidence | Confidence |
|--------|------|------|----------|------------|
| 0x220 | int | **CloakState** | Ghidra struct. 0=Uncloaked, 1=Cloaking, 2=Cloaked, 3=Uncloaking | HIGH |
| 0x224 | int | **CloakProgress** | StartCloaking: set to 0. StartUncloaking: set to RulesClass+0x628 - 1. GetVisualState: compared against 0x40/0x80/0xC0 for alpha levels | HIGH |
| 0x228 | byte | **CloakInitialized** | ProcessCloakMode: toggled by cloak/uncloak radio messages | MED |
| 0x22C | int | **CloakTimer.StartFrame** | StartCloaking: set to g_CurrentFrameCounter | HIGH |
| 0x230 | int | **CloakTimer.Mid** | StartCloaking: set from stack | HIGH |
| 0x234 | int | **CloakTimer.Duration** | StartCloaking: set to TechnoTypeClass+0x310 (CloakingSpeed) | HIGH |
| 0x238 | int | **CloakAnimMaxDuration** | StartCloaking: also set to CloakingSpeed | HIGH |
| 0x23C | int | **CloakAnimDirection** | StartCloaking: set to 1. StartUncloaking: set to -1. Controls animation direction. | HIGH |

**GetVisualState alpha mapping (CloakProgress 0x224):**
- Progress < 0x40 → visibility level 1 (barely visible shimmer)
- Progress < 0x80 → visibility level 2
- Progress < 0xC0 → visibility level 3
- Progress >= 0xC0 → fully cloaked / 3 depending on player alliance

### A2. Fire_At — Complete Analysis (919 lines)

**Weapon effect creation (0x304-0x324):**

| Offset | Type | Weapon Flag | Name | Confidence |
|--------|------|-------------|------|------------|
| 0x304 | ptr | IsLaser (+0x129) | **LaserBeamParticle** — ParticleSystemClass, created from weapon+0x11C | HIGH |
| 0x308 | ptr | Flag +0x12A | **ElectricBoltParticle** — ParticleSystemClass. corrected 2026-05-28: name is MISLEADING; this slot is dual-use. AI_Update also writes the damage-smoke particle here when health drops below threshold. Section I6 identifies it as DamageSmokePSystem; 0x30C is the ElectricBolt slot. See Section I6 for the authoritative particle block map — ROOT_CAUSE: INFERENCE_HARDENED | HIGH |
| 0x314 | ptr | Flag +0x12D | **BeamParticle3** — ParticleSystemClass | HIGH |
| 0x324 | ptr | IsSonic (+0x130) | **Wave** — WaveClass for sonic weapons (Ghidra-named) | HIGH |

**Burst rate activation (from Fire_At ~line 650):**
When a gattling-capable unit fires, burst rate data is computed:
```
Primary:  0x3F0 = 1 (active), 0x3F4 = 0x3DC clamped ≥ 1, 0x3E8 = 0x3D8 / 0x3F4
Secondary: 0x410 = 1 (active), 0x414 = 0x3FC clamped ≥ 1, 0x408 = 0x3F8 / 0x414
```

**ROF halving for mind-controlled units:**
```c
if (this->field_0x298 != 0) {  // IsMindControlVictim
    ROF = ROF / 2;
}
```
Confirmed: mind-controlled units fire at **half rate**. Active in YR.

**LastFireFrame (0x120):** Set to g_CurrentFrameCounter at the END of Fire_At (line ~870).
Constructor init: 0xFFFFFF9C (-100), meaning "never fired" — an intentionally old frame number.

**Muzzle flash timer (0x3BC-0x3C4):**
```c
if (TechnoTypeClass->HasMuzzleFlash) {
    this->MuzzleFlashTimer.Start = g_CurrentFrameCounter;
    this->MuzzleFlashTimer.Duration = 0xF;  // 15 frames
}
```

**Barrel rotation (0x43C):**
```c
this->BarrelRotationIndex++;
if (TechnoType->BarrelCount <= this->BarrelRotationIndex)
    this->BarrelRotationIndex = 0;
```
Uses TechnoTypeClass+0x6A4 for barrel count. For buildings, uses 0x69C instead.

**Target tracking list (0x470-0x484):**
Used for weapons with IsAttackAndMove flag (TechnoTypeClass+0x6B0). After firing:
```c
if (this->TargetList.Count < this->TargetList.Capacity || can_grow) {
    this->TargetList.Data[Count] = target;
    this->TargetList.Count++;
}
Set_ArchiveTarget(first_entry);  // re-target to first in list
```

### A3. Set_ArchiveTarget — Target Resolution

From Set_ArchiveTarget (0x006fcdb0, 82 lines):

Key behaviors:
1. Clears IsNewTarget (0x50C) at start
2. If target is same as current ArchiveTarget, returns immediately (no-op)
3. For infantry (RTTI == 2): if NotATransport and not special, clears Ammo (0x2FC) to 0
4. **Transport redirection**: If target is a unit with 0x2E4 (GarrisonBuildingPtr) set and is
   not a building, redirects target to 0x2E4 (the transport itself). This is how targeting
   a garrisoned infantry actually targets the building.
5. Clears CurrentBurstIndex (0x3B8) to 0 when target is cleared
6. Destroys LaserBeam particle (0x304) if new target can't use same weapon

### A4. Embedded Object Layout (0x338-0x3B8)

From constructor analysis and embedded constructors:

| Offset Range | Size | Type | Name | Evidence |
|-------------|------|------|------|----------|
| 0x338 | 4 | int | **PreviousGarrisonID** | Init: -1. Used to track last garrison | MED |
| 0x33C-0x34B | 16 | StorageClass | **OreStorage** | 4 floats, one per ore type. StorageClass::Constructor zeros all | HIGH |
| 0x34C-0x36F | 32 | embedded obj | **SensorTracker** | FUN_004a50f0: has timer at +8, flags at +0x18/+0x19. corrected 2026-05-28: was ~26 bytes (0x34C-0x365); binary LEA instructions show 32-byte size ending at 0x36F — ROOT_CAUSE: STRUCT_FAMILY_CASCADE | MED |
| 0x370-0x387 | 24 | FacingClass | **BodyFacing** | FUN_004c91e0(3): rotation rate 3. Tracks body direction. corrected 2026-05-28: was 0x366-0x37F (~22 bytes); LEA instructions at FacingClass constructors confirm 24-byte size, start 0x370 — ROOT_CAUSE: STRUCT_FAMILY_CASCADE | HIGH |
| 0x388-0x39F | 24 | FacingClass | **TurretFacing** | FUN_004c91c0(): rotation rate 0 (instant). Independent turret. corrected 2026-05-28: was 0x380-0x399 (~22 bytes); follows corrected BodyFacing — ROOT_CAUSE: STRUCT_FAMILY_CASCADE | HIGH |
| 0x3A0-0x3B7 | 24 | FacingClass | **BarrelFacing** | FUN_004c91c0(): rotation rate 0. For multi-barrel weapons. corrected 2026-05-28: was 0x39A-0x3B3 (~22 bytes); follows corrected TurretFacing — ROOT_CAUSE: STRUCT_FAMILY_CASCADE | HIGH |

**FacingClass internal layout (24 bytes) — corrected 2026-05-28: was 22 bytes; verified via LEA in FacingClass constructors (0x004c91e0, 0x004c91c0) — ROOT_CAUSE: STRUCT_FAMILY_CASCADE:**
```
+0x00: short Current      (current facing value, 0-65535)
+0x02: short padding
+0x04: short Target       (target facing to rotate toward)
+0x06: short padding
+0x08: int   Timer.StartFrame
+0x0C: int   Timer.Mid
+0x10: int   Timer.Duration
+0x14: short RotationRate (rate << 8, clamped ≤ 0x7F)
+0x16: short padding
```
Total: 24 bytes (0x18). Corrected from prior 22-byte layout which was missing the mid field and padding bytes.

### A5. PerformDeploy — Deploy System Fields

From PerformDeploy (0x00710000, 153 lines):

| Offset | Field | Evidence | Confidence |
|--------|-------|----------|------------|
| 0x2AC | LocomotorTarget | Set during deploy: links deployed building ↔ deployer unit | HIGH |
| 0x2B0 | **LinkedBuilding** | Set to the deployed building ptr (bidirectional with 0x2AC) | HIGH |
| 0x674 | ILocomotor | COM locomotor interface, released and replaced during deploy | HIGH |
| 0x694 | **DeployAnimPtr** | Checked for parasite weapon units | MED |
| 0x6AD | **deploy_or_locomotor_piggyback_active** (byte) | Runtime Foot guard set by active deploy/piggyback paths. Blocks non-null Set_Destination_Internal writes while set; exact clear lifecycle requires a separate audit. Older `IsDeployed` wording is too narrow. | HIGH |
| 0x6B6 | **IsDeploying** (byte) | Set to 1 during deploy transition. Cleared after. | HIGH |
| 0x5D4 | **TeamPtr** | Checked for team convoy operations on deploy | MED |

### A6. What_Action_OnObject — Action Fields

From What_Action_OnObject (0x006ffec0, 238 lines):

| Offset | Type | Name | Evidence | Confidence |
|--------|------|------|----------|------------|
| 0x298 | byte | **IsMindControlVictim** | When set, returns action 0 (no action possible) | HIGH |
| 0x2A8 | int | **EMP/DisableState** | Combined with TechnoType+0x692 to block actions | MED |
| 0x3D4 | byte | **IsTemporalTarget** | Blocks self-action when player-controlled + warping | HIGH |
| 0x114 | int | **AmmoCount** | Checked > 0 for infantry self-deploy (ammo-gated abilities) | HIGH |

**Note on 0x114:** In What_Action_OnObject, infantry (RTTI==1) check `param_1[0x45] > 0`
(byte offset 0x114) to determine if self-deploy action is available. This is separate from
the Ammo field at 0x2FC which is used for IFV/aircraft ammo. 0x114 appears to be an
**infantry-specific ammo** or **ability charge count**.

### A7. GetFireError — Complete Fire Permission Checks (412 lines)

Fields checked (summary of all FIRE_ILLEGAL returns):

| Check | Offset(s) | Meaning |
|-------|-----------|---------|
| target == null | — | No target |
| field_0x2DC != 0 | 0x2DC | Unit is "busy" (garrison entering?) |
| IsWarping (vtable+0x1D8) | — | Unit is chronoshifting |
| target == LocomotorTarget | 0x2AC | Can't fire at own locomotor target |
| IsBeingWarped (vtable+0x1D4) | — | Being temporal-warped |
| field_0x1C8 != 0 | 0x1C8 | IsDeployed (deploying units can't fire) |
| IsSinking | 0x3CD | Sinking ships can't fire |
| target == DrainTarget | 0x1CC | Can't fire at drain link target |
| target == field_0x11C | 0x11C | Can't fire at linked entity (transport?) |
| TemporalClass active + same target | 0x274 | Already temporaling this target |
| target.ChronoInTransit | target+0x27C | Target is in chrono transit |
| target.IsInvulnerable | vtable+0x160 | Iron curtained (for non-player) |
| LaserBeam/Bolt/Wave active | 0x304/308/314/324 | Beam weapon still active |
| field_0x8D != 0 | 0x234 | Cloak timer duration non-zero (cloaking) |
| elite + 0x11C link warping | 0x11C | Elite unit's garrison is being warped |
| target warping + non-temporal | — | Target is warping and weapon isn't temporal |

### A8. StorageClass Details

StorageClass is 16 bytes = 4 floats, one per ore type (0-3).
- GetTotalAsInt loops through all 4 slots, sums floors of positive values
- AddAmount(slot, amount) adds float to slot
- RemoveAmount(slot, amount) subtracts from slot

Embedded at TechnoClass+0x33C. Used by harvesters to track carried ore.

### A9. Remaining Open Questions (Updated)

1. **0x0F0 (param_1[0x3C]):** Init 0. AI_Update sets it from RulesClass+0xBE8 on damage
   volume change. Likely **ThreatPoseValue** or **EnemyRating**.

2. **0x114 (param_1[0x45]):** Init 0. Checked > 0 for infantry self-deploy in
   What_Action_OnObject. Could be infantry-specific **AbilityAmmo** or **DeployCharge**.

3. **0x148 (param_1[0x52]):** Init 0. Referenced in constructor but purpose unknown.

4. **0x1A8/0x1B0 timer:** Init duration 0. Usage not found in decompiled functions.
   Possibly **AntiAirTimer** or **IdleAnimTimer**.

5. **0x208:** Passed to RadarClass__MarkCellDirty in IdleActionTick. Likely a
   **CellStruct** (2 shorts = 4 bytes) for radar tracking position.

6. **0x2A4 (byte, param_1[0xA9]):** Init 0. Not definitively identified yet.

7. **0x2B8-0x2BC gap:** Between Target (0x2B4) and CaptureManager (0x2BC).
   Only 4 bytes — likely padding or a small flag.

8. **0x2CC-0x2E0 range:** Several 0-init pointers. 0x2D0=SpawnManager, 0x2D8=SlaveManager
   confirmed. 0x2CC, 0x2DC, 0x2E0 still unknown. 0x2DC blocks firing in GetFireError.

9. **0x338:** Init -1. Likely tracks last garrison building ID or previous state index.

10. **FacingClass exact byte boundaries:** The three FacingClass instances between 0x366-0x3B3
    have estimated boundaries. Exact alignment needs assembly-level `lea` verification.

---

## ADDENDUM: Open Question Resolution (session 2, iteration 1)

**Research date:** 2026-04-01
**Method:** Ghidra MCP — disassembly LEA tracing, byte-pattern search for MOV instructions,
decompilation of 15+ additional functions.

### Resolved Open Questions

| # | Offset | Type | Name | Init | Evidence | Confidence |
|---|--------|------|------|------|----------|------------|
| 1 | 0x0F0 | int | **ThreatPoseFlags** | 0 | AI_Update: set from RulesClass+0xBE8 when Volume__GetCategory() transitions to 0 (elite). Later in AI_Update (~line 490), compared across frames; change triggers building animation facing update for Powered state. Dual purpose: records elite-transition threat value AND tracks powered animation state for buildings. | MED |
| 2 | 0x114 | int | **ShotCount** | 0 | What_Action_OnObject: infantry (RTTI==1) checks `TechnoTypeClass+0x5E0 > 0` AND `this+0x114 > 0` to permit self-deploy action. TechnoTypeClass+0x5E0 is likely `Ammo` capacity. 0x114 is the current ammo/charge count for deploy-gated abilities (separate from 0x2FC which is IFV/aircraft ammo). | HIGH |
| 3 | 0x148 | int | **GattlingCycleCount** | 0 | FUN_00736df0 (UnitClass combat tick): incremented when IsGattling flag (TechnoTypeClass+0xCD5) is set and fire result is FIRE_OK (0) or FIRE_REARM (3). Counts completed fire cycles to track gattling weapon ramp progression. Separate from GattlingValue (0x144) which is threshold-compared. | HIGH |
| 4 | 0x1A8/0x1B0 | CDTimerClass | **IdleActionTimer** | duration 0 | Constructor: `[ESI+0x1A8] = g_CurrentFrameCounter`, `[ESI+0x1B0] = 0`. Standard CDTimerClass init (StartFrame=now, Duration=0 = expired). From AI_Update context near the retaliation scan code: this timer gates idle action processing. When expired, idle scan runs. | MED |
| 5 | 0x208-0x20C | CellStruct (2 shorts + padding) | **RadarTrackingCell** | 0 | IdleActionTick passes `this+0x208` to RadarClass__MarkCellDirty(). RegisterOnRadar passes `(this+0x208, this+0x20C)` to RadarClass__AddObjectToTracker(). UnregisterFromRadar does the reverse. This is the **cell position** where this object is registered on the radar minimap. | HIGH |
| 6 | 0x2A4 | byte | **IsProne** | 0 | FUN_00520ae0 (infantry animation sequencer): set to 1 during crawl-down sequence (case 0x1B), cleared to 0 during crawl-up sequence (case 0x1F). Gated by InfantryTypeClass+0xEC9 (Crawls flag). Tracks whether infantry is in prone/crawling position. | HIGH |
| 7 | 0x2B8 | ptr (4 bytes) | **SuspendedTarget** | 0 | FUN_007013a0 (called from FootClass__Set_NavCom_With_Suspend): copies Target (0x2B4) to 0x2B8 before assigning new target. This preserves the previous attack target when a unit is temporarily redirected (e.g., forced move, guard command). The suspended target can be restored when the override completes. | HIGH |
| 8 | 0x2DC | ptr | **SlaveOwner** | 0 | FUN_006b0ae0 (SlaveManagerClass::FreeAllSlaves): iterates slave list and sets `slave[0xB7] = 0` (byte offset 0x2DC) on each slave. Called from ReceiveDamage, TemporalClass__Update, TeleportLocomotionClass__PostWarpValidation. When non-zero, GetFireError returns FIRE_ILLEGAL — enslaved units cannot fire independently. Points back to the TechnoClass that owns the SlaveManagerClass managing this slave. | HIGH |
| 9 | 0x338 | int | **PreviousGarrisonID** | -1 (0xFFFFFFFF) | Constructor: `[ESI+0x338] = EBP` (EBP = -1). Located immediately before StorageClass at 0x33C (confirmed from `LEA ECX,[ESI+0x33C]` → StorageClass__Constructor). Value -1 means "no previous garrison". Tracks the last garrison/transport building for the unit. | MED |
| 10 | — | FacingClass x3 | **Exact boundaries verified** | — | Assembly LEA instructions in constructor confirm exact offsets. See table below. | HIGH |

### FacingClass Verified Boundaries (from constructor LEA instructions)

| Object | Start Offset | LEA Instruction | Constructor Called | Rate |
|--------|-------------|-----------------|-------------------|------|
| **BodyFacing** | 0x370 | `LEA ECX,[ESI+0x370]` | FUN_004c91e0 (with rate=3) | 3 |
| **TurretFacing** | 0x388 | `LEA ECX,[ESI+0x388]` | FUN_004c91c0 (rate=0, instant) | 0 |
| **BarrelFacing** | 0x3A0 | `LEA ECX,[ESI+0x3A0]` | FUN_004c91c0 (rate=0, instant) | 0 |

**Each FacingClass is 0x18 = 24 bytes** (not 22 as previously estimated).

FacingClass internal layout (24 bytes, from FUN_004c91e0 decompilation where param is `undefined2*`):
```
+0x00: short  Current        (current facing value, 0-65535)
+0x02: short  (padding)
+0x04: short  Target         (target facing to rotate toward)
+0x06: short  (padding)
+0x08: int    Timer.StartFrame
+0x0C: int    Timer.Mid      (unknown CDTimer field)
+0x10: int    Timer.Duration
+0x14: short  RotationRate   (rate << 8, clamped <= 0x7F00)
+0x16: short  (padding to 24 bytes)
```

**Previous estimate correction:** Report section A4 listed 0x366-0x37F, 0x380-0x399, 0x39A-0x3B3
(22 bytes each). The correct boundaries are 0x370-0x387, 0x388-0x39F, 0x3A0-0x3B7 (24 bytes each).

### Additional Embedded Object Boundaries (from constructor LEA instructions)

| Object | Start Offset | Constructor | Size | Purpose |
|--------|-------------|-------------|------|---------|
| **StorageClass** | 0x33C | `LEA ECX,[ESI+0x33C]` → StorageClass__Constructor (0x6C95E0) | 16 bytes | Ore storage (4 floats) |
| **SensorTracker** | 0x350 | `LEA ECX,[ESI+0x350]` → FUN_004A50F0 | 26 bytes | Timer + 2 flags at +0x18/+0x19. FUN_004A5150 checks completion. FUN_004A5360 state transitions. |
| **SoundEvent[0]** | 0x488 | `LEA ECX,[ESI+0x488]` → FUN_00405BE0 | 28 bytes | Sound/anim event handle |
| **SoundEvent[1]** | 0x4A4 | `LEA ECX,[ESI+0x4A4]` → FUN_00405BE0 | 28 bytes | Sound/anim event handle |
| **SoundEvent[2]** | 0x4C0 | `LEA ECX,[ESI+0x4C0]` → FUN_00405BE0 | 28 bytes | Sound/anim event handle |
| **SoundEvent[3]** | 0x4DC | `LEA ECX,[ESI+0x4DC]` → FUN_00405BE0 | 28 bytes | Sound positioning (used by VocClass__PlayAtPos in AI_Update) |

### Additional Fields Resolved (0x400-0x520 range)

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| 0x423 | byte | **IsOnRadar** | 0 | RegisterOnRadar sets to 1; UnregisterFromRadar sets to 0 | HIGH |
| 0x434 | int | **SuicideTeamPtr** | 0 | Fire_At: set from FootClass+0x5D4 (team pointer) when suicide weapon fires at a building (TechnoTypeClass+0xD3D flag) | MED |
| 0x488-0x4F7 | embedded[4] | **SoundEvents[4]** | vtable+zeros | Four 28-byte sound event objects. AI_Update accesses 0x488 for anim-related audio, 0x49C/0x4A0 as state flags, 0x4DC for positional sound | HIGH |
| 0x4F0 | int | **QueuedVoice** | -1 | AI_Update: when != -1, plays via VocClass__PlayAtPos using coords from SoundEvent[3] at 0x4DC. Reset to -1 after play. | HIGH |
| 0x4F4 | int | **LastPlayedVoice** | -1 | AI_Update: set from 0x4F0 after successful play. Compared to prevent duplicate playback of same sound. | HIGH |
| 0x49C | int | **SoundState** | 1 | AI_Update: toggled between 0/1 for sound event lifecycle when TechnoTypeClass+0xCA1 flag set | MED |
| 0x4A0 | int | **SoundActive** | 0 | AI_Update: when 0, releases sound and sets SoundState=1; when non-zero, plays pending sound | MED |

### Corrections to Previous Sections

**Section A4 (Embedded Object Layout):** The FacingClass boundaries were incorrect:
- Old: BodyFacing=0x366, TurretFacing=0x380, BarrelFacing=0x39A (22 bytes each)
- **Correct: BodyFacing=0x370, TurretFacing=0x388, BarrelFacing=0x3A0 (24 bytes each)**
- The SensorTracker object at 0x350 is 32 bytes (0x20), not 26 as initially estimated,
  ending at 0x36F. This pushes FacingClass start from 0x366 to 0x370.

**Section 5c (Pointer Fields):** 0x2DC was listed as GarrisonBuildingPtr in error. The actual
identity is **SlaveOwner** — a back-pointer from an enslaved unit to the TechnoClass that
owns the SlaveManagerClass managing it. 0x2E4 remains GarrisonBuildingPtr.

### Remaining Unknowns

| Offset | Status | Notes |
|--------|--------|-------|
| 0x2CC | UNRESOLVED | Pointer, init 0. Between MCRingAnim (0x2C8) and SpawnManager (0x2D0). Possibly another MC-related anim or link. |
| 0x2E0 | UNRESOLVED | Pointer, init 0. After SlaveManager (0x2D8). Possibly slave-related anim or link. |
| 0x438-0x43A | UNRESOLVED | 3 bytes, init 0 each. In the flag block after 0x434. |
| 0x44C-0x44D | UNRESOLVED | 2 bytes. 0x44C init 1 (CanGrow flag for DynVec?), 0x44D init 0. |
| 0x500-0x514 | UNRESOLVED | Several fields. 0x500 and 0x504 (EMPLockRemaining) known. 0x508-0x514 unknown. |

---

## ADDENDUM: Verification Pass (session 2, iteration 3)

**Research date:** 2026-04-01
**Method:** Ghidra MCP — byte-pattern search for MOV/CMP instructions at specific struct
offsets, decompilation of 20+ additional functions including AircraftClass__Unlimbo,
BuildingClass__UpdateRepairAndPower, BuildingClass__MissionRepairAndProduce,
UnitClass__Mission_Harvest, FootClass__AI, TemporalClass__InitiateWarp,
TemporalClass__DetachFromTarget, TemporalClass__Update, TechnoTypeClass__ReadINI (partial).

### Byte Flag Block 0x3CE-0x3D5: Verified Results

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| 0x3CD | byte | **IsSinking** | 0 | Ghidra struct. Already confirmed. | HIGH |
| 0x3CE | byte | **PreviousSinkState** | 0 | FootClass__AI (0x4da530): shadow copy of IsSinking (0x3CD) for edge detection. Code: `if (0x3CD != 0x3CE) { play sink/unsink sound effects; 0x3CE = 0x3CD; }`. Same pattern as 0x8D/0x8E (OnBridge shadow) and 0x425/0x426 (IsShipRocking shadow). **Not serialized by Save** (Save skips from 0x3CD to 0x3CF). | HIGH |
| 0x3CF | byte | **ShouldProtect** | 0 | ReceiveDamage (0x701900): checked alongside TechnoTypeClass+0xC96 ("ToProtect" INI key) to trigger AI base defense response FUN_00708080. Logic: `if (Type.ToProtect \|\| this.ShouldProtect) && !IsPlayerControlled && attacker != null → notify AI defense`. **Never set to 1 anywhere in the binary** — effectively dead code in YR. Likely a TS remnant where campaign triggers could set this flag. Serialized by Save but always 0. | HIGH (identity), LOW (usage in YR) |
| 0x3D0 | byte | **IsHarvesting** | 0 | UnitClass__Mission_Harvest (0x73e5e0): set to 1 at 0x73e8fa when harvester enters active ore collection (alongside mission state = 4). Cleared to 0 at 0x73e925 when harvesting stops. BuildingClass__MissionRepairAndProduce (0x44b780) reads it at 0x44c4bc: if set && AI-controlled → triggers `SetRepairState(1)` (vtable+0x1A0) to auto-repair the refinery. Serialized by Save. | HIGH |
| 0x3D1 | byte | **WasAttackedByEnemy** | 0 | ReceiveDamage: `if (attacker != null && !HouseClass__Is_Ally(attacker)) this->0x3D1 = 1;`. BuildingClass__UpdateRepairAndPower (0x450630): `if (0x3D1 && credits >= threshold && Random(0,50) < house_repair_chance) → trigger auto-repair`. This is the trigger for AI auto-repair: buildings that were attacked by enemies get flagged for repair consideration. Serialized by Save. | HIGH |
| 0x3D2 | byte | **Cloakable** | 0 | Ghidra struct + HasStealthAbility getter. Already confirmed. | HIGH |
| 0x3D3 | byte | **IsPrimaryFactory** | 0 | Ghidra struct. Already confirmed. | HIGH |
| 0x3D4 | byte | **IsAirborne** | 0 | AircraftClass__Unlimbo (0x414310): set to 1 when aircraft spawns at flight altitude. Controls height calculation: if 0, uses ground height; if 1, uses ground + flight ceiling. Also set by FUN_0065d8e0 and related team/script spawn functions (0x65d8e0, 0x65dd30, 0x65e660, 0x65e8ff, 0x65eb10) for paradropped units. What_Action_OnObject (0x6ffec0): `if (IsAirborne && IsControlledByPlayer) return NO_ACTION` — prevents player orders to airborne units. **Never cleared to 0 after being set** (except by constructor). Serialized by Save. | HIGH |
| 0x3D5 | byte | **HasSight** | 0 | UpdateReveal: gates all sight reveal processing. Already confirmed. | HIGH |

### Correction: Previous 0x3D4 Label

Section A6 (What_Action_OnObject) previously labeled 0x3D4 as "IsTemporalTarget" at HIGH
confidence based on the What_Action context. This was incorrect. The field is **IsAirborne**
— set when aircraft unlimbo at flight altitude or when units are paradropped. The temporal
system uses 0x278 (TemporalTargetingMe) instead. The What_Action check prevents player
orders to airborne units, not temporal targets.

### Field 0x278: TemporalTargetingMe — Verified

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| 0x278 | ptr | **TemporalTargetingMe** | 0 | TemporalClass__InitiateWarp (0x71af20): `*(int*)(target + 0x278) = this_temporal;` — stores the TemporalClass pointer on the target being warped. TemporalClass__DetachFromTarget (0x71abc0): `*(int*)(target + 0x278) = 0;` — cleared when temporal detaches. TemporalClass__Update (0x71a760): reads `target[0x9E]` (= 0x278 via int* indexing) to check if temporal link is stale. This is the back-pointer from a unit to the TemporalClass that is warping it. | HIGH |

### Fields 0x438-0x439, 0x43A, 0x44D: Confirmed Unused/Padding

Exhaustive byte-pattern search for all x86 MOV/CMP/TEST register encodings
(EAX/ECX/EDX/EBX/ESI/EDI/EBP) with displacements 0x438, 0x439, 0x43A, and 0x44D
found **zero** results outside the constructor. These bytes are:
- Initialized to 0 in constructor
- Never read by any function
- Never written by any function
- Not serialized by Save (Save skips this range entirely)

**Verdict:** True padding/dead fields. Safe to ignore in implementation.

### Field 0x514: PlanningNodeClass Pointer — Corrected

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| 0x514 | ptr | **PlanningToken** | 0 | Getter FUN_00705d20 (6 bytes): `return *(int*)(this + 0x514);`. Setter FUN_00705d10 (8 bytes): `*(int*)(this + 0x514) = param;`. Setter called from FUN_00638a80 which creates a 0x9C-byte PlanningNodeClass object (vtable PTR_FUN_007efe44, RTTI confirms DynamicVectorClass<PlanningNodeClass*>). Getter called by 29+ functions in the 0x633-0x63B range (event/action handlers) and ObjectSelection__PlayVoice. This is the **waypoint planning** system (Ctrl+Alt plan mode). NOT "AttachedEffectCount" as previously tentatively labeled. | HIGH |

### Early Range Fields 0x0F0-0x0FC: Status Update

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| 0x0F0 | int | **ThreatPoseFlags** | 0 | AI_Update: set from RulesClass+0xBE8 when damage volume category transitions. Prior assessment stands. | MED |
| 0x0F4 | byte | **Unused** | 0 | Exhaustive byte-pattern search found zero reads/writes outside constructor. The single hit at 0x6ef381 reads `*(*(param_1+0x24) + 0xF4)` which is a different struct (HouseClass member), not TechnoClass. Dead field. | HIGH |
| 0x0F8 | int | **Unknown** | 0 | Getter/setter pair at FUN_00487c70/FUN_00487c80 exist but are called from RadSiteClass__Constructor and WarheadTypeClass__Detonate on CellClass objects (offset 0xF8 coincidence), not TechnoClass. No confirmed TechnoClass-specific access found. Possibly dead or only accessed via subclass virtual calls. | LOW |
| 0x0FC | byte | **Unknown** | 0 | No reads/writes found outside constructor. Dead field or only accessed through subclass code not yet decompiled. | LOW |

### TechnoTypeClass INI Key Cross-Reference (New Findings)

| TechnoTypeClass Offset | INI Key | Type | Evidence |
|------------------------|---------|------|----------|
| 0xC96 | **ToProtect** | bool | ReadINI at 0x714be3. String at 0x8438DC. Controls AI base defense response. |
| 0xCCC | **Repairable** | bool | ReadINI at 0x714a84. String at 0x843950. |
| 0xD14 | **SelfHealing** | bool | ReadINI at 0x714ad9. String at 0x843928. |

### Summary of Confidence Changes

| Offset | Previous | New | Change Reason |
|--------|----------|-----|---------------|
| 0x3CE | LOW "Unknown" | **HIGH "PreviousSinkState"** | FootClass__AI edge detection pattern verified |
| 0x3CF | MED "Repairable" | **HIGH "ShouldProtect" (dead in YR)** | ReceiveDamage + ToProtect cross-ref; never set to 1 |
| 0x3D0 | LOW "IsInOre" | **HIGH "IsHarvesting"** | UnitClass__Mission_Harvest set/clear verified |
| 0x3D1 | MED "WasAttacked" | **HIGH "WasAttackedByEnemy"** | ReceiveDamage sets; BuildingClass__UpdateRepairAndPower reads for AI auto-repair |
| 0x3D4 | HIGH "IsTemporalTarget" | **HIGH "IsAirborne"** | AircraftClass__Unlimbo + team paradrop functions verified. Previous label was WRONG. |
| 0x278 | MED "TemporalWarper" | **HIGH "TemporalTargetingMe"** | TemporalClass__InitiateWarp/DetachFromTarget verified |
| 0x438-0x439 | LOW "padding" | **HIGH "padding/dead"** | Zero accesses found in entire binary |
| 0x43A | MED "MovementZoneOverride" | **HIGH "padding/dead"** | Zero accesses found; previous label was speculative |
| 0x44D | MED "HasNewPassenger" | **HIGH "padding/dead"** | Zero accesses found; previous label was speculative |
| 0x514 | LOW "AttachedEffectCount" | **HIGH "PlanningToken" (ptr)** | Getter/setter + PlanningNodeClass RTTI verified |
| 0x0F4 | MED "AnimActive" (byte) | **HIGH "Unused/Dead"** | Zero TechnoClass-specific accesses found |

### Additional Functions Decompiled This Session

- AircraftClass__Unlimbo (0x00414310) — aircraft spawn height + IsAirborne
- BuildingClass__UpdateRepairAndPower (0x00450630) — AI auto-repair logic
- BuildingClass__MissionRepairAndProduce (0x0044b780, partial) — building repair mission
- UnitClass__Mission_Harvest (0x0073e5e0) — harvesting flag management
- FootClass__AI (0x004da530, partial) — sink state edge detection
- TemporalClass__InitiateWarp (0x0071af20) — temporal target pointer write
- TemporalClass__DetachFromTarget (0x0071abc0) — temporal target pointer clear
- TemporalClass__Update (0x0071a760) — temporal damage + warp kill
- FUN_00705d10 / FUN_00705d20 — PlanningToken setter/getter
- FUN_00638a80 — PlanningNodeClass constructor + registration
- FUN_00708080 (partial) — AI base defense response
- FUN_0065d8e0 (partial) — team paradrop spawn

---

## ADDENDUM: Final Unknown Resolution (session 2, iteration 2)

**Research date:** 2026-04-01
**Method:** Ghidra MCP — Java script to find all MOV/LEA/CMP instructions accessing each offset
in the 0x6e0000-0x760000 range, followed by targeted decompilation of every function that
touches the offset. Cross-referenced with Load function (FUN_0070bf50), Detach function
(FUN_007077c0), GetOriginalOwner function (FUN_0070f820), CaptureManagerClass::CanCapture,
FootClass::TryEnterTransport, TechnoClass::Set_Destination, and FLY locomotor functions.

**Functions decompiled this session (15+):** FUN_0070f820 (GetOriginalOwner), FUN_007077c0
(Detach), FUN_0070bf50 (Load), CaptureManagerClass::CanCapture (0x00471c90),
CaptureManagerClass::CaptureUnit (0x00471d40), CaptureManagerClass::FreeUnit (0x00471ff0),
FUN_007105e0 (IsMindControlled check), FUN_006fc030 (CanAct check),
TemporalClass::InitiateWarp (0x0071af20), TemporalClass::Update (0x0071a760),
TechnoClass::OnDeployBegin (0x0070fc90), TechnoClass::GetByte_0x1C8 (0x0070fbd0),
FUN_0070d8f0 (DockIdleCheck), FUN_00700600 (What_Action_OnCell), FindOffset2CC script,
FindMultipleOffsets script.

### Resolved Fields

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| **0x2CC** | ptr | **HijackerInfantry** | 0 | FUN_0070f820 (GetOriginalOwner): `if (0x2C0 != 0) return CaptureManager.OrigOwner; if (0x2CC != 0) return 0x2E0; return 0x21C (Owner)`. CaptureManagerClass::CanCapture: if 0x2CC != 0 on target, capture is DENIED (same as if already mind-controlled). FUN_007077c0 (Detach): cleared when detaching. FUN_0070bf50 (Load): resolved as pointer. Points to the InfantryClass that has hijacked/driven this unit. When set, the original owner is preserved at 0x2E0. | **HIGH** |
| **0x2E0** | ptr | **OriginalOwnerHouse** | 0 | FUN_0070f820 returns `*(0x2E0)` as the "real owner" when 0x2CC (HijackerInfantry) is set. This is the HouseClass pointer for the unit's original owner BEFORE the hijacker took control. The current Owner (0x21C) switches to the hijacker's house; 0x2E0 preserves the old one for restoration. Load function resolves it as a pointer. | **HIGH** |
| **0x2D4** | ptr | **SpawnOwner** | 0 | RecordKill (0x00702d40): reads `[EDI+0x2D4]` three times — used to credit kills to the parent spawner rather than the spawned missile. FUN_007077c0 (Detach): cleared when detaching. Load resolves as pointer. Located between SpawnManager (0x2D0) and SlaveManager (0x2D8). Back-pointer from a spawned unit to its parent spawner TechnoClass. | **HIGH** |
| **0x2E8** | float | **PitchAngle** | 0.0 | AIRCRAFTCLASS_GHIDRA_REPORT.md: "owner.PitchAngle = pitch (offset 0x2E8 in TechnoClass, float)". FlyLocomotor::Is_Moving returns true if PitchAngle > 0.0. Aircraft gradually level out on ground: `PitchAngle -= decrement_rate; clamp >= 0`. Used for aircraft nose-up/down visual angle during flight. Save function serializes at 0x2E8 via FUN_004a1d70 (float serializer). | **HIGH** |
| **0x500** | ptr | **QueuedEntryTarget** | 0 | FootClass::TryEnterTransport (0x0070d7e4): reads 0x500, if pointing to invalid/dead object, clears to 0. If valid, issues movement command toward it. TechnoClass::Set_Destination (0x00741cb8): sets 0x500 to the destination building when entering a transport/building. FUN_007077c0 (Detach): cleared. Load resolves as pointer. This is the building/transport that the unit is queued to enter. | **HIGH** |
| **0x508** | int | **FlyLocomotor_CurrentSpeed** | 0 | Heavily used in FLY locomotor range (0x00759xxx). FUN_0070f670 writes it, FUN_0070f6a0/FUN_0070f6e0 read and clear it. Also in FUN_006f6ac0/FUN_006f6ca0 (TechnoClass flight state). Tracks the current flight speed value for aircraft. In TechnoTypeClass (different class), this offset has a different meaning. | **MED** |
| **0x510** | int/ptr | **SonicWaveTarget** | 0 | Constructor init 0. Fire_At (0x006fd7df): `MOV [EAX+0x510], ESI` — stores the firer into the target when a sonic weapon fires. FlyLocomotor functions compare against -1. In TechnoTypeClass this is a DynamicVectorClass. For TechnoClass instances, this tracks the source of a sonic wave affecting this unit. | **MED** |
| **0x514** | ~~int~~ ptr | ~~**AttachedEffectCount**~~ **PlanningToken** | 0 | corrected 2026-05-28: "AttachedEffectCount (int, LOW)" row was WRONG and is superseded. Getter FUN_00705d20: `return *(int*)(this+0x514)` and Setter FUN_00705d10: `*(int*)(this+0x514)=param` confirm a 4-byte pointer. See "Field 0x514: PlanningNodeClass Pointer — Corrected" in the Verification Pass addendum for full evidence — ROOT_CAUSE: INFERENCE_HARDENED | ~~LOW~~ **HIGH** |
| **0x2A8** | ptr | **DisableWeaponLink** | 0 | What_Action_OnObject: if 0x2A8 != 0 AND TechnoType+0x692 == false (not immune), returns NoAction. FUN_007077c0 (Detach): mutually clears — `target[0x2A8] = 0; self[0x2A8] = 0` — proving it's a bidirectional link. RockingUpdate: multiple checks for nonzero; affects rocking physics (disabled units don't rock normally). Load resolves as pointer. This is a link to the TechnoClass that has disabled this unit's weapons/actions (e.g., through a disabling attack). Both sides point to each other. | **HIGH** |

### Partially Resolved Fields

| Offset | Type | Name | Init | Evidence | Confidence |
|--------|------|------|------|----------|------------|
| **0x438** | byte | **Unknown flag** | 0 | Only 2 references in TechnoClass range: constructor init (0), and TechnoTypeClass constructor. Not accessed in any decompiled TechnoClass function. May be unused padding or a rarely-used feature flag. | **LOW** |
| **0x439** | byte | **Unknown flag** | 0 | Only 1 reference: constructor init. Not accessed anywhere in the TechnoClass function range. Likely unused/padding. | **LOW** |
| **0x43A** | byte | **MovementZoneOverride** | 0 | Constructor init 0. FUN_00700600 (What_Action_OnCell) at 0x00700840 and 0x0070087e: reads this byte. Purpose: an override flag checked during cell-action determination. May gate movement-zone-related special actions. | **MED** |
| **0x44C** | byte | **IsAllocated** | 1 | Constructor: `MOV byte ptr [ESI+0x44C], 0x1`. Only other TechnoClass reference is in TechnoTypeClass (different class for DynamicVectorClass). The init-to-1 pattern is unusual. This is likely a "valid/allocated" marker for the embedded DynamicVectorClass at 0x440 — the CanGrow flag that allows the vector to auto-expand. | **MED** |
| **0x44D** | byte | **HasNewPassenger** | 0 | Constructor init 0. MissionClass constructor (0x006f46f3): `CMP byte ptr [ESI+0x44D], BL` and clears to 0 at 0x006f4710. Checked and cleared during mission dispatch — a one-shot flag indicating a new passenger was added since last mission tick. | **MED** |

### Verified Previously-Identified Fields

| Offset | Previous Name | Verified Name | Status |
|--------|---------------|---------------|--------|
| 0x1C8 | IsDeployed | **IsDeployed** | CONFIRMED. TechnoClass::GetByte_0x1C8 simply returns the byte. OnDeployBegin sets it to 1. GetFireError blocks firing when set. |
| 0x1D8 | IsPermaDisguised | **IsPermaDisguised** | CONFIRMED (from Init_Managers in previous sessions). |
| 0x130 | Unknown | **GapShroudCellClass** | Pointer to a CellClass. UpdateCloakShroud and RemoveCloakShroud access it extensively. UnitClass functions (0x739ac0, 0x739cd0) modify it for gap generator units. It's the cell reference used by the gap generator shroud system to track which cell this unit's gap effect is centered on. Detach clears it. Load resolves as pointer. **HIGH** |
| 0x134 | Unknown | **GapShroudActive** | Byte flag, init 0. Set/cleared alongside 0x130 in gap generator UnitClass functions. FUN_006f77b0 reads it. UnitClass::Mission_Deploy_Building reads it. Indicates whether this unit's gap generator effect is currently active. **MED** |

### Complete Pointer Map (from Load Function 0x0070bf50)

The Load function resolves ALL pointer fields in TechnoClass by calling FUN_006cf240 (pointer
swizzle). This gives us the definitive list of every pointer field in TechnoClass:

```
0x21C  Owner (HouseClass*)
0x304  LaserBeamParticle (ParticleSystemClass*)     [+8 consecutive ptrs from 0x304]
0x308  ElectricBoltParticle
0x30C  (unnamed particle ptr)
0x310  DamageParticleSystem2
0x314  BeamParticle3
0x318  (unnamed)
0x31C  (unnamed)
0x320  (unnamed)
0x14C  TechnoTypeClass*
0x2E4  GarrisonBuildingPtr (BuildingClass*)
0x218  WarpState (swizzled — visual state ptr?)
0x500  QueuedEntryTarget (ObjectClass*)
0x2B4  Target (ObjectClass*)
0x2B8  SuspendedTarget (ObjectClass*)
0x2BC  CaptureManager (CaptureManagerClass*)
0x434  SuicideTeamPtr
0x2D0  SpawnManager (SpawnManagerClass*)
0x2D8  SlaveManager (SlaveManagerClass*)
0x2DC  SlaveOwner (TechnoClass*)
0x2C0  MindControlledBy (TechnoClass*)
0x2CC  HijackerInfantry (InfantryClass*)
0x2E0  OriginalOwnerHouse (HouseClass*)
0x2D4  SpawnOwner (TechnoClass*)
0x518  Disguise (TechnoTypeClass*)
0x51C  DisguisedAsHouse (HouseClass*)
0x274  TemporalClass*
0x278  TemporalWarper (TemporalClass* — who is warping this unit)
0x294  AirstrikeClass*
0x1CC  DrainTarget (TechnoClass*)
0x1D0  DrainedBy (TechnoClass*)
0x11C  LinkedTechno (TechnoClass*)
0x2A8  DisableWeaponLink (TechnoClass*)
0x2AC  LocomotorTarget (ObjectClass*)
0x2B0  LinkedBuilding (ObjectClass*)
0x428  ChronoSourceBuilding (BuildingClass*)
0x42C  ChronoSourceHouse (HouseClass*)
0x118  PassengerListHead (FootClass*)
0x324  Wave (WaveClass*)
0x130  GapShroudCellClass (CellClass*)
0x12C  AttachedBeamAnim (AnimClass*)
0x2C8  MCRingAnim (AnimClass*)
0x1D4  DrainAnim (AnimClass*)
```

### Key Behavioral Insights

**Hijacker System (0x2CC + 0x2E0):**
The hijacker/driver infantry system works by storing the infantry pointer at 0x2CC on the
hijacked vehicle. The vehicle's Owner (0x21C) is changed to the hijacker's house, while
the original owner is preserved at 0x2E0. The GetOriginalOwner function (FUN_0070f820)
returns the true owner by checking: (1) mind-controlled → ask CaptureManager, (2) hijacked
→ return 0x2E0, (3) otherwise → return 0x21C. CaptureManager::CanCapture refuses to capture
units that already have a hijacker (0x2CC != 0), treating hijack and mind control as mutually
exclusive ownership-override systems.

**DisableWeaponLink (0x2A8) is bidirectional:**
The Detach function proves this: when detaching from a target, it clears BOTH
`target[0x2A8] = 0` AND `self[0x2A8] = 0`. This means both the disabler and the disabled
unit point to each other. When 0x2A8 is set and the unit's TechnoType doesn't have
ImmuneToPsionics (TechnoType+0x692), the unit cannot perform any action — What_Action returns
0 (NoAction). RockingUpdate also checks this field to modify rocking behavior for disabled units.

**QueuedEntryTarget (0x500):**
Set by TechnoClass::Set_Destination when the destination is a building/transport the unit
should enter. FootClass::TryEnterTransport reads it to check if the queued building is still
valid/alive. If the building dies, the field is cleared and the unit recalculates. The Detach
function also clears this when the building is removed from the game.

---

## ADDENDUM: Inheritance & Unexplored Areas (session 2, iteration 4)

**Research date:** 2026-04-01
**Method:** Ghidra MCP — decompilation of ObjectClass constructor (0x5f3900), ObjectClass::AI
(0x5f3e70), ObjectClass::Save (0x5f6250), ObjectClass::ReceiveDamage (0x5f5390),
ObjectClass::Select (0x5f4520), ObjectClass::ShouldBeOnBridge (0x5f6a70),
MissionClass constructor (0x5b2da0), MissionClass::Mission_Dispatch (0x5b3060),
MissionClass::Queue_Mission (0x5b35e0), MissionClass::GetCurrentMission (0x5b3040),
RadioClass constructor (0x65a750), RadioClass::Transmit_Radio_Impl (0x65a970),
RadioClass::Broadcast_Radio_ToAll (0x65ace0), RadioClass::Tether_Count (0x6b7d80),
FootClass constructor (0x4d31e0), FootClass::Set_NavCom_With_Suspend (0x4d8f40),
FootClass::Set_Destination_Internal (0x4d94b0), FootClass::GetCurrentSpeed (0x4db1a0),
SensorTracker functions (0x4a50a0-0x4a5360), TechnoClass vtable at 0x7f4960 (232 entries).
TechnoClass::Detach (0x7077c0) fully traced for pointer cleanup.

### I1. ObjectClass Inheritance — Complete Field Map (0x00-0xAB)

AbstractClass occupies 0x00-0x23 (36 bytes). ObjectClass extends from 0x24 to ~0xAB.

**Source:** ObjectClass constructor at 0x5f3900 (param_1 is `int*`, multiply index by 4),
ObjectClass::Save at 0x5f6250, ObjectClass::AI at 0x5f3e70,
ObjectClass::ReceiveDamage at 0x5f5390.

| Byte Offset | Type | Name | Init | Evidence | Confidence |
|-------------|------|------|------|----------|------------|
| 0x00 | ptr | **vtable** | class vtable | Standard C++ | HIGH |
| 0x04 | ptr | **vtable_IRTTITypeInfo** | class vtable | Secondary vtable (RTTI-verified: BCD offset 4 → `IRTTITypeInfo`) | HIGH |
| 0x08 | ptr | **vtable_INoticeSink** | class vtable | Secondary vtable (RTTI-verified: BCD offset 8 → `INoticeSink`) | HIGH |
| 0x0C | ptr | **vtable_INoticeSource** | class vtable | Secondary vtable (RTTI-verified: BCD offset 12 → `INoticeSource`) | HIGH |
| 0x10 | int | **UniqueID** | auto-assigned | AbstractClass: unique object ID | HIGH |
| 0x14 | byte(flags) | **AbstractFlags** | varies | AbstractClass: bit 0=IsTechno (verified in Select/UnInit). Bits 1 and 2 labels ("IsObject", "IsFoot") are not binary-verified. | MEDIUM |
| 0x18 | int | **AbstractField_0x18** | varies | AbstractClass | LOW |
| 0x1C | int | **AbstractField_0x1C** | varies | AbstractClass | LOW |
| 0x20 | byte | **DirtyFlags** | varies | AbstractClass: marks object dirty for processing | MED |
| 0x24 | int | **FallingHeight** | 0 | ObjectClass ctor: `param_1[9]=0`. AI: used with 0x2C (FallingRate) for height interpolation during falling state. When 0x8D is set, object is falling: Z = Z + FallingRate each tick, FallingRate decremented by gravity. | HIGH |
| 0x28 | int | **FallingRate_Unused** | 0 | ObjectClass ctor: `param_1[10]=0`. Not clearly used in decompiled functions. May be a TS holdover. | LOW |
| 0x2C | int | **FallingRate** | 0 | ObjectClass ctor: `param_1[0xb]=0`. AI: modified during fall — decremented each tick, clamped to RulesClass+0x7B8/0x7BC limits. When 0x84 (IsFalling) is nonzero, decremented by 1; otherwise, lerped toward gravity. | HIGH |
| 0x30 | ptr | **AttachedSound1** | 0 | ObjectClass ctor: `param_1[0xc]=0`. AI: if nonzero, plays via VocClass::PlayAt using coords 0x9C-0xA4. Destroyed in destructor. Save: serialized via swizzle. | HIGH |
| 0x34 | ptr | **AttachedSound2** | 0 | ObjectClass ctor: `param_1[0xd]=0`. Save: serialized via swizzle. Same pattern as 0x30. | HIGH |
| 0x38 | ptr | **AttachedTag** | 0 | ObjectClass ctor: `param_1[0xe]=0`. ReceiveDamage: checked repeatedly — `if (this->AttachedTag != 0) TechnoClass__ProcessCellAction(...)`. These are map trigger tags that fire events on damage/death. | HIGH |
| 0x3C-0x63 | embedded | **Sound event objects** | varies | Two FUN_00405BE0 objects constructed in ObjectClass ctor. Each ~20 bytes. Used for looping ambient sounds. | MED |
| 0x64 | int | **VoiceIndex** | -1 (0xFFFFFFFF) | ObjectClass ctor: `param_1[0x19]=-1`. AI: if != -1, plays via VocClass::PlayAt at current position, then not cleared (persistent voice). | HIGH |
| 0x68 | byte | **VoicePlayed** | 0 | ObjectClass ctor: `param_1[0x1a]=0`. Guards voice playback. | MED |
| 0x6C | int | **Health** | 0xFF (255, placeholder) | ObjectClass ctor: `param_1[0x1b]=0xFF`. Overwritten by Unlimbo with actual Strength from TypeClass. Ghidra struct confirmed. | HIGH |
| 0x70 | int | **VisualHealth** | 0xFF (same as Health) | ObjectClass ctor: `param_1[0x1c]=0xFF`. Health bar animation: smoothly approaches actual Health. AI_Update transitions this toward Health each tick. | HIGH |
| 0x74 | byte | **IsFalling** | 0 | ObjectClass ctor: `param_1[0x1d]=0`. Save: serialized. AI: when set, object applies gravity physics each tick, modifying 0x2C and Location_Z. | HIGH |
| 0x78 | int | **FallAnimState** | 1 | ObjectClass ctor: `param_1[0x1e]=1`. Related to fall animation phase. | MED |
| 0x7C | int | **GroundType** | 0 | ObjectClass ctor: `param_1[0x1f]=0`. Tracks terrain type for movement sound selection. | MED |
| 0x80 | byte | **DiscoveredByCurrentPlayer** | 0 | ObjectClass ctor: byte init 0. Save: conditionally saved for single-player (GameMode==0 or 5). Select: checked — undiscovered objects cannot be selected. | HIGH |
| 0x81 | byte | **IsDiscoveredByPlayer** | 1 | ObjectClass ctor: byte 0x81=1. AI: gates sound playback — `if (0x81 == 0) play looping sounds`. Select: `if (0x81 != 0) deny selection`. Note: init 1 means "not yet in game" — Unlimbo sets to 0 when placed. | HIGH |
| 0x82 | byte | **InOpenToppedTransport** | 0 | ObjectClass ctor: byte 0x82=0. ShouldBeOnBridge: read. Set_Destination_Internal: if set && destination nonzero, returns early (correct: a unit inside a transport cannot self-issue moves). **Verified via sole writer `TechnoClass::SetInOpenTransport @ 0x00710470` — sets +0x82=1, calls vtable+0x3D0 (Hide), then cell-removal helper. Overloaded as "in transit / contained" marker by chrono-warp and airstrike-delivery code paths. See `BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md` §1.5.** | HIGH |
| 0x83 | byte | **IsSelected** | 0 | ObjectClass ctor: byte 0x83=0. Select: set to 1 on successful selection. | HIGH |
| 0x84 | byte | **IsFallingAnimPhase** | 0 | ObjectClass ctor: byte at param_1[0x21]=0. AI: used with FallingRate for animation direction (ascending vs descending). | MED |
| 0x88 | ptr | **CrashAnimPtr** | 0 | ObjectClass ctor: `param_1[0x22]=0`. AI: when nonzero, sets anim flag `(iVar1 + 0x195) = 0` during fall state cleanup. Crash/fall visual effect. | MED |
| 0x8C | byte | **OnBridge** | 0 | ObjectClass ctor: byte at param_1[0x23]=0. ShouldBeOnBridge: read and compared. Ghidra struct confirmed. | HIGH |
| 0x8D | byte | **IsFallingDown** | 0 | ObjectClass ctor: byte 0x8D=0. AI: gates the entire falling physics block — `if (0x8D == 0) return`. When set, the fall height/rate calculations run. Save: serialized. | HIGH |
| 0x8E | byte | **UnusedFlag_0x8E** | 0 | ObjectClass ctor: byte 0x8E=0. Save: serialized. Not read by any decompiled ObjectClass function. May be used by subclass code. | LOW |
| 0x8F | byte | **ShouldExplodeOnImpact** | 0 | ObjectClass ctor: byte 0x8F=0. AI: `if (0x8F != 0 && Health > 0) → call ReceiveDamage(Health, ...)` — deals remaining health as self-damage on ground impact. This is the "kills on landing" flag for falling objects (parachute failure, bomb impact). Save: serialized. | HIGH |
| 0x90 | byte | **IsAlive** | 1 | ObjectClass ctor: byte at param_1[0x24]=1. Ghidra struct confirmed. ReceiveDamage: checked repeatedly to early-exit if object died during trigger processing. | HIGH |
| 0x94 | int | **PositionRevealTimer** | -1 | ObjectClass ctor: `param_1[0x25]=-1`. Related to shroud reveal timing on placement. | MED |
| 0x98 | byte | **LogicVectorMembership** | 0 | ObjectClass ctor: byte at param_1[0x26]=0. `FUN_0055BAA0` returns early if set, otherwise inserts into the active LogicClass vector and sets it; `FUN_0055BAE0` removes and clears it. This is not `IsOnMap`; it is distinct from `InLimbo` (+0x81), `OnBridge` (+0x8C), and `IsAlive` (+0x90). | HIGH |
| 0x99 | byte | **CanBeTargeted** | 1 | ObjectClass ctor: byte 0x99=1. Init to 1 = targetable by default. | MED |
| 0x9C | int | **Location_X** | NullCoord | ObjectClass ctor: `param_1[0x27]=DAT_00ac1380`. Ghidra struct confirmed. Coords are Leptons. | HIGH |
| 0xA0 | int | **Location_Y** | NullCoord | ObjectClass ctor: `param_1[0x28]=DAT_00ac1384`. | HIGH |
| 0xA4 | int | **Location_Z** | NullCoord | ObjectClass ctor: `param_1[0x29]=DAT_00ac1388`. AI: modified during fall — `Location_Z = iVar3 + iVar1`. | HIGH |
| 0xA8 | ptr | **AnimClass_Ptr** | 0 | ObjectClass ctor: `param_1[0x2a]=0`. Destructor: if nonzero, calls FUN_00556B30 to detach. Likely the primary attached anim (e.g., damage fire, idle anim). | MED |

### I2. MissionClass Fields (0xAC-0xD3)

MissionClass adds ~40 bytes of mission state after ObjectClass.

**Source:** MissionClass constructor at 0x5b2da0, Mission_Dispatch at 0x5b3060,
GetCurrentMission at 0x5b3040, Queue_Mission at 0x5b35e0.

| Byte Offset | Type | Name | Init | Evidence | Confidence |
|-------------|------|------|------|----------|------------|
| 0xAC | int | **CurrentMission** | -1 (NONE) | MissionClass ctor: `param_1[0x2b]=-1`. GetCurrentMission: returns this, or QueuedMission if -1. Mission_Dispatch: big switch on this value (0-0x1F) dispatching to virtual Mission_X functions. | HIGH |
| 0xB0 | int | **PreviousMission** | -1 | MissionClass ctor: `param_1[0x2c]=-1`. Tracks last mission for mission-change detection. | HIGH |
| 0xB4 | int | **QueuedMission** | -1 (NONE) | MissionClass ctor: `param_1[0x2d]=-1`. Queue_Mission: writes here. GetCurrentMission: fallback when CurrentMission == -1. | HIGH |
| 0xB8 | byte | **MissionJustStarted** | 0 | MissionClass ctor: `param_1[0x2e]=0`. Queue_Mission: set to 0 when new mission queued. Tracks whether current mission has been "initialized" by its Mission_X handler. | HIGH |
| 0xBC | int | **MissionParam1** | 0 | MissionClass ctor: `param_1[0x2f]=0`. Mission-specific parameter. | MED |
| 0xC0 | int | **MissionParam2** | 0 | MissionClass ctor: `param_1[0x30]=0`. Mission-specific parameter. AI: set to 1 for IsFalling objects. | MED |
| 0xC4 | int | **MissionTickCounter** | 0 | MissionClass ctor: `param_1[0x31]=0`. AI_Update: incremented each tick (`0xC4 = 0xC4 + 1`) — counts how many ticks the current mission has been running. | HIGH |
| 0xC8 | int | **MissionTimer.StartFrame** | g_CurrentFrame | MissionClass ctor: `param_1[0x32]=g_CurrentFrameCounter`. Mission_Dispatch: set to g_CurrentFrame when mission tick fires. CDTimerClass start frame. | HIGH |
| 0xCC | int | **MissionTimer.Mid** | (uninitialized) | MissionClass ctor: not explicitly initialized. CDTimerClass mid field (stack value). | LOW |
| 0xD0 | int | **MissionTimer.Duration** | 0 | MissionClass ctor: `param_1[0x34]=0`. Mission_Dispatch: set to return value of Mission_X virtual function (delay until next dispatch). | HIGH |

**Mission Dispatch Switch (from Mission_Dispatch 0x5b3060):**

Each case calls a different virtual function at the given vtable offset:

| Mission ID | Name | Vtable Offset | Notes |
|------------|------|---------------|-------|
| 0 | Mission_Sleep | +0x204 | Default for unknown missions too |
| 1 | Mission_Attack | +0x210 | |
| 2 | Mission_Move | +0x22C | |
| 4 | Mission_Guard | +0x230 | |
| 5 | Mission_Enter | +0x21C | |
| 6 | Mission_Enter2 | +0x21C | Same as Enter |
| 7 | Mission_Harvest | +0x240 | |
| 8 | Mission_Retreat | +0x214 | |
| 9 | Mission_Return | +0x218 | |
| 10 | Mission_Patrol | +0x224 | |
| 11 | Mission_Repair | +0x220 | |
| 12 | Mission_Construction | +0x234 | |
| 13 | Mission_Deconstruction | +0x238 | |
| 14 | Mission_Capture | +0x20C | |
| 15 | Mission_Hunt | +0x228 | |
| 16 | Mission_SpyPlant | +0x23C | |
| 17 | Mission_SpyMoney | +0x214 | Same as Retreat |
| 18 | Mission_Paradrop | +0x244 | |
| 19 | Mission_Selling | +0x248 | |
| 20 | Mission_Missile | +0x24C | |
| 21 | Mission_Open | +0x258 | (0x258 = 600 decimal) |
| 22 | Mission_Rescue | +0x250 | |
| 23 | Mission_ParaFeed | +0x208 | |
| 24 | Mission_MissilePrep | +0x254 | |
| 25 | Mission_AreaGuard | +0x25C | |
| 26 | Mission_AreaGuardReturn | +0x260 | |
| 27 | Mission_Unload | +0x264 | |
| 28 | Mission_Deliberate | +0x268 | |
| 30 | Mission_Harmless | +0x26C | |
| 31 | Mission_HarvestAbort | +0x270 | |

### I3. RadioClass Fields (0xD4-0xEF)

RadioClass adds the radio contact array and related fields.

**Source:** RadioClass constructor at 0x65a750, Transmit_Radio_Impl at 0x65a970,
Broadcast_Radio_ToAll at 0x65ace0, Tether_Count at 0x6b7d80.

| Byte Offset | Type | Name | Init | Evidence | Confidence |
|-------------|------|------|------|----------|------------|
| 0xD4 | int | **RadioContactCount_Active** | 0 | RadioClass ctor: `param_1[0x35]=0`. Tether_Count: iterates from 0 to 0xE8 counting tethered contacts. | MED |
| 0xD8 | int | **RadioContactState** | 0 | RadioClass ctor: `param_1[0x36]=0`. | LOW |
| 0xDC | int | **RadioLastContactID** | 0 | RadioClass ctor: `param_1[0x37]=0`. | LOW |
| 0xE0 | ptr | **RadioContactList.vtable** | DynVec vtable | RadioClass ctor: `param_1[0x38]=&PTR_FUN_007e180c`. DynamicVectorClass vtable for the contact list. | HIGH |
| 0xE4 | ptr | **RadioContactList.Data** | alloc(4) | RadioClass ctor: allocates 4 bytes (`operator_new(4)`) and stores at 0xE4. Contact array pointer. Transmit_Radio_Impl reads/writes `param_1[0x39]` as base array. | HIGH |
| 0xE8 | int | **RadioContactList.Capacity** | 1 | RadioClass ctor: `param_1[0x3a]=1`. Broadcast_Radio_ToAll: loops from 0 to this value. Transmit_Radio_Impl: scans for existing contact or empty slot. | HIGH |
| 0xEC | byte | **RadioContactList.CanGrow** | 1 | RadioClass ctor: `param_1[0x3b]=1`. Standard DynVec flag. | HIGH |
| 0xED | byte | **RadioContactList.Initialized** | 0->1 | RadioClass ctor: set 0 then immediately set 1 after alloc. | HIGH |
| 0xEE-0xEF | — | **padding** | — | Alignment to TechnoClass start at 0xF0. | LOW |

**Radio protocol (from Transmit_Radio_Impl):**
- Message 2 (RADIO_TETHER): Searches contact array for empty slot. If target already in list, returns 1 (already tethered). Otherwise sends RADIO_TETHER to target; if target returns 1 (accepted), stores in array.
- Message 3 (RADIO_UNTETHER): Clears matching contact from array, then forwards message.
- All other messages: Forwarded directly to target via vtable+0x194 (Receive_Radio).

### I4. SensorTracker Embedded Object — Complete Layout (0x350-0x36F)

Embedded at TechnoClass+0x350, this is a 32-byte timer+state object used in AI_Update.
All functions use byte offsets relative to the object start (param_1 is `int` in all cases).

**Functions decompiled:**
- FUN_004A50F0 (Constructor): Inits timer and flags
- FUN_004A5110 (IsInPhaseA_AndB): Returns 1 if both flags set
- FUN_004A5130 (IsInPhaseA_Only): Returns 1 if phaseA set, phaseB clear
- FUN_004A5150 (CheckCompletion): Returns 1 if timer expired (progress ratio >= 1.0)
- FUN_004A51B0 (IsInPhaseB_Only): Returns 1 if phaseA clear, phaseB set
- FUN_004A51D0 (IsIdle): Returns 1 if both flags clear
- FUN_004A51F0 (StartPhaseA_B): Starts timer with duration, sets both flags
- FUN_004A5240 (StartPhaseA_Only): Starts timer, sets phaseA only
- FUN_004A5290 (Reverse): Flips phaseB flag, adjusts remaining time to elapsed
- FUN_004A52F0 (GetProgress): Returns (Duration - Remaining) / Duration as float
- FUN_004A5360 (StateTransition): Clears phaseA based on phaseB state

| Internal Offset | Byte Offset | Type | Name | Evidence | Confidence |
|-----------------|-------------|------|------|----------|------------|
| +0x00 | 0x350 | double | **AnimationRate** | StartPhaseA_B/StartPhaseA_Only: `*param_1 = duration * DAT_007e27f8`. Used in Reverse to compute step size. | HIGH |
| +0x08 | 0x358 | int | **Timer.StartFrame** | Constructor: `*(param_1+8) = g_CurrentFrame`. Standard CDTimer start. CheckCompletion reads it. | HIGH |
| +0x0C | 0x35C | int | **Timer.Mid** | StartPhaseA_B: set from stack. Standard CDTimer mid. | MED |
| +0x10 | 0x360 | int | **Timer.Duration** | Constructor: `*(param_1+0x10) = 0`. CheckCompletion/GetProgress: remaining = max(0, Duration - (now - StartFrame)). | HIGH |
| +0x14 | 0x364 | int | **TotalDuration** | Constructor: `*(param_1+0x14) = 0`. StartPhaseA_B: set to ftol(duration). GetProgress: used as denominator for ratio. Reverse: used to compute inverted remaining. | HIGH |
| +0x18 | 0x368 | byte | **PhaseA** | Constructor: `*(param_1+0x18) = 0`. All state query functions check this. When set, object is "active". | HIGH |
| +0x19 | 0x369 | byte | **PhaseB** | Constructor: `*(param_1+0x19) = 0`. Secondary state flag. PhaseA+PhaseB = deploying. PhaseA only = undeploying. PhaseB only = deployed. Both clear = idle. | HIGH |
| +0x1A-0x1F | 0x36A-0x36F | — | **padding** | Alignment to FacingClass at 0x370. | — |

**State machine:**
```
State 0 (Idle):      PhaseA=0, PhaseB=0  → IsIdle() = true
State 1 (Deploying): PhaseA=1, PhaseB=1  → IsInPhaseA_AndB() = true
State 2 (Deployed):  PhaseA=0, PhaseB=1  → IsInPhaseB_Only() = true
State 3 (Reverting): PhaseA=1, PhaseB=0  → IsInPhaseA_Only() = true
```

In AI_Update: `FUN_004A5150()` checks if timer expired, then `FUN_004A5360()` transitions state.
This is used for deploy/undeploy animations (siege tank, etc.) — NOT sensor tracking.
Better name: **DeployAnimTracker** (misnomer corrected from "SensorTracker").

### I5. TechnoClass Vtable — Complete Map (232 entries)

Vtable at **0x007F4960**. 232 entries (0x000-0x39C), ending at 0x007F4D5C + 4.

Key entries with verified names (vtable offset → address → name):

**AbstractClass virtuals (0x000-0x03C):**

| VTable | Address | Name | Evidence |
|--------|---------|------|----------|
| +0x000 | 0x410260 | AbstractClass::QueryInterface | IUnknown |
| +0x004 | 0x410300 | AbstractClass::AddRef | IUnknown (stub, returns 1) |
| +0x008 | 0x410310 | AbstractClass::Release | IUnknown (stub, returns 1) |
| +0x00C | 0x4C9150 | AbstractClass::GetClassID (pure) | IPersistStream — NOT WhatAmI (WhatAmI is at +0x02C) |
| +0x010 | 0x410450 | AbstractClass::IsDirty | IPersistStream — returns `this->Dirty (@+0x20) == 0` |
| +0x014 | 0x70BF50 | **TechnoClass::Load** | Decompiled fully |
| +0x018 | 0x70C250 | **TechnoClass::Save** | Decompiled fully |
| +0x020 | 0x7106E0 | **TechnoClass::ComputeCRC** | |
| +0x024 | 0x6F3F40 | **TechnoClass::Init_Managers** | Decompiled fully |
| +0x028 | 0x7077C0 | **TechnoClass::Detach** | Decompiled fully |
| +0x02C | 0x4C9150 | **WhatAmI** (returns 0, overridden by subclass) | |
| +0x034 | 0x70C270 | **TechnoClass::SaveState** | |

**ObjectClass virtuals (0x038-0x0FC):**

| VTable | Address | Name | Evidence |
|--------|---------|------|----------|
| +0x044 | 0x5F6690 | ObjectClass::GetTypeClass_2 | |
| +0x048 | 0x5F65A0 | ObjectClass::GetCoords | |
| +0x05C | 0x6F9E50 | **TechnoClass::AI_Update** | 625 lines, fully decompiled |
| +0x060 | 0x710410 | TechnoClass::FreeAllMindControlCaptures | |
| +0x064 | 0x6F32D0 | TechnoClass::GetTypeClass | |
| +0x068 | 0x703860 | **TechnoClass::GetVisualState** | Cloak alpha levels |
| +0x06C | 0x5F3E30 | ObjectClass::ObjectAI (base) | |
| +0x070 | 0x700600 | **TechnoClass::What_Action_OnCell** | |
| +0x074 | 0x6FFEC0 | **TechnoClass::What_Action_OnObject** | |
| +0x078 | 0x5F4260 | ObjectClass::Limbo | |
| +0x084 | 0x6F3270 | **TechnoClass::GetTechnoType_Trampoline** | Forwards to vtable+0x88 (`(**(code**)(*this+0x88))()`). Init_Managers (0x6F3F40) reads TechnoTypeClass-only fields off its return value (+0xCA4/+0xCA8/+0xCAC/+0xCB0 weapon burst, +0xD58/+0xD40 spawn/slave manager, +0xD2F/+0xD30 disguise) — returns TechnoTypeClass*, NOT HouseClass* Owner. corrected 2026-07-18: was "TechnoClass::GetOwnerType"; verified via `decompile_function 0x6F3270` + `decompile_function 0x6F3F40` — Ghidra's own current function name at this address already reads `TechnoClass__GetTechnoType_Trampoline` — ROOT_CAUSE: RTTI_LABEL_DRIFT | HIGH |
| +0x088 | 0x4E0130 | (stub — GetObjectType?) | |
| +0x08C | 0x708B30 | TechnoClass::CreateEffectAnim | |
| +0x094 | 0x701140 | TechnoClass::RecalcOccupation | |
| +0x098 | 0x5F42C0 | ObjectClass::RegisterDestruction | |
| +0x09C | 0x7010D0 | TechnoClass::MarkOccupation | |
| +0x0A0 | 0x700C40 | TechnoClass::ClearOccupation | |
| +0x0A8 | 0x5F6C80 | ObjectClass::GetRenderCoords_2 | |
| +0x0AC | 0x41BE00 | ObjectClass::GetRenderCoords | |
| +0x0B0 | 0x6F3AD0 | **TechnoClass::GetFLH** | Weapon firing location |
| +0x0B8 | 0x5F6BD0 | ObjectClass::GetYSort | |
| +0x0BC | 0x5F6A70 | ObjectClass::ShouldBeOnBridge | |
| +0x0DC | 0x5F5280 | ObjectClass::Destroy | |
| +0x0E0 | 0x702D40 | **TechnoClass::RecordKill** | Kill credit + veterancy |
| +0x0E4 | 0x703230 | TechnoClass::OnKilled | |
| +0x0E8 | 0x5F5940 | ObjectClass::Unlimbo | |
| +0x0EC | 0x5F4160 | ObjectClass::DropIn | |
| +0x0F8 | 0x5F65F0 | ObjectClass::UnInit | |
| +0x0FC | 0x703850 | TechnoClass::GetVisualState2 | |

**TechnoClass-specific virtuals (0x100+):**

| VTable | Address | Name | Evidence |
|--------|---------|------|----------|
| +0x100 | 0x7099D0 | **Retaliate_And_Scan** | Passive target acquisition |
| +0x104 | 0x5F4B10 | ObjectClass::DrawAtCoords | |
| +0x108 | 0x5F5B90 | ObjectClass::DrawVoxelShadow | |
| +0x10C | 0x6F60D0 | **TechnoClass::DrawBehind** | |
| +0x110 | 0x6F5190 | TechnoClass::DrawMain | |
| +0x114 | 0x5B3A50 | MissionClass::GetMissionTimerEntry | |
| +0x118 | 0x5F65D0 | ObjectClass::RemoveFromMap | |
| +0x11C | 0x6F4A40 | TechnoClass::Receive_Radio_Entry | |
| +0x120 | 0x70ADC0 | TechnoClass::UpdateSight | |
| +0x124 | 0x6F4A70 | **TechnoClass::Receive_Radio** | |
| +0x128 | 0x5F4730 | ObjectClass::DrawExtras | |
| +0x12C | 0x5F4870 | ObjectClass::DrawInfo | |
| +0x130 | 0x41BE80 | (stub — DrawPipsImpl?) | |
| +0x138 | 0x5F6C30 | ObjectClass::GetCell | |
| +0x13C | 0x6FC030 | **TechnoClass::CanAct** | Permission check |
| +0x148 | 0x6F9DD0 | TechnoClass::OnDamageStateChange | |
| +0x14C | 0x6FBFA0 | TechnoClass::ShouldAutoCloak | |
| +0x150 | 0x5F44A0 | ObjectClass::Deselect | |
| +0x154 | 0x70E2B0 | TechnoClass::ScaleByTemporal1 | |
| +0x158 | 0x70E340 | TechnoClass::ScaleByTemporal2 | |
| +0x15C | 0x70E300 | TechnoClass::ScaleByTemporal3 | |
| +0x160 | 0x41BF40 | **IsInvulnerable** | Checks IronCurtainTimer (0x18C/0x194). Returns true if timer has remaining time. | HIGH |
| +0x164 | 0x6F7970 | TechnoClass::ApplyExperienceBonus | |
| +0x168 | 0x7012C0 | TechnoClass::GetSightRange | |
| +0x16C | 0x701900 | **TechnoClass::ReceiveDamage** | 682 lines, fully decompiled |
| +0x170 | 0x710460 | TechnoClass::FreeAllMindControlCaptures | |
| +0x180 | 0x707DD0 | TechnoClass::ProcessEMP | |
| +0x184 | 0x5B3040 | **MissionClass::GetCurrentMission** | Returns CurrentMission or QueuedMission |
| +0x190 | 0x5F5C20 | ObjectClass::GetHealthPercentage | |
| +0x194 | 0x6F4AB0 | **TechnoClass::Receive_Radio** | |
| +0x1D4 | 0x70C5B0 | **IsWarpingOut** | Returns byte at 0x270 | HIGH |
| +0x1D8 | 0x70C5C0 | **IsBeingWarped** | Returns byte at 0x271 | HIGH |
| +0x1DC | 0x70C5D0 | **HasTemporalWeaponActive** | Checks 0x274 (TemporalClass*) and its target | MED |
| +0x1E0 | 0x70C5F0 | **IsNotWarping** | Returns !(0x270 or 0x271) | HIGH |
| +0x1E4 | 0x705D70 | **GetPipColor** | Returns color scheme entry based on house | HIGH |
| +0x1E8 | 0x5B35E0 | **MissionClass::Queue_Mission** | |
| +0x1EC | 0x5B3570 | MissionClass::GetQueuedMission | |
| +0x1F0 | 0x5B2FD0 | MissionClass::IsCurrentMission | |
| +0x1F4 | 0x7013A0 | TechnoClass::Set_ArchiveTarget_Internal | |
| +0x1F8 | 0x7013E0 | TechnoClass::ClearArchiveTarget | |
| +0x1FC | 0x5B3A10 | MissionClass::HasMissionQueued | |
| +0x200 | 0x4E0140 | (stub) | |

**Mission virtual functions (0x204-0x270):**

| VTable | Mission | Name |
|--------|---------|------|
| +0x204 | 0 | Mission_Sleep |
| +0x208 | 23 | Mission_ParaFeed |
| +0x20C | 14 | Mission_Capture |
| +0x210 | 1 | Mission_Attack |
| +0x214 | 8/17 | Mission_Retreat / Mission_SpyMoney |
| +0x218 | 9 | Mission_Return |
| +0x21C | 5/6 | Mission_Enter |
| +0x220 | 11 | Mission_Repair |
| +0x224 | 10 | Mission_Patrol |
| +0x228 | 15 | Mission_Hunt |
| +0x22C | 2 | Mission_Move |
| +0x230 | 4 | Mission_Guard |
| +0x234 | 12 | Mission_Construction |
| +0x238 | 13 | Mission_Deconstruction |
| +0x23C | 16 | Mission_SpyPlant |
| +0x240 | 7 | Mission_Harvest |
| +0x244 | 18 | Mission_Paradrop |
| +0x248 | 19 | Mission_Selling |
| +0x24C | 20 | Mission_Missile |
| +0x250 | 22 | Mission_Rescue |
| +0x254 | 24 | Mission_MissilePrep |
| +0x258 | 21 | Mission_Open |
| +0x25C | 25 | Mission_AreaGuard |
| +0x260 | 26 | Mission_AreaGuardReturn |
| +0x264 | 27 | Mission_Unload |
| +0x268 | 28 | Mission_Deliberate |
| +0x26C | 30 | Mission_Harmless |
| +0x270 | 31 | Mission_HarvestAbort |

**RadioClass virtuals (0x274-0x284):**

| VTable | Address | Name |
|--------|---------|------|
| +0x274 | 0x65ACB0 | RadioClass::Transmit_Radio_ToFirst |
| +0x278 | 0x65AAA0 | **RadioClass::Transmit_Radio** |
| +0x27C | 0x65A970 | **RadioClass::Transmit_Radio_Impl** |
| +0x280 | 0x65ACE0 | **RadioClass::Broadcast_Radio_ToAll** |
| +0x284 | 0x41BEE0 | (stub — returns 0) |

**TechnoClass combat/cloak virtuals (0x288-0x3C8):**

| VTable | Address | Name | Evidence |
|--------|---------|------|----------|
| +0x288 | 0x70C5A0 | **HasCloakAbility** | Returns byte at 0x3D2 (Cloakable) | HIGH |
| +0x28C | 0x6F3280 | **MISLEADING — not GetOwnerHouse** | corrected 2026-07-18: `decompile_function 0x6F3280` shows a boolean gate — checks `GetCurrentMission()` (vtable+0x184) against 0/6/0x10 and TechnoTypeClass+0xC94 (fetched via vtable+0x84), returns 0/1. It never reads or returns a HouseClass\* — "GetOwnerHouse" is WRONG. Ghidra has not relabeled this function (still `FUN_006f3280`), so no verified replacement name is available yet — left UNVERIFIABLE pending further RE. ROOT_CAUSE: INFERENCE_HARDENED |
| +0x290 | 0x459D80 | (stub) | |
| +0x294 | 0x70BE80 | **CanSelfHeal** | Checks SelfHealing INI flag + veteran/elite abilities. Uses Math::ftol for frame-based tick. Returns true if should heal this frame. | HIGH |
| +0x298 | 0x6F9E10 | TechnoClass::PreAI | |
| +0x29C | 0x41BEF0 | (stub — returns 0) | |
| +0x2A0 | 0x6FBDC0 | **ShouldLoseCloak (CanAutoCloak)** | Complex check: HasCloakAbility, veteran cloak ability, cell visibility, Cloakable flag | HIGH |
| +0x2A4 | 0x6FBC90 | **ShouldUncloak** | Checks targets, fire timer, EMP | HIGH |
| +0x2B0 | 0x70C620 | TechnoClass::SetGhostCell | |
| +0x2B4 | 0x708BC0 | TechnoClass::UpdateRepairSparkle | |
| +0x2BC | 0x70ADA0 | TechnoClass::UpdateRevealRadius | |
| +0x2C0 | 0x708B40 | TechnoClass::CreateDamageAnim | |
| +0x2C8 | 0x6FDA00 | TechnoClass::FireWeaponAt | |
| +0x2CC | 0x707F60 | TechnoClass::GetWeaponRange | |
| +0x2D0 | 0x6F3950 | TechnoClass::GetMaxPassengers | |
| +0x2E0 | 0x70D980 | TechnoClass::OnConvoyAction | |
| +0x2E4 | 0x6F3330 | **TechnoClass::SelectWeaponAgainst** | Weapon selection logic with Gattling | HIGH |
| +0x2E8 | 0x6F3820 | TechnoClass::GetFLH_Secondary | |
| +0x2EC | 0x704350 | TechnoClass::GetPipCount | |
| +0x2FC | 0x70AD50 | TechnoClass::UpdateReveal | |
| +0x300 | 0x6F3D60 | TechnoClass::GetTurretFacing | |
| +0x304 | 0x708C10 | TechnoClass::GetFireAnimState | |
| +0x308 | 0x708D70 | TechnoClass::GetBarrelFacing | |
| +0x30C | 0x707D20 | TechnoClass::UpdateVeteranAbilities | |
| +0x310 | 0x700D10 | TechnoClass::CanFireAtTarget | |
| +0x314 | 0x700D50 | TechnoClass::CanReachTarget | |
| +0x318 | 0x6FCFA0 | **GetROF** | Rate of fire calculation with veteran/gattling modifiers | HIGH |
| +0x31C | 0x707E60 | TechnoClass::GetThreatValue | |
| +0x324 | 0x70D1D0 | TechnoClass::HasWeaponAbility | |
| +0x328 | 0x70D420 | TechnoClass::StopAllTargeting | |
| +0x32C | 0x70D460 | TechnoClass::ClearAllTargets | |
| +0x360 | 0x6FC0B0 | **TechnoClass::GetFireError** | 412 lines, fully decompiled | HIGH |
| +0x364 | 0x6F8DF0 | TechnoClass::DrawFireAnims | |
| +0x368 | 0x6FCDB0 | **TechnoClass::Set_ArchiveTarget** | Target resolution + garrison redirect | HIGH |
| +0x36C | 0x6FDD50 | **TechnoClass::Fire_At** | 919 lines, fully decompiled | HIGH |
| +0x370 | 0x70F850 | TechnoClass::GetFireCoord | |
| +0x374 | 0x7014A0 | TechnoClass::CanEnterCell | |
| +0x378 | 0x70B280 | TechnoClass::UpdateSensorArray | |
| +0x390 | 0x70E120 | TechnoClass::OnFired | |
| +0x394 | 0x70E1A0 | TechnoClass::OnFired2 | |
| +0x398 | 0x70E140 | **GetWeaponPtr** | Gets WeaponStruct for index, with elite override | HIGH |
| +0x39C | 0x41BFA0 | **IsGattling** (stub returns 0) | Overridden in UnitClass/BuildingClass | HIGH |

**Note:** Entries +0x3FC (IsHasOpenTopped), +0x400 (IsGattling), +0x404 (GetGattlingStage),
+0x408 (GetGattlingROFDivisor) at 0x41BFA0/0x41BFB0/0x41BFC0/0x41BFD0 all return 0 in base
TechnoClass. They are overridden in UnitClass (vtable at 0x7F5AB4) and BuildingClass for
actual gattling behavior.

### I6. The 0x304-0x320 Pointer Block — Resolved

From Load (loop of 8 swizzles starting at 0x304), Detach (loop of 8 clears starting at
param_1[0xC1]=0x304), and Fire_At weapon creation:

| Offset | Type | Name | Created By | Weapon Flag | Confidence |
|--------|------|------|------------|-------------|------------|
| 0x304 | ptr | **LaserBeamParticle** | Fire_At | IsLaser (+0x129) | HIGH |
| 0x308 | ptr | **DamageSmokePSystem** | AI_Update (health < threshold) | — (damage-based) | HIGH |
| 0x30C | ptr | **ElectricBoltPSystem** | Fire_At | IsElectricBolt (+0x12A) | HIGH |
| 0x310 | ptr | **DamageSmokePSystem2** | AI_Update (health < threshold, secondary) | — (damage-based) | HIGH |
| 0x314 | ptr | **RadBeamParticle** | Fire_At | IsRadBeam (+0x12D) | HIGH |
| 0x318 | ptr | **Unused/Reserved** | None found in decompiled code | — | MED |
| 0x31C | ptr | **Unused/Reserved** | None found in decompiled code | — | MED |
| 0x320 | ptr | **Unused/Reserved** | None found in decompiled code | — | MED |

**Correction from previous sessions:** 0x308 was labeled "ElectricBoltParticle" and 0x310
"DamageParticleSystem2". Re-analysis of AI_Update shows 0x308 is created from random
particle system selection when health drops below RulesClass+0x1700 threshold — this is the
**damage smoke** effect, not weapon-fired electric bolt. The weapon electric bolt goes to
0x30C based on weapon flag ordering in Fire_At (IsLaser→0x304, +0x12A→0x308 in Fire_At's
"if 0x308 == 0" check, but AI_Update also writes to 0x308 for damage smoke).

The slots 0x318, 0x31C, 0x320 are loaded/swizzled and cleared in Detach loops, but no
creation code was found in any decompiled function (Fire_At, AI_Update, ReceiveDamage,
Init_Managers). These may be used by BuildingClass or other subclass code, or may be reserved
slots in the fixed-size 8-pointer array that are unused in standard YR gameplay.

### I7. FootClass Fields — First 200 Bytes (0x520-0x5E7)

**Source:** FootClass constructor at 0x4d31e0 (param_1 is `int*`),
Set_NavCom_With_Suspend (0x4d8f40), Set_Destination_Internal (0x4d94b0),
GetCurrentSpeed (0x4db1a0).

| Byte Offset | Type | Name | Init | Evidence | Confidence |
|-------------|------|------|------|----------|------------|
| 0x520 | int | **PathStepIndex** | -1 | Ctor: `param_1[0x148]=-1`. Index into pathfinding node list. | MED |
| 0x524 | short | **PathStepData1** | 0 | Ctor: `param_1[0x149]=0` (as short). | LOW |
| 0x526 | short | **PathStepData2** | 0 | Ctor: byte 0x526=0. | LOW |
| 0x528 | short | **PathCell_X** | 0 | Ctor: `param_1[0x14a]=0` (as short). Likely path destination cell X. | MED |
| 0x52A | short | **PathCell_Y** | 0 | Ctor: byte 0x52A=0. Likely path destination cell Y. | MED |
| 0x530 | int | **PathLength** | 0 | Ctor: `param_1[0x14c]=0`. Length of current path node list. | MED |
| 0x534 | int | **PathDataPtr** | 0 | Ctor: `param_1[0x14d]=0`. Pointer to path node array. | MED |
| 0x538 | int | **PathState** | 0 | Ctor: `param_1[0x14e]=0`. Current pathfinding state. | MED |
| 0x53C | byte | **PathInProgress** | 0 | Ctor: `param_1[0x14f]=0`. Flag for active pathfinding. | MED |
| 0x540 | int | **PathRetryCount** | 0 | Ctor: `param_1[0x150]=0`. | LOW |
| 0x558 | short | **WaypointCell1_X** | 0 | Ctor: `param_1[0x156]=0`. | LOW |
| 0x55A | short | **WaypointCell1_Y** | 0 | Ctor: byte 0x55A=0. | LOW |
| 0x55C | short | **WaypointCell2_X** | 0 | | LOW |
| 0x55E | short | **WaypointCell2_Y** | 0 | | LOW |
| 0x560 | short | **WaypointCell3_X** | 0 | | LOW |
| 0x562 | short | **WaypointCell3_Y** | 0 | | LOW |
| 0x564 | short | **WaypointCell4_X** | 0 | | LOW |
| 0x566 | short | **WaypointCell4_Y** | 0 | | LOW |
| 0x568 | int | **WaypointState1** | 0 | Ctor: `param_1[0x15a]=0`. | LOW |
| 0x56C | int | **WaypointState2** | 0 | Ctor: `param_1[0x15b]=0`. | LOW |
| 0x570 | int | **WaypointState3** | 0 | Ctor: `param_1[0x15c]=0`. | LOW |
| 0x578 | int | **MoveSpeedModifier** | 0 | Ctor: `param_1[0x15e]=0`. | LOW |
| 0x57C | int | **MoveTargetPtr** | 0 | Ctor: `param_1[0x15f]=0`. | LOW |
| 0x580 | int | **MovementCounter** | 0 | Ctor: `param_1[0x160]=0`. | LOW |
| 0x584 | int(hi) | **SpeedMultiplier** | 1.0 (0x3FF00000) | Ctor: `param_1[0x161]=0x3FF00000`. High dword of double = 1.0. Speed scaling factor. | HIGH |
| 0x588 | embedded | **TargetList1.vtable** | DynVec vtable | Ctor: `param_1[0x162]=&PTR_FUN_007e91ec`. | HIGH |
| 0x5A0 | int | **PathfindCounter** | 0 | Ctor: `param_1[0x168]=0`. Set_Destination_Internal: cleared to 0. | MED |
| 0x5A4 | int/ptr | **NavCom** | 0 | Set_NavCom_With_Suspend: copied to SuspendedNavCom before override. Set_Destination_Internal: `param_1[0x169] = param_2`. This is the current navigation target (ObjectClass pointer or cell ID). | HIGH |
| 0x5A8 | int/ptr | **SuspendedNavCom** | 0 | Set_NavCom_With_Suspend: `param_1[0x16a] = param_1[0x169]`. Preserves previous NavCom. | HIGH |
| 0x5C4 | int | **MissionTimerA.StartFrame** | g_CurrentFrame | Ctor: `param_1[400]=g_CurrentFrame`. | MED |
| 0x5D1 | byte | **FootFlag_0x5D1** | 0 | Ctor: byte 0x5D1=0. | LOW |
| 0x5D4 | int | **TeamPtr** | 0 | Ctor: `param_1[0x175]=0`. Pointer to TeamClass. Fire_At reads for suicide team. | HIGH |
| 0x5D8 | int | **TeamState1** | 0 | Ctor: `param_1[0x176]=0`. | LOW |
| 0x5DC | int | **TeamState2** | 0 | Ctor: `param_1[0x177]=0`. | LOW |
| 0x5E0 | int | **BaseReturnTimer.Start** | g_CurrentFrame | Ctor: `param_1[0x178]=g_CurrentFrame`. | MED |

### I8. FootClass Fields — Later Range (0x640-0x6B8)

| Byte Offset | Type | Name | Init | Evidence | Confidence |
|-------------|------|------|------|----------|------------|
| 0x648 | int | **MissionTimerB.Start** | g_CurrentFrame | Ctor: `param_1[0x192]`. | MED |
| 0x64C | int | **MissionTimerB.Mid** | — | | LOW |
| 0x650 | int | **MissionTimerB.Duration** | 10 | Ctor: `param_1[0x193]=10`. | MED |
| 0x654 | int | **MissionTimerC.Start** | g_CurrentFrame | | MED |
| 0x658 | int | **MissionTimerC.Mid** | — | | LOW |
| 0x65C | int | **MissionTimerC.Duration** | 0 | | MED |
| 0x660 | int | **StuckTimer.Start** | g_CurrentFrame | | MED |
| 0x664 | int | **StuckTimer.Mid** | — | | LOW |
| 0x668 | int | **StuckTimer.Duration** | 0 | | MED |
| 0x66C | int | **Unused_0x66C** | 0 | | LOW |
| 0x670 | int | **ReturnCoords** | 0 | | LOW |
| 0x674 | ptr | **ILocomotor** | DAT_008b3da8 | Ctor: `param_1[0x19e-0x1a0]=NullCoords`. Actually COM ILocomotion pointer. Set_Destination_Internal calls Head_To_Coord on `param_1[0x19d]`. | HIGH |
| 0x678 | int | **LocomotorData1** | NullCoord | | MED |
| 0x67C | int | **LocomotorData2** | NullCoord | | MED |
| 0x684 | byte | **LocomotorFlags** | 0xFF | Ctor: `param_1[0x1a1]=0xFF`. | MED |
| 0x685 | byte | **IsMoving** | 0 | Ctor: byte 0x685=0. | MED |
| 0x686 | byte | **IsLanding** | 0 | Ctor: byte 0x686=0. | MED |
| 0x687 | byte | **DeferredArrivalHookFlag** | 0 | Ctor: byte 0x687=0. OnArrival clears it and calls vtable `+0x174(&DAT_008B3DA8,1,0)`; stock Unit/Infantry resolve to Scatter. Producers remain deferred. | MED |
| 0x688 | byte | **FootFlag_0x688** | 0 | | LOW |
| 0x689 | byte | **FootFlag_0x689** | 0 | | LOW |
| 0x68A | byte | **FootFlag_0x68A** | 0 | | LOW |
| 0x68B | byte | **FootFlag_0x68B** | 0 | | LOW |
| 0x68C | byte | **FootFlag_0x68C** | 0 | | LOW |
| 0x68D | byte | **FootFlag_0x68D** | 0 | | LOW |
| 0x68E | byte | **FootFlag_0x68E** | 0 | | LOW |
| 0x68F | byte | **FootFlag_0x68F** | 0 | | LOW |
| 0x690 | byte | **FootFlag_0x690** | 0 | | LOW |
| 0x691 | byte | **FootFlag_0x691** | 0 | | LOW |
| 0x694 | int | **DeployAnimPtr** | 0 | Ctor: `param_1[0x1a5]=0`. PerformDeploy: checked for parasite weapon units. | MED |
| 0x698 | int | **FootData_0x698** | 0 | Ctor: `param_1[0x1a6]=0`. Fire_At: set to g_CurrentFrame+0x14 for weapons with IsAttackAndMove. | MED |
| 0x69C | int | **FootData_0x69C** | 0 | Ctor: `param_1[0x1a7]=0`. Fire_At (for buildings/gattling): incremented and modded by gattling stages. | MED |
| 0x6A0 | int | **CrawlTimer.Start** | g_CurrentFrame | | MED |
| 0x6A8 | int | **CrawlTimer.Duration** | 0 | | MED |
| 0x6AC | byte | **skip_head_to_coord_once** | 0 | Set_Destination_Internal clears it and skips target coord fetch plus locomotor `Head_To_Coord` once, after NavCom has already been written. | MED |
| 0x6AD | byte | **deploy_or_locomotor_piggyback_active** | 0 | Ctor: byte 0x6AD=0. Set_Destination_Internal silently rejects non-null destinations while set; null destination with owner `+0x2B0` clears the linked object's `+0x2AC`, clears owner `+0x2B0`, then sets `+0x6AE`. Not the same as `TechnoTypeClass+0x6AD`; exact clear paths require a separate lifecycle audit. | HIGH |
| 0x6AE | byte | **post_deploy_link_cleanup_marker** | 0 | Ctor: byte 0x6AE=0. Set_Destination_Internal sets it after null destination clears the owner/link object deploy-piggyback relationship. | MED |
| 0x6AF | byte | **TurretRateSync** | 0 | Written by `UnitClass::Facing_Update` (0x736990) only — clear at 0x736ad5, set at 0x736b16 from `CDTimerClass::Remaining()` when `Turret=yes && TurretSpins=no && timer>0`. Always 0 for non-turret units. Read by `Receive_Radio(0x16) TIMING_SYNC` and `Receive_Radio(0x17)` scatter-suppression as a "mid-turret-sync" guard. NOT a chrono-state field despite earlier inference. Corrected 2026-05-19 via TECHNOCLASS_0x6AF_CHRONO_STATE_FIELD report. | MED |
| 0x6B0 | byte | **FootFlag_0x6B0** | 0 | | LOW |
| 0x6B1 | byte | **FootFlag_0x6B1** | 0 | | LOW |
| 0x6B2 | byte | **FootFlag_0x6B2** | 0 | | LOW |
| 0x6B3 | byte | **FootFlag_0x6B3** | 0 | | LOW |
| 0x6B4 | byte | **FootFlag_0x6B4** | 0 | | LOW |
| 0x6B5 | byte | **FootFlag_0x6B5** | 0 | | LOW |
| 0x6B6 | byte | **IsDeploying** | 1 | Ctor: byte 0x6B6=1. PerformDeploy: set to 1 during deploy transition. Note: init 1 may mean "not yet deployed" or "initial state". | MED |
| 0x6B7 | byte | **FootFlag_0x6B7** | 0 | | LOW |
| 0x6B8 | byte | **FootFlag_0x6B8** | 0 | | LOW |

### I9. "SensorTracker" Renamed to DeployAnimTracker

Previous sections (A4, earlier addenda) referred to the embedded object at 0x350 as
"SensorTracker" based on speculative naming. Decompilation of all 10 functions in the
0x4A50xx range reveals this is actually a **deploy/undeploy animation state machine** with
a timer and two phase flags. It has no relation to radar sensors or sensor arrays.

**Corrected name:** DeployAnimTracker (0x350-0x36F, 32 bytes)

The object tracks animation progress for deploy/undeploy transitions:
- `StartPhaseA_B(duration)` → begins deploy animation
- `CheckCompletion()` → returns true when timer expires
- `StateTransition()` → advances from deploying→deployed or reverting→idle
- `GetProgress()` → returns 0.0-1.0 float for interpolated animation
- `Reverse()` → flips direction (deploy↔undeploy) and adjusts remaining time

### I10. Corrections to Previous Sections

**0x308 identity ambiguity:** Both Fire_At and AI_Update write to 0x308. Fire_At creates
it from weapon flag +0x12A (electric bolt weapon). AI_Update creates it as damage smoke
particle when health drops. These are different creation paths for the same pointer slot —
the object is a ParticleSystemClass either way. The slot holds whichever was created more
recently; weapon-created particles persist until weapon stops firing, damage particles persist
until health recovers. This dual-use is intentional — a unit can only have one active
particle effect per slot.

**Section A6 (0x114) correction:** Was labeled "AmmoCount" at HIGH confidence.
Re-examination shows 0x114 = param_1[0x45] * 4 = byte offset 0x114. In What_Action_OnObject
for infantry, this is checked as `this+0x114 > 0` for deploy-gated abilities. This is
consistent with the "ShotCount" label from iteration 1 (ammo/charge count for infantry
special abilities like C4 placement). The "AmmoCount" label was less precise.

### I11. Summary Statistics

**Total fields now mapped in TechnoClass (0x00-0x51F):** ~95% at MED+ confidence
**Total fields mapped in FootClass (0x520-0x6B8):** ~60% at MED+ confidence
**Vtable entries mapped:** 232/232 (100% addresses), ~80 named with HIGH confidence
**Remaining unknowns:** 0x318/0x31C/0x320 particle slots (likely subclass-only or unused),
~20 FootClass byte flags at LOW confidence, a few ObjectClass embedded sound objects.

### Functions Decompiled This Session (25+)

ObjectClass__Constructor (0x5f3900), ObjectClass__AI (0x5f3e70),
ObjectClass__Save (0x5f6250), ObjectClass__ReceiveDamage (0x5f5390),
ObjectClass__Select (0x5f4520), ObjectClass__ShouldBeOnBridge (0x5f6a70),
ObjectClass__GetHealthRatio (0x5f5c60),
MissionClass__Constructor (0x5b2da0), MissionClass__Mission_Dispatch (0x5b3060),
MissionClass__GetCurrentMission (0x5b3040), MissionClass__Queue_Mission (0x5b35e0),
RadioClass__Constructor (0x65a750), RadioClass__Transmit_Radio_Impl (0x65a970),
RadioClass__Broadcast_Radio_ToAll (0x65ace0), RadioClass__Tether_Count (0x6b7d80),
FootClass__Constructor (0x4d31e0), FootClass__Set_NavCom_With_Suspend (0x4d8f40),
FootClass__Set_Destination_Internal (0x4d94b0), FootClass__GetCurrentSpeed (0x4db1a0),
FUN_004A50F0-FUN_004A5360 (10 DeployAnimTracker functions),
TechnoClass__IsIronCurtainActive (0x41bf40), TechnoClass__IsWarpingOut (0x70c5b0),
TechnoClass__IsBeingWarped (0x70c5c0), TechnoClass__IsNotWarping (0x70c5f0),
TechnoClass__HasStealthAbility (0x70c5a0), TechnoClass__CanSelfHeal (0x70be80),
TechnoClass__CanAutoCloak/ShouldLoseCloak (0x6fbdc0),
TechnoClass__GetROF (0x6fcfa0), TechnoClass__GetWeaponPtr (0x70e140),
TechnoClass__GetPipColor (0x705d70),
FUN_006fd620 (laser/beam weapon creation),
FUN_0041BFB0/0041BFC0/0041BFD0 (IsGattling/GetStage/GetDivisor stubs)
