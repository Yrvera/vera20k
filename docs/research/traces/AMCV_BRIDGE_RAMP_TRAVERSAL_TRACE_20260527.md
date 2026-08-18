# AMCV Low-Bridge Ramp Traversal Runtime Locomotion Trace

Date: 2026-05-27

Scenario: AMCV drives from ground over a low bridge ramp onto the bridge span and off the far ramp. Scope is runtime locomotion only: `Can_Enter_Cell`/tube-entry argument shape, layer/height handling, path layer, movement occupancy, and visible movement continuity.

Verdict rule: PASS requires literal numerical equality between Rust and active `gamemd.exe`. If both sides were not computed, the stage is UNCHECKED. No Rust code or INI was modified.

## Scenario Data

Retail/YR AMCV values from `ini/rulesmd.ini`:

- `[AMCV]` has `Speed=4`, `ROT=5`, `Crusher=yes`, `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}`, and `MovementZone=Normal` at `ini/rulesmd.ini:6969-7000`.
- The drive locomotor GUID is the stock wheeled/tracked drive locomotor used by the live drive producer reports.

Concrete low-bridge model from verified research:

- Low bridges are TubeClass-backed, not high-bridge `OnBridge` deck traversal.
- Active low-bridge predicate is `CellClass::IsLowBridgeCell`: valid `cell+0x116` tube index and `cell+0xEC == 10`.
- Direction `8` is the tube path sentinel. For explicit `[Tubes]` data, it jumps from tube entry to `TubeClass+0x28` exit and then visible TubeMovement consumes `TubeClass+0x30` path steps.

## Pipeline

Order traced:

1. AMCV order/path search produces a path containing a direction-8 low-bridge tube edge.
2. Drive locomotion reaches the tube entry cell.
3. Runtime tube entry validates current-cell tube data and starts active tube movement.
4. Unit AI dispatches TubeMovement while the active tube index is non-negative.
5. TubeMovement advances through in-tube path steps.
6. TubeMovement exits at the far ramp/exit cell, updates occupancy, facing, and visible position.

## Stage Verdicts

| Stage | Rust output for this scenario | Active gamemd output | Verdict |
|---|---|---|---|
| AMCV data load | Source INI says AMCV `Speed=4`, `ROT=5`, `Crusher=yes`, drive locomotor, `MovementZone=Normal`; this trace did not execute the Rust rules loader to prove parsed runtime equality. | Same source values in active YR rules data, but no live gamemd memory dump was captured. | UNCHECKED |
| Low-bridge path representation | Rust has `TubeFact { entry, exit, direction, path_steps, source }`, `TubeSource::{AutoLowBridge, ExplicitMap}`, and `path_len() = path_steps.len()` in `src/map/tube_facts.rs:28-68`. A* only exposes a direction-8 edge for `ExplicitMap`, nonzero `path_len`, nonzero exit in `src/sim/pathfinding/core.rs:675-685` and pushes it as a ground-layer edge at `src/sim/pathfinding/core.rs:1320-1354`. | `gamemd` low bridges use `cell+0x116` tube index plus `LandType==10`; `MapCoord_Step_By_Direction` and path walking treat direction `8` as a tube jump to `Tube+0x28`. Verified active in standard YR. | UNCHECKED |
| Runtime `Can_Enter_Cell` / tube-entry shape | Rust does not call a UnitClass-shaped `Can_Enter_Cell(..., direction=8, ...)` for tube entry. `try_begin_path_tube_step` instead detects a non-adjacent next path coordinate, reads current cell `tube_index`, checks `tube.exit == next`, and begins tube state at `src/sim/movement/tube_movement.rs:57-96`. It also accepts zero-step auto shells through `begin_low_bridge_tube_movement` at `src/sim/movement/tube_movement.rs:44-55`, with tests pinning that at `src/sim/movement/tube_movement.rs:368-377` and `510-540`. | `UnitClass::Can_Enter_Cell @ 0x0073F0A0` is live for vehicle cell-entry legality. For low bridge/tube, direction `8` requires a valid tube and nonzero endpoint data and then returns clear; low bridge predicate is tube index plus `LandType==10`. Active in YR per `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md` and `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`. | FAIL |
| Active tube state initialization | Rust stores only `tube_id`, `cursor`, `entry`, `exit`, and phase in `LowBridgeTubeMovementState` at `src/sim/movement/tube_movement.rs:23-30`. It does not store a gamemd-style Z accumulator, copied path buffer, tube destination world coord, or tube path count divisor. | Drive producer enters when path direction is `8` and current drive track is `-1`; it reads `cell+0x116`, sets destination from `Tube+0x28`, copies path buffers, writes `object+0x63C=-1`, writes `object+0x684=tube_index`, clears `object+0x685`, and seeds `object+0x570 = current_ground + signed_trunc((exit_ground-current_ground)/Tube+0x1C0)`. Active in YR at `0x004B0F20` / write site `0x004B1380`. | NOT-IMPLEMENTED |
| Per-tick tube movement cadence | Rust advances exactly one tube path cell per Rust tick: read `tube.path_steps[cursor]`, move to that next cell, increment cursor once, and optionally finish immediately in `src/sim/movement/tube_movement.rs:124-147`. | gamemd uses speed-budget interpolation. If distance to current target exceeds budget, it moves partially and does not increment cursor; if it reaches the target, it increments cursor once and may spend leftover budget partially into the next segment, but does not drain arbitrary tube cells in one AI call. Active unit evidence: `UnitClass::TubeMovement @ 0x007359F0`, especially `0x00735A05..0x00735D34`. | FAIL |
| Unit final Z and far-ramp placement | Rust `finish_tube_movement` snaps to `state.exit` and then `move_entity_to_cell` may set `position.z` from bridge deck/low-bridge cell data through `resolve_tube_landing_bridge_state` at `src/sim/movement/tube_movement.rs:151-160` and `237-270`. It resets subcell to center at `src/sim/movement/tube_movement.rs:209-212`. | gamemd unit final branch writes X/Y from `Tube+0x28` center and writes Z from `object+0x570`; it does not recompute or clamp final Z from the exit cell ground or bridge deck. Active evidence: `0x00735FA1..0x00735FEC`. | FAIL |
| Occupancy/list layer on low-bridge final exit | Rust infers low-bridge landing as `MovementLayer::Bridge` when the destination is bridge-walkable / low-bridge tube cell at `src/sim/movement/tube_movement.rs:224-235`, projects a bridge update at `src/sim/movement/tube_movement.rs:262-270`, and moves occupancy into the projected bridge layer at `src/sim/movement/tube_movement.rs:181-207`. Tests pin this current behavior for explicit and zero-step tube cases at `src/sim/movement/tube_movement.rs:380-445` and `448-506`. | gamemd low-bridge TubeMovement final blocker checks use the ground object list `CellClass+0xE4`; this is not high-bridge `AltObject/+0xE8` or `OnBridge` landing. Active evidence: unit final blocker range `0x00735E5F..0x00735E6E` and low/high object-list comparison docs. | FAIL |
| Visible movement continuity | Rust tube movement changes grid cell and refreshes screen coords after each one-cell tube step (`src/sim/movement/tube_movement.rs:140-147`, `209-221`), not a speed-budget partial interpolation. | gamemd TubeMovement computes distance to the current tube target, moves partially when budget is insufficient, and can spend residual budget into one next segment after a cursor increment. This changes per-tick screen position and final Z continuity. | FAIL |
| Active standard-YR confirmation | Ghidra MCP spot decompile by raw address was unavailable in this session (`Function not found` for the raw addresses), so this trace relies on existing verified research docs for active YR confirmation. | The cited reports explicitly mark the drive producer, UnitClass TubeMovement, UnitClass AI dispatch, UnitClass Can_Enter_Cell, low bridge predicates, and direction-8 path walking as active in standard YR, not dormant TS-only code. | UNCHECKED |

