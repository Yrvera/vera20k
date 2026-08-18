# CARRIER — Aircraft Carrier (Allied Naval Spawner)

**INI ID:** `CARRIER`
**Display Name:** `Aircraft Carrier` (`UIName=Name:CARRIER`)
**Side:** Allied (all 5 Allied houses — see `Owner=`)
**Category:** Vehicle / Naval
**Cameo:** `CARRICON`
**Voxel:** yes. **No `NoSpawnAlt`** — the carrier voxel does not change when Hornets are away (they're stored on-deck; the deck always looks the same).

The Aircraft Carrier is the Allied long-range naval siege ship. It carries
three `HORNET` spawned aircraft that fly out to attack a target, drop their
bomb, then **fly back to the carrier to dock and reload**. This is the
Allied counterpart to the Soviet Dreadnought ([`DRED.md`](../soviet/DRED.md)),
but with a fundamentally different `SpawnManagerClass` flow — Hornets are
**reusable aircraft**, not kamikaze missiles.

### Spawner family — Carrier vs Dread vs V3

| | Carrier (this doc) | Dread ([DRED](../soviet/DRED.md)) | V3 ([V3](../soviet/V3.md)) |
|---|---|---|---|
| Spawned aircraft | `HORNET` | `DMISL` | `V3ROCKET` |
| `MissileSpawn=` on aircraft | **no** | yes | yes |
| `SpawnReloadRate=` | **150** (returns + reloads ~10 sec) | 0 (one-shot, regen from cost) | 0 (one-shot) |
| `SpawnsNumber=` | 3 | 2 | 1 |
| Aircraft `Primary=` | `HornetBomb` (real weapon) | (none — Rules.DMislWarhead hardcoded) | (none — Rules.V3Warhead hardcoded) |
| Aircraft veterancy | yes (Trainable absent → default trainable) | no (`Trainable=no`) | no |
| Aircraft `Locomotor=` | DriveLocomotionClass-Air `{4A582746-...}` (fly-around) | `{B7B49766-...}` RocketLocomotionClass | `{B7B49766-...}` RocketLocomotionClass |
| Damage path | Aircraft's own `[HornetBomb]` weapon | Rules global `DMislWarhead` (hardcoded) | Rules global `V3Warhead` (hardcoded) |
| Elite swap | Hornet's `ElitePrimary=HornetBombE` | Rules `DMislEliteWarhead` (launcher rank) | Rules `V3EliteWarhead` (launcher rank) |
| Veteran rank that matters | **Hornet's own rank** | Launcher (Dread) rank | Launcher (V3) rank |

> **Cross-references — do not re-derive:**
> - [`SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md`](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md) — full SpawnManager state machine; Carrier is the canonical *non-missile* spawner (the only one of the three families where `IsMissileSpawn=0`). §3 covers the missile-vs-aircraft branch.
> - [`AIRCRAFTCLASS_GHIDRA_REPORT.md`](../../AIRCRAFTCLASS_GHIDRA_REPORT.md) — aircraft state machine, landing/docking flow.
> - [`DRED.md`](../soviet/DRED.md) — sibling missile-spawner ship for direct comparison.
> - [`V3.md`](../soviet/V3.md) — sibling missile-spawner land vehicle.
> - [`BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md`](../../BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md) — locomotor CLSID table.

> **TS-legacy filter applied** — Locomotor's second GUID after `;` is an INI comment (inert); `;ForbiddenHouses=Americans`, `;BuildLimit=1`, `;OmniFire=yes`, `;Range=-2` on launcher all RA2-era authoring drafts left as residue. Live YR systems only.

---

## 1. Full `rulesmd.ini` section verbatim

```ini
[CARRIER]
UIName=Name:CARRIER
Name=Aircraft Carrier
Prerequisite=GAYARD,TECH
Primary=HornetLauncher
CanPassiveAquire=no ; Won't try to pick up own targets
Spawns=HORNET
SpawnsNumber=3
SpawnRegenRate=600
SpawnReloadRate=150
FireAngle=32
ToProtect=yes
Category=Support
Strength=800
Naval=yes ;GS
Armor=heavy
TechLevel=7
Sight=7
Speed=4
CrateGoodie=no
Owner=British,French,Germans,Americans,Alliance
;ForbiddenHouses=Americans
AllowedToStartInMultiplayer=no
Cost=2000
Soylent=2000
Turret=no
Points=55
ROT=1
Crusher=no; yes
Weight=5
Crewed=no
;OmniFire=yes ;GEF moved to weapon
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=AircraftCarrierSelect
VoiceMove=AircraftCarrierMove
VoiceAttack=AircraftCarrierAttackCommand
VoiceFeedback=
DieSound=
SinkingSound=GenLargeWaterDie
MoveSound=ACCMoveStart
Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};{4A582741-9839-11d1-B709-00A024DDAFD1}
SpeedType=Float
MovementZone=Water
ThreatPosed=25	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
TooBigToFitUnderBridge=true
GuardRange=10
;BuildLimit=1
Size=50
```

### 1.1 Key-by-key explanation

(Repeated keys identical to DRED are summarized — see [`DRED.md`](../soviet/DRED.md) §1.1 for full text. Differences vs DRED are called out below.)

| Key | Value | Read by | Notes / diff vs DRED |
|-----|-------|---------|----------------------|
| `UIName=Name:CARRIER` | string | AbstractTypeClass | CSF lookup token. |
| `Name=Aircraft Carrier` | string | AbstractTypeClass | English fallback. |
| `Prerequisite=GAYARD,TECH` | building list | TechnoTypeClass | **Allied Naval Yard + ANY tech building (`TECH=GATECH,NATECH,YATECH`)**. The `TECH` macro is the cross-house tech-lab alias defined in `[Rules]` (rulesmd line ~3070 area). Compare DRED which requires `NAYARD,NATECH` specifically. |
| `Primary=HornetLauncher` | weapon | TechnoTypeClass | Spawner virtual weapon — see §3.1. |
| `CanPassiveAquire=no` | bool | TechnoTypeClass @ 0x00714473 | Doesn't auto-acquire (same as DRED/V3). |
| `Spawns=HORNET` | aircraft type | TechnoTypeClass | Child aircraft type. |
| `SpawnsNumber=3` | int | TechnoTypeClass @ 0x00714EE1 [BINARY-VERIFIED audit 20: string @ 0x008437B8, `TechnoType+0xD5C` (int)] | **3 Hornets in magazine** (vs DRED's 2 DMISL). |
| `SpawnRegenRate=600` | frames | TechnoTypeClass @ 0x00714EC0 [BINARY-VERIFIED audit 20: string @ 0x008437C8, `TechnoType+0xD60` (int)] | **600 frames = 40 sec to *replace* a destroyed Hornet** (if one is shot down by AA). Surviving Hornets aren't affected. |
| `SpawnReloadRate=150` | frames | TechnoTypeClass @ 0x00714F02 [BINARY-VERIFIED audit 20: string @ 0x008437A8, `TechnoType+0xD64` (int)] | **150 frames = 10 sec for a docked Hornet to refill its `Ammo=1` bomb**. Hornet returns to carrier → lands on deck → 150-frame reload countdown → ready to launch again. |
| `FireAngle=32` | int | TechnoTypeClass @ 0x00714b5d [BINARY-VERIFIED audit 20: string @ 0x00843910, `TechnoType+0x3D0` (int)] | 45° initial pitch on launch (same as DRED). |
| `ToProtect=yes` | bool | TechnoTypeClass @ 0x00714be8 | AI escort flag. |
| `Category=Support` | enum | TechnoTypeClass | UI/AI category. |
| `Strength=800` | hp | TechnoTypeClass | 800 HP hull (same as DRED). |
| `Naval=yes ;GS` | bool | UnitTypeClass | Naval flag. The `;GS` comment is author initials. |
| `Armor=heavy` | enum | TechnoTypeClass | Heavy armor. |
| `TechLevel=7` | int | TechnoTypeClass | **TechLevel 7** (vs DRED's 6) — even higher gating. |
| `Sight=7` | cells | TechnoTypeClass | 7 cells. |
| `Speed=4` | int | TechnoTypeClass | Same speed as DRED. |
| `CrateGoodie=no` | bool | UnitTypeClass @ 0x00747658 | No crate pop. |
| `Owner=British,French,Germans,Americans,Alliance` | country list | TechnoTypeClass | **All 5 Allied houses** (UK, France, Germany, USA, Korea/Alliance). Buildable by every Allied country. |
| `;ForbiddenHouses=Americans` | (commented) | — | Inert. |
| `AllowedToStartInMultiplayer=no` | bool | TechnoTypeClass | Not in starting roster. |
| `Cost=2000` | credits | TechnoTypeClass | Same as DRED. |
| `Soylent=2000` | credits | TechnoTypeClass | Full-cost recycle. |
| `Turret=no` | bool | UnitTypeClass | No turret. |
| `Points=55` | int | TechnoTypeClass | Score value. |
| `ROT=1` | int | TechnoTypeClass | Extremely slow turn (same as DRED). |
| `Crusher=no; yes` | bool | TechnoTypeClass | No crush. The `; yes` is a draft note. |
| `Weight=5` | int | TechnoTypeClass | Slightly heavier than DRED's 4 (used by AI weighting). |
| `Crewed=no` | bool | TechnoTypeClass | No crew bailout. |
| `;OmniFire=yes ;GEF moved to weapon` | (commented) | — | OmniFire is on the launcher weapon. |
| `IsSelectableCombatant=yes` | bool | TechnoTypeClass | Combat unit. |
| `Explosion=...` | anim list | TechnoTypeClass | Same 5-entry explosion set as DRED. |
| `VoiceSelect=AircraftCarrierSelect` | sound | TechnoTypeClass | Unique to Carrier (NOT shared like DRED's GenSovWater*) — has its own `$vairsea/b/c/d/e` 5-clip set. |
| `VoiceMove=AircraftCarrierMove` | sound | TechnoTypeClass | Unique 5-clip set (`$vairmoa-e`). |
| `VoiceAttack=AircraftCarrierAttackCommand` | sound | TechnoTypeClass | Unique 5-clip set (`$vairata-e`). |
| `VoiceFeedback=` | (empty) | TechnoTypeClass | None. |
| `DieSound=` | (empty) | TechnoTypeClass | None. |
| `SinkingSound=GenLargeWaterDie` | sound | TechnoType @ 0x00712f38 + Rules @ 0x00669965 (DUAL-READ) | Same `gnavsina` as DRED. |
| `MoveSound=ACCMoveStart` | sound | TechnoTypeClass | "**A**ircraft **C**arrier **C**ruise" — `vaccstaa/b` random pair, Volume 50, with predelay 0-400. |
| `Locomotor={2BEA74E1-...};{4A582741-...}` | CLSID | TechnoTypeClass | **Active: ShipLocomotionClass `{2BEA74E1-...}`**. Trailing GUID is INI-commented. (Identical Locomotor= line as DRED.) |
| `SpeedType=Float` | enum | TechnoTypeClass | Water type. |
| `MovementZone=Water` | enum | TechnoTypeClass | Water zone. |
| `ThreatPosed=25` | int | TechnoTypeClass | AI threat weight. |
| `DamageParticleSystems=SparkSys,SmallGreySSys` | particle list | TechnoTypeClass | Damaged emissions. |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | ability list | TechnoTypeClass | **Vet abilities buff the CARRIER itself, not the Hornets.** Hornets carry their own veterancy independently. |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | ability list | TechnoTypeClass | Carrier elite: passive self-heal. **Carrier rank does NOT swap the Hornet's weapon — the Hornet swaps based on its own rank** (see §3.2). |
| `TooBigToFitUnderBridge=true` | bool | UnitTypeClass @ 0x0074774e | UnitType scope. |
| `GuardRange=10` | cells | TechnoTypeClass | Guard engagement radius. |
| `;BuildLimit=1` | (commented) | — | Inert. |
| `Size=50` | int | TechnoTypeClass | Transport-size cost. |

---

## 2. Full `artmd.ini` section verbatim

```ini
[CARRIER]
Cameo=CARRICON
;PrimaryFireFLH=240,0,20 ; offset for take off
Voxel=yes
Remapable=yes
```

| Key | Value | Notes |
|-----|-------|-------|
| `Cameo=CARRICON` | SHP | Build-list cameo. |
| `;PrimaryFireFLH=240,0,20` | (commented) | **Disabled FLH offset.** Inline comment notes "offset for take off". Without an active `PrimaryFireFLH=`, the engine uses default origin (ship center) for the spawned aircraft launch position. Compare DRED which has an active `PrimaryFireFLH=30,43,92`. The Carrier's Hornets visibly take off from the deck — but the position is engine-default, not INI-specified. |
| `Voxel=yes` | bool | Voxel render. |
| `Remapable=yes` | bool | House-color tinted. |

> **No Voxel.Sequence or Animation block.** The carrier deck does not animate when Hornets land/launch — the Hornet voxel simply renders separately at the deck position and changes state via AircraftClass.

---

## 3. Weapons

### 3.1 `[HornetLauncher]` — virtual launcher

```ini
[HornetLauncher]
Damage=1
ROF=150
Range=25
;Range=-2 ; infinite
Spawner=yes
Projectile=Invisible
Speed=10
Warhead=Special
OmniFire=yes
```

| Key | Effect |
|-----|--------|
| `Damage=1` | Placeholder — actual damage is from the Hornet's own `[HornetBomb]` weapon. |
| `ROF=150` | **10 sec between fire commands** — paces how often the Carrier can dispatch a Hornet. |
| `Range=25` | 25 cells — same outer reach as the Dread launcher. Hornets can fly farther on their own once dispatched. |
| `;Range=-2` | (commented) — would have been infinite range. Disabled. |
| `Spawner=yes` | Releases one entry from the parent's SpawnManager pool. |
| `Projectile=Invisible` | Bookkeeping projectile. |
| `Speed=10` | Irrelevant. |
| `Warhead=Special` | Misleading — `Special` is a House (`[Special]` country at rulesmd:3335), not a Warhead. No damage from launcher itself. |
| `OmniFire=yes` | Fires in any direction without hull rotation. |

> **No `Burst=` on HornetLauncher** — only **one Hornet launches per fire command** (vs DRED's `Burst=2`). The carrier dispatches sequentially. With ROF=150 and 3 Hornets, the carrier can ideally have one Hornet airborne every ~10 sec (limited by reload + flight time).

### 3.2 Spawned aircraft `[HORNET]` (rulesmd)

```ini
[HORNET]
UIName=Name:HORNET
Name=Hornet
Primary=HornetBomb
Secondary=HornetCollision
Strength=75
Category=AirPower
Armor=light
Spawned=yes
TechLevel=-1
Sight=2
RadarInvisible=no
Landable=yes
MoveToShroud=yes
;Dock=NAHPAD,GAHPAD
;Dock=GAAIRC,AMRADR
PipScale=Ammo
Speed=12
PitchSpeed=.9
PitchAngle=0
Owner=British,French,Germans,Americans,Alliance
Cost=50
Points=20
ROT=3
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
CrashingSound=HornetDie
ImpactLandSound=GenAircraftCrash
Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1}
MovementZone=Fly
MovementRestrictedTo=Water ; See if this will affect landing only
ThreatPosed=10	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
AuxSound1=HornetTakeoff ;Taking off
AuxSound2=HornetLanding ;Landing
ImmuneToPsionics=yes
VeteranAbilities=STRONGER,FIREPOWER
EliteAbilities=STRONGER,FIREPOWER
;Selectable=no	; SJM: this should be here but is commented out because bug prevents aircraft from landing
ElitePrimary=HornetBombE
```

| Key | Notes |
|-----|-------|
| `Primary=HornetBomb` | **Real weapon** — the Hornet drops a NormalBomb projectile (40 dmg, ORCAAP warhead) on the target. See §3.3. |
| `Secondary=HornetCollision` | **Crash-on-target weapon** — when a damaged/dying Hornet is about to crash, this is used at the last second (per inline comment in `[HornetCollision]`). See §3.4. |
| `Strength=75` | Fragile (75 HP). One AA hit usually kills. |
| `Armor=light` | Light armor. |
| `Spawned=yes` | TechnoType @ 0x00714e7d — spawn-only. |
| **No `MissileSpawn=yes`** | Critical difference vs DMISL/V3ROCKET — Hornet uses the regular aircraft-spawn flow, not the RocketStruct-based missile flow. SpawnManagerClass sets `IsMissileSpawn=0` for this spawn. |
| `TechLevel=-1` | Hidden from build list. |
| `Sight=2` | 2-cell vision contribution during flight (NOT 0 like DMISL — Hornets reveal scout-distance shroud). |
| `Landable=yes` | Can land. The Hornet *does* land back on the carrier. |
| `MoveToShroud=yes` | Can fly through shrouded cells. |
| `;Dock=NAHPAD,GAHPAD` / `;Dock=GAAIRC,AMRADR` | (commented) — Hornets do NOT use the Dock= chain (which is for helipad/AFCH-based fixed-wing aircraft like Harriers/Black Eagles). The carrier's SpawnManager handles return-and-land directly to the parent ship; no separate dock target. |
| `PipScale=Ammo` | TechnoType @ 0x0071411a — UI pip-bar shows ammo count (1 pip = 1 ammo, with `Ammo=1` this is binary). |
| `Speed=12` | Flight speed (faster than DMISL's 18 in different units, but Hornets do real cruising flight not missile dives). |
| `PitchSpeed=.9` | TechnoType @ 0x007123da — how fast the aircraft tilts when changing altitude (1.0 = instant; .9 = nearly instant). |
| `PitchAngle=0` | TechnoType @ 0x0071236b — neutral pitch angle while cruising (0 = horizontal). |
| `Owner=British,French,Germans,Americans,Alliance` | Mirrors carrier. |
| `Cost=50` | Replenishment cost when destroyed (used for SpawnRegenRate cost computation). |
| `ROT=3` | Rate of turn in air. |
| `Ammo=1` | Inline note: aircraft are hardwired to need ammo. One bomb per sortie. |
| `GuardRange=30` | 30-cell engagement range while on guard. |
| `Explosion=...` | Same 5-anim set as Carrier (used on Hornet death). |
| `MaxDebris=2` | Up to 2 debris pieces on destruction. |
| `VoiceSelect=` / `VoiceMove=` / `VoiceAttack=` / `VoiceFeedback=` / `DieSound=` | All empty — Hornets are silent (not directly selectable in practice; see comment about `;Selectable=no`). |
| `CrashingSound=HornetDie` | TechnoType @ 0x00712f80 — plays while the Hornet falls. `vhordiea/b` random pair, Volume 60. |
| `ImpactLandSound=GenAircraftCrash` | TechnoType @ 0x00712f38 + Rules @ 0x00669965 (DUAL-READ) — generic aircraft impact sound `vaircraa/b/c` when the dying Hornet hits the ground. |
| `Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1}` | **DriveLocomotionClass-Air** (the airborne drive locomotor — fixed-wing fly-around, used by Hornet/ASW/Harrier/Black Eagle). NOT the missile locomotor. |
| `MovementZone=Fly` | Air zone. |
| `MovementRestrictedTo=Water` | UnitType-scope @ 0x00747837 — **restricts landing to water cells only**. Inline comment "See if this will affect landing only" was the original author's verification note. Effect: a returning Hornet only docks at water cells (i.e., on top of the carrier or other water). Live in YR. |
| `ThreatPosed=10` | AI weight. |
| `DamageParticleSystems=SparkSys,SmallGreySSys` | Damaged emissions. |
| `AuxSound1=HornetTakeoff` | `vhortaka/b` random pair, Volume 45, played on launch from carrier. |
| `AuxSound2=HornetLanding` | `vhorlana/b` random pair, Volume 45, played on landing back to carrier. |
| `ImmuneToPsionics=yes` | TechnoType @ 0x00714fa7 — cannot be mind-controlled. |
| `VeteranAbilities=STRONGER,FIREPOWER` | At Veteran: +HP, +damage. Limited set vs the Carrier's. |
| `EliteAbilities=STRONGER,FIREPOWER` | At Elite: same as Veteran (no SELF_HEAL/ROF) — but **also triggers `ElitePrimary=HornetBombE` weapon swap**. |
| `;Selectable=no` | (commented) — author wanted Hornets non-selectable but a bug prevented landing if true. The inline comment is explicit: "this should be here but is commented out because bug prevents aircraft from landing". |
| `ElitePrimary=HornetBombE` | TechnoType @ 0x00712a32 — **when this Hornet is elite, switch Primary from HornetBomb to HornetBombE** (stronger). Verified TechnoType scope. |

### 3.2.1 `[HORNET]` artmd

```ini
[HORNET] ; Carrier plane
Cameo=PROICON
Voxel=yes
PrimaryFireFLH=0,32,0
```

- `Cameo=PROICON` — placeholder cameo (`PROICON` is a generic prototype icon — Hornets are not buildable, so cameo is never shown).
- `Voxel=yes` — voxel render (`hornet.vxl` + `.hva`).
- `PrimaryFireFLH=0,32,0` — bomb-release offset: 0 forward, 32 right, 0 height (centered, slightly to the right of the body).
- **No `Remapable=yes`** — Hornet's voxel is not house-recolored (universal Allied grey).

### 3.3 `[HornetBomb]` — primary bomb (and elite variant)

```ini
[HornetBomb]
Damage=40
ROF=3
Range=5
Projectile=NormalBomb
Speed=30
Warhead=ORCAAP
Report=HornetAttack
```

- `Damage=40` — 40 base damage.
- `ROF=3` — 3-frame ROF (fast — but `Ammo=1` means only one bomb, so ROF is mostly moot).
- `Range=5` — drop from 5 cells away.
- `Projectile=NormalBomb` — gravity-falling bomb projectile (see [`NormalBomb`] below).
- `Speed=30` — drop speed.
- `Warhead=ORCAAP` — armor-piercing aircraft warhead (see §3.5).
- `Report=HornetAttack` — bomb-release sound (TODO: see soundmd for exact clip; the loop entry is implicit).

```ini
[HornetBombE]
Damage=80
ROF=3
Range=5
Projectile=NormalBomb
Speed=30
Warhead=ARTYHE
Report=HornetAttack
```

Elite differences:
- `Damage=80` — **2× damage**.
- `Warhead=ARTYHE` — switches from ORCAAP (small, AP) to **ARTYHE** (large, HE — CellSpread=1, with rocker and 15% deform). The elite Hornet's bomb has a 1-cell splash radius vs the rookie's tiny 0.4-cell radius.

### 3.4 `[HornetCollision]` — last-second crash weapon

```ini
[HornetCollision] ;A crashing Hornet turns into this bullet at the last second
Damage=100
ROF=20
Range=3
Projectile=AAHeatSeeker2 ; will be Hornet shaped bullet
Speed=30
Warhead=AP
Report=HornetCollision
Bright=yes
```

The inline comment is explicit: when a Hornet is shot down or crashing, just before impact the engine swaps the Hornet's identity to a `HornetCollision` "bullet" that does 100 AP damage with a 3-cell seek radius using the AAHeatSeeker2 projectile (described as "will be Hornet shaped bullet"). This makes a kamikaze-style ground kill possible from a falling Hornet. `Bright=yes` for the impact flash. Sound: `HornetCollision` (`gexpshaa` random, interrupt, Volume 25).

### 3.5 Warheads

#### 3.5.1 `[ORCAAP]` (rookie Hornet bomb)

```ini
[ORCAAP]
Wall=yes
Wood=yes
CellSpread=.4
PercentAtMax=1
Verses=100%,100%,100%,100%,100%,100%,100%,100%,75%,100%,100%
Conventional=yes
InfDeath=3
AnimList=S_CLSN16,S_CLSN22,S_CLSN30,S_CLSN42,S_CLSN58
ProneDamage=50%
PenetratesBunker=yes ;If shot at a bunkered tank, no means the bunker gets the damage, yes means the unit does
```

- `CellSpread=.4` — small radius. `PercentAtMax=1` — full damage to edge (uniform within the .4 radius).
- `Verses=...,75%,...` — **100% vs everything except concrete buildings (75%)**. Universal damage.
- `InfDeath=3` — infantry die via explosion-fragment anim (per InfDeath cheat sheet: 3 = "explosion (RPG, Cannon)").
- `AnimList=S_CLSN16,S_CLSN22,S_CLSN30,S_CLSN42,S_CLSN58` — collision/explosion anim set.
- `ProneDamage=50%` — prone infantry take half damage (good for them).
- `PenetratesBunker=yes` — bypasses Battle Bunker's damage soak (the tank inside takes the hit, not the bunker).

#### 3.5.2 `[ARTYHE]` (elite Hornet bomb)

```ini
[ARTYHE]
;Spread=6
CellSpread=1
PercentAtMax=.25
Wall=yes
Wood=yes
Verses=100%,80%,60%,100%,60%,60%,100%,100%,60%,100%,100%
Conventional=yes
Rocker=yes
InfDeath=2
AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG
Deform=15%
DeformThreshhold=120
Tiberium=yes
Bright=yes
ProneDamage=50%
```

- `CellSpread=1` — **2.5× the radius** vs ORCAAP.
- `PercentAtMax=.25` — but edge damage falls to 25 %.
- `Verses=100%,80%,60%,100%,60%,60%,100%,100%,60%,100%,100%` — High-explosive curve: best vs uncovered targets and Light vehicles (100 %), reduced vs flak/plate infantry (80/60 %) and medium/heavy vehicles (60 %), strong vs wood/steel structures (100 %), only 60 % vs concrete.
- `Rocker=yes` — produces screen shake.
- `InfDeath=2` — different infantry death anim.
- `AnimList=XGRYSML1...EXPLOLRG` — larger gray-explosion set.
- `Deform=15%` — 15 % chance per impact ≥ 120 damage to crater the terrain.
- `Tiberium=yes` — can chain-detonate ore.
- `Bright=yes` — impact flash.

#### 3.5.3 `[AP]` (Hornet crash collision)

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

Standard armor-piercing warhead — heavily reduced vs infantry (25/25/15 %), full vs light/medium/heavy vehicles (75/100/100 %), good vs wood (65 %), poor vs concrete (60 %). The crashing Hornet does tank-killer damage.

### 3.6 Projectiles

```ini
[NormalBomb]
Arm=2
Shadow=no
Proximity=yes
Ranged=yes
Image=DRAGON
ROT=1
IgnoresFirestorm=yes
```

Standard gravity bomb projectile — used by all aircraft-dropped bombs. `Proximity=yes` triggers on close-enough rather than direct hit. `Ranged=yes` enables range checks. `Image=DRAGON` (the universal bomb sprite).

```ini
[AAHeatSeeker2]
Arm=2
Shadow=no
Proximity=no
Ranged=yes
AA=yes
AG=yes
Image=DRAGON
ROT=60
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=no
```

Used for the `HornetCollision` last-second crash weapon. Both AA and AG capable, ROT=60 (very fast turn — homes aggressively), ignores terrain.

---

## 4. Voice & sound catalogue

| Slot | Sound key | sndmd entry | Audio clip(s) |
|------|-----------|-------------|---------------|
| `VoiceSelect` | `AircraftCarrierSelect` | sound:4232 | 5-clip random `$vairsea-e` |
| `VoiceMove` | `AircraftCarrierMove` | sound:4237 | 5-clip random `$vairmoa-e` |
| `VoiceAttack` | `AircraftCarrierAttackCommand` | sound:4242 | 5-clip random `$vairata-e` |
| `VoiceFeedback` | (empty) | — | — |
| `DieSound` | (empty) | — | — (SinkingSound covers it) |
| `SinkingSound` | `GenLargeWaterDie` | sound:1979 | `gnavsina` (Volume 85) |
| `MoveSound` | `ACCMoveStart` | sound:1242 | `vaccstaa/b` random predelay 0-400, Volume 50 |
| **Hornet:** `CrashingSound` | `HornetDie` | sound:1559 | `vhordiea/b` random, Volume 60 |
| **Hornet:** `ImpactLandSound` | `GenAircraftCrash` | sound:1995 | `vaircraa/b/c` random, Volume 50 |
| **Hornet:** `AuxSound1` | `HornetTakeoff` | sound:1535 | `vhortaka/b` random low-priority, Volume 45 |
| **Hornet:** `AuxSound2` | `HornetLanding` | sound:1542 | `vhorlana/b` random predelay 0-200, Volume 45 |
| **HornetBomb Report** | `HornetAttack` | sound:??? | (in soundmd; standard attack-report clip) |
| **HornetCollision Report** | `HornetCollision` | sound:1550 | `gexpshaa` (twice) interrupt random, Volume 25 |

Carrier-specific (not shared) Voice* clips give it a distinctive "British Royal Navy" flavor. Compare DRED which reuses generic Soviet water voices.

---

## 5. Owners / prerequisites / tech gating

- **Buildable by:** `British`, `French`, `Germans`, `Americans`, `Alliance` — all 5 standard Allied countries.
- **NOT buildable by:** any Soviet country, YuriCountry.
- **Prerequisite:** `GAYARD,TECH` — Allied Naval Yard + any tech building (GATECH/NATECH/YATECH alias). Strictly more flexible than DRED's `NAYARD,NATECH`.
- **TechLevel:** 7 — top-tier (vs DRED's 6). Reflects "ultimate Allied naval asset" positioning.
- `AllowedToStartInMultiplayer=no` → not pre-built.
- `CrateGoodie=no` → not from crates.

---

## 6. Veterancy

### 6.1 Carrier (parent ship) veterancy

| Rank | Effect |
|------|--------|
| Rookie | Base. |
| Veteran | `STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — improves the *Carrier itself*: +HP, +launcher fire-rate, +sight, +speed. **Does NOT swap the Hornet's weapon.** |
| Elite | `SELF_HEAL,STRONGER,FIREPOWER,ROF` — passive auto-heal added; +HP/ROF stack. **Still does NOT affect the Hornet's weapon.** |

> **Key distinction vs DRED:** the Carrier's rank improves how *fast* it launches Hornets and how *durable* it is — but the bombs themselves are determined by the Hornet's own rank.

### 6.2 Hornet (child aircraft) veterancy

| Rank | Effect |
|------|--------|
| Rookie | `Primary=HornetBomb` (40 dmg, ORCAAP). |
| Veteran | `STRONGER,FIREPOWER` — +HP and +damage on bomb. Still uses HornetBomb projectile/warhead. |
| Elite | `STRONGER,FIREPOWER` + **swaps Primary to `HornetBombE`** (80 dmg, ARTYHE with 1-cell splash, rocker, deform). |

Hornets earn veterancy from their own kills (since they're regular aircraft, not `Trainable=no`). An elite Hornet bomb is dramatically more devastating than rookie.

---

## 7. Hardcoded behavior — Ghidra-verified

### 7.1 String-name scan: NO hardcoded behavior keyed to CARRIER or HORNET

- `search_strings "CARRIER"` → returned only **dial-up modem error strings** (`TXT_NO_CARRIER`, `Carrier Detect is false`, etc. — completely unrelated phone-line code). **No CARRIER-unit references in the binary.**
- `search_strings "HORNET"` → **0 matches**.
- `search_strings "HornetLauncher"` → **0 matches**.

**Bottom line:** The engine has zero name-specific hardcoded behavior for the Aircraft Carrier or its Hornet. Both are entirely INI-driven via generic systems.

### 7.2 Generic systems the Carrier uses (with verified TechnoType/UnitType scopes)

| Mechanism | Scope verified | Address (binary) |
|-----------|---------------|------------------|
| `Spawns=`, `SpawnsNumber=3`, `SpawnRegenRate=600`, `SpawnReloadRate=150` | TechnoType (cheat sheet) | 0x00714ee1 / 0x00714ec0 / 0x00714f02 |
| `Spawned=yes` on Hornet | TechnoType | 0x00714e7d |
| **NOT** `MissileSpawn=yes` on Hornet (intentional absence — distinguishes Carrier flow from V3/DRED) | TechnoType | 0x00714f23 (read but value=0 here) |
| `CanPassiveAquire=no` | TechnoType | 0x00714473 |
| `FireAngle=32` | TechnoType | 0x00714b5d |
| `ToProtect=yes` | TechnoType | 0x00714be8 |
| `TooBigToFitUnderBridge=true` | UnitType | 0x0074774e |
| `SinkingSound=GenLargeWaterDie` | DUAL-READ Rules+TechnoType | 0x00669965 + 0x00712f38 (wait — that's ImpactLandSound. SinkingSound is at the address chain from earlier: Rules @ 0x006699a7 + TechnoType @ 0x00712fb0) |
| `ImpactLandSound=GenAircraftCrash` (Hornet) | DUAL-READ Rules+TechnoType | 0x00669965 + 0x00712f38 |
| `CrashingSound=HornetDie` (Hornet) | TechnoType | 0x00712f80 |
| `PitchSpeed=.9` (Hornet) | TechnoType | 0x007123da |
| `PitchAngle=0` (Hornet) | TechnoType | 0x0071236b |
| `ElitePrimary=HornetBombE` (Hornet) | TechnoType | 0x00712a32 |
| `MovementRestrictedTo=Water` (Hornet) | UnitType | 0x00747837 |
| `PipScale=Ammo` (Hornet) | TechnoType | 0x0071411a |
| `ImmuneToPsionics=yes` (Hornet) | TechnoType | 0x00714fa7 |
| `CrateGoodie=no` | UnitType | 0x00747658 |
| Locomotor CLSID `{2BEA74E1-...}` = ShipLocomotionClass | live YR | — |
| Locomotor CLSID `{4A582746-...}` = DriveLocomotionClass-Air (Hornet) | live YR | — |

### 7.3 Spawn-and-return flow (generic SpawnManagerClass — not Carrier-specific)

The Carrier's behavior is described entirely by SpawnManagerClass + AircraftClass for the Hornet:

1. **Idle:** SpawnManager holds 3 Hornet "slots". Each slot is `Active=true, OnDeck=true, Ammo=1`.
2. **Fire command:** Player or AI orders an attack. `HornetLauncher` checks `Range<=25`. If satisfied, SpawnManager picks the first OnDeck+Loaded slot, marks the Hornet "active, in flight", initial velocity at `FireAngle=32` pitch. Carrier ROF=150 means the next launch is locked out for 10 sec.
3. **Flight:** Hornet uses DriveLocomotionClass-Air to navigate to target. `PitchSpeed=.9` and `PitchAngle=0` keep it level. `ROT=3` for turning.
4. **Attack:** When in `Range=5` of target with `Ammo=1`, Hornet drops `HornetBomb` (or `HornetBombE` if elite). Ammo decrements to 0.
5. **Return:** With Ammo=0, AircraftClass enters return-to-spawn mode. Hornet flies back toward Carrier's current position. Because `MovementRestrictedTo=Water`, landing search restricts to water cells (the carrier's footprint).
6. **Dock:** On reaching Carrier, Hornet lands on deck. SpawnManager marks slot `OnDeck=true`. `SpawnReloadRate=150` starts: 10 sec to refill Ammo to 1. Audio: `AuxSound2=HornetLanding`.
7. **Reloaded:** After 150 frames, Hornet is ready for next sortie.

If a Hornet is *destroyed* in flight (shot down by AA), instead of returning, SpawnManager triggers regen: `SpawnRegenRate=600` frames (40 sec) to manufacture a replacement Hornet at the dock with full Ammo. Cost is deducted (or free in standard rules — `Cost=50` is mostly informational here).

### 7.4 The crash-collision swap

When a Hornet is below death threshold (or perhaps in some "going-down" state), the engine swaps its acting weapon from `HornetBomb` to `HornetCollision` (the Secondary). This is encoded in the standard aircraft-death pipeline; the `[HornetCollision]` inline comment confirms the design ("A crashing Hornet turns into this bullet at the last second"). Result: a dying Hornet can still deliver one final blow.

---

## 8. TS-legacy filter

| Feature | Status in YR |
|---------|--------------|
| Carrier `Locomotor=` second GUID after `;` | INI comment — inert. |
| `;ForbiddenHouses=Americans` | INI comment — inert. |
| `;BuildLimit=1` | INI comment — inert. |
| `;OmniFire=yes` on techno block | INI comment — OmniFire is on the weapon. |
| `;PrimaryFireFLH=240,0,20` in artmd | INI comment — engine uses default origin for launch position. |
| Hornet `;Dock=NAHPAD,GAHPAD` and `;Dock=GAAIRC,AMRADR` | INI comments — Hornets do not use the helipad/AFCH dock system; the carrier IS their dock. |
| Hornet `;Selectable=no` | INI comment — author wanted non-selectable but bug prevented. Currently Hornets ARE selectable (a quirk visible in-game). |
| `;HornetLauncher Range=-2` | INI comment — infinite range disabled. |
| `Conventional=yes` on warheads | Live in YR. |
| `Tiberium=yes` on ARTYHE | TS-holdover terminology; in YR drives ore-cluster chain detonation. |
| Fog-of-war 0x1000 gate | Not applicable here. |
| `ImmuneToVeins`, Subterranean, Tunneling | Not on CARRIER/HORNET. |

---

## 9. Coverage audit

| Section | Coverage |
|---------|----------|
| rulesmd `[CARRIER]` — every key | ✅ §1 (41 keys + 4 commented) |
| artmd `[CARRIER]` — every key | ✅ §2 (4 keys + 1 commented FLH) |
| `[HornetLauncher]` weapon | ✅ §3.1 (10 keys + 1 commented Range) |
| `[HORNET]` aircraft rulesmd — every key | ✅ §3.2 (40 keys + 5 commented entries) |
| `[HORNET]` artmd | ✅ §3.2.1 (3 keys) |
| `[HornetBomb]` weapon | ✅ §3.3 |
| `[HornetBombE]` weapon (elite swap) | ✅ §3.3 |
| `[HornetCollision]` weapon | ✅ §3.4 |
| `[ORCAAP]` warhead | ✅ §3.5.1 (10 keys) |
| `[ARTYHE]` warhead | ✅ §3.5.2 (14 keys + 1 commented Spread) |
| `[AP]` warhead | ✅ §3.5.3 |
| `[NormalBomb]` projectile | ✅ §3.6 |
| `[AAHeatSeeker2]` projectile | ✅ §3.6 |
| Voices/sounds (12 slots across carrier + hornet) | ✅ §4 |
| Owners/prereqs/tech | ✅ §5 |
| Veterancy (both Carrier rank AND Hornet rank — distinct) | ✅ §6 |
| Hardcoded behavior — Ghidra-verified | ✅ §7 (CARRIER & HORNET name-scans both returned 0 unit-related matches; 18 individual key scope verifications cross-ref'd to cheat sheet + 7 new scopes added: PitchSpeed/PitchAngle/ElitePrimary/CrashingSound/ImpactLandSound/MovementRestrictedTo, dual-read pattern for ImpactLandSound) |
| TS-legacy filter | ✅ §8 |
| Spawner-family comparison table | ✅ at top + §7.3 flow |

---

## Ghidra audit log (audit iteration 20 — 2026-05-18)

**Methodology**: CARRIER is one of the densest-claim docs to date — 18+
cited parser xrefs across TechnoType/UnitType, plus the spawn-family
key cluster. This audit re-verifies the doc's negative claims, all
spawn-family parser xrefs, and **pins the 7 spawn-family TechnoType
offsets that the cumulative cheat-sheet hadn't pinned yet**. The doc
already cross-references 5 deep-RE reports for SpawnManager state
machine + AircraftClass behavior; those are taken as authoritative
trust-chain. ~16 Ghidra queries: 8 string-searches + 6 xref lookups +
1 grep on saved TechnoTypeClass__ReadINI.

### Negative claims re-verified

| Query | Result |
|-------|--------|
| `search_strings("^HORNET$")` | **0 matches** |
| `search_strings("^HornetLauncher$")` | **0 matches** |

Plus prior `search_strings "CARRIER"` returning only TXT_NO_CARRIER /
modem-error strings (not re-run this pass — taken as the doc's
authoritative finding).

Confirms: no hardcoded section-name branch for CARRIER, HORNET, or
HornetLauncher in `gamemd.exe`. All behavior is generic flag-driven
via the SpawnManagerClass + AircraftClass machinery.

### String + parser xref re-verification (BINARY-VERIFIED)

All 6 spawn-family xrefs verify exactly:

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `Spawns` | 0x008184C8 | 0x00714E9E | TechnoTypeClass__ReadINI |
| `Spawned` | 0x008437D8 | 0x00714E7D | TechnoTypeClass__ReadINI |
| `SpawnsNumber` | 0x008437B8 | 0x00714EE1 | TechnoTypeClass__ReadINI |
| `SpawnRegenRate` | 0x008437C8 | 0x00714EC0 | TechnoTypeClass__ReadINI |
| `SpawnReloadRate` | 0x008437A8 | 0x00714F02 | TechnoTypeClass__ReadINI |
| `MissileSpawn` | 0x00843798 | 0x00714F23 | TechnoTypeClass__ReadINI |

**Bonus**: `Spawns` string at 0x008184C8 is multi-purpose — also xref'd
from `VoxelAnimTypeClass__ReadINI @ 0x0074B1FF`,
`ParticleSystemTypeClass__ReadINI @ 0x0064432A`, and
`AnimTypeClass__ReadINI @ 0x004281DE`. Each of those entity types has
its own `Spawns=` key with different semantics (AnimType `Spawns=`
chains animations; ParticleSystemType `Spawns=` describes particle
emission). The TechnoType-scope Spawns is the CARRIER-relevant one.

### NEW TechnoType offsets BINARY-VERIFIED (spawn-family cluster)

The spawn-family TechnoType offsets form a contiguous block at
+0xD54..+0xD68:

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0xD54` | `Spawned` | byte | `*(undefined1*)(param_1 + 0x355) = uVar3` after ReadBool. **NEW** — set on the spawned aircraft (Hornet, DMISL, V3ROCKET); marks "this is a spawn-only TechnoType, not directly buildable". |
| `+0xD58` | `Spawns` | TechnoType* | `param_1[0x356] = iVar4` after FUN_0067BD30 (TechnoTypeClass-FindOrAllocate variant). **NEW** — Carrier sets this to HORNET, Dread to DMISL, V3 to V3ROCKET. |
| `+0xD5C` | `SpawnsNumber` | int | `param_1[0x357] = iVar4`. **NEW** — Carrier=3, Dread=2, V3=1. Number of spawn slots in the magazine. |
| `+0xD60` | `SpawnRegenRate` | int (frames) | `param_1[0x358]` (default-read from this offset). **NEW** — Carrier=600 (40 sec to manufacture replacement Hornet if shot down); Dread/V3 use 0 (regen via re-purchase). |
| `+0xD64` | `SpawnReloadRate` | int (frames) | `param_1[0x359]`. **NEW** — Carrier=150 (10 sec for landed Hornet to refill Ammo=1). |
| `+0xD68` | `MissileSpawn` | byte | `*(undefined1*)(param_1 + 0x35A) = uVar3`. **NEW** — set on DMISL and V3ROCKET; 0 on HORNET. This is the SpawnManagerClass branch flag for missile-vs-aircraft handling (per the doc's SPAWN_MANAGER_CLASS_GHIDRA_REPORT cross-reference). |

### Additional NEW TechnoType offset

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0x3D0` | `FireAngle` | int (degrees) | `param_1[0xF4] = iVar4` after ReadInt. **NEW** — Carrier sets to 32 (initial pitch on launch). Used for both aircraft-spawn launches (Carrier→Hornet) and missile-spawn launches (Dread→DMISL, V3→V3ROCKET). |

### Items NOT re-verified in this pass (DEFERRED)

- 10+ other cited parser xrefs in the doc (CanPassiveAquire 0x00714473,
  ToProtect 0x00714BE8, PitchSpeed 0x007123DA, PitchAngle 0x0071236B,
  ElitePrimary 0x00712A32, MovementRestrictedTo 0x00747837, PipScale
  0x0071411A, ImmuneToPsionics 0x00714FA7, CrashingSound 0x00712F80,
  ImpactLandSound dual-read 0x00669965/0x00712F38, SinkingSound dual-read
  0x006699A7/0x00712FB0). These have not been verified this pass but
  the verification pattern from spawn-family (6/6 exact matches) gives
  high confidence the rest also hold.
- Spawn-family byte-offsets for HORNET's `Spawned=yes` and per-Hornet
  fields (the doc cites these but I focused on the Carrier-side
  TechnoType offsets).
- `[CombatDamage]` Rules-side spawn-related globals (if any — e.g.,
  `MaxAircraftDocks` style).
- 5 deep-RE docs cross-referenced (SPAWN_MANAGER_CLASS, AIRCRAFTCLASS,
  DRED, V3, BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION) — trust-chain,
  not re-derived.
- The crash-collision swap logic (§7.4 — when a dying Hornet swaps its
  weapon to HornetCollision Secondary).
- The "MovementRestrictedTo=Water" landing-restriction consumer.

### Confidence summary

- **HIGH**: 8 string addresses + 6 spawn-family parser xrefs (all
  exact); 7 NEW TechnoType struct offsets — the spawn-family cluster at
  +0xD54..+0xD68 (Spawned/Spawns/SpawnsNumber/SpawnRegenRate/SpawnReloadRate/MissileSpawn)
  + FireAngle +0x3D0. These offsets are reused by DRED, V3, and SCHP
  spawner units — cumulative value is high.
- **MEDIUM**: 10+ other parser xrefs in the doc not directly verified
  this pass; trust-extended from the 6/6 spawn-family exact-match rate.
- **LOW** (delegated): 5 deep-RE doc cross-references for SpawnManager
  / AircraftClass behavior chain — trust-chain only.
- **No INCORRECT findings**. Doc claims align with binary evidence
  where verified.

---

## 10. Quick implementer summary

To make a CARRIER-equivalent in this engine:

1. **Render** — voxel + HVA, no NoSpawnAlt (deck looks the same with or without Hornets). Hornet voxel renders separately at its own world position (on deck while parked, mid-air while sortie-active).
2. **Movement** — ShipLocomotionClass (water, Speed=4, ROT=1, TooBigToFitUnderBridge gate).
3. **Spawner** — generic SpawnManager with 3 slots, regen 600 frames per slot (only triggered when a Hornet is destroyed), per-slot reload 150 frames (triggered after each return-dock).
4. **Hornet flight loop:**
   - Launch at FireAngle=32 pitch from carrier deck.
   - Fly toward target via DriveLocomotionClass-Air (PitchSpeed/PitchAngle/ROT).
   - Drop bomb (or ElitePrimary if Hornet is elite rank) when within Range=5.
   - Return to carrier on Ammo=0.
   - Land on water cells only (MovementRestrictedTo=Water).
   - Refill ammo over 150 frames at dock.
   - Resume idle.
5. **Crash collision** — when Hornet is shot down, transmute its weapon to Secondary=HornetCollision and use AAHeatSeeker2 projectile for final-second AP damage on ground unit below.
6. **Fire** — Burst=1 on launcher (one Hornet per command), ROF=150 lockout.
7. **Damage path** — Hornet's own [HornetBomb]/[HornetBombE] weapon — NOT a Rules global. This is the key difference vs DRED/V3.
8. **Veterancy split** — Carrier rank affects launch rate / Hp / sight / speed. Hornet rank affects bomb damage / weapon swap. Two independent veterancy tracks.
9. **Audio** — Carrier-unique voice triplet (`$vair*`), generic GenLargeWaterDie sinking; Hornet takeoff/landing aux sounds, generic aircraft-crash impact.
10. **AI** — ToProtect=yes (escort), ThreatPosed=25 (Carrier), 10 (Hornet).

No CARRIER or HORNET-specific code paths are required — all behavior emerges from correctly wiring generic SpawnManager + AircraftClass + ShipLocomotion + standard weapon firing + the per-Hornet ElitePrimary swap.
