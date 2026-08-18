# Global Timing Model - Ghidra Research Report

**Date:** 2026-05-18  
**Address(es):** `0x0055D360` (`Main_Tick`), `0x0055E160` (main wait/throttle helper), `0x006C8C40` (`GetRadarTimer`), `0x00426630` (`CDTimerClass__GetTimeRemaining`), `0x0046B640` (`CDTimerClass__Start`), `0x004C9480` (`CDTimerClass__Remaining` bool form), `0x004C9220` (`RateTimer__Set`), `0x004C93D0` (`RateTimer__Current`), `0x00427D00` (`AnimTypeClass__ReadINI`), `0x00421EA0` (`AnimClass__Constructor`), `0x00423AC0` (`AnimClass__AI`), `0x005FA350` (`OptionsClass__SetDefaults`), `0x005FA620` (`OptionsClass__ReadFromINI`), `0x004E1DE0` (`OptionsClass__ApplyFromInGameDialog`), `0x00671EA0` (`RulesClass__ReadMultiplayerDialogSettings`), `0x00697F10` (`SessionClass__ReadSkirmishSettings`), `0x005B67F0` (session/game-option packet apply), `0x0055AFB0` (`LogicClassPerTickUpdateLiveVector`, formerly documented as `LogicClass__PerTickUpdate`; same address/behavior — re-confirmed 2026-05-29 via `get_function_by_address 0x0055AFB0`)  
**Confidence:** High for static binary ordering, frame-counter timers, local skirmish speed source, INI parsing units, and current Rust timing-surface inventory. Medium for the exact wall-clock frame rate a retail process produces under load, because this pass did not attach to a live `gamemd.exe` process.  
**Active in YR:** Yes. The local skirmish path, options path, `CDTimerClass`, `RateTimer`, `AnimClass`, and `LogicClass__PerTickUpdate` paths are active in normal Yuri's Revenge. Network-specific branches are active only for network/multiplayer modes. Scenario-delay render-only branches are conditional.

## 1. Overview

`gamemd.exe` does not have a single simple "FPS constant" that explains all player-visible timing. It has one authoritative gameplay frame counter, `g_CurrentFrameCounter @ 0x00A8ED84`, plus a separate wall-clock throttle path that decides when the next frame can finish. Most gameplay timers store `g_CurrentFrameCounter` plus a duration in frames, while local skirmish pacing uses `GetRadarTimer() == timeGetTime() >> 4` as a 16 ms bucket source.

The important parity result is that the game-speed byte is a throttle budget, not a frame-rate number. The binary uses a mix of frame-count timers, throttle buckets, millisecond network budgets, INI minute-to-frame conversions, `AnimType Rate=900/Rate`, normalized animation delay adjustment, and per-frame modulo gates. A replacement engine must map each timing family explicitly; treating all "ticks" as the same unit will drift.

## 2. Key Globals And Fields

