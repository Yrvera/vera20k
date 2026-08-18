# Full Passability Recalc 0x0047D2B0 -- Ghidra Research Report

**Address(es):** `0x0047D2B0` primary (`CellClass::RecalcAttributes`), `0x00483C80` zone-type helper, `0x00586A00` list recalc caller, `0x00584550` incremental hierarchical-zone rebuild, `0x0056C510` full bridge-zone helper  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Per-cell attribute/passability recomputation at `0x0047D2B0`, its immediate zone/cache writes, representative live callers, and full-grid-vs-dirty-cell runtime implications.  
**Non-Scope:** A* search internals, bridge rendering, full bridge damage state machines, full passability matrix reader audit.  
**Confidence:** High for primary function, helper semantics, and caller categories; Medium for some field names inherited from existing docs.  
**Active in YR:** Yes -- called during normal scenario load and runtime terrain/overlay/object mutations in `gamemd.exe`.

## 1. Overview

`CellClass::RecalcAttributes` is a single-cell recomputation routine, not itself a global path-grid rebuild. It re-derives the cell's terrain/overlay-derived `LandType`, slope bytes, reduced `ZoneType`, and mirrors `ZoneType`/level into zone-map byte arrays used by pathing. Runtime callers pair it with local zone repair helpers; full-cell iteration exists during map/scenario/RMG initialization and explicit list/rectangle bridge updates.

Active in YR: Yes. Evidence: scenario load calls at `0x00687A5A` and `MapClass::InitCellAttributes` `0x00568DF4`; runtime calls from building placement `0x0044203A`, overlay mark `0x005FC981`, wall/ore/area damage `0x00480F03`/`0x00480BDC`/`0x0048A2F9`, and bridge repair/destruction xrefs.

## 2. Class Layout / Key Offsets

| Offset / storage | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `Cell+0x24` | packed map coordinate | read throughout `0x0047D2B0`, used by `ZoneMap::CellToZoneIndex` | Yes |
| `Cell+0x38` | `IsoTileTypeIndex`; can be changed by LAT/slope fixup | `0x0047D2B0` calls `CellClass::ApplyLAT_and_SlopeFixup`; helper writes `+0x38` | Yes |
| `Cell+0x44` | overlay type index, `-1` empty | read/write in primary; runtime callers mutate before recalc | Yes |
| `Cell+0xEC` | `LandType` | written from overlay `+0x298` or tile lookup path (corrected 2026-06-01: was `Cell+0x48`; binary shows writes/reads at `Cell+0xEC` via `disassemble_function 0x0047D2B0` and `disassemble_function 0x00483C80` - OFFSET_RETYPED_WRONG) | Yes |
| `Cell+0x4C` | reduced `ZoneType` 0..7 | `CellClass::RecalcZoneType @ 0x00483C80` writes it | Yes |
| `Cell+0x11B` | level byte | primary mirrors to zone arrays; hidden `level_override` can write it | Yes |
| `Cell+0x11C` | slope index byte | primary writes from `TMP_ReadSlopeType` and fallback paths | Yes |
| `Cell+0x11D` | height-in-pixels byte | formula after `FUN_00547150`: `(tile_height - 30) / 15` signed division | Yes |
| `Cell+0x11E` | overlay data/density/state byte | cleared when overlay is removed or slope-invalidated | Yes |
| `Cell+0x140` bit `0x10000` | neighbor draw/path side flag from tile data `+0x2E1` path | primary sets on listed neighbor cells | Yes/Conditional: only tiles with `IsoTileType+0x2E1 != 0` |
| `Cell+0x140` bit `0x20000` | sticky "tile anim spawned" guard | primary sets after creating tile anim if tile anim conditions pass | Yes/Conditional |
| `Map+0x68/0x0087F850` stride 4 | per-cell zone cache: byte0 `ZoneType`, byte1 level, bytes2-3 cluster id | primary writes byte0 and byte1; `UpdateBridgeZonesHelper` clears/fills bytes2-3 | Yes |
| `Map+0x70/0x0087F858` stride 10 | hierarchical per-cell zone data | primary writes byte `+8` = level; `MapClass::RecalcCellsAndRebuildZones` zeros `+0` before repair | Yes |

## 3. Core Logic

Primary flow, verified from `0x0047D2B0`:

1. Early-out if `this == &DAT_00ABDC50` sentinel out-of-bounds cell. Active in YR: Yes; many callers route invalid coords to this sentinel.
2. Compute zone-map index twice with `ZoneMap::CellToZoneIndex`: `(Map+0xF8 + 1 + Map+0xF4) * y + x`, clamped to `[0, Map+0x6C-1]`. Active in YR: Yes.
3. If an overlay exists, overlay `LandType` (`OverlayType+0x298`) can directly become the cell `LandType`; overlay classes with `LandType == 4`, `LandType == 9`, or `OverlayType+0x2AC != 0` take the early overlay branch. Active in YR: Yes.
4. If a valid TMP tile exists, slope is read from tile data; invalid tile index becomes `0xFFFF`, land resets to clear/overlay-driven fallback, slope resets to 0. Active in YR: Yes.
5. `CliffBackImpassability` at `Rules+0x664` is read in three repeated six-neighbor checks. If set to `2`, qualifying cells with otherwise passable land can be forced to `LandType = 3`. Active in YR: Yes; stock `rulesmd.ini` has `CliffBackImpassability=2`.
6. `CellClass::ApplyLAT_and_SlopeFixup` can rewrite the tile id before zone-type recomputation. Active in YR: Yes.
7. `CellClass::RecalcZoneType @ 0x00483C80` writes `Cell+0x4C`, then primary mirrors `Cell+0x4C` and `Cell+0x11B` to zone byte arrays. Active in YR: Yes.

`RecalcZoneType @ 0x00483C80` classification:

| Priority | Result | Evidence | Active in YR |
|---|---|---|---|
| out of playfield | `ZoneType=7` | `MapClass::Is_Cell_In_Playfield` branch | Yes |
| overlay `+0x22D` | `ZoneType=1` | first overlay flag branch | Yes/Conditional |
| overlay `+0x2A8` wall | `ZoneType=2` | wall branch | Yes |
| overlay land Wheel-column speed exactly `0.0`, or overlay `+0x2B5` | `ZoneType=6` | reads `DAT_0089EA48[land*9]`; gate branch | Yes |
| `LandType==2` | `ZoneType=4` water | direct branch | Yes |
| `LandType==6` | `ZoneType=3` beach | direct branch | Yes |
| land Wheel-column speed `<= 0.01` | `ZoneType=6` | `DAT_0089EA48[land*9] <= 0.01` | Yes |
| building object in content list | `ZoneType=5` or `6` depending type/status flags | object-list loop | Yes |
| default | `ZoneType=0` ground | fallthrough | Yes |

Negative detail: `0x0047D2B0` does not rebuild the 13 movement-zone reachability arrays and does not read `g_PassabilityMatrix`; `MapClass::UpdateBridgeZonesHelper @ 0x0056C510` does that after zones/clusters are rebuilt.

## 4. INI Keys

| Key / source | Default / stock YR relevance | Effect in this slice | Active in YR |
|---|---|---|---|
| `[General] CliffBackImpassability` in `rulesmd.ini` | `2` stock YR | Enables the repeated six-neighbor cliff-back land-type override in `0x0047D2B0` | Yes |
| `[LandTypes]` speed entries read into table base `0x0089EA40`; `0x0089EA48` used here | Stock land rows mostly align for path-zone purposes | RecalcZoneType uses Wheel column (`+8`) to classify impassable terrain/overlay land | Yes |
| OverlayType `Land=` | many stock overlays | RecalcAttributes uses overlay `+0x298`; RecalcZoneType can classify overlay-driven impassable/wall/road-ish cells | Yes |
| Overlay flags `IsWall`, crate/road-style `+0x22D`, gate `+0x2B5` | stock walls/gates/overlays | Change reduced `ZoneType` before path-zone graph rebuild | Yes/Conditional |

## 5. Integration Points

