---
name: v3rocket-doc
description: V3ROCKET — V3 Launcher's spawn-child kamikaze missile. Spawned=yes +
  MissileSpawn=yes (one-shot, no return). RocketLocomotion GUID (...9766). Damage
  comes from Rules-global V3Warhead=V3WH (Deform=10%, ProneDamage=70% "Presumes
  air burst"). Selectable=no (works because no landing required). NO Voice slots.
metadata:
  type: project
---

# V3ROCKET — V3 Rocket (kamikaze spawn-missile)

**INI ID:** `V3ROCKET`
**Display:** "V3 Rocket" (`UIName=Name:V3ROCKET`)
**Section:** `[AircraftTypes]`
**Owner side:** Soviet (Russians, Confederation, Africans, Arabs) — but
**not directly buildable** (`TechLevel=-1`); only exists as a spawn-child
of the [V3](../soviet/V3.md) Rocket Launcher.
**Role:** V3 Launcher's spawn-child suicide missile. **Kamikaze spawn
pattern** (`MissileSpawn=yes` — dies on impact, no return-to-dock).
Damage comes from a *Rules-global warhead* (`V3Warhead=V3WH`), not a
weapon defined on the V3ROCKET itself. Pairs with [DMISL](../soviet/DMISL.md)
(Dreadnought) and [CMISL](../yuri/CMISL.md) (Boomer Sub) as the three
Soviet+Yuri kamikaze missile spawn-children.

---

## Note on Ghidra unavailability this iteration

The Ghidra MCP server is offline this iteration (160 deferred tools
disconnected). No new cheat-sheet entries can be verified. All field-
scope claims below cross-reference *prior* cheat-sheet entries from
the existing index of verified ReadINI scopes.

---

## Rulesmd verbatim

```ini
[V3ROCKET]
UIName=Name:V3ROCKET
Name=V3 Rocket
FireAngle=1
Strength=50
Category=AirPower
Armor=special_2
Spawned=yes
MissileSpawn=yes
TechLevel=-1
Sight=1
RadarInvisible=no
Landable=yes
MoveToShroud=yes
Ammo=1 ;Aircraft are hard wired to require ammo
Speed=15
Owner=Russians,Confederation,Africans,Arabs
Cost=50
Points=20
ROT=3
Crewed=no
Explodes=no
GuardRange=30
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=2
Locomotor={B7B49766-E576-11d3-9BD9-00104B972FE8}
MovementZone=Fly
ThreatPosed=10	; This value MUST be 0 for all building addons
DamageParticleSystems=SmallGreySSys	; Sparks don't work well here.  SJM
AuxSound1=V3Attack ;Taking off
;AuxSound2=DROPDWN1 ;Landing
ImmuneToPsionics=yes
;VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
;EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
NoShadow=yes
Selectable=no
Trainable=no
DontScore=yes
```

### Key-by-key annotation

**Identity**
- `UIName=Name:V3ROCKET` — CSF resolves to "V3 Rocket".
- `Name=V3 Rocket` — internal description.
- `Category=AirPower` — sidebar/AI bucket (same as Hornet, BEAG).
- `FireAngle=1` — near-zero launch angle (vs Kirov's 64). V3ROCKET
  launches almost horizontally — characteristic of ballistic missiles
  that arc post-launch. TechnoType per cheat-sheet (`0x00843910 →
  0x00714b5d`).

**Tech / availability — pure spawn-only**
- *No `Prerequisite=`*.
- `TechLevel=-1` — unbuildable directly. Standard for spawn-children.
- `Owner=Russians,Confederation,Africans,Arabs` — 4 Soviet sub-factions.
  Matches parent V3.
- *No `AllowedToStartInMultiplayer=no`* — irrelevant for spawn-only.
- *No `CrateGoodie=`* — irrelevant.

**Spawn pattern — the kamikaze missile signature**
- `Spawned=yes` — spawn-child marker. TechnoType per cheat-sheet
  (`0x008437d8 → 0x00714e7d`).
- `MissileSpawn=yes` — **KEY KAMIKAZE FLAG**. TechnoType per
  cheat-sheet (`0x00843798 → 0x00714f23`). Differentiates one-shot
  suicide missiles from return-to-dock aircraft (HORNET, ASW).
  Mechanically:
  - Engine spawns V3ROCKET on V3's fire command.
  - Missile travels to target via RocketLocomotion.
  - On impact, missile *dies* — applies damage at impact cell.
  - V3 parent's SpawnManager removes the V3ROCKET from active list,
    flags the slot for refill via SpawnRegenRate.
  - No landing logic, no AuxSound2, no Ammo reload cycle.

**Combat — defense**
- `Strength=50` — fragile (similar to other spawn-missiles).
- `Armor=special_2` — *special-armor type*. Most flak weapons hit
  100% vs special_2 — easy to shoot down.

**Combat — NO weapons defined**
- *No `Primary=`*. *No `Secondary=`*. *No `ElitePrimary=`*.
- Damage on impact is handled via the **Rules-global `V3Warhead=V3WH`**
  setting at rulesmd line 819. See "Warhead" section below.
- V3WH is applied when the missile reaches its target — *hardcoded
  resolution from the Rules-global, not from a weapon line on the
  V3ROCKET itself*.

**Sight / radar**
- `Sight=1` — *minimal vision*. The missile doesn't scout — it has a
  pre-assigned target.
- `RadarInvisible=no` — appears on enemy radar.
- `Landable=yes` — *technically true but unused*. Since MissileSpawn=yes,
  the missile never lands; it dies on impact. The flag is set
  defensively (some engine code may require it on AircraftType units).
- `MoveToShroud=yes` — can fly into unexplored cells.
- `GuardRange=30` — autonomous tracking range after launch.

**Mobility**
- `Speed=15` — fast.
- `ROT=3` — moderate tracking.
- `Locomotor={B7B49766-E576-11d3-9BD9-00104B972FE8}` — **RocketLocomotion
  GUID**. **6th distinct locomotor type** (joining Drive ...741, Hover
  ...742, Aircraft ...746, Jumpjet 92612C46, Submarine 2BEA74E1).
  Specifically designed for one-shot missiles:
  - Vertical launch from origin.
  - Arcing horizontal travel.
  - Impact-detonation on target.
  - No takeoff/landing animations.
  See [ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  for full state machine.
- `MovementZone=Fly` — fly-zone pathing.

**Ammo (aircraft hardwiring)**
- `Ammo=1 ;Aircraft are hard wired to require ammo` — **verbatim
  Westwood comment**. Aircraft-class units require Ammo>0 to fire.
  V3ROCKET sets Ammo=1 as a defensive default — even though the
  missile fires once and dies, the engine's aircraft fire-check
  needs Ammo>0.

**Behavior flags**
- `Crewed=no` — no crew.
- `Explodes=no` — *does not detonate on its own death*. TechnoType per
  cheat-sheet (`0x0083355c → 0x007122c5`). The missile's damage is
  delivered via V3Warhead on target impact, NOT via the unit's own
  death-explosion. If V3ROCKET is shot down mid-flight, it just
  vanishes (no AoE).
- `GuardRange=30` — autonomous attack range after launch.
- `ThreatPosed=10` — modest AI threat (missile in flight).
- `Selectable=no` — **explicitly set** (unlike HORNET/ASW where the
  Westwood-bug commentary kept it commented). V3ROCKET *can* set
  Selectable=no without breaking landing because it has no landing.
  *Confirms the HORNET-bug theory*: the bug only manifests in
  Landable+return-to-dock units, not in fire-and-forget missiles.
- `Trainable=no` — no veterancy.
- `DontScore=yes` — *kills made by V3ROCKET do not grant XP back to
  the parent V3*. TechnoType per cheat-sheet (`0x00843ec0 →
  0x00713f4b`). The Soviet V3 only ranks up via its own direct kills,
  not via the missiles' kills. *Open*: this is the *child's* DontScore,
  but does it also affect score-on-self-death (the missile dying
  doesn't grant XP to whoever shoots it down)? Both interpretations
  plausible; field-name suggests both.

**Visuals**
- `Explosion=TWLT070,...` — explosion pool on death.
- `MaxDebris=2` — minimal debris.
- `DamageParticleSystems=SmallGreySSys` — **single particle system**
  (vs ground units with sparks+smoke). Verbatim comment: "Sparks don't
  work well here. SJM" — Westwood-tested that sparks didn't render
  correctly on the flying missile (maybe the spark spawn-position
  was inappropriate for high-altitude vehicles).
- `NoShadow=yes` — no shadow rendered. TechnoType per cheat-sheet
  (`0x008436e0 → 0x0071508e`). Consistent with high-altitude airborne
  missile.

**Voice / sound bindings (almost all empty)**
- *No `VoiceSelect=` / `VoiceMove=` / `VoiceAttack=` / `VoiceFeedback=`
  / `DieSound=`*. The missile has no player-interaction voices.
  *Consistent with Selectable=no — no need for voice samples since the
  player can't click on it.*
- `AuxSound1=V3Attack` — verbatim comment ";Taking off". Plays when
  the missile launches. Sample uses `vv3latta vv3lattb` (2-sample
  random-predelay, FShift -10 10).
- `;AuxSound2=DROPDWN1` — **commented landing sound** with placeholder
  reference. No landing event occurs (MissileSpawn=yes), so AuxSound2
  is moot anyway. The commented `DROPDWN1` would have been a generic
  "dropping down" SFX.

**Immunities**
- `ImmuneToPsionics=yes` — Yuri can't mind-control the missile mid-
  flight. Standard for all aircraft.

**Veterancy — disabled**
- `;VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` — commented.
- `;EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — commented.
- `Trainable=no` — no XP gain anyway.

---

## Artmd verbatim

```ini
[V3ROCKET]
;Trailer=DURASMOKE
SpawnDelay=2;1
Voxel=yes
Remapable=no
CanBeHidden=no
```

### Key-by-key annotation

- `;Trailer=DURASMOKE` — commented. The missile was once going to
  emit a DURASMOKE trailing animation; disabled in shipped YR.
  Trailer fields on projectiles/aircraft spawn anim-class objects
  in the flight path.
- `SpawnDelay=2;1` — *2-frame delay between spawn and active state*
  (`;1` historical commented). Used by the SpawnManager to stagger
  visual launch of burst-spawned children. With V3's Burst-related
  fields, multiple V3ROCKETs spawning in same frame would visually
  overlap; SpawnDelay=2 spreads them by 2 frames each.
- `Voxel=yes` — rendered from `v3rocket.vxl` + `.hva`.
- `Remapable=no` — **NOT remapable**. The missile uses default colors
  (gray/black), no house tint. Different from spawn-aircraft (HORNET,
  ASW) which also have no Remapable in artmd.
- `CanBeHidden=no` — never hidden by taller terrain/buildings. The
  missile is always rendered on top regardless of occluder.

**No `Cameo=`** — V3ROCKET doesn't appear in the sidebar (Selectable=no
+ no buildable).
**No `PrimaryFireFLH=`** — no weapon to define an FLH for.

---

## Weapons

**V3ROCKET has no weapons defined**. The damage on impact comes from
the *Rules-global warhead lookup*:

From rulesmd line 819:
```
V3Warhead=V3WH       ; this is the warhead on a V3 Rocket
```

This is a `[CombatDamage]` (or similar) section Rules-global that
declares the warhead used by V3 Rockets. The engine looks up
`V3Warhead` from Rules during the V3ROCKET's impact-detonation code
path, then applies V3WH at the impact cell.

**Cheat-sheet ref**: `V3Warhead` field at Rules-global level was
flagged in earlier notes. Rules-side mechanism for spawn-missile
warheads.

### Rules-global warhead — `[V3WH]`

```ini
[V3WH]
CellSpread=1
PercentAtMax=.25
Wall=yes
Wood=yes
Verses=100%,90%,80%,90%,70%,70%,100%,100%,50%,80%,0%
Conventional=yes
Rocker=yes
InfDeath=2
AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070
Deform=10%
DeformThreshhold=300
Tiberium=yes
Sparky=no
Bright=yes
ProneDamage=70%     ; Presumes air burst
```

- `CellSpread=1` — 1-cell AoE.
- `PercentAtMax=.25` — 25% damage at edge (heavy falloff). Direct
  hits matter most.
- `Wall=yes, Wood=yes` — damages walls and wooden buildings.
- `Verses=100%,90%,80%,90%,70%,70%,100%,100%,50%,80%,0%`:
  | Armor    | Multiplier |
  |----------|-----------|
  | none-flak-plate-light | 100/90/80/90 |
  | medium-heavy | 70/70 |
  | wood-steel | 100/100 |
  | concrete | 50 |
  | special_1 | 80 |
  | **special_2** | **0** |
  - 0% vs special_2 — **V3 rockets do not damage other V3 rockets**
    (special_2 is the missile-armor type). Self-protection against
    rocket-vs-rocket interactions.
  - 100% vs wood/steel (anti-structure focused).
  - 70% vs medium/heavy tanks. Useful but not ideal anti-armor.
- `Conventional=yes` — conventional damage.
- `Rocker=yes` — vehicles rock on impact.
- `InfDeath=2` — infantry-death type 2 (still undocumented in
  cheat-sheet — possible knockback variant).
- `Deform=10%` — terrain deformation chance. **Lower than SCHOPWH's
  15%** (Siege Chopper) and ARTYHE's 15% (Elite Hornet bomb). V3
  rocket impacts crater less frequently.
- `DeformThreshhold=300` — **much higher threshold than SCHOPWH (120)**.
  V3 rocket damage must exceed 300 to trigger deformation. With base
  V3WH damage being whatever the parent V3 deals + 25% edge falloff,
  reaching 300 requires direct hits with potentially elite damage
  scaling. **In practice**: V3 deformation is rare; SCHOPWH deformation
  is common.
- `Tiberium=yes` — affects ore tiles.
- `Sparky=no` — no spark effect.
- `Bright=yes` — palette flash on impact.
- `ProneDamage=70%` — **verbatim comment "Presumes air burst"**. Prone
  infantry take 70% damage (vs typical 50%). The 70% reflects that
  V3 rocket explodes *in the air above* the target, scattering damage
  downward — prone infantry can't dodge as well as they can vs
  ground-level explosions.

**Elite variant `[V3EWH]`** also exists (next section in rulesmd) for
elite V3 rockets — separate Rules-global as `V3EliteWarhead`. Not
documented in this iteration.

---

## Voices / sounds

```ini
[V3Attack]
Sounds= vv3latta vv3lattb
Control=random predelay
Delay=0 300
Limit=3
FShift= -10 10
Volume=70
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| (all Voice* / DieSound empty) | n/a | No player-interaction voices |
| `AuxSound1=V3Attack` | `[V3Attack]` | **Plays when missile launches from V3** (2-sample random-predelay) |
| `;AuxSound2=DROPDWN1` | (commented) | Landing event — moot (no landing) |

**Notable**: V3ROCKET uses AuxSound1 actively (`V3Attack` launch
sound) but commented AuxSound2 (no landing). Compare HORNET/ASW which
have both AuxSound1+AuxSound2 active because they return to dock.
**The AuxSound2 commenting confirms the kamikaze pattern**: no return,
no landing event, no need for the landing SFX slot.

---

## Hardcoded behavior

### 1. MissileSpawn=yes kamikaze pattern

The defining flag distinguishing V3ROCKET from HORNET/ASW. With
MissileSpawn=yes:
- Engine doesn't pursue landing logic.
- On target impact, the missile despawns (no debris cleanup needed).
- Parent V3's SpawnManager schedules a refill via SpawnRegenRate (80
  ticks per V3 doc).
- No AuxSound2 / landing event triggered.

See [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
for the full SpawnManager dispatcher logic.

### 2. RocketLocomotion (6th locomotor type)

`Locomotor={B7B49766-E576-11d3-9BD9-00104B972FE8}` — **RocketLocomotion**.
Sixth distinct GUID after Drive (...741), Hover (...742),
Aircraft (...746), Jumpjet (92612C46), Submarine (2BEA74E1).

The RocketLocomotion handles:
- Vertical launch at FireAngle from parent.
- Arcing flight trajectory to target.
- Impact-detonation on arrival.
- No landing/refuel logic.

Shared by V3ROCKET, DMISL (Dreadnought missile), CMISL (Boomer Sub
cruise missile). The three kamikaze missile children all use this
locomotor.

### 3. Selectable=no (works because no landing)

The verbatim Westwood-bug comment from HORNET said:
> "Selectable=no should be here but is commented out because bug
> prevents aircraft from landing"

V3ROCKET *can* set `Selectable=no` because *no landing happens*. This
confirms the bug interpretation: the Selectable system is required by
the landing-return code path, not by the firing or movement systems.
Kamikaze missiles don't trigger that path → can be unselectable.

### 4. Rules-global V3Warhead resolution

V3ROCKET has no weapons defined. Damage on impact comes from the
Rules-global `V3Warhead=V3WH` setting. This is a hardcoded engine
lookup — the missile-class code path queries Rules for the warhead
name when reaching the target.

Similar Rules-global warhead lookups for missile spawn-children:
- `V3Warhead` (for V3ROCKET).
- `DMislWarhead` / `DMislEliteWarhead` (for DMISL, per cheat-sheet
  Rules+0xfb4/+0xfbc from DRED doc).
- Open: CMISL's warhead resolution — likely uses a similar Rules-
  global (`CMislWarhead`?). Need to verify in CMISL iteration.

### 5. Explodes=no, NoShadow=yes, DontScore=yes

Three flags that together model the "this is a missile, not a real
unit" semantics:
- `Explodes=no` — no detonation on self-death (missile damages target,
  not itself).
- `NoShadow=yes` — no shadow rendered (high-altitude flight).
- `DontScore=yes` — kills made don't grant XP back to parent.

These flags signal "treat me as a transient projectile, not a
permanent entity" to the various engine subsystems (gore, score,
shadow renderer).

### 6. Trainable=no + commented Veteran/Elite abilities

Standard for spawn-children. The missile never gains XP, so the
ability lists are moot. They're commented for documentation purposes
— if a modder wanted to enable spawned-missile veterancy, they could
uncomment and the system might work (though the parent V3's XP
mechanism probably blocks this in practice).

---

## TS-legacy filter

- `;Trailer=DURASMOKE` — commented historical trailer animation.
- `;AuxSound2=DROPDWN1` — commented landing SFX.
- `;VeteranAbilities` / `;EliteAbilities` — commented veterancy.
- `SpawnDelay=2;1` historical value.
- No `ImmuneToVeins`, no `Subterranean`. YR-active mechanism.

---

## Comparison: V3ROCKET vs HORNET (Soviet kamikaze vs Allied return-to-dock)

| Field | V3ROCKET (Soviet kamikaze) | HORNET (Allied return-to-dock) |
|-------|----------------------------|----------------------------------|
| Section | AircraftTypes | AircraftTypes |
| Spawned | yes | yes |
| **MissileSpawn** | **yes** | **no** |
| TechLevel | -1 | -1 |
| Strength | 50 | 75 |
| Cost | 50 | 50 |
| Locomotor | **RocketLocomotion ({B7B49766-...})** | AircraftLocomotion ({4A582746-...}) |
| Weapons | **(none — Rules-global V3Warhead)** | HornetBomb + HornetCollision |
| Landable | yes (unused) | yes (active) |
| Selectable | **no (explicit)** | no (commented — bug) |
| AuxSound1 (takeoff) | active (V3Attack) | active (HornetTakeoff) |
| AuxSound2 (landing) | **commented (no landing event)** | active (HornetLanding) |
| Veterancy | disabled | reduced 2-ability |
| ElitePrimary | n/a | HornetBombE |
| DontScore | yes | not set |

**Defining contrast:** *MissileSpawn=yes* changes the entire architecture:
- Kamikaze: dies on impact, no return logic, simpler entity lifecycle.
- Return-to-dock: lands at parent, refuels, sortie cycle.

V3ROCKET, DMISL, CMISL all share the kamikaze pattern. HORNET, ASW
share the return-to-dock pattern. **Asymmetric Soviet/Yuri kamikaze
vs Allied reusable air-power doctrine**.

---

## Cross-references

- [V3.md](./V3.md) — parent V3 Launcher. SpawnsNumber=1
  SpawnReloadRate=0 (single one-shot rocket).
- [DMISL.md](./DMISL.md) — pending. Dreadnought's missile (peer
  kamikaze spawn-child).
- [CMISL.md](../yuri/CMISL.md) — pending. Boomer Sub's cruise missile
  (peer kamikaze).
- [HORNET.md](../allied/HORNET.md) — counterpart return-to-dock pattern.
- [ASW.md](../allied/ASW.md) — peer return-to-dock.
- [ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — RocketLocomotion state machine.
- [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
  — SpawnManager state machine.

---

## Coverage audit

- [x] Every rulesmd key annotated (~40 keys).
- [x] Every artmd key annotated (5 keys including commented Trailer).
- [x] **No weapons** documented (Rules-global V3Warhead lookup
  explained).
- [x] V3WH warhead documented (Deform=10%, ProneDamage=70% "Presumes
  air burst", 0% vs special_2).
- [x] All voice/sound bindings documented (active AuxSound1, commented
  AuxSound2).
- [x] Spawn-child status (Spawned=yes + MissileSpawn=yes,
  TechLevel=-1).
- [x] Veterancy: disabled (Trainable=no + commented Vet/Elite).
- [x] Hardcoded behavior: MissileSpawn pattern, RocketLocomotion
  (6th locomotor), Selectable=no proof-of-bug-fix, Rules-global
  warhead resolution, Explodes=no + NoShadow=yes + DontScore=yes
  triple.
- [x] TS-legacy filter: trailer + landing-SFX + veterancy commented.
- [x] Comparison table closes V3ROCKET vs HORNET (kamikaze vs return-
  to-dock).
- [ ] **No Ghidra verification this iteration** (MCP server offline).

**Ghidra status**: MCP server disconnected. All field-scope claims
cross-reference *prior* cheat-sheet entries:
- `Spawned` (TechnoType, MIND cheat-sheet)
- `MissileSpawn` (TechnoType, MIND cheat-sheet)
- `FireAngle` (TechnoType, MIND cheat-sheet)
- `Explodes` (TechnoType, cheat-sheet)
- `NoShadow` (TechnoType, SQD doc)
- `DontScore` (TechnoType, cheat-sheet)
- `Landable` (AircraftType, HORNET doc)
- `ImmuneToPsionics` (TechnoType, cheat-sheet)
- `Trainable` (TechnoType, cheat-sheet)
- `AuxSound1` (TechnoType, HORNET doc)

**No new cheat-sheet entries this iteration** due to Ghidra
unavailability.

**Open questions:**
- DontScore semantics (per-kill XP only? Or also per-self-death XP-to-
  killer?). Open follow-up.
- V3EliteWarhead (Rules-global) — separate from V3Warhead. V3 elite
  rank fires the elite missiles using a different warhead.
- The kamikaze missile trio (V3ROCKET, DMISL, CMISL) — close the trio
  in upcoming iterations.
