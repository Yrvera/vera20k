---
name: pdplane-doc
description: PDPLANE — Paradrop Plane. Generic-faction aircraft spawned by
  ParaDropSpecial / AmericanParaDropSpecial superweapons. AircraftLocomotion,
  Landable=no, Ammo=100, Crewed=yes, faction-universal Owner. Drops infantry per
  AmerParaDropInf/AllyParaDropInf/SovParaDropInf/YuriParaDropInf country tables.
  No real weapon (Primary=ParaDropWeapon is "Dummy"). Closes the "superweapon-
  delivery aircraft" archetype.
metadata:
  type: project
---

# PDPLANE — Paradrop Plane (Cargo Plane)

**INI ID:** `PDPLANE`
**Display:** "Cargo Plane" (`UIName=Name:PDPLANE`). The CSF string resolves to
"Cargo Plane" in shipped YR despite internal name == external file name. Note:
[CARGOPLANE] (separate INI entry) **also displays as "Cargo Plane" or
"Transport Plane"** — the two share CSF/art via `Image=PDPLANE`.
**Section:** `[AircraftTypes]`
**Owner side:** **Generic — all 10 faction countries listed**
(British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry).
The most expansive Owner list documented so far. PDPLANE is faction-universal
because every country's `ParaDropSpecial` superweapon uses it as the delivery
plane.
**Role:** The aircraft that delivers paratroopers when a player activates the
`ParaDropSpecial` (Tech Outpost CAOUTP) or `AmericanParaDropSpecial`
(American faction-unique paradrop) superweapon. Carries infantry per the
*country-specific paradrop tables* in Rules ([General] section
`AmerParaDropInf`/`AmerParaDropNum`/`AllyParaDropInf`/etc.). Pure transport
aircraft — Primary=ParaDropWeapon is a *dummy* per Westwood's verbatim
comment.

---

## Note on Ghidra unavailability

Ghidra MCP server remains offline this iteration. All field-scope claims
cross-reference prior verified cheat-sheet entries.

---

## Rulesmd verbatim

```ini
[PDPLANE]
UIName=Name:PDPLANE
Name=Cargo Plane
Strength=400
Category=AirLift
Armor=light
TechLevel=-1
Primary=ParaDropWeapon	; Doesn't really fire it; dummy weapon
Spawned=yes	; Created by another object and therefore not player controllable
LeadershipRating=10
Selectable=no
RadarInvisible=no
Sight=0
Landable=no
MoveToShroud=yes
PitchAngle=0 ; default is 20 degrees
Speed=15
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
Points=30
ROT=2
Crewed=yes
Ammo=100
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=2
Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1}
MovementZone=Fly
ThreatPosed=0	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
ImmuneToPsionics=yes
CanPassiveAquire=no ; Won't try to pick up own targets
CanRetaliate=no; Won't fire back when hit
MoveSound=PDPlaneMoveLoop
DieSound=
CrashingSound=PDPlaneDie
ImpactLandSound=GenAircraftCrash
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:PDPLANE` — CSF resolves to "Cargo Plane".
- `Name=Cargo Plane` — internal description.
- `Category=AirLift` — *unique sidebar/AI bucket*. **Distinct from
  AirPower** (which is the combat-air bucket for Kirov/Hornet/BEAG).
  AirLift is the *transport-air* bucket. CARGOPLANE has Category=AirPower
  instead — a difference between the two paradrop-related planes.
- `LeadershipRating=10` — *AI leadership rating*, modest. **NEW field
  not yet in cheat-sheet** (cannot verify scope this iteration due to
  Ghidra offline). Used by AI for unit-priority decisions.

**Tech / availability**
- `TechLevel=-1` — **not directly buildable**. Spawned by superweapons.
- `Spawned=yes` — spawn-child marker. Verbatim comment: "Created by
  another object and therefore not player controllable".
- *No `MissileSpawn=yes`* — PDPLANE is not a kamikaze missile; it
  flies in, drops infantry, flies away. But also *no return-to-dock*
  (Landable=no). **A third spawn-child paradigm**: drop-and-leave
  (delivery aircraft).
- *No `Prerequisite=`* — spawn-only.
- `Owner=` — *all 10 country slots*. Universal faction availability.
  Required because every country's paradrop superweapon needs the
  plane.

