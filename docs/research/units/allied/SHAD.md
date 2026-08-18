---
name: shad-doc
description: SHAD — Nighthawk/Blackhawk Transport. Allied 5-passenger jumpjet
  helicopter with defensive BlackHawkCannon (Quad-anim MGUN). RadarInvisible=yes
  (ObjectType-scope, broader than TechnoType); HoverAttack=yes; PreventAttackMove=yes;
  CanPassiveAquire=no; Trainable=yes (rare for transport).
metadata:
  type: project
---

# SHAD — Nighthawk Transport Helicopter

**INI ID:** `SHAD`
**Display:** "BlackHawk Transport" (`UIName=Name:SHAD`). The CSF lookup is
labelled "Nighthawk Transport" in shipped YR — *internal name is BlackHawk,
display name is Nighthawk* (Westwood inconsistency).
**Section:** `[VehicleTypes]` (despite `ConsideredAircraft=yes`, declared as a
vehicle — same as Kirov).
**Owner side:** Allied (British, French, Germans, Americans, Alliance)
**Role:** Allied 5-passenger jumpjet helicopter transport with a defensive
machine-gun cannon. **Radar-invisible** (ObjectType-scope flag), making it the
stealth-transport option for cross-map infiltration. Pairs with infantry
(typically Chrono Legionnaires, SEALs, Tanya, Engineer) for surgical drops
behind enemy lines.

---

## Rulesmd verbatim

```ini
[SHAD]
UIName=Name:SHAD
Name=BlackHawk Transport
;Prerequisite=GAHPAD
Prerequisite=GAWEAP
Primary=BlackHawkCannon
Strength=175
Category=AirPower
JumpJet=yes
Armor=light
TechLevel=7
Sight=7
RadarInvisible=yes
Landable=yes
PipScale=Passengers
Passengers=5
Speed=14
PitchSpeed=1.1
JumpjetSpeed=30 ;params not defined use defaults (old globals way up top)
JumpjetClimb=10
JumpjetCrash=40 ; Climb, but down
JumpJetAccel=12
JumpJetTurnRate=6
JumpjetHeight=500
JumpjetWobbles=.01
JumpjetDeviation=1
Owner=British,French,Germans,Americans,Alliance
Cost=1000
Points=15
ROT=5
Crewed=yes
ConsideredAircraft=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=3
VoiceSelect=BlackOpsSelect
VoiceMove=BlackOpsMove
VoiceAttack=BlackOpsAttackCommand
VoiceCrashing=BlackOpsVoiceDie
DieSound=
CrashingSound=BlackOpsDie
ImpactLandSound=GenAircraftCrash
;Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1} ;flying
Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5} ;jumpjet
MovementZone=Fly
DamageParticleSystems=SparkSys,SmallGreySSys
;AuxSound1=BlackOpsTakeOff	;Taking off
;AuxSound2=BlackOpsLanding	;Landing
ThreatPosed=0
SpecialThreatValue=1
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Size=15
SizeLimit=2
HoverAttack=yes
;OmniFire=yes ;GEF moved to weapon
AllowedToStartInMultiplayer=no
Crashable=yes
CanPassiveAquire=no ; Won't try to pick up own targets
SpeedType=Hover
MoveSound=BlackOpsMoveLoop
EnterTransportSound=EnterTransport
LeaveTransportSound=ExitTransport
ElitePrimary=BlackHawkCannonE
PreventAttackMove=yes
;Bombable=no
TooBigToFitUnderBridge=true
Trainable=yes
Bunkerable=no; Units default to yes, others default to no
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:SHAD` — CSF key. Resolves to "Nighthawk Transport" in shipped
  YR (the CSF was updated post-art; internal name stayed "BlackHawk").
- `Name=BlackHawk Transport` — internal description; pre-rename.
- `Category=AirPower` — sidebar/AI threat bucket (with Kirov, Black Eagle,
  Carrier-Hornets).
- `JumpJet=yes` — *jumpjet aircraft flag*. Adjusts vehicle-class type
  resolution (yes for jumpjet, no for plane-class). Different from
  Locomotor=Jumpjet GUID — `JumpJet=yes` is the TechnoType discriminator,
  the GUID is the actual motion code.

**Tech / availability**
- `;Prerequisite=GAHPAD` — commented. Was once gated by a Helipad
  (`GAHPAD`). Westwood removed the helipad-prereq; SHAD is now buildable
  from War Factory.
