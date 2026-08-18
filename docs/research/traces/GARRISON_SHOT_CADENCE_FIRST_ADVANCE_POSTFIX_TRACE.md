# Garrison Shot Cadence First-Advance Postfix Trace

Date: 2026-05-27
Slot: trace-swarm slot 1
Scenario: one occupied building fires one ordinary garrison shot using stock `OccupantAnim=UCFLASH`. Trace only whether Rust now spawns the garrison muzzle flash before the fixed-tick advancement step, so the first rendered post-shot frame can advance on the same fixed tick as active YR `AnimClass` can advance newly inserted logic objects.

## Pipeline

Native: occupied-building fire in object AI -> `TechnoClass::Fire_At` selects `WeaponType+0x110` (`OccupantAnim`) -> `AnimClass::Constructor(type=UCFLASH, delay=0, loopCount=1, drawFlags=0x600, zAdjust=0)` -> constructor calls `Middle()` immediately and reveals/registers the anim -> main logic-vector pass may call `AnimClass::AI` for newly appended objects because the vector count is re-read after each item.

Rust: fixed sim tick emits `SimFireEvent { garrison_muzzle_index, occupant_anim=UCFLASH }` -> app computes elapsed fixed ticks -> `tick_garrison_muzzle_flashes` spawns new `GarrisonMuzzleFlash` first -> the same call advances all flashes with `dt_ms = elapsed_fixed_ticks * SIM_TICK_MS` -> render draws `GarrisonMuzzleFlash.frame`.

## Scenario Inputs

- `ini/artmd.ini:16131..16133`: `[UCFLASH]` has `Layer=ground`, `Translucent=yes`, and no `Rate=`, `Start=`, `End=`, `LoopStart=`, `LoopEnd=`, `LoopCount=`, or `Next=`.
- `ini/art.ini:11583..11585` matches the same stock base section shape.
- `src/util/fixed_math.rs:51` defines `SIM_TICK_HZ = 45`; `src/app_types.rs:27` makes `SIM_TICK_MS = 1000 / 45 = 22` integer milliseconds.
- Prior verified report: `docs/research/CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md:120..133` says shot-triggered garrison `OccupantAnim` uses generic `AnimClass`, absent stock `Rate=` keeps constructor/default rate `1`, and active YR ordinary occupied shots use this path.

## Stage Verdicts

### Stage 1 - Active shot anim source

Input: occupied building fires one ordinary garrison shot with a weapon whose `OccupantAnim` is `UCFLASH`.

gamemd: `TechnoClass::Fire_At @ 0x006FF320` reads the selected weapon and, when the occupied-building branch applies, constructs an `AnimClass` from `WeaponType+0x110`. The decompiled active YR path constructs `AnimClass__Constructor(iVar9, &uStack_98, 0, 1, 0x600, 0, 0)` when `iVar9` is nonzero.

Rust: combat emits `occupant_anim` only for garrison shots at `src/sim/combat/mod.rs:2031..2055`; app filters `ev.occupant_anim` at `src/app_building_anim.rs:721..729`.

Verdict: PASS. Concrete anim id is `UCFLASH` on both paths for this stock scenario.

### Stage 2 - Stock UCFLASH frame delay

Input: stock `[UCFLASH]` has no `Rate=`.

gamemd: `AnimTypeClass::Constructor @ 0x00427530` initializes the internal rate to `1`; `AnimTypeClass::ReadINI @ 0x00427D00` only replaces it when `Rate=` exists. `AnimClass::Constructor @ 0x00421EA0` copies that type rate into `param_1[0x2f]` and `param_1[0x30]`.

Rust: `garrison_occupant_anim_rate_logic_frames` returns the art registry default when `Rate=` is absent; tests assert `DEFAULT_ART_RATE_LOGIC_FRAMES == 1` behavior at `src/app_building_anim.rs:811..821`. Delay conversion is `1 * SIM_TICK_MS = 22ms` at `src/app_building_anim.rs:769..773`.

Verdict: PASS. Native frame delay `1` logic tick equals Rust `rate_logic_frames=1` for stock `UCFLASH`.

### Stage 3 - Native delay-zero constructor start

Input: `AnimClass::Constructor(..., delay=0, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)`.

gamemd: `AnimClass::Constructor @ 0x00421EA0` stores `param_4` into `param_1[0x61]`, sets current frame field `param_1[0x2b] = 0`, and calls `AnimClass__Middle()` immediately when `param_1[0x61] == 0`.

Rust: new `GarrisonMuzzleFlash` starts with `frame: 0`, `elapsed_logic_ms: 0`, `rate_logic_frames: 1` at `src/app_building_anim.rs:731..745`.

Verdict: PASS for constructed starting state. Both paths start the visible one-shot anim at frame `0` before the first AI/advance step.

### Stage 4 - Native live logic-vector count

Input: an object is appended to the main logic vector during the active per-tick loop.

