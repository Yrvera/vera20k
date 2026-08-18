---
name: spyp-doc
description: SPYP — Soviet Spy Plane. Reveal-shroud superweapon-spawned aircraft.
  Primary=SpyCameraWeapon (Damage=6 = shroud-reveal radius, not damage); FlyBy=true
  (NEW field — "Don't slow down over your target"); DeathWeapon=BlimpBomb + .1
  modifier ("needs a death weapon or it will do nothing when it crashes since its
  weapon is a camera"). Provided by NARADR Soviet Radar Tower (not GASPYSAT).
metadata:
  type: project
---

# SPYP — Soviet Spy Plane

**INI ID:** `SPYP`
**Display:** "Soviet Spy Plane" (`UIName=Name:SpyP`)
**Section:** `[AircraftTypes]` (slot 11 of 12 per `[AircraftTypes]` list).
**Owner side:** **Generic — all 10 country slots**, but the providing
superweapon `SpyPlaneSpecial` is on **NARADR (Soviet Radar Tower) only**.
Other factions (Allied, Yuri) do not have access to the spy plane via
their radar buildings.
**Role:** Soviet faction's reveal-shroud aircraft. Spawned by the
NARADR-attached `SpyPlaneSpecial` superweapon. Flies in a straight line
over the clicked target, uncovering shroud cells along its path using a
`SpyCameraWeapon` whose "Damage" value is actually the **reveal radius
in cells**. Distinct from PDPLANE (paradrop) and CARGOPLANE (transport)
in mission profile but shares the same engine-direct/spawn-child pattern.

---

## Allied vs Soviet asymmetry

- **Soviet**: NARADR (Radar Tower) provides `SpyPlaneSpecial` →
  RechargeTime=4 min recharging spy plane.
- **Allied**: GASPYSAT (Spy Satellite Uplink) has `SpySat=yes` flag
  → permanent map reveal while building is operational, no
  recharging needed.
- **Yuri**: no equivalent (Psychic Sensor reveals attack approaches,
  not general shroud).

**Reveal-mechanic split**:
- Soviet: temporary, recharging, *uses the SPYP aircraft*.
- Allied: permanent, building-based, *no aircraft involved* (the
  GASPYSAT name "Spy Satellite Uplink" is thematic — engine just
  flips the map-reveal flag).

This is one of the better-documented faction asymmetries in YR.

---

## Note on Ghidra unavailability

Ghidra MCP server remains offline. All field-scope claims cross-
reference prior cheat-sheet entries. No new ReadINI scope verification
this iteration.

---

## Rulesmd verbatim

```ini
[SPYP]
UIName=Name:SpyP
Name=Soviet Spy Plane
;Image=PDPLANE
Strength=600
Category=AirLift
Armor=light
TechLevel=-1
Primary=SpyCameraWeapon
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
MoveSound=SpyPlaneMoveLoop
DieSound=
CrashingSound=SpyPlaneDie
ImpactLandSound=GenAircraftCrash
DeathWeapon=BlimpBomb
DeathWeaponDamageModifier=.1;gs needs a death weapon or it will do nothing when it crashes since its weapon is a camera
FlyBy=true	;GEF Don't slow down over your target
```

### Key-by-key annotation — diff vs PDPLANE

Most fields match PDPLANE's transport-plane template. This section
covers SPYP-distinctive differences.

**Diff vs PDPLANE:**
| Field | SPYP | PDPLANE |
|-------|------|---------|
| `Name` | "Soviet Spy Plane" | "Cargo Plane" |
| `Image=` | **commented `;Image=PDPLANE`** | (not set, own art) |
| `Strength` | **600** | 400 |
| `Primary` | **`SpyCameraWeapon`** (not dummy) | ParaDropWeapon (dummy) |
| `DeathWeapon` | **`BlimpBomb`** with `.1` modifier | (not set) |
| `FlyBy` | **`true`** (NEW field) | (not set) |
| `MoveSound` | SpyPlaneMoveLoop | PDPlaneMoveLoop |
| `CrashingSound` | SpyPlaneDie | PDPlaneDie |

**Key distinctions:**

1. **`;Image=PDPLANE` commented**: SPYP was originally going to reuse
   PDPLANE art via the same redirect pattern as CARGOPLANE. Westwood
   gave it its own dedicated voxel during development. The commented
   line is harmless historical.

2. **Strength=600** (vs PDPLANE's 400): SPYP is more durable.
   Reveal-shroud is a longer mission (must overfly target for full
   reveal), so the plane needs to survive AA fire longer.

3. **`Primary=SpyCameraWeapon` (active, not dummy)**: Unlike PDPLANE's
   dummy weapon, SPYP's Primary is *functional* — it's the actual
   shroud-reveal mechanic. See SpyCameraWeapon block below.

4. **`DeathWeapon=BlimpBomb` + `DeathWeaponDamageModifier=.1`**:
   **The verbatim comment is critical**: *"needs a death weapon or
   it will do nothing when it crashes since its weapon is a camera"*.
   Because SpyCameraWeapon does no damage (Damage=6 is a *reveal
   radius*, not damage), without a DeathWeapon the crashing plane
   would impact harmlessly. Adding BlimpBomb with `.1` modifier
   gives a small explosion on crash (BlimpBomb base = 250 damage ×
   0.1 = 25 damage). **Same pattern as DISK Floating Disc** (which
   uses the *same* `DeathWeapon=BlimpBombEffect` + `.1` modifier
   for the same reason). Cross-faction reuse of the same engine
   trick.
   - Per the DISK doc earlier: BlimpBombEffect's section comment
     reads "To make crashing guys use a big blimp bomb explosion,
     but not be forced to do a lot of damage to get the effect".
   - SPYP uses `BlimpBomb` (the basic 250-damage Kirov bomb) ×.1.
     DISK uses `BlimpBombEffect` (the effect-only variant) ×.1.
     Slightly different choice but same mechanic.

5. **`FlyBy=true`** with verbatim comment **"GEF Don't slow down over
   your target"**. **NEW field not yet in cheat-sheet** (cannot
   verify scope this iteration). The `FlyBy=true` flag controls
   aircraft behavior at target — by default, aircraft slow down or
   loiter when reaching their target; `FlyBy=true` forces a *straight
   pass-through* without slowing. **Critical for reveal mechanic**:
   the plane sweeps over a wide area in a single straight line,
   revealing shroud cells along its entire flight path. If it slowed
   down at the click target, the reveal would be concentrated at one
   point instead of spreading. Possibly TechnoType or AircraftType
   scope — Ghidra trace needed.

### Otherwise identical to PDPLANE (~25 fields)

The shared fields (Locomotor, Voice empty, Owner, Category=AirLift,
Spawned=yes, etc.) all match PDPLANE. See [PDPLANE.md](../civilian/PDPLANE.md)
for the shared-field annotations.

**Note**: SPYP has `Category=AirLift` (matching PDPLANE), whereas
CARGOPLANE has `Category=AirPower` — strengthens the hypothesis that
CARGOPLANE's AirPower is a Westwood typo/oversight, since SPYP (a
similar passive transport-style plane) correctly uses AirLift.

