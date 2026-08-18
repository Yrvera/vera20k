# Tiberium / Voxel / Vein Dirty Producer Stock Liveness - Ghidra Research Report

**Address(es):** `0x0055AFB0`, `0x007221B0`, `0x00722C40`, `0x00722440`, `0x00722F00`, `0x0071C730`, `0x00483780`, `0x00487190`, `0x00749F30`, `0x0074B050`, `0x0074CAC2`
**Investigation Mode:** exhaustive-slice for stock YR liveness of already-identified `MarkTerrainDirty` producer categories.
**Claimed Scope:** Standard YR liveness/frequency and dirty timing for ore/gem growth/spread, TIBTRE terrain spawning, VoxelAnimType `IsTiberium` ore placement, and veinhole/vein cleanup.
**Non-Scope:** Full `MarkTerrainDirty` caller matrix, `CellClass::GetRadarColor`, exact minimap pixel composition, full save/load serialization, full VoxelAnim physics, and Rust code changes.
**Confidence:** High for binary gates, dirty call timing, and stock INI defaults; Medium-High for stock map frequency from retail MIX binary-text scan; Medium for exact named-map census because MIX members were not extracted.
**Active in YR:** Yes for Riparius growth/spread, TIBTRE terrain-object spawning, and VoxelAnimType `IsTiberium`; No for stock Cruentus queue growth/spread; Conditional/custom-map for veinhole/vein cleanup.

## 0. Working Notes Gate

Target question: Determine standard YR liveness/frequency for minimap terrain-dirty producers related to ore/gem/tiberium spread, voxel animation tiberium-spawn gates, veinhole/vein cleanup, and TS legacy paths.

Non-goals: Do not redo `GetRadarColor`, full `MarkTerrainDirty` caller matrix, bridge dirty producers, object-dot pixel dirty, or non-radar ore economy mechanics.

Evidence needed to mark COMPLETE: binary owner/caller evidence for each producer class, stock INI/default evidence for each gate, timing of the dirty call relative to mutation, current Rust surface scan, and explicit active-in-YR classification for every material finding.

Stop conditions: Stop after active stock requirements are separated from inactive/legacy producers and implementation handoff names dirty-producing Rust deltas; defer full named-map corpus extraction and unrelated gameplay details.

## 1. Overview

Four minimap terrain-dirty categories matter here. Queue-backed Riparius growth/spread and full-removal reseed are live standard YR systems; TIBTRE01-03 terrain objects are live stock map objects and spawn ore through `TerrainClass::AI`; VoxelAnimType `IsTiberium` is live for stock meteor/gem debris VoxelAnims and calls `MarkTerrainDirty` when it actually places ore; veinhole/vein cleanup code exists and is callable if a veinhole/vein object exists, but stock YR map frequency is effectively not normal skirmish content and should not be imported as a required common-path producer.

The main Rust implication is narrow: minimap terrain dirty must be emitted for live ore placement/removal/spawn producers that actually call `RadarClass::MarkTerrainDirty`, but Rust should not invent vein cleanup as a normal YR skirmish producer.

## 2. Stock Liveness Matrix

