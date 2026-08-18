# Weapon Charge, Muzzle Flash, FLH

## Overview

**Player-visible effect:** when Prism Tower or Tesla Coil fires, the
**charge-up animation** plays first (the prism focuses, the tesla coil
sparks) for ~1.9 seconds before the actual beam launches. Other units
fire on the same tick they decide to fire — there's no charge delay,
but a **muzzle flash animation** (`Anim=`) appears at the **fire location
+ height** (FLH) the instant the bullet launches and plays through its
own `Rate=`-driven frame loop.

**Mechanism in plain terms:** there are two distinct "charge"
mechanisms in gamemd.exe and they look similar but are unrelated:

1. **`Charges=yes`** (per-`[Weapon]`) — a *gate* flag used by Tesla
   units to mark that the weapon uses limited per-firing charges. It
   does **not** add a delay before firing. The actual visual "charging"
   for Tesla-flavored units happens via #2.
2. **`IsAnimDelayedFire=yes` + `DelayedFireDelay=N`** (per-`[BuildingArt]`)
   — the *real* charge-delay mechanism. When set on a building's art
   section, `Mission_Attack` plays the building's `SpecialAnim` instead
   of firing immediately, and `GetFireError` blocks for `N` ticks while
   the anim plays. After `N` ticks, the bullet finally launches. Only
   Prism Tower (`[GAPRIS]`) and Tesla Coil (`[NATSLA]`) use this in
   shipping YR; both set `DelayedFireDelay=28` (≈1.9 s at the 15-FPS
   logic baseline).

The **muzzle flash** is a separate animation spawned at FLH on every
shot. The FLH triplet `(X, Y, Z)` lives in `[BuildingArt]` /
`[UnitArt]` / `[InfantryArt]` and tells the engine "spawn the muzzle
flash, place the bullet origin, and draw the laser starting point at
these coordinates relative to the firing unit's center." There are up
to 6 FLH triplets per type (Primary / Secondary × Standard / Elite +
two reserve slots), plus a `PrimaryFireDualOffset=` flag that
alternates which barrel of a 2-barrel unit fires next.

Bullet trajectory, projectile speed, and end-of-flight detonation are
covered by [animation-rate-delay.md](animation-rate-delay.md) (impact
animations) and a separate (not-yet-written) projectile-trajectory
doc. This doc focuses on the **time-between-trigger-and-bullet-launch**
and the **muzzle-flash visual timing**.

The clocks are:
- **Charge delay** (`DelayedFireDelay`) → **game-tick** (counted in
  `g_CurrentFrameCounter` units, scales with GameSpeed)
- **Muzzle flash animation** → **game-tick** (standard AnimClass:
  see [animation-rate-delay.md](animation-rate-delay.md))
- **Laser visual duration** (`LaserDuration`) → **game-tick** (default
  10 ticks, see "Hardcoded constants" below)

---

## INI surface

### `rulesmd.ini` — per-`[Weapon]` (charge-related flags)

```ini
[GADummyWeapon]
...
LaserDuration=10           ; ticks the laser beam visual remains on screen
Charges=yes                ; gate flag for Tesla weapons (limited per-firing charges)
```

| Key | Type | Default | WeaponTypeClass byte offset | Notes |
|---|---|---|---|---|
| `Charges=` | bool | `false` | `0x148` | Gate for "this weapon uses limited charges" (Tesla units consume charges from the building when firing). **Does not** add a fire delay. |
| `LaserDuration=` | int8 | **`10`** | `0x14E` | Game ticks the laser/beam visual remains on screen after the shot fires. Affects `IsLaser`, `DiskLaser`, `IsRadBeam`, `DrawBoltAsLaser` visuals. |
| `Bright=` | bool | `false` | `0x12F` | Weapon flash illuminates surrounding area for one tick |
| `IsLaser=` / `DiskLaser=` / `IsBigLaser=` / `IsLine=` / `IsRailgun=` / `IsRadBeam=` / `IsRadEruption=` / `IsMagBeam=` / `IsElectricBolt=` / `DrawBoltAsLaser=` / `IsHouseColor=` | bool | various | `0x149..0x153, 0x15C` | Visual style flags. Each enables a different post-fire visual; only the duration of those visuals is timing-relevant (`LaserDuration` for most; see "Hardcoded constants" for IsElectricBolt). |
| `LaserInnerColor=` / `LaserOuterColor=` / `LaserOuterSpread=` | RGB | `0,0,0` | `0x120..0x128` | Color triplets for the laser visual. Not timing. |
| `UseFireParticles=` / `UseSparkParticles=` | bool | `false` | `0x129..0x12A` | Spawn a persistent ParticleSystemClass attached to the unit; particle system has its own lifetime not gated by this weapon's ROF |
| `AttachedParticleSystem=` | `ParticleSystemTypeClass*` | NULL | `0x11C` | Particle-system type bound to weapon (used by UseFireParticles/UseSparkParticles/IsRailgun) |
| `IsSonic=` | bool | `false` | `0x130` | Spawns a `WaveClass` (0x240 bytes) at fire time; wave has its own per-frame propagation and damage cadence |

Cross-ref: per-tick burst dispatch and the ROF cooldown that gates
*how often* a charge is initiated is in
[weapon-rof-burst.md](weapon-rof-burst.md). This doc covers what
happens **inside** the charge-and-fire window.

### `artmd.ini` — per-`[BuildingArt]` (charge delay + FLH)

```ini
[NATSLA]                                ; Tesla Coil
PrimaryFireFLH=50,0,300
SecondaryFireFLH=50,0,300
NewTheater=yes
ActiveAnim=NATSLA_A
ActiveAnimDamaged=NATSLA_AD
ActiveAnimZAdjust=-30
ActiveAnimPowered=no   ; SJM: This means anim goes away when underpowered
ActiveAnimPoweredLight=yes ; SJM: This means anim goes away when underpowered
SpecialAnim=NATSLA_B
SpecialAnimDamaged=NATSLA_BD
SpecialAnimZAdjust=-30
SpecialAnimPowered=no  ; SJM: This means anim goes away when underpowered
SpecialAnimPoweredLight=yes ; SJM: This means anim goes away when underpowered
IsAnimDelayedFire=yes  ; SJM: Firing anim (SpecialAnim) delays firing of weapon
DelayedFireDelay=28    ; SJM: Must match playback of anim, and ideally audio too
```

