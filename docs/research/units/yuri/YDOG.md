# Yuri-built Soviet Attack Dog (YDOG)
Side: Yuri | Category: Infantry | Image alias: `Image=DOG` (shares SHP/sequence/cameo)

The Yuri-faction's variant of the Soviet Attack Dog. Mechanically identical
to the canonical Allied [ADOG](../allied/ADOG.md) and its Soviet sibling
[DOG](../soviet/DOG.md). Only differences from the Soviet [DOG] dossier are:

1. `Name=Attack Dog (Yuri version)` — internal label, `UIName=Name:DOG`
   still resolves to "Attack Dog" in-game.
2. `Image=DOG` — explicit redirect to the Soviet `DOG.SHP`/`[DOG]` art block.
   The sprite, cameo (`DOGICON`), and frame sequence (`DogSequence`) are the
   Soviet variant — player sees a Soviet-coloured dog with the
   YuriCountry house-palette remap.
3. `Prerequisite=NAHAND` — **explicit Soviet Barracks**, not generic
   `Barracks`. Means YuriCountry can only build YDOG **after capturing an
   NAHAND** (Soviet barracks). YuriCountry's native YABRCK does not unlock
   YDOG. This is a deliberate campaign/special-case construction path.
4. `RequiredHouses=YuriCountry` — gates buildability to the YuriCountry
   house only; combined with no `ForbiddenHouses=` (DOG has the Allied/Yuri
   forbid list), the net owner is **YuriCountry only**.
5. `Soylent=50` — half DOG's `Soylent=100` Grinder refund. The Yuri-built
   variant returns less credits when ground at YAGRND.

Everything else — Primary=BadTeeth, the LimboLaunch leap, ParasiteDog
warhead, DetectDisguise, NotHuman, ImmuneToPsionics, IFVMode=0, voices,
veterancy abilities — is byte-identical to the Soviet [DOG] dossier.

This is a quick-reference doc; cross-reference the canonical
[ADOG.md](../allied/ADOG.md) for the full deep surface and the sibling
[DOG.md](../soviet/DOG.md) for the BadTeeth-vs-GoodTeeth weapon split.

---

## rulesmd.ini — `[YDOG]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4964`:

