# MarkTerrainDirty Full Caller Matrix - Ghidra Research Report

**Address:** `RadarClass::MarkTerrainDirty @ 0x006551C0`  
**Investigation Mode:** exhaustive-slice for direct xref/caller coverage; bridge internals are category-only by user scope.  
**Claimed Scope:** All direct Ghidra xrefs to `0x006551C0`, their call-site categories, branch gates, terrain/radar source mutation, active YR status, and Rust-facing dirty event categories.  
**Non-Scope:** `MarkTerrainDirty` internals, `ClearBackground`, pixel dirty queue internals, bridge collapse/repair mechanics already settled, spy-satellite reveal, gap/shroud special effects, and Rust implementation patches.  
**Confidence:** High for direct xref list and non-bridge branch gates; Medium-High for bridge caller categorization because bridge mechanics were intentionally not re-investigated.  
**Active in YR:** Yes/Conditional by caller. The dirty primitive is live in ordinary radar update; individual producers depend on ore growth/spread, wall damage/sell, terrain limbo, bridge mutation, vein/voxel special flags, or map-editor gates.

## 0. Investigation Gate

Target question: enumerate the full live caller matrix for `RadarClass::MarkTerrainDirty @ 0x006551C0`: all xrefs/callers, branch conditions where the call happens, what terrain/radar source changes, active YR status, and Rust-facing dirty event categories.

Non-goals: do not redo dirty queue internals, `ClearBackground`, bridge repair/collapse details, spy-satellite reveal, or gap/shroud special effects beyond caller category.

Evidence needed to mark COMPLETE: `get_function_xrefs` for `0x006551C0`; assembly context around every call site; decompile/assembly evidence for each non-bridge caller and representative bridge caller groups; Rust touchpoint scan for dirty event categories.

Stop conditions: stop before mutating Ghidra, renaming functions, editing Rust, or expanding into the already-claimed spy/gap slots. If a caller belongs to a large settled bridge subsystem, list the call sites and branch gate class without redoing bridge mechanics.

## 1. Overview

`MarkTerrainDirty` is not just an ore hook. Ghidra returns 72 direct call instructions grouped into 38 caller functions. The callers fall into these categories: ore/tiberium placement/removal/spawn, wall overlay destruction and wall cleanup, terrain object limbo/removal, bridge state/tile mutation, TS/YR vein legacy cleanup, voxel animation tiberium spawning, and a generic cell repaint helper used by area damage/building placement call chains.

For Rust, the important result is that the current bridge-only radar terrain dirty publisher covers only one category. Native also dirties radar terrain for full ore removal, new ore placement, wall removal and wall-neighbor cleanup, terrain object limbo, sellable wall removal, voxel-spawned ore, and conditional vein/legacy overlay changes.

## 2. Full Direct Xref Matrix

`get_function_xrefs(address=0x006551C0, limit=100)` returned these direct callers. "Call sites" are the exact direct `CALL 0x006551C0` instruction addresses.

