# DISK — Floating Disc (Yuri Tier-3 Aircraft)

**INI ID:** `DISK`
**Display Name:** `Floating Disk` (`UIName=Name:DISK`)
**Side:** Yuri (`Owner=YuriCountry`)
**Category:** **Vehicle / AirPower hybrid** — placed in `[VehicleTypes]` (NOT `[AircraftTypes]`) but with Category=AirPower and ConsideredAircraft=yes. See §7.2.
**Cameo:** `DISKICON` / `DISKUICO` (AltCameo)
**Voxel:** yes

The Floating Disc is Yuri's flagship tier-3 unit — a hovering UFO armed with:
1. A **DiskLaser** primary weapon (90 dmg, 7-cell range, with dedicated
   `DiskLaserClass` sim object for the rotating ring effect)
2. A **DiskDrain** secondary weapon — special anti-building "drain"
   targeting that **siphons power and credits** from enemy power plants /
   refineries. Hardcoded `DrainWeapon=yes` mechanic.

It is one of the most heavily-hardcoded units in YR. Key engineered subsystems:
- Dedicated `DiskLaserClass` allocated each shot (separate from BulletClass)
- `DrainWeapon=yes` triggers building-drain state machine
- Jumpjet locomotor (despite being VehicleType)
- BalloonHover=yes (never lands voluntarily)
- TurretSpins=yes (whole-unit perma-spin animation)
- DeathWeapon=BlimpBombEffect with 10% damage modifier (controlled crash)
- ElitePrimary=DiskLaserE (cosmetic — DiskLaserE is essentially identical to DiskLaser; see §3.4)

> **Cross-references — do not re-derive:**
> - [`DISK_LASER_CLASS_GHIDRA_REPORT.md`](../../DISK_LASER_CLASS_GHIDRA_REPORT.md) (297 lines) — exhaustive DiskLaserClass coverage: struct layout (0x40 bytes, 4-vtable MI/COM-style), vtable@0x007E5FB8, AI function @ 0x004A7340, per-tick lifecycle, BulletAnimTracker integration, 16-entry rotation table, damage timing.
> - [`JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`](../../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md) (900 lines) — jumpjet state machine, balloon-hover branch, wobble math, climb/crash modes.
> - [`BRIDGE_JUMPJET_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`](../../BRIDGE_JUMPJET_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md) — bridge-state interaction, confirms BalloonHover behavior is live in YR.
> - [`LASER_DRAW_CLASS_GHIDRA_REPORT.md`](../../LASER_DRAW_CLASS_GHIDRA_REPORT.md) — LaserDrawClass for the green ring segments.
> - [`MIND.md`](./MIND.md), [`MAGNETRON-UTNK.md`](TBD) — sibling Yuri tier-3 units.

> **TS-legacy filter:** `BalloonHover=yes`, `Crashable=yes`, `TiltCrashJumpjet=yes` are *live YR jumpjet states* (verified per BRIDGE_JUMPJET docs §6). `JumpjetNoWobbles=yes` is live. Locomotor `{92612C46-...}` = JumpjetLocomotionClass — live in YR (Rocketeer/Disc/Lunar/Kirov-class). `;IsRadBeam=yes`, `;IsLaser=true`, `;IsHouseColor=true` on weapons are INI comments (the engine uses `DiskLaser=yes` to route to DiskLaserClass instead of generic laser-draw).

---

## 1. Full `rulesmd.ini` section verbatim

```ini
[DISK];gs changed to K, as per the Master Name Doc (thank god for that, or this would never straighten out)
UIName=Name:DISK
Name=Floating Disk
Prerequisite=YAWEAP,YATECH
Primary=DiskLaser
Secondary=DiskDrain
Strength=600 ;700
Category=AirPower
Armor=light
IsTilter=yes
TooBigToFitUnderBridge=true
TechLevel=2
Turret=yes
TurretSpins=yes;gs unit is one big turret so it can use existing permaspin
Sight=9
Speed=15
CrateGoodie=no
Crusher=no;yes
Owner=YuriCountry
Cost=1750
Soylent=1750
Points=25
ROT=100;gs super fast turn, ie turn on spot
AllowedToStartInMultiplayer=no
IsSelectableCombatant=yes
VoiceSelect=FloatingDiscSelect
VoiceMove=FloatingDiscMove
VoiceAttack=FloatingDiscAttackCommand
VoiceSecondaryWeaponAttack=FloatingDiscSteal
VoiceFeedback=
CrashingSound=FloatingDiscDie
MoveSound=FloatingDiscMoveLoop
CreateSound=FloatingDiscCreated
BalloonHover=yes ; ie never land
Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5} ;Jumpjet
SpeedType=Hover
MovementZone=Fly
MoveToShroud=yes
ThreatPosed=20	; This value MUST be 0 for all building addons
ConsideredAircraft=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=14
DamageParticleSystems=SparkSys,SmallGreySSys
DamageSmokeOffset=100, 100, 275
Weight=3.5
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Accelerates=false
ZFudgeColumn=8
ZFudgeTunnel=13
Size=6
OpportunityFire=yes
ElitePrimary=DiskLaserE
JumpjetSpeed=16 ;params not defined use defaults (old globals way up top called Jumpjet controls)
JumpjetClimb=8 ; 
JumpjetCrash=15 ; Climb, but down
JumpJetAccel=10
JumpJetTurnRate=100;gs superfast turn on spot, from 2
JumpjetHeight=750
JumpjetWobbles=.1 ; ! value of zero stop wobbles?  NO!  Wobbles of zero means div by 0 crash.  "How many wobbles would you like?"  "0"  "You must have wobbles!!!  I kill you!"
JumpjetDeviation=15
JumpjetNoWobbles=yes ; Really small numbers on two lines above don't actually slow down the wobbling since it is the amplitude of a sinusoidal curve
Crashable=yes ; JJ plummets down like aircraft
TiltCrashJumpjet=yes; can handle tilting while falling without freaking out
DeathWeapon=BlimpBombEffect
DeathWeaponDamageModifier=.1;gs needs a death weapon or it will do one laser blast's worth of crash damage.  This gives control
SelfHealing=yes
```

