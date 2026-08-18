# Low Bridge TubeClass Parity Design

## Goal

Make low and wood bridge traversal in Rust TubeClass-backed, matching Yuri's Revenge player-visible behavior, without folding low bridges into ordinary road or high-bridge deck movement.

## Accepted Scope

Implement automatic low/wood bridge tubes created from map terrain/overlay facts. Shape the data model so explicit map `[Tubes]` can be added later, but do not include `[Tubes]` parsing in this pass.

This scope is chosen because the current parity bug is low bridge behavior. Explicit map tubes are a related feature, but they are not required to stop low bridges from behaving like plain road.

## Architecture Context

The current bridge implementation is split across these systems:

- `src/map/resolved_terrain.rs` classifies terrain and overlays into `ResolvedTerrainCell` facts used by sim and rendering.
- `src/map/overlay_types.rs` identifies high and low bridge overlay families.
- `src/sim/bridge_state/mod.rs` owns mutable bridge state, bridge groups, damage/repair state, and `BridgeEndpointRecord` entries.
- `src/sim/pathfinding/core.rs` converts resolved terrain plus bridge state into `PathGrid` / `PathCell`.
- `src/sim/pathfinding/zone_build.rs`, `zone_map.rs`, and `zone_incremental.rs` build zone maps and inject bridge adjacency.
- `src/sim/world/world_hash.rs` hashes mutable deterministic sim state, including bridge endpoint records.

The important existing boundary is that `map/` owns immutable terrain facts and `sim/` owns deterministic runtime state. Rendering can consume bridge facts, but `sim/` must not depend on render/UI/audio/net.

Current Rust treats low bridge overlays as road-like terrain in `resolved_terrain.rs`: low bridges force `is_water = false`, `is_road = true`, `ground_blocked = false`, `terrain_class = Road`, `land_type = Road`, and zone class `GROUND`. That gives simple movement a passable cell, but it erases the TubeClass behavior verified in `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`.

## Impact Analysis

Touched modules:

- `src/map/resolved_terrain.rs`: stop the low-bridge road override and expose low-bridge tube facts.
- `src/map/overlay_types.rs`: continue using existing low bridge overlay identity; no broad change expected.
- New likely module `src/map/tube_facts.rs`: immutable TubeClass-shaped facts created during map resolution.
- `src/sim/bridge_state/mod.rs`: build low bridge endpoint records from tube facts instead of generic low bridge overlay groups.
- `src/sim/pathfinding/core.rs`: carry enough tube metadata into path cells or path stepping to support tube traversal.
- `src/sim/pathfinding/zone_build.rs`: use tube-backed low bridge records for low bridge adjacency, while preserving high-only redirect behavior.
- `src/sim/pathfinding/zone_map.rs` and `zone_incremental.rs`: preserve the existing `AllActive` vs `HighActiveOnly` split.
- `src/sim/movement/*`: add/route the TubeMovement equivalent for units, and later infantry if infantry movement is active in this engine path.
- `src/sim/world/world_hash.rs`: hash only mutable tube/bridge runtime state. Immutable map tube facts should not become mutable sim state unless copied into runtime state.

Primary risks:

- Accidentally breaking recent high bridge `bridge_kind` fixes by treating low records like high records.
- Losing render data by removing the low bridge road override too broadly. `BridgeLayer::Low` visual facts must remain.
- Encoding binary `LandType == 10` into the existing Rust `LandType` enum, which only represents movement passability columns `0..=7`, or accidentally reusing the existing TMP `raw_land_type` byte where the binary uses final `CellClass+0xEC`.
- Creating one tube per bridge instead of one automatic tube per qualifying cell.
- Leaving unit movement as ordinary ground movement after zones become tube-aware, which would still be player-visible drift.

## Chosen Approach

Use a split static/runtime model:

1. Immutable tube facts live with resolved map data.
2. Per-cell tube index lives on `ResolvedTerrainCell`.
3. Mutable low bridge active/damaged/connectivity state stays in `BridgeRuntimeState`.
4. Zones/pathing consume tube facts through sim-owned bridge state and path grid metadata.
5. Unit movement consumes tube traversal state through a TubeMovement-equivalent movement path.

This follows the existing map/sim boundary and keeps high bridge state unchanged. Low bridges stop being plain road because passability and zone connectivity come from tube-aware records, not from `is_road`.

## Tiny-Detail Ledger

Implementation must preserve these verified details:

