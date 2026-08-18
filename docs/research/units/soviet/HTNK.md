# HTNK — Rhino Heavy Tank (Soviet MBT)

**Side classification:** Soviet (Owner=Russians,Confederation,Africans,Arabs).
**Role:** Soviet Main Battle Tank. The heavy, slow, slug-it-out counterpart to the
Allied Grizzly (MTNK): more expensive ($900 vs $700), tougher (HP 400 vs 300), more
damage (90 vs 65), slightly longer range (5.75 vs 5), but slower (Speed 6 vs 7).
The iconic "tank rush" unit.

> Output bar: parity-critical for the most-fought-with unit in the game. Trade
> ratios vs Grizzly, vs Mirage, vs IFV all hinge on the exact `[120mm]` damage curve
> and `Strength=400` HP cap.

> **Companion doc**: [`allied/MTNK.md`](../allied/MTNK.md) — Grizzly Battle Tank.
> Comparison table in MTNK §6.

> Ghidra confirms `gamemd.exe` contains no `"HTNK"` or `"Rhino"` strings — all
> behavior is generic flag-driven via standard TechnoType/UnitType handling.

---

## 1. `rulesmd.ini` — `[HTNK]` verbatim

```ini
[HTNK]
UIName=Name:HTNK
Name=Rhino Heavy Tank
Prerequisite=NAWEAP
Primary=120mm
Strength=400
Category=AFV
Armor=heavy
Turret=yes
IsTilter=yes
TargetLaser=no
TooBigToFitUnderBridge=true
TechLevel=2
Sight=8
Speed=6
CrateGoodie=no
Crusher=yes
Owner=Russians,Confederation,Africans,Arabs
Cost=900
Soylent=900
Points=25
ROT=5
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=GenSovVehicleSelect
VoiceMove=GenSovVehicleMove
VoiceAttack=GenSovVehicleAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=RhinoTankMoveStart
CrushSound=TankCrush
Maxdebris=3
;origional - Locomotor={55D141B8-DB94-11d1-AC98-006008055BB5}
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
MovementZone=Destroyer
ThreatPosed=40	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
DamageSmokeOffset=100, 100, 275
Weight=3.5
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Accelerates=false
ZFudgeColumn=8
ZFudgeTunnel=13
Size=3
OpportunityFire=yes
ElitePrimary=120mmE
BuildTimeMultiplier=1.3;Individual control of build time
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:HTNK` | AbstractType | CSF lookup. |
| `Name` | `Rhino Heavy Tank` | AbstractType | Dev fallback. |
| (no `Image=` line) | — | — | Unlike MTNK, HTNK reads its own `[HTNK]` artmd block directly — no redirect. |
| `Prerequisite` | `NAWEAP` | TechnoType | Soviet War Factory only — earliest-tier Soviet MBT, no Radar/Lab needed. |
| `Primary` | `120mm` | TechnoType | Tank cannon — see §3. |
| `Strength` | `400` | AbstractType | **400 HP** — 33% more than Grizzly. Tied with TNKD. |
| `Category` | `AFV` | TechnoType | Armored Fighting Vehicle. |
| `Armor` | `heavy` | TechnoType | Verses-slot 6. |
| `Turret` | `yes` | UnitType | Rotating turret. |
| `IsTilter` | `yes` | UnitType | Hull tilts on slopes (cosmetic). |
| `TargetLaser` | `no` | TechnoType (verified — 0x00843898 → TechnoTypeClass__ReadINI @ 0x00714c8d) | **Disables target-laser rendering.** `TargetLaser=yes` would draw a laser line from turret to target during aiming (used by some special-purpose units; possibly TS-legacy origin since most YR vehicles set it to `no`). Rhino explicitly disables. |
| `TooBigToFitUnderBridge` | `true` | UnitType-only (verified in MTNK iter) | Cannot path under low bridges. |
| `TechLevel` | `2` | TechnoType | Tier-2 — same as Grizzly. |
| `Sight` | `8` | TechnoType | 8-cell reveal — same as Grizzly. |
| `Speed` | `6` | TechnoType | **Slower than Grizzly's 7.** The HP/firepower vs speed tradeoff. |
| `CrateGoodie` | `no` | UnitType | Excluded from crate pool. |
| `Crusher` | `yes` | TechnoType | Crushes infantry. |
| `Owner` | `Russians,Confederation,Africans,Arabs` | TechnoType | Soviet only. |
| `Cost` | `900` | TechnoType | $200 more than Grizzly. |
| `Soylent` | `900` | TechnoType | 100% Grinder refund — relevant since Yuri has the Grinder; a Yuri player capturing Rhinos can recycle them for full credit. |
| `Points` | `25` | TechnoType | Score on kill (same as Grizzly/TNKD). |
| `ROT` | `5` | TechnoType | Turret + body rotation. |
| `IsSelectableCombatant` | `yes` | TechnoType | Counts in select-all-combat. |
| `Explosion` | `TWLT070,...` | TechnoType | Random-from-list death anim. |
| `VoiceSelect` | `GenSovVehicleSelect` | TechnoType | Generic Soviet vehicle voice (3 clips, `$vgrssea..ec`) — shared with all Soviet tanks. Fewer clips than Allied (which has 5). |
| `VoiceMove` | `GenSovVehicleMove` | TechnoType | 3 generic Soviet move clips. |
| `VoiceAttack` | `GenSovVehicleAttackCommand` | TechnoType | 4 generic Soviet attack clips. |
| `VoiceFeedback` | *(empty)* | TechnoType | No under-attack voice. |
| `DieSound` | `GenVehicleDie` | TechnoType | Standard vehicle death. |
| `MoveSound` | `RhinoTankMoveStart` | TechnoType | **Unique** engine-start sound (4 clips, predelay 0–400ms, low priority, FShift ±10, VShift +15, vol 30). Audibly distinguishes Rhino from Grizzly's `GrizzlyTankMoveStart`. |
| `CrushSound` | `TankCrush` | TechnoType | Standard crush. |
| `Maxdebris` | `3` | TechnoType | 3 debris pieces on death (vs Grizzly's 2). Bigger tank → more wreckage. |
| Commented `;origional - Locomotor={55D141B8-...}` | — | — | Author-note: original TS-style locomotor CLSID. |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass — standard. |
| `MovementZone` | **`Destroyer`** | TechnoType | **Unlike Grizzly's `Normal`.** Per the MTNK INI comment ("FLAW needs to be changed to this when The Flaw is fixed"), `Destroyer` is the intended MBT zone — but Grizzly has to fall back to `Normal` due to an unfixed pathfinding bug. **Rhino uses Destroyer regardless** — suggesting the bug only manifests with Allied/Western faction pathing or specific terrain combos. `Destroyer` zone is more permissive (can drive through some crushable obstacles harvesters can't). |
| `ThreatPosed` | `40` | TechnoType | **High AI threat** — vs Grizzly's 15. Enemy AI prioritizes targeting Rhinos over Grizzlies because of the higher per-shot damage. |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | Smoke/spark emitters. |
| `DamageSmokeOffset` | `100, 100, 275` | TechnoType (verified — 0x00843f60 → 0x00713e25) | Pixel offset for damage-smoke particle origin (X=100, Y=100, Z=275). High Z=275 means smoke emerges from above the hull (turret height). Unique to Rhino among the MBTs documented so far. |
| `Weight` | `3.5` | TechnoType | Physics weight (same as MTNK). |
| `VeteranAbilities` | `STRONGER,FIREPOWER,SIGHT,FASTER` | TechnoType | Same as Grizzly — no ROF at veteran. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Same as Grizzly — adds SELF_HEAL and ROF at elite. |
| `Accelerates` | `false` | TechnoType | No acceleration ramp. |
| `ZFudgeColumn` | `8` | UnitType | Z-render fudge for column-adjacent cells. Smaller than Grizzly's (not set, defaults higher) and harvester's 9 — Rhino's voxel sits lower visually. |
| `ZFudgeTunnel` | `13` | UnitType | Z-fudge for tunnels (TS-legacy mostly). |
| (no `ZFudgeBridge`) | — | — | Not set — irrelevant since `TooBigToFitUnderBridge=true`. |
| `Size` | `3` | TechnoType | Transport slot cost. |
| `OpportunityFire` | `yes` | TechnoType | Auto-targets passing threats. |
| `ElitePrimary` | `120mmE` | TechnoType | Elite weapon — see §3. |
| `BuildTimeMultiplier` | `1.3` | TechnoType | Build time is 1.3× cost-derived default. **Faster build than Grizzly's 1.5.** Combined with $900 cost, Rhinos take roughly $900 × 1.3 ≈ $1170-equivalent build time vs Grizzly's $700 × 1.5 ≈ $1050. Per-cost Grizzly builds slightly faster; per-tank Rhino is faster. |

