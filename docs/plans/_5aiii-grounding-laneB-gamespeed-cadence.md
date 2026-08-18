# Slice 5a-iii Lane B grounding: GameSpeed (0..6) -> frame cadence

Goal: map the in-game Options "Game Speed" slider value to gamemd.exe's real per-frame
delay so the Rust slider drives genuine timing parity, replacing the arbitrary VERA tps
presets (15/30/60/.../500).

All addresses verified against gamemd.exe in Ghidra MCP this session (image base 0x00400000).
Confidence is VERIFIED-FROM-BINARY unless marked otherwise.

---

## TL;DR (the number you need)

The internal GameSpeed value at global `0x00A8EB60` (range 0..6, default 3) IS the per-frame
pacing interval, expressed in **units of 16 ms** (one "radar-timer" tick = `timeGetTime() >> 4`).

**Offline frame target = GameSpeed x 16 ms** (busy-wait loop):

| GameSpeed (internal) | Slider position (6 - internal) | Frame target | Cap (fps) |
|----------------------|-------------------------------|--------------|-----------|
| 6 (fastest)          | 0                             | 96 ms ... see note | ~uncapped/very high |
| 5                    | 1                             | 80 ms        | ~12.5 fps |
| 4                    | 2                             | 64 ms        | ~15.6 fps |
| 3 (default)          | 3                             | 48 ms        | ~20.8 fps |
| 2                    | 4                             | 32 ms        | ~31.2 fps |
| 1                    | 5                             | 16 ms        | ~62.5 fps |
| 0 (fastest in code)  | 6 (slowest slider)            | 0 ms         | uncapped (no sleep) |

Read this carefully: the internal value is INVERTED relative to the slider. The dialog stores
`internal = 6 - slider_pos` (verified at `OptionsClass__ApplyFromInGameDialog 0x004E1DE0`, line
`iVar5 = 6 - LVar2`). So **slider_pos = 0 is the user-facing "fastest", and it maps to internal
GameSpeed = 6**. But the pacing loop sleeps for `internal x 16 ms`, so a LARGER internal value
means a LONGER frame (slower). That means **internal 0 (slider "slowest" = pos 6) gives the
uncapped, fastest loop (0 ms target), and internal 6 (slider "fastest" = pos 0) gives the
longest 96 ms target (slowest)**.

This sign tension is real in the binary and must be reproduced exactly. See "Slider sign" below
for the resolution: the slider control is configured so the dialog's track is reversed; the
effective player-facing mapping ends up fast-at-low-internal. Treat the verified facts as:
- storage: `Options.GameSpeed = 6 - slider_pos`
- pacing: `frame_target_ms = Options.GameSpeed * 16`
Do NOT reinterpret either; port both verbatim and let them compose.

---

## 1. The consumer in the main-loop / FPS-governor path

`get_xrefs_to 0x00A8EB60` returns ~80 refs; the timing-relevant READ is in `Main_Tick`
(function at `0x0055D360`, body 0x0055D360-0x0055DEDB). All other reads are apply/INI/dialog
(`OptionsClass__ApplyFromInGameDialog`, `OptionsClass__ShowInGameDialog`,
`OptionsClass__ShowLauncherDialog`, `OptionsClass__SetDefaults`) or unrelated DATA-adjacency
false hits.

### 1a. Main_Tick captures GameSpeed into the pacing struct (offline branch)

Verified via `disassemble_function 0x0055D360`. Offline = `g_GameMode == 0` (global `0x00A8B238`).
The offline path lands at `0x0055D79E`:

```
0055d79e: MOV ESI, dword ptr [0x00a8eb60]   ; ESI = Options.GameSpeed (0..6)
0055d7a4: LEA ECX, [ESP + 0x14]
0055d7a8: CALL 0x006c8c40                    ; EAX = GetRadarTimer() = timeGetTime() >> 4
0055d7ad: MOV ECX, dword ptr [ESP + 0x14]
0055d7b1: MOV [0x00887348], EAX              ; pacing base timer  (DAT_00887348)
0055d7b6: MOV [0x0088734c], ECX              ; (DAT_0088734c, hi/aux)
0055d7bc: MOV [0x00887350], ESI              ; pacing INTERVAL = GameSpeed  (DAT_00887350)
```

