# Yuri-built Allied Attack Dog (YADOG)
Side: Yuri | Category: Infantry | Image alias: `Image=ADOG` (shares SHP/sequence/cameo)

The Yuri-faction's variant of the Allied Attack Dog. Mechanically identical
to the canonical Allied [ADOG](../allied/ADOG.md) and its Yuri sibling
[YDOG](./YDOG.md). Only differences from the canonical [ADOG] dossier are:

1. `Name=Allied Attack Dog (Yuri version)` — internal label only;
   `UIName=Name:DOG` resolves to "Attack Dog" in-game.
2. `Image=ADOG` — explicit redirect to the Allied `ADOG.SHP`/`[ADOG]` art
   block. Player sees an Allied-styled dog with the YuriCountry house-palette
   remap.
3. `Prerequisite=GAPILE` — **explicit Allied Barracks**, not generic
   `Barracks`. Means YuriCountry can only build YADOG **after capturing a
   GAPILE** (Allied barracks). YuriCountry's native YABRCK does not unlock
   YADOG, and unlike its Soviet sibling YDOG (which requires NAHAND), YADOG
   requires a captured Allied barracks specifically.
4. `RequiredHouses=YuriCountry` — gates buildability to YuriCountry; no
   `ForbiddenHouses=` (ADOG has the Soviet/Yuri forbid list).
5. `Soylent=50` — half ADOG's `Soylent=100` Grinder refund.

Everything else — Primary=GoodTeeth, the LimboLaunch leap, ParasiteDog
warhead, DetectDisguise, NotHuman, ImmuneToPsionics, IFVMode=0, voices,
veterancy abilities — is byte-identical to the canonical Allied [ADOG]
dossier.

This is a quick-reference doc; cross-reference the canonical
[ADOG.md](../allied/ADOG.md) for the full deep surface and the sibling
[YDOG.md](./YDOG.md) for the parallel Soviet-captured-base variant.

---

## rulesmd.ini — `[YADOG]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4913`:

```ini
[YADOG]
UIName=Name:DOG
Name=Allied Attack Dog (Yuri version)
NotHuman=yes
Image=ADOG
Category=Soldier
Primary=GoodTeeth
Secondary=VirtualScanner
NavalTargeting=6
Prerequisite=GAPILE
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
ImmuneToPsionics=yes
IFVMode=0
Trainable=no
Natural=yes
```

### Keys that differ from `[ADOG]`

| Key | YADOG value | ADOG value | Notes |
|-----|-------------|------------|-------|
| `Name=Allied Attack Dog (Yuri version)` | Internal label | "Allied Attack Dog" | Internal only — `UIName=Name:DOG` resolves both to "Attack Dog" |
| `Image=ADOG` | Redirect to ADOG SHP/art | (no Image=) | Yuri variant uses the same sprite, cameo, and sequence as Allied ADOG. No artmd section for YADOG exists (`grep "^\[YADOG\]" artmd.ini` → 0 matches) |
| `Prerequisite=GAPILE` | Explicit Allied Barracks | `Barracks` (generic) | **Critical gating quirk** — YuriCountry does not build GAPILE natively. YADOG is unlockable only after the Yuri player captures an enemy GAPILE. The Soviet sibling YDOG requires NAHAND; the two together cover both Allied- and Soviet-captured-base play |
| `RequiredHouses=YuriCountry` | Gate to YuriCountry only | (not set on ADOG) | `TechnoTypeClass` field, parsed at Ghidra `0x00843bb4`. Net owner: YuriCountry only |
| (no `ForbiddenHouses=`) | absent | `Russians,Confederation,Africans,Arabs,YuriCountry` | ADOG forbids Soviet+Yuri; YADOG uses the inverse via RequiredHouses |
| `Soylent=50` | Half-refund at Grinder | 100 | Yuri-built dog returns less credits when fed to YAGRND |

All other 39 keys are byte-identical to `[ADOG]` — see the
[ADOG dossier](../allied/ADOG.md) for key-by-key explanation
(NotHuman/Category/Primary=GoodTeeth/Secondary/NavalTargeting/LeadershipRating/
Strength/Armor/Reselect+RejoinIfLimboed/DefaultToGuardArea/TechLevel/Pip/Sight/
DetectDisguise/Speed/Cost/Points/IsSelectableCombatant/voices/DieSound/
Locomotor/PhysicalSize/MovementZone/ThreatPosed/ImmuneToRadiation/Bombable/
AllowedToStartInMultiplayer/Size/VeteranAbilities/EliteAbilities/
ImmuneToPsionics/IFVMode/Trainable/Natural).

### Implicit defaults (same as ADOG)

