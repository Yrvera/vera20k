# Modal Pump 0x00623120 Service Tick Contract - Ghidra Research Report

**Address(es):** `FUN_00623120 @ 0x00623120`, `FUN_0055CBF0 @ 0x0055CBF0`, `Main_Tick @ 0x0055D360`, `Process_NetworkMessages @ 0x005D4D50`, `Network_ServiceLoop @ 0x0048D080`, `OptionsClass__ShowInGameDialog @ 0x004E1D00`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The body-level modal pump contract for blocking shell/modeless modal loops, with in-game Options only as caller/liveness context.  
**Non-Scope:** Options control/chrome decode, resource widget layout beyond dialog-id liveness, broader `g_GameState` pause/frame-counter parity outside this pump, and full legacy modem mode behavior.  
**Confidence:** High for pump ordering, mode/blocker gates, reentrancy byte, Options liveness, and Rust app-layer handoff. Medium for exact Win32 wrapper identity of `FUN_0053E770`/`FUN_0053E730` inside `Process_NetworkMessages`; their message-pump role is verified by surrounding calls.  
**Active in YR:** Conditional. Active in all shell/modal loops that call `0x00623120`; standard campaign/skirmish Options use the offline freeze branch, while LAN/WOL Options use the guarded `Main_Tick` branch.

## 0. Working Notes

- Target question: What exactly does `FUN_00623120` service during blocking shell/modeless modal loops, and when does it advance gameplay?
- Non-goals: Options control layout, chrome, resource templates beyond caller id selection, and broader pause/frame-counter parity beyond this modal pump.
- Evidence needed to mark COMPLETE: decompile plus disassembly/caller evidence for pump ordering, mode gates, reentrancy guard, liveness, and Rust-facing handoff.
- Stop conditions: every pump branch and direct callee role is resolved or explicitly deferred; zero-add pass over the function adds no new material questions.

## 1. Overview

`FUN_00623120` is a single service tick, not the owner loop. It always calls `Process_NetworkMessages` first, then either runs a network-service-only branch or, when the mode/blocker/reentrancy gates allow it, calls `Main_Tick`. Active in YR: Yes for shell modal callers. Evidence: decompile `0x00623120`; disassembly `0x00623120..0x00623161`; callers include `OptionsClass__ShowInGameDialog @ 0x004E1D00`, `FUN_005D3490`, `FUN_006AE2C0`, and main/front-end shell runners.

For in-game Options, the caller creates a shell dialog, stores a stack result pointer in window long offset 8, shows the dialog, and loops on `FUN_00623120` until the result is no longer `-1`. Result `1` applies options and writes INI; a pump return of `1` forces result `2`, which skips persist. Active in YR: Yes for in-game Options. Evidence: decompile/disassembly `0x004E1D00..0x004E1DDA`.

## 2. Key Globals / Modes

| Global / value | Role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `g_GameMode @ 0x00A8B238 == 0` | Campaign/single-player offline mode; modal pump freezes sim and calls network service only. | pump branch `0x00623125..0x0062312C`; Main_Game scenario cases set `g_GameMode = 0` before campaign/scenario starts. | Yes, campaign |
| `g_GameMode == 5` | Offline skirmish; modal pump freezes sim and calls network service only. | pump branch `0x0062312E..0x00623131`; Main_Game case `0x0B` assigns `g_GameMode = 5` before `FUN_006AE2C0`. | Yes, skirmish |
| `g_GameMode == 3` | LAN/IPX network route; eligible for guarded `Main_Tick` in modal pump. | Main_Game case 3 assigns `g_GameMode = 3`; later constructs `IPXInterfaceClass`; Network_ServiceLoop has `g_GameMode == 3 || 4` branch. | Conditional, LAN |
| `g_GameMode == 4` | WOL/Internet route; eligible for guarded `Main_Tick` in modal pump. | Main_Game case 2 assigns `g_GameMode = 4`; WOL/Internet logging/string path; Network_ServiceLoop has `g_GameMode == 3 || 4` branch. | Conditional, WOL |
| `DAT_00A8D60E` | Additional pump blocker; if nonzero, pump takes network-service-only branch. | pump read `0x00623133..0x0062313A`; writer/read xrefs include modem/network setup paths. | Conditional |
| `DAT_00A8DAB4` | Additional nesting/blocker counter; if nonzero, pump takes network-service-only branch. | pump read `0x0062313C..0x00623143`; writer xrefs include `FUN_0055CFD0`, `ScenarioClass__Start_Scenario`, `FUN_006475F0`. | Conditional |
| `DAT_00ABCD58` | `Main_Tick` active/reentrancy byte; when nonzero, pump skips `Main_Tick`. | `FUN_0055CBF0` reads byte at `0x0055CBF0..0x0055CBF5`; `Main_Tick` sets at `0x0055D37C` and clears at `0x0055D866`, `0x0055DEB6`, `0x0055DEC8`. | Conditional |
| `g_GameActive @ 0x00A8E9A0` | Options caller uses pump return and this byte to handle teardown; Options dialog id selection uses active-game gate. | Options caller reads at `0x004E1D2A..0x004E1D47` and `0x004E1D81..0x004E1D8C`. | Yes |

