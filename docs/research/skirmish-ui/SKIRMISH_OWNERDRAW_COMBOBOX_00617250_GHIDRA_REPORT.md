# Skirmish Owner-Draw ComboBox `0x00617250` - Ghidra Research Report

**Address(es):** `OwnerDraw_ComboBox_00617250 @ 0x00617250`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR offline Skirmish dialog `0x102` combo controls that use `OwnerDraw_ComboBox_00617250`: player/AI, side/country, color, start, and team combos. This report covers selected-row paint, dropdown creation, listbox handoff, arrow PCX names/states, text rendering, item data, disabled/grey behavior, and boundaries to `OwnerDraw_ListBox_00618D40` / scrollbar callbacks.  
**Non-Scope:** color table population internals beyond messages and item/swatch data consumed by the combo callback; full listbox row renderer internals; static flag rendering beyond combo item-data handoff; screenshot/runtime pixel capture.  
**Confidence:** High for combo callback state/messages, dropdown creation, arrow PCX selection, selected text/swatch drawing, item-record wrapping, and Skirmish control IDs. Medium for exact semantic labels of some start/team list entries because this pass traced values through combo population helpers only as far as callback consumption.  
**Active in YR:** Yes. Evidence: `FUN_0060F9A0 @ 0x0060F9A0` assigns `"ComboBox"` controls to `0x00617250`; `FUN_006AE6E0 @ 0x006AE6E0` initializes offline Skirmish dialog `0x102` combo controls; `FUN_006ACEE0 @ 0x006ACEE0` routes Skirmish combo `WM_COMMAND` notifications.

## 1. Overview

`OwnerDraw_ComboBox_00617250` is the live owner-draw subclass callback for the standard offline Skirmish dialog's combo boxes. It does not populate Skirmish values by itself; Skirmish setup helpers feed it strings, item data, swatch colors, max dropdown row count, and grey/alternate state through Win32 combo messages plus custom owner-draw messages.

Player-visible output comes from three linked surfaces:

1. the collapsed combo paints a primitive frame, arrow PCX, selected row text, and optional selected color swatch;
2. `CB_SHOWDROPDOWN` / `0x14F` creates a custom `ComboDropWin` popup and hands the rows to the owner-draw listbox path;
3. `OwnerDraw_ListBox_00618D40` optionally creates a child `ScrollBar`, which uses the owner-draw scrollbar callback and its PCX grip/arrow art.

Every material finding below is active in YR for dialog `0x102` unless explicitly marked conditional.

## 2. Key State Offsets

Offsets are from the callback state pointer used inside `0x00617250` (`WindowExtra` record start + 4), not from the HWND hash-entry base.

| Offset | Purpose | Evidence | Active in YR |
|---:|---|---|---|
| `+0x10` | cached/backing `BSurface` pointer for collapsed paint | `0x006175xx..0x006178xx` paint path allocates/reuses surface and increments `DAT_00AC48B4` | Yes - `WM_PAINT` path |
| `+0x34` | linked list of owner-draw item records | add-string paths `0x4C1/0x4C2` allocate records and push list head | Yes - Skirmish helpers add rows via `0x4C2` |
| `+0xCC` | swatch drawing enabled flag set by `0x4DD` | assembly `0x00618A3B..0x00618A46`; paint reads `byte [state+0xCC]` at `0x00617A5D` | Yes - color/start/team helpers may enable; color path does |
| `+0xCD` | grey/alternate flag set by `0x4F1`; passed to arrow helper | assembly `0x00618A51..0x00618A5C`; paint reads at `0x006178C4`; arrow helper receives it | Conditional - Skirmish sets it for restricted/closed rows |
| `+0xD0` | max dropdown row count set by `0x4DE` | assembly `0x00618A67..0x00618A6E`; dropdown height code reads `state[0x34]` | Yes - Skirmish helpers set `7` or `9` |
| `+0xF4` | active dropdown HWND | open/close paths around `0x00617Fxx..0x006184xx`, close path clears it | Yes - only nonzero while dropdown open |
| `+0xF8` | stored current selection | `CB_GETCURSEL 0x147` returns `state+0xF8`; `CB_SETCURSEL 0x14E` writes it | Yes |
| `+0x110 + index*4` | per-item swatch color slots | custom `0x498` store at `0x00618A13..0x00618A2F`; paint reads at `0x00617A7C` | Yes - color combos use it |

