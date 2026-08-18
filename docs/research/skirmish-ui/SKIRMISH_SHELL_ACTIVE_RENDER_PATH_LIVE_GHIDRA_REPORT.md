# Skirmish Shell Active Render Path - Live Ghidra Report

## Superseded Asset-Family Correction - 2026-05-24

For standard Skirmish setup sidebar Start Game `0x617`, Choose Map `0x5AA`, and
Back `0x5C0`, older button rows in this report that name the generic PCX path
are superseded. The corrected classifier recheck proves these three right-panel
buttons are owner-draw type `1` and draw `SDBTNANM.SHP` frames `2`/`4`. Use
`SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md` before
changing the Rust button art path.

**Address(es):** `0x0072CF40`, `0x00622B50`, `0x0060CF00`, `0x00612B70`, `0x00621040`, `0x00640710`, `0x00640A40`
**Confidence:** High for the listed function bodies and call ordering verified in live Ghidra; medium where Ghidra's decompiler recovered bad prototypes but assembly/context resolved the behavior.
**Active in YR:** Yes for the offline Skirmish shell `0x102` path except `0x00640A40`, which is active YR map-preview/player-marker code but not called by the offline Skirmish dialog `WM_PAINT` path observed here.

## Overview

This was a fresh live Ghidra reinvestigation of the unresolved active render pieces for the offline Yuri's Revenge Skirmish shell. Live Ghidra was available; the requested addresses decompiled or disassembled in the loaded `gamemd.exe`. Numeric address lookup failed until using the project's existing function symbols, but memory at the requested addresses was present and matched executable code.

The player-visible path is a Win32 child dialog resource `0x102` hosted at shell client origin `(0,0)`. Common shell initialization creates/subclasses owner-draw records, assigns parent background assets, resizes/repositions selected controls, and on paint draws the parent background/right-panel cache before Skirmish-specific preview overlay code draws the map preview markers.

## Prior Report Conflicts Or Corrections

1. `MnScrnLCustomizeBattle.shp` remains a broad shell/WOL-style asset, not the offline Skirmish `0x102` background. Live `0x0072CF40` proves the directly loaded Skirmish-paired screen asset is `MnScrnLCoopGameSetup.shp` plus `MnScrnLCoopGameSetup.PAL`.
2. `MNSCRNL.SHP` is not merely an unproven candidate. Live `0x0060CF00` writes `DAT_00B0FB50` into the parent dialog `+0xE0`, and live right-panel loading maps `DAT_00B0FB50` to `MNSCRNL.SHP`. It is a parent-background asset for dialog `0x102`, especially relevant to the `g_ScreenWidth == 640` branch in `Background_Overlay`.
3. The parent record `+0xE0` and child records `+0xE0` are context-dependent. For the parent shell dialog record, `+0xE0` is a background SHP pointer. For right-anchored child records, prior reports correctly found the same offset is an optional numeric inset override. The `0x0060CF00` write does not feed child controls.
4. `bud_*` PCXs remain preload-only for normal Skirmish Start/Choose/Back buttons. Live `0x00612B70` again selects `bue_*` or `bde_*`, forces disabled state to the unpressed branch, then applies alpha.
5. `mmpb.shp` is not ordered between the offline Skirmish dialog's map preview draw and `STARTBUT.SHP` labels in `0x006AE3F0`. Live callers show `0x006AE3F0` calls `DrawStartPositions @ 0x00640710`; `0x00640A40` is called from a separate map preview/rendering context around `0x00553687`.

## Function-By-Function Findings

### `0x0072CF40` - Skirmish background/palette resource load

**Active in YR:** Yes. Called by `0x006AE2C0` before creating/showing offline Skirmish dialog `0x102`. Also called from other shell/WOL contexts, so the function is generic shell setup plus active offline Skirmish.

Verified behavior:

- Guard byte `DAT_00B0FCD9` prevents repeated loads. If it is nonzero, the function returns immediately.
- If `g_ScreenWidth == 800`, it loads `MnScrnLCoopGameSetup.shp` via `0x004A38D0`, writes ownership byte `DAT_00B0FCD8`, and stores the returned SHP pointer in `DAT_00B0FA18`.
- It always attempts to load `MnScrnLCoopGameSetup.PAL` through `0x0072ADE0`, passing raw palette output `DAT_00B0FCDC` and convert object output `DAT_00B0FCE0`.
- `0x0072ADE0` allocates `0x300` bytes for 256 RGB triplets, copies 256 palette entries, shifts each channel left by `2`, then constructs a `ConvertClass` against `DAT_00887310`.
- Failure behavior is not a hard error here. If file load fails, the corresponding pointer remains `0`, but `DAT_00B0FCD9` is still set to `1`. Cleanup checks ownership/pointer fields before freeing.
- Paired cleanup `0x0072CF90` frees `DAT_00B0FA18` only if `DAT_00B0FCD8` is set, frees `DAT_00B0FCDC`, destructs `DAT_00B0FCE0`, clears pointers, then clears `DAT_00B0FCD9`.

Screen-width branch detail:

- `800`: `DAT_00B0FA18 = MnScrnLCoopGameSetup.shp`; parent paint can use it.
- `640`: this function does not load `DAT_00B0FA18`; parent paint's `Background_Overlay` selects the `DAT_00B0FB50` path instead.
- Other widths: this function still loads the palette/convert object, but does not load `DAT_00B0FA18`. If `Background_Overlay` receives a null alternate SHP, the draw call depends on lower-level `CC_Draw_Shape` null handling, which was not reopened in this pass.

### `0x00622B50` - common shell paint/background/dialog path

**Active in YR:** Yes. `0x006AE3F0` delegates to it before Skirmish-specific message handling.

Verified behavior:

- `WM_INITDIALOG (0x110)` increments `DAT_00A8ED8C`, initializes common shell state, ensures an owner-draw metadata record for the dialog, calls `0x0060CF00` unless `FUN_0069BBE0()` says the alternate/WOL branch is active, then calls setup helpers in this order:
  `0x0060CAF0`, `0x0060C930`, `0x0060CCC0`, `0x0060CDB0`, child owner-draw enumeration `LAB_0060AAB0`, fullscreen shell-host check `0x0060C540`, optional resize/reposition `0x0060C4A0`, then final enumerations `LAB_0060A330` and `LAB_0060A5B0`.
- For dialog `0x102`, `0x0060C540` returns true and writes parent record flags `+0xB4 = 1` and byte `+0xC1 = 1`; `0x0060C4A0` moves the parent HWND to `(0,0,g_ScreenWidth,g_ScreenHeight)` and enumerates children for final layout.
- `WM_PAINT (0x0F)` finds the parent metadata record. If byte at parent state `+0xC0` is nonzero, it validates the full rect and returns `1`, suppressing the normal background draw.
- Otherwise it calls `WM_PAINT_Handler @ 0x00621E90`, then if the parent child-state byte `+0xBE` is set it sends custom `0x4E2` to child `0x71A`, calls `0x006071E0`, clears `+0xBE`, validates the full parent rect, and returns `0`.
- `WM_ERASEBKGND (0x14)` returns `1`, preventing GDI erase.
- `WM_CTLCOLOR* (0x132..0x138)` returns `GetStockObject(4)`.
- `WM_SETCURSOR (0x20/0x84 branch)` includes tooltip/hit-test behavior using child messages `0x4E8` and parent messages `0x4E9`; it is shell-generic and not part of the main offline Skirmish first-paint composition.

Background cache details from `WM_PAINT_Handler @ 0x00621E90`:

- It calls `GetClientRect` and `0x00775690` to convert the HWND to main shell/backbuffer coordinates.
- If parent state `+0x20` has no cached `BSurface`, it allocates one sized to the client rect and initializes a 16-bit pixel buffer.
- For parent mode `+0xB0 == 1`, it calls `RightPanel__Draw`, fetches parent state `+0x74`, `+0xE0`, `+0xE4`, then calls `Background_Overlay`.
- For parent mode `+0xB0 == 2`, it draws another SHP-family background path, not used by dialog `0x102` from `0x0060CF00`.
- Otherwise it falls back to `dbak6440.pcx`, centering the PCX if it is narrower/shorter than the screen.
- After composing into the cached `BSurface`, it blits the cache to `DAT_00887310` using the converted shell/backbuffer rect.

### `0x0060CF00` - parent dialog background asset setup

**Active in YR:** Yes for dialog `0x102`.

For dialog resource id `0x102`, the function writes:

| Parent state offset | Node expression in decompiler | Value for `0x102` | Meaning in this context |
|---:|---|---|---|
| `+0x74` | `piVar2[0x1E]` | `FUN_0072D030()` -> `DAT_00B0FCE0` | palette/convert object from `MnScrnLCoopGameSetup.PAL` |
| `+0xE0` | `piVar2[0x39]` | `DAT_00B0FB50` | parent background SHP pointer, loaded from `MNSCRNL.SHP` |
| `+0xE4` | `piVar2[0x3A]` | `DAT_00B0FA18` | alternate/width-dependent parent background SHP pointer, loaded from `MnScrnLCoopGameSetup.shp` only at width `800` |

These fields are written on the parent dialog's metadata record. Live caller/callee tracing did not show these values being copied into child records. Child controls get their own owner-draw metadata through `0x0060F9A0` and related callbacks; the right-anchor child `+0xE0` inset remains zero for `0x617`, `0x5AA`, and `0x468` per the prior follow-up and this pass did not contradict it.

`Background_Overlay` selection:

- It receives the parent `+0x74`, `+0xE0`, `+0xE4` values.
- It clamps the destination rect's right/bottom to an 800x600-centered region when larger than 800/600.
- It draws `param_4` when `g_ScreenWidth == 0x280` (`640`).
- It draws `param_5` otherwise.
- For dialog `0x102`, that means `MNSCRNL.SHP` on the 640 path and `MnScrnLCoopGameSetup.shp` on the 800 path. The >800 path still selects the alternate pointer but `0x0072CF40` only loaded it at exactly 800.

### `0x00612B70` - owner-draw button callback

**Active in YR:** Yes. Assigned to Skirmish Start `0x617`, Choose Map `0x5AA`, and Back `0x5C0` through `0x0060F9A0` when button style low bits satisfy `(style & 0x0B) == 0x0B`.

Verified behavior:

- Unknown/unhandled messages call the previous WndProc.
- `WM_ENABLE (0x0A)`, `WM_KILLFOCUS (0x08)`, and activation-related `0x21` return `0`.
- `WM_LBUTTONDOWN (0x201)` and double-click `0x203` play the shell click sound unless the state byte at `+0xBC` is set.
- `WM_TIMER (0x113)` toggles byte `+0xC5` and invalidates the whole control with erase `TRUE`.
- Custom `0x4DC` starts/stops a 1000 ms timer, sets/clears byte `+0xC4`, clears `+0xC5` on stop, kills the timer, and invalidates.
- `WM_PAINT` restores/caches the parent background into a per-control `BSurface`, composes the skin, draws text, alpha-blends if disabled, then validates the control rect.

Default cap/middle/cap behavior:

- It chooses state character `'u'` for unpressed and `'d'` for pressed. If `WS_DISABLED (0x08000000)` is set, it forces `'u'`.
- The second format character is fixed `'e'` on this path.
- The height suffix comes from threshold table `{24, 30}`. The loop selects the largest threshold not exceeding the client height; Skirmish's 37 px buttons therefore use `30`.
- It formats:
  - `b%c%c_li%d.pcx`
  - `b%c%c_mi%d.pcx`
  - `b%c%c_ri%d.pcx`
- Normal offline Skirmish buttons therefore use `bue_li30.pcx`, `bue_mi30.pcx`, `bue_ri30.pcx`; pressed uses `bde_li30.pcx`, `bde_mi30.pcx`, `bde_ri30.pcx`.
- Left cap is direct-blitted to `DAT_00887310`.
- Middle is tiled with `0x006BA3E0`. The helper locks source and destination, computes a centered modulo tile origin, and writes every destination pixel from `(src_x % src_w, src_y % src_h)`.
- Right cap is direct-blitted.
- Pressed state adds `+2` to content/text vertical placement.
- Disabled state does not switch to `bud_*`; it draws the unpressed enabled art and then applies `AlphaBlendRect(..., 0x80)`.

Fallback and missing-PCX behavior:

- If state `+0x10` has a cached/custom surface, it blits that instead of composing the PCX pieces.
- SHP modes at state `+0xB0` values `1..3` check both palette/helper and SHP pointers before `CC_Draw_Shape`.
- The default PCX cap/middle/cap path immediately dereferences `0x006BA140` results for width/blit. There is no primitive/GDI fallback for missing button PCXs in this body.

### `0x00621040` - shell text/font rendering wrapper

