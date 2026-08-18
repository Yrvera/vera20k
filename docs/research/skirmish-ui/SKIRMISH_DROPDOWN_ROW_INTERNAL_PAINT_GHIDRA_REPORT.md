# Skirmish Dropdown Row Internal Paint - Ghidra Research Report

**Target:** `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT`  
**Primary addresses:** `ComboDropWin_WndProc block @ 0x0060D540`, `OwnerDraw_ListBox_00618D40 @ 0x00618D40`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`  
**Investigation mode:** exhaustive-slice for standard offline Skirmish combo dropdown row paint.  
**Scope:** open dropdown row text placement, color swatch layout, selected/highlight fill, grey/disabled behavior, row clipping, top-index offset, and scrollbar-shrunken width effects.  
**Non-scope:** combo population semantics, scrollbar drag behavior, primitive bevel color audit, online/WOL variants, and screenshot capture.

## 1. Summary

The active standard Skirmish combo popup is the `"ComboDropWin"` window class registered at `0x0060D450` with WndProc `0x0060D540`. The older shorthand that assigns open combo dropdown row paint directly to `OwnerDraw_ListBox_00618D40` is too broad: `OwnerDraw_ListBox_00618D40` is the owner-drawn `"ListBox"` callback installed by `FUN_0060F9A0`, while `ComboDropWin` has its own WndProc and paints standard combo dropdown rows from the source combo state.

Active in YR: Yes for `ComboDropWin` as the standard offline Skirmish combo popup. Evidence: `FUN_0060D450 @ 0x0060D490..0x0060D4C2` registers class string `0x008357A0` with WndProc `0x0060D540`; `OwnerDraw_ComboBox_00617250` creates `"ComboDropWin"` on `CB_SHOWDROPDOWN` at the open branch; `ComboDropWin WM_CREATE` at `0x0060E0E9..0x0060E14F` stores the source combo HWND in `DAT_00AC48D0`.

The row paint result for normal dropdowns is:

- row height comes from the source combo `CB_GETITEMHEIGHT 0x154`;
- visible rows start at the popup/source state top index `+0xF0`;
- each row's Y is `(item_index - top_index) * item_height`;
- text rect starts at `row_left + 3`, `row_top`, spans to `row_left + row_width`, `row_top + item_height`;
- text is truncated against `client_width - 20`;
- selected row fill covers the full row rect before swatch/text;
- color-combo swatches fill the row rect after a 2 px inset, not a small left-only square;
- when a scrollbar exists, the row width is the already-shrunken client width, so paint never draws under the scrollbar.

## 2. Active Path Correction

`FUN_0060F9A0` maps real `"ListBox"` controls to `OwnerDraw_ListBox_00618D40` and real `"ComboBox"` controls to `OwnerDraw_ComboBox_00617250`. It does not map `"ComboDropWin"` to `OwnerDraw_ListBox_00618D40`; `"ComboDropWin"` is registered separately by `FUN_0060D450` with WndProc `0x0060D540`.

Active in YR: Yes. Evidence: class dispatch in `FUN_0060F9A0` selects `"ListBox" -> 0x00618D40` around its class-name comparison block, `"ComboBox" -> 0x00617250` in the next block, while `FUN_0060D450 @ 0x0060D490..0x0060D4C2` registers `"ComboDropWin"` with WndProc `0x0060D540`.

`OwnerDraw_ComboBox_00617250` opens the popup by creating `"ComboDropWin"` and passing the combo HWND as `lpParam`. `ComboDropWin WM_CREATE` stores that source combo HWND in global `DAT_00AC48D0`, then posts/handles standard list-like messages against the source combo.

Active in YR: Yes. Evidence: combo open branch creates `s_ComboDropWin_008357a0`; `ComboDropWin WM_CREATE` at `0x0060E0E9..0x0060E14F` calls `SetCapture`/initialization and writes `DAT_00AC48D0`.

Implementation-facing correction: docs and code comments should not claim standard Skirmish combo dropdown rows are directly painted by `OwnerDraw_ListBox_00618D40`. The shared row-paint concepts are similar, but the active popup renderer is `0x0060D540`.

## 3. Row Iteration, Top Index, And Clipping

The popup paint path reads item count from source combo `CB_GETCOUNT 0x146` and row height from source combo `CB_GETITEMHEIGHT 0x154`. It then uses the stored top index at state `+0xF0` as the first painted item.

Active in YR: Yes. Evidence: `0x0060D846..0x0060D870` sends `0x146` and `0x154` to `DAT_00AC48D0`; `0x0060DBF3` loads `state+0xF0`; row loop starts at `0x0060DC09`.

For each row, the vertical offset is computed from `(item_index - top_index) * item_height`, and paint stops when the row bottom exceeds the current list client height. This means the normal popup paints whole visible rows; it does not attempt to draw a partial extra row after the rounded dropdown height.

Active in YR: Yes. Evidence: row offset setup at `0x0060DC09..0x0060DC64`; row-bottom/client-height break at `0x0060DC6B..0x0060DC6D`; dropdown height was already rounded to item-height multiples by `OwnerDraw_ComboBox_00617250`.

Hit testing follows the same top-index model. For real owner-drawn ListBox controls, `OwnerDraw_ListBox_00618D40` custom `0x4E8` returns `top_index + y / item_height` after bounds checks. For `ComboDropWin`, scroll messages keep the same concept in state `+0xF0`.

Active in YR: Yes for the list family; Conditional for scrollbar-shifted top indices. Evidence: `OwnerDraw_ListBox_00618D40` `0x4E8` case returns `piVar38[0x3C] + y / LB_GETITEMHEIGHT`; `ComboDropWin` row loop reads `+0xF0`.

## 4. Row Text Geometry

The text rectangle begins three pixels inside the row: `left = row_left + 3`, `top = row_top`, `right = row_left + row_width`, `bottom = row_top + item_height`.

Active in YR: Yes. Evidence: `0x0060DE1F..0x0060DE47` builds the text rect with `left + 3`, unchanged top, full row width, and `top + item_height`; draw call follows at `0x0060DFAD..0x0060DFC8`.

Before drawing, the text is converted into the shared UTF-16 scratch and measured. The truncation loop repeatedly zero-terminates one UTF-16 code unit from the end until text width is no greater than `client_width - 20`.

Active in YR: Yes. Evidence: text fetch and conversion at `0x0060DCCE..0x0060DD3B`; width limit store from `client_width - 0x14` at `0x0060DF2D..0x0060DF40`; truncation loop at `0x0060DF4E..0x0060DFA1`.

The text draw helper is the same shell bitfont path used elsewhere. It receives a rectangle and flags including vertical-centering behavior; `FUN_00621040` centers vertically when its flags include bit `4`.

Active in YR: Yes. Evidence: `ComboDropWin` text call at `0x0060DFAD..0x0060DFC8`; `FUN_00621040 @ 0x00621040` checks `param_6 & 4` before vertical centering.

## 5. Color Swatch Layout

Color combo rows use the same per-item swatch slots that collapsed combo paint uses: `state + 0x110 + item_index * 4`. The popup checks `item_index < 0x32`, non-null source state, nonnegative swatch value, and swatch-enabled byte `+0xCC` before drawing a swatch.

Active in YR: Yes for standard color combos; Conditional for other combo families because they normally have swatch mode off or no nonnegative swatch slots. Evidence: per-row swatch pointer setup at `0x0060DC1C..0x0060DC23`; guards at `0x0060DE60..0x0060DE8B`; color combo population writes those slots through combo message `0x498` per `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`.

The popup swatch is not a fixed 20x20 left chip. It starts from the full row rect `(row_left, row_top, row_width, item_height)`, calls `FUN_0072A9E0(rect, 2)`, then fills the inset rectangle with the converted swatch color. For the standard 44 px color dropdown without a scrollbar and 23 px item height, the visible fill is approximately `40x19` at `(row_left+2, row_top+2)`.

Active in YR: Yes for color combos. Evidence: swatch rect setup at `0x0060DE95..0x0060DEC5`; `FUN_0072A9E0 @ 0x0060DEC5`; DirectDraw color conversion and fill at `0x0060DED3..0x0060DF2A`.

The string is still drawn after the swatch. Normal color rows use the `"ab"` placeholder text from population, and the draw color is the current row color when a swatch value is present, so the row is visually dominated by the swatch fill rather than a readable label.

Active in YR: Yes for normal color rows. Evidence: color population inserts display text pointer `0x00822B78` and swatch data via `0x498`; popup preserves the swatch color in the text-color register before the `FUN_00621040` call at `0x0060DFAD..0x0060DFC8`.

## 6. Selection Fill And Grey Behavior

The selected row is detected by comparing the current item index with the source combo selected index stored at state `+0xE8`. When it matches, the popup fills the full row rect before swatch/text drawing.

Active in YR: Yes. Evidence: selected index loaded from `state+0xE8` at `0x0060D8A0`; row comparison at `0x0060DD42..0x0060DD48`; selected fill rectangle and surface fill call at `0x0060DD4E..0x0060DE0A`.

Normal selected fill uses converted `DAT_00AC4604`. If the combo grey byte `+0xCD` is set, selected fill uses `DAT_00AC4880` instead. The same grey byte also changes the frame/background color and text color path.

Active in YR: Conditional. The path is active when helpers send `0x4F1=1`, such as restricted/grey color rows. Evidence: grey byte check at `0x0060DD72..0x0060DDA6` for selected fill color, grey frame color at `0x0060DB1C..0x0060DB81`, and grey text color at `0x0060DE31..0x0060DE60`; restricted color helper evidence in `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`.

Normal text color falls back to `DAT_00AC18A4` unless grey mode is set, in which case it uses `DAT_00AC1CB0`. If a valid swatch is drawn, the row color value replaces that text color for the subsequent text call.

Active in YR: Yes for normal text; Conditional for grey/swatch cases. Evidence: normal/grey text color setup at `0x0060DE1F..0x0060DE60`; swatch color overwrite and draw at `0x0060DE81..0x0060DF2A`.

## 7. Scrollbar-Shrunken Width

The popup/list scrollbar code computes scrollbar width as `DAT_00AC1DF0 * 2 + 0x12`, with `DAT_00AC1DF0 = 1`, so standard width is `20` px. When scrolling is required, the popup/list client is resized before paint, subtracting this width.

Active in YR: Conditional. Side/country combos are the standard Skirmish family most likely to exceed visible capacity. Evidence: setup value in `FUN_0060F9A0`; ComboDropWin scrollbar sizing and range setup around `0x0060D759..0x0060D802`; sibling `OwnerDraw_ListBox_00618D40` scrollbar creation/resizing around `0x0061BFD0..0x0061C45D`.

Row paint uses the current client width after that resize. Therefore text width, selected fill width, and swatch fill width all shrink with the list client and do not draw under the scrollbar. A scrolling 117 px side dropdown paints roughly 97 px rows; standard 44 px color dropdowns do not scroll because their normal row cap is 9 and they normally have at most 9 rows.

Active in YR: Yes for width consumption; Conditional for scroll presence. Evidence: current client rectangle derived before paint at `0x0060D575..0x0060D5A8`; paint width values consumed at `0x0060D846..0x0060D89C`; combo family caps from `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`.

## 8. Implementation Handoff

- Verified behavior: standard combo popups are `ComboDropWin` row paint, not a native or egui dropdown -> Rust delta: model an open popup with source-combo state and top index -> affected surface: `src/ui/skirmish_shell/*` combo/dropdown rendering and hit testing -> acceptance scenario: opening side/color/start/team combos shows a row-height-multiple popup below the collapsed combo with row paint driven by source items -> risk: using generic immediate dropdown widgets will miss arrow-only open, top-index, and row clipping.
- Verified behavior: color dropdown rows fill almost the full row with a 2 px inset swatch -> Rust delta: draw row swatches as full-row inset fills when swatch mode and per-item swatch are valid -> affected surface: Skirmish color combo renderer -> acceptance scenario: color dropdown rows at 800x600 are swatch-dominant, not left-chip text rows -> risk: using the collapsed 20x20 swatch geometry for open rows gives visibly wrong dropdowns.
- Verified behavior: selected fill and grey state use owner-draw colors before swatch/text -> Rust delta: keep selected index, grey byte, and optional swatch as separate row paint inputs -> affected surface: combo state/render structs -> acceptance scenario: restricted grey color combo and selected dropdown row use the right fill/text/swatch order -> risk: disabled rows may look active or selected colors may obscure the wrong area.

## 9. Stale-Doc Notes

- `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md` has been corrected to state that `ComboDropWin` is registered with WndProc `0x0060D540` and owns standard combo popup row paint; `OwnerDraw_ListBox_00618D40` owns real owner-drawn ListBox controls and shares related list/scrollbar concepts.
- `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md` has been corrected to state that standard `ComboDropWin` combo popup row paint is in the registered `ComboDropWin` WndProc at `0x0060D540`; `OwnerDraw_ListBox_00618D40` remains the real owner-drawn ListBox callback.

## 10. Open Questions

[DEFERRED] Screenshot-level validation of exact converted RGB colors for selected/grey fills. Static binary evidence identifies the source globals and conversion path, but not the final 16-bit palette presentation under every display mode.

[DEFERRED] Whether any nonstandard Skirmish-mod combo can force a scrolling color dropdown wider/narrower than the retail cap. Standard offline color combos do not scroll.

## Sources

- Ghidra read-only decompile: `OwnerDraw_ComboBox_00617250 @ 0x00617250`.
- Ghidra read-only decompile: `OwnerDraw_ListBox_00618D40 @ 0x00618D40`.
- Ghidra read-only decompile: `FUN_0060F9A0 @ 0x0060F9A0`.
- Ghidra/disassembly evidence from retail `gamemd.exe`: `FUN_0060D450 @ 0x0060D450`, registered WndProc block `0x0060D540`.
- Ghidra read-only decompile: `FUN_00621040 @ 0x00621040`.
- Prior docs cross-checked: `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`.