## 3. Core Callback Behavior

### Initialization (`0x497`)

Active in YR: Yes. Evidence: `FUN_0060F9A0 @ 0x0060F9A0` sends `0x497` after installing the subclass for all matched controls.

On `0x497`, the combo callback:

- sets combo item heights with `CB_SETITEMHEIGHT` / `0x153`;
- uses the active font height from the state font record (`font+0x1C`);
- sets default item height to `font_height + 2`;
- sets selection-field height to `font_height + 6`;
- avoids repeating height setup if state `+0x1C` already marks initialized and `CB_GETITEMHEIGHT` / `0x154` already matches `font_height + 6`;
- stores current selection `-1` at state `+0xF8`;
- fills exactly 50 swatch slots at state `+0x110` with `-1`.

Boundary detail: the `0x498` setter accepts index `<= 0x32` (`0..50`), but initialization fills exactly `0x32` dwords (`0..49`). Paint only reads selected indices `< 0x32`, so standard Skirmish painting never reads index `50`. Evidence: assembly `0x00618A13..0x00618A2F`, `0x00618AEA` init block, paint guard `0x00617A73..0x00617A7C`.

### Item String and Item Data Records

Active in YR: Yes. Evidence: Skirmish init helpers add rows using custom `0x4C2` and assign data through `0x151`.

The callback wraps normal combo item data. For add-string paths (`0x4C1/0x4C2` and related custom string messages), it calls the previous combo WndProc to insert a row, allocates an owner-draw item record of `string_length * 2 + 0x12` bytes, then stores that record pointer into real Win32 item data via `CB_SETITEMDATA` / `0x151`.

The record fields consumed by this callback are:

| Record offset | Purpose | Evidence | Active in YR |
|---:|---|---|---|
| `+0x00` | next owner-draw record in state linked list | add-string allocation path pushes previous state `+0x34` head | Yes |
| `+0x04` | caller item data returned/set by wrapped `0x150/0x151/0x199/0x19A` | wrapper block `0x00617D55` and decompile cases `0x150/0x151/0x199/0x19A` | Yes |
| `+0x08` | string pointer for selected text when record flag is nonzero | `CB_SETCURSEL 0x14E` sends `0x4B2` with record `+0x08` | Yes |
| `+0x0C` | string kind flag (`0` means convert/copy before display; nonzero uses pointer directly) | `CB_SETCURSEL` branch at `0x00617D55..0x00617E62` | Yes |

The external API visible to Skirmish helpers remains normal-looking: `0x150` returns the caller's data field, and `0x151` updates that field. Internally, the real Win32 item data is the record pointer.

### `CB_SETCURSEL` / Selected Text

Active in YR: Yes. Evidence: Skirmish init and population helpers call `0x14E`; selected paint reads the stored selection.

On `CB_SETCURSEL` / `0x14E`:

- state `+0xF8` is written with the requested selection;
- if selection is `-1`, it sends blank text through custom `0x4B4`;
- otherwise it retrieves the wrapped item record through the previous WndProc `CB_GETITEMDATA` / `0x150`;
- record `+0x0C == 0` uses a conversion/copy path and sends `0x4B4`;
- record `+0x0C != 0` sends record `+0x08` through `0x4B2`;
- it invalidates the combo with erase disabled;
- if style low bits are not `2`, it falls through to the previous WndProc after invalidation; style low bits `2` return directly.

Evidence: assembly `0x00617D55..0x00617E9B`.

### Collapsed Combo Paint

Active in YR: Yes. Evidence: `WM_PAINT 0x0F` path in `0x00617250`; all dialog `0x102` combo controls are class `"ComboBox"` and are hooked by `FUN_0060F9A0`.

Paint order for the collapsed combo:

