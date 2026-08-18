# Validation Modal 0x005D3490 Keyboard/Default Dismissal - Ghidra Research Report

**Address(es):** `0x005D3490`, `0x005D36A0`, `0x005D4D50`, `0x005D4E70`, `0x00622650`, `0x00622B50`, dialog resources `0xCE`, `0x120`, `0x121`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Enter/Escape/default-button dismissal for the native shell modal helper used by Skirmish Start validation failures at `0x005D3490`.  
**Non-Scope:** Choose Map keyboard behavior, parent `0x102` Escape-to-Back, modal pixel/chrome composition, Start validation trigger/text rediscovery, and successful launch/session packing.  
**Confidence:** High for active message-pump registration, `IsDialogMessageA` use, command IDs/results, and resource button IDs/styles; Medium for exact user-visible focus highlight because no live runtime key capture was taken.  
**Active in YR:** Yes for standard offline Skirmish validation modal calls; conditional for optional two/three-button variants.

## 0. Working Notes

- Target question: Does the native `0x005D3490` Start-validation modal dismiss via Enter/Escape/default-button behavior, and through which command IDs/results?
- Non-goals: Do not investigate Choose Map keyboard behavior, parent-shell global Escape, visual pixels, resource rectangles beyond keyboard/default implications, or validation failure text/conditions.
- Evidence needed to mark COMPLETE: decompile plus assembly for modal helper/proc, shell dialog registration, modal/message pump `IsDialogMessageA` path, resource control IDs/styles for the active dialog template, and current Rust keyboard scan.
- Stop conditions: stop when Enter, Escape, OK mouse, optional command IDs, current Rust delta, negative facts, and remaining uncertainties are all classified without mutating Ghidra or Rust.

## 1. Overview

`FUN_005D3490` creates a modeless shell dialog through `FUN_00622650`, registers that HWND in the shell dialog-message list, shows it, then loops until a stack result dword changes. During that loop `Process_NetworkMessages @ 0x005D4D50` calls `IsDialogMessageA` for registered shell dialogs before ordinary translation/dispatch, so dialog-key handling is active for this modal. Active in YR: Yes. Evidence: `0x005D3541` calls `0x00622650`; `0x00622650` calls `0x005D4E70`; `0x005D4DDB..0x005D4DEE` calls `IsDialogMessageA`; `0x005D3628..0x005D364F` loops via `0x00623120`; `0x00623120` calls `Process_NetworkMessages`.

For the one-button Start-validation modal, the native dialog resource is `0xCE`. It has only child `0x5B0` static text and child `0x5AE` owner-draw button texted as OK. It has no `IDOK (1)`, no `IDCANCEL (2)`, and no default pushbutton style in the resource, but `IsDialogMessageA` translates Enter/Escape dialog keys into `WM_COMMAND` IDs that the modal proc explicitly handles: command ID `1` or `2` with notification `0` sets result `1`; command ID `0x5AE` with notification `0` sets result `0`. Active in YR: Yes. Evidence: dialog resource `0xCE`; modal proc `0x005D36ED..0x005D3735`.

## 2. Native Dialog Template / Key-Relevant Controls

| Dialog | Selection condition in `0x005D3490` | Controls | Key/default implication | Active in YR |
|---|---|---|---|---|
| `0xCE` | no optional third/fourth text; ordinary Start validation calls use this | `0x5B0` static, `0x5AE` owner-draw OK button | no resource `IDOK/IDCANCEL`; dialog proc still accepts translated `IDOK/IDCANCEL` from `IsDialogMessageA` | Yes |
| `0x120` | third text non-empty, fourth empty | `2` owner-draw Cancel, `0x5B0` static, `0x5AE` owner-draw OK | Escape/IDCANCEL and explicit control `2` converge to result `1`; OK `0x5AE` returns `0` | Conditional, not used by ordinary Start validation |
| `0x121` | fourth text non-empty | `2` owner-draw Cancel, `0x5B0` static, `0x5AE` owner-draw OK, `0x5AF` owner-draw third button | third button `0x5AF` returns result `2`; Escape/IDCANCEL returns result `1` | Conditional, not used by ordinary Start validation |

Resource evidence from `gamemd.exe` `RT_DIALOG`:

