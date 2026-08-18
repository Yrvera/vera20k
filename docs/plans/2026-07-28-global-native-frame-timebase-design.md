# Global Native-Frame Timebase — Design

**Date:** 2026-07-28  
**Status:** APPROVED — global approach selected by the user on 2026-07-28  
**Target:** active standard Yuri's Revenge local skirmish, with stock
`[MultiplayerDialogSettings] GameSpeed=1` as the ordinary delivery case

## Goal

Make one deterministic Rust simulation step mean exactly one reached native
`Main_Tick` / game frame, then make every deterministic gameplay system consume
that same frame basis.

The player-visible target is that stock units move, turn, reload, animate,
produce, harvest, and wait at the same relative and wall-clock pace as active
Yuri's Revenge at the same `GameSpeed`. The first reported symptom is locomotion,
but the fix is deliberately global: a locomotion-only multiplier would leave
movement on a different clock from combat, production, mission timers, replay
steps, and animation.

This design supersedes the timebase choices in:

- `docs/plans/2026-05-28-native-frame-rate-design.md`
- the rate-mismatch deferral in
  `docs/plans/2026-05-28-native-frame-tick-contract-design.md`

It preserves that earlier work's verified pre-increment visibility and late
frame commit. It does not supersede the locomotor ownership and Drive control
flow in:

- `docs/plans/2026-07-20-per-object-ground-movement-drive-process-design.md`

## Architecture Context

### Verified native contract

The following facts are implementation authority:

1. One reached `Main_Tick` contains one full gameplay pass and one live-object
   pass. There is no inner multi-pass catch-up loop.
2. `g_CurrentFrameCounter` is visible at its old value throughout gameplay work
   and increments once, late, on the normal completed path.
3. Every eligible Foot object turn offers its active locomotor one `Process`
   call. Drive has no independent 15 Hz admission gate.
4. Local `GameSpeed` changes the wall-clock wait between reached ticks. It does
   not multiply or divide ordinary gameplay work such as locomotion, ROF, or
   production inside a tick. Verified normalized `AnimClass` delays are the
   narrow exception: `Normalized=yes` adjusts the frame delay from the speed
   byte after the `900 / Rate` conversion.
5. The local timer source is `timeGetTime() >> 4`: one bucket is 16 ms.
   Stored speeds `1..6` require `1..6` bucket advances. Stored speed `0` is
   uncapped/workload-limited.
6. Stock YR local skirmish defaults to stored `GameSpeed=1`, giving a verified
   static minimum period of one 16 ms bucket and a nominal ceiling near 62.5
   reached frames per second.
7. Native gameplay durations, animation rates, RateTimer/CDTimer state, and
   modulo gates are frame-counter based. The familiar 15 fps value is an
   INI/art authoring convention, not a fixed runtime logic rate.

Primary evidence:

- `docs/research/NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION_GHIDRA_REPORT.md`
- `docs/research/MAIN_TICK_SPEED_BUDGET_MS_PER_FRAME_GHIDRA_REPORT.md`
- `docs/research/OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`
- `docs/research/FRAME_BASIS_MOVEMENT_TURRET_GHIDRA_REPORT.md`
- `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`

This is active standard-YR evidence. Tiberian Sun names, constants, and inherited
comments are not authority unless the active YR call path proves their use.

### Current Rust contract

Rust currently has three incompatible timing interpretations:

| Clock/input | Current meaning | Default wall rate | Mismatch |
|---|---|---:|---|
| `ScenarioSession.tick` | one complete `advance_tick` pass | about 63/s | structural native-frame twin, but described as a separate Rust tick |
| `total_sim_ms` | `+22` for every pass | about 1,386 synthetic ms/s | not wall time and not a native gameplay clock |
| `binary_frame` | `floor(total_sim_ms * 15 / 1000)` | about 21/s | advances once per three complete gameplay passes |
| `tick_ms` | deterministic delta passed through sim | always 22 ms | converts native per-frame values into a synthetic seconds domain |

The app's current `tps_for_game_speed(1)` is directionally close to the native
one-bucket ceiling, but it reaches that rate by scaling elapsed time against a
45 Hz fixed-step quantum. A single render update may also execute several full
simulation passes as catch-up work.

