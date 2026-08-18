# LogicClass::PerTickUpdate — Rung 25 (Y): Tactical / DisplayClass per-tick

**Status:** VERIFIED from binary this session.
**Parent:** `LogicClass::PerTickUpdate` @ `0x0055AFB0` (label `LogicClassPerTickUpdateLiveVector`).
**Authority:** binary -> Ghidra. Body site keyed to **disassembly** at
`disassemble_function 0x0055AFB0`; driver identified through the live-object vtable
(`vtable__Tactical` @ `0x007f4348`) and keyed to `decompile_function 0x006d2540`.

---

## Order / position

- **Order:** 25 of 28. Runs immediately **after** Rung X
  (`MapClass__UpdateCrateRegenTimers` @ `0x0056BBE0`, body call `0055b65a`) and immediately
  **before** Rung Z (FactoryClass tick loop, body starting `0055b66a`).

## Body site (exact)

`disassemble_function 0x0055AFB0`, instructions `0055b65f`–`0055b66a`:

```
0055b65a  CALL 0x0056bbe0            ; <-- Rung X: MapClass__UpdateCrateRegenTimers
0055b65f  MOV  ECX,[0x00887324]      ; ECX = g_Tactical (singleton object pointer)   THIS RUNG
0055b665  MOV  EAX,[ECX]             ; EAX = object vtable
0055b667  CALL [EAX + 0x5c]          ; dispatch vtable slot +0x5c  (THIS RUNG)
0055b66a  MOV  EAX,[0x00a83e40]      ; <-- Rung Z begins (FactoryClass count load)
```

- **Gate: UNCONDITIONAL** at the body site. There is no branch guarding `0055b65f`–`0055b667`;
  the dispatch always fires once per tick. (Confirms the spine "gate unconditional".)
- **Walks a single object:** `g_Tactical` at `[0x00887324]` — exactly one dispatch, no loop.
  Confirms spine "walks single object (g_Tactical) @ 0055b65f-0055b667".

## Object identity — `g_Tactical` @ `0x00887324`

- `0x00887324` is labelled `g_Tactical` and is **null in the static image** (`read_memory
  0x00887324` -> all zeroes); the singleton is allocated and stored at runtime.
- **Constructed by** `Tactical__Constructor` @ `0x006d1e1e` (the lone non-teardown WRITE to
  `0x00887324`; verified via `get_xrefs_to 0x00887324` -> `006d1e1e in Tactical__Constructor
  [WRITE]`, and `decompile_function 0x006d1e1e` ends with `g_Tactical = param_1;`).
- The object is a **TacticalClass : DisplayClass** instance (Ghidra label
  `vtable__TacticalClass__DisplayClass`). **Important vtable note:** the constructor installs
  the **most-derived** primary vtable `*param_1 = &vtable__Tactical` (`0x007f4348`), NOT the
  base-class label `vtable__TacticalClass__DisplayClass` (`0x007e608c`). The runtime dispatch
  therefore resolves through `vtable__Tactical`.
