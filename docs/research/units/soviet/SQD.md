---
name: sqd-doc
description: SQD — Giant Squid. Soviet tier-9 organic naval predator. Dual-weapon
  Grab/Punch with LimboLaunch SQDJUMP projectile + ParasitePlus warhead (Paralyzes +
  Culling). Cloakable; Underwater; Organic; SHP-rendered. NEW cheat-sheet entries:
  Culling/Paralyzes (WarheadType), Bombable (ObjectType), NoShadow (TechnoType).
metadata:
  type: project
---

# SQD — Giant Squid

**INI ID:** `SQD`
**Display:** "Giant Squid" (`UIName=Name:SQD`)
**Section:** `[VehicleTypes]`
**Owner side:** Soviet (Russians, Confederation, Africans, Arabs)
**Role:** Soviet tier-9 *organic* naval predator. Grapples enemy ships
(LimboLaunch SquidGrab) or punches them (SquidPunch when grabbed-mode is
inappropriate, e.g. against subs). Paralyzes grabbed targets while draining HP
via ParasitePlus warhead with Culling=yes (kills outright at low HP). Pairs
with Allied Dolphin (DLPH) as the second SHP-rendered organic naval unit in
YR. Cloak + Sensors + Underwater triple-stack like submarines.

---

## Rulesmd verbatim

```ini
[SQD]
UIName=Name:SQD
Name=Giant Squid
NoShadow=yes
Category=AFV
Prerequisite=NAYARD,NATECH
Primary=SquidGrab ; HCB + Punch
Secondary=SquidPunch ; F, F, Punch
NavalTargeting=3
LandTargeting=1
WalkRate=2 ; these two are needed because unit as sprite is terribly hack. Doing units as infantry with DoControls could be considered
IdleRate=4 ; power of two helps performance (mod).  "How much slower should I animate when stopped? 1/x"
Strength=200
SuppressionThreshold=250; damage below this amount won't suppress the parasite
Organic=yes
Armor=light
TechLevel=9
Underwater=yes
Naval=yes
Turret=no
IsTilter=no
SelfHealing=yes
CrateGoodie=no
Sight=5
Sensors=yes
SensorsSight=8 ;4
GuardRange=5
DefaultToGuardArea=yes ; SJM: Squid should move to use its two range 1 weapons like dogs and TDs
Speed=8
Owner=Russians,Confederation,Africans,Arabs
Cost=1000
Soylent=1000
Points=20
ROT=40
AllowedToStartInMultiplayer=no
Crusher=no
Crewed=no
IsSelectableCombatant=yes
VoiceSelect=SquidSelect
VoiceMove=SquidAttackCommand
VoiceAttack=SquidAttackCommand
VoiceFeedback=SquidFear
MoveSound=SquidMove
DieSound=SquidDie
Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};<-Ship
SpeedType=Float
MovementZone=Water
ThreatPosed=25	; This value MUST be 0 for all building addons
Weight=.5
ImmuneToPsionics=yes
Parasiteable=no
Trainable=yes
Explodes=no
AccelerationFactor=0.01
ZFudgeColumn=8
ZFudgeTunnel=13
Bombable=yes
Size=30
Cloakable=yes
CloakingSpeed=5 ; Slowish, low is faster
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=STRONGER,FIREPOWER,ROF
MovementRestrictedTo=Water
ElitePrimary=SquidGrabE
EliteSecondary=SquidPunchE
TooBigToFitUnderBridge=true
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:SQD` — CSF key. Resolves to "Giant Squid".
- `Name=Giant Squid` — internal description.
- `Category=AFV` — AI threat-bucket.
- `NoShadow=yes` — **does not cast a shadow**. Distinguishes the Squid
  from other naval (which have shadows). **Ghidra-verified TechnoType**
  at `0x008436e0 → 0x0071508e`. **NEW cheat-sheet entry**. Used for
  underwater creatures and certain stealth/special-effect units (CMISL
  also has `NoShadow=yes`). Likely a perf optimization + visual fit
  for "creature below water surface".

**Tech / availability**
- `Prerequisite=NAYARD,NATECH` — Soviet Naval Yard **+ Battle Lab**.
  Tier-9 lockout — late-game predator.
- `TechLevel=9` — second-highest tier (only TL10 is higher: Kirov/MCV).
- `Owner=Russians,Confederation,Africans,Arabs` — 4 Soviet sub-factions.
- `AllowedToStartInMultiplayer=no`, `CrateGoodie=no` — standard.

**Combat — defense**
- `Strength=200` — fragile (same as Dolphin). Glass-cannon predator.
- `Armor=light` — light armor.
- `SelfHealing=yes` — passive HP regen (one of the few units with
  default self-heal, joining Kirov, Yuri Prime, Mirage from a similar
  pool).

