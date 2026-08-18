# Coordinate/Height Trace: Conscript crosses low bridge ramp

Date: 2026-05-21

Scope: one concrete scenario only: a Conscript starts on ground, receives a move
order across a low bridge ramp, transitions onto the bridge deck, continues across
bridge cells, and exits back to ground.

Verdict rule: `PASS` requires literal numerical equality between Rust output and
gamemd output. If both were not computed for the concrete scenario, the stage is
`UNCHECKED`.

## Concrete Fixture Used For Numerical Checks

The Rust-side bridge-deck transition checks use this explicit cell sequence:

- `C0 = (0,0)`, ground level `4`, no bridge.
- `C1 = (1,0)`, ramp/bridgehead level `4`, structural bridge flag set.
- `C2 = (2,0)`, bridge body ground level `0`, structural bridge flag set, deck
  level `4`.
- `C3 = (3,0)`, bridge body ground level `0`, structural bridge flag set, deck
  level `4`.
- `C4 = (4,0)`, ramp/bridgehead level `4`, structural bridge flag set.
- `C5 = (5,0)`, ground level `4`, no bridge.

This fixture is valid for checking the active bridge layer predicate. It is not a
retail map extraction, so exact player command path selection and exact per-tick
pixel output remain unchecked.

## Sources Checked

- `C:/Users/enok/Documents/ra2-rust-game-docs/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`
- Ghidra read-only decompilation of:
  - `WalkLocomotionClass__Head_To_Coord`
  - `WalkLocomotionClass__ProcessMovement`
  - `CellClass__AddContent`
  - `CellClass__RemoveContent`
  - `UnitClass__Can_Enter_Cell`
  - `AStar_main_loop`

All gamemd references cited below are active in standard YR paths, not dormant
Tiberian Sun legacy paths, unless explicitly labelled as adjacent.

## Pipeline

Move command target -> A* path height/layer search -> low bridge/tube or deck-cell
step selection -> locomotor boundary crossing -> object layer removal/addition ->
z and screen projection -> final visible unit position.

## Stage 1 - Conscript Locomotor Data

Rust:

- `ini/rulesmd.ini` `[E2]` is the YR-priority Conscript definition.
- `[E2] Speed=4`, `MovementZone=Infantry`, and
  `Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}`.

gamemd:

- `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` verifies the infantry
  walk locomotor path is active in standard YR.

Verdict: `PASS` - both sides select the walk/infantry movement family for the
Conscript.

## Stage 2 - Command Target And Chosen Path

Rust:

- Move commands eventually request a path through the sim pathfinder and store
  path cells/layers for the movement target.
- For this run, I did not drive an end-to-end UI command on a concrete retail map.

gamemd:

- `AStar_main_loop @ 0x00429A90` is active and carries path height state.
- I did not compute the exact retail selected path cells/layers for this concrete
  command.

Verdict: `UNCHECKED` - no literal cell-by-cell Rust vs gamemd command path was
computed.

## Stage 3 - Pathfinding Height Decisions

Rust:

- `src/sim/pathfinding/core.rs` carries bridge-aware path height and layer state.
- The local targeted tests
  `test_layered_astar_can_traverse_bridge_after_unrelated_rebuild` and related
  bridge transition tests passed.

gamemd:

- `AStar_main_loop @ 0x00429A90` uses separate ground/bridge closed/g-cost arrays
  and path height offsets.
- `BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md` verifies bridge
  transition handling: destination bridge cells can resolve to `Level + 4`, and
  four-level differences trigger bridge transition logic.

Verdict: `UNCHECKED` - the algorithms were traced, but exact numerical A* outputs
for the same map and click were not computed.

## Stage 4 - Bridge Layer Transition Predicate

Rust:

- `src/sim/movement/movement_bridge.rs` computes:
  - enter bridge layer when `dst_h == src_h - 4` and destination has a structural
    bridge flag.
  - exit bridge layer when destination lacks structural bridge and source has one.
- For the concrete sequence:
  - `C0 -> C1`: no bridge layer set.
  - `C1 -> C2`: set bridge layer, deck level `4`.
  - `C2 -> C3`: stay bridge layer, deck level `4`.
  - `C3 -> C4`: stay bridge layer, deck level `4`.
  - `C4 -> C5`: clear bridge layer.

gamemd:

- `WalkLocomotionClass__ProcessMovement @ 0x75AEC0` contains the same active YR
  predicate for `ObjectClass+0x8C`:
  - set when destination level equals source level minus `4` and destination has
    bridge flag `0x100`.
  - clear when destination lacks bridge flag `0x100` and source has it.

Verdict: `PASS` - boolean layer transitions for the five concrete boundary steps
match the active gamemd predicate exactly.

## Stage 5 - Occupancy Layer Selection

Rust:

- `src/sim/occupancy.rs` removes the entity by id from the old cell and adds it to
  the destination cell with the active movement layer.
