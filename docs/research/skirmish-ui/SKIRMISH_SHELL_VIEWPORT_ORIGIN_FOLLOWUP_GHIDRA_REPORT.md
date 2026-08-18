# Skirmish Shell Viewport Origin Follow-up: Right-Panel Inset and Back Button Rect

Date: 2026-05-16

Parent report:
`C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`

Scope: targeted follow-up only. This resolves the two open questions from the
parent report:

1. whether right-anchored Skirmish controls `0x617`, `0x5AA`, and `0x468` receive
   a nonzero owner-draw metadata override at offset `+0xE0`;
2. the exact final rect for Back button `0x5C0`, including `SDBTNANM.SHP`
   dimensions and right-panel layout globals.

No Rust implementation was written or modified.

## 1. Overview

**Address(es):** `0x0060B1D0`, `0x0060B350`, `0x0060CF00`,
`0x0072E450`, `0x0072EB50`, `RightPanel__ComputeLayoutRects`

**Confidence:** High for 800x600 and 1024x768 formulas/rects. Medium-high for
640x480 because the formulas and assets are verified, but no live screenshot was
captured.

**Active in YR:** Yes. This path is reached by the offline Skirmish shell dialog
resource `0x102` in standard Yuri's Revenge shell code. The right-panel assets
are loaded from retail `NEUTRAL.MIX`/`NTRLMD.MIX` through active shell draw
initialization.

## 2. Verified Binary Findings

### 2.1 Child `+0xE0` remains zero for the targeted right-anchored controls

`FUN_0060B1D0` right-anchors selected child controls. It looks up the child HWND's
owner-draw metadata record and reads dword offset `+0xE0`:

```text
if child_record.+0xE0 != 0:
    inset = child_record.+0xE0
else:
    inset = (168 - child_width) / 2
```

The earlier parent report treated this as an unresolved override. This pass found
that, for the Skirmish controls in scope, the child record keeps the constructor
default zero.

Evidence:

- `FUN_00623340` initializes owner-draw metadata by zeroing `0x80` dwords
  (`0x200` bytes), then sets only a small set of defaults such as type/font/string
  fields. Offset `+0xE0` is zero after construction.
- `FUN_0060F760` creates or updates metadata records for HWNDs and writes fields
  such as dword index `8`/`9` from its `param_2`, but does not write `+0xE0`.
- `FUN_0060F9A0` registers/subclasses controls and, at the end of child setup,
  writes fields such as type (`+0x68`), setup parameter (`+0x04`), caption/string
  state (`+0x28`), and a message flag (`+0x2C`). It does not write `+0xE0`.
- The `FUN_0060CF00` writes to offset `+0xE0` are parent-dialog background asset
  setup, not child-control inset setup.

Important correction: offset `+0xE0` is not a single semantic field across every
use. For parent shell-dialog records, `FUN_0060CF00` stores a background/shell
surface pointer there. For right-anchored child records, `FUN_0060B1D0` treats
the same offset as an optional numeric inset override. The Skirmish child records
in this path are not passed to `FUN_0060CF00`, so they keep zero and use the
default inset formula.

### 2.2 `FUN_0060CF00` only receives the shell parent HWND in this path

`FUN_0060CF00` is called from common dialog initialization, not from the child
layout callback:

| Call site | Context |
|---|---|
| `0x006228B8` in `FUN_00622820` | `ECX = ESI`, where `ESI` is the shell dialog HWND |
| `0x00622F6B` in `FUN_00622B50` | `ECX = ESI`, after dialog id has been written with `FUN_0060D2C0` |
| `0x0062300F` in `FUN_00622B50` | `ECX = ESI`, alternate init branch, same parent-dialog context |

For dialog id `0x102`, assembly at `0x0060D294..0x0060D2AE` does:

```text
CALL FUN_0072D030
MOV  [ESI + 0x74], EAX        ; parent shell background handle/state
MOV  ECX, [DAT_00B0FB50]
MOV  [ESI + 0xE0], ECX        ; parent background asset pointer
MOV  EDX, [DAT_00B0FA18]
MOV  [ESI + 0xE4], EDX        ; parent alternate/background asset pointer
```

That explains the apparent `+0xE0` writes found near `FUN_0060CF00`: they affect
the parent dialog record. They do not override `0x617`, `0x5AA`, or `0x468`.

### 2.3 Right-panel SHP dimensions

`RightPanel__Draw` lazily initializes the right-panel shell assets:

1. `FUN_00534E50` opens `NTRLMD.MIX` and `NEUTRAL.MIX`.
2. `Sidebar_RightPanel_SHP_Loading` loads the SHPs through the string pointer
   table beginning at `0x00844CD4`.
3. `RightPanel__ComputeLayoutRects` reads the loaded SHP header dimensions.

The retail asset headers were read directly from the installed game archives.
The relevant files resolve from `ra2.mix -> neutral.mix`.

