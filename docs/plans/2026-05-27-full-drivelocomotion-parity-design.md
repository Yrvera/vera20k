# Full DriveLocomotion Parity Design

## Goal

Make normal Drive-locomotor vehicle movement run through a gamemd-shaped DriveLocomotion lifecycle instead of generic `MovementTarget` vector stepping, closing the AMCV trace-swarm movement, facing, arrival, crush, and bridge/tube parity gaps.

## Architecture Context

Current Rust movement is command/path first. `World::resolve_move_info` converts rules data into speed and a coarse crusher bit, then `issue_move_command_with_layered` builds an A* path and attaches `MovementTarget`. `tick_movement_with_grids` owns per-tick rotation, speed ramping, drive-track advancement where present, direct lepton stepping, cell crossings, occupancy, crush, low-bridge tube dispatch, and final `MovementTarget` cleanup. See `src/sim/world/world_commands.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_tick.rs`, and `src/sim/movement/movement_step.rs`.

The repo already has useful Drive pieces: `DriveTrackState`, `begin_drive_track`, `advance_drive_track`, `interp_sub_step`, forced refinery/miner turn tracks, layered bridge pathing, split ground/bridge occupancy, and a low-bridge tube state. The AMCV traces show the standard player path does not yet make those pieces the authoritative DriveLocomotion mechanism. Open-ground Drive can bypass DriveTrack, AMCV starts from a debug 3x speed multiplier, `current_speed` is stamped as full speed, arrival is Rust target cleanup rather than locomotor-issued `Set_Destination(NULL, 1)`, normal `Crusher=yes` is absent, and low-bridge tubes are simplified.

The fit point is a new Drive-locomotion-owned runtime state on `GameEntity`, not a replacement of pathfinding wholesale. `MovementTarget` can remain as path/order storage during transition, but for Drive units it must stop being the direct physics owner. The Drive owner must consume path directions through DriveTrack/Process_Movement style stages, call a binary-shaped runtime `Can_Enter_Cell`, apply crush through the UnitClass PerCellProcess-shaped path, and perform arrival through the NavCom/destination lifecycle.

Relevant architecture constraints:

- `sim/` remains deterministic and must not depend on render/ui/audio/net.
- Entity iteration order must stay deterministic through `EntityStore`.
- Simulation math must remain fixed-point or integer; rendering-only interpolation may use floats outside sim.
- Existing bridge, pathfinding, occupancy, and miner/refinery hooks must keep their ownership boundaries.
- Low-bridge TubeClass data belongs to `map/` and `sim/`; presentation of bridge traversal belongs above `sim/`.

## Impact Analysis

Primary touched surfaces:

- `src/rules/object_type.rs`: parse and carry `Crusher=` and likely `Accelerates=` as distinct TechnoType flags.
- `src/sim/game_entity.rs`: store regular crusher capability and DriveLocomotion-owned state fields.
- `src/sim/world/world_commands.rs`: remove deployable AMCV speed multiplier, compute crusher from `Crusher=yes`, route Drive orders through the Drive destination entrypoint.
- `src/sim/movement/movement_commands.rs`: split path/order assignment from direct physics setup; introduce a Drive `Set_Destination`/`Head_To_Coord` bridge.
- `src/sim/movement/movement_tick.rs`: dispatch Drive entities to DriveLocomotion process before generic fallback; replace generic finalization for Drive arrival.
- `src/sim/movement/movement_step.rs`: reduce direct Drive vector stepping to a fallback or remove it from normal Drive paths after parity state is active.
- `src/sim/movement/drive_track.rs`: remain the point-table executor, but become the normal Drive path rather than an optional curve overlay.
- `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_occupancy.rs`, `src/sim/pathfinding/core.rs`: add a shared runtime `Can_Enter_Cell` call shape that preserves A* explicit-parent semantics and runtime null-parent semantics.
- `src/sim/movement/tube_movement.rs` and `src/map/tube_facts.rs`: replace one-cell-per-tick low-bridge traversal with gamemd-shaped active tube payload and speed-budget cadence for Drive/Unit traversal.
- `src/sim/world/world_hash.rs`: hash any new DriveLocomotion and tube state fields.
- Tests under `src/sim/movement`, `src/sim/pathfinding`, `src/rules`, and AMCV trace acceptance fixtures.

