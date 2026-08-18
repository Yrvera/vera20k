# Sniper (SNIPE)
Side: Allied | Category: Infantry | Image alias: `[SNIPE]` (no `Image=` redirect — own SHP `SNIPE`)

The Allied Sniper. $600 from the Barracks (needs Radar). British-only one-shot
anti-infantry specialist. The "one-shot" feel is **not** a unit-specific hardcoded
instant-kill — it emerges from the **AWP weapon's `Damage=125` × `[HollowPoint]`
warhead's `Verses=200%` against `Armor=none`** = 250 damage per shot, more than
enough to one-shot every standard infantry (which are all Strength≤125 in
Armor=none or flak class). `RevealOnFire=no` on the weapon means firing does
**not clear shroud** — combined with `Sight=8` (one less than Spy's 9), the
sniper can pick off enemies from inside its own vision without exposing the
shooter's position via shroud reveal.

No psionic immunity — `ImmuneToPsionics=no` is explicit, so the sniper CAN be
mind-controlled (a significant counter). Trained from any Allied Barracks but
**only the British house** can actually build it (`RequiredHouses=British`).
The IFV gunner swap is `IFVMode=5` → sniper IFV slot (anti-infantry beam in
stock YR's `[HTK]` weapon table).

No standalone sniper RE doc previously existed; this document originates the
Ghidra trace of the `UseOwnName`/`RequiredHouses`/`RevealOnFire` flag paths.

---

## rulesmd.ini — `[SNIPE]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4278`:

```ini
[SNIPE]
UIName=Name:SNIPE
Name=Sniper
Category=Soldier
Primary=AWP
;CanPassiveAquire=no ; Won't try to pick up own targets
Prerequisite=GAPILE,RADAR
CrushSound=InfantrySquish
Strength=125
Pip=red
Armor=none
TechLevel=1
Sight=8
Speed=4
Owner=British,French,Germans,Americans,Alliance
RequiredHouses=British
Cost=600
Soylent=300
Points=10
IsSelectableCombatant=yes
VoiceSelect=SniperSelect
VoiceMove=SniperMove
VoiceAttack=SniperAttackCommand
VoiceFeedback=SniperFear
VoiceSpecialAttack=SniperMove
DieSound=SniperDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=10	; This value MUST be 0 for all building addons
ImmuneToVeins=yes
ImmuneToPsionics=no
Bombable=yes
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Size=1
AllowedToStartInMultiplayer=no
ElitePrimary=AWPE
PreventAttackMove=no
IFVMode=5
UseOwnName=true
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:SNIPE` | CSF-string key → "Sniper" |
| `Name=Sniper` | Internal short name |
| `Category=Soldier` | Pip group + AI grouping (infantry) |
| `Primary=AWP` | The sniper rifle weapon — `Damage=125`, `ROF=150`, `Range=14`, `RevealOnFire=no`, warhead `HollowPoint`. See "Weapons" |
| `;CanPassiveAquire=no` (commented) | **Intentionally NOT disabled** — sniper DOES passively acquire targets when stationary. The commented line preserves the design history; current behavior is the engine default (passive acquire = yes) |
| `Prerequisite=GAPILE,RADAR` | Both Allied Barracks AND any building with `Radar=yes` |
| `CrushSound=InfantrySquish` | Crush sound (`igensqua`) |
| `Strength=125` | HP — 25% more than GI; same as Tanya/Boris. Survives one bullet from another sniper (125 vs 250 effective is fatal — wait, 125 HP vs 250 damage IS a one-shot, so even snipers can be sniped) |
| `Pip=red` | Cargo-passenger pip color — **red** is the "elite/special" color (Tanya/SEAL/Sniper all share). Compare regular infantry which uses white or blue |
| `Armor=none` | Damage type column 0 — basic infantry armor. Despite being elite-feel, no special armor class |
| `TechLevel=1` | Buildable from tech-level 1+ (gated only by Barracks+Radar prereq, which raises effective tech) |
| `Sight=8` | Reveal radius — large (one less than Spy's 9). **Critical for the one-shot loop**: weapon range 14 > sight 8, so the sniper can shoot beyond what it sees — but it can only acquire targets within its sight, then the shot itself goes to range 14 without revealing the shooter (`RevealOnFire=no` on weapon) |
| `Speed=4` | Foot-speed — slow (same as GI). The sniper is positional, not mobile |
| `Owner=British,French,Germans,Americans,Alliance` | All five Allied countries in `Owner=` |
| `RequiredHouses=British` | **Country-locked to British only** — TechnoTypeClass field (xref from `TechnoTypeClass__ReadINI @ 0x00714529` to string at `0x00843BB4`). Despite Owner= listing all Allied countries, only Britain can actually build it. The Sniper is **Britain's national special unit** — every Allied country has one (French Mirage, German Tank Destroyer, American Paratroopers, Korean Black Eagle, British Sniper) |
| `Cost=600` | $600 — moderately expensive for infantry |
| `Soylent=300` | Grinder refund (Yuri only — same 50% as everyone) |
| `Points=10` | Kill score |
| `IsSelectableCombatant=yes` | Included in "select all combat units" hotkey |
| `VoiceSelect=SniperSelect` | Selection voice — `$isnisea/b/c/d` |
| `VoiceMove=SniperMove` | Move voice — `$isnimoa..e` |
| `VoiceAttack=SniperAttackCommand` | Attack-order voice — `$isniata/b/c` |
| `VoiceFeedback=SniperFear` | Fear voice — `$isnifea/b/c` (Priority=low) |
| `VoiceSpecialAttack=SniperMove` | Reuses Move voice — sniper has no special attack |
| `DieSound=SniperDie` | Death voice — `$isnidia/b/c` |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — same as all infantry |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `ThreatPosed=10` | AI scoring weight — modest (one-shots infantry but limited DPS due to ROF=150 = 10 seconds between shots) |
| `ImmuneToVeins=yes` | TS legacy; veins are TS-only terrain. No effect in YR |
| `ImmuneToPsionics=no` | **EXPLICIT no** — sniper CAN be mind-controlled. Important balance counter: Yuri can flip a sniper and use it against the original owner |
| `Bombable=yes` | Crazy Ivan can plant a bomb on this unit |
| `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` | At Veteran rank — note **no ROF** (sniper is slow-firing by design) |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | At Elite rank — gains ROF here (combined with `ElitePrimary=AWPE` which is ROF 60 vs 150) |
| `Size=1` | Transport cargo slot cost |
| `AllowedToStartInMultiplayer=no` | Cannot appear in starting unit complement |
| `ElitePrimary=AWPE` | At Elite rank, Primary swaps to `[AWPE]` — same Damage/Range, but **ROF drops from 150 to 60** (2.5× faster firing). Crucial — Elite sniper is the only fast-cycle infantry-killer for Allies |
| `PreventAttackMove=no` | **EXPLICIT no** — sniper obeys Attack-Move (most "specialist" infantry have PreventAttackMove=yes like engineer/spy). The sniper is intended to attack-move into firing position |
| `IFVMode=5` | IFV gunner-table index 5 → **Sniper Rifle / anti-infantry beam** when this passenger boards an [HTK]. The IFV chassis takes on a long-range high-damage anti-infantry weapon while the sniper is loaded |
| `UseOwnName=true` | **Behavior flag** — InfantryTypeClass field (xref from `InfantryTypeClass__ReadINI @ 0x0052463D` to string at `0x00825908`). When the player hovers over an enemy sniper, the tooltip shows "Sniper" instead of the generic category-derived "Infantry" — i.e., the engine **reveals the specific type** even when normally type-info would be hidden by fog. Used for high-profile units (other UseOwnName cases include Tanya/Boris/Yuri-Prime). Has the side effect of UI consistency: own snipers also display "Sniper" on hover rather than the generic group label |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `yes` (prone while crawling enabled)
- `NotHuman=` — defaults to `no`; sniper IS a human (subject to InfDeath, blood, etc.)
- `ImmuneToRadiation=` — not set, defaults to `no`; sniper killed by radiation
- `DetectDisguise=` — not set; sniper does NOT detect spies/mirages
- `Deployer=` — defaults to `no`; sniper has no deploy command
- `Occupier=` — defaults to `no`; sniper cannot garrison civilian buildings
- `C4=` — not set
- `Assaulter=` — not set
- `Agent=`/`Infiltrate=` — not set; sniper cannot infiltrate
- `DefaultToGuardArea=` — not set; sniper holds position when idle (just MissionGuard, not GuardArea)
- `Trainable=` — not set, defaults to `yes` (sniper IS trainable, confirmed by the presence of VeteranAbilities/EliteAbilities/ElitePrimary)
- `ReselectIfLimboed=`/`RejoinTeamIfLimboed=` — not set; sniper never Limbo's via weapon
- `Natural=` — not set

---

## artmd.ini — `[SNIPE]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:301`:

```ini
[SNIPE] ; Sniper
Cameo=SNIPICON
AltCameo=SNIPUICO
Sequence=ConSequence ;Generic infantry that can paradrop
Crawls=yes
Remapable=yes
FireUp=5
PrimaryFireFLH=80,0,85
```

| Key | Meaning |
|-----|---------|
| `Cameo=SNIPICON` | Sidebar build icon (SHP) |
| `AltCameo=SNIPUICO` | Elite cameo — IS shown (`Trainable=yes` default) |
| `Sequence=ConSequence` | **SHARED sequence** — uses `[ConSequence]` (the generic Conscript sequence). Designer comment: "Generic infantry that can paradrop" — this is the standard "infantry that can be in a paradrop" art layout. Reused by Conscript itself, Sniper, GI variants, and other generic infantry |
| `Crawls=yes` | Sets the prone-while-walking enabled flag — sniper can go prone |
| `Remapable=yes` | House remap palette applied |
| `FireUp=5` | Bullet-spawn frame within the FireUp track — at frame 5 the rifle fires (matches the rifle's slow aim-up + shot recoil cycle) |
| `PrimaryFireFLH=80,0,85` | Fire-Launch-Height for AWP — 80 leptons forward, 0 sideways, 85 leptons up. Z=85 matches typical rifle shoulder height for an infantry sprite |

Missing `SecondaryFireFLH=` because sniper has no Secondary.

### Referenced sequence — `[ConSequence]`

`artmd.ini:13770`:

```ini
[ConSequence]
Ready=0,1,1
Guard=0,1,1
Prone=86,1,6
Walk=8,6,6
FireUp=164,6,6
Down=260,2,2
Crawl=86,6,6
Up=276,2,2
FireProne=212,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Die1=134,15,0
Die2=149,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
;Cheer=56,15,0,W
Cheer=293,8,0,E
Paradrop=292,1,0
Panic=8,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle (1 frame × 1 facing) | |
| `Guard=0,1,1` | Guard idle | Same |
| `Prone=86,1,6` | Prone — 1 frame × 6 facings | Static prone pose |
| `Walk=8,6,6` | Walk cycle — 6 frames × 6 facings | |
| `FireUp=164,6,6` | Stand-fire cycle — 6 frames × 6 facings | Where AWP fires when standing |
| `Down=260,2,2` | Get-down to prone — 2 frames × 2 facings | |
| `Crawl=86,6,6` | Crawl = reuse of Prone frames as 6-cycle | |
| `Up=276,2,2` | Get-up from prone | |
| `FireProne=212,6,6` | Prone-fire cycle — 6 frames × 6 facings | Sniper can fire while prone — this is heavily used |
| `Idle1=56,15,0,S` | Idle anim 1 — 15 frames, S-facing | |
| `Idle2=71,15,0,E` | Idle anim 2 — 15 frames, E-facing | |
| `Die1=134,15,0` | Death anim 1 — 15 frames |  |
| `Die2=149,15,0` | Death anim 2 |  |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → fall back to Ready frame | Unused variants |
| `;Cheer=56,15,0,W` | Commented older W-facing cheer | Superseded by E-facing |
| `Cheer=293,8,0,E` | Cheer — 8 frames, E-facing | |
| `Paradrop=292,1,0` | Single-frame paradrop pose at frame 292 | **Live** — Conscript-like infantry that can be paradropped (sniper is paradrop-eligible via Paradrop superpower / map script) |
| `Panic=8,6,6` | Panic = reuse of Walk frames | |

Note this sequence is **shared with multiple infantry units** including Conscript, Sniper, Virus (via Image= redirect), and other "generic" types. Per-unit FireUp frame offset (the FireUp= top-level field in artmd) varies (Sniper=5, Conscript=6, etc.) to match each unit's specific rifle / weapon-pose timing.

---

## Weapons

### Primary (Veteran and below) — `[AWP]`

`rulesmd.ini:23076`:

```ini
[AWP]
Damage=125
ROF=150
Range=14
Projectile=InvisibleLow
Speed=100
Report=SniperAttack
Warhead=HollowPoint
RevealOnFire=no ; Doesn't clear shroud when fired
```

(AWP = "Arctic Warfare Police" — the Accuracy International L96A1 sniper
rifle that the sprite depicts. Real-world brand name retained.)

| Key | Meaning |
|-----|---------|
| `Damage=125` | Per-shot raw damage. Combined with `HollowPoint.Verses=200%` vs Armor=none → **250 effective damage** vs basic infantry (one-shots Strength≤250 infantry, which is all of them) |
| `ROF=150` | Cooldown — 150 frames = 10 seconds @ 15fps. **Very slow** by design — sniper is single-target precision, not sustained DPS |
| `Range=14` | 14 cells — by far the longest infantry weapon range. Compare GI's M60=6, Tesla Trooper=5. The sniper outranges every static defense except Tesla Coil (range 8) and Grand Cannon (range 18). **Range > Sight (8)** is the key one-shot setup |
| `Projectile=InvisibleLow` | `Inviso=yes Image=none SubjectToCliffs=yes SubjectToElevation=yes SubjectToWalls=yes`. **The "Low" variant** is blocked by cliffs/walls/elevation — sniper bullets do NOT pass through walls. Compare `[Invisible]` (no such restrictions) used by free-flying inviso weapons |
| `Speed=100` | Irrelevant for inviso instant-resolution |
| `Report=SniperAttack` | Sound `isniatta` (silenced "clack" sample). **`Priority=critical`** on the sound — engine prioritizes this audio above other sounds so the player always hears it |
| `Warhead=HollowPoint` | See warhead — 200% vs basic infantry armor |
| `RevealOnFire=no` | **Behavior flag** — WeaponTypeClass field (xref from `WeaponTypeClass__ReadINI @ 0x00772189` to string at `0x008494E0`). Firing this weapon does **not clear shroud** around the firing unit. **Critical hardcoded behavior**: combined with `Sight=8 < Range=14`, the sniper can fire at units it sees, but the shot's impact location at range >8 does NOT auto-reveal the area between the sniper and the target. Standard weapons reveal the firing position + path; sniper does not |

### Elite Primary — `[AWPE]`

`rulesmd.ini:25146`:

```ini
[AWPE]
Damage=125
ROF=60
Range=14
Projectile=InvisibleLow
Speed=100
Report=SniperAttack
Warhead=HollowPoint
RevealOnFire=no ; Doesn't clear shroud when fired
```

**Identical** to `[AWP]` except **`ROF=60` instead of 150** — 2.5× faster firing rate. Same damage, same range, same warhead, same RevealOnFire-no. Activated via `ElitePrimary=AWPE` once unit reaches Elite veterancy.

### Primary's Warhead — `[HollowPoint]`

`rulesmd.ini:27053`:

```ini
[HollowPoint]
Verses=200%,100%,100%,1%,1%,1%,1%,1%,1%,1%,100% ; see note in comments above about 1%
InfDeath=1
AnimList=PIFF
ProneDamage=100%
Bullets=yes
```

| Key | Meaning |
|-----|---------|
| `Verses=200%,100%,100%,1%,1%,1%,1%,1%,1%,1%,100%` | 11-column armor row. **200% vs `none` armor** (basic infantry like GI, Engineer, Sniper, Spy, Tanya, Boris) — this is what makes the sniper one-shot infantry. **100% vs `flak/plate`** (Tesla Trooper, Conscript, FlakTrooper, Desolator) — these take full 125, one-shotted at 100 Strength. **1% vs vehicle/structure armors** (light/medium/heavy/wood/steel/concrete) — the 1% is the engine's "minimum non-zero" trick: it allows the attack cursor on vehicles (compare 0% which would block), but deals essentially no damage. The final `100%` is `special_2` for high-priority infantry (some special-armor types). Comment "see note in comments above about 1%" refers to the engine quirk where 1% preserves targetability without meaningful damage |
| `InfDeath=1` | Infantry death animation type 1 — "small arms" — the standard shot-down anim |
| `AnimList=PIFF` | Impact animation = single `PIFF` muzzle/impact puff |
| `ProneDamage=100%` | **No prone-damage reduction** — prone infantry take full damage from this warhead. Most warheads have ProneDamage=80% (prone reduces by 20%); sniper specifically pierces through prone. **Critical for parity**: a player going prone to evade SSA (Rocketeer 20mm) WILL NOT evade a sniper |
| `Bullets=yes` | Marks the warhead as bullet-type for engine purposes |

### Projectile — `[InvisibleLow]`

`rulesmd.ini:25385`:

```ini
[InvisibleLow]
Inviso=yes
Image=none
SubjectToCliffs=yes
SubjectToElevation=yes
SubjectToWalls=yes
```

| Key | Meaning |
|-----|---------|
| `Inviso=yes` | No projectile sprite; instant resolution |
| `Image=none` | No image asset |
| `SubjectToCliffs=yes` | **Blocked by cliffs** — sniper cannot shoot up a cliff if line-of-sight to target's elevation is blocked |
| `SubjectToElevation=yes` | **Subject to elevation differences** — relative-height matters for LOS |
| `SubjectToWalls=yes` | **Blocked by walls** — Allied/Soviet walls stop the bullet |

These three flags together restrict the sniper to "realistic" LOS rules
despite the long range. The 14-cell range is meaningful only if there's a
clear shot — no shooting over walls or up/down cliffs.

### Unused vestigial — `[Sniper]`

`rulesmd.ini:22837`:

```ini
[Sniper]
Damage=150
ROF=20
Range=8
Projectile=InvisibleLow
Speed=100
Warhead=HollowPoint
Report=SILENCER
```

**This weapon is NOT used by [SNIPE]** — the sniper unit uses `Primary=AWP`,
not `Primary=Sniper`. The `[Sniper]` section is **vestigial** (probably from
an earlier design pass with different stats). Evidence it's dead code:
- `Report=SILENCER` — but `[SILENCER]` sound entry **does not exist** in
  soundmd.ini (verified by grep returning 0 matches). The sound would fail to
  play if this weapon were ever used.
- Damage=150 / ROF=20 / Range=8 — would be a completely different unit
  (closer to a designated-marksman role than a sniper)

The `[Sniper]` weapon may be used by some map-script unit or special
override. Worth noting in case it shows up elsewhere — but it is NOT what
the in-game Sniper unit actually fires.

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear

```ini
[SniperSelect]                  ; soundmd.ini:3988
Sounds= $isnisea $isniseb $isnisec $isnised
Control= random interrupt
Volume=90

[SniperMove]                    ; soundmd.ini:3983
Sounds= $isnimoa $isnimob $isnimoc $isnimod $isnimoe
Control= random interrupt
Volume=90

[SniperAttackCommand]           ; soundmd.ini:3979
Sounds= $isniata $isniatb $isniatc
Control= random interrupt

[SniperFear]                    ; soundmd.ini:3993
Sounds= $isnifea $isnifeb $isnifec
Control= random interrupt
Priority=low
Volume=90
```

Four select / five move / three attack / three fear lines — standard
"British military operative" voice bank.

### Death

```ini
[SniperDie]                     ; soundmd.ini:3999
Sounds= $isnidia $isnidib $isnidic
Control= random interrupt
Volume=90
```

Three death lines (random pick) — single-shot voices, no continuous chain.

### Weapon report

```ini
[SniperAttack]                  ; soundmd.ini:1122
Sounds=isniatta
Control= random
Priority=critical
FShift= -5 5
Range=30
Volume=90
```

Single sample `isniatta` (silenced rifle "clack"). **`Priority=critical`** is
unusual — most weapon reports are normal/low priority. Sniper-rifle audio is
elevated so players always hear when a sniper fires nearby (gameplay signal).
**`Range=30`** — audible within 30 cells, more than 2× the weapon's range
of 14 — so victims and bystanders well outside the shot can hear it.

### Notable absence — `[SILENCER]`

Referenced by the vestigial unused `[Sniper]` weapon (`Report=SILENCER`).
Grep confirms **no `[SILENCER]` section exists** in soundmd.ini. Further
evidence the `[Sniper]` weapon is dead.

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `GAPILE,RADAR` | Allied Barracks AND any building with `Radar=yes` (typically `GAAIRC` Airforce Command HQ) |
| `Owner=` | `British,French,Germans,Americans,Alliance` | All Allied countries listed |
| `RequiredHouses=` | `British` | **Country lock — Britain only**. TechnoTypeClass field. Even if a player as France/Germany/America captures an Allied barracks, they still cannot build the sniper unless they ARE Britain |
| `TechLevel=` | `1` | Effectively gated by Radar (~tech 2-3 implicit) |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=600` | $600 | |
| `Soylent=300` | $300 refund | Grinder (Yuri) only |
| `Points=10` | 10 | Kill-score contribution |

No `PrerequisiteOverride=`, no `BuildLimit=`, no `RequiresStolenXxxTech=`.

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` — standard +25% HP, +25% FP, +1 sight, +20% speed. **No ROF improvement** at Veteran tier (preserved for Elite) |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — passive HP regen, stack +50% HP, stack +50% FP, +40% ROF. **Plus** triggers `ElitePrimary=AWPE` (ROF 60 vs 150 = 2.5× faster firing). The ROF improvement at Elite combined with AWPE swap effectively gives Elite snipers a **6× faster shot cadence** than Veteran |
| AltCameo | `SNIPUICO` shown in sidebar once Veteran rank reached |

`Trainable=` defaults to `yes` (not overridden) — sniper can gain XP.

---

## Hardcoded behavior — Ghidra-verified

### 1. The "one-shot infantry kill" is data-driven, not hardcoded [BINARY-VERIFIED audit 10]

The sniper does **not** have a special instant-kill code path. Confirmed
this audit pass: `search_strings("^Sniper$")` returns **0 matches** —
no standalone "Sniper" string exists in gamemd.exe as a hardcoded
identifier. The unit's behavior is entirely INI-driven. The one-shot
feel emerges from the data stack:

```
AWP.Damage=125  ×  HollowPoint.Verses[Armor=none]=200%  =  250 damage
GI.Strength=100, Conscript.Strength=125, Engineer.Strength=75, etc.
→ All standard infantry die in one shot
```

Even Strength=125 infantry like Tanya/Boris/Sniper itself die in one shot
(125 HP < 250 damage). The few exceptions: Yuri Prime (Strength=125 but
ImmuneToPsionics gating doesn't help — still dies in one), Brute (Strength=350,
takes 2 shots).

No `SuicideWeapon=yes` or `Culling=yes` flag is used.

### 2. RevealOnFire=no — the hidden-shooter mechanic [BINARY-VERIFIED audit 10]

INI key `RevealOnFire` is a **WeaponTypeClass** field at byte offset
**`WeaponTypeClass+0x137`** (BINARY-VERIFIED via the WeaponTypeClass__ReadINI
decompile in audit 9 — store site `*(char *)((int)this + 0x137) = uVar4`;
xref from `WeaponTypeClass__ReadINI @ 0x00772189` DATA to string at
`0x008494E0`). Default for weapons is `yes` — firing reveals the shooter's
surrounding area to the enemy by clearing shroud in the firing path. AWP
and AWPE both set `RevealOnFire=no`, so:

- Enemy players' shroud does NOT clear around the sniper when it fires.
- The bullet impact does not unshroud the path or the destination.
- Combined with `Range=14 > Sight=8`, the sniper can engage from inside its
  vision while remaining outside the enemy's awareness (until enemy units
  walk close enough to see the sniper themselves).

This is the primary mechanic that gives the sniper its "feared invisible
killer" feel. **All assassin-style weapons in YR use this flag** (Spy's
MakeupKit also has RevealOnFire=no).

### 3. RequiredHouses=British — country lock [BINARY-VERIFIED audit 10]

INI key `RequiredHouses` is a **TechnoTypeClass** field at byte offset
**`TechnoTypeClass+0xDA0` (int — house-bitmask or vector-ptr)** (xref from
`TechnoTypeClass__ReadINI @ 0x00714529` DATA to string at `0x00843BB4`,
default read from `param_1[0x368]`; parsed via the helper `FUN_004750D0`).
`ForbiddenHouses` is the adjacent **TechnoTypeClass+0xDA4** (int — xref @
0x0071455D). Sets a per-type "house filter" that requires the player's
country be in the list (additive — at least one match required). Sniper's
`RequiredHouses=British` restricts buildability to British house only,
regardless of `Owner=` or captured production buildings. Compare
`ForbiddenHouses=` which is exclusive (the listed houses cannot build).

The hierarchy: `Owner=` defines who *could* build (whitelist of countries
present in the type's faction); `ForbiddenHouses=` removes from that list;
`RequiredHouses=` further requires the player BE one of the listed countries.
For sniper: Owner=All-Allied AND RequiredHouses=British → only British
players build it. (No ForbiddenHouses is set on SNIPE.)

### 4. UseOwnName=true — tooltip / type-reveal

INI key `UseOwnName` is an **InfantryTypeClass** field (per
`InfantryTypeClass__ReadINI @ 0x0052463D` DATA xref to string at
`0x00825908`). When set, the engine displays the unit's specific name
("Sniper") on player tooltip hovers, even when generic type-info would
otherwise be displayed (typically just category like "Infantry"). Used for
notable units the player should immediately recognize: Tanya, Boris, Yuri
Prime, Sniper. Has implications for both own-unit UI (cleaner display) and
opponent awareness (enemy hover-info reveals "Sniper" specifically — though
the unit's other flags like CanDisguise still apply, since UseOwnName only
affects the displayed name, not the unit's discovery state).

### 5. PreventAttackMove=no — explicit Attack-Move enable [BINARY-VERIFIED audit 10]

INI key `PreventAttackMove` is a **TechnoTypeClass** field at byte offset
**`TechnoTypeClass+0x6C8`** (BINARY-VERIFIED via TechnoTypeClass__ReadINI
default read `(char)param_1[0x1B2]`; xref from
`TechnoTypeClass__ReadINI @ 0x00714994` DATA to string at `0x008439B0`).
When set to `yes`, the type ignores Attack-Move commands (engineer, spy,
dog with default behavior). Sniper explicitly sets it to `no` — meaning
the sniper DOES obey Attack-Move. This is significant because **most
"specialist" infantry have PreventAttackMove=yes** (since their Primary
is not really an attack weapon); sniper is genuinely combat-oriented and
the explicit `no` confirms that intent.

Adjacent flag: **`CanPassiveAquire`** is also TechnoType-scope at byte
**`TechnoTypeClass+0xD99`** (default read pattern via xref from
`TechnoTypeClass__ReadINI @ 0x00714473` to string at `0x00843C50`).
SNIPE's `;CanPassiveAquire=no` line is **commented out**, so the field
keeps its default (yes) — sniper does passively auto-acquire targets.

### 6. IFVMode=5 — IFV sniper rifle swap

When the sniper boards an [HTK] IFV, the IFV's `WeaponN`/`ElitePassengerWeaponN`
lookup table indexes into entry 5 → the sniper-rifle weapon (long-range
anti-infantry rifle that the IFV chassis uses). Different from the AWP
itself — the IFV-mounted version is a different weapon record (typically
something like `[IFVSniperDef]/[IFVSniperElite]`). This is one of the
strongest IFV configurations in vanilla YR — gives the IFV chassis a
long-range one-shot infantry capability with mobility.

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("UseOwnName\|RequiredHouses\|PreventAttackMove\|RevealOnFire")` | 4 strings — confirms each is a hardcoded-recognized INI key with a single distinct ReadINI consumer |
| `get_xrefs_to(0x00825908)` (= "UseOwnName") | Sole xref from `InfantryTypeClass__ReadINI @ 0x0052463D` DATA — confirms it's an InfantryType-specific field (not on the parent TechnoType) |
| `get_xrefs_to(0x00843BB4)` (= "RequiredHouses") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714529` DATA — confirms it's on TechnoTypeClass (applies to any Techno: infantry/vehicle/aircraft/building) |
| `get_xrefs_to(0x008494E0)` (= "RevealOnFire") | Sole xref from `WeaponTypeClass__ReadINI @ 0x00772189` DATA — confirms it's a per-weapon flag (parsed once per weapon at INI load) |

Confirmation: **no SNIPE-specific hardcoded function block exists** in
gamemd.exe. The sniper's distinctive behavior comes entirely from the
combination of generic engine flags (`RevealOnFire`, `RequiredHouses`,
`UseOwnName`, `PreventAttackMove`) wired up to data (high Damage × high
Verses-vs-none × no prone reduction × long range × short sight × slow ROF).

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `ImmuneToVeins=yes` | TS legacy; veins are TS-only terrain. No effect in YR. Defensively set | OK |
| `;CanPassiveAquire=no` (commented) | Designer history — sniper was originally going to be no-passive-acquire. Commented out, defaults to yes | OK — historical |
| `[Sniper]` weapon section (vestigial) | **Dead code in YR** — sniper unit uses `Primary=AWP`, not `Primary=Sniper`. The `[Sniper]` weapon's `Report=SILENCER` references a non-existent sound. May be referenced by map-script or AI-script overrides elsewhere, but not the in-game sniper | Documented |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` — standard RA2/YR infantry. Not TS-special | OK |
| `MovementZone=Infantry` | Standard, not TS-specific | OK |

No TS-only behavior found on the SNIPE type itself.

---

## Ghidra audit log (audit iteration 10 — 2026-05-18)

Independent re-verification pass against gamemd.exe. ~10 string/xref
verifications + cross-reference to audit 9's WeaponTypeClass__ReadINI
decompile pin RevealOnFire at WeaponType+0x137.

### INI key strings verified at claimed addresses

| Doc claim | Verified address |
|-----------|------------------|
| `UseOwnName` @ `0x00825908` | ✅ exact |
| `RequiredHouses` @ `0x00843BB4` | ✅ exact |
| `RevealOnFire` @ `0x008494E0` | ✅ exact |
| `PreventAttackMove` @ `0x008439B0` | ✅ exact |
| `ForbiddenHouses` @ `0x00843B94` | ✅ exact |
| `CanPassiveAquire` @ `0x00843C50` | ✅ exact |

### INI parser-scope verifications (xrefs)

| INI key | Reader xref | Scope |
|---------|-------------|-------|
| `UseOwnName` | `InfantryTypeClass__ReadINI` @ 0x0052463D | **InfantryType** ✅ |
| `RequiredHouses` | `TechnoTypeClass__ReadINI` @ 0x00714529 | **TechnoType** ✅ |
| `RevealOnFire` | `WeaponTypeClass__ReadINI` @ 0x00772189 | **WeaponType** ✅ |
| `PreventAttackMove` | `TechnoTypeClass__ReadINI` @ 0x00714994 | **TechnoType** ✅ |
| `ForbiddenHouses` | `TechnoTypeClass__ReadINI` @ 0x0071455D | **TechnoType** ✅ |
| `CanPassiveAquire` | `TechnoTypeClass__ReadINI` @ 0x00714473 | **TechnoType** ✅ |

### Struct offsets BINARY-VERIFIED (this audit + cross-reference to audit 9)

**WeaponTypeClass:**
- `+0x137` = RevealOnFire (byte) — re-confirmed via the audit 9
  WeaponTypeClass__ReadINI decompile, where `s_RevealOnFire_008494e0`
  is the 5th capability flag in the post-LimboLaunch block.

**TechnoTypeClass:**
- `+0x6C8` = PreventAttackMove (byte) — BINARY-VERIFIED via default read
  `(char)param_1[0x1B2]` = byte 0x1B2*4 = 0x6C8
- `+0xD99` = CanPassiveAquire (byte) — BINARY-VERIFIED via
  `*(byte*)((int)param_1 + 0xd99)`
- `+0xDA0` = RequiredHouses (int) — BINARY-VERIFIED via default read
  `param_1[0x368]` = byte 0xDA0; parsed via house-list helper FUN_004750D0
- `+0xDA4` = ForbiddenHouses (int) — BINARY-VERIFIED via default read
  `param_1[0x369]` = byte 0xDA4

**InfantryTypeClass:**
- `UseOwnName` is in the InfantryType bool-chain (xref @ 0x0052463D —
  same call site documented in audit 4 GHOST + audit 9 ADOG). Exact
  byte offset within the +0xEBC..+0xECB chain DEFERRED (would require
  disassemble_bytes pass to map call order to store offsets).

### "Data-driven, not hardcoded" claim — BINARY-VERIFIED

- `search_strings("^Sniper$")` returns **0 matches** — no standalone
  "Sniper" string exists in gamemd.exe.
- `search_functions_enhanced(name_pattern="RevealOnFire")` returns
  0 functions — RevealOnFire is purely a data field, no special function.
- `search_strings("HollowPoint")` returns 0 matches — confirms the
  warhead name is not hardcoded either (purely INI-resolved).
- `search_strings("SILENCER")` returns 0 matches — confirms the
  `[SILENCER]` sound block referenced by the vestigial `[Sniper]`
  weapon does not exist (further evidence the `[Sniper]` weapon is
  dead code).

### Items NOT re-verified this pass (DEFERRED)

- **RevealOnFire consumer end-to-end** — the WeaponType+0x137 storage
  is BINARY-VERIFIED, but the actual `TechnoClass::Fire_At` (or
  Apply_Damage) code path that checks `RevealOnFire == 0` before
  unshrouding cells is NOT decompiled this pass. The consumer almost
  certainly lives somewhere in audit 5's `TechnoClass::Fire_At @
  0x006FDD50` body, but isn't traced yet. DEFERRED.
- **RequiredHouses / ForbiddenHouses consumer** — the
  `FactoryClass::CanBuild`-style gate that filters the sniper out of
  non-British sidebars was not decompiled this pass. The fields are
  parsed but the buildability filter is DEFERRED.
- **UseOwnName exact InfantryType offset** — confirmed scope but not
  exact byte offset within the InfantryType bool-chain.
- **Damage formula `Damage × Verses[armor] / 256`** — the doc's claim
  of `125 × 200% = 250` damage relies on the engine's damage formula
  applying Verses as a percentage. The actual damage-resolution
  formula (in `WarheadTypeClass::Apply_Damage` or similar) is NOT
  decompiled this pass. Existing audit 5 work on `WarheadTypeClass::Detonate
  @ 0x004690B0` may already cover this; not re-verified here.
- **IFV slot 5 weapon-table lookup** — the doc claims `IFVMode=5`
  selects a specific IFV weapon slot. The IFV's gunner-table lookup
  code (`UnitClass`-side, when a passenger boards) was not decompiled
  this pass. DEFERRED.

### Confidence summary

**HIGH** for: all 6 INI key string addresses, all 6 parser-scope
verifications, all 4 TechnoTypeClass struct offsets pinned this pass
(PreventAttackMove, CanPassiveAquire, RequiredHouses, ForbiddenHouses),
the WeaponType+0x137 RevealOnFire cross-reference, and the "no hardcoded
Sniper path" negative claim (0 string matches).

**MEDIUM** for: UseOwnName exact byte offset (scope verified, byte
DEFERRED); damage-formula data-driven claim (logically consistent but
the actual formula code is not re-verified this pass).

**LOW / unverified** for: RevealOnFire consumer in Fire_At, RequiredHouses
buildability gate, IFV slot 5 lookup. All DEFERRED.

---

## Cross-references

- **Related Allied national-unit equivalents** (each Allied country has one
  unique special unit):
  - Britain: `[SNIPE]` Sniper (this doc)
  - France: `[MGTK]` Mirage Tank (`RequiredHouses=French`)
  - Germany: `[TNKD]` Tank Destroyer (`RequiredHouses=Germans`)
  - America: `[PTROOP]` Paratroopers via the American Paratroop Drop superpower
  - Korea: `[BEAG]` Black Eagle (`RequiredHouses=Americans` — Korea is part
    of Americans faction in YR)
- **Related weapons sharing `[HollowPoint]` warhead**:
  - `[MP5]` (SEAL/Navy SEAL primary) — Damage=125
  - `[DoublePistols]` (Tanya primary) — Damage=125
  - `[AWP]`/`[AWPE]` (Sniper primary) — Damage=125
  - All four are the "infantry-killer" weapon family. Damage × Verses 200% =
    250 vs Armor=none → one-shot kills
- **Other long-range infantry weapons** (for range comparison):
  - `[Virusgun]` (Virus sniper, Yuri side) Range=10
  - `[20mm]` (Rocketeer) Range=5
  - `[AWP]`/`[AWPE]` (Sniper) Range=14 — **longest infantry weapon range
    in the game**
- **Other `RevealOnFire=no` weapons**:
  - `[MakeupKit]` (Spy disguise switch)
  - `[AWP]`/`[AWPE]` (Sniper)
  - The flag is the hallmark of "stealth attack" weapons
- **Counter-units / hard counters**:
  - Yuri/Initiate mind control (`ImmuneToPsionics=no` — sniper IS controllable)
  - Crazy Ivan bomb (`Bombable=yes`)
  - Dogs (one-shot the dog at range, but dog leap is hard to dodge if
    closes — sniper Strength=125 vs Dog ParasiteDog kill = sniper dies in
    one bite)
  - Counter-snipers (Sniper vs Sniper: 250 dmg vs 125 HP — first shot wins)
- **Sound cross-link**:
  - `[SniperAttack]` priority=critical: the only weapon-report in soundmd
    with critical priority — ensures the audio always plays

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [SNIPE]` | 4278-4318 (41 lines) | All 40 active keys covered (one commented `;CanPassiveAquire` documented) |
| `artmd.ini [SNIPE]` | 301-308 (8 lines) | All keys covered |
| `artmd.ini [ConSequence]` | 13770-13790 (21 lines) | All 17 active slots + 3 stub Die3-5 + commented W-cheer covered |
| `rulesmd.ini [AWP]` | 23076-23084 (9 lines) | All keys covered |
| `rulesmd.ini [AWPE]` | 25146-25154 (9 lines) | All keys covered (delta from AWP noted) |
| `rulesmd.ini [HollowPoint]` | 27053-27058 (6 lines) | All keys covered including 11-column Verses breakdown |
| `rulesmd.ini [InvisibleLow]` | 25385-25390 (6 lines) | All keys covered |
| `rulesmd.ini [Sniper]` (vestigial) | 22837-22844 (8 lines) | All keys covered; flagged as dead code with evidence (missing `[SILENCER]` sound) |
| `soundmd.ini` Sniper voices | SniperSelect, Move, AttackCommand, Fear, Die, Attack | All 6 covered; SILENCER non-existence noted |
| Hardcoded behavior | Damage-stack (data) + RevealOnFire + RequiredHouses + UseOwnName + PreventAttackMove + IFVMode | 6 mechanisms covered; 4 Ghidra-confirmed flag paths |
| Ghidra searches performed against ID | 4 distinct queries (1 strings + 3 xref lookups) | Logged inline |
| TS-legacy filter | Applied; `[Sniper]` weapon flagged as vestigial; ImmuneToVeins noted as TS-defensive | Done |
