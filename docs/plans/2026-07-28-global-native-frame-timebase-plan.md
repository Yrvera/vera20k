# Global Native-Frame Timebase Implementation Plan

> **For Codex:** Execute this plan task-by-task. Tasks 2 through 7 are one atomic
> production-authority batch: do not run, test, commit, or hand off the game between
> those tasks. Task 8 is the first validation point after the batch.

**Goal:** Replace the hybrid 45 Hz / millisecond gameplay clock with one admitted
simulation step per native `gamemd.exe` gameplay frame, while keeping wall-clock
pacing in the app layer and preserving deterministic command, lifecycle, replay,
snapshot, and capture contracts.

**Architecture:** `LocalFramePacer` owns local wall-clock admission and admits zero
or one frame per outer app iteration. `Simulation::advance_frame` owns exactly one
deterministic gameplay pass under `ScenarioSession::native_frame`, commits that
wrapping counter late, then drains pending deletion and commits the monotonic
step ordinal. All ordinary gameplay consumers become frame-native in the same
production batch; presentation-only clocks remain outside `sim/`.

**Design Doc:** `docs/plans/2026-07-28-global-native-frame-timebase-design.md`

**Status:** Corrected after plan review; implementation has not started.

---

## Grounding Summary

- `Main_Tick @ 0x0055D360` performs one gameplay pass and increments
  `g_CurrentFrameCounter` once near its tail. Gameplay consumers see the
  pre-increment value.
- Live binary inspection confirms the ordinary tail order is frame commit, then
  `FUN_00725C70` pending-delete processing. Current Rust already mirrors this
  relative order in `src/sim/world/mod.rs`: synthetic frame commit precedes
  `process_pending_delete()`, and the `tick` ordinal follows.
- `DriveLocomotionClass::Process @ 0x004B0500` has no independent 15 Hz or
  every-third-call gate. A native object pass calls locomotion once per reached
  gameplay frame.
- `GetRadarTimer @ 0x006C8C40` returns `timeGetTime() >> 4`. Stored local speeds
  `1..6` require that many 16 ms buckets; speed `0` is uncapped.
- `AnimClass::AI @ 0x00423AC0`, `TickRadarEvent @ 0x0065FE00`, and normalized
  delay helper `0x005FB2E0` are frame-based. Normalization is the one verified
  deterministic gameplay-side use of the local speed byte.
- Infantry art `Sequence=` entries supply frame ranges, but cadence comes from
  the binary action-delay table and `ActionTimer`. The table has 42 action
  records and only actions `{0x09,0x0A,0x12,0x13,0x17,0x20}` normalize their
  delays.
- SHP vehicle bodies use `BodyFrameCounter` with literal `WalkRate` and
  `IdleRate` modulo gates; `WalkFrames` and `FiringFrames` are layout counts,
  not time conversions.
- Fly, Hover, Jumpjet, Rocket, Ship, Teleport, homing projectile, parachute
  descent, miner, and refinery-unload reports all establish one reached-frame
  state transitions. Mech, DropPod, and Tunnel are dormant TS families in stock
  YR and do not block ordinary skirmish, though retained Rust extensions must
  still stop accepting `dt_ms`.
- The current Rust authority is split across `SIM_TICK_HZ=45`,
  `SIM_TICK_MS=22`, `ScenarioSession.total_sim_ms`, a derived
  `binary_frame`, per-call movement work, millisecond animation/effect fields,
  and a multi-step app accumulator. A local Drive-only edit would therefore
  make locomotion run about three times as often relative to unconverted ROF and
  timer consumers.
- The current command contract is `CommandEnvelope { owner, execute_tick,
  payload }`; replay ticks also carry `state_hash`. Both must survive the API
  migration intact.
- `rulesmd.ini` supplies radar-event duration arrays and four scalar knobs, but
  the binary parses the arrays without copying them into the live type table.
  The verified 17-row compiled table controls dedup, visibility, blink duration,
  and uniqueness; only `RadarEventMinRadius`, `RadarEventSpeed`,
  `RadarEventRotationSpeed`, and `RadarEventColorSpeed` are active scalar INI
  inputs. `artmd.ini` supplies `Rate`, `RandomRate`, `Normalized`, `WalkRate`,
  `IdleRate`, `WalkFrames`, `FiringFrames`, and infantry `Sequence` data. No
  plan task may replace active INI data with guessed constants or activate a
  parsed-but-dead native key.
- Remaining uncertainties are bounded exactification residuals about achieved
  wall-rate jitter, dormant extension behavior, and parachute ownership. None
  changes the one-frame authority or blocks an ordinary stock-skirmish flip.

## Key Technical Decisions

- **One admitted simulation call is one native gameplay frame.**
  `advance_frame` has no `tick_ms`, delta-seconds, target-Hz, or pacing argument.
  — **Confidence:** high
  - **Source:** `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`;
    `MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md`;
    live `Main_Tick @ 0x0055D360`.

- **The production migration is atomic across all deterministic consumers and
  persisted contracts.** Inert pure helpers may land first, but no executable
  production path may combine frame-native locomotion with 45 Hz ROF/timers or
  old serialized layouts.
  — **Confidence:** high
  - **Source:** approved design architecture and current Rust consumer census.

- **`ScenarioSession::native_frame: u32` is the gameplay-time authority;
  `tick: u64` remains only the monotonic command/replay ordinal.**
  — **Confidence:** high
  - **Source:** current `ScenarioSession`, `CommandEnvelope`, and pre-increment
    native frame behavior.

- **The ordinary tail is gameplay under frame `N`, wrapping commit to `N+1`,
  pending-delete drain, then ordinal/hash/receipt publication.**
  — **Confidence:** high
  - **Source:** live `Main_Tick @ 0x0055D360`; current
    `src/sim/world/mod.rs` late region.

- **The app never catches up by batching simulation frames.** Each outer
  iteration admits zero or one frame. A long stall produces lower wall
  throughput, not a burst of hidden gameplay.
  — **Confidence:** high
  - **Source:** `NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION_GHIDRA_REPORT.md`;
    `TIMEGETTIME_WALLCLOCK_VS_FRAMECLOCK_SPLIT_GHIDRA_REPORT.md`;
    live `GetRadarTimer @ 0x006C8C40`.

- **Exact-step, replay, and headless callers bypass wall pacing and execute one
  explicit frame per call.** Exact step also re-anchors the local pacer so the
  next normal iteration does not consume stale elapsed wall time.
  — **Confidence:** high
  - **Source:** approved design and current exact-step ownership in
    `src/app_sim_tick.rs`.

- **Locomotion uses one current per-frame budget on every call.** This plan
  removes cadence/delta scaling but does not broaden movement ownership or
  exactify every speed modifier and x87 rounding detail.
  — **Confidence:** high for cadence; medium for retained non-stock extension
  mechanics
  - **Source:** `OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`;
    `GROUND_MOVEMENT_LIFECYCLE_EFFECT_OWNERSHIP_GHIDRA_REPORT.md`; locomotor
    family reports listed below.

- **Infantry and SHP vehicle animation do not use the generic guessed
  millisecond sequence cadence.** Infantry receives a frame-based action timer
  and exact 42-byte delay vector; SHP bodies receive literal rate modulo gates.
  — **Confidence:** high
  - **Source:** `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`;
    `TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md`; live bytes at
    `0x007EAF7C..0x007EB023`.

