# Skirmish Player Name Edit 0x6A0 Post-Implementation Audit - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x00614190`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** post-implementation audit of current Rust against standard offline Yuri's Revenge Skirmish dialog `0x102` player-name edit child `0x6A0`: focus, select-all, caret/selection, text input, 19-character cap, backspace/delete, Tab/Enter/Escape behavior, scroll/caret visibility, and launch readback.  
**Non-Scope:** Choose Map controls, status/help strip, online lobby edits, full shell-global accelerator manager, exact native mouse drag/clipboard/IME behavior, code patches.  
**Confidence:** High for setup/readback/cap/focus/select-all/Tab/current Rust source comparison; Medium for caret blink cadence and exact mouse caret placement because those need runtime or a separate old-Edit interaction trace.  
**Active in YR:** Yes for the standard offline Skirmish `0x102` path. Evidence: `FUN_006AE2C0` opens the shell and waits for `0x617`/`0x5C0`; `FUN_006AE6E0` initializes `0x6A0`; `FUN_006ACEE0` reads `0x6A0` on Start.

Working notes required before investigation:

- Target question: Does current Rust player-name edit behavior match native `0x6A0` after the recent implementation work?
- Non-goals: Do not patch Rust; do not investigate unrelated Skirmish controls; do not claim global Enter/Escape behavior without scoped evidence.
- Evidence needed to mark COMPLETE: prior `0x6A0` reports reconciled, Ghidra spot-checks for setup/readback/focus/Tab/Enter/Escape, current Rust source comparison, and Rust-facing handoff with test names.
- Stop conditions: stop after every scoped question is resolved/deferred; write only this report plus the shared claims file.

## 1. Overview

Current Rust has moved past the stale static-label gap. It now has first-class player-name edit state, focus/select-all, selection/caret state, insertion capped at 19 characters, backspace/delete, arrow/Home/End movement, Tab blur handling, scroll-to-caret, edit frame/selection/caret/text rendering, and launch-session readback from `player_name_edit.text`.

The remaining parity risks are narrower: Rust's Escape-to-close behavior is still not proven by the scoped native `0x102`/old-Edit evidence; current Tab behavior consumes Tab and blurs the edit but does not model the exact native next-tab target; mouse caret placement/drag selection and caret blink cadence remain approximate/untraced.

## 2. Verified Native Facts

