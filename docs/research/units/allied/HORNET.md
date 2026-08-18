---
name: hornet-doc
description: HORNET — Aircraft Carrier's spawn-child plane. Return-to-dock spawner
  pattern (NOT MissileSpawn=yes). HornetBomb basic + HornetBombE Elite weapon
  upgrade (CARRIER's ElitePrimary swaps the child's weapon). Strength=75/Ammo=1.
  HornetCollision secondary = kamikaze fallback. New cheat-sheet: Landable
  (AircraftType), AuxSound1 (TechnoType).
metadata:
  type: project
---

# HORNET — Carrier Plane

**INI ID:** `HORNET`
**Display:** "Hornet" (`UIName=Name:HORNET`)
**Section:** `[AircraftTypes]`
**Owner side:** Allied (British, French, Germans, Americans, Alliance) — but
**not directly buildable** (`TechLevel=-1`); only exists as a spawn-child of
the [CARRIER](../allied/CARRIER.md) Aircraft Carrier.
**Role:** Aircraft Carrier's reusable strike plane. **Return-to-dock spawn
pattern** (not kamikaze suicide like V3ROCKET/CMISL — Hornet lands back on
Carrier to reload). CARRIER spawns 4 Hornets with `SpawnReloadRate>0`
enabling rearm cycles. Pairs with ASW (Destroyer's helicopter) as the two
Allied return-to-dock spawn-children.

---

## Rulesmd verbatim

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

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:HORNET` — CSF key.
- `Name=Hornet` — internal description.
- `Category=AirPower`.

**Tech / availability — spawn-only**
- *No `Prerequisite=`* — Hornets aren't built from any factory. They
  exist only as Carrier-spawned aircraft.
- `TechLevel=-1` — **unbuildable** by player. Standard for spawn-
  children.
- `Owner=British,French,Germans,Americans,Alliance` — 5 Allied houses.
  Match the parent CARRIER's owner list.
- `Spawned=yes` — **marks the unit as a spawn-child**. TechnoType
  `0x008437d8 → 0x00714e7d` (per cheat-sheet from MIND). Required for
  the SpawnManager system on the parent (CARRIER).
- *No `MissileSpawn=yes` line* — **distinguishes Hornet from
  V3ROCKET/CMISL one-shot missiles**. Without MissileSpawn, the
  Hornet is a *reusable* spawn-child: flies to target, fires, returns
  to parent, reloads, ready for next sortie.
- *No `Selectable=no` line* — Hornet *is technically selectable*. The
  verbatim Westwood comment `;Selectable=no SJM: this should be here
  but is commented out because bug prevents aircraft from landing`
  reveals: Westwood *wanted* Hornets to be unselectable, but doing so
  triggers a bug where the aircraft can't land back at the Carrier.
  *Shipped state: player can technically click and select Hornets*,
  though usually not useful since the SpawnManager controls their AI.

**Combat — defense**
- `Strength=75` — *very fragile*. Among the lowest HP units.
- `Armor=light` — light armor.

**Combat — dual-weapon (Primary normal + Secondary kamikaze)**
- `Primary=HornetBomb` — *the working weapon*. 40 damage drop-bomb,
  ROF=3, Range=5, NormalBomb projectile, ORCAAP warhead. See Weapon
  section.
- `Secondary=HornetCollision` — *kamikaze fallback*. **The verbatim
  weapon comment says: "A crashing Hornet turns into this bullet at
  the last second"**. When the Hornet is shot down or crashes, the
  unit transforms into a HornetCollision projectile at the last
  moment, dealing 100 damage on impact. **Kamikaze death weapon — but
  only triggered on crash, not voluntary suicide**.
- `ElitePrimary=HornetBombE` — *elite swap on CARRIER's veterancy*.
  When the parent CARRIER reaches Elite, its spawned Hornets fire the
  upgraded HornetBombE (Damage 40→80, Warhead ORCAAP→ARTYHE with
  Deform=15% terrain crater).

**Sight / radar**
- `Sight=2` — *very short* (just 2 cells). The Hornet doesn't scout —
  it flies to a designated target.
- `RadarInvisible=no` — appears on enemy radar.
- `Landable=yes` — **Ghidra-verified AircraftTypeClass__ReadINI** at
  `0x0081804c → 0x0041cc54` → **AircraftType+0xE0A (byte, ReadBool)
  [BINARY-VERIFIED audit 29]** — re-confirms audit-26 BEAG cumulative.
  Assembly-context proof: writeback `MOV byte ptr [ESI + 0xe0a], AL` at
  0x0041cc67. The flag enables landing-cycle behavior (returns to
  Carrier between sorties). Without Landable=yes, the aircraft would
  have no return-to-dock pathway.
- `MoveToShroud=yes` — can attack into unexplored shroud.
- `PipScale=Ammo` — pip bar shows ammo (1 pip for 1 missile).

**Mobility**
- `Speed=12` — fast.
- `PitchSpeed=.9, PitchAngle=0` — pitch animation parameters.
- `ROT=3` — moderate turn rate.
- `Locomotor={4A582746-...}` — **AircraftLocomotion** (same GUID as
  BEAG). Fixed-wing aircraft physics.
- `MovementZone=Fly` — fly-zone.
- `MovementRestrictedTo=Water` — **the verbatim comment: "See if this
  will affect landing only"**. *Hornet's landing is restricted to
  water cells* (the design intent). **[INCORRECT — DEAD INI on
  Hornet, audit 29]**: `MovementRestrictedTo` is **UnitType-scope only**
  (string @ 0x00845d64, single xref @ 0x00747837 → `UnitTypeClass__ReadINI`).
  Hornet is `[AircraftTypes]` (parsed by `AircraftTypeClass__ReadINI`
  which calls `TechnoTypeClass__ReadINI` but NOT `UnitTypeClass__ReadINI`),
  so this key is never read for the Hornet. **Hornet's
  `MovementRestrictedTo=Water` has no engine effect.** Resolves the
  doc's prior open question — the Westwood "See if this will affect
  landing only" comment found out it doesn't.

**Aircraft-class specifics**
- `Crewed=no` — no crew.
- `Ammo=1` — single missile capacity. Same as BEAG.
- `GuardRange=30` — autonomous attack range in guard mode.

**Voice / sound bindings — almost all empty**
- `VoiceSelect=`, `VoiceMove=`, `VoiceAttack=`, `VoiceFeedback=`,
  `DieSound=` — **all empty**. Hornets don't have voices (consistent
  with the `;Selectable=no` design intent — players weren't supposed to
  interact with them).
- `CrashingSound=HornetDie` — crash plummet SFX.
- `ImpactLandSound=GenAircraftCrash` — impact SFX.
- `AuxSound1=HornetTakeoff` — **Ghidra-verified TechnoType** at
  `0x00844240 → 0x00712e18` → **TechnoType+0x52C (int VocClass index)
  [BINARY-VERIFIED audit 29]**. Assembly-context proof: preload
  `MOV EDI, dword ptr [EBP + 0x52c]` at 0x00712e03. Plays when the
  Hornet launches from the Carrier deck.
- `AuxSound2=HornetLanding` — **Ghidra-verified TechnoType** at
  `0x00844234 → 0x00712e48` → **TechnoType+0x530 (int VocClass index)
  [BINARY-VERIFIED audit 29]** (sibling to AuxSound1, parser xref at
  0x00712e48 — verifies the doc's "adjacent address" inference). Plays
  when Hornet lands back on Carrier deck.

**Notable**: **Hornet is the first unit documented where AuxSound1 and
AuxSound2 are actively used** (not commented out as in SHAD/ZEP/SCHP
templates). The Carrier-launch and Carrier-landing cycle is the
canonical use case for AuxSound1/AuxSound2.

**Veterancy — reduced ability list**
- `VeteranAbilities=STRONGER,FIREPOWER` — only 2 abilities (vs full
  unit's 4-5). Hornets don't get SIGHT or FASTER veteran upgrades.
- `EliteAbilities=STRONGER,FIREPOWER` — same 2.
- Plus the weapon swap to HornetBombE when CARRIER goes Elite.

**Why reduced**: Hornets are *controlled by the SpawnManager*, not
the player. They don't gain veterancy independently — they inherit
the parent CARRIER's rank. The reduced ability list reflects that
they're not a full-fledged veterancy unit.

**Behavior**
- `ImmuneToPsionics=yes` — Yuri can't mind-control the Hornet.
- `ThreatPosed=10` — modest AI threat.
- `Explosion=TWLT070,...` — explosion pool.
- `MaxDebris=2` — minimal debris.

**Commented Dock= alternatives**
- `;Dock=NAHPAD,GAHPAD` — commented (would have docked at any helipad).
- `;Dock=GAAIRC,AMRADR` — commented (would have docked at Airforce
  Command HQ / American Radar).
- *No active Dock= line* — Hornet docks at the *parent spawner*
  (CARRIER), not at any building. The SpawnManager handles this
  parent-binding directly. Different mechanism from BEAG's
  `Dock=GAAIRC,AMRADR` (which docks at buildings).

---

## Artmd verbatim

```ini
[HORNET] ; Carrier plane
Cameo=PROICON
Voxel=yes
PrimaryFireFLH=0,32,0
```

### Key-by-key annotation

- `Cameo=PROICON` — **shared cameo "PROICON"** (probably "PROjectile
  ICON" — generic missile/projectile cameo). Same cameo as ASW.
  Hornets don't have a unique sidebar button since they're not
  player-built.
- `Voxel=yes` — rendered from `hornet.vxl` + `hornet.hva`.
- *No `Remapable=yes`* — *unusual*. Most voxel units are remapable.
  Hornet might use the parent's house color via inheritance from the
  SpawnManager, or this is an oversight.
- `PrimaryFireFLH=0,32,0` — bomb-drop offset:
  - X=0 (centered).
  - Y=32 (32 leptons offset to one side — bomb drops from the side
    of the plane, not centered).
  - Z=0 (water/ground level relative to plane altitude).

**No `AltCameo=`** — single cameo.

---

## Weapons

### Primary — `[HornetBomb]`

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

- `Damage=40` — modest single-bomb damage.
- `ROF=3` — *very fast firing* (~0.2 sec at 15fps) — but combined with
  `Ammo=1` on Hornet, it fires once per sortie. ROF only matters in
  edge cases (e.g. Burst with Ammo>1, which Hornet doesn't have).
- `Range=5` — short.
- `Projectile=NormalBomb` — see projectile block. *Drops* the bomb;
  gravity does the work.
- `Speed=30` — projectile speed.
- `Warhead=ORCAAP` — same warhead as BEAG Maverick2. Universal
  100% Verses with PenetratesBunker=yes.
- `Report=HornetAttack` — fire SFX (`vospatta` — shared with Osprey
  attack, Limit=3 concurrent).

### Elite — `[HornetBombE]`

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

**Two changes vs basic:**
1. `Damage=80` (vs 40) — **2× damage**.
2. `Warhead=ARTYHE` (vs ORCAAP) — *artillery HE warhead* with
   **Deform=15%** terrain deformation. Compare with SCHP's SCHOPWH
   warhead which has the same Deform=15% mechanic.

**Activation**: triggered when *parent CARRIER reaches Elite rank*.
Confirmed pattern from CARRIER doc (CARRIER has `ElitePrimary=
HornetBombE` despite Hornet being the actual firing unit — the parent's
veterancy promotes the child's weapon).

### Secondary — `[HornetCollision]`

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

- **Verbatim section comment**: *"A crashing Hornet turns into this
  bullet at the last second"*. The Hornet's death animation transforms
  the falling Hornet *into a HornetCollision projectile* just before
  ground impact. The bullet then completes the impact damage.
- `Damage=100` — *2.5× the normal HornetBomb*. The kamikaze death-
  smack deals more damage than the deliberate strike.
- `Range=3` — short impact range.
- `Projectile=AAHeatSeeker2` — verbatim comment: "will be Hornet
  shaped bullet". The bullet uses AAHeatSeeker2 visual but probably
  has a custom Hornet voxel reskin (or never got finished).
- `Warhead=AP` — Armor Piercing (vs Bomb's ORCAAP). Anti-armor on
  impact.
- `Report=HornetCollision` — distinct collision SFX (`gexpshaa`,
  Limit=2 concurrent).
- `Bright=yes` — palette-flash on impact.

**Mechanism note**: HornetCollision isn't a *manually fired* secondary
weapon — it's triggered by the crashing-aircraft death code. The
verbatim Westwood comment makes this clear. *Open*: which Ghidra
function transforms the falling Hornet into the HornetCollision
projectile? Some `Aircraft::Crash` or `Aircraft::DamageReceived`
handler. Open follow-up.

### Projectile — `[NormalBomb]`

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

- `Arm=2` — 2-frame arming.
- `Shadow=no`, `Image=DRAGON` — dragon-missile-shape sprite.
- `Proximity=yes` — proximity-fused (detonates near target).
- `Ranged=yes` — fuse-based range check.
- `ROT=1` — *minimal tracking* — essentially a dropped bomb without
  homing.
- `IgnoresFirestorm=yes` — passes through Firestorm Wall (TS-legacy
  defense). Live-but-dormant in YR since Firestorm wasn't included.

### Warhead — `[ORCAAP]`

Already documented in [BEAG.md](./BEAG.md#warhead--orcaap). Universal
100% Verses with PenetratesBunker=yes (bypasses Tank Bunker).

### Warhead — `[ARTYHE]` (elite)

```ini
[ARTYHE]
CellSpread=1
PercentAtMax=.25
Wall=yes, Wood=yes
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

- **Identical structure to SCHOPWH** (Siege Chopper warhead).
  CellSpread=1 AoE with `Deform=15%` + `DeformThreshhold=120` —
  terrain cratering. Elite Hornet bombs crater the ground.
- `Verses=100%/80%/60%/100%/60%/60%/100%/100%/60%/100%/100%`:
  Stronger vs unarmored (100%), light (100%), wood (100%), steel
  (100%). Moderate vs medium/heavy (60%). 60% vs concrete.

---

## Voices / sounds

```ini
[HornetAttack]
Sounds= vospatta
Limit=3
Control= interrupt
FShift= -5 5
Volume=60

[HornetTakeoff]
Sounds= vhortaka vhortakb
Priority=low
FShift= -10 10
Control= random
Volume=45

[HornetLanding]
Sounds= vhorlana vhorlanb
Control= random predelay
Priority=low
Delay=0 200
FShift= -10 10
Volume=45

[HornetCollision]
Sounds=gexpshaa gexpshaa
Control=interrupt random
Priority=low
FShift= -10 10
Limit=2
VShift=10
Volume=25

[HornetDie]
Sounds=vhordiea vhordieb
Control=random
FShift=-5 5
Volume=60
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=`, `VoiceMove=`, `VoiceAttack=`, `VoiceFeedback=` | (empty) | Hornet has no player-interaction voices |
| `DieSound=` (empty) | n/a | No death-frame SFX (handled by CrashingSound chain) |
| `CrashingSound=HornetDie` | `[HornetDie]` | Sustained crash SFX (2-sample random) |
| `ImpactLandSound=GenAircraftCrash` | shared | Ground-impact SFX |
| `AuxSound1=HornetTakeoff` | `[HornetTakeoff]` | **Plays when Hornet launches from Carrier deck** |
| `AuxSound2=HornetLanding` | `[HornetLanding]` | **Plays when Hornet returns and lands on Carrier deck** |
| `Report=HornetAttack` (HornetBomb weapon) | `[HornetAttack]` | Bomb-drop SFX (Limit=3 concurrent — `vospatta` shared with Osprey) |
| `Report=HornetCollision` (HornetCollision weapon) | `[HornetCollision]` | Crash-impact SFX (Limit=2) |

**`HornetTakeoff`/`HornetLanding` are the canonical AuxSound1/AuxSound2
usage** — most units have these commented out, but the Hornet's
takeoff/landing cycle (sortie from Carrier, return to dock) actively
uses them. Audible cycle: takeoff sound → flight → bomb drop → land
sound → silence while reloading → takeoff sound for next sortie.

---

## Hardcoded behavior (Ghidra-verified)

### 1. Landable=yes at AircraftType scope (continues BEAG-discovered scope)

**Ghidra-verified AircraftTypeClass__ReadINI** at `0x0081804c → 0x0041cc54`.
**NEW cheat-sheet entry** in the AircraftType scope (joining `Fighter`
and `AirportBound` from BEAG iteration).

The AircraftType scope continues to grow:
- `Fighter` (0x00818034 → 0x0041cc84)
- `AirportBound` (0x0081803c → 0x0041cc6e)
- **`Landable`** (0x0081804c → 0x0041cc54) — **NEW**

Strings clustered at 0x008180xx in memory suggest an AircraftType-
specific string table for ReadINI keys.

### 2. AuxSound1/AuxSound2 TechnoType

`AuxSound1` (TechnoType `0x00844240 → 0x00712e18`, **NEW cheat-sheet
entry**). Verbatim comment ";Taking off". A *2-slot auxiliary sound*
system for unit-specific events:
- AuxSound1 = takeoff event (aircraft launching from dock).
- AuxSound2 = landing event (aircraft returning to dock).

Most documented units have these *commented out* (`;AuxSound1=Dummy ;Taking off`
on Kirov, SHAD, SCHP) — the field is read but the unit's
takeoff/landing isn't a meaningful event for those (Kirov never lands,
SHAD's transport doesn't dock at airport, SCHP deploys to ground).

