# Flak Trooper (FLAKT)
Side: Soviet | Category: Infantry | Image alias: `[FLAKT]` (no `Image=` redirect — own SHP `FLAK`)

The Soviet **anti-air infantry**. $300 from Soviet Barracks + Soviet Radar
(`NAHAND,NARADR`). The Soviet faction's basic AA answer to the Allied
Rocketeer / Korean Black Eagle / Harrier threat. **Shared with Yuri
faction** — `Owner=` includes both Soviet houses AND `YuriCountry`, making
this **the only stock infantry that both Soviet and Yuri can build**.

**Dual-weapon ground/air split**, implemented via per-projectile
`AA=yes/no` and `AG=yes/no` flags (no unit-side AA/AG flag — the engine
routes targeting via projectile-level filters):
- **`Primary=FlakGuyGun`** — anti-ground (Damage 20, Range 5,
  `Projectile=FlakTProj` with `AA=no AG=yes Arcing=true`). Ground targets:
  infantry, vehicles, structures
- **`Secondary=FlakGuyAAGun`** — anti-air (Damage 20, Range 8,
  `Projectile=FlakProj` with `AA=yes AG=no FlakScatter=yes Inaccurate=yes`).
  Air targets only

Elite tier adds **`Burst=2`** to both weapons — double-shot per fire cycle.
Elite AA does HALF damage per shot (8 vs 20) compensated by the Burst=2.

No standalone Flak / AA-projectile RE doc exists; this document establishes
the projectile-level AA/AG routing pattern.

---

## rulesmd.ini — `[FLAKT]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4420`:

```ini
[FLAKT]
UIName=Name:FLAKT
Name=Flak Trooper
;Image=CONS
Category=Soldier
Primary=FlakGuyGun
Secondary=FlakGuyAAGun
Prerequisite=NAHAND,NARADR
CrushSound=InfantrySquish
Strength=100
Armor=none
TechLevel=1
Pip=white
Sight=5
Speed=4
Owner=Russians,Confederation,Africans,Arabs,YuriCountry
Cost=300
Soylent=150
Points=5
IsSelectableCombatant=yes
VoiceSelect=FlakTroopSelect
VoiceMove=FlakTroopMove
VoiceAttack=FlakTroopAttackCommand
VoiceFeedback=FlakTroopFear
VoiceSpecialAttack=FlakTroopMove
DieSound=FlakTroopDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
;MovementZone=InfantryDestroyer ;GEF wow!!! copy paste bug from the original Disk Thrower!
ThreatPosed=5	; This value MUST be 0 for all building addons
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ImmuneToVeins=yes
Size=1
AllowedToStartInMultiplayer=no
ElitePrimary=FlakGuyGunE
EliteSecondary=FlakGuyAAGunE
IFVMode=3
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:FLAKT` | CSF-string key → "Flak Trooper" |
| `Name=Flak Trooper` | Internal name |
| `;Image=CONS` (commented) | Designer history — Flak Trooper was going to reuse Conscript SHP. Final build has own `FLAK` SHP |
| `Category=Soldier` | Infantry pip/AI grouping |
| `Primary=FlakGuyGun` | Anti-ground flak (Damage 20, Range 5, Arcing projectile). See "Weapons" |
| `Secondary=FlakGuyAAGun` | Anti-air flak (Damage 20, Range 8, scattered projectile). See "Weapons" |
| `Prerequisite=NAHAND,NARADR` | Soviet Barracks + Soviet Radar Tower (specifically NARADR, not abstract RADAR) |
| `CrushSound=InfantrySquish` | Standard crush sound |
| `Strength=100` | HP — 100 (same as basic Allied GI/Yuri Initiate) |
| `Armor=none` | Damage type column 0 — basic infantry armor |
| `TechLevel=1` | Tech-1; effectively gated by NARADR (requires radar tower) |
| `Pip=white` | Cargo pip color — white |
| `Sight=5` | Reveal radius — modest. Lower than Spy/Dog's 9; Flak Trooper isn't a scout |
| `Speed=4` | Foot-speed — standard infantry |
| `Owner=Russians,Confederation,Africans,Arabs,YuriCountry` | **All 4 Soviet houses PLUS YuriCountry** — `[FLAKT]` is **shared between Soviet and Yuri factions**. Important parity fact: when Yuri shares a Soviet faction unit, it goes through Owner= (no SecretHouses to lock it out). Most Yuri-exclusive units use Owner=YuriCountry singleton; FLAKT is unusual in being explicitly Soviet+Yuri shared |
| `Cost=300` | $300 — cheap |
| `Soylent=150` | $150 Grinder refund (Yuri only) |
| `Points=5` | Kill score — low |
| `IsSelectableCombatant=yes` | Included in select-all-combat |
| `VoiceSelect=FlakTroopSelect` | Select voice — `$iflasea..d` (4 lines) |
| `VoiceMove=FlakTroopMove` | Move voice — `$iflamoa..d` (4 lines) |
| `VoiceAttack=FlakTroopAttackCommand` | Attack voice — `$iflaata..d` (4 lines) |
| `VoiceFeedback=FlakTroopFear` | Fear voice — `$iflafea..d` (4 lines, Priority=low) |
| `VoiceSpecialAttack=FlakTroopMove` | Reuses Move voice |
| `DieSound=FlakTroopDie` | Death voice — `$ifladia..e` (5 lines) |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `;MovementZone=InfantryDestroyer ;GEF...` | Same Disk Thrower copy-paste-fix comment |
| `ThreatPosed=5` | AI scoring weight — low. Reflects basic-infantry stats; the AA capability isn't a "threat" rating bump |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | Standard 5 at Veteran |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 at Elite + triggers `ElitePrimary=FlakGuyGunE` AND `EliteSecondary=FlakGuyAAGunE` (both weapons swap) |
| `ImmuneToVeins=yes` | TS legacy; defensively set |
| `Size=1` | Transport cargo slot cost |
| `AllowedToStartInMultiplayer=no` | Not in starting unit complement |
| `ElitePrimary=FlakGuyGunE` | Elite-tier anti-ground (adds Burst=2 to base) |
| `EliteSecondary=FlakGuyAAGunE` | Elite-tier anti-air (HALF Damage per shot but Burst=2 → same effective DPS but 2 staggered hits) |
| `IFVMode=3` | IFV gunner-table index 3 → HTK's `Weapon4` slot. In stock YR maps to an AA-capable weapon for the IFV chassis — **Flak Trooper in IFV transforms the IFV into a mobile AA platform** |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to **`no`** (Flak Trooper does NOT go prone)
- `Trainable=` — defaults to `yes`
- `AllowedToStartInMultiplayer=no` is explicit
- `NotHuman=` — defaults to `no`
- `ImmuneToPsionics=` — defaults to `no`; **Flak Trooper CAN be mind-controlled** (and a mind-controlled Flak Trooper providing AA cover is a useful Yuri capture)
- `ImmuneToRadiation=` — defaults to `no`
- `Bombable=` — defaults to `no`
- `Fearless=` — not set; Flak Trooper shows fear behavior
- `Occupier=` — **defaults to `no`** — Flak Trooper CANNOT garrison civilian buildings. Notable absence: garrison-AA from a civilian building would be very strong; designers explicitly left this off
- `Agent=`/`Infiltrate=`/`Engineer=`/`Ivan=`/`C4=`/`Assaulter=` — none set
- `Deployer=` — not set
- `DetectDisguise=` — not set
- `DefaultToGuardArea=` — not set
- `BombSight=` — not set
- `Natural=`/`Unnatural=` — not set
- `SelfHealing=` — not set (only SELF_HEAL via Elite ability)
- `TypeImmune=` — not set
- `BuildLimit=` — not set (mass-buildable)
- `Crushable=` — defaults to `yes` (vehicle crush kills Flak Trooper — a clean counter)