So the per-frame pacing globals are:
- `DAT_00887348` = base timestamp, captured via `GetRadarTimer`
- `DAT_00887350` = the interval = **the raw GameSpeed value (0..6)**

(There is a separate single-player demo/observer branch at 0x0055D767/0x0055D770 that hardcodes
`Options.GameSpeed = 2` and `DAT_00887350 = 2` when `DAT_00a8eddc == 0`; that is the AI/no-human
case, verified in the same disassembly. The normal offline-with-human branch is 0x0055D79E above.)

### 1b. GetRadarTimer units (verified)

`decompile_function 0x006c8c40` (labeled `GetRadarTimer`):

```c
uint GetRadarTimer(void) { DWORD t = timeGetTime(); return t >> 4; }
```

So one timer unit = `1000/16` ms-resolution, i.e. **1 unit = 16 ms**. This is the unit the
pacing interval (`DAT_00887350` = GameSpeed) is measured in.

### 1c. The governor that consumes it: FUN_0055E160

`Main_Tick`'s offline return path calls `FUN_0055E160()` (call at 0x0055D854 and 0x0055DE9A)
right before returning. `get_xrefs_to 0x00887350` confirms `FUN_0055E160` is the only READER of
the interval besides `Main_Tick`'s own writes.

Verified via `disassemble_function 0x0055E160`. In **offline mode** the big `g_GameMode != 0 &&
!= 5` busy/network loop (0x0055E1C2..0x0055E2AF) is SKIPPED, and control reaches the tail pacing
loop at `LAB_0055E2E3` (0x0055E2E3):

```
0055e2e3: MOV ECX, [0x00887348]          ; base
0055e2e9: MOV ESI, [0x00887350]          ; interval = GameSpeed
0055e2f8: CALL 0x006c8c40                ; GetRadarTimer() (now>>4)
0055e303: SUB EAX, ECX                    ; elapsed = (now>>4) - base
0055e305: CMP EAX, ESI                    ; if elapsed >= GameSpeed -> done
0055e307: JGE 0x0055e33b                  ;   (exit pacing)
0055e309: SUB ESI, EAX                     ; else residual = GameSpeed - elapsed
...
0055e336: PUSH ESI
0055e337: CALL EDI                          ; EDI = [0x007e11f0] = Sleep(residual)
0055e339: JMP 0x0055e2e3                     ; loop until elapsed >= GameSpeed
```

`EDI = [0x007e11f0]`, and the decompiler resolves these `CALL EDI` sites as `Sleep(...)`
(decompile of FUN_0055E160 shows `Sleep(DVar3)` / `Sleep(DVar4 - ...)`). So the loop **spins,
sleeping the residual, until `(timeGetTime()>>4) - base >= GameSpeed`** -- i.e. until wall-clock
elapsed >= `GameSpeed * 16 ms`.

Net effect: **offline frame is paced to `GameSpeed * 16 ms`.** GameSpeed=0 => 0 => no wait =>
uncapped. GameSpeed=3 (default) => 48 ms => ~20.8 fps logic+render cap.

Note on residual unit: the `Sleep` argument is `residual = GameSpeed - elapsed`, which is in
16ms-units but passed to `Sleep` as raw ms -- so each individual Sleep under-sleeps and the loop
re-checks. The LOOP TERMINATION (the `CMP EAX,ESI / JGE` against the 16ms-unit timer) is what
sets the true cadence, and that is `GameSpeed * 16 ms` of wall time. Reproduce the termination
condition, not the imperfect per-iteration Sleep argument.

---

## 2. Exact GameSpeed -> delay mapping

It is NOT a 7-entry lookup table and NOT a curve. It is a direct linear identity:

```
pacing_interval_units = Options.GameSpeed        (no transform; MOV [0x887350], ESI)
frame_target_ms       = pacing_interval_units * 16   (because GetRadarTimer = timeGetTime>>4)
```

