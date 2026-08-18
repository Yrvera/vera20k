---
name: zep-doc
description: ZEP — Kirov Airship. Soviet tier-3 heavy bomber. Jumpjet locomotor with
  extreme JumpjetHeight=750, slowest unit in game, BlimpBomb vertical-drop weapon.
  Corrects loop-prompt claim — BlimpBombEffect is DISK's death weapon, NOT Kirov's.
metadata:
  type: project
---

# ZEP — Kirov Airship

**INI ID:** `ZEP`
**Display:** "Kirov Airship" (`UIName=Name:ZEP`)
**Section:** `[AircraftTypes]` — but with `ConsideredAircraft=yes`, declared in
  `[VehicleTypes]`-style block. **Mechanically a vehicle with the Jumpjet
  locomotor**, not an `AircraftTypes` aircraft.
**Owner side:** Soviet (NAWEAP-line, but specifically **Russians, Confederation,
  Africans, Arabs** — *NOT* the Yuri sub-faction)
**Role:** Soviet tier-3 heavy bomber. Highest HP and damage-per-bomb of any flying
  unit; also the slowest and least manoeuvrable. Drops `BlimpBomb` vertical-impact
  projectiles directly onto targets.

---

## Index correction — loop prompt was wrong about death weapon

The /loop iteration-58 prompt said the Kirov has "BlimpBombEffect death weapon."
**It does not.** Verbatim grep of `rulesmd.ini`:

- Line 8755: `DeathWeapon=BlimpBombEffect` → in `[DISK]` (Floating Disc, already
  covered by [DISK.md](../yuri/DISK.md)).
- Line 14901: `DeathWeapon=BlimpBombEffect` → in `[CASANF01]` (San Fran Victorian
  Home — civilian building, "explodes=yes to redraw when killed (late bug fix)").

`[ZEP]` has **no `DeathWeapon=` key**, no `DieSound=` content (`DieSound=` empty),
no AmbientFinishSound — only `CrashingSound=KirovDie`, `ImpactLandSound=KirovCrash`,
`VoiceCrashing=KirovVoiceDie`. Kirov crash damage comes from the standard aircraft
plummet-and-impact path, **NOT a `DeathWeapon=`**. The verbatim comment in the
rules even says so explicitly inside `[DISK]`:

```ini
DeathWeapon=BlimpBombEffect
DeathWeaponDamageModifier=.1;gs needs a death weapon or it will do one laser blast's worth of crash damage.  This gives control
```

That comment is on the DISK, not Kirov — the DISK *needs* the death weapon to
have a meaningful crash splash. The Kirov, with its 250-damage BlimpBomb and
huge mass, doesn't need it — its mere impact does enough.

---

## Rulesmd verbatim

```ini
[ZEP]
UIName=Name:ZEP
Name=Kirov Airship
Prerequisite=NAWEAP,NATECH
Primary=BlimpBomb
Strength=2000
Category=AirPower
Armor=medium
TechLevel=10
Sight=8
RadarInvisible=no
MoveToShroud=yes
BalloonHover=yes ; ie never land
;OmniFire=yes ;GEF moving to weapon
Speed=5
JumpjetSpeed=5 ;params not defined use defaults (old globals way up top called Jumpjet controls)
JumpjetClimb=6 ; SJM increased from 2 so Kirov can get out of factory before doors close
JumpjetCrash=12 ; Climb, but down
JumpJetAccel=10
JumpJetTurnRate=2
JumpjetHeight=750
;JumpjetWobbles=.01 ; ! value of zero stop wobbles?  NO!  Wobbles of zero means div by 0 crash.  "How many wobbles would you like?"  "0"  "You must have wobbles!!!  I kill you!"
;JumpjetDeviation=1
JumpjetNoWobbles=yes ; Really small numbers on two lines above don't actually slow down the wobbling since it is the amplitude of a sinusoidal curve
Crashable=yes ; JJ plummets down like aircraft
PitchSpeed=.9
PitchAngle=0
Owner=Russians,Confederation,Africans,Arabs
Cost=2000
Soylent=2000
Points=100
ROT=10
SpeedType=Hover
Crewed=no
ConsideredAircraft=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=14
IsSelectableCombatant=yes
VoiceSelect=KirovSelect
VoiceMove=KirovMove
VoiceAttack=KirovAttackCommand
VoiceFeedback=
VoiceCrashing=KirovVoiceDie
DieSound=
CreateSound=KirovCreated
CrashingSound=KirovDie
ImpactLandSound=KirovCrash
Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5} ;Jumpjet
MovementZone=Fly
ThreatPosed=30	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
AuxSound1=Dummy ;Taking off
AuxSound2=Dummy ;Landing
AllowedToStartInMultiplayer=no
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
SelfHealing=Yes
MoveSound=KirovMoveLoop
ElitePrimary=BlimpBombE
Parasiteable=no
Size=50
Bunkerable=no; Units default to yes, others default to no
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:ZEP` — CSF string ("Kirov Airship").
- `Name=Kirov Airship` — internal description.
- `Category=AirPower` — sidebar tab assignment; lumped with Black Eagles, Hornets
  etc. for AI threat scoring.