| Finding | Active in YR | Evidence | Current Rust implication |
|---|---|---|---|
| Offline Skirmish `0x102` is the live shell, and the launcher exits only when the dialog result becomes `0x617` Start or `0x5C0` Back. | Yes | `FUN_006AE2C0` decompile: creates shell via `FUN_00622650`, loops until `local_4 == 0x617 || local_4 == 0x5C0`. | `0x6A0` behavior is live offline YR shell behavior, not TS legacy. |
| Setup initializes child `0x6A0` as a 19-character capped edit from the player-name global. | Yes | `FUN_006AE6E0`; assembly `0x006AE6F2..0x006AE735`: `GetDlgItem(...,0x6A0)`, `SendMessageA(...,0xC5,0x13,0)`, convert `DAT_00A8B380` via `0x00735120`, send `0x4B2`, then `0x4D1`. | Current Rust's `PLAYER_NAME_MAX_CHARS = 19` and state-backed text are required. |
| Start reads edited text from `0x6A0` into a 20-wide-char buffer before local player/session packing. | Yes | `FUN_006ACEE0`; assembly `0x006AD375..0x006AD39F`: `GetDlgItem(...,0x6A0)`, `SendMessageA(...,0x4B3,0x14,&buffer)`, `FUN_00735090(DAT_00A8B380, buffer)`. | Launch must use the live edited text after validation. |
| `0x6A0` uses ordinary old `Edit`, not `NewEdit`. | Yes for old `Edit`; No for `NewEdit` on this child | Prior resource/class report plus `FUN_0060F9A0` class routing; `OwnerDraw_Edit_00614190` is the callback that contains this focus/Tab/readback behavior. | Do not import `NewEdit`-only timer/storage assumptions into this control. |
| First focus selects all existing text before the deferred focus choreography. | Yes | `OwnerDraw_Edit_00614190`; assembly `0x006143BE..0x006143FA`: on `WM_SETFOCUS`, sends `EM_SETSEL(-1,-1)`, conditionally posts `0x4B0`, then invalidates. | First printable input after focus should replace the default/current name, not append. |
| `0x4B0`/`0x4AF` are focus-restoration plumbing, not visible edit commands. | Yes | Prior focus report plus spot-check: no dedicated old-Edit `0x4B0` body; `0x4AF` restores focus/style at `0x00614558..0x006145AD`. | Rust should model stable focus/caret/input continuity, not literal USER32 message traffic. |
| Tab while focused is consumed by old `Edit`, moves focus to `GetNextDlgTabItem(parent, edit, 0)`, and returns handled. | Yes | `OwnerDraw_Edit_00614190` `WM_CHAR 0x102` branch, assembly/decompile `0x0061451B..0x00614555`; key down/up Tab are also intercepted before native handling. | Current Rust correctly prevents tab insertion and leaves edit focus, but exact next native focus target is not modeled. |
| Enter is not proven to mean Start in this slice. | Conditional | Old `Edit` has an Enter branch only when style bit `0x4` is set; otherwise it falls through to previous WndProc. `FUN_006AE3F0` has no scoped key branch. | Do not claim Start-on-Enter parity from this evidence. |
| Escape-to-Back is not proven by scoped `0x102`/old-Edit evidence. | No verified standard path in this slice | `FUN_006AE3F0` handles common parent, init `0x497`, paint `0xF`, command `0x111`, and `0x4E9`; no `WM_KEYDOWN`, `WM_CHAR`, or `VK_ESCAPE` branch found here. | Current Rust Escape-close is a parity risk until dialog-manager/runtime evidence proves it. |

## 3. Current Rust Implementation Status

| Rust surface | Status vs native | Evidence |
|---|---|---|
| `src/ui/skirmish_shell/state.rs:631` `PlayerNameEditState` | Mostly matches required observable state: text, focus, selection, caret, scroll. | Source scan current tree. |
| `state.rs:711` `focus_select_all` | Matches first-focus all-selection visible effect; it selects `(0,len)`, caret at end, clears scroll. | Native `EM_SETSEL(-1,-1)` at `0x006143BE..0x006143FA`; Rust lines `711..720`. |
| `state.rs:731` `insert_text` | Matches cap for normal insertion: filters controls, replaces selection, takes only capacity up to 19 chars. | Native `EM_SETLIMITTEXT 0x13`; Rust lines `731..752`; tests around `player_name_insert_caps_at_nineteen_chars`. |
| `state.rs:755..809` edit helpers | Broadly matches ordinary edit behavior for backspace/delete/left/right/home/end; selection deletion first is covered. | Rust source and tests around `player_name_backspace_and_delete_remove_selection_first`. |
| `state.rs:1049` `handle_player_name_tab` | Now consumes Tab and blurs edit. This fixes the previous stale audit claim that Tab was not special-cased. | Rust `state.rs:1049..1050`, `app.rs:1098..1100`. |
| `state.rs:2047` launch packing | Matches Start readback requirement at session level: launch session uses `state.player_name_edit.text.clone()`. | Native `0x006AD375..0x006AD39F`; Rust `state.rs:2044..2048`; `app_skirmish.rs:645`. |
| `app.rs:1070..1114` keyboard route | Focused edit consumes text, Backspace, Delete, arrows, Home, End, and Tab before shell/global hotkeys except Escape. | Current Rust source. |
| `app.rs:1147..1155` mouse focus route | Click in edit rect focuses/selects all; clicking elsewhere blurs. | Current Rust source. Exact native already-focused click/caret placement remains untraced. |
| `app.rs:1481..1489` Escape route | Mismatch/risk: Escape closes the native Skirmish shell before focused-edit input routing unless a modal is open. | No scoped native Escape proof in `FUN_006AE3F0`/`OwnerDraw_Edit_00614190`; current Rust source. |
| `app_skirmish_shell_render/controls.rs:98..140` and `text.rs:261..291` | Broadly matches visible edit frame/text/selection/caret/scissor/scroll contract. | Native paint reports; current Rust source. Caret blink/segment colors remain approximate. |

