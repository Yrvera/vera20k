# Skirmish Combo Dropdown Visual Parity - Ghidra Research Report

**Address(es):** `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `ComboDropWin WndProc block @ 0x0060D540`, `FUN_0060D450 @ 0x0060D450`, `FUN_00621040 @ 0x00621040`, `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`, `FUN_00620720 @ 0x00620720`, `FUN_006208F0 @ 0x006208F0`, `FUN_0072A9E0 @ 0x0072A9E0`, color helpers `0x004E45A0` / `0x004E4770`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** active standard YR offline Skirmish dialog `0x102` combo/dropdown visual contract: collapsed combo face, arrow state, selected item text/swatch, popup row geometry, selection fill, row clipping, scrollbar/scroll thumb, grey/disabled states, and input/redraw only where they affect visible combo/dropdown parity.
**Non-Scope:** parent shell composition, Choose Map modal, full text glyph internals beyond combo/dropdown caller contract, gameplay launch, CSF/localized string audit, and runtime screenshot capture.
**Confidence:** High for binary geometry, state gates, draw order, and current Rust deltas; Medium for final RGB appearance because this pass did not capture retail display pixels.
**Active in YR:** Yes for standard offline Skirmish `0x102` combo paths; some grey/scroll branches are Conditional as stated per finding.

## 0. Working Notes

Target question: What exact visible combo/dropdown behavior must current Rust reproduce for the gamemd-like Skirmish shell?
Non-goals: parent background/chrome composition, Choose Map `0x6B`, glyph raster internals, and gameplay launch.
Evidence needed to mark COMPLETE: prior-doc scan, current Rust scan, read-only decompile of combo/text/arrow/frame/color helpers, read-only assembly for `ComboDropWin` row loop, and scrollbar callback decompile.
Stop conditions: all material combo/dropdown visual questions either resolved with evidence or explicitly deferred to screenshot/runtime validation or another narrow system.

## 1. Overview

The active Skirmish combo path is a hybrid: the collapsed control is painted by `OwnerDraw_ComboBox_00617250`, opening creates a custom child window registered as `"ComboDropWin"` with WndProc block `0x0060D540`, and scrolling is delegated to the owner-draw scrollbar callback when a scrollbar child is required. The combo frame is primitive line/fill drawing through `FUN_006208F0`; the arrow and scrollbar grip pieces are owner-draw PCXs.

Current Rust now has the right broad model: `open_combo_dropdown`, top index,
dropdown row helpers, arrow-only open, and row selection state. As of
2026-05-23, `SKIRMISH_COMBODROPWIN_DROPDOWN_VISUAL_INPUT_RECHECK_GHIDRA_REPORT.md`
supersedes older selected-fill/current-Rust deltas: Rust now uses full content-row
selected fill, has text clipping coverage, pressed scrollbar arrow art selection,
proportional track-click math, and color row-8 omission. The current high-value
mismatch is direct dropdown mouse-wheel scrolling in Rust, because scoped YR
combo/popup/scrollbar callbacks still show no direct `WM_MOUSEWHEEL (0x20A)`
handler. Final popup/background/selected RGB still needs screenshot or surface
capture.

## 2. Key State / Constants

| Field / constant | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Collapsed face height | `24` px fixed paint face, independent of resource dropdown height | `0x0061745C`; prior combo geometry report | Yes |
| Arrow/click reserve | Rightmost `20` px (`0x14`) toggles dropdown and is excluded from text fit | mouse branch in `0x00617250`; text fit `0x00617B42..0x00617BAF` | Yes |
| Arrow position | top-left at `client_width - 19`, `y + 1`; destination uses native PCX size | `FUN_00620720 @ 0x00620720`; call context `0x00617C04` / prior `0x006178DC..0x0061791D` | Yes |
| Combo state `+0xCC` | swatch drawing enabled by `0x4DD` | `0x00618A3B..0x00618A46`; `0x004E45A0` sends `0x4DD=1` | Yes for color combos; Conditional elsewhere |
| Combo state `+0xCD` | owner-draw grey flag set by `0x4F1`; changes grey arrow/text/fill choices | `0x00618A51..0x00618A5C`; `0x004E4770` sends `0x4F1=1` | Conditional |
| Combo state `+0xD0` | max visible dropdown rows set by `0x4DE` | `0x00618A67..0x00618A6E`; helpers send `7` or `9` | Yes |
| Combo state `+0xF4` | active dropdown HWND | open/close branch `0x00617EAA..0x00618481` | Conditional while open |
| Combo state `+0xF8` | current selected index returned by `CB_GETCURSEL 0x147` and written by `CB_SETCURSEL 0x14E` | `0x00617250` decompile | Yes |
| Swatch slots | `state + 0x110 + index*4`; init fills 50 slots with `-1`, paint reads only `< 0x32` | `0x00618AEA..0x00618B61`, `0x00617A5D..0x00617B3F` | Yes |
| Scrollbar width | `DAT_00AC1DF0 * 2 + 0x12`, with `DAT_00AC1DF0 = 1`, so `20` px | `FUN_0060F9A0 @ 0x0060F9A0` initializes; scrollbar/list docs | Conditional when list scrolls |

## 3. Core Logic

### 3.1 Collapsed Face

Active in YR: Yes. `OwnerDraw_ComboBox_00617250` paints a cached collapsed surface on `WM_PAINT`: parent/background copy, primitive beveled frame, arrow PCX, optional disabled alpha overlay, optional swatch, truncated selected text, then validation. The frame is not a decoded retail face asset. `FUN_006208F0` converts globals such as `DAT_00AC1B98 = 0xC5BEA7` and `DAT_00AC1B94 = 0x807A68` through DirectDraw shifts/losses and draws inset bevel lines. Evidence: `0x00617250` decompile; `FUN_006208F0 @ 0x006208F0`; setup globals in `FUN_0060F9A0`.

Arrow PCX selection is handled by `FUN_00620720(surface, rect, direction, pressed, grey)`. Direction `0` uses down arrow format string `gdnarrow%c.pcx`; when grey is false it passes the string pointer offset by one byte, producing `dnarrowr.pcx` / `dnarrowp.pcx`. `%c` is `r` for released and `p` for pressed/open. The helper null-checks lookup before blit. Active in YR: Yes/Conditional for pressed and grey states. Evidence: `FUN_00620720 @ 0x00620720`.

Disabled style and grey state are different. `WS_DISABLED` triggers an alpha overlay after frame/arrow drawing; `0x4F1` grey state selects `gdnarrow*` arrow art and grey text/fill globals. Active in YR: Yes for disabled controls; Conditional for restricted/grey rows. Evidence: `0x00617250` disabled branch; `0x004E4770` restricted path.

### 3.2 Selected Text And Collapsed Swatch

Active in YR: Yes. The selected text is copied to `DAT_00AC18F8`, then repeatedly zero-terminated by one UTF-16 code unit until `BitFont__GetTextWidth <= client_width - 20`. The draw rect starts at `left + 2`, uses the fixed 24 px face height, and calls `FUN_00621040` with vertical-center flag `0x04`, not horizontal center. Evidence: `0x00617B42..0x00617C04`; `FUN_00621040 @ 0x00621040`.

Collapsed color swatch drawing is gated by `+0xCC`, selected index in `0..49`, and a nonnegative per-item swatch value. The source rect is the non-arrow face area (`client_width - 20`, height `24`); `FUN_0072A9E0(rect, 2)` insets left/top by `2` and subtracts `4` from width/height. A standard `44` px color combo therefore fills `(x+2, y+2, 20, 20)`. Active in YR: Yes for color combos; Conditional for other combos. Evidence: `0x00617A5D..0x00617B3F`; `FUN_0072A9E0 @ 0x0072A9E0`; `FUN_004E45A0`.

### 3.3 Popup Window Geometry And Row Iteration

Active in YR: Yes. `FUN_0060D450` registers class `"ComboDropWin"` with WndProc block `0x0060D540`. `OwnerDraw_ComboBox_00617250` creates it from `CB_SHOWDROPDOWN 0x14F` with child style `0x40000000`, parent `GetParent(combo_hwnd)`, width equal to combo client width, `x = combo_left - parent_left`, and `y = combo_top - parent_top + combo_client_height + 1`. Height is rounded down to an exact multiple of `CB_GETITEMHEIGHT(0)` after applying item count, available space, and `+0xD0` max rows. Evidence: class decompile `FUN_0060D450`; open branch decompile `0x00617250`; assembly context `0x0061815F..0x00618205`.

Popup row paint reads item count with `CB_GETCOUNT 0x146`, row height with `CB_GETITEMHEIGHT 0x154`, selected index from source state `+0xE8`, and top index from source/dropdown state `+0xF0`. Each row Y is `(item_index - top_index) * item_height`; row paint stops when row bottom exceeds popup client height, so standard draw is whole-row only. Active in YR: Yes. Evidence: assembly context `0x0060D759..0x0060D802`, `0x0060DBF3..0x0060DC6D`.

### 3.4 Dropdown Row Paint

Active in YR: Yes. Standard popup rows use current popup client width after scrollbar shrink. Row text rect is `left = row_left + 3`, `top = row_top`, `right = row_left + row_width`, `bottom = row_top + item_height`. Text is truncated against `client_width - 20`, then drawn by `FUN_00621040` with flags `0x04`, so it is vertically centered and left anchored. Evidence: assembly context `0x0060DE1F..0x0060DFC8`; `FUN_00621040`.

The selected row fill is full-row, not inset: the code compares current item index with selected index, builds the row rect, chooses normal selected fill `DAT_00AC4604` or grey selected fill `DAT_00AC4880`, converts it to the display format, and fills before swatch/text. Active in YR: Yes for selected rows; Conditional for grey selected fill. Evidence: assembly context `0x0060DD42..0x0060DE0A`.

Open color rows are swatch-dominant. If swatch mode is on, item index `< 0x32`, source state exists, and the swatch slot is nonnegative, the row rect is inset by `2` via `FUN_0072A9E0`, then filled with the swatch color. The text is still drawn afterward, with row text color overwritten by the swatch color value. Active in YR: Yes for color combos. Evidence: assembly context `0x0060DE60..0x0060DF2A`; color population `FUN_004E45A0`.

### 3.5 Scrollbar And Thumb

Active in YR: Conditional. Side/country dropdowns are the standard `0x102` family expected to scroll because they cap visible rows at `7` and can have more eligible rows. Scrollbar width is `20` px; when present the list content client is shrunk before row paint, so text, selection fill, and swatch fill do not draw under the scrollbar. Evidence: `FUN_0060F9A0` initializes `DAT_00AC1DF0 = 1`; `ComboDropWin` current-client reads at `0x0060D575..0x0060D5A8`; row width consumption `0x0060D846..0x0060D89C`.

The scrollbar callback `OwnerDraw_ScrollBar_0061C690` uses fixed arrow button height `0x16` (22 px). It computes thumb height from page/range data using a floating conversion, clamps it to at least `0x0E` (14 px), reserves the two 22 px arrow buttons, and places the thumb at `top + 0x16 + track_span * current / range`. Dragging recenters around half thumb height and clamps within `[0x16, bottom - thumb_h - 0x16]`. Active in YR: Conditional when a scrollbar child exists. Evidence: `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`, especially the `iStack_e8 < 0xf -> 0xe`, `0x16` arrow button, and drag branches.

Scrollbar paint uses primitive frame/background and PCX grip pieces. Normal grip names are `sbgripm.pcx`, `sbgript.pcx`, `sbgripb.pcx`; grey variants are `gsbgripm.pcx`, `gsbgript.pcx`, `gsbgripb.pcx`. It uses `FUN_00620720` for up/down arrows. Active in YR: Conditional. Evidence: `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`.

### 3.6 Color Population Inputs

Active in YR: Yes for normal color combos; Conditional for restricted grey rows. `FUN_004E45A0` resets the combo, sends `0x4DD=1`, `0x4DE=9`, inserts a sentinel row with item data `-2` and swatch `-1`, then inserts only initialized color rows `0..7` whose owner is this slot or unowned. It does not insert row `8` (`0x00606060`) in this normal loop. `FUN_004E4770` creates one restricted row, assigns item data `-2`, swatch `-1`, selects row `0`, and sets `0x4F1=1`. Evidence: decompile `0x004E45A0`, `0x004E4770`; prior color table report.

## 4. INI Keys

No INI key controls the combo/dropdown visual contract in this slice. The behavior is shell dialog/message/owner-draw state driven. `[Colors]` gameplay color schemes are not read by these scoped UI helper functions; the color combo swatches use the hardcoded table at `0x008316A8` through `FUN_004E43C0` / `FUN_004E45A0`. Active in YR: Yes. Evidence: scoped decompiled functions show no INI reads; prior color-combo report.

## 5. Integration Points

| Integration point | Role | Active in YR | Evidence |
|---|---|---|---|
| `FUN_0060F9A0` | hooks `"ComboBox"`, `"ScrollBar"`, initializes owner-draw globals and sends `0x497` | Yes | decompile `0x0060F9A0` |
| `OwnerDraw_ComboBox_00617250` | collapsed paint, selected text/swatch, open/close, item record wrapper | Yes | decompile `0x00617250` |
| `FUN_0060D450` / `0x0060D540` | registers and runs `ComboDropWin` popup | Yes | decompile `0x0060D450`; assembly contexts for `0x0060D540` |
| `FUN_00621040` | combo/dropdown text draw wrapper, vertical-centering and clipping | Yes | decompile `0x00621040` |
| `OwnerDraw_ScrollBar_0061C690` | scrollbar range/current/thumb paint/input | Conditional | decompile `0x0061C690` |
| `FUN_004E45A0` / `FUN_004E4770` | normal and restricted color-combo population | Yes/Conditional | decompile `0x004E45A0`, `0x004E4770` |

## 6. Current Rust Implementation Status

Rust now has substantial combo/dropdown scaffolding:

- `src/ui/skirmish_shell/state.rs:526` creates popup rects one pixel below the fixed 24 px face and caps side/color/start rows.
- `src/ui/skirmish_shell/state.rs:598` / `:616` subtract a 20 px scrollbar from content.
- `src/ui/skirmish_shell/state.rs:646` computes a proportional scroll thumb with a 14 px minimum.
- `src/ui/skirmish_shell/state.rs:898` opens only from the arrow hit area.
- `src/ui/skirmish_shell/state.rs:977` handles row selection, scrollbar arrows, page clicks, dragging, and outside close.
- `src/app_skirmish_shell_render.rs:691` draws collapsed face, color swatch, and arrow.
- `src/app_skirmish_shell_render.rs:905` draws popup background, selection fill, color swatches, scrollbar, and border.
- `src/app_skirmish_shell_render.rs:1548` / `:1609` draw collapsed labels and popup row labels.

Important Rust deltas:

- Collapsed face generation is intentionally primitive and structurally correct, but line colors are hardcoded in `src/render/skirmish_shell_chrome.rs:221` / `:227` instead of being derived from the binary globals and DirectDraw conversion.
- Rust selected dropdown fill is inset by one pixel and height `row_h - 2` at `src/app_skirmish_shell_render.rs:930..935`; gamemd fills the full row rect before swatch/text.
- Rust popup background/border colors are approximated constants at `src/app_skirmish_shell_render.rs:47..50`; gamemd uses owner-draw primitive/surface colors from shell globals.
- Rust text rect width for popup labels is `content.w - 3` at `src/app_skirmish_shell_render.rs:1614`; gamemd truncates text against current client width minus `20`, while the draw rect still extends to the row width.
- Rust has no `WS_DISABLED` alpha-overlay equivalent for disabled combos; it uses grey arrow selection in `push_combo_face` but not the disabled overlay branch.
- Rust color combo items currently enumerate `0..HOUSE_COLOR_COUNT` at `src/ui/skirmish_shell/state.rs:754`, while gamemd normal population inserts only rows `0..7` plus the `-2` sentinel, despite initializing row `8`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Prior reports and conflict scan | verified | listed Sources | none |
| `OwnerDraw_ComboBox_00617250` collapsed face/text/swatch/open | verified | decompile `0x00617250`; assembly contexts `0x00617BAF`, `0x00617C04`, `0x0061815F..0x00618205` | final RGB screenshot validation |
| `ComboDropWin` class registration | verified | `FUN_0060D450 @ 0x0060D450` | none |
| `ComboDropWin` row loop/paint | verified | assembly contexts `0x0060D759`, `0x0060DBF3`, `0x0060DC09`, `0x0060DD42`, `0x0060DE1F`, `0x0060DFC8` | full WndProc decompile unavailable because no function boundary exists |
| Text draw wrapper contract | verified | `FUN_00621040 @ 0x00621040` | glyph raster internals out-of-scope |
| Arrow helper | verified | `FUN_00620720 @ 0x00620720` | none |
| Primitive frame helper | verified | `FUN_006208F0 @ 0x006208F0` | screenshot-level final color validation |
| Inset helper | verified | `FUN_0072A9E0 @ 0x0072A9E0` | none |
| Scrollbar callback | verified | `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690` | exact listbox-to-scrollbar `0xE9` page/range payload for every combo family could be expanded if needed |
| Color normal/restricted population | verified | `FUN_004E45A0`, `FUN_004E4770` | no further visual gap |
| Current Rust comparison | verified | files/line scans in Section 6 | implementation work remains |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-CDV-001 - Is the active standard popup path `ComboDropWin` or direct native listbox paint? -> `ComboDropWin` class is registered and created; row paint uses its WndProc block and source combo state.` (evidence: `FUN_0060D450`, `0x006181EC..0x00618205`, `0x0060D540` assembly contexts)
- `[RESOLVED] OQ-CDV-002 - Is the collapsed face an asset? -> No; it is primitive bevel drawing through `FUN_006208F0`; arrow/grip pieces are PCXs.` (evidence: `0x00617250`, `FUN_006208F0`, `FUN_00620720`)
- `[RESOLVED] OQ-CDV-003 - What opens the dropdown? -> Mouse down/double-click only when `x > client_width - 20`, then posts `CB_SHOWDROPDOWN 0x14F`.` (evidence: `0x00617250` mouse branch; prior geometry report)
- `[RESOLVED] OQ-CDV-004 - How are selected collapsed labels drawn? -> truncate to `client_width - 20`, rect left `+2`, vertical-center-only via `FUN_00621040` flags `0x04`.` (evidence: `0x00617B42..0x00617C04`, `0x00621040`)
- `[RESOLVED] OQ-CDV-005 - How are popup rows positioned and clipped? -> top index `+0xF0`, row Y `(index-top)*item_height`, stop when row bottom exceeds client height; no partial extra row in standard popup.` (evidence: `0x0060DBF3..0x0060DC6D`)
- `[RESOLVED] OQ-CDV-006 - What is selected row fill geometry? -> full row rect before swatch/text; grey selected fill uses a different global.` (evidence: `0x0060DD42..0x0060DE0A`)
- `[RESOLVED] OQ-CDV-007 - What is open color swatch geometry? -> row rect inset by `2`; for standard 44 px no-scroll color rows this is about `40x19` at `(x+2,y+2)`.` (evidence: `0x0060DE60..0x0060DF2A`, `FUN_0072A9E0`)
- `[RESOLVED] OQ-CDV-008 - Does scrollbar shrink row content? -> Yes; row paint uses the current client width after scrollbar consumption, with a standard 20 px scrollbar.` (evidence: `FUN_0060F9A0`, `0x0060D575..0x0060D89C`)
- `[RESOLVED] OQ-CDV-009 - What is scrollbar thumb math? -> arrow buttons are 22 px; thumb minimum is 14 px; proportional position uses current/range over the post-button track span.` (evidence: `OwnerDraw_ScrollBar_0061C690`)
- `[RESOLVED] OQ-CDV-010 - Are disabled and grey states the same? -> No; disabled is an alpha overlay, grey is `0x4F1` state that changes arrow/text/fill globals.` (evidence: `0x00617250`, `0x004E4770`)
- `[RESOLVED] OQ-CDV-011 - Does normal color population include all nine initialized swatches? -> No; it inserts rows `0..7`, not row `8`, plus sentinel `-2`.` (evidence: `FUN_004E45A0`)
- `[RESOLVED] OQ-CDV-012 - Does current Rust model top-index and arrow-only open? -> Yes, current state helpers do this.` (evidence: `src/ui/skirmish_shell/state.rs:526`, `:898`, `:977`)
- `[DEFERRED] OQ-CDV-013 - Exact final RGB under retail display mode for primitive frame, popup background, selected fill, and disabled alpha.` (category: `needs-runtime-debugger`; reason: binary proves source globals/conversion, but screenshot/display capture is required for final pixel comparison; next-step-if-pursued: retail screenshot or live 16-bit surface capture)
- `[DEFERRED] OQ-CDV-014 - Full `ComboDropWin` WndProc decompile as one function.` (category: `requires-different-system-context`; reason: Ghidra has labels/assembly but no function boundary at `0x0060D540`; this report used assembly range evidence for material row behavior; next-step-if-pursued: read-only function-boundary reconstruction in a separate low-level pass)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Collapsed combo face is primitive bevel using shell globals, arrow PCX at `right-19,y+1`, disabled overlay separate from grey state | `0x00617250`, `FUN_006208F0`, `FUN_00620720`, `FUN_0060F9A0` | partial: primitive face exists, but colors are hardcoded and disabled alpha overlay is missing | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render.rs` | derive/centralize owner-draw primitive colors and add disabled overlay path while keeping grey arrow state separate | Disabled AI sibling side/color/start/team combos render dimmed but not as merely grey-arrow-only; proposed test `skirmish_combo_disabled_overlay_is_distinct_from_grey_arrow_state` | Do not replace primitive face with unverified PCX art |
| Dropdown selected fill covers the full row rect before swatch/text; text truncates to client width minus 20 while draw rect starts at `x+3` and spans row width | `0x0060DD42..0x0060DE0A`, `0x0060DE1F..0x0060DFC8` | implemented/current as of 2026-05-23: Rust uses full content-row selected fill and has text clipping coverage | `src/app_skirmish_shell_render.rs` text/dropdown draw construction | preserve current full-row selected fill, swatch/text order, and scrollbar-shrunk text clipping | Opening the side dropdown highlights the selected row edge-to-edge across content width and text never draws under the scrollbar; proposed test `skirmish_dropdown_selected_row_fill_and_text_clip_match_combodropwin` | Do not reintroduce inset/outline selected rows or collapsed-combo text geometry for popup rows |
| Scrollbar thumb uses native owner-draw range math: 22 px arrow buttons, 14 px minimum thumb, proportional position, PCX grip top/mid/bottom plus arrow helper | `OwnerDraw_ScrollBar_0061C690`, `FUN_00620720` | partial: Rust has 20 px scrollbar and 14 px min thumb but page/range payload and pressed/grey states are approximate | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | align thumb height/position to native range/current/page and add pressed arrow/thumb state if visible input parity is pursued | Side combo with more than 7 country rows scrolls one row on arrow, one page on track click, and thumb lands at the same top-index positions as retail; proposed test `skirmish_side_dropdown_scrollbar_thumb_matches_native_range_math` | Do not let row content draw under the scrollbar or invent a generic egui scrollbar |
| Normal color combo population exposes sentinel `-2` plus color rows `0..7`; row `8` is initialized but not inserted by `FUN_004E45A0`; restricted rows are single grey `-2` row | `FUN_004E45A0`, `FUN_004E4770` | mismatch if `HOUSE_COLOR_COUNT` includes 9; current Rust maps `0..HOUSE_COLOR_COUNT` | `src/ui/skirmish_shell/state.rs` combo item population and tests | make standard color combo items match binary insertion and model restricted grey row separately | Opening a normal color dropdown shows the sentinel plus eight color swatches, not the grey row-8 swatch; proposed test `skirmish_color_dropdown_omits_initialized_row_8_in_normal_population` | Do not equate initialized color table rows with visible normal dropdown rows |

## Negative Facts / Do Not Do

- Do not render the collapsed combo face from an unverified PCX or SHP. Active in YR: Yes. Evidence: `OwnerDraw_ComboBox_00617250` calls `FUN_006208F0`; only arrow uses `FUN_00620720` PCX lookup.
- Do not treat `OwnerDraw_ListBox_00618D40` as the direct standard combo popup row painter in current wording. Active in YR: Yes. Evidence: `FUN_0060D450` registers `ComboDropWin` with WndProc `0x0060D540`; row loop evidence is in that block.
- Do not draw selected popup row as an inset/outlined highlight. Active in YR: Yes. Evidence: full row fill block `0x0060DD42..0x0060DE0A`.
- Do not use collapsed color swatch geometry for open color rows. Active in YR: Yes. Evidence: collapsed swatch is non-arrow face inset to `20x20` at 44 px; open row swatch is full-row inset by `2`.
- Do not equate disabled style with `0x4F1` grey state. Active in YR: Yes/Conditional. Evidence: disabled alpha overlay in combo paint; restricted helper `FUN_004E4770` sets grey state.
- Do not expose normal color row `8` merely because the hardcoded table initializes it. Active in YR: Yes. Evidence: `FUN_004E45A0` loop stops before owner pointer `0x008B40A0`, so rows `0..7` are inserted.

## Remaining Uncertainty

- Screenshot-level final RGB for frame/popup/selected/disabled colors remains unresolved; binary source globals and conversion paths are verified, but retail capture is needed for final pixel validation.
- Full single-function decompile for `ComboDropWin @ 0x0060D540` remains unavailable without mutating Ghidra boundaries; material row/scroll evidence was taken from assembly contexts.
- Exact `0xE9` range/page payload from listbox to scrollbar for every combo family was not expanded beyond the owner-draw scrollbar callback contract; standard side-scroll acceptance can validate this after implementation.

## Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`: superseded by the follow-up correction now in that file: `ComboDropWin` is registered with WndProc block `0x0060D540` and owns the standard combo popup row loop; `OwnerDraw_ListBox_00618D40` remains the real owner-drawn `LISTBOX` callback.
- `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`: replace "Full `LAB_0060D540` popup window procedure behavior deferred" with "Material `ComboDropWin` row-paint behavior is covered by `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`; a full function-boundary reconstruction remains optional."
- `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`: replace current Rust status saying only color combos are modeled/immediate-cycling with "Current Rust now has `open_combo_dropdown`, row helpers, top-index scrolling, arrow-only open, popup rendering, full-row selected fill, pressed scrollbar arrow art, proportional track-click math, and color row-8 omission. Remaining deltas are direct mouse-wheel scrolling without verified YR callback evidence, exact popup/selected RGB, and any still-unverified disabled overlay/runtime scrollbar repeat details."