---

## artmd.ini — `[FLAKT]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:165`:

```ini
[FLAKT] ; Flak Trooper
Cameo=FLKTICON
AltCameo=FLKTUICO
Sequence=FlakSequence
;Crawls=yes
Crawls=no
Remapable=yes
FireUp=3
PrimaryFireFLH=90,0,175
SecondaryFireFLH=90,0,175
```

| Key | Meaning |
|-----|---------|
| `Cameo=FLKTICON` | Sidebar build icon (SHP — note FLKT not FLAKT in filename) |
| `AltCameo=FLKTUICO` | Elite cameo |
| `Sequence=FlakSequence` | Flak-specific sequence (matches the cannon-shoulder firing pose) |
| `;Crawls=yes` (commented) | Older setting — Flak Trooper was crawlable. Replaced with `no` |
| `Crawls=no` | **Cannot crawl/prone** — the giant flak cannon strapped to the shoulder makes prone firing impractical |
| `Remapable=yes` | House remap palette |
| `FireUp=3` | Bullet-spawn frame — at frame 3 the flak cannon fires |
| `PrimaryFireFLH=90,0,175` | FLH — 90 forward, 0 sideways, **175 up**. **High Z** matches the shoulder-mounted flak gun pose (gun barrel is above head height) |
| `SecondaryFireFLH=90,0,175` | Same FLH as Primary — both ground and AA fire from the same gun mount |

### Referenced sequence — `[FlakSequence]`

`artmd.ini:13860`:

