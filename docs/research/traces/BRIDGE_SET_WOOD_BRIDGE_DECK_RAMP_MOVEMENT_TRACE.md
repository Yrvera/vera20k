# BridgeSet / WoodBridgeSet Deck-Ramp Movement Trace

Date: 2026-05-26

Scenario: On a concrete bridge crossing with `BridgeSet` or `WoodBridgeSet` terrain underneath, order a Grizzly Tank (`[MTNK]`) across the bridge deck/ramp. Trace whether active YR `gamemd.exe` and Rust both let the unit traverse the bridge surface while still rejecting off-deck entry into underlying impassable cliff/water cells.

Scope is intentionally one mechanic and one player-visible scenario. Adjacent bridge collapse, repair huts, low-bridge tubes, render bridge-fudge, wall crushing, and waterfall/cliff-ramp traces are out of scope.

## Verdict

PASS: 0 | FAIL: 0 | UNCHECKED: 8 | NOT-IMPLEMENTED: 1

No PASS is claimed. The available research proves active YR mechanisms for bridge cell flags, bridge ramp/ramp-key predicates, Grizzly `Can_Enter_Cell`, and the broad `BridgeSet`/`WoodBridgeSet` impassable classifier, but this run did not compute a concrete stock-map or fixture path in both `gamemd.exe` and Rust and compare the resulting cells/layers numerically.

No FAIL is claimed for the exact scenario because a literal gamemd-vs-Rust path equality run was not executed. The main player-visible risk remains UNCHECKED: Rust has scaffolding that should allow bridge-deck movement over blocked underlying terrain, but current bridge facts can still be broader or shaped differently than `gamemd.exe`.

## Pipeline

1. Player command: player orders `[MTNK]` across a concrete/wood bridge deck/ramp.
2. Data: `[MTNK]` is a vehicle with `MovementZone=Normal`; `TooBigToFitUnderBridge=true` is parsed but not a movement gate in active YR.
3. Theater classification: `BridgeSet` / `WoodBridgeSet` tile ids are broadly classified as impassable by `IsCliffOrImpassableTile @ 0x004863d0`.
4. Bridge facts: active YR bridge deck/ramp movement comes from stamped bridge cell flags, bridge state bytes, exact bridge-ramp tile/subtile predicates, and dynamic `Can_Enter_Cell` / `CheckBridgeTraversal`, not from broad `BridgeSet` membership alone.
5. Rust resolved terrain: `ResolvedTerrainCell` carries `has_bridge_deck`, `bridge_walkable`, `bridge_transition`, `is_cliff_like`, `ground_walk_blocked`, and bridge fact fields.
6. Rust terrain cost: `TerrainCostGrid::from_resolved_terrain` returns cost `100` for `has_bridge_deck && !overlay_blocks`, before hard-blocking `is_cliff_like` / blocked terrain.
7. Rust path grid / A*: `PathGrid::from_resolved_terrain_with_bridges` exposes bridge-layer walkability and A* can search with `MovementLayer::Bridge`.
8. Runtime movement: Rust bridge movement uses bridge-layer path steps and bridge Z transition helpers; YR runtime uses current effective height plus `UnitClass::Can_Enter_Cell`.
9. Screen result: the player should see the Grizzly cross on the bridge surface, not drive into off-deck cliff/water cells.

## Stage Trace

### Stage 1 - Grizzly movement identity

Rust input: pathing target is a ground vehicle equivalent to `[MTNK]`.

Rust value: no Rust code in the scanned movement/pathfinding surfaces gates bridge movement on `too_big_to_fit_under_bridge`.

YR value: `[MTNK]` has `MovementZone=Normal` and `TooBigToFitUnderBridge=true`; active YR movement/pathfinding uses `UnitClass::Can_Enter_Cell @ 0x0073F0A0` and `CheckBridgeTraversal @ 0x004D9C60`, and verified research found no movement read of `UnitTypeClass+0xE16`.

Verdict: UNCHECKED. The mechanism is well-supported by active YR research, but no concrete command-path output was computed in this trace.

### Stage 2 - Broad BridgeSet / WoodBridgeSet impassable classification

Rust value: `TheaterCliffRanges::is_cliff_or_impassable_tile(tile_id, slope_byte)` returns true when `tile_id` is in `[BridgeSet, BridgeSet + 0x10)` or `[WoodBridgeSet, WoodBridgeSet + 0x10)`.

