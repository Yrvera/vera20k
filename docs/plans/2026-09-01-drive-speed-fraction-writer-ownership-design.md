# Drive Speed-Fraction Writer Ownership Design

## Goal

Remove the non-native Drive move-command speed-fraction mutation so ordinary
path installation preserves existing target/current fraction state and the
scheduled Drive movement rung remains the sole ordinary producer, closing the
first separable prerequisite for exact crate Speed semantics.

## Status and autonomous approval

**Status:** APPROVED after design-review revision for proportional planning
under the active autonomous Phase 14 goal.

The first design review returned `REVISE` for two issues now corrected here:
the ForceTrack residual no longer misstates target-only native behavior as a
full-fraction write, and the test plan now proves the production
`Simulation::apply_command` route before exercising the scheduled producer.

Adversarial approval questions:

- **Why approve this separately from the full crate runtime?** Active-retail
  caller evidence proves path installation itself does not write either
  fraction, while this Rust write affects every ordinary Drive move. Removing
  it closes one complete writer-ownership mechanism and avoids building crate
  Speed on a false mutation boundary.
- **What could still make ordinary skirmish feel wrong?** The broader
  `GetCurrentSpeed` staging, crate multiplier, forced-track, crush, second brake
  band, and RawTrack work remains open. This design does not claim those are
  solved; it preserves all existing scheduled-rung behavior and changes only
  the disproved command-time writer.
- **What could create expensive later rework?** Introducing a new speed API or
  moving target calculation into command handling would duplicate the scheduled
  producer. The chosen deletion-plus-contract-test approach does neither.

Approval decision: the boundary is active-YR verified, production-routed,
small, reversible, and dependency-coherent. No undiscoverable user choice
remains.

## Architecture Context

The player/team/AI command path reaches
`Simulation::apply_command_with_overlays` and then
`movement::issue_move_command_with_layered` through
`src/sim/world/world_commands.rs:600-725`. The command function plans the path,
installs `MovementTarget`, NavCom destination, Drive path replay, initial turn,
and optional first DriveTrack at
`src/sim/movement/movement_commands.rs:618-726`.

Drive-local speed state lives in
`DriveLocomotionRuntime::{target_speed_fraction,current_speed_fraction}` at
`src/sim/components.rs:440-508`; both default to zero and are serialized and
hashed as part of the entity. The ordinary live-object movement visit is owned
by `Simulation::advance_master_frame` at `src/sim/world/mod.rs:6530-6608`.
During that visit, `src/sim/movement/movement_tick.rs:1969-2058` computes the
terrain/health target fraction, calls
`drive_locomotion::update_drive_speed_fraction`, derives current movement
speed, and caches the owner-speed projection before DriveTrack consumption.

The current extra command-time call at
`src/sim/movement/movement_commands.rs:717-726` supplies target one and zero
acceleration/braking inputs. For `Accelerates=false` it writes both fraction
slots immediately; for `Accelerates=true` it at least writes the target slot.
Active retail proves no corresponding Move-command path-install writer exists.
The normal writers live in Drive `Process_Movement @ 0x004B2630` and
`Process_Drive_Track @ 0x004B0F20`; separate Stop/ForceTrack/lifecycle writers
remain separate mechanisms.

This slice follows the existing architecture: command code installs intent and
path state; the deterministic per-object movement visit owns speed-state
mutation and consumption. It introduces no new module or cross-layer
dependency.

## Impact Analysis

### Direct changes

- `src/sim/movement/movement_commands.rs`
  - remove the command-time `update_drive_speed_fraction` call;
  - retain destination, path, turn, track, and occupation installation order;
  - add the nearest provenance comment stating that path installation preserves
    the two fraction slots.
- `src/sim/movement/movement_tests.rs`
  - extend direct command-boundary coverage with sentinel fractions;
  - verify a mid-curve reissue preserves already-live fraction state.
- `src/sim/world/world_tests.rs`
  - issue `Command::Move` through `Simulation::apply_command`, prove the
    production dispatcher preserves sentinel fractions, then run the scheduled
    movement visit and prove the existing producer changes them.

### Dependents and blast radius

Every caller of `issue_move_command_with_layered` benefits from the same
ownership fix: player Move, AI/team orders, miner/refinery navigation, factory
exit movement, scatter, and scripted movement. No caller receives a new return
type or parameter. Ship is unaffected because the offending call is inside the
Drive-only branch and Ship already defers fraction mutation to its movement
rung.