The locomotion symptom follows directly:

- generic Walk/Ship movement converts an integer native per-frame budget to
  leptons/second with a `*15`, then applies only 22 ms of it per complete pass;
- normal Drive additionally admits a fresh budget only every third pass through
  `DRIVE_TRACK_SUBTICKS_PER_NATIVE_FRAME`;
- at default pacing, Walk therefore receives roughly one third of its expected
  per-pass travel and Drive receives fresh processing at roughly one third of
  its expected invocation opportunities.

The same synthetic conversion appears in ROF, turret rotation, air and special
movement, deterministic animation state, effects, capture receipts, replays,
and tests. Fixing only the two ground-movement sites would create two gameplay
time authorities.

### Existing owner boundaries

- The app layer owns wall-clock sampling, local `GameSpeed` pacing, pause/modal
  admission, rendering, and UI-only cosmetic time.
- `Simulation::advance_tick` owns one full deterministic gameplay pass and the
  late native-frame commit.
- The current production movement wrapper remains the movement owner during
  this timebase change. The approved per-object ground-movement design may later
  move that ownership, but it must inherit this design's one-frame/one-process
  cadence and may not introduce a second locomotion clock.
- Replay, snapshot, tactical-capture, bootstrap, and state-hash code expose the
  current clock contract and therefore belong to the atomic migration.

## Player-Experience Ledger

### Milestone-blocking

1. One admitted simulation step performs exactly one full native-equivalent
   gameplay frame.
2. The current native frame is constant during that step and increments once,
   late, on the normal completed path.
3. `GameSpeed` changes the wall-clock admission interval and only those
   per-frame values, such as normalized `AnimClass` delay, whose native
   mechanism explicitly consumes the speed byte.
4. Walk, Drive, Hover, Ship, Fly/Jumpjet, Rocket, Homing, Tunnel, DropPod, and
   other active stock movement paths receive per-frame inputs, not 22 ms deltas.
5. Drive gets a processing/budget opportunity on every eligible reached object
   turn. There is no every-third-step gate.
6. ROF, facing, production, harvesting, gates, ore, superweapons, AI cadence,
   and deterministic animations share the same frame basis before the
   production authority flips.
7. Exact-step, replay, save, capture, and state-hash contracts identify one
   native frame, not one 22 ms substep.

### Compounding

1. Game speed must scale the wall-clock pace of ordinary frame-counted gameplay
   uniformly. It must not change frame-indexed outcomes except where a verified
   native mechanism explicitly consumes the speed byte, notably normalized
   `AnimClass` delay.
2. No simulator code may infer elapsed seconds from a nominal frame rate.
3. Existing per-pass counters such as movement blocker delay decrement once per
   native frame and are not multiplied by three merely because old comments
   called a frame 15 Hz.
4. Offline Options/pause admits no frame and advances no gameplay timer.
5. Input, app service, and render interleaving must not be hidden behind a batch
   of several native frames in one host iteration.

### Exactification

1. Stored speed `0` remains workload-limited rather than being represented as a
   deterministic 60 Hz cap.
2. Exact achieved retail FPS and jitter under a particular machine/render load
   remain runtime-unmeasured. The verified bucket mechanism is sufficient for
   the architecture; a retail probe may refine performance expectations.
3. Network pacing uses a separate native millisecond budget and remains outside
   this local-skirmish design.

### Unknown-risk

1. Rust-authored millisecond animation/effect constants must be classified as
   either deterministic native gameplay or app-only cosmetics. They may not be
   converted with an assumed global FPS.
2. Rust heuristic constants currently named `*_TICKS` must be classified as a
   verified native frame count or an explicitly authored Rust behavior before
   their new value is selected.
3. Any INI value expressed as seconds/milliseconds must use that subsystem's
   verified native load-time conversion to frames. A generic runtime
   `seconds * 15` adapter is forbidden.

## Impact Analysis

