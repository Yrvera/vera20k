# Skirmish 0x102 Top Preview Chrome - SDTP Frame 1 / SDMPBTN Frame 0 Ghidra Report

Date: 2026-05-23

Investigation mode: exhaustive-slice

Scope: the standard offline Yuri's Revenge Skirmish dialog `0x102` right-panel
top preview head and lower map-title panel chrome after the common
`RightPanel__Draw` / `Background_Overlay` pass.

Non-scope: the preview bitmap decoder, `STARTBUT.SHP` marker projection, Choose
Map dialog `0x6B`, and shell transition animation frames.

Active in YR: Yes, for standard offline Skirmish dialog `0x102`.

## 1. Overview

The standard Skirmish right panel is not just `RightPanel__Draw`'s base stack.
After `RightPanel__Draw` and `Background_Overlay`, `WM_PAINT_Handler` checks
per-dialog flags and draws two additional chrome pieces for `0x102`:

- `SDTP.SHP` frame `1` over the top panel rect, creating the clean map-preview
  head.
- `SDMPBTN.SHP` frame `0` at `DAT_00B0FC14`, creating the lower black
  Battle/map-name panel.

This corrects older wording that treated `SDMPBTN.SHP` as merely loaded or only
"not preview backing." It is still not the preview bitmap itself, but it is
verified visible right-panel chrome for `0x102`.

## 2. Verified Binary Findings

### 2.1 `0x102` sets the top-highlight flag

`FUN_0060CAF0 @ 0x0060CAF0` includes dialog id `0x102` in its allow-list and
writes byte `+0xD9 = 1`.

In `WM_PAINT_Handler`, the record pointer used for flag checks is already offset
by `+4`, so the assembly reads `[EBX + 0xD5]`.

Assembly evidence around `0x00622120`:

```asm
00622120  MOV AL,byte ptr [EBX + 0xd5]
00622126  TEST AL,AL
00622128  JZ 0x00622135
0062212a  LEA EDX,[ESP + 0x28]
0062212e  MOV ECX,ESI
00622130  CALL 0x0072e8c0
```

### 2.2 `Sidebar_TopHighlight` draws `SDTP.SHP` frame 1

`Sidebar_TopHighlight @ 0x0072E8C0` decompiles to:

```text
local_8 = *DAT_00B0FC20;
local_4 = DAT_00B0FC20[1];
CC_Draw_Shape(g_SDTP_SHP, 1, &local_8, param_2, 0x400, ...);
```

`DAT_00B0FC20` is the normal right-panel top rect computed from `SDTP.SHP`.
At `800x600`, this is `(632,0,168,199)`.

### 2.3 `0x102` sets the SDMPBTN/minimap-button flag

`FUN_0060C930 @ 0x0060C930` includes dialog id `0x102` in its allow-list and
writes byte `+0xDA = 1`.

In `WM_PAINT_Handler`, the same `+4` record-pointer shift makes this read appear
as `[EBX + 0xD6]`.

Assembly evidence:

```asm
00622135  MOV AL,byte ptr [EBX + 0xd6]
0062213b  TEST AL,AL
0062213d  JZ 0x0062214a
0062213f  LEA EDX,[ESP + 0x28]
00622143  MOV ECX,ESI
00622145  CALL 0x0072e860
```

### 2.4 `Minimap_Button` draws `SDMPBTN.SHP` frame 0

`Minimap_Button @ 0x0072E860` decompiles to:

```text
local_8 = *DAT_00B0FC14;
local_4 = DAT_00B0FC14[1];
CC_Draw_Shape(DAT_00B0F9DC, 0, &local_8, param_2, 0x400, ...);
```

`DAT_00B0F9DC` is loaded from the right-panel SHP loader table entry
`SDMPBTN.SHP`.

`RightPanel__ComputeLayoutRects @ 0x0072EC70` computes `DAT_00B0FC14` from
`SDMPBTN.SHP` dimensions:

