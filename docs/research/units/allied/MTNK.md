# MTNK — Grizzly Battle Tank (Allied MBT)

**Side classification:** Allied (Owner=British,French,Germans,Americans,Alliance).
**Role:** Allied Main Battle Tank. Affordable, fast, light-armored MBT designed for
swarming and mobility rather than head-on slugging. Counter-pair to the Soviet
Rhino (HTNK): cheaper ($700 vs $900), faster (Speed 7 vs 6), but weaker per-shot
and HP'd (Damage 65 vs 120, HP 300 vs 400).

> Output bar: standard combat unit. Damage curve, speed, and turret-rotation feel
> are all core parity targets — players feel the difference between a 2-Grizzly-vs-1-Rhino
> trade outcome and a 3-vs-1 one.

> Ghidra confirms `gamemd.exe` contains no `"MTNK"` or `"Grizzly"` strings — all
> behavior is generic TechnoType / UnitType flag-driven.

> Notable: rulesmd has `[MTNK]` (the Grizzly), artmd has `[MTNK]` labeled "Apocalypse
> tank". The shipped MTNK in rulesmd uses `Image=GTNK` (the "Grizzly Medium Tank" art
> block, which IS the Grizzly's live art entry). **Correction (added iteration 40)**:
> the artmd `[MTNK]` block is **NOT orphan** — it is the **Apocalypse Tank's live art**.
> APOC's rulesmd entry has `Image=MTNK`, redirecting to that block. The legacy naming
> is: in early RA2 the Apocalypse was "MTNK" (Medium Tank slot) and the Grizzly was
> "GTNK"; YR shipped renames the rulesmd entries (`[MTNK]` = Grizzly, `[APOC]` =
> Apocalypse) but preserves the original artmd entry names via `Image=` redirects.

---

## 1. `rulesmd.ini` — `[MTNK]` verbatim

