# Right-Panel Draw Stack and Layout Rects — Ghidra Report

Date: 2026-05-19

Scope: Three functions that compose the right-panel chrome on the initial main menu
dialog `0xE2` and all other common-shell dialogs (mode `1`):

- `RightPanel__Draw @ 0x0072E450`
- `RightPanel__ComputeLayoutRects @ 0x0072EC70`
- `Background_Overlay @ 0x0072E730`

Primary question answered: which SHPs are drawn, at what pixel positions, with which
palette, in what z-order relative to the parent background, and how the central tiled
section row count is computed.

Active in YR: **Yes** for all three functions — all are called by
`WM_PAINT_Handler @ 0x00621E90`, which is reached from `FUN_00622B50` for every
common-shell mode-`1` dialog including `0xE2`.  
Call chain confirmed: `FUN_00531CC0` → dialog `0xE2` → `FUN_00622B50` WM_PAINT →
`WM_PAINT_Handler @ 0x00621E90` → `RightPanel__Draw`, then `Background_Overlay`.

Parent reports consulted (do not re-investigate):

- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_BACKGROUND_OVERLAY_PLACEMENT_FOLLOWUP_GHIDRA_REPORT.md`

No Rust code was modified.

---

## 1. Lazy-Init and Guard

`RightPanel__Draw` reads the global flag `DAT_00B0FBE0`. If zero (not yet initialized),
it calls:

1. `FUN_00534E50` — verified: reloads the CDFileClass/SHP asset files.
2. `Sidebar_RightPanel_SHP_Loading @ 0x0072EB50` — fills all 10 SHP globals from the
   CDFileClass constructor list.
3. `CDFileClass__Constructor` for the three palette/convert globals at `0x00B0FBCC`,
   `0x00B0FBD4`, `0x00B0FBDC` (SHELL.PAL, SHELL2.PAL, SDBTNANM.PAL respectively).
4. `RightPanel__ComputeLayoutRects()` — computes all layout rect globals.
5. Sets `DAT_00B0FBE0 = 1`.

Subsequent calls skip steps 1–5. `FUN_0072DFB0` (the teardown/reload path) clears
`DAT_00B0FBE0` back to zero and frees all rect globals before re-running the same
init sequence.

Active in YR: Yes — `DAT_00B0FBE0` is the one-shot init gate shared across all
common-shell dialogs.

---

## 2. SHP Global Address Table

Populated by `Sidebar_RightPanel_SHP_Loading @ 0x0072EB50` and the CDFileClass
constructor loop. Evidence: disassembly of `FUN_0072DFB0`, string block
`0x00845104–0x00845188`, pointer table `0x00844CD4–0x00844CF8`.

| Global | Asset | Load-table pointer |
|---|---|---|
| `g_SDBTNANM_SHP` (named) | `SDBTNANM.SHP` | `0x00844CD4` → string `0x00845178` |
| `DAT_00B0F9DC` | `SDMPBTN.SHP` | `0x00844CD8` → string `0x0084516C` |
| `DAT_00B0FAC0` | `SDWRNTMP.SHP` | `0x00844CDC` → string `0x0084515C` |
| `DAT_00B0FB50` | `MNSCRNS.SHP` | `0x00844CE0` → string `0x00845150` |
| `DAT_00B0FA04` | `MNSCRNL.SHP` | `0x00844CE4` → string `0x00845144` |
| `g_SDTP_SHP` (named) | `SDTP.SHP` | `0x00844CE8` → string `0x00845138` |
| `g_SDBTNBKGD_SHP` (named) | `SDBTNBKGD.SHP` | `0x00844CEC` → string `0x00845128` |
| `DAT_00B0FA38` | `SDBTM.SHP` | `0x00844CF0` → string `0x0084511C` |
| `DAT_00B0FAE8` | `LWSCRNS.SHP` | `0x00844CF4` → string `0x00845110` |
| `DAT_00B0FA54` | `LWSCRNL.SHP` | `0x00844CF8` → string `0x00845104` |

Confidence: High — string addresses verified in prior sessions; load sequence verified
in this session's decompile of `FUN_0072DFB0`.

---

## 3. Palette / Convert Table

Evidence: PAL string block `0x00845438–0x00845524`, pointer table `0x00844BE4–0x00844BEC`,
decompile of `RightPanel__Draw @ 0x0072E450` (lazy-init branch).

| Global (convert) | PAL file | Load address |
|---|---|---|
| `DAT_00B0FBCC` | `SHELL.PAL` | `0x00844BE4` |
| `DAT_00B0FBD4` | `SHELL2.PAL` | `0x00844BE8` |
| `DAT_00B0FBDC` | `SDBTNANM.PAL` | `0x00844BEC` |

`FUN_0072ADE0` allocates a raw 256-entry palette, left-shifts 6-bit channel values by
2, and builds a `ConvertClass` against `DAT_00887310` (the main drawing surface).

Confidence: High — from decompile of `RightPanel__Draw` lazy-init and prior verified
assembly (SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP).

---

## 4. `RightPanel__ComputeLayoutRects` — Full Layout Algorithm

**Signature** (fastcall): `void RightPanel__ComputeLayoutRects(int screen_w, int screen_h)`

Both parameters are loaded from globals just before the call:

```
0072e18d: MOV ECX, [0x008a00a4]   ; g_ScreenWidth  → param_1 / screen_w
0072e187: MOV EDX, [0x008a00a8]   ; g_ScreenHeight → param_2 / screen_h
0072e193: CALL 0x0072ec70
```

Verified from disassembly context at call site in `FUN_0072E1B0`.

### 4.1 Shell origin / centering offset

```c
local_c = 0;           // x_offset
local_10 = screen_w;   // right edge = screen_w (full width)
if (screen_w > 0x3ff) {                       // > 1023
    local_c  = (screen_w - 800) / 2;
    local_10 = screen_w - local_c;            // = (screen_w + 800) / 2
}
iVar6 = 0;             // y_offset
local_4 = screen_h;
local_8 = screen_h;
if (screen_h > 0x2ff) {                       // > 767
    iVar6   = (screen_h - 600) / 2;
    local_4 = screen_h - iVar6;               // = (screen_h + 600) / 2
    local_8 = screen_h + iVar6 * -2;          // = screen_h - 2 * iVar6 = 600 (at 768)
}
```

Key values at standard resolutions:

| Resolution | `local_c` (x_offset) | `iVar6` (y_offset) | `local_10` (right_edge) | `local_8` (usable_height) |
|---:|---:|---:|---:|---:|
| 640×480 | 0 | 0 | 640 | 480 |
| 800×600 | 0 | 0 | 800 | 600 |
| 1024×768 | 112 | 84 | 912 | 600 |

Confidence: High — directly from decompile of `0x0072EC70`.

### 4.2 Parent overlay rect (`DAT_00B0FC1C`)

Dimensions come from `MNSCRNS.SHP` at width 640, `MNSCRNL.SHP` otherwise (for width),
and same independent test for height:

```c
// Width dimension:
if (screen_w != 640)  sVar1 = *(short*)(DAT_00b0fa04 + 2);   // MNSCRNL width
else                  sVar1 = *(short*)(DAT_00b0fb50 + 2);   // MNSCRNS width

