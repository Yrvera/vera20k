---
name: bsub-doc
description: BSUB — Yuri Boomer submarine. Dual-weapon (BoomerTorpedo anti-naval +
  CruiseLauncher anti-land via CMISL spawn-missiles); Cloakable+Underwater sub
  locomotor; NavalTargeting=7/LandTargeting=2 dual-target priority; DecloakToFire=no
  on torpedo (fires from cloak); Spawns=CMISL one-shot missiles (SpawnReloadRate=0).
metadata:
  type: project
---

# BSUB — Yuri Boomer Submarine

**INI ID:** `BSUB`
**Display:** "Yuri Boomer" (`UIName=Name:Boomer`)
**Section:** `[VehicleTypes]`
**Owner side:** Yuri (`Owner=YuriCountry`)
**Role:** Yuri's tier-2 naval flagship. **Dual-purpose** submarine: torpedoes for
naval combat, cruise missiles for long-range land strikes. Cloaks underwater,
fires from cloak (DecloakToFire=no on torpedoes), and out-ranges every land
defense with its Range=20 spawn-missile launcher. Single most cost-effective
Yuri unit pound-for-pound.

---

## Rulesmd verbatim

```ini
[BSUB]
UIName=Name:Boomer
Name=Yuri Boomer
;Image=SUB
Prerequisite=YAYARD,RADAR
Primary=BoomerTorpedo
Secondary=CruiseLauncher;CruiseMissile
NavalTargeting=7
LandTargeting=2
FireAngle=64
Category=AFV
Strength=1200
Naval=yes
Armor=heavy
TechLevel=2
Underwater=yes
Sight=8
Sensors=yes
SensorsSight=8
Speed=5
CrateGoodie=no
Owner=YuriCountry
AllowedToStartInMultiplayer=no
Cost=2000
Soylent=2000
Turret=no
Points=30
ROT=2
Crusher=no;gs yes
Crewed=no
Weight=4
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=BoomerSelect
VoiceMove=BoomerMove
VoiceAttack=BoomerAttackWaterCommand
VoiceSecondaryWeaponAttack=BoomerAttackLandCommand
VoiceFeedback=SubFear
DieSound=GenSmallWaterDie
MoveSound=BoomerMoveStart
Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};{4A582741-9839-11d1-B709-00A024DDAFD1}
SpeedType=Float
MovementZone=Water
ThreatPosed=20	; This value MUST be 0 for all building addons
Accelerates=true
Cloakable=yes
CloakingSpeed=1
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
TooBigToFitUnderBridge=true
ElitePrimary=BoomerTorpedoE
Size=20
Spawns=CMISL
SpawnsNumber=2
SpawnRegenRate=80
SpawnReloadRate=0 ; missile spawn don't come back
;NoSpawnAlt=yes ; alternate voxel for out of spawns: xxxxWO (DREDWO)
Unnatural=yes	; for underwater units this means that they will be punched instead of grabbed by a squid
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:Boomer` — CSF key. Resolves to "Yuri Boomer".
- `Name=Yuri Boomer` — internal description.
- `;Image=SUB` — commented. Was once going to reuse Soviet Typhoon Sub
  art (`sub.vxl`); Westwood gave BSUB its own voxel. The commented line
  is harmless.
- `Category=AFV` — AI threat-bucket. *Same category as land tanks*
  despite being naval — unusual; Soviet SUB and Allied DEST also use
  AFV. Naval-vs-land classification happens via `Naval=yes`.

**Tech / availability**
- `Prerequisite=YAYARD,RADAR` — Yuri Sub Pen (Naval Shipyard) **and**
  any radar building. RADAR is a macro that resolves to any radar
  building of the owning house (typically Psychic Sensor or ConYard-tier
  radar).
- `TechLevel=2` — tier-2. **Surprisingly low** — given the firepower
  envelope, Boomer Sub feels tier-3 but is gated to tier-2.
- `Owner=YuriCountry` — Yuri only.
- `AllowedToStartInMultiplayer=no` — not a starting unit.
- `CrateGoodie=no` — not crate-eligible.

