# Skirmish Start Validation Modal Current Contract Recheck - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x005D3490`, `0x006AE2C0`, `0x006AE3F0`, `0x007B7100`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** current Rust versus active standard YR offline Skirmish Start command `0x617` validation failure UI: trigger conditions, message source/text, modal helper/control ids, parent/modal input blocking, no-start side effects, and current Rust deltas.  
**Non-Scope:** successful launch/session packing after `0x006AD34B`, gameplay spawn/init, Choose Map `0x6B`, random-map generation, exact native modal bitmap reconstruction, and custom/modded MPModes objects beyond the known selected-mode false gate.  
**Confidence:** High for native failure conditions/text source/control ids/no-start side effects and current Rust source status; Medium for exact modal pixel parity because this report did not render native screenshots.  
**Active in YR:** Yes for ordinary offline Skirmish validation failures; Conditional for selected-mode rejection by selected mode vtable return and output dword.

## 0. Working Notes

- Target question: Reconcile current Rust Start validation failure modal with active standard YR, especially after recent Rust modal wiring.
- Non-goals: Do not rediscover successful session packing, Choose Map behavior, launch/spawn generation, or unrelated shell layout.
- Evidence needed to mark COMPLETE: decompile and assembly evidence for native failure branches and modal helper; decoded text source from prior report; current Rust source scan for trigger mapping, modal render, OK dismissal, blocking behavior, and gaps.
- Stop conditions: stop after all native blocking failure branches and their current Rust counterparts are classified; write exactly this report and update the swarm claims row.

## 1. Overview

Native standard YR disables Start `0x617` before validation, shows a shell modal for ordinary blocking failures, re-enables Start after the modal returns, and exits before launch packing. Active in YR: Yes. Evidence: decompile `0x006ACEE0`; assembly ranges `0x006ACF92..0x006ACF9E`, `0x006AD05B..0x006AD34B`.

Current Rust is no longer log-only. It now maps the three ordinary native failure categories into `SkirmishValidationModalState`, renders a primitive centered modal, consumes mouse/wheel input while it is open, and clears it only on OK-button mouse-up. Current deltas are narrower: native modal template/pixels are still not verified, Start is not modeled as a native disabled control during the modal, Enter/Escape dismissal parity is unimplemented/uncertain, and the selected-mode `+0x14` output contract is still not modeled. Evidence: `src/app.rs:619..708`, `src/app.rs:1059..1077`, `src/app_skirmish_shell_render.rs:1345..1372`, `src/app_skirmish_shell_render.rs:2304..2329`, `src/app_skirmish_shell_render.rs:2599..2688`.

## 2. Native Failure UI Contract

| Behavior | Evidence | Active in YR |
|---|---|---|
| Offline Skirmish shell creates dialog `0x102`, stores a local result pointer at `GWL_USERDATA` offset `8`, pumps until result `0x617` or `0x5C0`, and returns true only for `0x617`. | decompile `0x006AE2C0` | Yes |
| `WM_COMMAND (0x111)` routes through `0x006AE3F0` into `0x006ACEE0`; Start/Back branch ignores nonzero notification high words. | decompile `0x006AE3F0`; assembly `0x006ACF7B..0x006ACF86` | Yes |
| Start `0x617` notification `0` disables control `0x617` before validation. | assembly `0x006ACF92..0x006ACF9E` | Yes |
| Active AI rows are row-type combo item-data `0`, `1`, or `2` from controls `0x50B/0x50E/0x516/0x51A/0x51B/0x51C/0x51D`; total players are active AI count plus local player. | decompile `0x006ACEE0`; assembly `0x006ACFBD..0x006AD052` | Yes |
| Modal helper `0x005D3490` writes message/body text to child `0x5B0`, second text to child `0x5AE`, optional button text to controls `2` and `0x5AF` only when non-empty, then runs its own modal pump until a local result changes. | decompile `0x005D3490` | Yes |
| Scoped Start failures pass zero optional button-label parameters; the visible OK/control text is the second argument `TXT_OK`, not a custom `param_3`/`param_4`. | assembly `0x006AD0A7..0x006AD0C6`, `0x006AD126..0x006AD145`, `0x006AD274..0x006AD293`, `0x006AD2E3..0x006AD316` | Yes |

