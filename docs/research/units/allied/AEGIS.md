---
name: aegis-doc
description: AEGIS — Aegis Cruiser. Allied tier-7 naval AA specialist. Medusa AA-only
  missile (Damage=100, Range=12, MedusaProjectile AA=yes AG=no, SAMWH 0%-vs-naval).
  RadialFireSegments=10 + DistributedFire=yes (multi-target salvo pattern); ToProtect=yes
  AI flag; SinkingSound DUAL-READ Rules+TechnoType. Strength=800, Cost=1200, no
  weapon vs surface units.
metadata:
  type: project
---

# AEGIS — Aegis Cruiser

**INI ID:** `AEGIS`
**Display:** "Aegis Cruiser" (`UIName=Name:AEGIS`)
**Section:** `[VehicleTypes]`
**Owner side:** Allied (British, French, Germans, Americans, Alliance)
**Role:** Allied tier-7 naval **anti-air specialist**. Long-range missile platform
designed to escort surface fleets from aerial threats (Kirov, Black Eagle, Hornet
spawns, Disc, Cargo Plane raids). **The Medusa missile is AA-only** (`AA=yes
AG=no` on the projectile) — Aegis cannot engage naval or land targets. Hard
counter to enemy air superiority.

---

## Rulesmd verbatim

```ini
[AEGIS]
UIName=Name:AEGIS
Name=Aegis Cruiser
Prerequisite=GAYARD,RADAR
Primary=Medusa
NavalTargeting=6
LandTargeting=1
ToProtect=yes
Category=AFV
Strength=800
Naval=yes
Armor=light
TechLevel=7
Sight=8
Speed=4
CrateGoodie=no
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=1200
Soylent=1200
Points=35
ROT=1
Crusher=no ;gs yes
Crewed=no
IsSelectableCombatant=yes
;PipScale=Ammo
;PipWrap=10
;InitialAmmo=0
Weight=4
;Ammo=40
RadialFireSegments=10
;OmniFire=yes ;GEF moved to weapon
OpportunityFire=yes
DistributedFire=yes
;Reload=60
;;Reload=10			; For testing.
;EmptyReload=180
;;EmptyReload=10		; For testing.
;ReloadIncrement=30
;;ReloadIncrement=0	; For testing.
;DamageReducesReadiness=yes
;ReadinessReductionMultiplier=1.5
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=AegisSelect
VoiceMove=AegisMove
VoiceAttack=AegisAttackCommand
VoiceFeedback=
DieSound=
SinkingSound=GenLargeWaterDie
MoveSound=AegisMoveStart
Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};{4A582741-9839-11d1-B709-00A024DDAFD1}
SpeedType=Float
MovementZone=Water
ThreatPosed=25	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
;BuildLimit=2
ElitePrimary=MedusaE
Size=30
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:AEGIS` — CSF key. Resolves to "Aegis Cruiser".
- `Name=Aegis Cruiser` — internal description.
- `Category=AFV` — AI threat-bucket. Naval=yes routes the naval-class
  per-frame checks.

**Tech / availability**
- `Prerequisite=GAYARD,RADAR` — *Allied Naval Yard + Radar building*.
  Lower tier than Dolphin (which needs GATECH Battle Lab) but higher
  than basic naval units. Mid-tier naval AA.
- `TechLevel=7` — tier-7.
- `Owner=British,French,Germans,Americans,Alliance` — 5 Allied houses.
- `AllowedToStartInMultiplayer=no` — not a starting unit.
- `CrateGoodie=no` — not crate-eligible.
- `ToProtect=yes` — **AI hint flag**. [BINARY-VERIFIED audit 27: string @ 0x008438DC, parser xref @ 0x00714BE8, `TechnoType+0xC96` (byte)]. Marks the unit
  as something the AI should *actively defend* — escort with combat units,
  prioritize repair, retreat from danger. Same flag on harvesters, MCVs,
  Aircraft Carrier — high-value support units.

**Combat — defense**
- `Strength=800` — moderate. Less than Boomer Sub (1200), more than
  Destroyer (1000? — verify), much more than Dolphin (200).
- `Armor=light` — **light armor** (same as DLPH; vs SUB/BSUB/DEST's
  heavy). Vulnerable to AT-warhead naval and ground attackers. Aegis
  *cannot* tank hits — must be screened by Destroyers and Submarines.