Risk areas:

- Tick ordering: moving DriveLocomotion earlier or later relative to combat, miner docking, occupancy, and bridge state can change visible outcomes.
- Determinism: new residual/speed-budget fields must be serialized and hashed.
- Path/routing behavior: `Can_Enter_Cell` return codes and runtime effects are shared by A*, movement, crush, scatter, bridge, tube, and blocked wait/repath logic.
- Miner/refinery workflows: chrono miner drive piggyback and accepted-cell arrival already depend on stopped/accepted-cell semantics.
- Bridge layer state: high bridge object-list and occupancy-bit layer selection are independently chosen in gamemd and must not be collapsed.
- Low bridge tubes: same-cell zero-step tube shells exist but must not be consumed as visible direction-8 traversal inputs.

## Chosen Approach

Recommended approach: introduce a DriveLocomotion owner state and route normal Drive units through it, while preserving existing pathfinding and MovementTarget storage as transitional path/order data.

This approach fits the codebase because it reuses existing modules and data paths, but moves authority to the component that gamemd uses. The alternative of patching `MovementTarget` would keep the wrong owner in place and require every trace finding to be emulated piecemeal. A full movement rewrite would be too broad and would risk breaking Walk, Teleport, Fly, Jumpjet, miner docking, and bridge systems unrelated to Drive parity.

The chosen design has three phases:

1. Data and command ownership: parse `Crusher=`/`Accelerates=`, remove the AMCV multiplier, add DriveLocomotion state, and route Drive orders through a `set_drive_destination` entrypoint that supports both cell NavCom targets and object/building NavCom targets. Object/building targets are not optional for "full" DriveLocomotion parity because active gamemd obtains dock/approach coords from the target object.
2. Runtime Drive process: implement a Drive `process`/`process_movement`/`process_drive_track` pipeline that consumes path directions, computed Drive target speed fraction, residual budget, DriveTrack points, runtime `Can_Enter_Cell`, crush, blocked/repath, bridge/tube entry, and arrival.
3. Bridge/tube and acceptance closure: make low-bridge direction-8 and UnitClass tube movement use a gamemd-shaped payload and add acceptance fixtures for open ground, diagonal turn, detour, bridge/tube traversal, and crush-on-path.

## Tiny-Detail Ledger

