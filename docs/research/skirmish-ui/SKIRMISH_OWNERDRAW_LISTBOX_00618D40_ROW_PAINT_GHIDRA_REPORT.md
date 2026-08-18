# Skirmish OwnerDraw ListBox `0x00618D40` Row Paint - Ghidra Research Report

**Address(es):** `OwnerDraw_ListBox_00618D40 @ 0x00618D40`, hook setup `FUN_0060F9A0 @ 0x0060F9A0`, Choose Map modal entry `0x005E68A0`, accept path `0x005E7160`, mode-list population `0x005D6130`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Real owner-drawn `LISTBOX` controls that use `OwnerDraw_ListBox_00618D40`, especially Choose Map modal controls `0x6EB` and `0x553`: row height, selected row fill, text inset, content width after scrollbar creation, custom hit-test/top-index behavior, and standard YR liveness.  
**Non-Scope:** `ComboDropWin` popup row paint except to distinguish it from real `LISTBOX`; full scrollbar drag/repeat internals already covered by scrollbar reports; Choose Map preview refresh and list population predicates beyond evidence needed for liveness.  
**Confidence:** High for real LISTBOX callback assignment, row-height formula, basic text row paint, selected fill extent, scrollbar/content shrink, and custom `0x4E8` hit testing. Medium for final screenshot RGB because no runtime capture was taken, and for non-text/custom-column row variants outside the Choose Map path.  
**Active in YR:** Yes. The Choose Map modal resource `0x6B` contains real `LISTBOX` controls `0x6EB` and `0x553`; common shell setup subclasses `"ListBox"` to `0x00618D40`; modal accept reads those controls with `LB_GETCURSEL` / `LB_GETITEMDATA`.

## Working Notes Gate

Target question: What exactly does `OwnerDraw_ListBox_00618D40` do for real owner-drawn LISTBOX row paint in the Choose Map modal, and what must Rust reproduce?

Non-goals: Do not reconstruct `ComboDropWin` popup row paint beyond the boundary fact that it is separate; do not implement Rust; do not re-open random-map or preview-refresh behavior.

Evidence needed to mark COMPLETE: prior-doc and Rust scans, read-only Ghidra/decompile evidence for hook setup and Choose Map liveness, read-only disassembly/decompile evidence for `0x00618D40` init/paint/hit-test/scrollbar blocks, and an implementation handoff naming Rust surfaces/tests.

Stop conditions: Ghidra requires mutating function-boundary repair, evidence expands into combo popup or preview systems, or a branch is outside real Choose Map LISTBOX behavior and can be explicitly deferred.

## 1. Overview

`OwnerDraw_ListBox_00618D40` is the live owner-draw callback for real shell `LISTBOX` windows. For Choose Map modal `0x6B`, controls `0x6EB` and `0x553` are not combo boxes and are not `ComboDropWin`; they are native listbox child windows subclassed by the common owner-draw hook.

For the basic text rows used by the modal, the callback draws the list frame, optionally fills the full selected item rectangle, then draws text with a two-pixel left inset. The visible row height is not the current Rust constant `16`; the callback initialization sets listbox item height from the active shell font measurement plus `2`.

## 2. Class Layout / Key Offsets

Offsets are from the owner-draw state pointer looked up for the HWND.

| Offset | Purpose | Evidence | Active in YR |
|---:|---|---|---|
| `+0x0C` | child scrollbar HWND / scrollbar-created marker | creation/read blocks `0x0061BBF8..0x0061BE42`; custom `0x4EF` returns it at `0x0061BBDC` | Conditional; only when rows overflow |
| `+0x34` | linked list of owner-draw row/item records for cleanup | destroy/remove blocks `0x00619020..0x00619074`, `0x00619ED4..0x00619F34` | Yes for owner-draw list item storage |
| `+0xE8` | optional per-row color table pointer; fallback text color is used when absent | paint color block `0x00619A30..0x00619A65` | Conditional; not required for ordinary Choose Map text rows |
| `+0xF0` | top index used by custom hit-test and scrollbar sync | `0x0061BBB7..0x0061BBC6`; scrollbar setup `0x00618F6B..0x00618FB0`, `0x0061BD57..0x0061BD97` | Yes when scrolled |
| `+0xF8` | optional custom row/column descriptor pointer | paint branch `0x0061943B..0x006194E8`; null path falls to basic text paint `0x00619A30` | Conditional; no evidence required for `0x6B` basic text rows |
| `+0x1E8` | stored callback/notify target from init message parameter | init block `0x006191BD..0x0061921B` | Yes for subclass init bookkeeping |

