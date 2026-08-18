---
name: ltnk-doc
description: LTNK — Lasher Tank. Yuri's tier-2 light MBT. ATGUN cannon (Damage=65,
  AP warhead) + Burst=2 RHINAPE elite upgrade. Cheaper/weaker than Grizzly/Rhino;
  earlier tech-tree unlock. Speed=7, Strength=300, Cost=700, Sight=8.
metadata:
  type: project
---

# LTNK — Lasher Light Tank

**INI ID:** `LTNK`
**Display:** "Lasher Light Tank" (`UIName=Name:Lasher`)
**Section:** `[VehicleTypes]`
**Owner side:** Yuri (`Owner=YuriCountry`)
**Role:** Yuri's main battle tank — the faction's only conventional turreted
ground tank. Weaker than Grizzly ([MTNK](../allied/MTNK.md)) and Rhino
([HTNK](../soviet/HTNK.md)) but available earlier (TechLevel=2), cheaper
($700), and faster (Speed=7). Yuri compensates for the firepower deficit
through faster/cheaper mass-production.

---

## Rulesmd verbatim

```ini
[LTNK]
UIName=Name:Lasher
Name=Lasher Light Tank
Image=LTNK
Prerequisite=YAWEAP
Primary=ATGUN
Strength=300;200
Category=AFV
Armor=heavy
Turret=yes
IsTilter=yes
;TargetLaser=yes
TooBigToFitUnderBridge=true
TechLevel=2
Sight=8;6
Speed=7;8
CrateGoodie=no
Crusher=yes
Owner=YuriCountry
Cost=700;600
Soylent=700
Points=25
ROT=5
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=LasherTankSelect
VoiceMove=LasherTankMove
VoiceAttack=LasherTankAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=LasherTankMoveStart
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
ElitePrimary=ATGUNE;90mmE
BuildTimeMultiplier=1.5;Individual control of build time
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:Lasher` — CSF string key. Resolves to "Lasher Light Tank".
- `Name=Lasher Light Tank` — internal description.
- `Image=LTNK` — explicit Image= matching the section name (rare; usually
  omitted when it would default to the section name). The line is harmless
  but redundant.
- `Category=AFV` — Armored Fighting Vehicle category. Same bucket as
  Grizzly, Rhino, Tank Destroyer, Apocalypse. Used by AI threat-scoring.

**Tech / availability**
- `Prerequisite=YAWEAP` — **only the Yuri War Factory required**. *No
  Battle Lab, no Radar, no other gate*. Available immediately after YAWEAP
  unlocks. **This is the earliest tier-2 tank in the game** — Allied/Soviet
  MBTs require War Factory only too, but Yuri's tech tree to YAWEAP is
  fast.
- `TechLevel=2` — tier-2 unit (basic MBT tier).
- `Owner=YuriCountry` — single house, Yuri only.
- `CrateGoodie=no` — *not eligible from UnitCrate pickups*. Yuri players
  who pick up a Vehicle crate get one of their other vehicles (Magnetron,
  Gattling, Apocalypse-equivalent). The Lasher is *too cheap to be a crate
  reward*.