```text
x = right_edge - SDMPBTN_width
y = SDBTNBKGD_y + SDBTNBKGD_h - SDMPBTN_h
w = SDMPBTN_width
h = SDMPBTN_height
```

With retail `SDMPBTN.SHP = 156x84`, at `800x600`:

- `right_edge = 800`
- `SDBTNBKGD_y = SDTP_h = 199`
- `SDBTNBKGD_h = 42`
- rect = `(644,157,156,84)`

This matches the lower black panel under the preview image and above/around the
Battle/map-name text, not the preview bitmap itself.

## 3. Corrected Active 0x102 Chrome Order

For standard offline Skirmish `0x102`, the visible right-panel shell composition
contains:

1. `RightPanel__Draw`: `SDTP.SHP` frame `0`, repeated `SDBTNBKGD.SHP`, optional
   `SDBTNANM.SHP` frame `10` depending on its separate gate, `SDBTM.SHP`, and
   `LWSCRNS/LWSCRNL`.
2. `Background_Overlay`: Skirmish parent background.
3. `Sidebar_TopHighlight`: `SDTP.SHP` frame `1` at `DAT_00B0FC20`.
4. `Minimap_Button`: `SDMPBTN.SHP` frame `0` at `DAT_00B0FC14`.
5. Skirmish dialog `WM_PAINT`: preview child `0x468` / preview helper and
   `DrawStartPositions` when preview data is present.

The preview bitmap remains separate. Do not fit `SDMPBTN.SHP` into the preview
rect; draw it at `DAT_00B0FC14`.

## 4. Current Rust Implementation Status

Current Rust has been updated to match this slice:

- `src/render/skirmish_shell_chrome.rs` loads `SDTP.SHP` frame `1` as
  `right_panel_top_highlight_sdtp_frame1`.
- `src/app_skirmish_shell_render.rs` draws `right_panel_top_highlight_sdtp_frame1`
  after parent background.
- `src/app_skirmish_shell_render.rs` draws `SDMPBTN.SHP` frame `0` using
  `sdmpbtn_rect`, not `layout.map_preview`.
- Unit coverage includes `sdmpbtn_rect_matches_verified_minimap_button_position`.

## 5. Stale Docs / Replacement Wording

Replace claims of the form:

> `SDMPBTN.SHP` is loaded but not established as standard Skirmish first-paint surface.

with:

> `SDMPBTN.SHP` is not the preview bitmap/backing, but it is verified visible
> Skirmish `0x102` right-panel chrome. `FUN_0060C930` sets the `0x102` flag,
> `WM_PAINT_Handler` calls `Minimap_Button @ 0x0072E860`, and that draws
> `SDMPBTN.SHP` frame `0` at `DAT_00B0FC14`.

Replace claims that describe the top panel only as `SDTP.SHP` frame `0` with:

> `RightPanel__Draw` first draws `SDTP.SHP` frame `0`; standard Skirmish `0x102`
> then draws `SDTP.SHP` frame `1` through `Sidebar_TopHighlight @ 0x0072E8C0`
> because `FUN_0060CAF0` sets the top-highlight flag for `0x102`.

Keep claims of the form:

> Do not use `SDMPBTN.SHP` as the preview bitmap/backing.

They remain correct as long as they do not imply `SDMPBTN.SHP` is absent from
the right panel.

## Sources

- `FUN_0060CAF0 @ 0x0060CAF0`
- `FUN_0060C930 @ 0x0060C930`
- `WM_PAINT_Handler @ 0x00621E90`; assembly `0x00622120..0x00622145`
- `Sidebar_TopHighlight @ 0x0072E8C0`
- `Minimap_Button @ 0x0072E860`
- `RightPanel__ComputeLayoutRects @ 0x0072EC70`
- asset dump: `SDTP.SHP` 168x199, 2 frames; `SDMPBTN.SHP` 156x84, 7 frames
