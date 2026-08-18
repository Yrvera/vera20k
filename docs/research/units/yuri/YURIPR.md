# Yuri Prime (YURIPR)
Side: Yuri | Category: Infantry | Image alias: `Image=YURIX` → `[YURIX]` artmd

The **Yuri Prime hero unit** — Yuri faction's analogue to Tanya / Boris.
$1500 from Yuri Barracks + Yuri Battle Lab (`YABRCK,YATECH`).
**`BuildLimit=1`** (one per player at a time). Two-stage mind-control hero:
**`Primary=SuperMindControl`** uses `Warhead=ControllerBuilding` (the
all-100% Verses variant) — meaning **Yuri Prime CAN mind-control buildings**
(unlike basic Yuri Clone whose Controller warhead is 0% vs structures).
**`Secondary=SuperPsiWave`** with `Warhead=SuperPsiPulse` (CellSpread=**5**
vs basic Yuri's 3, and **100% vs vehicles** in addition to infantry) —
the deployed psychic blast kills both infantry AND tanks in a wider area.
**`UndeployDelay=75`** (half of basic Yuri's 150) — Yuri Prime can blast
twice as often. **`ImmuneToPsionicWeapons=yes`** — fully immune to ALL
psionic damage (basic Yuri only has `ImmuneToPsionics`). **`SpeedType=Amphibious`**
+ **`MovementZone=AmphibiousDestroyer`** — Yuri Prime walks on water like
Tanya. **`Speed=6`** (vs basic Yuri's 4). **`Unnatural=yes`** — engine
flag marking Yuri Prime as opposite of "natural" (cows/Brutes), affecting
some AI heuristics.

Authoritative deep RE for mind-control:
[MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md).

---

## rulesmd.ini — `[YURIPR]` section

Verbatim from `ini/rulesmd.ini:5246`:

```ini
[YURIPR]
UIName=Name:YuriPrime
Name=Yuri Prime
Image=YURIX
Category=Soldier
Prerequisite=YABRCK,YATECH;Yuri Prime is now the high end yuri
Primary=SuperMindControl
Secondary=SuperPsiWave
OpenTransportWeapon=1;defaults to -1 (decide normally)  What weapon should I use in a Battle Fortress
CrushSound=InfantrySquish
Crushable=no
TiberiumProof=yes
Strength=150
Armor=flak
TechLevel=10
Pip=red
PixelSelectionBracketDelta=-26;gs higher number draws lower.  Pixel difference from normal for selection bracket
Sight=9
Speed=6
Owner=YuriCountry
AllowedToStartInMultiplayer=no
Cost=1500
Soylent=750
Points=50
IsSelectableCombatant=yes
VoiceSelect=YuriPrimeSelect
VoiceMove=YuriPrimeMove
VoiceAttack=YuriPrimeAttackCommand
VoiceFeedback=YuriPrimeFear
VoiceSpecialAttack=YuriPrimeMove
DieSound=YuriPrimeDie
MoveSound=YuriPrimeMoveLoop
CreateSound=YuriPrimeCreated
;Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1};
;MovementZone=Infantry

;SpeedType=Hover
;Locomotor={4A582742-9839-11d1-B709-00A024DDAFD1}
;MovementZone=Amphibious ; gs AMphibiousDestroyer I can't have a destroyer zone without a weapon!
;gs Correct in theory, but Hover only works properly for units.

SpeedType=Amphibious
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
MovementZone=AmphibiousDestroyer

PhysicalSize=1
ThreatPosed=25	; This value MUST be 0 for all building addons
SpecialThreatValue=1
ImmuneToVeins=yes
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ImmuneToPsionics=yes
ImmuneToPsionicWeapons=yes ;gs Patch
Deployer=yes
DeployFire=yes
UndeployDelay=75
Size=1
BuildLimit=1
;CanPassiveAquire=no ; Won't try to pick up own targets
IFVMode=15
Unnatural=yes
SelfHealing=yes
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:YuriPrime` | CSF-string key → "Yuri Prime" |
| `Name=Yuri Prime` | Internal name |
| `Image=YURIX` | **Art redirect** — rendering uses `[YURIX]` artmd entry (not `[YURIPR]`). YURIX = "Yuri Extended"/"Yuri Prime" SHP on disk |
| `Category=Soldier` | Infantry pip/AI grouping |
| `Prerequisite=YABRCK,YATECH` | Yuri Barracks + Yuri Battle Lab specifically. Inline comment: "Yuri Prime is now the high end yuri" — designed as the faction's tech-9-equivalent hero |
| `Primary=SuperMindControl` | Mind-control weapon variant — **uses `Warhead=ControllerBuilding`** (the all-100% Verses variant). Critical difference from basic Yuri's `[MindControl]` (Warhead=Controller, 0% vs structures): **Yuri Prime CAN mind-control buildings**. See "Weapons" |
| `Secondary=SuperPsiWave` | Deployed psychic blast — `Warhead=SuperPsiPulse` (CellSpread=5 vs basic PsiPulse's 3, **100% vs vehicles** in addition to infantry). See "Weapons" |
| `OpenTransportWeapon=1` | When loaded in Battle Fortress as cargo, the Battle Fortress fires Yuri Prime's **Secondary** (weapon index 1 — note: basic Yuri Clone doesn't have this, defaults to -1 "engine decides"). Forcing Secondary means BF-with-YuriPrime fires the PsiWave area blast — an area-attack weapon from a mobile platform |
| `CrushSound=InfantrySquish` | Moot — Crushable=no |
| `Crushable=no` | **Cannot be crushed** by vehicles. Same as Boris/Tesla Trooper/Desolator |
| `TiberiumProof=yes` | TS-legacy hazard immunity (no observable effect in YR — no Tiberium terrain) |
| `Strength=150` | HP — 150 (vs Yuri Clone's 100). 50% more than basic Yuri. Tougher hero |
| `Armor=flak` | Damage type column 1 — flak armor (vs Yuri Clone's `none`). Better protection from small arms |
| `TechLevel=10` | **Tech-10 cap** — matches Yuri Clone. Effectively no tech-level gate; controlled by Prerequisite chain |
| `Pip=red` | Cargo pip color — red (elite class) |
| `PixelSelectionBracketDelta=-26` | **Behavior key** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x00714166` DATA xref to string at `0x00843DC0`). Adjusts the selection-bracket Y position by N pixels. Inline comment: "higher number draws lower. Pixel difference from normal for selection bracket". **-26 = bracket drawn 26 pixels higher** than default. Used to accommodate the taller Yuri Prime sprite (the unit floats with a psychic aura — bracket needs to enclose the visible model) |
| `Sight=9` | Reveal radius — 9 (vs Yuri Clone's massive 12 — Yuri Prime has SMALLER sight). Interesting design choice: Yuri Prime is the close-engage hero, not a scout. Compensates with greater speed |
| `Speed=6` | **Foot-speed — 6** (vs Yuri Clone's 4). 50% faster. Hero-tier mobility (matches Tanya's 5, Boris's 5 — Yuri Prime is actually FASTER than Tanya) |
| `Owner=YuriCountry` | Yuri faction only — singleton owner list |
| `AllowedToStartInMultiplayer=no` | Not in starting unit complement |
| `Cost=1500` | $1500 — same as Boris (the costliest infantry tier) |
| `Soylent=750` | $750 Grinder refund (Yuri only — and YuriPR IS Yuri, so the refund applies to Yuri grinding his own hero) |
| `Points=50` | **Kill score 50** — matches SEAL/Boris (highest infantry tier) |
| `IsSelectableCombatant=yes` | Included in select-all-combat |
| `VoiceSelect=YuriPrimeSelect` | Select voice — `$iyupsea..f` (6 lines) |
| `VoiceMove=YuriPrimeMove` | Move voice — `$iyupmoa..g` (**7 lines**, the largest move bank of any infantry) |
| `VoiceAttack=YuriPrimeAttackCommand` | Attack voice — `$iyupata..g` (7 lines — tied for largest attack bank) |
| `VoiceFeedback=YuriPrimeFear` | Fear voice — `$iyupfea/b/c` (3 lines) |
| `VoiceSpecialAttack=YuriPrimeMove` | Reuses Move voice — no dedicated special-attack line |
| `DieSound=YuriPrimeDie` | Death voice — `$iyupdia..d` (4 lines) |
| `MoveSound=YuriPrimeMoveLoop` | **Looping move SFX** — `iyuplo1a/2a/2b/2b/3a` (5 samples crossfaded, Control=loop random all decay attack, Limit=2). Continuous psychic hum while Yuri Prime moves. Same pattern as Rocketeer's MoveLoop |
| `CreateSound=YuriPrimeCreated` | Build-completion sound — `$iyupcrd` single line, **`Type=global Priority=critical MinVolume=95 Volume=95`** — extremely loud global broadcast when Yuri Prime finishes training. Both players hear it |
| `;Locomotor={4A582744-...};` (commented) | Original infantry locomotor — replaced (still infantry locomotor, but with different MovementZone) |
| `;MovementZone=Infantry` (commented) | Original infantry MZ — replaced for amphibious |
| `;SpeedType=Hover` / `;Locomotor={4A582742-...}` / `;MovementZone=Amphibious` (commented block with designer notes) | **Designer history** — Yuri Prime was tested as a Hover-type unit. Inline notes: "AMphibiousDestroyer I can't have a destroyer zone without a weapon!" and "Correct in theory, but Hover only works properly for units." Hover-infantry was buggy; reverted |
| `SpeedType=Amphibious` | **Active SpeedType** — uses the Amphibious column in the speed-terrain table. Can walk on water (with appropriate speed multiplier vs land) |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — **still infantry locomotor**, NOT amphibious-specific. The water-walking is achieved via SpeedType + MovementZone + art Swim frames, NOT via a special locomotor |
| `MovementZone=AmphibiousDestroyer` | **Amphibious movement zone** — pathfinder treats this MZ as "can cross water cells". Same MZ used by Navy SEAL (`[GHOST]`). Both can wade through water |
| `PhysicalSize=1` | Pathfinder size class |
| `ThreatPosed=25` | AI scoring weight — high (matches Boris, SEAL) |
| `SpecialThreatValue=1` | Self-threat max — Yuri Prime "wants" mind-control targets maximally |
| `ImmuneToVeins=yes` | TS legacy; defensively set |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | Standard 5 at Veteran |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 at Elite. **No ElitePrimary** — Yuri Prime doesn't change weapon at Elite (matches basic Yuri) |
| `ImmuneToPsionics=yes` | Immune to mind-control (per [MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md) §2.3 — TechnoTypeClass+0xD35) |
| `ImmuneToPsionicWeapons=yes` | **Behavior flag** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x00714FC8` DATA xref to string at `0x0084373C`). Inline comment "gs Patch" — added in an update. **Stronger than ImmuneToPsionics**: blocks ALL psionic damage (including Psychic Dominator blast area damage, PsiPulse, SuperPsiPulse). Boris has ImmuneToPsionics=yes but ImmuneToPsionicWeapons=no (can be killed by Dominator blast); **Yuri Prime is immune to BOTH** — fully psionic-proof |
| `Deployer=yes` | Enables deploy command (InfantryTypeClass field) |
| `DeployFire=yes` | Deploying swaps weapon to Secondary (TechnoTypeClass field) |
| `UndeployDelay=75` | **75 frames (~5s @ 15fps)** — half of basic Yuri Clone's 150. Yuri Prime can deploy-blast more frequently. Designer balance: hero unit gets faster blast cycle |
| `Size=1` | Transport cargo slot cost |
| `BuildLimit=1` | **One Yuri Prime per player at a time**. Production queue rejects further orders until the existing one dies. Same as Boris/Tanya |
| `;CanPassiveAquire=no` (commented) | Defensive — defaults to yes (Yuri Prime DOES passively acquire MC targets in MissionGuard) |
| `IFVMode=15` | IFV gunner-table index 15 → HTK's `Weapon16` slot. In stock YR maps to a powerful psychic anti-vehicle weapon when Yuri Prime is garrisoned |
| `Unnatural=yes` | **Behavior flag** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x00714960` DATA xref to string at `0x008439D0`). Marks the unit as "unnatural" — opposite of `Natural=yes` (Cows, Brutes, animals). Affects some AI heuristics around target prioritization and possibly the "Yuri's creations" tagging for certain warhead effects. Yuri Prime is explicitly Unnatural — the clone-of-clone genetic-engineering theme |
| `SelfHealing=yes` | Passive HP regen (same as Boris/Desolator). Cap = Strength=150 |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `yes` (prone-walking enabled)
- `Trainable=` — defaults to `yes` (Yuri Prime gains veterancy)
- `NotHuman=` — defaults to `no` (human; subject to InfDeath, sniper headshot)
- `ImmuneToRadiation=` — defaults to `no` (radiation kills Yuri Prime — Desolator hard-counter)
- `Bombable=` — defaults to `no` (Crazy Ivan cannot bomb Yuri Prime with auto-detect, but can Bomb-mission)
- `Fearless=` — not set; Yuri Prime CAN show fear (rare given other flags)
- `Occupier=` — defaults to `no`; **Yuri Prime CANNOT garrison** civilian buildings
- `Agent=`/`Infiltrate=`/`Engineer=`/`Ivan=`/`C4=` — not set
- `Assaulter=` — not set
- `BombSight=` — not set
- `DetectDisguise=` — **NOT set** (basic Yuri Clone has it, but Yuri Prime does NOT — interesting design choice; perhaps the hero is too busy controlling to also detect disguises)
- `DefaultToGuardArea=` — not set
- `Natural=` — not set (Unnatural=yes is the inverse flag)
- `TypeImmune=` — **NOT set** (basic Yuri Clone has TypeImmune=yes; Yuri Prime does NOT — meaning **Yuri Prime can be mind-controlled by another Yuri Prime** (theoretically — but BuildLimit=1 per player + Owner=YuriCountry means this only happens in Yuri-vs-Yuri MP). Both have ImmuneToPsionics=yes which is the stronger lock, but TypeImmune is absent on the hero)
- `SecretHouses=` — NOT set (only Owner=YuriCountry singleton, which serves the same role for the hero)
- `PrerequisiteOverride=` — not set (no Kremlin Palace bypass for the hero)

---

## artmd.ini — `[YURIX]` section

`ini/artmd.ini:339`:

```ini
[YURIX] ; Yuri Prime
Cameo=YYPRICON;YURPICON
AltCameo=YYPRUICO
Sequence=YuriXSequence
Crawls=yes
Remapable=yes
FireUp=6
PrimaryFireFLH=10,0,195
SecondaryFireFLH=10,0,195 ; SJM: brain blast should come from head, not feet
```

| Key | Meaning |
|-----|---------|
| `Cameo=YYPRICON;YURPICON` | Sidebar build icon — active `YYPRICON`, commented alternate `YURPICON` (older naming) |
| `AltCameo=YYPRUICO` | Elite cameo |
| `Sequence=YuriXSequence` | Reference to `[YuriXSequence]` — Yuri-Prime-specific sequence with deploy frames AND wet/swim frames |
| `Crawls=yes` | Prone-capable (defensively — rarely used given Speed=6 and AoE blast strategy) |
| `Remapable=yes` | House remap palette applied |
| `FireUp=6` | Bullet-spawn frame |
| `PrimaryFireFLH=10,0,195` | Primary FLH — 10 forward, 0 sideways, **195 up**. Z=195 is **higher than basic Yuri's 140** — Yuri Prime's sprite is taller (the "ascended/floating" pose), and the beam emanates from a higher head position |
| `SecondaryFireFLH=10,0,195 ; SJM: brain blast should come from head, not feet` | Same FLH as Primary — same head-height for the deploy blast |

### Referenced sequence — `[YuriXSequence]`

`artmd.ini:14429`:

```ini
[YuriXSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Prone=86,1,6
Crawl=86,6,6
Die1=134,20,0
Die2=154,15,0
FireUp=169,6,6
FireProne=217,6,6
Down=260,2,2
Up=276,2,2
Deploy=265,7,0
Deployed=272,2,0 ; middle frame of deploy
Undeploy=274,6,0
;Deploy=292,7,0
;Deployed=299,2,0 ; middle frame of deploy
;Undeploy=301,6,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Cheer=280,8,0,E
Panic=8,6,6
Paradrop=0,1,1

Tread=0,1,1 ;gs to make Yuri prime go over water, he needs to be like Tanya; just fool the swim frames and it will work perfectly
Swim=8,6,6
WetAttack=169,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle | |
| `Guard=0,1,1` | Guard idle | |
| `Walk=8,6,6` | Walk cycle 6×6 | |
| `Idle1=56,15,0,S` | Idle 1 — 15 frames S | |
| `Idle2=71,15,0,E` | Idle 2 — E | |
| `Prone=86,1,6` | Prone 1 frame × 6 facings | |
| `Crawl=86,6,6` | Crawl reuses prone | |
| `Die1=134,20,0` | Death 1 — **20 frames** (longer than typical 15) — dramatic hero-death animation | |
| `Die2=154,15,0` | Death 2 — 15 frames | |
| `FireUp=169,6,6` | Standing fire — MC beam pose | |
| `FireProne=217,6,6` | Prone-fire | |
| `Down=260,2,2` | Get-down to prone | |
| `Up=276,2,2` | Get-up from prone | |
| `Deploy=265,7,0` | **Deploy anim** — 7 frames at 265. Plays when deploying for SuperPsiWave blast | |
| `Deployed=272,2,0 ; middle frame of deploy` | Held pose — 2 frames at 272 (middle of deploy block) | |
| `Undeploy=274,6,0` | Undeploy anim — 6 frames at 274 | |
| `;Deploy=292,7,0` `;Deployed=299,2,0` `;Undeploy=301,6,0` | Commented older deploy frames from basic Yuri | Replaced with hero-specific frames at 265-274 |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → Ready | |
| `Cheer=280,8,0,E` | Cheer | |
| `Panic=8,6,6` | Panic = Walk | |
| `Paradrop=0,1,1` | **Stub — Yuri Prime falls back to Ready frame** when paradropped | He has no paradrop-specific art (defensive entry) |
| `Tread=0,1,1` | **Water-tread frame** — stub falls back to Ready. Inline comment: "gs to make Yuri prime go over water, he needs to be like Tanya; just fool the swim frames and it will work perfectly". The "trick" is that Tread is reused as the walking-on-water visual; since it's a stub, Yuri Prime just shows his Ready pose while on water (not a distinct swim animation) |
| `Swim=8,6,6` | **Swim cycle** — reuses Walk frames. Same designer trick: Yuri Prime "swims" by appearing to walk on water |
| `WetAttack=169,6,6` | **Wet-attack cycle** — reuses FireUp frames. Yuri Prime can fire while on water with the same animation as on land |

The Tread/Swim/WetAttack triplet is what enables the amphibious behavior visually — without these, `MovementZone=AmphibiousDestroyer` would still let Yuri Prime pathfind across water, but the sprite would be missing frames and render incorrectly. Same trick as Tanya's sequence.

---

## Weapons

### Primary — `[SuperMindControl]` (the building-capable mind-control)

`rulesmd.ini:24051`:

```ini
[SuperMindControl]
Damage=1;Number of mind control links
ROF=200
Range=7
Projectile=PsychicControl
Speed=100
Warhead=ControllerBuilding
;Report=YuriMindControl
Anim=YURICNTL
FireOnce=yes
```

| Key | Meaning |
|-----|---------|
| `Damage=1` | Same as basic [MindControl] — **link count, not damage**. 1 simultaneous MC link |
| `ROF=200` | 200 frames cooldown — same as basic Yuri |
| `Range=7` | 7 cells — same as basic Yuri |
| `Projectile=PsychicControl` | Same inviso projectile |
| `Speed=100` | Irrelevant for inviso |
| `Warhead=ControllerBuilding` | **THE distinguishing change** — uses ControllerBuilding (Verses 100% across ALL 11 armor columns) instead of basic Yuri's Controller (Verses 0% vs structures). **Yuri Prime CAN mind-control buildings** — capture a refinery, war factory, super-weapon — while controlling the building it produces for Yuri's house, draws power from Yuri's grid, etc. Major strategic implication |
| `;Report=YuriMindControl` (commented) | Same as basic — sound played by global success-trigger, not per-shot |
| `Anim=YURICNTL` | Same Yuri-control animation |
| `FireOnce=yes` | One shot per command |

### Secondary — `[SuperPsiWave]` (the wide-area deploy blast)

`rulesmd.ini:24098`:

```ini
[SuperPsiWave]
Damage=250;Needed to be considered offensive unit
Range=1
ROF=50 ;200 needs to be closer to animation time (Kills everything anyway)
Projectile=Psychic
Speed=1
Warhead=SuperPsiPulse
AreaFire=yes ; just shoot straight at ground under feet
FireOnce=yes ; Only fire once; don't stay in attack mission
Report=YuriDeploy
Anim=RING1
```

Compared to basic Yuri's `[PsiWave]`:
- **Same Damage 250 / Range 1 / ROF 50 / AreaFire / FireOnce / Speed 1**
- **Different Warhead: SuperPsiPulse vs basic PsiPulse** — SuperPsiPulse has wider spread (CellSpread=5 vs 3) and damages vehicles (50% Verses vs basic's 0%)
- **Has Report=YuriDeploy** (basic Yuri's PsiWave has no Report). `YuriDeploy` plays `iyurat2a` — the deploy thump sound
- **Has Anim=RING1** — weapon-level animation `RING1` plays at Yuri Prime's position during the blast. Same RING1 used by Terrorist's TerrorBomb. Visual reinforcement

### Primary's Warhead — `[ControllerBuilding]` (already documented)

Already documented in [YURI.md](YURI.md). Recap:

```ini
[ControllerBuilding];Mind control warhead.  Will skip normal damage like EMP did
Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%
MindControl=yes
AnimList=YURICNTL
```

**ALL 11 columns = 100%** — including wood/steel/concrete structure armors.
Yuri Prime can target buildings with the MC cursor and capture them via
mind-control rather than the engineer-capture path.

### Secondary's Warhead — `[SuperPsiPulse]`

`rulesmd.ini:27177`:

```ini
[SuperPsiPulse]
CellSpread=5
PercentAtMax=.85
Verses=100%,100%,100%,50%,50%,50%,0%,0%,0%,0%,0%
InfDeath=6
PsychicDamage=yes ;gs psychic, but not mind control
AffectsAllies=no; Defaults to yes.
```

| Key | Meaning |
|-----|---------|
| `CellSpread=5` | **5 cells** vs basic PsiPulse's 3. Roughly 11×11 cell area at full coverage — devastating |
| `PercentAtMax=.85` | At spread edge, damage is 85% (same as basic) — minimal falloff |
| `Verses=100%,100%,100%,50%,50%,50%,0%,0%,0%,0%,0%` | **100% vs infantry (none/flak/plate)** PLUS **50% vs light/medium/heavy vehicle**. **The key upgrade**: SuperPsiPulse kills infantry AND damages tanks (125 dmg/tank vs Damage=250 with 50% Verses). Basic PsiPulse was 0% vs vehicles. **0% vs buildings** (wood/steel/concrete) — still cannot damage structures. 0% on specials |
| `InfDeath=6` | "Blown to bits" death — same as basic PsiPulse |
| `PsychicDamage=yes` | Psychic damage flag (kills without controlling) |
| `AffectsAllies=no` | Inline comment "Defaults to yes." Yuri Prime's blast does NOT damage his own allied units. Critical for deploying amid Yuri's infantry blob |

### Projectiles — `[PsychicControl]` and `[Psychic]`

Same as basic Yuri Clone — documented in [YURI.md](YURI.md).

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear / death (large hero voice bank)

```ini
[YuriPrimeSelect]                  ; soundmd.ini:5164
Sounds= $iyupsea $iyupseb $iyupsec $iyupsed $iyupsee $iyupsef
Control=random
Volume=85

[YuriPrimeMove]                    ; soundmd.ini:5169
Sounds=  $iyupmoa $iyupmob $iyupmoc $iyupmod $iyupmoe $iyupmof $iyupmog
Control=random
Volume=85

[YuriPrimeAttackCommand]           ; soundmd.ini:5174
Sounds= $iyupata $iyupatb $iyupatc $iyupatd $iyupate $iyupatf $iyupatg
Control=random
Volume=85

[YuriPrimeFear]                    ; soundmd.ini:5179
Sounds= $iyupfea $iyupfeb $iyupfec
Control=random
Volume=85

[YuriPrimeDie]                     ; soundmd.ini:5196
Sounds= $iyupdia $iyupdib $iyupdic $iyupdid
Control=random
Volume=85
```

**6 select / 7 move / 7 attack / 3 fear / 4 death** — among the largest
voice banks of any infantry. The 7 move + 7 attack lines tie with Ivan's
7-line select bank for largest single-bank.

### Extra voice — `[YuriPrimePsyResist]` (psychic-resist voice)

```ini
[YuriPrimePsyResist]               ; soundmd.ini:5184
Sounds= $iyuprea $iyupreb $iyuprec $iyupred
Control=random
Volume=85
```

**Unique to Yuri Prime — no other infantry has this**. Plays when Yuri
Prime is the target of a psionic attack (Yuri Clone's MindControl, Psychic
Tower, etc.) and his `ImmuneToPsionicWeapons=yes` blocks it. 4 alternate
lines play "I resist your puny mind-control" style dialogue. **No
hardcoded mechanism documented in standard INI keys** — likely triggered
by a special engine path when ImmuneToPsionicWeapons blocks a psionic
weapon. Not wired via standard `Voice*=` fields on the type.

### Creation (build-complete) — global broadcast

```ini
[YuriPrimeCreated]                 ; soundmd.ini:5189
Sounds= $iyupcrd
Type=global
Priority=critical
MinVolume=95
Volume=95
```

**Volume=95 + MinVolume=95** — by far the loudest creation sound in the
game (typical is 80). `Type=global Priority=critical` means **all players
hear it loudly** when Yuri Prime finishes training. Critical strategic
warning: a Yuri Prime is on the field. Compare Boris's CreateSound which is
also Type=global but MinVolume=80.

### Engine move loop

```ini
[YuriPrimeMoveLoop]                ; soundmd.ini:5247
Sounds= iyuplo1a iyuplo2a iyuplo2b iyuplo2b iyuplo3a
Control= loop random all decay attack
Priority=Low
Limit=2
Range=15
Volume=35
```

5 samples crossfaded continuously (with `iyuplo2b` deduped). Plays while
Yuri Prime moves — psychic ambient hum. `Limit=2` allows 2 concurrent (since
BuildLimit=1, this matters only when multiple players have Yuri Primes on
field). `Range=15` audible within 15 cells.

### Deploy report

```ini
[YuriDeploy]                       ; soundmd.ini:1192
Sounds=iyurat2a
```

Single sample for SuperPsiWave's `Report=YuriDeploy`. Basic Yuri Clone
does NOT have a deploy Report (his PsiWave is silent at the weapon
level). Yuri Prime explicitly adds a deploy SFX — auditory signal for
the bigger blast.

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `YABRCK,YATECH` | Yuri Barracks + Yuri Battle Lab. Inline comment notes Yuri Prime is "the high end yuri" |
| `Owner=` | `YuriCountry` | Yuri faction only — singleton (no SecretHouses needed) |
| `TechLevel=` | `10` | Maximum tech-level cap |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=1500` | $1500 | Same as Boris (most expensive infantry tier) |
| `Soylent=750` | $750 refund (Yuri only) | |
| `Points=50` | 50 | Among highest infantry point values |
| `BuildLimit=1` | — | **One Yuri Prime per player at a time** |

No `PrerequisiteOverride=`, no `SecretHouses=`, no `RequiredHouses=` (Owner singleton suffices).

The hero-unit-per-faction lineup:
- Allied: `[TANY]` Tanya ($1000, BuildLimit=1)
- Soviet: `[BORIS]` Boris ($1500, BuildLimit=1)
- **Yuri: `[YURIPR]` Yuri Prime ($1500, BuildLimit=1, this doc)**

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — standard 5 abilities |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — 4 abilities. **No ElitePrimary** — mind-control weapon doesn't change at Elite. The Elite tier mostly benefits SuperPsiWave damage (via FIREPOWER) and survivability (via SELF_HEAL stack) |
| AltCameo | `YYPRUICO` shown after Veteran promotion |

`Trainable=` defaults to `yes`.

---

## Hardcoded behavior — Ghidra-verified

### 1. Building-capable mind-control (the SuperMindControl trick)

The hardcoded mechanic for "mind-control can/cannot target buildings" is
**not on the weapon or unit — it's on the warhead's Verses spread**:

- Basic Yuri Clone's `[MindControl]` uses `Warhead=Controller` (Verses
  100% on infantry/vehicles, **0% on wood/steel/concrete**) → MC cursor
  filters out buildings (engine refuses 0-damage targets)
- Yuri Prime's `[SuperMindControl]` uses `Warhead=ControllerBuilding`
  (Verses **100% across ALL 11 armor columns**, including structures) →
  MC cursor lights up on buildings too

The CaptureManagerClass mechanic itself (per
[MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md))
handles building captures via the same MCNode linked-list infrastructure
as unit captures — the building's owner changes to Yuri's house, the
yellow swirl anim plays atop the building. When Yuri Prime dies, captured
buildings revert (same as captured units).

**Strategic implication**: Yuri Prime captures of war factories /
refineries / superweapon buildings are FAR more impactful than capturing
units — the building produces for Yuri's economy, contributes to his power
grid, etc.

### 2. AreaFire SuperPsiWave with wider CellSpread

Same hardcoded mechanism as basic Yuri's PsiWave but with bigger numbers:
- `AreaFire=yes` on the weapon (fires at own cell)
- `Warhead=SuperPsiPulse` with `CellSpread=5` (vs basic PsiPulse's 3)
- `Verses=100/100/100/50/50/50/0/0/0/0/0` (vs basic PsiPulse's 100/100/100/0/0/0/0/0/0/0/0)
- `PsychicDamage=yes` (real damage, not MC)
- `AffectsAllies=no` (no friendly fire)
- `Damage=250` × `Verses` per target

Yuri Prime's blast kills infantry AND damages vehicles (125 dmg/tank in
the inner radius), where basic Yuri Clone's PsiWave is infantry-only.

### 3. ImmuneToPsionicWeapons=yes — full psionic immunity

INI key `ImmuneToPsionicWeapons` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x00714FC8` DATA xref to string at `0x0084373C`).
**Stronger than `ImmuneToPsionics`** which only blocks mind-control:
ImmuneToPsionicWeapons blocks ALL psionic damage including:
- Psychic Dominator superweapon blast area damage
- PsiPulse / SuperPsiPulse area blasts
- Possibly Magnetron lift effects (verify when MAGNETRON is documented)
- Any warhead with `PsychicDamage=yes`

Boris has ImmuneToPsionics=yes but ImmuneToPsionicWeapons=**no** — Boris
can be killed by Dominator blast. Yuri Prime has BOTH — fully psionic-proof
hero. The "gs Patch" inline comment shows this was added in an update,
likely to address Yuri Prime being one-shotted by his own faction's
Dominator in certain scenarios.

### 4. Unnatural=yes — opposite of Natural

INI key `Unnatural` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x00714960` DATA xref to string at `0x008439D0`).
Marks the unit as "unnatural" — designed/cloned/genetically-engineered.
Inverse flag to `Natural=yes` (Cow, Brute, animals). Affects:
- AI threat-pick heuristics for which targets to prioritize
- Possibly some warhead-vs-Natural filters (e.g., Genetic Mutator
  superweapon converts "natural" things to Brutes; Unnatural blocks?)
- Certain "destroy all natural life" map scripts

Yuri Prime fits the unnatural theme — a clone of Yuri, genetically
engineered.

### 5. PixelSelectionBracketDelta=-26 — taller-sprite bracket adjustment

INI key `PixelSelectionBracketDelta` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x00714166` DATA xref to string at `0x00843DC0`).
Adjusts the Y position of the unit's selection bracket by N pixels.
Negative = bracket drawn higher (above the sprite); positive = lower.

Yuri Prime's value of -26 puts the bracket 26 pixels above the default
position — accommodates the taller "floating Yuri Prime" sprite. Without
this, the bracket would clip through the sprite's upper half.

Used by other units with non-standard sprite heights (Kirov Airship is
the canonical big-sprite case; verify when ZEP is documented).

### 6. UndeployDelay=75 (half basic Yuri)

Same TechnoTypeClass field as basic Yuri (xref `0x00714BA8` per
[YURI.md](YURI.md)). Yuri Prime's 75 frames (~5s) is exactly half of basic
Yuri's 150. Design: hero gets faster blast cycle. Practically allows
chaining SuperPsiWave blasts as Yuri Prime walks through enemy formations.

### 7. Amphibious infantry (SpeedType + MovementZone + Wet frames)

Three-part hardcoded combo (no single flag):
- `SpeedType=Amphibious` — uses Amphibious column in terrain speed table
- `MovementZone=AmphibiousDestroyer` — pathfinder treats this MZ as
  water-crossing
- `Sequence=YuriXSequence` with Tread/Swim/WetAttack slots present (even
  as stubs/reuses) — render system needs these slots to not crash on
  water cells

The combination matches Tanya / SEAL — Yuri Prime is one of only three
infantry that can walk on water. Designer comment in the artmd
(`Tread=0,1,1 ;gs to make Yuri prime go over water, he needs to be like
Tanya; just fool the swim frames and it will work perfectly`) confirms
this is intentional and a deliberate copy of Tanya's pattern.

### 8. CreateSound global broadcast

`CreateSound=YuriPrimeCreated` triggers a `Type=global Priority=critical
MinVolume=95 Volume=95` broadcast. Unique to hero units (Yuri Prime,
Boris, presumably Tanya — verify). Engine-level broadcast bypasses normal
camera-distance audio falloff.

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("Unnatural\|PixelSelectionBracketDelta\|ImmuneToPsionicWeapons")` | 3 strings — confirms all 3 hardcoded keys |
| `get_xrefs_to(0x0084373C)` (= "ImmuneToPsionicWeapons") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714FC8` DATA — confirms TechnoType-level full-psionic-immunity flag |
| `get_xrefs_to(0x008439D0)` (= "Unnatural") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714960` DATA — confirms TechnoType-level "unnatural entity" flag |
| `get_xrefs_to(0x00843DC0)` (= "PixelSelectionBracketDelta") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714166` DATA — confirms TechnoType-level selection-bracket Y-adjustment |

Plus reused confirmations from prior dossiers: ImmuneToPsionics, Deployer/
DeployFire, UndeployDelay, TiberiumProof, MindControl warhead flag,
ControllerBuilding warhead, PsychicDamage warhead flag.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `TiberiumProof=yes` | TS legacy (no Tiberium in YR); defensively set | OK |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set | OK |
| `;Locomotor={4A582744-...}` / `;MovementZone=Infantry` (commented) | Designer history — original infantry-only locomotor before amphibious change | OK |
| `;SpeedType=Hover` / `;Locomotor={4A582742-...}` (commented block) | Designer history — Yuri Prime was tested as Hover-type, reverted (inline note: "Hover only works properly for units") | OK |
| `;Deploy=292,7,0` (commented in artmd) | Older deploy frames inherited from basic Yuri | OK |
| `;CanPassiveAquire=no` (commented) | Defensive — defaults to yes | OK |
| `Tread=0,1,1` / `Swim=8,6,6` / `WetAttack=169,6,6` | YR-active (amphibious infantry support) — designer note confirms intent | OK |
| Mind-control / CaptureManager system | Fully YR-active | OK |
| `Unnatural=yes` | YR-active (TechnoType field, verified ReadINI xref) | OK |
| `ImmuneToPsionicWeapons=yes` | "gs Patch" — added in an update, YR-active | OK |

No TS-only behavior on Yuri Prime. All flags YR-active.

---

## Cross-references

- **Yuri infantry tier**:
  - `[INIT]` Yuri Initiate (documented) — basic flame infantry, no MC
  - `[YURI]` Yuri Clone (documented) — single-target MC, 100% vs no-buildings
  - **`[YURIPR]` Yuri Prime (this doc)** — MC + building MC, AoE blast, amphibious
  - `[BRUTE]` Brute — Strength=350 melee
  - `[VIRUS]` Virus — plague sniper
  - `[YENGINEER]` Yuri Engineer — capture/repair
- **Hero-unit-per-faction**:
  - Allied: `[TANY]` Tanya ($1000, C4 buildings)
  - Soviet: `[BORIS]` Boris ($1500, airstrike)
  - **Yuri: `[YURIPR]` Yuri Prime ($1500, building-MC, AoE blast, this doc)**
- **Amphibious infantry trio**:
  - `[GHOST]` Navy SEAL — Allied amphibious commando
  - `[TANY]` Tanya — Allied amphibious hero
  - **`[YURIPR]` Yuri Prime — Yuri amphibious hero (this doc)**
- **MC weapon ladder** (Damage = link count):
  - `[MindControl]` (Yuri Clone) — Damage=1, Warhead=Controller (no buildings)
  - **`[SuperMindControl]` (Yuri Prime, this doc) — Damage=1, Warhead=ControllerBuilding (all targets including buildings)**
  - `[MultipleMindControlTower]` (Psychic Tower) — Damage=3, 3 links, Warhead=Controller
  - `[MultipleMindControlTank]` (Master Mind) — Damage=3 + InfiniteMindControl=yes, unlimited with overload, Warhead=Controller
- **Psychic AoE blast ladder**:
  - `[PsiWave]` (Yuri Clone) — CellSpread=3, infantry only
  - **`[SuperPsiWave]` (Yuri Prime, this doc) — CellSpread=5, infantry + vehicles**
- **Building-capable warheads**:
  - `[ControllerBuilding]` (this doc, Yuri Prime Primary) — used by Yuri Prime to MC buildings
  - `[Controller]` (basic Yuri) — 0% vs buildings, cannot
- **Same `ImmuneToPsionicWeapons=yes` family** (full psionic immunity):
  - `[YURIPR]` Yuri Prime (this doc) — only stock infantry with this flag (verify when other Yuri-hero units / Magnetron etc. are documented)
- **Counter-units to Yuri Prime**:
  - **Snipers** (one-shot via 250 dmg vs Strength=150, NOT blocked by psionic immunity)
  - **Crazy Ivan** bomb (Bombable defaults to no, but Bomb mission still works)
  - **Long-range bombardment** (V3, Apocalypse, Dreadnought) outranges his Primary's 7
  - **Dogs** (Parasite warhead, NOT psionic) — but Yuri Prime can MC the dog first via SuperMindControl
  - **NOT effective**: any mind-control, any psionic weapon, vehicle crush (Crushable=no), most direct fire from infantry-tier weapons (Strength 150 + SelfHealing + flak armor)
- **Sound cross-link**:
  - `[YuriPrimePsyResist]` (PsyResist voice) — unique to Yuri Prime, plays on psionic-attack block. No other infantry has equivalent
- **Related global rules**: same MC system globals as basic Yuri (Rules.YuriMindControlSound, MindClearedSound, ControlledAnimationType, etc.)

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [YURIPR]` | 5246-5307 (62 lines) | All 50 active keys covered (8 commented locomotor/zone/passive comments documented) |
| `artmd.ini [YURIX]` | 339-347 (9 lines) | All keys covered |
| `artmd.ini [YuriXSequence]` | 14429-14458 (30 lines) | All 25 active slots + 3 commented older deploy + 3 stub Die3-5 + designer inline notes covered; Tread/Swim/WetAttack amphibious trick explained |
| `rulesmd.ini [SuperMindControl]` | 24051-24060 (10 lines) | All keys covered |
| `rulesmd.ini [SuperPsiWave]` | 24098-24108 (11 lines) | All keys covered |
| `rulesmd.ini [ControllerBuilding]` | Cross-referenced to YURI.md | Already documented; Yuri Prime uses this instead of basic Controller |
| `rulesmd.ini [SuperPsiPulse]` | 27177-27183 (7 lines) | All keys covered with 11-column Verses breakdown |
| `soundmd.ini` Yuri Prime voices | YuriPrimeSelect, Move, AttackCommand, Fear, Die, Created, PsyResist (unique), MoveLoop, YuriDeploy | All 9 covered |
| Hardcoded behavior | Building-MC via ControllerBuilding + AoE SuperPsiPulse + ImmuneToPsionicWeapons + Unnatural + PixelSelectionBracketDelta + UndeployDelay (half) + amphibious 3-part combo + global CreateSound | 8 mechanisms with 3 fresh Ghidra xrefs + 5 cross-referenced from prior docs |
| Ghidra searches performed against ID | 4 distinct queries (1 strings + 3 xref lookups) | Logged inline |
| TS-legacy filter | Applied; TiberiumProof/ImmuneToVeins defensive, all commented designer-history documented including the Hover-locomotor exploration | Done |
