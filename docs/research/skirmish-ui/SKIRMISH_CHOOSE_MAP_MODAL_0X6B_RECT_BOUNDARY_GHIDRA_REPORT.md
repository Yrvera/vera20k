# Skirmish Choose Map Modal 0x6B Rect Boundary - Ghidra Research Report

**Address(es):** `0x005E68A0`, callback block `0x005E6920`, `0x00622820`, `0x0060C540`, `ResizeShellChildControl_0060C0C0 @ 0x0060C0C0`, `0x00608CD0`, `0x00609730`, `0x00601360`, `0x0060B000`, `0x0060B1D0`, `0x0060B350`, `0x0060B550`, `0x0060B950`, `OwnerDraw_ListBox_00618D40 @ 0x00618D40`, `0x005E7160`
**Investigation Mode:** `/re-swarm` slot 5, narrow rect-boundary slice.

## Working Notes

- Target question: Which Choose Map modal dialog `0x6B` rects and controls belong in a separate modal table rather than setup dialog `0x102`, and what final helper rules apply to the modal boundary, listboxes, buttons, preview/title/statics, row hit rectangles, and high-res placement?
- Non-goals: scenario source ordering except where it affects visible list count/scrolling, random map generator UI, map preview decode, setup `0x102` complete table, combo dropdown internals, Rust edits, and Ghidra mutations.
- Evidence needed to mark COMPLETE: active Skirmish `0x5AA -> 0x6B` path; resource/control inventory; decompile plus assembly/disassembly for helper routing; final rect formulas for scoped controls at 800x600 and high-res; listbox row height/hit evidence; proof of setup helper reuse/non-reuse; Rust-facing handoff and negative facts.
- Stop conditions: stop after the modal rect/table boundary and list hit formula are resolved or after exact row-height proof remains unavailable from read-only static evidence; write only this report and the shared swarm claims file.

## Summary

Retail Choose Map is not a child overlay and not part of the setup dialog `0x102` rect table. The active setup command `0x5AA` hides setup and enters a separate shell dialog resource `0x6B` through `0x005E68A0`. That dialog is processed by the common fullscreen shell setup path and then has its children resized by `ResizeShellChildControl_0060C0C0`.

The important correction is that resource `0x6B` values are dialog units, not final raw pixels. With the verified shell base units `baseX=6`, `baseY=13`, the template maps to the same 800x600 shell basis as setup. Some `0x6B` children preserve their DLU-derived base positions; right-panel controls are selectively moved by the same generic shell helpers used by setup. Therefore the Choose Map table should be separate, but it should still use the common shell helper semantics.

Status is PARTIAL only because exact native `LB_GETITEMHEIGHT` for real `LISTBOX` controls was not found as a gamemd hardcoded value. The binary hit-test formula is verified: row index is `top_index + y / LB_GETITEMHEIGHT`, with client-bound and item-count checks. A Rust constant such as `16` should not be called binary-verified until a runtime/WM_MEASUREITEM trace or screenshot confirms the native listbox item height.

## Active Path And Dialog Boundary

| Finding | Active in YR | Evidence |
|---|---|---|
| Setup `0x5AA` opens the separate Choose Map modal wrapper; `0x6B` is not a setup `0x102` subtable. | Yes | `0x005E68A0` decompile; assembly `0x005E68B7 MOV EDX,0x6B`, `0x005E68BE PUSH 0x005E6920`, `0x005E68C4 CALL 0x00775700`; parent branch is the live `0x006ACEE0` `0x5AA` path per sibling reports. |
| Dialog resource `0x6B` uses the fullscreen shell path. Runtime parent rect becomes `(0,0,g_ScreenWidth,g_ScreenHeight)`, not a centered 533x369 pixel modal. | Yes | `0x00622820` calls `0x0060C540`; `0x0060C540` includes `iVar1 == 0x6B`; `0x00622820` fullscreen branch calls `MoveWindow(parent,0,0,g_ScreenWidth,g_ScreenHeight,0)` and enumerates `ResizeShellChildControl_0060C0C0`. |
| Resource `0x6B` is DIALOGEX `(0,0,533,369)` dialog units with 11 controls. At the verified MS Sans Serif 8 base units, this is the 800x600 shell basis. | Yes | Resource extraction in `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`; DLU base `6/13` is verified by `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`; helper evidence here confirms DLU-derived controls before resize. |
| `MnScrnLCustomizeBattle.shp/.PAL` is the modal asset branch for `0x6B`, not setup `0x102`. | Yes / Conditional for SHP at exactly 800-wide | Existing Ghidra evidence `0x0072D120`, `0x0060CF00`; this slot did not re-open asset internals. |