| Dialog | Style | Control | Class | Text | Style | Rect | Active in YR |
|---|---|---|---|---|---|---|---|
| `0xCE` | `0x40000040` | `0x5B0` | static `0x82` | `GUI:Blank` | `0x50000000` | `(40,40,220,50)` | Yes |
| `0xCE` | `0x40000040` | `0x5AE` | button `0x80` | `GUI:OK` | `0x5000000B` owner-draw | `(207,175,83,15)` | Yes |
| `0x120` | `0x40000040` | `2` | button `0x80` | `GUI:Cancel` | `0x5000000B` owner-draw | `(207,175,83,15)` | Conditional |
| `0x120` | `0x40000040` | `0x5AE` | button `0x80` | `GUI:OK` | `0x5000000B` owner-draw | `(207,155,83,15)` | Conditional |
| `0x121` | `0x40000040` | `0x5AF` | button `0x80` | `GUI:Blank` | `0x5000000B` owner-draw | `(207,155,83,15)` | Conditional |

No parsed modal button uses `BS_DEFPUSHBUTTON`; the low style nibble is `0xB` (`BS_OWNERDRAW`). Active in YR: Yes/Conditional as above. Evidence: resource parser over retail `gamemd.exe`.

## 3. Core Keyboard / Command Logic

`FUN_005D3490` selects dialog resource `0xCE`, `0x120`, or `0x121` before creation. For ordinary Start validation, only `param_1` and `param_2` are non-empty, so `ECX` remains `0xCE`, the dialog is created with proc `0x005D36A0`, `param_1` is sent to child `0x5B0`, and `param_2`/`TXT_OK` is sent to child `0x5AE`. Active in YR: Yes. Evidence: `0x005D351D..0x005D3541`, `0x005D3573..0x005D35AE`; Start failure reports show calls pass zero optional texts.

`FUN_00622650` registers every successfully created shell dialog in the `DAT_00ABFC94` list via `FUN_005D4E70`. `Process_NetworkMessages` loops pending messages and calls `IsDialogMessageA(registered_hwnd, &msg)` before accelerator translation and before `TranslateMessage`/`DispatchMessageA`. If `IsDialogMessageA` returns nonzero, the message is consumed and ordinary dispatch is skipped. Active in YR: Yes. Evidence: decompile `0x00622650`; assembly `0x005D4EAE..0x005D4EC3` stores the HWND; assembly `0x005D4D8A` loads `IsDialogMessageA`, `0x005D4DDB..0x005D4DEE` calls it, and `0x005D4DEE` jumps to the consumed-message path.

The modal dialog proc handles only the command IDs, not raw key messages:

| Incoming message/command | Native action | Modal return/result | Active in YR |
|---|---|---|---|
| `WM_COMMAND`, low word `0x5AE`, high word `0` | writes `0` to stack result | dismisses with return `0` | Yes |
| `WM_COMMAND`, low word `1` or `2`, high word `0` | writes `1` to stack result | dismisses with return `1` | Yes |
| `WM_COMMAND`, low word `0x5AF`, high word `0` | writes `2` to stack result | dismisses with return `2` | Conditional |
| `WM_COMMAND` same IDs with nonzero notification | ignored | modal remains open | Yes/Conditional |
| raw `WM_KEYDOWN`, `WM_CHAR`, or `WM_KEYUP` | no local branch; delegated to common handler first, then falls through | no direct dismissal | Yes |

Evidence: decompile `0x005D36A0`; assembly `0x005D36C7..0x005D374A`.

Conclusion for the active one-button Start-validation modal: clicking the visible OK button (`0x5AE`) dismisses with result `0`; Enter and Escape are expected to dismiss through `IsDialogMessageA` as translated `IDOK (1)` / `IDCANCEL (2)` `WM_COMMAND`s, both producing result `1`. The Start validation caller does not inspect the modal return value, so the player-visible result is simply dismissal for all three actions. Active in YR: Yes. Evidence chain: `0x005D3490 -> 0x00622650 -> 0x005D4E70`; `0x005D3490 -> 0x00623120 -> 0x005D4D50`; modal proc command branch `0x005D36ED..0x005D3735`; resource `0xCE`.

## 4. Current Rust Implementation Status

| Rust surface | Current status | Delta vs native |
|---|---|---|
| `src/app.rs` validation modal mouse path | `handle_validation_modal_mouse_up` clears modal only when cursor is inside `layout.ok_button`; mouse down consumes while modal is open | OK mouse dismissal exists, but native command-result distinction is not modeled |
| `src/app.rs` keyboard branch | Escape is consumed while `validation_modal` exists and does not close the parent shell | parent suppression is correct, but native modal Escape dismissal is missing |
| `src/app.rs::handle_skirmish_shell_key_input` | handles player-name/edit keys only; no validation modal Enter path found | native Enter dismissal is missing |
| `src/ui/skirmish_shell/layout.rs` | one primitive `ok_button` rect | enough for current mouse behavior, but does not carry native command IDs `0x5AE`, `1`, `2`, `0x5AF` |

