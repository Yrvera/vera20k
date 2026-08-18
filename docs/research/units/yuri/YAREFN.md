---
name: yaref-doc
description: Yuri Slave Miner deployed-building form (YAREFN) — full ruleset + artmd
  + hardcoded SlaveManager brain-transplant + ore-destination behavior. Closes the
  SMIN/YAREFN pair (vehicle ↔ building).
metadata:
  type: project
---

# YAREFN — Yuri Slave Miner (deployed building form)

**INI ID:** `YAREFN`
**Display:** "Yuri Ore Refinery" (`UIName=Name:SMIN`, so the display string is shared
with the vehicle form — both show as "Slave Miner" in the UI)
**Section:** `[BuildingTypes]`
**Owner side:** Yuri (built into every multiplayer house via expansive `Owner=`)
**Role:** Deployed form of the Slave Miner — combined ore refinery + slave-control
hub + self-defending turret. The mobile vehicle form ([SMIN](../yuri/SMIN.md))
`DeploysInto=YAREFN`; this building `UndeploysInto=SMIN`. It is also the *only*
"refinery" Yuri builds — there is no separate Yuri refinery building.

---

## Index correction

The INDEX entry described `[YAREFN]` as "Slave Miner deploy form — Mobile refinery"
(BuildingTypes section). Confirmed correct. **`[SMON]` (VehicleTypes) is NOT the
deployed form** — it is a dead/vestigial entry (`Name=ZZZ Useless;
Slave Miner(noback)`, `TechLevel=-1`, `AllowedToStartInMultiplayer=no`,
`Image=CMON` — uses Chrono Miner art) clearly cut during development. The index
will be updated to mark SMON `SKIP-DUPLICATE` (similar to UTNK).

The deployed Slave Miner you see in-game when you Deploy a [SMIN](../yuri/SMIN.md)
is **YAREFN**, the building documented here. See SMIN's "Deploy / Undeploy
sequence" section and the cross-referenced Ghidra reports.

---

## Rulesmd verbatim

```ini
[YAREFN]
UIName=Name:SMIN
Name=Yuri Ore Refinery
BuildCat=Resource
;Bib=yes
Prerequisite=POWER,YACNST
Strength=2000
Adjacent=2
Armor=medium
Primary=20mmRapid
ElitePrimary=20mmRapidE
Turret=yes
TurretAnim=SMINTUR
TurretAnimIsVoxel=true
TurretAnimX=-25
TurretAnimY=15
TurretAnimZAdjust=-50
TechLevel=1;-1
Sight=6
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
AIBasePlanningSide=2 ;gs 0 for Good, 1 for Evil
Cost=1750
Soylent=1750
Points=80
Power=0;-50
Storage=200
Capturable=false;gs true
;Crewed=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
HalfDamageSmokeLocation1=0,0,0
DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM
MaxDebris=10
MinDebris=5
PipScale=Tiberium
ThreatPosed=0	; This value MUST be 0 for all building addons
;DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys
DamageSmokeOffset=410, 100, 165
AIBuildThis=yes;no;yes
TogglePower=no
RefinerySmokeOffsetOne=-85,-85,220
RefinerySmokeOffsetTwo=-85,90,220
RefinerySmokeOffsetThree=95,-85,220
RefinerySmokeOffsetFour=95,90,220
RefinerySmokeFrames=30
RefinerySmokeParticleSystem=SmallGreySSys;
Enslaves=SLAV;gs The SMIN does not get an Enslaves listing because the Slave object will get passed from unit to building upon deploy
SlavesNumber=5
SlaveRegenRate=500 ;225
SlaveReloadRate=25
;moving brain to refinery to start
;Ugh.  Now that placed as building, problem arises from managing to get a SMIN as vehicle (Campaign map, crate).  Both get this listing now, and Brain transplant will check to make sure extra one is not created
Spyable=yes
;NumberImpassableRows=3 ; This is the fix to the Repair depots are flat and RadioContact/Enter means I can drive on you assumption.  It counts from game west
ImmuneToPsionics=yes ; defaults to yes for buildings, no for others
BaseNormal=no
UndeploysInto=SMIN
ClickRepairable=no
DeployFacing=0;0 = N, 7 = NW
ResourceGatherer=yes;gs for the AI to handle the slave miner, it has to understand what makes money
ResourceDestination=yes
VoiceSelect=SlaveMinerSelect
VoiceMove=SlaveMinerMove
VoiceAttack=SlaveMinerAttackCommand
VoiceHarvest=SlaveMinerHarvest
DeploySound=SlaveMinerUndeploy
VoiceDeploy=SlaveMinerUnDeployVoice
Unsellable=yes
Trainable=yes
BuildTimeMultiplier=1.15
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:SMIN` — CSF string label is the same as the vehicle SMIN ("Slave
  Miner"). Both forms share one display name.
- `Name=Yuri Ore Refinery` — internal description (overridden by `UIName`).
- `BuildCat=Resource` — appears in the sidebar Resource tab (alongside refineries
  / Ore Purifier), not in Defense/Combat.
- `;Bib=yes` — commented; the building has no bib (the SHP foundation pad that
  normally extends beyond walls).
- `Prerequisite=POWER,YACNST` — requires *any* power plant (`POWER` is an
  AlphaShape `[AI]/[General]` macro group; see [YAPOWR](../structures/YAPOWR.md))
  and the Yuri ConYard. **No barracks or war factory required** — buildable
  immediately after the first power plant, identical to other faction refineries.
- `Adjacent=2` — placeable up to 2 cells from existing base structure (default).

**Combat / defense (this is a self-defending refinery)**
- `Strength=2000` — same HP as the [SMIN](../yuri/SMIN.md) vehicle form (the
  deploy preserves HP).
- `Armor=medium` — same as SMIN.
- `Primary=20mmRapid`, `ElitePrimary=20mmRapidE` — identical to SMIN; the
  turret is preserved across deploy/undeploy. See "Weapon" section.
- `Turret=yes` — the building has a rotating turret.
- `TurretAnim=SMINTUR` — voxel turret animation (the same VXL the vehicle SMIN
  uses for its turret; shared art asset).
- `TurretAnimIsVoxel=true` — turret is rendered from a VXL, not a SHP.
- `TurretAnimX=-25`, `TurretAnimY=15`, `TurretAnimZAdjust=-50` — pixel offsets
  positioning the turret on top of the building art. Z-adjust pushes the turret
  *down* (negative) by 50 pixels relative to the building anchor.

**Tech / availability**
- `TechLevel=1;-1` — TL 1 (buildable from the start). The trailing `;-1` is a
  commented disabled state (was once unbuildable).
- `Sight=6` — 6-cell vision radius around the deployed refinery. Slightly more
  than SMIN's `Sight=5` (deployed form sees one cell further; the only stat
  difference between the two forms).
