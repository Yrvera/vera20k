# Skirmish ComboDropWin Dropdown Visual/Input Recheck - Ghidra Research Report

**Address(es):** `FUN_0060d450 @ 0x0060D450`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `ComboDropWin WndProc block @ 0x0060D540..0x0060F311`, `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`, `FUN_004e45a0 @ 0x004E45A0`, `FUN_004e4770 @ 0x004E4770`, `FUN_00621040 @ 0x00621040`, `FUN_00620720 @ 0x00620720`, `FUN_0060f9a0 @ 0x0060F9A0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** current standard offline Skirmish `0x102` combo dropdown visual/input parity recheck, especially side combo `ComboDropWin` selected-row fill, row text clipping/truncation, scrollbar pressed arrow/grip state, proportional track-click/top-index math, normal color row-8 omission, and direct mouse-wheel handling.
**Non-Scope:** real `LISTBOX`/Choose Map `0x6B` row behavior except contrast, full single-function recovery of `0x0060D540`, parent shell invalidation order after close, final screenshot RGB capture, and Rust implementation.
**Confidence:** High for the binary visual/input facts rechecked here and for current Rust source status; Medium for exact scrollbar thumb height rounding because the binary uses a floating conversion while this report did not reconstruct the exact FPU inputs beyond the existing decompile.
**Active in YR:** Yes for standard `0x102` combos and `ComboDropWin`; Conditional for scrollbar behavior when a dropdown overflows; No direct scoped callback path found for wheel input.

## 0. Working Notes

Target question: Does current Rust now match the YR `ComboDropWin` dropdown visual/input contract for standard Skirmish combos, and which deltas remain?

Non-goals: Do not investigate real `LISTBOX`/`0x6B` rows beyond distinguishing them from `ComboDropWin`; do not re-cover already settled popup placement/row caps except when needed to judge current Rust; do not edit Rust.

Evidence needed to mark COMPLETE: Prior-doc gap scan, current Rust scan, decompile plus assembly evidence for `ComboDropWin` registration/paint/hit-test/top-index, decompile evidence for scrollbar input/pressed state and no direct wheel handling, decompile evidence for normal color population, and a Rust-facing handoff with stale-doc replacement wording.

Stop conditions: Ghidra mutation would be required; investigation expands into Choose Map listboxes or full shell invalidation; no new material open questions remain except explicitly deferred screenshot/runtime items.

## 1. Overview

The earlier `ComboDropWin` binary findings still hold: standard Skirmish combo dropdown rows are painted and hit-tested by the registered `ComboDropWin` popup block, not by the real owner-drawn `LISTBOX` row callback. The binary paints selected popup rows as full content rows, truncates row text against the current client width minus `0x14`, shrinks the row client when the scrollbar exists, uses proportional scrollbar current/top-index math, inserts only normal color rows `0..7`, and has no direct `WM_MOUSEWHEEL (0x20A)` handler in the scoped combo/popup/scrollbar callbacks.

Current Rust has closed several stale gaps: selected row fill is now full-row, popup text clipping has a dedicated `client_width - 20` test, scrollbar arrow pressed art is wired, track clicks jump proportionally, and normal color row 8 is omitted. The remaining high-value delta found in this recheck is Rust's direct `handle_option_mouse_wheel` dropdown scrolling, which has no scoped binary callback evidence.

## 2. Key State / Constants

| Field / constant | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| `"ComboDropWin"` WndProc | Registered with WndProc `LAB_0060d540`; created by the combo open path | decompile `FUN_0060d450`; decompile `OwnerDraw_ComboBox_00617250` create path | Yes |
| source combo row height | Popup queries `CB_GETITEMHEIGHT (0x154)` from the source combo; standard Skirmish rows are 23 px from prior combo-init evidence | decompile `OwnerDraw_ComboBox_00617250`; assembly `0x0060F2C2..0x0060F2E7` | Yes |
| scrollbar width reserve | Scrollbar width is `DAT_00AC1DF0 * 2 + 0x12`; setup initializes `DAT_00AC1DF0 = 1`, so standard width is `20` px | decompile `FUN_0060f9a0`; prior/listbox geometry reports | Conditional on overflow |
| row text inset | Popup row text starts at `row_left + 3` | assembly `0x0060DE1F..0x0060DE47` | Yes |
| text fit reserve | Popup text fit subtracts `0x14` from current client width before truncation | assembly `0x0060DF2D..0x0060DF3C` | Yes |
| selected row fill | Current item index equals hot/selected row, then fills the full row rect before swatch/text | assembly `0x0060DD42..0x0060DE0A` | Yes |
| scrollbar arrows/thumb | Arrow buttons are `0x16` px; min thumb is `0x0E`; pressed arrow flags are stored and used for arrow PCX state | decompile `OwnerDraw_ScrollBar_0061C690`, contexts `0x0061D383..0x0061D3B8`, `0x0061D522..0x0061D548` | Conditional on overflow |
| normal color rows | Sentinel `-2` plus color rows whose table owner is slot or `-1`; loop stops before row 8 table tail | decompile `FUN_004e45a0` loop `DAT_008B4040` while pointer `< 0x008B40A0` | Yes |

## 3. Core Logic

### 3.1 ComboDropWin vs. Real LISTBOX

Active in YR: Yes. `FUN_0060d450` registers class name `"ComboDropWin"` with WndProc `LAB_0060d540`, and `OwnerDraw_ComboBox_00617250` creates that class when `CB_SHOWDROPDOWN (0x14F)` opens a standard combo. `FUN_0060f9a0` separately maps class `"ListBox"` to `OwnerDraw_ListBox_00618D40`, which is the real listbox path and not the standard combo popup row painter.

This recheck found no contradiction with the existing reports: `0x6B` Choose Map rows should stay documented as real listbox behavior, while side/color/start/team/AI combo popup rows should stay documented as `ComboDropWin`.

### 3.2 Selected Row Fill

Active in YR: Yes. In the `ComboDropWin` row paint range, the code compares the current row against the popup selected/hot row, builds the row rect from current row left/top/right/bottom, picks the normal or grey selected fill global, converts it through display-format masks/shifts, and calls the surface fill before swatch/text. Assembly context `0x0060DD42..0x0060DE0A` shows no inset arithmetic between the row rect setup and fill call.

Current Rust status: implemented. `src/app_skirmish_shell_render.rs:1110` builds `dropdown_selected_row_rect` as `content.x`, row Y, `content.w`, `COMBO_DROPDOWN_ROW_H`; `src/app_skirmish_shell_render.rs:2715` has `skirmish_dropdown_selected_row_fill_is_full_row`.

### 3.3 Text Clipping / Truncation

Active in YR: Yes. Popup row text rect starts at `row_left + 3`, uses the row top and row bottom, and then truncates the text until `BitFont__GetTextWidth <= current_client_width - 0x14`. The draw wrapper `FUN_00621040` vertically centers when flag `0x04` is set. Evidence: assembly `0x0060DE1F..0x0060DFC8`; decompile `FUN_00621040`.

Current Rust status: mostly implemented for the renderer contract. `src/app_skirmish_shell_render.rs:2738` tests `skirmish_dropdown_side_text_clip_uses_combodropwin_client_width_minus_20`. Runtime text rendering should continue to keep draw rect and truncation width separate: draw from `x+3`, but do not draw under the 20 px scrollbar reserve.

### 3.4 Scrollbar Pressed State and Proportional Top Index

Active in YR: Conditional. It applies when the dropdown overflows, normally side/country. `ComboDropWin` creates a child `"Scrollbar"` with style `0x50010001`, calls `FUN_0060f9a0`, syncs range/current through custom messages, and shrinks row content by the scrollbar column. Evidence: assembly context `0x0060E721..0x0060E745`; decompile `OwnerDraw_ScrollBar_0061C690`.

The scrollbar callback uses `0x16` px arrow zones. On `WM_LBUTTONDOWN`, it sets capture/timer, sets up/down pressed flags when the pointer is in an arrow zone, decrements/increments current by one row if possible, and invalidates/sends parent `WM_VSCROLL (0x115)` when current changes. Track clicks outside the thumb compute a new current by centering the thumb on the click, clamping into the track, and converting back through `((thumb_top - 0x16) * range) / track_span`; this is not a page-step by visible row count. Evidence: decompile `OwnerDraw_ScrollBar_0061C690`; assembly contexts `0x0061D383..0x0061D3B8`, `0x0061D4EB..0x0061D522`, `0x0061D522..0x0061D548`.

Current Rust status: substantially implemented. `src/app_skirmish_shell_render.rs:647` chooses pressed arrow art based on `dropdown_scroll_press`; `src/ui/skirmish_shell/state.rs:896` maps track clicks by centering the thumb on the click; `src/ui/skirmish_shell/state.rs:2227` and following tests cover arrow/drag/track behaviors. Remaining risk: exact native floating thumb-height rounding and repeat-timer feel are not proven by Rust tests in this report.

### 3.5 Color Row 8 Omission

Active in YR: Yes for normal color combos. `FUN_004e45a0` hides/resets the control, sends `0x4DD=1` and `0x4DE=9`, inserts sentinel item data `-2`, then loops a 3-dword color table from `DAT_008B4040` while the pointer is `< 0x008B40A0`. That loop inserts rows `0..7` when the owner is the current slot or `-1`; initialized row 8 is not inserted by this normal loop. Restricted color state is separate: `FUN_004e4770` inserts one grey sentinel row and sets `0x4F1=1`.

Current Rust status: implemented. `src/ui/skirmish_shell/state.rs:2455` has `skirmish_color_dropdown_normal_population_omits_initialized_row_8`; source scan shows `combo_items` now returns sentinel plus visible normal colors and the test asserts row 8 is absent.

### 3.6 Direct Mouse Wheel

Active in YR: No direct scoped callback path found. The rechecked decompiles for `OwnerDraw_ComboBox_00617250` and `OwnerDraw_ScrollBar_0061C690` show mouse down/up/move/double-click/timer/custom message paths, but no `WM_MOUSEWHEEL (0x20A)` case. Prior ComboDropWin assembly passes also found no `0x20A` branch in the material popup block. This does not prove Windows cannot translate wheel input elsewhere at runtime; it does prove the scoped owner-draw combo/popup/scrollbar callbacks do not directly implement wheel scrolling.

Current Rust status: mismatch unless a parent translation path is later verified. `src/ui/skirmish_shell/state.rs:506` implements `handle_option_mouse_wheel` and `src/ui/skirmish_shell/state.rs:2227` tests wheel scrolling. For parity, do not keep direct dropdown wheel scroll as standard YR behavior without new runtime evidence.

## 4. INI Keys

No INI key controls the scoped dropdown visual/input behavior. These facts are owner-draw/window-message driven. Color swatches are from hardcoded shell color data consumed by `FUN_004e45a0`; `[Colors]` INI gameplay color definitions were not read by the scoped functions. Active in YR: Yes. Evidence: scoped decompiles above and prior color-combo reports.

## 5. Integration Points

| Function / address | Role | Active in YR | Evidence |
|---|---|---|---|
| `FUN_0060d450 @ 0x0060D450` | Registers `ComboDropWin` WndProc `0x0060D540` | Yes | decompile |
| `OwnerDraw_ComboBox_00617250 @ 0x00617250` | Standard combo callback; opens/closes popup | Yes | decompile |
| `ComboDropWin @ 0x0060D540..0x0060F311` | Popup row paint, hit-test, top-index, scrollbar bridge | Conditional while open | assembly contexts |
| `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690` | Child scrollbar paint/input/current state | Conditional on overflow | decompile |
| `FUN_004e45a0 / FUN_004e4770` | Normal/restricted color combo population | Yes/Conditional | decompile |
| `FUN_00621040 @ 0x00621040` | Text draw wrapper, vertical centering flag `0x04` | Yes | decompile |
| `FUN_0060f9a0 @ 0x0060F9A0` | Subclass owner-draw controls, maps ComboBox/ListBox/Scrollbar separately | Yes | decompile |

## 6. Current Rust Implementation Status

Current Rust files scanned:

- `src/ui/skirmish_shell/state.rs:506` implements direct dropdown mouse-wheel scrolling.
- `src/ui/skirmish_shell/state.rs:896` computes proportional track-click top index by centering the thumb on the click.
- `src/ui/skirmish_shell/state.rs:918` supplies combo items; `src/ui/skirmish_shell/state.rs:2455` tests color row-8 omission.
- `src/app_skirmish_shell_render.rs:647` chooses released/pressed scrollbar arrow art from current press state.
- `src/app_skirmish_shell_render.rs:1110` builds full-row selected dropdown rects.
- `src/app_skirmish_shell_render.rs:2715` and `:2738` test full-row selected fill and `ComboDropWin` text clipping.

Status summary:

- Implemented or no longer a current delta: selected-row full fill, text clipping contract test, pressed arrow asset selection, proportional track-click state, color row-8 omission, content shrink under scrollbar.
- Still a delta: direct wheel scrolling exists in Rust but no direct scoped callback behavior was found in YR.
- Partially unresolved by this slot: exact display RGB and exact native scrollbar repeat-timer feel.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Prior docs/gap scan | verified | reports listed in Sources | none |
| Current Rust source scan | verified | file/line scans in Section 6 | no implementation performed |
| `ComboDropWin` registration | verified | decompile `FUN_0060d450`; decompile `OwnerDraw_ComboBox_00617250` | none |
| Standard combo vs real listbox distinction | verified | decompile `FUN_0060f9a0` maps `ComboBox` and `ListBox` separately | no `0x6B` listbox internals in scope |
| Selected full-row fill | verified | assembly `0x0060DD42..0x0060DE0A` | final RGB only |
| Row text clipping/truncation | verified | assembly `0x0060DE1F..0x0060DFC8`; `FUN_00621040` | glyph raster internals out of scope |
| Scrollbar pressed arrows/current/top-index | verified | decompile `OwnerDraw_ScrollBar_0061C690`; contexts `0x0061D383..0x0061D548` | exact repeat-timer feel/screenshot state |
| Normal color row 8 omission | verified | decompile `FUN_004e45a0`; Rust test scan | none |
| Direct wheel behavior | verified negative for scoped callbacks | decompiles `0x00617250`, `0x0061C690`; prior `0x0060D540` pass | runtime parent translation if wheel parity is still desired |
| `ComboDropWin` full function recovery | deferred | no function object at `0x0060D540` in Ghidra | mutating function creation is forbidden in this slot |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-CDR-001 - Is the standard popup still `ComboDropWin`, not real `LISTBOX`? -> Yes; class registration and combo open create `ComboDropWin`, while `FUN_0060f9a0` maps `ListBox` to a separate callback.` (evidence: `FUN_0060d450`, `OwnerDraw_ComboBox_00617250`, `FUN_0060f9a0`)
- `[RESOLVED] OQ-CDR-002 - Is selected popup fill full-row? -> Yes, row rect is filled before swatch/text without inset arithmetic.` (evidence: assembly `0x0060DD42..0x0060DE0A`)
- `[RESOLVED] OQ-CDR-003 - Does current Rust still inset selected fill? -> No; current helper returns full content row and has a regression test.` (evidence: `src/app_skirmish_shell_render.rs:1110`, `:2715`)
- `[RESOLVED] OQ-CDR-004 - How is popup text clipped? -> Left `+3`, vertical-centered draw, text truncated against client width minus `0x14`.` (evidence: assembly `0x0060DE1F..0x0060DFC8`; `FUN_00621040`)
- `[RESOLVED] OQ-CDR-005 - Does current Rust have a text clipping guard? -> Yes, source has a test for side dropdown `client_width - 20`.` (evidence: `src/app_skirmish_shell_render.rs:2738`)
- `[RESOLVED] OQ-CDR-006 - Are scrollbar pressed arrow states visible in binary? -> Yes, up/down pressed flags are stored and arrow helper receives pressed state; current Rust selects pressed art.` (evidence: `OwnerDraw_ScrollBar_0061C690`; `src/app_skirmish_shell_render.rs:647`)
- `[RESOLVED] OQ-CDR-007 - Are track clicks page steps? -> No; native computes current/top from click-centered thumb position.` (evidence: `OwnerDraw_ScrollBar_0061C690`; assembly `0x0061D4EB..0x0061D522`)
- `[RESOLVED] OQ-CDR-008 - Does current Rust still page-step track clicks? -> No; it now has proportional track-click helper and test coverage.` (evidence: `src/ui/skirmish_shell/state.rs:896`, `:2302`)
- `[RESOLVED] OQ-CDR-009 - Is normal color row 8 visible? -> No; normal `FUN_004e45a0` loop inserts rows `0..7` plus sentinel; current Rust test asserts row 8 absent.` (evidence: `FUN_004e45a0`; `src/ui/skirmish_shell/state.rs:2455`)
- `[RESOLVED] OQ-CDR-010 - Is direct wheel scrolling present in scoped binary callbacks? -> No direct `0x20A` case found in combo/popup/scrollbar callbacks.` (evidence: decompiles `OwnerDraw_ComboBox_00617250`, `OwnerDraw_ScrollBar_0061C690`; prior `0x0060D540` assembly pass)
- `[RESOLVED] OQ-CDR-011 - Does current Rust implement direct wheel scrolling? -> Yes, and this is a parity risk until runtime translation is proven.` (evidence: `src/ui/skirmish_shell/state.rs:506`, `:2227`)
- `[DEFERRED] OQ-CDR-012 - Does Windows/runtime parent translation ever convert wheel input into another message path for this popup?` (category: `needs-runtime-debugger`; reason: scoped callbacks have no direct `0x20A`, but this slot did not run retail with wheel input capture; next-step-if-pursued: live retail message trace over an open side dropdown)
- `[DEFERRED] OQ-CDR-013 - Exact final RGB for selected fill/background after display conversion.` (category: `needs-runtime-debugger`; reason: binary source globals/conversion are known from prior docs, but screenshot/surface capture is required for final pixel values; next-step-if-pursued: retail capture at 800x600)
- `[DEFERRED] OQ-CDR-014 - Exact scrollbar repeat-timer feel.` (category: `out-of-scope`; reason: this recheck focused static state/math and current Rust deltas; next-step-if-pursued: runtime trace of held scrollbar arrow repeat cadence)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Scoped YR combo/popup/scrollbar callbacks do not directly handle `WM_MOUSEWHEEL (0x20A)` | decompiles `OwnerDraw_ComboBox_00617250`, `OwnerDraw_ScrollBar_0061C690`; prior `0x0060D540` no-`0x20A` pass | mismatch: Rust scrolls open dropdowns directly on wheel | `src/ui/skirmish_shell/state.rs::handle_option_mouse_wheel`; tests around `dropdown_wheel_and_hit_test_use_top_index` | Remove or gate direct wheel scrolling unless a runtime parent translation path is verified | Open side combo, wheel over popup: standard parity acceptance should be no direct top-index change from this handler | `skirmish_dropdown_mouse_wheel_noops_without_verified_translation`; risk: convenient modern UX diverges from retail |
| Selected popup row fill is a full content row before swatch/text | assembly `0x0060DD42..0x0060DE0A` | none observed; implemented | `src/app_skirmish_shell_render.rs::dropdown_selected_row_rect`, dropdown render tests | Preserve full-row selected fill and draw order | Open side/color dropdown with selected row visible; highlight covers entire content width/row height, not inset | `skirmish_dropdown_selected_row_fill_is_full_row`; do not reintroduce inset/outline selection |
| Scrollbar track clicks jump to the click-centered proportional top/current, while arrow clicks step one row and pressed arrow art is transient | `OwnerDraw_ScrollBar_0061C690`; contexts `0x0061D383..0x0061D548` | none observed for broad math/state; exact repeat timing unchecked | `src/ui/skirmish_shell/state.rs::top_index_from_scrollbar_track_click`, `src/app_skirmish_shell_render.rs::push_dropdown_scrollbar_instances` | Preserve proportional track click and pressed arrow art; do not page-step | Side dropdown with >7 rows: click below thumb jumps to native top-index range and arrow art shows pressed only while pressed | `skirmish_side_dropdown_scrollbar_track_click_jumps_to_native_top_index`; risk: exact FPU rounding/repeat cadence not screenshot-proven |

