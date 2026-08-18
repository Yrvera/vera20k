---
name: cmisl-doc
description: CMISL — Boomer Sub's spawn-child cruise missile. Closes kamikaze
  missile trio (V3ROCKET + DMISL + CMISL). Owner=YuriCountry; near-identical to
  DMISL with Image=BSUBMISL art reuse and BoomerAttack1 launch SFX. Speed=20
  (fastest trio). CSF UIName=Name:DMISL reused (shows as "Dread Missile" in UI).
  CMISLWH/CMISLEWH warheads byte-identical to DMISLWH/DMISLEWH.
metadata:
  type: project
---

# CMISL — Cruise Missile (Boomer Sub spawn-child)

**INI ID:** `CMISL`
**Display:** **CSF UIName actively set to `Name:DMISL`** — shows as "Dread
Missile" in shipped YR tooltips. The commented `;UIName=Name:CMISL` shows
Westwood considered a distinct label but reused the Dreadnought's. Internal
`Name=Cruise Missile` is the rules-side identifier.
**Section:** `[AircraftTypes]`
**Owner side:** **Yuri** (`Owner=YuriCountry`) — *first kamikaze missile
spawn-child documented with Yuri ownership*. V3ROCKET and DMISL are Soviet;
CMISL is Yuri.
**Role:** Boomer Sub's spawn-child kamikaze cruise missile. Range=20 anti-
land strike payload of the BSUB submarine. **Closes the kamikaze missile
trio** (V3ROCKET ✓ + DMISL ✓ + CMISL ✓).

---

## Note on Ghidra unavailability

Ghidra MCP server remains offline. All field-scope claims cross-reference
prior cheat-sheet entries. No new ReadINI scope verifications this
iteration.

---

## Rulesmd verbatim