| Area | Primary files | Required change |
|---|---|---|
| Local frame pacing | `src/app.rs`, `src/app_types.rs`, `src/app_sim_tick.rs` | Replace 45 Hz scaled accumulator/catch-up batching with one local native-frame admission per outer app iteration using the verified speed-byte bucket period |
| Session clock | `src/sim/scenario_session.rs`, `src/sim/world/mod.rs` | Replace synthetic `binary_frame` derivation with one authoritative wrapping native-frame increment; remove deterministic `total_sim_ms` |
| Simulation API | `src/sim/world/mod.rs`, callers | Replace `advance_tick(..., tick_ms)` with a one-frame API that has no runtime millisecond delta |
| Fixed math | `src/util/fixed_math.rs` | Retire `SIM_TICK_HZ`, `dt_from_tick_ms`, and runtime per-second conversion from deterministic gameplay; expose verified per-frame integer/fixed helpers |
| Ground locomotion | `src/sim/movement/movement_tick.rs`, `movement_step.rs`, `drive_locomotion.rs` | Apply one frame of movement per invocation; remove the Drive 15 Hz/subtick gate while preserving speed fractions, residuals, strict point-budget comparisons, and retry masking |
| Air/special locomotion | `air_movement.rs`, `rocket_movement.rs`, `homing_movement.rs`, `tunnel_movement.rs`, `droppod_movement.rs`, `parachute_descent.rs`, `teleport_movement.rs` | Replace `tick_ms`/`dt` progression with each mechanism's per-frame update |
| Turning/facing | `movement/turret.rs`, `movement/facing_class.rs`, `movement/hover.rs` | Use frame-authored ROT/RateTimer inputs directly; preserve pre-increment current-frame reads |
| Combat | `src/sim/combat/mod.rs` | Retire ROF-frame → ms → substep conversion; seed/decrement cooldowns in native frames |
| Economy/AI/timers | production, miner, gate, ore, superweapon, AI, mission, docking, particles | Audit every `session.tick`, modulo, cooldown, and rate constant; migrate gameplay reads to the native frame and preserve verified frame counts |
| Deterministic animation/effects | `src/sim/animation.rs`, component animation/effect state, radar/world effects | Store and advance frame durations; remove simulator ms accumulation |
| Renderer cosmetics | `src/app_render/build_instances.rs`, `src/render/pixel_fx_sparkles.rs`, app animation modules | Use native-frame state for native/game-speeded visuals or app wall time for explicitly cosmetic UI; do not read a hashed synthetic sim-ms clock |
| Replay/save/hash | `src/sim/replay.rs`, `snapshot.rs`, `world_hash.rs`, `scenario_session.rs` | Version-break old timing artifacts, hash the new frame state, and reject rather than reinterpret old formats |
| Exact capture/bootstrap | `src/app_sim_tick.rs`, `match_bootstrap.rs`, `app_tactical_capture/*`, parity digest | Replace 45 Hz/22 ms assertions with one-step/one-native-frame assertions and bump the sealed contract |
| Tests/goldens | movement, combat, world, replay, capture, miner, lifecycle tests | Replace synthetic-ms expectations with frame-boundary assertions and rebaseline only after behavior is explained |

The implementation touches `src/app_sim_tick.rs`, which is dirty in the current
shared checkout from another task. Implementation must reconcile ownership and
actual repository state before editing; this design document does not authorize
overwriting that work.

## Chosen Approach

### Global single native-frame authority

The user selected the global approach on 2026-07-28.

One normal Rust gameplay step is one native-equivalent frame. No production
hybrid is allowed: locomotion cannot flip to per-frame budgets until all
ordinary gameplay clock consumers, persisted formats, and exact-step contracts
are migrated in the same authority change.

The implementation may prepare pure helpers and update inert/test-only fixtures
before the flip. It may not ship a runtime feature flag or a state in which some
systems still interpret one step as 22 ms.

### App-local frame pacer

The local pacer is wall-clock-only and must not enter deterministic state.

For each outer in-game app iteration:

1. sample the frame-start time;
2. admit at most one simulation frame;
3. run the normal app/render/service work for that frame;
4. for stored speeds `1..6`, wait until the frame-start bucket has advanced by
   the stored speed byte (`speed * 16 ms` nominal);