## Proposed Rust Tests

- `skirmish_combo_disabled_overlay_is_distinct_from_grey_arrow_state`
- `skirmish_dropdown_selected_row_fill_and_text_clip_match_combodropwin`
- `skirmish_side_dropdown_scrollbar_thumb_matches_native_range_math`
- `skirmish_color_dropdown_omits_initialized_row_8_in_normal_population`
- `skirmish_open_color_rows_use_full_row_inset_swatch_not_collapsed_chip`

## Sources

- Ghidra read-only decompile: `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `FUN_0060D450 @ 0x0060D450`, `FUN_00621040 @ 0x00621040`, `FUN_00620720 @ 0x00620720`, `FUN_006208F0 @ 0x006208F0`, `FUN_0072A9E0 @ 0x0072A9E0`, `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_004E45A0 @ 0x004E45A0`, `FUN_004E4770 @ 0x004E4770`.
- Ghidra read-only assembly contexts: `0x0060D759..0x0060D802`, `0x0060DBF3..0x0060DC6D`, `0x0060DD42..0x0060DE0A`, `0x0060DE1F..0x0060DFC8`, `0x0060E0E9..0x0060E14F`, `0x0061815F..0x00618205`, `0x00617BAF..0x00617C04`.
- Prior docs: `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`, `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
