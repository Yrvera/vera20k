# Skirmish ComboDropWin Row Text Truncation And Paint Order - Ghidra Research Report

**Target:** `SKIRMISH_COMBODROPWIN_ROW_TEXT_TRUNCATION_AND_PAINT_ORDER`  
**Investigation mode:** exhaustive-slice.  
**Scope:** standard Skirmish combo dropdown rows owned by `ComboDropWin`: row text left inset, row text width limit, binary pre-truncation versus render clipping/wrapping, selected fill coverage, scrollbar exclusion, and native paint ordering relative to parent controls.  
**Non-scope:** Choose Map/listbox controls, combo population semantics except where item count/top index affects row paint, scrollbar repeat timing, final RGB screenshot capture, and Rust implementation.  
**Primary evidence:** `FUN_0060D450 @ 0x0060D450`, registered `ComboDropWin` WndProc block `0x0060D540..0x0060F311`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`, text helper `FUN_00621040 @ 0x00621040`, and current Rust read-only scan.

## Working Notes

Target question: Does native YR pre-truncate ComboDropWin row text before drawing, what exact row bounds/order does it use, and can parent combo-face text paint over an open dropdown?

Non-goals: Do not investigate real `LISTBOX` controls except to avoid confusing them with `ComboDropWin`; do not re-audit full scrollbar drag timing, Choose Map listboxes, color population, or final display RGB.

Evidence needed to mark COMPLETE: proof that standard Skirmish reaches `ComboDropWin`; decompile or assembly for row text rect, width limit, truncation loop, selected fill, scrollbar shrink, hit-test/top-index, and popup creation/paint ownership; current Rust comparison; open questions drained or explicitly deferred.

Stop conditions: stop after row text truncation/paint order is proven for standard combo popups, one report is written, no Ghidra mutations are made, and any broader combo/listbox questions are listed as open or stale-doc wording.

## Summary

Standard Skirmish combo dropdown rows are painted by the registered `"ComboDropWin"` window procedure, not by the real owner-drawn `LISTBOX` callback. The row paint path draws the selected fill first across the full content row, then optional full-row-inset color swatch, then row text. Row text starts at `content.x + 3`, is vertically centered through `FUN_00621040` flag `0x04`, and is **binary pre-truncated** before drawing: YR repeatedly removes one UTF-16 code unit from the end until `BitFont__GetTextWidth <= current_client_width - 0x14`.

Current Rust has the broad geometry correct after recent dropdown work, including parent overlay suppression. The remaining handoff-critical mismatch risk is text handling: `src/app_skirmish_shell_render/text.rs` gives `shell_text::draw_in_rect` a `content.w - 20` width, and that renderer uses `BitFont::wrap_layout`. That is not the same contract as gamemd's single-line pre-truncation loop for long row labels.

## Verified Binary Findings

### 1. Active row owner is `ComboDropWin`, not the listbox callback

Active in YR: Yes for standard offline Skirmish combo popups. Evidence: `FUN_0060D450` decompile writes `local_28.lpfnWndProc = &LAB_0060d540` and registers class string `s_ComboDropWin_008357a0`; `OwnerDraw_ComboBox_00617250` decompile opens the dropdown with `CreateWindowExA(..., s_ComboDropWin_008357a0, ..., 0x40000000, ..., GetParent(combo), ..., combo_hwnd)`, then `SetCapture` and `ShowWindow`.

Implementation implication: standard side/color/start/team/AI dropdown rows must be modeled as custom combo popup rows, not as Choose Map or real `LISTBOX` rows.

### 2. Row text rect uses `left + 3`, row top/bottom, and vertical-centering flag only

Active in YR: Yes while a combo popup is visible. Evidence: assembly context `0x0060DE1F..0x0060DE47` loads normal text color from `DAT_00AC18A4`, computes `LEA EDX,[EAX + 0x3]`, stores that as rect left, stores row top, right, and bottom as row geometry; `0x0060DFAD..0x0060DFC8` pushes flags `0x4` and calls `FUN_00621040`. `FUN_00621040` decompile checks `param_6 & 4` and vertically centers by measuring text height, then calls the glyph draw routine.

Tiny detail: the draw rect right remains the current row/content right; the `-0x14` scrollbar reserve is used as the truncation width, not as the stored draw rect right in the native path.

### 3. Native row text is pre-truncated one UTF-16 code unit at a time

Active in YR: Yes for all standard ComboDropWin row text. Evidence: assembly `0x0060DF2D..0x0060DF40` stores `client_width - 0x14` as the width limit; `0x0060DF4E..0x0060DF5A` measures the scratch string; `0x0060DF6A` compares measured width to the limit; if too wide, `0x0060DF70..0x0060DF86` backs the UTF-16 pointer up by 2 bytes, decrements the length, writes a zero word, converts/refreshes the string, and `0x0060DF9A..0x0060DFA1` loops until it fits or length reaches zero.

Negative proof: this is not a wrapped-text draw. The binary mutates the scratch string before the draw call, so only the shortened single row string reaches `FUN_00621040`.

### 4. Selected row fill covers the full content row before swatch/text

Active in YR: Yes for the selected/hot row. Evidence: `0x0060DD42..0x0060DD48` compares current iterated item with selected/hot item; `0x0060DD4E..0x0060DE0A` builds the row rect and calls the surface fill. That block executes before the swatch block (`0x0060DE60..0x0060DF2A`) and before text draw (`0x0060DFAD..0x0060DFC8`).

Implementation implication: no inset selected highlight, no text-before-fill ordering, and no parent combo-face label may be composited after this popup text in the same render layer.

### 5. Scrollbar exclusion is built into content width and text fit

Active in YR: Conditional on overflowing dropdowns; Yes for the content contract when overflow exists. Evidence: prior `ComboDropWin` boundary report proves child scrollbar creation in `0x0060E648..0x0060E821`; this pass rechecked text fit at `0x0060DF2D..0x0060DF40`, where the current client/content width is reduced by `0x14`. Custom hit-test assembly at `0x0060F297..0x0060F307` rejects points outside `[0,width) x [0,height)`, reads item height via `CB_GETITEMHEIGHT 0x154`, gets item count via `CB_GETCOUNT 0x146`, then computes `top_index + y / item_height` with a cap to the last item (`0x0060F2C2..0x0060F2FD`).

Implementation implication: when a scrollbar is visible, selected fill, swatches, text fit, and hit-test should use content width excluding the 20 px scrollbar column.

### 6. Native parent text-over-popup artifact is not a ComboDropWin behavior

Active in YR: No as a native row paint behavior; the Rust artifact was implementation-specific. Evidence: popup creation is a separate `ComboDropWin` child window via `CreateWindowExA` in `OwnerDraw_ComboBox_00617250`, with its own WndProc `0x0060D540`, capture, and `ShowWindow`; row paint is inside the popup WndProc block `0x0060DD42..0x0060DFC8`. The row paint path does not call source-combo collapsed label drawing after popup rows. Parent/source controls repaint through their own child window callbacks, not a single batched text pass after popup paint.

Implementation implication: Rust's overlay suppression for parent combo-face labels is the right architecture fix for the current renderer; do not model this as a retail behavior.

## Current Rust Comparison

Current Rust matches or mostly matches:

- `combo_dropdown_content_rect` subtracts `COMBO_DROPDOWN_SCROLLBAR_W` when needed (`src/ui/skirmish_shell/state/combos.rs:122`).
- selected row fill uses the content row, not an inset row (`src/app_skirmish_shell_render/controls.rs:479`).
- color rows draw the full content row inset by 2 px (`src/app_skirmish_shell_render/controls.rs:496`).
- popup row labels are emitted after combo-face labels, and recent overlay suppression prevents parent labels from drawing over dropdown/validation overlays (`src/app_skirmish_shell_render/text.rs:510`, `:542`).

Current Rust mismatch/risk:

- `combo_dropdown_text_rect_for_current_renderer` sets `x = content.x + 3` and `w = content.w - 20`, but `shell_text::draw_in_rect` calls `font.wrap_layout(text, rect.w)` (`src/render/shell_text.rs:78`). For long labels, this can wrap or clip a full string rather than pre-shortening the UTF-16 string to the largest width-fitting prefix before draw. Standard stock labels usually fit, so the risk is most visible with localization, modded CSF, or unusually long map/player strings if they enter standard combo rows.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| ComboDropWin pre-truncates row text by repeatedly deleting one UTF-16 code unit until measured width <= `content_width - 20` | Add a no-wrap/pre-truncate path for dropdown row text before calling the shell text renderer, or add a text renderer mode that exactly implements this single-line truncation contract | `src/app_skirmish_shell_render/text.rs`, `src/render/shell_text.rs`, bitfont measurement helper | A long side dropdown label with spaces and a long unbroken token produces one shortened row string; no second line is emitted and no glyph appears under the scrollbar | `skirmish_combodropwin_row_text_pretruncates_without_wrapping` | Medium: stock labels may hide the bug, but localized/modded strings expose it |
| Selected fill covers the full content row before swatch/text, and scrollbar content width is excluded | Preserve current full-row selected fill and content shrink tests | `src/app_skirmish_shell_render/controls.rs`, `src/ui/skirmish_shell/state/combos.rs` | Open an overflowing side dropdown; selected fill spans only the content area, row text and swatch stop before scrollbar | `skirmish_combodropwin_selected_fill_and_text_exclude_scrollbar_column` | Low if current code stays unchanged |
| Parent/source combo text is never a native row-paint layer over an open popup | Keep overlay suppression in the Rust batched renderer; treat it as renderer architecture, not retail popup logic | `src/app_skirmish_shell_render/text.rs` | Open a dropdown over combo faces; underlying closed combo labels are absent inside the popup rect while unrelated labels outside remain visible | `skirmish_combodropwin_overlay_suppresses_parent_combo_face_text_only` | Low after the recent fix |

## Negative Facts / Do Not Do

- Do not route standard combo dropdown rows through `OwnerDraw_ListBox_00618D40`; Active in YR: No for this popup. Evidence: `ComboDropWin` registration at `0x0060D450` and row paint in `0x0060D540` block.
- Do not implement dropdown row text as normal wrapping text. Active in YR: No. Evidence: the `0x0060DF6A..0x0060DFA1` loop zero-terminates the UTF-16 scratch string before draw.
- Do not draw row text under the scrollbar. Active in YR: No for overflow popups. Evidence: text fit width subtracts `0x14`, and popup content width is shrunken when scrollbar exists.
- Do not draw selected rows as inset highlights. Active in YR: No. Evidence: selected fill block builds and fills the full row rect at `0x0060DD4E..0x0060DE0A`.
- Do not treat parent-text-over-popup as native behavior. Active in YR: No for ComboDropWin row paint. Evidence: popup is a separate WndProc/window and no parent text draw is called after row text in the row paint block.

## Open Questions

- None for the scoped row text truncation and paint order contract.
- Deferred outside this slice: exact final RGB of popup background/selected fill under retail display format, and child scrollbar repeat timing.

## Stale-Doc Replacement Wording

Exact path: `docs/research/skirmish-ui/SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`

Replace any wording equivalent to "Rust clipping is sufficient for long dropdown labels" with:

> ComboDropWin row text is not merely clipped or wrapped. The binary measures the UTF-16 scratch string against `current_client_width - 0x14` and zero-terminates one UTF-16 code unit at a time until the measured width fits, then draws the shortened single-line string with `FUN_00621040` flag `0x04`. Rust must use an equivalent no-wrap pre-truncation path for exact long-label parity.

Exact path: `docs/research/skirmish-ui/SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md`

Replace any wording that groups dropdown row text with ordinary clipped shell text with:

> Standard `ComboDropWin` popup row labels are a special caller contract: left inset `+3`, row-height rect, vertical-center flag `0x04`, and caller-side UTF-16 pre-truncation to `client_width - 0x14` before `FUN_00621040`. Treat dropdown rows as no-wrap, pre-truncated labels, not generic wrapped shell text.

## Sources

- Ghidra read-only decompile: `FUN_0060D450 @ 0x0060D450`.
- Ghidra read-only decompile: `OwnerDraw_ComboBox_00617250 @ 0x00617250`.
- Ghidra read-only decompile: `FUN_00621040 @ 0x00621040`.
- Ghidra read-only assembly contexts: `0x0060DD42`, `0x0060DE1F`, `0x0060DF2D`, `0x0060DF4E`, `0x0060DF6A`, `0x0060DF70`, `0x0060DF9A`, `0x0060DFAD`, `0x0060F297`, `0x0060F2C2`, `0x0060F2E7`.
- Prior docs checked: `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_COMBODROPWIN_0060D540_FUNCTION_BOUNDARY_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/app_skirmish_shell_render/text.rs`, `src/app_skirmish_shell_render/controls.rs`, `src/ui/skirmish_shell/state/combos.rs`, `src/render/shell_text.rs`.

**Status:** COMPLETE.
