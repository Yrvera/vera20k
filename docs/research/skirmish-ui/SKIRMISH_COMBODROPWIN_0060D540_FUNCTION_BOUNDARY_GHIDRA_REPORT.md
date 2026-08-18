# Skirmish ComboDropWin `0x0060D540` Function Boundary - Ghidra Research Report

**Address(es):** `0x0060D450` registration, `0x0060D540..0x0060F311` registered `ComboDropWin` WndProc block  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR offline Skirmish combo popup behavior owned by the registered `ComboDropWin` WndProc: recoverable function boundary, row paint ownership, selected row fill/text/swatch geometry, hit-test/top-index behavior, child-scrollbar bridge, and Rust implementation handoff.  
**Non-Scope:** normal `LISTBOX` row renderer `OwnerDraw_ListBox_00618D40` except contrast, Skirmish combo population semantics already covered by combo/color reports, full Win32 lifetime modeling beyond observable popup behavior.  
**Confidence:** High for registration address, active YR reachability, function-boundary range, row paint ownership, hit-test/top-index math, selected row/text geometry, and scrollbar delegation; Medium for exact internal state-field names because Ghidra has no function at `0x0060D540` and this pass used read-only assembly context rather than creating a function boundary.  
**Active in YR:** Yes. `FUN_0060D450` registers class `"ComboDropWin"` with WndProc address `0x0060D540`, and `OwnerDraw_ComboBox_00617250` creates that class for open Skirmish combo dropdowns.

## Working Notes

Target question: What behavior is actually inside the registered `ComboDropWin` WndProc block at `0x0060D540`, and what does Rust need to reproduce without modeling Win32?
Non-goals: Normal real `LISTBOX` `OwnerDraw_ListBox_00618D40` internals beyond contrast, combo collapsed paint, combo population order, and Choose Map modal listboxes.
Evidence needed to mark COMPLETE: Read-only Ghidra evidence for registration, boundary/exits, paint/input/top-index/scrollbar cases, prior-doc conflict check, Rust surface comparison, and final report at this path only.
Stop conditions: Open questions resolved or explicitly deferred, no Ghidra mutations, no Rust edits, zero-add review pass over the WndProc assembly contexts.

## 1. Overview

The custom combo popup is not painted by the real Win32 `LISTBOX` owner-draw callback. `ComboDropWin` is its own registered window class, and its WndProc block contains the dropdown row paint loop, hover/selection handling, custom hit-test, top-index clamp, and creation/synchronization of a child owner-drawn `Scrollbar` when the popup needs scrolling.

For Rust, this means a clean popup model is acceptable: keep item lists and selection in Rust state, paint rows directly, shrink the content width when a scrollbar exists, and emulate the small set of observable messages/math rather than representing the original hidden popup HWND/state table.

## 2. Key Offsets / State Fields

The WndProc's state object is recovered through the shell owner-draw hash tables. Ghidra labels are missing, so names below are semantic labels from observed reads/writes.