## 3. Failure Triggers, Text, And Side Effects

| Failure | Native condition | Text source and visible text | Native side effects | Evidence | Active in YR |
|---|---|---|---|---|---|
| Map capacity exceeded | selected map capacity `< active_ai_count + 1` | `TXT_SCENARIO_TOO_SMALL` formatted with capacity; retail English: `This map has a %d player max. The max includes human and computer players.`; second text `TXT_OK` | modal, re-enable Start, return before packing/result write | `0x006AD05B..0x006AD0DA`; prior text report | Yes |
| Fewer than two total players | `active_ai_count + 1 < 2` | `TXT_NEED_AT_LEAST_TWO_PLAYERS`; retail English: `You need at least two players to start the game!`; second text `TXT_OK` | modal, re-enable Start, return before packing/result write | `0x006AD0ED..0x006AD159`; prior text report | Yes |
| Same explicit team | local team control `0x76D` returns nonnegative and every active AI team equals it; local Team None/negative skips this check | `TXT_CANNOT_ALLY`; retail English: `Must have more than one team to start a game!`; second text `TXT_OK` | modal, re-enable Start, return before packing/result write | `0x006AD16C..0x006AD2A7`; helpers `0x004E5940`, `0x004E6030` from prior reports | Yes |
| Selected-mode rejection | selected MPModes vtable `+0x14` returns false and local output dword equals literal `0x617` | mode output string from `FUN_007B7100`; second text `TXT_OK` loaded via ID `0x469` | modal, re-enable Start, free output object, return before packing/result write | `0x006AD2BA..0x006AD34B`; decompile `0x007B7100` | Conditional |

No ordinary failure writes the final shell result or enters the session-packing block at `0x006AD34B`. Active in YR: Yes. Evidence: failure returns at `0x006AD0DA`, `0x006AD159`, and `0x006AD2A7`; selected-mode blocking return follows `0x006AD31B..0x006AD334`.

## 4. Current Rust Implementation Status