## Control Routing And Final Rects

Base DLU conversion uses `x,w = MulDiv(dlu,6,4)` and `y,h = MulDiv(dlu,13,8)` for positive values. The table below records the modal table that should be separate from setup `0x102`.

| Control | Resource DLU | Base px before helper | Helper route | 800x600 final | 1024x768 final rule/result | Active in YR |
|---|---:|---:|---|---:|---:|---|
| Mode list `0x6EB` | `(77,78,130,211)` | `(116,127,195,343)` | fallback preserve-current rect; not right-anchor | `(116,127,195,343)` | unchanged by helper: `(116,127,195,343)` | Yes |
| Map list `0x553` | `(225,78,130,211)` | `(338,127,195,343)` | fallback preserve-current rect; not right-anchor | `(338,127,195,343)` | unchanged by helper: `(338,127,195,343)` | Yes |
| Use Map `0x6C5` | `(425,122,108,23)` | `(638,198,162,37)` | owner-draw button plus `0x00608CD0` allowlist -> `0x0060B000`, tile-snap | `(644,199,156,42)` | right/bottom offset: `(756,283,156,42)` | Yes |
| Create Random Map `0x583` | `(425,149,108,23)` | `(638,242,162,37)` | owner-draw button plus `0x00608CD0` allowlist -> `0x0060B000`, tile-snap | `(644,241,156,42)` | right/bottom offset: `(756,325,156,42)` | Conditional by button visibility/click; control exists in standard path |
| Cancel `0x5C0` | `(425,346,108,23)` | `(638,562,162,37)` | owner-draw button plus `0x00609730` -> `0x0060B350` bottom/right helper | `(644,535,156,42)` | right/bottom offset: `(756,619,156,42)` | Yes |
| Preview static `0x468` | `(428,23,96,69)` | `(642,37,144,112)` | `0x00608CD0` preview allowlist -> `0x0060B1D0` right-anchor | `(644,37,144,112)` | right/bottom offset: `(756,121,144,112)` | Yes |
| Title static `0x694` | `(425,1,108,10)` | `(638,2,162,16)` | `0x00608CD0` title allowlist -> `0x0060B1D0`; `0x0060B950` adds +1 y in normal shell path | `(635,3,162,16)` | right/bottom offset: `(747,87,162,16)` | Yes |
| Select Engagement static `-1` | `(80,20,257,12)` | `(120,33,386,20)` | fallback preserve-current rect | `(120,33,386,20)` | unchanged by helper | Yes |
| Game Type heading `-1` | `(77,60,130,10)` | `(116,98,195,16)` | fallback preserve-current rect | `(116,98,195,16)` | unchanged by helper | Yes |
| Game Map heading `-1` | `(225,60,130,10)` | `(338,98,195,16)` | fallback preserve-current rect | `(338,98,195,16)` | unchanged by helper | Yes |
| Help/status static `0x695` | `(2,355,303,12)` | `(3,577,455,20)` | dialog-id allowlist `0x00601360` plus id `0x695` -> `0x0060B550` bottom-left helper | `(10,579,455,20)` | centered base offset for x/y: `(122,663,455,20)` | Yes |

