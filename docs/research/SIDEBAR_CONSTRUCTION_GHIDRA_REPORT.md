# Sidebar Construction & Assembly — Ghidra Research Report

Source: live decompilation of `gamemd.exe` via Ghidra MCP.
All layout math verified from raw decompiled C code.

---

## 1. Architecture Overview

The sidebar is **NOT a single image**. It is composed from **multiple tiled SHP pieces** whose
positions are **computed from the SHP header dimensions** (width at offset +2, height at offset +4).

There are **two independent panel systems**:

1. **Right-side panel** (cameo strip housing) — 10 SHPs, 8 layout rects
2. **Left/top panel** (radar, credits, power bar, button strip) — 16 SHPs, 14+ layout rects

Both systems:
- Load lazily on first draw
- Compute layout rects from SHP pixel dimensions (nothing hardcoded)
- Use vertical and/or horizontal tiling to fill available space
- Adapt to resolution via tile count adjustments and centering margins

---

## 2. SHP Header Structure (verified from Ghidra)

The SHP(TS) in-memory format:

**File Header (8 bytes):**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16 | `marker` — 0x0000 for direct SHP, 0xFFFF for lazy-load wrapper |
| +0x02 | u16 | `width` — max frame width |
| +0x04 | u16 | `height` — max frame height |
| +0x06 | u16 | `frame_count` |

**Per-Frame Header (24 bytes each, starting at +0x08):**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16 | frame_x (offset within bounds) |
| +0x02 | u16 | frame_y |
| +0x04 | u16 | frame_width |
| +0x06 | u16 | frame_height |
| +0x08 | u8 | flags (bit 1 = RLE compressed) |
| +0x14 | u32 | data_offset (from file start) |

**Lazy-load wrapper** (marker = 0xFFFF): created by `FUN_0069e430`, resolved on first access
by `FUN_0069e580`. Width/height are still at +2/+4 (copied from on-disk header), so layout
calculations work before the full SHP is loaded.

### The Key Insight: SHP-Dimension-Driven Layout

Every piece position is derived from reading the SHP header:
```c
width  = *(short*)(shp_ptr + 2);   // SHP header offset +2
height = *(short*)(shp_ptr + 4);   // SHP header offset +4
```

The layout calculator reads these dimensions and chains pieces together:
- Next piece Y = previous piece Y + previous piece height
- Tile count = available_space / tile_height
- Bottom piece fills remainder: `height = total_height - (top_height + tiles * tile_height)`

**This means changing the SHP files changes the layout automatically.** The engine adapts
to whatever dimensions the art has — it does not assume fixed pixel sizes.

---

## 3. Right-Side Panel (Cameo Strip Housing)

### SHP Loading (`FUN_0072eb50` at `0x0072eb50`)

| Global | SHP Name | Purpose |
|---|---|---|
| `DAT_00b0faf8` | **SDTP.SHP** | Top cap of sidebar |
| `DAT_00b0fa74` | **SDBTNBKGD.SHP** | Cameo background tile (VERTICALLY TILED) |
| `DAT_00b0fac4` | **SDBTNANM.SHP** | Button animation overlay (tiled with SDBTNBKGD) |
| `DAT_00b0fa38` | **SDBTM.SHP** | Bottom cap of sidebar |
| `DAT_00b0fb50` | **MNSCRNS.SHP** | Screen corner (640×480 variant) |
| `DAT_00b0fa04` | **MNSCRNL.SHP** | Screen corner (800×600+ variant) |
| `DAT_00b0fae8` | **LWSCRNS.SHP** | Lower corner (640×480) |
| `DAT_00b0fa54` | **LWSCRNL.SHP** | Lower corner (800×600+) |
| `DAT_00b0f9dc` | **SDMPBTN.SHP** | Minimap/scroll button |
| `DAT_00b0fac0` | **SDWRNTMP.SHP** | Warning template overlay |

