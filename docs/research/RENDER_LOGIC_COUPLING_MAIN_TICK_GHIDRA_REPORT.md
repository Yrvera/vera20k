# Render–Logic Coupling in Main_Tick — Ghidra Research Report

**Date:** 2026-05-28
**Address(es):** `Main_Tick @ 0x0055D360`, `RenderFrame_main @ 0x004F4480`, `FUN_0055E160 @ 0x0055E160`
**Confidence:** High for call site count, ordering, and branch liveness classification; High for no-interpolation verdict (no sub-frame position math found anywhere in the render path).
**Active in YR:** Yes for normal skirmish path; Conditional for scenario-delay and replay branches.

---

## Target Question

Is `RenderFrame_main` called exactly 1:1 per logic frame inside `Main_Tick`, or is rendering decoupled, interpolated, or run at a different cadence than the gameplay frame increment?

## Non-Goals

- Throttle/sleep budget math (slot 1)
- Game-speed setting source (slot 2)
- Timer frame-basis for CDTimer/RateTimer/AnimClass (slot 3)
- Frame increment guard-flag lifecycle (slot 5)
- Exhaustive audit of all `RenderFrame_main` callers outside `Main_Tick`

## Evidence Needed for COMPLETE

| Requirement | Status |
|---|---|
| Count every `RenderFrame_main` call site inside `Main_Tick` and classify each by branch condition | met |
| Establish ordering relative to `LogicClass__AI`, `Map__Logic`, `g_CurrentFrameCounter++`, `FUN_0055E160` | met |
| Confirm `RenderFrame_main` is the actual frame renderer (decompile) | met |
| Determine whether the render call uses interpolated sub-frame positions or committed logic state | met |
| Classify each call site: live in normal YR skirmish, replay-only, scenario-delay-only, or network-throttle-spin | met |

## Stop Conditions

Stop after classifying all three call sites in `Main_Tick` and the one inside `FUN_0055E160`. Do not audit callers in trigger actions, scenario setup, or map-editor code paths.

---

## 1. What is RenderFrame_main?

`RenderFrame_main @ 0x004F4480` is verified as the primary composite frame renderer.
Its decompile shows it:
1. Swaps `g_PrimarySurface` to the back buffer (`DAT_0088731C`).
2. Calls the display-chain blit at `*g_DisplayChain + 0x40`.
3. Calls `TacticalClass_Draw` three times (passes 0, 1, 2) — terrain + objects + overlay compositing.
4. Calls the main GScreen draw (`*param_1 + 0x40`) with a dirty flag.
5. Conditionally blits the sidebar surface if dirty (`DAT_00B0B519`).
6. Calls `FUN_005D49A0` (sound/audio service).
7. Optionally calls `House_AI_Tick()` again if `DAT_00A8B8B4` is set.
8. Calls `*g_DisplayChain + 0x3C` (flip/present).
9. Calls `*param_1 + 0x44` (GScreen finalize).

Evidence: `decompile_function 0x004F4480` — direct read of the function body.

This is unambiguously the full composite render frame, not a helper or stub.

---

## 2. Call Sites in Main_Tick

Three `RenderFrame_main` calls exist inside `Main_Tick @ 0x0055D360`.

### Call Site A — Scenario-Delay Early-Return Branch (0x0055D84F)

**Condition:** `g_GameMode == 0` or `5` AND `*(g_ScenarioClass_Instance + 0x62C) != 0`.
This is the scenario-delay / pre-game-start intro cinematic gate.

**Order within this branch:**
1. `Process_NetworkMessages()`
2. `Network_ServiceLoop()`
3. `Process_QueuedEvents()`
4. `(*g_Tactical + 0x5C)()` — tactical update/scroll
5. `RenderFrame_main()` ← call at `0x0055D84F`
6. `FUN_0055E160()` — throttle/sleep
7. `DAT_00ABCD58 = 0`
8. `return` — exits `Main_Tick` early

**Logic executed:** `LogicClass__AI` is NOT called. `Map__Logic` is NOT called. `g_CurrentFrameCounter` is NOT incremented.
**Active in YR:** Conditional — only fires while `ScenarioClass+0x62C != 0` (scenario delay countdown). This is a render-without-logic path. Zero gameplay advancement occurs.