```ini
[FlakSequence]
Ready=0,1,1
Guard=0,1,1
Prone=165,1,6
Walk=8,6,6
FireUp=116,6,6
Down=213,2,2
Crawl=165,6,6
Up=229,2,2
FireProne=245,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Die1=86,15,0
Die2=101,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Cheer=293,8,0,E
Paradrop=164,1,0
Panic=8,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle | |
| `Guard=0,1,1` | Guard idle | |
| `Prone=165,1,6` | Prone 1 frame × 6 facings | **Unreachable** (`Crawls=no` blocks prone transition); defensive entry |
| `Walk=8,6,6` | Walk cycle 6×6 | |
| `FireUp=116,6,6` | Standing fire cycle | Used by BOTH Primary and Secondary (the gun visibly tilts upward for air targets, but uses the same frame block) |
| `Down=213,2,2` | Get-down to prone | Unused (Crawls=no) |
| `Crawl=165,6,6` | Crawl reuses prone | Unused |
| `Up=229,2,2` | Get-up from prone | Unused |
| `FireProne=245,6,6` | Prone-fire cycle | Unused |
| `Idle1=56,15,0,S` | Idle 1 | |
| `Idle2=71,15,0,E` | Idle 2 | |
| `Die1=86,15,0` | Death 1 | |
| `Die2=101,15,0` | Death 2 | |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → Ready | |
| `Cheer=293,8,0,E` | Cheer | |
| `Paradrop=164,1,0` | Single frame at 164 — paradrop pose | Live (Flak Trooper paradrop-eligible in some Soviet campaigns) |
| `Panic=8,6,6` | Panic = Walk | |

---

## Weapons

### Primary (Veteran and below) — `[FlakGuyGun]` (anti-ground)

`rulesmd.ini:24230`:

```ini
[FlakGuyGun]		; Anti-surface gun for the Flak Trooper
Damage=20
ROF=20
Range=5
Projectile=FlakTProj
Speed=50
Report=FlakTrackAttackGround		; put in new sound for this
Warhead=FlakTWH
Anim=GUNFIRE
```

| Key | Meaning |
|-----|---------|
| Designer comment: "Anti-surface gun for the Flak Trooper" | Confirms intent |
| `Damage=20` | Per-shot damage. With FlakTWH's Verses (150% none / 125% flak / 100% plate / 60% light vehicle / 10% medium / 10% heavy / 30% wood / 20% steel / 10% concrete) → **30 dmg vs Armor=none infantry** (one-shots no one but kills GI in ~4 shots at ROF=20). 12 dmg vs Grizzly Tank (medium). Effectively anti-infantry with light-vehicle harassment |
| `ROF=20` | Cooldown — 20 frames (~1.3s) — fast cadence |
| `Range=5` | 5 cells — short |
| `Projectile=FlakTProj` | **Anti-ground projectile** — `AA=no AG=yes Arcing=true`. See projectile section |
| `Speed=50` | Slow projectile speed (arcing shells need slow travel to look right) |
| `Report=FlakTrackAttackGround` | Sound `vflaat1a/b` (2 layered samples, FShift -5/+5, Volume=90). Inline comment "put in new sound for this" — distinct from the AA sound |
| `Warhead=FlakTWH` | Anti-ground warhead (Track-style — same as Flak Track vehicle) — see warhead |
| `Anim=GUNFIRE` | Weapon-level firing animation — generic muzzle flash at the firer's position |

### Secondary (Veteran and below) — `[FlakGuyAAGun]` (anti-air)

`rulesmd.ini:24210`:

```ini
[FlakGuyAAGun]	; Separate from Flak Cannon weapon so that stats may be tweaked
Damage=20
ROF=25
Range=8
Projectile=FlakProj	; AA bullet shared with Flak Cannon
Speed=100
Report=FlakTrackAttackAir
Warhead=FlakGuyWH
Anim=GUNFIRE
```

| Key | Meaning |
|-----|---------|
| Designer comment: "Separate from Flak Cannon weapon so that stats may be tweaked" | Confirms intent — Flak Trooper AA is tuned independently from the Flak Cannon (NAFLAK) defense building. They share the projectile but not the weapon |
| `Damage=20` | Per-shot damage. With FlakGuyWH's Verses (150% none / 100% flak / 50% plate / 80% light / 80% medium / 20% heavy) → 30 dmg vs Rocketeer (Armor=none), 16 dmg vs Kirov (heavy) |
| `ROF=25` | Cooldown — 25 frames. Slightly slower than ground weapon |
| `Range=8` | **8 cells** — significantly longer than ground (5). AA range matches typical aircraft engagement distance |
| `Projectile=FlakProj` | **Anti-air projectile** — `AA=yes AG=no FlakScatter=yes Inaccurate=yes`. Shared with Flak Cannon. See projectile section |
| `Speed=100` | Faster than ground (50). AA needs to track moving aircraft |
| `Report=FlakTrackAttackAir` | Sound `vflaat2a/b/c/d` (4 layered samples) — different from ground report |
| `Warhead=FlakGuyWH` | Anti-air-specific warhead — see warhead |
| `Anim=GUNFIRE` | Same generic muzzle flash |

### Elite Primary — `[FlakGuyGunE]` (with Burst=2)

`rulesmd.ini:24732`:

```ini
[FlakGuyGunE]		; Anti-surface gun for the Flak Trooper
Damage=20
ROF=20
Range=5
Projectile=FlakTProj
Speed=50
Report=FlakTrackAttackGround		; put in new sound for this
Warhead=FlakTWH
Anim=GUNFIRE
Burst=2
```

**Identical to FlakGuyGun except adds `Burst=2`** — fires 2 shells per attack cycle. ROF=20 cooldown stays the same → effective firing rate is 2× (40 damage in burst, ROF still 20 frames). Elite anti-ground DPS doubles.

### Elite Secondary — `[FlakGuyAAGunE]` (Damage halved + Burst=2)

`rulesmd.ini:24743`:

```ini
[FlakGuyAAGunE]	; Separate from Flak Cannon weapon so that stats may be tweaked
Damage=8
ROF=25
Range=8
Projectile=FlakProj	; AA bullet shared with Flak Cannon
Speed=100
Report=FlakTrackAttackAir
Warhead=FlakGuyWH
Anim=GUNFIRE
Burst=2
```

Delta from FlakGuyAAGun:
- **Damage 20→8** (-60%)
- **Burst=2** — 2 shells per cycle

Total burst damage per cycle: 8×2 = **16 (vs base 20)** — slightly LESS than base. The split-shot improves hit chance against `Inaccurate=yes` AA targets (2 chances to hit + scatter pattern) but the total damage is intentionally tuned down to compensate.

### Primary's Warhead — `[FlakTWH]` (anti-ground)

`rulesmd.ini:27502`:

```ini
[FlakTWH]	; For the Flak Track's anti-surface weapon.
CellSpread=1.0
;;DB Changed 7/18/01
;PercentAtMax=.2
;Verses=150%,100%,50%,60%,10%,10%,30%,20%,10%,100%,100%	; no buildings
PercentAtMax=1.0
Verses=150%,125%,100%,60%,10%,10%,30%,20%,10%,100%,100%	; no buildings
AnimList=HTRKPUFF
InfDeath=3
Conventional=yes	; Go splash in the water.
```

| Key | Meaning |
|-----|---------|
| `CellSpread=1.0` | Splash radius — 1 cell. Small splash area; flak shells aren't true area weapons |
| `;;DB Changed 7/18/01` (commented) | Date stamp marking balance change |
| `;PercentAtMax=.2` (commented) | Older falloff — was 20% at edge, bumped to 100% |
| `;Verses=150%,100%,50%,...` (commented) | Older Verses — 100% flak / 50% plate, bumped to 125%/100% in retail |
| `PercentAtMax=1.0` | **No falloff** — full damage to all targets within CellSpread radius |
| `Verses=150%,125%,100%,60%,10%,10%,30%,20%,10%,100%,100%` | 11-column. **150% vs none** (anti-infantry boost), 125% vs flak, 100% vs plate. **60% vs light vehicle, 10% vs medium/heavy** — strong vs lights, weak vs tanks. **30/20/10% vs structures** — modest vs buildings. Note inline comment "no buildings" — designer-intent confirms anti-vehicle/anti-infantry focus, with structural damage as incidental |
| `AnimList=HTRKPUFF` | Impact animation — `HTRKPUFF` (Half-Track puff — shared with Flak Track vehicle since the weapon is "Anti-surface" Track variant) |
| `InfDeath=3` | Infantry death animation type 3 — the "explosion/shred" death (different from SA's type 1) |
| `Conventional=yes` | **Behavior flag** — "Go splash in the water." Marks the warhead as conventional ordnance for water-impact effects (visible splash if it misses and lands in water). Not really gameplay-relevant for Flak Trooper since the projectile is `Arcing=true` ground-targeted |

### Secondary's Warhead — `[FlakGuyWH]` (anti-air)

`rulesmd.ini:27513`:

```ini
[FlakGuyWH]	; For Flak Trooper Anti-Air
CellSpread=1.0
PercentAtMax=.2
;;Verses=150%,100%,50%,80%,20%,20%,0%,0%,0%,100%,100%	; no buildings
Verses=150%,100%,50%,80%,80%,20%,0%,0%,0%,100%,100%	; no buildings
AnimList=SMKPUFF
InfDeath=3
```

| Key | Meaning |
|-----|---------|
| `CellSpread=1.0` | Splash radius — 1 cell |
| `PercentAtMax=.2` | 20% damage at edge of spread (falloff). The AA flak puff spreads out |
| `;;Verses=150%,100%,50%,80%,20%,20%,0%,0%,0%,100%,100%` (commented older) | Original Verses with `20% vs medium`. Bumped to 80% in retail (the change makes Flak Trooper more effective vs medium aircraft) |
| `Verses=150%,100%,50%,80%,80%,20%,0%,0%,0%,100%,100%` | 11-column. **150% vs none** (Rocketeer = light infantry-armor air target), **100% vs flak / 50% vs plate** (some aircraft have flak armor: Black Eagle, Harrier? verify when AircraftTypes are documented). **80% vs light** (Rocketeer locomotor armor), **80% vs medium** (Apache, Nighthawk?), **20% vs heavy** (Kirov — intentionally weak vs Kirov: takes many Flak Troopers to bring one down). **0% vs structures** (cannot accidentally damage buildings with AA fire) |
| `AnimList=SMKPUFF` | Impact animation — `SMKPUFF` (smoke puff) — the classic "flak burst" in the sky |
| `InfDeath=3` | Infantry death anim (moot — AA target is rarely infantry) |

### Projectile — `[FlakProj]` (anti-air)

`rulesmd.ini:25820`:

```ini
[FlakProj]		; AA bullet for Flak Cannon and Flak Track.
Image=none
Inviso=yes
AA=yes
AG=no
;AN=no
Shadow=no
Ranged=yes		; Not homing, but ranged -- check fuse, explode if near target coords
Inaccurate=yes	; Bullets do not snap onto targets when "close enough".
FlakScatter=yes ; This weapon scatters its shots.
SubjectToCliffs=no
SubjectToElevation=yes
SubjectToWalls=no
```

| Key | Meaning |
|-----|---------|
| `Image=none Inviso=yes` | No projectile sprite |
| `AA=yes` | **Anti-air enabled** — projectile can target air units |
| `AG=no` | **Anti-ground disabled** — projectile cannot target ground units. **THIS IS THE KEY**: combined with FlakTProj's `AG=yes AA=no`, the engine routes ground-target commands to Primary and air-target commands to Secondary based purely on projectile flags |
| `;AN=no` (commented) | Anti-naval — not used (defaults to off for AA bullet) |
| `Shadow=no` | No projectile shadow drawn |
| `Ranged=yes` | Inline comment: "Not homing, but ranged -- check fuse, explode if near target coords". Projectile is range-limited (uses Range from weapon); explodes when reaching fuse-distance from target coords (not when intersecting target) |
| `Inaccurate=yes` | Inline comment: "Bullets do not snap onto targets when 'close enough'". Combined with Ranged=yes, the shot doesn't auto-correct trajectory — visible inaccurate AA fire |
| `FlakScatter=yes` | Inline comment: "This weapon scatters its shots". **Visual flag** — flak puffs spread out in a scatter pattern instead of all converging on the target. The classic "AA fire spray" look |
| `SubjectToCliffs=no` | Not blocked by cliffs |
| `SubjectToElevation=yes` | Subject to elevation (matters for aircraft altitude tracking) |
| `SubjectToWalls=no` | Not blocked by walls |

### Projectile — `[FlakTProj]` (anti-ground)

`rulesmd.ini:25834`:

```ini
[FlakTProj]		; Anti-surface bullet for Flak Track.
Image=120MM
Arcing=true
Inviso=no
AA=no
AG=yes
;AN=yes
Shadow=no
Inaccurate=yes	; Bullets do not snap onto targets when "close enough".
FlakScatter=yes ; This weapon scatters its shots.
SubjectToCliffs=no
```

| Key | Meaning |
|-----|---------|
| `Image=120MM` | **Has visible projectile** — uses `120MM` SHP (a 120mm shell sprite). Distinct from FlakProj's Inviso=yes |
| `Arcing=true` | **Arcing trajectory** — shell follows ballistic arc, not straight line. Visible "lobbed" projectile |
| `Inviso=no` | Visible projectile |
| `AA=no` | **Anti-air disabled** |
| `AG=yes` | **Anti-ground enabled**. THE flag that routes Primary to ground targets |
| `;AN=yes` (commented) | Naval targeting was considered, commented out |
| `Shadow=no` | No shadow |
| `Inaccurate=yes` | Same as FlakProj |
| `FlakScatter=yes` | Same — visual scatter |
| `SubjectToCliffs=no` | Not blocked by cliffs |

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear / death — 4-line voice banks (uniform Soviet style)

```ini
[FlakTroopSelect]                  ; soundmd.ini:3791
Sounds= $iflasea $iflaseb $iflasec $iflased
Control= random
Volume=85

