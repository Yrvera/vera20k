# CellClass::RecalcZoneType 0x00483C80 -- Ghidra Research Report

**Address(es):** `0x00483C80` primary writer, consumers `0x0047D2B0`, `0x0056C510`, `0x005840C0`, `0x0042C290`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact writer priority/order for reduced `CellClass+0x4C` zone type columns 0..7, immediate overlay/land/object/type offsets involved, and how the result feeds `ZonePassabilityMatrix`.  
**Non-Scope:** full terrain rendering, full `CellClass::RecalcAttributes`, complete A* search, and full parser/default audit for unrelated object type fields.  
**Confidence:** High for writer order and offsets; Medium for a few semantic labels where the binary proves offsets but not user-facing names.  
**Active in YR:** Yes. `CellClass::RecalcAttributes @ 0x0047D2B0` calls this helper at `0x0047D551`, `0x0047D7CD`, and `0x0047DD36`; `0x0047D2B0` is active during scenario load and runtime cell mutations.

## 1. Overview

`CellClass::RecalcZoneType @ 0x00483C80` writes a reduced 8-value zone type to `CellClass+0x4C`. `RecalcAttributes` mirrors it into `MapClass+0x68` byte 0; later zone builders/readers use it as the `ZonePassabilityMatrix[movementZone][zoneType]` column, where only matrix value `1` passes.

The writer is a priority classifier over playfield status, overlay flags, overlay land speed, base land type, base land speed, and then cell objects. It is not raw terrain rendering and not a raw `LandType` column writer.

## 2. Class Layout / Key Offsets

| Offset / storage | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `CellClass+0x24` | packed map coordinate passed to playfield test | `0x00483C8C..0x00483C90` calls `MapClass::Is_Cell_In_Playfield` | Yes |
| `CellClass+0x44` | overlay type index, `-1` means none | `0x00483CA4..0x00483CB2` | Yes |
| `CellClass+0x48` | `LandType` used after overlay checks | `0x00483D2A` | Yes |
| `CellClass+0x4C` | reduced zone type writer target | literal writes `0..7` in `0x00483C99..0x00483E1E` | Yes |
| `CellClass+0xE4` | first object/content pointer | `0x00483D72` starts object loop | Yes |
| object `+0x30` | next object in cell list | `0x00483DCD` advances loop | Yes |
| object vtable `+0x2C` | `WhatAmI` RTTI | `0x00483D80`; `ABSTRACTCLASS_GHIDRA_REPORT.md` maps `6=Building`, `0x24=Terrain` | Yes |
| `OverlayTypeClass+0x22D` | inherited `Crushable=` bool | `ObjectTypeClass::ReadINI @ 0x005F940A`; writer reads `0x00483CB5` | Yes |
| `OverlayTypeClass+0x298` | `Land=` enum | `OverlayTypeClass::ReadINI @ 0x005FE7A4`; writer reads `0x00483CDF` | Yes |
| `OverlayTypeClass+0x2A8` | `Wall=` bool | `OverlayTypeClass::ReadINI @ 0x005FE7D4`; writer reads `0x00483CCA` | Yes |
| `OverlayTypeClass+0x2B4` | `IsRubble=` bool | `OverlayTypeClass::ReadINI @ 0x005FE9FC`; writer reads `0x00483D1C` | Yes/Conditional |
| `OverlayTypeClass+0x2B5` | `IsARock=` bool | `OverlayTypeClass::ReadINI @ 0x005FE9DE`; writer reads `0x00483D07` | Yes/Conditional |
| `BuildingClass+0x520` | building type pointer | `0x00483D8E` | Conditional |
| `BuildingTypeClass+0x16BF` | `LaserFence=` bool | parser `0x004638F4`; writer reads `0x00483DB0` | No in stock YR grep; mod/legacy conditional |
| `BuildingTypeClass+0x16C0` | `FirestormWall=` bool | parser `0x00463909`; writer reads `0x00483D94` | No in stock YR grep; TS/mod conditional |
| `TerrainClass+0xC8` | terrain type pointer | `0x00483DF5`, `0x00483E0C` | Yes |
| `TerrainTypeClass+0x2A8` | `TemperateOccupationBits` | `TerrainTypeClass::ReadINI @ 0x0071DFCA`; writer compares to `7` | Yes |
| `TerrainTypeClass+0x2AC` | `SnowOccupationBits` | `TerrainTypeClass::ReadINI @ 0x0071DFD4`; writer compares to `7` | Yes |
| `ScenarioClass+0x1258` | theater selector branch (`==1` chooses snow bits) | `0x00483DDF..0x00483E18` | Yes/Conditional by map theater |
| `0x0089EA48 + land*9*4` | speed-table float slot used by this helper | reads at `0x00483CE8` and `0x00483D57`; writer xref from `0x0067413B` | Yes |
| `0x007E1748` | float `0.0` | memory bytes all zero; compare at `0x00483CEF` | Yes |
| `0x007E3808` | double `0.01` | bytes `7b14ae47e17a843f`; compare at `0x00483D5E` | Yes |