- AMCV active YR data is `Speed=4`, `ROT=5`, `Crusher=yes`, Drive locomotor `{4A582741-9839-11d1-B709-00A024DDAFD1}`, and `MovementZone=Normal`; this is standard YR, not TS legacy. [ini: `ini/rulesmd.ini:6969-7000`; trace: `AMCV_OPEN_GROUND_DRIVE_TRACE_20260527.md`]
- AMCV open-ground target cell center for `(45,40)` is `(11648,10368,0)`, and flat cell centers use `cell * 256 + 128`. [trace: `AMCV_OPEN_GROUND_DRIVE_TRACE_20260527.md`]
- `FootClass::Set_Destination_Internal` writes NavCom and calls locomotor `Head_To_Coord`; Drive arrival later reaches the NavCom cell and calls `Set_Destination(NULL, 1)` when the queue is empty. [doc: `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`]
- Locomotor arrival, not Mission_Move, is the normal arrival owner; the empty-queue path clears NavCom through the public null-destination path. [doc: `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`]
- Current Rust `MovementTarget` cleanup is not equivalent to Drive arrival; it clears `movement_target`, `drive_track`, snaps subcell, and sets phase idle. [src: `src/sim/movement/movement_tick.rs:1110`; trace: `AMCV_OPEN_GROUND_DRIVE_TRACE_20260527.md`]
- Raw AMCV speed budget must come from stock `Speed=4`, not the deployable 3x debug multiplier. [src: `src/sim/world/world_commands.rs:73`; trace: `AMCV_OPEN_GROUND_DRIVE_TRACE_20260527.md`]
- Drive speed budget is `GetCurrentSpeed + residual`, except same-tick retry/chained calls add no new speed and use residual only. [doc: `DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`]
- DriveTrack point consumption spends budget in 7-unit chunks; leftover budget is stored at DriveLocomotion residual and can interpolate visible coordinates by `residual * 1/7`. Residual interpolation does not perform the per-point facing update. [doc: `DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`]
- `Accelerates=false` is a distinct TechnoType flag. For Drive, it skips ramp math and directly assigns the current target speed fraction before normal `GetCurrentSpeed` and DriveTrack budget consumption; it does not mutate raw `Speed=` or bypass terrain/slope factors. [doc: `GRIZZLY_ACCELERATES_FALSE_SEMANTICS_GHIDRA_REPORT.md`]
- Direction/facing mapping for the diagonal AMCV scenario maps `(1,1)` to SE direction index `3`, facing byte `96`, and 16-bit facing target `0x6000`. [trace: `AMCV_TURNING_DIAGONAL_DRIVE_TRACE_20260527.md`; doc: `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`]
- DriveTrack consumed points update facing by shifting the track point heading byte left 8 and calling the FacingClass update path after the point coord/cell update. [trace: `AMCV_TURNING_DIAGONAL_DRIVE_TRACE_20260527.md`; doc: `DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`]
- Exact stock AMCV starting facing in a standard YR skirmish is UNCHECKED and must be traced before asserting frame-perfect initial-turn cadence. [UNKNOWN - needs RE; trace: `AMCV_TURNING_DIAGONAL_DRIVE_TRACE_20260527.md`]
- Exact retail DriveTrack point list for the diagonal AMCV leg is UNCHECKED and must be dumped or proven before claiming pixel-perfect path positions. [UNKNOWN - needs RE; trace: `AMCV_TURNING_DIAGONAL_DRIVE_TRACE_20260527.md`]
- Exact AMCV final arrival tick, residual byte/value, and body-facing timeline are UNCHECKED for several trace fixtures. [UNKNOWN - needs RE; traces: AMCV trace set]
- A* runs a zone/hierarchy precheck and default five-attempt retry path; current initial move commands can pass `zone_grid: None` and run a single A* path. [trace: `AMCV_OBSTACLE_DETOUR_TRACE_20260527.md`; doc: `PATHFINDING_ASTAR_GHIDRA_REPORT.md`]
- Path smoothing applies after A* as direction-array corner smoothing then straight-segment optimization; validation uses `Can_Enter_Cell`, cliff flags, and slope checks, with direction 8 excluded from smoothing. [doc: `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md`]
- Exact gamemd obstacle-detour waypoint cells and post-smoothing direction array for the `(40,40)->(48,40)` AMCV fixture with blocker `(44,40)` are UNCHECKED. Rust currently computes a north detour, but that must not be used as the parity oracle. [UNKNOWN - needs RE; trace: `AMCV_OBSTACLE_DETOUR_TRACE_20260527.md`]
- `UnitClass::Can_Enter_Cell` return codes 0-7 must be preserved numerically; code 3 is allied scatter/building obstruction and code 6 is stationary allied non-building, not bridge-ramp/cliff. [doc: `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`]
- A* passes explicit parent/current node cell and current node/path height; runtime Drive/Ship/Hover valid-direction calls pass parent/current `0`, direction, current effective height, and arg5 `1`. These two call shapes must not be collapsed. [doc: `RUNTIME_CAN_ENTER_CELL_NONCOVERED_CALLSITES_GHIDRA_REPORT.md`]
- Runtime null-parent bridge traversal reconstructs predecessor as `target + DirectionOffset[(direction - 4) & 7]`. [doc: `RUNTIME_CAN_ENTER_CELL_NONCOVERED_CALLSITES_GHIDRA_REPORT.md`]
- Runtime current height is current cell level plus `4` only when persistent `OnBridge` is true; it is not target-layer height. [doc: `RUNTIME_CAN_ENTER_CELL_NONCOVERED_CALLSITES_GHIDRA_REPORT.md`]
- Low bridge identity is valid `CellClass+0x116` tube index plus `LandType == 10`, not overlay identity alone. [doc: `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`]
- Direction `8` is the low-bridge/tube path sentinel. Valid tube entry requires a non-null tube and nonzero endpoint; invalid tube returns hard block. [doc: `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`; doc: `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`]
- Direction-8 visible tube traversal requires a usable tube path length; same-cell zero-step auto shells exist but must not be consumed as visible traversal inputs. [doc: `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`]
- Active tube payload includes object tube index `+0x684`, cursor `+0x685`, copied path buffer, destination world coordinate from `Tube+0x28`, and Z accumulator seeded by signed division over `Tube+0x1C0`. [doc: `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`; trace: `AMCV_BRIDGE_RAMP_TRAVERSAL_TRACE_20260527.md`]
- Unit tube movement advances by movement budget; it may move partially toward the current tube target, increment cursor once when reached, and spend leftover budget partially into at most one next segment. It does not advance one tube cell per Rust tick. [doc: `LOW_BRIDGE_TUBEMOVEMENT_FINAL_Z_INTERPOLATION_GHIDRA_REPORT.md`; trace: `AMCV_BRIDGE_RAMP_TRAVERSAL_TRACE_20260527.md`]
- Unit low-bridge tube final exit snaps X/Y to `Tube+0x28`, keeps accumulated Z, clears active tube state, and uses the ground object list, not high-bridge `OnBridge`/AltObject landing. [doc: `LOW_BRIDGE_TUBEMOVEMENT_FINAL_Z_INTERPOLATION_GHIDRA_REPORT.md`; trace: `AMCV_BRIDGE_RAMP_TRAVERSAL_TRACE_20260527.md`]
- Regular crusher capability is `TechnoType+0xD28` / `Crusher=`, distinct from `OmniCrusher` and MovementZone. AMCV has `Crusher=yes` with `MovementZone=Normal`. [doc: `CRUSH_SYSTEM_GHIDRA_REPORT.md`; trace: `AMCV_CRUSH_ON_PATH_TRACE_20260527.md`]
- `UnitClass::PerCellProcess` applies crush when the unit has `Crusher=yes` or veteran crusher ability, victim `CanCrushCheck` passes, and distance squared is `<= 0x3FFF`. [doc: `CRUSH_SYSTEM_GHIDRA_REPORT.md`]
- Crush kill plays victim `CrushSound` at crusher coordinates before deletion, frees mind-control captures, records kill with the crusher, and removes the victim. [doc: `CRUSH_SYSTEM_GHIDRA_REPORT.md`; trace: `AMCV_CRUSH_ON_PATH_TRACE_20260527.md`]
- Current Rust crush path is cell-based, lacks the `0x3FFF` distance gate, does not parse normal `Crusher=`, and for the AMCV trace produces zero victims. [src: `src/rules/object_type.rs`; src: `src/sim/movement/bump_crush.rs`; trace: `AMCV_CRUSH_ON_PATH_TRACE_20260527.md`]