- `IsLowBridgeCell` is true only when the cell has a valid tube index and binary cell `LandType == 10`. Source: `CellClass::IsLowBridgeCell @ 0x00484AB0`, `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`.
- `GetTubeAtCell` bounds-checks the cell tube index and returns `g_TubeArray[index]`; it does not also check land type. Source: `CellClass::GetTubeAtCell @ 0x00484F20`.
- Automatic low bridge tube creation happens during `CellClass::RecalcAttributes`, after final land type computation, only when land type is `10`, the current tube index is invalid, and the tile is in one of the four checked ranges. Source: `CellClass::RecalcAttributes @ 0x0047D2B0`.
- The automatic direction table is `[2, 4, 6, 0]`. Source: data at `0x0081CC20`.
- `TubeClass::Constructor` writes entry coord, exit coord, direction, path length `0`, fills path steps with `-1`, appends the tube, and writes the new tube index to the entry cell. Source: `TubeClass::Constructor @ 0x00727FD0`.
- Automatic low bridge tubes are per qualifying cell, not per bridge span. Source: report section "One tube per bridge, segment, or cell?"
- Direction `8` is a tube jump in map-coordinate stepping and path-walking. Missing tube produces coord `0`. Source: `MapCoord_Step_By_Direction @ 0x0042D490`, `Path_walk_directions_to_cell @ 0x00429780`.
- `ComputeBridgeZones` builds low bridge records from low bridge tube cells and writes `bridge_kind = 1`. Source: `MapClass::ComputeBridgeZones @ 0x0056D6E0`.
- `FindBridgeRecord` is high-only because it skips records where `bridge_kind != 0`. Source: `MapClass::FindBridgeRecord @ 0x0056DA10`.
- The lower-level zone helper has a non-high-bridge tube path that uses tube direction, side cells, and path-walking. Source: `FUN_00582D70`.
- Unit tube movement runs while the unit active tube byte is non-negative, consumes tube entry, exit, direction, path steps, path length, and clears the active tube byte to `0xFF` at completion. Source: `UnitClass::AI @ 0x007363B0`, `UnitClass::TubeMovement @ 0x007359F0`.
- Low bridge damage/repair updates overlay/tile state and recomputes zones. The checked primary low bridge functions did not directly delete tube records or clear cell tube indices. Source: report section "Damage, Destruction, and Repair"; confidence medium.
- Current Rust `LandType` is a compressed passability-column enum and cannot represent binary `LandType == 10`. The existing `TileMetadata.raw_land_type` is the TMP `terrain_type` byte, where byte `5` maps to Tunnel and byte `10` maps to Beach, so it also must not be used as the binary `CellClass+0xEC` value. Source: `src/sim/pathfinding/passability.rs`, `src/rules/terrain_rules.rs`.

## Design

### Components

#### `map::tube_facts`

Add a small immutable map-level model:

- `TubeId`: compact deterministic index, matching TubeClass array index semantics.
- `TubeFact`: entry coord, exit coord, direction, path steps, path length, source.
- `TubeSource`: initially `AutoLowBridge`; later can add `ExplicitMapTube`.
- `CellTubeIndex`: optional tube id stored per resolved terrain cell.

Automatic low bridge tubes should be emitted in the same order map cells are resolved, using deterministic map coordinate order. That preserves stable tube ids and makes save/hash behavior reproducible.

The initial auto low bridge `TubeFact` values:

- `entry = cell coord`
- `exit = cell coord`
- `direction = [2, 4, 6, 0][tile_sub_index]`
- `path_len = 0`
- `path_steps = []` or fixed storage semantically equivalent to all `-1`
- `source = AutoLowBridge`

The implementation should not allocate one tube per bridge group. It should allocate one tube per qualifying cell.

#### `ResolvedTerrainCell`

Add fields or accessors for:

- `tube_index: Option<TubeId>`
- `yr_cell_land_type` or `binary_cell_land_type` sufficient to test final `CellClass+0xEC == 10`
- `is_low_bridge_tube_cell()` equivalent to binary `IsLowBridgeCell`

Do not overload the existing compressed `land_type` passability column with value `10`.
Do not reuse the existing `raw_land_type` name for this field: in current Rust
that name means TMP `terrain_type` byte, not the final binary cell land type.
The new field should be named to make that distinction impossible to miss.

Keep existing visual bridge facts:

- `bridge_layer: Some(BridgeLayer { direction: Low, ... })`
- low bridge overlay identity
- low bridge damage/variant facts already used by bridge rendering

But remove the behavior where low bridge overlays force ordinary road passability.

#### `BridgeRuntimeState`

Keep mutable bridge state here. Add tube-aware low bridge record building without disrupting high bridge records.

