# Skirmish Shell Background, Text, and Preview - Ghidra Research Report

## Superseded Asset-Family Correction - 2026-05-24

For standard Skirmish setup sidebar Start Game `0x617`, Choose Map `0x5AA`, and
Back `0x5C0`, older button rows in this report that name the generic PCX path
are superseded. The corrected classifier recheck proves these three right-panel
buttons are owner-draw type `1` and draw `SDBTNANM.SHP` frames `2`/`4`. Use
`SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md` for the
current button asset-family contract.

**Date:** 2026-05-17  
**Program:** `gamemd.exe` loaded in live Ghidra MCP from `<ra2-install>/gamemd.exe`  
**Primary addresses:** `0x0072CF40`, `0x00622B50`, `0x00621E90`, `0x0060CF00`, `0x00612B70`, `0x00621040`, `0x00640710`, `0x00640A40`  
**Overall confidence:** High for standard offline Skirmish dialog `0x102` at 640/800-width background selection, button PCX path, text wrapper behavior, and preview marker ordering. Medium for `>800` background output because lower-level null-SHP draw behavior was not resolved.  
**Active in YR:** Yes for the offline Skirmish caller/dialog path except `0x00640A40`, which is active YR preview/assigned-player marker code but not reached by the confirmed offline Skirmish dialog first-paint path.

## 1. Scope and Method

This pass was requested after the earlier verified-assets policy treated Skirmish backgrounds as unknown. A live Ghidra MCP instance was available and connected to the `gamemd.exe` program. The debugger sidecar at `127.0.0.1:8099` was not running, so this is live Ghidra decompilation/disassembly and xref analysis, not hardware watchpoint/runtime breakpoint capture.

Prior reports read first:

- `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`
- `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
- `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
- `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`
- Later discovered prior live report: `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`

This report does not modify Rust code and does not touch `src/` or `sim/`.

## 2. Short Answers

| Question | Verified answer | Confidence | Active in offline YR Skirmish? |
|---|---|---:|---|
| Which background assets are actively used by dialog `0x102`? | `0x0072CF40` loads `MnScrnLCoopGameSetup.PAL` always once, and `MnScrnLCoopGameSetup.shp` only when `g_ScreenWidth == 800`. `0x0060CF00` writes `MNSCRNL.SHP` pointer `DAT_00B0FB50` into parent state `+0xE0` and writes `MnScrnLCoopGameSetup.shp` pointer `DAT_00B0FA18` into parent state `+0xE4`. `Background_Overlay @ 0x0072E730` chooses `+0xE0` when width is `640`, otherwise `+0xE4`. | High for 640/800; medium for `>800` | Yes for 640/800 branches |
| Palette path for parent/background SHPs? | `MnScrnLCoopGameSetup.PAL` string `0x00844F8C`, pointer table `0x00844D70`, loaded at `0x0072CF6A..0x0072CF7A` through `0x0072ADE0` into raw palette `DAT_00B0FCDC` and convert object `DAT_00B0FCE0`. `0x0060CF00` writes `FUN_0072D030()` result to parent `+0x74`. | High | Yes |
| Palette path for button/flag PCXs? | Owner-draw PCX preload/loader converts embedded PCX palette data to 16-bit surfaces; no external `DIALOG.PAL`, `SHELL.PAL`, `MAINBTTN.PAL`, or `SIDEBAR.PAL` path was found in the PCX owner-draw cache path. | High from prior + current preload confirmation | Yes |
| Palette path for right-panel SHPs? | `RightPanel__Draw @ 0x0072E450` loads right-panel SHPs and palette globals via sidebar/right-panel initialization. Exact independent palette identity for every right-panel SHP remains weaker than the parent-background palette proof. | Medium | Yes for SHP use; palette identity partially open |
| Is `MNSCRNL.SHP` active? | Yes. String `0x00845144` -> pointer table `0x00844CE4`; table read in right-panel/general shell loading; value `DAT_00B0FB50` is written into parent `+0xE0` for dialog `0x102`. `Background_Overlay` draws it when `g_ScreenWidth == 640`. | High | Yes, width 640 branch |
| Is `MNSCRNS.SHP` active for offline Skirmish? | It is in the broader shell table (`0x00845150`, `0x00844CE0`) but `0x0060CF00` does not select it for dialog `0x102` in the verified path. | High for non-selection by `0x0060CF00` | No direct offline `0x102` evidence |
| Is `MnScrnLCustomizeBattle.shp` active for offline Skirmish? | String exists (`0x00844FE0`, pointer table `0x00844D64`), but this pass found no dialog `0x102` selection; the direct Skirmish setup loader uses `MnScrnLCoopGameSetup.*` instead. | High | No evidence |
| Are `dbak6440.pcx`, `dlgsysa.pcx`, `dlgsysi.pcx` reached? | They are loaded/preloaded in generic owner-draw/shell paths: `dbak6440.pcx` is owner-draw preload and fallback in `WM_PAINT_Handler`; `dlgsysi.pcx` and `dlgsysa.pcx` are owner-draw preload assets. For parent mode `+0xB0 == 1` used by dialog `0x102`, `WM_PAINT_Handler` uses `RightPanel__Draw` + `Background_Overlay`, not the `dbak6440.pcx` fallback. | High | Generic shell/preload only, not confirmed visible in standard offline `0x102` first paint |
| Exact draw order? | Common parent paint first: right panel then parent background overlay into cached parent surface, blit to `DAT_00887310`; child owner-draw controls paint their own surfaces; Skirmish `WM_PAINT` then calls `0x00640710`, which blits preview surface, draws `STARTBUT.SHP`, then numeric labels. `mmpb.shp` is not in that offline dialog order. | High | Yes |
| Is `mmpb.shp` active? | Yes elsewhere: `0x00640A40` loads `mmpb.shp` and draws assigned-player/house markers. Callers show it is reached from `0x00552D60`/map-preview context, not from `0x006AE3F0` or `0x00640710`. | High | Active YR elsewhere; not standard offline `0x102` first paint |
| Exact owner-draw button path? | Start/Choose/Back are `162x37` client buttons, select `bue_*30.pcx` unpressed and `bde_*30.pcx` pressed, direct-blit left/right caps, tile middle with `0x006BA3E0`, pressed adds `+2` to text/content y, disabled forces unpressed art and alpha blends `0x80`; no `bud_*` use. | High | Yes |

