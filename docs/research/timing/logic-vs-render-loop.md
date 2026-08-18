# Logic Loop vs. Render Loop

## Overview

**Player-visible effect:** the engine renders exactly one frame per logic tick.
There is **no smooth render interpolation between logic ticks** — when the user
sets a slower GameSpeed, both logic and visuals slow down together at the same
rate. Pause (the in-game menu) freezes the unit-AI loop and stops new render
frames from being driven by the gameplay path, but a separate menu-driven
render keeps the screen alive.

**Mechanism in plain terms:** `gamemd.exe` is a single-threaded fixed-step
loop. `Main_Game` runs `Main_Tick` in a tight `do/while` loop until session
end. `Main_Tick` is one iteration that always does **input → logic-dispatch →
`Map::Logic` → `RenderFrame_main` → late housekeeping → counter increment** in
that order. Render is not a separate timeline; it is the last step inside the
logic tick. Pause flips one of two gates that skip the gameplay-and-render
block, after which a modal-dialog state machine (`State_Machine`) takes over
the outer loop and drives its own render via the `g_DisplayChain` vtable.

Four flags signal **session end** (not pause): when set, `Main_Tick` skips the
counter increment, `FUN_0055cfd0` cleans up, and `Main_Game`'s outer loop
falls through to the menu / score / mp-disconnect screen. The
iteration-1 doc characterized these as "freeze the clock"; the more accurate
characterization is "freeze the clock because the session is ending".

---

## INI surface

**None.** The logic-vs-render relationship is hardcoded; no INI key controls
it. The closest INI-driven concern is `[AudioVisual] DetailMinFrameRateNormal=15
/ DetailMinFrameRateMovie=20 / DetailBufferZoneWidth=5` — these gate
visual-effect downgrades (extra animations off, etc.) when the measured
frame rate dips below the threshold. They live on the render side and do not
re-time logic. Already documented in
[game-speed-master-clock.md](game-speed-master-clock.md); cross-referenced
here for completeness.

---

## Hardcoded constants

### `Main_Game` outer loop — `0x0048ccc0`

```c
do {
    cVar1 = Main_Tick();
    if (cVar1 && FUN_0055cfd0()) break;
    State_Machine();
    cVar1 = FUN_0055cfd0();
} while (cVar1 == '\0');
```

Two execution paths per outer iteration:

1. **`Main_Tick`** — runs always; drives the gameplay loop (or skips it during
   pause).
2. **`State_Machine`** — runs when `g_GameState != 0` (the game is in a modal
   state); drives the in-game menu / save dialog / load dialog / score screen
   / scenario-victory display.

### `Main_Tick` body — `0x0055d360`

The whole tick body, normalized:

```c
// (a) Network bookkeeping + input wait (MP: derives DAT_00887350 from
//     network frame budget; SP: just stores DAT_00a8eb60 as wait target).
//     timeGetTime() snapshot for FPS measurement.

// (b) GAMEPLAY BLOCK — gated by both "transition flag" and "game state":
if ((((_DAT_00a8d5f8 & 2) == 0) && (g_GameState == 0)) && (g_GameRunning != '\0')) {
    GScreenClass__Input(local_18c, local_184, local_188);   // poll DisplayChain for kb/mouse
    LogicClass__AI();                                        // (misnamed) input-event dispatch
    if (DAT_00a8b8b4 != '\0') House_AI_Tick();
    if (((byte)g_CurrentFrameCounter & 7) == 7 && g_GameMode == 4)
        Network_Keepalive();
    Map__Logic();                                            // per-cell "logic" pass (mostly flag marking)
    RenderFrame_main();                                      // (c) render is INSIDE the gameplay block
}

// (d) Replay record/playback bookkeeping (if recording or playing back —
//     gated by _DAT_00a8d5f8 & 1 / & 2 respectively). RenderFrame_main is
//     called again here on replay-playback so the screen still updates.

// (e) LATE HOUSEKEEPING — UNCONDITIONAL:
LogicClass__PerTickUpdate();
//   Inside: TiberiumClass::GrowthDriver_AllTypes, TiberiumClass::SpreadDriver_AllTypes,
//           BombClass::UpdateAll, DiskLaserClass arr loop, LaserDrawClass::UpdateAllAI,
//           LightningStorm::Process, EMPulseClass::UpdateAll, TeamClass arr loop,
//           LogicClass entity-array vtable+0x5c loop, FactoryClass arr loop,
//           HouseClass arr loop, DisplayClass tactical sound update.

// (f) FPS accumulator update + Network_ServiceLoop

// (g) ADVANCE THE MASTER CLOCK (gated by session-end flags):
if (DAT_00a83d49 == 0 && DAT_00a8ecd0 == 0 && DAT_008b41c0 == 0 && DAT_00a83d48 == 0) {
    g_CurrentFrameCounter = g_CurrentFrameCounter + 1;
    ...
    return (uint)(g_GameActive == '\0');
}
```

