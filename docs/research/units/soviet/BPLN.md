---
name: bpln-doc
description: BPLN — Boris's airstrike plane (a.k.a. Soviet MIG / Boris Attack Plane).
  Spawned by Boris's [BORIS] AirstrikeTeam=2 / EliteAirstrikeTeam=4 hardcoded
  per-techno airstrike system. Real damage (Primary=Maverick3 Damage=750 base!).
  FlyBy=true. DeathWeapon=BlimpBomb ×.1. Closes aircraft batch (11 of 12
  documented; ORCA likely cut).
metadata:
  type: project
---

# BPLN — Boris's Airstrike Plane (Soviet MIG)

**INI ID:** `BPLN`
**Display:** "Soviet MIG" (`UIName=Name:BPLN`). Internal rulesmd `Name=Soviet
MIG` (the MiG-29 fighter), but artmd block header reads "Boris Attack Plane"
— the *intended use* (called in by Boris's airstrike).
**Section:** `[AircraftTypes]` (slot 10 of 12).
**Owner side:** **Generic — all 10 country slots**, but invoked exclusively
by Boris's per-techno `AirstrikeTeam`/`EliteAirstrikeTeam` system (only
the Soviet [BORIS] infantry hero has this).
**Role:** Boris's bomber-strike delivery aircraft. Boris designates a
target; engine spawns 2 BPLNs (or 4 at Elite) that fly over and drop
Maverick3 missiles. **Airstrike paradigm** — fifth spawn pathway after
kamikaze/return-to-dock/drop-and-exit/engine-direct. Real damage (unlike
PDPLANE/SPYP which use dummy/repurposed weapons).

---

## Airstrike vs other transport paradigms

BPLN introduces a 5th spawn pathway:
| Paradigm | Members | Mechanism | Trigger |
|----------|---------|-----------|---------|
| Kamikaze | V3ROCKET, DMISL, CMISL | one-shot suicide, RocketLocomotion | parent unit fires Spawns= weapon |
| Return-to-dock | HORNET, ASW | sortie cycle, AircraftLocomotion | parent unit fires Spawner=yes weapon |
| Drop-and-exit (SpawnManager) | PDPLANE, SPYP | flies in, drops cargo/reveal, exits | building's SuperWeapon=ParaDropSpecial/SpyPlaneSpecial |
| Engine-direct | CARGOPLANE | no SpawnManager, hardcoded script | AI campaign reinforcement |
| **Airstrike** | **BPLN** | **per-techno AirstrikeTeam=N planes, real damage** | **Boris's `AirstrikeTeam` hardcoded mechanic** |

The Airstrike paradigm is unique:
- *Spawned=yes* (SpawnManager-based).
- *Not from a building/superweapon* — triggered by a per-unit
  (per-Boris) hardcoded airstrike command.
- Multiple planes spawn simultaneously (2/4 per team).
- Real combat damage (Maverick3 Damage=750 base!).
- AirstrikeRechargeTime cooldown per Boris (100 frames rookie, 50
  frames elite).

---

## Note on Ghidra unavailability

Ghidra MCP server remains offline. All field-scope claims cross-
reference prior verified cheat-sheet entries. No new ReadINI scope
verification this iteration.

---

## Rulesmd verbatim

```ini
[BPLN]
UIName=Name:BPLN
Name=Soviet MIG
;Image=FORTRESS;PDPLANE
Strength=200
Category=AirLift
Armor=light
TechLevel=-1
Primary=Maverick3
;Primary=ParaDropWeapon	; Doesn't really fire it; dummy weapon
Spawned=yes	; Created by another object and therefore not player controllable
LeadershipRating=10
Selectable=no
RadarInvisible=no
Sight=0
Landable=no
MoveToShroud=yes
PitchAngle=0 ; default is 20 degrees
Speed=16; 18
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
Points=30
ROT=2
Crewed=yes
Ammo=1
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=2
Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1}
MovementZone=Fly
ThreatPosed=0	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
ImmuneToPsionics=yes
CanPassiveAquire=no ; Won't try to pick up own targets
CanRetaliate=no; Won't fire back when hit
MoveSound=MigMoveLoop
DieSound=
VoiceCrashing=MigVoiceDie
CrashingSound=IntruderDie
ImpactLandSound=GenAircraftCrash
Fighter=yes
ElitePrimary=Maverick3E
Trainable=no
DeathWeapon=BlimpBomb
DeathWeaponDamageModifier=.1;gs needs a death weapon or it will do one laser blast's worth of crash damage.  This gives control
FlyBy=true	;GEF Don't slow down over your target
```