1. sends `CB_GETDROPPEDSTATE` / `0x157`;
2. computes client/window rectangles and parent-relative background source;
3. creates or reuses a cached `BSurface` sized to the combo client area;
4. copies/alpha-blends the parent background into the combo surface;
5. reads `GWL_STYLE`; `WS_DISABLED` (`0x08000000`) is remembered for a later alpha overlay;
6. draws a primitive beveled frame through `FUN_006208F0`;
7. builds an arrow rect on the right side and calls `FUN_00620720`;
8. if disabled, applies `AlphaBlendRect` after arrow/frame drawing;
9. if the combo style low bits are not `3`, it validates and returns without selected text draw;
10. for style low bits `3`, it reads the current selection from the previous WndProc, fetches the owner-draw item record, copies selected text into the global text scratch, optionally draws a swatch, truncates text until it fits, then calls `FUN_00621040`.

Important width constants:

- arrow/right reserved area is `0x14` pixels; click toggling and text fitting both use `client_width - 0x14`;
- arrow destination starts at `right - 0x13`, with a one-pixel inset after rect setup;
- selected text rect starts at client-left `+2`;
- selected text fit loop measures with `BitFont__GetTextWidth` and repeatedly zero-terminates one UTF-16 code unit from the end until width `<= client_width - 0x14`.

Evidence: primary decompile `0x00617250`; assembly `0x006178A0..0x0061791D`, `0x00617A5D..0x00617B3F`, `0x00617B42..0x00617BAF`.

### Selected Swatch Drawing

Active in YR: Yes for Skirmish color combos; conditional for other combos that set `0x4DD`. Evidence: color/start/team helpers send `0x4DD`; paint guard is at `0x00617A5D`.

The collapsed swatch is drawn only if all conditions hold:

- state byte `+0xCC` is nonzero;
- current selected index is `>= 0`;
- selected index is `< 0x32`;
- per-item swatch slot at `+0x110 + index*4` is not negative.

When the swatch value is nonnegative, the callback converts the stored RGB-like value through the active DirectDraw channel loss/shift globals and fills a small rectangle before drawing text. A swatch slot of `-1` suppresses this fill.

Evidence: assembly `0x00617A5D..0x00617B3F`; color-population follow-up report cross-check for `0x498` sender values.

### Arrow PCX Names and States

Active in YR: Yes. Evidence: `0x0061791D` calls `FUN_00620720`; `FUN_00620720 @ 0x00620720` formats and looks up the arrow PCX.

`FUN_00620720(surface, rect, direction, pressed, grey_flag)` chooses:

| Condition | Format string | Resulting PCXs |
|---|---|---|
| down arrow, normal | `"dnarrow%c.pcx"` via pointer offset into `"gdnarrow%c.pcx"` | `dnarrowr.pcx`, `dnarrowp.pcx` |
| down arrow, grey | `"gdnarrow%c.pcx"` | `gdnarrowr.pcx`, `gdnarrowp.pcx` |
| up arrow, normal | `"uparrow%c.pcx"` via pointer offset into `"guparrow%c.pcx"` | `uparrowr.pcx`, `uparrowp.pcx` |
| up arrow, grey | `"guparrow%c.pcx"` | `guparrowr.pcx`, `guparrowp.pcx` |

`%c` is `'r'` when `pressed == 0` and `'p'` when `pressed != 0`. The combo collapsed paint passes the grey flag from state `+0xCD`. The helper null-checks the PCX lookup before blitting, so missing arrow PCXs skip arrow blit rather than dereferencing.

Evidence: decompiled `FUN_00620720 @ 0x00620720`; string xrefs `0x008363D0` / `0x008363E0`; call site `0x0061791D`.

### Disabled and Grey Behavior

Active in YR: Yes for disabled style; conditional for grey state. Evidence: Skirmish init disables dependent side/color/start/team controls for closed/unused slots; helpers explicitly send `0x4F1`.

There are two separate states:

- Win32 disabled style (`WS_DISABLED`, `0x08000000`) is read during paint and causes an alpha overlay after frame/arrow draw. It does not switch the combo to a separate disabled frame asset.
- Owner-draw grey state (`0x4F1`, state `+0xCD`) changes arrow PCX family to `g*` variants and changes text/color global selection before swatch/text drawing.