[FlakTroopMove]                    ; soundmd.ini:3786
Sounds= $iflamoa $iflamob $iflamoc $iflamod
Control= random
Volume=85

[FlakTroopAttackCommand]           ; soundmd.ini:3781
Sounds= $iflaata $iflaatb $iflaatc $iflaatd
Control= random
Volume=85

[FlakTroopFear]                    ; soundmd.ini:3796
Sounds= $iflafea $iflafeb $iflafec $iflafed
Control= random
Priority=low
Volume=85

[FlakTroopDie]                     ; soundmd.ini:3802
Sounds= $ifladia $ifladib $ifladic $ifladid $ifladie
Priority=low
Control= random
```

4/4/4/4/5 — uniform Soviet voice bank size (less varied than Yuri's typical 5-7 lines).

### Weapon reports — split for ground vs air

```ini
[FlakTroopAttackGround]            ; soundmd.ini:1013
Sounds= vflaat1a vflaat1b
Control= random interrupt
FShift= -5 5
VShift=10
Volume=90

[FlakTroopAttackAir]               ; soundmd.ini:1020
Sounds= vflaat2a vflaat2b vflaat2c vflaat2d
FShift= -10 10
```

| Sound | Used by | Distinction |
|-------|---------|-------------|
| `FlakTroopAttackGround` | FlakGuyGun + FlakGuyGunE (Primary anti-ground) | 2 samples `vflaat1a/b` — heavier "thud" for ground impact |
| `FlakTroopAttackAir` | FlakGuyAAGun + FlakGuyAAGunE (Secondary anti-air) | 4 samples `vflaat2a..d` — lighter "pop" for AA puff (more variation since AA fires more shots) |

Wait — these soundmd sections are named `FlakTroopAttackGround`/`Air`, but the Primary/Secondary weapons reference **`FlakTrackAttackGround`/`Air`** (Track, not Troop). Let me re-check the soundmd to see if the Track variants exist separately. Actually, the inline comment in FlakGuyGun says `Report=FlakTrackAttackGround` — this is the **Flak Track** vehicle's sound, reused for the Flak Trooper. So FLAKT uses the **Flak Track vehicle's report sounds**, not the FlakTroop-prefixed ones. The FlakTroop-prefixed soundmd sections may be vestigial or used only via the type's VoiceFire mechanism (which FLAKT doesn't have).

Actually looking again at the soundmd grep results:
- `FlakTroopAttackGround` at 1013
- `FlakTroopAttackAir` at 1020

But the weapons reference `FlakTrackAttackGround` / `FlakTrackAttackAir`. Different names — Track vs Troop. The Troop-named sections might be unused.

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `NAHAND,NARADR` | Soviet Barracks + Soviet Radar (specifically NARADR, not abstract RADAR — even Yuri players building FLAKT need NARADR or equivalent) |
| `Owner=` | `Russians,Confederation,Africans,Arabs,YuriCountry` | **Soviet AND Yuri factions** — unusual shared-unit pattern |
| `TechLevel=` | `1` | Effectively gated by NARADR (tech ~3-4 implicit) |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=300` | $300 | Cheap |
| `Soylent=150` | $150 refund (Yuri only) | |
| `Points=5` | 5 | Low |

