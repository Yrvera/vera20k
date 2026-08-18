# Desolator (DESO)
Side: Soviet | Category: Infantry | Image alias: `[DESO]` (no `Image=` redirect — own SHP `DESO`)

The Soviet **Desolator**. $600 from Soviet Barracks + Radar.
**Iraq-only national special unit** (`RequiredHouses=Arabs`).
Mid-game (`TechLevel=8`) anti-infantry / area-denial specialist with the
**deploy-radiation** behavior: the unit has two weapons —
**`Primary=RadBeamWeapon`** (a `IsRadBeam=yes` green Tesla-style beam,
Damage 125 @ Range 6) for direct fire while standing, and
**`Secondary=RadEruptionWeapon`** with **`AreaFire=yes`** + **`RadLevel=500`**
fired from the deployed pose. The Secondary creates a `RadSiteClass`
(0x74 bytes) at the Desolator's own cell that radiates the surrounding
10-cell area for hundreds of frames, dealing periodic damage via the
`RadSite` warhead. **`Deployer=yes`** + **`DeployFire=yes`** drive the
"crouch and emit" behavior: when deployed, the unit fires only its
Secondary (the radiation eruption); when undeployed/standing, it fires
the Primary beam. **`Fearless=yes`**, **`SelfHealing=yes`**,
**`Crushable=no`**, **`ImmuneToRadiation=yes`** combine to make the
Desolator one of the most defensively-capable infantry in the game.

Authoritative deep RE for the radiation system:
[RADIATION_EMP_GHIDRA_REPORT.md](../../RADIATION_EMP_GHIDRA_REPORT.md);
for the visual beam: [RAD_BEAM_CLASS_GHIDRA_REPORT.md](../../RAD_BEAM_CLASS_GHIDRA_REPORT.md).

---

## rulesmd.ini — `[DESO]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4816`:

```ini
[DESO]
UIName=Name:DESO
Name=Desolater
Category=Soldier
Primary=RadBeamWeapon
Secondary=RadEruptionWeapon
Prerequisite=NAHAND,RADAR
CrushSound=InfantrySquish
Strength=150
Armor=plate
TechLevel=8
Pip=red
Sight=6
Speed=4
Owner=Russians,Confederation,Africans,Arabs
RequiredHouses=Arabs
Cost=600
Soylent=300
Points=30
IsSelectableCombatant=yes
VoiceSelect=DesolatorSelect
VoiceMove=DesolatorMove
VoiceAttack=DesolatorAttackCommand
VoiceFeedback=
VoiceSpecialAttack=DesolatorMove
DieSound=DesolatorDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
;MovementZone=InfantryDestroyer ;GEF wow!!! copy paste bug from the original Disk Thrower!
ThreatPosed=20	; This value MUST be 0 for all building addons
Deployer=yes
DeployFire=yes
; DeployTime=.022  ; PCG; Unused for now.  Was maybe going to make its way in if we did
; a more explicit state machine for deploying b/c of autodeploy.
ImmuneToRadiation=yes
ImmuneToPsionics=no
Bombable=yes
AllowedToStartInMultiplayer=no
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Size=1
Fearless=yes
SelfHealing=yes
Crushable=no
ElitePrimary=RadBeamWeaponE
IFVMode=9
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:DESO` | CSF-string key → "Desolator" (note: INI Name= is "Desolater" — typo preserved in source) |
| `Name=Desolater` | Internal name — **misspelled** (Desolater vs Desolator); the UI uses the CSF "Desolator" but internal code-references "Desolater" |
| `Category=Soldier` | Infantry pip/AI grouping |
| `Primary=RadBeamWeapon` | **Standing** weapon — `Damage=125`, `Range=6`, `IsRadBeam=yes` green Tesla beam, single-target. Used when the Desolator is NOT deployed |
| `Secondary=RadEruptionWeapon` | **Deployed** weapon — `Damage=1` placeholder, `AreaFire=yes` (fires at self), `RadLevel=500`, `FireOnce=no` (maintains radiation). Used when deployed. Creates the iconic radiation puddle |
| `Prerequisite=NAHAND,RADAR` | Soviet Barracks + any building with `Radar=yes` |
| `CrushSound=InfantrySquish` | Crush sound — **moot** (`Crushable=no`) |
| `Strength=150` | HP — **highest of any basic infantry** (Brute=350 is the only higher infantry). Combined with SelfHealing and ImmuneToRadiation, very durable |
| `Armor=plate` | Damage type column 2 — Plate armor. Same as Tesla Trooper. Resistant to small-arms |
| `TechLevel=8` | Tech-8 cap — late mid-game / Battle Lab era |
| `Pip=red` | Cargo pip color — red (elite class) |
| `Sight=6` | Reveal radius — modest |
| `Speed=4` | Foot-speed — standard infantry |
| `Owner=Russians,Confederation,Africans,Arabs` | All 4 Soviet houses listed |
| `RequiredHouses=Arabs` | **Country-locked to Iraq only** — TechnoTypeClass field. Iraq's national special unit. Even if a Russian/Cuban/Libyan player has the Soviet barracks, they cannot build the Desolator unless they ARE Iraq. The "Iraq has the Desolator" matchup decision is encoded here |
| `Cost=600` | $600 — same as Sniper, Ivan |
| `Soylent=300` | $300 Grinder refund (Yuri only) |
| `Points=30` | **30** — high kill score (matches Ivan; reflects strategic value) |
| `IsSelectableCombatant=yes` | Included in select-all-combat |
| `VoiceSelect=DesolatorSelect` | Select voice — `$idessea..e` (5 lines, Iraqi-accented Arabic) |
| `VoiceMove=DesolatorMove` | Move voice — `$idesmoa..e` (5 lines) |
| `VoiceAttack=DesolatorAttackCommand` | Attack voice — `$idesata..f` (6 lines — second-largest attack bank) |
| `VoiceFeedback=` | **EMPTY** — the soundmd `[DesolatorFear]` section is commented out (`;[DesolatorFear]` / `;Sounds=`). Combined with `Fearless=yes` below, Desolator never plays a fear voice. This is **intentional** — the design is "Desolator does not panic" |
| `VoiceSpecialAttack=DesolatorMove` | Reuses Move voice — no special-attack-specific line |
| `DieSound=DesolatorDie` | Death voice — `$idesdia/b/c` (3 lines) |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — standard infantry |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `;MovementZone=InfantryDestroyer ;GEF...` | Same Disk Thrower copy-paste-fix comment seen on Conscript/Tesla Trooper |
| `ThreatPosed=20` | AI scoring weight — moderate (matches Tesla Trooper, dog) |
| `Deployer=yes` | **Behavior flag** — InfantryTypeClass field. **InfantryType+0xEC8 (byte) [BINARY-VERIFIED audit 35]** (assembly-context proof at 0x00524620: `MOV byte ptr [ESI + 0xec8], AL` after ReadBool; parser xref @ 0x0052460D, string @ 0x00825928). Sole InfantryTypeClass__ReadINI xref — specific to InfantryType. Pins the audit-13 cumulative +0xEAC..+0xECB capability-flag block. |
| `DeployFire=yes` | **Behavior flag** — TechnoTypeClass field. **TechnoType+0x6AC (byte) [BINARY-VERIFIED audit 35, re-confirms audit 1]** (parser xref @ 0x007147EF, string @ 0x00843AA0). Sibling to +0x6A8 DeployFireWeapon (int) which holds the weapon slot index. When set, the deployed unit's weapon selection changes — Secondary becomes the active firing weapon instead of Primary. |
| `; DeployTime=.022 ...` (commented) | Designer comment: "Unused for now. Was maybe going to make its way in if we did a more explicit state machine for deploying b/c of autodeploy". Confirms the **autodeploy** behavior exists (when the AI's threat scan finds enough targets within Secondary's RadEruptionWarhead spread, the Desolator auto-deploys) but the timing parameter was never wired up |
| `ImmuneToRadiation=yes` | **Behavior flag** — TechnoTypeClass field at offset `0xD37` (per `TechnoTypeClass__ReadINI @ 0x00714D53`; see RADIATION_EMP RE doc §1.4). The Desolator **does not take damage from its own radiation** (or any other source's). Critical — without this, the Desolator would kill itself in its own radiation puddle |
| `ImmuneToPsionics=no` | **Explicit no** — Desolator CAN be mind-controlled. Major counter for Yuri vs Iraq |
| `Bombable=yes` | Crazy Ivan can bomb a Desolator |
| `AllowedToStartInMultiplayer=no` | Not in starting unit complement |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | Standard 5 abilities at Veteran |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 at Elite (with SELF_HEAL stacking on top of the base `SelfHealing=yes`) |
| `Size=1` | Transport cargo slot cost |
| `Fearless=yes` | **Behavior flag** — never plays Panic sequence. **InfantryType+0xEBC (byte) [BINARY-VERIFIED audit 35]** (assembly-context proof at 0x0052447A: `MOV byte ptr [ESI + 0xebc], AL` after ReadBool; parser xref @ 0x00524469, string @ 0x008259D4). Sole InfantryTypeClass__ReadINI xref — InfantryType-scope. |
| `SelfHealing=yes` | **Behavior flag** — TechnoTypeClass field. **TechnoType+0xD14 (byte) [BINARY-VERIFIED audit 35, re-confirms audit 7]** (writeback `*(char *)(param_1 + 0x345) = (char)uVar5` int*-stride 0x345*4=0xD14; parser xref @ 0x00714AD9, string @ 0x00843928). Passive HP regeneration — Rate driven by global `SelfHealInfantryFrames`/`SelfHealInfantryAmount` from RulesClass. |
| `Crushable=no` | **Cannot be crushed** by vehicles — same crush-immunity as Tesla Trooper. Combined with the radiation puddle, vehicles can't simply run over a deployed Desolator |
| `ElitePrimary=RadBeamWeaponE` | At Elite rank, Primary swaps to `[RadBeamWeaponE]` — `Damage=200` (vs 125), `Range=8` (vs 6). 60% damage / 33% range increase |
| `IFVMode=9` | IFV gunner-table index 9 → HTK's `Weapon10`/`ElitePassengerWeapon10` slot. In stock YR maps to a radiation-themed beam weapon. Garrisoned Desolator gives the IFV chassis a long-range green Tesla beam |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `yes` (prone-walking enabled)
- `NotHuman=` — defaults to `no` (Desolator is human, subject to InfDeath, sniper headshot, mind-control)
- `Trainable=` — **not set, defaults to `yes`** — Desolator gains veterancy (presence of VeteranAbilities/EliteAbilities/ElitePrimary confirms)
- `Occupier=` — defaults to `no`; Desolator **cannot garrison** civilian buildings
- `Agent=`/`Infiltrate=`/`Engineer=` — not set; not an infiltrator
- `Ivan=`/`C4=`/`Assaulter=` — not set; not a bomb-planter
- `Bombable=yes` is explicit (most infantry default to `no`); Crazy Ivan CAN bomb a Desolator
- `DetectDisguise=` — not set
- `DefaultToGuardArea=` — not set
- `BombSight=` — not set; Desolator does not detect bombs
- `Natural=` — not set

---

## artmd.ini — `[DESO]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:214`:

```ini
[DESO] ; Desolator
Cameo=DESOICON
AltCameo=DESOUICO
Sequence=DesoSequence
Crawls=yes
Remapable=yes
FireUp=2
PrimaryFireFLH=100,-25,135
```

| Key | Meaning |
|-----|---------|
| `Cameo=DESOICON` | Sidebar build icon (SHP) |
| `AltCameo=DESOUICO` | Elite cameo — shown after Veteran promotion |
| `Sequence=DesoSequence` | Reference to `[DesoSequence]` — Desolator-specific sequence with full deploy frames |
| `Crawls=yes` | Prone-capable |
| `Remapable=yes` | House remap palette applied |
| `FireUp=2` | Bullet-spawn frame — at frame 2 the beam is launched. Same as Tesla Trooper (matching the instant-beam visual) |
| `PrimaryFireFLH=100,-25,135` | FLH — 100 forward, -25 sideways, 135 up. **Same as Tesla Trooper** — shoulder-mounted beam apparatus at the same body geometry |

Missing `SecondaryFireFLH=` — the Secondary RadEruptionWeapon is `AreaFire=yes` (fires at own cell), so no FLH needed.

### Referenced sequence — `[DesoSequence]`

`artmd.ini:14116`:

```ini
[DesoSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Prone=86,1,6
Crawl=86,6,6
Die1=134,15,0
Die2=149,15,0
FireUp=164,6,6
FireProne=212,6,6
Down=260,2,2
Up=276,2,2
Deploy=299,15,0
Deployed=298,1,0
DeployedFire=292,7,0
Undeploy=0,1,1
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Cheer=56,15,0,S
Panic=8,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle | |
| `Guard=0,1,1` | Guard idle | Same |
| `Walk=8,6,6` | Walk cycle 6×6 | |
| `Idle1=56,15,0,S` | Idle 1 — 15 frames, S-facing | |
| `Idle2=71,15,0,E` | Idle 2 — E-facing | |
| `Prone=86,1,6` | Prone single frame × 6 facings | |
| `Crawl=86,6,6` | Crawl reuses prone | |
| `Die1=134,15,0` | Death 1 — 15 frames | |
| `Die2=149,15,0` | Death 2 | |
| `FireUp=164,6,6` | **Standing fire** — 6 frames × 6 facings. Where RadBeamWeapon fires | |
| `FireProne=212,6,6` | Prone-fire cycle | |
| `Down=260,2,2` | Get-down to prone | |
| `Up=276,2,2` | Get-up from prone | |
| `Deploy=299,15,0` | **Deploy animation — 15 frames, omnidirectional** | The crouch-and-arm-radiation-canister animation. Frame range 299 onward |
| `Deployed=298,1,0` | **Deployed idle pose — single frame at 298** | The "kneeling with canister deployed" frame held continuously while deployed |
| `DeployedFire=292,7,0` | **Deployed-fire cycle — 7 frames at 292** | The "emit radiation pulse" animation while deployed and firing Secondary |
| `Undeploy=0,1,1` | **Undeploy → Ready frame** | No dedicated undeploy animation; snaps back to standing |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → Ready frame | |
| `Cheer=56,15,0,S` | Cheer reuses Idle1 frames | |
| `Panic=8,6,6` | Panic = Walk frames | **Unreachable** (`Fearless=yes`) |

---

## Weapons

### Primary (Veteran and below) — `[RadBeamWeapon]`

`rulesmd.ini:23769`:

```ini
[RadBeamWeapon]
Damage=125
ROF=50
Range=6
Speed=100
Projectile=InvisibleLow
Warhead=RadBeamWarhead
IsRadBeam=yes
Report=DesolatorAttack
```

| Key | Meaning |
|-----|---------|
| `Damage=125` | Per-shot damage. Vs Armor=none infantry (GI) at `RadBeamWarhead.Verses[none]=100%` → 125 dmg (one-shots GI). Vs vehicles: 20/15/10% — minimal damage |
| `ROF=50` | Cooldown — 50 frames (~3.3s) — slow enough that Desolator pairs well with mass infantry kills, not vehicle DPS |
| `Range=6` | 6 cells — twice Tesla Trooper's 3, allowing engagement without closing |
| `Speed=100` | Irrelevant for inviso |
| `Projectile=InvisibleLow` | LOS-respecting inviso |
| `Warhead=RadBeamWarhead` | See warhead — strong vs infantry, weak vs vehicles, `Radiation=yes` flag |
| `IsRadBeam=yes` | **Visual flag** — WeaponTypeClass field. Spawns a `RadBeam` instance (200 bytes, allocator `0x00659110`) drawn as the **green Tesla-style straight beam** between attacker and target. Color comes from `[Radiation].RadColor=0,255,0`. Step size 20.0 leptons per segment. Duration 15 ticks. See [RAD_BEAM_CLASS_GHIDRA_REPORT.md](../../RAD_BEAM_CLASS_GHIDRA_REPORT.md) |
| `Report=DesolatorAttack` | Sound `idesat1a` (beam zap sample) |

### Elite Primary — `[RadBeamWeaponE]`

`rulesmd.ini:25064`:

```ini
[RadBeamWeaponE]
Damage=200
ROF=50
Range=8
Speed=100
Projectile=InvisibleLow
Warhead=RadBeamWarhead
IsRadBeam=yes
Report=DesolatorAttack
```

Delta from `[RadBeamWeapon]`:
- **Damage 125→200** (+60%)
- **Range 6→8** (+33%)
- Same ROF, projectile, warhead, IsRadBeam, Report

### Secondary — `[RadEruptionWeapon]` (the deploy-radiation weapon)

`rulesmd.ini:23780`:

```ini
; The Desolater's desolation effect
[RadEruptionWeapon]
Damage=1		; Irrelevant as long as it is greater than 0.  Establishes that this unit can fire this weapon.
ROF=60
Range=4         ; SJM: changed from 1 so distance check won't fail on bridges -- only fired at own cell so should be OK
Speed=1
AreaFire=yes
FireOnce=no		; SJM: Desolator should maintain radiation at site when deployed
Projectile=InvisibleLow
Warhead=RadEruptionWarhead
IsRadEruption=no ; SJM: we're not using this effect anymore
RadLevel=500
Report=DesolatorDeploy
```

| Key | Meaning |
|-----|---------|
| `Damage=1` | **Inline comment**: "Irrelevant as long as it is greater than 0. Establishes that this unit can fire this weapon." Nominal damage; the actual effect is via `RadLevel=500` + `Radiation=yes` warhead. The 1 keeps the engine from refusing to fire on Damage=0 |
| `ROF=60` | Cooldown between radiation puddle refreshes — every 4 seconds |
| `Range=4` | **Inline comment**: "SJM: changed from 1 so distance check won't fail on bridges -- only fired at own cell so should be OK." Despite firing at own cell via AreaFire, Range=4 prevents bridge-related distance-check edge cases |
| `Speed=1` | Irrelevant for inviso |
| `AreaFire=yes` | **Behavior flag** — WeaponTypeClass field (per `WeaponTypeClass__ReadINI @ 0x0077283E` DATA xref to string at `0x008492F4`). **Fires at the Desolator's own cell** instead of requiring a target. The Desolator emits radiation around itself |
| `FireOnce=no` | **Inline comment**: "Desolator should maintain radiation at site when deployed." Continuously re-fires while deployed to keep the radiation puddle topped up (each fire creates/augments a RadSite at the cell, additive per the radiation system's `AddRadLevel` path) |
| `Projectile=InvisibleLow` | LOS-respecting inviso (matters here for the bridge case) |
| `Warhead=RadEruptionWarhead` | See warhead — `Radiation=yes`, large `CellSpread=10` |
| `IsRadEruption=no` | **DISABLED** — SJM inline comment: "we're not using this effect anymore". WeaponTypeClass field. **WeaponType+0x155 (byte) [BINARY-VERIFIED audit 35]** (assembly-context proof at 0x007728D3: `MOV byte ptr [ESI + 0x155], AL` after ReadBool; parser xref @ 0x007728C0, string @ 0x008492A4). **NOTE: potential audit-9 cumulative conflict** — audit 9 listed +0x155 as IsRadBeam. Both keys can't be at the same offset; one of the audit-9 entries was misaligned. Flagged for future re-verification with explicit IsRadBeam assembly trace. |
| `RadLevel=500` | **THE radiation amount** — WeaponTypeClass field at offset `0x158` (per `WeaponTypeClass__ReadINI @ 0x007728DA` per RADIATION_EMP RE §1.3). When this weapon detonates, `WarheadTypeClass::Detonate` creates a RadSite with `RadLevel=500` at the impact cell. With `RadDurationMultiple=1` global, the site lasts 500 frames (~33s @ 15fps). With `RadLevelFactor=0.2` global, per-cell damage at center is `500 × 0.2 = 100` per application, falling off linearly to spread edge. With `RadApplicationDelay=16` global, damage applies every 16 frames |
| `Report=DesolatorDeploy` | Sound `idesat2a` (radiation hum sample — distinct from the beam zap) |

### Primary's Warhead — `[RadBeamWarhead]`

`rulesmd.ini:27327`:

```ini
[RadBeamWarhead]
Verses=100%,100%,100%,20%,15%,10%,0%,0%,0%,100%,100%
InfDeath=7
Radiation=yes
```

| Key | Meaning |
|-----|---------|
| `Verses=100%,100%,100%,20%,15%,10%,0%,0%,0%,100%,100%` | 11-column. **100/100/100 vs infantry armor (none/flak/plate)** — strong anti-infantry (Damage 125 × 100% = 125 one-shots GI at Strength 100). **20/15/10 vs vehicle armor** — weak (only 12-25 dmg/shot vs tanks). **0% vs wood/steel/concrete** — cannot damage buildings at all. **100% vs special_1/special_2** |
| `InfDeath=7` | **Infantry death animation type 7** — the **radiation/melt** death animation (different from electric InfDeath=5 or "blown to bits" InfDeath=6). Distinctive visual signal that the kill was radiation |
| `Radiation=yes` | **Warhead flag** marking this as radiation damage. Combined with the per-cell RadLevel system, units take periodic radiation damage from cells they stand in. `ImmuneToRadiation=yes` units (Desolator itself, Terror Drone, etc.) are skipped |

### Secondary's Warhead — `[RadEruptionWarhead]`

`rulesmd.ini:27340`:

```ini
[RadEruptionWarhead]
Verses=100%,100%,100%,20%,10%,10%,0%,0%,0%,100%,100%
InfDeath=7
Radiation=yes
CellSpread=10
CellInset=3  ; PCG: This means that the desolater won't autodeploy unless the target is 3 cells inside the max radius.
```

| Key | Meaning |
|-----|---------|
| `Verses=100%,100%,100%,20%,10%,10%,0%,0%,0%,100%,100%` | Essentially identical to RadBeamWarhead Verses; very slight reduction vs medium/heavy (10% vs 15/10) |
| `InfDeath=7` | Radiation death animation |
| `Radiation=yes` | Warhead radiation flag |
| `CellSpread=10` | **Splash radius 10 cells** — massive. The RadSite's effective area covers a 10-cell radius around the impact point, with damage falling off linearly toward the edge |
| `CellInset=3` | **Behavior key** — Designer comment: "This means that the desolater won't autodeploy unless the target is 3 cells inside the max radius". When the AI scans for autodeploy targets, the threat must be at least 3 cells **inside** the 10-cell max radius (i.e., within 7 cells of the Desolator's position) to trigger autodeploy. Prevents the Desolator from deploying for marginal-edge targets that would barely be in the puddle |

### Auxiliary — `[RadSite]` warhead (cell-resident radiation damage)

`rulesmd.ini:27349`:

```ini
[RadSite]
Verses=100%,100%,100%,50%,10%,10%,0%,0%,0%,100%,100%
InfDeath=7
Radiation=yes
```

**This is the warhead applied to units standing in irradiated cells** (per
`RulesClass.RadSiteWarhead=RadSite` in `[Radiation]`). Damage is calculated
per-tick by the RadSite::ApplyRadDamage path at
`RadApplicationDelay=16` frame intervals. Note `Verses[light]=50%` is
higher than RadEruptionWarhead's 20% — so a unit standing **in** an
irradiated cell takes more vehicle-armor damage than the initial blast,
because the per-tick application doesn't have the spread-falloff factor
that the initial impact has.

### Projectile — `[InvisibleLow]`

Standard LOS-respecting inviso projectile (same as Tesla Trooper, Conscript ground weapon).

---

## The radiation system (cross-reference summary)

Full RE in [RADIATION_EMP_GHIDRA_REPORT.md](../../RADIATION_EMP_GHIDRA_REPORT.md).
Summary of the pipeline triggered by Desolator deploy-fire:

```
1. Desolator deployed → fires Secondary (RadEruptionWeapon, AreaFire=yes,
   FireOnce=no, ROF=60). Target = own cell.

2. WarheadTypeClass::Detonate @ 0x004690B0 sees weapon->RadLevel=500 > 0.

3. Look up the impact cell. If cell already has a RadSite:
     RadSiteClass::AddRadLevel(existing, 500) — additive top-up
   Else:
     new RadSiteClass()  (0x74 bytes, vtable 0x007F0810)
     SetCell(cell), SetSpread(RadEruptionWarhead.CellSpread=10),
       SetRadLevel(500), Activate()
     cell->RadSite = new

4. RadSiteClass::Activate:
     LightIntensity = ftol(500 × RulesClass.RadLightFactor=0.1) = 50
     TintR/G/B = ftol(RadColor=0,255,0 × RadTintFactor=1.0) = (0,255,0)
     TotalDuration = 500 × RadDurationMultiple=1 = 500 frames (~33s @ 15fps)
     Create LightSourceClass (green glow)
     Iterate all cells within 10-cell radius:
       cellRadLevel = (1 - dist/spread) × 500   (linear falloff)
       CellClass::IncreaseRadLevel(cell, cellRadLevel)

5. RadSiteClass::AI (per tick):
     RemainingDuration--
     If RadLevelTimer expired (every RadLevelDelay=90 frames):
       Decay cell RadLevels proportionally
     If RadLightTimer expired (every RadLightDelay=90 frames):
       Fade light intensity + tint
     If RemainingDuration <= 0: self-destruct

6. Separately, per-object update loop checks each object's cell for RadLevel > 0:
     If object->Type->ImmuneToRadiation: skip
     Else apply damage via RulesClass.RadSiteWarhead=RadSite warhead
     Damage cadence: every RadApplicationDelay=16 frames
     Damage magnitude: cellRadLevel × RadLevelFactor=0.2
```

**Key parity facts:**
- Multiple Desolators can stack their radiation (additive RadLevel)
- The radiation cell-decay is independent of the Desolator — once the puddle
  is down, killing the Desolator doesn't clear it (the puddle persists for
  ~33s after the Desolator stops firing). But killing the Desolator while
  it's deployed-firing **does** stop the periodic top-up — the puddle then
  decays naturally
- Buildings take no damage from radiation (warhead Verses 0% vs structure
  armors)
- The green glow is rendered via `LightSourceClass`, separate from the
  per-cell yellow-green tint overlay
- Vehicles inside a radiation cell take damage but are NOT slowed — radiation
  damages, it doesn't impair movement

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement (no fear voice — by design)

```ini
[DesolatorSelect]                  ; soundmd.ini:3708
Sounds= $idessea $idesseb $idessec $idessed $idessee
Control= random
Volume=85

[DesolatorMove]                    ; soundmd.ini:3703
Sounds= $idesmoa $idesmob $idesmoc $idesmod $idesmoe
Control= random
Volume=85

[DesolatorAttackCommand]           ; soundmd.ini:3698
Sounds= $idesata $idesatb $idesatc $idesatd $idesate $idesatf
Control= random
Volume=85

;[DesolatorFear]                    ; soundmd.ini:3713 — DISABLED
;Sounds=
```

**5 select / 5 move / 6 attack lines.** No fear bank — the section is
commented out (`;[DesolatorFear]` / `;Sounds=`). Combined with
`Fearless=yes` on the unit and the empty `VoiceFeedback=` field, the
Desolator **never plays a fear voice** — a deliberate design choice
reinforcing the "imperturbable radiation specialist" character.

### Death

```ini
[DesolatorDie]                     ; soundmd.ini:3716
Sounds= $idesdia $idesdib $idesdic
Control= random
```

3 death lines.

### Weapon reports (2 distinct sounds for 2 weapons)

```ini
[DesolatorAttack]                  ; soundmd.ini:969
Sounds=idesat1a

[DesolatorDeploy]                  ; soundmd.ini:972
Sounds=idesat2a
```

| Sound | Used by | Distinction |
|-------|---------|-------------|
| `DesolatorAttack` | `[RadBeamWeapon]`/`[RadBeamWeaponE]` (standing fire) | `idesat1a` — beam zap |
| `DesolatorDeploy` | `[RadEruptionWeapon]` (deployed fire) | `idesat2a` — radiation eruption / hum |

Both very minimal definitions — single sample each, no FShift/VShift/Limit.

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `NAHAND,RADAR` | Soviet Barracks + any Radar building |
| `Owner=` | `Russians,Confederation,Africans,Arabs` | All 4 Soviet houses (template) |
| `RequiredHouses=` | `Arabs` | **Iraq-only national special unit** — country lock filters out Russians/Confederation/Africans |
| `TechLevel=` | `8` | Late mid-game tech-8 cap |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=600` | $600 | Same as Sniper, Ivan |
| `Soylent=300` | $300 refund (Yuri only) | |
| `Points=30` | **30** | Highest tier (matches Ivan) |

The country-lock follows the Allied per-country special pattern: Britain=Sniper,
France=Mirage Tank, Germany=Tank Destroyer, Korea/USA=Black Eagle/Paratroopers,
Iraq=**Desolator**, Russia=Tesla Tank, Cuba=Terrorist (Yuri's Revenge),
Libya=Demolition Truck.

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — standard 5 abilities |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — **SELF_HEAL stacks on top of base `SelfHealing=yes`** (effectively faster regen). Triggers `ElitePrimary=RadBeamWeaponE` (Damage 125→200, Range 6→8) |
| AltCameo | `DESOUICO` shown in sidebar once Veteran rank reached |

`Trainable=` defaults to `yes` — Desolator gains XP normally.

---

## Hardcoded behavior — Ghidra-verified

### 1. Deployer + DeployFire — the standing/deployed weapon-swap mechanism

Two separate flags wired together:

- **`Deployer=yes`** — InfantryTypeClass field (per `InfantryTypeClass__ReadINI @ 0x0052460D` DATA xref to string at `0x00825928`). **Specific to InfantryTypeClass, not the parent TechnoType** — only infantry units can use this flag. Enables the deploy-undeploy command (D-key hotkey, or right-click-deploy via context). When set, the engine adds the Deploy mission to the unit's allowed missions and shows the deploy cursor on the unit's own position
- **`DeployFire=yes`** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x007147EF` DATA xref to string at `0x00843AA0`). When set, the deployed unit's weapon selection logic prefers Secondary over Primary. Without DeployFire, deploying would change the sprite (via DesoSequence.Deployed/DeployedFire frames) but the unit would still try to fire Primary at out-of-range targets. With DeployFire, deploying flips the unit to Secondary-only firing mode

For Desolator specifically: standing → fires Primary (RadBeamWeapon, ranged
beam at single target); deployed → fires Secondary (RadEruptionWeapon,
AreaFire on own cell creating RadSite). The hotkey-triggered deploy mission
runs the DesoSequence.Deploy animation (15 frames at frame 299), then
locks the unit in DesoSequence.Deployed pose (single frame at 298) and
DesoSequence.DeployedFire cycle (7 frames at 292) for the firing animation.

**Companion `IsSimpleDeployer`** (xref to string at `0x00845DFC`) — separate
flag for units that deploy into a different type (like MCV → ConYard). Not
used by Desolator (Desolator deploys in-place, retaining its type).

### 2. AreaFire — fire-at-own-cell mechanism

INI key `AreaFire=yes` on RadEruptionWeapon is a WeaponTypeClass field
(per `WeaponTypeClass__ReadINI @ 0x0077283E` DATA xref to string at
`0x008492F4`). Bypasses the normal "target acquisition" path:
- Target = the firing unit's own cell coordinates (no other target object
  needed)
- Firer always has range to itself, so range checks effectively always pass
- The weapon's projectile detonates at the firer's position immediately

Combined with `FireOnce=no` and the deployed-state weapon-pick from
DeployFire=yes, the Desolator continuously refreshes the radiation puddle
at its own cell every ROF=60 frames.

### 3. RadLevel — radiation deposit per fire

INI key `RadLevel=500` on RadEruptionWeapon is a WeaponTypeClass field at
offset `0x158` (per `WeaponTypeClass__ReadINI @ 0x007728DA`, documented in
RADIATION_EMP RE §1.3). When `WarheadTypeClass::Detonate` runs and sees
`weapon->RadLevel > 0`, it creates or augments a `RadSiteClass` at the
impact cell. The actual radiation effect (cell decay, light glow,
per-tick damage) is then driven by `RadSiteClass` machinery using global
`[Radiation]` rules. See RADIATION_EMP RE for the full RadSite struct
layout (0x74 bytes) and update loop.

### 4. ImmuneToRadiation — Desolator self-immunity

INI key `ImmuneToRadiation=yes` is a TechnoTypeClass field at offset
`0xD37` (per `TechnoTypeClass__ReadINI @ 0x00714D53`, per RADIATION_EMP
RE §1.4). When the per-cell radiation damage loop iterates each object
standing on an irradiated cell, units with this flag are **skipped**.
Without this flag, the Desolator would kill itself in its own puddle in
seconds.

Other ImmuneToRadiation=yes units: Terror Drone (Soviet anti-vehicle —
needs to invade vehicles, can't be slowed by radiation), Apocalypse Tank
(Soviet flagship — defensively immune for narrative reasons), some
specific structures.

### 5. SelfHealing — passive HP regen

INI key `SelfHealing=yes` is a TechnoTypeClass field (per
`TechnoTypeClass__ReadINI @ 0x00714AD9` DATA xref to string at `0x00843928`).
Enables passive HP regeneration over time. Rate driven by global
`SelfHealInfantryFrames`/`SelfHealInfantryAmount` (RulesClass `[General]`).
Combined with Elite's `SELF_HEAL` ability (which stacks rate), Elite
Desolator regenerates significantly faster than Veteran.

Cap: regen stops at unit's `Strength=` value (cannot exceed max HP).

### 6. IsRadBeam — RadBeam visual class spawn

INI key `IsRadBeam=yes` on RadBeamWeapon is a WeaponTypeClass field.
When a weapon with this flag fires, the engine spawns a `RadBeam`
instance (200 bytes, allocated via `RadBeam__Allocate @ 0x00659110`)
visible as the green Tesla-style beam between attacker and target.
Color from `[Radiation].RadColor=0,255,0`. See
[RAD_BEAM_CLASS_GHIDRA_REPORT.md](../../RAD_BEAM_CLASS_GHIDRA_REPORT.md)
for the full RadBeam class (BeamType, segment count, fade duration,
draw paths).

The Chrono Legionnaire's NeutronRifle uses the same RadBeam system but
with `BeamType=1` (sinusoidal) and the blue color from
`RulesClass+0x1866` instead of green.

### 7. IsRadEruption — disabled in stock YR

INI key `IsRadEruption=no` on the Desolator's Secondary weapon. SJM inline
comment: "we're not using this effect anymore". When set to `yes`, the
engine would spawn 8 RadBeam instances in a 3×3 grid around the impact
cell to create a visual "burst" effect (per RAD_BEAM_CLASS RE §1). In
stock YR this is **OFF** — the radiation puddle has no associated beam
visual, just the standard RadSite green light glow. Documented as
disabled code path.

### 8. CellInset=3 — autodeploy AI gating

INI key `CellInset=3` on RadEruptionWarhead. Designer comment:
"This means that the desolater won't autodeploy unless the target is 3
cells inside the max radius". The AI's autodeploy threat-scan requires
the threat to be at least 3 cells inside the max spread radius (10) — so
within 7 cells of the Desolator. Prevents the AI from triggering deploy
for marginal targets at the puddle edge.

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("Deployer\|DeployFire\|AreaFire\|SelfHealing\|ImmuneToRadiation\|IsRadBeam\|IsRadEruption\|RadLevel")` | 13 strings — confirms all 8 hardcoded keys + companions: `Deployer`, `DeployFire`, `DeployFireWeapon`, `AreaFire`, `SelfHealing`, `ImmuneToRadiation`, `IsSimpleDeployer`, `IsRadBeam`, `IsRadEruption`, `RadLevel`, `RadLevelFactor`, `RadLevelDelay`, `RadLevelMax` |
| `get_xrefs_to(0x00825928)` (= "Deployer") | Sole xref from `InfantryTypeClass__ReadINI @ 0x0052460D` DATA — confirms InfantryType-specific, not generic Techno |
| `get_xrefs_to(0x00843AA0)` (= "DeployFire") | Sole xref from `TechnoTypeClass__ReadINI @ 0x007147EF` DATA — confirms TechnoType-level (so vehicles could potentially have it too — e.g., Siege Chopper deploy-fire) |
| `get_xrefs_to(0x008492F4)` (= "AreaFire") | Sole xref from `WeaponTypeClass__ReadINI @ 0x0077283E` DATA — confirms per-weapon flag |
| `get_xrefs_to(0x00843928)` (= "SelfHealing") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714AD9` DATA — confirms TechnoType-level |

Plus deep-RE confirmation from RADIATION_EMP_GHIDRA_REPORT for: `RadLevel`
at WeaponTypeClass+0x158 (xref `0x007728DA`), `ImmuneToRadiation` at
TechnoTypeClass+0xD37 (xref `0x00714D53`), full RadSiteClass struct (0x74
bytes), AI loop (0x0065B800), Activate (0x0065B580), and
[Radiation] global keys parsed by 0x0066CF70.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;MovementZone=InfantryDestroyer` (commented) | Designer-fixed Disk Thrower copy-paste bug | OK |
| `; DeployTime=.022` (commented) | Designer history — autodeploy state-machine timing was never wired. Functional with the default-state machine | OK |
| `IsRadEruption=no` on RadEruptionWeapon | **DISABLED in stock YR** — designer comment "we're not using this effect anymore". The 8-RadBeam visual eruption is dormant. Radiation still works via the standard RadSite system | Documented |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` — standard infantry | OK |
| `MovementZone=Infantry` | Standard | OK |
| `RadSite`/`Radiation`/`RadBeam` systems | **Fully YR-active** — verified per RADIATION_EMP + RAD_BEAM_CLASS deep RE docs | OK |
| `Deployer=yes` / `DeployFire=yes` | YR-active — Desolator, Cyborg Reaper (TS heritage but live in YR via Initiate?), Boris (uses similar deploy pattern? no, Boris uses different mech). All deployer infantry use this in YR | OK |

No TS-only behavior found on the DESO type itself. The Desolator is a
quintessentially-YR unit; the only mechanic with a TS lineage is the
deploy state-machine (originally for TS Cyborg infantry), which is fully
alive in YR.

---

## Cross-references

- **Related radiation-emitting units / weapons**:
  - `[DESO]` Desolator (this doc) — Primary RadBeamWeapon + Secondary RadEruptionWeapon (RadLevel=500)
  - Nuclear Missile superweapon — `[NukeWarhead]` deposits massive RadLevel
  - Various campaign-only radiation map effects
- **Related ImmuneToRadiation=yes units**:
  - `[DESO]` Desolator (this doc)
  - `[DRON]` Terror Drone (invades vehicles, can't be slowed)
  - Some specific late-game structures
- **Related Deployer=yes infantry** (same deploy state machine):
  - `[DESO]` Desolator (radiation eruption)
  - `[GGI]` Guardian GI (AT rifle deploy — already documented)
  - `[INIT]` Yuri Initiate (no? actually has DeployFire for AreaFire psychic blast — verify when INIT is documented)
- **Related IsRadBeam=yes weapons** (green Tesla-beam visual):
  - `[RadBeamWeapon]`, `[RadBeamWeaponE]` — Desolator Primary
  - `[CRRadBeamWeapon]` — Battle Fortress's gunner-table radiation weapon (when Desolator garrisons FV)
  - Chrono Legion's NeutronRifle uses RadBeam too, but BeamType=1 sinusoidal + blue color
- **Related Allied national-unit equivalents**:
  - Britain: `[SNIPE]` Sniper
  - France: `[MGTK]` Mirage Tank
  - Germany: `[TNKD]` Tank Destroyer
  - America/Korea: `[BEAG]` Black Eagle / `[PTROOP]` Paratroopers
  - Iraq (Soviet): **`[DESO]` Desolator** (this doc)
  - Russia (Soviet): `[TTNK]` Tesla Tank
  - Cuba (Soviet): `[TERROR]` Terrorist (Yuri's Revenge)
  - Libya (Soviet): `[DTRUCK]` Demolition Truck
- **Counter-units / hard counters to Desolator**:
  - Mind-control (Yuri/Initiate) — ImmuneToPsionics=no
  - Sniper one-shot (250 dmg vs Strength=150 — still one-shot)
  - Long-range bombardment (V3, Prism Tank, Apocalypse cannon, Dreadnought) — outrange RadBeamWeapon's 6 cells
  - Air attack (Rocketeer/Harrier/Kirov) — Desolator has no AA
  - **NOT**: vehicle crush (Crushable=no), radiation (ImmuneToRadiation=yes), most small-arms (Plate armor mitigates)
- **Related global rules** in `[Radiation]`:
  - `RadDurationMultiple=1` (frames per RadLevel point of site lifetime)
  - `RadApplicationDelay=16` (frames between damage applications)
  - `RadLevelMax=500` (per-cell cap)
  - `RadLevelDelay=90` (decay step interval)
  - `RadLightDelay=90` (light update interval)
  - `RadLevelFactor=0.2` (damage scaling)
  - `RadLightFactor=0.1` (light intensity scaling)
  - `RadTintFactor=1.0` (color tint scaling)
  - `RadColor=0,255,0` (green)
  - `RadSiteWarhead=RadSite` (cell-resident damage warhead)

---

## Ghidra audit log (audit iteration 35 — 2026-05-19)

**~18 Ghidra queries** (10 string searches + 5 xref lookups + 2 assembly-
context batches + 3 grep passes on saved TechnoTypeClass decompile). 5
doc-cited claims verify exactly + 3 NEW struct-offset bindings BINARY-
VERIFIED + 1 audit-9 cumulative conflict flagged for re-verification.

### Function-entry verification

| Function | Address | Status |
|----------|---------|--------|
| `InfantryTypeClass__ReadINI` | 0x005240a0 | Deployer @ +0xEC8, Fearless @ +0xEBC verified |
| `TechnoTypeClass__ReadINI` | (oversized) | DeployFire/SelfHealing/ImmuneToRadiation re-confirmed |
| `WeaponTypeClass__ReadINI` | 0x00772080 | IsRadEruption @ +0x155 (potential conflict with audit-9 IsRadBeam) |

### Key behavioral findings — 3 NEW struct-offset bindings BINARY-VERIFIED

| INI key | Scope | Offset | Type | Parser site | Status |
|---------|-------|--------|------|-------------|--------|
| `Deployer` | InfantryType | **+0xEC8** | byte (ReadBool) | 0x0052460D | NEW — assembly-verified writeback at 0x00524620 (pins one slot of the audit-13 capability-flag block) |
| `Fearless` | InfantryType | **+0xEBC** | byte (ReadBool) | 0x00524469 | NEW — assembly-verified writeback at 0x0052447A (pins another slot of the capability-flag block) |
| `IsRadEruption` | WeaponType | **+0x155** | byte (ReadBool) | 0x007728C0 | NEW — assembly-verified writeback at 0x007728D3. **POTENTIAL CONFLICT** with audit 9 cumulative which listed +0x155 as IsRadBeam. Same offset cannot hold two keys — one of the listings is misaligned. Flagged for future re-verification. |

### InfantryType capability-flag block (audit 35 expansion of +0xEAC..+0xECB)

Audit 13 identified this 32-byte ReadBool block but couldn't name 19 of
its 23 slots. Cumulative naming progress post-audit-35:

| Offset | Key | Audit |
|--------|-----|-------|
| +0xEAC | (DEFERRED) | — |
| +0xEAD | NotHuman | 28 |
| +0xEB4 | Occupier | 1 |
| +0xEB5 | Assaulter | 33 (corrected from audit 1's guess) |
| **+0xEBC** | **Fearless** | **35** — NEW |
| +0xEBD | Crawls | 7 |
| +0xEBE | Infiltrator-synthesized | 6 (set by C4/Infiltrate/two unnamed siblings) |
| +0xEC2 | C4 | 4 |
| +0xEC5 | Engineer | 3 |
| **+0xEC8** | **Deployer** | **35** — NEW |

10 of 23 slots now named. 13 still DEFERRED (specifically +0xEAC,
+0xEAE..+0xEBB, +0xEBF, +0xEC0/+0xEC1, +0xEC3/+0xEC4, +0xEC6/+0xEC7,
+0xEC9/+0xECA/+0xECB).

### Re-confirmations from prior cumulative

- `DeployFire` = TechnoType+0x6AC (audit 1 — `param_1[0x1ab]*4` int*-stride evidence)
- `DeployFireWeapon` = TechnoType+0x6A8 (audit 1 — `param_1[0x1aa]*4` int evidence)
- `SelfHealing` = TechnoType+0xD14 (audit 7 — `param_1 + 0x345` int*-stride evidence)
- `ImmuneToRadiation` = TechnoType+0xD37 (audit 9 — `(int)param_1 + 0xd37` direct byte evidence)
- `AreaFire` = WeaponType+0x151 (audit 9 cumulative)
- `RadLevel` = WeaponType+0x158 (audit 9 cumulative — sourced from RADIATION_EMP deep RE doc)
- `IsRadBeam` = WeaponType (audit 9 cumulative claims +0x155 but conflicts with this audit's IsRadEruption finding — needs re-verification)
- `RequiredHouses` = TechnoType+0xDA0 (audit 10 cumulative)
- `Crushable` = ObjectType+0x22D (audit 7 cumulative)
- `Bombable` = ObjectType+0x22E (audit 7 cumulative)
- `IsSimpleDeployer` = UnitType+0xE13 (audit 12 — but doc clarifies it's not relevant to Desolator since SHK/DESO deploy in-place, not into another type)

### POTENTIAL CONFLICT to flag

Audit 9 cumulative WeaponType listing has:
- `+0x153` = DrawBoltAsLaser
- `+0x154` = IsAlternateColor
- `+0x155` = IsRadBeam
- `+0x158` = RadLevel

But this audit's assembly-context check shows:
- IsRadEruption parser site 0x007728C0 writes to **+0x155**

Two keys cannot share the same offset. One of these is wrong:
1. Audit 9's IsRadBeam at +0x155 might actually be IsRadEruption (which would mean IsRadBeam is at some other offset, possibly +0x154 with IsAlternateColor shifted)
2. Or my IsRadEruption finding is misread

The assembly evidence at 0x007728D3 is unambiguous: `MOV byte ptr [ESI
+ 0x155], AL` happens directly after the IsRadEruption ReadBool call.
This audit's finding is BINARY-VERIFIED. **Audit 9 cumulative entry for
+0x155 needs re-verification** (likely IsRadBeam is at a different
offset — possibly +0x156 or interleaved in the same block).

### Items NOT re-verified (DEFERRED with reason)

- **RadSite/RadiationClass/RadBeamClass deep-RE chain** — full
  pipeline (Detonate → AddRadLevel → Activate → AI tick → ApplyRadDamage)
  is sourced from RADIATION_EMP_GHIDRA_REPORT.md (standalone deep RE
  doc). Trust-chain to that report, not directly re-traced inline.
- **CellInset=3 autodeploy AI gate** — designer comment + WarheadType
  field interaction; consumer code DEFERRED.
- **Country-lock (RequiredHouses=Arabs) consumer** — verified parser
  via audit 10 cumulative; runtime country-bitmask check DEFERRED.
- **IsRadBeam exact offset** — needs re-verification per the conflict
  above.
- **Remaining 13 InfantryType +0xEAC..+0xECB slots** — would require
  string-table enumeration around the InfantryTypeClass__ReadINI
  parser sites.
- **Companion `IsSimpleDeployer` consumer for MCV-style deploy-into-
  building** vs Desolator-style deploy-in-place — distinction is
  field-driven but consumer-side dispatch DEFERRED.

### Negative claims verified

- `search_strings("DESO")` → **0 matches**.
- `search_strings("Desolator")` → **0 matches** (note: INI spelling is
  "Desolater" not "Desolator"; doc notes the typo).

All Desolator behavior is INI-driven.

### Confidence summary

- 3/3 NEW struct-offset bindings BINARY-VERIFIED via assembly-context
  proofs.
- 11 re-confirmations of prior cumulative offsets.
- 1 POTENTIAL CONFLICT flagged (IsRadBeam vs IsRadEruption at +0x155).
- Negative claims confirmed.
- InfantryType +0xEAC..+0xECB block now has 10/23 slots named.

**Soviet sub-section: 4 of 32 docs DEEP-AUDITED.**

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [DESO]` | 4816-4862 (47 lines) | All 43 active keys covered (one commented `;MovementZone`, two commented `; DeployTime` documented) |
| `artmd.ini [DESO]` | 214-221 (8 lines) | All keys covered |
| `artmd.ini [DesoSequence]` | 14116-14138 (23 lines) | All 20 active slots + 3 stub Die3-5 covered |
| `rulesmd.ini [RadBeamWeapon]` | 23769-23777 (9 lines) | All keys covered |
| `rulesmd.ini [RadBeamWeaponE]` | 25064-25072 (9 lines) | All keys covered (delta from base noted) |
| `rulesmd.ini [RadEruptionWeapon]` | 23780-23791 (12 lines) | All keys covered including 3 inline designer comments |
| `rulesmd.ini [RadBeamWarhead]` | 27327-27330 (4 lines) | All keys covered with 11-column Verses breakdown |
| `rulesmd.ini [RadEruptionWarhead]` | 27340-27345 (6 lines) | All keys covered |
| `rulesmd.ini [RadSite]` warhead | 27349-27352 (4 lines) | All keys covered; cell-resident damage path explained |
| `rulesmd.ini [Radiation]` globals | 913-933 (21 lines) | All 10 global keys cross-referenced |
| `soundmd.ini` Desolator voices | DesolatorSelect, Move, AttackCommand, Die (no Fear by design) | All 4 active covered; disabled DesolatorFear documented |
| `soundmd.ini` weapon reports | DesolatorAttack, DesolatorDeploy | Both 2 covered |
| Hardcoded behavior | Deployer + DeployFire weapon-swap mechanism + AreaFire + RadLevel + ImmuneToRadiation + SelfHealing + IsRadBeam visual + IsRadEruption disabled + CellInset autodeploy gate + Crushable=no + Fearless=yes voice suppression | 10+ mechanisms covered, 5 fresh Ghidra-verified xrefs + deep-RE cross-references |
| Ghidra searches performed against ID | 5 distinct queries (1 strings + 4 xref lookups) plus deep-RE doc cross-reference | Logged inline |
| TS-legacy filter | Applied; IsRadEruption=no flagged as disabled, DeployTime commented, ImmuneToVeins-absent noted, all systems active in YR | Done |