### Layout Rect Calculation (`FUN_0072ec70` at `0x0072ec70`)

Called with `(screen_width, screen_height)`. Creates 8 RECT structs (`{x, y, w, h}`):

```
For resolutions > 1024 wide:  left_margin = (width - 800) / 2
For resolutions > 768 tall:   top_margin  = (height - 600) / 2
Otherwise: margins = 0
```

| Rect | Global | Computation |
|---|---|---|
| Background | `DAT_00b0fc1c` | `{left_margin, top_margin, corner_shp_w, corner_shp_h}` |
| Sidebar top | `DAT_00b0fc20` | `{screen_w - sdtp_w, top_margin, sdtp_w, sdtp_h}` |
| Top alt | `DAT_00b0fc18` | `{screen_w - sdtp_w, top_margin, sdtp_w, sdwrntmp_h}` |
| Cameo tile | `DAT_00b0fc24` | `{screen_w - sdtp_w, sdtp_h + top_margin, sdtp_w, sdbtnbkgd_h}` |
| Anim overlay | `DAT_00b0fc10` | `{sdtp_w - anim_w + sidebar_x, cameo_y, anim_w, anim_h}` |
| Map button | `DAT_00b0fc14` | `{screen_w - btn_w, tile_end - btn_h, btn_w, btn_h}` |
| Bottom cap | `DAT_00b0fc28` | `{screen_w - btm_w, tiles_end, btm_w, remaining_h}` |
| Lower corner | `DAT_00b0fc2c` | `{left_margin, screen_h - lwscrn_h, lwscrn_w, lwscrn_h}` |

**Tile count**: `DAT_00b0fa20 = (screen_h - sdtp_h) / sdbtnbkgd_h`

### Draw Function (`FUN_0072e450` at `0x0072e450`)

Draw order (top to bottom):

```
1. SDTP.SHP frame 0 ────────── top cap (right-aligned)
2. SDBTNBKGD.SHP frame 0 ──── tiled N times vertically (cameo background)
3. SDBTNANM.SHP frame 10 ──── tiled N times (button animation overlay)
4. SDBTM.SHP frame 0 ──────── bottom cap (fills remainder)
5. LWSCRNL/S.SHP frame 0 ──── lower-left corner piece
```

Visual layout:
```
                              ┌──────────────┐
                              │  SDTP.SHP    │ ← top cap
                              ├──────────────┤
                              │ SDBTNBKGD ×1 │ ← tile
                              │ SDBTNBKGD ×2 │ ← tile
                              │ SDBTNBKGD ×3 │ ← tile
                              │     ...       │ ← tiles repeat
                              │ SDBTNBKGD ×N │ ← tile
                              ├──────────────┤
                              │  SDBTM.SHP   │ ← bottom cap (fills remainder)
                              └──────────────┘
  ┌────────────┐
  │LWSCRNL.SHP │ ← lower-left corner
  └────────────┘
```

---

## 4. Left/Top Panel (Radar, Credits, Buttons)

### SHP Loading (`FUN_0072fa10` at `0x0072fa10`)

Has a **YR vs RA2 branch** (`ScenarioClass + 0x34B8 == 2` for YR):

**Resolution-dependent background (3 variants):**

| Global | RA2 SHP | YR SHP | Resolution |
|---|---|---|---|
| `DAT_00b0fad4` | BKGDSM.SHP | BKGDSMY.SHP | 640×480 |
| `DAT_00b0fac8` | BKGDMD.SHP | BKGDMDY.SHP | 800×600 |
| `DAT_00b0fa50` | BKGDLG.SHP | BKGDLGY.SHP | >800 |

**Radar frame:**

| Global | RA2 SHP | YR SHP |
|---|---|---|
| `DAT_00b0fa68` | RADAR.SHP | RADARY.SHP |

**Shared panel pieces:**

