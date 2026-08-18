# Pier / Bridge / Water Classification Reswarm - Slot 4

**Date:** 2026-05-27  
**Swarm slot:** 4  
**Claimed scope:** Active-YR terrain classification for pier/bridge/waterbridge/shore cells as it feeds `CellClass::RecalcZoneType`, reduced `ZoneType`, bridge layer routing, and low-bridge/tube handling.  
**Non-scope:** Full `UnitClass::Can_Enter_Cell`, A* neighbor ordering, path smoothing, or implementation.  
**Active in standard YR:** Yes for `RecalcAttributes`, `RecalcZoneType`, water/shore tiles, high bridges, low-bridge/tube cells, and LAT fixup. Lunar-specific zeroing disables most water/bridge theater globals for Lunar.

## Executive Summary

The binary does not have a single "pier" passability class in the checked evidence. Cells are classified by final `LandType`, reduced `ZoneType`, bridge tile-range predicates, overlay bridge stamps, and low-bridge tube state.

Verified active-YR facts:

- Water is `CellClass+0xEC LandType == 2`, then `RecalcZoneType` writes `ZoneType == 4`.
- Beach/shore is `LandType == 6`, then `RecalcZoneType` writes `ZoneType == 3`.
- High bridge tile identity is tile-range based: `IsBridge` checks `[BridgeSet, BridgeSet+16)` and `IsWoodBridge` checks `[WoodBridgeSet, WoodBridgeSet+16)`.
- Low bridge identity is not overlay ID alone. `IsLowBridgeCell` requires a valid signed tube index at `CellClass+0x116` and `LandType == 10`.
- `WaterBridge` is verified as a theater/LAT exemption range of exactly two tiles. This slot did not find a verified binary path that treats `WaterBridge` by name as a bridge deck or ground-passable class. Its movement classification must still come from the final TMP/land/tube/bridge mechanisms.

Current Rust likely has a player-visible drift risk around water/pier-like tiles because `PathGrid::from_resolved_terrain_with_bridges` makes `ground_walkable = true` for water cells and depends on later cost/zone gates to reject non-water movers. That is not the gamemd reduced-zone mechanism and is unsafe for helpers/smoothing/callers that read `PathGrid` as a complete cell-entry oracle.

## Sources Read

### Ghidra read-only decompilation in this pass

- `CellClass::RecalcZoneType @ 0x00483C80`
- `CellClass::RecalcAttributes @ 0x0047D2B0`
- `CellClass::ApplyLAT_and_SlopeFixup @ 0x0047CA80`
- `CellClass::IsOnBridgeSurface @ 0x00485060`
- `CellClass::IsShorePieceTile @ 0x004865B0`
- `CellClass::IsBridge @ 0x00486750`
- `CellClass::IsWoodBridge @ 0x00486770`
- `CellClass::IsLowBridgeCell @ 0x00484AB0`
- `CellClass::GetTubeAtCell @ 0x00484F20`
- `MapClass::ApplyBridgeTile @ 0x0057B440`
- TMP terrain-type to land-type helper `0x00544BE0`

### Research docs used

- `docs/research/CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md`
- `docs/research/SEA_TILES_GHIDRA_REPORT.md`
- `docs/research/WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md`
- `docs/research/bridges/02-cell-state-layering-zones/LOW_BRIDGE_ZONE_PRECHECK_LANDTYPE10_CONNECTIVITY_GHIDRA_REPORT.md`
- `docs/research/bridges/02-cell-state-layering-zones/BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_DOC_VERIFICATION.md`

### Rust touchpoints read

- `src/map/resolved_terrain.rs`
- `src/map/theater.rs`
- `src/map/overlay_types.rs`
- `src/sim/pathfinding/core.rs`
- `src/sim/pathfinding/passability.rs`
- `src/sim/pathfinding/terrain_cost.rs`

## VERIFIED Binary Findings

### 1. `RecalcZoneType` reduced-zone priority is final LandType driven

`CellClass::RecalcZoneType @ 0x00483C80` writes `CellClass+0x4C` using this relevant priority:

1. Out of playfield -> `ZoneType 7`.
2. Overlay branches: crushable -> `1`, wall -> `2`, overlay speed zero or rock -> `6`, rubble -> default ground.
3. Base `LandType == 2` -> `ZoneType 4` water.
4. Base `LandType == 6` -> `ZoneType 3` beach.
5. Base speed table threshold `<= 0.01` -> `ZoneType 6`.
6. Object/terrain occupancy branches.
7. Default -> `ZoneType 0` ground.

This function is active in YR. `CellClass::RecalcAttributes @ 0x0047D2B0` calls it during map load and runtime cell mutation, then mirrors the result into zone-map caches.

Important consequences:

- Water is not collapsed to generic impassable. It gets a distinct matrix column (`4`) before wheel-speed rejection can fire.
- Beach/shore is not ground. It gets a distinct matrix column (`3`).
- Ordinary ground, road art, rough, railroad, and tunnel-like positive-speed terrain can all fall through to reduced `ZoneType 0` unless an earlier overlay/object branch overrides.

### 2. Water classification

`CellClass::IsOnBridgeSurface @ 0x00485060` checks:

```text
g_WaterSet <= CellClass+0x38 IsoTileTypeIndex < g_WaterSet + 0xE
```

This is a 14-tile water-set range check. The function name is TS-legacy; checked docs and decompile show it is used as a water-surface tile predicate, notably for placement validation.

`CellClass::RecalcZoneType @ 0x00483C80` does not call this range predicate directly. It classifies water from final `LandType == 2`, written earlier by `RecalcAttributes` from TMP/theater tile data.

Known class:

- `WaterSet`: 14 tiles, active for water classification through final TMP land data and related predicates.

Unknown / not proved here:

- This pass did not enumerate every TMP cell inside the water set. The mechanism is verified; per-tile table content is delegated to retail TMP assets.

### 3. Shore / beach classification

`CellClass::IsShorePieceTile @ 0x004865B0` checks:

```text
g_ShorePieces <= CellClass+0x38 IsoTileTypeIndex < g_ShorePieces + 0x2A
```

The shore-piece range is 42 tiles. The active movement consequence is final `LandType == 6`, which `RecalcZoneType` maps to reduced `ZoneType 3` beach.

Known class:

- `ShorePieces`: 42 tiles, active YR theater range.

Unknown / not proved here:

- The exact visual/orientation meaning of each of the 42 shore tiles was not enumerated.

### 4. `WaterBridge` is a LAT exemption, not a verified movement class by itself

`CellClass::ApplyLAT_and_SlopeFixup @ 0x0047CA80` computes hardcoded LAT exemption ranges. For the Green LAT branch, it exempts:

```text
g_ShorePieces .. g_ShorePieces + 0x29
g_WaterBridge .. g_WaterBridge + 1
```

The range is inclusive in the decompile through precomputed high bounds, so it covers exactly two `WaterBridge` tiles. Theater INI confirms `WaterBridge=76` and `TileSet0076` / corresponding urban shifted tile set has `SetName = Water bridge`, `FileName = wbrdge`, `TilesInSet = 2`.

This pass did not find a `RecalcZoneType`, `IsBridge`, `IsWoodBridge`, or `IsLowBridgeCell` branch that reads `g_WaterBridge` as movement truth. Therefore:

- VERIFIED: `WaterBridge` affects LAT/visual transition handling.
- UNCHECKED: whether the two `wbrdge` TMPs carry Water, Tunnel, Beach, or another terrain byte in every theater asset.
- UNCHECKED/DRIFT risk: Rust treats any tileset whose `SetName` contains `"water"` as water in `TheaterLookup::is_water`, which includes `"Water bridge"`. That may be right only if the retail TMP land data agrees; the binary evidence says movement classification comes from final TMP/land/tube/bridge mechanisms, not the set name string.

### 5. High bridge identity is tile range based

`CellClass::IsBridge @ 0x00486750` returns true when:

```text
BridgeSet != -1
BridgeSet <= IsoTileTypeIndex < BridgeSet + 0x10
```

`CellClass::IsWoodBridge @ 0x00486770` returns true when:

```text
WoodBridgeSet != -1
WoodBridgeSet <= IsoTileTypeIndex < WoodBridgeSet + 0x10
```

Bridge zone lifecycle docs show high bridge record construction uses these predicates, plus direction/height lookup tables. This is separate from overlay ID alone.

Known classes:

- `BridgeSet`: first 16 tile IDs are concrete high-bridge candidates.
- `WoodBridgeSet`: first 16 tile IDs are wood high-bridge candidates.

Unknown / not repeated here:

- The per-offset direction/height tables for those 16 tiles are documented elsewhere and were not re-extracted in this slot.

### 6. Low bridge / tube identity is not overlay ID alone

`CellClass::IsLowBridgeCell @ 0x00484AB0` returns true only when:

```text
0 <= *(i16 *)(cell+0x116) < g_TubeArray.count
and *(i32 *)(cell+0xEC) == 10
```

`CellClass::GetTubeAtCell @ 0x00484F20` bounds-checks only `cell+0x116`; it does not check `LandType == 10`.

`CellClass::RecalcAttributes @ 0x0047D2B0` constructs TubeClass shells only for qualifying `LandType == 10` cells whose `IsoTileTypeIndex` lies in one of four exact 4-tile tunnel/low-bridge ranges. The constructor direction source is the four-byte direction table `[2, 4, 6, 0]`.