## 3. Core Logic

### Hook Assignment And Liveness

Active in YR: Yes.

`FUN_0060F9A0` reads each child class name and assigns `"ListBox"` to `OwnerDraw_ListBox_00618D40`, with kind `4`, then installs the shared subclass thunk and sends init message `0x497`. Evidence: decompile `0x0060F9A0`, class comparison block selecting `OwnerDraw_ListBox_00618D40`.

The Choose Map modal is standard YR UI: `0x005E68A0` creates dialog resource `0x6B`, calls common shell setup, sends `0x4A9`, shows the modal, and pumps it. Resource `0x6B` contains `0x6EB` and `0x553` as real `LISTBOX` controls with style `0x50000151`. Accept path `0x005E7160` reads `0x553` and `0x6EB` with listbox messages `0x188` and `0x199`. Evidence: decompile `0x005E68A0`, decompile `0x005E7160`, prior resource extraction report.

### Row Height

Active in YR: Yes.

On init message `0x497`, the listbox callback measures the active list font/text height, adds `2`, masks it to 16 bits, and sends `LB_SETITEMHEIGHT` (`0x1A0`) with `wParam = -1`. Evidence: disassembly `0x006191BD..0x0061920A`.

Prior combo/dropdown geometry reports identify the shell font as `g_GAME_FNT` and standard `GAME.FNT` cell height as `17` px. Combining that with this listbox init block gives a standard YR list row height of `19` px for these owner-drawn listboxes. This is an inference from the verified formula plus prior font evidence; the binary formula is the authoritative claim.

For Choose Map listbox rectangles `130 x 211`, `19` px rows produce 11 full visible rows with a 2 px bottom remainder. The callback does not use the current Rust `16` px constant.

### Paint Order For Basic Text Rows

Active in YR: Yes for basic text list rows; Conditional for custom row descriptor variants.

The `WM_PAINT` path:

1. draws a primitive 2-pixel list frame through `FUN_006208F0` at `0x0061926B`;
2. reads `LB_GETCOUNT` (`0x18B`) and `LB_GETTOPINDEX` (`0x18E`);
3. iterates from top index upward;
4. obtains each visible item rectangle with `LB_GETITEMRECT` (`0x198`);
5. stops painting when the item rectangle bottom would exceed the current client height;
6. for the basic text path, optionally fills selected row background, then draws text.

Evidence: read-only local disassembly of `0x00619230..0x00619B58`, frame call `0x00619255..0x0061926B`, count/top/item-rect calls `0x00619300..0x00619352`, row-bottom break `0x00619361..0x00619371`, basic text path `0x00619A30..0x00619B42`.

### Selected Row Fill

Active in YR: Yes.

Selection is tested with `LB_GETSEL` (`0x187`) for the row being painted. If selected, the callback fills the whole item rectangle translated into the destination surface; it does not inset or merely outline the selected row. The fill happens before text drawing.

Evidence: selected test and fill block `0x00619A65..0x00619AD0`; selected color conversion source `DAT_00AC4604` around `0x00619270..0x006192CF`; surface fill call through vtable `+0x14` at `0x00619AC6..0x00619AD0`.

Player-visible implication: the blue/selection fill spans the full content row width that remains after any scrollbar shrink, from the row's left edge to right edge and for the full row height.

### Text Inset And Text Color

Active in YR: Yes for `0x6EB`/`0x553` basic rows.

For the basic text path, the text rectangle starts at item-left plus `2` pixels. The top is the item top, and height is the item height. The right/bottom values are derived from the item rectangle dimensions, not a hardcoded row width. Text is converted into the draw scratch buffer, measured, truncated until it fits the available width, and rendered through `FUN_006211D0`.

Evidence: text rect setup `0x00619AD3..0x00619B1C`; conversion and text call `0x00619B23..0x00619B42`; truncation loop `0x00619941..0x006199AC`; earlier string extraction through item-data wrapper `0x006193D3..0x00619438`.

Text color normally falls back to owner-draw global `DAT_00AC18A4`; if a per-row color table at state `+0xE8` exists and the row value is not `-1`, that row color is used. If `WS_DISABLED` is set, text color is forced to `DAT_00AC1CB4`. Evidence: `0x00619A30..0x00619A65`.

### Scrollbar And Content Width

Active in YR: Conditional; active when item count exceeds visible row capacity.

The callback computes the owner-draw scrollbar width as `DAT_00AC1DF0 * 2 + 0x12`. `FUN_0060F9A0` initializes `DAT_00AC1DF0 = 1`, so standard width is `20` px. Evidence: init write in `0x0060F9A0`; listbox width calculation `0x00618E38..0x00618E4C`.