Hornet uses them actively because the Carrier-deck launch/recovery
cycle is the canonical takeoff/landing event.

### 3. Spawned=yes (TechnoType)

Per cheat-sheet (`0x008437d8 → 0x00714e7d`). Marks unit as spawn-child.
Combined with `TechLevel=-1`, makes Hornet unbuildable except via
SpawnManager.

### 4. No MissileSpawn=yes (key distinction from kamikaze spawn-children)

Hornet is `Spawned=yes` but **NOT** `MissileSpawn=yes`. The difference:
- *With MissileSpawn=yes* (V3ROCKET, DMISL, CMISL): unit dies on
  impact, doesn't return to dock. One-shot suicide.
- *Without MissileSpawn=yes* (HORNET, ASW): unit returns to parent
  dock after firing, refuels, ready for next sortie.

See [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
for the full SpawnManager state machine, including the dispatch logic
that distinguishes one-shot vs reusable spawns.

### 5. HornetCollision crash-transform mechanic

The Secondary `HornetCollision` weapon isn't manually fired — it's
*triggered by the aircraft-crash death code path*. When the Hornet
takes damage and the Crash sequence begins, the engine replaces the
falling Hornet with the HornetCollision projectile during the final
plummet frames.

Open question: which Ghidra function handles this? Likely
`AircraftClass::Take_Damage` or similar. Not yet traced. Open follow-up.

### 6. ImmuneToPsionics=yes

Standard TechnoType flag. Yuri can't mind-control aircraft (psi-
control range is land-bound). Same as DRON, ROBO, all aircraft.

