---
name: sub-doc
description: SUB — Soviet Typhoon Attack Submarine. Tier-2 single-weapon torpedo sub.
  Strength=600, Cost=1000; SubTorpedo Damage=100 DecloakToFire=no; Elite Burst=2;
  Cloakable+Underwater+Sensors. Pure submarine — simpler than Yuri BSUB (no spawn
  missiles, no LandTargeting beyond minimum).
metadata:
  type: project
---

# SUB — Typhoon Attack Submarine

**INI ID:** `SUB`
**Display:** "Typhoon Attack Sub" (`UIName=Name:SUB`)
**Section:** `[VehicleTypes]`
**Owner side:** Soviet (Russians, Confederation, Africans, Arabs)
**Role:** Soviet tier-2 naval pure-submarine. Stealth anti-naval torpedo
platform — cheaper, faster-to-build, and simpler than Yuri's [BSUB Boomer
Submarine](../yuri/BSUB.md). Closes the Soviet/Yuri submarine pair.

---

## Rulesmd verbatim

```ini
[SUB]
UIName=Name:SUB
Name=Typhoon Attack Sub
Prerequisite=NAYARD
Primary=SubTorpedo
NavalTargeting=5
LandTargeting=1
FireAngle=64
Category=AFV
Strength=600
Naval=yes
Armor=heavy
TechLevel=2
Underwater=yes
Sight=4
Sensors=yes
SensorsSight=7
Speed=4
CrateGoodie=no
Owner=Russians,Confederation,Africans,Arabs
AllowedToStartInMultiplayer=no
Cost=1000
Soylent=1000
Turret=no
Points=30
ROT=2
Crusher=no;gs yes
Crewed=no
Weight=4
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=TyphoonSubSelect
VoiceMove=TyphoonSubMove
VoiceAttack=TyphoonSubAttackCommand
VoiceFeedback=SubFear
DieSound=GenSmallWaterDie
MoveSound=SubMoveStart
Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};{4A582741-9839-11d1-B709-00A024DDAFD1}
SpeedType=Float
MovementZone=Water
ThreatPosed=20	; This value MUST be 0 for all building addons
Accelerates=true
Cloakable=yes
CloakingSpeed=1
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
TooBigToFitUnderBridge=true
ElitePrimary=SubTorpedoE
Size=20
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:SUB` — CSF key. Resolves to "Typhoon Attack Sub".
- `Name=Typhoon Attack Sub` — internal description.
- `Category=AFV` — AI threat-bucket (same as BSUB; Naval=yes routes the
  naval-class checks).

**Tech / availability**
- `Prerequisite=NAYARD` — *only* the Soviet Naval Shipyard required.
  **No RADAR prereq** (unlike BSUB which has `Prerequisite=YAYARD,RADAR`).
  SUB is buildable earlier in the tech tree — *the moment a Soviet player
  finishes their naval yard, they can build subs*.
- `TechLevel=2` — tier-2.
- `Owner=Russians,Confederation,Africans,Arabs` — 4 Soviet sub-factions.
- `AllowedToStartInMultiplayer=no` — not a starting unit.
- `CrateGoodie=no` — not crate-eligible.

**Combat — defense**
- `Strength=600` — **half of BSUB's 1200 HP**. Fragile by comparison.
- `Armor=heavy` — heavy armor, standard for naval combat units.