## 3. Verified Binary Findings

### 3.1 Offline Skirmish caller loads resources before dialog `0x102`

`FUN_006AE2C0` is the offline Skirmish setup launcher. It calls `FUN_0072CF40()` before `FUN_00622650(0)` creates the dialog. The same function stores the HWND in `DAT_00B0B59C`, pumps until local result is Start `0x617` or Back `0x5C0`, destroys the dialog, frees preview state, then calls `FUN_0072CF90()`.

Player-visible implication: background resources are not speculative preload baggage after the screen. The offline Skirmish path actively prepares them before showing dialog `0x102`.

Evidence:

- `0x006AE2C0` decompile: `FUN_0072cf40(); hWnd = FUN_00622650(0); ... FUN_0072cf90();`
- Dialog proc in `0x006AE3F0` handles `WM_PAINT` and delegates to common shell first.

### 3.2 `0x0072CF40` Skirmish background/palette loader

Disassembly of `0x0072CF40`:

```text
0x0072CF40  MOV AL,[0x00B0FCD9]
0x0072CF45  TEST AL,AL
0x0072CF47  JNZ 0x0072CF86
0x0072CF49  CMP dword ptr [0x008A00A4],0x320   ; g_ScreenWidth == 800
0x0072CF53  JNZ 0x0072CF6A
0x0072CF55  MOV ECX,dword ptr [0x00844D6C]     ; -> "MnScrnLCoopGameSetup.shp"
0x0072CF5B  MOV EDX,0x00B0FCD8                 ; ownership byte
0x0072CF60  CALL 0x004A38D0
0x0072CF65  MOV [0x00B0FA18],EAX
0x0072CF6A  MOV ECX,dword ptr [0x00844D70]     ; -> "MnScrnLCoopGameSetup.PAL"
0x0072CF70  PUSH 0x00B0FCE0
0x0072CF75  MOV EDX,0x00B0FCDC
0x0072CF7A  CALL 0x0072ADE0
0x0072CF7F  MOV byte ptr [0x00B0FCD9],1
```

Tiny details:

