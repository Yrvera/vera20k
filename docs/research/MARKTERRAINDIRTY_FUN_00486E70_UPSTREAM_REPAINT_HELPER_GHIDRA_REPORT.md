# MarkTerrainDirty FUN_00486E70 Upstream Repaint Helper - Ghidra Research Report

**Address(es):** `FUN_00486E70 @ 0x00486E70`, direct `RadarClass::MarkTerrainDirty @ 0x00486FE3`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The helper's repaint/dirty semantics and all direct upstream callers that eventually feed `RadarClass::MarkTerrainDirty` through this helper.  
**Non-Scope:** Full 72-call-site `MarkTerrainDirty` matrix, full building placement/lifecycle mechanics, full smudge spawn system, full overlay chain-reaction system, or Rust patches.  
**Confidence:** High for helper semantics, direct xrefs, call ordering, argument/cell semantics, and active/conditional liveness. Medium for exact human-readable names of some already-unnamed building branches.  
**Active in YR:** Yes/Conditional. The helper is reached by ordinary building placement and map smudge loading in standard YR; other paths are conditional on `ToTile=`/overlay `Explodes=`.

## 0. Working Notes Gate

Target question: classify `FUN_00486E70` and its callers that eventually feed `RadarClass::MarkTerrainDirty`, including repaint reasons, call ordering, argument/cell semantics, and standard YR liveness.

Non-goals: do not redo the full `MarkTerrainDirty` caller matrix, bridge mutation matrix, smudge spawn rules, building lifecycle, or Rust implementation.

Evidence needed to mark COMPLETE: decompile plus assembly for `0x00486E70`; direct xrefs to `0x00486E70`; decompile/assembly around each caller call site; caller liveness evidence from standard YR docs/INI where applicable; Rust touchpoint scan; implementation handoff.

Stop conditions: stop before mutating Ghidra, renaming functions, editing Rust/INI, or expanding into related dirty producers outside direct callers of this helper.

## 1. Overview

`FUN_00486E70` is a cell repaint-and-radar-dirty helper. It does not mutate the cell itself. It computes a tactical dirty rectangle from the cell's current terrain/overlay draw rects, calls `TacticalClass::DirtyScreenRect`, then calls `RadarClass::MarkTerrainDirty` for the same cell coordinate at `CellClass+0x24`.

The helper exists so callers can repaint stale tactical pixels and minimap terrain when a cell's overlay/smudge/building-occupation presentation is about to change or has just changed. The direct upstream reasons are: overlay explosive removal, building footprint/safety-margin redraw, building `ToTile=` footprint redraw, and smudge footprint placement.

## 2. Key Offsets / Globals

| Field / global | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| `CellClass+0x24` | Packed cell coordinate passed to `MarkTerrainDirty`. | Yes | `0x00486FDF..0x00486FE3` computes `cell + 0x24` and calls `0x006551C0`. |
| `CellClass+0x44` | `OverlayTypeIndex`, checked by rect helpers and overlay explosive caller. | Yes | `FUN_0047FB90`, `FUN_0047FDE0`, `Apply_area_damage @ 0x0048A2C4`. |
| `CellClass+0x48` | `SmudgeTypeIndex`, written by smudge footprint placer before helper call. | Conditional | `FUN_006B6080 @ 0x006B60EA..0x006B60FB`; smudge docs mark map-load/spawn paths active. |
| `CellClass+0x11E` | Overlay/smudge frame byte used by overlay rect helpers. | Yes | `FUN_0047FB90`, `FUN_0047FDE0`. |
| `CellClass+0x11F` | Smudge footprint frame/index byte; smudge footprint placer writes row-major index. | Conditional | `0x006B60F5`, `SMUDGE_CLASS_GHIDRA_REPORT.md`. |
| `g_RadarViewportOffsetX/Y @ 0x00886FA0/0x00886FA4` | Subtracted from tactical dirty rect before `DirtyScreenRect`. | Yes | `0x00486FA9..0x00486FBE`. |
| `g_RadarClass @ 0x0087F7E8` | Receiver for `RadarClass::MarkTerrainDirty`. | Yes | `0x00486FDA..0x00486FE3`. |
| `OverlayType+0x2B0` | `Explodes=` overlay flag; gates area-damage overlay chain removal. | Conditional/Yes when such overlay exists | `Apply_area_damage @ 0x0048A2D0..0x0048A2E9`; `combat/systems/chain_reaction.md` verifies identity and vanilla-map IC barrel liveness. |
| `BuildingType+0xE58` | `ToTile=` style building type pointer. | Conditional | `FUN_0043F180`; `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`; stock `[GAGREEN] ToTile=Green01`, `TechLevel=-1`. |