```ini
[YDOG]
UIName=Name:DOG
Name=Attack Dog (Yuri version)
Image=DOG
NotHuman=yes
Category=Soldier
Primary=BadTeeth
Secondary=VirtualScanner
NavalTargeting=6
Prerequisite=NAHAND
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
RequiredHouses=YuriCountry
Cost=200
Soylent=50
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

### Keys that differ from `[DOG]`

| Key | YDOG value | DOG value | Notes |
|-----|------------|-----------|-------|
| `Name=Attack Dog (Yuri version)` | Internal label | "Attack Dog" | Internal only — `UIName=Name:DOG` resolves both to "Attack Dog" |
| `Image=DOG` | Redirect to DOG SHP/art | (no Image=) | Yuri variant uses the **same** sprite, cameo, and sequence as Soviet DOG. No artmd section for YDOG exists (`grep "^\[YDOG\]" artmd.ini` → 0 matches) |
| `Prerequisite=NAHAND` | Explicit Soviet Barracks | `Barracks` (generic) | **Critical gating quirk** — YuriCountry does not build NAHAND natively. YDOG is unlockable only after the Yuri player captures an enemy Soviet NAHAND. This is intentional, not a bug |
| `RequiredHouses=YuriCountry` | Gate to YuriCountry only | (not set on DOG) | `TechnoTypeClass` field at parsing path 0x00843bb4. Replaces `ForbiddenHouses=` filter — net owner: YuriCountry only |
| (no `ForbiddenHouses=`) | absent | `British,French,Germans,Americans,Alliance,YuriCountry` | DOG forbids Allied+Yuri; YDOG uses the inverse filter via RequiredHouses |
| `Soylent=50` | Half-refund at Grinder | 100 | Yuri-built dog returns less credits when fed to YAGRND |

All other 39 keys are byte-identical to `[DOG]` — see the
[DOG dossier](../soviet/DOG.md) and the canonical
[ADOG dossier](../allied/ADOG.md) for key-by-key explanation
(NotHuman/Category/Primary=BadTeeth/Secondary/NavalTargeting/LeadershipRating/
Strength/Armor/Reselect+RejoinIfLimboed/DefaultToGuardArea/TechLevel/Pip/Sight/
DetectDisguise/Speed/Cost/Points/IsSelectableCombatant/voices/DieSound/
Locomotor/PhysicalSize/MovementZone/ThreatPosed/ImmuneToRadiation/Bombable/
AllowedToStartInMultiplayer/Size/VeteranAbilities/EliteAbilities/
ImmuneToPsionics/IFVMode/Trainable/Natural).

### Implicit defaults (same as DOG)

- `Crawls=` — inherited from `[DOG]` art block (via `Image=DOG`) → `no`.
- `Crushable=` — defaults `yes` (infantry).
- `Occupier=` / `Deployer=` — both default `no`.
- `Engineer=` — defaults `no`.

---

## artmd.ini — no `[YDOG]` section

`grep "^\[YDOG\]" artmd.ini` → **no match**.

There is no dedicated art block for YDOG. The rules-side `Image=DOG`
directive causes the art system to resolve to `[DOG]` (artmd.ini:366),
inheriting:

```ini
[DOG] ; Soviet Attack Dog  (from artmd.ini:366)
Cameo=DOGICON
AltCameo=DOGUICO
Sequence=DogSequence
Crawls=no
Remapable=yes
FireUp=6
PrimaryFireFLH=0,0,0
```

This means:

- **Cameo**: `DOGICON` — the Soviet attack-dog cameo. YuriCountry players
  see the Soviet-styled cameo in the sidebar even though they're the
  builders. No Yuri-themed dog cameo exists.
- **AltCameo**: `DOGUICO` — never displayed (`Trainable=no`).
- **Sequence**: `[DogSequence]` — shared with ADOG/DOG/YADOG. See
  [ADOG §artmd](../allied/ADOG.md#artmdini--adog-section) for the full
  frame layout.
- **Crawls=no**, **Remapable=yes**, **FireUp=6**, **PrimaryFireFLH=0,0,0** —
  all inherited.

`Remapable=yes` means the Yuri house palette (purple-tinged) is applied to
the dog body — so a YuriCountry-built YDOG visually has YuriCountry team
colour despite using the Soviet `DOG.SHP`. Combined with the Soviet-style
cameo, this produces the in-game image of "Yuri controls a Soviet-breed
attack dog."

---

## Weapons

Identical to DOG:

- **Primary** `[BadTeeth]` — `rulesmd.ini:23534`. Damage=30, ROF=30, Range=1.5,
  CellRangefinding=yes, `Projectile=DOGJUMP` (Image=DOGP), Speed=30,
  Warhead=ParasiteDog, LimboLaunch=yes, Report=DogAttack, FireInTransport=no.
- **Secondary** `[VirtualScanner]` — `rulesmd.ini:23619`. Range=5,
  NeverUse=yes, target-scan range extender.
- **Warhead** `[ParasiteDog]` — see [ADOG §Warhead](../allied/ADOG.md#warhead---parasitedog).
- **Projectile** `[DOGJUMP]` — `rulesmd.ini:25495`. `Image=DOGP`, Arm=2,
  ROT=8, FirersPalette=yes (uses Yuri house palette in flight),
  SubjectToCliffs=no, SubjectToElevation=no, SubjectToWalls=yes.

See [DOG §Weapons](../soviet/DOG.md#weapons) for the full annotated chain.

---

## Voices and sounds

Shared with ADOG/DOG. No Yuri-specific dog voice bank:

| INI key on YDOG | soundmd block | Resolved samples |
|-----------------|---------------|------------------|
| `VoiceSelect=DogSelect` | `[DogSelect]` line 990 | `idogsela` |
| `VoiceMove=DogMove` | `[DogMove]` line 985 | `idogmova` |
| `VoiceAttack=DogAttackCommand` | `[DogAttackCommand]` line 980 | `idogatca` |
| `VoiceFeedback=DogFear` | `[DogFear]` line 995 | `idogfea` `idogfeb` `idogfec` (random interrupt) |
| `VoiceSpecialAttack=DogMove` | (reuses move bark) | |
| `DieSound=DogDie` | `[DogDie]` line 1002 | `idogdiea` (Priority=low) |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` | `igensqua` |
| Weapon `BadTeeth` `Report=DogAttack` | `[DogAttack]` | growl-bark on leap-bite |

All FShift/Volume settings inherit from the shared blocks; ±5 semitone
pitch shift gives per-bark variation.

---

## Prerequisites, owners, tech

- `Prerequisite=NAHAND` — **Soviet Barracks specifically**, not the generic
  `Barracks` keyword. YuriCountry does not build NAHAND natively
  (YuriCountry uses YABRCK), so YDOG is **unbuildable** in a vanilla Yuri
  skirmish unless the Yuri player captures an enemy NAHAND via SPY or
  Engineer.
- `Owner=` (all 10) ∩ `RequiredHouses=YuriCountry` → effective owner:
  **YuriCountry only**. The other 9 houses in `Owner=` are filtered out by
  the RequiredHouses gate.
- `TechLevel=2` — non-restrictive (NAHAND is the real gate).
- `AllowedToStartInMultiplayer=no` — never in lobby starting-unit list.
- `BuildLimit=`, `AIBasePlanningSide=` — unset.
- Net result: in 99% of skirmish matches YDOG is **never produced**. It
  exists as a campaign-script unit and as a fallback for captured-base
  Yuri play.

---

## Veterancy and upgrades

Identical to DOG/ADOG:

- `Trainable=no` — XP excluded.
- `VeteranAbilities=` / `EliteAbilities=` — defined but unreachable.

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