**Active in YR:** Yes. Called by the owner-draw button path and other shell owner-draw controls.

Verified behavior:

- It receives a bitfont/draw object, text pointer, destination rect, RGB color, flags, and extra clipping/fade arguments. Ghidra's C prototype is shifted by calling convention, but assembly confirms the same call stack.
- RGB input is converted to the active 16-bit DirectDraw pixel format using `g_DD_RLoss/GShift/BLoss` and `g_DD_RShift/GShift/BShift`.
- It computes rect width as `right - left` and height as `bottom - top`.
- If flags contain bit `0x04`, it calls `BitFont__MeasureText`, reads measured text height, and vertically centers the text by adding `(rect_height - measured_height) / 2` to the starting y.
- It sets font/draw state: enable flag `1`, clip rect to the supplied rectangle, foreground color to the converted 16-bit color.
- It calls the lower bitfont draw routine `0x00434CD0`.
- Horizontal alignment is handled by flags passed to the lower routine: bit `0x01` centers a line within the available width, bit `0x02` right-aligns by using `available_width - measured_width`, and no bit means left-aligned.
- Newline handling in the lower routine advances by font line height. Tab handling advances to the next tab stop using the font's tab width. If text exceeds width, the lower routine wraps at the last recorded space when possible; otherwise it cuts before the overflowing glyph.
- Magic constants observed here: flag `0x04` for vertical centering, RGB-to-16-bit loss/shift globals, and lower routine fade/clip parameters passed through by callers. Button text in `0x00612B70` passes color `0x00000C05` in the recovered call site, which is converted through the same path.

### `0x00640710` - `DrawStartPositions`

**Active in YR:** Yes. Called by offline Skirmish dialog proc `0x006AE3F0` on `WM_PAINT` when the map preview object exists and the preview child is not in the suppressing state returned by `0x006067A0`.

Verified order:

1. Calls `ValidateRect(parent_dialog, NULL)` at function entry.
2. If preview object pointer `*param_1` is null, returns.
3. Looks up child `0x468` with `GetDlgItem`.
4. Converts the child HWND rect to shell/backbuffer coordinates using `0x00775690`. All final marker coordinates are in `DAT_00887310` coordinate space derived from child `0x468`, not raw dialog resource coordinates.
5. Reads the preview object's source rect via vtable `+0x78`, computes a scale factor from child rect to source preview dimensions using integer math with `*1000`.
6. Locks/clips the main backbuffer with vtable `DAT_00887310 + 0x14`.
7. Blits the preview object/surface to `DAT_00887310`.
8. Lazily loads `STARTBUT.SHP` from string `0x00836DE4` guarded by `DAT_00AC4E90 & 1`, storing the result in `DAT_00AC4E80`.
9. Iterates starts only if `ScenarioClass+0x113C` is in `1..8`.
10. For each start index `i`, reads X from `ScenarioClass+0x1140 + i*8` and Y from `ScenarioClass+0x1144 + i*8`.
11. Draws `STARTBUT.SHP`, frame `0`, at the projected child-space coordinate with X offset `-9` and Y offset `-6`.
12. Increments the index and draws numeric label `i + 1` after the shape through `0x004A61C0`, using text/color conversion and clipping against `DAT_00887310`.

Clipping/space:

- The child `0x468` rect is the final shell/backbuffer anchor. Prior verified final rects are `(644,37,144,112)` at 800x600 and `(756,121,144,112)` at 1024x768.
- The preview surface is scaled to fit inside the child rect while preserving aspect ratio, using integer scale `min(child_width*1000/src_height, child_height*1000/src_width)` as recovered by the decompiler/assembly.
- Draws are issued against `DAT_00887310`; clipping is obtained from the surface clip state around the preview rect.

### `0x00640A40` - `mmpb.shp` assigned-player marker path

**Active in YR:** Conditional. The function is live YR map-preview/player-marker code and uses `mmpb.shp`, but the direct caller found in this live pass is a separate map/render context at `0x00553687`, not offline Skirmish dialog proc `0x006AE3F0`.

Verified behavior:

- Early-outs if the preview/surface pointer is null.
- Walks map cells through `MapClass__CellIterator_*`, keeps cells in playfield, projects cell centers by `cell_x*0x100 + 0x80` and `cell_y*0x100 + 0x80`, and computes min/max projected bounds using divisions by `0x3C` and `0x1E`.
- Counts valid starts with `FUN_0068BD80(i)` for `i < 8`.
- Creates a temporary `DSurface`.
- Loads `mmpb.shp` from string `0x00836DF4`.
- Iterates assigned start slots beginning at `ScenarioClass+0x1180` in 4-byte steps.
- For a marker to draw: the start index must be valid, the assigned house slot must not be `-1`, `mmpb.shp` must be loaded, and the assigned house color scheme pointer at `+0x30C` must be nonzero.
- Draws `mmpb.shp`, frame `0`, with X expression including `-3` and Y expression including `-2`.
- After drawing into the temporary surface, it blits the temporary result back to the caller surface and destroys the temporary surface.

Ordering relative to offline Skirmish:

- `0x006AE3F0` `WM_PAINT` order is common shell handler first, then Skirmish preview path, then `ValidateRect`.
- In that Skirmish path, if `0x006067A0(child 0x468)` returns false, it calls `0x00640710`.
- `0x00640710` draws the preview surface first, then `STARTBUT.SHP`, then numeric labels.
- No direct call from `0x006AE3F0` or `0x00640710` to `0x00640A40` was found. Therefore `mmpb.shp` is not part of the confirmed offline Skirmish first-paint ordering; it is assigned-player marker code in another live YR preview context.

## Exact Asset And Palette Table

| Asset / palette | Address evidence | Loaded/stored at | Active in offline `0x102`? | Notes |
|---|---|---|---|---|
| `MnScrnLCoopGameSetup.shp` | string `0x00844FA8`, pointer table `0x00844D6C`, load at `0x0072CF55..0x0072CF65` | `DAT_00B0FA18`, ownership byte `DAT_00B0FCD8` | Yes, conditional width `800` | Parent alternate/background SHP for dialog `0x102`. |
| `MnScrnLCoopGameSetup.PAL` | string `0x00844F8C`, pointer table `0x00844D70`, load at `0x0072CF6A..0x0072CF7A` | raw palette `DAT_00B0FCDC`, convert object `DAT_00B0FCE0` | Yes | Loaded regardless of width once `0x0072CF40` runs. |
| `MNSCRNL.SHP` | string `0x00845144`, pointer table `0x00844CE4`, load writes `DAT_00B0FB50` at `0x0072DE0C..0x0072DE1C`/`0x0072E0C0..0x0072E0D0` | `DAT_00B0FB50` | Yes | Written to parent state `+0xE0` for dialog `0x102`; selected by `Background_Overlay` at width `640`. |
| `MNSCRNS.SHP` | string `0x00845150`, pointer table `0x00844CE0` | `DAT_00B0FAC0` in the right-panel/general shell load table | Generic shell; not directly selected by `0x0060CF00` for `0x102` | 640/small-screen corner/background asset in the broader table. |
| `SDBTNANM.SHP` | string `0x00845178`, pointer table `0x00844CD4` | `DAT_00B0FAC4` per prior/follow-up labels | Yes | Back button size/animation asset; 156x42 in prior retail-asset probe. |
| `SDBTNBKGD.SHP` | string `0x00845128`, pointer table `0x00844CEC` | right-panel table | Yes | Right-panel tile. |
| `SDTP.SHP` | string `0x00845138`, pointer table `0x00844CE8` | right-panel table | Yes | Right-panel top cap. |
| `SDBTM.SHP` | string `0x0084511C`, pointer table `0x00844CF0` | right-panel table | Yes | Right-panel bottom cap/edge. |
| `SDMPBTN.SHP` | string `0x0084516C`, pointer table `0x00844CD8` | right-panel table | Generic shell/right panel | Not the map preview background. |
| `STARTBUT.SHP` | string `0x00836DE4`, load at `0x0064089F..0x006408B2` | `DAT_00AC4E80`, guard `DAT_00AC4E90 & 1` | Yes | Available-start marker, frame `0`, followed by numeric text. |
| `mmpb.shp` | string `0x00836DF4`, load at `0x00640E40..0x00640E53` | local temp pointer in `0x00640A40` | No for offline `0x102` first-paint; active elsewhere | Assigned-player/house marker path. |
| `bue_li30/mi30/ri30.pcx` | format strings `0x0083589C`, `0x0083588C`, `0x0083587C`; selected by `0x00612B70` | owner-draw PCX cache | Yes | Unpressed enabled 37 px Skirmish buttons. |
| `bde_li30/mi30/ri30.pcx` | same format strings and state char `'d'` | owner-draw PCX cache | Yes | Pressed Skirmish buttons. |
| `bud_*` | preload strings only in `0x0061F210` | owner-draw PCX cache if preload succeeds | Preloaded only for this path | Not selected by normal Skirmish buttons. |
| `dbak6440.pcx` | pointer `PTR_s_dbak6440_pcx_00833654`, fallback in `WM_PAINT_Handler` | owner-draw PCX cache | Generic shell fallback | Not used when parent mode `+0xB0 == 1` for `0x102`. |