## Failures

1. Runtime tube entry is not modeled as the active `UnitClass::Can_Enter_Cell` direction-8 gate. Rust starts tube state from path shape and current-cell tube exit, and accepts zero-step shells that gamemd drive/walk producers cannot consume as visible traversal inputs without dividing by zero.
2. Rust lacks the gamemd active tube-state payload: `object+0x684`, `object+0x685`, path-buffer copy semantics, destination world coord, and the `object+0x570` Z accumulator seeded from signed integer division by `TubeClass+0x1C0`.
3. Rust advances one tube path cell per simulation tick. gamemd advances by movement budget and preserves residual movement into at most one next segment.
4. Rust final low-bridge tube exit writes bridge-style state and deck Z. gamemd unit final exit uses ground object list `CellClass+0xE4`, snaps X/Y to `Tube+0x28`, and keeps accumulated `object+0x570` Z.
5. Rust occupancy can move the AMCV into bridge occupancy on low-bridge cells. gamemd low-bridge TubeMovement final exit is ground-list based and is not the high-bridge `OnBridge` / `AltObject` mechanism.

## Adjacent Findings

- Current Rust high-bridge ramp/body occupancy is much closer: `movement_step.rs` projects `on_bridge` before choosing insertion layer, and tests pin ground->ramp no preclaim, ramp->body bridge insertion, body->ramp bridge retention, and ramp->ground clear. That is not this low-bridge trace.
- Runtime drive/walk `Can_Enter_Cell` for ordinary non-tube bridge motion has a separate active argument-shape issue: gamemd passes `target_cell, direction, current_effective_height, 0, 1`, while Rust runtime layer reconstruction supplies an explicit parent and uses path-layer/Z inputs. This matters adjacent to bridgehead runtime collision but is not the low-bridge TubeMovement failure.

## Verification Notes

- No Cargo tests were run, because the subagent was constrained to write exactly one file and Cargo would write build artifacts.
- No source files, INI files, shared claim files, or published docs outside this trace report were modified.

## Verdict Tally

PASS: 0 | FAIL: 5 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

## Sources

- `ini/rulesmd.ini:6969-7000` for AMCV drive locomotor and movement values.
- `docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md` for active `UnitClass::Can_Enter_Cell`, low-bridge tube predicate, direction-8 legality, and confirmed Rust parity gaps.
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` for `IsLowBridgeCell`, `GetTubeAtCell`, direction-8 stepping, `[Tubes]` parser facts, UnitClass TubeMovement, and active YR confirmation.
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md` for drive/walk active tube producers and zero-step shell distinction.
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBEMOVEMENT_FINAL_Z_INTERPOLATION_GHIDRA_REPORT.md` for active UnitClass TubeMovement timing, final Z, final ground-list exit, and current Rust status.
- `src/map/tube_facts.rs:28-68`, `src/sim/pathfinding/core.rs:675-685`, `src/sim/pathfinding/core.rs:1320-1354`, `src/sim/movement/tube_movement.rs:23-270`, `src/sim/movement/movement_tick.rs:440-453`, `src/sim/movement/movement_tick.rs:562-575`.
