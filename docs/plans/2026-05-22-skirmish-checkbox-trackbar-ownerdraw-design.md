# Skirmish Checkbox / Trackbar Owner-Draw Design

## Goal

Implement standard offline YR Skirmish `0x102` checkbox and trackbar owner-draw visuals, input, and launch-option packing using the verified retail geometry.

## Architecture Context

The experimental Skirmish shell is already split by responsibility:

- `src/ui/skirmish_shell/layout.rs` owns recovered dialog `0x102` pixel geometry. It already exposes button, preview, flag, color-combo, and trackbar rects.
- `src/ui/skirmish_shell/state.rs` owns shell state, hit testing, action application, and conversion into launch settings/session data.
- `src/render/skirmish_shell_chrome.rs` owns retail shell asset decode and atlas packing. It already supports SHP, PCX, flag color-key decode, and primitive bevel raster helpers.
- `src/app_skirmish_shell_render.rs` composes sprites and text for the active shell render pass.
- `src/skirmish_launch.rs` and `src/sim/game_options.rs` already contain the launch/runtime option fields this UI must eventually set.

This feature stays in `ui/`, `render/`, and app-level glue. It does not introduce any `sim/` dependency on UI or rendering. The active YR behavior is verified by `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`.

## Impact Analysis

Touched modules:

- `src/ui/skirmish_shell/layout.rs`: add checkbox control IDs/rects and small helper geometry for icon/text/plaque/thumb placement.
- `src/ui/skirmish_shell/state.rs`: add checkbox booleans, trackbar values, trackbar drag state, exact hit gates, and launch packing.
- `src/render/skirmish_shell_chrome.rs`: add verified checkbox/trackbar PCX assets to the atlas.
- `src/app_skirmish_shell_render.rs`: add sprite/text construction for checkbox icons, checkbox labels, trackbar rails, value plaques, thumbs, and numeric labels.
- `src/skirmish_launch.rs`: likely no shape change needed; all target fields already exist.

Risk areas:

- Trackbar value math is easy to approximate incorrectly; keep formulas as named pure functions with tests.
- Game speed is visually inverted: UI position is `6 - stored`.
- Checkbox labels are visible but not clickable in retail; hit testing must use the 18x18 icon gate only.
- Rendering a generic slider/checkbox would look plausible but violate verified assets and input behavior.
- Launch packing must happen on Start from shell control state; ordinary checkbox clicks should not directly mutate match state.

## Chosen Approach

Use first-class retail widgets inside the current Skirmish shell path.

The layout model will expose the five checkbox rects and three existing trackbar rects. The state model will own the option values and input gesture state. The renderer will draw from the same layout and state, using only verified standard `0x102` assets:

- `cue_i.pcx`
- `cce_i.pcx`
- `trakgrip.pcx`
- `trofl.pcx`
- `trofm.pcx`
- `trofr.pcx`

Disabled-state overlays and variant checkbox art are documented but not implemented in this patch because standard offline Skirmish does not normally initialize those paths. If a later trace finds a normal YR flow that disables or variants these controls, the model can add those states without changing the core layout/input split.

## Tiny-Detail Ledger