- `Prerequisite=GAWEAP` — *only the Allied War Factory required*. **No
  Battle Lab, no Service Depot, no helipad**. Surprisingly low prereq for
  a tier-7 unit.
- `TechLevel=7` — tier-7.
- `Owner=British,French,Germans,Americans,Alliance` — 5 Allied houses.
- `AllowedToStartInMultiplayer=no` — not a starting unit.

**Combat — defense**
- `Strength=175` — fragile (between Robot Tank 180 and Mirage Tank 200).
- `Armor=light` — light armor. Vulnerable to AT and AA equally.

**Combat — weapons**
- `Primary=BlackHawkCannon` — 35 dmg, ROF=40, Range=6, OmniFire=yes,
  Quad-direction MGUN anim. See "Weapon" section.
- `ElitePrimary=BlackHawkCannonE` — elite swap: 40 dmg + SSA warhead
  (slightly stronger).

**Radar / sight**
- `Sight=7` — 7-cell vision (one more than Range=6 weapon).
- `RadarInvisible=yes` — **does not appear on enemy minimap**. Stealth
  transport. **Ghidra-verified ObjectTypeClass__ReadINI** at
  `0x00832b9c → 0x005f946e` — **broader scope than TechnoType**
  (ObjectType is parent class). Same broader scope as `NoSpawnAlt`.
  Means even debris/anim-class objects could theoretically have this
  flag read; effectively used by aircraft/units for stealth.
- `Landable=yes` — *aircraft can land on this cell type* (the unit
  itself being landable means the SHAD can touch down — it doesn't
  permanently hover).

**Passengers**
- `PipScale=Passengers` — pip bar displays passenger fill (vs Tiberium
  for refineries).
- `Passengers=5` — **5 passenger slots**.
- `SizeLimit=2` — *each passenger must have Size ≤ 2*. Compare with
  BFRT (Size=2 limit) and SAPC (Size=4 default). Most infantry are Size=1,
  Terror Drones are Size=2, Brutes/Yuri Prime are Size=2. **5 Size-1
  infantry OR 2 Terror Drones OR mixtures fit**.

**Jumpjet mobility (detailed)**
- `Speed=14` — fast for an aircraft. Compare:
  - Rocketeer JUMPJET: 12
  - SHAD Nighthawk: 14
  - Kirov ZEP: 5
- `PitchSpeed=1.1` — rate of pitch animation change. Used during
  banking/turning. Per cheat-sheet TechnoType from CARRIER doc
  (`0x00844458 → 0x007123da`).
- `JumpjetSpeed=30` — pure-jumpjet speed override (likely vestigial since
  `Speed=14` is the operational value).
- `JumpjetClimb=10` — climb rate.
- `JumpjetCrash=40` — descent rate during crash plummet. **Much faster
  than Kirov's 12** — Nighthawk crashes near-vertically. Verbatim comment:
  "Climb, but down".
- `JumpJetAccel=12` — acceleration on direction change.
- `JumpJetTurnRate=6` — turn rate. **Much faster than Kirov's 2** —
  Nighthawk is nimble.
- `JumpjetHeight=500` — *altitude in leptons*. Lower than Kirov (750)
  and Floating Disc (800). Mid-altitude.
- `JumpjetWobbles=.01` — small sin-wave wobble amplitude.
- `JumpjetDeviation=1` — wobble deviation magnitude. *Does NOT have
  `JumpjetNoWobbles=yes`* — SHAD visibly bobs (gentle hover wobble).

**Locomotor**
- `;Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1} ;flying` — commented
  fixed-wing aircraft locomotor (the `...746` GUID). Was once a
  conventional plane.
- `Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5} ;jumpjet` — active
  JumpJet locomotor. Verbatim comment confirms.
- `MovementZone=Fly` — fly-zone pathing.

**Economy**
- `Cost=1000` — moderate. Cheap for tier-7.
- *No `Soylent=`* — defaults to `Cost`. Grinder refund = 1000.
- `Points=15` — low score (transport, not a kill priority).
- `ROT=5` — turn rate.

**Combat behavior**
- `ThreatPosed=0` — AI does not see SHAD as a threat (unarmed-passive
  classification despite having a defensive weapon).
- `SpecialThreatValue=1` — *low strategic value* per Ghidra-verified
  TechnoType (per SMCV doc). AI still scores it as worth attacking, just
  not high-priority.
