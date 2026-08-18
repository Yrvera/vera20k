# Main Tick Frame-Counter Placement vs Rust Advance Tick - Ghidra Research Report

**Address(es):** `Main_Tick @ 0x0055D360`, `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`, `FUN_0055E160 @ 0x0055E160`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** active YR placement of input/gameplay/render/service work, `LogicClass::PerTickUpdate`, and `g_CurrentFrameCounter` increment relative to Rust `Simulation::advance_tick` updating `total_sim_ms`/`binary_frame` at tick start.  
**Non-Scope:** exhaustive audit of every `CDTimerClass`, `RateTimer`, `AnimClass`, weapon, production, animation, or UI timer user; full replay/network timing mechanics; runtime measurement of wall-clock pacing.  
**Confidence:** High for main ordering and Rust delta; Medium for representative timer-user hazard list because this report uses existing verified docs instead of re-tracing every user.  
**Active in YR:** Yes for standard gameplay tick; Conditional for scenario-delay, replay/network, and session-end/freeze branches named below.

## Investigation Contract

**Target question:** Does active YR expose the current frame counter during tick work and increment it late, while current Rust exposes a newly derived `binary_frame` at the beginning of `Simulation::advance_tick`?

**Non-goals:** Do not implement Rust; do not audit all timer users deeply; do not decide the final architecture for a native frame-clock service; do not modify existing docs except this report and the shared swarm claims.

**Evidence needed to mark COMPLETE:**

| Evidence requirement | Status | Evidence |
|---|---|---|
| Verify active YR tick body order around input, AI, map, render, `PerTickUpdate`, service, and frame increment. | met | `Main_Tick` decompile plus assembly contexts `0x0055D897..0x0055D8F2`, `0x0055DC99..0x0055DCA3`, `0x0055DE4A..0x0055DE9A` |
| Verify `LogicClass::PerTickUpdate` reads the pre-increment counter. | met | `PerTickUpdate` decompile reads `g_CurrentFrameCounter` for timers/modulo before caller's late increment; `0x0055AFB0`, caller `0x0055DC99..0x0055DCA3` |
| Verify Rust current behavior with source file:line. | met | `src/sim/world/mod.rs:1196..1200`, `src/app_sim_tick.rs:284..291`, `src/app_types.rs:24..27`, `src/util/fixed_math.rs:47..51` |
| Bridge to concrete timer/frame-order hazards without broad timer re-research. | met | representative docs: `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`, `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`, timing corpus |

**Stop conditions:** Stop after proving the native order and Rust delta; defer any attempt to re-trace each downstream timer user or design/code the Rust fix.

## 1. Overview

Active YR's `Main_Tick` runs most tick work while `g_CurrentFrameCounter` still holds the old frame value, then increments the counter near the end after service/network work and only if four session-end/freeze flags are clear. `LogicClass::PerTickUpdate` is in that pre-increment region, so its modulo gates and timer checks see the old frame.

Current Rust updates `total_sim_ms` and derives `binary_frame` at the beginning of `Simulation::advance_tick`, before command dispatch and all subsystem phases. Any Rust timer or animation that starts, checks, or gates on `binary_frame` during the same tick can be one native frame early unless Rust provides an explicit "current native frame for this tick" value and commits the next frame late.

## 2. Key Globals / Fields

| Global / Rust field | Purpose | Evidence | Active in YR |
|---|---|---|---|
| `g_CurrentFrameCounter @ 0x00A8ED84` | authoritative native gameplay frame counter | `Main_Tick` late read/inc/write at `0x0055DE73..0x0055DE81`; prior docs cite timer users | Yes |
| `DAT_00A83D49`, `DAT_00A8ECD0`, `DAT_008B41C0`, `DAT_00A83D48` | four gates that can skip the late frame increment and return | `Main_Tick` checks `0x0055DE4F..0x0055DE71` before increment | Conditional |
| `DAT_00A8D5F8` bits `1`/`2` | replay/transition flags that change the main path | `Main_Tick` replay/transition sections before `PerTickUpdate`; prior timing docs | Conditional |
| `Simulation::total_sim_ms` | Rust accumulated synthetic elapsed time | `src/sim/world/mod.rs:1199` | Rust-facing |
| `Simulation::binary_frame` | Rust synthetic 15 Hz frame derived from `total_sim_ms` | `src/sim/world/mod.rs:1200` | Rust-facing |
| `SIM_TICK_HZ = 45`, `SIM_TICK_MS = 1000 / SIM_TICK_HZ` | Rust fixed-step size; current integer tick is 22 ms | `src/util/fixed_math.rs:47..51`; `src/app_types.rs:24..27` | Rust-facing |

