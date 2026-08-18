# Default YR Skirmish Frame Pace - Ghidra Extension Report

Date: 2026-05-17

**Address(es):** `0x0055D360` (`Main_Tick`), `0x0055E160` (`FUN_0055e160` wait helper), `0x006C8C40` (`GetRadarTimer`), plus speed-source helpers listed in Sources.
**Confidence:** High for static binary throttle/source/order findings; Medium for observed wall-clock FPS because no live `gamemd.exe` process was attached during this pass.
**Active in YR:** Yes for standard local skirmish mode `g_GameMode == 5`; network/multiplayer mode branches are conditional and not the default local skirmish path.

## 1. Overview

This is a targeted extension of `VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md`, focused on the recommended next question: what should VERA20k treat as the default YR skirmish timing source? The static binary answer is:

- default local YR skirmish speed resolves to stored speed byte `1`;
- that byte is used as a wait budget in `GetRadarTimer()` units;
- `GetRadarTimer()` is `timeGetTime() >> 4`, so one budget unit is a 16 ms bucket;
- `g_CurrentFrameCounter` increments once near the end of `Main_Tick`, after the game work and after `Network_ServiceLoop`, when pause/stop gates allow it;
- the static binary does **not** prove a hardcoded default `30 fps`, `45 fps`, or `60 fps` skirmish frame rate.

The missing part is a live measurement: the actual wall-clock `g_CurrentFrameCounter` increments/sec in retail under default local skirmish, including render workload and Windows `Sleep()` granularity. No `gamemd.exe` process was running, so this report cannot replace that runtime probe.

## 2. Key Globals / Fields

| Symbol / offset | Type | Meaning | Evidence |
|---|---:|---|---|
| `g_CurrentFrameCounter @ 0x00A8ED84` | `int` | Global game frame counter incremented late in `Main_Tick`. | `Main_Tick @ 0x0055D360` |
| `DAT_00A8EB60` | byte/int global | Live game speed / throttle code. For default YR skirmish, this is the stored speed byte `1`. | `Main_Tick`, `FUN_005b67f0`, `OptionsClass__ApplyFromInGameDialog` |
| `DAT_00887348` | int global | Start `GetRadarTimer()` bucket for local/game-mode wait helper. `-1` disables elapsed subtraction. | `Main_Tick`, `FUN_0055e160` |
| `DAT_00887350` | int global | Wait budget in `GetRadarTimer()` buckets for local/game-mode wait helper. | `Main_Tick`, `FUN_0055e160` |
| `DAT_00887328` | int global | Start `timeGetTime()` value for network wait path. | `Main_Tick`, `FUN_0055e160` |
| `DAT_00887330` | int global | Millisecond wait budget for network path. | `Main_Tick`, `FUN_0055e160` |
| `RulesClass+0x14A0` | int | `[MultiplayerDialogSettings] GameSpeed` rules default. | `RulesClass__ReadMultiplayerDialogSettings` |
| skirmish settings `+0x08` / `param_1[2]` | int | `[Skirmish] GameSpeed`, falling back to `RulesClass+0x14A0`. | `SessionClass__ReadSkirmishSettings` |

## 3. Core Logic

### 3.1 Default speed source

`RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` reads the rules default:

```text
RulesClass+0x14A0 =
    CCINIClass__ReadInt("MultiplayerDialogSettings", "GameSpeed", old RulesClass+0x14A0)
```

Retail INI values checked in this repo:

```ini
; ini/rulesmd.ini
[MultiplayerDialogSettings]
GameSpeed=1

; ini/rules.ini
[MultiplayerDialogSettings]
GameSpeed=0
```

YR `rulesmd.ini` patches the base RA2 value, so the default multiplayer/skirmish dialog source is `1`.

`SessionClass__ReadSkirmishSettings @ 0x00697F10` then reads the skirmish value:

```text
param_1[2] =
    CCINIClass__ReadInt(section, "GameSpeed", *(g_RulesClass_Instance + 0x14A0))
```

The local retail config at `<ra2-install>/RA2MD.INI` currently contains:

```ini
[Options]
GameSpeed=3
```

No `[Skirmish] GameSpeed=` entry was found in that file. Therefore normal local skirmish falls back to `RulesClass+0x14A0 == 1`, not the `[Options]` value `3`.

