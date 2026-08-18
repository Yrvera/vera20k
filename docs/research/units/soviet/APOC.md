# APOC — Apocalypse Tank (Soviet tier-4 heavy MBT)

**Side classification:** Soviet (Owner=Russians,Confederation,Africans,Arabs).
**Role:** Soviet endgame super-tank. 800 HP, dual weapons (anti-armor 120mmx +
anti-air MammothTusk missiles), Burst=2 on both weapons, `Explodes=yes` chain-reaction
death, `SelfHealing=yes` field repair. The most expensive direct-combat unit in the
Soviet lineup at $1750.

> Output bar: the Apocalypse is the unit that wins or loses a Soviet endgame push.
> Dual-weapon firing cadence (AG vs AA target switching), `Explodes=yes` chain damage
> radius, and the `TargetLaser=yes` aim-render must all match gamemd exactly.

> **Companion docs**:
> - [`soviet/HTNK.md`](./HTNK.md) — Rhino, the tier-2 Soviet MBT. APOC is its tier-4 big brother.
> - [`allied/MTNK.md`](../allied/MTNK.md) — Grizzly, the Allied counterpart MBT.
>
> Important INI hookup: APOC's `Image=MTNK` reads the **artmd `[MTNK]` block** (labeled
> "Apocalypse tank" — see §2). The Grizzly does NOT use `[MTNK]` art (it uses
> `[GTNK]`). The legacy naming retains the original RA2 slots: in pre-YR builds the
> Apocalypse occupied the "MTNK" art slot and the Grizzly was "GTNK".

> Ghidra confirms `gamemd.exe` contains no `"APOC"` / `"Apocalypse"` strings — all
> behavior is generic TechnoType flag-driven.

---

## 1. `rulesmd.ini` — `[APOC]` verbatim