## Design

### Components

Add a DriveLocomotion state component or fields on `GameEntity`, shaped around the active binary concepts rather than Rust `MovementTarget` physics:

- `destination_coord`: Drive destination coordinate equivalent to `Drive+0x34/+0x38/+0x3C`.
- `head_to_coord`: current head-to/intermediate coordinate equivalent to `Drive+0x40/+0x44/+0x48`.
- `track_index`, `point_index`, `is_on_track`, `is_reversed`: DriveTrack lifecycle fields equivalent to `+0x58/+0x5C/+0x63/+0x60`.
- `target_speed_fraction`: Drive local fraction equivalent to `+0x50`.
- `residual_budget`: integer/fixed residual equivalent to `+0x4C`.
- `drive_delay` or equivalent timer only where verified by Drive reports.
- `active_tube`: Unit tube state equivalent to `+0x684/+0x685` plus copied path-buffer cursor, destination coord, and Z accumulator.
- `navcom`: a Rust destination reference sufficient to model the visible `Foot+0x5A4` lifecycle. The initial full-parity shape must represent at least cell targets and object/building targets, because non-null `Set_Destination_Internal` obtains current coordinates from the target's virtual dock/approach coord provider. A cell-only enum is only acceptable for a deliberately narrowed cell-move implementation, not for the full DriveLocomotion design.
- `navcom_aux_or_clear_marker`: `NavCom_Aux` appears effectively dead in scoped YR evidence, but the null/non-null destination path still clears it. Rust can model this as an explicit clear side effect rather than a useful pointer field.

