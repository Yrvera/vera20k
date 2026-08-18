# Implementation Trace: Low Bridge TubeClass Height / Layer Updates

Date: 2026-05-21

Scenario: a ground unit enters an automatic zero-step or short low-bridge
`TubeClass` shell on a `LandType=10` low bridge cell and completes/snap-exits.

Scope is limited to tube start acceptance, movement completion, `position.z`,
`on_bridge`, `bridge_occupancy`, occupancy layer, and standard YR/gamemd
low-bridge TubeClass movement evidence.

## Sources Checked

- Rust:
  - `src/sim/movement/tube_movement.rs`
  - `src/sim/movement/movement_tick.rs`
  - `src/map/tube_facts.rs`
  - `src/map/resolved_terrain.rs`
- Existing verified docs:
  - `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
  - `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`
  - `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`
- Read-only Ghidra spot checks:
  - `CellClass::IsLowBridgeCell @ 0x00484AB0`
  - `CellClass::GetTubeAtCell @ 0x00484F20`
  - `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
  - `WalkLocomotionClass::ProcessMovement @ 0x0075AEC0`
  - `UnitClass::TubeMovement @ 0x007359F0`

All gamemd functions referenced above are documented as active in standard YR
and were spot-checked read-only in this run. No Ghidra mutating tool was used.

## Pipeline

Trigger: movement tick sees a path step that Rust treats as a tube step.

Rust chain:

1. `movement_tick.rs:428` runs active low-bridge tube movement before normal movers.
2. `movement_tick.rs:547` calls `try_begin_path_tube_step`.
3. `tube_movement.rs:57` identifies same-cell/non-adjacent tube path steps.
4. `tube_movement.rs:44` creates `LowBridgeTubeMovementState`.
5. `tube_movement.rs:99` advances active tube movement.
6. `tube_movement.rs:151` finishes by snapping to `state.exit`.
7. `tube_movement.rs:168` moves occupancy and updates entity fields.
8. `tube_movement.rs:224` infers a bridge landing layer for low-bridge tube cells.
9. `tube_movement.rs:237` projects landing bridge state, z, and `on_bridge`.

gamemd chain:

1. `CellClass::IsLowBridgeCell @ 0x00484AB0` requires valid `cell+0x116` tube index
   and `LandType == 10`.
2. `CellClass::GetTubeAtCell @ 0x00484F20` returns the indexed `TubeClass`.
3. Drive/walk locomotion enters tube movement only when current path direction is
   `8`, then writes active tube index/cursor to object `+0x684/+0x685`.
4. `UnitClass::TubeMovement @ 0x007359F0` consumes `TubeClass+0x24`, `+0x28`,
   `+0x2C`, `+0x30`, and `+0x1C0`.
5. On completion, it places the unit at `TubeClass+0x28`, clears active tube state
   by writing `0xFF` to `+0x684`, checks the exit cell ground object list
   `cell+0xE4`, and resumes normal movement state.

## Stage Results

### Stage 1 - Auto Tube Fact Shape

Rust output:

- `TubeFact::auto_low_bridge((x,y), dir)` creates:
  - `entry = (x,y)`
  - `exit = (x,y)`
  - `path_steps = []`
  - `path_len = 0`
  - source `AutoLowBridge`
- `AUTO_TUBE_DIRECTIONS = [2,4,6,0]`.

gamemd output:

- `TubeClass::Constructor @ 0x00727FD0` creates same-cell shell fields:
  - `Tube+0x24 = coord`
  - `Tube+0x28 = coord`
  - `Tube+0x2C = direction`
  - `Tube+0x1C0 = 0`
  - path buffer filled with `-1`
- Direction table is `[2,4,6,0]`.

Verdict: PASS. For a non-`(0,0)` qualifying cell, the static auto-shell data shape
matches numerically at the fields traced here.

### Stage 2 - Entry Trigger / Path Representation

Rust output:

- `try_begin_path_tube_step` starts tube movement from coordinate path shape:
  - adjacent nonzero delta: not tube;
  - same-cell or non-adjacent step: probe current cell `tube_index`;
  - if `tube.exit == next`, begin tube movement.
- No direction-8 path-step value is represented or checked.

gamemd output:

- Drive/walk producers enter active tube movement only when the current path
  direction is exactly `8`.
- They read `CellClass+0x116`, load `g_TubeArray[index]`, then write active tube
  index and cursor into object `+0x684/+0x685`.

Verdict: NOT-IMPLEMENTED. Rust has a coordinate-shape heuristic instead of the
standard YR direction-8 producer state.

### Stage 3 - Zero-Step Shell Acceptance

Rust output:

- `begin_low_bridge_tube_movement` accepts a zero-step auto shell.
- `tick_low_bridge_tube_movement` sees `cursor >= tube.path_len()` immediately.
- `finish_tube_movement` snap-completes to `state.exit` on the next tube tick.

gamemd output:

- Same-cell zero-step shells exist as map facts.
- The checked Drive and Walk producer branches divide by `TubeClass+0x1C0` while
  initializing tube traversal and have no zero guard.
- Prior verified lifecycle report conclusion: same-cell shells matter for low
  bridge predicates/zones/click logic, but should not be consumed as direction-8
  visible traversal inputs.

Verdict: FAIL. Rust turns the zero-step shell into a visible snap movement path;
standard YR evidence does not support a valid visible zero-step traversal.

