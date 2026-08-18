# Main_Tick Render–Logic Coupling — Ghidra Research Report

**Date:** 2026-05-28
**Addresses:** `Main_Tick @ 0x0055D360`, `RenderFrame_main @ 0x004F4480`,
`TacticalClass_Draw @ 0x006D3D10`, `FUN_0055E160 @ 0x0055E160`
**Confidence:** HIGH — all findings verified via live `decompile_function` calls this session.
**Active in YR:** Yes (standard skirmish path); Conditional (scenario-delay, replay, MP catch-up).

> **SUPERSEDES / EXTENDS:** `RENDER_LOGIC_COUPLING_MAIN_TICK_GHIDRA_REPORT.md` (prior slot-4
> report, same swarm). That doc contains the same verified conclusions. This report is the
> formally scoped slot-4 product for the current swarm batch. All findings are consistent;
> see §11 for stale-doc notes.

---

## Investigation Contract

**Target question:** Is render 1:1 per logic frame (option A), or decoupled/interpolated (option B)?

**Non-goals:** throttle sleep math; game-speed INI mapping; CDTimer/RateTimer internals; PerTickUpdate
callee audit; network-lag desync details.

**Evidence needed to mark COMPLETE:**

| Requirement | Status |
|---|---|
| Count every `RenderFrame_main` call in `Main_Tick`; classify each branch | met |
| Confirm single-call, no inner render loop, on normal skirmish path | met |
| Confirm render BEFORE `g_CurrentFrameCounter++` | met |
| Confirm `FUN_0055E160` is the only wait/throttle; confirm its extra-render spin is MP-only | met |
| Confirm `TacticalClass_Draw` reads discrete committed positions — no sub-frame lerp | met |
| Confirm no separate render thread | met |

**Stop conditions:** Stop after classifying the three `Main_Tick` call sites and the one in
`FUN_0055E160`. Do not audit further callers outside `Main_Tick`.

---

## 1. Overview

**Verdict: Option A — LOCKED 1:1 in local skirmish.** In a standard YR local skirmish
(`g_GameMode == 5`), exactly one `RenderFrame_main` call fires per `Main_Tick` invocation,
after all logic and before the frame-counter increment. There is no decoupling, no sub-frame
position interpolation, and no separate render thread. The throttle helper (`FUN_0055E160`)
on the mode-5 path performs only sleep/bucket-wait — its extra-render spin is gated to
network MP (`g_GameMode == 4`) only.

---

## 2. Call Sites in Main_Tick

Three `RenderFrame_main` calls exist in `Main_Tick @ 0x0055D360`
(verified via `decompile_function 0x0055D360`).

### Call Site A — Scenario-Delay Early-Return

**Condition:** `g_GameMode == 0 || 5` AND `*(g_ScenarioClass_Instance + 0x62C) != 0`
(pre-game cinematic / countdown gate).

Sequence: `Process_NetworkMessages` → `Network_ServiceLoop` → `Process_QueuedEvents` →
`(*g_Tactical + 0x5C)()` → **`RenderFrame_main()`** → `FUN_0055E160` → early `return`.

`LogicClass__AI` is NOT called. `Map__Logic` is NOT called. `g_CurrentFrameCounter` is
NOT incremented. This is a render-without-logic path.
Active in YR: **Conditional** (scenario intro delay only).
Evidence: `decompile_function 0x0055D360` — `LAB_0055d821` branch body and early `return`.

### Call Site B — Normal Active Gameplay Block (SKIRMISH PATH)

**Condition:** `(DAT_00A8D5F8 & 2) == 0` AND `g_GameState == 0` AND `g_GameRunning != 0`.
This is the standard skirmish path.

Sequence inside the block:
1. `GScreenClass__Input()`
2. `LogicClass__AI()`
3. optional `House_AI_Tick()`
4. optional `Network_Keepalive()`
5. `Map__Logic()`
6. **`RenderFrame_main()`**

The block is a straight-line `if`-body, not a loop. Render fires exactly once per
`Main_Tick` invocation on this path. After the block: desync/replay section →
`FUN_00551A30` → `LogicClassPerTickUpdateLiveVector` → service work →
`Network_ServiceLoop` → **`g_CurrentFrameCounter++`** (gated by four flags) →
**`FUN_0055E160`** (throttle).

Render is thus AFTER all logic and BEFORE the frame increment.
Active in YR: **Yes**.
Evidence: `decompile_function 0x0055D360` active block; inline order confirmed.

### Call Site C — Replay / Desync Branch

**Condition:** `(DAT_00A8D5F8 & 2) != 0` (replay playback flag).

Inside replay desync-check block: `FUN_004F42F0(0)` then **`RenderFrame_main()`**.
Active in YR: **Conditional** (replay mode only, not live skirmish).
Evidence: `decompile_function 0x0055D360` — `if ((DAT_00A8D5F8 & 2) != 0)` block.

