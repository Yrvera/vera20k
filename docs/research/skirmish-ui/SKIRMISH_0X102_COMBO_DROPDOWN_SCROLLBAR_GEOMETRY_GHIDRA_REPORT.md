# Skirmish 0x102 Combo Dropdown Scrollbar Geometry - Ghidra Report

**Address(es):** `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `ComboDropWin WndProc block @ 0x0060D540`, `OwnerDraw_ListBox_00618D40 @ 0x00618D40`, `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`, population helpers `0x004E3A00`, `0x004E45A0`, `0x004E50C0`, `0x004E5B60`  
**Investigation Mode:** exhaustive-slice for the requested combo/dropdown geometry and input behavior.  
**Claimed Scope:** standard offline Yuri's Revenge Skirmish setup dialog `0x102` combo face/dropdown/list/scrollbar geometry, top-index scrolling, mouse input, and family differences for AI type, side/country, color, start, and team combos.  
**Non-Scope:** launch/session packing except item data that changes visible rows; Choose Map modal `0x6B`; full RGB screenshot validation; mutating Ghidra function-boundary repair for `0x0060D540`.  
**Confidence:** High for geometry, caps, hit testing, row/top-index behavior, item-count differences, and Rust-facing deltas; Medium for exact display RGB and unresolved single-function decompile of `ComboDropWin`.  
**Active in YR:** Yes for standard `0x102` combo paths; scrollbar/grey/restricted branches are Conditional as stated.

## 0. Working Notes

Target question: Verify full combo/dropdown geometry and input behavior for standard offline Skirmish setup dialog `0x102`, including face/arrow/text/swatch rects, popup size, scrollbar shrink/thumb/buttons, top-index movement, mouse wheel/page/drag/hit testing, and family differences.

Non-goals: launch packing, unrelated shell controls, network-only lobby variants, Choose Map modal listboxes, final screenshot RGB, and any Ghidra database edits.

Evidence needed to mark COMPLETE: decompile plus address-range evidence for combo open/close and height math; decompile plus xref/caller evidence for YR-active subclass/init; decompile of scrollbar callback for input physics; population helper decompiles for row counts/caps; focused Rust scan for affected surfaces.

Stop conditions: no Ghidra read-only access; evidence expands into non-combo shell systems; `ComboDropWin` function-boundary mutation would be required; or unresolved open questions materially change Rust handoff.

## 1. Overview

The standard Skirmish combos are owner-drawn `ComboBox` controls hooked by `FUN_0060F9A0`, painted by `OwnerDraw_ComboBox_00617250`, and opened into a custom child window class named `ComboDropWin`. The popup is created one pixel below the collapsed 24 px face, with width equal to the combo client width and height rounded down to a whole number of owner-draw rows.

The scrollbar is not a generic Rust-like scroll area. `OwnerDraw_ListBox_00618D40` creates a child `"Scrollbar"` only when rows overflow the visible list height, shrinks the list client width by the native scrollbar width, and lets `OwnerDraw_ScrollBar_0061C690` handle arrows, track clicks, thumb drag, repeat timer, and parent `WM_VSCROLL`.

## 2. Verified Binary Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| `FUN_0060F9A0` maps class `"ComboBox"` to `OwnerDraw_ComboBox_00617250`; `FUN_006AE6E0` initializes the standard offline setup combo IDs. | decompile `0x0060F9A0`; xref to `0x00617250` from `0x0060FC64`; decompile `0x006AE6E0` | Yes |
| Collapsed face paint height is fixed at `24` px; dropdown row height is `font_height + 6`, read by `CB_GETITEMHEIGHT(0)`. With `GAME.FNT` height 17, standard rows are `23` px. | decompile `0x00617250`; init block `0x00618AEA..0x00618B46`; open read `0x006180FB..0x00618118` | Yes |
| Arrow hit/toggle zone is the rightmost `20` px and the arrow PCX is placed at `right - 19, y + 1`; text fitting excludes the same `20` px. | decompile `0x00617250`; mouse branch `0x00617D4E`; paint/text blocks `0x006178C4..0x00617C04` | Yes |
| `ComboDropWin` is registered by `FUN_0060D450` with WndProc `0x0060D540`; the combo creates it as a child of the combo parent. | decompile `0x0060D450`; create block `0x00618172..0x00618205` | Yes |
| Popup top is `combo_top_relative + combo_client_height + 1`, not the bottom of the tall resource combo rectangle. | decompile `0x00617250`; disassembly range `0x00618150..0x0061820F` | Yes |
| Popup width is the combo client width; popup height is rounded down by `height % item_height`. The `+1` in capped height does not survive final creation. | decompile `0x00617250`; disassembly range `0x00618150..0x0061820F` | Yes |
| Max visible rows are stored by custom message `0x4DE` at combo state `+0xD0`; side uses `7`, color/start/team use `9`, AI type does not set a cap and has 4 rows. | `0x00618A67..0x00618A6E`; decompiles `0x004E3A00`, `0x004E45A0`, `0x004E50C0`, `0x004E5B60`, `0x006AE6E0` | Yes |
| Scrollbar width is `DAT_00AC1DF0 * 2 + 0x12`; `FUN_0060F9A0` initializes `DAT_00AC1DF0 = 1`, so standard width is `20` px. | decompile `0x0060F9A0`; listbox decompile `0x00618D40` | Conditional when rows overflow |
| When a scrollbar exists, the listbox creates a child `"Scrollbar"` with style `0x50010001`, sends it `0xE9`, then `SetWindowPos` shrinks list width by scrollbar width. Row paint and hit testing use the shrunken client. | decompile `0x00618D40`; scrollbar block `0x0061BFD0..0x0061C45D` | Conditional |
| Scrollbar arrow buttons are `22` px high, minimum thumb height is `14` px, and thumb position is proportional to current/range over the post-button track. | decompile `0x0061C690`; thumb math and paint/input branches | Conditional |
| Track clicks outside the thumb jump the scrollbar current value by centering the thumb on the click and converting to a top/range value; they are not page-up/page-down by visible-row count. | decompile `0x0061C690`, `WM_LBUTTONDOWN 0x201` branch around track-click computation | Conditional |
| Thumb drag also recenters around half the thumb height, clamps between the two 22 px arrow buttons, and sends parent `WM_VSCROLL 0x115` only when current/range changes. | decompile `0x0061C690`, `WM_MOUSEMOVE 0x200` and final send branch | Conditional |
| Direct mouse wheel handling was not found in the combo, popup/listbox, or scrollbar callbacks; handled messages include mouse down/up/move/double-click, timer, `WM_VSCROLL`, and custom owner-draw messages. | decompiles `0x00617250`, `0x00618D40`, `0x0061C690`; no `0x20A` case in scoped callbacks | No direct callback path found |
| Combo hit testing while open forwards custom `0x4E8` to the active dropdown; the listbox returns `-1` outside client width/height, otherwise `top_index + y / item_height` if inside item count. | decompile `0x00617250` `0x4E8`; decompile `0x00618D40` `0x4E8` | Conditional while open |
| Selected popup row fill is full-row before swatch/text, not an inset rectangle. | `ComboDropWin` row assembly range `0x0060DD40..0x0060DFC7`; prior row-loop report | Yes |
| Collapsed color swatch uses non-arrow face inset by 2 px; at standard 44 px color combo width this is `20x20`. Open color rows use row rect inset by 2 px, so no-scroll 44 px rows produce a `40x19` swatch. | decompile `0x00617250`; row assembly range `0x0060DE60..0x0060DF2A`; inset helper `0x0072A9E0` from prior report | Yes for color combos |

## 3. Combo Family Differences

| Family | Controls | Width | Visible rows / cap | Scrollbar expectation | Active in YR |
|---|---:|---:|---|---|---|
| AI type | `0x50B`, `0x50E`, `0x516`, `0x51A..0x51D` | `150` px | 4 rows, no `0x4DE` cap needed | no standard scrollbar | Yes |
| Side/country | `0x6A1`, `0x510`, `0x513`, `0x514`, `0x51E..0x521` | `117` px | Random plus eligible multiplayer houses, cap `7` | standard family that can overflow and scroll | Yes |
| Color | `0x6A2`, `0x522..0x528` | `44` px | sentinel `-2` plus normal rows `0..7`, cap `9` | no standard scrollbar when all normal rows fit | Yes |
| Start | `0x6A3..0x6A8`, `0x6AA`, `0x6AB` | `38` px | sentinel `-2` plus map/ownership-limited rows, cap `9` | no standard scrollbar in normal 9-row cap | Yes |
| Team | `0x76D..0x774` | `38` px | optional `-2` row plus four team rows, cap `9` | no standard scrollbar in normal 5-row set | Yes |

Side, color, start, and team restricted/closed paths replace the normal list with one grey row and send `0x4F1=1`; disabled Win32 style is separate from this owner-draw grey state. Active in YR: Conditional. Evidence: `0x004E3B90`, `0x004E4770`, `0x004E5260`, `0x004E5CB0`, and combo paint in `0x00617250`.

## 4. Current Rust Status

Rust already models the broad custom-dropdown shape: `src/ui/skirmish_shell/state.rs` has `open_combo_dropdown`, `combo_dropdown_rect`, `combo_dropdown_content_rect`, top-index state, arrow-only hit testing, scrollbar arrows, track clicks, dragging, and row selection. `src/app_skirmish_shell_render.rs` draws combo faces, dropdown background, selected rows, color swatches, and scrollbar PCXs.

Observed deltas against verified retail behavior:

- Rust uses `COMBO_DROPDOWN_ROW_H = 23`, `COMBO_DROPDOWN_SCROLLBAR_W = 20`, arrow-button height `22`, and min thumb `14`, which matches the verified standard constants.
- Rust track-click behavior scrolls by visible row count; retail owner-draw scrollbar jumps to the absolute thumb position derived from the click.
- Rust selected row fill is inset by 1 px and height `row_h - 2`; retail fills the full row.
- Rust popup label clipping is based on the content rect width; retail row text uses a row rect starting at `x + 3` and truncates against current client width minus `20`.
- Rust color combo population uses `0..HOUSE_COLOR_COUNT`; normal retail population inserts the sentinel plus rows `0..7`, while row 8 is initialized in data but not normally inserted by `FUN_004E45A0`.
- Rust disabled combos currently use grey/disabled arrow selection but do not fully model the separate disabled alpha overlay versus `0x4F1` grey row state.
- Current Rust now has direct dropdown wheel scrolling at `src/ui/skirmish_shell/state.rs:506`, but scoped YR callbacks still have no direct `WM_MOUSEWHEEL (0x20A)` handler. Treat Rust wheel scrolling as a parity risk unless runtime parent translation is verified.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Active `0x102` combo hook/init | verified | `0x0060F9A0`, `0x006AE6E0`, xref `0x0060FC64 -> 0x00617250` | none |
| Collapsed face/arrow/text/swatch rects | verified | `0x00617250`, `0x00617A5D..0x00617C04` | final RGB screenshot |
| Dropdown create rect/top/height/caps | verified | `0x00617250`, `0x00618150..0x0061820F`, `0x00618A67..0x00618A6E` | none |
| Family row counts and caps | verified | `0x006AE6E0`, `0x004E3A00`, `0x004E45A0`, `0x004E50C0`, `0x004E5B60` | exact localized strings out of scope |
| Scrollbar creation and content-width shrink | verified | `0x00618D40`, `0x0061BFD0..0x0061C45D` | none for geometry |
| Scrollbar thumb/buttons/drag/track click | verified | `0x0061C690` | final pressed-frame screenshot |
| Mouse wheel | verified negative for scoped callbacks | `0x00617250`, `0x00618D40`, `0x0061C690` | possible parent translation remains not found |
| Full `ComboDropWin` single decompile | deferred | `0x0060D540` has label/assembly, no clean function boundary | read-only boundary reconstruction if ever required |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-CG-001 - Is the dropdown custom or native? -> Custom `ComboDropWin` child window.` (evidence: `0x0060D450`, `0x00618172..0x00618205`)
- `[RESOLVED] OQ-CG-002 - What creates the active standard path? -> `FUN_0060F9A0` hooks `"ComboBox"` and `FUN_006AE6E0` initializes the offline combo IDs.` (evidence: `0x0060F9A0`, `0x006AE6E0`)
- `[RESOLVED] OQ-CG-003 - What is the popup top/height formula? -> top is collapsed client height plus 1; height is capped then rounded down by row height.` (evidence: `0x00618150..0x0061820F`)
- `[RESOLVED] OQ-CG-004 - Does scrollbar shrink content? -> Yes, list width is reduced by 20 px before row paint/hit testing.` (evidence: `0x00618D40`, `0x0061BFD0..0x0061C45D`)
- `[RESOLVED] OQ-CG-005 - Are track clicks page scrolls? -> No, they jump by converting click position to scrollbar current/top value.` (evidence: `0x0061C690`)
- `[RESOLVED] OQ-CG-006 - Is direct wheel input implemented in these callbacks? -> No direct `0x20A` case found.` (evidence: decompiles `0x00617250`, `0x00618D40`, `0x0061C690`)
- `[RESOLVED] OQ-CG-007 - Which family normally scrolls? -> side/country can scroll; AI/color/start/team normally fit their caps.` (evidence: helper decompiles listed in Section 3)
- `[DEFERRED] OQ-CG-008 - Exact retail display RGB for primitive face, popup fill, selected fill, and disabled overlay.` (category: `needs-runtime-debugger`; reason: binary source globals are known, but screenshot/display capture is required for final pixel values; next-step-if-pursued: retail screenshot or live surface capture)
- `[DEFERRED] OQ-CG-009 - Full `ComboDropWin` function-boundary decompile.` (category: `requires-different-system-context`; reason: read-only Ghidra access exposes the WndProc label/assembly but not a clean function body; next-step-if-pursued: separate read-only boundary reconstruction)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Scrollbar track clicks jump to the top index implied by click-centered thumb position; arrows step one row and drag is absolute/proportional. | `OwnerDraw_ScrollBar_0061C690`; listbox `0x115` integration | mismatch: Rust track clicks page by visible row count | `src/ui/skirmish_shell/state.rs` combo scrollbar handlers | replace page-step track click with native absolute jump math while keeping 22 px arrow zones and 14 px min thumb | Side combo with >7 countries: clicking below the thumb lands at the same top index as native proportional scrollbar, not merely `+visible_rows`; proposed test `skirmish_side_dropdown_scrollbar_track_click_jumps_to_native_top_index` | Do not model the owner-draw scrollbar as an egui/page-scroll widget |
| Selected popup row is filled across the full list content row before swatch/text. | `ComboDropWin` assembly `0x0060DD40..0x0060DE0A` | implemented/current as of 2026-05-23: Rust now uses full content-row selected fill | `src/app_skirmish_shell_render.rs::dropdown_selected_row_rect` / `push_dropdown_instances` | preserve full content-row selected fill and swatch/text order | Open any selected side dropdown row and assert selected fill starts at content x/y and uses full row height; proposed test `skirmish_dropdown_selected_row_fill_is_full_row` | Do not reintroduce inset or outline selected rows |
| Normal color population inserts sentinel `-2` plus rows `0..7`; row 8 is initialized data but not inserted by `FUN_004E45A0`. | `FUN_004E45A0 @ 0x004E45A0`; loop over `DAT_008B4040..0x008B40A0` | possible mismatch if Rust exposes all `HOUSE_COLOR_COUNT` entries | `src/ui/skirmish_shell/state.rs::combo_items` and color tests | make visible normal color rows match the inserted retail rows and keep restricted one-row grey path separate | Opening a color combo shows sentinel plus eight normal color rows, not an extra initialized row 8; proposed test `skirmish_color_dropdown_normal_population_omits_initialized_row_8` | Do not equate initialized color table rows with visible dropdown rows |
| Popup row text/swatch uses content width after scrollbar shrink; direct wheel handling is absent in the scoped callbacks. | `0x00618D40` `0x4E8`, scrollbar shrink `0x0061BFD0..0x0061C45D`, scoped decompiles lack `0x20A` | mostly matching; ensure no row text/swatch under scrollbar and no invented wheel scroll | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | keep scrollbar content shrink and avoid adding unverified wheel scroll to the shell dropdown | Side dropdown row hit in scrollbar column does not select a row; mouse wheel over an open dropdown has no direct shell effect unless later parent translation is proven; proposed test `skirmish_dropdown_scrollbar_column_not_row_hit_and_wheel_noop` | Do not add convenient mouse-wheel behavior without binary evidence |

