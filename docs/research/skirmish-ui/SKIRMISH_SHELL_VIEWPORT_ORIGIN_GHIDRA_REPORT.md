# Skirmish Shell Viewport Origin and Resolution Behavior

Date: 2026-05-16

Scope: follow-up to `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`.
This report only covers the remaining host viewport/origin/resolution gap for the
offline Skirmish shell dialog. It intentionally does not repeat prior owner-draw,
asset-mapping, or resource-layout findings except where needed to anchor coordinates.

## Verified Binary Findings

### Dialog 0x102 creation and host

`Main_Game` calls `FUN_006AE2C0` at `0x0052E168`. In `FUN_006AE2C0`,
the offline Skirmish dialog is selected directly:

- `0x006AE31C`: loads dialog proc `0x006AE3F0`.
- `0x006AE321`: loads resource/dialog id `0x102`.
- `0x006AE328`: calls `FUN_00622650`.

`FUN_00622650` creates the window from a Win32 dialog resource:

- It calls `FUN_004A3B40(resource_id, 5)`; type `5` is `RT_DIALOG`.
- `FUN_004A3B40` calls `FindResourceA(DAT_0089F974, id, 5)`,
  `LoadResource`, and `LockResource`.
- `FUN_00622650` then calls
  `CreateDialogIndirectParamA(DAT_00B732F0, template, g_hWnd, proc, &local_8)`.

This is a modeless dialog, not a `DialogBoxParam` modal dialog. The parent passed
to Win32 is the main shell/game window `g_hWnd`; no intermediate child shell
control was found in this creation path. Resource `0x102` has style `0x40000040`,
which includes `WS_CHILD`, so the Skirmish dialog is a child window hosted directly
under `g_hWnd`.

After creation, `FUN_006AE2C0` stores the dialog HWND in `DAT_00B0B59C`, sets a
window long at offset `8`, calls `FUN_00622800` (`ShowWindow(hwnd, 1)` and
`SetForegroundWindow(hwnd)`), pumps `FUN_00623120`, then destroys the child dialog
with `FUN_00622720`.

### Initialization and fullscreen shell host path

The Skirmish dialog proc `FUN_006AE3F0` first delegates messages to
`FUN_00622B50`. On `WM_INITDIALOG`, `FUN_00622B50` performs the common shell
dialog setup:

- stores the current dialog HWND in `DAT_00AC48A8`;
- enumerates child windows through owner-draw and shell metadata callbacks;
- registers shell background and owner-draw state;
- calls `FUN_0060C540(hwnd)`;
- if that returns true, initializes a `{640, 480}` local pair from
  `DAT_007F5BE0` and `DAT_007F5BEC`, then calls `FUN_0060C4A0(hwnd, &pair)`.

`FUN_0060C540` returns true for a shell-dialog id set that includes dialog `0x102`.
Therefore offline Skirmish enters the fullscreen shell-host path.

`FUN_0060C4A0` is the key host viewport function:

- reads `g_ScreenWidth` from `0x008A00A4`;
- reads `g_ScreenHeight` from `0x008A00A8`;
- calls `MoveWindow(hwnd, 0, 0, g_ScreenWidth, g_ScreenHeight, 0)`;
- stores `DAT_00AC48A8 = hwnd`;
- enumerates child windows through the `LAB_0060C0C0` layout callback, passing
  the `{640, 480}` pair.

So resource `0x102` starts as a dialog template but is then resized to cover the
current shell client/backbuffer size. The dialog HWND origin is `(0, 0)` in the
main shell client. Its size is `g_ScreenWidth x g_ScreenHeight`.

### Dialog resource coordinate baseline

The resource is `DIALOGEX 0, 0, 533, 369`, 72 controls, MS Sans Serif 8, style
`0x40000040`. The observed Win32 dialog base units are `baseX = 6`, `baseY = 13`.
Resource DLU coordinates convert by the normal Win32 formula:

- `x_px = MulDiv(x_dlu, 6, 4)`;
- `w_px = MulDiv(w_dlu, 6, 4)`;
- `y_px = MulDiv(y_dlu, 13, 8)`;
- `h_px = MulDiv(h_dlu, 13, 8)`.

The template therefore maps to an approximately 800x600 shell layout before the
fullscreen host resize and selective child movement.

### Child post-creation transforms

No uniform child-control scale transform was found. Instead, `LAB_0060C0C0`
selectively moves specific controls after the parent dialog has been resized.