---

## 3. Fourth Call Site — FUN_0055E160 Network Catch-Up Spin

`FUN_0055E160 @ 0x0055E160` contains a fourth `RenderFrame_main` call inside a spin loop.
(verified via `decompile_function 0x0055E160`).

**Entry gate:** `g_GameMode != 0 && g_GameMode != 5`.
This is network MP only (`g_GameMode == 4`). For local skirmish (`g_GameMode == 5`) the
helper only sleeps/waits — the spin loop is NOT entered.

Inside the spin: `Network_ServiceLoop` → check budget remaining `>= 0x0B` → if time
remains AND `g_GameState == 0` AND `g_GameRunning == 1`: `GScreenClass__Input` +
`LogicClass__AI` + `(*g_Tactical + 0x5C)()` + **`RenderFrame_main()`**.

This is a network catch-up path that runs additional logic+render pairs inside the
throttle window when the frame budget allows. It does not affect local skirmish.

Active in YR: **Conditional** (network MP `g_GameMode == 4` only).
Evidence: `decompile_function 0x0055E160` — `if ((g_GameMode != 0) && (g_GameMode != 5))` block.

---

## 4. Ordering Table — Normal Skirmish

| # | Operation | Before/After Render | Evidence |
|---|---|---|---|
| 1 | `GScreenClass__Input` | Before | `decompile_function 0x0055D360` |
| 2 | `LogicClass__AI` | Before | `decompile_function 0x0055D360` |
| 3 | `House_AI_Tick` (conditional) | Before | `decompile_function 0x0055D360` |
| 4 | `Map__Logic` | Before | `decompile_function 0x0055D360` |
| **5** | **`RenderFrame_main`** | **—** | `decompile_function 0x0055D360` |
| 6 | `LogicClassPerTickUpdateLiveVector` | After | `decompile_function 0x0055D360` |
| 7 | `Network_ServiceLoop` | After | `decompile_function 0x0055D360` |
| 8 | `g_CurrentFrameCounter++` (gated) | After | `decompile_function 0x0055D360` |
| 9 | `FUN_0055E160` throttle/sleep | After increment | `decompile_function 0x0055D360` |

---

## 5. No Sub-Frame Interpolation

`TacticalClass_Draw @ 0x006D3D10` was fully decompiled this session
(verified via `decompile_function 0x006D3D10`).

Findings:
- Positions read are integer cell/pixel values from `TacticalClass` fields (`+0xB0..+0xBC`
  viewport corners, `+0xD64/+0xD68` current viewport).
- No `alpha`, `lerp`, or fractional accumulator between previous and current frame position
  found anywhere in the function body.
- The `ScrollSpeed` / `ScrollProgress` (`+0xD8` / `+0xDC` float fields) in `TacticalClass`
  are **camera pan interpolation**, not object-position interpolation. They move the
  viewport smoothly between two committed viewport positions; they do not produce
  sub-frame object positions.
- The `RateTimer` (`FacingClass::Set @ 0x004C9220`) that drives facing visual smoothing
  is a pure visual-angle effect; it does not lerp world-space positions.
- `Tactical_ObjectRenderingLoop()` is called in pass 2 with no fractional time argument.
  All objects draw at their committed logic-state positions.

**Interpolation verdict: NO.** Render reads committed discrete integer state.
Active in YR: Yes.
Evidence: `decompile_function 0x006D3D10`; `decompile_function 0x004F4480`.

---

## 6. No Separate Render Thread

- `Main_Tick` is a single-threaded function. `RenderFrame_main` is an inline call in the
  same call stack. No thread creation, semaphore, or shared buffer swap indicating a
  parallel render thread appears anywhere in the decompiled bodies.
- `FUN_0055E160` is a single-threaded spin/sleep loop.
- The engine is single-threaded: the entire input → logic → render cycle runs on one OS
  thread per `Main_Tick` call.

Active in YR: Yes.
Evidence: `decompile_function 0x0055D360`; `decompile_function 0x0055E160` — no thread API calls.

---

## 7. INI Keys

No INI key controls render-logic coupling or interpolation. `GameSpeed` controls throttle
budget only. Evidence: `decompile_function 0x0055D360`; `GAME_SPEED_SETTING_RATE_VS_CONTENT_GHIDRA_REPORT.md`.

---

