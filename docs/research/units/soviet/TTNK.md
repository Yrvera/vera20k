# TTNK — Tesla Tank (Soviet electro-MBT)

**Side classification:** Soviet (Owner=Russians,Confederation,Africans,Arabs;
**RequiredHouses=Russians** — Russian native build, SecretUnits universal pool).
**Role:** Soviet tier-3 mobile electro-tank. Fires `TankBolt` Tesla weapon (Damage=135,
Range=4, Warhead=Electric — InfDeath=5, 200% vs special_1, Wall=yes). At elite rank,
swaps to `TankBoltE` which uses the `Electricbounce` projectile —
`ShrapnelWeapon=TeslaFragment, ShrapnelCount=2` — **chain-lightning that strikes 2
additional targets after the primary hit**. Plus the elite weapon extends Range 4 → 6.

> Output bar: the elite chain-lightning is the unit's defining late-game upgrade. The
> Electricbounce projectile mechanic — `ShrapnelWeapon`/`ShrapnelCount` triggering
> secondary bullet spawns at impact — must reproduce gamemd's "Tesla Tank
> auto-electro-jumps to nearby enemies" behavior exactly. Bounce target selection
> (nearest? random?) and the per-bounce damage falloff (if any) are parity-critical.

> **Completes the SecretUnits trio** — TNKD ([`allied/TNKD.md`](../allied/TNKD.md)),
> DTRUCK ([`soviet/DTRUCK.md`](./DTRUCK.md)), TTNK (this doc). All three are
> `RequiredHouses=`-locked natives, universally unlocked via Tech Secret Lab capture
> (per `[General] SecretUnits=TNKD,TTNK,DTRUCK`).

> Ghidra confirms `gamemd.exe` contains no plain `"TTNK"` string — only `Name:TTNK`
> CSF lookup at 0x008299a0. All behavior is generic flag-driven.

---

## 1. `rulesmd.ini` — `[TTNK]` verbatim