### Render-only sub-path (transition / cutscene)

```c
if (*(int *)(g_ScenarioClass_Instance + 0x62c) != 0) {
    Process_NetworkMessages();
    Network_ServiceLoop();
    Process_QueuedEvents();
    (**(code **)(*g_Tactical + 0x5c))();
    RenderFrame_main();
    ...
    return;
}
```

Quoted from `Main_Tick` LAB_0055d821. When the scenario instance has its
"display-only" word at offset `0x62c` set, the tick returns immediately after
a render — no input, no AI, no logic, no counter advance. Used during opening
cinematics / scenario intros.

### `RenderFrame_main` — `0x004f4480`

```c
void RenderFrame_main(int *param_1) {
    g_PrimarySurface = DAT_0088731c;
    (**(code **)(*g_DisplayChain + 0x40))(DAT_0088731c, 0);     // begin frame
    ...
    if (FUN_0053bae0() == '\0') {
        TacticalClass_Draw(g_Tactical, DAT_0088731c, ..., 0);
        TacticalClass_Draw(g_Tactical, DAT_0088731c, ..., 1);
        (**(code **)(*param_1 + 0x40))(iVar1 == 2);              // GScreen draw
        TacticalClass_Draw(g_Tactical, DAT_0088731c, ..., 2);
    }
    if (DAT_00b0b519 != '\0' && g_IsMapEditor == '\0') {
        (**(code **)(*g_DisplayChain + 0x40))(g_SidebarSurface, 1);
        DAT_00b0b519 = '\0';
    }
    ...
    if (DAT_00a8b8b4 != '\0') House_AI_Tick();   // ← FPS counter / HUD update
    (**(code **)(*g_DisplayChain + 0x3c))(DAT_0088731c, 0);     // end frame
    (**(code **)(*param_1 + 0x44))();
}
```

`TacticalClass_Draw` is called **three times** per render — layers 0, 1, 2.
These are paint passes (terrain → objects → overlays), not separate frames.

`House_AI_Tick` is invoked **inside** `RenderFrame_main` as well as from
`Main_Tick` — it's both per-tick logic and a render-side housekeeping hook.

**No render interpolation:** there is no `Update(deltaTime)` pattern. The
render samples whatever logic-state is "current"; logic state only advances
once per outer tick.

### `State_Machine` — `0x0048c8b0`

Modal-state dispatcher invoked when `g_GameState != 0`:

```c
if (g_GameState == 0) return;
...
FUN_00683eb0();
(**(code **)(*g_DisplayChain + 0x14))();    // input-poll-for-menu
iVar1 = (**(code **)(*g_DisplayChain + 0x28))();
while (iVar1 != 0) {
    (**(code **)(*g_DisplayChain + 0x10))();
    iVar1 = (**(code **)(*g_DisplayChain + 0x28))();
}
switch (g_GameState) {
    case 1: FUN_004f10e0();                         // generic menu
    case 2: ...                                      // retire/abort
    case 3: switch(FUN_004f1840()) { ... }          // exit-confirm
    case 4: FUN_005fbef0(); g_GameState = 5;        // victory → options menu
    case 5: OptionsClass__ShowInGameDialog();        // options menu
    case 6: FUN_006b6230(); g_GameState = 5;        // sound controls
    case 7: FUN_0077d840();                          // score screen
    case 8: FUN_006586d0();                          // save game
    case 9: CDFileClass__Constructor();              // load game (CD prompt)
}
...
UpdateWindow(g_hWnd);
(**(code **)(*g_DisplayChain + 0x18))();
```

Confirmed `g_GameState` values:

| Value | Path | Purpose |
|---|---|---|
| 0 | Gameplay (gameplay block runs in `Main_Tick`) | Normal play |
| 1 | `FUN_004f10e0` | Generic menu (inner modal loop) |
| 2 | retire/abort flow | "Abandon this scenario" confirm |
| 3 | exit-confirm | Quit-to-main confirm; sets `DAT_008b41c0 = 1` on case-5 sub-result |
| 4 → 5 | victory → options | Mission victory display |
| 5 | `OptionsClass::ShowInGameDialog` | In-game options dialog (the pause menu) |
| 6 → 5 | sound-controls → options | Sound dialog |
| 7 | `FUN_0077d840` | Score screen |
| 8 | save-game dialog | Save dialog |
| 9 | load-game dialog | Load dialog (with CD prompt) |

`OptionsClass::ShowInGameDialog` (the in-game pause menu) is the **canonical
"pause"** entry point: when invoked, it sets `g_GameState = 5` and spins on
its own modal loop. Each modal iteration calls `Main_Tick` (since `Main_Game`
keeps running) but `Main_Tick`'s gameplay block is gated off by
`g_GameState != 0`, so per-tick AI / Input / Map / Render are skipped.

### `FUN_0055cfd0` (session-end handler) — `0x0055cfd0`

Called at end of each `Main_Game` iteration. Returns `g_GameActive == 0` (=
"loop should exit"). Inside, if any of the four session-end flags is set:

```c
if (DAT_00a83d49 != 0 || DAT_00a8ecd0 != 0 ||
    DAT_008b41c0 != 0 || DAT_00a83d48 != 0) {
    DAT_00a8dab4 = DAT_00a8dab4 + 1;
    FUN_00684240();
    ...
    // Clear all four flags. Route through:
    //   - DAT_00a8ecd0 path → FUN_00685dc0() (defeat screen?)
    //   - DAT_00a83d49 path → FUN_00685670() (victory screen?)
    //   - DAT_008b41c0 path → FUN_006863e0() (exit/quit cleanup)
    //   - DAT_00a83d48 path → GameExit__BattleControlTerminated() (graceful disconnect)
    ...
}
return g_GameActive == '\0';
```

### The four session-end flags (corrected from iteration 1)

| Flag | Set by | Semantics |
|---|---|---|
| `DAT_00a83d49` | `HouseClass::Update` @ `0x004f867c`, `0x004f8692`, `0x004f86ee`, `0x004f87bb` | **Local player victory** — set when this player's house wins (`field_0x1f7`); SP/MP path differ on `this == g_PlayerPtr` |
| `DAT_00a8ecd0` | `HouseClass::Update` @ `0x004f86f7`, `0x004f879c`, `0x004f87b2` | **Local player defeat** — set when this player's house loses (`field_0x1f8`); SP/MP differ on `this == g_PlayerPtr` |
| `DAT_008b41c0` | `State_Machine` @ `0x0048cb2e` (case 3 sub-case 5) | **User confirmed quit-to-main** from exit dialog |
| `DAT_00a83d48` | `EventClass::Execute` @ `0x004c7917` (case `0x13` EXIT event) | **Graceful disconnect / "I'm leaving" network event** |