- **Slot +0x5c** (`0x007f4348 + 0x5c = 0x007f43a4`): `read_memory 0x007f43a4` -> bytes
  `40 25 6d 00` = **`0x006d2540`** = `TacticalClass__AI`. This is the per-tick driver.
  (Confirms the spine's "driver vt+0x5c on g_Tactical".)

## Purpose (one line)

Per-tick service of the **tactical view/camera**: advances the smooth camera-scroll
interpolation toward the pending target, commits the resulting view center + view bounds, and
ticks a wall-clock radar-refresh timer.

## Driver — `TacticalClass__AI` @ `0x006d2540`

`decompile_function 0x006d2540`:

- **Per-tick dedup guard:** `if (*(this+0xa8) == g_CurrentFrameCounter) goto LAB_006d26ed;`
  — the heavy scroll-lerp block runs at most once per frame; the field `this+0xa8` is set to
  `g_CurrentFrameCounter` at `LAB_006d26ed`.
- **Early-return gate (internal):** `if ((DAT_00a8d5f8 & 2) != 0) return;` — a display/game-
  state flag (`get_xrefs_to 0x00a8d5f8` shows it is read/written by `Main_Game`, `Main_Tick`,
  `Init_Random_Number_System` — a main-loop mode/suppress flag, e.g. during load/serialize).
  Bit `0x2` set => the view tick is suppressed for that frame. This is an *internal* guard;
  the call from the parent is still unconditional.
- **Camera-scroll interpolation:** when scroll-active (`DAT_00a8ed5c != 0` and the current
  view center `this+0xd0/0xd4` differs from the home value `DAT_00b0ce08/0c` and the step
  `this+0xd8 != 0`), it advances the lerp fraction `this+0xdc += this+0xd8`, clamps it to 1.0,
  computes the interpolated point via `FUN_0075f5c0`, clamps it to the map/viewport via
  `FUN_006d8640`, writes the new view center into `this+0xd64/0xd68` (and mirror
  `this+0xd74/0xd78`), and recomputes view bounds via `FUN_006d8b30`.
- **Radar / view-refresh timer:** uses `GetRadarTimer()` to drive the `this+0xda0..0xdac`
  refresh-cadence block; period is loaded from `*(g_RulesClass_Instance + 0x50)`.
- **Tail commit:** at `LAB_006d26ed`, if the committed view center (`this+0xd74/0xd78`)
  diverged from the mirror (`this+0xd64/0xd68`) and no lerp ran this frame, it re-clamps via
  `FUN_006d8640` and recommits + recomputes bounds via `FUN_006d8b30`.

### Full callee set (verified)

`get_function_callees 0x006d2540` returns exactly:

- `FUN_006d8640` @ `0x006d8640` — viewport/radar coordinate **clamp** (pure integer math on
  `g_RadarViewportWidth/Height` and `DAT_0087f8dc..f0`; `decompile_function 0x006d8640`).
- `FUN_006d8b30` @ `0x006d8b30` — recompute view bounds: `Matrix3x4_TransformPoint` +
  `Math__ftol`, writes `this+0xb0/0xb4/0xd80..0xd8c` (`decompile_function 0x006d8b30`).
- `FUN_0075f5c0` @ `0x0075f5c0` — 2-component lerp finalize: two `Math__ftol` calls
  (`decompile_function 0x0075f5c0`). No RNG.
- `GetRadarTimer` @ `0x006c8c40` — `return timeGetTime() >> 4;` (`decompile_function
  0x006c8c40`). **Wall-clock** real-time read (winmm `timeGetTime`), NOT a frame counter and
  NOT an RNG.

## RNG

- **draws_rng: NO.** `TacticalClass__AI` and its entire transitive callee set
  (`FUN_006d8640`, `FUN_006d8b30`, `FUN_0075f5c0`, `GetRadarTimer`) contain **no RNG draw**.
  No reference to `Scen->Random`, `g_MainRng`, or `g_MapGenRng` appears at any draw site; the
  only "randomness-adjacent" call, `GetRadarTimer`, is `timeGetTime()` (system clock), used for
  a local-display refresh cadence.
- **rng_stream: none.** This rung does not advance any RNG stream and therefore contributes
  **nothing** to the lockstep RNG-draw order.

## Lockstep / determinism note

- The scroll-lerp and view-bounds outputs are written to TacticalClass fields
  (`this+0xd64/0xd68/0xd74/0xd78/0xd80..0xd8c`) that are **camera/view state, not simulation
  state** — they drive what the local player sees, not unit/economy logic. `sim/` must not
  depend on them.
- The radar-refresh cadence is driven by **wall-clock** `timeGetTime()`, which is inherently
  non-deterministic across machines. Because it only gates a local-display refresh (no sim
  state, no RNG), it does not threaten lockstep. A Rust port should keep this purely on the
  render/UI side and never feed it back into `sim/`.

## Active-in-YR / Tiberian Sun legacy

- **active_in_yr: YES (conditional on the display flag).** The tactical view tick runs every
  frame in a normal YR skirmish (the player's scrolling camera + radar). The only suppression
  is the internal `DAT_00a8d5f8 & 2` guard (load/serialize-style states), which is the normal
  by-design pause of the view, not a TS-legacy dead path.
- **ts_legacy: NO.** TacticalClass/DisplayClass is the live RA2/YR tactical view; nothing in
  this driver is gated behind a TS-only flag or otherwise dead in YR.

## Ghidra calls cited

- `disassemble_function 0x0055AFB0` — body site `0055b65f`–`0055b667`, unconditional dispatch.
- `read_memory 0x00887324` — g_Tactical null in static image.
- `get_xrefs_to 0x00887324` — construction site `006d1e1e in Tactical__Constructor [WRITE]`.
- `decompile_function 0x006d1e1e` — `*param_1 = &vtable__Tactical; ... g_Tactical = param_1;`.
- `read_memory 0x007f43a4` — vtable slot +0x5c = `0x006d2540`.
- `decompile_function 0x006d2540` — `TacticalClass__AI` body (gate, scroll lerp, radar timer).
- `get_function_callees 0x006d2540` — full callee set (4 functions, none RNG).
- `decompile_function 0x006d8640 / 0x006d8b30 / 0x0075f5c0 / 0x006c8c40` — callee bodies,
  confirming pure coordinate/viewport math + `timeGetTime` (no RNG).
- `get_xrefs_to 0x00a8d5f8` — confirms it's a main-loop mode/suppress flag.
