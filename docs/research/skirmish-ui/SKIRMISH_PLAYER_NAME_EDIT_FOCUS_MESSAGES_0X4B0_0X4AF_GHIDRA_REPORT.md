# Skirmish Player Name Edit Focus Messages 0x4B0 / 0x4AF - Ghidra Research Report

**Address(es):** `0x00614190` (`OwnerDraw_Edit_00614190`), `0x00610CA0..0x006128FE` common shell subclass thunk range, `0x00622470` child HWND collector callback, `0x007757E0` shell window-stack focus restore helper  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** deferred OQ-13 from `SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`: old `Edit` focus/caret custom messages `0x4B0` and `0x4AF` as they affect standard offline Skirmish dialog `0x102` player-name edit control `0x6A0`.  
**Non-Scope:** basic `0x6A0` identity/setup/readback, `NewEdit` behavior except contrast, full text-buffer plumbing, full shell invalidation architecture, online lobby edit controls, and runtime screenshot/RGB sampling.  
**Confidence:** High for `0x6A0` old-Edit consumer behavior, `0x4B0` self-post, `0x4AF` restore branch, and the common-shell child broadcast site; Medium for exact parent-frame schedule because `0x00610CA0` lacks a Ghidra function boundary and was inspected read-only from disassembly.  
**Active in YR:** Yes. Standard offline Skirmish dialog `0x102` installs the common shell subclass thunk and routes child `0x6A0` class `Edit` to `OwnerDraw_Edit_00614190`; the focus message paths are inside those active callbacks.

Working notes required before investigation:

- Target question: What do custom messages `0x4B0` and `0x4AF` do for the standard Skirmish player-name edit `0x6A0`, who produces/consumes them, and what keyboard-focus behavior must Rust reproduce?
- Non-goals: Do not re-investigate the settled facts that `0x6A0` is old `Edit`, seeded from `DAT_00A8B380`, limited to 19 chars, read back through `0x4B3`, and painted through the edit frame/text/caret helper.
- Evidence needed to mark COMPLETE: decompile plus address evidence for the old-Edit `WM_SETFOCUS -> 0x4B0` path and `0x4AF` restore branch; disassembly/caller evidence for `0x4AF` producers; false-positive filtering for unrelated `0x4B0/0x4AF` constants; Rust surface scan and testable handoff.
- Stop conditions: stop after all `0x4B0/0x4AF` focus-message producers/consumers relevant to standard `0x102` old `Edit` are resolved or explicitly deferred; do not mutate Ghidra; write only this report and the shared claims file if present.

## 1. Overview

The old Skirmish player-name edit does not keep native Win32 focus continuously through shell repaint/layout work. On `WM_SETFOCUS`, `OwnerDraw_Edit_00614190` selects the full edit text, posts `0x4B0` to itself, and invalidates; when that posted message re-enters the same callback, a generic entry guard sees the edit still has focus and temporarily moves focus to `g_hWnd` while marking a "restore me later" flag. The common shell subclass paint/dispatch path later enumerates descendants and sends `0x4AF` to each child HWND; the old edit consumes `0x4AF` by marking restore-in-progress, restoring focus to itself if the guard moved it away, and re-enabling a hidden/visible style bit when that bit was temporarily cleared during init/paint.

Rust does not need literal Win32 custom messages, but it does need the observable contract: clicking/focusing the player-name field keeps keyboard text entry on that field across shell redraws, selects/carets consistently, and never leaves focus stuck on the shell root after a repaint.

## 2. Key Flags / Offsets