- `src/sim/movement/movement_tick.rs` applies pending bridge render state before
  refreshing screen coordinates.
- Final layer result for the concrete sequence:
  - after `C1 -> C2`, the Conscript is in bridge occupancy on `C2`.
  - after `C2 -> C3`, the Conscript remains in bridge occupancy on `C3`.
  - after `C4 -> C5`, the Conscript is in ground occupancy on `C5`.

gamemd:

- `CellClass__RemoveContent @ 0x0047EA90` removes using the old `OnBridge` layer.
- `CellClass__AddContent @ 0x0047E8A0` adds using the updated `OnBridge` layer.
- `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md` verifies the active ordering:
  remove from old cell, update coordinates, update `OnBridge`, add to new cell.

Verdict: `PASS` - final ground/bridge occupant layer for each boundary crossing
matches the formula-derived gamemd result. Rust does not model the exact old-layer
parameter on remove, but final occupancy is numerically equal in this single-unit
fixture.

## Stage 6 - Per-Tick Z And Screen Projection

Rust:

- `src/util/lepton.rs` projects cell centers with:
  - `screen_x = 30 * (rx - ry) + subcell offset`
  - `screen_y = 15 * (rx + ry) + 15 - z * 15 + subcell offset`
- With center subcell and `z = 4`, Rust cell-center samples are:
  - `C0`: `(0, -45)`
  - `C1`: `(30, -30)`
  - `C2`: `(60, -15)`
  - `C3`: `(90, 0)`
  - `C4`: `(120, 15)`
  - `C5`: `(150, 30)`

gamemd:

- `CoordsToClient @ 0x006D1F10` maps cell centers as
  `screen_x = 30 * (rx - ry)`, `screen_y = 15 * (rx + ry) + 15`.
- `Tactical__AdjustForZ @ 0x006D20E0` then applies Z lift.
- I did not compute the exact gamemd per-tick lepton Z, subcell positions, and
  `AdjustForZ` pixels for the Conscript's walk animation cadence.

Verdict: `UNCHECKED` - Rust samples are computed, but exact gamemd per-tick
screen coordinates were not.

## Stage 7 - Low Bridge TubeClass Path

Rust:

- `src/sim/movement/tube_movement.rs` starts tube movement only for non-adjacent
  path steps and rejects `tube.path_len() == 0`.
- `move_entity_to_cell` updates cell position and screen coordinates but does not
  update `position.z`, `on_bridge`, or `bridge_occupancy`.

gamemd:

- `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` verifies active standard YR low-bridge
  tube logic:
  - low bridge cells require `LandType == 10` and a valid tube index.
  - direction `8` is the tube jump in path walking.
  - `InfantryClass::AI @ 0x0051BF00` calls the tube movement helper when the tube
    field is active.
  - `WalkLocomotionClass__ProcessMovement @ 0x75AEC0` contains active low bridge
    tube handling.

Verdict: `NOT-IMPLEMENTED` - if this scenario is the retail low-bridge
`LandType=10` TubeClass route rather than the structural bridge-deck predicate
fixture above, Rust does not yet reproduce the active gamemd low-bridge tube
movement and height/layer updates.

## Stage 8 - Final Player-Visible Position

Rust:

- For the structural deck fixture, final cell is `C5 = (5,0)`, `on_bridge=false`,
  ground occupancy, `z=4`, screen center `(150,30)`.

gamemd:

- The active layer predicate implies final `OnBridge=0` after leaving a structural
  bridge into a non-bridge destination cell.
- The exact final retail map coordinate and screen pixel for a clicked move order
  were not computed.

Verdict: `UNCHECKED` - final Rust fixture output is known, but exact gamemd output
for the same retail command was not computed.

## Adjacent Findings

- Rust bridge map-load and bridgehead derivation is not yet proven numerically
  equal to `OverlayClass::Mark`/bridgehead processing in gamemd. This can affect
  which cells receive ramp/deck facts before movement begins.
- `PathGrid` has an infantry-specific goal-height branch that was not traced here;
  it may matter for move orders ending on a bridge cell rather than crossing one.
- `cargo test -q bridge` has two unrelated bridge repair walker failures:
  `repair_scan_without_low_overlay_or_wood_tile_dispatches_high` and
  `repair_scan_low_overlay_dispatches_low_when_tile_predicate_false`.

## Verification

- Read-only Ghidra only. No mutating Ghidra tools were used.
- Targeted Rust bridge movement tests passed:
  - `on_bridge_fires_at_ramp_to_body_only`
  - `on_bridge_clears_at_ramp_to_ground_only`
  - `no_bridge_lookahead_pre_claim`
  - `test_layered_astar_can_traverse_bridge_after_unrelated_rebuild`
- Broad `cargo test -q bridge` was not clean because of the two unrelated bridge
  repair walker failures listed above.

## Verdict Tally

PASS: 3 | FAIL: 0 | UNCHECKED: 4 | NOT-IMPLEMENTED: 1

