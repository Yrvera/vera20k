# Skirmish Player-Name Edit 0x6A0 Design

## Goal

Replace the current static `"Player"` label in the Skirmish shell with the verified retail player-name edit control behavior for child `0x6A0`.

## Architecture Context

The current Skirmish shell already has the correct ownership split for this feature.

- `src/ui/skirmish_shell/layout.rs` owns the verified Skirmish shell geometry. `SkirmishShellLayout::player_name` already applies the `0x102` one-pixel fixup and resolves to `(58,59,151,23)` at 800x600.
- `src/ui/skirmish_shell/state.rs` owns shell state, hit testing, transient control state, combo/trackbar interaction, and conversion into `SkirmishLaunchSession`.
- `src/app_skirmish_shell_render.rs` owns shell sprite/text assembly. It currently draws literal `"Player"` into `layout.player_name` via the generic label path.
- `src/app.rs` routes Skirmish shell mouse events and Start/Choose/Back actions. It does not yet route text input or edit-focused key handling to the shell.
- `src/skirmish_launch.rs` is the data-only launch contract. It currently has no local player display-name field, and downstream skirmish startup still uses the hardcoded local owner name `"Player"`.

This work stays above `sim/`. UI state belongs in `ui/skirmish_shell`, rendering belongs in `app_skirmish_shell_render.rs`, input routing belongs in `app.rs`, and the committed name crosses the boundary as data in `skirmish_launch.rs`.

## Impact Analysis

Expected touched files:

| File | Responsibility |
|---|---|
| `src/ui/skirmish_shell/state.rs` | Add `PlayerNameEditState`, focus/text/key helpers, click handling, and launch readback from the edit buffer. |
| `src/app_skirmish_shell_render.rs` | Replace the generic `"Player"` label draw with edit-frame/text/caret rendering. |
| `src/app.rs` | Route Skirmish text/key input to `PlayerNameEditState` while the edit has focus; clear focus on modal/open shell transitions where appropriate. |
| `src/skirmish_launch.rs` | Add a data-only local-player name field to the launch session/local slot if the downstream owner name should be committed past shell state. |
| `src/app_skirmish.rs` | Consume the launch local player name instead of hardcoding `"Player"` for Battle-mode shell sessions. |

Risk areas:

- Text input must be captured only while the Skirmish shell is active and the edit is focused, so in-game hotkeys and main-menu shell buttons are not affected.
- Winit text input details are platform-sensitive. The implementation should prefer committed key text where available, handle control keys separately, and filter control characters such as `\r`, `\n`, and `\t` before inserting into the name buffer.
- Retail stores a narrow global string but paints via wide shell text. Rust can store UTF-8 internally, but the visible cap must match a 19-character player-name limit rather than arbitrary bytes.
- The current `shell_text::draw_in_rect` can draw text with scissor and alignment, but it does not yet expose selection/caret drawing. Caret/selection should be rendered as shell overlay primitives/sprites in the edit helper, not by changing global font rendering behavior unless needed.
- Exact final RGB after 16-bit display conversion remains runtime-sampling uncertainty. Use the verified source shell yellow until a screenshot sample supersedes it.

## Chosen Approach

Build a focused `0x6A0` edit state inside `SkirmishShellState`, not a generic edit-control framework and not a literal Win32 message bus.

The state module models the player-visible contract directly:

- the current edit text;
- whether the edit is focused;
- current selection/caret and the horizontal text scroll needed to keep the caret visible;
- 19-character insertion cap;
- first-focus select-all behavior;
- replacement of selected text on printable input;
- stable focus through shell redraws.

The render module draws the verified edit visual from that state: primitive frame, text inset, shell-yellow text, and focused caret/selection. `app.rs` supplies input events and does not own edit semantics.

This approach matches the existing shell architecture: state helpers already own combo and trackbar interactions, while render code reads shell state and app code routes platform events.

## Tiny-Detail Ledger

