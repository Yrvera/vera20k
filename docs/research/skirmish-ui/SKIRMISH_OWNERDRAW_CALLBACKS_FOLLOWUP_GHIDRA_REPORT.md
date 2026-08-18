---
title: Skirmish Owner-Draw Callback Follow-up (Ghidra Research Report)
date: 2026-05-16
---

# Skirmish Owner-Draw Callback Follow-up - Ghidra Research Report

## Scope

This report extends only the open gaps from:

- `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`

It does not re-document the full owner-draw callback table already covered there.
The follow-up targets were:

1. exact Skirmish button client sizes and 24px vs 30px PCX family selection;
2. PCX load/decode/conversion below the owner-draw cache;
3. color combo population, swatch storage, and `0x4DD` / `0x498` behavior;
4. disposition of `bst_*`, `bud_*`, `number*.pcx`, `BTN-MINS.SHP`, and
   `BTN-PLUS.SHP`;
5. final shell scaling/placement where recoverable.

**Active in YR:** Yes for dialog `0x102`, `FUN_006AE6E0`, `FUN_006ACEE0`,
owner-draw hook setup, owner-draw PCX preloading, button drawing, and color combo
helpers. Online-only `number*.pcx` use is active in WOL shell code, not offline
Skirmish.

**Overall confidence:** High for button DLU-to-pixel sizes, 30px PCX family
selection, `bud_*` non-use on normal Skirmish buttons, embedded-PCX-palette
conversion to 16-bit owner-draw surfaces, color combo table mechanics, and string
xrefs. Medium for final parent viewport origin beyond the 800x600 shell client,
because the resource and init path prove the dialog-relative coordinates but not
every higher-resolution shell hosting policy.

## 1. Verified Binary Findings

### 1.1 Dialog `0x102` DLU-to-pixel conversion and button family

Evidence:

- PE `RT_DIALOG` resource `0x102`, language `0x409`, from the prior layout
  report.
- Resource font: `MS Sans Serif`, 8 pt.
- Resource dialog rect: `(0,0,533,369)` dialog units.
- Button resource rects:
  - Start Game `0x617`: `(425,149,108,23)`;
  - Choose Map `0x5AA`: `(425,176,108,23)`;
  - Back `0x5C0`: `(425,346,108,23)`.
- Owner-draw button callback `OwnerDraw_Button_00612B70`.

The compiled dialog is sized so that the standard Win32 dialog base units are
`baseX=6`, `baseY=13` for this shell font:

```text
width_px  = MulDiv(dlu_x, 6, 4)
height_px = MulDiv(dlu_y, 13, 8)

dialog width:  MulDiv(533, 6, 4)  = 800 px
dialog height: MulDiv(369, 13, 8) = 600 px
button width:  MulDiv(108, 6, 4)  = 162 px
button height: MulDiv(23, 13, 8)  = 37 px
```

Therefore all three offline Skirmish owner-draw buttons have an actual Win32
client rect of `0,0,162,37` in the normal 800x600 shell path.

The button callback chooses between two size suffixes by comparing the actual
client height against two constants:

| Height threshold | Suffix candidate |
|---:|---:|
| `0x18` / 24 | `24` |
| `0x1E` / 30 | `30` |

The selection loop uses the larger suffix when the client height is at least
`30`. A 37 px client height therefore selects the `30` family for normal
Skirmish:

- unpressed: `bue_li30.pcx`, `bue_mi30.pcx`, `bue_ri30.pcx`;
- pressed: `bde_li30.pcx`, `bde_mi30.pcx`, `bde_ri30.pcx`.

The `24` family is still preloaded and reachable for smaller owner-draw buttons,
but these three Skirmish `0x102` buttons do not use it at normal shell scaling.

### 1.2 Final dialog-relative positions for the three buttons

Evidence: same resource and DLU conversion as above.

Using `baseX=6`, `baseY=13`, the dialog-relative rectangles are:

| Control | DLU rect | Pixel rect in 800x600 shell client |
|---|---:|---:|
| `0x617` Start Game | `(425,149,108,23)` | `(638,242,162,37)` |
| `0x5AA` Choose Map | `(425,176,108,23)` | `(638,286,162,37)` |
| `0x5C0` Back | `(425,346,108,23)` | `(638,562,162,37)` |

Tiny placement detail: `x=425` maps to `638`, not `637`, because Win32
`MulDiv(425,6,4)` rounds `637.5` to `638`. The dialog width maps to exactly the
800 px shell client width after the same rounding.

The resource style is a child-style shell dialog (`0x40000040`, with `DS_SETFONT`
and child-window style); the recovered `WM_INITDIALOG` path in `FUN_00622B50`
does not apply a second scale transform to child controls. No binary evidence was
found in this pass for per-control scaling after Windows creates the dialog.

### 1.3 `bud_*` is not used for normal Skirmish button states

Evidence:

- `OwnerDraw_Button_00612B70`.
- `batch_string_anchor_report("bud_")`.
- `FUN_0061F210`.

`bud_li24/mi24/ri24` and `bud_li30/mi30/ri30` have xrefs only from the preload
function `FUN_0061F210`.

The normal button paint path formats filenames with:

```text
b%c%c_li%d.pcx
b%c%c_mi%d.pcx
b%c%c_ri%d.pcx
```

The first `%c` is state:

- `'u'` when released/unpressed;
- `'d'` when pressed/armed.

The second `%c` is hardcoded to `'e'` on the normal enabled/disabled PCX path.
Disabled `WS_DISABLED` style (`0x08000000`) forces the first character back to
`'u'` and then applies `AlphaBlendRect(..., 0x80)` after drawing. It does not
select a `'d'` second character and does not generate `bud_*`.

Verified result for controls `0x617`, `0x5AA`, `0x5C0`:

- hover/armed/pressed visual state uses `bde_*` when the button state bit is set;
- unpressed and disabled base art uses `bue_*`;
- disabled appearance is an alpha overlay, not `bud_*`;
- `bud_*` remains preloaded-only for this Skirmish button path.

### 1.4 PCX cache load path and embedded palette conversion

Evidence:

- `FUN_0061F210` owner-draw preload.
- `FUN_006BA120`.
- loader at `0x006B9D00`, called from preload as `ECX=0x00AC4848`,
  `push name`, `push mode`, `push flag`.
- lookup helper `FUN_006BA140`.
- assembly at `0x006B9DA3..0x006BA09F`.

`FUN_0061F210` preloads owner-draw PCXs into the owner-draw cache rooted at
`0x00AC4848`. Most calls pass mode `2`; `FUN_006BA120(name)` calls the same
loader with mode `1` and flag `1`.

The loader behavior at `0x006B9D00`:

1. Zeroes a 768-byte palette scratch area (`0x100` triplets).
2. Constructs/opens a file object for the requested filename.
3. Calls `FUN_00630310`; if that returns `0`, it destructs the temporary object
   and returns `AL=0`.
4. For mode `2`, converts the 256 RGB triplets from the decoded PCX data into a
   256-entry 16-bit table using the active DirectDraw loss/shift globals:
   - red loss/shift globals around `0x008A0DD4` / `0x008A0DD0`;
   - green loss/shift globals around `0x008A0DDC` / `0x008A0DD8`;
   - blue loss/shift globals around `0x008A0DE4` / `0x008A0DE0`.
5. Allocates a destination surface record of `0x14 + width * height * 2` bytes.
6. Stores width and height from the decoded PCX metadata at destination offsets
   `+0x04` and `+0x08`.
7. Reads the decoded 8-bit indexed pixel buffer through a vtable call at
   `+0x5C`.
8. For each source pixel byte, indexes the converted 16-bit palette table and
   writes a 16-bit destination pixel.
9. Inserts a `0x30C`-byte cache entry into the hash table. The entry stores a
   pointer to the converted surface at entry `+0x04`; `FUN_006BA140` copies this
   record to scratch globals and returns that surface pointer.

No call in `FUN_0061F210`, `FUN_006BA120`, `FUN_006BA140`, or loader
`0x006B9D00` loads `DIALOG.PAL`, `SHELL.PAL`, or `MAINBTTN.PAL`. String searches
found those palettes elsewhere in the binary, but no direct xrefs from this
owner-draw PCX preload/conversion path.