## 3. Core Logic

| Priority | Result | Condition | Evidence | Active in YR |
|---:|---:|---|---|---|
| 1 | `7` | `MapClass::Is_Cell_In_Playfield(coord, 1)` returns false | `0x00483C90..0x00483C99` | Yes |
| 2 | `1` | overlay exists and `OverlayType+0x22D Crushable` is true | `0x00483CB5..0x00483CBF`; parser `0x005F940A` | Yes |
| 3 | `2` | overlay exists and `OverlayType+0x2A8 Wall` is true, after Crushable failed | `0x00483CCA..0x00483CD4`; parser `0x005FE7D4` | Yes |
| 4 | `6` | overlay exists and speed slot at `0x89EA48 + OverlayType.Land*9*4` equals exactly `0.0f` | `0x00483CDF..0x00483CFC`; constant `0x007E1748` | Yes |
| 5 | `6` | overlay exists and `OverlayType+0x2B5 IsARock` is true | `0x00483D07..0x00483D11`; parser `0x005FE9DE` | Yes/Conditional |
| 6 | `0` | overlay exists and `OverlayType+0x2B4 IsRubble` is true; earlier overlay tests did not return | `0x00483D1C..0x00483DD4`; parser `0x005FE9FC` | Yes/Conditional |
| 7 | `4` | `CellClass+0x48 LandType == 2` water | `0x00483D2A..0x00483D36` | Yes |
| 8 | `3` | `CellClass+0x48 LandType == 6` beach | `0x00483D40..0x00483D49` | Yes |
| 9 | `6` | speed slot at `0x89EA48 + LandType*9*4` is `<= 0.01` | `0x00483D54..0x00483D6B`; constant `0x007E3808` | Yes |
| 10 | `6` then continues | `WhatAmI == 6` building, `BuildingType+0x16C0 FirestormWall == 0`, `BuildingType+0x16BF LaserFence != 0`, and building state `+0x618` is neither `0xC` nor `8` | `0x00483D83..0x00483DCD` | Conditional; no stock YR `LaserFence=`/`FirestormWall=` entries found |
| 11 | `6` and returns | `WhatAmI == 6` building, `BuildingType+0x16C0 FirestormWall != 0`, and building field `+0x21C` has byte `+0x1FA != 0` | `0x00483D8E..0x00483DAC` | No for standard YR content found; TS/mod path |
| 12 | `5` or `2` | `WhatAmI == 0x24` terrain object; current theater picks `TerrainType+0x2A8` or `+0x2AC`; value `!=7` returns `5`, value `==7` returns `2` | `0x00483DDF..0x00483E1E`; parser `0x0071DFCA/0x0071DFD4` | Yes |
| 13 | `0` | default fallthrough, including empty object list | `0x00483DD4` | Yes |

Tiny details:

- The overlay `Crushable` check has higher priority than `Wall`. A stock overlay with both `Crushable=yes` and `Wall=yes` becomes column `1`, not column `2`. Active in YR: Yes.
- `IsRubble` is not an impassable shortcut. If it fires after earlier overlay checks fail, it jumps to the default `ZoneType=0`. Active in YR: Yes/Conditional.
- Water and beach land types are checked before the `<=0.01` land-speed threshold. Active in YR: Yes.
- The first overlay speed test is strict equality with `0.0f`; the later base-land speed test is `<= 0.01` against a double constant. Active in YR: Yes.
- The building LaserFence branch writes `6` but does not return; a later default write can overwrite it if no later returning object branch fires. Active in YR: Conditional/No for stock content.

## 4. INI Keys

