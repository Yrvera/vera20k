# PTROOP — Psi-Corp Trooper

**Side classification:** Yuri-themed (psi tech), universally buildable via tech-steal.
**Role:** Tech-steal psi unit — non-Yuri houses gain a mind-control infantry by infiltrating
a Yuri Battle Lab with a Spy.
**Tech-steal triplet:** CCOMAND (Allied tech) / TNKD (Soviet tech) / **PTROOP (Yuri tech)**.

> Output bar: indistinguishable from gamemd.exe for the player. INI is the source of
> truth; gamemd contains NO PTROOP-specific code path (string `PTROOP`/`PCOMMANDO`
> not present in `gamemd.exe`), so every behavior described below is derived from
> generic TechnoType / InfantryType / WeaponType / WarheadType handling driven by
> the values in this section.

---

## 1. `rulesmd.ini` — `[PTROOP]` verbatim

```ini
[PTROOP]
UIName=Name:PCOMMANDO
Name=Psi-Corp Trooper
Category=Soldier
Prerequisite=BARRACKS
RequiresStolenThirdTech=yes
Primary=MindControl
LeadershipRating=8
C4=yes
CrushSound=InfantrySquish
Crushable=no
TiberiumProof=yes
Strength=100
Armor=none
TechLevel=9
Pip=red
Sight=8
Speed=5
Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=1000
Soylent=500
Points=50
IsSelectableCombatant=yes
VoiceSelect=YuriSelect
VoiceMove=YuriMove
VoiceAttack=YuriAttackCommand
VoiceFeedback=
VoiceSpecialAttack=YuriMove
DieSound=YuriDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
;SpeedType=Amphibious
;MovementZone=AmphibiousDestroyer ; I am the only one with this zone, because it is now tied with being an infantry (part of seal stuck on tree bug)
ThreatPosed=25	; This value MUST be 0 for all building addons
SpecialThreatValue=1
ImmuneToVeins=yes
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
DetectDisguise=yes
ImmuneToPsionics=yes
;Deployer=yes
;DeployFire=yes
;UndeployDelay=75
ElitePrimary=MindControlE
IFVMode=4
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:PCOMMANDO` | AbstractType | CSF string key for sidebar/cursor label. (No matching `PCOMMANDO=` in repo `ini/` — string lives in `ra2md.csf`.) |
| `Name` | `Psi-Corp Trooper` | AbstractType | Internal/dev fallback name. Display name comes from CSF via `UIName`. |
| `Category` | `Soldier` | TechnoType | Group classifier for AI targeting; soldiers can be crushed by Crusher=yes vehicles unless `Crushable=no`. |
| `Prerequisite` | `BARRACKS` | TechnoType | Generic prereq token — resolves to any Allied/Soviet/Yuri barracks via `[AI] BuildBarracks=` mapping. Combined with `RequiresStolenThirdTech` below. |
| `RequiresStolenThirdTech` | `yes` | TechnoType (verified — see §6) | Buildable only by houses whose Spy has successfully infiltrated a Yuri Battle Lab. Acts as an AND-gate on top of `Prerequisite` + `Owner`. |
| `Primary` | `MindControl` | TechnoType | Main weapon — see §3. Damage=1 (link only), Range=7, ROF=200, Warhead=Controller. |
| `LeadershipRating` | `8` | TechnoType | Used in TS-era leadership/morale; in YR this contributes only marginally to `WhipPower`-style multipliers and is effectively cosmetic. (TS-LEGACY) |
| `C4` | `yes` | TechnoType | Unit can place demolition charges on buildings — see §7 hardcoded behavior. |
| `CrushSound` | `InfantrySquish` | TechnoType | Sound triggered when this unit is crushed by a vehicle. Sound def below. |
| `Crushable` | `no` | TechnoType | Cannot be crushed by tanks (override of `Soldier` default). Matches CCOMAND, IVAN, TANY, BORIS, SEAL: special infantry are non-crushable. |
| `TiberiumProof` | `yes` | **InfantryType** (verified scope: cheat sheet 0x00524xxx range) | TS-legacy: immunity to walking-on-tiberium damage. In YR there is no tiberium so this flag is dormant. **TS-LEGACY** (see §8). |
| `Strength` | `100` | AbstractType | Hitpoints. |
| `Armor` | `none` | TechnoType | Armor class. `Verses` row 1 against this armor. |
| `TechLevel` | `9` | TechnoType | Build-tree slot; combined with `RequiresStolenThirdTech` this acts as final-tech gate. `TechLevel=-1` would disable; `=9` keeps it visible at endgame. |
| `Pip` | `red` | InfantryType | Color of the carry-passenger pip in transports. |
| `Sight` | `8` | TechnoType | Reveal radius in cells. |
| `Speed` | `5` | TechnoType | Move speed (same as YURI, CCOMAND, TANY-era infantry). |
| `Owner` | full 10-country list | TechnoType | Every house can own PTROOP — but `RequiresStolenThirdTech` + `AllowedToStartInMultiplayer=no` gate actual availability. |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Cannot appear in start-game tech tree; unlocked only via Spy infiltration of Yuri lab. |
| `Cost` | `1000` | TechnoType | Build cost. |
| `Soylent` | `500` | TechnoType | Refund when fed to a Grinder (50% of cost — same Soylent/Cost ratio as YURI, INIT, BRUTE, CCOMAND). |
| `Points` | `50` | TechnoType | Score awarded on kill. |
| `IsSelectableCombatant` | `yes` | TechnoType | Counted in "select all combat units" + relevant for combat-AI threat. |
| `VoiceSelect` | `YuriSelect` | TechnoType | Re-uses YURI's selection voice (5 random "yes my master" clips). See §5. |
| `VoiceMove` | `YuriMove` | TechnoType | Re-uses YURI's move voice. |
| `VoiceAttack` | `YuriAttackCommand` | TechnoType | Re-uses YURI's attack voice. |
| `VoiceFeedback` | *(empty)* | TechnoType | No "under attack" voice line. |
| `VoiceSpecialAttack` | `YuriMove` | TechnoType | Reused as alternate attack voice. Verified scope: `VoiceSecondaryWeaponAttack` is on cheat sheet at TechnoType range. |
| `DieSound` | `YuriDie` | TechnoType | Death sound (3 random clips). |
| `Locomotor` | `{4A582744-9839-11d1-B709-00A024DDAFD1}` | TechnoType | `WalkLocomotionClass` CLSID — standard infantry biped. |
| `PhysicalSize` | `1` | TechnoType | Sub-cell footprint (1×1). |
| `MovementZone` | `Infantry` | TechnoType | Pathing zone — can step onto cells that allow infantry. |
| `;SpeedType=Amphibious` | *(commented)* | — | Author-note: PTROOP cannot swim; commented out to share the SEAL-tree-stuck bug fix. |
| `;MovementZone=AmphibiousDestroyer` | *(commented)* | — | Same note. |
| `ThreatPosed` | `25` | TechnoType | AI threat weight when scanning enemies. Mid-tier (Tanya=40, GI=10). |
| `SpecialThreatValue` | `1` | TechnoType | Multiplier for AI special-threat (high-value target marker). |
| `ImmuneToVeins` | `yes` | TechnoType | **TS-LEGACY** — veinholes are not in YR. Flag is read but no live consumer triggers in standard YR play. (See §8.) |
| `VeteranAbilities` | `STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | TechnoType | Bonuses applied at veteran rank — see §9. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Bonuses **added** at elite (cumulative with veteran). Note: no `FASTER` repeat, no `SIGHT` — elite swaps to self-heal + extra firepower. |
| `DetectDisguise` | `yes` | TechnoType (verified 0x0071443f) | Sees through Mirage Tank / Spy / CCOMAND disguises within `DetectDisguiseRange` (default 1). |
| `ImmuneToPsionics` | `yes` | TechnoType (verified 0x00714fa7) | Cannot be mind-controlled. Distinct from `ImmuneToPsionicWeapons` (0x00714fc8), which also blocks Psychic Dominator AoE etc. PTROOP only has the standard `ImmuneToPsionics`. See §7 for the behavioural distinction. |
| `;Deployer=yes / ;DeployFire=yes / ;UndeployDelay=75` | *(commented)* | — | Dead design churn. Suggests an earlier design where PTROOP deployed to fire (Desolator-like). The shipped unit has no deploy, no `Secondary`, no deploy-fire weapon. Do **not** implement; these lines are inert. |
| `ElitePrimary` | `MindControlE` | TechnoType | Elite-rank primary — Damage=10 (vs 1 at veteran/rookie — see §3), Range=14 (double), still 1 mind-control link. |
| `IFVMode` | `4` | TechnoType (verified 0x00714787) | When boarded into the Allied IFV (`[FV]`), this index selects the **Pistol turret** mode (`PistolTurretWeapon=4` → `Weapon5=CRMP5`). Quirk: PTROOP in an IFV fires `CRMP5` (SEAL's MP5), **NOT** a mind-control weapon. This is a vanilla YR design quirk shared with SEAL, TANY, CCOMAND, BORIS (all IFVMode=4). The dedicated psi-control IFV slot is `IFVMode=8` (used by YURI), not 4. |

---

## 2. `artmd.ini` — `[PTROOP]` section and animation sequence

### `[PTROOP]` art block

```ini
[PTROOP] ; P Trooper
Cameo=PSICICON
AltCameo=PSICUICO
Sequence=PsiTroopSequence
Crawls=yes
Remapable=yes
FireUp=6
PrimaryFireFLH=15,0,140
```

| Key | Value | Effect |
|-----|-------|--------|
| `Cameo` | `PSICICON` | Standard-tier build cameo. SHP filename `PSICICON.SHP`, palette `cameo.pal`. |
| `AltCameo` | `PSICUICO` | Yuri-faction-skinned cameo (used when the owning house is a Yuri faction). Engine picks `Cameo` vs `AltCameo` by `[Country] AlternateArrowCameos` and house side. |
| `Sequence` | `PsiTroopSequence` | Pointer to walk/idle/fire/death frame table — see below. |
| `Crawls` | `yes` | Has crawl/prone animation (uses `Crawl=` and `Prone=` rows from sequence). Engaged when force-fired upon or pinned by suppression. |
| `Remapable` | `yes` | House-color remap palette applied. |
| `FireUp` | `6` | Number of frames into the `FireUp` sequence before the projectile spawns (used to time muzzle/effect). |
| `PrimaryFireFLH` | `15,0,140` | Firing-pixel offset from infantry centre in voxel units (X=15 forward, Y=0 lateral, Z=140 height). High Z = beam emanates from head. |

### `[PsiTroopSequence]` referenced sequence

```ini
[PsiTroopSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Crawl=86,6,6
Prone=86,1,6
Die1=134,15,0
Die2=149,15,0
FireUp=164,6,6
FireProne=212,6,6
Down=260,2,2
Up=276,2,2
Paradrop=0,1,0
Cheer=307,8,0,E
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Panic=8,6,6
Deploy=292,7,0
Deployed=299,2,0 ; middle frame of deploy
Undeploy=301,6,0
```

Each row: `start_frame, frame_count, facings`. Optional 4th col is direction-lock (`S`=south, `E`=east, etc.).

| Row | Notes |
|-----|-------|
| `Ready=0,1,1` | Stationary stand pose, 1 frame, 1 facing. |
| `Walk=8,6,6` | 6-frame walk cycle × 6 facings. |
| `Idle1/Idle2` | Two unique idle anims (S- and E-locked). |
| `Crawl/Prone=86,...` | Crawl and prone share start frame 86 (Prone is single-frame still). |
| `Die1=134 / Die2=149` | Two ~15-frame death animations. The other `Die3/4/5=0,1,1` are stubs — there are only two unique deaths. |
| `FireUp=164,6,6` | 6-frame standing fire × 6 facings. The projectile spawns on frame `FireUp=6` from `[PTROOP]` art block. |
| `FireProne=212,6,6` | Prone-fire variant (used when crawled/pinned). |
| `Down=260 / Up=276` | Lay-down / get-up transitions for crawl. |
| `Paradrop=0,1,0` | Paratrooper-frame stub (no unique paradrop pose — falls back to `Ready`). |
| `Cheer=307,8,0,E` | Victory cheer (8 frames, east-facing only). |
| `Deploy/Deployed/Undeploy=292/299/301` | Unused at runtime because the `Deployer=yes` line in `rulesmd.ini` is commented out — these frames are dead data in the shipped game. |
| `Panic=8,6,6` | Reuses the walk frames as a panic-flee animation. |

---

## 3. Weapon — `[MindControl]` / `[MindControlE]`

```ini
[MindControl]
Damage=1;Number of mind control links
ROF=200
Range=7
Projectile=PsychicControl
Speed=100
Warhead=Controller
;Report=YuriMindControl
Anim=YURICNTL
FireOnce=yes
```

```ini
[MindControlE]
Damage=10;Needed to be considered offensive unit
ROF=200
Range=14
Projectile=PsychicControl
Speed=100
Warhead=Controller
;Report=YuriMindControl
Anim=YURICNTL
FireOnce=yes
```

| Key | Veteran/Rookie | Elite | Effect |
|-----|----------------|-------|--------|
| `Damage` | `1` | `10` | For MindControl warhead, damage is interpreted as link-count slot, not HP loss (Controller warhead has `MindControl=yes`, see §4). Elite `Damage=10` is a comment-flagged threat-eval tweak so the AI counts it as "offensive". |
| `ROF` | `200` | `200` | 200-tick (≈12 s) cooldown between mind-control fires. |
| `Range` | `7` | `14` | Elite **doubles** range from 7 → 14 cells. This is PTROOP's most significant elite upgrade. |
| `Projectile` | `PsychicControl` | same | See §3.1 — invisible inviso projectile. |
| `Speed` | `100` | same | Bullet speed (inviso = irrelevant visually but used for hit-time calc). |
| `Warhead` | `Controller` | same | See §4. |
| `;Report` | *(commented)* | same | YuriMindControl sound is intentionally suppressed for PTROOP (matches YURI's behavior; only the warhead's AnimList=YURICNTL provides feedback). |
| `Anim` | `YURICNTL` | same | Beam-trail/impact animation: see §3.2. |
| `FireOnce` | `yes` | same | Weapon fires a single shot then resolves — combined with mind-control's persistent link, this means the trooper does not auto-rebind. Verified WeaponType scope (`FireOnce` is in the WeaponType ReadINI list 0x00772xxx). |

### 3.1 `[PsychicControl]` projectile

```ini
[PsychicControl]
;Image=YURBLANK ; an invisible missile with a trailer
;ROT=100
Inviso=yes
Image=none
;Shadow=no
;Proximity=yes
;Ranged=yes
```

- `Inviso=yes` — no projectile actor; hit is resolved instantly along the firing line.
- `Image=none` — no sprite.
- Other lines commented: the author once tried a visible psychic-missile (`YURBLANK`) but reverted.

### 3.2 `[YURICNTL]` animation (anim played on target)

```ini
[YURICNTL]
Rate=450
```

- Rate=450 ticks-per-frame (very slow). All other anim defaults apply (no loop, plays once).
- This is the visible "psi rings" effect on the target during mind-control acquisition.

---

## 4. Warhead — `[Controller]`

```ini
[Controller];Mind control warhead.  Will skip normal damage like EMP did
Verses=100%,100%,100%,100%,100%,100%,0%,0%,0%,100%,100%
MindControl=yes
AnimList=YURICNTL
```

| Key | Effect |
|-----|--------|
| `Verses` | Damage multiplier per armor type. Slot order: `none, flak, plate, light, medium, heavy, wood, steel, concrete, special_1, special_2`. **Slots 7–9 (wood/steel/concrete) = 0%** → mind-control fails against buildings. |
| `MindControl` | WarheadType flag (verified 0x0075d7cf) — triggers `MindControlClass` linkage on target. The acquiring unit's `Type` (this `Controller` warhead) and the firing unit pointer become a mind-control entry in `MindControlClass`. |
| `AnimList` | `YURICNTL` plays on the target on hit (one entry → always picked). |

### Mind-control linkage behavior (deep-dive cross-refs)

See for full Ghidra reverse-engineering of the system:
- [MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md) (615 lines)
- [MIND_CONTROL_GHIDRA_REPORT.md](../../MIND_CONTROL_GHIDRA_REPORT.md) (451 lines)

Behavior summary as it applies to PTROOP specifically (not redocumented here, but consequences):
- PTROOP holds exactly **one** mind-control link (single slot — comes from the InfantryType base + no `Damage>1` override and no `InfiniteMindControl=yes` flag like Master Mind has).
- Breaking the link: PTROOP dies → target freed instantly. PTROOP's controlled unit dies → link clears. PTROOP forced to drop link (e.g., re-control by another mind-controller) → original link clears.
- Range check on **maintenance**: the mind-control link is maintained at firing range (7 cells / elite 14). Drift outside that range does NOT automatically break the link in YR (verified in MIND_CONTROL_SYSTEM doc).
- Building targets: `Verses` slots 7–9 = 0% → `MindControlClass::Add()` exits early; no link. PTROOP's MindControl primary is effectively useless vs buildings.

---

## 5. Voices / sounds

PTROOP shares the YURI voice set (no dedicated PTROOP voices in `soundmd.ini`).

```ini
[YuriAttackCommand]
Sounds= $iyurata $iyuratb $iyuratc $iyuratd $iyurate
Control= random interrupt

[YuriMove]
Sounds= $iyurmoa $iyurmob $iyurmoc $iyurmod
Control= random interrupt

[YuriSelect]
Sounds= $iyursea $iyurseb $iyursec $iyursed $iyursee
Control= random interrupt

[YuriDie]
Sounds= $iyurdia $iyurdib $iyurdic
Control= random interrupt
```

```ini
[InfantrySquish]
Sounds=igensqua
FShift= -10 10
Volume=65
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=YuriSelect` | 5 clips, random | Click-select |
| `VoiceMove=YuriMove` | 4 clips, random | Move order |
| `VoiceAttack=YuriAttackCommand` | 5 clips, random | Attack order |
| `VoiceSpecialAttack=YuriMove` | reuses move set | Special/secondary attack order (e.g., force-fire) |
| `VoiceFeedback=` *(empty)* | — | No "under attack" line |
| `DieSound=YuriDie` | 3 clips, random | Death |
| `CrushSound=InfantrySquish` | `igensqua` | When crushed (n/a since `Crushable=no`, but still set for completeness) |

No weapon-specific `Report=` (line is commented), and `MindControl` warhead carries no sound — so PTROOP mind-control is silent except for the target's `[YuriMindControlSound]` (global, `[AudioVisual]` — see Yuri docs).

---

## 6. Prerequisites / owners / tech

### Build-tree gate
- **`Prerequisite=BARRACKS`** — generic barracks token. Resolves per house to:
  - Allied houses → `GAPILE`
  - Soviet houses → `NAHAND`
  - Yuri house → `YABRCK`
- **`RequiresStolenThirdTech=yes`** — additional house-flag gate set by Spy infiltration of any Yuri tech building. The "Third" naming in the binary refers to Yuri as the third side. Verified at TechnoTypeClass__ReadINI 0x007144db reading string at 0x00843bfc.
- **`AllowedToStartInMultiplayer=no`** — combined with the above, PTROOP never appears in the starting sidebar; only after a successful infiltration.
- **`TechLevel=9`** — last build-level; with no `[General] TechLevel=` cap below 9 in standard play, this just delays availability until tech-trees are fully unlocked. Combined with the stolen-tech gate it is the effective endgame slot.

### Owner / RequiredHouses
- **`Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance`** — all 10 multiplayer countries plus `Alliance` (campaign neutral). No `RequiredHouses=` line ⇒ no country-lock; all countries can build PTROOP given the tech-steal unlock.

### Comparison to the tech-steal triplet

| Unit | Tech gate flag | Theme | Owner= |
|------|----------------|-------|--------|
| CCOMAND | `RequiresStolenAlliedTech=yes` | Allied (Chrono) | All 10 + Alliance |
| TNKD    | `RequiresStolenSovietTech=yes` | Allied vehicle (anti-armor) | (see TNKD doc — typically Allied-only) |
| **PTROOP** | **`RequiresStolenThirdTech=yes`** | **Yuri (psi)** | **All 10 + Alliance** |

Distinct from CCOMAND, PTROOP's tech-steal triplet companion: CCOMAND is built when a non-Allied house steals Allied tech (lab); PTROOP is built when a non-Yuri house steals Yuri tech (lab). Yuri-faction players themselves can also build PTROOP if they infiltrate a (different) Yuri Battle Lab — though in practice Yuri already has access to native YURI/YURIPR, so the stolen-tech unit is functionally redundant for them.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 PTROOP-specific code in `gamemd.exe`: **none**

Ghidra searches (search_strings):
| Query | Result |
|-------|--------|
| `PTROOP` | 0 matches |
| `PCOMMANDO` | 0 matches |
| `PsiCorps` | 0 matches |

⇒ **There is no PTROOP-specific function, branch, or hardcoded ID anywhere in gamemd.exe.** Every behavior listed below is generic to the *flags* PTROOP carries.

### 7.2 Flag-scope verification (Ghidra)

| Key | String at | Reading function | Class scope |
|-----|-----------|------------------|-------------|
| `RequiresStolenThirdTech` | 0x00843bfc | TechnoTypeClass__ReadINI @ 0x007144db | TechnoType |
| `RequiresStolenAlliedTech` (sibling) | 0x00843bb4 | TechnoTypeClass__ReadINI @ 0x00714529 | TechnoType |
| `ImmuneToPsionics` | 0x00843754 | TechnoTypeClass__ReadINI @ 0x00714fa7 | TechnoType |
| `ImmuneToPsionicWeapons` (sibling) | 0x0084373c | TechnoTypeClass__ReadINI @ 0x00714fc8 | TechnoType |
| `DetectDisguise` | 0x00843c78 | TechnoTypeClass__ReadINI @ 0x0071443f | TechnoType |
| `IFVMode` | 0x00843ae4 | TechnoTypeClass__ReadINI @ 0x00714787 | TechnoType |
| `PistolTurretWeapon` (IFV side) | 0x00845bc8 | UnitTypeClass__ReadINI @ 0x00747c8c | UnitType (the IFV vehicle, not PTROOP) |
| `MindControl` | 0x0081bbc8 | WarheadTypeClass__ReadINI @ 0x0075d7cf | WarheadType |

This confirms:
- All PTROOP-side gating flags live on `TechnoTypeClass` (so the same behavior applies if applied to any infantry/vehicle/aircraft/building, modulo runtime relevance).
- `MindControl=yes` is a **warhead** flag — PTROOP triggers mind-control by firing the `Controller` warhead, not via a unit-class flag.
- The IFV's per-passenger weapon selection reads from the IFV vehicle's `UnitType` keys; the passenger's `IFVMode` is just an index into that table.

### 7.3 Live behaviors driven by these flags (gamemd-internal)

| Behavior | Driver | Notes |
|----------|--------|-------|
| Build-tree gate (must infiltrate Yuri lab) | `RequiresStolenThirdTech` checked in HouseClass build-availability path | Same path as CCOMAND's stolen-Allied check, just different flag bit. |
| Mind-control link on hit | `[Controller].MindControl=yes` triggers `MindControlClass::Add()` for the firing unit | One link slot per attacker (no `Damage>1` override). |
| 0% damage vs buildings | `[Controller].Verses` slots 7–9 = 0% | Mind-control attempt vs building silently fails. |
| Building-attack via planted charge (C4 mission) | `C4=yes` allows `MISSION_SABOTAGE` against `BuildingClass` targets; charge detonates with `[CombatDamage] C4Warhead=Super` | Shared with SEAL/TANY/CCOMAND/IVAN. PTROOP technically has C4 but, since `Primary=MindControl` is its targeting weapon, the C4 mission is only entered when the player gives an attack order on a building (which the mind-control primary cannot resolve). In that case the unit walks to the building and plants the C4 charge — same routine as SEAL. **Observable consequence**: a player can order PTROOP to demolish buildings with a 1-shot C4 (via `Super` warhead). This is rarely seen because most players assume PTROOP only mind-controls. |
| Cannot be mind-controlled by enemy psi | `ImmuneToPsionics=yes` short-circuits `MindControlClass::Add()` on the receiving side | Distinct from `ImmuneToPsionicWeapons`: PTROOP can still be hit by Psychic Dominator AoE damage and `[ChaosGas]`-style psychic warheads, since those check the broader `ImmuneToPsionicWeapons` flag (which PTROOP does **not** set). |
| Cannot be crushed | `Crushable=no` overrides `Category=Soldier` default | |
| Sees through disguises | `DetectDisguise=yes` + default `DetectDisguiseRange=1` | Mirage Tanks, Spies, CCOMAND lose their disguise within 1 cell of PTROOP. |
| IFV pistol-mode passenger | `IFVMode=4` indexes into Allied IFV's `PistolTurretWeapon=4` → `Weapon5=CRMP5` | Notable design quirk: a psi-trooper in an IFV fires an MP5, not a psi weapon. The psi-IFV slot is `IFVMode=8` (YURI). |
| Standard infantry locomotion | `Locomotor={4A582744-...}` = `WalkLocomotionClass` | No special movement. |

### 7.4 Behaviors NOT present in PTROOP (despite design hints)

- **Deploy fire** — `;Deployer=yes` etc. are commented; the unit has no `Secondary` and no deploy-time weapon. Even if the source author uncomments them, there is no second weapon for the deploy mode to fire.
- **Amphibious / water walk** — `;SpeedType=Amphibious` commented out; PTROOP cannot enter water.
- **Self-heal at non-elite ranks** — only `EliteAbilities` includes `SELF_HEAL`. Veteran rank does not regen HP.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `TiberiumProof=yes` | YES (tiberium gone) | Dormant — no live consumer triggers. |
| `ImmuneToVeins=yes` | YES (veinholes gone) | Dormant. |
| `LeadershipRating=8` | Partial | Read into TechnoType; YR only uses it for marginal AI scoring. Functionally cosmetic. |
| `;Deployer / ;DeployFire / ;UndeployDelay` | n/a (commented) | Inactive. Dead-design churn. |
| `;SpeedType=Amphibious / ;MovementZone=AmphibiousDestroyer` | n/a (commented) | Inactive. |

All other flags are live in YR.

---

## 9. Veterancy

### Veteran (1 chevron) abilities — `STRONGER, FIREPOWER, ROF, SIGHT, FASTER`

Generic TechnoType veteran modifiers from `[General]` defaults (verified scopes from prior docs; values may be globals in `rulesmd.ini`):
- `STRONGER` — +25% HP (typical)
- `FIREPOWER` — +25% damage
- `ROF` — −25% ROF (faster cooldown)
- `SIGHT` — +20% sight range (Sight 8 → 9.6)
- `FASTER` — +20% speed (Speed 5 → 6)

Net for PTROOP at veteran rank:
- Still uses **`MindControl`** primary (not `MindControlE`) — `ElitePrimary` only swaps at elite.
- Stat bumps but same range (7) and same Damage=1 (link slot).

### Elite (2 chevrons) abilities — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative with veteran)

Elite adds `SELF_HEAL` (passive HP regen, ticks per `[General]` `SelfHealInfantryRate`). Note elite does NOT re-grant `FASTER` or `SIGHT` like some units — PTROOP's elite swap is more about staying alive than getting faster.

**Plus weapon swap**: `Primary` → `ElitePrimary=MindControlE`:
- Range 7 → **14** (doubled — biggest practical upgrade)
- Damage 1 → 10 (threat-eval flag; still 1 link)
- ROF, Projectile, Warhead, Anim, FireOnce: unchanged

---

## 10. Cross-references

### Direct dependencies (must exist in `rulesmd.ini`)
- `[MindControl]` — weapon (§3)
- `[MindControlE]` — elite weapon (§3)
- `[PsychicControl]` — projectile (§3.1)
- `[Controller]` — warhead (§4)
- `[YURICNTL]` (artmd) — mind-control hit animation (§3.2)
- `[PsiTroopSequence]` (artmd) — frame table (§2)
- `[YuriSelect] / [YuriMove] / [YuriAttackCommand] / [YuriDie] / [InfantrySquish]` (soundmd) — voices (§5)
- `[General] C4Warhead=Super` (rulesmd line 818) — used when PTROOP plants a C4 on a building (§7.3)
- `BARRACKS` token resolution (`[AI] BuildBarracks=` or per-faction barracks list) (§6)

### Conceptual companions
- **YURI** (`yuri/YURI.md`) — same voice set, same primary type but warhead = `Controller` (single link) for both; YURI also has `Deployer=yes` + Secondary `PsychicWave` for AoE, PTROOP does not.
- **YURIPR** (`yuri/YURIPR.md`) — uses `SuperMindBlast` (AoE psi blast) instead; both units share `ImmuneToPsionics`.
- **CCOMAND** (`allied/CCOMAND.md`) — Allied tech-steal sibling; both have `C4=yes`, `Crushable=no`, `IFVMode=4`, same TechLevel/Cost-class.
- **TNKD** (`allied/TNKD.md` — TODO) — Allied tech-steal vehicle; rounds out the triplet.

### Deep-RE docs (cross-reference, do not re-derive)
- [MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md)
- [MIND_CONTROL_GHIDRA_REPORT.md](../../MIND_CONTROL_GHIDRA_REPORT.md)
- [SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md](../../SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md) — for the `RequiresStolenThirdTech` flip on Spy entering a Yuri lab.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[PTROOP]` rulesmd key explained | ✅ §1 |
| Every `[PTROOP]` artmd key explained | ✅ §2 |
| `Sequence=PsiTroopSequence` fully expanded | ✅ §2 |
| Primary weapon + elite variant + projectile + warhead + impact anim | ✅ §3–§4 |
| All voices + crush sound expanded with verbatim sound defs | ✅ §5 |
| Prereqs / owners / tech gate analysed | ✅ §6 |
| Hardcoded behavior — at least one Ghidra search per scope | ✅ §7 (eight flag-scope searches; PTROOP-string searches returned 0 confirming no unit-specific code) |
| Veterancy detailed | ✅ §9 |
| TS-legacy filter applied | ✅ §8 |
| Cross-refs to weapon/warhead/anim/voice sections | ✅ §10 |
| Deep-RE reports linked, not duplicated | ✅ §4, §10 |

**Open follow-ups (none load-bearing):**
- Confirm `Volume`, `Priority` defaults for the 3 YURI* sound entries against `[SoundList]` overrides if any.
- Verify the exact tick value used by `[General] SelfHealInfantryRate=` to scope PTROOP's elite `SELF_HEAL` rate.
- The IFV `IFVMode→TurretIndex+TurretWeapon` dispatcher itself has not been Ghidra-decompiled in this doc; current claim "IFVMode=4 → PistolTurretWeapon=4 → Weapon5=CRMP5" is derived from the IFV's own INI table and the standard YR mapping, not from binary trace. Would benefit from a dedicated `/re-investigate IFV gunner dispatch` if a parity bug ever surfaces with PTROOP-in-IFV firing.