5. for stored speed `0`, do not impose a time budget;
6. if work already exceeded the budget, allow the next outer iteration
   immediately.

There is no accumulated multi-frame catch-up batch. A slow host is
throughput-limited just as native is; it does not hide several complete
gameplay frames behind one input/render opportunity.

Exact diagnostic stepping bypasses the wall wait, admits exactly one frame, and
re-anchors/discards pacing debt so ordinary execution cannot inherit a stale
remainder.

### Simulation frame state

`ScenarioSession` has two counters with deliberately different roles:

- `native_frame: u32` replaces `binary_frame`. It is the only gameplay time
  authority and mirrors `g_CurrentFrameCounter`, including wrapping arithmetic.
- `tick: u64` remains a monotonic executed-step ordinal for command scheduling,
  replay indexing, diagnostics, and long-run bookkeeping. It is not a gameplay
  duration source.

`total_sim_ms` is removed from deterministic session state and from the state
hash.

During a normal frame:

1. all gameplay work reads `native_frame == N`;
2. timers started and checked in the same frame observe elapsed `0`;
3. the normal tail commits `native_frame = N.wrapping_add(1)`;
4. pending-delete processing keeps its already-established position after the
   native frame commit;
5. the step ordinal and end-of-step hash commit at their established tail point.

The late session-end path that native proves skips the frame increment must
remain a terminal non-commit path when that behavior is implemented. Pausing is
handled by not entering the frame at all.

### Simulation API

The production entry point becomes conceptually:

```text
Simulation::advance_frame(commands, rules, map inputs) -> FrameResult
```

It has no `tick_ms`, delta-seconds, target-TPS, or `GameSpeed` argument.
`GameSpeed` is primarily an app/session pacing concern; adding it to
locomotion, combat, production, or generic timer calculations is forbidden.
The simulator may read the deterministic session speed byte only through a
named, evidence-backed normalized-animation conversion.

The name may be migrated mechanically from `advance_tick` to `advance_frame`,
but the semantic change and removal of `tick_ms` are mandatory even if the
existing symbol is temporarily retained during preparation.

### Gameplay clock migration policy

Every current timing consumer is assigned exactly one class:

1. **Native per-frame work:** execute once per admitted frame with no delta
   multiplier. Examples: object AI, locomotor `Process`, cooldown decrement,
   production progress, blocker counters.
2. **Native stored-start/due-frame timer:** store/read `native_frame` with the
   verified wrapping/inclusive comparison for that subsystem.
3. **Native authored rate/duration:** keep the INI/art frame count directly, or
   perform the subsystem's verified conversion. `AnimClass Rate=` uses integer
   `900 / Rate`; `Normalized=yes` then uses the verified speed-byte
   normalization table/formula. Runtime milliseconds are forbidden.
4. **App-only cosmetic wall time:** keep `Instant`/elapsed milliseconds outside
   `sim/`; it may affect UI/camera/audio polish but never gameplay, entity
   lifecycle, a state hash, or a tactical capture's deterministic observation.

Ambiguous consumers are blockers to the atomic authority flip, not candidates
for a generic `15`, `45`, or `63` conversion.

### Locomotion contract

- The base rules `Speed` conversion produces a per-frame lepton budget:
  `min(floor(clamp(Speed, 0, 100) * 256 / 100), 255)` before the mechanism's
  verified fractions/modifiers.
- Walk and other continuous ground paths apply one frame's result directly.
  They do not multiply by 15 and then multiply by an elapsed-seconds delta.
- Every eligible current-owner movement pass offers each active locomotor its
  one frame of work. The future per-object owner will preserve the same rule.
- Drive removes `DRIVE_TRACK_NATIVE_FRAME_HZ`,
  `DRIVE_TRACK_NATIVE_FRAME_MS`,
  `DRIVE_TRACK_SUBTICKS_PER_NATIVE_FRAME`, and `drive_delay` as cadence
  authority.
- A newly installed Drive track with zero budget still consumes no point and
  changes no coordinate/facing.
- Normal Drive applies its verified speed-state update and requests one fresh
  integer budget for that Process/DriveTrack invocation.
- A same-Process Drive retry repeats the verified pre-budget speed-state work
  but masks the second fresh integer contribution and consumes residual only.
