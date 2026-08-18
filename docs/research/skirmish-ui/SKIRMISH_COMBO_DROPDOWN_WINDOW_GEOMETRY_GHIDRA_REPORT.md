# Skirmish Combo Dropdown Window Geometry - Ghidra Research Report

**Address(es):** `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `OwnerDraw_ListBox_00618D40 @ 0x00618D40`, `FUN_0060D450 @ 0x0060D450`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Skirmish dialog `0x102` owner-draw combo dropdown geometry only: `ComboDropWin` creation rectangle, open/close placement, item height, `0x4DE` max-row behavior, scrollbar handoff, hit-test forwarding, and width differences between player/side/color/start/team combo families.
**Non-Scope:** collapsed-face paint geometry already covered by `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`; row-internal listbox text/swatch drawing; combo population semantics except where population controls row counts or `0x4DE`.
**Confidence:** High for create rect, height math, max-row behavior, scrollbar width, and standard offline Skirmish control families. Medium for runtime screenshots because no live capture was taken in this pass.
**Active in YR:** Yes. Offline Skirmish dialog `0x102` reaches `FUN_006AE6E0`, hooks `"ComboBox"` controls through `FUN_0060F9A0`, and the hooked controls use `OwnerDraw_ComboBox_00617250`; evidence from prior owner-draw reports plus rechecked callback at `0x00617250`.

## 1. Summary

The Skirmish owner-draw combo dropdown is a custom child popup named `ComboDropWin`, not the native list portion of the resource combo. Opening a combo computes a new child window under the collapsed control, gives it the same client width as the combo, chooses a height from item height, item count, `0x4DE` max-visible rows, and bottom-boundary space, then rounds that height down to a whole multiple of the row height.

Player-visible consequence: the resource combo height is only a capacity hint. Wide player combos open a 150 px wide four-row list. Side/country combos open 117 px wide and can scroll because they cap at 7 visible rows. Color combos open 44 px wide with up to 9 rows; start/team combos open 38 px wide with up to 9 rows, and standard offline populations fit without a scrollbar.

## 2. Activation And Class Registration

`FUN_0060D450` registers window class `"ComboDropWin"` with style `3`, hInstance `DAT_00B732F0`, WndProc `LAB_0060D540`, no icon/cursor/background brush, and class/menu name both pointing at string `0x008357A0`.

Active in YR: Yes. `OwnerDraw_ComboBox_00617250` creates this class from its `CB_SHOWDROPDOWN` / `0x14F` branch; evidence `FUN_0060D450 @ 0x0060D450`, `CreateWindowExA` call at `0x006181FE..0x00618205`.

No TS-only or WOL-only gate was found on this class or the `0x102` combo path. The same owner-draw callback is reached by standard offline Skirmish setup.

Active in YR: Yes. Evidence: `FUN_006AE6E0` initializes offline dialog `0x102` combos; prior `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md` verifies `FUN_0060F9A0` assigns `"ComboBox"` controls to `0x00617250`.

## 3. Item Height

On custom init message `0x497`, `OwnerDraw_ComboBox_00617250` sets two combo heights:

- `CB_SETITEMHEIGHT 0x153`, `wParam = -1`, `lParam = font_height + 2`;
- `CB_SETITEMHEIGHT 0x153`, `wParam = 0`, `lParam = font_height + 6`.

The open branch later reads dropdown row height with `CB_GETITEMHEIGHT 0x154`, `wParam = 0`, so the row height used for popup sizing is `font_height + 6`, not the 24 px collapsed-face paint constant.

Active in YR: Yes. Evidence: init block `0x00618AEA..0x00618B46`; open branch row-height read `0x006180FB..0x00618118`.

The shell font object is `g_GAME_FNT`; prior bitfont research records `GAME.FNT` cell height as 17 px. With that standard shell font, the dropdown row-height formula yields `23` px. This report treats `23` as the standard-YR value and the formula as the load-bearing behavior.

Active in YR: Yes for the formula; standard font evidence from `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` (`g_GAME_FNT`, cell height 17) and owner-draw init `0x00618B1A..0x00618B44`.

## 4. Open Trigger And Open/Close State

Mouse down or double-click only toggles the dropdown when the click is inside the rightmost 20 px arrow zone:

```text
if mouse_x > client_width - 0x14:
    dropped = SendMessage(combo, CB_GETDROPPEDSTATE 0x157)
    PostMessage(combo, CB_SHOWDROPDOWN 0x14F, dropped != 1)
