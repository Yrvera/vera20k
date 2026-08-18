# Skirmish Combo Dropdown Listbox Assembly - Ghidra Research Report

**Address(es):** `FUN_0060D450 @ 0x0060D450`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `ComboDropWin WndProc block @ 0x0060D540..0x0060F311`, `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`, common thunk `0x00610CA0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard offline YR Skirmish dialog `0x102` owner-draw combo dropdown/listbox assembly at the full-shell level: collapsed face vs open dropdown, row/list/scrollbar composition, sibling-window z-order over other child controls, hit rect/input routing, and current Rust fit.
**Non-Scope:** redoing settled combo item-data semantics, real Choose Map `LISTBOX` internals, final screenshot RGB capture, shell-wide modal flow, INI/gameplay launch behavior, Rust implementation.
**Confidence:** High for binary create/show/capture, row/list/scrollbar assembly, and Rust source comparison; Medium for final z-order wording because it combines direct USER32 calls with standard child-window z-order semantics rather than a runtime screenshot.
**Active in YR:** Yes. Standard offline Skirmish `0x102` installs owner-draw callbacks via `FUN_0060F9A0`, combo controls use `OwnerDraw_ComboBox_00617250`, and open dropdowns create the registered `ComboDropWin` class.

## 0. Working Gate

Target question: do Skirmish `0x102` open combo dropdowns assemble over the full shell like retail, including row/list/scrollbar pieces and input routing, and does current Rust fit that assembly?

Non-goals: do not rediscover settled per-control geometry except to resolve assembly contradictions; do not inspect Choose Map listbox rows; do not mutate Rust, INI, in-repo docs, Ghidra state, or claims files.

Evidence needed to mark COMPLETE: prior combo/dropdown docs scanned; current dirty Rust render/input surfaces scanned; read-only Ghidra decompile plus assembly contexts for `ComboDropWin` registration, combo `CreateWindowExA`/show/capture/parent notify, popup row/scrollbar assembly, and no direct wheel callback; stale-doc scan for outdated Rust/dropdown claims.

Stop conditions: stop if Ghidra function-boundary creation would be required; record material missing runtime screenshot facts as Remaining Uncertainty; write only this report.

## 1. Overview

Retail does not draw an in-place egui-style menu inside the combo face. The collapsed combo is a normal child combo subclassed by `OwnerDraw_ComboBox_00617250`; opening it creates a separate `WS_CHILD` window of class `"ComboDropWin"` as a sibling under the combo's parent dialog, one pixel below the 24 px collapsed face. The new child owns row paint, hit testing, top index, and optional child scrollbar synchronization until closed.

Current Rust now uses the same broad assembly: persistent `open_combo_dropdown` state, fixed popup rect, top index, content rect that shrinks for a scrollbar, full-row selected fill, full-row color swatches, scrollbar arrow/thumb pieces, and a render pass that emits dropdown instances after ordinary shell controls/flags. The main verified mismatch still present is direct mouse-wheel scrolling over an open dropdown; no scoped retail callback handles `WM_MOUSEWHEEL (0x20A)`.

## 2. Binary Assembly Facts

| Fact | Evidence | Active in YR |
|---|---|---|
| `"ComboDropWin"` is registered with WndProc `0x0060D540`, style `3`, no brush/cursor/icon, class/menu string `0x008357A0`. | decompile `FUN_0060D450`; assembly `0x0060D49E..0x0060D4C2` writes WndProc and calls `RegisterClassA` | Yes |
| The open branch creates `ComboDropWin` as a child of `GetParent(combo_hwnd)`, not as `WS_POPUP`; style is `0x40000000`, width is combo client width, and height is rounded to row-height multiples. | decompile `OwnerDraw_ComboBox_00617250`; assembly `0x006181DA..0x00618205` | Yes |
| The popup top is `combo_top_relative + combo_client_height + 1`; collapsed face paint remains fixed 24 px and is separate from dropdown row height. | decompile `0x00617250`; assembly `0x006181E6..0x006181F6`; collapsed paint `0x0061745C`; init `0x00618AEA..0x00618B46` | Yes |
| After creation the combo initializes owner-draw records, sends popup custom `0x7E8`, sends parent `0x4A9` with `lParam=1`, calls `SetCapture(dropdown_hwnd)`, calls `ShowWindow(dropdown_hwnd, 1)`, and stores state `+0xF4 = dropdown_hwnd`. | decompile `0x00617250`; assembly `0x00618436..0x00618481` | Yes |
| Close releases capture, sends parent `0x4A9` with `lParam=0`, destroys the popup, frees cached owner-draw surface state, and clears `+0xF4`. | decompile `0x00617250` close branch `0x00617EAA..0x006180A4` | Yes |
| Popup row paint uses source combo count, source combo row height, selected index, and top index; selected fill is full content row before swatch/text; text starts at `x+3` and is truncated against current client width minus `0x14`. | prior row report plus assembly contexts `0x0060D846..0x0060DFC8`, rechecked contexts `0x0060DD42`, `0x0060DE1F` | Yes |
| Color dropdown rows use full row rect inset by 2 px, not the collapsed 20x20 chip; standard no-scroll 44 px color rows produce a swatch-dominant `40x19` fill. | row assembly `0x0060DE60..0x0060DF2A`; inset helper `FUN_0072A9E0`; prior color population docs | Yes |
| When overflow exists, `ComboDropWin` creates a child `"Scrollbar"` with style `0x50010001`, subclasses it through `FUN_0060F9A0`, syncs range/current through `0xE9`, copies grey state via `0x4F1`, shrinks content width, calls `SetWindowPos`, `ShowWindow(5)`, and then a top/bring-to-front style USER32 call. | assembly `0x0060E721..0x0060E85E`; decompile `OwnerDraw_ScrollBar_0061C690`; prior scrollbar docs | Conditional on overflow |
| Direct mouse wheel handling is absent from the scoped combo, popup, and scrollbar callbacks. | decompile `0x00617250`, decompile `0x0061C690`, prior `0x0060D540` no-`0x20A` pass | No direct scoped callback path found |

## 3. Full-Shell Composition / Z-Order

The binary-backed assembly is a sibling-child popup, not an inline list drawn by the combo callback. `CreateWindowExA` receives parent `GetParent(combo_hwnd)`, class `"ComboDropWin"`, style `0x40000000`, and `lpParam = combo_hwnd`; then the code explicitly shows the new child and captures mouse to it.

Player-visible z-order implication: the open dropdown is a newly created child window of the dialog parent, so it is above the existing dialog child controls in normal USER32 child-window z-order. This report labels that last step as USER32 semantic inference; the binary evidence is the child creation/show/capture sequence at `0x006181DA..0x00618481`. No code path in the scoped combo branch draws the popup underneath row sibling controls or embeds it into the parent cached surface first.

The optional scrollbar is a child of the popup/list assembly, not a separate dialog-level sibling. It is created after overflow is detected and then shown/raised inside the popup assembly; row content uses the already-shrunken client width, so text/fill/hit testing do not extend under the scrollbar column.

The parent `0x4A9` notify is not a row renderer. Prior thunk work verifies it participates in invalidation/child refresh aggregation (`0x00611407..0x0061160B`), which explains why parent and child surfaces update coherently when the dropdown opens/closes. Rust does not need to emulate the HWND invalidation list if direct state-driven redraw gives the same frame result.

## 4. Hit Rects And Interaction

Collapsed opening remains arrow-only: mouse down/double-click toggles the dropdown only when `x > client_width - 20`. The open branch does not use the full face as a toggle hit rect.

While open, scrollbar hit testing has priority over row selection. Retail scrollbar arrows are 22 px high, thumb minimum is 14 px, arrow clicks step one row, track clicks convert the click-centered thumb position into a proportional current/top index, and drag clamps within the post-arrow track. Row hit testing uses current content client bounds and `top_index + y / item_height`; outside popup client bounds returns no item.

Clicking inside popup chrome but outside content rows is consumed by the popup rather than selecting a row. Clicking outside the popup closes it through the combo close path.

Direct wheel scrolling is not verified for YR. A future runtime message trace could prove an ancestor translation path, but the scoped callbacks do not implement it.

## 5. Current Rust Implementation Status

Current dirty Rust fits the retail assembly in several important ways:

- `src/ui/skirmish_shell/layout.rs:18..27` has the verified 24 px face, 23 px row, 20 px scrollbar, 22 px arrow buttons, 14 px min thumb, 20 px arrow reserve, and 2 px swatch inset constants.
- `src/ui/skirmish_shell/state.rs:833..895` and nearby helpers model open dropdown state, popup rect at `face + 1`, row caps, content shrink, native-like thumb/track math, and combo item lists.
- `src/ui/skirmish_shell/state.rs:1408..1495` prioritizes scrollbar hits, then content row selection, then popup chrome consumption, then outside close, matching the native assembly shape.
- `src/app_skirmish_shell_render.rs:1092..1156` renders the dropdown background, selected fill, color swatches, optional scrollbar, and frame as a separate pass.
- `src/app_skirmish_shell_render.rs:1536..1538` emits dropdown instances after ordinary shell controls and flags; `SHELL_DROPDOWN_DEPTH = 0.00034` is shallower than normal control depths, so the dropdown sits visually above them. The Choose Map modal is emitted after dropdown, but `src/app.rs:627..633` clears `open_combo_dropdown` when opening the modal, so standard state should not display both.
- `src/app_skirmish_shell_render.rs:1971..1989` draws dropdown text after normal shell text and before modal text; it uses the `ComboDropWin` row text helper, not Choose Map listbox row geometry.

Current Rust deltas / risks:

- `src/ui/skirmish_shell/state.rs:508..522` implements direct mouse-wheel top-index changes, and `src/app.rs:845..855` routes wheel input to it. This remains a retail parity risk.
- Rust still uses approximate pending-capture colors for popup background/track/final RGB; prior reports already defer screenshot-level RGB.
- Exact native scrollbar repeat-timer feel and FPU thumb-height rounding are not fully proven by this assembly pass.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Prior combo/dropdown docs | verified | reports listed in Sources | none for assembly gap |
| `ComboDropWin` class registration | verified | `FUN_0060D450`, assembly `0x0060D49E..0x0060D4C2` | none |
| Combo open create/show/capture/parent notify | verified | `OwnerDraw_ComboBox_00617250`, assembly `0x006181DA..0x00618481` | runtime screenshot for final overdraw pixels |
| Popup sibling z-order | verified-plus-inference | binary child creation/show/capture plus USER32 child z-order semantics | runtime screenshot could validate visually |
| Row fill/text/swatch clipping | verified-by-prior-and-spotcheck | `0x0060D846..0x0060DFC8`, contexts `0x0060DD42`, `0x0060DE1F` | final RGB |
| Scrollbar child assembly | verified | `0x0060E721..0x0060E85E`, `OwnerDraw_ScrollBar_0061C690` | exact repeat timer and FPU rounding |
| Direct mouse wheel | verified negative for scoped callbacks | `0x00617250`, `0x0061C690`, prior `0x0060D540` pass | possible runtime parent translation |
| Current Rust render/input fit | verified | Rust files/line scans in Section 5 | implementation work remains for wheel/RGB/repeat if pursued |
| INI/default source | verified none | scoped functions have no INI reads; prior docs agree | none |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-LA-001 - What is the target slice? -> full-shell combo dropdown/listbox assembly for standard Skirmish `0x102`, not a fresh row-geometry rediscovery.` (evidence: user target plus prior reports)
- `[RESOLVED] OQ-LA-002 - Is the popup active in YR? -> Yes, `FUN_0060D450` registers `ComboDropWin` and `OwnerDraw_ComboBox_00617250` creates it for standard combo open.` (evidence: `0x0060D450`, `0x00617250`)
- `[RESOLVED] OQ-LA-003 - Does open dropdown draw inline inside the collapsed combo callback? -> No; it creates a separate `WS_CHILD` sibling under the combo parent.` (evidence: `0x006181DA..0x00618205`)
- `[RESOLVED] OQ-LA-004 - How does the popup sit over other controls? -> Binary creates/shows a new child under the dialog parent and captures input to it; by USER32 child z-order semantics it appears above existing child controls.` (evidence: `0x006181DA..0x00618481`)
- `[RESOLVED] OQ-LA-005 - Does the combo callback itself paint rows? -> No; `ComboDropWin` owns row paint/hit/top-index; combo forwards custom hit-test while open.` (evidence: `0x0060D540..0x0060F311`; combo `0x4E8` case)
- `[RESOLVED] OQ-LA-006 - Does row content draw under the scrollbar? -> No; scrollbar creation shrinks the content client before row paint/hit.` (evidence: `0x0060E721..0x0060E85E`; prior row report)
- `[RESOLVED] OQ-LA-007 - Is Rust draw order broadly compatible? -> Yes; current Rust emits dropdown sprites after ordinary shell controls/flags and at shallower depth.` (evidence: `src/app_skirmish_shell_render.rs:1536`, `:35..42`, `:1092..1156`)
- `[RESOLVED] OQ-LA-008 - Is Rust input order broadly compatible? -> Mostly; scrollbar before rows, popup chrome consumes, outside closes. Direct wheel remains mismatching.` (evidence: `src/ui/skirmish_shell/state.rs:1408..1495`, `:508..522`)
- `[RESOLVED] OQ-LA-009 - Are INI keys involved? -> No scoped visual/listbox assembly key was found or reported in prior binary docs.` (evidence: scoped decompiles and prior reports)
- `[DEFERRED] OQ-LA-010 - Screenshot-level proof of final z-overdraw and RGB.` (category: `needs-runtime-debugger`; reason: static USER32 calls prove assembly but not captured pixels; next-step-if-pursued: retail screenshot with an open side dropdown overlapping row controls)
- `[DEFERRED] OQ-LA-011 - Runtime parent wheel translation possibility.` (category: `needs-runtime-debugger`; reason: scoped callbacks lack `0x20A`, but runtime message capture is needed to rule out ancestor translation; next-step-if-pursued: live message trace over open dropdown)
- `[DEFERRED] OQ-LA-012 - Exact scrollbar repeat-timer cadence and FPU thumb rounding.` (category: `out-of-scope`; reason: prior scrollbar reports cover geometry/math enough for assembly, while feel/rounding needs a dedicated input-timing pass; next-step-if-pursued: focused scrollbar timing trace)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Open combo creates a separate `ComboDropWin` child sibling under the dialog parent, then sends parent `0x4A9`, captures mouse, shows the popup, and stores active HWND. | `OwnerDraw_ComboBox_00617250`; assembly `0x006181DA..0x00618481`; `FUN_0060D450` | none observed for render layering; Rust uses a separate dropdown render pass after controls | `src/app_skirmish_shell_render.rs::push_dropdown_instances`, `build_skirmish_shell_instances`; `src/ui/skirmish_shell/state.rs::open_combo_dropdown` | Preserve dropdown as an overlay pass above ordinary shell controls and keep one active dropdown with outside-click close | Open a side dropdown over lower row controls: popup rows, selected fill, and scrollbar visibly cover flags/combos/text below and outside click closes it | `skirmish_combo_dropdown_renders_above_shell_child_controls`; risk: drawing dropdown before flags/controls reintroduces underpaint |
| Popup/list content shrinks when a scrollbar exists; scrollbar column is not a row hit area, and row content does not draw under it. | `0x0060E721..0x0060E85E`; row loop `0x0060D846..0x0060DFC8`; scrollbar docs | none observed for broad content/hit assembly | `src/ui/skirmish_shell/state.rs::combo_dropdown_content_rect`, `handle_combo_mouse_down`; `src/app_skirmish_shell_render.rs::push_dropdown_instances` | Preserve scrollbar-first hit order and content-width shrink for fill/text/swatch | Side dropdown with >7 rows: clicking scrollbar column scrolls/presses, not selects; row text/fill stops before scrollbar | `skirmish_dropdown_scrollbar_column_is_not_row_content`; risk: treating popup as one rectangular list makes scrollbar clicks select rows |
| Scoped retail combo/popup/scrollbar callbacks do not directly handle `WM_MOUSEWHEEL (0x20A)`. | decompile `0x00617250`, decompile `0x0061C690`, prior `0x0060D540` no-wheel pass | mismatch: Rust directly scrolls dropdown on wheel | `src/ui/skirmish_shell/state.rs::handle_option_mouse_wheel`; `src/app.rs::handle_skirmish_shell_mouse_wheel` | Remove/gate direct wheel top-index changes unless runtime parent translation is verified | Open side dropdown and wheel over it: parity mode leaves top index unchanged from this handler | `skirmish_combo_dropdown_wheel_does_not_scroll_without_verified_native_path`; risk: convenient modern UX diverges from retail |