| Global / Field | Address / Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `g_CurrentFrameCounter` | `0x00A8ED84` | Authoritative gameplay frame counter used by `CDTimerClass`, `RateTimer`, `AnimClass`, modulo gates, scenario timers, and many per-tick systems. | `Main_Tick`, `CDTimerClass__GetTimeRemaining`, `AnimClass__AI`, `LogicClass__PerTickUpdate` | Yes |
| `DAT_00A8EB60` | global | Live stored game-speed byte / throttle code. Lower values are faster. | `Main_Tick`, `OptionsClass__ApplyFromInGameDialog`, session packet apply | Yes |
| `DAT_00887348` | global | Start value for local `GetRadarTimer()` throttle bucket. `-1` disables elapsed subtraction. | `Main_Tick`, `FUN_0055E160` | Yes |
| `DAT_00887350` | global | Local throttle budget in `GetRadarTimer()` buckets for modes `0` and `5`. | `Main_Tick`, `FUN_0055E160` | Yes |
| `DAT_00887328` | global | Network/replay throttle start time from `timeGetTime()` milliseconds. | `Main_Tick`, `FUN_0055E160` | Conditional |
| `DAT_00887330` | global | Network/replay throttle budget in milliseconds. | `Main_Tick`, `FUN_0055E160` | Conditional |
| `RulesClass+0x14A0` | field | `[MultiplayerDialogSettings] GameSpeed` default. | `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` | Yes |
| Skirmish settings `+0x08` / `param_1[2]` | field | `[Skirmish] GameSpeed`, falling back to `RulesClass+0x14A0`. | `SessionClass__ReadSkirmishSettings @ 0x00697F10` | Yes |
| `CDTimerClass.start_frame` | `+0x00` | `g_CurrentFrameCounter` value when timer starts; `-1` means paused/not started. | `0x0046B640`, `0x00426630` | Yes |
| `CDTimerClass.duration` | `+0x08` | Countdown duration in game frames. | `0x0046B640`, `0x00426630` | Yes |
| `RateTimer.timer.start_frame` | `+0x08` inside RateTimer | Frame when facing/interpolation timer starts. | `0x004C9220`, `0x004C93D0` | Yes |
| `RateTimer.timer.duration` | `+0x10` inside RateTimer | `abs(delta) / rate`, integer division. | `0x004C9220`, `0x004C93D0` | Yes |
| `RateTimer.rate` | `+0x14` short | Per-frame interpolation divisor; `rate < 1` snaps. | `0x004C9480`, `0x004C9220`, `0x004C93D0` | Yes |
| `AnimType::Rate` | `type+0x2B0` | Internal frame delay, parsed as `900 / INI Rate`. | `AnimTypeClass__ReadINI @ 0x00427D00` | Yes |
| `AnimType::Normalized` | `type+0x362` | Enables normalized delay adjustment after `900 / Rate`. | `AnimTypeClass__ReadINI`, `AnimClass__Constructor`, `AnimClass__AI` | Yes |
| `AnimClass::CurrentFrame` | `this+0x0AC` | Current relative animation frame. | `AnimClass__Constructor`, `AnimClass__AI` | Yes |
| `AnimClass::LastFrameTime` | `this+0x0B4` | Written from `g_CurrentFrameCounter` when frame timer starts/reloads. | `AnimClass__Constructor`, `AnimClass__AI` | Yes |
| `AnimClass::FrameDelay` | `this+0x0BC` | Current CDTimer duration for frame advancement. | `AnimClass__Constructor`, `AnimClass__AI` | Yes |
| `AnimClass::FrameDelayReload` | `this+0x0C0` | Delay copied back into `FrameDelay` after frame advance. | `AnimClass__Constructor`, `AnimClass__AI` | Yes |

## 3. Core Logic

### 3.1 Main tick ordering

`Main_Tick @ 0x0055D360` establishes the local throttle budget before most game work. In normal local paths (`g_GameMode == 0` or `5`), it sets:

```text
DAT_00887348 = GetRadarTimer()
DAT_00887350 = DAT_00A8EB60
```

Mode `0` has a conditional override when `DAT_00A8EDDC == 0`: it writes both `DAT_00A8EB60` and `DAT_00887350` to `2`. Standard local skirmish is `g_GameMode == 5`, so that mode-0-only override is not the normal skirmish path.

The normal active tick then runs, in order:

1. `GScreenClass__Input`
2. `LogicClass__AI`
3. optional `House_AI_Tick`
4. network keepalive every eight frames in network mode
5. `Map__Logic`
6. `RenderFrame_main`
7. side work including `FUN_00551A30`
8. `LogicClass__PerTickUpdate`
9. tactical/UI/service routines
10. `Network_ServiceLoop`
11. only if stop/pause flags are clear: `g_CurrentFrameCounter++`
12. `FUN_0055E160` wait/throttle helper

Tiny detail: `CDTimerClass` users during the game-work portion see the old frame counter. A timer started during frame `N` stores `N`; code later in the same `Main_Tick` still sees `N` until the late increment executes. This matters for one-frame boundaries.

The late increment is gated by four globals:

```text
if DAT_00A83D49 == 0
and DAT_00A8ECD0 == 0
and DAT_008B41C0 == 0
and DAT_00A83D48 == 0:
    g_CurrentFrameCounter += 1
```

### 3.2 Local throttle helper

`GetRadarTimer @ 0x006C8C40` is exactly:

```text
return timeGetTime() >> 4
```

So each tick of that timer is a 16 ms bucket. `FUN_0055E160 @ 0x0055E160` subtracts elapsed bucket count from `DAT_00887350`:

```text
remaining = DAT_00887350
if DAT_00887348 != -1:
    elapsed = GetRadarTimer() - DAT_00887348
    if elapsed < remaining:
        remaining -= elapsed
    else:
        remaining = 0
```