## Negative Facts / Do Not Do

- Do not place the popup at the bottom of the resource combo rectangle; retail uses collapsed client height plus one pixel. Active in YR: Yes. Evidence: `0x00618150..0x0061820F`.
- Do not page-scroll the dropdown track by visible rows; retail track clicks jump to a proportional absolute value. Active in YR: Conditional. Evidence: `OwnerDraw_ScrollBar_0061C690`.
- Do not draw selected popup rows inset by one pixel. Active in YR: Yes. Evidence: `0x0060DD40..0x0060DE0A`.
- Do not let dropdown row content draw or hit-test under the scrollbar. Active in YR: Conditional. Evidence: listbox width shrink `0x0061BFD0..0x0061C45D` and `0x4E8` bounds in `0x00618D40`.
- Do not expose normal color row 8 just because the backing color table contains initialized data. Active in YR: Yes. Evidence: `FUN_004E45A0`.
- Do not conflate disabled Win32 style with owner-draw grey state `0x4F1`. Active in YR: Yes/Conditional. Evidence: combo paint `0x00617250`, restricted helpers `0x004E4770`, `0x004E5260`, `0x004E5CB0`.
- Do not add mouse-wheel scrolling for this dropdown unless a separate parent translation path is verified. Active in YR: No direct callback evidence. Evidence: no `0x20A` case in scoped callbacks.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`: replace "Side combo with more than 7 country rows scrolls one row on arrow, one page on track click" with "Side combo with more than 7 country rows scrolls one row on arrow, but a scrollbar track click jumps to the top index implied by centering the native thumb on the click."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`: superseded by the follow-up correction now in that file: `ComboDropWin` owns standard combo popup row paint/hit testing; `OwnerDraw_ListBox_00618D40` owns real owner-drawn `LISTBOX` controls such as Choose Map `0x6EB`/`0x553`.

## Sources

- Ghidra read-only decompile: `OwnerDraw_ComboBox_00617250 @ 0x00617250`.
- Ghidra read-only decompile: `OwnerDraw_ListBox_00618D40 @ 0x00618D40`.
- Ghidra read-only decompile: `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`.
- Ghidra read-only decompile: `FUN_0060D450 @ 0x0060D450`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_006AE6E0 @ 0x006AE6E0`.
- Ghidra read-only decompile: `FUN_004E3A00`, `FUN_004E3B90`, `FUN_004E45A0`, `FUN_004E4770`, `FUN_004E50C0`, `FUN_004E5260`, `FUN_004E5480`, `FUN_004E5B60`, `FUN_004E5CB0`, `FUN_004E5ED0`.
- Ghidra read-only disassembly ranges checked: `0x00618150..0x0061820F`, `0x00618A30..0x00618A74`, `0x0060DD40..0x0060DFC7`, `0x0061BFD0..0x0061C45C`.
- Prior docs cross-checked: `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`, `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