```ini
[TTNK]
UIName=Name:TTNK
Name=Tesla Tank
Prerequisite=NAWEAP,NARADR
Primary=TankBolt
Strength=300
Category=AFV
Armor=heavy
Turret=yes
IsTilter=yes
TooBigToFitUnderBridge=true
TechLevel=10
Sight=8
Speed=6
CrateGoodie=yes
Crusher=yes
Owner=Russians,Confederation,Africans,Arabs
RequiredHouses=Russians
Cost=1200
Soylent=1200
Points=25
ROT=5
IsSelectableCombatant=yes
AllowedToStartInMultiplayer=no
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=TeslaTankSelect
VoiceMove=TeslaTankMove
VoiceAttack=TeslaTankAttackCommand
CrushSound=TankCrush
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=TeslaTankMoveStart
Maxdebris=3
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
;MovementZone=Normal ;gs FLAW needs to be changed to this when The Flaw is fixed
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
ElitePrimary=TankBoltE
BuildTimeMultiplier=1.2 ;Individual control of build time
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:TTNK` | AbstractType | CSF lookup (verified at 0x008299a0 — CSF table only, no code-side hardcoded reference). |
| `Name` | `Tesla Tank` | AbstractType | Dev fallback. |
| (no `Image=` line) | — | — | TTNK reads its own `[TTNK]` artmd block directly — no redirect. |
| `Prerequisite` | `NAWEAP,NARADR` | TechnoType | Soviet War Factory + Radar — mid-tier prereq. |
| `Primary` | `TankBolt` | TechnoType | Tesla bolt weapon (Damage=135, Range=4, Warhead=Electric). See §3. |
| `Strength` | `300` | AbstractType | 300 HP — same as Grizzly Tank. Tied for mid-armor MBT. |
| `Category` | `AFV` | TechnoType | AFV classifier. |
| `Armor` | `heavy` | TechnoType | Verses-slot 6. |
| `Turret` | `yes` | UnitType | Rotating turret — fires while moving. |
| `IsTilter` | `yes` | UnitType | Voxel hull tilts on slopes. |
| `TooBigToFitUnderBridge` | `true` | UnitType-only | Cannot path under low bridges. |
| `TechLevel` | `10` | TechnoType | Highest tech level — combined with `RequiredHouses=Russians`, TTNK is endgame for Russian players only (unless Secret Lab grants). |
| `Sight` | `8` | TechnoType | 8-cell reveal — longer than TankBolt's Range=4. TTNK can spot beyond its own firing range. |
| `Speed` | `6` | TechnoType | Same as Rhino. Slower than Grizzly (7) but faster than DTRUCK (5). |
| `CrateGoodie` | `yes` | UnitType | Can drop from crates. |
| `Crusher` | `yes` | TechnoType | Crushes infantry. |
| `Owner` | 4 Soviet houses | TechnoType | All Soviet houses can own. |
| `RequiredHouses` | `Russians` | TechnoType (verified prior iter — 0x00843bb4) | **Only Russia natively builds TTNK** from its War Factory. Other Soviet houses (Confederation, Africans, Arabs) unlock via Secret Lab. |
| `Cost` | `1200` | TechnoType | $1200 — more expensive than Rhino ($900), cheaper than Apocalypse ($1750). |
| `Soylent` | `1200` | TechnoType | 100% Grinder refund. |
| `Points` | `25` | TechnoType | Standard score on kill. |
| `ROT` | `5` | TechnoType | Turret + body rotation. |
| `IsSelectableCombatant` | `yes` | TechnoType | Selectable. |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Not preplaced. |
| `Explosion` | `TWLT070,...` | TechnoType | Standard death pool. |
| `VoiceSelect` | `TeslaTankSelect` | TechnoType | 5 unique clips ($vtessea..ee). |
| `VoiceMove` | `TeslaTankMove` | TechnoType | 5 unique clips. |
| `VoiceAttack` | `TeslaTankAttackCommand` | TechnoType | 4 active clips ($vtesata..te; $vtesatc commented out — disabled). |
| `CrushSound` | `TankCrush` | TechnoType | Standard. |
| `VoiceFeedback` | *(empty)* | TechnoType | No under-attack voice. |
| `DieSound` | `GenVehicleDie` | TechnoType | Generic. |
| `MoveSound` | `TeslaTankMoveStart` | TechnoType | **2 clips only** (vtesstaa, vtesstab) — smaller pool than most. Predelay 0–400ms, low priority, VShift +15, vol 35. Missing `FShift` parameter. |
| `Maxdebris` | `3` | TechnoType | 3 debris pieces. |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass. |
| Commented `;MovementZone=Normal ;gs FLAW...` | — | — | Same FLAW workaround pattern. |
| `MovementZone` | `Destroyer` | TechnoType | Can traverse some crushable terrain. |
| `ThreatPosed` | `40` | TechnoType | High AI threat (tied with Rhino/Apocalypse). |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | |
| `DamageSmokeOffset` | `100, 100, 275` | TechnoType | Same as Rhino, BFRT, APOC, MIND. |
| `Weight` | `3.5` | TechnoType | Standard tank weight. |
| `VeteranAbilities` | `STRONGER,FIREPOWER,SIGHT,FASTER` | TechnoType | No ROF at veteran. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Elite adds SELF_HEAL + ROF. |
| `Accelerates` | `false` | TechnoType | No accel ramp. |
| `ZFudgeColumn` | `8` | UnitType | Standard. |
| `ZFudgeTunnel` | `13` | UnitType | TS-legacy. |
| `Size` | `3` | TechnoType | Transport slot. |
| `ElitePrimary` | `TankBoltE` | TechnoType | **Elite weapon swap — KEY upgrade.** Damage 135→150, ROF 75→60, Range 4→6, **and Projectile=`Electricbounce`** triggering chain-lightning to 2 additional targets via ShrapnelWeapon=TeslaFragment. See §3. |
| `BuildTimeMultiplier` | `1.2` | TechnoType | Build time is 1.2× cost-derived default — slower-than-cost build (similar to Rhino's 1.3×, faster than Grizzly's 1.5×). |

### Notable absent keys
- No `Image=` — own art block.
- No `Secondary=` — single Tesla weapon.
- No `OpportunityFire=yes` — TTNK does NOT auto-target. Manual attack orders only. Combined with strong `ElitePrimary`, this enforces deliberate-strike playstyle.
- No `Bunkerable=no` (defaults yes — TTNK can ride Battle Fortress).
- No `OmniCrusher` / `OmniCrushResistant`.
- No `Teleporter=`.
- No `ImmuneToPsionics` — **Yuri can mind-control TTNK** (significant counter).
- No `SelfHealing=yes` at rookie (only at elite via SELF_HEAL ability).
- No `Trainable=no` — TTNK **CAN** gain veterancy, unlike DTRUCK/CIVAN/MIND. Elite upgrade is meaningful.