| Field / source | Verified purpose | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00AC48D0` | Current popup/source combo HWND used by the WndProc while dispatching | WndProc setup and repeated `SendMessageA` calls at `0x0060D728..0x0060D773`, `0x0060E529..0x0060E57F` | Yes |
| popup state `+0x0C` | Child scrollbar HWND, zero when no scrollbar exists | create path writes at `0x0060E740`; mouse path tests/forwards through `0x0060E4B3..0x0060E4E2` | Conditional: only overflow popups |
| popup state `+0x10` | Cached popup backing `BSurface` pointer | paint allocation/store at `0x0060D8CD..0x0060D95C` | Yes while popup paints |
| popup state `+0xE8` | Current hot/selected row inside popup | paint reads at `0x0060D8A0`; mousemove/select writes around `0x0060E294..0x0060E2AC`; initial sync at `0x0060F276..0x0060F283` | Yes |
| popup state `+0xF0` | Top visible item index | paint loop starts from it at `0x0060DBF3..0x0060DC03`; getter `0x15B` at `0x0060E215`; setter/clamp `0x15C` at `0x0060E40C..0x0060E48A` | Conditional: nonzero only after scrolling |
| source combo state `+0xCC` | Swatch drawing enabled for popup rows | row paint guard `0x0060DE83..0x0060DE8F` | Conditional: color/start/team families that enable swatches |
| source combo state `+0xCD` | Grey/alternate state copied to popup rows and child scrollbar | row color gate `0x0060DE50..0x0060DE5A`; scrollbar state copy `0x0060E7AA..0x0060E81C` | Conditional: restricted/grey combo rows |
| source combo state `+0x110 + row*4` | Per-row swatch color slots | row loop pointer setup at `0x0060DC1C`; swatch read path `0x0060DE73..0x0060DF2D` | Conditional: swatch-enabled combo rows |

## 3. Function Boundary

`FUN_0060D450` registers two classes. The first class is `"ComboDropWin"` at string `0x008357A0`, and the `WNDCLASS` WndProc field is written with `0x0060D540` at `0x0060D49E`. The second class uses WndProc `0x0060D520`.

Ghidra does not have a function object at `0x0060D540` in this project, so no decompiler output was available for the primary WndProc without mutating the database. Read-only assembly shows:

- prologue begins at `0x0060D540` with a shell-global init flag check and `SUB ESP,0x37C`;
- standard WndProc stack cleanup exits use `RET 0x10`;
- observed exits include `0x0060E0D5`, `0x0060E15B`, `0x0060E22D`, `0x0060E409`, `0x0060E496`, `0x0060E5EB`, `0x0060E624`, `0x0060F273`, `0x0060F294`, and `0x0060F311`;
- next ordinary function begins at `0x0060F320`.

Verified boundary: `0x0060D540..0x0060F311` is the recoverable registered WndProc block for `ComboDropWin`. Active in YR: Yes, because the combo callback creates this class while opening standard shell combo dropdowns.

## 4. Core Logic

### Paint / Row Ownership

Active in YR: Yes. Evidence: `WM_PAINT` branch at `0x0060D846`, source combo `CB_GETCOUNT`/`CB_GETITEMHEIGHT` queries at `0x0060D84C..0x0060D870`, row loop and text call at `0x0060DBF3..0x0060DFC8`.

`ComboDropWin` paints rows itself. It is not just a forwarding shell around `OwnerDraw_ListBox_00618D40`.

Paint behavior:

1. Queries the source combo item count with `0x146` and item height with `0x154`.
2. Computes visible capacity from popup client height divided by item height.
3. Determines whether overflow exists by checking whether `item_count * item_height` is greater than the popup height.
4. Allocates/reuses a cached popup surface if state `+0x10` is null.
5. Restores/copies the parent/background surface, draws primitive frame rectangles, then loops visible rows.
6. Starts row iteration at state `+0xF0` and stops before the visible end; row Y is `(row_index - top_index) * item_height`.
7. Draws a selected/hot row fill when the iterated row equals state `+0xE8`.
8. Draws optional swatch color when the source combo state has swatches enabled and the per-row color slot is not negative.
9. Truncates row text until `BitFont__GetTextWidth <= client_width - 0x14`.
10. Calls `FUN_00621040` with flags `0x04` for vertical centering.

Tiny details:

- row text left inset is `+3` pixels (`0x0060DE25`);
- text row top/bottom are the current row top and `row_top + item_height`;
- the text width limit subtracts `0x14`, matching the scrollbar/arrow reserved width (`0x0060DF2D..0x0060DF3C`);
- text truncation removes one UTF-16 code unit at a time until it fits (`0x0060DF63..0x0060DFA3`);
- normal text color comes from `DAT_00AC18A4`; grey/alternate rows use `DAT_00AC1CB0` when the source combo state `+0xCD` is set (`0x0060DE1F..0x0060DE60`);
- swatch color conversion uses the same DirectDraw channel shift/loss globals as collapsed combo swatches (`0x0060DED3..0x0060DF19`).

### Scrollbar Creation / Synchronization

Active in YR: Conditional. It triggers only when item count exceeds visible row capacity. Evidence: overflow branch `0x0060E648..0x0060E821`; child scrollbar style/class creation `0x0060E721..0x0060E745`; sync messages `0x0060D7B2..0x0060D7F2`, `0x0060E17B..0x0060E1B9`, `0x0060E40C..0x0060E48A`.

When overflow exists and no scrollbar child is already stored, `ComboDropWin` creates a `"Scrollbar"` child with style `0x50010001`, calls `FUN_0060F9A0` to install owner-draw scrollbar behavior, sends custom `0xE9` with a 0x1C-byte scroll-info struct, copies the source combo grey flag into the scrollbar via `0x4F1`, and draws popup rows with a content width reduced by the scrollbar column.

Top-index behavior:

- getter `0x15B` returns popup state `+0xF0` (`0x0060E215..0x0060E22D`);
- setter `0x15C` clamps caller value to `[0, item_count - visible_capacity]`, writes state `+0xF0` only if changed, then invalidates the popup (`0x0060E40C..0x0060E48A`);
- scroll child change handling reads the child scrollbar value with `0xE1`, compares it to `0x15B`, and sends `0x15C` back to the popup if different (`0x0060E17B..0x0060E1B9`);
- no Rust implementation needs to model those messages if it preserves the same clamped top-index contract.

Scrollbar visual/drag details are delegated to the shell owner-draw scrollbar callback after `FUN_0060F9A0`; prior verified scrollbar facts remain applicable: arrow button height `0x16` (22), minimum thumb height `0x0E` (14), and thumb/top mapping based on current position over max. Active in YR: Conditional on overflow; evidence `OwnerDraw_ScrollBar_0061C690` prior report and child creation at `0x0060E745`.

### Hit-Test

Active in YR: Yes while popup exists. Evidence: custom `0x4E8` branch at `0x0060F297..0x0060F307`; callers include mousemove forwarding at `0x0060E262..0x0060E28B` and combo callback hit-test forwarding in prior combo report.

Custom message `0x4E8` interprets `lParam` as packed client coordinates. It returns `-1` when the point is outside `[0,width) x [0,height)`. Otherwise it returns:

```text
min(item_count - 1, top_index + y / item_height)
```

Important boundaries:

- X equal to width is outside;
- Y equal to height is outside;
- valid row index is capped at `item_count - 1`;
- negative packed coordinates become outside through the same comparisons after low/high word extraction.

### Mouse Input / Selection

Active in YR: Yes. Evidence: `WM_MOUSEMOVE 0x200` branch `0x0060E262..0x0060E3FD`; `WM_LBUTTONDOWN 0x201` and `WM_LBUTTONDBLCLK 0x203` branch `0x0060E499..0x0060E624`.

Mouse behavior:

- mousemove calls the popup's own `0x4E8` hit-test; if the hot row changes, state `+0xE8` is updated and the popup is invalidated;
- clicks in the scrollbar column are forwarded to the child scrollbar HWND instead of selecting a row;
- row click computes `top_index + y / item_height`, clamps it to the last item, sends `CB_SETCURSEL 0x14E` to the source combo, releases capture, closes the dropdown through `0x14F`, and sends parent `WM_COMMAND 0x111` with notification code `1` in the high word;
- clicks that miss selectable content close the dropdown without changing the source combo selection.

The Rust-visible result is "select row and close", "scrollbar consumes input and may update top index", or "outside closes", not the original HWND choreography.

## 5. INI Keys

No INI keys are read by this WndProc. It consumes already-populated combo rows, item data, swatch slots, source combo grey state, and source combo max-row/geometry results supplied by shell code.

## 6. Integration Points

| Function / address | Role | Active in YR |
|---|---|---|
| `FUN_0060D450 @ 0x0060D450` | registers `"ComboDropWin"` with WndProc `0x0060D540` | Yes |
| `OwnerDraw_ComboBox_00617250 @ 0x00617250` | creates `ComboDropWin` and passes source combo HWND as create param on open | Yes |
| `ComboDropWin WndProc @ 0x0060D540..0x0060F311` | popup row paint, hit-test, top-index, child scrollbar bridge, selection close | Conditional: only while combo popup is open |
| `FUN_0060F9A0 @ 0x0060F9A0` | hooks child `"Scrollbar"` created by popup | Conditional: only overflow popup |
| `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690` | paints/drives child scrollbar after hook setup | Conditional: only overflow popup |

## 7. Current Rust Implementation Status

Rust already models the main observable popup behavior without Win32:

- `src/ui/skirmish_shell/layout.rs:18` defines `COMBO_FACE_H = 24`, `COMBO_DROPDOWN_ROW_H = 23`, scrollbar width `20`, arrow button height `22`, and min thumb height `14`.
- `src/ui/skirmish_shell/state.rs:572` computes popup rect directly below the combo face.
- `src/ui/skirmish_shell/state.rs:666` shrinks content width by scrollbar width when overflow exists.
- `src/ui/skirmish_shell/state.rs:696` maps top index to thumb Y; `state.rs:781` maps track clicks proportionally back to top index.
- `src/ui/skirmish_shell/state.rs:1052` handles dropdown row clicks, scrollbar clicks, drag setup, close, and selection.
- `src/app_skirmish_shell_render.rs:909` draws popup background, selected row fill, color swatches, scrollbar, and border.
- `src/app_skirmish_shell_render.rs:1629` draws row text with `content.x + 3`, row height, and vertical centering.

Observed deltas / risks:

- Rust currently uses a constant row height (`23`) rather than querying a source combo item height; this is probably correct for standard Skirmish after prior combo-init verification, but the test should assert it as the item-height-derived value, not as arbitrary art height.
- Rust draws color swatches as `content.x + 2`, `row_y + 2`, `content.w - 4`, `19`; this matches the broad row-fill shape but should stay tied to verified popup row swatch behavior from `0x0060DE73..0x0060DF2D`.
- Rust has a proportional top-index mapping and clamped drag state; this matches the recovered `0x15C` clamp contract well enough without modeling child `Scrollbar` messages.
- Rust closes immediately on row click and pushes a UI sound. Native close/notification goes through source combo `0x14F` plus parent `WM_COMMAND`; ensure command effects, not Win32 sequence, are the acceptance target.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0060D450` registration | verified | assembly `0x0060D49E`, string `0x008357A0` | none |
| `0x0060D540` function boundary | verified-read-only | prologue `0x0060D540`, exits through `0x0060F311`, next function `0x0060F320` | optional future Ghidra function creation if mutation is approved |
| `WM_PAINT` row paint | verified | `0x0060D846..0x0060DFC8` | final screenshot color validation |
| row text rect/truncation | verified | `0x0060DE1F..0x0060DFC8` | none for implementation handoff |
| swatch row fill | verified | `0x0060DE73..0x0060DF2D` | exact final RGB after display format remains screenshot/runtime concern |
| scrollbar child creation | verified | `0x0060E648..0x0060E821` | full scrollbar callback internals not repeated |
| top-index getter/setter | verified | `0x0060E215..0x0060E22D`, `0x0060E40C..0x0060E48A` | none |
| custom hit-test `0x4E8` | verified | `0x0060F297..0x0060F307` | none |
| mousemove hot row | verified | `0x0060E262..0x0060E3FD` | hover rendering in Rust currently unchecked |
| click selection/close | verified | `0x0060E499..0x0060E624` | exact UI sound timing was not investigated |
| normal `OwnerDraw_ListBox_00618D40` contrast | deferred | non-scope by user request | separate slot for real LISTBOX rows |