## 9. Negative Facts / Do Not Do

- Do not route standard combo popup rows through real `OwnerDraw_ListBox_00618D40` or Choose Map listbox geometry. Active in YR: Yes. Evidence: `ComboDropWin` registration `0x0060D49E`; row block `0x0060D540..0x0060F311`.
- Do not draw the popup underneath sibling shell controls or as part of the collapsed combo face. Active in YR: Yes. Evidence: child `ComboDropWin` creation/show/capture `0x006181DA..0x00618481`.
- Do not let row fill/text/swatch paint or hit-test under the scrollbar column. Active in YR: Conditional on overflow. Evidence: scrollbar child assembly and content shrink `0x0060E721..0x0060E85E`.
- Do not use collapsed color swatch geometry for open color rows. Active in YR: Yes. Evidence: open row swatch assembly `0x0060DE60..0x0060DF2A`; collapsed swatch is separate `0x00617A5D..0x00617B3F`.
- Do not keep direct dropdown mouse-wheel scrolling as standard YR behavior without new runtime evidence. Active in YR: No direct scoped path found. Evidence: scoped callback decompiles lack `0x20A`.

## 10. Remaining Uncertainty

- Retail screenshot/surface capture still needed for final RGB and visual confirmation of overdraw pixels.
- Runtime wheel delivery remains unresolved outside scoped callbacks.
- Exact scrollbar repeat-timer cadence and FPU thumb rounding remain delegated to a future focused timing pass.

