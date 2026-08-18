---
name: beag-doc
description: BEAG — Korean Black Eagle. Allied tier-3 fighter, RequiredHouses=Alliance
  (Korean exclusive). Maverick2 missile (Damage=200 single, Elite=400 + Range 6→9).
  **NEW Ghidra scope: AircraftTypeClass__ReadINI (0x0041cxxx range)** with Fighter
  and AirportBound fields. Ammo=1 single-shot reload-at-airport. Standalone
  player-controlled aircraft (vs spawn-children).
metadata:
  type: project
---

# BEAG — Black Eagle

**INI ID:** `BEAG`
**Display:** "Black Eagle" (`UIName=Name:BEAGLE` — note CSF key is `BEAGLE`,
not `BEAG`)
**Section:** `[AircraftTypes]` — *the first AircraftType-section unit
documented in this index*. Previous "air" units (Kirov, Disc, SHAD) are
declared in `[VehicleTypes]` with `JumpJet=yes` / `ConsideredAircraft=yes`.
BEAG is the proper aircraft-class unit (fixed-wing jet, airport-bound).
**Owner side:** Allied (British, French, Germans, Americans, Alliance) **with
RequiredHouses=Alliance** — *Korea-exclusive*.
**Role:** Allied tier-3 strike fighter. South Korea's faction-unique unit.
Single-shot air-to-ground missile, returns to airport to reload. Replaces
the Allied generic fighter slot for Korean players (other Allied factions
get the Harrier, which is a different INI section — open question).

---

## Major scope discovery this iteration

**AircraftTypeClass__ReadINI is a new ReadINI scope** (0x0041cxxx range)
discovered while verifying BEAG fields. Two fields verified at this scope:
- `AirportBound` (0x0081803c → 0x0041cc6e)
- `Fighter` (0x00818034 → 0x0041cc84)

This adds AircraftType to the ReadINI hierarchy:

| Scope | Address range | Section it reads |
|-------|---------------|---------------------|
| `ObjectTypeClass__ReadINI` | 0x005f9xxx | broadest — any ObjectType |
| `TechnoTypeClass__ReadINI` | 0x00712-0x00715xxx | general unit/building |
| `UnitTypeClass__ReadINI` | 0x00747xxx | vehicles only |
| `InfantryTypeClass__ReadINI` | 0x00524xxx | infantry only |
| **`AircraftTypeClass__ReadINI`** | **0x0041cxxx** | **aircraft only (NEW)** |
| `WeaponTypeClass__ReadINI` | 0x00772xxx | weapons |
| `WarheadTypeClass__ReadINI` | 0x0075Dxxx | warheads |
| `BulletTypeClass__ReadINI` | 0x0046cxxx | projectiles |
| `RulesClass__Read*` | 0x00669-0x0067Dxxx | rules globals |

This means certain fields are *aircraft-only* — they read on aircraft-class
units but are silently ignored on vehicles/infantry. Vehicle-class units
with `JumpJet=yes` (Kirov, Disc, SHAD) read TechnoType fields but **NOT**
AircraftType fields. The flying-vehicle architecture is fundamentally
different from the airplane-aircraft architecture.

---

## Rulesmd verbatim

```ini
[BEAG]
UIName=Name:BEAGLE
Name=Black Eagle
Prerequisite=RADAR
Primary=Maverick2
CanPassiveAquire=no ; Won't try to pick up own targets
CanRetaliate=no; Won't fire back when hit
Strength=200
Category=AirPower
Armor=light
TechLevel=3
Sight=8
RadarInvisible=no
Landable=yes
MoveToShroud=yes
Dock=GAAIRC,AMRADR
PipScale=Ammo
Speed=14
PitchSpeed=1.1
PitchAngle=0
OmniFire=yes
Owner=British,French,Germans,Americans,Alliance
RequiredHouses=Alliance
Cost=1200
Points=20
ROT=3
Ammo=1
Crewed=yes
ConsideredAircraft=yes
AirportBound=yes ; If I ever need to land and there are no airports I crash because I can only land on them
GuardRange=30
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=3
IsSelectableCombatant=yes
VoiceSelect=BlackEagleSelect
VoiceMove=BlackEagleMove
VoiceAttack=BlackEagleAttackCommand
VoiceCrashing=BlackEagleVoiceDie
DieSound=
MoveSound=BlackEagleMoveLoop
CrashingSound=BlackEagleDie
ImpactLandSound=GenAircraftCrash
Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1}
MovementZone=Fly
ThreatPosed=20	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
;AuxSound1=BlackEagleTakeOff	;Taking off
;AuxSound2=BlackEagleLanding	;Landing
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=STRONGER,FIREPOWER,ROF
Fighter=yes
AllowedToStartInMultiplayer=no
ImmuneToPsionics=yes
ElitePrimary=Maverick2E
PreventAttackMove=yes
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:BEAGLE` — CSF key is `BEAGLE` (the full label), distinct
  from section name `BEAG` (3-letter abbreviation). Resolves to "Black
  Eagle".
- `Name=Black Eagle` — internal description.
- `Category=AirPower` — sidebar/AI threat bucket. Same as Kirov, Disc,
  Carrier-Hornets.