For local/menu-style modes (`g_GameMode == 0` or `5`), the helper sleeps against this bucket budget. If the work already consumed enough 16 ms buckets, it does not add the full budget again. If `DAT_00887348 == -1`, the helper treats `DAT_00887350` as a raw `Sleep()` argument, but the initialized local path sets `DAT_00887348` first.

For non-local modes, the helper uses `DAT_00887328` and `DAT_00887330` as a `timeGetTime()` millisecond budget. Network mode `4` can add `10` ms to the network budget up to three times based on remote frame-budget thresholds: one-quarter, one-half, and three-quarters of `g_NetworkFrameBudget`.

### 3.3 Default skirmish speed source

`OptionsClass__SetDefaults @ 0x005FA350` initializes `Options.GameSpeed` at field `+0x00` to `3`. `OptionsClass__ReadFromINI @ 0x005FA620` reads `[Options] GameSpeed=` directly into that same field without proving the local skirmish default.

Normal YR local skirmish is sourced through the multiplayer/skirmish settings:

```text
RulesClass+0x14A0 =
    ReadInt("MultiplayerDialogSettings", "GameSpeed", old RulesClass+0x14A0)

SkirmishSettings+0x08 =
    ReadInt(skirmish_section, "GameSpeed", RulesClass+0x14A0)
```

Retail INI values in this repo:

```ini
; ini/rulesmd.ini
[MultiplayerDialogSettings]
GameSpeed=1

; ini/rules.ini
[MultiplayerDialogSettings]
GameSpeed=0
```

YR `rulesmd.ini` patches the base RA2 value, so the default local skirmish value is stored speed byte `1` when no `[Skirmish] GameSpeed=` overrides it.

`FUN_005B67F0 @ 0x005B67F0` applies session/game-option packets and copies packet byte `+0xA2` to both `DAT_00A8B268` and `DAT_00A8EB60`. This is the live propagation path for the skirmish speed setting.

`OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0` maps UI slider position inversely:

```text
stored_speed = 6 - SendMessageA(game_speed_slider, TBM_GETPOS, 0, 0)
DAT_00A8EB60 = stored_speed
```

Tiny detail: in active non-local games, if the slider changes and command queue has fewer than `0x80` entries, the function enqueues a command with the new speed. It still writes `DAT_00A8EB60` at the end from the local `iVar6`.

### 3.4 CDTimerClass

`CDTimerClass__Start @ 0x0046B640`:

```text
timer.start_frame = g_CurrentFrameCounter
timer.duration = duration
```

`CDTimerClass__GetTimeRemaining @ 0x00426630`:

```text
duration = timer.duration
if timer.start_frame != -1:
    elapsed = g_CurrentFrameCounter - timer.start_frame
    if elapsed < duration:
        return duration - elapsed
    return 0
return duration
```

Tiny details:

- `start_frame == -1` returns raw duration, so paused/not-started timers do not count down.
- `duration == 0` is immediately expired.
- The boundary is `elapsed < duration`; at `elapsed == duration`, remaining is zero.
- The timer never decrements its own field. Remaining time is derived on read.

`CDTimerClass__Remaining @ 0x004C9480`, used inside the RateTimer/FacingClass layout, adds a `rate > 0` guard before evaluating the same frame-counter countdown.

### 3.5 RateTimer / FacingClass

`RateTimer__Set @ 0x004C9220` and `RateTimer__Current @ 0x004C93D0` implement frame-count interpolation for facing-like 16-bit values.

Core behavior:

```text
if rate < 1:
    Current() returns desired target directly

duration = abs(desired - saved) / rate
start_frame = g_CurrentFrameCounter
```

When a new target arrives mid-turn, `RateTimer__Set` first computes the current interpolated value from the existing timer and uses that as the new saved/start value. This means retargeting begins from the visible current angle, not from the old target.

Tiny details:

- Only the low 16 bits are interpolated in the decompiled math; the high 16 bits are carried from the target dword.
- Duration uses integer division. If `abs(delta) / rate < 1`, the timer can become zero-duration.
- At `elapsed == duration`, `RateTimer__Current` returns the final target.
- The same late-frame ordering applies because `start_frame` is `g_CurrentFrameCounter`.

### 3.6 AnimType and AnimClass timing

`AnimTypeClass__ReadINI @ 0x00427D00` parses `Rate=` as an integer and stores an internal frame delay:

```text
if INI Rate is absent:
    keep previous/default internal rate
else if INI Rate < 1:
    internal_rate = 0
else:
    internal_rate = 900 / INI Rate
AnimType::Rate = internal_rate
```

The same `900 / value` conversion is applied to `RandomRate=min,max`, except an endpoint of `-1` means "not specified" and is not converted. After conversion, negative max is clamped to zero, and if max is less than min, min is reduced to max.

`900` is the art timing convention: `60 seconds * 15 game frames/sec`. It is not the local skirmish throttle value and not proof that the whole game runs at a fixed 15 FPS wall-clock rate.

`AnimClass__Constructor @ 0x00421EA0` initializes:

- `CurrentFrame = 0`
- `FrameAdvanced = false`
- `LastFrameTime = g_CurrentFrameCounter`
- `FrameDelay = 0` initially, then set from the anim type rate or random rate
- `FrameDelayReload = FrameDelay`
- `FrameStep = +1`, negated for reverse paths

If `AnimType::Normalized` is true, the constructor applies `FUN_005FB2E0` after `900 / Rate` or random-rate selection. `AnimClass__AI @ 0x00423AC0` repeats that setup when chaining to `Next=`.

`AnimClass__AI` advances frames only when `CDTimerClass__GetTimeRemaining` returns zero and `FrameDelayReload != 0`. On frame advance:

```text
FrameAdvanced = true
CurrentFrame += FrameStep
LastFrameTime = g_CurrentFrameCounter
FrameDelay = FrameDelayReload
```

Tiny details:

- There is a one-AI-call first-frame hold through `this+0x19C` (`param_1[0x67]`) after construction.
- Trailer anims can spawn before normal frame advancement, gated by `g_CurrentFrameCounter % TrailerSeperation == 0` or separation `== 1`.
- `LoopCountRemaining` is stored as a byte at `this+0x195`; constructor computes `type->LoopCount * caller_loop_count`, clamps values below `1` up to `1`, and decrements unless the byte is `0` or `0xFF`.
- `LoopEnd == -1` is lazily filled from image frame count; when `Shadow` is true, `End` can be halved.
- `Rate=0` makes `FrameDelayReload == 0`; AI returns without normal frame advancement.