## 11. Stale Docs / Replacement Wording

- `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`: replace "Current Rust has `pressed_owner_draw_button` and selected preview caching, but it does not yet model Skirmish checkbox visual state, combo dropdown windows, combo selected-label rendering, or static text reveal timers" with "Current Rust now models Skirmish combo dropdown windows and selected-label/dropdown rendering through `open_combo_dropdown`, top-index state, popup rendering, and combo text draw paths; remaining combo-specific deltas are direct wheel behavior without verified native path, final RGB/disabled overlay, and exact scrollbar repeat/rounding."
- `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`: replace "`src/ui/skirmish_shell/state.rs:177` currently implements color combos as immediate cycling, not as a dropdown window with native combo state" with "Current Rust no longer treats standard color combos as immediate cycling only; combo clicks open a dropdown from the arrow zone and selection is applied from dropdown rows."
- `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`: replace OQ11 "Does current Rust model combo dropdown behavior? -> No; color combos cycle immediately" with "Does current Rust model combo dropdown behavior? -> Yes for the broad `ComboDropWin` assembly; verify/fix remaining wheel/RGB/repeat deltas against current dropdown reports."
- `SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md`: replace "Combo dropdown windows likely use the wrong row height and cap geometry" with "Current Rust uses the verified 23 px dropdown row height, family row caps, 20 px scrollbar reserve, and full-row content geometry; remaining dropdown risks are overlay/wheel/RGB/runtime timing items covered by current ComboDropWin reports."

