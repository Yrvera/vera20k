# Skirmish Shell UI Parity Design

## Goal

Bring the experimental offline Skirmish shell closer to retail Yuri's Revenge by rendering the verified shell controls and making Start Game validate and pack the same state those controls show.

## Architecture Context

The current shell is already split along useful boundaries.

- `src/ui/skirmish_shell/layout.rs` owns pixel/DLU layout for the shell surface: right panel, buttons, preview, flags, color combos, checkboxes, and trackbars.
- `src/ui/skirmish_shell/state.rs` owns shell state, hit testing, checkbox toggles, trackbar state, and the current `launch_session` conversion.
- `src/app_skirmish_shell_render.rs` owns sprite/text assembly for the experimental shell render pass.
- `src/render/skirmish_shell_chrome.rs` loads and packs verified retail shell assets.
- `src/skirmish_launch.rs` is a data-only bridge between shell state and app-level skirmish startup.
- `src/app_skirmish.rs` and `src/app_init.rs` consume the launch session during scenario setup.

The key invariant remains that `sim/` must not depend on `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`. This work should keep all shell-specific encoding in `ui/` and `skirmish_launch.rs`, then pass semantic launch data into app-level setup.

The current experimental shell renders chrome, right-panel pieces, preview texture, owner-draw buttons, and flags. It already has some checkbox and trackbar layout/input helpers, but it does not render the verified checkboxes, trackbars, collapsed combo faces, static labels, or exact button text rects. Its Start path is also narrower than the verified gamemd Start branch.

The primary orientation document is `docs/research/SKIRMISH_SHELL_UI_SYSTEM_MODEL_SYNTHESIS.md`. Its source ledger points to the underlying Ghidra reports for checkbox/trackbar geometry, combo geometry, text caller contracts, AI row item data, row visibility, and Start packing.

## Impact Analysis

Expected touched files:

| File | Responsibility |
|---|---|
| `src/ui/skirmish_shell/layout.rs` | Add missing row/control rect groups and helpers while preserving verified 640/800 tests. |
| `src/ui/skirmish_shell/state.rs` | Make AI row type item data exact, add map-capacity/team validation, and pack launch sessions from visible shell state. |
| `src/ui/skirmish_shell/mod.rs` | Re-export new layout/state helpers. |
| `src/render/skirmish_shell_chrome.rs` | Load/persist checkbox, trackbar, and combo-arrow PCX entries in the chrome atlas. |
| `src/app_skirmish_shell_render.rs` | Render checkboxes, trackbars, collapsed combos, statics, row text, and corrected button labels. |
| `src/skirmish_launch.rs` | Keep data-only launch fields/errors aligned with validated shell state. |
| `src/app.rs` | Route mouse move/up for trackbar dragging and keep validation failures inside the shell. |

Risk areas:

- AI row difficulty has two encodings. Retail shell item data is `0=Hard`, `1=Normal`, `2=Easy`, with `-1=None`; Rust's semantic enum order must not be used as item data by accident.
- Render helpers may need more atlas entries. Missing retail assets should degrade by skipping the specific visual piece, not by substituting unrelated art.
- Start validation should not make the experimental shell default-ready. The shell remains behind the existing dev toggle until visible controls and launch behavior are tested together.
- Exact full dropdown row paint, live preview start-marker overlays, exact >800 pixels, and post-launch mode callbacks are still outside this pass.

## Chosen Approach

Use an integrated parity pass behind the existing dev toggle. The same `SkirmishShellState` drives rendering, hit testing, Start validation, and `SkirmishLaunchSession` packing.

This is preferred over a visuals-only pass because a retail-looking shell that still starts invalid or mismatched games creates misleading parity. It is preferred over a Start-only pass because much of the launch state is otherwise invisible or hard to test.

The work should be implemented in small internal phases, but the design treats visible state and launch state as one contract.

## Tiny-Detail Ledger

- Start/Choose/Back button labels use an inset caller rect: released `top+1`, `right-2`; pressed shifts `left+2/top+5`; flags are horizontal center plus vertical center. Source: `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`.
- Checkbox art is `cue_i.pcx` unchecked and `cce_i.pcx` checked for standard offline Skirmish. `bst_*` assets are not used by this path. Source: `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`.
- Checkbox toggles only inside the 18x18 icon gate; label clicks do not toggle. Source: same report.
- Checkbox text starts at `control.x + 26`, is left anchored, and is vertically centered. Source: same report.
- Trackbars use `trakgrip.pcx` plus `trofl.pcx`, `trofm.pcx`, and `trofr.pcx`; rails are primitive/beveled, not plus/minus SHP controls. Source: same report.
- Standard 128x21 trackbars have active width 65 px, a 12 px thumb interval, and a top-pixel y gate that rejects the top 4 px. Source: same report.
- Game Speed visual position is `6 - stored`. Source: same report.
- Credits snap by `MoneyIncrement`; Unit Count snaps by 1. Source: same report and cited `rulesmd.ini` multiplayer dialog settings.
- Collapsed combo faces paint as fixed 24 px faces, reserve 20 px for the arrow, and use `(2,2,20,20)` swatch fill for 44 px color combos. Source: `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`.
- Offline AI row type order/item data is `-1=None`, `2=Easy`, `1=Normal`, `0=Hard`; only `0`, `1`, and `2` count active. Source: `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`.
- Selected-map start count hides/closes AI rows beyond capacity; Start re-counts active row item data instead of trusting visibility alone. Source: `SKIRMISH_PLAYER_AI_ROW_VISIBILITY_ENABLE_RULES_GHIDRA_REPORT.md`.
- Start validates selected map capacity, at least two total players, and same-explicit-team rejection before packing. Failures re-enable Start and stay in the shell. Source: `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`.
- Do not synthesize `STARTBUT.SHP` marker overlays from gameplay waypoints or decoded PreviewPack alone. Source: `SKIRMISH_SHELL_UI_SYSTEM_MODEL_SYNTHESIS.md`.