**Combat — defense**
- `Strength=1200` — among the highest naval HP (Soviet Dreadnought=1000,
  Allied Aircraft Carrier=1000). Boomer Sub outlasts both.
- `Naval=yes` — **TechnoType flag**. Marks the unit as naval class
  (separate code paths for movement / squid grab / port logic).
- `Armor=heavy` — heavy armor type.

**Combat — dual-weapon system**

This is the unit's defining mechanic:
- `Primary=BoomerTorpedo` — **anti-naval/underwater**. Range=7, Damage=60
  Burst=2 (effective 120 dmg per salvo), Torpedo projectile.
- `Secondary=CruiseLauncher;CruiseMissile` — **anti-land**, long-range.
  Range=20 (game-longest non-superweapon range), MinimumRange=8, Burst=2
  CMISL spawn-missiles. The trailing `;CruiseMissile` is a historical
  commented alternate name.
- `ElitePrimary=BoomerTorpedoE` — Burst=4 (double the torpedo salvo).
  *No `EliteSecondary`* — the cruise missile launcher doesn't get an
  elite upgrade. Same single-elite-weapon-swap pattern as MGTK Mirage.
- `NavalTargeting=7` — *targeting priority bias when attacking naval/
  underwater targets*. Ghidra-verified TechnoType at
  `0x00844510 → 0x007121be` (from SMIN doc cheat-sheet). Higher value =
  *more eagerly* engages naval targets. Boomer Sub aggressively favors
  naval engagement.
- `LandTargeting=2` — same logic but for *land targets*. Ghidra-verified
  TechnoType at `0x00844520 → 0x007121a4`. **NEW cheat-sheet entry**.
  Lower value than naval = the Boomer Sub is *more reluctant* to engage
  land targets — must be explicitly ordered, doesn't auto-engage cities.
- `FireAngle=64` — projectile launch angle (per cheat-sheet
  `0x00843910 → 0x00714b5d` TechnoType from MIND doc). 64 lift-angle for
  the cruise missiles to clear the water surface and arc inland. The
  torpedo fires nearly-horizontal.

**Sight / sensors / cloaking**
- `Sight=8` — 8-cell vision.
- `Sensors=yes` — *cloak-detector*. Boomer Sub can spot enemy cloaked
  subs at `SensorsSight=8`-cell range. Same flag as Destroyer (per DEST
  doc, TechnoType `0x00843e58 → 0x00714003`).
- `SensorsSight=8` — sensor detection range, matches Sight.
- `Cloakable=yes` — **submerges/cloaks while idle**, like all submarines.
  Standard TechnoType cloak system.
- `CloakingSpeed=1` — how fast cloak engages on idle. Value 1 = nearly
  instant (compare: Boris's cloak speed 5, slow). Boomer Sub cloaks fast.

**Mobility**
- `Speed=5` — same as Rhino, slower than Hover units.
- `ROT=2` — *very* slow turn rate. The Boomer Sub is unwieldy.
- `Accelerates=true` — *unlike land tanks*, the Boomer Sub uses gradual
  acceleration. Naval realism — subs ramp up speed gradually.
- `SpeedType=Float` — naval speed table.
- `MovementZone=Water` — water-only pathing.
- `Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C}` — **Submarine
  locomotor GUID**. Distinct from Drive (`...741`) and Hover (`...742`).
  The trailing `;{4A582741-...}` after the GUID is a *commented-out
  alternate Drive locomotor* (historical — when the BSUB used a generic
  ground-vehicle locomotor before the Sub locomotor was finalized).
- `Underwater=yes` — *renders below water surface*. Visible only to
  sensor-equipped units. **Ghidra-verified TechnoType** at
  `0x00843848 → 0x00714d74`. **NEW cheat-sheet entry**.
- `Unnatural=yes` — **affects Giant Squid grab behavior**. The verbatim
  comment says: "for underwater units this means that they will be
  punched instead of grabbed by a squid". The Squid's anti-naval grapple
  attack normally pulls vehicles to the surface; against Unnatural=yes
  units it switches to a *punch* attack instead. Per cheat-sheet
  `Unnatural` reads in TechnoType.
- `TooBigToFitUnderBridge=true` — UnitType-scope; can't pass under bridges.

**Economy**
- `Cost=2000` — premium tier-2 price.
- `Soylent=2000` — full refund.
- `Points=30` — moderate score.

**Crew / death**
- `Crewed=no` — no infantry eject.
- `Crusher=no;gs yes` — *cannot crush*. The commented "gs yes" (Greg
  Smith) is a historical override; final spec is no-crush. Boomers
  don't crush anything (water vehicles don't crush land infantry).