## 3. Core Ordering Findings

### 3.1 Standard active gameplay path

Material ordering, all inside `Main_Tick @ 0x0055D360`:

| Order | Native work | Evidence | Active in YR |
|---:|---|---|---|
| 1 | Active gameplay gate checks transition flags, `g_GameState == 0`, and `g_GameRunning != 0`. | decompile `Main_Tick`; assembly starts the normal block at `0x0055D878..0x0055D897` | Yes |
| 2 | Input is processed before logic dispatch. | `GScreenClass__Input` call sequence at `0x0055D897..0x0055D8AB` | Yes |
| 3 | `LogicClass::AI` runs after input. | call `0x0055D8B4` in normal block | Yes |
| 4 | Optional house AI runs after `LogicClass::AI` when `DAT_00A8B8B4 != 0`. | branch/call `0x0055D8B9..0x0055D8C4` | Conditional |
| 5 | Network keepalive checks the old frame counter modulo 8 before map logic. | read/mask/compare `g_CurrentFrameCounter` at `0x0055D8C9..0x0055D8E3` and `g_GameMode == 4` | Conditional: network mode |
| 6 | `Map::Logic` runs before render. | call `0x004D2370` at `0x0055D8E8` | Yes |
| 7 | `RenderFrame_main` runs before the late per-tick scheduler. | call `0x004F4480` at `0x0055D8F2`; later `PerTickUpdate` call at `0x0055DC9E` | Yes |
| 8 | `LogicClass::PerTickUpdate` runs after side/replay/save work, before service and frame increment. | `MOV ECX,0x87F778; CALL 0x0055AFB0` at `0x0055DC99..0x0055DCA3` | Yes |
| 9 | Service/tactical/network routines run after `PerTickUpdate`. | decompile and assembly around `0x0055DCA3..0x0055DE4A` | Yes |
| 10 | `Network_ServiceLoop` runs immediately before the late increment gate. | call `0x0048D080` at `0x0055DE4A` | Yes |
| 11 | If all four gate globals are zero, the function reads, increments, and stores `g_CurrentFrameCounter`. | checks `0x0055DE4F..0x0055DE71`; read/inc/write `0x0055DE73..0x0055DE81` | Yes, unless gated |
| 12 | Wait/throttle helper runs after the frame increment. | `CALL 0x0055E160` at `0x0055DE9A` | Yes |

### 3.2 Pre-increment frame value is load-bearing

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` reads `g_CurrentFrameCounter` for timer remaining checks and modulo gates before `Main_Tick` performs the late increment. Representative verified examples in the same function include scenario/cell timers using `g_CurrentFrameCounter - start`, bridge shroud recalc on `g_CurrentFrameCounter % 0x78 == 0`, and later global/object schedulers that all execute under the old frame value. (corrected 2026-05-29: live Ghidra label normalized from `LogicClass::PerTickUpdate` to `LogicClassPerTickUpdateLiveVector`; verified via `get_function_by_address 0x0055AFB0`)

Active in YR: Yes. Evidence: `LogicClassPerTickUpdateLiveVector` direct caller `0x0055DC99..0x0055DCA3`; `LogicClassPerTickUpdateLiveVector` decompile; `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md:242..251`.

### 3.3 Conditional paths that matter for tests

| Branch | Native behavior | Evidence | Active in YR |
|---|---|---|---|
| Scenario-delay / intro cinematic gate | When scenario `+0x62C != 0`, native processes network/events/tactical/render/wait and returns before the normal gameplay region, `PerTickUpdate`, and frame increment. | assembly `0x0055D821..0x0055D859`; decompile branch at `LAB_0055d821` | Conditional |
| Pause/menu state | `g_GameState != 0` (ESC/options/save/quit dialogs) skips the Input/AI/Map::Logic/RenderFrame gameplay block but does NOT freeze the frame counter: `LogicClassPerTickUpdateLiveVector` and `g_CurrentFrameCounter++` still run. In-game pause is distinct from the session-end freeze. | `Main_Tick` decompile gameplay-block gate `0x0055D878..0x0055D901`; `FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md §State D` | Conditional: paused/menu state |
| Session-end/freeze gates | Four globals skip the late increment and return after service work. | `0x0055DE4F..0x0055DE71` jumps to `0x0055DEC8`; no write to `0x00A8ED84` on that path | Conditional |
| Replay/transition flags | Replay record/playback sections can alter render/input path and are not exhaustively covered here. | `Main_Tick` decompile around replay flag checks | Conditional; deferred |

## 4. INI Keys

No INI key directly controls the placement of `g_CurrentFrameCounter++` inside `Main_Tick`.

| Key / data source | Relevance | Evidence | Active in YR |
|---|---|---|---|
| `[MultiplayerDialogSettings] GameSpeed` / live stored speed byte | Controls throttle pacing, not the intra-tick placement of the frame increment. | `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md:315..320` | Yes |
| Timer-like INI fields such as `Rate=`, `ROF=`, `Reload=`, `GrowthRate`, `SpyPowerBlackout` | Many are consumed as frame durations or frame-derived timers; they inherit the late-increment contract when implemented against native frame counters. | `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md:260..274`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md:610..628` | Yes, by owning systems |