| Producer family | Active in YR | Normal frequency | Dirty timing / path | Evidence |
|---|---|---|---|---|
| Riparius queue growth/spread | Yes | Periodic; driven every logic tick, processors fire at per-type intervals when `ScenarioClass+0x34A6` and percentages permit | New germination: `SpreadProcessor -> SpreadTiberium(0) -> PlaceTiberium(type,3) -> AddToGrowthQueue -> OverlayData=3 -> MarkTerrainDirty @ 0x00487368` | `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`; `TiberiumClass::GrowthDriver_AllTypes @ 0x00722C40`; `SpreadDriver_AllTypes @ 0x007221B0`; `PlaceTiberium @ 0x00487190`; `rulesmd.ini:30388-30396` |
| Riparius full removal / reseed | Yes | Common: harvester and ore-damage full removal | Full removal clears overlay/data, recalcs attributes, then calls `MarkTerrainDirty @ 0x00480BEA`; then clears spread bitmaps and reseeds neighbors | `Reduce_Tiberium @ 0x00480A80`; assembly `0x00480BCE..0x00480BEA`; harvester caller `0x0073D450`; `rulesmd.ini [CMIN]` |
| Cruentus queue growth/spread | No for stock queue processors | Stock gems do not naturally grow/spread | Processors return before placement because `GrowthPercentage=0` and `SpreadPercentage=0`; no queue-driven dirty from stock Cruentus | `TiberiumClass::SpreadProcessor @ 0x00722440`; `GrowthProcessor @ 0x00722F00`; `rulesmd.ini:30400-30407` |
| Vinifera/Aboreus queue growth/spread | Conditional | Data exists with positive percentages, but stock map frequency not established in this slot | Same queue mechanisms as Riparius when a map/rules path creates matching type entries | `rulesmd.ini:30413-30433`; queue driver binary evidence above |
| TIBTRE terrain-object spawn | Yes | Common on maps containing TIBTRE; stock retail MIX scan shows many `=TIBTRE01/02/03` map entries in `multimd.mix`, `mapsmd03.mix`, and `expandmd01.mix` | Probability hit starts terrain animation; midpoint calls `SpreadTiberium(1)`; successful new placement reaches `PlaceTiberium` and `MarkTerrainDirty @ 0x00487368` | `TerrainClass::AI @ 0x0071C730`; `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0`; `rulesmd.ini:28109-28152`; retail MIX `rg -a "TIBTRE0[123]"` |
| VoxelAnimType `IsTiberium` | Yes, conditional on VoxelAnim events | Event-driven: meteor/gem debris expiry/landing; not every match tick | `VoxelAnimClass::AI` checks type `+0x300`; meteor branch scans 8 neighbors, non-meteor checks impact cell; successful placement constructs overlay, adds growth queue, sets density `0`, then calls `MarkTerrainDirty @ 0x0074A561` or `0x0074A6F7` | `VoxelAnimClass__AI @ 0x00749F30`; `VoxelAnimTypeClass__ReadINI @ 0x0074B050`; `rulesmd.ini:30690-30774` |
| Veinhole/vein cleanup | Conditional; not normal stock skirmish requirement | Rare/custom-map or TS legacy. Retail MIX binary-text scan found rules/type definitions, but no plaintext map terrain entries like `=VEINTREE`; stock `VEINS` overlay is commented/dummied in YR `rulesmd.ini` | If a VeinholeMonster object exists, constructor/cleanup loops a 5x5 footprint and calls `MarkTerrainDirty @ 0x0074CAC2` before clearing overlay/data for overlay `0x7E` or overlay type `IsVeins` | `VeinholeMonsterClass__Constructor @ 0x0074C9F0`; call site `0x0074CAC2`; `rulesmd.ini:28154-28159`, `28667-28681`, `29846-29852`; retail MIX `rg -a "VEINTREE|VEINHOLE|VEINS"` |

## 3. Core Findings

### 3.1 Queue-backed ore growth/spread is standard YR-active

**Active in YR: Yes.** `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` calls growth first and spread second at `0x0055B4D7`. Both all-type drivers check `ScenarioClass+0x34A6`, then iterate `g_TiberiumClass_Array`.

`TiberiumClass::SpreadDriver_AllTypes @ 0x007221B0` fires a type when its `+0x100/+0x108` spread timer permits. `TiberiumClass::GrowthDriver_AllTypes @ 0x00722C40` uses `+0x11C/+0x124` for growth timing. This is live standard YR tick integration, not TS-only.

Stock `rulesmd.ini` has `[General] TiberiumGrows=yes` and `TiberiumSpreads=yes`, and `[Riparius] Growth=2200`, `GrowthPercentage=.06`, `Spread=2200`, `SpreadPercentage=.06`. Therefore Riparius queue producers are active in ordinary maps that contain ore.

Dirty timing for new queue spread is after overlay construction/growth-queue insertion and density write in `CellClass::PlaceTiberium @ 0x00487190`; the direct dirty call is `0x00487368 CALL 0x006551C0`. Partial existing density growth does not call `MarkTerrainDirty`.

### 3.2 Stock gems are active data but not queue-spread producers

**Active in YR: No for stock queue growth/spread; Conditional as harvestable/resource data.** `[Cruentus]` has `GrowthPercentage=0` and `SpreadPercentage=0`. `TiberiumClass::SpreadProcessor @ 0x00722440` exits when `SpreadPercentage <= 0.0`; `GrowthProcessor @ 0x00722F00` exits when `GrowthPercentage <= 0.0`.