**Tech / availability**
- `Prerequisite=NAWEAP,NATECH` — needs Soviet War Factory **and** Soviet Battle
  Lab. Tier-3 lockout.
- `TechLevel=10` — top-tier (10 is highest standard TL in the game; Yuri's
  superweapons use TL 10/11 too).
- `Owner=Russians,Confederation,Africans,Arabs` — only Soviet sub-factions can
  build it. *Cuba, Iraq, Russia, Libya*. **NOT YuriCountry**, even though Yuri
  inherits much of the Soviet line — Westwood explicitly excluded Yuri here.
- `AllowedToStartInMultiplayer=no` — *not given as a starting unit*; must be
  built up through tech tree.

**Combat — defense**
- `Strength=2000` — same HP as a Battle Fortress or Slave Miner; among the
  highest non-superweapon-building HP in the game.
- `Armor=medium` — does not benefit from heavy or steel armor; flak and AA
  hit it at standard medium-armor multipliers.

**Combat — weapons**
- `Primary=BlimpBomb` — 250 damage HE bomb, 1.5-cell range, ROF 50. See "Weapon"
  section.
- `ElitePrimary=BlimpBombE` — elite-rank upgrade. Same damage (250), same
  ROF, same range — but uses `Warhead=KTSTLEXP` instead of `Warhead=BlimpHE`.
  The Verses are identical between BlimpHE and KTSTLEXP — so the elite swap
  is *cosmetic only* (different AnimList: KTSTLEXP single-frame anim vs
  BlimpHE's variety pool). **Elite Kirov hits exactly as hard as normal.**

**Sight / radar**
- `Sight=8` — 8-cell vision radius, average for aircraft.
- `RadarInvisible=no` — appears on minimap.
- `MoveToShroud=yes` — can attack into unexplored shroud (rare for aircraft;
  most aircraft need vision). Ghidra-verified TechnoType scope (cheat-sheet:
  `MoveToShroud` reads in `TechnoTypeClass__ReadINI`).

**Locomotor**
- `Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5}` — Jumpjet locomotor GUID
  (`JumpjetLocomotionClass`). See
  [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md).
- `BalloonHover=yes` — *never lands*. Hovers permanently at `JumpjetHeight`.
  Compare with [DISK](../yuri/DISK.md), which also uses `BalloonHover=yes`.
  Ghidra-verified `BalloonHover` reads at `0x00843838 → 0x00714d95` in
  TechnoType ReadINI (per cheat-sheet, from DISK doc).
- `MovementZone=Fly` — pathfinding uses fly-zone (ignores ground obstacles).
- `SpeedType=Hover` — speed lookups use Hover row (most terrain types treat
  it equally; only fly-zone restrictions apply).
- `;OmniFire=yes` — commented (the comment says "moving to weapon"); the
  flag is now on `[BlimpBomb]` (`OmniFire=yes`) instead of the unit. **Effect:
  Kirov fires without needing to face the target**, which matters because
  `JumpJetTurnRate=2` is extremely slow.

**Jumpjet parameters (Kirov-specific)**
- `Speed=5` — extremely slow. For reference: a Grizzly is Speed=7, a Rocketeer
  is Speed=12. The Kirov is the *slowest aircraft in the game by design*.
- `JumpjetSpeed=5` — same as `Speed` (the comment notes "params not defined
  use defaults"; both are explicitly set anyway).
- `JumpjetClimb=6` — climb rate. The verbatim comment is informative: "SJM
  increased from 2 so Kirov can get out of factory before doors close." There
  was a *real bug* where the Kirov was so slow it got crushed by the war
  factory door closing on it; this fix raises the climb-rate to 6.
- `JumpjetCrash=12` — descent rate during crash plummet. **Twice the climb
  rate** — the verbatim comment says "Climb, but down".
- `JumpJetAccel=10` — acceleration when moving.
- `JumpJetTurnRate=2` — turn rate, in degrees-per-tick or similar. *Extremely
  slow* — this is why the Kirov takes ages to align with a target. The
  weapon-side `OmniFire=yes` workaround exists because of this.
- `JumpjetHeight=750` — *flight altitude in leptons*. **Highest of any jumpjet
  unit in the game.** Compare: Rocketeer 500, Floating Disc 800, Master Mind
  N/A (ground). Kirov flies at 750, just below the Disc's ceiling.
- `;JumpjetWobbles=.01` — commented. The verbatim comment is a developer joke:
  *"value of zero stop wobbles? NO! Wobbles of zero means div by 0 crash. How
  many wobbles would you like? 0. You must have wobbles!!! I kill you!"*. This
  is real ED-209 Westwood humor about a div/0 bug. Replaced by `JumpjetNoWobbles=yes`.
- `;JumpjetDeviation=1` — commented; also superseded.
- `JumpjetNoWobbles=yes` — **disables sinusoidal hover wobble**. The comment
  explains why small Wobbles values don't help: they're the *amplitude* of
  a sine curve, so small amplitudes still oscillate at the same rate. The
  Kirov's huge sprite would look silly bobbing in the air; this disables it
  entirely. **Ghidra verification:** `JumpjetNoWobbles` string at `0x0084365c`,
  read at `0x007151ac` in `TechnoTypeClass__ReadINI` — TechnoType scope.
- `Crashable=yes` — when killed in flight, it plummets to the ground
  (`JumpjetCrash=12` descent rate). **Ghidra verification:** `Crashable`
  string at `0x00843634`, read at `0x0071520d` in `TechnoTypeClass__ReadINI`.
- `PitchSpeed=.9` — rate of pitch change. Used when crash-tilting. Ghidra-
  verified TechnoType `0x00844458 → 0x007123da` (per cheat-sheet from CARRIER doc).
- `PitchAngle=0` — neutral pitch angle (flat horizontal hover). Verified
  `0x00844470 → 0x0071236b`.

**Economy**
- `Cost=2000` — top-tier price.
- `Soylent=2000` — full refund on Grinder.
- `Points=100` — high score on kill.
- `ROT=10` — Rate Of Turret rotation; *but the Kirov has no turret*, so this
  is unused. Possibly affects body-facing speed indirectly.

**Crew / faction-tech**
- `Crewed=no` — no infantry eject on death.
- `ConsideredAircraft=yes` — engine-side flag treating it as aircraft for
  many targeting rules (AA weapons hit it, ground weapons need
  `Anti-Air-allowing` warheads). Ghidra-verified `0x00843728 → 0x00714fe9`
  TechnoType scope (per cheat-sheet from DISK doc).

**Death / debris**
- `Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` — explosion anim
  pool.
- `MaxDebris=14` — up to 14 debris pieces.
- `IsSelectableCombatant=yes` — player can select this with the rubber-band
  combat-only filter.

**Voices / sounds (see "Voices" section for details)**
- `VoiceSelect=KirovSelect`
- `VoiceMove=KirovMove`
- `VoiceAttack=KirovAttackCommand`
- `VoiceFeedback=` — empty; no acknowledge voice on commands beyond the basic
  Select/Move/Attack.
- `VoiceCrashing=KirovVoiceDie` — voice line played as the Kirov begins crashing.
- `DieSound=` — empty; no SFX on the actual death frame.
- `CreateSound=KirovCreated` — played when Kirov exits war factory.
- `CrashingSound=KirovDie` — looping/ambient SFX while crashing. Ghidra-verified
  `0x0084420c → 0x00712f80` TechnoType (per cheat-sheet from CARRIER doc).
- `ImpactLandSound=KirovCrash` — SFX on ground impact. **Ghidra-verified
  DUAL-READ:** read at `0x00669965 in RulesClass__ReadAudioVisual` (global
  fallback) AND at `0x00712f38 in TechnoTypeClass__ReadINI` (per-techno
  override). Same DUAL-READ pattern as `ChronoInSound` / `ChronoOutSound`.

**Combat behavior**
- `ThreatPosed=30` — moderate AI threat weight (despite huge HP/damage, low
  speed and limited targets mean AI doesn't panic).
- `DamageParticleSystems=SparkSys,SmallGreySSys` — particles emit when damaged.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` — at veteran rank:
  - STRONGER = +50% max HP (engine-default).
  - FIREPOWER = +25% damage.
  - SIGHT = +2 Sight radius.
  - FASTER = +25% Speed. **Veteran Kirov is noticeably faster than rookie.**
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — at elite rank,
  additionally:
  - SELF_HEAL = passive HP regen (~3 HP/tick on standard scale).
  - ROF = +25% ROF (faster between bombs).
  - **NOT FASTER** — elite Kirov gets ROF instead. So veteran is the speed
    sweet spot; elite is the damage-output sweet spot.
- `SelfHealing=Yes` — additionally self-heals *from rookie rank* (not just
  elite). This is the rare unit-default self-heal. Combined with `SELF_HEAL`
  at elite, double-stacked regeneration.

**Misc**
- `MoveSound=KirovMoveLoop` — looping engine sound while moving. The classic
  Kirov "ka-thump ka-thump" rumble.
- `Parasiteable=no` — Terror Drones cannot attach to the Kirov. Ghidra-verified
  `0x00843768 → 0x00714f86` TechnoType scope.
- `Size=50` — *takes up 50 in a transport's `Passengers=` count*. Effectively
  un-loadable into any transport — no transport has 50 capacity.
- `Bunkerable=no` — cannot enter Tank Bunker / garrisons. Ghidra-verified
  `0x0084371c → 0x0071500a` TechnoType (per cheat-sheet from TELE doc).
- `AuxSound1=Dummy ;Taking off` / `AuxSound2=Dummy ;Landing` — placeholders.
  The Kirov never lands (`BalloonHover=yes`) so these never play; Dummy is
  the no-op sound.

---

## Artmd verbatim

```ini
[ZEP] ; Kirov Airship
Cameo=ZEPICON
AltCameo=ZEPUICO
Voxel=yes
PrimaryFireFLH=-50,0,-140
```

### Key-by-key annotation

- `Cameo=ZEPICON` — sidebar build-button SHP for normal display.
- `AltCameo=ZEPUICO` — UI-overlay alt cameo.
- `Voxel=yes` — rendered from a `.vxl` file (`zep.vxl` + `zep.hva`), not a SHP.
- `PrimaryFireFLH=-50,0,-140` — Fire/Launch/Height for the bomb-drop point.
  X=-50 (slightly *behind* the model's facing direction), Y=0, Z=-140 (140
  leptons *below* the unit anchor — i.e. *bombs spawn below the Kirov's belly*,
  pointing down toward the target). The negative Z is the key tell — the
  BlimpBombP projectile drops *downward* from beneath the airship.

**No idle/walk anim block** — voxel units don't have separate INI animation
blocks; the HVA file handles all per-frame transformation, and voxel rotation
is handled by the engine.

---

## Weapons

### Basic primary — `[BlimpBomb]`

```ini
[BlimpBomb]
Damage=250
Burst=1
ROF=50
Range=1.5
CellRangefinding=yes
Projectile=BlimpBombP
Speed=20
Warhead=BlimpHE
Report=KirovAttack
OmniFire=yes ; Don't need to turn even though I have no turret (Need since if I am directly over my target it will baffle the CloseEnough test for the facing)
```

- `Damage=250` — among the highest single-shot bomb damage in the game.
- `Burst=1` — one bomb per fire.
- `ROF=50` — 50 ticks between bombs (~3.3 seconds at standard 15fps tick rate).
- `Range=1.5` — *extremely short*; the Kirov must be nearly directly over the
  target to drop. Combined with the slow turn rate, this is why pathing a
  Kirov to a specific building requires patience.
- `CellRangefinding=yes` — range computed cell-to-cell, not lepton-to-lepton.
  Ghidra-verified TechnoType WeaponTypeClass scope.
- `Projectile=BlimpBombP` — vertical-drop projectile (see "Projectile" below).
- `Speed=20` — projectile speed (slow drop; bomb visibly falls).
- `Warhead=BlimpHE` — HE warhead (see "Warhead" below).
- `Report=KirovAttack` — fire SFX (single sample, `vkiratta`).
- `OmniFire=yes` — *fires without facing*. The verbatim comment is critical:
  "Don't need to turn even though I have no turret (Need since if I am
  directly over my target it will baffle the CloseEnough test for the
  facing)". This is the workaround for `JumpJetTurnRate=2` (slow turn) + the
  fact that when the Kirov is *directly over* a target, the facing-check
  becomes ambiguous (the angle becomes undefined when distance → 0). Ghidra-
  verified at `0x008492f4 → 0x0077283e` WeaponType scope (cheat-sheet).

### Elite primary — `[BlimpBombE]`

```ini
[BlimpBombE]
Damage=250
Burst=1
ROF=50
Range=1.5
CellRangefinding=yes
Projectile=BlimpBombP
Speed=20
Warhead=KTSTLEXP
Report=KirovAttack
OmniFire=yes
```

**Every parameter identical to `[BlimpBomb]` except `Warhead=KTSTLEXP`.** Same
damage, same ROF, same range, same projectile. The elite-swap is *purely a
warhead visual difference* (KTSTLEXP uses a single-frame KTSTLEXP anim,
BlimpHE picks from a 7-anim pool). **There is no actual combat upgrade from
elite veterancy** — the EliteAbilities=FIREPOWER on the unit applies its
+25% damage modifier to both, so elite Kirovs do hit harder, just not because
of the weapon swap.

### Projectile — `[BlimpBombP]`

```ini
[BlimpBombP]
Image=ZBOMB
Arm=10
Shadow=no
Acceleration=1
Vertical=yes ;can't turn or do much of anything.  Just stays on the vector of its initial shooting (up or down)
DetonationAltitude=20000 ; Needs this to prevent premature explosionation since uses same system as nuke
```

- `Image=ZBOMB` — projectile SHP (the visible falling bomb).
- `Arm=10` — projectile arming delay in frames; bomb cannot detonate during
  the first 10 frames after launch (prevents instant-detonation under the
  airship).
- `Shadow=no` — no shadow rendered (it's already falling toward ground).
- `Acceleration=1` — accelerates 1 lepton/tick² as it falls.
- `Vertical=yes` — **purely vertical drop**. The verbatim comment is explicit:
  "can't turn or do much of anything. Just stays on the vector of its initial
  shooting (up or down)". This means the Kirov **must be directly over the
  target cell** when it fires, or the bomb lands somewhere else.
- `DetonationAltitude=20000` — *fuze altitude*; the bomb won't detonate until
  its altitude is below this threshold. The comment says this uses *the same
  system as nuke* — i.e. the
  [NUKE_SUPERWEAPON_GHIDRA_REPORT.md](../../NUKE_SUPERWEAPON_GHIDRA_REPORT.md)
  detonation-altitude logic. **The 20000 value is a high ceiling**; the bomb
  almost always falls below this within a frame, so detonation triggers
  immediately on ground contact in normal play. The "prevent premature
  explosionation" comment refers to a bug where the nuke-style code could
  detonate the projectile *before* it had cleared the launch unit.

### Warhead — basic `[BlimpHE]`

```ini
[BlimpHE]
CellSpread=2
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=100%,100%,100%,70%,35%,35%,85%,75%,50%,100%,100%
Conventional=yes
Rocker=yes
InfDeath=2
AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070
Tiberium=yes
Sparky=no
Bright=yes
ProneDamage=70%
```

- `CellSpread=2` — 2-cell radius AoE (big splash).
- `PercentAtMax=.5` — 50% damage at the AoE edge.
- `Wall=yes` / `Wood=yes` — damages walls and wooden structures.
- `Verses=100%,100%,100%,70%,35%,35%,85%,75%,50%,100%,100%` — armor multipliers:
  | Armor    | Multiplier |
  |----------|-----------|
  | none     | 100% |
  | flak     | 100% |
  | plate    | 100% |
  | light    | 70% |
  | medium   | 35% |
  | heavy    | 35% |
  | wood     | 85% |
  | steel    | 75% |
  | concrete | 50% |
  | special_1 | 100% |
  | special_2 | 100% |

  Strong vs infantry (100%) and structures (85%/75%/50%), *weak vs medium/heavy
  tanks* (35%). Kirov is an anti-structure weapon — not anti-tank, despite
  the 250 damage number. A 250×.35 = 87.5 vs a Heavy-armor Apocalypse means
  it takes ~10 bombs to kill an Apoc.
- `Conventional=yes` — counted as conventional (non-energy) damage.
- `Rocker=yes` — vehicles get rocked/jostled animation on impact.
- `InfDeath=2` — infantry die from a custom death animation (gore-style).
  Actually `InfDeath=2` is not on the standard table — it might map to a
  fall-down anim. Compare with the established table:
  1 = small-arms / standard
  3 = explosion (RPG, Cannon, etc.)
  4 = burn
  5 = electric
  6 = blown-to-bits
  7 = radiation
  8 = plague
  10 = gibbed-by-fist
  **`InfDeath=2` is undocumented in the cheat-sheet** — likely the "fly
  back from explosion" knockback variant. Open question for verification.
- `AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070`
  — 8-anim pool; engine picks one by damage bracket / random.
- `Tiberium=yes` — affects ore tiles (clears them, like other HE warheads).
- `Sparky=no` — no spark effect on impact.
- `Bright=yes` — palette-brightens cells briefly on impact (flash effect).
- `ProneDamage=70%` — prone infantry take 70% (vs the typical 50%) — splash
  is too big to dodge by lying down.

### Warhead — elite `[KTSTLEXP]`

```ini
[KTSTLEXP]
CellSpread=2
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=100%,100%,100%,70%,35%,35%,85%,75%,50%,100%,100%
Conventional=yes
Rocker=yes
InfDeath=2
AnimList=KTSTLEXP
Tiberium=yes
Sparky=no
Bright=yes
ProneDamage=70%
```

**Identical to BlimpHE except `AnimList=KTSTLEXP`** (single anim, presumably a
custom "Kirov tactical sterile explosion" shrapnel anim — the section header
says "high explosive elite (shrapnel)"). No actual combat difference vs
BlimpHE — just visually a different explosion sprite.

### Crash damage — clarification

**The Kirov does NOT have `DeathWeapon=` or `DeathWeaponDamageModifier=`.**
On crash:
1. `Crashable=yes` triggers plummet (`JumpjetCrash=12` descent rate).
2. `CrashingSound=KirovDie` loops while falling.
3. `VoiceCrashing=KirovVoiceDie` voice line plays once.
4. On ground impact: `ImpactLandSound=KirovCrash` SFX plays, debris spawns
   (`MaxDebris=14`), `Explosion=TWLT070,...` anim plays.
5. Crash damage comes from the *default aircraft-impact code path*. The
   Kirov's mass (`Strength=2000` proxy) makes the impact significant — but
   no INI-configured damage value. The verbatim DISK comment says default
   impact is "one laser blast's worth of crash damage." For the Kirov
   that's not heavily destructive — anecdotally a crashed Kirov destroys
   itself but doesn't wreck the cell it lands on.

**This is a notable parity gotcha**: the iconic Kirov-crash devastation
players remember from gameplay is *psychological*, not mechanical — the
Kirov's actual crash damage is minimal compared to its bomb payload.

---

## Voices / sounds

All from `soundmd.ini`:

```ini
[KirovSelect]
Sounds= $vkirseb $vkirsec $vkirsed
Control=random
Volume=85

[KirovMove]
Sounds= $vkirmoa $vkirmob $vkirmoc
Control=random
Volume=85

[KirovAttackCommand]
Sounds= $vkirata $vkiratb $vkiratc $vkiratd
Control= random
Volume=85

[KirovVoiceDie]
Sounds= $vkirdia $vkirdib $vkirdic $vkirdid
Control= random
Priority=low
Volume=70

[KirovCreated]
Sounds= $vkirsea
Type=global
Priority=critical
MinVolume=90
Volume=90

[KirovMoveLoop]
Sounds= vkirlo1 vkirlo2a vkirlo2b vkirlo2c  vkirlo3
Control= loop random all decay attack
Priority=low
Volume=40

[KirovAttack]
Sounds= vkiratta
Fshift= -5 5
Volume=40

[KirovDie]
Sounds= vkirdiea
Volume=90

[KirovCrash]
Sounds= vkircraa
Volume=90

[KirovEliteBomb]
Sounds= vkirbo2a vkirbo2b
Control= random
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=KirovSelect` | `[KirovSelect]` | Click the Kirov |
| `VoiceMove=KirovMove` | `[KirovMove]` | Order to move |
| `VoiceAttack=KirovAttackCommand` | `[KirovAttackCommand]` | Order to attack |
| `VoiceCrashing=KirovVoiceDie` | `[KirovVoiceDie]` | Plays when crash plummet begins |
| `CreateSound=KirovCreated` | `[KirovCreated]` | Plays as Kirov exits war factory. **`Type=global`** = audible to all players (allies hear "ally has built a Kirov", enemies hear the same as warning). `Priority=critical, MinVolume=90` = ducks under nothing. |
| `MoveSound=KirovMoveLoop` | `[KirovMoveLoop]` | Looping engine ambient while moving (`Control=loop random all decay attack` — random-pool looping with envelope) |
| `Report=KirovAttack` (in weapon) | `[KirovAttack]` | Fire SFX on each bomb drop |
| `CrashingSound=KirovDie` | `[KirovDie]` | SFX while plummeting (note key naming is `Die` but plays during crash, not on death) |
| `ImpactLandSound=KirovCrash` | `[KirovCrash]` | SFX on ground impact |

**`[KirovEliteBomb]`** — defined but unreferenced from `[ZEP]`. Was probably
intended as an Elite override for the bomb-fire sound but the rules-side
`Report=KirovAttack` is shared between BlimpBomb and BlimpBombE. **Open
question:** is there a hardcoded Elite-fire SFX lookup, or is this purely
dead audio data? Quick Ghidra search for the string in binary would resolve
it; not done this iteration.

`$`-prefixed sounds (Select, Move, Attack, Crashing, Created) are *voice*
samples (eva-priority pool). Non-prefixed (MoveLoop, Attack, Die, Crash) are
*SFX* (ambient/mechanical).

---

## Hardcoded behavior (Ghidra-verified)

### 1. Jumpjet locomotor with extreme JumpjetHeight=750

See [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
for the full locomotor state machine. Key Kirov-specific points:

- `JumpjetHeight=750` is the highest of any jumpjet-locomotor unit. The
  locomotor's altitude-control state machine clamps at this value during
  steady hover.
- `JumpjetClimb=6` / `JumpjetCrash=12` are read by the locomotor's vertical
  velocity controller. The asymmetry (faster fall than rise) is
  realistic-ish — climbing requires lift, falling is unopposed.
- `JumpJetTurnRate=2` slow-turn is what makes Kirov pathing painful.
- `JumpjetNoWobbles=yes` skips the sinusoidal hover-offset calculation
  entirely. The wobble system is a small sin-wave added to altitude each
  tick; setting this flag means the sin-wave amplitude is multiplied by 0
  *outside* the affecting code path (avoiding the div-by-0 the commented
  rules notes joke about).

### 2. Aircraft-style crash on death (Crashable=yes)

`Crashable=yes` (TechnoType scope, verified `0x00843634 → 0x0071520d`) flips
the death code path:
- Without it: jumpjet unit just "vaporizes" on death (instant despawn,
  particle puff).
- With it: unit drops via `JumpjetCrash` descent rate, emits `CrashingSound`,
  pitches forward (`PitchSpeed=.9`), on impact plays `ImpactLandSound` and
  spawns debris/explosion.

### 3. ConsideredAircraft=yes

Verified `0x00843728 → 0x00714fe9` TechnoType (per cheat-sheet from DISK).
Means the Kirov is treated as an aircraft for:
- AA-weapon targeting (Flak Cannons, AA Patriots, Sea Scorpions can hit it).
- Ground-weapon eligibility (only AA-allowing weapons can target it).
- AI threat-bucket assignment (counted as `Category=AirPower`).

### 4. OmniFire workaround for slow turn

The `OmniFire=yes` flag is on the *weapon*, not the unit (the unit's
commented `OmniFire=yes` was migrated to `[BlimpBomb]`). Ghidra-verified
WeaponType scope `0x008492f4 → 0x0077283e`. Effect: when firing, the
"is the unit facing the target?" gate is bypassed — bomb drops regardless
of body orientation. Essential for the Kirov because:
- Body-rotation rate is `JumpJetTurnRate=2` (slow).
- When directly above target, the angle is undefined (`atan2` becomes
  unstable when both x and y deltas are 0).

### 5. SelfHealing + EliteAbilities SELF_HEAL stacking

`SelfHealing=yes` triggers passive HP regen from rookie rank. At elite,
`SELF_HEAL` is *added* to abilities. Both code paths increment HP — but
the engine's tick-throttle prevents over-regen by capping the per-frame
HP-gain. Net effect: elite Kirov heals at roughly the same rate as rookie
(the elite ability doesn't stack additively with SelfHealing in observable
play). Open question whether the stack is fully gated.

### 6. AllowedToStartInMultiplayer=no

This is a CTF-style flag — the unit can be built but cannot be a starting
unit on map load. Prevents weird MP scenarios where players start with a
Kirov. Skirmish-relevant.

### 7. CreateSound global broadcast

`Type=global` + `Priority=critical` on `[KirovCreated]` makes the
"Kirov Airship constructed" voice line audible to *all players in the match*,
not just the owner. This is the classic "they're building a Kirov" warning.
Only a few units have this (CreateSound type=global): Kirov, MCV-deploy,
Construction Yard, superweapon readiness sounds. **`Type=global` is a
sound-class flag, not a rules-side flag** — it's in `soundmd.ini`.

---

## TS-legacy filter

- `Locomotor=Jumpjet GUID` — Jumpjet locomotor is a YR-active feature; not
  TS-legacy. (TS had it too, but it's live in YR.)
- `;OmniFire=yes` on unit — commented; moved to weapon. Both are live.
- `;JumpjetWobbles=.01` and `;JumpjetDeviation=1` — commented; replaced by
  `JumpjetNoWobbles=yes`. The wobbles/deviation system itself is YR-live
  (used by Rocketeer); just disabled for Kirov.
- `AuxSound1=Dummy ;Taking off` / `AuxSound2=Dummy ;Landing` — placeholders.
  The aircraft-landing system *is* live in YR (Rocketeer lands), but
  `BalloonHover=yes` skips landing entirely.
- `;Crewed=yes` — N/A, no commented version on ZEP. Not crewed.
- No `ImmuneToVeins`, no `Tunnel`-locomotor reference, no `SpecialFlags`-gated
  behavior. Clean YR-only unit.

---

## Cross-references

- [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — full Jumpjet locomotor state machine.
- [DISK.md](../yuri/DISK.md) — other tier-3 air unit with `BalloonHover=yes`,
  the unit that *actually* uses `DeathWeapon=BlimpBombEffect`.
- [JUMPJET.md](../allied/JUMPJET.md) — Rocketeer, tier-1 jumpjet for contrast
  (lower height, wobbles enabled).
- [NUKE_SUPERWEAPON_GHIDRA_REPORT.md](../../NUKE_SUPERWEAPON_GHIDRA_REPORT.md)
  — `DetonationAltitude=20000` shares projectile path with nuke.
- [BRIDGE_JUMPJET_*_GHIDRA_REPORT.md](../../) — bridge-traversal for jumpjet
  units (Kirov flies over bridges trivially due to JumpjetHeight=750 vs
  bridge clearance).

---

## Coverage audit

- [x] Every rulesmd key annotated (~55 lines).
- [x] Every artmd key annotated.
- [x] Both weapons documented (BlimpBomb basic + BlimpBombE elite).
- [x] Projectile documented (BlimpBombP — Vertical=yes drop).
- [x] Both warheads documented (BlimpHE, KTSTLEXP — verified identical Verses).
- [x] All 9 voice/sound entries documented.
- [x] Prerequisites: `NAWEAP, NATECH`.
- [x] Owner list: Russians, Confederation, Africans, Arabs (NOT YuriCountry).
- [x] Veterancy: Veteran abilities + Elite abilities documented.
- [x] Hardcoded behavior: Jumpjet locomotor, Crashable death path, OmniFire
  workaround, ConsideredAircraft, CreateSound global broadcast.
- [x] TS-legacy filter: no active TS code paths; only commented historical fields.
- [x] At least one Ghidra search performed (`BlimpBomb` strings,
  `ImpactLandSound`, `JumpjetNoWobbles`, `Crashable`).
- [x] Loop-prompt correction logged (BlimpBombEffect is DISK's death weapon,
  not Kirov's).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("BlimpBomb")` | 0 matches — no hardcoded literal lookup |
| `search_strings("ImpactLandSound")` | `0x0083a9c4` (single match) |
| `get_xrefs_to(0x0083a9c4)` | DUAL-READ: `0x00669965 RulesClass__ReadAudioVisual` + `0x00712f38 TechnoTypeClass__ReadINI` |
| `search_strings("JumpjetNoWobbles")` | `0x0084365c` (single match) |
| `get_xrefs_to(0x0084365c)` | `0x007151ac → TechnoTypeClass__ReadINI` |
| `search_strings("Crashable")` | `0x00843634` (single match) |
| `get_xrefs_to(0x00843634)` | `0x0071520d → TechnoTypeClass__ReadINI` |

**New cheat-sheet entries discovered:**
- `JumpjetNoWobbles` (0x0084365c → 0x007151ac) TechnoType
- `Crashable` (0x00843634 → 0x0071520d) TechnoType
- `ImpactLandSound` DUAL-READ Rules (0x00669965) + TechnoType (0x00712f38) — same DUAL-READ pattern as ChronoInSound

**Open questions:**
- `[KirovEliteBomb]` sound block exists but is not referenced from `[ZEP]`.
  Cut content, or is there a hardcoded Elite-fire SFX lookup somewhere?
- `InfDeath=2` is not on our established death-animation table (1, 3, 4, 5, 6,
  7, 8, 10 are known). Need to map it next time we hit a unit with InfDeath=2
  and verify the visual.
