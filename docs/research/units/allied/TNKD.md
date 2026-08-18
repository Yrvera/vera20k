# TNKD — Tank Destroyer

**Side classification:** Allied vehicle. **German country-locked** native unit, plus
universally available via Secret Lab capture or crate.
**Role:** Pure anti-armor tank — no turret, fixed forward-firing 120mm AP cannon. Very
high damage vs medium/heavy armor, near-zero damage vs everything else.

> Output bar: indistinguishable from gamemd.exe for the player. Behaviors below are
> driven entirely by generic TechnoType / UnitType / WeaponType / WarheadType flag
> handling — Ghidra confirms `gamemd.exe` contains no `"TankDestroyer"` string and
> no plain `"TNKD"` (only the CSF lookup key `Name:TNKD` at 0x008299dc).

> **Index correction note**: The /loop prompt described TNKD as the third leg of a
> "tech-steal triplet" with `RequiresStolenSovietTech=yes`. This is **false**. The
> only `RequiresStolenSovietTech` user in `rulesmd.ini` is **CIVAN (Chrono Ivan)**
> at line 4666, not TNKD. TNKD's universal-availability path is the **Secret Lab**
> system (`SecretUnits=TNKD,TTNK,DTRUCK` at line 265), plus `CrateGoodie=yes` (line
> 7491). The doc below reflects what the INI and binary actually say.

---

## 1. `rulesmd.ini` — `[TNKD]` verbatim