**Tech / availability — Korean exclusive**
- `Prerequisite=RADAR` — *any radar building* (resolves to Allied
  Airforce Command HQ `GAAIRC` for Allied). Single prereq.
- `TechLevel=3` — tier-3 (early-mid game).
- `Owner=British,French,Germans,Americans,Alliance` — *all 5 Allied
  houses listed*, BUT...
- `RequiredHouses=Alliance` — **the gate**. **Ghidra-verified TechnoType**
  at `0x00843bb4 → 0x00714529` (per cheat-sheet). `RequiredHouses=`
  restricts the build to specific country slots. **`Alliance`** is the
  internal name for **South Korea** in YR — *the Korean player gets
  Black Eagle as their faction unique*, in place of the generic Allied
  Harrier. Other Allied players (British/French/Germans/Americans)
  cannot build BEAG despite being in the `Owner=` list — the
  RequiredHouses gate overrides.
- `AllowedToStartInMultiplayer=no` — not a starting unit.

**Combat — defense**
- `Strength=200` — fragile (matches Hornet, ASW). Aircraft are glass
  cannons.
- `Armor=light` — light armor; AT weapons hit hard.

**Combat — single-shot bomber missile**
- `Primary=Maverick2` — 200 dmg single-missile, Range=6, AirToGroundMissile
  projectile, ORCAAP warhead. See Weapon section.
- `ElitePrimary=Maverick2E` — Damage 200→400 (2× damage) AND Range 6→9
  (1.5× range). Substantial elite swap.
- `Burst=1;2` on weapon — *single missile per attack*. The `;2` historical
  commented value shows it was once 2-burst — Westwood reduced to 1.
- Combined with `Ammo=1`: **single-missile-then-return-to-base** doctrine.
  After firing once, the BEAG must fly home to GAAIRC to reload.
- `Ammo=1` — *one missile capacity*. Single-shot weapon then refuel.
  **Note**: the bare `Ammo` field name doesn't appear as a standalone
  string in the binary — the Ghidra `search_strings("Ammo")` returns
  only `AmmoCrateDamage` and `InitialAmmo`. The `Ammo=` field is likely
  read via a different mechanism (maybe a hardcoded offset in a
  TechnoTypeClass::ReadINI helper, or a different naming convention).
  Functionally: 1 missile per sortie.
- `OmniFire=yes` — fires without facing requirement. Aircraft strafing
  patterns benefit from omnifire.
- `CanPassiveAquire=no` — *will NOT auto-target enemies in range*.
  Verbatim "Won't try to pick up own targets". Same flag as SHAD and
  Carrier. Pilots only engage what they're explicitly ordered to attack.
- `CanRetaliate=no` — **does NOT auto-return-fire when hit**. *NEW
  behavioral flag* not yet tied to a cheat-sheet entry. Verbatim "Won't
  fire back when hit". Most units have CanRetaliate=yes (default);
  BEAG deliberately disables this to enforce strict pilot-control —
  the plane doesn't sortie missiles unexpectedly during retreat.
- `PreventAttackMove=yes` — cannot be Attack-Moved (TechnoType per
  cheat-sheet `0x008439b0 → 0x00714994` from SHAD). Strict pilot-
  control flag. Black Eagles fly *exact* waypoint routes; they don't
  improvise engagement during transit.