Concrete (frame target in ms, and the fps cap = 1000 / target):

```
GameSpeed 0 ->   0 ms  -> uncapped
GameSpeed 1 ->  16 ms  -> 62.5 fps
GameSpeed 2 ->  32 ms  -> 31.25 fps
GameSpeed 3 ->  48 ms  -> 20.833 fps   (default)
GameSpeed 4 ->  64 ms  -> 15.625 fps
GameSpeed 5 ->  80 ms  -> 12.5 fps
GameSpeed 6 ->  96 ms  -> 10.417 fps
```

This is the classic RA2 behaviour: the game does NOT separate sim-tick rate from render rate in
offline play -- one `Main_Tick` iteration = one logic frame + one rendered frame, throttled by
the GameSpeed sleep. "Game Speed = fastest" removes the sleep entirely and the loop runs as fast
as the CPU + vsync allow.

---

## 3. Default = 3 (verified)

`OptionsClass__SetDefaults` (writes `0x00A8EB60` at 0x005FA35A) seeds the default. The prompt's
ReadFromINI default of 3 is consistent with the dialog math: default internal GameSpeed = 3 =>
48 ms => ~20.8 fps, the canonical RA2 mid setting. (Confirmed default-write site exists via
`get_xrefs_to 0x00A8EB60` -> `From 005fa35a in OptionsClass__SetDefaults [WRITE]`. Exact constant
not separately disassembled here; treat "default 3" as the project-supplied value, consistent.)

---

## 4. Offline vs network (the dead branch)

`OptionsClass__ApplyFromInGameDialog 0x004E1DE0` (verified by decompile) does:

```c
iVar5 = 6 - SendMessageA(slider, TBM_GETPOS=0x400, 0, 0);   // internal = 6 - slider_pos
if (Options.GameSpeed != iVar5 && g_GameActive==1
    && g_GameMode != 0 && g_GameMode != 5) {                 // <-- NETWORK-ONLY guard
    FUN_004c6720(player, 0x0D, iVar5);                        // build EventClass type 0x0D
    ... copy 0x6F-byte event into g_CommandBuffer (queue) ...
}
Options.GameSpeed = iVar5;                                    // <-- always stores directly
```

Two facts confirmed:
1. **Offline (`g_GameMode == 0`) skips the event-queue branch entirely** (the `&& g_GameMode != 0`
   condition fails). The function simply does `Options.GameSpeed = 6 - slider_pos` directly into
   `0x00A8EB60`. `Main_Tick`/`FUN_0055E160` then read that global next frame. No netcode involved.
2. **Network (g_GameMode 1/2/3/4) defers** by queuing `EventClass` type **0x0D** (GAMESPEED).
   `EventClass__Execute` case `0xd` (verified, decompile of 0x004C794E region):
   `DAT_00a8eb60 = *(undefined4 *)(param_1 + 7);` -- i.e. it writes the same global, but only when
   the lockstep event fires on the agreed frame (so all peers change speed in sync). It also pops
   a "player changed game speed" message via StringTable id 0x593.

So for the Rust offline port, the network branch is irrelevant: store directly into the equivalent
of `Options.GameSpeed` and have the timing consumer read it. (When net play lands later, the
0x0D-event path is the parity model -- queue an event, apply on the synced frame.)

---

## 5. Slider sign (resolve the inversion before porting)

- Dialog -> storage: `Options.GameSpeed = 6 - slider_pos` (TBM_GETPOS at control 0x529).
- Storage -> pacing: `frame_ms = Options.GameSpeed * 16`.

Composing: `frame_ms = (6 - slider_pos) * 16`. So **slider_pos = 6 (the visual far end) => 0 ms =>
fastest; slider_pos = 0 => 96 ms => slowest.** In the retail dialog the trackbar is laid out so
the far/right end is "Faster", which lines up with this (far end = pos 6 = 0 ms). The Rust port
must NOT hardcode "higher slider = faster delay" independently -- it must reproduce BOTH halves:
the `6 - pos` storage AND the `*16` pacing. Verify the trackbar min/max orientation in
`OptionsClass__ShowInGameDialog 0x004E1D60` when wiring the actual UI control so the label ends up
on the correct end (UNCHECKED here -- not load-bearing for the cadence math, only for label side).