| Category | Caller | Call sites | Branch condition for call | Terrain/radar source changed | Active in YR |
|---|---|---:|---|---|---|
| Ore full removal | `CellClass::Reduce_Tiberium` | `0x00480BEA` | `param_2 > 0`, overlay maps to tiberium, and removal amount is enough to clear the cell; partial density reduction does not call. | Clears `OverlayTypeIndex=-1`, `OverlayData=0`, recalcs attributes, then dirties the ore cell. | Yes: harvesters/slaves/warhead ore damage can reach full removal. |
| Wall final removal | `CellClass::DestroyOverlay` | `0x00480F27` | Existing overlay is a wall (`OverlayType+0x2A8`), damage/random gates pass, and damage reaches final removal or forced `-1`; partial wall damage returns before this call. | Clears wall overlay, owner/ref fields, overlay data, recalcs attributes, zone orphaning, then dirties the destroyed wall cell. | Yes: wall damage/destruction is live. |
| Wall neighbor cleanup | `CellClass::PostDestructionWallCleanup` | `0x004807C2` | Loop over self plus four cardinal entries; call happens before the wall-overlay check for that visited cell. Caller may pass `flag`, but when `param_2==0` all five entries are processed. | Recomputes wall connectivity or auto-destroys isolated damaged wall pieces after a nearby wall vanishes; dirties each visited cell's radar terrain. | Yes: called by wall destruction and sell-wall path. |
| Generic terrain repaint helper | `FUN_00486E70` | `0x00486FE3` | Helper itself has no branch gate after entry; callers include area damage, building map occupation, and other cell-change paths. | Dirties tactical rect and radar terrain for `param_1 + 0x24`; mutation occurs in caller. | Conditional: active when its caller mutates a cell in gameplay. |
| Bridge low destroy walker | `MapClass::DestroyBridgeWalker_NS_Low`, `EW_Low`, `NS_High`, `EW_High` | `0x0057C004`, `0x0057C012`, `0x0057C020`, `0x0057C5C4`, `0x0057C5D2`, `0x0057C5E0`, `0x0057D27A`, `0x0057D288`, `0x0057D296`, `0x0057D845`, `0x0057D853`, `0x0057D861` | Destroy walker reaches damaged-to-destroyed-anchor branch; healthy-to-damaged branches do not use these three calls. | Marks the three bridge cells whose overlay changes to destroyed-anchor state. | Yes: bridge damage/collapse is live; mechanics settled elsewhere. |
| Bridge apply-destruction walkers | `MapClass::ApplyBridgeDestruction_NS_Low`, `EW_Low`, `NS_High`, `EW_High` | `0x0057E1EC`, `0x0057E21E`, `0x0057E24C`, `0x0057E6E8`, `0x0057E712`, `0x0057E744`, `0x0057EC49`, `0x0057EC7B`, `0x0057ECA9`, `0x0057F155`, `0x0057F17F`, `0x0057F1B1` | Per bridge variant, branch reaches tile application around damaged/destroyed span; calls mark computed bridge/ramp cells. | Dirties bridge cells affected by applied destruction propagation. | Yes: bridge collapse propagation is live; detailed mechanics out-of-scope. |
| Overlay flood propagation | `MapClass::SetOverlayAndPropagate` | `0x0056EC8E` | Target cell exists or fallback cell, and `this->IsoTileTypeIndex != param_2`; recursion visits neighbors with old tile type. | Changes `IsoTileTypeIndex`, recalcs attributes, marks the changed cell, and propagates to neighbors sharing the old type. | Yes/Conditional: used by bridge/ramp tile propagation and other map overlay propagation paths. |
| Bridge pavement toggle | `MapClass::ToggleBridgePavement` | `0x0056EADD` | For non-recursive entry, skips invalid `0xffff`/`0xff` and non-bridge pavement; then only calls if bit `Flags >> 13 & 1` differs from requested value. | Toggles cell flag bit `0x2000` and recurses over contiguous same-tile bridge pavement cells. | Yes: bridge pavement update path is live. |
| Bridge low ramp/edge update | `MapClass::UpdateRamp_NS_CollapseA_Low`, `NS_CollapseB_Low`, `EW_CollapseA_Low`, `EW_CollapseB_Low`, `UpdateBridgeEdgeTiles_Low` | `0x0056EFF9`, `0x0056F399`, `0x0056F95D`, `0x0056FD2D`, `0x00571014` | Variant-specific bridge ramp/edge state branch clears overlay/data or changes ramp tile; each call follows a bridge cell mutation. | Dirties low-bridge ramp/edge cells after overlay/data/bridge-flag mutation. | Yes: low bridge collapse/update is live. |
| Bridge direction setter NWSE | `CellClass::SetBridgeDirection_NWSE` | `0x0047E556`, `0x0047E631`, `0x0047E6F7`, `0x0047E82A` | Function always marks the anchor and three related cells after bridge direction flags/data are written; if `param_3==0`, `CellClass::BlowUpBridge` runs before the dirty call. | Writes bridge direction flags/links and `OverlayData`; marks each affected bridge cell. | Yes: bridge load/damage/repair paths call this helper. |
| Bridge high ramp/edge update | `MapClass::UpdateRamp_NS_CollapseA_High`, `NS_CollapseB_High`, `EW_CollapseA_High`, `EW_CollapseB_High`, `UpdateBridgeEdgeTiles_High` | `0x005724E9`, `0x00572889`, `0x00572E4D`, `0x0057321D`, `0x00576734` | Variant-specific high-bridge ramp/edge branch clears overlay/data or changes ramp tile; each call follows cell mutation. | Dirties high-bridge ramp/edge cells after overlay/data/bridge-flag mutation. | Yes: high bridge collapse/update is live. |
| Bridge direction setter NESW | `CellClass::SetBridgeDirection_NESW` | `0x0047E126`, `0x0047E201`, `0x0047E2C7`, `0x0047E3FA` | Same structure as NWSE: marks anchor and three related cells after flag/data writes; `param_3==0` calls `BlowUpBridge` before dirtying. | Writes bridge direction flags/links and `OverlayData`; marks each affected bridge cell. | Yes: bridge load/damage/repair paths call this helper. |
| Sell wall/building overlay | `HouseClass::Sell_Building_At_Cell` | `0x004FD006` | Cell valid, has overlay and owner, selling house is allowed to sell, overlay type is wall, matching `BuildingType+0xE54` found, and type is not forbidden at `+0x1579`; then clears overlay fields. | Removes sellable wall/building overlay from a cell and dirties radar terrain after `PostDestructionWallCleanup`. | Yes/Conditional: live for sellable wall overlays owned by a sell-capable house. |
| Ore/gem spread shrink helper | `FUN_00485590` | `0x00485778`, `0x00485A1D` | First call after current cell overlay/data is cleared or set based on `FUN_00485390`; second call inside cardinal-neighbor loop for eligible `OverlayTypeIndex==0x7E`, flat cells, and data `<0x30`. | Mutates overlay `0x7E` / ore-gem spread state and dirties current/neighbor terrain cells. | Conditional: tied to ore/gem spread legacy path; prior doc identifies active ore/gem spread caller, but exact stock-trigger frequency was not expanded here. |
| Ore/gem spread grow helper | `FUN_00485AF0` | `0x00485C96`, `0x0048627E` | Entry requires `FUN_00485460()!=0`; first call occurs before setting current cell `OverlayTypeIndex=0x7E`; second call occurs for neighbor cells that are empty or allowed overlay targets, with slope-specific frame/data selection. | Places or changes overlay `0x7E` spread state, sets `OverlayData`, recalcs attributes, and dirties current/neighbor terrain. | Conditional: ore/gem spread path; branch-gated by helper and cell eligibility. |
| New tiberium placement | `CellClass::PlaceTiberium` | `0x00487368` | Only the `CanPlaceTiberium==true` branch after constructing a new overlay and setting density. Existing-tiberium density growth branch does not call `MarkTerrainDirty`. | Creates a new tiberium overlay, adds growth queue entry, writes density, then dirties terrain. | Yes: tiberium spread and initial placement are live when rules/map allow. |
| Bridge tile footprint replacement | `FUN_00581140` | `0x005814E3`, `0x00581BF5` | Function branches on `IsoTileTypeIndex == DAT_00ABC2C8` or `DAT_00ABC2C8+1`, iterates nonzero tile footprint cells in playfield, recalcs attributes, stops targeting, then marks each cell. | Dirties cells affected by bridge tile image/zone replacement footprint. | Yes/Conditional: bridge tile replacement path; not re-investigated beyond matrix. |
| Terrain object limbo | `TerrainClass::Limbo` | `0x0071CA60` | After `ObjectClass::Conceal` and cell attribute recalc; call is gated by `g_MapEditorMode == 0`. | Removes/conceals terrain object from cell, updates zone, then dirties radar terrain. | Yes in gameplay; No in map editor. |
| Veinhole/vein cleanup | `VeinholeMonsterClass__Constructor` label at `0x0074CAC2` | `0x0074CAC2` | Loops `x/y = -2..2`; if cell overlay is `0x7E` or overlay type has `+0x2AE` vein flag, marks dirty before clearing overlay/data. | Clears vein/legacy overlay around veinhole object and dirties each cleared cell. | Conditional: TS/YR legacy vein/veinhole content; active if a veinhole/vein object exists. Standard YR stock frequency not proven here. |
| Voxel animation tiberium spawn | `VoxelAnimClass::AI` | `0x0074A561`, `0x0074A6F7` | Death/impact branch with VoxelAnimType flag at `+0x300` set and not water/ground-height rejected. One path scans 8 adjacent cells; one path affects the impact cell. | Constructs tiberium overlay, adds growth queue, sets density zero, and marks radar terrain. | Conditional: active for voxel anim types configured to spawn tiberium. |
| Bridge repair walkers | `MapClass::RepairBridgeWalker_NS_Low`, `EW_High`, `NS_High`, `EW_Low` | `0x0057FA8E`, `0x0057FACA`, `0x0057FB04`, `0x005809F1`, `0x00580A2D`, `0x00580A64`, `0x005804CA`, `0x00580506`, `0x00580540`, `0x0057FFA5`, `0x0057FFE1`, `0x00580018` | Repair walker reaches destroyed-anchor restoration branch; marks computed bridge cells as overlay/data are restored. | Dirties repaired bridge cells. | Yes: bridge repair hut/engineer repair is live; mechanics settled elsewhere. |

