# Skirmish Shell UI Parity Implementation Plan

> Execute task-by-task. Do not make the experimental shell the default in this plan.

**Goal:** Render the verified offline Skirmish shell controls and make Start Game validate/pack the same state those controls show.

**Design Doc:** [docs/plans/2026-05-22-skirmish-shell-ui-parity-design.md](2026-05-22-skirmish-shell-ui-parity-design.md)

---

## Grounding Summary

Primary synthesis:

- `docs/research/SKIRMISH_SHELL_UI_SYSTEM_MODEL_SYNTHESIS.md`

Underlying evidence:

- `skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_PLAYER_AI_ROW_VISIBILITY_ENABLE_RULES_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`

Current Rust surfaces:

- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/render/skirmish_shell_chrome.rs`
- `src/app_skirmish_shell_render.rs`
- `src/skirmish_launch.rs`
- `src/app.rs`

## Key Decisions

- Keep the whole path behind `dev_skirmish_shell_enabled`.
- Integrate visible controls and Start validation in one pass so the shell does not display state that Start ignores.
- Keep retail shell item data in `ui/skirmish_shell`; convert to semantic launch data before app-level setup.
- Do not implement preview start markers, full dropdown listbox rows, exact >800 pixels, or post-launch mode callbacks in this plan.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/ui/skirmish_shell/state.rs` | AI row type item data, active-row counting, Start validation, launch packing. |
| Modify | `src/ui/skirmish_shell/layout.rs` | Missing row/control rects and helper geometry. |
| Modify | `src/ui/skirmish_shell/mod.rs` | Re-export new public helpers/types. |
| Modify | `src/skirmish_launch.rs` | Validation errors and semantic launch contracts. |
| Modify | `src/render/skirmish_shell_chrome.rs` | Add checkbox, trackbar, and combo-arrow atlas entries. |
| Modify | `src/app_skirmish_shell_render.rs` | Render missing controls and corrected text. |
| Modify | `src/app.rs` | Mouse move/up for trackbar drag; validation failure stays in shell. |

## Parity-Critical Items

| Item | Verification |
|---|---|
| Checkbox icon-only toggle; label inert | Unit test with icon rect and label rect. |
| Checkbox art `cue_i.pcx` / `cce_i.pcx` | Atlas helper test or render helper test. |
| Trackbar 65 px active width, 12 px thumb interval, top 4 px rejected | Existing tests plus any moved helper tests. |
| Game Speed visual `6 - stored` | Existing test retained. |
| Button label inset/pressed rect | New helper test. |
| Collapsed combo 24 px face, 20 px arrow reserve, 2 px swatch inset | New layout/render helper test. |
| AI row item data `-1,2,1,0`; active rows only `0/1/2` | New state tests. |
| Start rejects no map, no opponent, capacity overflow, invalid start/color, same explicit team | New `launch_session` tests. |
| Start failure keeps shell open | App handler test if practical, otherwise state/action test plus manual verification. |

---

## Tasks

### Task 1: Make AI row type item data explicit

**Why:** Start validation and display text both depend on retail row type item data.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/mod.rs`

**Steps:**

1. Add a shell-facing row type enum for `None`, `Easy`, `Normal`, `Hard`.
2. Add methods:
   - `item_data() -> i32` returning `-1`, `2`, `1`, `0`;
   - `is_active() -> bool`;
   - conversion to semantic `AiDifficulty`.
3. Update `SkirmishShellOpponent` so active/inactive status is derived from row type.
4. Preserve current default behavior: one active Easy opponent by default.
5. Add tests for item data order, active counting, and default active row count.

**Checks:**

- `cargo test skirmish_shell --lib`

### Task 2: Extend Start validation and launch errors

**Why:** Retail Start does not exit the dialog until validation and packing succeed.

**Files:**

- `src/skirmish_launch.rs`
- `src/ui/skirmish_shell/state.rs`

**Steps:**

1. Extend `LaunchValidationError` for:
   - map capacity overflow;
   - same explicit team rejection.
2. Update `launch_session` to count active rows from row type item data.
3. Validate selected map capacity using `MapMenuEntry.multiplayer_start_waypoints.len()`.
4. Keep existing no-map, no-opponent, invalid-color, and invalid-start checks.
5. Add same-explicit-team validation when local team is explicit and all active AI teams match it.
6. Pack only active AI rows into `SkirmishLaunchSession`.

**Checks:**

- `cargo test launch_session --lib`
- `cargo test skirmish_launch --lib`

### Task 3: Add row/combo/static layout helpers

**Why:** Rendering should consume verified geometry from layout, not duplicate coordinates.

**Files:**

- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/mod.rs`

**Steps:**