Proposed Rust test names:

- `skirmish_dropdown_mouse_wheel_noops_without_verified_translation`
- `skirmish_dropdown_selected_row_fill_is_full_row`
- `skirmish_dropdown_side_text_clip_uses_combodropwin_client_width_minus_20`
- `skirmish_side_dropdown_scrollbar_track_click_jumps_to_native_top_index`
- `skirmish_scrollbar_arrow_entry_uses_pressed_art_only_while_pressed`

## 10. Negative Facts / Do Not Do

- Do not treat standard combo popup rows as real `LISTBOX`/Choose Map `0x6B` rows. Active in YR: Yes. Evidence: `FUN_0060d450` registers `ComboDropWin`; `FUN_0060f9a0` maps `ListBox` separately.
- Do not draw selected dropdown rows inset or outlined. Active in YR: Yes. Evidence: full row fill assembly `0x0060DD42..0x0060DE0A`.
- Do not page-step scrollbar track clicks by visible rows. Active in YR: Conditional on overflow. Evidence: `OwnerDraw_ScrollBar_0061C690` computes click-centered proportional current.
- Do not expose normal color row 8 merely because backing data has an initialized row. Active in YR: Yes. Evidence: `FUN_004e45a0` loop stops before row 8 insertion.
- Do not add or preserve direct dropdown mouse-wheel scrolling as standard YR behavior without runtime evidence. Active in YR: No direct scoped callback evidence. Evidence: no `0x20A` case in scoped combo/popup/scrollbar callback evidence.