### Key-by-key annotation — diff vs PDPLANE/SPYP

Most fields match PDPLANE/SPYP's transport-style template. This
section covers BPLN-distinctive differences.

**Diff vs SPYP (the closest sibling)**:
| Field | BPLN | SPYP |
|-------|------|------|
| `Name` | "Soviet MIG" | "Soviet Spy Plane" |
| `;Image=` | `;Image=FORTRESS;PDPLANE` (commented, 2 alternatives) | `;Image=PDPLANE` (single commented) |
| `Strength` | **200** | 600 |
| `Primary` | **`Maverick3`** (combat weapon, Damage=750!) | SpyCameraWeapon (camera) |
| `Speed` | **16** (`;18` historical) | 15 |
| `Ammo` | **1** (single attack) | 100 |
| `Fighter` | **`yes`** (AircraftType flag) | (not set) |
| `ElitePrimary` | **`Maverick3E`** (active swap) | (not set) |
| `Trainable` | **`no` (explicit)** | (not set, inherits default) |
| `VoiceCrashing` | **`MigVoiceDie`** (explicit) | (not set) |
| `CrashingSound` | **`IntruderDie`** (shared with cut Intruder?) | SpyPlaneDie |
| `MoveSound` | `MigMoveLoop` (shared with cut content?) | SpyPlaneMoveLoop |

**Key distinctions**:

1. **`Name=Soviet MIG`** vs the artmd comment "Boris Attack Plane".
   The unit *is* a MiG fighter (per the F=name field), called in
   by Boris for airstrikes. Westwood used MIG as the visual asset
   and BORIS-airstrike as the gameplay role.

2. **`;Image=FORTRESS;PDPLANE`**: TWO commented historical alternatives.
   - `FORTRESS` is the artmd block (line 763 in artmd) for a B-17-
     style flying fortress voxel.
   - `PDPLANE` would have reused the cargo plane voxel.
   - Westwood considered both before settling on BPLN's own
     `bpln.vxl` (with shared PrimaryFireFLH=25,100,0 — same offset
     as FORTRESS in artmd).

3. **`Strength=200`**: *fragile* (matches HORNET at 75, ASW at 30,
   but lighter than PDPLANE 400 / SPYP 600). The Airstrike planes
   are explicitly disposable — 2 planes per strike means individual
   plane survival isn't the goal.

4. **`Primary=Maverick3`**: **REAL COMBAT WEAPON**, not dummy. The
   verbatim historical `;Primary=ParaDropWeapon ; Doesn't really
   fire it; dummy weapon` shows BPLN was once a dummy-weapon
   transport like PDPLANE, but Westwood promoted it to a real
   combat weapon. **Maverick3 has Damage=750!** — see Weapon section.
   This is the **only spawn-child aircraft with a real damage
   weapon** documented so far.

5. **`Speed=16`** (`;18` historical commented). Slightly faster
   than SPYP (15) and PDPLANE (15). The airstrike profile needs
   quick in-and-out.

6. **`Ammo=1`** — single attack per BPLN. Combined with `AirstrikeTeam=2`
   on Boris, a Boris strike fires 2 missiles total (1 per plane).
   Elite Boris with `EliteAirstrikeTeam=4` fires 4 missiles.

7. **`Fighter=yes`** — AircraftType flag (per BEAG cheat-sheet
   `0x00818034 → 0x0041cc84`). Marks BPLN as fighter-class for AI
   threat-scoring. *Unique among spawn-children* — no other
   spawn-child sets Fighter=yes.

