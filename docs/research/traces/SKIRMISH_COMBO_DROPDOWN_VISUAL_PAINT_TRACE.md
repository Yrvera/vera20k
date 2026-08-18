# Skirmish Combo Dropdown Visual Paint Trace

**Scenario:** Standard offline YR Skirmish setup at 800x600; open side, color, start, and team combo dropdowns and trace collapsed face, arrow, open popup row paint, swatches, scrollbar, z-order, and clipping.

**Status:** COMPLETE. This trace is read-only against Rust and binary evidence; only this report file was written.

## Pipeline

`Mouse down on rightmost 20 px arrow` -> `SkirmishShellState.open_combo_dropdown` -> `combo_dropdown_rect/content/scrollbar geometry` -> `push_combo_instances collapsed face/arrow/swatch` -> `push_dropdown_instances popup background/selection/swatch/scrollbar/border` -> `collect_skirmish_shell_text_draws dropdown text` -> `screen over shell controls`

## Verdict Summary

PASS: 7 | FAIL: 6 | UNCHECKED: 4 | NOT-IMPLEMENTED: 0

## Stage Results

### Stage 1 - Collapsed Face Size And Combo Widths

Our 800x600 row-0 rectangles are side `(287,59,117,120)`, color `(423,59,44,119)`, start `(486,59,38,119)`, and team `(546,59,38,119)` from `src/ui/skirmish_shell/layout.rs`. `combo_face_rect` paints height `24`.

gamemd uses the same 117/44/38 widths and fixed 24 px owner-draw face height for these families. Evidence: `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, sections 2-4, active in standard YR via dialog `0x102` and `OwnerDraw_ComboBox_00617250`.

Verdict: PASS.

### Stage 2 - Collapsed Arrow Geometry And Asset Choice

Our arrow rect top-left is `rect.x + rect.w - 19, rect.y + 1`; for the color combo that is `(448,60)`. Arrow assets are `dnarrowr.pcx`, `dnarrowp.pcx`, `gdnarrowr.pcx`, `gdnarrowp.pcx` in `src/render/skirmish_shell_chrome.rs`.

gamemd uses arrow reserve `20`, arrow top-left `client_width - 19, y=1`, and the same down-arrow PCX names for normal/pressed/grey states. Evidence: `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, sections 3 and 5.

Verdict: PASS.

### Stage 3 - Collapsed Color Swatch Geometry

Our collapsed color swatch for the 44 px combo is `(425,61,20,20)` from `combo_swatch_rect`.

gamemd fills the non-arrow face after a 2 px inset, giving `(2,2,20,20)` relative to the 44 px combo. Evidence: `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, sections 4 and 7.

Verdict: PASS for geometry.

### Stage 4 - Collapsed Color Text

Our `combo_item_label` returns an empty string for color items and the shell does not draw color combo text over the swatch.

gamemd draws the selected item text after swatch fill. Normal color rows are populated with the `"ab"` display text pointer, even though the swatch dominates the control. Evidence: `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, section 7; `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`, section 4.

Player-visible difference: color combos are missing the tiny retail text overlay on top of the swatch.

Verdict: FAIL.

### Stage 5 - Arrow-Only Open Hit Zone

Our `arrow_hit_rect` opens only the rightmost 20 px of the 24 px face, and body clicks do not open the combo.

gamemd toggles dropdown only when mouse X is greater than `client_width - 20`. Evidence: `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`, section 4.

Verdict: PASS.

### Stage 6 - Dropdown Position, Width, Row Height, And Height Rounding

Our dropdown top is `face.y + 24 + 1`, width equals combo width, and row height is `23`. For row 0 at 800x600:

- side dropdown: `(287,84,117,161)` because cap `7 * 23`;
- color dropdown: `(423,84,44,184)` because current item count is 8;
- start dropdown: `(486,84,38,...)` depending on map start count, cap 9;
- team dropdown: `(546,84,38,115)` for the standard 5 rows.

gamemd creates `ComboDropWin` one pixel below the collapsed face, width equal to combo client width, row height `GAME.FNT cell height 17 + 6 = 23`, and final height rounded to a whole row multiple. Evidence: `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`, sections 3, 5, and 6.

Verdict: PASS for placement/width/row-height formula. Color height fails separately because row population is wrong.

### Stage 7 - Color Dropdown Row Count And First Row

Our color dropdown items are only `0..HOUSE_COLOR_COUNT`, so 8 rows and no sentinel row.

gamemd normal color population inserts a `-2` sentinel row first, then color rows `0..7`, sets max rows to 9, and therefore opens a 9-row, 207 px high color popup in the normal full-color case. Evidence: `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`, sections 3-4; active via `FUN_006AE6E0 -> FUN_004E43C0 -> FUN_004E4820`.

Player-visible difference: color dropdown is one row too short and lacks the first retail sentinel row.

Verdict: FAIL.

### Stage 8 - Open Color Row Swatch Paint

Our open color rows fill `(content.x + 2, row_y + 2, content.w - 4, 19)`. For the 44 px dropdown without scrollbar, that is `40x19`.

gamemd fills the full row rect after a 2 px inset; for the 44 px color dropdown and 23 px row, that is also `40x19`. Evidence: `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, section 5.

Verdict: PASS for populated color rows.

### Stage 9 - Selected Row Fill Rectangle

Our selected fill rect is inset by 1 px on all sides: `(content.x + 1, content.y + 1 + row*23, content.w - 2, 21)`.

gamemd fills the full row rect before swatch/text drawing. Evidence: `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, section 6.

Player-visible difference: selected row highlight has a one-pixel hollow border/gap that retail does not have.

Verdict: FAIL.

### Stage 10 - Dropdown Text Left And Width Handling