The restricted helper paths for side/start/team/color add one row, set swatch color `-1` where applicable, select row `0`, then send `0x4F1` with `1`. Normal population paths send `0x4F1` with `0`.

Evidence: `0x00617A31..0x00617A57` text color selection around grey flag; `0x006178C4..0x0061791D` arrow call; `0x00618A51..0x00618A5C` grey setter; helper functions `FUN_004E3B90`, `FUN_004E5260`, `FUN_004E5CB0`.

### Dropdown Creation and Close

Active in YR: Yes. Evidence: `WM_LBUTTONDOWN`/`WM_LBUTTONDBLCLK` branch posts `0x14F`; open branch creates `"ComboDropWin"`; `FUN_0060D450 @ 0x0060D450` registers that window class.

Mouse open behavior:

- on `WM_LBUTTONDOWN 0x201` or `WM_LBUTTONDBLCLK 0x203`, it plays a click sound;
- it toggles dropdown only when mouse X is greater than `client_width - 0x14`;
- it checks current dropped state via `0x157` and posts `0x14F` with the inverse state.

Open path (`0x14F`, `wParam != 0`):

- returns immediately if state `+0xF4` already has an active dropdown;
- computes dropdown row height from `CB_GETITEMHEIGHT 0x154`;
- reads row count from `CB_GETCOUNT 0x146`, clamping a count below `1` to `1`;
- applies max visible rows from state `+0xD0` when that value is `>= 1` and below/equal the row count;
- if no max is set, uses available lower screen/client space and row height to cap rows;
- clamps popup height against available bottom boundary;
- creates a child popup with class `"ComboDropWin"`, style `0x40000000`, width equal to combo client width, height rounded down to an exact multiple of row height;
- stores the original combo HWND as `lpParam` to `CreateWindowExA`;
- installs a synthetic/default owner-draw state record if the popup is not already in the state table;
- sends `0x7E8` to the popup;
- notifies the combo parent with `0x4A9`, `wParam = dropdown_hwnd`, `lParam = 1`;
- calls `SetCapture(dropdown_hwnd)`, `ShowWindow(dropdown_hwnd, 1)`, and stores dropdown HWND at state `+0xF4`.

Close path (`0x14F`, `wParam == 0`):

- returns `1` if no dropdown is active;
- releases capture;
- notifies the combo parent with `0x4A9`, `wParam = dropdown_hwnd`, `lParam = 0`;
- destroys the dropdown window;
- releases the dropdown's cached surface/state from owner-draw hash tables;
- clears state `+0xF4` to `0`.

Evidence: `0x00617Fxx..0x00618481` open branch; `0x00617EAA..0x006180xx` close branch; class registration `FUN_0060D450 @ 0x0060D450`; string xrefs to `ComboDropWin @ 0x008357A0`.

## 4. Skirmish Combo Inputs Consumed by the Callback

The callback consumes strings, item data, swatch slots, grey flag, and max row count. It does not know the gameplay meaning of each row except through item data returned by `0x150`.

| Combo family | Control IDs | Callback inputs | Item data consumed | Active in YR |
|---|---|---|---|---|
| player/AI | `0x50B`, `0x50E`, `0x516`, `0x51A..0x51D` | rows added via `0x4C2`; current selection via `0x14E`; no swatch setup seen | `-1`, `2`, `1`, `0` from `FUN_006AE6E0` | Yes - `FUN_006AE6E0`, `FUN_006ACEE0` |
| side/country | `0x6A1`, `0x510`, `0x513`, `0x514`, `0x51E..0x521` | normal rows set max rows `7`, grey flag `0`; restricted observer row sets grey flag `1` | `-3` observer, `-2` random, `0..9` countries | Yes - `FUN_004E3B90`, `FUN_004E3A00`, `FUN_004E3690` |
| color | `0x6A2`, `0x522..0x528` | swatches enabled `0x4DD=1`; max rows `9`; per-row swatches through `0x498`; grey flag normal/restricted | `-2` special/random row, `0..8` color IDs | Yes - callback consumption verified; population internals owned by slot 2 |
| start | `0x6A3..0x6A8`, `0x6AA`, `0x6AB` | swatches enabled, max rows `9`; normal path stores no per-row colors in this pass; restricted path stores `-1` swatch and grey flag `1` | `-2` special/random row, `0..8` start-list IDs | Yes - `FUN_004E50C0`, `FUN_004E5260`, `FUN_004E5480`, `FUN_004E5700` |
| team | `0x76D..0x774` | swatches enabled, max rows `9`; normal path may include optional `-2` row, then team rows; restricted path swatch `-1` and grey flag `1` | normal rows `0..3`; optional `-2` row | Yes - `FUN_004E5B60`, `FUN_004E5CB0`, `FUN_004E5ED0` |