```ini
[GAPRIS]                                ; Prism Tower
;PrimaryFirePixelOffset=-6,-60
PrimaryFireDualOffset=true
PrimaryFirePixelOffset=0,-4
PrimaryFireFLH=0,0,378
NewTheater=yes
ActiveAnim=GAPRIS_B
ActiveAnimDamaged=GAPRIS_BD
ActiveAnimZAdjust=-83
SpecialAnim=GAPRIS_A
SpecialAnimDamaged=GAPRIS_AD
SpecialAnimZAdjust=-35
SpecialAnimPowered=no
SpecialAnimPoweredLight=yes
IsAnimDelayedFire=yes
DelayedFireDelay=28
```

| Key | Type | Default | BuildingTypeClass byte offset | Notes |
|---|---|---|---|---|
| `IsAnimDelayedFire=` | bool | `false` | `0x16A7` | Building plays `SpecialAnim` then waits `DelayedFireDelay` ticks before bullet launches |
| `DelayedFireDelay=` | int | `0` | `0x16EC` | Game ticks of charge-up delay (Prism/Tesla = `28`) |
| `PrimaryFireFLH=X,Y,Z` | int×3 | `0,0,0` | (parsed inside `TechnoTypeClass::ReadINI` at offset `+0x89C..+0x8A4` for *all* TechnoTypes including buildings) | Primary-weapon muzzle position in leptons (Z = altitude) |
| `SecondaryFireFLH=X,Y,Z` | int×3 | `0,0,0` | (parsed similarly, separate offset block) | Secondary-weapon muzzle position |
| `ElitePrimaryFireFLH=X,Y,Z` | int×3 | inherit Primary | (parsed similarly, separate offset block) | Veteran/Elite primary FLH override |
| `EliteSecondaryFireFLH=X,Y,Z` | int×3 | inherit Secondary | (parsed similarly) | Veteran/Elite secondary FLH override |
| `PrimaryFireDualOffset=` | bool | `false` | (within FLH parse block) | When set, the primary FLH alternates left/right per shot — implements visual 2-barrel cycling for Prism Tower / similar |
| `PrimaryFirePixelOffset=dx,dy` | int×2 | `0,0` | (within FLH parse block) | Screen-space pixel offset added to FLH at draw time (for fine art alignment) |
| `SecondaryFirePixelOffset=dx,dy` | int×2 | `0,0` | (within FLH parse block) | Same for secondary |
| `PBarrelLength=` | int | `0` | `+0x8A8` (verified via direct memory read of `TechnoTypeClass::ReadINI`) | "Primary Barrel Length" — used by particle-system tube-effect drawing |
| `PBarrelThickness=` | int | `0` | `+0x8AC` | "Primary Barrel Thickness" |
| `ActiveAnim=` / `ActiveAnimDamaged=` | `AnimTypeClass*` | NULL | (in BuildingArt parse path) | Idle animation while building is active |
| `SpecialAnim=` / `SpecialAnimDamaged=` | `AnimTypeClass*` | NULL | (in BuildingArt parse path) | Charge-up animation triggered by `Mission_Attack` when `IsAnimDelayedFire=yes` |
| `ActiveAnimPowered=` / `SpecialAnimPowered=` | bool | `true` | (in BuildingArt parse path) | When `no`, anim disappears under low power. Used for Tesla Coil/Prism Tower so they go visually dead when powered down. |
| `ActiveAnimZAdjust=` / `SpecialAnimZAdjust=` | int | `0` | (in BuildingArt parse path) | Per-anim Z-sort adjustment in pixels |
| `ActiveAnimPoweredLight=` / `SpecialAnimPoweredLight=` | bool | `false` | (in BuildingArt parse path) | When `yes`, anim provides terrain illumination while playing |
| `DamageFireOffset0=x,y` | int×2 | `0,0` | (per-building damage smoke offset) | Pixel offset where the "damaged/burning" smoke anim attaches; not weapon timing |
| `CanBeHidden=` | bool | `true` | (in BuildingArt parse path) | Can be obscured by terrain/shroud at draw time |
| `OccupyHeight=` | int | `0` | (in BuildingArt parse path) | Building's height in tiles; affects shroud/draw layering, not firing |