- `0x6A0` is active in standard offline Yuri's Revenge Skirmish and is an ordinary resource class `Edit`, not static text and not `NewEdit`. Source: `docs/research/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`.
- Setup seeds the control from `DAT_00A8B380`, converted through the shell text path; Rust must not hardcode `"Player"` at render time. Source: `0x006AE6F2..0x006AE735`.
- Setup sends `EM_SETLIMITTEXT (0xC5)` with `wParam=0x13`, so the accepted visible name is capped at 19 characters. Source: `0x006AE70A..0x006AE714`.
- Start reads `0x6A0` via custom get-text message `0x4B3`, `wParam=0x14`, then copies the local wide buffer back to `DAT_00A8B380`. Source: `0x006AD375..0x006AD39F`.
- The final child rect is `(58,59,151,23)` at 800x600 after the `0x102` one-pixel fixup. Source: `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`; `SKIRMISH_SHELL_640X480_FINAL_VISIBLE_LAYOUT_TRACE.md`.
- Owner init moves the native edit client inward by `+1,+1` and shrinks by `-2,-2`, producing the edit-frame margin. Source: `OwnerDraw_Edit_00614190 @ 0x0061433D..0x00614373`.
- Paint passes text beginning at left `+2` to the edit text helper. Source: `OwnerDraw_Edit_00614190 @ 0x00614710..0x00614760`.
- Edit text color is `DAT_00AC18A4`, initialized to `0xFFFF` in the shell path. Source: `FUN_0060F9A0`; `OwnerDraw_Edit_00614190 @ 0x00614731..0x00614760`.
- The paint path includes primitive edit frame rendering and edit-helper caret drawing; focused edit cannot look like a plain static label. Source: `0x006146D0`; `FUN_00623880 @ 0x00623880`.
- The edit helper scrolls horizontally to keep the cursor visible with a 5px margin. Source: `FUN_00623880 @ 0x00623880`.
- On first focus, old `Edit` sends `EM_SETSEL(-1,-1)` before posting the deferred focus message, so typing replaces the current name instead of appending after it. Source: `0x006143BE..0x006143FA`.
- Tab and Enter are intercepted by old `Edit` behavior and must not be inserted into the player-name buffer as literal `\t` or `\r` characters. Source: `OwnerDraw_Edit_00614190 @ 0x006143B9..0x00614555`.
- `0x4B0`/`0x4AF` focus choreography is active but Rust should model the observable result: repaint-stable text input focus, selection, and caret continuity. Source: `SKIRMISH_PLAYER_NAME_EDIT_FOCUS_MESSAGES_0X4B0_0X4AF_GHIDRA_REPORT.md`.
- Mouse/context/focus/key paths invalidate/redraw the edit; Rust should request redraw after edit state changes. Source: `OwnerDraw_Edit_00614190 @ 0x0061477C..0x00614872`.
- Standard offline `0x102` does not verify a disabled visual branch for this control; do not add speculative disabled coloring. Source: `SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`.

## Design

### Components

#### `PlayerNameEditState`

Add a focused state object in `src/ui/skirmish_shell/state.rs`:

```rust
pub struct PlayerNameEditState {
    pub text: String,
    pub focused: bool,
    pub selection: Option<(usize, usize)>,
    pub caret: usize,
    pub scroll_x: i32,
}
```

The indices should be character positions, not byte offsets. Helper methods can translate to byte offsets internally when replacing UTF-8 ranges. The maximum accepted length is 19 characters.

Default text should come from the same source that represents the persisted local player name in Rust. Until a profile/options source exists, use `"Player"` as the initial value in state initialization only. Rendering must always read state.

#### State Helpers

Add narrow helpers rather than exposing mutation details to `app.rs`:

- `player_name_edit_rect_hit(layout, x, y) -> bool`
- `focus_player_name_edit(state)`
- `blur_player_name_edit(state)`
- `insert_player_name_text(state, text)`
- `handle_player_name_backspace(state)`
- `handle_player_name_delete(state)`
- `handle_player_name_left/right/home/end(state, shift)`
- `player_name_for_launch(state) -> String`