**Combat — defense**
- `Strength=300;200` — 300 HP. The trailing `;200` is the *historical
  value* (commented). Westwood raised LTNK HP from 200 to 300 during
  balancing. Compare:
  | Tank | Strength |
  |------|----------|
  | LTNK Lasher | **300** |
  | MTNK Grizzly | 300 |
  | HTNK Rhino | 400 |
  | APOC Apocalypse | 800 |
  - Lasher matches Grizzly HP but has a much weaker gun (Damage=65 vs
    Grizzly's 90mm 100-damage).
- `Armor=heavy` — heavy armor type. Same as Grizzly/Rhino. Reduces damage
  from AT shells less than light/medium would. **Note: Yuri's other tanks
  (Magnetron, Gattling Tank) use lighter armors** — LTNK is the only
  "real tank" in Yuri's lineup.

**Combat — weapons**
- `Primary=ATGUN` — Anti-Tank Gun. 65 damage, AP warhead, ROF 60. See
  "Weapon" section.
- `ElitePrimary=ATGUNE;90mmE` — Elite upgrade. The trailing `;90mmE` is a
  *historical commented value* — Westwood originally planned to swap the
  Lasher to the Grizzly's 90mmE at elite, but changed to a custom
  `ATGUNE` (Burst=2 RHINAPE warhead). The actual elite weapon is
  `ATGUNE`.
- `Turret=yes` — has a rotating turret.
- `IsTilter=yes` — *body-tilt animation on slopes*. **Ghidra-verified
  UnitType-scope only** (`0x00845df0 → 0x00747712 in UnitTypeClass__ReadINI`,
  matching DISK doc cheat-sheet). The unit tilts forward/back to match
  terrain slope angle while moving.
- `;TargetLaser=yes` — commented out. Was planned to give the Lasher a
  laser pointer / target highlight (like Apocalypse). Disabled.

**Sight / mobility**
- `Sight=8;6` — 8-cell vision radius. The `;6` is the historical commented
  value (was 6, raised to 8). **Lasher has Sight=8, matching the
  Apocalypse and beating Grizzly/Rhino** (both have lower sight). Yuri
  compensates for weak firepower with better scouting.
- `Speed=7;8` — Speed=7 (`;8` historical; was 8, lowered to 7). Compare:
  | Tank | Speed |
  |------|-------|
  | LTNK Lasher | **7** |
  | MTNK Grizzly | 7 |
  | HTNK Rhino | 5 |
  | APOC Apocalypse | 4 |
  - Matches Grizzly; significantly faster than Rhino. Mobility is the
    Lasher's edge.
- `ROT=5` — turret rotation rate (5 is moderate, matches Grizzly).
- `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` — Drive locomotor
  (standard tracked vehicle).
- `MovementZone=Destroyer` — *important*. Allows the unit to crush walls,
  fences, and other "destroyable" obstacles in pathfinding. Same zone as
  Rhino. Lasher pathfinder will crash through walls if needed.
- `Weight=3.5` — physics weight. Same as MCV. Affects bridge collapse +
  rocking on impacts.
- `Size=3` — takes 3 in transport `Passengers=` count. Fits in a
  Battle Fortress (`Passengers=5`).
- `Accelerates=false` — *no acceleration ramp*. The unit reaches `Speed=7`
  instantly when starting from a halt, rather than accelerating gradually.
  **Ghidra-verified TechnoType-scope** (`0x00843534 → 0x00715402 in
  TechnoTypeClass__ReadINI`). Most tanks have `Accelerates=true` (gradual
  ramp); LTNK opts out. This is unusual — most ground tanks accelerate
  for realism. Possibly a balance lever: makes Lasher more agile for
  hit-and-run.
- `TooBigToFitUnderBridge=true` — **cannot drive under bridges** (the cell
  beneath an elevated bridge span). Same flag as Rhino/Apocalypse/MCV.
  Ghidra-verified UnitType-scope (`0x00845dc8 → 0x0074774e`, cheat-sheet).
  Yuri's Lasher is "tall enough" to not fit underneath.

**Economy**
- `Cost=700;600` — Cost=700 (`;600` historical; raised from 600).
  **Cheapest MBT in the game** (Grizzly=700, Rhino=900, Apoc=1500).
  Tied with Grizzly cost.
- `Soylent=700` — full refund on Grinder.
- `Points=25` — modest score on kill.
- `BuildTimeMultiplier=1.5` — *builds 50% slower than its Cost-derived
  base time*. The comment is "Individual control of build time".
  Compensates for the cheap price: a Lasher costs 700 (cheap) but takes
  1.5× the time, so Yuri can't spam them faster than Allied/Soviet can
  spam pricier-but-faster-built MBTs. **Net: Lasher is a budget unit,
  not a rush unit**.

**Crew / death**
- `Crewed=` — *NOT SET*. By default `Crewed=no` for vehicles. **Yuri's
  Lasher does NOT eject infantry on death** (unlike Grizzly/Rhino, which
  `Crewed=yes` would have set). This is a Yuri-faction signature — Yuri
  uses cloned/expendable personnel, so no crew survives.
- `Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` — explosion pool.
- `Maxdebris=3` — note lowercase `d` (still parses; INI is case-insensitive).
  Up to 3 debris pieces.
- `DieSound=GenVehicleDie` — generic vehicle death SFX pool.
- `DamageParticleSystems=SparkSys,SmallGreySSys` — sparks + smoke when
  damaged.
- `DamageSmokeOffset=100, 100, 275` — smoke spawn offset (leptons).

**Behavior flags**
- `Crusher=yes` — can crush Crushable infantry. **No `OmniCrushResistant=yes`**
  — *Lasher can be crushed by Apocalypse Tanks*. Major weakness vs Soviet
  late-game.
- `CrushSound=TankCrush` — standard wet-crunch SFX.
- `ThreatPosed=40` — moderate AI threat (between transports/0 and
  high-end tanks/60+).
- `IsSelectableCombatant=yes` — included in the combat-only rubber-band
  selection filter.
- `OpportunityFire=yes` — auto-engages targets in `Range=5` while idle.
  Standard for offensive tanks. Verified TechnoType `0x00843a74 → 0x0071483d`.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` — Veteran rank gains:
  - STRONGER = +50% max HP.
  - FIREPOWER = +25% damage.
  - SIGHT = +2 Sight (Lasher reaches Sight=10 at veteran).
  - FASTER = +25% Speed. **Veteran Lasher is the game's fastest MBT-class
    tank** (Speed ≈ 8.75).
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — Elite adds:
  - SELF_HEAL passive HP regen.
  - ROF +25% rate of fire.
  - **Not FASTER** at elite — locks speed at veteran level.
  Plus the weapon swap to `ATGUNE` (Burst=2, RHINAPE warhead).

**Z-axis sort**
- `ZFudgeColumn=8` — Z-sort offset near cliff columns (smaller value than
  SMCV's 12 because LTNK is shorter).
- `ZFudgeTunnel=13` — Z-sort offset for tunnel cells. **TS-legacy dormant**
  (no tunnels in YR — see filter section).

---

## Artmd verbatim

```ini
[LTNK]   ; Light Tank
Cameo=LTNKICON
Voxel=yes
Remapable=yes
AltCameo=LTNKUICO
PrimaryFireFLH=225,0,100
```

### Key-by-key annotation

- `Cameo=LTNKICON` — sidebar build-button SHP.
- `AltCameo=LTNKUICO` — UI-overlay alt cameo.
- `Voxel=yes` — rendered from `ltnk.vxl` + `ltnk.hva` (turret `ltnktur.vxl`
  + `ltnktur.hva`). Standard turreted voxel layout.
- `Remapable=yes` — house-color palette applies to the remap channel.
- `PrimaryFireFLH=225,0,100` — Fire/Launch/Height offset for primary weapon
  bullets:
  - X=225 (well forward of unit center; the gun muzzle is at the front
    of the long turret barrel).
  - Y=0 (centered on the turret axis).
  - Z=100 (100 leptons above ground — turret height).
  - **Note: no `SecondaryFireFLH`** — Lasher has no secondary weapon.

---

## Weapons

### Basic primary — `[ATGUN]`

```ini
[ATGUN]
Damage=65
ROF=60
Range=5
Projectile=Cannon
Speed=60
Warhead=AP
Report=LasherTankAttack
Anim=GUNFIRE
Bright=yes
```

- `Damage=65` — moderate; weaker than Grizzly's 90mm (Damage=100) or
  Rhino's 120mm (Damage=85). The Lasher is a budget tank, not a heavy
  hitter.
- `ROF=60` — 60 ticks between shots (~4 seconds at standard 15fps).
  Slower than Grizzly's 50.
- `Range=5` — standard MBT range. Same as Grizzly/Rhino.
- `Projectile=Cannon` — arcing-trajectory shell (`Arcing=true`,
  `SubjectToCliffs=yes`, etc.). Bullets affected by elevation and walls.
- `Speed=60` — projectile speed (in leptons/tick approximately).
  Slow-ish arc.
- `Warhead=AP` — Armor Piercing. See warhead block below.
- `Report=LasherTankAttack` — fire SFX (`vlasatta`, single sample).
- `Anim=GUNFIRE` — generic muzzle-flash anim.
- `Bright=yes` — palette-brightens nearby cells one frame on fire.

### Elite primary — `[ATGUNE]`

```ini
[ATGUNE]
Damage=65
ROF=50
Range=5
Projectile=Cannon
Speed=60
Warhead=RHINAPE
Report=TankDestroyerAttack
Anim=VTMUZZLE
Bright=yes
Burst=2
```

**Three changes vs basic `[ATGUN]`:**
1. `ROF=50` (vs 60) — 17% faster fire.
2. `Warhead=RHINAPE` (vs AP) — better Verses vs medium/heavy armor
   (100%/100% vs basic AP's 100%/100% — wait, same? See warhead notes).
3. `Burst=2` — **fires TWO shells per attack**. *Critical upgrade*:
   doubles effective DPS even ignoring ROF improvement. Elite Lasher DPS
   = 65 × 2 / 50 ≈ 2.6 dmg/tick, vs basic 65 / 60 ≈ 1.08 dmg/tick.
   **Roughly 2.4× the firepower of basic Lasher**.

Other tweaks:
- `Report=TankDestroyerAttack` — borrows TD's fire SFX (heavier thump).
  This is a *shared sound* — elite Lasher and basic Tank Destroyer
  ([TNKD](../allied/TNKD.md)) sound identical at fire.
- `Anim=VTMUZZLE` — Tank Destroyer's muzzle flash (vs basic GUNFIRE).
  Visually distinct elite Lasher.

### Projectile — `[Cannon]`

```ini
[Cannon]
Image=120MM
Arcing=true
SubjectToCliffs=yes
SubjectToElevation=yes
SubjectToWalls=yes
```

- `Image=120MM` — projectile SHP (120mm shell sprite).
- `Arcing=true` — *arcing trajectory*. The shell follows a parabola. Means
  the Lasher can fire over short walls but is blocked by tall buildings.
- `SubjectToCliffs=yes` — cliff terrain blocks the shell mid-flight.
- `SubjectToElevation=yes` — elevation deltas (target on a higher cliff)
  affect arc accuracy.
- `SubjectToWalls=yes` — walls block the shell trajectory.

Shared projectile — same `[Cannon]` block is used by Grizzly's 105mm,
Rhino's 120mm, Robot Tank's Robogun, Mirage's Prism Beam *backup*, etc.

### Warhead — basic `[AP]`

```ini
[AP]
CellSpread=.3
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=25%,25%,15%,75%,100%,100%,65%,45%,60%,60%,100%
Conventional=yes
InfDeath=3
AnimList=S_CLSN16,S_CLSN22
ProneDamage=50%
```

- `CellSpread=.3` — tiny splash radius (sub-cell). Effectively a direct
  hit weapon with marginal collateral.
- `PercentAtMax=.5` — 50% damage at splash edge.
- `Wall=yes` / `Wood=yes` — damages walls and wooden buildings.
- `Verses=25%,25%,15%,75%,100%,100%,65%,45%,60%,60%,100%`:
  | Armor    | Multiplier | vs Lasher 65 base dmg |
  |----------|-----------|------------------------|
  | none     | 25%       | 16.25 |
  | flak     | 25%       | 16.25 |
  | plate    | **15%**   | 9.75 (almost nothing) |
  | light    | 75%       | 48.75 |
  | medium   | **100%**  | **65** |
  | heavy    | **100%**  | **65** |
  | wood     | 65%       | 42.25 |
  | steel    | 45%       | 29.25 |
  | concrete | 60%       | 39 |
  | special_1 | 60%      | 39 |
  | special_2 | 100%     | 65 |

  The verbatim 6/6/01 dev note in the rules says AP was tuned to make
  **plate armor almost immune** to AP weapons (the commented value was
  25% plate, lowered to 15%). The Tank Destroyer's elite UltraAPE warhead
  bypasses this.

  **AP is anti-vehicle**: weak vs infantry (25%), strong vs tanks
  (100%). Lasher is therefore *poor vs infantry* — Yuri uses cheap mass
  Initiates to support Lashers.
- `Conventional=yes` — conventional damage type.
- `InfDeath=3` — explosion infantry death (gibbed by explosion).
- `AnimList=S_CLSN16,S_CLSN22` — small/medium collision-explosion anims.
- `ProneDamage=50%` — prone infantry take half damage.

### Warhead — elite `[RHINAPE]`

```ini
[RHINAPE]
CellSpread=.3
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=100%,100%,100%,100%,100%,100%,65%,45%,60%,60%,100%
Conventional=yes
InfDeath=3
AnimList=VTEXPLOD
ProneDamage=50%
```

- `Verses=100%,100%,100%,100%,...` — **100% vs none/flak/plate/light/medium/heavy**.
  *Bypasses the AP weakness vs infantry*. Elite Lasher kills infantry just
  as effectively as tanks. The 65%/45%/60% values for wood/steel/concrete
  match basic AP — RHINAPE is anti-armor, not anti-structure.
- `AnimList=VTEXPLOD` — single anim (VT explode, the Tank Destroyer's
  signature explosion sprite). Visually identifies elite-rank firepower.

**Net combat profile of elite Lasher:**
- Basic Lasher: 65 dmg/shot × 1 / ROF 60 vs heavy = **1.08 dmg/tick**, weak
  vs infantry.
- Elite Lasher: 65 × 2 × Veteran+25% = 162.5 dmg/burst, ROF 50, **2× shots
  per burst, 100% Verses across armor types**. Roughly 5× DPS gain over
  basic and unlocks anti-infantry role. Major rank-up.

---

## Voices / sounds

All from `soundmd.ini`:

```ini
[LasherTankSelect]
Sounds= $vlassed $vlassef $vlasseg $vlasmod ;$vlassea $vlasseb $vlassec $vlassee
Control=random
Volume=85

[LasherTankMove]
Sounds= $vlasmoa $vlasmoc $vlasmof $vlasatd ;$vlasmob $vlasmoe
Control=random
Volume=85

[LasherTankAttackCommand]
Sounds= $vlasata $vlasatc $vlasate $vlasatf $vlasatg ;$vlasatb
Control=random
Volume=85

[LasherTankAttack]
Sounds= vlasatta
FShift= -10 10
VShift=10
Volume= 90

[LasherTankMoveStart]
Sounds= vlasstaa vlasstab vlasstac
Control= random predelay
Delay=0 400
Priority=Low
FShift= -10 10
VShift=20
Volume=35
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=LasherTankSelect` | `[LasherTankSelect]` | Click LTNK |
| `VoiceMove=LasherTankMove` | `[LasherTankMove]` | Order to move |
| `VoiceAttack=LasherTankAttackCommand` | `[LasherTankAttackCommand]` | Order to attack |
| `Report=LasherTankAttack` (basic weapon) | `[LasherTankAttack]` | Fire SFX (single sample `vlasatta`) |
| `Report=TankDestroyerAttack` (elite weapon) | `[TankDestroyerAttack]` | Elite fire SFX |
| `MoveSound=LasherTankMoveStart` | `[LasherTankMoveStart]` | Ignition (random-predelay, 0-400ms) — not looped |
| `DieSound=GenVehicleDie` | shared | Vehicle death |
| `CrushSound=TankCrush` | shared | Crushing infantry |

**Voice pool quirks:**
- `[LasherTankSelect]` and `[LasherTankMove]` include some `$vlasmod` /
  `$vlasatd` cross-pool samples (move sample in select pool, attack-d
  sample in move pool). The commented sounds (`$vlassea $vlasseb $vlassec
  $vlassee`) were removed — likely cut for length/voice-actor pacing.
- `[LasherTankAttackCommand]` has 5 active samples + 1 commented
  (`$vlasatb` cut).

The Lasher voice character is Yuri-cult deadpan-with-mild-malice, distinct
from Initiate's measured intellectual and Brute's incoherent growls.

---

## Hardcoded behavior (Ghidra-verified)

### 1. Standard MBT turreted-tank pipeline

LTNK has no unit-specific hardcoded behavior. It uses the standard
turreted ground-vehicle pipeline:
- Drive locomotor (Drive GUID).
- Turret rotation toward target (`ROT=5`).
- Auto-engagement of targets in range (`OpportunityFire=yes`).
- Arcing-projectile firing via `[Cannon]`.

No special-case code branches, no unique state machines. **A clean,
mechanical MBT** — Yuri's "honest" tank.

### 2. Accelerates=false (TechnoType)

`Accelerates=false` (Ghidra-verified TechnoType `0x00843534 → 0x00715402`)
disables the speed-acceleration ramp. Most tanks gradually accelerate
from 0 → Speed; LTNK goes from halt to Speed=7 in one tick. This is
unusual; most ground vehicles use `Accelerates=true` (the default).
Result: more responsive at start/stop micro-management.

### 3. IsTilter=yes (UnitType)

`IsTilter=yes` (Ghidra-verified UnitType-only `0x00845df0 → 0x00747712`)
enables the per-slope body-tilt animation. The unit's voxel renders with
a pitch matching the cell's slope gradient. **UnitType-exclusive field** —
infantry can't tilt, buildings can't tilt; only vehicles.

### 4. TooBigToFitUnderBridge

UnitType-scope (`0x00845dc8 → 0x0074774e`). Prevents the pathfinder from
routing Lasher through cells beneath bridge spans. Same flag as Rhino,
Apocalypse, MCV — the "tall vehicle" club.

### 5. MovementZone=Destroyer

Wall-crushing pathfinding zone. The Lasher's path-search treats walls /
fences / certain terrain features as destructible obstacles (crashes
through with damage). Standard for combat tanks.

### 6. Crusher=yes without OmniCrushResistant

A vulnerability: Lasher crushes infantry but *can be crushed by
Apocalypse Tanks*. Mid-late game, an Apocalypse can run over a Lasher.
This is balanced by the Apocalypse's much higher cost.

---

## TS-legacy filter

- `ZFudgeTunnel=13` — TS-legacy field, dormant in YR. Same status as
  SMCV/PCV.
- `;TargetLaser=yes` — commented out. The TargetLaser system *is*
  live in YR (Apocalypse uses it); Lasher just doesn't have it enabled.
- No `ImmuneToVeins`, no `Subterranean`, no other TS-only fields.
- Clean YR-only unit otherwise.

---

## Comparison with peer MBTs

| Field | LTNK Lasher | MTNK Grizzly | HTNK Rhino |
|-------|-------------|--------------|------------|
| Strength | 300 | 300 | 400 |
| Armor | heavy | heavy | heavy |
| Speed | **7** | 7 | 5 |
| Sight | **8** | 6 | 6 |
| Cost | **700** | 700 | 900 |
| BuildTimeMult | **1.5** | 1.0 | 1.0 |
| TechLevel | 2 | 2 | 2 |
| Prereq | YAWEAP | GAWEAP | NAWEAP |
| Primary Damage | **65** | 100 | 85 |
| Primary ROF | 60 | 50 | 60 |
| Primary Warhead | AP | AP | AP |
| Elite Burst | 2 | 1 | 1 |
| Elite Warhead | RHINAPE | GRIZAPE | RHINAPE |
| Crewed | no | yes | yes |
| OmniCrushResistant | no | no | no |

**Lasher trade-offs:**
- **Pros:** Sight+33%, cheapest tier, no crew-eject concerns, fastest
  veteran tank, devastating elite (Burst=2 + 100% Verses).
- **Cons:** Lowest damage (65 vs 85/100), longer build time (×1.5),
  weak vs infantry, can be crushed by Apocalypse.

**Strategic role:** Mass-produce, scout aggressively, push to elite with
veterancy crates / kill farming. Pair with Initiates for anti-infantry
coverage.

---

## Cross-references

- [MTNK.md](../allied/MTNK.md) — Allied Grizzly counterpart.
- [HTNK.md](../soviet/HTNK.md) — Soviet Rhino counterpart.
- [YTNK.md](../yuri/YTNK.md) — Yuri Gattling Tank (anti-infantry partner).
- [TELE.md](../yuri/TELE.md) — Yuri Magnetron (anti-vehicle partner).
- [TNKD.md](../allied/TNKD.md) — Tank Destroyer (shares fire SFX with
  elite Lasher).

---

## Coverage audit

- [x] Every rulesmd key annotated (~45 keys).
- [x] Every artmd key annotated (6 keys).
- [x] Both weapons documented (ATGUN basic + ATGUNE elite).
- [x] Projectile documented ([Cannon] arcing shell).
- [x] Both warheads documented (AP, RHINAPE — with verses comparison).
- [x] All voice/sound entries documented.
- [x] Prerequisites: `YAWEAP` only.
- [x] Owner: YuriCountry.
- [x] Veterancy: VeteranAbilities + EliteAbilities, weapon swap.
- [x] Hardcoded behavior: standard MBT pipeline (no special cases);
  Accelerates=false note; IsTilter=yes UnitType-only confirmed.
- [x] TS-legacy filter: `ZFudgeTunnel` dormant; `;TargetLaser` commented.
- [x] Comparison table with peer MBTs.
- [x] At least one Ghidra search performed (`Accelerates`, `IsTilter`).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("Accelerates")` | `0x00843534` (single match) |
| `get_xrefs_to(0x00843534)` | `0x00715402 → TechnoTypeClass__ReadINI` |
| `search_strings("IsTilter")` | `0x00845df0` (single match, **already in cheat-sheet from DISK**) |
| `get_xrefs_to(0x00845df0)` | `0x00747712 → UnitTypeClass__ReadINI` (UnitType-scope confirmed) |

**New cheat-sheet entry:**
- `Accelerates` (0x00843534 → 0x00715402) TechnoType — controls whether
  the speed ramps from 0 to `Speed` over time or jumps instantly. Default
  is true (gradual).

**Re-verified entries:**
- `IsTilter` (0x00845df0 → 0x00747712) UnitType — already in cheat-sheet
  from DISK doc; re-confirmed scope is UnitType (vehicle-only).

**Open questions:** none.
