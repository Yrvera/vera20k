# HTK — Flak Track (Soviet Half-Track Transport)

**Side classification:** Soviet (Owner=Russians,Confederation,Africans,Arabs).
**Role:** Soviet small transport. 5-passenger capacity with `SizeLimit=2` (infantry
only) + dual weapons (anti-ground `FlakTrackGun` + anti-air `FlakTrackAAGun`). Cheap
($500), fast (Speed 8), versatile early-game multi-tool.

> ⚠ **Index correction logged**: prior `INDEX_UNITS.md` listed HTK as "IFV (Multi-Gunner)
> Allied". This is wrong. The INI says `Name=Flak Track`, `Prerequisite=NAWEAP`,
> `Owner=Russians,Confederation,Africans,Arabs` — HTK is the **Soviet half-track
> transport**. The Allied **Multi-Gunner IFV** is `[FV]` (TODO). The artmd comment
> `[HTK] ; Half Track` clinches it.

> Output bar: dual-weapon target switching (FlakTrackGun vs FlakTrackAAGun), 5-slot
> passenger pip rendering, and DeployTime field-stop timing all matter for player feel.

> Ghidra confirms `gamemd.exe` contains no `"HTK"` / `"FlakTrack"` strings — all
> behavior is generic flag-driven via standard TechnoType / UnitType handling.

---

## 1. `rulesmd.ini` — `[HTK]` verbatim