## 3. Core Logic

### 3.1 Helper Semantics

`FUN_00486E70(this_cell)`:

1. Computes three current-cell draw rectangles:
   - `FUN_0047FF80`: base isometric tile rect, normally `60x30`, with height extension for special tile imagery.
   - `FUN_0047FB90`: current overlay/body rect, using overlay/tiberium shape lookup and cell frame data.
   - `FUN_0047FDE0`: alternate overlay frame rect, using `frame_count / 2 + cell_frame` and a special `0x80` flag/`9..17` frame offset adjustment.
2. Unions non-empty rectangles. Width/height less than `1` make a rect empty. Right/down expansion uses the native inclusive `+1` adjustment.
3. Calls `TacticalClass::DirtyScreenRect(rect.x - g_RadarViewportOffsetX, rect.y - g_RadarViewportOffsetY, rect.w, rect.h, 0)`.
4. Calls `RadarClass::MarkTerrainDirty(&cell->MapCoord)`.

Active in YR: Yes. Evidence: helper decompile at `0x00486E70`; assembly `0x00486FA9..0x00486FE3`.

Tiny details:

- The helper dirties tactical screen before radar terrain.
- Duplicate radar dirty suppression is owned by `MarkTerrainDirty`, not this helper.
- No helper-level playfield/bounds check is performed before `MarkTerrainDirty`; later radar flush code handles invalid/out-of-playfield cells.
- The helper observes the cell state at the exact caller point. Some callers invoke it before mutation, while smudge placement invokes it after writing smudge fields.

### 3.2 Caller Matrix

| Caller / call site | Repaint reason | Call ordering | Argument / cell semantics | Active in YR |
|---|---|---|---|---|
| `Apply_area_damage @ 0x0048A2E9` | `Explodes=yes` overlay chain cell is being removed and replaced by effects/recursive damage. | Calls helper before clearing `Cell+0x44`; then writes overlay `-1`, recalcs attributes, reassigns zone, rebuilds pathing, stops targeting, spawns anim/particles, and recursively applies C4-style area damage. | `ECX = target CellClass*` for the damaged cell. | Conditional/Yes for explosive overlays; vanilla maps use IC barrel-style explosive overlays per `combat/systems/chain_reaction.md`. |
| `BuildingClass::Place_OccupyMap @ 0x00442023` | Building occupation overlay `0xEF` is about to replace the current cell presentation. | Calls helper before writing `OverlayTypeIndex=0xEF`; then clears `Cell+0x40`, recalcs attributes, assigns orphaned zone, rebuilds zone/pathing, and stops targeting. | Iterates the building's vtable foundation cell-list deltas until `(0x7FFF,0x7FFF)`; each foundation cell is passed. | Yes; `BuildingClass::Update` and `BuildingClass::ReceiveDamage` call `Place_OccupyMap`. |
| `FUN_0043F180 @ 0x0043F58E` | `ToTile=` / terrain-tile-style building placement branch redraws affected building footprint plus safety margin. | After per-foundation placement checks and smudge clearing, loops a `(W+2) x (H+2)` rectangle from origin `(-1,-1)` and calls helper for each cell; then if placement succeeded, calls building vtable `+0x280(3)` and `+0xF8`. | Cell coordinate = building origin plus loop offset, including one-cell border around foundation. | Conditional. `ToTile=` exists in stock `[GAGREEN]`, but it is `TechLevel=-1`; normal skirmish production liveness not proven. |
| `FUN_0043F180 @ 0x0043F984` | Normal building mark/enter-cell path redraws footprint plus one-cell safety margin after object/list state and smudge clearing. | Calls `TechnoClass::EnterCell_AddToMultiCells` first, updates attached anim coordinates, clears smudges across the base foundation, then loops `(W+2) x (H+2)` and calls helper. | Cell coordinate = building origin plus loop offset, including one-cell border around foundation. | Yes; `0x0043F180` is BuildingClass vtable `+0x124`, active in placement/mark and radio refresh contexts. |
| `FUN_006B6080 @ 0x006B60FB` | Smudge footprint cell was written and must repaint tactical/radar terrain. | Writes `Cell+0x48 = SmudgeType+0x294`, writes `Cell+0x11F = row * width + col`, then calls helper. | Iterates the smudge type's `Width x Height` rectangle from the passed anchor cell. | Yes for map-load smudges; conditional for runtime smudges. `SmudgeClass::ReadINI @ 0x006B4C80` calls this path via missing-boundary code at `0x006B4C45`, and `ScenarioClass::Full_Init` calls map smudge loading. |