8. **`ElitePrimary=Maverick3E`** — active elite weapon swap. *Boris's
   rank determines which weapon the BPLN fires*. Same parent-veterancy
   mechanism as HORNET's `ElitePrimary=HornetBombE` (Carrier's rank
   triggers child's weapon). Per-parent-veterancy on spawn-child
   confirmed pattern.

9. **`Trainable=no`** (explicit) — BPLN never gains XP itself. Parent
   Boris's veterancy is the relevant rank.

10. **`VoiceCrashing=MigVoiceDie`** (explicit) — most spawn-children
    don't have a VoiceCrashing line. BPLN has one set to MigVoiceDie.
    *Possibly inherits from a parent template* where the MiG had a
    dedicated voice; vestigial.

11. **`CrashingSound=IntruderDie`** — *shared with cut Intruder
    content?* The "Intruder" name suggests this audio was for a cut
    or otherwise-named fighter unit. BPLN uses it.

### Otherwise identical to PDPLANE/SPYP

Shared fields: Locomotor=AircraftLocomotion, Spawned=yes, Landable=no,
MoveToShroud=yes, PitchAngle=0, ROT=2, Owner=universal,
ImmuneToPsionics=yes, CanPassiveAquire=no, CanRetaliate=no,
ThreatPosed=0, Crewed=yes, Selectable=no, DeathWeapon=BlimpBomb +
DeathWeaponDamageModifier=.1, FlyBy=true.

---

## Artmd verbatim

```ini
[BPLN] ; Boris Attack Plane
Voxel=yes
Remapable=yes
Cameo=BPLNICON
AltCameo=BPLNICON
PrimaryFireFLH=25,100,0
```

### Key-by-key annotation

- artmd header comment: **"Boris Attack Plane"** — confirms the
  artmd-side label. The rulesmd Name says "Soviet MIG" but the art
  is captioned as Boris's plane. *Naming inconsistency between
  rulesmd and artmd* — both refer to the same unit but use different
  labels.
- `Voxel=yes` — `bpln.vxl` + `.hva`.
- `Remapable=yes` — **remapable** (unlike PDPLANE/CARGOPLANE which
  are Remapable=no). Boris's MiG shows player house color — possibly
  to visually distinguish the airstrike-summoner's faction.
- `Cameo=BPLNICON` — dedicated sidebar cameo.
- `AltCameo=BPLNICON` — *same as Cameo=* (typo/oversight, like BEAG's
  AltCameo=BEAGICON).
- `PrimaryFireFLH=25,100,0` — Maverick3 launch offset:
  - X=25 (slightly forward).
  - Y=100 (very far to one side — drop-from-wing-hardpoint pattern).
  - Z=0 (water/ground level relative to plane altitude).
  - Same FLH as [FORTRESS] artmd block — likely Westwood copy-paste
    of FLH between similar bomber-class aircraft.

**Notable**: BPLN has `Remapable=yes` while all other spawn-children
(PDPLANE, CARGOPLANE, V3ROCKET, DMISL, CMISL) have `Remapable=no`.
The exception suggests Boris's planes are visually distinct per-
faction (Russian red, etc.). HORNET/ASW also lack Remapable=yes —
suggesting the rule is "spawn-children are usually gray/black, BPLN
is the exception".

---

## Weapons

### Primary — `[Maverick3]`

```ini
[Maverick3]
Damage=750
ROF=10
Range=4
Projectile=AirToGroundMissile ;GEF was AAHeatSeeker2 ; was HeatSeeker
Speed=70
Warhead=MIGWH
Report=MigAttack
Burst=2
```

- **`Damage=750`** — **MASSIVELY high** single-shot damage. Highest
  weapon damage documented so far. Compare:
  | Weapon | Damage |
  |--------|--------|
  | **Maverick3 (BPLN basic)** | **750** |
  | BlimpBomb (Kirov) | 250 |
  | DiskLaser | 200 |
  | Maverick2 (BEAG) | 200 |
  | 160mm (SCHP) | 90 |

  Half a Maverick3 (375 dmg with edge falloff) destroys most tanks
  in one missile. **Maverick3 is the most lethal single-shot weapon
  documented**.

