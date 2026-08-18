# Game-Speed Master Clock

## Overview

**Player-visible effect:** the user picks a speed in Options ("Slowest" … "Fastest").
Every gameplay event — unit movement, weapon ROF, build queue progress, animation,
audio cadence — speeds up or slows down with that setting in lockstep. The "Fastest"
setting runs the game roughly 6× faster than "Slowest" (linear in wait-time, not
exponential).

**Mechanism in plain terms:** `gamemd.exe` keeps a single integer counter,
`g_CurrentFrameCounter`, that increments by exactly **1** at the end of every
`Main_Tick`. Every gameplay timer in the engine compares against this counter
(`g_CurrentFrameCounter - start_frame >= duration_frames`) rather than against
wall-clock time. The GameSpeed slider does **not** change how much a single tick
"counts"; it changes how much **wall-clock ms** the engine waits between ticks.
The result: at "Slowest" the game runs at roughly 15 logic-ticks/sec; at "Fastest"
it runs at roughly 60 logic-ticks/sec; the game-state math is identical at every
speed, and only the rate the state advances differs.

There are **two distinct speed knobs** that flow through this clock:

1. **`OptionsClass::GameSpeed`** (per-machine, slider 0..6, **0=fastest**, **6=slowest**) —
   sets the wait between local logic ticks. Lives at `DAT_00a8eb60`.
2. **`SessionClass::FrameSendRate` / network frame budget** (sync'd across all peers
   via network event 0x20) — sets the lockstep network turn rate. Lives at
   `DAT_00a8b558` and `g_NetworkFrameBudget`. See
   [multiplayer-frame-step.md](multiplayer-frame-step.md).

Single-player uses **only** the slider. Multiplayer uses **both**: the slider sets
the local tick rate, the network frame budget caps it lockstep-wise so all peers
advance in step.

---

## INI surface

### `rulesmd.ini` — `[General]` (game-tick-affecting movement multiplier; distinct from the clock itself but flows through it)

```ini
GameSpeedBias=1.6       ; multiplier to overall game object movement speed
;was 1.2
```

Scope: **Rules-global**. Read by `RulesClass::ReadGeneral` @ `0x0066d530` via
`CCINIClass::ReadDouble` (key string at `0x0083bf24`, only xref is at
`0x00670b26` inside that function). Stored as a `double` in `RulesClass`.

**Not** the master clock — this is a movement-only scalar applied on top of
per-unit `Speed=` (covered in [movement-speed-turn-rate.md](movement-speed-turn-rate.md)).
Cross-referenced here because `GameSpeed` × `GameSpeedBias` together govern how
many cells per wall-clock-second a unit traverses.

### `rulesmd.ini` — `[MultiplayerDialogSettings]`

```ini
; GameSpeed = starting game speed. For some wacky reason, 0=fastest, 6=slowest. (def=0)

[MultiplayerDialogSettings]
...
GameSpeed=1
...
```

Scope: **Rules-global, used as multiplayer dialog default**. Read by
`RulesClass::ReadMultiplayerDialogSettings` @ `0x00671eb0` and stored at
`RulesClass + 0x14a0`. The comment in the file says `def=0` but the actual
default value present is `GameSpeed=1` — the comment is a hint of the encoding,
not the shipped default. **Distinct** from the player's local `OptionsClass`
slider; this is the rules-defined default for the multiplayer setup dialog.

### `rulesmd.ini` — `[AudioVisual]` (FPS-floor for detail downgrade, NOT the clock)

```ini
DetailMinFrameRateNormal=15     ; If frame rate drops below this value, various visual effects switch off.
DetailMinFrameRateMovie=20      ; As above, but applies when a movie is playing.
DetailBufferZoneWidth=5         ; To restore effects, frame rate must equal or exceed MinFrameRate plus this.
```

Scope: **Rules-global**. Quoted here because the `15 FPS` baseline floor is the
engine's design target for "playable" — confirming the GameSpeed table maps
roughly into the 15–60 FPS band. These keys gate **rendering effect downgrade**,
not logic-tick rate (see "Tick / frame topology").

### Not in INI: per-user slider