- `Weight=4` — heaviest naval physics weight.
- `Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` — explosion pool.
- `DieSound=GenSmallWaterDie` — generic small-naval death SFX (`vgendieb`
  variants in water).

**Voice / sound bindings**
- `VoiceSelect=BoomerSelect` — click voice (5-sample $vboose* pool).
- `VoiceMove=BoomerMove` — move-order voice.
- `VoiceAttack=BoomerAttackWaterCommand` — *primary-weapon attack voice*
  (water target, 6-sample $vbooa2* pool).
- `VoiceSecondaryWeaponAttack=BoomerAttackLandCommand` — *secondary-
  weapon attack voice* (land target, 6-sample $vbooa1* pool). Ghidra-
  verified TechnoType `0x00844038` (per cheat-sheet).
- `VoiceFeedback=SubFear` — ambient feedback (likely when taking damage
  or near enemy ships); plays a "sub anxiety" voice. Naval-specific.
- `MoveSound=BoomerMoveStart` — sub-engine ignition (2-sample random
  predelay).

**Behavior flags**
- `Turret=no` — no rotating turret. The torpedo tubes and missile
  launcher both fire from the body.
- `ThreatPosed=20` — moderate AI threat.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — *5 abilities*
  (one more than standard MBT). Extra ability: ROF (faster fire rate).
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — standard.
- Plus the weapon swap to `BoomerTorpedoE` (Burst=4 = double torpedo
  output).

**Size**
- `Size=20` — *enormous*. Cannot fit in any transport (no transport has
  `Passengers=20` capacity). Largest non-MCV Size value I've seen.

**Spawn system (CruiseLauncher feeds CMISL)**
- `Spawns=CMISL` — spawn child type is [CMISL] Cruise Missile aircraft.
- `SpawnsNumber=2` — **only 2 reserve missiles** at full load. Compare:
  - V3 Rocket: 1 (one-shot).
  - Dreadnought: 2 per pair.
  - Aircraft Carrier: 4 Hornets.
  - **Boomer Sub: 2 cruise missiles**. Limited reserve.
- `SpawnRegenRate=80` — 80 ticks (~5.3 sec at 15fps) to regenerate a
  spawn slot. Slow refresh — burst fire 2 missiles, then wait ~10
  seconds for both slots to refill.
- `SpawnReloadRate=0` — **one-shot missiles, no return-to-dock**. The
  verbatim comment is explicit: "missile spawn don't come back". The
  comparison: Carrier's Hornet has `SpawnReloadRate>0` (returns to
  Carrier, reloads). Boomer's CMISL has 0 (suicide-on-impact, identical
  pattern to V3 and Dreadnought missiles).
- `;NoSpawnAlt=yes ; alternate voxel for out of spawns: xxxxWO (DREDWO)`
  — commented. The NoSpawnAlt system swaps to a "missiles-depleted"
  voxel (DRED has DREDWO, V3 has its own). Boomer Sub *doesn't* have
  a depleted-state voxel — the BSUB voxel stays the same regardless.

---

## Artmd verbatim

```ini
[BSUB]	;Yuri Boomer submarine
Cameo=BSUBICON
Voxel=yes
Remapable=yes
PrimaryFireFLH=225,65,0
SecondaryFireFLH=0,0,-40
SecondSpawnOffset=-70,0,0
```

### Key-by-key annotation

- `Cameo=BSUBICON` — sidebar build-button SHP.
- `Voxel=yes` — rendered from `bsub.vxl` + `bsub.hva`.
- `Remapable=yes` — house-color remap applies.
- `PrimaryFireFLH=225,65,0` — torpedo launch offset:
  - X=225 (well forward; torpedo tubes at the bow).
  - Y=65 (offset *off-centerline* — torpedoes fire from the starboard
    side specifically). Distinctive: most tanks fire from Y=0 center.
  - Z=0 (water-level — torpedoes launch horizontally from the hull).