- Guard byte `DAT_00B0FCD9` prevents repeat loads.
- Width comparison is exact equality to `800` (`0x320`), not `>= 800`.
- `DAT_00B0FA18` remains whatever/null for non-800 widths unless another path writes it.
- Palette load runs even when width is not 800.
- The paired cleanup `0x0072CF90` frees `DAT_00B0FA18` only when ownership byte `DAT_00B0FCD8` is set, frees `DAT_00B0FCDC`, destructs `DAT_00B0FCE0`, and clears `DAT_00B0FCD9`.

String/xref evidence:

| Data | Xref chain |
|---|---|
| `MnScrnLCoopGameSetup.shp` at `0x00844FA8` | data pointer `0x00844D6C`, read at `0x0072CF55` |
| `MnScrnLCoopGameSetup.PAL` at `0x00844F8C` | data pointer `0x00844D70`, read at `0x0072CF6A` |

### 3.3 `0x0072ADE0` palette conversion path

`0x0072ADE0` opens the PAL file, allocates `0x300` bytes, iterates `0x100` RGB triplets, shifts each channel left by `2`, stores the raw palette at `DAT_00B0FCDC`, then constructs a `ConvertClass` against `DAT_00887310` and stores it at `DAT_00B0FCE0`.

Tiny details:

- The PAL path uses 256 entries exactly.
- Channel expansion is `<< 2`.
- The convert object is screen-surface dependent (`DAT_00887310`), so this is not just an asset-file decode; it is tied to current display format.

### 3.4 `0x0060CF00` assigns parent background fields for dialog `0x102`

For dialog id `0x102`, disassembly at `0x0060D294..0x0060D2AE`:

```text
0x0060D294  CALL 0x0072D030
0x0060D299  MOV [ESI + 0x74],EAX
0x0060D29C  MOV ECX,[0x00B0FB50]
0x0060D2A2  MOV [ESI + 0xE0],ECX
0x0060D2A8  MOV EDX,[0x00B0FA18]
0x0060D2AE  MOV [ESI + 0xE4],EDX
```

Here `ESI` is the parent dialog metadata record for the HWND passed to `0x0060CF00`.

| Parent offset | Value | Meaning for dialog `0x102` |
|---:|---|---|
| `+0x74` | result of `FUN_0072D030()` | parent convert/palette object, backed by `MnScrnLCoopGameSetup.PAL` path |
| `+0xE0` | `DAT_00B0FB50` | parent background SHP pointer; xref chain maps this to `MNSCRNL.SHP` |
| `+0xE4` | `DAT_00B0FA18` | alternate/width-dependent parent background SHP pointer; loaded as `MnScrnLCoopGameSetup.shp` only when width is exactly 800 |

Important offset warning: parent record `+0xE0` is a background SHP pointer here. Prior reports also found child-control `+0xE0` used as a right-anchor inset override. Those are context-specific uses of the same offset in different records; do not copy parent semantics into child layout code.

### 3.5 `WM_PAINT_Handler @ 0x00621E90` composition branch

`0x00622B50` delegates `WM_PAINT` to `WM_PAINT_Handler @ 0x00621E90` unless the parent record suppress byte at `+0xC0` is set. In `0x00621E90`, when parent mode `+0xB0 == 1`, it:

1. Creates/reuses a cached `BSurface` for the parent client.
2. Calls `RightPanel__Draw`.
3. Fetches the same parent metadata fields `+0x74`, `+0xE0`, `+0xE4`.
4. Calls `Background_Overlay`.
5. Optionally draws generic top-highlight/minimap/radar extras if bytes `+0xD5..+0xDB` are set.
6. Blits the cached parent surface to `DAT_00887310`.

If parent mode is not `1` or `2`, it falls back to `dbak6440.pcx` and centers the PCX if smaller than the screen. For standard dialog `0x102`, `0x0060CF00` selects mode/background fields consistent with the mode-1 branch, so `dbak6440.pcx` is not the confirmed offline Skirmish first-paint background.

### 3.6 `Background_Overlay @ 0x0072E730` width selection

`Background_Overlay` clamps the destination right/bottom to a centered 800x600 region when larger than 800/600, then draws:

```text
if g_ScreenWidth == 0x280:   ; 640
    CC_Draw_Shape(param_4, frame 0, ...)
else:
    CC_Draw_Shape(param_5, frame 0, ...)
```

For dialog `0x102`, `param_4` is parent `+0xE0` (`MNSCRNL.SHP`) and `param_5` is parent `+0xE4` (`MnScrnLCoopGameSetup.shp` at width 800).

Player-visible implication:

- At `640` width, the verified parent background asset is `MNSCRNL.SHP`.
- At `800` width, the verified parent background asset is `MnScrnLCoopGameSetup.shp`.
- At `>800`, the function chooses the alternate path, but `0x0072CF40` only populated `DAT_00B0FA18` at exactly 800. Do not invent a high-resolution background substitution until `CC_Draw_Shape(NULL, ...)` or a retail screenshot resolves the final output.

### 3.7 Right panel draw order

`RightPanel__Draw @ 0x0072E450` lazily opens/loading panel resources, computes layout rects, fills margins, then draws:

1. `SDTP.SHP` frame `0` at `DAT_00B0FC20`.
2. `SDBTNBKGD.SHP` frame `0` tiled vertically `DAT_00B0FA20` times at `DAT_00B0FC24`.
3. If the boolean parameter says to include it, `SDBTNANM.SHP` frame `10` tiled vertically at `DAT_00B0FC10`.
4. `SDBTM.SHP` frame `0` at `DAT_00B0FC28`.
5. A lower/side piece selected by width (`DAT_00B0FA54` normally, `DAT_00B0FAE8` at 640).

This happens inside the common parent paint path before Skirmish-specific preview markers.

### 3.8 Generic/background-candidate asset statuses

| Asset | Evidence | Active in offline `0x102`? | Classification |
|---|---|---|---|
| `MNSCRNL.SHP` | string `0x00845144`, pointer `0x00844CE4`; right-panel/general loads write `DAT_00B0FB50`; `0x0060CF00` writes `DAT_00B0FB50` to parent `+0xE0`; `Background_Overlay` draws parent `+0xE0` at width 640 | Yes at width 640 | Verified parent background |
| `MnScrnLCoopGameSetup.shp` | string `0x00844FA8`, pointer `0x00844D6C`; loaded by `0x0072CF40` at exact width 800 into `DAT_00B0FA18`; `0x0060CF00` writes it to parent `+0xE4` | Yes at width 800 | Verified parent background |
| `MnScrnLCoopGameSetup.PAL` | string `0x00844F8C`, pointer `0x00844D70`; loaded by `0x0072CF40` into `DAT_00B0FCDC/FCE0`; `0x0060CF00` parent `+0x74` points to this convert object | Yes | Verified parent palette |
| `MNSCRNS.SHP` | string `0x00845150`, pointer `0x00844CE0`; table read in broader shell/right-panel loads | No direct `0x102` selection found | Generic shell asset for this scope |
| `MnScrnLCustomizeBattle.shp` | string `0x00844FE0`, pointer `0x00844D64`; not selected by `0x0072CF40` or `0x0060CF00` dialog `0x102` branch | No evidence | Broad shell/WOL-style asset |
| `dbak6440.pcx` | owner-draw preload `0x0061F210`; fallback in `WM_PAINT_Handler` non-mode-1/2 branch | Not in confirmed `0x102` mode-1 paint | Generic fallback/preload |
| `dlgsysa.pcx` | loaded by `FUN_006BA120` in `0x0061F210` | Preload only here | Generic owner-draw pool |
| `dlgsysi.pcx` | loaded with mode `1` in `0x0061F210` | Preload only here | Generic owner-draw pool |

## 4. Button Rendering

### 4.1 Active callback

`0x0060F9A0` assigns `OwnerDraw_Button_00612B70` to Button controls whose style low bits satisfy the relevant `0x0B` owner-draw branch. Prior reports verified this covers Start `0x617`, Choose Map `0x5AA`, and Back `0x5C0`.

### 4.2 Asset family and state

`OwnerDraw_Button_00612B70` formats these strings:

```text
0x0083589C: b%c%c_li%d.pcx
0x0083588C: b%c%c_mi%d.pcx
0x0083587C: b%c%c_ri%d.pcx
```

For normal Skirmish buttons:

- first `%c` is `'u'` unpressed or `'d'` pressed;
- second `%c` is fixed `'e'`;
- height suffix is selected from threshold values `24` and `30`;
- Skirmish buttons are 37 px high after DLU conversion, so suffix `30` is selected.

Verified assets:

| State | Left | Middle | Right |
|---|---|---|---|
| Unpressed | `bue_li30.pcx` | `bue_mi30.pcx` | `bue_ri30.pcx` |
| Pressed | `bde_li30.pcx` | `bde_mi30.pcx` | `bde_ri30.pcx` |

