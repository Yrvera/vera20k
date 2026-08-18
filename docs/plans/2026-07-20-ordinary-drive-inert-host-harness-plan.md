# Ordinary Drive Inert Host Harness Implementation Plan

**Date:** 2026-07-20  
**Status:** IMPLEMENTED and cold-reviewed PASS as a bounded test-only harness; production activation remains forbidden  
**Authority boundary:** test-only evidence scaffolding; no production movement activation

## Goal

Add one `#[cfg(test)]` cloned-fixture trace in `src/sim/world/techno_ai.rs` that records the verified ordinary-Unit path from segmented Techno work through stored `MissionCom.current == Move`, Mission timer handling, the concrete Unit Move wrapper, `FootClass::Mission_Move`, the five immediate Foot gates, a non-authoritative Drive `Process` marker, and Foot return.

The harness must prove ordering and branch coverage without mutating the live `Simulation`, consuming the live Scenario RNG, executing production movement, or weakening the approved atomic Phase-1 ground-movement flip.

## Grounding Summary

- The approved architecture requires one later atomic production flip across ordinary Unit, miner, Infantry/Walk, Hover, Ship, forced-track, and active low-bridge-tube movement. It explicitly permits cloned-fixture/test-only preparation before that flip and rejects a Drive-only production bridge.
- Checkpoint A now proves the ordinary Unit host order in active `gamemd.exe`: Techno guard B is mid-pre; Mission Dispatch always calls Object AI before active/timer/health gates; mission 2 reaches Unit's concrete `+0x22C` wrapper; Foot Move owns the NavCom/`Is_Moving`/queue/arrival/jitter split; passive acquire, bomb, SlaveManager, and CaptureManager precede guard E; Foot later applies a separate post-Techno guard, five immediate Process gates, one ILocomotion `+0x40` call, and an immediate post-Process guard.
- A fresh read-only assembly check confirms both Techno guard branches (`0x006FA244`, `0x006FA73D`) jump to the Techno epilogue at `0x006FAFFD..0x006FB004`; return resumes after Foot's call at `0x004DA539`, where Foot unconditionally reads `+0x90` at `0x004DA53E` and fails to its own epilogue at `0x004DA548`. Therefore a guard-B/E inactive return still records the immediate Foot post-Techno read/fail, although no later Foot work runs.
- A second caller check bounds the harness at Foot return: Unit calls Foot at `0x0073647B`, resumes at `0x00736480`, and executes an intervening Unit-tail slice before its next `+0x90` read at `0x007365BB` (`JZ 0x00736981` at `0x007365C3`). There is no immediate Unit post-Foot active guard. This harness emits `FootReturnMarker` and stops; Unit tail and its delayed active guard are explicitly out of scope.
- `[Move] Rate=.016` parses through current `MissionControl` to `rate_frames == 14`. The handler adds one `Scenario.RandomRanged(0,2)` API result, so the returned delay is 14, 15, or 16 frame-counter counts. One ranged API call can advance the raw RNG more than once because candidate 3 is rejected.
- Current Rust does not execute that mechanism. The committed Mission foundation
  routes `unit_techno_bracket` through
  `legacy_unit_host_projection`, which wrapping-increments `MissionCom.ai_counter`
  and projects `derived_mission`; `unit_ai_shadow_step` also reads
  `derived_mission`. `process_drive_locomotion_shell` remains only a read-only
  Drive-presence marker, and production movement remains in
  `tick_movement_with_grids` after the object pass.
- The current committed source baseline is `dev` HEAD
  `147c050b24a2030dec86512c277d658890e7ccd3`
  (`sim: establish dormant Mission authority foundation`). That foundation
  supplies lossless `MissionId`, private `MissionCom` state, exact signed
  `MissionDispatchTimer`, and test-fixture accessors. The harness remains the
  sole tracked working-tree delta and must be committed separately, if at all.

## Architecture Context and Impact Analysis

The smallest safe seam is entirely inside the existing `#[cfg(test)] mod tests` in `src/sim/world/techno_ai.rs`. The runner accepts `&Simulation`, clones exactly one `GameEntity` plus `sim.clone_scenario_rng()`, mutates only those clones, and returns trace evidence. Because no production item or call site is added, the production scheduler, global movement owner, persisted state, snapshot version, and release binary remain unchanged.