- `SecondaryFireFLH=0,0,-40` — cruise missile launch offset:
  - X=0, Y=0 (centered).
  - Z=-40 (40 leptons *below* hull anchor — missiles launch from a
    *vertical-launch silo* in the hull. The negative Z creates a brief
    "missile rising from below" visual before the projectile begins
    moving).
- `SecondSpawnOffset=-70,0,0` — **second-slot spawn-position offset**.
  Ghidra-verified TechnoType at `0x008431d8 → 0x0071602e`. **NEW
  cheat-sheet entry**. When the second of two simultaneous missiles
  spawns, it uses this offset (X=-70, behind the first). With Burst=2,
  both missiles launch from slightly different points (first at
  SecondaryFireFLH, second offset by SecondSpawnOffset). Visually
  separates the two-missile salvo. This field is read on TechnoType but
  only used by Spawner=yes weapons with Burst>1.

---

## Weapons

### Primary — `[BoomerTorpedo]` (anti-naval)

```ini
[BoomerTorpedo]
Damage=60
ROF=120
Range=7
Projectile=Torpedo
Speed=25 ;18
Report=BoomerAttack2
Warhead=APSplash2
DecloakToFire=no
Burst=2
```

- `Damage=60` per torpedo. With `Burst=2`, each salvo delivers **120
  damage** at Range=7.
- `ROF=120` — 120 ticks (~8 seconds) between salvos. Slow but
  hard-hitting.
- `Projectile=Torpedo` — underwater-tracking torpedo projectile (used by
  SUB, DLPH, BSUB).
- `Speed=25` (`;18` historical commented value).
- `Warhead=APSplash2` — AP+splash variant.
- `DecloakToFire=no` — **the Boomer Sub stays cloaked while firing
  torpedoes**. Ghidra-verified WeaponType at `0x0084951c → 0x00772121`.
  **NEW cheat-sheet entry**. *Critical mechanic*: enemy doesn't see the
  Boomer until they have a Sensors-equipped unit nearby. The torpedoes
  themselves are visible but don't reveal the launcher.
- `Burst=2` — 2 torpedoes per attack.
- `Report=BoomerAttack2` — fire SFX (`vbooat2a`, single sample).

### Elite primary — `[BoomerTorpedoE]`

```ini
[BoomerTorpedoE]
Damage=60
ROF=120
Range=7
Projectile=Torpedo
Speed=18
Report=BoomerAttack2
Warhead=APSplash2
DecloakToFire=no
Burst=4
```

**Two changes vs basic:**
1. `Burst=4` (vs 2) — **double the torpedo salvo**. Per-shot output:
   60 × 4 = **240 damage per salvo**.
2. `Speed=18` (vs 25) — slower torpedoes? Possibly a balance lever for
   the doubled damage; the basic 25 might have been deemed too fast at
   elite.

Effect: elite Boomer Sub triples-down on anti-naval; one elite Boomer
can shred a Destroyer or Dreadnought in 2-3 salvos.

### Secondary — `[CruiseLauncher]` (anti-land)

```ini
[CruiseLauncher]
Damage=25  ;35
ROF=50
Burst=2
Range=20
MinimumRange=8; the missiles need time to align
Spawner=yes
Projectile=InvisibleHigh
Speed=15
Warhead=Special
OmniFire=yes
Report=BoomerAttack1
```

- `Damage=25` (historical `;35`) — the *launcher* damage. The actual
  damage to the target is from the CMISL aircraft's own death warhead.
  Spawner-weapon damage is typically just a small impact contribution.
- `ROF=50` — 50 ticks (~3.3 sec) between salvos. Faster than the
  torpedo's ROF=120.
- `Burst=2` — 2 missiles per salvo (matches `SpawnsNumber=2`).
- `Range=20` — **game-longest non-superweapon weapon range**. For
  reference: Prism Tank=10, V3=20 (also Spawner=yes), Dreadnought=22.
  The Boomer Sub outranges nearly every land defense.