Bridge zone lifecycle docs show low bridge records are created when a cell is not `IsBridge`/`IsWoodBridge` but is `IsLowBridgeCell`, then `GetTubeAtCell` supplies endpoint data.

Known class:

- Low bridge pathing identity: valid tube index plus final `LandType == 10`.

Known stale wording in older docs:

- Claims that low bridge overlays simply convert water to tunnel or that there is one tube per whole bridge are too broad. Verified evidence proves the predicate and constructor/shell path, but not every destruction/repair lifecycle mutation.

## Inference From Verified Findings

### Pier-like cells

No checked binary evidence names a separate "pier" reduced zone or pathing class. A pier-looking cell must be one of:

- a water/shore/waterbridge TMP tile with final `LandType` deciding `ZoneType`;
- a high bridge cell identified by `BridgeSet`/`WoodBridgeSet` range and bridge state;
- a low bridge/tube cell identified by `LandType == 10` plus `cell+0x116`;
- an ordinary ground/building/overlay cell.

Therefore a Rust fix should not add a hardcoded "pier" category without tracing the exact map cell's final tile ID, TMP terrain byte, overlay ID, bridge facts, and tube index.

## Current Rust Touchpoints

### `src/map/resolved_terrain.rs`

Observed current shape:

- `zone_class` constants match the reduced 0..7 concept.
- TMP merge stores both local `land_type` and `yr_cell_land_type`; TMP byte `5` is special-cased to `YR_CELL_LAND_TUNNEL = 10`.
- `zone_type` is computed during terrain resolution and broadly follows `RecalcZoneType` for water/beach/overlay branches.
- `build_auto_low_bridge_tubes` creates `TubeFact::auto_low_bridge` only when `yr_cell_land_type == 10` and the tile falls in a Rust-recognized low-bridge direction range.
- `classify_overlay_effects` treats hardcoded bridge overlay indices as bridge layers. Low bridge overlays set `is_low_bridge` and are excluded from `bridge_walkable`.
- `TheaterLookup::is_water` in `src/map/theater.rs` returns true when the tileset `SetName` contains `"water"`, which includes `Water bridge`.

Rust risks:

- `WaterBridge` classification by set-name substring is not the verified binary movement mechanism.
- Low bridge handling has some binary-shaped pieces (`yr_cell_land_type == 10`, auto tubes), but overlay classification still includes comments/branches that can make low bridges look like road/passable overlay surfaces. The binary predicate is final `LandType == 10` plus tube index.
- High bridge identity in Rust depends on overlay stamping plus theater tile/ramp facts. Binary high-bridge zone identity also depends on `BridgeSet`/`WoodBridgeSet` tile ranges. Any map state where overlay stamping and final tile range disagree is parity-sensitive.

### `src/sim/pathfinding/core.rs`

`PathGrid::from_resolved_terrain_with_bridges` currently computes:

```text
ground_walkable = ... !cell.ground_walk_blocked || cell.is_water
```

with earlier exceptions for bridge structural cells and bridge transitions.

This intentionally keeps water ground-walkable and relies on `TerrainCostGrid` / mover-specific logic to reject non-water ground movers. That is a dangerous architectural split for any caller that treats `PathGrid::is_walkable` or smoothing/helper predicates as gamemd-style `Can_Enter_Cell`.

### `src/sim/pathfinding/terrain_cost.rs`

The cost grid can block non-water movers if the final `SpeedType` cost is zero. That helps normal A* calls that carry a correct terrain-cost grid, but it does not fix every path legality consumer. The binary uses final cell legality/matrix concepts, not a generic walkable-water boolean plus optional cost overlay.

## DRIFT / UNCHECKED Findings

### D1 - DRIFT - `PathGrid` marks water as ground-walkable

**Evidence:** `src/sim/pathfinding/core.rs` uses `!cell.ground_walk_blocked || cell.is_water` for `ground_walkable`.

**Binary baseline:** Water is `ZoneType 4`; normal ground movement-zone rows block water through the reduced-zone passability matrix. Water is not a generic ground-walkable cell.

**Impact:** Any caller using `PathGrid` without the exact mover legality/cost/matrix gate can accept water or pier-like water cells for ground units.

### D2 - UNCHECKED/DRIFT risk - `WaterBridge` set-name water heuristic

**Evidence:** Rust `TheaterLookup::is_water` uses `SetName` substring `"water"`. Retail theater INI uses `SetName = Water bridge`.

**Binary baseline:** `WaterBridge` is verified as a two-tile Green-LAT exemption. Movement classification by `g_WaterBridge` was not verified. Final TMP land/tube/bridge state remains the movement source of truth.

**Impact:** WaterBridge/pier-looking cells may be over-classified as water in Rust if TMP land data or low-bridge/tube semantics say otherwise.

### D3 - DRIFT - Low bridge overlay identity can still leak into path layer semantics