```

Active in YR: Yes. Evidence: mouse branch in `OwnerDraw_ComboBox_00617250`, `0x00617D4E` case for `0x201/0x203`, comparison against `client_width - 0x14`, and post of `0x14F`.

Open path (`0x14F`, `wParam != 0`) returns immediately if state `+0xF4` already stores an active dropdown. On successful open it sends parent message `0x4A9` with `wParam = dropdown_hwnd`, `lParam = 1`, calls `SetCapture(dropdown_hwnd)`, shows the dropdown, and writes state `+0xF4 = dropdown_hwnd`.

Active in YR: Yes. Evidence: `0x006180A0` existing-dropdown guard; `0x00618436..0x00618481` parent notify/capture/show/store.

Close path (`0x14F`, `wParam == 0`) releases capture, sends parent `0x4A9` with `lParam = 0`, destroys the dropdown, releases cached owner-draw state/surface records, and clears state `+0xF4` to zero. If no dropdown is active, it returns `1`.

Active in YR: Yes. Evidence: close branch `0x00617EAA..0x006180A4`; state field `+0xF4` from prior owner-draw report.

## 5. `ComboDropWin` Creation Rectangle

The open branch first obtains the native dropped-control rectangle using `CB_GETDROPPEDCONTROLRECT 0x152`, then clamps that rectangle's bottom to the parent window bottom as measured by `FUN_00775690`. `FUN_00775690` converts window rectangles into game-client coordinates by subtracting the main `g_hWnd` client origin.

Active in YR: Yes. Evidence: `CB_GETDROPPEDCONTROLRECT` call `0x006180DC..0x006180E9`; bottom clamp `0x006180EB..0x006180F7`; `FUN_00775690 @ 0x00775690`.

Immediately before `CreateWindowExA`, the code converts both parent and combo window rectangles to game-client coordinates, subtracts parent origin, and creates the popup as a child of the combo parent:

```text
x      = combo_window_left - parent_window_left
y      = combo_window_top  - parent_window_top + combo_client_height + 1
width  = combo_client_width
height = computed_height - (computed_height % item_height)
class  = "ComboDropWin"
style  = 0x40000000
parent = GetParent(combo_hwnd)
lpParam = combo_hwnd
```

Active in YR: Yes. Evidence: parent/combo rect conversions `0x00618172..0x0061819C`; x/y/width/height setup and `CreateWindowExA` call `0x006181A1..0x00618205`; class string `0x008357A0`.

Tiny placement detail: the dropdown top is one pixel below the combo client-height baseline. It is not placed at the bottom of the resource template's tall dropdown capacity rectangle.

Active in YR: Yes. Evidence: y expression in assembly `0x006181DF..0x006181F5` and decompile `CreateWindowExA(..., in_stack_000000dc + 1 + combo_top_relative, ...)`.

## 6. Height And `0x4DE` Max-Visible Row Math

The `0x4DE` custom message stores its `lParam` directly at combo state `+0xD0`.

Active in YR: Yes. Evidence: message case `0x4DE`, `MOV [EBX+0xD0], ECX` at `0x00618A67..0x00618A6E`.

When opening, height selection uses:

```text
available_height = min(native_drop_rect.bottom, parent_rect.bottom) - native_drop_rect.top
item_height      = SendMessage(combo, CB_GETITEMHEIGHT 0x154, 0, 0)
count            = max(1, SendMessage(combo, CB_GETCOUNT 0x146, 0, 0))
available_rows   = (available_height - 1) / item_height
max_rows         = state+0xD0

if max_rows > 0:
    wanted_rows = min(count, max_rows)
    raw_height = wanted_rows * item_height + 1
else if available_rows <= count:
    raw_height = available_height
else:
    raw_height = count * item_height + 1