Disabled state:

- `WS_DISABLED` (`0x08000000`) forces the state char back to `'u'`.
- After drawing, `AlphaBlendRect(..., 0x80)` applies a 50% dim.
- `bud_*` assets are preloaded, but this path does not select them for standard Skirmish buttons.

### 4.3 Tiling and placement

The callback direct-blits the left cap, tiles the middle piece through `0x006BA3E0`, and direct-blits the right cap. The middle helper locks source and destination and uses modulo addressing over the source tile.

Pressed state adds `+2` to the content/text y position. This is a small visible feedback shift and should not be omitted.

Missing PCX behavior:

- The default cap/middle/cap path immediately dereferences `FUN_006BA140` lookup results.
- There is no primitive fallback in this callback body for missing Skirmish button pieces.
- A faithful renderer should log/skip/fail visibly rather than substituting generic art.

## 5. Text and Font Path

`FUN_00621040` is the shell owner-draw text wrapper used by the button path and other controls.

Verified behavior:

- Converts caller RGB color to active 16-bit DirectDraw format using `g_DD_*Loss` and `g_DD_*Shift`.
- Computes width as `rect.right - rect.left` and height as `rect.bottom - rect.top`.
- If flag bit `0x04` is set, calls `BitFont__MeasureText` and vertically centers text by adding `(rect_height - measured_height) / 2` to y.
- Sets draw state: enable flag `1`, clipping rectangle from the supplied rect, and converted foreground color.
- Calls lower bitfont draw routine `0x00434CD0`.
- Lower routine uses bit `0x01` for horizontal center and bit `0x02` for right align; no bit means left align.
- Lower routine handles tabs by advancing to the next font tab stop, newlines/carriage returns by moving one font line height, wraps at the last remembered space when possible, and otherwise cuts before the overflowing glyph.

Button text call-site implications:

- Button labels should be clipped to their control rect.
- Button labels should use center horizontal alignment and vertical-centering behavior.
- Pressed labels must move down by exactly 2 pixels with the pressed art.
- The recovered button path passed color value `0x00000C05` into this wrapper in prior/live analysis. Because the wrapper converts through 16-bit display masks, GPU RGBA matching still needs screenshot verification.

Open detail:

- This pass verifies wrapper behavior and lower bitfont behavior, but does not fully name the upstream loaded font asset beyond the existing bitfont object used by owner-draw state.

## 6. Preview, STARTBUT, and mmpb

### 6.1 Offline Skirmish `WM_PAINT` order

`FUN_006AE3F0` first calls `FUN_00622B50`. If common shell paint returns `0`, the Skirmish-specific `WM_PAINT` branch runs:

1. If `DAT_00AC1154 != 0`, look up child `0x468`.
2. Call `FUN_006067A0`.
3. If that returns `0`, call `DrawStartPositions @ 0x00640710`.
4. `ValidateRect(parent_dialog, NULL)`.

`get_function_callees(0x006AE3F0)` confirms direct callees include `DrawStartPositions @ 0x00640710` and do not include `0x00640A40`.

### 6.2 `DrawStartPositions @ 0x00640710`

Verified sequence:

1. Calls `ValidateRect(parent_dialog, NULL)` at entry.
2. If preview object pointer is null, returns.
3. Calls `GetDlgItem(parent, 0x468)`.
4. Calls `0x00775690` to convert the preview child HWND rect to shell/backbuffer coordinates.
5. Reads preview/source rect from the preview object vtable `+0x78`.
6. Computes integer scale using `*1000` fixed integer math.
7. Locks/clips `DAT_00887310` through vtable `+0x14`.
8. Blits the preview surface to `DAT_00887310`.
9. Lazily loads `STARTBUT.SHP` from string `0x00836DE4` when guard `DAT_00AC4E90 & 1` is clear.
10. Reads `ScenarioClass+0x113C`; draws only when count is `>0` and `<9`.
11. For each start index, reads X from `ScenarioClass+0x1140 + i*8` and Y from `ScenarioClass+0x1144 + i*8`.
12. Draws `STARTBUT.SHP`, frame `0`, with X offset `-9` and Y offset `-6`.
13. Draws numeric label `i + 1` after the shape through `FUN_004A61C0`.

Player-visible implication: available starts are preview surface first, `STARTBUT.SHP` frame 0 second, text label third. The marker is not a preview backing and not a UI placeholder.

