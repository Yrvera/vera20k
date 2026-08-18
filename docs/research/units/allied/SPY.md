# Spy (SPY)
Side: Allied | Category: Infantry | Image alias: `[SPY]` (no `Image=` redirect — own SHP)

The Allied Spy. $1000 from Barracks (with Battle Lab prereq), `PermaDisguise=yes`
infiltrator. Has no real combat weapon — `Primary=MakeupKit` is an inviso "camera"
that, when fired at an enemy infantry, swaps the spy's permanent disguise to that
infantry type. When the spy enters an enemy building (Mission Enter), the building
runs **one** of seven mutually-exclusive infiltration effects depending on the
building's type (radar reset, power blackout, stolen tech, super-weapon timer
reset, money steal, war-factory or barracks unlock — first-match-wins, checked in
that order). Inside an [HTK] IFV the spy converts the turret to the **Disguise
Engineer** weapon (`IFVMode=2` — radar-jam / radar-disable beam in YR's IFV
table). Cannot be a starting unit.

Authoritative deep RE for the infiltration dispatch:
[SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md](../../SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md).

---

## rulesmd.ini — `[SPY]` section

Verbatim from `ini/rulesmd.ini:3973`:

```ini
[SPY]
UIName=Name:SPY
Name=Spy ;locked
Category=Soldier
Prerequisite=GAPILE,GATECH
CrushSound=InfantrySquish
LeadershipRating=3
Strength=100
Primary=MakeupKit ; virtual weapon that picks disguise
CanPassiveAquire=no ; Won't try to pick up own targets
CanRetaliate=no; Won't fire back when hit
Armor=flak
TechLevel=5
Agent=yes
Infiltrate=yes
CanDisguise=yes; I appear differently on other people's computers
PermaDisguise=yes; and I appear that way always (Mirage Tank will be Can but not Perma)
Sight=9
Speed=4
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=1000
Soylent=500
Pip=blue
Points=5
VoiceSelect=SpySelect
VoiceMove=SpyMove
VoiceAttack=SpyAttackCommand
VoiceFeedback=SpyFear
VoiceSpecialAttack=SpySpecialAttack
DieSound=SpyDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=0	; This value MUST be 0 for all building addons
SpecialThreatValue=1
PreventAttackMove=yes
IFVMode=2
Trainable=no
StupidHunt=yes ;this guy can't handle a hunt command, so he should just run towards the player
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:SPY` | CSF-string key resolving to "Spy" |
| `Name=Spy ;locked` | Internal short name; `;locked` is a build-only comment marker (engine ignores) |
| `Category=Soldier` | Pip group + AI threat grouping (infantry) |
| `Prerequisite=GAPILE,GATECH` | **Both** Barracks and Battle Lab required — gating to tech-5 minimum even on accelerated builds |
| `CrushSound=InfantrySquish` | Crush sound when run over (sound `igensqua`) |
| `LeadershipRating=3` | Leadership-rating veterancy gain modifier — moot here (`Trainable=no`) |
| `Strength=100` | HP — same as GI |
| `Primary=MakeupKit` | **Virtual weapon** — inviso single-shot that, on hit with `MakesDisguise=yes` warhead, copies the target's type into the spy's PermaDisguise slot. **Range=-2** = infinite, **FireOnce=yes** = one shot per command, **Damage=1** but irrelevant. Comment in INI: `virtual weapon that picks disguise`. See "Weapons" section |
| `CanPassiveAquire=no` | **Disables auto-target acquisition** — spy won't fire MakeupKit at infantry it walks past. Disguise change is always a player-issued attack |
| `CanRetaliate=no` | **Disables damage-response retaliation** — when shot, the spy doesn't fire back (and wouldn't gain anything from doing so) |
| `Armor=flak` | Damage type column 1 — standard infantry armor; flak warheads do bonus damage |
| `TechLevel=5` | Buildable at game tech-level 5+; in skirmish always available once Barracks+Battle Lab are up |
| `Agent=yes` | **Behavior flag** — marks unit as a spy/infiltrator. When this unit's Mission Enter completes on an enemy building, the building runs `BuildingClass::OnSpyInfiltrate` (the 7-branch dispatch — see "Hardcoded Behavior") instead of the engineer's capture path. Causes the right-click cursor on enemy buildings to be the "Enter" cursor with spy semantics |
| `Infiltrate=yes` | Allows entering buildings hostile to the unit's house (engineer-style Mission Enter against enemies). Without this, Enter-mission can only target friendly or empty buildings |
| `CanDisguise=yes` | This unit is allowed to wear a disguise — sets the `IsDisguised` capability bit. Other players see the disguise sprite; the controlling player always sees the real spy SHP |
| `PermaDisguise=yes` | Disguise is **never automatically dropped** — only changed by another MakeupKit fire or by being mind-controlled. Contrast Mirage Tank (`CanDisguise=yes` only, drops on fire/move) |
| `Sight=9` | Reveal radius — large (1 more than GI's 8, used to spot defenses for safe approach) |
| `Speed=4` | Same as GI / engineer |
| `Owner=British,French,Germans,Americans,Alliance` | Allied countries only — **no Soviet/Yuri analogue** (there is no Soviet spy unit type) |
| `AllowedToStartInMultiplayer=no` | Cannot appear in starting-unit complement; must be produced |
| `Cost=1000` | Credits — twice the engineer's $500 |
| `Soylent=500` | Grinder refund (Yuri only — captured spy fed to Grinder gives 50%) |
| `Pip=blue` | Cargo-passenger pip color when loaded in transport |
| `Points=5` | Kill score for the player who kills this unit |
| `VoiceSelect=SpySelect` | Selection voice bank — `$ispysea/b/c/d` |
| `VoiceMove=SpyMove` | Move-order voice bank — `$ispyata $ispymob $ispymoc $ispymod $ispymoe` |
| `VoiceAttack=SpyAttackCommand` | Attack-order voice — `$ispyatb` (single line, used when ordered to disguise via MakeupKit) |
| `VoiceFeedback=SpyFear` | Fear/panic voice — `$ispyfea $ispyfeb` |
| `VoiceSpecialAttack=SpySpecialAttack` | Played on building infiltration order — `$ispyatd` (global broadcast — `Type=global`, plays to all players) |
| `DieSound=SpyDie` | Death sound — `$ispydia/b/c` |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — same as all infantry |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `ThreatPosed=0` | AI does not pick spies as priority targets (cannot fight back so AI threat = 0) |
| `SpecialThreatValue=1` | Bumps AI self-threat estimate so the spy "wants" infiltrate targets at max weight |
| `PreventAttackMove=yes` | **Suppresses Attack-Move action** — Attack-Move/Force-Move hotkeys behave as plain Move (no shoot-as-walk). Critical because MakeupKit changes disguise, not damage |
| `IFVMode=2` | IFV gunner-table index 2 → **Disguise Engineer** weapon when this passenger boards an [HTK]. In stock YR `[HTK].WeaponN`, gunner index 2 is `IFVDisguiseDef/IFVDisguiseElite` (a radar-jam beam in vanilla); when garrisoned the IFV chassis takes on the disguise weapon and the spy's own MakeupKit is unavailable |
| `Trainable=no` | **Cannot gain veterancy** — Veteran/Elite cameos never appear. AltCameo in artmd is defensively present but unused |
| `StupidHunt=yes` | AI "hunt" mission falls back to a simple charge-toward-player — the spy has no real combat behavior to drive a proper hunt |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `yes` (prone while crawling enabled)
- `Bombable=` — defaults to `false`; spy is not in the explicit Bombable list (only `[E1]` declares it) — Crazy Ivan can still attach a bomb via the Bomb mission
- `Crushable=` — defaults to `yes` for infantry; not overridden
- `ImmuneToVeins=` — not set; spy is technically vein-vulnerable, but veins are TS-only terrain
- `ImmuneToPsionics=` — defaults to `no`; **spy can be mind-controlled** (and being mind-controlled while disguised drops PermaDisguise back to the default `AlliedDisguise`/`SovietDisguise`/`ThirdDisguise` of the new controller's side)
- `Deployer=` — defaults to `no`; spy has no deploy command
- `Occupier=` — defaults to `no`; spy **cannot garrison** civilian buildings (verify-cursor at garrison-able buildings only shows the infiltrate cursor, not a garrison cursor)
- `BombSight=` — not set; spy does not reveal Ivan bombs (only engineer/Tanya do)
- `C4=` — not set; spy cannot demo-charge bridge huts or buildings
- `Assaulter=` — not set; spy cannot clear garrisoned civilian buildings (only `[GHOST]/[TANY]` do)
- `Spawned=` — not set
- `Naval=` — not set

---

## artmd.ini — `[SPY]` section

`ini/artmd.ini:131`:

```ini
[SPY]
Cameo=SPYICON
Sequence=SpySequence
Crawls=yes
Remapable=yes
FireUp=1
```

| Key | Meaning |
|-----|---------|
| `Cameo=SPYICON` | Sidebar build icon (SHP) — note **the disguise replaces only the in-world sprite, not the cameo** |
| `Sequence=SpySequence` | Reference to `[SpySequence]` block (frame layout) |
| `Crawls=yes` | Sets the prone-while-walking enabled flag on the type |
| `Remapable=yes` | House remap palette applied to colored pixels |
| `FireUp=1` | Bullet-spawn frame within the firing sequence (MakeupKit "fires" at frame 1 of the FireUp track — i.e., the camera flash on the very first up-frame) |

Note this section is **missing `PrimaryFireFLH=` / `SecondaryFireFLH=`**. Since
MakeupKit is `Projectile=InvisibleAll` and `RevealOnFire=no`, no muzzle flash or
projectile sprite is needed; the visible "shot" is the `Report=SpyAttack` sound
(`vmirat2a`) only.

Also missing `AltCameo=` despite many other infantry having one — consistent with
`Trainable=no` (no Elite cameo state to display).

### Referenced sequence — `[SpySequence]`

`artmd.ini:13948`:

```ini
[SpySequence]
Ready=0,1,1
Guard=0,1,1
Prone=86,1,6
Walk=8,6,6
FireUp=0,1,1
Down=164,2,2
Crawl=86,6,6
Up=180,2,2
FireProne=86,1,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Die1=134,15,0
Die2=149,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
;Cheer=196,8,0,W
Cheer=196,8,0,E
Panic=8,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle (1 frame × 1 facing) | Default stance — disguise is rendered over this |
| `Guard=0,1,1` | Same as Ready | Guard mission idle |
| `Prone=86,1,6` | Prone-down still (1 frame × 6 facings) | Slim coverage — spy rarely uses prone |
| `Walk=8,6,6` | Walk cycle (6 frames × 6 facings) | Note **6 facings** not 8 — spy art omits the back-quarters and reuses by mirror |
| `FireUp=0,1,1` | Single frame at offset 0 — MakeupKit "fire" is just the camera-click sound; no actual fire pose |
| `Down=164,2,2` | Get-down (2 frames × 2 facings) | Transition into prone |
| `Crawl=86,6,6` | Crawl cycle reuses prone frames | Standard infantry crawl |
| `Up=180,2,2` | Get-up (2 frames × 2 facings) | Transition out of prone |
| `FireProne=86,1,6` | Prone-fire reuses prone frames | No separate prone-fire pose |
| `Idle1=56,15,0,S` | Idle animation 1 — 15 frames, S-facing only | The "look around suspiciously" anim |
| `Idle2=71,15,0,E` | Idle animation 2 — 15 frames, E-facing only | The "check pocket watch" anim |
| `Die1=134,15,0` | Death animation 1 (15 frames, 0 facings = use base only) | |
| `Die2=149,15,0` | Death animation 2 | |
| `Die3=0,1,1` | Stub — falls back to Ready frame (unused death variants) | |
| `Die4=0,1,1` | Stub | |
| `Die5=0,1,1` | Stub | |
| `Cheer=196,8,0,E` | Cheer animation (8 frames, single E-facing) — the commented-out `Cheer=...,W` was the prior west-facing version |
| `Panic=8,6,6` | Panic = reuse of Walk frames | No dedicated panic-flee art |

No `Paradrop=` entry — spy **cannot** be paradropped (the engine fallback for missing
`Paradrop=` is "no rendered paradrop frame"; even if the spy were placed in a paradrop,
no special art plays).

No `Deploy*` entries — spy cannot deploy.

---

## Weapons

### Primary — `[MakeupKit]`

`rulesmd.ini:24140`:

```ini
[MakeupKit]
Damage=1
ROF=100
Range=-2 ; infinite
Projectile=InvisibleAll
Speed=100
FireOnce=yes ; Firing clears TarCom so only one shot is fired
Warhead=Snapshot
RevealOnFire=no ; Doesn't clear shroud when fired
Report=SpyAttack
FireInTransport=no;can't fire out of the BattleFortress
```

| Key | Meaning |
|-----|---------|
| `Damage=1` | Nominal — `Snapshot` warhead Verses produce 1-damage minimum but the point is the `MakesDisguise=yes` side-effect, not damage |
| `ROF=100` | Cooldown between disguise changes — 100 frames (~6.7s at 15fps). Discourages mash-spamming |
| `Range=-2` | Engine sentinel for **infinite range** — spy can change disguise to any visible infantry on the map (line of sight not required by weapon code; targeting requires the target be selectable, which requires reveal) |
| `Projectile=InvisibleAll` | `[InvisibleAll]` — `Inviso=yes Image=none AA=yes AG=yes`. No flight time, no visible projectile, instant resolution. AA+AG so the cursor doesn't pre-filter target type by air/ground |
| `Speed=100` | Irrelevant for inviso instant-resolution |
| `FireOnce=yes` | After one shot, TarCom is cleared — spy returns to idle, doesn't keep "firing" the camera at the same target |
| `Warhead=Snapshot` | Carries `MakesDisguise=yes` (see warhead below) |
| `RevealOnFire=no` | The shot does **not** clear shroud on the firing spy — important because spy uses disguise to remain unobtrusive |
| `Report=SpyAttack` | Sound `vmirat2a` ("camera shutter" SFX) played once. Same report used by the Mirage Tank disguise switch |
| `FireInTransport=no` | Cannot fire from inside [FV] Battle Fortress (no usable arc / would expose the BF as carrying a spy) |

### Primary's Warhead — `[Snapshot]`

`rulesmd.ini:27473`:

```ini
[Snapshot]
Verses=100%,100%,100%,0%,0%,0%,0%,0%,0%,100%,100%
MakesDisguise=yes
```

| Key | Meaning |
|-----|---------|
| `Verses=100%,100%,100%,0%,0%,0%,0%,0%,0%,100%,100%` | Eleven-column armor row matching the global armor order (`none, flak, plate, light, medium, heavy, wood, steel, concrete, special_1, special_2`). 100% on infantry-class armors (none/flak/plate) and the two specials; 0% on all vehicle/structure armors. **The 0% is what restricts the attack cursor to infantry only** — engine refuses to fire when projected damage is 0 |
| `MakesDisguise=yes` | **Engine flag** — on hit, copies the target's `TechnoTypeClass*` into the firing unit's `DisguisedAs` slot and (because `PermaDisguise=yes` on SPY) freezes that disguise until next MakeupKit fire or mind-control event |

### Projectile — `[InvisibleAll]`

`rulesmd.ini:25406`:

```ini
[InvisibleAll]
Inviso=yes
Image=none
AA=yes
AG=yes
;AN=yes
;AS=yes
```

Standard "no-sprite instant-hit" projectile. AN/AS lines commented out (Naval/Submerged
targeting) — irrelevant for MakeupKit since you can't disguise as a submarine anyway.

### No Secondary

Spy has **no `Secondary=` key**. Compare engineer (`Secondary=VirtualScanner`).
Effect: no scanner probe; spy's guard/AI behaviour cannot extend its "interest"
beyond MakeupKit's range, but since MakeupKit is `Range=-2` (infinite), this
doesn't matter in practice.

### Sister weapon for reference — `[TankMakeupKit]`

`rulesmd.ini:24152` — used by Mirage Tank, not the spy. Mirrors MakeupKit but
uses `[TankSnapshot]` (Verses 0%,0%,0%,1%,1%,1%,0%... — only vehicles) and
`TerrainFire=yes` (so the cursor also lights up on trees). Documented here for
contrast — spy targets infantry, mirage targets vehicles/terrain.

---

## Disguise system — defaults and runtime rules

The spy starts with a side-default disguise. From `rulesmd.ini:274`:

```ini
;*** Spy stuff ***
AlliedDisguise=E1
SovietDisguise=E2 ; these are the defaults for the spy if a MakeupKit hasn't been used
ThirdDisguise=INIT
```

| Key | Default | Meaning |
|-----|---------|---------|
| `AlliedDisguise=E1` | G.I. | Spy's appearance to **Allied victims** before any MakeupKit fire |
| `SovietDisguise=E2` | Conscript | Spy's appearance to **Soviet victims** before any MakeupKit fire |
| `ThirdDisguise=INIT` | Yuri Initiate | Spy's appearance to **Yuri victims** before any MakeupKit fire |

Once MakeupKit lands on an enemy infantry, the spy adopts **that specific TechnoTypeClass**
as the disguise (e.g. shoot a SHK Tesla Trooper, you appear as a Tesla Trooper to
everyone — not split per-viewer-side). This single-disguise-shown-to-all behaviour
overrides the per-side default once the spy has fired the camera.

Other relevant rulesmd values, from `rulesmd.ini:277`:

```ini
SpyPowerBlackout=1000 ; Frame time a spy shuts down power for (900 = 1 minute)
SpyMoneyStealPercent=.5 ; Percent of total money you take with a spy
```

| Key | Default | Used by |
|-----|---------|---------|
| `SpyPowerBlackout=1000` | 1000 frames (~66.7s @ 15fps) | BRANCH 2 (Power Plant infiltrate) — duration of the victim's "power offline" state |
| `SpyMoneyStealPercent=.5` | 0.5 (50%) | BRANCH 5 (Refinery infiltrate) — fraction of victim's *current cash balance* transferred to spy's owner |

And `rulesmd.ini:281`:

```ini
AttackCursorOnDisguise=yes ;gs If yes, the mouse will be an attack cursor on a disguised unit as if he is not disguised.
                                ;If no, you will still get an attack cursor on a fake-blinking Mirage and a spy _always_
```

| Key | Default | Effect |
|-----|---------|--------|
| `AttackCursorOnDisguise=yes` | yes | If `yes`, when an enemy unit hovers over a disguised spy, the attack cursor appears (the disguise visually fools the player but doesn't gate combat). If `no`, the disguise also fools the cursor — but the Mirage Tank's "fake blink" and the spy still always show the attack cursor regardless |

And `rulesmd.ini:286`:

```ini
InfantryBlinkDisguiseTime=20 ;must be bigger than 8 to be reliable, can be 0 to prevent infantry from detecting mirages; this is a logic blink so others will join in the shooting
```

| Key | Default | Effect |
|-----|---------|--------|
| `InfantryBlinkDisguiseTime=20` | 20 frames (~1.3s) | When an enemy infantry passes near a disguised spy/mirage, the disguise "blinks" — briefly shows the true appearance — for this many frames so that other nearby units get a chance to see and engage. Set to 0 to disable. Comment "must be bigger than 8 to be reliable" is the engine designers' note that values <8 may be skipped by the periodic detect tick |

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement

```ini
[SpySelect]                  ; soundmd.ini:4012
Sounds= $ispysea $ispyseb $ispysec $ispysed
Control= random interrupt
Volume=90

[SpyMove]                    ; soundmd.ini:4007
Sounds= $ispyata $ispymob $ispymoc $ispymod $ispymoe
Control= random interrupt
Volume=90

[SpyAttackCommand]           ; soundmd.ini:4004
Sounds= $ispyatb
```

`SpyAttackCommand` plays when the player orders the spy to MakeupKit-shoot a target.
Single line `$ispyatb` ("Targeting and acquiring…" or equivalent).

### Special-attack (building infiltrate) and fear / death

```ini
[SpySpecialAttack]           ; soundmd.ini:4032
Sounds=$ispyatd
Control=random interrupt
Type=global
Volume=90

[SpyFear]                    ; soundmd.ini:4017
Sounds= $ispyfea $ispyfeb
Control= random interrupt
Volume=90

[SpyDie]                     ; soundmd.ini:4022
Sounds= $ispydia $ispydib $ispydic
Control= random interrupt
```

`Type=global` on `SpySpecialAttack` means **all players hear it** when a spy is
ordered to infiltrate a building (alerts the victim before the spy reaches the
door — but the actual on-infiltrate EVA is played from `OnSpyInfiltrate`, see
"Hardcoded Behavior").

### Creation (training-complete) voice — referenced but not on the type

```ini
[SpyCreated]                 ; soundmd.ini:4026
Sounds= $ispyatc
Type= global
Priority=critical
MinVolume=45
```

`SpyCreated` exists but is **not wired to the [SPY] section** (no `CreateSound=`).
It is invoked only by EVA / mission scripting (e.g. campaign briefings). The
SPY does **not** speak on completion of build — only the standard EVA "Unit
ready" plays.

### Weapon report sound — `[SpyAttack]`

```ini
[SpyAttack]                  ; soundmd.ini:1130
Sounds=vmirat2a
Range=15
Volume=65
```

`vmirat2a` is the camera-shutter SFX. **Shared with the Mirage Tank** disguise
switch (whose weapon report is `MirageTankDisguise` → different bank but same
audio family). Range=15 means audible within 15 cells of the firing spy.

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `GAPILE,GATECH` | Both Allied Barracks AND Allied Battle Lab required. Soviet/Yuri equivalents do not satisfy because the prerequisite parser matches by literal type, not by abstract role — and spy is Allied-only anyway |
| `Owner=` | `British,French,Germans,Americans,Alliance` | Allied countries only; no `ForbiddenHouses=` needed because no Soviet/Yuri houses are in `Owner=` to filter |
| `TechLevel=` | `5` | Skirmish/MP techlevel cap; cap of 5 is mid-game |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=1000` | $1000 | 2× engineer cost |
| `Soylent=500` | $500 refund | Grinder (Yuri) only |
| `Points=5` | 5 | Kill-score contribution |

No `PrerequisiteOverride=`, no `BuildLimit=` (spy is unlimited build, subject only
to cost and queue). No `RequiresStolenXxxTech=` — spy is itself one of the units
that **enables** other side's stolen-tech units; it doesn't itself require any.

---

## Veterancy

| Field | Value | Notes |
|-------|-------|-------|
| `Trainable=no` | — | **Disables all XP gain.** No Veteran/Elite promotion; AltCameo absent from artmd; no `VeteranAbilities=` or `EliteAbilities=` keys (would be ignored if present) |

---

## Hardcoded behavior — Ghidra-verified

### 1. Building infiltration dispatch — `BuildingClass::OnSpyInfiltrate` (0x004571E0) [BINARY-VERIFIED]

When the spy finishes Mission Enter into an enemy building, the engine calls
`BuildingClass::OnSpyInfiltrate(this=victim_building)` with the spy's owner
HouseClass* on the stack. This is the master dispatch. Full RE in
[SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md](../../SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md);
summary here.

**Caller chain [BINARY-VERIFIED audit 6]:** `OnSpyInfiltrate` is invoked from
`InfantryClass::PerCellProcess` @ `0x00519630` — the same ~5KB function that
holds the engineer-capture / Mission Enter dispatch logic (consistent with
audit 3's finding that "Mission_Enter logic" lives inside PerCellProcess body,
not at any standalone Mission_Enter entry point).

Strict priority order — **first match wins**, all subsequent checks skipped:

| Pri | Condition (on victim `BuildingTypeClass`) | Effect |
|-----|--------------------------------------------|--------|
| 1 | Same owner (`Owner == spy_owner`) [BINARY-VERIFIED] | Early return — no effect (can't spy own buildings) |
| 2 | `Radar=yes` (TypeClass+0x16A4) [BINARY-VERIFIED] | **Shroud reset** — `FUN_0050BD10(victim)` wrapper checks `HouseClass+0x577A` (LowPowerState) [BINARY-VERIFIED] and only then calls `MapClass::RestoreShroud(victim)` @ `0x00577AB0` [BINARY-VERIFIED]; victim re-shrouded except current vision sources. EVA: victim hears `EVA_RadarSabotaged`, attacker hears `EVA_BuildingInfRadarSabotaged`. **Skipped if victim is in LowPower** (radar already offline) |
| 3 | `Power > 0` (TypeClass+0xEE0) [BINARY-VERIFIED] | **Power blackout** — `HouseClass::SpyPowerSabotage(victim, RulesClass+0xD64)` @ `0x0050BC90` [BINARY-VERIFIED]. Default 1000 frames. Writes victim `+0x5778=1` (PowerBlackedOut), `+0x2A4` (BlackoutStartFrame), `+0x2AC` (BlackoutDuration) [BINARY-VERIFIED]. Disables all the victim's powered buildings until timer expires. EVA: victim `EVA_PowerSabotaged`, attacker `EVA_BuildingInfiltrated` + `EVA_EnemyBasePoweredDown` |
| 4 | Type ∈ `RulesClass.BuildTech[]` (default `NATECH,GATECH,YATECH`) [BINARY-VERIFIED — Rules+0x920 data ptr, +0x92C count] | **Stolen tech** — sets `HouseClass+0x2BE/BD/BC` (StolenAllied/Soviet/Third) based on tech building's `AIBasePlanningSide` (TechnoTypeClass+0x6D0) [BINARY-VERIFIED]; flips `ProductionChanged=1` (HouseClass+0x1FC) [BINARY-VERIFIED]. Units with `RequiresStolenXxxTech=yes` become available in sidebar. EVA: victim `EVA_TechnologyStolen`, attacker `EVA_BuildingInfiltrated` + `EVA_NewTechnologyAcquired` |
| 5 | `SuperWeapon != -1` (TypeClass+0x16F0) [BINARY-VERIFIED] | **Reset SW timer** — `OnSpyWeaponInfiltrate(superClass)` @ `0x006CE0B0` [BINARY-VERIFIED]. Clears charge anim (SuperClass+0x68), clears `IsCharged` flag (SuperClass+0x6C), resets RechargeStartFrame (SuperClass+0x30) [all BINARY-VERIFIED]. EVA: attacker `EVA_BuildingInfiltrated` (no victim-specific) |
| 6 | `Storage > 0` (TypeClass+0x800) [BINARY-VERIFIED — TechnoType scope] | **Money steal** — transfers `victim_balance * RulesClass.SpyMoneyStealPercent` (Rules+0xD68, default 50%) [BINARY-VERIFIED] from victim to attacker via `Spend_Money` @ `0x004F9790` / `Add_Credits` @ `0x004F9950` [BINARY-VERIFIED]. Credits stored at HouseClass+0x30C. EVA: victim `EVA_CashStolen`, attacker `EVA_BuildingInfCashStolen` |
| 7 | `Factory=UnitType` (TypeClass+0xEB8 == 0x28) [BINARY-VERIFIED] | **War-factory spy** — sets `HouseClass+0x2C0 (SpiedWarFactory)=1`, `ProductionChanged=1`, triggers sidebar repaint (writes `DAT_00884B8E = 1`) [BINARY-VERIFIED]. Vehicles built afterwards arrive at Veteran rank. EVA: victim `EVA_TechnologyStolen`, attacker `EVA_BuildingInfiltrated` + `EVA_NewTechnologyAcquired` |
| 8 | `Factory=InfantryType` (TypeClass+0xEB8 == 0x10) [BINARY-VERIFIED] | **Barracks spy** — sets `HouseClass+0x2BF (SpiedBarracks)=1`, plus same `ProductionChanged` / sidebar repaint. Infantry built afterwards arrive at Veteran rank. EVA: same set as War-factory spy |
| 9 | None of the above | No game effect (spy is still consumed by the entry) |

**The spy is consumed in all branches** — at end of `OnSpyInfiltrate` the building
is reassigned Mission Guard via its vtable+0x124 with arg 2 [BINARY-VERIFIED], and the spy object
is removed by the upstream Mission Enter logic (separate from this dispatch).

**All seven branches are live in YR — none are TS-gated.** Verified by full
decompile of `0x004571E0` (audit 6) and cross-referenced in [SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md](../../SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md).

### 2. Disguise rendering split — `TechnoClass::IsDisguised_Getter` (0x0041C020) [BINARY-VERIFIED]

`TechnoClass__IsDisguised_Getter` at `0x0041C020` exists at the claimed address
(8-byte body, xref count 4 — all from vtable data tables at `007e236c`,
`007e3f84`, `007e8d5c`, `007f4a28`). [BINARY-VERIFIED audit 6]

**[INCORRECT — corrected audit 6]:** Doc previously claimed this was "the
runtime predicate that selects whether to draw the real SHP or the disguised
TechnoType's SHP" with "per-viewer-side, not per-frame" semantics. **The function is just a
1-byte flag getter** — its full body is `return *(undefined1*)(this + 0x1D8);`. It returns the
`IsDisguised` boolean on the TechnoClass instance. The per-viewer-side draw
decision happens elsewhere (in the draw-pipeline / sight-check code, not in
this getter). [BINARY-VERIFIED — TechnoClass+0x1D8 = IsDisguised flag, NEW
offset finding audit 6.]

### 3. Disguise detection — periodic blink

`InfantryBlinkDisguiseTime=20` drives a per-tick check: when any enemy unit is
within the spy's cell or adjacent cells, the disguise renders as the real spy
for `InfantryBlinkDisguiseTime` frames before reverting. Controlled by the same
type-bits used for Mirage Tank "fake blink." Setting the value to 0 disables
the blink entirely (would make spies effectively invincible to detection).

### 4. MakeupKit disguise change — engine-level `MakesDisguise` handler

Triggered by the `Snapshot` warhead's `MakesDisguise=yes` flag on hit. The
engine copies the **TargetTechno's TechnoTypeClass** pointer into the firing
unit's disguise slot and (since SPY has `PermaDisguise=yes`) freezes it.

Mind-control resets the disguise to the controller's side default
(`AlliedDisguise`/`SovietDisguise`/`ThirdDisguise`) for the duration of control —
the controlled spy temporarily appears as a generic basic infantry of the new
controller's side.

### 5. IFV transformation — `IFVMode=2`

The [HTK] Multi-Gunner IFV reads `IFVMode=N` from each garrisoned infantry passenger
to index into the IFV's `WeaponN`/`ElitePassengerWeaponN` lookup table. SPY's
`IFVMode=2` corresponds to the **Disguise Engineer / radar-jammer** weapon in
stock YR (the third weapon slot of [HTK]). The IFV chassis takes on this weapon
while the spy is loaded; the spy's own MakeupKit is not callable.

### 6. Agent + Infiltrate semantics

`Agent=yes` is the engine flag that swaps the destination of Mission Enter
against an enemy building from "Engineer capture path" to "Spy infiltration
path" (i.e. calls `OnSpyInfiltrate` instead of `BuildingClass::AssignToCapture`).
`Infiltrate=yes` is the prerequisite that permits Mission Enter against hostile
houses in the first place (without it, Enter resolves only on friendly /
unaffiliated structures).

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("AlliedDisguise\|SovietDisguise\|ThirdDisguise\|DefaultMirageDisguises")` | 4 strings at `0083C6A0/0083C690/0083C680/0083B488`; sole xref from `RulesClass__ReadGeneral @ 0066FBA1` (data) — confirms these are RulesClass INI keys read once at boot, not per-unit |
| `search_strings("MakeupKit")` | 0 matches — confirms `MakeupKit` is **not a hardcoded string** in the binary. Engine resolves the weapon by INI section name lookup, not by hardcoded reference; the name is interchangeable as long as a unit's `Primary=` points to a weapon with `Warhead=Snapshot` (or any `MakesDisguise=yes` warhead) |
| `search_functions_enhanced(name_pattern="Spy")` | 8 hits: `AircraftClass__Mission_SpyPlane`, `DrawOneSpySatellite`, `DrawSpySatelliteVision`, `HasSpySatelliteUpdate`, `BuildingClass__OnSpyInfiltrate @ 0x004571E0`, `HouseClass__Check_Spy_Reveal @ 0x004FAF00`, `HouseClass__SpyPowerSabotage @ 0x0050BC90`, `OnSpyWeaponInfiltrate @ 0x006CE0B0` |
| `search_functions_enhanced(name_pattern="Disguise")` | 5 hits: `TechnoClass__IsDisguised_Getter @ 0x0041C020`, `BuildingClass__RemoveDetectDisguiseAt @ 0x00455980`, `BuildingClass__AddDetectDisguiseAt @ 0x00455A80`, `CellClass__IncrementDisguiseDetectCount @ 0x00487170`, `CellClass__DecrementDisguiseDetectCount @ 0x00487180` — confirms cell-level disguise-detect counters (used by gap generators / sensors) and the per-building add/remove on construction |
| `search_functions_enhanced(name_pattern="Infiltrate")` | 2 hits: `BuildingClass__OnSpyInfiltrate @ 0x004571E0`, `OnSpyWeaponInfiltrate @ 0x006CE0B0` |
| `decompile_function(0x004FAF00)` | `HouseClass::Check_Spy_Reveal` — reads `RulesClass+0xEE4` (proximity threshold) and `RulesClass+0xEC8` per-side probability table; updates `HouseClass+0x54F4/0x54FC` (LastSpyRevealCell/Frame). This is **not** the spy unit's disguise blink — it's the periodic "house has spied/revealed cell" tracker. Documented here so future readers do not conflate the two |

Confirmation: **`BuildingClass::OnSpyInfiltrate`'s 7-branch dispatch is the
only SPY-specific hardcoded function block** in gamemd.exe; disguise rendering
and detection are generic TechnoClass machinery shared with Mirage Tank.

---

## Ghidra audit log (audit iteration 6 — 2026-05-18)

Independent re-verification pass against gamemd.exe. ~15 decompiles across the
spy-infiltration system, disguise machinery, and supporting INI parsers.

### Function entry points re-verified (this audit)

| Doc claim | Verified at exact address |
|-----------|---------------------------|
| `BuildingClass::OnSpyInfiltrate @ 0x004571E0` | ✅ exact (body 004571e0–004575a4, 964 bytes) |
| `TechnoClass::IsDisguised_Getter @ 0x0041C020` | ✅ exact (body 0041c020–0041c028, 8 bytes — thin getter, see correction below) |
| `HouseClass::SpyPowerSabotage @ 0x0050BC90` | ✅ exact (body 0050bc90–0050bcc0) |
| `OnSpyWeaponInfiltrate @ 0x006CE0B0` | ✅ exact (body 006ce0b0–006ce19f) |
| `HouseClass::Check_Spy_Reveal @ 0x004FAF00` | ✅ exact (body 004faf00–004fb0d6) |
| `BuildingClass::AddDetectDisguiseAt @ 0x00455A80` | ✅ exact (body 00455a80–00455b8c) |
| `BuildingClass::RemoveDetectDisguiseAt @ 0x00455980` | ✅ exact (body 00455980–00455a78) |
| `CellClass::IncrementDisguiseDetectCount @ 0x00487170` | ✅ exact (body 14 bytes — thin inline) |
| `CellClass::DecrementDisguiseDetectCount @ 0x00487180` | ✅ exact (body 14 bytes — thin inline) |
| `MapClass::RestoreShroud @ 0x00577AB0` | ✅ exact (body 00577ab0–00577ba3) |
| `FUN_0050BD10` (RestoreShroud wrapper, LowPower-gated) | ✅ exact (body 0050bd10–0050bd25, unlabeled but matches the FUN_0050BD10 reference in the standalone SPY report) |
| `HouseClass::Spend_Money @ 0x004F9790` | ✅ exact |
| `HouseClass::Add_Credits @ 0x004F9950` | ✅ exact (one-line `+= param_2` at HouseClass+0x30C) |
| `HouseClass::IsHumanPlayer @ 0x0050B6F0` | ✅ exact |
| `AircraftClass::Mission_SpyPlane @ 0x00417300` | ✅ exact (body 00417300–004176d9) — distinct unit/mechanism from SPY infantry |

### OnSpyInfiltrate branch order — re-verified by decompile

The 7-branch first-match dispatch decompiled in this audit matches the doc
table exactly. Confirmed branch tests (in evaluation order):

1. `if (this->Owner == in_stack_00000004) return;` — same-owner early return
2. `puVar1[0x16a4]` (BuildingType+0x16A4 Radar bool) → wrapper `FUN_0050BD10` → `RestoreShroud` (LowPower-gated)
3. `*(int *)(puVar1 + 0xee0) < 1` → if false, enter Power branch → `HouseClass::SpyPowerSabotage(this->Owner, *(int*)(g_RulesClass_Instance + 0xd64))`
4. BuildTech list scan: iterates `*(int*)(g_RulesClass_Instance + 0x92c)` entries starting at `*(int**)(g_RulesClass_Instance + 0x920)`; dispatches on `*(int *)(puVar1 + 0x6d0)` (AIBasePlanningSide) into HouseClass+0x2BE / +0x2BD / +0x2BC (700 decimal) for Allied/Soviet/Third
5. `*(int *)(puVar1 + 0x16f0) == -1` → if false, enter SuperWeapon branch → `OnSpyWeaponInfiltrate()`
6. `0 < *(int *)(puVar1 + 0x800)` → Storage > 0 → Spend_Money / Add_Credits via virtual `+0x18` on `Owner+0x24`
7. `*(int *)(puVar1 + 0xeb8) == 0x28` → Factory=UnitType → `puVar3[0x2c0] = 1` (SpiedWarFactory) + `puVar3[0x1fc] = 1` (ProductionChanged) + `DAT_00884b8e = 1` (sidebar repaint global) [BINARY-VERIFIED]
8. `*(int *)(puVar1 + 0xeb8) == 0x10` → Factory=InfantryType → `puVar3[0x2bf] = 1` (SpiedBarracks) + ProductionChanged + sidebar repaint
9. End: `(**(code **)(this->vtable + 0x124))(2)` — BuildingClass vtable slot +0x124 called with arg 2 = mission Guard

### Struct offsets BINARY-VERIFIED (this audit)

**BuildingTypeClass (`int param_1` byte-offset convention in
`BuildingTypeClass_ReadINI_Water` @ 0x0045FF40):**

| Offset | Field | INI key |
|--------|-------|---------|
| +0x16A4 | Radar (bool) | `Radar=` ✅ |
| +0xEE0 | Power (int) | `Power=` ✅ |
| +0xEE8 | ExtraPower (int) | `ExtraPower=` (new in this audit) |
| +0xEB8 | Factory (RTTI enum: 0x10=InfantryType, 0x28=UnitType) | `Factory=` ✅ |
| +0x16F0 | SuperWeapon (int, -1=none) | `SuperWeapon=` ✅ |
| +0x16F4 | SuperWeapon2 (int) | `SuperWeapon2=` (new) |

**TechnoTypeClass (`int *param_1` in `TechnoTypeClass__ReadINI` — indexed values
are int-array indices; for byte access casts to `(int)param_1 + offset` are used):**

| Offset | Field | INI key |
|--------|-------|---------|
| +0x5F4 | DetectDisguiseRange (int) | `DetectDisguiseRange=` (new — also consumed by AddDetectDisguiseAt as ring radius) |
| +0x6D0 | AIBasePlanningSide (int) | `AIBasePlanningSide=` ✅ (used in BuildTech branch) |
| +0x800 | Storage (int) | `Storage=` ✅ |
| +0xD2F | CanDisguise (byte) | `CanDisguise=` (new) |
| +0xD30 | PermaDisguise (byte) | `PermaDisguise=` (new) |
| +0xD31 | DetectDisguise (byte) | `DetectDisguise=` (new) |

**RulesClass (`RulesClass__ReadGeneral` @ 0x0066F9F0 + `FUN_00672AE0` for AI block):**

| Offset | Field | INI key |
|--------|-------|---------|
| +0x91C | BuildTech DynamicVector start (new) | (DV object) |
| +0x920 | BuildTech data ptr (DV+4) | `BuildTech=` ✅ |
| +0x92C | BuildTech count (DV+0x10) | (runtime) ✅ |
| +0xD58 | AlliedDisguise (TypeClass*) | `AlliedDisguise=` (new — was missing from doc/cumulative cheat-sheet) |
| +0xD5C | SovietDisguise (TypeClass*) | `SovietDisguise=` (new) |
| +0xD60 | ThirdDisguise (TypeClass*) | `ThirdDisguise=` (new) |
| +0xD64 | SpyPowerBlackout (int) | `SpyPowerBlackout=` ✅ |
| +0xD68 | SpyMoneyStealPercent (float) | `SpyMoneyStealPercent=` ✅ |
| +0xD6C | AttackCursorOnDisguise (byte) | `AttackCursorOnDisguise=` (new offset) |
| +0x1014 | InfantryBlinkDisguiseTime (int) | `InfantryBlinkDisguiseTime=` (new offset) |
| +0xEC8 | per-side spy-reveal probability table ptr | (runtime — read in Check_Spy_Reveal) |
| +0xEE4 | spy-reveal proximity threshold (int) | (runtime — read in Check_Spy_Reveal) |

**HouseClass:**

| Offset | Field | Set/read by |
|--------|-------|-------------|
| +0x1EC, +0x1ED | "current player" flags (byte, byte) | IsHumanPlayer |
| +0x1FC | ProductionChanged (byte) | OnSpyInfiltrate stolen-tech / war-factory / barracks ✅ |
| +0x21C | Owner (used implicit in Ghidra-typed `this->Owner`) | OnSpyInfiltrate same-owner check |
| +0x241 | shroud-visibility flag (byte) | MapClass::RestoreShroud (zeros it via HouseArray[idx]+0x241=0) ✅ |
| +0x2A4 | BlackoutStartFrame (int) | SpyPowerSabotage ✅ |
| +0x2A8 | (timer aux, set from `local_8` — uninitialized in SpyPowerSabotage; appears unused) | SpyPowerSabotage (note: param_3 `blackoutEndFrame` is in fn signature but never used) |
| +0x2AC | BlackoutDuration (int) | SpyPowerSabotage ✅ |
| +0x2BC | StolenThirdTech (byte, set when AIBasePlanningSide ≥ 2) | OnSpyInfiltrate ✅ |
| +0x2BD | StolenSovietTech (byte, set when AIBasePlanningSide == 1) | OnSpyInfiltrate ✅ |
| +0x2BE | StolenAlliedTech (byte, set when AIBasePlanningSide == 0) | OnSpyInfiltrate ✅ |
| +0x2BF | SpiedBarracks (byte) | OnSpyInfiltrate barracks branch ✅ |
| +0x2C0 | SpiedWarFactory (byte) | OnSpyInfiltrate war-factory branch ✅ |
| +0x2DC | spent-credits running total (int) | Spend_Money |
| +0x30C | AvailableCredits (int) | Spend_Money / Add_Credits ✅ |
| +0x5490, +0x5494 | last-spy-related cell coord pair | Check_Spy_Reveal |
| +0x54F4 | LastSpyRevealCell | Check_Spy_Reveal ✅ |
| +0x54FC | LastSpyRevealFrame | Check_Spy_Reveal ✅ |
| +0x5778 | PowerBlackedOut (byte) | SpyPowerSabotage ✅ |
| +0x577A | LowPowerState (byte — gates RestoreShroud) | FUN_0050BD10 wrapper ✅ |

**SuperClass:**

| Offset | Field | Set by |
|--------|-------|--------|
| +0x24 | CustomRechargeTime (int, -1 sentinel) | OnSpyWeaponInfiltrate ✅ |
| +0x28 | Type (SuperWeaponTypeClass*) | (constructor) ✅ |
| +0x30 | RechargeStartFrame (int) | OnSpyWeaponInfiltrate ✅ |
| +0x34 | aux frame field (set from `uStack_8`) | OnSpyWeaponInfiltrate (new — exact role TBD) |
| +0x38 | RechargeDuration (int) | OnSpyWeaponInfiltrate ✅ |
| +0x68 | ChargeAnim (AnimClass*) | OnSpyWeaponInfiltrate clears ✅ |
| +0x6C | IsCharged (byte) | OnSpyWeaponInfiltrate clears ✅ |
| +0x6F | IsOneShotFired (byte) | OnSpyWeaponInfiltrate zeros ✅ |
| +0x78 | CameoChargeFrame (int) | OnSpyWeaponInfiltrate sets -1 ✅ |
| (AnimClass+0x195) | IsActive byte on ChargeAnim | OnSpyWeaponInfiltrate zeros to deactivate ✅ |
| SuperWeaponTypeClass+0xB0 | RechargeTime (int) | (Type-side default, picked up when CustomRechargeTime == -1) ✅ |

**TechnoClass instance:**

| Offset | Field | Read by |
|--------|-------|---------|
| +0x1D8 | IsDisguised (byte) | TechnoClass::IsDisguised_Getter ✅ (new offset finding) |

**CellClass:**

| Offset | Field | Read by |
|--------|-------|---------|
| +0xAC | disguise-detect counter array (`short[NumHouses]`, indexed by house index) | CellClass::Increment/DecrementDisguiseDetectCount ✅ (new — confirms per-cell-per-house detect tracking, with the building's house index sourced from `Owner+0x30`) |

### Parser-scope verifications

| INI key | Reader function (xref) | Scope |
|---------|------------------------|-------|
| `Agent` | InfantryTypeClass__ReadINI @ 0x005245A1 | **InfantryType** ✅ (confirms doc's use of Agent on SPY) |
| `Infiltrate` | InfantryTypeClass__ReadINI @ 0x005244A1 | **InfantryType** ✅ |
| `PermaDisguise` | TechnoTypeClass__ReadINI @ 0x00714425 | **TechnoType** (corrects the implicit assumption that PermaDisguise lives on InfantryType — it's TechnoType-scope, used by Mirage Tank too) |
| `MakesDisguise` | WarheadTypeClass__ReadINI @ 0x0075D956 | **WarheadType** ✅ |
| `Storage` | TechnoTypeClass__ReadINI @ 0x00713130 | **TechnoType** ✅ |
| `Radar` | BuildingTypeClass_ReadINI_Water @ 0x0045FF5A | **BuildingType** ✅ |
| `Power` | BuildingTypeClass_ReadINI_Water @ 0x00461073 | **BuildingType** ✅ |
| `SuperWeapon` | BuildingTypeClass_ReadINI_Water @ 0x00460BB8 | **BuildingType** ✅ |
| `Factory` | BuildingTypeClass_ReadINI_Water @ 0x00460521 | **BuildingType** ✅ |
| `AIBasePlanningSide` | TechnoTypeClass__ReadINI @ 0x00714A02 | **TechnoType** ✅ |
| `DetectDisguise` | TechnoTypeClass__ReadINI @ 0x0071443F | **TechnoType** ✅ |
| `DetectDisguiseRange` | TechnoTypeClass__ReadINI @ 0x00714302 | **TechnoType** ✅ |
| `BuildTech` | FUN_00672AE0 (RulesClass AI block reader) @ 0x00672EC3 | **Rules-AI** ✅ |
| `SpyPowerBlackout` | RulesClass__ReadGeneral @ 0x0066FBFE | **Rules-General** ✅ |
| `SpyMoneyStealPercent` | RulesClass__ReadGeneral @ 0x0066FC25 | **Rules-General** ✅ |
| `AttackCursorOnDisguise` | RulesClass__ReadGeneral @ 0x0066FC43 | **Rules-General** ✅ |
| `InfantryBlinkDisguiseTime` | RulesClass__ReadGeneral @ 0x00671D7F | **Rules-General** ✅ |

### Corrections / discrepancies

1. **[INCORRECT corrected]**: The doc claimed `TechnoClass::IsDisguised_Getter`
   is the "runtime predicate that selects whether to draw the real SHP or the
   disguised TechnoType's SHP" with "per-viewer-side, not per-frame" semantics.
   The function is actually a trivial 1-byte flag getter
   (`return *(byte *)(this + 0x1D8);`). The per-viewer-side draw decision must
   live in the draw-pipeline / disguise-render code, not in this getter. Tag
   updated inline.

2. **[CALL-CHAIN ADDED]**: `OnSpyInfiltrate`'s sole caller is
   `InfantryClass::PerCellProcess` @ 0x00519630. This confirms the spy/engineer
   "Mission Enter" hand-off is dispatched from within PerCellProcess (consistent
   with audit 3's finding for ENGINEER).

3. **[NEW OFFSETS]**: Several BuildingType / TechnoType / Rules offsets used by
   the spy system were not previously in the doc or cheat-sheet. Added above
   and to the cumulative cheat-sheet.

4. **[CONSISTENCY-CHECK passed]**: The 7-branch first-match dispatch in
   `BuildingClass::OnSpyInfiltrate` matches the [SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md](../../SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md)
   standalone report's branch table exactly. The standalone report's "+0x520 =
   Type ptr on BuildingClass" claim is consistent with audit 4's prior
   BINARY-VERIFIED finding (GHOST).

### Items NOT re-verified this pass (DEFERRED)

- **AircraftClass::Mission_SpyPlane @ 0x00417300** — function entry confirmed
  but body not decompiled. It controls the SPYP superpower aircraft (separate
  unit from SPY infantry); audit when SPYP doc is reached.
- **OnSpyInfiltrate's CreateRadarEvent + radar-event flag** — present in
  decompile (`puVar5 = (undefined4 *)(**(code **)(this->vtable + 0x1b8))(...)`
  then `CreateRadarEvent(*puVar5)`) but the vtable+0x1b8 call is opaque without
  digging through BuildingClass vtable. DEFERRED.
- **vtable+0x124 (mission-set with arg 2)** — confirms the slot exists but
  whether it's strictly "SetMission" vs a different mission-related virtual is
  not separately confirmed in this audit. Cumulative findings from prior audits
  show BuildingClass vtable+0x274 = SetMission; +0x124 is a different slot.
  DEFERRED for vtable resolution.
- **Spend_Money's StorageClass branching** (the ore-storage drain path when
  AvailableCredits is insufficient) — visible in decompile but not deeply
  traced; not load-bearing for spy money-steal because spy attacker just gets
  the lump-sum and victim's Spend_Money handles any cascade.

### Confidence summary

**HIGH** for the 7-branch dispatch order, all BuildingType / Rules / HouseClass
offsets in branches 1–8, the SpyPowerSabotage timer-field writes, the
OnSpyWeaponInfiltrate SuperClass field writes, the RestoreShroud field writes,
and the parser-scope assignments for all 17 INI keys re-verified.

**MEDIUM** for the disguise-detect machinery — Increment/Decrement counters
verified, AddDetectDisguiseAt's ring iteration verified, but the consumer side
(which engine code reads `CellClass+0xAC[house]` to gate disguise reveal /
gap-generator behavior) was not traced this pass.

**LOW / unverified** for the `Type=global` SpySpecialAttack sound timing
relative to OnSpyInfiltrate's EVA voice playback ordering — not separately
decompiled this pass. Doc claim retained as-is.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `ImmuneToVeins=` (absent) | Veins are TS-only terrain; absence is moot in YR | OK |
| `MovementZone=Infantry` | Standard, not TS-special | OK |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` — same GUID used by all RA2/YR infantry, not the TS-only `JumpjetLocomotionClass` or `TunnelLocomotionClass` | OK |
| `Bombable=` (absent) | RA2/YR; no TS-only meaning | OK |
| `[Snapshot].Verses` 11 columns | Matches YR's 11-armor table (TS used 8). Confirmed alive | OK |
| `Agent=yes`, `Infiltrate=yes`, `CanDisguise=yes`, `PermaDisguise=yes` | All four are **live YR flags**; no TS-gating. Confirmed by decompile of `OnSpyInfiltrate` (no SpecialFlags check) | OK |
| `AttackCursorOnDisguise=` | Inherited from TS but **active and read in YR** (cursor predicate runs every selection check) | OK — keep |
| `InfantryBlinkDisguiseTime=` | Engine-active in YR (Mirage Tank reveal also depends on it) | OK — keep |
| `StupidHunt=yes` | Used by AI mission-pick fallback in YR; not TS-only | OK |

No TS-only behavior found on the SPY type itself.

---

## Cross-references

- **Related units** sharing the disguise / Snapshot machinery:
  - `[MGTK]` Mirage Tank — `[TankMakeupKit]`/`[TankSnapshot]`, `CanDisguise=yes` but `PermaDisguise` absent (drops on fire/move). Same `vmirat2a` audio family
  - `[HTK]` Multi-Gunner IFV — gunner table consumes `IFVMode=2`
- **Related buildings** that trigger spy effects:
  - `[GATECH]` `[NATECH]` `[YATECH]` — BuildTech list (BRANCH 4)
  - `[GAPOWR]` `[NAPOWR]` `[NANRCT]` `[YAPOWR]` — Power > 0 (BRANCH 2)
  - `[GAREFN]` `[NAREFN]` `[YAREFN]` `[GAOREP]` — Storage > 0 (BRANCH 5)
  - Any building with `Radar=yes` (`[NARADR]`, `[GAAIRC]`, `[GASPYSAT]`) — BRANCH 1
  - Any building with `SuperWeapon=N` set (`[GACSPH]`, `[GAWEAT]`, `[NAIRON]`, `[NAMISL]`, `[YAGNTC]`, `[YAPPET]`) — priority 5 (BRANCH 4 in standalone-report numbering)
  - Any `Factory=UnitType` (`[GAWEAP]`, `[NAWEAP]`, `[YAWEAP]`) — priority 7 (BRANCH 6 in standalone-report numbering)
  - Any `Factory=InfantryType` (`[GAPILE]`, `[NAHAND]`, `[YABRCK]`) — priority 8 (BRANCH 7 in standalone-report numbering)
- **Related rules keys** (search-jumps):
  - `AlliedDisguise`, `SovietDisguise`, `ThirdDisguise` — line 274-276
  - `SpyPowerBlackout`, `SpyMoneyStealPercent` — line 277-278
  - `AttackCursorOnDisguise` — line 281
  - `DefaultMirageDisguises` — line 285 (Mirage Tank only, not Spy)
  - `InfantryBlinkDisguiseTime` — line 286
  - `BuildTech` — line 3070
- **Powers / related concepts** (separate units — not this spy):
  - `[SPYP]` Spy Plane (`rulesmd.ini:11323`) — Allied superpower-spawned aircraft that flies a recon pass, reveals shroud. Distinct from the SPY infantry
  - `[GASPYSAT]` Spy Satellite Uplink (`rulesmd.ini:12187`) — building that permanently reveals the full map for the owner

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [SPY]` | 3973-4012 (40 lines) | All keys covered with explanation |
| `artmd.ini [SPY]` | 131-137 (7 lines) | All keys covered |
| `artmd.ini [SpySequence]` | 13948-13967 (20 lines) | All slots covered |
| `rulesmd.ini [MakeupKit]` | 24140-24150 (11 lines) | All keys covered |
| `rulesmd.ini [Snapshot]` | 27473-27475 (3 lines) | All keys covered |
| `rulesmd.ini [InvisibleAll]` | 25406-25414 (9 lines) | All keys covered |
| `soundmd.ini` SPY voices | SpySelect, SpyMove, SpyAttackCommand, SpyFear, SpyDie, SpySpecialAttack, SpyCreated, SpyAttack | All covered |
| `rulesmd.ini` SPY-related globals | AlliedDisguise, SovietDisguise, ThirdDisguise, SpyPowerBlackout, SpyMoneyStealPercent, AttackCursorOnDisguise, InfantryBlinkDisguiseTime, BuildTech | All covered |
| Hardcoded behavior | `OnSpyInfiltrate` 7-branch dispatch + sister functions | Covered (full RE in standalone report) |
| Ghidra searches performed against ID | 6 distinct queries (strings + 3 function-name searches + 1 decompile + xrefs) | Logged |
| TS-legacy filter | Applied; no TS-only behavior found on SPY type | Done |