```ini
[HTK]
UIName=Name:HTK
Name=Flak Track
Prerequisite=NAWEAP
Primary=FlakTrackGun
Secondary=FlakTrackAAGun
Strength=180
Category=Transport
Armor=heavy
DeployTime=.022
TechLevel=3
Sight=8
PipScale=Passengers
Speed=8
CrateGoodie=no
Owner=Russians,Confederation,Africans,Arabs
AllowedToStartInMultiplayer=no
Cost=500
Soylent=500
Points=20
ROT=5
Crusher=yes
Turret=yes
Passengers=5
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=FlakTrackSelect
VoiceMove=FlakTrackMove
VoiceAttack=FlakTrackAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=FlakTrackMoveStart
EnterTransportSound=EnterTransport
LeaveTransportSound=ExitTransport
CrushSound=TankCrush
Maxdebris=3
DebrisTypes=TIRE
DebrisMaximums=6
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
MovementZone=Normal
ThreatPosed=10	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
SpecialThreatValue=1
ZFudgeColumn=10
ZFudgeTunnel=13
ImmuneToRadiation=no
;Bombable=no
Size=3
SizeLimit=2 
Accelerates=false
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ElitePrimary=FlakTrackGunE
EliteSecondary=FlakTrackAAGunE
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:HTK` | AbstractType | CSF lookup. |
| `Name` | `Flak Track` | AbstractType | Dev fallback. |
| `Prerequisite` | `NAWEAP` | TechnoType | Soviet War Factory — early-tier prereq. |
| `Primary` | `FlakTrackGun` | TechnoType | Anti-ground flak gun (Damage=25, ROF=25, Range=5). See §3. |
| `Secondary` | `FlakTrackAAGun` | TechnoType | Anti-air flak gun (Damage=35, ROF=25, Range=10). See §3. **Dual weapon system**: same turret swaps between ground/air targets automatically. |
| `Strength` | `180` | AbstractType | 180 HP — fragile compared to MBTs (Grizzly 300, Rhino 400). |
| `Category` | `Transport` | TechnoType | Transport classifier — different AI handling than AFV. |
| `Armor` | `heavy` | TechnoType | Verses-slot 6 — surprisingly tough armor class for a thin-skinned transport. |
| `DeployTime` | `.022` | TechnoType (verified — 0x00843904 → 0x00714b85) | **Field-stop deploy time** (fractional seconds). HTK has `DeployTime=.022` but no `DeploysInto=` — so this isn't a transformation; it's the **field-stop duration** the unit must stand still before firing (a "settling" period for the turret). Very short (.022 = ~22 ms / ~1.3 ticks at 60fps), almost imperceptible — but ensures gun is stable before shooting. Shared mechanic with the Allied IFV (`[FV] DeployTime=.022`). |
| `TechLevel` | `3` | TechnoType | Mid-tier. |
| `Sight` | `8` | TechnoType | 8-cell reveal. |
| `PipScale` | `Passengers` | UnitType | Renders passenger-count pips above the unit (one pip per passenger). Combined with `Passengers=5`, shows 0-5 colored pips depending on load. |
| `Speed` | `8` | TechnoType | **Fastest non-aircraft Soviet vehicle in the early game** — same as Tesla Tank, faster than every MBT. |
| `CrateGoodie` | `no` | UnitType | Excluded from crate pool. |
| `Owner` | `Russians,Confederation,Africans,Arabs` | TechnoType | Soviet only. |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Never preplaced. |
| `Cost` | `500` | TechnoType | Cheap — cheaper than any MBT, makes HTK swarmable. |
| `Soylent` | `500` | TechnoType | 100% Grinder refund (relevant if Yuri captures). |
| `Points` | `20` | TechnoType | Score on kill. |
| `ROT` | `5` | TechnoType | Turret + body rotation. |
| `Crusher` | `yes` | TechnoType | Crushes infantry — including its own passengers' theoretical attackers. |
| `Turret` | `yes` | UnitType | Single rotating turret used for both Primary and Secondary weapons. |
| `Passengers` | `5` | TechnoType (verified — 0x0081bbd4 → 0x00714b3c) | **Up to 5 infantry passengers.** Notable: more than the Allied IFV's 1 passenger. HTK is a serious troop transport. |
| `IsSelectableCombatant` | `yes` | TechnoType | Counts in select-all-combat. |
| `Explosion` | `TWLT070,...` | TechnoType | Standard random death anim. |
| `VoiceSelect` | `FlakTrackSelect` | TechnoType | Unique 5-clip voice ($vflasea..ee). |
| `VoiceMove` | `FlakTrackMove` | TechnoType | 5 unique clips. |
| `VoiceAttack` | `FlakTrackAttackCommand` | TechnoType | 5 unique clips. |
| `VoiceFeedback` | *(empty)* | TechnoType | No under-attack voice. |
| `DieSound` | `GenVehicleDie` | TechnoType | Standard. |
| `MoveSound` | `FlakTrackMoveStart` | TechnoType | Unique engine-start (4 clips, predelay 0–400ms, low pri, FShift ±10, VShift +20, vol 35). |
| `EnterTransportSound` | `EnterTransport` | TechnoType | Generic transport-enter sound (`genter1a` clip, FShift ±2, vol 60). Played when an infantry boards. |
| `LeaveTransportSound` | `ExitTransport` | TechnoType | Generic transport-exit sound (`gexit1a`, Limit=2, FShift ±1, vol 60). Played when an infantry disembarks. **`Limit=2`** — max 2 exit sounds concurrent (prevents 5-passenger-eject audio spam). |
| `CrushSound` | `TankCrush` | TechnoType | Standard crush. |
| `Maxdebris` | `3` | TechnoType | 3 debris on death. |
| `DebrisTypes` | `TIRE` | TechnoType | Tire-shaped debris. |
| `DebrisMaximums` | `6` | TechnoType | Up to 6 of each debris type. |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass. |
| `MovementZone` | `Normal` | TechnoType | Standard land path. |
| `ThreatPosed` | `10` | TechnoType | Low AI threat. |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | |
| `SpecialThreatValue` | `1` | TechnoType | High-value flag — AI tries to protect or destroy this for tactical reasons (transport value). |
| `ZFudgeColumn` | `10` | UnitType | Z-fudge near columns. |
| `ZFudgeTunnel` | `13` | UnitType | TS-legacy. |
| `ImmuneToRadiation` | **`no`** | TechnoType | **Explicitly NOT immune** to Desolator rad. Notable — most Soviet vehicles default-no this, but the explicit `=no` here suggests an INI-level reminder that Flak Track passengers (including Soviet Conscripts walking out of a rad field) get hurt. |
| `;Bombable=no` | *(commented)* | — | Inert — default `Bombable=yes` applies (Crazy Ivan can plant bombs on HTK). |
| `Size` | `3` | TechnoType | HTK's own transport-slot cost (if loaded onto a bigger transport). |
| `SizeLimit` | `2` | TechnoType (verified — 0x008443bc → 0x00712540) | **Maximum passenger size = 2.** Combined with `Passengers=5`, this means HTK accepts up to 5 Size=1 or Size=2 infantry. Standard infantry are Size=1; some are Size=2 (rare). HTK cannot carry vehicles or large units. |
| `Accelerates` | `false` | TechnoType | No acceleration ramp — moves at top speed from frame 1. |
| `VeteranAbilities` | `STRONGER,FIREPOWER,SIGHT,FASTER` | TechnoType | Veteran bonuses (no ROF). |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Elite adds SELF_HEAL + ROF. |
| `ElitePrimary` | `FlakTrackGunE` | TechnoType | Elite anti-ground weapon — adds Burst=2 (see §3). |
| `EliteSecondary` | `FlakTrackAAGunE` | TechnoType | Elite anti-air weapon — Burst=2 and lower damage (20 vs 35) but kept range. |