| Caller category | Evidence | Behavior | Active in YR |
|---|---|---|---|
| Scenario/map load | `ScenarioClass::Full_Init @ 0x00687A5A`, `MapClass::InitCellAttributes @ 0x00568DF4` | Iterates all cells and calls `RecalcAttributes`; then initializes growth/spread queues and radar | Yes |
| RMG/map generation | `FUN_00598960` has repeated "Recalculating cell attributes" full loops | Full-map passes during random map generation | Yes/Conditional: RMG/editor/generation path |
| Explicit list/rectangle recalc | `MapClass::RecalcCellsAndRebuildZones @ 0x00586A00`; `FUN_005868A0` builds rectangle list | Recalc only supplied cells, zero their level-0 zone ids, then flood-fill unassigned cells | Yes |
| Building placement | `BuildingClass::Place_OccupyMap @ 0x0044203A` | Per foundation cell: set overlay `0xEF`, clear owner ptr, recalc, assign zone, incremental rebuild | Yes |
| Building sell | `HouseClass::Sell_Building_At_Cell @ 0x004FCFE7` | Clears overlay/data/owner, recalc, wall cleanup, assign zone, incremental rebuild | Yes |
| Overlay placement | `OverlayClass::Mark @ 0x005FC981` | Recalc on placed overlay; may merge zone and rebuild around one coord | Yes |
| Wall/overlay destruction | `CellClass::DestroyOverlay @ 0x00480F03`, `PostDestructionWallCleanup @ 0x00480969` | Clears overlay, recalc, updates/merges/orphans local zones | Yes |
| Ore/tiberium removal | `CellClass::Reduce_Tiberium @ 0x00480BDC`, `AnimClass::Middle @ 0x00424EDA` | On full removal recalc and local zone repair | Yes |
| Techno object enter/exit footprint | `TechnoClass::EnterCell_AddToMultiCells @ 0x005684E1`, `ExitCell_RemoveFromMultiCells @ 0x00568911` | Adds/removes content then recalc affected footprint cells | Yes |
| Bridge damage/repair/destruction | xrefs from `0x0057C229` etc.; `ProcessBridgeDamageStateMachine_Low @ 0x00571A40`; `FUN_00568E40` | Touched cell lists/rectangles plus bridge zone helper; not a blind full-map pass | Yes |

## 6. Current Rust Implementation Status

Current Rust uses a whole `PathGrid` rebuild for dynamic pathing changes: `src/app_sim_tick.rs` calls `rebuild_dynamic_path_grid`, which rebuilds `PathGrid::from_resolved_terrain_with_bridges(...)` and then zone grids. Bridge collapse/repair paths in `src/sim/world/bridge_orchestrator.rs` also rebuild path/zone grids when `zones_dirty` is true.