| Global | SHP Name | Purpose |
|---|---|---|
| `DAT_00b0fb08` | **CREDITS.SHP** | Credits display background |
| `DAT_00b0f9e0` | **TOP.SHP** | Top decoration / radar frame top |
| `DAT_00b0fa70` | **SIDE1.SHP** | Strip below radar |
| `DAT_00b0fafc` | **SIDE2B.SHP** | Power bar tile (VERTICALLY TILED) |
| `DAT_00b0fa00` | **SIDE3.SHP** | Strip piece 3 |
| `DAT_00b0fa8c` | **ADDON.SHP** | Addon/expansion area |
| `DAT_00b0fa48` | **LSPACER.SHP** | Bottom bar left spacer |
| `DAT_00b0fa3c` | **LENDCAP.SHP** | Bottom bar left end cap |
| `DAT_00b0fa90` | **BTTNBKGD.SHP** | Button background (HORIZONTALLY TILED) |
| `DAT_00b0fabc` | **RENDCAP.SHP** | Bottom bar right end cap |

### Layout Rect Calculation (`FUN_0072fc60` at `0x0072fc60`)

Stacks pieces **vertically along the right edge**, then adds a **horizontal bottom bar**:

**Vertical stack (right-aligned, top to bottom):**

| Rect | Global | Content |
|---|---|---|
| Background | `DAT_00b0fc30` | `{0, 0, bkgd_w, bkgd_h}` |
| Credits | `DAT_00b0fc34` | `{screen_w - credits_w, 0, credits_w, credits_h}` |
| Top | `DAT_00b0fc38` | `{same_x, credits_h, same_w, top_h}` |
| Radar | `DAT_00b0fc3c` | `{same_x, credits_h + top_h, same_w, radar_h}` |
| Side1 | `DAT_00b0fc44` | `{same_x, after_radar, same_w, side1_h}` |
| Side2B tile | `DAT_00b0fc48` | `{same_x, after_side1, same_w, side2b_h}` (tiled) |
| Side3 | `DAT_00b0fc4c` | `{same_x, after_tiles, same_w, side3_h}` |
| Addon | `DAT_00b0fc50` | `{same_x, after_side3, same_w, addon_h}` |

**Tile counts:**
- Vertical: `DAT_00b0fadc = (screen_h - side3_h - side1_h - radar_h - top_h - credits_h) / side2b_h`
- Horizontal: `DAT_00b0f9e4 = (screen_w - lendcap_w - credits_w - rendcap_w) / bttnbkgd_w`

**Horizontal bottom bar (left to right):**

| Rect | Global | Content |
|---|---|---|
| Spacer | `DAT_00b0fc5c` | `{0, screen_h - 32, lspacer_w, lspacer_h}` |
| End cap | `DAT_00b0fc60` | `{lspacer_w, screen_h - 32, lendcap_w, lendcap_h}` |
| Button tiles | `DAT_00b0fc68` | `{after_endcap, screen_h - 32, bttnbkgd_w, bttnbkgd_h}` (tiled) |
| Right cap | `DAT_00b0fc6c` | `{right_edge, screen_h - 32, rendcap_w, rendcap_h}` |

### Draw Function (`FUN_0072f540` at `0x0072f540`)

```
Top-to-bottom:                    Bottom bar (left-to-right):
┌─────────────────────────┐       ┌────┬────┬────┬───┬────┬────┐
│ BKGD*.SHP (background)  │       │LSPC│LEND│BTTN│...│BTTN│REND│
├────────┬────────────────┤       │ER  │CAP │BKGD│×N │BKGD│CAP │
│        │ CREDITS.SHP    │       └────┴────┴────┴───┴────┴────┘
│        ├────────────────┤
│        │ TOP.SHP        │
│        ├────────────────┤
│        │ RADAR(Y).SHP   │
│        ├────────────────┤
│        │ SIDE1.SHP      │
│        ├────────────────┤
│        │ SIDE2B.SHP ×1  │ ← tiled vertically
│        │ SIDE2B.SHP ×2  │
│        │ SIDE2B.SHP ×N  │
│        ├────────────────┤
│        │ SIDE3.SHP      │
│        ├────────────────┤
│        │ ADDON.SHP      │
│        └────────────────┘
```