---

## 2. `artmd.ini` — `[TTNK]` section

```ini
[TTNK]   ; Tesla tank
Voxel=yes
Remapable=yes
Cameo=TTNKICON
AltCameo=TTNKUICO
PrimaryFireFLH=60,0,100
ElitePrimaryFireFLH=60,0,100
```

| Key | Value | Effect |
|-----|-------|--------|
| `Voxel` | `yes` | Voxel-rendered from `TTNK.VXL`. |
| `Remapable` | `yes` | House-color remap. |
| `Cameo` | `TTNKICON` | Sidebar cameo. |
| `AltCameo` | `TTNKUICO` | Yuri-skinned cameo (if Yuri captures). |
| `PrimaryFireFLH` | `60,0,100` | Bolt origin: X=60 forward, Y=0 centered, Z=100 turret-coil height. |
| `ElitePrimaryFireFLH` | `60,0,100` | **Same FLH as Primary** — the elite weapon fires from the same coil position. No separate elite muzzle position needed for the chain-lightning visual (the bounce projectile spawns from the impact, not from TTNK). |

Notable absent:
- No `TurretOffset=` — defaults to voxel's hardcoded turret pivot.
- No `SecondaryFireFLH=` — no Secondary weapon.

---

## 3. Weapons — `[TankBolt]` / `[TankBoltE]`

### `[TankBolt]` (rookie/veteran)

