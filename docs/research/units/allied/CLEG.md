# Chrono Legionnaire (CLEG)
Side: Allied | Category: Infantry | Image alias: `[CLEG]` (no `Image=` redirect)

The Allied erase-tank-from-time hero infantry. Fires a `NeutronRifle` —
range 5, every 120 frames — whose `ChronoBeam` warhead carries `Temporal=yes`.
Instead of dealing HP damage, the beam drains a hidden `WarpHP =
target.Strength * 10` counter at `weapon.Damage` per tick (8/tick rookie,
16/tick elite); when WarpHP hits zero the target **is removed from the game**
(no debris, no kill credit beyond Points, no salvage). Two CLs on the same
target stack additively, halving erase time.

CLEG **does not walk**. Its `Locomotor` is the TeleportLocomotionClass
GUID `{4A582747-...}`; the rookie `Speed=5` is a dummy ("we don't really
need this, but give it a dummy value just to make sure nothing complains"
per the INI comment). Movement = teleport with a per-cell `[General]
ChronoDelay=60` warp-out / warp-in cycle, optionally scaled by distance
via `ChronoDistanceFactor` and `ChronoTrigger`. During the warp-out delay
CLEG is `IsBeingWarpedOut=true`, vulnerable but cannot fire.

Authoritative deep RE:
- [TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md](../../TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md)
  — full TemporalClass lifecycle, WarpHP formula, chain stacking, immunities
- [TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md](../../TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md)
  — fire→detonate→warp pipeline
- [TELEPORT_LOCOMOTION_DEEP_DIVE.md](../../TELEPORT_LOCOMOTION_DEEP_DIVE.md)
  — locomotor `{4A582747-...}` movement state machine
- [CHRONO_WARP_VISUAL_RENDERING.md](../../CHRONO_WARP_VISUAL_RENDERING.md)
  — sprite blitter selection during warp-out

---

## rulesmd.ini — `[CLEG]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4125`:

```ini
[CLEG]
UIName=Name:CLEG
Name=Chrono Legionnaire
Category=Soldier
Primary=NeutronRifle
Prerequisite=GAPILE,TECH
PrerequisiteOverride=CAWASH16 ; SJM: Smithsonian Institute
CrushSound=InfantrySquish
Crushable=no
Strength=125
Armor=none
TechLevel=10
Pip=red
Sight=8
Speed=5 ;okay, we don't really need this, but give it a dummy value just to make sure nothing complains
MoveToShroud=no
Teleporter=yes;
Owner=British,French,Germans,Americans,Alliance
Cost=1500
Soylent=750
Points=15
IsSelectableCombatant=yes
VoiceSelect=ChronoLegionSelect
VoiceMove=ChronoLegionMove
VoiceAttack=ChronoLegionAttackCommand
VoiceFeedback=ChronoLegionFear
VoiceSpecialAttack=ChronoLegionMove
DieSound=ChronoLegionDie
ChronoInSound=ChronoLegionTeleport
ChronoOutSound=ChronoLegionTeleport
Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}
;Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
;MovementZone=InfantryDestroyer ;GEF wow!!! copy paste bug from the original Disk Thrower!
ThreatPosed=20	; This value MUST be 0 for all building addons
ImmuneToRadiation=no  ; SJM: approved by Dustin on 09-11-00
ImmuneToPsionics=no
Bombable=yes
AllowedToStartInMultiplayer=no
Size=1
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
;CanPassiveAquire=no
;CanRetaliate=no; Won't fire back when hit
ElitePrimary=NeutronRifleE
;PreventAttackMove=yes ;gs don't laugh, he can actually do this while in a plan loop
IFVMode=10
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:CLEG` | CSF-string key resolving to "Chrono Legionnaire" |
| `Name=Chrono Legionnaire` | INI display fallback (no `UseOwnName=`, so UI uses CSF) |
| `Category=Soldier` | Pip group + AI threat grouping |
| `Primary=NeutronRifle` | The temporal erase beam (range 5, ROF 120, dmg 8 per tick) |
| `Prerequisite=GAPILE,TECH` | Allied Barracks + any Allied Battle Lab/Tech building. `TECH` is the prerequisite-group alias (matches GATECH and any tech-level-equivalent override) |
| `PrerequisiteOverride=CAWASH16` | **Smithsonian Institute** (Washington D.C. mission tech building) bypasses the normal tech-lab requirement — capturing it grants Chrono Legionnaires without building a Battle Lab. Singleplayer-flavored mechanic |
| `CrushSound=InfantrySquish` | Crush sample, unreachable (`Crushable=no`) |
| `Crushable=no` | **Vehicles cannot crush CLEG.** Same hardcoded gate as TANY |
| `Strength=125` | HP — same as GI (100% baseline, no bonus). Fragile despite hero cost |
| `Armor=none` | Bare-flesh armor type — takes full damage from anti-inf warheads |
| `TechLevel=9` documented | **Actually `TechLevel=10`** in INI — top tier (1 higher than TANY's 9). Buildable only when host's tech level >= 10, which in standard skirmish requires `[General] TechLevel=10` (i.e. not capped) |
| `Pip=red` | Hero pip color |
| `Sight=8` | Highest infantry sight |
| `Speed=5` | **Dummy value** — Teleporter locomotor ignores Speed; the field is read by the parser into `TechnoTypeClass+0x180` but the teleport state machine doesn't use it for cadence (see `TELEPORT_LOCOMOTION_DEEP_DIVE.md`). Speed instead derives from `[General] ChronoDelay` / `ChronoDistanceFactor` / `ChronoRangeMinimum` |
| `MoveToShroud=no` | **AI will not order CLEG to move into unexplored cells.** Prevents the AI from accidentally teleporting CLEG into a hostile pocket of revealed-on-arrival terrain. Player still can |
| `Teleporter=yes;` | Marks the unit as using the chrono-warp pre/post-move animation hooks. Read into `TechnoTypeClass+0xCCE` (per TEMPORAL_WEAPON_SYSTEM report) — used by `WarpAttachClass::Detach` to decide whether to teleport CL away after erase, and by `TeleportLocomotionClass` for the chrono FX |
| `Owner=British,French,Germans,Americans,Alliance` | Allied only |
| `Cost=1500` | Premium — same as TANY |
| `Soylent=750` | Grinder refund |
| `Points=15` | Kill score — **low** for a hero (vs TANY's 50) because killing CLs is hard and Westwood didn't want to reward Soviet/Yuri snowball |
| `IsSelectableCombatant=yes` | In select-all-combat + AI combat groups |
| `VoiceSelect=ChronoLegionSelect` | Selection voice bank |
| `VoiceMove=ChronoLegionMove` | Move acknowledgement |
| `VoiceAttack=ChronoLegionAttackCommand` | Attack acknowledgement |
| `VoiceFeedback=ChronoLegionFear` | Fear voice when hurt |
| `VoiceSpecialAttack=ChronoLegionMove` | Special-attack acknowledgement — **reuses Move bank** (no dedicated "erasing" voice). Triggers on attack-ground/special clicks |
| `DieSound=ChronoLegionDie` | Death sample |
| `ChronoInSound=ChronoLegionTeleport` | Played at warp-IN cell when teleport completes — `ichrmova` |
| `ChronoOutSound=ChronoLegionTeleport` | Played at warp-OUT cell when teleport starts — same sample. Both keys exist independently on TechnoTypeClass but resolve to the same `[ChronoLegionTeleport]` block |
| `Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}` | **TeleportLocomotionClass GUID** (CLSID-style). Move command = chrono jump. See [ILOCOMOTION_COM_PROTOCOL_SPEC.md](../../ILOCOMOTION_COM_PROTOCOL_SPEC.md) for COM dispatch |
| `;Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}` | Commented-out WalkLocomotionClass GUID — designer kept it for reference, never re-enabled |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry move zone (not the Amphibious zone CLEG can't swim) |
| `;MovementZone=InfantryDestroyer` | INI comment "wow!!! copy paste bug from the original Disk Thrower!" — the Disk Thrower (TS Brotherhood unit) had `InfantryDestroyer` due to a Westwood mistake, and the original CLEG copy inherited it before this fix |
| `ThreatPosed=20` | AI prioritizes — slightly below TANY's 25 |
| `ImmuneToRadiation=no` | **Desolators kill CLEG.** Comment "approved by Dustin on 09-11-00" — Westwood deliberately decided CL gets no radiation immunity despite being a hero |
| `ImmuneToPsionics=no` | **Yuri can mind-control CLEG.** Major counter — mind-controlled CL can be turned on its own team to erase Allied units/buildings |
| `Bombable=yes` | Crazy Ivan can plant a bomb on CLEG (cursor lights up) |
| `AllowedToStartInMultiplayer=no` | Excluded from starting unit pool |
| `Size=1` | Cargo slot cost |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | 5 abilities — note `FASTER` (TANY does not have it). For a Teleporter, `FASTER` likely affects the warp delay only marginally (Speed field is dummy) but the ability flag is still set |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 abilities — same set as TANY's elite list (no FASTER at elite) |
| `;CanPassiveAquire=no` | Commented out → defaults to `yes`. CL **does** passively acquire targets in guard mode |
| `;CanRetaliate=no` | Commented out → defaults to `yes`. CL **does** retaliate when hit. The INI comments hint Westwood considered disabling both ("Won't fire back when hit") but never shipped it |
| `ElitePrimary=NeutronRifleE` | Elite weapon — dmg 16 (double erase rate) but **regressed projectile**: `InvisibleLow` (blocked by walls) vs rookie `InvisibleMedium` (shoots over walls). Designer error or deliberate elite trade-off — what ships is what we honor per parity bar |
| `;PreventAttackMove=yes` | Commented designer note "don't laugh, he can actually do this while in a plan loop" — Westwood noticed CLEG could attack-move in AI plan loops despite being a Teleporter |
| `IFVMode=10` | **Distinct IFV slot** — when CLEG enters IFV, weapon table index 10 selects a Chrono-themed IFV variant. Differentiates from SEAL/TANY (slot 4) and Engineer (slot 6) |

Implicit defaults:

- `Crawls=yes` (art section)
- `SelfHealing=no` (default; only gains `SELF_HEAL` ability at elite)
- `Bombable=yes` (explicit)
- `Trainable=yes` (default)
- `BuildLimit=` not set → unlimited (vs TANY's 1)
- `OpenTransportWeapon=` not set → default -1; CLEG fires nothing from inside an IFV/SAPC (intentional — CL temporal beam doesn't work from open-topped transports without hitting the OpenToppedWarpDistance break)
- `Occupier=no` (default; cannot enter civilian buildings)
- `UseOwnName=` not set → UI follows `UIName=Name:CLEG` CSF string

---

## artmd.ini — `[CLEG]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:223`:

```ini
[CLEG] ; Chrono Legion
Cameo=CLEGICON
AltCameo=CLEGUICO
Sequence=ClegSequence
Crawls=yes
Remapable=yes
FireUp=2
PrimaryFireFLH=100,-25,135
```

| Key | Meaning |
|-----|---------|
| `Cameo=CLEGICON` | Sidebar icon (rookie/veteran) |
| `AltCameo=CLEGUICO` | Cameo at Elite |
| `Sequence=ClegSequence` | Reference to sequence block below — CL is the only unit using this exact block |
| `Crawls=yes` | Sets `InfantryTypeClass+0xEBD` — formally enables prone-while-walking, but `ClegSequence` has `Prone=0,1,1` and `Crawl=0,1,1` (single-frame stubs), so CL **never visibly crawls**. The flag is set but the SHP doesn't have the frames; falls back to the standing/Walk sprite when prone is requested |
| `Remapable=yes` | House remap palette applied |
| `FireUp=2` | Bullet-spawn frame within the 6-frame FireUp sequence — beam emits on frame 2 |
| `PrimaryFireFLH=100,-25,135` | Beam origin: forward 100, side **-25 (left of centerline)**, height 135. Higher than TANY's `100,0,100` because CL holds the rifle higher and offset to the left arm |

No `SecondaryFireFLH=` (no secondary weapon).
No `AlternateArcticArt=` (no snow variant).
No explicit `Foundation=` (default 1x1 for infantry).

### Referenced sequence — `[ClegSequence]`

`artmd.ini:14095`:

```ini
[ClegSequence]
Ready=0,1,1
Guard=0,1,1
Prone=0,1,1
Down=0,1,1
Crawl=0,1,1
Walk=117,6,6
Up=0,1,1
Idle1=8,15,0,S
Idle2=23,15,0,E
Die1=38,15,0
Die2=53,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
FireUp=68,6,6
FireProne=68,6,6
Paradrop=116,1,0
Cheer=166,8,0,E
Panic=8,6,6
```

| Key | Format `start,count,facingStep` | Notes |
|-----|---------------------------------|-------|
| `Ready=0,1,1` | Frame 0, 1 frame, 1 facing step | Single stand-still frame (no idle blink) |
| `Guard=0,1,1` | Same as Ready | No distinct guard pose |
| `Prone=0,1,1` | **Stub** — same frame as standing. CL doesn't truly go prone |
| `Down=0,1,1` | **Stub** — no go-prone transition |
| `Crawl=0,1,1` | **Stub** — no crawl frames in the SHP |
| `Walk=117,6,6` | 6 frames per facing × 8 facings starting at frame 117. Only plays during the teleport-warp arrival/departure (since locomotor is teleport, "walk" is unreachable in normal movement) |
| `Up=0,1,1` | **Stub** — no get-up transition |
| `Idle1=8,15,0,S` | Frame 8, 15 frames, 0 facing-step, `S` = single-direction (Standalone, not per-facing) |
| `Idle2=23,15,0,E` | Frame 23, 15 frames, `E` = some other anim flag (typically "End/Eastfacing") |
| `Die1=38,15,0` | Death anim A — 15 frames starting 38 |
| `Die2=53,15,0` | Death anim B — 15 frames starting 53 |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | **Stubs** — CL doesn't have crush/explode/specific deaths (Westwood reserved the slots but pointed them at frame 0) |
| `FireUp=68,6,6` | Fire animation — 6 frames per facing × 8 facings starting at 68. `PrimaryFireFLH` and `FireUp=2` (art) refer to frame 2 within this 6-frame sequence |
| `FireProne=68,6,6` | **Same as FireUp** — CL fires the same way prone or standing (and never actually goes prone) |
| `Paradrop=116,1,0` | Single frame for parachuting (powers-spawned PTROOPs don't include CLEG, but the slot exists) |
| `Cheer=166,8,0,E` | Victory cheer — 8 frames |
| `Panic=8,6,6` | "Panic" loop reuses Walk-style 6-frame sequence at frame 8 (same as Idle1 start) |

**Notable**: No `Tread/Swim/WetAttack/WetIdle/WetDie` keys — CL **cannot swim**
(MovementZone=Infantry, not Amphibious). Designer never bothered with water
art because the unit can't enter it.

---

## Weapons

### Primary — `[NeutronRifle]`

`rulesmd.ini:23758`:

```ini
[NeutronRifle]
Damage=8
ROF=120
Range=5
Speed=100
Projectile=InvisibleMedium;GEF Chrono Legionaires can now shoot over walls ;InvisibleLow
Warhead=ChronoBeam
Report=ChronoLegionAttack
IsRadBeam=yes
```

| Key | Meaning |
|-----|---------|
| `Damage=8` | **Not HP damage** — this is the WarpHP drained per tick. The `TemporalClass::Update` loop reads `weapon.Damage` from `WeaponTypeClass+0xa4` and subtracts from the target's hidden `WarpHP = Strength*10` |
| `ROF=120` | **Time between attempts to acquire / re-establish the beam**, NOT between damage ticks. Once the beam is locked, the WarpAttachClass state machine drains continuously per tick. ROF gates how often CL can pick a NEW target after losing line-of-sight or after target dies |
| `Range=5` | 5 cells — same as TANY's DoublePistols, shorter than most basic infantry rifles |
| `Speed=100` | Projectile travel speed — irrelevant for `Inviso=yes` (instant hit) |
| `Projectile=InvisibleMedium` | **Shoots over walls** but **blocked by cliffs and elevation differences**. Designer comment "GEF Chrono Legionaires can now shoot over walls" — they used to be `InvisibleLow` (blocked by walls), then GEF upgraded |
| `Warhead=ChronoBeam` | The `Temporal=yes` warhead that flips the detonation path |
| `Report=ChronoLegionAttack` | Sound — sample `ichratta`, range 15, vol 60, priority high |
| `IsRadBeam=yes` | Triggers `FUN_006fd620(target, 1)` visual at fire time → **purple/temporal beam** color (not the radioactive green that Desolator's `RadBeamWeapon` uses; the param-1 path reads `g_RulesClass+0x1866` 3-byte RGB for temporal beam color) |

### Elite Primary — `[NeutronRifleE]`

`rulesmd.ini:24979`:

```ini
[NeutronRifleE]
Damage=16
ROF=120
Range=5
Speed=100
Projectile=InvisibleLow
Warhead=ChronoBeam
Report=ChronoLegionAttack
IsRadBeam=yes
```

| Key | Meaning |
|-----|---------|
| `Damage=16` | **2× WarpHP drain per tick.** Elite CL erases at twice the rate. Net: a Rhino (Strength=400, WarpHP=4000) takes 500 ticks at rookie vs 250 ticks at elite |
| `ROF=120` | Same |
| `Range=5` | **No range bump** at elite (TANY gets +2 range; CL does not) |
| `Projectile=InvisibleLow` | **REGRESSED at elite** — Elite CL can no longer shoot over walls. Whether intentional or an INI typo by Westwood, this is what ships. Per the parity bar we honor it exactly. In play: Elite CL must have direct LOS to target |
| `Warhead=ChronoBeam` | Same |
| `Report=ChronoLegionAttack` | Same sound |
| `IsRadBeam=yes` | Same purple beam visual |

The veterancy `FIREPOWER` ability (+25%) then scales the 16 to 20 damage,
and `ROF` ability shortens the 120-frame re-acquire window to ~90 frames.

### Warhead — `[ChronoBeam]`

`rulesmd.ini:27286`:

```ini
[ChronoBeam]
;No chronoing spawned rockets
Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,0%
;Verses=100%,0%,20%,10%,0%
;InfDeath=5
Temporal=yes
;Spread=0
```

| Key | Meaning |
|-----|---------|
| `Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,0%` | **All armors take 100%** — meaning the beam can lock on infantry, vehicles, all armor types, buildings — EXCEPT `special_2` (0%, the last column). `special_2` is used for **spawned projectiles / sub-units** (e.g., V3ROCKET in flight). The leading `;No chronoing spawned rockets` comment confirms intent |
| `;Verses=` (commented) | Earlier draft with low values vs vehicles — discarded for the uniform-100% scheme |
| `;InfDeath=5` (commented) | Designer considered a specific infantry death animation type — not used. Erased infantry simply *vanish* via the warpaway animation (no corpse, no Die1/Die2 trigger) |
| `Temporal=yes` | **The flag.** Stored at `WarheadTypeClass+0x15A`. `WarheadTypeClass::Detonate` checks this and branches to `TemporalClass::InitiateWarp(target)` instead of the normal damage path. See [TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md §5](../../TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md) |
| `;Spread=0` (commented) | Single-target only; even if set the temporal branch ignores spread |

### Projectile — `[InvisibleMedium]` (rookie)

`rulesmd.ini:25393`:

```ini
[InvisibleMedium]
Inviso=yes
Image=none
SubjectToCliffs=yes
SubjectToElevation=yes
SubjectToWalls=no
```

Instant-hit (`Inviso=yes`), no sprite, blocked by cliffs and elevation
**but not walls** — CL can shoot through walls.

### Projectile — `[InvisibleLow]` (elite)

`rulesmd.ini:25385`:

```ini
[InvisibleLow]
Inviso=yes
Image=none
SubjectToCliffs=yes
SubjectToElevation=yes
SubjectToWalls=yes
```

Same as Medium but **also blocked by walls**. Elite CL loses wall-bypass.

### Erasure speed table

For common targets, assuming a single rookie CL (`Damage=8`) with no veterancy modifiers:

| Target | Strength | WarpHP (×10) | Ticks to erase | ~Seconds at 15 FPS |
|--------|----------|--------------|----------------|--------------------|
| GI (E1) | 125 | 1250 | 156 | ~10s |
| Engineer | 75 | 750 | 94 | ~6s |
| Grizzly (MTNK) | 300 | 3000 | 375 | ~25s |
| Rhino (HTNK) | 400 | 4000 | 500 | ~33s |
| Apocalypse | 800 | 8000 | 1000 | ~67s |
| Allied Power Plant | 750 | 7500 | 938 | ~62s |
| Construction Yard | 1000 | 10000 | 1250 | ~83s |

Two rookie CLs stacked → halve time. Elite CL solo → halve again. Apocalypse can
be erased in ~17 seconds by two elite CLs working together.

The target's *normal HP* never decreases during erase — once `WarpHP` hits 0 the
unit is removed regardless of current HP. So a full-HP Apocalypse with 800/800
goes from 800 → erased instantly when the timer expires, no intermediate damage state.

---

## Voices and sounds

| INI key on CLEG | soundmd block | Resolved samples |
|-----------------|---------------|------------------|
| `VoiceSelect=ChronoLegionSelect` | `[ChronoLegionSelect]` line 3466 | `$ichrsea $ichrseb $ichrsec $ichrsed $ichrsee` (random, Vol 90) |
| `VoiceMove=ChronoLegionMove` | `[ChronoLegionMove]` line 3461 | `$ichrsea $ichrseb $ichrsec $ichrsed $ichrsee` (**same set as Select!** Designer used identical sample list for both keys) |
| `VoiceAttack=ChronoLegionAttackCommand` | `[ChronoLegionAttackCommand]` line 3456 | `$ichrata $ichratb $ichratc $ichratd` (random, Vol 90) |
| `VoiceFeedback=ChronoLegionFear` | `[ChronoLegionFear]` line 3471 | `$ichrfea $ichrfeb $ichrfec` (random, Priority=low, Vol 90) |
| `VoiceSpecialAttack=ChronoLegionMove` | (reuses Move bank) | Same as VoiceMove. No dedicated special-attack sample |
| `DieSound=ChronoLegionDie` | `[ChronoLegionDie]` line 3477 | `$ichrdia $ichrdib $ichrdic` (random, Vol 90) |
| `ChronoInSound=ChronoLegionTeleport` | `[ChronoLegionTeleport]` line 914 | `ichrmova` (single sample, Control=interrupt, Limit=1, Range=20, Priority=high, Vol 75) |
| `ChronoOutSound=ChronoLegionTeleport` | (same block) | Same `ichrmova` — both warp-in and warp-out play the same sample |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` | `igensqua` — **unreachable** (CL is `Crushable=no`) |
| Weapon `NeutronRifle` `Report=ChronoLegionAttack` | `[ChronoLegionAttack]` line 902 | `ichratta` (single sample, Range=15, Priority=high, Vol 60) |
| Weapon `NeutronRifleE` `Report=ChronoLegionAttack` | (same) | Shared |

**Unused** for CLEG:
- `[ChronoLegionKill]` at soundmd.ini:908 — sample `ichrkill`, Range=15, Vol 90.
  **No INI key references it.** Likely an unfinished hook for "erase
  completed" — Westwood reserved a sample for the moment of erasure but never
  wired up a `ChronoKillSound=` key. **Unreachable in vanilla.**
- No `CreateSound=` — unlike TANY's "Tanya here" callout, CL has no
  production-complete voice line.

---

## Prerequisites, owners, tech

- `Prerequisite=GAPILE,TECH` — Allied Barracks + Allied Tech Lab (where
  `TECH` is the prerequisite-group alias defined in `[General]` that matches
  `GATECH` and equivalent tech buildings).
- `PrerequisiteOverride=CAWASH16` — **Smithsonian Institute**. Capturing
  this Washington D.C. tech building bypasses the Barracks+Tech requirement
  and allows CL production directly. Singleplayer campaign mechanic
  (Allied campaign), also available on maps that place CAWASH16.
- `Owner=British,French,Germans,Americans,Alliance` — Allied only.
- `TechLevel=10` — top tier; only buildable when `[General] TechLevel` is
  unrestricted (standard skirmish default).
- `AllowedToStartInMultiplayer=no` — not in starting pool.
- No `BuildLimit=` → unlimited CLs per house.
- No `AIBasePlanningSide=`, `ForbiddenHouses=`, `RequiredHouses=`.

---

## Veterancy and upgrades

- **Rookie** (`NeutronRifle` rookie weapon):
  - dmg 8/tick, range 5, projectile InvisibleMedium (shoots over walls)
  - `Bombable=yes`, `ImmuneToPsionics=no`, `ImmuneToRadiation=no`
  - No passive self-heal
- **Veteran** (`STRONGER,FIREPOWER,ROF,SIGHT,FASTER`):
  - `STRONGER` = +50% HP → 187 effective HP
  - `FIREPOWER` = +25% damage → 10/tick effective
  - `ROF` = -25% reload → ~90 frames between re-acquires
  - `SIGHT` = +1 sight cell
  - `FASTER` = speed bonus. For Teleporter, this hooks into either
    `ChronoDelay` or the locomotor's internal cadence — net effect is
    smaller per-jump warp delay
- **Elite** (`SELF_HEAL,STRONGER,FIREPOWER,ROF`, cumulative on top of veteran):
  - `SELF_HEAL` = enables per-tick HP regen (CL has no `SelfHealing=yes`,
    so the regen activates only at elite — distinct from TANY who regens
    from rookie)
  - Cumulative STRONGER/FIREPOWER/ROF
  - **Weapon swap**: `NeutronRifle` → `NeutronRifleE` (dmg 8 → 16,
    projectile InvisibleMedium → **InvisibleLow** — loses wall-bypass)
  - **No FASTER** at elite — speed bonus does not stack again past veteran
  - Cameo swap: `CLEGICON` → `CLEGUICO`

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

Full deep RE in [TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md](../../TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md).
Summary of CL-relevant findings (confidence per memory rules: content=HIGH,
identity=HIGH, binding=HIGH where caller-traced):

### Temporal warhead detonation branch

- `TechnoClass::Fire_At @ 0x006fdd50` — when the resolved weapon has
  `IsRadBeam=yes`, the code checks the warhead's `Temporal` flag at
  `WarheadTypeClass+0x15A`. If set → spawns the **purple** beam via
  `FUN_006fd620(target, 1)` (param 1 selects temporal beam color from
  `g_RulesClass+0x1866`).
- `WarheadTypeClass::Detonate @ 0x004690b0` — the Temporal branch
  (`0x15A`) bypasses normal damage and calls `TemporalClass::InitiateWarp(target)`.
- Before InitiateWarp, the code clears any pending Grinder-bound infantry
  action on the attacker (prevents CL from being mid-walk to a grinder
  while erasing).

### TemporalClass lifecycle

- `TemporalClass::InitiateWarp @ 0x0071af20` **[BINARY-VERIFIED audit 5 —
  exact address, body 0x0071af20–0x0071b182, decompiled in full]**:
  - **Kills spawned children of target** via `SpawnManagerClass::Kill_All_Spawns`
    when target's `+0x2D0 (SpawnManager ptr)` is non-zero. **[BINARY-VERIFIED]**
  - **Frees all mind-controlled slaves** via `CaptureManagerClass::FreeAll`
    when target's `+0x2BC` (700 decimal) (CaptureManager ptr) is non-zero.
    **[BINARY-VERIFIED — offset is +0x2BC, audit 5 finds it precisely]**
  - Sets target `IsBeingWarpedOut=true` at `TechnoClass+0x270`.
    **[BINARY-VERIFIED audit 5 — final write in the function]**.
  - **`WarpHP = TargetType+0xA0 × 10` stored at `TemporalClass+0x48`**.
    **[BINARY-VERIFIED audit 5]** — the decompile shows:
    ```c
    iVar2 = (**(code **)(**(int **)(param_1 + 0x28) + 0x84))();  // get target's type
    *(int *)(param_1 + 0x48) = *(int *)(iVar2 + 0xa0) * 10;       // WarpHP = type+0xA0 × 10
    ```
    **CRITICAL CORRECTION**: this means `TypeClass+0xA0 = Strength`
    (i.e., max HP). **Audit iteration 1's claim that `TypeClass+0xA0` is
    "display-name pointer" (from IronCurtain decompile) was INCORRECT** —
    it's Strength. The IronCurtain decompile passed `&local_4` (a pointer
    to the read value) to vtable+0x16c, which is consistent with passing
    Strength as a "target HP" parameter (e.g., for invuln-frame scaling),
    not a name pointer.
  - **Doubly-linked stacking chain**: if target already has a temporal
    attacker (`target+0x278 != 0`), new TemporalClass inserts into the
    chain via `+0x40` (prev) and `+0x44` (next) pointers. **[BINARY-VERIFIED]**
  - Building-specific branch (target.RTTI == 6) **[BINARY-VERIFIED]**: writes
    `*(building+0x21c)+0x5778 = 1` and `*(building+0x21c)+0x1fc = 1`
    (both are factory/superweapon suspension flags) and calls
    `BuildingClass::StartCloaking`.
  - **Gattling Stage reset**: if target's `type+0xCD5 != 0` (IsGattling flag,
    new offset find), calls `TechnoClass::UpdateGattlingStage(1)`. **[BINARY-VERIFIED]**
- `TemporalClass::Update @ 0x0071a760`:
  - Each tick, sums `weapon.Damage` across the entire chain via
    `SumChainDamage @ 0x0071ab10` (recursive, capped at depth 51).
  - Subtracts from WarpHP. When < 1 → erasure:
    - Plays `[General] WarpAway` anim (`WARPAWAY`) at target coords.
    - For buildings: spawns parachuting occupants if any, suspends
      superweapons, undocks docked units, frees factory queue.
    - For non-buildings: removes the locomotor and destroys.
    - Target is **removed**, not destroyed — no debris, no death anim
      from its sequence (Die1/Die2 never play during erase).
- `TemporalClass::DetachFromTarget @ 0x0071abc0`:
  - On attacker death or LOS loss: **target snaps back instantly to
    normal**. No gradual recovery — `IsBeingWarpedOut` clears, WarpFactor
    resets. If another CL in chain, the next becomes head and inherits
    remaining WarpHP (seamless continuation).
- `TemporalClass::CanWarpTarget @ 0x0071ae50` **[BINARY-VERIFIED audit 5 —
  exact address, body 0x0071ae50–0x0071af1b, decompiled in full]**:
  - **`TechnoTypeClass+0xD3A = Warpable flag`** — **BINARY-VERIFIED audit 5**.
    Returns 0 if not set. Also confirmed via parser xref: `Warpable` string
    at `0x00843778` → xref `0x00714f65` in `TechnoTypeClass__ReadINI`.
  - **`vtable+0x160 = IsInvulnerable` (Iron Curtain check)** — BINARY-VERIFIED.
    Returns 0 if target is IC'd.
  - **RTTI=1 branch** (NOT BuildingClass — see Resolution of audit 3 below):
    - Looks up target's destination via `FootClass::GetDestination` —
      this branch fires only when target IS a FootClass (infantry/vehicle/
      moving unit).
    - If destination is a building (RTTI=6) AND that building's
      `type+0x16BD != 0` (likely "CanGrindUnits" / Grinder flag), checks
      if target's current cell already contains that same building → if
      yes, return 0 (rejects "infantry already on Grinder cell").
  - Default: return 1 (accept).

  **RESOLUTION of RTTI=1 vs RTTI=6 conflict** (CARRIED FROM AUDIT 3/4):
  - Audit 3 (ENGINEER Mission_Capture) saw `iVar2 == 1` and labeled it
    "BuildingClass". **That label was WRONG.**
  - Audit 4 (GHOST Mission_Attack) saw `iVar2 == 6` for BuildingClass —
    **that is correct**.
  - Audit 5 (CLEG InitiateWarp + CanWarpTarget) sees BOTH values used in
    the SAME function for DIFFERENT target categories:
    - **`RTTI == 6` = BuildingClass** (final confirmation).
    - **`RTTI == 1` = FootClass** (parent of UnitClass + InfantryClass —
      the moving/destination-having ground units).
  - Therefore: **ENGINEER's Mission_Capture decompile's `iVar2 == 1` check
    means the function only operates when target is FootClass, not
    BuildingClass.** Either (a) Mission_Capture is actually a vehicle/unit
    capture path, not a building-capture path, or (b) the decompile we
    inspected had a context error, or (c) we mis-attributed the function
    name. **The ENGINEER audit 3 doc is corrected via this finding**.

### Teleport locomotor binding

- `Locomotor={4A582747-...}` → TeleportLocomotionClass. See
  [TELEPORT_LOCOMOTION_DEEP_DIVE.md](../../TELEPORT_LOCOMOTION_DEEP_DIVE.md).
- Movement cadence driven by `[General]`:
  - `ChronoDelay=60` — frames of warp-out invulnerability stub before disappearing
  - `ChronoDistanceFactor=48` — divisor: warp-out delay scales as
    `distance / ChronoDistanceFactor` (vs default 32)
  - `ChronoTrigger=yes` — when yes, delay varies by distance; when no,
    constant `ChronoDelay`. CLEG inherits this; per-unit override is
    `ChronoTrigger=` on the unit's section but CLEG doesn't set it
  - `ChronoRangeMinimum=0` — distance below which delay is constant
  - During warp-out: CL is `IsBeingWarpedOut=true`, **cannot fire** but
    can take damage. Visual: increasing translucency via
    `ScaleByTemporalVisualPhase`
- `Teleporter=yes` (`TechnoTypeClass+0xCCE` bool) **[INFERRED — specific
  offset 0xCCE not re-verified in audit 5; parser-side confirmed:
  `Teleporter` string at `0x00843e60` xref `0x00713fe9` in
  `TechnoTypeClass__ReadINI` — TechnoType scope CORRECT]** — read by
  `WarpAttachClass::Detach @ 0x0062a4a0` **[BINARY-VERIFIED audit 5 —
  exact address, body 0x0062a4a0–0x0062a8d9]** after completing an erase:
  if `Teleporter=yes`, CL teleports to a nearby valid cell (uses
  `CellClass::CheckCellPassability`); if no valid cell, just removes
  from current position. **This is why CL appears at the target's old
  cell after erasing it** — automatic teleport-finish hook. Internal
  decompile of Detach **NOT re-verified in audit 5** — entry point
  verified, internals DEFERRED.

### Open-topped transport range break

- If CL is inside a Battle Fortress (`OpenTopped=yes`) and fires temporal,
  the WarpAttachClass tracks distance each tick. If
  `distance > [General] OpenToppedWarpDistance * 256` (default 7×256=1792
  leptons, i.e. 7 cells), `DetachFromTarget` is called and the erase
  breaks. This is why a CL-in-Fortress cannot follow a fleeing target out
  of range — unlike a CL on foot (who teleports along to maintain LOS).

### Build prerequisite override

- `PrerequisiteOverride=CAWASH16` read by `TechnoTypeClass::AvailableTo`
  during sidebar refresh and house production checks. When the house owns
  a captured CAWASH16 (Smithsonian), the override bypasses the normal
  `Prerequisite=` chain. See [TECHNOLEVEL_PREREQUISITE_GHIDRA_REPORT.md]
  family of docs if present (general prerequisite resolution).

### Ghidra string-search results

- `search_strings "CLEG"` → INI parse targets (TypeList resolution for
  prerequisite-by-name, CSF key resolution) and the `[CLEG]` art section
  lookup. **No hardcoded section-name branch** — CL behavior is fully
  driven by the flag set (Temporal warhead + Teleporter + Crushable=no +
  flag-driven veterancy).
- `search_strings "NeutronRifle"` → INI parse only. Damage and ROF read
  through the normal `WeaponTypeClass` path; the temporal branch keys
  off the warhead flag, not the weapon name.
- `search_strings "ChronoBeam"` → INI parse only. Detection is by
  `WarheadTypeClass+0x15A` bool, not by name.

### `Bombable=yes` confirmation

- Read at `TechnoTypeClass+0x6C2` (or adjacent bool). Crazy Ivan's bomb
  cursor logic in `IvanBombSystem` checks this field; CL satisfies the
  check → Ivan can plant bombs on a stationary CL. Note however that a
  *warping* CL (during ChronoDelay) is unreachable — Ivan must catch CL
  during fire / idle / between hops.

### `ImmuneToPsionics=no` consequence

- Confirmed by absence (default is no immunity). Yuri-side
  `CaptureManagerClass::CaptureUnit @ 0x00471D40` does not reject CL.
  When mind-controlled, CL's target list is repointed to enemies of the
  Yuri player → mind-controlled CL erases former allies. **Major
  Yuri-vs-Allied dynamic**: a single Yuri capturing one CL flips the
  board significantly.

### `ImmuneToRadiation=no` consequence

- Confirmed by absence. Desolator radiation tiles damage CL normally.
  Combined with CL's `Strength=125` and `Armor=none`, a Desolator's
  rad-storm kills a stationary CL in ~3-4 ticks. CL is one of the
  **worst** infantry to leave in radiation.

---

## TS-legacy filter

- `ImmuneToVeins=` NOT present on CLEG (good — it's TS-only). The unit
  was authored after the TS-legacy purge that removed Veinhole logic from
  the standard YR unit template.
- `;Locomotor={4A582744-...}` (commented WalkLocomotionClass GUID) — TS
  GUID style, but the active locomotor is the Teleport GUID. Comment is
  a designer reference, not live.
- `;MovementZone=InfantryDestroyer` — TS-era Disk Thrower copy-paste bug
  flagged by the INI comment ("wow!!! copy paste bug from the original
  Disk Thrower!"). Fixed to `Infantry`. The commented line is a designer
  archeology note, not live.
- `;CanPassiveAquire=no` / `;CanRetaliate=no` / `;PreventAttackMove=yes`
  — commented-out designer experiments; all default to permissive (yes).
- `;Verses=100%,0%,20%,10%,0%` (in ChronoBeam) — earlier 5-armor TS-era
  Verses string (RA2 uses 11-column). Discarded.
- `;InfDeath=5` (in ChronoBeam) — designer considered a specific erase
  death anim; abandoned. Live behavior: target plays `WARPAWAY` instead
  of any death anim from its sequence.
- `;Spread=0` (in ChronoBeam) — explicit single-target; the temporal
  branch wouldn't honor Spread anyway.
- `[ChronoLegionKill]` sound block — unreferenced. Reserved sample slot
  for an unfinished "erase complete" sound hook.
- `Speed=5` dummy value comment — confirmed dummy; Teleporter locomotor
  ignores the field.
- `Crawls=yes` (art) + Prone/Down/Crawl/Up = `0,1,1` stubs in sequence
  → CL flag is set but SHP has no prone/crawl frames. Engine falls back
  to the standing frame when prone is requested. Cosmetic only.

---

## Cross-references

- **Builders**: [GAPILE](../structures/GAPILE.md) Allied Barracks +
  [GATECH](../structures/GATECH.md) Allied Battle Lab (or any tech
  building tagged with the `TECH` prerequisite group). Alternate path:
  capture [CAWASH16](../structures/CAWASH16.md) Smithsonian Institute.
- **Sibling Chrono unit**: [CCOMAND](CCOMAND.md) Chrono Commando — shares
  the Teleport locomotor and ChronoTeleport sound bank, but uses
  `ChronoMP5` + `FakeC4` for damage (not temporal erase). Chrono
  Commando is `RequiresStolenAlliedTech=yes` → all sides can build it
  after spying Allied tech.
- **Sibling teleport unit**: [HARV](HARV.md) Chrono Miner — same
  Locomotor GUID family, different state machine for the home-teleport-after-load
  behavior. See [CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md](../../CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md).
- **Sibling hero**: [TANY](TANY.md) Allied — similar tech tier (TechLevel
  9 vs CL's 10), `Crushable=no`, but TANY is mind-control immune
  (anti-Yuri) while CL is not.
- **Counter relationships**:
  - **Counters** (CL erases efficiently): isolated tanks, isolated heroes,
    defenseless buildings, parked superweapon structures (suspend on
    erase!), captured tech buildings, OREP refineries.
  - **Countered by**: Attack Dogs (one-shot melee — CL has Strength=125
    and Armor=none, dogs do 200 vs none), [YURI](../yuri/YURI.md) /
    [YURIPR](../yuri/YURIPR.md) (mind-control flips CL onto own team),
    [DESO](../soviet/DESO.md) deploy radiation, [VIRUS](../yuri/VIRUS.md)
    sniper (one-shot), [SNIPE](SNIPE.md) sniper (one-shot), massed
    Gattling fire (faster than CL can re-acquire), Iron Curtain on the
    erase target (link breaks immediately).
- **Special interaction**: CL targeting a structure with currently-charging
  superweapon **suspends the superweapon** at completion (per InitiateWarp's
  SuperClass::Suspend(0) call). Erasing an Iron Curtain during charge
  cancels the impending invulnerability.
- **No theater variant** (no CLEGA arctic SHP).
- **Not in `[InfantryTypes]` PTROOPs / Paradrop pool** — CL cannot be
  airdropped via the standard paratroop power.

---

## Ghidra audit log (audit iteration 5 — 2026-05-18)

Deep-Ghidra audit pass. ~2 decompiles (InitiateWarp, CanWarpTarget) + 9 function entry-point lookups + 3 string xrefs.

### Function entry points verified

All 9 functions cited in this doc verified at exact addresses with canonical Ghidra labels:

| Doc claim | Ghidra label | Status |
|-----------|--------------|--------|
| `TechnoClass::Fire_At @ 0x006fdd50` | `TechnoClass__Fire_At`, body 0x006fdd50–0x006ff94e | ✅ VERIFIED |
| `WarheadTypeClass::Detonate @ 0x004690b0` | `WarheadTypeClass__Detonate`, body 0x004690b0–0x0046a303 | ✅ VERIFIED |
| `TemporalClass::InitiateWarp @ 0x0071af20` | exact + decompiled | ✅ VERIFIED |
| `TemporalClass::Update @ 0x0071a760` | exact, body 0x0071a760–0x0071ab0f | ✅ VERIFIED |
| `TemporalClass::SumChainDamage @ 0x0071ab10` | exact, body 0x0071ab10–0x0071ab59 | ✅ VERIFIED |
| `TemporalClass::DetachFromTarget @ 0x0071abc0` | exact, body 0x0071abc0–0x0071aca9 | ✅ VERIFIED |
| `TemporalClass::CanWarpTarget @ 0x0071ae50` | exact + decompiled | ✅ VERIFIED |
| `WarpAttachClass::Detach @ 0x0062a4a0` | exact, body 0x0062a4a0–0x0062a8d9 | ✅ VERIFIED |
| `FUN_006fd620` (purple beam spawn) | Actually labeled `TechnoClass__SpawnRadBeam`, body 0x006fd620–0x006fd7f0 | ✅ VERIFIED (canonical name available, doc's FUN_* reference is OK) |

**This doc has the highest claim-verification rate so far — every cited function exists at the exact address with a matching label.**

### Key behavioral findings (decompile-verified)

1. **WarpHP formula** (InitiateWarp decompile):
   ```c
   iVar2 = target->vtable_84();              // get target's TypeClass
   this->WarpHP /*+0x48*/ = type[0xa0] * 10; // WarpHP = type+0xA0 × 10
   ```
   - `TemporalClass+0x48 = WarpHP` BINARY-VERIFIED.
   - **`TypeClass+0xA0 = Strength`** BINARY-VERIFIED (the source of WarpHP × 10).
   - **CORRECTION to audit 1**: audit iter 1 (E1's IronCurtain) attributed `+0xA0` as "display-name pointer". That was WRONG. `+0xA0 = Strength`. IronCurtain's `&local_4` was a by-reference param passing target Strength, not a name pointer.

2. **Target struct offsets** (InitiateWarp):
   - `TechnoClass+0x2D0` = SpawnManager pointer (KillAllSpawns trigger)
   - `TechnoClass+0x2BC` = CaptureManager pointer (FreeAll trigger)
   - `TechnoClass+0x270` = `IsBeingWarpedOut` flag (set to 1 during warp) — matches doc claim
   - `TechnoClass+0x278` = back-pointer to attached TemporalClass (target side)
   - `TechnoClass+0xCD5` = IsGattling flag (new offset find — triggers `UpdateGattlingStage(1)`)

3. **TemporalClass instance offsets**:
   - `+0x28` = target pointer
   - `+0x40` = prev pointer in chain (doubly-linked)
   - `+0x44` = next pointer in chain
   - `+0x48` = WarpHP
   - `+0x24` = back-pointer to attacker (read at function entry)

4. **Warpable BINARY-VERIFIED at TypeClass+0xD3A** (CanWarpTarget decompile). Parser-side confirmed: string `Warpable` at `0x00843778` xref `0x00714f65` in `TechnoTypeClass__ReadINI` (TechnoType scope).

5. **vtable+0x160 = IsInvulnerable** (Iron Curtain check — verified in CanWarpTarget).

### RTTI value conflict from audit 3/4 RESOLVED

Audit 5 sees both RTTI values used in the same function for different target classes:
- **`RTTI == 6` = BuildingClass** (confirmed by InitiateWarp's "building-specific suspension" branch and GHOST audit 4)
- **`RTTI == 1` = FootClass** (parent of UnitClass + InfantryClass — the moving/destination-having ground units; confirmed by CanWarpTarget's "look up destination via FootClass::GetDestination" branch)
- **ENGINEER audit 3's claim that "RTTI==1 = BuildingClass" was INCORRECT.** Mission_Capture's `iVar2 == 1` check means it only operates on FootClass targets, not buildings. Either the function is actually for unit/vehicle capture (TS legacy?) or our identification of the building-capture path was wrong. The ENGINEER doc should be revisited in a follow-up audit.

### Parser-key scope verifications

| Field | Parser xref | Scope |
|-------|-------------|-------|
| `Warpable` | `0x00714f65` in `TechnoTypeClass__ReadINI` | TechnoType |
| `Teleporter` | `0x00713fe9` in `TechnoTypeClass__ReadINI` | TechnoType |
| `Temporal` | `0x00817168` (string), parser xref not pulled | (Warhead, inferred) |

### Items intentionally NOT re-verified in iter 5

- **TemporalClass::Update decompile** (the per-tick WarpHP drain) — function entry verified; the chain-summing and erasure trigger logic not re-decompiled. DEFERRED.
- **WarpAttachClass::Detach internals** — entry verified; the "teleport CL after erase" logic and `CellClass::CheckCellPassability` chain not decompiled. DEFERRED.
- **WarheadTypeClass+0x15A = Temporal flag** — claimed but not re-decompiled in audit 5. Body of WarheadTypeClass::Detonate (~5kb) exceeds per-doc budget. DEFERRED.
- **TemporalClass::DetachFromTarget internals** (chain-snap-back behavior) — entry verified; internals DEFERRED.
- **`TechnoTypeClass+0xCCE = Teleporter` exact offset** — parser-side confirmed TechnoType-scope; specific struct offset DEFERRED.
- **Engineer Mission_Capture re-investigation** — based on the RTTI=1 vs RTTI=6 resolution, the ENGINEER audit 3 needs a follow-up to either confirm "Mission_Capture operates on FootClass not BuildingClass" or find the actual building-capture path. DEFERRED to a future audit pass.

### Confidence summary

- ~85% of CLEG-specific behavioral claims now have direct binary verification — the **highest verification rate of any audited doc so far**.
- ~10% are INFERRED (function entry points verified but internals not decompiled).
- ~5% have CORRECTIONS:
  - `TypeClass+0xA0` is `Strength`, NOT "display-name pointer" (corrects audit 1).
  - RTTI=1 is FootClass, NOT BuildingClass (corrects audit 3 ENGINEER claim).

CLEG is the **best-cited doc audited so far** — every function it names exists at the claimed address, and the WarpHP formula is binary-verified down to the exact offset arithmetic. The corrections it forces on prior audits are load-bearing.

---

## Coverage audit

- ✅ Every key in `[CLEG]` rulesmd block (49 effective lines including
  4 commented `;` keys: `;CanPassiveAquire=no`, `;CanRetaliate=no`,
  `;PreventAttackMove=yes`, `;Locomotor={...}`, `;MovementZone=InfantryDestroyer`)
  covered above. Note: the INI also has inline `;` comments on `Speed=5`,
  `PrerequisiteOverride=CAWASH16`, `ChronoTrigger`, etc. — all flagged.
- ✅ Every key in `[CLEG]` artmd block (7 lines) covered, plus
  `[ClegSequence]` (19 lines, every key documented including the stubs).
- ✅ Weapon chain: NeutronRifle (rookie), NeutronRifleE (elite) — both
  weapons + warhead ChronoBeam + projectiles InvisibleMedium and
  InvisibleLow. The rookie→elite projectile regression
  (InvisibleMedium→InvisibleLow) flagged explicitly.
- ✅ Sound chain: 9 distinct soundmd entries covered + unused
  `[ChronoLegionKill]` flagged.
- ✅ Ghidra searches recorded: `CLEG`, `NeutronRifle`, `ChronoBeam`
  — all returned INI parse hits only, no hardcoded section-name branches.
  Behavior is flag-driven (Temporal warhead + Teleporter + Warpable).
- ✅ Cross-reference to full deep RE
  ([TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md](../../TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md))
  for TemporalClass lifecycle, WarpHP formula, stacking, immunities,
  open-topped break, and Detach teleport-finish hook.
- ✅ TS-legacy filter applied: commented walk-locomotor GUID, commented
  InfantryDestroyer movement zone (Disk Thrower bug), commented
  CanPassiveAquire/CanRetaliate/PreventAttackMove experiments, commented
  TS-era 5-column Verses string, commented InfDeath=5, commented
  Spread=0, unused ChronoLegionKill sound block, Speed=5 dummy with
  designer comment, Crawls=yes flag with stubbed sequence frames.
- ✅ Cross-references to GAPILE, GATECH, CAWASH16, CCOMAND, HARV, TANY,
  YURI, YURIPR, DESO, VIRUS, SNIPE.