All four cause `Main_Tick` to **skip** `g_CurrentFrameCounter++` and return —
which is what iteration 1 observed as "freeze the clock". Re-categorizing as
session-end is accurate: the clock freezes because the session is teardown.

### `LogicClass::AI` is misnamed (it is input-event dispatch)

`LogicClass::AI` @ `0x0055dee0` is **not** the per-entity AI driver. It is an
input/network-event dispatch routine that delegates via a vtable lookup
(`FUN_0055f6e0`, `+0x14`, `+0x18`, `+0x1c`, `+0x20`) and handles special
key codes (`0x1b`, `0x09`, `0x25`–`0x28`). Caller `FUN_0055e420` handles chat
typing, beep / paragraph / Enter / Backspace, and event broadcasting. The
Ghidra label is a remnant from C&C/TS where the equivalent function did
contain AI dispatch.

**The actual per-entity per-tick AI loop** is the unnamed vtable-+0x5c loop at
the end of `LogicClass::PerTickUpdate`:

```c
iVar6 = 0;
if (0 < *(int *)(param_1 + 0x10)) {
    do {
        (**(code **)(**(int **)(*(int *)(param_1 + 4) + iVar6 * 4) + 0x5c))();
        iVar6 = iVar6 + 1;
    } while (iVar6 < *(int *)(param_1 + 0x10));
}
```

`param_1+4` is the LogicClass entity array; `param_1+0x10` is the count. The
vtable `+0x5c` slot is the per-tick `AI()` method on each `AbstractClass`
descendant. **Identifying which vtable slot is `AI()` is deferred** — it
should be cross-verified by reading a known TechnoClass vtable at a known
address and confirming the slot. Tracked as a follow-up.

### `Map::Logic` — `0x004d2370`

Per-tick map pass — iterates `DAT_008b3d14` (count `DAT_008b3d20`) and marks
cells with bit `0x400000` in `cell.field_0x140`. **Not the per-cell AI** — it
walks specific tracked objects (waypoints? trigger volumes?), reads their
position, and tags the underlying cell. The actual per-cell AI (smudges,
overlays) appears to live in `LogicClass::PerTickUpdate`.

**Confidence: MEDIUM** — only one iteration's analysis; mark for revisit if a
later doc (smudge-related, overlay-related) finds another per-cell dispatch.

### `GScreenClass::Input` — `0x004f4320`

Per-tick input poll. Reads mouse + keyboard via `g_DisplayChain` vtable
`+0x2c`, `+0x30`, `+0x5c`, `+0x28`. Returns three out-params: pressed key,
mouse-x, mouse-y. Forwarded to the screen's `+0x28` handler (input
dispatcher).

### Network frame budget gate (MP only)

```c
if (g_GameMode == 4 && 0x1e < g_NetworkFrameBudget) {
    // network frame-budget management:
    //   if any peer is lagging > 1/4 of g_NetworkFrameBudget, add 10ms wait
    //   if any peer is lagging > 1/2 of g_NetworkFrameBudget, add 10ms wait
    //   if any peer is lagging > 3/4 of g_NetworkFrameBudget, add 10ms wait
    goto LAB_0055d7c2;
}
```

In MP, if `g_NetworkFrameBudget > 30`, an adaptive pacing loop adds 10ms to
the wait per quartile of "biggest peer lag". The 0x1e (30) threshold is the
minimum network frame budget below which adaptive pacing is disabled.
Detail belongs in [multiplayer-frame-step.md](multiplayer-frame-step.md).

### Frame-pacing helpers

`FUN_005d5870` = **`timeBeginPeriod(1)`** — set Windows multimedia timer
resolution to 1 ms.
`FUN_005d5880` = **`timeEndPeriod(1)`** — restore default resolution.

These do **not** sleep — they enable high-precision waits. They bracket each
tick's pacing window.