---

## 5. Master Draw Orchestration

### WM_PAINT Handler (`FUN_00621e90` at `0x00621e90`)

This is the top-level compositor that decides what to draw:

```
Mode 0 (menu):      draws dbak6440.pcx background
Mode 1 (in-game):   right panel + left panel + overlays
Mode 2 (loading):   side-specific loading backgrounds (PUDLGBG*.SHP)
```

### Complete In-Game Draw Chain

```
FUN_00621e90 (WM_PAINT handler)
│
├── FUN_0072e450 (RIGHT sidebar frame)
│   ├── FUN_0072e2d0 (fill margins for >800×600)
│   ├── DrawSHP(SDTP.SHP)              ── top cap
│   ├── DrawSHP(SDBTNBKGD.SHP) × N    ── cameo bg tiles
│   ├── DrawSHP(SDBTNANM.SHP) × N     ── button overlay
│   ├── DrawSHP(SDBTM.SHP)            ── bottom cap
│   └── DrawSHP(LWSCRNL/S.SHP)        ── corner
│
├── FUN_0072e730 (background overlay)
│   └── DrawSHP(MNSCRNS/MNSCRNL.SHP)  ── resolution bg
│
├── FUN_0072e8c0 (sidebar top highlight)
│   └── DrawSHP(SDTP.SHP, frame=1)
│
├── FUN_0072e860 (minimap button)
│   └── DrawSHP(SDMPBTN.SHP, frame=0)
│
└── FUN_0072e920 (radar background)
    └── DrawSHP(side_specific_radar_bg)

FUN_006d0a30 (main game loop sidebar draw)
│
├── DrawSHP(CREDITS.SHP)              ── credits background
├── CreditsClass::Draw                ── credits text overlay
│
├── [if bottom bar dirty:]
│   ├── DrawSHP(LSPACER.SHP)          ── bottom bar left
│   ├── DrawSHP(LENDCAP.SHP)          ── bottom bar left cap
│   ├── DrawSHP(BTTNBKGD.SHP) × N    ── tiled button backgrounds
│   ├── [tooltip button renders]
│   └── DrawSHP(RENDCAP.SHP)          ── bottom bar right cap
│
└── SidebarClass::Draw                ── cameo strips (SIDE1/2/3 + cameos)
```

---

## 6. Per-Side Theming Pipeline

### How Side Selection Flows to Sidebar Rendering

```
1. Player's HouseType.Side → ScenarioClass+0x34B8  (0=Allied, 1=Soviet, 2=Yuri)
       │
2. MIX loading: side+1 → SIDEC%02d.MIX / SIDEC%02dMD.MIX
       │         *** Yuri (2) maps to Soviet (1) — shares SIDEC02*.MIX ***
       │
3. Palette loading: FUN_0072f350 → loads SIDEBAR.PAL variants from active MIX
       │             Yuri gets different palette load order
       │
4. Text color: SetSidebarTextColor picks 1 of 3 RGB values (0/1/2)
       │
5. Art loading: FUN_006a5840 → loads TAB00-03.SHP, GCLOCK2.SHP, etc. from active MIX
       │         *** Same filenames, different art per side's MIX ***
       │
6. Layout constants: FUN_006a5130 / FUN_006a5090 → per-side pixel offsets
       │             Allied: tab spacing 29, column width 63, tab height 26
       │             Soviet/Yuri: tab spacing 32, column width 64, tab height 18
       │
7. Drawing: uses ScenarioClass+0x34B8 to select observer icons, tab heights
```

### Side-to-MIX File Mapping