Do not treat "gem is a TiberiumClass" as "gem naturally grows/spreads in stock YR." It participates in other systems, including chain/debris paths, but not queue-driven natural growth/spread by stock defaults.

### 3.3 TIBTRE terrain-object spawning is stock YR-active and frequent on maps containing ore trees

**Active in YR: Yes.** `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` reads `SpawnsTiberium` at `+0x2B1`, `IsAnimated` at `+0x2B3`, `AnimationRate` at `+0x2A0`, and `AnimationProbability` at `+0x2A4`. Stock `TIBTRE01..03` set `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`.

`TerrainClass__AI @ 0x0071C730` rolls probability only when idle, starts an animation timer, and when the active animation reaches half the SHP frame count it resets frame/timer state, resolves the source cell, and calls `CellClass::SpreadTiberium(1)`. The force flag bypasses source spread gates, but target placement still goes through `CanPlaceTiberium` and `PlaceTiberium(type, 3)`.

Timing for minimap dirty is delayed: no dirty call on probability-hit tick; dirty happens only on successful midpoint placement, via `PlaceTiberium`'s germination branch and `MarkTerrainDirty @ 0x00487368`.

Retail-data liveness: binary text scan of retail MIX files found many map terrain entries assigning coordinates to `TIBTRE01`, `TIBTRE02`, and `TIBTRE03` in `multimd.mix`, `mapsmd03.mix`, and `expandmd01.mix`. This is enough to classify TIBTRE as a stock normal-content producer.

### 3.4 VoxelAnimType `IsTiberium` is live, but event-driven

**Active in YR: Yes, conditional.** `VoxelAnimTypeClass__ReadINI @ 0x0074B050` reads `IsTiberium` into `VoxelAnimTypeClass+0x300`; constructor default is false. Stock rules set `IsTiberium=true` on `CRYSTAL01`, `CRYSTAL02`, `METEOR02`, and `PEBBLE`.

`VoxelAnimClass__AI @ 0x00749F30` reads `type+0x300` after the duration/landing branch. If true and not rejected by water/ground-height gates:

- meteor branch scans the 8 adjacent cells around the landing cell;
- non-meteor branch checks the impact cell;
- each successful candidate calls `CanPlaceTiberium`, constructs a new overlay from the current tiberium type image range, calls `TiberiumClass::AddToGrowthQueue`, writes `OverlayData=0`, then calls `RadarClass::MarkTerrainDirty`.

Dirty timing: the dirty call is after overlay construction, growth-queue add, and `OverlayData=0`. The verified direct dirty sites are `0x0074A561` for the multi-cell meteor-style loop and `0x0074A6F7` for the single impact-cell path.

### 3.5 Veinhole/vein cleanup is conditional legacy, not a normal stock minimap requirement

**Active in YR: Conditional.** The binary path is real: `VeinholeMonsterClass__Constructor @ 0x0074C9F0` loops offsets `-2..2` in both axes. If a cell has overlay index `0x7E` or an overlay type with byte `+0x2AE` (`IsVeins`) set, it calls `RadarClass::MarkTerrainDirty @ 0x0074CAC2`, then clears `OverlayTypeIndex=-1` and `OverlayData=0`.

Stock YR data keeps remnants: `[TerrainTypes] 49=VEINTREE`, `[VEINTREE] IsVeinhole=true`, `[VEINHOLE] IsVeinholeMonster=true` and `IsVeins=true`, and `[VEINHOLEDUMMY] IsVeins=true`. But the `[VEINS]` overlay section is commented/dummied in YR `rulesmd.ini`, and retail MIX binary-text scan found veinhole/vein strings in rules/type definitions, not normal plaintext map terrain entries like `123045=VEINTREE`. By contrast, the same scan found many `=TIBTRE01/02/03` map entries.

Therefore this producer should be treated as a conditional/custom-map or TS legacy-compatible code path, not as a high-frequency stock YR minimap dirty source. If Rust later supports custom maps/rules that instantiate VEINTREE/VEINHOLE, the verified cleanup semantics become required for that configuration.

## 4. INI / Retail Data Checked

