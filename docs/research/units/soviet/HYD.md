---
name: hyd-doc
description: HYD — Soviet Sea Scorpion (NOT "Hydrofoil" as INDEX claimed). Tier-6
  naval AA+anti-surface dual-weapon vessel. FlakTrackGun (Primary, AG) + FlakWeapon
  (Secondary, AA, Range=12). Strength=400, Cost=600 — much cheaper than AEGIS.
  Discovered: FlakScatter + Inaccurate are BulletType-scope fields.
metadata:
  type: project
---

# HYD — Sea Scorpion

**INI ID:** `HYD`
**Display:** "Sea Scorpion" (`UIName=Name:HYD`)
**Section:** `[VehicleTypes]`
**Owner side:** **Soviet** (Russians, Confederation, Africans, Arabs) — NOT
Allied as the previous INDEX claim said.
**Role:** Soviet tier-6 dual-purpose naval AA/anti-surface vessel. Unlike Aegis
Cruiser (AA-only), the Sea Scorpion fires *both* anti-air missiles AND
anti-surface flak rounds. Versatile but lighter (Strength=400 vs Aegis 800),
much cheaper ($600 vs Aegis $1200). Closes the naval AA pair with AEGIS.

---

## INDEX correction recap

Confirmed during this iteration: **HYD = Sea Scorpion (Soviet)**, NOT
"Hydrofoil (Allied)". Index was wrong; verified from rulesmd:

```ini
[HYD]
UIName=Name:HYD
Name=Sea Scorpion
...
Owner=Russians,Confederation,Africans,Arabs
VoiceSelect=SeaScorpionSelect
```

The INI section name `HYD` (3-letter abbreviation) reflects an early RA2
design where this slot was the "Hydrofoil." During YR development Westwood
renamed the unit to Sea Scorpion and revised the weapon loadout from
HoverMissile to FlakTrackGun + FlakWeapon, but kept the `HYD` section
name. **There is no separate "Hydrofoil" unit in YR**.

Index update already logged in iteration 72.

---

## Rulesmd verbatim

```ini
[HYD]
UIName=Name:HYD
Name=Sea Scorpion
Prerequisite=NAYARD,NARADR
;Primary=HoverMissile
Primary=FlakTrackGun
Secondary=FlakWeapon
ToProtect=yes
Category=AFV
Strength=400
Naval=yes
Armor=heavy
TechLevel=6
MovementRestrictedTo=Water
Sight=8
Speed=8
CrateGoodie=no
Owner=Russians,Confederation,Africans,Arabs
AllowedToStartInMultiplayer=no
Cost=600
Soylent=600
Points=20
ROT=6
Crusher=no
Crewed=no
IsSelectableCombatant=yes
Weight=2
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=SeaScorpionSelect
VoiceMove=SeaScorpionMove
VoiceAttack=SeaScorpionAttackCommand
VoiceFeedback=
DieSound=GenSmallWaterDie
MoveSound=SeawolfMoveStart
Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};{4A582741-9839-11d1-B709-00A024DDAFD1}
;SpeedType=Amphibious ;gs Wha!?!
;MovementZone=Amphibious
SpeedType=Float
MovementZone=Water
ThreatPosed=25	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ElitePrimary=FlakTrackGunE
EliteSecondary=FlakWeaponE
Size=20
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:HYD` — CSF key. **Resolves to "Sea Scorpion"** in shipped
  YR. The CSF lookup overrides the historical "Hydrofoil" connotation
  of the section name.
- `Name=Sea Scorpion` — internal description.
- `Category=AFV` — AI threat-bucket.

**Tech / availability**
- `Prerequisite=NAYARD,NARADR` — Soviet Naval Yard **+ Soviet Radar Tower**.
  Slightly different from AEGIS (which needs `RADAR` generic macro
  resolving to any radar). Sea Scorpion specifically requires
  NARADR (Soviet radar tower).