The change affects deterministic state between command application and the
entity's scheduled movement visit. That difference is intentional and native:
reissue observers and state hashes must see the previous qwords until the
Drive visit. Snapshot schema and hash layout do not change; only values at that
time boundary change.

No RNG, INI, asset, rendering, audio, network protocol, or persistence format is
added. The risk is behavioral fallout in tests that accidentally relied on the
non-native eager write; those tests must be corrected only when their assertion
is about this ownership boundary, not massaged broadly.

## Chosen Approach

Delete only the disproved command-time fraction update and pin the ownership
contract with focused command-to-first-tick tests. Keep the existing scheduled
target computation and update function unchanged.

This is preferred because it exactly matches the verified native negative
claim, preserves Rust's current responsible owner, minimizes the state window
changed, and creates no adapter that the full crate Speed implementation would
later have to remove.

## Player-Experience Detail Ledger

- **MILESTONE-BLOCKING — command-time preservation.** A normal or reissued
  Drive path may update destination/path/turn/track but must not write either
  target or current speed-fraction slot. [doc:
  `PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md` lines 1537-1546;
  GHIDRA `Process_Movement @ 0x004B2630`, `Process_Drive_Track @ 0x004B0F20`]
- **MILESTONE-BLOCKING — scheduled first mutation.** The entity's live-object
  movement visit computes target from the current cell/next cell, terrain,
  locomotor speed type, and ConditionYellow state, then applies snap/ramp before
  speed consumption. The command must not pre-empt that live read. [Rust:
  `movement_tick.rs:1969-2058`; doc: Phase 3 report lines 826-968]
- **COMPOUNDING — `Accelerates=false`.** Native snaps current fraction to the
  scheduled target in `Process_Drive_Track`; Rust currently snaps eagerly to
  one at command time. Terrain and health can make the scheduled target non-one,
  so preservation must be tested with non-default sentinels and a later target
  change. [doc: Phase 3 report lines 817-968]
- **COMPOUNDING — `Accelerates=true`.** Constructor/rest state begins at zero;
  the scheduled rung ramps from the prior current value. A command-time target
  one is still a state/order drift even when current remains zero. [doc:
  `FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md` lines 306-329 and
  403-430]
- **COMPOUNDING — mid-curve reissue.** `TechnoClass::Set_Destination` preserves
  an in-flight curve and its speed state; a new path takes over at the committed
  head. The existing position/track preservation tests must also pin fraction
  preservation. [Rust: `movement_tests.rs:1623-1700`; GHIDRA
  `TechnoClass::Set_Destination @ 0x00741970`]
- **MILESTONE-BLOCKING — crate Speed readiness.** `Foot+0x580` is an independent
  persistent crate multiplier consumed by `GetCurrentSpeed`; it must not be
  conflated with target/current fraction or a command-time scalar. This slice
  only removes the false writer and does not claim the multiplier exists.
  [doc: `FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md` lines 232-252]
- **EXACTIFICATION-RESIDUAL — separate fraction writers.** Native ForceTrack
  writes only the locomotor target qword to exact one and does not write owner
  current fraction; Tube and Stop/lifecycle paths have their own separately
  ordered transitions. Current Rust's ForceTrack writes to current fraction and
  cached owner speed as well, and Tube contains additional target/current
  writes, so those paths are known drift rather than verified behavior to pin.
  This branch neither edits nor claims them: its diff/search audit proves only
  that the ordinary path-install deletion is scoped. Trigger: Tank Bunker,
  stop, and tube lifecycle. Frequency: common for stop, narrower for
  forced/tube paths. Player effect: movement start/stop feel. Downstream risk:
  high, so the discrepancies remain open for their own dependency-coherent
  correction rather than being preserved by new assertions. [Rust:
  `movement/mod.rs:159-173`, `navcom.rs:245-305`, `tube_movement.rs`; doc:
  Phase 3 report lines 930-955]
- **EXACTIFICATION-RESIDUAL — full GetCurrentSpeed mechanism.** House
  SpeedUnitsMult, crate multiplier, FASTER/VeteranSpeed, current fraction, x87
  conversion boundaries, and CTF half-speed remain a later coherent production
  mechanism. Trigger: ordinary movement plus crate/veterancy/house modifiers.
  Frequency: ordinary. Player effect and deterministic risk: high. This is not
  silently downgraded; it remains open in the crate disparity report. [doc:
  `FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md`]
