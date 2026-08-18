# DEST — Destroyer (Allied Naval Anti-Sub Escort)

**INI ID:** `DEST`
**Display Name:** `Destroyer` (`UIName=Name:DEST`)
**Side:** Allied (all 5 Allied houses)
**Category:** Vehicle / Naval
**Cameo:** `desticon` (lowercase in INI — case-insensitive)
**Voxel:** yes, with NoSpawnAlt voxel swap `DESTWO` when Osprey is away.

The Destroyer is the Allied tier-2 anti-naval escort. **Dual-role unit:** a
direct-fire 155mm naval gun (anti-surface) AND a spawned `ASW` Osprey
helicopter for anti-submarine work. Completes the 4-unit YR spawner family
(Carrier / Dread / V3 / Destroyer) as the *smallest* and *fastest* spawner.

### Spawner family — Destroyer in context

| | Destroyer (this) | Carrier ([CARRIER](./CARRIER.md)) | Dread ([DRED](../soviet/DRED.md)) | V3 ([V3](../soviet/V3.md)) |
|---|---|---|---|---|
| Side | Allied | Allied | Soviet | Soviet |
| Spawned child | ASW (Osprey heli) | HORNET (fighter) | DMISL (kamikaze missile) | V3ROCKET (kamikaze missile) |
| `SpawnsNumber=` | **1** | 3 | 2 | 1 |
| `SpawnRegenRate=` | 400 (~27 sec) | 600 (~40 sec) | 80 (~5 sec) | 150 (~10 sec) |
| `SpawnReloadRate=` | 150 (return+dock 10 sec) | 150 | 0 (one-shot) | 0 (one-shot) |
| `MissileSpawn=` on child | no | no | yes | yes |
| Primary weapon | **155mm** (direct fire, REAL weapon) | HornetLauncher (spawner virtual) | DredLauncher (spawner virtual) | V3Launcher (spawner virtual) |
| Secondary weapon | ASWLauncher (spawner virtual) | — | — | — |
| Cost / TechLevel / HP | 1000 / 4 / 600 | 2000 / 7 / 800 | 2000 / 6 / 800 | 1450 / 7 / 150 (land) |
| `Speed=` | **6** (fastest) | 4 | 4 | 5 |
| `Sensors=yes` | **yes** (only ship with it) | no | no | no |

> **Key distinguishing feature: DEST is the only spawner that *also* has a normal primary weapon.** Its main role is surface engagement with the 155mm gun; the Osprey is supplemental anti-sub. The other three spawners are *exclusively* spawner-platforms.

> **Cross-references — do not re-derive:**
> - [`CARRIER.md`](./CARRIER.md) — exhaustive treatment of the reusable-aircraft spawn pattern (same SpawnManagerClass flow, Osprey is mechanically near-identical to Hornet).
> - [`DRED.md`](../soviet/DRED.md) and [`V3.md`](../soviet/V3.md) — sibling missile-spawner units.
> - [`SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md`](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md) — full SpawnManager state machine.
> - [`AIRCRAFTCLASS_GHIDRA_REPORT.md`](../../AIRCRAFTCLASS_GHIDRA_REPORT.md) — aircraft landing/docking.
> - [`BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md`](../../BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md) — locomotor CLSID table.

> **TS-legacy filter:** `;Lobber=yes` on 155mm, `;Range=-2` on ASWLauncher, `;AntiUnderwater=yes` on ASWLauncher, `;Dock=GAAIRC,AMRADR` on Osprey, `;AS=yes/;AN=yes` on ASWVirt projectile, `;Selectable=no` on Osprey — all INI-commented and inert. `Sensors=yes` is **live** and is the only "anti-sub-can-see-submarine" mechanism (not a TS holdover).

---

## 1. Full `rulesmd.ini` section verbatim

```ini
[DEST]
UIName=Name:DEST
Name=Destroyer
Prerequisite=GAYARD
Primary=155mm
Secondary=ASWLauncher
NavalTargeting=1
Spawns=ASW
SpawnsNumber=1
SpawnRegenRate=400
SpawnReloadRate=150
NoSpawnAlt=yes ; alternate voxel for out of spawns: xxxxWO (DESTWO)
FireAngle=32
ToProtect=yes
Category=Support
Strength=600
Naval=yes ;GS
Armor=heavy
TechLevel=4
Sight=7
Speed=6
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=1000
Soylent=1000
Turret=no ; can't have a turrett and a NoSpawnAlt (both go in AuxVoxel)
Points=30
ROT=5
Crusher=no ;gs yes
Weight=3
Crewed=no
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=GenAllWaterSelect
VoiceMove=GenAllWaterMove
VoiceAttack=GenAllWaterAttackCommand
VoiceFeedback=
DieSound=
SinkingSound=GenLargeWaterDie
MoveSound=DestroyerMoveStart
Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C}
SpeedType=Float
MovementZone=Water
ThreatPosed=15	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
TooBigToFitUnderBridge=true
Sensors=yes
SensorsSight=8
OpportunityFire=yes ; since no turret, will only apply to helicopter (since ASWLauncher has OmniFire)
ElitePrimary=155mmE
Size=30
IsSelectableCombatant=yes
```

### 1.1 Key-by-key (focus on what differs from CARRIER)