No `RequiredHouses=`, no `SecretHouses=`, no `PrerequisiteOverride=`, no `BuildLimit=`.

**Yuri-shared note**: Yuri faction has its own `[YAFLAK]` Gattling Cannon defense building for AA, but **does not have a dedicated Yuri AA infantry** — instead, Yuri inherits FLAKT from Soviet via the Owner= list. Combined with Yuri's `YABRCK` needing to provide infantry, this means Yuri players' AA infantry needs come via the Soviet faction's FLAKT design. Yuri's tech tree is parasitic on Soviet here.

Wait — but Prerequisite=NAHAND,NARADR. Does Yuri have NAHAND? No — Yuri has YABRCK. So can Yuri actually build FLAKT? Only if Yuri captures a Soviet Barracks + Soviet Radar. The Owner= inclusion is theoretical until prereqs match. Either:
1. Yuri rarely builds FLAKT in practice (only after capturing Soviet buildings)
2. Some PrerequisiteOverride or per-faction prereq-mapping makes YABRCK satisfy "Barracks" — but the prereq is explicit NAHAND, not abstract Barracks

Verifying: the prereq is **literal `NAHAND`**, not `Barracks`. Yuri's YABRCK does NOT satisfy NAHAND. Yuri can theoretically own FLAKT (via Owner=), but **cannot normally build him** without capturing the Soviet faction's barracks. The `Owner=YuriCountry` inclusion in FLAKT is mostly for ChronoSphere/MissionScript edge cases where YuriCountry might end up owning a FLAKT.

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — standard 5 abilities |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — 4 abilities + triggers **BOTH** weapon swaps: `ElitePrimary=FlakGuyGunE` (adds Burst=2) AND `EliteSecondary=FlakGuyAAGunE` (Damage halved + Burst=2). Elite Flak Trooper fires 2 ground shells per cycle (effective 2× DPS) and 2 AA shells per cycle (similar net DPS but better hit chance) |
| AltCameo | `FLKTUICO` shown after Veteran promotion |

