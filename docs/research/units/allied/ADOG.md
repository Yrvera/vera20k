# Allied Attack Dog (ADOG)
Side: Allied | Category: Infantry | Image alias: `[ADOG]` (no `Image=` redirect — own SHP `ADOG`)

The Allied Attack Dog. $200 anti-infantry unit from the Barracks. Single-purpose
unit: **leap-bite** every infantry it can reach (Range=1.5 cells, jump-onto-target
projectile with `Parasite=yes` warhead) — the warhead's `Verses=100/100/100/0%/...`
restricts targeting to infantry-armor classes only, and the **`LimboLaunch=yes`**
weapon flag means the dog itself becomes the projectile (it's removed from the
map during the leap, then re-spawns at the target). Combined with the per-warhead
Parasite mechanic, this produces the "instant-kill any infantry on contact" feel
of Attack Dogs. **`DetectDisguise=yes`** reveals nearby spies and Mirage Tanks.
**`NotHuman=yes`** + **`ImmuneToPsionics=yes`** make dogs immune to mind-control
(Yuri/Yuri Prime / Psychic Sensor cannot grab dogs). Cannot enter IFV
(`IFVMode=0` is the literal "no swap" slot — IFV gunner 0 retains the chassis's
own default weapon, not the dog).

The Allied Attack Dog shares the same INI template (40 keys identical) as the
Soviet Attack Dog `[DOG]`, the Yuri-built Soviet variant `[YDOG]` (`Image=DOG`),
and the Yuri-built Allied variant `[YADOG]` (`Image=ADOG`). Only differences are
the weapon (`GoodTeeth` for Allied/`YADOG` vs `BadTeeth` for Soviet/`YDOG`),
voice and cameo file names, and house gating. **This doc is canonical for the
attack-dog family — Soviet/Yuri-variant docs should reference here for shared
behavior.**

No standalone dog/parasite RE doc previously existed; this document originates
the Ghidra trace of the bite/leap mechanism.

---

## rulesmd.ini — `[ADOG]` section

Verbatim from `ini/rulesmd.ini:3767`:

```ini
[ADOG]
UIName=Name:DOG
Name=Allied Attack Dog
NotHuman=yes
Category=Soldier
Primary=GoodTeeth
Secondary=VirtualScanner
NavalTargeting=6
Prerequisite=Barracks
LeadershipRating=7
CrushSound=InfantrySquish
Strength=100
Armor=none
ReselectIfLimboed=yes ; If selected when limbo and attacking infantry, reseect when unlimbo
RejoinTeamIfLimboed=yes ; If in a team when limbo shooting infantry, write it down and try to rejoin when unlimbo
DefaultToGuardArea=yes ; the much awaited dog default to move and attack when resting
TechLevel=2
Pip=white
Sight=9
DetectDisguise=yes
Speed=8
Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance
ForbiddenHouses=Russians,Confederation,Africans,Arabs,YuriCountry
Cost=200
Soylent=100
Points=10
IsSelectableCombatant=yes
VoiceSelect=DogSelect
VoiceMove=DogMove
VoiceAttack=DogAttackCommand
VoiceFeedback=DogFear
VoiceSpecialAttack=DogMove
DieSound=DogDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
;MovementZone=InfantryDestroyer ;GEF wow!!! copy paste bug from the original Disk Thrower!
ThreatPosed=20	; This value MUST be 0 for all building addons
ImmuneToRadiation=no
Bombable=yes
AllowedToStartInMultiplayer=no
Size=1
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER,SCATTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ImmuneToPsionics=yes
IFVMode=0
Trainable=no
Natural=yes
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:DOG` | CSF-string key resolving to "Attack Dog". **Same key** as Soviet DOG (the CSF entry is the canonical English name for the animal type, not per-side) |
| `Name=Allied Attack Dog` | Internal name — differs from `[DOG]` ("Attack Dog") and `[YADOG]` ("Allied Attack Dog (Yuri version)") |
| `NotHuman=yes` | **Behavior flag** — sets InfantryTypeClass flag (per `InfantryTypeClass__ReadINI @ 0x005243C6` xref to string at `0x00825A00`). Excludes the unit from human-targeting weapons (e.g., Psychic Dominator, ChronoLegionnaire's chrono-vortex, sniper headshot), suppresses the infantry-squish blood splat, and gates the "Cloning Vats produces a duplicate" path (dogs don't clone). Critical for parity |
| `Category=Soldier` | Pip group + AI threat grouping (infantry) |
| `Primary=GoodTeeth` | **Allied-only weapon ID** — same stats as Soviet's `BadTeeth` but uses `[ADOGJUMP]` projectile (Allied dog visual). See "Weapons" section |
| `Secondary=VirtualScanner` | Zero-damage probe used by guard-AI scan path. Extends "attack target awareness" past the 1.5-cell Primary range so the dog can hunt across its 9-cell Sight |
| `NavalTargeting=6` | **Naval targeting class 6** — engine value indicating "can target ships from land if adjacent" combined with the right warhead. Effect on dogs is minimal since the warhead has 0% vs ship armor classes, but the flag is set for AI threat-pick |
| `Prerequisite=Barracks` | Generic "Barracks" prereq (Allied → GAPILE; Soviet → NAHAND; Yuri → YABRCK) |
| `LeadershipRating=7` | Veterancy-gain modifier — high (7 of 10), so the dog promotes quickly per kill. Moot since `Trainable=no` |
| `CrushSound=InfantrySquish` | Crush sound when crushed by vehicle (sound `igensqua`) |
| `Strength=100` | HP — same as GI |
| `Armor=none` | Damage type column 0 — standard infantry |
| `ReselectIfLimboed=yes` | **Behavior flag** — TechnoTypeClass field. When the dog enters Limbo state during a bite-leap (via `LimboLaunch=yes` on weapon — see Weapons), if the player had it selected before the leap, **the dog is automatically re-added to selection when it Unlimbo's**. Without this flag, the player loses selection on every bite. Comment: "If selected when limbo and attacking infantry, reseect when unlimbo" |
| `RejoinTeamIfLimboed=yes` | **Behavior flag** — same mechanic for AI Teams: if the dog is in a Team when it Limbo's for a bite, the engine records its team membership and re-attaches it after Unlimbo. Otherwise team coordination breaks every leap. Comment: "If in a team when limbo shooting infantry, write it down and try to rejoin when unlimbo" |
| `DefaultToGuardArea=yes` | **Behavior flag** — TechnoTypeClass field (xref from `TechnoTypeClass__ReadINI @ 0x00714F44` to string at `0x00843784`). Idle dogs default to Guard-Area mission (will pursue and attack hostiles within their `Sight` radius from the last-ordered position) rather than Guard (stationary). Comment: "the much awaited dog default to move and attack when resting" — this was a player-request feature added late, making dogs the only stock infantry that aggressively guards an area without manual command |
| `TechLevel=2` | Buildable at tech-level 2+ (early game) |
| `Pip=white` | Cargo-passenger pip color when loaded in transport |
| `Sight=9` | Reveal radius — large (matches Spy); enables the guard-area patrol to spot intruders |
| `DetectDisguise=yes` | **Behavior flag** — TechnoTypeClass field (xref from `TechnoTypeClass__ReadINI` to string at `0x00843C78`). Dog reveals nearby disguised units (Spy, Mirage Tank) within its detection radius (`DetectDisguiseRange=` defaults to Sight). When the dog enters detection range, the disguise blinks (per `InfantryBlinkDisguiseTime=20`) revealing the true unit. **Critical anti-spy / anti-mirage feature** |
| `Speed=8` | Foot-speed — fast (Speed=4 for a typical GI). Lets dogs catch fleeing infantry |
| `Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance` | **All ten houses** in `Owner=` — but `ForbiddenHouses=` then filters down |
| `ForbiddenHouses=Russians,Confederation,Africans,Arabs,YuriCountry` | **Excludes all Soviet (4) + Yuri (1) houses**. Net effect: only Allied houses (British, French, Germans, Americans, Alliance) can build `[ADOG]`. The other dogs (`[DOG]`/`[YDOG]`/`[YADOG]`) handle other sides with mirror-image ForbiddenHouses filters |
| `Cost=200` | Credits — cheapest non-engineer infantry |
| `Soylent=100` | Grinder refund (Yuri only) |
| `Points=10` | Kill score |
| `IsSelectableCombatant=yes` | Included in "select all combat units" hotkey |
| `VoiceSelect=DogSelect` | Selection voice — `idogsela` (single bark sample) |
| `VoiceMove=DogMove` | Move-order voice — `idogmova` (single bark, lower volume) |
| `VoiceAttack=DogAttackCommand` | Attack-order voice — `idogatca` |
| `VoiceFeedback=DogFear` | Fear voice — `idogfea/b/c` (rare; Priority=low) |
| `VoiceSpecialAttack=DogMove` | Reuses Move voice — dog has no special attack |
| `DieSound=DogDie` | Death sound — `idogdiea` |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — same as all infantry. Note: during the bite-leap the dog is Limbo'd; it's not the locomotor that handles the jump, it's the projectile flying as `DOGJUMP`/`ADOGJUMP` (see Weapons) |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `;MovementZone=InfantryDestroyer ;GEF wow!!! copy paste bug from the original Disk Thrower!` | **Designer comment documenting a bug-fix** — the dog originally inherited the Disk Thrower's MovementZone via copy-paste; corrected to `Infantry` |
| `ThreatPosed=20` | AI scoring weight — medium (twice Engineer's effective 0). Dogs are real anti-infantry threat |
| `ImmuneToRadiation=no` | Dogs **CAN** be killed by radiation (Desolator deploy radiation, particle effects). Default would be no anyway; explicit for clarity |
| `Bombable=yes` | Crazy Ivan can plant a bomb on this unit. Engineer's `BombSight=4` will detect it |
| `AllowedToStartInMultiplayer=no` | Cannot appear in starting unit complement |
| `Size=1` | Transport cargo slot cost |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER,SCATTER` | Six abilities at Veteran tier (matches what Trainable would grant if Trainable=yes — but Trainable=no overrides). SCATTER means at Veteran the dog will scatter from incoming fire |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | Four abilities at Elite tier — dead in this case since Trainable=no |
| `ImmuneToPsionics=yes` | **Behavior flag** — combined with `NotHuman=yes`, makes the dog **immune to all mind-control** (Yuri, Yuri Prime, Psychic Tower, Psychic Dominator). One of the strongest anti-Yuri counters in the game |
| `IFVMode=0` | IFV gunner-table index 0 = **the IFV's default machinegun** (no swap). The dog passenger does not change the IFV weapon — it just rides as cargo. Compare engineer `IFVMode=1` (medic beam) or spy `IFVMode=2` (disguise/jam beam) which actively swap |
| `Trainable=no` | **Cannot gain veterancy** — kill counts don't promote. The presence of Veteran/Elite ability lists is defensive (would activate if Trainable were ever flipped) |
| `Natural=yes` | **Behavior flag** — marks the unit as a "natural" (animal/non-built) for engine purposes. Used by some AI threat-scoring and the "natural enemy" warhead-targeting paths. Cows and Brutes are also Natural=yes. Affects whether the unit triggers certain hardcoded enemy responses |

---

## artmd.ini — `[ADOG]` section

`ini/artmd.ini:375`:

```ini
[ADOG] ; Allied Attack Dog
Cameo=ADOGICON
AltCameo=ADOGUICO
Sequence=DogSequence
Crawls=no
Remapable=yes
FireUp=6
PrimaryFireFLH=0,0,0
```

| Key | Meaning |
|-----|---------|
| `Cameo=ADOGICON` | Sidebar build icon (SHP `ADOGICON` — Allied-skinned dog cameo). Soviet `[DOG]` uses `DOGICON` |
| `AltCameo=ADOGUICO` | Elite cameo — unused (Trainable=no) |
| `Sequence=DogSequence` | **Shared sequence** with `[DOG]`, `[YDOG]`, `[YADOG]` — the per-side differences are only in the cameo and the SHP body palette (Remapable=yes handles house tint) |
| `Crawls=no` | **Cannot crawl/go prone** — sets the prone-disabled flag on the type |
| `Remapable=yes` | House remap palette applied to the colored pixels (the dog's collar/bandana) |
| `FireUp=6` | Bullet-spawn frame within the FireUp track — at frame 6 the dog launches as the projectile (note the `LimboLaunch=yes` on the weapon triggers at this moment) |
| `PrimaryFireFLH=0,0,0` | **All zero** — the projectile spawns at the dog's centre with no FLH offset. The dog *becomes* the projectile (LimboLaunch), so the launch position is just its current position |

### Referenced sequence — `[DogSequence]`

`artmd.ini:14516`:

```ini
[DogSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Prone=0,1,1     ;Dog can't crawl, but spy needs this listing
Crawl=8,6,6
Die1=86,15,0
Die2=101,15,0
FireUp=116,6,6
FireProne=116,6,6
Down=8,2,6
Up=8,2,6
Cheer=164,8,0,E
Panic=8,6,6

Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle (1 frame × 1 facing) | Default stance |
| `Guard=0,1,1` | Guard idle | Same |
| `Walk=8,6,6` | Walk cycle — 6 frames × 6 facings | Standard 6-direction art (no back-quarters, mirrored at runtime) |
| `Idle1=56,15,0,S` | Idle anim 1 — 15 frames, single S-facing | "Scratch / look around" animation |
| `Idle2=71,15,0,E` | Idle anim 2 — 15 frames, single E-facing | "Pant / wag" animation |
| `Prone=0,1,1` | **Falls back to Ready frame** | Designer comment: "Dog can't crawl, but spy needs this listing" — the disguise system requires Prone= to exist for proper rendering when a Spy disguises as a dog |
| `Crawl=8,6,6` | Crawl = reuse of Walk frames | Same comment as Prone — needed for spy-disguised-as-dog rendering |
| `Die1=86,15,0` | Death anim 1 — 15 frames, omnidirectional | |
| `Die2=101,15,0` | Death anim 2 — 15 frames |  |
| `FireUp=116,6,6` | **Leap animation** — 6 frames × 6 facings | This is the bite-leap pose; ends at frame 6 with the dog mid-air, triggering LimboLaunch |
| `FireProne=116,6,6` | Reuses FireUp | Dogs can't fire prone since they can't go prone, but defensive entry present |
| `Down=8,2,6` | "Down" = reuse of Walk frames | Unused (Crawls=no) |
| `Up=8,2,6` | "Up" = reuse of Walk frames | Unused |
| `Cheer=164,8,0,E` | Cheer — 8 frames, E-facing | Played on team victory cheer |
| `Panic=8,6,6` | Panic = reuse of Walk frames | Rarely seen since dogs are aggressive |
| `Die3/4/5=0,1,1` | Stub entries — fall back to Ready frame | Dog only has Die1/Die2 variants |

---

## Weapons

### Primary — `[GoodTeeth]`

`rulesmd.ini:23547`:

```ini
; Dog humping
[GoodTeeth]
Damage=30
ROF=30
Range=1.5
CellRangefinding=yes
Projectile=ADOGJUMP
Speed=30
Warhead=ParasiteDog ; infantry only version
LimboLaunch=yes ; Limbo shooter at launch (one shot or become the bullet)
Report=DogAttack
FireInTransport=no;can't fire out of the BattleFortress
```

(Note: designer comment "Dog humping" is a holdover from internal slang —
"humping" was their term for the leap-attack motion.)

| Key | Meaning |
|-----|---------|
| `Damage=30` | Per-bite damage — but the warhead's `Parasite=yes` mechanism overrides damage with **attach-and-kill** semantics (see Warhead). The damage value is the fallback if Parasite resolution fails |
| `ROF=30` | Cooldown between bites — 30 frames (2s @ 15fps). Important: this only matters if the dog returns to bite multiple targets; against a single infantry it's a one-shot |
| `Range=1.5` | **Bite range** — 1.5 cells (must be within one cell of target to launch the leap) |
| `CellRangefinding=yes` | Use cell-center distance for the range check rather than lepton precision — forgiving 1.5-cell radius for the leap target lock |
| `Projectile=ADOGJUMP` | The flying-dog projectile (Allied-skinned). Soviet `BadTeeth` uses `DOGJUMP` — same projectile config, different SHP. See `[ADOGJUMP]` below |
| `Speed=30` | Projectile travel speed during the leap — fast (high enough that visible-to-impact is just a few frames at 1.5 cells) |
| `Warhead=ParasiteDog` | **Critical warhead** — "infantry only version" of Parasite. See warhead section below |
| `LimboLaunch=yes` | **THE key behavior flag** — WeaponTypeClass field (per `WeaponTypeClass__ReadINI @ 0x00772107` xref to string at `0x0084952C`). When this weapon fires, the **firing unit itself is removed from the map** (Limbo'd) at launch. The projectile carries the firing unit's identity. On impact: if the warhead's resolution kills/grabs the target, the firing unit re-spawns at the target location; if the warhead resolution fails or the target dies before impact, the dog re-spawns at the launch location. Comment: "Limbo shooter at launch (one shot or become the bullet)". `ReselectIfLimboed=yes` + `RejoinTeamIfLimboed=yes` on the type ensure selection/team are preserved across this Limbo round-trip |
| `Report=DogAttack` | Sound `idogatta` — single growl sample. Volume=50, FShift=-5 +5 |
| `FireInTransport=no` | Cannot fire from inside [FV] Battle Fortress (would expose the leap behavior in a way that breaks the BF abstraction) |

### Primary's Warhead — `[ParasiteDog]`

`rulesmd.ini:27141`:

```ini
[ParasiteDog];Woof woof
Verses=100%,100%,100%,0%,0%,0%,0%,0%,0%,0%,0%
Parasite=yes
InfDeath=1
Rocker=yes
```

(Designer comment ";Woof woof" — internal naming humour.)

| Key | Meaning |
|-----|---------|
| `Verses=100%,100%,100%,0%,0%,0%,0%,0%,0%,0%,0%` | **11-column armor table** — 100% vs `none/flak/plate` (all infantry armors), **0% vs everything else** (light/medium/heavy vehicle armors, wood/steel/concrete structure armors, both specials). **This is what restricts dogs to attacking infantry only** — engine refuses to fire when projected damage would be 0, so the dog cursor only highlights on infantry-armored targets. Compare `[ParasitePlus]` (squid grab) which is 100% across the board |
| `Parasite=yes` | **THE warhead flag** — WarheadTypeClass field (per `WarheadTypeClass__ReadINI @ 0x0075D83B` xref to string at `0x0081717C`). On hit, instantiates a `ParasiteClass` instance (constructor at `0x00629210`) that attaches to the victim. For dogs, the attach immediately resolves to "kill victim, un-Limbo attacker at victim's tile" — there's no parasitic-damage-over-time loop because `InfDeath=1` finalizes the infantry-death the same tick. Compare Terror Drone: also `Parasite=yes` but on a vehicle target, ParasiteClass runs a per-tick damage loop until host explodes. For dogs the effect is essentially instant kill |
| `InfDeath=1` | Infantry death animation type 1 ("small arms" / generic shot). Played on the killed infantry as it dies |
| `Rocker=yes` | Tank/vehicle rocking effect — moot since Verses=0% vs vehicle armors means dogs never hit vehicles, but defensively present |

### Projectile — `[ADOGJUMP]`

`rulesmd.ini:25509`:

```ini
[ADOGJUMP]
Image=ADOGP ;Hmm...Requires an Image entry to get at Rotates=.  Violates the same name default rule
AA=no
;AN=no
Arm=2
ROT=8 ;requires to use Rotates
Shadow=no
Proximity=yes
Ranged=yes
FirersPalette=yes
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=yes
```

| Key | Meaning |
|-----|---------|
| `Image=ADOGP` | **Required Image redirect** — engine has a default "projectile uses same name as section if no Image=" rule, but the Rotates= field on this projectile demands an explicit Image= (designer note: "Violates the same name default rule"). `ADOGP` = "Allied Dog Projectile" SHP — the flying-dog sprite during the leap |
| `AA=no` | Cannot target aircraft (dogs don't leap onto Rocketeers or Kirovs) |
| `;AN=no` | (Commented) — would prevent naval targeting; absent so naval is permitted by default (though irrelevant given Warhead Verses 0% vs ship armor) |
| `Arm=2` | Projectile "arming distance" — 2 leptons; effectively no minimum range, projectile is "armed" immediately on launch |
| `ROT=8` | Rate of Turn — 8 (rotation speed for the rotating projectile sprite). Comment "requires to use Rotates" — the projectile rotates during flight to face the target |
| `Shadow=no` | No projectile shadow drawn during the leap (the dog's own shadow disappears with the unit during Limbo) |
| `Proximity=yes` | Detonates on proximity to target (1-cell radius) — dog leap impacts on touching the target, not pixel-precise |
| `Ranged=yes` | Range-limited projectile — respects weapon `Range=` |
| `FirersPalette=yes` | Uses firing unit's house palette — so the leaping dog displays in the firing player's color, matching the on-ground state |
| `SubjectToCliffs=no` | Projectile ignores cliff line-of-sight blocking (the dog leaps over small obstacles) |
| `SubjectToElevation=no` | Projectile ignores elevation differences — dogs can leap up cliffs as part of the bite |
| `SubjectToWalls=yes` | **Walls block the leap** — Allied/Soviet walls stop the projectile (and so the dog re-spawns at the launch position, leap failed) |

### Soviet equivalent — `[BadTeeth]` / `[DOGJUMP]`

`rulesmd.ini:23534` (BadTeeth) and `rulesmd.ini:25495` (DOGJUMP). **Identical
in every numeric stat** to GoodTeeth/ADOGJUMP — only the projectile SHP differs
(`DOGP` for Soviet vs `ADOGP` for Allied), giving each side a visually distinct
leaping-dog sprite. Used by `[DOG]` (Soviet) and `[YDOG]` (Yuri-built Soviet
variant).

### Secondary — `[VirtualScanner]`

A zero-damage, no-projectile "weapon" with extended range used as a guard-AI
scan probe. Allows the dog's `MissionGuard`/`GuardArea` logic to detect
hostile infantry beyond the 1.5-cell Primary range, so the dog can charge
into bite range. Defined elsewhere in rulesmd as a shared infantry helper
weapon.

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear

```ini
[DogSelect]                  ; soundmd.ini:990
Sounds=idogsela
FShift= -5 5
Volume=85

[DogMove]                    ; soundmd.ini:985
Sounds=idogmova
Volume=35
FShift= -5 5

[DogAttackCommand]           ; soundmd.ini:980
Sounds=idogatca
Volume=70
FShift= -5 5

[DogFear]                    ; soundmd.ini:995
Sounds= idogfea idogfeb idogfec
Control= random interrupt
FShift= -5 5
Priority=low
Volume=65
```

Each dog voice is **a single sample** (Select/Move/Attack), except Fear which
has three. This is the **most minimal voice bank** of any infantry — dogs only
bark; no spoken lines.

### Death

```ini
[DogDie]                     ; soundmd.ini:1002
Sounds= idogdiea
FShift= -5 5
Priority=low
Volume=65
```

Single yelp on death — `idogdiea`.

### Bite report

```ini
[DogAttack]                  ; soundmd.ini:975
Sounds=idogatta
Volume=50
FShift= -5 5
```

Single growl/snarl played at bite-leap launch (`Report=DogAttack` on
GoodTeeth/BadTeeth). Volume=50 (medium-low so it doesn't overwhelm at
combat distance).

### Notable cross-references

- `[HuskySelect]` (soundmd.ini:1008) — reuses `idogsela` for the campaign-only
  Husky unit (also a non-built dog variant)
- `[DogMove]` is wired as `VoiceSpecialAttack=` defensively (dog has no
  special attack but the line plays if one is ever triggered via map script)

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `Barracks` | Generic abstract — resolves to GAPILE (Allied) since house gating restricts to Allied |
| `Owner=` | `Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance` | All 10 houses listed (template inherited) |
| `ForbiddenHouses=` | `Russians,Confederation,Africans,Arabs,YuriCountry` | Excludes all Soviet (4) + Yuri (1) → **only Allied houses** can build |
| `TechLevel=` | `2` | Available very early |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=200` | $200 | |
| `Soylent=100` | $100 refund | Grinder (Yuri) only |
| `Points=10` | 10 | Kill-score contribution |

No `PrerequisiteOverride=`, no `BuildLimit=`, no `RequiresStolenXxxTech=`.

---

## Veterancy

| Field | Value | Notes |
|-------|-------|-------|
| `Trainable=no` | — | **Cannot gain veterancy**. Veteran/Elite ability lists are defensively present but never activate |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER,SCATTER` | (inert) | Would grant +25/25/20/+1/20% + scatter-from-fire at Veteran |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | (inert) | Would grant HP regen + stack bonuses |
| `AltCameo=ADOGUICO` | — | Elite cameo defined in artmd, never shown |

---

## Hardcoded behavior — Ghidra-verified

### 1. LimboLaunch bite-leap mechanism (Primary path) [BINARY-VERIFIED audit 9]

INI key `LimboLaunch` is a **WeaponTypeClass** field at byte offset
**`WeaponTypeClass+0x132`** (BINARY-VERIFIED via `WeaponTypeClass__ReadINI`
@ 0x00772107 reading `*(byte*)((int)this + 0x132)` via
`CCINIClass__ReadBool(s_LimboLaunch_0084952c, ...)`).
When set on a weapon, firing the weapon causes the **firing unit to be
Limbo'd** (removed from the map state, hidden, paused) at the moment of
launch. The projectile carries the firing unit's identity. On impact:

- If the warhead resolution succeeds (e.g., `Parasite=yes` attaches and kills
  victim infantry): the firing unit Unlimbo's at the target location.
- If the projectile is intercepted, misses, or the target dies before
  impact: the firing unit Unlimbo's at its original launch position.

For dogs specifically, the leap is one-shot and instant-kill against
infantry; LimboLaunch + Parasite gives the visual feel of "the dog jumps,
lands on the enemy, the enemy dies, the dog appears next to the corpse."

### 2. ParasiteDog warhead → ParasiteClass attach + kill [BINARY-VERIFIED audit 9]

`Parasite=yes` is a **WarheadTypeClass** field (per
`WarheadTypeClass__ReadINI @ 0x0075D83B` DATA xref to string at `0x0081717C`).
On hit, instantiates a `ParasiteClass` instance via constructor at
`0x00629210` (Ghidra-labeled `ParasiteClass__Constructor`; body
0x00629210–0x006292A0). A second constructor variant exists at
`0x006292B0` (body 0x006292B0–0x00629387 — likely the parameterized variant).
The constructor:

```c
ParasiteClass::Constructor(this):
  AbstractClass::Constructor()
  this->LaunchFrame  = g_CurrentFrameCounter   // +0x2C (param_1[0xB])
  this->Unknown_0x38 = g_CurrentFrameCounter   // +0x38 (param_1[0xE])
  this->field_0x34   = 0                       // (param_1[0xD])
  this->field_0x40   = 0                       // (param_1[0x10])
  // ... vtable assignments for primary + 3 IUnknown interfaces ...
  // ... append this to global DynamicVectorClass<ParasiteClass*> ...
```

For a dog (Parasite-able warhead against infantry-armor target): the
ParasiteClass acts as a kill-resolver. The victim infantry is destroyed
immediately, an `InfDeath=1` animation plays, and the dog's LimboLaunch
round-trip resolves by re-spawning the dog at the victim's tile.

For a Terror Drone (Parasite-able warhead against vehicle-armor target):
the same ParasiteClass instead attaches to the vehicle and runs a per-tick
internal damage loop. Different victim type → different ParasiteClass
runtime behavior; same warhead infrastructure.

The dog's `ParasiteDog` warhead Verses (100/100/100/0/0/0/0/0/0/0/0) ensures
the dog version only fires against infantry-armor targets, which trigger the
instant-kill resolution path, not the host-attach loop.

### 3. NotHuman=yes — psionic / sniper exclusion

INI key `NotHuman` is an **InfantryTypeClass** field (per
`InfantryTypeClass__ReadINI @ 0x005243C6` DATA xref to string at `0x00825A00`).
Sets a per-type flag bit checked by:

- **Mind-control acquisition** (Yuri, Yuri Prime, Psychic Tower targeting
  scan): NotHuman units are filtered out — dogs cannot be mind-controlled.
- **Sniper weapon target gate** (Sniper's headshot warhead): NotHuman units
  excluded from the one-shot kill bonus.
- **Cloning Vats duplicator**: NotHuman units don't trigger the free-copy
  spawn when produced.
- **Infantry-squish blood splat** (CrushSound + blood anim): dog crush plays
  the standard sound but suppresses the human-blood splat anim.

Combined with `ImmuneToPsionics=yes` on the same type, dogs are **strongly
mind-control-immune** — a redundant double-flag for safety.

### 4. DefaultToGuardArea=yes — passive area patrol [BINARY-VERIFIED audit 9]

INI key `DefaultToGuardArea` is a **TechnoTypeClass** field at byte offset
**`TechnoTypeClass+0xD39`** (BINARY-VERIFIED via `TechnoTypeClass__ReadINI`
@ 0x00714F44 reading `*(byte*)((int)param_1 + 0xd39)` via
`CCINIClass__ReadBool(s_DefaultToGuardArea_00843784, ...)`).
When set, idle units default to `MissionGuardArea` (active area patrol within
Sight radius from the last-ordered position) instead of `MissionGuard`
(stationary attack-when-attacked). Designer comment: "the much awaited dog
default to move and attack when resting" — this was added late in development
based on player feedback. Dogs are the only stock infantry with this flag set.

### 5. DetectDisguise=yes — spy/mirage reveal [BINARY-VERIFIED audit 9 + audit 6]

INI key `DetectDisguise` is a **TechnoTypeClass** field at byte offset
**`TechnoTypeClass+0xD31`** (BINARY-VERIFIED audit 6 via TechnoTypeClass__ReadINI;
re-confirmed audit 9). Dog's presence within `DetectDisguiseRange=` (read
into TechnoType+0x5F4, audit 6) of a disguised unit (Spy or Mirage Tank)
triggers the per-tick disguise-detect check; the disguise blinks
(per `InfantryBlinkDisguiseTime=20`, Rules+0x1014, audit 6) revealing the
real unit. This is a passive aura — the dog doesn't need to be alerted, just
present. Critical for parity vs. spy infiltration play.

### 6. ReselectIfLimboed / RejoinTeamIfLimboed — bite-leap state preservation [BINARY-VERIFIED audit 9]

Both are **TechnoTypeClass** fields at byte offsets
**`TechnoTypeClass+0xD3C` (ReselectIfLimboed)** and
**`TechnoTypeClass+0xD3D` (RejoinTeamIfLimboed)** (BINARY-VERIFIED via
`TechnoTypeClass__ReadINI` @ 0x007142B4 and 0x007142CE respectively). They
patch a UX gap introduced by `LimboLaunch=yes`: without them, every
bite-leap would de-select the dog and break its AI Team membership (because
the unit literally leaves the world state during Limbo). With them set, the
engine records the pre-Limbo selection/team state and restores it on
Unlimbo.

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("LimboLaunch\|NotHuman\|DefaultToGuardArea\|ReselectIfLimboed\|RejoinTeamIfLimboed\|Parasite\|DetectDisguise")` | 13 strings found — confirmed all 7 keys are hardcoded-recognized: `Parasite` (warhead), `Parasiteable` (target eligibility), `NotHuman` (infantry type), `LimboLaunch` (weapon), `DefaultToGuardArea` (techno type), `DetectDisguise` (techno type), `DetectDisguiseRange` (techno type), `RejoinTeamIfLimboed` (techno type), `ReselectIfLimboed` (techno type), plus class RTTI strings `.?AVParasiteClass@@` and two VectorClass<ParasiteClass*> templates |
| `search_functions_enhanced(name_pattern="Parasite\|LimboLaunch\|Bite")` | 2 hits: `ParasiteClass__Constructor @ 0x00629210` (primary ctor, sets timestamps and vtables) and `ParasiteClass__Constructor @ 0x006292B0` (second variant — likely an alternate-args ctor) |
| `get_xrefs_to(0x0084952C)` (= "LimboLaunch") | Sole xref from `WeaponTypeClass__ReadINI @ 0x00772107` DATA — confirms LimboLaunch is a per-weapon flag, parsed once at INI load |
| `get_xrefs_to(0x00825A00)` (= "NotHuman") | Sole xref from `InfantryTypeClass__ReadINI @ 0x005243C6` DATA — confirms NotHuman is on InfantryTypeClass (not the parent TechnoType) |
| `get_xrefs_to(0x0081717C)` (= "Parasite") | Sole xref from `WarheadTypeClass__ReadINI @ 0x0075D83B` DATA — confirms Parasite is a warhead flag |
| `get_xrefs_to(0x00843784)` (= "DefaultToGuardArea") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714F44` DATA — confirms the field is parsed for all Techno types (not just infantry) |
| `decompile_function(0x00629210)` | ParasiteClass ctor: initializes two frame timestamps (offsets 0x2C, 0x38), three field clears (0x34, 0x40), vtable + 3 IUnknown vtable assignments, append to global DynamicVectorClass<ParasiteClass*>. Confirms ParasiteClass is a tracked AbstractClass-derived entity with per-instance lifetime separate from the host weapon |

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;MovementZone=InfantryDestroyer` (commented) | Designer-fixed copy-paste bug from Disk Thrower. **Not TS-legacy** but historical — kept as inline comment to document the fix. The active `MovementZone=Infantry` is correct for YR | OK |
| `;GEF going to be two dogs now DoubleOwned=Yes` (commented in `[DOG]` only) | Reference to a deferred design idea (two dogs per build); not implemented. Not on `[ADOG]` | N/A — historical comment |
| `NavalTargeting=6` | Used by AI target-pick; YR-active for any unit with this field. Effect on dog is minimal due to Verses=0% vs ships | OK |
| `Natural=yes` | YR-active; used by warhead/AI for "natural" entity rules. Not TS-only | OK |
| `LimboLaunch=yes` | YR-active in retail — used by dogs, Terror Drone (vehicle Parasite), and some campaign-only units | OK |
| `Parasite=yes` warhead | YR-active — `ParasiteClass` is constructed every match dogs/drones engage | OK |
| `ImmuneToRadiation=no` explicitly set | YR-active — radiation damage is gameplay-relevant (Desolator). Explicit `no` defensively overrides any defaults | OK |

No TS-only behavior found on the ADOG type itself.

---

## Ghidra audit log (audit iteration 9 — 2026-05-18)

Independent re-verification pass against gamemd.exe. ~12 function entry-point
verifications + decompiles of ParasiteClass constructor + WeaponTypeClass /
TechnoTypeClass / ObjectTypeClass / WarheadType ReadINI store-site searches
to pin exact field offsets.

### Function entry points re-verified

| Doc claim | Verified at exact address |
|-----------|---------------------------|
| `ParasiteClass::Constructor @ 0x00629210` | ✅ exact (Ghidra-labeled, body 0x00629210–0x006292A0, 144 bytes — primary constructor; calls `AbstractClass__Constructor_Full` then sets +0xB/+0xD/+0xE/+0x10 fields and 4 vtables; appends to global DynamicVectorClass at DAT_00ac4914) |
| `ParasiteClass::Constructor (2nd) @ 0x006292B0` | ✅ exact (Ghidra-labeled, body 0x006292B0–0x00629387 — likely parameterized variant; body DEFERRED) |
| `WeaponTypeClass::ReadINI` (via xref @ 0x00772107 for LimboLaunch) | ✅ exact — decompiled this pass; lots of WeaponType offsets verified inline below |
| `InfantryTypeClass::ReadINI` (via xref @ 0x005243C6 for NotHuman) | ✅ NotHuman is InfantryType-scope (xref data confirms) |
| `WarheadTypeClass::ReadINI` (via xref @ 0x0075D83B for Parasite) | ✅ Parasite is WarheadType-scope (xref data confirms) |
| `TechnoTypeClass::ReadINI` (xrefs at 0x00714F44, 0x007142B4, 0x007142CE, 0x00714946, 0x00714D53 for various flags) | ✅ All five keys confirmed TechnoType-scope |

### ParasiteClass constructor body — instance offsets BINARY-VERIFIED

```c
ParasiteClass::Constructor(this) {
    AbstractClass::Constructor_Full();
    this[0xB]  = g_CurrentFrameCounter;  // +0x2C: LaunchFrame
    this[0xD]  = 0;                       // +0x34: field_0x34 (cleared)
    this[0xE]  = g_CurrentFrameCounter;   // +0x38: secondary timestamp
    this[0x10] = 0;                       // +0x40: field_0x40 (cleared)
    *this     = &vtable__ParasiteClass;            // +0x0
    this[1]   = &vtable__ParasiteClass__secondary_4;   // +0x4
    this[2]   = &vtable__ParasiteClass__secondary_8;   // +0x8
    this[3]   = &vtable__ParasiteClass__secondary_12;  // +0xC
    // Append to global DynamicVectorClass<ParasiteClass*> at DAT_00ac4914:
    DAT_00ac4920++;
    *(ParasiteClass**)(DAT_00ac4914 + DAT_00ac4920 * 4) = this;
    return this;
}
```

ParasiteClass instance offsets BINARY-VERIFIED:
- **+0x0..+0xC = four vtable pointers** (multi-interface COM-style — primary + 3 IUnknown secondaries)
- **+0x2C = LaunchFrame** (g_CurrentFrameCounter at construction)
- **+0x34 = field_0x34** (cleared)
- **+0x38 = secondary timestamp** (g_CurrentFrameCounter, often used as "last damage tick" for vehicle-host loops)
- **+0x40 = field_0x40** (cleared)

Global tracking: ParasiteClass instances are appended to a DynamicVector at
`DAT_00ac4914` (array data ptr) / `DAT_00ac4920` (count), with capacity
sentinel `DAT_00ac4918`. [BINARY-VERIFIED audit 9]

### WeaponTypeClass offsets BINARY-VERIFIED (this audit)

`WeaponTypeClass__ReadINI` decompile pinned the following byte offsets
(this is param_2 = WeaponType, with reads as `*(byte*)((int)this + offset)`):

| Offset | Field | INI key |
|--------|-------|---------|
| +0x98 | AmbientDamage (int) | `AmbientDamage=` |
| +0x9C | Burst (int) | `Burst=` |
| +0xA0 | Projectile ptr | `Projectile=` |
| +0xA4 | Damage (int) | `Damage=` |
| +0xA8 | Speed (int) | `Speed=` |
| +0xAC | Warhead ptr | `Warhead=` |
| +0xB0 | ROF (int) | `ROF=` |
| +0xB4 | Range (int) | `Range=` |
| +0xB8 | MinimumRange (int) | `MinimumRange=` |
| +0xCC..0xD4 | Report sound list (3 ints) | `Report=` |
| +0xE8..0xF0 | DownReport sound list | `DownReport=` |
| +0x129 | UseFireParticles | `UseFireParticles=` |
| +0x12A | UseSparkParticles | `UseSparkParticles=` |
| +0x12B | OmniFire (byte) | `OmniFire=` |
| +0x12C | DistributedWeaponFire | `DistributedWeaponFire=` |
| +0x12D | IsRailgun | `IsRailgun=` |
| +0x12E | Lobber | `Lobber=` |
| +0x130 | IsSonic | `IsSonic=` |
| +0x131 | Spawner | `Spawner=` |
| **+0x132** | **LimboLaunch (byte)** ✅ | **`LimboLaunch=`** ← used by GoodTeeth/BadTeeth |
| +0x133 | DecloakToFire | `DecloakToFire=` |
| +0x134 | CellRangefinding | `CellRangefinding=` |
| +0x135 | FireOnce | `FireOnce=` |
| +0x136 | NeverUse | `NeverUse=` |
| +0x137 | RevealOnFire | `RevealOnFire=` |
| +0x138 | TerrainFire | `TerrainFire=` |
| +0x139 | SabotageCursor | `SabotageCursor=` |
| +0x13A | MigAttackCursor | `MigAttackCursor=` |
| +0x13B | DisguiseFireOnly | `DisguiseFireOnly=` |
| +0x13C | DisguiseFakeBlinkTime (int) | `DisguiseFakeBlinkTime=` |
| +0x140 | InfiniteMindControl | `InfiniteMindControl=` |
| +0x141 | FireWhileMoving | `FireWhileMoving=` |
| +0x142 | DrainWeapon | `DrainWeapon=` |
| +0x143 | FireInTransport (byte) | `FireInTransport=` |
| +0x144 | Suicide | `Suicide=` |
| +0x145 | TurboBoost | `TurboBoost=` |
| +0x146 | Supress | `Supress=` |
| +0x147 | Camera | `Camera=` |
| +0x148 | Charges | `Charges=` |
| +0x149 | IsLaser | `IsLaser=` |
| +0x14A | DiskLaser | `DiskLaser=` |
| +0x14B | IsLine | `IsLine=` |
| +0x14C | Bright | `Bright=` |
| +0x14D | IsHouseColor | `IsHouseColor=` |
| +0x14E | LaserDuration (int) | `LaserDuration=` |
| +0x14F | IsBigLaser | `IsBigLaser=` |
| +0x150 | IonSensitive | `IonSensitive=` |
| +0x151 | AreaFire | `AreaFire=` |
| +0x152 | IsElectricBolt | `IsElectricBolt=` |
| +0x153 | DrawBoltAsLaser | `DrawBoltAsLaser=` |
| +0x154 | IsAlternateColor | `IsAlternateColor=` |
| +0x155 | IsRadBeam | `IsRadBeam=` |
| +0x158 | RadLevel (int) | `RadLevel=` |
| +0x15C | IsMagBeam | `IsMagBeam=` |
| +0x120..+0x122 | LaserInnerColor (RGB) | `LaserInnerColor=` |
| +0x123..+0x125 | LaserOuterColor (RGB) | `LaserOuterColor=` |
| +0x126..+0x128 | LaserOuterSpread (RGB) | `LaserOuterSpread=` |

### TechnoTypeClass offsets BINARY-VERIFIED (this audit)

| Offset | Field | INI key | Notes |
|--------|-------|---------|-------|
| +0x693 | Natural (byte) | `Natural=` | Read via `*(byte*)((int)param_1 + 0x693)` |
| +0xD37 | ImmuneToRadiation (byte) | `ImmuneToRadiation=` | Read via `*(byte*)((int)param_1 + 0xd37)` |
| +0xD39 | DefaultToGuardArea (byte) | `DefaultToGuardArea=` | Read via `*(byte*)((int)param_1 + 0xd39)` |
| +0xD3C | ReselectIfLimboed (byte) | `ReselectIfLimboed=` | Read via `(char)param_1[0x34F]` |
| +0xD3D | RejoinTeamIfLimboed (byte) | `RejoinTeamIfLimboed=` | Read via `*(byte*)((int)param_1 + 0xd3d)` |

### Parser-scope verifications (this audit, via INI key xrefs)

| INI key | Reader xref | Scope |
|---------|-------------|-------|
| `LimboLaunch` | `WeaponTypeClass__ReadINI` @ 0x00772107 | **WeaponType** ✅ |
| `NotHuman` | `InfantryTypeClass__ReadINI` @ 0x005243C6 | **InfantryType** ✅ (not parent TechnoType) |
| `Parasite` | `WarheadTypeClass__ReadINI` @ 0x0075D83B | **WarheadType** ✅ |
| `DefaultToGuardArea` | `TechnoTypeClass__ReadINI` @ 0x00714F44 | **TechnoType** ✅ |
| `ReselectIfLimboed` | `TechnoTypeClass__ReadINI` @ 0x007142B4 | **TechnoType** ✅ |
| `RejoinTeamIfLimboed` | `TechnoTypeClass__ReadINI` @ 0x007142CE | **TechnoType** ✅ |
| `Natural` | `TechnoTypeClass__ReadINI` @ 0x00714946 | **TechnoType** ✅ |
| `ImmuneToRadiation` | `TechnoTypeClass__ReadINI` @ 0x00714D53 | **TechnoType** ✅ |
| `DetectDisguise` | (audit 6 — TechnoType+0xD31) | **TechnoType** ✅ (re-confirmed) |

### Items NOT re-verified this pass (DEFERRED)

- **LimboLaunch consumer end-to-end** — i.e., the actual InfantryClass code
  path that hides the firing unit, attaches it to the projectile, and
  un-Limbos it at the target. The WeaponType+0x132 flag is BINARY-VERIFIED
  as the storage, but the consumer (likely in InfantryClass::Fire_At or
  TechnoClass::Fire_At, audit 5) was not separately decompiled this pass.
  DEFERRED.
- **NotHuman exact byte offset** — confirmed InfantryType-scope via xref,
  but the exact offset within InfantryTypeClass's bool-chain (somewhere in
  +0xEBC..+0xECB given Engineer is at +0xEC5 and NotHuman's xref is earlier
  at 0x5243C6) was not pinned. DEFERRED.
- **Parasite exact byte offset on WarheadType** — confirmed WarheadType-scope
  via xref but not pinned to a specific byte offset. DEFERRED.
- **ParasiteClass::Update / tick loop** — for vehicle-host attach (Terror
  Drone), the per-tick damage loop and the eventual host destruction
  trigger were not decompiled. DEFERRED.
- **ParasiteClass invocation site** — i.e., where in the `WarheadType.Detonate`
  flow (audit 5 verified at 0x004690B0) the `Parasite=yes` branch
  instantiates a ParasiteClass. Not re-decompiled this pass — the WarheadType
  consumer code path was not walked. DEFERRED.
- **`InfantryBlinkDisguiseTime` actual disguise-blink consumer code** — the
  Rules+0x1014 storage was BINARY-VERIFIED in audit 6, but the per-tick
  consumer that applies the blink is not yet traced. DEFERRED.
- **`Bombable=yes` consumer** — the byte at ObjectType+0x22E (audit 7) gates
  Crazy Ivan's bomb-plant cursor; consumer code not re-verified.

### Confidence summary

**HIGH** for: all 2 ParasiteClass constructor addresses (Ghidra-labeled),
all ParasiteClass instance offsets (constructor body decompiled), all 5
TechnoTypeClass struct offsets pinned this pass (DefaultToGuardArea,
ReselectIfLimboed, RejoinTeamIfLimboed, ImmuneToRadiation, Natural),
WeaponTypeClass+0x132 = LimboLaunch, the full WeaponTypeClass capability-flag
block from +0x129..+0x15C, and the 9 INI parser-scope verifications.

**MEDIUM** for: ParasiteClass behavior interpretation (instant-kill vs
host-attach-loop dichotomy is documented in the doc but not separately
verified by decompiling the Detonate consumer path).

**LOW / DEFERRED**: NotHuman exact offset, Parasite exact offset on
WarheadType, ParasiteClass::Update loop, the actual LimboLaunch
projectile-attachment runtime in Fire_At.

---

## Cross-references

- **Sister units** (same template, see "Hardcoded Behavior" section for full
  shared mechanism — only the listed fields differ):
  - `[DOG]` Soviet Attack Dog (`rulesmd.ini:4369`) — `Primary=BadTeeth` (uses
    DOGJUMP projectile), `Name=Attack Dog`, ForbiddenHouses excludes Allied
    + Yuri, voice/cameo same `DogSelect/DOGICON`
  - `[YDOG]` Yuri-built Soviet variant (`rulesmd.ini:4964`) — `Image=DOG`,
    `Primary=BadTeeth`, `Prerequisite=NAHAND` (literal Soviet barracks
    required, not abstract), `RequiredHouses=YuriCountry`,
    `Soylent=50` (half the normal refund). When Yuri captures/has a Soviet
    barracks they can build this
  - `[YADOG]` Yuri-built Allied variant (`rulesmd.ini:4913`) — `Image=ADOG`,
    `Primary=GoodTeeth`, `Prerequisite=GAPILE` (literal Allied barracks),
    `RequiredHouses=YuriCountry`, `Soylent=50`. When Yuri has an Allied
    barracks they can build this
- **Related sister-weapons**:
  - `[BadTeeth]`/`[GoodTeeth]` — same stats, different projectile SHP
  - `[DOGJUMP]`/`[ADOGJUMP]` — same flight projectile config, different image
    (`DOGP`/`ADOGP`)
  - `[ParasiteDog]` — single shared warhead used by both GoodTeeth and
    BadTeeth (Verses ensure infantry-only resolution)
- **Same Parasite=yes warhead family** (shared `ParasiteClass` mechanism):
  - `[ParasitePlus]` — Giant Squid grab (100% across all armors,
    `Culling=yes` for red-HP kill, `Paralyzes=32767`)
  - Terror Drone's weapon `[TerrorDroneInvader]` (or similar) — uses
    `Parasite=yes` against vehicle-armor targets, triggering the
    host-attach damage-loop variant of `ParasiteClass`
- **Counter-units / hard counters to dogs**:
  - Anything not Soldier-category — dogs literally cannot target vehicles or
    structures (`ParasiteDog.Verses` 0% vs non-infantry armor)
  - Mirage Tank in tree disguise — dog will reveal it via DetectDisguise but
    won't engage (wrong armor)
  - Yuri / Yuri Prime / Initiate vs **non-dog** infantry — but cannot
    mind-control dogs (NotHuman + ImmuneToPsionics)
- **Soundmd cross-link**:
  - `[HuskySelect]` (`soundmd.ini:1008`) — reuses `idogsela` for the
    campaign-only Husky unit
- **Related INI globals**:
  - `InfantryBlinkDisguiseTime=20` — controls DetectDisguise blink duration
  - `[InfantrySquish]` crush sound — shared with all crushable infantry

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [ADOG]` | 3767-3814 (48 lines) | All 47 active keys covered with explanations (one commented-out MovementZone documented) |
| `artmd.ini [ADOG]` | 375-382 (8 lines) | All keys covered |
| `artmd.ini [DogSequence]` | 14516-14535 (20 lines) | All 17 active slots + 3 stub Die3-5 covered |
| `rulesmd.ini [GoodTeeth]` | 23547-23557 (11 lines) | All keys covered |
| `rulesmd.ini [ParasiteDog]` | 27141-27145 (5 lines) | All keys covered |
| `rulesmd.ini [ADOGJUMP]` | 25509-25521 (13 lines) | All keys covered (incl. one commented `;AN=no`) |
| `rulesmd.ini [BadTeeth]/[DOGJUMP]` | 23534/25495 | Referenced as sister-weapon (same stats) |
| `soundmd.ini` Dog voices | DogSelect, DogMove, DogAttackCommand, DogFear, DogDie, DogAttack | All 6 covered |
| Hardcoded behavior | LimboLaunch + Parasite + NotHuman + DefaultToGuardArea + DetectDisguise + Reselect/RejoinTeamIfLimboed | 6 distinct mechanisms covered with Ghidra confirmation |
| Ghidra searches performed against ID | 7 distinct queries (1 strings + 1 function search + 4 xref lookups + 1 decompile) | Logged inline |
| TS-legacy filter | Applied; no TS-only behavior found on the ADOG type | Done |