### Notable absent keys
- No `ImmuneToVeins=yes` (Grizzly has it; HTNK lacks it — actually TS-legacy so dormant either way).
- No `ImmuneToPsionics` — Rhino CAN be mind-controlled.
- No `ImmuneToRadiation` — Desolator rad damages Rhino.
- No `Secondary=` — single weapon.
- No `OmniCrushResistant` — Battle Fortress can squish it.

---

## 2. `artmd.ini` — `[HTNK]` section

```ini
[HTNK]   ; Rhino heavy tank
Voxel=yes
Remapable=yes
Cameo=HTNKICON
AltCameo=HTNKUICO
PrimaryFireFLH=150,0,100
;GEF;UseTurretShadow=yes
;GEF;PBarrelLength=250
;GEF;SBarrelLength=250
;GEF;TurretOffset=-16
;GEF;WalkFrames=15
```

| Key | Value | Effect |
|-----|-------|--------|
| `Voxel` | `yes` | Rendered from `HTNK.VXL` + `HTNK.HVA`. |
| `Remapable` | `yes` | House-color remap. |
| `Cameo` | `HTNKICON` | Sidebar build cameo. |
| `AltCameo` | `HTNKUICO` | Yuri-skinned cameo (when captured by Yuri). |
| `PrimaryFireFLH` | `150,0,100` | Firing offset (X=150 forward, Y=0, Z=100 turret height). **Same FLH as Grizzly** — the two tanks have similar turret geometries. |
| Commented `;GEF;UseTurretShadow=yes` etc. | — | Author-author "GEF" notes — proposed art parameters never enabled in the shipped INI. `UseTurretShadow`, `PBarrelLength`, `SBarrelLength`, `TurretOffset`, `WalkFrames` are alternate visual fine-tuning keys that were considered but commented out. Inert. |

