# Full DriveLocomotion Parity Implementation Plan

> Execute this plan task-by-task. Do not skip the RE gates marked as blocking.

**Goal:** Move normal Drive-locomotor vehicle movement from generic `MovementTarget`
physics to a gamemd-shaped DriveLocomotion owner, closing the AMCV trace-swarm gaps
for command destination setup, speed budget, DriveTrack consumption, runtime
`Can_Enter_Cell`, crush, low-bridge/tube traversal, and arrival teardown.

**Design Doc:** [docs/plans/2026-05-27-full-drivelocomotion-parity-design.md](2026-05-27-full-drivelocomotion-parity-design.md)

**Primary source traces:**

- `docs/research/traces/AMCV_OPEN_GROUND_DRIVE_TRACE_20260527.md`
- `docs/research/traces/AMCV_TURNING_DIAGONAL_DRIVE_TRACE_20260527.md`
- `docs/research/traces/AMCV_OBSTACLE_DETOUR_TRACE_20260527.md`
- `docs/research/traces/AMCV_BRIDGE_RAMP_TRAVERSAL_TRACE_20260527.md`
- `docs/research/traces/AMCV_CRUSH_ON_PATH_TRACE_20260527.md`

---

## Grounding Summary

- **Active YR data:** AMCV has `Speed=4`, `ROT=5`, `Crusher=yes`, Drive locomotor
  `{4A582741-9839-11d1-B709-00A024DDAFD1}`, and `MovementZone=Normal` in
  `ini/rulesmd.ini:6969-7000`.
- **Current Rust drift:** `src/sim/world/world_commands.rs` multiplies deployable
  unit speed by 3, `MovementTarget.current_speed` starts at full speed, generic
  vehicle stepping can bypass DriveTrack, normal `Crusher=` is not parsed, and
  arrival clears `MovementTarget` instead of going through the Drive/NavCom
  destination lifecycle.
- **Verified binary owners:** `FootClass::Set_Destination_Internal`,
  `DriveLocomotionClass::Process`, `DriveLocomotionClass::Process_Movement`,
  `DriveLocomotionClass::Process_Drive_Track`, `UnitClass::Can_Enter_Cell`,
  `UnitClass::PerCellProcess`, and `UnitClass::TubeMovement`.
- **Architecture decision:** Keep `MovementTarget` as transitional path/order data
  for compatibility, but stop using it as the direct physics owner for normal Drive
  units once the DriveLocomotion path is active.

## Blocking RE Gates

These do not block every task, but they block parity-complete implementation and
specific task groups.

| Gate | Blocks | Required output |
|---|---|---|
| Exact stock standard-YR AMCV skirmish starting facing | Frame-perfect diagonal turn assertions | Research note or trace confirming the starting facing byte. |
| Exact retail DriveTrack point list for straight and diagonal AMCV legs | Replacing all straight/diagonal stepping with parity claim | Verified point list or proof current Rust tables match the active selected tracks. |
| Exact AMCV body-facing timeline, first movement tick, arrival tick, and residual bytes | Pixel/frame-perfect acceptance for open-ground and diagonal traces | Trace/report with expected per-frame sequence. |
| Exact gamemd obstacle-detour waypoint cells and post-smoothing direction array for `(40,40)->(48,40)` with blocker `(44,40)` | Using any detour path as an oracle | Trace/report with raw and smoothed gamemd path. |
| Rust occupancy insertion/list order vs gamemd `Object+0x30` list order | Final `Can_Enter_Cell` object-list parity | Focused audit or explicit UNKNOWN carried into tests. |