---

## Artmd verbatim

```ini
[SPYP] ; Soviet Spy Plane
Cameo=SPYPICON
Voxel=yes
ShadowIndex=3 ;draw plane body, not propellers
DisableVoxelCache=yes ; SJM: this is a major cache hog
DisableShadowCache=yes ; SJM: this too
```

### Key-by-key annotation

- `Cameo=SPYPICON` — dedicated sidebar cameo (unlike CARGOPLANE which
  shares OBMBICON via PDPLANE redirect).
- `Voxel=yes` — rendered from `spyp.vxl` + `.hva`.
- `ShadowIndex=3` — **verbatim comment "draw plane body, not propellers"**.
  Voxel-stack layer 3 is selected for shadow rendering. Layers 0-2
  are likely propeller animation frames (which would create
  unrealistic spinning shadows); layer 3 is the static plane body
  for a clean shadow silhouette. **Same flag concept as SHAD's
  `ShadowIndex=2`** ("order of voxels got changed") — different
  layer choice per unit.
- `DisableVoxelCache=yes ; SJM` — SJM developer initials. SHAD has
  the same flag with SJM; PDPLANE has it with HY. **SJM flagged
  SPYP's voxel as a cache hog**, same as SHAD.
- `DisableShadowCache=yes ; SJM` — same.