### 6.3 `mmpb.shp` path at `0x00640A40`

`0x00640A40` is active YR code and loads `mmpb.shp` from string `0x00836DF4`, but the confirmed caller is `0x00552D60` in a separate map-preview/render context. It is not called by `0x006AE3F0` or `0x00640710`.

Verified behavior:

- Early-outs if the surface/preview pointer is null.
- Iterates map cells in the playfield and computes projected bounds.
- Counts valid starts by testing `FUN_0068BD80(i)` for `i < 8`.
- Allocates a temporary `DSurface`.
- Loads `mmpb.shp`.
- Iterates assigned start slots beginning at `ScenarioClass+0x1180`.
- Draws only if the assigned house slot is not `-1`, the SHP loaded, and the assigned house's color scheme has non-null data at `+0x30C`.
- Draws frame `0` with X offset `-3` and Y offset `-2`.
- Blits the temporary surface back and destroys the temporary.

Classification: active assigned-player/house marker path elsewhere; not part of standard offline Skirmish dialog `0x102` first-paint draw order.

## 7. Active-In-YR Status Matrix

| Asset/path | Active in YR? | Active in standard offline Skirmish dialog `0x102`? | Notes |
|---|---|---|---|
| `0x0072CF40` | Yes | Yes | Called by `0x006AE2C0` before dialog creation |
| `0x00622B50` | Yes | Yes | Common shell init/paint path, delegated first by `0x006AE3F0` |
| `0x00621E90` | Yes | Yes | Parent cache/background/right-panel paint handler |
| `0x0060CF00` | Yes | Yes | Parent background field setup for dialog id `0x102` |
| `0x00612B70` | Yes | Yes | Start/Choose/Back owner-draw button callback |
| `0x00621040` | Yes | Yes | Shell text wrapper used by button path |
| `0x00640710` | Yes | Yes | Offline preview surface and `STARTBUT.SHP` available-start markers |
| `0x00640A40` | Yes | No for confirmed first paint | Assigned-player marker path elsewhere |
| `MnScrnLCoopGameSetup.shp` | Yes | Yes at width 800 | Loaded only on exact `g_ScreenWidth == 800` |
| `MnScrnLCoopGameSetup.PAL` | Yes | Yes | Parent/background palette convert path |
| `MNSCRNL.SHP` | Yes | Yes at width 640 | Parent `+0xE0` and `Background_Overlay` 640 path |
| `MNSCRNS.SHP` | Yes broad shell | No direct evidence | Do not use for offline `0x102` by default |
| `MnScrnLCustomizeBattle.shp` | Yes broad shell table likely | No evidence | Do not use for offline `0x102` |
| `dbak6440.pcx` | Yes owner-draw/fallback | Not confirmed visible in `0x102` mode-1 path | Generic fallback/preload |
| `dlgsysa.pcx`, `dlgsysi.pcx` | Yes preload | Preload only in this scope | Generic owner-draw pool |
| `STARTBUT.SHP` | Yes | Yes | Available-start marker after preview surface |
| `mmpb.shp` | Yes | Not confirmed in dialog `0x102` first-paint | Assigned-player/house marker elsewhere |
| `bue_*30.pcx`, `bde_*30.pcx` | Yes | Yes | Normal/pressed Start/Choose/Back |
| `bud_*` | Preloaded | No visible use on normal buttons | Disabled uses alpha on `bue_*` |

## 8. Implementation Guidance for Rust Shell Renderer

1. Promote only these parent-background inputs from research-only to verified default roles: `MNSCRNL.SHP` for width `640`, `MnScrnLCoopGameSetup.shp` for width `800`, and `MnScrnLCoopGameSetup.PAL` as their palette source.
2. Keep high-resolution `>800` parent backgrounds blank/debug-only until `CC_Draw_Shape` null handling or retail screenshots prove the visible result. `Background_Overlay` chooses the alternate pointer there, but `0x0072CF40` only loads it at exactly 800.
3. Do not use `MNSCRNS.SHP`, `MnScrnLCustomizeBattle.shp`, `dbak6440.pcx`, `dlgsysa.pcx`, or `dlgsysi.pcx` as default offline Skirmish backgrounds. They may remain research candidates or generic shell assets.
4. Render right panel before child controls using the verified `RightPanel__Draw` order: `SDTP`, repeated `SDBTNBKGD`, optional `SDBTNANM` frame 10 overlay, `SDBTM`, then the lower/width-selected panel piece.
5. Render buttons from native cap/middle/cap PCXs. Do not stretch a single piece; repeat and clip the middle. Use `bue_*30` unpressed and `bde_*30` pressed. Disabled uses unpressed art plus alpha `0x80`, not `bud_*`.
6. Centralize shell text placement around the `0x00621040` behavior: clip rect, center alignment flags, vertical centering flag `0x04`, and pressed y offset `+2`.
7. Draw no retail art as the preview backing unless the real preview surface is decoded. When preview exists, draw it first, then `STARTBUT.SHP` frame `0`, then numeric labels. Do not use `mmpb.shp` or `SDMPBTN.SHP` as a preview placeholder.
8. Keep `mmpb.shp` available only for a future assigned-player/house marker implementation matching the separate caller context, not the offline available-start marker path.

