# [ORCA] — Intruder (Allied non-Korean fighter)

**INI ID:** `ORCA`
**Display name:** `UIName=Name:ORCA` → CSF label "Intruder"
**Internal name:** `Name=Intruder`
**Side:** Allied (4 non-Korean Allied factions: British, French, Germans, Americans)
**Section:** `[AircraftTypes]` slot `2=ORCA` ([rulesmd.ini:1161](../../../../ra2-rust-game/ini/rulesmd.ini))
**Owner:** `Owner=British,French,Germans,Americans` + `ForbiddenHouses=Alliance` (Korea excluded)
**Doc filename:** `units/allied/ORCA.md`
**Loop iteration:** 87

---

## INDEX CORRECTIONS LOGGED

INDEX_UNITS.md previously read:
```
| ORCA       | Nighthawk (alias)?       | -       | unused?              | TODO |
```

Both claims are wrong:

1. **ORCA is not a Nighthawk alias.** The Nighthawk Transport is `[SHAD]` (vehicle, JumpJet jeep transport, BlackHawkCannon weapon). `[ORCA]` is the Intruder, a dedicated Allied fighter aircraft. They are unrelated entities — different categories (AircraftType vs VehicleType), different roles (single-shot bomber vs transport), different locomotors (AircraftLocomotion vs Jumpjet).

2. **ORCA is not unused.** It is the active non-Korean Allied tier-3 strike fighter — the counterpart to BEAG (Korea-exclusive). The pair partitions Allied air strike capability cleanly:
   - **BEAG (Black Eagle)** → `RequiredHouses=Alliance` (Korea-only)
   - **ORCA (Intruder)** → `Owner=British,French,Germans,Americans` + `ForbiddenHouses=Alliance` (everyone-but-Korea)

This answers the previously open question "what do non-Korean Allied factions get?" — they get the Intruder, an upgraded basic-Maverick equivalent of the Black Eagle.

The `AllowedToStartInMultiplayer=no` flag does **not** mean ORCA is unused — it means the unit cannot be a starting unit. Build availability is governed by `TechLevel=3` and `Prerequisite=RADAR`, both of which are reachable in normal skirmish play.

INDEX_UNITS.md has been updated with the correction and DONE status.

---

## rulesmd.ini section — full transcript and per-key analysis