- `TechLevel=6` — tier-6 (vs AEGIS tier-7). Soviet Sea Scorpion unlocks
  one tier earlier than Allied Aegis.
- `Owner=Russians,Confederation,Africans,Arabs` — 4 Soviet sub-factions.
- `AllowedToStartInMultiplayer=no` — not a starting unit.
- `CrateGoodie=no` — not crate-eligible.
- `ToProtect=yes` — AI high-value flag (Ghidra-verified TechnoType
  `0x008438dc → 0x00714be8` from AEGIS doc cheat-sheet).

**Combat — defense**
- `Strength=400` — **half of AEGIS's 800**. Sea Scorpion is fragile.
- `Armor=heavy` — heavy armor (vs AEGIS's surprising light armor). Sea
  Scorpion has *better armor type* but *less raw HP*. Actually balances
  out depending on incoming warhead.

**Combat — dual-weapon system (the defining feature)**

This is the unit's defining mechanic — unlike Aegis (AA-only):
- `Primary=FlakTrackGun` — **anti-surface gun**. Range=5, Damage=25,
  ROF=25, FlakTProj projectile (`Arcing=true`, `AG=yes`, `AA=no`),
  FlakTWH warhead. *Anti-naval and anti-light-armor surface targets*.
- `Secondary=FlakWeapon` — **anti-air gun**. Range=12, Damage=40,
  ROF=20, FlakProj projectile (`Inviso=yes`, `AA=yes`, `AG=no`),
  FlakWH warhead. *Anti-aircraft (Kirov, Disc, Black Eagle, Hornet)*.
- `ElitePrimary=FlakTrackGunE` — Burst=2 elite anti-surface.
- `EliteSecondary=FlakWeaponE` — Burst=2 elite anti-air (Damage=35
  vs 40 — slight reduction, but Burst doubles total).
- `;Primary=HoverMissile` — **commented historical**. The Sea Scorpion
  was originally going to use the HoverMissile (the AA missile shared
  with IFV missile slot). Westwood swapped to FlakTrackGun + FlakWeapon
  during YR balancing.

**Sight / mobility**
- `Sight=8` — matches AEGIS sight.
- `Speed=8` — **fast** (vs AEGIS Speed=4, twice as fast). Sea Scorpion
  is a fast skirmisher.
- `ROT=6` — moderate (vs AEGIS ROT=1, extremely slow). Sea Scorpion is
  agile.
- `MovementRestrictedTo=Water` — **forces water-only pathfinding**.
  **Ghidra-verified UnitType-scope** at `0x00845d64 → 0x00747837` (per
  SMCV cheat-sheet, **re-confirmed** this iteration). Overrides any
  ambiguity in the MovementZone — Sea Scorpion cannot path onto beach
  cells, only into water. Compare with LCRF/SAPC/YHVR which use
  Amphibious MovementZone (can hover over land *and* water).
- `Locomotor={2BEA74E1-...}` — Submarine locomotor (shared with all
  naval). The `;{4A582741-...}` is the commented Drive alternative.
- `SpeedType=Float` — naval speed.
- `MovementZone=Water` — water-only. Combined with
  MovementRestrictedTo=Water, doubly enforced.
- The commented `;SpeedType=Amphibious ;gs Wha!?!` is a Westwood inside
  joke — Greg Smith's reaction to a previous "amphibious" speed type
  designation. Disabled.

**Economy**
- `Cost=600` — **half of AEGIS's $1200**. Sea Scorpion is the cheap
  alternative. Mass-producible in volume.
- `Soylent=600` — full refund.
- `Points=20` — moderate score.

**Crew / death**
- `Crewed=no`, `Crusher=no`, `IsSelectableCombatant=yes` — standard.
- `Weight=2` — moderate physics weight (vs AEGIS Weight=4 — Sea Scorpion
  is lighter).
- `Explosion=TWLT070,...` — explosion pool.
- `DieSound=GenSmallWaterDie` — generic *small-naval* death SFX (vs
  AEGIS's `GenLargeWaterDie` for the large surface ship). Sea Scorpion
  is classified as small naval = quick death animation.

**Voice / sound bindings**
- `VoiceSelect=SeaScorpionSelect` (5-sample $vscose* pool).
- `VoiceMove=SeaScorpionMove` (5-sample $vscomo* pool).
- `VoiceAttack=SeaScorpionAttackCommand` (5-sample $vscoat* pool).
- `VoiceFeedback=` — empty.
- `MoveSound=SeawolfMoveStart` — **reuses "Seawolf" engine SFX**. Was
  the unit briefly called "Sea Wolf" during development? The
  `[SeawolfMoveStart]` block plays `vseastaa/b/c/d` — 4-sample
  random-predelay underwater engine rumble. Possibly an artifact of
  the rename Hydrofoil → Sea Wolf → Sea Scorpion.
- *No `Report=` keys are defined on either weapon* (FlakTrackGun has
  `Report=FlakTrackAttackGround`, FlakWeapon has
  `Report=FlakCannonAttack` — both inherited from the weapon
  definitions, not the unit).

**Combat behavior**
- `ThreatPosed=25` — same as AEGIS (moderate AI threat). Reflects the
  unit's dual-purpose hazard.
- `DamageParticleSystems=SparkSys,SmallGreySSys` — sparks + smoke.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — 5 abilities.
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — standard.
- *Both* weapons swap to elite versions (FlakTrackGunE + FlakWeaponE).
  Burst=2 on both = double salvo at elite.

**Size**
- `Size=20` — enormous. Cannot fit in any transport.

**Notable absences vs AEGIS**
- *No `RadialFireSegments=`* — Sea Scorpion uses standard turret-style
  facing for its FlakWeapon. Slower AA reaction than Aegis's radial
  launch, but the higher Speed (8) and ROT (6) make up for it
  partially.
- *No `DistributedFire=yes`* — focuses fire on a single target rather
  than round-robin across multiple incoming aircraft. Less efficient
  AA escort than Aegis when facing swarm air attacks.
- *No commented Ammo system* — Sea Scorpion doesn't have the disabled-
  ammo dormant code that Aegis has.

---

## Artmd verbatim

```ini
[HYD] ; Sea Scorpion
Cameo=HOVRICON
Voxel=yes
Remapable=yes
PrimaryFireFLH=65,0,180
SecondaryFireFLH=65,0,180 ;gs needs own listing
```

### Key-by-key annotation

- `Cameo=HOVRICON` — **note the cameo asset name `HOVRICON`** — likely
  inherited from the "Hover"/"Hydrofoil" early development name.
  Sidebar build button.
- `Voxel=yes` — rendered from `hyd.vxl` + `hyd.hva` + turret voxel.
- `Remapable=yes` — house-color remap.
- `PrimaryFireFLH=65,0,180` — FlakTrackGun launch offset:
  - X=65 (moderate forward, turret-mounted).
  - Y=0 (centered).
  - Z=180 (high turret-top).
- `SecondaryFireFLH=65,0,180 ;gs needs own listing` — FlakWeapon launch
  offset. **Verbatim Greg-Smith comment: "needs own listing"** —
  acknowledging that *secondary should have its own FLH* but using same
  values as Primary anyway. Both weapons fire from the same point on
  the model. Effectively redundant, but the explicit second listing
  is required for the engine to use the field at all on the secondary.

**No `AltCameo=`** — single cameo.

---

## Weapons

### Primary — `[FlakTrackGun]` (anti-surface)

```ini
[FlakTrackGun]		; Anti-surface gun
Damage=25 ;25 -changed by DB on 7/18/01
ROF=25 ;40 -changed by DB on 7/18/01
Range=5
Projectile=FlakTProj
Speed=50
Report=FlakTrackAttackGround
Warhead=FlakTWH
Anim=GUNFIRE
```

- `Damage=25` — moderate. (`;25 -changed by DB on 7/18/01` —
  DB=designer name, kept current at 25; original may have been higher).
- `ROF=25` (`;40 -changed`) — faster than original (was 40, lowered to
  25).
- `Range=5` — *short range* — Sea Scorpion must close to engage
  surface naval.
- `Projectile=FlakTProj` — anti-surface flak shell (arcing trajectory).
- `Speed=50` — slow.
- `Report=FlakTrackAttackGround` — distinct ground-attack SFX
  (2-sample `vflaat1*`).
- `Warhead=FlakTWH` — anti-surface flak warhead.
- `Anim=GUNFIRE` — generic muzzle flash.

**Shared with Flak Track ground vehicle (HTK)** — Sea Scorpion's primary
is mechanically identical to the Flak Track's anti-ground weapon. Same
projectile, same warhead, same report. Audio/visual consistency between
the Flak family.

### Secondary — `[FlakWeapon]` (anti-air, shared with Flak Cannon)

```ini
[FlakWeapon]		; This belongs to Flak Cannon
Damage=40
ROF=20
Range=12
Projectile=FlakProj
Speed=100
Report=FlakCannonAttack
Warhead=FlakWH
Anim=GUNFIRE
```

- The verbatim header comment says *"This belongs to Flak Cannon"* —
  the weapon is shared with the Soviet Flak Cannon base defense (NAFLAK
  building).
- `Damage=40` — strong.
- `ROF=20` — fast.
- `Range=12` — **matches AEGIS Medusa range** (longest AA in game).
- `Projectile=FlakProj` — anti-air flak burst (`Inviso=yes`, `AA=yes`,
  `AG=no`).
- `Speed=100` — fast projectile.
- `Report=FlakCannonAttack` — Flak Cannon's fire SFX (shared).
- `Warhead=FlakWH` — anti-air flak warhead.

**Cross-unit weapon sharing**: This `FlakWeapon` is the Flak Cannon's
primary weapon, reused on HYD secondary. **The Sea Scorpion is
mechanically a "naval-mounted Flak Cannon + Flak Track"** — its two
weapons are the AA gun from the static defense and the AG gun from the
mobile flak vehicle.

### Elite weapons — `[FlakTrackGunE]` + `[FlakWeaponE]`

Both add `Burst=2` and minor stat tweaks:

```ini
[FlakTrackGunE]		; Anti-surface gun
Damage=25 (same)
ROF=25 (same)
Range=5 (same)
Burst=2 (NEW)

[FlakWeaponE]		; This belongs to Flak Cannon
Damage=35 (slightly lower than basic 40)
ROF=20 (same)
Range=10 (lower than basic 12)
Burst=2 (NEW)
```

**Elite trade-off on FlakWeaponE**: Damage and Range both *decrease*
slightly at elite (Damage 40→35, Range 12→10), but Burst=2 doubles the
shots. Net DPS:
- Basic: 40 / 20 ROF = 2 dps × 12 range.
- Elite: 35 × 2 / 20 = 3.5 dps × 10 range.
- **75% DPS increase but shorter range**. Interesting trade — elite
  Sea Scorpions need to be closer to AA targets.

### Projectile — `[FlakProj]` (AA shared)

```ini
[FlakProj]		; AA bullet for Flak Cannon and Flak Track.
Image=none
Inviso=yes
AA=yes
AG=no
Shadow=no
Ranged=yes		; Not homing, but ranged -- check fuse, explode if near target coords
Inaccurate=yes	; Bullets do not snap onto targets when "close enough".
FlakScatter=yes ; This weapon scatters its shots.
SubjectToCliffs=no
SubjectToElevation=yes
SubjectToWalls=no
```

- `Image=none` + `Inviso=yes` — invisible projectile.
- `AA=yes`, `AG=no` — anti-air only.
- `Ranged=yes` — *fuse-based*. Verbatim: "Not homing, but ranged --
  check fuse, explode if near target coords". The projectile flies in
  a straight line and detonates when close to target coordinates,
  rather than tracking. Distinguishes flak from missiles.
- `Inaccurate=yes` — **BulletType-scope, Ghidra-verified** at
  `0x0081b0ac → 0x0046c0ef`. **NEW cheat-sheet entry**. Verbatim:
  "Bullets do not snap onto targets when 'close enough'." Flak shells
  don't auto-hit even when target is within close range — they must
  detonate on their own fuse logic. Realistic flak modeling.
- `FlakScatter=yes` — **BulletType-scope, Ghidra-verified** at
  `0x0081b0a0 → 0x0046c105`. **NEW cheat-sheet entry**. Verbatim:
  "This weapon scatters its shots." The flak burst scatters in a
  cone pattern around the target rather than firing a tight beam.
  Combined with `Inaccurate=yes`, flak weapons are *area-denial*
  AA — hits multiple aircraft in a cluster but doesn't reliably
  kill single ones.

### Projectile — `[FlakTProj]` (anti-surface)

```ini
[FlakTProj]		; Anti-surface bullet for Flak Track.
Image=120MM
Arcing=true
Inviso=no
AA=no
AG=yes
Shadow=no
Inaccurate=yes
FlakScatter=yes
SubjectToCliffs=no
SubjectToElevation=yes
SubjectToWalls=yes
```

- `Image=120MM` — *visible* projectile (vs FlakProj which is Inviso).
- `Arcing=true` — arcs over targets.
- `AA=no`, `AG=yes` — anti-ground only.
- Same `Inaccurate=yes` + `FlakScatter=yes` flags — anti-surface flak
  also scatters and is inaccurate.

### Warhead — `[FlakWH]` (AA)

```ini
[FlakWH]	; For anti-air flak weapons.
CellSpread=1.0
PercentAtMax=.1
Verses=150%,80%,50%,100%,100%,20%,0%,0%,0%,100%,100%	; no buildings
AnimList=SMKPUFF
InfDeath=3
```

- `CellSpread=1.0` — *large* AoE radius. Flak hits multiple aircraft
  in a cluster.
- `PercentAtMax=.1` — only 10% damage at AoE edge. Centered hits do
  full damage, edge hits do almost nothing.
- `Verses=150%,80%,50%,100%,100%,20%,0%,0%,0%,100%,100%`:
  | Armor    | Multiplier | vs Damage 40 |
  |----------|-----------|----------------|
  | none     | **150%**  | 60 |
  | flak     | 80%       | 32 |
  | plate    | 50%       | 20 |
  | light    | 100%      | 40 |
  | medium   | 100%      | 40 |
  | heavy    | 20%       | 8 |
  | wood     | 0%        | 0 (no-buildings) |
  | steel    | 0%        | 0 |
  | concrete | 0%        | 0 |
  | special_1 | 100%     | 40 |
  | special_2 | 100%     | 100 |

  **150% vs unarmored infantry** — flak shreds parachuting paratroopers.
  Strong vs light/medium armor (100%), poor vs heavy (20%). 0% vs all
  building armors (consistent with `AA=yes AG=no` projectile — flak
  doesn't damage structures).
- `InfDeath=3` — explosion death.

### Warhead — `[FlakTWH]` (anti-surface)

```ini
[FlakTWH]	; For the Flak Track's anti-surface weapon.
CellSpread=1.0
PercentAtMax=1.0
Verses=150%,125%,100%,60%,10%,10%,30%,20%,10%,100%,100%	; no buildings
AnimList=HTRKPUFF
InfDeath=3
Conventional=yes	; Go splash in the water.
```

- `PercentAtMax=1.0` — *no falloff* (100% damage across the AoE).
- `Verses=150%,125%,100%,60%,10%,10%,30%,20%,10%,100%,100%`:
  Anti-light-armor (150%/125% vs none/flak), but **weak vs heavy
  armor** (10%). Sea Scorpion can't reliably damage Rhinos, Apocs.
- `Conventional=yes` — *with verbatim comment "Go splash in the water"*.
  Conventional damage type lets the projectile properly interact with
  water cells (creates splash anim).
- `AnimList=HTRKPUFF` — Half Track muzzle puff.

---

## Voices / sounds

```ini
[SeaScorpionSelect]
Sounds=$vscosea $vscoseb $vscosec $vscosed $vscosee
Control=random
Volume=85

[SeaScorpionMove]
Sounds=$vscomoa $vscomob $vscomoc $vscomod $vscomoe
Control=random
Volume=85

[SeaScorpionAttackCommand]
Sounds=$vscoata $vscoatb $vscoatc $vscoatd $vscoate
Control=random
Volume=85

[SeawolfMoveStart]
Sounds=vseastaa vseastab vseastac vseastad
Control= random predelay
Delay=0 400
Priority=Low
FShift= -10 10
VShift=15
Volume=40

[FlakTrackAttackGround]
Sounds= vflaat1a vflaat1b
FShift= -5 5
Control= random interrupt
VShift=10
Volume=90

[FlakTrackAttackAir]
Sounds= vflaat2a vflaat2b vflaat2c vflaat2d
FShift= -10 10
Control= random interrupt
Volume=95
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=SeaScorpionSelect` | `[SeaScorpionSelect]` | Click |
| `VoiceMove=SeaScorpionMove` | `[SeaScorpionMove]` | Move order |
| `VoiceAttack=SeaScorpionAttackCommand` | `[SeaScorpionAttackCommand]` | Attack order |
| `Report=FlakTrackAttackGround` (Primary FlakTrackGun) | `[FlakTrackAttackGround]` | Anti-surface fire SFX (2-sample) |
| `Report=FlakCannonAttack` (Secondary FlakWeapon) | shared with Flak Cannon | Anti-air fire SFX |
| `MoveSound=SeawolfMoveStart` | `[SeawolfMoveStart]` | Ignition — *uses "Seawolf" prefix*, indicating development-name history. |
| `DieSound=GenSmallWaterDie` | shared | Small-naval death |

**`Control=random interrupt`** on `[FlakTrackAttackAir]` (4 samples) —
each new fire interrupts the previous sample. Common for fast-firing
weapons.

*The Sea Scorpion has NO `VoiceFeedback=` setting* — completely silent
on damage/proximity events. Unlike SUB/BSUB/DLPH which use a silent
`[SubFear]`/`[DolphinFear]` block, Sea Scorpion just leaves the rules
key empty.

---

## Hardcoded behavior (Ghidra-verified)

### 1. FlakScatter + Inaccurate (BulletType-scope)

Two new BulletType-scope flags this iteration:
- `FlakScatter=yes` — **Ghidra-verified BulletType** at
  `0x0081b0a0 → 0x0046c105`. **NEW cheat-sheet entry**. *Scatters
  shots in a cone* around the target. Hits multiple targets in a
  cluster.
- `Inaccurate=yes` — **Ghidra-verified BulletType** at
  `0x0081b0ac → 0x0046c0ef`. **NEW cheat-sheet entry**. Disables the
  "snap-to-target when close" behavior — bullets must detonate on
  their own fuse logic.

Both used by `FlakProj` and `FlakTProj`. Combined effect: **flak
weapons are area-denial**, dealing modest scattered damage rather
than precise high-damage hits. Realistic anti-air design.

**BulletTypeClass__ReadINI scope reminder**: the 4th distinct ReadINI
function in the cheat-sheet (after Techno/Unit/Weapon/Warhead/Object).
Lives in `0x0046xxxx` range. The `ShrapnelWeapon` and `ShrapnelCount`
on Prism Tank also live here. **BulletType handles projectile-level
behavior**.

### 2. MovementRestrictedTo=Water (re-verified UnitType)

Per cheat-sheet (UnitType `0x00845d64 → 0x00747837` from SMCV doc).
Forces water-only pathfinding. Compared with `MovementZone=Amphibious`
(which allows but doesn't force water-only), MovementRestrictedTo is
a hard constraint.

### 3. Dual-weapon (Primary + Secondary) with elite swap

Both weapons swap at elite rank (ElitePrimary + EliteSecondary).
Versus AEGIS's single-weapon elite swap, Sea Scorpion has *both*
weapons upgrade simultaneously when reaching elite rank. **Double
elite payoff** — though each individual upgrade is smaller than
AEGIS's Medusa→MedusaE (which is a ~6× DPS jump).

### 4. ToProtect=yes AI flag

Same TechnoType field as AEGIS (`0x008438dc → 0x00714be8`). AI
escorts Sea Scorpions with combat units, prioritizes their repair,
retreats them from danger.

### 5. Standard Naval class

Same Submarine locomotor (`2BEA74E1-...`), same Naval=yes flag, same
GenSmallWaterDie classification (small naval, not Sinkable like
AEGIS's GenLargeWaterDie).

---

## TS-legacy filter

- `;Primary=HoverMissile` — commented historical AA missile (Westwood
  considered using IFV missile loadout).
- `;SpeedType=Amphibious ;gs Wha!?!` — Westwood inside joke. Disabled.
- `;MovementZone=Amphibious` — commented amphibious test.
- `;AN=no` on FlakProj — commented anti-naval flag (redundant given AG=no).
- The verbatim `;gs Wha!?!` Greg-Smith comment is a development-time
  reaction to a confusing previous designation.
- No `ImmuneToVeins`, no `Subterranean`. YR-active core mechanism.

---

## Comparison: Sea Scorpion vs Aegis Cruiser (naval AA pair)

| Field | HYD Sea Scorpion (Soviet) | AEGIS Aegis Cruiser (Allied) |
|-------|----------------------------|--------------------------------|
| Strength | **400** | 800 |
| Armor | heavy | **light** |
| Cost | **600** | 1200 |
| Speed | **8** | 4 |
| ROT | 6 | 1 |
| TechLevel | **6** | 7 |
| Prereq | NAYARD,NARADR | GAYARD,RADAR |
| Primary | FlakTrackGun (AG) | Medusa (AA) |
| Secondary | **FlakWeapon (AA)** | none |
| AA Range | 12 | 12 |
| AG Range | 5 | n/a |
| Elite | Both swap (FlakTrackGunE + FlakWeaponE) | Single swap (MedusaE) |
| Elite Burst | 2 (both) | 2 |
| RadialFireSegments | not set | **10** |
| DistributedFire | not set | **yes** |
| Sinkable category | Small (GenSmallWaterDie) | Large (GenLargeWaterDie + SinkingSound) |
| Voice family | SeaScorpion* | Aegis* |
| Cameo | HOVRICON | AGISICON |

**Trade-offs:**
- **HYD Sea Scorpion**: cheap ($600), fast (Speed=8), agile (ROT=6).
  Versatile dual-purpose AA+AG. Half-HP. No advanced AA tactics
  (no RadialFire, no DistributedFire). Mass-produce volume.
- **AEGIS Aegis Cruiser**: premium ($1200), slow (Speed=4), unwieldy
  (ROT=1). AA-only specialist. Double HP. Advanced multi-target
  systems (RadialFire 10 segments + DistributedFire). Quality over
  quantity.

**Anti-Kirov DPS comparison:**
- HYD FlakWeapon: 40 × 1 / 20 = 2 dps × ~100% medium-armor Verses = 2 dps.
- AEGIS Medusa: 100 × 1 / 15 = 6.67 dps × ~100% medium-armor (SAMWH 100% vs medium) = 6.67 dps.
- **AEGIS triples HYD's single-target DPS**, but at 2× the cost — net DPS-per-dollar slightly favors AEGIS.

**Multi-target AA escort:**
- HYD focuses fire one target. 2 Kirov incoming = 1 dies, 1 still bombing.
- AEGIS with DistributedFire = each missile targets different Kirov. 2 Kirov incoming = both damaged simultaneously, more likely to break off attack.

**Asymmetric design intent**: Soviets get cheap mass AA naval, Allies
get specialist quality AA naval. Reflects the broader RA2/YR design
philosophy.

---

## Cross-references

- [AEGIS.md](../allied/AEGIS.md) — Allied counterpart. Naval AA pair
  closed.
- [HTK.md](../soviet/HTK.md) — Soviet Flak Track ground vehicle.
  Shares FlakTrackGun + FlakWeapon mechanics. HYD is mechanically the
  naval-mounted Flak Track + Flak Cannon hybrid.
- [FLAKT.md](../soviet/FLAKT.md) — Soviet Flak Trooper, shares some
  flak weaponry.
- [LCRF.md](../allied/LCRF.md) — previous iteration which discovered
  the HYD index correction.
- [ZEP.md](../soviet/ZEP.md) — Kirov Airship, primary AA target HYD
  is designed to counter.

---

## Coverage audit

- [x] Every rulesmd key annotated (~50 keys).
- [x] Every artmd key annotated (5 keys + Greg-Smith comment).
- [x] Both primary + secondary weapons documented
  (FlakTrackGun, FlakTrackGunE, FlakWeapon, FlakWeaponE).
- [x] Both projectiles documented (FlakProj, FlakTProj).
- [x] Both warheads documented (FlakWH, FlakTWH).
- [x] All voice/sound bindings documented including 4-sample
  FlakTrackAttackAir pool.
- [x] Prerequisites: `NAYARD, NARADR`.
- [x] Owner: 4 Soviet sub-factions.
- [x] Veterancy: extended VeteranAbilities (5 incl. ROF), both weapons
  swap to elite.
- [x] Hardcoded behavior: dual-weapon Primary+Secondary, ToProtect AI
  flag (shared with AEGIS), MovementRestrictedTo=Water, Submarine
  locomotor reuse, FlakScatter + Inaccurate BulletType flags.
- [x] TS-legacy filter (commented historical fields + Greg-Smith joke).
- [x] Comparison table closes the naval AA pair (AEGIS vs HYD).
- [x] At least one Ghidra search performed (FlakScatter, Inaccurate —
  both NEW BulletType-scope entries).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("FlakScatter")` | `0x0081b0a0` (single match) |
| `get_xrefs_to(0x0081b0a0)` | `0x0046c105 → BulletTypeClass__ReadINI` |
| `search_strings("Inaccurate")` | `0x0081b0ac` (single match) |
| `get_xrefs_to(0x0081b0ac)` | `0x0046c0ef → BulletTypeClass__ReadINI` |

**New cheat-sheet entries (2):**
- `FlakScatter` (0x0081b0a0 → 0x0046c105) **BulletType** — scatter
  shots in a cone around target.
- `Inaccurate` (0x0081b0ac → 0x0046c0ef) **BulletType** — disable
  snap-to-target close-enough behavior.

Both add to the growing BulletType-scope cheat-sheet (joining
ShrapnelWeapon from SREF doc). **The BulletType scope is becoming a
recognized 4th INI-read class** alongside Techno/Weapon/Warhead and
the Rules-level readers.

**Re-confirmed:**
- `MovementRestrictedTo` UnitType `0x00845d64 → 0x00747837` (from SMCV).
- `ToProtect` TechnoType `0x008438dc → 0x00714be8` (from AEGIS).

**Naval AA pair closed**: AEGIS ✓ + HYD ✓.

**Open questions:**
- The `MoveSound=SeawolfMoveStart` naming hints at a development-time
  rename history (Hydrofoil → Sea Wolf → Sea Scorpion). Worth checking
  CSF strings for "Sea Wolf" references — possibly cut content.
- `MovementRestrictedTo` interaction with `MovementZone` — both keys
  set on HYD (Water for both). What happens if they disagree (e.g.
  MovementRestrictedTo=Water but MovementZone=Amphibious)? Open
  follow-up for a hypothetical mod scenario.
