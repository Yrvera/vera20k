# Garrison Shot Cadence Postfix Trace

Date: 2026-05-27
Slot: trace-swarm slot 3
Scenario: one occupied building fires repeated ordinary garrison shots; each shot spawns the weapon `OccupantAnim` muzzle flash. Concrete stock case: a GI-style occupant weapon using `OccupantAnim=UCFLASH`.

## Pipeline

Shot fire in object AI -> `TechnoClass::Fire_At` selects `WeaponType+0x110` -> `AnimClass::Constructor(delay=0, loopCount=1, drawFlags=0x600)` -> native `Middle()` starts immediately -> native logic-vector / `AnimClass::AI` frame countdown -> render draws current anim frame.

Rust pipeline: fixed sim tick emits `SimFireEvent` -> app drains fire events into `pending_fire_effects` -> `tick_garrison_muzzle_flashes` advances existing flashes -> same function spawns new `GarrisonMuzzleFlash` from pending event -> render draws `GarrisonMuzzleFlash.frame`.

## Scenario Inputs

- Stock `UCFLASH` art section has `Layer=ground` and `Translucent=yes`, with no `Rate=`, `Start=`, `End=`, `LoopStart=`, `LoopEnd=`, `LoopCount=`, or `Next=` (`ini/artmd.ini:16131..16133`).
- Active YR shot-triggered garrison flash is ordinary `TechnoClass::Fire_At`, not the chrono/warp `BuildingClass::Update` sparkle branch (`docs/research/CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md:108..135`, `224..227`).
- Read-only Ghidra spot-checks this run confirmed:
  - `AnimTypeClass::Constructor @ 0x00427530` initializes `AnimType+0x2B0` / `param_1[0xac]` to `1`.
  - `AnimTypeClass::ReadINI @ 0x00427D00` only overwrites that field when `Rate=` exists, using `900 / Rate` for positive values and `0` for non-positive values.
  - `AnimClass::Constructor @ 0x00421EA0` copies that rate into frame-delay fields, sets current frame to `0`, and calls `AnimClass::Middle()` immediately when constructor `delay=0`.
  - `AnimClass::AI @ 0x00423AC0` advances by `CurrentFrame += FrameStep` when the timer reaches zero and reloads from the copied rate.

## Stage Verdicts