| Surface | Planned impact | Forbidden impact |
|---|---|---|
| `src/sim/world/techno_ai.rs` test module | Add test-local events, gates, cloned runner, inertness witness, and focused tests | No edit above `#[cfg(test)] mod tests`; no `debug_assertions` runtime helper |
| `MissionCom` / `MissionDispatchTimer` | Read stored `current`; wrapping-increment `ai_counter` and rewrite the signed dispatch timer only on the cloned entity | No live mission write; no `derived_mission` call |
| `MissionControl` | Parse stock `[Move] Rate=.016` and read `rate_frames == 14` | No new rate conversion; no general mod-rate or wall-time claim |
| `SimRng` | Clone Scenario RNG; make the one inclusive ranged call on the clone | No live RNG draw; no scaled/mapgen ranged API |
| Drive locomotion | Call `process_drive_locomotion_shell(&cloned_entity)` only to emit `DriveProcessMarker` | No movement tick, track, speed, position, facing, path, NavCom, arrival, occupancy, or lifecycle mutation |
| Snapshots/hash | Compare before/after live witnesses | No snapshot version change or rebaseline |

## Key Technical Decisions

| Decision | Confidence and source |
|---|---|
| Compile the complete harness only under `#[cfg(test)]` and colocate it with existing Techno AI tests. | HIGH; approved design Migration stages 1–2, reconciled implementation contract, Checkpoint A section 13 |
| Use stored `GameEntity.mission.current`, never `derived_mission`. Reject a non-Move fixture. | HIGH; native dispatch reads `+0xAC`; current `derived_mission` is only a Rust projection |
| Represent native `+0x90` reads and the five immediate Foot gates as explicit injected booleans. Do not map them to `health.current` or `dying`. | HIGH for branch order; the Rust field mapping is unproved |
| Inject and count ILocomotion `+0x10 Is_Moving`; never call or name it `Is_Moving_Now`. | HIGH; Drive vtable `+0x10 -> 0x004AFB80`; protocol `Is_Moving_Now` is `+0x80` |
| Record `+0xCC` only as `DispatchWriteScratchMarker`, with no value and no persisted field. | HIGH for write position; exhaustive observability/value semantics remain deferred |
| Use committed `MissionDispatchTimer::due` over the full raw signed-dword domain and rewrite returned delays with `MissionDispatchTimer::from_raw(native_frame as i32, delay)`. | HIGH; the Mission foundation implements native signed wrapping subtraction, the `-1` start sentinel, inclusive comparison, and lossless signed fields |
| Name subsystem tokens `*Marker`; they prove order only. | HIGH; passive acquire, bomb, managers, tracker restart, arrival tail, and later Foot bodies are not implemented by this harness |
| Stop the trace at `FootReturnMarker`; do not model Unit tail or invent an immediate Unit post-Foot active guard. | HIGH; fresh assembly at `0x0073647B`, `0x00736480`, `0x007365BB`, and `0x007365C3` |

## Open Questions and Explicit Non-Claims

These do not block the bounded harness, but the implementation and tests must preserve the labels below.

- **Mission dispatch timer semantics — IMPLEMENTED in the committed foundation.**
  Preserve raw signed start/delay dwords. Reinterpret `native_frame` as the same
  32 raw bits, use signed wrapping subtraction, treat start `-1` as always due,
  and use the inclusive signed `elapsed >= delay` comparison. The harness covers
  negative starts/delays, high-bit frames, pending reverse-order frames, and
  signed-wrap cases without normalizing them.
- **MissionControl storage/unit generality — UNCHECKED.** Use the production parser only to obtain the stock positive `.016 -> 14` value. Do not repeat the stale fixed-15-FPS, wall-time, or proven “minutes” interpretation.
- **Mission `+0xCC` — DEFERRED.** Emit its native-order marker but invent no value or storage.
- **Dispatch health width — BOUNDED.** The harness covers the ordinary positive-health and zero-health branches using current Rust `Health`; it does not certify negative or otherwise non-representable native signed-dword health states.
- **Subsystem effects — NOT IMPLEMENTED.** Trace markers do not implement Object AI, passive acquisition, bomb detonation, SlaveManager, CaptureManager, tracker restart, OnArrival, late Techno work, or later Foot work.
- **Unit tail/delayed active guard — OUT OF SCOPE.** The trace stops at Foot return. It neither emits Unit-tail markers nor invents an immediate Unit post-Foot guard; Unit's real intervening tail and delayed `+0x90` read require a separately bounded contract.
- **Inactive byte is not physical deletion.** A `+0x90 == 0` trace branch records inactive-state control flow only. Do not label it deleted, removed, freed, or absent from storage.
- **Drive body and parity — NOT IMPLEMENTED/UNVERIFIED.** `DriveProcessMarker` means only that the cloned fixture reached the existing read-only shell. It proves no speed, pathfinding, track, arrival, cell, locomotion, or pixel parity.
- Checkpoints B–E remain blocked: exact `GetCurrentSpeed`, RawTrack metadata, complete ground population/precedence, lifecycle/effect ownership, and an executable retail oracle.

