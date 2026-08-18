# CellClass::RecalcZoneType 0x00483C80 -- Writer-Side Ghidra Research Report

**Address(es):** `0x00483C80` primary writer; integration through `0x0047D2B0`, `0x0056C510`, `0x005840C0`, `0x0042C290`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** writer-side semantics for reduced `CellClass+0x4C` zone type values `0..7`: inputs, branch priority, active YR callers, written values, and corrections to prior reduced-zone labels.  
**Non-Scope:** full `CellClass::RecalcAttributes`, A* internals, full `OverlayTypeClass` parser audit beyond fields used by this writer, and full object/building field-name recovery.  
**Confidence:** High for branch order, written values, and live caller chain; Medium for semantic names of a few object/building fields inherited from adjacent reports.  
**Active in YR:** Yes. `CellClass::RecalcAttributes @ 0x0047D2B0` calls this helper at `0x0047D551`, `0x0047D7CD`, and `0x0047DD36`; that caller is active during map load and runtime terrain/overlay/object mutations.

## 1. Overview

`CellClass::RecalcZoneType @ 0x00483C80` writes the reduced path-zone column stored at `CellClass+0x4C`. The value is later mirrored by `RecalcAttributes` into the per-cell zone cache and consumed as the column of `ZonePassabilityMatrix[movementZone][reducedZoneType]`.

This writer is not a raw `LandType` mapping. It is a priority classifier over playfield status, overlay flags, overlay land-speed, base `LandType`, base land-speed, and normal cell objects.

## 2. Class Layout / Key Offsets

| Offset / storage | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `CellClass+0x24` | packed cell coordinate for playfield test | `0x00483C8C..0x00483C99` | Yes; checked before any other branch |
| `CellClass+0x44` | overlay type index; `-1` means no overlay | `0x00483CA4..0x00483CB2` | Yes; standard overlay-bearing maps use it |
| `CellClass+0x48` | current `LandType` after `RecalcAttributes` processing | `0x00483D2A` | Yes |
| `CellClass+0x4C` | reduced zone type writer target | literal writes `0..7` in `0x00483C99..0x00483E1E` | Yes |
| `CellClass+0xE4` | normal object/content linked list | `0x00483D72` object loop | Yes |
| object `+0x30` | next pointer in normal cell object list | `0x00483DCD` | Yes |
| object vtable `+0x2C` | `WhatAmI` dispatch | `0x00483D80`; adjacent docs map `6=Building`, `0x24=Terrain` | Yes |
| `OverlayTypeClass+0x22D` | inherited `ObjectTypeClass` `Crushable=` bool | read at `0x00483CB5`; parser evidence `0x005F940A` | Yes |
| `OverlayTypeClass+0x298` | overlay `Land=` enum | read at `0x00483CDF`; parser evidence `0x005FE7A4` | Yes |
| `OverlayTypeClass+0x2A8` | overlay `Wall=` bool | read at `0x00483CCA`; parser evidence `0x005FE7D4` | Yes |
| `OverlayTypeClass+0x2B4` | `IsRubble=` bool | read at `0x00483D1C`; parser evidence `0x005FE9FC` | Conditional; active for content/mods that set it |
| `OverlayTypeClass+0x2B5` | `IsARock=` bool | read at `0x00483D07`; parser evidence `0x005FE9DE` | Conditional; active for content/mods that set it |
| `0x0089EA48 + land*9*4` | Wheel-column speed table slot used by this helper | reads at `0x00483CE8` and `0x00483D57`; table decoded in speed-table report | Yes |
| `0x007E1748` | float `0.0` for overlay-speed equality | memory constant, compare at `0x00483CEF` | Yes |
| `0x007E3808` | double `0.01` for land-speed threshold | memory constant, compare at `0x00483D5E` | Yes |
| `BuildingTypeClass+0x16BF` | `LaserFence=` bool | read at `0x00483DB0`; parser `0x004638F4` | Conditional; no stock YR INI hit found |
| `BuildingTypeClass+0x16C0` | `FirestormWall=` bool | read at `0x00483D94`; parser `0x00463909` | Conditional/TS legacy; no stock YR INI hit found |
| `TerrainTypeClass+0x2A8/+0x2AC` | temperate/snow occupation bits | read at `0x00483DDF..0x00483E18`; parser `0x0071DFCA/0x0071DFD4` | Yes; selected by theater |

