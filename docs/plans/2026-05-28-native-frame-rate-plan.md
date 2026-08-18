# Native Frame Rate Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. Read the
> Grounding Summary first — it materially refines the design doc's premise.

**Goal:** Make one `Simulation::advance_tick` advance exactly one native logic
frame so the game runs at gamemd's game-speed-determined rate (~62.5 fps at the
default skirmish speed byte 1), closing the current ~3× wall-clock slowdown.

**Architecture:** The app-layer scheduler converts wall time → integer frame steps
and calls `advance_tick` (~63×/s at default); `advance_tick` runs one ordered
logic pass and commits the frame counter late; render reads committed frame state.

**Design Doc:** `docs/plans/2026-05-28-native-frame-rate-design.md`

---

## Grounding Summary

- **Rate (verified):** one `Main_Tick` = one full logic pass = one
  `g_CurrentFrameCounter` increment; local skirmish throttles to
  `speed_byte × 16 ms` (`GetRadarTimer = timeGetTime()>>4`); default YR byte = 1 →
  ~62.5 fps cap; all gameplay timing is in frame units.
  `[NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION_GHIDRA_REPORT.md;
  GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md]`
- **Movement per-frame value (UNKNOWN #13 resolved):** `GetCurrentSpeed @
  0x004DB1A0` returns `floor(Speed×256/100)` leptons **per binary frame** (Speed=4
  → 10 lep/frame). The Rust `leptons_per_tick = speed×256/100` IS that per-frame
  value. `[FRAME_BASIS_MOVEMENT_TURRET_GHIDRA_REPORT.md §2.2;
  DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §3.3]`
- **The actual Rust bug — the fix is smaller than the design implied.** The
  scheduler already produces `sim_speed_tps` (~63 at byte 1) `advance_tick`
  calls/s **independent of `SIM_TICK_HZ`** (because `SIM_TICK_HZ × SIM_TICK_MS ≈
  1000`). The 3× slowdown is that `SIM_TICK_HZ = 45` makes each `advance_tick`
  advance only **1/3 of a frame**: (a) movement self-gates via `drive_delay`
  (`DRIVE_TRACK_SUBTICKS_PER_NATIVE_FRAME = 45/15 = 3`,
  `movement_step.rs:42-58`); (b) `binary_frame = total_sim_ms·15/1000` advances
  ~21/s; (c) dt-scaled locomotors get `dt = SIM_TICK_MS = 22 ms` instead of a full
  ~66 ms frame. Setting `SIM_TICK_HZ = 15` makes `SIM_TICK_MS = 66`, so each call
  advances one full frame and dt-scaled systems auto-correct 3×.
  `[repo: app_sim_tick.rs:237; app_types.rs:36-46; fixed_math.rs:51;
  movement_step.rs:42-58; world/mod.rs:1894-1895]`
- **Per-call vs dt-scaled vs absolute-ms distinction (drives the risk profile):**
  - *dt-scaled* (scale linearly with `tick_ms`: `homing v = speed·dt`, air/drive
    budgets, **and body rotation** `rot_to_facing_delta(rot, tick_ms)` =
    `rot·256·15·tick_ms/360000`, `turret.rs:39`) → run 3× slow today,
    **auto-correct** at `SIM_TICK_HZ=15`. Body rotation is in THIS bucket
    (review-confirmed) — it is **not** a regression hazard; the `FacingClass`
    migration (H-1) is a precision improvement, deferred. `[turret.rs:32,39;
    movement_step.rs:234]`
  - *per-call* (incremented once per `advance_tick`: homing `frame_counter`,
    sidewinder `% 15`, `rot_bam_per_tick`) → already at ~63/s ≈ native 62.5 fps,
    **unchanged** by the HZ change. Already ~right.
  - *absolute-ms phase/aging clocks* (**regress** under 22→66 ms — these count or
    phase against accumulated ms, so they speed up ~3×): `total_sim_ms` (the
    sparkle/pixel-fx phase clock — `build_instances.rs:348` →
    `pixel_fx_sparkles.rs:226 clock_ms`), `radar_events.tick(tick_ms)` and
    `world_effects` `tick(tick_ms)` (`world/mod.rs:1856,1861`), the app-layer anim
    ticks fed `SIM_TICK_MS` (`tick_animations` death-anim→despawn is a sim input;
    voxel/harvest/garrison-muzzle), and any production/power ms aging. These MUST
    be pinned to a legacy cadence or confirmed frame-correct in the core change.
    `[build_instances.rs:348; pixel_fx_sparkles.rs:222-226; world/mod.rs:1856,1861;
    app_sim_tick.rs:190-211,292-312]`
- **Repo pattern to mirror:** barrel `FacingClass` (frame-counter,
  `binary_frame`-driven) in `facing_class.rs`/`turret.rs` is the correct model;
  body rotation should mirror it (H-1).
- **INI keys:** `rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1`; per-unit
  `Speed=`, `ROT=` (already parsed). No new INI parsing.
- **Still unknown after grounding:** which app-layer anim ticks are pure
  presentation (safe to leave on a fixed legacy cadence) vs sim-affecting
  (death-anim→despawn). Resolved by Task 4's audit, not assumed.

## Key Technical Decisions

- **Core fix = `SIM_TICK_HZ 45→15` + `binary_frame = tick`**, not a scheduler
  rewrite. `tps_for_game_speed` already maps byte→fps. — **Confidence:** high
  (from analysis) — **Source:** repo `app_sim_tick.rs:237`, `app_types.rs:36`,
  `movement_step.rs:42-58`; FRAME_BASIS_MOVEMENT_TURRET report. **Flag for
  /review-plan + in-game validation** (Task 7) — the "63 calls/s × 1 frame = 62.5
  fps" chain is derived, not yet run.
- **Body rotation auto-corrects (dt-scaled); the `FacingClass` migration is
  deferred precision work, NOT an in-core regression fix** (review correction).
  `rot_to_facing_delta` scales with `tick_ms`, so 22→66 ms turns it ~3× faster =
  the same correction every other dt-scaled system gets. — **Confidence:** high —
  **Source:** repo `turret.rs:39` (formula), `movement_step.rs:234` (call). Note:
  FRAME_BASIS H-1 claims "~2× too fast" today — contradicts the formula (which is
  ~3× too slow); resolve via in-game check (Task 7), trust the formula over the
  doc.
- **Absolute-ms phase/aging clocks get pinned to a fixed legacy cadence constant
  (22 ms) to avoid 3× speed-up**: `total_sim_ms` (sparkle phase),
  `radar_events`/`world_effects` aging, and the `SIM_TICK_MS`-fed anim ticks.
  Proper frame migration deferred. — **Confidence:** medium — **Source:**
  build_instances.rs:348, pixel_fx_sparkles.rs:226, world/mod.rs:1856-1861,
  app_sim_tick.rs:190-211. Confirm scope in Task 4.
- **Exact pacing on `speed_byte × 16 ms` (byte 0 = one step per render update)**
  replaces the `tps/SIM_TICK_HZ` approximation for byte-accurate periods and the
  ~1% integer-rounding error (`SIM_TICK_MS=66` vs true 66.67). — **Confidence:**
  high — **Source:** rate report §2; FUN_0055e160.

## Open Questions

### Resolved During Planning

- *Is `leptons_per_tick` the native per-frame speed?* Yes —
  `floor(Speed×256/100)` per binary frame. `[FRAME_BASIS_MOVEMENT_TURRET §2.2]`
- *Does the scheduler rate depend on `SIM_TICK_HZ`?* No — it yields `sim_speed_tps`
  steps/s regardless; HZ only sets frame granularity per step.
- *Is render interpolation needed?* No (A1) — logic runs ~62.5 fps ≥ display;
  matches gamemd's render=logic lock.

### Deferred to Implementation

- Exact set of app-layer anim ticks that are sim-affecting vs pure presentation
  (Task 4 audit) — determines which keep the legacy 22 ms constant vs migrate.
- Final achieved fps vs the ~62.5 cap under real render load (Task 7 in-game).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/util/fixed_math.rs:47-51` | `SIM_TICK_HZ 45→15`; fix doc comment |
| Modify | `src/sim/world/mod.rs:1894-1895` | `binary_frame = execute_tick`; pin `total_sim_ms += 22` (legacy sparkle-phase constant, not the new 66 ms) |
| Modify | `src/sim/world/mod.rs:1856,1861` | `radar_events`/`world_effects` aging → legacy 22 ms constant (Task 4) |
| Keep | `src/sim/world/world_hash.rs:37` | keep `total_sim_ms` in hash (still deterministic; legacy-22 pin keeps assertion :1167 `total_sim_ms == 990` passing) |
| Modify | `src/sim/world/world_hash.rs:1168` | `binary_frame` assertion 14→45 (now 1:1 with tick) — Task 6 |
| Modify | `src/app_sim_tick.rs:190-211,292-312` | anim ticks → legacy 22 ms constant (Task 4) |
| Audit | `src/sim/production/production_queue.rs`, `src/sim/power_system.rs` | confirm frame-based or pin (Task 4) |
| Modify | `src/app_types.rs:27-46,166-176` | `frame_period_ms(speed_byte)`; byte 0 handling; update tps tests |
| Modify | `src/app_sim_tick.rs:234-285` | pace one frame per `frame_period_ms`; pass fixed nominal `tick_ms` to `advance_tick` |
| Defer | `src/sim/movement/movement_step.rs:234`, `turret.rs:32`, `facing_class.rs` | body rotation `FacingClass` migration — precision cleanup, NOT in this change (auto-corrects) |
| Modify | tests (see Task 6) | `world_hash`/`world_tests`, `miner_tests`, `drive_track_tests`, `combat_turret_facing_tests`, `app_types` |

## Interface Changes

- `fixed_math::SIM_TICK_HZ` value change (15). Consumers: `app_types::SIM_TICK_MS`,
  `app_sim_tick` scheduler, `movement_step` sub-gating, replay header. No signature
  change.
- `Simulation.binary_frame` semantics: now == `self.tick` exactly (1:1). No
  consumer signature change; all current `binary_frame` readers keep working.
- `app_types`: replace `tps_for_game_speed` with `frame_period_ms(speed_byte) ->
  Option<u32>` (None ⇒ step-per-update). Consumers: `app_sim_tick`, `app.rs`,
  `app_dev_overlay`.

## Sim Checklist

- [x] All math fixed-point — no f32/f64 added to sim logic.
- [x] New state in hash: none added; `binary_frame` now == `tick` (already hashed).
- [x] No sim→render/ui/audio/net dependency introduced.
- [x] Tick ordering: unchanged; only the per-tick frame *magnitude* and commit
      derivation change. Late commit preserved.
- [x] BTreeMap iteration order: unaffected.

## Risk Areas

1. **Absolute-ms clock regression** (highest) — `total_sim_ms` (sparkle phase),
   `radar_events`/`world_effects` aging, and `tick_animations` (death-anim →
   despawn, a sim/determinism input) all speed up ~3× if fed the new 66 ms.
   Mitigation: pin to legacy 22 ms (Task 2 for `total_sim_ms`; Task 4 for the rest).
2. **Body rotation** — review-corrected: dt-scaled, **auto-corrects** (not a
   regression). Verify in-game (Task 7). Note H-1 doc says "2× fast" while the
   formula says "3× slow"; trust the formula, confirm in-game.
3. **Pace tripling is intended** but touches every dt-scaled system — broad
   in-game verification required (Task 7).
4. **Replay hash divergence vs old recordings** — expected (clock changed); no
   frozen golden baseline in-repo.
5. **Determinism preserved** — frame integer, single commit, fixed (non-wall-clock)
   per-frame dt; net improvement (binary_frame no longer ms-derived).

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 1 | One `advance_tick` = one native frame | The whole pace correction; fires every match | In-game pace vs gamemd default (Task 7); `[GHIDRA 0x0055D360]` |
| 1 | dt-scaled translation 3× correction (incl. body rotation) | Unit move speed + turn rate every match | Speed=4 unit ≈ 2.44 cells/s; MCV turn vs gamemd; `[FRAME_BASIS_MOVEMENT_TURRET §2.2; turret.rs:39]` |
| 3 | `frame_period_ms(byte)=byte×16`, byte0 uncapped | Each speed setting matches gamemd pace | `frame_period_ms` unit test; `[rate report §2]` |
| 2 | `binary_frame == tick` (1:1) | Facing/turret/gate/miner/ore timer rate | same-tick + rate tests; `[Native Frame/Tick Contract]` |
| 2 | `total_sim_ms` pinned to 22 ms | Sparkle/pixel-fx phase rate must not 3× | sparkle speed unchanged in-game; `[pixel_fx_sparkles.rs:226]` |
| 4 | Death-anim→despawn + radar/world-effect aging preserved | Anim length, ping lifetime, removal timing | aging audit + in-game (Task 7) |

---

## Tasks

### Task 1: Set the sim frame rate to one native frame per tick

**Why:** Core correction — makes each `advance_tick` advance a full frame so
dt-scaled systems run at native pace. Ordered first; everything else builds on it.

**Files:** Modify `src/util/fixed_math.rs:47-51`

**Pattern:** Existing constant; only the value + stale comment change.

**Step 1: Change the constant and fix the comment**
```rust
/// Canonical simulation tick rate in Hz — one tick = one native logic frame
/// (one `g_CurrentFrameCounter` increment). The app-layer scheduler invokes
/// `advance_tick` at the game-speed rate (~62.5/s at the default skirmish speed
/// byte 1); INI timing values (ROF, Speed, ROT, Rate) are consumed directly as
/// frame counts. (Was 45, which sub-divided each native frame into thirds and
/// ran the game ~3× slow.)
pub const SIM_TICK_HZ: u32 = 15;
```

**Step 2: Verify**
Run: `cargo build -p <crate>` — expect compile success. `SIM_TICK_MS`
(`app_types.rs:27`) now derives to 66.

**Step 3: Commit** — `sim/timing: one advance_tick = one native frame (SIM_TICK_HZ 15)`

---

### Task 2: Collapse `binary_frame` to the frame/tick counter

**Why:** Make the frame counter a single source of truth (= `g_CurrentFrameCounter`
analog) instead of a separate 15-Hz-of-ms derivation. Keeps every existing
`binary_frame` consumer working (now 1:1 with `tick`).

**Files:** Modify `src/sim/world/mod.rs:1887-1896`

**Pattern:** Existing late-commit block; replace the derivation line.

**Step 1: Replace the derivation with a 1:1 assignment, and pin `total_sim_ms`**
`total_sim_ms` is the sparkle/pixel-fx **phase clock** (`build_instances.rs:348`
→ `pixel_fx_sparkles.rs:226 clock_ms`). It currently advances `tick_ms`/tick =
22 ms × ~63/s ≈ 1386 ms/s. After Task 1, `tick_ms` becomes 66 → it would advance
~3× faster and speed up sparkles. Pin its increment to a fixed legacy constant so
the sparkle phase rate is unchanged (still deterministic — a fixed constant).
```rust
        // Single frame clock: binary_frame == tick == g_CurrentFrameCounter
        // analog, committed late (mirrors Main_Tick's guarded increment).
        // total_sim_ms is a presentation phase clock (sparkle/pixel-fx); pin its
        // rate to the legacy per-tick ms so HZ change doesn't 3× the effect speed.
        // Deferred: migrate sparkles to a frame-based phase.
        const LEGACY_PRESENTATION_TICK_MS: u64 = 22;
        self.total_sim_ms = self.total_sim_ms.saturating_add(LEGACY_PRESENTATION_TICK_MS);
        self.binary_frame = execute_tick;
        self.tick = execute_tick;
```

**Step 2: `total_sim_ms` in the hash (`world_hash.rs:37`)**
Keep `self.total_sim_ms.hash(...)` — still deterministic (fixed-constant sum).
In `binary_frame_drift_free_at_22ms_ticks` (`world_hash.rs:1160-1169`), the
`total_sim_ms == 990` assertion (`world_hash.rs:1167`) **survives unchanged** —
the legacy-22 pin keeps 45×22 = 990. But the *next* assertion,
`assert_eq!(sim.binary_frame, 14)` (`world_hash.rs:1168`), **WILL FAIL**: after
`binary_frame = execute_tick` the value becomes 45 (= tick), not 14. That line
must be updated to `assert_eq!(sim.binary_frame, 45)` — **this is done in Task 6**
(test updates), not here. Confirm `build_instances.rs:348` read is unaffected.

**Step 3: Verify**
Run: `cargo build` — success. `binary_frame` now advances exactly once per tick;
sparkle phase rate unchanged.

**Step 4: Commit** — `sim: binary_frame == tick; pin total_sim_ms phase clock`

---

### Task 3: Game-speed byte → exact frame period (replaces tps approximation)

**Why:** Byte-accurate pacing (`speed_byte × 16 ms`) and correct byte-0
(uncapped) behavior; removes the ~1% integer-rounding pace error.

**Files:** Modify `src/app_types.rs:31-46`

**Pattern:** Replaces `tps_for_game_speed`; same module.

**Step 1: Add the frame-period mapping**
```rust
/// gamemd local-skirmish frame period: each Main_Tick is throttled to at least
/// `speed_byte × 16 ms` (GetRadarTimer 16 ms buckets). Byte 0 = uncapped
/// (None ⇒ scheduler steps once per render update). Verified:
/// docs/research/NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION_GHIDRA_REPORT.md.
pub(crate) fn frame_period_ms(stored_speed: u32) -> Option<u32> {
    if stored_speed == 0 {
        return None; // uncapped: one logic step per render update
    }
    Some(stored_speed.saturating_mul(GAME_SPEED_BUCKET_MS).max(1))
}
```

**Step 2: Keep a default helper**
```rust
pub(crate) fn default_yr_skirmish_frame_period_ms() -> Option<u32> {
    frame_period_ms(DEFAULT_YR_SKIRMISH_GAME_SPEED) // byte 1 → Some(16)
}
```

**Step 3: Verify (unit test)**
```rust
#[test]
fn frame_period_matches_gamemd_buckets() {
    assert_eq!(frame_period_ms(0), None);          // uncapped
    assert_eq!(frame_period_ms(1), Some(16));      // ~62.5 fps (default)
    assert_eq!(frame_period_ms(4), Some(64));      // ~15.6 fps (authoring ref)
    assert_eq!(frame_period_ms(6), Some(96));      // ~10.4 fps (slowest)
}
```
Run: `cargo test frame_period_matches_gamemd_buckets` — PASS.

**Step 4: Commit** — `app: game-speed byte → exact frame period (speed_byte×16ms)`

---

### Task 4: Absolute-ms regression guard (pin presentation/aging clocks)

**Why:** Every consumer that *counts or phases against accumulated ms* speeds up
~3× when `tick_ms` goes 22→66. Pin each to a legacy 22 ms cadence (or confirm it
is frame-based and intentionally 3×-corrected). **Body rotation is NOT here** — it
is dt-scaled (`rot_to_facing_delta` scales with `tick_ms`) and auto-corrects;
its `FacingClass` precision migration is deferred. `total_sim_ms` is handled in
Task 2.

**Files:** Modify `src/sim/world/mod.rs:1856,1861` (radar / world-effects aging),
`src/app_sim_tick.rs:190-211,292-312` (anim ticks). Audit
`src/sim/production/production_queue.rs` and `src/sim/power_system.rs` for ms
dependence.

**Pattern:** Existing wall-clock-vs-fixed split (crane/damage-fire already use real
`sim_elapsed`, `app_sim_tick.rs:188-189` comment) and Task 2's
`LEGACY_PRESENTATION_TICK_MS` pin.

**Step 1: Classify every `tick_ms` / `SIM_TICK_MS` consumer** as (a) frame-based
(advances per native frame — correct to leave on the new tick), (b) absolute-ms
aging/phase (must pin), or (c) sim-affecting:
- `radar_events.tick(tick_ms)` (`world/mod.rs:1856`) — ping lifetime aging.
- `world_effects … tick_with_start_sound(tick_ms)` (`world/mod.rs:1861`) — effect
  duration aging.
- `tick_animations` (`app_sim_tick.rs:292`) — death anim → `death_finished` →
  `despawn_entity` (**sim-affecting**; despawn timing must not change).
- `tick_voxel_animations`, `tick_harvest_overlays` (`:311-312`),
  `tick_garrison_muzzle_flashes` (`:198-201`).
- Audit production-queue progression and power aging for ms dependence.

**Step 2: Pin absolute-ms / sim-affecting consumers to a legacy constant** so
behavior is byte-identical to today; mark `// TODO: frame-based migration
(deferred)`. Genuinely frame-based consumers may take the new per-frame tick.
```rust
/// Deferred: presentation/aging clocks still tick on the legacy 22 ms cadence
/// until their frame-based migration. Keeps despawn/anim/ping timing unchanged
/// while the sim clock moves to one-frame-per-tick.
const LEGACY_PRESENTATION_TICK_MS: u32 = 22;
```
Pass this constant to the sim-side calls (`radar_events`, `world_effects`,
`tick_animations`) and in place of `SIM_TICK_MS` for the app-layer anim ticks.

**Step 3: Verify** radar-ping lifetime, world-effect duration, and death-anim
length + despawn frame are unchanged vs a pre-change run (manual +
`animation_tests`).

**Step 4: Commit** — `sim/app: pin absolute-ms presentation/aging clocks (defer migration)`

---

### Task 5: Pace the scheduler on the frame period

**Why:** Step exactly one frame per `frame_period_ms` of wall time (byte 0 = one
step per update); replaces the `× sim_speed_tps / SIM_TICK_HZ` scaling.

**Files:** Modify `src/app_sim_tick.rs:234-285`, and the `sim_speed_tps` reset
sites (`app.rs`, `app_input.rs`, `app_transitions.rs`, `app_spawn_pick.rs`,
`app_dev_overlay.rs`) to carry the speed byte / frame period.

**Pattern:** Existing `schedule_fixed_steps` accumulator; change the threshold
from `SIM_TICK_MS` to `frame_period_ms`.

**Step 1:** Store the current speed byte (or its `frame_period_ms`) on `AppState`
in place of `sim_speed_tps` semantics; default byte 1.
**Step 2:** In `advance_fixed_simulation`, accumulate raw `elapsed_ms` (no tps
scaling) and step once per `frame_period_ms`; byte 0 (`None`) ⇒ step exactly once
per call. Keep `MAX_SIM_STEPS_PER_FRAME` catch-up cap and the leftover clamp.
**Step 3:** `advance_tick` still receives a fixed per-frame `tick_ms` for the
retained `total_sim_ms` accumulator and any legacy ms consumers — pass the
nominal frame duration (`SIM_TICK_MS = 66`), NOT the variable real period (keeps
sim math fixed/deterministic).
**Step 4: Test** — scheduler steps N frames for `N × frame_period_ms` injected
wall time at a given byte; byte 0 steps once per call (mocked clock).
**Step 5: Commit** — `app: scheduler paces one frame per speed-byte period`

---

### Task 6: Update tests that encode the 45/22 ms model

**Why:** Existing tests assert the drifted model; update to the frame model.

**Files:** `src/app_types.rs` (tps tests → frame_period), `src/sim/world/world_hash.rs`
+ `world_tests.rs` (binary_frame derivation → 1:1 tick), `src/sim/miner/miner_tests.rs`
(67 ms-step captures), `src/sim/movement/drive_track_tests.rs` (22 ms steps),
`src/sim/combat/combat_turret_facing_tests.rs`.

**Step 1–N:** For each, replace 45/22-ms expectations with the frame-model values
(binary_frame == tick; drive advances one frame per call at 66 ms; turret/facing
frame counts). Specifically, in `world_hash.rs:1168` change
`assert_eq!(sim.binary_frame, 14)` → `assert_eq!(sim.binary_frame, 45)` (the
adjacent `total_sim_ms == 990` at :1167 stays). Run each module's tests to PASS.
**Commit** — `tests: update timing expectations to one-frame-per-tick model`

---

### Task 7: In-game pace verification vs gamemd

**Why:** Confirms the derived "63 calls/s × 1 frame = ~62.5 fps" chain and catches
per-system regressions the unit tests can't.

**Verify (default skirmish, speed byte 1):**
- Unit move speed (e.g. Grizzly straight line) matches gamemd default — should be
  ~3× faster than before this change.
- Reload cadence, build time, ore growth pace match gamemd default.
- **MCV/vehicle body turn rate matches gamemd** — resolves the H-1 "2× fast" vs
  formula "3× slow" contradiction; should now be ~native (body rotation
  auto-corrects). If still off, escalate the deferred `FacingClass` migration.
- **Sparkle/pixel-fx (railgun) animation speed unchanged** vs before (Task 2 pin).
- **Death-anim length, radar-ping lifetime, world-effect duration unchanged**
  (Task 4 pins).
- No homing-missile / aircraft anomalies (per-call ROT systems unchanged).
- Spot-check a slower speed (byte 4) — should be ~15.6 fps, visibly choppier,
  matching gamemd.
**Method:** run the app (`/run`), side-by-side vs retail gamemd at the same speed
setting; `/fidelity-check` on movement/combat if needed.

---

## Sources & References

- **Design doc:** `docs/plans/2026-05-28-native-frame-rate-design.md`
- **Rate research:** `docs/research/NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION_GHIDRA_REPORT.md`
- **Frame-basis:** `docs/research/FRAME_BASIS_MOVEMENT_TURRET_GHIDRA_REPORT.md`
  (movement lep/frame §2.2, turret §3, body-rotation H-1, ROF §1.3),
  `docs/research/DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md`
- **Timing model:** `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`,
  `docs/research/FRAME_COUNTER_PREINCREMENT_VS_RUST_BINARY_FRAME_GHIDRA_REPORT.md`
- **gamemd.exe addresses:** `Main_Tick 0x0055D360`, `FUN_0055e160` (throttle),
  `GetRadarTimer 0x006C8C40`, `SessionClass__ReadSkirmishSettings 0x00697F10`,
  `GetCurrentSpeed 0x004DB1A0`, `FacingClass::Set 0x004C9220`
- **INI:** `rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1`; `Speed=`, `ROT=`
- **Related code:** `fixed_math.rs:51`, `app_types.rs:27-46`,
  `app_sim_tick.rs:190-285`, `world/mod.rs:1894-1896`, `world_hash.rs:37`,
  `movement_step.rs:42-58`, `combat/mod.rs:73,2318`, `facing_class.rs`, `turret.rs`

## Deferred Follow-Ups (named, not cut)

- **Body rotation → frame-based `FacingClass` migration** (H-1): replace
  `rot_to_facing_delta(rot, tick_ms)` (`turret.rs:32`, called `movement_step.rs:234`)
  with the barrel-style `FacingClass` set/current using `sim.tick`, removing the
  hardcoded `×15` and `clamp(1,128)` distortion. Precision only — body rotation
  auto-corrects to ~native at SIM_TICK_HZ=15; this exactness pass is separate.
  `[FRAME_BASIS_MOVEMENT_TURRET H-1]`
- **Presentation/aging → frame-based migration** (the Task-2 / Task-4
  legacy-22-pinned clocks: `total_sim_ms`/sparkle phase, `radar_events`,
  `world_effects`, crane/damage-fire/muzzle/parachute, voxel/harvest/death anims).
  They stay on the legacy ms cadence; proper frame migration is a separate change.
  `[design "anims deferred"]`
- **Conversion-helper cleanup:** `rof_to_cooldown_ticks` (drop `GAME_FPS=15`,
  use ROF frames directly), `ra2_speed_to_leptons_per_second`/`dt_from_tick_ms` →
  per-frame helpers, remove dead `drive_delay`/`DRIVE_TRACK_SUBTICKS_PER_NATIVE_FRAME`
  sub-gating (no-op at 66 ms). Behavior already correct post-Task-1; clarity only.
- **`total_sim_ms` removal from sim/hash** once render no longer needs it.
- **MP network-frame pacing** (`1000/DAT_00a8b558` path) — separate scheduler.