- Standard offline Skirmish `0x102` routes button-style checkbox controls to `OwnerDraw_Checkbox_006163A0`; active in YR. Source: `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`.
- Standard trackbars route to `OwnerDraw_Trackbar_0061D950`; active in YR. Source: same report.
- Checkbox IDs are `0x54E`, `0x693`, `0x696`, `0x69A`, `0x69D`. Source: same report section 3.2.
- Checkbox final rects at 640 include `x-1` fixups for `0x54E/0x693/0x696/0x69A`; Build Off Ally remains `(302,369,249,18)`. Source: `SKIRMISH_SHELL_640X480_FINAL_VISIBLE_LAYOUT_TRACE.md`.
- Standard checkbox art is `cue_i.pcx` unchecked and `cce_i.pcx` checked. Source: checkbox asset selection table.
- Checkbox icon destination/click constants are fixed `18x18`. Source: checkbox paint geometry table.
- Checkbox label rect is control rect with `left += 26`. Source: `0x0061663E..0x00616646`.
- Checkbox label draw flags are `0x04`: left anchored and vertically centered. Source: `0x00616661..0x00616674`.
- Checkbox toggles only inside the 18x18 icon gate; label clicks do not toggle. Source: `0x006166EE..0x00616708`.
- Checkbox clicks notify parent, but standard option globals are applied later by rereading controls on Start/Back accept. Source: `FUN_006ACEE0`.
- Trackbar IDs are `0x529` game speed, `0x511` credits, `0x50C` unit count. Source: report section 3.2.
- Trackbar rects are `0x529=(404,286,128,21)`, `0x511=(404,314,128,21)`, `0x50C=(404,340,128,21)` in the current Rust final geometry. Source: layout tests and report.
- Standard trackbar normalizes to numeric plaque width `50` and active width `128 - 50 - 13 = 65`. Source: `0x0061DA52..0x0061DBAD`.
- Trackbar assets are `trakgrip.pcx`, `trofl.pcx`, `trofm.pcx`, `trofr.pcx`; `BTN-MINS.SHP` and `BTN-PLUS.SHP` are verified negative for this path. Source: trackbar asset table.
- Thumb draw position is `control_left + 1 + pixel_offset`, `control_top`; input treats thumb width as `12`. Source: `0x0061E00C..0x0061E0AD`, `0x0061E518..0x0061E540`.
- Trackbar interaction requires `mouse_y > client_bottom - 18`; the top 4 px of a 21 px control do not interact. Source: `0x0061E4F5..0x0061E512`.
- Raw mouse x uses `mouse_x - 6`, clamped to `[1, client_right - plaque_width - 12]`. Source: `0x0061E545..0x0061E568`.
- Raw value is `((x - 1) * (span + 1)) / active_width`, saturating at `span`, then quantized by step with truncating integer division. Source: `0x0061E568..0x0061E594`.
- Parent notification on changed trackbar value uses `WM_HSCROLL`, low word `5`, high word current absolute value. Source: final branch `0x0061E609`. Rust does not need Win32 messaging, but the same state-change boundary should drive local action handling.
- Game speed initializes UI position as `6 - stored` and applies stored value as `6 - TBM_GETPOS`. Source: `FUN_006AE6E0`, `FUN_006ACEE0`.
- Credits range and step come from Rules defaults: min `5000`, max `10000`, step `100` in YR. Source: `rulesmd.ini:3018..3021`.
- Unit Count range is Rules default `0..10`, step `1`. Source: `rulesmd.ini:3022..3024`.
- Build Off Ally fallback is enabled in standard YR unless overridden. Source: prior binary constructor evidence and current `GameOptions::default()`.

## Design

### Components

`layout.rs`

- Add `SkirmishCheckboxId` and `SkirmishCheckboxRect`.
- Add `SkirmishCheckboxRects` or a fixed `[SkirmishCheckboxRect; 5]` to `SkirmishShellLayout`.
- Keep trackbar rects as the existing `SkirmishTrackbarRects`.
- Add pure helpers for:
  - `checkbox_icon_rect(rect) -> RectPx`
  - `checkbox_text_rect(rect) -> RectPx`
  - `trackbar_plaque_rect(rect) -> RectPx`
  - `trackbar_active_width(rect) -> i32`
  - `trackbar_thumb_rect(rect, value_state) -> RectPx`

`state.rs`

- Add `SkirmishCheckboxId`, `SkirmishTrackbarId`, and `TrackbarDragState`.
- Extend `SkirmishShellState` with:
  - `game_speed: i32`
  - `unit_count: i32`
  - `super_weapons: bool`
  - `build_off_ally: bool`
  - `crates: bool`
  - `mcv_redeploy: bool`
  - existing `starting_credits` and `short_game`
- Add pure trackbar math functions:
  - value to pixel offset
  - mouse x to quantized value
  - thumb interval hit
  - vertical gate
- Add mouse-down/move/up handling for checkbox and trackbar controls, separate from owner-draw button press state.
- Preserve the existing `SkirmishShellAction` shape where practical; add action variants only when the app needs to react. Most option toggles can mutate local shell state directly in a dedicated state function.

