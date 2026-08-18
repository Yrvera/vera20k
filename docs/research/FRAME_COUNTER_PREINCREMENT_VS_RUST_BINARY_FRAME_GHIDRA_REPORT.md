# Frame Counter Preincrement vs Rust Binary Frame - Ghidra Report

**Date:** 2026-05-28  
**Target:** `FRAME_COUNTER_PREINCREMENT_VS_RUST_BINARY_FRAME`  
**Address(es):** `Main_Tick @ 0x0055D360`, `LogicClass::PerTickUpdate @ 0x0055AFB0`, `CDTimerClass::GetTimeRemaining @ 0x00426630`, `CDTimerClass::Start @ 0x0046B640`  
**Active in YR:** Yes. `Main_Tick`, `LogicClass::PerTickUpdate`, `CDTimerClass`, frame-counter modulo gates, and the Rust surfaces compared here are all on normal Yuri's Revenge gameplay paths.  
**Status:** COMPLETE for the ordering question and Rust exposure inventory. Wall-clock retail frame-rate measurement remains outside this target.

## Target Question

Does active YR expose the old `g_CurrentFrameCounter` value during `LogicClass::PerTickUpdate` and most per-frame work, incrementing the global frame counter only late in `Main_Tick`; and does current Rust instead derive `Simulation::binary_frame` at the start of `Simulation::advance_tick`, exposing a next-frame value during the same update?

## Non-goals

- Do not re-audit the full `LogicClass::PerTickUpdate` subsystem order.
- Do not measure retail wall-clock FPS or decide whether the app's fixed-step rate should be 45, 60, or bucket-derived.
- Do not implement or patch Rust timing.
- Do not mutate Ghidra state.

## Evidence Needed To Mark COMPLETE

- Direct `Main_Tick` evidence that `LogicClass::PerTickUpdate` is called before `g_CurrentFrameCounter++`.
- Direct `LogicClass::PerTickUpdate` evidence that it reads `g_CurrentFrameCounter` for active timers/modulo gates.
- Direct timer evidence that starts and remaining-time reads use `g_CurrentFrameCounter`.
- Current Rust evidence showing `total_sim_ms`/`binary_frame` update at the beginning of `Simulation::advance_tick`.

All four evidence requirements are met below.

## Verified Binary Facts

### 1. `Main_Tick` calls `LogicClass::PerTickUpdate` before the late frame increment

Read-only Ghidra decompile of `Main_Tick @ 0x0055D360` in this pass shows the normal work sequence includes `GScreenClass__Input`, `LogicClass__AI`, optional `House_AI_Tick`, `Map__Logic`, `RenderFrame_main`, side work, then:

```text
LogicClassPerTickUpdateLiveVector();
...
Network_ServiceLoop();
if DAT_00A83D49 == 0
and DAT_00A8ECD0 == 0
and DAT_008B41C0 == 0
and DAT_00A83D48 == 0:
    g_CurrentFrameCounter = g_CurrentFrameCounter + 1
```

This places `LogicClass::PerTickUpdate @ 0x0055AFB0` before the guarded increment. Existing audited documentation records the same claim in `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` and `AUDIT_LOG.md`; the audit log marks the late increment after `Network_ServiceLoop` as confirmed.

### 2. `LogicClass::PerTickUpdate` reads the pre-increment frame counter

Read-only Ghidra decompile of `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` shows active reads of `g_CurrentFrameCounter` before `Main_Tick` increments it. Examples in the same function:

- Scenario cell-action timer checks compute `g_CurrentFrameCounter - start_frame < duration`.
- Bridge shroud recalculation runs on `(int)g_CurrentFrameCounter % 0x78 == 0`.
- Ore-adjacent growth/spread timer setup writes current frame-derived start values before calling `TiberiumClass__GrowthDriver_AllTypes()` and `TiberiumClass__SpreadDriver_AllTypes()`.

Therefore per-tick systems in `LogicClass::PerTickUpdate` see frame `N`, not `N+1`, during the update that will later increment to `N+1`.

### 3. `CDTimerClass` stores and reads the current global frame

Read-only Ghidra decompile of `CDTimerClass::Start @ 0x0046B640`:

```text
*timer = g_CurrentFrameCounter;
timer[2] = duration;
```

Read-only Ghidra decompile of `CDTimerClass::GetTimeRemaining @ 0x00426630`:

```text
duration = timer[2];
if timer[0] != -1:
    elapsed = g_CurrentFrameCounter - timer[0];
    if elapsed < duration:
        return duration - elapsed;
    return 0;
return duration;
```

This confirms the timer boundary is frame-counter based and read-derived, not self-decrementing. A timer started and checked later in the same `Main_Tick` still sees elapsed `0` because the global counter has not advanced yet.

## Current Rust Facts

### 4. `Simulation::advance_tick` exposes a precomputed synthetic frame at tick start

Current Rust updates the synthetic frame at the top of `Simulation::advance_tick`:

```rust
self.total_sim_ms = self.total_sim_ms.saturating_add(tick_ms as u64);
self.binary_frame = ((self.total_sim_ms * 15) / 1000) as u32;
let execute_tick = self.tick.saturating_add(1);
```

Source: `src/sim/world/mod.rs:1187..1201`.

Current app scheduling calls `sim.advance_tick(..., SIM_TICK_MS)` inside the fixed-step loop, then advances several animation systems with the same `SIM_TICK_MS`. Source: `src/app_sim_tick.rs:269..312`. `SIM_TICK_MS` is `1000 / SIM_TICK_HZ`; current `SIM_TICK_HZ` is 45, so the integer step is 22 ms. Sources: `src/app_types.rs:24..27`, `src/util/fixed_math.rs:51`.