## 4. INI / Content Liveness

| Key / content | Effect here | Active in YR | Evidence |
|---|---|---|---|
| `OverlayType Explodes=` | Enables `Apply_area_damage` overlay chain-removal caller. | Conditional/Yes when map has explosive overlay; vanilla-map IC barrel path documented live. | `OverlayType+0x2B0`; `combat/systems/chain_reaction.md`. |
| `BuildingType ToTile=` | Enables the special `BuildingType+0xE58` placement path in `FUN_0043F180`. | Conditional. Stock `[GAGREEN] ToTile=Green01` exists but has `TechLevel=-1`. | `ini/rulesmd.ini:16355..16375`; `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`. |
| `Foundation=` | Determines building width/height used by the `(W+2) x (H+2)` helper loops. | Yes for standard buildings. | `BuildingTypeClass::GetFoundationWidth/Height`; `BIB_SYSTEM_GHIDRA_REPORT.md`. |
| `[Smudge]` map entries | Construct smudges and write footprint cells through `FUN_006B6080`. | Yes during map load when map contains unbaked smudges. | `SmudgeClass::ReadINI @ 0x006B4C80`; `ScenarioClass::Full_Init @ 0x00687B0E`; `SMUDGE_CLASS_GHIDRA_REPORT.md`. |
| `SmudgeType Width/Height` | Controls `FUN_006B6080` footprint dimensions. | Yes for smudges. | `SmudgeType+0x298/+0x29C`; `SMUDGE_CLASS_GHIDRA_REPORT.md`. |

## 5. Integration Points

- `FUN_00486E70 -> TacticalClass::DirtyScreenRect -> RadarClass::MarkTerrainDirty`: helper-level order is tactical first, radar second.
- `BuildingClass::Place_OccupyMap` is not the same as `FUN_0043F180`; both call this helper for building placement/marking, at different lifecycle stages.
- `FUN_006B6080` is a smudge footprint writer, not an overlay writer. It uses `Cell+0x48/+0x11F`, then dirties through this helper.
- The unnamed caller boundary around `0x006B4C45` is missing in Ghidra, but assembly shows it calls `SmudgeTypeClass::CanPlaceHere @ 0x006B5F80`, then `FUN_006B6080`, then clears the smudge object's byte `+0x74` and calls vtable `+0xF8`. `SmudgeClass::ReadINI @ 0x006B4C80` is separately verified as map-load entry.

## 6. Current Rust Implementation Status