```ini
[CMISL]
;UIName=Name:CMISL
UIName=Name:DMISL
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
RadarInvisible=no
Landable=yes
MoveToShroud=yes
Ammo=1 ;Aircraft are hard wired to require ammo
Speed=20
Owner=YuriCountry
Cost=50
Points=18;20
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
AuxSound1=BoomerAttack1
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

### Key-by-key annotation — diff vs DMISL/V3ROCKET

Most fields match DMISL exactly. This section covers CMISL-distinctive
differences.

**Diff vs DMISL:**
| Field | CMISL | DMISL |
|-------|-------|-------|
| `UIName` (active) | **`Name:DMISL`** (shared!) | `Name:DMISL` |
| `Name` | "Cruise Missile" | "Dread Missile" |
| `Image=` | **`BSUBMISL`** (art redirect) | (none — uses dmisl.vxl) |
| `Owner` | **`YuriCountry`** | Russians,Confederation,Africans,Arabs |
| `Speed` | **20** | 18 |
| `Points` | **18** (`;20` historical) | 20 |
| `AuxSound1` | BoomerAttack1 | DreadnoughtAttack |

**CSF label reuse**:
- `;UIName=Name:CMISL` — commented; would have been a distinct label.
- `UIName=Name:DMISL` — active; **CMISL displays as "Dread Missile"
  in tooltips and selection labels**. Cross-faction label reuse —
  Yuri's cruise missile shares the Dreadnought's CSF entry. Possibly
  to avoid creating a separate CSF lookup string for a similar
  weapon. Players probably never notice (spawn-children are
  Selectable=no — no tooltip exposure).

**`Image=BSUBMISL` art redirect**:
- The CMISL's voxel rendering uses the `[BSUBMISL]` artmd block
  (`bsubmisl.vxl` + `bsubmisl.hva`). The naming `BSUB-MISL` suggests
  this was originally meant to be the Boomer Sub's dedicated missile
  art. CMISL repurposes it via `Image=BSUBMISL`.
- The BSUB rulesmd doc noted this earlier: BSUB has art at line 1118
  `[BSUB]` and its missiles use `[BSUBMISL]` block at line 739 of
  artmd. Both BSUB-spawned missiles use this art (consistent with
  CMISL being BSUB's child).

**Yuri ownership**:
- `Owner=YuriCountry` — first kamikaze missile spawn-child with Yuri
  ownership. V3ROCKET (V3) and DMISL (DRED) are Soviet sub-factions
  (Russians, Confederation, Africans, Arabs). CMISL inherits BSUB's
  monolithic YuriCountry ownership.
- Crate spawning, AI dispatch, etc. all apply Yuri-faction filters.

**Speed=20** (fastest of trio):
| Unit | Speed |
|------|-------|
| V3ROCKET (V3) | 15 |
| DMISL (DRED) | 18 |
| **CMISL (BSUB)** | **20** |

The Yuri cruise missile is the fastest of the three kamikaze spawn-
children. Naval-launched, Range=20 — speed matters more here because
the long-range delivery exposes the parent BSUB for longer.

**Points=18** (`;20` historical):
- Slightly lower than V3ROCKET/DMISL (both 20 points).
- The `;20` commented value shows it was once 20; Westwood lowered to
  18. Marginal balance adjustment.

**`AuxSound1=BoomerAttack1`**: launches with the parent BSUB's
primary missile-launch SFX (`vbooat1a`). See BSUB voice block for
the audio sample. **Audio cohesion pattern continues**: each kamikaze
missile's launch SFX matches its parent's attack SFX:
- V3ROCKET → V3Attack
- DMISL → DreadnoughtAttack
- CMISL → BoomerAttack1

### Otherwise identical to DMISL

The rest of CMISL (FireAngle=1, Strength=50, Armor=special_2,
Spawned+MissileSpawn, RocketLocomotion, all Voice slots empty,
Selectable=no, Trainable=no, FlyBack=true, DontScore=yes,
Explodes=no, NoShadow=yes, ImmuneToPsionics=yes, all commented
Vet/Elite ability lines) matches DMISL exactly. See [DMISL.md](../soviet/DMISL.md)
for full annotation.

---

## Artmd verbatim — via `[BSUBMISL]` redirect

```ini
[BSUBMISL]
SpawnDelay=2;1
Voxel=yes
Remapable=no
CanBeHidden=no
```

### Key-by-key annotation

- `SpawnDelay=2;1` — 2-frame delay (matches V3ROCKET and DMISL). With
  Burst=2 on the parent BSUB CruiseLauncher, the 2 missiles spawn
  staggered by 2 frames.
- `Voxel=yes` — rendered from `bsubmisl.vxl` + `bsubmisl.hva`.
- `Remapable=no` — gray/black, no house tint.
- `CanBeHidden=no` — always rendered on top.

**No `Trailer=`, no `Cameo=`, no FLH** — same minimal pattern as
V3ROCKET and DMISL art blocks.

**Voxel asset sharing**: CMISL uses `bsubmisl.vxl` via the
`Image=BSUBMISL` redirect. The voxel is dedicated to BSUB-spawned
missiles. *Open*: does BSUB itself ever directly reference BSUBMISL
art for any internal animation (e.g. launch tube visuals)? Not yet
verified.

---

## Weapons

**CMISL has no weapons defined**. Damage on impact comes from
*Rules-global warhead lookup*:

From rulesmd lines 823-824:
```
CMislWarhead=CMISLWH       ; this is the warhead on a DredMissile
CMislEliteWarhead=CMISLEWH ; this is the warhead on a DredMissile when the launcher is elite
```

**Verbatim Westwood typo**: the rules comments say *"this is the
warhead on a DredMissile"* — but CMISL is the Yuri Cruise Missile,
not the Dreadnought's. **Copy-paste typo**: Westwood copied the
DMislWarhead comment text and forgot to change "DredMissile" to
"CruiseMissile". *Mechanically harmless* — the field still resolves
correctly to CMISLWH.

**Rules-global field names**:
- `CMislWarhead` — basic warhead for CMISL.
- `CMislEliteWarhead` — elite warhead when parent BSUB is at Elite
  rank.

Same Rules-global lookup pattern as `DMislWarhead`/`DMislEliteWarhead`
and `V3Warhead`/`V3EliteWarhead`. Engine selects based on parent
veterancy.

### Warheads — CMISLWH and CMISLEWH

**CMISLWH is byte-for-byte identical to DMISLWH**:

```ini
[CMISLWH]
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

