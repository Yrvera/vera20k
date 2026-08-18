---
title: Skirmish Owner-Draw Callback Bodies (Ghidra Research Report)
date: 2026-05-16
---

# Skirmish Owner-Draw Callback Bodies - Ghidra Research Report

## Superseded Asset-Family Correction - 2026-05-24

For standard Skirmish setup sidebar Start Game `0x617`, Choose Map `0x5AA`, and
Back `0x5C0`, older rows in this report that map the controls to
`bue_*30.pcx` / `bde_*30.pcx` are superseded. The corrected classifier recheck
proves these three right-panel buttons are owner-draw type `1` and draw
`SDBTNANM.SHP` frames `2`/`4`. Use
`SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md` for the
current contract before changing code.

## Scope

This report extends, but does not duplicate:

- `SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`
- `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
- `SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`

The focus here is the recovered owner-draw callback bodies around `0x00612B70`,
`0x00617250`, `0x006153E0`, `0x00614190`, `0x00614B30`, `0x00618D40`,
`0x0061C690`, `0x0061D950`, `0x006163A0`, `0x00616980`, and `0x0061E700`,
with specific mapping to offline Skirmish dialog resource `0x102`.

**Active in YR:** Yes. The hook setup is called from the shell dialog path,
offline Skirmish dialog `0x102` uses these Win32 classes/styles, and Skirmish
init/command handlers send the messages described below.

**Overall confidence:** High for callback assignment, messages, directly
referenced assets, and Skirmish control mapping. Medium for a few exact pixel
placement details where Ghidra recovered stack variables poorly but assembly
confirmed the constants and call ordering.

## 1. Verified Binary Findings

### 1.1 Hook setup and control classes

Evidence: `FUN_0060F9A0`.

`FUN_0060F9A0` reads the child class name with `GetClassNameA`, reads style via
`GetWindowLongA(hwnd, GWL_STYLE)`, installs wrapper proc `0x00610CA0` with
`SetWindowLongA(hwnd, GWL_WNDPROC, 0x610ca0)`, stores the real callback in the
owner-draw table, stores the previous WndProc in a second table, creates/updates
the per-control state block, then sends message `0x497` to the control.

It preloads the global owner-draw PCX pool through `FUN_0061F210` exactly once
when `DAT_00AC48D4 == 0`.

| Class / style condition | Callback | Kind stored | Skirmish use |
|---|---:|---:|---|
| `Button`, low style bits `0x0B` | `0x00612B70` | `0` | Start Game `0x617`, Choose Map `0x5AA`, Back `0x5C0` |
| `ComboBox` | `0x00617250` | `3` | player/side/color/start/team combos |
| `Static` | `0x006153E0` | `2` | flag statics `0x6DA..0x6E1`; map thumbnail placeholder is hooked but preview is mostly drawn by Skirmish paint code |
| `Edit` | `0x00614190` | `1` | player name edit `0x6A0` |
| `NewEdit` | `0x00614B30` | `1` | same custom edit framework, not the offline `0x6A0` class in resource `0x102` |
| `ListBox` | `0x00618D40` | `4` | combo dropdown popup/list behavior |
| `ScrollBar` | `0x0061C690` | `8` | list/dropdown scrollbar support |
| `msctls_trackbar32` | `0x0061D950` | `7` | sliders `0x529`, `0x511`, `0x50C` |
| `Button`, low style bits `0x03` | `0x006163A0` | `0` | checkboxes `0x54E`, `0x693`, `0x696`, `0x69A`, `0x69D` |
| `Button`, low style bits `0x09` | `0x00616980` | `0` | radio-like variant; not used by listed Skirmish controls |
| `Button`, low style bits `0x07` | `0x0061E700` | `0` | frame/group variant; not used by listed Skirmish controls |

Important style-order detail: the hook checks `(style & 7) == 7` before
`(style & 0x0B) == 0x0B`, then `(style & 3) == 3`, then `(style & 9) == 9`.
Skirmish's main buttons are therefore routed to `0x00612B70`; Skirmish
checkboxes route to `0x006163A0`.

### 1.2 Shared draw/conversion helpers

Evidence: `FUN_006BA140`, `FUN_006BA3E0`, `FUN_006BA580`, `FUN_006208F0`,
`FUN_00620720`, `FUN_00621040`, `FUN_006211D0`, `FUN_00623880`.

| Helper | Verified role |
|---|---|
| `FUN_006BA140` | Looks up an already loaded PCX/surface by name in the owner-draw asset cache. It returns `0` if no cache entry matches. |
| `FUN_006BA3E0` | Tiles a source PCX/surface over a destination rect. It centers the tile origin when the destination is larger, then uses modulo addressing across the source. |
| `FUN_006BA580` | Transparent blit helper. It skips pixels matching the caller-provided key color. Static flag images use this path with a magenta key color converted to the active 16-bit display format. |
| `FUN_006208F0` | Draws raised/sunken beveled rectangular frames with primitive line fills. It converts RGB globals through `g_DD_*Loss` / `g_DD_*Shift` before writing to the 16-bit surface. |
| `FUN_00620720` | Draws scrollbar arrow PCXs. It formats `uparrow%c.pcx` / `dnarrow%c.pcx` or `guparrow%c.pcx` / `gdnarrow%c.pcx`; `%c` is `r` released or `p` pressed. |
| `FUN_00621040` | Text draw wrapper. It converts an RGB color to the current 16-bit format, applies optional vertical centering when flag `4` is set, clips to the supplied rect, and calls the bitfont draw routine. |
| `FUN_006211D0` | Lower-level text draw with horizontal/vertical alignment modes. Used heavily by list/dropdown and frame variant paths. |
| `FUN_00623880` | Edit-control text renderer. It handles cursor/selection insertion, password masking, scroll offset, and calls `FUN_006211D0` for visible text spans. |

Palette/conversion finding:

- The callbacks do not load `DIALOG.PAL`, `SHELL.PAL`, or `MAINBTTN.PAL`
  directly.
- PCXs are preloaded through `CDFileClass__Constructor(name, 2, 0)` or
  `FUN_006BA120` for `dlgsysa.pcx`.
- All painting observed here writes to 16-bit `BSurface`/display surfaces.
  Runtime RGB constants are converted with the DirectDraw loss/shift globals.
- The PCX palette decode/conversion is below `CDFileClass__Constructor` /
  `FUN_006BA140` and was not re-decompiled in this pass. Do not infer external
  `.PAL` use from these callbacks.

### 1.3 Button callback `0x00612B70`

Applies to Skirmish `BUTTON` style low bits `0x0B`: Start Game `0x617`,
Choose Map `0x5AA`, Back `0x5C0`.

Messages handled:

| Message | Behavior |
|---:|---|
| `WM_PAINT` `0x0F` | Main draw path. Builds or reuses a cached `BSurface`, composes button skin, draws text, validates rect. |
| `WM_TIMER` `0x113` | Toggles state byte at control-state offset `+0xC5`, then invalidates. Used for blinking/attention behavior. |
| `WM_LBUTTONDOWN` `0x201`, `WM_LBUTTONDBLCLK` `0x203` | If the disabled/blocked flag at `+0xBC` is clear, plays shell click sound and continues to default processing. |
| `WM_ENABLE` `0x0A`, `WM_KILLFOCUS` `0x08`, `WM_MOUSEACTIVATE`/activation-related `0x21` | Returns `0` directly. |
| Custom `0x4DC` | Starts/stops a 1000 ms timer and toggles an auto-highlight byte. |

Default Skirmish button PCXs:

- Format strings at `0x0083589C`, `0x0083588C`, `0x0083587C` are
  `b%c%c_li%d.pcx`, `b%c%c_mi%d.pcx`, `b%c%c_ri%d.pcx`.
- Assembly at `0x00613240..0x0061355D` confirms:
  - first `%c` is state: `'u'` unpressed or `'d'` pressed;
  - second `%c` is fixed `'e'` on the default enabled PCX path;
  - `%d` selects the 24 or 30 pixel family using thresholds `0x18` and `0x1E`.
- The left cap is direct-blitted, the middle piece is tiled with
  `FUN_006BA3E0`, and the right cap is direct-blitted.
- Pressed state shifts the content/text placement: the draw code adds `+2` to
  the vertical position before text/button content.
- Disabled Win32 style `WS_DISABLED` (`0x08000000`) forces state back to `'u'`
  and applies `AlphaBlendRect(..., 0x80)` after drawing; it does not switch to
  a verified `bud_*` PCX in this callback.

Verified Skirmish implication:

- Start/Choose/Back use the `bue_*` / `bde_*` cap-middle-cap PCX family.
- `bud_*` files are preloaded, but this callback body did not show them on the
  normal Skirmish button path.
- `BTN-MINS.SHP` and `BTN-PLUS.SHP` are not referenced by this callback.

Fallback behavior:

- If a per-control custom image pointer at state `+0x14` exists, the callback
  blits that instead of composing PCX pieces.
- For SHP modes at state `+0xB0` values `1..3`, it checks both the palette/helper
  pointer and SHP pointer before `CC_Draw_Shape`.
- On the default PCX piece path, the code dereferences the result of
  `FUN_006BA140` without a robust null fallback. Missing button PCXs are not
  gracefully replaced by GDI drawing in this body.

### 1.4 ComboBox callback `0x00617250`

Applies to all Skirmish combo boxes:

- player/AI: `0x50B`, `0x50E`, `0x516`, `0x51A..0x51D`;
- side: `0x6A1`, `0x510`, `0x513`, `0x514`, `0x51E..0x521`;
- color: `0x6A2`, `0x522..0x528`;
- start: `0x6A3..0x6A8`, `0x6AA`, `0x6AB`;
- team: `0x76D..0x774`.

Messages handled:

| Message | Behavior |
|---:|---|
| `0x497` | Initialization. Sets combo item heights to font height `+2` and `+6`, marks initialized, sets current selection to `-1`, and fills 50 per-item color/data slots with `-1`. |
| `WM_PAINT` `0x0F` | Draws collapsed combo: cached parent background, alpha/fill, primitive frame, arrow button, selected text, optional color swatch. |
| `CB_GETCURSEL` `0x147` | Returns stored current selection. |
| `CB_SETCURSEL` `0x14E` | Stores selection, pushes selected text to the edit/display child via `0x4B2`/`0x4B4`, invalidates. |
| `CB_GETITEMDATA`/`CB_SETITEMDATA` wrappers `0x150`, `0x151`, `0x199`, `0x19A` | Wraps Win32 item-data storage with the owner-draw string item record. |
| `0x14F` | Opens/closes custom dropdown. Creates a `ComboDropWin` popup, copies list contents into it, captures mouse, and sends parent message `0x4A9`. |
| `WM_LBUTTONDOWN` `0x201`, `WM_LBUTTONDBLCLK` `0x203` | Plays click sound and toggles dropdown if click is in the arrow/right area. |
| Custom `0x4DD` | Enables per-item color/swatch drawing flag at state byte `+0xCC`. |
| Custom `0x4DE` | Sets maximum dropdown row count at state `+0xD0`. |
| Custom `0x4F1` | Sets grey/alternate arrow state byte at `+0xCD`. |

Assets and composition:

- The collapsed combo frame is primitive, not a cap-piece PCX frame. It calls
  `FUN_006208F0`.
- The arrow is drawn by `FUN_00620720`, which uses:
  - `dnarrowr.pcx` / `dnarrowp.pcx`;
  - `uparrowr.pcx` / `uparrowp.pcx`;
  - grey variants `gdnarrowr.pcx`, `gdnarrowp.pcx`,
    `guparrowr.pcx`, `guparrowp.pcx` when the grey flag is set.
- Selected text is measured and truncated until `BitFont__GetTextWidth` fits
  the available width (`control width - 0x14` in the recovered expression).
- If the color/swatch flag is enabled and selected index is in `0..49`, the
  callback fills a small rectangle using a color value from the `+0x44` array.

Fallback behavior:

- If no owner-draw state exists, it calls the previous WndProc.
- Arrow helper `FUN_00620720` checks for null PCX lookup before blitting.
- The rest of the collapsed combo is primitive drawing/text, so it does not
  depend on `cue_i.pcx` / `cce_i.pcx`.

### 1.5 Static callback `0x006153E0`

Applies to flag statics `0x6DA..0x6E1` and other static shell controls.

Messages handled:

| Message | Behavior |
|---:|---|
| `0x497` | Initializes static state, sets text color to `DAT_00AC18A4`, default kind `0`, and text vertical spacing value `0x0C`. |
| `WM_PAINT` `0x0F` | Draws by kind: text, PCX image, SHP animation/frame, VQ movie, or cached background. |
| `WM_DESTROY` `0x02`, `WM_SIZE`/move related `0x03`, `0x05`, `0x47` | Frees cached surface and invalidates. |
| `0x4B1` | Enables explicit fill color and stores the RGB color. |
| `0x4B2`, `0x4B4` | Copies cached background/parent surface back to the static. |
| `0x4D3..0x4E4`, `0x4DF..0x4F0` | VQ/SHP/movie/animation control messages; not central to Skirmish flags. |
| `0x4D5`, `0x4D6`, `0x4D7` | Set/get frame/notify target for kind `4`. |

Flag PCX flow:

- Side combo helper `FUN_004E3F70` maps side combo IDs to flag static IDs:
  - `0x6A1 -> 0x6DA`;
  - `0x510 -> 0x6DB`;
  - `0x513 -> 0x6DC`;
  - `0x51E -> 0x6DD`;
  - `0x514 -> 0x6DE`;
  - `0x51F -> 0x6DF`;
  - `0x520 -> 0x6E0`;
  - `0x521 -> 0x6E1`.
- It then reads the selected side item data from the side combo, calls
  `FUN_004E3560`, and passes the returned PCX surface to `FUN_00603D30`.
- Assembly at `0x004E4147..0x004E4152` confirms this flow:
  `ECX = side`, `CALL 0x004E3560`, `EDX = EAX`, `ECX = flag_hwnd`,
  `CALL 0x00603D30`.
- `FUN_00603D30` sets static kind/state `+0x74 = 2`, stores the PCX pointer at
  state `+0x18`, and invalidates the static.

Side-to-PCX mapping from `FUN_004E3560`:

| Item data | PCX |
|---:|---|
| `-3` | `obsi.pcx` |
| `-2` | `rani.pcx` |
| `0` | `usai.pcx` |
| `1` | `japi.pcx` |
| `2` | `frai.pcx` |
| `3` | `geri.pcx` |
| `4` | `gbri.pcx` |
| `5` | `djbi.pcx` |
| `6` | `arbi.pcx` |
| `7` | `lati.pcx` |
| `8` | `rusi.pcx` |
| `9` | `yrii.pcx` |

Image placement:

- For static kind `2`, the callback computes the available static client rect.
- If the image is narrower than the rect, it centers horizontally.
- If the image is shorter than the rect, it centers vertically.
- It calls `FUN_006BA580`, the transparent blit helper, with magenta
  `0xFF00FF` converted to the active 16-bit display format as the transparent
  key.
- If the image is larger than the rect, the destination rectangle remains
  clipped to the static area; the result is crop/clip rather than scaling.

Fallback behavior:

- If kind `2` has no PCX pointer, no image is drawn; the callback still validates
  the rect.
- The flag update path passes `0` if `FUN_004E3560` cannot find a matching PCX,
  so missing flag PCXs become blank flag statics rather than a known alternate
  art asset.

### 1.6 Edit callbacks `0x00614190` and `0x00614B30`

Offline Skirmish player name edit `0x6A0` is class `Edit` in resource `0x102`
and uses `0x00614190`. `NewEdit` uses the cleaner `0x00614B30` path for other
shell edit controls.

Important messages:

| Message | Behavior |
|---:|---|
| `0x497` | Moves the child in by `+1,+1` and shrinks width/height by `-2,-2`, producing a 1-pixel frame margin. If style `0x10000` was set, it stores a flag and clears that style bit. |
| `EM_SETLIMITTEXT` `0xC5` | NewEdit stores max length and enforces it. Skirmish init sends `0xC5, 0x13` to edit `0x6A0`. |
| `WM_SETFOCUS` `0x07` / `WM_KILLFOCUS` `0x08` | Selection/caret timer management and invalidation. |
| `WM_TIMER` `0x113` | Toggles caret visibility and invalidates. |
| `WM_PAINT` `0x0F`, `WM_ERASEBKGND` `0x14` | Draws cached background, primitive frame, and text/caret. |
| `WM_CHAR` `0x102` | Handles Enter/Tab specially and filters inserted characters. |
| `0x4B2`, `0x4B4`, `0x4B3`, `0x4B5` | Owner-draw text set/get helpers using narrow/wide conversions. |

Assets/composition:

- No PCX asset is directly referenced by these edit callbacks.
- The frame is primitive via `FUN_006208F0`.
- Text, cursor, selection, password masking, and scroll offset go through
  `FUN_00623880` and `FUN_006211D0`.

Fallback behavior:

- Unknown messages call the previous WndProc.
- If owner-draw state is absent, the callback falls back to the previous WndProc.

### 1.7 ListBox callback `0x00618D40`

Applies to owner-drawn list boxes and the custom `ComboDropWin` popup created
by the ComboBox callback.

Important messages:

| Message | Behavior |
|---:|---|
| `WM_PAINT` `0x0F` | Draws list background, selection fills, item text, optional per-item custom columns, and validates. |
| `LB_GETCOUNT` `0x18B`, `LB_GETTOPINDEX` `0x18E`, `LB_GETITEMRECT` `0x198`, item-data messages `0x199/0x19A` | Wraps Win32 list behavior and owner-draw item records. |
| `WM_SIZE` `0x05` | Repositions/updates child scrollbar if present. |
| `WM_DESTROY` `0x82` / reset `0x184` | Frees owner-draw item arrays and scrollbar/control state. |
| Custom `0x4E8` | Hit-tests a client point and returns the list item index or `-1`. |
| Custom `0x4EF` | Returns the child scrollbar HWND/state. |

Assets/composition:

- The list body itself is primitive/text drawing. It uses `FUN_006208F0` for the
  frame, surface fill calls for selected item rectangles, and `FUN_006211D0`
  for text.
- When item count exceeds visible rows, it creates a child `Scrollbar` window,
  calls `FUN_0060F9A0` on that scrollbar, and sends `0xE9` to configure it.
- Therefore list dropdowns indirectly use the scrollbar PCXs below when the
  popup needs scrolling.

Fallback behavior:

- Unknown messages call the previous WndProc when `cStack0000000F` remains set.
- Without enough items to scroll, no scrollbar child is created and no scrollbar
  PCX is used.

### 1.8 ScrollBar callback `0x0061C690`

Used by dropdown/list scrollbars. Skirmish combos can reach this when a combo
popup has more entries than visible rows.

Messages handled:

| Message | Behavior |
|---:|---|
| `WM_PAINT` `0x0F` | Draws vertical scrollbar, thumb, arrows, validates. |
| `SBM_SETSCROLLINFO`-like custom `0xE9` | Reads range/page/top values from the caller's struct and stores max/current values. |
| `0xE0` | Sets position if within `1..max`. |
| `0xE1` | Returns current position. |
| `0xE2` | Sets max/range and clamps current position. |
| `WM_LBUTTONDOWN` `0x201`, `WM_LBUTTONDBLCLK` `0x203` | Captures mouse, changes value, detects arrow/page/thumb areas. |
| `WM_MOUSEMOVE` `0x200`, `WM_LBUTTONUP` `0x202`, `WM_TIMER` `0x113` | Drag/repeat handling; repeat timer is `0x19` ms after initial `500` ms capture timer. |
| Custom `0x4D2` | Sets a parent-capture flag. |
| Custom `0x4F1` | Sets grey/alternate art flag at state byte `+0xCD`. |

Assets:

| Role | Normal PCX | Grey PCX |
|---|---|---|
| Thumb top | `sbgript.pcx` | `gsbgript.pcx` |
| Thumb middle | `sbgripm.pcx` | `gsbgripm.pcx` |
| Thumb bottom | `sbgripb.pcx` | `gsbgripb.pcx` |
| Up arrow released/pressed | `uparrowr.pcx` / `uparrowp.pcx` | `guparrowr.pcx` / `guparrowp.pcx` |
| Down arrow released/pressed | `dnarrowr.pcx` / `dnarrowp.pcx` | `gdnarrowr.pcx` / `gdnarrowp.pcx` |

Placement/composition:

- It reserves `0x16` pixels at top and bottom for arrow buttons.
- The thumb minimum height is clamped to at least `0x0E`.
- Track span is `(client_height - 0x2C) - thumb_height`, then clamped to at
  least `1`.
- Thumb Y is `0x16 + track_span * current / max` unless dragging, where cursor Y
  minus half the thumb height is clamped to `[0x16, bottom - 0x16 - thumb]`.
- Thumb middle is tiled with `FUN_006BA3E0`; top and bottom are direct-blitted.
- Arrows are drawn after backing-store restore and primitive frame lines.

Fallback behavior:

- `FUN_00620720` safely skips arrow draw if the arrow PCX lookup returns null.
- The thumb code assumes the `sbgrip*`/`gsbgrip*` PCXs are available once chosen.

### 1.9 Trackbar callback `0x0061D950`

Applies to Skirmish sliders:

- game speed `0x529`;
- credits `0x511`;
- unit count `0x50C`.

Messages handled:

| Message | Behavior |
|---:|---|
| `WM_PAINT` `0x0F` | Draws cached background, optional value plaque, track lines, grip, optional numeric text. |
| `TBM_GETPOS` `0x400` | Returns current value rounded to the configured step. |
| `TBM_SETPOS`-like `0x405` | Sets position, clamps to range. |
| `TBM_SETRANGE`-like `0x406` | Sets min/max from low/high words and clamps current. |
| Custom `0x4AB` | Sets step size. |
| Custom `0x4AC` | Enables/disables numeric value display. |
| Custom `0x4AE` | Sets a no-sound/suppress flag. |
| `WM_LBUTTONDOWN` `0x201`, `WM_LBUTTONDBLCLK` `0x203`, `WM_MOUSEMOVE` `0x200`, `WM_LBUTTONUP` `0x202` | Capture/drag/update. |

Assets:

- `trakgrip.pcx` for the draggable grip.
- `trofm.pcx`, `trofl.pcx`, `trofr.pcx` for the optional right-side value plaque
  when numeric display is enabled.

Placement/composition:

- The active track width is `(client_width - value_display_width) - 0x0D`.
- If numeric display is disabled, value display width is `0`; otherwise it uses
  `0x32`.
- The grip is 12 pixels wide in the position math: mouse X minus `6`, clamp to
  `[1, right - value_display_width - 0x0C]`.
- Value is quantized to the configured step:
  `((raw + min) / step) * step - min`.
- `trofm.pcx` is tiled by `FUN_006BA3E0`; `trofl.pcx` and `trofr.pcx` are cap
  pieces.
- `trakgrip.pcx` is direct-blitted over the track.
- Disabled style applies alpha with `DAT_00AC4898`.

Fallback behavior:

- The trackbar code assumes `trof*` and `trakgrip.pcx` exist if the corresponding
  branch executes.

### 1.10 Checkbox callback `0x006163A0`

Applies to Skirmish checkboxes:

- Short Game `0x54E`;
- MCV Repacks `0x693`;
- Crates Appear `0x696`;
- Super Weapons Allowed `0x69A`;
- Build Off Ally `0x69D`.

Messages handled:

| Message | Behavior |
|---:|---|
| `0x497` | Reads original button check state through previous WndProc message `0xF0` and stores it in state `+0xE8`. |
| `BM_GETCHECK`-like `0xF0` | Returns stored checked state. |
| `BM_SETCHECK`-like `0xF1` | Sets stored checked state and invalidates. Skirmish init sends this for all five options. |
| `WM_LBUTTONDOWN` `0x201`, `WM_LBUTTONDBLCLK` `0x203` | Toggles only when click X `< 0x12` and Y `< 0x12`; plays click sound; sends parent `WM_COMMAND` with checked state in high word. |
| `WM_PAINT` `0x0F` | Draws checkbox icon and label text. |
| `0x4E5`, `0x4E6`, `0x4E7` | Set/query two variant flags that switch the PCX family. |

Default Skirmish checkbox assets:

- With both variant flags clear, unchecked uses `cue_i.pcx`.
- With both variant flags clear, checked uses `cce_i.pcx`.
- Variant combinations use `cce_ir.pcx`, `cce_il.pcx`, and `cce_i.pcx`.
- The icon destination rect is exactly `0x12 x 0x12`.
- Label text starts after the icon with X offset `icon_height + 0x1A`.

Important correction:

- The `bst_uckg.pcx`, `bst_chkg.pcx`, `bst_uchk.pcx`, and `bst_chkd.pcx` files
  are preloaded in `FUN_0061F210`, but this callback body does not use them for
  the Skirmish checkbox styles in resource `0x102`.

Fallback behavior:

- The default checkbox paint path dereferences the PCX returned by
  `FUN_006BA140` without a robust missing-asset fallback.
- Unknown messages call the previous WndProc.

### 1.11 Button/radio variant callback `0x00616980`

Assigned when `Button` style low bits satisfy `(style & 9) == 9`, after the
earlier style checks fail.

Messages:

- `WM_PAINT` uses the same `b%c%c_li%d.pcx`, `b%c%c_mi%d.pcx`,
  `b%c%c_ri%d.pcx` cap/middle/cap composition pattern as the main button path.
- `BM_GETCHECK`-like `0xF0` and `BM_SETCHECK`-like `0xF1` read/write the stored
  state.
- `WM_LBUTTONDOWN`/`WM_LBUTTONDBLCLK` set checked state to `1` if it was `0`,
  play the click sound, invalidate, then fall through to previous WndProc.
- `WM_LBUTTONUP` `0x202` is wrapped with `LockWindowUpdate(parent)` while calling
  the previous WndProc.
- `0x497` initializes state by querying the previous WndProc with `0xF0`.

Skirmish relevance:

- None of the requested offline Skirmish resource `0x102` controls use this
  variant. It is included here because it was one of the requested callback
  bodies.

### 1.12 Frame/group variant callback `0x0061E700`

Assigned when `Button` style low bits satisfy `(style & 7) == 7`.

Messages:

- `WM_PAINT` draws an etched/framed text area with primitive line drawing and
  text via `FUN_006211D0`.
- `WM_ERASEBKGND` `0x14` returns `1`.
- Other messages, including `0x85`, fall back to previous WndProc.

Assets:

- No PCX/SHP assets are referenced by this callback body.
- It uses color globals `DAT_00AC1B94` and `DAT_00AC1B98`, converted to the
  active 16-bit display format, then draws two nested line passes.

Skirmish relevance:

- None of the requested offline Skirmish controls use this variant.

## 2. Offline Skirmish Dialog `0x102` Asset Mapping

| Skirmish controls | Callback | Verified assets / draw path |
|---|---|---|
| Map thumbnail static `0x468` | Static hook exists, but Skirmish dialog paint handles preview | `DAT_00AC1154` preview object, `DrawStartPositions @ 0x00640710`, `STARTBUT.SHP`; static callback is not the primary thumbnail renderer. |
| Flag statics `0x6DA..0x6E1` | `0x006153E0` | `usai.pcx`, `japi.pcx`, `frai.pcx`, `geri.pcx`, `gbri.pcx`, `djbi.pcx`, `arbi.pcx`, `lati.pcx`, `rusi.pcx`, `yrii.pcx`, `obsi.pcx`, `rani.pcx`; centered/cropped transparent blit. |
| Start Game `0x617`, Choose Map `0x5AA`, Back `0x5C0` | `0x00612B70` | `bue_li24/30.pcx`, `bue_mi24/30.pcx`, `bue_ri24/30.pcx` unpressed; `bde_li24/30.pcx`, `bde_mi24/30.pcx`, `bde_ri24/30.pcx` pressed. Middle tiled. |
| Player/side/color/start/team combos | `0x00617250`, popup/list via `0x00618D40`, scrollbars via `0x0061C690` | Primitive frame/text; arrow PCXs `dnarrowr/p`, `uparrowr/p`, grey `g*` variants; list popup primitive/text; scrollbar uses `sbgrip*`/`gsbgrip*` if needed. |
| Player name edit `0x6A0` | `0x00614190` | Primitive frame/text/caret; no direct PCX. |
| Sliders `0x529`, `0x511`, `0x50C` | `0x0061D950` | `trakgrip.pcx`; optional value plaque `trofl.pcx`, `trofm.pcx`, `trofr.pcx`; primitive track and text. |
| Checkboxes `0x54E`, `0x693`, `0x696`, `0x69A`, `0x69D` | `0x006163A0` | Default unchecked `cue_i.pcx`; default checked `cce_i.pcx`; variant flags can use `cce_il.pcx` / `cce_ir.pcx`, but Skirmish init only sends `0xF1` check-state messages. |

Assets investigated but not verified as used by offline Skirmish controls:

- `BTN-MINS.SHP` / `BTN-PLUS.SHP`: no references in the recovered owner-draw
  callback bodies.
- `bst_*`: preloaded, but not used by Skirmish checkbox style `0x03`.
- `bud_*`: preloaded; not shown on the default enabled/disabled main button
  path recovered from `0x00612B70`.
- `dlgsysa.pcx`, `dlgsysi.pcx`, `dbak6440.pcx`: preloaded/background-class
  assets, but the per-control callbacks here mostly copy cached parent
  backgrounds and do not directly name these for the listed controls.
- `number0.pcx..number9.pcx`: preloaded, but not referenced in these Skirmish
  owner-draw callback bodies. Start-position numbers on the map thumbnail are
  text drawn after `STARTBUT.SHP`, per the prior map-preview reports.

## 3. Inferred Asset Roles

These are inferences from verified code paths and asset names/dimensions, not
independent screenshot proof:

- `bue_*` means button-up enabled, `bde_*` means button-down enabled. The first
  format character is verified as `'u'`/`'d'`, and the second as `'e'`.
- The 24/30 suffix is a height family selected from the actual Win32 client
  height after dialog-unit conversion. The code's thresholds are verified, but
  the exact resource DLU-to-pixel result at each shell resolution still needs a
  screenshot or live Win32 measurement.
- `rani.pcx` is the random-side flag/icon for item data `-2`.
- `obsi.pcx` is observer for item data `-3`.
- Color combo swatches are likely enabled through ComboBox message `0x4DD` and
  per-item color slots at state `+0x44`, but the exact Skirmish color-population
  helper was not fully re-decompiled in this pass.

## 4. Unresolved / Open Questions

1. Exact 24-vs-30 button family used by the offline Skirmish 23-DLU buttons at
   each shell resolution remains open. The binary thresholds are known; the live
   pixel client rect still needs measurement.
2. The PCX decode/palette conversion below `CDFileClass__Constructor` /
   `FUN_006BA140` was not decompiled here. The callbacks prove use of 16-bit
   converted surfaces, not the full PCX palette loader internals.
3. Exact color-combo swatch population and color ordering need a focused trace
   through `FUN_004E4820`, `FUN_004E4C20`, and `FUN_004E4E20` if pixel-perfect
   color dropdown rows are required.
4. A live retail screenshot comparison is still needed to confirm the final
   shell-resolution scaling and whether disabled states ever force alternate
   preloaded `bud_*` assets outside the normal Skirmish path.
5. `bst_*`, `bud_*`, and `number*.pcx` remain verified preload assets only for
   this scope. Their use may belong to other shell dialogs or modes.

## Sources

Ghidra functions decompiled / disassembled:

- `FUN_0060F9A0` - owner-draw hook setup and callback assignment.
- `FUN_0061F210` - owner-draw PCX preload pool.
- `OwnerDraw_Button_00612B70` - main owner-draw button callback.
- `OwnerDraw_ComboBox_00617250` - combo callback and custom dropdown creation.
- `OwnerDraw_Static_006153E0` - static/text/image/movie callback.
- `OwnerDraw_Edit_00614190` - `Edit` callback.
- `OwnerDraw_NewEdit_00614B30` - `NewEdit` callback.
- `OwnerDraw_ListBox_00618D40` - real owner-drawn `LISTBOX` callback, including Choose Map `0x6EB`/`0x553`; standard combo popup rows are owned by `ComboDropWin` (`0x0060D540..0x0060F311`).
- `OwnerDraw_ScrollBar_0061C690` - scrollbar callback.
- `OwnerDraw_Trackbar_0061D950` - trackbar callback.
- `OwnerDraw_Checkbox_006163A0` - checkbox callback.
- `OwnerDraw_RadioVariant_00616980` - button/radio variant callback.
- `OwnerDraw_ButtonVariant_0061E700` - frame/group button variant callback.
- `FUN_00620720`, `FUN_006208F0`, `FUN_00621040`, `FUN_006211D0`,
  `FUN_00623880`, `FUN_006BA140`, `FUN_006BA3E0`, `FUN_006BA580` - shared draw,
  text, lookup, tile, and transparent-blit helpers.
- `FUN_006AE6E0` - offline Skirmish dialog initialization.
- `FUN_006ACEE0` - offline Skirmish command handler.
- `FUN_006ACD60` - team/start enable refresh helper.
- `FUN_004E3B90`, `FUN_004E3A00`, `FUN_004E3F70`, `FUN_004E3560`,
  `FUN_00603D30` - side combo population, side selection, and flag static PCX
  update path.

Prior reports referenced:

- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`