| Field / flag | Observed role | Active in YR | Evidence |
|---|---|---|---|
| old Edit record `+0xF0` (`piVar3[0x3C]`) | focus-restore-needed latch; set when entry guard or init redirects focus away; cleared by `0x4AF` after `SetFocus(edit)` | Yes | `0x00614288..0x006142B2`, `0x00614373..0x00614392`, `0x00614560..0x00614582` |
| old Edit record `+0xF4` (`piVar3[0x3D]`) | restore-in-progress / suppress-`0x4B0` latch; `0x4AF` sets it before restoring focus; `WM_SETFOCUS` posts `0x4B0` only when it is zero | Yes | `0x006143D5..0x006143F4`, `0x00614560..0x00614566` |
| old Edit record `+0xF8` (`piVar3[0x3E]`) | remembers that `WS_VISIBLE` (`0x10000`) was cleared and must be restored by `0x4AF` | Yes | `0x00614398..0x006143B4`, `0x0061458C..0x006145AD` |
| style bit `0x10000` | standard `WS_VISIBLE`; cleared on init when set, restored by `0x4AF` if the latch is set | Conditional, on current style | `0x00614398..0x006143B4`, `0x00614596..0x006145A7` |
| posted message `0x4B0` | self-posted after old Edit `WM_SETFOCUS`; not handled by a dedicated old-Edit switch branch, so its visible effect comes from the callback entry guard | Yes | `0x006143BE..0x006143FA`, fallback path to previous proc after no explicit branch |
| sent message `0x4AF` | restore message consumed explicitly by old Edit; also used as a string-table id elsewhere, which is not this mechanism | Yes for shell child broadcast; No for string-table false positive | `0x0061275F..0x006127A0`, `0x00614558..0x006145AD`; false positive `ScenarioClass__Read_Scenario @ 0x0068471E..0x00684749` |

## 3. Core Logic

### 3.1 Entry Guard: Focus Is Temporarily Redirected Before Message-Specific Handling

Active in YR: Yes. `OwnerDraw_Edit_00614190` first resolves the old edit's shared record. Before checking the incoming message ID, it calls `GetFocus`; if the focused HWND is this edit and the restore-in-progress latch is zero, it sets the restore-needed latch and calls `SetFocus(g_hWnd)`.

This guard applies to the later posted `0x4B0` because old Edit has no dedicated `0x4B0` switch branch. The posted message is therefore a deferred trigger for the entry guard rather than a message with unique body logic.

Evidence: decompile and assembly of `OwnerDraw_Edit_00614190`, especially `0x00614288..0x006142B2`; no old-Edit `cmp msg,0x4B0` branch appears in `0x006143B9..0x006145BC`; unhandled messages fall through to previous-proc dispatch at `0x00614872`.

### 3.2 `WM_SETFOCUS` Producer: Select All, Post `0x4B0`, Invalidate

Active in YR: Yes. On `WM_SETFOCUS (7)`, old Edit first sends native `EM_SETSEL(-1,-1)` to select the text. If the restore-in-progress latch is zero, it posts `0x4B0` to itself with zero `wParam/lParam`. It then reaches the invalidation path shared by key/focus/mouse events.

The important ordering is `EM_SETSEL` before `PostMessage(0x4B0)`, and the post is conditional on restore-in-progress being zero. That prevents the focus restore caused by `0x4AF` from recursively posting another `0x4B0`.

Evidence: `OwnerDraw_Edit_00614190` decompile and assembly `0x006143BE..0x006143FA`; invalidation continuation `0x006147EE..0x00614872`. Active path is reached for child `0x6A0` because prior report verifies class `Edit -> OwnerDraw_Edit_00614190`.

### 3.3 `0x4AF` Consumer: Restore Focus And Style Without Reposting

Active in YR: Yes. On `0x4AF`, old Edit sets restore-in-progress to `1`, checks the restore-needed latch, and if set calls `SetFocus(edit)` then clears restore-needed. If the style-restore latch is set, it ORs `0x10000` into the current style and calls `SetWindowLongA(GWL_STYLE, style)`. It returns zero without calling the previous WndProc.

Because restore-in-progress is set before `SetFocus(edit)`, the subsequent `WM_SETFOCUS` path does not post another `0x4B0`. The player-visible result is stable focus after the shell finishes a paint/layout cycle.

Evidence: `OwnerDraw_Edit_00614190` decompile and assembly `0x00614558..0x006145AD`; `WM_SETFOCUS` post guard at `0x006143D5..0x006143F4`.

### 3.4 Primary `0x4AF` Producer: Common Shell Child Broadcast After Parent/Child Paint Work