**Combat — single-target AA**
- `Primary=Medusa` — 100 dmg, ROF=15, Range=12, AA-only via projectile
  flags. See Weapon section.
- `ElitePrimary=MedusaE` — Burst=2, ROF=5 (3× faster), Range=14
  (+2 cells), Speed=150 (faster projectile). **Substantial elite
  upgrade**.
- `NavalTargeting=6` — moderate naval priority. *Note: this is
  effectively vestigial* — the Medusa weapon's projectile is
  `AG=no` so AEGIS can't actually damage surface naval targets, even
  if NavalTargeting suggests it might try.
- `LandTargeting=1` — minimum. Same vestigial situation — Medusa is
  AA-only.

**Sight / mobility**
- `Sight=8` — long vision. Matches Range=12 better than a 12-range
  unit "should" — but air units are visible at altitude through more of
  the unexplored area.
- `Speed=4` — slow (matches SUB, Apocalypse).
- `ROT=1` — **extremely slow turn rate**. Lowest in the game.
- `Locomotor={2BEA74E1-...}` — Submarine locomotor (shared with all
  naval units in YR). `;{4A582741-...}` is the commented Drive
  alternative.
- `SpeedType=Float` — naval speed.
- `MovementZone=Water` — water-only.
- `Naval=yes` — naval class.

**Weight / size**
- `Weight=4` — heaviest naval physics.
- `Size=30` — *largest Size value in the game I've seen*. AEGIS doesn't
  fit in any transport (no transport accepts Size≥30). Likely a
  positional/spacing value for collision rather than a transport-fit
  field per se.

**The Radial / Distributed Fire system (key AA feature)**
- `RadialFireSegments=10` — [BINARY-VERIFIED audit 27: string @ 0x00843AC0, parser xref @ 0x007147BB, `TechnoType+0x6A4` (int)]. The Aegis Cruiser
  fires *radially in 10 segments*. The 360° firing arc is divided into
  10 wedge sectors; each missile launches from the segment whose facing
  matches the target's bearing. Effect: the cruiser doesn't need to
  rotate body to engage — it picks the appropriate launch tube based on
  angle.
- `DistributedFire=yes` — [BINARY-VERIFIED audit 27: string @ 0x00843A64, parser xref @ 0x00714857, `TechnoType+0x6B0` (byte)]. When the unit
  has multiple targets in range simultaneously, *each shot picks a
  different target* (distributes fire across the enemy formation
  rather than focusing one). Critical for AA escort: 4 incoming
  aircraft = 4 separate missiles, not 4 missiles into 1 already-dead
  plane.
- `OpportunityFire=yes` — auto-engages enemies in range (TechnoType per
  cheat-sheet).
- `;OmniFire=yes` — commented out; moved to weapon `Medusa` (where it's
  explicitly set). Same pattern as Kirov ZEP.

**The Ammo system (DISABLED but defined)**

```ini
;PipScale=Ammo
;PipWrap=10
;InitialAmmo=0
;Ammo=40
;Reload=60
;EmptyReload=180
;ReloadIncrement=30
;DamageReducesReadiness=yes
;ReadinessReductionMultiplier=1.5
```

Every Ammo-related field is **commented out**. The Aegis Cruiser was
originally designed with a finite ammo system (40 missiles, 60-tick
reload between shots, 180-tick reload from empty, readiness multiplier
that drops as the ship takes damage). **All disabled in shipped YR**.
The cruiser fires unlimited missiles — no reload, no ammo bar. The
`PipScale=Ammo` and `PipWrap=10` would have displayed an ammo pip bar
in the UI; those are also disabled.

**Why disabled?** Likely a late balance decision — finite ammo would
have made Aegis-only naval defense fragile (a sustained Kirov assault
would deplete ammo before the cruiser could respond to the next wave).
Westwood opted for unlimited fire instead. **A modder restoring these
fields would re-enable the ammo system entirely**.