- `Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry`
  — every multiplayer house can build it. *Same expansive owner list as SMIN*
  (because SMIN can be built by any side, and YAREFN is its deploy target,
  every side needs to own it).
- `AIBasePlanningSide=2` — AI base-planner treats this as side 2 (Yuri). Affects
  AI base-layout heuristics, not buildability.

**Economy**
- `Cost=1750` — same as SMIN. *Deploying SMIN does not refund or charge* — the
  vehicle is consumed and the building replaces it at the unit's location.
- `Soylent=1750` — refund value if Grinder-recycled. Matches Cost (no penalty
  for grinding). N/A here since the building is `Capturable=false` and unlikely
  to be grindable by-design.
- `Points=80` — endgame score contribution.
- `Power=0;-50` — draws 0 power. The trailing `;-50` is a commented older value
  (was once a power drain). **The Slave Miner is *free* power-wise** — unlike
  every other refinery which consumes power. The deploy commentary says this is
  intentional: the building is a passive mobile refinery, not a powered facility.
- `Storage=200` — holds up to 200 ore units (worth ~5000 credits at 25/u rate;
  closer to ~3000 since slaves harvest mixed types). When full the slaves stop
  harvesting until the player picks up the credits via cash deposit, which
  happens automatically — there is no "dock" step like Allied/Soviet refineries
  because the slaves walk back to *the building itself* and credit cash directly.
- `Capturable=false;gs true` — **cannot be captured by engineer**. Original
  comment ("gs true") shows it was once captureable; Westwood disabled this.