| Rust surface | Current state | Delta vs helper callers |
|---|---|---|
| `src/sim/world/mod.rs` | Has `radar_terrain_dirty_cells` and `mark_radar_terrain_dirty_cells`; current producers are sparse. | Needs producer coverage for building placement/mark cells and smudge/explosive-overlay cells, not just bridge repair. |
| `src/render/minimap.rs` | `apply_bridge_terrain_dirty_cells` handles a bridge-specific dirty interpretation and reuploads minimap texture generation. | Native helper wants generic `CellClass::GetRadarColor` terrain refresh for any dirtied cell after current cell mutation state is reflected. |
| `src/sim/production/production_placement.rs` / `src/app_commands.rs` | Ready-building placement exists. | Needs native building-placement radar dirty sets: base foundation cells before `0xEF`, plus mark/enter-cell `(W+2)x(H+2)` safety-margin cells at the right placement stage. |
| `src/sim/combat/smudge_dispatch.rs`, `src/sim/smudge_grid.rs` | Smudge grid and spawn dispatcher exist. | Smudge footprint placement does not appear to publish radar terrain dirty cells per footprint cell. |
| `src/sim/combat/mod.rs`, `src/sim/overlay_grid.rs` | Overlay/ore/wall combat effects exist in abstractions. | Explosive overlay chain removal via `OverlayType.Explodes` must dirty the cell before/around overlay removal if implementing that path. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working-notes gate | verified | section 0 | none |
| `FUN_00486E70` helper semantics | verified | decompile + assembly `0x00486E70..0x00486FEF` | none |
| Direct xrefs to helper | verified | `get_function_xrefs 0x00486E70`: five call sites | none |
| Helper callees | verified | `get_function_callees 0x00486E70` | exact semantic names of rect helpers are inferred from field use, not renamed |
| `Apply_area_damage` caller | verified | decompile and assembly `0x0048A2C4..0x0048A312` | stock map inventory for every explosive overlay not enumerated |
| `BuildingClass::Place_OccupyMap` caller | verified | decompile and assembly `0x00441FD8..0x0044205F` | none for this slice |
| `FUN_0043F180` `ToTile=` caller | verified for dirty loop | decompile and assembly `0x0043F540..0x0043F5C3` | exact broader mode semantics outside dirty loop |
| `FUN_0043F180` normal mark caller | verified for dirty loop | decompile and assembly `0x0043F936..0x0043F99D`; vtable binding docs | exact broader mode semantics outside dirty loop |
| `FUN_006B6080` smudge footprint caller | verified | decompile `0x006B6080`; assembly `0x006B60EA..0x006B60FB` | missing Ghidra function boundary for caller at `0x006B4C45` |
| Smudge map-load liveness | verified | `SmudgeClass::ReadINI @ 0x006B4C80`, xref from `ScenarioClass::Full_Init @ 0x00687B0E`, docs | runtime smudge spawn paths not reclassified |
| Rust producer coverage | touched-not-exhausted | `rg` scan over `src/` | implementation pass |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the mode/scope? -> exhaustive-slice for direct callers of `FUN_00486E70`, not the full radar dirty matrix.` (evidence: user slot prompt; report scope)
- `[RESOLVED] OQ-02 - Does `FUN_00486E70` mutate the cell? -> no; it computes dirty rects and calls tactical/radar dirty helpers only.` (evidence: `0x00486E70..0x00486FEF`)
- `[RESOLVED] OQ-03 - What argument reaches `MarkTerrainDirty`? -> `cell + 0x24`, the cell's packed map coordinate.` (evidence: `0x00486FDF..0x00486FE3`)
- `[RESOLVED] OQ-04 - Are tactical and radar dirties ordered? -> tactical dirty first, radar terrain dirty second.` (evidence: `0x00486FD1` before `0x00486FE3`)
- `[RESOLVED] OQ-05 - How many direct helper callers exist? -> five direct call sites in four caller functions.` (evidence: `get_function_xrefs 0x00486E70`)
- `[RESOLVED] OQ-06 - Is `Apply_area_damage` caller before or after overlay clear? -> before overlay clear, while current overlay draw rect still exists.` (evidence: `0x0048A2E9` before `0x0048A2F0`)
- `[RESOLVED] OQ-07 - Is explosive overlay path YR-active? -> conditional/yes when explosive overlays are present; IC barrel overlay chain is documented live on vanilla maps.` (evidence: `combat/systems/chain_reaction.md`)
- `[RESOLVED] OQ-08 - Is `Place_OccupyMap` caller before or after `0xEF` write? -> before `OverlayTypeIndex=0xEF`.` (evidence: `0x00442023` before `0x0044202C`)
- `[RESOLVED] OQ-09 - What cells does `Place_OccupyMap` call for? -> the building foundation vtable list until `(0x7FFF,0x7FFF)` sentinel.` (evidence: `0x00441FD8..0x00442023`; `BIB_SYSTEM_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-10 - What cells do `FUN_0043F180` loops call for? -> `(W+2)x(H+2)` rectangle offset by `-1,-1` around foundation.` (evidence: `0x0043F540..0x0043F58E`, `0x0043F936..0x0043F984`)
- `[RESOLVED] OQ-11 - Is normal `FUN_0043F180` building mark path active? -> yes; BuildingClass vtable `+0x124` binds to `0x0043F180`, and placement execution docs mark it live.` (evidence: `BUILDINGCLASS_VTABLE_COMPLETE.md`; `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-12 - Is `ToTile=` path active in stock build menu? -> conditional only; stock `[GAGREEN] ToTile=Green01` exists but is `TechLevel=-1`.` (evidence: `ini/rulesmd.ini:16355..16375`)
- `[RESOLVED] OQ-13 - Is smudge footprint path active? -> yes for map-load smudges; conditional for runtime smudges.` (evidence: `SmudgeClass::ReadINI @ 0x006B4C80`; xref `0x00687B0E`; `SMUDGE_CLASS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-14 - Does smudge call helper before or after writing cell smudge state? -> after writing `Cell+0x48` and `Cell+0x11F`.` (evidence: `0x006B60EA..0x006B60FB`)
- `[RESOLVED] OQ-15 - Does current Rust have a generic producer for these cells? -> no complete producer coverage found; existing radar dirty channel is bridge-skewed.` (evidence: `src/sim/world/mod.rs`, `src/render/minimap.rs`, `src/sim/combat/smudge_dispatch.rs`)
- `[DEFERRED] OQ-16 - Which exact stock maps contain unbaked `[Smudge]` entries or explosive overlay instances?` (category: out-of-scope; reason: caller liveness is proven; map-corpus inventory is separate; next-step-if-pursued: retail map scan)
- `[DEFERRED] OQ-17 - Exact broader side effects of `FUN_0043F180(mode=2)` radio/anim refresh outside dirty loops.` (category: out-of-scope; reason: this report only needs helper call semantics; next-step-if-pursued: focused BuildingClass vtable `+0x124` mode audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Building placement emits radar terrain dirty through two mechanisms: `Place_OccupyMap` dirties each base foundation cell before writing overlay `0xEF`, and the BuildingClass mark/enter-cell path dirties a `(W+2)x(H+2)` safety-margin rectangle. | `0x00442023`, `0x0044202C`, `0x0043F984`; building docs | Missing/unchecked | `src/sim/production/production_placement.rs`, `src/sim/world/mod.rs`, `src/render/minimap.rs` | Publish native cell sets at the same placement stages, and render dirty cells from current terrain/overlay state. | Place a `2x2` building: foundation cells and one-cell border receive terrain dirty entries with native dedupe/order; minimap reflects occupied foundation/surround changes without a full refresh. Proposed test: `minimap_dirty_building_place_foundation_and_margin_cells_native_order`. | Do not mark only the foundation or only the final object dot; the one-cell safety-margin dirty loop is separate and active. |
| Smudge footprint placement writes `Cell+0x48/+0x11F` first, then calls helper per footprint cell, so minimap terrain must refresh after the smudge state exists. | `FUN_006B6080 @ 0x006B60EA..0x006B60FB`; `SMUDGE_CLASS_GHIDRA_REPORT.md` | Missing/unchecked | `src/sim/combat/smudge_dispatch.rs`, `src/sim/smudge_grid.rs`, `src/sim/world/mod.rs`, `src/render/minimap.rs` | When a smudge is placed, enqueue every smudge footprint cell for radar terrain dirty after mutating the smudge grid. | Load or spawn a `2x2` crater/scorch: four footprint cells refresh minimap terrain once each, after smudge state is visible to radar color/render source. Proposed test: `minimap_dirty_smudge_footprint_after_smudge_write`. | Do not treat smudges as tactical-only decals; native routes smudge placement through radar terrain dirty. |
| Explosive overlay chain removal calls helper before clearing the overlay, then removes overlay and recursively applies C4-style damage/effects. | `Apply_area_damage @ 0x0048A2C4..0x0048A312`; `combat/systems/chain_reaction.md` | Missing/unchecked | `src/sim/combat/mod.rs`, `src/sim/overlay_grid.rs`, `src/sim/world/mod.rs` | Implement explosive-overlay removal as a radar terrain dirty producer for the affected cell, preserving call ordering relative to overlay clear when retained surfaces are modeled. | Destroy an `Explodes=yes` overlay/barrel: the cell is enqueued for terrain dirty and the refreshed minimap no longer shows the old overlay after the update pass. Proposed test: `minimap_dirty_explosive_overlay_chain_removal_cell`. | Do not confuse `OverlayType+0x2B0 Explodes` with `OverlayType+0x2B1 ChainReaction` or with Warhead `Tiberium=yes`; they gate different mechanisms. |

### Negative Facts / Do Not Do

- Do not model `FUN_00486E70` as the mutation itself. It is a repaint/dirty helper; mutations are in the callers.
- Do not call `MarkTerrainDirty` with screen coordinates. The helper passes `CellClass+0x24` map cell coordinates.
- Do not assume helper calls always happen after mutation. Building `Place_OccupyMap` and explosive overlay removal call it before the overlay write/clear; smudge footprint placement calls it after smudge fields are written.
- Do not limit building placement radar dirty to foundation cells. The `FUN_0043F180` loops dirty a one-cell margin around the foundation.
- Do not drop smudge cells from radar dirty just because smudges are cosmetic; native smudge placement dirties radar terrain per footprint cell.

### Stale Docs / Follow-up Docs

`docs/research/MARKTERRAINDIRTY_FULL_CALLER_MATRIX_GHIDRA_REPORT.md`

Replace:

`Generic terrain repaint helper | FUN_00486E70 | 0x00486FE3 | Helper itself has no branch gate after entry; callers include area damage, building map occupation, and other cell-change paths. | Dirties tactical rect and radar terrain for param_1 + 0x24; mutation occurs in caller. | Conditional: active when its caller mutates a cell in gameplay.`

With:

`Generic terrain repaint helper | FUN_00486E70 | 0x00486FE3 | Computes current-cell terrain/overlay draw rect union, calls TacticalClass::DirtyScreenRect, then RadarClass::MarkTerrainDirty(cell+0x24). Direct callers are Apply_area_damage explosive-overlay removal, BuildingClass::Place_OccupyMap foundation occupation, BuildingClass vtable +0x124 mark/enter-cell loops for ToTile and normal placement, and smudge footprint placement FUN_006B6080. Call ordering differs by caller: building/overlay removal call before overlay mutation, smudge placement calls after Cell+0x48/+0x11F writes. | Dirties tactical repaint rect and radar terrain for the target cell. | Yes/Conditional: ordinary building placement and map smudges are active; ToTile and explosive-overlay paths depend on stock/content conditions.`

## Remaining Uncertainty

- Exact stock-map inventory of unbaked smudges and explosive overlays was not enumerated.
- The missing Ghidra function boundary around the `0x006B4C45 -> FUN_006B6080` caller was not created because swarm Ghidra access is read-only.
- Full `FUN_0043F180(mode=2)` radio/attached-animation write set remains outside this helper-focused slice.

## Sources

- Ghidra decompile/disassembly: `FUN_00486E70 @ 0x00486E70`; assembly `0x00486FA9..0x00486FE3`.
- Ghidra xrefs: `get_function_xrefs 0x00486E70`, direct callers at `0x0048A2E9`, `0x00442023`, `0x006B60FB`, `0x0043F58E`, `0x0043F984`.
- Ghidra decompile/assembly: `Apply_area_damage @ 0x00489280`; `BuildingClass::Place_OccupyMap @ 0x00441F60`; `FUN_0043F180 @ 0x0043F180`; `FUN_006B6080 @ 0x006B6080`; `SmudgeTypeClass::CanPlaceHere @ 0x006B5F80`; `FUN_0047FB90`; `FUN_0047FDE0`; `FUN_0047FF80`.
- Prior docs: `MARKTERRAINDIRTY_FULL_CALLER_MATRIX_GHIDRA_REPORT.md`, `RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE_GHIDRA_REPORT.md`, `SMUDGE_CLASS_GHIDRA_REPORT.md`, `SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md`, `combat/systems/chain_reaction.md`, `BIB_SYSTEM_GHIDRA_REPORT.md`, `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`, `BUILDINGCLASS_VTABLE_COMPLETE.md`.
- INI: `ini/rulesmd.ini` `[GAGREEN] ToTile=Green01`, `TechLevel=-1`.
- Rust scanned: `src/sim/world/mod.rs`, `src/render/minimap.rs`, `src/sim/production/production_placement.rs`, `src/app_commands.rs`, `src/sim/combat/smudge_dispatch.rs`, `src/sim/smudge_grid.rs`, `src/sim/combat/mod.rs`, `src/sim/overlay_grid.rs`.