## 9. Open Questions - Final State

- `[RESOLVED] OQ1 - Is `0x0060D540` active in standard YR Skirmish? -> Yes; class registration stores it as `"ComboDropWin"` WndProc and combo open creates that class.` (evidence: `0x0060D49E`, prior combo open report `0x00617Fxx..0x00618481`)
- `[RESOLVED] OQ2 - Does Ghidra have a function boundary at `0x0060D540`? -> No; read-only decompile fails, but assembly gives boundary `0x0060D540..0x0060F311`.` (evidence: Ghidra no-function result; assembly `0x0060D540`, `0x0060F311`, `0x0060F320`)
- `[RESOLVED] OQ3 - Who owns combo popup row paint? -> `ComboDropWin` WndProc itself, not `OwnerDraw_ListBox_00618D40`.` (evidence: paint loop `0x0060D846..0x0060DFC8`)
- `[RESOLVED] OQ4 - What row height does popup use? -> Source combo `CB_GETITEMHEIGHT 0x154`.` (evidence: `0x0060D863..0x0060D874`, `0x0060E529..0x0060E554`, `0x0060F2C2..0x0060F2E7`)
- `[RESOLVED] OQ5 - What is the row text rect? -> left `+3`, row top/bottom from `item_height`, current content right, vertical-center flags `0x04`.` (evidence: `0x0060DE1F..0x0060DFC8`)
- `[RESOLVED] OQ6 - Does row text avoid the scrollbar? -> Yes; text-fit width subtracts `0x14` and content right reflects scrollbar shrink.` (evidence: `0x0060DF2D..0x0060DF3C`; scrollbar branch `0x0060E648..0x0060E821`)
- `[RESOLVED] OQ7 - How is top index clamped? -> custom `0x15C` clamps to `[0, item_count - visible_capacity]`, then invalidates if changed.` (evidence: `0x0060E40C..0x0060E48A`)
- `[RESOLVED] OQ8 - What does `0x4E8` return? -> `-1` outside, else `min(item_count - 1, top_index + y / item_height)`.` (evidence: `0x0060F297..0x0060F307`)
- `[RESOLVED] OQ9 - Does popup create the scrollbar itself? -> Yes, class `"Scrollbar"` style `0x50010001`, then `FUN_0060F9A0`, `0xE9`, and `0x4F1`.` (evidence: `0x0060E721..0x0060E821`)
- `[RESOLVED] OQ10 - Can Rust implement this without Win32? -> Yes; model popup rows, top index, content shrink, and selection/close effects directly.` (evidence: binary behavior above; Rust surfaces `state.rs`, `app_skirmish_shell_render.rs`)
- `[DEFERRED] OQ11 - Exact child scrollbar drag/repeat timer timing.` (category: out-of-scope; reason: popup delegates to owner-draw scrollbar callback and this target was the ComboDropWin boundary; next-step-if-pursued: audit `OwnerDraw_ScrollBar_0061C690` timing if scrollbar feel mismatches retail)
- `[DEFERRED] OQ12 - Real `LISTBOX` `OwnerDraw_ListBox_00618D40` row geometry.` (category: out-of-scope; reason: explicitly non-goal beyond contrast; next-step-if-pursued: slot 2 listbox row report)

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `ComboDropWin` owns combo-popup row paint; text rect is `content.x + 3`, row height from source combo item height, and text fit excludes `0x14` scrollbar/arrow width | `0x0060D846..0x0060DFC8`; registration `0x0060D49E` | mostly none observed; keep constants tied to item-height proof | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs::push_dropdown_instances`, dropdown text loop | Paint popup rows directly with `left+3`, vertical center, and content shrink when scrollbar exists | `skirmish_combodropwin_row_text_rect_uses_left3_and_scrollbar_shrink` | Do not route combo popup rows through real LISTBOX renderer or reuse Choose Map listbox row geometry |
| Top index is a clamped popup field; setter clamps to `[0, item_count - visible_capacity]`, hit-test returns `top + y / row_h` capped to last row | `0x0060E40C..0x0060E48A`; `0x0060F297..0x0060F307` | none observed; Rust proportional helpers are compatible | `src/ui/skirmish_shell/state.rs::set_open_combo_top_index`, `handle_combo_mouse_down`, wheel/drag helpers | Preserve clamped top-index semantics and row hit-test boundaries | `skirmish_combodropwin_hit_test_caps_to_last_item_and_rejects_edges` | Do not page-scroll on track clicks if proportional top-index mapping is already intended |
| Overflow popup creates/uses a child scrollbar, but Rust only needs the observable geometry and value mapping: content width shrinks by `0x14`, arrows are 22px, thumb min is 14px, top index syncs through `0x15C` | `0x0060E648..0x0060E821`; prior `OwnerDraw_ScrollBar_0061C690` constants | mostly none observed | `src/ui/skirmish_shell/state.rs::combo_dropdown_content_rect`, `combo_dropdown_scroll_thumb_rect`, `top_index_from_*`, `src/app_skirmish_shell_render.rs::push_dropdown_scrollbar_instances` | Keep scrollbar drawing/input as direct state updates; no HWND child needed | `skirmish_combodropwin_scrollbar_shrinks_content_and_clamps_thumb_top_index` | Do not model Win32 child windows just to reproduce popup scrolling |

Proposed Rust test names:

- `skirmish_combodropwin_row_text_rect_uses_left3_and_scrollbar_shrink`
- `skirmish_combodropwin_hit_test_caps_to_last_item_and_rejects_edges`
- `skirmish_combodropwin_top_index_clamps_to_count_minus_visible`
- `skirmish_combodropwin_scrollbar_shrinks_content_and_clamps_thumb_top_index`
- `skirmish_combodropwin_color_swatch_stays_inside_content_row`

## 11. Negative Facts / Do Not Do

- Do not say combo popup rows are painted by `OwnerDraw_ListBox_00618D40`; the registered `ComboDropWin` WndProc contains the popup paint loop. Evidence: `0x0060D49E`, `0x0060D846..0x0060DFC8`.
- Do not reuse Choose Map `0x6B` real listbox row geometry for combo dropdown rows; combo popup is a separate custom window class. Evidence: `ComboDropWin` string/class registration and custom `0x4E8` branch.
- Do not implement popup scrolling as page-up/page-down track clicks if it conflicts with the clamped/proportional top-index contract; the popup syncs a scalar top index through `0x15C`. Evidence: `0x0060E40C..0x0060E48A`.
- Do not let row text draw under the scrollbar; the row text fit path reserves `0x14` and the popup creates a scrollbar column when overflow exists. Evidence: `0x0060DF2D..0x0060DFC8`, `0x0060E648..0x0060E821`.
- Do not model original HWNDs/state hash tables in Rust unless needed for another reason; the observable outputs are row paint, hit-test, top-index, scrollbar geometry, selection, and close notification effects.

## 12. Remaining Uncertainty

- Exact child scrollbar repeat timing and pressed-arrow transient state were not re-audited here because `ComboDropWin` delegates that to `OwnerDraw_ScrollBar_0061C690`. This is not material for the row paint/top-index handoff unless retail scrollbar feel still mismatches after geometry/state parity.
- Ghidra still lacks an actual function object at `0x0060D540`; the boundary is recovered from read-only assembly and registration evidence. A future mutating Ghidra cleanup could create the function, but this report intentionally did not.

## 13. Stale Docs / Follow-up Docs

Replacement wording for stale combo-popup claims:

- Replace "combo dropdown rows are owned by `OwnerDraw_ListBox_00618D40`" with: "standard combo dropdown rows are painted and hit-tested by the registered `ComboDropWin` WndProc block at `0x0060D540..0x0060F311`; real `LISTBOX` controls use `OwnerDraw_ListBox_00618D40` separately."
- Replace "combo popup reuses listbox row geometry" with: "combo popup row text starts at `content.x + 3`, uses source combo item height, truncates to current content width minus `0x14`, and uses `FUN_00621040` with vertical-centering flag `0x04`."
- Replace "dropdown scrollbar behavior belongs to the listbox callback" with: "`ComboDropWin` creates and syncs its own child `Scrollbar` when overflow exists; the child scrollbar callback owns thumb/arrow painting, while `ComboDropWin` owns top-index clamp and content shrink."

## Sources

- Ghidra read-only assembly context: `FUN_0060D450 @ 0x0060D450`.
- Ghidra read-only assembly context: registered `ComboDropWin` WndProc block `0x0060D540..0x0060F311`.
- Ghidra read-only decompile/check: `FUN_0060D320` adjacent owner-draw state lookup helper.
- Prior docs checked: `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`.
- Prior docs checked: `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`.
- Prior docs checked: `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`.
- Prior docs checked: `docs/research/skirmish-ui/SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/ui/skirmish_shell/layout.rs`.
- Rust read-only scan: `src/ui/skirmish_shell/state.rs`.
- Rust read-only scan: `src/app_skirmish_shell_render.rs`.
