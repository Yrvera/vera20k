# Visible Pace Timing Audit - Ghidra/Rust Report

Date: 2026-05-17

Scope:

- Default Yuri's Revenge skirmish speed evidence.
- Whether the "45 FPS" claim is a GameMD default or a VERA20k scheduling choice.
- Current Rust timing surfaces that affect player-visible movement and animation pace.

## 1. Executive Result

Normal default YR skirmish does not expose a verified "45 FPS" default in the checked binary paths. The verified default skirmish setting is the stored game-speed byte `1`, sourced from `rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1` and copied to the live throttle setting. That byte is not an FPS value.

GameMD uses multiple visible clocks:

- Main tick throttle for standard skirmish uses `GetRadarTimer()`, which is `timeGetTime() >> 4`, so the throttle unit is a 16 ms bucket.
- `AnimType Rate=` uses a separate art timing convention: `internal_delay = 900 / Rate`, where `900 = 60 * 15` frames per minute.
- Many visible systems are frame-count based, but not all visible systems use the same clock or conversion path.

Therefore the current Rust claim of `SIM_TICK_HZ=45` is not proven as the GameMD default skirmish tick/FPS. It is a VERA20k implementation choice that must be made safe by mapping every GameMD frame-based visible system onto the correct synthetic clock.

## 2. Verified GameMD Evidence

### Default Skirmish Speed Source

`RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` reads:

```text
RulesClass+0x14A0 = ReadInt("MultiplayerDialogSettings", "GameSpeed", old_value)
```

Checked INI values:

```ini
; rulesmd.ini
[MultiplayerDialogSettings]
GameSpeed=1

; rules.ini
[MultiplayerDialogSettings]
GameSpeed=0
```

YR `rulesmd.ini` patches the base RA2 value, so the default multiplayer/skirmish dialog speed is `1`.

`SessionClass__ReadSkirmishSettings @ 0x00697F10` reads:

```text
settings+0x08 = ReadInt(section, "GameSpeed", RulesClass+0x14A0)
```

The local retail `RA2MD.INI` checked at `C:/Users/enok/Documents/Command and Conquer Red Alert II/RA2MD.INI` has `[Options] GameSpeed=3` and no `[Skirmish] GameSpeed=`, so normal skirmish falls back to rules speed `1`, not options speed `3`.

### Live Speed Propagation

The lobby/session packet path `FUN_005B67F0 @ 0x005B67F0` copies packet byte `+0xA2`:

```text
DAT_00A8B268 = packet+0xA2
DAT_00A8EB60 = DAT_00A8B268
```

This confirms the live throttle setting is the stored speed byte, with the UI slider inverted elsewhere.

`OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0` computes:

```text
stored_speed = 6 - slider_position
DAT_00A8EB60 = stored_speed
```

So higher visible slider position maps to lower stored delay. The stored byte is a speed/delay code, not an FPS value.

### Main Tick Throttle

`GetRadarTimer @ 0x006C8C40` is:

```text
return timeGetTime() >> 4
```

`Main_Tick @ 0x0055D360` standard mode-0/mode-5 path sets:

```text
DAT_00887348 = GetRadarTimer()
DAT_00887350 = DAT_00A8EB60
```

For normal skirmish (`g_GameMode == 5`), the mode-0 speed-2 override is skipped. The speed-2 branch is real, but it is gated on `g_GameMode == 0`.

`FUN_0055E160 @ 0x0055E160` subtracts elapsed `GetRadarTimer()` buckets from `DAT_00887350` and sleeps until the bucket budget is consumed. That means speed `1` is nominally one 16 ms throttle bucket, plus work time and Windows sleep granularity. It is not a hard-coded 45 FPS loop.

### Frame Counter Ordering

`Main_Tick @ 0x0055D360` increments `g_CurrentFrameCounter` late, after input, logic, map logic, render, side work, network/service processing, and only if pause/stop flags allow it.

Rust increments/derives its synthetic `binary_frame` at the start of `Simulation::advance_tick`, which is observably different for timers started and checked within one tick.

## 3. 45 FPS Claim Audit

The only checked direct `45 FPS` claim is in the repo documentation:

- `C:/Users/enok/Documents/ra2-rust-game/docs/index.md` says the sim runs at fixed 45 ticks/sec and refers to 45 FPS as standard multiplayer FPS.

The checked GameMD evidence does not verify that as the default YR skirmish gameplay frame rate. Binary evidence supports:

- stored skirmish speed byte `1`;
- skirmish throttle unit of 16 ms `GetRadarTimer()` buckets;
- art animation `Rate=` conversion using 15 frames/sec as a content timing convention;
- other network/multiplayer branches with millisecond budgets and `0x3c / DAT_00A8B558`, gated away from standard skirmish mode-5 in the checked path.

Conclusion: 45 may be a community label, implementation target, or multiplayer/network effective setting under some conditions, but it is not established as the default GameMD skirmish tick rate by the current verified binary evidence.

## 4. Current Rust Surface Classification