Active in YR: Yes. The common shell subclass thunk range lacks a formal Ghidra function boundary in this project, so this was inspected read-only from retail executable disassembly. In the active `0x00610CA0` thunk range, one path enumerates child windows of the current HWND via the callback at `0x00622470`, stores HWNDs into a temporary vector, then loops from `0` to count-1 and calls `SendMessageA(child,0x4AF,0,0)` for each collected child. The vector is freed immediately after the loop.

This path is the normal restore producer for old Edit controls affected by the entry guard. It is child-broadcast, not player-name-specific; `0x6A0` participates because it is a subclassed child of the active Skirmish dialog.

Evidence: `0x0061275A..0x00612771` pushes callback `0x00622470` and calls USER32-style child enumeration; `0x0061277B..0x006127A0` bounds-checks each index and sends `0x4AF`; `0x006127AB..0x006127B9` frees the vector. `0x00622470` appends HWNDs to the vector at `0x00622585..0x006225D1`. YR activity follows prior subclass report: `FUN_00622B50 -> FUN_0060F9A0` installs `0x00610CA0` on standard `0x102` parent/children.

### 3.5 Secondary `0x4B0` Producer: Shell Window-Stack Restore Helper

Active in YR: Conditional. `FUN_007757E0` restores a previous shell/dialog window from a global window stack, calls `SetForegroundWindow`, invalidates and updates it, stores it in `DAT_00B72F44`, calls `SetFocus(hWnd)`, then sends `0x4B0` to that restored window. This is shell-global window-stack behavior, not the standard player-name self-post.

For this slice, it is relevant as a producer of the same custom message but not as the normal `0x6A0` typing path. It could matter if the active restored window itself is an old Edit consumer or routes to one, but that requires a broader shell modal/window-stack investigation.

Evidence: `FUN_007757E0` decompile; `SendMessageA(DAT_00B72F44,0x4B0,0,0)` after `SetFocus(hWnd)`. Active in YR is conditional on non-empty `DAT_00B72F50` shell window stack.

### 3.6 False Positives For Constants

Active in YR: No for this focus-message mechanism. Several raw `0x4B0`/`0x4AF` constants are not HWND custom focus messages:

- `ScenarioClass__Read_Scenario @ 0x0068471E..0x00684749` uses `0x4AF` as a string-table ID passed to `StringTable__LoadString`, not as `SendMessage`.
- `0x00694354` and `0x00694D31` pass `0x4B0` to `0x00540D10` with non-HWND arguments, not USER32 `SendMessage/PostMessage`.
- `0x004C6EA1` and `0x005EF10D` pass `0x4B0` into game/object helper constructors or event calls, not old Edit focus handling.

These were filtered by surrounding call shape and are not Rust-facing for Skirmish player-name focus.

## 4. INI Keys

No INI keys control these messages. Active in YR: Yes, because the behavior is shell HWND/message state reached from standard dialog setup, not optional INI-driven gameplay.

## 5. Integration Points

| Integration point | Behavior | Active in YR | Evidence |
|---|---|---|---|
| `0x006AE3F0` / `0x00622B50` / `0x0060F9A0` | standard Skirmish `0x102` installs the common shell subclass and old `Edit` owner proc | Yes | prior player-name and subclass reports; `FUN_0060F9A0` decompile |
| `OwnerDraw_Edit_00614190` | consumes `WM_SETFOCUS`, posts `0x4B0`, redirects focus in entry guard, consumes `0x4AF` restore | Yes for `0x6A0` | `0x00614288..0x006145AD` |
| common subclass thunk `0x00610CA0..0x006128FE` | child paint/dispatch path sends `0x4AF` to enumerated child HWNDs | Yes | disassembly `0x0061275A..0x006127B9`; prior subclass report |
| `0x00622470` | child enumeration callback appends HWNDs into vector used by `0x4AF` broadcast | Yes | read-only disassembly `0x00622470..0x006225D1` |
| `FUN_007757E0` | shell-global window-stack restore sends `0x4B0` to restored top window | Conditional | decompile `0x007757E0` |