## 3. Category Details

### 3.1 Ore and tiberium

- Full removal: `CellClass::Reduce_Tiberium @ 0x00480A80` marks terrain only after clearing the overlay and recalculating attributes. Partial density reduction writes `OverlayData` and returns without radar terrain dirty. Active in YR: Yes. Evidence: decompile and assembly `0x00480BE1..0x00480BEA`.
- New placement: `CellClass::PlaceTiberium @ 0x00487368` marks only the new-overlay branch. The existing-tiberium growth branch can change density and dirty tactical screen/add spread queue, but has no `MarkTerrainDirty` xref. Active in YR: Yes. Evidence: decompile at `0x00487368`.
- Voxel-spawned ore: `VoxelAnimClass::AI` has two call sites under VoxelAnimType `+0x300` tiberium-spawn gate. Active in YR: Conditional. Evidence: `0x0074A561`, `0x0074A6F7`.

### 3.2 Walls and overlay destruction

- `DestroyOverlay` marks only final wall removal, not partial wall damage. Active in YR: Yes. Evidence: `0x00480F27`.
- `PostDestructionWallCleanup` marks self/cardinal cleanup cells before checking whether each visited cell is a wall, so neighbor cells can be terrain-dirtied even when no wall remains there. Active in YR: Yes. Evidence: `0x004807C2`.
- `HouseClass::Sell_Building_At_Cell` clears a sellable wall overlay and marks the sold cell after `PostDestructionWallCleanup`. Active in YR: Conditional on sellable owned wall/building overlay. Evidence: `0x004FD006`.