Delta from binary: gamemd's ordinary runtime model is dirty-cell/list/rectangle recomputation plus incremental zone repair (`AssignOrphanedCellZone`, `MergeAdjacentCellZone`, `FUN_00584550`), with full-cell iteration reserved for load/RMG or explicit generated lists. A Rust full-grid rebuild can be behaviorally acceptable if completed before any path query that should see the mutation, but it is not the binary's timing/ownership model and risks one-frame lag where updates happen at app-frame boundaries.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellClass::RecalcAttributes @ 0x0047D2B0` | verified | full decompile | none for scoped passability/cache writes |
| sentinel OOB early-out | verified | `this == DAT_00ABDC50` branch | none |
| `ZoneMap::CellToZoneIndex` | verified | decompile by name | none |
| `CellClass::RecalcZoneType @ 0x00483C80` | verified | full decompile | exact user-facing labels for overlay `+0x22D` not renamed here |
| zone cache mirror writes | verified | primary writes `*zone=+0x4C`, `zone[1]=+0x11B`, `stride10+8=+0x11B` | none |
| `MapClass::RecalcCellsAndRebuildZones @ 0x00586A00` | verified | full decompile | exact vector type name not renamed |
| `FUN_00584550` incremental rebuild | touched-not-exhausted | decompile by name | full graph edge semantics belong to zone-edge slot |
| `MapClass::UpdateBridgeZonesHelper @ 0x0056C510` | touched-not-exhausted | full decompile enough to confirm matrix use | full bridge-zone graph semantics out-of-scope |
| runtime caller categories | verified | xrefs + representative decompiles | no exhaustive proof for every bridge walker body |
| A* and Can_Enter_Cell consumers | deferred | xrefs to zone/Pathfinder functions | out-of-scope per user |
| bridge rendering dirty rectangles | deferred | several callers dirty screen/radar | out-of-scope per user |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] Q1 -- Is 0x0047D2B0 a full-grid function or single-cell function? -> Single-cell; global/list behavior is in callers.` (evidence: `0x0047D2B0`, `0x00586A00`, `0x00687A5A`)
- `[RESOLVED] Q2 -- What pathing classification does it recompute? -> `Cell+0x4C` ZoneType 0..7 through `0x00483C80`.` (evidence: `0x00483C80`)
- `[RESOLVED] Q3 -- What path grid/cache bytes are written? -> zone byte0, zone byte1, stride10 level byte; cluster ids are not rebuilt here.` (evidence: `0x0047D551`, `0x0047D7CD`, `0x0047DD36`)
- `[RESOLVED] Q4 -- Does it read passability matrix? -> No; matrix read is in `0x0056C510`.` (evidence: `MapClass::UpdateBridgeZonesHelper`)
- `[RESOLVED] Q5 -- Does standard YR use full-grid rebuild after ordinary runtime changes? -> No for ordinary cell mutations; runtime callers do dirty/list local repairs.` (evidence: `0x0044203A`, `0x005FC981`, `0x00480F03`, `0x005684E1`)
- `[RESOLVED] Q6 -- Are full-map loops live? -> Yes during scenario/map load and RMG.` (evidence: `0x00687A5A`, `0x00598960`, `0x00568DF4`)
- `[RESOLVED] Q7 -- Are bridge paths full-map? -> Representative bridge paths build local lists/rectangles and/or call bridge-zone helper; not blind full-map rebuild in checked paths.` (evidence: `0x00571A40`, `0x00568E40`, `0x00586A00`)
- `[RESOLVED] Q8 -- Is `CliffBackImpassability` TS-only? -> No; stock YR default is 2 and branch is active.` (evidence: `rulesmd.ini`, `0x0047D2B0`)
- `[RESOLVED] Q9 -- Does it set `Cell+0x140` bit `0x40000`? -> Not in primary decompile; seen writes are `0x10000` and `0x20000` in this function.` (evidence: `0x0047D2B0`)
- `[RESOLVED] Q10 -- Null/OOB cell behavior? -> Invalid coordinates use sentinel `DAT_00ABDC50`; primary returns immediately for sentinel.` (evidence: `0x0047D2B0`, caller decompiles)
- `[RESOLVED] Q11 -- Zero/empty overlay behavior? -> `OverlayTypeIndex == -1` falls back to tile land or clear; invalid/no tile sets clear land and slope 0.` (evidence: `0x0047D2B0`)
- `[DEFERRED] Q12 -- Exact passability matrix reader behavior in A*/Can_Enter_Cell` (category: out-of-scope; reason: separate swarm slots cover matrix readers/A*; next-step-if-pursued: inspect zone precheck and matrix reader call sites)
- `[DEFERRED] Q13 -- Exact labels for every overlay flag offset used by RecalcZoneType` (category: requires-different-system-context; reason: overlay type parser/flags full audit out-of-scope; next-step-if-pursued: trace OverlayTypeClass parser offsets)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Runtime cell mutations recalc only changed cells/lists and then repair local zones; full-map recalc is not the ordinary runtime model. | `0x0044203A`, `0x00480F03`, `0x005684E1`, `0x00586A00`, `0x00584550` | mismatch/architectural drift: `rebuild_dynamic_path_grid` rebuilds whole `PathGrid` and zone grid at app-frame boundary | `src/app_sim_tick.rs`, `src/sim/pathfinding/zone_incremental.rs`, `src/sim/pathfinding/zone_map.rs` | Future dynamic pathing should support dirty-cell/list updates or prove full rebuild has identical before-query timing. | Place a wall/building footprint and issue/continue movement in the same sim tick; path query after mutation must see new `ZoneType` without a frame-late stale grid. Proposed test: `dynamic_path_grid_dirty_cell_update_visible_same_tick`. | Do not rely on end-of-frame full rebuild if same-tick path consumers can query before it. |
| `ZoneType` is the reduced 0..7 classification from `RecalcZoneType`, not raw LandType; impassable check uses Wheel-column speed at `0x89EA48`. | `0x00483C80`, `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` | unchecked/maybe drift: Rust `PathGrid` walkability uses resolved terrain blocking flags and richer land concepts | `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/zone_build.rs`, `src/map/resolved_terrain.rs` | Ensure zone construction uses binary-equivalent `ZoneType` values before matrix/zone grouping, especially rough/clear/road/water/beach/wall/building distinction. | Clear and Rough stock terrain with positive Wheel speed should share Ground `ZoneType=0`; Water should be `4`; Beach `3`; wall overlay `2`. Proposed test: `zone_type_recalc_uses_binary_reduced_columns`. | Do not use raw RA2 `LandType` as zone column. |
| `0x0047D2B0` mirrors `Cell+0x4C`/level into zone byte arrays but does not rebuild cluster ids; cluster/zone graph rebuild is a separate helper. | `0x0047D551`, `0x0047D7CD`, `0x0047DD36`, `0x0056C510` | mismatch risk: Rust may conflate cell walkability rebuild and zone graph rebuild | `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_incremental.rs`, `src/app_sim_tick.rs` | Keep per-cell classification update separate from connectivity/zone-id flood-fill so local changes can be ordered like gamemd. | After clearing one wall/overlay cell, changed cell classification updates first, then local zone id connectivity updates around that coordinate. Proposed test: `zone_rebuild_separates_cell_classification_from_cluster_ids`. | Do not implement `RecalcAttributes` as "rebuild all zones" by default. |