| Stage | gamemd output | Rust output | Verdict |
|---|---|---|---|
| Active YR shot anim source | Occupied `BuildingClass` `Fire_At` uses `WeaponType+0x110` (`OccupantAnim`) for the shot flash; active for ordinary garrison shots (`0x006FF320..0x006FF41D`, `docs/research/CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md:108..116`). | Rust pending fire event carries `occupant_anim`; `tick_garrison_muzzle_flashes` filters events with `ev.occupant_anim` (`src/app_building_anim.rs:727..736`). | PASS: concrete source is `UCFLASH == UCFLASH`. |
| Constructor/default Rate for stock `UCFLASH` | No stock `Rate=` key, so `AnimTypeClass::Constructor` default remains internal frame delay `1`; `ReadINI` does not touch it when key is absent. | `DEFAULT_ART_RATE_LOGIC_FRAMES = 1`; absent `Rate` stores and returns `rate_logic_frames = 1` (`src/rules/art_data.rs:227..228`, `391..394`, `557..562`; `src/app_building_anim.rs:767..779`). | PASS: `1 == 1` logic tick per frame for stock `UCFLASH`. |
| Explicit `Rate=` conversion used by this path | Native positive `Rate=` stores `900 / Rate`; example `Rate=300` stores `3`. `Rate<=0` stores `0`. Active generic AnimType parser in YR (`0x00427D00`). | `art_rate_to_logic_frames(300)` returns `3`; `Rate<=0` returns `0` (`src/rules/art_data.rs:203..212`). | PASS for conversion mechanism: `3 == 3`, `0 == 0`. |
| Existing-flash per-tick advancement | For a live anim with copied rate `1`, one `AnimClass::AI` game-tick can advance current frame by one: `frame N -> N+1`; timer reload is `1` (`AnimClass::AI @ 0x00423AC0`, `docs/research/timing/animation-rate-delay.md:248..258`, `339..351`). | For an existing `GarrisonMuzzleFlash` with `rate_logic_frames=1`, `dt_ms = 1 * SIM_TICK_MS` makes `elapsed_logic_ms >= delay_ms`, so Rust advances `frame N -> N+1` and subtracts the delay (`src/app_building_anim.rs:689..700`, `760..764`; caller computes elapsed fixed ticks at `src/app_sim_tick.rs:176..200`). | PASS for already-existing flashes: one elapsed fixed sim tick advances exactly one frame. |
| Elapsed-time source after app fix | Native anim timers are driven by game-tick `AnimClass::AI`, not render-frame wall time (`docs/research/timing/animation-rate-delay.md:328..351`). | Rust now captures `sim.tick` before/after `advance_fixed_simulation` and passes `elapsed_ticks * SIM_TICK_MS`, not app `elapsed_ms`, into `tick_garrison_muzzle_flashes` (`src/app_sim_tick.rs:176..200`). | PASS for source class: render-frame wall time is no longer consumed by this path. |
| Newly spawned flash first advance | Native objects inserted during the main forward logic-vector pass can be AI-updated in that same tick because `LogicClass::PerTickUpdate` reloads the vector count after each iteration; this is verified for same-tick bullet AI (`docs/research/AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md:26..27`, `72`). `AnimClass::Constructor` uses the normal object reveal/registration path and `delay=0`; for stock `UCFLASH` rate `1`, the first same-pass `AnimClass::AI` can produce visible frame `1` before the next render. | Rust advances existing flashes first, then spawns new flashes with `frame=0` and `elapsed_logic_ms=0`; the comment explicitly says newly constructed flashes are not advanced in the same call (`src/app_building_anim.rs:690..704`, `738..753`). First post-shot Rust render therefore shows frame `0` for one fixed tick longer. | FAIL: initial-frame cadence is one fixed tick late for shots created during the native object-AI pass. Concrete first post-shot frame can be native `1` vs Rust `0`. |
| Full proof of `AnimClass` insertion order for this exact occupied-shot constructor | The scheduler and constructor evidence strongly imply same-pass AI, but this trace did not live-breakpoint a garrison `Fire_At` shot and inspect the exact inserted `AnimClass` index relative to the current logic-vector cursor. | Rust code order is directly visible. | UNCHECKED: the FAIL above is high-confidence from shared scheduler evidence, but the exact occupied-shot insertion cursor was not live-captured in this run. |

## Timing Summary

- Native stock `UCFLASH` selected by an occupied shot: constructor current frame starts at `0`, rate delay is `1` game tick, and the anim uses generic `AnimClass::AI`.
- Rust stock `UCFLASH`: new app-layer flash starts at `frame=0`, delay is `1 * SIM_TICK_MS`, and existing flashes advance once per elapsed fixed tick.
- Remaining cadence drift: Rust intentionally prevents a newly spawned flash from being advanced during the same app tick that spawned it; native scheduling can update newly inserted objects during the same logic-vector pass.

## Adjacent Findings

- Generic `AnimClass` lifecycle fields (`End`, `LoopStart`, `LoopEnd`, `LoopCount`, `Next`, `Shadow`, `PingPong`, `RandomRate`) remain outside this cadence-only trace.
- Rust clamps `rate_logic_frames=0` to one tick in `garrison_occupant_anim_delay_ms`; that matters for modded/static `Rate=0` anims, but the concrete stock `UCFLASH` scenario has no `Rate=0`.
- Z-adjust/depth and draw ordering are separate trace slots and were not expanded here.

## Verdict Tally

PASS: 5 | FAIL: 1 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Top Player-Visible Findings

1. Stage: newly spawned flash first advance. Player-visible difference: each occupied-shot flash can linger on frame `0` for one extra fixed tick before advancing, making the muzzle flash cadence one tick late at spawn. Rust: `src/app_building_anim.rs:690..704`, `738..753`. gamemd evidence: `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::AI @ 0x00423AC0`, dynamic logic-vector count evidence in `docs/research/AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md:26..27`, `72`.

Status: COMPLETE
