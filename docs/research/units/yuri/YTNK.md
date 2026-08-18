# YTNK — Gattling Tank (Yuri Multi-Stage Spin-Up Vehicle)

**INI ID:** `YTNK`
**Display Name:** `Gattling Tank` (`UIName=Name:GattTank`)
**Side:** Yuri (`Owner=YuriCountry`)
**Category:** Vehicle / AFV
**Cameo:** `YTNKICON` / `YTNKUICO` (AltCameo)
**Voxel:** yes

The Gattling Tank is Yuri's tier-2 mass-produce anti-infantry / anti-air
vehicle. Its defining mechanic is the **multi-stage gattling weapon system** —
a per-instance accumulator that ramps up while firing and decays while idle,
cycling through 3 progressively faster/heavier weapon stages. Each stage has
a separate ground weapon AND a separate anti-air weapon, swapping based on
target type. The visual+audio effect is the iconic "spinning barrels →
faster fire rate → louder gun loop" gattling charge-up.

The gattling system is a **shared engine subsystem** — also used by
`[YAGGUN]` (Yuri Gattling Cannon defense structure). Any unit/building with
`IsGattling=yes` participates.

> **Cross-references — do not re-derive:**
> - [`GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md`](../../GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md) (536 lines) — full system: TechnoType offsets 0xCD5..0xD10 (IsGattling/WeaponStages/Stage1..6/EliteStage1..6/RateUp/RateDown), TechnoClass runtime fields 0x140 CurrentGattlingStage + 0x144 GattlingValue + 0x148 GattlingCycleCount, charge-up function `IncreaseGattlingStage` @ 0x0070DE70, decay function `UpdateGattlingStage` @ 0x0070E000, weapon-index lookup `SelectWeaponAgainst` @ 0x006F3330, elite-aware `GetWeapon` @ 0x0070E140, BuildingClass fire/decay sites, TemporalClass-triggered decay on warp. **All deep RE is in that doc.**
> - [`DISK.md`](./DISK.md), [`TELE.md`](./TELE.md), [`CAOS.md`](./CAOS.md), [`MIND.md`](./MIND.md) — sibling Yuri hardcoded-behavior units (each has a different state machine).

> **TS-legacy filter:** `;Image=TTNK`, `;TargetLaser=yes`, `;yes` Crusher are INI comments. `;was 5` ROT note is historical. `;DownReport=...` keys on weapons are commented (would have played a wind-down sound between stages); disabled. `GattlingCycleCount` (TechnoClass+0x148) is incremented but **never read** by any consumer in the binary — likely TS-legacy / dead field per GATTLING_WEAPON_STAGE doc §2.

---

## 1. Full `rulesmd.ini` section verbatim

```ini
[YTNK]
UIName=Name:GattTank
Name=Gattling Tank
;Image=TTNK
Prerequisite=YAWEAP
Primary=AGGattling
Secondary=AAGattling
Strength=210
Category=AFV
Armor=light
Turret=yes
IsTilter=yes
;-----Gattling stuff-------
;GEF There is a _lot_ of stuff in this section, hope it's worth it

;Do I have a gattling gun or not
IsGattling=yes

;How weapons does it have? Currently, all Gattling Units have had anti-air
;capability, so the mechanics currently depends on having twice the number
;of stages in weapons, alternating anti-ground first, and anti-air second.
TurretCount=1
WeaponCount=6

Weapon1=AGGattling
EliteWeapon1=AGGattlingE
Weapon2=AAGattling
EliteWeapon2=AAGattlingE
Weapon3=AGGattling2
EliteWeapon3=AGGattling2E
Weapon4=AAGattling2
EliteWeapon4=AAGattling2E
Weapon5=AGGattling3
EliteWeapon5=AGGattling3E
Weapon6=AAGattling3
EliteWeapon6=AAGattling3E

;How many stages does this gattling gun have, and how long does it
;take to progress through these stages;
WeaponStages=3
Stage1=200
Stage2=400
;This last stage is used to determine what the maximum fireing timer can be. Once it
;hits this it will stop increasing. If this is larger than the previous stage, then
;it will have a grace period once the unit stops firing before it needs to drop
;down to the lower weapon.
Stage3=600

EliteStage1=100
EliteStage2=200
EliteStage3=300

;How many increments or decrements does the timer get per frame?
;If RateDown is zero, then it overrides the previous stage vaules,
;causing the tank to instantly go to zero when it stops firing
;if it can't find a new target
RateUp=1
RateDown=50


;-----End Gattling stuff-------

;TargetLaser=yes
TooBigToFitUnderBridge=true
TechLevel=4
Sight=10
Speed=6
CrateGoodie=no
Crusher=no;yes
Owner=YuriCountry
Cost=600
Soylent=600
Points=25
ROT=10 ;was 5
AllowedToStartInMultiplayer=no
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=GattlingTankSelect
VoiceMove=GattlingTankMove
VoiceAttack=GattlingTankAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=FlakTrackMoveStart
CrushSound=TankCrush
Maxdebris=3
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
MovementZone=Destroyer
ThreatPosed=40	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
DamageSmokeOffset=100, 100, 275
Weight=3.5
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Accelerates=false
ZFudgeColumn=8
ZFudgeTunnel=13
Size=3
OpportunityFire=yes
ElitePrimary=AGGattlingE;90mmE
EliteSecondary=AAGattlingE;90mmE
```

