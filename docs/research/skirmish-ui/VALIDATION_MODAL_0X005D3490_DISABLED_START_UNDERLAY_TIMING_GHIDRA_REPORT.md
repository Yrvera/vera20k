# Validation Modal 0x005D3490 Disabled Start Underlay Timing - Ghidra Report

**Address(es):** `0x006ACEE0`, `0x005D3490`, `0x006AE2C0`, `0x006AE3F0`, `0x00612B70`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Standard YR offline Skirmish Start command `0x617` timing for disabling/re-enabling the Start child HWND around blocking validation modal `0x005D3490`, including whether the Start disabled state is forced to paint behind the modal.
**Non-Scope:** Broad trackbar/checkbox disabled states, successful launch/session packing, modal template geometry, modal bitmap composition, and validation-modal keyboard/default-button behavior.
**Confidence:** High for disable/re-enable order and branch liveness; Medium for visual observability because exact runtime repaint/screenshot evidence is not available from static binary alone.
**Active in YR:** Yes for the standard offline Skirmish shell `0x102` Start command path; Conditional for visible disabled underlay, depending on whether a parent paint occurs while modal `0x005D3490` is active.

## 0. Working Notes

- Target question: Does native standard YR disable Start `0x617` before the validation modal, is that disabled state painted under/behind the modal, and exactly when is Start re-enabled on each ordinary failure branch?
- Non-goals: Do not investigate broad trackbar/checkbox disabled flow, successful launch packing, modal resource rectangles/pixels, or keyboard/default dismissal.
- Evidence needed to mark COMPLETE: decompile plus assembly/caller evidence for path liveness, disable order, modal call order, re-enable order per failure branch, and owner-draw disabled paint reachability; explicitly defer only screenshot/runtime-only visibility.
- Stop conditions: Stop after every scoped Start failure branch has a disable -> modal -> re-enable classification and current Rust handoff; write only this report plus the shared claims row.

## 1. Overview

Native `gamemd.exe` disables Start `0x617` immediately after accepting a Start `WM_COMMAND` with notification `0`, before counting active players or evaluating any Start validation predicate. Active in YR: Yes. Evidence: `0x006AE2C0` creates/pumps offline Skirmish dialog `0x102`; `0x006AE3F0` routes `WM_COMMAND 0x111` to `0x006ACEE0`; assembly `0x006ACF8C..0x006ACF9E` performs `GetDlgItem(hwnd, 0x617)` and `EnableWindow(..., 0)`.

Every scoped blocking branch calls modal helper `0x005D3490` while Start is disabled, then re-enables Start with `EnableWindow(..., 1)` after the modal returns and before returning from the Start handler. Active in YR: Yes. Evidence: capacity `0x006AD0C6..0x006AD0DA`; min-player `0x006AD145..0x006AD159`; same-team `0x006AD293..0x006AD2A7`; selected-mode-output-`0x617` `0x006AD316..0x006AD32A`.

The Start disabled visual is a real owner-draw state if a paint occurs while the HWND has `WS_DISABLED`; however, the Start handler itself does not call `InvalidateRect` or `UpdateWindow` between the disable call and the modal helper. Active in YR: Conditional. Evidence: `OwnerDraw_Button_00612B70 @ 0x00612B70` reads style via `GetWindowLongA(..., GWL_STYLE)` and tests `0x08000000` in paint; Start handler assembly shows direct validation/modal work after `0x006ACF9E` with no explicit repaint call before modal branches.

## 2. Entry And Liveness

| Finding | Evidence | Active in YR |
|---|---|---|
| Offline Skirmish creates and pumps dialog `0x102` until result `0x617` or `0x5C0`. | decompile `0x006AE2C0` | Yes |
| Dialog proc `0x006AE3F0` handles `WM_COMMAND 0x111` by calling `0x006ACEE0`. | decompile `0x006AE3F0`; command branch calls `FUN_006acee0(param_4, param_3 >> 0x10)` | Yes |
| Start/Back command handler ignores nonzero notification and enters Start branch only for `param_2 == 0x617`, `param_4 == 0`. | decompile `0x006ACEE0`; assembly `0x006ACF7B..0x006ACF92` | Yes |
| The Start button is a subclassed/owner-draw Button control in standard `0x102`, so `WS_DISABLED` can affect its paint path. | prior `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`; decompile `0x00612B70` | Yes |

