# Soviet Attack Dog (DOG)
Side: Soviet | Category: Infantry | Image alias: `[DOG]` (no `Image=` redirect — own SHP `DOG`)

The Soviet faction's Attack Dog. Mechanically identical to the canonical Allied
[ADOG](../allied/ADOG.md) — $200 anti-infantry leap-bite unit from NAHAND.
Only differences from the canonical Allied dossier are:

1. `Name=Attack Dog` (internal — `UIName=Name:DOG` resolves both variants to
   the same CSF entry "Attack Dog" in-game).
2. `Primary=BadTeeth` instead of `GoodTeeth`. Same Damage/ROF/Range/Warhead —
   only the projectile sprite differs (`DOGJUMP` uses `Image=DOGP` vs
   ADOG's `ADOGJUMP` `Image=ADOGP`). Player-visible: identical pixel
   silhouette during the leap; in vanilla art these are essentially the same
   SHP. **Net gameplay output: bit-identical bite behavior.**
3. `ForbiddenHouses=British,French,Germans,Americans,Alliance,YuriCountry` —
   excludes the 5 Allied houses + YuriCountry → net Owner: **Russians,
   Confederation, Africans, Arabs** (the four Soviet houses).
4. art: `Cameo=DOGICON` / `AltCameo=DOGUICO` (vs ADOG's `ADOGICON`/`ADOGUICO`).
   The dog's SHP and sequence (`Sequence=DogSequence`) are shared between
   both factions.

Voice bank is **shared** (`DogSelect/Move/AttackCommand/Fear/Die`) — both
factions' dogs bark with identical samples. Every other key in the section is
byte-identical to ADOG.

This is a quick-reference doc; cross-reference the canonical
[ADOG.md](../allied/ADOG.md) for the full surface (LimboLaunch leap mechanism,
ParasiteDog warhead, DetectDisguise, ImmuneToPsionics, IFVMode=0 quirk,
veterancy abilities, hardcoded behavior).

---

## rulesmd.ini — `[DOG]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4369`:

```ini
[DOG]
UIName=Name:DOG
Name=Attack Dog
NotHuman=yes
Category=Soldier
Primary=BadTeeth
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
ForbiddenHouses=British,French,Germans,Americans,Alliance,YuriCountry
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
;GEF going to be two dogs now DoubleOwned=Yes
ImmuneToPsionics=yes
IFVMode=0
Trainable=no
Natural=yes
```

### Keys that differ from `[ADOG]`

| Key | DOG value | ADOG value | Notes |
|-----|-----------|-----------|-------|
| `Name=Attack Dog` | Internal short name | `Allied Attack Dog` | Internal only — `UIName=Name:DOG` resolves both to "Attack Dog" |
| `Primary=BadTeeth` | Soviet bite weapon | `GoodTeeth` | Identical stats — only Projectile differs (`DOGJUMP` vs `ADOGJUMP`). Same Damage=30, ROF=30, Range=1.5, Warhead=ParasiteDog, LimboLaunch=yes |
| `ForbiddenHouses=British,French,Germans,Americans,Alliance,YuriCountry` | 5 Allied + 1 Yuri forbidden | 4 Soviet + 1 Yuri forbidden | Net DOG owner: 4 Soviet houses; net ADOG owner: 5 Allied houses |
| Inline comment `;GEF going to be two dogs now DoubleOwned=Yes` | Present in DOG | **Not present** in ADOG | Westwood comment-only; `DoubleOwned=` is not actually set. Harmless data difference |

All other 41 keys are byte-identical to `[ADOG]` — see the
[ADOG dossier](../allied/ADOG.md) for key-by-key explanation
(NotHuman/Category/Secondary/NavalTargeting/Prerequisite/LeadershipRating/
Strength/Armor/Reselect+RejoinIfLimboed/DefaultToGuardArea/TechLevel/Pip/Sight/
DetectDisguise/Speed/Cost/Soylent/Points/IsSelectableCombatant/voices/
DieSound/Locomotor/PhysicalSize/MovementZone/ThreatPosed/ImmuneToRadiation/
Bombable/AllowedToStartInMultiplayer/Size/VeteranAbilities/EliteAbilities/
ImmuneToPsionics/IFVMode/Trainable/Natural).

### Implicit defaults (same as ADOG)

- `Crawls=` — set in artmd section to `no` (dog has no prone state).
- `Crushable=` — defaults `yes` (infantry).
- `Occupier=` / `Deployer=` — both default `no`.
- `Engineer=` — defaults `no`.

---

## artmd.ini — `[DOG]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:366`:

```ini
[DOG] ; Soviet Attack Dog
Cameo=DOGICON
AltCameo=DOGUICO
Sequence=DogSequence
Crawls=no
Remapable=yes
FireUp=6
PrimaryFireFLH=0,0,0
```

| Key | Meaning |
|-----|---------|
| `Cameo=DOGICON` | Sidebar icon — **DOGICON.SHP**, the Soviet attack-dog cameo. Single-frame remappable |
| `AltCameo=DOGUICO` | Elite cameo — **never displayed** because `Trainable=no` |
| `Sequence=DogSequence` | Shared `[DogSequence]` block used by ADOG/DOG/YDOG/YADOG. Defines the bite/leap/walk/idle frame layout. See [ADOG.md §artmd](../allied/ADOG.md#artmdini--adog-section) for the full sequence list |
| `Crawls=no` | Dog has no prone-while-moving animation (visually unsuitable for a quadruped) |
| `Remapable=yes` | Soviet house palette remap applied to dog body |
| `FireUp=6` | Bullet-spawn frame within the bite-firing sequence — frame 6 of FireUp track (where the LimboLaunch leap projectile spawns) |
| `PrimaryFireFLH=0,0,0` | Muzzle-flash launch height = 0 lepton offset from cell center (dog leaps from its own footprint) |

ADOG's artmd block is byte-identical except `Cameo=ADOGICON` / `AltCameo=ADOGUICO`. Sequence, FireUp frame, and PrimaryFireFLH are the same.

---

## Weapons

### Primary — `[BadTeeth]`

`rulesmd.ini:23534`:

```ini
[BadTeeth]
Damage=30
ROF=30
Range=1.5
CellRangefinding=yes
Projectile=DOGJUMP
Speed=30
Warhead=ParasiteDog ; infantry only version
LimboLaunch=yes ; Limbo shooter at launch (one shot or become the bullet)
Report=DogAttack
FireInTransport=no;can't fire out of the BattleFortress
```

Differences from `[GoodTeeth]` (ADOG's primary):

| Key | BadTeeth | GoodTeeth | Notes |
|-----|----------|-----------|-------|
| `Projectile=` | `DOGJUMP` | `ADOGJUMP` | The two projectiles use `Image=DOGP` vs `Image=ADOGP` — Soviet vs Allied dog leap sprite. Both have `Arm=2`, `ROT=8`, `FirersPalette=yes`, `SubjectToCliffs=no`, `SubjectToElevation=no`, `SubjectToWalls=yes`, `Proximity=yes`, `Ranged=yes`, `Shadow=no`. Mechanically identical — only the in-flight sprite differs |

All other 9 weapon keys are byte-identical: Damage=30, ROF=30, Range=1.5,
CellRangefinding=yes, Speed=30, Warhead=ParasiteDog, LimboLaunch=yes,
Report=DogAttack, FireInTransport=no. See
[ADOG §Weapons](../allied/ADOG.md#weapons) for full annotation including
the LimboLaunch leap mechanism and ParasiteDog warhead.

### Projectile — `[DOGJUMP]`

`rulesmd.ini:25495`:

```ini
[DOGJUMP]
Image=DOGP ;Hmm...Requires an Image entry to get at Rotates=.  Violates the same name default rule
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
| `Image=DOGP` | Soviet dog projectile sprite (the in-flight leaping dog frames). `DOGP.SHP` vs ADOG's `ADOGP.SHP` |
| `AA=no` | Cannot target air |
| `Arm=2` | TS-legacy "arming time" — bullet must travel 2 frames before counting; irrelevant for short-range leap, defensive default |
| `ROT=8` | Projectile rotation speed (8 of 256 = full turn / 32 frames). Comment notes the parser requires this key to enable `Rotates=` on the in-flight sprite, letting the dog face its travel direction |
| `Shadow=no` | No drop shadow under the leaping dog |
| `Proximity=yes` | Triggers detonation on coming within range of target (rather than direct impact pixel test) |
| `Ranged=yes` | Uses the weapon's `Range=` field to set max travel distance |
| `FirersPalette=yes` | Use the firer's house palette (Soviet team colour) instead of the projectile's own palette |
| `SubjectToCliffs=no` | Ignores cliff-blocking — dog can leap over short cliffs as long as range permits |
| `SubjectToElevation=no` | Damage not scaled by terrain height difference |
| `SubjectToWalls=yes` | Walls block the leap (dog cannot bite through a Soviet wall) |

ADOGJUMP is identical except `Image=ADOGP`. See
[ADOG §Projectile](../allied/ADOG.md#projectile---adogjump) for full
annotation.

### Warhead — `[ParasiteDog]`

Shared with ADOG. See [ADOG §Warhead](../allied/ADOG.md#warhead---parasitedog)
for the full annotation. Key effect: `Parasite=yes` with infantry-only
`Verses=` causes the dog to **consume** any target infantry (100% damage at
all veterancy classes; armor=plate/medium/heavy/wood all 0%).

### Secondary — `[VirtualScanner]`

Same `rulesmd.ini:23619` block as the engineers — `Range=5`, `NeverUse=yes`,
pure target-scan range extender for the AI's guard-mission. See
[ADOG §Secondary](../allied/ADOG.md#secondary---virtualscanner) for full
annotation.

---

## Voices and sounds

`c:/Users/enok/Documents/ra2-rust-game/ini/soundmd.ini`:

| INI key on DOG | soundmd block | Resolved samples |
|----------------|---------------|------------------|
| `VoiceSelect=DogSelect` | `[DogSelect]` line 990 | `idogsela` (FShift=-5..5, Volume=85) |
| `VoiceMove=DogMove` | `[DogMove]` line 985 | `idogmova` (FShift=-5..5, Volume=35) |
| `VoiceAttack=DogAttackCommand` | `[DogAttackCommand]` line 980 | `idogatca` (FShift=-5..5, Volume=70) |
| `VoiceFeedback=DogFear` | `[DogFear]` line 995 | `idogfea` `idogfeb` `idogfec` (random interrupt, FShift=-5..5) |
| `VoiceSpecialAttack=DogMove` | (same as VoiceMove) | reuses move bark |
| `DieSound=DogDie` | `[DogDie]` line 1002 | `idogdiea` (Priority=low, FShift=-5..5) |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` | `igensqua` |
| Weapon `BadTeeth` `Report=DogAttack` | `[DogAttack]` sound block | growl-bark on each leap-bite — **shared with GoodTeeth** |

**Voice bank is identical to ADOG** — both factions use the `Dog*` sound
blocks (no `EngSov`-style faction split for dog vocalisations). The
`FShift=-5..5` randomises pitch by ±5 semitones each play, giving the
illusion of multiple distinct dogs without separate samples.

---

## Prerequisites, owners, tech

- `Prerequisite=Barracks` — generic. For Soviet houses resolves to `NAHAND`.
- `Owner=` (all 10) ∩ `¬ForbiddenHouses=` (excludes 5 Allied + 1 Yuri) →
  effective owner: **Russians, Confederation, Africans, Arabs** (all 4
  Soviet houses).
- `TechLevel=2` — buildable once Barracks is up (effectively from match
  start since Barracks is the prereq itself).
- `AllowedToStartInMultiplayer=no` — never in lobby starting-unit list.
- `BuildLimit=`, `RequiredHouses=`, `AIBasePlanningSide=` — all unset.

---

## Veterancy and upgrades

Identical to ADOG:

- `Trainable=no` — dog is excluded from veterancy XP awards.
- `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER,SCATTER` — defined
  but unreachable because `Trainable=no`. Defensive presence (engineer-style
  pattern — see [ADOG §Veterancy](../allied/ADOG.md#veterancy-and-upgrades))
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — same: defined but
  unreachable.

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

**There is no DOG-specific code in gamemd.exe.** All behavior is driven by
shared flags on `InfantryTypeClass` / `TechnoTypeClass` / `WeaponTypeClass` /
`WarheadTypeClass` and routes through the same paths documented in
[ADOG.md §Hardcoded behavior](../allied/ADOG.md#hardcoded-behavior-in-gamemdexe-ghidra-verified):

- **Leap-bite mechanism**: `WeaponTypeClass+LimboLaunch` bit on
  `BadTeeth`/`GoodTeeth` → engine limbos the dog at fire-time, animates the
  jump projectile (`DOGJUMP`/`ADOGJUMP`) along an arc, then either re-spawns
  the dog at target (success) or at firer (target died mid-leap). Same code
  path as Giant Squid grab and Yuri Initiate psychic blast launch.
- **Parasite warhead**: `Warhead=ParasiteDog` with `Parasite=yes` triggers
  the parasite-consume path on infantry impact. The infantry-only `Verses=`
  ensures vehicles/buildings/aircraft take 0% damage from a stray bite.
- **DetectDisguise** (`TechnoTypeClass+0x6E8`, per RE cheat-sheet): each
  tick the dog's sight scan reveals disguised Spies and Mirage Tanks within
  `Sight=9` cells.
- **NotHuman + ImmuneToPsionics** (`InfantryTypeClass+0xEC2` /
  `TechnoTypeClass+0x6F1`): mind-control attempts (Yuri, Yuri Prime,
  Psychic Tower, Psychic Dominator AoE) skip dogs entirely. The
  `NotHuman=yes` flag also blocks Cloning Vat duplication and Soylent Green
  refund variations.
- **IFVMode=0**: passenger gunner-table index 0 = "use chassis's own
  Weapon1", i.e. no swap. Net effect: dog cannot meaningfully convert an
  IFV (the IFV defaults to its base Maverick missile). See
  [ADOG §IFVMode](../allied/ADOG.md#ifv-passenger-quirk).
- **ReselectIfLimboed / RejoinTeamIfLimboed** (`TechnoTypeClass` flags):
  during the LimboLaunch leap, the dog is removed from the world; these
  flags ensure selection state and team membership survive the brief limbo
  period.
- **DefaultToGuardArea** (`TechnoTypeClass+0x6CE`): idle dogs default to
  `MissionGuardArea` rather than `MissionGuard`, expanding their target
  acquisition radius to `GuardRange` and making them proactively chase
  intruders without explicit orders.

### Ghidra string-search results for "DOG", "BadTeeth"

- `search_strings "^DOG$"` (anchored) → **0 matches** (run 2026-05-17). Note
  ripgrep-style regex anchors are not supported by the Ghidra string search;
  this is interpreted as a literal pattern.
- `search_strings "BadTeeth"` → **0 matches** (run 2026-05-17).

Confirmed: gamemd.exe contains **no hardcoded branch** keyed off the section
name `DOG` or the weapon name `BadTeeth`. The engine reads the section into
the same `InfantryTypeClass` template as every other infantry; only the
flag bits (LimboLaunch on the weapon, Parasite on the warhead, DetectDisguise,
NotHuman, ImmuneToPsionics, NavalTargeting) drive behavior.

The string `"DOG"` does appear in gamemd.exe as a substring of unrelated
strings (e.g. file paths, SHP names like `DOGP.SHP`), but no standalone
section-name comparison exists in the code.

---

## TS-legacy filter

Same as ADOG:

- `Locomotor={4A582744-...}` — TS-era WalkLocomotionClass GUID, alive in YR.
- `MovementZone=InfantryDestroyer` commented out — copy-paste leftover from
  the TS Disk Thrower template, Westwood explicitly notes this in the INI
  comment.
- `NavalTargeting=6` — TS-era anti-amphibious targeting heuristic. Mostly
  inactive in YR (Soviet faction has Tesla Trooper, Conscript, Crazy Ivan,
  but the dog rarely encounters amphibious infantry); defensive flag.
- `ImmuneToRadiation=no` — explicit; this is YR-relevant (Desolator
  radiation does kill dogs). Not TS legacy.
- `Bombable=yes` — Crazy Ivan can attach a bomb to a dog. Active in YR.
- `Natural=yes` — TS-era flag; in YR this affects the Genetic Mutator
  superweapon (`Natural=yes` units are exempt from being mutated into
  Brutes). Active in YR.

---

## Cross-references

- **Canonical dossier**: [ADOG](../allied/ADOG.md) — full key-by-key rules,
  art, weapon, projectile, warhead, voice, hardcoded-behavior coverage.
  This doc enumerates Soviet-specific deltas only.
- **Variants in the dog family**:
  - [ADOG](../allied/ADOG.md) — Allied Attack Dog (canonical).
  - [YDOG](../yuri/YDOG.md) — Yuri-built Soviet variant (`Image=DOG`,
    `Primary=BadTeeth`, RequiredHouses=YuriCountry).
  - [YADOG](../yuri/YADOG.md) — Yuri-built Allied variant (`Image=ADOG`,
    `Primary=GoodTeeth`, RequiredHouses=YuriCountry).
- **Builder**: NAHAND (Soviet Barracks). Yuri can also build SENGINEER-tier
  units but Yuri's dog variants come from YABRCK.
- **Counter targets** (what DOG kills most efficiently): all enemy infantry,
  especially [SPY](../allied/SPY.md), [TANY](../allied/TANY.md),
  [GHOST](../allied/GHOST.md), [IVAN](../soviet/IVAN.md),
  [ENGINEER](../allied/ENGINEER.md) — single-bite kills via Parasite
  warhead. Cannot bite [YURI](../yuri/YURI.md)/[YURIPR](../yuri/YURIPR.md)
  before being mind-controlled? Actually **yes** can — `ImmuneToPsionics=yes`
  on dogs means Yuri cannot mind-control them, so dog wins the matchup.
- **Vulnerable to**: any non-infantry attacker, Tanya/SEAL gunfire,
  Desolator radiation, Crazy Ivan bombs (`Bombable=yes`).
- **Special interactions**:
  - **DetectDisguise** reveals disguised spies and Mirage Tanks within
    Sight=9.
  - **NotHuman=yes** exempts dogs from Cloning Vats duplication (NACLON),
    Soylent Green discount variations, and Genetic Mutator (`Natural=yes`
    also contributes here).

---

## Coverage audit

- ✅ Every key in `[DOG]` rulesmd block (47 lines, line 4369–4417) covered —
  explicit table for the 4 keys that differ from ADOG, plus reference to
  canonical dossier for the 41 identical keys.
- ✅ Every key in `[DOG]` artmd block (8 lines, line 366–373) covered.
- ✅ Weapon chain: BadTeeth (verbatim) → DOGJUMP projectile (verbatim) →
  ParasiteDog warhead (delegated to canonical). Difference vs GoodTeeth
  flagged: Image=DOGP vs ADOGP only.
- ✅ Sound chain: 7 distinct soundmd entries enumerated. Voice bank is
  **shared** with ADOG — no Soviet-specific dog barks.
- ✅ Ghidra search: `search_strings "DOG"`/`"BadTeeth"` → 0 matches.
  Confirms no hardcoded section-name or weapon-name branch.
- ✅ TS-legacy filter applied (Locomotor GUID, MovementZone=InfantryDestroyer
  comment, NavalTargeting, Natural=yes Genetic Mutator interaction).
- ✅ Cross-references to ADOG, YDOG, YADOG, NAHAND, counter/vulnerable
  matchups, NACLON / Genetic Mutator interactions.