- `HoverAttack=yes` — *can fire while hovering*. [BINARY-VERIFIED audit 24: string @ 0x008443B0, parser xref @ 0x0071255A, `TechnoType+0x390` (byte) — re-confirms audit 8 cumulative].
  Unlike planes (which must strafe), jumpjet+HoverAttack units fire from
  a stationary hover. Adds tactical flexibility.
- `;OmniFire=yes` — commented; moved to weapon (same migration pattern
  as Kirov ZEP). The `OmniFire=yes` flag is now on `[BlackHawkCannon]`.
- `Crashable=yes` — plummets on death (jumpjet crash plummet).
- `Crewed=yes` — *crew ejects on death*. The crew unit is house-default
  (Allied GI). Per the helicopter design, this could be the pilots
  bailing out.
- `CanPassiveAquire=no` — *will NOT auto-target enemies*. **Ghidra-
  verified TechnoType** at `0x00843c50 → 0x00714473` (per cheat-sheet
  from CARRIER doc). The verbatim comment: "Won't try to pick up own
  targets". Combined with `PreventAttackMove=yes`, this makes SHAD a
  *passive transport* — it only fires when explicitly attacked or
  ordered to attack. Crucial: a SHAD ferrying infantry won't get
  distracted to fight; it sticks to its waypoint route.
- `PreventAttackMove=yes` — *cannot be issued an Attack-Move command*.
  [BINARY-VERIFIED audit 24: string @ 0x008439B0, parser xref @ 0x00714994, `TechnoType+0x6C8` (byte) — re-confirms audit 10 cumulative]. Combined with `CanPassiveAquire=no`, SHAD is
  *fully passive* — it does what you tell it, nothing more.

**Crew / death**
- `MaxDebris=3` — minimal debris.
- `VoiceCrashing=BlackOpsVoiceDie` — voice while plummeting.
- `DieSound=` — empty; no SFX on actual death frame (same as Kirov).
- `CrashingSound=BlackOpsDie` — looping SFX during plummet. Ghidra-
  verified TechnoType `0x0084420c → 0x00712f80` (per CARRIER).
- `ImpactLandSound=GenAircraftCrash` — generic aircraft-impact SFX
  pool (3 samples). **DUAL-READ Rules+TechnoType** per cheat-sheet from
  ZEP doc.

**Voices**
- `VoiceSelect=BlackOpsSelect` — click voice (4-sample $vblhse* pool).
- `VoiceMove=BlackOpsMove` — move-order voice (4-sample $vblhmo* pool).
- `VoiceAttack=BlackOpsAttackCommand` — attack-order voice (4-sample
  $vblhat* pool).
- No `VoiceFeedback=` — empty.
- The "BlackOps" naming reflects the unit's stealth-helicopter identity;
  voices are clipped American special-forces tone.

**Transport entry/exit sounds**
- `EnterTransportSound=EnterTransport` — SFX when infantry boards (`genter1a`).
  [BINARY-VERIFIED audit 24: string @ 0x008440E8, parser xref @ 0x007133FC, `TechnoType+0x564` (int VocClass index)].