| Key / section | Stock YR value | Active in YR | Effect |
|---|---|---|---|
| `[General] TiberiumGrows` / `TiberiumSpreads` | `yes` / `yes` | Yes by default | Enables standard ore growth/spread scenario flags. |
| `[Riparius] GrowthPercentage` / `SpreadPercentage` | `.06` / `.06` | Yes | Queue processors run when interval and heap permit. |
| `[Cruentus] GrowthPercentage` / `SpreadPercentage` | `0` / `0` | No for queue growth/spread | Stock gems do not naturally grow/spread. |
| `[Vinifera]`, `[Aboreus]` percentages | `.06` / `.06` | Conditional | Data exists; stock map usage not proven here. |
| `[TIBTRE01..03]` | `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`, `Immune=yes` | Yes | Terrain-object midpoint ore spawning. |
| `[VoxelAnims] CRYSTAL01/02`, `METEOR02`, `PEBBLE` | `IsTiberium=true` | Conditional | VoxelAnim impact ore placement and radar dirty. |
| `[VEINTREE]` | `IsVeinhole=true` | Conditional | Type exists; no normal stock map entry proven. |
| `[VEINHOLE]`, `[VEINHOLEDUMMY]` | `IsVeins=true` / `IsVeinholeMonster=true` | Conditional | Cleanup branch recognizes these overlays if present. |
| `;gs[VEINS]` in `rulesmd.ini` | commented/dummied | No as stock YR overlay family | Do not implement normal YR veins from this commented section. |

## 5. Current Rust Implementation Status