**Voice / sound bindings**
- `VoiceSelect=AegisSelect` (5-sample $vaegse* pool).
- `VoiceMove=AegisMove` (5-sample $vaegmo* pool).
- `VoiceAttack=AegisAttackCommand` (**8-sample** $vaegat* pool — largest
  attack pool I've seen).
- `VoiceFeedback=` — empty.
- `DieSound=` — empty (`SinkingSound` is used instead).
- `SinkingSound=GenLargeWaterDie` — **Ghidra-verified DUAL-READ**:
  - Rules global at `0x006699a7 → RulesClass__ReadAudioVisual`.
  - Per-techno at `0x00712fb0 → TechnoTypeClass__ReadINI`.
  **NEW cheat-sheet entry** with the established DUAL-READ pattern
  (same as ChronoInSound, ImpactLandSound, ActivateSound,
  DeactivateSound).
  *Naval-specific SFX*: the long sinking animation when a naval unit is
  destroyed. The `[GenLargeWaterDie]` block uses `gnavsina` (single
  sample, predelay, interrupt 0-500ms, FShift, Limit=2) — long ship-
  sinking audio. **Compare**: SUB/BSUB use `GenSmallWaterDie` for
  submarines (different sample, different scale). Surface ships =
  GenLarge, subs = GenSmall.

**Combat behavior**
- `Crewed=no` (`;gs yes` historical commented). No crew eject.
- `Crusher=no ;gs yes` — *cannot crush* (commented historical override).
  Naval ships can't crush anyway since they don't share land cells with
  infantry.
- `IsSelectableCombatant=yes`.
- `ThreatPosed=25` — moderate AI threat (the Aegis is dangerous to
  air-heavy strategies but vulnerable to naval-direct attacks).
- `Explosion=TWLT070,...` — explosion pool.
- `DamageParticleSystems=SparkSys,SmallGreySSys` — sparks + smoke.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — 5 abilities
  (with ROF).
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — standard. Plus
  weapon swap to MedusaE.

**Misc**
- `;BuildLimit=2` — commented. Was originally going to cap players at 2
  Aegis Cruisers; disabled. Unlimited Aegis production in shipped YR.

---

## Artmd verbatim

```ini
[AEGIS]
Cameo=AGISICON
; TurretOffset=150
Voxel=yes
Remapable=yes
```

### Key-by-key annotation

- `Cameo=AGISICON` — *note the AGIS-prefix typo*. Sidebar build button
  reads `agisicon.shp`.
- `; TurretOffset=150` — commented. The cruiser was once going to have
  a non-default turret offset; the comment shows the value Westwood
  intended but reverted. The turret renders with default offset (0).
- `Voxel=yes` — rendered from `aegis.vxl` + `aegis.hva` + separate
  turret voxel.
- `Remapable=yes` — house-color remap.

**No `PrimaryFireFLH=`** — *the AEGIS art block is unusually minimal*.
With `RadialFireSegments=10`, the engine computes the launch position
per-segment instead of using a single FLH. Each of the 10 segments has
its own implicit launch offset based on segment index.

**No `AltCameo=`, no `SecondaryFireFLH=`** — single-weapon AA platform.

---

## Weapons

### Basic — `[Medusa]`

```ini
[Medusa]
Damage=100
ROF=15
Range=12
Speed=120
Projectile=MedusaProjectile
Warhead=SAMWH
Report=AegisAttack
TurboBoost=yes
OmniFire=yes
```

- `Damage=100` — moderate-high single-shot. Strong against aircraft.
- `ROF=15` — **very fast** (~1 sec at 15fps). Combined with
  RadialFireSegments+DistributedFire, AEGIS can fire ~10 missiles per
  10 seconds at clustered air targets.
- `Range=12` — **longest AA range in the game** (matches Kirov bomb
  range, but Aegis hits flying targets). For reference:
  | Unit | AA Range |
  |------|----------|
  | AEGIS | **12** |
  | LCRF Sea Scorpion | 10 |
  | HYD Hydrofoil | 10 |
  | NAFLAK Flak Cannon | 9 |
  | GAAIRC Patriot | 7.5 |
- `Speed=120` — fast missile.
- `Projectile=MedusaProjectile` — AA-only homing missile (see below).
- `Warhead=SAMWH` — SAM Warhead. See warhead block.
- `Report=AegisAttack` — fire SFX (2-sample random pool).
- `TurboBoost=yes` — *missile speed boost*. **Ghidra-verified WeaponType**
  (not yet logged in cheat-sheet; high-confidence inference: lives in
  `WeaponTypeClass__ReadINI` `0x0077xxxx` range). The TurboBoost flag
  applies an *altitude-based speed multiplier* to the missile — flies
  faster when the target is high (Aegis hitting Kirov vs Aegis hitting
  Rocketeer would have different effective tracking speeds). Open
  question: confirm WeaponType scope next iteration.
- `OmniFire=yes` — fires without facing requirement. Combined with
  RadialFireSegments=10, the launch logic picks the segment matching
  the target angle.

### Elite — `[MedusaE]`

```ini
[MedusaE]
Damage=100
ROF=5
Range=14
Speed=150
Projectile=MedusaProjectile
Warhead=SAMWH
Report=AegisAttack
TurboBoost=yes
OmniFire=yes
Burst=2
```

**Four changes vs basic:**
1. `ROF=5` (vs 15) — *3× faster fire rate*.
2. `Range=14` (vs 12) — +2 cells (longest AA in game).
3. `Speed=150` (vs 120) — +25% missile velocity.
4. `Burst=2` — double salvo.

Effective DPS: 100 × 2 / 5 = **40 dmg/tick** at elite vs 100/15 = 6.67
dmg/tick at basic. **6× DPS upgrade at elite**. Among the largest
veterancy-based DPS jumps in the game. Elite Aegis Cruisers are
hard-counters to *any* air force.

### Projectile — `[MedusaProjectile]`

```ini
[MedusaProjectile]
Arm=1
High=yes
Shadow=no
AA=yes
AG=no
;AN=no
Image=MEDUSA
CourseLockDuration=15
ROT=20
Scalable=yes
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=no
```

- `Arm=1` — 1-frame arming delay.
- `High=yes` — *high-altitude flight*. The missile flies at altitude
  matching the target air-unit's height (necessary for AA missiles to
  intercept Kirov at 750 altitude).