On focus, set `focused = true`, select the full text, and put the caret at the end of the selected range. On printable input, replace the selection first, then insert only as many characters as fit under the 19-character cap. Backspace/delete remove the active selection first.

After any edit or caret movement, update `scroll_x` so the caret remains visible inside the edit text rect with the verified 5px cursor margin. This can be a pure helper that takes the text rect width and the measured caret prefix width; it should clamp back toward zero when the caret moves left.

The first implementation can keep selection support to "select all" plus collapsed caret if mouse dragging selection is not yet verified. That still preserves the currently verified Start/readback and first-typing behavior. If mouse selection is later verified, it can extend the same state object.

#### Input Routing

In `app.rs`, when `Self::skirmish_shell_active(state)`:

- left-click inside `layout.player_name` focuses the edit, closes combo dropdowns, clears button/drag transient state, and consumes the click;
- left-click outside the edit blurs it unless the click opens/uses another control;
- committed key text is routed to `insert_player_name_text` only while focused, after filtering `\r`, `\n`, `\t`, and other control characters;
- keyboard control keys route to edit helpers while focused;
- Escape keeps the existing shell-cancel behavior unless retail evidence says focused edit consumes it differently.

The edit path should run before shell/global hotkeys for printable input while focused. It should not affect in-game hotkeys because the Skirmish shell is only active on the main-menu shell path.

#### Rendering

Replace the generic `push_label_draw(..., "Player", layout.player_name, ...)` call with an edit-specific helper:

- derive an outer rect from `layout.player_name`;
- draw the primitive edit frame using the same bevel color constants already used for owner-draw/combo primitives, or add a small primitive rectangle helper if needed;
- derive the text rect from the verified edit inset: outer/client frame inset plus text left `+2`;
- draw `shell.player_name_edit.text` in shell yellow with left alignment, scissor clipping, and `scroll_x` applied to the text origin;
- if focused, draw the selection background when `selection` is non-empty and a two-pixel caret at the current caret x when selection is collapsed.

Caret x can be computed by measuring the visible prefix with `BitFont`/existing shell text measurement APIs, then subtracting `scroll_x`. If the font API lacks a direct prefix-width helper, add that helper in `render/bit_font` or `render/shell_text` with tests; do not duplicate glyph width math in the Skirmish renderer.

Blink timing is not yet fully verified for old `Edit`. The implementation may start with a stable visible caret while focused, then add blink if a later timing investigation pins exact cadence. The design must not omit caret entirely.

#### Launch Readback

Retail reads the edit text on Start, not from a separate static label. Rust should mirror this by making `launch_session` read `state.player_name_edit.text`.

Preferred data flow:

```text
PlayerNameEditState.text
  -> launch_session(...)
  -> SkirmishLaunchSession.local.player_name
  -> app_skirmish Battle setup owner_name
```

If the launch contract is extended, keep it data-only:

```rust
pub struct SkirmishLocalSlot {
    pub player_name: String,
    pub country: LaunchCountry,
    pub color_index: u8,
    pub start_position: LaunchStartPosition,
    pub team: LaunchTeam,
}
```

Downstream setup should use this value instead of hardcoding `"Player"` for shell-launched games.

### Interfaces / Contracts

- `SkirmishShellState` owns edit semantics. `app.rs` only passes mouse/text/key events into helper functions.
- Render code reads `PlayerNameEditState` and layout; it does not mutate text or focus.
- `SkirmishLaunchSession` carries committed player-name data without UI/control IDs.
- No shell edit state crosses into `sim/`; the simulation only sees house names through existing launch/setup data.

### Data Flow

