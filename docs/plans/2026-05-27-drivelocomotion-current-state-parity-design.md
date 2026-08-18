# DriveLocomotion Current-State Parity Design

## Goal

Finish normal Drive-locomotor parity by making the existing NavCom and DriveLocomotion runtime scaffold authoritative for Drive movement, while closing the remaining AMCV trace-swarm gaps without replacing unrelated movement systems.

## Architecture Context

Current Rust movement is in a transition state. Older AMCV trace reports correctly identified major DriveLocomotion gaps, but several findings are now stale:

- The AMCV deployable 3x speed multiplier is no longer present. `Simulation::resolve_move_info` now uses `ra2_speed_to_leptons_per_second(o.speed)` directly, and `resolve_move_info_uses_stock_amcv_speed_without_deployable_multiplier` pins stock AMCV speed.
- `Crusher=` and `Accelerates=` are parsed and stored on entities as `regular_crusher` and `drive_accelerates`.
- `NavigationState`, `NavTargetRef`, `DriveCoord`, and `DriveLocomotionRuntime` exist in `src/sim/components.rs`.
- Normal Drive move commands already call `movement::navcom::set_destination_internal_cell`, seed Drive path directions, and initialize Drive speed fraction state.
- `movement_step.rs` now has Drive-specific residual-budget helpers and retry-after-selection support.

The core architecture is still command/path first. `MovementTarget` stores path cells, path layers, speed, final goal, retry timers, and direct vector fields. `tick_movement_with_grids` owns Drive and non-Drive movement in one broad loop: speed updates, DriveTrack stepping, straight lepton stepping, cell crossing, bridge state, occupancy, crush, tube dispatch, and finalization.

The intended fit point is not a rewrite. The repo already has the right native-shaped pieces. The design should keep those pieces and finish moving Drive-only authority out of generic `MovementTarget` fields:

- Owner destination state belongs to `NavigationState`.
- Drive destination, path direction cursor, speed fraction, residual budget, active track, and tube payload belong to `DriveLocomotionRuntime` and DriveTrack state.
- `MovementTarget` can remain a compatibility path adapter during migration, but for Drive units it must not be the source of truth for speed, arrival, facing, DriveTrack consumption, tube traversal, or crush eligibility.

Project constraints that apply:

- `sim/` must not depend on render, UI, sidebar, audio, or net.
- Movement state must remain deterministic, serializable, and hashable.
- Simulation math must remain integer/fixed-point.
- `EntityStore` deterministic iteration order must be preserved.
- Existing Walk, Teleport, Fly, Jumpjet, miner/refinery, bridge, and production behavior must not be silently pulled into the Drive migration without verified evidence.

## Impact Analysis

Primary touched modules:

- `src/sim/components.rs`: existing `NavigationState`, `DriveLocomotionRuntime`, `DriveTubePayload`, and any missing Drive fields.
- `src/sim/game_entity.rs`: serialized entity fields, defaults, hashing surfaces.
- `src/rules/object_type.rs`: already parses `Crusher=` and `Accelerates=`, but tests and field semantics remain part of the acceptance surface.
- `src/sim/world/world_commands.rs`: move-info propagation and `mover_is_crusher` vs `regular_crusher` semantics.
- `src/sim/world/world_spawn.rs`: already initializes `regular_crusher` and `drive_accelerates`; acceptance tests should keep this pinned.
- `src/sim/movement/navcom.rs`: owner destination commit, null destination, pending arrival clear.
- `src/sim/movement/movement_commands.rs`: Drive command setup, NavCom commit, path direction seeding, and DriveTrack start.
- `src/sim/movement/drive_locomotion.rs`: Drive speed fraction owner; likely grows to include more Process/Process_Movement helpers.
- `src/sim/movement/movement_tick.rs`: dispatch order, speed fraction update, Drive process integration, finalization, and pending arrival clear.
- `src/sim/movement/movement_step.rs`: DriveTrack budget consumption and the point where straight vector movement still acts as fallback.
- `src/sim/movement/movement_occupancy.rs` and `src/sim/pathfinding/cell_entry.rs`: runtime `Can_Enter_Cell`, crush, scatter, and occupied-cell return codes.
- `src/sim/pathfinding/zone_search.rs` and `src/sim/movement/movement_path.rs`: zone precheck and retry approximation versus active gamemd behavior.
- `src/sim/movement/tube_movement.rs` and `src/map/tube_facts.rs`: low-bridge TubeMovement payload and timing.
- `src/sim/world/world_hash.rs`: deterministic hashing for any new or reinterpreted state.

Risk areas:

- Drive migration can accidentally change Walk or scripted direct movement if generic `MovementTarget` behavior is changed too broadly.
- Existing Drive tests may pass while still masking parity drift if Drive falls back to straight vector stepping.
- Regular `Crusher=yes` is currently stored but intentionally inert for `mover_is_crusher`; wiring it changes pathing and runtime cell-entry behavior.
- Low-bridge tube traversal currently uses a simplified model; fixing it touches path direction 8, bridge layer assumptions, occupancy, and Z.
- Pathfinding zone search already has a 5-retry approximation, but command paths can still pass `zone_grid: None`; implementation must distinguish "feature exists" from "this producer uses it."
- Arrival must preserve the current scaffold's `pending_arrival_clear` behavior while matching gamemd's empty-queue `Set_Destination(NULL, 1)` lifecycle.

## Chosen Approach

Approach A: finish the existing native-like DriveLocomotion scaffold.

The design keeps `MovementTarget` as a transitional path adapter but makes `NavigationState`, `DriveLocomotionRuntime`, and `DriveTrackState` authoritative for normal Drive-locomotor units. This matches the direction already present in the current code and avoids both a generic movement patch pile and a risky full rewrite.

The implementation should be staged by ownership:

1. Pin current fixes and stale-trace corrections with tests.
2. Make regular crusher capability flow into the Drive/pathing/cell-entry decisions without conflating it with `MovementZone`.
3. Close DriveTrack authority gaps so normal Drive movement does not fall back to unproven straight vector stepping in cases where gamemd consumes DriveTrack/path directions.
4. Tighten NavCom arrival clearing and queued-arrival behavior around the existing `pending_arrival_clear` scaffold.
5. Replace low-bridge TubeMovement's simplified cell-per-tick/bridge-landing model with the verified UnitClass tube payload and speed-budget cadence.
6. Re-audit pathfinding producer inputs so player move commands use the intended zone precheck/retry surface where available.

## Tiny-Detail Ledger

