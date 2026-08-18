# Bridge Crossing Path Oracle Requirements Trace

Date: 2026-05-26

Scenario: follow-up to the theater cliff/ramp trace swarm. The concrete player
action is a Grizzly Tank ordered across one high bridge deck/ramp whose
underlying theater tiles may fall in `BridgeSet` or `WoodBridgeSet`.

Goal: determine what exact evidence is required to turn the prior bridge slot
from UNCHECKED into PASS or FAIL. This trace is research-only. It does not edit
Rust, INI, or published docs outside this trace report.

## Verdict

PASS: 0 | FAIL: 0 | UNCHECKED: 7 | NOT-IMPLEMENTED: 2

No end-to-end bridge movement PASS can be claimed yet. The active gamemd
mechanism is known well enough to define the oracle contract, but no current
local artifact provides a concrete same-cell dump of:

- gamemd map-load bridge-stamped cell facts,
- gamemd `UnitClass::Can_Enter_Cell` return codes per candidate step,
- gamemd carried A* path height and bridge/ground closed-list selection,
- Rust resolved terrain, path grid, terrain cost, A* path cells/layers, and
  runtime bridge-layer movement for the same start/goal.

The actionable result is a precise harness requirement. Until that exists, a
bridge crossing affected by `BridgeSet`/`WoodBridgeSet` broad impassability is
UNCHECKED, not successful parity.

## Pipeline

Player move order
-> `[MTNK]` movement identity
-> map-load bridge overlay stamping
-> theater numeric `BridgeSet`/`WoodBridgeSet` impassable classification
-> `CellClass` bridge flags/state bytes and height/slope bytes
-> A* candidate layer/height selection
-> `UnitClass::Can_Enter_Cell` with `CheckBridgeTraversal`
-> Rust `ResolvedTerrainGrid` / `PathGrid` / `TerrainCostGrid`
-> Rust A* path cells and layers
-> runtime drive movement and bridge-layer occupancy
-> screen result: tank crosses bridge deck and cannot enter off-deck blocked cells

## Stage Results

### Stage 1 - Unit Identity

Input: Grizzly Tank `[MTNK]`, normal ground vehicle.

gamemd evidence: bridge movement legality for vehicles uses
`UnitClass::Can_Enter_Cell @ 0x0073F0A0`; `TooBigToFitUnderBridge=true` is not a
movement gate in the verified bridge research slice.

Rust evidence: Rust routes vehicle pathing through movement/pathfinding layers;
bridge movement has path layer and bridge-specific movement code.

Verdict: UNCHECKED. This trace did not compute a full live order result for a
specific Grizzly instance.

### Stage 2 - BridgeSet / WoodBridgeSet Broad Impassability

gamemd evidence: `IsCliffOrImpassableTile @ 0x004863d0` includes
`BridgeSet` and `WoodBridgeSet` half-open ranges `[base, base+0x10)`.

Rust evidence: `TheaterCliffRanges::is_cliff_or_impassable_tile` includes the
same `bridge_set` and `wood_bridge_set` range checks in `src/map/theater.rs`.

Verdict: UNCHECKED for the concrete bridge crossing. The range mechanism is
verified, but this trace did not select one stock bridge cell and compute the
same final tile id/subtile/slope byte in both engines.

### Stage 3 - Map-Load Bridge Facts

gamemd evidence: bridge facts are not derived from broad set membership alone.
Research verifies map-load `OverlayClass::Mark` calls
`SetBridgeDirection_NESW` / `SetBridgeDirection_NWSE`, which writes bridge flags
and state bytes. Relevant bridge flags include `0x80` anchor marker, `0x100`
structural bridge, `0x200` bridgehead, and `0x400` destroyed marker.

Rust evidence: `ResolvedTerrainCell` carries `has_bridge_deck`,
`bridge_walkable`, `bridge_transition`, bridge facts, bridge layer data, and
derived deck level.

Verdict: NOT-IMPLEMENTED for a concrete parity oracle. There is no report or
harness output here that lists the exact gamemd-stamped bridge flags/state bytes
and exact Rust bridge facts for the same selected bridge crossing.

### Stage 4 - A* Path Height And Candidate Layer

gamemd evidence: normal A* passes a concrete current path height into
`Can_Enter_Cell`, not generally `-1`. The candidate closed-list split uses the
current path height against the candidate cell level and bridge flag `0x100`.

Rust evidence: `src/sim/pathfinding/core.rs` has `BRIDGE_HEIGHT_THRESHOLD = 2`,
`is_at_bridge_level`, `compute_neighbor_height`, and bridge traversal predicates.
Rust tests cover synthetic bridgehead/body cases, but they are not tied to a
gamemd cell dump.

Verdict: UNCHECKED. Rust has modeled pieces, but no concrete gamemd path-height
sequence was compared for the same start, bridgehead, body, and destination.

### Stage 5 - UnitClass::Can_Enter_Cell / CheckBridgeTraversal

gamemd evidence: `CheckBridgeTraversal @ 0x004D9C60` gates diff-0, diff-1, and
diff-4 bridge/height movement. It uses signed `Level`, `SlopeIndex`, structural
bridge flag `0x100`, bridgehead flag `0x200`, and can force the bridge object
list byte when entering a bridgehead from below. `UnitClass::Can_Enter_Cell`
has a pre-vtable object-list selection and post-vtable occupancy-bit overwrite;
those two decisions can disagree in edge cases.