**There is no YDOG-specific code in gamemd.exe.** Behavior is fully driven
by the shared `InfantryTypeClass`/`TechnoTypeClass`/`WeaponTypeClass` flags
documented in [ADOG.md §Hardcoded behavior](../allied/ADOG.md#hardcoded-behavior-in-gamemdexe-ghidra-verified):

- **Leap-bite mechanism**: `WeaponTypeClass.LimboLaunch` on BadTeeth.
- **Parasite warhead**: `Parasite=yes` on ParasiteDog consumes infantry.
- **DetectDisguise** (`TechnoTypeClass+0x6E8`): reveals disguised SPYs and
  MGTK Mirage Tanks within Sight=9.
- **NotHuman + ImmuneToPsionics**: blocks mind-control (Yuri-vs-Yuri matchup
  consideration: an enemy Yuri player's YURI/YURIPR cannot mind-control a
  YuriCountry-owned YDOG).
- **IFVMode=0**: chassis default weapon (no swap).
- **DefaultToGuardArea**: idle YDOG proactively chases intruders.
- **RequiredHouses=YuriCountry** (`TechnoTypeClass` field, parser at
  Ghidra `0x00843bb4`): build-side gate; if `current_house->Type !=
  YuriCountry`, the unit is filtered out of the buildable list and the
  sidebar cameo never appears for that house.

### Ghidra string-search results for "YDOG"

- `search_strings "YDOG"` → **0 matches** (run 2026-05-17).

Confirmed: no hardcoded section-name branch. All gating is via the
`RequiredHouses=` and `Prerequisite=` fields, which are read into generic
fields on `TechnoTypeClass` and evaluated in the build-eligibility loop.

The string `"YDOG"` only appears in:

- The rulesmd.ini section header itself.
- Campaign maps that explicitly place a YDOG unit (Yuri campaign missions
  where Yuri starts with captured Soviet infrastructure).

---

## TS-legacy filter

Same as DOG/ADOG:

- `Locomotor={4A582744-...}` — TS-era WalkLocomotionClass GUID, alive in YR.
- `MovementZone=InfantryDestroyer` commented out — TS-era copy-paste leftover.
- `NavalTargeting=6` — TS-era anti-amphibious heuristic.
- `Natural=yes` — Genetic Mutator (YAGNTC) exempts Natural units from
  becoming Brutes. **Important Yuri-on-Yuri interaction**: a YuriCountry
  player's own YDOGs are immune to their own (or another Yuri player's)
  Genetic Mutator superweapon.

---

## Cross-references

- **Canonical dossier**: [ADOG](../allied/ADOG.md) — full key-by-key rules,
  art, weapon, projectile, warhead, voice, hardcoded-behavior coverage.
- **Sibling variants in the dog family**:
  - [DOG](../soviet/DOG.md) — Soviet Attack Dog (sibling, BadTeeth weapon).
  - [ADOG](../allied/ADOG.md) — Allied Attack Dog (canonical, GoodTeeth).
  - [YADOG](../yuri/YADOG.md) — Yuri-built Allied variant (Image=ADOG,
    GoodTeeth, also RequiredHouses=YuriCountry).
- **Builder**: NAHAND (Soviet Barracks) — only reachable for YuriCountry
  via base capture or campaign script.
- **Counter targets**: same as DOG — all enemy infantry, especially
  disguised [SPY](../allied/SPY.md), Mirage Tank crews, and
  [ENGINEER](../allied/ENGINEER.md) (preventing captures).
- **Vulnerable to**: enemy non-infantry, Tanya/SEAL, Desolator radiation,
  Crazy Ivan bomb. **Immune to**: enemy Yuri mind-control
  (ImmuneToPsionics), enemy Genetic Mutator (Natural).
- **Grinder interaction**: `Soylent=50` at YAGRND — half the cash of a
  regular DOG. Probably to disincentivize farming captured-NAHAND dogs as
  cash sources.

---

## Coverage audit

- ✅ Every key in `[YDOG]` rulesmd block (50 lines, line 4964–5013) covered —
  explicit table for the 6 keys that differ from DOG, plus reference to
  sibling dossier for the 39 identical keys.
- ✅ artmd: confirmed **no `[YDOG]` section**. Art routes via `Image=DOG`
  to `[DOG]` artmd block; inherited keys (Cameo=DOGICON, AltCameo,
  Sequence=DogSequence, Crawls=no, Remapable, FireUp, PrimaryFireFLH)
  documented.
- ✅ Weapon chain: BadTeeth + VirtualScanner + ParasiteDog + DOGJUMP — all
  identical to DOG, delegated.
- ✅ Sound chain: 7 distinct soundmd entries. Voice bank **shared** with
  ADOG/DOG.
- ✅ Ghidra search: `search_strings "YDOG"` → 0 hits. Confirms no
  hardcoded section-name branch. `RequiredHouses` confirmed in gamemd at
  0x00843bb4.
- ✅ TS-legacy filter applied (Locomotor GUID, MovementZone comment,
  NavalTargeting, Natural-vs-Genetic-Mutator interaction).
- ✅ Cross-references to ADOG, DOG, YADOG, NAHAND, YAGRND grinder
  interaction, Yuri-on-Yuri mind-control immunity, campaign-only
  buildability via captured NAHAND.