- AMCV active YR data is `Speed=4`, `ROT=5`, `Crusher=yes`, Drive locomotor, and `MovementZone=Normal`. Source: `ini/rulesmd.ini:6969-7000`; `docs/research/traces/AMCV_OPEN_GROUND_DRIVE_TRACE_20260527.md`.
- Current Rust no longer has the old AMCV 3x deployable multiplier. Source: `src/sim/world/world_commands.rs`; current test `resolve_move_info_uses_stock_amcv_speed_without_deployable_multiplier`.
- `Crusher=yes` is parsed and stored as `regular_crusher`, but current move info keeps it separate from `mover_is_crusher`. Source: `src/rules/object_type.rs`, `src/sim/game_entity.rs`, `src/sim/world/world_commands.rs`; test `resolve_move_info_carries_regular_crusher_but_keeps_legacy_crush_inert`.
- `Accelerates=` is parsed and stored as `drive_accelerates`; Drive speed fraction update consumes it. Source: `src/rules/object_type.rs`, `src/sim/movement/drive_locomotion.rs`, `src/sim/movement/movement_tick.rs`.
- Normal Drive move command writes owner NavCom separately from `MovementTarget`. Source: `src/sim/movement/movement_commands.rs`, `src/sim/movement/navcom.rs`, tests in `src/sim/movement/movement_tests.rs`.
- `FootClass::Set_Destination_Internal` writes NavCom and calls locomotor `Head_To_Coord`; normal empty-queue Drive arrival later calls `Set_Destination(NULL, 1)`. Source: `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`.
- Drive destination/head-to state is distinct from active path/track execution; Rust has `DriveLocomotionRuntime.destination` and `head_to`. Source: `docs/research/DRIVELOCOMOTION_HEAD_TO_COORD_CLEAR_NAVIGATION_STATE_GHIDRA_REPORT.md`; `src/sim/components.rs`.
- Drive speed budget is `GetCurrentSpeed + residual`; same-tick retry/chained calls add no fresh speed and use residual only. Source: `docs/research/miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`.
- DriveTrack point cost is exactly 7 budget units; residual interpolation uses `1/7` and does not perform the per-point facing update. Source: `docs/research/miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`.
- Track point heading updates body facing by shifting heading byte left 8 and calling the FacingClass update path after point coord/cell update. Source: `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`; `docs/research/miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`.
- Direction mapping is `N,NE,E,SE,S,SW,W,NW`; `(1,0)` maps to facing byte `64`; `(1,1)` maps to facing byte `96` and 16-bit `0x6000`. Source: `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`; AMCV traces.
- Exact stock YR skirmish AMCV starting facing remains unchecked in the trace bundle and must not be assumed for pixel/frame-perfect initial-turn acceptance. Source: `docs/research/traces/AMCV_TURNING_DIAGONAL_DRIVE_TRACE_20260527.md`.
- Exact retail DriveTrack point coordinates for the AMCV diagonal leg remain unchecked and need RE or a runtime oracle before claiming pixel-perfect positions. Source: `docs/research/traces/AMCV_TURNING_DIAGONAL_DRIVE_TRACE_20260527.md`.
- A* uses zone precheck and bounded retry semantics; current `zone_search.rs` has a 5-retry corridor approximation, but current command producers can still pass `zone_grid: None`. Source: `docs/research/PATHFINDING_ASTAR_GHIDRA_REPORT.md`; `src/sim/pathfinding/zone_search.rs`; `src/sim/movement/movement_commands.rs`.
- Post-A* smoothing/optimization is active in gamemd and validates shortcuts through cell-entry, cliff, and slope checks. Source: `docs/research/PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md`.
- Exact gamemd obstacle-detour cells for the AMCV blocker fixture remain unchecked; current Rust's selected detour must not become the oracle. Source: `docs/research/traces/AMCV_OBSTACLE_DETOUR_TRACE_20260527.md`.
- Runtime `Can_Enter_Cell` call shape must preserve direction/current-height/parent semantics; A* and runtime calls are not identical. Source: `docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`.
- Low bridge identity is valid tube index plus `LandType == 10`, and direction 8 is the tube path sentinel. Source: `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`.
- Direction-8 visible traversal must not consume zero-step auto shells; drive/walk producers divide by nonzero tube path length. Source: `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`.
- Unit TubeMovement payload uses active tube index, cursor, copied path buffer, target coord, and Z accumulator. Source: `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBEMOVEMENT_FINAL_Z_INTERPOLATION_GHIDRA_REPORT.md`.
- Unit TubeMovement advances by movement budget: partial movement if target distance exceeds budget, else one cursor increment and optional partial movement into the next segment. It does not move one tube path cell per tick. Source: `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBEMOVEMENT_FINAL_Z_INTERPOLATION_GHIDRA_REPORT.md`.
- Unit final tube exit snaps X/Y to `TubeClass+0x28`, keeps accumulated Z, clears active tube state, and uses ground object list semantics, not high-bridge landing. Source: `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBEMOVEMENT_FINAL_Z_INTERPOLATION_GHIDRA_REPORT.md`.
- If the unit tube final-exit ground object list is nonempty, the final branch does not clear active tube state; it enters the blocked-exit handling path instead. Source: `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBEMOVEMENT_FINAL_Z_INTERPOLATION_GHIDRA_REPORT.md`.
- `UnitClass::PerCellProcess` applies crush when the unit has `Crusher=yes` or veteran crusher ability, victim `CanCrushCheck` passes, distance squared is `<= 0x3FFF`, and the target is not limbo/falling. Source: `docs/research/CRUSH_SYSTEM_GHIDRA_REPORT.md`.
- `UnitClass::PerCellProcess` has a visible phase split: `entering != 0` scatters from the selected cell object list, while `entering == 0` performs the crush kill pass. Source: `docs/research/CRUSH_SYSTEM_GHIDRA_REPORT.md`.
- Crush sound plays at crusher coordinates before victim deletion, followed by mind-control cleanup, kill attribution, and removal. Source: `docs/research/CRUSH_SYSTEM_GHIDRA_REPORT.md`.
- Current Rust crush helper remains cell-based and sound emission currently uses victim coordinates; this must be corrected for the AMCV route-infantry scenario. Source: `src/sim/movement/bump_crush.rs`; `docs/research/traces/AMCV_CRUSH_ON_PATH_TRACE_20260527.md`.

## Design

### Components

Use the existing components as the home for Drive state:

- `NavigationState`: owner destination state. Keep `nav_com`, `nav_com_aux`, `suspended_nav_com`, `nav_queue`, and `pending_arrival_clear`.
- `NavTargetRef`: keep cell/object/building/entity variants. Full Drive parity needs more than cell targets because active gamemd resolves object/building dock or approach coordinates through the target.
- `DriveCoord`: integer world coordinate triplet.
- `DriveLocomotionRuntime`: authoritative Drive state for destination, head-to, path directions, turn state, active track metadata, speed fractions, residual budget, delay, and active tube payload.
- `DriveTubePayload`: grow or reinterpret this as the UnitClass tube state equivalent rather than leaving `LowBridgeTubeMovementState` as the authority for Drive units.
- `DriveTrackState`: remain the point-table executor. It should stay focused on raw track point stepping, residual interpolation, transform flags, cell jump, and chain readiness.
- `MovementTarget`: keep as a path cell/layer adapter and compatibility shell during migration. For Drive units, it should not own Drive speed, arrival, or body turn semantics.