For dialog `0x102`, `FUN_00608CD0(parent, child)` selects controls for the
right-panel anchor helper `FUN_0060B1D0`. The verified Skirmish ids in this set
include:

- `0x468` map preview static;
- `0x5AA` Choose Map button;
- `0x5A8`;
- `0x617` Start button;
- `0x6EC`;
- `0x694` Skirmish title.

`FUN_00609730(parent, child)` selects the Back button `0x5C0` for the bottom
anchor helper `FUN_0060B350` when the parent dialog is `0x102`.

`FUN_0060B950` applies a few one-pixel resource corrections for dialog `0x102`,
but not to the key controls requested here:

- `0x50C`: y is moved by `-1`;
- `0x54E`, `0x693`, `0x696`, `0x69A`: x is moved by `-1`;
- `0x6A0`: x is moved by `+1` and width by `+1`.

The color combos `0x6A2`, `0x522..0x528` and flag statics `0x6DA..0x6E1` were not
found in the right-anchor, bottom-anchor, or one-pixel fixup allowlists examined
for dialog `0x102`. They retain their resource-derived pixel positions after the
parent dialog is resized.

### Right-panel anchor helper

`FUN_0060B1D0` anchors selected right-panel children against an 800-wide shell
content region inside the current screen. In the normal branch (`FUN_0069BBE0()`
returns zero), the helper computes:

- `offset_x = max(0, (parent_width - 800) / 2)`;
- `offset_y = max(0, (parent_height - 600) / 2)`;
- default right-panel inset `= (168 - child_width) / 2`, using
  `DAT_007F5BF8 == 168`, unless the owner-draw metadata field at `+0xE0`
  supplies a nonzero inset;
- final x is `parent_width - offset_x - child_width - inset`;
- final y is `original_child_y + offset_y`.

The `{640, 480}` pair passed to the callback does not subtract a vertical offset
for this dialog path because `(480 - 600) / 2` is negative and clamps to zero.

### Back button bottom anchor helper

`FUN_0060B350` handles Back button `0x5C0`. It does not keep the resource
rectangle. In the normal branch it computes:

- `offset_x = max(0, (parent_width - 800) / 2)`;
- `x = parent_width - offset_x - 0x9C`;
- width from `*(short *)(g_SDBTNANM_SHP + 2)`;
- height from `*(short *)(g_SDBTNANM_SHP + 4)`;
- y from the computed shell/right-panel layout rects `DAT_00B0FC24` and
  `DAT_00B0FC28`.

The x-origin is therefore verified. The exact final y and dimensions depend on the
loaded `SDBTNANM.SHP` dimensions and the computed layout globals and were not fully
resolved in this pass.

### Background/surface origin

`FUN_00775690(hwnd, rect)` converts any HWND window rectangle into main-shell
client/backbuffer coordinates:

- calls `GetWindowRect(hwnd, rect)`;
- obtains the main `g_hWnd` client origin in screen coordinates via
  `GetClientRect(g_hWnd)` and `ClientToScreen(g_hWnd)`;
- subtracts that main-client screen origin from the HWND screen rect.

`WM_PAINT_Handler` uses this conversion for shell dialogs before drawing or
caching backgrounds. For dialog `0x102`, the parent dialog origin converted this
way is `(0, 0)` after `FUN_0060C4A0`, and child controls convert to their final
shell/backbuffer rectangles.

`DAT_00887310` is the main display/backbuffer surface used by the shell paint code.
`WM_PAINT_Handler` creates or reuses a per-dialog cached `BSurface` sized from the
dialog client, draws shell background material into that surface, then blits it to
`DAT_00887310` at the dialog's converted shell-client origin.

`DrawStartPositions` confirms the child-control coordinate relationship for the
map preview:

- it obtains `GetDlgItem(hwnd, 0x468)`;
- calls `FUN_00775690` on the preview HWND;
- draws preview/start-position material to `DAT_00887310` using the resulting
  preview rectangle.

Therefore map preview overlay painting is in final shell/backbuffer coordinates
derived from the preview child HWND, not raw dialog-resource coordinates.

Owner-draw button/combo/statics callbacks paint their own child client areas.
Their cached parent backgrounds are aligned by HWND-to-shell coordinate conversion;
the draw calls inside each child remain child-client relative.

### Background assets and resolution branches