- `Crawls=` — inherited from `[ADOG]` art block (via `Image=ADOG`) → `no`.
- `Crushable=` — defaults `yes` (infantry).
- `Occupier=` / `Deployer=` — both default `no`.
- `Engineer=` — defaults `no`.

---

## artmd.ini — no `[YADOG]` section

`grep "^\[YADOG\]" artmd.ini` → **no match**.

There is no dedicated art block for YADOG. The rules-side `Image=ADOG`
directive causes the art system to resolve to `[ADOG]` (artmd.ini:375),
inheriting:

```ini
[ADOG] ; Allied Attack Dog  (from artmd.ini:375)
Cameo=ADOGICON
AltCameo=ADOGUICO
Sequence=DogSequence
Crawls=no
Remapable=yes
FireUp=6
PrimaryFireFLH=0,0,0
```

This means:

- **Cameo**: `ADOGICON` — the Allied attack-dog cameo. YuriCountry players
  see the Allied-styled cameo in the sidebar even though they're the
  builders.
- **AltCameo**: `ADOGUICO` — never displayed (`Trainable=no`).
- **Sequence**: `[DogSequence]` — shared with ADOG/DOG/YDOG. See
  [ADOG §artmd](../allied/ADOG.md#artmdini--adog-section) for the full
  frame layout.
- **Crawls=no**, **Remapable=yes**, **FireUp=6**, **PrimaryFireFLH=0,0,0** —
  all inherited.

`Remapable=yes` causes the YuriCountry house palette to be applied to the
Allied dog SHP body, so a YuriCountry-built YADOG is visually the Allied
dog tinted Yuri-purple. Combined with the Allied-style cameo, this
produces the image of "Yuri controls an Allied-breed attack dog."

---

## Weapons

Identical to ADOG:

- **Primary** `[GoodTeeth]` — `rulesmd.ini:23547`. Damage=30, ROF=30,
  Range=1.5, CellRangefinding=yes, `Projectile=ADOGJUMP` (Image=ADOGP),
  Speed=30, Warhead=ParasiteDog, LimboLaunch=yes, Report=DogAttack,
  FireInTransport=no.
- **Secondary** `[VirtualScanner]` — `rulesmd.ini:23619`. Range=5,
  NeverUse=yes, target-scan range extender.
- **Warhead** `[ParasiteDog]` — see [ADOG §Warhead](../allied/ADOG.md#warhead---parasitedog).
- **Projectile** `[ADOGJUMP]` — `rulesmd.ini:25509`. `Image=ADOGP`, Arm=2,
  ROT=8, FirersPalette=yes (uses Yuri house palette in flight),
  SubjectToCliffs=no, SubjectToElevation=no, SubjectToWalls=yes.

See [ADOG §Weapons](../allied/ADOG.md#weapons) for the full annotated chain.

---

## Voices and sounds

Shared with ADOG/DOG/YDOG. No Yuri-specific dog voice bank:

| INI key on YADOG | soundmd block | Resolved samples |
|------------------|---------------|------------------|
| `VoiceSelect=DogSelect` | `[DogSelect]` line 990 | `idogsela` |
| `VoiceMove=DogMove` | `[DogMove]` line 985 | `idogmova` |
| `VoiceAttack=DogAttackCommand` | `[DogAttackCommand]` line 980 | `idogatca` |
| `VoiceFeedback=DogFear` | `[DogFear]` line 995 | `idogfea` `idogfeb` `idogfec` (random interrupt) |
| `VoiceSpecialAttack=DogMove` | (reuses move bark) | |
| `DieSound=DogDie` | `[DogDie]` line 1002 | `idogdiea` (Priority=low) |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` | `igensqua` |
| Weapon `GoodTeeth` `Report=DogAttack` | `[DogAttack]` | growl-bark on leap-bite |

All FShift/Volume settings inherit from the shared blocks.

---

## Prerequisites, owners, tech

- `Prerequisite=GAPILE` — **Allied Barracks specifically**, not the generic
  `Barracks` keyword. YuriCountry does not build GAPILE natively, so YADOG
  is **unbuildable** in a vanilla Yuri skirmish unless the Yuri player
  captures an enemy GAPILE via SPY/Engineer.
- `Owner=` (all 10) ∩ `RequiredHouses=YuriCountry` → effective owner:
  **YuriCountry only**.
- `TechLevel=2` — non-restrictive (GAPILE is the real gate).
- `AllowedToStartInMultiplayer=no` — never in lobby starting-unit list.
- `BuildLimit=`, `AIBasePlanningSide=` — unset.
- Net result: like YDOG, in 99% of skirmish matches YADOG is **never
  produced**. Campaign-script unit and captured-base fallback.

---

## Veterancy and upgrades

Identical to ADOG:

- `Trainable=no` — XP excluded.
- `VeteranAbilities=` / `EliteAbilities=` — defined but unreachable.

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

**There is no YADOG-specific code in gamemd.exe.** Behavior is fully driven
by the shared `InfantryTypeClass`/`TechnoTypeClass`/`WeaponTypeClass` flags
documented in [ADOG.md §Hardcoded behavior](../allied/ADOG.md#hardcoded-behavior-in-gamemdexe-ghidra-verified):

- **Leap-bite mechanism**: `WeaponTypeClass.LimboLaunch` on GoodTeeth.
- **Parasite warhead**: `Parasite=yes` on ParasiteDog consumes infantry.
- **DetectDisguise**: reveals disguised SPYs and Mirage Tanks within
  Sight=9.
- **NotHuman + ImmuneToPsionics**: mind-control immune (Yuri-vs-Yuri).
- **IFVMode=0**: chassis default weapon (no swap).
- **DefaultToGuardArea**: idle YADOG proactively chases intruders.
- **RequiredHouses=YuriCountry**: build-side gate.

### Ghidra string-search results for "YADOG"

- `search_strings "YADOG"` → **0 matches** (run 2026-05-17).

Confirmed: no hardcoded section-name branch. All gating is via the
`RequiredHouses=` and `Prerequisite=` fields, evaluated in the
build-eligibility loop.

The string `"YADOG"` only appears in the rulesmd.ini section header and
campaign maps that place a YADOG explicitly. No vtable override, no special
case.

---

## TS-legacy filter

Same as ADOG/DOG/YDOG:

- `Locomotor={4A582744-...}` — TS-era WalkLocomotionClass GUID, alive in YR.
- `MovementZone=InfantryDestroyer` commented out — TS-era copy-paste leftover.
- `NavalTargeting=6` — TS-era anti-amphibious heuristic.
- `Natural=yes` — Genetic Mutator (YAGNTC) exempts Natural units from
  becoming Brutes — YuriCountry's own YADOGs are immune to their own (or
  another Yuri player's) Genetic Mutator superweapon.

---

## Cross-references

- **Canonical dossier**: [ADOG](../allied/ADOG.md) — full key-by-key rules,
  art, weapon, projectile, warhead, voice, hardcoded-behavior coverage.
- **Sibling variants in the dog family**:
  - [DOG](../soviet/DOG.md) — Soviet Attack Dog.
  - [ADOG](../allied/ADOG.md) — Allied Attack Dog (canonical).
  - [YDOG](./YDOG.md) — Yuri-built Soviet variant (`Image=DOG`,
    BadTeeth, requires captured NAHAND).
- **Builder**: GAPILE (Allied Barracks) — only reachable for YuriCountry
  via base capture or campaign script.
- **Counter targets**: same as ADOG — all enemy infantry, especially
  disguised [SPY](../allied/SPY.md), Mirage Tank crews, and
  [ENGINEER](../allied/ENGINEER.md).
- **Vulnerable to**: enemy non-infantry, Tanya/SEAL, Desolator radiation,
  Crazy Ivan bomb. **Immune to**: enemy mind-control (ImmuneToPsionics),
  enemy Genetic Mutator (Natural).
- **Grinder interaction**: `Soylent=50` at YAGRND — half the cash of an
  ADOG. Same disincentive pattern as YDOG.

---

## Coverage audit

- ✅ Every key in `[YADOG]` rulesmd block (49 lines, line 4913–4961) covered —
  explicit table for the 6 keys that differ from ADOG, plus reference to
  canonical dossier for the 39 identical keys.
- ✅ artmd: confirmed **no `[YADOG]` section**. Art routes via `Image=ADOG`
  to `[ADOG]` artmd block; inherited keys documented.
- ✅ Weapon chain: GoodTeeth + VirtualScanner + ParasiteDog + ADOGJUMP — all
  identical to ADOG, delegated.
- ✅ Sound chain: 7 distinct soundmd entries. Voice bank **shared** with
  ADOG/DOG/YDOG.
- ✅ Ghidra search: `search_strings "YADOG"` → 0 hits. Confirms no
  hardcoded section-name branch.
- ✅ TS-legacy filter applied (Locomotor GUID, MovementZone comment,
  NavalTargeting, Natural-vs-Genetic-Mutator interaction).
- ✅ Cross-references to ADOG, DOG, YDOG, GAPILE, YAGRND grinder
  interaction, Yuri-on-Yuri mind-control immunity, campaign-only
  buildability via captured GAPILE.
