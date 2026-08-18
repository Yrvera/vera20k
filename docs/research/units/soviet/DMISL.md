---
name: dmisl-doc
description: DMISL — Dreadnought's spawn-child kamikaze missile. Spawned=yes +
  MissileSpawn=yes. RocketLocomotion. NO weapons (Rules-global DMislWarhead /
  DMislEliteWarhead lookup). FlyBack=true (NEW field vs V3ROCKET). Elite warhead
  DMISLEWH has MININUKE AnimList + CellSpread=3 (mini-nuke aesthetic).
metadata:
  type: project
---

# DMISL — Dread Missile (kamikaze spawn-missile)

**INI ID:** `DMISL`
**Display:** "Dread Missile" (`UIName=Name:DMISL`)
**Section:** `[AircraftTypes]`
**Owner side:** Soviet (Russians, Confederation, Africans, Arabs) — but
**not directly buildable** (`TechLevel=-1`); only exists as a spawn-child
of the [DRED](../soviet/DRED.md) Dreadnought.
**Role:** Dreadnought's spawn-child kamikaze missile. Naval long-range
ballistic missile. Near-mirror of V3ROCKET architecture. Pairs with
[V3ROCKET](./V3ROCKET.md) and [CMISL](../yuri/CMISL.md) as the three
Soviet+Yuri kamikaze missile spawn-children. **Key distinguishing
feature**: separate elite-warhead (DMislEliteWarhead) for elite-rank
Dreadnought salvos. **NEW: `FlyBack=true` field on DMISL** not on
V3ROCKET — behavioral difference open for investigation.

---

## Note on Ghidra unavailability

The Ghidra MCP server remains offline this iteration. All field-scope
claims cross-reference *prior* cheat-sheet entries from earlier docs.

---

## Rulesmd verbatim

```ini
[DMISL]
UIName=Name:DMISL
Name=Dread Missile
FireAngle=1
Strength=50
Category=AirPower
Armor=special_2
Spawned=yes
MissileSpawn=yes
TechLevel=-1
Sight=0
RadarInvisible=no
Landable=yes
MoveToShroud=yes
Ammo=1 ;Aircraft are hard wired to require ammo
Speed=18;20
Owner=Russians,Confederation,Africans,Arabs
Cost=50
Points=20
ROT=4
Crewed=no
Explodes=no
GuardRange=30
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=2
Locomotor={B7B49766-E576-11d3-9BD9-00104B972FE8}
MovementZone=Fly
ThreatPosed=10	; This value MUST be 0 for all building addons
DamageParticleSystems=SmallGreySSys	; Sparks don't work well here.  SJM
AuxSound1=DreadnoughtAttack ;Taking off
;AuxSound2=DROPDWN1 ;Landing
ImmuneToPsionics=yes
;VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
;EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
NoShadow=yes
Selectable=no
Trainable=no
FlyBack=true
DontScore=yes
```

### Key-by-key annotation — diff vs V3ROCKET

Most fields are *bit-for-bit identical* to V3ROCKET. This section
covers the differences:

**Diff vs V3ROCKET:**
| Field | DMISL | V3ROCKET |
|-------|-------|----------|
| `UIName` | `Name:DMISL` | `Name:V3ROCKET` |
| `Name` | "Dread Missile" | "V3 Rocket" |
| `Sight` | **0** | 1 |
| `Speed` | **18** | 15 |
| `ROT` | **4** | 3 |
| `AuxSound1` | DreadnoughtAttack | V3Attack |
| `FlyBack` | **`true`** | (not set) |

**Sight=0** (vs V3ROCKET=1) — *zero vision*. Even more aggressive cut
than V3ROCKET. The missile relies entirely on the parent's targeting
to point it at the right cell.

**Speed=18** (`;20` historical) — *faster than V3ROCKET (15)*. The
Dreadnought's missile flies 20% faster, matching the naval-platform's
need to deliver damage quickly across distance.

**ROT=4** (vs V3ROCKET=3) — slightly better tracking. Naval missiles
may need to adjust mid-flight against moving naval targets.