## File Map

### Modify

- `src/sim/world/techno_ai.rs`
  - Add all new imports, types, helpers, and tests inside the existing `#[cfg(test)] mod tests` only.

### Read-only dependencies

- `src/sim/mission/mod.rs` and `src/sim/mission/state.rs` — `MissionCom`,
  stored `current`, `ai_counter`, and test-only fixture access.
- `src/sim/mission/timer.rs` — exact signed `MissionDispatchTimer::due` and raw
  signed construction.
- `src/sim/mission/control.rs` — production INI parse and `rate_frames`.
- `src/sim/rng.rs` — cloned `next_range_u32_inclusive(0,2)` and full logical state.
- `src/sim/movement/drive_locomotion.rs` — read-only `process_drive_locomotion_shell` marker.
- `src/sim/snapshot.rs` and `src/sim/world/world_hash.rs` — live inertness witnesses only.

No new Rust module, persisted field, runtime flag, handled-ID set, scheduler call, movement skip, snapshot version, or production API is permitted.

## Test-Only Interface

Add this test-local shape; names may be adjusted only for Rust style, not semantics:

```rust
fn trace_cloned_ordinary_drive_host(
    sim: &Simulation,
    id: u64,
    mission_control: &MissionControl,
    native_frame: u32,
    gates: HostTraceGates,
) -> Result<ClonedHostTrace, HostTraceError>;
```

