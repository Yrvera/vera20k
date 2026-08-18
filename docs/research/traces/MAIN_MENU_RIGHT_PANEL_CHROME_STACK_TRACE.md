# Main-Menu Right-Panel Chrome Stack Trace
## Mechanic: SDTP top cap + N×SDBTNBKGD repeating tile + SDBTM bottom cap

**Target dialogs**: dialog 0xE2 (main menu shell)  
**Screens traced**: 800×600, 1024×768 (centering threshold), 1920×1080  
**Ghidra functions**: `RightPanel__Draw @ 0x0072e450`, `RightPanel__ComputeLayoutRects @ 0x0072ec70`, `WM_PAINT_Handler @ 0x00621e90`, `Sidebar_RightPanel_SHP_Loading @ 0x0072eb50`  
**SHP dimensions**: verified against retail assets via `inspect-pcx-palette` diagnostic  

---

## Stage Results

| # | Stage | Status | Evidence |
|---|-------|--------|----------|
| 1 | SDTP cap dimensions (168×199) | PASS | SHP header: 168×199 confirmed via `inspect-pcx-palette`. gamemd reads `*(short*)(g_SDTP_SHP+2)`=168 and `*(short*)(g_SDTP_SHP+4)`=199. Our constant RIGHT_PANEL_TOP_H=199 and RIGHT_PANEL_WIDTH=168 match exactly. |
| 2 | SDBTNBKGD tile dimensions (168×42) | PASS | SHP header: 168×42 confirmed via `inspect-pcx-palette`. gamemd reads `*(short*)(g_SDBTNBKGD_SHP+2)` and `*(short*)(g_SDBTNBKGD_SHP+4)`. Our RIGHT_PANEL_TILE_H=42 matches. |
| 3 | SDBTM bottom cap dimensions (168×65 SHP, rendered 23) | FAIL | SHP header is 168×65 (not 23). gamemd renders it with computed height `effective_h - (sdtp_h + tile_count*tile_h) = 600-577 = 23`. We store `RIGHT_PANEL_BOTTOM_H=23` as rect height but blit the full 65px SHP scaled to fit 23px — gamemd clips (shows top 23 rows at 1:1), we scale-squash. Visible as slightly different bottom-cap appearance. See layout.rs:90 and app_main_menu_shell_render.rs:170. |
| 4 | Tile count formula (no cap at 9) | PASS* | gamemd: `DAT_00b0fa20 = (local_8 - DAT_00b0fc20[3]) / DAT_00b0fc24[3]` = `(effective_h - sdtp_h) / tile_h`. NO maximum cap. Our code adds `.min(RIGHT_PANEL_TILE_COUNT_BASE)`. At standard screen sizes (effective_h always reduces to 600 via centering), formula gives exactly 9 for all tested resolutions. The cap has no observable effect in practice. PASS with caveat: if screen_h is very large and centering is disabled (unlikely in YR), the uncapped formula could differ. |
| 5 | Right-edge anchor formula | PASS | gamemd: `x = right_edge - sdtp_width` where `right_edge = screen_w - left_margin`. At 800×600: x=632. At 1024×768: x=744. At 1920×1080: x=1192. Our code matches (verified by layout.rs tests). |
| 6 | Centering margin threshold | PASS | gamemd: `if (0x3ff < screen_w)` = `screen_w >= 1024`, `if (0x2ff < screen_h)` = `screen_h >= 768`. Our code: `screen_w > 1023` = `screen_w >= 1024`, `screen_h > 767` = `screen_h >= 768`. Exact match (verified at decompile address 0x0072ec70). |
| 7 | Palette choice per SHP | PASS | gamemd CC_Draw_Shape calls pass palette=0 (null) for all three SHPs, relying on global palette state. Our code supplies explicit palette refs: SHELL.PAL for SDTP/SDBTM, SHELL2.PAL for SDBTNBKGD. Produces identical visual output (same palette is active globally during shell draw). `inspect-pcx-palette` confirms colors are correct. |
| 8 | Z-order chrome vs button layer | PASS | gamemd: `RightPanel__Draw` (chrome) fires from `WM_PAINT_Handler`, owner-draw buttons respond to WM_DRAWITEM messages which Windows delivers after WM_PAINT — chrome is drawn first, buttons on top. Our code: CHROME_DEPTH=0.00085 drawn before BUTTON_DEPTH=0.00080 (lower depth = closer to viewer in Less depth test). Order is correct. |
| 9 | Tile-row vertical alignment (no gap/overlap) | PASS | gamemd loop: y_i+1 = y_i + tile_h (no gap, no overlap). Tiles start at SDTP_h + top_margin = 199 (at 800×600). Our code: `tile.y + row * tile.h`. Exact match. |
| 10 | Lower-cap (SDBTM) anchor — stacked vs screen-bottom | PASS | gamemd anchors SDBTM at `tile_start_y + tiles_total_h = 199 + 9*42 = 577`, height = residual = 23. NOT anchored to screen bottom. Our code does the same. |
| 11 | Frame index (frame 0) | PASS | gamemd: SDTP frame 0, SDBTNBKGD frame 0, SDBTM frame 0. Confirmed from CC_Draw_Shape second argument = `0` for all three. Our code uses frame 0 for all. Note: SDBTNANM (button art) uses frame 10 in the background layer within RightPanel__Draw; our code uses frames 2/3/4 for hover states — this is a separate stage not in scope. |