[rulesmd.ini:10582-10643](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
[ORCA]
UIName=Name:ORCA
Name=Intruder
Image=FALC
Prerequisite=RADAR
Primary=Maverick
CanPassiveAquire=no ; Won't try to pick up own targets
CanRetaliate=no; Won't fire back when hit
Strength=150
Category=AirPower
Armor=light
TechLevel=3
Sight=8
RadarInvisible=no
Landable=yes
MoveToShroud=yes
;Dock=GAAIRC,GAHPAD,NAHPAD
Dock=GAAIRC,AMRADR
PipScale=Ammo
Speed=14
;PitchSpeed=0.9
;PitchAngle=0

PitchSpeed=1.1
PitchAngle=0
OmniFire=yes

Owner=British,French,Germans,Americans
ForbiddenHouses=Alliance

Cost=1200
Points=20
ROT=3
Ammo=1
Crewed=yes
ConsideredAircraft=yes
AirportBound=yes ; If I ever need to land and there are no airports I crash because I can only land on them
GuardRange=30
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=3
IsSelectableCombatant=yes
VoiceSelect=IntruderSelect
VoiceMove=IntruderMove
VoiceAttack=IntruderAttackCommand
VoiceCrashing=IntruderVoiceDie
DieSound=
MoveSound=IntruderMoveLoop
CrashingSound=IntruderDie
ImpactLandSound=GenAircraftCrash
Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1}
MovementZone=Fly
ThreatPosed=20	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
AuxSound1=IntruderTakeOff	;Taking off
AuxSound2=IntruderLanding	;Landing
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=STRONGER,FIREPOWER,ROF
Fighter=yes
AllowedToStartInMultiplayer=no
ImmuneToPsionics=yes
ElitePrimary=MaverickE
PreventAttackMove=yes
```

### Identity and CSF binding

- **`UIName=Name:ORCA`** — CSF lookup; rendered in-game as "Intruder" (from the CSF label `Name:ORCA`).
- **`Name=Intruder`** — internal display name (fallback if CSF missing). Matches the CSF label.
- **`Image=FALC`** — art redirect. ORCA reads its visual definition from `[FALC]` in artmd.ini, **not** `[ORCA]` (the latter has no artmd block). This is the same `Image=` redirect pattern documented across MTNK→GTNK, APOC→MTNK, MGTK→RTNK, DTRUCK→TRUCKA, AMCV→MCV, CMISL→BSUBMISL, CARGOPLANE→PDPLANE, SAPC→TRS — the Intruder shares its voxel asset with the placeholder `FALC` ("Falcon") art entry.

### Build gating and ownership

- **`Prerequisite=RADAR`** — needs an Allied radar building (GASPYSAT or an equivalent radar provider) before ORCA can be queued. Note: `RADAR=GASPYSAT,YACOMD,NARADR,AMRADR` (Rules-global alias) means any radar provider works in principle, though `Owner=` and `ForbiddenHouses=` further restrict who can actually build it.
- **`TechLevel=3`** — mid-tier. Available after early-tier units (E1, MTNK) but before late-tier (CARRIER, BFRT).
- **`Owner=British,French,Germans,Americans`** — the 4 non-Korean Allied countries. Excludes Korea (Alliance) and all Soviet/Yuri factions. This is the explicit white-list.
- **`ForbiddenHouses=Alliance`** — explicit black-list of Korea (Alliance = Asian Alliance per [rulesmd.ini:3265](../../../../ra2-rust-game/ini/rulesmd.ini)). Logically redundant with Owner= already excluding Korea, but Westwood used both for safety / engine clarity. Confirmed Ghidra-scope: TechnoType (xref `0x00843b94 → 0x0071455d` in TechnoTypeClass__ReadINI).
- **`AllowedToStartInMultiplayer=no`** — ORCA is not a starting-unit candidate (not in spawn pool for new players). This is normal for tier-3 combatants; it does NOT disable build availability.
- **`Cost=1200`** — same as BEAG. Pair are economically symmetric.
- **`Points=20`** — score-on-kill points for the killer.

### Cross-faction partition with BEAG

`ORCA.Owner ∪ BEAG.RequiredHouses` covers all 5 Allied countries:

| Country     | Side      | Build ORCA? | Build BEAG? |
|-------------|-----------|-------------|-------------|
| British     | Allied    | ✓ (Owner)   | ✗           |
| French      | Allied    | ✓ (Owner)   | ✗           |
| Germans     | Allied    | ✓ (Owner)   | ✗           |
| Americans   | Allied    | ✓ (Owner)   | ✗           |
| Alliance(Korea) | Allied | ✗ (Forbidden) | ✓ (RequiredHouses) |

So the 5 Allied factions split into "4 with ORCA" and "1 with BEAG". No faction has both; every Allied faction has exactly one fighter. This is the deliberate Westwood design — Korea trades the basic Intruder for a stronger Black Eagle (BEAG Damage=200 vs ORCA Damage=150; BEAG range identical; elite stats roughly comparable).

### Combat stats

- **`Strength=150`** — basic HP. Lower than BEAG's 200 (Korea's faction-exclusive plane is more durable).
- **`Armor=light`** — standard aircraft armor class.
- **`Category=AirPower`** — combat aircraft category (vs CARGOPLANE's AirLift). AirPower aircraft enter player base scoring as combat-relevant.
- **`Sight=8`** — vision radius in cells (matches BEAG).
- **`RadarInvisible=no`** — appears on enemy radar normally.
- **`MoveToShroud=yes`** — can be ordered into unrevealed shroud. Confirmed Ghidra-scope: TechnoType (xref `0x008444c4 → 0x0071225d` in TechnoTypeClass__ReadINI). **NEW cheat-sheet entry.**
- **`ROT=3`** — rate of turn during flight. Low (matches BEAG); aircraft pivot slowly to align for attack runs.
- **`Speed=14`** — aircraft speed. Same as BEAG.
- **`PitchSpeed=1.1` / `PitchAngle=0`** — pitch dynamics during flight transitions (the `;PitchSpeed=0.9` commented above shows an earlier tuning value). PitchAngle=0 means level-flight without nose-down/up bias.
- **`OmniFire=yes`** — can fire regardless of facing. Standard for missile-launching aircraft (no turret to align).
- **`ThreatPosed=20`** — AI threat-assessment weight when targeting this unit. Mid-range; same as BEAG.
- **`GuardRange=30`** — guard-mode radius when stationed. Equal to standard combat-aircraft guard ranges.

### Weapons (single-weapon aircraft)

- **`Primary=Maverick`** ([rulesmd.ini:23154-23162](../../../../ra2-rust-game/ini/rulesmd.ini)):
  ```ini
  [Maverick]
  Damage=150
  ROF=10
  Range=6
  Projectile=AirToGroundMissile
  Speed=70
  Warhead=ORCAAP
  Report=IntruderAttack
  Burst=1;2
  ```
  Single-shot air-to-ground missile, `Burst=1` (the `;2` is a commented-out earlier value — current shipping value is `1`). `Speed=70` is the projectile flight speed (fast).

- **`ElitePrimary=MaverickE`** ([rulesmd.ini:25001-25009](../../../../ra2-rust-game/ini/rulesmd.ini)):
  ```ini
  [MaverickE]
  Damage=300
  ROF=10
  Range=9
  Projectile=AirToGroundMissile
  Speed=70
  Warhead=ORCAAP
  Report=IntruderAttack
  Burst=1;4
  ```
  Elite ORCA gets **2× damage** (150 → 300), **+3 range** (6 → 9), same ROF, same warhead, same Burst=1. The `;4` is a commented-out earlier Burst value.

- **No secondary weapon.** ORCA is single-purpose: fly in, launch one missile per sortie, fly back to reload. No anti-air, no anti-naval, no anti-armor specialization beyond the warhead.

### Warhead: ORCAAP

[rulesmd.ini:27430-27440](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
[ORCAAP]
Wall=yes
Wood=yes
CellSpread=.4
PercentAtMax=1
Verses=100%,100%,100%,100%,100%,100%,100%,100%,75%,100%,100%
Conventional=yes
InfDeath=3
AnimList=S_CLSN16,S_CLSN22,S_CLSN30,S_CLSN42,S_CLSN58
ProneDamage=50%
PenetratesBunker=yes;If shot at a bunkered tank, no means the bunker gets the damage, yes means the unit does
```