`IsAnimDelayedFire` is read at `0x004611aa`; `DelayedFireDelay` is
read immediately after, at `0x004611c7` — both directly inside
`BuildingTypeClass::ReadINI` itself (function body `0x0045fe50`–
`0x00464a62`). (corrected 2026-07-12: was "`IsAnimDelayedFire`
identified at `0x004611c7` inside `BuildingTypeClass::ReadINI_Water`
(a sub-routine)" — no `ReadINI_Water` function exists in the binary
(`search_functions_enhanced name_pattern=ReadINI_Water` → 0 results),
and `0x004611c7` is the `DelayedFireDelay` push site, not
`IsAnimDelayedFire`'s; verified via `get_function_by_address
0x004611c7` [resolves inside the same `BuildingTypeClass::ReadINI`
body, not a distinct sub-routine] and `get_assembly_context` on xrefs
`004611aa`/`004611c7` — INFERENCE_HARDENED.) The "+0x16a7" and
"+0x16ec" byte offsets are CONFIRMED correct (`MOV byte ptr
[EBP+0x16a7],AL` / `MOV dword ptr [EBP+0x16ec],EAX` at those exact
sites) and remain cross-referenced in
[PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md](../PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md)
§ O7.

### `artmd.ini` — per-`[VehicleArt]` / `[InfantryArt]` (FLH only)

```ini
[E1]                                    ; Infantry example
;...
PrimaryFireFLH=60,0,100
;...
```

```ini
[FV]                                    ; Vehicle example (IFV)
;...
PrimaryFireFLH=100,-25,135
SecondaryFireFLH=100,-25,135
;...
```

Vehicles and infantry use the same FLH keys (`PrimaryFireFLH`,
`SecondaryFireFLH`, `ElitePrimaryFireFLH`, `EliteSecondaryFireFLH`,
`PrimaryFireDualOffset`, `PrimaryFirePixelOffset`,
`SecondaryFirePixelOffset`, `PBarrelLength`, `PBarrelThickness`) —
all parsed by the shared `TechnoTypeClass::ReadINI` at `0x00715da1`
into the same byte offsets on the TechnoType.

There is **no** `IsAnimDelayedFire` / `DelayedFireDelay` for vehicles
or infantry. The charge-delay mechanism is exclusively for buildings.

### `artmd.ini` `FiringFrames` (Unit-type art) and InfantryTypeClass fire-sync fields

(corrected 2026-07-12: this subsection previously presented
`FiringFrames` as a `rulesmd.ini` per-`[InfantryType]` key with a
fabricated `[E1]` INI example. Verified this session by grepping every
file in `ini/`: `FiringFrames` appears ONLY in `art.ini` / `artmd.ini`,
never in `rules.ini` / `rulesmd.ini`, and only on non-infantry
sections — `[DLPH]` Dolphin, `[DRON]` Terror Drone, `[SQD]` Squid (all
`UnitType`, not `InfantryType`). `FiringSyncFrame0/1` and
`BurstDelay0/1` do not appear in ANY shipping `ini/` file for any unit
— the quoted `[E1]` block with these keys set does not exist in stock
content. Root cause: INFERENCE_HARDENED — an illustrative example was
written as if verified and never checked against the actual INI.)

```ini
; ini/artmd.ini — FiringFrames is a Unit-type art key, not infantry:
[DLPH]                                  ; allied dolphin
WalkFrames=6
FiringFrames=6
```

| Key | Type | Default | Byte offset | Notes |
|---|---|---|---|---|
| `FiringFrames=` (`artmd.ini`, Unit-type only) | int | (constructor) | (read at `0x00747809`, inside `UnitTypeClass::ReadINI`, body `0x00747620`–`0x00747eab`) | Sequence-cycle frame count for Unit/creature art (Dolphin, Terror Drone, Squid). Unrelated to `InfantryTypeClass`. |
| `FiringSyncFrame0=` (InfantryTypeClass, unused in shipping INI) | int | `0` | `0xE40` | Animation frame within the firing cycle at which the **shot** dispatches (standing pose) |
| `FiringSyncFrame1=` (InfantryTypeClass, unused in shipping INI) | int | `0` | `0xE44` | Animation frame at which the shot dispatches (prone pose) |
| `BurstDelay0=` / `BurstDelay1=` (InfantryTypeClass, unused in shipping INI) | int | `0` | `0xE48` / `0xE4C` | Per-shot delay between burst shots (see [weapon-rof-burst.md](weapon-rof-burst.md)) |

`FiringSyncFrame0/1` and `BurstDelay0/1` are read inside
`InfantryTypeClass::ReadINI` at `0x005240a0` — a distinct,
correctly-labeled function, confirmed via decompile
(`CCINIClass__ReadInt()` results stored to `param_1+0xe40/0xe44/
0xe48/0xe4c` in that order) and via vtable-slot divergence from
`UnitTypeClass::ReadINI` (both vtables share a prefix of generic
`TechnoTypeClass`-level slots, then diverge at the `ReadINI` slot:
`0x00747620` for the `UnitTypeClass` vtable, `0x005240a0` for the
`InfantryTypeClass` vtable). (corrected 2026-07-12: the earlier
Ghidra-queries-log entry read "`0x00747809` ... misnamed — actually
InfantryTypeClass override" — this was backwards. `0x00747809` is
genuinely inside `UnitTypeClass::ReadINI` and reads the unrelated
`FiringFrames` key; the real `InfantryTypeClass::ReadINI`
(`0x005240a0`) does not read `FiringFrames` at all — RTTI_LABEL_DRIFT.)
Constructor defaults for all four fields are confirmed `0`
(`InfantryTypeClass::Constructor` @ `0x005236a0`:
`param_1[0x390..0x393] = 0`, i.e. `0xE40/0xE44/0xE48/0xE4C`). Since no
stock `[InfantryType]` section overrides them, every shipping infantry
unit uses these zero defaults.

**Critical infantry-only mechanic:** infantry fire is **gated by their
animation frame**, not by the FireTimer alone. `InfantryClass::Fire_At_Target`
@ `0x005206B0` (confirmed via live decompile) checks whether
`this->field_0xF8` (current anim frame index) matches
`FiringSyncFrame0` (standing) or `FiringSyncFrame1` (prone). Only when
the animation frame matches is `vtable+0x3CC` (Fire) actually
dispatched — confirmed directly in the decompile: `if (*(int
*)&param_1->field_0xf8 != iVar7) { return; }` followed by
`(**(code **)(param_1->vtable + 0x3cc))(...)`. Per
[BURST_WEAPON_FIRING_GHIDRA_REPORT.md](../BURST_WEAPON_FIRING_GHIDRA_REPORT.md)
(re-verified verbatim against live decompile this session):

```c
iVar3 = *(int *)&param_1[1].field_0x1a0;       // InfantryType* (this+0x5A0)
cVar1 = param_1[1].field_0x1bb;                 // byte at this+0x5BB — "is-prone" flag
iVar7 = *(int *)(iVar3 + 0xe40);                // FiringSyncFrame0 (standing)
if (cVar1 != '\0')
    iVar7 = *(int *)(iVar3 + 0xe44);            // FiringSyncFrame1 (prone)
```

So for infantry, the **effective** fire cadence is the worse of: the
weapon's ROF, or the infantry's animation period (which is set by the
infantry's `Sequence=` block timings in `[InfantrySequence]`). Because
`FiringSyncFrame0/1` are always `0` in shipping content (see above),
the practical rule for every stock infantry unit is "fire dispatches
only on the tick the firing animation cycle is at frame 0" — there is
no per-unit standing-vs-prone tuning observed in shipping YR/RA2, only
the theoretical capability. The relationship between the weapon's ROF
and how often the animation cycle returns to frame 0 (governed by
`[InfantrySequence]` `Rate=`/frame-count, independent of
`FiringSyncFrame`) still determines the effective cadence; exact
per-unit numbers are unverified this session. Detailed infantry
sequence timing is covered in
[infantry-sequence-timing.md](infantry-sequence-timing.md).

### `rulesmd.ini` — global (`[General]`)

No keys directly control charge time, muzzle-flash duration, or FLH
parsing globally. The relevant globals:

| Key | Where | Notes |
|---|---|---|
| `[General] PrismType=` | `RulesClass+0x49C` | Identifies the prism cascade firing tower type. See [PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md](../PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md). |
| `[General] PrismSupportDelay=` | `RulesClass+0x4A4` | Default `45` ticks — cooldown on a prism support tower after it has emitted a support beam. |
| `[General] PrismSupportDuration=` | `RulesClass+0x4A8` | Default `15` ticks — lifetime of the support-beam laser visual. |
| `[General] PrismSupportMax=` | `RulesClass+0x4A0` | Default `8` — max number of prism support beams a firing tower can accumulate. |
| `[General] PrismSupportModifier=` | (RulesClass field) | Default `100` (%) — per-supporter damage multiplier added. |
| `[General] PrismSupportHeight=` | `RulesClass+0x4AC` | **Dead INI key** — parsed but never read. Do not implement. |

Detailed semantics in [PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md](../PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md);
the timing here is `DelayedFireDelay` (28 ticks) for the charge anim
+ `PrismSupportDelay` (45 ticks) for the support-tower cooldown +
`PrismSupportDuration` (15 ticks) for the support-beam visual.

---

## Hardcoded constants

### `DelayedFireDelay=28` for both Prism Tower and Tesla Coil

Quoted in `artmd.ini`:

```ini
[NATSLA] DelayedFireDelay=28   ; SJM: Must match playback of anim, and ideally audio too
[GAPRIS] DelayedFireDelay=28   ; SJM: Must match playback of anim, and ideally audio too
```

28 game ticks. The exact per-second wall-clock effect varies with
GameSpeed slider — see [game-speed-master-clock.md](game-speed-master-clock.md)
for the conversion table. At the engine's standard `~15 ticks/sec`
logic baseline (slot 4 in the slider, "Slow"), 28 ticks ≈ **1.87 s**
— matches the in-game charge cycle observed for Tesla Coil and Prism
Tower.

The comment line in the INI is operationally meaningful: the
animation's frame count × `Rate=` must equal `DelayedFireDelay`,
otherwise the visual will desync from the bullet launch. For NATSLA_B
(Tesla Coil charge anim), the artmd entry has a specific
frame-count/Rate pair sized to land at 28 ticks; same for GAPRIS_A.

### `LaserDuration=10` (default)

`WeaponTypeClass + 0x14E` (a single byte). Constructor initializes to
**10**. Each laser draw consumes this value as the lifetime of the
`LaserDrawClass` visual. After `LaserDuration` ticks, the laser disappears
even if the firing animation hasn't completed. Used by `IsLaser=yes` /
`DiskLaser=yes` / `IsRadBeam=yes` / `DrawBoltAsLaser=yes` /
`IsElectricBolt=yes`.

The 10-tick default ≈ 0.67 s at the 15-FPS baseline — short enough to
feel like a beam pulse rather than a sustained ray. Prism Tower's
`PrismType` weapon overrides this; cross-ref the prism docs.

### `LaserDrawClass` (0x5C bytes per object)

`Fire_At` Phase 14 (per [FIRE_AT_ANALYSIS.md](../FIRE_AT_ANALYSIS.md)
§ 54): when `IsLaser=yes`, calls `FUN_006fd210` which allocates a
`LaserDrawClass` (0x5C bytes via `FUN_0054fe60`). The laser is
registered into a global laser draw list; per-tick rendering (in
`LaserDrawClass::UpdateAllAI` called from
`LogicClass::PerTickUpdate`) decrements its remaining-life counter
and removes it when it expires.

`+0x20` on the laser: when `IsHouseColor=yes`, set to 1 to enable
house-color tinting at draw time.

### `WaveClass` (0x240 bytes) for sonic weapons

`IsSonic=yes` causes `Fire_At` Phase 13 to allocate a `WaveClass` of
0x240 bytes and store at `this->Wave` (offset deferred — likely
`+0x488` based on Soviet/Allied sonic-disrupter analyses).
**Per-frame propagation** of the sonic wave is independent of the
weapon's ROF — once spawned, the wave travels at its own speed
defined by `BulletTypeClass.Speed` of the projectile and applies
damage at each tile it crosses. The visual fade-out is also internal
to `WaveClass`.

### `IsMagBeam` (Magnetron) — `WaveClass` type 3

Same `WaveClass` allocation but constructed with type parameter `3`
(magnetron). Only created if `this->Wave == NULL` and target is not a
building. The "lift" effect (gradual altitude increase of the targeted
vehicle) is in [magnetron-lift-cycle.md](magnetron-lift-cycle.md) —
not here.

### Muzzle-flash anim selection logic (Fire_At Phase 12)

From [FIRE_AT_ANALYSIS.md](../FIRE_AT_ANALYSIS.md) § 45:

```
- If weapon.Anim count == 8: select by turret facing direction (8-way)
- If weapon.Anim count > 0 but != 8: use the first anim in the list
- If naval unit (vtable+0x400): use weapon.OccupantAnim (+0x110)
- If airstrike (this+0x82) and weapon.OpenToppedAnim (+0x118): use OpenToppedAnim
```

The selected anim is constructed via `AnimClass::Constructor` at the
FLH coords. Its lifetime is governed by its **own** `End=` / `Rate=`
fields (see [animation-rate-delay.md](animation-rate-delay.md)) — the
weapon does not control the muzzle-flash duration directly. Typical
muzzle-flash anims (e.g., `MGUN-N`) are 3–6 frames at `Rate=900` →
~3–6 ticks visible = 0.2–0.4 s.

**Important:** the muzzle flash AnimClass is *attached* to the firing
unit via `FUN_00424b50` for non-buildings. For buildings, the anim's
Z-offset is adjusted but it's spawned as a standalone AnimClass.
"Attached" means the anim's position follows the unit as it moves —
critical for fast-firing weapons on moving units (e.g., V3 launcher
recoiling while reversing).

### `Bright=yes` (single-tick illumination flash)

`WeaponTypeClass + 0x12F`. When set, the area around the muzzle is
illuminated for **one tick** at the moment of fire. Implementation
detail in `Fire_At`'s post-launch path; the bright effect is a
per-tick render flag, not a long-lived animation.

### `IsElectricBolt` (Tesla) — `FUN_006fd620`

`Fire_At` Phase 13 § 50: when `IsElectricBolt=yes`, calls
`FUN_006fd620` which creates an electric bolt visual. Uses three rules
fields:

```c
RulesClass+0x1830   // ElectricBoltColor1
RulesClass+0x1866   // ElectricBoltColor2
RulesClass+0x1869   // ElectricBoltColor3
```

The bolt has its own internal duration (~15 ticks; deferred for
verification). Composes with `LaserDuration` if `DrawBoltAsLaser=yes`
is also set — in that case, the bolt is rendered using the laser
renderer with `LaserDuration` lifetime instead of the internal bolt
duration.

### `IsRadEruption` — `FUN_006fd800` (multi-cell radiation eruption)

Per [FIRE_AT_ANALYSIS.md](../FIRE_AT_ANALYSIS.md) § 52: iterates a
**3×3 grid** around the firer, placing radiation particles with random
offsets `±128 leptons` and random intensities `5..20`. Each particle
is a long-lived AnimClass. Lifetime governed by the per-AnimType
`End=` value. Composes with the `[General] Radiation...` rules
covered in [radiation-tick-rate.md](radiation-tick-rate.md).

### Building multi-barrel cycling (Fire_At Phase 9 § 35)

```c
if (firer.WhatAmI() == 6) {           // building
    this+0x69C++;                      // MultiBarrelIndex
    this+0x69C %= vtable.GetBarrelCount();
}
```

`MultiBarrelIndex` at `+0x69C` is **separate** from `CurrentBurstIndex`
at `+0x3B8`. It cycles every shot (regardless of burst phase) and is
used to pick which FLH triplet to use for the next shot (combined with
`PrimaryFireDualOffset=true`).

### Reserve FLH triplets

Per byte-offset decode of `TechnoTypeClass::ReadINI` at `0x00715da1`,
the FLH parse sequence (each `Read3DPoint` reads X, Y, Z into 3 ints):

| Order | Key | TechnoType byte offset of (X, Y, Z) |
|---|---|---|
| 1 | `PrimaryFireFLH=` | `0x89C, 0x8A0, 0x8A4` |
| 2 | (`PBarrelLength=`) | `0x8A8` — between FLH 1 and FLH 2 in the read sequence |
| 3 | (`PBarrelThickness=`) | `0x8AC` |
| 4 | `SecondaryFireFLH=` | (next ReadString call — offsets deferred but adjacent block) |
| 5 | `ElitePrimaryFireFLH=` | (subsequent block) |
| 6 | `EliteSecondaryFireFLH=` | (subsequent block) |

Verified by direct memory dump of the ReadINI block at `0x00715d80`:

```
8d bd 9c 08 00 00      LEA EDI, [EBP+0x89C]     ; destination for FLH triplet 1
8d b5 f8 01 00 00      LEA ESI, [EBP+0x1F8]     ; source (default value 0,0,0)
57                     PUSH EDI
68 f8 32 84 00         PUSH 0x008432f8           ; "PrimaryFireFLH" string
8b ce                  MOV ECX, ESI
e8 13 f1 e0 ff         CALL ReadString-style FLH parser
```

The triplets are stored as 3 consecutive `int`s (so `(0x89C, 0x8A0,
0x8A4)`) representing X, Y, Z in **leptons** (one cell = 256 leptons).

### `GetFLH` virtual (`vtable + 0xB0`)

Per [FIRE_AT_ANALYSIS.md](../FIRE_AT_ANALYSIS.md) § 16: `Fire_At`
calls `this->vtable.GetFLH(weapon_idx)` to obtain the muzzle position
for the current shot. The base implementation reads from the
TechnoType's stored FLH triplet for the given weapon index (0 =
primary, 1 = secondary). Subclass overrides (BuildingClass,
InfantryClass, AircraftClass) apply:

- **Turret rotation:** rotate the FLH triplet by the current turret
  facing so the muzzle is at the actual barrel tip
- **PrimaryFireDualOffset:** alternate left/right based on
  `MultiBarrelIndex`
- **Veteran/Elite check:** use `ElitePrimaryFireFLH` if the unit is
  veteran/elite (with FIREPOWER veterancy ability) AND the override is
  set
- **PrimaryFirePixelOffset:** add screen-space pixel delta to the
  final draw coords (separately from the world-coord FLH)

The full `GetFLH` virtual implementation is deferred to a focused
FLH-pipeline doc if one becomes needed.

### `RevealOnFire` (post-launch shroud reveal)

`WeaponTypeClass + 0x137`, default **true**. Per
[FIRE_AT_ANALYSIS.md](../FIRE_AT_ANALYSIS.md) § 58: when set and the
firer is player-controlled, the shroud at the target location is
revealed. Reveal radius and persistence are owned by
[shroud-reveal-decay.md](shroud-reveal-decay.md); the `RevealOnFire`
flag just gates whether the reveal happens at all.

### `LastFireFrame` snapshot

`Fire_At` § 59: `this->field_0x120 = g_CurrentFrameCounter` at end of
every fire. Used by EVA voice cooldown and unit voice gating to avoid
"my unit reports for duty" spam after every shot. Cross-ref
[voice-cooldown-overlap.md](voice-cooldown-overlap.md).

---

## Tick / frame topology

### Sequence for a Prism Tower / Tesla Coil shot (with `IsAnimDelayedFire=yes`)

**Tick T:** Mission_Attack selects target, `GetFireError` returns
FIRE_OK. Engine sets the building's animation state to play
`SpecialAnim` (the charge animation) and arms a per-building "delayed
fire" timer.

**Tick T+1 .. T+27:** `GetFireError` returns FIRE_BUSY (charge in
progress). Animation plays. The animation's `Rate=` × `End=` is
calibrated to last exactly 28 ticks. **Tick T+1 through T+27 do not
advance the FireTimer cooldown for this weapon** — the building is in
the "charging" state, not the "cooling" state.

**Tick T+28:** charge complete. `Fire_At` is finally invoked.
Single bullet launches. FireTimer arms with the weapon's `ROF=` value
plus end-of-burst jitter. The `SpecialAnim` returns to `ActiveAnim`
(idle).

**Tick T+28+ROF:** ready to start a new charge cycle.

So **effective wall-clock cadence** for a Prism Tower (ROF=28) is
approximately:
- 28 ticks charge + ~28 ticks ROF + jitter ≈ 56 ticks per shot ≈ 3.7 s
  at 15 FPS

This is the in-game observation: Prism Towers fire every ~3.5–4 s in
normal play.

### Sequence for a normal (non-charge) shot

**Tick T:** `Fire_At` invoked. Muzzle anim spawned at FLH. Bullet
launched. FireTimer armed with `GetROF()` return. Cross-ref
[weapon-rof-burst.md](weapon-rof-burst.md).

**Tick T+ROF:** ready to fire again.

The muzzle flash plays from T to T+(`Anim.End` × `Anim.Rate / 900`)
— typically 3–6 ticks for a `Rate=900` anim of 3–6 frames. Always
shorter than the ROF.

### Laser visual decay

**Tick T:** `IsLaser=yes` weapon fires. `LaserDrawClass` allocated.

**Tick T..T+LaserDuration:** laser drawn each frame. Internal
"strength" / "alpha" interpolates from 1.0 → 0.0 across the
LaserDuration window (verified pattern; exact curve in `LaserDrawClass`
update — deferred).

**Tick T+LaserDuration:** `LaserDrawClass` removes itself from the
global list and frees memory.

### Clock binding summary

| Subsystem | Clock | Driver |
|---|---|---|
| `DelayedFireDelay` countdown | game-tick | per-building "delayed fire" timer (set by `Mission_Attack`, polled by `GetFireError`) |
| `SpecialAnim` charge anim | game-tick | per-building anim state machine (BuildingClass::UpdateAnimation) |
| Muzzle flash AnimClass | game-tick | standard AnimClass (see [animation-rate-delay.md](animation-rate-delay.md)) |
| `LaserDrawClass` lifetime | game-tick | `LaserDrawClass::UpdateAllAI` (called from `LogicClass::PerTickUpdate`) |
| `WaveClass` propagation | game-tick | per-wave update (called from `LogicClass::PerTickUpdate`) |
| Particle systems | game-tick | per-particle-system update |
| `GetFLH` per-shot | game-tick | called inside `Fire_At` (which is called per-tick when not in busy state) |
| `LastFireFrame` snapshot | game-tick | `g_CurrentFrameCounter` snapshot |
| `Bright=` illumination | game-tick (1 tick visible) | per-render flag |

All visual / timing systems here are on the master game-tick clock.
None are wall-clock-driven, so all scale with GameSpeed slider.

---

## Multipliers and modifiers

### `IsAnimDelayedFire=yes`

Building-only. Gates the entire charge-delay path. Without it, the
building fires immediately on `Mission_Attack` (no `SpecialAnim`
played). Setting `IsAnimDelayedFire=no` on Prism Tower / Tesla Coil
would make them fire instantly (an obvious tuning lever but not
shipping behavior).

### `DelayedFireDelay=N`

Ticks of charge delay. **No multipliers apply** — neither veterancy,
nor power state, nor crate powerup scale this value. The animation
just runs for N ticks and the bullet launches.

**Power-off does affect it indirectly:** if the building is powered
down (low power), the `ActiveAnimPowered=no` flag causes the anim
to disappear — but the `SpecialAnim` is governed by its own
`SpecialAnimPowered=no` setting too (for Tesla Coil and Prism Tower:
`SpecialAnimPowered=no` is set → SpecialAnim is also disabled under
low power). And `BuildingClass::CheckLowPower` (cross-ref
[power-state-machine.md](power-state-machine.md)) gates whether the
building can even initiate an attack mission. So in practice: a
powered-down Prism Tower can't fire at all, not "fires slower".

### `PrimaryFireDualOffset=true`

Alternates which FLH coord is used per shot. Set to `true` on Prism
Tower so the visible beam alternates between the tower's two prism
focal points. Driven by `MultiBarrelIndex` (`+0x69C`) cycling per
shot.

### `vtable.GetBarrelCount()` (per-type override)

The denominator for `MultiBarrelIndex % GetBarrelCount()`. For most
buildings = 1 (one barrel = no cycling). For some Allied ships and
Prism Tower = 2. Determines the cycle period.

### Veteran/Elite FLH override

When the firing unit is veteran (or elite) AND it has
`ElitePrimaryFireFLH=` set in art, the FLH used by `Fire_At` switches
to the elite triplet. Used by veteran units that gain visible barrel
upgrades — the new visual barrel tip differs from the base position.

### `Charges=yes` weapon flag

Per-fire, consumes one charge from the building's charge pool. When
the pool is empty, the weapon cannot fire (similar to `Ammo=0` for
units). Charge regen cadence is per-building — deferred to a future
"tesla-charges" doc if needed.

### `LaserDuration=` override

Per-weapon. Longer values make the beam visible for longer; common
values: 10 (default, single-shot pulse), 15 (Prism Tower beam),
20+ (Yuri sonic beam-like effects). Compositing: even if the next
shot fires before the previous laser fades, a new `LaserDrawClass`
is spawned — they overlap visually.

---

## Edge cases

### Pause behavior

Per [logic-vs-render-loop.md](logic-vs-render-loop.md): the building's
`Mission_Attack` runs in the gameplay block (suppressed during pause)
but `BuildingClass::Update` runs in `LogicClass::PerTickUpdate`
(unconditional). The `DelayedFireDelay` countdown is in the building's
update state machine — so during pause, the charge timer continues to
advance. **Player-visible effect:** open the menu while a Prism Tower
is mid-charge, wait 28 ticks (a few seconds wall-clock at default
unpause speed), close the menu — the Prism Tower may have completed
its charge "during" the pause and fire immediately on resume. This is
faithful gamemd behavior; not a bug. Verification of the specific
counter location deferred to a focused power/building-state doc.

### Mid-charge target loss

If the target dies during the 28-tick charge window:
1. Building's mission state stays in "attack" until next
   Mission_Attack tick.
2. Mission_Attack reselects target via SelectTarget; if a new valid
   target exists in range, charge continues and bullet fires at the
   new target on tick T+28.
3. If no valid target, the building cancels the charge — the
   `SpecialAnim` may finish playing visually but no bullet launches.
   On the next valid target, a fresh 28-tick charge begins.

### Mid-charge power loss

If the building loses power mid-charge:
1. `SpecialAnim` disappears (`SpecialAnimPowered=no`).
2. The fire-delay countdown is reset/cancelled.
3. On power restoration, a fresh 28-tick charge must begin.

### Mid-cascade interactions (Prism Tower)

Per [PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md](../PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md):
during the 28-tick charge of the firing tower, `BuildingClass::Update`
checks every 2 ticks for nearby eligible supporters and propagates
support beams. Each supporter then runs its own 28-tick charge as a
chain. The cascade can accumulate up to `PrismSupportMax=8` supporters
before the firing tower's countdown expires. **The 28-tick charge is
not extended by cascade**; it caps the cascade duration.

### `IsLaser=yes` with high ROF

If a weapon has both `IsLaser=yes` and short `ROF=` (e.g.,
`LaserDuration=10` + `ROF=15`), back-to-back lasers overlap visually
for ~5 ticks (one new laser spawns 5 ticks before the previous fades).
Common on Gattling Cannon's Stage 4 weapon (Gattling escalation).

### `IsSonic=yes` weapon with mid-fire WaveClass

If a `IsSonic=yes` weapon fires while its previous wave is still
propagating, the new fire spawns a second `WaveClass` (no
deduplication). The unit can have multiple waves in flight
simultaneously, each damaging cells it crosses.

### Save / load mid-charge

Save state includes the building's animation state and the
delayed-fire countdown (in whatever struct field holds it — likely
within BuildingClass's animation state machine, not on TechnoClass).
On load, the building resumes mid-charge with the correct remaining
ticks.

### Replay determinism

`Random::RandomRanged` (used by the end-of-burst ROF jitter, the
muzzle-flash anim selection of 1-of-many, the Gattling scatter index,
etc.) is deterministic across peers. Charge timing is purely
arithmetic — no RNG involved. So Prism cascades and Tesla shots play
identically on every machine.

### `DecloakToFire=yes` (default) before charge

If the firing unit is cloaked, it decloaks **before** the charge
animation begins. Decloak takes `CloakingSpeed` ticks (per
[cloak-uncloak-delay.md](cloak-uncloak-delay.md)). So a cloaked Tesla
Trooper's effective fire delay = `CloakingSpeed + 28` ticks. (Tesla
Trooper is not a building, doesn't actually use `IsAnimDelayedFire`,
but Prism Tower / Tesla Coil don't cloak — so the composite is
theoretical for those buildings. Listed for completeness.)

### Veteran/elite Prism Tower FLH

Prism Tower doesn't promote (it's a building), so `ElitePrimaryFireFLH`
is irrelevant for it. Veteran-promotion FLH switching is mostly for
vehicles and infantry.

---

## TS-legacy filter

| Field / branch | TS-legacy? | Notes |
|---|---|---|
| `[Weapon] Charges` flag | **Live in YR** | Tesla units. |
| `[Weapon] LaserDuration` | **Live in YR** | All laser weapons. |
| `[Weapon] Bright` | **Live in YR** | Flash illumination. |
| `[Weapon] IsLaser / DiskLaser / IsBigLaser / IsLine` | **Live in YR** | Prism, Floating Disc, etc. |
| `[Weapon] IsElectricBolt / DrawBoltAsLaser` | **Live in YR** | Tesla. |
| `[Weapon] IsRadBeam / IsRadEruption` | **Live in YR** | Desolator. |
| `[Weapon] IsMagBeam` | **Live in YR** | Magnetron. |
| `[Weapon] IsSonic` | **Live in YR** | Sonic Disrupter. |
| `[Weapon] UseFireParticles / UseSparkParticles / IsRailgun` | **Live in YR** | Particle weapons. |
| `[BuildingArt] IsAnimDelayedFire / DelayedFireDelay` | **Live in YR** | Tesla Coil, Prism Tower. |
| `[BuildingArt] SpecialAnim* / ActiveAnim* / *Damaged / *ZAdjust / *Powered / *PoweredLight` | **Live in YR** | Building animation state machine. |
| `PrimaryFireFLH / SecondaryFireFLH / ElitePrimaryFireFLH / EliteSecondaryFireFLH` | **Live in YR** | All units / buildings. |
| `PrimaryFireDualOffset / PrimaryFirePixelOffset / SecondaryFirePixelOffset` | **Live in YR** | Multi-barrel buildings. |
| `PBarrelLength / PBarrelThickness` | **Live in YR (but cosmetic)** | Particle tube length/thickness drawing. |
| `FiringSyncFrame0 / FiringSyncFrame1 / BurstDelay0 / BurstDelay1` (InfantryTypeClass) | **Live in YR (code path); always constructor default `0` — no stock unit sets these keys** | Infantry firing-frame gate. `FiringFrames` (corrected 2026-07-12: is an unrelated `artmd.ini` Unit-type key — DLPH/DRON/SQD — not `[InfantryType]`; see corrected INI-surface section above). |
| `[General] PrismSupportDelay / PrismSupportDuration / PrismSupportMax / PrismSupportModifier / PrismType` | **Live in YR** | Prism cascade mechanic. |
| `[General] PrismSupportHeight` | **DEAD INI KEY in YR** | Parsed but never read — do not implement. |
| `ChargedAnimTime` (per superweapon) | **Live in YR** | Superweapon "about-to-fire" anim threshold (NOT a per-weapon charge delay — see PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md § O6 for correction of an earlier misattribution). |

---

## Cross-references

- [game-speed-master-clock.md](game-speed-master-clock.md) — game-tick
  to wall-clock conversion for `DelayedFireDelay`
- [logic-vs-render-loop.md](logic-vs-render-loop.md) — pause behavior
  of `BuildingClass::Update` (where the charge timer lives) vs.
  `Mission_Attack` (where the trigger fires)
- [animation-rate-delay.md](animation-rate-delay.md) — muzzle-flash
  AnimClass timing (`Rate=` / `End=` for `MGUN-N`, etc.); charge
  animation timing for `SpecialAnim`
- [weapon-rof-burst.md](weapon-rof-burst.md) — what happens *after*
  the charge completes (ROF cooldown begins)
- [infantry-sequence-timing.md](infantry-sequence-timing.md) —
  `FiringSyncFrame0/1` interaction with `[InfantrySequence]` blocks
- [PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md](../PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md)
  — full prism cascade timing (28-tick charge + 45-tick supporter
  cooldown + 15-tick beam-visual lifetime)
- [PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md](../PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md)
  — beyond-cap behavior
- [PRISM_FORWARDING_GHIDRA_REPORT.md](../PRISM_FORWARDING_GHIDRA_REPORT.md)
  — beam-relay multi-fire sequence (uses the building multi-barrel
  shortcut covered in [weapon-rof-burst.md](weapon-rof-burst.md))
- [FIRE_AT_ANALYSIS.md](../FIRE_AT_ANALYSIS.md) — full per-tick
  per-shot dispatch sequence including FLH/GetFLH and muzzle anim
  spawn (Phases 12–14)
- [FIRE_AT_PIPELINE_GHIDRA_REPORT.md](../FIRE_AT_PIPELINE_GHIDRA_REPORT.md)
  — verification companion to FIRE_AT_ANALYSIS
- [WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md](../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md)
  — full weapon-type struct layout
- [cloak-uncloak-delay.md](cloak-uncloak-delay.md) — `DecloakToFire=yes`
  pre-charge decloak delay
- [shroud-reveal-decay.md](shroud-reveal-decay.md) — `RevealOnFire=yes`
  shroud reveal at target location
- [voice-cooldown-overlap.md](voice-cooldown-overlap.md) — `LastFireFrame`
  voice gating
- [radiation-tick-rate.md](radiation-tick-rate.md) — `IsRadEruption`
  per-cell radiation
- [magnetron-lift-cycle.md](magnetron-lift-cycle.md) — `IsMagBeam`
  WaveClass lift cycle
- [building-construction-anim.md](building-construction-anim.md) —
  `ActiveAnim` / `SpecialAnim` general animation slots

---

## Coverage audit

| Item | Disposition |
|---|---|
| `[Weapon] Charges` | Owned here (gate; no delay added) |
| `[Weapon] LaserDuration` | Owned here |
| `[Weapon] Bright` | Owned here |
| `[Weapon] IsLaser / DiskLaser / IsBigLaser / IsLine / IsHouseColor` | Visual-style flags owned here (timing-relevant only via LaserDuration) |
| `[Weapon] IsElectricBolt / DrawBoltAsLaser` | Owned here |
| `[Weapon] IsRadBeam / IsRadEruption` | Cross-referenced to [radiation-tick-rate.md](radiation-tick-rate.md) |
| `[Weapon] IsMagBeam` | Cross-referenced to [magnetron-lift-cycle.md](magnetron-lift-cycle.md) |
| `[Weapon] IsSonic` | Owned here (WaveClass lifetime — internal); per-wave damage cadence is in a future sonic-weapon doc if one becomes needed |
| `[Weapon] UseFireParticles / UseSparkParticles / IsRailgun / AttachedParticleSystem` | Owned here (particle system attached to weapon) |
| `[Weapon] RevealOnFire` | Cross-referenced to [shroud-reveal-decay.md](shroud-reveal-decay.md) |
| `[Weapon] LaserInnerColor / LaserOuterColor / LaserOuterSpread` | Visual color; cross-referenced (not timing) |
| `[BuildingArt] IsAnimDelayedFire / DelayedFireDelay` | Owned here |
| `[BuildingArt] ActiveAnim / ActiveAnimDamaged / ActiveAnimZAdjust / ActiveAnimPowered / ActiveAnimPoweredLight` | Owned here (charge anim infrastructure) |
| `[BuildingArt] SpecialAnim / SpecialAnimDamaged / SpecialAnimZAdjust / SpecialAnimPowered / SpecialAnimPoweredLight` | Owned here |
| `PrimaryFireFLH / SecondaryFireFLH / ElitePrimaryFireFLH / EliteSecondaryFireFLH` | Owned here |
| `PrimaryFireDualOffset / PrimaryFirePixelOffset / SecondaryFirePixelOffset` | Owned here |
| `PBarrelLength / PBarrelThickness` | Owned here (verified offsets in this iteration) |
| `FiringSyncFrame0 / FiringSyncFrame1 / BurstDelay0 / BurstDelay1` (InfantryTypeClass, always default `0` in shipping content) | Owned here (fire-anim gating); detailed sequence in [infantry-sequence-timing.md](infantry-sequence-timing.md) |
| `FiringFrames` (corrected 2026-07-12: `artmd.ini` Unit-type key — DLPH/DRON/SQD — not `[InfantryType]`) | Owned here (file/section attribution corrected) |
| `[General] PrismSupportDelay / PrismSupportDuration / PrismSupportMax / PrismSupportModifier / PrismType` | Cross-referenced to [PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md](../PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md) |
| `[General] PrismSupportHeight` | Owned here (flagged dead) |
| `ChargedAnimTime` | Cross-referenced to [superweapon-recharge.md](superweapon-recharge.md) (NOT a per-weapon charge delay despite the misleading name) |
| `GetFLH` virtual (`vtable+0xB0`) | Owned here (described); detailed subclass overrides deferred to future FLH-pipeline doc |
| `GetBarrelCount` virtual (`vtable+0x408`) | Owned here (described); detailed per-class values deferred |
| `LaserDrawClass` (0x5C bytes) | Owned here |
| `WaveClass` (0x240 bytes) | Owned here (allocation + lifetime); per-tick propagation deferred to sonic/magnetron docs |
| `MultiBarrelIndex` (`+0x69C`) | Owned here |
| `LastFireFrame` (`+0x120`) | Cross-referenced to [voice-cooldown-overlap.md](voice-cooldown-overlap.md) |

---

## Ghidra queries log (this iteration)

| Query | Result |
|---|---|
| Read [FIRE_AT_ANALYSIS.md](../FIRE_AT_ANALYSIS.md) lines 60–260 | Confirmed 14 phases of `Fire_At`; FLH @ vtable+0xB0; muzzle anim selection logic (8-way vs 1-of-list vs naval-OccupantAnim vs OpenToppedAnim); LaserDrawClass (0x5C bytes); WaveClass (0x240 bytes); IsElectricBolt RulesClass+0x1830/0x1866/0x1869 colors; IsRadEruption 3×3 grid with ±128 lepton scatter |
| Read [PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md](../PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md) lines 570–670 | Confirmed `IsAnimDelayedFire` + `DelayedFireDelay` source (artmd, not rulesmd); both NATSLA and GAPRIS use `DelayedFireDelay=28`; ChargedAnimTime is superweapon-only (correction); PrismSupportHeight is dead INI key |
| `grep ^IsAnimDelayedFire artmd.ini` | 2 hits (NATSLA, GAPRIS) — only 2 building types in shipping YR use this charge mechanism |
| `search_strings "DelayedFireDelay"` | 1 hit: `0x0081a74c` |
| `get_xrefs_to 0x0081a74c` | Read by `BuildingTypeClass::ReadINI_Water` at `0x004611c7` |
| `search_strings "IsAnimDelayedFire"` | 1 hit: `0x0081a760` |
| `search_strings "PrimaryFireFLH"` | 2 hits: `0x008432f8` and `ElitePrimaryFireFLH` @ `0x00843288` |
| `get_xrefs_to 0x008432f8` | Read by `TechnoTypeClass::ReadINI` at `0x00715da1` |
| `read_memory 0x00715d80 len=160` | Decoded the FLH ReadString call: `LEA EDI, [EBP+0x89C]` → FLH triplet 1 stored at `TechnoType + (0x89C, 0x8A0, 0x8A4)`; next field at `+0x8A8` is PBarrelLength, `+0x8AC` is PBarrelThickness |
| `read_memory 0x008432d4 len=96` | Confirmed string-table layout: `PBarrelThickness` @ `0x8432d4`, `PBarrelLength` @ `0x8432e8` (corrected 2026-07-12: was `0x8432e4` — `search_strings "^PBarrelLength$"` returns `0x008432e8`; OFFSET_RETYPED_WRONG on the log citation only, the `0x8A8` destination-offset claim itself was already correct), `PrimaryFireFLH` @ `0x8432f8` |
| `search_strings "FiringFrames"` | 1 hit: `0x00845d84` |
| `get_xrefs_to 0x00845d84` | Read by `UnitTypeClass::ReadINI` at `0x00747809` (misnamed — actually InfantryTypeClass override) |
| `search_strings "FiringSyncFrame"` | 1 hit: `0x00845cb0` ("FiringSyncFrame%d") |
| `search_strings "LaserDuration"` | 1 hit: `0x0084931c` |
| `get_xrefs_to 0x0084931c` | Read by `WeaponTypeClass::ReadINI` at `0x007727d5` (already documented at offset `0x14E`) |
| `search_strings "Charges"` | 1 hit: `0x008493c0` (already documented at offset `0x148`) |
| `grep ^PrimaryFireFLH artmd.ini head -20` | Confirmed widely used (Rocketeer, Spy, infantry, IFV, etc.); typical values 50–100 leptons X, 0 Y, 85–378 Z |
| Read `[NATSLA]` and `[GAPRIS]` artmd sections | Confirmed both set `IsAnimDelayedFire=yes` + `DelayedFireDelay=28` + `SpecialAnim*` + `ActiveAnim*` |