**Combat — single weapon, anti-naval only**
- `Primary=SubTorpedo` — 100 dmg, ROF=120, Range=7, **NO Burst** (single
  torpedo per salvo, vs BSUB's Burst=2). See "Weapon" section.
- **No `Secondary=`** — *unlike BSUB, the Typhoon Sub has no anti-land
  missile launcher*. Pure anti-naval/anti-underwater.
- `ElitePrimary=SubTorpedoE` — elite swap: Burst=2 added (vs BSUB's
  elite Burst=4). Doubles the single-shot to 200 damage per salvo.
- `NavalTargeting=5` — moderate naval-target priority (vs BSUB's 7,
  more aggressive). Ghidra-verified TechnoType per BSUB doc.
- `LandTargeting=1` — *minimal* land-target priority. **Effectively no
  land-attack capability** — the SUB has no land weapon to fire even if
  it wanted to. Value=1 is the floor.
- `FireAngle=64` — projectile launch angle (vestigial here since the SUB
  has no missile launcher requiring an upward arc; affects only Torpedo
  trajectory which fires near-horizontal anyway).

**Sight / sensors / cloaking**
- `Sight=4` — *short vision range* (vs BSUB's 8). The Typhoon Sub has
  the worst visual scouting of any naval unit. Compensated by sensors.
- `Sensors=yes` — *cloak detection* (TechnoType `0x00843e58 → 0x00714003`).
- `SensorsSight=7` — sensor range (vs BSUB's 8). Slightly shorter cloak
  detection than the Boomer.
- `Cloakable=yes` — **submarines underwater while idle**.
  **Ghidra-verified TechnoType** at `0x00843ea8 → 0x00713f7f`. **NEW
  cheat-sheet entry**. The Cloakable flag toggles the standard cloak
  state machine: idle → fade to invisible, fire/move → temporary
  decloak (unless `DecloakToFire=no` on the weapon).
- `CloakingSpeed=1` — fast cloak transition (1 frame). **Ghidra-verified
  TechnoType** at `0x0084443c → 0x00712441`. **NEW cheat-sheet entry**.
  Compare with Boris (CloakingSpeed=5, slow).

**Mobility**
- `Speed=4` — *very slow* (vs BSUB's 5). The slowest sub in the game.
- `ROT=2` — extremely slow turn rate (matches BSUB).
- `Accelerates=true` — gradual acceleration (subs ramp up; not instant
  like land tanks).
- `Weight=4` — heaviest naval physics weight (same as BSUB).
- `Size=20` — enormous, cannot fit in any transport.
- `TooBigToFitUnderBridge=true` — cannot pass under bridge spans.
- `Underwater=yes` — *renders below water surface*. Only visible to
  Sensors-equipped enemies. Ghidra-verified TechnoType (from BSUB doc
  cheat-sheet, `0x00843848 → 0x00714d74`).
- `SpeedType=Float` — naval speed table (vs BSUB which uses... also Float
  — same).
- `MovementZone=Water` — water-only pathing (vs BSUB same).
- `Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};{4A582741-...}` —
  **Submarine locomotor** (same GUID as BSUB). The `;` separator after
  the primary GUID begins a comment; the second GUID is an *annotation*
  showing the historical Drive locomotor alternative. INI comment-after-
  semicolon behavior — the actual locomotor is the first GUID.

**Economy**
- `Cost=1000` — moderate. Half of BSUB ($2000). The Typhoon Sub is the
  Soviet's main anti-naval workhorse: cheap, mass-producible.
- `Soylent=1000` — full refund on Grinder.
- `Points=30` — moderate score.

**Crew / death**
- `Crewed=no` — no infantry eject. The verbatim `;gs yes` on Crusher is
  the historical override (commented out by Greg Smith); shipped state
  is `Crewed=no`.
- `Crusher=no;gs yes` — *cannot crush*. Same as BSUB.
- `Turret=no` — no turret; fires from hull torpedo tubes.
- `Explosion=TWLT070,...` — explosion anim pool.
- `DieSound=GenSmallWaterDie` — generic small-naval death SFX.
- `MoveSound=SubMoveStart` — **shares the Squid's move-start SFX**
  (`[SubMoveStart]` block plays `vsqumova vsqumovb` — the squid's
  move sounds reused for the sub). Audible underwater rumble.

**Voice / sound bindings**
- `VoiceSelect=TyphoonSubSelect` (5-sample $vsubse* pool).
- `VoiceMove=TyphoonSubMove` (5-sample $vsubmo* pool).
- `VoiceAttack=TyphoonSubAttackCommand` (5-sample $vsubat* pool).
- `VoiceFeedback=SubFear` — the `[SubFear]` block exists in soundmd but
  has **`Volume=0`** and no `Sounds=` line. *Effectively a no-op*. The
  SubFear feedback voice is intentionally silent — subs don't have
  ambient damage chatter. Same convention as BSUB (both naval cloaked
  units use this empty feedback slot).
- `MoveSound=SubMoveStart` (reuses squid SFX).

**Combat behavior**
- `ThreatPosed=20` — same as BSUB.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — 5 abilities
  (including ROF). Same as BSUB.
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — standard.
- Plus the weapon swap to `SubTorpedoE` (Burst=2 — doubles torpedo
  damage per salvo).

**Notable absences (vs BSUB)**
- *No `Spawns=`* — single-weapon platform. No missile launcher.
- *No `Unnatural=yes`* — *the Squid can grab a Typhoon Sub* (vs BSUB
  which gets punched). Soviet players must rely on torpedo range/cloak
  to avoid Squid grapples. **Asymmetric naval-vs-naval interaction
  between sub variants**.
- *No `Bunkerable=no`* line — defaults handle it.
- Smaller everything: HP/2, sight/2, sensors/1, no secondary.

---

## Artmd verbatim

```ini
[SUB]	;soviet submarine
Cameo=SubICON
Voxel=yes
Remapable=yes
PrimaryFireFLH=150,0,0
```

### Key-by-key annotation

- `Cameo=SubICON` — note the case-mixed `SubICON` (vs uppercase
  `BSUBICON`). Sidebar build button.
- `Voxel=yes` — rendered from `sub.vxl` + `sub.hva`.
- `Remapable=yes` — house-color remap.
- `PrimaryFireFLH=150,0,0` — torpedo launch offset:
  - X=150 (forward; torpedo tubes at the bow).
  - Y=0 (*centerline* — unlike BSUB which has Y=65 starboard offset).
  - Z=0 (water-level — torpedoes launch horizontally from the hull).

**No `SecondaryFireFLH`** (no secondary weapon). **No `AltCameo=`**,
**no `SecondSpawnOffset=`** (no Spawner=yes weapon). Minimal art block.

---

## Weapons

### Primary — `[SubTorpedo]`

```ini
[SubTorpedo]
Damage=100
ROF=120
Range=7
Projectile=Torpedo
Speed=25 ;18
Report=SubAttack
Warhead=APSplash
DecloakToFire=no
```

- `Damage=100` — heavy single-shot. **Higher per-shot than BSUB's 60**.
- `ROF=120` — slow (8 sec at 15fps).
- `Range=7` — standard naval range.
- `Projectile=Torpedo` — underwater-tracking projectile (shared with
  BSUB and DLPH).
- `Speed=25` (`;18` historical commented).
- `Report=SubAttack` — fire SFX (`vsubatta`, single sample).
- `Warhead=APSplash` — Armor Piercing + Splash. See warhead block below.
- `DecloakToFire=no` — **fires while cloaked**. Same WeaponType flag as
  BSUB's torpedo (Ghidra-verified WeaponType `0x0084951c → 0x00772121`
  from BSUB doc).
- **No `Burst=`** — *single torpedo per salvo*. Compare BSUB Burst=2.

### Elite — `[SubTorpedoE]`

```ini
[SubTorpedoE]
Damage=100
ROF=120
Range=7
Projectile=Torpedo
Speed=18
Report=SubAttack
Warhead=APSplash
DecloakToFire=no
Burst=2
```

**Two changes vs basic:**
1. `Burst=2` — doubles output (200 dmg/salvo).
2. `Speed=18` (vs 25) — slower projectile. *Same Speed asymmetry pattern
   as BSUB elite torpedo* — Westwood reduced elite-torpedo speed
   (presumably for visual readability of the slower powerful shot).

### Warhead — `[APSplash]`

```ini
[APSplash]; for units whose missiles are having trouble hitting
CellSpread=.5
PercentAtMax=.8
Wall=yes
Wood=yes
Verses=25%,25%,25%,75%,100%,100%,65%,65%,60%,25%,100%
Conventional=yes
InfDeath=3
AnimList=S_CLSN16,S_CLSN22,S_CLSN30,S_CLSN42,S_CLSN58
ProneDamage=50%
```

- The verbatim section comment is informative: "for units whose missiles
  are having trouble hitting" — the splash radius compensates for
  underwater projectile inaccuracy.
- `CellSpread=.5` — half-cell splash.
- `PercentAtMax=.8` — 80% damage at AoE edge (high preserved damage).
- `Wall=yes` / `Wood=yes` — damages walls and wooden buildings.
- `Verses=25%,25%,25%,75%,100%,100%,65%,65%,60%,25%,100%`:
  | Armor    | Multiplier | vs SUB Damage=100 |
  |----------|-----------|---------------------|
  | none     | 25%       | 25 |
  | flak     | 25%       | 25 |
  | plate    | 25%       | 25 |
  | light    | 75%       | 75 |
  | medium   | **100%**  | **100** |
  | heavy    | **100%**  | **100** |
  | wood     | 65%       | 65 |
  | steel    | 65%       | 65 |
  | concrete | 60%       | 60 |
  | special_1 | 25%      | 25 |
  | special_2 | 100%     | 100 |

  **Anti-armor profile**: 100% vs medium/heavy/special_2. Strong vs
  Destroyers, Aircraft Carriers, Dreadnoughts (all heavy armor naval).
  Weak vs infantry/light units (25%) — but torpedoes can't typically
  hit infantry anyway.
- `Conventional=yes` — conventional damage type.
- `InfDeath=3` — explosion infantry death.
- `AnimList=S_CLSN16...S_CLSN58` — 5-anim collision-explosion pool.

Compare with BSUB's `APSplash2`: same CellSpread/PercentAtMax, but
APSplash2 has Verses=100%,100%,100%,75%,... — *stronger vs unarmored
infantry/flak/plate* than basic APSplash. The BSUB's torpedo damage is
60 but with better Verses; the SUB's is 100 with poorer infantry-side
Verses. Net DPS vs naval is similar.

### Projectile — `[Torpedo]`

```ini
[Torpedo]
Arm=2
Shadow=no
;Proximity=yes
Ranged=yes
Image=SUBT
ROT=12 ;4
AA=no
;AN=yes
AG=yes
;AS=yes
Level=yes
```

- `Arm=2` — 2-frame arming delay (prevents instant-detonation right
  after launch).
- `Shadow=no` — no shadow rendered (it's underwater).
- `Ranged=yes` — *projectile has a max distance check* — disarms if
  it overshoots.
- `Image=SUBT` — torpedo SHP (`subt.shp` — the visible underwater wake).
- `ROT=12` (`;4` historical) — Rate Of Turn for tracking the target.
  ROT=12 is fast — the torpedo can track maneuvering vessels.
- `AA=no` — *not anti-air*. Can't shoot down aircraft.
- `AG=yes` — *anti-ground*. Can hit naval/land targets on water-surface
  contact.
- `Level=yes` — flies level (no arc). Torpedoes don't arc — they swim
  horizontally toward target.

### Notable — no `Inviso=yes`, no `Vertical=yes`

The Torpedo is *visible* (the wake), unlike Boomer's missile projectile
which is `Inviso=yes`. Players see torpedoes incoming and can sometimes
dodge them with quick maneuvering.

---

## Voices / sounds

All from `soundmd.ini`:

```ini
[TyphoonSubSelect]
Sounds=$vsubsea $vsubseb $vsubsec $vsubsed $vsubsee
Control=random
Volume=85

[TyphoonSubMove]
Sounds=$vsubmoa $vsubmob $vsubmoc $vsubmod $vsubmoe
Control=random
Volume=85

[TyphoonSubAttackCommand]
Sounds=$vsubata $vsubatb $vsubatc $vsubatd $vsubate
Control=random
Volume=85

[SubAttack]
Sounds= vsubatta
FShift= -10 10
Volume=50

[SubMoveStart]
Sounds= vsqumova vsqumovb
Control= random
FShift= -10 10
Volume=45

[SubFear]
Volume=0	; no sound
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=TyphoonSubSelect` | `[TyphoonSubSelect]` | Click |
| `VoiceMove=TyphoonSubMove` | `[TyphoonSubMove]` | Move order |
| `VoiceAttack=TyphoonSubAttackCommand` | `[TyphoonSubAttackCommand]` | Attack order |
| `VoiceFeedback=SubFear` | `[SubFear]` | **silent block** (Volume=0, no Sounds=). Intentional no-op. |
| `Report=SubAttack` (weapon) | `[SubAttack]` | Fire SFX (`vsubatta`, single sample) |
| `DieSound=GenSmallWaterDie` | shared | Death |
| `MoveSound=SubMoveStart` | `[SubMoveStart]` | **reuses Giant Squid move SFX** (`vsqumova vsqumovb`). Underwater ambient rumble. |

**`SubFear` silent-block convention**: when a unit needs a `VoiceFeedback`
slot for the rules-side validation but no audible feedback is wanted,
the convention is to define an empty `Volume=0` block rather than leaving
the rules-side key empty. Both `SUB` and `BSUB` use this trick.

**`SubMoveStart` borrowing Squid audio**: rather than commission separate
sub-ignition SFX, Westwood reused the Giant Squid's `vsqumov` move samples
for both the Typhoon Sub *and* the Squid itself. Standardizes the
underwater-creature audio profile.

---

## Hardcoded behavior (Ghidra-verified)

### 1. Cloakable=yes + CloakingSpeed=1

`Cloakable=yes` (TechnoType `0x00843ea8 → 0x00713f7f`, **NEW** this
iteration) triggers the cloak state machine. The unit is invisible to
non-Sensors-equipped enemies while not currently firing/moving.

`CloakingSpeed=1` (TechnoType `0x0084443c → 0x00712441`, **NEW** this
iteration) — *fast cloak transition*. The fade-to-invisible takes 1
frame. Comparison points:
- SUB / BSUB: CloakingSpeed=1 (instant cloak).
- DLPH (Dolphin): not Cloakable.
- Boris: CloakingSpeed=5 (slow cloak).
- Stealth Generator buildings: configured differently.

### 2. DecloakToFire=no on torpedoes

The standard cloak state machine *decloaks the unit briefly while
firing*. `DecloakToFire=no` on a weapon (WeaponType `0x0084951c →
0x00772121` per BSUB doc) overrides this — the unit stays cloaked
during fire. Both SUB and BSUB exploit this for stealth attack patterns.

The Typhoon Sub fires from cloak — enemies see torpedoes spawning from
"nowhere" until they bring a sensor unit nearby.

### 3. Underwater + Sensors mutual visibility

`Underwater=yes` (per BSUB doc TechnoType `0x00843848`) renders the unit
below water surface. Without sensor-equipped enemy units, the SUB is
*completely invisible* (cloak + underwater render).

The SUB's `Sensors=yes SensorsSight=7` lets it detect *other* cloaked
subs at 7-cell range. Sub-vs-sub naval combat is fundamentally sensor-
range based: whoever spots first gets the first torpedo salvo. SUB's
range-7 is shorter than BSUB's range-8 — Boomer Sub spots Typhoon Sub
1 cell before Typhoon spots Boomer.

### 4. Naval=yes (per BSUB doc cheat-sheet)

TechnoType `0x0084395c → 0x00714a6a`. Marks the unit as naval-class:
- Built at Shipyard.
- Squid can grab (or punch if Unnatural=yes — SUB does NOT have this).
- Vulnerable to Torpedo projectiles.

### 5. NavalTargeting=5 / LandTargeting=1

`NavalTargeting=5` (TechnoType `0x00844510 → 0x007121be`) — moderate
priority to engage naval targets. `LandTargeting=1` (TechnoType
`0x00844520 → 0x007121a4`) — minimum priority. The SUB *will* try to
auto-engage land targets if specifically ordered (force-fire), but its
auto-target scan barely considers them — and the torpedo's water-bound
trajectory makes most land attacks impractical anyway.

### 6. Single weapon, no spawn manager

Unlike BSUB (Spawn manager at TechnoClass+0x2D8 driving CMISL spawn-
missiles), SUB has no spawn system. The TechnoClass+0x2D8 slot is unused
for SUB. Simpler internal state.

### 7. Sub locomotor

GUID `{2BEA74E1-7CCA-11d3-BE14-00104B62A16C}` — Submarine locomotor.
Same as BSUB. The locomotor handles:
- Underwater movement physics (no surface skating).
- Cloak-state-aware visibility toggling.
- Sub-surface depth maintenance.

---

## TS-legacy filter

- The `Locomotor=...;{4A582741-...}` annotation after the semicolon is a
  comment showing the historical Drive locomotor alternative. Same
  pattern as BSUB.
- `Crusher=no;gs yes` — commented historical override.
- `Primary=Torpedo;...` — none here; SUB's primary is named directly.
- `Speed=25;18` on weapon — historical projectile speed.
- No `ImmuneToVeins`, no `Subterranean`, no other TS-only fields.

**TS reference**: Submarines existed in Tiberian Sun (TS Naval was
heavily cut from final TS, but the code is inherited). Most of the
locomotor + cloak machinery dates back to TS. **YR-active mechanism**:
the submarine + cloak system is fully live in YR (BSUB uses it too).

---

## Comparison: SUB vs BSUB (the submarine pair)

| Field | SUB (Soviet) | BSUB (Yuri) |
|-------|--------------|-------------|
| Strength | **600** | 1200 |
| Cost | **1000** | 2000 |
| Speed | 4 | **5** |
| Sight | 4 | **8** |
| SensorsSight | 7 | **8** |
| TechLevel | 2 | 2 |
| Prereq | NAYARD | YAYARD,**RADAR** |
| Primary Damage | **100** | 60 |
| Primary Burst | 1 | **2** |
| Primary Range | 7 | 7 |
| Primary Warhead | APSplash | APSplash2 |
| Elite Burst | 2 | **4** |
| **Secondary weapon** | **none** | **CruiseLauncher (Range=20)** |
| **Spawns** | none | **CMISL ×2** |
| Cloakable | yes | yes |
| CloakingSpeed | 1 | 1 |
| Underwater | yes | yes |
| Unnatural | not set | **yes** (Squid punches instead of grabs) |
| Sensors | yes | yes |

**Trade-offs:**
- **SUB**: Higher per-shot damage (100), simpler, cheaper. Naval-only.
  Vulnerable to Squid grab.
- **BSUB**: Higher HP (2× SUB), more burst-throughput (Burst=2 doubles
  per-salvo), adds anti-land cruise missiles, immune to Squid grab.
  Twice the cost and requires RADAR prereq.

**Per-DPS comparison vs heavy-armor naval target:**
- SUB SubTorpedo: 100 / 120 ROF = 0.83 dps × 100% Verses = 0.83 dps.
- BSUB BoomerTorpedo: 60 × 2 / 120 ROF = 1.00 dps × 100% Verses = 1.00 dps.
- **BSUB is 20% higher DPS** despite lower per-shot damage, due to Burst=2.

**The Typhoon Sub is the volume option; the Boomer Sub is the specialist
option.** A Soviet player builds 6 SUBs for $6000; a Yuri player builds
3 BSUBs for $6000. The Soviet's 6 subs have 6× 100 dmg in spread fire;
the Yuri's 3 boomers have 3× (60×2)=360 dmg, plus 3× 25 cruise missile
land-strike capability. The Yuri solution is more strategic; the Soviet
solution is more brute-force.

---

## Cross-references

- [BSUB.md](../yuri/BSUB.md) — Yuri counterpart submarine. Different
  trade-offs but shares many mechanics (Cloakable, Underwater,
  Sub locomotor).
- [DLPH.md](../allied/DLPH.md) — pending. Allied counter to subs
  (sonic-pulse anti-submarine).
- [DEST.md](../allied/DEST.md) — already done. Allied destroyer with
  Sensors=yes for sub-detection.
- [SQD.md](../soviet/SQD.md) — pending. Giant Squid mind-ranking sub
  killer.
- [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
  — for contrast (SUB has no spawn manager; BSUB does).

---

## Coverage audit

- [x] Every rulesmd key annotated (~45 keys).
- [x] Every artmd key annotated (4 keys).
- [x] Single weapon (SubTorpedo basic + Elite SubTorpedoE).
- [x] Torpedo projectile and APSplash warhead documented.
- [x] All voice/sound bindings documented including silent `[SubFear]`.
- [x] Prerequisites: `NAYARD` (no RADAR — earliest sub in the game).
- [x] Owner: 4 Soviet sub-factions.
- [x] Veterancy: extended VeteranAbilities (5 incl. ROF), Burst=2 elite
  swap.
- [x] Hardcoded behavior: Cloakable + CloakingSpeed (both NEW
  cheat-sheet), DecloakToFire (re-confirmed from BSUB), Sub locomotor,
  Underwater + Sensors interactions.
- [x] TS-legacy filter applied (no active TS code; commented historical
  lines).
- [x] Comparison table with peer submarine BSUB (the pair closer).
- [x] At least one Ghidra search performed (Cloakable, CloakingSpeed
  both new).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("^Cloakable$")` | `0x00843ea8` (single match) |
| `get_xrefs_to(0x00843ea8)` | `0x00713f7f → TechnoTypeClass__ReadINI` |
| `search_strings("CloakingSpeed")` | `0x0084443c` (single match) |
| `get_xrefs_to(0x0084443c)` | `0x00712441 → TechnoTypeClass__ReadINI` |

**New cheat-sheet entries (2):**
- `Cloakable` (0x00843ea8 → 0x00713f7f) TechnoType — toggles the cloak
  state machine.
- `CloakingSpeed` (0x0084443c → 0x00712441) TechnoType — frame count for
  cloak transition (1=instant, higher=slower).

**Re-confirmed from prior cheat-sheet:**
- `Underwater` (0x00843848) TechnoType — from BSUB doc.
- `DecloakToFire` (0x0084951c) WeaponType — from BSUB doc.
- `Naval` (0x0084395c) TechnoType — from SAPC doc.
- `Sensors` (0x00843e58) TechnoType — from DEST doc.
- `SensorsSight` (0x00843d50) TechnoType — from DEST doc.

**Soviet/Yuri submarine pair closed**: SUB ✓ + BSUB ✓.

**Open questions:**
- The `[SubMoveStart]` audio re-use from Squid — is this intentional
  Westwood design or a placeholder that never got replaced? Hard to
  tell. Doesn't affect functionality.
- The `[SubFear]` silent-block convention — does the engine emit any
  log warning when a zero-volume voice block is referenced? Likely
  ignored silently.
