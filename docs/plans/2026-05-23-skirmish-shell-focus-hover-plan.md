# Skirmish Shell Focus And Hover Implementation Plan

> Execute task-by-task. This plan starts from the current working tree, where
> player-name edit state/rendering and `0x695` status rect/state/rendering are
> already partially implemented.

**Goal:** Complete the shared Skirmish shell focus/hover layer so native player-name
edit `0x6A0` and status/help child `0x695` behave from the player's point of view
like standard offline YR Skirmish.

**Design Doc:** [docs/plans/2026-05-23-skirmish-shell-focus-hover-design.md](2026-05-23-skirmish-shell-focus-hover-design.md)

---

## Grounding Summary

**Current Rust already has:**

- `PlayerNameEditState` with text, focus, selection, caret, scroll, 19-char cap, delete/backspace/arrow/home/end helpers.
- Click-to-focus/select-all and Start launch packing from `player_name_edit.text`.
- Edit frame/text/selection/caret rendering.
- `SkirmishShellLayout::status_help` with verified `0x695` rect tests.
- `SkirmishShellState::status_help_text` plus `set_status_help_text` / `clear_status_help_text`.
- Status text rendering in `app_skirmish_shell_render.rs`.

**Still missing or unpinned:**

- Shared hover/control identity resolver for status text.
- Verified `STT:*` key mapping for hovered controls, starting with player edit, buttons, preview, checkboxes, trackbars, and combo faces.
- AI row-state item-specific status override.
- App mouse-move wiring to update `status_help_text`.
- Tab/focus behavior for the player edit.
- Tests proving focus survives redraw/status/dropdown churn and typed text does not leak to global hotkeys.
- Explicit handling of the unverified Escape-to-Back behavior.

**Primary evidence:**

- `SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`
- `SKIRMISH_PLAYER_NAME_EDIT_FOCUS_MESSAGES_0X4B0_0X4AF_GHIDRA_REPORT.md`
- `SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_INPUT_FOCUS_MESSAGE_BROAD_RECHECK_GHIDRA_REPORT.md`
- `SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`
- `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`
- `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`
- Current files: `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs`

---

## Key Technical Decisions

- Reuse existing layout rect helpers, but prefer a dedicated hover/status target type over turning `ShellControlId` into a fake complete `0x102` child inventory. If `ShellControlId` is extended, keep it explicitly documented and tested as incomplete.
- Keep state/render/app split: hit testing and status keys in `ui/skirmish_shell`, localization and redraw routing in `app.rs` / render edge.
- Do not build a pseudo-Win32 message bus for `0x4B0`, `0x4AF`, `0x4E8`, or `0x4E9`. Implement the verified observable outcomes.
- Keep `player_name_edit.focused` as the current focus source unless implementation finds a clean reason to introduce `FocusedShellControl`; do not add two unsynchronized focus states.
- Treat Escape-close as unverified. Do not add tests or comments that claim it is native parity.

---

## File Map

| Path | Responsibility |
|---|---|
| `src/ui/skirmish_shell/layout.rs` | expose geometry helpers; keep `ShellControlId` subset status explicit if touched |
| `src/ui/skirmish_shell/state.rs` | hover resolver, status key mapping, edit Tab behavior, state tests |
| `src/ui/skirmish_shell/mod.rs` | re-export new helpers/types |
| `src/app.rs` | update status on mouse move; route Tab/focus behavior; avoid text leakage |
| `src/app_skirmish_shell_render.rs` | keep status/edit rendering; add focused render tests if needed |

---

## Tasks

### Task 1: Pin the current partial implementation with focused tests

**Why:** The source has advanced beyond the broad re-swarm report. Before adding behavior, lock down the pieces that already match the design.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/app_skirmish_shell_render.rs`

**Steps:**

1. Confirm existing tests cover:
   - `PLAYER_NAME_MAX_CHARS == 19`
   - first focus selects all and first typed text replaces default
   - backspace/delete selection behavior
   - caret scroll 5px margin
   - `0x695` rects at 640, 800, 1024
2. Add missing focused tests only if a listed behavior lacks coverage.
3. Do not change production logic in this task unless tests reveal a current regression.

**Acceptance:**

- `cargo test player_name --lib`
- `cargo test status_help --lib`

### Task 2: Add shell hover identity for status

**Why:** `0x695` depends on knowing which native child/control the cursor is over. This should be shared with focus/status, not duplicated in `app.rs`.

**Files:**

- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/mod.rs`

**Steps:**