### 1.1 Key-by-key explanation (gattling-specific keys + standard keys)

| Key | Value | Read by | Effect |
|-----|-------|---------|--------|
| `UIName=Name:GattTank` | string | AbstractTypeClass | CSF lookup. |
| `Name=Gattling Tank` | string | AbstractTypeClass | English fallback. |
| `;Image=TTNK` | (commented) | — | Would have used Tesla Tank voxel; disabled. |
| `Prerequisite=YAWEAP` | building | TechnoTypeClass | **Yuri War Factory only** — no Battle Lab gate. Available early. |
| `Primary=AGGattling` | weapon | TechnoTypeClass | **Default Primary** — used by code paths that read `TechnoType.Primary` directly. But the actual fire-time weapon comes from `Weapon[stage*2]` lookup via gattling stage system. Functionally equivalent to `Weapon1`. |
| `Secondary=AAGattling` | weapon | TechnoTypeClass | Default Secondary — functionally equivalent to `Weapon2`. |
| `Strength=210` | hp | TechnoTypeClass | 210 HP — moderate for a tier-2 tank. |
| `Category=AFV` | enum | TechnoTypeClass | AFV. |
| `Armor=light` | enum | TechnoTypeClass | Light armor — vulnerable to AP/HE weapons. |
| `Turret=yes` | bool | UnitTypeClass | Turret present (the spinning barrels). |
| `IsTilter=yes` | bool | UnitType @ 0x00747712 | Body tilts on turns. |
| `IsGattling=yes` | bool | TechnoType +0xCD5 @ 0x0071401d (NEW THIS DOC) | **Master gattling-system enable flag.** Verified TechnoType scope. When set, the engine routes the unit through the multi-stage weapon switching machinery: `UnitClass::Fire_At_Target` calls `IncreaseGattlingStage` after every successful fire and `UpdateGattlingStage` (decay) before each new fire attempt. Per GATTLING_WEAPON_STAGE doc §3. |
| `TurretCount=1` | int | TechnoType @ 0x00712851 (cheat sheet) | Single turret. |
| `WeaponCount=6` | int | TechnoType @ 0x0071286b (cheat sheet) | **6 weapons total** (3 stages × 2 modes [ground/air] = 6). Must equal `WeaponStages × 2` per gattling design. |
| `Weapon1=AGGattling` through `Weapon6=AAGattling3` | weapon refs | TechnoType | The 6-weapon array. Stage layout (per GATTLING_WEAPON_STAGE doc §3): `Weapon[stage*2] = ground`, `Weapon[stage*2+1] = air`. So Weapon1/3/5 are anti-ground for stages 0/1/2 respectively, and Weapon2/4/6 are anti-air. |
| `EliteWeapon1=AGGattlingE` through `EliteWeapon6=AAGattling3E` | weapon refs | TechnoType | Elite-rank parallel weapon set. `GetWeapon` (vtable+0x3F8 per GATTLING doc §5) returns the EliteWeapon[i] when the unit is elite, otherwise Weapon[i]. |
| `WeaponStages=3` | int | TechnoType +0xCD8 @ 0x00714037 (NEW THIS DOC) | **Number of reachable stages (0..N-1 = 0..2 here).** Verified TechnoType scope. |
| `Stage1=200` | int | TechnoType +0xCDC | Threshold for stage 1. |
| `Stage2=400` | int | TechnoType +0xCE0 | Threshold for stage 2. |
| `Stage3=600` | int | TechnoType +0xCE4 | **Threshold for stage 3 AND value cap when WeaponStages=3.** Per inline comment: "This last stage is used to determine what the maximum fireing timer can be. Once it hits this it will stop increasing. If this is larger than the previous stage, then it will have a grace period once the unit stops firing before it needs to drop down to the lower weapon." So the accumulator is capped at 600. Stage 2 → stage 3 transition happens at 400. The 200-value gap between 400 (entry) and 600 (cap) IS the "grace period" — you have 200 worth of accumulator headroom that can decay before stage drops back to 2. |
| `EliteStage1=100` / `EliteStage2=200` / `EliteStage3=300` | int | TechnoType +0xCF4/0xCF8/0xCFC | **Elite stage thresholds — half of normal**, so elite ranks up through stages 2× faster (and decay-back happens 2× faster too at the higher cap). |
| `RateUp=1` | int | TechnoType +0xD0C @ 0x00714051 (NEW THIS DOC) | **Accumulator gain per "fire tick" — +1.** Verified TechnoType scope. |
| `RateDown=50` | int | TechnoType +0xD10 | **Accumulator loss per "decay tick" — -50/frame.** Decay is 50× faster than ramp-up. Inline comment: "If RateDown is zero, then it overrides the previous stage values, causing the tank to instantly go to zero when it stops firing if it can't find a new target". |
| `;TargetLaser=yes` | (commented) | — | Disabled. |
| `TooBigToFitUnderBridge=true` | bool | UnitType @ 0x0074774e | Bridge gate. |
| `TechLevel=4` | int | TechnoTypeClass | Mid-tier; Prerequisite is the real gate. |
| `Sight=10` | cells | TechnoTypeClass | **10-cell sight** — long-range visibility (matches the 8-cell AA weapon range). |
| `Speed=6` | int | TechnoTypeClass | Standard tank speed. |
| `CrateGoodie=no` | bool | UnitType @ 0x00747658 | No crate. |
| `Crusher=no;yes` | bool | TechnoTypeClass | No crush. `;yes` draft. |
| `Owner=YuriCountry` | country list | TechnoTypeClass | Yuri only. |
| `Cost=600` | credits | TechnoTypeClass | **Very cheap (600)** — designed for mass production. The Gattling Tank is Yuri's spam unit. |
| `Soylent=600` | credits | TechnoTypeClass | Full recycle. |
| `Points=25` | int | TechnoTypeClass | Score on kill. |
| `ROT=10 ;was 5` | int | TechnoTypeClass | Turret turn rate 10 (doubled from 5 historically). |
| `AllowedToStartInMultiplayer=no` | bool | TechnoTypeClass | Not pre-built. |
| `IsSelectableCombatant=yes` | bool | TechnoTypeClass | Combat unit. |
| `Explosion=...` | anim list | TechnoTypeClass | Standard 5-anim. |
| `VoiceSelect=GattlingTankSelect` | sound | TechnoTypeClass | Unique select. |
| `VoiceMove=GattlingTankMove` | sound | TechnoTypeClass | Unique move. |
| `VoiceAttack=GattlingTankAttackCommand` | sound | TechnoTypeClass | Unique attack. |
| `VoiceFeedback=` | (empty) | TechnoTypeClass | None. |
| `DieSound=GenVehicleDie` | sound | TechnoTypeClass | Generic vehicle death. |
| `MoveSound=FlakTrackMoveStart` | sound | TechnoTypeClass | **Shared with Soviet Flak Track** (audio asset reuse). |
| `CrushSound=TankCrush` | sound | TechnoTypeClass | Generic (irrelevant — Crusher=no). |
| `Maxdebris=3` | int | TechnoTypeClass | 3 debris pieces. |
| `Locomotor={4A582741-...}` | CLSID | TechnoTypeClass | DriveLocomotionClass. |
| `MovementZone=Destroyer` | enum | TechnoTypeClass | MBT-class pathfinding. |
| `ThreatPosed=40` | int | TechnoTypeClass | High AI threat (40 — same as Magnetron). |
| `DamageParticleSystems=SparkSys,SmallGreySSys` | particle list | TechnoTypeClass | Damaged emissions. |
| `DamageSmokeOffset=100, 100, 275` | x,y,z leptons | TechnoType @ 0x00713e25 | Smoke offset. |
| `Weight=3.5` | float | TechnoTypeClass | Fractional weight. |
| `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` | ability list | TechnoTypeClass | Vet bonuses. |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | ability list | TechnoTypeClass | Elite bonuses + self-heal. |
| `Accelerates=false` | bool | TechnoTypeClass | Constant speed. |
| `ZFudgeColumn=8` / `ZFudgeTunnel=13` | int | TechnoTypeClass | Z-render tweaks. |
| `Size=3` | int | TechnoTypeClass | Transport cost. |
| `OpportunityFire=yes` | bool | TechnoType @ 0x0071483d | Engage during movement. |
| `ElitePrimary=AGGattlingE;90mmE` | weapon | TechnoType @ 0x00712a32 | **Elite Primary swap → `AGGattlingE`.** The `;90mmE` is an INI comment showing an alternate (Apocalypse cannon) that was considered as an elite upgrade. Active value: AGGattlingE. |
| `EliteSecondary=AAGattlingE;90mmE` | weapon | TechnoType (similar cheat-sheet entry) | Elite Secondary → AAGattlingE. Same `;90mmE` draft comment. |