**No `PrimaryFireFLH=`** — the SpyCameraWeapon doesn't fire a
projectile in the visual sense (it just reveals cells); no FLH needed.

**No `AltCameo=`** — single cameo.

---

## Weapons

### Active "weapon" — `[SpyCameraWeapon]`

```ini
[SpyCameraWeapon]
Damage=6;range of shroud to reveal
Range=20;howfar away to start revealing
Burst=1
Projectile=InvisibleHigh
Warhead=DummyWarhead
Report=SpyPlaneSnapshot
```

**This weapon's fields have non-standard semantics** — they don't
represent literal damage:

- **`Damage=6`** — verbatim comment "range of shroud to reveal". The
  Damage field is *re-purposed* as the shroud-reveal radius in cells.
  6 cells around the plane's current position are uncovered as it
  passes.
- **`Range=20`** — verbatim "howfar away to start revealing". *Reveal
  trigger distance* in cells — the plane starts revealing when it's
  within 20 cells of the target. This is a *much larger range* than
  any combat weapon.
- `Burst=1` — single "fire" per detection (continuous reveal).
- `Projectile=InvisibleHigh` — invisible projectile (no visible bullet).
- `Warhead=DummyWarhead` — *literal dummy warhead* (no damage
  application).
- `Report=SpyPlaneSnapshot` — fire SFX. *Plays the camera-snapshot
  sound effect* each time the reveal fires. Audio feedback for the
  player that their spy plane is actively revealing.

**Mechanism**:
1. SPYP spawns at map edge after SpyPlaneSpecial is triggered.
2. Plane flies straight toward the click target.
3. When within Range=20 cells of target, the SpyCameraWeapon "fires"
   — engine queries Damage=6 as the reveal radius, uncovers shroud
   cells in a 6-cell circle around the plane's current position.
4. Plays SpyPlaneSnapshot SFX.
5. FlyBy=true keeps the plane moving (no slowdown at target).
6. Plane exits map edge, despawns.