## 9. Inference and Remaining Unknowns

Verified binary findings above are separate from these inferences/open items:

- The parent background should be modeled as a dialog parent cache/background overlay, not as a stretched fullscreen image. This is an implementation inference from `WM_PAINT_Handler` and `Background_Overlay`.
- `MnScrnLCoopGameSetup.PAL` is verified for parent background convert object. Exact right-panel SHP palette identity is still not as tightly isolated in this pass.
- `>800` visible background output remains unresolved. The code path is known, but if `DAT_00B0FA18` is null above 800, lower-level `CC_Draw_Shape` behavior decides whether nothing, stale art, or a failure occurs.
- The upstream font asset identity for owner-draw text remains partially unnamed. The wrapper/lower bitfont behavior is verified.
- Runtime screenshot capture at 640x480, 800x600, and 1024x768 remains valuable for pixel confirmation, especially high-res.

## 10. Sources

Fresh Ghidra MCP evidence used in this pass:

- `list_instances`: connected to `gamemd.exe`, image base `0x00400000`, executable path `<ra2-install>/gamemd.exe`.
- `0x0072CF40` decompile/disassembly: guard, exact width-800 branch, `MnScrnLCoopGameSetup.shp/.PAL` loads.
- `0x0072CF90` decompile: paired cleanup.
- `0x0072ADE0` decompile: PAL decode/convert object construction.
- `0x004A38D0` decompile: SHP/file load helper with ownership byte.
- `0x006AE2C0` decompile: offline Skirmish caller ordering.
- `0x006AE3F0` decompile/callees: common shell delegation, Skirmish `WM_PAINT`, `DrawStartPositions` call.
- `0x00622B50` decompile: common shell init/paint handling.
- `0x00621E90` decompile: parent cache, right-panel, background overlay, `dbak6440.pcx` fallback branch.
- `0x0060CF00` decompile/disassembly: dialog `0x102` parent `+0x74/+0xE0/+0xE4` writes.
- `0x0072E730` decompile: `Background_Overlay` branch.
- `0x0072E450` decompile: right-panel draw order.
- `0x00612B70` decompile: button message handling, PCX format strings, pressed/disabled behavior.
- `0x00621040` and `0x00434CD0` decompiles: text wrapper and lower bitfont draw behavior.
- `0x00640710` decompile/callees: preview surface, `STARTBUT.SHP`, numeric label order.
- `0x00640A40` decompile/callers/callees: `mmpb.shp` assigned-player marker path outside confirmed offline dialog first paint.
- String/xref checks:
  - `MnScrnLCoopGameSetup.PAL` `0x00844F8C` -> `0x00844D70` -> `0x0072CF6A`
  - `MnScrnLCoopGameSetup.shp` `0x00844FA8` -> `0x00844D6C` -> `0x0072CF55`
  - `MNSCRNL.SHP` `0x00845144` -> `0x00844CE4`
  - `MNSCRNS.SHP` `0x00845150` -> `0x00844CE0`
  - `MnScrnLCustomizeBattle.shp` `0x00844FE0` -> `0x00844D64`
  - `dbak6440.pcx` `0x008336FC`, owner-draw/fallback refs
  - `dlgsysa.pcx` `0x00836284`, `dlgsysi.pcx` `0x00836290`
  - `STARTBUT.SHP` `0x00836DE4` -> `0x006408A3`
  - `mmpb.shp` `0x00836DF4` -> `0x00640E44`
  - button format strings `0x0083587C/8C/9C`