## 3. Core Logic Contract

The exact ordered service tick is:

1. Call `Process_NetworkMessages @ 0x005D4D50` unconditionally.
2. If `g_GameMode == 0`, `g_GameMode == 5`, `DAT_00A8D60E != 0`, or `DAT_00A8DAB4 != 0`, call `Network_ServiceLoop @ 0x0048D080` and return `0`.
3. Otherwise call `FUN_0055CBF0`. If it returns nonzero, skip `Main_Tick` and return `0`.
4. If the reentrancy byte is zero, call `Main_Tick @ 0x0055D360`; return `1` only when `Main_Tick` returns nonzero, otherwise return `0`.

Active in YR: Yes for all callers of this service tick; the sim-advance branch is conditional on session mode and guard state. Evidence: decompile `0x00623120`; disassembly range `0x00623120..0x00623161`; direct callee list from `get_function_callees 0x00623120`.

### 3.1 Message / Input / Repaint Service

`Process_NetworkMessages` runs before every branch. If `g_hWnd` is nonzero, it checks/retrieves pending messages through wrappers, then routes each message through registered shell dialogs with `IsDialogMessageA`, accelerator tables with `TranslateAcceleratorA`, a custom message hook at `DAT_00ABFD34`, and finally `TranslateMessage`/`DispatchMessageA`. This is the pump's input and repaint service: input, dialog keys, and `WM_PAINT` are serviced by normal Win32 message dispatch even when sim is frozen.

Active in YR: Yes when the main window exists. Evidence: decompile `0x005D4D50`; disassembly `0x005D4D50..0x005D4E6D`, especially registered-dialog `IsDialogMessageA` at `0x005D4DDB..0x005D4DEE` and `TranslateMessage`/`DispatchMessageA` at `0x005D4E3B..0x005D4E47`.

Important consequence: offline modal pump repaint is UI/window-message repaint, not tactical recomposition. In offline `{0,5}`, `FUN_00623120` never reaches `Main_Tick`, so no normal `RenderFrame_main` path, no PerTickUpdate, and no frame-counter increment occur inside this pump tick. Active in YR: Yes for campaign/skirmish modal loops. Evidence: primary pump branch `0x00623125..0x0062315F`; `Main_Tick` render/frame code is only reachable through the skipped `0x0062314E` call.

### 3.2 Network Service

The offline/blocker branch calls `Network_ServiceLoop` directly after message processing. The network eligible branch does not call `Network_ServiceLoop` separately; when it calls `Main_Tick`, `Main_Tick` reaches its own network service at `0x0055DE4A` on normal late-tick paths. If `DAT_00ABCD58` is nonzero, the pump returns after message processing and does not explicitly call `Network_ServiceLoop`.

Active in YR: Yes/Conditional. Direct network service is active for offline/blocker branches; `Main_Tick` network service is active when the guarded advance path runs. Evidence: primary pump disassembly `0x00623145..0x0062315F`; `Network_ServiceLoop` decompile `0x0048D080`; `Main_Tick` network-service call at `0x0055DE4A`.

### 3.3 Reentrancy Guard

`FUN_0055CBF0` is a two-instruction wrapper returning `DAT_00ABCD58`. `Main_Tick` sets this byte to `1` after passing the `g_GameActive` and `g_GameRunning` entry checks, and clears it on scenario-delay return, normal late return, and session-end return. This makes the modal pump refuse recursive `Main_Tick` entry when a tick is already active.