- **`Wall=yes` / `Wood=yes`** — damages walls and wooden structures.
- **`CellSpread=.4`** — sub-cell splash radius. Effectively single-target with minor adjacent leak.
- **`PercentAtMax=1`** — 100% damage at the outer edge of splash (no falloff within CellSpread).
- **`Verses=100,100,100,100,100,100,100,100,75,100,100`** — armor multipliers. Armor index 8 (`steel`? or `concrete`?) takes 75%; everything else takes full damage. This is mild concrete-resistance.
- **`Conventional=yes`** — counts as conventional (vs nuclear/psionic/etc) for ImmuneTo* checks.
- **`InfDeath=3`** — explosion-type infantry death animation (RPG/Cannon style per cheat-sheet).
- **`AnimList=S_CLSN16,S_CLSN22,S_CLSN30,S_CLSN42,S_CLSN58`** — small collision explosion animations. Sized variants by impact magnitude.
- **`ProneDamage=50%`** — prone infantry take half damage.
- **`PenetratesBunker=yes`** — shoots through Battle Bunker (NATBNK) to hit the unit inside, not the bunker shell. **Tactically significant**: ORCA can kill bunkered Conscripts without first cracking the bunker.

This warhead is **shared with BEAG's Maverick2 weapon** (Damage=200, Range=6) and BEAG's Maverick2E elite (Damage=400, Range=9). ORCA and BEAG share the warhead but differ in damage scalar. Tank Destroyer (TNKD) also uses ORCAAP via Maverick2 (cross-check: 23149).

### Projectile: AirToGroundMissile