Evidence:

- `0x00608CD0` includes dialog `0x6B` for title `0x694`, preview `0x468`, and has a specific `0x6B` branch allowing `0x583` and `0x6C5`.
- `0x00609730` includes dialog `0x6B` for control `0x5C0`.
- `0x00601360` includes dialog `0x6B`; `ResizeShellChildControl_0060C0C0` only calls `0x0060B550` when `GetDlgCtrlID == 0x695`.
- `0x0060B000` sets owner-draw button width/height from `SDBTNANM.SHP` (`156x42`) and snaps Y to the nearest `DAT_00B0FC24` tile row.
- `0x0060B350` sets the bottom/right button to the last visible right-panel tile row.
- `0x0060B1D0` right-anchors right-panel statics/preview while preserving width/height.
- `0x0060B550` moves `0x695` to `x = centered_offset_x + 10`, `y = screen_h - control_h - centered_offset_y - 1`.

## Lists, Row Count, And Hit Rect

| Finding | Active in YR | Evidence |
|---|---|---|
| `0x6EB` and `0x553` are real owner-drawn `LISTBOX` controls, not combo dropdowns. | Yes | Resource class/style `LISTBOX 0x50000151`; `FUN_0060F9A0` maps `"ListBox"` to `OwnerDraw_ListBox_00618D40` at `0x0060FC18..0x0060FC29`; accept uses `LB_GETCURSEL 0x188` and `LB_GETITEMDATA 0x199` in `0x005E7160`. |
| Hit testing is client-rect based and item-height based, not a hardcoded modal constant. | Yes | `OwnerDraw_ListBox_00618D40` custom `0x4E8` case assembly `0x0061BB47..0x0061BBD9`: reject if x/y outside client width/height, call `LB_GETITEMHEIGHT 0x1A1`, call `LB_GETCOUNT 0x18B`, compute `top_index + y / item_height`, reject outside count. |
| Visible full-row capacity is `floor(client_height / LB_GETITEMHEIGHT)`, with top index from list state `+0xF0`; scrollbar presence can shrink client width but does not change the y formula. | Yes / Conditional on item count for scrollbar | Decompile of `OwnerDraw_ListBox_00618D40`; scrollbar creation/resize block `0x0061BC05..0x0061C45D`; state `+0xF0` read in the `0x4E8` hit path. |
| Exact retail item height for these real listboxes was not proven from a static hardcoded constant. | Conditional / unresolved | The binary reads `LB_GETITEMHEIGHT`; this slot found no `LB_SETITEMHEIGHT 0x1A0` for `0x6EB`/`0x553` in the modal init. A runtime/WM_MEASUREITEM trace or screenshot is needed before calling Rust's `CHOOSE_MAP_LIST_ROW_H = 16` binary-verified. |

Implementation implication: if runtime confirms `LB_GETITEMHEIGHT == 16`, each 343 px list client has 21 full 16 px rows plus a 7 px bottom remainder; hit-testing in that remainder still computes row 21 and then rejects it if it is beyond item count, or accepts it if enough items exist and the native listbox client includes that area. Until confirmed, Rust should model row hit-testing through a native-row-height parameter rather than treating `16` as a settled binary constant.

## Preview And Static Paint

The modal callback has its own `WM_PAINT` path. At `0x005E696B..0x005E6990`, it calls the shell paint helper, tests `DAT_00AC1154`, and calls `DrawStartPositions @ 0x00640710` with the modal HWND before validating the rect. Active in YR: Yes. This resolves the earlier uncertainty that the chooser preview static was merely present: dialog `0x6B` can paint the preview/start overlay through the same parent-owned preview routine when the preview object exists.

This does not prove every row-selection preview-refresh timing detail. The init path selects the map row matching `DAT_00A8B254` (`0x005E6F94..0x005E701B`), and sibling reports cover preview refresh after modal return. This slot did not prove a live preview update on every list selection before `Use Map`.