## 3. Disable And Re-Enable Timing

### 3.1 Initial Start Disable

On Start command `0x617` with notification `0`, `0x006ACEE0` immediately calls:

1. `GetDlgItem(parent_hwnd, 0x617)`
2. `EnableWindow(start_hwnd, 0)`

This happens before `CDFileClass__Constructor()`, before active-row counting, before map-capacity/min-player/team checks, and before selected-mode acceptance. Active in YR: Yes. Evidence: decompile `0x006ACEE0`; assembly `0x006ACF8C..0x006ACF9E` followed by count setup at `0x006ACFA4..0x006AD052`.

### 3.2 Capacity Failure

When selected map capacity is less than `active_ai_count + 1`, the branch formats the capacity message, calls `0x005D3490`, then re-enables Start:

- Modal call: `CALL 0x005d3490` at `0x006AD0C6`.
- Re-enable setup/call: `PUSH 0x1`, `PUSH 0x617`, `PUSH parent`, `GetDlgItem`, `EnableWindow` at `0x006AD0CB..0x006AD0DA`.
- Return: `0x006AD0E0..0x006AD0EA`.

Active in YR: Yes. Evidence: decompile `0x006ACEE0`; assembly context `0x006AD05B..0x006AD0DA`.

### 3.3 Fewer Than Two Total Players

When `active_ai_count + 1 < 2`, the branch loads min-player text, calls `0x005D3490`, then re-enables Start:

- Modal call: `CALL 0x005d3490` at `0x006AD145`.
- Re-enable: `0x006AD14A..0x006AD159`.
- Return: `0x006AD15F..0x006AD169`.

Active in YR: Yes. Evidence: decompile `0x006ACEE0`; assembly context `0x006AD0ED..0x006AD159`.

### 3.4 Same Explicit Team

When the local team is nonnegative and every active AI row has the same explicit team, the branch loads cannot-ally text, calls `0x005D3490`, then re-enables Start:

- Same-team loop failure exit to modal: `0x006AD23C..0x006AD293`.
- Modal call: `CALL 0x005d3490` at `0x006AD293`.
- Re-enable: `0x006AD298..0x006AD2A7`.
- Return: `0x006AD2AD..0x006AD2B7`.

Active in YR: Yes. Evidence: decompile `0x006ACEE0`; assembly context `0x006AD16C..0x006AD2A7`.

### 3.5 Selected-Mode Output `0x617`

The selected MPMode/category callback can reject Start. It blocks only when the callback returns false and writes output dword `0x617`; that branch calls `0x005D3490`, then re-enables Start:

- Callback: vtable `+0x14` call at `0x006AD2D2`.
- Gate: `TEST AL,AL`; then `CMP [ESP+0x1c],0x617` at `0x006AD2D5..0x006AD2E1`.
- Modal call: `CALL 0x005d3490` at `0x006AD316`.
- Re-enable: `0x006AD31B..0x006AD32A`.
- Cleanup/return: `0x006AD330..0x006AD343`.

Active in YR: Conditional. Evidence: decompile/assembly `0x006AD2BA..0x006AD34B`. Condition is selected mode acceptance callback returns false and output dword equals literal `0x617`.

## 4. Repaint / Visibility Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| The Start handler does not explicitly repaint the disabled Start button before the modal call. | Assembly from `0x006ACF92..0x006AD316` shows `EnableWindow(FALSE)` followed by validation work and modal calls; no `InvalidateRect`, `UpdateWindow`, or parent paint call is present in the Start handler before `0x005D3490`. | Yes as a negative binary fact |
| `0x005D3490` enters a blocking modal pump after creating/showing its dialog; parent messages can be pumped while the modal is active. | decompile `0x005D3490`: after `FUN_00622800`, it loops on `FUN_00623120()` and calls `FUN_00532100()` while local result is negative. | Yes |
| A disabled visual can only appear if the Start owner-draw paint executes while `WS_DISABLED 0x08000000` is set. | decompile `OwnerDraw_Button_00612B70`: paint path obtains style via `GetWindowLongA(..., -0x10)`, uses disabled-style tests, and applies disabled overlay/text-color logic around `0x006135FD..0x0061361B`. | Conditional |
| The owner-draw button proc does not install a special `WM_ENABLE` repaint branch of its own; `WM_ENABLE 0x0A` falls through to the original window proc. | decompile `0x00612B70`: explicit cases include `WM_TIMER 0x113`, `WM_PAINT 0x0F`, mouse down/double-click, and custom `0x4DC`; `WM_ENABLE` is not special-cased. | Yes |
| Static analysis alone cannot prove that the player sees a disabled Start frame before, beneath, or after the validation modal. | Combined evidence above: no explicit pre-modal repaint; modal pump may process paints; exact modal coverage/retail screenshot belongs to other slots/runtime capture. | Conditional / runtime-only |