## 6. Current Rust Implementation Status

| Rust surface | Current status | Binary mismatch / implication |
|---|---|---|
| `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs` | `SkirmishShellState` has no player-name text, edit-focus, text-selection, or caret field in the scanned struct. | Missing old-Edit observable state: focused field, typed buffer, selected/caret state, and repaint-stable focus retention. |
| `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs` | render path still pushes literal `"Player"` at `layout.player_name`. | Mismatch: binary displays editable player-name text, not a static literal. This report adds the focus-restoration requirement to the prior visual/text gap. |
| `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs` | `PlayerName0x6a0` rect matches prior geometry `(58,59,151,23)`. | Geometry basis is usable; focus/caret state must be layered over this rect. |
| `C:/Users/enok/Documents/ra2-rust-game/src/app.rs` | app has keyboard plumbing generally, but no scanned route that directs text input to Skirmish `0x6A0`. | Missing input focus dispatch for typed characters/backspace/selection/caret while Skirmish shell is active. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Prior `0x6A0` identity/setup/readback basics | verified-by-prior | `SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md` | none; not re-covered |
| old Edit entry guard focus redirect | verified | `0x00614288..0x006142B2` | none |
| old Edit `WM_SETFOCUS -> EM_SETSEL -> PostMessage(0x4B0)` | verified | `0x006143BE..0x006143FA` | none |
| old Edit behavior for delivered `0x4B0` | verified | no dedicated branch; entry guard plus fallback `0x00614872` | exact native edit response to unknown custom message is not player-visible here |
| old Edit `0x4AF` restore branch | verified | `0x00614558..0x006145AD` | none |
| common thunk `0x4AF` child broadcast | verified | `0x0061275A..0x006127B9`; `0x00622470..0x006225D1` | exact frame/tick schedule deferred to shell-global timing if needed |
| `FUN_007757E0` `0x4B0` producer | touched-not-exhausted | decompile `0x007757E0` | broader shell window-stack contexts |
| false-positive `0x4AF` string-table use | verified negative | `ScenarioClass__Read_Scenario @ 0x0068471E..0x00684749` | none |
| false-positive `0x4B0` non-HWND uses | verified negative enough | local disassembly around `0x004C6EA1`, `0x005EF10D`, `0x00694354`, `0x00694D31` | exact helper semantics out-of-scope |
| Rust focus/input/render delta | verified by scan | files in Section 6 | implementation pending |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is this slice active in standard offline Skirmish? -> Yes; `0x6A0` is old `Edit`, common subclassing is installed, and old Edit consumes the focus messages.` (evidence: prior `0x6A0` report; `0x0060F9A0`; `0x00614190`)
- `[RESOLVED] OQ-02 - Who posts `0x4B0` for player-name old Edit? -> `OwnerDraw_Edit_00614190` posts it to itself during `WM_SETFOCUS` after native `EM_SETSEL(-1,-1)`.` (evidence: `0x006143BE..0x006143FA`)
- `[RESOLVED] OQ-03 - Does old Edit have a dedicated `0x4B0` switch body? -> No; delivery reaches the entry guard and then previous-proc fallback.` (evidence: `0x006143B9..0x006145BC`, `0x00614872`)
- `[RESOLVED] OQ-04 - What is the visible purpose of self-posted `0x4B0`? -> It defers focus redirection to the callback entry guard, which marks restore-needed and moves focus to `g_hWnd`.` (evidence: `0x00614288..0x006142B2`, `0x006143EE`)
- `[RESOLVED] OQ-05 - Who consumes `0x4AF` for old Edit? -> `OwnerDraw_Edit_00614190`; it restores focus and style state and returns handled.` (evidence: `0x00614558..0x006145AD`)
- `[RESOLVED] OQ-06 - Why does `0x4AF` restoration not recurse into another `0x4B0` post? -> It sets restore-in-progress before `SetFocus(edit)`, and the `WM_SETFOCUS` branch only posts when that flag is zero.` (evidence: `0x00614560..0x0061457C`, `0x006143D5..0x006143F4`)
- `[RESOLVED] OQ-07 - Who normally sends `0x4AF` to Skirmish children? -> The common shell subclass path enumerates children with `0x00622470` and sends `SendMessageA(child,0x4AF,0,0)` to each HWND.` (evidence: `0x0061275A..0x006127A0`)
- `[RESOLVED] OQ-08 - Is `0x0068472A` a `0x4AF` focus-message producer? -> No; it is `StringTable__LoadString(...,0x4AF)` in scenario loading.` (evidence: `ScenarioClass__Read_Scenario @ 0x0068471E..0x00684749`)
- `[RESOLVED] OQ-09 - Are other raw `0x4B0` constants player-name focus producers? -> Only `0x006143EE` is the old-Edit self-post for this control; `0x007757E0` is shell window-stack restore; others inspected are non-HWND helper arguments.` (evidence: `0x006143BE..0x006143FA`; `FUN_007757E0`; local disassembly contexts)
- `[RESOLVED] OQ-10 - Is `0x4AF` tied to an INI/default gate or TS-only setting? -> No INI gate found; it is shell UI message behavior active through standard YR dialog setup.` (evidence: decompiled setup paths; no INI reads in these functions)
- `[RESOLVED] OQ-11 - Does current Rust model player-name focus/caret/text state? -> No scanned state/render/input route exists; render draws literal `"Player"` in the edit rect.` (evidence: `state.rs::SkirmishShellState`; `app_skirmish_shell_render.rs` player-name draw)
- `[DEFERRED] OQ-12 - Exact native previous-proc result for delivered `0x4B0`.` (category: bounded-cost-too-high; reason: old Edit's observable custom behavior is the pre-fallback entry guard; proving USER32 unknown-message return value does not affect Rust focus/caret output; next-step-if-pursued: runtime trace native edit WndProc return)
- `[DEFERRED] OQ-13 - Exact frame/tick schedule of the common thunk path that broadcasts `0x4AF` under every shell repaint/invalidation state.` (category: requires-different-system-context; reason: producer and consumer are verified, but every parent-frame scheduling condition is shell-global invalidation work beyond `0x6A0`; next-step-if-pursued: focused common-shell paint timing investigation)
- `[DEFERRED] OQ-14 - Full semantics of `FUN_007757E0` shell window-stack restore for modals/online dialogs.` (category: out-of-scope; reason: not the normal offline `0x6A0` self-post/restore path; next-step-if-pursued: shell modal focus stack investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x6A0` focus is repaint-stable: `WM_SETFOCUS` selects text and posts `0x4B0`; deferred guard temporarily moves focus to shell root; later `0x4AF` restores focus without recursive `0x4B0` | `0x00614288..0x006145AD`; `0x0061275A..0x006127A0` | missing | `src/ui/skirmish_shell/state.rs`, `src/app.rs` | Add direct edit-focus state for `layout.player_name`; keep text input routed to the name field across shell redraw/map-preview invalidation cycles | Focus player-name field, type a character, trigger a redraw by changing map/option, then type another character; both characters land in the name field and focus is not lost | Do not emulate focus by letting app-level keyboard handling fall back to shell/global hotkeys while edit is active; proposed test `skirmish_player_name_focus_survives_shell_redraw_and_keeps_text_input` |
| On first focus, native old Edit selects all text before the deferred focus guard runs | `0x006143BE..0x006143FA` | missing | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Track selection/caret for player-name edit; initial focus should select current name or otherwise render caret/selection consistent with full-selection behavior before replacement typing | Default name is focused, pressing a printable key replaces the selected default instead of appending after it; caret then remains in the field | Do not implement the field as append-only text after click; proposed test `skirmish_player_name_initial_focus_selects_existing_name_before_typing` |
| `0x4AF` is a child-broadcast restore message, not a per-control data setter or string key; old Edit consumes it by restoring focus/style and returning handled | `0x0061275A..0x006127A0`; `0x00614558..0x006145AD`; false positive `0x0068471E..0x00684749` | design/risk gap | `src/app_skirmish_shell_render.rs`, shell UI docs/tests | Model the observable restore result as UI state after redraw, not as a literal app-wide custom-message queue | During any Skirmish redraw, focused player-name edit remains active and visible/caret rendering resumes; no user-visible `0x4AF` event exists | Do not use every raw `0x4AF` constant as focus evidence or add a generic custom-message bus; proposed test `skirmish_player_name_redraw_restores_focus_without_custom_message_bus` |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`: replace final-state OQ-13 with: "`[RESOLVED by SKIRMISH_PLAYER_NAME_EDIT_FOCUS_MESSAGES_0X4B0_0X4AF_GHIDRA_REPORT.md] Old `Edit` posts `0x4B0` to itself on `WM_SETFOCUS` after `EM_SETSEL(-1,-1)`; delivery of that posted message triggers the edit callback entry guard, which marks focus-restore-needed and moves focus to `g_hWnd`; the common shell subclass path later enumerates child HWNDs and sends `0x4AF`, which old `Edit` consumes by setting restore-in-progress, restoring focus/style, and returning handled. Rust should model repaint-stable player-name edit focus/caret/input state, not literal Win32 custom messages.`"
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`: replace Remaining Uncertainty bullet "`Exact old-Edit custom `0x4B0` / `0x4AF` focus-restoration choreography remains out of scope...`" with: "`Old-Edit `0x4B0` / `0x4AF` focus restoration is covered by `SKIRMISH_PLAYER_NAME_EDIT_FOCUS_MESSAGES_0X4B0_0X4AF_GHIDRA_REPORT.md`; remaining uncertainty is only exact native previous-proc return for delivered `0x4B0` and full shell-global frame scheduling for every `0x4AF` broadcast condition.`"