## 3. Core Logic

| Priority | Written value | Branch meaning | Evidence | Active in YR |
|---:|---:|---|---|---|
| 1 | `7` | Cell is outside playfield | `0x00483C90..0x00483C99` | Yes |
| 2 | `1` | Overlay exists and `OverlayType/ObjectType+0x22D Crushable=` is true | `0x00483CB5..0x00483CBF`, parser `0x005F940A` | Yes |
| 3 | `2` | Overlay exists and `OverlayType+0x2A8 Wall=` is true, after `Crushable` failed | `0x00483CCA..0x00483CD4`, parser `0x005FE7D4` | Yes |
| 4 | `6` | Overlay exists and Wheel-speed slot for overlay `Land=` is exactly `0.0f` | `0x00483CDF..0x00483CFC`, constant `0x007E1748` | Yes |
| 5 | `6` | Overlay exists and `OverlayType+0x2B5 IsARock=` is true | `0x00483D07..0x00483D11` | Conditional |
| 6 | `0` | Overlay exists and `OverlayType+0x2B4 IsRubble=` is true; earlier overlay blockers did not return | `0x00483D1C..0x00483DD4` | Conditional |
| 7 | `4` | `CellClass+0x48 LandType == 2` water | `0x00483D2A..0x00483D36` | Yes |
| 8 | `3` | `CellClass+0x48 LandType == 6` beach | `0x00483D40..0x00483D49` | Yes |
| 9 | `6` | Wheel-speed slot for base `LandType` is `<= 0.01` | `0x00483D54..0x00483D6B`, constant `0x007E3808` | Yes |
| 10 | `6`, non-returning | Building object has `LaserFence` conditions and state not `0xC`/`8`; loop continues afterward | `0x00483D83..0x00483DCD` | Conditional; no stock YR `LaserFence=` found |
| 11 | `6`, returning | Building object has `FirestormWall` condition and active owner/status byte | `0x00483D8E..0x00483DAC` | No for stock YR INI; conditional for TS/mod content |
| 12 | `5` or `2` | Terrain object: current-theater occupation bits `!= 7` returns `5`; `== 7` returns `2` | `0x00483DDF..0x00483E1E`, parser `0x0071DF00` | Yes |
| 13 | `0` | Default/fallthrough, including empty object list | `0x00483DD4` | Yes |

Tiny details with parity impact:

- `Crushable=` has priority over `Wall=`. An overlay with both bits becomes value `1`, not `2`. Active in YR: Yes; evidence `0x00483CB5` precedes `0x00483CCA`.
- Column `1` is not `Crate=` and not road art. The writer reads inherited `Crushable=` at `+0x22D`; `OverlayType+0x2AA Crate=` is not read here. Active in YR: Yes.
- `IsRubble=` is not an impassable shortcut. If reached, it goes to value `0`. Active in YR: Conditional; evidence `0x00483D1C..0x00483DD4`.
- Overlay-speed and base-land-speed checks differ: overlay speed uses exact `== 0.0f`; base land uses `<= 0.01` against a double constant. Active in YR: Yes.
- The speed slot is the Wheel column at `0x0089EA48`, not the Foot-column table base `0x0089EA40`. Active in YR: Yes.
- Water and beach return before the `<= 0.01` threshold. Active in YR: Yes.
- The LaserFence branch can be overwritten because it writes `6` and continues through the object loop/default path. Active in YR: Conditional/No stock; evidence `0x00483DCA..0x00483DD4`.

## 4. INI Keys