**Combat — dual-weapon Grab/Punch system**

- `Primary=SquidGrab` — **the signature attack**. *LimboLaunch=yes*
  projectile (SQDJUMP) carries the Squid itself to the target, then
  applies ParasitePlus warhead (Paralyzes + Culling). The Squid
  *becomes* its own projectile while grappling.
- `Secondary=SquidPunch` — *anti-non-grab punch attack*. Used when:
  - Target is Unnatural=yes (Boomer Sub punches instead of grabs — see
    BSUB doc).
  - Target is aircraft (impossible) or other non-grabbable.
  - Player force-fires.
  - Squid is detached from grappled target.
- `ElitePrimary=SquidGrabE` — Damage 15→40 (~2.67× damage per grab).
- `EliteSecondary=SquidPunchE` — Damage 100→200 (2× damage per punch).
  Plus `Projectile=InvisibleAll` change (vs basic InvisibleLow — `;gs !
  Dude! This gives the squid an anti-air weapon!` comment in basic).
- `NavalTargeting=3` — *low* naval priority (vs HYD 5, BSUB 7). Squid
  is choosy.
- `LandTargeting=1` — minimum (Squid is naval-only anyway).
- `WalkRate=2 / IdleRate=4` — **SHP-render animation rates**. Same SHP-
  render-vehicle architecture as Dolphin. Verbatim "sprite is terribly
  hack" comment.

**Sight / sensors / cloak**
- `Sight=5` — moderate (better than SUB=4).
- `Sensors=yes`, `SensorsSight=8` (`;4` historical) — *spots cloaked
  units at 8-cell range*. Same sensor range as Dolphin and Boomer Sub.
- `GuardRange=5` — auto-engagement range in Guard mode.
- `DefaultToGuardArea=yes` — **Ghidra-verified TechnoType** at
  `0x00843784 → 0x00714f44` (per cheat-sheet). The verbatim comment
  "Squid should move to use its two range 1 weapons like dogs and TDs"
  explains: the Squid defaults to **Guard Area** mission instead of
  Guard Stationary. Same as attack dogs and Terror Drones — units with
  short-range melee need to *move toward* threats rather than wait
  passively.
- `Cloakable=yes` — Ghidra-verified TechnoType (per SUB cheat-sheet,
  `0x00843ea8 → 0x00713f7f`).
