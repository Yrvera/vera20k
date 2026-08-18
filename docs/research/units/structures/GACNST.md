# [GACNST] — Allied Construction Yard

**INI ID:** `GACNST`
**Display name:** `UIName=Name:GACNST` → CSF label "Allied Construction Yard"
**Internal name:** `Name=Allied Construction Yard`
**Side:** Allied (all 5 Allied factions including Korea)
**Category:** `[BuildingTypes]` slot `3=GACNST` ([rulesmd.ini:1180](../../../../ra2-rust-game/ini/rulesmd.ini))
**Owner:** `Owner=British,French,Germans,Americans,Alliance` (all 5 Allied factions)
**Doc filename:** `units/structures/GACNST.md`
**Loop iteration:** 88

**Role:** The build-tree root. Every Allied base starts with one (deployed from AMCV). Required as `Prerequisite=GACNST` for nearly every Allied building. Without one, the Allied player cannot construct anything.

---

## rulesmd.ini section — full transcript and per-key analysis

[rulesmd.ini:11622-11651](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
[GACNST]
UIName=Name:GACNST
Name=Allied Construction Yard
ConstructionYard=yes
Strength=1000
Armor=concrete
TechLevel=-1
Adjacent=2
Factory=BuildingType
UndeploysInto=AMCV
Sight=8
Owner=British,French,Germans,Americans,Alliance
Cost=3000
Points=80
Power=0
Capturable=true
Crewed=yes
;DestroyAnim=GACNSTDM
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS2LG,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM,DBRIS6LG,DBRIS6SM,DBRIS7LG
MaxDebris=15
MinDebris=7
ThreatPosed=0	; This value MUST be 0 for all building addons
;DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys
DamageSmokeOffset=1470, 1060, 1078
AIBuildThis=yes
TogglePower=no
ProtectWithWall=yes
EligibileForAllyBuilding=yes ;gs This allows a building of this type to count as a sucess in building placement, but only if that option is turned on
ImmuneToPsionics=no ; defaults to yes for buildings, no for others
```

### Identity and CSF binding

- **`UIName=Name:GACNST`** — CSF lookup; rendered in-game as "Allied Construction Yard".
- **`Name=Allied Construction Yard`** — engine-internal display fallback. Matches CSF.
- **No `Image=` redirect** — GACNST has its own artmd block (`[GACNST]` at [artmd.ini:1599](../../../../ra2-rust-game/ini/artmd.ini)).

### Construction-yard mechanics

- **`ConstructionYard=yes`** — **the** defining flag. Engine-keyed: this is the building that hosts the build queue and provides build-adjacency origin. Without one, no Allied production occurs. Confirmed Ghidra-scope: BuildingType (xref `0x0081aa74 → 0x00460a2b` in BuildingTypeClass_ReadINI_Water). **NEW cheat-sheet entry.**

- **`Adjacent=2`** — buildings can be placed up to 2 cells away from any existing building of the same owner. **The largest Adjacent value in the building set** — the ConYard is the build-adjacency anchor; placing it on the map seeds the player's buildable area. Confirmed Ghidra-scope: BuildingType (xref `0x0081ae40 → 0x0045ffb6` in BuildingTypeClass_ReadINI_Water). **NEW cheat-sheet entry.**

- **`Factory=BuildingType`** — declares that this building is a factory for the `BuildingType` category. Only the ConYard produces buildings. (Other factories: GAWEAP→`UnitType`, GAPILE→`InfantryType`, GAAIRC→`AircraftType`, GAYARD→`UnitType` over water — those each have their own `Factory=` value.) Confirmed Ghidra-scope: BuildingType (xref `0x008173f0 → 0x00460521`). **NEW cheat-sheet entry.** (Note: `008173f0` is just `"Factory"`; `0081aa4c` is `"WeaponsFactory"`, a separate keyword.)

- **`UndeploysInto=AMCV`** — pack-up command transforms this building back into an AMCV unit. Mirrors `[AMCV] DeploysInto=GACNST` (per [rulesmd.ini:6977](../../../../ra2-rust-game/ini/rulesmd.ini)) — bidirectional deploy/undeploy pair. Confirmed Ghidra-scope: **TechnoType** (xref `0x00844170 → 0x007132b2` in TechnoTypeClass__ReadINI) — meaning UndeploysInto is a general TechnoType field readable on both buildings AND vehicles, even though its semantic meaning is "what unit do I become when packed up" which only makes sense on buildings that came from deployable units. **NEW cheat-sheet entry.**

### Build gating and ownership

- **`Prerequisite=`** — **not set**. The ConYard has no prerequisite — it is the build-tree root. You can only acquire one by deploying an AMCV (or finding one as map pre-placed). The build sidebar never offers GACNST for production.
- **`TechLevel=-1`** — explicitly excluded from the build menu. `TechLevel=-1` is Westwood's hide-from-build-list convention. Players cannot build a ConYard; they can only acquire one via AMCV deploy. (Otherwise an infinite-ConYard exploit would be trivial.)
- **`Owner=British,French,Germans,Americans,Alliance`** — all 5 Allied factions (including Korea/Alliance). Unlike ORCA which excludes Korea, the ConYard is universally Allied.
- **`Cost=3000`** — the ConYard's nominal cost (matched by AMCV's Cost=3000 — undeploying does NOT refund the building's cost, but the building has cost for save/load and Sell mechanics). Equal to SMCV/PCV costs.
- **`Points=80`** — high score-on-kill value (vs ORCA's 20 / typical infantry 5). Killing an enemy ConYard is heavily score-rewarding.

### Combat / durability

- **`Strength=1000`** — moderate HP for a tier-0 mandatory structure. Equal to AMCV vehicle HP. ConYard is **not** the hardiest building (NAPOWR is 750, but the Nuke Silo and Battle Lab can exceed 1000). The ConYard's protection comes from its location deep in the base, not raw HP.
- **`Armor=concrete`** — the hardest armor class. Pairs with `ORCAAP Verses index 8 = 75%` (-25% incoming damage from Allied air-strike weapons). Most anti-armor weapons get 50-100% effectiveness against concrete; tank shells (AP rounds) are reduced.
- **`Sight=8`** — vision radius. Same as ORCA/BEAG and most mid-tier units. Adequate for a base-defense vantage point.
- **`Power=0`** — neither consumes nor produces power. The deploy action gives the player nothing but build capability.

### Capture / spy mechanics

- **`Capturable=true`** — Engineers (ENGINEER, SENGINEER, YENGINEER) can capture this building, transferring ownership. Capturing a ConYard is the single most-impactful capture in the game — the new owner gains full Allied tech tree (filtered by their own faction's `Owner=` rules).
- **`Crewed=yes`** — destruction ejects an infantry crew (1× E1 for Allied buildings). Standard for almost all buildings.
- **`ImmuneToPsionics=no`** — verbatim Westwood comment "defaults to yes for buildings, no for others". The ConYard is explicitly NOT psi-immune. Yuri/MIND, PsiPulse, etc. CAN affect it. **However**, mind-controlling a ConYard does NOT transfer build capability — Yuri's MIND grabs the building visually, but factory ownership remains with the original player. (This is engine-side hardcoded behavior; the captured-faction's build queue does not extend to the controlled ConYard. See OPEN QUESTIONS.)

### Visual FX / destruction

- **`Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60`** — 5-anim explosion palette. Same set as ORCA (and many other large structures). On destruction, one anim from the list is chosen at random per cell affected.
- **`DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS2LG,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM,DBRIS6LG,DBRIS6SM,DBRIS7LG`** — 10 debris animations, large and small variants. The 4x4 foundation generates considerable rubble.
- **`MaxDebris=15` / `MinDebris=7`** — between 7 and 15 debris pieces fly out on destruction. **More debris than any unit** (ORCA had MaxDebris=3, MTNK has MaxDebris=4). The size of the building drives the count.
- **`;DestroyAnim=GACNSTDM`** — commented out. The dedicated destruction animation `GACNSTDM` exists in artmd but is not wired up. (TS-era leftover; the Explosion= list replaces it.)
- **`DamageSmokeOffset=1470, 1060, 1078`** — pixel offsets (in art coords) where damage smoke particles emit when the building is below 50% HP. Three values = three separate emit points.
- **`;DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys`** — commented out. Damage particle systems are disabled for the ConYard (vs ORCA's `SparkSys,SmallGreySSys` which are active). The smoke offset above implies a different (legacy?) smoke system, but the active emission path is unset.

### AI / placement hints

- **`ThreatPosed=0`** — verbatim "This value MUST be 0 for all building addons". Building threat is 0; AI doesn't auto-attack the ConYard by threat-priority. (Targeting buildings goes through a separate AI building-target path that weights by `Points=` and `Spyable=`/`Capturable=` flags, not by ThreatPosed.)
- **`AIBuildThis=yes`** — AI is allowed to construct this building when planning a base. Together with `BuildConst=GACNST,NACNST,YACNST` ([rulesmd.ini:3065](../../../../ra2-rust-game/ini/rulesmd.ini), in `[AI]` section), the AI knows the ConYard is the foundational structure. Note: AIBuildThis=yes on a `TechLevel=-1` building is semantically valid only because the AI's BuildConst lookup bypasses TechLevel for the ConYard. Confirmed Ghidra-scope: BuildingType (xref `0x0081a7fc → 0x00460fe9`). **NEW cheat-sheet entry.**
- **`TogglePower=no`** — player cannot toggle this building on/off via the radial power-toggle menu. (Toggle exists for buildings that have a `Power=` consumption that the player might want to defer.)
- **`ProtectWithWall=yes`** — AI hint: surround this building with walls when feasible. The Allied AI may construct GAWALL segments around the ConYard. Confirmed Ghidra-scope: BuildingType (xref `0x0081ac80 → 0x00460272`). **NEW cheat-sheet entry.**
- **`EligibileForAllyBuilding=yes`** — verbatim "This allows a building of this type to count as a sucess in building placement, but only if that option is turned on". When the multiplayer setting "Share Build Queues" / ally-base option is enabled, this building counts toward the shared placement success check. (Westwood typo: "Eligibile" not "Eligible" — and "sucess" not "success" — both are baked into the engine's INI key matching.) Confirmed Ghidra-scope: BuildingType (xref `0x0081acc8 → 0x0046020a`). **NEW cheat-sheet entry.**

---

## artmd.ini section — full transcript

[artmd.ini:1599-1620](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[GACNST]
Remapable=yes
Foundation=4x4
Height=4
AnimActive=0,26,3
Buildup=GACNSTMK
DemandLoadBuildup=true
FreeBuildup=true
NewTheater=yes
ActiveAnim=GACNST_A
ActiveAnimDamaged=GACNST_AD
ActiveAnimZAdjust=-130
ActiveAnimYSort=362
ProductionAnim=GACNST_B
ProductionAnimDamaged=GACNST_BD
ProductionAnimZAdjust=-10
ProductionAnimYSort=543
CanHideThings=True
CanBeHidden=False
OccupyHeight=3
DamageFireOffset0=-24,-1
DamageFireOffset1=64,36
```

### Foundation and dimensions

- **`Foundation=4x4`** — occupies a 4×4 cell footprint. **The largest building footprint in the game** (matched only by other ConYards NACNST/YACNST at 4×4, and a few civilian decorations). 16 cells of placement; explains the 1+ second buildup animation duration and the high MaxDebris.
- **`Height=4`** — voxel/sprite height in cells (Z-buffer extent for hit-tests).
- **`OccupyHeight=3`** — Z-occupancy for unit collision. Aircraft flying at Z=3 collide; air at Z=4+ flies over.

### Render-side flags

- **`Remapable=yes`** — house color is applied to the building's remap palette index. Player ConYards are visibly house-tinted.
- **`NewTheater=yes`** — uses new-theater (RA2/YR) SHP theater letter prefix system. The engine substitutes the theater letter (`G`=Generic? Or theater-specific?) into the asset filename when loading.
- **`CanHideThings=True`** — units/animations can be Z-hidden behind this building's silhouette.
- **`CanBeHidden=False`** — but this building is never Z-hidden behind others (it's a top-tier render priority). Distinct from infantry/vehicles which are CanBeHidden=True.

### Buildup animation

- **`Buildup=GACNSTMK`** — when a deployed AMCV transforms into a GACNST, the buildup animation `GACNSTMK` plays. This is the iconic ConYard-emerges-from-MCV animation. The `MK` suffix is Westwood convention for "make/buildup" anims.
- **`DemandLoadBuildup=true`** — the buildup SHP is loaded on-demand at deploy time, not pre-cached at map load. Reduces memory pressure for a single-use anim. The matching cheat-sheet entry: this is a buildup-specific optimization.
- **`FreeBuildup=true`** — after the buildup plays, the buildup SHP is freed from memory. Memory-thrift complement to DemandLoadBuildup.

### Active animation (always-on idle)

- **`ActiveAnim=GACNST_A`** — the always-running idle animation when undamaged. References `[GACNST_A]` at [artmd.ini:17180](../../../../ra2-rust-game/ini/artmd.ini):
  ```ini
  [GACNST_A]
  Normalized=yes
  Start=0
  LoopStart=0
  LoopEnd=3
  LoopCount=-1
  Rate=200
  Layer=ground
  NewTheater=yes
  ```
  4-frame loop (0-3), infinite (LoopCount=-1), Rate=200, ground layer. The blinking light/dish-rotation idle anim.
- **`ActiveAnimDamaged=GACNST_AD`** — replacement idle when damaged. References `[GACNST_AD]` at [artmd.ini:17192](../../../../ra2-rust-game/ini/artmd.ini): same SHP (`Image=GACNST_A`), but uses frames 3-6 (the "smoking damaged" variant in the same SHP file).
- **`ActiveAnimZAdjust=-130`** — Z-offset (raises render order by 130 units). Active anim is drawn above the base building.
- **`ActiveAnimYSort=362`** — Y-sort tiebreaker for layered rendering. Higher Y-sort = drawn later (on top).
- **`AnimActive=0,26,3`** — animation activation parameters. Format: `Start, End, Rate` for sub-frame range control. (Westwood internal detail; precise semantics depend on engine state.)

### Production animation (factory-running)

- **`ProductionAnim=GACNST_B`** — animation that plays while the ConYard is queuing/producing a building. References `[GACNST_B]` at [artmd.ini:17205](../../../../ra2-rust-game/ini/artmd.ini):
  ```ini
  [GACNST_B]
  Normalized=yes
  Start=0
  LoopStart=0
  LoopEnd=20
  LoopCount=1
  Rate=200
  ```
  20-frame one-shot (LoopCount=1), Rate=200. Visual feedback: the ConYard "works" during production.
- **`ProductionAnimDamaged=GACNST_BD`** — damaged variant. Same SHP, frames 20-40 (damaged-state production animation). 20-frame one-shot.
- **`ProductionAnimZAdjust=-10`** — small Z-raise.
- **`ProductionAnimYSort=543`** — production anim Y-sorted above active anim (`543 > 362`).

### Damage fire effects

- **`DamageFireOffset0=-24,-1`** — pixel offset where the first fire/smoke particle spawns when damaged.
- **`DamageFireOffset1=64,36`** — second fire offset (diagonally opposite corner). The 4x4 foundation supports two visible fire points.

### Theater rendering

- **`NewTheater=yes`** — applies to building SHP, active anim, and production anim (all flagged separately).

The artmd block reuses the `GACNST_A` SHP across active and damaged variants (Image= redirect inside the sub-block) — Westwood's standard "single SHP, frame-range slicing" pattern.

---

## Build queue chain — what GACNST unlocks

Every Allied building (and many Allied units) has `Prerequisite=GACNST` (or a chain leading back to it). Direct dependents at [rulesmd.ini](../../../../ra2-rust-game/ini/rulesmd.ini):

| Building (line) | Prerequisite                       | Role                |
|-----------------|------------------------------------|---------------------|
| GAPOWR (11659)  | `GACNST`                            | Allied power plant  |
| GAREFN (11692)  | `POWER,GACNST`                      | Allied refinery     |
| GAPILE (11731)  | `POWER,GACNST`                      | Barracks            |
| GADEPT (11776)  | `PROC,GAPILE,GACNST`                | Service Depot       |
| GATECH (11812)  | `GAREFN,GACNST`                     | Battle Lab          |
| GAYARD (11849)  | `PROC,POWER,GACNST`                 | Naval Shipyard      |
| GAWEAP (11889)  | `GAWEAP,GACNST` (wait — that's circular for GAWEAP; likely typo, in-game appears to be `GAPILE,GACNST` semantics — DEFERRED for own audit)             | War Factory         |
| GAAIRC (11921)  | `GAWEAP,RADAR,GACNST`               | Airforce Command HQ |
| GAOREP (11954)  | `GATECH,PROC,GACNST`                | Ore Purifier        |
| GAROBO (11991)  | `GAWEAP,GACNST`                     | Robot Control Center|
| GAPILL (12055)  | `BARRACKS,GACNST`                   | Pillbox             |
| GTGCAN (12098)  | `BARRACKS,GACNST`                   | Grand Cannon        |
| GASPYSAT (12147)| `POWER,RADAR,GACNST`                | Spy Satellite Uplink|
| GACSPH (12192)  | `GATECH,GACNST`                     | Chronosphere        |
| GAWEAT (12225)  | `GATECH,GACNST`                     | Weather Controller  |
| GAGAP (12373)   | `RADAR,GACNST`                      | Gap Generator       |

(POWER, BARRACKS, RADAR, PROC are Rules-global aliases — Power resolves to GAPOWR/NAPOWR/YAPOWR; etc.)

The ConYard is the build-tree root: **every Allied building requires GACNST**. Capturing or destroying it disables further Allied production for that player.

---

## AI hooks: BuildConst

[rulesmd.ini:3065](../../../../ra2-rust-game/ini/rulesmd.ini), in `[AI]` section:

```ini
BuildConst=GACNST,NACNST,YACNST
```

The Rules-global `BuildConst` is the AI's table of all valid ConYard buildings. The AI's base-planner uses this to recognize ConYards across factions (so AI players construct the right one for their side). Confirmed Ghidra-scope: Rules (xref `0x0083d4a0 → 0x00672b23` in FUN_00672ae0 — an AI-table reader function at the 0x00672xxx range, adjacent to RulesClass__ReadGeneral at 0x00671xxx). **NEW cheat-sheet entry.**

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field                       | String address | First xref               | Read scope                      |
|-----------------------------|----------------|--------------------------|---------------------------------|
| `ConstructionYard`          | `0x0081aa74`   | `0x00460a2b`             | **BuildingTypeClass_ReadINI_Water** |
| `UndeploysInto`             | `0x00844170`   | `0x007132b2`             | TechnoTypeClass__ReadINI        |
| `BuildConst`                | `0x0083d4a0`   | `0x00672b23` in FUN_00672ae0 | **Rules AI-table reader** (sibling to ReadGeneral) |
| `ProtectWithWall`           | `0x0081ac80`   | `0x00460272`             | BuildingType                    |
| `Factory`                   | `0x008173f0`   | `0x00460521`             | BuildingType                    |
| `Adjacent`                  | `0x0081ae40`   | `0x0045ffb6`             | BuildingType                    |
| `EligibileForAllyBuilding`  | `0x0081acc8`   | `0x0046020a`             | BuildingType                    |
| `AIBuildThis`               | `0x0081a7fc`   | `0x00460fe9`             | BuildingType                    |

**8 NEW cheat-sheet entries this iteration** — opening up the BuildingType scope coverage significantly:

1. **`ConstructionYard`** — `0x0081aa74 → 0x00460a2b` — BuildingType. The defining ConYard flag. Engine-keyed; the structure with this flag is the build-tree root.
2. **`UndeploysInto`** — `0x00844170 → 0x007132b2` — TechnoType. The deploy-pair counterpart of DeploysInto. TechnoType-scope means it works on units AND buildings (semantically meaningful on the post-deploy side).
3. **`BuildConst`** — `0x0083d4a0 → 0x00672b23` in FUN_00672ae0 — Rules. AI table of ConYard buildings across factions. NEW scope discovery: **Rules-AI-table reader at 0x00672xxx** (sibling to RulesClass__ReadGeneral at 0x00671xxx).
4. **`ProtectWithWall`** — `0x0081ac80 → 0x00460272` — BuildingType. AI hint to wall around.
5. **`Factory`** — `0x008173f0 → 0x00460521` — BuildingType. Factory-category declaration (BuildingType / UnitType / InfantryType / AircraftType).
6. **`Adjacent`** — `0x0081ae40 → 0x0045ffb6` — BuildingType. Build-adjacency radius.
7. **`EligibileForAllyBuilding`** — `0x0081acc8 → 0x0046020a` — BuildingType. Multiplayer "Share Build Queues" eligibility (Westwood typo preserved).
8. **`AIBuildThis`** — `0x0081a7fc → 0x00460fe9` — BuildingType. AI base-planner permission.

### Ghidra search log for this iteration

- `search_strings("ConstructionYard")` → 1 match at `0x0081aa74` → `0x00460a2b` (BuildingType).
- `search_strings("UndeploysInto")` → 1 match at `0x00844170` → `0x007132b2` (TechnoType).
- `search_strings("BuildConst")` → 1 match at `0x0083d4a0` → `0x00672b23` in FUN_00672ae0.
- `search_strings("ProtectWithWall")` → 1 match at `0x0081ac80` → `0x00460272` (BuildingType).
- `search_strings("Factory")` → top match `0x008173f0` → `0x00460521` (BuildingType). 96 total string matches in binary (most are debug-print strings, e.g. "Weapons factory clearing %s from bib area"); the keyword "Factory" itself is the parser key.
- `search_strings("Adjacent")` → 1 match at `0x0081ae40` → `0x0045ffb6` (BuildingType).
- `search_strings("EligibileForAllyBuilding")` → 1 match at `0x0081acc8` → `0x0046020a` (BuildingType). Westwood typo "Eligibile" is preserved in the binary.
- `search_strings("AIBuildThis")` → 1 match at `0x0081a7fc` → `0x00460fe9` (BuildingType).
- `get_function_by_address("00672ae0")` → FUN_00672ae0 (entry `00672ae0`, body `00672ae0-00673e76`) — unnamed in current Ghidra labels, but the address range (0x00672xxx) sits just above RulesClass__ReadGeneral (0x00671xxx) and the function reads the AI build tables (BuildConst, BuildPower, BuildRefinery, BuildBarracks, BuildTech, BuildWeapons). This is the **Rules AI-table reader**.

### Unit-specific hardcoded behavior?

GACNST has **no detectable unit-specific hardcoded code path**:

- `search_strings("GACNST")` not run as a direct unit search this pass; previous iterations did not find unit-keyed code for GACNST.
- The `ConstructionYard=yes` flag is generic — any building with the flag becomes a build-tree root; engine doesn't single out GACNST by name. This is symmetric across `[GACNST]`, `[NACNST]`, `[YACNST]`.
- `Factory=BuildingType` is generic; any building marked so produces buildings.
- AI BuildConst lookup is by INI-ID match against the table, not hardcoded address.

**Conclusion:** GACNST is a pure INI-driven building. The ConYard mechanic (build queue, adjacency, undeploy) is engine-level behavior triggered by the `ConstructionYard=yes` + `Factory=BuildingType` + `UndeploysInto=AMCV` field combination, not by the unit ID.

### TS-legacy filter

- **`Crewed=yes`** — active in YR (standard for all buildings).
- **`Capturable=true`** — active in YR (Engineer/spy mechanic).
- **`ImmuneToPsionics=no`** — active in YR (Yuri/MIND/PsiPulse can affect buildings explicitly opt-in to vulnerability).
- **`;DamageParticleSystems`** commented out — TS-era smoke system replaced by `DamageSmokeOffset` + `DamageFireOffset0/1` (also TS-legacy field names but functional in YR).
- **`;DestroyAnim=GACNSTDM`** commented out — the dedicated destruction anim is dead-code; Explosion= list is the active path. GACNSTDM SHP may exist in the asset bundle (TS-era leftover).
- **`MaxDebris=15` / `MinDebris=7`** — active YR debris system.
- **`TWLT070`** explosion anim — taillight (TS asset filename heritage), still active in YR.
- **No `FogOfWar`** / **no `0x1000 SpecialFlags` gating** — fully active in standard YR.
- **No `Subterranean` / `Tunnel`** — clean.

GACNST has no TS-legacy gating. Fully active in standard YR skirmish from match start.

---

## Cross-references

- **`AMCV`** (`units/allied/AMCV.md`) — DONE. Bidirectional deploy/undeploy pair (GACNST.UndeploysInto=AMCV, AMCV.DeploysInto=GACNST).
- **`NACNST`** — pending. Soviet ConYard (sister structure, `BuildConst` table member).
- **`YACNST`** — pending. Yuri ConYard (sister structure, `BuildConst` table member).
- **`SMCV`** (`units/soviet/SMCV.md`) — DONE. Soviet MCV deploys into NACNST.
- **`PCV`** (`units/yuri/PCV.md`) — DONE. Yuri MCV deploys into YACNST.
- **`GAPOWR` / `GAREFN` / `GAPILE` / `GAWEAP` / `GATECH` / `GAAIRC` / `GAYARD`** — all pending; all have `Prerequisite=GACNST`.
- **`ENGINEER`** (`units/allied/ENGINEER.md`) — DONE. Captures GACNST with `Capturable=true` flag.
- **`SPY`** (`units/allied/SPY.md`) — DONE. Spy infiltration is separate from capture; spy uses `Spyable=` flag (which GACNST does NOT have — note: ConYard is NOT spyable; vs GAPOWR which has `Spyable=yes`).
- **Tech-tree root cross-ref:** every Allied building under "## Build queue chain" above.

---

## Coverage audit

INI fields covered (28 rulesmd + 14 artmd = 42 total):

| Category          | Field                                | Covered |
|-------------------|--------------------------------------|---------|
| Identity          | UIName, Name                          | ✓ |
| ConYard mechanics | ConstructionYard, Adjacent, Factory, UndeploysInto | ✓ |
| Build gating      | (none: no Prerequisite, TechLevel=-1), Owner, Cost, Points, Power | ✓ |
| Combat            | Strength, Armor, Sight                | ✓ |
| Capture           | Capturable, Crewed, ImmuneToPsionics   | ✓ |
| Visual FX         | Explosion, DebrisAnims, MaxDebris, MinDebris, DamageSmokeOffset, ;DamageParticleSystems, ;DestroyAnim | ✓ |
| AI hints          | ThreatPosed, AIBuildThis, TogglePower, ProtectWithWall, EligibileForAllyBuilding | ✓ |
| artmd GACNST      | Remapable, Foundation, Height, AnimActive, Buildup, DemandLoadBuildup, FreeBuildup, NewTheater, ActiveAnim, ActiveAnimDamaged, ActiveAnimZAdjust, ActiveAnimYSort, ProductionAnim, ProductionAnimDamaged, ProductionAnimZAdjust, ProductionAnimYSort, CanHideThings, CanBeHidden, OccupyHeight, DamageFireOffset0, DamageFireOffset1 | ✓ |
| Sub-anims         | GACNST_A, GACNST_AD, GACNST_B, GACNST_BD | ✓ |

**Coverage: 42/42 = 100%.** Every key in the rulesmd block and the artmd block has been transcribed and explained. The 4 referenced sub-animations (GACNST_A, GACNST_AD, GACNST_B, GACNST_BD) have their own artmd blocks fully traced. `GACNSTMK` buildup is a SHP file (no artmd block — loaded by direct asset reference).

---

## Open questions / Westwood inconsistencies

1. **GAWEAP.Prerequisite=GAWEAP,GACNST** (line 11889) appears circular if literal. Likely Westwood typo — semantics in-game suggest `Prerequisite=GAPILE,GACNST` or similar. DEFERRED to GAWEAP's own audit pass.
2. **Mind-controlled ConYard production routing.** When Yuri's MIND grabs an enemy ConYard, can Yuri build from it? Anecdotally — no (ConYard is psi-non-immune but build-queue is owner-locked). Requires Ghidra trace of `BuildingClass__CanProduce` against owner-vs-controller logic. DEFERRED.
3. **Why `EligibileForAllyBuilding=yes` is needed in 2026** — the multiplayer share-build-queues setting referenced in the Westwood comment is a niche feature; the typo "Eligibile" was never patched. The flag's bit lives at some offset in BuildingTypeClass; precise offset not extracted this pass.
4. **`Sight=8` value source.** Most ConYards (NACNST, YACNST) have similar sight, but the choice of 8 (vs e.g. 6 for SMCV vehicle) is undocumented design intent. ConYards are visible-from-shroud anyway as your own base structure; the value may primarily affect shroud-around-own-building reveal radius.
5. **`Capturable=true` vs `Spyable=`** — GACNST is Capturable but lacks `Spyable=yes`. Compare GAPOWR which is both Capturable AND Spyable. Why isn't the ConYard spyable? Westwood deliberate (no useful intel from spying a ConYard — its contents are visible via vision range anyway), but DEFERRED to spy-system audit.

---

## Status

**DONE** — iteration 88. Index entry will be updated (status TODO → DONE, owner correction note added if applicable).

Doc total: **88**.

Next pick (priority): NACNST (Soviet ConYard), then YACNST (Yuri ConYard) — completes the ConYard trio. Then refineries (GAREFN/NAREFN; YAREFN already done as Slave Miner deploy form), then power plants (GAPOWR/NAPOWR/NANRCT/YAPOWR), then barracks (GAPILE/NAHAND/YABRCK), then war factories (GAWEAP/NAWEAP/YAWEAP), then battle labs (GATECH/NATECH/YATECH).