Player/AI rows are populated directly in `FUN_006AE6E0`: the callback only stores and returns their item data. `FUN_006ACEE0` later treats selected item data `0`, `1`, or `2` as active AI/player-like slot values for launch validation, while `-1` is not counted.

Side/country item data is later used by `FUN_004E3690` / `FUN_004E3CE0` to update flag statics; values outside `-3..9` trigger a fallback call through `DAT_00A8B23C+0x28` before the flag PCX lookup. The combo callback itself only returns those item data values through `0x150`.

Start and team helpers use global 12-byte-row tables (`DAT_008B3F30..` and `DAT_008B3FC0..`) with an owner/slot field. Normal population includes rows whose owner is this slot or `-1`; selection handlers clear the old owner and write the selecting slot unless item data is `-2`. This mirrors the color ownership pattern but this report only claims the values sent to and returned by the combo callback.

## 5. ListBox and Scrollbar Boundary

Active in YR: Yes when a dropdown is open; scrollbar is conditional on visible row capacity. Evidence: `0x00617250` creates `ComboDropWin`; the follow-up `SKIRMISH_COMBODROPWIN_0060D540_FUNCTION_BOUNDARY_GHIDRA_REPORT.md` verifies that the registered `ComboDropWin` WndProc block at `0x0060D540..0x0060F311` owns standard combo popup row paint, hit testing, top-index clamping, and scrollbar synchronization.

The combo callback's boundary with the dropdown list is:

- it creates the `ComboDropWin` window and owner-draw state;
- it stores the dropdown HWND at state `+0xF4`;
- it sends parent message `0x4A9` on open/close;
- it forwards hit-testing through custom `0x4E8`: combo converts screen/client coordinates into dropdown-relative coordinates and sends `0x4E8` to the dropdown;
- it does not draw the dropdown rows itself.

The registered `ComboDropWin` WndProc owns dropdown row paint, selection fill, hit testing, list item records sourced from the combo, and optional child-scrollbar synchronization. Its `0x4E8` handler returns `-1` when X/Y is outside client bounds and otherwise returns `min(item_count - 1, top_index + y / item_height)`.

When combo popup contents need scrolling, `ComboDropWin` creates class `"Scrollbar"` with style `0x50010001`, calls `FUN_0060F9A0` on it, syncs top-index/range state, and shrinks the popup content width by the scrollbar width. The scrollbar callback `OwnerDraw_ScrollBar_0061C690` then paints `sbgript/m/b.pcx` or `gsbgript/m/b.pcx` plus the same up/down arrow helper. Combo code does not call scrollbar drawing directly.

Evidence: `ComboDropWin` WndProc block `0x0060D540..0x0060F311`, row paint `0x0060D846..0x0060DFC8`, hit test `0x0060F297..0x0060F307`, scrollbar sync `0x0060E648..0x0060E821`, and scrollbar callback `0x0061C690`.

## 6. Integration Points

