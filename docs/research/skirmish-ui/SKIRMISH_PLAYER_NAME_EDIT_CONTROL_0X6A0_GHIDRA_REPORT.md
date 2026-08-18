# Skirmish Player Name Edit Control 0x6A0 - Ghidra Research Report

**Address(es):** `0x006AE6E0`, `0x00614190`, `0x00614B30`, `0x00623880`, `0x006ACEE0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard offline Yuri's Revenge Skirmish dialog `0x102` player-name edit control `0x6A0`: setup, owner-draw routing, text source/readback, paint/caret/input behavior, geometry/insets, color/password/disabled handling, and Rust-facing implications.  
**Non-Scope:** Choose Map preview refresh, listbox row paint, combo popup layout, trackbar disabled flow, start-marker clipping, online lobby edit controls, and full shell-global text storage internals beyond the `0x6A0` calls needed here.  
**Confidence:** High for standard `0x102` reachability, setup/readback, class callback selection, paint/caret core, and Rust delta; Medium for exact visual RGB after 16-bit display conversion because no runtime screenshot sample was taken.  
**Active in YR:** Yes. The offline launcher creates dialog `0x102` with proc `0x006AE3F0` per prior rect/start reports; `0x006AE3F0` delegates init message `0x497` to `0x006AE6E0`; `0x006AE6E0` explicitly initializes child `0x6A0`; Start `0x617` in `0x006ACEE0` reads child `0x6A0`.

Working notes required before investigation:

- Target question: What does standard offline Skirmish dialog `0x102` actually do for player-name edit control `0x6A0`, and what must Rust change for visible/input parity?
- Non-goals: Do not investigate Choose Map preview refresh, listbox row paint, combo popup internals, trackbar disabled runtime flow, start-marker clipping, or online lobby edit controls.
- Evidence needed to mark COMPLETE: active `0x102` setup/readback evidence for `0x6A0`; callback selection between `Edit` and `NewEdit`; owner-draw paint/input/focus/caret and text limit behavior; Rust surface comparison and concrete tests.
- Stop conditions: stop if `0x6A0` is not active in standard YR, if Ghidra lacks read-only evidence for the callback boundary, if the path requires runtime-only observation for a material claim, or after all scoped open questions are resolved/deferred.

## 1. Overview

The local player-name field in the offline Skirmish setup screen is a real editable Win32 child, not a static label. It is RT_DIALOG child `0x6A0`, class `Edit`, final rect `(58,59,151,23)`, and it uses `OwnerDraw_Edit_00614190`; `OwnerDraw_NewEdit_00614B30` is part of the same shell edit family but is not the `0x6A0` callback in resource `0x102`.

Setup seeds the edit from global narrow string `DAT_00A8B380`, converted to wide by `0x00735120`, sent via custom text message `0x4B2`, then locks custom state with message `0x4D1`. On Start, `0x006ACEE0` reads the wide edit text with `0x4B3` into a 20-wide-char buffer and copies it back to `DAT_00A8B380` before country/color/start/team/session packing.

## 2. Class Layout / Key Offsets

The exact shared record type is shell-global and partly documented in the subclass-thunk reports. For this slice, these fields were observed in the edit callbacks:

| Record field | Observed role | Evidence | Active in YR |
|---|---|---|---|
| owner-proc map entry | class callback: `Edit -> 0x00614190`, `NewEdit -> 0x00614B30` | `FUN_0060F9A0 @ 0x0060FCF0..0x0060FD80` decompile; prior resource row says `0x6A0` class `Edit` | Yes for `Edit` on `0x6A0`; No for `NewEdit` on `0x6A0` |
| text buffer pointer | dynamic wide text used by `0x4B2/0x4B3` and mirrored into native edit text | `OwnerDraw_Edit_00614190 @ 0x0061489C..0x00614974`; prior `0x00610CA0` text plumbing | Yes |
| edit max length | `NewEdit` stores `EM_SETLIMITTEXT` at record `+0x48`; old `Edit` forwards unhandled `0xC5` to native edit WndProc | `OwnerDraw_NewEdit_00614B30 @ 0x00614D08..0x00614D20`; `OwnerDraw_Edit_00614190` lacks a `0xC5` branch and forwards unhandled messages at `0x00614872..0x00614889` | Conditional: `0x6A0` limit is native-forwarded, not `NewEdit`-stored |
| focus-suppression flags | old `Edit` can temporarily move focus to `g_hWnd` until custom `0x4AF` restores focus | `OwnerDraw_Edit_00614190 @ 0x00614373..0x006143B4`, `0x00614558..0x006145AD` | Yes in edit callback; normal stock reachability depends on focus messages |
| caret blink/toggle | `NewEdit` uses `WM_SETFOCUS`, `WM_KILLFOCUS`, and `WM_TIMER` to toggle record `+0x4C`; old `Edit` routes caret state through the shared edit painter and invalidates on focus/key/mouse messages | `OwnerDraw_NewEdit_00614B30 @ 0x00614E32..0x00615020`; `OwnerDraw_Edit_00614190 @ 0x0061477C..0x00614872`; `FUN_00623880 @ 0x00623880` | Yes for edit-family paint; exact old-Edit blink message source beyond `0x4B0/0x4AF` is not expanded |

## 3. Core Logic

### 3.1 Setup and Text Source

Active in YR: Yes. `FUN_006AE6E0` begins standard Skirmish init by clearing the session/player buffer at `DAT_00A8B250`, then gets child `0x6A0`. If the HWND is non-null it sends:

1. `EM_SETLIMITTEXT (0xC5)`, `wParam=0x13`, so the visible player name is capped to 19 characters plus terminator capacity.
2. `0x4B2` with `lParam = 0x00735120(DAT_00A8B380)`, converting the current narrow player-name global to the wide custom text representation.
3. `0x4D1`, `wParam=1`, updating shell edit state through the common subclass thunk.

Evidence: decompile `0x006AE6E0`; assembly context `0x006AE6F2..0x006AE735` shows `GetDlgItem(...,0x6A0)`, `SendMessageA(...,0xC5,0x13,0)`, `MOV ECX,0xA8B380; CALL 0x00735120`, `SendMessageA(...,0x4B2,0,EAX)`, then `SendMessageA(...,0x4D1,1,0)`.

### 3.2 Callback Selection: Old `Edit`, Not `NewEdit`

Active in YR: Yes for old `Edit`. The verified `0x102` resource inventory lists child `0x6A0` as class `Edit`, not `NewEdit`. `FUN_0060F9A0` maps class `NewEdit` to `OwnerDraw_NewEdit_00614B30`, but the next class branch maps the ordinary edit class to `OwnerDraw_Edit_00614190`. Therefore `NewEdit` behavior is useful as family contrast but must not be assigned to offline `0x6A0`.

Evidence: `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md` resource row for `0x6A0`; `FUN_0060F9A0 @ 0x0060FCF0..0x0060FD80`; `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md` row `Edit -> 0x00614190`.

### 3.3 Geometry, Frame, and Text Insets

Active in YR: Yes. The final child window rect is not the raw DLU rect. Resource/DLU gives `(57,59,150,23)`; resize fixup `FUN_0060B950` changes it to `(58,59,151,23)`.

The owner-draw `0x497` init then moves the native edit client inward by `+1,+1` and shrinks client width/height by `-2,-2`, creating the primitive-frame margin. On paint, the edit callback builds a surface matching the client span, draws primitive frame via `FUN_006208F0` unless the parent class comparison suppresses it, and passes a text rect beginning at left `+2` to `FUN_00623880`.

Evidence: prior complete child matrix for final rect; `OwnerDraw_Edit_00614190 @ 0x0061433D..0x00614373` for `MoveWindow(...,+1,+1,w-2,h-2)`; `0x006146D0` primitive frame call; `0x00614710..0x00614760` text-paint call arguments with left offset `+2`. `OwnerDraw_NewEdit_00614B30` has the same `0x497` `SetWindowPos(...,+1,+1,w-2,h-2)` pattern at `0x00614C87..0x00614D05`, but it is not the `0x6A0` callback.

### 3.4 Paint Color, Password Flag, Caret, and Selection

Active in YR: Yes. Edit text uses `DAT_00AC18A4` as the text color source, which setup initializes to `0xFFFF` (yellow in the shell 16-bit palette path). The paint caller reads the edit style with `GetWindowLongA(GWL_STYLE)`, shifts by 5, and passes bit `0x20` as the password-mask flag to `FUN_00623880`; this is not a disabled-state color branch. The same helper draws text segments through `FUN_006211D0`, scrolls horizontally to keep the cursor visible with a 5px margin, and draws a two-pixel-wide caret through two `FUN_00620050` calls when cursor visibility is enabled and there is no selection.

Evidence: `FUN_0060F9A0` initializes `DAT_00AC18A4 = 0xFFFF`; `OwnerDraw_Edit_00614190 @ 0x00614728..0x00614760` reads `GWL_STYLE`, computes `(style >> 5) & 1`, pushes `DAT_00AC18A4`, and calls `0x00623880`; `FUN_00623880` decompile shows password masking to `0x002A`, three text segment draws via `0x006211D0`, 5px cursor margin, and two caret line draws via `0x00620050`.

No standard `0x102` path found disables child `0x6A0`, and the edit paint path does not test `WS_DISABLED` to choose a disabled text color. Active in YR: Conditional only if an external/nonstandard caller disables the HWND; not standard offline setup.

### 3.5 Input, Focus, and Start Readback

Active in YR: Yes. `OwnerDraw_Edit_00614190` intercepts Tab/Enter and regular key/focus/mouse messages:

- Tab key-down/up is consumed before native edit handling; NewEdit sends parent `0x4DB`, while old Edit has its own Tab navigation inside `WM_CHAR`.
- On focus, old Edit sends `EM_SETSEL(-1,-1)`, posts custom `0x4B0` if not in the restored-focus state, then invalidates itself.
- Enter in old Edit with style bit `0x4` sends native text-change style parent `WM_COMMAND` with high word `0x501`; otherwise it falls through to native handling.
- Focus loss, key up/system key/mouse down, and mouse move/context-menu messages invalidate the edit; mouse move and context menu return handled (`1`).

On Start, `FUN_006ACEE0` reads `0x6A0` after validation and before local-player country packing. It sends `0x4B3`, `wParam=0x14`, `lParam=&local_buffer`; `OwnerDraw_Edit_00614190` copies the wide text and force-terminates the last code unit at index `wParam - 1`. `0x006ACEE0` then copies that local wide string back to `DAT_00A8B380` with `0x00735090`.

Evidence: `OwnerDraw_Edit_00614190 @ 0x006143B9..0x006143F4`, `0x00614438..0x00614555`, `0x0061477C..0x00614872`, and `0x00614935..0x00614974`; Start readback assembly `0x006AD375..0x006AD39F`.

## 4. INI Keys

| INI key | Default | Effect in this slice | Active in YR |
|---|---|---|---|
| none | none | Player-name edit setup, painting, input, and launch readback are shell HWND/message/global-string behavior, not INI-driven. | Yes |

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| `0x006AE3F0` dialog proc | On message `0x497`, calls `0x006AE6E0`; on `WM_COMMAND`, calls `0x006ACEE0`. | decompile `0x006AE3F0` | Yes |
| `0x006AE6E0` setup | Initializes `0x6A0` as a 19-char capped player-name edit from `DAT_00A8B380`. | `0x006AE6F2..0x006AE735` | Yes |
| `FUN_0060F9A0` subclass setup | Installs common thunk and class owner proc; ordinary `Edit` maps to `0x00614190`. | decompile `0x0060F9A0` | Yes |
| `OwnerDraw_Edit_00614190` | Handles `0x6A0` paint, text get/set mirroring, focus/key invalidation, frame/text/caret draw handoff. | decompile and assembly contexts above | Yes |
| `OwnerDraw_NewEdit_00614B30` | Similar edit-family callback, but not used by offline `0x6A0`. | decompile `0x00614B30`; resource class contrast | No for `0x6A0`, Conditional elsewhere |
| `FUN_00623880` | Edit-specific text/selection/password/caret helper. | decompile `0x00623880` | Yes |
| `0x006ACEE0` Start | Reads `0x6A0` through `0x4B3`, stores `DAT_00A8B380`, then packs local session fields. | `0x006AD375..0x006AD39F` | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current status | Binary mismatch |
|---|---|---|
| `src/ui/skirmish_shell/layout.rs` | Has `layout.player_name` and already applies `x+1,w+1` to `(58,59,151,23)`. | Geometry matches final child rect, but no inner edit client/frame/text inset state is represented. |
| `src/app_skirmish_shell_render.rs` | Renders literal `"Player"` into `layout.player_name` with generic label helper. | Mismatch: binary renders the editable player-name string from state/global, with primitive edit frame, left+2 text inset, edit text helper, selection/caret behavior, and yellow text. |
| `src/ui/skirmish_shell/state.rs` | No player-name field or edit focus/caret/input state found in the scanned Skirmish shell state. | Missing: typed text, 19-char cap, focus/caret, Start readback into launch options/session. |
| `src/app.rs` | Shell input dispatch has Skirmish buttons/combos/trackbars but no observed player-name text editing path in the scanned surfaces. | Missing keyboard/text-event route for `0x6A0`. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard `0x102` reachability | verified | prior launcher reports; `0x006AE3F0` decompile | none |
| `0x6A0` resource class and final rect | verified-by-prior | `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md` | none |
| `0x6A0` setup text source and limit | verified | `0x006AE6F2..0x006AE735` | none |
| `Edit` vs `NewEdit` callback selection | verified | `0x0060FCF0..0x0060FD80`; resource class `Edit` | none |
| `OwnerDraw_Edit_00614190` `0x497` geometry inset | verified | `0x0061433D..0x00614373` | none |
| `OwnerDraw_Edit_00614190` paint path | verified | `0x0061462C..0x00614760`; `0x00623880` | runtime screenshot RGB sampling only |
| `OwnerDraw_Edit_00614190` focus/key invalidation | verified | `0x006143B9..0x006145AD`, `0x0061477C..0x00614872` | exact `0x4B0` producer semantics are not expanded |
| `OwnerDraw_Edit_00614190` `0x4B3` get text | verified | `0x00614935..0x00614974` | none |
| `OwnerDraw_NewEdit_00614B30` contrast | verified enough for non-use on `0x6A0` | decompile `0x00614B30`; resource class `Edit` | full online/custom edit consumers out of scope |
| Start readback into `DAT_00A8B380` | verified | `0x006AD375..0x006AD39F` | none |
| Standard disabled state for `0x6A0` | verified absent in standard path | `0x006AE6E0`, `0x006ACEE0`, edit paint style bits | binary-wide nonstandard disabling out of scope |
| Rust render/input/state delta | verified by scan | `layout.rs`, `state.rs`, `app_skirmish_shell_render.rs`, `app.rs` | implementation pending |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is dialog 0x102 active in standard offline YR? -> Yes; prior launcher reports and `0x006AE3F0` show standard init/command routing.` (evidence: `0x006AE3F0`; prior complete rect/start reports)
- `[RESOLVED] OQ-02 - Is child 0x6A0 present and visible? -> Yes; resource row class `Edit`, role player name, final rect `(58,59,151,23)`.` (evidence: `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-03 - What initializes the player name text? -> `0x006AE6E0` sends `0x4B2` with `0x00735120(DAT_00A8B380)`.` (evidence: `0x006AE6F2..0x006AE735`)
- `[RESOLVED] OQ-04 - What is the max length? -> setup sends `EM_SETLIMITTEXT` with `0x13`, and Start reads a `0x14`-wide-char buffer with forced terminator.` (evidence: `0x006AE70A..0x006AE714`, `0x006AD381..0x006AD38E`, `0x00614935..0x00614974`)
- `[RESOLVED] OQ-05 - Does 0x6A0 use NewEdit? -> No; resource class is `Edit`, and `FUN_0060F9A0` maps ordinary edit to `0x00614190`.` (evidence: `0x0060FCF0..0x0060FD80`; resource row)
- `[RESOLVED] OQ-06 - What are edit insets? -> final child rect gets prior `x+1,w+1`; owner init moves native client `+1,+1,w-2,h-2`; paint text starts at left `+2`.` (evidence: complete child matrix; `0x0061433D..0x00614373`; `0x00614710..0x00614760`)
- `[RESOLVED] OQ-07 - What color is edit text? -> `DAT_00AC18A4`, initialized to `0xFFFF`, is passed to the edit text painter.` (evidence: `0x0060F9A0`; `0x00614731..0x00614760`)
- `[RESOLVED] OQ-08 - Is there a standard disabled visual branch? -> No standard `0x102` disable writer found, and paint passes style bit `0x20` as password mask rather than `WS_DISABLED`.` (evidence: `0x00614728..0x00614760`; setup/start decompiles)
- `[RESOLVED] OQ-09 - Is caret behavior present? -> Yes; edit paint helper draws a 2px caret when cursor visibility is enabled and no selection is active; focus/key/mouse messages invalidate the control.` (evidence: `0x00623880`; `0x0061477C..0x00614872`)
- `[RESOLVED] OQ-10 - How is edited text committed on Start? -> `0x006ACEE0` sends `0x4B3` to `0x6A0`, then copies the local wide buffer to `DAT_00A8B380`.` (evidence: `0x006AD375..0x006AD39F`)
- `[RESOLVED] OQ-11 - Are INI keys involved? -> No; this is shell HWND/message/global-string behavior.` (evidence: no INI reads in decompiled setup/paint/readback; INI grep found no relevant key)
- `[RESOLVED] OQ-12 - Does current Rust already model the editable name? -> No scanned Skirmish state field/input route was found; render draws literal "Player" at `layout.player_name`.` (evidence: `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/state.rs`, `src/app.rs`)
- `[RESOLVED by SKIRMISH_PLAYER_NAME_EDIT_FOCUS_MESSAGES_0X4B0_0X4AF_GHIDRA_REPORT.md] OQ-13 - Old `Edit` posts `0x4B0` to itself on `WM_SETFOCUS` after `EM_SETSEL(-1,-1)`; delivery triggers the edit callback entry guard, which marks focus-restore-needed and moves focus to `g_hWnd`; the common shell subclass path later enumerates child HWNDs and sends `0x4AF`, which old `Edit` consumes by setting restore-in-progress, restoring focus/style, and returning handled. Rust should model repaint-stable player-name edit focus/caret/input state, not literal Win32 custom messages.`
- `[DEFERRED] OQ-14 - Final captured RGB for edit yellow/caret after 16-bit display conversion.` (category: `needs-runtime-debugger`; reason: binary source color is verified but final display sampling was not captured; next-step-if-pursued: retail screenshot pixel sample of focused/unfocused player-name edit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x6A0` is an editable player-name field seeded from `DAT_00A8B380`, not a static `"Player"` label | `0x006AE6F2..0x006AE735`; `0x006AD375..0x006AD39F` | missing/mismatch | `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs` | Add player-name state, keyboard/text input focus for the edit rect, 19-char cap, and Start readback into launch/session settings | Open Skirmish, focus name field, type a 19-char name, Start stores that exact name and rejects/truncates additional chars | Do not leave a hardcoded `"Player"` label in the edit rect; proposed test `skirmish_player_name_edit_accepts_text_and_start_commits_19_char_limit` |
| Edit visual uses final rect `(58,59,151,23)`, client/frame inset, primitive frame, left+2 text inset, yellow edit text, and caret via edit painter | complete rect report; `0x0061433D..0x00614373`; `0x006146D0`; `0x00614710..0x00614760`; `0x00623880` | missing visual frame/caret; rect present | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs` | Render edit frame and player text using edit-specific inset rather than generic label rect; show focus/caret state when active | At 800x600, the name edit occupies `(58,59,151,23)`, text begins inside the frame, and focused empty/typed field shows a 2px caret without layout shift | Do not reuse static-label top-anchor or combo text rects for this control; proposed test `skirmish_player_name_edit_uses_binary_frame_inset_and_caret_rect` |
| Standard offline `0x102` uses old `Edit` callback `0x00614190`; `NewEdit` is not the player-name class | `0x0060FCF0..0x0060FD80`; resource class row | none if implementing behavior directly; risk in docs/design | docs/specs and future owner-draw renderer tests | Model only observed old-edit output for `0x6A0`; keep `NewEdit` as contrast unless a separate consumer is verified | A regression test names the callback/source and asserts `0x6A0` is ordinary edit-class behavior | Do not import `NewEdit`-only max-length/caret timer details as proof of `0x6A0`; proposed test `skirmish_player_name_uses_ordinary_edit_ownerdraw_contract` |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`: replace the deferred `OQ-10` wording with: "`[RESOLVED by SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md] Player-name/edit control `0x6A0` is ordinary resource class `Edit`, routed to `OwnerDraw_Edit_00614190`, seeded from `DAT_00A8B380`, rendered through the edit frame/text/caret helper with left+2 text inset and `DAT_00AC18A4` yellow, capped at 19 characters, and read back by Start through `0x4B3` into `DAT_00A8B380`.`"
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`: replace the implementation-status row "`player_name` text | Unchecked..." with: "`player_name` edit `0x6A0` | Mismatch: current Rust draws a static literal in the edit rect; binary uses editable `DAT_00A8B380` text, primitive edit frame, left+2 text inset, 19-character cap, focus/caret behavior, and Start readback.`"
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`: refine the `EM_SETLIMITTEXT` row to: "`EM_SETLIMITTEXT (0xC5)` is sent to Skirmish `0x6A0` with `wParam=0x13`; because `0x6A0` is old `Edit`/`0x00614190`, this message is forwarded to the native edit WndProc rather than stored by `NewEdit`'s record field. `NewEdit` has its own explicit max-length storage for other shell edit controls.`"

### Negative Facts / Do Not Do

- Do not treat `0x6A0` as static text. Active in YR: No. Evidence: RT_DIALOG class `Edit`; `FUN_0060F9A0` maps edit class to `OwnerDraw_Edit_00614190`; Start reads the control through `0x4B3`.
- Do not use `OwnerDraw_NewEdit_00614B30` as the direct offline player-name callback. Active in YR: No for `0x6A0`. Evidence: resource class `Edit`, not `NewEdit`; callback selection `0x0060FCF0..0x0060FD80`.
- Do not hardcode the displayed name to `"Player"`. Active in YR: No. Evidence: setup sources `DAT_00A8B380`; Start copies edited text back into `DAT_00A8B380`.
- Do not omit the edit frame/caret just because the current Rust layout rect is correct. Active in YR: Yes, frame/caret path exists. Evidence: `0x006146D0`, `0x00614760`, `0x00623880`.
- Do not add speculative normal disabled visuals for this field. Active in YR: Conditional only. Evidence: standard setup/start paths do not disable `0x6A0`; paint uses style bit `0x20` as password flag, not `WS_DISABLED`.

### Remaining Uncertainty

- Old-Edit `0x4B0` / `0x4AF` focus restoration is covered by `SKIRMISH_PLAYER_NAME_EDIT_FOCUS_MESSAGES_0X4B0_0X4AF_GHIDRA_REPORT.md`; remaining uncertainty is only exact native previous-proc return for delivered `0x4B0` and full shell-global frame scheduling for every `0x4AF` broadcast condition.
- Final captured RGB for edit text/caret after the 16-bit display/capture path needs runtime sampling if pixel-perfect screenshots are required.

## Sources

- Ghidra read-only decompile: `FUN_0060F9A0 @ 0x0060F9A0`, `OwnerDraw_Edit_00614190 @ 0x00614190`, `OwnerDraw_NewEdit_00614B30 @ 0x00614B30`, `FUN_00623880 @ 0x00623880`, `FUN_006AE3F0 @ 0x006AE3F0`, `FUN_006AE6E0 @ 0x006AE6E0`, `FUN_006ACEE0 @ 0x006ACEE0`, `FUN_00735120 @ 0x00735120`.
- Ghidra assembly contexts: `0x006AE6F2..0x006AE735`, `0x0061433D..0x006145AD`, `0x0061462C..0x00614760`, `0x00614935..0x00614974`, `0x00614C87..0x00614D20`, `0x00614F02..0x00615020`, `0x00615331..0x00615380`, `0x006AD375..0x006AD39F`.
- Prior reports checked: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`.
- Rust surfaces scanned: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`.
- INI files checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`; no player-name edit key found.