The player's chosen speed lives in the **user options profile** (the
`OptionsClass` written to `RA2MD.INI` / `RA2.INI`), not in `rulesmd.ini`. The
key name is the same — `GameSpeed` under `[Options]` — but the file is
per-machine, not part of the gameplay rules. Read by `OptionsClass::ReadFromINI`
@ `0x005fa646`:

```c
uVar5 = CCINIClass__ReadInt(s_Options_008254dc, s_GameSpeed_008332e8, *param_1);
*param_1 = uVar5;
```

Default = `3` (medium), set by `OptionsClass::SetDefaults` @ `0x005fa350`
(first field assignment: `*param_1 = 3;`).

---

## Hardcoded constants

### Speed-value range: 0..6 (7 positions)

Decoded from `OptionsClass::ApplyFromInGameDialog` @ `0x004e1de0`:

```c
LVar2 = SendMessageA(pHVar1, 0x400, 0, 0);   // slider position 0..6
iVar5 = 6 - LVar2;                            // → internal speed value 6..0
```

The dialog slider runs 0 (visually-leftmost = "Slowest") to 6 (rightmost =
"Fastest"). The conversion `6 - slider` makes the **internal** `GameSpeed`
value run the opposite way: **0 = Fastest, 6 = Slowest**. This is the value
stored in `DAT_00a8eb60` and the one referenced by every timing system in the
engine.

The label-string table at `0x00822730` confirms the 7 positions (read 28 bytes,
7 LPCSTR entries):

| Internal value | Label string (RVA) | Display label |
|---|---|---|
| 0 | `0x00822800` | `TXT_SLOWEST` |
| 1 | `0x008227f4` | `TXT_SLOWER` |
| 2 | `0x008227e8` | `TXT_SLOW` |
| 3 | `0x008227dc` | `TXT_MEDIUM` |
| 4 | `0x008227d0` | `TXT_FAST` |
| 5 | `0x008227c4` | `TXT_FASTER` |
| 6 | `0x008227b8` | `TXT_FASTEST` |

**WARNING — direction-of-encoding pitfall:** the `rulesmd.ini` comment line
says `0=fastest, 6=slowest`. The string-table index above says the opposite
(table index 0 → SLOWEST). These two views agree once you separate **label
order** (`0..6` walking through `SLOWEST..FASTEST` in the dialog list) from
**stored value** (`6 - slider` = `0=Fastest, 6=Slowest`). The INI comment
documents the **stored value**. The table at `0x00822730` is keyed by **slider
index**, not stored value. When implementing, use the stored-value convention
(`0=Fastest, 6=Slowest`); the slider→stored conversion is the only place the
inversion appears.

### Default GameSpeed values

| Source | Default | Address |
|---|---|---|
| `OptionsClass::SetDefaults` (single-player local slider) | `3` (Medium) | `0x005fa350`, first store `*param_1 = 3` |
| `[MultiplayerDialogSettings] GameSpeed` (rules-global MP default) | `1` (Faster) | `rulesmd.ini` line 3026 |
| `SessionClass::ReadSkirmishSettings` fallback | inherits `RulesClass + 0x14a0` (= `1`) | `0x00697f10` |

### Wait-derivation constants in `Main_Tick`

`Main_Tick` @ `0x0055d360`, MP/replay path:

```c
lVar1 = (longlong)DAT_00a8b558;
DAT_00887350 = (int)(0x3c / lVar1);   // 0x3c = 60
local_1ac = (int)(1000 / lVar2);      // 1000 = ms per second
```

- `0x3c = 60` — likely a "60 frames per network turn" constant for the network
  scheduler; divided by the network frame budget to produce an interval count.
- `1000 / DAT_00a8b558` — ms-per-tick if `DAT_00a8b558` is interpreted as
  ticks-per-second. With `DAT_00a8b558 = 60`, this yields ~17 ms (60 FPS);
  with `DAT_00a8b558 = 15`, this yields ~67 ms (15 FPS).