// Height dimension:
if (screen_h != 480)  sVar2 = *(short*)(DAT_00b0fa04 + 4);   // MNSCRNL height
else                  sVar2 = *(short*)(DAT_00b0fb50 + 4);   // MNSCRNS height

DAT_00b0fc1c = new int[4];
DAT_00b0fc1c[0] = local_c;       // x = x_offset (0 at ≤1023 wide)
DAT_00b0fc1c[1] = iVar6;         // y = y_offset (0 at ≤767 tall)
DAT_00b0fc1c[2] = sVar1;         // w = SHP width
DAT_00b0fc1c[3] = sVar2;         // h = SHP height
```

Width and height thresholds are independent: a non-standard 640×600 resolution would
use MNSCRNS width but MNSCRNL height.

### 4.3 SDTP top-cap rect (`DAT_00B0FC20`)

```c
iVar4 = *(short*)(g_SDTP_SHP + 2);   // SDTP pixel width
sVar1 = *(short*)(g_SDTP_SHP + 4);   // SDTP pixel height
iVar5 = local_10 - iVar4;            // x = right_edge - SDTP_width  (right-aligned)

DAT_00b0fc20[0] = iVar5;             // x (right-aligned to right_edge)
DAT_00b0fc20[1] = iVar6;             // y = y_offset
DAT_00b0fc20[2] = iVar4;             // w = SDTP_width
DAT_00b0fc20[3] = sVar1;             // h = SDTP_height
```

Result at 800×600: x = 800 − SDTP_width, y = 0. SDTP is right-aligned to the
`right_edge` of the 800-wide shell band.

### 4.4 SDBTNBKGD tiled rect (`DAT_00B0FC24`)

Immediately below SDTP, same x as SDTP:

```c
sVar1 = *(short*)(g_SDBTNBKGD_SHP + 4);  // tile height