## 11. Remaining Uncertainty

- Runtime wheel delivery remains unresolved: no direct scoped handler exists, but this slot did not capture live Windows messages to prove whether an ancestor translates wheel input.
- Exact final RGB for popup background/selected fill remains screenshot/runtime work.
- Exact scrollbar repeat-timer feel and FPU thumb-height rounding were not re-derived beyond the decompiled constants and broad proportional math.

## 12. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`: replace current-Rust delta "Rust selected dropdown fill is inset by one pixel and height `row_h - 2`" with "Current Rust now uses full content-row selected fill and has `skirmish_dropdown_selected_row_fill_is_full_row`; preserve this behavior."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`: replace "Rust text rect width for popup labels is `content.w - 3`" with "Current Rust has a side-dropdown text clipping test for `ComboDropWin` client width minus `20`; keep draw left at `x+3` and keep text out of the scrollbar reserve."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`: replace "Rust has no direct mouse-wheel handling on dropdowns" with "Current Rust now has direct dropdown wheel scrolling at `src/ui/skirmish_shell/state.rs:506`, but scoped YR callbacks still have no direct `WM_MOUSEWHEEL (0x20A)` handler; treat Rust wheel scrolling as a parity risk unless runtime parent translation is verified."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`: replace "remaining deltas are pixel-level selected fill, primitive colors, disabled overlay, scrollbar state math, and color population row-8 omission" with "Current Rust has closed selected fill, basic text clipping, pressed arrow state, proportional track-click, and color row-8 omission; remaining combo dropdown deltas are direct wheel behavior, final RGB/disabled overlay, and exact scrollbar repeat/rounding."

## Sources

- Ghidra read-only decompile: `FUN_0060d450`, `OwnerDraw_ComboBox_00617250`, `OwnerDraw_ScrollBar_0061C690`, `FUN_004e45a0`, `FUN_004e4770`, `FUN_00621040`, `FUN_00620720`, `FUN_0060f9a0`.
- Ghidra read-only assembly contexts: `0x0060DD42..0x0060DE0A`, `0x0060DE1F..0x0060DFC8`, `0x0060E40C..0x0060E48A`, `0x0060F297..0x0060F311`, `0x0060E721..0x0060E745`, `0x0061D383..0x0061D548`.
- Prior docs cross-checked: `SKIRMISH_COMBODROPWIN_0060D540_FUNCTION_BOUNDARY_GHIDRA_REPORT.md`, `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`, `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_SCROLLBAR_SOUNDS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/ui/skirmish_shell/layout.rs`.