Active in YR: Conditional, when modal/message processing re-enters the pump while `Main_Tick` is active. Evidence: `FUN_0055CBF0` decompile/disassembly `0x0055CBF0..0x0055CBF5`; xrefs to `DAT_00ABCD58`; `Main_Tick` writes at `0x0055D37C`, `0x0055D866`, `0x0055DEB6`, `0x0055DEC8`; primary pump test/call at `0x00623145..0x0062314C`.

### 3.4 Offline Freeze vs Network Advance

For active standard offline play, the pump freezes simulation:

- Campaign `g_GameMode == 0`: message pump plus `Network_ServiceLoop`, no `Main_Tick`.
- Skirmish `g_GameMode == 5`: message pump plus `Network_ServiceLoop`, no `Main_Tick`.

For active modern network play, the pump advances only when both blocker globals are clear and `DAT_00ABCD58 == 0`:

- LAN/IPX `g_GameMode == 3`: eligible for `Main_Tick`.
- WOL/Internet `g_GameMode == 4`: eligible for `Main_Tick`.

The assembly condition is actually "not 0, not 5, blockers clear"; legacy modes 1/2 also fall through if they exist and blockers are clear, but this report does not claim modern YR gameplay behavior for modem/serial paths. Active in YR: Yes for 0/5/3/4; legacy 1/2 deferred. Evidence: pump assembly `0x00623125..0x00623155`; Main_Game mode assignments in `0x0052D9A0`; `SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md` legacy/dormant partition for modem dialogs.

## 4. In-Game Options Caller Context

`State_Machine @ 0x0048C8B0` case 5 calls `OptionsClass__ShowInGameDialog @ 0x004E1D00`. After return, if `g_GameState` is still `5`, it writes `g_GameState = 1`; otherwise it continues state handling. Active in YR: Yes for in-game Options. Evidence: decompile `0x0048C8B0`; assembly `0x0048C9C9..0x0048C9E6`; caller list for `0x004E1D00`.

`OptionsClass__ShowInGameDialog` chooses dialog `0xBBB` when `g_GameActive == 1` and `0xF5` otherwise, using callback/proc `0x004E1FE0`. This report does not inspect the controls. Active in YR: Yes for active game; shell fallback conditional. Evidence: assembly `0x004E1D2A..0x004E1D47`.

The Options loop:

- Initializes stack result to `-1`.
- Creates/shows dialog through `FUN_00622650` and `FUN_00622800`.
- Calls `FUN_00623120` while result remains `-1`.
- If pump returns `1`, stores result `2`.
- If result is `1`, calls `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0` and `OptionsClass__WriteToINI @ 0x005FAD10`.
- Tears down with `FUN_00622720`.

Active in YR: Yes. Evidence: decompile/disassembly `0x004E1D00..0x004E1DDA`.

## 5. Relation to Game-Active / Pause / Frame Counter

The broader `Main_Tick` pause matrix remains true only when `Main_Tick` is actually called. If `Main_Tick` runs with `g_GameState != 0`, it skips the normal gameplay block but still reaches PerTickUpdate and the late frame-counter increment, as documented in `FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md`.

This modal pump adds a narrower precondition: offline campaign/skirmish Options does not call `Main_Tick` at all, so neither the broader pause path nor the frame counter can run during those pumped modal frames. Network Options can call `Main_Tick`, and then the broader `g_GameState != 0` behavior applies: gameplay input/AI/render block is skipped, PerTickUpdate and frame counter can advance, and network service runs through `Main_Tick`.

Active in YR: Yes for offline Options freeze; conditional for network Options advance. Evidence: pump disassembly `0x00623125..0x00623155`; `Main_Tick` `g_GameState` gate at `0x0055D878..0x0055D901`; late frame increment at `0x0055DE73..0x0055DE81`; frame-counter matrix report.

## 6. INI Keys

No INI key directly controls `FUN_00623120`, `DAT_00ABCD58`, or the modal pump advance decision. Options control values and persistence are outside this report's scope. Active in YR: No direct INI control. Evidence: primary pump reads globals only; no INI readers in `0x00623120`.

## 7. Current Rust Implementation Status

Current Rust has no direct app-layer modal pump decision equivalent to `FUN_00623120`:

| Rust surface | Current status | Delta vs verified pump |
|---|---|---|
| `src/app_sim_tick.rs:146..178` | `advance_in_game_runtime` computes elapsed time and gates all fixed simulation by `!state.paused` unless debug frame-step is requested. | Missing session-mode/reentrancy modal-pump decision; offline freeze matches only the offline branch, not the network Options branch. |
| `src/app_sim_tick.rs:234..295` | `advance_fixed_simulation` owns fixed-step accumulation and calls `sim.advance_tick`. | Correct surface to reuse from app layer; do not push modal/session UI state into `sim/`. |
| `src/app.rs:2742..2743` | In-game redraw loop calls `update_elapsed_ms` then `advance_in_game_runtime`. | Needs an app-layer service-tick seam if native modal loops are modeled. |
| `src/app.rs:3040..3060` and `src/ui/pause_menu.rs` | ESC pause menu is represented by `state.paused` and egui overlay. | Broader pause/frame-counter parity remains separate; this report only proves Options modal pump gating. |
| `src/ui/shell/controller.rs:190..203` | Dialog keyboard route exists in registration order and is render-agnostic. | Useful for message/input parity; not a sim advance policy. |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes / target scope | verified | section 0 | none |
| Primary pump ordering | verified | decompile `0x00623120`; disassembly `0x00623120..0x00623161` | none |
| Direct callee list | verified | `get_function_callees 0x00623120` | none |
| Message/input/repaint dispatch | verified | decompile/disassembly `0x005D4D50` | exact wrapper names for `0x0053E770/0x0053E730` not needed |
| Network service role | verified | decompile `0x0048D080`; pump and Main_Tick callsites | none for this contract |
| `DAT_00ABCD58` guard | verified | `0x0055CBF0`; xrefs; Main_Tick write ranges | none |
| Offline mode freeze | verified | pump branches for mode 0/5; Main_Game mode assignments | none |
| Network mode advance | verified | pump fall-through; Main_Game mode assignments; Network_ServiceLoop 3/4 branch | legacy modes 1/2 deferred |
| Options caller liveness | verified | `OptionsClass__ShowInGameDialog`; State_Machine caller | controls/chrome out of scope |
| Relation to frame counter | verified | pump skips or calls `Main_Tick`; Main_Tick late frame increment; frame-counter matrix doc | broader pause parity out of scope |
| Current Rust handoff surface | verified | `src/app_sim_tick.rs`, `src/app.rs`, `src/ui/shell/controller.rs` scans | implementation not performed |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x00623120` a loop or one service body? -> One service body; owner functions loop around it.` (evidence: callers, especially `0x004E1D67..0x004E1D98` and `0x005D3490` prior report)
- `[RESOLVED] OQ-02 - Does the pump always process messages first? -> Yes, first instruction is call `0x005D4D50`.` (evidence: `0x00623120`)
- `[RESOLVED] OQ-03 - Does message processing route dialog keyboard/input/repaint? -> Yes, registered dialogs receive `IsDialogMessageA`, then accelerators/hook, then `TranslateMessage`/`DispatchMessageA`.` (evidence: `0x005D4D50..0x005D4E6D`)
- `[RESOLVED] OQ-04 - Which offline modes freeze sim in the pump? -> `g_GameMode == 0` and `== 5`.` (evidence: `0x00623125..0x00623131`; Main_Game assignments)
- `[RESOLVED] OQ-05 - Which modern network modes can advance in the pump? -> `g_GameMode == 3` LAN/IPX and `== 4` WOL/Internet, when blockers and reentrancy guard permit.` (evidence: Main_Game assignments; `Network_ServiceLoop` 3/4 branch; pump fall-through)
- `[RESOLVED] OQ-06 - What does `FUN_0055CBF0` do? -> It returns `DAT_00ABCD58`.` (evidence: decompile/disassembly `0x0055CBF0`)
- `[RESOLVED] OQ-07 - Who owns normal `DAT_00ABCD58` set/clear? -> `Main_Tick` sets on entry after active/running checks and clears on exits; cleanup helper also clears during session-end paths.` (evidence: xrefs to `0x00ABCD58`; `0x0055D37C`, `0x0055D866`, `0x0055DEB6`, `0x0055DEC8`; `0x0055CFD0`)
- `[RESOLVED] OQ-08 - Does reentrancy guard still call network service? -> Not explicitly in `0x00623120`; it returns after message processing when `FUN_0055CBF0` is nonzero.` (evidence: `0x00623145..0x0062315F`)
- `[RESOLVED] OQ-09 - Does offline Options call `Main_Tick` through this pump? -> No; mode 0/5 branch returns after direct network service.` (evidence: `0x00623125..0x0062315F`; Options caller loop)
- `[RESOLVED] OQ-10 - Does network Options call `Main_Tick` through this pump? -> Yes, if mode is not 0/5, blockers are clear, and `DAT_00ABCD58 == 0`.` (evidence: `0x00623133..0x00623155`)
- `[RESOLVED] OQ-11 - How does Options result/persist interact with pump return? -> pump return `1` writes local result `2`; only result `1` applies/writes INI.` (evidence: `0x004E1D70..0x004E1DB0`)
- `[RESOLVED] OQ-12 - Does this report decide Options controls/chrome? -> No; only dialog id/proc and pump loop are used for liveness.` (evidence: scope)
- `[DEFERRED] OQ-13 - Legacy mode 1/2 modal-pump gameplay behavior.` (category: out-of-scope; reason: target contract requested campaign/skirmish vs LAN/WOL and legacy modem paths are not modern standard YR; next-step-if-pursued: investigate Main_Game legacy cases plus `DAT_00A8D60E` writers)
- `[DEFERRED] OQ-14 - Exact semantic names for `DAT_00A8D60E` and every writer of `DAT_00A8DAB4`.` (category: out-of-scope; reason: pump contract only needs their branch effect; next-step-if-pursued: investigate writer families `0x0052F3F0`, `0x006475F0`, `ScenarioClass__Start_Scenario`)

