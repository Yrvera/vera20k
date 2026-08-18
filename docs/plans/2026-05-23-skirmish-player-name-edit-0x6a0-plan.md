# Skirmish Player-Name Edit 0x6A0 Implementation Plan

> Execute task-by-task. Do not make the experimental shell the default in this plan.

**Goal:** Replace the static Skirmish shell `"Player"` label with the verified retail `0x6A0` editable player-name control, including focus, 19-character cap, caret/text rendering, and Start readback.

**Design Doc:** [docs/plans/2026-05-23-skirmish-player-name-edit-0x6a0-design.md](2026-05-23-skirmish-player-name-edit-0x6a0-design.md)

---

## Grounding Summary

Primary design:

- `docs/plans/2026-05-23-skirmish-player-name-edit-0x6a0-design.md`

Verified research:

- `docs/research/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_FOCUS_MESSAGES_0X4B0_0X4AF_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`

Current Rust surfaces:

- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/mod.rs`
- `src/app.rs`
- `src/app_skirmish_shell_render.rs`
- `src/render/shell_text.rs`
- `src/render/bit_font.rs`
- `src/skirmish_launch.rs`
- `src/app_skirmish.rs`

Existing broader plan overlap:

- `docs/plans/2026-05-23-skirmish-shell-default-battle-path-plan.md` contains broader player-name tasks, but mixes them with default-route, validation modal, and Battle startup work. This plan is the narrow executable slice for `0x6A0` only.

## Key Decisions

- Keep the shell behind the current native/dev route. This plan only fixes the player-name control.
- Add a focused `PlayerNameEditState` under `SkirmishShellState`; do not add a generic edit widget yet.
- Store edit positions as character indices, not byte indices.
- Enforce the 19-character cap during editing, not only during Start.
- Track horizontal text scroll so long names keep the caret visible with the verified 5px margin.
- Filter committed control text such as Enter, Tab, carriage return, and newline before insertion.
- Model stable focus/caret behavior directly; do not add a literal Win32 `0x4B0`/`0x4AF` message bus.
- Render the control through the shell renderer, not egui.
- Carry the committed name through launch data so app setup stops hardcoding local owner `"Player"` for shell-launched sessions.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/ui/skirmish_shell/state.rs` | Add edit state, focus/text/key helpers, hit testing, and Start readback. |
| Modify | `src/ui/skirmish_shell/mod.rs` | Re-export any new public helper needed by app/render tests. |
| Modify | `src/app.rs` | Route mouse, committed text, and focused edit keys while Skirmish shell is active. |
| Modify | `src/app_skirmish_shell_render.rs` | Render edit frame, text inset, selection/caret, and remove static `"Player"` draw. |
| Modify if needed | `src/render/bit_font.rs` or `src/render/shell_text.rs` | Add a tested prefix-width helper for caret x calculation. |
| Modify | `src/skirmish_launch.rs` | Add local player name to data-only launch contract. |
| Modify | `src/app_skirmish.rs` | Use session local player name for shell-launched owner name. |

## Parity-Critical Items

| Item | Verification |
|---|---|
| `0x6A0` is editable ordinary `Edit`, not static text or `NewEdit` | State/render tests must remove literal `"Player"` draw and name ordinary edit report in test comments. |
| Initial text comes from shell state, not render literal | Default-state test plus render test that draws from mutated state. |
| 19-character cap from `EM_SETLIMITTEXT 0x13` | Insertion test with overlong input. |
| First focus selects all text before typing | Test typing after focus replaces default instead of appending. |
| Start reads the edit buffer | `launch_session` test with edited name. |
| Final rect remains `(58,59,151,23)` at 800x600 | Existing layout test plus edit text/caret rect test. |
| Text uses edit-specific inset, not generic label rect | Render helper rect test. |
| Long names scroll horizontally to keep the cursor visible with a 5px margin | State/render tests for caret visibility after over-width insertion and left/right movement. |
| Enter/Tab/control text is not inserted into the name | Input helper tests using `\r`, `\n`, and `\t`. |
| Focus survives shell redraw/control changes | State/app routing test around option changes or dropdown close. |
| Focused edit shows caret | Render helper test for caret primitive or sprite. |
| No literal Win32 custom-message bus | Code review check: state helpers model focus directly. |

---

## Tasks

### Task 1: Add `PlayerNameEditState`

