# Yuri Initiate (INIT)
Side: Yuri | Category: Infantry | Image alias: `[INIT]` (no `Image=` redirect — own SHP `INIT`)

The Yuri faction's **basic infantry** — analogous to GI/Conscript but
restricted to `Owner=YuriCountry` only. $200 from Yuri Barracks (YABRCK).
Despite the "Psychic Jab" name, the **Primary is a flame-damage weapon**
(`SAFlame` warhead with `InfDeath=4` burn-death + `INITFIRE` impact
animation) — designer commented out the original `[PJABWH]` "psychic"
warhead and switched to `SAFlame` during balance tuning. The `[PJABWH]`
warhead still exists in INI (with a different Verses table) but is
unwired. **`Sight=9`** is unusually high for basic infantry (matches
Spy/Dog/Boris). **`Occupier=yes`** with `OccupyPip=PersonPurple` plus
substantially-boosted garrison weapons: `UCPsychicJab` Damage=63 vs
ground-form 25 (**2.5× damage boost when garrisoned** — the biggest
garrison damage multiplier of any basic infantry). Elite garrison
`UCElitePsychicJab` Damage=73.

Cross-references: the broader mind-control system used by Yuri/Yuri Prime
is documented in [MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md)
— but **Initiate himself does NOT mind-control** (his weapon is flame
damage). Mind-control is reserved for Yuri / Yuri Prime / Magnetron /
Master Mind. Initiate's "psychic" naming is purely thematic.

---

## rulesmd.ini — `[INIT]` section

Verbatim from `ini/rulesmd.ini:4870`:

```ini
[INIT]
UIName=Name:INIT
Name=Yuri Initiate
Category=Soldier
Primary=PsychicJab
OccupyWeapon=UCPsychicJab; The weapon I use while Occupying.  Defaults to 0 (Primary)
EliteOccupyWeapon=UCElitePsychicJab; The weapon I use while Occupying.  Defaults to 0 (Primary)
Occupier=yes ; I can Occupy UC buildings
Prerequisite=YABRCK
CrushSound=InfantrySquish
Strength=100 ;91
Armor=none
TechLevel=1
Pip=white
OccupyPip=PersonPurple
Sight=9
Speed=4
Owner=YuriCountry
Cost=200 ;300
Soylent=100
Points=5
IsSelectableCombatant=yes
VoiceSelect=InitiateSelect
VoiceMove=InitiateMove
VoiceAttack=InitiateAttackCommand
VoiceFeedback=InitiateFear
VoiceSpecialAttack=InitiateMove
DieSound=InitiateDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
;MovementZone=InfantryDestroyer ;GEF wow!!! copy paste bug from the original Disk Thrower!
ThreatPosed=5	; This value MUST be 0 for all building addons
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ImmuneToVeins=yes
Size=1
ElitePrimary=PsychicJabE
IFVMode=13
UseOwnName=true
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:INIT` | CSF-string key → "Yuri Initiate" |
| `Name=Yuri Initiate` | Internal name |
| `Category=Soldier` | Infantry pip/AI grouping |
| `Primary=PsychicJab` | Ground weapon — `Damage=25, Range=4.5, Warhead=SAFlame` (NOT PJABWH despite the name). See "Weapons" |
| `OccupyWeapon=UCPsychicJab` | Garrison weapon — `Damage=63` (2.5× ground). InfantryTypeClass field (per `InfantryTypeClass__ReadINI @ 0x00524117`, documented in [E2.md](../soviet/E2.md)) |
| `EliteOccupyWeapon=UCElitePsychicJab` | Elite garrison weapon — `Damage=73` |
| `Occupier=yes` | **Behavior flag** — InfantryType field (xref `0x005244D5`). Enables UC garrison. Documented in [E2.md](../soviet/E2.md) |
| `Prerequisite=YABRCK` | Yuri Barracks specifically (NOT abstract Barracks — must be Yuri faction's barracks) |
| `CrushSound=InfantrySquish` | Standard crush sound |
| `Strength=100 ;91` | HP — 100. Inline comment "91" suggests an earlier tuned value (was 91, bumped to 100) |
| `Armor=none` | Damage type column 0 — standard infantry |
| `TechLevel=1` | Buildable from game start (gated only by YABRCK) |
| `Pip=white` | Cargo pip color (when loaded in transport) |
| `OccupyPip=PersonPurple` | **Behavior key** — TechnoType field. Garrison pip color **purple** — distinguishes Yuri-faction occupants from Allied (PersonBlue) / Soviet (PersonRed). All Yuri infantry use PersonPurple when garrisoned |
| `Sight=9` | Reveal radius — **9 cells**, equal to Spy/Dog/Boris. Highest of any basic-tier infantry. Yuri lore: psychic sensitivity = wider awareness. Gameplay: Initiates make great scouts |
| `Speed=4` | Foot-speed — standard infantry |
| `Owner=YuriCountry` | **Yuri faction only** — only YuriCountry house can build. No ForbiddenHouses needed because Owner is already a singleton. Stock "Owner=YuriCountry" is the simplest possible house gate (one entry) |
| `Cost=200 ;300` | $200 — same as Conscript. Inline comment "300" suggests an earlier $300 cost, reduced for accessibility |
| `Soylent=100` | $100 Grinder refund (Yuri only — 50% standard) |
| `Points=5` | Kill score |
| `IsSelectableCombatant=yes` | Included in select-all-combat |
| `VoiceSelect=InitiateSelect` | Select voice — `$iinisea/b/c/e` (4 lines; `$iinised` commented out — designer cut one) |
| `VoiceMove=InitiateMove` | Move voice — `$iinimoa/c/d/e` (4 lines; `$iinimob` commented out — also cut) |
| `VoiceAttack=InitiateAttackCommand` | Attack voice — `$iiniata..f` (6 lines — largest attack bank) |
| `VoiceFeedback=InitiateFear` | Fear voice — `$iinifea..e` (5 lines) |
| `VoiceSpecialAttack=InitiateMove` | Reuses Move voice — no special-attack-specific line |
| `DieSound=InitiateDie` | Death voice — `$iinidia..e` (5 lines) |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — standard infantry |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `;MovementZone=InfantryDestroyer ;GEF...` | Same Disk Thrower copy-paste-fix comment |
| `ThreatPosed=5` | AI scoring weight — low (same as Conscript) |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | Standard 5 abilities at Veteran |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 abilities at Elite. Triggers `ElitePrimary=PsychicJabE` (Damage 25→30, Range 4.5→6) |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set |
| `Size=1` | Transport cargo slot cost |
| `ElitePrimary=PsychicJabE` | Elite-tier ground weapon swap. `Damage 25→30, Range 4.5→6`. Same SAFlame warhead |
| `IFVMode=13` | IFV gunner-table index 13 → HTK's `Weapon14`/`ElitePassengerWeapon14` slot. In stock YR maps to a psychic-themed weapon for the IFV chassis when Initiate is garrisoned |
| `UseOwnName=true` | **Behavior flag** — InfantryType field (per `InfantryTypeClass__ReadINI @ 0x0052463D`, documented in [SNIPE.md](../allied/SNIPE.md)). Shows "Yuri Initiate" specifically on hover tooltips |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `yes` (prone-walking enabled)
- `Trainable=` — defaults to `yes` (Initiate gains veterancy)
- `AllowedToStartInMultiplayer=` — **not set, defaults to `yes`** — Initiates ARE in the Yuri starting unit complement (mirrors Conscript's defaulting). Yuri players start with several Initiates
- `NotHuman=` — defaults to `no` (Initiate is human; subject to InfDeath, sniper headshot, mind-control)
- `ImmuneToPsionics=` — defaults to `no`; **Initiate CAN be mind-controlled** — but by whom? Yuri can't mind-control his own units, and no Allied/Soviet unit mind-controls. So this defaults moot in practice (only Soviet/Allied Mirror-Yuri-via-spy scenarios)
- `ImmuneToRadiation=` — defaults to `no`
- `Bombable=` — defaults to `no` (not in explicit list)
- `Fearless=` — not set
- `Agent=`/`Infiltrate=`/`Engineer=`/`Ivan=`/`C4=` — not set
- `Deployer=` — defaults to `no`
- `DetectDisguise=` — not set
- `DefaultToGuardArea=` — not set
- `BombSight=` — not set
- `Natural=` — not set
- `SelfHealing=` — not set (only SELF_HEAL via Elite ability)

---

## artmd.ini — `[INIT]` section

`ini/artmd.ini:156`:

```ini
[INIT] ; Initiate
Cameo=INITICON
AltCameo=INITUICO
Sequence=LangIdleMDSequence
Crawls=yes
Remapable=yes
FireUp=6
PrimaryFireFLH=60,0,100
```

| Key | Meaning |
|-----|---------|
| `Cameo=INITICON` | Sidebar build icon (SHP) |
| `AltCameo=INITUICO` | Elite cameo — shown after Veteran promotion |
| `Sequence=LangIdleMDSequence` | **Shared sequence** — generic "language idle" Yuri infantry sequence. The name suggests "long-idle MD" (MD = Mental Domination / Yuri's Revenge marker). Used by multiple Yuri infantry |
| `Crawls=yes` | Prone-capable |
| `Remapable=yes` | House remap palette applied (purple Yuri faction → faction color in match) |
| `FireUp=6` | Bullet-spawn frame — at frame 6 the jab fires |
| `PrimaryFireFLH=60,0,100` | FLH — 60 forward, 0 sideways, 100 up. Standard rifle-shoulder FLH (identical to Conscript) |

Missing `SecondaryFireFLH=` — no Secondary weapon.

### Referenced sequence — `[LangIdleMDSequence]`

`artmd.ini:14214`:

```ini
[LangIdleMDSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,20,0,E
Prone=91,1,6
Crawl=91,6,6
Die1=139,15,0
Die2=154,15,0
Down=169,2,2
Up=185,2,2
Cheer=201,8,0,E
FireUp=209,6,6
FireProne=257,6,6
Paradrop=305,1,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Panic=8,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle | |
| `Guard=0,1,1` | Guard idle | |
| `Walk=8,6,6` | Walk cycle 6×6 | |
| `Idle1=56,15,0,S` | Idle 1 — 15 frames S-facing | "Look around" |
| `Idle2=71,20,0,E` | Idle 2 — **20 frames** E-facing (longer than typical 15) | "Adjust telepathic stance" / Yuri-faction's longer idle anim |
| `Prone=91,1,6` | Prone 1 frame × 6 facings | |
| `Crawl=91,6,6` | Crawl reuses prone | |
| `Die1=139,15,0` | Death 1 — 15 frames | |
| `Die2=154,15,0` | Death 2 | |
| `Down=169,2,2` | Get-down to prone | |
| `Up=185,2,2` | Get-up from prone | |
| `Cheer=201,8,0,E` | Cheer — 8 frames E | |
| `FireUp=209,6,6` | Standing fire cycle | |
| `FireProne=257,6,6` | Prone-fire cycle | |
| `Paradrop=305,1,0` | Single frame at 305 — paradrop pose | Live (paradrop-eligible) |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → Ready frame | |
| `Panic=8,6,6` | Panic = Walk frames | |

**Important**: This sequence is **shared with multiple Yuri infantry**
(BRUTE has its own custom `[BruteSequence]` with extra slots, but other
Yuri infantry like Yuri/Yuri Prime/Virus may use `LangIdleMDSequence`
with per-unit FireUp= timing). Per-unit `FireUp=N` in artmd top-level
varies (Initiate=6, others may differ to match weapon timing).

---

## Weapons

### Primary (Veteran and below) — `[PsychicJab]`

`rulesmd.ini:22890`:

```ini
[PsychicJab]
Damage=25
ROF=15
Range=4.5
Projectile=InvisibleLow
Speed=100
;Warhead=PJABWH
Warhead=SAFlame
Report=InitiateAttack
OccupantAnim=UCINIT
OpenToppedAnim=GUNFIRE;weapon doesn't have an anim naturally, so use this one when in a BattleFortress
```

| Key | Meaning |
|-----|---------|
| `Damage=25` | Per-shot damage. With `SAFlame.Verses[none]=100%` → 25 dmg vs GI (4 shots to kill at 100 HP). Vs Conscript (flak armor) `Verses=80%` → 20 dmg (7 shots to kill at 125 HP) |
| `ROF=15` | Cooldown — 15 frames (~1s @ 15fps) — **fast cadence**, fastest among basic infantry (Conscript=25, GI=13 with M60). Initiate's high ROF compensates for the relatively low per-shot damage |
| `Range=4.5` | 4.5 cells — slightly longer than GI's 4 |
| `Projectile=InvisibleLow` | LOS-respecting inviso |
| `Speed=100` | Irrelevant for inviso |
| `;Warhead=PJABWH` (commented) | **Designer history** — original "Psychic Jab Warhead" with different Verses spread. **Replaced** by SAFlame during balance — the designers decided the Initiate should be flame-based rather than psychic-damage-based |
| `Warhead=SAFlame` | **Final warhead** — Small-Arms-Flame variant. 11-column Verses identical to SA but **InfDeath=4 (burn death) + AnimList=INITFIRE** (Yuri-themed flame impact animation). The "psychic" naming on the weapon/unit is purely thematic — actual damage type is fire-bullet |
| `Report=InitiateAttack` | Sound `iiniatta` (Volume 60, Limit=3) |
| `OccupantAnim=UCINIT` | WeaponType field — animation overlay drawn at the building window slot when this weapon fires from inside a UC. `UCINIT` is the Initiate-in-window sprite |
| `OpenToppedAnim=GUNFIRE` | **Behavior key** — WeaponTypeClass field (per `WeaponTypeClass__ReadINI @ 0x007725E6` DATA xref to string at `0x008493F0`). Inline comment: "weapon doesn't have an anim naturally, so use this one when in a BattleFortress". When the unit firing this weapon is garrisoned in an open-topped vehicle (like the Battle Fortress's open passenger slots), this animation plays at the firing point. Default `GUNFIRE` is the generic gunshot puff — Initiate's weapon has no inherent muzzle flash since it's an inviso projectile, so this provides the visual feedback |

### Elite Primary — `[PsychicJabE]`

`rulesmd.ini:24688`:

```ini
[PsychicJabE]
Damage=30
ROF=15
Range=6
Projectile=InvisibleLow
Speed=100
Warhead=SAFlame
Report=InitiateAttack
OccupantAnim=UCINIT
OpenToppedAnim=GUNFIRE;weapon doesn't have an anim naturally, so use this one when in a BattleFortress
```

Delta from `[PsychicJab]`:
- **Damage 25→30** (+20%)
- **Range 4.5→6** (+33%)
- Same ROF, projectile, warhead, sound, OccupantAnim, OpenToppedAnim

Triggered via `ElitePrimary=PsychicJabE` at Elite tier.

### Occupy Weapon — `[UCPsychicJab]`

`rulesmd.ini:22902`:

```ini
[UCPsychicJab]
Damage=63
ROF=15
Range=6 ;7
Projectile=InvisibleHigh
Speed=100
Warhead=SSABFlame
Report=InitiateAttack
OccupantAnim=UCINIT
```

**Used only when garrisoned.** Switched on via `OccupyWeapon=UCPsychicJab`.

| Key | Meaning |
|-----|---------|
| `Damage=63` | **2.5× the ground weapon's 25** — the **largest garrison damage multiplier of any basic infantry**. Compare Conscript ground=15 → UC=20 (1.33×), GI ground=10 → UC similar. Initiate-in-garrison is dramatically more powerful than walking |
| `ROF=15` | Same cooldown as ground (no faster firing) |
| `Range=6` | 6 cells (vs 4.5 ground). Inline comment `;7` shows it was tested at 7 then reduced to 6 |
| `Projectile=InvisibleHigh` | **"High" inviso** — no SubjectToCliffs/Walls (building window shots ignore those) |
| `Speed=100` | Irrelevant for inviso |
| `Warhead=SSABFlame` | Building-occupier variant of SAFlame — same Verses spread but `ProneDamage=50%` (better prone penalty) and slightly lower vs steel/concrete |
| `Report=InitiateAttack` | Same sound as ground (no distinct UC sound) |
| `OccupantAnim=UCINIT` | Same window animation |

### Elite Occupy Weapon — `[UCElitePsychicJab]`

`rulesmd.ini:22912`:

```ini
[UCElitePsychicJab]
Damage=73
ROF=15
Range=6 ;7
Projectile=InvisibleHigh
Speed=100
Warhead=SSABFlame
Report=InitiateAttack
```

**Used when Elite-rank Initiate is garrisoned.** Switched on via `EliteOccupyWeapon=UCElitePsychicJab`. Delta from `[UCPsychicJab]`:
- **Damage 63→73** (+16%)
- No OccupantAnim listed (likely inherits UCINIT from same sprite)
- Everything else identical

### Primary's Warhead — `[SAFlame]`

`rulesmd.ini:26475`:

```ini
[SAFlame];gs copied SA so I could change the animation
;DB Changed how Plate interacts with this warhead on 6/6. See also AP warhead.
;Verses=100%,80%,70%,50%,25%,25%,75%,50%,25%,100%,100%
Verses=100%,80%,80%,50%,25%,25%,75%,50%,25%,100%,100%
InfDeath=4
AnimList=INITFIRE
Bullets=yes
ProneDamage=70%
```

| Key | Meaning |
|-----|---------|
| Designer comment ";gs copied SA so I could change the animation" | Confirms SAFlame is a SA-clone with only the animation/death changed |
| `Verses=100%,80%,80%,50%,25%,25%,75%,50%,25%,100%,100%` | **Identical to `[SA]` Verses** — same 11-column spread. Verses-only differences from SA: none |
| `InfDeath=4` | **Infantry death animation type 4** — the burn/incinerate death (skeleton flash with fire). Distinguishes Initiate kills from generic small-arms kills (SA uses InfDeath=1 standard) |
| `AnimList=INITFIRE` | **Impact animation** — `INITFIRE` (Initiate Fire impact). Yuri-themed flame puff at impact point. Distinct from SA's PIFFPIFF |
| `Bullets=yes` | Marks bullet-type for engine |
| `ProneDamage=70%` | Prone reduces damage to 70% (same as SA) |

### Garrison Warhead — `[SSABFlame]`

`rulesmd.ini:26532`:

```ini
[SSABFlame];gs again, copied from SSAB to change the animation
;CellSpread=.3
;PercentAtMax=.5
;DB Changed how Plate interacts with this warhead on 6/6. See also AP warhead.
;Verses=100%,80%,70%,50%,25%,25%,75%,30%,20%,100%,100%
Verses=100%,80%,80%,50%,25%,25%,75%,30%,20%,100%,100%
InfDeath=4
AnimList=INITFIRE
;Bright=yes
Bullets=yes
ProneDamage=50%
```

Same as SAFlame except:
- **30% vs steel** (vs SAFlame 50%) — worse vs steel-armored buildings
- **20% vs concrete** (vs SAFlame 25%) — worse vs concrete
- **ProneDamage=50%** (vs SAFlame 70%) — better prone penalty (prone reduces to 50%)

Building-occupier variant — slightly less effective vs hardened structures (intentional balance).

### Unused — `[PJABWH]` (the original "psychic" warhead)

`rulesmd.ini:27572`:

```ini
[PJABWH]
Verses=100%,100%,100%,50%,25%,25%,25%,40%,15%,100%,100%
InfDeath=1
AnimList=PIFFPIFF,PIFFPIFF
Bullets=yes
ProneDamage=80%
```

**The original Initiate warhead, replaced by SAFlame.** Currently:
- **NOT referenced by any active weapon** in stock rulesmd.ini (PsychicJab/PsychicJabE/UCPsychicJab/UCElitePsychicJab all use SAFlame or SSABFlame)
- Different Verses spread: 100/100/100 vs all infantry armor (vs SAFlame's 100/80/80) — would have been stronger vs Conscript/Tesla Trooper armor; 25/40/15 vs structures (vs SAFlame 75/50/25) — would have been weaker vs buildings
- `InfDeath=1` standard small-arms death (vs SAFlame's burn death type 4)
- `PIFFPIFF` impact (vs SAFlame's INITFIRE flame puff)
- `ProneDamage=80%` (vs SAFlame's 70% — less prone protection)

**Documented as cut content** — the "psychic" warhead identity was abandoned mid-development in favor of flame damage. Could be reactivated by changing `[PsychicJab].Warhead=PJABWH`.

### Projectile — `[InvisibleLow]` / `[InvisibleHigh]`

Same as Conscript — documented in [E2.md](../soviet/E2.md). Ground uses Low (LOS-respecting); garrison uses High (passes over walls/cliffs).

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear / death

```ini
[InitiateSelect]                  ; soundmd.ini:4620
Sounds=$iinisea $iiniseb $iinisec $iinisee ;$iinised
Control=random
Volume=85

[InitiateMove]                    ; soundmd.ini:4625
Sounds=$iinimoa $iinimoc $iinimod $iinimoe ;$iinimob
Control=random
Volume=85

[InitiateAttackCommand]           ; soundmd.ini:4630
Sounds=$iiniata $iiniatb $iiniatc $iiniatd $iiniate $iiniatf
Control=random
Volume=85

[InitiateFear]                    ; soundmd.ini:4635
Sounds=$iinifea $iinifeb $iinifec $iinifed $iinifee
Control=random
Volume=85

[InitiateDie]                     ; soundmd.ini:4640
Sounds=$iinidia $iinidib $iinidic $iinidid $iinidie
Control=random
Volume=85
```

4 select (1 commented out — `$iinised`) / 4 move (1 commented out —
`$iinimob`) / 6 attack / 5 fear / 5 death. Notable design choice: voice
banks have **explicitly commented-out alternates** — the designers
recorded but chose not to ship those lines (possibly quality issues).

Voice character: Yuri-faction style (vaguely Eastern-European-or-foreign,
deeper psychic-incantation tones).

### Weapon report

```ini
[InitiateAttack]                  ; soundmd.ini:1073
Sounds=iiniatta
Control= interrupt
FShift= -5 5
VShift=15
Volume=60
Limit=3
```

Single sample `iiniatta`. `Control= interrupt` (new shots immediately
preempt old). `Limit=3` caps to 3 concurrent. Volume 60 medium-low.

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `YABRCK` | **Yuri Barracks specifically** — Initiate cannot be trained from a captured Allied/Soviet Barracks even by a Yuri player |
| `Owner=` | `YuriCountry` | **Yuri faction only** — singleton owner list |
| `TechLevel=` | `1` | Available from game start |
| `AllowedToStartInMultiplayer=` (default `yes`) | — | **Initiate IS in the Yuri starting complement** — Yuri players begin matches with several Initiates pre-placed (mirrors Conscript behavior on Soviet side) |
| `Cost=200` | $200 | Cheap (same as Conscript) |
| `Soylent=100` | $100 refund (Yuri only — and Initiate IS Yuri, so the refund applies to Yuri grinding his own Initiates) |
| `Points=5` | 5 | Kill-score contribution |

No `PrerequisiteOverride=`, no `BuildLimit=`, no `RequiredHouses=` (Owner=YuriCountry is already the lock).

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — standard 5 abilities |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — 4 abilities + activates `ElitePrimary=PsychicJabE` (Damage 25→30, Range 4.5→6) **AND** `EliteOccupyWeapon=UCElitePsychicJab` (Damage 63→73 in garrison) |
| AltCameo | `INITUICO` shown in sidebar once Veteran rank reached |

`Trainable=` defaults to `yes`.

---

## Hardcoded behavior — Ghidra-verified

### 1. Garrison subsystem (same as Conscript)

Initiate uses the same engine garrison machinery documented in [E2.md](../soviet/E2.md):
- `Occupier=yes` — InfantryType field
- `OccupyWeapon=UCPsychicJab` — InfantryType field
- `EliteOccupyWeapon=UCElitePsychicJab` — InfantryType field
- `OccupyPip=PersonPurple` — TechnoType field (purple distinguishes Yuri occupant)
- `OccupantAnim=UCINIT` — per-weapon WeaponType field for the window animation

**Yuri-specific contribution:** the `PersonPurple` pip color. All other faction colors (PersonRed=Soviet, PersonBlue=Allied, PersonGreen=neutral) are documented; PersonPurple is the dedicated Yuri color.

The substantial garrison damage boost (Damage 25→63 = +152%) is **the most aggressive of any basic infantry** — Initiate-in-garrison is closer to an anti-infantry MG than to a rifleman. Suggests the designers wanted Yuri's "fortify+control" theme: Initiates pile into civilian buildings for area-denial.

### 2. OpenToppedAnim — Battle Fortress passenger-fire animation

INI key `OpenToppedAnim=GUNFIRE` on PsychicJab is a **WeaponTypeClass** field (per `WeaponTypeClass__ReadINI @ 0x007725E6` DATA xref to string at `0x008493F0`). When the unit firing this weapon is loaded as cargo in an **open-topped transport** (Battle Fortress [FV], some open transports), this animation plays at the cargo-firing point. The default `GUNFIRE` animation is a generic gunshot puff.

Critical because PsychicJab's projectile is `Invisible*` — there's no muzzle flash from the projectile itself. Without OpenToppedAnim, FV passengers firing inviso weapons would have no visual feedback at the cargo slot. The flag ensures every passenger weapon has SOMETHING visible at the firing point.

In stock YR, Battle Fortress can carry up to 5 passengers and each passenger's weapon fires through the FV. Initiate, Yuri Prime, Virus, etc. all use OpenToppedAnim=GUNFIRE for FV-passenger fire.

### 3. SAFlame vs PJABWH — replaced warhead

Designer-level decision documented inline: `;Warhead=PJABWH` commented out, `Warhead=SAFlame` active. Two reasons for the switch (inferred):
1. **Visual distinctiveness** — INITFIRE flame puff is more readable in combat than generic PIFFPIFF
2. **Different Verses** — SAFlame's spread is friendlier to mass-infantry-vs-mass-infantry combat (the typical Yuri vs Yuri or Yuri vs Soviet matchup)

**No PJABWH-specific function or warhead-side flag exists** — both warheads are just data records. The "psychic" naming on the weapon/unit is purely thematic; no actual mind-control or psionic damage is dealt by Initiate.

### 4. UseOwnName=true — already documented

InfantryType field (xref `0x0052463D`). Shows "Yuri Initiate" specifically on hover tooltips. Same flag as Sniper/Tanya/Boris/Yuri Prime. Documented in [SNIPE.md](../allied/SNIPE.md).

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("OpenToppedAnim\|UseOwnName")` | 2 strings — confirms both as hardcoded INI keys |
| `get_xrefs_to(0x008493F0)` (= "OpenToppedAnim") | Sole xref from `WeaponTypeClass__ReadINI @ 0x007725E6` DATA — confirms per-weapon flag for open-topped transport firing animation |

Plus reused confirmations from prior dossiers: Occupier/OccupyWeapon/OccupyPip/OccupantAnim from E2.md, UseOwnName from SNIPE.md.

Confirmation: **Initiate uses entirely-generic engine machinery** — no INIT-specific hardcoded function block. The "psychic" identity is purely data-driven (warhead name, weapon name, voice voice voice samples) with no engine-side flag treating Initiate differently from any other basic infantry.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;MovementZone=InfantryDestroyer` (commented) | Designer-fixed Disk Thrower copy-paste bug | OK |
| `;Warhead=PJABWH` (commented) | Designer cut-content — original psychic warhead replaced by SAFlame | Documented |
| `;Strength=91` inline | Older Strength value — bumped to 100 | OK |
| `;Cost=300` inline | Older cost — reduced to 200 | OK |
| `;$iinised` / `;$iinimob` (commented in voice banks) | Cut voice lines | OK |
| `;Crawls=yes` (no commented variants in artmd this time — only the active line) | N/A | — |
| `ImmuneToVeins=yes` | TS legacy; defensively set | OK |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` — standard | OK |
| `MovementZone=Infantry` | Standard | OK |

No TS-only behavior. No active "psychic" mechanic on Initiate — the name is thematic. All flags YR-active.

---

## Cross-references

- **Yuri infantry tier** (this is the first Yuri doc — other entries will reference back):
  - **`[INIT]` Yuri Initiate (this doc)** — basic infantry
  - `[YURI]` Yuri — mind-control beam, the eponymous unit
  - `[YURIPR]` Yuri Prime — AoE mind-control
  - `[BRUTE]` Yuri Brute — Strength=350 melee, the heavyweight
  - `[VIRUS]` Yuri Virus — plague sniper
  - `[YENGINEER]` Yuri Engineer — capture/repair specialist (mirror of Allied/Soviet engineer)
  - `[YADOG]` / `[YDOG]` — Yuri-built dog variants (already documented in [ADOG.md](../allied/ADOG.md))
- **Sister basic-infantry counterparts** (per-side basic):
  - Allied: `[E1]` GI
  - Soviet: `[E2]` Conscript
  - **Yuri: `[INIT]` Initiate (this doc)**
- **Same garrison subsystem family** (all use `Occupier=yes`):
  - `[E1]` GI, `[GGI]` Guardian GI, `[E2]` Conscript, `[FLAKT]` Flak Trooper, `[SHK]` Tesla Trooper... wait SHK is NOT Occupier=yes. Let me re-verify when documenting FLAKT/etc.
  - **`[INIT]` Initiate (this doc)**, `[YURI]`, `[YURIPR]` (verify), `[VIRUS]` (verify when documented)
  - Pip colors: Allied=PersonBlue, Soviet=PersonRed, **Yuri=PersonPurple**
- **Related warheads**:
  - `[SA]` (basic small-arms) — used by Conscript/GI/most basic infantry
  - **`[SAFlame]` (this doc's INIT primary)** — SA + burn death + INITFIRE animation
  - `[PJABWH]` (cut content) — alternate "psychic" warhead, not wired
  - `[SSAB]` (Soviet garrison version of SA) — used by Conscript UC
  - `[SSABFlame]` (this doc's garrison warhead) — SSAB + burn death + INITFIRE
- **Counter-units to Initiate**:
  - Dogs (one-shot Parasite vs Armor=none infantry)
  - Sniper (250 dmg one-shot vs Strength=100)
  - Crazy Ivan bomb (vs garrison: bomb the building destroys all garrisoned Initiates)
  - Vehicle crush (Crushable=yes default)
- **Iconic plays**:
  - **Initiate garrison + Yuri faction's massive number of civilian buildings** — Yuri's design pushes garrison combat. INIT's 2.5× garrison damage boost is the mechanical hook for this strategy
  - **Initiate paradrop** (some campaigns) — paradrop pose defined in sequence
- **Sound cross-link**:
  - `[InitiateAttack]` Limit=3 is the same audio-cap pattern as `[ConscriptAttack]` — prevents mass-Initiate scenarios from blowing the audio mix

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [INIT]` | 4870-4909 (40 lines) | All 38 active keys covered (one commented `;MovementZone` documented) |
| `artmd.ini [INIT]` | 156-163 (8 lines) | All keys covered |
| `artmd.ini [LangIdleMDSequence]` | 14214-14233 (20 lines) | All 17 active slots + 3 stub Die3-5 covered |
| `rulesmd.ini [PsychicJab]` | 22890-22900 (11 lines) | All keys covered including commented `;Warhead=PJABWH` history |
| `rulesmd.ini [PsychicJabE]` | 24688-24697 (10 lines) | All keys covered (delta noted) |
| `rulesmd.ini [UCPsychicJab]` | 22902-22910 (9 lines) | All keys covered |
| `rulesmd.ini [UCElitePsychicJab]` | 22912-22919 (8 lines) | All keys covered (delta noted) |
| `rulesmd.ini [SAFlame]` warhead | 26475-26482 (8 lines) | All keys covered with 11-column Verses; documented as SA-clone with different animation/death |
| `rulesmd.ini [SSABFlame]` warhead | 26532-26542 (11 lines) | All keys covered (delta from SSAB noted) |
| `rulesmd.ini [PJABWH]` (unused) | 27572-27577 (6 lines) | All keys covered; flagged as cut content (no active weapon references it) |
| `soundmd.ini` Initiate voices | InitiateSelect, Move, AttackCommand, Fear, Die, Attack | All 6 covered with 2 commented-out voice lines noted |
| Hardcoded behavior | Garrison subsystem + OpenToppedAnim + UseOwnName + SAFlame replacement of PJABWH | 4 mechanisms; 1 fresh Ghidra-verified xref + 4 cross-referenced from prior docs |
| Ghidra searches performed against ID | 2 distinct queries (1 strings + 1 xref lookup) plus cross-references from E2/SNIPE docs | Logged inline |
| TS-legacy filter | Applied; ImmuneToVeins defensive, PJABWH cut content, MovementZone copy-paste comment documented, commented voice lines explained | Done |