- Point cost remains exactly 7 and consumption remains strict `budget > 7`.
- Blocked/path-delay counters decrement once per reached native frame. They are
  not rescaled merely because the old simulation used three substeps.

### Non-locomotion contract

- ROF and burst/cooldown state use frame counts. A positive `ROF=N` is not
  converted through milliseconds.
- Facing/ROT progression uses native frame input and the pre-increment current
  frame.
- Production, AI, mission, docking, ore, gate, superweapon, particle, and
  lifecycle modulo/cooldown consumers use `native_frame` when they represent
  gameplay time. `tick` remains available only for ordinal/transport concerns.
- Deterministic entity animation advances only when a native frame is admitted.
  Rendering the same committed frame repeatedly cannot advance it.
  `Normalized=yes` animation delays are derived from `Rate=` and the
  deterministic session speed byte exactly as native specifies.
- World/radar effects that influence deterministic entity lifetime are
  frame-based. Pure renderer sparkles or UI fades may use app time, but their
  clock must be outside `Simulation`.

### Persisted format boundary

Backward reinterpretation is explicitly rejected.

- Replay format becomes version 2 and records an explicit
  `NativeMainTickV1` clock contract instead of `tick_hz=45`.
- Replay version 1 is rejected with a descriptive unsupported-clock error.
  Commands are not silently rescheduled and old hashes are not compared under
  the new contract.
- Snapshot format increments from version 30 to 31 because serialized session
  layout and deterministic behavior change.
- Tactical-capture/profile/evidence contracts bump their schema and replace
  `exact_step_hz=45` / `sim_tick_ms=22` with an explicit native-frame-step
  contract.
- Exact-step receipts contain the ordinal and native-frame before/after values.
  Validation requires ordinal delta `1`, wrapping native-frame delta `1`, and
  no residual pacing debt.
- State hashes remove `total_sim_ms`, replace `binary_frame` with
  `native_frame`, and retain the step ordinal.

## Tiny-Detail Ledger

1. **[MILESTONE]** First admitted frame executes with `native_frame=0`.
2. **[MILESTONE]** `native_frame` is constant throughout gameplay work.
3. **[MILESTONE]** A timer started and checked in the same frame has elapsed
   zero.
4. **[MILESTONE]** The normal frame tail increments the native counter exactly
   once with `wrapping_add(1)`.
5. **[MILESTONE]** The established frame-commit-before-pending-delete order is
   preserved.
6. **[MILESTONE]** One outer local app iteration admits at most one gameplay
   frame.
7. **[MILESTONE]** Local speed bytes `1..6` use `speed * 16 ms` minimum
   frame-start spacing; byte `0` has no imposed wait.
8. **[MILESTONE]** Changing `GameSpeed` cannot alter per-frame displacement,
   cooldown decrement, or RNG call order. A replay recorded under one speed
   hashes deterministically; verified normalized animation state may differ
   across speeds because native explicitly normalizes its frame delay.
9. **[MILESTONE]** Exact step always executes one frame regardless of
   `GameSpeed`.
10. **[MILESTONE]** No deterministic sim API accepts `tick_ms` after the
    authority flip.
11. **[MILESTONE]** No ordinary movement mechanism uses
    `dt_from_tick_ms`.
12. **[MILESTONE]** Drive has no scheduler-imposed frozen pair of subticks
    between budget opportunities.
13. **[MILESTONE]** Drive retry masking, residual ownership, strict
    `budget > 7`, and fresh-track zero-budget behavior are unchanged.
14. **[MILESTONE]** Positive weapon ROF is represented in frames without a
    runtime milliseconds round trip.
15. **[COMPOUNDING]** Per-frame integer/residual truncation occurs at the same
    mechanism boundary as native; the global change does not introduce
    float/delta accumulation.
16. **[COMPOUNDING]** Per-pass counters such as `blocked_delay` keep one
    decrement per frame and are not multiplied by three.
17. **[COMPOUNDING]** Offline Options/pause executes no frame and commits no
    counter.
18. **[COMPOUNDING]** Repeated renders of one committed frame do not advance
    deterministic animation/effects.
