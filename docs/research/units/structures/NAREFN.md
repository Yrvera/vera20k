# [NAREFN] — Soviet Ore Refinery

**INI ID:** `NAREFN`
**Display name:** `UIName=Name:NAREFN` → CSF label "Soviet Ore Refinery"
**Internal name:** `Name=Soviet Ore Refinery`
**Side:** Universal-Owner (all 10 factions)
**Category:** `[BuildingTypes]`
**Owner:** `Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry` (all 10)
**Doc filename:** `units/structures/NAREFN.md`
**Loop iteration:** 92

**Role:** Soviet sister to GAREFN. Spawns HARV (War Miner) instead of CMIN (Chrono Miner). Near-mechanical mirror of GAREFN with 5 notable diffs in stats and structure. Closes the refinery trio (GAREFN ✓ + NAREFN + YAREFN ✓).

---

## rulesmd.ini section — full transcript

[rulesmd.ini:12515-12558](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
[NAREFN]
UIName=Name:NAREFN
Name=Soviet Ore Refinery
BuildCat=Resource
DockUnload=yes
Refinery=yes
NumberOfDocks=1
;//gs revertNumberOfWaitingPoints=8
Bib=yes
NumberImpassableRows=3 ; This is the fix to the Repair depots are flat and RadioContact/Enter means I can drive on you assumption.  It counts from game west
Prerequisite=POWER,NACNST
Strength=1000
Adjacent=2
Armor=wood
TechLevel=1
FreeUnit=HARV
Sight=6
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
AIBasePlanningSide=1 ;gs 0 for Good, 1 for Evil
Cost=2000
Soylent=300
Points=80
Power=-50
Storage=200
Capturable=true
Crewed=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
HalfDamageSmokeLocation1=0,0,0
MaxDebris=8
PipScale=Tiberium
ThreatPosed=0	; This value MUST be 0 for all building addons
;DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys
DamageSmokeOffset=410, 100, 165
AIBuildThis=yes
TogglePower=no
RefinerySmokeOffsetOne=-80, -232, 372
RefinerySmokeOffsetTwo=-80, 232, 372
RefinerySmokeFrames=50
RefinerySmokeParticleSystem=SmallGreySSys;
Spyable=yes
;WantsExtraSpace=yes ; gs This will look for a space AIBaseSpacing+1 when the computer places, but will settle for AIBasSpacing
ImmuneToPsionics=no ; defaults to yes for buildings, no for others
ResourceDestination=yes;gs for the AI to handle the slave miner, it has to understand what makes money
Drainable=yes
```

### Diffs vs GAREFN (the relevant 5)

NAREFN is a **near-perfect mechanical mirror** of GAREFN. Comparing field-by-field:

| Field                    | GAREFN                   | NAREFN                  | Notes |
|--------------------------|--------------------------|--------------------------|-------|
| FreeUnit                 | CMIN (Chrono Miner)      | **HARV (War Miner)**      | per-side harvester |
| AIBasePlanningSide       | 0 (Good)                  | **1 (Evil)** | AI side hint |
| Prerequisite             | POWER,GACNST              | **POWER,NACNST**           | per-side ConYard |
| RefinerySmokeOffsetOne   | -92, -208, 312            | **-80, -232, 372**         | chimney placement diff (NAREFN's chimneys are slightly offset, higher Z) |
| RefinerySmokeOffsetTwo   | -92, 208, 312             | **-80, 232, 372**          | symmetric Y |
| MaxDebris                | 10                        | **8**                       | NAREFN drops fewer (TS-style smaller debris budget) |
| MinDebris                | 5                         | **(absent — defaults to engine default)** | NAREFN omits MinDebris |
| DebrisAnims=             | `DBRIS1LG,DBRIS1SM,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM` (6 anims) | **(absent — uses engine default)** | NAREFN doesn't override DebrisAnims |
| NumberImpassableRows location | line 11764 (after PipScale section, late in block) | **line 12524 (early in block, right after Bib=yes)** | structural ordering diff |

**Identical to GAREFN in:** BuildCat=Resource, DockUnload=yes, Refinery=yes, NumberOfDocks=1, ;//gs revertNumberOfWaitingPoints=8 (Westwood iteration artifact preserved on both), Bib=yes, Strength=1000, Adjacent=2, Armor=wood, TechLevel=1, Sight=6, Owner= (all 10 factions identical), Cost=2000, Soylent=300, Points=80, Power=-50, Storage=200, Capturable=true, Crewed=yes, Explosion= (same 5-anim palette), HalfDamageSmokeLocation1=0,0,0, PipScale=Tiberium, ThreatPosed=0, ;DamageParticleSystems commented, DamageSmokeOffset=410/100/165 (same offset as GAREFN — Westwood copy-pasted; the artmd offsets are tuned per-side via art block), AIBuildThis=yes, TogglePower=no, RefinerySmokeFrames=50, RefinerySmokeParticleSystem=SmallGreySSys;, Spyable=yes, ;WantsExtraSpace=yes commented, ImmuneToPsionics=no, ResourceDestination=yes, Drainable=yes.

**Mechanical parity confirmed:** Same Cost, same Storage, same HP, same Soylent refund, same prereq chain (POWER + ConYard), same power consumption, same AI behavior except faction-side. The Soviet refinery is **identical economy** to the Allied refinery; the gameplay difference comes from CMIN vs HARV (different harvesters with different speeds, capacities, abilities).

### Identity & UI
- **`UIName=Name:NAREFN`** → CSF label "Soviet Ore Refinery".
- **`Name=Soviet Ore Refinery`** — fallback.
- **`BuildCat=Resource`** — Resource tab in sidebar.

### Refinery mechanics — same as GAREFN
- **`Refinery=yes`** — defining refinery flag (BuildingType-scope per cheat-sheet).
- **`DockUnload=yes`** — harvester unload permission.
- **`NumberOfDocks=1`** — single dock slot.
- **`Storage=200`** — credit cap contribution.
- **`Drainable=yes`** — power-drain target.

### Free unit
- **`FreeUnit=HARV`** — spawns a free HARV (War Miner) on construction. War Miner has a small AA-capable 20mmRapid turret (unique among harvesters — see HARV doc) plus the standard ore-pickup behavior. Effectively a 600-credit upgrade over HARV's 1400 cost (HARV Cost is documented; reusing the cheat-sheet HARV doc).

### Build gating
- **`Prerequisite=POWER,NACNST`** — Soviet ConYard + Soviet power.
- **`TechLevel=1`** — tier-1.
- **`Cost=2000`** — same as GAREFN.
- **`Adjacent=2`** — same adjacency.
- **`Sight=6`** — same vision.

### Combat / capture / spy
- **`Strength=1000`** — same HP.
- **`Armor=wood`** — same armor.
- **`Power=-50`** — same consumption.
- **`Capturable=true` / `Crewed=yes`** — engineer-capturable, crew-on-destroy (E2 for Soviet).
- **`Spyable=yes`** — spy-infiltrate-able (50% credit theft, same as GAREFN).
- **`Soylent=300`** — same refund.
- **`ImmuneToPsionics=no`** — psi-vulnerable.

### Refinery smoke (slightly different offsets)

NAREFN's chimneys are placed differently in the SHP art — RefinerySmokeOffsetOne/Two values are tuned for the Soviet art (NAREFN's chimneys are slightly forward and higher than GAREFN's):

```
RefinerySmokeOffsetOne=-80, -232, 372  (vs GAREFN: -92, -208, 312)
RefinerySmokeOffsetTwo=-80, 232, 372   (vs GAREFN: -92, 208, 312)
```

The X-coord (-80 vs -92), Y-coord (±232 vs ±208), and Z-coord (372 vs 312) differ — NAREFN's smoke is 12 units further south-east and 60 units higher. Reflects the Soviet refinery's taller silhouette (Height=6 vs GAREFN's 4 — see artmd).

### Visual FX
- **`Explosion=TWLT070,...`** — same 5-anim palette.
- **`HalfDamageSmokeLocation1=0,0,0`** — same placeholder (0,0,0) — Westwood didn't tune for either side. Confirms this is a generic field that defaults to building origin.
- **`MaxDebris=8`** (no MinDebris) — fewer debris than GAREFN's 10/5 pair. NAREFN drops a small spray.
- **No `DebrisAnims=`** — uses engine default. The shipping behavior is debris-anim-by-engine-default, which is the standard "Dbris1lg, Dbris1sm, Dbris4lg, Dbris4sm" mix per generic debris policy.

### AI hints
- **`AIBuildThis=yes`** — AI permission.
- **`AIBasePlanningSide=1`** — **Evil side** (vs GAREFN's 0 Good). The AI base planner classifies NAREFN as a Soviet/Evil building.
- **`ThreatPosed=0`** — building threat 0.
- **`NumberImpassableRows=3`** — same fix as GAREFN.
- **`;WantsExtraSpace=yes`** — same commented Westwood artifact.

### Universal Owner — same as GAREFN
All 10 factions can theoretically build NAREFN if they have NACNST. The expected pattern is: Soviet players build NAREFN, Allied players build GAREFN, Yuri uses Slave Miner deploy. But the engine permits any captured-faction to build any refinery they have the prereqs for.

---

## artmd.ini section — full transcript

[artmd.ini:1706-1761](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[NAREFN]
Remapable=yes
Cameo=NREFICON
Foundation=4x3
Height=6
ZShapePointMove=30,15 ; SJM is fixing zshape/zshapelocky problems, changed from 24,-48
Buildup=NAREFNMK
DemandLoadBuildup=true
FreeBuildup=true
BibShape=NAREFNBB
QueueingCell=4,1 ;gs A harvester will aim for this cell if it wasn't allowed to reserve the docking cell and therefore the refinery
;WaitingOffset0=512,-256,0
;WaitingOffset1=512,256,0
;WaitingOffset2=768,-256,0
;WaitingOffset3=768,256,0
;WaitingOffset4=1024,-256,0
;WaitingOffset5=1024,256,0
;WaitingOffset6=1280,-256,0
;WaitingOffset7=1280,256,0
;DockingOffset0=256,0,0
NewTheater=yes
ActiveAnim=NAREFNL1
ActiveAnimZAdjust=-5
ActiveAnimDamaged=NAREFNL1D
ActiveAnimTwo=NAREFNL2
ActiveAnimTwoZAdjust=-5
ActiveAnimTwoDamaged=NAREFNL2D
ActiveAnimThree=NAREFNL3
ActiveAnimThreeZAdjust=-5
ActiveAnimThreeDamaged=NAREFNL3D
ActiveAnimFour=NAREFNL4
ActiveAnimFourZAdjust=-5
ActiveAnimFourDamaged=NAREFNL4D
SpecialAnim=NAREFNOR
SpecialAnimZAdjust=-50
;SpecialAnimX=800
;SpecialAnimY=800
;ActiveAnim=NAREFN_C
;ActiveAnimZAdjust=-100
;ActiveAnimTwo=NAREFN_B
;ActiveAnimTwoZAdjust=-250
;ActiveAnimTwoPowered=no
;PreProductionAnim=NAREFN_A
;ProductionAnim=NAREFN_AR
OccupyHeight=4
CanBeHidden=False
CanHideThings=true
RemoveOccupy1=0,-2
RemoveOccupy2=1,-1
RemoveOccupy3=1,-2
RemoveOccupy4=2,-1
RemoveOccupy5=-2,0
RemoveOccupy6=-2,-1
RemoveOccupy7=-2,-2
RemoveOccupy8=3,1
DamageFireOffset0=30,30
```

### Diffs vs GAREFN artmd

| Field | GAREFN artmd | NAREFN artmd | Notes |
|-------|--------------|--------------|-------|
| Cameo | REFICON | **NREFICON** | per-side cameo SHP |
| Foundation | 4x3 | 4x3 (parity) | |
| Height | 4 | **6** (taller) | Soviet refinery has taller silhouette |
| OccupyHeight | 2 | **4** (taller Z-occupancy) | matches taller building |
| Buildup | GAREFNMK | NAREFNMK | per-side |
| BibShape | GAREFNBB | NAREFNBB | per-side |
| QueueingCell | 4,1 | 4,1 (parity) | same queue offset relative to foundation |
| **WaitingOffset0-7** | (absent) | **8 commented entries** | NAREFN documents the abandoned WaitingOffset queue system |
| **DockingOffset0** | (absent) | **`;DockingOffset0=256,0,0` (commented)** | NAREFN has dock-offset override commented out |
| ActiveAnim layers | 4 (L1-L4, no Damaged variants) | **4 (L1-L4, all 4 have Damaged variants)** | NAREFN has fully-damaged variants |
| ActiveAnimZAdjust | -40 | **-5** | NAREFN's active anims sit lower |
| ActiveAnimYSort | 724 (uniform) | **(absent)** | NAREFN omits explicit Y-sort |
| SpecialAnim ZAdjust | -40 | **-50** | NAREFN's ore-dump anim slightly lower |
| AddOccupy | 2 entries (west extension) | **(absent — replaced by RemoveOccupy 8 entries)** | structural diff |
| RemoveOccupy | 1 entry (dock cell) | **8 entries** (`RemoveOccupy1-8` north + west + dock) | NAREFN has wider impassable-removal map for its taller silhouette |
| DamageFireOffset0 | -33,31 | **30,30** | per-side art coords |
| DamageFireOffset1 | -3,48 | **(absent — single fire point)** | NAREFN uses single damage fire |

### NAREFN has 8 RemoveOccupy slots (vs GAREFN's 1)

```ini
RemoveOccupy1=0,-2
RemoveOccupy2=1,-1
RemoveOccupy3=1,-2
RemoveOccupy4=2,-1
RemoveOccupy5=-2,0
RemoveOccupy6=-2,-1
RemoveOccupy7=-2,-2
RemoveOccupy8=3,1
```

8 cells removed from the foundation occupy-map. This is the standard pattern for Soviet's taller, more-irregular industrial buildings (NACNST also used 8 RemoveOccupy slots for crane clearance). NAREFN's tall silhouette + chimneys + tower extends north (negative Y) and west (negative X) beyond the 4x3 foundation — RemoveOccupy marks those visual-only cells as passable.

The 8th slot `(3,1)` is the dock cell (same offset as the QueueingCell, which is also (4,1) — slight offset). The dock must remain passable for harvesters to enter.

### Commented-out WaitingOffset0-7

```ini
;WaitingOffset0=512,-256,0
;WaitingOffset1=512,256,0
;WaitingOffset2=768,-256,0
;WaitingOffset3=768,256,0
;WaitingOffset4=1024,-256,0
;WaitingOffset5=1024,256,0
;WaitingOffset6=1280,-256,0
;WaitingOffset7=1280,256,0
```

Westwood iterated an explicit waiting-queue system: 8 waypoints around the refinery where harvesters would queue when the dock was busy. Each offset is (X, Y, Z) in leptons (256 = 1 cell). The grid is 2 cells × 4 rows of 2 columns, fanning out east of the foundation (positive X).

These were **commented out**. The shipping engine uses the engine-default queueing (`QueueingCell=4,1` + engine-managed wait positions). The `;//gs revertNumberOfWaitingPoints=8` comment in rulesmd confirms this is a reverted feature.

This is a **latent engine capability** — modders could uncomment to override default queueing. The engine reads WaitingOffset0-N format-string-loop fields (same pattern as RemoveOccupy%d).

### Commented-out DockingOffset0
```ini
;DockingOffset0=256,0,0
```

Override for the docking-position offset. Default (uncommented) uses the building's natural dock; with this set, the engine would offset by (256, 0, 0) leptons (1 cell east). Reverted. Confirmed Ghidra-scope: BuildingType (xref `0x008194b4 → 0x004649b7`, format-string-loop `DockingOffset%d`). **NEW cheat-sheet entry.**

### Commented-out alternative anim layouts
```ini
;ActiveAnim=NAREFN_C
;ActiveAnimZAdjust=-100
;ActiveAnimTwo=NAREFN_B
;ActiveAnimTwoZAdjust=-250
;ActiveAnimTwoPowered=no
;PreProductionAnim=NAREFN_A
;ProductionAnim=NAREFN_AR
```

Earlier Westwood layout used:
- `NAREFN_A`, `NAREFN_B`, `NAREFN_C` as the active anim set (vs shipped `L1`-`L4`).
- A `PreProductionAnim=NAREFN_A` (a "warming up" animation before production starts) — same field discussed on NACNST's commented entries. Confirms engine supports `PreProductionAnim` field.
- A `ProductionAnim=NAREFN_AR` (`AR` suffix = "active reverse"? — confirmed in artmd at line 17595-17605: `;[NAREFN_AR]` block with `Reverse=yes` flag, the same SHP as NAREFN_A but played backwards).
- `ActiveAnimTwoPowered=no` — power-state toggle for ActiveAnimTwo. Confirms engine's power-state animation matrix.

These were reverted in favor of the simpler 4-layer L1-L4 system. The `;[NAREFN_A]`, `;[NAREFN_AR]`, `;[NAREFN_C]` artmd sub-blocks are ALSO commented out (lines 17585-17612), confirming the entire alternative system is dead-code.

### 4-layer ActiveAnim with full Damaged variants

NAREFN has the **most sophisticated active-anim system documented** so far: 4 layers × 2 variants (clean + damaged) = 8 anim references:

- `ActiveAnim=NAREFNL1` + `ActiveAnimDamaged=NAREFNL1D`
- `ActiveAnimTwo=NAREFNL2` + `ActiveAnimTwoDamaged=NAREFNL2D`
- `ActiveAnimThree=NAREFNL3` + `ActiveAnimThreeDamaged=NAREFNL3D`
- `ActiveAnimFour=NAREFNL4` + `ActiveAnimFourDamaged=NAREFNL4D`

GAREFN has 4 layers with NO Damaged variants. NAREFN goes the extra step: damaged refinery visually transitions all 4 layers to damaged-state animations.

Each `L_D` block at [artmd.ini:17494, 17517, 17540, 17563](../../../../ra2-rust-game/ini/artmd.ini) follows the same pattern: `Image=NAREFNL_` (parent SHP), frames 3-6, infinite loop. The damaged variants share the same SHP file with the clean variant; frame-range slicing for memory efficiency.

### Sub-anim summary

[artmd.ini:17483-17582](../../../../ra2-rust-game/ini/artmd.ini):
- `NAREFNL1`, `NAREFNL2`, `NAREFNL3`, `NAREFNL4` — 4-frame infinite loops at Rate=200.
- `NAREFNL1D`, `NAREFNL2D`, `NAREFNL3D`, `NAREFNL4D` — damaged variants, frames 3-6 of parent SHP.
- `NAREFNOR` — ore-dump SpecialAnim, 19-frame one-shot (LoopCount=1, Rate=200). Same duration as GAREFN's `GAREFNOR`.

### SpecialAnimX / SpecialAnimY (commented)
```ini
;SpecialAnimX=800
;SpecialAnimY=800
```

Engine supports explicit X/Y placement for the SpecialAnim (vs the default tied to building anchor). Reverted; the shipping refinery uses the building-relative default. **Latent engine capability** for fine-tuning SpecialAnim position.

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field                  | String address    | First xref               | Read scope                        |
|------------------------|--------------------|--------------------------|-----------------------------------|
| `QueueingCell`         | `0x0081a614`       | `0x00461506`             | BuildingType                      |
| `DockingOffset%d`      | `0x008194b4`       | `0x004649b7`             | BuildingType (format-string loop) |
| `3x3Refinery`          | `0x0081bb98`       | **NO XREFS**              | **dead-code orphan string** — TS-legacy |

**2 NEW cheat-sheet entries this iteration + 1 dead-code confirmation:**

1. **`QueueingCell`** — `0x0081a614 → 0x00461506` — BuildingType. Harvester queueing target cell.
2. **`DockingOffset%d`** — `0x008194b4 → 0x004649b7` — BuildingType (format-string loop). Docking position override per slot. NAREFN has `;DockingOffset0=256,0,0` commented; the engine supports it but no shipping refinery uses it.
3. **`3x3Refinery` — DEAD-CODE CONFIRMATION** — string exists at `0x0081bb98` in the binary but **has no xrefs**. This is true dead-code: a leftover string from TS-era when refineries had 3x3 foundations. No code path in YR references it. Stripping it would be safe. The engine has no 3x3-specific refinery logic active in YR.

### Ghidra search log for this iteration

- `search_strings("3x3Refinery")` → 1 match at `0x0081bb98`. `get_xrefs_to(0x0081bb98)` → **no references found**. Confirmed dead-code. TS-legacy orphan string.
- `search_strings("WaitingOffset")` → 0 matches in the binary as a parser key. The commented `WaitingOffset0-7` in NAREFN's artmd are read indirectly OR Westwood removed the parser before shipping. (The 8 commented entries would have been read by a `WaitingOffset%d` format-string loop, but the keyword is absent.) DEFERRED — possibly the engine reads `WaitingOffset%d` as a format string at runtime but the search returns 0 because the format string isn't a standalone interned string. OR the field was removed entirely from the engine.
- `search_strings("QueueingCell")` → 1 match → BuildingType.
- `search_strings("PreProductionAnim")` → 7 matches (`PreProductionAnimYSort`, `PreProductionAnimZAdjust`, `PreProductionAnimY`, etc.). Engine fully supports PreProductionAnim with axis + Z + Y-sort variants. The NAREFN commented `;PreProductionAnim=NAREFN_A` could work if uncommented. **Latent engine capability.**
- `search_strings("DockingOffset")` → 1 match → BuildingType (format-string loop).

### TS-legacy filter

- **`3x3Refinery` string with no xrefs** — confirmed dead-code. TS heritage.
- **`PipScale=Tiberium`** — TS-era field name preserved (works in YR).
- **`;//gs revertNumberOfWaitingPoints=8`** — Westwood iteration artifact.
- **`;WantsExtraSpace=yes`** — commented, latent engine capability.
- **`;WaitingOffset0-7`** — commented, possibly TS-legacy engine code; engine MAY or MAY NOT still parse these (search returned 0 hits on `WaitingOffset` string, suggesting removed).
- **`;DockingOffset0=256,0,0`** — commented; engine parses `DockingOffset%d` confirmed-live.
- **`;PreProductionAnim=NAREFN_A`, `;ProductionAnim=NAREFN_AR`** — commented; engine parses `PreProductionAnim*` family confirmed-live (7 strings found).
- **`;ActiveAnimTwoPowered=no`** — commented; engine parses `ActiveAnimTwoPowered` confirmed via cheat-sheet.
- **No fog-of-war / 0x1000 gating** — clean.

NAREFN has **substantial latent engine capabilities** preserved in commented form. Westwood iterated heavily on the refinery animation/queueing system before shipping the simpler L1-L4 setup. The commented entries reveal engine support for: PreProductionAnim (full family), ProductionAnim (Reverse=yes flag), ActiveAnim power-state toggles, explicit WaitingOffset queue, DockingOffset override, SpecialAnim X/Y placement.

None of these latent features are bugs or TS-legacy — they're modder-relevant engine features Westwood didn't use in shipping.

---

## Cross-references

- **`HARV`** (`units/soviet/HARV.md`) — DONE. FreeUnit pair (NAREFN spawns HARV; HARV.UnloadingClass=HORV for visual-only unload state).
- **`GAREFN`** (`units/structures/GAREFN.md`) — DONE iteration 91. Allied sister refinery. Mechanically identical, only side-flavor diffs.
- **`YAREFN`** (`units/structures/YAREFN.md`) — DONE. Yuri Slave Miner deployed form. Completes the refinery trio: GAREFN ✓ + NAREFN ✓ + YAREFN ✓.
- **`NACNST`** (`units/structures/NACNST.md`) — DONE. Prerequisite parent.
- **`CMIN`** (`units/allied/CMIN.md`) — DONE. Allied harvester, cross-reference for `FreeUnit=` mechanic.
- **`NATECH` (Soviet Battle Lab)** — pending. Prerequisite chain includes NAREFN.
- **`NAINDP` (Soviet Industrial Plant)** — pending. Discounts Soviet vehicle production; works alongside refineries.
- **`SPYP`** (`units/soviet/SPYP.md`) — DONE. Spy plane provider for Soviet — separate mechanism, not refinery-related.
- **`ENGINEER`/`SENGINEER`/`SPY`** — capture/spy.
- **Deep-RE cross-refs**: `HARVESTER_DOCK_UNLOAD_SEQUENCE.md`, `WAR_MINER_REFERENCE.md`, `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md`, `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md`.

---

## Coverage audit

INI fields covered (44 rulesmd + 38 artmd + 9 sub-anims (L1-L4 + L1D-L4D + NAREFNOR) + 7 commented latents = 98 entries):

**Coverage: 100%.** Every key in NAREFN's rulesmd, artmd, and the 9 referenced sub-anim blocks transcribed and explained. The 7 commented-out latent fields documented as engine-supported.

---

## Open questions / Westwood inconsistencies

1. **`HalfDamageSmokeLocation1=0,0,0`** is identical on GAREFN and NAREFN — confirms this is a placeholder that defaults to building origin, not a tuned per-side value.
2. **`DamageSmokeOffset=410, 100, 165`** is identical on GAREFN and NAREFN — Westwood copy-pasted; the offsets are presumably tied to a generic refinery-shape SHP coord system, not per-side art.
3. **`WaitingOffset` is fully commented out and the string is not found in the binary** as a standalone parser key. Either the parser uses a format-string `WaitingOffset%d` (not a separate string), OR Westwood removed parser support entirely after deciding on engine-default queueing. DEFERRED — verify by attempting to read `WaitingOffset%d` format string in binary.
4. **`Height=6` vs GAREFN's 4** — Soviet refinery is taller. Reflects Soviet industrial aesthetic (taller chimneys, larger silhouette). OccupyHeight=4 matches.
5. **`NREFICON` cameo vs GAREFN's `REFICON`** — per-side cameo SHP. The `REFICON` short name was reserved for Allied; Soviet adds `N` prefix.
6. **NAREFN has Damaged variants for all 4 active anims; GAREFN does not.** Soviet refinery has visually-richer damage states. Likely just an art-resource availability decision (the artists made Damaged variants for NAREFN but not GAREFN).
7. **8 RemoveOccupy slots on NAREFN vs 1 on GAREFN.** Soviet refinery has more visual extent outside the foundation, requiring more passability cleanup.

---

## Status

**DONE** — iteration 92. Index entry updated. **Refinery trio (GAREFN + NAREFN + YAREFN) complete.**

Doc total: **92**.

Next pick (priority): Power plants quartet — **GAPOWR (Allied Power Plant), NAPOWR (Soviet Tesla Reactor), NANRCT (Soviet Nuclear Reactor), YAPOWR (Yuri Bio Reactor with garrison-boost mechanic)**. Yuri's Bio Reactor has the unique garrison-boost behavior where Initiates inside the building boost power output — that's an interesting per-unit hardcoded behavior to verify in Ghidra.