raw_height = min(raw_height, parent_rect.bottom - drop_rect.top)
final_height = raw_height - (raw_height % item_height)
```

Active in YR: Yes. Evidence: row-height/count/max code `0x006180FB..0x0061815D`; bottom clamp `0x0061815F..0x00618170`; modulo-rounding before `CreateWindowExA` `0x006181DA..0x006181F4`.

The `+1` added to capped/count-based raw heights is not preserved in the final window height because the `CreateWindowExA` height is rounded down by `height % item_height`. For a capped 7-row side combo with a 23 px row height, the actual created height is `161` px, not `162` px.

Active in YR: Yes. Evidence: `IMUL EAX, EBX; INC EAX` at `0x00618159..0x0061815C`, then `IDIV EBX; SUB ESI, EDX` at `0x006181DA..0x006181EA`.

## 7. Standard Skirmish Combo Families

The `0x102` resource widths below are from the verified DLU-to-pixel conversion (`baseX=6`, `baseY=13`) in the prior layout/owner-draw reports. The dropdown uses combo client width, so these widths also become `ComboDropWin` widths.

| Family | Control IDs | Width | Standard rows / cap | Scroll expectation |
|---|---:|---:|---|---|
| player/AI type | `0x50B`, `0x50E`, `0x516`, `0x51A..0x51D` | `150` px | 4 rows, no `0x4DE` set in `FUN_006AE6E0` | no scroll in standard offline |
| side/country | `0x6A1`, `0x510`, `0x513`, `0x514`, `0x51E..0x521` | `117` px | `0x4DE = 7`; rows are Random plus eligible multiplayer houses | scroll when eligible row count exceeds 7 |
| color | `0x6A2`, `0x522..0x528` | `44` px | `0x4DE = 9`; normal population inserts sentinel plus colors `0..7` | no scroll when all 9 fit |
| start | `0x6A3..0x6A8`, `0x6AA`, `0x6AB` | `38` px | `0x4DE = 9`; Random plus map-limited starts `1..8` | no scroll for standard 9-row maximum |
| team | `0x76D..0x774` | `38` px | `0x4DE = 9`; optional None plus A-D | no scroll for standard 5-row maximum |

Active in YR: Yes for the listed standard offline families. Evidence: player rows populated in `FUN_006AE6E0`; side `FUN_004E3A00` sends `0x4DE=7`; color `FUN_004E45A0`, start `FUN_004E50C0`, and team `FUN_004E5B60` send `0x4DE=9`; widths from `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`.

Width-specific player-visible result:

- player dropdown content has the full 150 px width and only four rows;
- side dropdown begins at 117 px and, when it scrolls, loses 20 px to a scrollbar, leaving about 97 px list content;
- color dropdown is only 44 px wide but normally does not scroll, so the swatch-heavy rows keep the full narrow width;
- start/team dropdowns are 38 px wide and normally do not scroll; if a nonstandard path ever forced scrolling, the list content would be reduced to about 18 px after the 20 px scrollbar.

Active in YR: Yes for standard rows/no-scroll expectations; Conditional for the hypothetical narrow-scroll result. Evidence: `CreateWindowExA` width argument `combo_client_width` at `0x006181EC..0x006181F4`; scrollbar width proof below.

## 8. Scrollbar Handoff

`ComboDropWin` owns standard combo popup row paint and hit testing once the popup exists. `OwnerDraw_ListBox_00618D40` owns real owner-drawn `LISTBOX` controls such as Choose Map `0x6EB`/`0x553`; its list/scrollbar concepts are related but should not be described as the combo popup row owner.

Active in YR: Conditional. This branch is active when a dropdown is open and the row count exceeds visible capacity; side/country combos are the standard offline family most likely to reach it. Evidence: `ComboDropWin` WndProc block `0x0060D540..0x0060F311`, row loop `0x0060D846..0x0060DFC8`, and scrollbar/top-index setup around `0x0060D759..0x0060D802`.

Scrollbar width is `DAT_00AC1DF0 * 2 + 0x12`. The owner-draw setup initializes `DAT_00AC1DF0` to `1`, so the standard scrollbar width is `20` px.

Active in YR: Yes for the owner-draw shell path. Evidence: `MOV EBP,0x1` then `MOV [0x00AC1DF0], EBP` at `0x0060FA23..0x0060FA2F`; listbox width formula at `0x00618E38..0x00618E48`; scrollbar creation uses the same width at `0x0061BFD0..0x0061C45D`.

When the scrollbar is created, the listbox callback:

- creates class `"Scrollbar"` with style `0x50010001`;
- calls `FUN_0060F9A0` on the scrollbar;
- stores the list HWND into scrollbar owner-draw state;
- clears scrollbar grey byte `+0xCD`;
- sends scrollbar message `0xE9` with range/page/top fields;
- resizes the list window with `SetWindowPos`, subtracting the scrollbar width from list width;
- shows and brings the scrollbar to top.

Active in YR: Conditional. Evidence: `OwnerDraw_ListBox_00618D40` scrollbar block `0x0061BFD0..0x0061C45D`.

## 9. Hit Testing While Open

The combo callback does not draw or choose rows itself. Its custom `0x4E8` case forwards a coordinate to the active dropdown after translating from combo-relative/window coordinates into dropdown-relative coordinates using combo and dropdown window rectangles.

Active in YR: Conditional, only while state `+0xF4` has an active dropdown. Evidence: combo `0x4E8` case `0x00618A79..0x00618ADF`.

The dropdown/listbox `0x4E8` handler returns `-1` when X is outside client width or Y is outside client height. Otherwise it computes:

```text
index = top_index + y / item_height
return index if 0 <= index < item_count else -1
```

Active in YR: Conditional, only for an open `ComboDropWin`. Evidence: `ComboDropWin` custom `0x4E8` handling in `0x0060F297..0x0060F307`, which returns `-1` outside client bounds and otherwise caps `top_index + y / item_height` to the last row.

## 10. Coverage Ledger

| Area | Status | Evidence | Remaining |
|---|---|---|---|
| `ComboDropWin` class registration | verified | `FUN_0060D450 @ 0x0060D450` | none |
| dropdown open/close state `+0xF4` | verified | `0x00617EAA..0x00618481` | none |
| create rect x/y/width/height | verified | `0x00618172..0x00618205` | runtime screenshot not captured |
| row height formula | verified | `0x00618AEA..0x00618B46`, `0x006180FB..0x00618118` | none |
| `0x4DE` storage and cap math | verified | `0x00618A67..0x00618A6E`, `0x0061813F..0x0061815D` | none |
| standard combo-family caps | verified | `FUN_006AE6E0`, `FUN_004E3A00`, `FUN_004E45A0`, `FUN_004E50C0`, `FUN_004E5B60` | exact house-count-dependent side row count varies by loaded rules |
| scrollbar creation/width | verified | `ComboDropWin` setup `0x0060D759..0x0060D802`; follow-up `SKIRMISH_COMBODROPWIN_0060D540_FUNCTION_BOUNDARY_GHIDRA_REPORT.md` | none for combo popup geometry |
| hit-test forwarding | verified | combo `0x00618A79..0x00618ADF`; `ComboDropWin` `0x4E8` case `0x0060F297..0x0060F307` | none for geometry |

## 11. Open Questions

[RESOLVED] OQ1 - Exact dropdown row internal text/swatch layout is owned by `ComboDropWin`, not `OwnerDraw_ListBox_00618D40`. The follow-up `SKIRMISH_COMBODROPWIN_0060D540_FUNCTION_BOUNDARY_GHIDRA_REPORT.md` verifies popup row paint in `0x0060D846..0x0060DFC8`: text starts at content `x+3`, selected fill covers the full row, and scrollbar-shrunken content width is respected. Real `LISTBOX` row paint remains a separate path documented by `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`.

[DEFERRED] OQ2 - Screenshot-level validation of actual 640/800/high-res clipped dropdowns. Reason: this pass used static binary evidence only; runtime capture belongs to a validation trace.

## Sources

- Ghidra read-only decompile: `OwnerDraw_ComboBox_00617250 @ 0x00617250`.
- Ghidra read-only decompile: `OwnerDraw_ListBox_00618D40 @ 0x00618D40`.
- Ghidra read-only decompile: `FUN_0060D450 @ 0x0060D450`.
- Ghidra read-only decompile: `FUN_00775690 @ 0x00775690`.
- Ghidra read-only decompile: `FUN_006AE6E0`, `FUN_004E3A00`, `FUN_004E45A0`, `FUN_004E50C0`, `FUN_004E5B60`.
- Prior docs cross-checked: `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`, `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`, `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`.