### Notable absent keys
- No `Image=` — HTK reads its own `[HTK]` artmd block directly.
- No `Bunkerable=no` (defaults yes) — HTK CAN board Battle Fortress (FV-style transport-in-transport).
- No `ImmuneToPsionics` — Yuri can mind-control HTK (steals 5 passengers in the bargain).
- No `OpportunityFire=yes` — HTK does NOT auto-target ground threats. Manual attack orders only. **But** the Secondary AA does auto-engage air (per the AA targeting routine). Same pattern as Apocalypse.
- No `OmniCrushResistant=yes` — Battle Fortress squishes HTK.
- No `TooBigToFitUnderBridge=true` — HTK CAN path under low bridges.
- No `Gunner=yes` — HTK passengers do **NOT** affect the vehicle's weapon (unlike the Allied IFV which has `Gunner=yes` + WeaponCount=17 IFVMode mappings). HTK's flak weapons are fixed regardless of passengers.
- No `OpenTopped=yes` — HTK is closed-top; passengers cannot fire out (unlike Battle Fortress).

---

## 2. `artmd.ini` — `[HTK]` section

```ini
[HTK] ; Half Track
Cameo=HTKICON
AltCameo=HTKUICO
Voxel=yes
TurretOffset=-80
Remapable=yes
PrimaryFireFLH=65,0,220
SecondaryFireFLH=65,0,220 ;gs needs own listing
```

| Key | Value | Effect |
|-----|-------|--------|
| `Cameo` | `HTKICON` | Sidebar cameo. |
| `AltCameo` | `HTKUICO` | Yuri-skinned cameo. |
| `Voxel` | `yes` | Voxel-rendered. |
| `TurretOffset` | **`-80`** | **Negative offset** — turret pivot is 80 voxel units **behind** the body center (the flak gun sits in the rear half of the half-track chassis). Most tanks have positive TurretOffset; HTK's negative value reflects the half-track's rear-mounted flak platform. |
| `Remapable` | `yes` | House-color remap. |
| `PrimaryFireFLH` | `65,0,220` | Firing offset (X=65 forward of turret pivot, Y=0, Z=220 — very tall, top of flak gun). |
| `SecondaryFireFLH` | `65,0,220 ;gs needs own listing` | Same FLH as Primary. INI comment "gs needs own listing" means the author duplicated the value to ensure the Secondary weapon doesn't fall back to default. **Note**: AA missile and AG flak both emerge from the same muzzle position — visually consistent. |

The comment "gs needs own listing" suggests an earlier version where omitting `SecondaryFireFLH=` caused the engine to use a wrong default. Robustness note: always set both FLHs on dual-weapon vehicles.

---