The reveal covers a *swath* (the plane's flight path) plus a 6-cell-
radius circle at the click point. Players see a long stripe of
revealed shroud, with a thicker disc at the destination.

### Hardcoded warhead — DummyWarhead

DummyWarhead is a Rules-level placeholder warhead used for non-damage
weapons. Likely has all-zero Verses and no anim — exists purely to
satisfy the engine's "every weapon must have a warhead" validation
without actually applying damage.

### Death weapon — `[BlimpBomb]` (×0.1 modifier)

When SPYP is shot down or crashes, the BlimpBomb warhead detonates
at the impact cell with `0.1` damage modifier:
- BlimpBomb base Damage=250 (Kirov bomb).
- With `DeathWeaponDamageModifier=.1` → effective damage 25.

See [ZEP.md](./ZEP.md#weapons) (Kirov doc) for the full BlimpBomb
weapon block (anti-structure, 250 damage, big AoE warhead BlimpHE).
The .1 modifier just scales the resulting damage.

**Why a death weapon?** Verbatim Westwood comment: *"needs a death
weapon or it will do nothing when it crashes since its weapon is a
camera"*. The SpyCameraWeapon doesn't damage, so a crashing SPYP
without DeathWeapon would just disappear. The DeathWeapon ensures a
small visual+damage event on impact for player feedback.

---

## Voices / sounds

```ini
[SpyPlaneMoveLoop]
Sounds=vspylo2a vspylo2b vspylo2c
Control=random loop all attack
Limit=3
Priority=high
Range=10
Volume=60

[SpyPlaneDie]
Sounds=vspylo3a
Range=20
Volume=45
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| (all Voice* empty) | n/a | No player-interaction voices |
| `MoveSound=SpyPlaneMoveLoop` | `[SpyPlaneMoveLoop]` | Engine loop during flight (3-sample random-loop, Volume=60, Range=10 close-range audibility) |
| `DieSound=` (empty) | n/a | No instant-death SFX |
| `CrashingSound=SpyPlaneDie` | `[SpyPlaneDie]` | Crash plummet SFX (single sample `vspylo3a`) |
| `ImpactLandSound=GenAircraftCrash` | shared | Generic impact |
| `Report=SpyPlaneSnapshot` (SpyCameraWeapon) | `[SpyPlaneSnapshot]` | Camera-shutter SFX during reveal |

**Note**: `[SpyPlaneSnapshot]` block exists in soundmd (separate
from the listed entries above; not retrieved in this iteration). The
SFX would be a *camera-shutter click* matching the spy-camera theme.

**`vspylo*` audio prefix**: shared with the BEAG's `vintlo*` (similar
historical naming convention) — though BEAG uses `vintlo` ("intercept-
loop") and SPYP uses `vspylo` ("spy-loop"). Different prefix per role.

---

## Hardcoded behavior

### 1. FlyBy=true (NEW field, scope unverified)

The `FlyBy=true` field is the **defining flag** for the spy-plane
mission profile. Verbatim Westwood comment: *"GEF Don't slow down
over your target"*.

Default aircraft behavior:
- Aircraft fly toward target.
- Aircraft slow down / loiter when reaching target (allows multiple
  attacks).
- Aircraft circle / re-engage if target survives.

With `FlyBy=true`:
- Aircraft flies straight through target without slowing.
- Single pass; no loiter.
- Aircraft continues on straight-line path until map edge.

**Critical for reveal mechanic** — the reveal-swath effect depends on
the plane covering a long flight path. Slowing/circling would
concentrate the reveal in one spot.

**Open Ghidra verification**: scope likely TechnoType (AircraftType-
specific behavior would also make sense). Field name suggests it
could be on either.

### 2. SpyCameraWeapon repurposed damage/range fields

The `Damage=6 ; range of shroud to reveal` is a *fundamental Westwood
trick*: repurposing a standard weapon field for a custom non-damage
purpose. The engine's weapon-fire code:
1. Reads `Damage` value.
2. Normally applies it as HP-loss to target.
3. For SpyCameraWeapon (identified somehow — perhaps via
   `Warhead=DummyWarhead` or hardcoded weapon-name check), instead
   passes the Damage value to the shroud-reveal subsystem as the
   reveal-radius.

Similarly:
- `Range=20` (normally weapon engagement range) becomes "start
  revealing when within 20 cells".
- `Burst=1` (normally shots per fire) is moot.

**Open question**: how does the engine identify SpyCameraWeapon for
the repurposed behavior? Possibly:
- Hardcoded weapon-name check on `SpyCameraWeapon`.
- Hardcoded warhead-name check on `DummyWarhead`.
- Some IsCamera or similar flag (not visible in INI).

Open Ghidra trace required.

### 3. DeathWeapon with .1 modifier (cross-faction shared trick)

The exact pattern used by DISK (`DeathWeapon=BlimpBombEffect` + `.1`)
and SPYP (`DeathWeapon=BlimpBomb` + `.1`) — *"the unit needs a
death weapon for the crash to register, but we don't want big
damage"*. The 10% damage modifier gives just enough impact to make
the crash visible.

Other units with this pattern:
- DISK (Yuri Floating Disc).
- SPYP (Soviet Spy Plane).
- CASANF01 (civilian San Fran Victorian Home — line 14901 verbatim
  comment "needs to be explodes=yes to redraw when killed (late bug
  fix)").

The trick exists because Westwood's crash code requires *some* damage
event to trigger the crash-effects pipeline. Pure-camera/pure-effect
units would otherwise crash silently.

### 4. SuperWeapon=SpyPlaneSpecial provided by NARADR only

Soviet Radar Tower NARADR has `SuperWeapon=SpyPlaneSpecial`. Allied
GASPYSAT has `SpySat=yes` (permanent reveal). Yuri has no equivalent.
**Faction asymmetry confirmed**.

### 5. Spawned=yes + Landable=no (drop-and-exit paradigm)

Same as PDPLANE — SPYP is a SpawnManager child (Spawned=yes) but
doesn't land (Landable=no). Single sortie, exits map edge. **Third
member of drop-and-exit paradigm** (PDPLANE, SPYP, ?CARGOPLANE).

Wait — CARGOPLANE has NO Spawned=yes (engine-direct, not SpawnManager).
So the *strict* drop-and-exit paradigm via SpawnManager has 2 members
so far: PDPLANE and SPYP. CARGOPLANE is the engine-direct variant.

### 6. Universal Owner list

Same as PDPLANE/CARGOPLANE — all 10 country slots. SPYP can serve any
faction's reveal mission, but in practice only Soviets get the
superweapon to trigger it.

### 7. ShadowIndex=3 (per-unit voxel-slice shadow)

The art-block `ShadowIndex=3` selects voxel-stack layer 3 for the
shadow silhouette. **Verbatim "draw plane body, not propellers"**
— layers 0-2 are propeller animation frames (rotating), layer 3 is
the static plane body. Without this, the shadow would show the
propellers spinning (incorrect — propellers should be motion-blurred
not crisply shadowed).

Compare with SHAD's `ShadowIndex=2` ("order of voxels got changed").
Both units use the field but with different slice indices reflecting
their voxel asset layouts.

---

## SpyPlaneSpecial superweapon

```ini
[SpyPlaneSpecial]
UIName=Name:SpyP
Name=SpyPlane Flyby
IsPowered=false
RechargeVoice=
ChargingVoice=
ImpatientVoice=
SuspendVoice=
RechargeTime=4
Type=SpyPlane
Action=SpyPlane
SidebarImage=SPYPICON
ShowTimer=no
DisableableFromShell=no
FlashSidebarTabFrames=120; default is always, put 0 for never, or a number for x
```

- `IsPowered=false` — works even when player is low-power.
- `RechargeTime=4` — 4 minutes between uses.
- `Type=SpyPlane` / `Action=SpyPlane` — engine-side dispatcher action
  class. Same naming pattern as `Type=ParaDrop` / `Action=ParaDrop` on
  ParaDropSpecial. The Type/Action pair tells the engine which spawn
  routine to invoke when the player triggers it.
- `SidebarImage=SPYPICON` — sidebar button cameo.
- `ShowTimer=no` — timer hidden on sidebar.
- **`FlashSidebarTabFrames=120`** — *flash the sidebar tab for 120
  frames* when the superweapon is ready. Verbatim default comment:
  "default is always, put 0 for never, or a number for x".
  **NEW field not yet in cheat-sheet**.

**Provider**: NARADR (Soviet Radar Tower) at rulesmd line 12601-12631.
`SuperWeapon=SpyPlaneSpecial` line at 12630.

---

## TS-legacy filter

- `;Image=PDPLANE` — commented historical (would have reused PDPLANE
  art).
- `;default is 20 degrees` — verbatim engine default reference.
- No `ImmuneToVeins`, no `Subterranean`. YR-active mechanism.

---

## Comparison: SPYP vs other transport-paradigm aircraft

| Field | SPYP (Soviet) | PDPLANE (universal) | CARGOPLANE (universal) |
|-------|---------------|---------------------|--------------------------|
| Display | "Soviet Spy Plane" | "Cargo Plane" | "Transport Plane" |
| Category | AirLift | AirLift | **AirPower** (anomaly) |
| Strength | **600** | 400 | 400 |
| Spawned | yes | yes | **(no!)** |
| Spawn-paradigm | SpawnManager drop-and-exit | SpawnManager drop-and-exit | **Engine-direct** |
| Primary | **SpyCameraWeapon (active, repurposed)** | ParaDropWeapon (dummy) | **(none)** |
| FlyBy | **true** | (not set) | (not set) |
| DeathWeapon | **BlimpBomb ×.1** | (not set) | (not set) |
| Cameo | SPYPICON (own) | OBMBICON | OBMBICON (shared via PDPLANE) |
| Art | own voxel | own voxel | PDPLANE redirect |
| MoveSound | SpyPlaneMoveLoop | PDPlaneMoveLoop | PDPlaneMoveLoop (shared) |
| Providing superweapon | SpyPlaneSpecial (NARADR) | ParaDropSpecial / AmericanParaDropSpecial | (engine-direct, no SW) |
| Faction asymmetry | **Soviet-only via NARADR** | universal (CAAIRP + American Airforce Command) | universal (script-driven) |

**Key trio observations**:
- **SPYP is the most "active" of the transport-paradigm planes** —
  has a real weapon (SpyCameraWeapon), DeathWeapon, and unique
  FlyBy=true behavior.
- **SPYP is the only faction-restricted one** — superweapon provider
  is Soviet-exclusive. PDPLANE/CARGOPLANE serve any faction.
- **SPYP has dedicated audio identity** — its own MoveSound and
  CrashingSound (vspylo prefix), vs PDPLANE/CARGOPLANE sharing the
  sparlo prefix.

---

## Cross-references

- [PDPLANE.md](../civilian/PDPLANE.md) — peer drop-and-exit aircraft
  (paradrop mission instead of reveal).
- [CARGOPLANE.md](../civilian/CARGOPLANE.md) — peer transport-style
  aircraft (engine-direct spawn).
- [DISK.md](../yuri/DISK.md) — peer unit with the DeathWeapon ×.1
  modifier trick.
- [ZEP.md](./ZEP.md) — Kirov Airship; uses BlimpBomb as its primary
  weapon (which SPYP repurposes as its DeathWeapon).
- [NARADR.md](../structures/NARADR.md) — pending. Soviet Radar Tower,
  provides SpyPlaneSpecial.
- [GASPYSAT.md](../structures/GASPYSAT.md) — pending. Allied Spy
  Satellite Uplink (alternative reveal mechanic via SpySat=yes
  flag, no SPYP).

---

## Coverage audit

- [x] Every rulesmd key annotated (~35 keys, diff vs PDPLANE).
- [x] Every artmd key annotated (5 keys including ShadowIndex=3
  rationale).
- [x] **Active Primary=SpyCameraWeapon** documented with repurposed
  Damage/Range field semantics.
- [x] DeathWeapon=BlimpBomb ×.1 modifier mechanism explained
  (Westwood verbatim comment).
- [x] FlyBy=true behavior characterized.
- [x] All voice/sound bindings documented including SpyPlaneSnapshot
  camera SFX.
- [x] Spawn-child status (Spawned=yes, drop-and-exit paradigm).
- [x] SpyPlaneSpecial superweapon block documented (RechargeTime=4
  min, FlashSidebarTabFrames=120, Type=SpyPlane action class).
- [x] Faction asymmetry: Soviet-only via NARADR vs Allied permanent
  via GASPYSAT vs Yuri none.
- [x] Hardcoded behavior: FlyBy mechanism, repurposed SpyCameraWeapon
  fields, DeathWeapon ×.1 modifier shared with DISK, ShadowIndex=3
  voxel-stack selection.
- [x] TS-legacy filter applied.
- [x] Comparison table with PDPLANE and CARGOPLANE.
- [ ] **No Ghidra verification this iteration** (MCP server offline).

**Ghidra status**: MCP server still disconnected. No new cheat-sheet
entries. Field-scope claims cross-reference prior verified entries.

**Re-confirmed cheat-sheet:**
- All shared fields with PDPLANE/CARGOPLANE.
- DeathWeapon + DeathWeaponDamageModifier (TechnoType, per cheat-sheet
  notes).

**Open questions:**
- `FlyBy` field scope — likely TechnoType or AircraftType. Ghidra
  trace pending.
- `FlashSidebarTabFrames` field scope — Rules-level superweapon
  declaration. Open.
- How does the engine identify `SpyCameraWeapon` for repurposed
  Damage/Range semantics? Hardcoded weapon-name check? DummyWarhead
  flag? Open Ghidra trace.
- `LeadershipRating` field scope (still not verified across
  multiple iterations).

**Drop-and-exit paradigm members (SpawnManager-based):**
- PDPLANE ✓
- SPYP ✓ (this iteration)
- (CARGOPLANE bypasses SpawnManager — engine-direct variant)

**Aircraft documented so far** (per `[AircraftTypes]` slot 1-12):
- Slot 1 APACHE — *cut* (commented out in rules and artmd).
- Slot 2 ORCA — likely cut (in priority queue).
- Slot 3 HORNET ✓
- Slot 4 V3ROCKET ✓
- Slot 5 ASW ✓
- Slot 6 DMISL ✓
- Slot 7 PDPLANE ✓
- Slot 8 BEAG ✓
- Slot 9 CARGOPLANE ✓
- Slot 10 BPLN — pending.
- Slot 11 SPYP ✓ (this iteration)
- Slot 12 CMISL ✓

**10 of 12 aircraft documented**. Remaining: ORCA (likely cut), BPLN
(B2 Spirit Bomber for AirstrikeTeamType — used by some superweapon).