When rows overflow, it creates a child class `"Scrollbar"` with style `0x50010001`, subclasses it through `FUN_0060F9A0`, links the parent listbox into the scrollbar state, clears scrollbar grey byte `+0xCD`, sends custom `0xE9` with range/page/current top values, and resizes/repositions list content so rows no longer draw under the scrollbar. Evidence: creation/setup block `0x0061BBF8..0x0061BE42`, scrollbar sync block `0x00618F08..0x00618FB0`, update block `0x0061BD57..0x0061BD97`.

For `0x6EB` and `0x553`, the scrollbar appears only if the actual row count exceeds the capacity implied by `LB_GETITEMHEIGHT` and client height. With standard `19` px row height and `211` px list height, overflow starts above 11 full visible rows.

### Custom Hit-Test / Top Index

Active in YR: Conditional; active when callers send custom message `0x4E8`.

The custom `0x4E8` handler decodes `x = LOWORD(lParam)` and `y = HIWORD(lParam)`, rejects coordinates where `x >= client_width` or `y >= client_height`, reads `LB_GETITEMHEIGHT` (`0x1A1`) and `LB_GETCOUNT` (`0x18B`), then returns:

```text
state.top_index + y / item_height
```

If that index is negative or greater than `count - 1`, it returns `-1`. Evidence: disassembly `0x0061BB47..0x0061BBD9`.

Tiny edge detail: because coordinates are extracted into zero-extended 16-bit values, negative Windows coordinates encoded in `lParam` become large positive values and are rejected by the width/height comparisons rather than the signed `< 0` checks. Evidence: `0x0061BB47..0x0061BB7C`.

The handler uses total client height for the first bounds check, not "number of full rows * row height". If a list height has a bottom remainder, the remainder maps to the next `y / item_height` row when that row exists. This matters for custom hit tests; ordinary native listbox mouse selection may still be handled by the previous WndProc.

## 4. INI Keys

No INI key controls `OwnerDraw_ListBox_00618D40` row height, fill color, text inset, or scrollbar width. These are shell owner-draw constants/globals initialized in binary code.

| Source | Result | Active in YR |
|---|---|---|
| `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini` search for `LISTBOX`, `6EB`, `553`, Choose Map row geometry | no relevant row-paint keys found | Yes; binary UI path |
| `ini/mpmodesmd.ini` | mode/category data populates `0x6EB`, but not paint geometry | Yes for row content, not paint |

## 5. Integration Points

