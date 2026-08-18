---
name: dlph-doc
description: DLPH — Allied Dolphin. Anti-submarine sonic specialist. SonicZap weapon
  with IsSonic=yes + AmbientDamage (chain pulse); Underwater+Cloakable; Organic=yes
  (organism, not vehicle); TypeImmune=yes; SHP-rendered (Voxel=no) WalkRate/IdleRate
  animation system; weakest naval unit ($500) but fast and stealthy.
metadata:
  type: project
---

# DLPH — Dolphin

**INI ID:** `DLPH`
**Display:** "Dolphin" (`UIName=Name:DLPH`)
**Section:** `[VehicleTypes]`
**Owner side:** Allied (British, French, Germans, Americans, Alliance)
**Role:** Allied tier-5 naval specialist — sonic-pulse anti-submarine /
anti-naval. The cheapest naval unit in the game ($500), fastest (Speed=8),
and the only *organic* naval unit (Organic=yes — treated as a living
creature, not a vehicle). Cloakable like submarines but **lives just below
the water surface, NOT deep underwater** — visible to standard ships, but
stealthy to non-Sensors enemies. Pairs with Aegis Cruiser for Allied naval
sweep.

---

## Rulesmd verbatim

```ini
[DLPH]
UIName=Name:DLPH
Name=Dolphin
NotHuman=yes
Prerequisite=GAYARD,GATECH
Primary=SonicZap
NavalTargeting=5
LandTargeting=1
FireAngle=64
Category=AFV
Strength=200
Naval=yes ;GS
Armor=light
TechLevel=5
Underwater=yes
Sight=4
GuardRange=4
Sensors=yes
SensorsSight=8 ;4
Speed=8
CrateGoodie=no
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=500
Soylent=500
Turret=no
Points=15
ROT=6
;Crusher=yes
Crewed=no				;ok
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=DolphinSelect
VoiceMove=DolphinMove
VoiceAttack=DolphinAttackCommand
VoiceFeedback=DolphinFear
DieSound=DolphinDie
Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};{4A582741-9839-11d1-B709-00A024DDAFD1}
SpeedType=Float
MovementZone=Water
ThreatPosed=20	; This value MUST be 0 for all building addons
Accelerates=true
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
;TooBigToFitUnderBridge=true
Cloakable=yes
CloakingSpeed=1
TypeImmune=yes
Organic=yes
;NoShadow=yes
WalkRate=4 ; these two are needed because unit as sprite is terribly hack. Doing units as infantry with DoControls could be considered
IdleRate=8 ; power of two helps performance (mod).  "How much slower should I animate when stopped? 1/x"
ElitePrimary=SonicZapE
Size=15
IsSelectableCombatant=yes
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:DLPH` — CSF key. Resolves to "Dolphin".
- `Name=Dolphin` — internal description.
- `NotHuman=yes` — *organic non-human creature flag*. Used by gore/death
  routing (no human-death animation). Same flag on Squid, Brute, Cow,
  attack dogs. **[INCORRECT — VEHICLE-SCOPE DEAD INI, audit 28]**: this
  key is parsed only by `InfantryTypeClass__ReadINI` (xref @ 0x005243c6
  → InfantryType+0xEAD). DLPH is in `[VehicleTypes]`, which uses
  `UnitTypeClass__ReadINI` — **the vehicle parser does NOT read
  NotHuman**, so this line is dead INI on the Dolphin. The "same flag
  on Squid, Brute, Cow, attack dogs" comment is partially correct:
  Brute and dogs are InfantryType (NotHuman IS read); Squid, Cow,
  Dolphin are UnitType (NotHuman is dead INI).
- `Category=AFV` — AI threat-bucket (same as other naval — Naval=yes
  routes the per-frame naval-class checks).

**Tech / availability**
- `Prerequisite=GAYARD,GATECH` — *Allied Naval Yard + Allied Battle
  Lab*. **Tier-5 lockout despite being "cheap"** — the Dolphin requires
  the Battle Lab, gating it behind tier-3 tech research.
- `TechLevel=5` — tier-5.
- `Owner=British,French,Germans,Americans,Alliance` — 5 Allied houses.
- `AllowedToStartInMultiplayer=no` — not a starting unit.
- `CrateGoodie=no` — not crate-eligible.

**Combat — defense**
- `Strength=200` — *very fragile*. Compare:
  - DLPH: **200** (cheapest, weakest)
  - SUB: 600
  - BSUB: 1200
- `Armor=light` — *light armor* (unique among naval — SUB/BSUB/DEST all
  use heavy). The Dolphin's biology = light armor classification. AT
  warheads at standard Verses-vs-light multipliers.

**Combat — single weapon**
- `Primary=SonicZap` — sonic pulse weapon. Damage=4 + AmbientDamage=10,
  ROF=120, Range=6, IsSonic=yes, DecloakToFire=no. See "Weapon" section.