- `MinimumRange=8` — *can't fire at targets closer than 8 cells*.
  Verbatim comment: "the missiles need time to align". Below
  MinimumRange, the Spawner system fails to launch.
- `Spawner=yes` — **launches spawn-children** (CMISL aircraft) instead
  of conventional projectiles. Per cheat-sheet, `Spawner=yes` reads in
  WeaponType at `0x00849538 → 0x007720ed`. The 2 spawn slots are
  defined by the TechnoType-side `Spawns=CMISL SpawnsNumber=2`.
- `Projectile=InvisibleHigh` — used for the spawner placeholder
  projectile (the actual visible flight is the CMISL aircraft itself).
- `Speed=15` — placeholder projectile speed (irrelevant since CMISL
  flies on its own locomotor).
- `Warhead=Special` — placeholder warhead for the launcher's nominal
  damage. The actual damage on impact is CMISL-side.
- `OmniFire=yes` — fires without facing requirement. Combined with
  ROT=2 (very slow turn), the Boomer Sub effectively launches missiles
  from any orientation.
- `Report=BoomerAttack1` — fire SFX (`vbooat1a`, single sample, Limit=2
  concurrent).

### Spawn child — `[CMISL]` Cruise Missile

```ini
[CMISL]
UIName=Name:DMISL  ; intentionally same as Dreadnought missile
Name=Cruise Missile
Image=BSUBMISL
FireAngle=1
Strength=50
Category=AirPower
Armor=special_2
Spawned=yes
MissileSpawn=yes
TechLevel=-1
Sight=0
Speed=20
Owner=YuriCountry
Cost=50
Locomotor={B7B49766-E576-11d3-9BD9-00104B972FE8}
MovementZone=Fly
GuardRange=30
ImmuneToPsionics=yes
FlyBack=true
DontScore=yes
NoShadow=yes
Selectable=no
Trainable=no
```

- `Image=BSUBMISL` — separate art (`bsubmisl.vxl`). The missile flies as
  a small voxel projectile.
- `Spawned=yes` + `MissileSpawn=yes` — **suicide-on-impact missile**
  spawn (same pattern as V3ROCKET, DMISL). The MissileSpawn flag tells
  the spawn manager "this child dies on impact, do not return to dock".
- `Locomotor={B7B49766-E576-11d3-9BD9-00104B972FE8}` — **RocketLocomotion**
  GUID (same as V3, DMISL). The trajectory is computed by the
  RocketLocomotion class — see [ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md).
- `FireAngle=1` — near-horizontal launch angle (very low FireAngle = the
  missile climbs almost flat from the sub).
- `GuardRange=30` — *autonomous attack range while flying* — once
  launched the missile homes on its assigned target up to 30 cells.
- `Strength=50, Armor=special_2` — fragile; can be shot down by AA
  weapons (Flak Cannon, AA Patriot, Sea Scorpion). Same shootdown
  vulnerability as V3 rockets.
- `ImmuneToPsionics=yes` — Yuri/Master Mind can't mind-control the
  missile mid-flight.
- `FlyBack=true` — *flight-pattern flag*. Specifies the post-impact
  cleanup; for MissileSpawn this is mostly inert.
- `DontScore=yes` — kills from CMISL don't grant XP to the Boomer Sub.
  Boomer Sub still gets XP from the torpedo kills, but the cruise
  missiles are "free" (this is why Boomer doesn't elite-up from
  long-range land spam alone).
- `NoShadow=yes` — no shadow rendered for the missile.
- `Selectable=no` — player can't click the missile.

**CMISL warhead**: not visible in the rules block; the death damage
comes from the *projectile/spawn impact* code path. Standard pattern
for MissileSpawn — see V3ROCKET / DMISL for parallels. The specific
warhead used on impact is `DMislWarhead` or similar Rules-global
warhead (need verification — open question).

---

## Voices / sounds