**Identical fields to DMISLWH** (see [DMISL.md](../soviet/DMISL.md#basic-warhead--dmislwh)):
- `CellSpread=1.5`, `PercentAtMax=.25`
- Same Verses array.
- Same anim pool.
- Same Deform mechanics.

The byte-identical warhead means **CMISL and DMISL deal mechanically
identical damage** despite belonging to different factions. Westwood
either:
1. Copy-pasted the DMISLWH block as the starting point for CMISLWH
   and forgot to balance it independently.
2. Deliberately balanced CMISL to match DMISL's hit profile.

Either way: **no faction-asymmetric damage between Yuri and Soviet
kamikaze missiles**.

**CMISLEWH (elite)** is also byte-identical to DMISLEWH:

```ini
[CMISLEWH]
CellSpread=3
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=100%,90%,80%,100%,80%,80%,85%,65%,28%,80%,0%
Conventional=yes
Rocker=no
InfDeath=2
AnimList=MININUKE
Tiberium=yes
Sparky=no
Bright=yes
ProneDamage=70%
```

**Same mini-nuke visual (`AnimList=MININUKE`)** as elite Dreadnought.
Elite Yuri Boomer Sub cruise missiles also paint mushroom-cloud
visuals on impact. **Cross-faction visual reuse** — the mini-nuke
isn't Soviet-specific.

---

## Voices / sounds

CMISL uses the same audio mechanism as V3ROCKET/DMISL:
- All Voice* / DieSound empty.
- `AuxSound1=BoomerAttack1` → `[BoomerAttack1]` block (see BSUB doc).
- `;AuxSound2=DROPDWN1` commented — no landing event.

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| (all Voice* empty) | n/a | No player-interaction voices (Selectable=no) |
| `AuxSound1=BoomerAttack1` | `[BoomerAttack1]` (BSUB doc) | **Plays when CMISL launches from BSUB** — shared with parent's primary fire SFX (`vbooat1a`, Limit=2 concurrent) |
| `;AuxSound2=DROPDWN1` | (commented) | Landing — moot |
| `DieSound=` (not set) | n/a | No dedicated death SFX (handled by Explosion= pool) |

**Audio cohesion pattern complete**: the three kamikaze spawners all
have their child missile's AuxSound1 mirror the parent's fire SFX.
Visual: 4 missile units (V3 + DRED + BSUB + child each) — only 3
audio identities (parent's primary fire reused for launch). Compact
audio design.

---

## Hardcoded behavior

### 1. MissileSpawn=yes kamikaze (shared trio architecture)

Same architecture as V3ROCKET and DMISL. See [V3ROCKET.md](../soviet/V3ROCKET.md#1-missilespawnyes-kamikaze-pattern)
for the mechanism details.

### 2. RocketLocomotion GUID

Same `{B7B49766-...}` as V3ROCKET/DMISL. 6th distinct locomotor type
(per V3ROCKET doc).

### 3. Rules-global CMislWarhead resolution

Same two-warhead pattern as DMISL:
- `CMislWarhead=CMISLWH` (basic).
- `CMislEliteWarhead=CMISLEWH` (elite, mini-nuke visual).

Engine selects based on parent BSUB's veterancy at launch time. The
elite-warhead promotion mechanism is uniform across V3/DRED/BSUB.

**Open**: are `CMislWarhead` and `CMislEliteWarhead` Rules-global
fields stored at distinct offsets in the Rules class, or do they
share storage with `DMislWarhead`/`DMislEliteWarhead`? Likely
distinct fields (engine needs separate lookup keys), but the
byte-identical warhead values suggest Westwood might have considered
sharing.

### 4. FlyBack=true (matches DMISL, not V3ROCKET)

Confirms the **FlyBack ↔ Burst>1 correlation hypothesis** from DMISL
iteration:
- V3 fires Burst=1 → V3ROCKET has no FlyBack.
- DRED fires Burst=2 → DMISL has FlyBack=true.
- BSUB CruiseLauncher fires Burst=2 → **CMISL has FlyBack=true**.

The correlation holds across all three parent-child pairs. **Strong
evidence that FlyBack=true enables Burst-divergence trajectory
spreading** (preventing salvo missiles from converging on the same
cell). Still open Ghidra verification.

### 5. CSF label sharing (UIName=Name:DMISL on CMISL)

Yuri's cruise missile labeled as "Dread Missile" in tooltips. Possibly
to:
- Avoid creating a separate CSF entry.
- Maintain consistent labeling across all kamikaze missile children.
- Mask the spawn-child from player attention (tooltips never show
  since Selectable=no anyway).

Most likely: dev-time shortcut that became permanent.

### 6. CMISL/DMISL byte-identical warhead = no faction asymmetry on damage

Yuri and Soviet kamikaze missiles deal identical damage profiles
across all 11 armor types. **The faction asymmetry is on the parent
unit** (BSUB anti-land Range=20 + cloak + Boomer-only naval; vs DRED
naval-bombardment + heavy HP + Soviet-only). The spawn-children are
mechanically uniform.

---

## TS-legacy filter

- `;UIName=Name:CMISL` commented — historical label.
- `;AuxSound2=DROPDWN1` commented — landing SFX.
- `;VeteranAbilities` / `;EliteAbilities` commented.
- `SpawnDelay=2;1` historical value (matches V3ROCKET, DMISL).
- `Points=18;20` historical.
- The CMislWarhead comment "this is the warhead on a DredMissile" —
  Westwood typo (copied from DMISL doc, didn't update text).
- `;AnimList=XGRYSML1...` historical commented on CMISLEWH (replaced
  with MININUKE).
- No `ImmuneToVeins`, no `Subterranean`. YR-active mechanism.

---

## Comparison: the kamikaze missile trio (CLOSED)

| Field | V3ROCKET (V3) | DMISL (DRED) | CMISL (BSUB) |
|-------|---------------|--------------|---------------|
| Display CSF | `Name:V3ROCKET` | `Name:DMISL` | **`Name:DMISL` (shared)** |
| Internal Name | V3 Rocket | Dread Missile | Cruise Missile |
| Owner | Soviet (4) | Soviet (4) | **YuriCountry** |
| Sight | 1 | 0 | 0 |
| Speed | 15 | 18 | **20** (fastest) |
| ROT | 3 | 4 | 4 |
| **FlyBack** | (not set) | **true** | **true** |
| **Image=** | (none, v3rocket.vxl) | (none, dmisl.vxl) | **BSUBMISL (redirect)** |
| AuxSound1 | V3Attack | DreadnoughtAttack | BoomerAttack1 |
| Rules-global warhead | V3Warhead | DMislWarhead | CMislWarhead |
| Rules-global elite warhead | V3EliteWarhead | DMislEliteWarhead | CMislEliteWarhead |
| Parent's Burst | 1 | 2 | 2 |
| Points | 20 | 20 | **18** |
| Warhead identical to DMISL? | no (V3WH unique) | reference | **yes (CMISLWH = DMISLWH)** |
| Elite warhead AnimList | (V3EWH) | MININUKE | MININUKE |

**Trio analysis:**
- **V3ROCKET is the outlier** in 3 dimensions: no FlyBack, Sight=1
  (vs 0), Speed=15 (slowest).
- **CMISL is the outlier** in 3 dimensions: Yuri owner (vs Soviet),
  Image redirect (uses BSUBMISL voxel), shares CSF label with DMISL.
- **DMISL and CMISL share warhead identity** — same damage profile
  byte-for-byte. The faction split (Soviet vs Yuri) doesn't change
  missile damage.
- **All three share**: RocketLocomotion GUID, MissileSpawn=yes,
  Trainable=no, Selectable=no, NoShadow=yes, DontScore=yes, Explodes=no,
  ImmuneToPsionics=yes, Locomotor, Armor=special_2, etc.

**The "transient projectile" template**: ~30 shared fields across
the trio define a uniform "this is a kamikaze missile" archetype.
Westwood used template-driven design — V3ROCKET established the
pattern, DMISL refined it (added FlyBack + elite warhead), CMISL
inherited from DMISL with faction-specific cosmetic tweaks.

**Allied counterpart**: HORNET/ASW (return-to-dock pattern, see
[HORNET.md](../allied/HORNET.md)). The two paradigms:
- **Kamikaze missile trio** (Soviet + Yuri): MissileSpawn=yes,
  one-shot, RocketLocomotion, no Voice slots.
- **Return-to-dock pair** (Allied): MissileSpawn=no, reusable,
  AircraftLocomotion, AuxSound1+AuxSound2 active.

**Asymmetric Soviet/Yuri kamikaze vs Allied reusable air-power
doctrine** — confirmed across all 5 spawn-children documented.

---

## Cross-references

- [BSUB.md](./BSUB.md) — parent Boomer Sub. CruiseLauncher Burst=2,
  Range=20, MinimumRange=8, Spawns=CMISL SpawnsNumber=2
  SpawnReloadRate=0.
- [V3ROCKET.md](../soviet/V3ROCKET.md) — trio peer (Soviet V3 child).
- [DMISL.md](../soviet/DMISL.md) — trio peer (Soviet Dreadnought
  child); CMISL is a near-clone of DMISL.
- [HORNET.md](../allied/HORNET.md) + [ASW.md](../allied/ASW.md) —
  opposing return-to-dock paradigm.
- [ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — RocketLocomotion state machine.
- [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
  — SpawnManager dispatcher.

---

## Coverage audit

- [x] Every rulesmd key annotated (~40 keys, diff vs DMISL).
- [x] Every artmd key annotated via Image=BSUBMISL redirect (4 keys
  in [BSUBMISL] block).
- [x] **No weapons** documented (Rules-global CMislWarhead +
  CMislEliteWarhead lookup explained, Westwood typo in comment
  noted).
- [x] Both warheads documented (CMISLWH basic + CMISLEWH elite mini-
  nuke) — both byte-identical to DMISL counterparts.
- [x] All voice/sound bindings documented (AuxSound1=BoomerAttack1
  shared with parent, audio cohesion pattern noted).
- [x] Spawn-child status (Spawned=yes + MissileSpawn=yes,
  TechLevel=-1).
- [x] Hardcoded behavior: trio architecture, CMislWarhead two-variant
  Rules-global, FlyBack=true confirming Burst>1 correlation, CSF
  label sharing, byte-identical warhead with DMISL.
- [x] TS-legacy filter applied.
- [x] Comparison table **closes the kamikaze missile trio** with
  V3ROCKET + DMISL + CMISL.
- [ ] **No Ghidra verification this iteration** (MCP server offline).

**Ghidra status**: MCP server still disconnected. No new cheat-sheet
entries. Field-scope claims cross-reference prior verified entries
from V3ROCKET, DMISL, BSUB, and earlier docs.

**Re-confirmed from prior cheat-sheet:**
- All shared fields with V3ROCKET/DMISL.
- `CMislWarhead` / `CMislEliteWarhead` Rules-global pattern (per
  BSUB doc open question — now confirmed by string presence and
  warhead block existence).

**Open questions resolved this iteration:**
- ✓ CMISL warhead resolution — confirmed `CMislWarhead`/`CMislEliteWarhead`
  Rules-global, byte-identical to DMislWarhead/DMislEliteWarhead.
- ✓ FlyBack ↔ Burst>1 correlation — confirmed across all 3 trio
  members (V3 Burst=1 no FlyBack; DRED+BSUB Burst=2 with FlyBack).

**Open questions remaining:**
- Ghidra trace of `FlyBack` field — scope and exact behavior.
- Ghidra trace of `CMislWarhead`/`CMislEliteWarhead` Rules-global
  offsets — confirm they're at distinct addresses from DMisl variants
  (likely; need verification).
- The `Image=BSUBMISL` redirect mechanism on AircraftType — confirm
  it works the same as TechnoType `Image=` redirects.
- The CSF label sharing (CMISL UIName=Name:DMISL) — does the engine
  warn/error when two units reference the same CSF entry, or is it
  silently accepted? Likely accepted; no harm.

**Kamikaze missile trio CLOSED**: V3ROCKET ✓ + DMISL ✓ + CMISL ✓.

**Spawn-children paradigm comparison complete:**
- Soviet+Yuri kamikaze trio (V3ROCKET, DMISL, CMISL) — uniform
  template, ~30 shared fields, RocketLocomotion.
- Allied return-to-dock pair (HORNET, ASW) — AircraftLocomotion,
  AuxSound1+2 active, Landable=yes operational.

5 spawn-children total documented across the two paradigms.