1. Add a dedicated `ShellHoverTarget` / `SkirmishHoverTarget` unless implementation proves that extending `ShellControlId` will stay clearer.
   - It must represent dynamic row/item identity without implying a complete `0x102` child inventory.
   - If `ShellControlId` is extended instead, add a doc comment and test proving it is still only a focused subset, not the complete dialog matrix.
2. Cover at least:
   - `PlayerName0x6a0`
   - `StartGame0x617`, `ChooseMap0x5aa`, `Back0x5c0`
   - `MapPreview0x468`
   - `StatusHelp0x695`
   - checkbox ids
   - trackbar ids
   - combo faces by `SkirmishComboId`
   - AI row-state dropdown/current item identity if the resolver can know it cheaply
3. Add `hovered_shell_control(layout, shell, maps, x, y)` with native-style priority:
   - open dropdown/list content before parent face controls
   - modal parent controls blocked when `choose_map_modal` or validation modal owns input
   - status strip self-hover returns `StatusHelp0x695`
4. Keep geometry helpers pure and testable.

**Acceptance:**

- Unit tests prove Start, player edit, status strip, one checkbox, one trackbar, one combo face, and open dropdown row resolve to the expected identity.
- A guardrail test or doc comment prevents treating the hover/control enum as the full `0x102` child inventory.

### Task 3: Add status/help key resolver

**Why:** Native status text comes from `STT:*` keys and item-specific overrides, not visible GUI labels.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/mod.rs`

**Steps:**

1. Add `status_help_key_for_hover(...) -> Option<&'static str>` or an equivalent enum that can represent direct text overrides.
2. Map verified controls to keys from the status report:
   - player edit -> `STT:SkirmishEditPlayer`
   - Start -> `STT:SkirmishButtonStartGame`
   - Back -> `STT:SkirmishButtonBack`
   - checkboxes from `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`, e.g. `0x54E -> STT:SkirmishCBoxShortGame`, `0x69A -> STT:SkirmishCBoxSWAllowed`, `0x69D -> STT:SkirmishCBoxBuildOffAlly`, `0x693 -> STT:SkirmishCBoxRedeploys`, `0x696 -> STT:SkirmishCBoxCrates`
   - side/country controls -> `STT:SkirmishComboCountry`
   - color controls -> `STT:SkirmishComboColor`
   - AI row-state controls -> generic fallback `STT:SkirmishComboAIPlayer`, with item-specific `0x4E9` override below taking precedence
   - start controls `0x6A3..0x6AB` -> `STT:HostComboStart`; do not invent `STT:SkirmishComboStart`
   - team controls `0x76D..0x774` -> `STT:HostComboTeam`
   - Choose Map, preview, and trackbars using the verified `FUN_006040B0` table from `SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md` / broad recheck
3. Add the AI row-state item-specific override:
   - item data `-1` -> `STT:PlayerNone`
   - `2` -> `STT:PlayerDumbAI`
   - `1` -> `STT:PlayerSmartAI`
   - `0` -> `STT:PlayerGeniusAI`
4. Return `None` for `StatusHelp0x695` and unknown controls.
5. Do not use visible `GUI:*` label keys as status text.

**Acceptance:**

- Tests:
  - `status_help_start_uses_stt_skirmish_button_start_game`
  - `status_help_player_name_uses_stt_skirmish_edit_player`
  - `status_help_self_hover_is_blank`
  - `status_help_ai_row_state_uses_item_specific_stt`
  - `status_help_checkbox_uses_verified_stt_key`
  - `status_help_trackbar_uses_verified_stt_key`
  - `status_help_preview_uses_verified_stt_key`
  - `status_help_side_combo_uses_stt_skirmish_combo_country`
  - `status_help_start_combo_uses_stt_host_combo_start`
  - `status_help_team_combo_uses_stt_host_combo_team`

### Task 4: Wire hover updates in app input

**Why:** Native updates `0x695` from cursor hit testing. Rust currently has state/rendering, but needs app input to refresh it.

**Files:**

- `src/app.rs`
- `src/ui/skirmish_shell/state.rs`

**Steps:**

1. Add an app helper that localizes a returned `STT:*` key through the current CSF table and falls back to blank if missing.
2. In `handle_skirmish_shell_mouse_move`, compute hover/status before or alongside existing drag handling.
3. Request redraw when `status_help_text` changes.
4. Clear any existing parent `status_help_text` when opening `choose_map_modal` or a validation modal, so stale hover text does not remain visible under modal ownership.
5. While `choose_map_modal` or validation modal is open, do not update parent `0x102` status text from parent controls. Keep status blank or delegate only to modal-specific status once verified.
6. Clear status when the cursor leaves all mapped controls or hovers `0x695`.