19. **[COMPOUNDING]** App wall-clock state is neither serialized nor hashed.
20. **[COMPOUNDING]** Gameplay uses `native_frame`; the `tick` ordinal is not
    accepted as an elapsed-time shortcut.
21. **[FORMAT]** Replay v1, snapshot v30, and the old tactical capture clock
    schema are rejected rather than guessed.
22. **[EXACTIFICATION]** Speed byte `0` achieved rate depends on host workload
    and render throughput.
23. **[EXACTIFICATION]** Retail default achieved jitter remains unmeasured;
    the static one-bucket mechanism is verified.
24. **[UNKNOWN-RISK]** Each existing Rust-authored millisecond effect/animation
    constant needs an owner/evidence classification before conversion.
25. **[UNKNOWN-RISK]** Each Rust-authored `*_TICKS` heuristic needs an explicit
    retained-behavior or native-parity decision before the global flip.

## Design

### Components

#### `LocalFramePacer` (app layer)

An app-only pacing component owns:

- current stored local game-speed byte;
- frame-start timestamp/bucket;
- next eligible frame deadline;
- pause/exact-step re-anchoring.

It owns no deterministic gameplay state and is absent from snapshots/hashes.
Its output is only `admit_zero_or_one_frame`.

#### `ScenarioSession.native_frame`

This replaces `binary_frame` as the authoritative native-width gameplay clock.
CDTimer/RateTimer-style helpers accept it directly and define their wrapping and
inclusive/exclusive comparisons explicitly.

#### `ScenarioSession.tick`

This remains a `u64` step ordinal. Command scheduling and replay entry indices
may use it. Gameplay cadence code may not.

#### Per-frame conversion helpers

`fixed_math` exposes units that state their frame basis, for example a
per-frame lepton conversion. Helpers named `*_per_second` are not used by
deterministic movement unless the subsystem truly consumes a real-time value
at a load boundary.

No replacement global constant such as `SIM_TICK_HZ=63` is introduced: native
logic has a variable wall-clock rate.

#### Clock-consumer census

The implementation plan must carry a checked census of:

- every `tick_ms`, `total_sim_ms`, `binary_frame`, `SIM_TICK_HZ`, and
  `SIM_TICK_MS` reference;
- every gameplay read of `session.tick`;
- every `*_ticks`, `elapsed_ms`, `duration_ms`, modulo, cooldown, and
  rate field in deterministic state.

Each entry receives one of the four migration classes above and a cited native
claim, retained Rust behavior decision, or explicit blocker. The atomic flip
cannot proceed with unclassified ordinary-play consumers.

### Interfaces / Contracts

```text
App wall clock + local GameSpeed
        |
        v
LocalFramePacer::admit() -> 0 or 1
        |
        v
Simulation::advance_frame(...)
        |
        +-- all gameplay reads native_frame N
        +-- one object/AI/movement/combat/economy pass
        +-- late native_frame commit N -> N+1
        +-- pending delete / ordinal / hash tail
        |
        v
committed deterministic frame for render/replay/capture
```

Contract boundaries:

- App pacing never supplies a delta to the simulation.
- Simulation never reads `Instant`, wall time, or target FPS. It reads local
  `GameSpeed` only through verified native speed-byte consumers such as
  normalized animation delay.
- A simulation call cannot perform zero or several gameplay frames.
- Render may observe committed state multiple times without mutating
  deterministic time.
- Replay/headless/exact-step callers execute explicit frame counts and do not
  emulate wall pacing.

### Data Flow

For a normal default-speed local frame:

1. App records frame start and sees that one 16 ms bucket budget is eligible.
2. Commands due for ordinal `tick + 1` are collected.
3. `advance_frame` begins under `native_frame=N`.
4. Object AI, movement, combat, production, timers, animation, and effects each
   receive exactly one frame opportunity in their established order.
5. Locomotion consumes per-frame budgets; no clock scaling occurs inside it.
6. The normal tail commits `native_frame=N+1`.
7. Pending-delete and the established end-of-step tail run.
8. The step ordinal becomes `tick+1`; the committed state hash is recorded.
9. App renders the committed state and services presentation work.
10. If the frame completed before the budget expired, the app waits until the
    frame-start bucket has advanced by one. Otherwise the next outer iteration
    is immediately eligible.