| Rust surface | Observed status | Delta |
|---|---|---|
| `src/sim/world/mod.rs::mark_radar_terrain_dirty_cells` | Generic dirty list exists, but known producer coverage is bridge-oriented. | Needs live ore/tiberium producer integration, not bridge-only generation. |
| `src/render/minimap.rs::apply_bridge_terrain_dirty_cells` | Dirty consumer is bridge-specific. | Native dirty cells should re-run current terrain/overlay radar color, not only bridge overlay colors. |
| `src/sim/ore_growth.rs` | RA1-style scan/reservoir growth/spread. | Does not model YR per-type queues, positive/zero percentages, or native dirty timing. |
| `src/sim/terrain_spawn.rs` | TIBTRE spawner state exists and captures delayed terrain animation concepts. | Still needs exact native placement side effects and radar terrain dirty integration through `PlaceTiberium`-equivalent path. |
| `src/sim/combat/mod.rs` | Comment notes vein destruction is not implemented. | Correct for normal stock YR; do not add common-path vein cleanup without custom-map/rules activation. |
| `src/map/overlay_types.rs` | Parses `IsVeins` / `IsVeinholeMonster`. | Data parse exists; should remain inert unless overlays/objects actually appear. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes gate | verified | Section 0 | none |
| Logic tick growth-before-spread integration | verified | `0x0055AFB0` decompile | none |
| Spread driver liveness gate | verified | `0x007221B0`; `ScenarioClass+0x34A6`; INI defaults | exact writer chain for `+0x34A6` not re-traced |
| Growth driver liveness gate | verified | `0x00722C40`; INI defaults | exact growth multiplier source out-of-scope |
| Riparius active percentages | verified | `rulesmd.ini:30388-30396`; processors `0x00722440`, `0x00722F00` | none |
| Cruentus stock zero percentages | verified | `rulesmd.ini:30400-30407`; processor early exits | none |
| TIBTRE terrain-object spawn liveness | verified | `0x0071DEA0`, `0x0071C730`, `rulesmd.ini:28109-28152`, retail MIX scan | exact named-map count deferred |
| TIBTRE dirty timing | verified | `TerrainClass::AI -> SpreadTiberium(1) -> PlaceTiberium -> 0x00487368` | exact direction labels deferred elsewhere |
| VoxelAnimType `IsTiberium` parser/default | verified | `0x0074B050`, `VoxelAnimTypeClass__Constructor`, `rulesmd.ini:30690-30774` | none |
| VoxelAnim dirty timing | verified | `VoxelAnimClass__AI`, direct dirty sites `0x0074A561`, `0x0074A6F7` | exact event trigger frequency per map/script not counted |
| Veinhole cleanup dirty branch | verified | `0x0074CAC2` assembly/decompile | exact custom-map use not counted |
| Stock YR vein normality | touched-not-exhausted | `rulesmd.ini` commented `[VEINS]`; retail MIX binary-text scan | packed member extraction could build exact census |
| Rust dirty producer coverage | verified source scan | `src/sim/world/mod.rs`, `src/render/minimap.rs`, `src/sim/ore_growth.rs`, `src/sim/terrain_spawn.rs` | implementation contract/fix pass |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is queue-backed ore growth/spread TS-only? -> No; live standard YR tick path calls growth then spread, and stock Riparius percentages are positive.` (evidence: `0x0055AFB0`, `0x007221B0`, `0x00722C40`, `rulesmd.ini [Riparius]`)
- `[RESOLVED] OQ-02 - Do stock gems naturally grow/spread? -> No through queue processors; Cruentus percentages are zero, so processors exit.` (evidence: `0x00722440`, `0x00722F00`, `rulesmd.ini [Cruentus]`)
- `[RESOLVED] OQ-03 - Does TIBTRE dirty radar on probability hit? -> No; dirty only follows successful midpoint placement through `PlaceTiberium`.` (evidence: `0x0071C730`, `0x00487190`, `0x00487368`)
- `[RESOLVED] OQ-04 - Are TIBTRE01-03 stock YR data? -> Yes; registered terrain types and many retail MIX map entries contain `=TIBTRE01/02/03`.` (evidence: `rulesmd.ini [TerrainTypes] 46-48`, retail MIX `rg -a`)
- `[RESOLVED] OQ-05 - Is VoxelAnimType `IsTiberium` a separate live field? -> Yes; parser writes `+0x300`, consumer is `VoxelAnimClass::AI`.` (evidence: `0x0074B050`, `0x00749F30`)
- `[RESOLVED] OQ-06 - When does VoxelAnim dirty terrain? -> After successful overlay construction/growth-queue add/density-zero write, at `0x0074A561` or `0x0074A6F7`.` (evidence: `VoxelAnimClass__AI`, assembly contexts)
- `[RESOLVED] OQ-07 - Is veinhole cleanup code real? -> Yes; `0x0074CAC2` marks dirty before clearing overlay/data in a 5x5 footprint.` (evidence: `VeinholeMonsterClass__Constructor`)
- `[RESOLVED] OQ-08 - Is veinhole cleanup a normal stock skirmish minimap producer? -> No evidence for normal frequency; stock YR keeps definitions but comments/dummies `[VEINS]`, and retail MIX scan found no map `=VEINTREE` entries while finding many TIBTRE entries.` (evidence: `rulesmd.ini`, retail MIX `rg -a`)
- `[RESOLVED] OQ-09 - Should Rust implement normal vein dirty producers now? -> No for stock normal parity; keep parsed data inert unless custom maps/rules instantiate it.` (evidence: liveness split above; Rust `src/sim/combat/mod.rs` note)
- `[RESOLVED] OQ-10 - Which dirty producers are implementation-critical for minimap parity? -> full removal, new placement from queue/TIBTRE/VoxelAnim, and growth/spread queue side effects that call dirty.` (evidence: direct dirty sites `0x00480BEA`, `0x00487368`, `0x0074A561`, `0x0074A6F7`)
- `[DEFERRED] OQ-11 - Exact named stock map count for every TIBTRE/VEINTREE occurrence.` (category: bounded-cost-too-high; reason: requires MIX member extraction rather than binary-text scan; next-step-if-pursued: extract map INIs and count `[Terrain]` values)
- `[DEFERRED] OQ-12 - Exact map/script frequency of VoxelAnim meteor/gem debris events.` (category: requires-different-system-context; reason: producer liveness is proven but event source census belongs to weapon/superweapon/map-trigger research; next-step-if-pursued: trace `VoxelAnimClass` creation producers by type)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Riparius queue spread and TIBTRE successful placement call `PlaceTiberium(type,3)`, then dirty radar terrain only on new-cell germination. | `0x0055AFB0`, `0x007221B0`, `0x0071C730`, `0x00487190`, dirty call `0x00487368`; `rulesmd.ini [Riparius]`, `[TIBTRE01..03]` | Missing/mismatched: Rust growth/spread is scan-based and minimap dirty is bridge-focused. | `src/sim/ore_growth.rs`, `src/sim/terrain_spawn.rs`, `src/sim/world/mod.rs`, `src/render/minimap.rs` | Route successful YR-equivalent new ore placement through a terrain dirty producer after overlay/data mutation, not on probability hit or partial density growth. | A TIBTRE probability hit starts animation with no minimap dirty; 33 stock ticks later, a successful neighbor placement queues exactly that cell for radar terrain refresh. Proposed test: `minimap_dirty_tibtree_midpoint_success_not_probability_hit` | Do not dirty minimap for failed placement or every density write. |
| Full ore removal clears overlay/data, recalcs, calls `MarkTerrainDirty`, then reseeds spread queues. | `Reduce_Tiberium @ 0x00480A80`; assembly `0x00480BCE..0x00480BEA`; `REDUCE_TIBERIUM...` report | Miner/resource removal does not publish native terrain dirty/reseed semantics. | miner harvest, combat ore damage, `src/sim/overlay_grid.rs`, future tiberium queue module | Emit radar terrain dirty on full removal only, after overlay clear/recalc-equivalent state is applied. | A full harvest of a visible Riparius cell clears overlay and queues that cell for minimap terrain refresh; partial harvest does not. Proposed test: `minimap_dirty_full_ore_removal_not_partial_density_reduction` | Do not model full removal as only a resource-node stock decrement. |
| VoxelAnimType `IsTiberium` places ore on impact/expiry and calls `MarkTerrainDirty` after placement. | `VoxelAnimTypeClass__ReadINI @ 0x0074B050`; `VoxelAnimClass__AI @ 0x00749F30`; dirty calls `0x0074A561`, `0x0074A6F7`; `rulesmd.ini [CRYSTAL01/02]`, `[METEOR02]`, `[PEBBLE]` | Rust has no verified VoxelAnim ore placement dirty producer in the scanned surfaces. | VoxelAnim simulation/effects surface if/when implemented, `src/sim/world/mod.rs`, minimap dirty API | Implement VoxelAnim ore placement as a conditional producer tied to actual VoxelAnim events, with density zero and growth queue side effect before dirty. | A stock `PEBBLE` or `CRYSTAL01` landing that passes `CanPlaceTiberium` creates ore and queues the affected terrain cell; water/failed candidates do not. Proposed test: `minimap_dirty_voxelanim_tiberium_successful_impact_only` | Do not confuse AnimTypeClass `IsTiberium` with VoxelAnimTypeClass `IsTiberium`. |
| Veinhole/vein cleanup marks dirty only if a veinhole/vein object/overlay is actually present; it is not a normal stock YR skirmish producer. | `0x0074CAC2`; `rulesmd.ini [VEINTREE]`, `[VEINHOLE]`, commented `[VEINS]`; retail MIX text scan | Rust parses vein flags but does not implement vein cleanup. | `src/map/overlay_types.rs`, `src/sim/combat/mod.rs`, future custom-map overlay mutation | Leave stock path inert for normal YR; if custom maps instantiate VEINTREE/VEINHOLE, implement the 5x5 dirty-before-clear cleanup. | A custom-map VEINHOLE object clearing nearby vein overlays dirties each cleared cell before removal; a normal stock map without VEINTREE does not create any vein dirty events. Proposed test: `minimap_dirty_veinhole_cleanup_only_when_vein_content_exists` | Do not import TS vein spread/damage as common YR behavior. |

## 9. Negative Facts / Do Not Do

- Do not treat all `TiberiumClass` data as stock-natural spread. Active in YR: No for stock Cruentus queue spread/growth because both percentages are zero (`rulesmd.ini:30404-30407`; `0x00722440`, `0x00722F00`).
- Do not dirty minimap on TIBTRE probability-hit tick. Active in YR: No; the dirty call is delayed until successful midpoint `PlaceTiberium` germination (`0x0071C730`, `0x00487368`).
- Do not confuse `AnimTypeClass+0x358 IsTiberium` with `VoxelAnimTypeClass+0x300 IsTiberium`. Active in YR: both can be live, but this dirty producer is the VoxelAnim field (`0x0074B050`, `0x00749F30`).
- Do not implement TS-style veins as normal YR skirmish minimap producers. Active in YR: Conditional only when vein/veinhole content exists; stock YR `[VEINS]` is commented/dummied and retail map scan did not show normal `=VEINTREE` map entries.
- Do not collapse new placement, partial growth, and full removal into one "ore changed" dirty trigger. Active in YR: dirty is called on new germination and full removal, not partial density growth (`0x00487368`, `0x00480BEA`).

## 10. Stale Docs / Follow-up Docs

- `docs/research/combat/systems/chain_reaction.md`: replace "no `IsVeins=yes` or `IsVeinholeMonster=yes` overlay in retail rulesmd" with: "YR `rulesmd.ini` retains `[VEINHOLE] IsVeinholeMonster=true, IsVeins=true` and `[VEINHOLEDUMMY] IsVeins=true`, while the ordinary `[VEINS]` overlay is commented/dummied. Treat vein cleanup as conditional legacy/custom-map content, not as a normal stock skirmish producer."
- `docs/research/TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md`: replace the row claiming `[TIBTRE03] Armor/IsVeinhole/Strength` with: "`[VEINTREE]` has `Armor=None`, `IsVeinhole=true`, and `Strength=1000`; stock `[TIBTRE03]` has `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`, and `Immune=yes`, but no `IsVeinhole=true` in `rulesmd.ini`."

## 11. Remaining Uncertainty

- Exact named-map census for VEINTREE/VEINHOLE requires extracting map members from MIX archives. Binary-text scan is strong enough to classify TIBTRE as common and VEINTREE as not observed in normal plaintext map entries, but it is not a full packed-map database.
- Exact VoxelAnim event frequency per stock map/script/weapon remains a separate producer-census task. This report proves the dirty producer is live when those events occur.
- Exact direction-table labels for spread/voxel neighbor loops are intentionally deferred to direction-table research; the dirty-producing call timing does not depend on naming index 0.

## Sources

- Ghidra decompiled: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`, `TiberiumClass__SpreadDriver_AllTypes @ 0x007221B0`, `TiberiumClass__GrowthDriver_AllTypes @ 0x00722C40`, `TiberiumClass__SpreadProcessor @ 0x00722440`, `TiberiumClass__GrowthProcessor @ 0x00722F00`, `TerrainClass__AI @ 0x0071C730`, `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0`, `CellClass__SpreadTiberium @ 0x00483780`, `CellClass__PlaceTiberium @ 0x00487190`, `CellClass__CanPlaceTiberium @ 0x004838E0`, `VoxelAnimTypeClass__Constructor`, `VoxelAnimTypeClass__ReadINI @ 0x0074B050`, `VoxelAnimClass__AI @ 0x00749F30`, `VeinholeMonsterClass__Constructor @ 0x0074C9F0`.
- Ghidra assembly contexts: `0x00487368`, `0x00480BEA`, `0x0074A561`, `0x0074A6F7`, `0x0074CAC2`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Retail data scan: `rg -a "TIBTRE01|TIBTRE02|TIBTRE03"` and `rg -a "VEINTREE|VEINHOLE|VEINS"` over `C:/Users/enok/Documents/Command and Conquer Red Alert II/*.mix`.
- Prior docs referenced: `MARKTERRAINDIRTY_FULL_CALLER_MATRIX_GHIDRA_REPORT.md`, `RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`, `PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md`, `REDUCE_TIBERIUM_FULL_REMOVAL_SIDE_EFFECTS_AND_RETURN_VALUE_GHIDRA_REPORT.md`, `TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md`, `ANIMTYPECLASS_TIBERIUM_FLAG_CONSUMERS_GHIDRA_REPORT.md`, `VOXELANIMCLASS_GHIDRA_REPORT.md`, `TERRAIN_OBJECT_LIGHT_KEYS_AND_LIGHTSOURCE_OWNERSHIP_GHIDRA_REPORT.md`, `traces/RIPARIUS_GROWTH_SPREAD_QUEUE_STANDARD_YR_TRACE.md`, `traces/TIBTREE_MIDPOINT_FORCE_SPREAD_DENSITY3_TRACE.md`.
- Rust scanned: `src/sim/world/mod.rs`, `src/render/minimap.rs`, `src/sim/ore_growth.rs`, `src/sim/terrain_spawn.rs`, `src/sim/combat/mod.rs`, `src/map/overlay_types.rs`, `src/rules/terrain_object_type.rs`.