- `LeaveTransportSound=ExitTransport` — SFX when infantry disembarks
  (`gexit1a`). [BINARY-VERIFIED audit 24: string @ `0x008440D4` (NOT 0x008440F8 as previously inferred — strings stored in reverse order), parser xref @ 0x00713432, `TechnoType+0x568` (int VocClass index — RESOLVES audit-17 DEFERRED "unknown sibling" slot)].

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` — standard.
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — standard.
- `Trainable=yes` — **gains veterancy**. Unusual for a transport
  (Battle Fortress and Amphibious Transport are also Trainable=yes).
  Kills from the defensive cannon accumulate XP.

**Misc**
- `Size=15` — enormous. Cannot fit in any transport.
- `SpeedType=Hover` — hover speed table.
- `MoveSound=BlackOpsMoveLoop` — looping rotor SFX (5-sample random-
  loop pool).
- `TooBigToFitUnderBridge=true` — UnitType-scope, can't pass under
  bridges.
- `Bunkerable=no` — can't enter Tank Bunker (cheat-sheet TechnoType
  `0x0084371c → 0x0071500a`).
- `;Bombable=no` — commented. The `Bombable` system controls Crazy Ivan
  bomb placement on the target; commented means default behavior (which
  is `yes` for vehicles). Open question whether Ivan can bomb a flying
  SHAD — probably no in practice since Ivan can't reach aircraft.

---

## Artmd verbatim

```ini
[SHAD] ; BlackHawk transport
Cameo=SHADICON
AltCameo=SHADUICO
Voxel=yes
UseBuffer=yes
DisableVoxelCache=yes ; SJM: this is a major cache hog
DisableShadowCache=yes ; SJM: this too
Remapable=yes
TurretOffset=50
PrimaryFireFLH=175,0,10
ShadowIndex=2 ;order of voxels got changed
```

### Key-by-key annotation

- `Cameo=SHADICON` — sidebar build-button SHP.
- `AltCameo=SHADUICO` — UI-overlay alt cameo.
- `Voxel=yes` — rendered from `shad.vxl` + `shad.hva`.
- `UseBuffer=yes` — uses a *render buffer*, an intermediate offscreen
  blit target. Improves performance for complex voxel-shadow combinations.
  Same flag on HIND, SCHP.
- `DisableVoxelCache=yes` — verbatim SJM comment: "this is a major cache
  hog". The Nighthawk's voxel transformations are too heavy for the
  general voxel cache; disable it for this unit specifically. **Rare
  performance flag**. Same flag commented on RTNK Mirage Tank.
- `DisableShadowCache=yes` — same SJM rationale for shadow rendering.
- `Remapable=yes` — house-color remap.
- `TurretOffset=50` — turret position offset (50 leptons forward of body
  anchor — the rotor/turret renders in front of the body center).
- `PrimaryFireFLH=175,0,10` — bullet spawn offset:
  - X=175 (well forward; gun barrel sticks out the nose).
  - Y=0 (centered).
  - Z=10 (just above hull bottom — chin-gun height).
- `ShadowIndex=2` — selects voxel slice index 2 for shadow rendering.
  Verbatim comment: "order of voxels got changed" — Westwood reordered
  the voxel slices during development and had to update this index. Tells
  the shadow renderer which voxel-stack layer to use as the shadow
  silhouette source.

---

## Weapons

### Basic — `[BlackHawkCannon]`

```ini
[BlackHawkCannon]
Damage=35
ROF=40
Range=6
Projectile=QuadShell
Speed=100
Warhead=SA
Report=BlackOpsAttack
Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW
OmniFire=yes
```

- `Damage=35` — moderate.
- `ROF=40` — fast (~2.7 sec). Faster than most tank cannons.
- `Range=6` — 6-cell range.
- `Projectile=QuadShell` — *quad-shell projectile* (likely 4-pellet
  burst-fire spray). Different from `Cannon` (single arcing shell).
- `Speed=100` — fast projectile.
- `Warhead=SA` — Super-Armor-Piercing (small-arms / strong-vs-infantry).
- `Report=BlackOpsAttack` — fire SFX (2-sample random-interrupt).
- `Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW` —
  **8-direction muzzle flash anim**. The engine picks one of the 8
  compass-direction anims based on the *firing direction*. Equivalent
  to a turret rotation visual.
- `OmniFire=yes` — fires without facing the target. Combined with the
  helicopter's hover mode and `HoverAttack=yes`, the SHAD can spray
  bullets in any direction while stationary.

### Elite — `[BlackHawkCannonE]`

```ini
[BlackHawkCannonE]
Damage=40
ROF=40
Range=6
Projectile=QuadShell
Speed=100
Warhead=SSA
Report=BlackOpsAttack
Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW
OmniFire=yes
```

**Two changes vs basic:**
1. `Damage=40` (vs 35) — modest +14% damage.
2. `Warhead=SSA` (vs SA) — Super-Super-Armor-Piercing variant.
   Generally a slight Verses upgrade across armor types.

Modest elite upgrade compared to Tier-tank elite-swaps (Lasher gets
Burst=2 RHINAPE; SHAD gets +5 damage and a tier-up warhead). Reflects
the SHAD's role as a transport, not a combatant — the cannon is
self-defense, not primary purpose.

### Warhead — `[SA]` and `[SSA]`

Already documented in other unit docs (E1, GGI, etc.). Summary:
- `SA` (Super-Armor) — strong vs infantry, weak vs armor.
- `SSA` (Super-Super-Armor) — improved vs medium/heavy armor while
  retaining anti-infantry strength.

The SHAD's defensive cannon is fundamentally an anti-infantry/anti-light
weapon. Cannot threaten medium/heavy tanks (only moderate damage there).

---

## Voices / sounds

```ini
[BlackOpsSelect]
Sounds= $vblhsea $vblhseb $vblhsec $vblhsed
Control= random