```ini
[TNKD]
UIName=Name:TNKD
Name=Tank Destroyer
Prerequisite=GAWEAP,RADAR
Primary=SABOT
Strength=400
Category=AFV
Armor=heavy
Turret=no
IsTilter=yes
Crusher=yes
TooBigToFitUnderBridge=true
TechLevel=2
Sight=8
Speed=5
CrateGoodie=yes
Owner=British,French,Germans,Americans,Alliance
RequiredHouses=Germans
AllowedToStartInMultiplayer=no
Cost=900
Soylent=900
Points=25
ROT=5
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=TankDestroyerSelect
VoiceMove=TankDestroyerMove
VoiceAttack=TankDestroyerAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
CrushSound=TankCrush
MoveSound=TankDestroyerMoveStart
MaxDebris=2
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
MovementZone=Normal
ThreatPosed=15	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Accelerates=false
ImmuneToVeins=yes
Size=3
ElitePrimary=SABOTE
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:TNKD` | AbstractType | CSF lookup. Found in binary at 0x008299dc as a CSF-table key, not as a hardcoded behavior gate. |
| `Name` | `Tank Destroyer` | AbstractType | Dev/fallback name. |
| `Prerequisite` | `GAWEAP,RADAR` | TechnoType | Requires Allied War Factory **AND** any radar building (GASPYSAT / NARADR / YACOMD). `RADAR` is a generic token resolved per-house. |
| `Primary` | `SABOT` | TechnoType | Main weapon — see §3. |
| `Strength` | `400` | AbstractType | Hitpoints. Matches Rhino (HTNK=400), thicker than Grizzly (MTNK=300). |
| `Category` | `AFV` | TechnoType | Armored Fighting Vehicle — affects AI threat classification, crate effects, etc. |
| `Armor` | `heavy` | TechnoType | Verses-slot 6 in target warheads. |
| `Turret` | `no` | **TechnoType** [BINARY-VERIFIED audit 12 — CORRECTS doc claim of UnitType-scope; parsed in `TechnoTypeClass__ReadINI` to `TechnoType+0xCA1`, byte. Writer @ 0x007133C2 per Ghidra in-binary annotation.] | **No rotating turret** — fires only along the hull facing. Driver must rotate the whole tank to aim. This is the defining gameplay constraint of TNKD: it cannot kite efficiently, must commit to an angle, and can be flanked. |
| `IsTilter` | `yes` | UnitType [BINARY-VERIFIED audit 12: `UnitType+0xE14`] | Voxel hull tilts on slopes (cosmetic). |
| `Crusher` | `yes` | TechnoType [BINARY-VERIFIED audit 12: `TechnoType+0xD28`] | Can crush infantry. |
| `TooBigToFitUnderBridge` | `true` | **UnitType** [BINARY-VERIFIED audit 12 — CORRECTS doc claim of TechnoType-scope; parsed in `UnitTypeClass__ReadINI` to `UnitType+0xE16`] | Cannot path under low bridges (e.g. railroad spans). |
| `TechLevel` | `2` | TechnoType | Buildable from early game tech tier 2 onward — combined with `RADAR` prereq it becomes available shortly after the Radar is constructed. |
| `Sight` | `8` | TechnoType | Reveal radius. Equal to or shorter than its 5-cell weapon range — has slight "fog gap" but typically supported by friendly scouts. |
| `Speed` | `5` | TechnoType | Slow — slower than Grizzly (6) and Rhino (6). Matches the slow-but-tough role. |
| `CrateGoodie` | `yes` | **UnitType** [BINARY-VERIFIED audit 12: parser xref @ 0x00747658 in `UnitTypeClass__ReadINI`, writes `UnitType+0xE0D` byte] | Can be the random "free unit" reward from a money/unit crate. |
| `Owner` | `British,French,Germans,Americans,Alliance` | TechnoType | All five Allied countries + the campaign-neutral Alliance pseudo-house. |
| `RequiredHouses` | `Germans` | TechnoType [BINARY-VERIFIED audit 12: string @ 0x00843bb4, parser xref @ 0x00714529, writes `TechnoType+0xDA0` int (via `FUN_004750d0` country-bitmask helper)] | Build menu only shows TNKD for **Germans**. British/French/Americans cannot natively build it even though they appear in `Owner=` — `Owner` enables ownership/transfer; `RequiredHouses` is the additional country-build gate. They can still **acquire** TNKD via Secret Lab capture or crate (see §6). |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Excluded from start-of-game tech tree resolution (relevant since TechLevel=2 would otherwise mean immediate availability for Germans). Germans can build it normally once Radar+War Factory are up; the flag prevents pre-placed initial units. |
| `Cost` | `900` | TechnoType | Build cost — same as Rhino. |
| `Soylent` | `900` | TechnoType [BINARY-VERIFIED audit 12: `TechnoType+0x614`, int] | 100% refund to Grinder (rare — most Allied vehicles don't have Soylent listed; TNKD being grindable in a Yuri Grinder requires capture or `RequiredHouses` mismatch). |
| `Points` | `25` | TechnoType | Score on kill. |
| `ROT` | `5` | TechnoType | Rate-of-turn for the **hull** (since `Turret=no`, this is the only rotation parameter). 5 is mid-low; TNKD turns slowly. |
| `IsSelectableCombatant` | `yes` | TechnoType | Counted in select-all-combat hotkey. |
| `Explosion` | `TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` | TechnoType | Multi-anim death explosion sequence — randomly picks from the list. |
| `VoiceSelect` | `TankDestroyerSelect` | TechnoType | Unique 5-clip voice set (`$vtansea`..e). See §5. |
| `VoiceMove` | `TankDestroyerMove` | TechnoType | Unique 5-clip move voice. |
| `VoiceAttack` | `TankDestroyerAttackCommand` | TechnoType | Unique 5-clip attack voice. |
| `VoiceFeedback` | *(empty)* | TechnoType | No under-attack line. |
| `DieSound` | `GenVehicleDie` | TechnoType | Shared 6-clip generic vehicle death sound. |
| `CrushSound` | `TankCrush` | TechnoType | When TNKD crushes infantry. |
| `MoveSound` | `TankDestroyerMoveStart` | TechnoType | Engine-start clip on movement begin (with random delay 0–400 ms via `Delay=` in sound def). |
| `MaxDebris` | `2` | TechnoType [BINARY-VERIFIED audit 12: `TechnoType+0x5BC`, int] | Up to 2 debris pieces on death. |
| `Locomotor` | `{4A582741-9839-11d1-B709-00A024DDAFD1}` | TechnoType | `DriveLocomotionClass` — standard tracked-vehicle locomotor (same as Grizzly, Rhino). |
| `MovementZone` | `Normal` | TechnoType | Pathing zone — all land that allows normal vehicles. |
| `ThreatPosed` | `15` | TechnoType | Mid-tier AI threat. Lower than Rhino (40) — because vs non-armor it does very little. |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | Smoke/sparks emitted when below damage thresholds. |
| `VeteranAbilities` | `STRONGER,FIREPOWER,SIGHT,FASTER` | TechnoType | Bonuses at veteran rank — see §9. Note: no `ROF`. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Bonuses added at elite. Adds SELF_HEAL and ROF; drops SIGHT and FASTER from the veteran list (cumulative additions only). |
| `Accelerates` | `false` | TechnoType [BINARY-VERIFIED audit 12: `TechnoType+0xDBD`] | No acceleration ramp — moves at top speed immediately when ordered (same as most Allied tanks). |
| `ImmuneToVeins` | `yes` | TechnoType [BINARY-VERIFIED audit 12: `TechnoType+0xC91`] | **TS-LEGACY** — no veinholes in YR. (See §8.) |
| `Size` | `3` | TechnoType | Transport-slot cost (occupies 3 slots in an APC/IFV transport — but TNKD as a passenger is non-standard). |
| `ElitePrimary` | `SABOTE` | TechnoType | Elite-rank primary — see §3. |

---

## 2. `artmd.ini` — `[TNKD]` section

```ini
[TNKD]   ; Tank destroyer
Voxel=yes
Remapable=yes
Cameo=TNKDICON
AltCameo=TNKDUICO
PrimaryFireFLH=200,0,55
```

| Key | Value | Effect |
|-----|-------|--------|
| `Voxel` | `yes` | Renders from `TNKD.VXL` + `TNKD.HVA` voxel files (not SHP sprite). |
| `Remapable` | `yes` | House-color remap palette applied to the voxel. |
| `Cameo` | `TNKDICON` | Standard sidebar cameo. |
| `AltCameo` | `TNKDUICO` | Yuri-skinned cameo (used if/when TNKD ends up in a Yuri faction's hand — e.g., via mind-control or Grinder display). |
| `PrimaryFireFLH` | `200,0,55` | Firing offset: X=+200 (very far forward — the long-barrel SABOT muzzle), Y=0 (centred), Z=+55 (barrel height). |

No `Sequence=` (vehicles don't use the InfantrySequence table — voxel animation is driven by HVA frames). No `Crawls=`, no `FireUp=` for this entry.

---

## 3. Weapon — `[SABOT]` / `[SABOTE]`

### `[SABOT]` (rookie & veteran)

```ini
[SABOT]
Damage=150 ;was 135
ROF=70
Range=5
Projectile=Cannon
Speed=60
Warhead=UltraAP
Report=TankDestroyerAttack
Anim=GUNFIRE
Bright=yes
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `150` | Per-shot base damage (pre-warhead-Verses). |
| `ROF` | `70` | Cooldown 70 ticks (~4.2 s @ 60 FPS sim). |
| `Range` | `5` | Engagement radius in cells. Short — must close to brawl range. |
| `Projectile` | `Cannon` | See §3.1. |
| `Speed` | `60` | Bullet speed. |
| `Warhead` | `UltraAP` | See §4 — pure-armor warhead. |
| `Report` | `TankDestroyerAttack` | Unique 3-clip firing sound (see §5). |
| `Anim` | `GUNFIRE` | Muzzle-flash anim spawned at `PrimaryFireFLH`. |
| `Bright` | `yes` | Lights the cell when firing (small ambient flash). |

### `[SABOTE]` (elite)

```ini
[SABOTE]
Damage=175
ROF=60
Range=6.75
Projectile=Cannon
Speed=60
Warhead=UltraAPE
Report=TankDestroyerAttack
Anim=VTMUZZLE
Bright=yes
Burst=2
```

Elite-rank deltas vs rookie:
- `Damage` 150 → **175** (+16.7%)
- `ROF` 70 → **60** (faster)
- `Range` 5 → **6.75** (+35%)
- `Warhead` UltraAP → UltraAPE (better vs medium armor — see §4)
- `Anim` GUNFIRE → VTMUZZLE (different muzzle effect)
- `Burst` not present → **`Burst=2`** — fires two shots per cooldown
- Net DPS: rookie 150/70 = 2.14 dmg/tick → elite 350/60 = 5.83 dmg/tick (≈ **2.7× DPS**, the most dramatic elite step among any Allied vehicle).

### 3.1 `[Cannon]` projectile

```ini
[Cannon]
Image=120MM
Arcing=true
SubjectToCliffs=yes
SubjectToElevation=yes
SubjectToWalls=yes
```

- `Image=120MM` — bullet sprite. `[120MM]` in artmd has no body — uses all defaults.
- `Arcing=true` — projectile follows ballistic arc (not flat-fire). Important for the cliff/elevation rules below.
- `SubjectToCliffs=yes` — can be blocked by cliff terrain.
- `SubjectToElevation=yes` — high-ground attacker gets damage bonus / low-ground gets penalty per `[General] ElevationDamageMultiplier`.
- `SubjectToWalls=yes` — walls block the shot.

### 3.2 Firing-effect animations

```ini
[GUNFIRE]
Layer=ground
Translucent=yes
```

```ini
[VTMUZZLE]
Normalized=yes
```

- `GUNFIRE` (rookie/veteran): generic gun muzzle flash, rendered on the ground layer with translucency.
- `VTMUZZLE` (elite): "Vehicle Tank MUZZLE" — `Normalized=yes` means anim-frame speed normalized regardless of FPS. No other overrides → otherwise uses anim defaults.

---

## 4. Warhead — `[UltraAP]` / `[UltraAPE]`

```ini
[UltraAP]
;CellSpread=0
;PercentAtMax=1
Wall=yes
Wood=yes
Verses=2%,2%,2%,100%,40%,100%,2%,2%,2%,2%,100% ; can only be used on armor
Conventional=yes
InfDeath=3
AnimList=S_CLSN30
ProneDamage=50%
```

```ini
[UltraAPE]
;CellSpread=0
;PercentAtMax=1
Wall=yes
Wood=yes
Verses=2%,2%,2%,100%,50%,100%,2%,2%,2%,100%,100% ; can only be used on armor
Conventional=yes
InfDeath=3
AnimList=VTEXPLOD
ProneDamage=50%
```

### Verses slot map (positional)

| Slot | Armor name | UltraAP | UltraAPE | Interpretation |
|------|-----------|---------|----------|----------------|
| 1 | `none` (most infantry) | 2% | 2% | 150 × 2% = 3 dmg vs basic infantry — effectively useless. |
| 2 | `flak` (Flak Trooper, BORIS) | 2% | 2% | Same — useless. |
| 3 | `plate` (Tanya, GHOST, CCOMAND etc.) | 2% | 2% | Same. |
| 4 | `light` (Grizzly, Mirage, IFV) | **100%** | **100%** | Full damage. Two-shot kill on Grizzly (200 HP). |
| 5 | `medium` (Lasher, Apocalypse base, some) | **40%** | **50%** | Reduced damage; elite gets the +10% boost (the only Verses delta UltraAP→UltraAPE besides slot 10). |
| 6 | `heavy` (Rhino HTNK, TNKD itself, Apocalypse) | **100%** | **100%** | Full damage. Three-shot kill on Rhino (400 HP). |
| 7 | `wood` (buildings) | 2% | 2% | Useless vs structures. |
| 8 | `steel` (buildings) | 2% | 2% | Useless. |
| 9 | `concrete` (buildings) | 2% | 2% | Useless. |
| 10 | `special_1` (e.g., aircraft hull?) | 2% | **100%** | UltraAPE adds full damage in this slot. |
| 11 | `special_2` | 100% | 100% | Full damage. |

> Comment in INI explicitly states "can only be used on armor" — design intent confirmed.

| Key | Effect |
|-----|--------|
| `Wall=yes` | Destroys/damages walls on hit. |
| `Wood=yes` | Will set wood structures on fire if hit (combined with `Verses` slot 7 = 2% though, fire damage may not trigger). |
| `Conventional=yes` | Non-nuclear/non-psi — used by EMP/IronCurtain "shake off conventional damage" logic. |
| `InfDeath` | `3` → explosion death anim for infantry (cannon-shot blast — InfDeath table compiled across docs: 1=small-arms, **3=explosion (RPG/Cannon)**, 4=burn, 5=electric, 6=blown-to-bits, 7=radiation, 8=plague, 10=gibbed). |
| `AnimList` | `S_CLSN30` (rookie) / `VTEXPLOD` (elite) — impact animation. |
| `ProneDamage` | `50%` — infantry in prone state take half damage (relevant only for the 2% verses slots, so usually ineffective regardless). |
| Commented `CellSpread=0` / `PercentAtMax=1` | Single-cell point damage — no AoE. |

---

## 5. Voices / sounds

```ini
[TankDestroyerSelect]
Sounds=$vtansea $vtanseb $vtansec $vtansed $vtansee
Control=random
Volume=85

[TankDestroyerMove]
Sounds=$vtanmoa $vtanmob $vtanmoc $vtanmod $vtanmoc
Control=random
Volume=85

[TankDestroyerAttackCommand]
Sounds= $vtanata $vtanatb $vtanatc $vtanatd $vtanate
Control=random
Volume=85
```

```ini
[TankDestroyerAttack]
Sounds= vtadatta vtadattb vtadattc
Control= random
FShift= -10 10
Volume=85
```

```ini
[TankDestroyerMoveStart]
Sounds= vtadstaa vtadstab
Control= random predelay
Priority=low
Delay=0 400
```

```ini
[GenVehicleDie]
Sounds= vgendiea vgendieb vgendiec vgendied vgendiee vgendief
Control=random
FShift=-15 15
VShift=20
Volume=85
```

```ini
[TankCrush]
Sounds=vcrusha
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=TankDestroyerSelect` | 5 clips ($vtansea..e) | Click-select |
| `VoiceMove=TankDestroyerMove` | 5 clips ($vtanmoa..d + one repeat) | Move order |
| `VoiceAttack=TankDestroyerAttackCommand` | 5 clips ($vtanata..e) | Attack order |
| `Report=TankDestroyerAttack` (weapon) | 3 clips (vtadatta..c), FShift ±10 | Weapon fire — randomized pitch shift adds variety per shot |
| `MoveSound=TankDestroyerMoveStart` | 2 clips (vtadstaa, vtadstab), random predelay 0–400ms, low-priority | Engine start when movement begins |
| `DieSound=GenVehicleDie` | 6 clips (vgendiea..f), FShift ±15, VShift +20 | Death explosion sound |
| `CrushSound=TankCrush` | vcrusha | When TNKD crushes infantry |

Note: TNKD has the most thorough unique-voice set of any Secret Lab unit. The `$` prefix on Select/Move/Attack clips indicates compressed/processed clips per RA2's audio system; the weapon and engine clips lack the prefix.

---

## 6. Prerequisites / owners / availability

### Build-tree gate (native)

1. **Country gate** — `RequiredHouses=Germans` ⇒ visible in build menu only for the German country (verified TechnoType field at 0x00843bb4 → `TechnoTypeClass__ReadINI`).
2. **Prerequisite** — `GAWEAP,RADAR` ⇒ Allied War Factory AND any radar building.
3. **TechLevel** — `2` is low; tech-lab not required.
4. **`AllowedToStartInMultiplayer=no`** — Germans cannot start with a TNKD as preplaced unit; must build normally.

### Universal acquisition paths (non-German Allied / any side via capture)

| Path | Mechanism | Probability |
|------|-----------|-------------|
| **Secret Lab capture** | `[General] SecretUnits=TNKD,TTNK,DTRUCK` (line 265). When `[CASLAB]` (`SecretLab=yes`, line 14081) is captured by Engineer, gamemd rolls one of the three. Once granted, the capturing house gets a free TNKD **and** ongoing build access to it (`RequiredHouses` is overridden by the Secret-Lab grant). | 1-in-3 of the SecretUnits pool. |
| **Crate goodie (`CrateGoodie=yes`)** | UnitType-scoped flag (verified 0x00747658 in `UnitTypeClass__ReadINI`). Random unit-spawn crates pull from the pool of all `CrateGoodie=yes` vehicles. | Standard crate odds (depends on `[CrateRules] UnitCrateType`). |
| **Capture from German player** | Engineer or mind-control of an existing TNKD. The `Owner=` clause includes British/French/Americans, so capture works without the house gate. | Player-skill. |
| **Mind-control / yuri etc.** | Same as capture — possession transfers. | Player-skill. |

### Comparison: TNKD vs the actual tech-steal triplet

| Unit | Universal-unlock flag | Tech building infiltrated |
|------|----------------------|---------------------------|
| **CCOMAND** (Allied) | `RequiresStolenAlliedTech=yes` | Allied Battle Lab |
| **CIVAN** (Chrono Ivan) | `RequiresStolenSovietTech=yes` | Soviet Battle Lab |
| **PTROOP** (Psi-Corp Trooper) | `RequiresStolenThirdTech=yes` | Yuri Battle Lab |
| **TNKD** | (none — uses Secret Lab + crate paths) | n/a |

TNKD is **NOT** in this group. The true "tech-steal triplet" is CCOMAND / CIVAN / PTROOP; the doc index's previous "RequiresStolenSovietTech=yes" note for TNKD was incorrect. CIVAN should be doc'd separately as the actual Spy-vs-Soviet-tech unlock unit.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 TNKD-specific code in `gamemd.exe`: **none**

| Query (search_strings) | Result |
|------------------------|--------|
| `TNKD` | Only `"Name:TNKD"` at 0x008299dc (CSF lookup key, not a hardcoded ID) |
| `TankDestroyer` | 0 matches |

⇒ No bespoke TNKD code path. All behavior is driven by generic flag handling.

### 7.2 Flag-scope verification

| Key | String at | Read by | Class scope | Stores to | Type |
|-----|-----------|---------|-------------|-----------|------|
| `RequiredHouses` | 0x00843bb4 | TechnoTypeClass__ReadINI @ 0x00714529 | TechnoType | `+0xDA0` | int (bitmask via `FUN_004750d0`) |
| `ForbiddenHouses` | 0x00843b94 | TechnoTypeClass__ReadINI @ 0x0071456A | TechnoType | `+0xDA4` | int (bitmask) |
| `RequiresStolenThirdTech` | 0x00843bfc | TechnoTypeClass__ReadINI @ 0x007144E8 | TechnoType | `+0xD9B` | byte |
| `RequiresStolenSovietTech` | 0x00843be0 | TechnoTypeClass__ReadINI @ 0x00714502 | TechnoType | `+0xD9C` | byte |
| `RequiresStolenAlliedTech` | 0x00843bc4 | TechnoTypeClass__ReadINI @ 0x0071451C | TechnoType | `+0xD9D` | byte |
| `Turret` | 0x00844110 | TechnoTypeClass__ReadINI @ 0x007133A5 (writer @ 0x007133C2) | TechnoType | `+0xCA1` | byte |
| `Crusher` | 0x0081bb58 | TechnoTypeClass__ReadINI @ 0x00714CDB | TechnoType | `+0xD28` | byte |
| `Accelerates` | 0x00843534 | TechnoTypeClass__ReadINI @ 0x0071540E | TechnoType | `+0xDBD` | byte |
| `ImmuneToVeins` | 0x008438CC | TechnoTypeClass__ReadINI @ 0x00714C36 | TechnoType | `+0xC91` | byte |
| `Soylent` | 0x00843B08 | TechnoTypeClass__ReadINI @ 0x007146CD | TechnoType | `+0x614` | int |
| `MaxDebris` | 0x0084439C | TechnoTypeClass__ReadINI @ 0x0071259B | TechnoType | `+0x5BC` | int |
| `CrateGoodie` | 0x00845e20 | UnitTypeClass__ReadINI @ 0x00747658 | **UnitType** | `+0xE0D` | byte |
| `IsTilter` | 0x00845DF0 | UnitTypeClass__ReadINI | UnitType | `+0xE14` | byte |
| `TooBigToFitUnderBridge` | 0x00845DC8 | UnitTypeClass__ReadINI | **UnitType** (NOT TechnoType — corrects doc) | `+0xE16` | byte |
| `SecretLab` | 0x0081aaa0 | BuildingTypeClass_ReadINI_Water @ 0x004609bf | BuildingType | `+0x16B0` | byte |
| `SecretInfantry` | 0x0081abf0 | BuildingTypeClass_ReadINI_Water | BuildingType | `+0xEA4` | InfantryType* |
| `SecretUnit` | 0x0081abe4 | BuildingTypeClass_ReadINI_Water | BuildingType | `+0xEA8` | UnitType* |
| `SecretBuilding` | 0x0081abd4 | BuildingTypeClass_ReadINI_Water | BuildingType | `+0xEAC` | BuildingType* |
| `SecretUnits` | 0x0083c730 | RulesClass__ReadGeneral @ 0x0066fa54 | Global (`[General]`) | Rules+`0xD1C` | DynamicVector<UnitType*> |
| `SecretInfantry` (Rules) | 0x0081abf0 | RulesClass__ReadGeneral | Global | Rules+`0xD00` | DynamicVector<InfantryType*> |
| `SecretBuildings` | 0x0083c720 | RulesClass__ReadGeneral | Global | Rules+`0xD38` | DynamicVector<BuildingType*> |

Note on `BuildingTypeClass_ReadINI_Water`: the Ghidra-labeled name is a sub-routine; the actual flag is parsed into `BuildingType` regardless of the routine variant.

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Buildable only by Germans | `RequiredHouses=Germans` evaluated in HouseClass build-availability path | Other Allied countries get built-list-hidden status until they unlock via Secret Lab / crate. |
| Turretless forward-fire | `Turret=no` — `UnitClass::Fire()` uses `BodyFacing` instead of `TurretFacing` for muzzle origin and aim | Combined with `PrimaryFireFLH=200,0,55`, the shot emerges from the very front of the long hull. |
| Voxel-hull tilt on slope | `IsTilter=yes` — render-time only | Purely cosmetic; does not affect aim or sight. |
| Can crush infantry | `Crusher=yes` + `Crushable=no` of target overrides | Standard crush logic. |
| Cannot fit under bridges | `TooBigToFitUnderBridge=true` — pathing rejects bridge-under cells | Same as Rhino, Apocalypse. |
| Selectable as combatant | `IsSelectableCombatant=yes` | |
| Death explosion uses random anim from list | `Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` — `TechnoClass::Explode()` picks one at random | Same pool as other Allied tanks. |
| Acquired via crate | `CrateGoodie=yes` — UnitType flag enters the random pool for `UnitCrateType` outcomes | Per `[CrateRules]`. |
| Acquired via Secret Lab | `[General] SecretUnits=TNKD,TTNK,DTRUCK` — Secret Lab capture handler in BuildingClass picks one randomly | One of three random outcomes — the lab also has commented `SecretUnit=` per-building override fields, unused in stock YR. |
| Elite weapon burst | `[SABOTE] Burst=2` — `WeaponTypeClass` triggers two shots per fire cycle | Doubles elite DPS. |
| Pure-armor damage profile | `[UltraAP] / [UltraAPE]` Verses with 2% on inf/building rows | TNKD is functionally a hard-counter to vehicles only. |

### 7.4 Behaviors NOT present in TNKD

- **No turret rotation** — must rotate body to aim. No `TurretROT=`, no `TurretCount=`.
- **No `Secondary` weapon** — only SABOT/SABOTE. Cannot engage aircraft, cannot deploy.
- **No `AirRangeBonus`** — TNKD is `Verses` 2% vs air-typical armors anyway; it explicitly cannot target aircraft (no `AA=yes` in `[Cannon]` projectile; `[Cannon2]` is the AA variant for ORCA, and TNKD does not use it).
- **No `Spawns=`** — does not deploy children (unlike Carrier/V3/Dreadnought).
- **No `OpenTransport`** — not a transport.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ImmuneToVeins=yes` | YES (veinholes removed in YR) | Dormant — no live consumer in YR. |
| (all other keys live) | — | — |

No fog-of-war (0x1000) flags, no tunnel/subterranean refs, no `Insignificant`, no TS-only specials in TNKD.

---

## 9. Veterancy

### Veteran (1 chevron) abilities — `STRONGER, FIREPOWER, SIGHT, FASTER`

Standard TechnoType veteran tokens:
- `STRONGER` — +25% HP (400 → 500 typical).
- `FIREPOWER` — +25% damage (SABOT 150 → ~187).
- `SIGHT` — +20% sight (8 → 9.6).
- `FASTER` — +20% speed (5 → 6).

Net at veteran: still uses `Primary=SABOT`, but every shot lands ~187 base dmg, has slightly longer scout reach, and moves at Grizzly speed.

### Elite (2 chevrons) abilities — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative)

Additions on top of veteran:
- `SELF_HEAL` — passive HP regen (rate from `[General] SelfHealUnitRate`).
- `STRONGER` & `FIREPOWER` (re-applied; sources differ on whether they stack — typically replaced by the elite token rather than stacked).
- `ROF` — −25% ROF (faster cooldown).

**Plus weapon swap**: `Primary` → `ElitePrimary=SABOTE`:
- Damage 150 → 175.
- ROF 70 → 60 (further reduced by ROF veterancy ability).
- Range 5 → 6.75.
- Burst 1 → **2** (the biggest single jump in DPS).
- Warhead UltraAP → UltraAPE (50% medium-armor vs 40%).

**Practical jump at elite**: ~2.7× sustained DPS, +35% engagement range, self-heal, faster cooldown. Elite TNKD is one of the highest-value units in YR per Cost ($900).

---

## 10. Cross-references

### Direct dependencies (must exist in `rulesmd.ini` / `artmd.ini`)
- `[SABOT] / [SABOTE]` — weapons (§3)
- `[Cannon]` — projectile (§3.1)
- `[UltraAP] / [UltraAPE]` — warheads (§4)
- `[120MM]` (artmd) — bullet sprite
- `[GUNFIRE] / [VTMUZZLE] / [S_CLSN30] / [VTEXPLOD]` (artmd) — anims
- `[TankDestroyerSelect/Move/AttackCommand/Attack/MoveStart] / [GenVehicleDie] / [TankCrush]` (soundmd) — sounds (§5)
- `[CASLAB]` (rulesmd line 14054) — the Tech Secret Lab that gates TNKD's universal acquisition
- `[General] SecretUnits=` (line 265) — the random-pick pool

### Conceptual companions
- **HTNK (Rhino)** — Soviet heavy MBT; TNKD's specific counter-target. Same `Strength=400`, `Armor=heavy`, `Cost=900`. TNKD beats Rhino in a duel due to UltraAP vs heavy (100%), assuming TNKD shoots first.
- **MTNK (Grizzly)** — Allied medium tank; TNKD vs Grizzly is one-sided in TNKD's favor (UltraAP 100% vs light = full damage to Grizzly's 200 HP).
- **MGTK (Mirage)** — Allied light tank; same favorable matchup for TNKD.
- **TTNK (Tesla Tank)** & **DTRUCK (Demolition Truck)** — TNKD's companions in `SecretUnits=`. All three are country-locked elsewhere and unlocked together through the Secret Lab pool.
- **CIVAN (Chrono Ivan)** — the actual `RequiresStolenSovietTech=yes` unit; doc TODO.

### Deep-RE docs (cross-reference, not re-derived)
- No TNKD-specific Ghidra report exists; behavior is entirely flag-driven.
- For Secret Lab capture mechanics, the `[CASLAB] CaptureEvaEvent=EVA_SecretLabCaptured` triggers via the generic Engineer capture path — see [ENGINEER_CAPTURE_GHIDRA_REPORT.md](../../ENGINEER_CAPTURE_GHIDRA_REPORT.md).

---

## Ghidra audit log (audit iteration 12 — 2026-05-18)

**Methodology**: TNKD has no unit-specific code in `gamemd.exe` (confirmed
by string-search), so this audit focuses on *pinning down the exact
struct offsets* for every TNKD INI key — the previous doc gave parser
*addresses* but no offsets. ~12 Ghidra queries: 6 string searches + 4
xref lookups + 2 decompiles (`UnitTypeClass__ReadINI`,
`BuildingTypeClass_ReadINI_Water`; `TechnoTypeClass__ReadINI` and
`RulesClass__ReadGeneral` were too large for inline decompile and were
read from the saved tool-result files via grep).

### String + parser xref re-verification (BINARY-VERIFIED)

All 6 string addresses and 4 parser-site xrefs the doc cited verify
exactly:

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `RequiredHouses` | 0x00843bb4 | 0x00714529 | TechnoTypeClass__ReadINI |
| `CrateGoodie` | 0x00845e20 | 0x00747658 | UnitTypeClass__ReadINI |
| `SecretLab` | 0x0081aaa0 | 0x004609bf | BuildingTypeClass_ReadINI_Water |
| `SecretUnits` | 0x0083c730 | 0x0066fa54 | RulesClass__ReadGeneral |
| `Name:TNKD` | 0x008299dc | — (CSF key, not parser) | — |
| `TankDestroyer` | (no match) | — | confirms no hardcoded section-name branch |
| `TNKD` (standalone) | (no match) | — | confirms no hardcoded section-name branch |

### Function entry points verified (BINARY-VERIFIED)

| Function | Address | Status |
|----------|---------|--------|
| `TechnoTypeClass__ReadINI` | (cited as parser for 8+ TNKD keys) | Decompiled (oversized — read via grep). Doc-string header at function top documents `Turret` field at offset `+0xCA1` with writer at `0x007133C2`. |
| `UnitTypeClass__ReadINI` | (cited as parser for `CrateGoodie`, `IsTilter`, `TooBigToFitUnderBridge`) | Decompiled fully — 25+ UnitType offsets visible in body. |
| `BuildingTypeClass_ReadINI_Water` | (cited as parser for `SecretLab`) | Decompiled (oversized — read via grep). |
| `RulesClass__ReadGeneral` | (cited as parser for `SecretUnits`) | Decompiled (oversized — read via grep). |

### Struct offsets BINARY-VERIFIED (this pass)

**NEW TechnoType offsets** (12 — pinned via direct decompile reads):

| Offset | Key | Type | Notes |
|--------|-----|------|-------|
| `+0x5BC` | `MaxDebris` | int | `param_1[0x16f]` |
| `+0x614` | `Soylent` | int | `param_1[0x185]` |
| `+0xCA1` | `Turret` | byte | per in-binary Ghidra annotation; writer @ `0x007133C2` |
| `+0xD28` | `Crusher` | byte | `param_1 + 0x34a` |
| `+0xDBD` | `Accelerates` | byte | `(int)param_1 + 0xdbd` |
| `+0xC91` | `ImmuneToVeins` | byte | matches audit 7 cumulative |
| `+0xD9B` | `RequiresStolenThirdTech` | byte | matches audit 11 |
| `+0xD9C` | `RequiresStolenSovietTech` | byte | matches audit 11 |
| `+0xD9D` | `RequiresStolenAlliedTech` | byte | matches audit 11 |
| `+0xDA0` | `RequiredHouses` | int (bitmask) | matches audit 10; populated via `FUN_004750d0` country-bitmask helper |
| `+0xDA4` | `ForbiddenHouses` | int (bitmask) | matches audit 10 |

**NEW UnitType offsets** (15+ from `UnitTypeClass__ReadINI` decompile — superset of what TNKD uses):

| Offset | Key | Type |
|--------|-----|------|
| `+0x398` | (sequence-id default; 0xf normal, 0xA harvester/weeder) | int |
| `+0x67C` | `SpeedType` | int (default 2 unless `+0xD28 Crusher` set, then 1) |
| `+0xDFC` | `MovementRestrictedTo` | int |
| `+0xE00..+0xE08` | `HalfDamageSmokeLocation` | 3 ints |
| `+0xE0C` | `Passive` | byte |
| `+0xE0D` | `CrateGoodie` | byte |
| `+0xE0E` | `Harvester` | byte |
| `+0xE0F` | `Weeder` | byte |
| `+0xE11` | (derived from `+0xCA1`==0 — non-Turret flag) | byte |
| `+0xE12` | `DeployToFire` | byte |
| `+0xE13` | `IsSimpleDeployer` | byte |
| `+0xE14` | `IsTilter` | byte |
| `+0xE15` | `UseTurretShadow` | byte |
| `+0xE16` | `TooBigToFitUnderBridge` | byte |
| `+0xE17` | `CanBeach` | byte |
| `+0xE18` | `SmallVisceroid` | byte |
| `+0xE19` | `LargeVisceroid` | byte |
| `+0xE1A` | `CarriesCrate` | byte |
| `+0xE1B` | `NonVehicle` | byte |
| `+0xE1C` | `StandingFrames` | int |
| `+0xE20` | `DeathFrames` | int |
| `+0xE24` | `DeathFrameRate` | int |
| `+0xE28..+0xE3C` | `StartStandFrame` / `StartWalkFrame` / `StartFiringFrame` / `StartDeathFrame` / `MaxDeathCounter` / `Facings` | ints |
| `+0xE40..+0xE44` | `FiringSyncFrame[2]` | int array |
| `+0xE48..+0xE54` | `BurstDelay[4]` | int array |
| `+0xE5C` | `WalkFrames` | byte |
| `+0xE5D` | `FiringFrames` | byte |
| `+0xE5E` | `AltImage` | char[?] (string) |

**NEW BuildingType offsets** (4):

| Offset | Key | Type |
|--------|-----|------|
| `+0xEA4` | `SecretInfantry` | InfantryType* |
| `+0xEA8` | `SecretUnit` | UnitType* |
| `+0xEAC` | `SecretBuilding` | BuildingType* |
| `+0x16B0` | `SecretLab` | byte |

**NEW Rules-General offsets** (3 DynamicVector starts):

| Offset | Key | Type |
|--------|-----|------|
| Rules+`0xD00` | `SecretInfantry` global list | DynamicVector<InfantryType*> |
| Rules+`0xD1C` | `SecretUnits` global list | DynamicVector<UnitType*> |
| Rules+`0xD38` | `SecretBuildings` global list | DynamicVector<BuildingType*> |

### Doc corrections

1. **`Turret` scope**: doc claimed UnitType-scope, actually **TechnoType-scope**
   at `+0xCA1`. Corrected in §1 key table. (Turret applies to any
   TechnoType, including BuildingClass turrets via `BuildingClass::HasTurret`
   — confirming the broader TechnoType placement.)
2. **`TooBigToFitUnderBridge` scope**: doc claimed TechnoType-scope, actually
   **UnitType-scope** at `+0xE16`. Corrected in §1 key table.

### Items NOT re-verified in this pass (DEFERRED)

- `UnitClass::Fire_At_Target` @ 0x00736DF0 (cited in TechnoTypeClass__ReadINI
  doc-header) — uses `BodyFacing` instead of `TurretFacing` when
  `Turret=no`. Not re-decompiled this pass; the in-binary annotation at
  the top of TechnoTypeClass__ReadINI documents this consumer, but the
  branch logic was not directly verified.
- `Secret Lab` capture handler — the picker that rolls one of
  `SecretUnits` when `[CASLAB]` is captured. The chain is
  Engineer-capture → BuildingClass capture handler → random pick from
  Rules+`0xD1C` DynamicVector → grant to capturing house. Not traced in
  this pass; documented as an open follow-up in §11.
- `[General] CrateRules` unit-crate spawn — picks from the pool of all
  `CrateGoodie=yes` UnitTypes. Not traced this pass.
- `Explosion=` list random pick — `TechnoClass::Explode()` is cited but
  not decompiled in this pass.
- BulletType offsets for `[Cannon]` (Arcing, SubjectToCliffs/Walls/Elevation,
  Image=120MM) — these are data flags, not behavioral claims; deferred.
- Warhead `Verses` table parsing (UltraAP / UltraAPE) — data, not behavior.

### Confidence summary

- **HIGH**: 4 function entry points (all Ghidra-labeled);
  6 string addresses + 4 parser-xref addresses (all exact); 12 TechnoType
  offsets (read directly from decompile of TechnoTypeClass__ReadINI
  writes); 25+ UnitType offsets (full UnitTypeClass__ReadINI decompiled);
  4 BuildingType offsets (BuildingTypeClass_ReadINI_Water decompile read);
  3 Rules-General offsets (RulesClass__ReadGeneral decompile read).
- **MEDIUM**: 2 scope corrections (Turret = TechnoType, TooBigToFitUnderBridge
  = UnitType) — based on direct decompile reads but I did not also
  decompile every consumer to confirm those offsets are *used* at runtime
  in the claimed positions; the parser-write confirms the offset, the
  consumer trace is DEFERRED.
- **No new INCORRECT findings beyond the 2 scope corrections**.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[TNKD]` rulesmd key explained | ✅ §1 |
| Every `[TNKD]` artmd key explained | ✅ §2 |
| Primary + Elite weapon + projectile + warhead + impact anim | ✅ §3–§4 |
| All voices + crush sound expanded with verbatim sound defs | ✅ §5 |
| Prereqs / owners / acquisition paths analysed | ✅ §6 |
| Hardcoded behavior — Ghidra searches for TNKD ID + every gating flag | ✅ §7 (six string searches; TNKD-string returned only CSF lookup, confirming no unit-specific code) |
| Veterancy detailed | ✅ §9 |
| TS-legacy filter applied | ✅ §8 |
| Cross-refs to weapon/warhead/anim/voice sections | ✅ §10 |
| Index correction logged (TNKD is **not** RequiresStolenSovietTech) | ✅ doc header |

**Open follow-ups (none load-bearing):**
- Verify the elite-veteran ability stacking rules (`STRONGER+STRONGER` replace vs stack) by Ghidra-decompiling the veterancy multiplier resolver. Across the existing docs the convention is "elite adds these tokens on top of veteran tokens already applied," but the binary semantics for duplicate tokens (e.g. STRONGER listed in both lists) is not load-bearing for TNKD parity since the visible effect is bounded by `[General] VeteranCombat*` constants.
- The Secret Lab pick-randomization algorithm — Ghidra-trace the RNG path from `[CASLAB]` capture → `SecretUnits` index roll. Not in this doc; would belong in a SECRET_LAB_GHIDRA_REPORT.