Low bridge records should be considered only from cells where `ResolvedTerrainCell::is_low_bridge_tube_cell()` is true, using the cell tube index and the corresponding `TubeFact`. That predicate is necessary but not sufficient for record insertion.

The low-record builder must mirror the verified `ComputeBridgeZones` low branch:

- iterate cells in deterministic map order;
- require the current cell to satisfy `IsLowBridgeCell`;
- require an opposite low-bridge neighbor pair: direction `2` and direction `6`, or direction `4` and direction `0`;
- read the current cell's tube with `GetTubeAtCell`;
- use the current cell coord and tube exit coord as the candidate record endpoints;
- insert the record as `BridgeRecordKind::Low` only when the binary ordering/duplicate filter accepts it.

The exact `FUN_0042B1C0` comparison inputs can remain a Phase 0 verification item if implementation wants byte-for-byte record ordering. The design must not treat every valid low tube cell as an inserted bridge record.

High bridge endpoint discovery should remain high-bridge-oriented. Do not reuse the generic "bridge group max-distance endpoints" logic for low bridge tubes.

Runtime damage/repair state should gate whether low bridge tube connectivity contributes to active zones/pathing. It should not delete immutable `TubeFact`s unless future binary evidence proves tube deletion occurs.

#### Pathfinding And Zones

`PathGrid` / `PathCell` need enough tube metadata for path stepping and zone building:

- optional tube index
- whether the cell satisfies low bridge tube predicate
- tube direction or lookup into the map tube registry

The existing zone split is correct and should be preserved:

- `inject_bridge_adjacency(... BridgeRecordFilter::AllActive)` may consume low and high records.
- `build_bridge_redirect(... BridgeRecordFilter::HighActiveOnly)` remains high-only.

The low bridge adjacency builder must use tube facts and the verified direction/path behavior, not generic road adjacency.

#### Movement

Unit movement must gain a TubeMovement-equivalent path before the feature is considered parity complete.

The Rust movement state should represent:

- active tube id, equivalent to the signed byte at unit `+0x684`
- current tube path step, equivalent to `+0x685`
- terminal state where active tube is cleared

Because the report leaves the exact producer of active tube assignment as an open question, the implementation plan should begin this phase with a narrow verification pass: trace where the path planner or movement code writes the active tube index for low bridge entry. This is a targeted RE check, not a new broad investigation.

#### Damage And Repair

Damage and repair should:

- update existing low bridge overlay/tile state
- invalidate/recompute affected zones
- gate active low bridge tube connectivity through runtime bridge state
- preserve static tube ids and cell tube indices unless a targeted write audit proves the binary clears them

This matches the current research confidence: the visible invalidation path is through overlay/state/zones, not tube object deletion.

### Data Flow

1. Map/terrain load resolves base terrain and overlays.
2. Low bridge overlay/tile classification identifies candidate low bridge tube cells.
3. Resolved terrain exposes binary low bridge land type separately from compressed movement passability.
4. For each qualifying cell, map resolution creates one `TubeFact` and stores the tube id on the cell.
5. `BridgeRuntimeState` builds high bridge records using existing high bridge logic and low bridge records using the tube-backed low branch: current low tube cell, opposite low-neighbor pair, current tube lookup, and ordering/duplicate filter.
6. Zone maps inject all active bridge records for adjacency but keep high-only redirect behavior.
7. Path walking handles direction `8` as a tube jump through the cell tube index.
8. Unit movement enters and consumes active tube movement state instead of treating the low bridge as normal road.
9. Damage/repair changes bridge activity and triggers zone recomputation without deleting static tube facts.

### Error Handling

Map-time tube construction should be deterministic and non-panicking:

- If a candidate low bridge tile cannot map to one of the four verified direction table entries, do not synthesize a guessed tube. Record no tube and make the failure visible in tests/log diagnostics.
- If a cell references an out-of-range tube id, treat it as invalid for `IsLowBridgeCell` and path stepping, matching the binary's bounds-check behavior.
- Direction `8` with no valid tube should produce the engine's equivalent of invalid coord `0` in low-level path-walking helpers, but high-level pathfinding should avoid selecting such paths.

### Testing Strategy

Map/terrain tests:

- Low bridge overlays no longer force `Road` passability.
- Candidate low bridge cells produce one tube per qualifying cell.
- Tube ids are deterministic across repeated map loads.
- Auto tube direction uses `[2, 4, 6, 0]`.
- Existing high bridge resolved facts and rendering bridge layer facts remain unchanged.

Zone/path tests:

- Low bridge records are `BridgeRecordKind::Low`.
- High bridge records remain `BridgeRecordKind::High`.
- A lone valid low tube cell does not create a low bridge record without the verified opposite-neighbor low-cell pattern.
- A low tube cell with the verified `2/6` or `4/0` opposite-neighbor pattern creates at most the ordered low record accepted by the duplicate filter.
- High-only redirect ignores low bridge records.
- All-active adjacency includes low bridge records when the bridge is active.
- Direction `8` path walking uses the current cell tube index and tube exit.

Movement tests:

- A unit entering a low bridge tube records active tube state.
- Tube movement consumes path steps and clears active tube at exit.
- Auto low bridge tubes with same-cell entry/exit do not collapse into ordinary road movement.

Damage/repair tests:

- Damaging or destroying a low bridge changes active zone connectivity.
- Repair restores active low bridge connectivity.
- Static tube ids and cell tube indices remain stable across damage/repair unless future RE proves otherwise.

Regression tests:

- Existing high bridge pathing, bridgehead, and `bridge_kind` tests remain green.
- World hash changes when mutable low bridge active/connectivity state changes.
- World hash does not depend on nondeterministic tube allocation order.

## Implementation Boundary

Phase 0: Targeted pre-code verification

- Verify the exact producer that writes active tube id/path step for unit low bridge entry.
- Do a focused write audit for `CellClass+0x116` around low bridge damage/repair if implementation would otherwise clear tube indices.

Phase 1: Static map tube facts

- Add immutable TubeClass-shaped facts in `map/`.
- Add per-cell tube index and raw/binary land type predicate.
- Stop low bridges from being forced into ordinary road passability.

Phase 2: Bridge records and zones

- Build low bridge endpoint records from tube-backed cells using the verified `ComputeBridgeZones` low-branch eligibility checks.
- Preserve high bridge endpoint behavior.
- Wire tube-backed low records into existing zone adjacency filters.

Phase 3: Path walking

- Add direction `8` tube jump semantics to the Rust path-walking equivalent.
- Keep invalid/missing tube behavior explicit and tested.

Phase 4: Movement

- Add unit TubeMovement-equivalent state and stepping.
- Add infantry equivalent only if infantry movement currently reaches the same bridge traversal surface in Rust; otherwise document it as blocked on infantry movement implementation.

Phase 5: Damage and repair

- Gate low bridge tube connectivity through runtime state.
- Recompute/invalidate zones on low bridge damage and repair.
- Preserve static tube registry unless binary evidence requires mutation.

## Architectural Decisions

- Tube registry belongs to map resolved facts because auto tubes are created from static cell attributes during map load.
- Per-cell tube index belongs on `ResolvedTerrainCell` because the binary stores it on `CellClass`, and many consumers ask "what tube is at this cell?"
- Mutable active/damaged state belongs in `BridgeRuntimeState`, not in `map/`.
- Existing compressed `LandType` must not be extended with `10`; add a separate raw/binary land type field or low bridge tube predicate.
- Low bridge visuals stay in bridge overlay/layer facts; low bridge movement stops using the road override.
- Existing high bridge bridge-kind work remains the boundary for high bridge redirects.

## Alternatives Considered

### Alternative A: Put the entire tube registry in `BridgeRuntimeState`

This keeps all bridge-related data in one sim module, but it mixes immutable map-load facts with mutable damage state. It also makes `ResolvedTerrainCell` unable to answer the equivalent of `IsLowBridgeCell` without reaching into sim. Rejected.

### Alternative B: Add tube-like edges directly to `PathGrid` only

This is the smallest pathing patch, but it does not model per-cell tube index, direction `8`, `GetTubeAtCell`, cursor/action routing, or unit TubeMovement. It would likely keep player-visible drift. Rejected.

### Alternative C: Implement full `[Tubes]` parsing now

This would share more infrastructure with the original TubeClass, but it widens the implementation into explicit map tunnel behavior before the low bridge bug is fixed. Deferred. The chosen `TubeFact` shape keeps this path open.

## Open Questions

- Exact producer of active unit/infantry tube state still needs a narrow RE check before movement implementation.
- Exact duplicate/order filter inputs in `ComputeBridgeZones` low record insertion may need final assembly verification if the Rust low record order must match byte-for-byte behavior. The existence of the filter is verified; the implementation must not insert one low record per tube cell.
- Full low bridge surface/ramp visual mutation remains owned by existing bridge damage/rendering work, not this TubeClass plan.

## Handoff

Use this design as the input to `/write-plan` or direct implementation planning. Do not start by editing movement code first; the static map tube facts and road-passability removal are the foundation that all later phases depend on.