```ini
[MTNK]
UIName=Name:MTNK
Name=Grizzly Battle Tank
Image=GTNK
Prerequisite=GAWEAP
Primary=105mm
Strength=300
Category=AFV
Armor=heavy
Turret=yes
IsTilter=yes
Crusher=yes
TooBigToFitUnderBridge=true
TechLevel=2
Sight=8
Speed=7
CrateGoodie=no
Owner=British,French,Germans,Americans,Alliance
Cost=700
Soylent=700
Points=25
ROT=5
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=GenAllVehicleSelect
VoiceMove=GenAllVehicleMove
VoiceAttack=GenAllVehicleAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=GrizzlyTankMoveStart
CrushSound=TankCrush
MaxDebris=2
; origional - Locomotor={55D141B8-DB94-11d1-AC98-006008055BB5}
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
;MovementZone=Destroyer ;gs FLAW needs to be changed to this when The Flaw is fixed
MovementZone=Normal
ThreatPosed=15	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Accelerates=false
ImmuneToVeins=yes
Size=3
OpportunityFire=yes
ElitePrimary=105mmE
BuildTimeMultiplier=1.5;Individual control of build time
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:MTNK` | AbstractType | CSF lookup. |
| `Name` | `Grizzly Battle Tank` | AbstractType | Dev fallback. |
| `Image` | `GTNK` | AbstractType | **Art-block redirect** — rulesmd MTNK uses artmd `[GTNK]` ("Grizzly Medium Tank") for rendering. The artmd `[MTNK]` block exists but is stale legacy data labeled "Apocalypse tank" — never consulted. |
| `Prerequisite` | `GAWEAP` | TechnoType | Only Allied War Factory needed — earliest-tier MBT, no tech-lab or radar gate. |
| `Primary` | `105mm` | TechnoType | Tank gun — see §3. |
| `Strength` | `300` | AbstractType | 300 HP. Mid-tier — between TNKD/Rhino (400) and Light Tank LTNK (~225). 3-hit kill from Rhino's 120mm; 5-hit from Grizzly's own 105mm. |
| `Category` | `AFV` | TechnoType | Armored Fighting Vehicle classifier. |
| `Armor` | `heavy` | TechnoType | Verses-slot 6. Tied with Rhino, TNKD, Apocalypse. |
| `Turret` | `yes` | UnitType | Rotating turret — gun can fire 360° while moving. Unlike TNKD (Turret=no), Grizzly can shoot-and-scoot effectively. |
| `IsTilter` | `yes` | UnitType | Voxel hull tilts on slopes (cosmetic). |
| `Crusher` | `yes` | TechnoType | Crushes infantry. |
| `TooBigToFitUnderBridge` | `true` | **UnitType only** (verified — 0x00845dc8 → UnitTypeClass__ReadINI @ 0x0074774e) | Cannot path under low railroad bridges. Pathfinder rejects bridge-under cells. Shared with TNKD, Rhino, Apocalypse, V3, Tesla Tank. |
| `TechLevel` | `2` | TechnoType | Tier-2 — combined with `GAWEAP`-only prereq, available very early. |
| `Sight` | `8` | TechnoType | 8-cell reveal — long enough that the tank's vision exceeds its 5-cell weapon range. Useful for spotting threats before they spot the tank. |
| `Speed` | `7` | TechnoType | **Fastest MBT in YR.** Compare: Rhino/Apocalypse=6, TNKD=5. Speed advantage is Grizzly's defining trait — flank, retreat, reposition. |
| `CrateGoodie` | `no` | UnitType | Cannot drop from crates. |
| `Owner` | 5 Allied countries | TechnoType | Allied only. |
| `Cost` | `700` | TechnoType | $200 cheaper than Rhino ($900) and TNKD ($900). Designed to be spammable. |
| `Soylent` | `700` | TechnoType | 100% Grinder refund (irrelevant — Allied has no Grinder; relevant only on capture). |
| `Points` | `25` | TechnoType | Score on kill (same as Rhino/TNKD). |
| `ROT` | `5` | TechnoType | Turret rotation rate (also body rotation since no `TurretROT=` override). 5 is mid — quick enough to track moving targets. |
| `IsSelectableCombatant` | `yes` | TechnoType | Counted in select-all-combat hotkey. |
| `Explosion` | `TWLT070,...` | TechnoType | Random-from-list death anim. |
| `VoiceSelect` | `GenAllVehicleSelect` | TechnoType | Generic Allied vehicle voice (5 clips, `$vgrasea..ee`) — **shared with all Allied tanks/vehicles**. No unique Grizzly voice set. |
| `VoiceMove` | `GenAllVehicleMove` | TechnoType | Generic Allied move (6 clips). |
| `VoiceAttack` | `GenAllVehicleAttackCommand` | TechnoType | Generic Allied attack (5 clips). |
| `VoiceFeedback` | *(empty)* | TechnoType | No "under attack" voice. |
| `DieSound` | `GenVehicleDie` | TechnoType | Standard 6-clip vehicle death. |
| `MoveSound` | `GrizzlyTankMoveStart` | TechnoType | **Unique** engine-start sound (3 clips, random predelay 0–400ms, low priority, FShift ±10, VShift +15, vol 40). Distinguishes Grizzly audio from Rhino's `RhinoTankMoveStart` despite shared voice set. |
| `CrushSound` | `TankCrush` | TechnoType | Standard crush sound. |
| `MaxDebris` | `2` | TechnoType | Just 2 debris pieces on death. |
| Commented `; origional - Locomotor={55D141B8-...}` | — | — | Author-note: original-RA2 locomotor CLSID was different. Switched to standard drive locomotor. |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass. |
| Commented `;MovementZone=Destroyer ;gs FLAW needs to be changed to this when The Flaw is fixed` | — | — | Author-note: there's an unfixed pathfinding "FLAW" that prevents Destroyer zone. Live: `Normal` zone. |
| `MovementZone` | `Normal` | TechnoType | Standard land vehicle path — cannot path through crushable obstacles (must drive around walls, unlike harvesters). |
| `ThreatPosed` | `15` | TechnoType | Mid-tier AI threat. Lower than Rhino (40) since the per-shot damage is lower. |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | Smoke/spark emitters when damaged. |
| `VeteranAbilities` | `STRONGER,FIREPOWER,SIGHT,FASTER` | TechnoType | Veteran bonuses. **No `ROF`** — Grizzly doesn't get faster firing at veteran. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Elite adds SELF_HEAL and ROF. |
| `Accelerates` | `false` | TechnoType | **No acceleration ramp** — moves at top speed immediately when ordered. Same as Rhino. Makes Speed=7 actually feel like 7 from frame 1, no startup lag. |
| `ImmuneToVeins` | `yes` | TechnoType | **TS-LEGACY** dormant. |
| `Size` | `3` | TechnoType | Transport slot cost. Cannot fit in SAPC (size 2 max), can fit in Battle Fortress / Nighthawk. |
| `OpportunityFire` | `yes` | TechnoType [BINARY-VERIFIED audit 15: string @ 0x00843A74, parser xref @ 0x0071483D, `TechnoType+0x6AF` (byte)] | **Will auto-target threats in range without explicit attack order.** Critical for Grizzly's role: a moving Grizzly takes opportunistic shots at enemies it passes, which is what makes "tank rush" feel responsive. |
| `ElitePrimary` | `105mmE` | TechnoType | Elite weapon (Damage unchanged at 65, but Burst=2, different warhead — see §3). |
| `BuildTimeMultiplier` | `1.5` | TechnoType [BINARY-VERIFIED audit 15: string @ 0x00843CF0, parser xref @ 0x00714371, `TechnoType+0x608` (float bits stored as int via ReadDouble cast)] | INI comment: "Individual control of build time". Grizzly's build time is **1.5× the default** for its cost — slower than HARV/CMIN per-dollar despite being cheaper. Compensates for the spammability. Default build time is derived from `[General] BuildSpeed=`. |