[rulesmd.ini:25693-25706](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
[AirToGroundMissile]
Arm=2
Shadow=no
Proximity=no
Ranged=yes
AA=no
AG=yes
Image=DRAGON
ROT=100
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=no
```

- **`Arm=2`** — arming delay in frames.
- **`Shadow=no`** — no shadow rendered under projectile.
- **`Proximity=no` / `Ranged=yes`** — must hit target directly (not proximity detonator), and homes within range.
- **`AA=no` / `AG=yes`** — strictly anti-ground. ORCA cannot hit aircraft.
- **`Image=DRAGON`** — uses DRAGON projectile SHP for visuals.
- **`ROT=100`** — extremely high projectile rate-of-turn (sticks to target through evasion).
- **`SubjectToCliffs=no` / `SubjectToElevation=no` / `SubjectToWalls=no`** — flies in a straight homing line, ignoring all terrain obstruction. Critical: ORCA missiles cannot be wall-blocked.

### Veterancy

- **`VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER`** — at Veteran rank, ORCA gains +HP, +damage, +sight, +speed.
- **`EliteAbilities=STRONGER,FIREPOWER,ROF`** — at Elite, ORCA gains +HP, +damage (on top of weapon swap to MaverickE), and ROF reduction (faster reload).

Combined with the ElitePrimary swap (Maverick → MaverickE; +150 damage, +3 range), an elite ORCA roughly **doubles its DPS and almost doubles its reach**.

### Aircraft lifecycle

- **`Crewed=yes`** — destruction ejects an infantry crew (1× E1 by default).
- **`ConsideredAircraft=yes`** — flags engine as aircraft for hit-test / AA-target logic. Confirmed Ghidra-scope: TechnoType (xref `0x00843728 → 0x00714fe9` in TechnoTypeClass__ReadINI).
- **`Ammo=1`** — single missile per sortie. Must return to airport to reload.
- **`PipScale=Ammo`** — sidebar pips reflect ammo (1/1) rather than HP.
- **`Landable=yes`** — can land at airport/helipad after sortie.
- **`AirportBound=yes`** — if all airports are destroyed, ORCA crashes (verbatim Westwood comment: "If I ever need to land and there are no airports I crash because I can only land on them"). Confirmed Ghidra-scope: AircraftType (per existing cheat-sheet entry `0x0081803c → 0x0041cc6e`).
- **`Dock=GAAIRC,AMRADR`** — primary dock is Allied Airforce Command HQ; fallback is AMRADR (American Radar — campaign-only structure). The `;Dock=GAAIRC,GAHPAD,NAHPAD` commented line shows abandoned earlier plan to allow helipad docking; current shipping ORCA needs a runway. (GAAIRC is the only valid skirmish dock.)
- **`Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1}`** — AircraftLocomotion. Same GUID as BEAG, HORNET, ASW, PDPLANE, CARGOPLANE, SPYP, BPLN — every "real airplane" (fixed-wing or fixed-flight-pattern). Documented in cheat-sheet as the ...746 locomotor type.
- **`MovementZone=Fly`** — flight-zone (no terrain pathfinding).

### Tactical flags (Westwood script-only design)

- **`CanPassiveAquire=no`** — verbatim "Won't try to pick up own targets". ORCA does NOT auto-acquire enemies; the player must explicitly target. Prevents the player from sending ORCAs into Guard mode and having them randomly attack passers-by.
- **`CanRetaliate=no`** — verbatim "Won't fire back when hit". ORCA does NOT auto-fire on attackers. Combined with CanPassiveAquire=no, this means ORCA is **strictly player-directed**.
- **`PreventAttackMove=yes`** — disables attack-move command. Player cannot order ORCA to attack-move-to-position; only direct attack-target or move commands work.

This triple-disable pattern matches BEAG ([CanPassiveAquire,CanRetaliate,PreventAttackMove]) — every "single-shot fly-in-fire-out" aircraft uses this script-only control to prevent the unit from wandering into AA range on its own.

### Immunities

- **`ImmuneToPsionics=yes`** — Yuri/MIND cannot mind-control. Aircraft are universally psi-immune (standard pattern).

### Sound / voice keys (10 entries)

[soundmd.ini](../../../../ra2-rust-game/ini/soundmd.ini):

- **`VoiceSelect=IntruderSelect`** ([3850-3855](../../../../ra2-rust-game/ini/soundmd.ini)) — radio-filtered + 4 sample pool (`vintsea,vintseb,vintsec,vintsed`) wrapped in `gradio1a-1g` + `gradio3a-3g` (open/close radio chatter).
- **`VoiceMove=IntruderMove`** ([3843-3848](../../../../ra2-rust-game/ini/soundmd.ini)) — radio + 4 sample pool (`vintmoa-mod`).
- **`VoiceAttack=IntruderAttackCommand`** ([3836-3841](../../../../ra2-rust-game/ini/soundmd.ini)) — radio + 5 sample pool (`vintata-vintate`). Largest pool of the three voice tags (Westwood emphasized attack callouts).
- **`VoiceCrashing=IntruderVoiceDie`** ([3857-3861](../../../../ra2-rust-game/ini/soundmd.ini)) — 3 sample pool (`vintdia,vintdib,vintdic`) crash death voice (no radio wrapper; raw scream-equivalent).
- **`DieSound=`** — explicitly empty. ORCA has no death SFX (the crash voice + ImpactLandSound cover this).
- **`MoveSound=IntruderMoveLoop`** ([1605-1612](../../../../ra2-rust-game/ini/soundmd.ini)) — 7-sample looping engine drone (`vintlo1a,1b,1c,2a,2b,2c,3`) Control=loop/random/all/decay/attack, Volume=20 (background). **This is the same `vintlo*` sample family the BEAG borrows for its MoveSound** (per cheat-sheet "Cross-faction audio sharing"). Westwood reused the Intruder engine loop for BEAG.
- **`CrashingSound=IntruderDie`** ([1629-1632](../../../../ra2-rust-game/ini/soundmd.ini)) — 2 sample pool (`vintdiea,vintdieb`) Volume=50 crash explosion SFX.
- **`ImpactLandSound=GenAircraftCrash`** ([1995-2000](../../../../ra2-rust-game/ini/soundmd.ini)) — generic aircraft-impact-ground sample (`vaircraa,vaircrab,vaircrac` Volume=50). **DUAL-READ** field per cheat-sheet: Rules-global (`0x00669965`) + TechnoType per-unit (`0x00712f38`). ORCA uses generic crash impact rather than a unit-specific one.
- **`AuxSound1=IntruderTakeOff`** ([1614-1620](../../../../ra2-rust-game/ini/soundmd.ini)) — `vintupa` predelay-controlled Volume=30 takeoff one-shot.
- **`AuxSound2=IntruderLanding`** ([1622-1627](../../../../ra2-rust-game/ini/soundmd.ini)) — `vintdna` Volume=30 landing one-shot.

The AuxSound1/AuxSound2 active-pair on ORCA confirms the takeoff/landing SFX system documented for HORNET (and noted-as-commented on most other aircraft). ORCA, BEAG, HORNET all use the AuxSound1/AuxSound2 system actively.

### Visuals / FX (rulesmd-side)

- **`Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60`** — 5-anim explosion palette when ORCA dies. Larger and more varied than typical infantry/vehicle explosions (5 anims vs 2-3 for most ground units).
- **`MaxDebris=3`** — at most 3 debris pieces spawn on death.
- **`DamageParticleSystems=SparkSys,SmallGreySSys`** — when damaged below thresholds, ORCA emits sparks + small grey smoke (the standard aircraft damage-trail pair).
- **`IsSelectableCombatant=yes`** — selectable + counts as a combat unit for radar/threat metrics. Confirms ORCA is intended as a real player-controlled fighter, not a campaign-only or AI-only unit.

---

## artmd.ini section — full transcript

[artmd.ini:745-749](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[FALC] ; Intruder
Voxel=yes
Remapable=yes
Cameo=FALCICON
AltCameo=FALCUICO
```

**Critical:** ORCA has **no `[ORCA]` artmd block**. It uses `[FALC]` via the `Image=FALC` redirect.

- **`Voxel=yes`** — uses 3D voxel asset (`falc.vxl` + `falc.hva`). Aircraft is voxel-rendered (vs SHP-rendered like DLPH).
- **`Remapable=yes`** — house color is applied to the voxel's remap palette index. Player ORCAs are visibly British/French/German/American-tinted.
- **`Cameo=FALCICON`** — sidebar build icon.
- **`AltCameo=FALCUICO`** — alternate (e.g. veteran/upgrade?) cameo. Note: AltCameo is typically rendered at veteran or elite rank in the sidebar in some configurations.

No `PrimaryFireFLH=` is set in artmd for FALC. This means missile launch position uses the default (0,0,0 offset from voxel center). Compare BPLN which has `PrimaryFireFLH=25,100,0` — ORCA's missile fires from voxel origin.