**Evidence:** Rust identifies low bridge overlays in `classify_overlay_effects`; low bridge overlays set `has_bridge_deck` but not `bridge_walkable`, and comments mention road overrides. Rust also has binary-shaped `yr_cell_land_type == 10` tube handling.

**Binary baseline:** `IsLowBridgeCell` ignores overlay ID and requires valid tube index plus `LandType == 10`.

**Impact:** Maps with low bridges can differ if overlay ID is used as movement truth, or if a LandType-10 tube cell is not present where binary would require it.

### D4 - UNCHECKED - Per-theater `WaterBridge` TMP terrain bytes

**Evidence:** Theater INI proves `wbrdge` is a two-tile set in Temperate/Desert and shifted in Urban. This slot did not parse retail TMP payloads for the two `wbrdge` tiles.

**Required follow-up:** Dump `wbrdge*.tem/sno/urb/des` TMP terrain bytes and compare to gamemd final `LandType` after `RecalcAttributes`.

### D5 - UNCHECKED - Exact "pier" map-cell class

**Evidence:** No distinct binary "pier" class was found in research-index/doc search or Ghidra functions checked here.

**Required follow-up:** For the concrete repro map, log the offending cell's final tile ID, tileset name, TMP terrain byte, overlay ID, bridge flags, `zone_type`, `tube_index`, and Rust `PathCell` fields.

## Implementation Handoff

Do not implement from this slot alone if the target is a specific pier symptom. First capture one repro cell and classify it through the fields below.

Required model boundaries:

1. Treat `ResolvedTerrainCell.zone_type` as the binary-facing reduced zone classification, not `PathGrid.ground_walkable`.
2. Do not let `PathGrid::is_walkable` stand in for gamemd `Can_Enter_Cell` / `CheckPassability` for ground units near water/shore/bridge cells.
3. Preserve separate meanings:
   - `LandType == 2` -> water -> reduced `ZoneType 4`.
   - `LandType == 6` -> beach -> reduced `ZoneType 3`.
   - `LandType == 10` plus valid tube index -> low bridge/tunnel identity.
   - `BridgeSet`/`WoodBridgeSet` first 16 tiles -> high bridge tile identity.
   - `WaterBridge` -> verified LAT exemption only unless TMP/cell evidence proves more.
4. For WaterBridge/pier-looking cells, prefer retail TMP data over tileset-name heuristics.
5. Fix order should likely be:
   - add a route/cell diagnostic dump for the repro;
   - remove or quarantine water-as-ground-walkable from generic `PathGrid` consumers;
   - route all mover-specific goal redirection/smoothing/helper checks through reduced-zone/movement-zone legality plus speed cost and bridge/tube layer rules;
   - only then adjust `WaterBridge` classification if the repro dump proves it is wrong.

Acceptance scenarios:

- A normal ground vehicle cannot be redirected, smoothed, scattered, or staged onto a `ZoneType 4` water cell, even if `PathGrid` has water metadata.
- An amphibious/hover/naval unit still respects Water/Beach matrix rows and speed type costs.
- A high bridge route uses bridge-layer state only for cells that are valid high bridge tile/bridge-fact cells.
- A low bridge route is valid only when the cell is `LandType == 10` and has a valid tube index.
- A WaterBridge tile is classified from its final TMP/land/tube state, not from `SetName` alone.

## Open Questions

1. What exact map/cell produces the reported "drives outside the pier/on water" symptom?
2. What are the retail TMP terrain bytes for the two `WaterBridge` tiles in each theater?
3. Does the concrete repro cell have a bridge overlay, a bridge tile-range ID, a low-bridge tube index, or only a water/shore TMP tile?
4. Do all relevant Rust callers pass a `TerrainCostGrid` and resolved terrain into A*, or do any still consume `PathGrid::is_walkable` as a complete legality answer?
5. Are there stock maps where WaterBridge is used without a low-bridge/tube record, and if so what does gamemd classify those cells as after `RecalcAttributes`?

## Shared Claims

- `CellClass::RecalcZoneType @ 0x00483C80` is active YR and writes reduced `ZoneType` to `CellClass+0x4C`.
- Water pathing legality is not "walkable terrain"; it is reduced `ZoneType 4` plus MovementZone/SpeedType legality.
- Shore/beach pathing legality is reduced `ZoneType 3`; only amphibious-family movement zones pass it in stock behavior.
- High bridge identity is first-16 tile-range based for `BridgeSet` / `WoodBridgeSet`, plus bridge state/layer handling.
- Low bridge identity is `cell+0x116` valid tube index plus `LandType == 10`, not overlay ID alone.
- `WaterBridge` is verified as a two-tile LAT exemption. Treat movement semantics as UNCHECKED until TMP/cell data is dumped.
- Current Rust `PathGrid` water-as-ground-walkable behavior is a real drift risk for the pier/water symptom.