`skirmish_shell_chrome.rs`

- Add atlas fields for:
  - checkbox unchecked/checked PCX
  - trackbar grip
  - trackbar plaque left/middle/right
- Decode these as PCX entries. Use the same transparent-index behavior as retail asset inspection confirms during implementation; if current decoder evidence is insufficient, inspect the PCX palette/index before choosing color-key handling.
- Classify these assets as verified owner-draw controls in tests.

`app_skirmish_shell_render.rs`

- Add draw roles for checkbox and trackbar pieces.
- Draw checkbox icon at native `18x18` top-left and draw text in `left+26` rect with vertical-centering.
- Draw trackbar plaque at right, rail/bevel in the active track area, grip at `left+1+pixel_offset`, and numeric text in the right value rect.
- Keep all positions in native screen pixels; do not scale the controls.

### Interfaces / Contracts

- `compute_layout(width,height)` remains the single source of screen-space geometry.
- State hit tests receive `&SkirmishShellLayout` and window/render coordinates.
- Render receives `&SkirmishShellLayout` and `&SkirmishShellState`.
- `launch_session()` packs all option values from `SkirmishShellState` into `SkirmishLaunchOptions` on Start.

### Data Flow

```text
Window mouse input
  -> app.rs skirmish shell mouse handler
  -> ui::skirmish_shell::compute_layout
  -> state.rs checkbox/trackbar/button hit handling
  -> SkirmishShellState updated
  -> render_skirmish_shell uses same layout + state
  -> StartGame action calls launch_session / launch_settings
  -> SkirmishLaunchOptions -> GameOptions
```

### Error Handling

- Missing verified shell assets should log warnings and omit only that visual piece when optional, following the existing atlas pattern.
- Mandatory core shell chrome behavior should not be weakened for checkbox/trackbar additions.
- If trackbar PCXs are missing, keep state/input deterministic but visibly incomplete; log specific asset names.
- No panics for ordinary user input outside control rects.

### Testing Strategy

Focused unit tests:

- Checkbox rects match verified 800/640 geometry.
- Checkbox icon gate toggles; label area does not.
- Checkbox text rect applies `left+26`.
- Trackbar active width is 65 for `128x21`.
- Mouse top-edge gate rejects top 4 px.
- Thumb interval starts drag instead of immediate remap.
- Mouse x to value mapping matches verified clamp and integer formulas.
- Game speed packs inverted value.
- Credits snaps by 100 and clamps to `5000..10000`.
- Unit count clamps to `0..10`.
- Launch session packs all checkbox and trackbar options.
- Atlas classification includes only verified owner-draw PCX assets, not `bst_*`, `BTN-MINS.SHP`, or `BTN-PLUS.SHP`.

Manual smoke after implementation:

- Run the shell at 800x600 or 640x480.
- Confirm checkbox icons draw at the lower options rows.
- Confirm clicking labels does not toggle.
- Confirm dragging/clicking each trackbar updates thumb and numeric value.

## Architectural Decisions

- Follow the existing shell split: layout in `ui`, decode/atlas in `render`, composition in app glue, launch packing in `ui/skirmish_shell/state.rs`.
- Do not introduce a generic widget framework yet. The verified retail behavior is specific enough that generic abstractions would risk hiding parity details.
- Keep disabled and variant checkbox branches deferred because the standard offline Skirmish path does not initialize them. This is not accepted drift for standard enabled controls; it is out-of-scope verified helper behavior.
- Keep all shell geometry in integer pixels. No normalized sliders or egui controls.

## Alternatives Considered

### Render-only first

Rejected. It would show controls that do not behave like retail, leaving a visible parity hole in normal Skirmish setup.

### Generic reusable shell widgets

Rejected for this patch. It may be useful after more shell dialogs need the same owner-draw callbacks, but this task has enough verified callback-specific detail that a generic layer would add risk before value.

### Minimal state-only option packing

Rejected. It would let Start pack options but leave the screen visibly incomplete, which fails the player-visible parity target.