1. Add rect structs for row controls where geometry is verified: AI type, country, color, start, team, and static labels.
2. Add helper functions for collapsed combo face rect, combo text rect, arrow rect, and color swatch rect.
3. Keep existing 640/800/1024 tests passing.
4. Add tests for 24 px combo face, 20 px arrow reserve, and `(2,2,20,20)` color swatch inset.

**Checks:**

- `cargo test skirmish_shell::layout --lib`

### Task 4: Load missing owner-draw assets into the atlas

**Why:** The verified shell path uses retail PCX assets for these controls.

**Files:**

- `src/render/skirmish_shell_chrome.rs`

**Steps:**

1. Add optional atlas fields for `cue_i.pcx`, `cce_i.pcx`, `trakgrip.pcx`, `trofl.pcx`, `trofm.pcx`, `trofr.pcx`, and `dnarrowr.pcx`.
2. Add optional pressed/grey down-arrow entries only if the existing asset loader can resolve them without inventing substitutes.
3. Keep mandatory/optional behavior consistent with current shell atlas style.
4. Add classification tests for the new verified owner-draw assets.

**Checks:**

- `cargo test skirmish_shell_chrome --lib`

### Task 5: Correct button text caller rects

**Why:** Current button text uses the full button rect plus a simple y offset, which differs from the verified caller contract.

**Files:**

- `src/app_skirmish_shell_render.rs`

**Steps:**

1. Add a small helper that returns the released/pressed button text rect.
2. Released rect: `left = button_left`, `top = button_top + 1`, `right = button_left + width - 2`, `bottom = button_top + height`.
3. Pressed rect shifts only `left += 2` and `top += 5`.
4. Use horizontal center plus vertical center flags.
5. Add helper tests.

**Checks:**

- `cargo test app_skirmish_shell_render --lib`

### Task 6: Render checkboxes and trackbars

**Why:** These are always-visible controls in a normal Skirmish setup and already have layout/input state.

**Files:**

- `src/app_skirmish_shell_render.rs`

**Steps:**

1. Add checkbox render helper using `cue_i.pcx` or `cce_i.pcx` plus text draws from `checkbox_text_rect`.
2. Add trackbar render helper for primitive rail placeholder, plaque pieces, numeric value text, and `trakgrip.pcx` thumb.
3. Use `trackbar_visual_value`, `trackbar_pixel_offset`, `trackbar_thumb_rect`, and `trackbar_value_text_rect`.
4. Do not use plus/minus SHP controls.
5. Add tests for asset selection and thumb/plaque placement helpers.

**Checks:**

- `cargo test app_skirmish_shell_render --lib`
- `cargo test skirmish_shell --lib`

### Task 7: Render collapsed combo faces and row/static text

**Why:** Rows need to show the same state that Start validates.

**Files:**

- `src/app_skirmish_shell_render.rs`
- `src/ui/skirmish_shell/state.rs`

**Steps:**

1. Render collapsed combo face frames for visible row controls.
2. Render down-arrow PCX in the reserved 20 px zone.
3. Render color swatches for color combos using the verified inset.
4. Render AI row type labels from item data: `GUI:None`, `GUI:AIEasy`, `GUI:AINormal`, `GUI:AIHard`.
5. Render local/player and static labels from CSF where available, with fallbacks.
6. Keep full dropdown/listbox row paint out of scope.

**Checks:**

- `cargo test app_skirmish_shell_render --lib`

### Task 8: Wire shell input and Start failure behavior in app

**Why:** Existing state helpers for trackbar drag are not fully routed through app events.

**Files:**

- `src/app.rs`

**Steps:**

1. Route cursor movement to `handle_option_mouse_move` while the dev shell is active.
2. Route mouse up to `handle_option_mouse_up` after button release handling.
3. On `launch_session` error, keep the shell open and preserve state.
4. Log or surface the typed validation error without starting map loading.
5. Do not make the dev shell default.

**Checks:**

- `cargo test skirmish_shell --lib`
- Manual smoke: toggle shell, drag sliders, press Start with invalid rows, verify shell remains open.

### Task 9: Focused verification pass

**Why:** This is a player-visible UI path and needs a check beyond compilation.

**Steps:**

1. Run:

```powershell
cargo test skirmish_shell --lib
cargo test skirmish_launch --lib
cargo test app_skirmish_shell_render --lib
```

2. If the app can run locally, open the experimental shell at 800x600 and verify:
   - checkboxes render and toggle only on icons;
   - trackbars draw and drag;
   - collapsed row combos show labels/swatches/arrows;
   - Start stays in shell on validation failure;
   - Start succeeds for one valid local player plus one active AI row.

## Out of Scope

- Default-enabling the experimental shell.
- Full dropdown/listbox row painting.
- `STARTBUT.SHP` preview marker projection.
- Exact >800 retail pixel matching.
- MPModes post-launch callbacks and start-unit budget parity.