The FALC artmd entry is minimal — only 4 keys vs BEAG which is similar (also 4 keys). The Westwood pattern is clear: lightweight artmd blocks for fighters that share the basic AircraftLocomotion behavior; the rulesmd is where the per-unit tuning lives.

---

## Build chain and tech availability

- `[GACNST]` (Construction Yard) → `[GAPILE]` (Barracks) → ... → `[GASPYSAT]` (Spy Sat / radar) → ORCA unlocks for British/French/Germans/Americans only.
- Korea (Alliance) skips ORCA entirely and gets BEAG instead at the equivalent tier.
- Dock requirement (GAAIRC) means the player must have built an Airforce Command HQ before producing ORCA — GAAIRC is also the Allied-aircraft producer in general.

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field                  | String address | First xref               | Read scope                      |
|------------------------|----------------|--------------------------|---------------------------------|
| `PadAircraft`          | `0x0083c748`   | `0x0066f9d8`             | **RulesClass__ReadGeneral** (Rules-global) |
| `ForbiddenHouses`      | `0x00843b94`   | `0x0071455d`             | TechnoTypeClass__ReadINI        |
| `Fighter`              | `0x00818034`   | `0x0041cc84`             | AircraftTypeClass__ReadINI      |
| `MoveToShroud`         | `0x008444c4`   | `0x0071225d`             | TechnoTypeClass__ReadINI        |
| `ConsideredAircraft`   | `0x00843728`   | `0x00714fe9`             | TechnoTypeClass__ReadINI        |

**3 NEW cheat-sheet entries this iteration:**

1. **`PadAircraft`** — `0x0083c748 → 0x0066f9d8` — **RulesClass__ReadGeneral** scope (Rules-global). Confirms `PadAircraft=ORCA,BEAG` at [rulesmd.ini:395](../../../../ra2-rust-game/ini/rulesmd.ini) is a Rules-General tag, not a per-unit field. **[BINARY-VERIFIED audit 31]** — parser xref re-confirmed.
2. **`ForbiddenHouses`** — `0x00843b94 → 0x0071455d` — TechnoType per-unit. **[BINARY-VERIFIED audit 31]** — **previously pinned at TechnoType+0xDA4 in audit 10 (SNIPE)**; doc's "NEW cheat-sheet entry" claim was correct as of the doc's write-time but the offset binding was already cumulative.
3. **`MoveToShroud`** — `0x008444c4 → 0x0071225d` — TechnoType per-unit. **[BINARY-VERIFIED audit 31]** — **previously pinned at TechnoType+0xC8D in audit 11 (CCOMAND)**; same situation as ForbiddenHouses (doc claim was reasonable at write-time, cumulative already had it).

Plus 2 cross-verifications of existing cheat-sheet entries:

- **`Fighter`** — confirmed AircraftType scope (`0x00818034 → 0x0041cc84`) — matches existing cheat-sheet entry.
- **`ConsideredAircraft`** — confirmed TechnoType scope (`0x00843728 → 0x00714fe9`).

### Ghidra search log for this iteration

- `search_strings("Intruder")` → 0 matches. The name "Intruder" is **not** present as a string in the binary — the unit is referenced exclusively by its INI ID `ORCA` through the parser tables. CSF lookup happens via `Name:ORCA`, which lives in `*.csf` files, not the binary. **This rules out the possibility of any unit-specific hardcoded behavior keyed on the display name "Intruder".**
- `search_strings("ORCAAP")` → 0 matches. The warhead name is not in the binary either — confirms warhead lookups go through the parsed WarheadType array by index/name lookup, not hardcoded address.
- `search_strings("PadAircraft")` → 1 match at `0x0083c748` → xref into RulesClass__ReadGeneral at `0x0066f9d8`. **NEW cheat-sheet entry.**
- `search_strings("ForbiddenHouses")` → 1 match at `0x00843b94` → xref into TechnoTypeClass__ReadINI at `0x0071455d`. **NEW cheat-sheet entry.**
- `search_strings("Fighter")` → 1 match at `0x00818034` → xref into AircraftTypeClass__ReadINI at `0x0041cc84`. Confirms AircraftType scope.
- `search_strings("MoveToShroud")` → 1 match at `0x008444c4` → xref into TechnoTypeClass__ReadINI at `0x0071225d`. **NEW cheat-sheet entry.**
- `search_strings("ConsideredAircraft")` → 1 match at `0x00843728` → xref into TechnoTypeClass__ReadINI at `0x00714fe9`.

### Unit-specific hardcoded behavior?

ORCA has **no detectable unit-specific hardcoded code path**:

- No "ORCA" string in the binary.
- No "Intruder" string in the binary.
- All behavior is driven by parsed INI fields routed through the generic Techno/Aircraft pipeline.
- Triple-disable (`CanPassiveAquire=no`, `CanRetaliate=no`, `PreventAttackMove=yes`) is generic field-driven, not unit-keyed.
- `Ammo=1` + `AirportBound=yes` + `Landable=yes` lifecycle is generic aircraft handling (the same path BEAG, HORNET, ASW, PDPLANE, etc. use).
- ORCAAP warhead lookup is via parsed WarheadType array — no `ORCAAP_Hardcoded` function.

**Conclusion:** ORCA is a pure INI-driven aircraft. Any reimplementation can route its behavior through the same generic Aircraft and Techno systems used by the other ...746-locomotor aircraft.