> Note: `ElitePrimary=AGGattlingE` and `EliteWeapon1=AGGattlingE` are redundant — both point to the same elite weapon. When IsGattling=yes, the engine uses `EliteWeapon[i]` from the gattling weapon array; `ElitePrimary` is consulted for compatibility with non-gattling-aware code paths.

---

## 2. Full `artmd.ini` section verbatim

```ini
[YTNK]   ; Yuri Gattling tank
Voxel=yes
Remapable=yes
Cameo=YTNKICON
AltCameo=YTNKUICO
PrimaryFireFLH=160,30,150
SecondaryFireFLH=160,30,150
ElitePrimaryFireFLH=160,30,150
EliteSecondaryFireFLH=160,30,150
Weapon1FLH=160,30,150
Weapon2FLH=160,30,150
Weapon3FLH=160,30,150
Weapon4FLH=160,30,150
Weapon5FLH=160,30,150
Weapon6FLH=160,30,150
EliteWeapon1FLH=160,30,150
EliteWeapon2FLH=160,30,150
EliteWeapon3FLH=160,30,150
EliteWeapon4FLH=160,30,150
EliteWeapon5FLH=160,30,150
EliteWeapon6FLH=160,30,150
```

| Key | Value | Notes |
|-----|-------|-------|
| `Voxel=yes` | bool | `ytnk.vxl` + `.hva`. |
| `Remapable=yes` | bool | House-color tinted (Yuri purple). |
| `Cameo=YTNKICON` / `AltCameo=YTNKUICO` | SHP | Cameos. |
| `PrimaryFireFLH=160,30,150` | x,y,z leptons | Default Primary FLH — gun muzzle. |
| `SecondaryFireFLH=160,30,150` | x,y,z leptons | Default Secondary FLH. |
| `ElitePrimaryFireFLH` / `EliteSecondaryFireFLH` | (same) | Same offset for elite. |
| `Weapon1FLH` through `Weapon6FLH` | (same) | **6 explicit per-weapon FLH overrides** — all set to identical (160,30,150). |
| `EliteWeapon1FLH` through `EliteWeapon6FLH` | (same) | 6 elite-weapon FLH overrides — also identical. |