Evidence: `decompile_function 0x0055D360` body; `get_assembly_context 0x0055D84F` confirming the CALL to `0x004F4480` followed immediately by CALL `0x0055E160` and function return.

### Call Site B — Normal Active Gameplay Block (0x0055D8F2)

**Condition:** `(DAT_00A8D5F8 & 2) == 0` AND `g_GameState == 0` AND `g_GameRunning != 0`.
This is the standard skirmish path.

**Order within this block:**
1. `GScreenClass__Input()`
2. `LogicClass__AI()`
3. optional `House_AI_Tick()` if `DAT_00A8B8B4 != 0`
4. optional `Network_Keepalive()` if `(g_CurrentFrameCounter & 7) == 7` and network mode
5. `Map__Logic()` at `0x0055D8E8`
6. `RenderFrame_main()` ← call at `0x0055D8F2`

After this block, execution continues to the desync/replay section, `FUN_00551A30`, `LogicClassPerTickUpdateLiveVector`, service work, `Network_ServiceLoop`, the four-gate `g_CurrentFrameCounter++` increment, then `FUN_0055E160`.

**Active in YR:** Yes — this is the primary skirmish path.
**Render fires exactly once per iteration of this block.** The block is not a loop; it executes top-to-bottom once per `Main_Tick` invocation. Therefore render fires exactly once per gameplay frame.

Evidence: `decompile_function 0x0055D360` — the active block is a straight-line if-body, not a loop; `get_assembly_context 0x0055D8F2` confirming CALL `0x004F4480` immediately after CALL `0x004D2370` (Map__Logic), then falling through to the post-render work.

### Call Site C — Replay / Desync Branch (0x0055DBBE)

**Condition:** `(DAT_00A8D5F8 & 2) != 0` (replay playback flag set).
This is inside the replay desync-check block that reads, checksums, and rehydrates object state from the replay stream.

**Order within this branch:**
`FUN_004F42F0(0)` (some pre-render flush) then `RenderFrame_main()` at `0x0055DBBE`.

**Active in YR:** Conditional — only when the replay flag bit 2 is set. Not active in a normal live skirmish.

Evidence: `decompile_function 0x0055D360` — the `if ((DAT_00A8D5F8 & 2) != 0)` block; `get_assembly_context 0x0055DBBE` (via context of `0x0055DBC3` showing CALL `0x004F4480` at `0x0055DBBE`).

---

## 3. Fourth Call Site — Inside FUN_0055E160 (Network Catch-Up Spin)

`FUN_0055E160 @ 0x0055E160` (the throttle/sleep helper) contains a **fourth** `RenderFrame_main` call deep inside a spin loop.

**Condition:** `g_GameMode != 0` AND `g_GameMode != 5` (network modes only, e.g. mode 4) AND `g_GameState == 0` AND `g_GameRunning == 1` AND remaining budget `>= 0x0B`.

In this spin loop the helper runs additional `GScreenClass__Input` + `LogicClass__AI` + tactical draw + `RenderFrame_main` iterations to catch up when the frame budget allows. This is a **network catch-up path** that runs extra logic+render pairs inside the throttle window.

**Active in YR:** Conditional — network multiplayer only (`g_GameMode == 4`). Not active in local skirmish (`g_GameMode == 5`).

Evidence: `decompile_function 0x0055E160` — the `if ((g_GameMode != 0) && (g_GameMode != 5))` spin loop body.

---

## 4. Ordering Relative to Key Landmarks

For the **normal active skirmish path** (Call Site B, the only live path in a standard local YR skirmish):

| Order | Operation | Evidence |
|---:|---|---|
| 1 | `GScreenClass__Input` | decompile `0x0055D360` active block |
| 2 | `LogicClass__AI` | decompile `0x0055D360` active block |
| 3 | optional `House_AI_Tick` | decompile `0x0055D360` active block |
| 4 | `Map__Logic` | call `0x004D2370` at `0x0055D8E8` |
| 5 | **`RenderFrame_main`** | call `0x004F4480` at `0x0055D8F2` |
| 6 | desync/replay section | `DAT_00A8D5F8` branch block |
| 7 | `FUN_00551A30` | call at `0x0055DBC8` |
| 8 | `LogicClassPerTickUpdateLiveVector` | decompile `0x0055D360` |
| 9 | service / tactical / accounting work | decompile `0x0055D360` |
| 10 | `Network_ServiceLoop` | call `0x0048D080` before increment |
| 11 | `g_CurrentFrameCounter++` (gated by 4 flags) | `0x0055DE73..0x0055DE81` |
| 12 | `FUN_0055E160` (throttle/sleep) | call `0x0055E160` at `0x0055DE9A` |