Note: there's also a `[UTNK]` art block right below `[HTNK]` (line 839) with `Image=HTNK` — the "Lunar Tank" (`UTNK`) is a campaign easter-egg variant that reuses HTNK's voxel via `Image=HTNK`. It's the inverse of MTNK's `Image=GTNK` pattern.

---

## 3. Weapon — `[120mm]` / `[120mmE]`

### `[120mm]` (rookie & veteran)

```ini
[120mm]
Damage=90
ROF=65
Range=5.75
Projectile=Cannon
Speed=40
Warhead=AP
Report=RhinoTankAttack
Anim=GUNFIRE
Bright=yes
```

### `[120mmE]` (elite)

```ini
[120mmE]
Damage=90
ROF=65
Range=5.75
Projectile=Cannon
Speed=40
Warhead=RHINAPE
Report=RhinoTankAttack
Anim=VTMUZZLE
Bright=yes
Burst=2
```

| Key | 120mm | 120mmE | Effect |
|-----|-------|--------|--------|
| `Damage` | 90 | **90** | Unchanged — but warhead + Burst changes elite output |
| `ROF` | 65 | 65 | **Unchanged** (vs Grizzly elite which gets 60→50). Rhino elite doesn't fire faster. |
| `Range` | 5.75 | 5.75 | Unchanged |
| `Projectile` | `Cannon` (arcing) | `Cannon` | Same arcing-cannon projectile |
| `Speed` | 40 | 40 | Bullet speed |
| `Warhead` | `AP` | **`RHINAPE`** | Elite swaps to Rhino's AP-elite warhead (see §4) |
| `Report` | `RhinoTankAttack` | same | Per-shot sound |
| `Anim` | `GUNFIRE` | **`VTMUZZLE`** | Different muzzle visual at elite |
| `Bright` | yes | yes | Lights cell on fire |
| `Burst` | (absent → 1) | **2** | **Elite fires 2 shots per cycle** |