| Key | Binary field | Writer effect | Active in YR |
|---|---|---|---|
| `Crushable=` | ObjectType/OverlayType `+0x22D` | overlay cell writes `1` | Yes; stock overlays use inherited object-type flags |
| `Wall=` | OverlayType `+0x2A8` | overlay cell writes `2` only if `Crushable` failed | Yes; wall overlays |
| `Land=` | OverlayType `+0x298` | indexes Wheel-column speed for overlay strict-zero test | Yes |
| `IsARock=` | OverlayType `+0x2B5` | overlay cell writes `6` | Conditional |
| `IsRubble=` | OverlayType `+0x2B4` | overlay cell returns/falls through as `0` | Conditional |
| `[LandType] Wheel=` | speed table column at `0x0089EA48` | controls overlay/base impassability classification | Yes; loaded from rules at scenario init |
| `LaserFence=` | BuildingType `+0x16BF` | conditional non-returning write `6` | Conditional; no stock YR INI evidence |
| `FirestormWall=` | BuildingType `+0x16C0` | conditional returning write `6` | Conditional/TS legacy; no stock YR INI evidence |
| `TemperateOccupationBits=` | TerrainType `+0x2A8` | terrain object writes `5` unless value is `7`, then `2` | Yes |
| `SnowOccupationBits=` | TerrainType `+0x2AC` | same as above for snow theater | Yes/Conditional by theater |

## 5. Integration Points