Active in YR comparison: current Rust delta only. Evidence: `rg validation_modal|Escape|Enter|KeyCode` scan and focused `src/app.rs` read.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes and scope | verified | section 0 | none |
| Modal helper resource selection and text/control writes | verified | `0x005D351D..0x005D35AE` | visual paint outside scope |
| Active Start-validation one-button resource `0xCE` | verified | resource parse plus prior Start failure call shape | none for keyboard |
| Optional resources `0x120/0x121` | verified-for-key-ids | resource parse; `0x005D3526..0x005D3535` | not active in ordinary Start validation |
| Shell dialog registration | verified | `0x00622650`, `0x005D4E70`, assembly `0x005D4EAE..0x005D4EC3` | none |
| `IsDialogMessageA` pump path | verified | `0x005D4D50`, assembly `0x005D4D8A..0x005D4DEE` | live key capture optional |
| Modal proc command IDs/results | verified | `0x005D36A0`, assembly `0x005D36ED..0x005D374A` | none |
| Raw key-message local handling | verified-negative | no `WM_KEYDOWN/WM_CHAR/WM_KEYUP` branch in `0x005D36A0` after common handler | none |
| Current Rust validation modal keyboard behavior | verified | `src/app.rs` scan | implement/test Enter/Escape dismissal if chosen |
| Choose Map keyboard | not-touched | out of scope | use existing Choose Map reports |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x005D3490` the active Start-validation modal helper? -> Yes; ordinary Start validation failure branches call it with message text, `TXT_OK`, and zero optional texts.` (evidence: prior Start validation reports; `0x005D3490` decompile)
- `[RESOLVED] OQ-02 - Which dialog resource is active for ordinary one-button Start validation? -> `0xCE`.` (evidence: `0x005D351D..0x005D3541`; resource parse)
- `[RESOLVED] OQ-03 - Does the modal HWND participate in `IsDialogMessageA`? -> Yes; `0x00622650` registers it and `Process_NetworkMessages` iterates the registered list before dispatch.` (evidence: `0x005D4E70`; `0x005D4D50`)
- `[RESOLVED] OQ-04 - Does the modal pump use `IsDialogMessageA` before ordinary dispatch? -> Yes; nonzero `IsDialogMessageA` skips accelerator/translate/dispatch.` (evidence: `0x005D4DDB..0x005D4DEE`)
- `[RESOLVED] OQ-05 - Does the active resource define default `IDOK` or `IDCANCEL` controls? -> No for `0xCE`; it contains only `0x5B0` and owner-draw button `0x5AE`.` (evidence: resource parse)
- `[RESOLVED] OQ-06 - Does the active resource define `BS_DEFPUSHBUTTON`? -> No; the button low style is owner-draw `0xB`.` (evidence: resource parse)
- `[RESOLVED] OQ-07 - What does clicking OK `0x5AE` do? -> `WM_COMMAND 0x5AE` with notification `0` writes modal result `0`.` (evidence: `0x005D3729..0x005D3735`)
- `[RESOLVED] OQ-08 - What does translated Enter/IDOK do? -> `WM_COMMAND 1` with notification `0` writes modal result `1`.` (evidence: `IsDialogMessageA` path plus `0x005D370D..0x005D3726`)
- `[RESOLVED] OQ-09 - What does translated Escape/IDCANCEL do? -> `WM_COMMAND 2` with notification `0` writes modal result `1`.` (evidence: `IsDialogMessageA` path plus `0x005D370D..0x005D3726`)
- `[RESOLVED] OQ-10 - Are nonzero command notifications accepted? -> No; all command-result branches require notification high word `0`.` (evidence: `0x005D3716..0x005D3718`, `0x005D3729..0x005D372B`, `0x005D3740..0x005D3742`)
- `[RESOLVED] OQ-11 - Does raw `WM_KEYDOWN/WM_CHAR` directly dismiss in the modal proc? -> No local key branch; dismissal depends on dialog-manager translation or button command delivery.` (evidence: `0x005D36C7..0x005D374A`)
- `[RESOLVED] OQ-12 - Does current Rust dismiss validation modal on Escape? -> No; it consumes Escape and returns without clearing the modal.` (evidence: `src/app.rs` keyboard branch scan)
- `[RESOLVED] OQ-13 - Does current Rust dismiss validation modal on Enter? -> No validation-modal Enter/default path found.` (evidence: `src/app.rs` keyboard scan)
- `[DEFERRED] OQ-14 - Exact focus rectangle / focused-child visual after `WM_INITDIALOG`.` (category: `needs-runtime-debugger`; reason: binary proves `SetFocus(dialog)` and return `0`, but the visible focus paint cannot be proven statically; next-step-if-pursued: runtime screenshot/key trace)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Enter dismisses the active one-button validation modal through dialog-manager `IDOK` translation, producing native modal result `1`; caller ignores the result | `0x005D4D50` `IsDialogMessageA`; `0x005D36ED..0x005D3726`; resource `0xCE` | missing | `src/app.rs` keyboard handling for `validation_modal` | Pressing Enter while the validation modal is open clears the modal and keeps the parent shell open/no launch | Open invalid Start modal, press Enter: modal closes, shell remains, no launch, Start can be clicked again | Do not route Enter to Start/launch behind the modal; proposed test `skirmish_validation_modal_enter_dismisses_without_launch` |
| Escape dismisses the active one-button validation modal through dialog-manager `IDCANCEL` translation, producing native modal result `1`; caller ignores the result | `0x005D4D50` `IsDialogMessageA`; `0x005D36ED..0x005D3726`; resource `0xCE` | mismatch: Rust consumes Escape but leaves modal open | `src/app.rs` keyboard branch before parent shell close | Pressing Escape while validation modal is open clears only the modal; it must not close the parent Skirmish shell | Open invalid Start modal, press Escape: modal closes, parent shell remains | Do not preserve current "consume but ignore" behavior; proposed test `skirmish_validation_modal_escape_dismisses_not_parent_shell` |
| Visible OK button `0x5AE` mouse command returns modal result `0`; optional `2`/`0x5AF` buttons are not present in ordinary Start validation `0xCE` | resource parse; `0x005D3729..0x005D3744` | current OK mouse dismissal is functionally aligned for one-button variant | `src/app.rs`, future native command-id model if optional shell modal is generalized | Keep OK mouse-up clearing the active validation modal; no need to model optional buttons for ordinary Start failures | Click OK on capacity/no-opponent/same-team modal: modal closes and shell remains | Do not add Cancel/third buttons to the ordinary Start-validation modal; proposed test `skirmish_validation_modal_ok_click_dismisses_one_button_modal` |