`GetRadarTimer` @ `0x006c8c40` = **`timeGetTime() >> 4`** — wall-clock ms
shifted right 4 (= ms / 16, → ~62.5 Hz units). Used as a coarse timer for
voice-pump and UI loops; **not** the master clock.

### Sleep / yield calls in `Main_Tick`

```c
while (g_GameRunning == '\0') {
    if (g_GameMode != 0 && g_GameMode != 5) {
        Sleep(10);
        Process_NetworkMessages();
        break;
    }
    Sleep(500);                  // ← SP/replay pre-game wait
    Process_NetworkMessages();
}
```

The only explicit `Sleep` in the per-tick body is the **pre-game spinwait**
(before `g_GameRunning` becomes nonzero). Per-tick frame pacing **does not
call `Sleep`** explicitly — pacing is governed by the network frame budget
wait calculation (MP) or runs uncapped (SP). The slider value
`DAT_00a8eb60` is recorded into `DAT_00887350` for telemetry/use elsewhere
but `Main_Tick` itself never calls `Sleep(DAT_00887350)`.

**Confidence: HIGH for the absence of `Sleep` in the per-tick body** —
read directly. **Confidence MEDIUM on what actually paces SP** — the
GScreenClass / DisplayChain input layer may block on Windows message pump
for the remainder of the tick budget. Cross-ref deferred to a future
window-pump doc if needed.

---

## Tick / frame topology

| Stage | Clock | Function | Gated by |
|---|---|---|---|
| Pre-game spin | wall-clock | `Sleep(500)` / `Sleep(10)` in `Main_Tick` | `g_GameRunning == 0` |
| Input poll | game-tick | `GScreenClass::Input` | `_DAT_00a8d5f8 & 2 == 0 && g_GameState == 0` |
| Input-event dispatch | game-tick | `LogicClass::AI` (misnamed) | same |
| House AI | game-tick | `House_AI_Tick` | same + `DAT_00a8b8b4` |
| Network keepalive | game-tick / 8 | `Network_Keepalive` | same + `g_GameMode == 4` |
| Map/cell logic | game-tick | `Map::Logic` | same |
| **Render** | **game-tick (1:1 with logic)** | `RenderFrame_main` | same |
| Replay record/playback | game-tick | inline | `_DAT_00a8d5f8 & 1` or `& 2` |
| Tiberium growth / spread | game-tick | `TiberiumClass::Growth/SpreadDriver_AllTypes` | unconditional |
| Bombs | game-tick | `BombClass::UpdateAll` | unconditional |
| Disk lasers, laser draws, lightning, EMP | game-tick | various | unconditional |
| Per-entity AI loop (vtable+0x5c) | game-tick | unnamed loop in `LogicClass::PerTickUpdate` | unconditional |
| Factories | game-tick | vtable+0x5c iter over `g_FactoryClass_Array` | unconditional |
| Houses | game-tick | vtable+0x5c iter over `g_HouseClass_Array` | unconditional |
| FPS accumulator | wall-clock + game-tick | inline `timeGetTime()` delta | unconditional |
| Counter advance | game-tick | `g_CurrentFrameCounter += 1` | no session-end flag |

**Key takeaway:** the only stages that pause-gate are inside the gameplay
block: Input, input-event dispatch, House AI, network keepalive, Map::Logic,
RenderFrame_main. **Everything in `LogicClass::PerTickUpdate` runs during
pause**, including the per-entity vtable-+0x5c AI loop, Tiberium growth,
factories, and houses.

This means *if* `vtable+0x5c` is `AI()`:
- Animations continue during pause (AnimClass::AI)
- Tiberium continues growing during pause
- Building production progresses during pause (FactoryClass::Update)
- HouseClass::Update runs during pause — which is where the win/loss flags
  get set, where superweapon charge accumulates, where build queues advance