- `CloakingSpeed=5` — *slow cloak transition* (vs SUB/BSUB/DLPH's 1).
  Verbatim comment "Slowish, low is faster" — *Squid takes 5 frames
  to fade to invisible*. Tactical impact: Squid is briefly visible
  after stopping movement, vulnerable to spot-then-shoot reaction.

**Mobility**
- `Speed=8` — fast (matches Dolphin).
- `ROT=40` — **very high** turn rate (vs SUB ROT=2). Squid pivots
  instantly. Likely because SHP-rendered 8-direction creatures can
  "snap" between facings without smooth interpolation.
- `Locomotor={2BEA74E1-...};<-Ship` — Submarine locomotor. The trailing
  `<-Ship` is a non-comment-style annotation (the parser likely treats
  everything after the GUID as comment; literal `<-Ship` is a developer
  note that Squid uses the ship-class locomotor despite being organic).
- `SpeedType=Float`.
- `MovementZone=Water`.
- `MovementRestrictedTo=Water` — *forced* water-only (UnitType per
  cheat-sheet `0x00845d64 → 0x00747837`).
- `Underwater=yes` — renders below surface (TechnoType per BSUB
  cheat-sheet).
- `Naval=yes` — naval class.
- `IsTilter=no` — *explicitly disabled*. Most naval/vehicle units have
  IsTilter=yes (slope tilt); Squid doesn't tilt because it's organic
  and animation handles its motion separately.
- `AccelerationFactor=0.01` — *extremely low acceleration ramp*. Per
  cheat-sheet TechnoType (`0x008443e0 → 0x007124bc` from CAOS). Squid
  takes a long time to reach full speed from rest. Trade-off with the
  high ROT — Squid is agile in turning but slow to accelerate.

**Weight + Size**
- `Weight=.5` — *lightest non-trivial naval weight* (vs 4 for AEGIS,
  3.5 for tanks). Squid is biologically light (mostly water).
- `Size=30` — *largest Size value seen so far*. Cannot fit in any
  transport.

**Immunities + interactions**
- `ImmuneToPsionics=yes` — *cannot be mind-controlled*. Surprising for
  an organic — most organics (Dolphin included) lack this. The Squid
  is specifically Yuri-resistant. **Ghidra-verified TechnoType
  `0x00843754 → 0x00714fa7`** (cheat-sheet).
- `Parasiteable=no` — Terror Drones can't attach to Squid.
- `Organic=yes` — living creature flag (TechnoType per DLPH cheat-sheet
  `0x00843714 → 0x0071502b`).
- `Trainable=yes` — gains veterancy. Most naval is Trainable=yes;
  DLPH oddly inherits this; Squid also.
- `Explodes=no` — *does not explode on death*. TechnoType per
  cheat-sheet (`0x0083355c → 0x007122c5`). The Squid sinks rather than
  detonates.
- `Bombable=yes` — **Crazy Ivan can plant a bomb on Squid**.
  **Ghidra-verified ObjectTypeClass__ReadINI** at
  `0x00832bcc → 0x005f9420`. **NEW cheat-sheet entry — BROADER scope**.
  Same pattern as `NoSpawnAlt` and `RadarInvisible` (ObjectType-level,
  not TechnoType). Means `Bombable` is read for *any* ObjectType (could
  apply to terrain, anims, etc., though only used by units in practice).
  - **Strategic note**: A Crazy Ivan with a Hovercraft / SHAD ferry
    could plant a bomb on a Squid. Unusual interaction.

**SuppressionThreshold**
- `SuppressionThreshold=250` — verbatim comment: "damage below this
  amount won't suppress the parasite". Per cheat-sheet TechnoType
  (`0x008436ec → 0x0071506d`). When the *grabbed victim* takes damage,
  if any single hit deals < 250 damage, the parasite (the Squid)
  isn't dislodged. Only hits dealing ≥250 damage break the grapple.

**Crew / death**
- `Crewed=no`, `Crusher=no`, `Turret=no`.
- `IsSelectableCombatant=yes`.

**Voice / sound bindings**
- `VoiceSelect=SquidSelect` (single-sample `vsqusela`).
- `VoiceMove=SquidAttackCommand` — *same as attack* (2-sample pool).
  Squid attacks and moves are conceptually the same action (it swims
  toward target).
- `VoiceAttack=SquidAttackCommand`.
- `VoiceFeedback=SquidFear` — **silent block** (Volume=0). Same
  convention as SUB/BSUB/DLPH `*Fear` blocks.
- `MoveSound=SquidMove` (random 2-sample). **Note**: This is a *random*
  pool, not an ignition-only pool like other naval. Squid plays
  swimming sounds during continuous movement.
- `DieSound=SquidDie` — death SFX (`vsqudiea`).
- *No `EnterTransportSound`/`LeaveTransportSound`* — Squid can't
  enter transports (Size=30 too big anyway).

**Combat behavior**
- `ThreatPosed=25` — moderate AI threat.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — 5 abilities.
- `EliteAbilities=STRONGER,FIREPOWER,ROF` — **NO SELF_HEAL** (since
  SelfHealing=yes is on by default at rookie, the elite SELF_HEAL
  ability would be redundant; explicitly omitted).

**Z-fudge**
- `ZFudgeColumn=8` — Z-sort offset for cliff columns.
- `ZFudgeTunnel=13` — TS-legacy dormant.

**TooBigToFitUnderBridge=true** — UnitType per cheat-sheet.

---

## Artmd verbatim

```ini
[SQD] ; Squid
Voxel=no
Remapable=yes
Cameo=SQDICON
WalkFrames=20
FiringFrames=16
```

### Key-by-key annotation

- `Voxel=no` — **SHP-rendered sprite** (same as DLPH). Voxel=no
  vehicles use the WalkFrames/FiringFrames animation system.
- `Remapable=yes` — house-color remap on remap-channel pixels.
- `Cameo=SQDICON` — sidebar build button.
- `WalkFrames=20` — *20 frames per direction* (vs DLPH's 6). The
  Squid animation is more elaborate — 20 frames per 8 directions =
  160 walk SHP frames.
- `FiringFrames=16` — 16 firing frames per direction = 128 firing
  frames.

**Total SHP frame count**: ~288 frames at minimum (walk + firing) +
idle + death + grab animation. Substantial sprite art.

**No `PrimaryFireFLH=`, no `SecondaryFireFLH=`** — the SquidGrab
projectile is LimboLaunch=yes (Squid becomes the projectile), so FLH
is irrelevant. SquidPunch is a melee-range attack — no FLH offset.

---

## Weapons

### Primary — `[SquidGrab]` (LimboLaunch grapple)

```ini
[SquidGrab]
Damage=15
ROF=99	; SJM: This is now ignored.  Damage frequency handled by special Squid code.
Range=1.5
CellRangefinding=yes
Projectile=SQDJUMP
Speed=30
DecloakToFire=no
Warhead=ParasitePlus ; Plus paralysis
LimboLaunch=yes ; Limbo shooter at launch (one shot or become the bullet)
Report=SquidAttack
Anim=SQDG_N,SQDG_NE,SQDG_E,SQDG_SE,SQDG_S,SQDG_SW,SQDG_W,SQDG_NW ; order matches 0=N & CW
```

- `Damage=15` — per-tick grab damage.
- `ROF=99` — **the verbatim comment is critical**: *"This is now
  ignored. Damage frequency handled by special Squid code."* The Squid
  grapple has a *special hardcoded damage tick rate* that overrides the
  weapon's ROF. The 99 value is vestigial.
- `Range=1.5` — short-range grapple (must be next to target).
- `CellRangefinding=yes` — range computed cell-to-cell.
- `Projectile=SQDJUMP` — the LimboLaunch carrier projectile.
- `Speed=30` — projectile speed.
- `DecloakToFire=no` — *fires while cloaked* (per cheat-sheet
  WeaponType `0x0084951c → 0x00772121`). The Squid stays cloaked
  during grapple — enemies see only the grabbed ship being damaged.
- `Warhead=ParasitePlus` — see warhead block.
- `LimboLaunch=yes` — **the signature mechanic**. **Ghidra-verified
  WeaponType cheat-sheet** at `0x0084952c → 0x00772107`. The Squid
  *removes itself from the world* on launch, *becomes* the SQDJUMP
  projectile, and is *re-added* attached to the target on impact. Same
  pattern as DRON Terror Drone (DroneJump).
  - Verbatim comment: *"Limbo shooter at launch (one shot or become
    the bullet)"*.
- `Report=SquidAttack` — fire SFX (`vsquat1a`).
- `Anim=SQDG_N,SQDG_NE,SQDG_E,SQDG_SE,SQDG_S,SQDG_SW,SQDG_W,SQDG_NW` —
  *8-direction grab animation*. The engine picks the appropriate SQDG_*
  anim based on facing. Same pattern as SHAD's MGUN-* anims.

### Elite primary — `[SquidGrabE]`

```ini
[SquidGrabE]
Damage=40 (vs 15 basic — ~2.67× damage per grab tick)
ROF=99 (same — ignored)
Range=1.5 (same)
... (otherwise identical to basic)
```

Same mechanics, just higher per-tick damage. Elite Squid drains
grabbed ships ~2.7× faster.

### Secondary — `[SquidPunch]` (anti-non-grab)

```ini
[SquidPunch]
Damage=100;50
ROF=32 ;hard 16*2 until unit sprites get more complicated
Range=1.83
Projectile=InvisibleLow ;gs ! Dude!  This gives the squid an anti-air weapon! --> was InvisibleAll
Speed=30
Warhead=HE
;AntiOrganic=yes
Report=SquidAttackNonShip
;IsSonic=Yes
```

- `Damage=100` (`;50` historical commented — doubled).
- `ROF=32` — verbatim comment: *"hard 16*2 until unit sprites get more
  complicated"*. The ROF is tied to the SHP animation cycle — 16 frames
  per punch × 2 cycles = 32. Architectural quirk of SHP-vehicle.
- `Range=1.83` — slightly longer than grab (1.5).
- `Projectile=InvisibleLow` — the verbatim Greg-Smith comment is
  important: *"gs ! Dude! This gives the squid an anti-air weapon!
  --> was InvisibleAll"*. The original `InvisibleAll` projectile would
  have allowed the Squid to punch *aircraft*; Westwood changed to
  `InvisibleLow` to restrict to surface targets only.
- `Warhead=HE` — standard High-Explosive (not the Parasite warhead).
- `;AntiOrganic=yes` — commented historical anti-organic flag.
- `;IsSonic=Yes` — commented (was once going to be a sonic punch?).
- `Report=SquidAttackNonShip` — *distinct fire SFX* from the grab
  (though both share `vsquat1a` sample — `SquidAttack` and
  `SquidAttackNonShip` both reference the same sound).

### Elite secondary — `[SquidPunchE]`

```ini
[SquidPunchE]
Damage=200 (2× basic)
ROF=32 (same)
Range=1.83 (same)
Projectile=InvisibleAll (vs basic InvisibleLow — restored AA capability!)
Speed=30
Warhead=HE
...
```

**Notable: elite Squid Punch uses `InvisibleAll` projectile** — this
*restores the anti-air capability* that the Greg-Smith comment said
was deliberately removed at basic rank. So **elite Squids can punch
aircraft** but basic Squids cannot. Unusual elite-side capability
gain.

### Projectile — `[SQDJUMP]`

```ini
[SQDJUMP]
Inviso=yes ;### temp
Image=none ;### temp
AA=no
Arm=2
ROT=8 ;requires to use Rotates
Shadow=no
Proximity=yes
Ranged=yes
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=yes
```

- `Inviso=yes`, `Image=none` — **the verbatim `;### temp` comments
  suggest placeholder values that never got replaced**. The SQDJUMP
  projectile is invisible — only the Squid disappearing-and-reappearing
  visual happens. The verbatim `;Image=SQDP ;Hmm...Requires an Image
  entry to get at Rotates=. Violates the same name default rule`
  commented block hints at a Westwood engine quirk where
  `Rotates=` flag requires an explicit Image= entry, even if it would
  default to the same name.
- `AA=no` — anti-air disabled (consistent with the basic Squid Punch
  restriction).
- `Arm=2` — 2-frame arming delay.
- `ROT=8` — *needs Rotates flag to work*.
- `Proximity=yes` — detonates near target (proximity-fused).
- `Ranged=yes` — fuse-based range check.

### Warhead — `[ParasitePlus]`

```ini
[ParasitePlus]; SquidGrab
Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%
Parasite=yes
Culling=yes ; kills instead of damages if victim in Red
Paralyzes=32767 ; SJM: Last a long time.  Will be reset in code to last as long as Squid grapples.
InfDeath=1
Rocker=yes
```

- `Verses=100%,...,100%` — *all-100%*. The grapple doesn't care about
  armor type — it deals full damage to whatever it grabs.
- `Parasite=yes` — **enables the parasite-attach state machine**. Per
  cheat-sheet WarheadType (`0x0081717c → 0x0075d83b`, from DRON doc).
  Same flag as DroneJump (Terror Drone) — both share the
  ParasiteClass attach behavior.
- `Culling=yes` — **Ghidra-verified WarheadType** at
  `0x00847d10 → 0x0075d938`. **NEW cheat-sheet entry**. Verbatim:
  *"kills instead of damages if victim in Red"*. When the grabbed
  target's HP drops into the Red zone (typically ≤25% HP per RA2/YR
  conventions), the next damage tick *kills* outright instead of just
  reducing HP. **Critical detail**: Squid grapple has *built-in
  execution* — once you're at Red, you're dead next tick regardless
  of how much damage the next hit would normally deal.