**Combat — fragile but tankier than other spawn-children**
- `Strength=400` — **much higher than other spawn-children** (V3ROCKET=50,
  HORNET=75, ASW=30). PDPLANE survives most AA fire to complete a
  paradrop run. The fragility-vs-survival trade-off favors making
  the delivery successful.
- `Armor=light` — light armor.

**Combat — dummy weapon**
- `Primary=ParaDropWeapon` — **the verbatim comment "Doesn't really
  fire it; dummy weapon"** is the engine-side documentation: this
  weapon is declared to satisfy the Primary= rule but the engine
  never fires it.
- The dummy mechanism: the plane needs a Primary= for the
  Aircraft-class fire-check system (`Ammo>0` + `Primary=` required for
  movement state machine), but the actual "fire" event is *not the
  weapon* — it's the paradrop-infantry-spawn code path triggered by
  proximity to drop target.
- `Ammo=100` — *high ammo count*. Combined with the dummy weapon,
  this just satisfies "Ammo is hardwired to require ammo" without
  meaningful effect. 100 is just a big-enough-to-not-deplete number.

**Sight / radar**
- `Sight=0` — **zero vision**. Plane doesn't scout — flies straight
  to designated drop site.
- `RadarInvisible=no` — appears on enemy radar (defenders see paradrop
  incoming).
- `Landable=no` — **does not land**. The plane flies in, drops
  infantry mid-air, exits the map. *No return-to-base cycle*.
- `MoveToShroud=yes` — can fly into unexplored cells.

**Mobility**
- `Speed=15` — same as V3ROCKET. Moderate.
- `PitchAngle=0` — *flat horizontal flight*. Verbatim comment "default
  is 20 degrees" — most aircraft pitch up/down 20° during banking;
  PDPLANE is forced flat to drop infantry cleanly out the back.
- `ROT=2` — *very slow turn*. Same as Kirov. The plane flies straight-
  line routes without much maneuvering.
- `Locomotor={4A582746-...}` — AircraftLocomotion (5th locomotor type,
  per BEAG/HORNET cheat-sheet). Fixed-wing aircraft physics.
- `MovementZone=Fly`.

**Combat behavior**
- `Crewed=yes` — *crew ejects on death*? Or pilots are notional?
  Aircraft Crewed=yes typically doesn't eject crew (the crash code
  path handles it differently). Open question.
- `ThreatPosed=0` — AI doesn't see PDPLANE as a threat (it's a
  delivery aircraft, not combat).
- `ImmuneToPsionics=yes` — Yuri can't mind-control the plane.
- `CanPassiveAquire=no` (verbatim "Won't try to pick up own targets")
  — no auto-target acquisition.
- `CanRetaliate=no` (verbatim "Won't fire back when hit") — no return-
  fire. **The triple-disable** (PreventAttackMove implied by Spawned=
  yes spawn-only nature + CanPassiveAquire=no + CanRetaliate=no)
  enforces "fly the route, don't engage, just deliver" behavior.

**Voice / sound bindings**
- *All Voice* slots* not set — silently inherit defaults (no voice).
- `MoveSound=PDPlaneMoveLoop` — engine drone loop (3-sample `sparlo2*`
  random looping pool).
- `DieSound=` — empty.
- `CrashingSound=PDPlaneDie` — crash plummet SFX (single sample
  `sparlo3`).
- `ImpactLandSound=GenAircraftCrash` — generic impact.

**No AuxSound1/AuxSound2** — PDPLANE doesn't take off from a parent
or land at one. Spawned mid-air at map edge → flies to drop site →
exits map edge. No takeoff/landing events.

**Destruction**
- `Explosion=TWLT070,...` — explosion pool.
- `MaxDebris=2` — minimal.
- `DamageParticleSystems=SparkSys,SmallGreySSys` — sparks + smoke (the
  comment "Sparks don't work well here" from spawn-missile docs does
  NOT apply — PDPLANE flies low enough that sparks render OK).

**No `Selectable=no` Westwood-bug commentary** — PDPLANE has
`Selectable=no` *explicitly set*. Same as V3ROCKET — works because
PDPLANE doesn't land (Landable=no). **Confirms the HORNET-bug theory
once again**: Selectable=no only breaks landing-cycle aircraft.

---

## Artmd verbatim

```ini
[PDPLANE] ; Paradrop Plane
Cameo=OBMBICON
Voxel=yes
PrimaryFireFLH=0,32,0
DisableVoxelCache=yes	; HY
DisableShadowCache=yes	; HY
```