This is at odds with conventional "pause = freeze everything" intuition, but
matches Westwood's classic behavior where opening the in-game menu does
**not** halt the world entirely. Verification of this against gamemd live
behavior is **deferred** — the per-entity loop's vtable slot needs naming
first.

### Render-only frame paths

| Path | When | What it does |
|---|---|---|
| `Main_Tick` LAB_0055d821 early-return | `*(int*)(g_ScenarioClass_Instance + 0x62C) != 0` (scenario-delay int flag; NOT byte `0x18B`) | Render once, no logic, no counter |
| `Main_Tick` replay-playback (`_DAT_00a8d5f8 & 2`) | During replay playback | Reads recorded inputs, renders, returns |
| `State_Machine` dispatch | `g_GameState != 0` | Drives menu/dialog rendering via `g_DisplayChain` vtable; gameplay block in `Main_Tick` is skipped |

---

## Multipliers and modifiers

### `g_GameState != 0` — modal pause

Skips the gameplay-and-render block in `Main_Tick`. `State_Machine` in the
outer `Main_Game` loop drives the modal UI.

### `_DAT_00a8d5f8 & 2` — game-transition flag

Skips the gameplay-and-render block. Set during scenario-end transitions and
replay playback. Forces an early return through the render-only sub-path so
the screen stays alive while scenario teardown / next-scenario setup happens
asynchronously.

### `g_GameRunning == 0` — pre-tick spinwait

Drains network messages while sleeping 500 ms (SP) or 10 ms (MP) until
something else clears the flag. Used at scenario start before the first real
tick.

### `g_GameActive == 0` — exit the outer loop

When the app is about to terminate; checked at the start of `Main_Tick` to
short-circuit; checked at end of `FUN_0055cfd0` to break the `Main_Game` `do
/ while`.

### Session-end flags (`DAT_00a83d49`, `DAT_00a8ecd0`, `DAT_008b41c0`, `DAT_00a83d48`)

When any is set, `Main_Tick` skips `g_CurrentFrameCounter++`. `FUN_0055cfd0`
in the outer `Main_Game` loop then runs cleanup and routes to the appropriate
post-game screen (victory / defeat / quit / disconnect).

### No fractional / no sub-stepping

There is no fixed-timestep accumulator like `while (dt > step) { tick();
dt -= step; }`. The engine does not run multiple logic steps per render
frame; it runs **exactly one** logic step per render. Slowdown (game-speed
slider, network lag, CPU starvation) produces both slower logic and slower
visuals at the same ratio. This is a determinism win — every machine
observes the same sequence of integer frames.

---

## Edge cases

### Pause behavior is partial, not total

As detailed above, `LogicClass::PerTickUpdate` and `g_CurrentFrameCounter`
both run during pause. Only the gameplay-and-render block (Input / AI /
Map / Render) skips. **Player-visible effect:** the world appears frozen
because nothing is rendered new, but underlying state (Tiberium, factories,
houses, animation frame indices) advances.

**Cross-reference for follow-up:** every later doc that mentions a per-tick
cadence (`game-tick`) needs to be checked against this partial-pause model.
If a system's behavior is "frozen during the menu", it must live behind the
gameplay-block gate (e.g. inside `LogicClass::AI` chain or via per-entity
AI() that itself checks `g_GameState`). If it lives in `LogicClass::PerTickUpdate`,
it advances during pause.

### Save / load mid-tick

`g_GameState = 8 / 9` (save/load dialog) is set from the menu (case 5) →
modal sub-flow. The save snapshot is taken between `Main_Tick` iterations
(not mid-tick), so `g_CurrentFrameCounter`, all `field_0x298`-style "start
frame" markers, and the entity vtable-+0x5c countdowns are coherent.

### Replay determinism contract

- Each player input becomes an `EventClass` with `g_CurrentFrameCounter` as
  its timestamp.
- During replay, `Network_ServiceLoop` injects recorded events back into the
  queue at the same frame.