### Interfaces / Contracts

`set_destination_internal_cell` and null destination helpers:

- Keep these as the owner NavCom entrypoints.
- Preserve the visible side effects: clear `nav_com_aux`, write or clear `nav_com`, seed Drive destination/head-to for non-null cell targets, and clear through shared null destination for arrival.
- Extend only with verified guards/timers as needed; do not invent broad queue semantics.

`process_drive_locomotion`:

- Add or extract a Drive-owned tick phase from `tick_movement_with_grids`.
- It should own normal Drive speed fraction, active track processing, path direction consumption, runtime cell entry, crush, tube entry, and arrival checks.
- It must refresh a non-cell NavCom target's dock/approach coordinate during Drive processing when the target moved, then call the Drive head-to setter with the fresh coordinate. A cell-only implementation is acceptable only for an explicitly narrowed cell-move milestone, not for the full Drive parity design.
- It may call existing movement helpers, but those helpers should take Drive state explicitly instead of deriving Drive behavior from generic `MovementTarget` fields.

`process_drive_track`:

- Use the existing `drive_track` point-table implementation, but drive it from `DriveLocomotionRuntime.residual_budget` and Drive current speed.
- Preserve no-fresh-speed retry behavior through an explicit parameter or helper, as current `advance_drive_track_retry_after_selection` already begins to do.
- Ensure residual interpolation and facing update semantics stay in the DriveTrack layer.

`runtime_can_enter_cell`:

- Build a shared cell-entry API that can express both A* and runtime call shapes.
- Do not collapse object-list layer, occupancy-bit layer, path layer, current height, and predecessor/current-cell interpretation into one layer value.
- Return numeric code categories in a way movement, pathfinding, scatter, crush, bridge, and tube producers can consume.

`apply_drive_crush`:

- Treat `regular_crusher` as the normal `Crusher=yes` capability for UnitClass-style crush processing.
- Keep `MovementZone` passability separate from crush capability.
- Preserve the `PerCellProcess` phase split: entering-cell handling scatters eligible occupants, and the actual crush kill pass runs only after the unit is fully in the cell.
- Apply the distance-squared gate and sound coordinate rule.
- Add hooks or placeholders for mind-control cleanup and kill attribution, marking missing downstream systems explicitly.

`begin_drive_tube_traversal` and `tick_unit_tube_movement`:

- Start from direction 8 only when the current cell has a valid explicit tube with nonzero path length.
- Store tube index, cursor, path target coord, copied path/cursor state, and Z accumulator.
- Advance by speed budget, not by one path cell per Rust tick.
- Finalize units using accumulated Z and ground-list occupancy semantics.
- If the final exit's ground object list is blocked, keep active tube state and execute the verified blocked-exit behavior instead of clearing tube state or forcing a bridge-style landing.

### Data Flow

1. A player move command resolves rules/entity data through `resolve_move_info`.
2. If the entity uses Drive locomotion, command setup writes owner NavCom and Drive destination/head-to through `set_destination_internal_cell`.
3. Existing pathfinding produces path cells/layers, then Drive command setup derives Drive path directions and seeds `DriveLocomotionRuntime.path`.
4. `MovementTarget` is attached as the transitional path adapter.
5. On each tick, Drive entities are handled by the Drive phase inside `tick_movement_with_grids`.
6. The Drive phase computes Drive target/current speed fraction, consumes DriveTrack or selects a new track/path direction, and runs runtime cell-entry checks.
7. Cell entry applies bridge state, occupancy, crush, blocked handling, or tube entry in gamemd-shaped order.
8. When Drive reaches the NavCom cell with an empty queue, it defers and then clears through the null-destination helper, matching the existing `pending_arrival_clear` scaffold.

### Error Handling

- Command failure leaves prior movement/destination state unchanged unless verified gamemd behavior says to stop or clear.
- Missing path data for a Drive entity should stop through the owner null-destination path, not silently continue vector motion.
- Invalid direction-8 tube input should block/stop according to verified producer behavior; zero-step shell traversal must not be used as a fallback.
- Any unsupported Drive state should be logged through deterministic debug events and covered by a failing parity test rather than hidden behind generic movement fallback.

### Testing Strategy

Immediate current-state pinning:

- `resolve_move_info_uses_stock_amcv_speed_without_deployable_multiplier`.
- `object_type_parses_regular_crusher_for_amcv_fixture`.
- `resolve_move_info_carries_regular_crusher_but_keeps_legacy_crush_inert` must be replaced or inverted in the same implementation change that wires `regular_crusher` into pathing/runtime crush. It must not remain as a passing test once AMCV `Crusher=yes` is active.
- Drive command writes NavCom and Drive destination/head-to while attaching only transitional `MovementTarget` execution state.
- Drive speed fraction tests for `Accelerates=true` and `Accelerates=false`.

Drive movement parity tests:

- Open-ground AMCV at `Speed=4` uses residual budget and does not fall back to generic full-speed vector stepping.
- Direction change selects/continues DriveTrack and uses retry/no-fresh-speed budget correctly.
- Arrival clears via pending NavCom/null-destination path, not direct target cleanup.
- Non-cell NavCom targets refresh their dock/approach coordinate while moving, or the scoped milestone explicitly excludes non-cell targets and keeps this test pending.
- Generic non-Drive movement behavior remains unchanged.

Pathfinding tests:

- Initial move command with a `ZoneGrid` uses zone precheck/retry surface.
- Same-zone failure and cross-zone failure behavior match the verified pathfinding contract where currently known.
- Obstacle-detour acceptance remains `UNCHECKED` until a gamemd oracle for exact cells is captured.

Crush tests:

- AMCV with `Crusher=yes` and `MovementZone=Normal` crushes centered enemy E1 on the path.
- Non-crusher normal vehicle does not crush.
- Entering-cell crush phase scatters or waits as gamemd does; the victim is not killed until the fully-in-cell crush phase.
- `0x3FFF` distance squared passes; `0x4000` fails.
- Crush sound event is emitted at crusher coordinates.
- Victim removal updates occupancy immediately and entity store deterministically.

Tube tests:

- Direction 8 requires valid explicit tube path data.
- Zero-step auto shell is not consumed as visible traversal.
- Unit TubeMovement uses speed-budget interpolation and at most one cursor increment per tick.
- Unit final tube exit keeps accumulated Z and ground-list occupancy.
- Unit final tube exit with a blocker in the ground object list does not clear active tube state.

End-to-end trace acceptance:

- Regenerate the five AMCV trace reports after implementation:
  - `AMCV_OPEN_GROUND_DRIVE_TRACE_20260527.md`
  - `AMCV_TURNING_DIAGONAL_DRIVE_TRACE_20260527.md`
  - `AMCV_OBSTACLE_DETOUR_TRACE_20260527.md`
  - `AMCV_BRIDGE_RAMP_TRAVERSAL_TRACE_20260527.md`
  - `AMCV_CRUSH_ON_PATH_TRACE_20260527.md`

## Architectural Decisions

- Follow the existing scaffold instead of creating a new locomotion framework.
- Keep `MovementTarget` as an adapter until Drive can be fully separated; do not remove it from Walk/Infantry, air, teleport, miner scripted moves, or direct movement paths as part of this design.
- Keep DriveTrack data and point stepping in `drive_track.rs`; put Drive owner process/state transitions in `drive_locomotion.rs` or small sibling modules.
- Keep low-bridge tube facts in map/terrain data and UnitClass-style tube movement in sim/movement.
- Treat `regular_crusher` as a gameplay capability, not as a MovementZone. Pathing and runtime entry must consult it explicitly.
- Do not claim exact AMCV initial facing, exact diagonal track points, obstacle-detour path cells, or final residual/tick equality until those unknowns are verified.

No intentional tech debt is introduced. The transitional use of `MovementTarget` is acknowledged existing debt and should shrink as Drive authority moves into DriveLocomotion state.

## Alternatives Considered

### Patch Generic Movement

Keep all current behavior in `MovementTarget` and add conditionals for AMCV speed, DriveTrack, crush, tube, and arrival. This is faster locally but keeps the wrong owner model. It makes every future parity report harder because Drive, Walk, direct scripted movement, and special locomotors remain tangled behind one generic path state.

Rejected for full parity.

### Full DriveLocomotion Rewrite

Build a new Drive tick pipeline and route Drive units completely away from `MovementTarget` in one pass. This is architecturally clean, but too broad for the current repo because the native-like scaffold already exists and many systems still depend on the current movement loop.

Deferred unless the existing scaffold proves structurally unable to preserve Drive parity.

### Approach A: Finish Existing Scaffold

Use current `NavigationState`, `DriveLocomotionRuntime`, DriveTrack, zone search, and movement helpers as the base, then move authority one mechanism at a time. This gives every trace detail a home while minimizing unrelated churn.

Chosen.