## 12. Proposed Rust Tests

- `skirmish_combo_dropdown_renders_above_shell_child_controls`
- `skirmish_dropdown_scrollbar_column_is_not_row_content`
- `skirmish_combo_dropdown_wheel_does_not_scroll_without_verified_native_path`
- `skirmish_dropdown_overlay_order_precedes_choose_map_modal_only_when_modal_closed`
- `skirmish_combo_dropdown_open_close_clears_single_active_overlay_state`

## Sources

- Ghidra read-only decompile: `FUN_0060D450 @ 0x0060D450`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `FUN_0060F9A0 @ 0x0060F9A0`, `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`.
- Ghidra read-only assembly contexts: `0x0060D49E..0x0060D4C2`, `0x0061815F..0x00618205`, `0x00618436..0x00618481`, `0x0060E0E9..0x0060E14F`, `0x0060E721..0x0060E85E`, `0x0060DD42..0x0060DE1F`, `0x0061D383..0x0061D548`.
- Prior reports: `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`, `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_COMBODROPWIN_DROPDOWN_VISUAL_INPUT_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_SCROLLBAR_SOUNDS_GHIDRA_REPORT.md`, `SKIRMISH_COMBODROPWIN_0060D540_FUNCTION_BOUNDARY_GHIDRA_REPORT.md`, `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`, `SKIRMISH_0X102_FIRST_PAINT_COMPOSITION_VS_RUST_DRAW_ORDER_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/app.rs`.
- INI files checked: none; the scoped assembly is USER32/owner-draw message driven and prior reports found no INI keys for this visual/input slice.