- Because logic is deterministic (fixed-point arithmetic + sorted
  `g_HouseClass_Array` / `g_FactoryClass_Array` iteration + `BTreeMap`-equivalent
  storage), replaying the same events on the same starting state reproduces
  every frame bit-for-bit.
- Render does **not** affect replay output — it samples logic state
  read-only.

### Mid-tick session end

The four session-end flags can be set from inside `LogicClass::PerTickUpdate`
(via `HouseClass::Update`). The flag is **checked at end-of-tick**, after
`Main_Tick`'s late housekeeping completes. So the tick that ends the
session still finishes its render and its `LogicClass::PerTickUpdate`; the
**next** tick's counter increment is the one that is skipped.

### Network desync trigger

`Desync_Handler` is invoked from `Main_Tick`'s replay-record path when the
recorded checksum of `g_CurrentObjects` diverges from the local
checksum. Indicates logic states diverged across peers. The desync handler
freezes the local sim (probably sets one of the session-end flags or queues
a Disconnect event).

### Low-FPS fallback

If `DAT_00a8b564 > 0`, the average ms-per-tick is `DAT_00a8b560 / DAT_00a8b564`.
Compared to `DetailMinFrameRateNormal=15` (= 67 ms target), the engine
switches off "extra animations" when the moving average exceeds this. This
is a **render-side** detail downgrade — it does not change the logic clock.
Cross-ref [game-speed-master-clock.md](game-speed-master-clock.md).

---

## TS-legacy filter

| Branch | TS-legacy? | Notes |
|---|---|---|
| `LogicClass::AI` (the misnamed input dispatcher) | **Repurposed in YR** | Function name dates from TS where it really was AI; the YR body is input/chat dispatch. |
| Scenario instance `field_0x18B` (intro-frame flag) | **Live in YR** | Used by skirmish/mission intro cinematics. |
| `g_GameMode == 5` (replay) | **Live in YR** | Replay playback path. |
| `g_GameMode == 4` (MP / WOL) | **Live in YR** | Internet MP path. |
| `g_GameMode == 3` (LAN) | **Live in YR** | LAN MP path. |
| Render-only sub-path on `scenario[0x62c]` | **Live in YR** | Intro cinematic gate. |
| `State_Machine` case 4 (victory display) | **Live in YR** | Mission victory. |
| `State_Machine` case 9 (load-game CD prompt) | **Live in YR but vestigial** | The CD-prompt path exists; modern installs may bypass via no-CD patches. |
| The "Active Players List" lookup loop in `LogicClass::PerTickUpdate` | **Possibly TS-leftover formatting** | Looks like a scoreboard-style update; needs a focused look from a UI/scoreboard doc to confirm. |

---

## Cross-references

- [game-speed-master-clock.md](game-speed-master-clock.md) — defines
  `g_CurrentFrameCounter` and the GameSpeed slider; this doc shows where
  in the tick body it advances and which subsystems gate vs. don't
- [multiplayer-frame-step.md](multiplayer-frame-step.md) — `g_NetworkFrameBudget`,
  adaptive pacing in MP, the `0x1e` threshold
- [animation-rate-delay.md](animation-rate-delay.md) — animations live in
  `LogicClass::PerTickUpdate`'s per-entity loop and therefore advance during
  pause
- [ore-growth-spread.md](ore-growth-spread.md) — Tiberium growth runs in
  `LogicClass::PerTickUpdate` and therefore advances during pause
- [building-construction-anim.md](building-construction-anim.md) /
  [unit-build-time.md](unit-build-time.md) — `g_FactoryClass_Array` is
  iterated in `LogicClass::PerTickUpdate` and therefore advances during pause
- [power-state-machine.md](power-state-machine.md) — `HouseClass::Update` runs
  in `LogicClass::PerTickUpdate` and therefore power state advances during pause

---

## Coverage audit