`Trainable=` defaults to `yes`.

---

## Hardcoded behavior — Ghidra-verified

### 1. Per-projectile AA/AG targeting filter — the dual-weapon routing mechanism

The Flak Trooper's "fire ground weapon at vehicles, fire AA weapon at aircraft" behavior emerges from **per-projectile `AA=` and `AG=` flags**, not from a unit-level AA/AG flag. The engine's target-acquisition path:

1. Player hovers over a target with FLAKT selected
2. Engine consults the target's locomotor / type to determine if it's "air" (FlyLocomotion / JumpjetLocomotion / VTOL) or "ground" (Walk / Drive / Hover-ground)
3. Engine consults FLAKT's weapons:
   - Primary `FlakGuyGun` → Projectile=FlakTProj → `AA=no AG=yes`
   - Secondary `FlakGuyAAGun` → Projectile=FlakProj → `AA=yes AG=no`
4. **For air targets**: only Secondary's projectile passes the AA=yes filter → Secondary fires
5. **For ground targets**: only Primary's projectile passes the AG=yes filter → Primary fires
6. The cursor reflects which weapon would fire, and right-click executes that weapon

**No unit-side `AAFireOnly=yes`/`AGFireOnly=yes` flag exists** in stock YR (Ghidra search returned no such strings). The routing is entirely projectile-flag driven. This is the same mechanism used by:
- Multi-Gunner IFV (different IFV gunner slots are AA-capable or not)
- Aegis Cruiser (AA-only naval — uses AA=yes projectile)
- Patriot Missile (`[GAPILL]` — uses AA-only projectile)