### Notable absent keys
- No `Secondary=` — single weapon, no AA.
- No `Spawns=` — no child units.
- No `Passengers=` — not a transport.
- No `Teleporter=` — doesn't chrono-warp.
- No `Bunkerable=no` — defaults to yes; Grizzly CAN be loaded into Battle Fortress.

---

## 2. `artmd.ini` — referenced via `Image=GTNK`

MTNK's `Image=GTNK` redirects to:

```ini
[GTNK]   ; Grizzly Medium Tank
Voxel=yes
Remapable=yes
Cameo=GTNKICON
AltCameo=GTNKUICO
PrimaryFireFLH=150,0,100
```

| Key | Value | Effect |
|-----|-------|--------|
| `Voxel` | `yes` | Voxel-rendered from `GTNK.VXL` + `GTNK.HVA`. |
| `Remapable` | `yes` | House-color remap. |
| `Cameo` | `GTNKICON` | Sidebar build cameo. |
| `AltCameo` | `GTNKUICO` | Yuri-skinned cameo (if captured by Yuri). |
| `PrimaryFireFLH` | `150,0,100` | Firing offset: X=+150 (long tank barrel), Y=0 (centred), Z=+100 (turret-cannon height). |

No `TurretOffset=` — defaults to the voxel's hardcoded turret pivot. No
`SecondaryFireFLH=` since no Secondary weapon.

### artmd `[MTNK]` block — NOT consulted by Grizzly (consulted by Apocalypse via `Image=MTNK`)

```ini
[MTNK]   ; Apocalypse tank
Voxel=yes
Remapable=yes
Cameo=MTNKICON
AltCameo=MTNKUICO
PrimaryFireFLH=190,25,120
```

The artmd `[MTNK]` block is the **Apocalypse Tank's live art entry** — not consulted by
the rulesmd Grizzly (which redirects to `[GTNK]` via `Image=GTNK`). The comment
"Apocalypse tank" is accurate: this IS the Apocalypse's art. See [`soviet/APOC.md`](../soviet/APOC.md) §2 for the Apocalypse-side consumer.

**Parity-critical: honor `Image=` redirects.** If your engine reads artmd `[MTNK]`
directly without honoring rulesmd's `Image=` redirect, the Grizzly will incorrectly
inherit the Apocalypse's voxel parameters (wrong cameo `MTNKICON`, wrong FLH
`190,25,120`), and the Apocalypse may render with the wrong art entirely.

---

## 3. Weapon — `[105mm]` / `[105mmE]`

### `[105mm]` (rookie & veteran)

```ini
[105mm]
Damage=65
ROF=60
Range=5
Projectile=Cannon
Speed=40
Warhead=AP
Report=GrizzlyTankAttack
Anim=GUNFIRE
Bright=yes
```

### `[105mmE]` (elite)

```ini
[105mmE]
Damage=65
ROF=50
Range=5
Projectile=Cannon
Speed=40
Warhead=GRIZAPE
Report=GrizzlyTankAttack
Anim=VTMUZZLE
Bright=yes
Burst=2
```

| Key | 105mm | 105mmE | Effect |
|-----|-------|--------|--------|
| `Damage` | 65 | **65** | Unchanged at elite — but warhead change + Burst makes elite far stronger |
| `ROF` | 60 | **50** | Elite fires faster (-17% cooldown) |
| `Range` | 5 | 5 | Unchanged |
| `Projectile` | `Cannon` (arcing) | `Cannon` | Same — both use the arcing cannon projectile |
| `Speed` | 40 | 40 | Bullet speed |
| `Warhead` | `AP` | **`GRIZAPE`** | **Elite swaps to GRIZAPE warhead** (radically different Verses — see §4) |
| `Report` | `GrizzlyTankAttack` | same | Per-shot sound |
| `Anim` | `GUNFIRE` | **`VTMUZZLE`** | Elite uses Vehicle Tank Muzzle anim (different visual) |
| `Bright` | yes | yes | Lights the cell on fire |
| `Burst` | (absent → 1) | **2** | **Elite fires 2 shots per cycle** — biggest single jump |

