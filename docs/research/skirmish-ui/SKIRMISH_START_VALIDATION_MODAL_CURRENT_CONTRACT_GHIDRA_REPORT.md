# Skirmish Start Validation Modal Current Contract - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x005D3490`, `0x006AE2C0`, `0x006AE3F0`, `0x007B7100`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** current Rust versus active standard YR offline Skirmish Start command `0x617` validation failure UI: validation predicates, native modal text/button contract, parent disabled/re-enabled path, accepted selected-mode behavior, no-start side effects, and whether current `NoSelectedMode` handling should become a native modal.  
**Non-Scope:** Create Random Map, player-name edit, successful launch/session packing after the validation boundary, gameplay spawn/init, Choose Map keyboard behavior, exact modal bitmap reconstruction, and custom/modded MPModes objects beyond the selected-mode false/output gate.  
**Confidence:** High for native failure conditions/text source/control ids/no-start side effects and current Rust source status; Medium for exact modal visual parity and validation-modal keyboard dismissal because those were bounded out.  
**Active in YR:** Yes for ordinary offline Skirmish validation failures; Conditional for selected-mode rejection by selected mode vtable result and output dword.

## 0. Working Notes

- Target question: Reconcile current Rust Start validation modal implementation with native Start failure behavior, including predicates, text/buttons, parent disabled/re-enabled flow, no-start side effects, selected-mode acceptance, and `NoSelectedMode`.
- Non-goals: Do not investigate Create Random Map, player-name edit, Choose Map keyboard behavior, successful session packing, spawn/init, or native modal pixel reconstruction.
- Evidence needed to mark COMPLETE: decompile plus assembly/context evidence for native failure branches and modal helper; current Rust source scan for mapping, render, input blocking, OK dismissal, and `NoSelectedMode`; stale-doc replacement wording.
- Stop conditions: stop once all scoped native blocking branches and Rust counterparts are classified; write only this report plus the shared swarm claims row.

## 1. Overview

Native standard YR disables Start `0x617` before validation, shows a blocking shell modal for ordinary Start failures, re-enables Start after the modal returns, and exits before session packing. Active in YR: Yes. Evidence: `0x006ACEE0`; assembly/context `0x006ACF92..0x006ACF9E`, `0x006AD05B..0x006AD34B`.

Current Rust is not log-only anymore. It maps the three ordinary native failure categories into `SkirmishValidationModalState`, renders a primitive validation modal overlay, consumes parent shell input while open, and clears it only by OK-button mouse-up. Active in YR comparison: Yes, as a current Rust parity surface. Evidence: `src/app.rs:622..708`, `src/app.rs:1116..1131`, `src/app_skirmish_shell_render.rs:258..310`, `src/app_skirmish_shell_render.rs:437`.

The remaining Rust-facing gaps are narrower: native modal art/template geometry is still unverified, focused app/render acceptance tests are missing, validation-modal Enter/Escape parity remains uncertain, transient native Start disabled timing is approximated by modal input blocking/render state, and selected-mode `+0x14` false/output handling is not modeled. Active in YR: Yes/Conditional as noted per item. Evidence: sections 2-7.

## 2. Native Validation Contract

| Finding | Evidence | Active in YR |
|---|---|---|
| Offline Skirmish shell creates dialog `0x102`, stores a local result pointer at `GWL_USERDATA` offset `8`, pumps until result `0x617` or `0x5C0`, and returns true only for `0x617`. | decompile `0x006AE2C0` | Yes |
| `WM_COMMAND (0x111)` routes through `0x006AE3F0` into `0x006ACEE0`; Start/Back command handling ignores nonzero notification high words. | decompile `0x006AE3F0`; context `0x006ACF7B..0x006ACF86` | Yes |
| Start `0x617` notification `0` calls `GetDlgItem(hwnd, 0x617)` and `EnableWindow(..., 0)` before validation. | context `0x006ACF92..0x006ACF9E` | Yes |
| Active AI rows are counted from row-type combo item data `0`, `1`, or `2` for controls `0x50B/0x50E/0x516/0x51A/0x51B/0x51C/0x51D`; total players are active AI count plus local player. | decompile `0x006ACEE0`; context `0x006ACFBD..0x006AD052` | Yes |
| Modal helper `0x005D3490` writes the first non-empty text to child `0x5B0`, second non-empty text to child `0x5AE`, optional third text to control `2`, optional fourth text to `0x5AF`, then runs a blocking modal pump until a local result changes. | decompile `0x005D3490` | Yes |
| Scoped Start failure calls pass zero optional button-label parameters; visible OK text is the second text argument, not an optional control override. | call contexts around `0x006AD0A7..0x006AD0C6`, `0x006AD126..0x006AD145`, `0x006AD274..0x006AD293`, `0x006AD2E3..0x006AD316` | Yes |