**Update (resolved in iteration 3):** the actual pacing routine is
`FUN_0055e160` @ `0x0055e160`, called near the end of `Main_Tick` after
`LogicClass::PerTickUpdate`. It compares `GetRadarTimer()` deltas against
`DAT_00887350` — meaning **`DAT_00887350` is in `GetRadarTimer` units
(= `timeGetTime() >> 4` = `~16 ms` each)**. So:

| GameSpeed value | Display label | Per-tick wait |
|---|---|---|
| 0 | Fastest | 0 × 16 ms = 0 ms (uncapped — limited only by AI+render time) |
| 1 | Faster | 1 × 16 ms ≈ 16 ms (~62 ticks/sec ceiling) |
| 2 | Fast | 2 × 16 ms ≈ 32 ms (~31 ticks/sec) |
| 3 | Medium (SP default) | 3 × 16 ms ≈ 48 ms (~20 ticks/sec) |
| 4 | Slow | 4 × 16 ms ≈ 64 ms (~15.6 ticks/sec) |
| 5 | Slower | 5 × 16 ms ≈ 80 ms (~12.5 ticks/sec) |
| 6 | Slowest | 6 × 16 ms ≈ 96 ms (~10.4 ticks/sec) |

These are **ceilings**, not floors — if AI + render take longer than the
wait, the tick stretches and the effective FPS drops below the ceiling. The
`[AudioVisual] DetailMinFrameRateNormal=15` threshold is consistent: slot 4
sits right at 15 ticks/sec, so any further slowdown trips the detail
downgrade. In MP, `FUN_0055e160` spinwaits with `Sleep(0)` and
`Network_ServiceLoop` instead of `Sleep(N)` so it can drain network frames
during the wait.

**Confidence: HIGH** — `FUN_0055e160` decompiled and reviewed in iteration 3,
the SP path's `Sleep(DAT_00887350)` and the `GetRadarTimer` unit derivation
are both directly observed. `FUN_005d5870` / `FUN_005d5880` are
`timeBeginPeriod(1)` / `timeEndPeriod(1)` — high-precision-timer setup, not
the pacing.

### Bridge shroud recalc cadence (consumes the master clock)

`LogicClass::PerTickUpdate` @ `0x0055afb0`:

```c
if ((int)g_CurrentFrameCounter % 0x78 == 0) {
    MapClass__RecalcBridgeShroudFlags();
}
```

`0x78 = 120` logic ticks. At GameSpeed=Fastest (≈60 ticks/sec) this fires
every ~2 seconds; at Slowest (≈15 ticks/sec) every ~8 seconds. Quoted to
demonstrate the rule **every periodic system in the engine ticks on
`g_CurrentFrameCounter`**, not on wall-clock.

### Network keepalive cadence

`Main_Tick` @ `0x0055d360`:

```c
if ((((byte)g_CurrentFrameCounter & 7) == 7) && (g_GameMode == 4)) {
    Network_Keepalive();
}
```

`& 7 == 7` → every 8 logic ticks, MP only. Confirms again: `g_CurrentFrameCounter`
is the unit-of-time, not wall-clock.

### FPS measurement accumulator (read-only telemetry)

`Main_Tick` @ `0x0055d360`:

```c
DVar10 = timeGetTime();
iVar4 = DVar10 - _DAT_00a8b55c;
if (1000 < iVar4) { iVar4 = 1000; }
DAT_00a8b560 = DAT_00a8b560 + iVar4;   // ms accumulator
DAT_00a8b564 = DAT_00a8b564 + 1;        // tick accumulator
```

Running ms-per-tick average. `House_AI_Tick` @ `0x0055f47d` (caller of the
"Req fps : %d" log string at `0x0082a1b4`) prints this. **Pure telemetry — does
not gate the clock.**

---

## Tick / frame topology

`Main_Game` @ `0x0048ccc0` runs:

```c
do {
    cVar1 = Main_Tick();
    if (cVar1 && FUN_0055cfd0()) break;
    State_Machine();
    cVar1 = FUN_0055cfd0();
} while (cVar1 == '\0');
```

One iteration = one `Main_Tick` = one logic frame. Inside `Main_Tick`
@ `0x0055d360`:

```c
// (1) Network/input + speed-derivation:
//   - In MP path: DAT_00887350 = 0x3c / DAT_00a8b558 (= network turn divisor)
//   - In SP path: DAT_00887350 = DAT_00a8eb60 (= local GameSpeed)
//   - timeGetTime() snapshot for FPS measurement

// (2) Input + logic + render dispatch (only when active and unpaused):
if (g_GameState == 0 && g_GameRunning != 0 && (_DAT_00a8d5f8 & 2) == 0) {
    GScreenClass__Input(...);
    LogicClass__AI();          // per-tick AI dispatch
    if (DAT_00a8b8b4) House_AI_Tick();
    if ((g_CurrentFrameCounter & 7) == 7 && g_GameMode == 4)
        Network_Keepalive();
    Map__Logic();               // per-tick map / cell dispatch
    RenderFrame_main();         // render dispatch
}

// (3) Replay record/playback bookkeeping (if recording or playing back)

// (4) Late housekeeping:
LogicClass__PerTickUpdate();    // ore growth, bombs, lasers, factories, houses, …

// (5) FPS accumulator update + Network_ServiceLoop

// (6) Advance the master clock (only if not in a freeze state):
if (DAT_00a83d49 == 0 && DAT_00a8ecd0 == 0 && DAT_008b41c0 == 0 && DAT_00a83d48 == 0) {
    g_CurrentFrameCounter = g_CurrentFrameCounter + 1;
}
```

### Clock binding (which clock each subsystem lives on)

| Subsystem | Clock | Evidence |
|---|---|---|
| Master logic counter | game-tick (defined by `Main_Tick`) | `g_CurrentFrameCounter += 1` at end of `Main_Tick` |
| AI dispatch | game-tick | `LogicClass::AI()` called once per `Main_Tick` |
| Map / cell logic | game-tick | `Map::Logic()` called once per `Main_Tick` |
| Ore growth, bombs, lasers, factories | game-tick | All driven from `LogicClass::PerTickUpdate` |
| Bridge shroud recalc | game-tick / 120 | `g_CurrentFrameCounter % 0x78 == 0` |
| Network keepalive | game-tick / 8 (MP only) | `(g_CurrentFrameCounter & 7) == 7 && g_GameMode == 4` |
| Render frame | game-tick (1:1 with logic) | `RenderFrame_main()` called inside `Main_Tick` after `Map::Logic` |
| FPS counter (telemetry) | wall-clock | `timeGetTime()` snapshots |
| Per-tick wait pacing | wall-clock vs. game-tick mix | Pacing target `DAT_00887350` derived from GameSpeed; consumed by `FUN_005d5870`/`FUN_005d5880` |
| Cinematic delay countdown (e.g. event `0x1d`) | wall-clock | uses `GetRadarTimer()` deltas, not `g_CurrentFrameCounter` |
| Radar timer | wall-clock | `GetRadarTimer()` returns a ms value |

**Confidence (binding for game-tick rows): HIGH** — directly observed in
`Main_Tick` / `LogicClass::PerTickUpdate` decompilation, callers traced via
`get_function_callers`.

**Confidence (binding for "wall-clock vs. game-tick mix" row): MEDIUM** — the
exact pacing arithmetic inside `FUN_005d5870` / `FUN_005d5880` is not yet
decompiled in this doc; deferred to [logic-vs-render-loop.md](logic-vs-render-loop.md).

### Render vs. logic separation

`RenderFrame_main` @ `0x004f4480` is called **inside** `Main_Tick`, after
`Map::Logic`. There is **no separate render thread** and **no inter-tick render
interpolation** in this code path — render runs once per logic tick. The
"render rate" and "logic rate" are therefore the same number in `gamemd.exe`:
both = GameSpeed-derived ticks/sec. (Cross-ref:
[logic-vs-render-loop.md](logic-vs-render-loop.md) will examine whether any
finer render-only updates exist via the `g_DisplayChain` vtable.)

---

## Multipliers and modifiers

### Network event 0x0d — sync the slider across all peers (MP)

`EventClass::Execute` @ `0x004c807d`, case `0x0d`:

```c
case 0x0d:
    DAT_00a8eb60 = *(undefined4 *)(param_1 + 7);
    ...
```

Confirms: when the local player changes the GameSpeed slider via
`OptionsClass::ApplyFromInGameDialog`, an event `0x0d` is queued and broadcast
so every peer's `DAT_00a8eb60` advances together. Without this, MP would
desync immediately.

### Network event 0x20 — set network frame budget (MP)

`EventClass::Execute` @ `0x004c807d`, case `0x20`:

```c
case 0x20:
    ...
    DAT_00a8b558 = (uint)*(ushort *)(param_1 + 7);    // FPS / turn-rate
    g_NetworkFrameBudget = (uint)*(ushort *)(param_1 + 9);
    ...
```

Independent of the user slider — adjusts the network turn batching to compensate
for latency. See [multiplayer-frame-step.md](multiplayer-frame-step.md).

### Network event 0x1b — set frame budget alone

`EventClass::Execute` @ `0x004c807d`, case `0x1b`:

```c
case 0x1b:
    g_NetworkFrameBudget = (uint)(byte)param_1[0xd];
    return;
```

### `GameSpeedBias`

`RulesClass::ReadGeneral` reads `GameSpeedBias` as a `double` (default `1.6`).
**Does NOT scale the master clock.** It scales **per-unit movement speed**
(see [movement-speed-turn-rate.md](movement-speed-turn-rate.md)). Listed here
only because the name is misleading.

### No other clock multiplier exists

Searches for `FrameStep`, `FrameRate` (as a clock concept rather than the
`DetailMin*` cosmetic gates) returned no clock-affecting consumer. The only
two knobs feeding the clock are the per-machine slider and the MP network
frame budget. AI difficulty, country bonuses, veterancy bonuses, crate
boosts, Powered=, Stealth= etc. do **not** modify the master clock — they
modify individual gameplay constants that are then evaluated against the
clock (e.g. a veteran unit gets a faster ROF, but ROF is still counted in
`g_CurrentFrameCounter` ticks).

---

## Edge cases

### Pause / resume

The render+logic block in `Main_Tick` is gated on:

```c
if ((((_DAT_00a8d5f8 & 2) == 0) && (g_GameState == 0)) && (g_GameRunning != '\0'))
```

When paused:
- `g_GameState != 0` skips Input / LogicClass::AI / HouseAI / Map::Logic / Render.
- `LogicClass::PerTickUpdate` still runs (so animations on the pause menu can
  continue, but most state advancement is skipped because per-tick AI is gone).
- `g_CurrentFrameCounter` is **still incremented** at the end of `Main_Tick`
  unless one of `DAT_00a83d49 / DAT_00a8ecd0 / DAT_008b41c0 / DAT_00a83d48` is
  set — those four flags collectively define "freeze the clock".

**Correction (resolved in [logic-vs-render-loop.md](logic-vs-render-loop.md)):**
the four flags are **session-end** flags, not generic "freeze the clock"
flags. They are: `DAT_00a83d49` (local victory, set by `HouseClass::Update`),
`DAT_00a8ecd0` (local defeat, set by `HouseClass::Update`), `DAT_008b41c0`
(quit-to-main confirmed, set by `State_Machine`), `DAT_00a83d48` (graceful
disconnect, set by `EventClass::Execute` case `0x13`). The clock "freezes"
on the last tick of a session because the session is tearing down — not
because of any in-game pause condition. **In-game pause** is implemented by
`g_GameState != 0`, which gates the gameplay-and-render block but **does
not** stop the counter.

### Freeze (Iron Curtain, EMP, Mind Control, Chrono freeze, Stasis)

These **do not freeze the master clock**. They freeze the affected unit only,
by setting per-unit flags / countdowns that are then compared against
`g_CurrentFrameCounter`. The clock continues to advance for everything else.
Each freeze type's per-unit timing surface is its own doc:
- [iron-curtain-duration.md](iron-curtain-duration.md)
- [emp-stun-duration.md](emp-stun-duration.md)
- [mind-control-duration.md](mind-control-duration.md)
- [chrono-warp-cooldown.md](chrono-warp-cooldown.md)

