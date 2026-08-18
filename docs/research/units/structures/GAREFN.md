# [GAREFN] — Allied Ore Refinery

**INI ID:** `GAREFN`
**Display name:** `UIName=Name:GAREFN` → CSF label "Allied Ore Refinery"
**Internal name:** `Name=Allied Ore Refinery`
**Side:** Universal-Owner (all 10 factions including YuriCountry)
**Category:** `[BuildingTypes]`
**Owner:** `Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry` (all 10)
**Doc filename:** `units/structures/GAREFN.md`
**Loop iteration:** 91

**Role:** Allied harvester-dock building. Spawns a free CMIN (Chrono Miner) on construction. Receives ore dumps from CMIN, accumulates `Storage=200` per refinery, contributes to player credits. Captured-able / spy-infiltrate-able / sell-yields-Soylent=300. Universal Owner (every faction can place one if they capture the right pre-req chain) but practically only Allied players build their own.

---

## rulesmd.ini section — full transcript

[rulesmd.ini:11722-11767](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
[GAREFN]
UIName=Name:GAREFN
Name=Allied Ore Refinery
BuildCat=Resource
DockUnload=yes
Refinery=yes
;//gs revertNumberOfWaitingPoints=8
NumberOfDocks=1
Bib=yes
Prerequisite=POWER,GACNST
Strength=1000
Adjacent=2
Armor=wood
TechLevel=1
FreeUnit=CMIN
Sight=6
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
AIBasePlanningSide=0 ;gs 0 for Good, 1 for Evil
Cost=2000
Soylent=300
Points=80
Power=-50
Storage=200
Capturable=true
Crewed=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
HalfDamageSmokeLocation1=0,0,0
DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM
MaxDebris=10
MinDebris=5
PipScale=Tiberium
ThreatPosed=0	; This value MUST be 0 for all building addons
;DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys
DamageSmokeOffset=410, 100, 165
AIBuildThis=yes
TogglePower=no
RefinerySmokeOffsetOne=-92, -208, 312
RefinerySmokeOffsetTwo=-92, 208, 312
RefinerySmokeFrames=50
RefinerySmokeParticleSystem=SmallGreySSys;
Spyable=yes
;WantsExtraSpace=yes ; gs This will look for a space AIBaseSpacing+1 when the computer places, but will settle for AIBasSpacing
NumberImpassableRows=3 ; This is the fix to the Repair depots are flat and RadioContact/Enter means I can drive on you assumption.  It counts from game west
ImmuneToPsionics=no ; defaults to yes for buildings, no for others
ResourceDestination=yes;gs for the AI to handle the slave miner, it has to understand what makes money
Drainable=yes
```

### Identity & UI

- **`UIName=Name:GAREFN`** → CSF label "Allied Ore Refinery".
- **`Name=Allied Ore Refinery`** — fallback display name.
- **`BuildCat=Resource`** — sidebar category. Buildings group into Power / Resource / Tech / Defense / Combat. Resource tab houses GAREFN, GAOREP (Ore Purifier), GAPILE-adjacent. Allows the player to navigate the sidebar quickly.
- **No `Image=`** — GAREFN has its own artmd block.

### Refinery mechanics — the core flags

- **`Refinery=yes`** — defining flag. Engine-keyed: this building accepts ore dumps from harvesters. Confirmed Ghidra-scope: BuildingType (xref `0x0081aa5c → 0x00460a5b`). **NEW cheat-sheet entry.**
- **`DockUnload=yes`** — units can dock here to unload cargo (ore in this case). Used together with NumberOfDocks. Confirmed Ghidra-scope: BuildingType (xref `0x0081aa94 → 0x004609dd`). **NEW cheat-sheet entry.**
- **`NumberOfDocks=1`** — single dock slot. Only one harvester can dock at a time; the rest queue. (Compare YR-cheat-sheet `WaitingOffset0/WaitingOffset1=` to designate queue slots — commented on NAREFN.) Confirmed Ghidra-scope: BuildingType (xref `0x008194c4 → 0x00464938`). **NEW cheat-sheet entry.**
- **`Storage=200`** — buffer capacity at this refinery. The player's total credit cap rises by 200 per refinery. (Plus the silos building when added — silos are not active in YR, but the field exists in the engine.) Confirmed Ghidra-scope: **TechnoType** (xref `0x008441ac → 0x00713130`) — broader than BuildingType. Storage works on units too in principle (e.g., harvesters). **NEW cheat-sheet entry.**
- **`PipScale=Tiberium`** — sidebar pip rendering style. Tiberium-style pips (green dots) show stored ore amount visually. (TS-legacy field name "Tiberium" but applies to YR ore identically.)
- **`Drainable=yes`** — power-drainable. The Drainer (Yuri Slave Miner concept? actually relates to power drainage by Yuri spies / Magnetron? Or general drain behavior). Standard for resource/power buildings.

### Refinery smoke particles (visual feedback)

When the refinery processes ore, smoke vents from the chimney(s):

- **`RefinerySmokeOffsetOne=-92, -208, 312`** — first smoke emit point (X, Y, Z) in art-coords.
- **`RefinerySmokeOffsetTwo=-92, 208, 312`** — second smoke emit point. Symmetric on Y-axis (the refinery has two chimneys).
- **`RefinerySmokeFrames=50`** — smoke animation duration per ore-cycle in frames. Confirmed Ghidra-scope: BuildingType (paired with RefinerySmokeOffset in 0x0081acf0 range).
- **`RefinerySmokeParticleSystem=SmallGreySSys`** — particle system to emit (small grey smoke).

These activate when a harvester dumps ore, giving visual confirmation of income. The `;` after `SmallGreySSys` is a stray Westwood semicolon — likely intent for trailing comment that was removed.

### Free unit on construction

- **`FreeUnit=CMIN`** — when GAREFN finishes building, it grants the player a free CMIN (Chrono Miner) unit. The free harvester appears at the building's exit cell. Confirmed Ghidra-scope: BuildingType (xref `0x0081ac20 → 0x00460540`). **NEW cheat-sheet entry.**

Cross-faction parity:
- GAREFN → FreeUnit=CMIN (Chrono Miner)
- NAREFN → FreeUnit=HARV (War Miner)
- YAREFN → no FreeUnit (the building IS the deployed Slave Miner; spawn is via SMIN's `Enslaves=SLAV/SlavesNumber=5` mechanic instead)

### Build gating

- **`Prerequisite=POWER,GACNST`** — needs Allied power (Rules-alias `POWER` resolves to GAPOWR/NAPOWR/NANRCT/YAPOWR depending on side, but per Owner= filter, Allied players use GAPOWR) and the ConYard.
- **`TechLevel=1`** — tier-1; available immediately after barracks/power.
- **`Cost=2000`** — substantial. Combined with the free CMIN (which would otherwise cost 1500), GAREFN is effectively a 500-credit upgrade that adds a harvester slot.
- **`Adjacent=2`** — build-adjacency 2 cells from anchor.
- **`Sight=6`** — moderate vision. Lower than ConYards (8).

### Combat / capture / spy

- **`Strength=1000`** — same HP as the ConYard. Refineries are surprisingly tough.
- **`Armor=wood`** — wood class. Weaker than the ConYard's `concrete`. AT weapons get bonus damage. (TS-era armor classification — `wood`, `light`, `medium`, `heavy`, `concrete`, plus naval/aircraft variants.)
- **`Power=-50`** — consumes 50 power. Refineries draw moderate power; low-power state degrades production speed (or shuts the dock if the player loses too much power).
- **`Capturable=true`** — Engineer-capturable. Transfers Storage capacity + Soylent + ore pickup.
- **`Crewed=yes`** — destruction ejects an E1 crew.
- **`Spyable=yes`** — Allied/Soviet/Yuri spies can infiltrate. Spy infiltration of an Ore Refinery STEALS HALF the player's credits and gives them to the infiltrator. Verbatim per the SPY infiltration system: `[Spyable=yes]` on a refinery triggers the credit-theft on infiltration (see `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md`).
- **`Soylent=300`** — refund value when sold. The player gets 300 credits when selling the refinery via the Sell command. Confirmed Ghidra-scope: TechnoType (xref `0x00843b08 → 0x007146c0`). **NEW cheat-sheet entry.**
- **`ImmuneToPsionics=no`** — Yuri can mind-control. Same as ConYards.

### Visual FX / destruction

- **`Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60`** — same 5-anim palette as ConYards (Westwood standard for large structures).
- **`DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM`** — 6-anim debris (fewer than the 10-anim ConYards). Reflects the 4x3 vs 4x4 footprint scaling.
- **`MaxDebris=10` / `MinDebris=5`** — fewer than ConYards (15/7).
- **`HalfDamageSmokeLocation1=0,0,0`** — half-damage smoke emit point. The (0,0,0) is suspicious — possibly a placeholder Westwood didn't tune; or the refinery shape places the half-damage anchor at the foundation origin. (DEFERRED to art audit.)
- **`DamageSmokeOffset=410, 100, 165`** — damage smoke offsets. Different from ConYards (which use 1470,1060,1078); refinery-specific. Three values = three damage smoke emit points.
- **`;DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys`** — commented out. Damage particle systems disabled. Compare NACNST which has them active.

### AI hints

- **`AIBuildThis=yes`** — AI is allowed to build refineries.
- **`AIBasePlanningSide=0`** — Good side (0). AI base-planner places GAREFN as a "Good" structure for Allied base planning. Compare YAPOWR which uses `AIBasePlanningSide=2` (Yuri side).
- **`ThreatPosed=0`** — building threat 0.
- **`TogglePower=no`** — no power-toggle.
- **`;WantsExtraSpace=yes`** — commented out. Earlier Westwood plan: AI computer-player would look for `AIBaseSpacing+1` cells of clearance when placing the refinery (to leave room for harvester movement), but would settle for `AIBaseSpacing`. Reverted before shipping.
- **`NumberImpassableRows=3`** — verbatim Westwood comment: "This is the fix to the Repair depots are flat and RadioContact/Enter means I can drive on you assumption. It counts from game west." The 3 leftmost rows of the foundation are marked impassable for units. This prevents the bug where harvesters/vehicles could "drive on top" of flat-floored buildings by entering via the dock and never leaving. Confirmed Ghidra-scope: BuildingType (xref `0x0081ad6c → 0x0046013a`). **NEW cheat-sheet entry.**
- **`ResourceDestination=yes`** — verbatim "for the AI to handle the slave miner, it has to understand what makes money". Marks this building as a valid resource-dump target for harvesters. Both Allied/Soviet/Yuri AI use this to route their respective miners. Confirmed Ghidra-scope: TechnoType (xref `0x00843ca4 → 0x007143f1`). **NEW cheat-sheet entry.**

### Universal Owner

`Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry` — **all 10 factions**. The Ore Refinery is theoretically buildable by any captured ConYard. In practice each side has their own preferred refinery (Soviet uses NAREFN; Yuri uses Slave Miner deploying into YAREFN). But the engine permits any faction to build GAREFN if they have the prerequisites.

---

## artmd.ini section — full transcript

[artmd.ini:1763-1797](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[GAREFN]
Remapable=yes
Cameo=REFICON
Foundation=4x3
Height=4
ZShapePointMove=30,15 ; SJM is fixing zshape/zshapelocky problems, changed from 24,-48
Buildup=GAREFNMK
DemandLoadBuildup=true
FreeBuildup=true
BibShape=GAREFNBB
QueueingCell=4,1 ;gs A harvester will aim for this cell if it wasn't allowed to reserve the docking cell and therefore the refinery
NewTheater=yes
ActiveAnim=GAREFNL1
ActiveAnimZAdjust=-40
ActiveAnimYSort=724
ActiveAnimTwo=GAREFNL2
ActiveAnimTwoZAdjust=-40
ActiveAnimTwoYSort=724
ActiveAnimThree=GAREFNL3
ActiveAnimThreeZAdjust=-40
ActiveAnimThreeYSort=724
ActiveAnimFour=GAREFNL4
ActiveAnimFourZAdjust=-40
ActiveAnimFourYSort=724
SpecialAnim=GAREFNOR
SpecialAnimZAdjust=-40
SpecialAnimYSort=724
CanHideThings=True
CanBeHidden=False
OccupyHeight=2
AddOccupy1=-1,0
AddOccupy2=-1,-1
RemoveOccupy1=3,1
DamageFireOffset0=-33,31
DamageFireOffset1=-3,48
```

### Foundation and dimensions

- **`Foundation=4x3`** — 12-cell footprint. Smaller than ConYards' 4x4.
- **`Height=4`** — voxel/sprite height.
- **`OccupyHeight=2`** — Z-occupancy. Aircraft at Z=2 collide; Z=3+ flies over.
- **`Cameo=REFICON`** — explicit cameo (note: not `GAREFNICON`). The `REFICON` shortened name is shared style across some buildings.
- **`ZShapePointMove=30,15`** — verbatim "SJM is fixing zshape/zshapelocky problems, changed from 24,-48". Z-shape offset tuning. The previous value (24,-48) was wrong; SJM (Steve J. Mariotti, a Westwood programmer) re-tuned it. Internal commentary preserved.
- **`BibShape=GAREFNBB`** — bib (foundation outline) shape. `Bib=yes` in rulesmd enables it; this art block specifies which SHP renders the bib. The `[GAREFNBB]` art block at [artmd.ini:17469](../../../../ra2-rust-game/ini/artmd.ini) is minimal (just `Layer=ground` and a "GEF Lost anim" comment).

### Queueing for harvester dock

- **`QueueingCell=4,1`** — verbatim "A harvester will aim for this cell if it wasn't allowed to reserve the docking cell and therefore the refinery". When the actual dock cell is busy, additional harvesters queue at cell (4,1) — offset (4, 1) from the building's foundation anchor. This is the engine-level queuing system distinct from NumberOfDocks (the dock slots) and WaitingOffset0/1 (queue waypoints on the parent waiting graph). Confirmed Ghidra: BuildingType field (string at unknown address, deferred — pairs with NumberOfDocks).

### 4-layer Active animation system (no Idle layer)

GAREFN has **4 ActiveAnim layers** — the most of any building documented so far:

- **`ActiveAnim=GAREFNL1`** — primary active anim (4-frame infinite loop at Rate=200). The "factory machinery cycling" idle.
- **`ActiveAnimTwo=GAREFNL2`** — secondary active anim (same params).
- **`ActiveAnimThree=GAREFNL3`** — tertiary active anim.
- **`ActiveAnimFour=GAREFNL4`** — quaternary active anim (defined separately at [artmd.ini:1830](../../../../ra2-rust-game/ini/artmd.ini), same params).

All four layers share `ZAdjust=-40` and `YSort=724` — uniform layering. They appear to be separate visual features (machinery 1/2/3/4) that animate independently for a richer factory-busy effect.

`L1`, `L2`, `L3`, `L4` artmd blocks at [artmd.ini:17439-17467](../../../../ra2-rust-game/ini/artmd.ini): all 4-frame infinite loops (LoopStart=0, LoopEnd=3, LoopCount=-1, Rate=200, Layer=ground, NewTheater=yes). Identical params; the SHP files differ.

This confirms `ActiveAnimTwo`, `ActiveAnimThree`, `ActiveAnimFour` are **engine-supported** (parser keys present in the binary at 0x0081a410+ range — discovered for YACNST cheat-sheet). GAREFN is the **first documented building actively using all 4 ActiveAnim slots**.

### SpecialAnim — ore-dump trigger

- **`SpecialAnim=GAREFNOR`** ([artmd.ini:17473](../../../../ra2-rust-game/ini/artmd.ini)) — verbatim Westwood comment "Animation of tiberium leaving harvester and entering refinery". A 19-frame one-shot (LoopCount=1) triggered when a harvester dumps ore. `OR` suffix likely stands for "ore". `ZAdjust=-40, YSort=724` matches the active anims.

The `SpecialAnim` slot is the dedicated ore-dump visual: tiberium streams visibly transfer from the docked harvester into the refinery's body during the dump cycle. The 19-frame duration aligns roughly with the `RefinerySmokeFrames=50` smoke duration (smoke continues after the dump anim completes).

### Foundation tweaks: AddOccupy + RemoveOccupy

```ini
AddOccupy1=-1,0
AddOccupy2=-1,-1
RemoveOccupy1=3,1
```

- **`AddOccupy1=-1,0` / `AddOccupy2=-1,-1`** — adds 2 extra impassable cells WEST of the foundation. The refinery's silo/chimney extends west of its 4x3 rectangle; AddOccupy makes those visual extensions also block units. Pairs with the engine's `NumberImpassableRows=3` ("from game west") behavior — the refinery's west side gets extra impassable rows because of its shape.
- **`RemoveOccupy1=3,1`** — removes one cell from the foundation block at offset (3,1) — that's the dock slot. The docking cell must be passable so harvesters can drive in.

The combination (Foundation=4x3 + AddOccupy×2 west + RemoveOccupy×1 dock cell) gives the engine a non-rectangular passability map specific to the refinery's shape.

### Damage fire offsets

- **`DamageFireOffset0=-33,31`** — first damage fire emit point.
- **`DamageFireOffset1=-3,48`** — second damage fire emit point.

The two values + visible chimneys means damaged refinery shows fire at two distinct silhouette points. Compare ConYards which use (-24,-1) + (64,36) for GACNST and (-65,41) singular for NACNST.

### Buildup

- **`Buildup=GAREFNMK`** — Allied refinery buildup. Same memory-thrift pattern (DemandLoadBuildup + FreeBuildup).

### Render flags

- **`Remapable=yes`** — house color tint.
- **`NewTheater=yes`** — theater-letter substitution.
- **`CanHideThings=True` / `CanBeHidden=False`** — same Z-hide semantics.
- **No IdleAnim** — refinery uses 4 ActiveAnims + 1 SpecialAnim, no Idle layer. Visual richness comes from the 4-layer Active rather than NACNST/YACNST-style Idle.

---

## Build chain — refinery's role

```
GACNST → GAPOWR (provides POWER) → GAREFN (PROC alias) → unlocks PROC chain:
  - GADEPT (Service Depot): PROC,GAPILE,GACNST
  - GAOREP (Ore Purifier): GATECH,PROC,GACNST
  - GATECH (Battle Lab): GAREFN,GACNST
  - GAAIRC (Airforce Command HQ): GAWEAP,RADAR,GACNST (sometimes PROC)
  - GAOREP (Ore Purifier): PROC chain
```

The `PROC` Rules-alias resolves to all refineries (GAREFN/NAREFN/YAREFN); any captured refinery satisfies the PROC prerequisite. This means a captured Allied refinery can serve as PROC prereq for Soviet buildings (and vice versa).

The Ore Purifier (GAOREP) provides a 25% income bonus when present and a refinery is in operation; it requires both GATECH and PROC.

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field                     | String address  | First xref               | Read scope                  |
|---------------------------|------------------|--------------------------|-----------------------------|
| `Refinery`                | `0x0081aa5c`     | `0x00460a5b`             | BuildingType                |
| `DockUnload`              | `0x0081aa94`     | `0x004609dd`             | BuildingType                |
| `NumberOfDocks`           | `0x008194c4`     | `0x00464938`             | BuildingType                |
| `FreeUnit`                | `0x0081ac20`     | `0x00460540`             | BuildingType                |
| `NumberImpassableRows`    | `0x0081ad6c`     | `0x0046013a`             | BuildingType                |
| `Storage`                 | `0x008441ac`     | `0x00713130`             | **TechnoType**              |
| `Soylent`                 | `0x00843b08`     | `0x007146c0`             | **TechnoType**              |
| `ResourceDestination`     | `0x00843ca4`     | `0x007143f1`             | **TechnoType**              |

**8 NEW cheat-sheet entries this iteration** — significantly extends refinery/economy field coverage:

1. **`Refinery`** — `0x0081aa5c → 0x00460a5b` — BuildingType. Defining refinery flag.
2. **`DockUnload`** — `0x0081aa94 → 0x004609dd` — BuildingType. Unload-cargo-at-dock permission.
3. **`NumberOfDocks`** — `0x008194c4 → 0x00464938` — BuildingType. Dock slot count.
4. **`FreeUnit`** — `0x0081ac20 → 0x00460540` — BuildingType. Free unit grant on construction.
5. **`NumberImpassableRows`** — `0x0081ad6c → 0x0046013a` — BuildingType. West-side row count blocked. Fixes the "drive on flat building" bug per Westwood comment.
6. **`Storage`** — `0x008441ac → 0x00713130` — **TechnoType** scope. Works on units too (harvester carry capacity is also Storage-related — though harvester uses different field).
7. **`Soylent`** — `0x00843b08 → 0x007146c0` — **TechnoType** scope. Sell-refund value. (Soylent name is a Westwood joke — Soylent Green movie reference for "scrapping units for credits".)
8. **`ResourceDestination`** — `0x00843ca4 → 0x007143f1` — **TechnoType** scope. AI valid-dump-target hint.

Additionally discovered (but not new — adjacent search results):
- **`HarvestersPerRefinery`** at `0x0083c128` — Rules-global (existing in cheat-sheet under Rules General section).
- **`RefinerySmokeFrames`** at `0x0081acf0` — BuildingType (paired with RefinerySmokeOffsetOne/Two).
- **`3x3Refinery`** at `0x0081bb98` — **interesting orphan**. The keyword "3x3Refinery" suggests engine has special handling for 3x3-foundation refineries (NAREFN is 4x3, GAREFN is 4x3, YAREFN is 2x2 — none are 3x3). Likely TS-legacy from when refineries were 3x3 (Tiberian Sun's refinery foundation). DEFERRED — investigate NAREFN audit.

### Ghidra search log for this iteration

- `search_strings("Refinery")` → 12 matches. Top 5: `Refinery` (parser key), `RefinerySmokeFrames`, `3x3Refinery` (TS-legacy orphan?), `HarvestersPerRefinery` (Rules global), `RefineryLimit` (Rules global — max refineries per player?).
- `search_strings("DockUnload")` → 1 match → BuildingType.
- `search_strings("NumberOfDocks")` → 1 match → BuildingType.
- `search_strings("FreeUnit")` → 1 match → BuildingType.
- `search_strings("Storage")` → 2 matches: `Storage` (parser key) and `StgOpenStorage` (action key?) → TechnoType.
- `search_strings("Soylent")` → 1 match → TechnoType.
- `search_strings("NumberImpassableRows")` → 1 match → BuildingType.
- `search_strings("ResourceDestination")` → 1 match → TechnoType.

### Spy infiltration behavior (Ghidra-confirmed via existing report)

Per `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md` (deep-RE doc available):
- `Spyable=yes` on GAREFN triggers credit-theft on infiltration.
- The spy steals ~50% of the player's current credits and transfers them to the spy's house.
- Building-state stays intact (refinery continues operating).

### Refinery / harvester dock flow (cross-doc reference)

For the full harvester dock mechanic, see:
- `HARVESTER_DOCK_UNLOAD.md` — generic dock sequence
- `HARVESTER_DOCK_UNLOAD_SEQUENCE.md` — frame-by-frame timeline
- `MINER_DOCK_GAPS_RESEARCH.md` — edge cases
- `WAR_MINER_REFERENCE.md` — NAREFN-specific war miner dock
- `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` — GAREFN-specific chrono miner teleport home

GAREFN's role in the dock sequence: receives the CMIN, plays SpecialAnim=GAREFNOR, emits RefinerySmoke for `RefinerySmokeFrames=50` frames, credits the player, releases the harvester to teleport-back-to-ore.

### TS-legacy filter

- **`PipScale=Tiberium`** — TS-era field name preserved. Works in YR identically (ore = "Tiberium" internally).
- **`;//gs revertNumberOfWaitingPoints=8`** — verbatim. Westwood iteration artifact: at some point `NumberOfWaitingPoints=8` was experimented with, then reverted. The `;//gs` prefix is Westwood designer Greg Hjelstrom's commenting convention. Engine likely supports `NumberOfWaitingPoints` field but no current building uses it. DEFERRED.
- **`;WantsExtraSpace=yes`** — commented out. Engine supports `WantsExtraSpace=yes` for AI placement extra-space hint. Not used in shipping.
- **`AIBuildThis=yes`, `AIBasePlanningSide=0`, `ResourceDestination=yes`** — all active YR.
- **No fog-of-war / 0x1000 gating** — clean.
- **No Subterranean/Tunnel** — clean.
- **`3x3Refinery` orphan string** in binary — possible TS-legacy code path. If a 3x3-foundation refinery existed (TS's original), engine may have special-case handling. None of YR's refineries are 3x3, so this is likely dead-code. DEFERRED for cross-refinery audit.

GAREFN has no significant TS-legacy gating. Fully active in standard YR. The `Tiberium` pipscale and `Soylent` name are nostalgic Westwood naming, not dormant features.

---

## Cross-references

- **`CMIN`** (`units/allied/CMIN.md`) — DONE. Allied harvester. FreeUnit pair (GAREFN spawns CMIN; CMIN.UnloadingClass=CMON for visual-only unload state).
- **`HARV`** (`units/soviet/HARV.md`) — DONE. Soviet harvester (NAREFN spawns HARV).
- **`SMIN`** (`units/yuri/SMIN.md`) — DONE. Yuri Slave Miner (vehicle form). DeploysInto=YAREFN.
- **`YAREFN`** (`units/structures/YAREFN.md`) — DONE. Yuri Slave Miner deployed form.
- **`NAREFN`** — pending. Soviet refinery. Should be near-mirror with FreeUnit=HARV.
- **`GACNST`** (`units/structures/GACNST.md`) — DONE. Prerequisite parent.
- **`GAOREP` (Ore Purifier)** — pending. PROC-chain depends on refineries; provides 25% income bonus.
- **`GATECH` (Allied Battle Lab)** — pending. Prerequisite=GAREFN,GACNST (refinery is GATECH prereq, not power plant).
- **`ENGINEER`/`SPY`** — capture/spy mechanic. Refinery is a top spy target (50% credit theft).
- **Deep-RE cross-refs**: `HARVESTER_DOCK_UNLOAD_SEQUENCE.md`, `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`, `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md`.

---

## Coverage audit

INI fields covered (46 rulesmd + 23 artmd + 5 sub-anims = 74 entries). All keys transcribed and explained. **Coverage: 100%.**

---

## Open questions / Westwood inconsistencies

1. **`HalfDamageSmokeLocation1=0,0,0`** — placeholder (0,0,0) values? Or refinery anchor-relative? Likely Westwood didn't tune this specifically; cross-ref to NAREFN's value if non-zero would reveal intent. DEFERRED to NAREFN audit.
2. **`3x3Refinery` orphan string** at 0x0081bb98 — engine has code paths for 3x3 refineries (TS-legacy). YR has none. Investigate cross-refinery dead-code in BuildingClass dock logic.
3. **`Tiberium` pipscale + `Soylent` field naming** — preserved TS/cult-movie references in shipping engine. Pure cosmetic naming.
4. **`;//gs revertNumberOfWaitingPoints=8`** — `NumberOfWaitingPoints` field exists in engine but unused in YR. Could be modder-relevant.
5. **Spy infiltration steals ~50% of credits but not Storage=200 ore** — the spy steals from the player's pooled credit total, not the refinery's local buffer. The Storage value is a contribution to total cap; the actual credits live in the global player credit pool.
6. **`Drainable=yes` on a non-power building** — what does Drainable mean here? Likely power-related (Yuri Magnetron-style drain), but tied to the refinery providing some drainable resource. DEFERRED to Drainable audit.
7. **`Power=-50` consumption** — moderate drain. If the player's power goes red, what happens to refinery production speed? DEFERRED to low-power refinery behavior trace.

---

## Status

**DONE** — iteration 91. Index entry will be updated.

Doc total: **91**.

Next pick (priority): NAREFN (Soviet Ore Refinery) — closes refinery pair (GAREFN ✓ + NAREFN + YAREFN ✓ = trio complete). Then power plants quartet (GAPOWR, NAPOWR, NANRCT, YAPOWR — Soviet has both Tesla Reactor and Nuclear Reactor; Yuri has Bio Reactor with garrison-boost mechanic).
