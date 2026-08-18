# Skirmish Shell Focus And Hover Design

## Goal

Complete the standard offline Skirmish shell focus/hover layer so player-name edit `0x6A0` and status/help strip `0x695` reproduce native YR-visible behavior.

## Architecture Context

The Skirmish shell is already split across UI state, layout, app input routing, and renderer surfaces:

- `src/ui/skirmish_shell/state.rs` owns shell state and pure UI state transitions. It already has `PlayerNameEditState`, edit text/caret/selection helpers, `status_help_text`, pending UI sounds, dropdown state, and launch packing from `player_name_edit.text`.
- `src/ui/skirmish_shell/layout.rs` computes stable native rects. It already exposes `layout.player_name` and `layout.status_help`, including verified `0x695` bottom-left rect tests.
- `src/app.rs` routes winit input into the shell. It already routes focused edit key input, mouse focus/blur, modal blocking, and redraw requests.
- `src/app_skirmish_shell_render.rs` renders shell chrome, text, edit frame, selection, caret, and player-name text. It does not yet render `status_help_text`.
- `sim/` remains out of scope. This is shell UI behavior before match launch.

Recent research docs describe current Rust as missing player-name/status behavior, but that is partly stale against the current working tree. The remaining gap is not from-scratch edit state; it is a coherent focus/hover resolver that hardens the partial implementation and makes status/help text use the same control identity map.

## Impact Analysis

Touched modules:

- `src/ui/skirmish_shell/state.rs`: add focused/hovered shell control identity and status resolver helpers; harden edit Tab/focus behavior; keep current edit state.
- `src/ui/skirmish_shell/layout.rs`: keep existing `player_name` and `status_help` rects; optionally expose helper hit tests for status/control identity.
- `src/app.rs`: route cursor moves through hover resolver, route key input through focused control, and stop claiming Escape parity unless separately traced.
- `src/app_skirmish_shell_render.rs`: render `status_help_text` in `layout.status_help`; keep edit frame/text/caret/selection rendering.
- Tests in existing module test blocks for state/layout/render behavior.

Main risks:

- Accidentally treating visible control labels as status text instead of `STT:*` CSF keys.
- Breaking existing combo/dropdown/trackbar mouse capture while adding hover state.
- Letting global hotkeys consume text input while the edit owns focus.
- Treating current Escape-to-close behavior as verified parity. It is not verified by the scoped binary reports.

## Chosen Approach

Use a shared shell focus/hover resolver.

Add explicit shell control identity in `ui/skirmish_shell/state.rs`, for example:

- `FocusedShellControl::PlayerNameEdit0x6a0`
- `HoveredShellControl` variants for player edit, buttons, checkboxes, trackbars, combo faces/dropdown rows, preview, right-panel statics, and status strip.

The app layer asks the state/layout layer what control is under the cursor. The state layer derives status text from that identity using native `STT:*` keys and known item-specific overrides. The renderer only draws the resulting `status_help_text`.

This keeps the architecture direct and Rust-native while preserving the observable Win32 behavior. It does not emulate custom messages like `0x4B0`, `0x4AF`, `0x4E8`, or `0x4E9`; those are represented by their visible outcomes: stable edit focus, select-all-on-focus, status text update on hover, blank fallback, and Start readback.

## Tiny-Detail Ledger