### 3.2 Live speed propagation

The lobby/session packet bridge `FUN_005b67f0 @ 0x005B67F0` copies packet byte `+0xA2`:

```text
DAT_00A8B268 = *(byte *)(packet + 0xA2)
DAT_00A8EB60 = DAT_00A8B268
```

This keeps the live throttle byte equal to the stored skirmish speed. `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0` maps the in-game slider inversely:

```text
slider_position = SendMessageA(game_speed_slider, TBM_GETPOS, 0, 0)
stored_speed = 6 - slider_position
DAT_00A8EB60 = stored_speed
```

The stored byte is a delay/speed code where lower is faster. It is not an FPS number.

### 3.3 Main tick local skirmish throttle

`GetRadarTimer @ 0x006C8C40` is exactly:

```text
return timeGetTime() >> 4
```

So `GetRadarTimer()` advances once per 16 ms bucket, subject to the normal behavior of `timeGetTime()`.

In `Main_Tick @ 0x0055D360`, the standard local game path for `g_GameMode == 5` reaches the same local throttle setup as mode `0` without the mode-0 speed override:

```text
uVar19 = DAT_00A8EB60

if g_GameMode == 0 and DAT_00A8EDDC == 0:
    DAT_00A8EB60 = 2
    DAT_00887348 = GetRadarTimer()
    DAT_00887350 = 2
else:
    DAT_00887348 = GetRadarTimer()
    DAT_00887350 = uVar19
```

For default local skirmish, `g_GameMode == 5`, so the speed-2 branch is skipped and `DAT_00887350 = DAT_00A8EB60 = 1`.

### 3.4 Wait helper details

`FUN_0055e160 @ 0x0055E160` computes remaining wait from `DAT_00887350`:

```text
remaining = DAT_00887350
if DAT_00887348 != -1:
    elapsed = GetRadarTimer() - DAT_00887348
    if elapsed < remaining:
        remaining -= elapsed
    else:
        remaining = 0
```

For game modes `0` and `5`, it then loops/sleeps while remaining local radar buckets exist:

```text
if mode is 0 or 5:
    if DAT_00887348 == -1:
        Sleep(DAT_00887350)
    else:
        Sleep(DAT_00887350 - (GetRadarTimer() - DAT_00887348))
```

Tiny but important details:

- `DAT_00887350` is interpreted in `GetRadarTimer()` buckets in local/skirmish mode, not milliseconds.
- `DAT_00887348 == -1` changes the meaning: the helper sleeps the raw budget value directly.
- With normal local setup, `DAT_00887348` is set from `GetRadarTimer()` before game work, so elapsed work time is subtracted from the speed budget.
- If the game work already consumed at least the budget, remaining wait becomes zero.
- Static decompilation gives a nominal speed-1 throttle of one 16 ms bucket, plus/render workload and sleep granularity. It does not prove the realized runtime counter rate without a live measurement.

### 3.5 Frame counter ordering

`Main_Tick` runs input, logic, optional house AI, map logic, render, per-tick side work, UI/tactical services, and `Network_ServiceLoop()` before the frame counter increment. The increment is gated:

```text
if DAT_00A83D49 == 0
and DAT_00A8ECD0 == 0
and DAT_008B41C0 == 0
and DAT_00A83D48 == 0:
    g_CurrentFrameCounter += 1
    if DAT_00B07784 != 0 and DAT_00B07784 < g_CurrentFrameCounter:
        FUN_00684290()
        DAT_00B07784 = 0
    FUN_0055e160()
    FUN_00725c70()
    FUN_00637270()
```

Therefore logic/render inside tick `N` read the old frame value. Timers started during tick `N` use `N` until the late increment completes.

There is also a scenario-delay/render-only branch earlier in `Main_Tick`:

```text
if ScenarioClass+0x62C != 0:
    Process_NetworkMessages()
    Network_ServiceLoop()
    Process_QueuedEvents()
    Tactical draw/update
    RenderFrame_main()
    FUN_0055e160()
    return without incrementing g_CurrentFrameCounter
```

This path can render/process side work while the gameplay frame counter does not advance.

### 3.6 Network/multiplayer branches are different