**Practical DPS** (vs `none` armor, 100% Verses for both AP and GRIZAPE):
- Rookie: 65 × 1 / 60 = 1.08 dmg/tick
- Elite: 65 × 2 / 50 = 2.60 dmg/tick → **~2.4× DPS at elite**

Vs `heavy` armor (Rhino-style — AP=100% but GRIZAPE=100%): same multiplier, so elite is 2.4× damage to other MBTs. Combined with `SELF_HEAL` and `STRONGER` from veterancy, an elite Grizzly is genuinely fearsome.

### 3.1 Projectile — `[Cannon]`

```ini
[Cannon]
Image=120MM
Arcing=true
SubjectToCliffs=yes
SubjectToElevation=yes
SubjectToWalls=yes
```

Same arcing-cannon projectile used by TNKD (`SABOT`), HARV (elite `20mmRapidE`), Rhino (`120mm`). Arc respects cliffs/elevation/walls — Grizzly can lob over low cover.

### 3.2 Muzzle animations

`[GUNFIRE]` — translucent ground-layer muzzle flash.

`[VTMUZZLE]` — Normalized vehicle-tank muzzle (Normalized=yes, otherwise defaults).

---

## 4. Warheads — `[AP]` / `[GRIZAPE]`

### `[AP]` (rookie & veteran)

```ini
[AP]
CellSpread=.3
PercentAtMax=.5
Wall=yes
Wood=yes
;DB Changed AP shot on 6/6/01 to make plate armor almost immune to attacks by AP weapons.
;Verses=25%,25%,25%,75%,100%,100%,65%,45%,60%,60%,100%
Verses=25%,25%,15%,75%,100%,100%,65%,45%,60%,60%,100%
Conventional=yes
InfDeath=3
AnimList=S_CLSN16,S_CLSN22
ProneDamage=50%
```

| Slot | Armor | Damage | Notes |
|------|-------|--------|-------|
| 1 | none | 25% | Weak vs basic infantry (1-shot would require 4 hits) |
| 2 | flak | 25% | Weak vs Flak Trooper armor |
| 3 | plate | **15%** | Very weak vs Tanya/SEAL/CCOMAND — the INI comment-history shows this was nerfed from 25% (effectively almost immune). Plate armor was made "almost immune to attacks by AP weapons" on 6/6/01 per the author note. |
| 4 | light | 75% | Strong vs Grizzly/Mirage/IFV (light-armor light tanks) |
| 5 | medium | 100% | Full damage vs medium-armor units |
| 6 | heavy | 100% | Full damage vs heavy-armor MBTs (Rhino, TNKD, Apocalypse, Grizzly itself) |
| 7 | wood | 65% | Decent vs civilian buildings |
| 8 | steel | 45% | Mid vs steel buildings |
| 9 | concrete | 60% | Mid vs concrete fortifications |
| 10 | special_1 | 60% | (Terror Drone?) |
| 11 | special_2 | 100% | |

| Key | Effect |
|-----|--------|
| `CellSpread` | `.3` — small AoE (0.3-cell radius) |
| `PercentAtMax` | `.5` — 50% damage at AoE edge |
| `Wall=yes` | Damages walls |
| `Wood=yes` | Sets wood-armor structures on fire |
| `Conventional=yes` | Standard kinetic (vs psychic/nuclear) |
| `InfDeath` | `3` — explosion death (RPG/Cannon-style) |
| `AnimList` | `S_CLSN16, S_CLSN22` — random pick from 2 collision spark anims |
| `ProneDamage` | `50%` — infantry in prone state take half damage |

### `[GRIZAPE]` (elite only)

```ini
[GRIZAPE]
CellSpread=.3
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=100%,100%,100%,100%,100%,100%,65%,45%,60%,60%,100%
Conventional=yes
InfDeath=3
AnimList=VTEXPLOD
ProneDamage=50%
```

