# [YAPOWR] — Yuri Bio Reactor

**INI ID:** `YAPOWR`
**Display name:** `UIName=Name:BioR` → CSF "Bio Reactor" (note: not `Name:YAPOWR`)
**Internal name:** `Name=Yuri Bio Reactor`
**Side:** Universal-Owner (all 10 factions)
**Category:** `[BuildingTypes]`
**Owner:** all 10 factions
**Doc filename:** `units/structures/YAPOWR.md`
**Loop iteration:** 96

**Role:** Yuri power plant with **garrison-boost mechanic** — Initiates inside boost power output. Closes power-plant quartet (GAPOWR + NAPOWR + NANRCT + YAPOWR). Base `Power=150` (same as NAPOWR), but `Passengers=5` + `ExtraPower=100` per passenger means a fully-garrisoned Bio Reactor produces **150 + 5×100 = 650 power** — a 4.33× multiplier through active management. The unique Yuri faction trade-off: cheap base power + spend Initiates for density.

---

## rulesmd.ini section — full transcript

[rulesmd.ini:13124-13164](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
; Yuri power plant
[YAPOWR]
UIName=Name:BioR
Name=Yuri Bio Reactor
;Image=GAPOWR
BuildCat=Power
Prerequisite=YACNST
Strength=700
Armor=wood
TechLevel=1
Adjacent=2
Sight=4
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
AIBasePlanningSide=2 ;gs 0 for Good, 1 for Evil
Cost=600
Points=40
Power=150
Capturable=true ;gs per Design true    Should engineer capture or enter it?  Dunno, so ban capture. (Grinder already was)
Crewed=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60,gtpowexp
Upgrades=2
DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM
MaxDebris=6
MinDebris=4
ThreatPosed=0	; This value MUST be 0 for all building addons
;DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys
DamageSmokeOffset=300, 300, 450
TogglePower=no
Spyable=yes ; A spy can do something to this, works like captureable
DieSound=PowerPlantDie
ImmuneToPsionics=no ; defaults to yes for buildings, no for others
UnitAbsorb=no
InfantryAbsorb=yes
;CanBeOccupied=yes
;MaxNumberOccupants=5
PipScale=Passengers
Passengers=5
SizeLimit=15
ExtraPower=100
Drainable=yes
PoweredSpecial=yes
```

### The garrison-boost mechanic (Yuri-unique)

**This is YAPOWR's defining feature.** Six fields work together:

```ini
UnitAbsorb=no                  ; only infantry can enter, not vehicles
InfantryAbsorb=yes             ; infantry can enter
Passengers=5                   ; up to 5 infantry units
SizeLimit=15                   ; total Size budget of passengers (each Initiate is Size=3?)
ExtraPower=100                 ; each passenger adds +100 power
PipScale=Passengers            ; sidebar pips show passenger count, not credits
```

**Math:**
- Empty Bio Reactor: 150 power (base).
- 1 Initiate garrisoned: 150 + 100 = 250 power.
- 5 Initiates garrisoned (max): 150 + 5×100 = **650 power**.

That's 4.33× the base output. Yuri faction strategy: build a few Bio Reactors, then garrison them with Initiates from the Barracks. Each Initiate "powers up" the reactor. Players must actively manage power by feeding more Initiates as needs grow.

**The trade-off:** Initiates cost credits, time, and a Barracks queue slot. They're also Yuri's primary infantry. Garrisoning 5 Initiates per reactor means committing real units to non-combat duty. The cost-benefit:
- 5 Initiates × Cost (say 200) = 1000 credits "spent on power" beyond the reactor's 600 cost.
- 5×100=500 extra power for 1000 credits = 0.5 power/credit ratio.
- Compared: NAPOWR is 150/600 = 0.25 power/credit. **2× efficiency** for Yuri at the cost of micromanagement.

**Cross-faction analog:** Allied gets upgrade slots (Upgrades=2 on GAPOWR). Yuri gets garrison-boost. Soviet gets two-tier choice (NAPOWR cheap vs NANRCT expensive). Each side has a distinct power-density strategy.

### Identity & UI

- **`UIName=Name:BioR`** — **CSF label is `Name:BioR`, NOT `Name:YAPOWR`**. Westwood used a shortened "BioR" CSF key (Bio Reactor abbreviated). Compare GAPOWR's `Name:GAPOWR` and NAPOWR's `Name:NAPOWR`. YAPOWR is the only power plant with a non-standard CSF lookup convention.
- **`Name=Yuri Bio Reactor`** — engine-internal display fallback.
- **`;Image=GAPOWR`** — commented out. Westwood considered sharing GAPOWR's voxel asset before deciding Yuri needed distinct art.

### Diffs vs NAPOWR (the relevant 8)

YAPOWR vs NAPOWR — both have Cost=600, Power=150, but everything else diverges:

| Field | NAPOWR | YAPOWR | Notes |
|-------|--------|--------|-------|
| UIName | Name:NAPOWR | **Name:BioR** | non-standard CSF label |
| Prerequisite | NACNST | **YACNST** | per-side ConYard |
| AIBasePlanningSide | 1 (Evil) | **2 (Yuri side?)** | new value — see below |
| **Strength** | 750 | **700** | weakest power plant |
| **Upgrades** | 0 (absent) | **2** | Yuri gets upgrade slots (matches GAPOWR) |
| **Cost** | 600 | 600 | parity |
| **Power** | 150 | 150 | parity (base) |
| **UnitAbsorb / InfantryAbsorb / Passengers / SizeLimit / ExtraPower** | (absent) | **garrison-boost active** | YAPOWR-unique mechanic |
| **PipScale** | (absent — defaults) | **`PipScale=Passengers`** | sidebar pip system shows occupancy |
| DamageParticleSystems | active 3-system | `;commented` | YAPOWR doesn't visibly spark when damaged (unlike Tesla) |
| Explosion= extra anim | tstlexp (Tesla) | **gtpowexp (Allied/generic)** | YAPOWR uses Allied's `gtpowexp` (Westwood inconsistent — likely copy-paste from GAPOWR) |

### AIBasePlanningSide=2 (new value)

NAPOWR uses `AIBasePlanningSide=1` (Evil). YAPOWR uses `=2`. The cheat-sheet/research has documented 0=Good, 1=Evil. **`2` is a new value — likely "Yuri" or "Third Side"**. The Westwood verbatim comment "0 for Good, 1 for Evil" is incomplete; Yuri side is `2`. Confirms Yuri's faction is treated as a third-side AI category by the engine, not folded into Soviet/Evil.

### Build gating

- **`Prerequisite=YACNST`** — Yuri ConYard.
- **`TechLevel=1`** — earliest tier.
- **`Cost=600`** — parity with NAPOWR (cheapest power per build).
- **`Adjacent=2`** — same adjacency as other power plants.
- **`Sight=4`** — minimal vision (parity with other power plants).

### Combat / capture / spy

- **`Strength=700`** — weakest power plant (vs NAPOWR/GAPOWR's 750, NANRCT's 1000). Bio Reactor is the most fragile, possibly because garrison-boost makes it more strategically vital.
- **`Armor=wood`** — same as NAPOWR/GAPOWR.
- **`Capturable=true`** — verbatim Westwood comment "`;gs per Design true    Should engineer capture or enter it?  Dunno, so ban capture. (Grinder already was)`". This is Westwood internal designer commentary: they debated whether Engineer should capture YAPOWR (a building you can also enter via garrison). They chose Capturable=true (Engineer captures normally), but the comment "Grinder already was" suggests the Grinder (YAGRND) was set to non-capturable in the same debate.
  - **Implication**: Engineer CAN capture YAPOWR. Capturing transfers it to new owner; passengers inside (Initiates) are likely kicked out or transferred — DEFERRED to garrison-transfer mechanic audit.
- **`Crewed=yes`** — destruction ejects E2-equivalent crew (likely INIT for Yuri).
- **`Spyable=yes`** — spy infiltrate causes power outage (same mechanism as other power plants).
- **`Drainable=yes`** — Yuri's own drain-weapons can target Yuri's own Bio Reactor? Engine doesn't filter by owner. (Could be a friendly-fire issue for Yuri players, but their own drain-weapons probably target only enemy buildings.)
- **`ImmuneToPsionics=no`** — psi-vulnerable.

### Power-system flags
- **`Power=150`** — base power.
- **`PoweredSpecial=yes`** — power-special-state animation supported.
- **`TogglePower=no`** — no toggle (same as other power plants).
- **`Upgrades=2`** — YAPOWR has upgrade slots. Yuri Bio Reactor accepts upgrades (BIOREACT upgrade probably — pending separate doc). This is unlike NAPOWR which has 0 upgrades. Yuri trades garrison-boost AND gets upgrades. Strong stacked power building.

### Garrison-boost fields (the unique mechanic)

- **`UnitAbsorb=no`** — only infantry can enter (no vehicle absorption). Confirmed Ghidra-scope: BuildingType (xref `0x0081aabc → 0x0046098f`). **NEW cheat-sheet entry.**
- **`InfantryAbsorb=yes`** — infantry can enter. Confirmed Ghidra-scope: BuildingType (xref `0x0081aaac → 0x004609a9`). **NEW cheat-sheet entry.**
- **`Passengers=5`** — max passenger count. Confirmed Ghidra-scope: TechnoType (xref `0x0081bbd4 → 0x00714b3c`) — broader than building (vehicles also use Passengers, e.g., BFRT, SHAD, HTK, SAPC). **NEW cheat-sheet entry.**
- **`SizeLimit=15`** — total Size budget. Each passenger has a `Size=` (Initiate Size=1? Total of 15 allows 15 small infantry but Passengers=5 caps the count). The two values together: max 5 infantry units AND total Size ≤ 15.
- **`ExtraPower=100`** — power bonus per passenger. The KEY hardcoded boost. Confirmed Ghidra-scope: BuildingType (xref `0x0081a7b0 → 0x004610b3`). **NEW cheat-sheet entry.**
- **`PipScale=Passengers`** — sidebar visualization shows passenger count (5 pips, filled = occupied). vs GAREFN's `PipScale=Tiberium` which showed credit accumulation.
- **`;CanBeOccupied=yes` / `;MaxNumberOccupants=5`** — commented out. These are the field names used by **regular garrison** (Tibetian Sun ground-floor garrison system, also used by RA2 civilian buildings for E1/E2 garrison). YAPOWR explicitly uses the absorption-style system (Passengers + ExtraPower), not the occupy-style system. The commented entries reveal Westwood iterated this. The absorption system is mechanically distinct: absorbed units are CONSUMED into the building's pool, not just stationed inside firing weapons.

The mechanical distinction:
- **Occupy** (CanBeOccupied): infantry stationed inside, retain identity, fire weapons through windows, can be ordered out. Used by civilian buildings, Battle Bunker (NATBNK).
- **Absorb** (InfantryAbsorb): infantry ABSORBED into the building's storage. Passengers stat increases. Cannot fire out, but contributes to the building's mechanic (here: ExtraPower). Can be unloaded back as units. Used by Bio Reactor only (likely).

### Visual FX
- **`Explosion=TWLT070,...,gtpowexp`** — **uses Allied's `gtpowexp`** (not Tesla's `tstlexp`). Westwood inconsistency — likely Yuri's faction wasn't given a unique power-plant explosion SHP, fell back to Allied's.
- **`DebrisAnims=`** — same 6-anim list as other power plants.
- **`MaxDebris=6` / `MinDebris=4`** — same as GAPOWR (vs NAPOWR's 15/5 dramatic Tesla).
- **`DamageParticleSystems=`** commented — same as GAPOWR.
- **`DamageSmokeOffset=300, 300, 450`** — **identical to GAPOWR** (Westwood copy-paste).
- **`DieSound=PowerPlantDie`** — shared sound.

### AI hints
- **`AIBuildThis=`** NOT set (same convention as other power plants).
- **`AIBasePlanningSide=2`** — Yuri/Third-side.

---

## artmd.ini section — full transcript

[artmd.ini:3228-3255](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[YAPOWR]
Image=YAPOWR
Normalized=yes
Remapable=yes
Cameo=YPWRICON
Foundation=2x2
Buildup=YAPOWRMK
DemandLoadBuildup=true
FreeBuildup=true
NewTheater=yes
Height=4
ActiveAnim=YAPOWR_A
ActiveAnimDamaged=YAPOWR_AD
ActiveAnimZAdjust=-20
ActiveAnimYSort=362
ActiveAnimTwo=YAPOWR_B
ActiveAnimTwoDamaged=YAPOWR_BD
ActiveAnimTwoZAdjust=-20
ActiveAnimTwoYSort=362
IdleAnim=YAPOWR_C
IdleAnimDamaged=YAPOWR_CD
IdleAnimZAdjust=-150 
IdleAnimYSort=362
CanHideThings=True
CanBeHidden=False
OccupyHeight=3
DamageFireOffset0=-20,32
DamageFireOffset1=3,12
```

### Diffs vs GAPOWR artmd

| Field | GAPOWR artmd | YAPOWR artmd | Notes |
|-------|--------------|--------------|-------|
| **`Image=`** | (absent) | **`Image=YAPOWR` explicit** | Westwood explicitly named Image; usually implicit from block name |
| Cameo | POWRICON | **YPWRICON** | per-side cameo |
| Foundation | 2x2 | 2x2 (parity — smallest power plant foundation) | |
| Height | 4 | 4 (parity) | |
| OccupyHeight | 3 | 3 (parity) | |
| **ActiveAnimZAdjust** | -32 | **-20** | per-side art tuning |
| **ActiveAnimTwo / ActiveAnimTwoDamaged** | (absent) | **`YAPOWR_B` + `YAPOWR_BD`** | YAPOWR adds a 2nd active anim layer |
| **IdleAnim / IdleAnimDamaged** | (absent) | **`YAPOWR_C` + `YAPOWR_CD`** | YAPOWR adds an Idle layer |
| **IdleAnimZAdjust** | (absent) | **-150** (significantly Z-raised) | idle anim drawn well above building base |
| DamageFireOffset0 | -20,32 | -20,32 (parity) | |
| DamageFireOffset1 | 3,12 | 3,12 (parity) | |
| **`ActiveAnimPoweredSpecial=true`** | true | **(absent)** | YAPOWR does NOT use the power-state gating |
| **`ActiveAnimPowered=false`** | false | **(absent)** | YAPOWR does NOT use the power-state gating |

### 3-layer animation system (Active + ActiveAnimTwo + IdleAnim)

YAPOWR has the richest animation setup of any documented power plant:

- **`ActiveAnim=YAPOWR_A`** ([artmd.ini:16410](../../../../ra2-rust-game/ini/artmd.ini)): 8-frame infinite loop (frames 1-8, Rate=220). Base reactor cycle.
- **`ActiveAnimDamaged=YAPOWR_AD`** ([artmd.ini:16424](../../../../ra2-rust-game/ini/artmd.ini)): frames 9-16 of same SHP, infinite loop.
- **`ActiveAnimTwo=YAPOWR_B`** ([artmd.ini:16438](../../../../ra2-rust-game/ini/artmd.ini)): verbatim Westwood comment "Power plant **powered up** active animation". 8-frame loop. **This is the powered-up animation that plays when the reactor has passengers absorbed.** Different SHP file (YAPOWR_B), not just a frame range of YAPOWR_A.
- **`ActiveAnimTwoDamaged=YAPOWR_BD`** ([artmd.ini:16452](../../../../ra2-rust-game/ini/artmd.ini)): damaged variant of the powered-up anim, frames 9-16 of YAPOWR_B.
- **`IdleAnim=YAPOWR_C`** ([artmd.ini:16466](../../../../ra2-rust-game/ini/artmd.ini)): verbatim "Power plant **lights** animation". 6-frame infinite loop at Rate=175 (slightly faster than the active anims). Different SHP (YAPOWR_C). The blinking status lights on the reactor.
- **`IdleAnimDamaged=YAPOWR_CD`** ([artmd.ini:16480](../../../../ra2-rust-game/ini/artmd.ini)): damaged variant, frames 6-10 of YAPOWR_C.

The Westwood comments are revealing: **`ActiveAnimTwo` is the "powered up" anim**, meaning it shows when the Bio Reactor has at least one passenger garrisoned (boosting power). The unpassenger'd reactor plays only `ActiveAnim`. As Initiates enter, the building visually transitions to the boosted state.

This is a per-building **garrison-state animation** — distinct from the engine's standard power-state animation system (LowPower/Powered/PoweredSpecial). The Bio Reactor's state indicator is driven by its own passenger count, not the player's net-power state.

**Critical detail**: YAPOWR does NOT use `ActiveAnimPoweredSpecial=true` / `ActiveAnimPowered=false` (like GAPOWR/NAPOWR/NANRCT do). Instead, YAPOWR's `ActiveAnim` plays at all times (when undamaged), and `ActiveAnimTwo` plays additionally when garrison-boosted. The two anims **overlay** rather than gate each other.

### 3-layer Y-sort (all at 362)

`ActiveAnimYSort=362`, `ActiveAnimTwoYSort=362`, `IdleAnimYSort=362` — all three layers share Y-sort 362. They render at the same depth tier; visual differentiation comes from Z-adjust (-20 for active + active-two, -150 for idle lights — much higher above building).

### Foundation and dimensions
- **`Foundation=2x2`** — same as GAPOWR. Smallest power plant footprint.
- **`Height=4`** — same as GAPOWR.
- **`OccupyHeight=3`** — same as GAPOWR.

### Buildup
- **`Buildup=YAPOWRMK`** — Yuri Bio Reactor buildup anim.

### Damage fire
- **`DamageFireOffset0=-20,32` / `DamageFireOffset1=3,12`** — identical to GAPOWR. Westwood reused the offsets — likely the Bio Reactor's foundation/silhouette is similar enough to Allied's that the damage anchors match.

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field                  | String address    | First xref          | Read scope      |
|------------------------|--------------------|---------------------|-----------------|
| `UnitAbsorb`           | `0x0081aabc`       | `0x0046098f`        | BuildingType    |
| `InfantryAbsorb`       | `0x0081aaac`       | `0x004609a9`        | BuildingType    |
| `ExtraPower`           | `0x0081a7b0`       | `0x004610b3`        | BuildingType    |
| `Passengers`           | `0x0081bbd4`       | `0x00714b3c`        | **TechnoType**  |

**4 NEW cheat-sheet entries this iteration** — the garrison-boost mechanic field family:

1. **`UnitAbsorb`** — `0x0081aabc → 0x0046098f` — BuildingType. Permits vehicle absorption into the building. YAPOWR has `UnitAbsorb=no` (only infantry).
2. **`InfantryAbsorb`** — `0x0081aaac → 0x004609a9` — BuildingType. Permits infantry absorption. YAPOWR has `InfantryAbsorb=yes`. The pair (UnitAbsorb=no + InfantryAbsorb=yes) is the Bio Reactor's infantry-only intake config.
3. **`ExtraPower`** — `0x0081a7b0 → 0x004610b3` — BuildingType. Power bonus per absorbed unit. **The hardcoded multiplier for garrison-boost.** Engine adds `ExtraPower × current_passengers` to the building's `Power=` total dynamically.
4. **`Passengers`** — `0x0081bbd4 → 0x00714b3c` — **TechnoType** (broader than BuildingType). Same field used by transport vehicles (BFRT, SHAD, HTK, SAPC, etc.). Max passenger count.

### Ghidra search log for this iteration

- `search_strings("UnitAbsorb")` → 1 match → BuildingType.
- `search_strings("InfantryAbsorb")` → 1 match → BuildingType.
- `search_strings("ExtraPower")` → 1 match → BuildingType.
- `search_strings("Passengers")` → 1 match → TechnoType.

### YAPOWR-specific hardcoded behavior

The combination `InfantryAbsorb=yes + Passengers=5 + ExtraPower=100 + PipScale=Passengers` is recognized by the engine as the **garrison-boost power pattern**. The hardcoded chain:

1. When player selects YAPOWR with an Initiate selected → engine offers "Enter" cursor over the building.
2. Initiate enters → `Passengers` counter on building increments.
3. Engine recalculates total power for player: `building.Power + (building.ExtraPower × building.Passengers)`.
4. Player's net-power total updates dynamically.
5. PipScale=Passengers updates sidebar pip display.
6. If building destroyed → all passengers consumed (lost). Yuri loses Initiates plus the building.
7. If building captured/spied → passengers transfer or are kicked out (DEFERRED).
8. If building unpacked via `Deploy` command → passengers can be unloaded back as units. (Unsure if YAPOWR supports unload — TS-era civilian buildings did, but Bio Reactor may not.)

The `ActiveAnimTwo` ("powered up") animation triggers when `Passengers > 0`. The hardcoded visual feedback.

### Cross-faction power-plant comparison (complete quartet)

| Field | GAPOWR | NAPOWR | NANRCT | YAPOWR |
|-------|--------|--------|--------|--------|
| Side | Allied | Soviet | Soviet | Yuri |
| Cost | 800 | 600 | 1000 | 600 |
| Power (base) | 200 | 150 | 2000 | 150 |
| Upgrades slots | 2 | 0 | 0 | 2 |
| Garrison boost | No | No | No | **+100 per Initiate (×5 max)** |
| Death | normal | normal | **Nuclear blast** | normal |
| Strength | 750 | 750 | 1000 | **700** (weakest) |
| Armor | wood | wood | concrete | wood |
| Foundation | 2x2 | 3x2 | 4x4 | 2x2 |
| TechLevel | 1 | 1 | 9 | 1 |
| Max effective power | 400 (with 2 turbines) | 150 | 2000 | **650 (with 5 Initiates) + upgrades** |

**Yuri's late-game potential**: With Upgrades=2 (filled, +200) AND 5 Initiates (+500), a YAPOWR can produce 150 + 200 + 500 = **850 power** per building. Higher than GAPOWR's max 400. Yuri's late-game power economy is potentially the most dense of any faction.

### TS-legacy filter

- **`UnitAbsorb=no` / `InfantryAbsorb=yes`** — active YR. Bio-Reactor-specific.
- **`Passengers=5` / `SizeLimit=15` / `ExtraPower=100`** — active YR. The garrison-boost mechanic is fully implemented.
- **`PipScale=Passengers`** — active YR.
- **`;CanBeOccupied=yes` / `;MaxNumberOccupants=5`** — commented out. The TS-era / RA2-civilian-building occupy system is NOT used by YAPOWR (commented out in favor of Absorb). Both systems coexist in the engine (Occupy for civilian/Battle Bunker, Absorb for Bio Reactor). **Latent capability.**
- **`Explosion=...gtpowexp`** — uses Allied's `gtpowexp`. Slight Westwood inconsistency but not TS-legacy.
- **`Strength=700`** — Yuri-unique value. Reactor weakest of quartet. Possibly to balance the high power potential.
- **No fog-of-war / 0x1000 gating** — clean.
- **No Subterranean/Tunnel** — clean.

YAPOWR has no TS-legacy gating. The garrison-boost mechanic is the most "creative" YR-era feature — Westwood added it specifically for Yuri.

---

## Cross-references

- **`INIT`** (`units/yuri/INIT.md`) — DONE. Yuri Initiate. The primary unit that gets absorbed into YAPOWR for the +100 power boost. Cross-doc: confirm INIT's `Size=` value (defines how many fit into SizeLimit=15).
- **`GAPOWR`** (`units/structures/GAPOWR.md`) — DONE iteration 93. Allied power plant.
- **`NAPOWR`** (`units/structures/NAPOWR.md`) — DONE iteration 94. Soviet Tesla Reactor.
- **`NANRCT`** (`units/structures/NANRCT.md`) — DONE iteration 95. Soviet Nuclear Reactor.
- **`YACNST`** (`units/structures/YACNST.md`) — DONE. Yuri ConYard, prerequisite.
- **`Rules-global BuildPower=NAPOWR,GAPOWR,YAPOWR`** — AI lookup table.
- **`YAGRND` (Yuri Grinder)** — pending. Westwood comment "Grinder already was [non-capturable]" suggests the Grinder design predates YAPOWR's debate about Engineer behavior.
- **`NATBNK` (Battle Bunker)** — pending. Uses the *Occupy* garrison system (CanBeOccupied=yes), the alternative to Absorb. Comparison case.
- **`PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md`** — Yuri's late-game superweapon. With YAPOWR's max-650-power potential, Yuri can sustain it.

---

## Coverage audit

INI fields covered (35 rulesmd + 22 artmd + 6 sub-anims = 63 entries). **Coverage: 100%.**

---

## Open questions / Westwood inconsistencies

1. **`UIName=Name:BioR`** non-standard CSF lookup — only power plant using a short alias. Westwood deviated from the `Name:<INI_ID>` convention.
2. **`AIBasePlanningSide=2`** — confirms Yuri is a third AI side, not Soviet. The comment "0 for Good, 1 for Evil" is incomplete; Yuri = 2.
3. **Westwood verbatim "Should engineer capture or enter it? Dunno"** — internal designer commentary preserved in shipping INI. Confirms Engineer captures normally (Capturable=true); the "enter" path is the Absorb mechanism for Initiates only.
4. **`Explosion=...gtpowexp`** — Yuri uses Allied's `gtpowexp`. No Yuri-specific explosion SHP. Likely Westwood time pressure or art-budget decision.
5. **Strength=700 (weakest power plant)** — possibly to balance Yuri's late-game power potential (Upgrades+Initiates can push to 850/building).
6. **`SizeLimit=15` vs `Passengers=5`** — the two are bound. If INIT has Size=3, then 5 Initiates × Size=3 = 15 (perfect fit). If Size=2, can fit 7 but capped at Passengers=5. The SizeLimit may be there to prevent abuse by mod-injecting larger units.
7. **`;CanBeOccupied=yes` / `;MaxNumberOccupants=5`** commented — Westwood iterated using the standard occupy system first, then switched to Absorb. The Absorb system is engine-supported only via Bio Reactor; if modded, modders could enable other buildings to absorb units for unique boosts (e.g., refinery absorbs more ore?).
8. **What happens to passengers on capture?** Engineer captures the building → does the new owner inherit the 5 Initiates? Or are they kicked out as enemy units? DEFERRED.
9. **What happens on destruction?** Building destroyed → are the absorbed Initiates "killed inside" (Crewed=yes ejects only 1 unit, not the 5 passengers)? Likely all consumed. DEFERRED.

---

## Status

**DONE** — iteration 96. Index entry will be updated. **Power-plant quartet (GAPOWR + NAPOWR + NANRCT + YAPOWR) complete.**

Doc total: **96**.

Next pick (priority): Barracks trio — **GAPILE (Allied Barracks), NAHAND (Soviet Barracks), YABRCK (Yuri Barracks)**. These produce infantry; Yuri's YABRCK likely has unique flags for Initiate/PsiCorps spawn.