The supporting model must remain inside `mod tests`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveGate {
    GuardB,
    Dispatch,
    GuardE,
    FootPostTechno,
    FootPostProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnitMoveByte {
    Byte6e1,
    Byte6e2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HostTraceEvent {
    TechnoPreThroughRocking,
    ActiveGate { gate: ActiveGate, pass: bool },
    TechnoRemainingPre,
    MissionAiCounter { before: u32, after: u32 },
    MissionDispatchEnter,
    ObjectAiMarker,
    DispatchTimerGate { due: bool },
    DispatchHealthGate { pass: bool },
    UnitMoveRead6e0 { nonzero: bool },
    UnitMoveClear6d2,
    UnitMoveCheckSaved6e0 { nonzero: bool },
    UnitMoveCheck { byte: UnitMoveByte, nonzero: bool },
    QueueMissionMarker { mission_id: MissionId, arg: u32 },
    UnitTrackerCheckMarker,
    UnitTrackerRestartMarker,
    FootMissionMove,
    NavComCheck { present: bool },
    IsMovingCall { moving: bool },
    NullLocomotorInvariant,
    OnArrivalMarker { arg0: u32, arg1: u32 },
    RateLookup { mission: MissionType, frames: u32 },
    ScenarioRandomRangedApi {
        low: u32,
        high: u32,
        value: u32,
        raw_advances: usize,
    },
    DispatchWriteStart { frame: i32 },
    DispatchWriteScratchMarker,
    DispatchWriteDelay { delay: i32 },
    PassiveAcquireMarker,
    BombMarker,
    SlaveManagerMarker,
    CaptureManagerMarker,
    TechnoLatePostMarker,
    FootPreProcessMarker,
    FootProcessGate { ordinal: u8, pass: bool },
    DriveProcessMarker,
    FootLaterWorkMarker,
    FootReturnMarker,
}

#[derive(Debug, Clone, Copy)]
struct HostTraceGates {
    guard_b_active: bool,
    dispatch_active: bool,
    guard_e_active: bool,
    foot_post_techno_active: bool,
    foot_post_process_active: bool,
    unit_move_bytes: [bool; 3],
    tracker_needs_restart: bool,
    is_moving: bool,
    foot_process_gates: [bool; 5],
    class_special_pre_foot_path: bool,
    lifecycle_countdown_exit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClonedHostTrace {
    events: Vec<HostTraceEvent>,
    mission_after: MissionCom,
    scenario_rng_after: SimRngLogicalState,
    is_moving_calls: u8,
    move_random_ranged_calls: u8,
}
```

`HostTraceGates::ordinary()` must set all five active gates and all five Foot gates to pass, all three Unit wrapper bytes to zero, the tracker restart off, `is_moving=true`, and both out-of-scope class-special/lifecycle paths off.

`HostTraceError` must distinguish at least missing entity, non-Unit, non-Move
stored mission, miner, dock, aircraft, non-ordinary/special locomotor, active
tube, forced track, class-special/lifecycle paths, missing Drive runtime where
the null-NavCom invariant exception does not apply, and stock Move rate other
than 14. Exact signed timer inputs are accepted losslessly rather than rejected.
The null-locomotor case is not an ordinary validation error only when Foot Move
has null NavCom: record `NullLocomotorInvariant` and end without fabricating a
safe native continuation.

## Task 1: Add the Cloned Host Runner

Inside `techno_ai.rs`'s existing test module:

1. Import `IniFile`, `MissionCom`, `MissionControl`, `MissionDispatchTimer`,
   `MissionId`, `SimRngLogicalState`, and `GameSnapshot` locally.
2. Add `stock_move_control()` using exactly:

   ```rust
   MissionControl::from_ini(&IniFile::from_str("[Move]\nRate=.016\n"))
   ```

3. Read the clone's `MissionDispatchTimer` without normalization. Accept every
   raw signed start/delay pair and every `u32` frame bit pattern; evaluate them
   with the committed exact signed/wrapping `due` implementation.
4. Validate the cloned source entity as an ordinary Unit fixture:
   - category `Unit`;
   - stored `mission.current == MissionType::Move`;
   - active and primary `LocomotorState` kinds are both `Drive`, `piggyback.is_none()`, and `!is_overridden()`;
   - `drive_locomotion.is_some()` for every safe ordinary-path case; the sole exception is null NavCom plus missing runtime, which emits `NullLocomotorInvariant` and stops;
   - both tube carriers are absent: `low_bridge_tube_state.is_none()` and `drive_locomotion.active_tube.is_none()`;
   - `forced_drive_track.is_none()`;
   - no miner, dock, aircraft mission, or other active special movement state;
   - injected class-special pre-Foot and lifecycle-countdown paths disabled.
5. Clone the entity and Scenario RNG. The runner takes only `&Simulation`; it must never obtain `&mut Simulation`.
6. Execute the event algorithm below on the clones only.

### Required ordered algorithm

1. Emit `TechnoPreThroughRocking`, then guard B. On failure, Techno returns through its epilogue to Foot; emit the caller's immediate `FootPostTechno` active-gate read as failed, emit `FootReturnMarker`, then end with no Foot pre-work.
2. Emit `TechnoRemainingPre`; wrapping-increment the clone's
   `mission.ai_counter` through its test fixture and record before/after.
3. Emit `MissionDispatchEnter`, `ObjectAiMarker`, and the Dispatch active gate.
4. If the Dispatch active gate is false, latch the same injected `+0x90` byte inactive for the remainder of this marker-only trace: do not evaluate the timer or handler. After returning to Techno, the early-post markers still execute, but guard E and Foot post-Techno must both record false before `FootReturnMarker`; no later Foot/Drive marker is possible. If active, evaluate the clone timer with `MissionDispatchTimer::due(native_frame)`.
   - Not due: record the timer gate only; do not read health, enter the Unit wrapper, draw RNG, or rewrite the timer.
   - Due: record the bounded clone predicate `health.current > 0`. A failed health gate invokes no handler and performs no timer rewrite; make no claim about native negative-health bit patterns that current Rust cannot represent.
5. On due/live stored Move, execute the Unit wrapper markers in native order:
   - read `+0x6E0`;
   - clear `+0x6D2` marker;
   - emit `UnitMoveCheckSaved6e0` for the saved pre-clear value;
   - short-circuit on saved `+0x6E0`, then read/check `+0x6E1`, then read/check `+0x6E2`;
   - read `+0x6E1` and `+0x6E2` only if the preceding tests fall through; do not use eager destructuring, `.iter().any()`, or any model that erases the native read order;
   - any nonzero byte emits
     `QueueMissionMarker { mission_id: MissionId::from_known(MissionType::Guard), arg: 0 }`
     and returns handler delay 1 without Foot Move or Move RNG;
   - otherwise emit tracker check, optional restart, then enter Foot Move.
6. Foot Move:
   - record NavCom presence from the cloned entity;
   - live NavCom: do not call injected `Is_Moving`;
   - null NavCom: require a cloned active Drive runtime, call the injected `Is_Moving` exactly once, and record it;
   - null NavCom + stopped + `mission.queued() == MissionId::NONE`: emit
     `OnArrivalMarker {0,1}`, return 1, and consume no Move RNG;
   - all other safe branches: read stock Move rate 14, make exactly one `next_range_u32_inclusive(0,2)` call on the cloned Scenario RNG, and return `14 + jitter`.
7. For a returned handler delay, record native write order as signed start,
   scratch marker, signed delay, then install
   `MissionDispatchTimer::from_raw(native_frame as i32, delay)` through the
   clone's Mission test fixture. Do not invent scratch data.
8. Emit passive acquire, bomb, SlaveManager, and CaptureManager markers in that order, then guard E. If Dispatch previously observed the same `+0x90` byte false, force this guard false; otherwise use the injected guard-E value. On failure, Techno returns through its epilogue to Foot; emit the caller's immediate `FootPostTechno` active-gate read as failed, emit `FootReturnMarker`, then end with no late Techno or Foot pre-work. Ignore/prevent any inconsistent later injected true state on an already-inactive return path.
9. Emit late Techno, then the separate Foot post-Techno gate. A failed Foot gate emits `FootReturnMarker` and ends before Foot pre-Process work.
10. Emit Foot pre-Process work and evaluate the five injected Process gates in ordinal order. On a failure, emit no Drive marker, then emit the later-Foot/join marker and `FootReturnMarker`.
11. If all gates pass, call only `process_drive_locomotion_shell(&cloned_entity)`. Emit `DriveProcessMarker` only for `DriveProcessOutcome::Processed`; never describe this as completed movement.
12. Emit the immediate post-Process active gate. Failure reaches the shared Foot epilogue and emits `FootReturnMarker`; success emits later Foot and then `FootReturnMarker`. Stop the harness there in both cases. Do not trace the subsequent Unit tail or its delayed active read.

For RNG evidence, do not infer `raw_advances` from an index delta modulo 250 because that aliases 250 or more draws. Run the real `next_range_u32_inclusive(0,2)` once on the harness clone. Run a second probe clone through the same low-two-bit mask/reject loop, incrementing an unbounded test-local `usize` once per `next_u32()` until a candidate is accepted. Assert that the probe's accepted value and complete `logical_state()` equal the real API clone, then attach the exact probe count to the one API event. Keep `move_random_ranged_calls` separate so one API call and multiple raw advances remain distinct facts.

## Task 2: Add a Reusable Inertness Witness

Add a test-only wrapper that captures these live values before and after every trace invocation and asserts exact equality:

- `sim.state_hash()`;
- `sim.rng_state()` (Scenario, Main, and MapGen full logical states);
- the live entity's `MissionCom`;
- `GameSnapshot::save(sim, 0, 0, "checkpoint_a_host_trace", 0)` bytes;
- occupancy debug representation, generation, and occupied-cell count;
- lengths of live sound, fire, pending-smudge, bale, bunker-wall, and world-effect queues.

The witness should return the `ClonedHostTrace` only after all equality assertions pass. This is regression evidence that the harness is inert; it is not gamemd parity evidence.

## Task 3: Add Focused Branch Tests

Use the shared prefix `checkpoint_a_ordinary_drive_host_` so one Cargo filter runs the complete slice.

1. `checkpoint_a_ordinary_drive_host_due_move_full_order_is_inert`
   - stored Move, zero-delay signed timer, live NavCom, healthy clone, wrapper bytes zero, all gates pass;
   - assert the complete baseline event order from Techno pre through `FootReturnMarker`;
   - assert zero `Is_Moving` calls, one ranged API call, delay 14–16, and unchanged live witnesses.
2. `checkpoint_a_ordinary_drive_host_timer_not_due_still_marks_process`
   - use a small armed timer and frame one count before due;
   - assert Object AI/active/timer markers, no health/wrapper/Move/RNG/write markers, then full Techno post and eligible Drive marker.
3. `checkpoint_a_ordinary_drive_host_due_health_failure_skips_handler_and_write`
   - keep injected active bytes true but set clone health to zero;
   - assert health read/fail, no handler/RNG/timer write, and return to Techno post.
4. `checkpoint_a_ordinary_drive_host_dispatch_inactive_propagates_to_foot_return`
   - assert Object AI precedes the failed Dispatch active gate and no timer or handler runs;
   - assert passive/bomb/slave/capture still follow, then guard E is forced false, Foot post-Techno is forced false, and `FootReturnMarker` ends the trace with no late Techno, Foot pre-work, or Drive marker.
5. `checkpoint_a_ordinary_drive_host_each_unit_wrapper_byte_queues_guard`
   - table-drive `[1,0,0]`, `[0,1,0]`, `[0,0,1]`;
   - assert exact `Read6e0 -> Clear6d2 -> CheckSaved6e0 -> conditional Check6e1 -> conditional Check6e2` order and that no later byte is eagerly read after a true check;
   - assert queue `(5,0)`, returned/write delay 1, no Foot Move/RNG, and later eligible Drive marker.
6. `checkpoint_a_ordinary_drive_host_foot_move_branch_matrix_uses_is_moving`
   - live NavCom: zero `Is_Moving` calls, one ranged call;
   - null NavCom + moving: one `Is_Moving` call, one ranged call;
   - null NavCom + stopped + queued mission: one `Is_Moving`, no arrival, one ranged call;
   - null NavCom + stopped + no queue: one `Is_Moving`, arrival `(0,1)`, delay 1, zero ranged calls.
7. `checkpoint_a_ordinary_drive_host_null_locomotor_is_invariant`
   - null NavCom and no Drive runtime;
   - assert invariant marker and no fabricated arrival, timer write, Techno post, or Drive marker.
8. `checkpoint_a_ordinary_drive_host_guard_exits_truncate_exact_segments`
   - table-drive guard B fail, guard E fail, and Foot post-Techno fail;
   - for guard B/E failure, assert Techno returns to Foot, the immediate Foot post-Techno active read is recorded as failed, and `FootReturnMarker` ends the bounded trace;
   - assert no remaining Techno segment or Foot pre-work occurs, and prevent an injected true Foot state from overriding the already-failed native byte.
9. `checkpoint_a_ordinary_drive_host_each_foot_process_gate_short_circuits`
   - flip each of the five gate booleans separately;
   - assert gates are visited only through the failed ordinal, no Drive marker occurs, and later Foot join plus `FootReturnMarker` still occur.
10. `checkpoint_a_ordinary_drive_host_post_process_guard_uses_epilogue_exit`
    - all Process gates pass, then post-Process active is false;
    - assert Drive marker, failed guard, then `FootReturnMarker`, with no later-Foot marker and no Unit-tail event of any kind.
11. `checkpoint_a_ordinary_drive_host_rng_rejection_advances_clone_twice`
    - construct the fixture as `Simulation::with_seed(9)` before capturing the inertness witness, so the runner still obtains its RNG only through `sim.clone_scenario_rng()`;
    - independently prove raw low-two-bit candidates `3` then `0` with two `next_u32()` calls on a reference clone;
    - assert one ranged API event, `raw_advances == 2`, and full cloned RNG logical state equality with the two-step reference; live RNG remains unchanged.
12. `checkpoint_a_ordinary_drive_host_reads_stored_move_not_derived_projection`
    - arrange stored `mission.current = Move` while the current optional-machine projection would not be Move;
    - assert the Move wrapper/handler trace runs without calling `derived_mission`.
13. `checkpoint_a_ordinary_drive_host_rejects_out_of_scope_fixtures`
    - cover missing entity, non-Unit, non-Move, both active-tube carriers,
      forced track, miner, dock, aircraft, Drive-primary mismatch, piggyback,
      override, class-special path, lifecycle exit, missing runtime outside the
      null-NavCom invariant exception, and bad stock Move rate;
    - assert explicit errors rather than coercion into the ordinary path.
14. `checkpoint_a_ordinary_drive_host_uses_exact_signed_dispatch_timer_domain`
    - cover start `-1` with positive/zero delay, reverse-order frame
      `(start=10, delay=5, now=9)`, high-bit start/current combinations,
      `i32::MIN` delay, and negative-start signed-wrap due behavior;
    - assert the exact due/not-due marker, one AI-counter increment, no handler
      or timer rewrite while pending, and signed raw start/delay writes when due.

## Validation

The implementation-owning task runs these steps serially:

1. Before Cargo, check for another owner:

   ```powershell
   Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
   ```

2. Format only the edited file and inspect for unrelated churn:

   ```powershell
   rustfmt --edition 2024 --config skip_children=true src/sim/world/techno_ai.rs
   git diff -- src/sim/world/techno_ai.rs
   ```

3. Run the bounded suite:

   ```powershell
   cargo test -q checkpoint_a_ordinary_drive_host_
   ```

   Report the literal `test result:` line.

4. Run the existing adjacent regression checks:

   ```powershell
   cargo test -q unit_ai_mission_dispatch_precedes_locomotor_process
   cargo test -q s1_no_hash_change_shadow
   cargo test -q rate_to_frames_uses_900_per_minute
   cargo test -q random_ranged_power_of_two_span_matches_gamemd_draw_stream
   ```

5. Run the grouped adjacent Techno-AI and Mission regressions:

   ```powershell
   cargo test -p vera20k techno_ai -- --nocapture
   cargo test -p vera20k --lib mission -- --nocapture
   ```

6. Run one final compile check:

   ```powershell
   cargo check -q -p vera20k
   ```

7. Static scope audit:

   ```powershell
   git diff --check
   git diff -- src/sim/world/techno_ai.rs
   rg -n "trace_cloned_ordinary_drive_host|DriveProcessMarker" src/sim/world/techno_ai.rs
   git diff --unified=0 -- src/sim/world/techno_ai.rs | rg "^\+.*(tick_movement_with_grids|derived_mission)"
   ```

   The final `git diff | rg` command is expected to return no matches (exit 1). The new harness block must contain neither `tick_movement_with_grids` nor `derived_mission`, and the diff must not change production code above the existing test module.

## Stop Condition

Stop and return to research rather than broadening the patch if any of these occurs:

- the helper requires `&mut Simulation` or a production call site;
- a branch cannot be represented without inventing a native field mapping;
- a test requires production movement, live RNG, a snapshot bump, or a handled-ID skip;
- the timer test requires normalizing raw signed fields, bypassing the committed
  `MissionDispatchTimer`, or claiming general mod-rate equivalence;
- the implementation tries to extend beyond `FootReturnMarker` into Unit tail or adds an immediate Unit post-Foot active guard;
- any current production hash/snapshot/RNG/mission/occupancy/event witness changes.

Completion means the test-only helper and all fourteen named focused tests pass,
the adjacent checks pass, `cargo check -q -p vera20k` passes, and the Rust diff
is confined to the existing `#[cfg(test)]` module. It does not close Checkpoints
B–E and does not authorize production activation.

## Mandatory Post-Implementation Review

Cold-review the Rust diff against this checklist before any later staging or commit request:

- one test-only helper, no production symbols or call sites;
- stored mission only; no `derived_mission` in the new block;
- clone-only Scenario draw and full live RNG equality;
- one ranged API event versus one-or-more raw advances kept distinct;
- exact Object AI/timer/health and Unit wrapper branch ordering;
- explicit `Is_Moving` call counts and null-locomotor invariant;
- exact passive/bomb/slave/capture order and guard E placement;
- all five Foot gates and immediate post-Process guard;
- every safe trace ends at `FootReturnMarker`; no Unit-tail event or immediate Unit post-Foot gate is present;
- inactive `+0x90` branches are never described as physical deletion/removal;
- `DriveProcessMarker` never described as movement execution or parity;
- no `+0xCC` value, production skip, snapshot bump, hash rebaseline, or atomic-flip claim.

## Sources

- `docs/plans/2026-07-20-per-object-ground-movement-drive-process-design.md`
- `docs/plans/2026-07-20-ground-movement-atomic-flip-readiness-investigation-plan.md`
- `docs/contracts/2026-07-20-ground-drive-process-track-stepping-implementation-contract.md`
- `docs/research/TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md`
- `docs/research/FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`
- `docs/research/OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`
- Active `gamemd.exe` anchors cited by Checkpoint A: `0x006F9E50`, `0x005B3060`, `0x00740A90`, `0x004D4200`, `0x004DA530`, `0x004B0500`
- Current Rust: `src/sim/world/techno_ai.rs`,
  `src/sim/mission/{mod,state,timer,control}.rs`, `src/sim/rng.rs`,
  `src/sim/movement/drive_locomotion.rs`, `src/sim/world/mod.rs`, and
  `src/sim/snapshot.rs`
- Stock data: `ini/rulesmd.ini [Move] Rate=.016` with base `ini/rules.ini` agreement