### 7. AircraftLocomotion (5th locomotor GUID re-confirmed)

`Locomotor={4A582746-...}` — AircraftLocomotion (same as BEAG). Fixed-
wing aircraft physics: forward-velocity requirement, takeoff/landing
at airport-class buildings (here: the Carrier serves as the airport).

### 8. The `;Selectable=no` bug commentary

The verbatim Westwood comment is highly informative:
```
;Selectable=no	; SJM: this should be here but is commented out because bug prevents aircraft from landing
```

*Westwood admits a shipping bug*: setting `Selectable=no` (the
correct design choice for Hornets) **breaks the aircraft's landing
behavior**. So the Hornet remains technically selectable in shipped
YR. The bug presumably involves the player-interaction code path
being required for the SpawnManager's landing reconnection logic.

---

## TS-legacy filter

- `IgnoresFirestorm=yes` on NormalBomb — TS-legacy (Firestorm Wall
  was a TS feature, not in YR). The flag is read but the
  Firestorm-blocking system is dormant in YR. Bomb passes through
  any Firestorm trace — moot since no Firestorm exists.
- `;Dock=NAHPAD,GAHPAD` / `;Dock=GAAIRC,AMRADR` — commented historical
  Dock alternatives.
- `;Selectable=no` Westwood-bug commentary.
- No `ImmuneToVeins`, no `Subterranean`. **YR-active core mechanism.**