| Integration point | Role | Evidence | Active in YR |
|---|---|---|---|
| `CellClass::RecalcAttributes @ 0x0047D2B0` | only direct caller found in this slice; invokes writer and mirrors result | call sites `0x0047D551`, `0x0047D7CD`, `0x0047DD36` | Yes |
| `MapClass+0x68` per-cell zone cache | receives `CellClass+0x4C` as byte 0 after recalculation | `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md` | Yes |
| `MapClass::UpdateBridgeZonesHelper @ 0x0056C510` | groups cached reduced zone types and builds per-`MovementZone` zone arrays | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` | Yes |
| `ZoneMap::FloodFillReachableZones @ 0x005840C0` | reads `CellClass+0x4C` as matrix column | reader report | Yes |
| `Zone_precheck @ 0x0042C290` | consumes reduced zone type through hierarchy records/matrix column | reader/precheck reports | Yes |

## 6. Current Rust Implementation Status

| Surface | Status vs writer |
|---|---|
| `src/sim/pathfinding/zone_build.rs` | Contains binary-shaped `MOVEMENT_CLASS_PASSABILITY[13][8]` and uses `ResolvedTerrainCell.zone_type` in the terrain-aware path, but `movement_class_for_cell` can override to Building from `PathGrid`. It does not itself implement the full `0x00483C80` overlay/object priority. |
| `src/sim/pathfinding/passability.rs` | Still exposes a local `LandType` column model and remapped `PASSABILITY_MATRIX`; comments correctly mention reduced ZoneType in places but still describe columns as local land types. |
| `src/sim/overlay_grid.rs` | Runtime overlay recalculation uses `crate_type -> wall -> tiberium -> is_gate`; this mismatches the verified writer priority and fields: `Crushable -> Wall -> overlay Wheel-speed==0 -> IsARock -> IsRubble->Ground`. |
| `src/sim/pathfinding/zone_map.rs` / `zone_search.rs` | Current zone grids and reduced precheck consume movement classes, but exact writer-side classification must be correct before matrix/zone parity can be trusted. |

No Rust files were modified in this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellClass::RecalcZoneType @ 0x00483C80` | verified | prior full Ghidra report plus adjacent reports | none for scoped branch order |
| Values written to `CellClass+0x4C` | verified | literal writes `0..7` in `0x00483C99..0x00483E1E` | none |
| Overlay branch priority | verified | `0x00483CB5..0x00483D24`, parser xrefs | exact stock overlay roster not enumerated here |
| Speed-table slot and thresholds | verified | `0x00483CE8`, `0x00483D57`, `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` | no re-extraction of all table values in this report |
| Base land branches | verified | `0x00483D2A..0x00483D6B` | none |
| Object loop shape | verified | `0x00483D72..0x00483E1E` | exact semantic label for building `+0x21C/+0x1FA` deferred |
| Active YR caller chain | verified | `0x0047D2B0` xrefs and full-passability report | no runtime watchpoint |
| Rust scan | touched-not-exhausted | selected file reads and `rg` | no tests run; no code edits by swarm rule |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- What writes CellClass+0x4C? -> CellClass::RecalcZoneType @ 0x00483C80 writes values 0..7.` (evidence: `0x00483C80`)
- `[RESOLVED] OQ-2 -- Is the function active in standard YR? -> Yes through RecalcAttributes during load and runtime mutations.` (evidence: `0x0047D551`, `0x0047D7CD`, `0x0047DD36`)
- `[RESOLVED] OQ-3 -- What are all written values? -> `0` default/ground, `1` Crushable overlay, `2` wall/terrain-occupation-7, `3` beach, `4` water, `5` terrain object/building-like occupation, `6` impassable/blocker, `7` outside playfield.` (evidence: `0x00483C99..0x00483E1E`)
- `[RESOLVED] OQ-4 -- Is column 1 road or crate? -> No; writer reads `Crushable=` at `+0x22D`; `Crate=` is not read here.` (evidence: `0x00483CB5`, `0x005F940A`)
- `[RESOLVED] OQ-5 -- Does Wall override Crushable? -> No; Crushable is checked first and returns value 1.` (evidence: `0x00483CB5..0x00483CD4`)
- `[RESOLVED] OQ-6 -- Is IsRubble impassable? -> No; it reaches/defaults to value 0.` (evidence: `0x00483D1C..0x00483DD4`)
- `[RESOLVED] OQ-7 -- Which speed column is used? -> Wheel column (`0x0089EA48`), both for overlay exact-zero and base-land `<=0.01` checks.` (evidence: `0x00483CE8`, `0x00483D57`, `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-8 -- Are water and beach threshold-gated? -> No; `LandType==2` and `LandType==6` return before the land-speed threshold.` (evidence: `0x00483D2A..0x00483D49`)
- `[RESOLVED] OQ-9 -- Does this result feed the passability matrix? -> Yes; it is mirrored by RecalcAttributes and consumed as the reduced-zone column by matrix readers.` (evidence: `0x0047D2B0`, `0x0056C510`, `0x005840C0`, `0x0042C290`)
- `[RESOLVED] OQ-10 -- Are TS legacy branches present? -> Yes for Firestorm/LaserFence-style building branches; stock YR INI grep found no normal activation.` (evidence: `0x00483D8E..0x00483DCD`, INI grep)
- `[DEFERRED] OQ-11 -- Exact semantic name of building `+0x21C/+0x1FA`.` (category: `requires-different-system-context`; reason: not needed for reduced-zone value recovery; next-step-if-pursued: audit BuildingClass/HouseClass fields around Firestorm wall status)
- `[DEFERRED] OQ-12 -- Runtime watchpoint confirmation for CellClass+0x4C writes.` (category: `needs-runtime-debugger`; reason: static Ghidra evidence and prior reports resolve branch semantics; next-step-if-pursued: watch writes during map load and overlay mutation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Overlay classification priority is `Crushable(+0x22D)->1`, then `Wall(+0x2A8)->2`, then overlay Wheel-speed exact `0.0`/`IsARock(+0x2B5)->6`, then `IsRubble(+0x2B4)->0`, before base land checks. | `0x00483CB5..0x00483D24`; parser xrefs `0x005F940A`, `0x005FE7D4`, `0x005FE9DE`, `0x005FE9FC` | mismatch: `overlay_grid.rs` uses `crate_type`, `wall`, `tiberium`, `is_gate` as the runtime priority | `src/sim/overlay_grid.rs`, overlay type registry, `src/map/resolved_terrain.rs` | Runtime overlay mutations must recompute the same reduced `zone_type` columns as gamemd. | Overlay with both `Crushable=yes` and `Wall=yes` yields zone type `1`; rubble-only overlay yields `0`. Proposed Rust test: `recalc_zone_type_overlay_priority_matches_gamemd`. | Do not use `Crate=` or road art as column 1; do not make `IsRubble` impassable. |
| Base land classification returns Water `4` for `LandType=2` and Beach `3` for `LandType=6` before applying Wheel-speed `<=0.01`; other positive-speed land falls through to Ground `0`. | `0x00483D2A..0x00483D6B`; `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` | partial: Rust has `ResolvedTerrainCell.zone_type`, but `passability.rs` still carries a local land-column compatibility model | `src/sim/pathfinding/passability.rs`, `src/map/resolved_terrain.rs`, `src/sim/pathfinding/zone_build.rs` | Preserve binary-facing reduced zone type separately from raw/local terrain enums. | Clear, Rough, Road, Railroad, Tunnel stock terrain with positive Wheel speed share reduced zone type `0`; Water is `4`; Beach is `3`. Proposed Rust test: `recalc_zone_type_land_speed_threshold_order_matches_gamemd`. | Do not use raw RA2/YR `LandType` or local Rust `LandType` directly as matrix column. |
| Terrain objects use current-theater occupation bits: selected value `!=7` writes `5`, selected value `==7` writes `2`. | `0x00483DDF..0x00483E1E`, parser `0x0071DFCA/0x0071DFD4` | unchecked/missing: terrain object blocking appears folded into `terrain_object_blocks`/PathGrid rather than exact reduced-zone columns | terrain object resolution feeding `ResolvedTerrainCell.zone_type`; `src/sim/pathfinding/zone_build.rs` | Terrain object occupation bits must affect reduced zone type, not just generic walkability. | Terrain object with current-theater occupation bits `7` classifies as `2`, while bits `4` classify as `5`. Proposed Rust test: `terrain_object_occupation_bits_select_zone_type_column`. | Do not collapse all terrain objects to one generic blocking/building zone type. |

### Negative Facts / Do Not Do

- Do not label reduced zone column `1` as `Crate`, `Road`, or visible road art. Active in YR: Yes; evidence `0x00483CB5` reads `+0x22D Crushable=`, not `OverlayType+0x2AA Crate=`.
- Do not let `Wall=` override `Crushable=`. Active in YR: Yes; evidence `+0x22D` is tested before `+0x2A8`.
- Do not make `IsRubble=` impassable. Active in YR: Conditional; evidence the branch reaches value `0`.
- Do not use the Foot speed column for RecalcZoneType impassability. Active in YR: Yes; evidence reads use `0x0089EA48`, the Wheel column.
- Do not treat the LaserFence write as an unconditional final value. Active in YR: Conditional/no stock; evidence the branch continues and can be overwritten.
- Do not rediscover or relitigate matrix reader semantics here: rows are `MovementZone`, columns are reduced zone types, and only matrix value `1` passes. Active in YR: Yes; settled by `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`.

### Remaining Uncertainty

- Exact semantic label for the building object `+0x21C/+0x1FA` status byte in the Firestorm branch remains unresolved; the branch is not active for stock YR INI content found in this slice.
- No live runtime watchpoint was run in this subagent; branch semantics are static Ghidra findings from prior Ghidra reports and adjacent verified documents.
- The report does not enumerate every stock overlay/terrain object instance that hits each branch; it only verifies the writer-side branch semantics.

### Stale Docs / Follow-up Docs

- `ZONE_PASSABILITY_VERIFIED.md`: replace "Column 1 is IsCrate" with "Column 1 is assigned when an overlay's inherited `ObjectTypeClass+0x22D Crushable=` flag is true; `OverlayTypeClass+0x2AA Crate=` is not read by `CellClass::RecalcZoneType @ 0x00483C80`."
- `TODO_ZONE_FIDELITY_FIXES.md`: replace "If road overlay (IsRoad) -> ZoneType 1 (Road)" with "If overlay `Crushable=` (`OverlayType/ObjectType+0x22D`) -> ZoneType 1; this check precedes `Wall=`."
- Rust comments in `src/sim/pathfinding/passability.rs` should avoid saying the matrix columns are local `LandType` columns. Correct wording: "The binary matrix columns are reduced `ZoneType` values from `CellClass::RecalcZoneType`; any local land-type remap is a compatibility layer."

## Sources

- Primary prior Ghidra report read: `C:/Users/enok/Documents/ra2-rust-game-docs/CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md`.
- Adjacent binary-backed reports read: `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md`, `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md`, `MOVEMENT_CLASSIFIERS_REFERENCE.md`, `CELLCLASS_ZONES_SPEED_BRIDGES.md`.
- Ghidra addresses cited from those reports: `0x00483C80`, `0x0047D2B0`, `0x0056C510`, `0x005840C0`, `0x0042C290`, `0x005F940A`, `0x005FE7A4`, `0x005FE7D4`, `0x005FE9DE`, `0x005FE9FC`, `0x0071DFCA`, `0x0071DFD4`.
- INI files searched: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust surfaces scanned: `src/sim/pathfinding/passability.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/overlay_grid.rs`.