```ini
[BoomerSelect]
Sounds=$vboosea $vbooseb $vboosec $vboosed $vboosee
Control=random
Volume=85

[BoomerMove]
Sounds=$vboomoa $vboomob $vboomoc $vboomod $vboomoe
Control=random
Volume=85

[BoomerAttackLandCommand]
Sounds=$vbooa1a $vbooa1b $vbooa1c $vbooa1d $vbooa1e $vbooa1f
Control=random
Volume=85

[BoomerAttackWaterCommand]
Sounds=$vbooa2a $vbooa2b $vbooa2c $vbooa2d $vbooa2e $vbooa2f
Control=random
Volume=85

[BoomerAttack1]
Sounds=vbooat1a
FShift= -10 10
Limit=2
Volume=60

[BoomerAttack2]
Sounds= vbooat2a
FShift= -10 10
Volume=50

[BoomerMoveStart]
Sounds= vboostaa vboostab
Control= random predelay
Delay=0 400
Priority=Low
FShift= -10 10
VShift=20
Volume=60

[SubFear]      ; existing block — feedback
Sounds= ...    ; (block exists at line ~1863)
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=BoomerSelect` | `[BoomerSelect]` | Click |
| `VoiceMove=BoomerMove` | `[BoomerMove]` | Move-order voice |
| `VoiceAttack=BoomerAttackWaterCommand` | `[BoomerAttackWaterCommand]` | Primary-weapon attack order (torpedo) |
| `VoiceSecondaryWeaponAttack=BoomerAttackLandCommand` | `[BoomerAttackLandCommand]` | Secondary-weapon attack order (cruise missile) |
| `VoiceFeedback=SubFear` | `[SubFear]` | Ambient damage/anxiety voice |
| `DieSound=GenSmallWaterDie` | shared | Death SFX |
| `MoveSound=BoomerMoveStart` | `[BoomerMoveStart]` | Ignition (2-sample random predelay) |
| `Report=BoomerAttack1` (CruiseLauncher) | `[BoomerAttack1]` | Missile launch SFX (Limit=2 concurrent) |
| `Report=BoomerAttack2` (BoomerTorpedo) | `[BoomerAttack2]` | Torpedo launch SFX |

**`VoiceSecondaryWeaponAttack` is rare** — only units with distinct
primary/secondary weapons that warrant *different* command voices use it.
Other examples: Aegis Cruiser (different air-vs-air-vs-naval voices),
Destroyer (155mm vs ASW).

---

## Hardcoded behavior (Ghidra-verified)

### 1. Dual-weapon NavalTargeting/LandTargeting priorities

`NavalTargeting=7` (TechnoType `0x00844510 → 0x007121be`, established
in SMIN doc) and `LandTargeting=2` (TechnoType `0x00844520 → 0x007121a4`,
**NEW** this iteration) bias the AI's auto-target selection:
- High `NavalTargeting` value = unit aggressively engages naval targets.
- Low `LandTargeting` value = unit reluctant to auto-engage land
  targets.

Boomer Sub's 7-vs-2 ratio means *it preferentially shoots ships when
both options are available*. The cruise missile launcher only fires when
explicitly ordered or when the AI determines a land target is the only
option.

### 2. Cloak+DecloakToFire=no torpedo

The Boomer Sub cloaks via the standard `Cloakable=yes` system. The
`DecloakToFire=no` flag on `[BoomerTorpedo]` (WeaponType-scope,
`0x0084951c → 0x00772121`, **NEW** this iteration) keeps the unit
cloaked while firing. Critical for ambush gameplay: enemies see
torpedoes spawning from "nowhere" until they bring a sensor unit close.

The CruiseLauncher *doesn't* have `DecloakToFire=no` set; so when firing
cruise missiles, the Boomer Sub *decloaks*. **Strategic implication**:
torpedoes from cloak (stealth attack) vs missiles from visible-state
(reveals position). A skilled Yuri player keeps the Boomer cloaked
for torpedo fights and only surfaces when missile bombardment is
worth the position-reveal.

### 3. Underwater=yes + Unnatural=yes Squid interaction