DAT_00b0fc24[0] = iVar5;                  // x = same right-aligned x as SDTP
DAT_00b0fc24[1] = DAT_00b0fc20[3] + iVar6; // y = SDTP_height + y_offset
DAT_00b0fc24[2] = iVar4;                  // w = SDTP_width (same column)
DAT_00b0fc24[3] = sVar1;                  // h = per-tile height (used as y-increment)
```

### 4.5 Tile-row count (`DAT_00B0FA20`) — the central tiled section

```c
DAT_00b0fa20 = (local_8 - DAT_00b0fc20[3]) / DAT_00b0fc24[3];
//           = (usable_height - SDTP_height) / SDBTNBKGD_tile_height
```

This is integer division (floor). `local_8` is the usable vertical extent of the
shell band (`screen_h` at ≤767, or `600` at `1024×768`). The top-cap SDTP height is
subtracted first; the remainder is divided by one tile's height.

At 800×600 with retail SDTP height of approx 48px and SDBTNBKGD tile height of approx
30px, this is floor((600 − 48) / 30) = 18. The actual pixel values come from the
retail SHP headers; the formula is verified.

Confidence: High — directly from decompile at formula site `DAT_00b0fa20 = (local_8 - DAT_00b0fc20[3]) / DAT_00b0fc24[3]`.

### 4.6 SDBTNANM overlay rect (`DAT_00B0FC10`)

Coincident with SDBTNBKGD column but horizontally inset to SDBTNANM's own width:

```c
sVar1 = *(short*)(g_SDBTNANM_SHP + 2);   // SDBTNANM width
sVar2 = *(short*)(g_SDBTNANM_SHP + 4);   // SDBTNANM height

DAT_00b0fc10[0] = (DAT_00b0fc24[2] - sVar1) + *DAT_00b0fc24;
  // x = (SDBTNBKGD_width - SDBTNANM_width) + SDBTNBKGD_x  (right-aligns within column)
DAT_00b0fc10[1] = DAT_00b0fc24[1];  // y = same start y as SDBTNBKGD
DAT_00b0fc10[2] = sVar1;
DAT_00b0fc10[3] = sVar2;
```

### 4.7 SDBTM bottom-cap rect (`DAT_00B0FC28`)

Positioned immediately after all tile rows:

```c
iVar5_btm = DAT_00b0fc24[3] * DAT_00b0fa20;   // total tile band height
sVar1 = *(short*)(DAT_00b0fa38 + 2);           // SDBTM pixel width (DAT_00B0FA38 = SDBTM)

DAT_00b0fc28[1] = DAT_00b0fc24[1] + iVar5_btm; // y = SDBTNBKGD_y + tile_band_height
DAT_00b0fc28[0] = local_10 - sVar1;            // x = right_edge - SDBTM_width
DAT_00b0fc28[2] = sVar1;                       // w = SDBTM_width
DAT_00b0fc28[3] = local_8 - (DAT_00b0fc20[3] + iVar5_btm);
  // h = usable_height - (SDTP_height + tile_band_height) = residual to fill bottom
```

The SDBTM height rect value is computed as the remaining pixel gap, not read from the
SHP header directly. This means SDBTM is drawn to fill exactly the remaining space.

### 4.8 Lower/side strip rect (`DAT_00B0FC2C`) — LWSCRNL / LWSCRNS

Width and height chosen independently, same pattern as parent overlay:

```c
if (screen_w != 640)  sVar1 = *(short*)(DAT_00b0fa54 + 2);  // LWSCRNL width
else                  sVar1 = *(short*)(DAT_00b0fae8 + 2);  // LWSCRNS width

if (screen_h != 480)  sVar2 = *(short*)(DAT_00b0fa54 + 4);  // LWSCRNL height
else                  sVar2 = *(short*)(DAT_00b0fae8 + 4);  // LWSCRNS height