| Slot | Armor | Damage | Δ vs AP |
|------|-------|--------|---------|
| 1 | none | **100%** | +75% — elite Grizzly now full-damages basic infantry |
| 2 | flak | **100%** | +75% |
| 3 | plate | **100%** | +85% — plate-armor immunity REMOVED at elite (Tanya/SEAL eat full damage) |
| 4 | light | **100%** | +25% |
| 5 | medium | 100% | unchanged |
| 6 | heavy | 100% | unchanged |
| 7 | wood | 65% | unchanged |
| 8 | steel | 45% | unchanged |
| 9 | concrete | 60% | unchanged |
| 10 | special_1 | 60% | unchanged |
| 11 | special_2 | 100% | unchanged |

GRIZAPE = "Grizzly Armor Piercing Elite". The defining trait: **infantry slots all jump to 100%**. Rookie/veteran Grizzlies barely scratch infantry; elite Grizzlies wreck them. Combined with `Burst=2` and faster ROF, an elite Grizzly is a serious infantry-killer. Also the only AnimList change: `VTEXPLOD` (Vehicle Tank Explosion) instead of small-arms `S_CLSN16/22`.

---

## 5. Voices / sounds

```ini
[GenAllVehicleSelect]
Sounds= $vgrasea $vgraseb $vgrasec $vgrased $vgrasee
Control= random
Volume=85

[GenAllVehicleMove]
Sounds= $vgramoa $vgramob $vgramoc $vgramod $vgramoe $vgramof
Control= random
Volume=85

[GenAllVehicleAttackCommand]
Sounds= $vgraata $vgraatb $vgraatc $vgraatd $vgraate
Control= random
Volume=85
```

```ini
[GrizzlyTankMoveStart]
Sounds= vgristaa vgristab vgristac
Control= random predelay
Delay=0 400
Priority=low
FShift= -10 10
VShift=15
Volume=40
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=GenAllVehicleSelect` | 5 clips ($vgrasea..ee) | Click-select — shared with all Allied tanks |
| `VoiceMove=GenAllVehicleMove` | 6 clips | Move order — shared |
| `VoiceAttack=GenAllVehicleAttackCommand` | 5 clips | Attack order — shared |
| `VoiceFeedback=` *(empty)* | — | No under-attack voice |
| `DieSound=GenVehicleDie` | 6 clips, FShift ±15 | Death |
| `MoveSound=GrizzlyTankMoveStart` | **3 clips (unique)**, random predelay 0–400ms, low priority, FShift ±10, VShift +15, vol 40 | Engine-start when movement begins — the only Grizzly-specific audio |
| `CrushSound=TankCrush` | `vcrusha` | When Grizzly crushes infantry |
| `Report=GrizzlyTankAttack` (weapon) | (in soundmd) | Per-shot fire sound |

The Grizzly is audibly distinguishable from Rhino primarily by `GrizzlyTankMoveStart` vs Rhino's `RhinoTankMoveStart` — the engine ignition has a different timbre/pitch. Selection/attack voices are shared across all Allied tanks.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `GAWEAP` — Allied War Factory only. **No radar, no Service Depot.** Earliest-tier MBT.
- **TechLevel** = `2`.
- **Owner**: 5 Allied countries.
- **`CrateGoodie=no`** — explicitly excluded from crate pool.
- **`AllowedToStartInMultiplayer=` is absent** — meaning the unit IS allowed for start placement, but practically Allied players never start with preplaced Grizzlies (only the AMCV → GACNST → free CMIN starter sequence).
- **`BuildTimeMultiplier=1.5`** — actual build time is 1.5× the cost-derived default. Slows down the spam-rate to balance the cheap $700 cost.

### Grizzly vs Rhino — combat matchup

| Aspect | Grizzly (MTNK) | Rhino (HTNK) |
|--------|----------------|---------------|
| Cost | $700 | $900 |
| HP | 300 | 400 |
| Armor | heavy | heavy |
| Primary | 105mm Damage=65, ROF=60, Range=5 | 120mm Damage=90+, ROF=80+, Range=5+ (see HTNK doc) |
| Warhead | AP / GRIZAPE elite | various / 120mmE |
| Speed | **7** | 6 |
| Turret | yes | yes |
| ROT | 5 | 5 |
| BuildTimeMultiplier | 1.5 | 1.3 |
| Crusher | yes | yes |
| TooBigToFitUnderBridge | true | true |
| Burst (elite) | 2 | 1 |
| OpportunityFire | yes | yes |