```ini
[TankBolt]
Damage=135
ROF=75  ;changed on 11/29 from 60 to 75
Range=4
Speed=100
Warhead=Electric
Report=TeslaTankAttack
Projectile=InvisibleLow
IsElectricBolt=true
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `135` | Per-hit damage — between Grizzly (65) and Rhino (90), heavier than both per-shot. |
| `ROF` | `75` | INI comment: "changed on 11/29 from 60 to 75". Slower than original — author nerf. 75-tick cooldown = ~1.25s/shot. |
| `Range` | `4` | **Short range** — 4 cells. Combined with Sight=8, TTNK can be flanked at distance. |
| `Speed` | `100` | Bolt speed (irrelevant — IsElectricBolt instant). |
| `Warhead` | `Electric` | See §4. Verses 100/100/100/85/100/100/50/50/50/200/100 — strong vs all units, weak vs buildings, 2× vs special_1 (Terror Drone armor). |
| `Report` | `TeslaTankAttack` | Per-shot fire sound. |
| `Projectile` | `InvisibleLow` | Inviso projectile shell (the bolt visual is handled by IsElectricBolt). |
| `IsElectricBolt` | `true` | WeaponType (verified — 0x008492e4 → WeaponTypeClass__ReadINI @ 0x00772854). **Hardcoded flag.** Triggers the engine's Tesla-bolt rendering pipeline (drawn arc between firer and target with electric VFX). Per the cheat sheet: `IsElectricBolt` is paired with `IsAlternateColor`, `IsLine` and other render-channel flags for special weapon visuals. |

### `[TankBoltE]` (elite — chain-lightning)

```ini
[TankBoltE]
Damage=150
ROF=60 ;changed on 11/29 from 50 to 60
Range=6
Speed=100
Warhead=Electric
Report=TeslaTankAttack
Projectile=Electricbounce
IsElectricBolt=true
```

| Key | TankBolt | TankBoltE | Δ |
|-----|----------|-----------|----|
| `Damage` | 135 | **150** | +11% damage |
| `ROF` | 75 | **60** | Faster — INI history "changed on 11/29 from 50 to 60" (nerfed) |
| `Range` | 4 | **6** | +50% range — the biggest elite quality-of-life gain |
| `Speed` | 100 | 100 | Same |
| `Warhead` | Electric | Electric | Same warhead |
| `Report` | TeslaTankAttack | TeslaTankAttack | Same sound |
| `Projectile` | InvisibleLow | **`Electricbounce`** | **Chain-lightning projectile — see §3.1** |
| `IsElectricBolt` | true | true | Same |

**Elite DPS calculation** (vs `none` armor, Verses 100%):
- Rookie: 135 × 1.0 / 75 = 1.80 dmg/tick
- Elite: 150 × 1.0 / 60 = 2.50 dmg/tick → **+39% DPS**
- **PLUS** chain-lightning hits 2 additional targets per shot → effective DPS in target-rich environments is ~3× rookie.

### 3.1 Projectiles

#### `[InvisibleLow]` (rookie)

Standard inviso projectile that respects cliffs/elevation/walls — referenced earlier.

#### `[Electricbounce]` (elite — chain-lightning)

```ini
[Electricbounce]
ShrapnelWeapon=TeslaFragment
ShrapnelCount=2
Inviso=yes
Image=none
SubjectToCliffs=yes
SubjectToElevation=no
SubjectToWalls=no
```

| Key | Value | Effect |
|-----|-------|--------|
| `ShrapnelWeapon` | `TeslaFragment` | **The chain-lightning sub-weapon.** When this projectile detonates on the primary target, it spawns secondary projectiles using `TeslaFragment` as their weapon definition. Each fragment hits a nearby target with TeslaFragment's damage. |
| `ShrapnelCount` | `2` | **2 secondary bounces per primary hit.** A TankBoltE shot hits the primary target with `Electric` warhead, then 2 nearby enemies get hit by `TeslaFragment` shots. |
| `Inviso` | `yes` | No visible projectile body. |
| `Image` | `none` | No sprite. |
| `SubjectToCliffs` | `yes` | Cliffs block. |
| `SubjectToElevation` | `no` | Ignores elevation (electric bolts don't drop with terrain). |
| `SubjectToWalls` | `no` | **Walls do NOT block bounces** — TeslaFragment can chain through cover. |

**Chain mechanic semantics**: per the standard YR ShrapnelWeapon convention (used by other chain weapons like SuperComet), `ShrapnelCount` secondary shots are spawned at the primary impact point and target nearby enemies. Selection algorithm (nearest? line-of-sight? random within radius?) is engine-internal — likely nearest within some default radius from the primary impact cell. **Open follow-up**: confirm exact target-pick logic for the 2 chain bolts.

### 3.2 `[TeslaFragment]` (the chain sub-weapon)

Not yet read in this iteration — would need a separate lookup. Brief expected definition based on pattern: damage scaled down from TankBoltE (likely 50-75), short range, Electric warhead, IsElectricBolt=true. Worth grepping in a follow-up.

---

## 4. Warhead — `[Electric]`

```ini
[Electric]
Verses=100%,100%,100%,85%,100%,100%,50%,50%,50%,200%,100%
InfDeath=5
Wood=yes
; SJM: No piff-piff animation -- electric bolts now spawn spark systems instead.
Wall=yes	; SJM: This allows Tesla Coils to destroy bridges (approved by DB)
;CellSpread=.3
;PercentAtMax=.5
AnimList=TSTIMPCT
```

| Slot | Armor | Verses | Notes |
|------|-------|--------|-------|
| 1 | none | 100% | Full vs basic infantry |
| 2 | flak | 100% | Full vs Flak Trooper |
| 3 | plate | 100% | Full vs plate (Tanya/SEAL) |
| 4 | light | 85% | Slightly reduced vs Grizzly/Mirage/IFV |
| 5 | medium | 100% | Full vs medium |
| 6 | heavy | 100% | Full vs heavy MBTs (Rhino, Apocalypse) |
| 7 | wood | 50% | Half-damage vs wood buildings |
| 8 | steel | 50% | Half vs steel |
| 9 | concrete | 50% | Half vs concrete (Tesla actually treats all building armors equally at 50%) |
| 10 | special_1 | **200%** | **Double damage vs special_1 (Terror Drone armor)** — Tesla Tank is a strong drone counter |
| 11 | special_2 | 100% | |

| Key | Effect |
|-----|--------|
| `InfDeath` | `5` — **Electric infantry death** (per InfDeath table: 5=electric, used by Shock/Electric warheads). Infantry caught in the bolt die with the electrocution animation. |
| `Wood` | `yes` — sets wood structures on fire on hit. |
| `Wall` | `yes` — INI comment: "This allows Tesla Coils to destroy bridges (approved by DB)". Damages walls and bridge structures. |
| Commented `;CellSpread=.3 / ;PercentAtMax=.5` | — | Inert. The author considered AoE but the shipped version is single-target (chain-lightning provides the multi-hit effect instead). |
| `AnimList` | `TSTIMPCT` (Tesla impact animation). |
| Comment `; SJM: No piff-piff animation -- electric bolts now spawn spark systems instead.` | Note: standard hit-spark `PIFFPIFF` replaced by particle systems for electric weapons. Visual difference from AP/SA weapons. |

**Strategic note**: Tesla Tank's Electric warhead is **balanced anti-unit**: 100% vs almost all unit armors, 50% vs all buildings, 200% vs drones. The combination makes TTNK a versatile multi-role MBT — kills infantry, MBTs, and drones equally well, but cannot siege buildings effectively.

---

## 5. Voices / sounds

```ini
[TeslaTankSelect]
Sounds=$vtessea $vtesseb $vtessec $vtessed $vtessee
Control=random
Volume=85