### Error Handling

- Loading an old replay, snapshot, or capture contract returns an explicit
  version/clock mismatch; no compatibility shim guesses timing.
- Exact-step validation fails if either the ordinal or native frame fails to
  advance exactly once, or if pacing debt remains.
- Debug builds assert the post-frame relation between step ordinal and native
  frame modulo `u32` for ordinary completed frames.
- A deterministic timing consumer left in milliseconds or using the ordinal as
  elapsed gameplay time fails the implementation census/review gate.
- Invalid local stored speed values are clamped only at the existing UI/session
  boundary; the pacer receives the canonical `0..6` value.

### Testing Strategy

#### Scheduler

- speed `1..6` yields the verified `16, 32, 48, 64, 80, 96 ms` minimum
  frame-start periods;
- speed `0` imposes no deadline;
- one app iteration admits at most one frame, including after a long stall;
- a frame whose work exceeds its budget allows the next outer iteration
  immediately without accumulated catch-up;
- offline pause admits zero frames;
- exact-step admits exactly one frame at every speed and clears/re-anchors
  pacing state.

#### Frame contract

- fresh simulation begins at native frame `0`;
- all within-frame probes observe the same frame;
- same-frame timer start/check yields elapsed `0`;
- normal tail commits one wrapping increment;
- frame commit remains before pending-delete processing;
- step ordinal increments once and the end-of-step hash sees committed values.

#### Locomotion

- stock open-ground Walk applies the verified integer per-frame budget on every
  eligible consecutive frame;
- stock open-ground MTNK/Drive has a budget opportunity on every eligible
  consecutive frame with no two frozen subticks;
- fresh Drive track with zero budget preserves center/facing;
- Drive budget exactly `7` consumes no point;
- same-Process track chaining repeats speed-state work but contributes no
  second fresh integer budget;
- blocker delay decrements once per frame and is not rescaled by three;
- Walk, Drive, Hover, Ship, Fly/Jumpjet, Rocket, Homing, Tunnel, DropPod,
  Parachute, and Teleport tests contain no runtime-delta assumption.

The three 2026-07-28 traces become regression scenarios:

- `docs/research/traces/MTNK_OPEN_GROUND_DRIVE_FEEL_RETRACE_20260728.md`
- `docs/research/traces/E2_STATIC_WALL_WALK_FEEL_RETRACE_20260728.md`
- `docs/research/traces/MTNK_FRIENDLY_LOOKAHEAD_BLOCKER_RETRACE_20260728.md`

The blocker scenario specifically guards against the false fix of multiplying
or dividing a once-per-frame delay solely from the obsolete 15 Hz
interpretation.

#### Cross-system timing

- positive `ROF=N` seeds/decrements an N-frame cooldown according to the
  subsystem's verified boundary semantics;
- facing, gate, miner, ore, production, superweapon, and deterministic
  animation tests use the same pre-increment native frame;
- changing local `GameSpeed` leaves locomotion, ROF, production, and RNG
  frame-indexed outcomes unchanged; a separate normalized-animation test proves
  the verified speed-dependent frame-delay exception;
- repeated render calls without an admitted frame leave all deterministic
  animation/effect state unchanged.

#### Persisted contracts

- replay v2 round-trips with `NativeMainTickV1`;
- replay v1 is rejected with a descriptive clock-contract error;
- snapshot v31 round-trips; snapshot v30 is rejected;
- new tactical capture profile/evidence round-trips and the old 45/22 schema is
  rejected;
- exact-step receipt verifies ordinal `+1`, wrapping native frame `+1`, and
  empty pacing debt;
- hash tests prove removal of `total_sim_ms` and inclusion of `native_frame`.

#### Validation tiers

During implementation, use scoped `cargo test -p vera20k --lib <module/filter>`
tests only after checking that no other session owns Cargo. Run the full
`cargo test -p vera20k --lib` exactly once at the merge/certification boundary,
per `ENGINE.md`.

### Determinism