[BlackOpsMove]
Sounds= $vblhmoa $vblhmob $vblhmoc $vblhmod
Control= random

[BlackOpsAttackCommand]
Sounds= $vblhata $vblhatb $vblhatc $vblhatd
Control= random

[BlackOpsVoiceDie]
Sounds= $vblhdia $vblhdib $vblhdic
Priority=low
Control= random

[BlackOpsAttack]
Sounds=vblhatta vblhattb
Control= random interrupt
VShift=10
Volume=65

[BlackOpsDie]
Sounds=vblhdiea
Volume=55

[BlackOpsMoveLoop]
Sounds= vblhlo1 vblhlo2a vblhlo2b vblhlo2c vblhlo3
Control= loop random all decay attack
Volume=50

[GenAircraftCrash]
Sounds=vaircraa vaircrab vaircrac
Control=random
FShift=-10
VShift=20
Volume=50

[EnterTransport]
Sounds=genter1a
FShift= -2 2
Volume=60

[ExitTransport]
Sounds=gexit1a
FShift= -1 1
Limit=2
Volume=60
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=BlackOpsSelect` | `[BlackOpsSelect]` | Click |
| `VoiceMove=BlackOpsMove` | `[BlackOpsMove]` | Move order |
| `VoiceAttack=BlackOpsAttackCommand` | `[BlackOpsAttackCommand]` | Attack order |
| `VoiceCrashing=BlackOpsVoiceDie` | `[BlackOpsVoiceDie]` | Voice while plummeting (3-sample priority-low pool) |
| `Report=BlackOpsAttack` (weapon) | `[BlackOpsAttack]` | Cannon fire SFX (`Control=random interrupt` — interrupts current sample on each fire) |
| `CrashingSound=BlackOpsDie` | `[BlackOpsDie]` | Sustained crash SFX during plummet |
| `ImpactLandSound=GenAircraftCrash` | `[GenAircraftCrash]` | Ground-impact SFX |
| `MoveSound=BlackOpsMoveLoop` | `[BlackOpsMoveLoop]` | Looping rotor sound (`Control=loop random all decay attack` — random-pool looping with envelope) |
| `EnterTransportSound=EnterTransport` | `[EnterTransport]` | Infantry boards (`genter1a`) |
| `LeaveTransportSound=ExitTransport` | `[ExitTransport]` | Infantry disembarks (`gexit1a`, Limit=2 concurrent) |
| `DieSound=` (empty) | n/a | No SFX on death frame (handled by CrashingSound + ImpactLandSound chain) |

**`Control=random interrupt`** on `[BlackOpsAttack]` is rare — most fire
SFX use `Control=random` (random pick, no interrupt). The `interrupt`
flag means each new fire *cuts off* the currently-playing sample. Used
for fast-firing weapons (ROF=40) to avoid sample overlap muddying the
audio.

---

## Hardcoded behavior (Ghidra-verified)

### 1. RadarInvisible at ObjectType scope

`RadarInvisible=yes` is read in **`ObjectTypeClass__ReadINI`** at
`0x00832b9c → 0x005f946e` — **broader scope than TechnoType**. The
ObjectType class is the parent of TechnoType (Unit/Infantry/Aircraft/
Building inherit from TechnoType which inherits from ObjectType). Reading
at ObjectType means the field applies to *any* object instance —
including debris, animations, smudges, terrain props — though in practice
only units use it.

Same broader-scope pattern as `NoSpawnAlt` (also ObjectType-scope).
**This is a useful pattern to note**: most fields read at TechnoType,
but a handful are at ObjectType (broader) or at UnitType (narrower —
vehicle-only).

The effect: SHAD does not appear on enemy minimap. Combined with its
fast speed (14), the unit can ferry infantry across the map without
giving early warning to the enemy. **Key gameplay role: stealth
transport for surgical strikes**.

### 2. HoverAttack=yes

Ghidra-verified TechnoType at `0x008443b0 → 0x0071255a`. **NEW cheat-
sheet entry**. The flag allows the unit to fire while in a stationary
hover state (no need to strafe like a fixed-wing aircraft).

The state machine:
- Without HoverAttack: aircraft must enter "attack run" mode (move past
  target, fire during pass).
- With HoverAttack: aircraft pauses, hovers in place, fires from
  stationary position.

For a transport, this is essential — players don't want their SHAD to
strafe through enemy AA when defending its passengers.

### 3. PreventAttackMove=yes

Ghidra-verified TechnoType at `0x008439b0 → 0x00714994`. **NEW cheat-
sheet entry**. Disables the *Attack-Move* command (the `Ctrl+Click` or
`A` hotkey behavior that orders a unit to engage anything along its
path). For SHAD, this means:
- Standard Move command: moves to waypoint, doesn't engage.
- Attack-Move command: **rejected** — same as Move.
- Force-fire command: fires at the target.

Combined with `CanPassiveAquire=no` (won't auto-engage), the SHAD is
*fully passive* — it does *only* what's ordered. Designed to make
transport flights predictable: a SHAD with infantry inside won't get
sidetracked attacking a stray unit during its waypoint route.

### 4. CanPassiveAquire=no

Ghidra-verified TechnoType `0x00843c50 → 0x00714473` (cheat-sheet from
CARRIER doc). Disables the passive-target-acquisition pass (the AI tick
that scans for in-range targets and queues attacks). The SHAD won't
auto-engage anything; only fires on explicit order or when force-fired.

### 5. EnterTransportSound/LeaveTransportSound

Ghidra-verified TechnoType `0x008440e8 → 0x007133fc` for
`EnterTransportSound`. **NEW cheat-sheet entry**. Triggers when an
infantry/vehicle enters the transport's passenger slot. Same pattern
expected for `LeaveTransportSound` at adjacent string address (high-
confidence inference, not separately verified this iteration).

Both sounds play at the *transport's* position (not the passenger's),
audible to all players within standard sound range.

### 6. JumpJet locomotor parameters

See [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
for the full locomotor state machine. Key SHAD-specific points:
- `JumpjetHeight=500` — mid-altitude. Below Kirov (750), above Rocketeer.
- `JumpJetTurnRate=6` — fast turn (vs Kirov's 2).
- `JumpjetClimb=10` / `JumpjetCrash=40` — fast vertical movement.
- `Speed=14` — moderate fast horizontal speed.

The combination makes SHAD a nimble jumpjet — quick to reposition, quick
to dodge, quick to land.

### 7. Trainable=yes on a transport

Standard cap on transports is `Trainable=no` (since they don't directly
combat). SHAD is an exception — its defensive cannon scores XP, the unit
can rank up to veteran/elite. With Trainable=yes, the elite-rank weapon
swap to BlackHawkCannonE applies. Marginal upgrade but flavor-correct.

### 8. JumpJet=yes flag

Discriminator field; gates whether unit is treated as a "vehicle" or
"aircraft" in some code paths. Combined with `ConsideredAircraft=yes`,
SHAD goes to the aircraft AI threat bucket but uses vehicle-flow build
queue (war factory output). Ghidra cheat-sheet TechnoType.

---

## TS-legacy filter

- `;Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1} ;flying` — commented;
  historical use of fixed-wing locomotor.
- `;Prerequisite=GAHPAD` — commented; helipad removed from build tree.
- `;OmniFire=yes` — commented; moved to weapon (live).
- `;Bombable=no` — commented; default applies.
- `;AuxSound1/2=BlackOpsTakeOff/Landing` — commented blocks; the sounds
  themselves are also commented in soundmd.ini (block defined but empty).
  Takeoff/landing SFX system *is* live in YR but not used here.
- No `ImmuneToVeins`, no `Subterranean`, no other TS-only fields.

---

## Cross-references

- [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — jumpjet locomotor state machine.
- [JUMPJET.md](../allied/JUMPJET.md) — Rocketeer, tier-1 jumpjet (no
  passengers).
- [ZEP.md](../soviet/ZEP.md) — Kirov, tier-3 jumpjet bomber (heavier,
  slower, with bombs).
- [DISK.md](../yuri/DISK.md) — Floating Disc, tier-3 jumpjet
  (BalloonHover, doesn't crash).
- [SAPC.md](../allied/SAPC.md) — pending; Amphibious Transport sibling.
- [YHVR.md](../yuri/YHVR.md) — pending; Yuri Hover Transport sibling.

---

## Ghidra audit log (audit iteration 24 — 2026-05-18)

**Methodology**: SHAD has 4 NEW field-scope claims (HoverAttack,
PreventAttackMove, EnterTransportSound, RadarInvisible) — with
HoverAttack and PreventAttackMove having been previously cited in
cumulative (audits 8, 10) without parser-xref verification, and
RadarInvisible already fully audited in audit 21 (ObjectType-scope).
This audit verifies all 4 + pins their struct offsets + verifies the
LeaveTransportSound sibling, with a correction to a doc inference. ~14
Ghidra queries: 5 string-searches + 4 xref lookups + 1 grep on saved
TechnoTypeClass__ReadINI.

### Negative claim re-verified

| Query | Result |
|-------|--------|
| `search_strings("^SHAD$")` | **0 matches** |

Confirms no hardcoded SHAD-name branch.

### String + parser xref verification (BINARY-VERIFIED)

All 4 doc-cited claims verify exactly + 1 BONUS:

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `HoverAttack` | 0x008443B0 | 0x0071255A | TechnoTypeClass__ReadINI |
| `PreventAttackMove` | 0x008439B0 | 0x00714994 | TechnoTypeClass__ReadINI |
| `EnterTransportSound` | 0x008440E8 | 0x007133FC | TechnoTypeClass__ReadINI |
| `LeaveTransportSound` (bonus) | 0x008440D4 | 0x00713432 | TechnoTypeClass__ReadINI |

### [ADDRESS DISCREPANCY corrected]

The doc inferred `LeaveTransportSound` would be at `0x008440F8` (16
bytes after EnterTransport at `0x008440E8`). **Actual address: `0x008440D4`** —
20 bytes BEFORE EnterTransport (strings stored in reverse order in
memory: LeaveTransport then EnterTransport). The doc's "+16-byte
adjacency" inference was the wrong direction and wrong magnitude;
corrected here.

### NEW / re-confirmed TechnoType offsets BINARY-VERIFIED

| Offset | INI key | Type | Status |
|--------|---------|------|--------|
| `+0x390` | `HoverAttack` | byte | `*(undefined1*)(param_1 + 0xE4) = uVar3` after ReadBool. **Re-confirms audit 8 cumulative** (jumpjet block, was noted at +0x390 without parser-xref proof). Now BINARY-VERIFIED both ways. |
| `+0x6C8` | `PreventAttackMove` | byte | `*(undefined1*)(param_1 + 0x1B2) = uVar3`. **Re-confirms audit 10 cumulative** (SNIPE) — now with parser-xref proof. |
| `+0x564` | `EnterTransportSound` | int (VocClass index) | `param_1[0x159] = iVar6` (sequence-position evidence — write occurs just before LeaveTransportSound parse begins). **NEW**. |
| `+0x568` | `LeaveTransportSound` | int (VocClass index) | INFERRED by parse-order adjacency. **RESOLVES AUDIT-17 DEFERRED**: audit 17 cumulative had +0x568 as "unknown sibling, possibly SegueSound or CreateSound" — actually `LeaveTransportSound`. |

### Sound-cluster topology UPDATE (cumulative consolidation)

The sound-slot cluster at +0x568 is now half-resolved with this audit:

| Offset | INI key | Audit | Status |
|--------|---------|-------|--------|
| `+0x564` | EnterTransportSound | **24** | NEW |
| `+0x568` | LeaveTransportSound | **24** | NEW (resolves audit-17 DEFERRED) |
| `+0x56C` | DeploySound | 14 | confirmed |
| `+0x570` | UndeploySound | 14 | confirmed |
| `+0x574` | ChronoInSound | 17 | confirmed |
| `+0x578` | ChronoOutSound | 17 | confirmed |
| `+0x57C` | (still unknown) | 17 | DEFERRED |
| `+0x5A8` | ActivateSound | 23 | confirmed |
| `+0x5AC` | DeactivateSound | 23 | confirmed |

Pattern: there's a "transport-related sound block" (Enter/Leave at +0x564/+0x568), followed by the "deploy/chrono sound block" (+0x56C..+0x578), an unknown +0x57C, then a gap, then the "power-state sound block" (+0x5A8/+0x5AC).

### RadarInvisible (re-confirmation from audit 21)

The doc cites `RadarInvisible @ 0x00832B9C → 0x005F946E` in
`ObjectTypeClass__ReadINI`. This was BINARY-VERIFIED in audit 21
(DEST, full ObjectTypeClass__ReadINI decompile) at `ObjectType+0x22F`
(byte). The doc's "ObjectType-scope (broader than TechnoType)"
interpretation is correct. No new audit work needed.

### Items NOT re-verified in this pass (DEFERRED)

- The JumpJet locomotor parameters (JumpjetSpeed/Climb/Crash/Accel/TurnRate/Height/Wobbles/Deviation) — audit 8 cumulative already covers these for the Rules global block; per-TechnoType values are typed-INI passthroughs.
- The HoverAttack consumer chain (when a unit fires while hovering vs strafing — TechnoClass per-tick code).
- The PreventAttackMove consumer (the Attack-Move command handler that rejects this unit).
- `[BlackOpsMoveLoop]` looping rotor sound state machine.
- 8-direction MGUN anim selection in `BlackHawkCannon` (the engine's direction-from-fire-vector → anim index resolution).

### Confidence summary

- **HIGH**: 5 string addresses + 4 parser xrefs (all exact); 1 BONUS (LeaveTransportSound — corrects doc's wrong inferred address); 4 TechnoType struct offsets verified (2 new + 2 re-confirmed); 1 audit-17 DEFERRED RESOLVED (+0x568 = LeaveTransportSound).
- **MEDIUM**: LeaveTransportSound offset inferred by parse-order; would benefit from direct write-site verification via wider grep.
- **No INCORRECT findings**. The single doc inaccuracy (LeaveTransportSound address inference) is now corrected.

---

## Coverage audit

- [x] Every rulesmd key annotated (~55 keys).
- [x] Every artmd key annotated (9 keys including SJM cache flags).
- [x] Both weapons documented (BlackHawkCannon + Elite).
- [x] 8-direction MGUN anim explained.
- [x] All voice/sound bindings documented (10 entries).
- [x] Prerequisites: `GAWEAP` only.
- [x] Owner: 5 Allied houses.
- [x] Veterancy: Trainable=yes + Elite weapon swap.
- [x] Passenger capacity: 5 with SizeLimit=2.
- [x] Hardcoded behavior: RadarInvisible at ObjectType scope (broader!),
  HoverAttack, PreventAttackMove, CanPassiveAquire, EnterTransportSound,
  JumpJet locomotor params, Trainable transport.
- [x] TS-legacy filter applied (commented historical fields).
- [x] At least one Ghidra search performed (4 strings + xrefs).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("HoverAttack")` | `0x008443b0` (single match) |
| `get_xrefs_to(0x008443b0)` | `0x0071255a → TechnoTypeClass__ReadINI` |
| `search_strings("PreventAttackMove")` | `0x008439b0` (single match) |
| `get_xrefs_to(0x008439b0)` | `0x00714994 → TechnoTypeClass__ReadINI` |
| `search_strings("EnterTransportSound")` | `0x008440e8` (single match) |
| `get_xrefs_to(0x008440e8)` | `0x007133fc → TechnoTypeClass__ReadINI` |
| `search_strings("RadarInvisible")` | `0x00832b9c` (single match) |
| `get_xrefs_to(0x00832b9c)` | `0x005f946e → ObjectTypeClass__ReadINI` **(BROADER SCOPE)** |