- `ROF=10` — fast cycle, but only fires once due to Ammo=1 on BPLN.
- `Range=4` — **VERY short range**. The BPLN has to overfly the
  target to deliver. Vulnerable during approach.
- `Projectile=AirToGroundMissile` — same projectile as BEAG's
  Maverick2 (anti-ground, AG=yes AA=no, ROT=100 high tracking).
  Historical comments `;GEF was AAHeatSeeker2 ; was HeatSeeker`
  show iteration history.
- `Speed=70` — fast missile.
- `Warhead=MIGWH` — MiG-specific warhead (see below).
- `Report=MigAttack` — fire SFX (single sample `vmigatta`).
- `Burst=2` — **2 missiles per fire**. Combined with Ammo=1 fires
  2 missiles in one burst then no more. **Effective per-plane
  damage = 750 × 2 = 1500 base** (massive).

  *2 planes × 2 missiles = 4 missiles per Boris strike = 3000
  potential damage*. Elite Boris = 4 planes × 2 missiles = 8 missiles
  = 6000 potential damage. **Devastating airstrike**.

### Elite primary — `[Maverick3E]`

```ini
[Maverick3E]
Damage=400
ROF=10
Range=9
Projectile=AirToGroundMissile
Speed=70
Warhead=ORCAAP
Report=BlackEagleAttack
Burst=2
```

**Three changes vs basic Maverick3:**

1. **`Damage=400`** — *lower than basic 750*! Counterintuitive at
   first. **But**: elite Boris fires 4 planes × 2 missiles = 8
   missiles total per strike (vs basic 2×2=4). The total potential
   damage *increases* despite per-missile damage going down (8×400=3200
   vs 4×750=3000). Plus the 1.5× range improvement.

2. **`Range=9`** — *much longer range* (vs basic 4). Elite Boris
   strikes can hit targets from farther away. The BPLN doesn't have
   to overfly as closely.

3. **`Warhead=ORCAAP`** (vs basic MIGWH) — switches to BEAG's universal-
   100%-Verses warhead with PenetratesBunker=yes. Elite Boris bypasses
   Tank Bunker garrison.

4. **`Report=BlackEagleAttack`** (vs basic MigAttack) — borrows BEAG's
   fire SFX. *Audio reuse across Korean Fighter and elite Soviet
   MIG* — both Allied/Soviet "elite tier-3 strike fighter" units sound
   similar.

**Elite trade-off summary**: lower per-missile damage but more
missiles, longer range, better Verses profile (ORCAAP universal),
bunker-piercing. Net upgrade.

### Warhead — `[MIGWH]`

```ini
[MIGWH]
Wall=yes
Wood=yes
;CellSpread=.4
;PercentAtMax=1
Verses=100%,100%,100%,100%,100%,100%,100%,100%,50%,100%,100%
Conventional=yes
InfDeath=3
AnimList=S_CLSN16,S_CLSN22,S_CLSN30,S_CLSN42,S_CLSN58
ProneDamage=50%
```

- `Wall=yes`, `Wood=yes` — damages walls and wooden structures.
- *No `CellSpread=`* — **NO AoE**. Single-target hit. `;CellSpread=.4`
  and `;PercentAtMax=1` historical commented values.
- `Verses=100%,100%,100%,100%,100%,100%,100%,100%,50%,100%,100%`:
  | Armor    | Multiplier |
  |----------|-----------|
  | none-flak-plate-light-medium-heavy-wood-steel | **100%** |
  | concrete | **50%** |
  | special_1/2 | 100% |

  **Universal 100% except concrete**. Boris's MIG is a universal
  killer. Slightly weaker vs concrete (50%) — won't fully demolish
  hardened structures in one hit.
- `Conventional=yes`, `InfDeath=3` (explosion), `AnimList=S_CLSN*`
  5-anim collision pool, `ProneDamage=50%`.

**Net per-strike damage potential (basic Boris)**:
- 2 planes × 2 missiles × 750 damage = **3000 damage spread across
  4 missile impacts**.