### 5. Rust systems most exposed to one-frame or timer drift

The highest-risk surfaces are consumers that treat `binary_frame` or `sim.tick` as the native frame source inside the same tick that starts or checks a timer:

- Gate runtime: `tick_gate_runtimes(..., self.binary_frame)` in `src/sim/world/mod.rs:1267..1274`.
- Combat and turret/facing: `tick_combat_with_fog(..., self.tick, tick_ms, self.binary_frame)` and `turret::tick_turret_rotation(..., self.binary_frame)` in `src/sim/world/mod.rs:1447..1468`.
- `FacingClass`: `set`, `snap`, `current`, and `is_rotating` store/check `start_frame` against caller-supplied `binary_frame` in `src/sim/movement/facing_class.rs:80..163`.
- Native ore growth/spread and terrain spawning: `tick_native_growth_driver`, `tick_native_spread_driver`, and `terrain_spawn` receive `self.binary_frame` in `src/sim/world/mod.rs:1693..1744`.
- Miner dock/mission timing: miner dock sequence stores and checks `sim.binary_frame` for retry/unload/deploy waits in `src/sim/miner/miner_dock_sequence.rs`.
- Presentation-like world effects and generic entity animations use millisecond elapsed time (`tick_ms`) instead of frame-counter `CDTimerClass` semantics in `src/sim/world/mod.rs:1806..1821` and `src/sim/animation.rs:334..352`.
- Production/combat cooldowns convert frame counts to milliseconds/fixed-step ticks, notably `rof_to_cooldown_ticks` in `src/sim/combat/mod.rs:2318..2328` and production queue progression in `src/sim/production/production_queue.rs`.
- Superweapon duration paths use `sim.tick as u32` for Iron Curtain / Force Shield starts, which is late-incremented at the end of `advance_tick` but is a separate Rust tick counter, not the same as `binary_frame`.

## Inference

The binary behavior is old-frame-visible-during-update. Current Rust behavior is boundary-precomputed-frame-visible-during-update for every subsystem wired to `binary_frame`. When `total_sim_ms + tick_ms` crosses a 15 Hz boundary, Rust exposes `binary_frame == N+1` at the beginning of the fixed step. Native `Main_Tick` would expose `g_CurrentFrameCounter == N` throughout `LogicClass::PerTickUpdate`, `CDTimerClass` checks, active-object AI, factories, houses, render, and service work, then increment to `N+1` late.

This is a real parity hazard even when a Rust timer duration equals the native duration, because the start/check ordering can be shifted by one frame. It also compounds with the separate unresolved wall-clock pace question: `binary_frame` is a 15 Hz synthetic frame, while Rust fixed steps currently run at 45 Hz base scheduling and speed scaling.

## Implementation Handoff

- Introduce or document a per-update "visible native frame" value: systems that model `g_CurrentFrameCounter` should read the pre-increment frame for the duration of an update and advance it only at the native-equivalent late boundary.
- Audit all current `binary_frame` consumers and classify them as frame-counter timers, presentation wall-clock effects, or Rust-only scheduling. Do not assume a single `tick_ms` or `sim.tick` replacement is parity-safe.
- Add focused regression scenarios for same-update start/check behavior: `CDTimerClass`-style timer start then query, `FacingClass::set/current` on a boundary-crossing step, gate transition start/check, ore growth queue priority, and one combat ROF/turret case.

## Negative Facts / Do Not Do

- Do not treat `SIM_TICK_HZ = 45` or `SIM_TICK_MS = 22` as `g_CurrentFrameCounter`.
- Do not move every timer to milliseconds; native `CDTimerClass`, `RateTimer`, `AnimClass`, and many modulo gates are frame-counter based.
- Do not "fix" this by adding one to every `binary_frame` consumer. Native uses old frame during update and increments late; some Rust consumers using `sim.tick` already have different timing semantics.
- Do not collapse `binary_frame`, `sim.tick`, and app wall-clock scheduling into one clock without per-system evidence.

## Remaining Uncertainty

- The exact retail wall-clock increment rate for `g_CurrentFrameCounter` in a default local YR skirmish was not measured in this slot.
- This slot did not fully reconcile subsystem order inside `LogicClass::PerTickUpdate`; it only used enough order evidence to place `LogicClass` before the frame increment.
- Some Rust consumers may already be locally compensated by tests or by using `self.tick` instead of `binary_frame`; each consumer still needs a per-system parity audit.

## Stop Conditions

- Stop before implementation; this report is research-only.
- Stop before broad subsystem-order claims not needed for the frame-counter placement question.
- Stop if Ghidra evidence contradicted the audited global timing report. It did not.

## Stale-Doc Wording

No behavioral stale-doc correction found for `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`; its core late-increment claim remains confirmed. Some older line references in timing docs point at prior `world/mod.rs` line numbers, but the current code still performs the same start-of-`advance_tick` update at `src/sim/world/mod.rs:1199..1200`.

## Sources

- Read-only Ghidra decompile: `Main_Tick @ 0x0055D360`.
- Read-only Ghidra decompile: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.
- Read-only Ghidra decompile: `CDTimerClass::GetTimeRemaining @ 0x00426630`.
- Read-only Ghidra decompile: `CDTimerClass::Start @ 0x0046B640`.
- `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`.
- `docs/research/VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md`.
- `docs/research/AUDIT_LOG.md`.
- Rust source files cited above.
