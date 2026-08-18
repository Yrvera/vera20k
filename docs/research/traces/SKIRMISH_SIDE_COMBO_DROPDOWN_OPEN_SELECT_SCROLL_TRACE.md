# Skirmish Side Combo Dropdown Open/Select/Scroll Trace

**Trace target:** Native/dev Skirmish shell dialog `0x102`, 800x600, local player Side combo.
**Concrete scenario:** click the local player Side combo arrow, open the popup, click the scrollbar down arrow once, select the non-default `Great Britain`/`British` side row, and close by selecting it.
**Rust surfaces:** `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
**Verdict tally:** PASS: 6 | FAIL: 4 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0
**Status:** COMPLETE

## Pipeline

Trigger -> arrow hit test -> open dropdown state -> popup geometry -> popup paint -> scrollbar input/top index -> row hit selection -> closed combo state/redraw.

## Concrete Rust Values

At `compute_layout(800, 600)`, the local player side combo is `RectPx { x: 287, y: 59, w: 117, h: 120 }`.

The scenario click on the arrow uses `x=403,y=60`, inside Rust's rightmost 20 px arrow reserve. Rust opens:

- item count: `11` (`Random` plus 10 `SkirmishCountry::ALL` entries)
- visible rows: `7`
- dropdown: `RectPx { x: 287, y: 84, w: 117, h: 161 }`
- content: `RectPx { x: 287, y: 84, w: 97, h: 161 }`
- scrollbar: `RectPx { x: 384, y: 84, w: 20, h: 161 }`
- top index initially `0`, max top index `4`
- thumb at top index `0`: `x=384,y=106,w=20,h=74`

Clicking the scrollbar down arrow once at `x=385,y=244` changes Rust `top_index` from `0` to `1`.

Selecting `Great Britain` after that uses item index `5`, visible row `4`, click `x=289,y=177`; Rust writes `player_country_random=false`, `player_country=GreatBritain`, and closes `open_combo_dropdown=None`.

## gamemd Evidence

The standard offline Skirmish path is active in YR:

- Dialog `0x102` installs proc `FUN_006AE3F0`; its `WM_COMMAND` path routes through `FUN_006ACEE0`. Verified by current Ghidra decompile of `FUN_006AE3F0` and report `SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`.
- `FUN_004e3a00(hwnd, control)` populates country combos: sends max visible rows `0x4DE=7`, inserts Random with item data `-2`, then inserts eligible `HouseTypeClass` countries with item data `0..9`. Current Ghidra decompile confirms this path.
- `FUN_004e4170(hwnd, control, -1)` reads current selection and returns item data, falling back only if outside `-3..9`. Current Ghidra decompile confirms this getter.
- `FUN_0060D450` registers `"ComboDropWin"` with WndProc label `0x0060D540`; current Ghidra decompile confirms registration.
- `OwnerDraw_ComboBox_00617250` is the active owner-draw combo WndProc; current Ghidra decompile confirms rightmost 20 px toggle, `CB_SHOWDROPDOWN 0x14F`, popup creation, collapsed paint, selected text truncation, and open/close.
- `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md` verifies `ComboDropWin` row geometry, selected-row fill, text clipping, scrollbar width, and scrollbar callback behavior for active standard YR combo paths.

## Stage Results

| Stage | Verdict | Comparison |
|---|---:|---|
| Active standard YR path | PASS | `FUN_006AE3F0` handles dialog `0x102`, country helper `FUN_004e3a00` and getter `FUN_004e4170` are active for standard offline Skirmish. |
| Side item list/count | PASS | gamemd inserts Random `-2` plus stock country item data `0..9`; Rust exposes `Random` plus 10 countries at `src/ui/skirmish_shell/state.rs:746`. |
| Arrow-open trigger for concrete click | PASS | gamemd opens only from rightmost 20 px; Rust `combo_arrow_at` tests the same reserve at `src/ui/skirmish_shell/state.rs:874` and `:898`. The concrete click `x=403` is inside both. |
| Popup geometry | PASS | Rust computes `287,84,117,161`, matching the verified contract: combo x, one pixel below 24 px face, 7 rows at 23 px. |
| Background/border/dropdown primitive colors | FAIL | gamemd derives primitive colors from owner-draw globals and display conversion; Rust uses approximate constants `SHELL_DROPDOWN_BG_RGB`, `SHELL_DROPDOWN_BORDER_RGB`, and `SHELL_DROPDOWN_SELECTED_RGB` at `src/app_skirmish_shell_render.rs:47`. |
| Selected-row fill geometry | FAIL | gamemd fills the full current row rect before text; Rust insets by 1 px and uses height `row_h - 2` at `src/app_skirmish_shell_render.rs:932`. For selected America at top `0`, gamemd row is `x=287,y=107,w=97,h=23`; Rust draws `x=288,y=108,w=95,h=21`. |
| Popup text clipping/truncation | FAIL | gamemd truncates row text until `BitFont__GetTextWidth <= client_width - 20`, then draws from `x+3` to row width; Rust passes a draw rect `content.w - 3` and relies on clipping at `src/app_skirmish_shell_render.rs:1619`. Long side names such as `Great Britain` can draw/clip differently. Exact glyph width remains screenshot/font-runtime dependent, but the caller contract is different. |
| Scrollbar content shrink | PASS | gamemd shrinks row client width when a 20 px scrollbar exists; Rust subtracts `COMBO_DROPDOWN_SCROLLBAR_W` at `src/ui/skirmish_shell/state.rs:616`. Concrete content width is `97`. |
| Scrollbar thumb height/position | UNCHECKED | Rust computes thumb `20x74` at y `106` for top `0`; gamemd uses owner-draw scrollbar range/page payload and floating conversion. Prior docs verify the algorithm family, but this trace did not compute the exact native range/page payload for the side combo. |
| Scrollbar pressed/arrow visual state | FAIL | gamemd owner-draw scrollbar uses released/pressed arrow and grip PCX states; Rust always pushes released up/down arrows in `push_dropdown_scrollbar_instances` at `src/app_skirmish_shell_render.rs:657` and `:666`. The player does not see the native pressed scrollbar arrow feedback. |
| Scroll one row by down arrow | PASS | Rust down-arrow click changes top index `0 -> 1` at `src/ui/skirmish_shell/state.rs:987`; prior gamemd scrollbar report verifies arrow-step scrolling. |
| Select non-default `Great Britain` row and close | PASS | Rust click on visible row 4 after top `1` selects item index `5`, writes `GreatBritain`, and closes at `src/ui/skirmish_shell/state.rs:1015`. gamemd country item data for British is `4`; selecting row index `5` maps to that non-default country. |
| Exact message/action ordering | UNCHECKED | gamemd routes native messages through `ComboDropWin`, `CB_SETCURSEL`, and parent notifications; Rust applies selection directly during mouse-down handling. The final selected state matches for this concrete row, but exact notification timing/order was not numerically traced. |
| Final collapsed redraw after selection | UNCHECKED | Rust should display `Great Britain` after closing, but collapsed combo text uses the same approximate caller/text color path as the dropdown. Exact post-close pixels were not compared against a retail screenshot. |

## Top Player-Visible Failures

1. **Popup paint:** dropdown background, border, and selected color are approximate constants, not gamemd owner-draw converted colors; affected Rust: `src/app_skirmish_shell_render.rs:47`; gamemd evidence: `OwnerDraw_ComboBox_00617250`, `FUN_006208F0`, `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`.
2. **Selected-row fill:** selected row is visibly inset by 1 px in Rust instead of full-row fill; affected Rust: `src/app_skirmish_shell_render.rs:932`; gamemd evidence: `ComboDropWin` selected fill block `0x0060DD42..0x0060DE0A`.
3. **Dropdown text clipping:** Rust clips to its draw rect instead of pre-truncating to `client_width - 20`; affected Rust: `src/app_skirmish_shell_render.rs:1619`; gamemd evidence: `ComboDropWin` text block `0x0060DE1F..0x0060DFC8`.
4. **Scrollbar pressed feedback:** Rust renders scrollbar arrows as released even while clicked; affected Rust: `src/app_skirmish_shell_render.rs:657`; gamemd evidence: `OwnerDraw_ScrollBar_0061C690` plus `FUN_00620720`.

## Adjacent Findings

- This trace did not audit color combo row-8 omission, checkbox hit testing, owner-draw button sounds, or Choose Map modal behavior.
- Exact final RGB still needs retail screenshot or 16-bit surface capture; binary evidence proves source globals and conversion paths, not the final captured display pixels.