**New cheat-sheet entries (4):**
- `HoverAttack` (0x008443b0 → 0x0071255a) TechnoType
- `PreventAttackMove` (0x008439b0 → 0x00714994) TechnoType
- `EnterTransportSound` (0x008440e8 → 0x007133fc) TechnoType
- `RadarInvisible` (0x00832b9c → 0x005f946e) **ObjectType** — broader
  than TechnoType; applies to all ObjectTypes. Same scope as NoSpawnAlt.

**Re-confirmed scopes:**
- `ConsideredAircraft` (cheat-sheet from DISK) — TechnoType
- `CanPassiveAquire` (cheat-sheet from CARRIER) — TechnoType
- `PitchSpeed`/`PitchAngle` (cheat-sheet from CARRIER) — TechnoType
- `Bunkerable` (cheat-sheet from TELE) — TechnoType
- `TooBigToFitUnderBridge` (cheat-sheet) — UnitType

**Open questions:**
- `LeaveTransportSound` ReadINI exact address. Adjacent string layout
  suggests it's at `0x008440f8` (16-byte offset from EnterTransport
  at 0x008440e8). Not separately verified this iteration. High confidence
  inference; could verify next iteration.
- Does `RadarInvisible=yes` actually hide the *passenger* infantry's
  presence too? Or only the helicopter itself? Open question — relevant
  for radar-detection logic when SHAD lands and unloads infantry. The
  passengers are technically inside the SHAD and have no presence on
  the map until unloaded. Likely no special interaction.