```text
Skirmish shell opens
  -> SkirmishShellState::default seeds PlayerNameEditState.text
  -> render draws edit frame + state text

Mouse click inside layout.player_name
  -> focus_player_name_edit
  -> select all text
  -> request redraw

Text/key input while focused
  -> state helper mutates text/selection/caret under 19-char cap
  -> request redraw

Start Game
  -> launch_session reads PlayerNameEditState.text
  -> SkirmishLaunchSession carries local player_name
  -> app_skirmish uses local player_name as owner_name
```

### Error Handling

The edit helper should be total and non-panicking:

- clamp caret/selection to character count after every mutation;
- ignore unsupported control keys;
- ignore committed control text such as Enter, Tab, carriage return, newline, and other `char::is_control()` characters;
- reject excess inserted characters beyond the 19-character cap;
- allow an empty string unless future Start-validation evidence proves retail rejects it.

No user-facing error is needed for over-limit text; retail simply caps the edit.

### Testing Strategy

Unit tests in `src/ui/skirmish_shell/state.rs`:

- `player_name_default_is_state_not_render_literal`
- `player_name_focus_selects_all_existing_text`
- `player_name_typing_after_focus_replaces_default`
- `player_name_insert_caps_at_19_chars`
- `player_name_enter_and_tab_do_not_insert_control_text`
- `player_name_backspace_removes_selection_before_previous_char`
- `player_name_click_focus_survives_option_state_changes`
- `player_name_scroll_keeps_caret_visible_with_5px_margin`
- `launch_session_reads_player_name_edit_text`

Render/helper tests in `src/app_skirmish_shell_render.rs` or a small render helper module:

- `player_name_edit_text_rect_uses_verified_inset`
- `player_name_edit_applies_horizontal_scroll_to_text_and_caret`
- `player_name_edit_renders_caret_when_focused`
- `player_name_edit_does_not_emit_static_player_label`

Launch tests in `src/skirmish_launch.rs` / `src/app_skirmish.rs`:

- `skirmish_launch_session_carries_local_player_name`
- `battle_setup_uses_session_local_player_name_for_owner`

Focused command set after implementation:

```powershell
cargo test -q skirmish_shell
cargo test -q skirmish_launch
cargo test -q app_skirmish
cargo test -q app_skirmish_shell_render
```

Manual verification:

- open Skirmish shell;
- click player-name edit;
- type a new 19-character name and confirm extra input is ignored;
- click other controls and return to the edit;
- Start Game and confirm the launched local house/player owner uses the edited name.

## Architectural Decisions

- Do not build a generic edit widget yet. Only `0x6A0` is in scope and verified active for this pass. The state shape is intentionally small enough to extract later if another active edit control is researched.
- Do not model literal Win32 `0x4B0`/`0x4AF` messages. They are implementation machinery for focus restoration; the Rust contract is repaint-stable edit focus and caret/text continuity.
- Do not use egui for the field. The player-visible frame, font, scissor, text inset, color, and caret must match the asset-backed shell renderer.
- Keep the 19-character cap in the edit helper, not only at Start, so visible behavior matches retail while typing.
- Keep all shell control IDs and edit details outside `sim/`.

## Alternatives Considered

### Generic Shell Edit Primitive

This would add a reusable shell edit-control abstraction immediately. It is parity-capable, but it adds abstraction before another active edit consumer is verified. Rejected for this pass; the `PlayerNameEditState` layout can be extracted later if needed.

### Egui Text Field Overlay

This would provide fast text input, but it would visibly drift on frame, font, caret, scissor, color, draw order, and focus behavior. Rejected.

### Literal Win32 Custom-Message Bus

This would copy internal message names like `0x4B0` and `0x4AF`, but it would be the wrong abstraction in Rust. The verified player-visible behavior is stable focus/text/caret continuity through redraws. Rejected.

### Render-Only Static Name Replacement

This would swap the literal `"Player"` for a configurable string but still lack focus, typing, selection, cap, caret, and Start readback. Rejected because it leaves the main player-visible parity hole intact.