YR value: `IsCliffOrImpassableTile @ 0x004863d0` uses the same broad half-open ranges for `BridgeSet` and `WoodBridgeSet`.

Concrete scenario implication: an off-deck cell whose final tile id falls in those ranges should remain blocked as underlying terrain unless bridge facts/layers make the surface traversable.

Verdict: UNCHECKED. The range mechanism matches the verified report, but no exact scenario tile id and slope byte were sampled from a concrete bridge crossing in both engines.

### Stage 3 - Bridge ramp predicate is narrower than broad bridge-set membership

Rust value: `TheaterData` stores bridge top/middle keys and `BridgeRampTileTable` models bridge-ramp keys separately from the broad cliff/impassable range. `ResolvedTerrainCell::is_wood_bridge_repair_tile` is also separate from movement traversal.

YR value: `MapClass::IsBridgeRampTile @ 0x005746c0` checks bridge-specific keys and exact `cell+0x11A` values; it is not simply "tile is in BridgeSet/WoodBridgeSet".

Concrete scenario implication: deck/ramp movement must come from bridge facts plus the narrow bridge-ramp/transition semantics, while off-deck `BridgeSet` or `WoodBridgeSet` terrain remains rejected.

Verdict: UNCHECKED. The binary mechanism is verified, but this trace did not compute the exact bridge-ramp key/subtile for a selected concrete crossing.

### Stage 4 - Rust terrain-cost bridge-deck override

Rust computation for a bridge deck over impassable terrain:

```text
 has_bridge_deck = true
 overlay_blocks = false
 is_cliff_like = true or ground_walk_blocked = true underneath
 terrain cost = 100 (COST_NORMAL)
```

Code path: `src/sim/pathfinding/terrain_cost.rs` tests `cell.has_bridge_deck && !cell.overlay_blocks` before `hard_blocked`.

YR value: active YR does not use a single terrain-cost override. `UnitClass::Can_Enter_Cell` uses bridge flags, path height, layer occupancy, locomotor passability, and `CheckBridgeTraversal`; deck traversal is legal when bridge structural/transition/height conditions match.

Verdict: UNCHECKED. Rust produces the expected local value (`100`) for the modeled deck case, but no YR edge cost / return code for the same concrete cell was computed.

### Stage 5 - Rust off-deck rejection

Rust computation for an off-deck impassable `BridgeSet` / `WoodBridgeSet` terrain cell:

```text
 has_bridge_deck = false
 canonical_ramp = None
 is_cliff_like = true or ground_walk_blocked = true
 terrain cost = 0 (COST_BLOCKED)
```

Code path: `TerrainCostGrid::from_resolved_terrain` hard-blocks `(cell.is_cliff_like && !ramp_passable) || overlay_blocks || terrain_object_blocks`.

YR value: broad `IsCliffOrImpassableTile` marks `BridgeSet` / `WoodBridgeSet` as impassable, but full off-deck movement rejection is consumed downstream through cell land/slope/height/bridge flags and `Can_Enter_Cell`.

Verdict: UNCHECKED. The Rust local value is clear, but this run did not compute the equivalent YR `Can_Enter_Cell` result for an off-deck adjacent cell.

### Stage 6 - Rust bridge-layer A* traversal

Rust value: `PathGrid::from_resolved_terrain_with_bridges` preserves `bridge_walkable` when a bridge is intact and makes `PathGrid::is_walkable_on_layer(x, y, MovementLayer::Bridge)` true for bridge-walkable cells. `PathCell::can_enter_bridge_layer_from_ground()` requires `bridge_walkable && transition`.

YR value: A* calls the unit vtable `+0x1AC` (`UnitClass::Can_Enter_Cell`) and bridge traversal sub-check `+0x1B0` (`CheckBridgeTraversal`). A* carries path height, and bridge legality depends on `0x100`, `0x200`, signed levels, slope index, and parent/candidate edge shape.

Verdict: UNCHECKED. No concrete path cell/layer sequence from Rust was compared against active YR for the same bridge.

### Stage 7 - Runtime movement over the bridge deck

Rust value: movement has bridge-specific path step and Z transition code that uses bridge layer/deck level fields.

YR value: runtime drive movement calls `Can_Enter_Cell(target_cell, direction, current_effective_height, 0, 1)`, where `current_effective_height = current_cell.level + (OnBridge ? 4 : 0)`. `CheckBridgeTraversal` derives the parent cell when runtime passes zero.

Verdict: UNCHECKED. No tick-by-tick Grizzly runtime movement comparison was computed.