- Wall time decides only whether the local app admits the next frame; it never
  changes the contents of an admitted frame.
- Headless, replay, exact-step, and future lockstep execution advance explicit
  integer frame counts without wall time.
- All gameplay progression uses integer/fixed per-frame arithmetic.
- The native frame is hashed and serialized with native `u32` wrapping
  semantics.
- `GameSpeed` remains deterministic match/session metadata. It cannot influence
  per-frame state transitions unless a verified native mechanism explicitly
  reads it; normalized `AnimClass` delay is the named exception.
- No unordered collection or floating-point delta is added to a deterministic
  hot path.

## Architectural Decisions

1. **Global, not locomotion-local.** The clock authority flips atomically across
   ordinary gameplay systems.
2. **One step is one native frame.** There are no hidden 3× substeps.
3. **Variable wall rate, no simulator Hz.** The sim has no `15`, `45`, or `63`
   Hz runtime constant; local pacing uses the verified speed-byte bucket.
4. **One outer iteration, at most one frame.** Slow hosts lose wall throughput
   rather than batching several complete native frames behind one render/input
   opportunity.
5. **Native frame is gameplay authority.** The u64 tick survives only as an
   ordinal.
6. **Milliseconds stay outside deterministic gameplay.** Subsystem-specific
   authored values are converted only when native evidence proves the
   conversion. Normalized `AnimClass` may recompute its frame delay when the
   deterministic speed byte changes, matching the native helper.
7. **Late commit is preserved.** The current frame remains pre-increment-visible
   throughout gameplay.
8. **Existing movement ownership is not broadened.** The timebase correction
   removes cadence drift but does not perform the approved per-object
   ground-movement authority flip.
9. **Old timing artifacts are intentionally incompatible.** Replay, snapshot,
   and tactical-capture formats receive explicit version boundaries.
10. **No render interpolation prerequisite.** Retail itself advances/renderers
    per reached frame. Interpolation may be designed later if visual sampling
    proves necessary, but it may not create deterministic substeps.

## Alternatives Considered

### Locomotion-only per-frame multiplier

Rejected. It would make units move faster while weapons, production, timers,
air/special locomotors, deterministic animation, replay stepping, and game
speed retained the obsolete clock. The first ordinary combined
move/attack/harvest loop would expose the mismatch.

### Retain three substeps and run about 187.5 steps/second

Rejected. Native has no such substep loop. It would triple scheduler, AI, RNG,
collision, and hot-path cost unless every non-movement system gained another
gate, recreating two clocks and making 20k-scale performance substantially
worse.

### Set `SIM_TICK_HZ` to 63

Rejected. 62.5/s is only the nominal default local cap. Native runtime rate
varies with `GameSpeed` and workload; a fixed global simulator Hz would encode
the same category error under a new number.

### Preserve accumulated multi-frame catch-up

Rejected for the local native path. Native performs one `Main_Tick` per outer
iteration with render/service work between iterations. Batching several full
frames changes input visibility, presentation ordering, and workload behavior.

### Keep `total_sim_ms` as deterministic compatibility state

Rejected. It is neither real wall time nor a native gameplay clock, and keeping
it invites future consumers to reconstruct the obsolete 15/45 model. Renderer
cosmetics must use native-frame state or app-owned wall time.

### Translate replay v1 on load

Rejected by user-approved compatibility policy. The old format's commands and
hashes were recorded under a different timing contract; silent translation
would imply equivalence that does not exist.

## Non-Goals

- Implement or activate the approved per-object ground-movement ownership
  migration.
- Fix Drive track cursor, cell-handoff, occupancy, pathfinding, or arrival
  disparities unrelated to cadence.
- Add render interpolation or deterministic movement substeps.
- Claim measured retail FPS/jitter that has not been captured dynamically.
- Implement the separate network pacing/budget path.
- Exactify every Rust AI heuristic or cosmetic animation as part of
  brainstorming; each must be classified before the implementation authority
  flip.
- Preserve replay v1, snapshot v30, or the old tactical-capture 45/22 contract.
- Change native frame widths, strict Drive budget comparisons, residual
  ownership, RNG order, or current phase ordering except where the clock input
  itself is removed.