Verified conclusion: owner-draw PCX controls use embedded PCX palettes decoded
from the PCX file and immediately convert indexed pixels to the active 16-bit
surface format. External shell `.PAL` files are not part of this path.

### 1.5 Missing-PCX behavior

Evidence:

- loader `0x006B9D00` early return at `0x006B9D5B..0x006B9D99`;
- lookup helper `FUN_006BA140`;
- button and checkbox callback dereferences after lookup.

If a PCX cannot be opened/decoded, `FUN_00630310` returns `0`, the loader returns
`AL=0`, and no cache entry is inserted.

Later lookup through `FUN_006BA140(name, ...)`:

- returns the converted surface pointer if a matching cache entry exists;
- returns `0` if no cache entry matches.

Missing-asset fallout is callback-specific:

| Path | Missing lookup behavior |
|---|---|
| main button cap/middle/cap | immediately dereferences the returned surface for width/blit; no robust fallback in `OwnerDraw_Button_00612B70` |
| checkbox icon | immediately dereferences the returned icon surface; no robust fallback in `OwnerDraw_Checkbox_006163A0` |
| combo arrow helper | prior report verified null checks in arrow helper |
| flag static | receives `0` image pointer and renders blank instead of crashing |

So missing Skirmish button or checkbox PCXs are not replaced by a primitive GDI
button. Missing flag PCXs are blankable because the static image path stores and
checks the image pointer differently.

### 1.6 Color combo table initialization

Evidence:

- `FUN_004E43C0`.
- static color values at `0x008316A8`.
- global table beginning at `0x008B4038`.

`FUN_004E43C0` initializes the multiplayer/skirmish color table before combo
population. It loads nine color-name strings with string IDs `0x1DB..0x1E3` from
`GDlgSupp.cpp`, copies nine raw RGB-like color values from `0x008316A8`, and
sets each color's owner slot to `-1`.

The table layout is 12 bytes per color:

| Offset in row | Meaning |
|---:|---|
| `+0x00` | color display string pointer |
| `+0x04` | swatch color value |
| `+0x08` | owning player slot index, or `-1` if available |

The nine swatch values copied from `0x008316A8` are:

| Color ID / item data | Swatch value |
|---:|---:|
| `0` | `0x000DE2DD` |
| `1` | `0x001919FF` |
| `2` | `0x00E2742A` |
| `3` | `0x002ED13E` |
| `4` | `0x0019A0FF` |
| `5` | `0x00E6D732` |
| `6` | `0x00BD2895` |
| `7` | `0x00EB9AFF` |
| `8` | `0x00606060` |

The callback later converts these values to 16-bit display format using the same
DirectDraw channel loss/shift globals before filling the swatch rectangle.

### 1.7 Color combo control mapping and population

Evidence:

- `FUN_004E4820`.
- `FUN_004E45A0`.
- `FUN_004E4770`.
- `FUN_004E49A0`.
- `FUN_004E4C20`.
- `FUN_004E4E20`.
- Skirmish dialog command handler `FUN_006ACEE0`.

Color combo control IDs map to player slots as:

| Slot | Control |
|---:|---:|
| `0` | `0x6A2` |
| `1` | `0x522` |
| `2` | `0x523` |
| `3` | `0x524` |
| `4` | `0x525` |
| `5` | `0x526` |
| `6` | `0x527` |
| `7` | `0x528` |

`FUN_004E4820` refreshes all eight color combos. For each slot:

- in game modes `3` or `4`, if the row belongs to the local player or the
  player record has field `+0x6B == -1`, it calls `FUN_004E4770`;
- otherwise it calls `FUN_004E45A0`.

`FUN_004E45A0` is the normal available-color population path:

1. Hides the combo with `ShowWindow(hwnd, 0)`.
2. Clears items with `CB_RESETCONTENT` / `0x14B`.
3. Sends custom message `0x4DD` with `lParam=1` to enable swatch drawing.
4. Sends custom message `0x4DE` with `lParam=9` to set max visible/dropdown rows.
5. Adds a first string-table row from string ID `0x20A`.
6. Sets that first row's item data to `-2`.
7. Sends custom message `0x498` for that row with color `-1`.
8. Iterates the nine color-table rows and includes a color if its owner is
   either this combo's slot or `-1`.
9. For each included color:
   - adds a row using the string pointer at `0x00822B78` as the display text;
   - sends `0x498` with the row index and the swatch color from table `+0x04`;
   - sends `0x151` to store item data equal to the color ID `0..8`;
   - remembers the selected row if the color owner is this slot.
10. Sets the current selection with `0x14E`.
11. Sends `0x4F1` with `0` to clear the grey/alternate combo state.
12. Restores visibility if the combo had been visible before.

`FUN_004E4770` is the restricted/grey path. It clears the combo, enables swatches,
sets max rows to `9`, adds one row from string ID `0x237`, assigns item data
`-2`, stores per-row color `-1`, selects row `0`, and sends `0x4F1` with `1`
to enable the grey combo state.

### 1.8 Combo owner-draw messages `0x497`, `0x498`, `0x4DD`, and swatch draw

Evidence:

- `OwnerDraw_ComboBox_00617250`.
- assembly `0x00618AEA..0x00618B61` for custom `0x497`.
- assembly `0x00618A13..0x00618A36` for custom `0x498`.
- assembly `0x00618A3B..0x00618A74` for `0x4DD`, `0x4F1`, and `0x4DE`.
- assembly `0x00617A5D..0x00617B3F` for collapsed swatch drawing.

The owner-draw combo state stores:

| State offset | Meaning |
|---:|---|
| `+0xCC` | swatch drawing enabled by `0x4DD` |
| `+0xCD` | grey/alternate combo flag set by `0x4F1` |
| `+0xD0` | max dropdown rows set by `0x4DE` |
| `+0xF8` | current selection, initialized to `-1` |
| `+0x110 + index*4` | per-item swatch color slot written by `0x498` |

Custom message `0x497` initializes the combo owner-draw state and fills 50
per-item color slots at `+0x110` with `-1` using `ECX=0x32` and `REP STOSD`.

Custom message `0x498` stores the caller-supplied color value:

```text
if (wParam <= 0x32) {
    state[0x110 + wParam * 4] = lParam;
}
```

Tiny boundary detail: initialization fills exactly 50 slots (`0..49`), but the
store guard accepts `wParam == 50`. The Skirmish color callers only use row
indices generated from the first row plus nine colors, so normal Skirmish does
not hit index `50`.

Collapsed combo paint draws a swatch only when:

- `state+0xCC` is nonzero;
- current selection is `>= 0`;
- current selection is `< 0x32`.

It reads the per-item color from `state+0x110 + selection*4`. If the color is
`-1`, the fill value remains `-1`; otherwise it converts the stored channel value
to the active 16-bit DirectDraw format and fills a small rectangle before drawing
the selected text.

### 1.9 Color change handling

Evidence:

- Skirmish command handler `FUN_006ACEE0`.
- `FUN_004E4C20`.
- `FUN_004E49A0`.
- `FUN_004E4820`.

`FUN_006ACEE0` routes `WM_COMMAND` notifications for color controls
`0x6A2`, `0x522..0x528` to `FUN_004E4C20` only when notification code is `1`.

`FUN_004E4C20`:

1. Maps the control ID to slot `0..7`.
2. Scans the nine color-table rows and clears any row whose owner equals that
   slot by writing `-1`.
3. Reads the combo current selection with `0x147`.
4. Reads selected item data with `0x150`.
5. If selected item data is not `-2`, writes this slot index into
   `color_table[item_data].owner`.
6. Calls the all-row refresh loop, repopulating every color combo so already
   claimed colors disappear from the other rows.

`FUN_004E49A0` performs the same owner-table update for programmatic selection:
it clears old ownership for the slot, scans combo items until item data equals
the requested color ID, selects that row, and if the item data is not `-2`,
marks that color as owned by the slot.