| Function / address | Role | Active in YR |
|---|---|---|
| `FUN_0060F9A0 @ 0x0060F9A0` | assigns `"ComboBox"` to `OwnerDraw_ComboBox_00617250`, installs universal subclass, sends `0x497` | Yes - shell dialog child enumeration |
| `FUN_0060D450 @ 0x0060D450` | registers `"ComboDropWin"` class with window proc `LAB_0060D540` | Yes - required before custom dropdown creation |
| `OwnerDraw_ComboBox_00617250 @ 0x00617250` | selected row paint, wrapper item data, dropdown creation, custom combo messages | Yes |
| `ComboDropWin WndProc block @ 0x0060D540..0x0060F311` | standard combo popup row paint, hit test, item records, optional scrollbar child | Conditional - only while a combo dropdown exists |
| `OwnerDraw_ListBox_00618D40 @ 0x00618D40` | real owner-drawn `LISTBOX` controls such as Choose Map `0x6EB`/`0x553`; not the standard combo popup row owner | Conditional - only while real listbox controls exist |
| `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690` | scrollbar for dropdowns/lists needing scroll | Conditional - only when item count exceeds visible rows |
| `FUN_006AE6E0 @ 0x006AE6E0` | offline Skirmish dialog init; populates player/AI combos and calls side/color/start/team refresh helpers | Yes - dialog `0x102` init |
| `FUN_006ACEE0 @ 0x006ACEE0` | offline Skirmish command handler; routes combo notifications | Yes |

No TS-only gate was found on the combo owner-draw path. The code is part of the YR shell UI and is reached through standard dialog subclass setup.

## 7. Current Rust Implementation Status

Rust currently has partial layout/state scaffolding for dialog `0x102`, but not the gamemd combo/dropdown behavior:

- `src/ui/skirmish_shell/layout.rs:29` defines only button, preview, player-name, color-combo, and flag IDs; player/AI, side, start, and team combo IDs are not modeled there.
- `src/ui/skirmish_shell/layout.rs:167` lays out only color combos among combo controls.
- `src/ui/skirmish_shell/state.rs:16` exposes `SelectColor` as the only combo-like action.
- `src/ui/skirmish_shell/state.rs:143` hit-tests the entire color combo rectangle as an immediate color action, while gamemd toggles a dropdown only from the `client_width - 0x14` arrow zone.
- `src/ui/skirmish_shell/state.rs:177` cycles local color indices modulo 8; gamemd uses owner-draw item records, `0x150` item data, a special `-2` row, and a nine-color table for color combos.

This status is documentation only; no Rust files were changed in this investigation.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `OwnerDraw_ComboBox_00617250` primary callback | verified | decompile `0x00617250`; assembly spot checks `0x00617A5D`, `0x006181FE`, `0x00618A13` | none for claimed slice |
| `0x497` combo init | verified | `0x00618AEA` block; prior `FUN_0060F9A0` report | none |
| selected row item data wrapper | verified | `0x00617D55` wrapper and add-string allocation paths | exact string encoding helper internals deferred as utility |
| collapsed selected text paint/truncation | verified | paint block `0x00617A5D..0x00617BAF`; `FUN_00621040 @ 0x00621040` | screenshot comparison deferred |
| swatch draw and `0x498` bounds | verified | assembly `0x00617A5D..0x00617B3F`, `0x00618A13..0x00618A2F` | color population internals out of scope |
| arrow PCX selection | verified | `FUN_00620720 @ 0x00620720`; string xrefs `0x008363D0`, `0x008363E0` | none |
| dropdown open/close | verified | `0x00617Fxx..0x00618481`; `ComboDropWin` string xrefs | exact `LAB_0060D540` popup wndproc internals deferred; not needed for combo-created handoff |
| listbox handoff | verified | combo sends `0x7E8`, `0x4A9`, capture/show; listbox `0x4E8` hit-test verified | list row decorative columns touched-not-exhausted |
| scrollbar boundary | verified | `OwnerDraw_ListBox_00618D40` creates scrollbar and calls `FUN_0060F9A0`; `OwnerDraw_ScrollBar_0061C690` paints assets | full scrollbar drag behavior documented in prior callback report; not repeated |
| Skirmish player/AI combos | verified | `FUN_006AE6E0`, `FUN_006ACEE0` | exact localized strings not repeated |
| Skirmish side/country combos | verified | `FUN_004E3B90`, `FUN_004E3A00`, `FUN_004E3690` | side helper internals beyond item data/flag handoff out of scope |
| Skirmish color combos | touched-not-exhausted | callback consumption; prior follow-up report | slot 2 owns color population internals |
| Skirmish start/team combos | verified for callback-consumed values | `FUN_004E50C0`, `FUN_004E5260`, `FUN_004E5480`, `FUN_004E5B60`, `FUN_004E5CB0`, `FUN_004E5ED0` | exact user-facing string labels/order can be verified in a separate UX/string pass |