`MovementTarget` remains as transitional path/order storage:

- It may store the current path cell sequence and final goal while DriveLocomotion owns movement.
- For Drive entities, `MovementTarget.current_speed`, direct `move_dir_x/y`, and generic finalization are not authoritative.
- Existing Walk/Infantry, direct miner scripted movement, Teleport piggyback, and tests can remain on `MovementTarget` until each family is migrated deliberately.

### Interfaces / Contracts

`set_drive_destination(...)`:

- Called from command surfaces for active Drive locomotor units.
- Mirrors the relevant `Set_Destination_Internal` side effects for Drive: clears NavCom_Aux, writes or clears NavCom, preserves the non-null early-return gates once verified for this call surface, writes the blocked/path retry timers required by the NavCom report, and calls clear-navigation/stop behavior through the same null-target path used by arrival.
- Computes/refreshes path data using existing pathfinding, but stores it as Drive path queue/directions for DriveLocomotion consumption.
- Calls Drive `head_to_coord` using the destination cell center or target object's dock/approach coordinate.
- Does not set `facing_target` as the primary vehicle turn owner.

`compute_drive_target_speed_fraction(...)`:

- Runs before `Process_Drive_Track` speed-budget consumption.
- Owns the Drive local `target_speed_fraction` equivalent to `DriveLocomotionClass+0x50`, including terrain, slope, health, group/formational, bridge/tube context, and any other verified modifiers.
- Feeds `Accelerates=false` by assigning the computed target fraction directly to the owning Techno speed fraction. It must not assign raw `Speed=`, unconditional `1.0`, or a terrain-ignorant value.

`process_drive_locomotion(...)`:

- Runs from `tick_movement_with_grids` for Drive entities before generic ground movement.
- Mirrors `DriveLocomotionClass::Process`: process active track, possibly process movement to select next track, then process drive track with retry/chained flag.
- Applies speed fraction/ramp rules, `GetCurrentSpeed`, residual budget, 7-budget point consumption, residual interpolation, facing updates, cell coordinate/occupancy updates, runtime `Can_Enter_Cell`, crush, blocked wait/repath, bridge/tube entry, and arrival.
- Returns explicit events: moved, blocked, crushed victims, started tube, arrived, stopped, needs generic fallback only for unsupported transitional states.

`runtime_can_enter_cell(...)`:

- Shared by DriveLocomotion, pathfinding, movement occupancy, bridge/tube logic, and later other locomotors.
- Takes a call-shape enum, not a simplified layer enum:
  - A* shape: candidate, direction, explicit parent/current cell, path height, arg5.
  - Runtime Drive shape: candidate, direction, current effective height, parent/current `None`, arg5 `1`.
  - Runtime direction `-1` candidate shape for later Hover/Jumpjet work.
- Returns numeric code 0-7 plus any bridge-list/height side effects needed by caller.
- Preserves object-list layer and occupancy-bit layer as independent values.

`apply_drive_per_cell_process(...)`:

- Handles crush using a UnitClass PerCellProcess-shaped pass, not only deferred cell occupancy.
- Uses parsed `Crusher=`, veteran crusher ability when implemented, victim `CanCrushCheck`, `0x3FFF` distance gate, victim sound at crusher coord, mind-control cleanup hook, kill attribution, and removal.
- Can call into existing bump/crush helpers only after their inputs are expanded to preserve the verified gates.