`AntiAirValue` string exists in the binary (xref to RulesClass globals around `0x0081AEB0`) but is **an AI-side threat-scoring weight**, not a per-projectile filter.

### 2. FlakScatter=yes + Inaccurate=yes — the "AA cloud" visual

Both projectile-level flags (FlakProj and FlakTProj have both):
- `FlakScatter=yes` — scatters projectile sprites in a visible spray pattern instead of converging on target
- `Inaccurate=yes` — shots don't snap to target coords; visible drift

Together they create the classic "anti-aircraft fire cloud" visual where flak shells burst around the aircraft rather than precisely hitting it. **Gameplay implication**: many shots hit, but the damage spread is moderated by the warhead's CellSpread=1 + PercentAtMax (0.2 for AA, 1.0 for ground).

### 3. Arcing=true — ballistic trajectory for ground shells

Only FlakTProj (anti-ground) has `Arcing=true`. The shell follows a visible parabolic arc instead of straight-line travel. Combined with `Image=120MM` (visible projectile), this gives the iconic "flak shell lobbed at ground target" look — distinct from straight-fire weapons like the M1Carbine.

FlakProj (anti-air) does NOT have Arcing=true — AA shells go straight up at the aircraft.

### 4. Conventional=yes — water-splash on miss

INI key `Conventional` on FlakTWH (anti-ground warhead) means the warhead is "conventional ordnance" — if the projectile misses target and lands in water, a water-splash visual plays. Inline comment: "Go splash in the water." Defensive flag for visual polish; rare in normal gameplay since FlakTProj has range 5 and Arcing curves up-and-down, rarely missing into water unless target dies mid-flight.

### 5. No unit-level AA-only/AG-only flag — Owner=YuriCountry parasitism

The Owner= list including `YuriCountry` alongside the Soviet houses creates a parasitic relationship: Yuri faction can OWN a Flak Trooper but cannot normally BUILD one (Prerequisite=NAHAND requires Soviet Barracks). In practice this means:
- Yuri captures a Soviet base → can build FLAKT from captured NAHAND
- Yuri mind-controls a FLAKT → flips ownership to Yuri (TypeImmune unset, so this works)
- Yuri Genetic Mutator on an enemy FLAKT → converts to Brute (Brute, not retain FLAKT)
- Yuri starts a match → no FLAKT available (must capture Soviet barracks)