gamemd: `LogicClass::PerTickUpdate @ 0x0055B608` loops over `*(param_1 + 0x10)` and reloads that count after each vtable `+0x5C` AI call. `ObjectClass::Reveal @ 0x005F4F60` can call `FUN_0055BAA0`; `FUN_0055BAA0 @ 0x0055BAA0` calls `DynamicVector__Insert` when object byte `+0x98 == 0`; `DynamicVector__Insert @ 0x005519B0` appends at `vector[count]` and increments count.

Rust: not a native global object vector; app receives fixed-tick fire events after `advance_fixed_simulation`.

Verdict: PASS for native scheduler capability only. Active YR can AI-update newly appended logic objects in the same per-tick pass.

### Stage 5 - Rust spawn-before-advance order after local fix

Input: one pending fire event with `occupant_anim=UCFLASH`; one fixed sim tick elapsed in this app update.

Rust: `tick_garrison_muzzle_flashes` now builds `new_flashes` first at `src/app_building_anim.rs:698..749`, extends `state.garrison_muzzle_flashes` at `src/app_building_anim.rs:750`, then calls `retain_mut(|flash| advance_garrison_muzzle_flash(flash, dt_ms))` at `src/app_building_anim.rs:752..756`.

Computation: new flash starts `frame=0`, `elapsed_logic_ms=0`; caller passes `dt_ms = 1 * SIM_TICK_MS = 22` from `src/app_sim_tick.rs:176..200`; `advance_garrison_muzzle_flash` adds `22`, compares against `delay_ms=22`, subtracts `22`, and increments frame to `1` at `src/app_building_anim.rs:759..766`. The focused helper test asserts exactly this at `src/app_building_anim.rs:837..858`.

gamemd comparison: same-pass-capable native `AnimClass::AI` with copied rate `1` advances by one frame when its timer expires; `AnimClass::AI @ 0x00423AC0` increments `param_1[0x2b]` by `param_1[0x31]` and reloads the copied rate when the timer has no remaining time.

Verdict: PASS for the local first-advance defect. In the one-fixed-tick post-shot case, Rust now reaches `frame 1`, not the previous `frame 0`.

### Stage 6 - App fixed-tick elapsed source

Input: app update advances simulation by `N` fixed ticks.

Rust: `advance_in_game_runtime` captures `garrison_flash_start_tick`, advances fixed simulation, computes `garrison_flash_elapsed_ticks`, then passes `elapsed_ticks * SIM_TICK_MS` into `tick_garrison_muzzle_flashes` at `src/app_sim_tick.rs:176..200`.

gamemd: `AnimClass::AI` is game-logic tick driven, not render wall-time driven.

Verdict: PASS for the concrete one-tick scenario (`N=1`). The consumed class of time is fixed logic ticks, not render-frame elapsed time.

### Stage 7 - Exact occupied-shot insertion cursor

Input: this exact occupied-building `Fire_At` call constructs `UCFLASH` during a particular position in the main logic-vector pass.

gamemd: the decompiled functions prove delay-zero construction, reveal/append mechanics, and live-vector count reload. This run did not live-breakpoint the exact occupied-building shot and record the new `AnimClass` index relative to the current logic-vector cursor.

Rust: direct code order is known.

Verdict: UNCHECKED. The intended native same-tick behavior is well-supported by active YR functions, but exact cursor equality for this concrete occupied-shot instance was not captured in this trace.

### Stage 8 - Multi-tick app catch-up batches

Input: app frame advances more than one fixed sim tick and the shot occurs in one of those internal ticks.

Rust: new flashes receive the full `garrison_flash_elapsed_ticks * SIM_TICK_MS` after all fixed ticks in the batch, because pending fire events do not carry their creation tick.

gamemd: anim AI advances according to the tick in which the anim is inserted.

Verdict: UNCHECKED for this concrete one-shot/first-frame trace. The requested one-fixed-tick first-advance case is fixed, but exact catch-up-batch attribution was not traced here.

## Timing Summary

- Before the local fix, Rust advanced existing flashes and only then appended newly spawned flashes, making the first post-shot draw stay at frame `0`.
- After the local fix, Rust appends new `GarrisonMuzzleFlash` objects before the same fixed-tick advance pass.
- For stock `UCFLASH`, absent `Rate=` gives native delay `1` logic tick and Rust `rate_logic_frames=1`; with one elapsed fixed tick, Rust computes `frame 0 -> 1`.
- Full native cursor capture and multi-tick catch-up attribution remain unchecked, not silently passed.

## Adjacent Findings

- Generic `AnimClass` lifecycle parity (`End`, `LoopStart`, `LoopEnd`, `LoopCount`, `Next`, `Shadow`, `PingPong`, `RandomRate`) remains outside this first-advance trace.
- ZAdjust/depth is a separate trace slot and was not evaluated here.
- Multi-tick app catch-up may need fire-event creation tick metadata if traces later require exact same-frame advancement across batched fixed ticks.

## Verdict Tally

PASS: 6 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Top Player-Visible Findings

No FAIL or NOT-IMPLEMENTED findings in this concrete one-fixed-tick first-advance trace.

Status: COMPLETE