`Underwater=yes` (TechnoType `0x00843848 → 0x00714d74`, **NEW** this
iteration) renders the unit below water surface (visible only to
sensors). `Unnatural=yes` modifies the Soviet Giant Squid's anti-naval
attack: the Squid normally *grabs* surface ships and drags them under,
but against Unnatural=yes Boomer Subs it *punches* instead. Damage
mechanism differs:
- Grab: locked into a multi-tick drain.
- Punch: discrete damage chunks, Boomer can break away.

This is a genuine combat-meaningful mechanic — Yuri's Boomer Sub is
*harder to neutralize with a Squid* than a Soviet Typhoon Sub would be.

### 4. CMISL spawn-child suicide missiles

`Spawns=CMISL SpawnsNumber=2 SpawnReloadRate=0` is the *MissileSpawn
pattern* — same as V3 and Dreadnought:
- 2 reserve missiles.
- Each missile is `Spawned=yes MissileSpawn=yes` — flies to target,
  dies on impact.
- SpawnReloadRate=0 means children don't return to dock.
- `SpawnRegenRate=80` is the *new-missile production rate* (~5 sec to
  build one replacement missile in the launcher).

See [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
for the full SpawnManager state machine. The Boomer Sub's spawn manager
sits at TechnoClass+0x2D8 (same offset as Slave Manager — they share
the slot since Spawn and Slave systems use the same base allocation).

### 5. SecondSpawnOffset positioning

`SecondSpawnOffset=-70,0,0` (TechnoType `0x008431d8 → 0x0071602e`,
**NEW** this iteration) — when Burst=2 spawns 2 missiles in one frame,
the second instance is positioned at this offset relative to the
first's launch point. Used for visual separation of the salvo. Field is
read on TechnoType but only consumed by Spawner=yes weapons with Burst>1.

### 6. FireAngle=64 missile launch

`FireAngle=64` (TechnoType, per cheat-sheet from MIND doc) sets the
upward launch angle in degrees-ish units. 64 is steep enough for cruise
missiles to clear the water surface and arc inland; the torpedoes ignore
FireAngle and fire near-horizontal (different code path, projectile=Torpedo).

### 7. Sensors=yes + SensorsSight=8

Same flag pattern as Destroyer (see [DEST.md](../allied/DEST.md)). The
Boomer Sub detects enemy cloaked units (subs, Mirage Tanks) at 8-cell
range — *meaningfully self-defends against rival subs*.

### 8. VoiceSecondaryWeaponAttack

TechnoType-scope per cheat-sheet (`0x00844038`). Provides a separate
voice block for secondary-weapon attack orders. Required when the
primary and secondary weapons fire under fundamentally different
contexts (naval-vs-land in Boomer's case).

---

## TS-legacy filter

- `;NoSpawnAlt=yes ; alternate voxel for out of spawns: xxxxWO (DREDWO)`
  — commented out. The NoSpawnAlt visual-swap system *is* live in YR
  (V3 uses it for the `V3WO` voxel swap when missiles depleted), just
  not enabled here.
- `;Image=SUB` — commented; historical art reuse.
- `;CruiseMissile` after the Secondary line — historical secondary name.
- No `ImmuneToVeins`, no `Subterranean`, no other TS-only fields.
- `Underwater=yes` is YR-live (used by SUB, BSUB, DLPH).

---

## Comparison with peer naval

| Field | BSUB Boomer | SUB Typhoon | DEST Destroyer | DRED Dreadnought |
|-------|-------------|-------------|----------------|-------------------|
| Strength | **1200** | 600 | 1000 | 1000 |
| Armor | heavy | heavy | heavy | heavy |
| Speed | 5 | 6 | 6 | 5 |
| Cost | 2000 | 1000 | 1000 | 2000 |
| Cloakable | yes | yes | no | no |
| Sensors | **yes** | no | yes | no |
| Underwater | yes | yes | no | no |
| Primary | BoomerTorpedo | SubTorpedo | 155mm | DreadCannon |
| Secondary | **CruiseLauncher Range=20** | none | OspreyLaunch | DMisl Spawn |
| Spawns | CMISL ×2 | none | ASW (return) | DMISL ×2 (one-shot) |
| TechLevel | 2 | 2 | 7 | 8 |

**Boomer Sub stands out:**
- **Highest naval HP** (1200 vs 600-1000 for peers).
- **Cloakable AND has Sensors** (unique combo — most cloakers can't see
  through other cloaks).
- **20-cell missile range** equals Dreadnought's, but with the
  cloak-attack advantage.
- **TechLevel=2** — earliest among comparable tier (Dread is tier-8).
- **Yuri gets one extremely strong unit at low tech** — the asymmetric
  balance of Yuri's weak overall naval is concentrated in this one unit.

---

## Cross-references

- [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
  — for the SpawnManager state machine driving CMISL.
- [ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — for CMISL's RocketLocomotion physics.
- [V3.md](../soviet/V3.md) — sibling MissileSpawn unit pattern.
- [DRED.md](../soviet/DRED.md) — sibling MissileSpawn naval unit pattern.
- [DEST.md](../allied/DEST.md) — peer Sensors+naval unit (return-to-dock
  spawn pattern instead of one-shot).
- [CARRIER.md](../allied/CARRIER.md) — peer return-to-dock spawner (no
  Sensors).
- CMISL — Cruise Missile spawn-child (the AircraftType section).

---

## Coverage audit

- [x] Every rulesmd key annotated (~55 keys).
- [x] Every artmd key annotated (5 keys including SecondSpawnOffset).
- [x] Both primary weapons documented (BoomerTorpedo + ElitePrimary
  BoomerTorpedoE) + secondary (CruiseLauncher).
- [x] Spawn child CMISL fully documented (separate AircraftType section).
- [x] All voice/sound bindings documented (8 keys + 2 fire SFX).
- [x] Prerequisites: `YAYARD, RADAR`.
- [x] Owner: YuriCountry.
- [x] Veterancy: extended VeteranAbilities (5 incl. ROF), elite swap to
  BoomerTorpedoE Burst=4.
- [x] Hardcoded behavior: dual-weapon targeting priorities, cloak+
  DecloakToFire=no, Underwater+Unnatural Squid interaction, CMISL
  one-shot spawn pattern, SecondSpawnOffset, FireAngle, Sensors+
  SensorsSight, VoiceSecondaryWeaponAttack.
- [x] TS-legacy filter applied (no active TS code; only commented
  historical fields).
- [x] Comparison table with peer naval units.
- [x] At least one Ghidra search performed (multiple — see below).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("LandTargeting")` | `0x00844520` (single match) |
| `get_xrefs_to(0x00844520)` | `0x007121a4 → TechnoTypeClass__ReadINI` |
| `search_strings("DecloakToFire")` | `0x0084951c` (single match) |
| `get_xrefs_to(0x0084951c)` | `0x00772121 → WeaponTypeClass__ReadINI` |
| `search_strings("Underwater")` | `0x00843848` (single match) |
| `get_xrefs_to(0x00843848)` | `0x00714d74 → TechnoTypeClass__ReadINI` |
| `search_strings("SecondSpawnOffset")` | `0x008431d8` (single match) |
| `get_xrefs_to(0x008431d8)` | `0x0071602e → TechnoTypeClass__ReadINI` |

**New cheat-sheet entries (4):**
- `LandTargeting` (0x00844520 → 0x007121a4) TechnoType — adjacent to
  `NavalTargeting` at 0x00844510, dual-target priority bias.
- `DecloakToFire` (0x0084951c → 0x00772121) **WeaponType** — gates
  whether firing the weapon decloaks the unit. Per-weapon, not per-unit.
- `Underwater` (0x00843848 → 0x00714d74) TechnoType — submarine render
  flag.
- `SecondSpawnOffset` (0x008431d8 → 0x0071602e) TechnoType — per-instance
  spawn position offset for Burst>1 Spawner weapons.

**Open questions:**
- CMISL's actual impact warhead isn't visible in the [CMISL] AircraftType
  section. Likely uses a Rules-global Cruise Missile warhead or the
  Spawner's `Warhead=Special` default. Worth checking when CMISL gets
  its own doc.
- `VoiceFeedback=SubFear` trigger conditions — when exactly does this
  fire? On taking damage? On enemy proximity? Open follow-up.