**Why:** The shell needs an editable state source before input, render, or Start can stop using a static label.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/mod.rs` if exports are needed

**Steps:**

1. Add `PLAYER_NAME_MAX_CHARS: usize = 19`.
2. Add `PlayerNameEditState` with:
   - `text: String`
   - `focused: bool`
   - `selection: Option<(usize, usize)>`
   - `caret: usize`
   - `scroll_x: i32`
3. Add `player_name_edit: PlayerNameEditState` to `SkirmishShellState`.
4. Default the text to `"Player"` in state initialization only.
5. Add character-index helpers for clamping caret/selection and converting character ranges to byte ranges.
6. Add a pure helper for updating horizontal `scroll_x` from text-rect width, caret prefix width, and the verified 5px cursor margin.

**Tests:**

- `player_name_default_is_state_not_render_literal`
- `player_name_caret_and_selection_are_clamped_to_char_count`
- `player_name_char_indices_handle_utf8_without_byte_split`
- `player_name_scroll_starts_at_zero_and_clamps_nonnegative`

**Checks:**

```powershell
cargo test -q player_name --lib
```

### Task 2: Implement Focus And Text Editing Helpers

**Why:** Retail focus selects all first, typing replaces selected text, and text is capped while editing.

**Files:**

- `src/ui/skirmish_shell/state.rs`

**Steps:**

1. Add `player_name_edit_rect_hit(layout, x, y) -> bool`.
2. Add `focus_player_name_edit` that focuses the edit and selects the full current text.
3. Add `blur_player_name_edit` that clears focus and collapses selection.
4. Add `insert_player_name_text`, replacing active selection first and truncating inserted characters to the 19-character cap.
5. Filter out `\r`, `\n`, `\t`, and other `char::is_control()` characters before inserting text.
6. Add backspace/delete helpers that remove selection first, then neighboring character.
7. Add left/right/home/end helpers for collapsed caret movement. Shift-selection can be added only if it stays local and tested; mouse-drag selection is not required by current evidence.
8. Update horizontal `scroll_x` after every insertion, deletion, selection collapse, and caret movement so the caret remains visible with the verified 5px margin.
9. Add a helper to close dropdown/drag transient state when the edit takes focus.

**Tests:**

- `player_name_focus_selects_all_existing_text`
- `player_name_typing_after_focus_replaces_default`
- `player_name_insert_caps_at_19_chars`
- `player_name_enter_and_tab_do_not_insert_control_text`
- `player_name_backspace_removes_selection_before_previous_char`
- `player_name_delete_removes_selection_before_next_char`
- `player_name_caret_keys_move_within_bounds`
- `player_name_scroll_keeps_caret_visible_with_5px_margin`
- `player_name_scroll_moves_back_left_when_caret_moves_left`

**Checks:**

```powershell
cargo test -q player_name --lib
```

### Task 3: Carry Player Name Through Launch Data

**Why:** Retail Start reads `0x6A0`; Rust should commit the edited shell buffer instead of preserving a hardcoded owner name.

**Files:**

- `src/skirmish_launch.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/app_skirmish.rs`

**Steps:**

1. Add `player_name: String` to `SkirmishLocalSlot`.
2. Update every construction site and test fixture for `SkirmishLocalSlot`.
3. In `launch_session`, set `local.player_name` from `state.player_name_edit.text`.
4. In shell-launched Battle setup, use `session.local.player_name` for the local slot owner name instead of hardcoding `"Player"`.
5. Preserve fallback behavior for non-session legacy skirmish paths if they still exist.

**Tests:**

- `launch_session_reads_player_name_edit_text`
- `skirmish_launch_session_carries_local_player_name`
- `battle_setup_uses_session_local_player_name_for_owner`

**Checks:**

```powershell
cargo test -q skirmish_launch --lib
cargo test -q app_skirmish --lib
```

### Task 4: Route Skirmish Shell Mouse And Keyboard Input

**Why:** The edit must receive focus and text input only while the native Skirmish shell is active.

**Files:**

- `src/app.rs`
- `src/ui/skirmish_shell/state.rs`

**Steps:**

1. In Skirmish shell mouse-down routing, test `layout.player_name` before owner-draw buttons/options.
2. If hit, focus the edit, clear dropdown/drag/button transient state, request redraw, and consume the click.
3. If another shell control is clicked, blur the edit only after the clicked control has had a chance to consume focus/action as needed.
4. Route `WindowEvent::KeyboardInput` pressed events with `KeyEvent.text` to `insert_player_name_text` when the Skirmish shell is active and the edit is focused.
5. Do not insert `KeyEvent.text` for named/control keys after filtering. In particular, winit can report Enter text as `"\r"`; that must be handled as a control key, not a name character.
6. Route Backspace/Delete/Left/Right/Home/End to edit helpers while focused.
7. Keep Escape behavior unchanged for now: it cancels/exits the shell path. If later RE proves focused old `Edit` consumes Escape, that becomes a separate correction.
8. Request redraw after edit mutations.

**Tests:**

- `skirmish_player_name_edit_focus_changes_on_rect_click`
- `skirmish_player_name_edit_consumes_text_only_when_focused`
- `skirmish_player_name_enter_does_not_insert_carriage_return`
- `skirmish_player_name_tab_does_not_insert_tab`
- `skirmish_player_name_focus_survives_shell_redraw_and_keeps_text_input`
- `skirmish_player_name_clicking_other_control_blurs_or_transfers_focus`

**Checks:**

```powershell
cargo test -q skirmish_player_name --lib
```

### Task 5: Add Text Measurement Support For Caret Placement

**Why:** The focused edit needs a caret at the visible text position, and duplicating glyph width math in the Skirmish renderer would be brittle.

**Files:**

- `src/render/bit_font.rs`
- `src/render/shell_text.rs`
- `src/app_skirmish_shell_render.rs`

**Steps:**

1. Reuse `BitFont::text_width` for prefix measurement unless caret placement needs a wrapper for visibility or naming.
2. If a wrapper is needed, keep it thin and tested against `text_width`.
3. Keep measurement behavior consistent with `wrap_layout` and `build_text`.
4. Use the measured prefix width minus `PlayerNameEditState::scroll_x` for caret x calculation in the edit renderer.

**Tests:**

- `bit_font_text_width_matches_player_name_prefix_measurement`
- `player_name_caret_x_uses_measured_prefix_width`
- `player_name_caret_x_subtracts_horizontal_scroll`

**Checks:**

```powershell
cargo test -q bit_font --lib
cargo test -q player_name --lib
```

### Task 6: Render Edit Frame, Text, Selection, And Caret

**Why:** Current Rust draws a plain static label. Retail paints an owner-draw edit frame, yellow text, and focused caret.

**Files:**

- `src/app_skirmish_shell_render.rs`
- `src/render/shell_text.rs` only if helper exposure is needed

**Steps:**

1. Remove the generic `push_label_draw(..., "Player", layout.player_name, ...)` call.
2. Add `player_name_edit_text_rect(layout.player_name)` using verified edit inset: frame/client inset plus left `+2`.
3. Render a primitive edit frame at `layout.player_name` using existing shell primitive/bevel color conventions.
4. Draw `shell.player_name_edit.text` in shell yellow from edit state with `scroll_x` applied to the text origin.
5. Draw selection background if `selection` is non-empty and focused.
6. Draw a 2-pixel caret when focused and selection is collapsed, using measured prefix width minus `scroll_x`.
7. Keep caret stable/visible initially. Exact blink timing is deferred until a timing investigation verifies old-Edit cadence.

**Tests:**

- `player_name_edit_text_rect_uses_verified_inset`
- `skirmish_player_name_edit_uses_binary_frame_inset_and_caret_rect`
- `player_name_edit_applies_horizontal_scroll_to_text_and_caret`
- `player_name_edit_renders_caret_when_focused`
- `player_name_edit_does_not_emit_static_player_label`

**Checks:**

```powershell
cargo test -q app_skirmish_shell_render --lib
cargo test -q player_name --lib
```

### Task 7: Focused Regression And Manual Visual Check

**Why:** This is player-visible UI. Unit tests catch state and launch contracts, but visual parity still needs a screen check.

**Files:**

- no planned code changes unless this task finds defects

**Steps:**

1. Run focused tests:

```powershell
cargo test -q skirmish_shell --lib
cargo test -q skirmish_launch --lib
cargo test -q app_skirmish --lib
cargo test -q app_skirmish_shell_render --lib
```

2. Start the app and open the native Skirmish shell.
3. At 800x600 or a known shell layout, verify the edit appears at the existing `layout.player_name` rect and no static `"Player"` label is drawn over it.
4. Click the edit and type a replacement name.
5. Confirm first typing after focus replaces the default.
6. Confirm names longer than 19 characters are capped while typing.
7. Confirm the caret remains visible near the right edge while typing a long 19-character name.
8. Confirm Enter and Tab do not add visible control characters to the name.
9. Click other shell controls, then return and continue typing.
10. Start Game and verify the launched local owner/player name uses the edited value.

**Acceptance:**

- Focused tests pass.
- Manual check confirms edit input, visible frame/text/caret, cap, and Start readback.
- No shell default-route behavior changes in this plan.

---

## Final Acceptance Criteria

- `0x6A0` no longer renders as a hardcoded static `"Player"` label.
- The Skirmish shell has editable player-name state with first-focus select-all behavior.
- Printable input replaces selection and respects the 19-character cap.
- Enter, Tab, carriage return, newline, and other control characters are not inserted into the name.
- Backspace/delete/caret movement are bounded and non-panicking.
- Horizontal scrolling keeps the caret visible for over-width names using the verified 5px margin.
- Focused edit state survives shell redraw/control interaction well enough for continuous typing.
- Rendered edit uses the verified rect/inset, shell-yellow text, primitive frame, and focused caret.
- Start reads the edit state and carries the name through `SkirmishLaunchSession`.
- Shell-launched Battle setup uses the committed local player name.
- No generic shell edit framework, egui text field, or Win32 custom-message bus is introduced.