- `Paralyzes=32767` — **Ghidra-verified WarheadType** at
  `0x00847d18 → 0x0075d922`. **NEW cheat-sheet entry**. The grabbed
  target *cannot move or fire* for the duration. Verbatim comment:
  *"Last a long time. Will be reset in code to last as long as Squid
  grapples."* The 32767 value is a max-int placeholder — the engine's
  special Squid code resets the duration each tick the grapple
  remains active. **Net effect: grabbed targets are paralyzed for the
  entire grapple duration, then released (alive or dead)**.
- `InfDeath=1` — small-arms infantry death.
- `Rocker=yes` — vehicles rock during the grab.

### HE Warhead (for SquidPunch)

The `[HE]` warhead is a standard High-Explosive used by many weapons.
Not unique to Squid. See standard warhead reference (general anti-
vehicle/anti-structure profile).

---

## Voices / sounds

```ini
[SquidSelect]
Sounds= vsqusela
FShift= -10 10

[SquidAttackCommand]
Sounds= vsqumova vsqumovb
Control= random
FShift= -10 10

[SquidMove]
Sounds= vsqumova vsqumovb
FShift= -10 10
Control= random
Volume=35

[SquidAttack]
Sounds=vsquat1a
FShift= -10 10

[SquidAttackNonShip]
Sounds=vsquat1a
FShift= -10 10

[SquidDie]
Sounds=vsqudiea
FShift= -10 10

[SquidFear]
Volume=0	; no sound
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=SquidSelect` | `[SquidSelect]` | Click (single sample) |
| `VoiceMove=SquidAttackCommand` | `[SquidAttackCommand]` | Move order (**same as attack command** — 2-sample pool) |
| `VoiceAttack=SquidAttackCommand` | `[SquidAttackCommand]` | Attack order |
| `VoiceFeedback=SquidFear` | `[SquidFear]` | **silent block** (Volume=0) |
| `MoveSound=SquidMove` | `[SquidMove]` | Continuous swimming SFX (2-sample random, Volume=35 low) |
| `Report=SquidAttack` (SquidGrab) | `[SquidAttack]` | Grapple SFX |
| `Report=SquidAttackNonShip` (SquidPunch) | `[SquidAttackNonShip]` | Punch SFX (same sample as grab, different name) |
| `DieSound=SquidDie` | `[SquidDie]` | Death |

