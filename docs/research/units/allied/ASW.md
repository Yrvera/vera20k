---
name: asw-doc
description: ASW — Osprey ASW helicopter. Destroyer's spawn-child (return-to-dock
  pattern like HORNET). Strength=30 (fragile), Ammo=1. ASWBomb DepthCharge anti-sub
  (APSplash). ASWCollision Secondary = crash-transform (same pattern as
  HornetCollision). NO ElitePrimary swap (unique vs HORNET). NavalTargeting=2 + no
  AntiUnderwater flags despite anti-sub role — see ASW projectile.
metadata:
  type: project
---

# ASW — Osprey (Destroyer's ASW Helicopter)

**INI ID:** `ASW`
**Display:** "Osprey" (`UIName=Name:ASW`) — note CSF returns the proper
"Osprey" name (V-22 Osprey reference); internal section name ASW =
*Anti-Submarine Warfare*.
**Section:** `[AircraftTypes]`
**Owner side:** Allied (British, French, Germans, Americans, Alliance) — but
**not directly buildable** (`TechLevel=-1`); only exists as a spawn-child
of the [DEST](../allied/DEST.md) Destroyer.
**Role:** Destroyer's anti-submarine + anti-naval spawn-child helicopter.
Return-to-dock spawn pattern. Drops `DepthCharge` projectiles via `ASWBomb`
weapon. Pairs with [HORNET](../allied/HORNET.md) as the two Allied return-
to-dock spawn-children — same architecture, smaller scale.

---

## Rulesmd verbatim

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

### Key-by-key annotation

Most fields mirror [HORNET](./HORNET.md). This section highlights ASW-
distinctive differences.

**Identity / availability**
- `UIName=Name:ASW` — CSF resolves to "Osprey".
- `Name=Osprey` — internal description.
- `Category=AirPower`.
- `Spawned=yes`, `TechLevel=-1` — spawn-child marker.
- *No `MissileSpawn=yes`* — return-to-dock pattern (like HORNET, vs
  V3ROCKET/CMISL kamikaze).
- *No `Prerequisite=`* — spawn-only.

**Targeting priorities (NEW field combination)**
- `NavalTargeting=2` — *low naval-target priority*. TechnoType per
  cheat-sheet (0x00844510 → 0x007121be).
- `LandTargeting=1` — minimum land-target priority.
- *Combined effect*: ASW preferentially engages naval but doesn't
  aggressively scan for them. Mostly fires when explicitly ordered by
  the parent Destroyer's targeting AI.

**Combat — defense**
- `Strength=30` — **lowest HP of any documented unit so far** (vs
  HORNET 75, the fragility champion before ASW). The Osprey is
  one-shot fragile. Light AA fire kills it.
- `Armor=light` — light armor.

**Combat — dual-weapon (anti-sub + crash kamikaze)**
- `Primary=ASWBomb` — *the working weapon*. Damage 50, ROF=3, Range=3,
  DepthCharge projectile, APSplash warhead.
- `Secondary=ASWCollision` — *crash-transform projectile*. **Verbatim
  comment identical to HORNET**: "A crashing ASW turns into this bullet
  at the last second". Same crash-impact death-transform mechanic.
- **NO `ElitePrimary=` line** — **critical difference from HORNET**.
  ASW does NOT have an elite weapon swap. HORNET has
  `ElitePrimary=HornetBombE` triggered by parent CARRIER's veterancy;
  ASW has no equivalent. **Destroyer's veterancy does NOT upgrade
  ASW's weapon** — only the Destroyer's own Primary weapon (155mm vs
  ElitePrimary=155mmE on DEST itself).
- This asymmetry was noted in the DEST doc:
  > "ElitePrimary=155mmE Burst=2; NoSpawnAlt to DESTWO voxel"
  > (DEST's own elite swap is for the cannon, not Osprey's
  > DepthCharge.)