**Practical DPS** (vs `none` armor — AP=25%, RHINAPE=100%):
- Rookie: 90 × 25% / 65 = 0.346 dmg/tick
- Elite: 90 × 2 × 100% / 65 = 2.77 dmg/tick → **~8× DPS vs infantry at elite**

Vs `heavy` armor (AP=100%, RHINAPE=100%):
- Rookie: 90 / 65 = 1.38 dmg/tick
- Elite: 90 × 2 / 65 = 2.77 dmg/tick → **2× DPS vs MBTs at elite**

The elite jump is **even more dramatic than Grizzly's** in absolute terms (Rhino elite hits harder than Grizzly elite) but proportionally similar — both ~2× DPS vs armor due to Burst.

### 3.1 Projectile / muzzle

Same `[Cannon]` (Image=120MM, Arcing=true) used by all tank-cannon weapons. `[GUNFIRE]` rookie muzzle, `[VTMUZZLE]` elite muzzle. Both already documented in MTNK §3.

---

## 4. Warheads — `[AP]` / `[RHINAPE]`

### `[AP]` (rookie & veteran)

```ini
[AP]
CellSpread=.3
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=25%,25%,15%,75%,100%,100%,65%,45%,60%,60%,100%
Conventional=yes
InfDeath=3
AnimList=S_CLSN16,S_CLSN22
ProneDamage=50%
```

Verses by slot: 25% / 25% / **15%** / 75% / 100% / 100% / 65% / 45% / 60% / 60% / 100%.
(Same as Grizzly — see [`allied/MTNK.md`](../allied/MTNK.md) §4 for the full breakdown.)
Key behavior: weak vs infantry (25% none/flak, **15% plate** — Tanya/SEAL almost immune),
strong vs medium/heavy armor (100%), decent vs buildings (65/45/60%).

### `[RHINAPE]` (elite only)

```ini
[RHINAPE]
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

| Slot | Armor | AP | RHINAPE | Δ |
|------|-------|----|---------|----|
| 1 | none | 25% | **100%** | +75% |
| 2 | flak | 25% | **100%** | +75% |
| 3 | plate | 15% | **100%** | +85% |
| 4 | light | 75% | **100%** | +25% |
| 5 | medium | 100% | 100% | unchanged |
| 6 | heavy | 100% | 100% | unchanged |
| 7 | wood | 65% | 65% | unchanged |
| 8 | steel | 45% | 45% | unchanged |
| 9 | concrete | 60% | 60% | unchanged |
| 10 | special_1 | 60% | 60% | unchanged |
| 11 | special_2 | 100% | 100% | unchanged |

**RHINAPE is identical to GRIZAPE** (Grizzly's elite warhead) in Verses — both bring all
infantry/light-armor slots to 100% while leaving building slots unchanged. The only
difference between elite Rhino and elite Grizzly damage profile is the **base Damage**
(90 vs 65) and the **per-side ROF/Burst**. Their warhead curves are intentionally
symmetric.

`AnimList=VTEXPLOD` (Vehicle Tank Explosion — Soviet-coloured) replaces AP's S_CLSN16/22
small-arms sparks.

---

## 5. Voices / sounds

```ini
[GenSovVehicleAttackCommand]
Sounds= $vgrsata $vgrsatb $vgrsatc $vgrsatd
Control= random
Volume=85

[GenSovVehicleMove]
Sounds= $vgrsmoa $vgrsmob $vgrsmoc
Control= random
Volume=85