`FUN_0072CF40`, called before the Skirmish dialog is shown, loads the Skirmish
shell background resources. It contains a direct `g_ScreenWidth == 800` branch
before loading one of the `MnScrnLCoopGameSetup` resources into `DAT_00B0FA18`.
It also loads palette/background state into the `DAT_00B0FCDC` / `DAT_00B0FCE0`
region used later by shell paint setup.

`RightPanel__ComputeLayoutRects(screen_w, screen_h)` contains explicit larger-screen
centering behavior:

- if `screen_w > 1023`, it computes horizontal centering from `(screen_w - 800) / 2`;
- if `screen_h > 767`, it computes vertical centering from `(screen_h - 600) / 2`;
- it has 640/480 branches for some sidebar/shell pieces;
- it writes the right-panel layout globals later used by paint and Back-button
  placement.

`RightPanel__Draw` and `Background_Overlay` also clip/offset around an 800x600
content region when the screen is larger than 800x600.

### Options/INI source for resolution

No Skirmish-specific layout scale/origin key was found in `rules.ini`,
`rulesmd.ini`, `art.ini`, or `artmd.ini`.

The relevant binary string references are general video options:

- `[Video] ScreenWidth`;
- `[Video] ScreenHeight`;
- `AllowHiResModes`.

These strings are referenced by `WinMain`, `OptionsClass__ReadFromINI`, and
`OptionsClass__WriteToINI`. The Skirmish shell layout uses the resulting global
screen dimensions, not a Skirmish-specific INI layout setting.

## Key Control Coordinates

The "resource px" column is the Win32 dialog-template result before selective shell
movement. The "final" columns are shell-client/backbuffer coordinates after
`FUN_0060C4A0` and the child-layout callback.

### Buttons and map preview

For right-anchored controls below, the final x assumes the owner-draw metadata inset
field at `+0xE0` remains zero, causing the default `(168 - width) / 2` inset to be
used. No write to a nonzero inset for these specific controls was found in the
investigated path, but this remains an inference rather than a direct live capture.

| Control | Resource DLU | Resource px | Helper | Final formula | 800x600 final | 1024x768 final | 640x480 formula result |
|---|---:|---:|---|---|---:|---:|---:|
| `0x617` Start | `(425,149,108,23)` | `(638,242,162,37)` | `FUN_0060B1D0` | `x=W-ox-162-3`, `y=242+oy` | `(635,242,162,37)` | `(747,326,162,37)` | `(475,242,162,37)` |
| `0x5AA` Choose Map | `(425,176,108,23)` | `(638,286,162,37)` | `FUN_0060B1D0` | `x=W-ox-162-3`, `y=286+oy` | `(635,286,162,37)` | `(747,370,162,37)` | `(475,286,162,37)` |
| `0x5C0` Back | `(425,346,108,23)` | `(638,562,162,37)` | `FUN_0060B350` | `x=W-ox-156`; y/size from right-panel layout and `SDBTNANM.SHP` | x `644`; y/size unresolved | x `756`; y/size unresolved | x `484`; y/size unresolved |
| `0x468` map preview | `(429,23,96,69)` | `(644,37,144,112)` | `FUN_0060B1D0` | `x=W-ox-144-12`, `y=37+oy` | `(644,37,144,112)` | `(756,121,144,112)` | `(484,37,144,112)` |

Where:

- `W` and `H` are `g_ScreenWidth` and `g_ScreenHeight`;
- `ox = max(0, (W - 800) / 2)`;
- `oy = max(0, (H - 600) / 2)`.

The 640x480 column is the direct result of the verified formulas. It is not a
separate live screenshot confirmation that the final retail Skirmish shell is
usable or visually correct at 640x480.

### Color combos

No post-creation move/scale transform was found for these controls in the dialog
`0x102` path. Because the parent dialog is moved to shell origin `(0, 0)`, final
shell-client coordinates match the resource pixel coordinates.

| Control | Resource DLU | Resource px / final shell-client |
|---|---:|---:|
| `0x6A2` | `(282,36,29,73)` | `(423,59,44,119)` |
| `0x522` | `(282,52,29,73)` | `(423,85,44,119)` |
| `0x523` | `(282,68,29,73)` | `(423,111,44,119)` |
| `0x524` | `(282,84,29,73)` | `(423,137,44,119)` |
| `0x525` | `(282,100,29,73)` | `(423,163,44,119)` |
| `0x526` | `(282,116,29,73)` | `(423,189,44,119)` |
| `0x527` | `(282,132,29,73)` | `(423,215,44,119)` |
| `0x528` | `(282,148,29,73)` | `(423,241,44,119)` |

