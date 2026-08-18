# Skirmish Checkbox / Trackbar Owner-Draw Implementation Plan

> Execute this plan task-by-task. This is a planning document only; do not implement Rust from this document until it is explicitly approved.

**Goal:** Implement standard offline YR Skirmish `0x102` checkbox and trackbar owner-draw visuals, input, and launch-option packing from the approved design in `docs/plans/2026-05-22-skirmish-checkbox-trackbar-ownerdraw-design.md`.

**Design Input:** User-approved Approach 1: first-class retail widgets in the existing skirmish shell layout/state/render path. Primary research source is `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`.

---

## Grounding Summary

- Standard offline YR Skirmish `0x102` actively uses owner-draw callbacks for five option checkboxes and three trackbars.
- Rust already has the shell layout/render/state split needed for this feature.
- Trackbar rects already exist in `SkirmishShellLayout`; checkbox rects are missing.
- `SkirmishLaunchOptions` and `GameOptions` already contain the gameplay option fields.
- This plan does not implement disabled checkbox overlays or variant checkbox art because standard offline Skirmish does not normally initialize those paths.
- This plan does not complete the full skirmish slot table or combo chrome; it only fills the checkbox/trackbar parity hole.

## Key Technical Decisions

- **Keep geometry in `layout.rs`.** Checkbox rects, icon rects, label rects, plaque rects, active track width, and thumb rect helpers should be pure integer functions.
- **Keep input math in `state.rs`.** Trackbar clamp/quantization/drag behavior should be unit-testable without GPU/app state.
- **Use existing atlas pattern.** Add verified PCX entries to `SkirmishShellChromeAtlas`; do not create a second texture or special render path.
- **Render from layout + state only.** The renderer should not duplicate trackbar value math except for calling pure helpers.
- **Pack options on Start.** Shell clicks update shell state; `launch_session()` packs the accepted values into `SkirmishLaunchOptions`.
- **Do not introduce generic widgets yet.** The callback-specific retail details are the point of this feature.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/ui/skirmish_shell/layout.rs` | Add checkbox IDs/rects and widget geometry helpers |
| Modify | `src/ui/skirmish_shell/state.rs` | Add checkbox/trackbar state, input handling, launch packing, pure math tests |
| Modify | `src/ui/skirmish_shell/mod.rs` | Re-export any new public layout/state types needed by app/render |
| Modify | `src/render/skirmish_shell_chrome.rs` | Load/atlas verified checkbox and trackbar PCX assets |
| Modify | `src/app_skirmish_shell_render.rs` | Draw checkbox icons/labels and trackbar rail/plaque/thumb/value text |
| Modify | `src/app.rs` | Route skirmish mouse move/down/up through new checkbox/trackbar state handlers |
| Maybe modify | `src/skirmish_launch.rs` | Only if existing option fields need small helper coverage; no schema expansion expected |
| Maybe modify | `src/ui/main_menu.rs` | Only if temporary `SkirmishSettings` needs option bridge preservation before `launch_session()` is used directly |

## Task Plan

### Task 1 - Layout: represent checkbox controls

**Why:** Rendering and input need shared screen-space checkbox rects.

Steps:

1. Add `SkirmishCheckboxId` with:
   - `ShortGame0x54e`
   - `McvRepacks0x693`
   - `CratesAppear0x696`
   - `SuperWeapons0x69a`
   - `BuildOffAlly0x69d`
2. Add `SkirmishCheckboxRect { id, rect }`.
3. Add `checkboxes: [SkirmishCheckboxRect; 5]` to `SkirmishShellLayout`.
4. Compute DLU rects:
   - `0x54E`: `(48,176,100,10)`
   - `0x693`: `(48,193,100,10)`
   - `0x696`: `(48,210,100,10)`
   - `0x69A`: `(48,228,103,10)`
   - `0x69D`: `(201,227,166,11)`
5. Apply verified `x -= 1` fixup to the first four checkbox controls.
6. Add helpers:
   - `checkbox_icon_rect(rect)`
   - `checkbox_text_rect(rect)`
7. Add tests:
   - 800 geometry for all five checkbox rects.
   - 640 geometry for all five checkbox rects.
   - icon rect is `18x18` at top-left.
   - text rect left is `rect.x + 26`.

Acceptance:

- `cargo test --lib skirmish_shell::layout` equivalent focused run passes, or `cargo test --lib skirmish_shell` if filtering by module is awkward.

### Task 2 - Layout/state: pure trackbar geometry and value math

**Why:** Trackbar feel depends on exact clamp, quantization, and thumb math.

Steps:

1. Add `SkirmishTrackbarId` with:
   - `GameSpeed0x529`
   - `Credits0x511`
   - `UnitCount0x50c`
2. Add constants:
   - plaque width `50`
   - active-width subtract `13`
   - thumb width `12`
   - mouse x bias `6`
   - minimum clamp x `1`
3. Add pure helpers:
   - `trackbar_plaque_rect(rect)`
   - `trackbar_value_text_rect(rect)`
   - `trackbar_active_width(rect)`
   - `trackbar_pixel_offset(value, min, max, step, rect)`
   - `trackbar_thumb_rect(rect, pixel_offset)`
   - `trackbar_mouse_allowed_y(rect, mouse_y)`
   - `trackbar_mouse_value(rect, mouse_x, min, max, step)`
   - `trackbar_thumb_hit(rect, pixel_offset, mouse_x, mouse_y)`
4. Keep all math integer/truncating.
5. Add tests:
   - active width is `65` for `128x21`.
   - top 4 px are rejected for `21` px controls.
   - thumb interval is `[thumb_x, thumb_x + 12)`.
   - mouse x clamps below/above range.
   - credits values snap by `100`.
   - unit count snaps by `1`.

Acceptance:

- Focused state/layout tests pass.

### Task 3 - Shell state: add option values and input gestures

**Why:** The controls must be real stateful shell controls, not just rendered art.

Steps:

1. Extend `SkirmishShellState` with:
   - `game_speed: i32`
   - `unit_count: i32`
   - `super_weapons: bool`
   - `build_off_ally: bool`
   - `crates: bool`
   - `mcv_redeploy: bool`
2. Keep existing `starting_credits` and `short_game`.
3. Initialize defaults from `SkirmishLaunchOptions::default()` or `GameOptions::default()` so Rust follows the same source for every option.
4. Add `TrackbarDragState { id, dragging_thumb: bool }` or equivalent to `SkirmishShellState`.
5. Add input functions:
   - checkbox mouse down/up or click toggle using icon-only gate.
   - trackbar mouse down: if y gate passes, start drag if inside thumb, otherwise remap value.
   - trackbar mouse move: update active drag.
   - trackbar mouse up: clear drag.
6. Decide whether option toggles return `SkirmishShellAction::None` after local state mutation or introduce option-specific action variants. Prefer local mutation and `None` unless app-level sound handling requires a returned event.
7. Preserve owner-draw button press state behavior.

Tests:

- Clicking checkbox icon toggles state.
- Clicking checkbox label does not toggle.
- Trackbar top edge does not change value.
- Trackbar outside-thumb click remaps.
- Thumb hit starts drag.
- Mouse move updates while dragging and stops after mouse up.

Acceptance:

- Focused `state.rs` tests pass.

### Task 4 - Launch packing: carry all accepted options

**Why:** Retail applies these settings when Start accepts the shell.

Steps:

1. In `launch_session()`, pack:
   - `starting_credits`
   - `unit_count`
   - `game_speed`
   - `short_game`
   - `super_weapons`
   - `build_off_ally`
   - `crates`
   - `mcv_redeploy`
2. Preserve existing launch slot/country/color behavior.
3. Handle game speed inversion at the boundary:
   - if shell stores the visual trackbar position, pack `6 - visual`.
   - if shell stores the final game option value, render/input helpers convert to visual position.
4. Prefer storing final game option values in state and using helper conversion for the visual trackbar. This keeps `SkirmishLaunchOptions` packing simple.

Tests:

- Launch session packs all option fields.
- Game speed visual position conversion is covered.
- Build Off Ally default remains true.

Acceptance:

- Existing `launch_session_*` tests continue to pass with expanded assertions.

### Task 5 - Atlas: load verified PCX assets

**Why:** Render path needs the exact owner-draw art.

Steps:

1. Add `SkirmishShellChromeAtlas` fields:
   - `checkbox_unchecked_cue_i`
   - `checkbox_checked_cce_i`
   - `trackbar_grip_trakgrip`
   - `trackbar_plaque_left_trofl`
   - `trackbar_plaque_middle_trofm`
   - `trackbar_plaque_right_trofr`
2. Add these PCX files to `build_skirmish_shell_chrome_atlas()`.
3. Verify transparent handling before finalizing:
   - inspect current PCX decoder behavior and, if needed, add a focused ignored retail asset dimension/palette test.
   - use the same transparent-index/color-key behavior as the asset requires.
4. Extend `classify_shell_asset()` tests:
   - new assets are verified owner-draw controls.
   - `bst_*`, `BTN-MINS.SHP`, and `BTN-PLUS.SHP` stay rejected or research/other.
5. Add ignored retail-dimension tests if dimensions are not already documented in code.

Acceptance:

- Atlas tests pass without GPU retail dependencies except ignored tests.

### Task 6 - Render: checkbox visuals and labels

**Why:** The lower option controls must become visible and match retail layout.

Steps:

1. Add draw roles for checkbox icon and checkbox label.
2. In `build_skirmish_shell_instances()`, push checked/unchecked PCX icon at `checkbox_icon_rect`.
3. In text drawing, add labels:
   - `GUI:ShortGame`
   - `GUI:MCVRepacks`
   - `GUI:CratesAppear`
   - `GUI:SuperWeaponsAllowed`
   - `GUI:BuildOffAlly`
4. Use `checkbox_text_rect` and vertical-centered left alignment.
5. Keep labels out of toggle hit area.

Tests:

- A pure helper test can count expected semantic draw roles if draw roles are expanded.
- Text-origin helper should cover vertical center / left alignment if needed.

Acceptance:

- `cargo test --lib skirmish_shell` passes.

### Task 7 - Render: trackbar visuals and numeric text

**Why:** Sliders need the exact retail pieces and value feedback.

Steps:

1. Add draw roles for:
   - trackbar rail/bevel
   - trackbar plaque left/middle/right
   - trackbar grip
   - trackbar value text
2. Render plaque:
   - middle at right-side plaque region, tiled/clipped as needed.
   - left and right caps at verified positions.
3. Render primitive rail using the existing bevel helper or direct sprite entry if the helper is only atlas-time.
4. Render grip at `rect.x + 1 + pixel_offset`, `rect.y`.
5. Draw numeric text in `[right-49, top, right, bottom]` equivalent, centered as verified by flags `0x05`.
6. Render game speed numeric value according to retail visual position or displayed absolute value after confirming from report. If not explicit, follow callback behavior: numeric text formats `TBM_GETPOS` absolute value.

Tests:

- Plaque rect is right 50 px.
- Thumb rect for min/max values lands at expected endpoints.
- Semantic draw order includes trackbar pieces after shell chrome/buttons/flags as implemented.

Acceptance:

- `cargo test --lib skirmish_shell` passes.

### Task 8 - App input integration

**Why:** Mouse move/down/up currently only handles owner-draw buttons and color combos.

Steps:

1. Update `handle_skirmish_shell_mouse_down()`:
   - compute layout once.
   - let new state input handler process checkbox/trackbar first or in verified control priority order.
   - preserve owner-draw button press tracking.
2. Update `CursorMoved` handling for dev skirmish shell if trackbar drag is active.
3. Update `handle_skirmish_shell_mouse_up()`:
   - clear/finish trackbar drag before generic hit-test fallback.
   - keep Start/Choose/Back click behavior unchanged.
4. If checkbox/trackbar changes should play the same shell click sound, use the existing audio mechanism if already available for skirmish shell; otherwise defer sound with a clear TODO and no behavior guess.

Tests:

- Prefer pure state tests. App event tests are likely too heavy.

Acceptance:

- Manual smoke can toggle and drag controls without breaking Start/Back.

### Task 9 - Verification pass

Run:

1. `cargo fmt`
2. `cargo test --lib skirmish_shell`
3. `cargo test --lib skirmish_launch`
4. `cargo test --lib large_screen` if layout helpers touched shared large-screen behavior
5. `cargo build --bin vera20k`

Manual smoke:

1. Launch app.
2. Enter dev skirmish shell path.
3. Confirm five checkbox icons render in lower options rows.
4. Confirm icon clicks toggle; label clicks do not.
5. Confirm three trackbars render with plaque/thumb/value.
6. Confirm dragging/clicking updates values and Start still launches.

## Implementation Order

1. Layout structs/helpers and tests.
2. Trackbar math helpers and tests.
3. State option fields/input and tests.
4. Launch packing tests.
5. Atlas fields/assets/tests.
6. Checkbox rendering.
7. Trackbar rendering.
8. App input integration.
9. Formatting, focused tests, build, smoke.

This order keeps the riskiest parity math in pure tests before any GPU/render work.

## Acceptance Criteria

- Checkbox controls appear at verified positions using `cue_i.pcx` and `cce_i.pcx`.
- Checkbox icon clicks toggle; label clicks do not.
- Trackbar visuals use `trakgrip.pcx` and `trofl/trofm/trofr.pcx`, with 50 px value plaque and 65 px active track width for standard `128x21` controls.
- Trackbar input follows verified vertical gate, thumb interval, clamp, value, and quantization formulas.
- Game Speed packs with retail inversion.
- Credits and Unit Count pack into launch options with verified ranges/steps.
- `ShortGame`, `MCVRepacks`, `CratesAppear`, `SuperWeaponsAllowed`, and `BuildOffAlly` pack into launch options on Start.
- Focused tests pass and the app builds.

## Non-Goals

- Disabled-state checkbox/trackbar overlays.
- Variant checkbox art via `0x4E5/0x4E6/0x4E7`.
- Full slot table completion.
- Combo chrome completion.
- Retail screenshot pixel comparison.
- Making the experimental skirmish shell the default flow.

## Risk Controls

- Do not approximate trackbars with normalized floats.
- Do not use egui widgets.
- Do not use `bst_*`, `BTN-MINS.SHP`, or `BTN-PLUS.SHP` for these standard controls.
- Do not mutate simulation state during shell clicks.
- Do not add broad refactors to the skirmish shell while implementing these controls.