`begin_drive_tube_traversal(...)` and `tick_unit_tube_movement(...)`:

- Start only from direction-8 Drive/Walk producer shape with a valid nonzero path-length tube.
- Store active tube index/cursor/payload on the entity.
- Advance by speed budget with partial movement and at most one cursor increment plus residual partial step per tick.
- Exit at `Tube+0x28`, preserve accumulated Z, clear active tube state, and use ground-list final occupancy semantics.

### Data Flow

1. Player move command resolves object rules and entity state.
2. Drive units call `set_drive_destination`; non-Drive units continue using current entrypoints.
3. `set_drive_destination` assigns NavCom, computes path/directions, sets Drive head-to/destination, and leaves direct physics fields inactive.
4. Each tick, `tick_movement_with_grids` dispatches active Drive entities to `process_drive_locomotion`.
5. `process_drive_locomotion` computes the Drive target speed fraction, applies `Accelerates=` semantics, then consumes path directions through `runtime_can_enter_cell`, DriveTrack selection, and `process_drive_track`.
6. When a cell is entered, occupancy/layer changes apply immediately in gamemd order; `apply_drive_per_cell_process` handles crush and other cell-entry side effects.
7. If direction 8 is selected, Drive starts active tube traversal and later `tick_unit_tube_movement` owns movement until exit.
8. On arrival at NavCom cell with no queued path, Drive calls the Rust equivalent of `Set_Destination(NULL, 1)`; this clears NavCom/NavCom_Aux and Drive state through the shared null-target stop/clear-navigation path rather than direct `MovementTarget` finalization.

### Error Handling

This is simulation logic, so invalid states should be deterministic and explicit:

- Invalid or missing path target returns `false` from command setup and leaves prior movement state unchanged unless gamemd evidence says stop.
- Runtime hard-block code 7 clears/stops/repaths according to Drive Process_Movement evidence, not by panicking.
- Direction-8 with invalid or zero-step tube path must hard-block or stop according to the producer evidence; it must not start Rust's existing zero-step shell traversal.
- Internal invariant failures such as mismatched path/path_layers should remain debug assertions and deterministic release behavior.

### Testing Strategy

Focused tests before broad integration:

- Rules tests:
  - `object_type_parses_regular_crusher`
  - `object_type_accelerates_defaults_true`
  - `object_type_parses_accelerates_false`
- Command tests:
  - AMCV `Speed=4` is not multiplied by deployable 3x.
  - AMCV `Crusher=yes` sets regular crusher capability while `MovementZone=Normal`.
  - Drive move command writes cell NavCom state and does not set generic `facing_target` as the primary turn owner.
  - Drive move command to an object/building NavCom target uses the target's dock/approach coordinate provider, not a cell-only shortcut.
  - Null destination and arrival clear NavCom/NavCom_Aux and run the shared stop/clear-navigation path.
- Drive unit tests:
  - Open-ground AMCV uses `Speed=4` budget and residual.
  - `Accelerates=false` assigns the computed Drive target speed fraction, including a non-1.0 modifier fixture, before `GetCurrentSpeed`.
  - Diagonal Drive starts movement through DriveTrack/RateTimer path after exact RE facts are filled.
  - Arrival goes through `Set_Destination(NULL, 1)` equivalent.
- Runtime `Can_Enter_Cell` tests:
  - A* explicit parent and runtime null parent produce different reconstructed predecessor where verified.
  - Runtime current height comes from current effective height, not target layer.
  - Numeric return code taxonomy matches the verified table.
- Crush tests:
  - AMCV crushes centered enemy E1 on path when `Crusher=yes`.
  - Non-crusher `MovementZone=Normal` vehicle does not crush.
  - Distance squared `0x4000` fails while `0x3FFF` passes.
  - Crush sound emits at crusher coord.