## 5. Current Rust Implementation Status

| Rust surface | Current status | Delta vs verified native timing |
|---|---|---|
| `src/app.rs:657..668` | On Start error, Rust creates `validation_modal` and requests redraw. | Rust does not model a native HWND disable before validation; it moves directly to modal state. |
| `src/app.rs:738..749` | `show_skirmish_validation_modal` sets modal, clears pressed/dropdown/drag/edit state. | Good functional blocking state; no separate pre-modal disabled interval exists. |
| `src/app.rs:1154..1170` | Mouse down is consumed while modal exists; OK mouse-up clears modal. | Parent input blocking matches the important effect; native re-enable timing maps to modal clear/retry. |
| `src/app.rs:1510..1516` | Escape while validation modal exists is consumed and does not close shell. | Keyboard dismissal parity belongs to slot 3, not this report. |
| `src/app_skirmish_shell_render.rs:253..260` | Start button is rendered disabled whenever `validation_modal.is_some()`. | This may overstate native visibility: native Start is disabled during modal, but no explicit disabled-underlay paint is proven. |
| `src/app_skirmish_shell_render.rs:307..310`, `437..439` | Validation modal instances/text draw after shell button instances/text setup. | Functionally compatible with "disabled under modal if shell draws first"; exact native pixels are not proven. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes and scope | verified | section 0 | none |
| Dialog liveness `0x006AE2C0` | verified | decompile | none |
| Command dispatch `0x006AE3F0` | verified | decompile | none |
| Start initial disable | verified | `0x006ACF8C..0x006ACF9E` | none |
| Capacity failure re-enable | verified | `0x006AD0C6..0x006AD0DA` | none |
| Min-player failure re-enable | verified | `0x006AD145..0x006AD159` | none |
| Same-team failure re-enable | verified | `0x006AD293..0x006AD2A7` | none |
| Selected-mode output `0x617` re-enable | verified | `0x006AD316..0x006AD32A` | callback implementations out of scope |
| Explicit pre-modal repaint | verified-negative | `0x006ACF92..0x006AD316` | none |
| Modal pump can process messages | verified | `0x005D3490` | exact dialog proc/key behavior belongs to slot 3 |
| Owner-draw disabled paint support | verified | `0x00612B70`; prior button reports | exact disabled pixels/assets belong to paint-composition work |
| Disabled Start underlay actually visible in retail frame capture | deferred | static binary cannot screenshot | runtime debugger/screenshot capture |
| Current Rust surfaces | verified | Codegraph/source scan | app/render tests |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this path live in standard YR? -> Yes; offline Skirmish dialog `0x102` pumps through `0x006AE2C0`, `0x006AE3F0`, and Start command `0x006ACEE0`.` (evidence: `0x006AE2C0`, `0x006AE3F0`)
- `[RESOLVED] OQ-02 - Is Start disabled before validation? -> Yes; `EnableWindow(GetDlgItem(hwnd,0x617),0)` runs before validation counting and predicates.` (evidence: `0x006ACF8C..0x006ACF9E`)
- `[RESOLVED] OQ-03 - Is capacity failure re-enabled? -> Yes, after `0x005D3490` returns and before branch return.` (evidence: `0x006AD0C6..0x006AD0DA`)
- `[RESOLVED] OQ-04 - Is min-player failure re-enabled? -> Yes, after `0x005D3490` returns and before branch return.` (evidence: `0x006AD145..0x006AD159`)
- `[RESOLVED] OQ-05 - Is same-team failure re-enabled? -> Yes, after `0x005D3490` returns and before branch return.` (evidence: `0x006AD293..0x006AD2A7`)
- `[RESOLVED] OQ-06 - Is selected-mode output `0x617` re-enabled? -> Yes, after `0x005D3490` returns, then callback cleanup runs before return.` (evidence: `0x006AD316..0x006AD334`)
- `[RESOLVED] OQ-07 - Does native force a repaint before modal creation? -> No explicit repaint/update call exists between disable and modal calls in the Start handler.` (evidence: `0x006ACF92..0x006AD316`)
- `[RESOLVED] OQ-08 - Can the owner-draw button display disabled state? -> Yes, if painted while `WS_DISABLED` is set.` (evidence: `0x00612B70`, style test `0x08000000`)
- `[RESOLVED] OQ-09 - Does the owner-draw button special-case WM_ENABLE? -> No; `WM_ENABLE 0x0A` falls through to the original window proc, not a local custom repaint branch.` (evidence: decompile `0x00612B70`)
- `[RESOLVED] OQ-10 - Does modal helper pump messages while Start is disabled? -> Yes, `0x005D3490` runs a blocking modal pump until result changes.` (evidence: decompile `0x005D3490`)
- `[RESOLVED] OQ-11 - Should Rust pre-grey invalid setups before Start click? -> No; native disables only after Start command notification `0`.` (evidence: `0x006ACF7B..0x006ACF9E`)
- `[RESOLVED] OQ-12 - Does Rust currently keep Start usable after modal OK? -> Yes; modal clear removes the rendered disabled state and Start action can run again.` (evidence: `src/app.rs:1158..1170`, `src/app_skirmish_shell_render.rs:253..260`)
- `[DEFERRED] OQ-13 - Is the disabled Start underlay visibly seen by a player under/around the native modal?` (category: `needs-runtime-debugger`; reason: static binary proves state and possible paint, but not actual frame visibility or modal coverage; next-step-if-pursued: retail screenshot or debugger capture during invalid Start modal)
- `[DEFERRED] OQ-14 - Exact disabled button pixel/frame used under modal.` (category: `requires-different-system-context`; reason: belongs to paint-composition/button visual asset slot; next-step-if-pursued: drain `OwnerDraw_Button_00612B70` SDBTNANM frame/color path plus screenshot)

## 8. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `0x006ACEE0` | Start `0x617`, notify `0`; `0x006ACF8C..0x006ACF9E` | n/a | child HWND `0x617` | n/a | Yes | disable Start HWND |
| 2 | `0x006ACEE0` | validation failure branch | n/a | n/a | n/a | Yes/Conditional | calls blocking modal |
| 3 | `0x005D3490` | any non-empty second text argument in scoped failures | modal dialog resource | child text controls `0x5B0`, `0x5AE` | shell dialog path | Yes | modal overlay/pump |
| 4 | `OwnerDraw_Button_00612B70` | only if Start receives `WM_PAINT` while `WS_DISABLED` set | SDBTNANM/button assets, exact frame out of scope | Start `0x617` rect from prior child-rect reports | disabled style path | Conditional | disabled underlay |
| 5 | `0x006ACEE0` | after modal returns in failure branch | n/a | child HWND `0x617` | n/a | Yes | re-enable Start HWND |

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| Start button owner-draw assets | yes | conditional | conditional during modal | no | yes | possible disabled underlay | no | no | `0x00612B70`; prior SDBTNANM reports |
| Native `0x005D3490` modal resource | yes | yes | yes | no | yes | yes | no | no | decompile `0x005D3490` |
| Rust disabled Start under modal | yes | yes when `validation_modal.is_some()` | yes in Rust | no | yes | under modal | no | no | `src/app_skirmish_shell_render.rs:253..260`, `307..310` |

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Start is not disabled in idle/invalid setup; it disables only after Start `0x617` command notification `0`. | `0x006ACF7B..0x006ACF9E` | Rust currently matches by not pre-disabling invalid idle setups | `src/app.rs`, `src/app_skirmish_shell_render.rs` | Keep Start clickable even when validation will fail | Configure all AIs inactive: Start button remains visually enabled before click, click opens validation modal | Do not pre-grey invalid setups; proposed test `skirmish_invalid_setup_keeps_start_enabled_until_click` |
| Each blocking failure keeps Start disabled while `0x005D3490` runs, then re-enables it after modal return and before returning from handler. | `0x006AD0C6..0x006AD0DA`, `0x006AD145..0x006AD159`, `0x006AD293..0x006AD2A7`, `0x006AD316..0x006AD32A` | Rust approximates this with `validation_modal.is_some()` disabling render/input, then clears on OK; no separate HWND-style state needed unless finer tests require it | `src/app.rs`, `src/app_skirmish_shell_render.rs` | Preserve modal-time Start input block and immediate retry after OK | Invalid setup, double-click Start: only one modal/no launch; OK clears modal; second Start click opens modal again | Do not leave Start disabled after OK; proposed test `skirmish_validation_modal_ok_reenables_start_retry` |
| Native does not force a disabled Start repaint before modal creation; visible disabled underlay is conditional on message pumping/paint while modal is active. | negative assembly scan `0x006ACF92..0x006AD316`; `0x005D3490` modal pump; `0x00612B70` disabled paint style test | Rust currently always renders Start disabled under the modal overlay; acceptable for functional blocking, but should not be treated as screenshot-proven native visual parity | `src/app_skirmish_shell_render.rs` | If visual parity work gets strict, gate disabled-underlay claims behind runtime screenshot/template evidence | Render-order test may assert Start draw precedes modal overlay, but not that native visibly exposes the disabled frame | Do not spend implementation effort on exact underlay pixels before modal template/paint slots finish; proposed test `skirmish_validation_modal_blocks_start_without_requiring_prepaint` |

## 10. Negative Facts / Do Not Do

- Do not pre-disable Start based on an invalid setup before the user clicks Start. Active in YR: Yes. Evidence: disable occurs only inside Start command branch `0x006ACF8C..0x006ACF9E`.
- Do not leave Start disabled after capacity, min-player, same-team, or selected-mode-output-`0x617` modal dismissal. Active in YR: Yes. Evidence: re-enable calls at `0x006AD0CB..0x006AD32A`.
- Do not claim native explicitly repaints the parent disabled Start before showing `0x005D3490`. Active in YR: No. Evidence: no explicit repaint/update call between `0x006ACF9E` and modal calls in scoped branches.
- Do not use trackbar/checkbox disabled-flow reports to justify Start behavior. Active in YR: No for this target. Evidence: Start uses direct `EnableWindow` on child `0x617`; scoped trackbar report says option trackbars stay enabled.
- Do not treat Rust's always-rendered disabled Start under a modal as native-pixel verified. Active in YR comparison: Yes. Evidence: Rust render gate at `src/app_skirmish_shell_render.rs:253..260`; native visibility remains runtime-conditional.

## 11. Remaining Uncertainty

- Retail-frame observability of the disabled Start underlay is not statically proven. A runtime screenshot/debugger capture during an invalid Start validation modal is required to prove whether the player can actually see any disabled Start pixels outside or under the modal.
- Exact disabled Start button frame/color under `WS_DISABLED` belongs to the button/modal paint-composition slots; this report verifies the timing and state, not the final pixels.

## 12. Stale Docs / Follow-up Wording

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`: replace "Start is not modeled as a native disabled control during the modal" with "Rust models the native modal-time disabled Start effect by rendering/input-blocking Start while `validation_modal.is_some()`, but it does not model a separate HWND-style disable before validation. Native disables Start after Start command notification `0`, calls the blocking modal on failures, and re-enables Start after the modal returns."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_FAILURE_SHELL_MODAL_CURRENT_RECHECK_GHIDRA_REPORT.md`: replace "it does not model native transient `EnableWindow(0/1)`" with "it approximates native `EnableWindow(0/1)` by tying disabled rendering/input to `validation_modal.is_some()`; native disables before validation and re-enables after modal return, but static evidence does not prove a forced pre-modal disabled repaint."

## Sources

- Ghidra read-only decompile/assembly: `0x006ACEE0`, `0x005D3490`, `0x006AE2C0`, `0x006AE3F0`, `0x00612B70`.
- Prior reports referenced: `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_START_FAILURE_SHELL_MODAL_CURRENT_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`, `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`.
- Rust scanned: `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/state/hit_test.rs`.