> **All 16 FLH entries are identical (160,30,150).** Since the gattling tank fires all weapons from the same muzzle, the per-weapon FLH override is redundant; only `PrimaryFireFLH=160,30,150` would suffice. The full enumeration exists because the engine reads each Weapon[N]FLH separately and the author included them defensively. AlternateFLH%d / Weapon[N]FLH is at TechnoType 0x00715faf per cheat sheet.

---

## 3. Weapons

### 3.1 Weapon stage table

The 6 weapons + 6 elite weapons form a 3×2 grid:

| Stage | Ground (Weapon[stage*2]) | Air (Weapon[stage*2+1]) |
|-------|--------------------------|-------------------------|
| 0 | AGGattling (Damage=25, ROF=16, GattWH) | AAGattling (Damage=25, ROF=16, GattWH) |
| 1 | AGGattling2 (Damage=25, ROF=16, SA) | AAGattling2 (Damage=30, ROF=8, GattWH) |
| 2 | AGGattling3 (Damage=25, ROF=16, SSA) | AAGattling3 (Damage=40, ROF=4, GattWH) |

| Stage | Elite Ground (EliteWeapon[stage*2]) | Elite Air (EliteWeapon[stage*2+1]) |
|-------|-------------------------------------|------------------------------------|
| 0 | AGGattlingE (Damage=25, ROF=16, GattWH) | AAGattlingE (Damage=25, ROF=16, GattWH) |
| 1 | AGGattling2E (Damage=25, ROF=16, SA) | AAGattling2E (Damage=25, ROF=8, GattWH) |
| 2 | AGGattling3E (Damage=25, ROF=16, SSA) | AAGattling3E (Damage=25, ROF=4, GattWH) |

**Observations on the weapon scaling:**