### 3.3 Terrain objects and generic cell repaint

- `TerrainClass::Limbo` calls only when `g_MapEditorMode == 0`; map editor conceal does not mark radar terrain through this branch. Active in YR: Yes in gameplay, No in map editor. Evidence: `0x0071CA60`.
- `FUN_00486E70` is a pure dirty helper around `param_1+0x24`; direct callers include `Apply_area_damage`, `BuildingClass__Place_OccupyMap`, `FUN_006B6080`, and two call sites in `FUN_0043F180`. Active in YR: Conditional by caller. Evidence: direct call `0x00486FE3`; xrefs to helper.

### 3.4 Bridge mutation

Bridge calls dominate the raw xref count. They are still one Rust category for this report: bridge tile/state/ramp/edge/repair mutations must emit native cells, but the repair/collapse mechanism is already settled in bridge reports.

Load-bearing bridge categories:

- Destroy walkers mark three cells only in damaged-to-destroyed-anchor branches. Healthy-to-damaged transition does not use those three direct calls. Evidence: representative `MapClass::DestroyBridgeWalker_NS_Low @ 0x0057C004`.
- Direction setters mark anchor plus three related cells after writing bridge flags/data; if `param_3==0`, `CellClass::BlowUpBridge` runs before the dirty call. Evidence: `CellClass::SetBridgeDirection_NESW @ 0x0047E126`; NWSE assembly contexts at `0x0047E556..0x0047E82A`.
- Ramp/edge/update helpers mark cells after overlay/data/tile mutations. Evidence: representative `MapClass::UpdateRamp_NS_CollapseA_Low @ 0x0056EFF9`.
- Repair walkers mark restored destroyed-anchor cells. Evidence: assembly contexts `0x0057FA8E..0x00580018`, `0x005804CA..0x00580A64`.