[TeslaTankMove]
Sounds=$vtesmoa $vtesmob $vtesmoc $vtesmod $vtesmoe
Control=random
Volume=85

[TeslaTankAttackCommand]
Sounds=$vtesata $vtesatb $vtesatd $vtesate ;$vtesatc
Control=random
Volume=85
```

```ini
[TeslaTankMoveStart]
Sounds=vtesstaa vtesstab
Control=random predelay
Priority=low
Delay=0 400
VShift=15
Volume=35
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=TeslaTankSelect` | 5 unique clips | Click-select |
| `VoiceMove=TeslaTankMove` | 5 unique clips | Move order |
| `VoiceAttack=TeslaTankAttackCommand` | 4 active clips (5th `$vtesatc` commented out) | Attack order |
| `VoiceFeedback=` *(empty)* | — | No under-attack |
| `DieSound=GenVehicleDie` | 6 generic clips | Death |
| `MoveSound=TeslaTankMoveStart` | **2 clips** (smaller than most), predelay 0–400ms, low pri, VShift +15, vol 35 (no FShift) | Engine start |
| `Report=TeslaTankAttack` (weapon) | (in soundmd — referenced) | Per-shot bolt sound |
| `CrushSound=TankCrush` | `vcrusha` | Crushes infantry |

The minimalist `MoveSound` pool (2 clips vs most units' 3-4) and missing `FShift`
parameter give TTNK a slightly less varied engine audio. The Tesla bolt fire sound
(`TeslaTankAttack`) is the unit's signature audio — distinctive "crackle-zap" that
players associate with Soviet electric weapons.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `NAWEAP,NARADR` — Soviet War Factory + Radar.
- **TechLevel** = `10` (endgame).
- **Owner**: 4 Soviet houses (CAN own).
- **`RequiredHouses=Russians`** — only Russia BUILDS natively.
- **`CrateGoodie=yes`** — can drop from crates.
- **`AllowedToStartInMultiplayer=no`** — not preplaced.
- **Cost** = $1200.

### Acquisition paths (same pattern as TNKD, DTRUCK)

| Path | Mechanism |
|------|-----------|
| **Native build (Russia only)** | `RequiredHouses=Russians` |
| **Secret Lab capture** | `[General] SecretUnits=TNKD,TTNK,DTRUCK` — 1-in-3 random grant |
| **Capture from Russian player** | Engineer or mind-control |
| **Crate drop** | `CrateGoodie=yes` |
| **Yuri mind-control** | Steals the TTNK |

### SecretUnits trio comparison

| Unit | Native country | TechLevel | Cost | Role |
|------|----------------|-----------|------|------|
| **TNKD** ([`allied/TNKD.md`](../allied/TNKD.md)) | Germans | 2 | $900 | AT-only Tank Destroyer |
| **DTRUCK** ([`soviet/DTRUCK.md`](./DTRUCK.md)) | Africans (Libya) | 10 | $1500 | Nuclear suicide truck |
| **TTNK** (this doc) | Russians | 10 | $1200 | Tesla electro-MBT |

All three are TechLevel-high (2 for TNKD because it's combat-grade early-tier, 10 for the others because they're endgame specials), with Soviet houses having 2 of 3 (Russia + Libya), Allied having 1 of 3 (Germany). No SecretUnits are Yuri or non-faction-locked.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 TTNK-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `TTNK` | Only `"Name:TTNK"` at 0x008299a0 (CSF lookup) — no plain-ID code reference |

⇒ **No TTNK-specific code path.** All behavior is generic flag-driven.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `IsElectricBolt` | 0x008492e4 | WeaponTypeClass__ReadINI @ 0x00772854 | **WeaponType** |

Plus prior verifications (carried):
- `RequiredHouses` — TechnoType
- `SecretUnits` (global) — RulesClass
- `ShrapnelWeapon` / `ShrapnelCount` — likely BulletType (projectile-class) — not verified this iter; flagged for follow-up

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Builds natively only for Russia | `RequiredHouses=Russians` | Build-availability gate |
| Universal via Secret Lab | `[General] SecretUnits=TNKD,TTNK,DTRUCK` | 1-in-3 grant on CASLAB capture |
| Tesla-bolt visual rendering | `[TankBolt/E] IsElectricBolt=true` | Engine's special electric-bolt arc renderer |
| Strong vs Terror Drones | `[Electric] Verses[10]=200%` | TTNK is a designated anti-drone unit |
| Damages walls / bridges | `[Electric] Wall=yes` + warhead Wood/Wall flags | Tesla can break bridges (author confirmed) |
| Electric infantry death anim | `[Electric] InfDeath=5` | Electrocution death (vs small-arms slumping) |
| Crushes infantry | `Crusher=yes` | |
| Standard veterancy curve | `Trainable` defaults to yes | TTNK gains XP normally |
| Elite chain-lightning | `ElitePrimary=TankBoltE` with `Projectile=Electricbounce, ShrapnelCount=2` | The most dramatic single elite step in YR |
| Cannot path under bridges | `TooBigToFitUnderBridge=true` | |

### 7.4 Behaviors NOT present

- No `OmniCrusher` / `OmniCrushResistant`.
- No `Spawns=` / `Passengers=` / `Teleporter=`.
- No `OpportunityFire=` — strictly manual-attack.
- No `Secondary` weapon.
- No `ImmuneToPsionics` — Yuri can mind-control TTNK.
- No `ImmuneToRadiation` — Desolators damage TTNK.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ZFudgeTunnel=13` | YES | Dormant render value. |
| Commented `;MovementZone=Normal ;gs FLAW...` | n/a (workaround) | Inactive. |
| `[Electric] Wall=yes ; SJM: This allows Tesla Coils to destroy bridges` | n/a | Live mechanic — Tesla units can break bridges. |
| Commented `;CellSpread/PercentAtMax` on Electric warhead | n/a | Inactive — single-target hit; chain provides multi-hit. |