| Side | Name | MIX Files | Note |
|---|---|---|---|
| 0 | Allied | SIDEC01MD.MIX, SIDEC01.MIX, SIDENC01.MIX | Unique art |
| 1 | Soviet | SIDEC02MD.MIX, SIDEC02.MIX, SIDENC02.MIX | Unique art |
| 2 | Yuri | **Same as Soviet** (SIDEC02*.MIX) | Palette swap only |

### Side-Specific Radar Backgrounds

| Global | Allied 640 | Allied 800+ | Soviet 640 | Soviet 800+ | Yuri 800+ |
|---|---|---|---|---|---|
| `DAT_00b0fb34` | ASCRBKSM | ASCRBKMD | SSCRBKSM | SSCRBKMD | SYCRBKMD |
| `DAT_00b0fb00` | ASCRTSM | ASCRTMD | SSCRTSM | SSCRTMD | SYCRTMD |
| `DAT_00b0fb30` | ASCRASM | ASCRAMD | SSCRASM | SSCRAMD | SYCRAMD |

---

## 7. Resolution Handling

### Resolution Branching Points

| Width | Height | Behavior |
|---|---|---|
| 640 | 480 | Uses small SHP variants (MNSCRNS, LWSCRNS, BKGDSM/Y) |
| 800 | 600 | Uses large SHP variants (MNSCRNL, LWSCRNL, BKGDMD/Y) — **designed target** |
| >800 | — | Uses BKGDLG/Y; reads `[Art_800]` section from UIMD.INI |
| >1024 | — | Adds left margin: `(width - 800) / 2` — content centered |
| — | >768 | Adds top margin: `(height - 600) / 2` — content centered |

### How Resolution Affects Tile Counts

The engine adapts by adjusting tile counts, not SHP art:
```c
// Right panel vertical tiles:
tile_count = (screen_height - top_cap_height) / tile_height;

// Left panel vertical tiles (power bar area):
tile_count = (screen_h - credits_h - top_h - radar_h - side1_h - side3_h) / side2b_h;

// Bottom bar horizontal tiles:
tile_count = (screen_w - left_cap_w - sidebar_w - right_cap_w) / button_tile_w;
```

### >800px Art Override (UIMD.INI)

`FUN_007681e0` reads UI layout from UIMD.INI inside the side MIX:
```c
if (screen_width > 800) {
    result = ReadINISection("Art_800", ...);
    if (result < 1) fallback_to_default_section();
}
```

The `[Art_800]` section can override: SideBarSize, HelpBarSize, TextRect, TooltipRect,
TitleRect, Background SHP, SideBar SHP, HelpBar SHP, and all button art.

### SideBarSize

Read from UIMD.INI as a `{width, height}` point. Default = `{0, 0}`.
This controls the **NewSidebar internal panel dimensions**, NOT the hardcoded 158px
(`DAT_00886f94 = 0x9E`) which is always set in `FUN_006a5130`.

---

## 8. Palettes

| Global | File | Used By |
|---|---|---|
| `DAT_00b0fbcc` | **SHELL.PAL** | Right sidebar frame SHPs |
| `DAT_00b0fbd4` | **SHELL2.PAL** | Additional sidebar SHPs |
| `DAT_00b0fbdc` | **SDBTNANM.PAL** | Button animation SHP |
| `DAT_00b0fb68` | **DIALOG.PAL** | Allied sidebar variant |
| `DAT_00b0fb70` | **DIALOGY.PAL** | Soviet/Yuri sidebar variant |
| `DAT_00b0fb60` | **DIALOGN.PAL** | Neutral sidebar variant |
| `DAT_00b0fb78` | **MAINBTTN.PAL** | Main button art |
| `DAT_00b0fbe4` | (sidebar chrome) | ConvertClass at `DAT_0087f6cc` — TAB, SELL, REPAIR, GCLOCK2 |
| `DAT_00b0fbfc` | (faction icons) | ConvertClass at `DAT_0087f6d0` — OBSALLI, USAI, etc. |