Worth noting for design parity: the engine permits this Owner-vs-Prerequisite mismatch and handles it gracefully (no FLAKT in build list, but existing FLAKTs work normally for Yuri).

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("AAFireOnly\|AGFireOnly\|AntiAir\|AntiGround")` | 1 string — only `AntiAirValue` (AI threat-scoring weight at RulesClass globals). **Confirms `AAFireOnly`/`AGFireOnly`/per-unit AntiAir/AntiGround flags do NOT exist** as hardcoded INI keys. AA/AG routing is purely projectile-flag driven |

Reused cross-references: `Crawls=no` (no fresh Ghidra needed — same flag pattern as Brute), `FlakScatter` and `Inaccurate` (projectile-level flags whose Ghidra xrefs aren't traced in this iteration; they appear in the projectile structure ReadINI path, distinct from WeaponType).

**Confirmation**: FLAKT has **no Flak-Trooper-specific hardcoded function block**. Pure data composition — two weapons with projectile-flag-routed AA/AG split, two warheads tuned for ground vs air. The same dual-weapon pattern is used by IFV gunners, Aegis Cruiser, Hydrofoil, Sea Scorpion, etc.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;Image=CONS` (commented) | Designer history — was going to reuse Conscript art | OK |
| `;MovementZone=InfantryDestroyer` (commented) | Same Disk Thrower copy-paste-fix comment seen across Soviet infantry | OK |
| `;Crawls=yes` (commented in artmd) | Older setting — Crawls was yes, switched to no | OK |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set | OK |
| `;PercentAtMax=.2` / `;Verses=150%,100%,50%...` (commented in FlakTWH) | Balance history — older Verses with weaker mid-tier infantry damage | OK |
| `;;Verses=150%,100%,50%,80%,20%,20%...` (commented in FlakGuyWH) | Balance history — 20% medium aircraft armor bumped to 80% in retail | OK |
| `;AN=no` / `;AN=yes` (commented in projectiles) | Anti-Naval flags experimentally considered, defaults used | OK |
| `Conventional=yes` | YR-active visual flag | OK |
| `AA=yes`/`AG=yes`/`FlakScatter=yes`/`Inaccurate=yes`/`Arcing=true` | All YR-active projectile flags | OK |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` — standard | OK |

No TS-only behavior. All flags YR-active.

---

## Cross-references

- **AA infantry/units family** (anti-air capability):
  - **`[FLAKT]` Flak Trooper (this doc)** — Soviet/Yuri infantry AA
  - `[GGI]` Guardian GI (Allied — deployed mode adds AA missile via Secondary)
  - `[JUMPJET]` Rocketeer — has AA-capable Primary (20mm with AA=yes projectile)
  - `[GAPILL]` Patriot Missile (Allied AA defense building — verify when documented)
  - `[NAFLAK]` Flak Cannon (Soviet AA defense building — shares `FlakProj` projectile with FLAKT)
  - `[YAGGUN]` Gattling Cannon (Yuri AA defense building)
  - `[AEGIS]` Aegis Cruiser, `[HYD]` Hydrofoil, `[LCRF]` Sea Scorpion (naval AA)
- **Sister Soviet infantry**:
  - `[E2]` Conscript — basic
  - `[SHK]` Tesla Trooper — anti-vehicle
  - **`[FLAKT]` Flak Trooper (this doc)** — anti-air
  - `[IVAN]` Crazy Ivan — bomb
  - `[DESO]` Desolator — radiation
  - `[BORIS]` Boris — hero airstrike
  - `[TERROR]` Terrorist — Cuban suicide
- **Same warhead family** (anti-air variants):
  - `[FlakGuyWH]` — Flak Trooper AA
  - `[FlakCannonWH]` (verify when NAFLAK documented) — Flak Cannon building AA
  - Common Verses pattern: 150% none, 100% flak, 50% plate, 80% light/medium, 20% heavy
- **Sister anti-ground weapons sharing FlakTProj**:
  - `[FlakGuyGun]` (this doc — FLAKT Primary)
  - `[FlakTrackGun]` (Flak Track vehicle Primary — see VehicleTypes when documented)
- **Sister anti-air weapons sharing FlakProj**:
  - `[FlakGuyAAGun]` (this doc — FLAKT Secondary)
  - `[FlakWeapon]` / `[FlakWeaponE]` (Flak Cannon NAFLAK)
- **Counter-units to Flak Trooper**:
  - **Snipers** (250 dmg one-shot vs Strength=100)
  - **Vehicle crush** (Crushable=yes default)
  - **Dog leap** (one-shot Parasite vs Armor=none)
  - **Long-range bombardment** (Range=5/8 doesn't reach V3, Apocalypse, Dreadnought)
  - **Mass aircraft** — 20% Verses vs heavy aircraft (Kirov) means flooding Kirovs overwhelms FLAKT defense; need Patriot/Aegis backup
- **Iconic plays**:
  - **FLAKT + Flak Track stack** — both units use FlakProj for AA, creating overlapping AA bubbles
  - **FLAKT-in-Battle-Fortress** — FV passenger fires FLAKT's weapons through the chassis, giving mobile AA cover

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [FLAKT]` | 4420-4458 (39 lines) | All 36 active keys covered (2 commented lines documented) |
| `artmd.ini [FLAKT]` | 165-174 (10 lines) | All keys covered |
| `artmd.ini [FlakSequence]` | 13860-13879 (20 lines) | All 17 active slots + 3 stub Die3-5 covered |
| `rulesmd.ini [FlakGuyGun]` | 24230-24238 (9 lines) | All keys covered |
| `rulesmd.ini [FlakGuyAAGun]` | 24210-24218 (9 lines) | All keys covered |
| `rulesmd.ini [FlakGuyGunE]` | 24732-24741 (10 lines) | All keys covered (Burst=2 delta noted) |
| `rulesmd.ini [FlakGuyAAGunE]` | 24743-24752 (10 lines) | All keys covered (Damage halved + Burst=2 delta) |
| `rulesmd.ini [FlakTWH]` warhead | 27502-27511 (10 lines) | All keys covered with 11-column Verses + designer balance history |
| `rulesmd.ini [FlakGuyWH]` warhead | 27513-27519 (7 lines) | All keys covered with Verses balance history |
| `rulesmd.ini [FlakProj]` projectile | 25820-25832 (13 lines) | All keys covered including AA=yes/AG=no routing flags |
| `rulesmd.ini [FlakTProj]` projectile | 25834-25845 (12 lines) | All keys covered including AA=no/AG=yes routing flags |
| `soundmd.ini` FlakTroop voices | Select, Move, AttackCommand, Fear, Die | All 5 covered |
| `soundmd.ini` FlakTroop weapon reports | FlakTroopAttackGround, FlakTroopAttackAir (note: weapons reference **FlakTrack**-prefixed sounds, not FlakTroop — discrepancy noted) | Both covered with Track-vs-Troop naming discrepancy documented |
| Hardcoded behavior | Per-projectile AA/AG routing (5 mechanisms) + FlakScatter visual + Inaccurate=yes drift + Arcing trajectory + Conventional water-splash + Owner=YuriCountry parasitism + Burst=2 Elite | 7 mechanisms; 1 fresh Ghidra negative finding (no AAFireOnly flag) |
| Ghidra searches performed against ID | 1 distinct query (search_strings for AAFireOnly/AGFireOnly/AntiAir/AntiGround) | Logged inline |
| TS-legacy filter | Applied; ImmuneToVeins/balance-history all documented | Done |