Active in YR: Yes for the full sequence above. Evidence: `decompile_function 0x0055D360`; `MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md` ordering table.

---

## 5. Interpolation Check

`RenderFrame_main` at `0x004F4480` renders from committed object positions. Its decompile shows no sub-frame lerp, no alpha-blend between previous and next positions, no separate "render position" accumulator. `TacticalClass_Draw` is called with the current committed logic state.

The `RateTimer` mechanism (`0x004C9220`, `0x004C93D0`) is used for **facing interpolation** — smoothly rotating a unit's visual facing between logic-frame commits — but this is a pure visual effect driven by frame-count deltas, not a positional interpolation between logic states. It does not represent decoupling of render position from logic position.

There is no separate render-loop thread, no fixed-step/variable-render split, no alpha for position between physics frames found anywhere in `RenderFrame_main` or its callees as examined here.

Evidence: `decompile_function 0x004F4480` — no sub-frame position math present; `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` section 2 (`RateTimer` fields).

---

## 6. Net Verdict

**LOCKED 1:1 render-per-logic-frame in normal local skirmish.**

In a standard YR skirmish (`g_GameMode == 5`), exactly one `RenderFrame_main` call fires per `Main_Tick` invocation (Call Site B), unconditionally within the normal gameplay block, after all logic (input → AI → map) and before service/network/frame-increment. There is no decoupling, no interpolation, no catch-up-render loop, and no separate render cadence. The throttle helper (`FUN_0055E160`) on this path performs only sleep/bucket-wait — it does not contain extra logic or render iterations for mode 5.

The scenario-delay branch (Call Site A) fires render without logic advancement — this is a known special case for pre-game countdowns, not a decoupled render mode.

The network catch-up spin (fourth call site inside `FUN_0055E160`) is mode-4 multiplayer only and explicitly guarded out for `g_GameMode == 5`.

**Interpolation verdict: NO.** Render reads the committed logic state once per frame. No sub-frame position lerp exists.

---

## 7. INI Keys

No INI key controls render cadence or render-logic coupling. `GameSpeed` controls throttle budget (slot 2 scope), not the render/logic relationship.

---

## 8. Implementation Handoff

### Handoff 1 — Render Must Be Locked 1:1 to Sim Frame

**Verified behavior:** In local skirmish, exactly one `RenderFrame_main` fires per `Main_Tick`, after all logic, before frame increment. Active in YR: Yes.
Evidence: `decompile_function 0x0055D360` Call Site B; `get_assembly_context 0x0055D8F2`.
**Current Rust delta:** App renders separately from the fixed-step sim loop; render cadence is driven by the wgpu/winit event loop, which is not locked to sim ticks.
**Affected Rust surface:** `src/app_sim_tick.rs` render call site; `src/app_building_anim.rs` and all render/overlay dispatch.
**Required effect:** Issue one render per sim fixed-step, after all sim subsystems complete and before the frame counter commits. The render call should consume the post-logic, pre-increment state.
**Acceptance scenario:** In a paused or single-stepping test harness, advancing one sim tick produces exactly one rendered frame with state matching the tick's output, not a stale prior state or an interpolated intermediate.
**Test name:** `test_render_locked_one_to_one_with_sim_frame`
**Risk:** Do not render asynchronously from a separate OS thread and expect parity — any render that reads state mid-logic produces tearing of committed object positions.

### Handoff 2 — No Position Interpolation Between Logic Frames

**Verified behavior:** `RenderFrame_main` reads committed object positions from the logic state without any sub-frame lerp. Active in YR: Yes.
Evidence: `decompile_function 0x004F4480` — no interpolation math.
**Current Rust delta:** Unknown whether any Rust render path applies `elapsed_since_last_sim_tick` fractional interpolation to unit positions.
**Affected Rust surface:** Any render system that computes `render_position = prev_pos + (next_pos - prev_pos) * alpha` where `alpha = elapsed / tick_ms`.
**Required effect:** Remove sub-frame position interpolation if present. Render the committed logic position directly. (Facing visual smoothing via `RateTimer` equivalent is acceptable — it is a pure cosmetic effect on the visual angle, not on the world-space position.)
**Acceptance scenario:** A unit at cell (10, 10) that moves to (11, 10) in one tick renders at (11, 10) immediately after that tick, not at a fractional position during the throttle sleep.
**Test name:** `test_render_uses_committed_logic_position_no_interpolation`
**Risk:** Do not conflate `RateTimer`-based facing visual smoothing (acceptable) with positional lerp (not in gamemd). Only facing rotation is smoothed.