### Save / load persistence

`g_CurrentFrameCounter` is part of the saved game state. The slider value
(`DAT_00a8eb60`) is **not** part of the in-game save — it lives in the user's
options profile and is reapplied on load.

**Confidence: LOW** — this is inferred from the dual-storage design (options vs.
session) and the typical Westwood save-format convention. Not directly verified
from a save-file decoder in this iteration. Mark for follow-up when the
save/load doc is written.

### Replay determinism

`g_CurrentFrameCounter` is the determinism anchor. Replays record every player
input keyed by frame number; play-back fires `EventClass::Execute` on the same
frame. Because GameSpeed-change events (`0x0d`) and frame-budget events (`0x20`)
are themselves recorded and replayed, the **observable speed** of a replay
matches what each player experienced live, even if the replay machine has a
different default slider.

### End-of-mission pause

`g_GameState != 0` (set when the mission ends) skips the gameplay block; only
`LogicClass::PerTickUpdate` runs. The clock still advances unless one of the
four freeze flags is set.

### Low power / power-down

Does **not** affect the master clock. Powered-down buildings get per-building
slowdown via their own timers compared against the still-advancing
`g_CurrentFrameCounter`. See [power-state-machine.md](power-state-machine.md).

---

## TS-legacy filter

| Field / branch | TS-legacy? | Notes |
|---|---|---|
| `GameSpeed` slider 0..6 | **Live in YR** | The core clock — every system depends on it. |
| `GameSpeedBias` | **Live in YR** | Verified at `RulesClass::ReadGeneral`; not gated. |
| `[MultiplayerDialogSettings] GameSpeed=1` | **Live in YR** | Used as MP default. |
| `DetailMinFrameRateNormal / Movie / DetailBufferZoneWidth` | **Live in YR** | Detail downgrade gating; not the clock itself. |
| `ShadowGrow=no` (in MP settings) | **DESUPPORTED in YR** | rulesmd.ini comment marks it `DESUPPORTED`. Shroud regrow tick is TS legacy. |
| `Shroud` (in MP settings) | **NOT YET SUPPORTED** per rulesmd comment | Marked `NOT YET SUPPORTED` — treat as dead. |
| `FogOfWar=no` default | **TS legacy gating** | Default off. Fog-tick paths gated by `SpecialFlags & 0x1000` — confirmed in `LogicClass::PerTickUpdate` at the `(((*g_ScenarioClass_Instance & 0x1000) != 0) && (*(double *)(g_RulesClass_Instance + 0x1648) != _g_Const_0_0))` block. |
| TS subterranean locomotor tick | **Dead in YR** | Confirmed dormant per project memory. |

---

## Cross-references

- [logic-vs-render-loop.md](logic-vs-render-loop.md) — the per-tick wait
  pacing (`FUN_005d5870` / `FUN_005d5880`), the freeze-flag enumeration, and
  whether any render-only sub-frame updates exist
- [animation-rate-delay.md](animation-rate-delay.md) — how artmd `Rate=` /
  `Delay=` map to this clock vs. the anim sub-tick
- [multiplayer-frame-step.md](multiplayer-frame-step.md) — `DAT_00a8b558` /
  `g_NetworkFrameBudget`, network events `0x1b` and `0x20`, lockstep batching
- [movement-speed-turn-rate.md](movement-speed-turn-rate.md) — where
  `GameSpeedBias` is applied
- [power-state-machine.md](power-state-machine.md) — `Powered=` poll cadence
  on the master clock
- [shroud-reveal-decay.md](shroud-reveal-decay.md) — confirms the
  `SpecialFlags & 0x1000` fog-of-war gate is off by default in YR

---

## Coverage audit

Every INI key, global, address, and event mentioned above is routed:

| Item | Disposition |
|---|---|
| `[General] GameSpeedBias` | Owned by [movement-speed-turn-rate.md](movement-speed-turn-rate.md); cross-referenced |
| `[MultiplayerDialogSettings] GameSpeed=1` | Owned here (rules-side MP default for the master clock) |
| `[AudioVisual] DetailMinFrameRateNormal / Movie / DetailBufferZoneWidth` | Quoted here for context; rendering-effect downgrade is a separate concern (no dedicated doc yet — flag if a "render-detail-throttle.md" becomes needed) |
| `[Options] GameSpeed` (per-machine user options file, default 3) | Owned here |
| `[MultiplayerDialogSettings] ShadowGrow` / `Shroud` / `FogOfWar` | TS-legacy flags; cross-referenced to [shroud-reveal-decay.md](shroud-reveal-decay.md) |
| `g_CurrentFrameCounter` | Defined here (the master clock itself) |
| `DAT_00a8eb60` (slider value) | Defined here |
| `DAT_00a8b558` (network frame budget) | Cross-referenced to [multiplayer-frame-step.md](multiplayer-frame-step.md) |
| `g_NetworkFrameBudget` | Cross-referenced to [multiplayer-frame-step.md](multiplayer-frame-step.md) |
| `DAT_00887350` (derived wait/interval) | Cross-referenced to [logic-vs-render-loop.md](logic-vs-render-loop.md) |
| `0x78 = 120` bridge shroud cadence | Quoted as example; owned by a future bridge-shroud doc (or routed inside [shroud-reveal-decay.md](shroud-reveal-decay.md)) |
| `& 7 == 7` network keepalive | Owned by [multiplayer-frame-step.md](multiplayer-frame-step.md) |
| Event 0x0d (set slider) | Defined here |
| Event 0x1b / 0x20 (network frame budget) | Cross-referenced to [multiplayer-frame-step.md](multiplayer-frame-step.md) |
| Four freeze flags (`DAT_00a83d49 / DAT_00a8ecd0 / DAT_008b41c0 / DAT_00a83d48`) | Cross-referenced to [logic-vs-render-loop.md](logic-vs-render-loop.md) |

---

## Ghidra queries log (this iteration)

| Query | Result |
|---|---|
| `search_strings "GameSpeed"` | 6 hits: `0x00820b64`, `0x0082280c`, `0x008332d8`, `0x008332e8`, `0x0083bf24`, `0x00840030` |
| `get_xrefs_to 0x0083bf24` (GameSpeedBias key) | 1 hit: `RulesClass::ReadGeneral` @ `0x00670b26` |
| `get_xrefs_to 0x008332e8` (GameSpeed key) | 5 hits incl. `OptionsClass::ReadFromINI`, `SessionClass::ReadSkirmishSettings`, `RulesClass::ReadMultiplayerDialogSettings` |
| `decompile_function 0x005fa646` (OptionsClass::ReadFromINI) | Confirmed `GameSpeed` is field offset 0 in `OptionsClass` |
| `decompile_function 0x005fa350` (OptionsClass::SetDefaults) | Confirmed default `GameSpeed = 3` |
| `decompile_function 0x004e1de0` (OptionsClass::ApplyFromInGameDialog) | Confirmed slider→internal conversion `iVar5 = 6 - LVar2` |
| `decompile_function 0x0055d360` (Main_Tick) | Confirmed `g_CurrentFrameCounter += 1` at tick end; freeze-flag gate; network keepalive cadence |
| `decompile_function 0x0055afb0` (LogicClass::PerTickUpdate) | Confirmed bridge-shroud `% 0x78` cadence; FogOfWar `& 0x1000` gate |
| `decompile_function 0x004c807d` (EventClass::Execute) | Confirmed event 0x0d (set GameSpeed), 0x1b (frame budget), 0x20 (network frame budget) |
| `get_function_callers 0x0055afb0` | Confirmed `Main_Tick` is sole caller |
| `read_memory 0x00822730 len=128` | Confirmed 7-entry speed-label LPCSTR table → `TXT_SLOWEST..TXT_FASTEST` |
| `read_memory 0x008227b8 len=96` | Confirmed label-string contents |
| `search_strings "FPS"` / `"Slowest"` / `"FrameRate"` | Found `0x0082a1b4 "Req fps : %d"`, `0x00822800 "TXT_SLOWEST"`, FPS log via `House_AI_Tick` |