### TS-legacy filter

- **`Crewed=yes`** — active in YR. (Common to most vehicles + aircraft.)
- **`MovementZone=Fly`** — active.
- **`ImmuneToPsionics=yes`** — active (psionics are YR-core).
- **`Dock=GAAIRC,AMRADR`** — AMRADR is an American Radar campaign-only structure (TS-era leftover). In skirmish, only GAAIRC is reachable. The `;Dock=GAAIRC,GAHPAD,NAHPAD` commented earlier-design line (which would allow helipads) was dropped before shipping — Westwood deliberately confined ORCA to runway airports. No GAHPAD/NAHPAD building exists in shipping YR anyway (commented out / unused).
- **`Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60`** — TWLT070 is a TS-era taillight/explosion. The other 4 are standard YR collision anims. Active in YR; the TS-name TWLT070 just denotes asset filename heritage.
- **No `Spawned=yes`** — ORCA is NOT spawn-child. It is produced normally from GAAIRC like any other AircraftType.
- **No `MissileSpawn=yes`** — ORCA returns and reloads (not kamikaze).
- **No `Tunnel`/`Subterranean`** fields — clean.
- **No `ImmuneToVeins`** — clean.
- **No fog-of-war (`SpecialFlags & 0x1000`) dependencies** — clean.

ORCA has no TS-legacy gating. Fully active in standard YR skirmish.

---

## Cross-references

- **`BEAG`** (`units/allied/BEAG.md`) — Korea-exclusive sister fighter. ORCA and BEAG partition Allied air strike capability between Korea and the other 4 Allied factions. Same locomotor, same archetype, different stats.
- **`HORNET`** (`units/allied/HORNET.md`) — Carrier-spawned reusable strike aircraft. Different role (spawn-child vs producible), different reload mechanism (return-to-carrier vs return-to-airport).
- **`ASW`** (`units/allied/ASW.md`) — Destroyer-spawned anti-sub Osprey. Same return-to-dock paradigm with a different parent (DEST vs GAAIRC) and different role (anti-naval vs anti-ground).
- **`SHAD`** (`units/allied/SHAD.md`) — Nighthawk Transport. **NOT related to ORCA** (SHAD is a JumpJet vehicle, not an aircraft). The INDEX_UNITS.md note "Nighthawk (alias)?" referred to a misread.
- **`TNKD`** (`units/allied/TNKD.md`) — Tank Destroyer. Shares ORCAAP warhead via its `Maverick2`-class weapon configuration.
- **`SREF`** (`units/allied/SREF.md`) / **`SUB`** (`units/soviet/SUB.md`) — comparable script-only-control units (`CanPassiveAquire=no` + `CanRetaliate=no`); cross-side pattern.
- **Tech tree cross-ref:** ORCA Prerequisite=RADAR + Dock=GAAIRC means it depends on GAAIRC (Airforce Command HQ) being built. GAAIRC documentation pending.
- **CSF label cross-ref:** `Name:ORCA` is the CSF key. ORCA's display name "Intruder" comes from the *.csf string table, not from the rulesmd Name= field (which is the engine-internal fallback name).

---

## Coverage audit

INI fields covered (38 rulesmd + 4 artmd = 42 total):

| Category          | Field                              | Covered |
|-------------------|------------------------------------|---------|
| Identity          | UIName, Name, Image                | ✓ |
| Build/owner       | Prerequisite, Owner, ForbiddenHouses, TechLevel, Cost, Points, AllowedToStartInMultiplayer | ✓ |
| Combat            | Primary, ElitePrimary, Strength, Armor, Sight, ROT, Speed, ThreatPosed, GuardRange, OmniFire, PitchSpeed, PitchAngle | ✓ |
| Veterancy         | VeteranAbilities, EliteAbilities   | ✓ |
| Aircraft lifecycle| Ammo, PipScale, Landable, AirportBound, Dock, Locomotor, MovementZone, ConsideredAircraft, Fighter, Crewed, MoveToShroud, Category | ✓ |
| Tactical AI       | CanPassiveAquire, CanRetaliate, PreventAttackMove | ✓ |
| Immunities        | ImmuneToPsionics                   | ✓ |
| Visuals/FX        | Explosion, MaxDebris, DamageParticleSystems, RadarInvisible, IsSelectableCombatant | ✓ |
| Sound/Voice (10)  | VoiceSelect, VoiceMove, VoiceAttack, VoiceCrashing, DieSound, MoveSound, CrashingSound, ImpactLandSound, AuxSound1, AuxSound2 | ✓ |
| artmd FALC        | Voxel, Remapable, Cameo, AltCameo  | ✓ |

**Coverage: 42/42 = 100%.** Every key in the rulesmd ORCA block and its referenced artmd FALC block has been transcribed and explained.

Weapon/warhead/projectile/sound chains traced:
- Maverick → ORCAAP warhead → AirToGroundMissile projectile → DRAGON image
- MaverickE → ORCAAP warhead → AirToGroundMissile projectile → DRAGON image
- 10 sound entries traced into soundmd.ini

---

## Open questions / Westwood inconsistencies