---

## Comparison: HORNET vs ASW vs spawn-missile children

| Field | HORNET (Carrier) | ASW (Destroyer) | V3ROCKET (V3) | CMISL (BSUB) |
|-------|------------------|------------------|---------------|--------------|
| Section | AircraftTypes | AircraftTypes | AircraftTypes | AircraftTypes |
| Spawned | yes | yes | yes | yes |
| **MissileSpawn** | **no** | **no** | **yes** | **yes** |
| TechLevel | -1 | -1 | -1 | -1 |
| Strength | 75 | ? | 50 | 50 |
| Primary | HornetBomb | OspreyAttack? | (death warhead) | (death warhead) |
| Secondary | HornetCollision (kamikaze on crash) | - | - | - |
| ElitePrimary swap | HornetBombE | OspreyAttackE? | - | - |
| Returns to dock | **yes** | **yes** | **no (suicide)** | **no (suicide)** |
| Ammo | 1 | 1 | 1 | 1 |
| Landable | yes | yes | n/a (one-shot) | n/a (one-shot) |
| AuxSound1/2 | active (takeoff/landing) | active | not set | not set |
| Locomotor | AircraftLocomotion | AircraftLocomotion | RocketLocomotion | RocketLocomotion |

**The two patterns:**
- **Return-to-dock**: HORNET, ASW. Fixed-wing aircraft with
  AircraftLocomotion, Landable=yes, AuxSound1/2 cycle. Reusable.