### Handoff 3 — Scenario-Delay Path Renders Without Advancing Logic

**Verified behavior:** When `ScenarioClass+0x62C != 0`, `Main_Tick` renders and throttles but does NOT call `LogicClass__AI`, `Map__Logic`, or increment `g_CurrentFrameCounter`. Active in YR: Conditional (scenario start delay).
Evidence: `decompile_function 0x0055D360` Call Site A + early return; `get_assembly_context 0x0055D84F`.
**Current Rust delta:** Unknown whether Rust has an equivalent render-without-logic path for map-loading/pre-game display.
**Affected Rust surface:** `src/app_sim_tick.rs` scenario state; any app-level "waiting for game start" path.
**Required effect:** While scenario delay is active, issue render ticks at the throttle cadence without advancing simulation state.
**Acceptance scenario:** During the pre-game countdown the map renders but no unit movement, production, or frame counters advance.
**Test name:** `test_scenario_delay_renders_without_logic_advance`
**Risk:** Do not skip rendering entirely during scenario delay — the original renders continuously to show the loading/countdown state.

---

## 9. Negative Facts / Do Not Do

1. **Do not add sub-frame position interpolation.** gamemd renders committed positions once per frame with no lerp. Evidence: `decompile_function 0x004F4480` — no interpolation math present.
2. **Do not decouple render from sim tick rate.** gamemd runs render inline in the same function as logic, once per frame, on the same thread. Evidence: Call Site B in `decompile_function 0x0055D360`.
3. **Do not allow the throttle/sleep to issue extra render calls in local skirmish.** `FUN_0055E160`'s extra-render spin is guarded by `g_GameMode != 0 && g_GameMode != 5`. For mode 5 (local skirmish) the helper only sleeps. Evidence: `decompile_function 0x0055E160`.
4. **Do not count the scenario-delay render (Call Site A) as a normal logic frame.** It fires without logic advancement and returns early. Evidence: `decompile_function 0x0055D360` early-return structure.
5. **Do not count the replay render (Call Site C) as relevant for live skirmish.** It is behind the `DAT_00A8D5F8 & 2` replay flag. Evidence: `decompile_function 0x0055D360` replay block.

---

## 10. Remaining Uncertainty

- The internal `TacticalClass_Draw` passes (0, 1, 2) are not fully decompiled here; the three-pass split is verified by call count and argument sequence but the pass-2 contents (e.g. whether it reads a cached vs freshly computed render list) are out of scope for this coupling question.
- `FUN_005D49A0` (called inside `RenderFrame_main`) has not been decompiled; classified as audio/sound service based on position in the call sequence. Does not affect the coupling verdict.
- Exact semantics of `ScenarioClass+0x62C` (the scenario-delay field) lifecycle are out of scope; liveness of Call Site A is documented as conditional.

---

## 11. Stale-Doc Notes

`GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` section 3.1 lists the tick ordering correctly (steps 1–12) but does not state an explicit render-coupling verdict. The ordering table is accurate and is not stale. **No correction needed; this report adds the missing explicit verdict.**

`MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md` section 3.1 step 7 states `RenderFrame_main` runs before `PerTickUpdate`. This is verified and not stale. **No correction needed.**

Neither prior doc claims interpolation is present or absent; the interpolation-absent finding is new here.

---

## Sources

- `decompile_function 0x0055D360` — `Main_Tick` full decompile
- `decompile_function 0x004F4480` — `RenderFrame_main` full decompile
- `decompile_function 0x0055E160` — `FUN_0055E160` throttle helper full decompile
- `get_assembly_context 0x0055D84F` — scenario-delay call site
- `get_assembly_context 0x0055D8F2` — normal gameplay call site
- `get_assembly_context 0x0055DBC3` — context confirming replay-branch call at `0x0055DBBE`
- `get_function_callers 0x004F4480` — full caller list
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md`