- `0x6A0` is a real editable child in standard offline Skirmish `0x102`, not a static label. Source: `SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`, setup/readback `0x006AE6F2..0x006AE735`, `0x006AD375..0x006AD39F`.
- `0x6A0` uses ordinary `Edit -> OwnerDraw_Edit_00614190`, not `NewEdit`. Source: `SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`, `0x0060FCF0..0x0060FD80`.
- Player-name final rect is `(58,59,151,23)` at 800x600; client/text insets are edit-specific. Source: same report; current `layout.rs`.
- Native setup sends `EM_SETLIMITTEXT 0x13`; Start reads `0x14` code units and terminates at index 19. Observable cap is 19 visible chars. Source: same report.
- First focus selects all text before deferred focus guard; first printable char replaces default text. Source: `SKIRMISH_PLAYER_NAME_EDIT_FOCUS_MESSAGES_0X4B0_0X4AF_GHIDRA_REPORT.md`; input broad recheck.
- Edit focus must remain stable across redraws/status updates/dropdown changes; Rust should model the state, not literal `0x4B0/0x4AF` messages. Source: focus report and `SKIRMISH_SHELL_INPUT_FOCUS_MESSAGE_BROAD_RECHECK_GHIDRA_REPORT.md`.
- Edit text is yellow, uses edit-specific inset, horizontal scroll keeps caret visible with 5px margin, and caret is 2px wide when focused with no selection. Source: `FUN_00623880` evidence in player-name edit report.
- Tab is verified inside the old Edit path. Global Enter/Escape behavior is not verified in the scoped Skirmish dialog path. Source: input broad recheck.
- `0x695` is a visible bottom-left status/help static, blank by default. Source: `SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md`.
- `0x695` rects are `640=(10,459,615,20)`, `800=(10,579,615,20)`, `1024=(122,663,615,20)`. Source: status report and current layout tests.
- Hover status update is driven by cursor hit testing: child `0x4E8`, then parent `0x4E9`, then `FUN_006040B0` `STT:*` fallback, then empty string fallback. Source: status report, `0x00622CCB..0x00622E83`.
- `0x695` has no self-tooltip mapping; hovering the strip itself leaves it blank. Source: status report, `FUN_006040B0` no `0x695` case.
- Status text must use localized `STT:*` keys or item-specific text, not visible GUI labels. Source: status report and broad input recheck.

## Design

### Components

**Shell control identity**

Add a compact enum in `state.rs` for hit-test identity. It should model native child/control ids at the level needed by status/focus behavior:

- `PlayerNameEdit0x6a0`
- `OwnerDrawButton(OwnerDrawButton)`
- `Checkbox(SkirmishCheckboxId)`
- `Trackbar(SkirmishTrackbarId)`
- `ComboFace(SkirmishComboId)`
- `ComboDropdownItem { id, row/item }`
- `MapPreview0x468`
- `RightPanelTitle0x694`, `RightPanelGameType0x6ec`, `RightPanelMap0x5a8`
- `StatusHelp0x695`
- `None`

This enum is UI-only and must not cross into `sim/`.

**Focus owner**

Keep focused input ownership simple:

- `FocusedShellControl::PlayerNameEdit0x6a0`
- later variants only when verified and needed.

Current `player_name_edit.focused` can either remain the direct field or be derived from the focus owner. Prefer one source of truth in implementation; if both exist, add helper methods so they cannot disagree.

**Hover/status resolver**

Add a resolver that takes `layout`, current shell state, maps, cursor position, and CSF lookup callback or key-returning helper. It should:

1. Determine hovered shell control.
2. Return empty text for no hover or `StatusHelp0x695`.
3. Use item-specific text where verified, especially AI row state combo status.
4. Otherwise map control identity to verified `STT:*` keys.
5. Leave unsupported/unknown controls blank until a report verifies their key.

### Interfaces / Contracts

State-level helpers:

- `hovered_shell_control(layout, shell, maps, x, y) -> HoveredShellControl`
- `status_help_key_for_hover(hover, shell, maps) -> Option<&'static str>`
- `status_help_text_for_hover(hover, state, csf_lookup) -> String` or app-level localization around a key
- `update_status_help_from_cursor(shell, layout, maps, x, y, localize) -> bool`

App-level routing:

- On shell mouse move, update hover/status first, then continue existing drag/dropdown/trackbar handling.
- While a modal is open, parent hover/status should not update from parent controls unless native modal-specific behavior is implemented.
- On text input/key down, if `PlayerNameEdit0x6a0` owns focus, route printable/backspace/delete/arrow/home/end/tab to edit handling and do not forward typed chars to global hotkeys.

Renderer contract:

- Draw edit frame/text/selection/caret from `player_name_edit`.
- Draw `status_help_text` into `layout.status_help` only when non-empty.
- Use shell yellow text and existing shell text scissor path.

### Data Flow

Mouse move:

`winit CursorMoved -> app.rs -> compute_layout -> update_status_help_from_cursor -> handle_option_mouse_move for active drags -> request redraw if status changed`

Mouse down:

`winit MouseInput -> app.rs -> if player_name rect hit, focus/select-all; if other parent control hit, blur edit where native focus leaves edit; modal state blocks parent`

Key input:

`winit KeyboardInput -> app.rs -> focused control dispatch -> PlayerNameEditState mutation -> sync scroll -> redraw`

Render:

`render_skirmish_shell_with_atlas -> build shell instances/edit instances -> build_shell_text_draws -> player-name text -> status_help_text draw`

Start:

`launch_session` already uses `player_name_edit.text`; keep this and add regression coverage.

### Error Handling

- Missing CSF key should fall back to blank or the key string only where existing localization helpers already do that for shell labels. For status/help parity, prefer blank over visible wrong label when the mapping is unverified.
- Empty player-name input should remain allowed unless binary evidence later proves rejection.
- Non-printable/control characters are ignored for edit text.
- Unicode handling should remain char-count based for the 19-character cap; YR uses wide text, so byte-length caps would be the wrong internal model.

### Testing Strategy

State/layout tests:

- `player_name_focus_selects_all_and_first_char_replaces_default`
- `player_name_insert_caps_at_nineteen_chars`
- `player_name_focus_survives_status_and_dropdown_updates`
- `player_name_tab_blurs_or_moves_focus_without_inserting_tab`
- `status_help_strip_0x695_bottom_left_rects` already exists; keep it.
- `status_help_defaults_blank_and_self_hover_blank`
- `status_help_hover_start_uses_stt_skirmish_button_start_game`
- `status_help_hover_player_name_uses_stt_skirmish_edit_player`

App/input tests where feasible:

- focused edit consumes printable text and prevents global hotkey handling.
- Escape behavior is either gated behind future research or marked as non-parity behavior; do not assert retail parity for it yet.

Render tests:

- player-name text scissor and caret x already have tests; add status text draw scissor/rect coverage.
- focused selection draws before text and caret only appears when no selection.

Visual/manual check:

- 800x600 shell: no-hover status strip is blank.
- hover Start: status text appears in bottom-left strip.
- focus player name, type 20 chars: visible text caps to 19 and Start session carries that value.

## Architectural Decisions

- Do not build a generic Win32 custom-message bus. The verified observable behavior is smaller: focus stability, text copy/readback, hover status text, blank fallback, and localized `STT:*` mapping.
- Keep control identity in `ui/skirmish_shell`, not in `app.rs`, so hit testing and status logic remain testable without the window/event loop.
- Keep localization at the app/render edge where CSF access already exists. State can return keys or text through a callback rather than depending on app state directly.
- Preserve current partial edit implementation instead of replacing it. The code already matches several ledger items; implementation should harden and connect it.
- Do not route any of this through `sim/`; shell UI has no deterministic gameplay state role before launch.

## Alternatives Considered

### Pseudo-Win32 Message Model

Represent `0x4B2`, `0x4B3`, `0x4E8`, `0x4E9`, `0x4B0`, and `0x4AF` as internal messages.

Rejected because it adds a new abstraction that the rest of the Rust shell does not use. It would obscure simple testable outcomes and invite over-modeling unverified message-manager details.

### Separate Ad Hoc Fixes

Patch player-name behavior and status strip independently.

Rejected because both need the same control identity and input ownership. Separate fixes would duplicate hit testing and likely drift when dropdowns, modal blocking, and status text need to agree about which control is active or hovered.

### Status Only First

Implement only `0x695` because player-name edit is now partially implemented.

Rejected as incomplete for the approved scope. Player-name edit still needs focus/keyboard/status integration and is the natural owner of the focus side of the shared resolver.