| SHP | Source | Width | Height | Frames | Used by |
|---|---|---:|---:|---:|---|
| `SDBTNANM.SHP` | `ra2.mix -> neutral.mix` | 156 | 42 | 17 | Back button width/height and animation overlay |
| `SDBTNBKGD.SHP` | `ra2.mix -> neutral.mix` | 168 | 42 | 1 | vertical right-panel tile height |
| `SDTP.SHP` | `ra2.mix -> neutral.mix` | 168 | 199 | 2 | right-panel top cap |
| `SDBTM.SHP` | `ra2.mix -> neutral.mix` | 168 | 65 | 1 | right-panel bottom cap |
| `SDMPBTN.SHP` | `ra2.mix -> neutral.mix` | 156 | 84 | 7 | minimap/scroll button rect |
| `SDWRNTMP.SHP` | `ra2.mix -> neutral.mix` | 168 | 177 | 6 | warning/template overlay |
| `MNSCRNL.SHP` | `ra2.mix -> neutral.mix` | 632 | 568 | 1 | large screen corner/background |
| `MNSCRNS.SHP` | `ra2.mix -> neutral.mix` | 472 | 448 | 1 | 640 screen corner/background |
| `LWSCRNL.SHP` | `ra2.mix -> neutral.mix` | 632 | 32 | 1 | large lower screen edge |
| `LWSCRNS.SHP` | `ra2.mix -> neutral.mix` | 472 | 32 | 1 | 640 lower screen edge |

The key Back-button dimensions are therefore `156x42`, not the dialog resource
placeholder size `162x37`.

### 2.4 Back button y is the last full right-panel tile, not the bottom cap

`FUN_0060B350` computes Back button `0x5C0` from right-panel layout globals:

```text
offset_x = max(0, (parent_width - 800) / 2)
x = parent_width - offset_x - 0x9C       ; 0x9C == 156
width = SDBTNANM.SHP.width              ; 156
height = SDBTNANM.SHP.height            ; 42
y = ((fc28.y - fc24.y) / fc24.h - 1) * fc24.h + fc24.y
```

Where:

- `fc24` is `DAT_00B0FC24`, the repeated `SDBTNBKGD.SHP` tile rect;
- `fc28` is `DAT_00B0FC28`, the bottom-cap/remainder rect;
- `fc24.h` is `SDBTNBKGD.SHP.height == 42`.

That `-1` is visible and load-bearing: Back is placed on the last full
`SDBTNBKGD` tile before the bottom cap, not at the top of the bottom cap.

## 3. Final Rects

### 3.1 Right-panel layout globals

For `RightPanel__ComputeLayoutRects(screen_w, screen_h)`:

- if `screen_w > 1023`: `left_margin = (screen_w - 800) / 2`,
  `effective_right = screen_w - left_margin`;
- otherwise: `left_margin = 0`, `effective_right = screen_w`;
- if `screen_h > 767`: `top_margin = (screen_h - 600) / 2`,
  `effective_height = screen_h - 2 * top_margin`;
- otherwise: `top_margin = 0`, `effective_height = screen_h`.

Using the verified retail SHP dimensions:

| Mode | `fc20` SDTP rect | `fc24` tile rect | tile count `DAT_00B0FA20` | `fc28` bottom rect |
|---|---:|---:|---:|---:|
| 800x600 | `(632,0,168,199)` | `(632,199,168,42)` | 9 | `(632,577,168,23)` |
| 1024x768 | `(744,84,168,199)` | `(744,283,168,42)` | 9 | `(744,661,168,23)` |
| 640x480 | `(472,0,168,199)` | `(472,199,168,42)` | 6 | `(472,451,168,29)` |

### 3.2 Final key control rects

These are final shell-client/backbuffer coordinates after dialog creation,
fullscreen host resize, and child layout callbacks.

| Control | 800x600 final | 1024x768 final | 640x480 formula result | Evidence |
|---|---:|---:|---:|---|
| `0x617` Start | `(635,242,162,37)` | `(747,326,162,37)` | `(475,242,162,37)` | `FUN_0060B1D0`, child `+0xE0 == 0`, default inset `(168-162)/2 == 3` |
| `0x5AA` Choose Map | `(635,286,162,37)` | `(747,370,162,37)` | `(475,286,162,37)` | `FUN_0060B1D0`, child `+0xE0 == 0`, default inset `(168-162)/2 == 3` |
| `0x468` map preview | `(644,37,144,112)` | `(756,121,144,112)` | `(484,37,144,112)` | `FUN_0060B1D0`, child `+0xE0 == 0`, default inset `(168-144)/2 == 12` |
| `0x5C0` Back | `(644,535,156,42)` | `(756,619,156,42)` | `(484,409,156,42)` | `FUN_0060B350`, `SDBTNANM.SHP=156x42`, `DAT_00B0FC24/28` formulas |

The 640x480 column remains a formula result rather than a live-render capture.
It is now stronger than the parent report because the asset dimensions and Back
button formula are resolved, but no screenshot/trace confirmed the final visual
composition at that resolution.

## 4. Inferences and Implementation-Relevant Behavior

The right-panel inset uncertainty from the parent report is resolved for the
requested controls. Implementers should treat `0x617`, `0x5AA`, and `0x468` as
using the default right-panel centering inset in standard Skirmish:

```text
right_panel_width = 168
inset = (right_panel_width - control_width) / 2
```

Back button `0x5C0` is not part of the dialog-template visual layout after the
shell helper runs. The resource rect `(638,562,162,37)` is only a placeholder.
The actual final visual/control rect comes from `SDBTNANM.SHP` and right-panel
tile layout:

```text
width  = 156
height = 42
x = screen_width - max(0, (screen_width - 800) / 2) - 156
y = fc24.y + (tile_count - 1) * 42
```

For 800x600, this makes Back sit at y `535`, one full 42-pixel tile above the
right-panel bottom-cap rect at y `577`.

## 5. Open Questions

1. 640x480 still needs live visual confirmation if that mode matters. The formula
   path is now fully resolved, but the final player-visible shell composition in
   640x480 was not captured.

2. The report does not attempt to globally rename owner-draw metadata offset
   `+0xE0`, because the binary uses it differently depending on whether the record
   belongs to a parent shell dialog or a child control. A broader metadata layout
   investigation should name this field only with context-specific aliases.

## Suggested Labels

| Address / symbol | Suggested label | Evidence / purpose |
|---|---|---|
| `0x0060B1D0` | `ShellDialog_RightAnchorChild_DefaultInset` | Uses child record `+0xE0` override when nonzero, otherwise `(168 - width) / 2`. |
| `0x0060B350` | `ShellDialog_BackButtonTileAnchor` | Places Back button using `SDBTNANM.SHP` dimensions and `DAT_00B0FC24/28`. |
| `0x0060CF00` | `ShellDialog_SetParentBackgroundAssets` | Writes parent record `+0x74`, `+0xE0`, `+0xE4` background asset fields. |
| `0x00623340` | `OwnerDrawMetadata_InitZeroed` | Zeroes `0x80` dwords and sets default owner-draw metadata fields. |
| `0x0060F760` | `OwnerDrawMetadata_EnsureRecordAndSetPhase` | Creates/updates HWND metadata records without writing child `+0xE0`. |
| `0x0060F9A0` | `OwnerDraw_SubclassAndPopulateControlMetadata` | Subclasses controls, sets type/string/message fields, leaves child `+0xE0` zero. |
| `0x0072E450` | `RightPanel_DrawAndLazyInit` | Opens/loads right-panel resources, computes layout once, draws panel pieces. |
| `0x00534E50` | `Shell_LoadNeutralMixArchives` | Opens `NTRLMD.MIX` and `NEUTRAL.MIX` before right-panel SHP loading. |
| `0x0072EB50` | `Sidebar_RightPanel_LoadSHPs` | Loads `SDBTNANM`, `SDTP`, `SDBTNBKGD`, `SDBTM`, etc. through string table. |
| `RightPanel__ComputeLayoutRects` | `RightPanel_ComputeRectsFromSHPDimensions` | Computes `DAT_00B0FC20/24/28` and tile count from screen and SHP dimensions. |
| `0x00844CD4` | `RightPanelSHPNameTable` | Pointer table beginning with `SDBTNANM.SHP`. |
| `0x00B0FAC4` | `g_SDBTNANM_SHP` | Loaded `SDBTNANM.SHP`; Back button dimensions are read from `+2/+4`. |
| `0x00B0FA74` | `g_SDBTNBKGD_SHP` | Loaded `SDBTNBKGD.SHP`; tile height is 42. |
| `0x00B0FAF8` | `g_SDTP_SHP` | Loaded `SDTP.SHP`; top cap is 168x199. |
| `0x00B0FA38` | `g_SDBTM_SHP` | Loaded `SDBTM.SHP`; bottom cap is 168x65. |
| `0x00B0FC24` | `g_RightPanelTileRect` | `{x,y,w,h}` for repeated `SDBTNBKGD.SHP` tiles. |
| `0x00B0FC28` | `g_RightPanelBottomRect` | `{x,y,w,h}` for bottom remainder/cap; used by Back y formula. |
| `0x00B0FA20` | `g_RightPanelTileCount` | Integer tile count `(effective_height - SDTP.h) / SDBTNBKGD.h`. |

## Sources

- Ghidra decompilation/assembly:
  - `FUN_0060B1D0`
  - `FUN_0060B350`
  - `FUN_0060CF00`
  - `FUN_00623340`
  - `FUN_0060F760`
  - `FUN_0060F9A0`
  - `FUN_00622B50`
  - `FUN_00622820`
  - `FUN_00534E50`
  - `Sidebar_RightPanel_SHP_Loading`
  - `RightPanel__ComputeLayoutRects`
  - `RightPanel__Draw`
- Retail asset headers read from:
  - `C:/Users/enok/Documents/Command and Conquer Red Alert II/ra2.mix`
  - nested `neutral.mix`
- Parent report:
  - `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`
- Existing sidebar reports used only for cross-checking names/globals:
  - `SIDEBAR_CONSTRUCTION_GHIDRA_REPORT.md`
  - `SIDEBAR_RADAR_POSITIONING.md`