### Flag statics

No post-creation move/scale transform was found for these controls in the dialog
`0x102` path. Final shell-client coordinates match the resource pixel coordinates.

| Control | Resource DLU | Resource px / final shell-client |
|---|---:|---:|
| `0x6DA` | `(150,36,32,12)` | `(225,59,48,20)` |
| `0x6DB` | `(150,52,32,12)` | `(225,85,48,20)` |
| `0x6DC` | `(150,68,32,12)` | `(225,111,48,20)` |
| `0x6DD` | `(150,84,32,12)` | `(225,137,48,20)` |
| `0x6DE` | `(150,100,32,12)` | `(225,163,48,20)` |
| `0x6DF` | `(150,116,32,12)` | `(225,189,48,20)` |
| `0x6E0` | `(150,132,32,12)` | `(225,215,48,20)` |
| `0x6E1` | `(150,148,32,12)` | `(225,241,48,20)` |

## Inferred Coordinate and Viewport Behavior

The Skirmish shell should be modeled as an 800x600 logical shell composition hosted
inside a child dialog whose HWND is resized to the actual shell client size.

At 800x600, the parent dialog and shell/backbuffer origins coincide at `(0, 0)`.
Most controls keep their Win32 resource positions. The right-panel controls are
still passed through the anchor helper; for the map preview this reproduces the
resource x exactly, while Start and Choose Map shift from resource x `638` to
inferred final x `635` when the default 168-wide right-panel inset is used.

At 1024x768 and larger modes, the binary centers an 800x600 shell content region
inside the actual screen for right-panel/background layout. Right-panel controls
selected by `FUN_00608CD0` receive `+112,+84` at 1024x768 relative to their
800x600 logical region, modulo their right-panel inset. Unselected controls such
as color combos and flag statics are not moved by the child-layout callback found
in this path, so they remain at their resource pixel coordinates in the resized
dialog.

At 640x480, the verified formulas clamp the 800x600 centering offsets to zero.
Right-anchored controls are pulled left by the smaller parent width, while
unselected controls keep their original 800-layout coordinates. The codebase also
contains 640/480 branches in shell/right-panel computation, but this pass did not
produce a direct live-render confirmation of final 640x480 Skirmish appearance.

Background rendering uses shell/backbuffer coordinates rather than pure dialog DLU
coordinates. Cached parent backgrounds include the dialog/control HWND offset by
calling `FUN_00775690`; for the fullscreen Skirmish parent this offset is normally
zero, while child controls receive their converted final HWND rectangles.

No INI setting was found that scales or re-origins the Skirmish layout directly.
The relevant input is the global video mode (`ScreenWidth`, `ScreenHeight`,
`AllowHiResModes`), which feeds `g_ScreenWidth` and `g_ScreenHeight`.

## Unresolved / Open Questions

1. The exact final y and size for Back button `0x5C0` were not fully recovered.
   `FUN_0060B350` uses `g_SDBTNANM_SHP` dimensions and right-panel layout globals;
   this pass verified the x-origin formula but not the final loaded asset dimensions
   and computed y.

2. The right-panel helper can use a nonzero owner-draw metadata inset at `+0xE0`.
   No write to that field for `0x617`, `0x5AA`, or `0x468` was found in the
   investigated path, so the final positions above use the verified default inset
   formula. A targeted watch of that metadata field during initialization would
   turn those inferred final x values into fully verified live values.

3. 640x480 behavior was verified as code paths and formulas, not as a live captured
   rendered Skirmish shell. The formulas indicate mixed behavior: right-anchored
   controls adapt to the smaller width, while unselected dialog controls remain at
   their 800-layout positions.

4. The exact `WM_PAINT_Handler` function start address should be labeled once the
   local Ghidra function boundary is confirmed. Its behavior was analyzed through
   the decompiled named function and call sites, but this report does not assign a
   new address label for it.

## Suggested Labels