| Function / area | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_0060F9A0 @ 0x0060F9A0` | maps `"ListBox"` to `0x00618D40`, initializes owner-draw globals, sends `0x497` | decompile | Yes |
| `0x005E68A0` | creates Choose Map modal resource `0x6B` | decompile | Yes |
| `0x005D6130` | populates `0x6EB` rows and stores item data | decompile; sends add-string/custom list message and `0x19A` | Yes |
| `0x005E7160` | accept reads selected rows from `0x553` and `0x6EB` | decompile; listbox messages `0x188`/`0x199` | Yes |
| `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690` | paints/handles child scrollbar when list overflows | decompile; listbox creates `"Scrollbar"` child | Conditional |
| `ComboDropWin @ 0x0060D540` | standard combo popup row painter; not real LISTBOX callback | prior row report; class registration `0x0060D450` | Yes for combos, No for `0x6B` real LISTBOX |

## 6. Current Rust Implementation Status

Relevant Rust surfaces scanned:

| Rust surface | Current state |
|---|---|
| `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs` | `CHOOSE_MAP_LIST_ROW_H` is `16`; `choose_map_modal_list_row_at` divides by this fixed value and does not account for owner-draw `font_height + 2`, top index, or scrollbar width |
| `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs` | has modal selection/top-index state, but row rendering/input parity is incomplete |
| `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs` | no verified renderer for `0x6B` listbox frame, full-row selected fill, `+2` text inset, or native listbox scrollbar behavior |

No Rust files were modified by this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `OwnerDraw_ListBox_00618D40` real ListBox liveness | verified | `FUN_0060F9A0` decompile; `0x6B` resource report; `0x005E7160` decompile | none |
| Init row-height formula | verified | `0x006191BD..0x0061920A` | runtime screenshot can confirm final 19 px on a specific install/font |
| Basic text row paint | verified | `0x00619A30..0x00619B42` | none for Choose Map text rows |
| Selected row fill extent | verified | `0x00619A65..0x00619AD0` | exact final RGB screenshot optional |
| Text inset and truncation | verified | `0x00619AD3..0x00619B42`, `0x00619941..0x006199AC` | lower `FUN_006211D0` raster internals out-of-scope |
| Scrollbar width and content shrink | verified | `0x00618E38..0x00618E4C`, `0x0061BBF8..0x0061BE42`, `0x0061BD57..0x0061BD97` | full scrollbar drag/repeat behavior belongs to scrollbar report |
| Custom `0x4E8` hit-test | verified | `0x0061BB47..0x0061BBD9` | ordinary native mouse-selection WndProc path not re-derived |
| Custom row descriptor variants via state `+0xF8` | touched-not-exhausted | branch `0x0061943B..0x00619A18` | not needed for basic Choose Map listboxes unless future evidence shows they configure descriptors |
| `ComboDropWin` popup row paint | deferred | prior reports distinguish class and WndProc | out-of-scope except stale-doc wording |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is `OwnerDraw_ListBox_00618D40` active for real Choose Map controls? -> Yes, `0x6B` controls `0x6EB`/`0x553` are resource `LISTBOX` children and shell setup maps `"ListBox"` to `0x00618D40`.` (evidence: `0x0060F9A0`, `0x005E68A0`, resource `0x6B`, `0x005E7160`)
- `[RESOLVED] OQ-02 - Does this report cover combo popup rows? -> No; standard combo popup rows are owned by `ComboDropWin`, while this report covers real `LISTBOX` windows.` (evidence: `0x0060D450`, prior `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-03 - What row height does the real listbox use? -> Init sends `LB_SETITEMHEIGHT` with measured shell font/text height plus `2`; standard inferred value is `19` px from `GAME.FNT` height `17`.` (evidence: `0x006191BD..0x0061920A`; prior font report)
- `[RESOLVED] OQ-04 - Is selected fill inset? -> No, selected fill covers the full item rectangle before text.` (evidence: `0x00619A65..0x00619AD0`)
- `[RESOLVED] OQ-05 - Where does text start? -> Basic row text starts at item-left plus `2` px and uses the item rect height.` (evidence: `0x00619AD3..0x00619B42`)
- `[RESOLVED] OQ-06 - Does row paint draw under the scrollbar? -> No, overflow creates a child scrollbar and uses the shrunken content client for row paint/hit concepts.` (evidence: `0x00618E38..0x00618E4C`, `0x0061BBF8..0x0061BE42`)
- `[RESOLVED] OQ-07 - What is the scrollbar width? -> `DAT_00AC1DF0 * 2 + 0x12`; standard `DAT_00AC1DF0 = 1`, so `20` px.` (evidence: `0x0060F9A0`, `0x00618E38..0x00618E4C`)
- `[RESOLVED] OQ-08 - How does custom hit-test map a point to a row? -> It rejects points outside client bounds, then returns `top_index + y / LB_GETITEMHEIGHT`, capped against `LB_GETCOUNT - 1`.` (evidence: `0x0061BB47..0x0061BBD9`)
- `[RESOLVED] OQ-09 - Are INI keys involved in row paint geometry? -> No relevant INI keys found; geometry is binary owner-draw state.` (evidence: INI scan)
- `[RESOLVED] OQ-10 - Does `0x6EB` store mode display rows and item data? -> Yes, `0x005D6130` appends display rows and stores the MPModes object pointer as item data.` (evidence: decompile `0x005D6130`)
- `[RESOLVED] OQ-11 - Does accept depend on listbox item data? -> Yes, `0x005E7160` reads `0x553` and `0x6EB` with `LB_GETCURSEL`/`LB_GETITEMDATA`.` (evidence: `0x005E7160`)
- `[RESOLVED] OQ-12 - What happens when row item text is too wide? -> The callback measures and repeatedly truncates the scratch string before drawing.` (evidence: `0x00619941..0x006199AC`)
- `[RESOLVED] OQ-13 - Does disabled style affect text color? -> Yes, `WS_DISABLED` forces `DAT_00AC1CB4` for text.` (evidence: `0x00619A55..0x00619A65`)
- `[DEFERRED] OQ-14 - Full custom row descriptor branch at state `+0xF8`.` (category: `out-of-scope`; reason: Choose Map evidence reaches the basic text-row path and no `0x6B` setup evidence requires descriptor variants; next-step-if-pursued: trace senders that configure `+0xF8` and prove a live modal uses them)
- `[DEFERRED] OQ-15 - Native previous-WndProc mouse selection details for real listboxes.` (category: `requires-different-system-context`; reason: custom hit-test is resolved, but ordinary click selection may be handled by the previous Win32 listbox proc; next-step-if-pursued: trace `WM_LBUTTONDOWN` fallthrough/default proc and `LBN_SELCHANGE` notification in `0x005E6920`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Real Choose Map listboxes use owner-draw row height `font_height + 2`, standard inferred `19` px, not `16` | `0x006191BD..0x0061920A`; prior font evidence | mismatch: `CHOOSE_MAP_LIST_ROW_H = 16` | `src/ui/skirmish_shell/layout.rs` | derive or set owner-draw list row height from the shell font contract for `0x6B` listboxes | proposed test `choose_map_modal_listbox_row_height_matches_ownerdraw_font_plus_two`: a 211 px list exposes 11 full rows plus remainder, not 13 rows | Do not keep a convenient 16 px row grid |
| Selected row fill spans the full item rectangle before text | `0x00619A65..0x00619AD0` | missing/unchecked renderer | `src/app_skirmish_shell_render.rs` | draw full content-width selected fill for highlighted `0x6EB`/`0x553` rows | proposed test `choose_map_modal_listbox_selected_fill_is_full_row`: selected row fill begins at list content left and spans to content right | Do not inset or outline the selected row |
| Basic row text starts at item-left `+2` and is truncated before `FUN_006211D0` draw | `0x00619AD3..0x00619B42`, `0x00619941..0x006199AC` | missing/unchecked renderer | `src/app_skirmish_shell_render.rs`; text layout helpers | position modal list labels two pixels from row left and clip/truncate to row width | proposed test `choose_map_modal_listbox_text_uses_two_px_inset_and_clips`: long map names do not overflow into scrollbar/button area | Do not reuse ComboDropWin's `+3` text inset for real ListBox rows |
| Overflow creates a 20 px child scrollbar and shrinks row content width | `0x00618E38..0x00618E4C`, `0x0061BBF8..0x0061BE42` | missing | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | when rows exceed capacity, reserve 20 px and keep text/fill/hit content out of scrollbar column | proposed test `choose_map_modal_listbox_scrollbar_shrinks_row_content_width`: overflowing map list row fill/text stops before scrollbar | Do not draw text or selected fill under scrollbar art |
| Custom `0x4E8` hit-test is `top_index + y / item_height` after client-bounds checks | `0x0061BB47..0x0061BBD9` | mismatch if using fixed 16 px helper and no top index | `src/ui/skirmish_shell/layout.rs::choose_map_modal_list_row_at`, modal state hit testing | include top index and owner-draw item height in row mapping; reject scrollbar column if content is shrunken | proposed test `choose_map_modal_listbox_hit_test_uses_top_index_and_ownerdraw_height`: after scroll, clicking first visible row returns the top index, not row zero | Do not use viewport-local row index alone |

### Stale Docs / Follow-up Docs

- `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`: corrected to state that `ComboDropWin` owns standard combo popup row paint; `OwnerDraw_ListBox_00618D40` owns real owner-drawn `LISTBOX` controls such as Choose Map `0x6EB`/`0x553` and supplies real-listbox scrollbar/hit-test behavior.
- `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`: corrected to state that combo popup row paint is in `ComboDropWin`; real `LISTBOX` row paint is in `OwnerDraw_ListBox_00618D40`.
- `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`: replace "exact row internal paint deferred" with "Real `0x6B` listboxes use `OwnerDraw_ListBox_00618D40`: row height is font height + 2, selected fill is full-row, basic text inset is +2 px, overflow reserves a 20 px scrollbar."

## Sources

- Ghidra read-only decompile: `FUN_0060F9A0 @ 0x0060F9A0`.
- Ghidra read-only decompile: `FUN_005E68A0 @ 0x005E68A0`.
- Ghidra read-only decompile: `FUN_005E7160 @ 0x005E7160`.
- Ghidra read-only decompile: `FUN_005D6130 @ 0x005D6130`.
- Ghidra read-only decompile: `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`.
- Ghidra/read-only local disassembly from `gamemd.exe`: `OwnerDraw_ListBox_00618D40` ranges `0x00618E38..0x00618FB0`, `0x006191BD..0x0061920A`, `0x00619230..0x00619B58`, `0x0061BB47..0x0061BBD9`, `0x0061BBF8..0x0061BE42`.
- Prior docs referenced: `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODE_CATEGORY_0X6EB_GHIDRA_REPORT.md`, `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`.
- INI files checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`, `rulesmd.ini`, `art.ini`, `artmd.ini`, `mpmodesmd.ini`.
- Rust surfaces scanned: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`.