- **Kamikaze missile**: V3ROCKET, DMISL, CMISL. MissileSpawn=yes,
  RocketLocomotion, no Landable, one-shot suicide. Cheap to spawn,
  fast to deploy, no return overhead.

Allied factions get *return-to-dock* spawners (CARRIER + DEST). Soviet
+ Yuri get *kamikaze missile* spawners (V3, DRED, BSUB). Asymmetric
naval-air doctrine.

---

## Cross-references

- [CARRIER.md](./CARRIER.md) — parent spawner. Has `Spawns=HORNET
  SpawnsNumber=4 SpawnReloadRate>0` configuration.
- [ASW.md](./ASW.md) — pending. Destroyer's spawn-child (peer return-
  to-dock pattern).
- [BEAG.md](./BEAG.md) — peer standalone AircraftType (player-
  buildable). Same AircraftLocomotion GUID, same AircraftType-scope
  fields (Landable, etc.).
- [V3.md](../soviet/V3.md) + V3ROCKET — opposing kamikaze pattern.
- [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
  — SpawnManager state machine reference.

---

## Ghidra audit log (audit iteration 29 — 2026-05-19)

**~15 Ghidra queries** (10 string searches + 6 xref lookups + 5 grep
passes on saved TechnoTypeClass__ReadINI decompile + 1 assembly-context
batch covering AuxSound1/CrashingSound/Landable). All 3 doc-cited claims
verify exactly (Landable / AuxSound1 / Spawned), 1 doc inference upgraded
to BINARY-VERIFIED (AuxSound2 adjacent xref), 1 doc open question
resolved as DEAD INI (MovementRestrictedTo).

### Function-entry verification

| Function | Address | Status |
|----------|---------|--------|
| `AircraftTypeClass__ReadINI` | 0x0041CC20–0x0041CDA3 | [BINARY-VERIFIED] — same parser as BEAG audit 26 |
| `TechnoTypeClass__ReadINI` | (oversized) | grep-verified for AuxSound1/2/ImpactLandSound/CrashingSound/PitchSpeed |
| `RulesClass__ReadAudioVisual` | (oversized) | DUAL-READ partner for ImpactLandSound @ 0x00669965 |
| `UnitTypeClass__ReadINI` | (audit 12 known) | sole MovementRestrictedTo consumer @ 0x00747837 (UnitType-scope only) |

### Key behavioral findings — 6 NEW struct-offset bindings BINARY-VERIFIED

| INI key | Scope | Offset | Type | Parser site | Status |
|---------|-------|--------|------|-------------|--------|
| `Landable` | AircraftType | **+0xE0A** | byte (ReadBool) | 0x0041cc54 | re-confirms audit 26 |
| `AuxSound1` | TechnoType | **+0x52C** | int VocClass | 0x00712e18 | doc-cited |
| `AuxSound2` | TechnoType | **+0x530** | int VocClass | 0x00712e48 | NEW (verifies doc's adjacent-address inference) |
| `ImpactLandSound` | TechnoType | **+0x540** | int VocClass | 0x00712f38 | NEW (TechnoType side of DUAL-READ) |
| `CrashingSound` | TechnoType | **+0x544** | int VocClass | 0x00712f80 | NEW |
| `PitchSpeed` | TechnoType | **+0x3A8** | double | 0x007123da | NEW (aircraft pitch animation parameter) |

Assembly-context proofs:
- Landable: `0x0041cc54: PUSH 0x81804c` → `CALL 0x005295f0` →
  `0x0041cc67: MOV byte ptr [ESI + 0xe0a], AL` ✓
- AuxSound1: preload at `0x00712e03: MOV EDI, dword ptr [EBP + 0x52c]` →
  ReadString → store at +0x52C ✓
- CrashingSound: preload at `0x00712f6b: MOV EDI, dword ptr [EBP + 0x544]`
  + previous-result-store `0x00712f65: MOV [EBP + 0x540], EAX`
  (which is ImpactLandSound) ✓

### DUAL-READ pattern extended

**`ImpactLandSound`** confirmed in the established DUAL-READ family — parsed
in BOTH:
- `RulesClass__ReadAudioVisual` @ 0x00669965 (global default — string @ 0x0083a9c4)
- `TechnoTypeClass__ReadINI` @ 0x00712f38 (per-unit override → TechnoType+0x540)

This joins ChronoInSound, ChronoOutSound, SinkingSound, ActivateSound,
DeactivateSound — all use the same dual-read default-then-override
pattern.

`AuxSound1`/`AuxSound2`/`CrashingSound` are SINGLE-READ (TechnoType-only
— NOT parsed by RulesClass__ReadAudioVisual). The default for these is
the prior value or VocClass-name fallback only.

### Sound-cluster topology — consolidated post-audit-29

Three distinct TechnoType-level sound clusters now mapped:

| Cluster | Range | Members |
|---------|-------|---------|
| Transport cluster | +0x564..+0x568 | EnterTransportSound, LeaveTransportSound (audit 24) |
| Deploy/chrono cluster | +0x56C..+0x578 | DeploySound, UndeploySound (audit 14), ChronoInSound, ChronoOutSound (audit 17) |
| Power-state cluster | +0x5A8..+0x5AC | ActivateSound, DeactivateSound (audit 23) |
| Aircraft cluster | +0x52C..+0x548 | **AuxSound1 +0x52C (NEW)**, **AuxSound2 +0x530 (NEW)**, +0x534/+0x538/+0x53C (DEFERRED siblings), **ImpactLandSound +0x540 (NEW)**, **CrashingSound +0x544 (NEW)**, SinkingSound +0x548 (audit 27) |

The Aircraft cluster is the largest (~12 ints / 48 bytes) and contains
3 newly-pinned offsets + 1 NEW DUAL-READ confirmation in this audit.

### Discrepancies / corrections

**[INCORRECT — DEAD INI]**: `MovementRestrictedTo=Water` on Hornet is
ineffective. The key is UnitType-scope only (single xref @ 0x00747837 in
`UnitTypeClass__ReadINI`); the AircraftType parser does not read it. The
Westwood verbatim comment `"See if this will affect landing only"` is a
historical "we tried this, never confirmed it works" — and the audit
confirms it doesn't. Hornet's landing-on-water behavior must be coming
from elsewhere (likely the AircraftLocomotion + parent-Carrier's water
positioning, or the SpawnManager's parent-bound landing logic — neither
involves MovementRestrictedTo).

### Items NOT re-verified (DEFERRED with reason)

- **Sound cluster +0x534/+0x538/+0x53C unknown siblings** — int-aligned
  sound slots between AuxSound2 (+0x530) and ImpactLandSound (+0x540)
  visible via grep evidence (`param_1[0x14f] = iVar6;` write before
  ImpactLandSound preload). INI-key mappings DEFERRED.
- **HornetCollision crash-transform mechanism** — the actual code path
  that converts a crashing Hornet into the HornetCollision projectile
  was NOT decompiled this pass. Likely lives in `AircraftClass::Crash`
  or `AircraftClass::Take_Damage`. Doc's open question stands.
- **`;Selectable=no` Westwood bug** — the specific aircraft-landing
  code-path failure when Selectable=no is set; would require tracing
  AircraftClass-landing-reconnection logic. DEFERRED.
- **AuxSound1/AuxSound2 consumer** in AircraftClass-takeoff /
  AircraftClass-landing transition code. Parsed offsets are pinned;
  consumer-side dispatch DEFERRED.
- **PitchSpeed/PitchAngle consumer** in AircraftClass per-frame pitch
  interpolation code path. DEFERRED.

### Cross-references re-confirmed

- `Spawned` (audit 20 cumulative) — TechnoType+0xD54 (re-confirmed via
  string + xref at 0x00714e7d).
- `MissileSpawn` (audit 20 cumulative) — TechnoType+0xD68 (not set on
  Hornet, confirms return-to-dock pattern via flag absence).
- `ImmuneToPsionics` (audit 7 cumulative) — TechnoType+0xD35.
- `AircraftLocomotion` GUID `{4A582746-...}` (audit 20+26 cumulative) —
  same as BEAG / Osprey-class spawn-children.

### Negative claims verified

- `search_strings("HORNET")` → **0 matches** (no HORNET-specific code).
- `search_strings("Hornet")` → **0 matches** (no Hornet-specific code).
  All Hornet behavior is INI-driven (consistent with audit-20 CARRIER
  observation, audit-21 DEST observation, etc.).

### Confidence summary

- 6/6 struct-offset bindings BINARY-VERIFIED with parser-site + writeback
  evidence (3 with explicit assembly-context proof).
- 1 NEW DUAL-READ pattern confirmation (ImpactLandSound joins the
  Chrono/Sinking/Activate/Deactivate family).
- 1 DEAD INI finding flagged (MovementRestrictedTo=Water on Hornet).
- Negative claims (HORNET/Hornet → 0 matches) confirmed.
- No INCORRECT findings in the doc beyond the resolved
  MovementRestrictedTo open question (which the doc had already
  flagged as uncertain).

---

## Coverage audit

- [x] Every rulesmd key annotated (~45 keys including empty Voice*
  slots and the `;Selectable=no` bug commentary).
- [x] Every artmd key annotated (4 keys).
- [x] Both weapons documented (HornetBomb basic + HornetBombE elite,
  HornetCollision crash-transform secondary).
- [x] NormalBomb projectile documented (with IgnoresFirestorm
  TS-legacy note).
- [x] ORCAAP + ARTYHE warheads documented (ARTYHE has Deform=15%
  terrain cratering matching SCHOPWH).
- [x] All voice/sound bindings documented including active
  AuxSound1/AuxSound2 usage (canonical takeoff/landing example).
- [x] Owner: 5 Allied houses.
- [x] Spawn-child status (`Spawned=yes`, `TechLevel=-1`,
  no `MissileSpawn`).
- [x] Veterancy: reduced 2-ability VeteranAbilities/EliteAbilities;
  elite weapon swap inherits from parent CARRIER's rank.
- [x] Hardcoded behavior: **Landable AircraftType scope (NEW)**,
  **AuxSound1 TechnoType (NEW)**, return-to-dock pattern vs missile-
  spawn, HornetCollision crash-transform, Westwood bug commentary.
- [x] TS-legacy filter applied (IgnoresFirestorm dormant).
- [x] Comparison table with peer spawn-children (HORNET, ASW,
  V3ROCKET, CMISL).
- [x] At least one Ghidra search performed (Spawned, AuxSound1,
  Landable — 2 new entries + 1 re-confirmed).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("^Spawned$")` | `0x008437d8` (single match, re-confirmed from cheat-sheet) |
| `search_strings("AuxSound1")` | `0x00844240` (single match) |
| `get_xrefs_to(0x00844240)` | `0x00712e18 → TechnoTypeClass__ReadINI` |
| `search_strings("^Landable$")` | `0x0081804c` (single match) |
| `get_xrefs_to(0x0081804c)` | `0x0041cc54 → AircraftTypeClass__ReadINI` |

**New cheat-sheet entries (2):**
- `Landable` (0x0081804c → 0x0041cc54) **AircraftType** — enables
  landing-cycle behavior. **3rd AircraftType-scope field**.
- `AuxSound1` (0x00844240 → 0x00712e18) TechnoType — takeoff event
  SFX. Sibling `AuxSound2` is at adjacent address (high-confidence
  inference, not separately verified).

**Re-confirmed:**
- `Spawned` (0x008437d8 → 0x00714e7d) TechnoType (per MIND).
- `Locomotor=AircraftLocomotion` GUID (per BEAG).

**Open questions:**
- `MovementRestrictedTo=Water` on Hornet — the field is normally
  UnitType-scope. Does it work on AircraftType too via TechnoType
  inheritance, or is it read but ignored? Open follow-up.
- The crash-transform mechanism that converts a falling Hornet into
  the HornetCollision projectile — which Ghidra function handles
  this? Open follow-up trace.
- The `;Selectable=no` Westwood bug — what specific code path breaks
  when aircraft are unselectable? Open follow-up.