**Trade math (1v1, no veterancy)**: Grizzly does 65 dmg/60 ticks = 1.08 dmg/tick; Rhino does ~120 dmg/80 ticks = 1.50 dmg/tick (estimate; see HTNK doc for exact). Grizzly's 300 HP / 1.50 = 200 ticks to die. Rhino's 400 HP / 1.08 = 370 ticks to die. **Grizzly loses 1v1 by ~170 ticks (~10 seconds)** of "wasted" damage. To trade favorably, Allied needs 2 Grizzlies vs 1 Rhino at minimum, with the cost advantage ($1400 vs $900 — Allied loses $500) — or to exploit Grizzly's speed advantage to retreat damaged units.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 MTNK-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `MTNK` | 0 matches |
| `Grizzly` | 0 matches |

⇒ **No MTNK-specific code path.** All behavior is generic flag-driven.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `TooBigToFitUnderBridge` | 0x00845dc8 | UnitTypeClass__ReadINI @ 0x0074774e | **UnitType only** (not TechnoType — only vehicles can have this flag) |
| `BuildTimeMultiplier` | 0x00843cf0 | TechnoTypeClass__ReadINI @ 0x00714371 | TechnoType |
| `OpportunityFire` | 0x00843a74 | TechnoTypeClass__ReadINI @ 0x0071483d | TechnoType |