Rust evidence: `src/sim/pathfinding/cell_entry.rs` has explicit
`CanEnterLayerContext` with terrain, object-list, and occupancy-bit layers. Its
module comment still frames bridge legality as an approximation of the original
two-pass mechanism.

Verdict: UNCHECKED. No concrete per-step return codes 0-7 were computed from
gamemd and Rust for the same bridge candidate cells.

### Stage 6 - Rust Resolved Terrain / Cost / PathGrid

Rust evidence: `PathGrid::from_resolved_terrain_with_bridges` marks intact
structural bridge cells walkable, preserves bridge walkability/transition, and
lets intact bridge deck override underlying terrain blocking. `TerrainCostGrid`
also returns normal cost for `has_bridge_deck && !overlay_blocks` before
hard-blocking cliff-like underlying terrain.

gamemd evidence: active YR deck traversal is not a terrain-cost override; it is
the combined result of bridge flags, path height, cell levels, object/occupancy
layers, and `Can_Enter_Cell`.

Verdict: UNCHECKED. Rust local behavior can be computed, but the equivalent
gamemd return values for the same selected cell sequence are absent.

### Stage 7 - Runtime Movement And Screen Result

Rust evidence: runtime drive movement checks the next path layer; entering from
ground into bridge requires `can_enter_bridge_layer_from_ground`, while a unit
already on bridge can continue over bridge-walkable cells. Movement code updates
bridge layer and occupancy.

gamemd evidence: runtime drive movement also calls `Can_Enter_Cell` with current
effective height and bridge-specific traversal checks.

Verdict: NOT-IMPLEMENTED for parity oracle capture. There is no same-tick
runtime comparison showing the Grizzly's layer, cell, path height, occupancy
list, and visible position across the bridge in both engines.

## Required Oracle Contract

A bridge crossing trace can become PASS/FAIL only after capturing one exact
scenario with these values:

1. Map/scenario identity: stock map or minimal fixture name, theater, bridge
   overlay ids, start cell, target cell, and selected bridge route cells.
2. Theater tile facts for every relevant cell: tile id, subtile, slope byte,
   level byte, land type, and whether the tile falls inside `BridgeSet` or
   `WoodBridgeSet`.
3. gamemd map-load bridge facts per cell: flags `0x80`, `0x100`, `0x200`,
   `0x400`, bridge state byte, bridge anchor pointer identity, structural
   bridge direction/family, and deck level.
4. Rust map-load bridge facts per same cell: `has_bridge_deck`,
   `bridge_walkable`, `bridge_transition`, `bridge_facts` raw flags, bridge
   layer/direction/family, `level`, `slope_type`, and `bridge_deck_level`.
5. gamemd A* per candidate step: current node cell, candidate cell, direction,
   current path height, candidate closed-list layer, `CheckBridgeTraversal`
   result, `UnitClass::Can_Enter_Cell` return code, selected object list,
   selected occupancy bits, edge cost, and resulting carried path height.
6. Rust A* per same step: current node cell/layer, candidate cell/layer,
   computed neighbor height, bridge traversal predicate result, walkability,
   terrain cost, entity/occupancy layer selection, edge cost, and final path.
7. Runtime movement comparison: tick number, current cell, next cell, active
   layer, on-bridge state, occupancy layer before/after, and visible cell/height
   at each step.

If any of those values are missing, mark the bridge slot UNCHECKED. If all are
present, compare literal numbers and mark PASS only where they match exactly.

## Findings

No computed FAIL was proven.

The blocker is evidentiary, not necessarily implementation failure: Rust has
bridge-layer pathing and bridge-deck terrain overrides, but the active gamemd
bridge movement mechanism has enough layer/height/state detail that local Rust
tests cannot prove parity without a same-cell gamemd oracle.

Highest player-visible risk: a tank may refuse a valid bridgehead, enter the
bridge layer from the wrong cell, path onto a bridge body from ground, or be
allowed into off-deck impassable terrain if Rust bridge facts differ from
gamemd stamping. This would be visible on bridge maps.

## Suggested Next Trace

Run one concrete bridge crossing trace after selecting a stock bridge cell
sequence and dumping the oracle values above. The trace should be named around
the selected map and route, for example:

`/trace-action Grizzly crosses high bridge on <map> from (<x1>,<y1>) to (<x2>,<y2>) with per-step Can_Enter_Cell oracle`

Do not run another broad swarm until this single oracle exists; otherwise the
bridge slot will keep returning UNCHECKED.

## Sources

- `docs/research/THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md`
- `docs/research/bridges/00-system-models/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`
- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md`
- `docs/research/bridges/04-locomotion-height-tubes/GRIZZLY_TOOBIG_UNDER_BRIDGE_CONSUMER_GHIDRA_REPORT.md`
- `src/map/theater.rs`
- `src/map/resolved_terrain.rs`
- `src/sim/pathfinding/core.rs`
- `src/sim/pathfinding/cell_entry.rs`
- `src/sim/pathfinding/terrain_cost.rs`
- `src/sim/movement/movement_step.rs`