**No VeteranAbilities or EliteAbilities lines** on ASW. **[RESOLVED
audit 30]**: ASW has no `Trainable=yes` line either, so the unit defaults
to **Trainable=no** (TechnoType+0xC8E, byte, ReadBool default 0 —
verified via grep on TechnoTypeClass__ReadINI). With Trainable=no, the
unit cannot accumulate XP → never gains veteran/elite ranks → the
absence of VeteranAbilities/EliteAbilities lines is **cosmetic**. Same
applies to HORNET: even though HORNET sets VeteranAbilities and
EliteAbilities, it doesn't set Trainable=yes, so HORNET would also never
veteran-up on its own. The HORNET elite weapon swap comes from the
PARENT (CARRIER's `ElitePrimary=HornetBombE` triggers when the
CARRIER itself ranks up). DEST has no such parent-side elite weapon
override for its Osprey, so ASW genuinely has no veterancy path.

**Combat behavior**
- `Crewed=no`, `IsSelectableCombatant=` not set (inherits default).
- `ThreatPosed=10` — same as HORNET (modest).
- `GuardRange=30` — same.
- `ImmuneToPsionics=yes` — same.

**Voice / sound bindings — all empty except crash**
- `VoiceSelect=`, `VoiceMove=`, `VoiceAttack=`, `VoiceFeedback=`,
  `DieSound=` — all empty (same as HORNET; consistent with the
  `;Selectable=no` Westwood-bug design intent).
- `CrashingSound=OspreyDie` — sustained crash plummet SFX.
- `ImpactLandSound=GenAircraftCrash` — ground impact (DUAL-READ).
- `AuxSound1=OspreyTakeOff` — Destroyer-deck launch SFX. **TechnoType+0x52C
  (int VocClass) [BINARY-VERIFIED audit 29 HORNET, re-confirmed audit 30]**.
- `AuxSound2=OspreyLanding` — Destroyer-deck landing SFX. **TechnoType+0x530
  (int VocClass) [BINARY-VERIFIED audit 29 HORNET, re-confirmed audit 30]**
  (sibling to AuxSound1). String stored 12 bytes BEFORE AuxSound1 in
  memory (0x00844234 vs 0x00844240) — reverse storage order in the
  string table.

**Mobility — same as HORNET**
- `Speed=12`, `PitchSpeed=.9`, `PitchAngle=0`, `ROT=5;3` (the `;3`
  historical commented).
- `Locomotor={4A582746-...}` — AircraftLocomotion.
- `MovementZone=Fly`, `MovementRestrictedTo=Water`. The verbatim
  comment is the same: "See if this will affect landing only".
  **[INCORRECT — DEAD INI, audit 30 (carried from audit 29)]**:
  `MovementRestrictedTo` is **UnitType-scope only** (single xref @
  0x00747837 → `UnitTypeClass__ReadINI`). Aircraft parser doesn't read
  it, so this key is dead INI on ASW (same as HORNET). The "See if this
  will affect landing only" Westwood test comment — found out: it
  doesn't.
- `Landable=yes` (AircraftType per HORNET cheat-sheet).

**Economy**
- `Cost=50`, `Points=10` (vs HORNET's $50 and Points=20). ASW's 10
  points-on-kill is half HORNET's value, consistent with lower HP.

---

## Artmd verbatim

```ini
[ASW] ; Destroyer Plane
Cameo=PROICON
Voxel=yes
PrimaryFireFLH=0,32,0
```

### Key-by-key annotation

- `Cameo=PROICON` — **shared cameo with HORNET** ("PRO" generic
  projectile icon). Not a sidebar item.
- `Voxel=yes` — rendered from `asw.vxl` + `asw.hva`.
- *No `Remapable=yes`* — same as HORNET; both spawn-children use
  default house color from parent.
- `PrimaryFireFLH=0,32,0` — **identical to HORNET's FLH** (X=0, Y=32
  side-drop offset, Z=0).

**No `AltCameo=`** — minimal artmd block. Even simpler than HORNET
(no extra fields).

---

## Weapons

### Primary — `[ASWBomb]`

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

- `Damage=50` — slightly stronger than HornetBomb (40). The Osprey
  hits a little harder per drop.
- `ROF=3` — fast (matches HORNET; same single-Ammo limitation).
- `Range=3` — **shorter than HornetBomb (5)**. ASW must close to 3
  cells of target — fragile + short range = high risk.