### Stage 4 - Short Explicit Tube Completion Coordinate

Rust output for a two-step explicit tube `entry=(0,0)`, `exit=(2,0)`,
`path_steps=[2,2]`:

- tick 1: position cell becomes `(1,0)`;
- tick 2: position cell becomes `(2,0)`;
- tube state is cleared.

gamemd output for a fully initialized tube with `Tube+0x28=(2,0)`:

- `UnitClass::TubeMovement` completion places the unit at `TubeClass+0x28`.

Verdict: PASS for final cell coordinate only: `(2,0) == (2,0)` in this concrete
short-tube shape. Timing and intermediate pixels are separate unchecked stages.

### Stage 5 - Short Tube Timing / Interpolation

Rust output:

- One path-step cell advance occurs per `tick_low_bridge_tube_movement` call.
- The implementation does not compute gamemd's in-tube lepton interpolation from
  locomotor speed, remaining distance, and `TubeClass+0x1C0`.

gamemd output:

- `UnitClass::TubeMovement` interpolates through tube path points and Z increments
  using `TubeClass+0x1C0`, ground heights, and locomotor movement budget.

Verdict: UNCHECKED. I computed Rust's synthetic test step count but did not compute
the exact gamemd tick cadence and per-tick coordinates for the same retail unit.

### Stage 6 - Exit Occupancy Layer

Rust output:

- `infer_tube_landing_layer` returns `MovementLayer::Bridge` when the destination
  is bridge-walkable and is a low-bridge tube cell.
- `move_entity_to_cell` moves occupancy to the bridge layer when projected
  `on_bridge` becomes true.

gamemd output:

- `UnitClass::TubeMovement` checks the exit cell ground object list at
  `cell+0xE4` before completing.
- `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md` and the spot-checked tube
  movement path separate low-bridge/tube handling from high-bridge `AltObject`
  / bridge-list semantics.

Verdict: FAIL. Rust puts the low-bridge tube exit occupant on bridge occupancy;
gamemd tube exit uses the ground object list for the checked unit path.

### Stage 7 - `on_bridge` / `bridge_occupancy`

Rust output:

- `resolve_tube_landing_bridge_state` sets `position.z = deck_level` and returns
  `BridgeStateUpdate::Set(deck_level)` for bridge-walkable low-bridge tube cells.
- `apply_pending_bridge_render_state` then sets:
  - `on_bridge = true`
  - `bridge_occupancy = Some(deck_level)`
  - locomotor layer to bridge

gamemd output:

- The checked low-bridge tube completion clears active tube state and updates
  normal movement state.
- The high-bridge `OnBridge` writer path is separate normal bridge movement logic;
  no matching low-bridge TubeMovement write to the high-bridge `OnBridge` state was
  found in the checked active unit tube path.

Verdict: FAIL. Rust turns low-bridge tube completion into high-bridge-style
`on_bridge` / `bridge_occupancy` state.

### Stage 8 - `position.z`

Rust output:

- In the focused zero-step shell test, destination deck level is `4`, so Rust
  writes `position.z = 4`.
- In the explicit two-step test, destination bridge deck level is `4`, so Rust
  finishes with `position.z = 4`.

gamemd output:

- Producer and tube movement code use `CellClass::GetGroundHeight`, tube entry/exit
  cells, and `TubeClass+0x1C0` to compute Z interpolation.
- I did not compute the exact gamemd Z number for the same retail low-bridge tile,
  unit, path length, and tick.

Verdict: UNCHECKED for exact numeric Z. The surrounding layer evidence strongly
suggests Rust's deck-level write is suspect, but exact gamemd Z was not computed.

### Stage 9 - Runtime Bridge State / Damage Gate

Rust output:

- `tick_low_bridge_tube_movement` builds a fresh `PathGrid` from static
  `ResolvedTerrainGrid` each tube tick.

gamemd output:

- Existing low-bridge docs show damage/repair updates overlay/state/zones and
  validates/invalidates bridge zones while tube identity may remain attached.

Verdict: UNCHECKED for this intact-cell scenario. This is an adjacent risk for
destroyed/repaired low bridges, not a computed mismatch in the single intact
zero-step/short-tube scenario traced here.

## Findings

1. Rust now accepts and snap-completes automatic zero-step tube shells, but active
   gamemd producer evidence says zero-step shells are not valid visible direction-8
   traversal inputs.
2. Rust lacks the direction-8 path-step producer model and instead starts tube
   movement from coordinate shape.
3. Rust lands low-bridge tube movement on the bridge occupancy layer; gamemd unit
   TubeMovement uses the exit cell ground object list.
4. Rust sets high-bridge-style `on_bridge` and `bridge_occupancy` on low-bridge
   tube landing; the checked gamemd low-bridge tube path does not.
5. Exact final Z remains unchecked because gamemd's low-bridge tube Z formula was
   not numerically evaluated for a matching retail fixture.

## Adjacent Findings

- Static `PathGrid::from_resolved_terrain` inside tube ticking cannot reflect
  destroyed/repaired low-bridge runtime state. This was not traced as a failure in
  the intact-cell scenario, but it is likely to matter when low bridge damage state
  changes.
- Infantry has a parallel tube movement routine. This report spot-checked unit
  `TubeMovement`; infantry should be traced separately before claiming full infantry
  parity.

## Verdict Tally

PASS: 2 | FAIL: 3 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1

## Status

COMPLETE