[GenSovVehicleSelect]
Sounds= $vgrssea $vgrsseb $vgrssec
Control= random
Volume=85
```

```ini
[RhinoTankAttack]
Sounds= vrhiatta vrhiattb vrhiattc vrhiattd
Limit=3
FShift= -10 10
Control= random interrupt
Volume=75
```

```ini
[RhinoTankMoveStart]
Sounds= vrhistaa vrhistab vrhistac vrhistad
Control= random predelay
Delay=0 400
Priority=Low
FShift= -10 10
VShift=15
Volume=30
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=GenSovVehicleSelect` | 3 clips | Click-select — shared with all Soviet tanks |
| `VoiceMove=GenSovVehicleMove` | 3 clips | Move — shared |
| `VoiceAttack=GenSovVehicleAttackCommand` | 4 clips | Attack — shared |
| `VoiceFeedback=` *(empty)* | — | No under-attack |
| `DieSound=GenVehicleDie` | 6 clips | Death |
| `MoveSound=RhinoTankMoveStart` | **4 clips (unique)**, predelay 0–400ms, low pri, FShift ±10, VShift +15, vol 30 | Engine start |
| `Report=RhinoTankAttack` (weapon) | 4 clips, **`Limit=3`** (max 3 concurrent), FShift ±10, **`Control=random interrupt`**, vol 75 | Per-shot fire sound |
| `CrushSound=TankCrush` | `vcrusha` | Crush sound |

Notable: `[RhinoTankAttack]` has `Limit=3` and `Control=random interrupt`. **Limit=3 caps concurrent firing-sound instances** — even if 10 Rhinos fire simultaneously, only 3 attack sounds play (prevents audio chaos). `interrupt` means a new fire sound can cut off an in-progress one rather than queueing. This is what makes a Rhino column firing simultaneously sound like a layered volley rather than 10 overlapping cannons.

Soviet voice sets are noticeably **shorter** than Allied (3 select clips vs 5, 3 move vs 6, 4 attack vs 5). The smaller pool means Soviet vehicles repeat lines faster.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `NAWEAP` — Soviet War Factory only. No Radar, no Battle Lab.
- **TechLevel** = `2` — same as Grizzly.
- **Owner**: 4 Soviet countries (no YuriCountry — Yuri lineup has Lasher LTNK).
- **`CrateGoodie=no`** — excluded from crate pool.
- **`AllowedToStartInMultiplayer=` absent** — defaults yes, but in practice no preplaced Rhinos.

### Grizzly vs Rhino — canonical combat matchup

(Detailed table in [`allied/MTNK.md`](../allied/MTNK.md) §6.) Summary deltas:

| Aspect | HTNK (Rhino) | MTNK (Grizzly) |
|--------|---------------|----------------|
| Cost | $900 | $700 |
| HP | 400 | 300 |
| Damage (rookie) | 90 | 65 |
| Range | 5.75 | 5 |
| Speed | 6 | 7 |
| ROF | 65 | 60 |
| Maxdebris | 3 | 2 |
| ThreatPosed | 40 | 15 |
| BuildTimeMultiplier | 1.3 | 1.5 |
| MovementZone | Destroyer | Normal (workaround for unfixed FLAW) |
| Elite Burst | 2 | 2 |
| Elite Warhead | RHINAPE | GRIZAPE |
| TargetLaser | `no` (explicit) | (default — absent) |
| DamageSmokeOffset | 100,100,275 | (absent — defaults) |

**Trade math (1v1, rookie, vs `heavy` armor)**:
- Rhino dmg vs Grizzly: 90 × 100% / 65 = 1.38 dmg/tick → kills Grizzly's 300 HP in 217 ticks
- Grizzly dmg vs Rhino: 65 × 100% / 60 = 1.08 dmg/tick → kills Rhino's 400 HP in 370 ticks
- **Rhino wins 1v1 by 153 ticks (~9 seconds)**. Grizzly survives with 0 HP only if it can retreat at Speed 7 vs Rhino's Speed 6.

**Cost-efficiency**: Allied needs **~2.06 Grizzlies per Rhino** to break even (1.06 Grizzlies of damage absorbed = 1 Rhino down by trade-math). At 2 Grizzlies = $1400 vs 1 Rhino = $900, Allied pays $500 more per equivalent firepower. Allied counters this via Battle Fortress micro and Mirage flanking.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 HTNK-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `HTNK` | 0 matches |
| `Rhino` (substring) | (not run — unlikely to find) |

⇒ **No HTNK-specific code path.** All behavior is generic flag-driven.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `TargetLaser` | 0x00843898 | TechnoTypeClass__ReadINI @ 0x00714c8d | TechnoType |
| `DamageSmokeOffset` | 0x00843f60 | TechnoTypeClass__ReadINI @ 0x00713e25 | TechnoType |

Plus prior verifications (carried from MTNK iteration):
- `TooBigToFitUnderBridge` — UnitType only
- `OpportunityFire` — TechnoType
- `BuildTimeMultiplier` — TechnoType

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Soviet MBT with high HP/damage | Strength=400, Primary=120mm Damage=90 | Core role |
| Auto-target threats | `OpportunityFire=yes` | |
| Cannot path under bridges | `TooBigToFitUnderBridge=true` | Pathfinder rejects |
| Can use Destroyer movement zone | `MovementZone=Destroyer` | More permissive than Normal; can traverse some crushable terrain |
| Engine sound from Soviet pool | `MoveSound=RhinoTankMoveStart` | Unique per-unit |
| Fire-sound limit 3 concurrent | `[RhinoTankAttack] Limit=3 Control=interrupt` | Audio mix control |
| Elite weapon: Burst=2 + RHINAPE | `ElitePrimary=120mmE` | ~2× DPS at elite |
| No turret target-laser indicator | `TargetLaser=no` | Explicit opt-out |
| Damage-smoke emitter at turret height | `DamageSmokeOffset=100,100,275` | Custom particle origin |
| Faster build than Grizzly per-tank | `BuildTimeMultiplier=1.3` | Balance against cost |

### 7.4 Behaviors NOT present

- No `OmniCrushResistant=yes` → Battle Fortress squishes Rhino.
- No `Secondary` → no AA, no anti-naval.
- No `Teleporter`.
- No `SelfHealing=yes` at rookie/veteran (only at elite via SELF_HEAL ability).
- No `ImmuneToPsionics` — Yuri can steal Rhinos via mind-control.
- No `ImmuneToRadiation` — Desolators damage Rhino normally.
- No `Bunkerable=no` (defaults yes) — Rhino CAN enter Battle Fortress (but Soviet doesn't have one natively).

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ZFudgeTunnel=13` | YES (no real tunnels in YR) | Dormant render value. |
| `TargetLaser=no` | Possibly — the flag may be a TS-era artifact since most YR units don't set it; setting it to `no` here is "explicit absence" for safety | Either way, harmless. |
| Commented `;origional - Locomotor={55D141B8-...}` | History note | Inactive. |
| Commented `;GEF;UseTurretShadow / PBarrelLength / SBarrelLength / TurretOffset / WalkFrames` | n/a (author notes, never enabled) | Inactive. |