| Item | Disposition |
|---|---|
| `[AudioVisual] DetailMinFrameRate*` | Owned by [game-speed-master-clock.md](game-speed-master-clock.md); cross-referenced |
| `g_CurrentFrameCounter` | Defined by [game-speed-master-clock.md](game-speed-master-clock.md); use site documented here |
| `g_GameState` | Defined here (modal-state-machine dispatcher) |
| `_DAT_00a8d5f8 & 1 / & 2` | Defined here (replay-record / game-transition flags) |
| `g_GameRunning` / `g_GameActive` | Defined here (pre-game / app-shutdown gates) |
| `g_GameMode` 0..5 | Cross-referenced; defined more fully in [multiplayer-frame-step.md](multiplayer-frame-step.md) |
| Four session-end flags | Defined here (corrected from iteration 1's "freeze the clock" framing) |
| `DAT_00887350` derived wait | Quoted; the actual SP pacing mechanism (Windows message pump? input layer?) is deferred to a future window-pump doc |
| `0x78 = 120` / `& 7` cadences | Owned by [game-speed-master-clock.md](game-speed-master-clock.md) |
| `vtable + 0x5c` per-entity AI | Defined here as a concept; vtable-slot identity verification deferred |
| `vtable + 0x40 / 0x3c / 0x44 / 0x28 / 0x2c / 0x30` (DisplayChain) | Render & input vtable slots; documented at use-site here. Full DisplayChain vtable layout deferred to a render-architecture doc if one becomes needed |
| `FUN_005d5870 / FUN_005d5880` (`timeBeginPeriod / timeEndPeriod`) | Defined here |
| `GetRadarTimer` | Defined here |

---

## Ghidra queries log (this iteration)

| Query | Result |
|---|---|
| `decompile_function 0x005d5870` | `timeBeginPeriod(1)` |
| `decompile_function 0x005d5880` | `timeEndPeriod(1)` |
| `search_functions "GetRadarTimer"` | `0x006c8c40` |
| `decompile_function 0x006c8c40` | `timeGetTime() >> 4` |
| `decompile_function 0x0055cfd0` (session-end handler) | Confirmed all four flags trigger session cleanup; routes to victory/defeat/quit/disconnect handlers |
| `search_functions "State_Machine"` | `0x0048c8b0` |
| `decompile_function 0x0048c8b0` | Modal-state dispatcher with 9 cases mapped above |
| `decompile_function 0x004d2370` (Map::Logic) | Per-tick cell flag-marking pass |
| `decompile_function 0x0055dee0` (LogicClass::AI) | Confirmed misnamed — actually input-event dispatch via vtable+0x14/+0x18/+0x1c/+0x20 |
| `decompile_function 0x0055e420` (caller of LogicClass::AI) | Input/chat/event dispatch |
| `decompile_function 0x004f10e0` (State_Machine case 1) | Generic modal-loop |
| `decompile_function 0x004f4320` (GScreenClass::Input) | Per-tick input poll via DisplayChain vtable+0x2c/+0x30/+0x5c/+0x28 |
| `get_xrefs_to 0x00a83d48` (4th session-end flag) | Set by Main_Game (WRITE) + EventClass::Execute case 0x13 (WRITE), read by Main_Tick + FUN_0055cfd0 |
| `get_xrefs_to 0x00a83d49` (1st session-end flag) | Set by HouseClass::Update on `field_0x1f7` (victory) |
| `get_xrefs_to 0x00a8ecd0` (2nd session-end flag) | Set by HouseClass::Update on `field_0x1f8` (defeat) |
| `get_xrefs_to 0x008b41c0` (3rd session-end flag) | Set by State_Machine case 3 (quit confirm) |
| `get_xrefs_to 0x00a8d5f8` (transition flag) | Set/read across Main_Game + Main_Tick + replay paths |
| `decompile_function 0x004f86f0` (HouseClass::Update) | Confirmed semantics of the four flags (victory/defeat/quit/disconnect) |