Static controls:

- `0x694` is the right-panel title static `GUI:ChooseMap`, right-anchored and nudged down by `0x0060B950`.
- The heading statics are resource id `-1`; they preserve DLU-derived base rects and should not be modeled as mutable state controls.
- `0x695` is the bottom help/status strip using the bottom-left helper; it is not the setup dialog's wider `0x102` status rect.

## Setup Helper Reuse

Active in YR: Yes. Dialog `0x6B` reuses the generic shell helpers (`0x0060B000`, `0x0060B1D0`, `0x0060B350`, `0x0060B550`, `0x0060B950`) through `ResizeShellChildControl_0060C0C0`, but it does not reuse the setup-specific `0x102` allowlist entries for `0x5AA`, `0x617`, `0x6EC`, or `0x5A8`.

The reuse boundary is:

- Reused generic shell machinery: fullscreen parent move, owner-draw subclass install, right-panel title/preview anchors, PCX button snap helper, bottom button helper, bottom help strip helper, static text owner-draw.
- Not reused as setup `0x102` table entries: setup `Choose Map 0x5AA`, setup `Start 0x617`, setup game-type label `0x6EC`, setup selected-map label `0x5A8`, player rows, checkboxes, trackbars, and combo controls.

## Current Rust Delta

Current Rust surface scan:

- `src/ui/skirmish_shell/layout.rs` has `compute_choose_map_modal_layout`, but it models a centered `533x369` pixel dialog and uses raw resource-like modal coordinates for lists.
- The binary evidence here says runtime parent is fullscreen and child controls are DLU-derived or helper-moved. The listboxes should be `(116,127,195,343)` at 800x600, not `(210,193,130,211)`.
- Current Rust exposes only `title` and `preview` for modal statics; it omits `SelectEngagement`, `GameType`, `GameMap`, and `0x695` from the modal layout struct.
- `choose_map_modal_list_row_at` hardcodes `CHOOSE_MAP_LIST_ROW_H = 16`; binary hit-testing reads native `LB_GETITEMHEIGHT`.
- Modal state exists in `state.rs`, but rendering/integration gaps are covered by sibling visual-integration reports.

## Implementation Handoff

| Verified behavior | Current Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| `0x6B` final rect table is fullscreen-shell DLU plus helper routes, not a centered raw `533x369` overlay. | `compute_choose_map_modal_layout` centers a 533x369 pixel dialog and places listboxes at raw `(77,78,130,211)` offsets. | `src/ui/skirmish_shell/layout.rs`, modal renderer | At 800x600, mode list is `(116,127,195,343)`, map list is `(338,127,195,343)`, Use Map is `(644,199,156,42)`, Cancel is `(644,535,156,42)`; at 1024x768 only helper-routed right/bottom controls receive center offsets. | `choose_map_modal_0x6b_uses_dlu_and_shell_resize_helpers` | High: wrong modal placement is immediately visible and corrupts hit testing. |
| Real listboxes hit-test with owner-draw font+2 row height and top index, not a baked row constant. | `CHOOSE_MAP_LIST_ROW_H = 16` is hardcoded, while `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md` verifies standard inferred row height `19` from `GAME.FNT` height `17`. | `src/ui/skirmish_shell/layout.rs`, `state.rs` list hit testing | Row 0 starts at list top and row mapping uses `top_index + y / item_height`; a 211 px list exposes 11 full rows plus remainder under the standard 19 px row height. | `choose_map_modal_list_hit_test_uses_native_item_height_and_top_index` | Medium: row selection off-by-one shows up only when clicking lower list rows or after scroll. |
| Modal preview/title/status controls have their own `0x6B` rects and helper routes. | Modal layout omits `0x695` and heading statics and uses setup-like/centered preview placement. | `layout.rs`, `app_skirmish_shell_render.rs` | Opening Choose Map renders title at the right panel top, preview at `(644,37,144,112)`, headings above the two listboxes, and status strip at bottom-left; setup `0x6EC`/`0x5A8` labels are absent. | `choose_map_modal_static_and_preview_rects_match_0x6b_table` | Medium: visual mismatch and stale setup labels in modal. |