No fog-of-war refs, no real Tiberium refs.

---

## 9. Veterancy

### Veteran (1 chevron) — `STRONGER, FIREPOWER, SIGHT, FASTER`
- `STRONGER` — +25% HP (300 → 375)
- `FIREPOWER` — +25% damage (135 → 169)
- `SIGHT` — +20% sight (8 → 9.6)
- `FASTER` — +20% speed (6 → 7.2 — matches Grizzly rookie speed)

### Elite (2 chevrons) — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative)
- Adds `SELF_HEAL` (passive HP regen)
- Reapplies STRONGER, FIREPOWER, ROF
- `ROF` — −25% ROF (75 → ~56, but TankBoltE's own ROF=60)

**Plus weapon swap**: `TankBolt → TankBoltE`:
- Damage 135 → 150
- ROF 75 → 60
- Range 4 → 6 (+50%)
- Projectile: InvisibleLow → **Electricbounce** (chain-lightning)
- ShrapnelCount 0 → 2

**Practical elite jump in target-rich environment** (3 targets in chain range):
- Rookie: 135 dmg/75 ticks = 1.80 dmg/tick on 1 target
- Elite: (150 + 2 × TeslaFragment damage) / 60 ticks ≈ 3-4 dmg/tick effectively across 3 targets

The chain-lightning is the single most dramatic elite upgrade in the Soviet vehicle
lineup — TTNK transitions from "Rhino-equivalent damage" to "multi-target electro-shredder".

---

## 10. Cross-references

### Direct dependencies
- `[TankBolt]` / `[TankBoltE]` — weapons (§3)
- `[InvisibleLow]` — rookie projectile
- `[Electricbounce]` — elite chain-lightning projectile (§3.1)
- `[TeslaFragment]` — chain sub-weapon (not expanded this iter — TODO)
- `[Electric]` — warhead (§4)
- `[TSTIMPCT]` (artmd) — Tesla impact animation
- `[TTNK]` (artmd) — art block (own; no Image= redirect)
- `[NAWEAP] / [NARADR]` — prereqs
- `[TeslaTankSelect/Move/AttackCommand/Attack/MoveStart]` (soundmd) — voices and sounds
- `[GenVehicleDie] / [TankCrush]` (soundmd) — generic
- `[General] SecretUnits=TNKD,TTNK,DTRUCK` — Secret Lab pool

### Conceptual companions
- **SHK (Tesla Trooper)** ([`soviet/SHK.md`](./SHK.md)) — infantry Tesla unit. Uses `AssaultBolt` weapon — Damage=10, Range=1.83, Warhead=ElectricAssault. TTNK is the vehicle scaled-up version.
- **TESLA (Tesla Coil)** — building defense (TODO). Same `[Electric]` warhead family.
- **TNKD** ([`allied/TNKD.md`](../allied/TNKD.md)) and **DTRUCK** ([`soviet/DTRUCK.md`](./DTRUCK.md)) — SecretUnits triplet siblings.
- **CASLAB (Tech Secret Lab)** — capture rolls 1 of 3 SecretUnits.
- **HTNK (Rhino)** ([`soviet/HTNK.md`](./HTNK.md)) — Soviet "base" MBT. TTNK is the elite-tier electric MBT counterpart.

### Deep-RE docs
- None directly relevant — Tesla mechanics are flag-driven through the standard weapon/warhead path. The `IsElectricBolt=true` electric-bolt arc renderer may have specialized code but no dedicated report has been written.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[TTNK]` rulesmd key explained | ✅ §1 |
| Every `[TTNK]` artmd key explained — `ElitePrimaryFireFLH=PrimaryFireFLH` (no separate elite muzzle) noted | ✅ §2 |
| Both weapons + both projectiles + warhead | ✅ §3–§4 |
| **`Electricbounce` chain-lightning projectile** documented with ShrapnelWeapon=TeslaFragment, ShrapnelCount=2 | ✅ §3.1 |
| **`[Electric]` warhead — 200% vs special_1 (drones)** highlighted | ✅ §4 |
| All voices + smaller 2-clip MoveSound pool noted | ✅ §5 |
| Prereqs / owners / SecretUnits triplet comparison | ✅ §6 |
| **SecretUnits trio comparison table** (TNKD vs DTRUCK vs TTNK) | ✅ §6 |
| Hardcoded behavior — Ghidra search + IsElectricBolt scope verification | ✅ §7 |
| TS-legacy filter | ✅ §8 |
| **Veterancy — elite chain-lightning as the most dramatic elite step** | ✅ §9 |
| Cross-refs to SHK (infantry sibling), SecretUnits trio, related buildings | ✅ §10 |

**Open follow-ups (parity-critical):**
- **`[TeslaFragment]` sub-weapon definition**: not read this iteration. What's the damage, range, ROF of each chain bolt? Likely scaled-down TankBoltE (~50-75 dmg). Brief grep in next iteration.
- **`Electricbounce` ShrapnelWeapon target-selection algorithm**: how does the engine pick the 2 secondary targets? Nearest within radius? Line-of-sight? Random? Worth a Ghidra-trace of `BulletClass::Explode` or `ShrapnelWeapon` handler.
- **`ShrapnelCount`/`ShrapnelWeapon` scope verification**: likely BulletType (projectile-class), but unverified this iter. Add to next iteration.
- **`Electric` warhead `Wall=yes` bridge-destroy behavior**: SJM comment confirms "Tesla Coils to destroy bridges (approved by DB)". Verify TTNK can also break bridges via the same warhead — INI semantics suggest yes (same warhead, same flag).
- **`IsElectricBolt=true` render pipeline**: Ghidra-trace the bolt-arc rendering (firer-to-target line with electric VFX). Likely shares code with Tesla Coil's `IsElectricBolt` rendering.