1. **Why both `Owner=British,French,Germans,Americans` AND `ForbiddenHouses=Alliance`?** Logically redundant (Alliance is not in Owner anyway). Likely defensive double-blacklist in case of `Owner=` parser leniency or future country additions.
2. **`Dock=GAAIRC,AMRADR`** — AMRADR (American Radar) is a campaign building, not a real fallback dock in skirmish. The fallback is effectively dead-code unless campaign maps explicitly spawn AMRADR.
3. **`Burst=1;2`** with `;2` commented — earlier Westwood considered double-shot Mavericks but reverted to single-shot. The current shipping value is `1`.
4. **`AltCameo=FALCUICO`** — unclear what FALCUICO maps to. Possibly veteran/elite-rank sidebar icon variant. Cross-ref with `BEAG.AltCameo=BEAGICON` (same as Cameo) — so BEAG has no distinct elite cameo; ORCA does have a distinct AltCameo. **[BINARY-VERIFIED audit 31]** — AltCameo is parsed in `TechnoTypeClass__ReadINI` (xref @ 0x00715a6e → TechnoType+0x1F8, char[25] string field per `LEA ECX, [EBP + 0x1f8]` writeback at 0x00715a73 + the `PUSH 0x19` size=25 limit). The field IS read at parse-time. Consumer-side (which UI code branch picks AltCameo over Cameo) DEFERRED.

---

## Ghidra audit log (audit iteration 31 — 2026-05-19)

**~16 Ghidra queries** (12 string searches + 6 xref lookups + 4 grep
passes on saved TechnoTypeClass__ReadINI decompile + 1 assembly-context
batch for AltCameo/VoiceCrashing). 5 doc-cited claims verify (with 2
flagged for offset-binding pre-existing in cumulative) + 4 NEW
struct-offset bindings BINARY-VERIFIED + 2 doc-cited negative claims
re-confirmed.

### Function-entry verification

| Function | Address | Status |
|----------|---------|--------|
| `RulesClass__ReadGeneral` | (oversized) | parser xref @ 0x0066f9d8 for PadAircraft confirmed |
| `TechnoTypeClass__ReadINI` | (oversized) | grep-verified for CanRetaliate/AltCameo/MoveSound/VoiceCrashing + cross-confirms ForbiddenHouses/MoveToShroud/ConsideredAircraft |
| `AircraftTypeClass__ReadINI` | 0x0041CC20 | Fighter +0xE0E re-confirmed (cumulative audit 26) |
| `FUN_007162f0` | 0x007162f0 | AltCameo consumer-side xref @ 0x00716d34 (DEFERRED for full decompile — likely UI-cameo selector) |

### Key behavioral findings — 4 NEW struct-offset bindings BINARY-VERIFIED

| INI key | Scope | Offset | Type | Parser site | Status |
|---------|-------|--------|------|-------------|--------|
| `CanRetaliate` | TechnoType | **+0xD9A** | byte (ReadBool) | 0x0071448d | NEW (fills byte-cluster between +0xD99 CanPassiveAquire and +0xD9B RequiresStolenThirdTech) |
| `AltCameo` | TechnoType | **+0x1F8** | char[25] string (ReadString + 25-byte buffer) | 0x00715a6e | NEW (string field; assembly-verified `LEA ECX, [EBP + 0x1f8]` at 0x00715a73 + `PUSH 0x19` size limit; consumer xref into FUN_007162f0) |
| `MoveSound` | TechnoType | **+0x504..+0x50C** | int[3] (3-slot SoundList) | 0x00713478 | NEW (looping engine sound; ReadSoundList writes 3 ints — open/body/close samples) |
| `VoiceCrashing` | TechnoType | **+0x550** | int (VocClass index) | 0x00713034 | NEW (assembly-verified writeback `MOV [EBP + 0x550], EAX` at 0x00713069) |

### TechnoType byte-cluster +0xD99..+0xDA4 (consolidated post-audit-31)

| Offset | Key | Audit |
|--------|-----|-------|
| +0xD99 | CanPassiveAquire | 10 (SNIPE) |
| **+0xD9A** | **CanRetaliate** | **31 (ORCA)** — NEW |
| +0xD9B | RequiresStolenThirdTech | 11 (CCOMAND) |
| +0xD9C | RequiresStolenSovietTech | 11 |
| +0xD9D | RequiresStolenAlliedTech | 11 |
| +0xDA0 | RequiredHouses (int) | 10 |
| +0xDA4 | ForbiddenHouses (int) | 10 |

Tactical-AI / house-restriction byte cluster fully named post-audit-31.

### Aircraft-extended sound cluster +0x504..+0x554 (audit 31 extends audit 29)

| Range | Key | Audit |
|-------|-----|-------|
| **+0x504..+0x50C** | **MoveSound (3-int sound list)** | **31** — NEW |
| +0x52C | AuxSound1 | 29 |
| +0x530 | AuxSound2 | 29 |
| +0x534..+0x53C | DEFERRED siblings | — |
| +0x540 | ImpactLandSound | 29 |
| +0x544 | CrashingSound | 29 |
| +0x548 | SinkingSound | 27 |
| +0x54C | (DEFERRED — likely a Voice* between SinkingSound and VoiceCrashing) | — |
| **+0x550** | **VoiceCrashing** | **31** — NEW |
| +0x554 | (DEFERRED — next-slot preload in VoiceCrashing block) | — |