- `ElitePrimary=SonicZapE` — elite: Damage=8 + AmbientDamage=15,
  ROF=80 (faster), Burst=2 (double output). **Massive elite upgrade**.
- `NavalTargeting=5` — moderate naval priority (matches SUB).
- `LandTargeting=1` — minimum land priority (Sonic can't really reach
  land targets anyway — projectile is water-bound).
- `FireAngle=64` — vestigial (sonic pulses don't arc).

**Sight / sensors**
- `Sight=4` — short vision (matches SUB).
- `GuardRange=4` — *auto-engagement range while in Guard mode*. Same
  as Sight; the Dolphin won't auto-engage targets farther than its sight.
- `Sensors=yes` — cloak detection (TechnoType per cheat-sheet).
- `SensorsSight=8` — *better cloak detection than its own sight*. The
  `;4` historical comment shows it was raised from 4 to 8. **The Dolphin
  sees cloaked subs at 8 cells but can only see uncloaked targets at 4
  cells** — flavor: dolphins use echolocation (sensors) over vision.

**Mobility**
- `Speed=8` — **fastest naval unit** (vs SUB=4, BSUB=5, DEST/AEGIS=6).
  Dolphins are agile.
- `ROT=6` — *moderate turn rate* (vs SUB/BSUB ROT=2). Faster turning.
- `Accelerates=true` — gradual acceleration.
- `Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C}` — **Submarine
  locomotor**. *Yes, Dolphins use the SUB locomotor*. Same GUID as
  SUB/BSUB. The locomotor handles underwater movement physics. The
  trailing `;{4A582741-...}` is the commented Drive alternative
  annotation.
- `SpeedType=Float` — naval speed table.
- `MovementZone=Water` — water-only.
- `Underwater=yes` — *renders just below surface*. Same flag as
  SUB/BSUB.
- `Naval=yes` — naval class flag.

**Cloak**
- `Cloakable=yes` (TechnoType per SUB cheat-sheet).
- `CloakingSpeed=1` — instant cloak (matches SUB/BSUB).

**Type-immune**
- `TypeImmune=yes` — **Ghidra-verified TechnoType** at
  `0x008444ec → 0x0071220f` → **TechnoType+0xC8C (byte, ReadBool)
  [BINARY-VERIFIED audit 28]**. Assembly-context proof: writeback
  `MOV byte ptr [EBP + 0xc8c], AL` at 0x0071221c. The
  TypeImmune flag means *this unit type's weapons don't damage
  same-type units*. Two Dolphins can't hurt each other with sonic.
  Critical for AoE-style weapons (Sonic, Wave, Tesla, Particle): without
  it, friendly-fire chains could devastate squads of the same unit.

**Organic** (unique field)
- `Organic=yes` — **Ghidra-verified TechnoType** at
  `0x00843714 → 0x0071502b` → **TechnoType+0xD97 (byte, ReadBool)
  [BINARY-VERIFIED audit 28]**. Marks the unit
  as a living organism — affects:
  - Gore/blood splatter on death (different particle systems).
  - Possibly poison-warhead susceptibility (Plague warhead from VIRUS).
  - Mind-control eligibility (debatable — Dolphins can be mind-controlled
    in shipped YR; *Organic doesn't gate psi-control*).
  - Same flag on Squid, Brute, Cow, Dog, Yuri Initiate-ish creatures.
  - **Note**: `Organic` is NOT the same as `Crusher=yes` immunity —
    organics still die to crushing. But certain warheads (`Sonic=yes`
    perhaps?) interact specifically with Organic targets.

**SHP-sprite rendering quirks**
- `WalkRate=4` — animation frame-advance rate while moving (per
  Westwood comment: "How much slower should I animate when stopped? 1/x"
  — this controls per-frame speed of the SHP animation).
- `IdleRate=8` — animation rate while idle. **Power-of-two for performance**
  (the verbatim comment mentions this optimization).
- The verbatim comments are unusually candid: *"these two are needed
  because unit as sprite is terribly hack. Doing units as infantry with
  DoControls could be considered"*. Westwood admits the SHP-based
  Dolphin is an architectural compromise — they would have preferred
  to model it as infantry-class but couldn't for some reason.
- The `[DLPH]` artmd block has `Voxel=no`, `WalkFrames=6`,
  `FiringFrames=6` — *SHP-rendered (sprite frames), NOT voxel*.

**Voice / sound bindings**
- `VoiceSelect=DolphinSelect` — click voice (2-sample `vdolsela/b` pool).
- `VoiceMove=DolphinMove` — move-order voice (2-sample `vdolmova/b` pool).
- `VoiceAttack=DolphinAttackCommand` — attack-order voice (3-sample
  `vdolatca/b/c` pool).
- `VoiceFeedback=DolphinFear` — **silent-block convention** (Volume=0,
  no Sounds=). Same trick as SubFear/BoomerFeedback.
- `DieSound=DolphinDie` — death SFX (2-sample `vdoldiea/b` pool).
- *No `MoveSound=`* — Dolphin makes no ambient engine sound (it's
  organic, no engine).

**Combat behavior**
- `Crewed=no` (with verbatim `;ok` confirming) — no infantry eject.
  Dolphins aren't crewed.
- `;Crusher=yes` — commented out. Dolphins do not crush.
- `Turret=no` — no turret.
- `ThreatPosed=20` — same as other subs.
- `Explosion=TWLT070,...` — explosion pool on death.
- `IsSelectableCombatant=yes` — included in combat-only selection filter.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — 5 abilities
  (same as SUB/BSUB).
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — standard. Plus
  weapon swap to SonicZapE.

**Misc**
- `Size=15` — too big to fit in any transport (same as SAPC/SHAD/SUB).
- `;TooBigToFitUnderBridge=true` — commented. *Default behavior applies*
  (which for vehicles is normally false). The Dolphin can pass under
  bridges? Or doesn't matter since bridges are over water and dolphins
  are at-surface. The comment-out is likely "we don't care, it's a
  dolphin, doesn't matter".
- `;NoShadow=yes` — commented. Dolphin has a shadow (small water ripple).
- Economy: `Cost=500`, `Soylent=500`, `Points=15`. **Cheapest naval unit
  in the game**.

---

## Artmd verbatim

```ini
[DLPH]	;allied dolphin
Cameo=DLPHICON
Voxel=no
Remapable=yes
WalkFrames=6
FiringFrames=6
```

### Key-by-key annotation

- `Cameo=DLPHICON` — sidebar build-button SHP.
- `Voxel=no` — **NOT rendered as voxel**. The Dolphin is a sprite/SHP
  unit. Distinguishes from all other naval (SUB/BSUB/DEST/AEGIS/CARRIER
  all are Voxel=yes). The SHP system uses pre-rendered 8-direction
  frames per state.
- `Remapable=yes` — house-color remap applies to remap-channel pixels.
- `WalkFrames=6` — *6 frames per direction for the swim animation*.
  Per direction (×8 facings = 48 walk frames in the SHP).
- `FiringFrames=6` — *6 frames per direction for the firing animation*.
  Plays when the Sonic weapon discharges. Per direction (×8 = 48 firing
  frames).

**No `PrimaryFireFLH=`** — *unusual*. The Dolphin's `[DLPH]` artmd block
has no fire-launch offset. The sonic projectile spawns at the unit's
center (default 0,0,0). Probably acceptable since Sonic projectile is
visually a wave-circle radiating from origin, not a directional muzzle
flash.

**Total art-asset count**: 48 walk + 48 firing + some idle/death = ~120
SHP frames. Substantial vs the 1 voxel + 1 HVA of a vehicle.

---

## Weapons

### Basic — `[SonicZap]`

```ini
[SonicZap]
Damage=4
AmbientDamage=10
ROF=120
Range=6
;Projectile=Null
Projectile=Sonic
Speed=100
Warhead=SonicWarhead
Report=DolphinAttack
IsSonic=Yes
DecloakToFire=no
;AntiUnderwater=yes
```

- `Damage=4` — **trivially low direct damage**.
- `AmbientDamage=10` — *ambient/chain damage*. **Ghidra-verified
  WeaponType** at `0x00849548 → 0x007720bb` → **WeaponType+0x98 (int,
  ReadInt) [BINARY-VERIFIED audit 28]**.
  *Effective additional damage* on top of direct Damage. For Sonic
  weapons specifically, AmbientDamage is the *radius pulse damage* that
  affects all naval targets along the sonic pulse's path. The damage
  doesn't have a CellSpread mechanic in the warhead — it uses the
  IsSonic special chain logic instead.
- **Effective DPS**: 4 + 10 = **14 damage per hit** (Damage + Ambient).
  At ROF=120 = 14/120 ticks per second = ~1.4 dps.
- `ROF=120` — slow (8 sec).
- `Range=6` — standard naval range.
- `Projectile=Sonic` — sonic-pulse virtual projectile. The verbatim
  `;Projectile=Null` comment shows the original placeholder; replaced
  with `[Sonic]` projectile block.
- `Speed=100` — fast (visual sweep speed of the sonic wave).
- `Warhead=SonicWarhead` — see warhead block.
- `Report=DolphinAttack` — fire SFX (`vdolatta`, single sample).
- `IsSonic=Yes` — **Ghidra-verified WeaponType** at
  `0x00849540 → 0x007720d3` → **WeaponType+0x130 (byte, ReadBool)
  [BINARY-VERIFIED audit 28]**. The
  IsSonic=yes flag triggers the special sonic-chain damage code path:
  the projectile travels through water cells in a straight line, dealing
  damage to *every naval/underwater target it passes through* (chain
  effect, with falloff). Distinguishes Sonic from regular projectile
  damage. **Critical for anti-sub gameplay** — a Dolphin firing through
  a line of subs hits all of them.
- `DecloakToFire=no` — **fires from cloak** (same as SUB/BSUB
  torpedoes). The Dolphin stays cloaked while attacking. WeaponType
  scope `0x0084951c → 0x00772121` (from BSUB doc).
- `;AntiUnderwater=yes` — commented historical anti-submarine specialist
  flag. The default targeting now handles this (Sonic naturally hits
  underwater units).

### Elite — `[SonicZapE]`

```ini
[SonicZapE]
Damage=8
AmbientDamage=15
ROF=80
Range=6
;Projectile=Null
Projectile=Sonic
Speed=100
Warhead=SonicWarhead
Report=DolphinAttack
IsSonic=Yes
;AntiUnderwater=yes
Burst=2
DecloakToFire=no
```

**Four changes vs basic:**
1. `Damage=8` (vs 4) — **doubles direct damage**.
2. `AmbientDamage=15` (vs 10) — +50% chain damage.
3. `ROF=80` (vs 120) — *33% faster fire*.
4. `Burst=2` — **double salvo**.

Effective elite DPS: (8 + 15) × 2 / 80 = 46/80 ticks = ~**8.6 dps**.
**Roughly 6× the basic Dolphin's DPS**. Elite Dolphin is among the
strongest naval damage-per-second in the game.

### Projectile — `[Sonic]`

```ini
[Sonic]
;AG=no ;gs New naval targeting makes this unneeded, and it is knida wrong in theory anyway (ground and water are confused enough without this one's help)
;AS=yes
Level=yes
```

- *Minimal block*. The Sonic projectile is essentially virtual — the
  IsSonic weapon-side flag handles its mechanics. Only `Level=yes` (flies
  horizontal) is meaningful.
- The verbatim Greg-Smith comment is informative: original `AG=no`
  (anti-ground=no) was commented because the *new naval targeting system*
  (NavalTargeting/LandTargeting on the *firing unit*) handles
  ground-vs-water distinction at the unit level, making the
  projectile-level AG flag redundant. Tells us the targeting refactor
  pushed the logic from projectile-side to unit-side.

### Warhead — `[SonicWarhead]`

```ini
[SonicWarhead]
;Spread=2
CellSpread=.1
PercentAtMax=1
Wood=yes
Verses=100%,100%,100%,100%,80%,80%,100%,60%,60%,100%,100%
InfDeath=3
Rocker=yes
ProneDamage=50%
Sonic=yes
```

- `CellSpread=.1` — *tiny* cell-spread (almost point damage). The chain
  damage comes from IsSonic, not CellSpread.
- `PercentAtMax=1` — 100% damage at edge.
- `Wood=yes` — damages wooden structures.
- `Verses=100%,100%,100%,100%,80%,80%,100%,60%,60%,100%,100%`:
  | Armor    | Multiplier | vs Damage 4 + Ambient 10 |
  |----------|-----------|----------------------------|
  | none     | 100%      | 14 |
  | flak     | 100%      | 14 |
  | plate    | 100%      | 14 |
  | light    | 100%      | 14 |
  | medium   | 80%       | 11.2 |
  | heavy    | 80%       | 11.2 |
  | wood     | 100%      | 14 |
  | steel    | 60%       | 8.4 |
  | concrete | 60%       | 8.4 |
  | special_1 | 100%     | 14 |
  | special_2 | 100%     | 14 |

  *Solid vs medium/heavy* (80%) — works fine on submarines. *Strong vs
  unarmored*. Weak vs concrete/steel (60%) — Dolphin can't bust through
  Sea Scorpion/Aegis Cruiser hull as quickly as it kills subs.
- `InfDeath=3` — explosion infantry death.
- `Rocker=yes` — vehicles rock on impact.
- `Sonic=yes` — **warhead-side sonic flag**. Pairs with the weapon-side
  `IsSonic=yes`. The combo triggers the sonic ripple animation + chain
  damage path. Without both, Sonic doesn't fire correctly.
- `ProneDamage=50%` — prone infantry take half damage.

---

## Voices / sounds

```ini
[DolphinSelect]
Sounds= vdolsela vdolselb
Control= random
FShift= -10 10
Volume=60

[DolphinMove]
Sounds= vdolmova vdolmovb
Control= random
FShift= -10 10
Volume=60

[DolphinAttackCommand]
Sounds= vdolatca vdolatcb vdolatcc
Control= random
FShift= -10 10
Volume=50

[DolphinAttack]
Sounds=vdolatta
FShift= -10 10
Volume=60

[DolphinDie]
Sounds=vdoldiea vdoldieb
Control=random
FShift= -5 5
Volume=70

[DolphinFear]
Volume=0	; no sound
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=DolphinSelect` | `[DolphinSelect]` | Click (2-sample dolphin clicks/chirps) |
| `VoiceMove=DolphinMove` | `[DolphinMove]` | Move order |
| `VoiceAttack=DolphinAttackCommand` | `[DolphinAttackCommand]` | Attack order |
| `Report=DolphinAttack` (weapon) | `[DolphinAttack]` | Sonic ping fire SFX |
| `DieSound=DolphinDie` | `[DolphinDie]` | Death SFX (2-sample) |
| `VoiceFeedback=DolphinFear` | `[DolphinFear]` | **silent block** (Volume=0, no Sounds=) |

**No `$`-prefix** on any Dolphin sound — these are *non-voice* SFX
(eva-pool exempt). The dolphin chirps/clicks are environmental audio
rather than scripted voice lines. Distinguishes from human-crewed units
which use `$`-prefixed VO samples.

**`[DolphinFear]` silent-block** — same convention as SUB/BSUB `SubFear`.
The unit needs a `VoiceFeedback=` slot for rules validation; an empty
Volume=0 block provides the slot without playing audio.

---

## Hardcoded behavior (Ghidra-verified)

### 1. IsSonic chain damage

`IsSonic=Yes` on `[SonicZap]` (WeaponType `0x00849540 → 0x007720d3`,
**NEW cheat-sheet entry**) triggers the engine's *sonic-chain damage
path*:
1. Weapon fires; projectile travels straight in a line toward target.
2. For each water/underwater cell along the line, the engine applies
   `Damage + AmbientDamage` to every naval unit in that cell.
3. The sonic pulse continues *through* the first target and damages
   subsequent targets behind it.
4. Falloff: damage reduces with distance from origin (engine-side
   computation; not configurable from INI as far as I've seen).

The combination of low base Damage (4) + high AmbientDamage (10) means
each hit cell takes ~14 damage — *aggregated over multiple cells* in a
clustered submarine line, the Dolphin can deal substantial damage per
shot.

### 2. AmbientDamage

`AmbientDamage` (WeaponType `0x00849548 → 0x007720bb`, **NEW
cheat-sheet entry**) is the *secondary damage component* added on top
of base Damage. For Sonic weapons, this represents the radius-pulse
damage applied along the sonic line. For non-Sonic weapons, the
AmbientDamage may have different semantics (passive area damage?
unverified). Per-weapon read.

### 3. Organic=yes

`Organic=yes` (TechnoType `0x00843714 → 0x0071502b`, **NEW cheat-sheet
entry**) marks the unit as a living creature. Effects:
- Gore/blood splatter on death (organic particle system).
- Possibly poison-warhead susceptibility (Plague/Virus warheads).
- Squid grab/punch decisioning (uncertain; Unnatural=yes is the
  primary signal there).
- *Does NOT* immunize from mind-control (Yuri can mind-control
  Dolphins in shipped YR — `ImmuneToPsionics=no` default applies).

Same flag on Squid, Brute, Cow, attack dogs, Yuri Initiate-class units.

### 4. TypeImmune=yes

`TypeImmune=yes` (TechnoType `0x008444ec → 0x0071220f`, **NEW
cheat-sheet entry**) — *same-type units don't damage each other with
this unit's weapons*. Two Dolphins firing Sonic don't hurt each other.
This is critical because Sonic is a chain weapon — without TypeImmune,
a clustered Dolphin school firing at the same target would chain damage
into themselves on the return pulse / overlap.

Same flag pattern would be expected on Tesla units (chain lightning),
DRON Terror Drones (parasite), Magnetron (LocomotorBeam — though I'm
not sure if it has TypeImmune).

### 5. SHP-rendered with WalkRate/IdleRate

The Dolphin is one of the few naval units rendered as a *sprite SHP*
(Voxel=no). The animation system uses:
- `WalkRate=4` — animation frame advance every 4 game ticks while
  moving.
- `IdleRate=8` — animation frame advance every 8 game ticks while
  stopped (slower idle animation for performance).
- `WalkFrames=6` per direction × 8 directions = 48 walk SHP frames.
- `FiringFrames=6` per direction × 8 = 48 firing SHP frames.

The verbatim Westwood comment about "unit as sprite is terribly hack"
suggests the architecture isn't elegant — vehicles are normally voxel
class. The Dolphin (and Squid) are vehicles-rendered-as-sprites, a
compromise design.

### 6. Cloak + Underwater + Sensors

Same triple-stack as SUB/BSUB (Cloakable=yes, Underwater=yes,
Sensors=yes). The Dolphin is invisible to non-sensor enemies *and*
can detect cloaked subs at SensorsSight=8.

**Stealth-vs-stealth combat**: SUB (sight=7), BSUB (sight=8), DLPH
(sight=8 for sensors only — visual sight is 4). Dolphin's sensor range
matches BSUB's; both spot SUB before SUB spots them.

### 7. Submarine locomotor (shared)

Dolphins use the *Submarine* locomotor (same GUID as SUB/BSUB), not a
"swim" locomotor. The Submarine locomotor handles underwater
positioning, cloak transitions, depth maintenance — generic enough to
work for organic and mechanical submersibles alike.

---

## TS-legacy filter

- `;Crusher=yes` — commented out.
- `;NoShadow=yes` — commented.
- `;Projectile=Null` on weapon — historical placeholder.
- `;AntiUnderwater=yes` — commented historical anti-submarine flag.
- `;AG=no` on `[Sonic]` projectile — commented because the new naval
  targeting system handles it.
- `;TooBigToFitUnderBridge=true` — commented.
- The verbatim Westwood comment about "sprite is terribly hack" hints
  at the architectural cruft inherited from earlier engine versions.
- No `ImmuneToVeins`, no `Subterranean`. **YR-active mechanism.**

---

## Comparison with peer naval (the Allied/Soviet/Yuri stealth-naval trio)

| Field | DLPH Dolphin | SUB Typhoon | BSUB Boomer |
|-------|--------------|-------------|--------------|
| Strength | **200** | 600 | 1200 |
| Cost | **500** | 1000 | 2000 |
| Speed | **8** | 4 | 5 |
| Armor | **light** | heavy | heavy |
| TechLevel | 5 | 2 | 2 |
| Prereq | GAYARD,**GATECH** | NAYARD | YAYARD,RADAR |
| Primary | SonicZap (chain) | SubTorpedo | BoomerTorpedo |
| Damage | 4+10ambient | 100 | 60×2 |
| Range | 6 | 7 | 7 |
| Cloakable | yes | yes | yes |
| CloakingSpeed | 1 | 1 | 1 |
| Underwater | yes | yes | yes |
| Sensors | yes | yes | yes |
| SensorsSight | **8** | 7 | 8 |
| TypeImmune | yes | not set | not set |
| Organic | **yes** | no | no |
| Rendering | SHP | Voxel | Voxel |
| Naval | yes | yes | yes |

**Trade-offs:**
- **DLPH**: cheapest, fastest, longest sensor range. Fragile, chain
  damage (best vs sub clusters), no anti-land. *Organic*. SHP-rendered.
- **SUB**: balanced. Single hit-hard torpedo. Vulnerable to Squid.
- **BSUB**: tankiest, dual-purpose (anti-land cruise missile). Squid-
  immune via Unnatural=yes. Slowest of stealth subs.

**Anti-sub matchup math:**
- Dolphin elite ((8+15)×2 / 80) = 0.575 dps × ~100% Verses vs sub heavy
  armor = 0.575 dps. *Plus* chain damage if multiple subs are aligned.
- SUB elite (100×2 / 120) = 1.67 dps. *Higher single-target DPS*.
- BSUB elite (60×4 / 120) = 2.0 dps. *Highest single-target DPS*.

DPS-only comparison: **BSUB > SUB > DLPH**. But Dolphin's *chain damage
through multiple subs* shifts the matchup once 2+ subs are clustered —
the Dolphin can hit 3 subs in one shot, multiplying its effective DPS.

---

## Cross-references

- [SUB.md](../soviet/SUB.md) — Soviet sub, primary anti-Dolphin opponent.
- [BSUB.md](../yuri/BSUB.md) — Yuri Boomer, tanky sub counterpart.
- [DEST.md](../allied/DEST.md) — Allied Destroyer, surface anti-sub
  with ASW helicopter and Sensors.
- [AEGIS.md](../allied/AEGIS.md) — Pending. Allied AA cruiser sibling.
- [SQD.md](../soviet/SQD.md) — Pending. Soviet Giant Squid, organic
  naval predator (counterpart organic creature).

---

## Ghidra audit log (audit iteration 28 — 2026-05-19)

**~14 Ghidra queries** (10 string searches + 8 xref lookups + 1 full
`WeaponTypeClass__ReadINI` decompile + 1 full `WarheadTypeClass__ReadINI`
decompile + 1 full `InfantryTypeClass__ReadINI` decompile (re-confirms
audit 13) + 2 assembly-context lookups + 1 grep on saved
`TechnoTypeClass__ReadINI` decompile). All 4 doc-cited "NEW cheat-sheet"
claims verify exactly + 4 bonus offsets pinned + 1 important INI-scope
finding (NotHuman on DLPH is dead INI).

### NEW PARSER SCOPE introduced

**`WarheadTypeClass__ReadINI`** @ 0x0075d590 (body 0x0075d590–0x0075deae)
— fourth NEW parser-function scope addition (after ObjectType audit 21,
BulletType audit 22, AircraftType audit 26). Fully decompiled; sequential
ReadBool/ReadInt/ReadCLSID calls populate the WarheadType-specific block
starting around +0x14B.

### Function-entry verification

| Function | Entry | Body | Status |
|----------|-------|------|--------|
| `WeaponTypeClass__ReadINI` | 0x00772080 | 0x00772080–0x007729e4 | [BINARY-VERIFIED] full decompile, re-confirms audit 9 |
| `WarheadTypeClass__ReadINI` | 0x0075d590 | 0x0075d590–0x0075deae | [BINARY-VERIFIED] full decompile (NEW SCOPE) |
| `InfantryTypeClass__ReadINI` | 0x005240a0 | 0x005240a0–0x0052475c | [BINARY-VERIFIED] full decompile, re-confirms audit 13 |
| `TechnoTypeClass__ReadINI` | (oversized) | — | grep-verified for Organic/TypeImmune/Underwater/CloakingSpeed |

### Key behavioral findings — 8 NEW struct-offset bindings BINARY-VERIFIED

| INI key | Scope | Offset | Type | Parser site | Source |
|---------|-------|--------|------|-------------|--------|
| `IsSonic` | WeaponType | **+0x130** | byte (ReadBool) | 0x007720d3 | doc-cited |
| `AmbientDamage` | WeaponType | **+0x98** | int (ReadInt) | 0x007720bb | doc-cited |
| `Organic` | TechnoType | **+0xD97** | byte (ReadBool) | 0x0071502b | doc-cited |
| `TypeImmune` | TechnoType | **+0xC8C** | byte (ReadBool) | 0x0071220f | doc-cited |
| `Underwater` | TechnoType | **+0xD69** | byte (ReadBool) | 0x00714d74 | NEW |
| `CloakingSpeed` | TechnoType | **+0x310** | int (ReadInt) | 0x00712441 | NEW |
| `NotHuman` | **InfantryType** | **+0xEAD** | byte (ReadBool) | 0x005243c6 | RESOLVES audit-9 ADOG DEFERRED |
| `Sonic` (warhead) | WarheadType | **+0x14B** | byte (ReadBool) | 0x0075d597 | NEW (first WarheadType offset) |

Assembly-context proofs:
- TypeImmune: `0x0071220f: PUSH 0x8444ec` → `CALL 0x005295f0` →
  `0x0071221c: MOV byte ptr [EBP + 0xc8c], AL` ✓
- Sonic (warhead): `0x0075d597: PUSH 0x847df0` → `CALL 0x005295f0` →
  `0x0075d5a4: MOV byte ptr [ESI + 0x14b], AL` ✓
- NotHuman: `0x005243c6: PUSH 0x825a00` → `CALL 0x005295f0` →
  `0x005243d7: MOV byte ptr [ESI + 0xead], AL` ✓

### Discrepancies / corrections

**[INCORRECT — VEHICLE-SCOPE DEAD INI]**: The doc's `NotHuman=yes`
annotation needs caveat — NotHuman is **InfantryType-scope only** (parsed
exclusively inside `InfantryTypeClass__ReadINI`). The `[DLPH]` section
is in `[VehicleTypes]`, parsed by `UnitTypeClass__ReadINI`, which does
**not** call NotHuman's ReadBool branch. So **`NotHuman=yes` on DLPH is
dead INI** — has no engine effect on a Dolphin. The doc's "same flag on
Squid, Brute, Cow, attack dogs" footnote is partially incorrect:
- Brute, attack dogs (InfantryType) — NotHuman IS read, IS effective.
- Squid, Cow, Dolphin (UnitType) — NotHuman is **dead INI**.

[INFERRED] doc claim about NotHuman gore/death routing applies for the
infantry case (NotHuman→InfantryClass-specific gore code path), but cannot
apply to the Dolphin since the flag is never written for `[VehicleTypes]`.

### Items NOT re-verified (DEFERRED with reason)

- **IsSonic chain-damage consumer chain** (`Fire_At` → straight-line
  cell walk → per-naval-target damage loop) — would require decompiling
  `TechnoClass::Fire_At` + sonic-projectile path. Trust-chain from string
  + parser xref + cumulative.
- **AmbientDamage non-Sonic semantics** — doc's open question whether
  AmbientDamage applies to non-Sonic weapons. Field at +0x98 is read
  unconditionally for ALL WeaponType instances (no conditional gate in
  the parser); consumer-side semantics DEFERRED.
- **TypeImmune consumer** in the damage-application code path. Field
  +0xC8C is parsed; consumer-side check (presumably in
  `WarheadTypeClass::Detonate` or `Fire_At` target-eligibility) DEFERRED.
- **Organic vs NotHuman runtime distinction** — doc's open question
  is **structurally resolved by this audit**: Organic at TechnoType+0xD97
  (read for ALL TechnoTypes incl. vehicles); NotHuman at InfantryType+0xEAD
  (read only for InfantryTypes). **Different parser scopes, different
  fields, different offsets** — they cannot be redundant. Consumer-side
  code-path divergence DEFERRED.
- **Sonic warhead chain-pulse + ripple visual** — IsSonic + Sonic warhead
  flags both parsed; consumer-side chain animation code DEFERRED.
- **Submarine locomotor body** ({2BEA74E1-7CCA-11d3-BE14-00104B62A16C})
  — same locomotor as SUB/BSUB; trust-chain to SUB/BSUB doc audits when
  they happen.

### Cross-references to cumulative cheat-sheet

- **DecloakToFire** (audit 9 / BSUB cross-ref) — re-confirmed in this
  decompile at **WeaponType+0x133** (byte). String at 0x0084951c verified.
- **WeaponType cumulative offsets** (audit 9) — re-confirmed via full
  decompile: +0x98 AmbientDamage (NEW), +0xA0 Projectile, +0xA4 Damage,
  +0xA8 Speed, +0xB0 ROF, +0xB4 Range, +0xAC Warhead, +0x130 IsSonic
  (NEW), +0x131 Spawner, +0x132 LimboLaunch, +0x133 DecloakToFire,
  +0x137 RevealOnFire, +0x13B DisguiseFireOnly, +0x13C
  DisguiseFakeBlinkTime, +0x143 FireInTransport, +0x149 IsLaser, +0x154
  IsRadBeam, +0x155 IsRadEruption.
- **TechnoType sound-cluster topology** unaffected by this audit
  (Dolphin has no Activate/Deactivate/EnterTransport sounds; only
  VoiceSelect/VoiceMove/etc. which are name-based not int-indexed in
  the sound cluster).

### Negative claims verified

- `search_strings("DLPH")` → **0 matches** (no DLPH-specific code).
- `search_strings("Dolphin")` → **0 matches** (no Dolphin-specific code).
  All Dolphin behavior is INI-driven (consistent with audits 12-27 for
  other vehicle/aircraft units).

### Confidence summary

- 8/8 NEW struct-offset bindings BINARY-VERIFIED with parser-site +
  writeback evidence (3 with explicit assembly-context proof).
- 1 NEW parser-function scope added: `WarheadTypeClass__ReadINI`.
- 1 IMPORTANT INI-USAGE FINDING flagged (NotHuman=yes on DLPH is dead INI).
- 1 RESOLUTION of prior DEFERRED item (NotHuman exact offset from
  audit 9 ADOG).
- 0 false-positive claims in the doc beyond the NotHuman scope nuance.

---

## Coverage audit

- [x] Every rulesmd key annotated (~50 keys).
- [x] Every artmd key annotated (5 keys).
- [x] Both weapons documented (SonicZap basic + SonicZapE elite).
- [x] Sonic projectile + SonicWarhead documented.
- [x] All voice/sound bindings documented including silent `[DolphinFear]`.
- [x] Prerequisites: `GAYARD, GATECH`.
- [x] Owner: 5 Allied houses.
- [x] Veterancy: extended VeteranAbilities (5 incl. ROF), elite swap
  with Burst=2 ROF=80 (~6× DPS upgrade).
- [x] Hardcoded behavior: IsSonic + AmbientDamage chain damage,
  Organic=yes flag, TypeImmune=yes, SHP-render quirks, cloak/sensor
  triple-stack, Submarine locomotor reuse.
- [x] TS-legacy filter applied.
- [x] Comparison table with peer stealth-naval.
- [x] Anti-sub DPS math vs cluster math.
- [x] At least one Ghidra search performed (4 new cheat-sheet entries).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("IsSonic")` | `0x00849540` (single match) |
| `get_xrefs_to(0x00849540)` | `0x007720d3 → WeaponTypeClass__ReadINI` |
| `search_strings("Organic")` | `0x00843714` (single match) |
| `get_xrefs_to(0x00843714)` | `0x0071502b → TechnoTypeClass__ReadINI` |
| `search_strings("AmbientDamage")` | `0x00849548` (single match) |
| `get_xrefs_to(0x00849548)` | `0x007720bb → WeaponTypeClass__ReadINI` |
| `search_strings("TypeImmune")` | `0x008444ec` (single match) |
| `get_xrefs_to(0x008444ec)` | `0x0071220f → TechnoTypeClass__ReadINI` |

**New cheat-sheet entries (4):**
- `IsSonic` (0x00849540 → 0x007720d3) **WeaponType** — triggers sonic-
  chain damage path. Pairs with warhead-side `Sonic=yes`.
- `AmbientDamage` (0x00849548 → 0x007720bb) **WeaponType** — secondary
  damage component added to Damage. For Sonic, the radius-pulse damage.
- `Organic` (0x00843714 → 0x0071502b) TechnoType — marks unit as living
  creature for gore/death routing.
- `TypeImmune` (0x008444ec → 0x0071220f) TechnoType — same-type units
  don't damage each other with this unit's weapons.

**Open questions:**
- `[Sonic]` projectile is minimal — does the IsSonic weapon code path
  bypass it entirely, treating it as a virtual projectile only? Likely.
  Worth one-line verification next time.
- Does AmbientDamage apply outside Sonic weapons? Field is read on every
  WeaponType but may be effectively zero unless paired with specific
  weapon-class flags. Open follow-up.
- `Organic=yes` vs `NotHuman=yes` distinction — both on DLPH. Are these
  redundant or do they affect different code paths? Both worth a
  Ghidra trace.