## 8. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|
| Exactly one `RenderFrame_main` per `Main_Tick` on skirmish path; render after all logic, before frame-counter increment; no interpolation (verified: `decompile_function 0x0055D360`, `0x006D3D10`) | Rust runs a separate wgpu/winit render loop decoupled from fixed sim steps; some paths may interpolate render position by elapsed-since-last-sim-tick | `src/app_sim_tick.rs` render dispatch; any render path that accepts `elapsed_frac` | Advancing one sim tick in a headless test produces exactly one render call reading the post-logic committed state | `test_render_locked_one_to_one_with_sim_frame` | Do not render from a separate OS thread; do not apply positional lerp for parity |
| No sub-frame position interpolation — `TacticalClass_Draw` reads discrete integer positions (verified: `decompile_function 0x006D3D10`) | Unknown whether Rust render applies `prev_pos + (next_pos - prev_pos) * alpha` fractional interpolation | Any Rust render helper that computes interpolated world-space position | Unit at cell (10,10) that moves to (11,10) in one tick renders at (11,10) immediately, not at a fractional position during sleep | `test_render_uses_committed_logic_position_no_interpolation` | Facing visual smoothing via `RateTimer` equivalent is acceptable; world-space positional lerp is not |
| Scenario-delay path (`ScenarioClass+0x62C != 0`) renders once per `Main_Tick` without advancing logic or frame counter (verified: `decompile_function 0x0055D360` Call Site A early-return) | Unknown whether Rust has an equivalent render-without-logic path for map load / pre-game display | `src/app_sim_tick.rs` scenario-state gate | During pre-game countdown the map renders continuously but no unit movement or frame counters advance | `test_scenario_delay_renders_without_logic_advance` | Do not skip rendering entirely during scenario delay |

---

## 9. Negative Facts / Do Not Do

1. **Do not add sub-frame position interpolation.** gamemd renders committed discrete positions
   once per frame with no positional lerp. Evidence: `decompile_function 0x006D3D10` and
   `decompile_function 0x004F4480` — no interpolation math present.
2. **Do not decouple render from sim tick rate in local skirmish.** Native runs render inline
   with logic on the same thread once per `Main_Tick`. Evidence: `decompile_function 0x0055D360`
   Call Site B straight-line if-body.
3. **Do not allow the throttle helper to issue extra renders in local skirmish.** `FUN_0055E160`'s
   extra-render spin is guarded `g_GameMode != 0 && g_GameMode != 5` — explicitly excludes
   mode 5. Evidence: `decompile_function 0x0055E160`.
4. **Do not count the scenario-delay render as a logic frame.** Call Site A fires without logic
   advancement and returns early. Evidence: `decompile_function 0x0055D360` LAB_0055d821.
5. **Do not count the replay-branch render (Call Site C) as relevant for live skirmish.**
   It is behind the `DAT_00A8D5F8 & 2` replay flag. Evidence: `decompile_function 0x0055D360`.

---

## 10. Remaining Uncertainty

- The `TacticalClass_Draw` pass-2 `Tactical_ObjectRenderingLoop()` callee was not
  further decompiled. Its interface takes no fractional-time argument and calls no
  lerp helper visible at this level, but deep internals of individual object draw calls
  were not audited. Risk is LOW — no argument path carries a sub-frame alpha.
- `FUN_005D49A0` (audio service inside `RenderFrame_main`) was not decompiled. Classified
  as audio-only based on position; does not affect the coupling verdict.
- Exact lifecycle of `ScenarioClass+0x62C` (scenario-delay countdown) is not audited;
  liveness of Call Site A is documented as conditional.

---

## 11. Stale-Doc Notes

`RENDER_LOGIC_COUPLING_MAIN_TICK_GHIDRA_REPORT.md` (prior slot-4 product) contains identical
verified conclusions. **Not stale; no correction needed.** This report is the formally-scoped
product of the current swarm batch; both documents are consistent and may be cross-referenced.

`logic-vs-render-loop.md` § "Render-only frame paths" table correctly lists the scenario-delay
path at `LAB_0055d821` but labels the offset as `g_ScenarioClass_Instance[0x18B]`. The live
decompile shows the gate is `*(int*)(g_ScenarioClass_Instance + 0x62C) != 0` (an int
field, not byte `0x18B`). The `0x18B` reference likely names a different scenario flag
visible earlier in `Main_Tick`. **Minor stale claim in `logic-vs-render-loop.md`: the
scenario-delay render-only gate is `+0x62C` (int), not `[0x18B]` (byte).** The behavioral
description remains accurate.

---

## Sources

- `decompile_function 0x0055D360` — `Main_Tick` full decompile (this session)
- `decompile_function 0x004F4480` — `RenderFrame_main` full decompile (anchor doc + this session)
- `decompile_function 0x006D3D10` — `TacticalClass_Draw` full decompile (this session)
- `decompile_function 0x0055E160` — throttle helper full decompile (this session)
- `docs/research/RENDER_LOGIC_COUPLING_MAIN_TICK_GHIDRA_REPORT.md` — prior slot-4 report
- `docs/research/GSCREEN_RTACTICAL_GHIDRA_REPORT.md` §6 — `RenderFrame_main` body
- `docs/research/timing/logic-vs-render-loop.md` — tick topology and frame paths
- `docs/research/MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md` §3.1 — ordering table