| Address / symbol | Suggested label | Evidence / purpose |
|---|---|---|
| `0x006AE2C0` | `Skirmish_ShowOfflineDialog` | Selects dialog id `0x102`, proc `0x006AE3F0`, creates/pumps/destroys offline Skirmish dialog. |
| `0x006AE3F0` | `Skirmish_DialogProc` | Dialog proc passed for resource `0x102`; delegates common shell handling first. |
| `0x00622650` | `Shell_CreateModelessDialogFromResource` | Loads dialog template and calls `CreateDialogIndirectParamA(..., g_hWnd, ...)`. |
| `0x004A3B40` | `WinResource_LockDialogTemplate` | `FindResourceA`/`LoadResource`/`LockResource` helper for RT_DIALOG. |
| `0x00622800` | `Shell_ShowDialogWindow` | Calls `ShowWindow(hwnd, 1)` and `SetForegroundWindow(hwnd)`. |
| `0x00622720` | `Shell_DestroyDialogWindow` | Destroys the modeless shell dialog and clears dialog-stack state. |
| `0x00622B50` | `Shell_CommonDialogProc` | Common WM_INITDIALOG/paint/owner-draw dispatcher for shell dialogs. |
| `0x0060C540` | `ShellDialog_UsesFullscreenShellHost` | Returns true for dialog ids including `0x102`. |
| `0x0060C4A0` | `ShellDialog_ResizeToScreenAndRepositionChildren` | Resizes parent dialog to `g_ScreenWidth x g_ScreenHeight` and enumerates children. |
| `LAB_0060C0C0` | `ShellDialog_PostCreateChildLayoutProc` | Child enumeration callback that dispatches right/bottom/fixup movement. |
| `0x00608CD0` | `ShellDialog_ShouldRightAnchorChild` | Identifies right-panel children for `FUN_0060B1D0`; includes `0x468`, `0x5AA`, `0x617` for `0x102`. |
| `0x00609730` | `ShellDialog_ShouldBottomAnchorBackButton` | Identifies Back button `0x5C0` for bottom/right anchoring. |
| `0x0060B1D0` | `ShellDialog_RightAnchorChildInShell800` | Computes centered 800x600 shell offsets and right-panel child x/y. |
| `0x0060B350` | `ShellDialog_BottomBackButtonAnchor` | Computes Back button x/y/size from screen width, SHP dimensions, and layout globals. |
| `0x0060B950` | `ShellDialog_OnePixelControlFixups` | Applies per-dialog one-pixel resource corrections. |
| `0x0060CF00` | `ShellDialog_SelectBackgroundAssetsByDialogId` | Selects shell background mode/assets for dialog ids including `0x102`. |
| `0x0072CF40` | `SkirmishShell_LoadBackgroundResources` | Loads Skirmish shell background/palette resources; contains `g_ScreenWidth == 800` branch. |
| `0x0072CF90` | `SkirmishShell_FreeBackgroundResources` | Paired cleanup for the Skirmish shell background resources. |
| `0x00775690` | `HWND_RectToShellClientCoords` | Converts HWND screen rects to main shell client/backbuffer coordinates. |
| `0x007F5BE0` | `ShellResolutionConstants_640_800_1024` | Constant table: 640, 800, 1024, 480, 600, 768, 168, 32. |
| `0x007F5BF8` | `ShellRightPanelWidth_168` | Right-panel width used by default inset calculation. |
| `0x008A00A4` | `g_ScreenWidth` | Screen width global used by shell host and layout code. |
| `0x008A00A8` | `g_ScreenHeight` | Screen height global used by shell host and layout code. |
| `0x00887310` | `g_MainBackBufferSurface` | Main shell/backbuffer surface used by paint and preview drawing. |
| `0x00AC48A8` | `g_CurrentShellDialogHwnd` | Current shell dialog HWND during init/layout. |
| `0x00B0B59C` | `g_SkirmishDialogHwnd` | Stores HWND returned by offline Skirmish dialog creation. |
| `0x00B0FA18` | `g_SkirmishShellBgResource800` | Resource loaded in the `g_ScreenWidth == 800` branch of `FUN_0072CF40`. |
| `0x00B0FCDC` / `0x00B0FCE0` | `g_SkirmishShellPaletteOrBgState` | Background/palette state loaded by `FUN_0072CF40` and used by shell paint setup. |
| `0x00B0FC24` / `0x00B0FC28` | `g_ShellRightPanelLayoutRects` | Layout globals consumed by Back-button placement and shell panel drawing. |

## Sources Checked

- `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
- `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
- `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
- `SIDEBAR_RADAR_POSITIONING.md`
- `SIDEBAR_SYSTEM_GHIDRA_REPORT.md`
- Ghidra decompilation/assembly for the functions and regions listed in the
  Suggested Labels table.
- Repo INI scan for `rules.ini`, `rulesmd.ini`, `art.ini`, `artmd.ini` shell
  layout/origin keys.