**Notable**: SAPC/SUB/DLPH use `MoveSound=*MoveStart` (one-shot
ignition). **Squid uses `MoveSound=SquidMove`** — *random-pool not
predelayed*. The `[SquidMove]` block has `Control=random` (continuous
random pool), Volume=35 (quiet), perfect for an ambient swimming sound
loop during movement.

**Voice doubling**: `VoiceMove = VoiceAttack = SquidAttackCommand`.
The Squid only has one "command response" voice category — moving
toward target and attacking are conceptually unified (it swims to grab).

**`vsqumova/b` is shared**: the same 2-sample pool plays for *both*
Squid movement orders (loud) *and* SUB ignition (the SubMoveStart
also uses these samples, see SUB doc). **Westwood audio cross-reuse**:
the Soviet sub's "underwater rumble" engine sound and the Giant
Squid's "tentacle swirl" creature sound are the same audio file.
Acoustic confusion is part of the gameplay — players can't easily
distinguish sub from squid by sound.

---

## Hardcoded behavior (Ghidra-verified)

### 1. LimboLaunch SQDJUMP grapple state machine

Verified WeaponType flag (per cheat-sheet `0x0084952c → 0x00772107`).
The Squid's `Primary=SquidGrab` is LimboLaunch=yes, triggering:
1. Squid disappears from world (limbo state) at fire moment.
2. SQDJUMP projectile travels to target (visible-less, since Image=none).
3. On impact: Squid re-spawns *attached to target* (parasite state).
4. ParasitePlus warhead applies per-tick damage + Paralyzes for
   indefinite duration (32767 → engine-overridden).
