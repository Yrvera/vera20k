# Native Frame Rate Design

## Goal

Make one `Simulation::advance_tick` equal exactly one native logic frame (one
`g_CurrentFrameCounter` increment), paced by the game-speed byte
(`speed_byte × 16 ms` period; byte 0 = one step per render update), with a single
integer frame clock and all gameplay durations consumed as raw frame counts —
closing the ~3× wall-clock pace gap against gamemd at the default skirmish speed.

Roadmap item #2 follow-up in
`docs/plans/2026-05-28-foundational-scheduler-roadmap-todo.md`; rate research in
`docs/research/NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION_GHIDRA_REPORT.md`.

## Architecture Context

Three clocks currently flow through the sim/app boundary:

- **`sim.tick`** — `+1` per `advance_tick`; committed late (`world/mod.rs:1896`).
  This is the true structural twin of `g_CurrentFrameCounter`: one full logic
  pass per increment.
- **`binary_frame`** — `(total_sim_ms · 15) / 1000`, `total_sim_ms += tick_ms`,
  committed late (`world/mod.rs:1894-1895`). Advances at 1/3 of `sim.tick` (the
  45/15 ratio). Consumed by all native-frame-timer systems (facing/turret/gate/
  miner-dock/ore-growth).
- **`tick_ms`** — fixed `SIM_TICK_MS = 22` (`app_types.rs:27`,
  `fixed_math.rs:51` `SIM_TICK_HZ = 45`); passed into `advance_tick` and the
  presentation animation ticks.

The app-layer scheduler (`app_sim_tick.rs:234-285`) accumulates wall-clock time,
scales it by `sim_speed_tps / SIM_TICK_HZ` (`:237`), and runs N fixed 22 ms steps
(`schedule_fixed_steps :764`). `sim_speed_tps` comes from `tps_for_game_speed`
(`app_types.rs:36`); default byte 1 → 63 tps. Net at default: ~63 `advance_tick`
calls/s, `binary_frame` ~21/s, game advancing at ~21 logic frames/s.

Movement is calibrated to the same nominal-15 frame model:
`ra2_speed_to_leptons_per_second = leptons_per_tick · 15` (`fixed_math.rs:370`),
stepped by `dt = tick_ms/1000` (`dt_from_tick_ms :107`), so 3 ticks = one native
frame of travel (comment: "baseline = gamemd Slowest"). Combat ROF:
`rof_to_cooldown_ticks` uses `GAME_FPS = 15` (`combat/mod.rs:73,2318`) → `N` ROF
frames ≈ `3N` ticks.

The render layer reads sim-precomputed screen positions directly — the sim writes
`pos.screen_x/screen_y` each tick via `lepton_to_screen`, and `units.rs:188`
notes "Screen position is computed by the sim layer … No renderer-side
interpolation needed." Turret/facing render reads `f.current(sim.binary_frame)`
(`units.rs:259`, `shp.rs:324`).

The state hash folds in `self.tick`, `self.total_sim_ms`, and `self.binary_frame`
(`world_hash.rs:36-38`); the hash is computed after the late commit.

### Native authority being preserved