`FUN_004E4E20` is not a population function. It resolves `param_3 == -1` to the
current selected row and then sends `0x150` to retrieve item data. It is used by
tooltip/help paths to map a combo row back to its color item data.

### 1.10 `number0.pcx..number9.pcx`

Evidence:

- `FUN_0061F210` preload xrefs.
- `FUN_00783A90`.
- callers at `0x007845D1`, `0x00784663`, `0x007846E4`.
- string context at `0x0084A430`: `D:\ra2mdpost\wonline.cpp`.

`number0.pcx..number9.pcx` are preloaded by the shell owner-draw preload and are
also used by `FUN_00783A90`, a digit-to-PCX lookup helper:

| Input | Return |
|---:|---|
| `0..9` | `FUN_006BA140("numberN.pcx", 0)` |
| other | `0` |

The recovered callers are in a `wonline.cpp` code region and send the returned
digit surfaces to control `0x53E` with message `0x4A8`. They format multi-digit
online values by hundreds/tens/ones and substitute a blank surface if the value
is out of range.

Verified result: `number*.pcx` are not used by offline Skirmish dialog `0x102`.
They are preloaded globally and used by WOL/online shell UI code.

### 1.11 `bst_*`, `BTN-MINS.SHP`, and `BTN-PLUS.SHP`

Evidence:

- `batch_string_anchor_report("bst_")`.
- `list_strings("BTN-")`.
- prior retail archive probe from `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`.

`bst_uckg.pcx`, `bst_chkg.pcx`, `bst_uchk.pcx`, and `bst_chkd.pcx`:

- have string xrefs only from `FUN_0061F210`;
- were not found by the prior retail archive probe;
- are not referenced by `OwnerDraw_Checkbox_006163A0`, which uses
  `cue_i.pcx`, `cce_i.pcx`, `cce_ir.pcx`, and `cce_il.pcx`;
- are not proven for offline Skirmish or any other shell path in this pass.

`BTN-MINS.SHP` and `BTN-PLUS.SHP`:

- exist as strings at `0x0083FDB8` and `0x0083FDC8`;
- `batch_string_anchor_report` found no function xrefs for `BTN-MINS.SHP`;
- the prior archive probe did not resolve either asset in the configured RA2/YR
  install;
- no recovered owner-draw callback or Skirmish command path uses them.

Verified result: none of these assets are proven Skirmish-use assets. `bst_*`
are preload-only strings in the recovered path; `BTN-MINS.SHP` / `BTN-PLUS.SHP`
remain unresolved static strings with no current use-site evidence.

## 2. Inferred Asset Roles

These are inferences from verified code and asset naming, not independently
verified screenshot facts.

- `bue_*` means button-up enabled, and `bde_*` means button-down enabled. The
  code proves the first character is `'u'`/`'d'` and the second is `'e'`.
- `bud_*` likely denotes a disabled button family by naming convention, but the
  normal Skirmish button callback does not select it. It may belong to another
  unrecovered shell button variant or may be abandoned preload baggage.
- The color values in `0x008316A8` are intended as RGB-like swatch colors. The
  renderer treats the low, middle, and high bytes as channels and converts them
  through the active DirectDraw channel masks before filling the swatch.
- `bst_*` naming suggests button-state/check-state graphics, but no use-site was
  found. Treat that as a naming hint only.

## 3. Unresolved / Open Questions

1. **Higher-resolution shell hosting:** The 533x369 DLU dialog maps exactly to
   800x600 with `baseX=6`, `baseY=13`, and `FUN_00622B50` does not rescale child
   controls after creation. This pass did not fully prove how a larger desktop
   mode hosts the 800x600 shell client: fixed top-left, centered viewport, or
   another outer-shell transform.
2. **`bud_*` alternate use-site:** `bud_*` has no xrefs beyond preload and is not
   used by normal Skirmish buttons. A broader whole-shell callback/table audit
   would be needed to prove whether another dialog uses it.
3. **`bst_*` status:** `bst_*` strings are preload-only in the recovered code and
   were not resolved in the prior archive probe. They remain unresolved until an
   install/archive variant or hidden use-site proves otherwise.