**Acceptance:**

- Moving over Start sets localized `STT:SkirmishButtonStartGame` text.
- Moving over `0x695` clears it.
- Opening Choose Map and moving over the parent Start rect does not set parent Start status.
- Hovering Start, then opening Choose Map, leaves the status strip blank instead of preserving stale Start text.

### Task 5: Harden player-name focus and Tab behavior

**Why:** Current edit input is partially implemented, but Tab/focus stability and unverified global key behavior need explicit handling.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/app.rs`

**Steps:**

1. Add a state helper for verified edit Tab behavior:
   - pressing Tab while edit is focused must not insert `\t`
   - this pass should consume Tab and leave edit focus, but must not claim the exact next native focus target unless a focused tab-order trace pins it
2. Ensure focused edit consumes printable text, Backspace, Delete, Left, Right, Home, End, and Tab before global hotkeys.
3. Keep first-click focus selecting all text.
4. Keep focus stable across status updates and dropdown changes unless the user clicks a different parent control.
5. Do not claim Escape behavior as parity. Either leave current behavior as an explicitly unverified app policy or gate it behind a future trace.

**Acceptance:**

- `player_name_tab_does_not_insert_control_text_and_leaves_edit_focus`
- If exact tab destination is still unverified, tests assert only the scoped observable behavior: no inserted control text and no remaining edit focus.
- `player_name_focus_survives_status_hover_update`
- `player_name_focus_survives_dropdown_open_close_until_explicit_blur`
- Existing player-name text/cap/backspace tests remain green.

### Task 6: Render and text-path verification for `0x695`

**Why:** The renderer currently draws `status_help_text`, but the plan needs a regression gate for placement/scissor/text color.

**Files:**

- `src/app_skirmish_shell_render.rs`
- `src/ui/skirmish_shell/layout.rs`

**Steps:**

1. Keep status text draw tied to `layout.status_help`, not right-panel statics.
2. Use shell yellow text and existing shell text scissor.
3. Add or update tests so a non-empty status string produces a draw scoped to the `0x695` rect.
4. Ensure empty status text emits no visible text.
5. When `choose_map_modal` is open, parent `0x102` status text must not be drawn under or inside the modal. The modal can gain its own verified status path later, but this pass should not leak parent hover text.

**Acceptance:**

- Render tests verify status draw rect/scissor and blank-state no-text behavior.
- Render tests verify `choose_map_modal` open suppresses parent status text even if stale parent `status_help_text` is present.

### Task 7: Focused verification pass

**Why:** This is UI/input behavior; compile-only is not enough.

**Commands:**

- `cargo test player_name --lib`
- `cargo test status_help --lib`
- `cargo test skirmish_shell --lib`
- `cargo fmt`

**Manual/visual checks:**

1. Open the Rust native Skirmish shell at 800x600, comparing against original YR only where the checklist calls for parity confirmation.
2. No-hover status strip is blank.
3. Hover Start: bottom-left strip shows localized Start help.
4. Hover player name: strip shows localized player edit help.
5. Hover status strip itself: strip clears.
6. Click player name, type 20 printable chars: text caps to 19.
7. Click player name and type one char immediately: default `Player` is replaced, not appended.
8. Start launch session carries the edited name.

**Acceptance:**

- Focused tests pass.
- Manual checklist matches the expected visible behavior.
- Any remaining Escape/Enter behavior is documented as unverified, not as parity.

---

## Negative Facts / Do Not Do

- Do not route this through `sim/`.
- Do not use visible `GUI:*` labels as status/help text.
- Do not show permanent text in `0x695`.
- Do not let `0x695` self-hover produce a tooltip.
- Do not add sounds for hover/status changes.
- Do not claim Escape-to-Back or Enter-to-Start parity from this plan.
- Do not emulate the Win32 custom-message system; model observable state.

---

## Acceptance Gate

This plan is complete when:

1. Player-name edit focus/input/select-all/caret behavior remains covered and functional.
2. `0x695` status/help text updates from verified hover control identity and `STT:*` keys.
3. Status strip stays blank for no-hover and self-hover.
4. Parent status is cleared on modal entry and stays blocked while modal state owns input.
5. Focused edit consumes text input and does not leak it to global hotkeys.
6. Focused tests and the manual visual checklist pass.