`Main_Tick` is the native order owner: one input→logic→render→service pass, then
one guarded `g_CurrentFrameCounter++`, then the throttle wait. The Rust analog is:
the app-layer scheduler owns the frame rate (how many frames to advance per
wall-clock slice), `Simulation::advance_tick` owns one frame's ordered logic and
the single late frame-counter commit, and the render layer reads committed frame
state. No native C++ structure (Main_Tick's inline loop, the bucket globals) is
ported literally — only the contract: one logic pass = one frame, paced by
`speed_byte × 16 ms`, durations counted in frames.

## Impact Analysis

**Sim layer (clock collapse — `binary_frame`/`total_sim_ms` → `sim.tick`):**
- `world/mod.rs` — remove `total_sim_ms`/`binary_frame` derivation
  (`:1894-1895`); `self.tick` becomes the single frame clock. `advance_tick`
  stops scaling movement by `tick_ms` (each call = one frame).
- `world_hash.rs:37-38` — drop `total_sim_ms`/`binary_frame`; hash `self.tick`
  only. (Determinism improvement — removes ms-derived hash state.)
- `movement/facing_class.rs`, `movement/turret.rs` — `binary_frame` →
  frame clock (RateTimer math already frame-based; just the source changes).
- `gate_runtime.rs`, `miner/miner_dock_sequence.rs`, `ore_growth.rs`,
  `terrain_spawn.rs`, `combat/smudge_dispatch.rs`, `tiberium/mod.rs` —
  `binary_frame` arg → frame clock (all are relative/stored-start CDTimers;
  no per-consumer ±1 — proven uniform in the Native Frame/Tick Contract).
- `combat/mod.rs` — `rof_to_cooldown_ticks`: ROF frames used directly as a
  frame cooldown (drop `GAME_FPS=15` and the `/tick_ms` conversion). Turret/fire
  checks read the frame clock.
- `movement/movement_tick.rs`, `homing_movement.rs`, `droppod_movement.rs`,
  `drive_track.rs` — replace `dt_from_tick_ms`/`ra2_speed_to_leptons_per_second`
  with per-frame lepton steps (apply the native per-frame speed once per frame).

**App scheduler (rate mapping):**
- `app_types.rs` — replace `tps_for_game_speed`/`SIM_TICK_MS` with
  `frame_period_ms(speed_byte) = speed_byte × 16` (byte 0 → step-per-update).
- `app_sim_tick.rs:234-285` — scheduler steps one frame per `frame_period_ms`
  of accumulated wall time (no `× sim_speed_tps / SIM_TICK_HZ`); keep the
  catch-up cap (`MAX_SIM_STEPS_PER_FRAME`) to avoid the spiral of death.
  `advance_tick` no longer needs `tick_ms` for sim math.
- `fixed_math.rs:51` — `SIM_TICK_HZ` removed/repurposed; stale 15 fps doc-comment
  deleted.
- `app.rs`, `app_input.rs`, `app_transitions.rs`, `app_spawn_pick.rs` —
  `sim_speed_tps`/`sim_accumulator_ms` reset sites adapt to the new field(s);
  dev overlay (`app_dev_overlay.rs`) shows frame rate instead of tps.
- `replay` header `tick_hz` (`app_sim_tick.rs:262`) — record the speed byte /
  frame period instead of a fixed 45.

**Render layer:**
- `app_instances/units.rs:259`, `shp.rs:324` — `sim.binary_frame` → frame clock.
- A1 chosen: **no render interpolation** — render keeps reading the latest
  committed frame's `screen_x/screen_y`. No new render state.

**Tests (encode the 45/15 or ms model — update to frame model):**
- `app_types.rs` tests (`default_yr_skirmish_tps == 63`),
  `world_hash.rs`/`world_tests.rs` binary-frame derivation tests,
  `miner_tests.rs` 67 ms-step capture helpers, `combat_turret_facing_tests.rs`,
  movement tests using `tick_ms`.

**Blast radius / risk:**
1. **Pace tripling (intended).** Every frame-counted system runs ~3× faster in
   wall-clock at default speed — the parity correction. Must verify in-game that
   default pace now matches gamemd default.
2. **Movement per-frame value.** The per-frame lepton step must equal the native
   per-frame speed; the current `leptons_per_tick` formula is already the
   per-frame value (only the rate was wrong), but verify against the movement
   research before shipping (flagged in the ledger).
3. **Determinism.** Frame is integer; one commit point; sim math has no ms input;
   replay records the frame sequence. Batching N frames in one app-update = N
   identical sequential advances → lockstep-safe. Net determinism *improves*
   (ms-derived `total_sim_ms` leaves the hash).
4. **Replay-hash divergence vs old recordings** — expected (the clock changed);
   no frozen golden baseline in-repo.

## Tiny-Detail Ledger

1. One `advance_tick` = one native frame = one `g_CurrentFrameCounter` increment;
   no inner logic loop. `[GHIDRA 0x0055D360]`
2. Frame counter commits **late**, after all phase work (already implemented).
   `[GHIDRA 0x0055D360; world/mod.rs:1896]`
3. During a tick the frame value is constant (single commit) → same-tick
   start/check yields `elapsed == 0`. `[Native Frame/Tick Contract]`
4. Local-skirmish frame period = `speed_byte × 16 ms`; the 16 ms unit is
   `GetRadarTimer = timeGetTime() >> 4`. `[GHIDRA 0x0055E160, 0x006C8C40]`
5. Default YR skirmish `speed_byte = 1` → ~16 ms/frame → ~62.5 fps cap.
   `[GHIDRA 0x00697F10; rulesmd MultiplayerDialogSettings GameSpeed=1]`
6. Speed byte → fps: 0 uncapped, 1≈62.5, 2≈31.3, 3≈20.8, 4≈15.6, 5≈12.5,
   6≈10.4; UI slider = `6 − byte`. `[GHIDRA 0x0055E160, 0x004E1DE0]`
7. Byte 0 (uncapped) = logic runs at render throughput → Rust: one logic step
   per render update, no wall-clock minimum. `[GHIDRA 0x0055E160 budget 0 path]`
8. All gameplay durations are frame counts — ROF, build, ore growth, facing ROT,
   AnimType `Rate=900/Rate`, modulo gates — consumed directly, no ms conversion.
   `[GHIDRA 0x0046B640/0x00426630; GLOBAL_TIMING_MODEL report]`
9. CDTimer: `elapsed = frame − start`; running while `elapsed < duration`
   (exclusive); `duration == 0` immediately expired; `start == -1` returns full
   duration. `[GHIDRA 0x00426630]`
10. `CDTimerClass::Start` stores the current (pre-increment) frame. `[GHIDRA
    0x0046B640]`
11. Modulo gate fires on the tick whose pre-increment frame ≡ 0 mod N (e.g.
    bridge-shroud `% 0x78`). `[GHIDRA 0x0055AFB0]` — no live Rust gate yet.
12. First tick exposes frame 0. `[construction: tick = 0]`
13. Movement applies the native per-frame lepton step once per frame; the speed
    setting changes pace only via the frame *rate*, not the per-frame distance.
    Per-frame value must match native. `[ini: rulesmd Speed=; verify vs movement
    research — UNKNOWN exact native formula, current leptons_per_tick assumed]`
14. Render fps is decoupled from logic fps but reads the latest committed frame
    (A1 = no interpolation), matching gamemd's render-inside-Main_Tick lock.
    `[GHIDRA 0x0055D360 RenderFrame_main is per-frame]`
15. Network/MP frame pacing uses the separate `1000 / DAT_00a8b558` ms budget,
    NOT the local bucket path — out of scope; flagged for the future MP
    scheduler. `[GHIDRA 0x0055D360 network branch; get_xrefs_to 0x00a8b558]`

## Chosen Approach

**Option A — one `advance_tick` per native frame, single integer frame clock,
A1 render (no interpolation).**

- Collapse `total_sim_ms` and `binary_frame` into `self.tick`. One field is the
  frame counter; it is `g_CurrentFrameCounter`. Keep the existing late commit.
- `advance_tick` advances the game by exactly one frame: movement applies the
  per-frame lepton step, combat counts ROF in frames, facing/turret/gate/miner/
  ore all read `self.tick`. No `tick_ms` enters sim math.
- The app-layer scheduler converts wall-clock elapsed → integer frame steps using
  `frame_period_ms(speed_byte) = speed_byte × 16` (byte 0 → one step per render
  update). Catch-up capped.
- Render reads the latest committed frame's positions and `self.tick` for facing
  — unchanged read path, just the clock source consolidated.

Rationale: most faithful (mirrors `Main_Tick`'s one-pass-per-frame, native pace
at every speed), single source of truth for game time, and a determinism
improvement (no ms in the hash). A1 over interpolation because, with logic at the
native rate, there is nothing to smooth at normal speeds and interpolation would
make the port *smoother than gamemd* at slow speeds (a deviation).

## Design

### Components

- **`Simulation` (sim owner of frame order):** holds `tick: u32` as the single
  frame counter (== `g_CurrentFrameCounter`). `binary_frame`/`total_sim_ms`
  removed. `advance_tick(commands, rules, …)` — drop the `tick_ms` parameter from
  sim math (keep a thin shim only if presentation callers still need it during
  the deferred-anim transition).
- **Frame scheduler (app owner of rate):** `app_sim_tick.rs` computes
  `frames_to_step` from accumulated wall time and `frame_period_ms(speed_byte)`,
  then loops `advance_tick` that many times. Owns `frame_accumulator_ms` and the
  speed byte (replacing `sim_speed_tps`/`sim_accumulator_ms` semantics).
- **Speed mapping:** `app_types::frame_period_ms(speed_byte) -> Option<u32>`
  (`None`/0 ⇒ step-per-update). Replaces `tps_for_game_speed`.
- **Movement per-frame step:** `fixed_math` exposes a per-frame lepton step
  helper (replacing `ra2_speed_to_leptons_per_second` + `dt_from_tick_ms`).

### Interfaces / Contracts

- `Simulation.tick: u32` — during `advance_tick`, the frame `N` being executed;
  consumers store it as a start-frame and compute `tick.saturating_sub(start)`
  for elapsed (canonical CDTimer contract). Committed late.
- `advance_tick` advances state by exactly one frame; calling it K times advances
  K frames deterministically, independent of wall clock.
- Render contract: render may read `sim.tick` and `pos.screen_x/screen_y` at any
  time; it must never write back. No interpolation state (A1).

### Data Flow

```
app update (wall dt) ──> frame_accumulator_ms += dt
  frames = 0
  while frame_accumulator_ms >= frame_period_ms(speed) && frames < CAP:
        frame_accumulator_ms -= frame_period_ms(speed)
        sim.advance_tick(due_commands, …)   # one native frame; tick = N → N+1 late
        frames += 1
  (byte 0: step exactly once per update, ignore accumulator minimum)
render ──> reads sim.tick + committed screen_x/screen_y (latest frame)
```

### Error Handling

No new error types. Spiral-of-death guarded by the catch-up cap (existing
`MAX_SIM_STEPS_PER_FRAME`); leftover accumulator clamped so a stall cannot queue
unbounded frames. Speed byte clamped to `0..=6`.

### Testing Strategy

1. **Pace:** at byte 1, K wall-clock seconds → ~62.5·K `advance_tick` calls
   (scheduler unit test against a mocked clock).
2. **Speed mapping:** `frame_period_ms` returns 16/32/48/64/80/96 for bytes
   1..6 and step-per-update for 0.
3. **Single-clock:** `sim.tick` is the only frame field; same-tick start/check →
   `elapsed == 0` (carried from the Native Frame/Tick Contract test).
4. **Frame-count determinism:** stepping N frames in one batch == N single-frame
   steps (identical hash sequence), independent of injected wall dt.
5. **Combat/facing in frames:** ROF=N frames fires every N frames; facing ROT
   reaches target in `ceil(delta/rot)` frames — no `×3`.
6. **Regression:** state-hash internal-consistency tests pass with the
   single-field hash; updated miner-dock/turret tests reflect the frame clock.
7. **In-game pace check:** default skirmish unit move/reload/build/ore pace
   visually matches gamemd default (manual, against retail).

### Determinism

Single integer frame clock; one commit point per frame; sim math has zero
wall-clock/ms input; both the per-frame logic and the frame count are
deterministic. The wall-clock→frame-count conversion lives entirely in the app
layer and never affects per-frame state. Replay stores the frame/command
sequence; playback reproduces frames regardless of local fps. Removing
`total_sim_ms` from the hash eliminates the only ms-derived hash input — a net
determinism improvement. Lockstep-safe for skirmish/replay; MP frame
synchronization (network budget) is a separate future scheduler.

## Architectural Decisions

- **Native authority contract:** app-layer scheduler owns rate (Main_Tick's
  pacing role), `advance_tick` owns one frame's ordered logic + the single late
  commit (Main_Tick's logic pass + guarded increment), render reads committed
  frame state. Rust-native ownership, gamemd-native semantics.
- **Single source of truth for game time** (`sim.tick`); removes the 45/15
  dual-clock and the ms-derived `binary_frame`.
- **A1 (no interpolation)** chosen as most parity-correct and most Rust-native;
  interpolation deferred as an optional future toggle, explicitly a deviation.
- **Deferred (named DRIFT / follow-up):** presentation animations (crane,
  damage-fire, muzzle, parachute) stay ms-based this pass — they also run ~3× off
  and should migrate to frame-based timing in a separate change; MP network-frame
  pacing (`1000/DAT_00a8b558`) is out of scope.
- **UNKNOWN to verify before ship:** the exact native per-frame lepton speed
  formula (ledger #13) — confirm the current `leptons_per_tick` is the per-frame
  value, not a per-(1/3-frame) value, against the movement research.

## Alternatives Considered

- **Option B — keep ×3 sub-stepping, raise the rate to native fps** (byte 1 →
  ~187.5 sim-ticks/s): keeps render-smooth sub-steps but heavier, retains two
  clocks at a fixed 3:1, and is not a literal mirror of native (which has no
  sub-steps). Rejected by user in favor of A.
- **A2 — render interpolation between frames:** smooth at all speeds but smoother
  than gamemd at slow speeds (deviation), adds render-only prev-position state and
  a feedback-into-hash hazard. Rejected as default; viable later optional toggle.
- **Keep nominal 15 fps + fix only tps scaling:** can't reach native pace without
  effectively becoming Option B; leaves the dual-clock confusion. Rejected.