**`FlyBack=true`** — **the unique field not on V3ROCKET**. *Open
question*: this field is normally associated with patrol-and-return
aircraft behavior. On a MissileSpawn=yes kamikaze, FlyBack semantics
are unclear:
- Possibility 1: FlyBack triggers a *fly-toward-target* arc-completion
  state (the missile completes its arc back down to target after
  apex). Without it, the missile might launch straight up forever.
- Possibility 2: FlyBack is *vestigial* — copy-pasted from a non-
  kamikaze design template; engine ignores it for MissileSpawn=yes.
- Possibility 3: Differentiates DMISL's salvo behavior — Dreadnought
  fires 2 missiles per salvo (`Burst=2` on the parent's DredLauncher),
  and FlyBack might control inter-missile spacing during the launch.

V3ROCKET doesn't have FlyBack=true, and V3 fires only 1 missile per
salvo. **The FlyBack=true on DMISL but not V3ROCKET likely correlates
with the salvo-fire-2 pattern**. Open follow-up — requires Ghidra
trace.

**`AuxSound1=DreadnoughtAttack`** — uses the parent's attack sound
name. Same audio sample as the Dreadnought's manual fire SFX. Two
DMISLs launching simultaneously play 2× DreadnoughtAttack (limited
to Limit=2 concurrent per the sound block).

### Otherwise identical to V3ROCKET

The rest of the fields (FireAngle=1, Strength=50, Armor=special_2,
Spawned+MissileSpawn, RocketLocomotion, no Voice slots, Selectable=no,
Trainable=no, etc.) all match V3ROCKET. See
[V3ROCKET.md](./V3ROCKET.md) for full key-by-key annotation of the
shared fields.

---

## Artmd verbatim

```ini
[DMISL]
SpawnDelay=2;1
Voxel=yes
Remapable=no
CanBeHidden=no
```

### Key-by-key annotation

- `SpawnDelay=2;1` — 2-frame delay between spawn and active state
  (`;1` historical commented). **Identical to V3ROCKET**. With
  Burst=2 on the Dreadnought, the SpawnDelay staggers the two
  missiles by 2 frames each — visual separation during launch.
- `Voxel=yes` — rendered from `dmisl.vxl` + `.hva`.
- `Remapable=no` — gray/black missile, no house tint.
- `CanBeHidden=no` — always rendered (not occluded by terrain/buildings).

**No `Trailer=`** (V3ROCKET has commented `;Trailer=DURASMOKE`; DMISL
doesn't even have the commented line). DMISL's voxel includes the
visible exhaust as part of the model.

---

## Weapons

**DMISL has no weapons defined**. Damage on impact comes from the
*Rules-global warhead lookup* — but DMISL has **TWO** Rules-globals
(unlike V3ROCKET which only has one):

From rulesmd line 820-822:
```
DMislWarhead=DMISLWH       ; this is the warhead on a DredMissile
DMislEliteWarhead=DMISLEWH ; this is the warhead on a DredMissile when the launcher is elite
```

The Dreadnought has both a basic and elite warhead variant. When the
parent DRED reaches Elite rank, its salvos switch to `DMISLEWH`
(mini-nuke aesthetic — see below).

**Cheat-sheet refs**: `DMislWarhead` and `DMislEliteWarhead` are
Rules-global fields. Per existing notes from DRED doc:
- `DMislWarhead` (0x0083b1a8 → 0x0066c3db → Rules+0xfb4)
- `DMislEliteWarhead` (0x0083b184 → 0x0066c458 → Rules+0xfbc)

### Basic warhead — `[DMISLWH]`

```ini
[DMISLWH]
CellSpread=1.5
PercentAtMax=.25
Wall=yes
Wood=yes
Verses=100%,90%,80%,100%,80%,80%,85%,65%,28%,80%,0%
Conventional=yes
Rocker=no
InfDeath=2
AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070
Deform=10%
DeformThreshhold=300
Tiberium=yes
Sparky=no
Bright=yes
ProneDamage=70%     ; Presumes air burst
```

- `CellSpread=1.5` — **larger AoE than V3WH (1.0)**. Dreadnought
  missiles hit a bigger area.
- `PercentAtMax=.25` — 25% damage at edge (same falloff as V3WH).
- `Verses=100%,90%,80%,100%,80%,80%,85%,65%,28%,80%,0%`:
  | Armor | Mult | vs V3WH | vs ORCAAP |
  |-------|------|---------|-----------|
  | none | 100% | same | same |
  | flak | 90% | same | -10% |
  | plate | 80% | same | -20% |
  | light | **100%** | +10% (was 90) | same |
  | medium | 80% | +10% (was 70) | -20% |
  | heavy | 80% | +10% (was 70) | -20% |
  | wood | 85% | -15% (was 100) | -15% |
  | steel | 65% | -35% (was 100) | -35% |
  | concrete | **28%** | -22% (was 50) | -47% |
  | special_1 | 80% | same | -20% |
  | **special_2** | **0%** | same | same |
  - **Stronger vs medium/heavy tanks** (80% vs V3WH's 70%) — anti-armor
    optimization. Naval missiles need to crack tanks.
  - **Weaker vs steel/concrete** (65%/28% vs V3WH's 100%/50%) —
    *poor anti-structure*. Dreadnought missile is anti-armor, not
    siege.
  - 0% vs special_2 — same self-protection as V3WH (missiles don't
    damage other missiles).
- `Conventional=yes`, `Rocker=no` (V3WH has Rocker=yes; DMISLWH
  doesn't rock vehicles).
- `InfDeath=2` — same.
- `Deform=10%, DeformThreshhold=300` — same as V3WH.
- `ProneDamage=70%` — same "Presumes air burst" pattern.

### Elite warhead — `[DMISLEWH]`

```ini
[DMISLEWH]
CellSpread=3
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=100%,90%,80%,100%,80%,80%,85%,65%,28%,80%,0%
Conventional=yes
Rocker=no
InfDeath=2
;AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070
AnimList=MININUKE
Tiberium=yes
Sparky=no
Bright=yes
ProneDamage=70%     ; Presumes air burst
```

**Two upgrades vs basic DMISLWH:**
1. `CellSpread=3` (vs 1.5) — **2× AoE radius**. Elite Dreadnought
   missiles affect 4× the area (πr² area).
2. `PercentAtMax=.5` (vs 0.25) — **2× edge damage**. Combined with
   the larger spread, edge cells take ~4× the damage they would from
   basic DMISLWH.

**`AnimList=MININUKE`** — *single anim slot* (vs basic's 8-anim
random pool). The MININUKE anim is the *mini-nuclear-explosion*
sprite — a small mushroom cloud effect. The verbatim
`;AnimList=XGRYSML1...` historical comment shows the original 8-anim
list was replaced.

**Visual identity**: Elite Dreadnought missiles render as mini-nuke
mushroom explosions on impact. Distinctive elite-rank visual cue —
players see the mushroom and know the Dreadnought is elite.

**Verses identical to basic**: armor multipliers don't change at
elite. The elite upgrade is *pure AoE expansion + edge-damage*, not
damage-vs-armor profile.

### Note on CMISLWH (next in rulesmd)

The CMISLWH (Boomer Sub cruise missile warhead) immediately follows
DMISLEWH in rulesmd and has the same structure. CMISL uses CMISLWH
for its damage — same Rules-global resolution pattern. See CMISL
iteration for that warhead's full block.

---

## Voices / sounds

```ini
[DreadnoughtAttack]
Sounds= vdreatta vdreattb
Control= random interrupt
FShift= -10 10
Limit=2
Volume=60
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| (all Voice* / DieSound empty) | n/a | No player-interaction voices (Selectable=no) |
| `AuxSound1=DreadnoughtAttack` | `[DreadnoughtAttack]` | **Plays when DMISL launches from Dreadnought deck** (2-sample `vdreatta/b`, Limit=2 concurrent) |
| `;AuxSound2=DROPDWN1` | (commented) | Landing event — moot (MissileSpawn=yes, no landing) |

**Audio sharing with parent**: `DreadnoughtAttack` is the parent
DRED's primary fire SFX. When DMISL launches, it plays the same
sample as the Dreadnought firing — audio cohesion across spawn parent
and child.

**Limit=2 concurrent**: with Burst=2 on the parent firing 2 DMISLs
simultaneously, both AuxSound1 plays of DreadnoughtAttack hit the
Limit=2 cap — only 2 audible per moment. Prevents flood when multiple
Dreadnoughts salvo.

---

## Hardcoded behavior

### 1. MissileSpawn=yes kamikaze (shared with V3ROCKET/CMISL)

Same architecture as V3ROCKET. See [V3ROCKET.md](./V3ROCKET.md#1-missilespawnyes-kamikaze-pattern)
for the mechanism.

### 2. RocketLocomotion (shared 6th locomotor type)

Same `{B7B49766-...}` GUID as V3ROCKET and CMISL.

### 3. Rules-global warhead resolution — TWO variants

DMISL is the **first documented spawn-child with both basic and elite
Rules-global warheads**:
- `DMislWarhead=DMISLWH` — used when parent DRED is rookie/veteran.
- `DMislEliteWarhead=DMISLEWH` — used when parent DRED is Elite.

The engine selects which warhead based on the *parent's veterancy
rank* at the moment of missile launch. The DMISL itself never gains
veterancy (Trainable=no) — the parent's rank dictates the missile's
power.

**Compare V3ROCKET**: V3 also has a `V3EliteWarhead` Rules-global
(noted in V3ROCKET doc as open follow-up). Both V3 and DRED have the
elite-warhead-upgrade pattern.

**Compare CMISL**: not yet verified, but likely follows the same
pattern with `CMislWarhead` Rules-global.

### 4. FlyBack=true — open question

`FlyBack=true` on DMISL but **NOT on V3ROCKET**. Possible reasons:
- Salvo-fire behavior (DRED Burst=2, V3 Burst=1).
- Naval target tracking (DMISL hits moving targets at sea).
- Trajectory arc-completion logic.
- Vestigial / engine-ignored for MissileSpawn=yes.

**Open follow-up**: requires Ghidra trace once MCP is back online.
Searching for `FlyBack` field in TechnoTypeClass__ReadINI would
reveal the scope; tracing the field-consumer would reveal the
behavior.

### 5. Mini-nuke aesthetic via AnimList=MININUKE

The elite Dreadnought missile's `[DMISLEWH]` uses `AnimList=MININUKE`
— a single anim slot showing a mini-nuke mushroom cloud. The
verbatim `;AnimList=XGRYSML1...` comment shows the original generic
8-anim pool was replaced with the dedicated mushroom effect.

**Cross-references**:
- Nuke superweapon ([NUKE_SUPERWEAPON_GHIDRA_REPORT.md](../../NUKE_SUPERWEAPON_GHIDRA_REPORT.md))
  uses a separate full-scale nuke explosion anim.
- Elite Dreadnought's mini-nuke is a visual *reference* to the full
  nuke superweapon, but mechanically much weaker (CellSpread=3, not a
  superweapon).

### 6. Selectable=no + no Voice slots + Trainable=no

Same triple as V3ROCKET. The "transient projectile" pattern.

---

## TS-legacy filter

- `;AuxSound2=DROPDWN1` — commented landing SFX.
- `;VeteranAbilities` / `;EliteAbilities` — commented.
- `SpawnDelay=2;1` historical value.
- `Speed=18;20` historical commented value (was 20, lowered to 18).
- `;AnimList=XGRYSML1...` historical commented (replaced with MININUKE
  on elite warhead).
- No `ImmuneToVeins`, no `Subterranean`. YR-active mechanism.

---

## Comparison: the kamikaze missile trio

| Field | V3ROCKET (V3) | DMISL (DRED) | CMISL (BSUB) |
|-------|---------------|---------------|--------------|
| Display | "V3 Rocket" | "Dread Missile" | "Cruise Missile" |
| Sight | 1 | **0** | 0 (per BSUB doc) |
| Speed | 15 | **18** | 20 (per BSUB doc) |
| ROT | 3 | **4** | 4 (per BSUB doc) |
| **FlyBack** | (not set) | **true** | true (per BSUB doc) |
| AuxSound1 | V3Attack | DreadnoughtAttack | BoomerAttack1 (per BSUB doc) |
| Rules-global warhead | V3Warhead | DMislWarhead | (CMislWarhead?) |
| Rules-global elite warhead | V3EliteWarhead | DMislEliteWarhead | unknown |
| Parent's Burst | 1 | 2 | 2 |
| Image | v3rocket.vxl | dmisl.vxl | bsubmisl.vxl |

**Key trio observations:**
1. **V3ROCKET is the outlier** — has `FlyBack` unset, while DMISL and
   CMISL both have it.
2. **Parent's Burst correlates with FlyBack=true** — both Burst=2
   parents (DRED, BSUB) have FlyBack=true children; the Burst=1
   parent (V3) has FlyBack unset.
3. **Speed escalates by parent**: V3 (15) < DRED (18) < BSUB (20).
   Naval missiles get progressively faster.
4. **ROT escalates by parent**: V3 (3) < DRED (4) = BSUB (4). Naval
   missiles get better mid-flight tracking.

**Strong hypothesis**: FlyBack=true is required for Burst>1 missile
spawns to *spread out their trajectories* (Westwood's mechanism for
preventing all burst missiles from converging on the exact same cell
in a flawed straight line). The Burst=1 case doesn't need it.

---

## Cross-references

- [DRED.md](./DRED.md) — parent Dreadnought. SpawnsNumber=2,
  Burst=2.
- [V3ROCKET.md](./V3ROCKET.md) — V3 Launcher's spawn-child (peer
  kamikaze, no FlyBack).
- [CMISL.md](../yuri/CMISL.md) — Boomer Sub's spawn-child (pending
  full doc; BSUB doc has overview).
- [BSUB.md](../yuri/BSUB.md) — Boomer Sub parent.
- [HORNET.md](../allied/HORNET.md) — counterpart return-to-dock
  pattern.
- [ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — RocketLocomotion state machine.
- [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
  — SpawnManager dispatcher.

---

## Coverage audit

- [x] Every rulesmd key annotated (~40 keys, diff vs V3ROCKET).
- [x] Every artmd key annotated (5 keys).
- [x] **No weapons** documented (Rules-global DMislWarhead +
  DMislEliteWarhead lookup explained).
- [x] Both warheads documented (DMISLWH basic + DMISLEWH elite mini-
  nuke).
- [x] All voice/sound bindings documented (active AuxSound1=
  DreadnoughtAttack shared with parent fire SFX).
- [x] Spawn-child status (Spawned=yes + MissileSpawn=yes,
  TechLevel=-1).
- [x] Veterancy: disabled on child (parent's rank selects warhead).
- [x] Hardcoded behavior: MissileSpawn kamikaze, RocketLocomotion,
  TWO Rules-global warheads (basic + elite), FlyBack=true open
  question, mini-nuke elite visual identity.
- [x] TS-legacy filter applied.
- [x] Comparison table closes the kamikaze missile trio (V3ROCKET,
  DMISL, CMISL).
- [ ] **No Ghidra verification this iteration** (MCP server offline).

**Ghidra status**: MCP server still disconnected. No new cheat-sheet
entries. All field-scope claims cross-reference prior entries.

**Re-confirmed from prior cheat-sheet:**
- All shared fields with V3ROCKET (Spawned, MissileSpawn, etc.).
- DMislWarhead / DMislEliteWarhead Rules-global (per DRED doc).

**Open questions:**
- `FlyBack=true` field scope and exact behavior. The correlation
  with Burst>1 parents is a strong hypothesis. Open Ghidra trace.
- The Burst-spread theory — does FlyBack literally cause trajectory
  divergence between salvo missiles? Or some other mechanism?
- CMISL warhead resolution — likely `CMislWarhead` Rules-global, but
  not yet verified.
- V3EliteWarhead — V3 also has elite-warhead Rules-global per
  V3ROCKET doc; the pattern is consistent across kamikaze spawners.