5. Culling=yes: when target reaches Red HP, next tick kills outright.
6. On target death OR grapple-break (≥250 damage hit per
   SuppressionThreshold): Squid detaches and re-enters world.

Same state machine as DRON Terror Drone (see
[PARASITE_CLASS_GHIDRA_REPORT.md](../../PARASITE_CLASS_GHIDRA_REPORT.md)
for full ParasiteClass details). The Squid is the *naval Terror
Drone equivalent* — same mechanism, water-bound, much larger Size.

### 2. Special Squid damage code (overrides ROF)

The verbatim Westwood comment on `ROF=99 ; SJM: This is now ignored.
Damage frequency handled by special Squid code.` reveals:
- The Squid grapple uses a **hardcoded per-tick damage rate**, not
  the weapon's ROF.
- The damage tick frequency is unknown from INI alone — would
  require Ghidra trace into the Squid-specific code path.
- Open question: what's the actual damage tick rate? Probably tied
  to the parasite-state update loop. Worth investigating in a
  dedicated Squid Ghidra report.

### 3. Culling + Paralyzes WarheadType flags

Both **NEW cheat-sheet entries**:
- `Culling` (WarheadType `0x00847d10 → 0x0075d938`) — kills outright
  in Red HP zone instead of damaging. Without this flag, a victim
  could drift at 1 HP forever (low-damage drips never finishing the
  job). With Culling, the engine recognizes the "execution" moment
  and finalizes the kill.
- `Paralyzes` (WarheadType `0x00847d18 → 0x0075d922`) — disables
  victim's ability to move and fire. Value is duration in frames
  (32767 = effectively forever; engine overrides per Squid code).

### 4. Bombable=yes at ObjectType scope

**NEW BROADER-SCOPE entry**: `Bombable` is read at
`ObjectTypeClass__ReadINI` (`0x00832bcc → 0x005f9420`), not at
TechnoType. This is the **4th known ObjectType-scope field**:
- NoSpawnAlt (V3/DRED swap voxel)
- RadarInvisible (SHAD stealth)
- Bombable (Crazy Ivan target eligibility) — **NEW**
- Plus one more from ObjectType range to be discovered.

ObjectType is the parent of TechnoType in the class hierarchy.
Reading at ObjectType means *any* ObjectType subclass can have the
field — Building, Vehicle, Infantry, Aircraft, Terrain, even
Anim/Smudge. In practice the Bombable flag controls Ivan's bomb-
plant eligibility on the target.

**Strategic implication**: A bomb-planted Squid is unusual (Ivan
can't reach water), but cross-faction interactions (a captured Yuri
+ Bombard-via-SHAD ferry scenario) could exploit this.

### 5. NoShadow=yes TechnoType

**NEW cheat-sheet entry**: `NoShadow` (TechnoType `0x008436e0 →
0x0071508e`). Disables the unit's projected shadow rendering. Used
by underwater/aerial-special units:
- SQD Squid
- CMISL Cruise Missile (aircraft spawn)
- *Probably* most spawn-aircraft (V3ROCKET, DMISL).

Performance-and-aesthetics flag — underwater creatures shouldn't
cast a shadow on the water surface (no rendering pipeline for that).

### 6. DefaultToGuardArea=yes