4. **`BTN-MINS.SHP` / `BTN-PLUS.SHP`:** The strings exist but no xrefs or retail
   archive resolution were found. They remain unresolved static strings.
5. **Live screenshot validation:** The binary now proves 30px button-family
   selection and color-swatch mechanics, but a retail screenshot would still be
   useful for pixel-level confirmation of the final shell-origin policy.

## 4. Suggested Labels

These labels are documentation suggestions for future Ghidra/project naming. They
are not proof that the Ghidra database was renamed.

| Address / range | Suggested label | Kind | Evidence / reason |
|---:|---|---|---|
| `0x00612B70` | `OwnerDraw_Button_CapPieceProc` | function | Button owner-draw callback selected for Skirmish `0x617`, `0x5AA`, `0x5C0`; formats `b%c%c_li/mi/ri%d.pcx`. |
| `0x00617250` | `OwnerDraw_ComboBox_Proc` | function | ComboBox owner-draw callback; handles `0x497`, `0x498`, `0x4DD`, dropdown creation, selected text, and swatches. |
| `0x006163A0` | `OwnerDraw_Checkbox_Proc` | function | Button style `0x03` callback; draws `cue_i.pcx` / `cce_i.pcx` checkbox icons. |
| `0x0061F210` | `OwnerDraw_PreloadPcxPool` | function | One-time owner-draw shell PCX preload list. |
| `0x006BA120` | `OwnerDraw_LoadDialogSystemPcx` | wrapper | Calls the owner-draw PCX loader with mode `1`, flag `1`; used for `dlgsysa.pcx`. |
| `0x006BA140` | `OwnerDraw_FindCachedPcxSurface` | function | Hash lookup by PCX name; returns converted cached surface pointer or `0`. |
| `0x006B9D00..0x006BA09F` | `OwnerDraw_LoadPcxTo16BitSurface` | assembly region | Opens/decodes PCX, converts embedded palette indexed pixels to 16-bit surface, inserts cache entry. |
| `0x006BA3E0` | `OwnerDraw_TileSurfaceRect` | function | Tiled middle-piece blit helper used by button/scrollbar/trackbar pieces. |
| `0x006BA580` | `OwnerDraw_TransparentBlitSurface` | function | Transparent keyed blit helper used by static flag images. |
| `0x0060F9A0` | `OwnerDraw_HookControlWindow` | function | Installs owner-draw wndproc, stores previous proc, dispatches callback by class/style, sends `0x497`. |
| `0x00622B50` | `ShellDialog_BaseWndProc` | function | Shared shell dialog proc; handles `WM_INITDIALOG`, owner-draw hook enumeration, shell painting, tooltips. |
| `0x006AE3F0` | `SkirmishDialog_Proc` | function | Dialog `0x102` proc; delegates base shell handling, initializes on `0x497`, handles paint and commands. |
| `0x006AE6E0` | `SkirmishDialog_InitControls` | function | Populates player/side/color/start/team controls and initializes sliders/check boxes. |
| `0x006ACEE0` | `SkirmishDialog_Command` | function | `WM_COMMAND` handler; routes color selection notification to `FUN_004E4C20`. |
| `0x004E43C0` | `SkirmishColorTable_InitNamesAndSwatches` | function | Loads nine color names, copies nine swatch values from `0x008316A8`, clears owners to `-1`. |
| `0x004E45A0` | `SkirmishColorCombo_PopulateAvailable` | function | Normal color combo population; sends `0x4DD`, `0x4DE`, `0x498`, item data, and selection. |
| `0x004E4770` | `SkirmishColorCombo_PopulateRestrictedGrey` | function | Restricted/grey color combo population path for local/closed conditions in modes `3`/`4`. |
| `0x004E4820` | `SkirmishColorCombos_RefreshAll` | function | Iterates eight color controls and chooses normal vs grey population. |
| `0x004E49A0` | `SkirmishColorCombo_SelectColorAndRefresh` | function | Programmatic color selection; clears old owner, selects requested item data, refreshes all combos. |
| `0x004E4C20` | `SkirmishColorCombo_OnSelectionChanged` | function | Command-time color change handler; updates global color ownership and refreshes all combos. |
| `0x004E4E20` | `SkirmishColorCombo_GetSelectedItemData` | function | Reads current or supplied row and returns item data through `0x150`; used by tooltip/help logic. |
| `0x008B4038` | `SkirmishColorTable` | data | Nine 12-byte rows: name pointer, swatch value, owning slot. |
| `0x008316A8` | `SkirmishColorSwatchDefaults` | data | Nine raw swatch color values copied into `SkirmishColorTable`. |
| `0x00618A13..0x00618A36` | `OwnerDraw_ComboBox_SetItemSwatchColor` | message case | Custom `0x498`; writes `state+0x110+index*4`. |
| `0x00618A3B..0x00618A74` | `OwnerDraw_ComboBox_SetSwatchGreyAndMaxRows` | message cases | Custom `0x4DD`, `0x4F1`, `0x4DE`; sets bytes `+0xCC`, `+0xCD`, and dword `+0xD0`. |
| `0x00618AEA..0x00618B61` | `OwnerDraw_ComboBox_InitState` | message case | Custom `0x497`; initializes current selection and fills 50 swatch slots with `-1`. |
| `0x00617A5D..0x00617B3F` | `OwnerDraw_ComboBox_DrawSelectedSwatch` | paint block | Reads selected swatch slot and fills the collapsed combo swatch rectangle. |
| `0x00783A90` | `ShellNumberPcx_GetDigitSurface` | function | Maps digit `0..9` to `numberN.pcx` cache lookup; returns `0` outside range. |
| `0x007845D1`, `0x00784663`, `0x007846E4` | `WolShell_DrawNumberPcxDigits_CallSites` | call sites | WOL `wonline.cpp` caller region using `ShellNumberPcx_GetDigitSurface`. |
| `0x0083FDB8` | `s_BTN_MINS_SHP_Unxrefed` | string | Static string `BTN-MINS.SHP`; no function xrefs found. |
| `0x0083FDC8` | `s_BTN_PLUS_SHP_Unxrefed` | string | Static string `BTN-PLUS.SHP`; no function xrefs found. |
| `0x00835E5C..0x00835E8C` | `s_bst_checkbox_state_pcx_preload_strings` | strings | `bst_uckg/chkg/uchk/chkd.pcx`; xrefs only from preload in this pass. |
| `0x00835D24..0x00835DD4` | `s_bud_button_disabled_pcx_preload_strings` | strings | `bud_*` button pieces; xrefs only from preload in this pass. |
| `0x00835F5C..0x00835FC8` | `s_number_digit_pcx_strings` | strings | `number9.pcx..number0.pcx`; used by `ShellNumberPcx_GetDigitSurface` and preload. |