Our dropdown text starts at `content.x + 3`, matching gamemd's left inset. Our text rect width is `content.w - 3`, and `render/shell_text.rs::draw_in_rect` wraps text to the rect width and clips by scissor.

gamemd text starts at `row_left + 3` but truncates text against `client_width - 20` by repeatedly removing UTF-16 code units before drawing; it does not wrap the row label. Evidence: `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, section 4.

Player-visible difference: long side names can wrap or clip differently and can consume the right arrow-reserve margin gamemd uses as truncation budget.

Verdict: FAIL.

### Stage 11 - Dropdown Color Row Text

Our color dropdown labels are empty, so no `"ab"` text is drawn after the color row swatch.

gamemd still draws the row string after swatch fill for color rows. Normal rows use the `"ab"` placeholder and draw with the row color value. Evidence: `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, section 5; `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`, section 4.

Player-visible difference: open color rows are pure swatches, missing the retail overpainted placeholder text.

Verdict: FAIL.

### Stage 12 - Scrollbar Width And Content Shrink

Our scrollbar width is 20 px, and when scrolling is needed `combo_dropdown_content_rect` subtracts 20 px from row content width.

gamemd standard owner-draw scrollbar width is 20 px (`DAT_00AC1DF0 * 2 + 0x12` with `DAT_00AC1DF0 = 1`) and the list client is shrunk before row paint. Evidence: `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`, section 8; `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, section 7.

Verdict: PASS.

### Stage 13 - Scrollbar Arrow Button And Thumb Paint

Our scrollbar uses `uparrowr.pcx`, `dnarrowr.pcx`, `sbgript.pcx`, `sbgripm.pcx`, and `sbgripb.pcx`, but uses hardcoded 22 px button zones and current thumb math.

gamemd evidence verifies owner-draw scrollbar creation and 20 px width. This trace did not compute gamemd's exact up/down button rectangle height, thumb top/bottom segment placement, or converted track colors against our values.

Verdict: UNCHECKED.

### Stage 14 - Dropdown Z-Order

Our instance push order draws collapsed controls first, flags next, then dropdown popup instances after flags and controls. Dropdown text is emitted after collapsed shell text in the text collection path.

gamemd creates `ComboDropWin`, shows it, captures input, and brings scrollbar child to top when present. Evidence: `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`, sections 4 and 8.

The popup is intended to sit over shell controls. Exact cross-pass equality between our sprite/text depth values and gamemd HWND/surface paint order was not computed.

Verdict: UNCHECKED.

### Stage 15 - Side Dropdown Row Content

Our side dropdown uses `Random` plus 10 countries in `[Countries]` order. At 800x600 it caps to 7 visible rows and scrolls.

gamemd standard side population inserts Random plus eligible multiplayer houses, and the verified YR `[Countries]` order is `Americans, Alliance, French, Germans, British, Africans, Arabs, Confederation, Russians, YuriCountry`. Evidence: `SKIRMISH_FLAG_PCX_PALETTE_AND_NATIVE_CLIP_GHIDRA_REPORT.md`, section 2; `ini/rulesmd.ini` `[Countries]`.

Text localization, exact CSF strings, and pixel widths were not measured against gamemd for each visible side row.

Verdict: UNCHECKED.

### Stage 16 - Start And Team Dropdown Caps

Our start combos cap at 9 rows. Our team combos have no explicit max cap but standard team population is 5 rows, so the 800x600 standard visible height still lands at 115 px.

gamemd sets max rows to 9 for start and team helpers, and standard team rows fit without scroll. Evidence: `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`, section 7.

Player-visible difference in this exact standard scenario: none computed for team because 5 rows fit under both models. Latent mismatch: nonstandard team populations would need the 9-row cap.

Verdict: PASS for standard team height; UNCHECKED for nonstandard overflow.

## Failures

1. **Color dropdown missing sentinel row** - `src/ui/skirmish_shell/state.rs:754` returns only color rows `0..7`; gamemd inserts sentinel `-2` then rows `0..7`. Color popup is 184 px instead of 207 px in the normal full-color case.
2. **Selected row fill is inset** - `src/app_skirmish_shell_render.rs:930` builds `content.x + 1`, `content.w - 2`, `COMBO_DROPDOWN_ROW_H - 2`; gamemd fills the full row rect.
3. **Dropdown text wraps/clips instead of truncating to `client_width - 20`** - `src/app_skirmish_shell_render.rs:1625` uses `content.w - 3`, then `src/render/shell_text.rs:76` wraps layout; gamemd truncates the source string to `client_width - 20`.
4. **Open color rows omit retail `"ab"` text overlay** - `src/app_skirmish_shell_render.rs:1278` returns an empty color label; gamemd draws the item string after swatch fill.
5. **Collapsed color combo omits retail color item text overlay** - same empty color label path; gamemd collapsed owner-draw also draws selected item text after the swatch.
6. **Team max-row cap is not modeled** - `src/ui/skirmish_shell/state.rs:551` gives Team max rows `0`; gamemd sends max rows `9`. No visible difference in standard 5-row team dropdown, but the model is not retail-complete.

## Adjacent Findings

- Side flags are paired statics, not dropdown-row art. This scenario did not find evidence that standard side dropdown rows draw flag PCXs inside the open popup.
- Exact scrollbar button heights, thumb segment heights, pressed scrollbar states, and converted owner-draw colors need a narrower scrollbar visual trace before they can be marked PASS or FAIL.
- Exact CSF/localized string pixel widths for side/start/team rows were not measured. Geometry is verified; text bitmap equality is still open.

## Sources

- `docs/research/skirmish-ui/SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_FLAG_PCX_PALETTE_AND_NATIVE_CLIP_GHIDRA_REPORT.md`
- Rust inspected: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/render/shell_text.rs`