---

## 6. Rust-port recommendation (replaces the VERA tps presets)

Current Rust: the egui pause card exposed arbitrary tps presets (15/30/60/120/250/500). Those are
not gamemd values and break parity.

Replace with:
1. Store an `i32 game_speed` in the options state, range 0..6, default 3 (matches `Options.GameSpeed`
   at 0x00A8EB60 / ReadFromINI default).
2. Dialog -> store: `game_speed = 6 - slider_pos` (7-position slider, pos 0..6).
3. Frame governor -> derive the per-frame target: `frame_target = game_speed * 16 ms`
   (`GAMESPEED_TICK_MS = 16`, a named constant; it is `timeGetTime() >> 4`). game_speed = 0 means
   "no cap" (skip the wait). Pace the offline loop to that wall-clock target rather than to a tps.
4. Do NOT model the per-iteration Sleep residual literally; model the loop TERMINATION: advance the
   frame once `elapsed_ms >= game_speed * 16`. The intermediate Sleep(residual) is just yield
   behaviour, not the cadence.
5. Drop the 15/30/.../500 tps preset list from the pause/options UI entirely.
6. Network play (later): when `game_speed` changes in a netgame, queue a GAMESPEED event (gamemd
   EventClass 0x0D) and apply on the synced frame; do not mutate the global immediately. Out of
   scope for the offline slider, noted for completeness.

A faithful offline implementation is therefore: 7 discrete settings, each a multiple of 16 ms
(0/16/32/48/64/80/96), default 48 ms (~20.8 fps), where "fastest" = 0 ms = uncapped.

---

## Verification ledger

- `get_current_program_info` -> gamemd.exe loaded, image base 0x00400000.
- `get_xrefs_to 0x00A8EB60` -> timing READ isolated to `Main_Tick` (0x0055D79E); rest apply/INI/dialog.
- `disassemble_function 0x0055D360` (Main_Tick) -> 0x0055D79E stores `Options.GameSpeed` into
  pacing interval `DAT_00887350`; base `DAT_00887348 = GetRadarTimer()`.
- `decompile_function 0x006C8C40` (GetRadarTimer) -> `timeGetTime() >> 4` => 1 unit = 16 ms.
- `get_xrefs_to 0x00887350` -> consumer = `FUN_0055E160` (+ Main_Tick writes).
- `disassemble_function 0x0055E160` -> offline tail loop 0x0055E2E3: spins on
  `(GetRadarTimer() - base) >= GameSpeed`, sleeping residual via `[0x007e11f0]` (Sleep).
- `decompile_function 0x004E1DE0` (ApplyFromInGameDialog) -> `internal = 6 - slider_pos`; network
  guard `g_GameMode != 0 && != 5`; offline stores `0x00A8EB60` directly; net queues EventClass 0x0D.
- `decompile_function 0x004C794E` (EventClass__Execute) case 0xd -> `DAT_00a8eb60 = arg`.
- `get_xrefs_to 0x00A8EB60` -> `OptionsClass__SetDefaults 0x005FA35A [WRITE]` seeds default (=3 per project).

## Unverified / UNCHECKED (do not treat as load-bearing)

- Exact default constant written by `OptionsClass__SetDefaults` not disassembled byte-for-byte this
  session; "default = 3" taken from project context + dialog consistency.
- Trackbar min/max orientation in `OptionsClass__ShowInGameDialog` (label side "Faster") not
  inspected; affects only which visual end shows "fast", not the cadence math.
- The exact import-resolution detail of `[0x007e11f0]` (the on-disk image holds a thunk-stub
  address); identification as `Sleep` rests on the decompiler's import resolution, which is
  consistent across all call sites in FUN_0055E160 (Sleep(0), Sleep(residual)).
