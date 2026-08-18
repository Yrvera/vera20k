---
name: cargoplane-doc
description: CARGOPLANE — Transport Plane. Image=PDPLANE redirect (shares voxel
  with Paradrop Plane). NOT Spawned=yes (key diff from PDPLANE) — engine-direct
  reinforcement aircraft for AI campaign delivery scripts. No Primary weapon, no
  Ammo, Landable=no. Universal Owner. Category=AirPower (vs PDPLANE's AirLift).
metadata:
  type: project
---

# CARGOPLANE — Transport Plane

**INI ID:** `CARGOPLANE`
**Display:** "Transport Plane" (`UIName=Name:PDPLANE` — **CSF label shared
with PDPLANE**, so tooltips/UI may show "Cargo Plane" instead. Same dev-
shortcut as CMISL→DMISL CSF sharing).
**Section:** `[AircraftTypes]` (slot 9 of 12).
**Owner side:** **Generic — all 10 country slots** (same as PDPLANE).
Faction-universal.
**Role:** Engine-direct cargo/reinforcement delivery aircraft. **Key
distinction from PDPLANE**: CARGOPLANE *lacks `Spawned=yes`* — it's not a
SpawnManager child. Used by hardcoded engine reinforcement code (AI
campaign scripts, possibly Tech Airport free-unit delivery) that spawns
aircraft directly without going through the spawner pipeline. Shares
`Image=PDPLANE` art — same voxel, different role.

---

## Note on Ghidra unavailability

Ghidra MCP server remains offline. All field-scope claims cross-reference
prior verified cheat-sheet entries. No new ReadINI scope verification
this iteration.

---

## Rulesmd verbatim

```ini
[CARGOPLANE]
Image=PDPLANE
UIName=Name:PDPLANE
Name=Transport Plane
Strength=400
Category=AirPower
Armor=light
TechLevel=-1
LeadershipRating=10
;Selectable=no
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

### Key-by-key annotation — diff vs PDPLANE

Most fields are *bit-for-bit identical* to PDPLANE. This section covers
the **6 differences**.

**Diff vs PDPLANE:**
| Field | CARGOPLANE | PDPLANE |
|-------|-----------|---------|
| `Image=` | **`PDPLANE` (active redirect)** | (not set, uses own art) |
| `Name=` | "Transport Plane" | "Cargo Plane" |
| `Category=` | **`AirPower`** | `AirLift` |
| `Spawned=` | **(not set)** | `yes` |
| `Selectable=no` | **commented `;Selectable=no`** | active |
| `Primary=` | **(not set)** | `ParaDropWeapon` (dummy) |
| `Ammo=` | **(not set)** | `100` |

The other ~28 fields match PDPLANE exactly (Strength=400, Armor=light,
Locomotor, voice/sound, etc.). See [PDPLANE.md](./PDPLANE.md) for the
shared-field annotation.

**Key implications of the differences:**

1. **`Image=PDPLANE` (active)**: CARGOPLANE uses PDPLANE's voxel
   asset (`pdplane.vxl` + `.hva`). The two planes are visually
   identical in-game — same model, same color. Westwood reused the
   single plane voxel for two roles.

2. **`Category=AirPower` (vs PDPLANE AirLift)**: This places
   CARGOPLANE in the *combat-air* sidebar/AI bucket instead of
   *transport-air*. **Mechanical impact**: AI threat scoring,
   target priority weighting, and audit lists may treat CARGOPLANE
   as a combat threat. Unusual — the plane has no weapons. Possibly
   a Westwood oversight (the category should logically be AirLift
   like PDPLANE).

3. **No `Spawned=yes`**: **Critical difference**. PDPLANE is a
   SpawnManager child triggered by ParaDropSpecial superweapons.
   CARGOPLANE is *not* a spawn-child — there's no SpawnManager
   queuing it. The engine spawns CARGOPLANE directly via *hardcoded
   reinforcement code* (campaign AI scripts, free-unit delivery
   timer events, etc.). **The mechanism is NOT INI-driven**.

4. **`;Selectable=no` commented**: PDPLANE has it active (works
   because Landable=no). CARGOPLANE has it *commented out* —
   meaning **CARGOPLANE is technically Selectable=yes in shipped YR**.
   This is unusual since the plane has no Voice slots (would yield
   silent click). Possibly an oversight or intentional debug-
   accessibility.

5. **No `Primary=`**: CARGOPLANE has no weapon at all (not even a
   dummy). For an Aircraft-class unit this would normally fail
   validation — but the engine may have a separate code path for
   "transport-only" aircraft that don't require Primary=.
   Alternatively: the validator only checks for `Spawned=yes`
   aircraft (which need dummy weapons), and non-Spawned aircraft
   bypass the check.

6. **No `Ammo=`**: Without a Primary weapon, Ammo is moot. The
   `Ammo=100` on PDPLANE was a placeholder for the dummy weapon's
   firing system. CARGOPLANE just doesn't engage.

### Shared fields (cross-reference to PDPLANE)

- `Strength=400`, `Armor=light`, `MaxDebris=2`.
- `Sight=0`, `Landable=no`, `MoveToShroud=yes`.
- `PitchAngle=0` (flat flight — paradrop drop requirement).
- `Speed=15`, `ROT=2`.
- `Locomotor=AircraftLocomotion ({4A582746-...})`.
- All Voice* slots empty.
- `MoveSound=PDPlaneMoveLoop`, `CrashingSound=PDPlaneDie`,
  `ImpactLandSound=GenAircraftCrash`.
- `ImmuneToPsionics=yes`, `CanPassiveAquire=no`, `CanRetaliate=no`.
- `LeadershipRating=10`.
- `ThreatPosed=0`.
- Universal Owner list.
- `Crewed=yes`.

---

## Artmd verbatim — via `Image=PDPLANE` redirect

```ini
[PDPLANE] ; Paradrop Plane
Cameo=OBMBICON
Voxel=yes
PrimaryFireFLH=0,32,0
DisableVoxelCache=yes	; HY
DisableShadowCache=yes	; HY
```

CARGOPLANE has **no separate artmd block** — the `Image=PDPLANE`
redirect causes art lookup to use `[PDPLANE]`. Both planes share:
- Same voxel asset (`pdplane.vxl` + `.hva`).
- Same cameo (`OBMBICON`).
- Same FLH (0, 32, 0) — vestigial for CARGOPLANE since no weapon.
- Same `DisableVoxelCache=yes ; HY` performance flag.
- Same `DisableShadowCache=yes ; HY`.

**No CARGOPLANE-specific cameo**: when CARGOPLANE appears in the
in-game UI (rare), it shares PDPLANE's `OBMBICON`.

---

## Weapons

**CARGOPLANE has NO weapons defined**. Unlike PDPLANE which has the
dummy `Primary=ParaDropWeapon`, CARGOPLANE doesn't even bother with a
dummy. **Implications**:

- The engine must have a code path that allows Aircraft-class units
  without Primary= (or treats missing Primary as "transport-only").
- The plane will never fire under any circumstances.
- `CanPassiveAquire=no` + `CanRetaliate=no` + no Primary makes the
  plane completely inert to threat scoring.

**Open question**: does the engine actually load CARGOPLANE correctly
without a Primary=? If the Aircraft validator requires Primary, the
plane might silently fail to spawn (which would explain its
apparently-unused status). Open Ghidra trace.

---

## Voices / sounds

Identical to PDPLANE:
- All Voice* slots empty.
- `MoveSound=PDPlaneMoveLoop` — engine drone loop (3-sample
  `sparlo2*` random pool, Range=20, Volume=35 quiet ambient).
- `DieSound=` empty.
- `CrashingSound=PDPlaneDie` — crash plummet SFX (single sample
  `sparlo3`).
- `ImpactLandSound=GenAircraftCrash` — generic impact.

**No AuxSound1/AuxSound2** — same as PDPLANE. Drop-and-exit / direct-
spawn aircraft don't have takeoff/landing events.

See [PDPLANE.md](./PDPLANE.md#voices--sounds) for full block details.

---

## Hardcoded behavior

### 1. Engine-direct spawn (no SpawnManager involvement)

CARGOPLANE's lack of `Spawned=yes` means it's NOT routed through the
SpawnManager system. Possible engine code paths that spawn it:
- **AI Reinforcement Scripts** — campaign/mission TaskForce scripts
  can directly create aircraft at map edges as part of mission
  triggers.
- **Tech Airport (CAAIRP) free-unit delivery**? — but CAAIRP has
  `SuperWeapon=ParaDropSpecial` which uses PDPLANE, not CARGOPLANE.
  So CAAIRP probably *doesn't* use CARGOPLANE.
- **Hardcoded campaign cutscene/cinematic cargo delivery** — story
  scenes where a transport plane is shown.
- **Cargo crate / supply drop events** — engine-driven random
  cargo-pickup events in some scenarios.

The most likely use: **scripted AI campaign reinforcements**. The
engine creates `CARGOPLANE` instances when a script's trigger fires,
flies them to a designated cell, despawns them. The plane itself
delivers nothing — the script's *next* trigger spawns the actual
cargo unit at the same cell.

**Open question**: confirm via Ghidra trace which engine function(s)
spawn CARGOPLANE. Likely candidates include
`HouseClass::Create_Aircraft` or `TaskForce::Spawn`.

### 2. Image=PDPLANE redirect (art reuse pattern)

The `Image=PDPLANE` redirect in CARGOPLANE's rulesmd causes the
engine's art-asset loader to use PDPLANE's `[PDPLANE]` artmd block
instead of looking for `[CARGOPLANE]` in artmd.

This is the same redirect pattern used by:
- MTNK rulesmd → artmd [GTNK]
- APOC rulesmd → artmd [MTNK]
- MGTK rulesmd → artmd [RTNK]
- DTRUCK rulesmd → artmd [TRUCKA]
- CMISL rulesmd → artmd [BSUBMISL]
- CARGOPLANE rulesmd → artmd [PDPLANE]

**Westwood's pattern**: when two units share visual identity, use the
Image= redirect to avoid duplicating art blocks.

### 3. Category=AirPower oddity

Both PDPLANE and CARGOPLANE are passive transport aircraft, but they
have *different categories*:
- PDPLANE: `Category=AirLift` — transport-air bucket.
- CARGOPLANE: `Category=AirPower` — combat-air bucket.

**Plausible Westwood reasoning**:
- AirLift is the "true transport" category (passive, no combat).
- AirPower is "combat" (Kirov, Hornet, BEAG).

CARGOPLANE being AirPower despite having no weapons is *anomalous*.
Possibilities:
1. Westwood typo / oversight (should have been AirLift).
2. Intentional — placing CARGOPLANE in AirPower makes AI prioritize
   shooting it down (better than ignoring an inert plane that might
   be carrying reinforcements).
3. Different AI behavior expected for AirPower category vs AirLift
   (priority targeting, threat evaluation, etc.).

Without Ghidra trace of how Category= affects AI behavior, the
distinction's impact is unclear.

### 4. Empty Primary= bypasses Aircraft validation?

PDPLANE has `Primary=ParaDropWeapon` (dummy) — required to satisfy
the Aircraft-class fire-system validator. CARGOPLANE has no Primary
at all. Possible explanations:
1. **Aircraft validator has a "transport mode" path** that skips the
   Primary check for certain configurations.
2. **CARGOPLANE silently fails to spawn** when actually triggered
   (which would explain the lack of in-game observation).
3. **Engine treats missing Primary as "use a default no-op weapon"**
   internally.

Open Ghidra trace required to resolve.

### 5. Universal Owner list

Same as PDPLANE — all 10 country slots. The plane works for any
faction's reinforcements / campaign script.

### 6. `Selectable=no` left commented

Unlike PDPLANE which has `Selectable=no` active, CARGOPLANE has it
commented. The plane is theoretically clickable in-game (though
players rarely see it long enough to click). **Possibly a debug
convenience** — Westwood devs may have wanted CARGOPLANE selectable
to test scripted reinforcements during development.

---

## TS-legacy filter

- `;Selectable=no` — commented historical.
- `Image=PDPLANE` — active redirect, not TS-legacy.
- `;default is 20 degrees` — verbatim engine default comment (same as
  PDPLANE).
- No `ImmuneToVeins`, no `Subterranean`. YR-active mechanism (if
  used at all).

---

## Comparison: CARGOPLANE vs PDPLANE (the cargo-plane pair)

| Field | CARGOPLANE | PDPLANE |
|-------|-----------|---------|
| Display CSF | `Name:PDPLANE` (shared) | `Name:PDPLANE` (shared) |
| Internal Name | "Transport Plane" | "Cargo Plane" |
| Category | **AirPower** | AirLift |
| Spawned | **(not set)** | yes |
| Spawn paradigm | engine-direct (script/AI) | spawn-child (SpawnManager) |
| Primary weapon | **(none)** | ParaDropWeapon (dummy) |
| Ammo | (not set) | 100 |
| Selectable | (commented) | no (active) |
| Art | PDPLANE redirect | own block |
| Strength | 400 | 400 |
| Locomotor | Aircraft | Aircraft |
| Owner | universal (10) | universal (10) |
| AuxSound1/2 | (none) | (none) |

**The pair are visually identical** (same voxel) but **mechanically
distinct** in spawn pathway:
- PDPLANE: declared spawn-child, triggered by ParaDropSpecial
  superweapon, drops infantry mid-flight per country tables.
- CARGOPLANE: engine-direct spawn, triggered by hardcoded script /
  AI campaign reinforcement code, delivers... something (unclear in
  shipped YR; possibly never reached in standard skirmish play).

**Possibly the most "vestigial" aircraft documented so far** —
CARGOPLANE has a complete rulesmd entry, complete artmd setup via
redirect, listed at slot 9 of AircraftTypes, but no observable usage
path in standard YR gameplay. Open follow-up to find concrete
trigger.

---

## Cross-references

- [PDPLANE.md](./PDPLANE.md) — pair partner. Drop-and-exit paradigm.
  Spawn-child via superweapon.
- [HORNET.md](../allied/HORNET.md) + [ASW.md](../allied/ASW.md) —
  return-to-dock paradigm peers.
- [V3ROCKET.md](../soviet/V3ROCKET.md) + [DMISL.md](../soviet/DMISL.md)
  + [CMISL.md](../yuri/CMISL.md) — kamikaze paradigm peers.
- [CAAIRP] (Tech Airport) — uses ParaDropSpecial → PDPLANE, NOT
  CARGOPLANE.
- [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
  — SpawnManager (CARGOPLANE bypasses this).

---

## Coverage audit

- [x] Every rulesmd key annotated (~30 keys, diff vs PDPLANE).
- [x] Every artmd reference annotated (via Image=PDPLANE redirect).
- [x] **No weapons** documented (NO Primary= at all, unlike PDPLANE's
  dummy).
- [x] All voice/sound bindings documented (shared with PDPLANE).
- [x] Spawn-child status: **NOT Spawned=yes** — engine-direct
  reinforcement, not SpawnManager-managed.
- [x] Hardcoded behavior: 4th spawn pathway distinction (engine-
  direct vs SpawnManager), Image=PDPLANE art redirect, Category=
  AirPower oddity, missing-Primary mystery, universal Owner.
- [x] TS-legacy filter applied (no active TS-only fields).
- [x] Comparison table closes the cargo-plane pair (CARGOPLANE +
  PDPLANE).
- [ ] **No Ghidra verification this iteration** (MCP server offline).

**Ghidra status**: MCP server still disconnected. No new cheat-sheet
entries. All field-scope claims cross-reference prior verified
entries from PDPLANE iteration.

**Re-confirmed cheat-sheet:**
- AircraftLocomotion GUID (per BEAG/HORNET/PDPLANE).
- All shared fields with PDPLANE.

**Open questions:**
- What engine code path spawns CARGOPLANE? Most likely AI
  reinforcement scripts (campaign TaskForce triggers); confirm via
  Ghidra. Search for "CARGOPLANE" string references should reveal
  the consumer functions when MCP returns.
- Does the Aircraft validator accept CARGOPLANE without Primary=?
  Or is CARGOPLANE silently broken in shipped YR?
- Why `Category=AirPower` instead of `AirLift`? Westwood oversight,
  AI tuning, or different validation path?
- Is CARGOPLANE ever observable in standard skirmish gameplay? Or
  is it purely campaign/cinematic content?
- The `[AircraftTypes]` slot 9 enumeration confirms 12 aircraft total
  in YR — current iteration has documented 7 (V3ROCKET, DMISL, HORNET,
  ASW, BEAG, PDPLANE, CMISL); 5 remain (APACHE [cut], ORCA [cut],
  CARGOPLANE [this iteration], BPLN [pending], SPYP [pending]).

**Cargo-plane pair CLOSED**: PDPLANE ✓ + CARGOPLANE ✓.
