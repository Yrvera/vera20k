# Skirmish 0x102 Verified Layout Dropdown Preview Parity Pass Design

## Goal

Bring the verified standard offline Skirmish `0x102` first-screen layout, dropdown, and preview overlay details into parity with `gamemd.exe` without expanding into Choose Map `0x6B` or right-panel reveal work.

## Architecture Context

The current Skirmish shell is already split along useful boundaries:

- `src/ui/skirmish_shell/layout.rs` computes logical pixel rectangles for the `0x102` setup shell.
- `src/ui/skirmish_shell/state.rs` owns shell state, combo/dropdown item lists, top-index state, hit testing, and input actions.
- `src/app_skirmish_shell_render.rs` translates shell state/layout into sprite and text instances for WGPU rendering.

This pass stays above `sim/` and does not change gameplay, map parsing, launch state, or asset-loading ownership. The design follows the existing pattern of encoding verified shell formulas as small layout/state/render helpers plus focused tests beside the existing Skirmish shell tests.

## Impact Analysis

Touched files:

- `src/ui/skirmish_shell/layout.rs`
  - Apply the verified `0x102` one-pixel fixups for unit-count trackbar and four option checkboxes.
  - Update/add table-driven rect tests.

- `src/ui/skirmish_shell/state.rs`
  - Replace dropdown track-click page stepping with native proportional top-index calculation.
  - Keep scrollbar arrows as one-row steps.
  - Keep row hit testing inside the content rect only, not the scrollbar column.
  - Restrict normal color combo visible population to retail inserted rows.

- `src/app_skirmish_shell_render.rs`
  - Draw selected dropdown row fill as a full content row.
  - Replace preview aspect fit float/rounding with integer per-mille truncation.
  - Submit STARTBUT marker sprites and labels without fitted-preview containment filtering or fitted-preview scissor.
  - Move numeric labels to the verified offset and use the yellow overlay color path available in Rust.

Regression risks:

- Existing layout tests will need expected rect updates for `0x50C` and four checkboxes.
- Dropdown tests that expect page scroll on track click must be rewritten to assert native proportional jumps.
- Removing fitted-preview scissor for live overlays may expose marker pixels outside the fitted image. That is intended retail behavior, but visual checks should verify the render target still clips correctly.
- The label glyph helper remains Rust's existing bit-font path, not a full `FUN_004A61C0` clone. This pass fixes caller origin/color/clip behavior, not full font internals.

## Chosen Approach

Use a focused parity patch in the existing functions.

This is preferred over a new generic native-control layer because every target in this pass already has a clear owner and local test surface. It is also preferred over a full 72-child `0x102` table model because the verified deltas are narrow and do not require replacing the current layout structure.

## Tiny-Detail Ledger

- Unit-count trackbar `0x50C` final rect is `(404,340,128,21)`, not raw DLU `(404,341,128,21)`. Source: `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`.
- Checkboxes `0x54E`, `0x693`, `0x696`, and `0x69A` final x is `71`; `0x69D` remains x `302`. Source: `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`.
- Ordinary row controls, color combos, flags, start/team combos, option labels, and most trackbars are not high-res shifted or scaled. Source: `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`.
- Dropdown popup top is `combo_top + COMBO_FACE_H + 1`, not the bottom of the tall resource combo rect. Source: `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`.
- Dropdown height is whole owner-draw rows after cap/rounding. Source: `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`.
- Scrollbar width is `20` px, and content width/hit testing shrink by that width when present. Source: `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`.
- Scrollbar arrow buttons are `22` px high and step one row. Source: `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`.
- Scrollbar track clicks jump to the top index implied by centering the thumb on the click; they are not page-up/page-down by visible rows. Source: `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`.
- Selected popup row fill spans the full content row before swatch/text. Source: `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`.
- Normal color combo inserts sentinel `-2` plus color rows `0..7`; initialized row `8` is not normally inserted. Source: `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`.
- Direct mouse-wheel handling was not found in the scoped combo/popup/scrollbar callbacks. Source: `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`.
- Preview child `0x468` remains parent-painted and right-anchored; this pass does not change that rect formula. Source: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`.
- Preview image aspect fit uses integer per-mille truncation; Dustbowl `138x75` inside `144x112` fits `(child_x+1, child_y+17, 143, 78)`. Source: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`.
- Live overlay count gate is `1..8` from `[Header]` preview fields; do not synthesize live overlays from `[Waypoints]` or `[Map] LocalSize`. Source: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`.
- `STARTBUT.SHP` frame `0` is drawn at `(anchor_x-9, anchor_y-6)`. Source: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`.
- Numeric labels are 1-based and drawn at `(anchor_x-2, anchor_y-6)` with the `"Yellow"` color path and destination-surface clipping. Source: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`.
- Marker sprites and labels are not rejected for projected anchors outside the fitted preview rect; clipping is delegated to the destination surface. Source: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`.
- Preview numeric labels are not ordinary `FUN_00621040` shell text. Source: `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`.

## Design

### Components

#### Layout Fixups

In `compute_layout`, keep the current explicit rectangle construction and apply only the verified post-resize fixups:

- Decrement `unit_count_trackbar.y` by `1`.
- Shift only the first four checkbox rects left by `1`.
- Leave `BuildOffAlly0x69d` unchanged.

This should be expressed directly where the rects are created, or through a tiny helper with an explicit name such as `apply_0102_checkbox_fixup`. Do not imply this is a general checkbox rule.