## 9. Open Questions - Final State

[RESOLVED] OQ1 - Is `0x00617250` active in standard YR offline Skirmish dialog `0x102`? Yes. Evidence: `FUN_0060F9A0 @ 0x0060F9A0` class dispatch; `FUN_006AE6E0 @ 0x006AE6E0` initializes the dialog's combo IDs.

[RESOLVED] OQ2 - Does the collapsed combo use PCX chrome for the frame? No. The frame is primitive via `FUN_006208F0`; only the arrow uses PCX art. Evidence: paint path `0x00617880..0x0061791D`; `FUN_00620720 @ 0x00620720`.

[RESOLVED] OQ3 - Which arrow PCX names are used? `dnarrowr/p.pcx`, `uparrowr/p.pcx`, and grey `gdnarrowr/p.pcx`, `guparrowr/p.pcx`. Evidence: `FUN_00620720 @ 0x00620720`, strings `0x008363D0`, `0x008363E0`.

[RESOLVED] OQ4 - Does disabled style equal grey arrow state? No. `WS_DISABLED` causes alpha overlay; `0x4F1` state `+0xCD` chooses grey arrow/text behavior. Evidence: `0x006178C4..0x0061792A`, `0x00618A51..0x00618A5C`.

[RESOLVED] OQ5 - What data does `CB_GETITEMDATA` return to Skirmish code? The record field at owner-draw item record `+0x04`, not the internal record pointer. Evidence: wrapper cases `0x150/0x151/0x199/0x19A`.

[RESOLVED] OQ6 - Does the combo draw dropdown rows itself? No. It creates `ComboDropWin` and hands row painting/hit-testing to listbox behavior; combo only forwards `0x4E8` hit-test when needed. Evidence: open path around `0x006181FE..0x00618481`; listbox `0x4E8` case.

[DEFERRED] OQ7 - Exact localized string text and final row order for every start/team variant. Reason: callback consumes item data and strings but does not define localization; this requires a focused UI/string pass. Category: out-of-scope.

[DEFERRED] OQ8 - Full `LAB_0060D540` popup window procedure behavior. Reason: dropdown creation/listbox handoff is verified; popup proc internals were not required to establish combo callback behavior. Category: bounded-cost-too-high for this slot.

## Sources

- Ghidra decompiled/read-only: `OwnerDraw_ComboBox_00617250 @ 0x00617250`
- Ghidra decompiled/read-only: `FUN_00620720 @ 0x00620720`
- Ghidra decompiled/read-only: `FUN_00621040 @ 0x00621040`
- Ghidra decompiled/read-only: `OwnerDraw_ListBox_00618D40 @ 0x00618D40`
- Ghidra decompiled/read-only: `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`
- Ghidra decompiled/read-only: `FUN_0060D450 @ 0x0060D450`
- Ghidra decompiled/read-only: `FUN_006AE6E0 @ 0x006AE6E0`
- Ghidra decompiled/read-only: `FUN_006ACEE0 @ 0x006ACEE0`
- Ghidra decompiled/read-only: `FUN_004E3B90`, `FUN_004E3A00`, `FUN_004E3690`, `FUN_004E3CE0`
- Ghidra decompiled/read-only: `FUN_004E50C0`, `FUN_004E5260`, `FUN_004E5480`, `FUN_004E5700`
- Ghidra decompiled/read-only: `FUN_004E5B60`, `FUN_004E5CB0`, `FUN_004E5ED0`
- String xrefs: `ComboDropWin @ 0x008357A0`; arrow format strings `0x008363D0`, `0x008363E0`
- Prior docs cross-checked: `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`, `FUN_0060F9A0_OWNERDRAW_SUBCLASS_SETUP_GHIDRA_REPORT.md`, `traces/SKIRMISH_PLAYER_AI_COMBOS_FLAGS_TRACE.md`, `SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`
- Rust status checked: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`