| Key | Binary field | Effect in this writer | Stock YR activity |
|---|---|---|---|
| `Crushable=` | ObjectType/OverlayType `+0x22D` | ZoneType `1` when overlay present | Yes |
| `Wall=` | OverlayType `+0x2A8` | ZoneType `2`, but only after `Crushable` fails | Yes |
| `Land=` | OverlayType `+0x298` | indexes speed table for overlay strict-zero test | Yes |
| `NoUseTileLandType=` | OverlayType `+0x2AC` | not read directly here; affects `LandType` earlier in `RecalcAttributes` | Yes |
| `IsARock=` | OverlayType `+0x2B5` | ZoneType `6` | Yes/Conditional |
| `IsRubble=` | OverlayType `+0x2B4` | ZoneType `0` escape/fallthrough | Yes/Conditional |
| `LaserFence=` | BuildingType `+0x16BF` | conditional write `6`, non-returning | No in stock YR grep; mod/legacy conditional |
| `FirestormWall=` | BuildingType `+0x16C0` | conditional write `6`, returning if owner/status flag also true | No in stock YR grep; TS/mod conditional |
| `TemperateOccupationBits=` | TerrainType `+0x2A8` | terrain object: `!=7` -> `5`, `==7` -> `2` on non-snow theater | Yes |
| `SnowOccupationBits=` | TerrainType `+0x2AC` | terrain object: `!=7` -> `5`, `==7` -> `2` on snow theater | Yes |

## 5. Integration Points

- `CellClass::RecalcAttributes @ 0x0047D2B0` is the only direct caller found: calls at `0x0047D551`, `0x0047D7CD`, and `0x0047DD36`. Active in YR: Yes.
- After the helper returns, `RecalcAttributes` mirrors `CellClass+0x4C` to `MapClass+0x68[index*4+0]`; it also mirrors cell level to `MapClass+0x68[index*4+1]` and `MapClass+0x70[index*10+8]`. Active in YR: Yes.
- `MapClass::UpdateBridgeZonesHelper @ 0x0056C510` groups cells by `MapClass+0x68` byte 0 and creates per-`MovementZone` zone arrays from `ZonePassabilityMatrix`; only matrix value `1` passes. Active in YR: Yes.
- `ZoneMap::FloodFillReachableZones @ 0x005840C0` reads `CellClass+0x4C` directly as the matrix column. Active in YR: Yes.
- `Zone_precheck @ 0x0042C290` consumes edge zone types as matrix columns during hierarchical path precheck. Active in YR: Yes.

## 6. Current Rust Implementation Status