#### Dropdown State/Input

Keep the existing `OpenComboDropdown { id, top_index }` model.

Add or reuse a helper for native absolute scrollbar position:

- Compute the same thumb height as `combo_dropdown_scroll_thumb_rect`.
- Use the track span after subtracting two `22` px arrow zones and thumb height.
- For track clicks outside the thumb, compute the target thumb top by centering the thumb on the click, clamp it to the track, and convert to `top_index` with the existing rounded proportional formula.
- Keep arrows calling `scroll_open_combo_by_rows(..., +/-1)`.
- Keep drag using the existing `top_index_from_thumb_y` path.

Color combo item population should expose the retail normal rows. If the current enum does not model sentinel `-2` for colors, the implementation should decide whether to add a `Random/Auto` color item variant or verify whether Rust's current UI intentionally omits the sentinel. The normal row-8 omission is not optional: visible rows must not include initialized row `8`.

#### Dropdown Render

Change selected row fill from:

- `content.x + 1`, `content.y + 1 + row * row_h`, `content.w - 2`, `row_h - 2`

to:

- `content.x`, `content.y + row * row_h`, `content.w`, `row_h`

Keep swatches and text drawn after the selected fill. Keep scrollbar rendering separate and do not let content use the scrollbar column.

#### Preview Fit And Overlays

Replace `aspect_fit_rect` with the binary integer formula:

- `scale = min(dst.w * 1000 / src_w, dst.h * 1000 / src_h)`.
- `fitted_w = src_w * scale / 1000`.
- `fitted_h = src_h * scale / 1000`.
- Center using integer truncation matching the documented `src * scale / 2000` half-scaled behavior.

Keep `project_preview_start_positions` source-gated by `PreviewSourceBounds`; no waypoint fallback.

Remove fitted-preview containment checks from marker sprite and label builders. Remove fitted-preview scissor around marker and label draw calls. The render target/destination surface is the clipping boundary for this pass.

Move label instances to `(anchor_x-2, anchor_y-6)`. Use the existing yellow shell label color constant for this pass unless a later standalone `FUN_004A61C0` glyph/color investigation provides a more exact font path.

### Interfaces / Contracts

- `compute_layout(screen_w, screen_h)` continues returning `SkirmishShellLayout`.
- Combo helpers continue returning `RectPx` and item vectors.
- Render instance builders keep their current signatures unless simplifying preview overlay clipping requires removing an unused `preview_rect` parameter.
- No public gameplay/session contracts change.

### Data Flow

1. App asks `compute_layout` for `0x102` rects.
2. Input routes through `handle_combo_mouse_down` and updates `open_combo_dropdown.top_index` or applies selection.
3. Renderer asks dropdown helpers for popup/content/scrollbar/thumb rects.
4. Renderer draws dropdown background, selected row, row swatches/text, scrollbar, then border.
5. Renderer decodes/fits preview, projects `[Header]` overlay points, submits preview image, STARTBUT marker sprites, labels, then shell text.

### Error Handling

All new helpers should preserve the current fail-closed behavior:

- Empty combo item lists return no popup.
- Zero or negative preview dimensions return an empty fitted rect.
- Missing `STARTBUT.SHP` still allows numeric labels to be submitted if live overlay positions exist, matching the report's caller behavior.
- Missing preview source bounds produces no live overlays.

### Testing Strategy

Add or update focused tests:

- `skirmish_unit_count_trackbar_applies_0102_fixup_y_minus_one`.
- `skirmish_option_checkboxes_apply_0102_fixup_x_minus_one`.
- `skirmish_side_dropdown_scrollbar_track_click_jumps_to_native_top_index`.
- `skirmish_dropdown_selected_row_fill_is_full_row`.
- `skirmish_color_dropdown_normal_population_omits_initialized_row_8`.
- `skirmish_preview_aspect_fit_uses_gamemd_integer_per_mille_truncation`.
- `start_marker_overlays_use_destination_surface_clip_not_preview_rect`.
- `start_marker_labels_use_startbut_overlay_origin_and_yellow_color`.
- Preserve or update loose-Dustbowl no-live-overlay tests so `[Waypoints]` is not used as fallback.

Run focused Rust tests for `skirmish_shell` and `app_skirmish_shell_render` if the current workspace build blockers allow it. If unrelated known compile blockers remain, record them explicitly.

## Architectural Decisions

- Keep this as a focused patch in existing shell modules. The behavior is narrow and already has direct owners.
- Do not introduce a full native Win32 control simulation layer yet.
- Do not replace `SkirmishShellLayout` with a full 72-child matrix table in this pass.
- Do not implement Choose Map `0x6B`; latest research marks exact listbox item height unresolved.
- Do not implement right-panel reveal/disabled-state text here; it is verified but outside this selected tight scope.

## Alternatives Considered

### Native Helper Layer

A mini helper layer for shell-native controls would make names like native scrollbar track math and native preview fit reusable. It was not chosen because this pass is local and the current functions already provide clear homes.

### Full `0x102` Matrix Model

A data-backed 72-child matrix could be useful later, especially for complete visual audits. It was not chosen because it would pull in unrelated surfaces and increase risk without being necessary for the verified deltas.

### Defer Preview Overlays

Deferring overlay clipping/label offsets would leave a known visible parity gap on maps with live `[Header]` overlay fields. The evidence is strong enough to include it now.