---

## 9. DrawSHP Function (`0x004aed70`)

```c
void DrawSHP(
    int* shp_ptr,       // SHP file pointer (direct or lazy-load)
    int  frame_index,   // which frame to draw
    short* position,    // {x, y} screen position
    int* clip_rect,     // clipping rectangle
    uint flags,         // rendering flags
    int  remap_ptr,     // palette remap (0 = none)
    int  zshape,        // z-shape buffer pointer
    uint param8,        // unused for sidebar
    uint z_adjust,      // z-value (1000 for sidebar = always on top)
    ...                 // additional params (0 for sidebar)
);
```

**Key flags:**
- `0x400` — Standard blit (all sidebar draws use this)
- `0x401` — Transparent blit (DARKEN.SHP overlay)
- `0x404` — Transparent + darken blend (GCLOCK2, flash effects)
- `0x200` — Center sprite on position (not used by sidebar)

---

## 10. Vestigial Animation SHPs (b0b478/b0b47c/b0b480)

Globals `DAT_00b0b478`, `DAT_00b0b47c`, `DAT_00b0b480` are **never written** in YR.
They are remnants of Tiberian Sun's sidebar open/close animation. All code that
references them is guarded by null checks:

```c
if (DAT_00b0b478 != 0) {  // Always false in YR
    DrawSHP(DAT_00b0b478, animation_frame, ...);
}
```

The animation frame at `SidebarClass+0x5394` and direction at `+0x5398` exist but
the SHPs they would draw are never loaded. **These can be safely ignored for YR.**

---

## 11. Lazy Initialization Pattern

Both panel systems use lazy init guarded by `DAT_00b0fbe0`:

```c
void DrawRightSidebarFrame(...) {
    if (!DAT_00b0fbe0) {
        LoadNTRLMix();           // NTRLMD.MIX + NEUTRAL.MIX
        LoadRightPanelSHPs();    // 10 SHP files
        LoadPalettes();          // SHELL.PAL, SHELL2.PAL, SDBTNANM.PAL
        CalcLayoutRects();       // 8 rects from SHP dimensions
        DAT_00b0fbe0 = true;
    }
    // ... draw ...
}
```

This means changing resolution at runtime would NOT recalculate layout rects unless
the flag is reset. The layout is computed once on first draw.

---

## 12. Complete SHP Inventory

### Right Panel (from NTRLMD.MIX / NEUTRAL.MIX)

| SHP | Tiling | Frames Used | Role |
|---|---|---|---|
| SDTP.SHP | No | 0 (cap), 1 (highlight) | Top cap |
| SDBTNBKGD.SHP | Vertical × N | 0 | Cameo slot background |
| SDBTNANM.SHP | Vertical × N | 10 | Button animation overlay |
| SDBTM.SHP | No (fills remainder) | 0 | Bottom cap |
| MNSCRNS.SHP | No | 0 | Screen corner (640) |
| MNSCRNL.SHP | No | 0 | Screen corner (800+) |
| LWSCRNS.SHP | No | 0 | Lower corner (640) |
| LWSCRNL.SHP | No | 0 | Lower corner (800+) |
| SDMPBTN.SHP | No | 0 | Minimap button |
| SDWRNTMP.SHP | No | 0 | Warning template |

### Left/Top Panel (from NTRLMD.MIX)

| SHP | Tiling | Frames Used | Role |
|---|---|---|---|
| BKGDSM(Y).SHP | No | 0 | Background (640) |
| BKGDMD(Y).SHP | No | 0 | Background (800) |
| BKGDLG(Y).SHP | No | 0 | Background (>800) |
| CREDITS.SHP | No | 0 | Credits display bg |
| TOP.SHP | No | 0 | Radar frame top |
| RADAR(Y).SHP | No | 0 | Radar housing |
| SIDE1.SHP | No | 0 | Below radar strip |
| SIDE2B.SHP | Vertical × N | 0 | Power bar area tile |
| SIDE3.SHP | No | 0 | Strip piece 3 |
| ADDON.SHP | No | 0 | Expansion area |
| LSPACER.SHP | No | 0 | Bottom bar left spacer |
| LENDCAP.SHP | No | 2 | Bottom bar left cap |
| BTTNBKGD.SHP | Horizontal × N | 0 | Bottom bar button tile |
| RENDCAP.SHP | No | 0 | Bottom bar right cap |
| SIDEBTTN.SHP | No | — | Side button |