| Surface | Classification | Rust evidence | Player-visible symptom |
|---|---|---|---|
| App fixed-step scheduling / `sim_speed_tps` | Unknown / needs probe | `C:/Users/enok/Documents/ra2-rust-game/src/app_types.rs:24` re-exports `SIM_TICK_HZ`; `:36` maps speed byte to TPS; `:44` defaults to speed 1. `C:/Users/enok/Documents/ra2-rust-game/src/app_sim_tick.rs:222` scales elapsed time by `sim_speed_tps / SIM_TICK_HZ`. | Default speed is now based on the correct stored speed byte, but the effective number of sim ticks per wall second must be measured. |
| `SIM_TICK_HZ=45`, `SIM_TICK_MS=22` | Unknown / risky | `C:/Users/enok/Documents/ra2-rust-game/src/util/fixed_math.rs:47` comments 15 Hz, but `:51` sets 45. `C:/Users/enok/Documents/ra2-rust-game/src/app_types.rs:27` derives 22 ms ticks. | Any raw per-tick visible system can run at a Rust-specific cadence rather than GameMD frame cadence. |
| Movement dt | Unknown / likely too fast unless conversion was intentionally seconds-based | `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs:1063` passes `tick_ms`; `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_tick.rs:351` converts to fixed seconds. | Unit cells/sec may not match retail if `Speed=` was calibrated as frame-based but Rust schedules ~speed-1 bucket-rate ticks. |
| RA2 `Speed=` conversion | Unknown / needs binary or retail movement probe | `C:/Users/enok/Documents/ra2-rust-game/src/util/fixed_math.rs:381` uses `floor(speed * 256 / 60) * 15`; `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_commands.rs:72` applies a development 3x MCV boost. | A known Speed= unit may cross cells too quickly/slowly; MCV movement is deliberately non-retail during diagnosis. |
| Synthetic `binary_frame` derivation/order | Probably too slow for speed-1 wall-clock and one-frame early in ordering | `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs:1014` adds `tick_ms`, then `:1015` computes `binary_frame = total_sim_ms * 15 / 1000` at tick start. | Facing/turret/frame-timer consumers can be phase-shifted and may advance at ~15/sec while standard skirmish logic throttles differently. |
| Gas/smoke/fire particle cadence | Probably too fast for raw particle AI; partially correct for normalized fire spawn | `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs:1407` ticks particles every Rust sim tick. Gas/smoke/fire use `sim.tick` gates in `gas.rs:38`, `gas.rs:146`, `smoke.rs:32`, `fire.rs:185`, `fire.rs:203`. `spawn.rs:152` implements normalized `StateAIAdvance`. | Smoke/gas/fire can animate/drift/spawn at Rust tick speed. FireStream state-advance initialization is much improved, but cadence still needs measurement. |
| Spark/Railgun particles | Unknown for pace because absent | `C:/Users/enok/Documents/ra2-rust-game/src/sim/particles/spawn.rs:42` skips Spark/Railgun; `C:/Users/enok/Documents/ra2-rust-game/src/sim/particles/system_ai.rs:109` no-ops them. | Sparks and railgun trails are missing, not merely wrong-speed. |
| `WorldEffect` / AnimClass-like effects | Probably wrong across speed settings; possibly too slow at speed 1 | `C:/Users/enok/Documents/ra2-rust-game/src/sim/components.rs:563` stores `rate_ms`; `:586` ticks by milliseconds. `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs:1507` ticks effects with `tick_ms`. | Explosions/chrono/wake effects approximate 15fps wall-clock, not full AnimClass frame semantics with normalization and late frame ordering. |
| Building active/idle/damage-fire overlays | Probably wrong; likely too slow at speed 1 if GameMD tick is bucket-throttled | `C:/Users/enok/Documents/ra2-rust-game/src/app_building_anim.rs:25` and `:69` use app wall-clock `dt_ms`; `:207` advances damage fires by `rate_ms`. | Building anims can drift from retail; slot AnimClass replacement/frame-preservation rules are not represented by a wall-clock overlay loop. |
| Infantry animation cadence | Unknown / likely wrong, not simply fast/slow | `C:/Users/enok/Documents/ra2-rust-game/src/rules/infantry_sequence.rs:24` hardcodes ms defaults; `C:/Users/enok/Documents/ra2-rust-game/src/sim/animation.rs:338` advances by elapsed ms. | Infantry walk/fire/prone/death sequences may each differ because GameMD uses action timers and selected normalized actions. |
| SHP vehicle animation cadence | Unknown / likely wrong, not simply fast/slow | `C:/Users/enok/Documents/ra2-rust-game/src/rules/shp_vehicle_sequence.rs:19` hardcodes ms defaults; scan found no `WalkRate`/`IdleRate` consumer. | Dolphins, terror drones, squid, and other SHP vehicles do not follow raw frame modulo gates. |
| Parachute descent | Probably too fast if Rust schedules speed-1 as ~63 22ms ticks/sec | `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/parachute_descent.rs:99` runs one descent integration per sim tick; `tick_ms` only gates pause. | Paradrops may land too quickly relative to retail. |
| PARACH animation | Probably too slow relative to current descent | `C:/Users/enok/Documents/ra2-rust-game/src/rules/ruleset.rs:1111` reads `[PARACH] Rate`; `:1113` converts it to ms. `C:/Users/enok/Documents/ra2-rust-game/src/app_chute_anim.rs:67` advances frames by app `dt_ms`. | Canopy frame loop and falling body can visibly disagree. |
| Garrison muzzle overlays | Probably wrong; likely too slow and wrong ownership/model | `C:/Users/enok/Documents/ra2-rust-game/src/app_building_anim.rs:642` handles only garrison flashes; `:692` hardcodes `67 ms`. | Garrison fire flashes may not match AnimClass timing or port cadence. |
| Non-garrison muzzle overlays | Unknown pace because mostly absent | Current scan only found garrison-specific muzzle flash handling. | Normal shots can lack muzzle flashes entirely. |