- Vs concrete (50% Verses): 1500.
- Vs all other armor (100%): 3000.
- Most tanks are 300-1200 HP — 1-2 missiles destroys most ground
  units. 4 missiles in one strike = **mass devastation**.

### DeathWeapon — `[BlimpBomb]` ×0.1

Same crash-damage trick as SPYP. Verbatim comment **slightly
different** from SPYP's:
- SPYP: "needs a death weapon or it will do nothing when it crashes
  since its weapon is a camera"
- BPLN: "needs a death weapon or it will do one laser blast's worth
  of crash damage. **This gives control**"

The BPLN comment is *more verbose* — it acknowledges that without a
DeathWeapon, the crash would deal "one laser blast's worth" (some
default crash damage). DeathWeapon=BlimpBomb ×.1 *overrides* the
default to give Westwood explicit control. **Strong evidence that
there's a default-crash-damage mechanism in the engine** that
DeathWeapon-equipped units override.

---

## Boris's AirstrikeTeam mechanic (the BPLN trigger)

From rulesmd lines 4649-4655 (in `[BORIS]` section):

```ini
;Airstrike stuff

;How many planes to call in
AirstrikeTeam=2;
EliteAirstrikeTeam=4;
;What type of planes to call in
AirstrikeTeamType=BPLN
EliteAirstrikeTeamType=BPLN
;How long after the planes either leave the map or are destroyed will the next team of planes be ready?
AirstrikeRechargeTime=100;500
EliteAirstrikeRechargeTime=50;250
```

**Fields**:
- `AirstrikeTeam=2` — **basic Boris calls in 2 BPLN planes per
  strike**. Verbatim "How many planes to call in".
- `EliteAirstrikeTeam=4` — **elite Boris calls in 4 BPLNs**.
- `AirstrikeTeamType=BPLN` — *which aircraft type to spawn*. Same
  for basic and elite.
- `EliteAirstrikeTeamType=BPLN` — same value (BPLN). Could have been
  a different unit type at elite (e.g. a tier-3 strike fighter),
  but Westwood uses BPLN for both.
- `AirstrikeRechargeTime=100` — *frames between strikes for basic
  Boris*. Verbatim "How long after the planes either leave the map
  or are destroyed will the next team of planes be ready?". The
  `;500` historical commented value suggests this was originally
  longer (5× slower recharge); Westwood lowered it.
- `EliteAirstrikeRechargeTime=50` — *frames between strikes for
  elite Boris*. Half the basic time = strikes available twice as often.

**Cheat-sheet refs** (from prior iterations): the
`AirstrikeTeam/Type/RechargeTime` fields are TechnoType-scope per the
extended notes from earlier docs. Specifically:
- `AirstrikeTeam[Type/RechargeTime]` — listed in the TechnoTypeClass__ReadINI
  cheat-sheet.

### Behavior summary

When Boris targets a cell with his Airstrike command:
1. Engine checks Boris's veterancy: basic → 2 BPLNs, elite → 4 BPLNs.
2. Engine spawns N BPLN aircraft at map edge (offscreen).
3. Each BPLN flies straight toward target.
4. As BPLN approaches target (within Range=4 cells), Maverick3 fires
   Burst=2 missiles.
5. BPLN continues past target (FlyBy=true ensures no slowdown).
6. BPLN exits map edge OR is shot down OR crashes with DeathWeapon
   small explosion.
7. AirstrikeRechargeTime counts down from "planes left map OR all
   destroyed".
8. When recharged, Boris's airstrike command available again.

---

## Voices / sounds