## 3. Failure Predicates, Text, And No-Start Effects

| Failure | Native condition | Native text/button | Side effects | Evidence | Active in YR |
|---|---|---|---|---|---|
| Map capacity exceeded | selected map capacity `< active_ai_count + 1` | body `TXT_SCENARIO_TOO_SMALL` formatted with capacity; OK text `TXT_OK` | modal, re-enable Start, return before packing/result write | `0x006AD05B..0x006AD0DA`; prior text report | Yes |
| Fewer than two total players | `active_ai_count + 1 < 2` | body `TXT_NEED_AT_LEAST_TWO_PLAYERS`; OK text `TXT_OK` | modal, re-enable Start, return before packing/result write | `0x006AD0ED..0x006AD159`; prior text report | Yes |
| Same explicit team | local team control `0x76D` returns nonnegative and every active AI team equals it; local Team None/negative skips the branch | body `TXT_CANNOT_ALLY`; OK text `TXT_OK` | modal, re-enable Start, return before packing/result write | `0x006AD16C..0x006AD2A7`; prior team helper reports for `0x004E5940`/`0x004E6030` | Yes |
| Selected-mode rejection | selected MPMode vtable `+0x14` returns false and local output dword equals literal `0x617` | output body from `FUN_007B7100`; OK text loaded with string id `0x469` | modal, re-enable Start, free output object, return before packing/result write | `0x006AD2BA..0x006AD34B`; decompile `0x007B7100` | Conditional |

Retail English text remains as previously verified: `TXT_SCENARIO_TOO_SMALL` is `This map has a %d player max. The max includes human and computer players.`, `TXT_NEED_AT_LEAST_TWO_PLAYERS` is `You need at least two players to start the game!`, `TXT_CANNOT_ALLY` is `Must have more than one team to start a game!`, and `TXT_OK` is `OK`. Active in YR: Yes for local retail English. Evidence: `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`.

No ordinary failure reaches the session-packing block beginning at `0x006AD34B`, no ordinary failure writes the final shell result, and no ordinary failure begins map load/start. Active in YR: Yes. Evidence: failure returns at `0x006AD0DA`, `0x006AD159`, and `0x006AD2A7`; selected-mode blocking return follows `0x006AD31B..0x006AD334`.

Start is re-enabled after the blocking modal returns and before each failure branch exits. Active in YR: Yes. Evidence: re-enable contexts `0x006AD0CB..0x006AD0DA`, `0x006AD14A..0x006AD159`, `0x006AD298..0x006AD2A7`, and `0x006AD31B..0x006AD32A`.

## 4. Current Rust Implementation Status