| Rust surface | Current status | Delta vs native |
|---|---|---|
| `src/ui/skirmish_shell/state.rs:316..328` | `SkirmishValidationModalState` carries `message` and `ok_button`. | Compatible with native two-text failure calls, but does not model native control ids `0x5B0/0x5AE/2/0x5AF`. |
| `src/ui/skirmish_shell/state.rs:1853..1889` | `launch_session` validates selected map, capacity overflow, no active opponents, and same explicit team before building session slots. | Ordinary native data categories are present; selected-mode `+0x14` acceptance/output dword contract is missing. |
| `src/skirmish_launch.rs:210..231` | `LaunchValidationError` includes `NoEnabledOpponent`, `MapCapacityExceeded`, `SameExplicitTeam`, plus Rust-internal failures. | Native modal categories are representable; Rust-internal errors intentionally remain log-only in `src/app.rs`. |
| `src/app.rs:619..708` | Start `Err` maps the three native ordinary failures to localized CSF text and sets `validation_modal`; it also clears pressed/dropdown/drag/edit state. | Major stale-doc correction: visible modal wiring now exists. Remaining mismatch: Start is not a real disabled button until modal return; it is input-blocked by modal state. |
| `src/app.rs:1059..1077` | mouse-down is consumed whenever validation modal is open; mouse-up clears modal only if cursor is inside `layout.ok_button`. | Broadly matches modal blocking and explicit OK dismissal by mouse; Enter/Escape behavior is not implemented for validation-modal OK/cancel. |
| `src/app.rs:1166..1192`, `src/app.rs:1419..1425` | mouse move/wheel and global Escape are consumed while validation modal is open. | Prevents parent shell interaction behind modal; native exact keyboard behavior of `0x005D3490` was not proven in this slot. |
| `src/ui/skirmish_shell/layout.rs:849..867` | primitive centered validation layout with `dialog`, `message`, and `ok_button`. | Not verified against native modal template/control rectangles; likely visual parity gap. |
| `src/app_skirmish_shell_render.rs:1345..1372`, `src/app_skirmish_shell_render.rs:2304..2329`, `src/app_skirmish_shell_render.rs:2599..2688` | primitive solid panel/outline/button and text are rendered when `validation_modal` is present. | Modal is functional but not proven native-art/native-geometry parity. |
| tests in `src/ui/skirmish_shell/state.rs:2940..2970` | cover data-level ordinary validation errors. | Missing app/render tests for modal state mapping, visible text draw, OK dismissal, Start reuse, and no-launch side effects. |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes and target scope | verified | section 0 | none |
| Shell loop `0x006AE2C0` | verified | Ghidra decompile | none |
| Command dispatch `0x006AE3F0` | verified | Ghidra decompile | none |
| Start notification disable | verified | `0x006ACF92..0x006ACF9E` | exact visual disabled-frame behavior belongs to slot 5 |
| Active AI row count | verified | `0x006ACFBD..0x006AD052` | none |
| `0x005D3490` modal helper/control ids | verified | Ghidra decompile | exact dialog resource rectangles/pixels not covered |
| Capacity failure branch | verified | `0x006AD05B..0x006AD0DA` | native screenshot comparison deferred |
| Min-player failure branch | verified | `0x006AD0ED..0x006AD159` | native screenshot comparison deferred |
| Same explicit team branch | verified | `0x006AD16C..0x006AD2A7` | native screenshot comparison deferred |
| Selected-mode `0x617` gate | verified | `0x006AD2BA..0x006AD34B`, prior mode sweep | custom/modded mode objects out of scope |
| Current Rust ordinary validation categories | verified | `src/ui/skirmish_shell/state.rs:1853..1889` | none |
| Current Rust modal mapping and display | verified | `src/app.rs:619..708`, render paths listed above | add focused tests and visual parity work |
| Native validation-modal keyboard dismissal | touched-not-exhausted | `0x005D3490` modal pump decompiled, but dialog proc/button command mapping not drained here | follow-up if Enter/Escape parity is required |
| Successful packing | not-touched | out of scope | use existing packing reports |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the scoped path live in standard YR? -> Yes; offline Skirmish shell pumps dialog result `0x617/0x5C0` and routes `WM_COMMAND` to `0x006ACEE0`.` (evidence: `0x006AE2C0`, `0x006AE3F0`)
- `[RESOLVED] OQ-02 - Which ordinary failures block Start? -> capacity overflow, fewer than two players, and all active players on one explicit team.` (evidence: `0x006AD05B..0x006AD2A7`)
- `[RESOLVED] OQ-03 - What text appears? -> capacity uses `TXT_SCENARIO_TOO_SMALL`, min-player uses `TXT_NEED_AT_LEAST_TWO_PLAYERS`, same-team uses `TXT_CANNOT_ALLY`, all with second text `TXT_OK`.` (evidence: prior text report; call sites `0x006AD073..0x006AD293`)
- `[RESOLVED] OQ-04 - Which modal controls receive text? -> `0x005D3490` sends `0x4B2` to children `0x5B0` and `0x5AE`; optional controls `2` and `0x5AF` are only written for non-empty optional args.` (evidence: decompile `0x005D3490`)
- `[RESOLVED] OQ-05 - Does native disable and re-enable Start? -> Yes; disable before validation, re-enable after modal and before return.` (evidence: `0x006ACF92..0x006ACF9E`, `0x006AD0CB..0x006AD32A`)
- `[RESOLVED] OQ-06 - Does native pack/start after ordinary failure? -> No; ordinary failure branches return before `0x006AD34B` packing/result writes.` (evidence: decompile `0x006ACEE0`)
- `[RESOLVED] OQ-07 - Is same-team a start-position collision check? -> No; it reads team controls and skips when local team is negative/None.` (evidence: `0x006AD16C..0x006AD2A7`)
- `[RESOLVED] OQ-08 - Does selected-mode false always show the generic modal? -> No; only false plus output dword `0x617` blocks.` (evidence: `0x006AD2D5..0x006AD346`)
- `[RESOLVED] OQ-09 - Does current Rust have ordinary data validation? -> Yes.` (evidence: `src/ui/skirmish_shell/state.rs:1853..1889`)
- `[RESOLVED] OQ-10 - Does current Rust set a visible validation modal on native ordinary Start failures? -> Yes; prior current-contract wording is stale.` (evidence: `src/app.rs:619..708`)
- `[RESOLVED] OQ-11 - Does current Rust render validation modal text/instances? -> Yes, via primitive modal draw/text paths.` (evidence: `src/app_skirmish_shell_render.rs:1345..1372`, `2304..2329`, `2599..2688`)
- `[RESOLVED] OQ-12 - Does current Rust block parent shell mouse/wheel behind the modal? -> Yes; modal state consumes mouse down/up, mouse wheel, status hover, and global Escape shell-close path.` (evidence: `src/app.rs:1059..1077`, `1166..1192`, `1419..1425`)
- `[DEFERRED] OQ-13 - Exact native modal pixels/resource rectangles for `0x005D3490`.` (category: out-of-scope; reason: this slot verifies failure UI contract and current Rust deltas, not native visual reconstruction; next-step-if-pursued: resource/template extraction plus screenshot comparison)
- `[DEFERRED] OQ-14 - Validation-modal Enter/Escape/default-button keyboard parity.` (category: requires-different-system-context; reason: `0x005D3490` pump is verified but its dialog proc/button command mapping was not drained; next-step-if-pursued: trace modal resource/proc commands for controls `2` and `0x5AF`)
- `[DEFERRED] OQ-15 - Custom/modded MPModes object intentionally writing output dword `0x617`.` (category: out-of-scope; reason: stock/local mode sweep already exists; next-step-if-pursued: investigate custom MPModes construction/extension path)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Capacity overflow shows `TXT_SCENARIO_TOO_SMALL` formatted with capacity plus `TXT_OK`, re-enables Start, and does not launch | `0x006AD05B..0x006AD0DA`; prior text report | mostly implemented functionally; missing tests and native modal visual parity | `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs` | keep CSF message mapping, prove modal draw and no launch with tests, later replace primitive modal geometry/art with native-verified layout | map capacity `2`, local plus two enabled AIs: visible modal says map has `2 player max`, OK dismisses, no pending launch/session transition, Start can be clicked again | Do not append non-native requested/capacity suffix; proposed tests `skirmish_start_capacity_error_sets_validation_modal_text`, `skirmish_capacity_modal_ok_allows_retry` |
| No enabled opponent shows `TXT_NEED_AT_LEAST_TWO_PLAYERS` plus `TXT_OK` and does not launch | `0x006AD0ED..0x006AD159`; prior text report | functionally mapped; missing app/render acceptance tests | same | preserve native min-player text and modal blocking behavior | all AI rows disabled: shell remains, modal text is `You need at least two players to start the game!`, OK clears modal | Do not reduce this to log-only; proposed test `skirmish_start_no_opponent_modal_uses_native_text` |
| Same explicit team shows `TXT_CANNOT_ALLY` plus `TXT_OK`; local Team None skips this branch | `0x006AD16C..0x006AD2A7` | functionally mapped; missing app/render acceptance tests | `src/ui/skirmish_shell/state.rs`, `src/app.rs` | preserve Team None skip and same-explicit-team modal text | local team 1 and all active AIs team 1 blocks; local Team None with same AI teams does not block for this reason | Do not implement as start-position collision; proposed test `skirmish_start_same_explicit_team_modal_uses_cannot_ally_text` |
| Modal blocks parent shell input until OK/result, then Start is usable again | `0x005D3490`; re-enable ranges `0x006AD0CB..0x006AD32A` | Rust consumes input and OK clears modal, but does not model native disabled `0x617`; test coverage missing | `src/app.rs`, owner-draw button state/render | prove double-click/behind-modal input cannot start or mutate shell and OK permits retry | invalid setup, double-click Start: one modal, no combo/dropdown/launch side effects; OK then Start shows modal again | Do not leave Start permanently disabled; proposed tests `skirmish_validation_modal_consumes_parent_clicks`, `skirmish_validation_modal_ok_clears_and_retry_works` |
| Selected-mode rejection blocks only when `+0x14` false also writes output dword `0x617`; stock/local modes do not hit this path | `0x006AD2BA..0x006AD34B`; prior mode sweep | missing mode acceptance model; no immediate stock path delta | future MPModes model and `launch_session` validation | preserve strict native gate when selected-mode acceptance is added | synthetic mode false+`0x617` blocks with mode output body and OK; false+zero is not treated as this modal | Do not make every false mode return a blocking modal; proposed test `skirmish_mode_rejection_blocks_only_native_start_code` |

## 8. Negative Facts / Do Not Do

- Do not say current Rust is still log-only for native ordinary Start validation failures. Active in YR comparison: Yes; current Rust sets and renders `validation_modal`. Evidence: `src/app.rs:619..708`; `src/app_skirmish_shell_render.rs:2599..2688`.
- Do not transition to loading, pack a session, or write a launch result after ordinary native validation failures. Active in YR: Yes. Evidence: ordinary branches return before `0x006AD34B`.
- Do not leave Start unusable after a validation failure. Active in YR: Yes. Evidence: re-enable ranges `0x006AD0CB..0x006AD32A`.
- Do not treat same-team validation as start-position collision. Active in YR: Yes. Evidence: `0x006AD16C..0x006AD2A7` reads team controls.
- Do not make every selected-mode false return a blocking modal. Active in YR: Conditional. Evidence: `0x006AD2D9..0x006AD346` gates the modal on output dword `0x617`.
- Do not claim primitive Rust modal layout is native-pixel verified. Active in YR comparison: Yes; Rust currently uses a hand-built centered panel/button layout. Evidence: `src/ui/skirmish_shell/layout.rs:849..867`, `src/app_skirmish_shell_render.rs:1345..1372`.

## 9. Remaining Uncertainty

- Exact `0x005D3490` modal dialog resource/control rectangles and final pixels remain visual parity work.
- Native keyboard behavior for validation modal Enter/Escape/default buttons is not fully proven by this slot.
- Custom/modded MPModes objects that intentionally write output dword `0x617` remain outside this retail/local recheck.

## 10. Stale Docs / Replacement Wording

- In `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_GHIDRA_REPORT.md`, replace: "It does not currently set `validation_modal` on Start failure, does not draw validation modal instances/text, and only logs `LaunchValidationError` from the Start action path." with: "Current Rust now maps `MapCapacityExceeded`, `NoEnabledOpponent`, and `SameExplicitTeam` to `SkirmishValidationModalState`, renders a primitive validation modal, consumes parent shell input while it is open, and dismisses it via OK mouse-up. Remaining deltas are native modal visual/template parity, focused app/render tests, native disabled Start timing, validation-modal keyboard parity, and future selected-mode `+0x14` output handling."
- In `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_GHIDRA_REPORT.md`, replace the current Rust status rows for `src/app.rs`, `src/ui/skirmish_shell/layout.rs`, and `src/app_skirmish_shell_render.rs` with the rows in section 4 of this report.
- Keep prior text mapping wording from `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`; it remains current for the three ordinary native failures and `TXT_OK`.

## Sources

- Ghidra read-only decompile/assembly: `0x006ACEE0`, `0x005D3490`, `0x006AE2C0`, `0x006AE3F0`, `0x007B7100`; assembly contexts around `0x006ACF7B`, `0x006AD05B`, `0x006AD0ED`, `0x006AD16C`, `0x006AD2BA`, `0x005D3490`.
- Prior reports: `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`.
- INI checked: `ini/mpmodesmd.ini` stock/local roster.
- Rust scanned: `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, `src/skirmish_launch.rs`.