- Low-bridge/tube tests:
  - Direction 8 requires valid tube and nonzero endpoint.
  - Zero-step auto shell is valid for predicate/zone but not visible traversal.
  - Unit tube movement advances by speed budget, not one path cell per tick.
  - Final low-bridge exit uses ground-list occupancy and accumulated Z.
- Trace acceptance:
  - Re-run the five AMCV traces as acceptance reports after implementation.

### Determinism

All new DriveLocomotion and tube fields must be serialized and included in `world_hash`. Budget/residual math should use integer or fixed-point representations that match verified truncation/clamp behavior. Entity iteration stays sorted by stable id. Any victim selection must preserve gamemd object-list order or explicitly document `UNKNOWN - needs RE` if Rust occupancy order still differs.

## Architectural Decisions

### Decision 1: DriveLocomotion Owns Drive Movement

Use a dedicated DriveLocomotion state for normal Drive units. This follows the binary owner and prevents further patches from adding Drive-specific exceptions to generic `MovementTarget` stepping.

### Decision 2: MovementTarget Becomes Transitional Path/Order Data

Do not delete `MovementTarget` in this design. Many non-Drive systems still depend on it. For Drive, keep it only as a path/order compatibility surface until callers are migrated to explicit NavCom/path queue data.

### Decision 3: Runtime Can_Enter_Cell Is Shared But Call-Shape Aware

Do not add separate approximate passability helpers for Drive, A*, tube, and bridge. Use one core evaluator with explicit call shapes so A* explicit-parent and runtime null-parent semantics remain distinct.

### Decision 4: Low-Bridge Tubes Stay Separate From High-Bridge Layers

Low bridges are TubeClass movement, not high-bridge `OnBridge` deck traversal. The design keeps low-bridge tube payload and final ground-list semantics separate from high-bridge bridge occupancy.

### Decision 5: Unknown Exact Timing Facts Are Research Gates

The design proceeds with architecture because the owner/mechanism mismatch is proven. Exact stock starting facing, exact DriveTrack point list, exact frame timeline, and exact residual bytes remain blocking gates before parity-complete claims or pixel-perfect acceptance.

## Alternatives Considered

### Alternative A: Patch MovementTarget In Place

This would remove the 3x multiplier, add `Crusher=`, and teach current generic stepping more DriveTrack cases. It is lower risk for a quick AMCV symptom fix, but it keeps the wrong owner. It would still drift on NavCom arrival, `Process_Drive_Track` residual cadence, runtime `Can_Enter_Cell` argument shape, direction-8 tube entry, and DriveTrack facing updates. Rejected because it creates known parity drift.

### Alternative B: Full Movement Rewrite For All Locomotors

This would replace `MovementTarget` with a binary-shaped locomotor suite for Drive, Walk, Ship, Hover, Jumpjet, Fly, Teleport, and Tube at once. It has good long-term parity shape, but it is too broad for the AMCV trace findings and would mix verified Drive facts with less-verified locomotor families. Rejected for unnecessary blast radius.

### Alternative C: DriveLocomotion Owner With Transitional Path Storage

This is the chosen approach. It closes the specific active-gamemd owner mismatch while reusing existing pathfinding, occupancy, bridge, drive-track, and miner integration points. It introduces new state but keeps the implementation path reviewable and testable.

## Follow-Up Research Gates

- Trace exact stock standard-YR AMCV skirmish starting facing.
- Dump or prove exact retail DriveTrack point list for open-ground straight and diagonal AMCV legs.
- Capture exact per-frame body-facing sequence and first movement tick for AMCV east and SE scenarios.
- Capture exact arrival tick/residual bytes for AMCV open-ground and diagonal fixtures.
- Capture exact gamemd waypoint cells and post-smoothing direction array for the AMCV obstacle-detour fixture before using a Rust-computed detour as an oracle.
- Audit Rust occupancy insertion/list order against gamemd `Object+0x30` order for runtime `Can_Enter_Cell`.
- If bridge/tube implementation reaches Hover/Ship/Mech before Drive is complete, line-by-line decompile those producer sites first.