| Rust surface | Current status | Delta vs native |
|---|---|---|
| `src/ui/skirmish_shell/state.rs:327` | `SkirmishValidationModalState` carries `message` and `ok_button`. | Represents the two visible texts used by scoped Start failures; does not model native control ids `0x5B0/0x5AE/2/0x5AF`. |
| `src/ui/skirmish_shell/state.rs:1962` `launch_session` | rejects selected-map missing, capacity overflow, no enabled opponent, same explicit team, unknown selected mode, invalid color, and invalid start position. | Ordinary native data categories are present; selected-mode `+0x14` callback/output dword is not modeled. |
| `src/skirmish_launch.rs:233` | `LaunchValidationError` includes ordinary native failures plus Rust-internal errors including `NoSelectedMode`. | Ordinary modal categories are representable; `NoSelectedMode` is Rust-internal and should remain log/repair unless a future selected-mode callback model produces native output `0x617`. |
| `src/app.rs:622..708` | Start `Err` maps `MapCapacityExceeded`, `NoEnabledOpponent`, and `SameExplicitTeam` to localized CSF text and sets `validation_modal`; it clears pressed/dropdown/drag/edit state. | Major stale-doc correction: modal wiring now exists. Remaining mismatch: native transient disable/re-enable is approximated by modal input blocking and rendered disabled state. |
| `src/app.rs:1116..1131` | mouse-down is consumed whenever validation modal is open; mouse-up clears the modal only if cursor is inside `layout.ok_button`. | Broadly matches blocking modal and OK mouse dismissal; native Enter/Escape/default-button behavior not proven in this slot. |
| `src/app.rs:1231`, `src/app.rs:1252`, `src/app.rs:1483` | hover/wheel/global Escape paths are consumed while validation modal is open. | Blocks parent shell interactions behind the modal; exact native keyboard behavior remains deferred. |
| `src/ui/skirmish_shell/layout.rs:849..867` | primitive centered validation layout with `dialog`, `message`, and `ok_button`. | Not verified against native modal resource rectangles/pixels. |
| `src/app_skirmish_shell_render.rs:258..310`, `src/app_skirmish_shell_render.rs:437` | Start button is drawn disabled when `validation_modal.is_some()`, and validation modal instances/text are pushed after shell/modal text draw setup. | Functional overlay exists; native art/template parity not proven. |
| `src/app_skirmish_shell_render.rs:1093` | test verifies semantic draw order for validation modal overlay. | Useful draw-order test; missing app-level Start failure -> modal text -> OK dismissal -> retry tests. |

## 5. `NoSelectedMode` Contract

Current Rust's `LaunchValidationError::NoSelectedMode { mode_id }` is an internal consistency failure from resolving `selected_mode_id` against the loaded mode list. Native standard YR does not have a generic "selected mode id missing" modal in the Start branch; it has a selected MPMode object pointer and calls that object's vtable `+0x14`. A Start-blocking modal occurs only when that callback returns false and the callback's output dword is `0x617`. Active in YR: Conditional. Evidence: `0x006AD2BA..0x006AD34B`.

Implementation implication: do not map current `NoSelectedMode` directly to a native validation modal. It should be prevented by Choose Map/default-state repair, logged as internal corruption, or converted only through a future native selected-mode acceptance model that can represent false plus output `0x617`. Active in YR comparison: Yes. Evidence: current Rust `mode_by_id(...).ok_or(NoSelectedMode)` at `src/ui/skirmish_shell/state.rs:1998`; native gate at `0x006AD2D9..0x006AD346`.

## 6. Negative Facts / Do Not Do

