# Skirmish Combo Owner-Draw Geometry - Ghidra Research Report

**Address(es):** `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `FUN_00620720 @ 0x00620720`, `FUN_0072A9E0 @ 0x0072A9E0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** collapsed owner-draw geometry for Skirmish dialog `0x102` slot-table combo controls, including arrow placement, text fit/draw rects, swatch fill placement, dropdown face/list boundary, and width differences between player/side/color/start/team combo families.
**Non-Scope:** combo population semantics except where messages change geometry; full dropdown listbox row paint; flag static rendering; map preview markers; final high-resolution shell hosting origin.
**Confidence:** High for callback-relative geometry and Skirmish control widths; Medium for final on-screen origin because that belongs to the separate high-resolution shell-hosting slot.
**Active in YR:** Yes. Evidence: `FUN_0060F9A0 @ 0x0060F9A0` assigns class `"ComboBox"` controls to `OwnerDraw_ComboBox_00617250`; `FUN_006AE6E0 @ 0x006AE6E0` initializes offline Skirmish dialog `0x102` combo IDs.

## 1. Overview

Skirmish slot-table combo boxes use the Windows dialog template for their outer control placement, but `OwnerDraw_ComboBox_00617250` paints the collapsed face itself. The resource height (`73` or `74` dialog units) is not the painted face height; the owner-draw paint path uses a fixed `24` pixel collapsed face, reserves `20` pixels on the right for the arrow zone, and truncates text against the non-arrow width.

Active in YR: Yes. The path is reached from standard offline Skirmish dialog `0x102` through the shell owner-draw subclass setup. Evidence: `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_006AE6E0 @ 0x006AE6E0`, prior dialog resource report for `RT_DIALOG 0x102`.

## 2. Skirmish Slot Combo Control Widths

The Skirmish dialog resource is `533x369` dialog units with `MS Sans Serif` 8 pt. Prior resource extraction established the shell base units used by this dialog as `baseX=6`, `baseY=13`, mapping the full resource to `800x600`.

Active in YR: Yes. Evidence: `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md` PE `RT_DIALOG 0x102`; `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md` DLU conversion.

| Combo family | Control IDs | DLU width | Pixel width at 800x600 shell | Resource dropdown height | Owner-draw collapsed face |
|---|---|---:|---:|---:|---:|
| player/AI rows 1-7 | `0x50B`, `0x50E`, `0x516`, `0x51A..0x51D` | `100` | `150` px | `74 DLU -> 120 px` | `24` px |
| side/country | `0x6A1`, `0x510`, `0x513`, `0x514`, `0x51E..0x521` | `78` | `117` px | `74 DLU -> 120 px` | `24` px |
| color | `0x6A2`, `0x522..0x528` | `29` | `44` px | `73/74 DLU -> 119/120 px` | `24` px |
| start position | `0x6A3..0x6A8`, `0x6AA`, `0x6AB` | `25` | `38` px | `73 DLU -> 119 px` | `24` px |
| team | `0x76D..0x774` | `25` | `38` px | `73 DLU -> 119 px` | `24` px |

The row vertical stride is `16` dialog units, which maps to about `26` pixels. That leaves a small visual gap below the fixed `24` pixel combo face.

Active in YR: Yes. Evidence: resource row rects in `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`; paint constant `0x18` at `0x0061745C`.

## 3. Collapsed Face Paint Geometry

On `WM_PAINT`, `OwnerDraw_ComboBox_00617250` constructs the selected/collapsed face using these constants:

| Geometry element | Formula relative to combo client | Evidence | Active in YR |
|---|---|---|---|
| face height | `24` px (`0x18`) | `MOV [ESP+0x68],0x18` at `0x0061745C` | Yes |
| arrow/click reserve | rightmost `20` px (`0x14`) | mouse gate `client_width - 0x14` in decompile; text fit subtracts `0x14` at `0x00617B4E` | Yes |
| arrow top-left X | `client_width - 19` | `ADD EAX,-0x13` at `0x006178DC`; arrow helper call at `0x0061791D` | Yes |
| arrow top-left Y | `1` px | `INC EDX` before arrow rect store at `0x006178DF..0x006178FC` | Yes |
| text draw left | `client_left + 2` | `in_stack_0000008c = left + 2` in decompile before `FUN_00621040`; assembly block `0x00617BAF` vicinity | Yes |
| text fit width | `client_width - 20` | `ADD ECX,-0x14` then compare loop at `0x00617B42..0x00617BAF` | Yes |
| swatch source box before inset | non-arrow face area, `width = client_width - 20`, `height = 24` | rect values copied from `0x00617898..0x006178C1` and swatch block `0x00617A8B..0x00617ACF` | Yes for swatch-enabled combos |
| swatch inset | `2` px on each side, producing fill box `(x+2, y+2, width-4, height-4)` | `FUN_0072A9E0 @ 0x0072A9E0` called with `param_2=2` at `0x00617ACF` | Yes for swatch-enabled combos |

The frame is primitive, not PCX chrome. The callback calls `FUN_006208F0` with inset `2` at `0x00617893`; that helper draws beveled lines/fills using converted DirectDraw colors. The arrow is the PCX component.