### 1.1 Key-by-key explanation

| Key | Value | Read by | Effect |
|-----|-------|---------|--------|
| `UIName=Name:DISK` | string | AbstractTypeClass | CSF lookup. |
| `Name=Floating Disk` | string | AbstractTypeClass | English fallback (note spelling: "Disk" not "Disc"; inline author comment confirms the K rename per "Master Name Doc"). |
| `Prerequisite=YAWEAP,YATECH` | building list | TechnoTypeClass | Yuri War Factory + Yuri Battle Lab. Strict Yuri tier-3 gating. |
| `Primary=DiskLaser` | weapon | TechnoTypeClass | DiskLaserClass-bearing weapon. See §3.1. |
| `Secondary=DiskDrain` | weapon | TechnoTypeClass | Building-drain weapon (DrainWeapon=yes). See §3.2. |
| `Strength=600 ;700` | hp | TechnoTypeClass | 600 HP. `;700` is a draft-down note. |
| `Category=AirPower` | enum | TechnoTypeClass | UI category. AirPower category affects display ordering and AI targeting profiles. |
| `Armor=light` | enum | TechnoTypeClass | Light armor — vulnerable to AA weapons. |
| `IsTilter=yes` | bool | UnitType @ 0x00747712 | **Body tilts when changing direction.** Verified UnitType scope. Combined with `TiltCrashJumpjet=yes`, allows tilt animation during both flight maneuvers and crash plummet. |
| `TooBigToFitUnderBridge=true` | bool | UnitType @ 0x0074774e | Cannot path under bridges (UnitType scope; matches naval units). |
| `TechLevel=2` | int | TechnoTypeClass | **TechLevel 2**? — surprisingly low for a tier-3 unit. The actual gating is via `Prerequisite=YAWEAP,YATECH` (Battle Lab), which is itself a high-tech building. The low TechLevel here just means "show in build list once prerequisites met". |
| `Turret=yes` | bool | UnitTypeClass | Has a turret (animation purposes). |
| `TurretSpins=yes` | bool | TechnoType @ 0x00713360 | **The turret rotates continuously**, irrespective of target. Verified TechnoType scope. Inline comment: "unit is one big turret so it can use existing permaspin" — the entire disc IS the turret, giving the iconic spinning-UFO effect. |
| `Sight=9` | cells | TechnoTypeClass | 9-cell vision — very wide (vs Carrier's 7, Hornet's 2). |
| `Speed=15` | int | TechnoTypeClass | Fast — flight speed in JumpjetLocomotion units. |
| `CrateGoodie=no` | bool | UnitType @ 0x00747658 | No crate pop. |
| `Crusher=no;yes` | bool | TechnoTypeClass | No crush. `;yes` draft. |
| `Owner=YuriCountry` | country list | TechnoTypeClass | **Yuri only**. The single Yuri faction post-YR. |
| `Cost=1750` | credits | TechnoTypeClass | Premium tier-3 cost. |
| `Soylent=1750` | credits | TechnoTypeClass | Full recycle value. |
| `Points=25` | int | TechnoTypeClass | Score on kill. |
| `ROT=100` | int | TechnoTypeClass | **Maximum turn rate** (100 = essentially instantaneous). Inline: "super fast turn, ie turn on spot". |
| `AllowedToStartInMultiplayer=no` | bool | TechnoTypeClass | Not pre-built. |
| `IsSelectableCombatant=yes` | bool | TechnoTypeClass | Combat unit. |
| `VoiceSelect=FloatingDiscSelect` | sound | TechnoTypeClass | Unique Disc selection voice. |
| `VoiceMove=FloatingDiscMove` | sound | TechnoTypeClass | Unique movement voice. |
| `VoiceAttack=FloatingDiscAttackCommand` | sound | TechnoTypeClass | Unique attack voice (Primary). |
| `VoiceSecondaryWeaponAttack=FloatingDiscSteal` | sound | TechnoType @ 0x00844038 (cheat sheet) | **Separate voice for Secondary weapon** — DiskDrain attacks play `FloatingDiscSteal` voice ("we are stealing your energy" theme). Verified TechnoType scope. This is a unit-level (not weapon-level) per-weapon voice routing. |
| `VoiceFeedback=` | (empty) | TechnoTypeClass | None. |
| `CrashingSound=FloatingDiscDie` | sound | TechnoType @ 0x00712f80 | Sound during the death plummet. |
| `MoveSound=FloatingDiscMoveLoop` | sound | TechnoTypeClass | **Loop sound** (FloatingDiscMoveLoop is a continuous flight hum — `MoveLoop` suffix indicates it loops in soundmd). |
| `CreateSound=FloatingDiscCreated` | sound | TechnoTypeClass | Sound when the unit is built (`FloatingDiscCreated` plays on production-completion). Most units use Rules global "vehicle-built" report; Disc has its own per-unit sound override. |
| `BalloonHover=yes` | bool | TechnoType @ 0x00714d95 | **Never lands voluntarily.** Verified TechnoType scope. Inline comment confirms. The Disc maintains JumpjetHeight=750 constantly; only crashes land it. Cross-ref BRIDGE_JUMPJET_HEIGHT_MINUS_ONE doc §6 (state 4 voluntary-land branch is skipped for BalloonHover units). |
| `Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5}` | CLSID | TechnoTypeClass | **JumpjetLocomotionClass** — the standard jumpjet locomotor (Rocketeer, Lunar Infantry, Floating Disc, Siege Chopper variants, Kirov, SHAD, HIND all use this CLSID). Inline `;Jumpjet` comment. See JUMPJET_LOCOMOTION_CLASS doc. |
| `SpeedType=Hover` | enum | TechnoTypeClass | Hover passability — affects pathfinder zone selection. |
| `MovementZone=Fly` | enum | TechnoTypeClass | Air zone. |
| `MoveToShroud=yes` | bool | TechnoType | Can move through shrouded cells. |
| `ThreatPosed=20` | int | TechnoTypeClass | AI threat weight. |
| `ConsideredAircraft=yes` | bool | TechnoType @ 0x00714fe9 | **Marks this VehicleType as "AI/code-treats-this-as-aircraft"** despite being in `[VehicleTypes]`. Verified TechnoType scope. Affects: AI targeting (lumps it with aircraft for ground-AA targeting decisions), some pathfinder paths, threat profile. |
| `Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` | anim list | TechnoTypeClass | Standard 5-anim destruction set. |
| `MaxDebris=14` | int | TechnoTypeClass | **Up to 14 debris pieces** on destruction (vs 2 for most units — UFO breaks into lots of fragments). |
| `DamageParticleSystems=SparkSys,SmallGreySSys` | particle list | TechnoTypeClass | Damaged emissions. |
| `DamageSmokeOffset=100, 100, 275` | x,y,z leptons | TechnoType (cheat sheet: 0x00713e25) | Position offset (forward 100, side 100, up 275) for smoke emission when damaged. |
| `Weight=3.5` | float | TechnoTypeClass | Fractional weight — uncommon (most units use integer Weight). Used by AI weight calculations and transport-loading. |
| `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` | ability list | TechnoTypeClass | Vet: +HP, +damage, +sight, +speed. No ROF. |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | ability list | TechnoTypeClass | Elite: passive self-heal + ROF added (no SIGHT/FASTER). |
| `Accelerates=false` | bool | TechnoTypeClass | **Constant speed — no acceleration ramp.** Disc moves at top speed instantly. |
| `ZFudgeColumn=8` | int | TechnoTypeClass | Z-buffer render fudge for column terrain (rendering depth tweak). |
| `ZFudgeTunnel=13` | int | TechnoTypeClass | Z-buffer render fudge for tunnel terrain. (Both are render-order corrections to avoid Z-fighting against specific terrain pieces.) |
| `Size=6` | int | TechnoTypeClass | Transport-cost. |
| `OpportunityFire=yes` | bool | TechnoType @ 0x0071483d | Fires on opportunistic targets during movement. |
| `ElitePrimary=DiskLaserE` | weapon | TechnoType @ 0x00712a32 | At elite rank, Primary swaps to `[DiskLaserE]` (which is essentially identical to `[DiskLaser]` — see §3.4 for the comparison). |
| `JumpjetSpeed=16` | int | TechnoType (Rules @ 0x006743D0) | Per-unit override of JumpjetControls global. Top speed in jumpjet units. |
| `JumpjetClimb=8` | int | TechnoType | Climb rate. |
| `JumpjetCrash=15` | int | TechnoType | Descent rate during crash. Inline comment: "Climb, but down". |
| `JumpJetAccel=10` | int | TechnoType | Acceleration ramp for jumpjet motion. |
| `JumpJetTurnRate=100` | int | TechnoType | Air turn rate. Inline: "superfast turn on spot, from 2". |
| `JumpjetHeight=750` | leptons | TechnoType | Cruise altitude — 750 leptons above terrain. |
| `JumpjetWobbles=.1` | float | TechnoType @ 0x0071518b | Wobble amplitude (sinusoidal). Inline comment is a famous Westwood joke about "you must have wobbles!!!  I kill you!" — value of 0 would div-by-0 crash, so a tiny value is used here. |
| `JumpjetDeviation=15` | int | TechnoType | Wobble path deviation. |
| `JumpjetNoWobbles=yes` | bool | TechnoType | **Disables actual wobble rendering.** The .1 amplitude above is residual; this flag suppresses the sinusoidal motion entirely. Inline comment confirms: "Really small numbers ... don't actually slow down the wobbling". Net effect: Disc flies in a perfectly straight line, no UFO-bobbing. |
| `Crashable=yes` | bool | TechnoType | **Plummets down like aircraft on destruction.** Without this, jumpjet units would simply vanish; with it, they fall and detonate on ground impact. |
| `TiltCrashJumpjet=yes` | bool | TechnoType | Can tilt during the falling crash without state-machine breakage. Inline: "can handle tilting while falling without freaking out". |
| `DeathWeapon=BlimpBombEffect` | weapon | TechnoType + Rules (DUAL-READ; cheat sheet) | **On destruction, fires this weapon** at ground impact point. See §3.5. |
| `DeathWeaponDamageModifier=.1` | float | TechnoType (0x00844488) | **Scales the death-weapon damage to 10 %**. Inline comment explains: needs a death weapon (otherwise crash does the disc's own laser damage, which is too low), so use the huge BlimpBombEffect but scaled to 10 % for design control. Net death damage: 250 × 0.1 = 25 base, with BlimpHEEffect's 2-cell radius. |
| `SelfHealing=yes` | bool | TechnoType @ cheat-sheet (~0x00714e2c likely) | **Passive HP regeneration** at all ranks (not just elite). Disc auto-heals over time. |

---

## 2. Full `artmd.ini` section verbatim

```ini
[DISK] ; Yuri Flying Disk
Cameo=DISKICON
AltCameo=DISKUICO
Voxel=yes
TurretOffset=0
PrimaryFireFLH=0,0,75
```

| Key | Value | Notes |
|-----|-------|-------|
| `Cameo=DISKICON` | SHP | Standard cameo. |
| `AltCameo=DISKUICO` | SHP | Alternate (unbuildable / disabled) cameo — shows when prerequisites missing. |
| `Voxel=yes` | bool | `disk.vxl` + `.hva` voxel render. |
| `TurretOffset=0` | int | No turret offset — the turret is centered on the body. Since the disc IS the turret (`TurretSpins=yes` + `Turret=yes`), the offset is zero. |
| `PrimaryFireFLH=0,0,75` | x,y,z leptons | Weapon emerges from disc center (0,0) at height 75 leptons (top of body). |

> **No Voxel.Sequence or Animation block.** The spinning effect comes from `TurretSpins=yes` runtime turret rotation (not a baked SHP/HVA animation).

---

## 3. Weapons

### 3.1 `[DiskLaser]` — primary

```ini
[DiskLaser]
Damage=90
ROF=80
Range=7
Projectile=InvisibleAll
Speed=40
Report=FloatingDiscAttack
Warhead=DiskWH
Bright=yes
;IsRadBeam=yes
LaserInnerColor=216,0,184
LaserOuterColor=80,0,88
LaserOuterSpread=0,0,0
LaserDuration=15
;IsLaser=true	; this flag tells the game to use the special laser draw effect
DiskLaser=yes; new ring draw laser
OmniFire=yes
```

| Key | Effect |
|-----|--------|
| `Damage=90` | 90 base damage. |
| `ROF=80` | ~5.3 sec between shots. |
| `Range=7` | 7-cell engagement range. |
| `Projectile=InvisibleAll` | Bookkeeping invisible projectile (the real visual is the DiskLaserClass ring effect). |
| `Speed=40` | Irrelevant — DiskLaser doesn't use bullet motion. |
| `Report=FloatingDiscAttack` | Attack sound. |
| `Warhead=DiskWH` | Yuri's custom disc warhead. See §3.6. |
| `Bright=yes` | Causes brief light flash at fire position. |
| `;IsRadBeam=yes` | (commented) — would have routed to RadBeamClass; disabled. |
| `LaserInnerColor=216,0,184` / `LaserOuterColor=80,0,88` / `LaserOuterSpread=0,0,0` | RGB color triples for the laser ring (**pink-purple inner, dark purple outer**). Verified read by LaserDrawClass when DiskLaserClass spawns ring segments. |
| `LaserDuration=15` | Ring persists for 15 frames. |
| `;IsLaser=true` | (commented) — would have routed to standard LaserDrawClass. The `DiskLaser=yes` flag below routes to DiskLaserClass instead. |
| `DiskLaser=yes` | **The hardcoded routing flag** — WeaponType @ 0x00772645 (verified). Read into a flag at WeaponType+0x14A (per DISK_LASER_CLASS_GHIDRA_REPORT §Creation). **When this flag is set, `TechnoClass::Fire_At` (0x006FDD50) skips normal BulletClass creation and instead allocates a DiskLaserClass (0x40 bytes) via `new` + Constructor, then calls `BulletAnimTracker::Register`.** Inline comment: "new ring draw laser". |
| `OmniFire=yes` | Can fire in any direction (because TurretSpins=yes means no facing-required). |

> The full DiskLaserClass lifecycle is in [`DISK_LASER_CLASS_GHIDRA_REPORT.md`](../../DISK_LASER_CLASS_GHIDRA_REPORT.md):
> - 0x40-byte struct, 4-vtable MI/COM-style layout
> - Inserted into `g_DiskLaserClass_Array` @ 0x008A020C, ticked via `LogicClass::PerTickUpdate` (slot 23 of vtable = AI function @ 0x004A7340)
> - 16-entry rotation table drives the procedural ring expansion (no per-frame SHP)
> - Damage application is deferred to a SPECIFIC ring step (not bullet-impact timing)
> - ~10 sim frames lifetime
> - On firing-disc death mid-attack, the DiskLaser is cleaned up via the secondary tracker array shared with particle systems / techno-cell-action.

### 3.2 `[DiskDrain]` — secondary (building drain)

```ini
[DiskDrain]
Damage=1
Burst=1
ROF=50
Range=1.5
CellRangefinding=yes
Projectile=InvisibleVertical
Speed=20
Warhead=AntiB
Report=KirovAttack
OmniFire=yes ; Don't need to turn even though I have no turret (Need since if I am directly over my target it will baffle the CloseEnough test for the facing)
FireOnce=yes
DrainWeapon=yes
FireWhileMoving=no
```

| Key | Effect |
|-----|--------|
| `Damage=1` | **Token damage** — actual effect is the drain, not damage. |
| `Burst=1` | One trigger per fire command. |
| `ROF=50` | ~3.3 sec between attempts. |
| `Range=1.5` | Must be very close — disc hovers directly over the target. |
| `CellRangefinding=yes` | WeaponType flag (cheat sheet) — uses cell-based range rather than lepton distance. |
| `Projectile=InvisibleVertical` | Bookkeeping invisible projectile. |
| `Speed=20` | Irrelevant. |
| `Warhead=AntiB` | "Anti-Building" warhead — see §3.6. Verses=0 for almost everything except wood/steel/concrete (100 %), enforcing that DiskDrain only works on buildings. |
| `Report=KirovAttack` | Reuses Kirov's attack sound (deep mechanical drone — also used by BlimpBombEffect, sister weapon). |
| `OmniFire=yes` | Fire in any direction. Inline comment explains: "Don't need to turn even though I have no turret (Need since if I am directly over my target it will baffle the CloseEnough test for the facing)". |
| `FireOnce=yes` | WeaponType flag (cheat sheet — only fires once per attack-target order; ROF gates re-engagement). |
| **`DrainWeapon=yes`** | **WeaponType @ 0x0077223f (verified).** The critical hardcoded flag — when set, the weapon enters a "drain state" instead of dealing damage. The disc anchors over the target building and continuously transfers credits + power from the target's owner to the disc's owner. Drain rate is governed by `[General] DrainMoneyFrameDelay` + `DrainMoneyAmount` (per cheat sheet — read by `RulesClass__ReadCombatDamage` into Rules+0x314/+0x318). The drain animation is `DrainAnimationType` (also from CombatDamage, Rules+0x31c). |
| `FireWhileMoving=no` | Cannot fire while in motion — disc must hover stationary over target. |

> **Drain mechanic details (cross-ref to existing CombatDamage research):**
> - Rules `DrainMoneyFrameDelay` and `DrainMoneyAmount` set per-tick drain rate.
> - Each frame the disc holds the target, credits transfer to disc owner, deducted from target owner.
> - Target power building: if drained while online, contributes to power-output loss (the building visually keeps running but its owner shows reduced surplus).
> - Disc cannot move or attack while draining.
> - Cancelled if disc is destroyed, target dies, or player manually orders disc away.

### 3.3 `[DiskLaserE]` — elite primary swap

```ini
[DiskLaserE]
Damage=90
ROF=80
Range=7
Projectile=InvisibleAll
Speed=40
Report=FloatingDiscAttack
Warhead=DiskWH
Bright=yes
;IsHouseColor=true
LaserInnerColor = 216,0,184
LaserOuterColor = 80,0,88
LaserOuterSpread= 0,0,0
LaserDuration = 15
;IsLaser=true	; this flag tells the game to use the special laser draw effect
DiskLaser=yes; new ring draw laser
OmniFire=yes
```

**Functionally identical to `[DiskLaser]`** — same damage, ROF, range, warhead, colors, duration. The `[DiskLaserE]` block exists only because:
1. The engine routes per-rank weapon via `Primary` vs `ElitePrimary` even if values are the same.
2. The `;IsHouseColor=true` commented note in DiskLaserE (vs `;IsRadBeam=yes` in DiskLaser) suggests it was *originally* going to be house-colored — but the live INI keeps the same purple. Cosmetic intent disabled.

**The DiskLaserE provides NO mechanical upgrade.** Elite Disc's weapon damage boost comes entirely from the **VeteranAbilities/EliteAbilities FIREPOWER multiplier** (standard +10/+25% damage stacking), not from a different weapon. The Disc's elite rank does also add `SELF_HEAL` (passive auto-heal) and `ROF` (faster firing) via abilities — but the weapon block itself is duplicated for no behavioral change.

> If the elite block were truly identical at runtime, the engine could read either; including the duplicate `[DiskLaserE]` ensures correctness even if a future patch tweaks one.

### 3.4 Elite swap summary

| Aspect | Rookie | Veteran | Elite |
|--------|--------|---------|-------|
| Primary weapon | DiskLaser | DiskLaser | DiskLaserE (same values) |
| Damage multiplier from ability | 1.0× | 1.10× (FIREPOWER) | 1.25× (FIREPOWER) |
| Effective damage | 90 | 99 | 113 |
| ROF | 80 | 80 | 60 (ROF ability) |
| HP multiplier | 1.0× (600) | 1.10× (660) | 1.25× (750) |
| Sight | 9 | 11 (SIGHT) | 9 (no SIGHT) |
| Speed | 15 | 17 (FASTER) | 15 (no FASTER) |
| Self-heal | no | no | **yes (SELF_HEAL)** |

(Multipliers approximate; consult VeterancyClass for exact ability percentages.)

### 3.5 `[BlimpBombEffect]` — DeathWeapon

```ini
[BlimpBombEffect];gs To make crashing guys use a big blimp bomb explosion, but not be forced to do a lot of damage to get the effect
Damage=250
Burst=1
ROF=50
Range=1.5
CellRangefinding=yes
Projectile=BlimpBombP
Speed=20
Warhead=BlimpHEEffect
Report=KirovAttack
OmniFire=yes ; Don't need to turn even though I have no turret (Need since if I am directly over my target it will baffle the CloseEnough test for the facing)
```

| Key | Effect |
|-----|--------|
| `Damage=250` | Base 250, but Disc applies `DeathWeaponDamageModifier=.1` → effective **25** at ground impact. |
| `Range=1.5` | Vertical fall-strike range. |
| `Projectile=BlimpBombP` | Vertical falling-bomb projectile (shared with Kirov DeathWeapon). |
| `Warhead=BlimpHEEffect` | See below. |
| `Report=KirovAttack` | Deep boom sound (shared). |

Warhead:
```ini
[BlimpHEEffect]
CellSpread=2
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=100%,100%,100%,70%,35%,35%,85%,75%,50%,100%,100%
Conventional=yes
Rocker=yes
InfDeath=2
AnimList=EXPLOMED
Tiberium=yes
Sparky=no
Bright=yes
ProneDamage=70%
```

- `CellSpread=2`, `PercentAtMax=.5` — 2-cell radius, 50 % edge falloff.
- `Verses=100/100/100/70/35/35/85/75/50/100/100` — Full damage vs infantry classes and light vehicles, reduced 35 % vs medium/heavy vehicles, moderate vs buildings.
- `Rocker=yes` — screen shake.
- `Bright=yes`, `ProneDamage=70%`.

> **Net: a destroyed Disc's crash impact does ~25 base damage in a 2-cell radius with rocker effect.** Designed so that a falling disc doesn't randomly nuke its own units (BlimpBombEffect's raw 250 would be devastating; the 10 % modifier keeps it controlled).

### 3.6 Warheads

#### 3.6.1 `[DiskWH]` — DiskLaser impact

```ini
[DiskWH]
Wall=no
Verses=100%,100%,100%,50%,50%,50%,100%,100%,100%,100%,100%
AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070
InfDeath=7
PenetratesBunker=yes ;If shot at a bunkered tank, no means the bunker gets the damage, yes means the unit does
```

- `Wall=no` — does NOT destroy walls.
- `Verses=100/100/100/50/50/50/100/100/100/100/100` — **100 % vs infantry classes, 50 % vs all vehicle armor types, 100 % vs buildings/special**. The Disc is excellent vs structures and infantry, half-damage vs tanks. Notably no CellSpread (single-target laser).
- `InfDeath=7` — **radiation death animation** (per InfDeath cheat sheet). Infantry hit by DiskLaser die with a radiation-poisoning animation (consistent with the "alien beam" theme). No actual `Radiation=yes` on the warhead, so no rad-site spawns — just the death-anim styling.
- `PenetratesBunker=yes` — bypasses Battle Bunker damage soak.

#### 3.6.2 `[AntiB]` — DiskDrain "warhead" (anti-building gate)

```ini
[AntiB]
Verses=0%,0%,0%,0%,0%,0%,100%,100%,100%,0%,0%
Bullets=yes
```

- `Verses=0/0/0/0/0/0/100/100/100/0/0` — **Zero damage vs every armor except wood (100 %), steel (100 %), concrete (100 %)**. This is the targeting gate: ensures DiskDrain ONLY engages buildings. Non-building targets receive zero damage and the targeting code (combined with `DrainWeapon=yes`) refuses to engage them.
- `Bullets=yes` — counts as bullet-class damage (uncertain effect — likely shared with other anti-building utility warheads like NukeB, MagneShakeWH).
- No CellSpread (single-target).

### 3.7 Projectile bookkeeping

`Projectile=InvisibleAll` and `Projectile=InvisibleVertical` are standard bookkeeping projectiles (no visual, no physical motion). They exist because every WeaponType requires a Projectile= entry, but the actual rendering goes through DiskLaserClass / DrainAnimationType respectively.

---

## 4. Voice & sound catalogue

| Slot | Sound key | sndmd entry | Audio clip(s) |
|------|-----------|-------------|---------------|
| `VoiceSelect` | `FloatingDiscSelect` | sound:4509 | unique disc-select voice |
| `VoiceMove` | `FloatingDiscMove` | sound:4514 | unique disc-move voice |
| `VoiceAttack` | `FloatingDiscAttackCommand` | sound:4519 | unique disc-attack voice (Primary) |
| `VoiceSecondaryWeaponAttack` | `FloatingDiscSteal` | sound:4524 | **unique drain voice** ("stealing energy") — plays when ordered to drain a building |
| `VoiceFeedback` | (empty) | — | — |
| `CrashingSound` | `FloatingDiscDie` | sound:5708 | death plummet sound |
| `MoveSound` | `FloatingDiscMoveLoop` | sound:1466 | **looping flight hum** (continuous UFO drone) |
| `CreateSound` | `FloatingDiscCreated` | sound:5697 | played at production-complete (replaces generic "vehicle built" report) |
| `DiskLaser Report` | `FloatingDiscAttack` | sound (TODO entry) | laser fire sound |
| `DiskDrain Report` | `KirovAttack` | sound (shared with Kirov/BlimpBomb) | deep mechanical drone |
| `BlimpBombEffect Report` | `KirovAttack` | (same) | death-impact sound |
| (looping) | `FloatingDiscStealLoop` | sound:1473 | **continuous drain-in-progress loop** — plays as long as DiskDrain is active over a target |

> **The Disc has ONE OF THE MOST EXTENSIVE per-unit sound sets in YR.** Eight unique `Floating*` sound entries, plus two looping clips (MoveLoop and StealLoop) and a per-attack split (separate Primary/Secondary voices via `VoiceSecondaryWeaponAttack`). Designed to make the Disc feel alien and distinct.

---

## 5. Owners / prerequisites / tech gating

- **Buildable by:** `YuriCountry` only (the single Yuri faction).
- **Prerequisite:** `YAWEAP,YATECH` — Yuri War Factory AND Yuri Battle Lab. Top-tier gating.
- **TechLevel:** 2 (low number — actual gating via Prerequisite).
- **Cost:** 1750 — premium tier-3 cost.
- `AllowedToStartInMultiplayer=no` — not pre-built.
- `CrateGoodie=no` — not from crates.

---

## 6. Veterancy

See §3.4 table for the full ability+weapon-swap matrix.

| Rank | Effect |
|------|--------|
| Rookie | Base — DiskLaser (90 dmg), HP=600, Sight=9, Speed=15. |
| Veteran | `STRONGER,FIREPOWER,SIGHT,FASTER` — HP+10%, damage+10%, sight+, speed+. |
| Elite | `SELF_HEAL,STRONGER,FIREPOWER,ROF` + `ElitePrimary=DiskLaserE` swap — passive HP regen added, +damage and +ROF (no sight/speed bonus on elite). Weapon swap is cosmetic-only (values identical to DiskLaser). |

Note: `SelfHealing=yes` on the techno gives the Disc passive HP regen *at all ranks*, independent of the elite `SELF_HEAL` ability. Elite just adds the additional ability layer.

---

## 7. Hardcoded behavior — Ghidra-verified

### 7.1 String-name scan

- `search_strings "DISK"` → not searched directly; would catch unrelated computer-disk strings. The relevant lookups are via the WeaponType flag `DiskLaser` (verified at 0x00817138).
- `search_strings "DiskLaser"` → 5 matches: 1 INI key string, 3 RTTI typeids for VectorClass/DynamicVectorClass/DiskLaserClass templates, and `DiskLaserChargeUp` (a Rules audio key at 0x0083a670). **Confirmed: the engine has a dedicated DiskLaserClass with full vtable and storage arrays.** See cross-referenced doc.

### 7.2 Why DISK is in `[VehicleTypes]` despite being aircraft-like

The Floating Disc is registered as a VehicleType, not an AircraftType. Reasons:
1. **Selection grouping** — vehicles group with vehicles in box-select / "select all combat units" hotkeys; aircraft go to a separate group. The Disc plays as ground-controllable.
2. **No auto-return-to-airpad** — AircraftType triggers AircraftClass return-to-dock logic after ammo depletion. The Disc has unlimited ammo (DiskLaser has no Ammo= field), uses jumpjet locomotor, and stays where directed.
3. **AI threat profile** — `ConsideredAircraft=yes` (TechnoType @ 0x00714fe9, verified) tells the AI/code to treat it as an aircraft for *threat evaluation* (e.g., AA defenses will target it), but the unit class stays VehicleType.
4. **Pathfinder zone** — `MovementZone=Fly` + `SpeedType=Hover` + JumpjetLocomotion CLSID — same as flying units, regardless of class.

This is one of YR's "hybrid" patterns: VehicleType chassis + ConsideredAircraft flag + jumpjet locomotor.

### 7.3 Verified field scopes (new this doc)

| Field | Scope | Address |
|-------|-------|---------|
| `DrainWeapon=yes` | WeaponType | **0x0077223f** (NEW) |
| `DiskLaser=yes` | WeaponType | **0x00772645** (NEW) |
| `BalloonHover=yes` | TechnoType | **0x00714d95** (NEW) |
| `IsTilter=yes` | UnitType | **0x00747712** (NEW) |
| `TurretSpins=yes` | TechnoType | **0x00713360** (NEW) |
| `ConsideredAircraft=yes` | TechnoType | **0x00714fe9** (NEW) |
| `JumpjetWobbles=.1` | TechnoType | **0x0071518b** (NEW) |
| `VoiceSecondaryWeaponAttack` | TechnoType | 0x00844038 (already in cheat sheet) |
| `DeathWeapon=BlimpBombEffect` | TechnoType per-unit override + Rules global | DUAL-READ |
| `DeathWeaponDamageModifier=.1` | TechnoType | 0x00844488 |
| `OpportunityFire=yes` | TechnoType | 0x0071483d |
| `ElitePrimary=DiskLaserE` | TechnoType | 0x00712a32 |
| `DamageSmokeOffset` | TechnoType | 0x00713e25 |
| `ImmuneToPsionics` (n/a — not on DISK; the disc CAN be mind-controlled actually) | TechnoType | 0x00714fa7 |
| `CrateGoodie=no` | UnitType | 0x00747658 |
| `TooBigToFitUnderBridge=true` | UnitType | 0x0074774e |

### 7.4 DiskLaserClass hardcoded subsystem

This is the most significant hardcoded mechanism for the Disc. From [`DISK_LASER_CLASS_GHIDRA_REPORT.md`](../../DISK_LASER_CLASS_GHIDRA_REPORT.md):

1. **Creation path:** `TechnoClass::Fire_At` @ `0x006FDD50` checks the weapon's `+0x14A` byte (the `DiskLaser=yes` flag). If set, allocates new DiskLaserClass(0x40 bytes), calls Constructor @ `0x004A7A30`, then `BulletAnimTracker::Register` @ `0x004A71A0`.
2. **Storage:** Per-instance pointer added to global array `g_DiskLaserClass_Array` @ `0x008A020C` with vector metadata at 0x008A0208/210/218.
3. **Per-tick update:** `LogicClass::PerTickUpdate` @ `0x0055B5A1` iterates the array and calls vtable slot 23 (offset 0x5C) = `DiskLaserClass::AI` @ `0x004A7340` for each instance.
4. **AI function:** Manages State (countdown timer), StepCounter (ring expansion index 0..8), and applies warhead damage at a SPECIFIC ring step (not at frame 0). Lifetime ~10 sim frames.
5. **Rendering:** Each step, spawns `LaserDrawClass` segments using the 16-entry rotation table plus an `InitialFacing` derived from atan2(target-source). Produces the rotating-ring expansion visual.
6. **Cleanup:** On firing-disc death mid-attack, a secondary tracker array (`g_0x00B0F6A0`) shared with particle systems / techno-cell-action ensures the DiskLaser is destroyed too. No orphaned ring-rendering.

### 7.5 Drain mechanic hardcoded chain

1. **Weapon flag:** `DrainWeapon=yes` set on `[DiskDrain]` WeaponType. Verified at 0x00849470 → 0x0077223f.
2. **Target gate:** `Warhead=AntiB` Verses zeros out everything but buildings — refuses to engage non-building targets at the targeting layer (before drain logic activates).
3. **Drain math:** Rules globals `DrainMoneyAmount` (Rules+0x318) and `DrainMoneyFrameDelay` (Rules+0x314) — every DrainMoneyFrameDelay frames, transfer DrainMoneyAmount credits from target owner to disc owner.
4. **Power impact:** Implementation detail (not exhaustively re-verified here, but suggested by the design) — the drained building's power-output contribution to its owner is suppressed while drain is active, so a refinery or power plant under drain effectively goes "offline" from the owner's perspective.
5. **Animation:** `DrainAnimationType` (Rules+0x31c) — visual overlay anim played at the target during drain (likely a beam from disc to building).
6. **Voice:** `VoiceSecondaryWeaponAttack=FloatingDiscSteal` plays on order; `FloatingDiscStealLoop` plays continuously during drain.
7. **Cancellation:** Drain ends if disc dies, target dies, target is sold/captured by another player, or player orders disc to do something else.

This is one of the few unit-specific economic mechanics in YR — most weapons just do damage. The Disc, the Money-Mover-spawned-by-tech-derrick, and the Yuri Battle Bunker garrison-grant are the major asymmetric economic systems.

### 7.6 BalloonHover + Crashable interaction

- **`BalloonHover=yes`** + JumpjetLocomotion = Disc maintains JumpjetHeight=750 constantly, never enters the voluntary-land branch (state 4 of JumpjetLocomotor per BRIDGE_JUMPJET docs).
- **`Crashable=yes`** + `TiltCrashJumpjet=yes` = when destroyed (HP→0), enter crash-plummet state with tilt animation, falling at JumpjetCrash=15 rate. On ground impact, fire `DeathWeapon=BlimpBombEffect` with 10 % damage modifier.
- Without Crashable, the Disc would just delete on HP→0 (no visible death animation). With it, you see the dramatic fall + explosion.

---

## 8. TS-legacy filter

| Feature | Status in YR |
|---------|--------------|
| Locomotor `{92612C46-...}` = JumpjetLocomotionClass | Live in YR (Disc, Rocketeer, Kirov, etc.). |
| `BalloonHover=yes` | Live in YR. Bypasses voluntary-land jumpjet state. |
| `Crashable=yes` | Live in YR. |
| `TiltCrashJumpjet=yes` | Live in YR. |
| `JumpjetNoWobbles=yes` | Live in YR. |
| `DrainWeapon=yes` | Live YR hardcoded mechanism. Not TS-legacy (TS had no equivalent — disc-style drain is a YR addition). |
| `DiskLaser=yes` | Live YR. Routes to dedicated DiskLaserClass. |
| `TurretSpins=yes` | Live YR (also used by some defensive structures). |
| `ConsideredAircraft=yes` | Live YR hybrid pattern. |
| `;IsLaser=true`, `;IsHouseColor=true`, `;IsRadBeam=yes` on weapons | INI comments — disabled. The `DiskLaser=yes` routing supersedes. |
| `;700` (Strength), `;yes` (Crusher) | INI comments — draft annotations. |
| `Conventional=yes` on warheads | Live in YR. |
| `Tiberium=yes` on BlimpHEEffect | TS-holdover terminology; in YR drives ore-cluster chain detonation. |
| `;Maverick`-type unrelated cleanup | Not on DISK. |
| Subterranean / Tunneling / ImmuneToVeins | Not on DISK. |
| Fog-of-war 0x1000 gate | Not on DISK. |

---

## 9. Coverage audit

| Section | Coverage |
|---------|----------|
| rulesmd `[DISK]` — every key | ✅ §1 (60+ keys including all jumpjet params + commented draft notes) |
| artmd `[DISK]` — every key | ✅ §2 (5 keys) |
| `[DiskLaser]` weapon (primary) | ✅ §3.1 (15 keys + 3 commented IsRadBeam/IsLaser/IsHouseColor) |
| `[DiskDrain]` weapon (secondary, drain) | ✅ §3.2 (13 keys) |
| `[DiskLaserE]` weapon (elite swap) | ✅ §3.3 + §3.4 (functionally identical analysis) |
| `[BlimpBombEffect]` weapon (death) | ✅ §3.5 |
| `[DiskWH]` warhead | ✅ §3.6.1 |
| `[AntiB]` warhead (building gate) | ✅ §3.6.2 |
| `[BlimpHEEffect]` warhead (death explosion) | ✅ §3.5 |
| Projectiles InvisibleAll/InvisibleVertical/BlimpBombP | ✅ §3.7 |
| Voices / sounds (8 named slots + 2 loops + 3 reports = 13 sound bindings) | ✅ §4 |
| Owners / prereqs / tech | ✅ §5 |
| Veterancy | ✅ §6 + §3.4 (matrix) |
| Hardcoded behavior — Ghidra-verified | ✅ §7 (1 string-scan, **7 NEW field-scope verifications added to cheat sheet**: DrainWeapon @ 0x0077223f, DiskLaser @ 0x00772645, BalloonHover @ 0x00714d95, IsTilter @ 0x00747712, TurretSpins @ 0x00713360, ConsideredAircraft @ 0x00714fe9, JumpjetWobbles @ 0x0071518b; full DiskLaserClass cross-ref + drain mechanic chain) |
| TS-legacy filter | ✅ §8 |
| Cross-references (DISK_LASER_CLASS, JUMPJET_LOCOMOTION, BRIDGE_JUMPJET_HEIGHT, LASER_DRAW) | ✅ at top + inline |
| VehicleType-vs-AircraftType architecture (ConsideredAircraft pattern) | ✅ §7.2 |
| Drain-mechanic hardcoded chain (weapon flag → warhead gate → Rules drain math → animation/voice) | ✅ §7.5 |

---

## 10. Quick implementer summary

To make a DISK-equivalent:

1. **Render** — voxel + HVA; spinning turret animation via TurretSpins=yes (continuous rotation, ignores target facing).
2. **Movement** — JumpjetLocomotionClass with per-unit Jumpjet* overrides; BalloonHover=yes skips voluntary-land state; JumpjetNoWobbles=yes suppresses sinusoidal motion.
3. **Death** — Crashable=yes + TiltCrashJumpjet=yes: HP→0 triggers fall-plummet with tilt; on ground impact, fire DeathWeapon=BlimpBombEffect × DeathWeaponDamageModifier=0.1.
4. **Primary attack (DiskLaser)** —
   - WeaponType has `DiskLaser=yes` flag at offset 0x14A.
   - In Fire_At, when flag is set, skip BulletClass and instead allocate DiskLaserClass(0x40 bytes), insert into per-tick array, call BulletAnimTracker.Register with derived InitialFacing.
   - DiskLaserClass AI ticks 10 frames, expanding ring via 16-entry rotation table, spawning LaserDrawClass segments.
   - Apply warhead damage at a specific step (not impact); 90 dmg with DiskWH warhead.
5. **Secondary attack (DiskDrain)** —
   - WeaponType has `DrainWeapon=yes` flag.
   - Warhead AntiB with Verses=0% vs non-building armor types gates targeting to buildings only.
   - On engage: disc anchors over target (FireWhileMoving=no), enters drain state, transfers `Rules.DrainMoneyAmount` credits per `Rules.DrainMoneyFrameDelay` frames from target owner to disc owner.
   - Drain animation overlay on target (Rules.DrainAnimationType).
   - Cancelled on disc death, target death, manual move command.
6. **Audio** — separate VoiceAttack (primary) and VoiceSecondaryWeaponAttack (secondary, drain-themed); MoveLoop continuous flight hum; FloatingDiscStealLoop during drain; CreateSound for production-complete.
7. **Veterancy** — Vet abilities buff stats; Elite adds SELF_HEAL + ROF + swaps to DiskLaserE (cosmetic; functionally identical).
8. **AI flags** — ConsideredAircraft=yes for AA targeting; OpportunityFire=yes during movement; ThreatPosed=20.
9. **Build gate** — YAWEAP+YATECH prerequisites; YuriCountry only; AllowedToStartInMultiplayer=no.

The Disc requires a dedicated `DiskLaserClass` sim entity and a hardcoded `DrainWeapon` state machine — these cannot be expressed via generic WeaponType handling alone. Both are essential for parity.