Zero-add pass result: re-reading `FUN_00623120` after resolving the caller/guard questions added no new material branches or open questions.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Modal pump advances sim only when mode is network-eligible and blockers/reentrancy permit; offline `{0,5}` freezes. | `0x00623125..0x00623155`; `0x0055CBF0`; Main_Game mode assignments | missing pure decision; current `state.paused` is one app-wide boolean | `src/app_sim_tick.rs`, `src/app.rs` | Add app-layer `modal_pump_should_advance_sim(session_mode, reentrancy, blockers)` or equivalent; do not change `World::advance_tick` signature | Unit: campaign/skirmish return false; LAN/WOL false when reentrant/blockers set, true otherwise | Proposed test `modal_pump_should_advance_sim_matches_gamemd_modes_and_reentrancy`; risk: treating all paused modals as offline freezes breaks network Options |
| Offline in-game Options pumps messages and network service but does not call fixed sim or tactical recomposition. | pump branch `0x00623125..0x0062315F`; Options loop `0x004E1D67..0x004E1D98`; `Process_NetworkMessages` dispatch | current pause freeze broadly prevents sim, but not as a narrow modal pump contract and not tied to session mode | `src/app.rs`, `src/app_sim_tick.rs`, render/app redraw path | Service input/dialog repaint while keeping `World.tick` and fixed sim accumulator unchanged for campaign/skirmish Options | Open in-game Options in skirmish; pump N frames; `World.tick` delta is 0; dialog remains responsive; no catch-up burst on close | Proposed test `offline_options_modal_pump_freezes_world_tick_and_keeps_ui_responsive`; risk: asserting battlefield recomposes each pump frame |
| Network Options can call existing fixed simulation path from app layer while `g_GameState != 0` semantics remain future work. | pump fall-through `0x00623145..0x00623155`; Main_Tick pause gate `0x0055D878..0x0055D901`; frame increment `0x0055DE73..0x0055DE81` | no network modal advance branch; broader pause/frame-counter split not implemented | `src/app_sim_tick.rs`, app network/session mode owner | In LAN/WOL modal service tests, call existing `advance_fixed_simulation` only when pure decision is true; keep `sim/` independent of UI/render/net | Headless network-mode modal service tick advances exactly scheduled fixed ticks while offline equivalent advances zero | Proposed test `network_options_modal_pump_advances_fixed_sim_without_sim_layer_dependency`; risk: putting `SessionMode` into `sim/` or conflating this with full `g_GameState` parity |

## 11. Negative Facts / Do Not Do