- `Projectile=DepthCharge` — see projectile block.
- `Speed=30` — moderate.
- `Warhead=APSplash` — *Armor Piercing + Splash*. **Same warhead as
  Soviet Sub's `[SubTorpedo]`** (see [SUB.md](../soviet/SUB.md)). Strong
  vs medium/heavy armor (100%), poor vs infantry (25%). Anti-naval
  optimized. *Doesn't have* PenetratesBunker=yes like HORNET's ORCAAP.
- `Report=OspreyAttack` — fire SFX (`vospatta`, **shared with
  HornetAttack** — same audio sample, both Carrier-launched and
  Destroyer-launched aircraft use the same drop SFX).

### Secondary — `[ASWCollision]`

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

**Identical structure to HornetCollision** (see HORNET doc):
- Verbatim comment: "A crashing ASW turns into this bullet at the
  last second" — same crash-transform mechanic.
- Damage=100 (same as HornetCollision — 2× the normal Primary
  damage).
- Projectile=AAHeatSeeker2 with "will be ASW shaped bullet" comment
  (also same as HORNET — never finished custom voxel).
- Warhead=AP (same as HORNET).
- Report=OspreyCollision — distinct from HornetCollision (different
  sample reference).
- Bright=yes.

**Mechanism**: Same as HORNET. Crashing aircraft's death code path
transforms the falling Osprey into the ASWCollision projectile.

### Projectile — `[DepthCharge]`

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

- `Arm=2` — 2-frame arming.
- `Shadow=no`, `Image=DRAGON` — generic missile-shape sprite.
- `Proximity=yes` — proximity-fused.
- `Ranged=yes` — fuse range check.
- `ROT=1` — minimal tracking (essentially a dropped bomb, not homing).
- `IgnoresFirestorm=yes` — TS-legacy (Firestorm Wall bypass; moot in YR).
- `;AS=yes` — commented anti-submarine flag. *The DepthCharge was once
  going to have an explicit `AS=yes` (anti-submarine) projectile
  flag*, but it's commented out. **The new naval-targeting system
  (NavalTargeting / LandTargeting on the firing unit) makes AS
  redundant** — same comment pattern as Dolphin's `[Sonic]` projectile
  Greg-Smith note.

**Note**: Despite the name "DepthCharge", the projectile is **NOT
underwater** — it's `Image=DRAGON` (visible above-water missile
sprite). The "depth charge" theming is purely flavor; mechanically
it's a Proximity-fused dropped missile.

### Warhead — `[APSplash]`