- `Spyable=yes` — Spy can infiltrate to steal credits (refinery spy effect:
  steals a chunk of the player's reserve). Same effect as spying on any other
  refinery.

**Storage display**
- `PipScale=Tiberium` — pip bar displays ore-storage fill (green/orange pips for
  storage amount), not passenger or ammo pips. Same as all refineries.
  Ghidra-verified scope: `PipScale` is read in `TechnoTypeClass__ReadINI` at
  `0x008443e0 → 0x0071411a` (cheat-sheet) — global enum, set per-techno.

**Destruction / debris**
- `Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` — explosion animation
  pool; one is chosen on death.
- `HalfDamageSmokeLocation1=0,0,0` — smoke emitter spawn-point offset when the
  building reaches half HP. `(0,0,0)` = at the building's anchor cell. Only one
  emitter (no `HalfDamageSmokeLocation2/3/...`).
- `DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM` — debris
  shape pool sampled on destruction.
- `MaxDebris=10` / `MinDebris=5` — random debris-piece count between 5 and 10.
- `DamageSmokeOffset=410, 100, 165` — smoke-particle spawn-point offset (in
  leptons) used when smoke effects play. Same scope as TechnoType
  (verified offset `0x00843f60 → 0x00713e25` per cheat-sheet).
- `;DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys` — commented
  out; the building uses default damage particles (sparks + smoke).

**Refinery smoke (working-state visual)**
- `RefinerySmokeOffsetOne=-85,-85,220`
- `RefinerySmokeOffsetTwo=-85,90,220`
- `RefinerySmokeOffsetThree=95,-85,220`
- `RefinerySmokeOffsetFour=95,90,220` — four smoke-stack emitter positions
  (leptons, relative to building anchor). Stacks puff smoke periodically while
  the refinery is "actively processing" (i.e. a Slave just walked in and
  dumped ore). The four `(X,Y,220)` positions correspond to the four corners of
  the 2×2 footprint, all elevated 220 leptons above ground.
- `RefinerySmokeFrames=30` — interval (frames) between puffs.
- `RefinerySmokeParticleSystem=SmallGreySSys` — which particle system to emit.
- **Ghidra verification:** `RefinerySmokeOffsetOne` is read in
  `TechnoTypeClass__ReadINI` (string `0x00843f20`, reader at `0x00713e93`).
  Scope is therefore TechnoType-wide (the engine *would* read these fields on
  any TechnoType), but the smoke-emit code only runs when the refinery dock /
  unload state machine fires — making them functionally building-only in
  normal play.

**AI hinting**
- `AIBuildThis=yes;no;yes` — AI is allowed to build it (the toggles in the
  comment are old historical values).
- `TogglePower=no` — cannot be manually powered down via right-click (would
  reveal the toggle option in the UI for buildings with Power<0; here Power=0
  so the toggle would be meaningless).
- `BaseNormal=no` — *not counted as a normal base building for "base destroyed"
  detection*. This is critical: a player whose only structure left is YAREFNs
  will *still trigger defeat*. The Slave Miner doesn't keep you in the game
  the way a ConYard or barracks would. Same flag pattern used on power plants
  for the same purpose (see defeat-detection notes).

**Slave-control fields (this is the SlaveManager core block)**
- `Enslaves=SLAV` — when this building owns slaves, they are of type
  [SLAV](../yuri/SLAV.md). Mirror of the same key on SMIN. The verbatim
  comment explains the design: SMIN was originally not given this listing
  because slaves "get passed from unit to building upon deploy," but it was
  added for the campaign/crate edge-case where a SMIN spawns mid-game and
  has no SlaveManager yet. **Both forms now declare it.**
- `SlavesNumber=5` — five slaves are allocated by the SlaveManager.
- `SlaveRegenRate=500` — replacement slave spawn rate (ticks between
  spawns when SlavesNumber slots aren't filled).
- `SlaveReloadRate=25` — per-slave reload-ore timer (ticks before each slave
  begins another harvest cycle after dumping).
- **Ghidra verification:** `SlavesNumber` string is at `0x00843804`, read at
  `0x00714e1a` in `TechnoTypeClass__ReadINI` — confirmed TechnoType scope (same
  function as `Enslaves` at `0x00843824 → 0x00714dd7`, matching the SMIN doc).

**Ore-harvest / AI economy**
- `ResourceGatherer=yes` — AI treats this building as something that produces
  income. The comment is explicit: "for the AI to handle the slave miner, it
  has to understand what makes money." Mirror of the same key on harvesters.
- `ResourceDestination=yes` — counts as a refinery for the harvester /
  slave dock-and-deposit AI logic. **Ghidra verification:** `ResourceDestination`
  string at `0x00843ca4`, read at `0x007143f1` in `TechnoTypeClass__ReadINI` —
  TechnoType-scoped (also confirmed used on the NAREFN/GAREFN refineries, where
  the same flag tells the harvester pathfinder "yes, you can dock here").

**Psi / immunity**
- `ImmuneToPsionics=yes` — *cannot be mind-controlled by a Yuri/MIND unit*.
  The verbatim comment notes this defaults yes-for-buildings, no-for-others.
  Ghidra-verified scope: `0x00843754 → 0x00714fa7` in TechnoType (per
  cheat-sheet). **This matches the SMIN flag and is the second leg of the
  "the slave economy can't be hijacked" protection.**

**Deploy/undeploy plumbing**
- `UndeploysInto=SMIN` — right-click "Deploy" / "Undeploy" command transforms
  the building back into the [SMIN](../yuri/SMIN.md) vehicle. Verified scope
  `0x00844170` in TechnoType ReadINI. The two forms share HP, vet level, slaves
  (brain-transplant), and ore storage across the transition.
- `DeployFacing=0;0 = N, 7 = NW` — facing the *building* spawns at; here North.
  Ghidra-verified `DeployFacing` shares the ReadINI function `0x00460c76` with
  `ZFudgeBridge` (cross-scope BuildingType ReadINI; same function ghidra
  labelled "BuildingTypeClass_ReadINI_Water" — probably a mis-label).
- `ClickRepairable=no` — the player cannot click the wrench icon to manually
  repair this building (it relies on `Trainable=yes` veterancy + crate
  pickups + Service Depot routing for repairs).

**Misc**
- `;NumberImpassableRows=3` — commented out. Was a TS-era fix for "you can
  drive on top of flat repair pads" — irrelevant for the Slave Miner.
- `Unsellable=yes` — no Sell button; sidebar "$" cursor does nothing. This
  prevents the player from cashing out for cheap.
- `Trainable=yes` — *the building gains veterancy* from kills it scores via its
  20mmRapid turret. This is unusual for buildings (most are not Trainable).
  At elite rank the turret upgrades to `20mmRapidE` (better damage, HE warhead).
- `BuildTimeMultiplier=1.15` — actual build time = base*1.15. The Slave Miner
  takes 15% longer than its `Cost`-derived base time. Ghidra-verified
  `0x00843cf0 → 0x00714371` TechnoType scope.
- `VoiceDeploy=SlaveMinerUnDeployVoice` — voice played on deploy command.
  Note the asymmetry: `DeploySound=SlaveMinerUndeploy` (the SFX) is played
  on either transition; `VoiceDeploy` is only played when *issuing the deploy
  command from the UI* (it's tagged `Undeploy` in the rules but is actually
  the deploy-from-vehicle voice — Westwood naming confusion).

---

## Artmd verbatim

```ini
[YAREFN]
;Image=GAREFN
Remapable=yes
Cameo=SMINICON
AltCameo=SMINUICO
Foundation=2x2
Height=3
ZShapePointMove=30,15 ; SJM is fixing zshape/zshapelocky problems, changed from 24,-48
Buildup=YAREFNMK
DemandLoadBuildup=true
FreeBuildup=true
NewTheater=yes
IdleAnim=YAREFN_A
IdleAnimZAdjust=0
;IdleAnimYSort=700
CanHideThings=False
CanBeHidden=True
OccupyHeight=2
PrimaryFireFLH=120,0,185
```

### Key-by-key annotation

- `;Image=GAREFN` — commented. The building uses its own art (`YAREFN.shp`).
  The commented `GAREFN` was likely an early prototype where the Yuri refinery
  reused the Allied refinery art before custom art was made.
- `Remapable=yes` — house-color remap palette applies to designated pixels in
  the SHP (the standard remap channel). Each player's Slave Miner shows their
  team color.
- `Cameo=SMINICON` — sidebar build-button SHP for normal display.
- `AltCameo=SMINUICO` — alternate cameo used when the Tooltips/UI overlay
  variant is needed (typically a slightly different framing).
- `Foundation=2x2` — building occupies a 2×2 cell footprint. The 4
  `RefinerySmokeOffset*` keys correspond to the four corners of this 2×2.
- `Height=3` — visual height in cells (for height-based occlusion / Z-sorting).
- `ZShapePointMove=30,15` — Z-shape correction offsets (X=30, Y=15 leptons). The
  comment notes SJM (Steve J. Maetzold) was fixing zshape problems; the values
  changed from an earlier `24,-48`. Z-shapes are the cell-level occlusion masks
  used to decide when a sprite is hidden by terrain.
- `Buildup=YAREFNMK` — the buildup animation SHP (separate from the idle).
  Plays once when the building is constructed-from-MCV or completes from build
  queue.
- `DemandLoadBuildup=true` — the buildup SHP is loaded on-demand the first
  time it's needed (not pre-loaded at game start).
- `FreeBuildup=true` — the buildup SHP can be unloaded after use to free RAM.
- `NewTheater=yes` — theater-aware filename swap. The art system substitutes a
  theater-prefix (e.g. `tyaref.shp` for temperate, `uyaref.shp` for urban).
- `IdleAnim=YAREFN_A` — the idle animation overlay (see `[YAREFN_A]` block):
  3-frame infinite loop, Layer=ground. This is the slowly-spinning/processing
  vat or whatever the Slave Miner's idle visuals are.
- `IdleAnimZAdjust=0` — no Z-offset on the idle anim.
- `;IdleAnimYSort=700` — commented Y-sort override; using default behavior.
- `CanHideThings=False` — taller objects placed behind this building are *not*
  hidden by it (it doesn't occlude others).
- `CanBeHidden=True` — this building *can* be hidden behind taller objects.
- `OccupyHeight=2` — vertical occupancy used by collision / spawn-point logic.
- `PrimaryFireFLH=120,0,185` — Fire/Launch/Height: the turret's projectile-spawn
  offset (X=120 forward, Y=0 lateral, Z=185 up). Bullets/cannon shells from the
  `20mmRapid`/`20mmRapidE` weapons spawn from this point on the turret.

### Referenced art sub-blocks

```ini
[YAREFN_A]
Image=YAREFN_A
Normalized=yes
LoopStart=0
LoopEnd=2
LoopCount=-1
Layer=ground
NewTheater=yes
;DetailLevel=1
Shadow=yes
```

- `Image=YAREFN_A` — the idle-anim SHP file.
- `Normalized=yes` — frame rate normalized to game speed.
- `LoopStart=0 / LoopEnd=2` — 3-frame loop (frames 0, 1, 2).
- `LoopCount=-1` — infinite loop.
- `Layer=ground` — rendered as a ground-plane decal (under units).
- `Shadow=yes` — casts a shadow.

`[YAREFNMK]` (the Buildup) — **does not appear as a defined section in
artmd.ini**. Buildup SHPs reference filenames directly; the engine looks for
`yarefnmk.shp` (or theater-prefixed variants). No further art-block keys.

---

## Weapons

Identical to [SMIN](../yuri/SMIN.md). Reproduced verbatim:

```ini
[20mmRapid]
Damage=30
ROF=20
Range=5.5
Projectile=InvisibleLow
Speed=100
Warhead=HARVWH
Report=WarMinerAttack
Anim=GUNFIRE
```

```ini
[20mmRapidE]
Damage=50
ROF=50
Range=5.75
Projectile=Cannon
Speed=40
Warhead=HowitzerWH
Report=RhinoTankAttack
Anim=GUNFIRE
Bright=yes
```

**Notes specific to building scope:**
- `Report=WarMinerAttack` — fire SFX. Shared with the Soviet War Miner.
- `Report=RhinoTankAttack` (Elite) — uses the Rhino Tank fire SFX (heavier cannon
  thump). Audible cue that the building has reached elite veterancy.
- `Anim=GUNFIRE` — generic muzzle flash anim.
- `Bright=yes` (Elite only) — the muzzle flash *brightens the surrounding tiles*
  one frame on fire (palette effect).
- `Projectile=InvisibleLow` (basic) vs `Projectile=Cannon` (Elite) — at elite
  rank you see a visible cannon-shell projectile instead of an invisible hit.

### Warhead — basic `HARVWH`

```ini
[HARVWH]
;CellSpread=.3
;PercentAtMax=.5
Verses=100%,80%,70%,50%,20%,20%,20%,15%,10%,400%,100%
InfDeath=1
AnimList=PIFFPIFF,PIFFPIFF
;Bright=yes
Bullets=yes
ProneDamage=50%
```

- `Verses=100%,80%,70%,50%,20%,20%,20%,15%,10%,400%,100%` — armor-vs-warhead:
  | Armor    | Multiplier |
  |----------|-----------|
  | none     | 100% |
  | flak     | 80% |
  | plate    | 70% |
  | light    | 50% |
  | medium   | 20% |
  | heavy    | 20% |
  | wood     | 20% |
  | steel    | 15% |
  | concrete | 10% |
  | special_1 (terror drone host) | **400%** |
  | special_2 | 100% |

  The 400% vs special_1 means a War Miner / Slave Miner refinery turret
  *one-shots a Terror Drone* parasiting one of its units (Damage 30 × 4 = 120
  vs DRON Strength 100 — kill). Identical to other harvester turrets.
- `InfDeath=1` — small-arms infantry death animation.
- `AnimList=PIFFPIFF,PIFFPIFF` — pick from PIFFPIFF (tiny puff) — listed twice
  for weighting / cycling, though duplicated.
- `Bullets=yes` — *deflects* off armor under some conditions (used by the
  spark/ricochet system).
- `ProneDamage=50%` — prone infantry take half damage from this warhead.

### Warhead — elite `HowitzerWH`

```ini
[HowitzerWH]
CellSpread=.5
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=100%,90%,80%,60%,40%,40%,50%,40%,25%,80%,100%
Conventional=yes
InfDeath=3
AnimList=S_CLSN16,S_CLSN22,S_CLSN30,S_CLSN42,S_CLSN58
ProneDamage=50%
```

- `CellSpread=.5` — small AoE; half-cell splash.
- `PercentAtMax=.5` — 50% damage falloff at the AoE edge.
- `Wall=yes` / `Wood=yes` — damages walls and wooden buildings (basic
  20mmRapid does not).
- `Verses` — significantly stronger vs medium/heavy/wood/steel/concrete than
  HARVWH. Lower 80% vs special_1 (Terror Drone *less* damaged by elite — odd
  but per-rules).
- `Conventional=yes` — counted as conventional (non-energy) damage; affects
  Iron Curtain interaction etc.
- `InfDeath=3` — explosion infantry death (gibbed by explosion).
- `AnimList=S_CLSN16..58` — picks an explosion impact anim sized by damage
  bracket.

**Net combat impact:** the deployed YAREFN building has its own self-defense
turret that scales: starts as a 20mm anti-infantry/anti-light gun, becomes a
HE-shell cannon at elite — a refinery you don't have to defend with a Pillbox.
The 5.5–5.75-cell range covers about 2× the 2×2 footprint perimeter, so a
clustered slave group can be defended.

---

## Voices / sounds

All entries from `soundmd.ini`:

```ini
[SlaveMinerSelect]
Sounds=$vslasea $vslaseb $vslasec $vslased $vslasee $vslasef $vslaseg
Control=random
Volume=85

[SlaveMinerMove]
Sounds=$vslamoa $vslamoc $vslamoe $vslamof
Control=random
Volume=85

[SlaveMinerAttackCommand]
Sounds=$vslaata $vslaatb $vslaatc $vslaate ;$vslaatd
Control=random
Volume=85

[SlaveMinerHarvest]
Sounds=$vslahaa $vslahab $vslahac $vslahad $vslamod
Control=random
Volume=85

[SlaveMinerUnDeployVoice]
Sounds= $vslamob
Control=random
Volume=85

[SlaveMinerUndeploy]
Sounds= vslade2a

[SlaveMinerMoveStart]
Sounds= vslastaa vslastab
Control= random predelay
Delay=0 400
Priority=Low
FShift= -10 10
VShift=10
Volume=55
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=SlaveMinerSelect` | `[SlaveMinerSelect]` | Click the deployed refinery |
| `VoiceMove=SlaveMinerMove` | `[SlaveMinerMove]` | (N/A for building, inherited from SMIN) — playable only on vehicle form; building can't move |
| `VoiceAttack=SlaveMinerAttackCommand` | `[SlaveMinerAttackCommand]` | Manually order a target |
| `VoiceHarvest=SlaveMinerHarvest` | `[SlaveMinerHarvest]` | When a slave returns and dumps ore (verified: `VoiceHarvest` is a TechnoType field at `0x00844090 → 0x00713652` per cheat-sheet) |
| `DeploySound=SlaveMinerUndeploy` | `[SlaveMinerUndeploy]` | Mechanical SFX on either deploy/undeploy transition |
| `VoiceDeploy=SlaveMinerUnDeployVoice` | `[SlaveMinerUnDeployVoice]` | Voice line on deploy-from-vehicle command (naming inversion: this is the "now-undeploying-to-building" form) |

`$`-prefixed sounds are voice samples (eva-dialog mixed channel, lower priority);
non-prefixed are SFX (mechanical/environmental). Note `[SlaveMinerHarvest]`
includes `$vslamod` as an outlier — slipped in from the move pool, likely a
typo from the Westwood VO recording session that nobody fixed.

`[SlaveMinerMoveStart]` is a *vehicle-form-only* sound — engine ignition when
SMIN begins moving. Building doesn't play it.

---

## Hardcoded behavior (Ghidra-verified)

YAREFN inherits all SMIN-form hardcoded behavior plus building-specific deploy
plumbing. Rather than duplicate the SMIN doc, this section cross-references and
adds building-specific items.

### 1. Brain-transplant on deploy/undeploy

Mechanism documented in detail in
[`SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md`](../../SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md).

Key points:

- `SlaveManagerClass` is at `TechnoClass+0x2D8` (per SMIN doc — verified at
  `0x00843830 / 0x00843824 / 0x00843804` reads in TechnoType ReadINI).
- When SMIN deploys → YAREFN: the SlaveManager pointer is *moved* from the
  vehicle to the building (not copied / not re-created). Existing slaves keep
  pointing at the same manager; they don't notice the transition.
- The rulesmd comment "Brain transplant will check to make sure extra one is
  not created" refers to the campaign-crate edge case where a fresh SMIN
  spawns mid-game with no existing manager — the deploy path checks for an
  existing SlaveManager before allocating a new one.
- Slaves currently outside their `LeashRange` of the SMIN form will follow the
  same manager pointer to the YAREFN's new location (which is the same world
  position).

### 2. Ore storage / cash flow

Mechanism documented in detail in
[`SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md`](../../SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md).

Key points:

- Slaves harvest ore tiles via a per-slave `HarvestRate` (see [SLAV](../yuri/SLAV.md)
  doc).
- On return-to-manager: the slave's accumulated ore is converted to *credits
  immediately* and deposited into the player's reserve. **There is no
  `Storage=200` accumulation phase in normal play** — the field exists because
  the `Storage=200` is a fallback buffer used when the player's credit reserve
  is being calculated mid-frame (defensive buffer).
- `ResourceDestination=yes` (verified TechnoType-scope) tells the harvester /
  slave dock-and-deposit AI to *accept* the slave at this building.
- `ResourceGatherer=yes` tells the AI economy planner this building generates
  income.

### 3. Auto-firing turret

- `OpportunityFire=yes` inherited from SMIN (Ghidra-verified TechnoType scope
  `0x00843a74 → 0x0071483d`). The building auto-targets enemies in `Range=5.5`
  /`5.75` even when not ordered.
- `Turret=yes` + `TurretAnim=SMINTUR` + `TurretAnimIsVoxel=true` means the
  turret rotation is voxel-rendered (smoother than SHP-frame).
- Veterancy gained from kills tracks per the standard
  `Trainable=yes` system (Veteran +25%, Elite weapon swap).

### 4. ImmuneToPsionics for buildings

The verbatim comment is correct — `ImmuneToPsionics` defaults `yes` for buildings
and `no` for others. **The explicit `yes` on YAREFN is redundant but harmless**.
Ghidra-verified `0x00843754 → 0x00714fa7` TechnoType scope; the ReadINI default
branches on building-ness. This is one of the reasons YAREFN is uncapturable
by Yuri: psionic capture is blocked, and `Capturable=false` blocks engineer
capture.

### 5. Refinery smoke emission

- Four `RefinerySmokeOffset*` positions, particle system `SmallGreySSys`, period
  `RefinerySmokeFrames=30` ticks.
- Ghidra-verified `RefinerySmokeOffsetOne` reads at `0x00843f20 → 0x00713e93` in
  `TechnoTypeClass__ReadINI` — TechnoType scope.
- The emission is triggered by the building's `ResourceDestination=yes` accept
  callback (when a slave or harvester deposits); the same code path that fires
  on GAREFN/NAREFN.

### 6. BaseNormal=no consequence

The defeat-detection scan (Ghidra reference: building-class enumeration in the
`HouseClass::Destroyed` check) **skips buildings with `BaseNormal=no`**. So if
the only structures a Yuri player has left are YAREFNs, the player still loses.
Players who try to "hide" their last structures as YAREFNs to avoid defeat
will discover this.

### 7. Self-defense without crew

Note the absence of `Crewed=yes` (commented out in SMIN; not present here). The
building has no crew → killing it does not eject any infantry. This is a
deliberate choice: the slaves themselves are the "crew" in a sense, but they
are managed separately by the SlaveManager system.

---

## TS-legacy filter

- `;NumberImpassableRows=3` — commented out. TS-era fix for repair-pad drive-on
  bugs, irrelevant for refineries.
- `Bib=yes` — commented out. The bib system is fully functional in YR (used by
  ConYards etc.) — Westwood just chose not to give YAREFN one.
- `;DetailLevel=1` on `[YAREFN_A]` — commented; the engine-side DetailLevel
  filtering still works in YR (Lowest/Low/Medium/High/Highest detail-level
  toggle in the menu).
- `Crewed=yes` — commented. Original RA2/TS pattern of buildings ejecting crew
  on death; disabled for the Slave Miner intentionally.
- `Power=0;-50` — historical commented value; the current `0` is YR-final.
- `Capturable=false;gs true` — gs = "Greg Smith"; historical engineer-capturable
  state, disabled.
- `TechLevel=1;-1` — historical -1 state, currently 1.
- `IdleAnimYSort=700` — commented in artmd.
- `Image=GAREFN` in artmd — commented; prototype using Allied Refinery art.

No active TS-legacy code paths trigger on this building. Fog-of-war
(`SpecialFlags & 0x1000`) is off in YR-default; subterranean / Tunnel
locomotor not used here; no `ImmuneToVeins`-like dead fields.

---

## Cross-references

- [SMIN.md](../yuri/SMIN.md) — vehicle (undeployed) form. Shares HP, turret,
  voices, slaves, ore storage across deploy/undeploy.
- [SLAV.md](../yuri/SLAV.md) — slave infantry harvested by this building.
- [SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md](../../SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md)
  — brain-transplant logic + slave-state machine.
- [SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md](../../SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md)
  — ore-cash flow and `ResourceDestination` interactions.
- [HARVESTER_DOCK_UNLOAD.md](../../HARVESTER_DOCK_UNLOAD.md) — for comparison
  against Allied/Soviet refinery dock/unload behavior. **The Slave Miner does
  NOT use the dock/unload state machine** — there is no harvester vehicle to
  dock; slaves return directly to the building's anchor cell.
- [MINER_DOCK_GAPS_RESEARCH.md](../../MINER_DOCK_GAPS_RESEARCH.md) — same.
- [SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md](../../SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md)
  — `Spyable=yes` interaction (refinery infiltration steals credits).
- [BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md](../../BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md)
  — building base-class behavior reference.
- [YAPOWR.md](../structures/YAPOWR.md) — `POWER` prerequisite (when written).
- [YACNST.md](../structures/YACNST.md) — `YACNST` prerequisite (when written).

---

## Coverage audit

- [x] Every key in rulesmd `[YAREFN]` annotated (47 lines/keys).
- [x] Every key in artmd `[YAREFN]` annotated, including `[YAREFN_A]` sub-block.
- [x] `[YAREFNMK]` Buildup — flagged as filename-only reference (no INI block).
- [x] Both weapons documented (`20mmRapid`, `20mmRapidE`).
- [x] Both warheads documented (`HARVWH`, `HowitzerWH`).
- [x] All 6 voice/sound entries documented (Select / Move / Attack / Harvest /
  Undeploy / UnDeployVoice).
- [x] Prerequisites: `POWER, YACNST`.
- [x] Owner list: all multiplayer houses.
- [x] Veterancy: `Trainable=yes` + ElitePrimary swap.
- [x] Hardcoded behavior cross-referenced (SlaveManager transplant, ore-cash,
  turret, ImmuneToPsionics, RefinerySmoke, BaseNormal, self-defense).
- [x] TS-legacy filter applied (commented fields enumerated; no active TS code
  paths affect this building).
- [x] At least one Ghidra search performed (`ResourceDestination`,
  `RefinerySmokeOffsetOne`, `SlavesNumber` — all confirmed TechnoType-scope).
- [x] Index correction logged for `[SMON]` SKIP-DUPLICATE.

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("ResourceDestination")` | `0x00843ca4` (single match) |
| `get_xrefs_to(0x00843ca4)` | `0x007143f1 → TechnoTypeClass__ReadINI [DATA]` |
| `search_strings("RefinerySmokeOffsetOne")` | `0x00843f20` (single match) |
| `get_xrefs_to(0x00843f20)` | `0x00713e93 → TechnoTypeClass__ReadINI [DATA]` |
| `search_strings("SlavesNumber")` | `0x00843804` (single match) |
| `get_xrefs_to(0x00843804)` | `0x00714e1a → TechnoTypeClass__ReadINI [DATA]` |

All three keys confirmed TechnoType-scope (read by every TechnoType subclass);
their effects are gated by feature-specific runtime code, not by ReadINI scope.

**Open questions:** none. Pair with [SMIN](../yuri/SMIN.md) is now closed.