- **Radar event lifetime uses the verified compiled 17-row type table, not the
  six parsed duration arrays.** The four active scalar INI values remain rules
  inputs. Persistent float32 radar state uses `NativeF32Bits` and the existing
  `X87Chop53` pattern; rendering converts committed bits to presentation floats.
  — **Confidence:** high
  - **Source:** `RADAR_EVENT_CLASS_GHIDRA_REPORT.md`; current deterministic
    native-float pattern in `src/util/native_x87.rs`.

- **Replay v2 retains `Vec<CommandEnvelope>` and `state_hash`, adds the speed
  byte per frame, and declares `NativeMainTickV1` instead of `tick_hz`.**
  Snapshot becomes v31 and tactical-capture/profile/evidence schemas become v2;
  old formats are rejected.
  — **Confidence:** high
  - **Source:** approved design; current `replay.rs`, `snapshot.rs`, and
    tactical-capture schemas.

- **Deterministic sim math remains integer/fixed-point.** No wall `Instant`,
  `f32`, or `f64` enters hashed gameplay state.
  — **Confidence:** high
  - **Source:** `ENGINE.md` deterministic architecture and current sim boundary.

## Open Questions

### Resolved During Planning

- **Does the native Drive path have its own 15 Hz gate?** No.
  `DriveLocomotionClass::Process @ 0x004B0500` runs when reached by the object
  pass and contains no independent every-third-call cadence gate.

- **Does opening offline Options continue gameplay behind the modal?** No for
  the standard local path. Outer pump modes `0` and `5` do not enter
  `Main_Tick`; pause is modeled by admitting no frame.

- **Should commands become `Vec<Command>` in replay or `advance_frame`?** No.
  Preserve `CommandEnvelope` so owner and deterministic `execute_tick` metadata
  are not lost.

- **Does pending deletion happen before the native-frame increment?** No on the
  ordinary committed path. The frame counter increments first, pending deletion
  follows, and Rust's monotonic ordinal remains after that drain.

- **Can infantry cadence use `Rate=` or a guessed frames-to-ms conversion?** No.
  Use the binary action-delay vector:

  ```text
  action 00..29 delays =
  [0,0,6,3,1,1,1,1,1,3,3,1,1,1,1,1,3,1,3,3,1,1,1,2,1,1,1,1,1,1,1,1,3,1,3,1,3,4,6,3,1,1]
  ```

  Only action IDs `09,0A,12,13,17,20` pass through native normalization.

- **Do DropPod, Tunnel, and Mech need new stock-parity investigation before the
  flip?** No. They are dormant TS families with no active stock YR INI
  references. Retained Rust extension paths are mechanically made one-call-per-
  frame and remain explicitly non-stock.

- **Does a paradropped infantryman switch to a parachute locomotor?** No in the
  native ordinary path. It remains the infantry object with a per-frame falling
  state while `PARACH` is an attached `AnimClass`.

- **Should the six radar duration arrays drive live event timing?** No. YR
  parses them into `RulesClass`, but the live table at `0x007F0998` has zero
  writers and remains the compiled 17-row configuration. The arrays may be
  retained as inert rules metadata only if another consumer already needs them;
  this slice must not wire them into runtime behavior.

### Deferred to Implementation

- **What wall throughput does speed `0` achieve?** It is deliberately workload
  limited. Verify only that no deadline is imposed and no catch-up batch occurs;
  an exact FPS target would contradict the binary contract.

- **What is retail's host-specific timing jitter at default speed?** The static
  one-bucket mechanism is verified. Runtime observation may characterize jitter,
  but it cannot change the admission algorithm.

- **Should retained non-stock DropPod/Tunnel/Mech extensions later be
  exactified?** Trigger: custom content uses those Rust extensions. Frequency:
  never in ordinary stock skirmish. Player effect: custom mover cadence may
  still differ beyond removal of `dt_ms`. Downstream risk: isolated to non-stock
  content; record as a residual, not a blocker.

- **Should parachute ownership be corrected in this slice?** Only if removing
  `dt_ms` cannot preserve the current path without a second clock. Otherwise
  make the existing descent transition once per frame and record the separate
  ownership disparity. Trigger: paradrop. Frequency: occasional. Player effect:
  descent details can differ; ordinary global time authority remains correct.

## File Map