1. **Anti-ground (AG) stages have CONSTANT damage (25)** but UPGRADE WARHEAD: GattWH → SA → SSA. Each successive warhead penetrates harder armor better (see §3.3 warhead Verses tables).
2. **Anti-air (AA) stages have INCREASING damage AND DECREASING ROF**: 25/16 → 30/8 → 40/4. Effective AA DPS scales ~5× from stage 0 to stage 2.
3. **Elite has a quirk**: `AAGattling*E` damage stays at 25 across all stages (vs non-elite's 25/30/40). This appears to be an INI bug or a deliberate balance choice — elite anti-air does LESS per-shot damage than non-elite at higher stages. The FIREPOWER veteran-ability multiplier likely compensates.
4. **All AG weapons fire at ROF=16, Burst=2** — they don't get faster, only stronger via warhead. **AA weapons get progressively faster**, simulating the gattling spin-up sound on air targets.

### 3.2 Individual weapon blocks

#### Stage 0 — AGGattling / AAGattling

```ini
[AGGattling]
Damage=25;50
ROF=16
Range=6
Projectile=Invisiblelow ;GEF Anti ground ;SA
Speed=100
Warhead=GattWH
Report=GattlingGunAttackLoop1
;DownReport=GattlingGunDecreaseLoop1
Burst = 2
Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW
```

```ini
[AAGattling]
Damage=25
ROF=16
Range=8
Projectile=Invisible4 ;GEF Anti air ;SA
Speed=100
Warhead=GattWH
Report=GattlingGunAttackLoop1
;DownReport=GattlingGunDecreaseLoop1
Burst = 2
Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW
```

Key fields shared across both:
- `Range=6` (ground) / `Range=8` (air) — AA reaches further.
- `Projectile=Invisiblelow` (ground) / `Invisible4` (air) — air projectile has trajectory tuned for high-altitude targets.
- `Speed=100` — fast invisible bullets.
- `Burst=2` — 2 shots per fire (the "stuttering" gattling effect).
- `Anim=MGUN-N,...,MGUN-NW` — **8-direction muzzle-flash anim set** — engine picks the cardinal-direction MGUN-X anim matching the turret facing. Creates the directional muzzle-flash visual.
- `Report=GattlingGunAttackLoop1` — **stage-1 gun loop audio** (looping sound for the slow-fire phase).
- `;DownReport=GattlingGunDecreaseLoop1` — (commented) would have played a wind-down sound; disabled.

#### Stage 1 — AGGattling2 / AAGattling2

```ini
[AGGattling2]
Damage=25;50
ROF=16
Range=6
Projectile=Invisiblelow
Speed=100
Warhead=SA       ; ← UPGRADED: armor-piercing
Report=GattlingGunAttackLoop2
Anim=GUNFIRE     ; ← different muzzle anim
Burst = 2
```

```ini
[AAGattling2]
Damage=30        ; ← +5 damage
ROF=8            ; ← 2× faster fire
Range=8
Projectile=Invisible4
Speed=100
Warhead=GattWH
Report=GattlingGunAttackLoop2
Anim=GUNFIRE
Burst = 2
```

Stage 1 changes:
- Anti-ground swaps warhead GattWH → **SA** (better vs Plate infantry: 70→80%, better vs Light vehicle: 50→50% — actually same; the SA upgrade is mainly Verses curve refinement).
- Anti-air gets +5 damage and 2× faster fire (ROF 8 vs 16) — escalation begins.
- `Anim=GUNFIRE` — switches from 8-direction MGUN- anim to single-frame GUNFIRE (faster animation matches the faster ROF).
- `Report=GattlingGunAttackLoop2` — stage-2 audio loop (faster, more intense).

#### Stage 2 — AGGattling3 / AAGattling3

```ini
[AGGattling3]
Damage=25;50
ROF=16
Range=6
Projectile=Invisiblelow
Speed=100
Warhead=SSA      ; ← UPGRADED AGAIN: super armor-piercing
Report=GattlingGunAttackLoop3
Anim=VTMUZZLE    ; ← VT muzzle anim
Burst = 2
```

```ini
[AAGattling3]
Damage=40        ; ← +10 damage from stage 1
ROF=4            ; ← 2× faster again (4× faster than stage 0)
Range=8
Projectile=Invisible4
Speed=100
Warhead=GattWH
Report=GattlingGunAttackLoop3
Anim=VTMUZZLE
Burst = 2
```

Stage 2 changes:
- Anti-ground swaps warhead SA → **SSA** (best armor penetration: 100% vs Plate, 60% vs Light vehicle — see §3.3).
- Anti-air gets +10 more damage (40) and another 2× ROF (4). Now **8× faster DPS** than stage 0 air weapon.
- `Anim=VTMUZZLE` — final muzzle anim (most intense visual).
- `Report=GattlingGunAttackLoop3` — top-stage audio loop (loudest, fastest cycling).

#### Elite variants

Mostly identical to non-elite, with the noted anti-air damage quirk:

| | Non-elite | Elite |
|---|---|---|
| AGGattlingE | (same as AGGattling) | identical |
| AAGattlingE | (same as AAGattling) | identical |
| AGGattling2E | (same as AGGattling2) | identical |
| AAGattling2E | Damage=30 | **Damage=25** (drops from 30) |
| AGGattling3E | (same as AGGattling3) | identical |
| AAGattling3E | Damage=40 | **Damage=25** (drops from 40) |

> **Elite weapon damage quirk:** Elite-rank weapons all have Damage=25 (whereas non-elite stages 1/2 air weapons have 30/40). This makes elite AA *weaker per-shot* than non-elite AA at upper stages. The FIREPOWER ability scalar (+25% at elite) partially compensates but doesn't fully reach the non-elite 30/40 values. **Likely an INI authoring oversight or deliberate balance** — confirming would require checking Westwood's design notes. From a player POV: elite gattling tanks have noticeably more durable HP (STRONGER) and faster overall cycling (ROF ability), but per-shot damage doesn't escalate.

### 3.3 Warheads

#### `[SA]` — Stage 1 anti-ground

```ini
[SA]
Verses=100%,80%,80%,50%,25%,25%,75%,50%,25%,100%,100%
InfDeath=1
AnimList=PIFFPIFF,PIFFPIFF
Bullets=yes
ProneDamage=70%
```

- vs none/flak/plate: 100/80/80 — strong vs infantry.
- vs light/medium/heavy vehicle: 50/25/25 — moderate vs LV, weak vs MV/HV.
- vs structures: 75/50/25.
- `Bullets=yes` — bullet-type warhead (utility flag).

#### `[GattWH]` — Stage 0 anti-ground AND all stages anti-air

```ini
[GattWH]
Verses=100%,80%,70%,50%,30%,10%,10%,5%,3%,200%,50%
InfDeath=1
AnimList=PIFFPIFF,PIFFPIFF
Bullets=yes
ProneDamage=70%
```

- vs infantry: 100/80/70 — good.
- vs light/medium/heavy: 50/30/10 — strong vs LV, poor vs MV/HV.
- vs structures: 10/5/3 — **terrible vs buildings**.
- **vs special_1 (aircraft armor): 200%** — **2× damage vs aircraft** (the AA design).
- vs special_2: 50%.
- `ProneDamage=70%` — prone infantry take 70% damage.

**The 200% vs special_1 is the defining feature** — Gattling Tank is a primary AA platform for Yuri.

#### `[SSA]` — Stage 2 anti-ground (super AP)

```ini
[SSA]
Verses=100%,100%,100%,60%,40%,40%,75%,50%,25%,100%,100%
InfDeath=1
AnimList=PIFFPIFF,PIFFPIFF
Bullets=yes
ProneDamage=80%
```

- vs infantry: 100/100/100 — **all infantry classes take FULL damage** (including Plate-armored vets).
- vs vehicle: 60/40/40 — moderate vs LV, weak vs MV/HV (still not great vs tanks).
- vs structures: 75/50/25.
- `ProneDamage=80%` — slightly less prone reduction than SA/GattWH.

**SSA is the best anti-infantry warhead in the game.** A stage-2 Gattling Tank shreds infantry formations.

### 3.4 No elite distinct warheads

All 6 elite weapons reuse the same 3 warheads (GattWH/SA/SSA) — no elite-specific warhead variants. Differences between elite and non-elite are limited to the per-shot damage values noted in §3.2 (and the AA-damage quirk).

### 3.5 Projectiles

- `Invisiblelow` — ground bullets, low trajectory.
- `Invisible4` — air bullets, AA-tuned trajectory.

Both are bookkeeping invisible projectiles; the visible effects come from `Anim=` (muzzle flash) and `AnimList` on the impact warhead.

---

## 4. Voice & sound catalogue

| Slot | Sound key | sndmd entry | Audio clip(s) |
|------|-----------|-------------|---------------|
| `VoiceSelect` | `GattlingTankSelect` | sound:4529 | unique select |
| `VoiceMove` | `GattlingTankMove` | sound:4534 | unique move |
| `VoiceAttack` | `GattlingTankAttackCommand` | sound:4539 | unique attack |
| `VoiceFeedback` | (empty) | — | — |
| `DieSound` | `GenVehicleDie` | sound:1961 | generic vehicle death |
| `MoveSound` | `FlakTrackMoveStart` | sound:1457 | **shared with Soviet Flak Track** (audio reuse) |
| `CrushSound` | `TankCrush` | sound:5472 | generic crush (irrelevant — Crusher=no) |
| **Stage 0 weapon Report** | `GattlingGunAttackLoop1` | sound:1480 | **looping slow gun fire** |
| **Stage 1 weapon Report** | `GattlingGunAttackLoop2` | sound:1486 | **looping medium gun fire** |
| **Stage 2 weapon Report** | `GattlingGunAttackLoop3` | sound:1492 | **looping rapid gun fire** |
| (commented) `DownReport=GattlingGunDecreaseLoop1/2/3` | — | — | Would have been wind-down sounds; disabled. |

**Three stage-specific looping audio clips** — `GattlingGunAttackLoop1/2/3` provide the audio escalation. The engine swaps the active loop sound when the weapon stage changes (per GATTLING_WEAPON_STAGE doc §3, where `AnimClass::UpdateLoopingSound` is gated on IsGattling in `TechnoClass::AI_Update`).

---

## 5. Owners / prerequisites / tech gating

- **Buildable by:** `YuriCountry` only.
- **Prerequisite:** `YAWEAP` only — Yuri War Factory. **No Battle Lab.**
- **TechLevel:** 4 — mid-tier (TechLevel value, but real gating is War Factory).
- **Cost:** 600 — **cheap mass-produce** unit. Yuri's affordable AA + anti-infantry.
- `AllowedToStartInMultiplayer=no` → not pre-built.
- `CrateGoodie=no` → not from crates.

---

## 6. Veterancy

| Rank | Effect |
|------|--------|
| Rookie | Base — uses Weapon[0..5] (AGGattling/AAGattling/AGGattling2/AAGattling2/AGGattling3/AAGattling3), normal Stage1=200, Stage2=400, Stage3=600 thresholds. |
| Veteran | `STRONGER,FIREPOWER,SIGHT,FASTER` — +HP, +damage, +sight, +speed. **Still uses non-elite weapon set** (Weapon[0..5]). |
| Elite | `SELF_HEAL,STRONGER,FIREPOWER,ROF` — passive self-heal, +HP, +damage, +ROF. **Swaps to EliteWeapon[0..5]** (AGGattlingE/AAGattlingE/etc.) **AND uses EliteStage1=100/EliteStage2=200/EliteStage3=300 thresholds**. |

> **Critical mechanic:** Elite ranks gain a **2× faster gattling ramp-up** (thresholds halved). Combined with ROF ability, elite gattling tanks reach top stage in half the firing time and maintain it longer. This is more impactful than the per-shot damage difference noted in §3.2.

---

## 7. Hardcoded behavior — Ghidra-verified

### 7.1 String-name scan

- `search_strings "IsGattling"` → 1 match @ 0x00843e4c → TechnoTypeClass__ReadINI @ 0x0071401d. **NEW: IsGattling @ TechnoType 0x0071401d** (verified scope).
- `search_strings "WeaponStages"` → 1 match @ 0x00843e3c → TechnoTypeClass__ReadINI @ 0x00714037. **NEW: WeaponStages @ TechnoType 0x00714037**.
- `search_strings "RateUp"` → 1 match @ 0x00843e34 → TechnoTypeClass__ReadINI @ 0x00714051. **NEW: RateUp @ TechnoType 0x00714051** (and adjacent RateDown).
- `search_strings "WeaponCount"` → 1 match @ 0x0084433c → TechnoType (cheat sheet 0x0071286b).

### 7.2 The gattling state machine (cross-ref summary)

From [`GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md`](../../GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md):

**Per-instance state on TechnoClass:**
- `+0x140 CurrentGattlingStage` (int): which stage (0..WeaponStages-1).
- `+0x144 GattlingValue` (int): accumulator, range [0, Stage[WeaponStages]].
- `+0x148 GattlingCycleCount` (int): incremented on each successful fire, **never read** — vestigial.
- `+0x4B8`/`+0x4D4` byte flags: spinup-sound/muzzle-anim bookkeeping (low confidence on exact names).

**Charge-up function `IncreaseGattlingStage` @ 0x0070DE70:**
- Called by `UnitClass::Fire_At_Target` (`0x00736DF0`) AND `BuildingClass::Mission_Attack` (`0x0044ACF0`) after each successful fire.
- Adds `RateUp` (+0xD0C) to `GattlingValue` (+0x144).
- Caps at `Stage[WeaponStages]` (the last threshold value).
- If `GattlingValue` crosses a higher threshold: advance `CurrentGattlingStage` (+0x140); play `Report` audio for the new weapon.

**Decay function `UpdateGattlingStage` @ 0x0070E000:**
- Called per-tick when the unit is NOT firing (in `TechnoClass::AI_Update`'s gattling branch).
- Subtracts `RateDown × ticks_since_last_fire` from `GattlingValue`.
- If `GattlingValue` crosses a lower threshold: decrement `CurrentGattlingStage`.
- If `RateDown=0`: instantly drops `GattlingValue` to 0 (per inline INI comment).

**Weapon selection `SelectWeaponAgainst` @ 0x006F3330:**
- Maps `(CurrentGattlingStage, target_type)` → weapon index in the Weapon[] array.
- For air target: index = `stage*2 + 1`.
- For ground target: index = `stage*2`.
- Returned weapon goes through `GetWeapon` @ 0x0070E140 which applies the elite-aware swap (returns EliteWeapon[i] if unit is elite).

**Temporal/warp interaction (Chrono Legion freeze):**
- `TemporalClass::InitiateWarp` @ 0x0071AF20 decays the target's gattling stage when warp begins. This means a Chrono Legion freezing a Gattling Tank causes it to lose its spin-up, requiring re-ramp on unfreeze.

### 7.3 Why "all FLH = 160,30,150" is redundant

All 16 FLH entries (PrimaryFireFLH, SecondaryFireFLH, ElitePrimaryFireFLH, EliteSecondaryFireFLH, Weapon1FLH..Weapon6FLH, EliteWeapon1FLH..EliteWeapon6FLH) are identical. Since the visible turret has a single muzzle position (the spinning barrel), all weapons fire from the same FLH. The author could have used just `PrimaryFireFLH=160,30,150` (with engine fallback to default for the others), but explicitly listed every override defensively — perhaps as documentation, or because the engine's FLH-fallback chain is unclear for gattling-multiweapon setups. **Net effect: identical visual; minor INI bloat.**

### 7.4 The "RateDown=50 grace period" balance

The combination of `Stage3=600` (cap) and `Stage2=400` (threshold) creates a 200-value buffer between "fully ramped" and "drops back to stage 1". At `RateDown=50/frame`, this 200-value buffer decays in `200/50 = 4 frames`. So:

- **Sustained firing maintains stage 2 indefinitely** (RateUp=1 per fire frame compensates for RateDown=50 elsewhere).
- **Brief firing gaps (< 4 frames between targets)** keep the gattling at top stage.
- **Longer gaps (4+ frames)** cause stage drop-back to 1 (and further gaps to 0).

This rewards continuous engagement — switching targets mid-fight while there's another within ~6-8 cells maintains top-stage DPS. It punishes "ramp-up-then-stop" behavior.

### 7.5 Verified field scopes (new this doc)

| Field | Scope | Address |
|-------|-------|---------|
| `IsGattling=yes` | TechnoType +0xCD5 | **0x0071401d** (NEW) |
| `WeaponStages=N` | TechnoType +0xCD8 | **0x00714037** (NEW) |
| `RateUp=N` (adjacent RateDown) | TechnoType +0xD0C/+0xD10 | **0x00714051** (NEW) |
| `WeaponCount=N` | TechnoType | 0x0071286b (cheat sheet) |
| `TurretCount=N` | TechnoType | 0x00712851 (cheat sheet) |
| `IsTilter=yes` | UnitType | 0x00747712 |
| `TooBigToFitUnderBridge=true` | UnitType | 0x0074774e |
| `OpportunityFire=yes` | TechnoType | 0x0071483d |
| `ElitePrimary=` | TechnoType | 0x00712a32 |
| `DamageSmokeOffset` | TechnoType | 0x00713e25 |
| Stage1..Stage6 / EliteStage1..EliteStage6 | TechnoType +0xCDC..+0xD08 | per GATTLING doc §2 |
| `CurrentGattlingStage` / `GattlingValue` / `GattlingCycleCount` runtime fields | TechnoClass +0x140..+0x148 | per GATTLING doc §2 |

---

## 8. TS-legacy filter

| Feature | Status in YR |
|---------|--------------|
| Gattling system (IsGattling, WeaponStages, etc.) | **Live YR** (Gattling Tank + Gattling Cannon). |
| Locomotor `{4A582741-...}` = DriveLocomotionClass | Live. |
| `GattlingCycleCount` (+0x148) | **Dead** — incremented but never read; likely TS-legacy. Safe to omit. |
| `;Image=TTNK`, `;TargetLaser=yes`, `;yes` Crusher, `;was 5` ROT, `;DownReport=...` | INI comments — drafts/disabled. |
| `;90mmE` on ElitePrimary/EliteSecondary | INI comment — alternate weapon idea. |
| `;50` on AG weapon Damage | INI comment — historical balance. |
| `Bullets=yes` on warheads | Live YR utility flag. |
| `;DB Changed how Plate interacts...` author comments on warheads | Documentation, not code. |
| Fog-of-war 0x1000 gate | Not on YTNK. |
| ImmuneToVeins / Subterranean / Tunneling | Not on YTNK. |

---

## 9. Coverage audit

| Section | Coverage |
|---------|----------|
| rulesmd `[YTNK]` — every key | ✅ §1 (50+ keys including all gattling-stage keys and commented drafts) |
| artmd `[YTNK]` — every key | ✅ §2 (20 keys including all 16 FLH entries) |
| All 12 weapon blocks (6 normal + 6 elite) | ✅ §3.1 (table) + §3.2 (block-by-block for non-elite + diffs for elite) |
| 3 warheads (GattWH, SA, SSA) | ✅ §3.3 |
| Projectiles (Invisiblelow, Invisible4) | ✅ §3.5 |
| Voices / sounds (10 bindings including 3 stage-specific loops) | ✅ §4 |
| Owners / prereqs / tech | ✅ §5 |
| Veterancy (including EliteStage threshold halving) | ✅ §6 |
| Hardcoded behavior — Ghidra-verified | ✅ §7 (**3 NEW field-scope verifications**: IsGattling @ 0x0071401d, WeaponStages @ 0x00714037, RateUp @ 0x00714051, all TechnoType; full state machine cross-ref to GATTLING_WEAPON_STAGE doc) |
| TS-legacy filter | ✅ §8 |
| Cross-references | ✅ at top + inline |
| INI quirks: redundant FLH bloat, elite AA damage downgrade | ✅ §3.2, §7.3 |
| RateDown grace-period balance explanation | ✅ §7.4 |

---

## 10. Quick implementer summary

To make a YTNK-equivalent:

1. **Render** — voxel + HVA; spinning turret animation (visible barrel spin should scale with current stage — at higher stages, barrels spin faster). Standard 8-direction muzzle anim (MGUN-* / GUNFIRE / VTMUZZLE) per stage.
2. **Movement** — DriveLocomotionClass (ground, MovementZone=Destroyer); Speed=6, ROT=10.
3. **Gattling system core (per-instance state):**
   - `CurrentGattlingStage` (int): 0..WeaponStages-1.
   - `GattlingValue` (int): accumulator [0, Stage[WeaponStages]].
   - On each successful fire: call `IncreaseGattlingStage(unit)`:
     - `GattlingValue += RateUp`.
     - Cap at `Stage[WeaponStages]`.
     - If `GattlingValue >= Stage[CurrentStage+1]`: advance stage, swap loop audio.
   - On each AI tick when NOT firing: call `UpdateGattlingStage(unit)`:
     - `GattlingValue -= RateDown × ticks_since_last_fire`.
     - If `RateDown == 0`: drop to 0 immediately.
     - If `GattlingValue < Stage[CurrentStage]`: drop stage, swap loop audio.
4. **Weapon selection per fire:**
   - Determine target type (ground/air).
   - Weapon index = `CurrentGattlingStage × 2 + (target_is_air ? 1 : 0)`.
   - Pick `EliteWeapon[index]` if unit is elite, else `Weapon[index]`.
   - Resolved weapon's Range/Damage/Warhead/ROF/Burst/Anim/Report all apply.
5. **Looping audio:** swap to `Report` of current weapon on stage change; stop loop when not firing.
6. **Temporal interaction:** when target enters TemporalClass warp, decay its gattling stage to 0.
7. **Veterancy:** Vet uses non-elite weapons and stages. Elite uses EliteWeapon[] AND EliteStage[] thresholds (typically halved for faster ramp).
8. **Audio** — Gattling-tank-unique voice set; FlakTrackMoveStart shared with Soviet Flak Track.
9. **AI flags** — High ThreatPosed=40 (AI prioritizes destroying gattling tanks); OpportunityFire=yes; standard tank pathing.
10. **Build gate** — YAWEAP prerequisite (no Battle Lab); YuriCountry only.
11. **Vestigial field**: `GattlingCycleCount` (+0x148) — increment on fire if porting verbatim, but no consumer reads it; safe to omit.
12. **FLH bloat warning**: the artmd block has 16 identical FLH entries; a single `PrimaryFireFLH=160,30,150` with engine-default fallback for the per-weapon overrides is functionally equivalent and cleaner.

The Gattling Tank requires the dedicated multi-stage weapon-cycling state machine documented in GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT. The system is reusable for any IsGattling=yes unit/building (currently YTNK and YAGGUN).