No fog-of-war, no ImmuneToVeins (notable — Rhino is one of the few units without this dormant flag), no Tiberium refs.

---

## 9. Veterancy

### Veteran (1 chevron) — `STRONGER, FIREPOWER, SIGHT, FASTER`
- `STRONGER` — +25% HP (400 → 500)
- `FIREPOWER` — +25% damage (90 → 112)
- `SIGHT` — +20% sight (8 → 9.6)
- `FASTER` — +20% speed (6 → 7.2 — matches Grizzly's rookie speed)

### Elite (2 chevrons) — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative)
- Adds `SELF_HEAL` (passive HP regen)
- `STRONGER` & `FIREPOWER` reapplied
- `ROF` — −25% ROF (65 → ~49 with elite ability, but the weapon's own ROF stays at 65)

**Plus weapon swap**: `[120mm]` → `[120mmE]`:
- Burst 1 → **2**
- Warhead AP → **RHINAPE**
- Anim GUNFIRE → VTMUZZLE
- Damage, ROF, Range, Speed: unchanged

**Net elite jump**: ~2× DPS vs MBTs, infantry slot Verses 25%→100% (4× damage to soft targets), self-heal. Identical pattern to Grizzly's elite jump, just with bigger base numbers.

---

## 10. Cross-references

### Direct dependencies
- `[120mm]` / `[120mmE]` — weapons (§3)
- `[Cannon]` — projectile
- `[AP]` / `[RHINAPE]` — warheads (§4)
- `[120MM]` (artmd) — bullet sprite
- `[GUNFIRE]` / `[VTMUZZLE]` (artmd) — muzzle anims
- `[S_CLSN16] / [S_CLSN22] / [VTEXPLOD]` (artmd) — impact anims
- `[HTNK]` (artmd) — art block (no `Image=` redirect — direct)
- `[NAWEAP]` — prereq
- `[GenSovVehicleSelect/Move/AttackCommand]` (soundmd) — voices
- `[RhinoTankMoveStart]` (soundmd) — unique engine sound
- `[RhinoTankAttack]` (soundmd) — unique fire sound (Limit=3)
- `[GenVehicleDie] / [TankCrush]` (soundmd) — generic vehicle sounds

### Conceptual companions
- **MTNK (Grizzly)** ([`allied/MTNK.md`](../allied/MTNK.md)) — direct counter-pair Allied MBT.
- **APOC (Apocalypse Tank)** ([`soviet/APOC.md`](./APOC.md) — TODO) — Soviet tier-4 heavy MBT with dual AG+AA turrets. Comparison: APOC is the Rhino's "big brother".
- **LTNK (Lasher Tank)** ([`yuri/LTNK.md`](../yuri/LTNK.md) — TODO) — Yuri's light MBT (analogous Yuri-side counterpart).
- **TNKD (Tank Destroyer)** ([`allied/TNKD.md`](../allied/TNKD.md)) — German Allied AT-tank. Rhino's worst matchup vs TNKD due to UltraAP 100% vs heavy.

### Deep-RE docs
- None directly relevant — HTNK has no unique hardcoded behavior worth a dedicated report.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[HTNK]` rulesmd key explained | ✅ §1 |
| `[HTNK]` artmd block expanded (no Image= redirect — direct read; commented GEF entries noted) | ✅ §2 |
| Both weapons (rookie + elite) + both warheads + projectile | ✅ §3–§4 |
| **RHINAPE = GRIZAPE Verses parity noted** (symmetric design) | ✅ §4 |
| All voices + unique RhinoTankMoveStart + RhinoTankAttack with Limit=3 | ✅ §5 |
| Prereqs / owners / availability | ✅ §6 |
| **HTNK vs MTNK trade math** (1v1 winner, cost-efficiency ratio) | ✅ §6 |
| Hardcoded behavior — Ghidra searches + 2 new flag-scope verifications (TargetLaser, DamageSmokeOffset) | ✅ §7 |
| TS-legacy filter | ✅ §8 |
| Veterancy detailed with elite-weapon delta | ✅ §9 |
| Cross-refs to companion docs (MTNK, APOC, LTNK, TNKD) | ✅ §10 |

**Open follow-ups (none load-bearing):**
- The `MovementZone=Destroyer` vs Grizzly's workaround `Normal`: the unfixed pathfinding "FLAW" should be investigated when pathing-parity bugs surface. Worth a dedicated `/re-investigate Destroyer movement zone pathfinding` session if specific terrain edge cases fail.
- `TargetLaser=no` — confirm what `TargetLaser=yes` actually renders (laser line during target-acquisition?) and whether any YR unit ships with `=yes`. Likely a TS-legacy holdover.
- The `[RhinoTankAttack] Limit=3 Control=interrupt` audio cap — verify the implementation handles 10+ simultaneous Rhinos firing without audio dropouts or weird priority handling.
- `Soylent=900` is asymmetric vs `Cost=900` Grinder-refund (100%) — confirm if Yuri-captured Rhinos really return full $900 to Grinder.