The implementation executor must refresh this map after Task 0 because the
shared checkout is currently dirty. These are real current paths, not the stale
paths from the superseded plan.

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/app_frame_pacer.rs` | App-only zero-or-one frame admission and re-anchoring |
| Modify | `src/lib.rs` | Export the app pacer module |
| Modify | `src/app_types.rs` | Remove app aliases for 45 Hz / 22 ms gameplay constants |
| Modify | `src/app_sim_tick.rs` | Replace accumulator/catch-up with one-frame admission; migrate exact receipts and callers |
| Modify | `src/app.rs`, `src/app_init.rs` | Store/init the app-only pacer and event-loop deadline |
| Modify | `src/app_in_game_options_input.rs` | Apply speed changes at a frame boundary and re-anchor pacing |
| Modify | `src/app_building_anim.rs`, `src/app_fire_effects.rs`, `src/app_chute_anim.rs` | Separate native deterministic animation from presentation-only wall time |
| Modify | `src/app_instances/mod.rs`, `src/app_instances/shp.rs`, `src/app_instances/units.rs`, `src/app_instances/particles.rs` | Render committed frame state without mutating deterministic time |
| Modify | `src/sim/scenario_session.rs` | Replace `total_sim_ms`/`binary_frame` with `native_frame` |
| Modify | `src/sim/world/mod.rs` | Rename/reshape the one-pass API and preserve late tail order |
| Modify | `src/sim/world/world_hash.rs` | Hash `native_frame` and frame-native serialized state |
| Modify | `src/sim/command.rs`, `src/sim/world/world_commands.rs`, `src/sim/world/world_orders.rs` | Preserve ordinal command scheduling under `advance_frame` |
| Modify | `src/sim/movement/mod.rs` and all current files under `src/sim/movement/` that accept `dt_ms`, substeps, or synthetic frame values | Convert ground, air, special, projectile, parachute, and retained extension transitions to one native frame |
| Modify | `src/util/fixed_math.rs` | Remove `SIM_TICK_HZ` and ms/tick gameplay conversions; retain integer per-frame helpers |
| Modify | `src/sim/combat/mod.rs` and affected `src/sim/combat/*` modules | Store/decrement ROF, rearm, pursuit, and combat delays in frames |
| Modify | `src/sim/production/mod.rs` and affected `src/sim/production/*` modules | Convert progress/deadlines from raw 45 Hz ticks/ms to native frames |
| Modify | `src/sim/miner/mod.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/harvest_mission.rs`, `src/sim/miner/miner_dock_sequence.rs` | Convert harvest/load/dump mission cadence to frame units |
| Modify | `src/sim/aircraft/mod.rs` and affected `src/sim/aircraft/*` modules | Convert aircraft mission/rearm/drop cadence to frame units |
| Modify | `src/sim/power_system.rs`, `src/sim/ore_growth.rs`, `src/sim/radar.rs` | Convert subsystem timers and all 17 native radar-event types to frames |
| Modify | `src/sim/superweapon/mod.rs` and affected `src/sim/superweapon/*` modules | Convert charge, duration, and phase timers to native frames |
| Modify | `src/sim/animation.rs`, `src/sim/anim_class.rs`, `src/sim/components.rs` | Replace deterministic ms animation/effect state with owner-specific frame state |
| Modify | `src/rules/art_data.rs`, `src/rules/infantry_sequence.rs`, `src/rules/shp_vehicle_sequence.rs` | Preserve raw art timing data and expose frame-authored fields |
| Modify | `src/rules/radar_event_config.rs`, `src/rules/ruleset.rs` | Parse the four active radar scalar keys without activating dead arrays |
| Modify | `src/sim/replay.rs` | Replay v2 native clock contract, speed samples, preserved envelopes and hashes |
| Modify | `src/sim/snapshot.rs` | Snapshot v31 and explicit v30 rejection |
| Modify | `src/app_tactical_capture_contract.v1.json` (rename to `.v2.json`) | Sealed capture clock contract v2 |
| Modify | `src/app_tactical_capture/profile.rs`, `src/app_tactical_capture/manifest.rs`, `src/app_tactical_capture/session.rs`, `src/app_tactical_capture/evidence.rs`, `src/app_tactical_capture/script.rs` | Replace 45 Hz / 22 ms schema fields and observations |
| Modify | `src/match_bootstrap.rs` and parity/capture digest call sites found by census | Replace synthetic tick-zero/frame formulas with native-frame fields |
| Modify | Existing focused tests adjacent to every path above | Assert frame boundaries, wrapping, order, persistence, and no catch-up |
| Modify if mapped | `system_map/` source files selected by `loop` / `mechanism` lookup | Update only verified timing-authority connections changed by the implementation |

No new catch-all `sim/time.rs` module is created. Owner-specific frame state
stays with the subsystem that consumes it.

## Interface Changes

### Simulation entry point

Replace the current `advance_tick(..., tick_ms) -> TickResult` with:

```rust
pub fn advance_frame(
    &mut self,
    commands: &[CommandEnvelope],
    rules: Option<&RuleSet>,
    height_map: &BTreeMap<(u16, u16), u8>,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
) -> TickResult
```

The result type stays `TickResult` in this slice to avoid an unrelated public
rename. All current app, replay, test, bootstrap, and tool callers are migrated
in the atomic batch. No compatibility wrapper may keep accepting `tick_ms`.

### Session clock

```rust
pub struct ScenarioSession {
    // existing scenario identity/options fields unchanged
    pub tick: u64,
    pub native_frame: u32,
}
```

Delete `total_sim_ms` and `binary_frame`. `native_frame` starts at `0`, is read
throughout one frame, and commits with `wrapping_add(1)` in the late region.

### App pacing

```rust
#[derive(Debug)]
pub(crate) struct LocalFramePacer {
    last_frame_start_bucket: Option<u64>,
}

impl LocalFramePacer {
    pub(crate) fn should_admit(
        &mut self,
        now_ms: u64,
        game_speed: u8,
        paused: bool,
    ) -> bool;

    pub(crate) fn record_admitted_frame(&mut self, frame_start_ms: u64);
    pub(crate) fn reanchor(&mut self, now_ms: u64);
    pub(crate) fn next_deadline_ms(&self, game_speed: u8) -> Option<u64>;
}
```

The bucket is `now_ms >> 4`. For speed `0`, an unpaused iteration is eligible
without a deadline. For speeds `1..6`, eligibility is wrapping-safe bucket
distance from the last admitted frame start. Pause returns false and does not
mutate deterministic state. Exact step calls `advance_frame` directly, then
re-anchors the pacer.

### Native animation helpers

```rust
pub(crate) const fn anim_rate_to_base_delay(rate: i32) -> u32 {
    if rate <= 0 { 0 } else { (900 / rate) as u32 }
}

pub(crate) const fn normalize_anim_delay(base_delay: u32, game_speed: u8) -> u32;
```

`normalize_anim_delay` clamps the speed-table index to `0..=7`, returns zero for
zero delay, uses the verified four small-delay rows, and otherwise computes
`(base_delay * 8) / (speed + 1)` with integer arithmetic. Random-rate selection
happens before normalization.

### Replay v2

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum ReplayClockContract {
    NativeMainTickV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayHeader {
    pub version: u32,
    pub clock: ReplayClockContract,
    pub seed: u64,
    pub map_name: String,
    pub rules_hash: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayTick {
    pub tick: u64,
    pub native_frame: u32,
    pub game_speed: u8,
    pub commands: Vec<CommandEnvelope>,
    pub state_hash: u64,
}
```

Playback validates version `2` and the clock enum, applies the recorded speed
to `session.game_options.game_speed` at the same frame boundary, retains each
envelope's `owner` and `execute_tick`, calls `advance_frame` once, and compares
the recorded hash. Version `1` is rejected with an unsupported-clock error.

### Snapshot and capture

- Snapshot version becomes `31`; v30 is rejected, not upgraded or reinterpreted.
- Tactical capture uses schema strings ending in `.v2`.
- Replace `exact_step_hz` and `sim_tick_ms` with
  `clock_contract: "NativeMainTickV1"` and
  `exact_step_frames_per_call: 1`.
- Replace `total_sim_ms` and `binary_frame` observations with
  `native_frame`.
- Exact receipts contain `tick_before`, `tick_after`,
  `native_frame_before`, and `native_frame_after`; successful receipts require
  both deltas to equal one with wrapping only on the native frame.

## Sim Checklist

- [ ] All deterministic gameplay math is integer/fixed-point; no `f32`/`f64`
  delta enters `sim/`.
- [ ] `native_frame`, frame-unit animation/effect fields, and mutable
  `GameOptions.game_speed` are represented in the deterministic state hash.
- [ ] `sim/` gains no dependency on render, UI, sidebar, audio, net, `Instant`,
  or OS time.
- [ ] `advance_frame` preserves current region order except for the explicitly
  planned late clock commit and removal of synthetic substep/time conversion.
- [ ] All `EntityStore`/`BTreeMap` passes preserve current deterministic
  iteration order and RNG call order.
- [ ] No hot path allocates merely to perform frame conversion.
- [ ] `tick` is used for command/replay ordinals only, never as an elapsed-time
  substitute.
- [ ] Every deterministic `*_ms`, raw per-45-Hz decrement, modulo, and synthetic
  frame consumer is classified before Task 7 closes.

## Risk Areas

- **Shared dirty checkout:** `dev` currently contains broad uncommitted
  startup/loading/RMG work, including overlapping `app.rs`, `app_init.rs`,
  `app_sim_tick.rs`, and `lib.rs`; Cargo is also currently owned by another
  process. Task 0 must reconcile actual state before any production edit.
- **Hybrid production authority:** removing Drive's gate before ROF/timers and
  app pacing flips would create roughly three locomotion opportunities for each
  still-converted legacy cadence. Tasks 2–7 therefore form one non-runnable,
  non-committable batch.
- **Persisted layout mismatch:** `SequenceDef`, `Animation`,
  `DriveLocomotionRuntime`, and session fields serialize today. Their layout may
  change only in the same atomic batch that bumps snapshot/replay/capture
  contracts.
- **Command metadata loss:** simplifying call sites to `Command` would discard
  owner and scheduled ordinal. Cross-system replay tests must exercise delayed
  envelopes.
- **Late lifecycle order:** pending deletion must remain after the native frame
  commit. Mid-pass death/removal paths and hash publication are sensitive to
  this order.
- **Animation owner collapse:** native `AnimClass`, infantry action timers, SHP
  body counters, and retained generic/non-native sequences are distinct timing
  domains. A generic frames-to-ms or universal 15 fps conversion is forbidden.
- **Runtime speed changes:** normalized animation legitimately changes cadence
  from the deterministic speed byte. Replay must sample the byte per frame and
  the hash must include session options.
- **Capture contract blast radius:** tactical capture currently seals exact
  `45` Hz / `22` ms assertions in profile, session, evidence, and bootstrap
  checks. All schema consumers must migrate together.
- **Terminal non-commit path:** the approved design requires an eventual
  session-end flag to skip both frame commit and pending-delete drain. If no
  terminal latch exists in the current world API, do not invent one in this
  slice; preserve ordinary order and record the terminal branch as a residual.

## Player-Experience Critical Items

Representative scenario: a default-speed stock skirmish with a Grizzly/Rhino
ground move and turn, infantry walk/fire/death sequences, a Terror Drone or
Dolphin SHP body, a Harrier or Rocketeer/Jumpjet mover, ore harvesting and
refinery unload, production, radar events, superweapon charge, Options pause,
save/load, replay, and one exact tactical-capture step.

| Task | Class | Item | Why it matters | Verification |
|---|---|---|---|---|
| 2–7 | MILESTONE-BLOCKING | One global frame authority | Prevents movement, fire, production, and effects from running at contradictory rates every match | Static clock census plus cross-system frame test |
| 2–7 | MILESTONE-BLOCKING | Preserve command envelopes and late lifecycle order | Avoids delayed commands executing on wrong frames and corpses disappearing at the wrong boundary | Replay delayed-command test and pending-delete order test |
| 2–7 | MILESTONE-BLOCKING | Atomic persisted-format break | Stops old saves/replays/captures from being silently interpreted under new timing | Explicit rejection tests |
| 4 | COMPOUNDING | One locomotor transition per frame | Movement feel is continuously visible and was the reported symptom | Ground/air/special displacement and blocker-delay tests |
| 5 | COMPOUNDING | ROF, production, miner, timers | These systems repeatedly interact with motion and expose global cadence drift | Stock combat/economy frame-count tests |
| 6 | COMPOUNDING | Infantry, SHP, `AnimClass`, radar cadence | Wrong visual/action cadence appears continuously and can alter fire/death timing | Action table, modulo, normalized-delay, and side-effect order tests |
| 7 | COMPOUNDING | No catch-up; pause freezes | Bursts after a stall and gameplay behind Options feel immediately wrong | Pacer and paused-session tests |
| 9 | EXACTIFICATION | Speed-0 achieved FPS and host jitter | Workload-dependent; does not change deterministic per-frame results | Observe no imposed deadline; record measured throughput only |
| 4 | EXACTIFICATION | Non-stock TS locomotor mechanics | Absent in stock ordinary play; only cadence wrapper is in scope | Compile/unit coverage and residual note |
| 4 | UNKNOWN-RISK | Parachute ownership disparity | Paradrops are ordinary but the current Rust owner differs from native | Ensure one frame descent update; focused paradrop trace if restructuring becomes necessary |

---

## Tasks

### Task 0: Reconcile ownership and freeze the execution boundary

**Why:** The shared `dev` checkout and Cargo are currently owned by other work.
Actual repository/process state must be safe before touching overlapping code.

**Files:** Read-only; do not modify the plan or production code in this task.

**Pattern:** `ENGINE.md` shared-checkout and Cargo ownership rules.

**Steps:**

1. Run:

   ```powershell
   git status --short
   git branch --show-current
   git worktree list
   git log --oneline -10 -- src/app_sim_tick.rs src/sim/world/mod.rs src/sim/scenario_session.rs
   Get-Process cargo,rustc -ErrorAction SilentlyContinue |
       Select-Object ProcessName,Id,StartTime
   ```

2. Confirm whether another session owns `dev`, any path in the File Map, or
   Cargo. Do not infer ownership from this document's dated snapshot.
3. If another task still owns overlapping paths or Cargo, wait or coordinate
   that exact overlap. Do not edit around active hunks and do not terminate its
   processes.
4. When sole ownership is established, work directly on `dev` as required by
   `AGENTS.md`. Preserve all unrelated dirty files.
5. Record the literal preflight output in the eventual handoff. Do not stage,
   commit, or create a branch unless the user separately authorizes it or
   concurrent ownership requires a worktree.

**Verify:** Production edits may begin only when overlapping ownership is clear
and no other Cargo/rustc process is active.

### Task 1: Add and test inert frame-native primitives

**Why:** Pure app pacing, normalization, and raw parser helpers can be proven
without changing production simulation authority or serialized runtime layout.

**Files:**

- Create: `src/app_frame_pacer.rs`
- Modify: `src/lib.rs`
- Modify: `src/rules/art_data.rs`
- Modify: `src/rules/radar_event_config.rs`
- Modify: `src/rules/ruleset.rs`
- Modify: adjacent unit-test modules only

**Pattern:** App-owned wall time in current `app_sim_tick.rs`; rules parsers
remain data-only and simulator-independent.

**Steps:**

1. Implement `LocalFramePacer` with the exact interface in **Interface
   Changes**. Unit tests must cover:
   - first unpaused call admits one frame;
   - speed `1` requires one 16 ms bucket advance;
   - speeds `2..6` require exactly their bucket counts;
   - speed `0` is always eligible when unpaused;
   - pause admits zero and does not consume a deadline;
   - a 10-second stall still produces one boolean admission, never a count;
   - `reanchor` makes the next timed frame wait from the new bucket;
   - `next_deadline_ms` is absent for speed `0` and exact for `1..6`.
2. Add `anim_rate_to_base_delay` and `normalize_anim_delay` as pure integer
   helpers. Tests must assert zero/negative rates, truncating `900 / Rate`, all
   four verified small-delay rows for speed indices `0..7`, the large-delay
   formula, and speed-byte clamping.
3. Preserve raw `Rate`, `RandomRate`, and `Normalized` data in art types. Do not
   convert them to milliseconds in the new API. Keep any old production field
   temporarily unchanged until the atomic batch.
4. Remove the invented singular `RadarEventDuration` / 13,000 ms default from
   the new API. Parse the four active `[General]` scalar keys
   `RadarEventMinRadius`, `RadarEventSpeed`, `RadarEventRotationSpeed`, and
   `RadarEventColorSpeed` as their native float32 bit patterns using
   `NativeF32Bits`; do not expose Rust `f32` to deterministic simulation.
5. Add parser tests using stock active scalar values:

   ```text
   RadarEventMinRadius=8
   RadarEventSpeed=1.2
   RadarEventRotationSpeed=.05
   RadarEventColorSpeed=.1
   ```

   Assert the exact `f32::to_bits()` values. Add a fixture with changed duration
   arrays and assert the active scalar config is unchanged.
6. Confirm no new helper is called by a production simulation path yet and no
   serialized runtime struct changed.

**Verify:**

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue
cargo test -p vera20k --lib app_frame_pacer -- --nocapture
cargo test -p vera20k --lib normalize_anim_delay -- --nocapture
cargo test -p vera20k --lib radar_event_config -- --nocapture
```

Expected: focused tests pass; the production game still uses the old authority.

### Task 2: Begin atomic batch — change persisted contracts and public interfaces

**Why:** Interface and schema changes must be defined before consumers, but they
must not be tested, run, committed, or handed off until Tasks 3–7 complete.

**Files:**

- Modify: `src/sim/scenario_session.rs`
- Modify: `src/sim/replay.rs`
- Modify: `src/sim/snapshot.rs`
- Modify: `src/app_tactical_capture_contract.v1.json` (rename to `.v2.json`)
- Modify: `src/app_tactical_capture/profile.rs`
- Modify: `src/app_tactical_capture/manifest.rs`
- Modify: `src/app_tactical_capture/evidence.rs`
- Modify: `src/sim/world/mod.rs`

**Pattern:** Existing explicit snapshot-version rejection and replay header
validation.

**Steps:**

1. Replace session `total_sim_ms` and `binary_frame` with
   `native_frame: u32`, initialized to zero. Keep `tick: u64`.
2. Change the public simulation signature to the exact `advance_frame`
   interface above. Retain `TickResult`; remove `tick_ms` without adding a
   compatibility wrapper.
3. Define replay v2 exactly as shown above. Retain `Vec<CommandEnvelope>` and
   `state_hash`; add `native_frame` and `game_speed` per replay frame.
4. Validate replay version `2` and `ReplayClockContract::NativeMainTickV1`.
   Return a descriptive unsupported-clock error for version `1`; do not
   reschedule commands or compare old hashes.
5. Bump `SNAPSHOT_VERSION` from `30` to `31` and update the version comment to
   name the session clock and frame-unit serialized state. Keep strict mismatch
   rejection and add an explicit v30 rejection fixture.
6. Rename the sealed tactical contract to v2 and replace Hz/ms fields with the
   native clock fields from **Interface Changes**. Update profile, manifest, and
   evidence data types together; old v1 strings must fail validation.
7. Do not run Cargo, the app, a capture, or replay. Continue directly to Task 3.

**Verify:** Static inspection only: every schema declares the native contract
and no new format can be serialized with the old runtime field layout.

### Task 3: Atomic batch — establish the one-frame world clock and tail

**Why:** The simulator must have one late-committed native frame before any
consumer is wired to it.

**Files:**

- Modify: `src/sim/world/mod.rs`
- Modify: `src/sim/world/world_hash.rs`
- Modify: `src/sim/command.rs`
- Modify: `src/sim/world/world_commands.rs`
- Modify: `src/sim/world/world_orders.rs`

**Pattern:** Current `run_late_region` and one end-of-pass
`process_pending_delete()` drain.

**Steps:**

1. Keep `execute_tick = session.tick.saturating_add(1)` as the command ordinal
   for the frame. Continue filtering `CommandEnvelope.execute_tick` against
   that ordinal and preserve envelope owner/payload handling.
2. Replace every synthetic `binary_frame` read in the world spine with the
   pre-increment `session.native_frame`.
3. Remove accumulation/derivation of `total_sim_ms` and `binary_frame`.
4. At the ordinary committed tail, execute exactly:

   ```rust
   self.session.native_frame = self.session.native_frame.wrapping_add(1);
   self.process_pending_delete();
   self.session.tick = execute_tick;
   ```

   Preserve all existing late-region work around these statements unless a
   cited native ordering requires movement. Hash/receipt publication observes
   the committed state.
5. If an existing terminal-session latch already distinguishes a non-committed
   Main-Tick exit, make that branch skip both native-frame commit and pending
   deletion. If no such latch exists, do not create a speculative session-end
   system; preserve ordinary order and record the residual.
6. Replace world-hash inputs `total_sim_ms`/`binary_frame` with
   `native_frame`. Confirm existing `GameOptions` hashing includes
   `game_speed`; add no duplicate speed field.
7. Continue directly to Task 4 without running Cargo or the app.

**Verify:** Static order check only. The one wrapping increment must appear
before the single pending-delete drain, and `tick` must remain the command
ordinal rather than elapsed time.

### Task 4: Atomic batch — convert all locomotion families to one frame

**Why:** Locomotion is the reported symptom, but it can become production-active
only inside the global batch.

**Files:**

- Modify: `src/sim/movement/movement_tick.rs`
- Modify: `src/sim/movement/movement_step.rs`
- Modify: `src/sim/movement/drive_locomotion.rs`
- Modify: `src/sim/movement/drive_track.rs`
- Modify: `src/sim/movement/air_movement.rs`
- Modify: `src/sim/movement/hover.rs`
- Modify: `src/sim/movement/jumpjet_movement.rs`
- Modify: `src/sim/movement/rocket_movement.rs`
- Modify: `src/sim/movement/homing_movement.rs`
- Modify: `src/sim/movement/teleport_movement.rs`
- Modify: `src/sim/movement/parachute_descent.rs`
- Modify: `src/sim/movement/droppod_movement.rs`
- Modify: `src/sim/movement/tunnel_movement.rs`
- Modify: `src/sim/movement/facing_class.rs`
- Modify: `src/sim/movement/turret.rs`
- Modify: any additional file under `src/sim/movement/` found by the Task 7
  census to consume `dt_ms`, tick-rate constants, substep ordinals, or
  synthetic frames
- Modify: `src/util/fixed_math.rs`

**Pattern:** Owner-specific native locomotor `Process` reports; current movement
phase and reservation ownership remain in place.

**Steps:**

1. Remove Drive's every-third-step gate from `movement_step.rs`. Delete its
   cadence counter/phase from serialized `DriveLocomotionRuntime`; do not merely
   pin it to an always-open value.
2. Rename speed conversion helpers so their units are explicit
   `leptons_per_frame`. Preserve current verified stock-unit output values and
   integer residual behavior; remove `SIM_TICK_HZ`, `SIM_TICK_MS`, and
   frames-to-substeps/ms helpers.
3. Execute one ground movement budget and one blocker-delay decrement per
   admitted frame. Preserve reservation, bridge, occupancy, crush, arrival,
   track, and effect ownership/order.
4. Convert Fly to one frame of speed acceleration and per-frame position;
   Hover to one frame of speed ramp and XY budget; Jumpjet to one current state
   transition; Rocket to one phase/timer/position/turn transition; Ship to one
   drive/slope transition.
5. Convert homing projectile steering/position to one Bullet AI frame and
   Teleport/chrono delays to literal wrapping frame timers.
6. Convert the current parachute descent state to one frame per call and use
   the native `0,-1,-2,-3` initial fall-rate ramp with the rules clamp where the
   existing data path supports it. Do not preserve a millisecond accumulator.
   Do not broaden into attached-`PARACH` lifecycle unless required to eliminate
   a second gameplay clock.
7. Convert retained DropPod/Tunnel/Mech-style Rust extensions mechanically to
   one transition per frame. Label tests/comments as retained non-stock
   behavior; make no stock-parity claim.
8. Convert facing/turret `RateTimer` consumers to pre-increment
   `native_frame`. Preserve wrapping comparisons and integer interpolation.
9. Delete movement `dt_ms`, substep-count, and synthetic-frame parameters all
   the way through call sites. Do not add floats or per-frame allocations.
10. Continue directly to Task 5 without running Cargo or the app.

**Verify:** Static census only: no movement production signature accepts
`dt_ms`, `SIM_TICK_MS`, `SIM_TICK_HZ`, or a 45 Hz substep index.

### Task 5: Atomic batch — convert combat, economy, AI, and subsystem timers

**Why:** These ordinary systems must share locomotion's frame cadence before the
new clock is executable.

**Files:**

- Modify: affected `src/sim/combat/*`
- Modify: affected `src/sim/production/*`
- Modify: `src/sim/miner/miner_system.rs`
- Modify: `src/sim/miner/harvest_mission.rs`
- Modify: `src/sim/miner/miner_dock_sequence.rs`
- Modify: affected `src/sim/aircraft/*`
- Modify: `src/sim/power_system.rs`
- Modify: `src/sim/ore_growth.rs`
- Modify: affected `src/sim/superweapon/*`
- Modify: affected docking, gate, particle, mission, AI, and world modules
  reported by the Task 7 census

**Pattern:** Existing `MissionTimer` wrapping-frame representation and verified
owner-specific reports; do not build a universal seconds conversion.

**Steps:**

1. Replace ROF/rearm ms/substep conversions with literal frame deadlines or
   countdowns. Decrement once per `advance_frame`; preserve fire decision and
   weapon side-effect order.
2. Convert production progress, placement readiness, factory exits, sell
   progress, power state delays, ore growth/spread, docking, aircraft rearm/drop
   missions, superweapon charge/duration/phase, particle lifetime, AI cadence,
   and mission delay consumers classified as gameplay time.
3. For every raw `session.tick` timer, decide explicitly:
   - keep `tick` only if it is command/replay ordinal metadata;
   - otherwise store/read `native_frame` with wrapping-safe comparisons.
4. Convert harvester loading mission delays to literal frames.
5. Implement refinery unloading with the verified per-frame accumulator:
   stock default `HarvesterDumpRate=.016` produces a `14.4` frame gate from the
   native `*900` rule; use the project's deterministic fixed/integer
   representation, drain one whole storage slot per gate, and retain the
   terminating empty gate. Do not use floating-point sim math.
6. Preserve object iteration and RNG order. Conversion must not add new random
   draws or reorder BTreeMap passes.
7. Continue directly to Task 6 without running Cargo or the app.

**Verify:** Static census only: no ordinary gameplay duration in these owners is
derived from 22 ms, 45 Hz, a synthetic 15 Hz frame, or wall time.

### Task 6: Atomic batch — convert deterministic animation, radar, and effects

**Why:** Native animation contains gameplay-visible and lifecycle side effects;
it cannot remain on milliseconds after the authority flip.

**Files:**

- Modify: `src/sim/anim_class.rs`
- Modify: `src/sim/animation.rs`
- Modify: `src/sim/components.rs`
- Modify: `src/rules/art_data.rs`
- Modify: `src/rules/infantry_sequence.rs`
- Modify: `src/rules/shp_vehicle_sequence.rs`
- Modify: `src/sim/radar.rs`
- Modify: `src/app_building_anim.rs`
- Modify: `src/app_fire_effects.rs`
- Modify: `src/app_chute_anim.rs`
- Modify: affected render-instance readers under `src/app_instances/`

**Pattern:** Existing `AnimStore`/`sim/anim_class.rs` lifecycle owner, infantry
sequence range parser, and SHP body sequence parser.

**Steps:**

1. Represent `AnimClass` rate, random-rate range, last-frame time, frame delay,
   reload, step, loops, and next transitions in native frames. Select a random
   base delay first; apply `normalize_anim_delay` only when
   `Normalized=yes`, using the deterministic session speed byte.
2. Preserve `AnimClass` spawn, owner callback/detach, damage, trailer,
   loop/end, `Next=`, and pending-removal order. Repeated renders of one
   committed frame must not advance it.
3. Replace serialized generic `Animation.tick_ms`/`elapsed_ms` fields with
   frame-unit fields. Retained non-native fallback assets may use an explicit
   `frame_delay`, but ordinary infantry and SHP bodies must not use that
   fallback cadence.
4. Add an infantry action-timer representation with:
   - the exact 42-entry delay vector from **Open Questions**;
   - timer start/duration/reload in `native_frame`;
   - normalization only for action IDs
     `{0x09,0x0A,0x12,0x13,0x17,0x20}`;
   - random-start clamping to at least one frame;
   - completion when `DoingFrame >= sequence.frame_count`;
   - fire gating at the configured action frame;
   - death/special completion routed through the existing removal/pending-delete
     lifecycle rather than immediate raw deletion.
5. Represent SHP `BodyFrameCounter` as a frame counter. Increment moving bodies
   when `native_frame % WalkRate == 0`; use the idle path only when
   `IdleRate != 0` and its modulo matches. Use `WalkFrames`/`FiringFrames` only
   for image-frame layout.
6. Give `RadarEventType` explicit native discriminants and migrate every
   producer/caller:

   ```rust
   #[repr(u8)]
   pub enum RadarEventType {
       Combat = 0,
       Noncombat = 1,
       Dropzone = 2,
       BaseUnderAttack = 3,
       HarvesterUnderAttack = 4,
       EnemyObjectSensed = 5,
       UnitReady = 6,
       UnitLost = 7,
       UnitRepaired = 8,
       SpyInfiltration = 9,
       BuildingCaptured = 10,
       BeaconPlaced = 11,
       ConstructionComplete = 12,
       ImpactSilent = 13,
       BridgeRepaired = 14,
       StructureAbandoned = 15,
       AllyUnderAttack = 16,
   }
   ```

   The snapshot bump covers the enum-layout change. Do not collapse several
   native types into the current seven-variant model.
7. Replace radar event singular/default ms duration with the verified compiled
   17-row table whose rows are
   `{dedup_distance_cells, visibility_duration_frames,
   blink_duration_frames, unique}`:

   ```text
   0:(8,200,400,1)   1:(8,200,400,0)   2:(8,200,400,0)
   3:(8,200,600,1)   4:(8,200,400,1)   5:(6,200,400,1)
   6:(2,0,200,1)     7:(8,0,200,1)     8:(2,0,400,1)
   9:(5,0,400,0)    10:(8,0,100,0)    11:(8,200,200,1)
   12:(8,200,400,0) 13:(8,0,5,0)      14:(8,0,200,1)
   15:(8,0,400,1)   16:(8,200,600,1)
   ```

   Store start/deadline frames and persistent radius, rotation,
   rotation-speed, color-fade, and fade-speed as `NativeF32Bits`, updating them
   once per frame through the existing `X87Chop53` arithmetic pattern. The four
   active scalar INI keys seed those fields; the six dead arrays do not.
   Rendering may convert the committed bit patterns to `f32` outside `sim/`.
8. Add tests asserting all 17 rows, the explicit enum-to-row mapping, active
   draw colors, duration phase boundaries, and that modified duration arrays do
   not alter runtime rows.
9. Replace deterministic world-effect `rate_ms`, `elapsed_ms`, `duration_ms`,
   and age-ms fields with frame delays/ages. Keep presentation-only camera,
   tooltip, message, FPS, chat, and audio-fade wall time outside `sim/`.
10. Make app building/fire/chute modules read committed native animation state
   for gameplay-owned visuals. Any retained app-only cosmetic animation must be
   named and excluded from serialization/hash/capture.
11. Continue directly to Task 7 without running Cargo or the app.

**Verify:** Static owner check only: infantry, SHP body, native `AnimClass`,
radar, and deterministic effect cadence have distinct frame-native owners; none
uses runtime milliseconds.

### Task 7: End atomic batch — wire app admission, replay, capture, and every caller

**Why:** This closes the batch so the new production authority is coherent and
executable for the first time.

**Files:**

- Modify: `src/app.rs`
- Modify: `src/app_init.rs`
- Modify: `src/app_types.rs`
- Modify: `src/app_sim_tick.rs`
- Modify: `src/app_in_game_options_input.rs`
- Modify: `src/lib.rs`
- Modify: `src/sim/replay.rs`
- Modify: `src/app_tactical_capture/session.rs`
- Modify: `src/app_tactical_capture/script.rs`
- Modify: `src/match_bootstrap.rs`
- Modify: every production/test/tool caller found by the required census

**Pattern:** Existing app ownership of wall time and current exact-step direct
simulation path.

**Steps:**

1. Store `LocalFramePacer` in app/runtime state, initialize it from current wall
   time, and remove the fixed-step accumulator, configured TPS scaling,
   `MAX_SIM_STEPS_PER_FRAME`, and catch-up loop.
2. In a normal unpaused outer iteration:
   - sample frame-start wall time;
   - ask the pacer for a boolean admission using the current session speed;
   - call `advance_frame` at most once;
   - record the admitted frame start;
   - continue normal service/render work;
   - expose the next event-loop deadline for speeds `1..6`.
3. Speed `0` has no imposed deadline. If frame work already exceeds the speed
   budget, the next iteration may run immediately, but no missed frames are
   accumulated.
4. Offline Options/pause bypasses admission entirely. Do not call a zero-delta
   simulation frame.
5. Apply a local speed change at a frame boundary, update
   `session.game_options.game_speed`, record it in replay, and re-anchor the
   pacer. Do not pass speed to locomotion/combat/economy calculations.
6. Rewrite `advance_in_game_runtime_exact_step` to call `advance_frame`
   directly regardless of current speed/pause pacing, clear/re-anchor pacing
   afterward, and return the four-field native receipt.
7. Update exact receipt validation:
   - `tick_after == tick_before + 1`;
   - `native_frame_after == native_frame_before.wrapping_add(1)`;
   - no `total_sim_ms` formula exists.
8. Update replay recording/playback to preserve envelopes and hashes, apply
   recorded speed at the frame boundary, call one `advance_frame`, and validate
   the committed `native_frame`.
9. Update tactical capture, bootstrap, profile, evidence, script budgets, and
   sealed tick-zero checks to the v2 native contract. Convert old tick budgets
   only where research establishes literal native frame counts; do not multiply
   by `45/15`.
10. Run a repository-wide classification census before any build:

    ```powershell
    rg -n "SIM_TICK_HZ|SIM_TICK_MS|total_sim_ms|binary_frame|advance_tick|tick_ms|dt_ms|elapsed_ms|duration_ms|rate_ms|configured_tps|MAX_SIM_STEPS_PER_FRAME|accumulator" src tests
    rg -n "session\\.tick|% *[0-9]+|ticks_remaining|cooldown|deadline|timer" src/sim
    ```

11. Classify every hit as:
    - deleted legacy gameplay clock;
    - command/replay ordinal;
    - verified frame-native gameplay state;
    - explicitly app-only cosmetic wall time.

    Ordinary-play ambiguous hits block Task 8. Comments/tests that describe the
    old clock must also be migrated. Do not whitelist by filename alone.
12. Remove `SIM_TICK_HZ` and `SIM_TICK_MS` exports after the final caller is
    gone. The atomic batch is now closed.

**Verify:** The census has no unclassified ordinary gameplay consumer. Only now
may Cargo or the app run.

### Task 8: Compile the atomic batch and repair integration errors

**Why:** This is the first safe executable checkpoint after the authority flip.

**Files:** Only files already in the File Map or newly revealed direct callers;
do not broaden behavior.

**Pattern:** Scoped `--lib` tests required by `AGENTS.md`.

**Steps:**

1. Confirm Cargo is free:

   ```powershell
   Get-Process cargo,rustc -ErrorAction SilentlyContinue
   ```

2. Run:

   ```powershell
   cargo check -p vera20k --lib
   ```

3. Repair compile errors by completing the planned interface migration. Do not
   restore `tick_ms`, a compatibility wrapper, synthetic `binary_frame`, or
   catch-up logic.
4. Run focused module tests:

   ```powershell
   cargo test -p vera20k --lib app_frame_pacer -- --nocapture
   cargo test -p vera20k --lib sim::movement -- --nocapture
   cargo test -p vera20k --lib sim::animation -- --nocapture
   cargo test -p vera20k --lib sim::replay -- --nocapture
   cargo test -p vera20k --lib sim::snapshot -- --nocapture
   ```

5. If another session takes Cargo, wait; do not launch competing builds.

**Expected:** The library compiles and focused owners pass under the single
native-frame contract.

### Task 9: Add cross-system authority and persistence tests

**Why:** Local unit tests cannot prove that movement, combat, lifecycle, pacing,
and persisted contracts share one boundary.

**Files:**

- Modify: `src/sim/world/global_parity_harness_tests.rs`
- Modify: `src/sim/world/lifecycle_tests.rs`
- Modify: `src/sim/movement/movement_tests.rs`
- Modify: `src/sim/combat/combat_tests.rs`
- Modify: `src/sim/animation_tests.rs`
- Modify: `src/sim/replay.rs`
- Modify: `src/sim/snapshot.rs`
- Modify: `src/app_sim_tick.rs` test module
- Modify: tactical capture test modules

**Pattern:** Existing in-module deterministic world/replay/snapshot tests.

**Steps:**

1. Add a world test starting at `native_frame=N` that proves all in-frame
   consumers observe `N`, the tail commits `N+1`, pending deletion drains after
   that commit, and `tick` advances once.
2. Add wraparound coverage starting at `u32::MAX`; consumers see MAX and the
   committed frame is zero.
3. Add a representative ground-movement test proving one Drive budget is
   consumed each frame, no every-third gate exists, and `blocked_delay`
   decrements once.
4. Add air/special tests for Fly/Hover/Jumpjet/Rocket/Teleport/homing and a
   paradrop asserting one state transition per explicit frame.
5. Add combat/economy tests proving ROF, production, miner load/unload, power,
   radar event, and superweapon deadlines decrement/expire in literal frame
   counts.
6. Add infantry tests for the 42-entry delay vector, the six normalized action
   IDs, fire-frame gating, and death completion pending removal.
7. Add SHP tests proving moving modulo, idle-zero disable behavior, and
   `WalkFrames` as layout only.
8. Add native `AnimClass` tests for base delay, random-before-normalized order,
   speed changes, loops/Next, owner detach, and repeated-render non-advancement.
9. Add pacer integration tests for:
   - one frame maximum after a long stall;
   - speed `0` uncapped eligibility;
   - pause admits zero;
   - exact step admits one and re-anchors;
   - changing speed changes only admission and verified normalized animation,
     not per-frame displacement, cooldown decrement, or RNG order.
10. Add replay v2 round-trip with a delayed `CommandEnvelope`, recorded speed,
    native frame, and matching state hash. Assert v1 rejection.
11. Add snapshot v31 round-trip with nonzero `native_frame` and frame-unit
    animation/movement state. Assert v30 rejection.
12. Add capture/profile tests asserting v2 native fields and v1 rejection.

**Verify:**

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue
cargo test -p vera20k --lib global_native_frame -- --nocapture
cargo test -p vera20k --lib replay -- --nocapture
cargo test -p vera20k --lib snapshot -- --nocapture
cargo test -p vera20k --lib tactical_capture -- --nocapture
```

Expected: all new authority and format tests pass.

### Task 10: Static cleanup, focused fidelity check, and final certification

**Why:** Prove the old authority is absent, the ordinary player loop is coherent,
and the repository is left stable.

**Files:**

- Modify if required: comments/tests in already mapped files
- Modify if mapped connection changed: the affected source file under
  `system_map/`
- Do not create evidence bundles; this is ordinary behavior timing, not a pixel
  or frame-capture delivery.

**Pattern:** `ENGINE.md` certification and System Map rules.

**Steps:**

1. Repeat the Task 7 census. Expected gameplay hits:
   - no `SIM_TICK_HZ`, `SIM_TICK_MS`, `total_sim_ms`, or `binary_frame`;
   - no `advance_tick` production interface;
   - no deterministic movement/combat/economy/animation `dt_ms`;
   - any remaining `elapsed_ms`/`duration_ms` is app-only cosmetic wall time and
     is neither serialized nor hashed.
2. Run `rustfmt --edition 2024 --check <edited-leaf-file.rs>` for each edited
   leaf Rust file. Never run crate-wide `cargo fmt`, and never pass a `mod.rs`
   because rustfmt would recurse into unrelated submodules.
3. Confirm Cargo is free, then run:

   ```powershell
   cargo clippy -p vera20k --lib -- -D warnings
   ```

4. Perform one focused fidelity observation in the representative default
   skirmish:
   - issue movement and attack orders;
   - observe infantry and SHP body cadence;
   - watch harvester unload and radar event;
   - open offline Options and confirm gameplay freezes;
   - resume and confirm no catch-up burst;
   - change local speed and confirm per-frame movement/ROF relation is stable;
   - execute an exact capture step and confirm one ordinal/one native frame.
5. Use the frozen System Map's `loop` / `mechanism` lookup to find the existing
   timing connection. If the verified implementation changed that mapped
   connection, update only its affected source nodes/edges under `system_map/`
   and run:

   ```powershell
   python -m tools.system_map check --require-sources
   ```

   Do not add tooling features, bulk registry rows, or a changelog.
6. Per `AGENTS.md`, run the full library suite exactly once at merge-to-dev
   certification:

   ```powershell
   Get-Process cargo,rustc -ErrorAction SilentlyContinue
   cargo test -p vera20k --lib
   ```

7. Leave no Cargo/rustc process running and no merge in progress. The handoff
   records branch, changed files, literal validation output, remaining residuals,
   and exact next safe action. Do not stage or commit unless separately
   authorized.

**Expected:** Static scans are clean/classified, formatting/clippy pass, the
representative scenario has coherent frame cadence with pause/no-catch-up, and
the single full library suite passes.

## Sources & References

### Design and architecture

- `docs/plans/2026-07-28-global-native-frame-timebase-design.md`
- `ENGINE.md`
- `AGENTS.md`

### Verified timing and scheduler research

- `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`
- `docs/research/NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION_GHIDRA_REPORT.md`
- `docs/research/MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md`
- `docs/research/FRAME_COUNTER_PREINCREMENT_VS_RUST_BINARY_FRAME_GHIDRA_REPORT.md`
- `docs/research/TIMEGETTIME_WALLCLOCK_VS_FRAMECLOCK_SPLIT_GHIDRA_REPORT.md`
- `docs/research/ADVANCE_TICK_PHASE_PARTITION_NATIVE_SPINE_GHIDRA_REPORT.md`
- `docs/research/OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`
- `docs/research/GROUND_MOVEMENT_LIFECYCLE_EFFECT_OWNERSHIP_GHIDRA_REPORT.md`

### Animation, radar, and lifecycle research

- `docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
- `docs/research/TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md`
- `docs/research/RADAR_EVENT_CLASS_GHIDRA_REPORT.md`
- `docs/research/RADAR_EVENT_PRODUCER_TYPE_MATRIX_GHIDRA_REPORT.md`
- `docs/research/COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_ATTACHEDOWNER_DETACH_LIFECYCLE_GHIDRA_REPORT.md`

### Locomotor and economy research

- `docs/research/FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/TELEPORT_LOCOMOTION_DEEP_DIVE.md`
- `docs/research/AAHEATSEEKER2_HOMINGTRACK_EXACT_MATH_GHIDRA_REPORT.md`
- `docs/research/BULLET_CLASS_AI_GHIDRA_REPORT.md`
- `docs/research/PARACHUTED_INFANTRY_DESCENT_RENDER_GHIDRA_REPORT.md`
- `docs/research/TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md`
- `docs/research/miner/MISSION_HARVEST_GHIDRA_REPORT.md`
- `docs/research/miner/REFINERY_UNLOAD_DRAIN_GRANULARITY_GHIDRA_REPORT.md`

### Live binary anchors checked for this plan

- `Main_Tick @ 0x0055D360`
- `DriveLocomotionClass::Process @ 0x004B0500`
- `AnimClass::AI @ 0x00423AC0`
- `TickRadarEvent @ 0x0065FE00`
- `GetRadarTimer @ 0x006C8C40`
- normalized animation helper `@ 0x005FB2E0`
- infantry action records `@ 0x007EAF7C`, delay byte at
  `0x007EAF7F + action_id * 4`

### INI authority

- `ini/rulesmd.ini` `[General]`
  `RadarEventVisibilityDurations=200,200,200,200,200,200` and
  `RadarEventDurations=400,400,400,400,400,400` — parsed by native but
  inactive at runtime
- `ini/rulesmd.ini` `[General]` `RadarEventMinRadius=8`,
  `RadarEventSpeed=1.2`,
  `RadarEventRotationSpeed=.05`, `RadarEventColorSpeed=.1`
- `ini/artmd.ini` `Rate=`, `RandomRate=`, `Normalized=`
- `ini/artmd.ini` infantry `Sequence=` sections
- `ini/artmd.ini` / type data `WalkRate=`, `IdleRate=`, `WalkFrames=`,
  `FiringFrames=`
- Native default `HarvesterDumpRate=.016` when stock INI omits the key

### Current Rust anchors

- `src/sim/scenario_session.rs`
- `src/sim/world/mod.rs`
- `src/app_sim_tick.rs`
- `src/sim/command.rs`
- `src/sim/replay.rs`
- `src/sim/snapshot.rs`
- `src/sim/world/world_hash.rs`
- `src/util/fixed_math.rs`
- `src/sim/movement/movement_step.rs`
- `src/sim/movement/drive_locomotion.rs`
- `src/sim/animation.rs`
- `src/sim/anim_class.rs`
- `src/rules/infantry_sequence.rs`
- `src/rules/shp_vehicle_sequence.rs`
- `src/sim/radar.rs`

## Plan Self-Review

1. **Spec coverage:** Every approved design requirement maps to Tasks 1–10,
   including global authority, pacing, late commit, no catch-up, pause, all
   ordinary consumers, normalized animation, persistence, and capture.
2. **Placeholder scan:** No unresolved marker, cross-task shorthand, or
   unspecified test placeholder remains.
3. **Architecture:** Wall time stays app-owned; deterministic time stays
   simulator-owned; no generic second timing service is introduced.
4. **Interface ordering:** Inert primitives precede the atomic batch; persisted
   contracts and public interfaces begin that batch; every caller closes it.
5. **Risk coverage:** Hybrid cadence, serialized layout, command metadata,
   lifecycle order, animation owners, speed changes, and capture schemas have
   explicit regression tests.
6. **Self-containment:** Exact paths, interfaces, edit rules, source claims,
   test cases, and commands are stated.
7. **Sim compliance:** Integer/fixed math, hash coverage, deterministic
   iteration, no presentation dependency, and tick ordering are explicit.
8. **Grounding:** The plan cites current Rust, authoritative INI data, verified
   research reports, and live binary anchors.
9. **Confidence:** Every technical decision is tagged. Medium confidence is
   limited to retained non-stock extension mechanics, which do not block stock
   skirmish.
10. **Deferred questions:** Only runtime throughput/jitter and bounded
    exactification residuals remain; no ordinary authority/lifecycle question
    is deferred.
11. **Player experience:** The ledger covers locomotion feel, combat/economy
    coupling, animation, pause/no-catch-up, persistence, custom extensions, and
    paradrop risk in a representative stock scenario.