- Do not implement `FUN_00623120` as "always advance sim behind modals." Active in YR: No for campaign/skirmish Options. Evidence: mode 0/5 branch `0x00623125..0x0062315A` skips `Main_Tick`.
- Do not claim the battlefield animates behind offline in-game Options. Active in YR: No for pump-driven campaign/skirmish Options; only OS/dialog paint and direct network service run. Evidence: `0x00623125..0x0062315F`; `Main_Tick` not reached.
- Do not model `DAT_00ABCD58` as a user pause flag. It is a `Main_Tick` active/reentrancy byte read by `FUN_0055CBF0`. Evidence: `0x0055CBF0`; Main_Tick set/clear xrefs.
- Do not call `Network_ServiceLoop` unconditionally after `FUN_0055CBF0` blocks `Main_Tick`. The native reentrancy-skip path returns after message processing. Evidence: `0x00623145..0x0062315F`.
- Do not push session mode or modal UI state into `sim/` to reproduce this. The verified native split is app/shell pump deciding whether to call `Main_Tick`; Rust should decide at app layer and reuse existing fixed simulation entry. Evidence: `FUN_00623120` is shell/app pump; `sim/` layering rule.
- Do not treat the broad `g_GameState != 0` frame-counter report as proof that offline Options advances frames; this pump prevents `Main_Tick` in mode 0/5, so the broader branch never runs. Evidence: `0x00623125..0x0062315F`; frame-counter matrix applies only after `Main_Tick` entry.

## 12. Stale Docs / Replacement Wording

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md`: replace C2 wording:
  - Old: `Pump keeps the world live: the pump tick advances sim + network even while a "modal" dialog is up. The in-game options dialog animates the battlefield behind it; a Rust modal that freezes the world is observably wrong.`
  - New: `Pump keeps shell responsiveness live, but sim advance is mode-gated. FUN_00623120 always calls Process_NetworkMessages; campaign/offline skirmish modes 0 and 5, or the DAT_00A8D60E/DAT_00A8DAB4 blockers, take a Network_ServiceLoop-only branch and do not call Main_Tick. Only non-offline modes, practically LAN 3 and WOL/Internet 4, can call Main_Tick, and only when DAT_00ABCD58/FUN_0055CBF0 says no tick is already active. Offline in-game Options freezes world/frame advancement while the dialog remains message-responsive; network Options can advance through Main_Tick.`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md`: replace inventory row for `0x00623120`:
  - Old: `Pump tick (body, not loop) - Process_NetworkMessages + Main_Tick`
  - New: `Pump tick (body, not loop) - Process_NetworkMessages first; then Network_ServiceLoop-only for mode 0/5 or blocker globals; otherwise guarded FUN_0055CBF0 -> Main_Tick, returning 1 only when Main_Tick returns nonzero.`
- `C:/Users/enok/Documents/ra2-rust-game/docs/plans/2026-06-01-shell-substrate-slice5b-kickoff.md`: replace sub-step 3 sentence:
  - Old: `service_tick: always net + input + repaint; advance via the EXISTING advance_fixed_simulation iff the pure decision is true.`
  - New: `service_tick: always processes input/dialog/repaint messages first; offline/blocker branches run network service without sim advance; non-reentrant LAN/WOL branches advance via the EXISTING advance_fixed_simulation. The reentrant guard path must not explicitly run network service after skipping advance.`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/ADDRESS_MAP.md`: replace row `0x00A8B238`:
  - Old: `GameMode (0=SP,1=Skirm,2=LAN,3=WOL,4=TCP)`
  - New: `GameMode for active YR pump-relevant paths: 0=campaign/SP, 3=LAN/IPX, 4=WOL/Internet, 5=offline skirmish; modes 1/2 are legacy modem/serial paths and must not be labeled as offline skirmish.`

## Sources

- Ghidra read-only decompile/disassembly: `0x00623120`, `0x0055CBF0`, `0x005D4D50`, `0x0048D080`, `0x0055D360`, `0x0048C8B0`, `0x004E1D00`, `0x0052D9A0`, `0x0055CFD0`.
- Ghidra xrefs/callers: callers of `0x00623120`; callers of `0x004E1D00`; xrefs to `0x00ABCD58`, `0x00A8B238`, `0x00A8D60E`, `0x00A8DAB4`.
- Research docs referenced: `docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md`, `docs/research/FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md`, `docs/research/RANDOM_SCENARIO_ENGINE_SUBSTRATE_SERVICE_STUDY.md`, `docs/research/skirmish-ui/VALIDATION_MODAL_0X005D3490_KEYBOARD_DEFAULT_DISMISSAL_GHIDRA_REPORT.md`.
- Rust surfaces scanned: `src/app_sim_tick.rs`, `src/app.rs`, `src/app_input.rs`, `src/ui/pause_menu.rs`, `src/ui/shell/controller.rs`, `src/app_types.rs`.