**Sight / radar**
- `Sight=8` — moderate visual range.
- `RadarInvisible=no` — appears on enemy radar (unlike SHAD's stealth).
  Defenders see BEAG coming.
- `MoveToShroud=yes` — can attack into unexplored shroud cells.
- `Landable=yes` — *aircraft can land*. Required for the airport
  return-to-base cycle.
- `PipScale=Ammo` — pip bar shows ammo (1 pip = 1 missile loaded).

**Mobility**
- `Speed=14` — fast. Matches SHAD.
- `PitchSpeed=1.1, PitchAngle=0` — pitch animation parameters
  (TechnoType per cheat-sheet from CARRIER doc).
- `ROT=3` — *moderate turn rate*. Slower than SHAD (6) but faster than
  Kirov (2). Fighters bank in flight.
- `Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1}` — **AircraftLocomotion
  GUID** (`...746`). **Distinct from Drive (`...741`), Hover (`...742`),
  Jumpjet (`{92612C46-...}`), Submarine (`{2BEA74E1-...}`), Rocket
  (`{B7B49766-...}`)**. The 5th locomotor GUID. *Fixed-wing aircraft*
  movement — requires forward speed, can't hover, must land at airport.
- `MovementZone=Fly` — fly-zone pathing.

**Aircraft-class flags**
- `ConsideredAircraft=yes` — treated as aircraft for targeting rules
  (TechnoType, per cheat-sheet from DISK doc).
- `Fighter=yes` — **AircraftType-only flag**. [BINARY-VERIFIED audit 26: string @ 0x00818034, parser xref @ 0x0041CC84, `AircraftType+0xE0E` (byte) — first AircraftType-scope addition to cheat sheet]. Marks
  the aircraft as a fighter class — affects:
  - AI threat-scoring (fighters prioritize air-vs-air engagements).
  - Combat-AI behavior (fighters may break off for rearming differently
    than bombers).
  - The exact behavioral difference is not fully documented here;
    likely the Fighter flag enables fighter-specific maneuvering or
    targeting heuristics.
- `AirportBound=yes` — **AircraftType-only flag**. [BINARY-VERIFIED audit 26: string @ 0x0081803C, parser xref @ 0x0041CC6E, `AircraftType+0xE0D` (byte)]. Verbatim comment: *"If I ever need to land and
  there are no airports I crash because I can only land on them"*.
  Forces the aircraft to land *only* at airport-class buildings
  (GAAIRC, AMRADR). If no airport exists, the BEAG crashes after
  fuel/ammo depletion. **Tactical consequence**: destroying the
  enemy's airport while their fighters are airborne crashes the
  fighters.

**Dock**
- `Dock=GAAIRC,AMRADR` — *list of valid landing buildings*. `GAAIRC` =
  Allied Airforce Command HQ. `AMRADR` = American Radar (campaign-only
  building?). The fighter cycles between these.

**Crew / death**
- `Crewed=yes` — crew (pilot) ejects on death... actually for aircraft
  this may be different. The crash mechanism handles aircraft death
  separately from ground-vehicle crew-eject. Worth verifying.
- `Explosion=TWLT070,...` — explosion pool.
- `MaxDebris=3` — minimal debris.
- `DieSound=` — empty (handled by CrashingSound/ImpactLandSound).
- `CrashingSound=BlackEagleDie` — looping SFX during plummet.
- `ImpactLandSound=GenAircraftCrash` — generic aircraft-impact SFX
  (DUAL-READ per ZEP doc).
- `IsSelectableCombatant=yes`.

**Voice / sound bindings**
- `VoiceSelect=BlackEagleSelect` — mixed Korean radio + pilot voice
  pool (12-sample! with $-prefixed pilot + non-prefixed radio chatter).
- `VoiceMove=BlackEagleMove` — same mixed pool.
- `VoiceAttack=BlackEagleAttackCommand` — same.
- `VoiceCrashing=BlackEagleVoiceDie` — 5-sample $vbledi* pool.
- `MoveSound=BlackEagleMoveLoop` — jet engine loop (7-sample random-
  loop pool of `vintlo*` — *NOTE: shares "vintlo" audio prefix with
  another unit*, possibly cut "Intercept" content).
- `;AuxSound1/2=BlackEagleTakeOff/Landing` — commented; the takeoff/
  landing sound blocks in soundmd are also empty (`Sounds=` blank).
  Disabled in shipped YR.

**Combat behavior**
- `ThreatPosed=20` — moderate AI threat.
- `GuardRange=30` — *very long auto-engagement range* in Guard mode
  (vs Sight=8). Once airborne, BEAG patrols a 30-cell radius.
- `ImmuneToPsionics=yes` — cannot be mind-controlled by Yuri. Aircraft
  pilots are out of psi-control range.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` — 4 abilities
  (no ROF — single-shot weapon means ROF buff is useless until reload).
- `EliteAbilities=STRONGER,FIREPOWER,ROF` — adds ROF at elite. Combined
  with the weapon swap (Maverick2 → Maverick2E with double damage +
  longer range), elite BEAG is significantly stronger.
- Note: **no SELF_HEAL** — aircraft don't repair mid-flight; they
  must land at GAAIRC's repair pad function (built-in for Airforce
  Command HQ).

**Damage particles**
- `DamageParticleSystems=SparkSys,SmallGreySSys`.

---

## Artmd verbatim

```ini
[BEAG] ; Black Eagle
Voxel=yes
Remapable=yes
Cameo=BEAGICON
AltCameo=BEAGICON
```

### Key-by-key annotation

- `Voxel=yes` — rendered from `beag.vxl` + `beag.hva`.
- `Remapable=yes` — house-color remap.
- `Cameo=BEAGICON` — sidebar build button.
- `AltCameo=BEAGICON` — *same as `Cameo=`* (typo or deliberate
  fallback? Most units have distinct AltCameo. Probably a placeholder
  for an alt cameo that was never created — BEAGICON serves both).

**No `PrimaryFireFLH=`** — missile launch handled by AircraftType-
specific code (likely from each wing-hardpoint based on direction).

---

## Weapons

### Basic — `[Maverick2]`

```ini
[Maverick2]
Damage=200
ROF=10
Range=6
Projectile=AirToGroundMissile ;GEF was AAHeatSeeker2 ; was HeatSeeker
Speed=70
Warhead=ORCAAP
Report=BlackEagleAttack
Burst=1;2
```

- `Damage=200` — strong single-hit. Useful vs tier-2/3 ground targets.
- `ROF=10` — *would be very fast* (10 ticks ~= 2/3 second) — but
  combined with `Ammo=1`, the BEAG fires *once* per sortie. ROF only
  matters between sorties via Burst (which is 1).
- `Range=6` — *short for an aircraft*. BEAG must close to ~6 cells of
  target. AA defenses at 7.5+ cell range can engage BEAG before it
  can fire.
- `Projectile=AirToGroundMissile` — see projectile block. `;GEF was
  AAHeatSeeker2 ; was HeatSeeker` historical comments show two
  iterations of projectile choice: HeatSeeker → AAHeatSeeker2 →
  AirToGroundMissile (final).
- `Speed=70` — moderate missile speed.
- `Warhead=ORCAAP` — see warhead block.
- `Report=BlackEagleAttack` — fire SFX (2-sample `vbleatt*` random).
- `Burst=1;2` — single missile (historical 2-burst).

### Elite — `[Maverick2E]`

```ini
[Maverick2E]
Damage=400
ROF=10
Range=9
Projectile=AirToGroundMissile
Speed=70
Warhead=ORCAAP
Report=BlackEagleAttack
Burst=1;2
```

**Two upgrades vs basic:**
1. `Damage=400` — **2× damage** (200 → 400).
2. `Range=9` — **1.5× range** (6 → 9). Elite BEAG out-ranges most AA
   defenses.

Elite BEAG is a major qualitative upgrade — at 400 damage per
sortie, it can one-shot most light/medium vehicles, and the 9-range
lets it stand off from Patriot (Range=7.5) and similar AA.

### Projectile — `[AirToGroundMissile]`

```ini
[AirToGroundMissile]
Arm=2
Shadow=no
;Proximity=yes
Proximity=no
Ranged=yes
AA=no
AG=yes
Image=DRAGON
ROT=100 ;was 60
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=no
```

- `Arm=2` — 2-frame arming.
- `Shadow=no`, `Image=DRAGON` — missile uses `dragon.shp` (RA2's
  generic missile sprite; SAM-shaped).
- `Proximity=no` (verbatim disable; `;Proximity=yes` historical
  enabled) — *direct-hit only*, no proximity fuse.
- `Ranged=yes` — fuse-based range check.
- `AA=no, AG=yes` — anti-ground only. Black Eagle cannot fight other
  aircraft.
- `ROT=100` (`;was 60`) — **very high tracking rate**. The missile
  homes aggressively after launch.

### Warhead — `[ORCAAP]`

```ini
[ORCAAP]
Wall=yes
Wood=yes
CellSpread=.4
PercentAtMax=1
Verses=100%,100%,100%,100%,100%,100%,100%,100%,75%,100%,100%
Conventional=yes
InfDeath=3
AnimList=S_CLSN16,S_CLSN22,S_CLSN30,S_CLSN42,S_CLSN58
ProneDamage=50%
PenetratesBunker=yes;If shot at a bunkered tank, no means the bunker gets the damage, yes means the unit does
```

- `Wall=yes`, `Wood=yes` — damages walls and wooden buildings.
- `CellSpread=.4` — small AoE.
- `PercentAtMax=1` — full damage at edge (no falloff).
- `Verses=100%,100%,100%,100%,100%,100%,100%,100%,75%,100%,100%`:
  | Armor    | Multiplier | vs Damage 200 |
  |----------|-----------|-----------------|
  | none-flak-plate-light-medium-heavy-wood-steel | 100% | 200 |
  | concrete | **75%** | 150 |
  | special_1 | 100% | 200 |
  | special_2 | 100% | 200 |

  **Universal 100% damage** across nearly all armor types except 75%
  vs concrete (buildings). Black Eagle is a *generalist* striker —
  effective against everything from infantry to tanks to buildings.
  *Anti-armor-piercing* (the name AP).
- `Conventional=yes` — conventional damage.
- `InfDeath=3` — explosion infantry death.
- `PenetratesBunker=yes` — **the critical building-defense bypass**.
  Verbatim: *"If shot at a bunkered tank, no means the bunker gets
  the damage, yes means the unit does"*. When the target is inside a
  Tank Bunker (NATBNK), the *occupant* takes damage, not the bunker
  itself. Ghidra-verified WarheadType `0x00847e08 → 0x0075d52f` per
  cheat-sheet (from CARRIER doc). Bypass of Soviet's Tank Bunker
  garrison.
- `ProneDamage=50%` — prone infantry take half damage.

---

## Voices / sounds

```ini
[BlackEagleSelect]
Sounds= vblecl1a vblecl1b vblecl1c $vblesea $vbleseb $vblesec $vblesed $vblesee $vblesef vblecl3a vblecl3b vblecl3c
Control= random attack decay
Attack=3
Decay=3
Volume=85

[BlackEagleMove]
Sounds= vblecl1a vblecl1b vblecl1c $vblemoa $vblemob $vblemoc vblecl3a vblecl3b vblecl3c
Control= random attack decay

[BlackEagleAttackCommand]
Sounds= vblecl1a vblecl1b vblecl1c $vbleata $vbleatb $vbleatc vblecl3a vblecl3b vblecl3c
Control= random attack decay

[BlackEagleVoiceDie]
Sounds= $vbledia $vbledib $vbledic $vbledid $vbledie
Priority=low
Control= random
Volume=65

[BlackEagleAttack]
Sounds=vbleatta vbleattb
FShift= -5 5
Volume=45

[BlackEagleDie]
Sounds=vblediea vbledieb
Control=random
Volume=50

[BlackEagleMoveLoop]
Sounds= vintlo1a vintlo1b vintlo1c vintlo2a vintlo2b vintlo2c vintlo3
Control= loop random all decay attack
Attack=3
Priority=low
FShift= -10 10
VShift=10
Volume=20
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=BlackEagleSelect` | `[BlackEagleSelect]` | Click — **12-sample mixed pool** (3 radio chatter `vblecl1*` + 6 pilot voice `$vblese*` + 3 radio chatter `vblecl3*`) |
| `VoiceMove=BlackEagleMove` | `[BlackEagleMove]` | Move order (9-sample mixed) |
| `VoiceAttack=BlackEagleAttackCommand` | `[BlackEagleAttackCommand]` | Attack order (9-sample mixed) |
| `VoiceCrashing=BlackEagleVoiceDie` | `[BlackEagleVoiceDie]` | Voice during crash |
| `Report=BlackEagleAttack` (weapon) | `[BlackEagleAttack]` | Missile launch SFX (2-sample) |
| `MoveSound=BlackEagleMoveLoop` | `[BlackEagleMoveLoop]` | Jet engine loop (7-sample looping pool of `vintlo*`) |
| `CrashingSound=BlackEagleDie` | `[BlackEagleDie]` | Sustained crash SFX |
| `ImpactLandSound=GenAircraftCrash` | shared | Impact SFX |
| `DieSound=` (empty) | n/a | Death frame is silent (handled by crash chain) |

**Multi-sample mixed pools** — `[BlackEagleSelect]` mixes 6 voice
samples (`$`-prefixed) with 6 radio-chatter samples (non-prefixed,
`vblecl*`). The `Control= random attack decay` with `Attack=3, Decay=3`
means: each playback fades in over 3 frames and out over 3 frames,
creating the radio-transmission effect. Distinctive Korean fighter
pilot audio identity.

**`vintlo*` pool**: the Black Eagle's engine loop uses
`vintlo1a/b/c, vintlo2a/b/c, vintlo3` — 7 samples, prefix `vintlo`
(likely "vehicle-INTerceptor-LOop" or similar). May be shared with a
cut "Intercept" or "Interceptor" unit.

---

## Hardcoded behavior (Ghidra-verified)

### 1. AircraftTypeClass__ReadINI scope (NEW)

**The big discovery**: BEAG is the first standalone AircraftTypes-section
unit I've documented, and it reveals **`AircraftTypeClass__ReadINI`** at
`0x0041cxxx` as a distinct ReadINI scope. Two fields verified:
- `Fighter` (0x00818034 → 0x0041cc84) — fighter-class flag.
- `AirportBound` (0x0081803c → 0x0041cc6e) — *must land at airport*.

Both fields are *aircraft-only* — vehicles with `JumpJet=yes`
(Kirov, Disc, SHAD) declared in `[VehicleTypes]` do NOT read these
fields. The flying-vehicle architecture uses jumpjet locomotion;
the aircraft architecture uses AircraftLocomotion with airport-
return cycles.

### 2. RequiredHouses=Alliance gates BEAG to Korea

`RequiredHouses` (TechnoType `0x00843bb4 → 0x00714529`). Restricts
buildability to specific country slots. **`Alliance` is the internal
identifier for South Korea** (the in-game country list maps:
British=Britain, French=France, Germans=Germany, Americans=USA,
Alliance=Korea, then Soviet sub-factions Russians/Confederation/
Africans/Arabs, then YuriCountry).

The combination of `Owner=British,French,Germans,Americans,Alliance`
(all Allied houses listed) + `RequiredHouses=Alliance` means:
- The Owner gate alone would allow all 5 Allied houses.
- But RequiredHouses additionally constrains to *only* Alliance/Korea.
- Net: only Korean players can build BEAG. Other Allied factions get
  a different aircraft (likely Harrier — open question for follow-up
  iteration).

### 3. AirportBound + Ammo=1 reload cycle

**AirportBound=yes** forces a specific reload pattern:
1. BEAG launches from GAAIRC (airport) with Ammo=1 (one missile).
2. Player issues attack command. BEAG flies to target, fires
   Maverick2, depletes Ammo to 0.
3. BEAG is now out of ammo. Cannot engage further targets.
4. Auto-routes back to GAAIRC (or AMRADR if available) to land.
5. Landing triggers reload (Ammo refills to 1 over time).
6. BEAG re-launches when player issues next attack order.

If the player loses GAAIRC and AMRADR while BEAG is airborne, the
BEAG cannot land — verbatim: *"If I ever need to land and there are
no airports I crash because I can only land on them"*. **Tactical
consequence**: Soviet players targeting GAAIRC first destroys all
airborne Korean Black Eagles upon next landing attempt.

### 4. CanRetaliate=no behavior

The `CanRetaliate=no` flag (not yet Ghidra-verified for scope this
iteration; likely TechnoType) disables auto-return-fire. When BEAG
is hit by AA, it does NOT immediately retarget the attacker — stays
on its current orders. Combined with `CanPassiveAquire=no` and
`PreventAttackMove=yes`, BEAG is *fully scripted by player commands*
— exactly what the player tells it, nothing more.

### 5. Aircraft Locomotor (5th known GUID)

`Locomotor={4A582746-...}` — the **fixed-wing aircraft locomotor**.
Distinct from Drive (...741), Hover (...742), Jumpjet (`{92612C46-
...}`), Submarine (`{2BEA74E1-...}`), Rocket (`{B7B49766-...}`).
**5th GUID in the cheat-sheet**:

| GUID suffix | Locomotor class | Used by |
|-------------|-----------------|---------|
| `...741` | Drive | Land vehicles |
| `...742` | Hover | Hovercraft, Robot Tank, naval AA |
| `...746` | **AircraftLocomotion** | **BEAG, planes (NEW)** |
| `{92612C46-...}` | JumpjetLocomotion | Kirov, Rocketeer, Disc, SHAD |
| `{2BEA74E1-...}` | SubmarineLocomotion | All naval (subs, ships, organic) |
| `{B7B49766-...}` | RocketLocomotion | V3ROCKET, DMISL, CMISL spawn missiles |

The AircraftLocomotion handles:
- Forward-velocity dependency (can't hover, must move forward to fly).
- Takeoff/landing animations at airports.
- Banking (PitchSpeed, PitchAngle) during turns.
- Ammo-based return-to-base logic.

### 6. PenetratesBunker on ORCAAP warhead

Per cheat-sheet WarheadType `0x00847e08 → 0x0075d52f` (from CARRIER
doc). Bypasses Tank Bunker garrison wall — *the bunkered unit takes
damage, not the bunker*. Critical anti-Tank-Bunker capability.

### 7. PreventAttackMove + CanPassiveAquire + CanRetaliate triple-disable

All three behavioral-control flags are set. BEAG is *fully script-
driven*:
- `PreventAttackMove=yes` — no attack-move command.
- `CanPassiveAquire=no` — no auto-engagement during waypoint travel.
- `CanRetaliate=no` — no return-fire when hit.

The Korean Black Eagle requires *deliberate player control* — single-
shot, hand-aimed missile strikes against high-value targets.

---

## TS-legacy filter

- `;Proximity=yes` on AirToGroundMissile — commented historical.
- `;ROT=60` historical — raised to 100.
- `;AAHeatSeeker2 ; was HeatSeeker` — three-iteration projectile
  history annotated in comments.
- `Burst=1;2` — historical 2-burst.
- `;Burst=2` not present.
- `;AuxSound1/2=BlackEagleTakeOff/Landing` — commented (and the sound
  blocks are also commented in soundmd). Takeoff/landing SFX disabled.
- *No `ImmuneToVeins`, no `Subterranean`*. **YR-active core
  mechanism.**

---

## Comparison: standalone aircraft profile

| Field | BEAG Black Eagle | ZEP Kirov (vehicle) | SHAD Nighthawk (vehicle) |
|-------|------------------|---------------------|----------------------------|
| Section | **AircraftTypes** | VehicleTypes | VehicleTypes |
| ReadINI scope | **AircraftType** | TechnoType only | TechnoType only |
| Locomotor | **Aircraft (...746)** | Jumpjet (`{92612C46-...}`) | Jumpjet (`{92612C46-...}`) |
| Strength | 200 | 2000 | 175 |
| Cost | 1200 | 2000 | 1000 |
| Speed | 14 | 5 | 14 |
| Ammo | 1 | not applicable | not applicable |
| AirportBound | yes | no (BalloonHover) | no (BalloonHover) |
| Fighter | yes | no | no |
| Landable | yes | yes (Balloon "land") | yes |
| Crashable | not set (always crashes via aircraft death) | yes | yes |
| RequiredHouses | Alliance (Korea) | none | none |
| Has crew | yes | no | yes |

**Architectural distinction**: BEAG is a true aircraft (AircraftType,
AircraftLocomotion, Ammo+AirportBound reload cycle). Kirov/Disc/SHAD
are *vehicles with jumpjet locomotion* — they're really tanks that fly.

**Korean exclusive**: BEAG is the only Korea-only unit in the
documented index so far. Other RequiredHouses-gated units include:
- CCOMAND (RequiresStolenAlliedTech)
- TNKD (RequiredHouses=Germans) — Tank Destroyer
- DTRUCK (RequiredHouses=Africans) — Demolitions Truck
- TTNK (RequiredHouses=Russians) — Tesla Tank
- BEAG (RequiredHouses=Alliance) — Black Eagle

The Allied/Soviet faction-exclusives. South Korea's BEAG joins the
faction-exclusive roster.

---

## Cross-references

- [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — jumpjet vs aircraft locomotor distinction.
- [ZEP.md](../soviet/ZEP.md) — Soviet "tier-10 air bomber" alternative
  (Kirov, jumpjet not aircraft).
- [SHAD.md](../allied/SHAD.md) — Allied stealth transport (jumpjet).
- [DISK.md](../yuri/DISK.md) — Yuri air unit (BalloonHover jumpjet).
- [TNKD.md](../allied/TNKD.md) — Germany-exclusive faction unit (peer
  RequiredHouses gate).
- Harrier — pending. Generic Allied fighter for non-Korean houses.
- ORCA — pending. Possibly cut Allied aircraft.

---

## Ghidra audit log (audit iteration 26 — 2026-05-18)

**Methodology**: BEAG introduces a **NEW parser-function scope**
(`AircraftTypeClass__ReadINI`) — the first AircraftType-scope addition
to the cumulative cheat sheet, parallel to the audit-21 ObjectType and
audit-22 BulletType discoveries. This audit verifies all BEAG claims
AND fully decompiles `AircraftTypeClass__ReadINI` to enumerate the
complete AircraftType-specific field layout. ~9 Ghidra queries: 4
string searches + 2 xref lookups + 1 get_function_by_address + 1 full
AircraftTypeClass__ReadINI decompile + 1 grep on TechnoTypeClass__ReadINI.

### Negative claim re-verified

| Query | Result |
|-------|--------|
| `search_strings("^BEAG$")` | **0 matches** |

Confirms no hardcoded BEAG-name branch.

### String + parser xref re-verification (BINARY-VERIFIED)

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `AirportBound` | 0x0081803C | 0x0041CC6E | **AircraftTypeClass__ReadINI** ← NEW PARSER SCOPE |
| `Fighter` | 0x00818034 | 0x0041CC84 | AircraftTypeClass__ReadINI |
| `InitialAmmo` | 0x00843AEC | 0x00714755 | TechnoTypeClass__ReadINI |

### NEW function entry: `AircraftTypeClass__ReadINI`

| Function | Entry | Body | Status |
|----------|-------|------|--------|
| `AircraftTypeClass__ReadINI` | `0x0041CC20` | `0x0041CC20–0x0041CDA3` | **Fully decompiled this pass**. Sole parser for the 10 AircraftType-scope keys. Calls `TechnoTypeClass__ReadINI(param_2)` first since AircraftType inherits from TechnoType (which itself inherits from ObjectType). Third NEW parser scope discovered (after audit 21 ObjectType + audit 22 BulletType). |

### NEW AircraftType offsets BINARY-VERIFIED (10 entries — first AircraftType-scope audit)

The AircraftType has a small body — only 10 keys parsed (param_1 is plain `int`, direct byte offsets):

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0xDFC` | `Carryall` | byte | Aircraft can carry other units (Carryall transport — TS holdover, dormant in YR) |
| `+0xE00` | `Trailer` | AnimType* | trailing animation behind the aircraft |
| `+0xE04` | `SpawnDelay` | int | delay between consecutive spawn-launches (for spawner-aircraft like Carrier Hornets) |
| `+0xE08` | `Rotors` | byte | helicopter rotor animation flag |
| `+0xE09` | `CustomRotor` | byte | custom rotor sprite override |
| `+0xE0A` | `Landable` | byte | aircraft can land (vs perpetually airborne) |
| `+0xE0B` | `FlyBy` | byte | fly-by attack pattern (vs hover-attack) |
| `+0xE0C` | `FlyBack` | byte | return-to-base behavior |
| `+0xE0D` | `AirportBound` | byte | **the BEAG claim** — must land at airport-class buildings (GAAIRC/AMRADR); crashes if no airport available |
| `+0xE0E` | `Fighter` | byte | **the BEAG claim** — fighter-class flag; affects AI air-vs-air targeting heuristics |

### NEW TechnoType offset BINARY-VERIFIED

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0x680` | `InitialAmmo` | int | `param_1[0x1A0] = iVar4` after ReadInt. **NEW** — initial ammo count at unit spawn (vs Ammo= which is the runtime current ammo). BEAG sets to 1 (single missile). |

### Open question: bare `Ammo` field

The doc raises this — `search_strings("Ammo")` returns only `AmmoCrateDamage` and `InitialAmmo`. The bare `Ammo=` INI key doesn't appear as a standalone string. Possible explanations:
- The parser uses a different `Ammo` constant in a separate data section (DAT_*).
- The string `Ammo` overlaps with a longer string (substring match in parser).
- The field is read directly from a pre-allocated string buffer in the AircraftTypeClass__ReadINI body (the `&DAT_0081BBE0` reference adjacent to the InitialAmmo parse may be the bare "Ammo" string).

DEFERRED — need to read memory at 0x0081BBE0 to confirm.

### Cumulative parser hierarchy (post-audit-26)

| Scope | Address range | Parser entry | Audited |
|-------|---------------|--------------|---------|
| `ObjectTypeClass::ReadINI` | 0x005F9xxx | 0x005F92D0 | audit 7+21 (full) |
| `TechnoTypeClass::ReadINI` | 0x00712-0x00715xxx | — (oversized) | audits 1-26 cumulative |
| `UnitTypeClass::ReadINI` | 0x00747xxx | — | audit 12 (full) |
| `InfantryTypeClass::ReadINI` | 0x00524xxx | 0x005240A0 | audit 13 (full) |
| `BuildingTypeClass::ReadINI_Water` | 0x00460xxx | — | audit 12 |
| `AircraftTypeClass::ReadINI` | **0x0041Cxxx** | **0x0041CC20** | **audit 26 (full) — NEW** |
| `BulletTypeClass::ReadINI` | 0x0046Cxxx | 0x0046BEE0 | audit 22 (full) |
| `WeaponTypeClass::ReadINI` | 0x00772xxx | — | audit 9 |
| `WarheadTypeClass::ReadINI` | 0x0075Dxxx | — | (cheat-sheet only) |
| `RulesClass::Read{General,CombatDamage,AudioVisual,JumpjetControls}` | 0x00669-0x0067Dxxx | various | audits 6+12+17+19 |

### Cross-cumulative re-confirmations

- `RequiredHouses` TechnoType+0xDA0 (audit 10) — doc cite verified.
- `CanPassiveAquire` TechnoType+0xD99 (audit 10) — doc cite verified.
- `PreventAttackMove` TechnoType+0x6C8 (audit 10 + audit 24) — doc cite verified.
- `ImmuneToPsionics` TechnoType+0xD35 (audit 7) — doc cite verified.
- `PenetratesBunker` WarheadType (cheat-sheet) — doc cite trust-chain.

### Items NOT re-verified in this pass (DEFERRED)

- `CanRetaliate=no` exact field scope (doc says "not yet tied to cheat-sheet" — likely TechnoType).
- The bare `Ammo` field parser (DAT_0081BBE0 lookup investigation).
- Aircraft Locomotor CLSID `{4A582746-...}` — used by BEAG, but also by the audit-20 CARRIER Hornets, audit-21 DEST Ospreys (where it's labeled "DriveLocomotionClass-Air" in those doc claims). Note: the doc may have a naming inconsistency — `{4A582746-...}` is the SAME GUID used by Hornet/Osprey aircraft. So the "5th locomotor GUID" framing is technically correct as a separate locomotor class but it's been seen before in audits 20/21.
- The "Alliance = Korea" RequiredHouses semantic (rulesmd-driven country mapping; not Ghidra-relevant).
- AmRADR (American Radar) `Dock=` target — campaign-only building, not directly audited.

### Confidence summary

- **HIGH**: 4 string addresses + 2 parser xrefs (all exact); **1 NEW parser function fully decompiled (AircraftTypeClass__ReadINI) — the third major scope addition (after ObjectType + BulletType)**; 10 NEW AircraftType offsets; 1 NEW TechnoType offset (InitialAmmo +0x680).
- **MEDIUM**: The bare `Ammo` field parser mechanism (DEFERRED — DAT_0081BBE0 investigation needed).
- **No INCORRECT findings**. The doc's "5th locomotor GUID" framing is slightly misleading (the GUID was seen in audits 20+21 for Hornet/Osprey) but the AircraftType scope discovery is genuinely new.

---

## Coverage audit

- [x] Every rulesmd key annotated (~55 keys).
- [x] Every artmd key annotated (4 keys).
- [x] Weapons documented (Maverick2 basic + Maverick2E elite).
- [x] AirToGroundMissile projectile documented.
- [x] ORCAAP warhead documented with PenetratesBunker reference.
- [x] All voice/sound bindings documented including 12-sample
  multi-pool BlackEagleSelect.
- [x] Prerequisites: `RADAR`.
- [x] Owner + RequiredHouses=Alliance (Korea exclusive).
- [x] Veterancy: 4-ability veteran, 3-ability elite (no SELF_HEAL),
  weapon swap with 2× damage + 1.5× range.
- [x] Hardcoded behavior: **AircraftTypeClass__ReadINI scope discovery**,
  AirportBound + Ammo reload cycle, Fighter flag, Korean exclusivity
  via RequiredHouses, triple-disable script-only control, AircraftLocomotion
  GUID.
- [x] TS-legacy filter applied (historical projectile-iteration
  comments + commented AuxSound).
- [x] Comparison with vehicle-class aircraft (Kirov, SHAD).
- [x] At least one Ghidra search performed (4 strings + xrefs).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("AirportBound")` | `0x0081803c` (single match) |
| `get_xrefs_to(0x0081803c)` | `0x0041cc6e → AircraftTypeClass__ReadINI` **(NEW SCOPE)** |
| `search_strings("^Fighter$")` | `0x00818034` (single match) |
| `get_xrefs_to(0x00818034)` | `0x0041cc84 → AircraftTypeClass__ReadINI` |
| `search_strings("RequiredHouses")` | `0x00843bb4` (single match) |
| `get_xrefs_to(0x00843bb4)` | `0x00714529 → TechnoTypeClass__ReadINI` (already in cheat-sheet) |
| `search_strings("Ammo")` | 2 matches — only `AmmoCrateDamage` (Rules) and `InitialAmmo` (TechnoType); bare `Ammo` field NOT a standalone string |
| `get_xrefs_to(0x00843aec)` (InitialAmmo) | `0x00714755 → TechnoTypeClass__ReadINI` |

**New cheat-sheet entries (2) + NEW READINI SCOPE:**
- **`AircraftTypeClass__ReadINI`** (0x0041cxxx range) — NEW scope for
  AircraftTypes-section units.
- `AirportBound` (0x0081803c → 0x0041cc6e) **AircraftType**.
- `Fighter` (0x00818034 → 0x0041cc84) **AircraftType**.
- `InitialAmmo` (0x00843aec → 0x00714755) TechnoType — initial ammo
  count at unit spawn.

**Re-confirmed:**
- `RequiredHouses` TechnoType `0x00843bb4 → 0x00714529`.

**Open questions:**
- The bare `Ammo` field doesn't appear as a standalone string in the
  binary. How is it being read? Possibly:
  1. Combined-string parse (e.g. searching for `Ammo=` directly in the
     INI line buffer).
  2. Hardcoded offset within a TechnoType helper that reads the field
     without using a string match.
  3. Field name stored adjacent to a longer string (substring overlap).
  Open Ghidra trace needed.
- What is the Allied default aircraft for non-Korean houses? `Harrier`
  is the typical RA2 answer but its INI ID needs verification. Open
  follow-up iteration.
- `[ORCA]` mentioned in priority queue — likely cut. Worth confirming.