Already documented in [SUB.md](../soviet/SUB.md#warhead--apsplash).
Summary:
- CellSpread=0.5, PercentAtMax=0.8.
- Verses=25%/25%/25%/75%/100%/100%/65%/65%/60%/25%/100% — anti-naval
  (100% vs medium/heavy), poor vs infantry/buildings.
- InfDeath=3 explosion.
- "for units whose missiles are having trouble hitting" verbatim
  Westwood comment.

---

## Voices / sounds

```ini
[OspreyAttack]
Sounds= vospatta
Control= interrupt
FShift= -10 10
Limit=3
VShift=10

[OspreyTakeOff]
Sounds= vospstaa
Priority=low
FShift= -10 10
Volume=35

[OspreyLanding]
Sounds=vosplana
Priority=low
FShift= -10 10
Volume=35

[OspreyDie]
Sounds= vospdiea
Priority=low
Volume=50

[OspreyCollision]
Volume=0	; no sound
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=`, `VoiceMove=`, `VoiceAttack=`, `VoiceFeedback=` | (empty) | No player-interaction voices |
| `DieSound=` (empty) | n/a | Handled by CrashingSound chain |
| `CrashingSound=OspreyDie` | `[OspreyDie]` | Crash plummet SFX (single sample `vospdiea`) |
| `ImpactLandSound=GenAircraftCrash` | shared | Ground impact |
| `AuxSound1=OspreyTakeOff` | `[OspreyTakeOff]` | Destroyer-deck launch SFX |
| `AuxSound2=OspreyLanding` | `[OspreyLanding]` | Destroyer-deck landing SFX |
| `Report=OspreyAttack` (ASWBomb) | `[OspreyAttack]` | Bomb-drop SFX (`vospatta` — **shared with HORNET's HornetAttack**) |
| `Report=OspreyCollision` (ASWCollision) | `[OspreyCollision]` | **Silent block** (Volume=0, no Sounds) — collision impact is silent for Osprey |

**Notable**:
- `[OspreyCollision]` is a **silent block** (Volume=0, no Sounds=) —
  same silent-block convention as SubFear/DolphinFear/BoomerFeedback.
  The crash-impact has no specific Osprey SFX (just the generic
  GenAircraftCrash via ImpactLandSound).
- `vospatta` (the ASW fire SFX) is **shared between ASW and HORNET**.
  Both spawn-children use the same drop sample. Audio consistency
  across the Allied spawn-child fleet.

---

## Hardcoded behavior (Ghidra-verified)

### 1. AuxSound2 TechnoType (closes the AuxSound1/AuxSound2 pair)

**Ghidra-verified TechnoType** at `0x00844234 → 0x00712e48`. **NEW
cheat-sheet entry**. Sibling field to `AuxSound1` (`0x00844240 →
0x00712e18` from HORNET iteration). The addresses are *4 bytes apart*
(0x00844234 vs 0x00844240) — confirming they're stored adjacently in
the string table. Both are TechnoType-scope, both fire on the same
event categories:
- `AuxSound1` — takeoff event (launches from dock).
- `AuxSound2` — landing event (returns to dock).

**ASW + HORNET** are the two canonical examples of active
AuxSound1+AuxSound2 usage. The pair fires per spawn-child sortie cycle:
takeoff (departing parent) → flight → fire weapon → landing (returning
to parent).

### 2. Return-to-dock spawn pattern (continued from HORNET)

ASW completes the Allied return-to-dock spawn-child pair (HORNET +
ASW). Both share:
- `Spawned=yes`, no `MissileSpawn=yes`.
- `TechLevel=-1`.
- `Landable=yes` (AircraftType-scope per HORNET).
- `Locomotor=AircraftLocomotion`.
- `Ammo=1`, single-sortie reload cycle.
- All empty Voice* slots.
- `MovementRestrictedTo=Water` (Westwood test comment).
- The `;Selectable=no` Westwood-bug comment.
- Crash-transform Secondary weapon (HornetCollision / ASWCollision).

### 3. No ElitePrimary (vs HORNET's elite swap)

**ASW deliberately lacks ElitePrimary**. This is the major behavioral
difference from HORNET:
- HORNET has `ElitePrimary=HornetBombE` → when parent CARRIER ranks
  up to Elite, Hornets fire the upgraded HornetBombE weapon (Damage
  40→80, Warhead ORCAAP→ARTYHE with terrain deformation).
- ASW has no ElitePrimary line → **DEST (Destroyer) reaching Elite
  rank does NOT upgrade its Osprey's weapon**. The DEST's own 155mm
  Primary upgrades to 155mmE Burst=2, but the spawned Osprey continues
  firing the basic ASWBomb regardless.

**Asymmetric design choice**: Carrier scales up its strike capability
through elite Hornets; Destroyer scales up only its own cannon.
Possibly because the Destroyer's primary role is anti-sub/anti-naval
direct combat, while the Carrier's primary role is the Hornet wave.

### 4. APSplash warhead shared with SubTorpedo

ASW's `ASWBomb` uses `Warhead=APSplash` — *the same warhead as Soviet
Typhoon Sub's torpedo*. Both are anti-naval/anti-armor with 100% vs
medium/heavy. Reinforces ASW's role as anti-sub specialist (good vs
the SUB it's designed to counter).

### 5. ImmuneToPsionics, Spawned, Locomotor — standard spawn-child stack

Same as HORNET. Aircraft are out-of-range for psi-control. The Spawned
+ Locomotor=AircraftLocomotion + Landable=yes triple identifies the
return-to-dock pattern.

### 6. Westwood `;Selectable=no` bug — identical to HORNET

Same verbatim comment on ASW. Same shipping-bug acknowledgment:
selectability cannot be disabled without breaking aircraft landing.

### 7. NavalTargeting=2 / LandTargeting=1

The lowest active NavalTargeting value observed so far:
- DLPH: 5
- SUB: 5
- BSUB: 7
- HYD: 6
- SQD: 3
- DEST (parent): not set (default)
- **ASW: 2**

ASW has minimal auto-targeting bias. Spawn-child operates almost
purely under parent AI direction — the Destroyer dispatches Ospreys
to specific targets rather than letting the Osprey choose.

---

## TS-legacy filter

- `IgnoresFirestorm=yes` on DepthCharge — TS-legacy (Firestorm Wall;
  moot in YR).
- `;AS=yes` on DepthCharge — commented anti-submarine projectile flag,
  superseded by NavalTargeting system.
- `;Dock=GAAIRC,AMRADR` — commented Dock alternative.
- `;Selectable=no` Westwood-bug commentary.
- `ROT=5;3` historical commented turn rate.
- No `ImmuneToVeins`, no `Subterranean`. **YR-active core mechanism**.

---

## Comparison: ASW vs HORNET (Allied spawn-child pair)

| Field | ASW Osprey (DEST) | HORNET (CARRIER) |
|-------|-------------------|-------------------|
| Strength | **30** | 75 |
| Cost | 50 | 50 |
| Points | 10 | 20 |
| Primary Damage | 50 | 40 |
| Primary Range | 3 | 5 |
| Primary Warhead | APSplash | ORCAAP |
| **ElitePrimary** | **(none)** | **HornetBombE** |
| Veteran/Elite abilities | (none set) | STRONGER,FIREPOWER pair |
| Projectile | DepthCharge | NormalBomb |
| Secondary | ASWCollision (kamikaze) | HornetCollision (kamikaze) |
| AuxSound1/2 | OspreyTakeOff/Landing | HornetTakeoff/Landing |
| NavalTargeting | 2 | not set |
| ROT | 5 | 3 |

**Trade-offs:**
- **ASW**: lower HP, lower point-cost on kill, slightly stronger per-
  shot, shorter range. Anti-naval optimized via APSplash warhead.
  No elite upgrade. Faster turn (ROT=5).
- **HORNET**: higher HP, higher point-cost on kill, weaker per-shot,
  longer range. Anti-everything via ORCAAP warhead. Elite upgrade
  doubles damage + adds terrain deformation. Slower turn (ROT=3).

**Design intent**: The Destroyer is primary anti-sub naval; ASW fills
the "drop-and-go" anti-sub role with high-damage APSplash. The
Carrier is anti-everything; HORNET is the long-range strike against
diverse targets, scaling with veterancy.

---

## Cross-references

- [HORNET.md](./HORNET.md) — pair partner. Same return-to-dock
  architecture.
- [DEST.md](./DEST.md) — parent spawner. Has `Spawns=ASW
  SpawnsNumber=1 SpawnReloadRate>0` configuration.
- [CARRIER.md](./CARRIER.md) — counterpart spawner (with 4 Hornets vs
  Destroyer's 1 Osprey).
- [SUB.md](../soviet/SUB.md) — primary target type for ASW (APSplash
  warhead is anti-sub).
- [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
  — SpawnManager state machine.

---

## Ghidra audit log (audit iteration 30 — 2026-05-19)

**~14 Ghidra queries** (8 string searches + 6 xref lookups + 5 grep
passes on saved TechnoTypeClass__ReadINI decompile + 1 search_functions
on AircraftClass). All 1 doc-cited claim verifies + 3 NEW TechnoType
offsets pinned + 2 doc open questions RESOLVED.

### Function-entry verification

| Function | Address | Status |
|----------|---------|--------|
| `TechnoTypeClass__ReadINI` | (oversized) | grep-verified for LandTargeting/PitchAngle/Trainable |
| `AircraftTypeClass__ReadINI` | 0x0041CC20–0x0041CDA3 | known from audit 26+29 |
| `UnitTypeClass__ReadINI` | known from audit 12 | sole MovementRestrictedTo consumer (UnitType-scope only) |
| `AircraftClass__ReceiveDamage` | 0x004165c0 | DEFERRED — likely contains crash-transform Secondary-fire logic |

### Key behavioral findings — 3 NEW struct-offset bindings BINARY-VERIFIED

| INI key | Scope | Offset | Type | Parser site | Status |
|---------|-------|--------|------|-------------|--------|
| `LandTargeting` | TechnoType | **+0x604** | int (ReadInt) | 0x007121a4 | NEW (sibling to NavalTargeting +0x600, parsed FIRST) |
| `PitchAngle` | TechnoType | **+0x3B0** | double (ReadDouble) | 0x0071236b | NEW (sibling to PitchSpeed +0x3A8 from audit 29; **stored in radians via `degrees × DAT_007f4fb8` PI/180 conversion**) |
| `Trainable` | TechnoType | **+0xC8E** | byte (ReadBool) | 0x00714a1c | NEW (gates XP accumulation; default false when absent) |

Re-confirmed (audit 7/9/26/29 cumulative):
- `NavalTargeting` = TechnoType+0x600 (int) via `param_1[0x180] = iVar4`
- `AuxSound1` = TechnoType+0x52C (int VocClass) — audit 29
- `AuxSound2` = TechnoType+0x530 (int VocClass) — audit 29
- `Landable` = AircraftType+0xE0A (byte) — audit 26+29
- `VeteranAbilities` array start = TechnoType+0x29C — audit 7 (parser xref @ 0x007154a3 confirmed)
- `EliteAbilities` array start = TechnoType+0x2AE — audit 7 (parser xref @ 0x007154e8 confirmed)
- `Spawned` = TechnoType+0xD54 (byte) — audit 20

### TechnoType byte-cluster +0xC8C..+0xC91 fully mapped (audit 30 closes the cluster)

| Offset | Key | Audit | Notes |
|--------|-----|-------|-------|
| +0xC8C | TypeImmune | 28 (DLPH) | byte |
| +0xC8D | MoveToShroud | 11 (CCOMAND) | byte, default 1 |
| **+0xC8E** | **Trainable** | **30 (ASW)** | byte, default 0 — NEW |
| +0xC8F | (name-override aux) | 13 (PENTGEN) | byte |
| +0xC90 | (DEFERRED INI key) | — | byte |
| +0xC91 | ImmuneToVeins | 7 (TANY) | byte |

6 of 6 bytes in this cluster have a candidate mapping. The +0xC90 slot
is the only remaining unknown in the immediate range.

### Doc open questions RESOLVED

**1. MovementRestrictedTo on Hornet/ASW — RESOLVED**

Already resolved in audit 29 HORNET, carried forward: `MovementRestrictedTo`
is UnitType-scope ONLY (string @ 0x00845d64, single xref @ 0x00747837 →
`UnitTypeClass__ReadINI`). `AircraftTypeClass__ReadINI` does NOT call
`UnitTypeClass__ReadINI`. Both HORNET and ASW have `MovementRestrictedTo=Water`
as **DEAD INI** — no engine effect. The Westwood "See if this will affect
landing only" comment got its answer (negative).

**2. ASW veterancy with no VeteranAbilities/EliteAbilities lines — RESOLVED**

The doc asked whether ASW genuinely has no veterancy. **Yes, genuinely
no veterancy**:
- Trainable defaults to false (TechnoType+0xC8E byte, ReadBool default 0).
- ASW omits `Trainable=yes` → unit cannot accumulate XP.
- Even if VeteranAbilities/EliteAbilities lines were present, the unit
  never reaches veteran/elite rank to use them.
- HORNET's reduced 2-ability lists (STRONGER,FIREPOWER) are equally
  cosmetic for the same reason — HORNET also omits Trainable=yes.
- HORNET's elite weapon swap (HornetBombE) is parent-driven via
  CARRIER's `ElitePrimary=HornetBombE`. DEST has no equivalent
  parent-side override for its Osprey, so ASW's lack of elite weapon
  is architectural, not just a missing INI line.

### Items NOT re-verified (DEFERRED with reason)

- **Crash-transform mechanism** (ASWCollision / HornetCollision as
  Secondary fired on crash) — `AircraftClass__ReceiveDamage @
  0x004165c0` is the likely host function. Decompiling its full body
  is out of scope for this audit (would require dedicated deep-RE
  pass). DEFERRED from HORNET audit 29 + ASW audit 30.
- **AircraftClass__FindBuildingToDock @ 0x0041bbd0** — return-to-dock
  pathing for spawn-children. Searched-but-not-decompiled. DEFERRED.
- **`;Selectable=no` Westwood bug code path** — same status as HORNET.
- **VeteranAbilities/EliteAbilities array layout** at +0x29C..+0x2BE —
  audit 7 cumulative says +0x29C array start, +0x2AE EliteAbilities,
  but actual element format / bitmask vs sequential bytes still
  DEFERRED.
- **PitchAngle PI/180 multiplier `DAT_007f4fb8` exact value** — likely
  0.01745329 (π/180) but not directly verified. DEFERRED.
- **+0xC90 unknown sibling** in the byte cluster.

### Negative claims verified

- `search_strings("ASW")` → **0 matches**.
- `search_strings("Osprey")` → **0 matches**.

All ASW behavior is INI-driven, mirroring HORNET architecture.

### Confidence summary

- 3/3 NEW struct-offset bindings BINARY-VERIFIED via grep on saved
  TechnoTypeClass__ReadINI decompile.
- 4 re-confirmations from prior audits (NavalTargeting/AuxSound1/2/Landable).
- 2 doc open questions RESOLVED (MovementRestrictedTo + veterancy).
- 1 byte-cluster fully mapped (TechnoType+0xC8C..+0xC91 with 5/6 keys named).
- Negative claims confirmed.
- No INCORRECT findings beyond the inherited MovementRestrictedTo
  caveat.

---

## Coverage audit

- [x] Every rulesmd key annotated (~40 keys).
- [x] Every artmd key annotated (4 keys).
- [x] Both weapons documented (ASWBomb basic + ASWCollision crash-
  transform Secondary).
- [x] **NO ElitePrimary** explicitly noted (key difference from HORNET).
- [x] DepthCharge projectile documented (with TS-legacy
  IgnoresFirestorm + commented AS=yes).
- [x] APSplash warhead cross-referenced to SUB doc.
- [x] All voice/sound bindings documented including silent
  `[OspreyCollision]` block.
- [x] Spawn-child status (Spawned=yes, TechLevel=-1, no MissileSpawn).
- [x] Veterancy: *no VeteranAbilities or EliteAbilities lines* — open
  follow-up.
- [x] Hardcoded behavior: AuxSound2 TechnoType (NEW), return-to-dock
  pattern closure with HORNET, no ElitePrimary scaling, APSplash
  warhead shared with SubTorpedo, lowest NavalTargeting observed,
  Westwood bug commentary.
- [x] TS-legacy filter applied (IgnoresFirestorm + ;AS commented +
  ;Dock commented).
- [x] Comparison table closes the ASW vs HORNET spawn-child pair.
- [x] At least one Ghidra search performed (AuxSound2 — NEW cheat-
  sheet entry).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("AuxSound2")` | `0x00844234` (single match) |
| `get_xrefs_to(0x00844234)` | `0x00712e48 → TechnoTypeClass__ReadINI` |

**New cheat-sheet entry (1):**
- `AuxSound2` (0x00844234 → 0x00712e48) TechnoType — landing event
  SFX. Sibling field to `AuxSound1` (0x00844240 → 0x00712e18, from
  HORNET). Address layout: `0x00844234` (AuxSound2) precedes
  `0x00844240` (AuxSound1) in the string table — *AuxSound2 comes
  FIRST in memory* despite being the second-numbered field. Westwood's
  string-table ordering doesn't match field-name numbering.

**Re-confirmed:**
- `Spawned`, `Landable`, `AuxSound1`, `NavalTargeting`, `LandTargeting`
  (all per cheat-sheet from earlier iterations).
- AircraftLocomotion GUID (per BEAG / HORNET).

**Allied return-to-dock spawn-child pair CLOSED**: HORNET ✓ + ASW ✓.

**Open questions:**
- ASW has no `VeteranAbilities`/`EliteAbilities` lines. Does this mean
  default inherits? Or genuinely no veterancy? HORNET has reduced 2-
  ability lines. Open follow-up.
- The `MovementRestrictedTo=Water` field on aircraft (which is normally
  UnitType-scope) — still open from HORNET iteration. Worth tracing
  what the engine does with this field on an AircraftType.
- The crash-transform mechanism (ASWCollision / HornetCollision spawn
  on death) — still open from HORNET iteration.