| Key | Value | Read by | Effect |
|-----|-------|---------|--------|
| `UIName=Name:DEST` | string | AbstractTypeClass | CSF lookup. |
| `Name=Destroyer` | string | AbstractTypeClass | English fallback. |
| `Prerequisite=GAYARD` | building | TechnoTypeClass | **Only Allied Naval Yard required** — NO tech-lab prerequisite (vs Carrier's `GAYARD,TECH`). This is a tier-2 unit, available much earlier. |
| `Primary=155mm` | weapon | TechnoTypeClass | **REAL primary weapon** — direct-fire ballistic 155mm cannon. See §3.1. |
| `Secondary=ASWLauncher` | weapon | TechnoTypeClass | Spawner-virtual ASW launcher (Osprey). See §3.2. |
| `NavalTargeting=1` | enum | TechnoType @ 0x007121be [BINARY-VERIFIED audit 21: string @ 0x00844510, `TechnoType+0x600` (int) — re-confirms audit 7 cumulative] | Targeting profile #1 (see rulesmd comment block at line 3691): `1 = NAVAL_SUBPRIMARY` — can shoot ground targets with secondary weapon but **submarines are the primary target**. |
| `Spawns=ASW` | aircraft type | TechnoTypeClass | Spawned child. |
| `SpawnsNumber=1` | int | TechnoType @ 0x00714ee1 | **Single Osprey** — only one aircraft in the magazine. |
| `SpawnRegenRate=400` | frames | TechnoType @ 0x00714ec0 | ~27 sec to manufacture replacement Osprey if shot down. |
| `SpawnReloadRate=150` | frames | TechnoType @ 0x00714f02 | 10 sec to reload after dock (same as Carrier). |
| `NoSpawnAlt=yes` | bool | **ObjectType** @ 0x005F943E [BINARY-VERIFIED audit 21: string @ 0x00832BC0, parser in `ObjectTypeClass__ReadINI`, `ObjectType+0x1E8` (byte). ObjectType-scope means this also works on BuildingType/AnimType/etc., not just unit-class TechnoTypes.] | Swap to `DESTWO` voxel when Osprey is away — visible "empty deck" appearance. |
| `FireAngle=32` | int | TechnoType @ 0x00714b5d | 45° initial pitch for Osprey launch. |
| `ToProtect=yes` | bool | TechnoType @ 0x00714be8 | AI escort flag. |
| `Category=Support` | enum | TechnoTypeClass | Support category. |
| `Strength=600` | hp | TechnoTypeClass | **600 HP** — lighter than Carrier/Dread (800). |
| `Naval=yes` | bool | UnitTypeClass | Naval flag. |
| `Armor=heavy` | enum | TechnoTypeClass | Heavy armor. |
| `TechLevel=4` | int | TechnoTypeClass | **Tier 2** — much earlier than Carrier (7) or Dread (6). Standard mid-game naval availability. |
| `Sight=7` | cells | TechnoTypeClass | Standard ship sight. |
| `Speed=6` | int | TechnoTypeClass | **Fastest of the four spawners** — 50 % faster than Carrier/Dread (4). |
| `Owner=British,French,Germans,Americans,Alliance` | country list | TechnoTypeClass | All 5 Allied houses. |
| `AllowedToStartInMultiplayer=no` | bool | TechnoTypeClass | Not pre-built. |
| `Cost=1000` | credits | TechnoTypeClass | Half the cost of Carrier/Dread. |
| `Soylent=1000` | credits | TechnoTypeClass | Full-cost recycle. |
| `Turret=no` | bool | UnitTypeClass | No turret. The inline comment is critical: "can't have a turrett and a NoSpawnAlt (both go in AuxVoxel)" — the engine's auxiliary voxel slot is single-use, occupied by either a turret OR the empty-spawn variant. **A naval-spawner ship cannot have both a turret-mounted gun AND a NoSpawnAlt swap; the design forces a choice.** |
| `Points=30` | int | TechnoTypeClass | Half of Carrier/Dread's 55. |
| `ROT=5` | int | TechnoTypeClass | **5× faster turning than Carrier/Dread (1)** — agile ship. Combined with no-turret + OmniFire weapons, effectively omnidirectional combat. |
| `Crusher=no ;gs yes` | bool | TechnoTypeClass | No crush. `;gs yes` is a draft annotation. |
| `Weight=3` | int | TechnoTypeClass | Lighter than Carrier (5). |
| `Crewed=no` | bool | TechnoTypeClass | No crew bailout. |
| `Explosion=...` | anim list | TechnoTypeClass | Standard 5-anim destruction set. |
| `VoiceSelect=GenAllWaterSelect` | sound | TechnoTypeClass | **Generic ALLIED water-unit voice** (`$vwaa*` — distinct from CARRIER's unique `$vair*` set). Shared with other Allied ships (Hydrofoil, Dolphin, Aegis). |
| `VoiceMove=GenAllWaterMove` | sound | TechnoTypeClass | Generic Allied water move. |
| `VoiceAttack=GenAllWaterAttackCommand` | sound | TechnoTypeClass | Generic Allied water attack. |
| `VoiceFeedback=` | (empty) | TechnoTypeClass | None. |
| `DieSound=` | (empty) | TechnoTypeClass | None. |
| `SinkingSound=GenLargeWaterDie` | sound | DUAL-READ | Standard sinking groan. |
| `MoveSound=DestroyerMoveStart` | sound | TechnoTypeClass | `vdesstaa/b` random predelay 0-400, Priority=Low, Volume 30. |
| `Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C}` | CLSID | TechnoTypeClass | **ShipLocomotionClass** — **single GUID, no trailing comment** (cleaner authoring vs CARRIER/DRED). |
| `SpeedType=Float` | enum | TechnoTypeClass | Water type. |
| `MovementZone=Water` | enum | TechnoTypeClass | Water zone. |
| `ThreatPosed=15` | int | TechnoTypeClass | 60 % of Carrier/Dread's 25 — lower AI threat weight. |
| `DamageParticleSystems=SparkSys,SmallGreySSys` | particle list | TechnoTypeClass | Damaged emissions. |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | ability list | TechnoTypeClass | Standard vet bonuses. |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | ability list | TechnoTypeClass | Standard elite bonuses + self-heal. |
| `TooBigToFitUnderBridge=true` | bool | UnitType @ 0x0074774e | Bridge-block. |
| `Sensors=yes` | bool | TechnoType @ 0x00714003 [BINARY-VERIFIED audit 21: string @ 0x00843E58, `TechnoType+0xC9D` (byte)] | **Submarine-detection ability**. The destroyer can see cloaked submarines (Typhoon Sub, Boomer Sub) within `SensorsSight` cells. |
| `SensorsSight=8` | cells | TechnoType @ 0x007142e8 [BINARY-VERIFIED audit 21: string @ 0x00843D50, `TechnoType+0x5F0` (int) — adjacent to audit-6 +0x5F4 DetectDisguiseRange, forms a "detection-range cluster"] | **8-cell sub-detection range**. Greater than visual Sight=7 — the destroyer reveals submarines at a wider radius than it sees regular units. |
| `OpportunityFire=yes ; since no turret, will only apply to helicopter (since ASWLauncher has OmniFire)` | bool | TechnoType @ 0x0071483d (cheat sheet) | Will engage targets during movement. Inline comment explains: since DEST has no turret, only the OmniFire-tagged ASWLauncher (which fires in any direction) benefits from OpportunityFire mid-movement; the 155mm gun needs the hull to face the target. |
| `ElitePrimary=155mmE` | weapon | TechnoType @ 0x00712a32 | **Elite swap on Primary** — when Destroyer is elite, its 155mm gains a `Burst=2` (double salvo). See §3.4. |
| `Size=30` | int | TechnoTypeClass | Transport-cost (smaller than Carrier/Dread's 50). |
| `IsSelectableCombatant=yes` | bool | TechnoTypeClass | Combat unit. |

---

## 2. Full `artmd.ini` section verbatim

```ini
[DEST]
Cameo=desticon
Voxel=yes
Remapable=yes
PrimaryFireFLH=280,0,120
```

| Key | Value | Notes |
|-----|-------|-------|
| `Cameo=desticon` | SHP | Build-list cameo (lowercase in INI; the engine is case-insensitive for SHP names). |
| `Voxel=yes` | bool | `dest.vxl` + `.hva` + `destwo.vxl` (the NoSpawnAlt alternate). |
| `Remapable=yes` | bool | House-color tinted. |
| `PrimaryFireFLH=280,0,120` | x,y,z leptons | **155mm muzzle position:** 280 forward, 0 sideways, 120 up. The cannon visibly fires from the bow gun. |

> Note: this `PrimaryFireFLH` is for the 155mm direct-fire weapon, not the Osprey launch. The Osprey's launch position is engine-default (no FLH defined on DEST for the spawner).

---

## 3. Weapons

### 3.1 `[155mm]` — primary direct-fire cannon

```ini
[155mm]
Damage=60
ROF=110
Range=8
MinimumRange=0
Projectile=Ballistic
Speed=10
Warhead=ARTYHE
Report=DestroyerAttack
Anim=GUNFIRE
Lobber=no
;Lobber=yes
```

| Key | Effect |
|-----|--------|
| `Damage=60` | Per-shot base damage. |
| `ROF=110` | ~7 sec between shots. |
| `Range=8` | 8-cell engagement range — moderate naval range (vs Carrier's 25-cell Hornet sortie). |
| `MinimumRange=0` | No dead zone; can fire on adjacent targets. |
| `Projectile=Ballistic` | Arcing shell — see §3.5. |
| `Speed=10` | Projectile speed. |
| `Warhead=ARTYHE` | Standard HE artillery warhead (CellSpread=1, rocker, deform — see [`CARRIER.md`](./CARRIER.md) §3.5.2 for full ARTYHE keys). |
| `Report=DestroyerAttack` | `vdesatta/b` random gunfire sound. |
| `Anim=GUNFIRE` | Muzzle flash anim (universal GUNFIRE SHP). |
| `Lobber=no` | Direct-fire trajectory (despite `Projectile=Ballistic` arcing). Inline `;Lobber=yes` is a commented author draft. |

### 3.2 `[ASWLauncher]` — virtual launcher for Osprey

```ini
[ASWLauncher]
Damage=1
ROF=150
Range=-2 ; infinite
Spawner=yes
Projectile=ASWVirt
;AntiUnderwater=yes
Speed=10
Warhead=Special
OmniFire=yes
```

| Key | Effect |
|-----|--------|
| `Damage=1` | Placeholder. |
| `ROF=150` | 10-sec lockout between Osprey dispatches. |
| `Range=-2 ; infinite` | **Infinite range** — Osprey can be dispatched to any visible target on the map. Compare HornetLauncher which has Range=25. The destroyer is a long-arm scout/anti-sub platform. |
| `Spawner=yes` | Releases from SpawnManager. |
| `Projectile=ASWVirt` | Bookkeeping projectile (see §3.5). |
| `;AntiUnderwater=yes` | (commented) — would enable a special anti-sub flag; disabled. |
| `Speed=10` | Irrelevant. |
| `Warhead=Special` | Misleading — Special is a House, not a Warhead. No damage from launcher. |
| `OmniFire=yes` | Fire in any direction. |

### 3.3 `[ASW]` (Osprey) aircraft — full section

```ini
[ASW]
UIName=Name:ASW
Name=Osprey
Primary=ASWBomb
Secondary=ASWCollision
NavalTargeting=2
LandTargeting=1
Strength=30
Category=AirPower
Armor=light
Spawned=yes
TechLevel=-1
Sight=2
RadarInvisible=no
Landable=yes
MoveToShroud=yes
;Dock=GAAIRC,AMRADR
PipScale=Ammo
Speed=12
PitchSpeed=.9
PitchAngle=0
Owner=British,French,Germans,Americans,Alliance
Cost=50
Points=10
ROT=5;3
Ammo=1
Crewed=no
GuardRange=30
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=2
VoiceSelect=
VoiceMove=
VoiceAttack=
VoiceFeedback=
DieSound=
CrashingSound=OspreyDie
ImpactLandSound=GenAircraftCrash
Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1}
MovementZone=Fly
MovementRestrictedTo=Water ; See if this will affect landing only
ThreatPosed=10	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
AuxSound1=OspreyTakeOff ;Taking off
AuxSound2=OspreyLanding ;Landing
ImmuneToPsionics=yes
;Selectable=no	; SJM: this should be here but is commented out because bug prevents aircraft from landing
```

Diffs vs HORNET (see [`CARRIER.md`](./CARRIER.md) §3.2 for full Hornet treatment — most keys are identical):

| Key | DEST/ASW | CARRIER/HORNET | Note |
|-----|----------|----------------|------|
| `Name=` | Osprey | Hornet | Different aircraft type. |
| `Primary=` | `ASWBomb` | `HornetBomb` | Different weapon. |
| `Secondary=` | `ASWCollision` | `HornetCollision` | Both are crash-collision weapons. |
| `NavalTargeting=` | **2** | (not set on Hornet) | `2 = NAVAL_PRIMARY` — submarines are the Osprey's primary target. |
| `LandTargeting=` | **1** | (not set on Hornet) | `1 = LAND_SECONDARY` — can hit land but it's secondary. |
| `Strength=` | **30** | 75 | Osprey is far more fragile. |
| `Points=` | 10 | 20 | Less score on kill. |
| `ROT=` | **5;3** | 3 | Slightly more agile (5; legacy `;3` is inline comment of old value). |
| `GuardRange=` | 30 | 30 | Same. |
| `CrashingSound=` | OspreyDie | HornetDie | Different audio. |
| `AuxSound1/2=` | OspreyTakeOff/Landing | HornetTakeoff/Landing | Different audio. |
| **No `ElitePrimary=`** | — | `HornetBombE` | **Osprey has no elite swap** — its weapon stays ASWBomb at all ranks. Combined with no VeteranAbilities/EliteAbilities block, the Osprey doesn't gain meaningful elite bonuses; ranks up only for cosmetic chevrons. |
| `;Dock=` (single commented line) | `;Dock=GAAIRC,AMRADR` only | both `;Dock=NAHPAD,GAHPAD` AND `;Dock=GAAIRC,AMRADR` | Same intent — disabled, destroyer is the dock. |

#### 3.3.1 `[ASW]` artmd

```ini
[ASW] ; Destroyer Plane
Cameo=PROICON
Voxel=yes
PrimaryFireFLH=0,32,0
```

Identical to Hornet's artmd block (3 keys, same FLH offset). `PROICON` placeholder cameo — ASW is not directly buildable. No `Remapable=yes` (universal Allied grey).

### 3.4 Weapons of Osprey

```ini
[ASWBomb]
Damage=50
ROF=3
Range=3
Projectile=DepthCharge
Speed=30
Warhead=APSplash
Report=OspreyAttack
```

| Key | Effect |
|-----|--------|
| `Damage=50` | 50 base damage per depth charge. |
| `ROF=3` | 3-frame ROF (with Ammo=1, this is largely moot). |
| `Range=3` | Drop from 3 cells away (closer than Hornet's 5 — Osprey hovers above the target). |
| `Projectile=DepthCharge` | Falling depth-charge projectile (see §3.5). |
| `Speed=30` | Drop speed. |
| `Warhead=APSplash` | **Anti-armor splash warhead** — strong vs vehicles and submarines, weak vs infantry (Verses 25/25/25 for inf classes, 75/100/100 for vehicles, 25 % for special_1). See full warhead in §3.6. |
| `Report=OspreyAttack` | `vospatta` interrupt sound, Volume default. |

```ini
[ASWCollision] ;A crashing ASW turns into this bullet at the last second
Damage=100
ROF=20
Range=3
Projectile=AAHeatSeeker2 ; will be ASW shaped bullet
Speed=30
Warhead=AP
Report=OspreyCollision
Bright=yes
```

Identical structure to HornetCollision (100 AP dmg, AAHeatSeeker2, Range=3, Bright=yes). Inline comment confirms same crash-on-target swap mechanism. Sound `OspreyCollision` is intentionally silent (`Volume=0 ; no sound`).

### 3.5 Elite-Destroyer 155mm variant

```ini
[155mmE]
Damage=60
ROF=110
Range=8
MinimumRange=0
Projectile=Ballistic
Speed=10
Warhead=ARTYHE
Report=DestroyerAttack
Anim=GUNFIRE
Lobber=no
Burst=2
```

The only difference vs base `[155mm]` is `Burst=2` — elite Destroyers fire **two 155mm shells per fire command** (effective doubling of DPS via ROF unchanged). Same range, damage per shot, warhead.

### 3.6 Projectiles

```ini
[Ballistic]
Image=120MM
Arcing=true
SubjectToCliffs=no
SubjectToElevation=yes
SubjectToWalls=no
```

Arcing artillery shell. `Image=120MM` (universal cannon shell sprite). `SubjectToElevation=yes` — terrain elevation affects the arc; the shell can clear walls but is affected by hills.

```ini
[DepthCharge]
Arm=2
Shadow=no
Proximity=yes
Ranged=yes
Image=DRAGON
ROT=1
IgnoresFirestorm=yes
;AS=yes
```

Standard falling-bomb projectile (same `DRAGON` image as NormalBomb). `Proximity=yes` triggers on near-target. `;AS=yes` commented = anti-sub flag disabled — but the warhead/launcher targeting handles the sub-engagement instead.

```ini
[ASWVirt]
AG=no
;AS=yes
AA=no
;AN=yes
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=no
```

Invisible bookkeeping projectile for the spawner launcher. `AG=no` `AA=no` (and commented `AS`/`AN`) — does not directly damage anything; it exists to satisfy the projectile-required slot on the weapon.

### 3.7 Warheads

#### 3.7.1 `[ARTYHE]` — 155mm impact

See full key list in [`CARRIER.md`](./CARRIER.md) §3.5.2 (used there as elite Hornet bomb warhead). Key facts:
- `CellSpread=1`, `PercentAtMax=.25` (1-cell radius, 25 % edge falloff)
- `Verses=100/80/60/100/60/60/100/100/60/100/100` — HE curve
- `Rocker=yes`, `Deform=15%`, `DeformThreshhold=120`, `Tiberium=yes`, `Bright=yes`, `ProneDamage=50%`
- `InfDeath=2`

#### 3.7.2 `[APSplash]` — Osprey depth charge

```ini
[APSplash]; for units whose missiles are having trouble hitting
CellSpread=.5
PercentAtMax=.8
Wall=yes
Wood=yes
Verses=25%,25%,25%,75%,100%,100%,65%,65%,60%,25%,100%
Conventional=yes
InfDeath=3
AnimList=S_CLSN16,S_CLSN22,S_CLSN30,S_CLSN42,S_CLSN58
ProneDamage=50%
```

- `CellSpread=.5` / `PercentAtMax=.8` — small but uniform-ish splash.
- `Verses=25/25/25/75/100/100/65/65/60/25/100`:
  - vs infantry classes: 25 % (poor — anti-sub charges aren't anti-personnel)
  - vs light vehicle: 75 %
  - vs medium/heavy: **100 %** (excellent vs submarines, which fall into medium-armor class)
  - vs structures: 65/65/60 %
  - vs special_1: 25 % (immune-ish)
  - vs special_2: 100 %
- `InfDeath=3` (explosion).

The inline comment "for units whose missiles are having trouble hitting" is the original author's wry note about lobbing-aircraft accuracy issues.

#### 3.7.3 `[AP]` — Osprey crash collision

See [`CARRIER.md`](./CARRIER.md) §3.5.3. Same warhead used by the Hornet's crash collision.

---

## 4. Voice & sound catalogue

| Slot | Sound key | sndmd entry | Audio clip(s) |
|------|-----------|-------------|---------------|
| `VoiceSelect` | `GenAllWaterSelect` | sound:4176 | `$vwaasea-d` random (4 clips, generic Allied water-unit) |
| `VoiceMove` | `GenAllWaterMove` | sound:4171 | `$vwaamoa-e` random (5 clips) |
| `VoiceAttack` | `GenAllWaterAttackCommand` | sound:4166 | `$vwaaata-c` random (3 clips) |
| `VoiceFeedback` | (empty) | — | — |
| `DieSound` | (empty) | — | — (SinkingSound covers) |
| `SinkingSound` | `GenLargeWaterDie` | sound:1979 | `gnavsina` |
| `MoveSound` | `DestroyerMoveStart` | sound:1385 | `vdesstaa/b` random predelay 0-400, Priority=Low, Volume 30 |
| **155mm Report** | `DestroyerAttack` | sound:1379 | `vdesatta/b` random, FShift -10/+10, VShift 15 |
| **Osprey:** `CrashingSound` | `OspreyDie` | sound:1728 | `vospdiea` Priority=low, Volume 50 |
| **Osprey:** `ImpactLandSound` | `GenAircraftCrash` | sound:1995 | `vaircraa-c` random |
| **Osprey:** `AuxSound1` | `OspreyTakeOff` | sound:1716 | `vospstaa` Priority=low, Volume 35 |
| **Osprey:** `AuxSound2` | `OspreyLanding` | sound:1722 | `vosplana` Priority=low, Volume 35 |
| **ASWBomb Report** | `OspreyAttack` | sound:1709 | `vospatta` interrupt, FShift -10/+10, Limit 3, VShift 10 |
| **ASWCollision Report** | `OspreyCollision` | sound:1733 | (silent — `Volume=0`) |

Destroyer uses **generic Allied** water voices (`$vwaa*`), not unit-unique like CARRIER's `$vair*`. Shared with Dolphin, Aegis, Hydrofoil.

---

## 5. Owners / prereqs / tech gating

- **Buildable by:** all 5 Allied countries.
- **Prerequisite:** `GAYARD` only — Naval Yard alone. No tech-lab gate. **The Destroyer is available the moment a player has a Naval Yard.**
- **TechLevel:** 4 — mid-game tier 2. Substantially earlier than Carrier (7).
- **Cost:** 1000 (half of Carrier/Dread).
- `AllowedToStartInMultiplayer=no` → not pre-built.

---

## 6. Veterancy

| Rank | Effect |
|------|--------|
| Rookie | Base — 155mm Burst=1, ARTYHE warhead, ASW Osprey with ASWBomb (APSplash). |
| Veteran | `STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — buffs the *Destroyer* (+HP, +damage, +ROF, +sight, +speed). 155mm gains the standard veteran damage multiplier but stays Burst=1. **No swap of Osprey weapon.** |
| Elite | `SELF_HEAL,STRONGER,FIREPOWER,ROF` + **`ElitePrimary=155mmE` swaps Primary to gain `Burst=2`** — effective DPS doubles on the 155mm. Self-heal added. **Osprey is NOT upgraded (it has no ElitePrimary).** |

> Same pattern as Carrier: parent rank improves the ship and its real weapons, but the spawned aircraft has its own (or in Osprey's case, *no*) veterancy track.

---

## 7. Hardcoded behavior — Ghidra-verified

### 7.1 String-name scan: NO hardcoded behavior keyed to DEST or ASW

- `search_strings "DEST"` returned 39 matches — **all are unrelated:** Windows API names (`HeapDestroy`, `DestroyWindow`, `ImageList_Destroy`), C++ RTTI BlitTransRemap class names, and the pathfinder zone enum strings (`Destroyer`, `InfantryDestroyer`, `AmphibiousDestroyer` — these are `MovementZone=` enum values for ground MBT-class units, NOT the DEST unit ID). **No DEST-unit-specific references in the binary.**
- `search_strings "DESTWO"` → 0 matches (NoSpawnAlt logic builds the name at runtime).
- `search_strings "ASW"` → not searched (similar generic-name expectation; the warhead/projectile sections show no name-specific code paths exist for ASW either).

**Bottom line:** All Destroyer / Osprey behavior is INI-driven through generic systems (SpawnManager, AircraftClass, ShipLocomotion, standard direct-fire WeaponType handling, Sensors detection generic).

### 7.2 Verified field scopes (key ones for this unit)

| Field | Scope | Address |
|-------|-------|---------|
| `Sensors=yes` | TechnoType | 0x00714003 (NEW THIS DOC) |
| `SensorsSight=8` | TechnoType | 0x007142e8 (NEW THIS DOC) |
| `NavalTargeting=1` (on DEST) and `=2` (on ASW) | TechnoType | 0x007121be |
| `ElitePrimary=155mmE` | TechnoType | 0x00712a32 |
| `OpportunityFire=yes` | TechnoType | 0x0071483d |
| `NoSpawnAlt=yes` | ObjectType (broader) | 0x005f943e |
| `Spawns/SpawnsNumber/SpawnRegenRate/SpawnReloadRate/Spawned/MissileSpawn` | TechnoType | cheat-sheet refs |
| `FireAngle=32` | TechnoType | 0x00714b5d |
| `ToProtect=yes` | TechnoType | 0x00714be8 |
| `SinkingSound/ImpactLandSound` | DUAL-READ Rules+TechnoType | 0x00669965 (and ~0x006699a7) + TechnoType per-unit |
| `CrashingSound` | TechnoType | 0x00712f80 |
| `PitchSpeed/PitchAngle` | TechnoType | 0x007123da / 0x0071236b |
| `PipScale=Ammo` | TechnoType | 0x0071411a |
| `MovementRestrictedTo=Water` | UnitType | 0x00747837 |
| `ImmuneToPsionics=yes` | TechnoType | 0x00714fa7 |
| `TooBigToFitUnderBridge=true` | UnitType | 0x0074774e |
| Locomotor CLSID `{2BEA74E1-...}` = ShipLocomotionClass | live YR | — |
| Locomotor CLSID `{4A582746-...}` = DriveLocomotionClass-Air | live YR | — |

### 7.3 Submarine detection mechanic (`Sensors=yes`)

The Destroyer is the **standard counter to Soviet Typhoon Subs and Yuri Boomer Subs**. Mechanic:

1. Both subs have some form of cloaked/underwater visibility — they're hidden on the minimap and tactical view at distance.
2. The Destroyer's `Sensors=yes` flag, combined with `SensorsSight=8`, reveals any cloaked underwater unit within 8 cells of the Destroyer.
3. Once revealed, friendly units (the Destroyer itself, allied Aegis Cruisers, Dolphins, anything else with anti-sub weapons) can target the sub.
4. The Destroyer's Osprey can then drop depth charges with the 100 %-vs-medium-armor APSplash warhead.

> Sensors is **not** a TS holdover — verified live in YR and used by exactly this role-niche (anti-sub escorts). The only YR units with `Sensors=yes` are DEST (verified) and a handful of structures/units that need cloak-piercing.

### 7.4 The Dual-Weapon System (Primary + Secondary)

Many units have only Primary; the Destroyer is one of the few with BOTH Primary AND Secondary actively used. The targeting logic (generic WeaponType selection):
1. Player issues attack-target command.
2. Engine checks the target's properties:
   - Surface unit / structure / ground → **Primary (155mm)**
   - Submarine / underwater target → **Secondary (ASWLauncher → spawn Osprey)**
3. Auto-acquire: if `CanPassiveAquire` (not set on DEST, so defaults to yes), the destroyer scans for targets and picks the appropriate weapon by target type.

The `NavalTargeting=1` enum value selects this profile from the table in rulesmd comment block (line ~3691):
```
NAVAL_SUBPRIMARY=1 — Able to shoot ground/surface targets with primary weapon, but underwater targets get the secondary
```

---

## 8. TS-legacy filter

| Feature | Status in YR |
|---------|--------------|
| `;Lobber=yes` on 155mm | INI comment — disabled. The cannon is direct-fire arcing. |
| `;Range=-2` on HornetLauncher (commented but ASWLauncher uses it live) | Live on ASWLauncher (infinite range Osprey dispatch). |
| `;AntiUnderwater=yes` on ASWLauncher | INI comment — would set a flag; disabled, but warhead/targeting handles anti-sub via Verses + NavalTargeting. |
| `;Dock=GAAIRC,AMRADR` on ASW | INI comment — Osprey docks on the destroyer, not at airpads. |
| `;AS=yes` / `;AN=yes` on ASWVirt | INI comment — would have been "anti-sub" / "anti-naval" projectile flags. Disabled. |
| `;Selectable=no` on ASW | INI comment — author wanted non-selectable but bug. |
| `Conventional=yes` on warheads | Live in YR. |
| `Tiberium=yes` on ARTYHE | TS-holdover terminology; in YR drives ore-cluster chain. |
| Fog-of-war 0x1000 | Not applicable. |
| ImmuneToVeins / Subterranean / Tunneling | Not on DEST/ASW. |

---

## 9. Coverage audit

| Section | Coverage |
|---------|----------|
| rulesmd `[DEST]` — every key | ✅ §1 (47 keys + 2 commented) |
| artmd `[DEST]` — every key | ✅ §2 (4 keys) |
| `[155mm]` weapon | ✅ §3.1 (11 keys + 1 commented Lobber) |
| `[155mmE]` weapon (elite) | ✅ §3.5 |
| `[ASWLauncher]` weapon | ✅ §3.2 (9 keys + 1 commented AntiUnderwater) |
| `[ASW]` aircraft rulesmd | ✅ §3.3 (37 keys + 2 commented Dock/Selectable) |
| `[ASW]` artmd | ✅ §3.3.1 (3 keys) |
| `[ASWBomb]` weapon | ✅ §3.4 |
| `[ASWCollision]` weapon | ✅ §3.4 |
| `[Ballistic]`, `[DepthCharge]`, `[ASWVirt]` projectiles | ✅ §3.6 |
| `[ARTYHE]`, `[APSplash]`, `[AP]` warheads | ✅ §3.7 (APSplash detailed; others cross-ref'd to CARRIER doc) |
| Voices/sounds (14 slots) | ✅ §4 |
| Owners/prereqs/tech | ✅ §5 |
| Veterancy (Destroyer rank + Osprey lack-of-rank) | ✅ §6 |
| Hardcoded behavior — Ghidra-verified | ✅ §7 (DEST/DESTWO name scans both returned 0 unit-related matches; **2 NEW field-scope verifications: Sensors @ 0x00714003 + SensorsSight @ 0x007142e8**, both TechnoType. Cross-ref'd 17 keys to cheat sheet.) |
| TS-legacy filter | ✅ §8 |
| Spawner-family comparison table | ✅ at top |
| Sensors/submarine detection mechanic | ✅ §7.3 |
| Dual-weapon Primary/Secondary system | ✅ §7.4 |

---

## Ghidra audit log (audit iteration 21 — 2026-05-18)

**Methodology**: DEST is mid-density — 4 NEW field-scope claims
(Sensors, SensorsSight, NoSpawnAlt, NavalTargeting) + dual-weapon
system + spawn-family reuse from CARRIER (audit 20). This audit
verifies all 4 NEW scopes, decompiles `ObjectTypeClass__ReadINI` in
full (which had been treated as "auditable via grep" in audit 7), and
discovers 6+ NEW ObjectType offsets as a bonus. ~13 Ghidra queries: 6
string searches + 4 xref lookups + 1 grep + 1 full
ObjectTypeClass__ReadINI decompile.

### Negative claims re-verified

| Query | Result |
|-------|--------|
| `search_strings("^DESTWO$")` | **0 matches** (NoSpawnAlt name built at runtime) |
| `search_strings("^ASW$")` | **0 matches** |

Confirms doc's claim: no hardcoded DEST / DESTWO / ASW-specific code.

### String + parser xref verification (BINARY-VERIFIED)

All 4 NEW doc-cited claims verify exactly:

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `Sensors` | 0x00843E58 | 0x00714003 | TechnoTypeClass__ReadINI |
| `SensorsSight` | 0x00843D50 | 0x007142E8 | TechnoTypeClass__ReadINI |
| `NoSpawnAlt` | 0x00832BC0 | 0x005F943E | **ObjectTypeClass__ReadINI** (broader-than-TechnoType scope) |
| `NavalTargeting` | 0x00844510 | 0x007121BE | TechnoTypeClass__ReadINI |

Bonus: `Sensors` string has TWO copies in the binary — lowercase
`Sensors` at 0x00843E58 (the INI-key string consumed by parser) and
uppercase `SENSORS` at 0x00846430 (likely a separate constant — perhaps
a debug-print enum or vtable label).

### NEW TechnoType offsets BINARY-VERIFIED

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0xC9D` | `Sensors` | byte | `*(char*)((int)param_1 + 0xC9D) = (char)uVar5` after ReadBool. **NEW**. Live in YR for sub-detection role. |
| `+0x5F0` | `SensorsSight` | int (cells) | `param_1[0x17C] = iVar4`. **NEW**. Adjacent to audit-6 `+0x5F4 = DetectDisguiseRange` — confirms a "detection-range cluster" at +0x5F0..+0x5F8 in TechnoType. |
| `+0x600` | `NavalTargeting` | int (enum) | `param_1[0x180] = iVar4`. **Re-confirms audit 7 cumulative**. Enum values: 1=NAVAL_SUBPRIMARY (DEST), 2=NAVAL_PRIMARY (Osprey), per rulesmd comment block. |

### NEW function entry: `ObjectTypeClass__ReadINI` fully decompiled

| Function | Entry | Body | Status |
|----------|-------|------|--------|
| `ObjectTypeClass__ReadINI` | `0x005F92D0` | (per cumulative cheat-sheet, audit 7 had verified this range) | **Fully decompiled this pass** — sole parser for ObjectType-scope keys (the broadest layer above TechnoType / InfantryType / UnitType / BuildingType / AnimType / TerrainType). |

### NEW ObjectType offsets BINARY-VERIFIED (from this decompile)

Audit 7 had pinned several ObjectType offsets via ObjectTypeClass__ReadINI grep. The full decompile this pass adds many more:

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0x1E8` | `NoSpawnAlt` | byte | `*(undefined1*)(param_1 + 0x7A) = uVar2`. **NEW**. Causes voxel swap to `<UnitID>WO` (e.g., DESTWO) when SpawnManager has no spawns out. |
| `+0x1F0` | `CrushSound` | int (VocClass index) | `param_1[0x7C]` after VocClass__FindByName. **NEW**. (ObjectType-scope, inherited by all unit-class types.) |
| `+0x1F4` | `AmbientSound` | int (VocClass index) | `param_1[0x7D]`. **NEW**. |
| `+0x9C` | `Armor` | int (enum) | `param_1[0x27] = iVar4` after FUN_004753F0 (= Armor-enum-lookup helper). **NEW** — confirms ObjectType-scope (above TechnoType). |
| `+0x22C` | `Theater` | byte | `*(undefined1*)(param_1 + 0x8B)`. **NEW**. |
| `+0x230` | `Selectable` | byte | `*(undefined1*)(param_1 + 0x8C)`. **NEW**. |
| `+0x22F` | `RadarInvisible` | byte | `*(undefined1*)((int)param_1 + 0x22F)`. **NEW**. |
| `+0x238` | `HasRadialIndicator` | byte | `*(undefined1*)(param_1 + 0x8E)`. **NEW**. |
| `+0x98..+0x9A` | `RadialColor` | RGB (short+byte) | `*(undefined2*)(param_1 + 0x26)` + `*(undefined1*)((int)param_1 + 0x9A)`. **NEW**. |
| `+0x23B..+0x23D` | `LineTrailColor` | RGB (short+byte) | adjacent to +0x23A UseLineTrail. **NEW** (extends audit-7 partial). |
| `+0x240` | `LineTrailColorDecrement` | int | `param_1[0x90]`. **NEW**. |
| `+0x211` | `AlternateArcticArt` | byte | **NEW**. |
| `+0x213` | `AlphaImage` | char[25] string | **NEW**. |
| `+0x7E` | `Image` | char[25] string | **NEW** (ObjectType-scope Image= field — different layer than TechnoType `Image=` redirect). |

### NEW ObjectType-scope distinction discovery

The audit 14 (AMCV) found `Image=MCV` is a TechnoType-scope mechanism;
audit 15 (MTNK) confirmed `Image=GTNK`. But this decompile reveals that
**ObjectType ALSO has an `Image=` field at +0x7E** (parsed in
ObjectTypeClass__ReadINI). These are the same field — ObjectType is the
parent of TechnoType, so the parse-and-store at ObjectType-level happens
once, and TechnoType subclasses inherit. Resolves the prior
DEFERRED-question about which layer parses `Image=`.

### Spawn-family cluster reuse (re-confirms audit 20)

DEST uses the spawn-family cluster pinned in audit 20:
- `+0xD58 = Spawns` (= ASW Osprey TechnoType ptr)
- `+0xD5C = SpawnsNumber` (= 1)
- `+0xD60 = SpawnRegenRate` (= 400)
- `+0xD64 = SpawnReloadRate` (= 150)
- `+0x3D0 = FireAngle` (= 32)

Not re-decompiled this pass; cross-referenced via cumulative cheat-sheet.

### Items NOT re-verified in this pass (DEFERRED)

- 17+ other doc-cited parser xrefs (cross-referenced to cheat sheet).
- `;Lobber=yes` / `;AntiUnderwater=yes` etc. INI-commented lines (the
  scope of `Lobber` and `AntiUnderwater` is documented but the parser
  isn't re-verified — these are inert anyway).
- The Sensors-revealing-sub-cloak consumer chain in
  `DisplayClass::DrawIt` or similar render code.
- The dual-weapon Primary-vs-Secondary selection logic (chooses 155mm
  vs ASWLauncher based on target).
- The `NavalTargeting` enum at +0x600 — confirmed as int storage, but
  the consumer-side targeting-priority logic NOT traced this pass.

### Confidence summary

- **HIGH**: 6 string addresses + 4 parser xrefs (all exact); 3 NEW
  TechnoType struct offsets (Sensors +0xC9D, SensorsSight +0x5F0,
  NavalTargeting +0x600 re-confirms); 1 NEW function fully decompiled
  (ObjectTypeClass__ReadINI); 13+ NEW ObjectType offsets — significant
  cumulative addition since ObjectType is the parent layer for ALL
  unit/structure/animation/terrain types.
- **MEDIUM**: NavalTargeting enum value (1, 2, etc.) semantic
  inferred from rulesmd comment block — not Ghidra-verified at
  consumer side.
- **No INCORRECT findings**. Doc's 4 NEW field-scope claims all verify
  exactly. The NoSpawnAlt-is-ObjectType-broader observation (doc §1.1)
  is BINARY-VERIFIED.

---

## 10. Quick implementer summary

To make a DEST-equivalent:

1. **Render** — voxel + HVA, with NoSpawnAlt swap to `<ID>WO` voxel when SpawnManager.RemainingSpawns==0 (visibly empty deck).
2. **Movement** — ShipLocomotionClass (water, Speed=6 fast, ROT=5 agile, TooBigToFitUnderBridge gate).
3. **Spawner** — generic SpawnManager with 1 slot, regen 400, reload 150.
4. **Osprey flight** — same as Hornet (DriveLocomotionClass-Air, MovementRestrictedTo=Water docks on destroyer); but with APSplash anti-sub bomb warhead.
5. **Primary weapon** — direct-fire 155mm cannon (8-cell range, ARTYHE warhead). Hull must face target (Turret=no, OmniFire NOT on 155mm).
6. **Secondary weapon** — ASWLauncher spawns Osprey (infinite range, OmniFire). Triggered by submarine targets via NavalTargeting=1 selection.
7. **Sensors** — submarine detection within 8 cells (SensorsSight=8). Reveals cloaked underwater units to all friendly units.
8. **Veterancy** — Destroyer rank improves hull + 155mm DPS (ElitePrimary=155mmE adds Burst=2 at elite). Osprey has no rank progression.
9. **Audio** — generic Allied water voice set + DestroyerMoveStart + DestroyerAttack on cannon fire + full Osprey audio set (takeoff/landing/die/crash).
10. **AI** — ToProtect=yes (escort), ThreatPosed=15 (Destroyer; lower than Carrier/Dread). OpportunityFire=yes only meaningful for Osprey-spawn during transit.
11. **Build gate** — Naval Yard only (GAYARD), TechLevel=4 — earlier than the other spawners.

No DEST or ASW-specific code paths needed — generic SpawnManager + AircraftClass + ShipLocomotion + standard WeaponType + Sensors detection covers all behavior.