### Negative Facts / Do Not Do

- Do not call `0x0047D2B0` a full-grid rebuild. It accepts one `CellClass*` and writes one cell's classification/cache; full/list iteration is in callers. Evidence: `0x0047D2B0`, `0x00586A00`, `0x00687A5A`. Active in YR: Yes.
- Do not expect `0x0047D2B0` to read `g_PassabilityMatrix` or movement-zone rows. The matrix is consumed in `MapClass::UpdateBridgeZonesHelper @ 0x0056C510` and downstream zone readers. Active in YR: Yes.
- Do not set `Cell+0x140` bit `0x40000` from this function. The scoped decompile shows `0x10000` and `0x20000` writes here, not `0x40000`. Active in YR: Yes.
- Do not use raw `LandType` as the path-zone column; `RecalcZoneType` compresses to 8 `ZoneType` values and treats most positive-speed ground as `0`. Active in YR: Yes.
- Do not treat `CliffBackImpassability` as dormant TS legacy in this slice; `rulesmd.ini` stock default `2` makes the branch active. Active in YR: Yes.

### Remaining Uncertainty

- Exact semantic names for overlay offsets `+0x22D`, `+0x2B4`, and `+0x2B5` were not fully re-derived here; their effects in `RecalcZoneType` are verified, but parser/source labels belong to an OverlayTypeClass audit.
- Full A*/Can_Enter_Cell matrix reader behavior is intentionally out-of-scope for this slot; this report only proves where `RecalcAttributes` writes the reduced cell classification consumed by later pathing.
- Some bridge walker caller bodies were sampled rather than exhaustively re-covered because the parent scope explicitly excluded full bridge rendering/damage investigation.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/gap-scans/2026-05-08-disparity-scan-pathfinding.md` says `0x40000` is set during `CellClass::RecalcAttributes`. Replacement wording: "`CellClass::RecalcAttributes @ 0x0047D2B0` does not write `Cell+0x140` bit `0x40000`; the scoped passability recalc writes/mirrors `ZoneType` and level and may set bits `0x10000`/`0x20000`. Treat any `0x40000` cliff-ramp source as unresolved here or cite the actual writer."

## Sources

- Ghidra: `0x0047D2B0`, `0x00483C80`, `0x0047D020`, `0x00586A00`, `0x00568DF4`, `0x0056EC80`, `0x0044203A`, `0x005684E1`, `0x00568911`, `0x005FC981`, `0x00480F03`, `0x00480969`, `0x00480BDC`, `0x0048A2F9`, `0x00424EDA`, `0x004FCFE7`, `0x00598960`, `0x00687A5A`, `0x00584550`, `0x0056C510`, `0x00571A40`, `0x00568E40`.
- Prior docs: `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md`, `ZONE_PASSABILITY_VERIFIED.md`, `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`, `AUDIT_LOG.md`.
- INI: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scan: `src/app_sim_tick.rs`, `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_incremental.rs`, `src/sim/world/bridge_orchestrator.rs`.