```ini
[APOC]
UIName=Name:APOC
Name=Apocalypse
Image=MTNK
Category=AFV
TargetLaser=yes
Primary=120mmx
Secondary=MammothTusk
Strength=800
Explodes=yes
Prerequisite=NAWEAP,NATECH
CrateGoodie=yes
Armor=heavy
Turret=yes
TechLevel=7
Sight=6
Speed=4
Owner=Russians,Confederation,Africans,Arabs
Cost=1750
Soylent=1750
Points=60
ROT=5
Crusher=yes
SelfHealing=yes
Crewed=no
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=ApocalypseSelect
VoiceMove=ApocalypseMove
VoiceAttack=ApocalypseAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=ApocalypseMoveStart
CrushSound=TankCrush
Maxdebris=3
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
Weight=3.5
MovementZone=Destroyer
ThreatPosed=40	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
AllowedToStartInMultiplayer=no
ZFudgeColumn=9
ZFudgeTunnel=15
Size=6
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ElitePrimary=120mmxE
BuildTimeMultiplier=1.0
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:APOC` | AbstractType | CSF lookup. |
| `Name` | `Apocalypse` | AbstractType | Dev fallback. |
| `Image` | `MTNK` | AbstractType | **Reads artmd `[MTNK]` block** ("Apocalypse tank" labeled). Cameo=MTNKICON, AltCameo=MTNKUICO, PrimaryFireFLH=190,25,120 — see §2. |
| `Category` | `AFV` | TechnoType | Armored Fighting Vehicle. |
| `TargetLaser` | `yes` | TechnoType (verified in HTNK iter — 0x00843898) | **Enables target-laser rendering.** When the turret aims at a target, a laser line is drawn from turret to target during the aiming phase. **APOC is the only YR vehicle that explicitly sets `=yes`** — Rhino has `=no`, others omit (default likely `no`). The laser is a visual targeting reticle, presumably for the Apocalypse's "lock on, then fire" feel. |
| `Primary` | `120mmx` | TechnoType | Anti-ground cannon (Damage=100, ROF=80, Range=5.75, Burst=2). See §3.1. |
| `Secondary` | `MammothTusk` | TechnoType | **Anti-air missile** (Damage=50, ROF=80, Range=8, AA=yes Burst=2). See §3.2. Together the two weapons make APOC a self-defending dual-role unit — fires 120mmx at ground, MammothTusk at air, without manual targeting. |
| `Strength` | `800` | AbstractType | **800 HP** — 2× Rhino's 400 HP. Tied with Mastermind for tankiest non-MCV vehicle. |
| `Explodes` | `yes` | TechnoType (verified — 0x0083355c → 0x007122c5; also OverlayType ref) | **Death chain-reaction.** On death, APOC triggers an area-damage explosion using the global `DeathWeapon` (from `[CombatDamage]`) at the death location. The blast can damage adjacent units/buildings — making clustered Apocalypses a self-destructive pile when one dies. Per the cheat sheet, `DeathWeapon` is the global default warhead/damage for `Explodes=yes` units. |
| `Prerequisite` | `NAWEAP,NATECH` | TechnoType | Soviet War Factory **AND Battle Lab** — tier-4 gate. |
| `CrateGoodie` | `yes` | UnitType | Can drop from crates — a free APOC is one of the most valuable crate goodies. |
| `Armor` | `heavy` | TechnoType | Verses-slot 6. |
| `Turret` | `yes` | UnitType | Rotating turret. |
| `TechLevel` | `7` | TechnoType | Combined with NATECH prereq, this is the standard tier-4 gate. |
| `Sight` | `6` | TechnoType | 6-cell reveal — **shorter than Rhino's 8**. Counter-intuitive — a tier-4 tank that sees less than the tier-2. May be to nerf "Apocalypse scout" abuse. |
| `Speed` | `4` | TechnoType | **Slow** — slower than Rhino (6) and Grizzly (7). The Apocalypse is a brawler, not a maneuver unit. |
| `Owner` | `Russians,Confederation,Africans,Arabs` | TechnoType | Soviet only. |
| `Cost` | `1750` | TechnoType | $1750 — 2× Rhino's $900. Most expensive Soviet combat unit (only SMCV at $3000 exceeds it). |
| `Soylent` | `1750` | TechnoType | 100% Grinder refund (relevant if Yuri captures). |
| `Points` | `60` | TechnoType | High score on kill — bigger than HTNK/MTNK (25) by 2.4×. |
| `ROT` | `5` | TechnoType | Turret + body rotation. |
| `Crusher` | `yes` | TechnoType | Crushes infantry. |
| `SelfHealing` | `yes` | TechnoType | **Auto-regenerates HP** (rate from `[General] SelfHealUnitRate`). Crucial — APOC heals during combat lulls, unlike Rhino which doesn't have SelfHealing until elite (via SELF_HEAL ability). |
| `Crewed` | `no` | TechnoType | No infantry survivors on death. |
| `IsSelectableCombatant` | `yes` | TechnoType | Counts in select-all-combat. |
| `Explosion` | `TWLT070,...` | TechnoType | Death-explosion anim pool. |
| `VoiceSelect` | `ApocalypseSelect` | TechnoType | **Unique** 5-clip voice ($vaposea..ee) — Apocalypse has its own voice family, unlike Rhino which shares the generic Soviet vehicle pool. |
| `VoiceMove` | `ApocalypseMove` | TechnoType | 5 unique clips. |
| `VoiceAttack` | `ApocalypseAttackCommand` | TechnoType | 6 unique clips. |
| `VoiceFeedback` | *(empty)* | TechnoType | No under-attack voice. |
| `DieSound` | `GenVehicleDie` | TechnoType | Standard. |
| `MoveSound` | `ApocalypseMoveStart` | TechnoType | Unique engine-start (3 clips, predelay 0–400ms, low pri, FShift ±5, VShift +10, vol 50 — louder than Rhino's vol 30). |
| `CrushSound` | `TankCrush` | TechnoType | Standard. |
| `Maxdebris` | `3` | TechnoType | 3 debris on death. |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass. |
| `Weight` | `3.5` | TechnoType | Standard tank weight. |
| `MovementZone` | `Destroyer` | TechnoType | Same as Rhino — can traverse some crushable terrain. |
| `ThreatPosed` | `40` | TechnoType | High AI threat (same as Rhino). |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Not preplaced. |
| `ZFudgeColumn` | `9` | UnitType | Z-fudge (taller than Rhino's 8). |
| `ZFudgeTunnel` | `15` | UnitType | TS-legacy. |
| `Size` | **`6`** | TechnoType | **Cannot fit in any transport** — same Size as MCV. Battle Fortress max-passenger Size is 3-ish; APOC is too big. |
| `VeteranAbilities` | `STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | TechnoType | **Includes `ROF` at veteran** — unlike Rhino/Grizzly which only get ROF at elite. APOC fires faster at chevron 1. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Adds SELF_HEAL (already had SelfHealing — may stack or boost rate); reapplies STRONGER/FIREPOWER/ROF. |
| `ElitePrimary` | `120mmxE` | TechnoType | Elite primary — Burst=4 (vs rookie's 2). See §3.1. |
| `BuildTimeMultiplier` | `1.0` | TechnoType | **No build-time multiplier** — APOC builds at the cost-derived default. Combined with $1750 cost, this means slowest-per-tank build time in the Soviet lineup (Rhino 1.3× × $900 vs APOC 1.0× × $1750 → APOC takes ~50% longer to build per unit). |

### Notable absent keys
- No `Accelerates=false` (default — has acceleration ramp at low speeds; combined with `Speed=4` makes APOC feel even sluggish).
- No `TooBigToFitUnderBridge=true` — **APOC CAN path under bridges**, despite its size. Quirky — Rhino can't but APOC can. Likely an INI oversight or intentional bridge-friendly design.
- No `ImmuneToVeins` — likely just omitted; TS-legacy dormant either way.
- No `ImmuneToPsionics` — Apocalypse CAN be mind-controlled by Yuri (extremely dangerous — $1750 stolen).
- No `ImmuneToRadiation` — Desolators damage APOC normally.
- No `OmniCrushResistant=yes` — Battle Fortress can squish (theoretical — Allied doesn't natively meet Soviet via crush).
- No `OpportunityFire=yes` — **APOC does NOT auto-target.** Player must explicitly issue attack orders. This is the opposite of Rhino's `OpportunityFire=yes`. Likely because the dual-weapon target acquisition needs explicit input to avoid mis-firing AA at ground or AG at air.

---

## 2. `artmd.ini` — `[MTNK]` (referenced via `Image=MTNK`)

APOC's `Image=MTNK` redirects to:

```ini
[MTNK]   ; Apocalypse tank
Voxel=yes
Remapable=yes
Cameo=MTNKICON
AltCameo=MTNKUICO
PrimaryFireFLH=190,25,120
```

| Key | Value | Effect |
|-----|-------|--------|
| `Voxel` | `yes` | Voxel-rendered from `MTNK.VXL` + `MTNK.HVA` (the Apocalypse's voxel files — original RA2 "Medium Tank" slot). |
| `Remapable` | `yes` | House-color remap. |
| `Cameo` | `MTNKICON` | Sidebar cameo (the iconic Apocalypse twin-cannon icon). |
| `AltCameo` | `MTNKUICO` | Yuri-skinned cameo. |
| `PrimaryFireFLH` | `190,25,120` | Firing offset: X=+190 forward (long twin-cannon barrels), Y=+25 right (right-side cannon offset), Z=+120 (turret height). Asymmetric Y reflects the left-right twin-cannon geometry. |

No `SecondaryFireFLH=` — but the Apocalypse has a Secondary weapon (MammothTusk AA). The Secondary's FLH defaults to the Primary FLH (190,25,120), which means the AA missiles emerge from the same right-cannon position as the AG shells.

The `[MTNK]` art block is shared between rulesmd `[APOC]` and (incorrectly imagined to be) the Grizzly. **The Grizzly does NOT consult this block** — it has its own `Image=GTNK` redirect to `[GTNK]`.

### `[APOCEXP]` — Apocalypse Tank death-explosion animation

```ini
[APOCEXP]  ; Apocalypse Tank Explosion
Report=Explosion13
Crater=yes
Normalized=yes
Translucent=yes
UseNormalLight=yes
```

Used by the `ApocAP` warhead's `AnimList=APOCEXP`. Plays a unique large-scale explosion
when the primary cannon hits — adds visual weight to each Apocalypse shot.

- `Report=Explosion13` — explosion sound clip.
- `Crater=yes` — leaves permanent crater on terrain.
- `Normalized=yes` — frame timing FPS-normalized.
- `Translucent=yes` — alpha-blended.
- `UseNormalLight=yes` — uses scene ambient light.

---

## 3. Weapons — primary `[120mmx]` (anti-ground) + secondary `[MammothTusk]` (anti-air)

### 3.1 `[120mmx]` (rookie/veteran primary)

```ini
[120mmx]
Damage=100
ROF=80
Range=5.75
Projectile=Cannon
Speed=40
Warhead=ApocAP
Report=ApocalypseAttackGround
Anim=APMUZZLE
Burst=2
Bright=yes
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `100` | Per-shot damage — **higher than Rhino's 90** but **slower ROF** (80 vs 65). |
| `ROF` | `80` | 80-tick cooldown. |
| `Range` | `5.75` | Same as Rhino. |
| `Projectile` | `Cannon` | Arcing cannon. |
| `Speed` | `40` | Bullet speed. |
| `Warhead` | `ApocAP` | Apocalypse-specific AP warhead — Verses 25/25/25/75/100/100/**100/100**/70/60/100. Notably 100% vs wood and steel (compare AP's 65/45), making APOC better vs buildings. |
| `Report` | `ApocalypseAttackGround` | Unique ground-fire sound (1 clip vapoat1a, FShift ±10, vol 90 — loud). |
| `Anim` | `APMUZZLE` | "Apocalypse Muzzle" — **not defined in artmd** (no `[APMUZZLE]` block found). Likely falls back to default muzzle render (or to a built-in animation). Worth verifying. |
| `Burst` | **`2`** | **Fires 2 shots per cycle even at rookie** — the twin-cannon visual matches the mechanic. Damage per cycle: 100 × 2 = 200. Compare Rhino rookie (Burst=1, 90/cycle): APOC delivers ~2.2× damage per cycle. |
| `Bright` | `yes` | Lights cell when firing. |

**DPS vs heavy armor (Rhino, MBT)** at rookie:
- APOC: 100 × 2 × 100% / 80 = 2.5 dmg/tick
- Rhino: 90 × 1 × 100% / 65 = 1.38 dmg/tick → **APOC delivers ~1.8× DPS at rookie**.

**Trade math 1v1 APOC vs Rhino at rookie**:
- APOC kills Rhino's 400 HP in 400 / 2.5 = 160 ticks (~9.6s).
- Rhino kills APOC's 800 HP in 800 / 1.38 = 580 ticks (~34.8s).
- **APOC wins by ~25 seconds** of "wasted" damage. Net cost-efficiency: APOC $1750 vs Rhino $900 → APOC is 1.94× cost; deals 1.8× DPS and absorbs 2× HP → roughly cost-neutral (slight edge to APOC). The kicker is the AA Secondary giving APOC a role Rhino cannot fill.

### 3.2 `[MammothTusk]` (anti-air secondary)

```ini
[MammothTusk]
Damage=50
ROF=80
Range=8
Projectile=AAHeatSeeker
Speed=20
Warhead=HE
Burst=2
Report=ApocalypseAttackAir
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `50` | Per-shot. |
| `ROF` | `80` | Same cooldown as primary. |
| `Range` | `8` | **Longer than primary's 5.75** — APOC's AA reach exceeds its AG reach. Aircraft engaged at 8 cells. |
| `Projectile` | `AAHeatSeeker` | Heat-seeker missile (`AA=yes, AG=no, Proximity=yes, Ranged=yes, Image=DRAGON, ROT=80`) — tracks air targets, won't lock on ground. |
| `Speed` | `20` | Slower bullet speed — missile flight. |
| `Warhead` | `HE` | High-Explosive — Verses 100/100/100/70/70/35/75/40/20/80/100. Strong vs all infantry slots; poor vs heavy armor (35%). Optimised for air targets (which typically have light/flak armor). |
| `Burst` | **`2`** | 2 missiles per cycle. |
| `Report` | `ApocalypseAttackAir` | 3 clips (vapoat2a..2c), FShift ±10, VShift +20 — distinguishes AA-fire from AG-fire sound. |

The AA Secondary fires automatically when air targets enter range, regardless of the player's manual orders — but the primary 120mmx does NOT auto-engage ground (no `OpportunityFire=yes`). So a stationary APOC reacts to air threats but ignores ground threats unless ordered.

### 3.3 `[120mmxE]` (elite primary)

```ini
[120mmxE]
Damage=100
ROF=80
Range=5.75
Projectile=Cannon
Speed=40
Warhead=ApocAPE
Report=ApocalypseAttackGround
Anim=VTMUZZLE
Burst=4
Bright=yes
```

| Key | 120mmx | 120mmxE | Δ |
|-----|--------|---------|----|
| `Damage` | 100 | 100 | unchanged |
| `ROF` | 80 | 80 | unchanged |
| `Range` | 5.75 | 5.75 | unchanged |
| `Warhead` | ApocAP | **ApocAPE** | Better vs infantry (slot 1-3: 25/25/25 → 100/100/100) |
| `Anim` | APMUZZLE | VTMUZZLE | Standard elite muzzle (different visual) |
| `Burst` | 2 | **4** | **Doubles burst — fires 4 shots per cycle** |

**Practical jump**: elite Burst=4 + ApocAPE infantry-100% means elite APOC delivers 100 × 4 = 400 dmg per cycle. Vs `none` armor infantry slot: 400 × 100% / 80 = 5.0 dmg/tick (vs rookie's 100 × 2 × 25% / 80 = 0.625 dmg/tick) → **8× DPS vs infantry at elite**. Vs heavy: 400 × 100% / 80 = 5.0 dmg/tick → **2× DPS vs MBTs at elite**.

(No `[MammothTuskE]` exists — the Secondary AA weapon is unchanged at elite. APOC's elite upgrade is anti-ground only.)

### 3.4 Projectiles

- `[Cannon]` (primary) — arcing 120MM Image, respects cliffs/elevation/walls.
- `[AAHeatSeeker]` (secondary) — heat-seeker (Image=DRAGON, ROT=80, Proximity, Ranged, AA=yes, AG=no, SubjectToCliffs=no, SubjectToElevation=no, SubjectToWalls=no — flies over terrain).

---

## 4. Warheads — `[ApocAP]` / `[ApocAPE]` / `[HE]`

### `[ApocAP]` (primary rookie)

```ini
[ApocAP]
CellSpread=.3
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=25%,25%,25%,75%,100%,100%,100%,100%,70%,60%,100%
Conventional=yes
InfDeath=3
AnimList=APOCEXP
ProneDamage=50%
```

| Slot | Armor | Verses | Notes |
|------|-------|--------|-------|
| 1 | none | 25% | Weak vs basic infantry |
| 2 | flak | 25% | Weak vs Flak Trooper |
| 3 | plate | **25%** | **Stronger vs plate than AP/GRIZAPE rookie** (which has 15%) — Apocalypse can actually hurt Tanya, plate-armor units |
| 4 | light | 75% | Strong vs Grizzly/IFV |
| 5 | medium | 100% | Full vs medium |
| 6 | heavy | 100% | Full vs heavy MBTs |
| 7 | wood | **100%** | **Full damage vs wood buildings** (vs AP's 65%) |
| 8 | steel | **100%** | **Full damage vs steel buildings** (vs AP's 45%) |
| 9 | concrete | 70% | Better vs concrete (vs AP's 60%) |
| 10 | special_1 | 60% | |
| 11 | special_2 | 100% | |

The Apocalypse's ApocAP is **massively better vs buildings** than Rhino's AP — slot 7 (100% vs 65%) and slot 8 (100% vs 45%). This is why APOC is the Soviet endgame siege unit: it tears through buildings while Rhinos struggle.

`AnimList=APOCEXP` — unique impact animation (see §2). `InfDeath=3` standard explosion infantry death.

### `[ApocAPE]` (primary elite)

```ini
[ApocAPE]
CellSpread=.3
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=100%,100%,100%,75%,100%,100%,100%,100%,70%,60%,100%
Conventional=yes
InfDeath=3
AnimList=VTEXPLOD
ProneDamage=50%
```

Elite warhead: infantry slots (1-3) all jump to 100%, light/medium/heavy stay at 75/100/100, building slots stay strong. `AnimList=VTEXPLOD` (standard elite tank explosion, not the unique APOCEXP).

### `[HE]` (secondary AA — shared warhead)

```ini
[HE]
CellSpread=.5
PercentAtMax=.5
Wall=yes
Wood=yes
;;DB Changed 7/18/01
;;Verses=100%,90%,80%,70%,35%,35%,75%,40%,20%,80%,100%
;;Verses=100%,100%,100%,70%,35%,35%,75%,40%,20%,80%,100%
Verses=100%,100%,100%,70%,70%,35%,75%,40%,20%,80%,100%
Conventional=yes
Rocker=no
```

Generic High-Explosive warhead — shared with many other AoE weapons. Strong vs infantry (100/100/100), decent vs light (70%), good vs medium (70%) but weak vs heavy (35%). Optimal vs aircraft (which use light/flak armors). CellSpread=0.5 gives a small splash on each missile.

Commented history-Verses lines show two prior tunings (90/80/70/35/35 and 100/100/70/35/35) before the shipped 100/100/70/70/35. The current `medium` slot is 70% (was 35%), making HE viable vs medium-armor units too.

---

## 5. Voices / sounds

```ini
[ApocalypseSelect]
Sounds=$vaposea $vaposeb $vaposec $vaposed $vaposee
Control=random
Volume=85

[ApocalypseMove]
Sounds=$vapomoa $vapomob $vapomoc $vapomod $vapomoe
Control=random
Volume=85

[ApocalypseAttackCommand]
Sounds=$vapoata $vapoatb $vapoatc $vapoatd $vapoate $vapoatf
Control=random
Volume=85
```

```ini
[ApocalypseAttackGround]
Sounds=vapoat1a
FShift= -10 10
Volume=90

[ApocalypseAttackAir]
Sounds=vapoat2a vapoat2b vapoat2c
Control= random
FShift= -10 10
VShift=20
```

```ini
[ApocalypseMoveStart]
Sounds=vapostaa vapostab vapostac
Control= random predelay
Delay=0 400
Priority=Low
FShift= -5 5
VShift=10
Volume=50
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=ApocalypseSelect` | 5 unique clips | Click-select — **unique Apocalypse pool**, not shared Soviet generic |
| `VoiceMove=ApocalypseMove` | 5 unique clips | Move |
| `VoiceAttack=ApocalypseAttackCommand` | 6 unique clips | Attack |
| `VoiceFeedback=` *(empty)* | — | No under-attack |
| `DieSound=GenVehicleDie` | 6 generic clips | Death |
| `MoveSound=ApocalypseMoveStart` | 3 unique clips, predelay 0–400ms, low pri, vol 50 (louder than Rhino's vol 30) | Engine start |
| `Report=ApocalypseAttackGround` (primary) | 1 clip, FShift ±10, vol 90 (loud) | Per-shot AG fire |
| `Report=ApocalypseAttackAir` (secondary) | 3 clips, FShift ±10, VShift +20 | Per-shot AA fire |
| `CrushSound=TankCrush` | `vcrusha` | Crush |

The Apocalypse has the most **distinct audio profile** of any Soviet vehicle: unique select/move/attack voices, unique engine start, separate ground and air fire sounds. Players hearing an Apocalypse engagement know exactly what's happening without looking.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `NAWEAP,NATECH` — Soviet War Factory **AND Battle Lab**. Late-game gate.
- **TechLevel** = `7` (only campaign TechLevel cap above this).
- **Owner**: 4 Soviet countries (no Yuri).
- **`CrateGoodie=yes`** — can drop from crates (rare jackpot).
- **`AllowedToStartInMultiplayer=no`** — never preplaced.
- **Cost** = $1750 — most expensive direct-combat unit.

### Apocalypse vs Rhino comparison

| Aspect | APOC | HTNK (Rhino) |
|--------|------|---------------|
| Cost | $1750 | $900 |
| HP | 800 | 400 |
| Speed | 4 | 6 |
| Primary | 120mmx (Burst=2, 100 dmg, ApocAP) | 120mm (Burst=1, 90 dmg, AP) |
| Secondary | MammothTusk AA missile | none |
| Range | 5.75 + 8 (AA) | 5.75 |
| Sight | 6 | 8 |
| ROT | 5 | 5 |
| Armor | heavy | heavy |
| SelfHealing | **yes** | no (only elite via SELF_HEAL ability) |
| Explodes | **yes** (chain damage) | no |
| TargetLaser | **yes** | no |
| OpportunityFire | **no** (manual AG targeting) | yes |
| BuildTimeMultiplier | **1.0** | 1.3 |
| Veteran ROF | **yes** | no (only elite) |
| Elite Burst | 4 | 2 |
| Prereq | NAWEAP + NATECH | NAWEAP only |
| TechLevel | 7 | 2 |
| Size | 6 | 3 |

APOC is roughly 2× Rhino on most axes (HP, damage per cycle, cost) — but adds AA capability, self-heal, and a death-chain mechanic. The trade-offs: slower speed, shorter sight, no opportunistic fire, much later tech tier.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 APOC-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `APOC` | 0 matches |
| `Apocalypse` | 0 matches |

⇒ **No APOC-specific code path.** All behavior is generic flag-driven.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `Explodes` | 0x0083355c | TechnoTypeClass__ReadINI @ 0x007122c5 + OverlayTypeClass ref | TechnoType + OverlayType |

Plus prior verifications (HTNK iteration carried over):
- `TargetLaser` — TechnoType
- `TooBigToFitUnderBridge` (absent on APOC) — UnitType only
- `OpportunityFire` (absent on APOC) — TechnoType

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Dual-weapon target switching (AG vs AA) | `Primary=120mmx` + `Secondary=MammothTusk` + `AAHeatSeeker.AA=yes,AG=no` + `Cannon.AA=` (unset) | Engine picks Primary vs Secondary based on target type |
| Chain-reaction death damage | `Explodes=yes` → `[CombatDamage] DeathWeapon` global | Per cheat sheet: DeathWeapon is the global default warhead+damage for `Explodes=yes` units. Adjacent units can be damaged in the chain. |
| Target-laser aim indicator | `TargetLaser=yes` | Visual targeting line during turret aim — APOC is one of the only YR units with this |
| Auto-heal during combat lulls | `SelfHealing=yes` (no elite required) | Rate from `[General] SelfHealUnitRate` |
| Burst=2 twin-cannon fire | `[120mmx] Burst=2` | Twin-barrel firing matches the visual model |
| AA-missile auto-engage on air targets | `Secondary` weapon's AA targeting | Player doesn't need to manually order |
| **No** AG opportunistic-fire | `OpportunityFire` absent | Player must explicitly order ground attacks |
| **Cannot** path under bridges (paradoxically possible — `TooBigToFitUnderBridge` absent) | Default behavior | APOC's voxel IS big but the flag is unset — engine allows bridge-under pathing |
| Elite Burst=4 at chevron 2 | `[120mmxE] Burst=4` | Most dramatic single elite step in YR |
| Veteran ROF bonus (not waiting for elite) | `VeteranAbilities=...,ROF,...` | Faster cooldown earlier than Rhino/Grizzly |

### 7.4 Behaviors NOT present

- No `Spawns=` (no child units).
- No `Passengers=` / `OpenTransport` — not a transport.
- No `Teleporter=` — does not chrono-warp.
- No `ImmuneToPsionics` — Yuri can mind-control APOC (extremely punishing — $1750 captured).
- No `ImmuneToRadiation` — Desolators damage APOC.
- No `OmniCrushResistant=yes` — Battle Fortress could squish (theoretical).
- No `Bunkerable=no` — but Size=6 makes APOC ineligible for transports anyway.
- No `Crewed=yes` — no infantry survivors.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ZFudgeTunnel=15` | YES | Dormant render value. |
| (no `ImmuneToVeins`) | — | Not set — but irrelevant anyway. |

No fog-of-war refs, no Tiberium refs, no real tunnels.

---

## 9. Veterancy

### Veteran (1 chevron) — `STRONGER, FIREPOWER, ROF, SIGHT, FASTER`
- `STRONGER` — +25% HP (800 → 1000)
- `FIREPOWER` — +25% damage (100 → 125, MammothTusk 50 → 62)
- **`ROF`** — −25% ROF (80 → ~60 — **APOC fires faster at veteran**, unlike Rhino/Grizzly which only get ROF at elite)
- `SIGHT` — +20% sight (6 → 7.2)
- `FASTER` — +20% speed (4 → 4.8)

### Elite (2 chevrons) — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative)
- Adds `SELF_HEAL` (stacks with/boosts the always-on `SelfHealing=yes`?)
- Reapplies STRONGER, FIREPOWER, ROF

**Plus weapon swap**: `120mmx` → `120mmxE`:
- Burst 2 → **4**
- Warhead ApocAP → ApocAPE (infantry slots 100% — 4× damage to soft targets)
- Anim APMUZZLE → VTMUZZLE

(No `EliteSecondary` — MammothTusk AA missile is unchanged at elite.)

**Practical elite jump**: ~4× DPS vs infantry, ~2× DPS vs MBTs, faster ROF, self-heal boost. An elite Apocalypse is roughly equivalent in firepower to **two rookie Apocalypses** thanks to the Burst=4.

---

## 10. Cross-references

### Direct dependencies
- `[120mmx]` / `[120mmxE]` — primary weapons (§3)
- `[MammothTusk]` — secondary AA weapon (§3)
- `[Cannon]` — primary projectile
- `[AAHeatSeeker]` — secondary projectile
- `[ApocAP]` / `[ApocAPE]` — primary warheads (§4)
- `[HE]` — secondary warhead (shared)
- `[120MM]` / `[DRAGON]` (artmd) — bullet sprites
- `[APMUZZLE]` (artmd — **not defined**, fallback to default) / `[VTMUZZLE]` — muzzle anims
- `[APOCEXP]` / `[VTEXPLOD]` (artmd) — impact anims
- `[MTNK]` (artmd via `Image=MTNK`) — art block (the "Apocalypse tank" labeled entry)
- `[NAWEAP]` / `[NATECH]` — prereqs
- `[ApocalypseSelect/Move/AttackCommand/AttackGround/AttackAir/MoveStart]` (soundmd) — voices and sounds
- `[GenVehicleDie] / [TankCrush]` — generic vehicle sounds
- `[General] DeathWeapon=` — global death-explosion warhead/damage for `Explodes=yes`

### Conceptual companions
- **HTNK (Rhino)** ([`soviet/HTNK.md`](./HTNK.md)) — tier-2 Soviet MBT counterpart.
- **MTNK (Grizzly)** ([`allied/MTNK.md`](../allied/MTNK.md)) — Allied counterpart tier-2 MBT (uses `Image=GTNK`, NOT the artmd `[MTNK]` block).
- **TTNK (Tesla Tank)** ([`soviet/TTNK.md`](./TTNK.md) — TODO) — Soviet tier-3 tank.
- **MIND (Mastermind)** ([`yuri/MIND.md`](../yuri/MIND.md) — TODO) — also Strength=800 (tied for tankiest non-MCV).
- **HOWI (Prism Tank)** ([`allied/HOWI.md`](../allied/HOWI.md) — TODO) — Allied tier-4 vehicle counterpart (different role — long-range siege).

### Deep-RE docs
- None directly relevant — APOC has no unique hardcoded behavior worth a dedicated report. Cross-references the standard combat damage / explode-on-death paths covered in generic warhead docs.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[APOC]` rulesmd key explained | ✅ §1 |
| `Image=MTNK` → artmd `[MTNK]` block expanded | ✅ §2 |
| **MTNK doc retroactively corrected** (artmd [MTNK] is APOC's live art, NOT orphan) | ✅ §2 + MTNK §2 |
| `[APOCEXP]` death-anim included | ✅ §2 |
| Both primary (rookie+elite) + secondary (no elite variant) + all 3 warheads + 2 projectiles | ✅ §3–§4 |
| All voices + 2 unique attack sounds (ground vs air) + unique engine start | ✅ §5 |
| Prereqs / owners / availability | ✅ §6 |
| **APOC vs HTNK detailed comparison table** | ✅ §6 |
| Hardcoded behavior — Ghidra searches + `Explodes` flag-scope verification | ✅ §7 (Explodes is TechnoType + OverlayType — surprising dual-scope) |
| **Dual-weapon target-switching logic** described | ✅ §7 |
| TS-legacy filter | ✅ §8 |
| Veterancy detailed with veteran-ROF (unique among MBTs) + elite Burst=4 | ✅ §9 |
| Cross-refs to companion docs | ✅ §10 |

**Open follow-ups (none load-bearing):**
- `[APMUZZLE]` artmd block doesn't exist — the `120mmx` weapon references it but no definition. Need to verify what gamemd actually renders when `Anim=` points at an undefined artmd entry: fallback to default muzzle? Silent omit? Engine error? Worth checking in a parity audit.
- `Explodes` second xref from `OverlayTypeClass__ReadINI` (0x005fe840) is unusual — investigate whether overlays can be `Explodes=yes` (e.g., Tiberium overlay explodes?). Not load-bearing for APOC but worth knowing.
- `SelfHealing=yes` + elite `SELF_HEAL` ability — stack or boost rate? Not load-bearing but worth a one-pass binary verification.
- The `TargetLaser=yes` rendering — what does the laser look like at the pixel level? Color, length, opacity, timing relative to firing? Worth a fidelity-check pass when implementing.
- APOC's lack of `TooBigToFitUnderBridge=true` (despite Size=6) — confirm in-game whether APOC actually passes under bridges that Rhino can't. May be an intentional design quirk or INI oversight.