## Design

### Components

#### Shell row state

Introduce or refactor toward a shell-facing AI row type that preserves retail item data explicitly:

```rust
pub enum SkirmishAiRowType {
    None,
    Easy,
    Normal,
    Hard,
}
```

The type should expose `item_data()` returning `-1`, `2`, `1`, or `0`, and `is_active()` for `{Easy, Normal, Hard}`. It should not rely on enum discriminants.

`SkirmishShellOpponent` should use this row type as the active/inactive authority. A compatibility helper can keep `enabled` behavior during migration, but launch validation should count row type item data.

#### Layout

Extend `SkirmishShellLayout` with row-control rects for the visible local row and seven AI rows. Existing represented controls remain stable:

- `player_name`
- `flags[8]`
- `color_combos[8]`
- `trackbars`
- `checkboxes`

Add row combo families only where geometry is verified enough for collapsed faces. Full dropdown listbox row paint is not in scope.

#### Atlas

Extend `SkirmishShellChromeAtlas` with optional entries:

- `checkbox_unchecked_cue_i`
- `checkbox_checked_cce_i`
- `trackbar_thumb_trakgrip`
- `trackbar_plaque_left_trofl`
- `trackbar_plaque_mid_trofm`
- `trackbar_plaque_right_trofr`
- `combo_arrow_down_released_dnarrowr`
- optional pressed/grey arrow entries if present and needed

Missing optional pieces should log and skip rendering only that piece.

#### Render pass

Add render helpers that read only `layout`, `shell`, and `atlas`:

- `push_checkbox_instances`
- `push_trackbar_instances`
- `push_collapsed_combo_instances`
- `push_shell_static_text_draws`
- `push_row_text_draws`
- corrected `push_button_label_draw`

Use `shell_text::draw_in_rect` for text, with caller-specific rects and flags from the ledger.

#### Start validation and packing

`launch_session(state, maps)` should validate in this order:

1. selected map exists;
2. active AI row count from row item data;
3. selected-map capacity from `MapMenuEntry.multiplayer_start_waypoints.len()`;
4. at least one active opponent;
5. same-explicit-team rejection when the local player has an explicit team and all active AI rows have the same explicit team;
6. color/start index validity;
7. pack local + active AI slots into `SkirmishLaunchSession`.

Validation failures return typed `LaunchValidationError` values. `app.rs` keeps the shell open on error.

### Interfaces / Contracts

- Render code must not own shell truth. It renders `SkirmishShellState`.
- `SkirmishLaunchSession` remains data-only and independent of render/UI implementation details.
- Shell item data is converted to semantic launch difficulty at the shell/session boundary.
- App-level scenario setup consumes semantic launch data and does not learn owner-draw control IDs.

### Data Flow

```text
mouse/key input
  -> ui::skirmish_shell::state helpers
  -> SkirmishShellState
  -> app_skirmish_shell_render reads state for visible shell
  -> Start Game
  -> launch_session validates visible state and packs SkirmishLaunchSession
  -> app.rs stores pending session and starts selected map
  -> app_init/app_skirmish apply launch session
```

### Error Handling

Extend `LaunchValidationError` with variants for map capacity and same-team rejection. App-level handling can initially log or surface a lightweight shell message, but it must not start the game on failure.

### Testing Strategy

Unit tests:

- AI row type item data/order and active-row counting.
- `launch_session` rejects no map, no active opponent, too many players for map starts, invalid color/start, and same explicit team.
- `launch_session` converts `Hard/Normal/Easy` item data to the semantic launch difficulty correctly.
- Button label rect helper matches released/pressed caller contract.
- Checkbox helper selects `cue_i.pcx` vs `cce_i.pcx`.
- Trackbar helpers preserve 65 px active width, 12 px thumb interval, y gate, and game-speed inversion.
- Collapsed combo helper preserves 24 px face, 20 px arrow reserve, and color swatch inset.

Focused checks:

- `cargo test skirmish_shell`
- `cargo test skirmish_launch`
- `cargo test app_skirmish_shell_render`

Full app/manual verification is useful after implementation because this is player-visible UI.

## Architectural Decisions

- Keep the dev toggle. The shell should improve behind the experimental path and not replace the egui Skirmish setup until the remaining visible and launch gaps are closed.
- Keep shell control IDs and retail item data in `ui/skirmish_shell`, not in `sim/`.
- Do not introduce a generic widget framework. The retail shell is owner-draw and asset-backed; small focused helpers fit the existing render module better.
- Defer dropdown listbox rows, preview markers, >800 exact background behavior, and post-launch mode callbacks because the synthesis marks them as investigation-blocked or outside this first integrated pass.

## Alternatives Considered

### Visuals first

Would render checkboxes, trackbars, combos, labels, and corrected buttons before changing Start behavior. Rejected because it makes the shell look more retail while accepting mismatched launch states.

### Start packing first

Would fix validation/session packing before rendering missing controls. Rejected because much of the state remains invisible and harder to test from the actual shell.

### Replace with egui controls

Rejected. The verified shell path is a retail asset-backed owner-draw UI, not an egui form.
