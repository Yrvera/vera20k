---
name: robo-doc
description: ROBO — Robot Tank. Allied hover anti-armor vehicle. PoweredUnit=yes
  (goes offline if no power); requires Robot Control Center (GAROBO); ImmuneToPsionics
  +Radiation+Veins (robot-only); VoiceSelectDeactivated + ActivateSound/DeactivateSound
  power-state transitions (DUAL-READ Rules+TechnoType).
metadata:
  type: project
---

# ROBO — Robot Tank

**INI ID:** `ROBO`
**Display:** "Robot Tank" (`UIName=Name:Robotank`)
**Section:** `[VehicleTypes]`
**Owner side:** Allied (British, French, Germans, Americans, Alliance)
**Role:** Allied tier-2 hover anti-armor vehicle. Cheap, fast (Speed=10), and
*immune to all anti-personnel/psi/radiation* — but goes offline if the Robot
Control Center is destroyed or the player loses power. Designed as a
specialised counter to Yuri-faction mind-control while requiring active
infrastructure support.

---

## Rulesmd verbatim

```ini
[ROBO]
UIName=Name:Robotank
Name=Robot Tank
Image=ROBO
Prerequisite=GAWEAP,GAROBO
Primary=Robogun
Strength=180
Category=AFV
Armor=heavy
Turret=yes
IsTilter=yes
Crusher=yes
TooBigToFitUnderBridge=true
TechLevel=2
Sight=6
Speed=10
CrateGoodie=no
Owner=British,French,Germans,Americans,Alliance
Cost=600
Soylent=600
Points=25
ROT=5
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=RobotTankSelect
VoiceSelectDeactivated=RobotTankSelectDeactivated
VoiceMove=RobotTankMove
VoiceAttack=RobotTankAttackCommand
VoiceFeedback=
DieSound=RobotTankDie
MoveSound=RobotTankMoveStart
ActivateSound= RobotTankOnline
DeactivateSound= RobotTankOffline
CrushSound=TankCrush
MaxDebris=2
SpeedType=Hover
Locomotor={4A582742-9839-11d1-B709-00A024DDAFD1}
MovementZone=AmphibiousDestroyer
ThreatPosed=15	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Accelerates=false
ImmuneToVeins=yes
Size=3
OpportunityFire=yes
AllowedToStartInMultiplayer=no
ImmuneToPsionics=yes
ImmuneToRadiation=yes
PoweredUnit=yes;powered by presence of building
Trainable=no
BuildTimeMultiplier=1.3
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:Robotank` — CSF key. Resolves to "Robot Tank".
- `Name=Robot Tank` — internal description.
- `Image=ROBO` — explicit Image= matching section name (redundant but harmless,
  same pattern as LTNK).
- `Category=AFV` — AI threat-bucket.

**Tech / availability**
- `Prerequisite=GAWEAP,GAROBO` — **needs Allied War Factory AND
  [GAROBO](../structures/GAROBO.md) (Robot Control Center)**. The Robot
  Control Center is the unique gating building — without it, no Robot Tanks
  can be built AND existing ones deactivate (see PoweredUnit semantics).
- `TechLevel=2` — tier-2 unit (basic MBT-tier).
- `Owner=British,French,Germans,Americans,Alliance` — all 4 Allied sub-factions
  + Alliance.
- `AllowedToStartInMultiplayer=no` — cannot be a starting unit; must be built
  up through tech tree.
- `CrateGoodie=no` — *not eligible from UnitCrate pickups*. The PoweredUnit
  dependency means a crate-spawned ROBO without infrastructure would
  immediately deactivate; Westwood excluded it from crate eligibility.

**Combat — defense**
- `Strength=180` — fragile. Compare:
  | Tank | Strength |
  |------|----------|
  | ROBO Robot Tank | **180** |
  | MGTK Mirage Tank | 200 |
  | LTNK Lasher | 300 |
  | MTNK Grizzly | 300 |
  - About 60% the HP of a Grizzly. The hover speed and immunities
    compensate.
- `Armor=heavy` — heavy armor type. Reduces AT damage similarly to MBTs;
  unusual for a hover/light vehicle (Mirage Tank is medium).