TechnoType per cheat-sheet (`0x00843784 → 0x00714f44`). The Squid
defaults to *Guard Area* mission (move-to-engage in radius) instead
of *Guard Stationary* (wait-and-fire). Same as DOG, ADOG, YDOG, TDOG
(Terror Drone), and other melee/grapple-range units that need to
close on targets.

### 7. SHP-rendered organic with WalkRate/IdleRate

Same architectural quirk as Dolphin (verbatim Westwood comment about
"sprite is terribly hack" — they would have preferred infantry-class
modeling). The Squid uses 8-direction SHP frames with WalkRate=2,
IdleRate=4. The grapple animation `SQDG_N..NW` 8-direction set is
selected based on facing during attack.

### 8. Cloak triple-stack (Cloak + Underwater + Sensors)

Same flag combo as SUB, BSUB, DLPH. The Squid is invisible to non-
sensor-equipped enemies + lives underwater + can detect other
cloaked units. Stealth predator.

**However**: `CloakingSpeed=5` (slow vs SUB/BSUB/DLPH=1) means the
Squid takes 5 frames to re-cloak after surfacing/firing. Tactical
exposure window — a Destroyer with Sensors can spot the Squid during
this transition and engage before recloak.

---

## TS-legacy filter

- `;Image=SQDP ;Hmm...Requires an Image entry to get at Rotates=.
  Violates the same name default rule` — verbatim Westwood
  engine-quirk note. Disabled.
- `;### temp` placeholders on SQDJUMP — never replaced.
- `;AntiOrganic=yes` on SquidPunch — commented historical (Squid
  could once punch through organic armor specifically).
- `;IsSonic=Yes` on SquidPunch — commented.
- `;gs ! Dude! This gives the squid an anti-air weapon!` — Greg-Smith
  notice that switching InvisibleAll → InvisibleLow removed anti-air
  capability (which gets restored at elite via SquidPunchE).
- `Locomotor=...;<-Ship` — `<-Ship` literal text after the `;` comment
  marker is a developer note ("uses Ship locomotor").
- `Verses=100%,...,100%` historical — `;100%,80%,70%,50%,30%,30%,30%,
  20%,5%,100%,100%` would have been a tiered version. Disabled in
  favor of all-100%.
- `;2/3` historical-formula notes on warhead lines.
- No `ImmuneToVeins`, no `Subterranean`. **YR-active core
  mechanism.**

---

## Comparison: organic naval predators (Squid vs Dolphin)

| Field | SQD Giant Squid (Soviet) | DLPH Dolphin (Allied) |
|-------|---------------------------|------------------------|
| Strength | 200 | 200 |
| Armor | light | light |
| Speed | 8 | 8 |
| Cost | 1000 | 500 |
| TechLevel | **9** | 5 |
| Prereq | NAYARD,NATECH | GAYARD,GATECH |
| Primary | **SquidGrab (LimboLaunch parasite)** | **SonicZap (chain damage)** |
| Damage profile | grab+drain over time | direct chain pulse |
| Range | 1.5 (close-range grapple) | 6 (mid-range) |
| Sensors+SensorsSight | yes, 8 | yes, 8 |
| Cloakable+CloakingSpeed | yes, **5 (slow)** | yes, **1 (fast)** |
| Organic | yes | yes |
| Underwater | yes | yes |
| SelfHealing | yes | not set |
| Trainable | yes | yes |
| Sprite rendering | yes (Voxel=no) | yes (Voxel=no) |
| WalkFrames | **20** | 6 |
| FiringFrames | **16** | 6 |
| TypeImmune | not set | yes |
| SuppressionThreshold | 250 | not set |
| AccelerationFactor | 0.01 | not set |

**Trade-offs:**
- **SQD**: Higher tech tier, more expensive. Slow cloak. Massive SHP
  art assets. *Specialist* ship-killer via grapple+execute mechanic
  with built-in Culling=yes kill-shot.
- **DLPH**: Lower tech tier, cheaper. Fast cloak. Smaller SHP assets.
  *Generalist* anti-sub chain weapon.

**Squid is a true "hero unit" tier-9 finisher**: against an enemy
ship in Red HP, the Squid grapples and executes. Cannot stand off
and skirmish — must close to 1.5 cells. Dolphin can pick at clusters
from 6 cells but can't reliably execute.

**Asymmetric ship-kill capability**: Squid kills ships *deterministic*
once grappled (Paralyzes + drain + Cull). Dolphin must continue
firing the same target until HP=0. Squid is more effective per
engagement; Dolphin is more flexible across engagements.

---

## Cross-references

- [DLPH.md](../allied/DLPH.md) — Allied Dolphin (organic naval pair).
- [DRON.md](../soviet/DRON.md) — sibling ParasiteClass unit (Terror
  Drone). Shares LimboLaunch+ParasitePlus+attach state machine.