## 5. Smallest Empirical Rust Probes

1. Runtime pace counter: for each real wall-clock second, log scheduled fixed steps, `sim.tick` delta, `total_sim_ms` delta, and `binary_frame` delta at default speed.
2. Movement cells/sec: spawn a non-MCV known Speed= unit, issue a straight 10-cell move, record wall seconds, sim ticks, and cells/sec.
3. Particle state probe: spawn `FireStreamSys`, record first particle `state_ai_advance`, `animation_state`, `state_advance_counter`, and lifetime every wall second.
4. PARACH coupled probe: paradrop one infantry from fixed altitude, record wall-clock landing time, sim ticks to landing, PARACH frame changes/sec, and chute frame at landing.
5. AnimClass-like effect probe: spawn a known `Rate=400` effect and count frame changes over wall seconds at default speed.
6. Infantry/SHP probe: log E1 walk/fire frame changes and one SHP vehicle with `WalkFrames` over both wall time and sim ticks.

These probes should stay sim-side or app-orchestration-side only; sim logic must not depend on render/UI/audio/net.

## 6. Prioritized Parity Plan

1. Establish the authoritative GameMD-visible frame clock for standard skirmish speed `1` with a retail/runtime probe, not a docs label.
2. Add the small Rust probes above before retuning constants.
3. Separate movement calibration from animation calibration. First measure known `Speed=` cells/sec and remove/disable the current development MCV boost during diagnosis.
4. Convert generic AnimClass-like effects to frame-delay semantics: `900 / Rate`, `Normalized`, small-rate table, random delays, loop rules, and late frame-counter ordering.
5. Keep special visible cadence systems separate: infantry action timers, SHP vehicle `WalkRate`/`IdleRate`, particles, parachute descent, and superweapon/temporal/gap state machines should not be forced through one millisecond overlay loop.
6. Finish missing visible systems before declaring pace parity: Spark/Railgun and normal `WeaponType.Anim=` muzzle flashes are currently absent.
7. After clocks are unified, re-audit same-tick ordering against GameMD's late `g_CurrentFrameCounter++`.

## 7. Open Questions

1. What is the measured retail GameMD `g_CurrentFrameCounter` delta per wall-clock second in a default local YR skirmish with stored speed `1`?
2. Does the observed retail loop settle near one 16 ms bucket plus work time, or does a separate runtime path clamp/pace it under real play conditions?
3. Which branch, if any, is the source of the repo's "45 FPS standard multiplayer" statement? The standard skirmish mode-5 path checked here did not prove it.
4. What is the retail cells/sec for a controlled `Speed=4` and `Speed=8` movement probe at stored speed `1`?

## Sources

- Ghidra decompile: `Main_Tick @ 0x0055D360`
- Ghidra decompile: `FUN_0055E160 @ 0x0055E160`
- Ghidra decompile: `GetRadarTimer @ 0x006C8C40`
- Ghidra decompile: `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`
- Ghidra decompile: `SessionClass__ReadSkirmishSettings @ 0x00697F10`
- Ghidra decompile: `OptionsClass__SetDefaults @ 0x005FA350`
- Ghidra decompile: `OptionsClass__ReadFromINI @ 0x005FA620`
- Ghidra decompile: `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0`
- Ghidra decompile: `FUN_0069BAB0 @ 0x0069BAB0`
- Ghidra decompile: `FUN_0069BB40 @ 0x0069BB40`
- Ghidra decompile: `FUN_005B67F0 @ 0x005B67F0`
- Existing report: `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SPEED_AND_PARTICLE_NORMALIZED_GHIDRA_REPORT.md`
- Existing report: `C:/Users/enok/Documents/ra2-rust-game-docs/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
- Existing report: `C:/Users/enok/Documents/ra2-rust-game-docs/TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md`
- Existing report: `C:/Users/enok/Documents/ra2-rust-game-docs/TICK_ANIMATION_VISIBLE_LEFTOVERS_GHIDRA_REPORT.md`
- Existing report: `C:/Users/enok/Documents/ra2-rust-game-docs/PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md`
- Rust files cited in section 4.