## Negative Facts / Do Not Do

- Do not add Choose Map controls to the setup `0x102` rect table. Active in YR: No; evidence `0x005E68A0` creates dialog resource `0x6B`, and `0x00622820` stores/processes the `0x6B` dialog id separately.
- Do not treat `0x6EB` or `0x553` as dropdown combos. Active in YR: No; evidence resource class `LISTBOX`, owner-draw class dispatch `0x0060FC18..0x0060FC29`, and accept listbox messages in `0x005E7160`.
- Do not use raw resource coordinate pixels like `(77,78,130,211)` or a centered `533x369` pixel modal for final rects. Active in YR: No; evidence DIALOGEX DLU resource, verified base units, fullscreen parent move, and resize helper routing.
- Do not copy setup labels `0x6EC` and `0x5A8` into the chooser. Active in YR: No; resource `0x6B` has heading statics with id `-1`, title `0x694`, status `0x695`, and preview `0x468`, but no `0x6EC` or `0x5A8`.
- Do not keep row height `16` as a binary-verified value. Active in YR: Yes for standard Choose Map listboxes; follow-up `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md` verifies owner-draw row height as font height plus 2, standard inferred `19` px.

## Remaining Uncertainty

- Exact native row height is resolved for the standard owner-draw path by `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`: font height plus 2, standard inferred `19` px.
- Exact listbox row paint insets are resolved for basic Choose Map rows by `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`: selected fill spans the full item rectangle and basic text starts at item-left `+2`.
- Live preview refresh on every modal list selection before `Use Map` is resolved by `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`: normal row highlight/category rebuild does not refresh the preview; Use Map commit and parent return are the normal refresh boundary.

## Stale Docs / Replacement Wording

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`: replace wording that presents resource control rects as final pixel rects with: "Resource `0x6B` control coordinates are dialog units. At the verified shell base units (`6/13`), listboxes `0x6EB` and `0x553` start from `(116,127,195,343)` and `(338,127,195,343)` before helper routing; right-panel controls then move through `ResizeShellChildControl_0060C0C0` helpers."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_0X6B_VISUAL_INTEGRATION_GHIDRA_REPORT.md`: replace "Center modal controls inside the shell coordinate system after fullscreen parent move" with: "After the fullscreen parent move, ordinary `0x6B` children preserve their DLU-derived base positions; only helper-routed right/bottom controls receive the common shell center offsets."
- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs` tests are not docs, but the current expected modal rects are stale relative to this binary evidence.

## Sources

- Read-only Ghidra decompile: `0x005E68A0`, `0x00622820`, `0x0060C540`, `0x0060C0C0`, `0x00608CD0`, `0x00609730`, `0x00601360`, `0x0060B000`, `0x0060B1D0`, `0x0060B350`, `0x0060B550`, `0x0060B950`, `0x00618D40`.
- Read-only Ghidra assembly/context: `0x005E68B7..0x005E690F`, callback block `0x005E6920..0x005E7038`, init/list selection `0x005E6EA6..0x005E701B`, class dispatch `0x0060FC18..0x0060FC29`, list hit-test `0x0061BB47..0x0061BBD9`.
- Existing binary-backed docs reconciled: `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_VISUAL_INTEGRATION_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODE_CATEGORY_0X6EB_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`.
- Rust scan: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`.

## Status

PARTIAL: dialog boundary, helper routing, final modal rect table, and list hit-test formula are verified; exact native `LB_GETITEMHEIGHT` for `0x6EB`/`0x553` remains unresolved and blocks a COMPLETE verdict for row height/visible-row count.