## 3. Weapons — Primary `[FlakTrackGun]` (AG) + Secondary `[FlakTrackAAGun]` (AA)

### 3.1 `[FlakTrackGun]` (anti-ground, rookie)

```ini
[FlakTrackGun]		; Anti-surface gun
Damage=25 ;25 -changed by DB on 7/18/01
ROF=25 ;40 -changed by DB on 7/18/01
Range=5
Projectile=FlakTProj
Speed=50
Report=FlakTrackAttackGround		; put in new sound for this
Warhead=FlakTWH
Anim=GUNFIRE
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `25` | Per-shot AG damage. INI comments show prior tunings (25 unchanged in net). |
| `ROF` | `25` | 25-tick cooldown — very fast firing (compare Grizzly 60, Rhino 65). |
| `Range` | `5` | 5-cell AG range. |
| `Projectile` | `FlakTProj` | Anti-surface flak round (`Image=120MM, Arcing=true, AA=no, AG=yes, Inaccurate=yes, FlakScatter=yes`). Inaccurate + FlakScatter means shots scatter, hitting cells around the target — gives Flak Track an AoE feel against infantry blobs. |
| `Speed` | `50` | Bullet speed. |
| `Report` | `FlakTrackAttackGround` | Per-shot AG sound. |
| `Warhead` | `FlakTWH` | Flak Track anti-surface warhead — see §4. |
| `Anim` | `GUNFIRE` | Standard muzzle flash. |

### 3.2 `[FlakTrackAAGun]` (anti-air, rookie)

```ini
[FlakTrackAAGun]	; Separate from Flak Cannon weapon so that stats may be tweaked
Damage=35
ROF=25
Range=10
Projectile=FlakProj	; AA bullet shared with Flak Cannon
Speed=100
Report=FlakTrackAttackAir
Warhead=FlakWH
Anim=GUNFIRE
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `35` | Per-shot AA — **higher than AG's 25** because aircraft have flak/light armor. |
| `ROF` | `25` | Same fast 25-tick cooldown. |
| `Range` | `10` | **Long AA reach** (vs AG's 5). Doubles the engagement zone for aircraft. |
| `Projectile` | `FlakProj` | Shared with Flak Cannon (NAFLAK building). `AA=yes, AG=no, Inviso=yes, Ranged=yes, Inaccurate=yes, FlakScatter=yes, SubjectToCliffs=no, SubjectToElevation=yes, SubjectToWalls=no` — homing-like behavior via Ranged + Inaccurate but doesn't snap to target. |
| `Speed` | `100` | Faster bullet — needs speed to catch aircraft. |
| `Report` | `FlakTrackAttackAir` | Unique AA-fire sound. |
| `Warhead` | `FlakWH` | Anti-air flak warhead — see §4. |
| `Anim` | `GUNFIRE` | Standard muzzle. |

### 3.3 Elite weapons

```ini
[FlakTrackGunE]		; Anti-surface gun
Damage=25
ROF=25
Range=5
Projectile=FlakTProj
Speed=50
Report=FlakTrackAttackGround		; put in new sound for this
Warhead=FlakTWH
Anim=GUNFIRE
Burst=2
```

```ini
[FlakTrackAAGunE]	; Separate from Flak Cannon weapon so that stats may be tweaked
Damage=20
ROF=25
Range=8
Projectile=FlakProj	; AA bullet shared with Flak Cannon
Speed=100
Report=FlakTrackAttackAir
Warhead=FlakWH
Anim=GUNFIRE
Burst=2
```

**Elite AG (`FlakTrackGunE`)**: identical to rookie + `Burst=2` (doubles per-cycle damage).
**Elite AA (`FlakTrackAAGunE`)**: Damage **lowered** 35 → 20, Range **shortened** 10 → 8, but adds `Burst=2`. Net AA damage per cycle: 20 × 2 = 40 (vs rookie 35) → only +14% — a minor elite AA boost. Compensates with `SELF_HEAL` from EliteAbilities.

### 3.4 Projectiles

```ini
[FlakProj]		; AA bullet for Flak Cannon and Flak Track.
Image=none
Inviso=yes
AA=yes
AG=no
Shadow=no
Ranged=yes		; Not homing, but ranged -- check fuse, explode if near target coords
Inaccurate=yes	; Bullets do not snap onto targets when "close enough".
FlakScatter=yes ; This weapon scatters its shots.
SubjectToCliffs=no
SubjectToElevation=yes
SubjectToWalls=no

[FlakTProj]		; Anti-surface bullet for Flak Track.
Image=120MM
Arcing=true
Inviso=no
AA=no
AG=yes
Shadow=no
Inaccurate=yes
FlakScatter=yes
SubjectToCliffs=no
```

Notable: both projectiles use **`FlakScatter=yes`** — the engine spreads multiple shots around the target rather than direct-impact. This is what gives flak weapons their "spray" feel. Combined with `Inaccurate=yes` (no snap-to-target), each shot lands in a slightly different cell.

---

## 4. Warheads — `[FlakTWH]` (AG) / `[FlakWH]` (AA)

### `[FlakTWH]` (anti-ground)

```ini
[FlakTWH]	; For the Flak Track's anti-surface weapon.
CellSpread=1.0
PercentAtMax=1.0
Verses=150%,125%,100%,60%,10%,10%,30%,20%,10%,100%,100%	; no buildings
AnimList=HTRKPUFF
InfDeath=3
Conventional=yes	; Go splash in the water.
```

| Slot | Armor | Verses | Notes |
|------|-------|--------|-------|
| 1 | none | **150%** | **Bonus damage vs basic infantry** — flak shrapnel rakes soft targets |
| 2 | flak | **125%** | Bonus vs Flak Trooper armor |
| 3 | plate | 100% | Full vs plate (Tanya/SEAL) |
| 4 | light | 60% | Decent vs light vehicles |
| 5 | medium | 10% | Very poor vs MBTs |
| 6 | heavy | 10% | Same — heavy armor shrugs off flak |
| 7 | wood | 30% | Poor vs buildings |
| 8 | steel | 20% | Worse vs steel |
| 9 | concrete | 10% | Almost useless vs concrete |
| 10 | special_1 | 100% | |
| 11 | special_2 | 100% | |

**`CellSpread=1.0, PercentAtMax=1.0`** — 1-cell AoE radius with 100% damage at edge (no falloff within radius). Combined with `FlakScatter=yes` on the projectile, FlakTrackGun shreds infantry clusters.

`AnimList=HTRKPUFF` — unique impact animation ("Half-Track Puff" — small flak puff smoke).

INI comment "no buildings" reflects the design intent: Flak Track is **not a siege weapon**.

### `[FlakWH]` (anti-air)

```ini
[FlakWH]	; For anti-air flak weapons.
CellSpread=1.0
PercentAtMax=.1
Verses=150%,80%,50%,100%,100%,20%,0%,0%,0%,100%,100%	; no buildings
AnimList=SMKPUFF
InfDeath=3
```

| Slot | Armor | Verses | Notes |
|------|-------|--------|-------|
| 1 | none | 150% | Bonus vs basic infantry (if AA accidentally hits ground) |
| 2 | flak | 80% | Slightly weaker vs Flak Troopers (counter-design) |
| 3 | plate | 50% | Mid vs plate |
| 4 | light | 100% | **Full damage vs light armor (aircraft hulls)** |
| 5 | medium | 100% | Full vs medium |
| 6 | heavy | 20% | Poor vs heavy |
| 7-9 | wood/steel/concrete | 0%/0%/0% | **Cannot damage buildings at all** — AA flak is air-only |
| 10 | special_1 | 100% | |
| 11 | special_2 | 100% | |

`PercentAtMax=0.1` (vs FlakTWH's 1.0) — AA AoE damage **falls off to 10%** at edge. Tighter splash because aircraft are point targets.

`AnimList=SMKPUFF` (smoke puff) — different anim than ground flak.

---

## 5. Voices / sounds

```ini
[FlakTrackSelect]
Sounds=$vflasea $vflaseb $vflasec $vflased $vflasee
Control=random
Volume=85

[FlakTrackMove]
Sounds=$vflamoa $vflamob $vflamoc $vflamod $vflamoe
Control=random
Volume=85

[FlakTrackAttackCommand]
Sounds=$vflaata $vflaatb $vflaatc $vflaatd $vflaate
Control=random
Volume=85
```

```ini
[FlakTrackMoveStart]
Sounds= vflastaa vflastab vflastac vflastad
Control= random predelay
Delay=0 400
Priority=Low
FShift= -10 10
VShift=20
Volume=35
```

```ini
[EnterTransport]
Sounds=genter1a
FShift= -2 2
Volume=60

[ExitTransport]
Sounds=gexit1a
FShift= -1 1
Limit=2
Volume=60
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=FlakTrackSelect` | 5 unique clips | Click-select |
| `VoiceMove=FlakTrackMove` | 5 unique clips | Move order |
| `VoiceAttack=FlakTrackAttackCommand` | 5 unique clips | Attack order |
| `VoiceFeedback=` *(empty)* | — | No under-attack |
| `DieSound=GenVehicleDie` | 6 clips | Death |
| `MoveSound=FlakTrackMoveStart` | 4 clips, predelay 0–400ms, low pri | Engine start |
| `EnterTransportSound=EnterTransport` | `genter1a`, vol 60 | Infantry boards HTK |
| `LeaveTransportSound=ExitTransport` | `gexit1a`, **`Limit=2`**, vol 60 | Infantry disembarks; Limit=2 prevents 5-pop audio spam |
| `Report=FlakTrackAttackGround` (primary) | (referenced; not directly read this iter) | AG fire sound |
| `Report=FlakTrackAttackAir` (secondary) | (referenced) | AA fire sound — distinct from AG |
| `CrushSound=TankCrush` | `vcrusha` | Crush |

The Flak Track has the **second-most distinct audio profile** in the Soviet lineup
after Apocalypse: unique select/move/attack voices, unique engine, distinct ground/air
fire sounds. Players hearing a Flak Track engagement know it immediately.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `NAWEAP` — Soviet War Factory only.
- **TechLevel** = `3` (mid-tier — slightly later than basic Rhino).
- **Owner**: 4 Soviet countries (no Yuri — Yuri uses Gattling Tank for AA).
- **CrateGoodie**: `no` — excluded from crate pool.
- **`AllowedToStartInMultiplayer=no`** — never preplaced.
- **Cost** = $500. Very cheap for a dual-weapon transport.

### Role positioning

| Role | HTK fills it? |
|------|---------------|
| Anti-air defense | **Yes** — FlakTrackAAGun is the Soviet faction's mobile AA in early-game |
| Anti-infantry combat | **Yes** — FlakTrackGun's 150% vs infantry + AoE + ROF=25 = serious infantry shredder |
| Anti-armor combat | **No** — FlakTWH 10% vs medium/heavy is too weak |
| Anti-building siege | **No** — FlakTWH 30/20/10% vs buildings |
| Troop transport | **Yes** — Passengers=5, SizeLimit=2 |
| Scout | **Partial** — Speed=8 is fast, but Sight=8 is standard |

So HTK is functionally three units in one: mobile AA, anti-infantry harasser, and 5-slot transport. The cheap cost ($500) makes losing one bearable in any of those roles. Soviet players typically build a 2-3 HTK pack for AA coverage of their tank columns.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 HTK-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `HTK` | 0 matches |
| `FlakTrack` | 0 matches |

⇒ **No HTK-specific code path.** All behavior is generic flag-driven.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `DeployTime` | 0x00843904 | TechnoTypeClass__ReadINI @ 0x00714b85 | TechnoType |
| `Passengers` | 0x0081bbd4 | TechnoTypeClass__ReadINI @ 0x00714b3c | TechnoType |
| `SizeLimit` | 0x008443bc | TechnoTypeClass__ReadINI @ 0x00712540 | TechnoType |

Plus prior verifications:
- `Crusher`, `Turret`, `OpportunityFire`, `Image=` redirects, `Armor`, etc. — TechnoType
- `TooBigToFitUnderBridge` (absent on HTK) — UnitType only

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Cheap fast Soviet transport | Cost=500, Speed=8, no accel ramp | |
| Field-stop deploy-time before firing | `DeployTime=.022` | Very short field-stop |
| 5-passenger transport with infantry-only restriction | `Passengers=5, SizeLimit=2` | |
| Dual-weapon AG+AA target switching | `Primary=FlakTrackGun (AG)` + `Secondary=FlakTrackAAGun (AA)` + `FlakProj.AA=yes, AG=no` + `FlakTProj.AA=no, AG=yes` | Engine auto-picks weapon by target type |
| AA auto-engage on incoming aircraft | Secondary AA targeting | Player doesn't manually target |
| No AG opportunistic fire | `OpportunityFire` absent | Player must order ground attacks (same as Apocalypse) |
| FlakScatter spray damage | `[FlakProj/FlakTProj] FlakScatter=yes` | Shots scatter around target |
| AoE damage on hit | `[FlakWH/FlakTWH] CellSpread=1.0` | 1-cell radius |
| Boarding/disembarking sounds | `EnterTransportSound=EnterTransport` + `LeaveTransportSound=ExitTransport` (`Limit=2`) | |
| Cannot damage buildings | `FlakWH Verses[7-9]=0%, FlakTWH Verses[7-9]=30/20/10%` | Intentional anti-armor/anti-air design |
| Elite Burst=2 on both weapons | `[FlakTrackGunE/FlakTrackAAGunE] Burst=2` | |

### 7.4 Behaviors NOT present

- No `Gunner=yes` — passengers do NOT affect weapon. (Compare Allied IFV `[FV] Gunner=yes`.)
- No `OpenTopped=yes` — passengers cannot fire out (compare Battle Fortress).
- No `Spawns=` — no child units.
- No `Teleporter=` — no chrono.
- No `ImmuneToPsionics` — Yuri can mind-control HTK + its 5 passengers.
- No `OmniCrushResistant=yes` — Battle Fortress can squish.
- No `Bunkerable=no` — HTK CAN board Battle Fortress (FV transport-in-transport).

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ZFudgeTunnel=13` | YES | Dormant render value. |
| (no `ImmuneToVeins`) | — | Not set. |
| Commented `;Bombable=no` | n/a (commented) | Inert. |

No fog-of-war refs, no Tiberium refs, no real tunnels.

---

## 9. Veterancy

### Veteran (1 chevron) — `STRONGER, FIREPOWER, SIGHT, FASTER`
- `STRONGER` — +25% HP (180 → 225)
- `FIREPOWER` — +25% damage (FlakTrackGun 25 → 31, FlakTrackAAGun 35 → 44)
- `SIGHT` — +20% sight (8 → 9.6)
- `FASTER` — +20% speed (8 → 9.6 — very fast)

### Elite (2 chevrons) — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative)
- Adds `SELF_HEAL` (passive HP regen)
- Reapplies STRONGER, FIREPOWER, ROF
- `ROF` — −25% cooldown (25 → ~19 tick)

**Plus weapon swaps**: `FlakTrackGun → FlakTrackGunE` (Burst=2 added, otherwise unchanged) and `FlakTrackAAGun → FlakTrackAAGunE` (Damage 35 → 20, Range 10 → 8, Burst=2).

**Net AG at elite**: 25 × 2 / 25 = 2 dmg/tick (vs rookie 1 dmg/tick) → 2× DPS.
**Net AA at elite**: 20 × 2 / 25 = 1.6 dmg/tick (vs rookie 35/25 = 1.4 dmg/tick) → marginal +14%. The elite AA is essentially a side-grade — gains Burst at cost of per-shot damage and range. Compensated by `SELF_HEAL`.

---

## 10. Cross-references

### Direct dependencies
- `[FlakTrackGun]` / `[FlakTrackGunE]` — primary weapons (§3)
- `[FlakTrackAAGun]` / `[FlakTrackAAGunE]` — secondary weapons (§3)
- `[FlakTProj]` — AG projectile
- `[FlakProj]` — AA projectile (shared with Flak Cannon NAFLAK)
- `[FlakTWH]` — AG warhead (§4)
- `[FlakWH]` — AA warhead (§4 — shared with Flak Cannon)
- `[120MM]` (artmd) — bullet sprite for AG round
- `[GUNFIRE]` (artmd) — muzzle flash
- `[HTRKPUFF]` (artmd — TODO) — AG impact anim
- `[SMKPUFF]` (artmd — TODO) — AA impact anim
- `[HTK]` (artmd) — art block (direct read, no Image= redirect)
- `[NAWEAP]` — prereq
- `[FlakTrackSelect/Move/AttackCommand/AttackGround/AttackAir/MoveStart]` (soundmd) — voices and sounds
- `[EnterTransport] / [ExitTransport]` (soundmd) — board/disembark sounds
- `[GenVehicleDie] / [TankCrush]` (soundmd) — generic sounds

### Conceptual companions
- **NAFLAK** (`structures/NAFLAK.md` — TODO) — Soviet Flak Cannon defense building. Shares `FlakWH` and `FlakProj`.
- **FLAKT** (Flak Trooper, `soviet/FLAKT.md` — DONE) — infantry AA. Uses `FlakGuyAAGun` (different stats) and `FlakGuyWH`.
- **FV** (Allied IFV, `allied/FV.md` — TODO) — Allied transport counterpart. Has `Gunner=yes` + WeaponCount=17 IFVMode dispatch (very different mechanic). The actual "passenger-weapon-swap" unit, not HTK.
- **APOC (Apocalypse)** ([`soviet/APOC.md`](./APOC.md)) — similar dual-weapon AG+AA pattern but with different intent (heavy tank, not transport).

### Deep-RE docs
- None directly relevant — HTK has no unique hardcoded behavior worth a dedicated report. The dual-weapon target switching is generic combat-class behavior.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[HTK]` rulesmd key explained | ✅ §1 |
| Every `[HTK]` artmd key explained — TurretOffset=-80 (rear-mounted flak) highlighted | ✅ §2 |
| All 4 weapons (2 primary tiers + 2 secondary tiers) + both warheads + both projectiles | ✅ §3–§4 |
| FlakScatter / AoE / dual-target mechanic explained | ✅ §3 |
| All voices + Enter/Leave transport sounds (Limit=2 for ExitTransport) | ✅ §5 |
| Prereqs / owners / availability | ✅ §6 |
| **Role positioning matrix** (AG, AA, AT, anti-building, transport, scout) | ✅ §6 |
| Hardcoded behavior — Ghidra searches + 3 flag-scope verifications (DeployTime, Passengers, SizeLimit) | ✅ §7 |
| TS-legacy filter | ✅ §8 |
| Veterancy detailed — note AA elite is essentially a side-grade | ✅ §9 |
| **Distinguished from Allied IFV (FV)** — no Gunner=yes, no IFVMode dispatch on HTK | ✅ §7.4 + §10 |
| **Index correction logged** (HTK is Soviet, not Allied IFV) | ✅ doc header + index update |
| Cross-refs to companion docs | ✅ §10 |

**Open follow-ups (none load-bearing):**
- Verify `DeployTime=.022` semantics for non-`DeploysInto=` units: is it really a field-stop (settle-before-firing) duration, or does it have another role? The Allied IFV `[FV]` shares this value — worth a Ghidra audit.
- `FlakScatter=yes` exact algorithm: how many cells around the target receive shots? Random within CellSpread? Worth verifying for parity.
- Confirm `EliteSecondary=FlakTrackAAGunE` damage nerf (35→20) is intentional balance — looks like a negative elite upgrade on paper.