## 4. Coverage Ledger

| Area / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard shell activation | verified | `FUN_006AE2C0`, `FUN_006AE3F0` | none |
| Setup limit/text source | verified | `0x006AE6F2..0x006AE735` | none |
| Start readback | verified | `0x006AD375..0x006AD39F` | none |
| Old Edit vs NewEdit identity | verified-by-prior + spot-check | `SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`, `OwnerDraw_Edit_00614190` | none |
| Focus/select-all | verified | `0x006143BE..0x006143FA` | exact native click placement after already focused is separate |
| Backspace/delete/native edit text operations | touched-not-exhausted | old `Edit` falls through to previous WndProc for ordinary edit keys; Rust implements deterministic equivalents | clipboard/IME/word-navigation out of scope |
| Tab behavior | verified | `0x0061451B..0x00614555`; Rust `app.rs:1098` | exact next tab target/order in Rust not modeled |
| Enter behavior | touched-not-exhausted | old Edit conditional style-bit path; no scoped parent key branch | runtime/dialog-manager trace if Start-on-Enter matters |
| Escape behavior | verified negative for scoped paths | no branch in `FUN_006AE3F0`; old Edit has no Escape branch | runtime/dialog-manager trace before implementing/claiming parity |
| Scroll/caret visibility | verified-by-prior + Rust scan | prior `FUN_00623880` report; Rust `update_player_name_scroll_for_caret` | exact caret blink cadence/display sampling deferred |
| Current Rust implementation scan | verified | source files listed | no code changes in this report |

## 5. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x6A0` live in standard offline YR? -> Yes; setup and Start explicitly touch it in the active `0x102` shell.` (evidence: `FUN_006AE2C0`, `FUN_006AE6E0`, `FUN_006ACEE0`)
- `[RESOLVED] OQ-02 - What is the exact cap? -> Setup sends `EM_SETLIMITTEXT` with `0x13`; Start reads with `0x14` capacity.` (evidence: `0x006AE6F2..0x006AE735`, `0x006AD375..0x006AD39F`)
- `[RESOLVED] OQ-03 - Does current Rust enforce the cap? -> Yes for normal text insertion and selection replacement.` (evidence: `state.rs:731..752`, player-name cap test)
- `[RESOLVED] OQ-04 - Does first focus select all? -> Yes natively and in Rust.` (evidence: `0x006143BE..0x006143FA`; `state.rs:711..720`)
- `[RESOLVED] OQ-05 - Does Start use edited text? -> Yes natively; current Rust launch session now uses `player_name_edit.text`.` (evidence: `0x006AD375..0x006AD39F`; `state.rs:2047`)
- `[RESOLVED] OQ-06 - Is Tab verified? -> Yes; old Edit moves focus to next dialog tab item and returns handled.` (evidence: `0x0061451B..0x00614555`)
- `[RESOLVED] OQ-07 - Does current Rust handle Tab? -> Yes, it routes `KeyCode::Tab` to `handle_player_name_tab`, which blurs edit and consumes the key.` (evidence: `app.rs:1098..1100`, `state.rs:1049..1050`)
- `[RESOLVED] OQ-08 - Is Enter globally Start? -> Not proven by this slice; old Edit Enter is conditional and parent proc has no scoped key branch.` (evidence: `OwnerDraw_Edit_00614190`, `FUN_006AE3F0`)
- `[RESOLVED] OQ-09 - Is Escape globally Back? -> Not proven by this slice; current Rust closes anyway.` (evidence: `FUN_006AE3F0`; `app.rs:1481..1489`)
- `[RESOLVED] OQ-10 - Does current Rust still draw static literal Player? -> No; renderer draws `player_name_edit.text`.` (evidence: `app_skirmish_shell_render/text.rs:261..291`)
- `[DEFERRED] OQ-11 - Exact native caret blink/timer cadence for old `Edit`.` (category: needs-runtime-debugger; reason: current Rust uses steady caret; native cursor visibility cadence needs runtime/video proof; next-step-if-pursued: retail focused-edit frame capture)
- `[DEFERRED] OQ-12 - Exact native already-focused click-to-caret and drag selection behavior.` (category: bounded-cost-too-high; reason: core select-all/input/readback slice is resolved; detailed mouse editing is a separate interaction trace; next-step-if-pursued: old-Edit mouse message trace)
- `[DEFERRED] OQ-13 - Global dialog-manager Enter/Escape accelerators for `0x102`.` (category: needs-runtime-debugger; reason: not visible in scoped proc/old-Edit decompile; next-step-if-pursued: break on `FUN_006ACEE0`/dialog result while pressing Enter/Escape in retail)