When the path is not mode `0` or `5`, `Main_Tick` has network-specific budgets:

```text
if DAT_00A8B558 == 0:
    DAT_00887350 = 2
    DAT_00887330 = 0x21
else:
    DAT_00887350 = 0x3C / DAT_00A8B558
    DAT_00887330 = 1000 / DAT_00A8B558
```

Mode `4` also adjusts `DAT_00887330` upward by 10 ms chunks based on remote frame-budget thresholds. These branches are real, but they are not the default local skirmish `g_GameMode == 5` path.

This is the likely source of confusion around "FPS settings": the binary has a requested/network FPS global (`DAT_00A8B558`, default 30 in other reports) and formulas involving `0x3C`, but the checked local skirmish path uses stored `GameSpeed` and `GetRadarTimer()` buckets.

## 4. Timing Consumers Checked

The following consumers were re-checked or spot-checked because they affect visible pace:

| Consumer | Binary timing source | Evidence | VERA20k implication |
|---|---|---|---|
| `AnimClass__AI` | `CDTimerClass__GetTimeRemaining()` from `g_CurrentFrameCounter`; reloads frame delay and writes last-frame/start frame from current global frame. | `AnimClass__AI @ 0x00423AC0` | Millisecond-only `rate_ms` loops are not enough for full parity. |
| `ParticleSystemClass__AI_Smoke` / Fire / Gas | Spawn and parity gates use `g_CurrentFrameCounter` in the docs and particle AI functions. | Existing particle reports plus function search/decompile names at `0x0062ED40`, `0x0062F9A0`, `0x0062E6D0` | Rust `sim.tick` particle gates are only correct if mapped to the same frame cadence. |
| `ParticleSystemClass__AI_Spark` / Railgun | Own particle system logic, not `AnimClass Rate=`; active standard YR systems. | `ParticleSystemClass__AI_Spark @ 0x0062E840`, `ParticleSystemClass__AI_Railgun @ 0x0062F230`, existing particle report | Missing systems are visible absence, not just cadence mismatch. |
| SHP vehicle body animation | `g_CurrentFrameCounter % WalkRate` / `% IdleRate` gates `FootClass+0x538`. | `FootClass__AI @ 0x004DA530` | Hardcoded ms SHP vehicle sequences cannot match `WalkRate`/`IdleRate`. |
| Infantry action animation | `InfantryClass__Do_Action` writes `ActionTimer.start = g_CurrentFrameCounter`, with selected actions normalized through `FUN_005FB2E0`. | `InfantryClass__Do_Action @ 0x0051D6F0` | Hardcoded ms infantry sequence rates are not the binary cadence. |

## 5. INI Keys

| INI key | Section | Default / checked value | Effect |
|---|---|---|---|
| `GameSpeed` | `[MultiplayerDialogSettings]` in `rulesmd.ini` | `1` | Default skirmish dialog speed source in YR. |
| `GameSpeed` | `[MultiplayerDialogSettings]` in `rules.ini` | `0` | Base RA2 fallback, patched by YR `rulesmd.ini`. |
| `GameSpeed` | `[Skirmish]` in `RA2MD.INI` | absent locally | If present, overrides rules default for skirmish settings. |
| `GameSpeed` | `[Options]` in local `RA2MD.INI` | `3` | Options default/read value; not the fallback used by `SessionClass__ReadSkirmishSettings` for absent `[Skirmish] GameSpeed`. |
| `Rate` / `RandomRate` / `Normalized` | art `AnimType` sections | varies | Drives `AnimClass` frame-delay semantics, not local skirmish throttle directly. |
| `WalkRate` / `IdleRate` | techno/unit type rules | defaults documented elsewhere as `1` / `0` | Raw modulo gates for SHP vehicle body frame counter. |
| `SpawnFrames`, `SparkSpawnFrames`, `SpawnSparkPercentage` | particle system types | varies | Particle-specific frame gates; not `AnimClass Rate=`. |

## 6. Current Rust Implementation Status

This pass did not modify Rust. Current status relevant to timing:

- `src/app_types.rs:30-45` now uses `DEFAULT_YR_SKIRMISH_GAME_SPEED = 1` and maps speed to approximate TPS via 16 ms buckets.
- `src/util/fixed_math.rs:51` still sets `SIM_TICK_HZ = 45`.
- `src/app_sim_tick.rs:226-235` scales elapsed time by `sim_speed_tps / SIM_TICK_HZ`, so speed changes alter how many 22 ms sim steps run per wall-clock second.
- `src/sim/world/mod.rs:1014-1015` derives `binary_frame` at the start of `advance_tick`, whereas GameMD increments `g_CurrentFrameCounter` late.
- `src/sim/world/mod.rs:1407` ticks particle systems once per Rust sim tick.
- `src/sim/particles/system_ai.rs:56` uses `pt.state_ai_advance` rather than the per-particle rewritten `p.state_ai_advance`.
- `src/sim/world/world_commands.rs:72-76` has a development 3x speed multiplier for deployable units.
- `src/sim/movement/parachute_descent.rs:99-132` integrates descent once per sim tick and only uses `tick_ms` as a pause guard.

## 7. Open Questions

1. What is the measured retail `g_CurrentFrameCounter` delta per wall-clock second in a default local YR skirmish with stored speed `1`?
2. Does the live loop settle near 62.5 frames/sec when workload is light, or lower because of render/work and `Sleep()` granularity?
3. Which exact binary branch, if any, originally motivated the repo's "45 FPS standard multiplayer" statement? The verified local skirmish path does not prove it.
4. What are retail cells/sec for controlled `Speed=4`, `Speed=8`, and MCV movement at stored speed `1`?
5. For implementation planning, should VERA20k expose `sim_speed_tps` as a debug scheduler knob only, while frame-based visible systems consume a separate GameMD-frame clock?

## 8. Recommended Probe Order

1. **Retail live pace probe:** attach to `gamemd.exe`, start a default local skirmish, and sample `g_CurrentFrameCounter @ 0x00A8ED84` plus `DAT_00A8EB60` every wall-clock second for at least 30 seconds.
2. **Retail slider probe:** repeat for each visible speed slider position, logging stored `DAT_00A8EB60`, counter/sec, and whether the local path still uses `GetRadarTimer()` buckets.
3. **VERA20k clock probe:** log `sim.tick`, `binary_frame`, and wall-clock seconds at default speed so the Rust side can be compared directly to retail.
4. **Movement probe:** measure cells/sec for known `Speed=` units and for MCV/deployables.
5. **Particle/Anim probe:** measure FireStream state changes, smoke/gas spawn counts, and one `Rate=400` `AnimClass` effect.

## Sources

- Ghidra decompile: `Main_Tick @ 0x0055D360`
- Ghidra decompile: `FUN_0055e160 @ 0x0055E160`
- Ghidra decompile: `GetRadarTimer @ 0x006C8C40`
- Ghidra decompile: `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`
- Ghidra decompile: `SessionClass__ReadSkirmishSettings @ 0x00697F10`
- Ghidra decompile: `FUN_005b67f0 @ 0x005B67F0`
- Ghidra decompile: `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0`
- Ghidra decompile: `FUN_0069bab0 @ 0x0069BAB0`
- Ghidra decompile: `FUN_0069bb40 @ 0x0069BB40`
- Ghidra spot-check: `AnimClass__AI @ 0x00423AC0`
- Ghidra spot-check: `FootClass__AI @ 0x004DA530`
- Ghidra spot-check: `InfantryClass__Do_Action @ 0x0051D6F0`
- Function lookup: `ParticleSystemClass__AI_Smoke @ 0x0062ED40`
- Function lookup: `ParticleSystemClass__AI_Fire @ 0x0062F9A0`
- Function lookup: `ParticleSystemClass__AI_Gas @ 0x0062E6D0`
- Function lookup: `ParticleSystemClass__AI_Spark @ 0x0062E840`
- Function lookup: `ParticleSystemClass__AI_Railgun @ 0x0062F230`
- Existing report: `docs/research/VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md`
- Existing report: `docs/research/SKIRMISH_SPEED_AND_PARTICLE_NORMALIZED_GHIDRA_REPORT.md`
- Existing report: `docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
- Existing report: `docs/research/TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md`
- Existing report: `docs/research/PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md`
- INI: `ini/rulesmd.ini`
- INI: `ini/rules.ini`
- Local config: `<ra2-install>/RA2MD.INI`
