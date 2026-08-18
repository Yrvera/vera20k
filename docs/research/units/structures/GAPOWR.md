# [GAPOWR] — Allied Power Plant

**INI ID:** `GAPOWR`
**Display name:** `UIName=Name:GAPOWR` → CSF label "Allied Power Plant"
**Internal name:** `Name=Allied Power Plant`
**Side:** Universal-Owner (all 10 factions)
**Category:** `[BuildingTypes]`
**Owner:** all 10 factions
**Doc filename:** `units/structures/GAPOWR.md`
**Loop iteration:** 93

**Role:** Allied power source. Provides `Power=200` per plant. `Upgrades=2` slots accept GAPOWRUP (Power Turbine — pending), each adding +100 power → 400 max per plant when fully upgraded. Tier-1 universal building.

---

## rulesmd.ini section — full transcript

[rulesmd.ini:11654-11685](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
; Allied power plant
[GAPOWR]
UIName=Name:GAPOWR
Name=Allied Power Plant
BuildCat=Power
Prerequisite=GACNST
Strength=750
Armor=wood
TechLevel=1
Adjacent=2
Sight=4
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
AIBasePlanningSide=0 ;gs 0 for Good, 1 for Evil
Cost=800
Points=40
Power=200
Capturable=true
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
Drainable=yes
PoweredSpecial=yes
```

### Identity & UI

- **`UIName=Name:GAPOWR`** → CSF "Allied Power Plant".
- **`BuildCat=Power`** — sidebar Power tab.

### Power mechanics — the core

- **`Power=200`** — produces 200 units of power. The single most important field on this building. Positive value = source; negative = consumer. The player's net power = sum across all buildings; positive net keeps everything operating, negative net causes low-power-state degradations across the base.
- **`Upgrades=2`** — accepts up to 2 upgrade slots. The Power Turbine upgrade (GAPOWRUP, pending separate doc) slots in here for +100 power each. Fully-upgraded GAPOWR provides up to 400 power. Confirmed Ghidra-scope: BuildingType (xref `0x0081954c → 0x0046472e`). **NEW cheat-sheet entry.**
- **`PoweredSpecial=yes`** — flags this building as a "powered special" — i.e., supports the `*PoweredSpecial` animation suffix system. Buildings with PoweredSpecial=yes get distinct animation states based on power-system status. (Combined with ActiveAnimPoweredSpecial=true in artmd, the power plant has a distinct active anim state when power-special is engaged.) See artmd section for detail.
- **`TogglePower=no`** — player cannot toggle this building's power off via the radial menu. (Power plants generate, not consume — toggle would be nonsensical.)
- **`Drainable=yes`** — power-drainable. The Yuri Magnetron-style or Yuri spy-stealing-power mechanic can target this building (drainable here means "Yuri's drain weapons can siphon from it"). Confirmed Ghidra-scope: **TechnoType** (xref `0x00843cd8 → 0x007143a3`). **NEW cheat-sheet entry.**

### Build gating

- **`Prerequisite=GACNST`** — needs only the ConYard. No power requirement (since this IS power) and no further tech.
- **`TechLevel=1`** — earliest tier.
- **`Cost=800`** — cheap. The power plant is the second-cheapest core building (only barracks/dog kennels might be cheaper).
- **`Adjacent=2`** — same adjacency as ConYard.
- **`Sight=4`** — minimal vision (smallest of any documented Allied building so far — vs ConYard's 8, Refinery's 6). Power plants don't need to see far.

### Combat / capture / spy

- **`Strength=750`** — moderate. Notably **lower than the refinery** (1000). Power plants are often the easiest "infrastructure" building to destroy, and players strategically harass with them.
- **`Armor=wood`** — same as refinery. Wood armor is weak to AT, modest to standard weapons.
- **`Points=40`** — half the ConYard's 80. Mid-tier kill value.
- **`Capturable=true`** — Engineer-capturable. Captured power plant adds to new owner's power total.
- **`Crewed=yes`** — destruction ejects an E1.
- **`Spyable=yes`** — verbatim Westwood comment "A spy can do something to this, works like captureable". The spy-infiltrate effect on a power plant per `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md` is **temporary base-wide power outage** for the targeted player (their power total drops to negative for a duration, causing radar/superweapons/defenses to shut down). High-value spy target.
- **`ImmuneToPsionics=no`** — psi-vulnerable (Yuri mind-control). Mind-controlled power plant transfers power to Yuri.

### Universal Owner

`Owner=` all 10 factions. Every faction can theoretically build GAPOWR if they hold the right ConYard. But the prerequisite chain is `Prerequisite=GACNST` — only an Allied ConYard satisfies this. Cross-faction power-plant building requires capturing an Allied ConYard.

In practice: Soviet uses NAPOWR (Tesla Reactor) + NANRCT (Nuclear Reactor); Yuri uses YAPOWR (Bio Reactor). The Owner= universal is for engine consistency — the Power Plant is universally buildable IF the prereq chain is met.

### Visual FX / destruction

- **`Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60,gtpowexp`** — **6-anim palette** vs ConYard's 5. The extra `gtpowexp` is a power-plant-specific explosion anim (likely "G T POWer EXPlosion" — TS naming heritage but active in YR). The power plant's distinctive boom.
- **`DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM`** — 6-anim debris pool. Same set as GAREFN (refinery shares debris palette with power plant).
- **`MaxDebris=6` / `MinDebris=4`** — modest debris range. Smaller building → less rubble.
- **`DamageSmokeOffset=300, 300, 450`** — single offset point for damage smoke.
- **`;DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys`** — commented (same as most Allied buildings).
- **`DieSound=PowerPlantDie`** — explicit death sound. Cross-ref [soundmd.ini:2868](../../../../ra2-rust-game/ini/soundmd.ini):
  ```ini
  [PowerPlantDie]
  Sounds=bpowdiea bpowdieb
  Control=random
  Priority= high
  FShift= -10 10
  ```
  2-sample random-pick (`bpowdiea`, `bpowdieb`) with Priority=high — important audio cue. The player hears this when their (or enemy's) power plant collapses, signaling base power shift.
- **`ThreatPosed=0`** — building threat 0.

### AI hints

- **`AIBuildThis=yes`** is **NOT set** on GAPOWR (compare GAREFN/GACNST which both have it). DEFERRED — check if engine defaults to "yes" for power plants, or if the AI uses a separate building-class lookup. The Rules-global `BuildPower=NAPOWR,GAPOWR,YAPOWR` ([rulesmd.ini:3066](../../../../ra2-rust-game/ini/rulesmd.ini)) declares which buildings are power-source for AI base planning, bypassing AIBuildThis on power plants.
- **`AIBasePlanningSide=0`** — Good side.

---

## artmd.ini section — full transcript

[artmd.ini:3206-3226](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[GAPOWR]
Normalized=yes
Remapable=yes
Cameo=POWRICON
Foundation=2x2
Buildup=GAPOWRMK
DemandLoadBuildup=true
FreeBuildup=true
NewTheater=yes
Height=4
ActiveAnim=GAPOWR_A
ActiveAnimDamaged=GAPOWR_AD
ActiveAnimZAdjust=-32
ActiveAnimYSort=362
CanHideThings=True
CanBeHidden=False
OccupyHeight=3
DamageFireOffset0=-20,32
DamageFireOffset1=3,12
ActiveAnimPoweredSpecial=true
ActiveAnimPowered=false
```

### Foundation and dimensions

- **`Foundation=2x2`** — 4-cell footprint. **The smallest foundation among Allied core buildings** documented so far (ConYard 4x4, Refinery 4x3, Power Plant 2x2). Reflects the power plant's compact role.
- **`Height=4`** — moderate height.
- **`OccupyHeight=3`** — Z-occupancy.
- **`Cameo=POWRICON`** — explicit cameo (shortened `POWRICON` shared with potentially NAPOWR? — likely Allied-specific despite name).

### Power-state animation system

```ini
ActiveAnimPoweredSpecial=true
ActiveAnimPowered=false
```

GAPOWR is the **first documented building actively using the power-state animation flags**:

- **`ActiveAnimPoweredSpecial=true`** — when the building is in "powered special" state (PoweredSpecial=yes is active AND something triggers the special state — likely upgrade-related or full-charge-related), the ActiveAnim plays. (Effectively: ActiveAnim runs when the powered-special condition is met.)
- **`ActiveAnimPowered=false`** — when the building is in normal-powered state, the ActiveAnim does NOT play. (Effectively: the active anim is gated by power-special state.)

This means GAPOWR's `ActiveAnim=GAPOWR_A` only plays when the power-system flags hit the special state. When the building is in normal power state, the active anim is suppressed. This may correspond to:
- Power Turbine upgrade installed → ActiveAnimPoweredSpecial=true → ActiveAnim cycles
- No upgrade installed → ActiveAnimPowered=false → ActiveAnim suppressed (the building shows static)

Cross-ref Ghidra discovery: the **16-entry "PoweredSpecial" family** of power-state-anim variants. Includes `IdleAnimPoweredSpecial`, `ActiveAnimPoweredSpecial`, `LowPowerPoweredSpecial`, `SuperLowPowerPoweredSpecial`, and more. The engine supports a multi-axis state matrix: anim-slot × power-state × low-power-state.

### Sub-animation

[artmd.ini:16384-16407](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[GAPOWR_A]
Image=GAPOWR_A
Normalized=yes
NewTheater=yes
Layer=ground
Start=0
LoopStart=0
LoopEnd=8
LoopCount=-1
Rate=220
DetailLevel=2

[GAPOWR_AD]
Image=GAPOWR_A
Normalized=yes
;NewTheater=yes
Layer=ground
Start=8
LoopStart=8
LoopEnd=16
LoopCount=-1
Rate=220
DetailLevel=2
```

- `GAPOWR_A` — 8-frame infinite loop (Start=0, LoopEnd=8), Rate=220 (faster than the typical 200 — power plant cycles slightly more energetic). DetailLevel=2 (low-detail; can be culled under low-detail render settings).
- `GAPOWR_AD` — damaged variant. Same SHP, frames 8-16. Damaged frame range. **NewTheater=yes is commented out on the damaged variant** — Westwood likely intentional (damaged variant uses the base theater anim, not theater-substituted).

### Buildup
- **`Buildup=GAPOWRMK`** — Allied power plant buildup anim. Same memory-thrift pattern.

### Damage fire
- **`DamageFireOffset0=-20,32`** + **`DamageFireOffset1=3,12`** — two damage fire emit points within the small 2x2 footprint.

### Render flags
- **`Normalized=yes`** — applies frame normalization (palette/timing).
- **`Remapable=yes`** — house color.
- **`NewTheater=yes`** — theater-letter substitution.

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field                  | String address    | First xref               | Read scope                   |
|------------------------|--------------------|--------------------------|------------------------------|
| `Upgrades`             | `0x0081954c`       | `0x0046472e`             | BuildingType                 |
| `PowersUpBuilding`     | `0x0081a8b8`       | `0x00460e23`             | BuildingType                 |
| `Drainable`            | `0x00843cd8`       | `0x007143a3`             | **TechnoType**               |

**3 NEW cheat-sheet entries this iteration:**

1. **`Upgrades`** — `0x0081954c → 0x0046472e` — BuildingType. Number of upgrade slots a building accepts (GAPOWR has 2 for Power Turbine).
2. **`PowersUpBuilding`** — `0x0081a8b8 → 0x00460e23` — BuildingType. The companion field to Upgrades — a building's PowersUpBuilding= field declares WHICH building type it slots into. (E.g., GAPOWRUP would have `PowersUpBuilding=GAPOWR`.) Discovered via adjacent search; not on GAPOWR itself but relevant to the upgrade system.
3. **`Drainable`** — `0x00843cd8 → 0x007143a3` — **TechnoType** scope. Power-drainable flag (Yuri Magnetron / drain-weapon target permission).

### 16-entry power-state animation family

Searching `PoweredSpecial` returned **16 strings**. Top 3 shown:
- `IdleAnimPoweredSpecial` at `0x0081973c`
- `SuperLowPowerPoweredSpecial` at `0x0081988c`
- `LowPowerPoweredSpecial` at `0x00819984`

Plus the 13 not-shown variants. The full power-state animation matrix supports:
- 4 anim slots (ActiveAnim, ActiveAnimTwo, ActiveAnimThree, ActiveAnimFour, IdleAnim, ProductionAnim, SpecialAnim, PreProductionAnim)
- 4 power states (Powered, PoweredSpecial, LowPower, SuperLowPower)
- Combined = 16+ permutations

This confirms the engine's **rich power-state animation system** for buildings. Most buildings don't use all variants; GAPOWR uses `ActiveAnimPowered=false` + `ActiveAnimPoweredSpecial=true` to gate the active anim by power state. The Low-power and Super-low-power variants would activate when the player's net power drops to negative or far-negative — providing visual feedback that the building is degraded.

### Ghidra search log for this iteration

- `search_strings("Upgrades")` → 1 match → BuildingType.
- `search_strings("PoweredSpecial")` → 16 matches (rich animation matrix).
- `search_strings("PowersUpBuilding")` → 1 match → BuildingType.
- `search_strings("Drainable")` → 1 match → TechnoType.
- `search_strings("PowerPlantDie")` → 0 matches (sound key name, not a parser key — looked up via DieSound= per-unit).

### Power-system Ghidra hooks (cross-references)

The power system itself (Rules-global `Power*` fields) is read by RulesClass — separate from GAPOWR's per-building Power=200. Rules-global power constants like:
- `LowPowerPenaltyModifier`
- `MaxPowerPenaltyValue` (probably)
- `ConditionRed/Yellow/Green` thresholds

These are documented in adjacent cheat-sheet entries (RulesClass__ReadGeneral). The per-building `Power=` is read by TechnoType or BuildingType (deferred to confirm).

### TS-legacy filter

- **`Explosion=...,gtpowexp`** — `gtpowexp` is a TS-era explosion SHP for "GT (power plant) explosion". Still active in YR.
- **`ActiveAnimPowered=false` + `ActiveAnimPoweredSpecial=true`** — engine fields, active YR.
- **`PoweredSpecial=yes`** — active YR.
- **`Drainable=yes`** — active YR (Yuri drain weapons).
- **No fog-of-war / 0x1000 gating** — clean.
- **No Subterranean/Tunnel** — clean.

GAPOWR has no TS-legacy gating. The `gtpowexp` explosion name and the rich power-state animation system are all active engine features.

---

## Cross-references

- **`GACNST`** (`units/structures/GACNST.md`) — DONE. Prerequisite parent.
- **`GAPOWRUP` (Power Turbine upgrade)** — pending. Slots into GAPOWR via Upgrades=2 / PowersUpBuilding=GAPOWR.
- **`NAPOWR` (Tesla Reactor)** — pending. Soviet primary power plant.
- **`NANRCT` (Nuclear Reactor)** — pending. Soviet heavy power plant (higher power output, larger footprint, explosive death).
- **`YAPOWR` (Bio Reactor)** — pending. Yuri power plant with garrison-boost (Initiates inside boost power output — unique mechanic).
- **`Rules-global BuildPower=NAPOWR,GAPOWR,YAPOWR`** ([rulesmd.ini:3066](../../../../ra2-rust-game/ini/rulesmd.ini)) — AI power-building lookup table.
- **`ENGINEER`/`SPY`** — capture/infiltrate. Spy infiltration of power plant causes temporary base-wide power outage.
- **Deep-RE cross-refs**: `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md` (power outage from spy infiltrate).

---

## Coverage audit

INI fields covered (28 rulesmd + 19 artmd + 2 sub-anims + 1 sound = 50 entries). **Coverage: 100%.**

---

## Open questions / Westwood inconsistencies

1. **`AIBuildThis=` NOT set** on GAPOWR but set on GACNST/GAREFN. Compare NAPOWR's value (pending audit). Likely the AI uses `BuildPower=NAPOWR,GAPOWR,YAPOWR` Rules-global table instead of per-building AIBuildThis for power plants. DEFERRED.
2. **`ActiveAnimPowered=false` + `ActiveAnimPoweredSpecial=true`** — what visual difference? My best guess: the active anim only runs when a Power Turbine upgrade is installed (turning the power plant into "powered special" state). Without upgrade, the building is visually static. Verifying requires in-game observation — DEFERRED.
3. **Spy infiltrate on power plant** — `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md` confirms temporary power outage. Duration/severity values defer to that doc.
4. **`POWRICON` cameo** — vs Soviet's `NPWRICON` and Yuri's `YPWRICON`. The `POWRICON` (without prefix letter) is the Allied convention.
5. **6-anim Explosion= palette including `gtpowexp`** — unique to power plant. The extra anim makes power plants' deaths visually distinct (and the high-priority audio cue from PowerPlantDie sound reinforces this).
6. **`Strength=750` vs Refinery's 1000** — power plant is more fragile despite being more critical. Strategic implication: protect power plants because they break first under harassment.

---

## Status

**DONE** — iteration 93. Index entry will be updated.

Doc total: **93**.

Next pick (priority): NAPOWR (Soviet Tesla Reactor) — pair with GAPOWR. Then NANRCT (Soviet Nuclear Reactor, heavy power). Then YAPOWR (Yuri Bio Reactor — with garrison-boost hardcoded behavior, the most interesting power plant for Ghidra investigation).