### 3.7 LogicClass per-tick update

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` (formerly documented as `LogicClass__PerTickUpdate`; live Ghidra label, re-confirmed 2026-05-29 via `get_function_by_address 0x0055AFB0`) runs before the late frame-counter increment in `Main_Tick`. It increments `DAT_00ABCD40`, then processes several frame-driven systems using the current pre-increment `g_CurrentFrameCounter`.

Relevant timing examples:

- Scenario cell-action timers use `g_CurrentFrameCounter - start < duration` tests.
- Bridge shroud recalculation runs when `g_CurrentFrameCounter % 0x78 == 0`.
- Ore growth/spread drivers run from this per-tick path, with their own timer fields.
- Team, laser, lightning, EMP, tactical, factory, and house updates are dispatched from this path.

This confirms that "per tick" in many class AI/update methods means "once per `Main_Tick` before the global frame counter increments," not "once per render-frame delta" and not necessarily "once per a Rust fixed step."

## 4. INI Keys And Units

| Key | File / Section | YR default observed | Binary unit / effect | Evidence |
|---|---|---:|---|---|
| `[MultiplayerDialogSettings] GameSpeed` | `rulesmd.ini` | `1` | Stored speed byte copied to live throttle for skirmish/session. Lower is faster. | `RulesClass__ReadMultiplayerDialogSettings`, `SessionClass__ReadSkirmishSettings`, `FUN_005B67F0` |
| `[MultiplayerDialogSettings] GameSpeed` | `rules.ini` | `0` | Base RA2 fallback, patched by YR `rulesmd.ini`. | INI files |
| `[Options] GameSpeed` | RA2MD.INI/options | local install had `3` in prior report | User options value; read into `Options+0`, not the normal skirmish fallback when `[Skirmish] GameSpeed` is absent. | `OptionsClass__ReadFromINI`; prior reports |
| `[AnimType] Rate` | `art.ini` / `artmd.ini` | many values | Internal frame delay `900 / Rate`, integer division; `<1` becomes zero. | `AnimTypeClass__ReadINI` |
| `[AnimType] Normalized` | `art.ini` / `artmd.ini` | many `yes` entries | Applies normalized delay helper after internal frame delay selection. | `AnimTypeClass__ReadINI`, `AnimClass__Constructor`, `AnimClass__AI` |
| `[AnimType] RandomRate` | `art.ini` / `artmd.ini` | many values | Each endpoint converted with `900 / endpoint`; `-1` means unspecified. | `AnimTypeClass__ReadINI` |
| `[AnimType] RandomLoopDelay` | `art.ini` / `artmd.ini` | many values | Stored as frame delays used after loop wrap. | `AnimTypeClass__ReadINI`, `AnimClass__AI` |
| `[General] ReloadRate` | `rulesmd.ini` | `.3` minutes | Air ammo reload rate; docs and Rust convert with 15 frames/sec assumptions. | INI, prior timing report |
| `[General] BuildSpeed` | `rulesmd.ini` | `.7` minutes per 1000 credits | Production timing input. | INI, `FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md` |
| `[General] GrowthRate` | `rulesmd.ini` | `5` minutes | Ore growth timing input. | INI, `TIBERIUM_QUEUE_SEEDING_AND_TIMING_REPORT.md` |
| `[General] GameSpeedBias` | `rulesmd.ini` | `1.6` | Movement-speed bias, not the same as stored skirmish `GameSpeed`. | INI |
| `[General] ChronoDelay` / `ChronoDistanceFactor` / `ChronoMinimumDelay` | `rulesmd.ini` | `60`, `48`, `16` | Chrono delay values in frames/ticks/leptons-derived frames. | INI, chrono docs |
| `[General] SpyPowerBlackout` | `rulesmd.ini` | `1000` | Frame time. Comment notes `900 = 1 minute`, matching 15 frames/sec convention. | INI |
| `[General] C4Delay` | `rulesmd.ini` | `.03` minutes | Minute-based delay. | INI |
| `[General] IvanTimedDelay` | `rulesmd.ini` | `450` | Frame delay. | INI |
| `[General] RadApplicationDelay` | `rulesmd.ini` | `16` | Frame delay between radiation applications. | INI |
| `[General] RadLevelDelay` | `rulesmd.ini` | `90` | Frame delay between radiation level decrements. | INI |
| `[General] RadLightDelay` | `rulesmd.ini` | `90` | Frame delay between radiation light decrements. | INI |

## 5. Integration Points

### Calls into timing

| Caller / System | Timing Source | Notes |
|---|---|---|
| `Main_Tick` | `GetRadarTimer`, `timeGetTime`, `g_CurrentFrameCounter` | Establishes throttle budget, runs logic/render, increments frame counter late. |
| `FUN_0055E160` | `GetRadarTimer` buckets for local, `timeGetTime` ms for network | Sleeps after work and frame-counter increment on the normal path. |
| `CDTimerClass` users | `g_CurrentFrameCounter` | Timers are computed on read, not decremented. |
| `RateTimer` users | `g_CurrentFrameCounter` | Used by facing/interpolation-like values; retargets from visible current value. |
| `AnimClass` | `CDTimerClass`, `g_CurrentFrameCounter`, `AnimType::Rate` | Frame-count animation, not render-delta animation. |
| `LogicClass__PerTickUpdate` | current pre-increment frame counter | Runs ore/spread, bombs, teams, lasers, factories, houses, etc. |
| Local skirmish/session setup | rules/skirmish `GameSpeed` | Copies stored speed byte to live throttle. |
| Options dialog | UI slider -> `6 - position` | Live speed byte is inverted relative to visible slider. |

### Ordering hazard

GameMD's `g_CurrentFrameCounter` increments after most logic and render work. Any Rust model that computes a synthetic frame at the beginning of `advance_tick` can shift timer starts, expirations, `RateTimer` interpolation, and same-tick checks by one frame unless it deliberately models "old frame visible during update, increment late."

## 6. Current Rust Implementation Status

This section records current implementation surfaces only. It is not an implementation plan.

| Rust surface | Current state | Parity risk |
|---|---|---|
| `src/util/fixed_math.rs:51` | `SIM_TICK_HZ = 45`, while nearby comments still describe 15 Hz / 66 ms semantics. | One Rust tick is not one GameMD frame. Any system using `sim.tick` or per-step countdowns as frame counts can run at the wrong cadence unless mapped. |
| `src/app_types.rs:25-45` | `SIM_TICK_MS = 1000 / SIM_TICK_HZ`; default skirmish speed byte `1` is converted by `tps_for_game_speed(1)` to `63` tps using `16 ms` buckets. Tests assert `default_yr_skirmish_tps() == 63` (test assertion at `src/app_types.rs:168`). _(Re-anchored 2026-05-29: values unchanged; verified via Grep `default_yr_skirmish_tps`/`63` in `src/app_types.rs` — assertion now lives at line 168.)_ | Binary proves speed byte `1` is a throttle budget, not a hard FPS. `63 tps` is an approximation and still needs live retail measurement under workload/sleep granularity. |
| `src/app_sim_tick.rs:151-236` | App runtime scales elapsed wall time by `sim_speed_tps / SIM_TICK_HZ`, schedules fixed steps, and passes `SIM_TICK_MS` into sim and animation ticks. | Speed changes run more/fewer fixed steps per wall-clock second; GameMD instead keeps frame-count semantics and throttles main frames with bucket/millisecond wait paths. |
| `src/sim/world/mod.rs:1398` | `binary_frame` is derived from `total_sim_ms * 15 / 1000` (field declared at `mod.rs:276-287`). _(Re-anchored 2026-05-29: formula unchanged but moved; verified via Grep `binary_frame` in `src/sim/world/mod.rs` — line 229 now holds an unrelated `burst_index`/`SimFireEvent` struct field.)_ | This is useful for frame-based parity, but only systems wired to `binary_frame` get GameMD-like frame units. |
| `src/sim/world/mod.rs:1392-1398` | `total_sim_ms` and `binary_frame` are committed **late**, after all phase work, near the end of `advance_tick` — the code comment explicitly mirrors `Main_Tick`'s guarded late `g_CurrentFrameCounter` increment. _(Re-anchored 2026-05-29: the prior `mod.rs:1013-1016` anchor is now wall-neighbor cleanup; the derivation moved to `mod.rs:1392-1398` with an explicit "committed late" comment. The earlier "one-frame-early" parity-risk note is obsolete and removed — the Rust port now deliberately holds the previous frame's value during the tick. Verified via Read of `src/sim/world/mod.rs:1388-1399`.)_ | GameMD increments `g_CurrentFrameCounter` late, after logic/render/service work; the Rust port now matches this by committing the synthetic 15 Hz frame at the end of `advance_tick`. |
| `src/rules/art_data.rs:181-186` | Converts art `Rate=` to milliseconds: `(900 / Rate) * 1000 / 15`. | Matches the unnormalized 15-frame convention as wall-clock delay, but collapses frame-counter timers into ms and does not by itself model `Normalized=yes`, random rate, loop delay, or late-counter ordering. |
| `src/sim/combat/mod.rs:2193-2203` | Converts ROF frames to cooldown ticks via `frames * 1000 / GAME_FPS`, then divides by current `tick_ms`. | Frame-based weapon cadence is converted to fixed-step ticks; exact parity depends on `GAME_FPS`, current tick size, and same-tick fire/cooldown ordering. |
| `src/sim/animation.rs` | Generic sequences use `tick_ms` / `elapsed_ms` loops. | GameMD `AnimClass` is frame-counter based with CDTimer semantics and normalized-rate adjustment. |
| `src/app_sim_tick.rs:176-204` | Some building/muzzle/parachute presentation effects tick from capped `sim_elapsed` wall-clock-like ms outside the deterministic sim step. | Several visible GameMD effects are frame-counter or class-AI driven, so presentation-layer ms loops need per-effect audit. |

## 7. Findings Matrix

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Default local YR skirmish speed source is `rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1`, through `RulesClass+0x14A0` and skirmish settings fallback. | `RulesClass__ReadMultiplayerDialogSettings`, `SessionClass__ReadSkirmishSettings`, INI | High | Yes |
| `[Options] GameSpeed=3` is an options default/user setting and is not the normal fallback for local skirmish when `[Skirmish] GameSpeed` is absent. | `OptionsClass__SetDefaults`, `OptionsClass__ReadFromINI`, skirmish settings function | High | Yes |
| Stored speed byte is inverted relative to the UI slider (`6 - slider_position`). | `OptionsClass__ApplyFromInGameDialog` | High | Yes |
| Local throttle uses `GetRadarTimer() == timeGetTime() >> 4`; budget units are 16 ms buckets. | `GetRadarTimer`, `Main_Tick`, `FUN_0055E160` | High | Yes |
| Speed byte is not an FPS number. Work time is subtracted from bucket budget before sleep. | `Main_Tick`, `FUN_0055E160` | High | Yes |
| Network modes use a separate `timeGetTime()` millisecond budget and can adjust budget in 10 ms increments from network lag thresholds. | `Main_Tick`, `FUN_0055E160` | High | Conditional |
| `g_CurrentFrameCounter` increments late, after input, logic, map logic, render, per-tick side work, and network service. | `Main_Tick` | High | Yes |
| `CDTimerClass` timers are computed from `g_CurrentFrameCounter`; they do not self-decrement. | `CDTimerClass__Start`, `CDTimerClass__GetTimeRemaining` | High | Yes |
| `RateTimer` retargets from the current interpolated visible value and uses integer frame math. | `RateTimer__Set`, `RateTimer__Current` | High | Yes |
| `AnimType Rate=` is `900 / Rate` integer frame delay; `900` is the art convention `60*15`, not the local throttle rate. | `AnimTypeClass__ReadINI` | High | Yes |
| `AnimClass` advancement is frame-counter/CDTimer based and writes `LastFrameTime = g_CurrentFrameCounter` on frame advance. | `AnimClass__Constructor`, `AnimClass__AI` | High | Yes |
| `LogicClass__PerTickUpdate` executes before the late frame increment and dispatches many visible/gameplay systems. | `Main_Tick`, `LogicClass__PerTickUpdate` | High | Yes |
| The repo's `45 Hz` sim tick is an internal scheduling choice unless every GameMD frame-based system is explicitly mapped to `binary_frame` or equivalent. | Rust scan plus binary frame-counter evidence | High | Yes |

## 8. Open Questions

1. What is the measured retail wall-clock `g_CurrentFrameCounter` delta/sec in a default local YR skirmish with stored speed byte `1`, across 30+ seconds and normal render workload?
2. What are the measured deltas/sec for every visible speed slider position after `OptionsClass__ApplyFromInGameDialog` maps the UI value to `6 - slider_position`?
3. Which branch, if any, originally motivated the repo's `45 Hz` assumption? The checked local skirmish path does not prove a hardcoded 45 FPS default.
4. What exact delay table/behavior does `FUN_005FB2E0` implement for normalized animation rates for all speed bytes and small internal delays? Existing docs report this, but this global pass only spot-checked the call sites.
5. Which Rust presentation effects are intended to be render-time-only abstractions, and which should be mapped to GameMD frame counters because they correspond to `AnimClass`, `RateTimer`, or class-AI visible output?

## Sources

- Ghidra decompile: `Main_Tick @ 0x0055D360`
- Ghidra decompile: `FUN_0055E160 @ 0x0055E160`
- Ghidra decompile: `GetRadarTimer @ 0x006C8C40`
- Ghidra decompile: `CDTimerClass__GetTimeRemaining @ 0x00426630`
- Ghidra decompile: `CDTimerClass__Start @ 0x0046B640`
- Ghidra decompile: `CDTimerClass__Remaining @ 0x004C9480`
- Ghidra decompile: `RateTimer__Set @ 0x004C9220`
- Ghidra decompile: `RateTimer__Current @ 0x004C93D0`
- Ghidra decompile: `AnimTypeClass__ReadINI @ 0x00427D00`
- Ghidra decompile: `AnimClass__Constructor @ 0x00421EA0`
- Ghidra decompile: `AnimClass__AI @ 0x00423AC0`
- Ghidra decompile: `OptionsClass__SetDefaults @ 0x005FA350`
- Ghidra decompile: `OptionsClass__ReadFromINI @ 0x005FA620`
- Ghidra decompile: `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0`
- Ghidra decompile: `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`
- Ghidra decompile: `SessionClass__ReadSkirmishSettings @ 0x00697F10`
- Ghidra decompile: `FUN_005B67F0 @ 0x005B67F0`
- Ghidra decompile: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` (formerly `LogicClass__PerTickUpdate`)
- `docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
- `docs/research/DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md`
- `docs/research/VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md`
- `docs/research/TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
- Rust files read: `src/util/fixed_math.rs`, `src/app_types.rs`, `src/app_sim_tick.rs`, `src/sim/world/mod.rs`, `src/rules/art_data.rs`, `src/sim/combat/mod.rs`, `src/sim/animation.rs`