## Sources

Ghidra functions / regions decompiled or assembly-checked:

- `OwnerDraw_Button_00612B70`
- `OwnerDraw_ComboBox_00617250`
- `OwnerDraw_Checkbox_006163A0`
- `FUN_0060F9A0`
- `FUN_0061F210`
- `FUN_00622B50`
- `FUN_006AE3F0`
- `FUN_006AE6E0`
- `FUN_006ACEE0`
- `FUN_004E43C0`
- `FUN_004E45A0`
- `FUN_004E4770`
- `FUN_004E4820`
- `FUN_004E49A0`
- `FUN_004E4C20`
- `FUN_004E4E20`
- `FUN_006BA120`
- `FUN_006BA140`
- loader region `0x006B9D00..0x006BA09F`
- `FUN_00783A90`
- WOL caller region `0x007845D1`, `0x00784663`, `0x007846E4`

Binary/resource evidence:

- PE `RT_DIALOG` resource `0x102`, language `0x409`.
- Static color table source at `0x008316A8`.
- Color table destination at `0x008B4038`.
- `BTN-MINS.SHP` string at `0x0083FDB8`.
- `BTN-PLUS.SHP` string at `0x0083FDC8`.
- `wonline.cpp` string at `0x0084A430`.

Prior reports referenced:

- `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