## 6. Visual/UI Composition Ledger

| Order | Function / surface | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | Rust `push_player_name_edit_instances` | always during shell render | primitive bevel helper | `layout.player_name` final `(58,59,151,23)` | shell chrome atlas | Yes | edit frame |
| 2 | Rust selection fill | focused and `selection != None` | solid rect | selected text span inside `player_name_edit_text_rect` | selected RGB constant | Yes when selecting | selection overlay |
| 3 | Rust caret | focused and no selection | solid 2px rect | caret x minus scroll, inset y | shell label RGB | Yes when focused/no selection | caret |
| 4 | Rust `push_player_name_edit_text_draw` | text non-empty | bitfont text | text rect `(61,60,147,21)` at 800x600, scissored | shell label RGB | Yes | edit text |

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| primitive edit bevel / shell chrome atlas | Yes | Yes | Yes | No | Yes | No | No | No | Rust controls renderer; prior ownerdraw edit frame report |
| bitfont shell text | Yes | Yes | Yes | Yes | No | No | No | No | Rust text renderer; prior `DAT_00AC18A4` text-color report |
| selection/caret solid rects | Yes | Conditional | Conditional | No | No | Yes | No | No | Rust controls renderer; native `FUN_00623880` caret/selection behavior |

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Escape-to-Back is not proven by scoped native `0x102`/old-Edit evidence. | `FUN_006AE3F0` has no key/Escape branch; old Edit has no Escape branch; current Rust closes at `app.rs:1481..1489`. | mismatch/risk | `src/app.rs` keyboard dispatch | Gate/remove Escape-close parity claim unless runtime dialog-manager evidence proves it; at minimum do not let Escape bypass focused edit under a false parity label. | Focus name edit, press Escape: behavior must be justified by a runtime trace before being asserted as native parity. Proposed test: `player_name_escape_does_not_claim_unverified_back_behavior`. | Do not treat absence of a parent branch as proof of Back; runtime may still have dialog-manager accelerators. |
| Tab leaves the old Edit via `GetNextDlgTabItem`; current Rust only blurs. | `0x0061451B..0x00614555`; Rust `app.rs:1098`, `state.rs:1049`. | partial | `src/ui/skirmish_shell/state.rs`, `src/app.rs` | Current no-insert/blur is acceptable as a bounded approximation, but exact parity needs a focus owner/tab-order target if later controls become keyboard-focusable. | Focus name edit, press Tab: no `\t` inserted, edit no longer focused, and future focus owner matches verified tab order. Proposed test: `player_name_tab_moves_focus_to_next_shell_control_without_inserting_tab`. | Do not implement Tab as printable text or global hotkey while edit owns input. |
| First focus selects all and Start reads edited text. | `0x006143BE..0x006143FA`, `0x006AD375..0x006AD39F`; Rust `state.rs:711`, `state.rs:2047`. | none observed | `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render/text.rs` | Preserve current behavior through future shell input/render refactors. | Click name, type `Ace`, Start: default is replaced and session/local owner uses `Ace`. Proposed test: `player_name_focus_replaces_default_and_launch_uses_edited_name`. | Do not regress to static text or append-after-default behavior. |