- **UNKNOWN-RISK — none introduced.** The selected negative writer claim has
  exhaustive active-retail producer/caller evidence. No TS-only behavior is
  used; the Unit/Drive path is ordinary active YR.

## Design

### Components

No new component is introduced. `DriveLocomotionRuntime` remains the state
owner. `issue_move_command_with_layered` remains the path/destination installer.
`movement_tick` remains the ordinary target/current fraction producer and
consumer coordinator.

### Interfaces / Contracts

The existing command API is unchanged. Its Drive postcondition becomes
explicit:

```text
successful Drive path installation
  may mutate destination, head, path, turn, track, and occupation
  must preserve target_speed_fraction
  must preserve current_speed_fraction
  must preserve owner_current_speed until the scheduled movement visit
```

The scheduled movement visit retains its existing contract:

```text
read live terrain/health/path state
compute target fraction
apply native snap/ramp to target/current slots
derive current speed and owner speed
consume DriveTrack budget
```

`owner_current_speed` is not currently written by the offending command call,
but the test will pin it as part of the same negative boundary so a later eager
projection cannot recreate the drift.

### Data Flow

1. Command resolution computes raw top speed and acceleration inputs.
2. Path planning succeeds.
3. Command installation updates intent/path/turn/track state while preserving
   all three speed projections.
4. The live-object movement visit reads the newly installed next cell plus
   current terrain/health facts.
5. The existing Drive fraction producer updates target/current state.
6. Current movement speed and Drive budget consume the updated result in the
   same visit.

No state is deferred beyond its native scheduled owner and no event queue is
added.

### Error Handling

Unsuccessful pathfinding continues to return false without changing speed
state. A successful path with no Drive runtime creates the runtime at native
constructor-equivalent zero defaults and then preserves those defaults until
the scheduled visit. Existing safe failure behavior for missing entity, grid,
or path remains unchanged.

### Testing Strategy

Focused `--lib` tests will prove:

1. `Simulation::apply_command(Command::Move)` reaches the production dispatcher
   and a successful Drive command preserves nontrivial
   target/current/owner-speed sentinel values exactly;
2. a fresh default Drive command leaves both fractions zero before the first
   scheduled visit;
3. the next `Simulation::advance_tick` scheduled visit still applies the existing
   `Accelerates=false` target snap and `Accelerates=true` ramp behavior;
4. a mid-curve reissue preserves in-flight fractions as well as position,
   track, occupation, and path anchor;
5. diff/search confirms ForceTrack, Stop, Ship, and tube code is not changed;
   existing tests for those paths remain regression evidence only and are not
   presented as proof that their known residuals match native.

Validation command after checking for another Cargo owner:

```text
cargo test -p vera20k --lib sim::movement::movement_tests::
```

A narrower filter may be used while iterating; no full-suite run belongs to
this prerequisite slice until the PR certification point required by the
active goal.

## Architectural Decisions

- Follow the existing command-intent versus scheduled-mutation boundary.
- Preserve `DriveLocomotionRuntime` as the Rust-native owner rather than
  introducing native object inheritance or a new command-time adapter.
- Make no snapshot-version change because the serialized schema is unchanged.
- Make no attempt to implement the full crate multiplier or native x87 speed
  formula in this branch; those require their own dependency-coherent design
  and evidence gates.
- Keep all separate verified fraction writers intact.

No new technical debt is introduced. The existing broader Drive speed
exactification residual remains explicitly open.

## Alternatives Considered

### 1. Recommended: delete the eager writer and pin preservation

Architecturally minimal and exact at the audited boundary. It lets the existing
scheduled owner observe live terrain/health state and creates no future crate
adapter. Risk is limited to exposing tests that depended on the drift.

### 2. Move target calculation into command installation

Rejected. Command time lacks the native scheduled `Process_Movement` ownership
and would snapshot terrain/health too early, duplicate movement-tick logic, and
still be wrong for mid-curve reissues and same-process transitions.

### 3. Defer deletion into the full crate/GetCurrentSpeed implementation

Rejected. The false writer is reached in ordinary movement now and is a proven
dependency boundary. Leaving it in place would make the later crate mechanism
depend on incorrect state ordering and enlarge that branch's review surface.