## Paint Order And Coordinate-Space Summary

1. `0x006AE2C0` loads Skirmish background resources through `0x0072CF40`, creates modeless child dialog `0x102`, shows it, pumps messages, then frees preview and background resources on exit.
2. `0x006AE3F0` delegates each message to `0x00622B50` first.
3. `WM_INITDIALOG`: `0x00622B50` creates/updates the parent metadata record, `0x0060CF00` writes parent background asset fields, owner-draw child setup subclasses controls, `0x0060C4A0` resizes the parent to the current screen and repositions selected children.
4. `WM_PAINT`: `0x00622B50` calls `WM_PAINT_Handler`, which draws/caches the parent background/right-panel path into a parent `BSurface`, then blits to `DAT_00887310`.
5. After common paint returns `0`, `0x006AE3F0` handles Skirmish-specific `WM_PAINT`: if preview object exists and child `0x468` is not suppressing preview draw, it calls `0x00640710`.
6. `0x00640710` uses final child `0x468` HWND coordinates via `0x00775690`, blits the preview surface, draws `STARTBUT.SHP` markers, then draws numeric labels.
7. Child owner-draw buttons paint in their own callbacks, restore cached parent background, compose PCX pieces in child-client space, draw text through `0x00621040`, alpha if disabled, then validate.

## TS/Generic-Shell Risk Register

| Item | Classification | Active in YR / offline `0x102` status |
|---|---|---|
| `0x0072CF40` | Generic shell resource loader with offline Skirmish caller | Active in YR: Yes. Offline `0x102`: Yes. |
| `MnScrnLCoopGameSetup.*` | Generic name but direct Skirmish setup caller uses it | Active in YR: Yes. Offline `0x102`: Yes. |
| `MNSCRNL.SHP` | Generic/right-panel shell background asset | Active in YR: Yes. Offline `0x102`: Yes as parent `+0xE0`, especially 640 branch. |
| `MNSCRNS.SHP` | Generic shell small-screen asset | Active in YR: Yes in broad shell table. Offline `0x102`: not directly selected by `0x0060CF00` in this pass. |
| `MnScrnLCustomizeBattle.shp/.PAL` | WOL/broad shell table | Active in YR: likely for other shell screens. Offline `0x102`: No evidence. |
| `bud_*` | Owner-draw preload pool | Active in YR: preloaded. Offline `0x102` normal buttons: No visible use found. |
| `number*.pcx` | Owner-draw preload/digit helper pool | Active in YR: preloaded and used elsewhere. Offline `0x102` start labels: No; labels are text after `STARTBUT.SHP`. |
| `mmpb.shp` | Map-preview/player marker path | Active in YR: Conditional/elsewhere. Offline `0x102` first-paint path: not called by `0x006AE3F0`/`0x00640710`. |
| `WM_SETCURSOR` tooltip path in `0x00622B50` | Generic shell UI | Active in YR: Yes. Offline visual composition: not part of first-paint background. |
| `FUN_0069BBE0` alternate branch | WOL/alternate shell branch gate | Active in YR: Conditional. Offline standard Skirmish path observed through `0x006AE2C0` takes the non-WOL branch. |

## Implementation Implications