### Key-by-key annotation

- `Cameo=OBMBICON` — *"OBMB"* prefix likely from "ORCA-BOMBER" early
  development; reused for PDPLANE cameo asset.
- `Voxel=yes` — rendered from `pdplane.vxl` + `.hva`.
- `PrimaryFireFLH=0,32,0` — same Y=32 offset as HORNET/ASW (sideways
  paratrooper drop position).
- `DisableVoxelCache=yes ; HY` — **HY = developer initials** (likely
  Hyo Yi or similar). Performance flag like SHAD's
  `; SJM: this is a major cache hog`. PDPLANE's voxel rotation
  during flight is heavy enough to disable the general voxel cache.
- `DisableShadowCache=yes ; HY` — same for shadow rendering.

**Note**: `DisableVoxelCache=yes` was also on SHAD with SJM (Steve J.
Maetzold) developer initials. Now PDPLANE shows HY initials. Different
Westwood devs flagged different units' cache problems. Possibly a
cache-pressure performance audit happened during YR development.

---

## Weapons

### Dummy primary — `[ParaDropWeapon]`

```ini
[ParaDropWeapon]	; Dummy weapon, not actually fired.
Damage=60
ROF=130
Range=4
Projectile=AAHeatSeeker2 ; was HeatSeeker
Speed=30
Warhead=MaverickHE
Burst=1
```

- **Verbatim section header**: "Dummy weapon, not actually fired."
- Despite being a dummy, the weapon has full INI definition (Damage=60,
  ROF=130, etc.). Engine reads it, allocates internal weapon-class
  pointer, but the fire code path never triggers.
- `Projectile=AAHeatSeeker2` — `;was HeatSeeker` historical iteration.
- `Warhead=MaverickHE` — anti-air HE warhead. Not actually applied
  (dummy).
- `Burst=1` — explicit, even though unused.

**Why declare a dummy**: the Aircraft-class engine machinery requires
a Primary=. Without it, the unit fails validation and crashes /
silently no-ops. Defining a dummy weapon (with `Damage=60` etc. as
placeholders) lets the engine load the unit normally; the actual
"weapon" behavior (paradrop spawning) is handled in separate
hardcoded paradrop code.

**The actual paradrop logic** is engine-side (NOT INI-defined):
1. Player activates `ParaDropSpecial` (or `AmericanParaDropSpecial`)
   superweapon, clicks a drop target.
2. Engine spawns a PDPLANE at map edge (or specific spawn cell).
3. PDPLANE flies in straight line toward target along
   AircraftLocomotion.
4. When plane reaches within `ParadropRadius=1024` leptons of target,
   engine queries the country-specific paradrop table:
   - American: `AmerParaDropInf=E1, AmerParaDropNum=8`.
   - Allied (non-American): `AllyParaDropInf=E1, AllyParaDropNum=6`.
   - Soviet: `SovParaDropInf=E2, SovParaDropNum=9`.
   - Yuri: `YuriParaDropInf=INIT, YuriParaDropNum=6`.
5. Engine spawns the per-country infantry (8 GIs / 6 GIs / 9
   Conscripts / 6 Initiates), each with a Parachute=PARACH attached.
6. Parachutes drift down via `ParachuteMaxFallRate=-3` (Rules global
   line 68).
7. PDPLANE continues flying, exits map edge, despawns.

---

## Paradrop superweapon entries (Rules-global)

### `[ParaDropSpecial]` — generic paradrop (Tech Outpost CAOUTP)

```ini
[ParaDropSpecial]
UIName=Name:Para
Name=Paratrooper Drop
IsPowered=false
RechargeVoice=
ChargingVoice=
ImpatientVoice=
SuspendVoice=
RechargeTime=4
Type=ParaDrop
Action=ParaDrop
SidebarImage=PARAICON
ShowTimer=no
DisableableFromShell=no
```

- `IsPowered=false` — *does not require player power* to function.
- `RechargeTime=4` — 4 minutes between paradrops.
- `Type=ParaDrop` / `Action=ParaDrop` — engine-side action class. The
  paired Type+Action identifies the engine's hardcoded paradrop
  dispatcher.
- `SidebarImage=PARAICON` — sidebar superweapon button.
- `ShowTimer=no` — *timer is hidden* on sidebar. Player can't see
  when ready.

