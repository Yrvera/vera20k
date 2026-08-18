# Skirmish 0x102 Verified Layout Dropdown Preview Parity Pass Implementation Plan

> Execute this plan task-by-task. This is a planning document only; do not implement Rust from this document until it is explicitly approved.

**Goal:** Implement the approved focused parity pass from `docs/plans/2026-05-22-skirmish-0x102-verified-layout-dropdown-preview-parity-pass-design.md`.

**Design Input:** User-approved Approach A: patch the existing `layout.rs`, `state.rs`, and `app_skirmish_shell_render.rs` surfaces directly, without introducing a new native-control layer.

---

## Grounding Summary

- Standard offline YR Skirmish setup uses dialog `0x102`; most child controls preserve resource rects, but `0x50C` and four checkbox controls receive verified one-pixel fixups.
- Rust already has a matching shell split: layout in `layout.rs`, state/input in `state.rs`, and draw instance generation in `app_skirmish_shell_render.rs`.
- The latest `/re-swarm` made the layout, dropdown, and preview overlay deltas implementation-ready.
- This plan does not implement Choose Map `0x6B`, right-panel reveal animation, player-name edit text, or exact retail scrollbar drag skin/pressed-frame polish.

## Key Technical Decisions

- **Keep the patch local.** Do not replace `SkirmishShellLayout` with a full 72-child matrix table in this pass.
- **Represent fixups explicitly.** The `0x50C` y-1 and first-four-checkbox x-1 adjustments are special `0x102` behavior, not reusable widget rules.
- **Reuse current dropdown state.** Keep `OpenComboDropdown { id, top_index }`; only replace track-click math and item population.
- **Preserve content shrink.** Dropdown row drawing and hit testing must continue using `combo_dropdown_content_rect`.
- **Use integer preview math.** Replace float/rounding in `aspect_fit_rect` with the verified per-mille truncation formula.
- **Do not synthesize overlays.** Preview overlays remain sourced only from verified `[Header]` preview fields.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/ui/skirmish_shell/layout.rs` | Apply verified `0x102` rect fixups and update rect tests. |
| Modify | `src/ui/skirmish_shell/state.rs` | Replace dropdown track-click math and restrict visible color rows. |
| Modify | `src/app_skirmish_shell_render.rs` | Fix selected row fill, preview aspect fit, STARTBUT marker/label clipping and offsets. |
| Maybe modify | `src/ui/skirmish_shell/mod.rs` | Only if new helper functions need re-export for tests/render. |

## Task Plan

### Task 1 - Layout: Apply Verified `0x102` One-Pixel Fixups

**Why:** The unit-count trackbar and four option checkboxes are visibly one pixel off against the verified dialog matrix.

Steps:

1. In `compute_layout`, make `unit_count_trackbar` mutable and decrement `y` by `1`.
2. Shift only these checkbox rects left by `1`:
   - `ShortGame0x54e`
   - `McvRepacks0x693`
   - `CratesAppear0x696`
   - `SuperWeapons0x69a`
3. Leave `BuildOffAlly0x69d` unchanged.
4. Update layout tests that currently expect raw DLU positions.
5. Add focused tests if existing table coverage is not explicit enough:
   - `skirmish_unit_count_trackbar_applies_0102_fixup_y_minus_one`
   - `skirmish_option_checkboxes_apply_0102_fixup_x_minus_one`

Acceptance:

- At 800x600, `trackbars.unit_count == RectPx::new(404,340,128,21)`.
- At 1024x768, `trackbars.unit_count` remains `RectPx::new(404,340,128,21)`; ordinary controls do not receive high-res offsets.
- At 800x600, first four checkbox controls start at x `71`.
- At 1024x768, the first four checkbox controls still start at x `71`.
- `BuildOffAlly0x69d` still starts at x `302`.

### Task 2 - Dropdown State: Native Track-Click Top Index

**Why:** Retail scrollbar track clicks jump proportionally; current Rust page-scrolls by visible row count.

Steps:

1. Add a helper such as `top_index_from_scrollbar_track_click(...)`.
2. Reuse the same scrollbar rect, thumb rect, max top index, arrow height `22`, and min thumb behavior already used by `combo_dropdown_scroll_thumb_rect`.
3. For track clicks outside the thumb:
   - Center the thumb on the click: `target_thumb_top = y - thumb.h / 2`.
   - Clamp to the post-arrow track.
   - Convert to `top_index` using the same rounded proportional formula as drag.
4. Keep top arrow as one-row up and bottom arrow as one-row down.
5. Keep thumb drag behavior routed through `top_index_from_thumb_y`.
6. Update existing dropdown scrollbar tests that currently assert page-scroll behavior.
7. Add or rename a focused test:
   - `skirmish_side_dropdown_scrollbar_track_click_jumps_to_native_top_index`.

Acceptance:

- A side/country dropdown with overflow changes to the proportional top index on track click.
- Clicking the arrow zones still changes top index by exactly one row.
- Clicking in the scrollbar column does not select a row.

### Task 3 - Dropdown State: Restrict Normal Color Rows

**Why:** Retail normal color combo population inserts the sentinel plus rows `0..7`; initialized row `8` is not normally visible.

Steps:

1. Inspect `SkirmishComboItem::Color` and current labels/rendering for color items.
2. Add an explicit color sentinel item for the retail `-2` row if the existing Rust model does not already represent it.
3. Wire sentinel rendering and selection intentionally:
   - It must appear before color rows `0..7` in normal color dropdowns.
   - It must not expose initialized row `8`.
   - It must not break existing stored player/opponent color indices.
4. If sentinel support cannot be implemented without broad enum/render/input rewiring, stop before changing color population and ask for a scope decision; do not ship a no-sentinel partial fix under this plan.
5. Add a test:
   - `skirmish_color_dropdown_normal_population_omits_initialized_row_8`.

Acceptance:

- Normal color dropdown item order is sentinel `-2`, then colors `0..7`.
- Normal visible color item indices do not include `8`.
- Existing selected player/opponent colors clamp safely if stale state still contains row `8`.
- The test must assert both sentinel presence and row-8 omission.

### Task 4 - Dropdown Render: Full-Row Selected Fill

**Why:** Retail fills the whole content row before swatch/text; Rust currently insets the selected fill by one pixel.

Steps:

1. In `push_dropdown_instances`, replace the selected rect with:
   - x `content.x`
   - y `content.y + (selected - open.top_index) * COMBO_DROPDOWN_ROW_H`
   - w `content.w`
   - h `COMBO_DROPDOWN_ROW_H`
2. Keep the selected fill depth before swatch/text.
3. Preserve dropdown background and border draw order.
4. Add a focused render-instance test:
   - `skirmish_dropdown_selected_row_fill_is_full_row`.

Acceptance:

- Selected fill does not draw into the scrollbar column because `content.w` is already shrunk.
- Selected fill starts at the exact row y, not `row_y + 1`.
- Swatch/text still draw on top of selected fill.

### Task 5 - Preview: Replace Aspect Fit With Integer Per-Mille Formula

**Why:** Float rounding makes common preview surfaces one pixel too wide/left versus `gamemd.exe`.

Steps:

1. Rewrite `aspect_fit_rect` with integer math:
   - `scale_w = dst.w * 1000 / src_w`
   - `scale_h = dst.h * 1000 / src_h`
   - `scale = min(scale_w, scale_h)`
   - `fitted_w = src_w * scale / 1000`
   - `fitted_h = src_h * scale / 1000`
2. Center using the report's half-scaled truncation formula, not `(dst.w - fitted_w) / 2`:
   - `fit_x = dst.x + dst.w / 2 - (src_w * scale) / 2000`
   - `fit_y = dst.y + dst.h / 2 - (src_h * scale) / 2000`
3. Keep zero/negative guards.
4. Update the existing Dustbowl aspect-fit test.
5. Add or rename:
   - `skirmish_preview_aspect_fit_uses_gamemd_integer_per_mille_truncation`.

Acceptance:

- At 800x600, preview child `(644,37,144,112)` with `138x75` source fits to `(645,54,143,78)`.
- No floating-point math remains in `aspect_fit_rect`.

### Task 6 - Preview: Submit STARTBUT Sprites And Labels Without Fitted-Rect Clipping

**Why:** Retail submits live overlays even when the projected anchor is outside the fitted preview rect and relies on destination-surface clipping.

Steps:

1. Remove `preview_rect.contains(x, y)` filtering from marker sprite submission.
2. Remove `preview_rect.contains(x, y)` filtering from marker label submission.
3. Remove fitted-preview scissor around marker sprite draw.
4. Remove fitted-preview scissor around marker label draw.
5. Keep render-target clipping from the pass itself; do not introduce a new preview scissor.
6. Preserve render order: preview surface first, STARTBUT marker sprites second, numeric labels third, then ordinary shell text.
7. Add tests:
   - `start_marker_overlays_use_destination_surface_clip_not_preview_rect`
   - preserve loose-Dustbowl no-live-overlay behavior.

Acceptance:

- Projected anchors outside the fitted image still produce marker/label instances.
- Missing `PreviewSourceBounds` still produces no marker/label instances.
- No `[Waypoints]` fallback is added.
- Marker sprites and numeric labels remain above the preview surface and below ordinary shell text.

### Task 7 - Preview: Correct Label Origin And Color

**Why:** Retail numeric labels are 1-based and drawn at `anchor-2, anchor-6` with the yellow overlay color path.

Steps:

1. In `push_start_marker_labels`, draw labels at:
   - x `anchor_x - 2`
   - y `anchor_y - 6`
2. Keep labels 1-based.
3. Use the existing yellow label color constant rather than `SHELL_BUTTON_TEXT_RGB_00000C05`.
4. Do not block labels on `STARTBUT.SHP` asset availability.
5. Add or update:
   - `start_marker_labels_use_startbut_overlay_origin_and_yellow_color`.

Acceptance:

- Label `1` for an anchor `(120,70)` is emitted at `(118,64)`.
- Label color uses the yellow shell/overlay color constant.
- Labels can be built even if marker sprite atlas entry is absent.

### Task 8 - Verification

**Why:** This pass is pixel/input-sensitive and should prove both pure logic and render-instance deltas.

Steps:

1. Run `cargo fmt -v`.
2. Run the focused tests that cover:
   - `src/ui/skirmish_shell/layout.rs`
   - `src/ui/skirmish_shell/state.rs`
   - `src/app_skirmish_shell_render.rs`
3. Run `git diff --check`.
4. If Cargo is blocked by the known unrelated bridge compile issue, record the exact blocker instead of broadening this patch.
5. If a dev server or screenshot harness is already available and cheap to run, capture the Skirmish shell at 800x600/1024x768. Otherwise rely on focused unit/render-instance tests for this plan.

Acceptance:

- Formatting completes.
- Focused tests pass, or failures are only from known unrelated compile blockers.
- Diff check has no whitespace errors.

## Out Of Scope

- Choose Map modal `0x6B`; exact native listbox item height remains unresolved.
- Right-panel kind-1 reveal animation and disabled-state text color coverage.
- Full `FUN_004A61C0` glyph/color implementation for preview labels.
- Exact retail scrollbar skin pressed-state/drag-repeat polish.
- Full 72-child `0x102` matrix data model.

## Rollback / Stop Conditions

- If implementing color sentinel support requires broad enum/render/input rewiring, stop before changing color population and ask for a scope decision.
- If removing preview scissor causes markers to draw outside the render target or breaks renderer assumptions, stop and inspect the batch renderer clipping contract before layering fixes.
- If dropdown proportional top-index math cannot be tested deterministically with current helpers, add the pure helper first and test it before changing input routing.
- Do not fix unrelated bridge/build failures as part of this plan unless the user explicitly redirects.