- Treat `gamemd.exe` output as the visual spec: dialog `0x102` background is parent-record driven, not a single guessed static backdrop.
- Load/use `MnScrnLCoopGameSetup.shp/.PAL` for the 800-width active parent path, and keep `MNSCRNL.SHP` as the verified parent `+0xE0` background asset.
- Do not wire `MnScrnLCustomizeBattle.shp` into offline Skirmish without new evidence; this pass found the more specific active path.
- Keep child `+0xE0` as zero/default inset unless a child-specific write is proven. Do not copy parent `+0xE0` asset pointers into children.
- Render Skirmish buttons from `bue_*30` / `bde_*30` cap-middle-cap PCXs at the verified 37 px client height; do not use `bud_*` for disabled normal Skirmish buttons.
- For missing button PCXs, a faithful engine should not silently substitute a generic button; gamemd's callback has no robust fallback on this path.
- Draw map preview overlays in final child `0x468` backbuffer coordinates: preview surface first, `STARTBUT.SHP` frame `0` markers second, numeric text labels last.
- Keep `mmpb.shp` separate from available-start marker rendering until implementing the assigned-player marker context that calls `0x00640A40`.

## Open Questions

1. The `>800` background case remains behaviorally risky: `0x0072CF40` only loads `DAT_00B0FA18` at exactly 800, while `Background_Overlay` selects the alternate pointer for non-640 widths. A live screenshot or lower-level `CC_Draw_Shape(NULL,...)` trace is needed before deciding what the player sees at 1024x768.
2. `0x006071E0` has a large right-panel/child redraw path with poor decompiler recovery. It is not the first-paint parent background path, but a separate focused pass would be needed to name every flag and frame used there.
3. `0x00621040` font identity is verified as the owner-draw bitfont object passed through metadata/global draw state, but this pass did not name the upstream loaded font asset beyond the existing `g_GAME_FNT`/bitfont state seen in owner-draw metadata setup.
4. The exact lower-level `CC_Draw_Shape` null handling was not decompiled. This matters for the high-resolution `DAT_00B0FA18 == 0` case and missing SHP edge cases.
5. Retail screenshot capture remains useful for pixel confirmation at 640x480, 800x600, and 1024x768.

## Sources With Addresses And Evidence Notes

- `0x0072CF40` live decompile and assembly: guard `DAT_00B0FCD9`, width branch, `MnScrnLCoopGameSetup.shp/.PAL` loads.
- `0x0072CF90` live decompile: paired cleanup and ownership/failure behavior.
- `0x004A38D0` live decompile: SHP/file load returns pointer and writes ownership byte.
- `0x0072ADE0` live decompile: PAL load, 256 triplets, `<<2` channel expansion, `ConvertClass` creation.
- `0x006AE2C0` live decompile: calls `0x0072CF40`, creates/show/pumps/destroys offline Skirmish dialog, then calls `0x0072CF90`.
- `0x006AE3F0` live decompile: common shell delegation, Skirmish `WM_PAINT`, `0x468`, `0x006067A0`, `DrawStartPositions`, final `ValidateRect`.
- `0x00622B50` live decompile: common shell `WM_INITDIALOG`, `WM_PAINT`, `WM_ERASEBKGND`, owner-draw setup order, validation.
- `0x00621E90` (`WM_PAINT_Handler`) live decompile: cached parent `BSurface`, parent mode branches, `RightPanel__Draw`, `Background_Overlay`, `dbak6440.pcx` fallback.
- `0x0060CF00` live decompile: dialog id to parent background asset mapping; exact `0x102` writes to `+0x74`, `+0xE0`, `+0xE4`.
- `0x0072DFB0`/`0x0072E071` assembly context: right-panel/SHP table loads including `MNSCRNL.SHP`, `SDBTNANM.SHP`, `SDBTNBKGD.SHP`, `SDTP.SHP`.
- `0x00612B70` live decompile: button messages, cap/middle/cap PCX format strings, state selection, disabled alpha, cache/validate behavior.
- `0x006BA140`, `0x006BA3E0`, `0x006BA580` live decompile: PCX cache lookup, tiling behavior, transparent blit behavior.
- `0x00621040` live decompile and assembly: color conversion, clipping, vertical centering flag `0x04`, lower bitfont draw call.
- `0x00434CD0`, `BitFont__MeasureText @ 0x00433CF0` live decompile: text wrap/alignment, line height, tabs, lower draw details.
- `0x00640710` live decompile and assembly: child `0x468` coordinate conversion, preview blit, `STARTBUT.SHP` load/draw, numeric label after shape.
- `0x00640A40` live decompile and assembly: `mmpb.shp` assigned-player marker path, temp surface, assigned house/color checks, offsets.
- `0x00553687` assembly xref: caller of `0x00640A40`, showing it is separate from offline Skirmish dialog proc.
- Prior reports read: `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`.