## 4. Current Rust Implementation Status

| Rust surface | Current state | Delta vs caller matrix |
|---|---|---|
| `src/sim/world/mod.rs::radar_terrain_dirty_cells` and `mark_radar_terrain_dirty_cells` | A deduped sim-side list/generation exists. | Shape is usable, but producers are bridge-only. |
| `src/sim/world/world_orders.rs` | Bridge repair calls `mark_radar_terrain_dirty_cells(outcome.radar_cells)`. | Covers a subset of bridge repair only. No ore, wall, terrain limbo, sell-wall, voxel-spawn, or generic overlay producer coverage. |
| `src/render/minimap.rs::apply_bridge_terrain_dirty_cells` | Applies dirty cells by consulting bridge state and rewriting `base_terrain_rgba`. | Name and behavior are bridge-specific; native dirty cells require `CellClass::GetRadarColor` over the current terrain/overlay/shroud source, not bridge-only color replacement. |
| `src/app_render/build_instances.rs` | Passes sim dirty list/generation to minimap update. | Transport exists, but native incremental surface and producer coverage are incomplete. |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Direct xrefs to `0x006551C0` | verified | `get_function_xrefs`, 72 call sites listed above | none |
| Ore full removal | verified | `0x00480BEA` | implementation |
| New tiberium placement | verified | `0x00487368` | implementation |
| Existing ore density growth no dirty call | verified | `CellClass::PlaceTiberium` decompile: no xref in growth branch | compare with color branch in final minimap rewrite |
| Wall final removal | verified | `0x00480F27` | implementation |
| Wall neighbor cleanup | verified | `0x004807C2` | implementation |
| Terrain object limbo | verified | `0x0071CA60` | implementation |
| Sell wall/building overlay | verified | `0x004FD006` | implementation |
| Generic repaint helper | touched-not-exhausted | `0x00486FE3`, helper xrefs | exact role/name of every upstream caller if needed |
| Ore/gem spread legacy helpers | touched-not-exhausted | `0x00485778`, `0x00485A1D`, `0x00485C96`, `0x0048627E`; prior `LAT_RETRIGGER...` doc | exact stock trigger frequency |
| Bridge destroy/apply/ramp/repair/direction categories | verified for matrix | all bridge call sites and representative decompiles | bridge mechanics remain in bridge reports |
| Veinhole/vein cleanup | touched-not-exhausted | `0x0074CAC2` | stock map frequency and object lifecycle outside this scope |
| Voxel animation tiberium spawn | verified gate shape | `0x0074A561`, `0x0074A6F7` | enumerate stock VoxelAnimTypes with `+0x300` if implementing content coverage tests |
| Rust dirty producer coverage | verified mismatch | `src/sim/world/mod.rs`, `src/sim/world/world_orders.rs`, `src/render/minimap.rs` | implementation pass |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the investigation mode? -> exhaustive-slice for direct `MarkTerrainDirty` xrefs; bridge mechanics category-only by scope.` (evidence: user slot prompt; Ghidra xrefs)
- `[RESOLVED] OQ-02 - How many direct call instructions exist? -> 72 direct call instructions in the current Ghidra program.` (evidence: `get_function_xrefs 0x006551C0`)
- `[RESOLVED] OQ-03 - Does partial ore density reduction mark terrain dirty? -> no; only full removal calls at `0x00480BEA`.` (evidence: `CellClass::Reduce_Tiberium @ 0x00480A80`)
- `[RESOLVED] OQ-04 - Does existing-tiberium density growth in `PlaceTiberium` mark terrain dirty? -> no direct call in that branch; new overlay placement does call at `0x00487368`.` (evidence: `CellClass::PlaceTiberium`)
- `[RESOLVED] OQ-05 - Do wall partial damage frames mark terrain dirty? -> not through the final `DestroyOverlay` call; final removal calls at `0x00480F27`.` (evidence: `CellClass::DestroyOverlay`)
- `[RESOLVED] OQ-06 - Does wall neighbor cleanup dirty only wall cells? -> no; the dirty call precedes the wall-overlay check for each visited cleanup entry.` (evidence: `CellClass::PostDestructionWallCleanup @ 0x004807C2`)
- `[RESOLVED] OQ-07 - Are terrain object removals map-editor gated? -> yes; `TerrainClass::Limbo` skips `MarkTerrainDirty` when `g_MapEditorMode != 0`.` (evidence: `0x0071CA60`)
- `[RESOLVED] OQ-08 - Are bridge repair/destroy calls direct producers? -> yes; repair/destroy/apply/ramp/direction helpers account for most direct call sites.` (evidence: xref matrix)
- `[RESOLVED] OQ-09 - Does Rust have a general producer API? -> only a generic list exists; known producer call is bridge repair order.` (evidence: `src/sim/world/mod.rs`, `src/sim/world/world_orders.rs`)
- `[DEFERRED] OQ-10 - Exact stock trigger frequency for ore/gem spread helper `FUN_00485590`/`FUN_00485AF0`.` (category: requires-different-system-context; reason: caller matrix only needed direct `MarkTerrainDirty` producers; next-step-if-pursued: investigate tiberium/ore spread scheduler end-to-end)
- `[DEFERRED] OQ-11 - Full stock VoxelAnimType content list for `+0x300` tiberium spawning.` (category: out-of-scope; reason: matrix proves branch gate, not content inventory; next-step-if-pursued: parser/content audit for VoxelAnimType tiberium spawn flag)
- `[DEFERRED] OQ-12 - Live standard-map veinhole/vein frequency.` (category: out-of-scope; reason: TS/YR legacy category only; next-step-if-pursued: map corpus and object construction trace)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ore/tiberium terrain dirty producers include full removal, new placement, and voxel-spawned ore; partial density reduction and existing-density growth do not call `MarkTerrainDirty`. | `0x00480BEA`, `0x00487368`, `0x0074A561`, `0x0074A6F7`; negative branch evidence in decompiles | Missing; Rust dirty producer is bridge-only | `src/sim/world/mod.rs`, ore/tiberium mutation systems, future minimap dirty API | Emit radar terrain dirty only for native-producing ore events, not every density write. | `minimap_dirty_full_ore_removal_and_new_ore_placement_but_not_partial_density_growth` | Do not use "any ore amount changed" as the dirty trigger. |
| Wall destruction dirties final destroyed wall cell and wall-neighbor cleanup dirties self/cardinal entries before wall eligibility checks; sell-wall path also dirties. | `0x00480F27`, `0x004807C2`, `0x004FD006` | Missing | overlay/wall damage systems, sell command path, `src/sim/world/mod.rs` dirty list | Publish dirty cells for final wall removal, cleanup neighbors, and sellable wall removal. | `minimap_dirty_wall_final_removal_and_cleanup_neighbors_once` | Do not limit cleanup dirty cells to cells that still contain a wall. |
| Terrain object limbo dirties terrain only outside map editor. | `0x0071CA60` | Unchecked/missing | terrain object removal/despawn path | Publish dirty cell when a gameplay terrain object is concealed/removed; skip map-editor-only path if modeled. | `minimap_dirty_terrain_limbo_gameplay_not_map_editor` | Do not treat static terrain-object loading as the same as gameplay limbo. |
| Bridge dirty producers are numerous but category-bounded: destroy/apply/ramp/edge/direction/repair/tile-footprint changes. | xrefs `0x0057C004..0x00580018`, `0x0047E126..0x0047E82A`, `0x0056EADD`, `0x0056EC8E`, `0x005814E3`, `0x00581BF5` | Partially present for bridge repair only | `src/sim/bridge_state`, `src/sim/world/world_orders.rs`, `src/render/minimap.rs` | Bridge systems should emit exact native cell sets for each state/tile mutation category, then renderer should recolor via native radar color source. | `minimap_dirty_bridge_destroy_repair_ramp_edge_direction_cell_sets_match_native` | Do not collapse all bridge changes into a full minimap refresh if dirty/copy cadence parity is required. |

## 8. Negative Facts / Do Not Do

- Do not mark radar terrain for partial ore harvest/density reduction. `Reduce_Tiberium` calls only on full removal (`0x00480BEA`).
- Do not mark radar terrain for existing-tiberium density growth in `PlaceTiberium`; the direct call is only in the new-overlay branch (`0x00487368`).
- Do not restrict wall cleanup dirty cells to still-wall cells. `PostDestructionWallCleanup` dirties before the wall-overlay check (`0x004807C2`).
- Do not treat bridge dirty coverage as only repair-hut restoration. Destroy/apply/ramp/edge/direction/tile-footprint helpers also call `MarkTerrainDirty`.
- Do not merge generic terrain dirty producers into object/pixel dirty. These callers feed raw/generated terrain refresh, not final object-dot `MarkCellDirty`.

## 9. Stale Docs / Follow-Up Docs

`docs/research/ADDRESS_MAP.md`

Replace:

`| 0x006551C0 | RadarClass::MarkTerrainDirty (37 callers) | 32 lines | RADAR_MINIMAP |`

With:

`| 0x006551C0 | RadarClass::MarkTerrainDirty (72 direct call sites across 38 caller functions in current Ghidra; see MARKTERRAINDIRTY_FULL_CALLER_MATRIX_GHIDRA_REPORT.md) | 32 lines | RADAR_MINIMAP |`

`docs/research/RADAR_MINIMAP_RENDERING.md`

Replace the broad "37 callers - every game system" wording around `MarkTerrainDirty` with:

`MarkTerrainDirty has 72 direct call sites in the current Ghidra program. They group into ore/tiberium full-removal/new-placement/spawn, wall final removal and cleanup, terrain limbo, sell-wall removal, bridge state/tile/ramp/edge/direction/repair mutation, vein/legacy cleanup, voxel tiberium spawning, and generic cell repaint helper categories. See MARKTERRAINDIRTY_FULL_CALLER_MATRIX_GHIDRA_REPORT.md for caller conditions.`

## 10. Remaining Uncertainty

- Exact stock trigger frequency for ore/gem spread helper paths `FUN_00485590` and `FUN_00485AF0` was not expanded.
- Exact stock VoxelAnimType list for the `+0x300` tiberium-spawn gate was not enumerated.
- Veinhole/vein cleanup is verified as a conditional caller, but standard YR map/object frequency remains outside this matrix.
- Generic helper `FUN_00486E70` upstream callers were listed but not renamed or exhaustively classified beyond active caller examples.

## Sources

- Ghidra xrefs: `RadarClass::MarkTerrainDirty @ 0x006551C0`, `get_function_xrefs(limit=100)`.
- Ghidra assembly context for all direct call sites listed in section 2.
- Ghidra decompiles: `CellClass::Reduce_Tiberium`, `CellClass::DestroyOverlay`, `CellClass::PostDestructionWallCleanup`, `FUN_00486E70`, `MapClass::SetOverlayAndPropagate`, `MapClass::ToggleBridgePavement`, `HouseClass::Sell_Building_At_Cell`, `FUN_00485590`, `FUN_00485AF0`, `CellClass::PlaceTiberium`, `FUN_00581140`, `TerrainClass::Limbo`, `VeinholeMonsterClass__Constructor`, `VoxelAnimClass::AI`, representative bridge helpers.
- Prior docs referenced: `RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE_GHIDRA_REPORT.md`, `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `bridges/01-assets-map-load-overlay/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`.
- Rust scanned: `src/sim/world/mod.rs`, `src/sim/world/world_orders.rs`, `src/render/minimap.rs`, `src/app_render/build_instances.rs`.