- Do not say current Rust is still log-only for ordinary native Start validation failures. Active in YR comparison: Yes; current Rust sets and renders `validation_modal`. Evidence: `src/app.rs:622..708`, `src/app_skirmish_shell_render.rs:258..310`.
- Do not transition to loading, pack a session, or write a launch result after ordinary native validation failures. Active in YR: Yes. Evidence: ordinary branches return before `0x006AD34B`.
- Do not leave Start unusable after a validation failure. Active in YR: Yes. Evidence: re-enable ranges `0x006AD0CB..0x006AD32A`.
- Do not treat same-team validation as a start-position collision. Active in YR: Yes. Evidence: `0x006AD16C..0x006AD2A7` reads team controls, while start-position validation is not this branch.
- Do not make every selected-mode false return a blocking modal. Active in YR: Conditional. Evidence: `0x006AD2D9..0x006AD346` gates the modal on output dword `0x617`.
- Do not map Rust `NoSelectedMode` to one of the native ordinary validation messages. Active in YR comparison: Yes; native selected-mode handling uses an object callback/output contract, not id lookup failure text. Evidence: `0x006AD2BA..0x006AD34B`.
- Do not claim the primitive Rust modal layout is native-pixel verified. Active in YR comparison: Yes. Evidence: Rust hand-built layout at `src/ui/skirmish_shell/layout.rs:849..867`; exact native modal template/pixels deferred.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Required working notes and scope | verified | section 0 | none |
| Shell loop `0x006AE2C0` | verified | Ghidra decompile | none for scoped failure contract |
| Command dispatch `0x006AE3F0` | verified | Ghidra decompile | none for scoped failure contract |
| Start notification disable | verified | `0x006ACF92..0x006ACF9E` | exact disabled-frame pixels belong to visual work |
| Active AI row count | verified | `0x006ACFBD..0x006AD052` | none |
| `0x005D3490` modal helper/control ids | verified | Ghidra decompile | exact resource/template rectangles deferred |
| Capacity failure branch | verified | `0x006AD05B..0x006AD0DA` | native screenshot comparison deferred |
| Min-player failure branch | verified | `0x006AD0ED..0x006AD159` | native screenshot comparison deferred |
| Same explicit team branch | verified | `0x006AD16C..0x006AD2A7` | native screenshot comparison deferred |
| Selected-mode false/output `0x617` branch | verified | `0x006AD2BA..0x006AD34B`, `0x007B7100` | custom/modded mode objects out of scope |
| Current Rust ordinary validation data categories | verified | `src/ui/skirmish_shell/state.rs:1962` | none for ordinary categories |
| Current Rust modal mapping and state clearing | verified | `src/app.rs:622..708` | add focused app tests |
| Current Rust modal rendering | verified | `src/app_skirmish_shell_render.rs:258..310`, `src/app_skirmish_shell_render.rs:437` | native visual parity deferred |
| Current Rust `NoSelectedMode` | verified | `src/ui/skirmish_shell/state.rs:1998`; native gate `0x006AD2D9..0x006AD346` | keep log/internal unless native selected-mode callback model is added |
| Validation-modal keyboard dismissal | touched-not-exhausted | `0x005D3490` modal pump decompiled; current Rust global Escape consumes but does not dismiss | follow-up modal proc/key command trace |
| Successful launch/session packing | not-touched | out of scope | use MPModes/session packing reports |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the scoped path live in standard YR? -> Yes; offline Skirmish shell pumps result `0x617/0x5C0` and routes `WM_COMMAND` to `0x006ACEE0`.` (evidence: `0x006AE2C0`, `0x006AE3F0`)
- `[RESOLVED] OQ-02 - Which ordinary failures block Start? -> capacity overflow, fewer than two players, and all active players on one explicit team.` (evidence: `0x006AD05B..0x006AD2A7`)
- `[RESOLVED] OQ-03 - What text appears? -> capacity uses `TXT_SCENARIO_TOO_SMALL`, min-player uses `TXT_NEED_AT_LEAST_TWO_PLAYERS`, same-team uses `TXT_CANNOT_ALLY`, all with second text `TXT_OK`.` (evidence: prior text report; call sites `0x006AD073..0x006AD293`)
- `[RESOLVED] OQ-04 - Which modal controls receive text? -> `0x005D3490` sends `0x4B2` to children `0x5B0` and `0x5AE`; optional controls `2` and `0x5AF` are only written for non-empty optional args.` (evidence: decompile `0x005D3490`)
- `[RESOLVED] OQ-05 - Does native disable and re-enable Start? -> Yes; disable before validation, re-enable after modal and before return.` (evidence: `0x006ACF92..0x006ACF9E`, `0x006AD0CB..0x006AD32A`)
- `[RESOLVED] OQ-06 - Does native pack/start after ordinary failure? -> No; ordinary failure branches return before `0x006AD34B` packing/result writes.` (evidence: decompile `0x006ACEE0`)
- `[RESOLVED] OQ-07 - Is same-team a start-position collision check? -> No; it reads team controls and skips when local team is negative/None.` (evidence: `0x006AD16C..0x006AD2A7`)
- `[RESOLVED] OQ-08 - Does selected-mode false always show a blocking modal? -> No; only false plus output dword `0x617` blocks.` (evidence: `0x006AD2D5..0x006AD346`)
- `[RESOLVED] OQ-09 - Does current Rust have ordinary data validation? -> Yes.` (evidence: `src/ui/skirmish_shell/state.rs:1962`)
- `[RESOLVED] OQ-10 - Does current Rust set a visible validation modal on native ordinary Start failures? -> Yes; earlier current-contract wording is stale.` (evidence: `src/app.rs:622..708`)
- `[RESOLVED] OQ-11 - Does current Rust render validation modal text/instances? -> Yes, via primitive modal draw/text paths.` (evidence: `src/app_skirmish_shell_render.rs:258..310`, `src/app_skirmish_shell_render.rs:437`)
- `[RESOLVED] OQ-12 - Does current Rust block parent shell input behind the modal? -> Yes for mouse/wheel/status/Escape shell-close paths.` (evidence: `src/app.rs:1116..1131`, `src/app.rs:1231`, `src/app.rs:1252`, `src/app.rs:1483`)
- `[RESOLVED] OQ-13 - Should current `NoSelectedMode` get a native modal? -> No; it is a Rust id-resolution failure, while native selected-mode blocking is callback false plus output dword `0x617`.` (evidence: `src/ui/skirmish_shell/state.rs:1998`; native `0x006AD2BA..0x006AD34B`)
- `[DEFERRED] OQ-14 - Exact native modal pixels/resource rectangles for `0x005D3490`.` (category: out-of-scope; reason: this slot verifies current failure contract, not native visual reconstruction; next-step-if-pursued: resource/template extraction plus screenshot comparison)
- `[DEFERRED] OQ-15 - Validation-modal Enter/Escape/default-button keyboard parity.` (category: requires-different-system-context; reason: `0x005D3490` pump is verified but its dialog proc/button command mapping was not drained; next-step-if-pursued: trace modal resource/proc commands for controls `2` and `0x5AF`)
- `[DEFERRED] OQ-16 - Custom/modded MPModes object intentionally writing output dword `0x617`.` (category: out-of-scope; reason: stock/local selected-mode behavior already covered by MPModes reports; next-step-if-pursued: investigate custom MPModes extension path)

## 9. Visual/UI Composition Ledger

This report does not claim native modal pixel composition. It verifies the behavioral modal contract and current Rust overlay existence.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `0x006ACEE0` | Start `0x617`, notify `0` | n/a | disables Start control `0x617` | n/a | yes | parent command gate |
| 2 | `0x005D3490` | validation failure branch | modal dialog resource, not reconstructed here | child texts `0x5B0`, `0x5AE`; optional `2`, `0x5AF` | shell text path, not pixel-reconstructed here | yes | blocking modal |
| 3 | Rust `push_validation_modal_instances` / text draws | `validation_modal.is_some()` | primitive Rust modal/button, no native asset proof | `compute_validation_modal_layout` centered rects | Rust shell text renderer | yes in Rust | current functional overlay |

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| Native `0x005D3490` modal resource | yes | yes | yes | no | yes | yes | no | no | decompile `0x005D3490` |
| Rust primitive validation panel/button | yes | yes | yes when `validation_modal` present | no | yes | yes | no | no | `src/app_skirmish_shell_render.rs:258..310`, `src/ui/skirmish_shell/layout.rs:849..867` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Capacity overflow shows `TXT_SCENARIO_TOO_SMALL` formatted with capacity plus `TXT_OK`, re-enables Start, and does not launch. | `0x006AD05B..0x006AD0DA`; text report | functionally mapped; missing focused app/render tests and native visual parity | `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs` | preserve CSF mapping, modal draw, no-launch state, and OK retry; later replace primitive modal visuals with verified native layout if needed | capacity `2`, local plus two enabled AIs: visible modal says map has `2 player max`, OK dismisses, no launch/session transition, Start can be clicked again | Do not append non-native requested/capacity suffix; proposed test `skirmish_start_capacity_error_sets_validation_modal_text` |
| No enabled opponent shows `TXT_NEED_AT_LEAST_TWO_PLAYERS` plus `TXT_OK` and does not launch. | `0x006AD0ED..0x006AD159`; text report | functionally mapped; missing app/render acceptance tests | same | preserve native min-player text and modal blocking behavior | all AI rows disabled: shell remains, modal text is `You need at least two players to start the game!`, OK clears modal | Do not reduce this to log-only; proposed test `skirmish_start_no_opponent_modal_uses_native_text` |
| Same explicit team shows `TXT_CANNOT_ALLY` plus `TXT_OK`; local Team None skips this branch. | `0x006AD16C..0x006AD2A7` | functionally mapped; missing app/render acceptance tests | `src/ui/skirmish_shell/state.rs`, `src/app.rs` | preserve Team None skip and same-explicit-team modal text | local team A and all active AIs team A blocks; local Team None with same AI teams does not block for this reason | Do not implement as start-position collision; proposed test `skirmish_start_same_explicit_team_modal_uses_cannot_ally_text` |
| Modal blocks parent shell input until OK/result, then Start is usable again. | `0x005D3490`; re-enable ranges `0x006AD0CB..0x006AD32A` | Rust consumes input and OK clears modal; native disabled `0x617` timing is approximated | `src/app.rs`, owner-draw button state/render | prove double-click/behind-modal input cannot start or mutate shell and OK permits retry | invalid setup, double-click Start: one modal, no combo/dropdown/launch side effects; OK then Start shows modal again | Do not leave Start permanently disabled; proposed test `skirmish_validation_modal_ok_clears_and_retry_works` |
| Selected-mode rejection blocks only when `+0x14` false also writes output dword `0x617`; stock/local modes do not use a generic id-missing modal. | `0x006AD2BA..0x006AD34B`; selected-mode reports | missing selected-mode callback model; current `NoSelectedMode` is Rust-internal | future MPModes acceptance model and `launch_session` validation | do not display a native validation modal for `NoSelectedMode`; when callback model exists, block only false+`0x617` with callback output body + OK | synthetic mode false+`0x617` blocks with output body and OK; false+zero and missing id do not reuse ordinary modal text | Do not make every mode false or missing id a native modal; proposed test `skirmish_mode_rejection_blocks_only_native_start_code` |

## 11. Stale Docs / Follow-up Docs

- In `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`, no replacement needed for the current-state summary; it already says Rust maps three ordinary failures to a rendered primitive validation modal.
- Replace older wording wherever it appears in `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_GHIDRA_REPORT.md`: "It does not currently set `validation_modal` on Start failure, does not draw validation modal instances/text, and only logs `LaunchValidationError` from the Start action path." -> "Current Rust maps `MapCapacityExceeded`, `NoEnabledOpponent`, and `SameExplicitTeam` to `SkirmishValidationModalState`, renders a primitive validation modal, consumes parent shell input while it is open, and dismisses it via OK mouse-up. Remaining deltas are native modal visual/template parity, app/render acceptance tests, native disabled Start timing, validation-modal keyboard parity, and future selected-mode `+0x14` output handling."
- Add/update implementation wording for `NoSelectedMode`: "Current Rust `NoSelectedMode` is an internal selected-mode id/list mismatch. Native Start does not show a generic missing-mode modal; selected-mode blocking occurs only when the selected MPMode callback returns false and writes output dword `0x617`."

## Sources

- Ghidra read-only decompile/assembly: `0x006ACEE0`, `0x005D3490`, `0x006AE2C0`, `0x006AE3F0`, `0x007B7100`; assembly contexts around `0x006ACF7B`, `0x006AD05B`, `0x006AD0ED`, `0x006AD16C`, `0x006AD2BA`, `0x005D3490`.
- Prior reports: `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`, `SKIRMISH_SELECTED_MODE_START_VALIDATION_CONTRACT_GHIDRA_REPORT.md`.
- Rust scanned: `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, `src/skirmish_launch.rs`.