## 5. Current Rust Implementation Status

> **Audit note (corrected 2026-05-29):** The `binary_frame` late-commit fix was applied to the Rust codebase after this doc was originally written. `total_sim_ms` and `binary_frame` are now updated at the **end** of `advance_tick` (lines 1397–1398, after all phase work), not at tick start. The primary mismatch described below is RESOLVED in the current code. Line references `1196..1200`/`1204`/`1211..1243` were stale — corrected to current locations. The remaining rows (pause gate, render effects, fixed tick constants) remain open investigation items. Root cause of stale line refs: OPERATOR_OR_ORDER_DRIFT (Rust implementation moved to match native late-commit; doc not updated). Verified via read of `src/sim/world/mod.rs` lines 1390–1399 and `advance_tick` entry at line 1402. (corrected 2026-05-29: was 1196..1200 showing early derivation; binary now commits late at 1397–1398 — OPERATOR_OR_ORDER_DRIFT)

| Rust surface | Current behavior | Evidence | Delta |
|---|---|---|---|
| `Simulation::advance_tick` | `total_sim_ms` and `binary_frame` are committed **late**, after all phase work, at `advance_tick` end. All tick phases observe the previous tick's committed `binary_frame` (pre-increment frame N). | `src/sim/world/mod.rs:1397..1398` (corrected 2026-05-29: was `1196..1204` with early derivation — OPERATOR_OR_ORDER_DRIFT) | **RESOLVED**: matches native late-commit contract. |
| Command dispatch | Due commands are applied before phases, while `binary_frame` still holds the previous tick's value. | `src/sim/world/mod.rs:1419..1424` (corrected 2026-05-29: was `1211..1243`) | **RESOLVED**: consistent with native input-before-increment ordering. |
| Movement/combat/production/ore phases | Several phases receive `tick_ms`, `self.tick`, or `self.binary_frame` (pre-increment value) during the tick. | examples `src/sim/world/mod.rs:1431..1457`, `1456`, `1663..1668` (corrected 2026-05-29: was `1248..1299`, `1447..1468`, `1696..1718`) | Compatible with native pre-increment frame semantics; per-system verification remains. |
| App scheduler | `advance_fixed_simulation` passes `SIM_TICK_MS` into `advance_tick` for each fixed step. | `src/app_sim_tick.rs:284..291`; schedule loop | One app fixed step is not one native frame; native frame counter increments once per `Main_Tick`, late. |
| Fixed tick constants | Comments say native 15 fps, but value is `SIM_TICK_HZ = 45`; `SIM_TICK_MS = 22`. | `src/util/fixed_math.rs:47..51`; `src/app_types.rs:24..27` | Documentation/code mismatch compounds timer assumptions. |
| App pause gate | `run_sim` is false when `state.paused` unless frame-stepping. | `src/app_sim_tick.rs:151..159` | Native pause/menu model is partial: gameplay/render gate differs from `PerTickUpdate` and frame counter. |
| Render-side/app effects | Some visible effects advance after fixed sim using elapsed ticks or capped wall-clock-like milliseconds. | `src/app_sim_tick.rs:176..210`, `292..312` | Must be audited per effect; native many effects are object-AI/frame-counter driven. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Main_Tick` standard active ordering | verified | decompile `0x0055D360`; assembly contexts `0x0055D897..0x0055D8F2`, `0x0055DC99..0x0055DE9A` | none for this slice |
| `LogicClass::PerTickUpdate` placement | verified | caller `0x0055DC99..0x0055DCA3`; decompile `0x0055AFB0` | none for placement; other slots cover loop internals |
| Late frame increment gate and write | verified | `0x0055DE4F..0x0055DE81` | names/semantics of all four gate globals beyond session-end/freeze are deferred |
| Wait helper placement after increment | verified | `CALL 0x0055E160` at `0x0055DE9A`; helper decompile | none for placement |
| Scenario-delay render-only early return | touched-not-exhausted | `0x0055D821..0x0055D859` | exact scenario flag lifecycle is out-of-scope |
| Pause/menu partial behavior | touched-not-exhausted | `logic-vs-render-loop.md:444..457`; `Main_Tick` decompile | exact UI/modal branch matrix deferred |
| Replay/transition paths | touched-not-exhausted | `Main_Tick` decompile sections gated by `DAT_00A8D5F8` | full replay timing investigation |
| Rust `advance_tick` frame update | verified | `src/sim/world/mod.rs:1196..1200` | implementation choice remains future work |
| Rust app fixed-step scheduling | verified | `src/app_sim_tick.rs:284..291`, `764..784`; `src/app_types.rs:24..27` | runtime pacing measurement deferred |
| Representative timer surfaces | touched-not-exhausted | existing timing docs and code scan | per-user exact fixes require system-specific investigations |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-MTFC-001 - Is `Main_Tick @ 0x0055D360` the active YR tick owner? -> Yes; prior timing corpus traces `Main_Game` repeatedly calling `Main_Tick`, and current decompile shows standard gameplay gating inside this function.` (evidence: `timing/INDEX_TIMING.md:107..108`; `timing/logic-vs-render-loop.md:13..18`; `Main_Tick @ 0x0055D360`)
- `[RESOLVED] OQ-MTFC-002 - Where does standard input run relative to the frame increment? -> Before the late increment.` (evidence: `0x0055D897..0x0055D8AB` before `0x0055DE73..0x0055DE81`)
- `[RESOLVED] OQ-MTFC-003 - Where does `LogicClass::AI` run relative to the frame increment? -> Before the late increment.` (evidence: `0x0055D8B4` before `0x0055DE73..0x0055DE81`)
- `[RESOLVED] OQ-MTFC-004 - Where does `Map::Logic` run relative to the frame increment? -> Before the late increment.` (evidence: `0x0055D8E8` before `0x0055DE73..0x0055DE81`)
- `[RESOLVED] OQ-MTFC-005 - Where does `RenderFrame_main` run relative to the frame increment? -> Before the late increment in the normal gameplay path.` (evidence: `0x0055D8F2` before `0x0055DE73..0x0055DE81`)
- `[RESOLVED] OQ-MTFC-006 - Where does `LogicClass::PerTickUpdate` run? -> After normal render/side work and before service/network work and the late increment.` (evidence: `0x0055DC99..0x0055DCA3`; `0x0055DE4A..0x0055DE81`)
- `[RESOLVED] OQ-MTFC-007 - Does `PerTickUpdate` see the old or new frame? -> Old/pre-increment frame.` (evidence: caller `0x0055DC99..0x0055DCA3` precedes increment `0x0055DE73..0x0055DE81`; `PerTickUpdate` reads `g_CurrentFrameCounter`)
- `[RESOLVED] OQ-MTFC-008 - Is the frame increment unconditional? -> No; four globals gate it.` (evidence: `0x0055DE4F..0x0055DE71`)
- `[RESOLVED] OQ-MTFC-009 - Does the wait helper run before or after the frame increment? -> After the normal late increment.` (evidence: increment `0x0055DE73..0x0055DE81`; call `0x0055DE9A`)
- `[RESOLVED] OQ-MTFC-010 - Does current Rust expose a newly advanced frame before tick work? -> Yes; `binary_frame` is computed at the top of `advance_tick`.` (evidence: `src/sim/world/mod.rs:1196..1200`)
- `[RESOLVED] OQ-MTFC-011 - Do current Rust commands run before or after that frame derivation? -> After it.` (evidence: `src/sim/world/mod.rs:1196..1243`)
- `[RESOLVED] OQ-MTFC-012 - Does current Rust run one fixed step per native frame? -> No; app constants define 45 Hz / 22 ms fixed steps while `binary_frame` maps to 15 Hz.` (evidence: `src/util/fixed_math.rs:47..51`; `src/app_types.rs:24..27`)
- `[RESOLVED] OQ-MTFC-013 - Is pause behavior a full-stop in native? -> No for this slice; prior timing doc says `PerTickUpdate` and frame counter continue while the gameplay/render block skips.` (evidence: `timing/logic-vs-render-loop.md:444..457`; `Main_Tick` decompile)
- `[RESOLVED] OQ-MTFC-014 - Are there INI keys controlling frame increment placement? -> No direct key found; speed/timer INI values feed throttle or timer durations, not placement.` (evidence: docs and source/INI search)
- `[DEFERRED] OQ-MTFC-015 - Which exact Rust timer users must be converted first?` (category: `requires-different-system-context`; reason: slot scope asks for representative hazards, not all timer users; next-step-if-pursued: rank users by player visibility and current use of `binary_frame`, `sim.tick`, or ms timers.)
- `[DEFERRED] OQ-MTFC-016 - What are the exact meanings and writers of all four late-increment gate globals?` (category: `requires-different-system-context`; reason: placement is verified; full flag lifecycle belongs to pause/session/replay timing; next-step-if-pursued: xref each global and map default YR values.)
- `[DEFERRED] OQ-MTFC-017 - Does retail wall-clock pacing produce exactly 15, 60, or variable main ticks per second under each speed?` (category: `needs-runtime-debugger`; reason: static code proves order and throttle units, not runtime scheduler granularity; next-step-if-pursued: sample `g_CurrentFrameCounter` against wall clock in retail.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native tick work observes the pre-increment frame; `g_CurrentFrameCounter` is committed late after service/network work. Active in YR: Yes. | `Main_Tick` decompile; assembly `0x0055D897..0x0055D8F2`, `0x0055DC99..0x0055DE9A` | Mismatch: Rust computes `binary_frame` at tick start. | `src/sim/world/mod.rs::advance_tick`; any future native-frame clock accessor. | Provide a native-frame view where command/subsystem work uses the current frame and the next frame is committed only after the tick's native-equivalent work. | Start at frame `N`; dispatch a command that starts a frame timer and then check it later in the same `advance_tick`; observed current frame remains `N`, not `N+1`. Proposed test: `advance_tick_exposes_preincrement_binary_frame_until_tick_commit`. | Do not "fix" by subtracting one ad hoc at individual call sites; that will drift modulo gates and timers inconsistently. |
| `LogicClassPerTickUpdateLiveVector` runs before late frame increment and uses the pre-increment frame for modulo/timer checks. Active in YR: Yes. | caller `0x0055DC99..0x0055DCA3`; increment `0x0055DE73..0x0055DE81`; `LogicClassPerTickUpdateLiveVector` decompile | **RESOLVED**: Rust commits `binary_frame` late (mod.rs:1397–1398), so ore/gate/combat surfaces all read the pre-increment value during the tick. (corrected 2026-05-29: was "Partial mismatch ... receive `self.binary_frame` after tick-start derivation" — Rust now derives `binary_frame` at tick end, not tick start; verified via read of `src/sim/world/mod.rs:1397-1398`) | `src/sim/world/mod.rs` binary_frame consumers at `1456` (gate_runtime), `1663`/`1668` (combat), `1907`/`1920`/`1946` (ore growth); future scheduler service. (corrected 2026-05-29: was `1267..1274`, `1447..1468`, `1696..1718` — `1267..1274` was never a binary_frame consumer; that region is owner-index rebuild + `apply_due_commands` command dispatch. Re-anchored via Grep of `binary_frame` in `src/sim/world/mod.rs`) | PerTick-style systems should use the same pre-increment frame for the whole pass, including modulo gates like 120-frame bridge/tiberium-style checks. | Seed `binary_frame=N` where `N % 120 == 119`; run one tick; native-equivalent pre-increment modulo should not fire until the next committed frame. Proposed test: `pertick_modulo_gate_uses_preincrement_frame`. | Do not treat `binary_frame` as a wall-clock timestamp that can be recomputed any time during the tick. |
| Native pause/menu skips the normal input/AI/map/render block but does not globally stop `PerTickUpdate` or frame-counter progression. Active in YR: Conditional. | `timing/logic-vs-render-loop.md:444..457`; `Main_Tick` decompile gating | Mismatch/unchecked: Rust `state.paused` prevents `advance_fixed_simulation` entirely. | `src/app_sim_tick.rs:151..159`; future pause semantics boundary between gameplay/render and late scheduler. | Separate "render/input/gameplay block paused" from "frame/per-tick scheduler advances" if parity target includes native pause/menu behavior. | Enter pause/menu, advance one main tick; normal input/AI/map/render effects remain frozen, but a `PerTickUpdate` counter and frame counter advance. Proposed test: `paused_main_tick_advances_pertick_frame_but_skips_gameplay_block`. | Do not model pause as a single app-level stop switch for all native tick work. |

### Representative Affected Surfaces

- `CDTimerClass`-like starts/checks: same-tick start/check boundaries can be one frame early if Rust sees the next frame at tick start. Active in YR: Yes. Evidence: `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md:284`, `321..323`.
- `RateTimer` interpolation: retarget/current calculations are frame-counter based and can shift one frame if retargeting observes a premature frame. Active in YR: Yes. Evidence: `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md:285`, `323`.
- `AnimClass` and visible frame delays: many frame advances are frame-counter/CDTimer based, not render-delta based. Active in YR: Yes. Evidence: `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md:286`, `325`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md:650..658`.
- `LogicClass::PerTickUpdate` modulo gates such as bridge shroud every `0x78` frames: the modulo uses the current pre-increment frame. Active in YR: Yes. Evidence: `PerTickUpdate` decompile; `timing/game-speed-master-clock.md:197..210`.

## 9. Negative Facts / Do Not Do

- Do not claim `binary_frame = floor(total_sim_ms * 15 / 1000)` is parity-correct by itself; native placement is late, not just 15 Hz. Active in YR: Yes; evidence: `0x0055DE73..0x0055DE81`.
- Do not run commands against a newly committed native frame unless a specific native path proves commands are post-increment. Active in YR: Yes; evidence: input/logic command region precedes increment.
- Do not convert every frame timer to Rust fixed-step countdowns; current Rust uses 45 Hz fixed steps, while native timer values are frame-counter based. Active in YR: Yes; evidence: `SIM_TICK_HZ=45` source plus native timer docs.
- Do not use render/app wall-clock milliseconds for `AnimClass`-equivalent lifecycle without proving that effect is not native object-AI/frame-counter driven. Active in YR: Yes for many anims; evidence: timing docs.
- Do not treat pause/menu as a full simulation halt without a branch-specific native proof. Active in YR: Conditional; evidence: timing pause doc and `Main_Tick` gating.

## 10. Stale Docs / Follow-up Docs

> **Audit note (corrected 2026-05-29):** The Rust fix described in this section's "replace with" wording has since been applied. `binary_frame` is now committed late (end of `advance_tick`, lines 1397–1398), matching native behavior. The replacement wording below is superseded by the current code; this section is preserved for historical context. (corrected 2026-05-29: OPERATOR_OR_ORDER_DRIFT — Rust was fixed; doc wording was not updated)

Replace older wording that says:

> `binary_frame` is advanced at the start of `advance_tick`; this matches a 15 Hz native frame clock.

with:

> `binary_frame` is committed late, at the end of `advance_tick` (after all phase work), mirroring native `g_CurrentFrameCounter` which remains at the old frame throughout input, AI, map logic, render, `LogicClass::PerTickUpdate`, and service work, then increments late. This contract is now implemented in Rust.

The source line reference `src/sim/world/mod.rs:1196..1200` (cited in earlier versions of this doc) is stale — the late-commit update is now at lines **1397–1398**. (corrected 2026-05-29)

## 11. Remaining Uncertainty

- Exact lifecycle and default values of the four increment-gate globals are not fully mapped here.
- Replay/transition paths were touched only enough to avoid overclaiming the standard gameplay path.
- Runtime wall-clock measurement of frame rate under speed settings is not part of this static placement proof.
- Per-timer-user Rust fixes remain system-specific follow-up work.

## Sources

- Ghidra decompile: `Main_Tick @ 0x0055D360`
- Ghidra assembly context: normal input/AI/map/render block `0x0055D897..0x0055D8F2`
- Ghidra assembly context: `LogicClass::PerTickUpdate` call `0x0055DC99..0x0055DCA3`
- Ghidra assembly context: service/increment/wait sequence `0x0055DE4A..0x0055DE9A`
- Ghidra decompile: `LogicClass::PerTickUpdate @ 0x0055AFB0`
- Ghidra decompile: `FUN_0055E160 @ 0x0055E160`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/timing/logic-vs-render-loop.md`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/timing/game-speed-master-clock.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_sim_tick.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_types.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/util/fixed_math.rs`