| Surface | Status vs writer |
|---|---|
| `src/sim/pathfinding/zone_build.rs` | Has binary-shaped `MOVEMENT_CLASS_PASSABILITY[13][8]` and uses `ResolvedTerrainCell.zone_type` for terrain-aware rebuilds. `movement_class_for_cell` can override to `BUILDING` from `PathGrid`, but it does not implement the exact `0x00483C80` overlay/object priority. |
| `src/sim/pathfinding/passability.rs` | Still exposes a local `LandType` column model and `PASSABILITY_MATRIX` remapping. Comments call column 1 road and conflate local land columns with binary reduced zone types. |
| `src/sim/overlay_grid.rs` | Runtime overlay update approximates priority as `crate_type -> wall -> tiberium -> is_gate`; this does not match the verified writer: `Crushable(+0x22D) -> Wall(+0x2A8) -> overlay speed==0 -> IsARock(+0x2B5) -> IsRubble(+0x2B4)->Ground`. |
| `src/app_sim_tick.rs` | Dynamic path grid rebuild blocks walls through `PathGrid`, then rebuilds zones; this can conceal reduced-zone column mistakes for some cells because object blocking and zone type are coupled. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00483C80` writer | verified | full decompile + assembly context | none |
| Playfield branch | verified | `0x00483C90..0x00483C99`, `0x00578460` | none |
| Overlay priority/order | verified | `0x00483CB5..0x00483D24`, parsers `0x005F92D0`, `0x005FE770` | none for offsets |
| Base land water/beach/speed threshold | verified | `0x00483D2A..0x00483D6B`; constants `0x007E1748`, `0x007E3808` | actual speed table content comes from separate Rules load |
| Object loop | verified | `0x00483D72..0x00483E1E`; `ABSTRACTCLASS_GHIDRA_REPORT.md` | user-facing label for building `+0x21C/+0x1FA` deferred |
| Terrain object branch | verified | `0x00483DDF..0x00483E1E`, `0x0071DF00` | exact theater enum labels beyond `==1` snow branch deferred |
| ZonePassabilityMatrix consumers | touched-not-exhausted | prior reader report + spot decomp `0x0056C510`, `0x005840C0`, `0x0042C290` | full A* not repeated by scope |
| Rust surface scan | touched-not-exhausted | selected file reads and `rg` | no Rust edits/tests by swarm rule |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- What writes CellClass+0x4C? -> CellClass::RecalcZoneType @ 0x00483C80, called only from RecalcAttributes in this xref slice.` (evidence: `0x00483C80`, xrefs from `0x0047D551/0x0047D7CD/0x0047DD36`)
- `[RESOLVED] OQ-2 -- Exact column priority? -> playfield, overlay Crushable, overlay Wall, overlay speed==0, IsARock, IsRubble->Ground, water, beach, land speed<=0.01, object loop, default.` (evidence: `0x00483C90..0x00483E1E`)
- `[RESOLVED] OQ-3 -- Is column 1 Crate/Road? -> No; writer reads inherited Crushable= at OverlayType+0x22D.` (evidence: `0x00483CB5`, `ObjectTypeClass::ReadINI @ 0x005F940A`)
- `[RESOLVED] OQ-4 -- Is IsRubble impassable? -> No; it jumps to the default ZoneType=0 return.` (evidence: `0x00483D1C..0x00483DD4`)
- `[RESOLVED] OQ-5 -- What constants gate speed tests? -> overlay land speed equals float 0.0; base land speed compares <= double 0.01.` (evidence: `0x00483CEF`, `0x00483D5E`; memory `0x007E1748`, `0x007E3808`)
- `[RESOLVED] OQ-6 -- Does this feed ZonePassabilityMatrix? -> Yes; RecalcAttributes mirrors +0x4C to zone cache, then matrix readers consume the reduced zone type as column and require value 1 to pass.` (evidence: `0x0047D551`; `0x0056C510`, `0x005840C0`, `0x0042C290`)
- `[RESOLVED] OQ-7 -- Is the writer active in standard YR? -> Yes through load/runtime RecalcAttributes; some TS/mod-specific building subbranches are inactive in stock INI.` (evidence: caller xrefs; INI grep for `LaserFence=`/`FirestormWall=`)
- `[DEFERRED] OQ-8 -- Exact semantic name of building object field +0x21C/+0x1FA.` (category: out-of-scope; reason: not needed to recover reduced zone type columns; next-step-if-pursued: run a BuildingClass/HouseClass field audit)
- `[DEFERRED] OQ-9 -- Runtime watchpoint on CellClass+0x4C.` (category: needs-runtime-debugger; reason: debugger not running and static xrefs were sufficient for this slice; next-step-if-pursued: watch writes during map load and overlay mutation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Overlay priority is `Crushable(+0x22D)->1`, `Wall(+0x2A8)->2`, overlay speed exact `0.0` or `IsARock(+0x2B5)->6`, `IsRubble(+0x2B4)->0`, before base land checks. | `0x00483CB5..0x00483D24`; parsers `0x005F92D0`, `0x005FE770` | mismatch: `overlay_grid.rs` uses `crate_type`, `wall`, `tiberium`, `is_gate` instead of verified order/fields | `src/sim/overlay_grid.rs`, overlay type registry, `src/sim/pathfinding/zone_build.rs` | Runtime overlay mutations must recompute the same reduced `zone_type` columns as `0x00483C80` | Cell with overlay flags `Crushable=yes, Wall=yes` yields `ZoneType=1`; rubble overlay with `IsRubble=yes` and no prior overlay blocker yields `ZoneType=0`; proposed test `recalc_zone_type_overlay_priority_matches_gamemd` | Do not use `Crate=` or visible road art as column 1 |
| Base land type returns Water `4` for `LandType=2` and Beach `3` for `LandType=6` before applying the `<=0.01` speed threshold; all other positive-speed land falls through to Ground `0`. | `0x00483D2A..0x00483D6B` | partial: `passability.rs` still exposes local LandType columns and remapping comments; `zone_build.rs` uses `cell.zone_type` when available | `src/sim/pathfinding/passability.rs`, `src/map/resolved_terrain.rs`, `src/sim/pathfinding/zone_build.rs` | Preserve a binary-facing reduced zone type separate from local land/speed-cost enums | Clear/Rough/Road/Railroad TMP cells with positive speed share `ZoneType=0`; Water is `4`; Beach is `3`; proposed test `recalc_zone_type_land_speed_threshold_order_matches_gamemd` | Do not drive matrix columns directly from raw TMP or local `LandType` enum |
| Terrain objects use theater-specific occupation bits: selected `TerrainType+0x2A8/+0x2AC != 7` returns `5`, selected value `==7` returns `2`. | `0x00483DDF..0x00483E1E`; `TerrainTypeClass::ReadINI @ 0x0071DF00` | unchecked/missing: terrain object zone type appears folded into block flags and `terrain_object_blocks` rather than exact occupation-bit columns | terrain object resolution feeding `ResolvedTerrainCell.zone_type`, `src/sim/pathfinding/zone_build.rs` | Terrain object occupation bits must affect reduced zone type, not just walkability | A terrain object with current-theater occupation bits `7` classifies as Wall `2`, while one with bits `4` classifies as Building `5`; proposed test `terrain_object_occupation_bits_select_zone_type_column` | Do not collapse all terrain objects to generic building/blocker |

### Negative Facts / Do Not Do

- Do not label column `1` as `Crate` or road art. Active in YR: Yes; evidence: `0x00483CB5` reads `+0x22D`, and `ObjectTypeClass::ReadINI @ 0x005F940A` parses `Crushable=` there. `OverlayType+0x2AA Crate` is not read in this writer.
- Do not make `IsRubble` impassable. Active in YR: Yes/Conditional; evidence: `0x00483D1C..0x00483DD4` jumps to `ZoneType=0`.
- Do not let `Wall=` override `Crushable=`. Active in YR: Yes; evidence: `+0x22D` is tested before `+0x2A8` at `0x00483CB5..0x00483CD4`.
- Do not apply the land speed threshold before water/beach. Active in YR: Yes; evidence: `LandType==2` and `==6` return before `0x00483D57..0x00483D6B`.
- Do not treat the building `LaserFence` write to `6` as a guaranteed final return. Active in YR: Conditional/No for stock content; evidence: `0x00483DCA` writes `6` then `0x00483DCD` continues the object loop and `0x00483DD4` can write `0`.

### Remaining Uncertainty

- Exact semantic label for building object `+0x21C/+0x1FA` in the `FirestormWall` branch is deferred; the branch is not stock-YR active by INI grep.
- No runtime debugger/watchpoint proof was collected because the debugger server was not running.
- The precise speed-table load values for every land type were not re-extracted here; this report only verifies the writer's slot/threshold use.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/ZONE_PASSABILITY_VERIFIED.md`: replace "Column 1 is IsCrate" with "Column 1 is assigned when an overlay's inherited `ObjectTypeClass+0x22D Crushable=` flag is true; `OverlayTypeClass+0x2AA Crate=` is not read by `CellClass::RecalcZoneType @ 0x00483C80`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/TODO_ZONE_FIDELITY_FIXES.md`: replace "If road overlay (IsRoad) -> ZoneType 1 (Road)" with "If overlay `Crushable=` (`OverlayType/ObjectType+0x22D`) -> ZoneType 1; this check precedes `Wall=`."
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/passability.rs`: replace comments describing matrix columns as local `LandType` with "The binary matrix columns are reduced `ZoneType` values from `CellClass::RecalcZoneType`; local land-type remaps must be treated as compatibility shims, not the source of truth."

## Sources

- Ghidra decompiled/read: `0x00483C80`, `0x0047D2B0`, `0x00578460`, `0x0056C510`, `0x005840C0`, `0x0042C290`, `0x005F92D0`, `0x005FE770`, `0x0071DF00`, `0x00461000`.
- Ghidra assembly context: `0x00483C90..0x00483E1E`, parser string xrefs `0x005F940A`, `0x005FE7A4`, `0x005FE7D4`, `0x005FE9DE`, `0x005FE9FC`, `0x0071DFCA`, `0x0071DFD4`.
- Ghidra memory: `0x007E1748`, `0x007E3808`, `0x0082A594`.
- Prior docs: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md`, `ZONE_PASSABILITY_VERIFIED.md`, `WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md`, `ABSTRACTCLASS_GHIDRA_REPORT.md`.
- INI grep: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scan: `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/passability.rs`, `src/sim/overlay_grid.rs`, `src/sim/pathfinding/core.rs`, `src/app_sim_tick.rs`.