**Provider**: Tech Outpost ([CAOUTP]) building grants ParaDropSpecial
when captured.

### `[AmericanParaDropSpecial]` — American faction-unique paradrop

```ini
[AmericanParaDropSpecial]
UIName=Name:APara
Name=American Paratrooper Drop
IsPowered=false
RechargeTime=4
Type=AmerParaDrop
Action=AmerParaDrop
SidebarImage=APARICON
ShowTimer=no
DisableableFromShell=no
```

- **`Type=AmerParaDrop`** vs ParaDrop — *different engine action
  class*. The American superweapon triggers a *separate* paradrop
  code path that queries `AmerParaDropInf`/`AmerParaDropNum`
  specifically (8 GIs).
- `SidebarImage=APARICON` — separate sidebar icon.

**Provider**: American Airforce Command HQ (or similar; per line 12362
in rulesmd, a building grants AmericanParaDropSpecial).

### Rules-global paradrop tables

From rulesmd lines 235-251:

```ini
;************ American Paradrop Special Rules ***********
;These two lists _must_ have the same number of elements, otherwise bad crashiness will result

AmerParaDropInf=E1
AmerParaDropNum=8

AllyParaDropInf=E1
AllyParaDropNum=6

SovParaDropInf=E2
SovParaDropNum=9

YuriParaDropInf=INIT
YuriParaDropNum=6
```

- **Verbatim Westwood warning**: "These two lists _must_ have the
  same number of elements, otherwise bad crashiness will result".
  Engine assumes paired-array indexing — if lengths differ, undefined
  behavior (crash).
- Commented `;AmerParaDropInf=E1,GHOST,ENGINEER` / `;AmerParaDropNum=6,6,6`
  — historical multi-type drop, simplified to single-type.

**Per-country drop count + type**:
| Country | Type | Count |
|---------|------|-------|
| American | E1 (GI) | 8 |
| Allied (non-American) | E1 | 6 |
| Soviet | E2 (Conscript) | 9 |
| Yuri | INIT (Initiate) | 6 |

**Soviets get the most paratroopers** (9 vs 6/8) — possibly to
compensate for Conscript being weaker than American GI.

### `ParadropRadius=1024` (Rules-global)

From rulesmd line 202:
```
ParadropRadius=1024   ; Drop paratroopers if plane is within this many leptons from drop site.
```

- **1024 leptons** = 4 cells (256 leptons per cell). The plane
  starts dropping when it's within 4 cells of the click target. The
  paratroopers spawn over the next several frames as the plane
  passes over the target.

---

## Voices / sounds