### Side-Specific (from SIDEC*.MIX)

| SHP | Purpose |
|---|---|
| SIDE1/2/3.SHP (in SidebarClass::Draw) | Cameo strip frame (top/tile/bottom) |
| TAB00-03.SHP | Tab buttons (themed per side) |
| GCLOCK2.SHP | Progress clock (same art, different palette) |
| SELL.SHP, REPAIR.SHP | Button icons |
| R-UP.SHP, R-DN.SHP | Scroll arrows |
| DARKEN.SHP | Unbuildable overlay |

---

## 13. Key Function Reference

| Address | Name | Purpose |
|---|---|---|
| `0x00621e90` | WM_PAINT handler | Master sidebar compositor (mode dispatch) |
| `0x0072ddb0` | SidebarSurface::Init | Loads 10 SHPs + 3 palettes, lazy init guard |
| `0x0072eb50` | LoadRightPanelSHPs | Loads right-side panel SHP files |
| `0x0072ec70` | CalcRightPanelRects | Computes 8 layout rects from SHP dimensions |
| `0x0072e450` | DrawRightSidebarFrame | Draws right panel (SDTP + tiles + SDBTM) |
| `0x0072e2d0` | FillMargins | Fills black margins for >800×600 |
| `0x0072e730` | DrawBackgroundOverlay | Draws MNSCRNS/MNSCRNL corner |
| `0x0072fa10` | LoadLeftPanelSHPs | Loads left/top panel SHPs (YR branching) |
| `0x0072fc60` | CalcLeftPanelRects | Computes 14+ layout rects for left panel |
| `0x0072f540` | DrawLeftPanel | Draws all left panel pieces (observer path) |
| `0x006d0a30` | DrawSidebarGameplay | Main in-game sidebar draw (credits + buttons + cameos) |
| `0x006d0e60` | DrawCreditsBG | Repaints CREDITS.SHP before text overlay |
| `0x00534fa0` | InitSideMixFiles | Loads SIDEC%02d.MIX per side (Yuri→Soviet) |
| `0x0072f350` | LoadSidebarPalettes | Loads side-aware palettes from active MIX |
| `0x007681e0` | LoadUIArt | Reads UIMD.INI, Art_800 override for >800px |
| `0x0072d460` | LoadRadarBGs | Loads side×resolution radar backgrounds |
| `0x006a5840` | LoadCameoSHPs | Loads TAB, GCLOCK2, SELL, etc. from side MIX |

---

## 14. Summary: How the Sidebar Is Built

**The sidebar construction is logic-driven, not hardcoded.** The engine:

1. Loads SHP files from side-specific MIX archives
2. Reads each SHP's header to get its pixel dimensions
3. Computes layout rectangles by chaining pieces top-to-bottom (right panel) or stacking vertically then horizontally (left panel)
4. Fills gaps via tiling: SDBTNBKGD vertically for the cameo area, SIDE2B vertically for the power bar, BTTNBKGD horizontally for the bottom button bar
5. Bottom caps fill whatever height remains after tiling
6. For >800×600, adds centering margins but keeps the 800×600 content area
7. Different sides get different art via MIX file swapping (same filenames, different art)
8. Different resolutions get different corner/background SHPs (SM/MD/LG variants)
9. The 158px sidebar width is the ONE hardcoded value (`DAT_00886f94 = 0x9E`)