Plus prior verifications:
- `IsTilter` — UnitType (cheat sheet)
- `Crusher`, `Turret`, `Image=`, `Armor`, etc. — TechnoType/AbstractType
- `OmniCrusher` / `OmniCrushResistant` — TechnoType (relevant: Grizzly doesn't have `OmniCrushResistant=yes` so Battle Fortress can squish it)

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Cheap, fast Allied MBT | Cost=700, Speed=7, no acceleration ramp | Designed for swarm |
| Auto-targets passing threats | `OpportunityFire=yes` | Tank rush "just works" — players don't need to micro-attack each engagement |
| Cannot path under bridges | `TooBigToFitUnderBridge=true` | Pathfinder rejects bridge-under cells |
| Crushes infantry | `Crusher=yes`, default `Crushable=yes` | Standard crush; but Battle Fortress can OmniCrush Grizzly since no `OmniCrushResistant=yes` |
| Voxel hull tilts on slope | `IsTilter=yes` | Cosmetic |
| Elite weapon: 2× Burst + GRIZAPE warhead | `ElitePrimary=105mmE` with `Burst=2` and `Warhead=GRIZAPE` | Most dramatic veterancy upgrade — see §3 |
| Slower-than-cost build time | `BuildTimeMultiplier=1.5` | Balance throttle |
| No XP earned from non-combat | `Trainable` default yes — Grizzly CAN gain veterancy | Combat XP is normal for an MBT |

### 7.4 Behaviors NOT present

- **No `OmniCrushResistant=yes`** — Battle Fortress squishes Grizzly (unlike MCV).
- **No `Spawns=`** — no child units.
- **No `Passengers=`** — not a transport.
- **No `Secondary=`** — single weapon, no anti-air, no anti-naval.
- **No `Teleporter=`** — does not chrono-warp on its own (Chronosphere can teleport it like any unit).
- **No `Bunkerable=no`** — defaults yes; Grizzly CAN board Battle Fortress.
- **No `SelfHealing=yes`** at rookie/veteran — only at elite via `EliteAbilities=SELF_HEAL`.
- **No `ImmuneToPsionics`** — Grizzly CAN be mind-controlled by Yuri.
- **No `ImmuneToRadiation`** — Desolator rad damages Grizzly.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ImmuneToVeins=yes` | YES | Dormant. |
| Commented `;MovementZone=Destroyer` | n/a | Author-acknowledged unfixed pathfinding "FLAW" — Destroyer zone would be ideal but is broken in YR. Live: Normal zone. |
| Commented `; origional - Locomotor={55D141B8-...}` | n/a (history) | Old TS-style locomotor CLSID, replaced. |

---

## 9. Veterancy

### Veteran (1 chevron) — `STRONGER, FIREPOWER, SIGHT, FASTER`
- `STRONGER` — +25% HP (300 → 375)
- `FIREPOWER` — +25% damage (65 → 81)
- `SIGHT` — +20% sight (8 → 9.6)
- `FASTER` — +20% speed (7 → 8.4 — fastest tank in the game)

**Net at veteran**: Still uses `[105mm]` weapon (AP warhead), but stat-buffed. Plate armor still resists.

### Elite (2 chevrons) — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative)
- Adds `SELF_HEAL` (passive HP regen from `[General] SelfHealUnitRate`)
- `STRONGER` and `FIREPOWER` reapplied (token semantics — see TNKD §9 open follow-up)
- `ROF` — −25% ROF (faster cooldown)

**Plus weapon swap**: `[105mm]` → `[105mmE]`:
- ROF 60 → 50
- Burst 1 → **2** (doubles per-cycle damage)
- Warhead AP → **GRIZAPE** (infantry slots 100% instead of 25%)
- Anim GUNFIRE → VTMUZZLE

**Practical elite jump**: ~2.4× sustained DPS, infantry slot Verses 25%→100% (4× damage to soft targets), self-heal, ROF acceleration. Elite Grizzly is arguably the best per-cost late-game MBT in the Allied lineup. The 105mmE Burst=2 + GRIZAPE combo is what makes veteran Grizzlies viable into endgame.

---

## 10. Cross-references

### Direct dependencies
- `[105mm]` / `[105mmE]` — weapons (§3)
- `[Cannon]` — projectile
- `[AP]` / `[GRIZAPE]` — warheads (§4)
- `[120MM]` (artmd) — bullet sprite
- `[GUNFIRE]` / `[VTMUZZLE]` (artmd) — muzzle anims
- `[S_CLSN16] / [S_CLSN22] / [VTEXPLOD]` (artmd) — impact anims
- `[GTNK]` (artmd via `Image=GTNK`) — art block
- `[GAWEAP]` — prereq
- `[GenAllVehicleSelect/Move/AttackCommand]` (soundmd) — voices
- `[GrizzlyTankMoveStart]` (soundmd) — unique engine sound
- `[GenVehicleDie] / [TankCrush]` (soundmd) — generic vehicle sounds

### Conceptual companions
- **HTNK (Rhino)** ([`soviet/HTNK.md`](../soviet/HTNK.md) — TODO) — direct counter-pair Soviet MBT.
- **TNKD (Tank Destroyer)** ([`allied/TNKD.md`](./TNKD.md)) — German Allied AT-only tank. Different role (no turret, anti-armor only).
- **HOWI (Howitzer)** ([`allied/HOWI.md`](./HOWI.md) — TODO) — Allied artillery (`Image=HWTZ`, also TechLevel=-1 in current INI).
- **MGTK (Mirage Tank)** ([`allied/MGTK.md`](./MGTK.md) — TODO) — Allied tier-2.5 tank with tree disguise.

### Deep-RE docs
- None directly relevant — MTNK has no unique hardcoded behavior worth a dedicated report.

---

## Ghidra audit log (audit iteration 15 — 2026-05-18)

**Methodology**: MTNK has no unit-specific code in `gamemd.exe`
(confirmed by string-search). The doc cites 3 specific Ghidra parser
xrefs (TooBigToFitUnderBridge, BuildTimeMultiplier, OpportunityFire);
this audit re-verifies them and pins their struct offsets. ~10 Ghidra
queries: 5 string-searches + 3 xref lookups + 1 grep on saved
TechnoTypeClass__ReadINI decompile + 1 INI cross-check.

### Negative claims re-verified

| Query | Result |
|-------|--------|
| `search_strings("^MTNK$")` | **0 matches** |
| `search_strings("^Grizzly$")` | **0 matches** |

Confirms: no hardcoded section-name branch, no `Grizzly`-keyword
behavior gate. All MTNK behavior is data-driven via standard
TechnoType/UnitType/AbstractType flag handling.

### String + parser xref re-verification (BINARY-VERIFIED)

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `TooBigToFitUnderBridge` | 0x00845DC8 | 0x0074774E | UnitTypeClass__ReadINI |
| `BuildTimeMultiplier` | 0x00843CF0 | 0x00714371 | TechnoTypeClass__ReadINI |
| `OpportunityFire` | 0x00843A74 | 0x0071483D | TechnoTypeClass__ReadINI |

All 3 string addresses and parser-function names match the doc's claims
exactly.

### Struct offsets BINARY-VERIFIED (this pass)

| Class | Offset | INI key | Type | Source |
|-------|--------|---------|------|--------|
| TechnoType | `+0x608` | `BuildTimeMultiplier` | float-as-int | `param_1[0x182] = (int)(float)extraout_ST0` after `CCINIClass::ReadDouble`. **NEW**. |
| TechnoType | `+0x6AF` | `OpportunityFire` | byte | `*(char*)((int)param_1 + 0x6af) = (char)uVar5` after `CCINIClass::ReadBool`. **NEW**. |
| UnitType | `+0xE16` | `TooBigToFitUnderBridge` | byte | Re-confirms audit 12 cumulative (UnitType-scope, not TechnoType). |

### Cross-INI verification of doc's `Image=MTNK` claim

The doc claims "artmd `[MTNK]` block is the Apocalypse's live art
because APOC's rulesmd has `Image=MTNK`". Verified:
`c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:7791` reads
`Image=MTNK` inside the `[APOC]` block (lines 7788–7827). Confirms the
doc's interpretation — Grizzly's `Image=GTNK` redirects to artmd
`[GTNK]`, while Apocalypse's `Image=MTNK` consumes the stale-labeled
artmd `[MTNK]` block. No engine-level Ghidra change needed; this is
data-driven via the `Image=` resolver (TechnoType-scope, AbstractType
field — already covered in cumulative for AMCV audit 14's
`Image=MCV` discussion).

### Items NOT re-verified in this pass (DEFERRED)

- `BuildTimeMultiplier` consumer in build-queue timer logic (the
  `(Cost / [General] BuildSpeed) × BuildTimeMultiplier` formula
  hypothesized in the doc) — offset is verified, consumer chain is not.
- `OpportunityFire` consumer in the auto-targeting/scan path — offset
  verified, runtime gating function DEFERRED.
- `Image=` resolver function in the AbstractType layer — already
  cross-referenced indirectly via AMCV audit 14's `Image=MCV`
  observation; not re-decompiled this pass.
- Turret-vs-Body rotation distinction (`Turret=yes` MTNK vs `Turret=no`
  TNKD) — TechnoType+0xCA1 verified in audit 12, consumer in
  `UnitClass::Fire_At_Target` / `Facing_Update` DEFERRED.
- `BuildTimeMultiplier`'s float-vs-int storage convention — Ghidra
  shows `param_1[0x182] = (int)(float)extraout_ST0`, which is a
  float-to-int reinterpret-cast (not a truncation). The 4 bytes at
  +0x608 hold the IEEE-754 bit pattern of a float, despite the
  field-store being typed `int`. Pattern is unusual but consistent with
  similar `float-stored-as-int` fields elsewhere in TechnoTypeClass.

### Confidence summary

- **HIGH**: 5 string addresses (all exact); 3 parser xrefs (all exact);
  2 NEW TechnoType struct offsets (BuildTimeMultiplier +0x608,
  OpportunityFire +0x6AF — both read directly from
  TechnoTypeClass__ReadINI decompile); 1 UnitType re-confirmation
  (TooBigToFitUnderBridge +0xE16 from audit 12); 1 cross-INI
  confirmation (APOC has `Image=MTNK` at rulesmd:7791).
- **No INCORRECT findings in the doc**. The 3 in-line Ghidra cites all
  resolve exactly. The "stale `[MTNK]` artmd block is Apocalypse art"
  story is consistent with the INI evidence.
- **No new vtable / RTTI / phantom findings** (MTNK doc doesn't claim
  any).

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[MTNK]` rulesmd key explained | ✅ §1 |
| `Image=GTNK` redirect + stale `[MTNK]` artmd block noted as parity hazard | ✅ §2 |
| Both weapons (rookie + elite) + both warheads + projectile | ✅ §3–§4 |
| Detailed Verses delta tables (AP vs GRIZAPE — biggest elite jump in infantry slots) | ✅ §4 |
| All voices + unique GrizzlyTankMoveStart sound | ✅ §5 |
| Prereqs / owners / availability | ✅ §6 |
| **Grizzly vs Rhino combat math** (trade ratio, cost effective) | ✅ §6 |
| Hardcoded behavior — Ghidra searches + 3 flag-scope verifications | ✅ §7 (TooBigToFitUnderBridge confirmed UnitType-only, BuildTimeMultiplier + OpportunityFire confirmed TechnoType) |
| TS-legacy filter | ✅ §8 |
| Veterancy detailed with elite-weapon delta | ✅ §9 |
| Cross-refs to companion docs | ✅ §10 |

**Open follow-ups (none load-bearing):**
- Verify `BuildSpeed` global formula: actual build time = `(Cost / [General] BuildSpeed) × BuildTimeMultiplier`? Confirm via decompile of build-queue timer logic.
- The unfixed pathfinding "FLAW" mentioned in the commented `;MovementZone=Destroyer` line is an open YR issue. Worth a dedicated investigation if pathing parity bugs surface with Grizzly behavior near walls/destroyers.
- Verify `Accelerates=false` semantics — does it truly mean "no acceleration ramp" or just "no separate accel value"? Cross-reference with [VehicleClass] processing.
- Confirm the Apocalypse Tank (APOC) does NOT consult the stale artmd `[MTNK]` block — should use its own `Image=APOC` entry.