Implementation may start with data, state, command, and scaffold tasks, but any
acceptance test that depends on an unresolved gate must assert `UNKNOWN` or stay
disabled/pending until the gate is resolved.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/rules/object_type.rs` | Parse `Crusher=` and `Accelerates=` with verified defaults. |
| Modify | `src/sim/game_entity.rs` | Store regular crusher capability and DriveLocomotion/NavCom state. |
| Modify | `src/sim/components.rs` or new `src/sim/movement/drive_locomotion.rs` | Define DriveLocomotion/NavCom/tube state types. |
| Modify | `src/sim/world/world_commands.rs` | Remove AMCV speed multiplier, compute crusher from `Crusher=`, route Drive moves to Drive destination entrypoint. |
| Modify | `src/sim/movement/movement_commands.rs` | Add `set_drive_destination` and separate path/order storage from Drive physics setup. |
| Modify | `src/sim/movement/movement_tick.rs` | Dispatch active Drive entities through DriveLocomotion before generic movement; avoid generic finalization for Drive arrivals. |
| Modify | `src/sim/movement/drive_track.rs` | Ensure normal Drive path consumes tracks and residual budget as owner state. |
| Modify | `src/sim/pathfinding/cell_entry.rs` | Numeric `Can_Enter_Cell` code model and runtime/A* call-shape support. |
| Modify | `src/sim/movement/movement_occupancy.rs` | Use runtime `Can_Enter_Cell` call shape and object-list/occupancy split. |
| Modify | `src/sim/movement/tube_movement.rs` | Replace simplified low-bridge tube traversal with unit tube payload/cadence. |
| Modify | `src/sim/world/world_hash.rs` | Hash new DriveLocomotion/NavCom/tube fields. |
| Modify | Movement/rules/pathfinding tests | Add focused regression and acceptance tests. |

## Interface Changes

- Add `ObjectType::crusher: bool`, parsed from `Crusher=`, default `false`.
- Add `ObjectType::accelerates: bool`, parsed from `Accelerates=`, default `true`.
- Add `GameEntity::regular_crusher: bool` or equivalent, distinct from
  `omni_crusher` and `MovementZone`.
- Add DriveLocomotion state with at least:
  - destination coord
  - head-to coord
  - Drive path direction queue/cursor or equivalent path-order owner
  - turn target, 16-bit facing/rate-timer owner state, and first-movement gate where verified
  - track index / point index / on-track / reversed flags
  - target speed fraction
  - residual budget
  - Drive delay where verified
  - NavCom target reference
  - active unit tube payload
- Add a call-shape-aware `runtime_can_enter_cell` API that keeps A* explicit-parent
  calls separate from runtime Drive null-parent calls.
- Add `set_drive_destination` for Drive units. Existing movement command entrypoints
  may continue for non-Drive locomotors.

## Sim Checklist

- [ ] No new dependency from `sim/` to render/ui/sidebar/audio/net.
- [ ] No f32/f64 in simulation state or formulas unless the verified binary field
      is explicitly modeled and deterministically quantized; prefer fixed/integer
      for Rust state.
- [ ] All new state participates in serialization and `world_hash`.
- [ ] Entity iteration remains deterministic.
- [ ] Existing non-Drive locomotor paths are preserved unless a task explicitly
      migrates them.
- [ ] Every parity-sensitive test cites a trace or Ghidra report, or marks the
      assertion `UNKNOWN - needs RE`.

## Risk Areas

- **Owner split bugs:** During transition, a Drive entity could have both active
  DriveLocomotion state and generic `MovementTarget` physics. Tests must assert
  generic physics is bypassed for active Drive.
- **NavCom scope creep:** Full Drive parity needs object/building NavCom targets.
  A cell-only implementation must be explicitly scoped as partial.
- **Speed fraction drift:** `Accelerates=false` assigns the computed Drive target
  fraction, not raw top speed or unconditional `1.0`.
- **Can_Enter_Cell flattening:** A* explicit parent and runtime null parent are not
  interchangeable.
- **Crush ordering:** Crush sound, kill attribution, mind-control cleanup, and victim
  deletion timing are side effects, not cosmetic extras.
- **Early crusher activation:** Parsing and storing `Crusher=yes` is safe early, but
  feeding it into the current cell-based Rust crush kill path is not. Regular
  crusher behavior must not become live until the Drive `PerCellProcess` path owns
  the distance gate, sound anchor, kill attribution, mind-control cleanup hook, and
  deletion order.
- **Tube shell misuse:** Auto low-bridge same-cell zero-step tubes exist, but visible
  direction-8 traversal must not consume them as path movement.

---

## Tasks

### Task 1: Parse `Crusher=` and `Accelerates=`

**Status 2026-05-27:** Done. `ObjectType` now parses both fields with tests.

**Why:** AMCV has `Crusher=yes` while `MovementZone=Normal`; Grizzly-style
`Accelerates=false` is a distinct Drive flag. Both are active gamemd data and cannot
be inferred from existing Rust fields.

**Files:**

- `src/rules/object_type.rs`
- `src/rules/object_type` tests in the same file or existing test module

**Steps:**

1. Add `crusher: bool` to `ObjectType`.
2. Parse `Crusher=` with default `false`.
3. Add `accelerates: bool` to `ObjectType`.
4. Parse `Accelerates=` with default `true`.
5. Add tests:
   - `object_type_parses_regular_crusher_for_amcv_fixture`
   - `object_type_crusher_defaults_false`
   - `object_type_parses_accelerates_false`
   - `object_type_accelerates_defaults_true`

**Verification:**

- `cargo test object_type_parses_regular_crusher`
- `cargo test object_type_accelerates`

### Task 2: Propagate regular crusher capability as inert data

**Status 2026-05-27:** Done. Regular `Crusher=` is stored and hashed, but remains
inert for live legacy crush until Task 11 owns the Drive `PerCellProcess` path.

**Why:** Current Rust derives `mover_is_crusher` from `omni_crusher` or crusher
movement zones. That makes AMCV non-crushing despite `Crusher=yes`, but simply
turning on the current Rust crush path would create a different drift because that
path is cell-based and lacks gamemd `PerCellProcess` side effects.

**Files:**

- `src/sim/game_entity.rs`
- `src/sim/world/world_spawn.rs`
- `src/sim/world/world_commands.rs`
- `src/sim/movement/movement_step.rs`
- `src/sim/pathfinding/cell_entry.rs`
- `src/sim/movement/bump_crush.rs`

**Steps:**

1. Add entity-level regular crusher capability, distinct from `omni_crusher`.
2. Populate it from `ObjectType::crusher` when spawning units.
3. Add explicit fields to move/path/crush snapshots that can carry regular crusher
   separately from `MovementZone`.
4. Do not let regular crusher feed the existing legacy cell-based runtime crush kill
   path in this task.
5. Keep live `MoveInfo::mover_is_crusher` behavior unchanged unless the same patch
   also implements Task 11's Drive `PerCellProcess` path and proves the two surfaces
   stay in lockstep.
6. Preserve existing `OmniCrusher` behavior.

**Tests:**

- AMCV with `MovementZone=Normal` reports crusher capability.
- A non-`Crusher=yes` `MovementZone=Normal` vehicle does not.
- Regular crusher data is present in snapshots but does not activate legacy runtime
  crush before Task 11.
- Existing omni-crusher tests still pass.

### Task 3: Remove deployable-unit speed multiplier

**Status 2026-05-27:** Done. AMCV move info now uses stock `Speed=4` without the
temporary deployable-unit multiplier.

**Why:** AMCV must use stock `Speed=4`; the current deployable 3x multiplier is a
trace-proven player-visible drift.

**Files:**

- `src/sim/world/world_commands.rs`
- Movement command tests

**Steps:**

1. Remove the `deploys_into.is_some() { 3 }` speed multiplier.
2. Keep locomotor `speed_multiplier` semantics if they are separately verified.
3. Add an AMCV move-info test proving effective speed comes from `Speed=4`.

**Verification:**

- Focused world command/movement tests.
- Existing MCV deploy tests should still pass.

### Task 4: Introduce DriveLocomotion/NavCom state types

**Status 2026-05-27:** Done. Drive/NavCom/tube runtime state exists, serializes,
and participates in state hashing.

**Why:** Drive needs state equivalent to destination/head-to coords, active track,
target speed fraction, residual budget, NavCom, and active tube state.

**Files:**

- New `src/sim/movement/drive_locomotion.rs` or nearby established module
- `src/sim/movement/mod.rs`
- `src/sim/game_entity.rs`
- `src/sim/world/world_hash.rs`

**Steps:**

1. Define `DriveLocomotionState`.
2. Define `NavComTarget` with at least cell target and object/building target variants.
3. Define a Drive path-order owner: direction queue/cursor or an equivalent
   representation that can feed `Process_Movement`/DriveTrack without making
   generic `MovementTarget` physics authoritative.
4. Define Drive turn state for the verified 16-bit target-facing/RateTimer path.
5. Define `DriveTubeState` or shared unit tube payload equivalent to `+0x684/+0x685`.
6. Add optional DriveLocomotion state to `GameEntity`.
7. Include new state in serialization and `world_hash`.
8. Add state-construction tests and hash-delta tests.

**Notes:**

- A cell-only `NavComTarget` is not sufficient for this full plan.
- If object/building target references cannot be implemented immediately, split the
  implementation and explicitly mark the result partial.

### Task 5: Add `set_drive_destination`

**Status 2026-05-27:** Done for normal cell-target Drive moves. Move commands now
write owner NavCom, Drive destination/head-to, Drive path directions, and initial
Drive-owned turn target; zone-grid routing is threaded where simulation context is
available. Object/building dock/approach targets remain future work under the later
NavCom/docking tasks.

**Why:** Move commands must route through a NavCom + `Head_To_Coord`-shaped lifecycle
instead of directly making `MovementTarget` the physics owner.

**Files:**

- `src/sim/movement/movement_commands.rs`
- `src/sim/world/world_commands.rs`
- `src/sim/movement/drive_locomotion.rs`
- `src/sim/movement/movement_path.rs`
- `src/sim/pathfinding/zone_search.rs`

**Steps:**

1. Add `set_drive_destination` for active Drive units.
2. Clear NavCom_Aux equivalent on every destination set.
3. Write or clear NavCom.
4. Preserve verified non-null early-return gates, or mark unverified gates as TODO
   with tests disabled until RE.
5. Write blocked/path retry timers required by the NavCom report.
6. For non-null targets, compute `head_to_coord` from cell center or target
   dock/approach coord provider.
7. Route Drive path creation through the hierarchy/zone-precheck-capable path when
   `zone_grid` exists; a `zone_grid: None` player move is an explicit partial
   fallback, not the parity path.
8. Preserve default five-attempt hierarchy retry where verified.
9. For null targets, route through shared stop/clear-navigation behavior.
10. Stop setting generic `facing_target` as the primary Drive turn owner.

**Tests:**

- Cell move writes cell NavCom and Drive head-to coord.
- Object/building move calls target dock/approach provider.
- Drive player move with a zone grid takes the hierarchy precheck path.
- Null destination clears NavCom/NavCom_Aux and Drive navigation state.
- Generic `MovementTarget` physics is not activated as the Drive owner.

### Task 6: Add computed Drive target speed fraction owner

**Status 2026-05-27:** Done as a scoped scaffold. Drive now stores the computed
target speed fraction from currently modeled terrain/slope/crowd modifiers, and
`Accelerates=false` copies that target directly to the Drive current fraction
without mutating raw `Speed=`. The verified `Accelerates=true` ramp cadence remains
future work under the Drive process tasks.

**Why:** `Accelerates=false` consumes the computed Drive target fraction, not raw
speed or unconditional full speed.

**Files:**

- `src/sim/movement/drive_locomotion.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/pathfinding/terrain_speed.rs` or existing speed modifier surfaces

**Steps:**

1. Add `compute_drive_target_speed_fraction`.
2. Include currently verified speed modifiers available in Rust: terrain speed,
   slope/bridge context where implemented, health/group modifiers where already
   modeled.
3. Store result in DriveLocomotion state.
4. If `accelerates=false`, assign computed target fraction directly before
   `GetCurrentSpeed` equivalent.
5. If `accelerates=true`, use the verified ramp branch where supported; mark any
   missing constants or flags as TODO with evidence.

**Tests:**

- `Accelerates=false` with a non-1.0 modifier assigns that modified fraction.
- `Accelerates=false` does not overwrite raw `Speed=`.
- Default `Accelerates=true` enters ramp branch where acceleration data is present.

### Task 7A: Implement Drive turn/RateTimer ownership

**Why:** The diagonal AMCV trace fails before the first moving step because Rust
pre-rotates vehicles with generic 8-bit `facing_target`, while gamemd Drive uses a
DriveLocomotion/RateTimer-shaped 16-bit facing target and gates first movement
through that owner.

**Files:**

- `src/sim/movement/drive_locomotion.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/components.rs` or `src/sim/game_entity.rs`
- Facing helper tests

**Steps:**

1. Add Drive-owned turn state for target direction/facing, 16-bit target facing,
   rate/timer progress, and whether movement may consume the next DriveTrack point.
2. Route initial Drive direction selection through this owner instead of generic
   `MovementTarget.facing_target`.
3. Preserve existing generic facing behavior for non-Drive locomotors.
4. Make first movement wait or proceed according to the verified Drive turn branch,
   not according to Rust's current 8-bit pre-rotation helper.
5. Let consumed DriveTrack points update facing through the DriveTrack point heading
   path once Task 9 is active.
6. Keep exact frame-count assertions disabled until the starting-facing and timeline
   RE gates are resolved.

**Tests:**

- Drive move to an SE cell writes a 16-bit target equivalent to `0x6000`.
- Drive entities do not use generic `MovementTarget.facing_target` as the primary
  turn owner.
- Non-Drive vehicle/infantry facing behavior remains on the existing path.
- Exact first-movement tick remains `UNKNOWN - needs RE` until the stock-facing and
  frame-timeline gates are closed.

### Task 7: Implement Drive `process` skeleton and dispatch

**Why:** DriveLocomotion must become the per-tick owner before detailed track, crush,
and tube work can be validated.

**Files:**

- `src/sim/movement/drive_locomotion.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/movement/movement_step.rs`

**Steps:**

1. Dispatch Drive entities with active DriveLocomotion state before generic
   `MovementTarget` movement.
2. Add `process_drive_locomotion` with branches for:
   - active track
   - no active track but has path/NavCom
   - active tube state
   - arrived/stopped
3. Return explicit events for movement, blocked, tube-start, crush, arrival, and stop.
4. Ensure generic `finalize_finished_entities` does not clear active Drive arrivals.
5. Keep non-Drive entities on existing movement path.

**Tests:**

- Drive entity with active Drive state bypasses generic lepton stepping.
- Non-Drive entity still uses existing movement.
- Arrival uses Drive null-destination path, not generic finalization.

### Task 8: Implement call-shape-aware runtime `Can_Enter_Cell`

**Why:** Runtime Drive checks pass parent/current `0`; A* passes explicit parent.
Collapsing them causes bridge and obstacle drift.

**Files:**

- `src/sim/pathfinding/cell_entry.rs`
- `src/sim/pathfinding/core.rs`
- `src/sim/movement/movement_occupancy.rs`
- `src/sim/movement/drive_locomotion.rs`

**Steps:**

1. Introduce numeric return-code model 0-7 with corrected labels.
2. Add call-shape enum for A*, runtime Drive, and future direction `-1` candidate calls.
3. Preserve explicit parent for A*.
4. Preserve null parent for runtime Drive and reconstruct predecessor via
   `(direction - 4) & 7`.
5. Compute runtime current height from current effective height, not target layer.
6. Preserve object-list layer and occupancy-bit layer independently.
7. Wire Drive processing to use this API.

**Tests:**

- A* explicit parent and runtime null parent differ on a bridge-edge fixture.
- Runtime current height ignores target layer.
- Return-code taxonomy matches verified numeric table.

### Task 9: Implement DriveTrack-owned movement budget and residual

**Why:** Normal Drive movement must consume DriveTrack points with 7-budget chunks,
residual carry, and residual interpolation.

**Files:**

- `src/sim/movement/drive_locomotion.rs`
- `src/sim/movement/drive_track.rs`
- `src/sim/movement/movement_step.rs`

**Steps:**

1. Route Drive path directions through DriveTrack selection for normal Drive units.
2. Consume budget as `(retry ? 0 : current_speed_budget) + residual`.
3. Spend 7 budget units per consumed track point.
4. Store residual.
5. Apply residual interpolation without per-point facing update.
6. Apply per-consumed-point facing update.
7. Clear residual when verified no-track early-exit branch requires it.

**Blocking gates:**

- Exact point-list and frame-timeline gates block parity-complete assertions.

**Tests:**

- Budget consumes one point at 7 and stores remainder below 7.
- Retry call adds no new speed.
- Residual interpolation does not update facing.
- Consumed point updates facing.

### Task 10: Implement Drive arrival through `Set_Destination(NULL, 1)`

**Why:** Arrival side effects must use the public destination lifecycle, not direct
`MovementTarget` cleanup.

**Files:**

- `src/sim/movement/drive_locomotion.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/world/world_hash.rs`

**Steps:**

1. Detect current cell equals NavCom cell.
2. If no queued path/NavCom waypoint remains, call the Rust null-destination path.
3. Clear NavCom/NavCom_Aux.
4. Clear Drive destination/head-to/track state according to verified arrival branch.
5. Preserve destination object visibility where accepted-cell reports require it
   for miner/refinery flow, if implementing object targets in the same phase.
6. Emit/record arrival debug events only after state writes match gamemd order.

**Tests:**

- Cell arrival clears via shared null-destination path.
- Generic finalization is not used for Drive.
- Accepted-cell stopped state remains observable for miner/refinery tests.

### Task 11: Implement regular Drive crush application

**Why:** AMCV must crush a centered enemy E1 on path through `Crusher=yes`, with
gamemd side effects.

**Files:**

- `src/sim/movement/drive_locomotion.rs`
- `src/sim/movement/bump_crush.rs`
- `src/sim/movement/movement_occupancy.rs`
- `src/sim/world` sound/event surfaces as needed

**Steps:**

1. Add `apply_drive_per_cell_process`.
2. Gate crusher by regular `Crusher=yes` or verified veteran crusher ability when
   implemented.
3. Activate regular crusher in live Drive pathfinding/runtime only together with this
   Drive `PerCellProcess` path, so path planning and runtime cell entry do not drift.
4. Use victim `CanCrushCheck`.
5. Apply distance squared gate `<= 0x3FFF`.
6. Play victim `CrushSound` at crusher coordinates before deletion.
7. Add hooks/placeholders for mind-control cleanup and kill attribution.
8. Remove victim in gamemd order.

**Tests:**

- AMCV crushes centered enemy E1.
- Centered victim distance `0` passes.
- Distance `0x4000` fails while `0x3FFF` passes.
- Friendly/non-crushable/deployed cases follow verified gates.
- Crush sound anchor is crusher coord.
- Enabling regular crusher cannot route through the legacy cell-based kill path.

### Task 12: Replace low-bridge tube traversal payload/cadence

**Why:** Current Rust tube movement advances one tube path cell per tick and can treat
low-bridge exit like high-bridge occupancy. gamemd uses active tube state and
speed-budget interpolation.

**Files:**

- `src/sim/movement/tube_movement.rs`
- `src/sim/movement/drive_locomotion.rs`
- `src/map/tube_facts.rs`
- `src/map/resolved_terrain.rs`

**Steps:**

1. Preserve same-cell zero-step auto tubes for predicate/zone use.
2. Prevent visible direction-8 traversal through zero-step shell tubes.
3. On valid direction-8 Drive producer, store active tube index/cursor/payload.
4. Copy or reference tube path buffer according to verified payload needs.
5. Seed destination world coord from `Tube+0x28`.
6. Seed Z accumulator from signed division over `Tube+0x1C0`.
7. Tick tube movement by speed budget:
   - partial move if target distance exceeds budget
   - if target reached, increment cursor once
   - optionally spend leftover partially into next segment
   - do not loop through arbitrary tube cells
8. On unit exit, snap X/Y to `Tube+0x28`, preserve accumulated Z, clear active tube,
   and use ground-list final occupancy.

**Tests:**

- Direction 8 requires non-null tube and nonzero endpoint.
- Zero-step auto shell does not start visible traversal.
- Fast unit increments at most one cursor per tick.
- Final exit uses ground-list occupancy and accumulated Z.

### Task 13: Audit Drive path hierarchy and detour exactness

**Why:** Task 5 must route Drive player moves through the hierarchy/zone-precheck
path, but the obstacle trace still has an unresolved exact waypoint/smoothing oracle.
This task closes or explicitly carries that remaining exactness gap.

**Files:**

- `src/sim/movement/movement_commands.rs`
- `src/sim/movement/movement_path.rs`
- `src/sim/pathfinding/zone_search.rs`
- `src/sim/pathfinding/core.rs`

**Steps:**

1. Confirm Drive player move pathfinding has access to `zone_grid` in every normal
   command surface.
2. Confirm missing `zone_grid` remains an explicit partial/fallback path, not a
   silent parity path.
3. Verify the default five-attempt hierarchy retry path is reached where the
   research says it should be.
4. Reconcile Drive direction queue output with post-A* smoothing and straight-segment
   optimization.
5. Do not assert exact obstacle detour cells until the RE gate is resolved.

**Tests:**

- Drive move with zone grid performs precheck.
- Missing zone grid is explicit partial/fallback, not silent parity path.
- Exact obstacle fixture remains pending until gamemd path oracle exists.

### Task 14: Integration acceptance tests for AMCV trace set

**Why:** The implementation must close player-visible trace failures without hiding
unknown exact facts.

**Files:**

- Movement integration tests
- Trace docs if producing post-fix verification reports

**Steps:**

1. Add open-ground AMCV test for speed budget, Drive owner, and arrival lifecycle.
2. Add diagonal AMCV test for DriveTrack ownership, with exact frame assertions gated
   on RE.
3. Add obstacle-detour test for hierarchy/precheck, with exact path gated on RE.
4. Add low-bridge/tube traversal test for direction-8 payload/cadence.
5. Add crush-on-path test for AMCV vs E1.
6. Re-run or regenerate the five AMCV traces as post-implementation acceptance docs.

**Verification:**

- Focused movement/rules/pathfinding tests.
- Final `cargo check -q` after focused tests pass.

---

## Suggested Execution Order

1. Tasks 1-3: data plumbing and the obvious AMCV speed drift fix. Task 2 must keep
   regular crusher runtime behavior inert until Task 11 is implemented in the same
   patch or a later patch.
2. Task 4: state scaffolding and hashing.
3. Task 5: command/NavCom lifecycle plus hierarchy/zone-precheck routing for Drive
   player moves.
4. Task 6: target speed fraction.
5. Task 7A: Drive turn/RateTimer ownership.
6. Tasks 7-10: Drive owner process, `Can_Enter_Cell`, DriveTrack budget, arrival.
7. Task 11: crush, including activation of regular crusher behavior.
8. Task 12: low-bridge tube movement.
9. Task 13: path hierarchy/detour exactness audit.
10. Task 14: trace acceptance and remaining RE-gated exactness.

## Do Not Do

- Do not call the result full parity while any blocking RE gate remains unresolved.
- Do not add AMCV-only movement special cases.
- Do not keep generic `MovementTarget` vector stepping as the normal Drive physics path.
- Do not model `Crusher=yes` through MovementZone.
- Do not activate `Crusher=yes` through the legacy cell-based Rust crush kill path.
- Do not model `Accelerates=false` as raw top speed or unconditional `1.0`.
- Do not collapse A* and runtime `Can_Enter_Cell` parent semantics.
- Do not consume zero-step low-bridge shell tubes as visible traversal.