## 8. Negative Facts / Do Not Do

- Do not keep Escape as "consume but leave modal open" for validation modal parity. Active in YR: Yes for dismissal through translated `IDCANCEL`; evidence `0x005D4D50`, `0x005D370D..0x005D3726`.
- Do not route Escape to parent Back while the validation modal is open. Active in YR: No for parent-close leakage; the registered modal is handled by `IsDialogMessageA` first. Evidence: `0x005D4DDB..0x005D4DEE`.
- Do not add a visible Cancel button to ordinary Start validation failures. Active in YR: No; dialog `0xCE` contains only `0x5B0` and `0x5AE`. Evidence: resource parse.
- Do not implement global Enter as Start while the validation modal is open. Active in YR: No; translated `IDOK` is consumed by modal proc and writes modal result `1`. Evidence: `0x005D370D..0x005D3726`.
- Do not require `0x5AE` to be a default pushbutton for Enter dismissal. Active in YR: No; `0x5AE` is owner-draw style `0x5000000B`, while Enter is handled through the modal proc's IDOK branch. Evidence: resource parse plus `IsDialogMessageA` path.

## 9. Remaining Uncertainty

- Exact focused-control/focus-rectangle visual state after modal init requires runtime screenshot or debugger observation; it does not change the Enter/Escape dismissal handoff.

## 10. Stale Docs / Replacement Wording

- `docs/research/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`: replace "Native keyboard behavior for validation modal Enter/Escape/default buttons is not fully proven by this slot." with "Native validation-modal keyboard dismissal is now verified: `0x005D3490` creates/registers the modal through `0x00622650`/`0x005D4E70`; `Process_NetworkMessages @ 0x005D4D50` calls `IsDialogMessageA` for registered shell dialogs; the modal proc `0x005D36A0` handles translated `IDOK (1)` and `IDCANCEL (2)` `WM_COMMAND`s by setting modal result `1`. Rust should dismiss the validation modal on Enter and Escape while keeping the parent shell open."
- `docs/research/skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_GHIDRA_REPORT.md`: replace "native Enter/Escape/default-button behavior not proven" with the same wording above.

## Sources

- Ghidra read-only decompile/assembly: `0x005D3490`, `0x005D36A0`, `0x005D4D50`, `0x005D4E70`, `0x00622650`, `0x00622B50`, `0x00623120`.
- Retail resource parse from `<ra2-install>/gamemd.exe`: `RT_DIALOG` IDs `0xCE`, `0x120`, `0x121`.
- Prior reports referenced for active Start-validation call shape only: `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`.
- Rust scanned: `src/app.rs`, `src/ui/skirmish_shell/layout.rs`.