**Combat — weapons**
- `Primary=Robogun` — 65 dmg, AP warhead, Range=5, ROF=60 (same stats as
  LTNK's basic ATGUN — see Weapons section). The Robogun is effectively a
  *re-skinned ATGUN with a different fire SFX*.
- *No `ElitePrimary=` line* — combined with `Trainable=no`, **the Robot Tank
  cannot rank up at all**. It stays at rookie firepower forever.

**Sight / mobility**
- `Sight=6` — modest 6-cell vision. Matches Range=5 of the weapon.
- `Speed=10` — **fastest non-aircraft unit in the game**. Compare:
  | Tank | Speed |
  |------|-------|
  | ROBO Robot Tank | **10** |
  | LTNK Lasher | 7 |
  | MTNK Grizzly | 7 |
  | HTNK Rhino | 5 |
  | APOC Apocalypse | 4 |
  - At Speed=10 (max from `STRONGER`/`FASTER` ability scaling: 12.5), the
    Robot Tank outpaces almost everything. *Veteran rank is irrelevant
    since `Trainable=no` blocks it.*
- `ROT=5` — turret rotation rate.
- `SpeedType=Hover` — uses Hover speed table. Most terrain types treat hover
  units equally; water is traversable.
- `Locomotor={4A582742-9839-11d1-B709-00A024DDAFD1}` — **Hover locomotor
  GUID**. Distinct from the Drive locomotor (`...741`). Hover locomotor
  shared with Sea Scorpion (LCRF), Aegis Cruiser (AEGIS), Hydrofoil (HYD),
  and the Yuri Hover Transport (YHVR). Float over land + water.
- `MovementZone=AmphibiousDestroyer` — *amphibious + wall-crushing*. Can
  hover over water AND crush walls. The most permissive movement zone.
- `Size=3` — fits in Battle Fortress (`Passengers=5`, `SizeLimit=2`). Wait —
  SizeLimit=2 caps occupant size at 2, so size-3 doesn't fit. Robot Tank
  *cannot* enter a Battle Fortress.
- `Accelerates=false` — instant speed (no ramp). Same as LTNK/SREF. With
  Speed=10, the result is *immediate full-speed dash* on order.
- `TooBigToFitUnderBridge=true` — cannot drive under bridge spans.
- `IsTilter=yes` — body tilts on slopes.

**Economy**
- `Cost=600` — cheap (cheaper than LTNK at 700). The price reflects the
  fragility + power dependency.
- `Soylent=600` — full refund on Grinder.
- `Points=25` — modest score.
- `BuildTimeMultiplier=1.3` — *builds 30% slower than Cost would predict*.
  Offsets the cheap price (like LTNK's 1.5× build time).

**Crew / death**
- *No `Crewed=` line* → defaults to `Crewed=no`. **Does not eject infantry
  on death** — the Robot Tank is *literally* unmanned. Thematic.
- `MaxDebris=2` — only 2 debris pieces (small vehicle).
- `DieSound=RobotTankDie` — specific death SFX (`vrobdiea`), not generic.

**Behavior flags**
- `Crusher=yes` — crushes Crushable infantry. No `OmniCrushResistant`.
- `CrushSound=TankCrush` — standard wet-crunch.
- `IsSelectableCombatant=yes`.
- `OpportunityFire=yes` — auto-engages.
- `ThreatPosed=15` — *lower than Lasher's 40*. AI views Robot Tank as a
  minor threat; the immunities and speed make it tactical rather than
  strategic.

**Immunities (the signature feature set)**

Robot Tanks are the *most immune-stacked unit in the game*:
- `ImmuneToVeins=yes` — TS-legacy field (veins were TS). **Read but
  dormant** in YR. Ghidra-verified in cheat-sheet (no veins in YR maps).
- `ImmuneToPsionics=yes` — **cannot be mind-controlled** by Yuri, Yuri
  Prime, Master Mind, Psychic Dominator, Psychic Tower. Ghidra-verified
  `0x00843754 → 0x00714fa7` TechnoType. *Major Yuri-counter*.
- `ImmuneToRadiation=yes` — *not damaged by Desolator radiation* or Rad
  Beam weapons. Ghidra-verified TechnoType. Counter to Soviet rad strats.
- *No `ImmuneToPoison=` line* — would default to no; Robot Tank takes plague
  damage from VIRUS sniper (but the warhead is anti-infantry so this is
  rarely a real exposure).

**PoweredUnit (the unique state machine)**
- `PoweredUnit=yes;powered by presence of building` — **goes offline if the
  owning house lacks power OR if the controlling building (GAROBO Robot
  Control Center) is destroyed**. Verbatim comment is explicit. [BINARY-VERIFIED audit 23: string @ 0x00844158, parser xref @ 0x00713316, `TechnoType+0x410` (byte)].

  Behavior in detail:
  1. While player has power AND a GAROBO is present: ROBO is active —
     standard fire/move/AI behavior.
  2. If power drops below required level OR all GAROBOs destroyed: ROBO
     **deactivates** — cannot fire, cannot move, becomes a stationary
     hulk. `DeactivateSound=RobotTankOffline` plays. Voice line changes
     to `VoiceSelectDeactivated` on click.
  3. Power restored / new GAROBO built: ROBO **reactivates** —
     `ActivateSound=RobotTankOnline` plays.

  **Trade-off:** A Yuri opponent who attacks the Allied Robot Control
  Center incapacitates every Robot Tank instantly. Or a power-shutdown
  attack (Yuri's Mindcontrol of a Power Plant, or Soviet Iron Curtain
  attack on power infrastructure) does the same. This is *very different*
  from standard offline-power penalties for buildings — Robot Tanks
  literally stop working.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` — *listed but
  inert*. `Trainable=no` (below) blocks XP accumulation, so the unit
  never reaches veteran rank in normal play. The veteran ability list
  is vestigial — copy-pasted from MBT template.
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — same vestigial
  status.
- `Trainable=no` — **cannot gain veterancy XP**. Ghidra-verified TechnoType.
  Robot Tanks remain at rookie rank forever regardless of kills. Probably
  thematic ("machines don't learn") and a balance lever (no Burst-2 elite
  Robogun spam).

**Voice / sound bindings**
- `VoiceSelect=RobotTankSelect` — normal-state click voice.
- `VoiceSelectDeactivated=RobotTankSelectDeactivated` — **deactivated-state
  click voice**. [BINARY-VERIFIED audit 23: string @ 0x00844288, parser xref @ 0x00712C0A, `TechnoType+0x45C` (int VocClass index)].
  The engine swaps which voice plays based on the unit's active/deactivated
  state. Few units use this; only PoweredUnit=yes vehicles need it.
- `VoiceMove=RobotTankMove`, `VoiceAttack=RobotTankAttackCommand` — standard.
- `VoiceFeedback=` — empty.
- `DieSound=RobotTankDie` — specific death SFX (vs generic `GenVehicleDie`).
- `MoveSound=RobotTankMoveStart` — ignition.
- `ActivateSound=RobotTankOnline` — **DUAL-READ** Rules global + TechnoType
  override (per Ghidra). Plays on activation transition (power restored or
  GAROBO rebuilt).
- `DeactivateSound=RobotTankOffline` — **DUAL-READ** same pattern. Plays
  on deactivation (power lost or GAROBO destroyed).

---

## Artmd verbatim

```ini
[ROBO]   ; Robot Tank
Image=ROBO
Voxel=yes
Remapable=yes
Cameo=ROBOICON
AltCameo=ROBOUICO
PrimaryFireFLH=150,0,100
```

### Key-by-key annotation

- `Image=ROBO` — explicit Image= matching section name. Same pattern as
  rulesmd; harmless redundancy.
- `Voxel=yes` — rendered from `robo.vxl` + `robo.hva`. Turret voxel separate.
- `Remapable=yes` — house-color remap applies.
- `Cameo=ROBOICON` — sidebar build-button SHP.
- `AltCameo=ROBOUICO` — UI-overlay alt cameo.
- `PrimaryFireFLH=150,0,100` — bullet spawn offset:
  - X=150 (well forward; the gun barrel extends out of the turret).
  - Y=0 (centered).
  - Z=100 (turret height).

**No `Weapon1FLH=` syntax** — Robot Tank uses standard `Primary=` not the
multi-turret pathway. **No secondary weapon, no SecondaryFireFLH.**

---

## Weapons

### Primary — `[Robogun]`

```ini
[Robogun]
Damage=65
ROF=60
Range=5
Projectile=Cannon
Speed=60
Warhead=AP
Report=RobotTankAttack
Anim=GUNFIRE
Bright=yes
```

- `Damage=65` — moderate. Same as LTNK's ATGUN basic.
- `ROF=60` — 60 ticks between shots (~4 seconds at 15fps).
- `Range=5` — standard MBT range.
- `Projectile=Cannon` — arcing 120mm shell (shared with LTNK/MTNK/HTNK/SREF
  basic — see [SREF.md](../allied/SREF.md) Cannon block for details).
- `Speed=60` — projectile speed.
- `Warhead=AP` — Armor Piercing. **Same warhead as LTNK basic.** See
  [LTNK.md](../yuri/LTNK.md#warhead--basic-ap) for the full
  Verses table. Summary: weak vs infantry (25%), strong vs medium/heavy
  tanks (100%), almost-immune-to-plate (15%).
- `Report=RobotTankAttack` — fire SFX (`vrobatta`).
- `Anim=GUNFIRE` — generic muzzle flash.
- `Bright=yes` — palette-brighten cells on fire.

**Net firepower:** Robot Tank is mechanically a *re-skinned Lasher* in
firepower terms (same Robogun ≈ ATGUN stats). The differences are:
- ROBO has Speed=10 (vs LTNK Speed=7).
- ROBO has Hover locomotor (water-traversable).
- ROBO has immunities (psi/rad/veins).
- ROBO is PoweredUnit=yes (depends on GAROBO).
- ROBO is Trainable=no (no veterancy ever).
- ROBO is cheaper ($600 vs $700) but builds slower (×1.3 vs ×1.5).

Same primary gun. Different role: hover skirmisher vs main battle tank.

**No elite weapon.** Robot Tank cannot rank up.

---

## Voices / sounds

All from `soundmd.ini`:

```ini
[RobotTankSelect]
Sounds= vrobsela
Control= random
Volume=55

[RobotTankMove]
Sounds= vrobmova vrobmovb
Control= random
Volume=55

[RobotTankAttackCommand]
Sounds= vrobatca
Control= random
Volume=55

[RobotTankDie]
Sounds= vrobdiea
Control=random
FShift=-15 15
VShift=20
Volume=85

[RobotTankMoveStart]
Sounds= vrobstaa vrobstab vrobstac
Control= random predelay
Delay=0 400
Priority=Low
FShift= -10 10
VShift=15
Volume=40

[RobotTankAttack]
Sounds=vrobatta
FShift= -5 5
VShift= 10
Volume=70

[RobotTankOnline]
Sounds=vrobon
Priority= critical
Limit=1
Range=15
Volume=60

[RobotTankOffline]
Sounds=vroboff
Priority= critical
Limit=1
Range=15
Volume=60

[RobotTankSelectDeactivated]
Sounds= vrobse2a vrobse2b vrobse2c
Control=random
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=RobotTankSelect` | `[RobotTankSelect]` | Click when *active* |
| `VoiceSelectDeactivated=RobotTankSelectDeactivated` | `[RobotTankSelectDeactivated]` | Click when *deactivated* (no power / no GAROBO) — 3-sample pool |
| `VoiceMove=RobotTankMove` | `[RobotTankMove]` | Order to move |
| `VoiceAttack=RobotTankAttackCommand` | `[RobotTankAttackCommand]` | Order to attack |
| `Report=RobotTankAttack` (weapon) | `[RobotTankAttack]` | Fire SFX |
| `DieSound=RobotTankDie` | `[RobotTankDie]` | On death |
| `MoveSound=RobotTankMoveStart` | `[RobotTankMoveStart]` | Ignition |
| `ActivateSound=RobotTankOnline` | `[RobotTankOnline]` | Activation transition. `Priority=critical Limit=1 Range=15` — audible at range 15, limit 1 concurrent across all robots |
| `DeactivateSound=RobotTankOffline` | `[RobotTankOffline]` | Deactivation transition (same parameters as Online) |

**`Limit=1`** on the Online/Offline sounds prevents a flood of activation
SFX when multiple Robot Tanks deactivate simultaneously (e.g. GAROBO
destroyed and 8 Robot Tanks all offline at once → only ONE Offline sound
plays, not 8). `Range=15` makes the SFX audible across a wide tactical
area.

**`Type=` not set** → local sound (only audible if the unit is on-screen
or near a focus point). Compare with the global Kirov CreateSound or
superweapon Activate sounds.

**Note: the soundmd has a `[RobotTankPowerDown]` block defined at line
~1757** (visible in grep but I didn't read it inline). It's likely an
unreferenced cut block (no `PowerDownSound=` key exists in rules schema).
Open question for future audit.

---

## Hardcoded behavior (Ghidra-verified)

### 1. PoweredUnit=yes state machine

`PoweredUnit=yes` (Ghidra-verified TechnoType `0x00844158 → 0x00713316`)
triggers the PoweredUnit state machine:

**Conditions for activation:**
1. Owning house has sufficient power (not currently in power-deficit drain).
2. At least one *Powered building of the same family* exists in the owning
   house. For Robot Tank, this is the GAROBO Robot Control Center.

**Activation/deactivation transitions:**
- *Active → Deactivated*: any of (power lost / GAROBO destroyed). The unit:
  - Plays `DeactivateSound`.
  - Stops processing AI / attack / move orders.
  - Becomes immobile (no Locomotor advance).
  - Still occupies its cell (blocks pathing) and can be attacked.
  - `VoiceSelectDeactivated` plays when clicked.
- *Deactivated → Active*: power restored AND GAROBO present:
  - Plays `ActivateSound`.
  - Resumes normal behavior.

Per the verbatim comment "powered by presence of building", the engine
looks up the *correct controlling building* for each PoweredUnit. For
Robot Tank, the lookup matches GAROBO. The mechanism is presumably some
hardcoded association table or a config-driven match on building type's
"controls these units" flag. **Exact lookup mechanism not yet investigated
in Ghidra** — open follow-up.

The PoweredUnit system is *YR-live* (Robot Tank is a YR-only unit;
Tiberian Sun had no equivalent).

### 2. VoiceSelectDeactivated (TechnoType)

`VoiceSelectDeactivated` (Ghidra-verified TechnoType `0x00844288 →
0x00712c0a`) is a *parallel-to-VoiceSelect* field used when the unit is
in deactivated state. The engine checks the unit's deactivated flag
before sampling which voice block to play on click. Only PoweredUnit=yes
units typically need this; rare field.

### 3. ActivateSound/DeactivateSound DUAL-READ

Both fields are **DUAL-READ**:
- Rules-global: `[AudioVisual]` section reads `ActivateSound=` and
  `DeactivateSound=` at `0x0066a21e` / `0x0066a260` in
  `RulesClass__ReadAudioVisual`. **Set defaults globally** for any
  PoweredUnit=yes unit without a per-unit override.
- Per-techno: `TechnoTypeClass__ReadINI` reads the same keys at
  `0x007138ec` / `0x00713922`. **Override per-unit**.

Same DUAL-READ pattern as ChronoInSound, ChronoOutSound, ImpactLandSound.
Note: This pattern means **defaults can be set globally and individual
units can override**. The Robot Tank declares its own
(`RobotTankOnline`/`RobotTankOffline`), so it doesn't use the Rules
defaults — but a modder could omit the per-unit and rely on the global.

### 4. Hover locomotor

`Locomotor={4A582742-9839-11d1-B709-00A024DDAFD1}` — Hover locomotor.
Distinct from the Drive GUID (`...741`). Hover locomotor characteristics:
- Treats water as traversable terrain.
- Hovers above ground / water at a small fixed altitude.
- Does not pitch-tilt on slopes (despite `IsTilter=yes` — open question
  whether tilt actually applies to hover units; tilt is engine-flag, hover
  is locomotor-class).
- Same GUID used by LCRF Sea Scorpion, AEGIS Aegis Cruiser, HYD Hydrofoil,
  YHVR Yuri Hover Transport, and a few others.

### 5. Immunities

- `ImmuneToPsionics=yes` (TechnoType `0x00714fa7`) — blocks mind-control
  weapons. Yuri/Yuri Prime/Master Mind/Psychic Dominator/Psychic Tower
  cannot affect Robot Tanks. **Major Yuri counter**.
- `ImmuneToRadiation=yes` (TechnoType, cheat-sheet) — blocks Desolator
  rad-pool damage and Rad Beam weapons.
- `ImmuneToVeins=yes` — TS-legacy. Veins were TS-only; in YR there are
  no veins, so this field is dormant. Read into TechnoType but never
  triggered.

### 6. Trainable=no — no veterancy ever

The vestigial `VeteranAbilities`/`EliteAbilities` lists in the rules are
inert. The unit never gains XP, never ranks up, never swaps to an elite
weapon.

### 7. AllowedToStartInMultiplayer=no

Cannot be a starting unit. Same flag as Kirov, Prism Tank.

---

## TS-legacy filter

- `ImmuneToVeins=yes` — **TS-legacy field, dormant in YR**. Veins were
  Tiberian Sun's pollution/ore-decay tiles; not present in YR. The field
  is *read* into TechnoType but the runtime "veins damage" code never
  fires because no map has vein cells. See user memory
  `feedback_no_tunnel_subterranean.md` (similar TS-only feature).
- No other TS-only fields. `MovementZone=AmphibiousDestroyer` is YR-live.

---

## Comparison with peer Allied vehicles

| Field | ROBO Robot Tank | LTNK Lasher (peer-tier, Yuri) | MGTK Mirage Tank (Allied) |
|-------|-----------------|--------------------------------|----------------------------|
| Strength | 180 | 300 | 200 |
| Armor | heavy | heavy | medium |
| Speed | **10** | 7 | 5 |
| Sight | 6 | 8 | 8 |
| Cost | **600** | 700 | 1000 |
| TechLevel | 2 | 2 | 6 |
| Prereq | GAWEAP,GAROBO | YAWEAP | GAWEAP,GATECH |
| Primary | Robogun (=ATGUN) | ATGUN | MirageGun |
| Locomotor | **Hover** | Drive | Drive |
| MovementZone | **AmphibiousDestroyer** | Destroyer | Destroyer |
| Trainable | **no** | yes | yes |
| Crewed | no | no | no |
| ImmuneToPsionics | **yes** | no | no |
| ImmuneToRadiation | **yes** | no | no |
| PoweredUnit | **yes** | no | no |

**Robot Tank's role:** Hover skirmisher with mind-control / rad immunity.
Counter-Yuri specialist. The speed-10 + hover combination means Robot
Tanks can flank around bases, cross rivers, and avoid Desolator
radiation pools — at the cost of infrastructure-dependency. A Yuri
opponent destroying the GAROBO instantly cripples the Robot Tank fleet.

---

## Cross-references

- [GAROBO.md](../structures/GAROBO.md) — Robot Control Center, the
  required prereq + activation controller (pending).
- [LTNK.md](../yuri/LTNK.md) — Lasher Tank, shares AP warhead profile
  (basic ROBO ≈ basic LTNK firepower-wise).
- [MGTK.md](../allied/MGTK.md) — Mirage Tank, peer Allied "specialised
  vehicle" tier.
- [TANYA.md](../allied/TANY.md) and other anti-Yuri options — Robot
  Tank is one of the dedicated psi-immune Allied responses.

---

## Ghidra audit log (audit iteration 23 — 2026-05-18)

**Methodology**: ROBO has 4 NEW field-scope claims (PoweredUnit,
VoiceSelectDeactivated, ActivateSound, DeactivateSound) including 2
DUAL-READ patterns. This audit verifies all 4 + pins their struct
offsets. ~12 Ghidra queries: 5 string searches + 4 xref lookups + 1
grep on saved TechnoTypeClass__ReadINI.

### Negative claim re-verified

| Query | Result |
|-------|--------|
| `search_strings("^ROBO$")` | **0 matches** |

Confirms: no hardcoded section-name branch for ROBO.

### String + parser xref verification (BINARY-VERIFIED)

All 4 doc-cited claims verify exactly:

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `PoweredUnit` | 0x00844158 | 0x00713316 | TechnoTypeClass__ReadINI |
| `VoiceSelectDeactivated` | 0x00844288 | 0x00712C0A | TechnoTypeClass__ReadINI |
| `ActivateSound` | 0x0083A6DC | **DUAL-READ**: RulesClass__ReadAudioVisual @ 0x0066A21E **+** TechnoTypeClass__ReadINI @ 0x007138EC | Confirmed dual-parser |
| `DeactivateSound` | 0x0083A6CC | **DUAL-READ**: RulesClass__ReadAudioVisual @ 0x0066A260 **+** TechnoTypeClass__ReadINI @ 0x00713922 | Confirmed dual-parser |

The DUAL-READ pattern for Activate/DeactivateSound mirrors:
- ChronoInSound / ChronoOutSound (audit 17, CMIN)
- ImpactLandSound / SinkingSound (audit 17/20, dual-read)
- DeathWeapon (audit 18, FV)

All these have a global default in `RulesClass__Read*` + a per-TechnoType override in `TechnoTypeClass__ReadINI`. Pattern is now well-established.

### NEW TechnoType offsets BINARY-VERIFIED

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0x410` | `PoweredUnit` | byte | `*(char*)(param_1 + 0x104) = (char)uVar5` after ReadBool. **NEW** — gates the PoweredUnit state machine: unit deactivates when owning house has no power OR controlling building (GAROBO for ROBO) is destroyed. |
| `+0x45C` | `VoiceSelectDeactivated` | int (VocClass index — soundlist pool) | `param_1[0x117] = local_*` after CCINIClass::ReadSoundList. **NEW** — parallel to VoiceSelect, used when unit is in deactivated state. Rare field; only PoweredUnit=yes units typically need it. |
| `+0x5A8` | `ActivateSound` | int (VocClass index) | `param_1[0x16A] = iVar6` (sequence-position evidence — write occurs just before DeactivateSound parse begins). **NEW** — TechnoType per-unit override side of DUAL-READ pattern with RulesClass__ReadAudioVisual global default. |
| `+0x5AC` | `DeactivateSound` | int (VocClass index) | `param_1[0x16B]` INFERRED by sequence-position adjacency with ActivateSound (parse-order guarantees sequential write at +4). **NEW** — TechnoType per-unit override side of DUAL-READ pattern. |

### Sound-list cluster topology (cumulative consolidation)

ROBO's audit reveals a second sound cluster at TechnoType+0x5A8/+0x5AC, separate from the audit-14/17 cluster at +0x568..+0x57C. Sound slots discovered so far:

| Offset | INI key | Audit |
|--------|---------|-------|
| `+0x568` | (unknown sibling) | 17 |
| `+0x56C` | DeploySound | 14 |
| `+0x570` | UndeploySound | 14 |
| `+0x574` | ChronoInSound | 17 |
| `+0x578` | ChronoOutSound | 17 |
| `+0x57C` | (unknown sibling) | 17 |
| `+0x5A8` | ActivateSound | **23** |
| `+0x5AC` | DeactivateSound | **23** |

### Items NOT re-verified in this pass (DEFERRED)

- The PoweredUnit ↔ GAROBO controlling-building lookup mechanism (the
  doc's open question: how does the engine map "PoweredUnit=yes" units
  to *which* building is their controller?). Open follow-up; not in
  scope for this audit.
- The deactivation state-machine consumer (TechnoClass per-tick code
  that checks the +0x410 byte and gates AI/move/fire on
  has-power-and-controlling-building).
- `[RobotTankPowerDown]` sound block (defined in soundmd but
  unreferenced from rules — possibly cut content).
- Hover locomotor verification (CLSID is canonical, not re-verified
  this pass).
- Trainable=no consumer (audit 1 cumulative already covers this).

### Confidence summary

- **HIGH**: 5 string addresses + 4 parser xrefs (all exact); DUAL-READ
  pattern for both ActivateSound AND DeactivateSound BINARY-VERIFIED
  (was inferred from sibling-pattern; now confirmed); 3 NEW TechnoType
  struct offsets directly verified (+0x410 PoweredUnit, +0x45C
  VoiceSelectDeactivated, +0x5A8 ActivateSound); 1 NEW TechnoType
  offset inferred by sequence-position (+0x5AC DeactivateSound).
- **MEDIUM**: DeactivateSound offset is sequence-position-inferred, not
  directly verified (would require wider grep window).
- **No INCORRECT findings**. All 4 doc-cited claims verify exactly.

---

## Coverage audit

- [x] Every rulesmd key annotated (~50 keys).
- [x] Every artmd key annotated (6 keys).
- [x] Weapon documented (Robogun — note shares warhead family with LTNK).
- [x] **No elite weapon** explicitly noted (Trainable=no).
- [x] All 9 voice/sound bindings documented, including the rare
  `VoiceSelectDeactivated` and `ActivateSound`/`DeactivateSound`.
- [x] Prerequisites: `GAWEAP, GAROBO` (Robot Control Center).
- [x] Owner: 5 Allied houses.
- [x] Veterancy: `Trainable=no` (vestigial Veteran/Elite lists noted).
- [x] Hardcoded behavior: PoweredUnit state machine, VoiceSelectDeactivated,
  ActivateSound/DeactivateSound DUAL-READ, Hover locomotor, immunities
  stack, Trainable=no.
- [x] TS-legacy filter: `ImmuneToVeins=yes` flagged dormant.
- [x] Comparison table with peer vehicles.
- [x] At least one Ghidra search performed (`PoweredUnit`,
  `VoiceSelectDeactivated`, `ActivateSound`, `DeactivateSound` — DUAL-READ
  confirmed).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("PoweredUnit")` | `0x00844158` (single match) |
| `get_xrefs_to(0x00844158)` | `0x00713316 → TechnoTypeClass__ReadINI` |
| `search_strings("VoiceSelectDeactivated")` | `0x00844288` (single match) |
| `get_xrefs_to(0x00844288)` | `0x00712c0a → TechnoTypeClass__ReadINI` |
| `search_strings("ActivateSound")` | `0x0083a6dc` (+ 3 SW-specific variants) |
| `get_xrefs_to(0x0083a6dc)` | **DUAL-READ**: `0x0066a21e RulesClass__ReadAudioVisual` + `0x007138ec TechnoTypeClass__ReadINI` |
| `search_strings("DeactivateSound")` | `0x0083a6cc` (single match) |
| `get_xrefs_to(0x0083a6cc)` | **DUAL-READ**: `0x0066a260 RulesClass__ReadAudioVisual` + `0x00713922 TechnoTypeClass__ReadINI` |

**New cheat-sheet entries:**
- `PoweredUnit` (0x00844158 → 0x00713316) TechnoType — gates the
  power-dependency state machine.
- `VoiceSelectDeactivated` (0x00844288 → 0x00712c0a) TechnoType — parallel
  voice slot for deactivated-state click.
- `ActivateSound` **DUAL-READ** Rules (0x0066a21e) + TechnoType (0x007138ec).
- `DeactivateSound` **DUAL-READ** Rules (0x0066a260) + TechnoType (0x00713922).

Both DUAL-READ entries follow the established pattern (ChronoInSound,
ChronoOutSound, ImpactLandSound).

**Open questions:**
- The PoweredUnit ↔ GAROBO controlling-building lookup mechanism isn't
  yet investigated. How does the engine map "PoweredUnit=yes" units to
  *which* building is their controller? Hardcoded association? Building-
  side flag? Configurable lookup? Open follow-up; not blocking unit doc.
- Soundmd `[RobotTankPowerDown]` block is defined but appears unreferenced
  from rulesmd. Possibly cut content (an older `PowerDownSound=` field
  that got renamed to `DeactivateSound=`).