### Stage 8 - Map-load bridge facts feeding the scenario

Rust value: current bridge facts still include compatibility booleans and inferred/deck views in `resolved_terrain.rs`; prior bridge research says older broad inference, side expansion, height normalization, and gap-fill are not equivalent to active YR map-load bridge stamping.

YR value: high-bridge facts are stamped by `OverlayClass::Mark` through `CellClass::SetBridgeDirection_*`, with specific bridge bits (`0x80`, `0x100`, `0x200`, etc.) and `[OverlayDataPack]` state bytes. `CellClass::RecalcAttributes` does not globally derive bridge structure.

Verdict: NOT-IMPLEMENTED for a concrete equality harness / map-cell dump for this scenario. The trace cannot prove the selected Rust bridge cells are the same cells/layers/levels YR would stamp without such a dump.

## Player-Visible Findings

No computed FAIL for the concrete Grizzly crossing was proven in this trace.

Most visible unresolved risk: if Rust marks a bridge deck/transition cell differently from YR, the Grizzly may path onto a bridge from the wrong side, refuse a valid bridge ramp, or drive into/off the bridge layer incorrectly. This would be common and visible on bridge maps, but remains UNCHECKED for this exact scenario until a concrete crossing is numerically compared.

Second visible unresolved risk: off-deck `BridgeSet` / `WoodBridgeSet` cells can be over-allowed if a broad Rust bridge-deck view is applied where YR has only impassable underlying terrain. This is not demonstrated as happening in the current files for the exact scenario, but the previous bridge reports warn that broad bridge inference is not a binary-equivalent source of truth.

## Adjacent Findings

- Low bridges are tube/land-type driven in active YR and should not be inferred from overlay ID alone. This is adjacent because the current scenario is concrete/wood high bridge deck/ramp movement, not low-bridge tube travel.
- `TooBigToFitUnderBridge=true` is live for `[MTNK]` but affects render consumers, not Grizzly movement legality in the verified YR slice.
- Return-code naming drift in `cell_entry.rs` for codes 3 and 6 is adjacent to bridge movement but not traced here.

## Evidence

- `docs/research/THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md`
  - `IsCliffOrImpassableTile @ 0x004863d0` includes `BridgeSet` and `WoodBridgeSet` ranges `[base, base+0x10)`.
  - `MapClass::IsBridgeRampTile @ 0x005746c0` is bridge-key/subtile specific and not broad set membership.
- `docs/research/bridges/01-assets-map-load-overlay/BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md`
  - Active YR bridge traversal uses bridge flags, level, path height, and dynamic `Can_Enter_Cell` / `CheckBridgeTraversal`.
  - Current Rust broad resolved-terrain bridge inference is not equivalent to map-load stamping.
- `docs/research/bridges/01-assets-map-load-overlay/BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`
  - Active map-load high bridge stamping comes from overlay IDs `0x18`, `0x19`, `0xED`, `0xEE` and writes specific per-cell bridge flags and state bytes.
- `docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`
  - `UnitClass::Can_Enter_Cell @ 0x0073F0A0` is live for vehicle pathing, with `CheckBridgeTraversal @ 0x004D9C60`.
- `docs/research/bridges/04-locomotion-height-tubes/GRIZZLY_TOOBIG_UNDER_BRIDGE_CONSUMER_GHIDRA_REPORT.md`
  - `[MTNK] TooBigToFitUnderBridge=true` is not a movement/pathfinding gate in the verified active YR slice.
- Rust scanned:
  - `src/map/theater.rs`
  - `src/map/resolved_terrain.rs`
  - `src/sim/pathfinding/terrain_cost.rs`
  - `src/sim/pathfinding/core.rs`
  - `src/sim/pathfinding/zone_build.rs`
  - `src/sim/movement/movement_bridge.rs`

## Required Follow-Up To Turn UNCHECKED Into PASS/FAIL

1. Select one concrete stock or minimal fixture bridge crossing and record the exact cell coordinates, final tile ids, subtiles, bridge flags, levels, bridge state bytes, overlay ids, and Grizzly start/goal.
2. Compute active YR per-step `Can_Enter_Cell` return codes, carried path heights, selected object-list layers, selected occupancy-bit layers, and final path cells/layers.
3. Compute Rust resolved terrain, terrain cost, `PathGrid`, A* path cells/layers, and runtime bridge Z transitions for the same cells.
4. Compare literal numbers. Only then mark deck traversal and off-deck rejection PASS or FAIL.