### Negative Facts / Do Not Do

- Do not regress `0x6A0` to static text. Active in YR: No. Evidence: setup sends text to child `0x6A0`; Start reads child `0x6A0` back through `0x4B3`.
- Do not treat `NewEdit` as the player-name implementation. Active in YR: No for `0x6A0`. Evidence: resource class is ordinary `Edit`; old `Edit` callback contains the verified focus/Tab/readback behavior.
- Do not insert Tab into the player name. Active in YR: No. Evidence: old Edit consumes Tab and calls `GetNextDlgTabItem` / `SetFocus` at `0x0061451B..0x00614555`.
- Do not claim Start-on-Enter from this audit. Active in YR: Conditional/unverified. Evidence: old Edit Enter handling is style-bit-gated; parent proc has no scoped key branch.
- Do not claim Escape-to-Back from this audit. Active in YR: No verified standard path in this slice. Evidence: no scoped `0x102` proc or old-Edit Escape branch.
- Do not build literal `0x4B0`/`0x4AF` plumbing in Rust. Active in YR: implementation detail only. Evidence: player-visible result is focus/select-all/caret/input continuity.

### Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_0X6A0_POST_IMPLEMENTATION_AUDIT_GHIDRA_REPORT.md`: replace older wording that said "`Tab` is not handled as native focus traversal; it is only prevented from insertion if delivered as control text" with: "Current Rust now routes `KeyCode::Tab` through focused edit input and blurs/consumes the edit; this matches the verified no-insert/leave-edit half of native Tab behavior, but not the exact `GetNextDlgTabItem` destination."
- `docs/research/skirmish-ui/SKIRMISH_0X102_CHILD_CONTROL_RECT_MATRIX_CURRENT_RUST_GHIDRA_REPORT.md`: replace "player-name edit focus/input and status/help hover text absent" with: "player-name edit focus/input/readback are now implemented; remaining player-name deltas are exact Tab focus destination, Escape parity, mouse caret/drag selection, and caret blink/pixel cadence."
- `docs/research/skirmish-ui/SKIRMISH_0X102_FIRST_PAINT_COMPOSITION_BROAD_RECHECK_GHIDRA_REPORT.md`: replace "Rust still renders a literal `Player` text surface for the player-name area" with: "Rust now renders `player_name_edit.text` with edit frame, selection/caret, scissor, and launch readback; remaining visual uncertainty is caret blink/selected-text pixel parity."

### Remaining Uncertainty

- Exact native caret blink/timer cadence and final captured RGB for caret/selection/text.
- Exact already-focused click-to-caret, mouse drag selection, clipboard, IME, and word-navigation behavior.
- Global dialog-manager Enter/Escape accelerators for `0x102`; scoped proc/old-Edit evidence does not prove them.

## Sources

- Ghidra read-only decompile: `FUN_006AE2C0 @ 0x006AE2C0`, `FUN_006AE3F0 @ 0x006AE3F0`, `FUN_006AE6E0 @ 0x006AE6E0`, `FUN_006ACEE0 @ 0x006ACEE0`, `OwnerDraw_Edit_00614190 @ 0x00614190`.
- Ghidra read-only disassembly contexts: `0x006AE6F2..0x006AE735`, `0x006AD375..0x006AD39F`, `0x006143BE..0x00614555`, `0x00614558..0x006145AD`.
- Prior docs: `docs/research/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_FOCUS_MESSAGES_0X4B0_0X4AF_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_SHELL_INPUT_FOCUS_MESSAGE_BROAD_RECHECK_GHIDRA_REPORT.md`.
- Current Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render/controls.rs`, `src/app_skirmish_shell_render/text.rs`, `src/app_skirmish.rs`.