### Negative Facts / Do Not Do

- Do not implement `0x4B0` as a visible text-edit command. Active in YR: No. Evidence: old Edit has no dedicated `0x4B0` branch; effect comes from the entry guard before fallback (`0x00614288..0x006142B2`, `0x00614872`).
- Do not let `SetFocus`/redraw cycles steal keyboard input from the player-name edit permanently. Active in YR: No. Evidence: `0x4AF` restores focus when the restore-needed latch is set (`0x00614560..0x00614582`).
- Do not treat all raw `0x4AF` constants as focus messages. Active in YR: No for false positive. Evidence: `ScenarioClass__Read_Scenario` uses `0x4AF` as a string-table ID (`0x0068471E..0x00684749`).
- Do not build a literal Win32 custom-message bus in Rust for this. Active in YR: implementation detail only. Evidence: player-visible effect is focus/caret/text continuity; messages exist inside USER32 owner-draw shell plumbing, not gameplay.
- Do not reuse `NewEdit` timer/caret assumptions for `0x6A0`. Active in YR: No for this control. Evidence: prior report verifies resource class `Edit -> OwnerDraw_Edit_00614190`; this report's focus path is old Edit-specific.

### Remaining Uncertainty

- Exact native previous-proc return value for delivered custom `0x4B0` is not proven; no Rust-facing behavior was found beyond the pre-fallback entry guard.
- Exact shell-global scheduling conditions for every `0x4AF` broadcast path are not fully exhausted; producer/consumer behavior for standard child restoration is verified.
- `FUN_007757E0` shell window-stack `0x4B0` use is only touched enough to classify; modal/window-stack focus behavior is a separate system.

## Sources

- Ghidra read-only decompile: `OwnerDraw_Edit_00614190 @ 0x00614190`, `FUN_0060F9A0 @ 0x0060F9A0`, `ScenarioClass__Read_Scenario @ 0x00684620`, `FUN_007757E0 @ 0x007757E0`.
- Read-only disassembly from retail `gamemd.exe`: `0x00610CA0..0x006128FE`, focused ranges `0x0061275A..0x006127B9`, `0x00622470..0x006225D1`, and false-positive contexts around `0x004C6EA1`, `0x005EF10D`, `0x00694354`, `0x00694D31`.
- Prior docs checked: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`.
- Rust surfaces scanned: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`.
- INI files: no relevant INI keys; this behavior is shell HWND/message driven.