```ini
[PDPlaneMoveLoop]
Sounds=sparlo2a sparlo2b sparlo2c
Control=random loop all
Limit=3
Priority=high
Range=20
Volume=35

[PDPlaneDie]
Sounds=sparlo3
Range=20
Volume=45
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| (all Voice* empty) | n/a | No player-interaction voices |
| `MoveSound=PDPlaneMoveLoop` | `[PDPlaneMoveLoop]` | Looping engine drone during flight (3-sample random-loop, Range=20, Volume=35 quiet ambient) |
| `DieSound=` (empty) | n/a | No instant-death SFX |
| `CrashingSound=PDPlaneDie` | `[PDPlaneDie]` | Crash plummet SFX (single sample) |
| `ImpactLandSound=GenAircraftCrash` | shared | Ground impact |

**No `Report=` on the dummy weapon** — `ParaDropWeapon` has no
`Report=` line, consistent with dummy status. Even if it were
"fired", no sound would play.

**The `sparlo*` audio family** — "spawn-loop"? Or "spar" for some
plane name? Same naming root as `[PDPlaneDie]=sparlo3`. Likely "spy
plane long-range" historical naming reused for PDPLANE.

---

## Hardcoded behavior

### 1. Spawned=yes but NOT MissileSpawn AND NOT Landable

**New spawn-child paradigm**: drop-and-exit aircraft.

Three paradigms documented across spawn-children:
- **Kamikaze missile** (V3ROCKET, DMISL, CMISL): `Spawned=yes +
  MissileSpawn=yes`. Dies on impact, no Landable. RocketLocomotion.
- **Return-to-dock** (HORNET, ASW): `Spawned=yes + Landable=yes + no
  MissileSpawn`. Lands at parent, refuels. AircraftLocomotion.
- **Drop-and-exit** (PDPLANE): `Spawned=yes + Landable=no + no
  MissileSpawn`. Flies in, drops cargo, exits map edge. No return.
  AircraftLocomotion.

The PDPLANE paradigm requires neither dock nor on-impact death — it
just flies straight off the map after delivering its payload.

### 2. Dummy Primary=ParaDropWeapon

The verbatim Westwood comment "Doesn't really fire it; dummy weapon"
documents the engine-side dummy pattern:
- Aircraft validation requires `Primary=`.
- Engine declares the dummy weapon at INI parse time.
- The actual fire-handler code path is bypassed (paradrop spawn is a
  separate code branch).
- The dummy weapon's `Damage=60, Warhead=MaverickHE, etc.` are *never
  applied* — they exist as placeholder values to satisfy the parser.

**Open question**: which engine function bypasses the weapon-fire
code for PDPLANE? Likely the Aircraft-class movement code checks for
a specific superweapon-spawn-type flag before firing the Primary,
and short-circuits to paradrop dispatch instead. Open Ghidra trace
required.

### 3. Country-specific paradrop tables

The Rules-global `AmerParaDropInf`/`AllyParaDropInf`/`SovParaDropInf`/
`YuriParaDropInf` tables drive the per-country infantry-type lookup.
The engine identifies the player's faction via the unit's `Owner=`
list match against the country-of-the-firing-player, then queries the
appropriate table.

**Cheat-sheet refs**: These Rules-global fields would be in
`RulesClass__ReadGeneral` per cheat-sheet (0x00671xxx range). Not yet
verified this iteration.

### 4. ParadropRadius=1024 (Rules-global)

Sets the proximity-to-target threshold for paratrooper deployment.
1024 leptons = 4 cells. The plane begins dropping when within 4
cells of target; multiple paratroopers deploy in sequence as the
plane passes over.

### 5. Type=ParaDrop vs Type=AmerParaDrop

Two engine-side action classes:
- `Type=ParaDrop` — uses non-American paradrop table (Ally/Sov/Yuri
  based on owning faction).
- `Type=AmerParaDrop` — *always* uses AmerParaDropInf (8 GIs).

The American Airforce Command HQ provides `AmericanParaDropSpecial`
even if the player isn't American — wait, that doesn't make sense.
Probably both superweapons exist on different buildings:
- `ParaDropSpecial` on Tech Outpost (CAOUTP).
- `AmericanParaDropSpecial` on American Airforce Command HQ
  (faction-restricted).

**The American faction gets *both* paradrops** — better count (8)
from their own superweapon plus the Tech Outpost ParaDrop if
captured. Non-American factions only get ParaDropSpecial.

### 6. No AuxSound1/AuxSound2

PDPLANE has no takeoff or landing events:
- Spawns mid-air at map edge (no takeoff).
- Exits map edge (no landing).

The complete absence of `AuxSound1=` / `AuxSound2=` lines (vs other
documented aircraft) marks PDPLANE as a *transient delivery aircraft*.

### 7. PitchAngle=0 (flat flight)

Most aircraft have `PitchAngle` default of 20 degrees (per verbatim
PDPLANE comment). PDPLANE has 0 — completely flat. **Likely related
to the paradrop drop mechanism**: the parachute deployment animation
expects the plane to be perfectly horizontal so paratroopers fall
straight down.

### 8. CanRetaliate=no + CanPassiveAquire=no + ThreatPosed=0

Complete pacifist configuration. PDPLANE doesn't engage, doesn't
return fire, doesn't even register as a threat for AI prioritization.
Pure delivery.

---

## TS-legacy filter

- `;AmerParaDropInf=E1,GHOST,ENGINEER` / `;AmerParaDropNum=6,6,6` —
  commented historical multi-type drop, simplified to E1 only.
- `;was HeatSeeker` on ParaDropWeapon's Projectile — historical.
- `;default is 20 degrees` PitchAngle comment — verbatim engine
  default reference.
- No `ImmuneToVeins`, no `Subterranean`. YR-active mechanism.

---

## Comparison: spawn-child paradigm complete (PDPLANE adds the 3rd)

| Field | Kamikaze (V3ROCKET/DMISL/CMISL) | Return-to-dock (HORNET/ASW) | **Drop-and-exit (PDPLANE)** |
|-------|----------------------------------|------------------------------|------------------------------|
| Strength | 50 | 75 / 30 | **400** (most durable) |
| Spawned | yes | yes | yes |
| MissileSpawn | yes | no | **no** |
| Landable | yes (unused) | yes (active) | **no** |
| Locomotor | RocketLocomotion | AircraftLocomotion | **AircraftLocomotion** |
| Damage path | Rules-global Warhead | Primary weapon | **Engine-side paradrop spawn** |
| Cycle | one-shot suicide | sortie-return-reload | **fly in, drop, exit** |
| Owner | parent faction only | parent faction only | **universal (10 factions)** |
| AuxSound1 | active (launch) | active (takeoff) | **not set** |
| AuxSound2 | commented | active (landing) | **not set** |

**PDPLANE is the only universal-Owner spawn-child** — flexibility to
serve any faction's paradrop superweapon. Other spawn-children inherit
the parent's faction.

**The 3 paradigms summary**:
- Kamikaze: damage delivery, dies on impact.
- Return-to-dock: reusable strike, infinite sorties.
- Drop-and-exit: cargo delivery, single-use disposal off-map.

---

## Cross-references

- [CARGOPLANE.md] — pending. Similar plane with `Image=PDPLANE`
  redirect, Category=AirPower vs PDPLANE's AirLift. Used for
  GeneticMutator or other cargo deliveries.
- [CAOUTP.md](../structures/CAOUTP.md) — pending. Tech Outpost
  building, provides ParaDropSpecial superweapon.
- American Airforce Command HQ — pending. Provides
  AmericanParaDropSpecial.
- [HORNET.md](../allied/HORNET.md) + [ASW.md](../allied/ASW.md) —
  return-to-dock paradigm peers.
- [V3ROCKET.md](../soviet/V3ROCKET.md) + [DMISL.md](../soviet/DMISL.md)
  + [CMISL.md](../yuri/CMISL.md) — kamikaze paradigm peers.
- [BEAG.md](../allied/BEAG.md) — peer standalone AircraftType
  (player-buildable; AircraftType scope discovered there).

---

## Coverage audit

- [x] Every rulesmd key annotated (~35 keys).
- [x] Every artmd key annotated (6 keys including HY-initialed cache
  flags).
- [x] **Dummy ParaDropWeapon explained** (Westwood's "doesn't really
  fire it" verbatim).
- [x] All voice/sound bindings documented.
- [x] Spawn-child status (Spawned=yes, TechLevel=-1, no MissileSpawn,
  Landable=no — drop-and-exit paradigm).
- [x] Universal Owner=all-10-countries pattern documented.
- [x] Hardcoded behavior: drop-and-exit paradigm, dummy Primary
  mechanism, country-specific paradrop tables, ParadropRadius=1024
  threshold, Type=ParaDrop vs Type=AmerParaDrop action distinction,
  PitchAngle=0 flat-flight requirement.
- [x] Cross-referenced paradrop superweapons (ParaDropSpecial,
  AmericanParaDropSpecial).
- [x] TS-legacy filter applied.
- [x] Comparison table closes the spawn-child paradigm trio (kamikaze,
  return-to-dock, drop-and-exit).
- [ ] **No Ghidra verification this iteration** (MCP server offline).

**Ghidra status**: MCP server still disconnected. No new cheat-sheet
entries. Field-scope claims cross-reference prior verified entries.

**Re-confirmed cheat-sheet:**
- `Spawned`, `Landable`, `CanPassiveAquire`, `CanRetaliate`, AircraftLocomotion
  GUID — all per prior iterations.

**Open questions:**
- `LeadershipRating` field scope (not yet in cheat-sheet) — likely
  TechnoType.
- The dummy-weapon bypass mechanism — which engine function handles
  PDPLANE's paradrop spawn instead of weapon fire?
- `Type=ParaDrop` / `Type=AmerParaDrop` superweapon action-class —
  Ghidra trace needed to understand the engine-side dispatcher.
- `[CARGOPLANE]` vs `[PDPLANE]` — both use Image=PDPLANE but have
  different categories. What's CARGOPLANE specifically used for?
  Likely Genetic Mutator delivery or other superweapon-spawn-payload.
  Open follow-up iteration.

**5 spawn-children + 1 standalone AircraftType (BEAG) documented** —
spawn-child paradigm trio now complete.