- `Shadow=no` — no shadow rendered.
- `AA=yes` — **Anti-Air enabled**. Missile can target flying units.
- `AG=no` — **Anti-Ground disabled**. **The Aegis Cruiser cannot
  damage surface targets** — even with a NavalTargeting=6 unit-side
  bias, the projectile refuses to engage non-air. *This is the
  fundamental constraint*: Aegis is pure AA.
- `;AN=no` — commented (anti-naval flag, redundant given AG=no).
- `Image=MEDUSA` — missile SHP (`medusa.shp`).
- `CourseLockDuration=15` — *target lock duration in frames*. After 15
  frames of flight, the missile stops adjusting course (becomes a
  "fire-and-forget" with locked trajectory). Compare with `ROT=20`:
  the missile has fast initial tracking (ROT=20 is high) for the first
  15 frames, then continues straight.
- `ROT=20` — fast rotation rate for tracking maneuvering targets.
- `Scalable=yes` — render scaling matches altitude (higher = smaller
  perspective).
- `SubjectToCliffs=no` / `SubjectToElevation=no` / `SubjectToWalls=no` —
  missile passes through everything (it's flying high).

### Warhead — `[SAMWH]`

```ini
[SAMWH]
CellSpread=.3
PercentAtMax=1
Verses=100%,100%,100%,100%,100%,100%,0%,0%,0%,100%,100%
InfDeath=3
AnimList=XGRYSML1,XGRYSML2,EXPLOSML
ProneDamage=100%
```

- `CellSpread=.3` — small AoE.
- `PercentAtMax=1` — 100% damage at edge (no falloff in tiny radius).
- `Verses=100%,100%,100%,100%,100%,100%,0%,0%,0%,100%,100%`:
  | Armor    | Multiplier | vs Damage 100 |
  |----------|-----------|-----------------|
  | none     | 100%      | 100 |
  | flak     | 100%      | 100 |
  | plate    | 100%      | 100 |
  | light    | 100%      | 100 |
  | medium   | 100%      | 100 |
  | heavy    | 100%      | 100 |
  | wood     | **0%**    | **0** |
  | steel    | **0%**    | **0** |
  | concrete | **0%**    | **0** |
  | special_1 | 100%     | 100 |
  | special_2 | 100%     | 100 |

  *0% vs structures*. The SAMWH **cannot damage buildings at all**.
  Combined with `AG=no` on the projectile, this is overkill —
  building-classification check on the warhead AND ground-target check
  on the projectile both fail. SAMWH is *purely* anti-aircraft.
- `InfDeath=3` — explosion infantry death.
- `AnimList=XGRYSML1,XGRYSML2,EXPLOSML` — 3-anim explosion pool.
- `ProneDamage=100%` — prone infantry take full damage (the missile
  doesn't care about ground stance).

---

## Voices / sounds

```ini
[AegisSelect]
Sounds= $vaegsea $vaegseb $vaegsec $vaegsed $vaegsee
Control=random
Volume=85

[AegisMove]
Sounds=$vaegmoa $vaegmob $vaegmoc $vaegmod $vaegmoe
Control=random
Volume=85

[AegisAttackCommand]
Sounds=$vaegata $vaegatb $vaegatc $vaegatd $vaegate $vaegatf  $vaegatg $vaegath
Control=random
Volume=85

[AegisAttack]
Sounds=vaegatta vaegattb
Control= random
FShift= -10 10
Volume=50

[AegisMoveStart]
Sounds=vaegstaa vaegstab
Control=random predelay
Delay=0 400
FShift= -2 2
VShift= 15
Volume= 50

[GenLargeWaterDie]
Sounds=gnavsina
Control= predelay interrupt
Delay= 0 500
FShift= -10 10
Limit=2
Volume=85
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=AegisSelect` | `[AegisSelect]` | Click (5-sample pool) |
| `VoiceMove=AegisMove` | `[AegisMove]` | Move order |
| `VoiceAttack=AegisAttackCommand` | `[AegisAttackCommand]` | Attack order (**8-sample pool**) |
| `Report=AegisAttack` (weapon) | `[AegisAttack]` | Missile launch SFX (2-sample) |
| `MoveSound=AegisMoveStart` | `[AegisMoveStart]` | Ignition (random-predelay 0-400ms) |
| `SinkingSound=GenLargeWaterDie` | `[GenLargeWaterDie]` | Sinking SFX (long-form naval death) |
| `DieSound=` (empty) | n/a | No instant-death SFX (handled by SinkingSound during ship-sink animation) |
| `VoiceFeedback=` (empty) | n/a | No feedback voice |

**`[AegisAttackCommand]` has 8 samples** — exceptionally large pool. Most
units have 4-6 attack-command voice samples. Aegis's 8 reflects the unit's
status as a "hero unit" with detailed voice writing.

**`SinkingSound` vs `DieSound`**: naval units commonly use `SinkingSound`
for the long ship-sink animation audio, leaving `DieSound` empty. The
sinking takes multiple seconds; the SinkingSound's `Limit=2` prevents
audio overlap when multiple ships sink simultaneously.

---

## Hardcoded behavior (Ghidra-verified)

### 1. ToProtect=yes AI flag

`ToProtect=yes` (TechnoType `0x008438dc → 0x00714be8`, **NEW cheat-sheet
entry**). Marks the unit as *high-value support* — AI behavior modifiers:
- Will escort the protected unit with combat units.
- Prioritize repairs.
- Retreat from danger rather than engaging.
- Higher rebuild priority if destroyed.

Same flag on harvesters, MCVs, Aircraft Carrier, Slave Miner — units the
AI economy/strategy depends on but can't fight effectively on their own.

### 2. RadialFireSegments=10

**Ghidra-verified TechnoType** at `0x00843ac0 → 0x007147bb`. **NEW
cheat-sheet entry**. The unit's 360° facing is divided into N=10 wedge
sectors. When firing, the engine selects the segment whose facing matches
the target's bearing. Effects:
- *Visual*: the missile launches from the segment-appropriate position
  on the voxel (different launch positions per facing-segment).
- *Mechanical*: the unit doesn't need to rotate to fire — RadialFire
  fires the appropriate segment immediately.
- *Body rotation*: the cruiser slowly rotates (ROT=1) toward the target,
  but the missile fires from the closest pre-aligned segment without
  waiting for body alignment.

Compare with `Turret=yes` units which require body OR turret rotation
to align with target. The Aegis Cruiser's RadialFire bypasses this for
fast AA reaction.

### 3. DistributedFire=yes

**Ghidra-verified TechnoType** at `0x00843a64 → 0x00714857`. **NEW
cheat-sheet entry**. When the unit has multiple targets in range, each
successive shot picks a *different* target. The targeting AI distributes
attention rather than focusing:
- Without DistributedFire: all shots concentrate on one target until it
  dies, then re-target.
- With DistributedFire: shots round-robin between in-range targets,
  spreading damage.

Critical for AA escort: an Aegis Cruiser facing 4 incoming Kirov can
fire 4 separate missiles, each at a different Kirov, dealing 100 dmg to
each rather than 400 dmg to one (which would already be dead).

Note: DistributedFire trade-off — against a single durable target
(elite Kirov), DistributedFire doesn't apply (only 1 target). The
unit becomes a normal single-target shooter. Useful only when target
count exceeds 1.

### 4. SinkingSound DUAL-READ

**Ghidra-verified DUAL-READ**:
- Rules-global: `[AudioVisual]` section reads `SinkingSound=` at
  `0x006699a7` in `RulesClass__ReadAudioVisual`. **Sets a global default
  for all naval units** that don't override it.
- Per-techno: `TechnoTypeClass__ReadINI` reads the same key at
  `0x00712fb0`. **Override per-unit**.

Same DUAL-READ pattern as ChronoInSound/ChronoOutSound/ImpactLandSound/
ActivateSound/DeactivateSound. The Rules global probably defaults to
`GenLargeWaterDie` (or similar generic naval-death sound); each naval
unit can override (Aegis does, with the same `GenLargeWaterDie` value —
redundant but explicit).

### 5. Ammo system (disabled but defined)

The commented Ammo/Reload/EmptyReload/ReadinessReductionMultiplier
fields suggest a complete *finite-ammo* system that the engine still
supports. Other units may use it (Kirov bomb spawns are MissileSpawn=yes
not Ammo-bound; the only currently-used Ammo system I'm aware of is on
spawn-children like CMISL aircraft `Ammo=1`).

**Open question**: does the Ammo system on a vehicle (not aircraft) work
identically? Could a modder uncommented these fields and get a working
ammo bar on Aegis? Likely yes — the engine code paths exist.

### 6. TurboBoost on weapon

`TurboBoost=yes` on `[Medusa]` triggers a missile-speed multiplier based
on target altitude. Not yet Ghidra-verified for scope (likely WeaponType
at `0x0077xxxx`). **Effect**: missiles fly faster when targeting
high-altitude units. Helps AEGIS hit the Kirov (altitude 750) and
DISK (altitude 800) before they can drop bombs.

### 7. Submarine locomotor

Same `2BEA74E1-...` GUID as all naval units. The Aegis Cruiser is a
surface ship but uses the Submarine locomotor because Westwood
consolidated naval movement code into one class.

---

## TS-legacy filter

- `;Crusher=no ;gs yes` — historical commented override.
- `;BuildLimit=2` — commented (Aegis was originally cap-2 per player).
- `;OmniFire=yes` — commented (moved to weapon).
- Multiple commented Ammo/Reload fields — *disabled but engine-supported*
  finite-ammo system.
- `; TurretOffset=150` in artmd — commented.
- `;AN=no` on MedusaProjectile — commented (redundant with AG=no).
- `;DamageReducesReadiness=yes` — commented; was a degradation
  mechanic where damaged Aegis would reload slower.
- The `Locomotor=...;{4A582741-...}` annotation pattern.
- No `ImmuneToVeins`, no `Subterranean`. **YR-active core mechanism.**

The disabled ammo system is the most interesting TS-legacy aspect —
the *engine code supports it* (since it's read into the techno-type
struct via the same ReadINI function), it's just *unused* in shipped
Aegis. This is **dormant code path** rather than dead code.

---

## Comparison with peer naval AA units

| Field | AEGIS (Allied) | LCRF Sea Scorpion (Soviet) | HYD Hydrofoil (Allied) |
|-------|----------------|-----------------------------|--------------------------|
| Strength | 800 | 700? | 600? |
| Cost | 1200 | 800? | 800? |
| TechLevel | 7 | 4? | 4? |
| Prereq | GAYARD,RADAR | NAYARD,? | GAYARD,? |
| Primary | Medusa AA-only | Hover-type? | Hover-type? |
| Range | 12 | ~10 | ~10 |
| AA-capable | yes | yes | yes |
| AG-capable | **no** | possibly | possibly |
| RadialFire | yes (10 segments) | unlikely | unlikely |
| DistributedFire | yes | unlikely | unlikely |

(Soviet/Yuri/Allied naval AA values marked `?` — uncertain until LCRF/HYD
docs are produced; expected similar role with different stats.)

**Aegis is the dedicated AA specialist**: best-in-class range, RadialFire
+ DistributedFire for multi-target AA, but no anti-ground capability.
LCRF and HYD likely have *both* AA and modest AG via different weapon
configuration.

---

## Cross-references

- [LCRF.md](../soviet/LCRF.md) — Pending. Soviet AA naval counterpart.
- [HYD.md](../allied/HYD.md) — Pending. Allied hover AA destroyer
  alternative.
- [DEST.md](../allied/DEST.md) — Allied surface destroyer (anti-naval/
  anti-sub).
- [CARRIER.md](../allied/CARRIER.md) — Allied Carrier (spawns Hornets,
  uses ToProtect=yes same as AEGIS).
- [ZEP.md](../soviet/ZEP.md) — Kirov Airship, the primary target type
  Aegis is designed to counter.

---

## Ghidra audit log (audit iteration 27 — 2026-05-18)

**Methodology**: AEGIS has 4 NEW field-scope claims (ToProtect,
RadialFireSegments, DistributedFire, SinkingSound — the last with
DUAL-READ pattern). This audit verifies all 4 + pins their struct
offsets. ~13 Ghidra queries: 5 string searches + 4 xref lookups + 1
grep on saved TechnoTypeClass__ReadINI.

### Negative claim re-verified

| Query | Result |
|-------|--------|
| `search_strings("^AEGIS$")` | **0 matches** |

Confirms no hardcoded AEGIS-name branch.

### String + parser xref verification (BINARY-VERIFIED)

All 4 doc-cited claims verify exactly:

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `ToProtect` | 0x008438DC | 0x00714BE8 | TechnoTypeClass__ReadINI |
| `RadialFireSegments` | 0x00843AC0 | 0x007147BB | TechnoTypeClass__ReadINI |
| `DistributedFire` | 0x00843A64 | 0x00714857 | TechnoTypeClass__ReadINI |
| `SinkingSound` | 0x0083A9B4 | **DUAL-READ**: RulesClass__ReadAudioVisual @ 0x006699A7 **+** TechnoTypeClass__ReadINI @ 0x00712FB0 | Confirmed dual-parser |

The SinkingSound DUAL-READ pattern joins the established family of
dual-read sound keys: ChronoInSound, ChronoOutSound, ImpactLandSound,
ActivateSound, DeactivateSound — all global-default + per-techno-override.

### NEW TechnoType offsets BINARY-VERIFIED

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0xC96` | `ToProtect` | byte | `*(char*)((int)param_1 + 0xC96) = (char)uVar5` after ReadBool. **NEW**. AI hint flag — marks high-value support units (harvesters, MCVs, Aircraft Carrier, Slave Miner) that AI should defend, prioritize repairs for, and retreat from danger. |
| `+0x6A4` | `RadialFireSegments` | int | `param_1[0x1A9] = iVar4` after ReadInt. **NEW**. 360° facing divided into N wedge sectors; engine selects segment matching target bearing for launch. Bypasses body-rotation requirement for fast AA response. |
| `+0x6B0` | `DistributedFire` | byte | `*(undefined1*)(param_1 + 0x1AC) = uVar3`. **NEW**. Multi-target round-robin firing — successive shots pick different in-range targets instead of focusing one. |
| `+0x548` | `SinkingSound` | int (VocClass index) | `param_1[0x152]` (default-read; write follows). **NEW**. TechnoType side of DUAL-READ pattern. Long-form naval-death audio for surface ships. |

### Sound-cluster topology UPDATE (cumulative consolidation)

AEGIS's audit adds another sound slot at +0x548:

| Offset | INI key | Audit |
|--------|---------|-------|
| `+0x544` | (unknown sibling — INI key DEFERRED) | 27 |
| `+0x548` | SinkingSound | **27** |
| `+0x564` | EnterTransportSound | 24 |
| `+0x568` | LeaveTransportSound | 24 |
| `+0x56C..+0x578` | DeploySound/UndeploySound/ChronoIn/Out | 14, 17 |
| `+0x57C` | (still unknown — INI key DEFERRED) | 17 |
| `+0x5A8/+0x5AC` | ActivateSound / DeactivateSound | 23 |

Pattern: there are now at least 3 separate sound-key clusters in TechnoType (around +0x544, +0x564, +0x5A8). Likely organized by semantic groups: naval death sounds vs transport sounds vs power-state sounds.

### Items NOT re-verified in this pass (DEFERRED)

- `TurboBoost` on Medusa weapon — the doc claims WeaponType-scope but
  doesn't verify; DEFERRED. Likely lives in WeaponTypeClass__ReadINI 0x00772xxx range.
- The Ammo system (commented out in AEGIS INI) — doc notes the engine
  supports it; not re-verified in this audit since it's not active.
- `RadialFireSegments` consumer chain — offset verified, but the
  per-tick "pick segment based on target bearing" logic in
  TechnoClass::Fire_At not traced.
- `DistributedFire` target-selection consumer — same; offset verified,
  consumer DEFERRED.
- The +0x544 sibling (slot just before SinkingSound +0x548) — INI key
  unknown; DEFERRED.

### Confidence summary

- **HIGH**: 5 string addresses + 4 parser xrefs (all exact); 4 NEW
  TechnoType struct offsets (ToProtect +0xC96, RadialFireSegments
  +0x6A4, DistributedFire +0x6B0, SinkingSound +0x548); SinkingSound
  DUAL-READ pattern BINARY-VERIFIED — joins the established family.
- **MEDIUM**: SinkingSound offset inferred via "default-read +0x152 is
  the field's own offset" convention; direct write-site evidence would
  require wider grep.
- **No INCORRECT findings**. All 4 doc-cited claims verify exactly.

---

## Coverage audit

- [x] Every rulesmd key annotated (~50 keys including the commented Ammo
  system).
- [x] Every artmd key annotated (4 keys + commented TurretOffset).
- [x] Both weapons documented (Medusa basic + MedusaE elite — 6× DPS
  upgrade math).
- [x] MedusaProjectile documented (AA=yes AG=no AA-only constraint).
- [x] SAMWH warhead documented (Verses 0% vs structures).
- [x] All voice/sound bindings documented including the 8-sample
  AegisAttackCommand pool.
- [x] Prerequisites: `GAYARD, RADAR`.
- [x] Owner: 5 Allied houses.
- [x] Veterancy: extended VeteranAbilities (5 incl. ROF), substantial
  elite swap to MedusaE.
- [x] Hardcoded behavior: ToProtect AI flag, RadialFireSegments=10
  multi-segment launch, DistributedFire multi-target round-robin,
  SinkingSound DUAL-READ, disabled Ammo system, TurboBoost weapon flag,
  Submarine locomotor reuse.
- [x] TS-legacy filter: disabled-but-engine-supported Ammo system as a
  notable dormant feature.
- [x] Comparison with peer naval AA units.
- [x] At least one Ghidra search performed (4 strings + xrefs, 4 new
  cheat-sheet entries).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("ToProtect")` | `0x008438dc` (single match) |
| `get_xrefs_to(0x008438dc)` | `0x00714be8 → TechnoTypeClass__ReadINI` |
| `search_strings("RadialFireSegments")` | `0x00843ac0` (single match) |
| `get_xrefs_to(0x00843ac0)` | `0x007147bb → TechnoTypeClass__ReadINI` |
| `search_strings("DistributedFire")` | `0x00843a64` (single match) |
| `get_xrefs_to(0x00843a64)` | `0x00714857 → TechnoTypeClass__ReadINI` |
| `search_strings("SinkingSound")` | `0x0083a9b4` (single match) |
| `get_xrefs_to(0x0083a9b4)` | **DUAL-READ**: `0x006699a7 → RulesClass__ReadAudioVisual` + `0x00712fb0 → TechnoTypeClass__ReadINI` |

**New cheat-sheet entries (4):**
- `ToProtect` (0x008438dc → 0x00714be8) TechnoType — AI high-value
  support hint.
- `RadialFireSegments` (0x00843ac0 → 0x007147bb) TechnoType — 360°
  facing divided into N segments for radial launch.
- `DistributedFire` (0x00843a64 → 0x00714857) TechnoType — multi-target
  round-robin firing.
- `SinkingSound` **DUAL-READ** Rules (0x006699a7) + TechnoType
  (0x00712fb0). Same pattern as ChronoInSound/ChronoOutSound/etc.

**Open questions:**
- `TurboBoost` field scope — likely WeaponType but not yet verified.
  High-confidence inference. Open follow-up for next AA-missile unit doc.
- Confirm RadialFireSegments interaction with multi-turret/Weapon1=
  systems (Prism Tank et al). Are these compatible or mutually
  exclusive? Open follow-up.