- [PARASITE_CLASS_GHIDRA_REPORT.md](../../PARASITE_CLASS_GHIDRA_REPORT.md)
  — ParasiteClass attach mechanics (shared with DRON/SQD).
- [BSUB.md](../yuri/BSUB.md) — Boomer Sub, `Unnatural=yes` (Squid
  punches instead of grabbing it).
- [SUB.md](../soviet/SUB.md) — Typhoon Sub (no Unnatural — Squid
  grabs SUB normally).

---

## Coverage audit

- [x] Every rulesmd key annotated (~60 keys including SuppressionThreshold,
  AccelerationFactor, DefaultToGuardArea, NoShadow, Bombable).
- [x] Every artmd key annotated (5 keys).
- [x] Both weapons documented (SquidGrab basic + Elite, SquidPunch
  basic + Elite — note elite punch restores AA capability).
- [x] SQDJUMP projectile documented (Inviso, AA=no, Proximity).
- [x] ParasitePlus warhead documented (Culling + Paralyzes mechanics).
- [x] All voice/sound bindings documented including silent
  `[SquidFear]` and Squid-shared `vsqumov*` audio.
- [x] Prerequisites: `NAYARD, NATECH`.
- [x] Owner: 4 Soviet sub-factions.
- [x] Veterancy: extended VeteranAbilities (5 incl. ROF), elite swap
  on both weapons; no SELF_HEAL (already has default SelfHealing).
- [x] Hardcoded behavior: LimboLaunch grapple, special Squid damage
  code (ROF override), Culling + Paralyzes warhead mechanics,
  Bombable ObjectType scope (NEW), NoShadow TechnoType (NEW),
  DefaultToGuardArea, Cloak triple-stack (with slower CloakingSpeed=5),
  SHP-render organic.
- [x] TS-legacy filter applied (multiple `;### temp` placeholders +
  Greg-Smith comments).
- [x] Comparison table closes the organic-naval pair (Squid vs Dolphin).
- [x] At least one Ghidra search performed (4 strings + xrefs, 4
  new cheat-sheet entries).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("Culling")` | `0x00847d10` (single match) |
| `get_xrefs_to(0x00847d10)` | `0x0075d938 → WarheadTypeClass__ReadINI` |
| `search_strings("Paralyzes")` | `0x00847d18` (single match) |
| `get_xrefs_to(0x00847d18)` | `0x0075d922 → WarheadTypeClass__ReadINI` |
| `search_strings("^Bombable$")` | `0x00832bcc` (single match) |
| `get_xrefs_to(0x00832bcc)` | `0x005f9420 → ObjectTypeClass__ReadINI` **(BROADER SCOPE)** |
| `search_strings("NoShadow")` | `0x008436e0` (single match) |
| `get_xrefs_to(0x008436e0)` | `0x0071508e → TechnoTypeClass__ReadINI` |

**New cheat-sheet entries (4):**
- `Culling` (0x00847d10 → 0x0075d938) **WarheadType** — kill outright
  if victim in Red HP.
- `Paralyzes` (0x00847d18 → 0x0075d922) **WarheadType** — disable
  target movement/fire for N frames.
- `Bombable` (0x00832bcc → 0x005f9420) **ObjectType** — Ivan bomb
  target eligibility. **4th known ObjectType-scope field** (joining
  NoSpawnAlt, RadarInvisible, NoShadow).
- `NoShadow` (0x008436e0 → 0x0071508e) **TechnoType** — disable
  shadow rendering.

**Re-confirmed:**
- `LimboLaunch` (WeaponType, per MIND/CCOMAND cheat-sheet).
- `Parasite` (WarheadType `0x0081717c → 0x0075d83b`, per DRON).
- `Cloakable` + `CloakingSpeed` (TechnoType, per SUB).
- `DefaultToGuardArea` (TechnoType, per cheat-sheet extended notes).
- `SuppressionThreshold` (TechnoType, per SMIN).
- `AccelerationFactor` (TechnoType, per CAOS).

**Organic naval pair closed**: SQD ✓ + DLPH ✓.

**Open questions:**
- Squid grapple's actual per-tick damage rate (the verbatim "handled
  by special Squid code" comment hides the value). Open follow-up for
  a dedicated Ghidra trace into Squid-grapple code path.
- The `<-Ship` annotation in `Locomotor=...;<-Ship` line — is the
  Locomotor parser robust to arbitrary trailing text after `;`?
  Likely yes (treats as comment). Open verification.
- Elite SquidPunchE's `InvisibleAll` projectile-swap *re-enabling*
  anti-air — is this intentional Westwood design (rewarding elite
  Squids with AA capability) or an oversight (forgot to update
  ProjectileE to InvisibleLow)? Open game-balance question.