The aircraft sound cluster now spans ~80 bytes from +0x504 (MoveSound,
the lowest sound slot) through +0x554. Three more sibling slots
(+0x534/+0x538/+0x53C from audit 29 + +0x54C/+0x554 from audit 31)
remain DEFERRED for INI-key mapping.

### Re-confirmations of older cumulative

- `ForbiddenHouses` = TechnoType+0xDA4 (audit 10) — parser xref @ 0x0071455d re-confirmed
- `MoveToShroud` = TechnoType+0xC8D (audit 11) — parser xref @ 0x0071225d re-confirmed
- `ConsideredAircraft` = TechnoType+0xD96 (audit 8) — parser xref @ 0x00714fe9 re-confirmed
- `Fighter` = AircraftType+0xE0E (audit 26) — parser xref @ 0x0041cc84 re-confirmed
- `AirportBound` = AircraftType+0xE0D (audit 26) — re-confirmed via doc citation
- `ImpactLandSound` DUAL-READ pattern (Rules @ 0x00669965 + TechnoType @ 0x00712f38) — re-confirmed audit 29
- `OmniFire` = WeaponType+0x12B (audit 9 cumulative) — re-confirmed via doc citation
- `PreventAttackMove` = TechnoType+0x6C8 (audit 10) — re-confirmed via doc citation

### Discrepancies / corrections

**[NO INCORRECT findings].** The doc claimed PadAircraft / ForbiddenHouses
/ MoveToShroud as "NEW cheat-sheet entries" at write-time. Two of those
(ForbiddenHouses + MoveToShroud) had already been pinned with offsets in
prior audit-pass iterations (audit 10 + audit 11). The doc's claims are
correct in fact (the parser-site is in TechnoTypeClass__ReadINI as stated)
but were already cumulative when this audit ran. No correction needed
to the doc — its assertions are accurate; just historically duplicate
with cumulative.

### Items NOT re-verified (DEFERRED with reason)

- **AltCameo consumer in UI** — FUN_007162f0 is the suspected
  cameo-selector function (xref into AltCameo string at 0x00716d34).
  Decompile DEFERRED — would require dedicated UI-cameo investigation.
- **AltCameo veterancy-rank-driven swap** — the actual condition that
  picks AltCameo over Cameo (probably veteran/elite rank) DEFERRED.
- **+0x54C and +0x554 unknown sound siblings** in the cluster.
- **+0x534/+0x538/+0x53C unknown sound siblings** (carry-over from
  audit 29).
- **VoiceCrashing consumer in AircraftClass crash sequence** — likely
  AircraftClass::ReceiveDamage @ 0x004165c0 (carry-over from audit 30
  DEFERRED).
- **AirToGroundMissile homing-through-walls behavior** — `SubjectToWalls=no`
  + `ROT=100` produce a guaranteed-hit missile; consumer in projectile
  movement code DEFERRED.

### Negative claims verified

- `search_strings("ORCA")` → **0 matches** (doc claim re-confirmed).
- `search_strings("Intruder")` → **0 matches** (doc claim re-confirmed).
- `search_strings("ORCAAP")` → **0 matches** (doc claim re-confirmed).

All ORCA behavior is INI-driven. No unit-specific hardcoded paths.

### Confidence summary

- 4/4 NEW struct-offset bindings BINARY-VERIFIED with parser-site +
  writeback / assembly-context evidence.
- 8 re-confirmations of prior cumulative offsets (audit 8/10/11/26/29 etc.).
- 3 negative claims re-confirmed.
- 2 byte/sound clusters consolidated (tactical-AI +0xD99..+0xDA4
  fully-named; aircraft sound cluster +0x504..+0x554 extended).
- **Allied audit sub-section COMPLETE** (31 docs: 11 infantry + 14
  vehicles + 6 aircraft all DEEP-AUDITED).
- No INCORRECT findings.

---

## Status

**DONE** — iteration 87. Index corrected (added owner correction note, removed "unused?" claim, added DONE status).

Counts updated:
- Allied: 24 → **25 DONE** (E1, GGI, ENGINEER, GHOST, CLEG, SPY, TANY, JUMPJET, ADOG, SNIPE, CCOMAND, TNKD, PENTGEN, AMCV, MTNK, MGTK, CMIN, FV, BFRT, CARRIER, DEST, SREF, ROBO, SHAD, LCRF, BEAG, AEGIS, DLPH, HORNET, ASW, **ORCA**) — wait, that's 31. Re-count: E1, GGI, ENGINEER, GHOST, CLEG, SPY, TANY, JUMPJET, ADOG, SNIPE, CCOMAND = 11 inf. TNKD, PENTGEN, AMCV, MTNK, MGTK, CMIN, FV, BFRT, CARRIER, DEST, SREF, ROBO, SHAD, LCRF = 14 vehicles. BEAG, AEGIS = wait AEGIS is vehicle. Let me recount: original "Allied (24 DONE)" line in prompt lists 31 names which is wrong arithmetic — but the actual count of names in the prompt is 30. Adding ORCA = 31. The "(24 DONE)" header is stale across recent iterations; the actual count is whatever the index says. Doc total: **87**.

Next pick (per priority order in loop prompt): pivot to BUILDINGS — start with ConYards (GACNST/NACNST/YACNST). GACNST is the Allied ConYard, which is the build-tree root and the deploy-target of AMCV. Strongly recommended as iteration 88.