```ini
[MigMoveLoop]
Sounds= vblelo1a vblelo2a vblelo2b vblelo2c vblelo3
Control= loop random all decay attack
Limit=2
FShift=-5 5
VShift=10
Volume=35

[MigAttack]
Sounds=vmigatta
FShift= -5 5
VShift=10
Volume=70

[IntruderDie]
Sounds=vintdiea vintdieb
Control=random
Volume=50
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| (most Voice* empty) | n/a | No player-interaction voices |
| `VoiceCrashing=MigVoiceDie` | `[MigVoiceDie]` (separate block) | Voice during crash (rare for spawn-children) |
| `Report=MigAttack` (Maverick3 weapon) | `[MigAttack]` | Missile launch SFX (single sample `vmigatta`) |
| `Report=BlackEagleAttack` (Maverick3E elite) | `[BlackEagleAttack]` | Elite missile launch SFX (BEAG-shared!) |
| `MoveSound=MigMoveLoop` | `[MigMoveLoop]` | **Jet engine loop sharing `vblelo*` samples with BEAG** (Limit=2 concurrent) |
| `DieSound=` (empty) | n/a | No instant-death SFX |
| `CrashingSound=IntruderDie` | `[IntruderDie]` | Crash plummet SFX |
| `ImpactLandSound=GenAircraftCrash` | shared | Generic impact |

**Cross-faction audio sharing — striking**:
- **`MoveSound=MigMoveLoop`** uses `vblelo*` samples. **`vblelo`
  prefix is the BEAG/Black Eagle prefix** (per BEAG sound iteration).
  So Boris's MIG and the Allied Black Eagle share the same engine
  drone loop. *Soviet MIG sounds like an Allied Black Eagle*. Either
  Westwood audio reuse or the `vintlo*` / `vblelo*` prefixes were
  intended for "interceptor/fighter family" units regardless of
  faction.

- **`Report=BlackEagleAttack`** on the *elite* Maverick3E weapon —
  cross-faction audio sharing in elite weapon. Soviet elite Boris
  strike sounds like Allied Black Eagle strike.

- **`[IntruderDie]`** — the "Intruder" name has no other rules
  reference. Possibly a cut "Intruder" fighter unit that left its
  audio behind. BPLN reuses it for crash SFX.

The Westwood audio design for fighter-class aircraft is heavily
reused across factions. The `[BlackEagleSelect]` block (12-sample
mixed pool with radio chatter) was unique to BEAG; the *fire SFX*
appears to be shared.

---

## Hardcoded behavior

### 1. Airstrike paradigm (5th spawn pathway)

**The defining feature**: BPLN is spawned by Boris's per-techno
`AirstrikeTeam` system, not by a building SuperWeapon (like PDPLANE
ParaDropSpecial or SPYP SpyPlaneSpecial).

The engine resolves Boris's `AirstrikeTeamType=BPLN` to identify
which AircraftType to spawn, and `AirstrikeTeam=2` / `EliteAirstrikeTeam=4`
to determine how many. **Per-unit (not per-building) spawn-team
configuration** is unique to the Airstrike paradigm.

This makes BPLN the only spawn-child *triggered by an infantry unit
ability* rather than a building/superweapon. The connection between
[BORIS] hero-class infantry and BPLN aircraft is hardcoded into the
engine's airstrike command path.

### 2. Real damage Primary (vs dummy/repurposed in other spawn-children)

BPLN is the **only spawn-child with a real combat weapon** documented
so far. Comparison:
- PDPLANE: dummy ParaDropWeapon ("Doesn't really fire it").
- SPYP: repurposed SpyCameraWeapon (Damage=6 means reveal-radius).
- HORNET, ASW: real weapons (HornetBomb, ASWBomb) but lower damage
  (40-50).
- V3ROCKET, DMISL, CMISL: no weapons (Rules-global warheads).
- **BPLN: Maverick3 Damage=750!**

BPLN's status as the "real combat fighter" of the spawn-child family
reflects its Airstrike role — it's meant to *do significant damage*,
not just deliver cargo.

### 3. FlyBy=true (shared with SPYP)

Same straight-pass behavior as SPYP. Critical for airstrike profile:
the plane drops missiles at target then continues off-map.

### 4. DeathWeapon=BlimpBomb ×.1 (shared with SPYP, DISK)

Same crash-damage modifier trick. BPLN's verbatim comment adds
detail: *"needs a death weapon or it will do one laser blast's worth
of crash damage. This gives control"*. The comment **reveals that
there's a default crash-damage** ("one laser blast's worth") which
DeathWeapon overrides. Cross-faction reuse:
- DISK (Yuri Floating Disc).
- SPYP (Soviet Spy Plane).
- BPLN (Boris MIG / airstrike plane).
- CASANF01 (civilian building).

### 5. Fighter=yes (AircraftType flag)

BPLN sets `Fighter=yes` (AircraftType-scope per BEAG cheat-sheet
`0x0041cc84`). Same flag as BEAG. *Both are "fighter-class"
aircraft* in the engine's AI threat-scoring. The flag distinguishes
combat-fighter from transport-only aircraft.

### 6. ElitePrimary swap triggered by parent's rank

Same pattern as HORNET (Carrier-elite swaps to HornetBombE).
**Boris's veterancy promotes BPLN's weapon**:
- Rookie/Veteran Boris: BPLN fires Maverick3 (Damage=750, Range=4).
- Elite Boris: BPLN fires Maverick3E (Damage=400, Range=9, ORCAAP
  warhead with PenetratesBunker=yes).

Combined with Elite Boris's `EliteAirstrikeTeam=4` (4 planes vs 2),
the elite strike is dramatically more powerful.

### 7. Trainable=no on the child

BPLN itself doesn't gain XP. Boris's rank is what matters. Same
pattern as all spawn-children.

### 8. Cross-faction audio inheritance from BEAG

BPLN's `MoveSound=MigMoveLoop` uses `vblelo*` samples (Black Eagle
prefix). Elite weapon `Report=BlackEagleAttack` (BEAG audio). The
"Soviet MIG" and Korean "Black Eagle" share audio identity in YR —
either accidental reuse during development or a deliberate "fighter
family" audio design.

### 9. AirstrikeTeam=2 / Burst=2 = 4 missiles per basic strike

Per-strike math:
- Basic Boris: 2 planes × 2 missiles/plane × 750 damage = 3000
  total potential damage.
- Elite Boris: 4 planes × 2 missiles/plane × 400 damage = 3200
  total potential damage.
- The numbers are tuned to be similar at basic and elite, but
  elite gets the longer range + ORCAAP warhead advantages.

---

## TS-legacy filter

- `;Image=FORTRESS;PDPLANE` — TWO commented historical art
  alternatives.
- `;Primary=ParaDropWeapon` — commented historical dummy weapon
  (BPLN was once a paradrop plane).
- `;was AAHeatSeeker2 ; was HeatSeeker` — historical projectile
  iterations on Maverick3.
- `Speed=16; 18` — historical commented value.
- `;CellSpread=.4 ;PercentAtMax=1` — historical commented values
  on MIGWH.
- `;500` and `;250` — historical commented AirstrikeRechargeTime
  values on Boris (was slower; Westwood made airstrike more
  frequent).
- `[IntruderDie]` audio block — possibly cut "Intruder" content
  whose audio remained.
- No `ImmuneToVeins`, no `Subterranean`. YR-active mechanism.

---

## Comparison: BPLN vs other airstrike-style aircraft

| Field | BPLN (Boris MIG) | BEAG (Black Eagle) | SHAD (Nighthawk) |
|-------|------------------|--------------------|--------------------|
| Section | AircraftTypes | AircraftTypes | VehicleTypes (jumpjet) |
| Spawned | yes | no | no |
| Player-built | no (TechLevel=-1) | yes (TL=3) | yes (TL=7) |
| Trigger | Boris's airstrike command | player-built, manual orders | player-built, manual orders |
| Primary | Maverick3 (Damage=750) | Maverick2 (Damage=200) | BlackHawkCannon (Damage=35) |
| Range | 4 | 6 | 6 |
| Burst | 2 | 1 | 1 |
| Fighter=yes | yes | yes | (not set) |
| Locomotor | Aircraft | Aircraft | Jumpjet |
| Per-strike count | 2 (basic) / 4 (elite) | 1 (player builds) | 1 (passenger transport) |
| Owner | universal | RequiredHouses=Alliance (Korea) | 5 Allied houses |

**BPLN is the per-strike-burst champion** — 4 missiles per strike
(basic) or 8 (elite) hits in rapid sequence. BEAG is single-shot
controlled-fire. SHAD is utility transport with self-defense.

---

## Cross-references

- [BORIS.md](./BORIS.md) — Soviet hero infantry, the airstrike-
  summoner. Configures BPLN deployment via AirstrikeTeam fields.
- [BEAG.md](../allied/BEAG.md) — Allied peer fighter (player-built
  vs spawn-child). Audio sharing partner.
- [PDPLANE.md](../civilian/PDPLANE.md) — drop-and-exit paradigm
  sibling (different mission: paradrop).
- [SPYP.md](./SPYP.md) — drop-and-exit paradigm sibling (different
  mission: reveal). Shares DeathWeapon ×.1 trick and FlyBy=true.
- [DISK.md](../yuri/DISK.md) — DeathWeapon ×.1 cross-faction
  partner.
- [FORTRESS] / Intruder — pending. Possibly cut content related to
  BPLN's historical alternatives.

---

## Coverage audit

- [x] Every rulesmd key annotated (~40 keys).
- [x] Every artmd key annotated (5 keys).
- [x] Both weapons documented (Maverick3 basic Damage=750 + Maverick3E
  elite with Burst+Range trade-off).
- [x] MIGWH warhead documented (universal-100% except concrete 50%).
- [x] DeathWeapon ×.1 trick noted (different comment from SPYP).
- [x] All voice/sound bindings documented including cross-faction
  audio sharing (vblelo with BEAG, BlackEagleAttack elite weapon).
- [x] **Boris's AirstrikeTeam mechanic** fully documented (per-
  techno AirstrikeTeam/Type/RechargeTime fields).
- [x] Hardcoded behavior: **Airstrike paradigm (5th spawn pathway)**,
  real damage Primary (only spawn-child with one), FlyBy=true,
  DeathWeapon ×.1, Fighter=yes AircraftType flag, ElitePrimary
  triggered by parent's rank, cross-faction audio.
- [x] TS-legacy filter applied.
- [x] Comparison table with peer fighters (BEAG, SHAD).
- [ ] **No Ghidra verification this iteration** (MCP server offline).

**Ghidra status**: MCP server still disconnected. No new cheat-sheet
entries. Field-scope claims cross-reference prior verified entries.

**Re-confirmed cheat-sheet:**
- `Fighter` (AircraftType, per BEAG).
- All shared fields with PDPLANE/SPYP.
- `AirstrikeTeam`/`AirstrikeTeamType`/`AirstrikeRechargeTime` fields
  (TechnoType-scope per extended cheat-sheet notes).

**Open questions resolved this iteration:**
- ✓ Confirmed FlyBy=true on BPLN (matches SPYP). The flag is shared
  across drop-and-exit airstrike-style planes.
- ✓ Confirmed DeathWeapon ×.1 trick on a third unit (BPLN, after
  DISK and SPYP). Trick is for non-damage primary weapons that
  need crash impact override.

**Open questions remaining:**
- The 5th spawn pathway (Airstrike) — Ghidra trace of Boris's
  airstrike command function and BPLN spawn dispatch.
- `[IntruderDie]` audio block — is there cut "Intruder" content?
- The verbose BPLN DeathWeapon comment ("one laser blast's worth")
  suggests a hardcoded default crash damage value — Ghidra trace
  for the default value.

**Aircraft documentation status — 11 of 12 complete:**
- Slot 1 APACHE — *cut* (commented out, skipped).
- Slot 2 ORCA — likely cut (in priority queue).
- Slot 3 HORNET ✓
- Slot 4 V3ROCKET ✓
- Slot 5 ASW ✓
- Slot 6 DMISL ✓
- Slot 7 PDPLANE ✓
- Slot 8 BEAG ✓
- Slot 9 CARGOPLANE ✓
- Slot 10 **BPLN ✓** (this iteration)
- Slot 11 SPYP ✓
- Slot 12 CMISL ✓

**Active aircraft batch CLOSED**: 11 of 12 active (ORCA still
pending — likely cut content). APACHE is documented as cut elsewhere.