DAT_00b0fc2c[0] = local_c;            // x = x_offset (left edge of shell band)
DAT_00b0fc2c[1] = local_4 - sVar2;   // y = bottom of visible area - strip height
DAT_00b0fc2c[2] = sVar1;             // w
DAT_00b0fc2c[3] = sVar2;             // h
```

Key difference from right-panel assets: the lower strip is positioned at `x = local_c`
(the left edge of the centered shell band), not right-aligned. At 800×600 this is `x = 0`.

Confidence: High — directly from decompile.

---

## 5. `RightPanel__Draw` — Full Draw Order

**Signature** (fastcall): `void RightPanel__Draw(undefined4 param_1, undefined4* param_2, char param_3)`

Called from `WM_PAINT_Handler` as:
```c
RightPanel__Draw((char)piVar9[0x35] == '\0');
```
`piVar9` is the dialog window-extra record pointer (`piVar9 = record_ptr + 1`).
`piVar9[0x35]` is byte at window-extra offset `+0xD4` (int-slot 0x35 × 4).

The `param_2` rect pointer carries the offscreen surface bounds passed from the handler.
`param_3` is the `SDBTNANM` overlay enable flag: `'\0'` = draw the overlay.

Clip guard (in both `RightPanel__Draw` and `Background_Overlay`):

```c
if (local_8 > 800) local_8 = (local_8 - 800) / 2 + 800;
if (local_4 > 600) local_4 = (local_4 - 600) / 2 + 600;
```

This clamps the right/bottom of the draw region to the centered 800×600 band.

### Draw order table

All calls use `CC_Draw_Shape @ 0x004AED70` with flags `0x400` and constant args
`0, 0, 0, 1000, 0, 0, 0, 0, 0`. Flag `0x400` does NOT include `0x200` (center),
so all sprites are positioned at the rect's x,y without centering adjustment.

| Order | SHP global | Asset | Frame | Position rect | Convert / PAL | Condition |
|---:|---|---|---:|---|---|---|
| 1 | `g_SDTP_SHP` | `SDTP.SHP` | 0 | `DAT_00B0FC20` | `DAT_00B0FBCC` / SHELL.PAL | Always |
| 2 | `g_SDBTNBKGD_SHP` | `SDBTNBKGD.SHP` | 0 | `DAT_00B0FC24`, y += tile_h each iteration | `DAT_00B0FBD4` / SHELL2.PAL | Repeated `DAT_00B0FA20` times |
| 3 | `g_SDBTNANM_SHP` | `SDBTNANM.SHP` | 10 | `DAT_00B0FC10`, y += tile_h each iteration | `DAT_00B0FBDC` / SDBTNANM.PAL | Only when `param_3 == '\0'` (flag byte `+0xD4 == 0`) |
| 4 | `DAT_00B0FA38` | `SDBTM.SHP` | 0 | `DAT_00B0FC28` | `DAT_00B0FBCC` / SHELL.PAL | Always |
| 5 | width-selected | `LWSCRNS.SHP` (640) / `LWSCRNL.SHP` (else) | 0 | `DAT_00B0FC2C` | `DAT_00B0FBCC` / SHELL.PAL | Always |

For step 2 (SDBTNBKGD tile loop):
- Loop count = `DAT_00B0FA20` = `(usable_height - SDTP_height) / SDBTNBKGD_tile_height`
- Y increments by `DAT_00B0FC24[3]` (= SDBTNBKGD tile pixel height) each iteration

For step 3 (SDBTNANM overlay loop, when active):
- Loop count = same `DAT_00B0FA20`
- Y increments by `DAT_00B0FC10[3]` (= SDBTNANM tile pixel height)
- Frame is always `10` (not `0`)

Confidence: High — decompiled `0x0072E450` directly in this session.

---

## 6. `Background_Overlay` — Parent Background Draw

**Signature** (fastcall): called as `Background_Overlay(convert, small_shp, large_shp)`

From `WM_PAINT_Handler` call site:
- `param_1` / ECX = convert (`piVar1[0x1e]` = dialog record `+0x74`, i.e., the convert for dialog `0xE2`)
- `param_2` / EDX = rect pointer (surface bounds, same `param_2` used above)
- `param_3` = `piVar1[0x39]` = small SHP (dialog record `+0xE4` at `piVar2[0x39]`)
- `param_4` = `piVar1[0x3a]` = large SHP (dialog record `+0xE8` at `piVar2[0x3A]`)

Note: The decompiler signature shows 5 params (`param_1..param_5`); `param_2` is the
rect, and `param_3` is the convert. Assembly shows ECX is the convert and EDX is the
rect at the function entry. Due to fastcall register/stack ordering, `param_4` is the
small SHP and `param_5` is the large SHP.

Width switch:
```c
local_18 = DAT_00b0fc1c[0];  // x from layout rect
local_14 = DAT_00b0fc1c[1];  // y from layout rect
if (g_ScreenWidth == 640) {
    CC_Draw_Shape(param_4, 0, &local_18, &local_10, 0x400, ...);  // small SHP
    return;
}
CC_Draw_Shape(param_5, 0, &local_18, &local_10, 0x400, ...);      // large SHP
```

### For dialog `0xE2` specifically

`FUN_0060CF00` handles dialog `0xE2` in the generic common-shell branch (not a
special case like Skirmish `0x102`):

```c
record[0x1E] = FUN_0072E280();  // returns DAT_00B0FBCC = SHELL.PAL convert
record[0x39] = DAT_00B0FB50;    // MNSCRNS.SHP
record[0x3A] = DAT_00B0FA04;    // MNSCRNL.SHP
```

So for dialog `0xE2`:

| Screen width | Background drawn | Asset | Convert | Position |
|---:|---:|---|---|---|
| 640 | param_4 = `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FBCC` / SHELL.PAL | `(DAT_00B0FC1C[0], DAT_00B0FC1C[1])` = `(0, 0)` at ≤1023 |
| 800 | param_5 = `DAT_00B0FA04` | `MNSCRNL.SHP` | `DAT_00B0FBCC` / SHELL.PAL | `(0, 0)` at 800×600 |
| 1024 | param_5 = `DAT_00B0FA04` | `MNSCRNL.SHP` | `DAT_00B0FBCC` / SHELL.PAL | `(112, 84)` |

This contrasts with Skirmish `0x102`, which uses `DAT_00B0FA18`
(`MnScrnLCoopGameSetup.shp`) as its large background.

Frame is always `0`. Flags are `0x400` (no center-sprite flag).

Confidence: High — from decompile of `0x0072E730` and `FUN_0060CF00` (prior report).

---

## 7. Z-order: Right Panel vs Parent Background

Call sequence in `WM_PAINT_Handler` mode-1 branch:
```
RightPanel__Draw(...)     ← drawn first (SDTP, SDBTNBKGD, SDBTNANM, SDBTM, LWSCRN*)
Background_Overlay(...)   ← drawn second (MNSCRNS / MNSCRNL)
```

The right-panel SHP stack is drawn **before** the parent background overlay. In the
composited offscreen `BSurface`, `Background_Overlay` draws on top of the right-panel
SHPs.

The lower/side strip (`LWSCRN*`) occupies the bottom-left area (`x = local_c`), while
all other right-panel SHPs are right-aligned to `local_10 - asset_width`. The
background overlay draws at the shell band origin `(local_c, iVar6)`.

Visually, `Background_Overlay` (`MNSCRNS/MNSCRNL`) covers the left portion of the
screen while the right panel SHPs occupy the right side. The stripe drawing order
within `RightPanel__Draw` is: top-cap → tile band → bottom-cap → side strip.

Confidence: High — call order verified from `WM_PAINT_Handler` decompile.

---

## 8. Verified Negative: `Background_Overlay` Is Called for Dialog `0xE2`

`Background_Overlay` is NOT a verified negative for `0xE2` — it IS called. The
anchor doc's §"Parent Background and Right Panel Assets" correctly documents it.

The only functions where `Background_Overlay` is NOT called are dialogs where
`FUN_0072E260()` returns `0` (init gate not set), or when the alternate left-panel
branch is taken. For `0xE2` in standard YR, `FUN_0072E260()` returns `DAT_00B0FBE0`,
which is set to `1` by the lazy-init sequence on the first paint.

`FUN_0072E260 @ 0x0072E260` body:
```c
return DAT_00b0fbe0;
```

Confidence: High.

---

## 9. Pixel Position Summary at Standard Resolutions

All coordinates derived from the verified formulas in §4. Pixel values for asset
dimensions use the correct formula but the actual pixel size depends on retail SHP
header values; the positions below are formula-derived.

Let `SW` = screen width, `SW` = screen height. Let SDTP_W, SDTP_H denote SDTP pixel
dimensions from SHP header at `[+2]` and `[+4]`.

| Asset | x | y |
|---|---|---|
| `MNSCRNS.SHP` or `MNSCRNL.SHP` (background) | `max((SW-800)/2, 0)` if `SW>1023`, else `0` | `max((SH-600)/2, 0)` if `SH>767`, else `0` |
| `SDTP.SHP` | `right_edge − SDTP_W` | `y_offset` |
| `SDBTNBKGD.SHP` tile 0 | `right_edge − SDTP_W` | `y_offset + SDTP_H` |
| `SDBTNBKGD.SHP` tile i | same x | `y_offset + SDTP_H + i × tile_H` |
| `SDBTNANM.SHP` tile i | `right_edge − SDTP_W + (SDTP_W − SDBTNANM_W)` | same y as SDBTNBKGD tile i |
| `SDBTM.SHP` | `right_edge − SDBTM_W` | `y_offset + SDTP_H + N_tiles × tile_H` |
| `LWSCRNS.SHP` or `LWSCRNL.SHP` | `x_offset` (= `local_c`) | `local_4 − LW_H` |

where `right_edge = local_10 = SW` if `SW ≤ 1023`, else `(SW + 800) / 2`.

---

## 10. Open Questions

1. The semantic meaning of dialog window-extra byte `+0xD4` (the `SDBTNANM` overlay
   flag) — what player-visible state it represents — remains unresolved. Its writer
   is `FUN_00608440 @ 0x00608440` (4 callers). See
   `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md`.

2. Exact retail pixel dimensions of `SDTP.SHP`, `SDBTNBKGD.SHP`, and `SDBTM.SHP`
   from SHP headers have not been read in this session; the formulae are verified but
   the resulting pixel positions require these values to produce absolute numbers.

3. `DAT_00B0FC18` (the `SDWRNTMP.SHP` rect, computed alongside SDTP) and `DAT_00B0FC14`
   (a rect for `DAT_00B0F9DC` = `SDMPBTN.SHP`) are computed by
   `RightPanel__ComputeLayoutRects` but never used in `RightPanel__Draw`. They may be
   used by adjacent sidebar/right-panel functions not decompiled in this scope.

4. The `FUN_0072E9F0` and `FUN_0072EAD0` callers of `RightPanel__Draw` were not
   decompiled; they may represent alternate draw paths (e.g., forced repaint or
   non-`WM_PAINT` paint paths). For standard `0xE2`, only the `WM_PAINT_Handler` path
   is verified active.

---

## Sources (This Session)

Ghidra functions decompiled:

- `RightPanel__Draw @ 0x0072E450`
- `RightPanel__ComputeLayoutRects @ 0x0072EC70`
- `Background_Overlay @ 0x0072E730`
- `WM_PAINT_Handler @ 0x00621E90`
- `FUN_0072E260 @ 0x0072E260`
- `FUN_0072DFB0 @ 0x0072DFB0`
- `Sidebar_RightPanel_SHP_Loading @ 0x0072EB50`
- `FUN_0072E820 @ 0x0072E820`
- `FUN_0072E1B0 @ 0x0072E1B0`
- `FUN_0072AA40 @ 0x0072AA40`

Assembly context read:

- Call sites of `RightPanel__ComputeLayoutRects` at `0x0072E18D` (screen_w/h args)
- Function entry context of `Background_Overlay @ 0x0072E730`

Caller traces:

- `RightPanel__Draw`: callers at `0x0072E820`, `0x0072E9F0`, `0x0072EAD0`, `WM_PAINT_Handler @ 0x00621E90`
- `Background_Overlay`: callers at `0x0072E820`, `WM_PAINT_Handler @ 0x00621E90`
- `RightPanel__ComputeLayoutRects`: callers at `0x0072AA40`, `FUN_0072DFB0`, `FUN_0072E1B0`, `RightPanel__Draw`, `SidebarSurface__Init @ 0x0072DDB0`

Prior reports consulted (no re-investigation performed):

- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_BACKGROUND_OVERLAY_PLACEMENT_FOLLOWUP_GHIDRA_REPORT.md`