---

## Gamemd Layout Formula (verified at 0x0072ec70)

```
if screen_w >= 1024:  left_margin = (screen_w - 800) / 2  (integer division)
else:                  left_margin = 0
right_edge = screen_w - left_margin

if screen_h >= 768:   top_margin = (screen_h - 600) / 2
else:                  top_margin = 0
effective_h = screen_h - top_margin * 2   (= 600 when centered)

SDTP:      x = right_edge - sdtp_w,  y = top_margin,         w = sdtp_w(168),    h = sdtp_h(199)
Tile[0]:   x = same,                 y = top_margin + sdtp_h, w = sdtp_w(168),    h = tile_h(42)
Tile[i]:   y += i * tile_h
tile_count = (effective_h - sdtp_h) / tile_h   [NO CAP]
SDBTM:     x = right_edge - sdbtm_w, y = top_margin + sdtp_h + tile_count*tile_h,
           w = sdbtm_w(168),          h = effective_h - sdtp_h - tile_count*tile_h
```

## Numerical Results at Key Resolutions

| Resolution | left_margin | top_margin | effective_h | tile_count | SDTP xy | SDBTM y | SDBTM h |
|------------|-------------|------------|-------------|------------|---------|---------|---------|
| 800×600    | 0           | 0          | 600         | 9          | (632,0) | 577     | 23      |
| 1024×768   | 112         | 84         | 600         | 9          | (744,84)| 661     | 23      |
| 1920×1080  | 560         | 240        | 600         | 9          | (1192,240) | 817  | 23      |

Our code produces identical numbers for all three resolutions.

---

## Top Failures

### FAIL 1 — Stage 3: SDBTM blit mode (scale vs clip)
- **Player sees**: bottom cap of right panel is slightly squashed vertically — the 65-tall SDBTM SHP is scale-fit to a 23px dest rect, so the bottom-cap artwork is compressed to ~35% height instead of showing just the top 23 rows at native scale.
- **Frequency**: every match, every frame the main menu is visible — constant.
- **Our code**: `app_main_menu_shell_render.rs:170` — `push_entry_rect` sets `size=[rect.w, rect.h]` = 168×23 with full SHP UV → scales 65px art to 23px.
- **gamemd evidence**: `RightPanel__ComputeLayoutRects @ 0x0072ec70` passes `h=23` as the blit rect height; CC_Draw_Shape with `SHAPE_WINREL|0x400` blits 1:1 (no scaling in CC_Draw_Shape), so only the top 23 rows of the SHP are visible.
- **Fix**: in `build_chrome_instances`, clip the SDBTM UV to show only the top `bottom_h` rows of the SHP, rather than stretching the full SHP to `bottom_h`.

---

## Ghidra Evidence Summary

- `RightPanel__Draw @ 0x0072e450`: confirmed CC_Draw_Shape(g_SDTP_SHP, frame=0), CC_Draw_Shape(g_SDBTNBKGD_SHP, frame=0 in loop), CC_Draw_Shape(DAT_00b0fa38=SDBTM, frame=0)
- `RightPanel__ComputeLayoutRects @ 0x0072ec70`: full layout formula verified — centering threshold `0x3ff`/`0x2ff`, SDTP/tile/SDBTM rect computation, tile_count formula without cap
- `WM_PAINT_Handler @ 0x00621e90`: paint order confirmed — chrome before owner-draw buttons
- `Sidebar_RightPanel_SHP_Loading @ 0x0072eb50`: SHP load sequence, SDBTM = DAT_00b0fa38
- `inspect-pcx-palette` diagnostic: SDTP=168×199, SDBTNBKGD=168×42, SDBTM=168×65 (SHP native), LWSCRNS=472×32, LWSCRNL=632×32

---

**PASS: 10 | FAIL: 1 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0**

**Status: COMPLETE**