Active in YR: Yes. Evidence: `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `FUN_006208F0 @ 0x006208F0`, `FUN_00620720 @ 0x00620720`.

## 4. Width-Specific Resulting Rectangles

At the normal 800x600 shell mapping, the callback-relative collapsed face geometry is:

| Combo family | Client width | Non-arrow text fit width | Arrow top-left | Swatch fill box if color is nonnegative |
|---|---:|---:|---:|---:|
| player/AI | `150` | `130` | `(131, 1)` | normally not enabled |
| side/country | `117` | `97` | `(98, 1)` | normally not enabled |
| color | `44` | `24` | `(25, 1)` | `(2, 2, 20, 20)` |
| start | `38` | `18` | `(19, 1)` | swatch mode set, but normal rows have no nonnegative swatch slot in this slice |
| team | `38` | `18` | `(19, 1)` | swatch mode set, but normal rows have no nonnegative swatch slot in this slice |

For color combos, the `44` px width leaves a `24` px non-arrow area and a `20x20` filled swatch after the `2` px inset. For start/team, the face geometry is the same narrow shape, but the verified population helpers do not store normal per-row swatch colors; the swatch slots remain `-1` unless a helper writes them, so the callback skips the fill.

Active in YR: Yes for geometry; Conditional for swatch fill, because it requires `0x4DD=1`, current selection in `0..49`, and a nonnegative `0x498` swatch value. Evidence: `0x00617A5D..0x00617B3F`, `FUN_004E45A0`, `FUN_004E50C0`, `FUN_004E5B60`.

## 5. Arrow Asset Selection

The arrow helper is `FUN_00620720`. The combo paint path passes direction `0` for the collapsed down arrow and passes the grey flag from owner-draw state byte `+0xCD`.

| State | PCX names | Evidence | Active in YR |
|---|---|---|---|
| normal released down arrow | `dnarrowr.pcx` | `FUN_00620720` formats `gdnarrow%c.pcx` then skips the leading `g` when grey flag is false | Yes |
| normal pressed down arrow | `dnarrowp.pcx` | `%c` is `'p'` when pressed argument is nonzero | Conditional - pressed/open state |
| grey released down arrow | `gdnarrowr.pcx` | grey flag keeps the leading `g` | Conditional - `0x4F1=1` |
| grey pressed down arrow | `gdnarrowp.pcx` | same helper with pressed argument nonzero | Conditional - grey plus pressed/open state |

The helper null-checks the cached PCX lookup before blitting, so a missing arrow asset skips the arrow blit rather than crashing inside this helper.

Active in YR: Yes/Conditional as above. Evidence: `FUN_00620720 @ 0x00620720`; call site `0x0061791D`; grey state setter `0x4F1` in `OwnerDraw_ComboBox_00617250`.

## 6. Text Placement and Truncation

Selected text is copied into the global Unicode text scratch and measured before drawing. The truncation loop repeatedly zero-terminates one UTF-16 code unit from the end until `BitFont__GetTextWidth` reports a width no greater than `client_width - 20`.

The draw call uses `FUN_00621040`; that function centers vertically only when its flags include bit `4`. The combo path did not prove a special per-control horizontal offset beyond left `+2` and the text-width truncation.

Active in YR: Yes. Evidence: `0x00617B42..0x00617BAF` truncation loop; `FUN_00621040 @ 0x00621040`.

Player-visible implication:

- Wide player/AI combos have `130` px of text-fit width.
- Side/country combos have `97` px of text-fit width.
- Color combos have only `24` px of text-fit width and are visually dominated by the swatch fill.
- Start/team combos have `18` px of text-fit width, enough for compact numeric/team labels only.

Active in YR: Yes. Evidence: dialog widths from `RT_DIALOG 0x102` and text-fit subtraction at `0x00617B4E`.

## 7. Swatch Placement and Gating

The collapsed swatch is drawn only when all guard conditions pass:

1. owner-draw state byte `+0xCC` is nonzero, set by custom message `0x4DD`;
2. current selection is `>= 0`;
3. current selection is `< 0x32`;
4. per-item swatch slot at `state + 0x110 + selection*4` is nonnegative.

When those pass, the callback copies the non-arrow face box, calls `FUN_0072A9E0(rect, 2)`, converts the stored RGB-like DWORD through the active DirectDraw channel shifts/losses, fills the inset box, then draws selected text afterward.

Active in YR: Yes for color combos; Conditional for other combo families. Evidence: `0x00617A5D..0x00617B3F`; custom `0x4DD` / `0x498` handling at `0x00618A13..0x00618A46`; `FUN_0072A9E0 @ 0x0072A9E0`.

Important narrow-combo result: a `44` px color combo produces a `20x20` swatch fill at `(2,2)` before text draw. A `38` px start/team combo would produce a `14x20` fill if a nonnegative swatch slot were set, but standard normal start/team population in the checked helpers does not set such slots.

Active in YR: Yes/Conditional as above. Evidence: `FUN_004E45A0` stores color swatches with `0x498`; `FUN_004E50C0` and `FUN_004E5B60` enable swatch mode but do not store normal per-row swatch colors in the decompiled normal paths.

## 8. Dropdown Boundary

The collapsed combo click target is not the entire face. On `WM_LBUTTONDOWN` / `WM_LBUTTONDBLCLK`, the callback toggles the dropdown only when mouse X is greater than `client_width - 20`.

Active in YR: Yes. Evidence: decompiled mouse branch in `OwnerDraw_ComboBox_00617250`, condition `in_stack_000000d8 + -0x14 < LOWORD(lParam)`.

When opened through `CB_SHOWDROPDOWN` / `0x14F`, the callback creates a `ComboDropWin` child popup. The popup width is the combo client width. Height is derived from `CB_GETITEMHEIGHT`, item count, max visible rows from custom `0x4DE`, and available lower screen/client space, then rounded down to a whole multiple of row height.

Active in YR: Yes. Evidence: open branch around `0x006180xx..0x00618481`; `CreateWindowExA(..., "ComboDropWin", width=combo_client_width, height=height - height % item_height, ...)`.

This report does not claim dropdown row internal paint geometry. Standard combo popup row paint belongs to the registered `ComboDropWin` WndProc; `OwnerDraw_ListBox_00618D40` is the separate real `LISTBOX` owner-draw path, including Choose Map `0x6EB`/`0x553`.

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `OwnerDraw_ComboBox_00617250` collapsed paint | verified | decompile `0x00617250`; assembly `0x0061745C`, `0x00617893`, `0x006178C1..0x0061791D`, `0x00617A5D..0x00617BAF` | none for claimed geometry |
| fixed 24 px face height | verified | `0x0061745C` | none |
| 20 px arrow reserve and click gate | verified | mouse branch and text-fit branch `0x00617B4E` | none |
| arrow PCX top-left and names | verified | `0x006178DC..0x0061791D`; `FUN_00620720 @ 0x00620720` | exact PCX pixel dimensions not extracted here |
| primitive combo frame | verified | `FUN_006208F0 @ 0x006208F0` call at `0x00617893` | exact line colors are broader owner-draw chrome context |
| selected text truncation | verified | `0x00617B42..0x00617BAF`; `FUN_00621040` | screenshot-level font comparison deferred |
| swatch fill geometry | verified | `0x00617A5D..0x00617B3F`; `FUN_0072A9E0 @ 0x0072A9E0` | none for collapsed fill rect |
| Skirmish control widths | verified | `RT_DIALOG 0x102` prior resource extraction | final high-resolution origin owned by slot 1 |
| dropdown popup sizing boundary | verified | `0x14F` open branch; `ComboDropWin` creation | `ComboDropWin` row paint geometry out-of-scope |

## 10. Open Questions - Final State

[RESOLVED] OQ1 - Does the callback use a fixed collapsed face height or the resource combo height? It uses a fixed `24` px paint face; resource height is dropdown capacity. Evidence: `0x0061745C` and dialog resource combo heights.

[RESOLVED] OQ2 - Is the combo frame an asset? No, the frame is primitive through `FUN_006208F0`; the arrow is PCX art. Evidence: call at `0x00617893`; `FUN_00620720`.

[RESOLVED] OQ3 - What is the arrow reserve? `20` px; arrow top-left is at `client_width - 19, y=1`. Evidence: `0x006178C1`, `0x006178DC..0x0061791D`.

[RESOLVED] OQ4 - Where is selected text constrained? It starts at left `+2` and is truncated to fit `client_width - 20`. Evidence: `0x00617B42..0x00617BAF`.

[RESOLVED] OQ5 - Where is the color swatch drawn? In the non-arrow face area after a `2` px inset; for a `44` px color combo this is `(2,2,20,20)`. Evidence: `0x00617A8B..0x00617ACF`; `FUN_0072A9E0`.

[RESOLVED] OQ6 - Exact standard combo popup row internal text/swatch geometry is outside this collapsed-geometry slot but no longer owner-ambiguous: `ComboDropWin` owns popup row paint and hit testing; `OwnerDraw_ListBox_00618D40` owns real `LISTBOX` controls such as Choose Map `0x6EB`/`0x553`. Follow-up: `SKIRMISH_COMBODROPWIN_0060D540_FUNCTION_BOUNDARY_GHIDRA_REPORT.md`.

[DEFERRED] OQ7 - Final shell origin at resolutions above 800x600. Reason: assigned to swarm slot 1. Category: out-of-scope.

## Sources

- Ghidra read-only decompile: `OwnerDraw_ComboBox_00617250 @ 0x00617250`.
- Ghidra read-only decompile: `FUN_00620720 @ 0x00620720`.
- Ghidra read-only decompile: `FUN_006208F0 @ 0x006208F0`.
- Ghidra read-only decompile: `FUN_00621040 @ 0x00621040`.
- Ghidra read-only decompile: `FUN_0072A9E0 @ 0x0072A9E0`.
- Ghidra read-only decompile: `FUN_0060F9A0 @ 0x0060F9A0`.
- Ghidra read-only decompile: `FUN_006AE6E0 @ 0x006AE6E0`.
- Ghidra read-only decompile: `FUN_004E45A0`, `FUN_004E50C0`, `FUN_004E5B60`, `FUN_004E5260`, `FUN_004E5CB0`.
- Prior docs: `docs/research/skirmish-ui/SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`.
- Prior docs: `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`.
- Prior docs: `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`.
- Prior docs: `docs/research/skirmish-ui/SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`.
