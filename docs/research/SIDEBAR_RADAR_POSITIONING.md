# Sidebar & Radar/Minimap Positioning -- Ghidra Research Report

## Key Constants

| Address       | Name                | Value  | Description                          |
|---------------|---------------------|--------|--------------------------------------|
| `0x008a00a4`  | screen_width        | varies | Screen width in pixels (640/800/1024+) |
| `0x008a00a8`  | screen_height       | varies | Screen height in pixels              |
| `0x007f5bf8`  | SIDEBAR_WIDTH       | 0xA8 (168) | Sidebar panel width in pixels   |
| `0x007f5bfc`  | SIDEBAR_HEIGHT      | varies | From SHP dimensions                  |
| `0x00a8b230`  | g_ScenarioClass_Instance | ptr | ScenarioClass singleton; +0x34B8 = side index (0=Allied,1=Soviet,2=Yuri) (corrected 2026-05-29: was HouseClass_Player — binary RadarClass__Init_For_House reads `g_ScenarioClass_Instance + 0x34b8`, not a HouseClass field; ROOT_CAUSE: RTTI_LABEL_DRIFT; via decompile_function 0x00652e90) |

## Global SHP Pointers -- Complete Map

### Radar Background SHPs (FUN_0072d460 @ 0x72d460)

Three globals are set based on side and resolution:

| Global        | Role                        |
|---------------|-----------------------------|
| `DAT_00b0fb34`| Radar frame open SHP        |
| `DAT_00b0fb00`| Radar background (gameplay) |
| `DAT_00b0fb30`| Radar frame close SHP       |

**Complete filename table (side x resolution):**

| Side    | Res=640               | Res=800+              |
|---------|-----------------------|-----------------------|
| Allied  | ASCRBKSM.SHP / ASCRTSM.SHP / ASCRASM.SHP | ASCRBKMD.SHP / ASCRTMD.SHP / ASCRAMD.SHP |
| Soviet  | SSCRBKSM.SHP / SSCRTSM.SHP / SSCRASM.SHP | SSCRBKMD.SHP / SSCRTMD.SHP / SSCRAMD.SHP |
| Yuri    | SSCRBKSM.SHP / SSCRTSM.SHP / SSCRASM.SHP (reuses Soviet SM) | SYCRBKMD.SHP / SYCRTMD.SHP / SYCRAMD.SHP |

**Naming convention:** `{side}SCR{type}{size}.SHP`
- Side: `A`=Allied, `SS`=Soviet, `SY`=Yuri
- Type: `BK`=Background, `T`=Transition (frame open), `A`=Animation (frame close)
- Size: `SM`=Small (640), `MD`=Medium (800+)

**Mapping to globals (for Allied, 640):**
```
DAT_00b0fb34 = ASCRBKSM.SHP   (ptr at [0x844c58])  -> stored with flag at 0xb0fc70
DAT_00b0fb00 = ASCRTSM.SHP    (ptr at [0x844c60])  -> stored with flag at 0xb0fc71
DAT_00b0fb30 = ASCRASM.SHP    (ptr at [0x844c68])  -> stored with flag at 0xb0fc72
```

### Radar Transition Movie SHP (FUN_0072d830 @ 0x72d830)

| Global        | Role                                |
|---------------|-------------------------------------|
| `DAT_00b0fb1c`| Side-specific minimap movie SHP     |

**Filename table:**

| Side    | Res=640          | Res=800+          |
|---------|------------------|-------------------|
| Allied  | MPASCRNS.SHP     | MPASCRNL.SHP      |
| Soviet  | MPSSCRNS.SHP     | MPSSCRNL.SHP      |
| Yuri    | MPSSCRNS.SHP (reuses Soviet) | MPYSCRNL.SHP |

### All Sidebar SHPs — MIX_LoadNeutral (FUN_0072fa10 @ 0x72fa10)

Loaded once (lazy-init from LeftPanel__Draw). Ghidra labels this `MIX_LoadNeutral` — it loads radar, credits, and all panel SHPs in one pass, not just the left panel. Most are resolution-independent; the first three depend on Yuri side: (corrected 2026-05-29: section header was "Left Panel SHPs" — function loads all sidebar SHPs; ROOT_CAUSE: MISLEADING; via decompile_function 0x0072fa10 + 0x0072f540)

| Global          | Non-Yuri SHP     | Yuri (side==2) SHP | Description                     |
|-----------------|------------------|--------------------|---------------------------------|
| `DAT_00b0fa68`  | BKGDLG.SHP       | BKGDLGY.SHP       | Left panel background (large)   |
| `DAT_00b0fac8`  | BKGDMD.SHP        | BKGDMDY.SHP       | Left panel background (medium)  |
| `DAT_00b0fad4`  | BKGDSM.SHP        | BKGDSMY.SHP       | Left panel background (small/640) |
| `DAT_00b0fa50`  | SIDEBTTN.SHP      | --                 | Side button strip               |
| `DAT_00b0f9ec`  | SIDE2B.SHP        | --                 | Side 2 button                   |
| `DAT_00b0fb08`  | RADAR.SHP         | --                 | Radar area (credits area)       |
| `DAT_00b0f9e0`  | TOP.SHP           | --                 | Top strip below credits         |
| `DAT_00b0fa70`  | CREDITS.SHP       | --                 | Credits display area            |
| `DAT_00b0fafc`  | BKGDLG.SHP (!!!) | RADARY.SHP         | Tiled fill strip (resolution-dependent) |
| `DAT_00b0fa00`  | RENDCAP.SHP       | --                 | Right end cap                   |
| `DAT_00b0fa8c`  | BTTNBKGD.SHP      | --                 | Button background               |
| `DAT_00b0fa48`  | LENDCAP.SHP       | --                 | Left end cap                    |
| `DAT_00b0fa3c`  | LSPACER.SHP       | --                 | Left spacer (bottom bar)        |
| `DAT_00b0fa90`  | SIDE1.SHP / SIDE2.SHP / SIDE3.SHP | -- | Side identifier strip      |
| `DAT_00b0faa8`  | ADDON.SHP         | --                 | Addon tiled strip               |
| `DAT_00b0fabc`  | LWSCRNL.SHP/LWSCRNS.SHP | --          | Lower screen edge               |

### Right Panel / Screen Edge SHPs (FUN_0072eb50 @ 0x72eb50)

(corrected 2026-05-29: several SHP file names were WRONG — binary `Sidebar_RightPanel_SHP_Loading` uses Ghidra-labelled globals; ROOT_CAUSE: RTTI_LABEL_DRIFT; via decompile_function 0x0072eb50 + 0x0072e450)

| Global              | Ghidra label / SHP role                                    | Description                                      |
|---------------------|------------------------------------------------------------|--------------------------------------------------|
| `g_SDBTNANM_SHP`    | g_SDBTNANM_SHP (SHP name unknown — drawn frame 10 as cap) | Right panel left-edge cap (drawn when !draw_caps)|
| `DAT_00b0f9dc`      | FSSLG.SHP (unverified name)                                | Right edge large-res piece                       |
| `DAT_00b0fac0`      | FSSSM.SHP (unverified name)                                | Right edge small-res piece                       |
| `DAT_00b0fb50`      | FSBCLG.SHP (at 640 for radar bg height)                   | Full-screen bar cap (640 height variant)         |
| `DAT_00b0fa04`      | FSBCSM.SHP (at >640 for radar bg height)                  | Full-screen bar cap (>640 height variant)        |
| `g_SDTP_SHP`        | g_SDTP_SHP — drawn at DAT_00b0fc20 (top strip)            | Right panel top strip (was FSASM.SHP — WRONG)   |
| `g_SDBTNBKGD_SHP`  | g_SDBTNBKGD_SHP — tiled at DAT_00b0fc24 (body)           | Right panel body strip (was MNSCRNL.SHP — WRONG)|
| `DAT_00b0fa38`      | Bottom piece — drawn at DAT_00b0fc28                       | Right panel bottom piece (was MNSCRNS.SHP)       |
| `DAT_00b0fae8`      | SDTP.SHP — used at 640 for DAT_00b0fc2c                   | Side top (640 variant)                           |
| `DAT_00b0fa54`      | SDBTM.SHP — used at >640 for DAT_00b0fc2c                 | Side bottom (>640 variant)                       |

### Radar Buttons (RadarClass::Init_For_House @ 0x652e90)

| Global          | SHP File          |
|-----------------|-------------------|
| `DAT_00b048e0`  | DIPLOBTN.SHP      |
| `DAT_00b048ac`  | OPTBTN.SHP        |
| `DAT_00b04a38`  | = DAT_00b0fa68 (BKGDLG.SHP or BKGDLGY.SHP) -- radar frame animation |

---

## 2. FUN_0072e920 -- Draw Radar Background During Gameplay

```c
// Address: 0x0072e920
// Draws the radar background SHP during normal gameplay
void DrawRadarBackground(DSurface* surface) {
    int pos_x = DAT_00b0fc1c->x;   // radar_rect.x
    int pos_y = DAT_00b0fc1c->y;   // radar_rect.y

    // For screen widths >= 800, offset x by +80 pixels
    if (screen_width > 799) {       // 0x320 = 800
        pos_x += 0x50;             // +80 pixels
    }

    DrawSHP(DAT_00b0fb00,          // radar background SHP (ASCR*SM/MD.SHP)
            0,                      // frame 0
            &pos,                   // {pos_x, pos_y}
            surface,                // draw surface
            0x400,                  // flags
            0, 0, 0, 1000, 0, 0, 0, 0, 0);
}
```

**Key insight:** At 800+ resolution, the radar background is shifted right by 80px (0x50) relative to the radar rect origin. At 640, it draws at the radar rect origin directly.

---

## 3. FUN_0072e9f0 -- Radar Frame Open Transition

```c
// Address: 0x0072e9f0
// Draws the radar "opening" transition frame
void DrawRadarFrameOpen(DSurface* surface) {
    FUN_0072e2d0();                // Clear/prepare right-side screen rects
    FUN_0072e450(0);               // Draw right panel background

    int pos_x = DAT_00b0fc1c->x;
    int pos_y = DAT_00b0fc1c->y;

    DrawSHP(g_RadarFrameOpen_SHP,  // frame-open SHP (ASCRBK*.SHP) -- corrected 2026-05-29: was DAT_00b0fb34; binary uses g_RadarFrameOpen_SHP; ROOT_CAUSE: RTTI_LABEL_DRIFT; via decompile_function 0x0072e9f0
            0,                      // frame 0
            &pos,                   // radar rect origin
            surface, 0x400, 0, 0, 0, 1000, 0, 0, 0, 0, 0);
}
```

## 4. FUN_0072ead0 -- Radar Frame Close Transition

```c
// Address: 0x0072ead0
// Draws the radar "closing" transition frame
void DrawRadarFrameClose(DSurface* surface) {
    FUN_0072e2d0();                // Clear/prepare right-side screen rects
    FUN_0072e450(0);               // Draw right panel background

    int pos_x = DAT_00b0fc1c->x;
    int pos_y = DAT_00b0fc1c->y;

    DrawSHP(g_MinimapMovie_SHP,    // minimap movie SHP (MP*SCRN*.SHP) -- corrected 2026-05-29: was DAT_00b0fb1c; binary uses g_MinimapMovie_SHP; ROOT_CAUSE: RTTI_LABEL_DRIFT; via decompile_function 0x0072ead0
            0,                      // frame 0
            &pos,                   // radar rect origin
            surface, 0x400, 0, 0, 0, 1000, 0, 0, 0, 0, 0);
}
```

---

## 5. FUN_0072fc60 -- Left Panel Layout Rect Calculator (COMPLETE)

This is the master layout function. It computes 16 rects (Rect = {x, y, width, height}) for all left-panel SHP pieces.

```c
// Address: 0x0072fc60
// param_1 = screen_width, param_2 = screen_height
void ComputeLeftPanelRects(int screen_w, int screen_h) {

    // ===== SELECT BACKGROUND SHP BASED ON RESOLUTION =====
    // Three variants: SM (640), MD (800), LG (1024+)
    SHP* bkgd_shp;
    if (screen_w == 640)       bkgd_shp = DAT_00b0fad4;  // BKGDSM.SHP / BKGDSMY.SHP
    else if (screen_w == 800)  bkgd_shp = DAT_00b0fac8;  // BKGDMD.SHP / BKGDMDY.SHP
    else                       bkgd_shp = DAT_00b0fa50;  // SIDEBTTN.SHP (large res fallback)

    short bkgd_w = bkgd_shp->width;   // offset +2 in SHP header
    short bkgd_h = bkgd_shp->height;  // offset +4 in SHP header

    // ===== RECT: DAT_00b0fc30 -- Top-left background =====
    // Positioned at screen origin (0,0)
    DAT_00b0fc30 = { x=0, y=0, w=bkgd_w, h=bkgd_h };

    // ===== RECT: DAT_00b0fc34 -- RADAR.SHP strip =====
    int radar_w = DAT_00b0fb08->width;   // RADAR.SHP width
    int radar_h = DAT_00b0fb08->height;  // RADAR.SHP height
    int right_x = screen_w - radar_w;    // Right-aligned
    DAT_00b0fc34 = { x=right_x, y=0, w=radar_w, h=radar_h };

    // ===== RECT: DAT_00b0fc38 -- TOP.SHP =====
    short top_h = DAT_00b0f9e0->height;  // TOP.SHP height
    int top_y = radar_h;                  // Immediately below RADAR.SHP
    DAT_00b0fc38 = { x=right_x, y=top_y, w=radar_w, h=top_h };

    // ===== RECT: DAT_00b0fc3c -- CREDITS.SHP (BKGDLG) =====
    int credits_y = top_y + top_h;        // Below TOP.SHP
    short credits_h = DAT_00b0fa68->height;  // BKGDLG(Y).SHP height
    DAT_00b0fc3c = { x=right_x, y=credits_y, w=radar_w, h=credits_h };

    // ===== RECT: DAT_00b0fc44 -- CREDITS.SHP display =====
    int cred_y = credits_y + credits_h;
    short cred_h = DAT_00b0fa70->height;  // CREDITS.SHP height
    DAT_00b0fc44 = { x=right_x, y=cred_y, w=radar_w, h=cred_h };

    // ===== RECT: DAT_00b0fc48 -- Tiled fill strip =====
    int fill_y = cred_y + cred_h;
    short fill_h = DAT_00b0fafc->height;  // fill strip height
    DAT_00b0fc48 = { x=right_x, y=fill_y, w=radar_w, h=fill_h };

    // ===== RECT: DAT_00b0fc4c -- BTTNBKGD.SHP =====
    short bttn_h = DAT_00b0fa8c->height;  // BTTNBKGD.SHP height
    DAT_00b0fc4c = { x=right_x, y=fill_y, w=radar_w, h=bttn_h };
    // (y updated after tile count computed)

    // ===== RECT: DAT_00b0fc50 -- LENDCAP.SHP =====
    short lend_h = DAT_00b0fa48->height;  // LENDCAP.SHP height
    DAT_00b0fc50 = { x=right_x, y=fill_y, w=radar_w, h=lend_h };
    // (y updated after tile count computed)

    // ===== COMPUTE VERTICAL TILE COUNT =====
    // How many fill strips needed to fill remaining vertical space
    DAT_00b0fadc = (screen_h
                   - DAT_00b0fc4c->h       // BTTNBKGD height
                   - DAT_00b0fc44->h        // CREDITS height
                   - DAT_00b0fc3c->h        // BKGDLG height
                   - DAT_00b0fc38->h        // TOP height
                   - DAT_00b0fc34->h        // RADAR height
                   ) / DAT_00b0fc48->h;     // / fill strip height

    // Reposition pieces after tiling:
    fill_y += DAT_00b0fadc * DAT_00b0fc48->h;
    DAT_00b0fc4c->y = fill_y;                           // BTTNBKGD below tiles
    DAT_00b0fc50->y = DAT_00b0fc4c->h + fill_y;         // LENDCAP below BTTNBKGD

    // ===== BOTTOM BAR RECTS (screen_h - 32) =====
    int bottom_y = screen_h - 0x20;  // 32 pixels from bottom
    int bottom_w = DAT_00a8eb84 - 0xA8;  // screen_w - 168 (sidebar width)

    // RECT: DAT_00b0fc58 -- Bottom bar fill
    DAT_00b0fc58 = { x=0, y=bottom_y, w=bottom_w, h=0x20 };

    // RECT: DAT_00b0fc5c -- LSPACER.SHP
    short lspc_w = DAT_00b0fa3c->width;
    short lspc_h = DAT_00b0fa3c->height;
    DAT_00b0fc5c = { x=0, y=bottom_y, w=lspc_w, h=lspc_h };

    // RECT: DAT_00b0fc60 -- SIDE*.SHP (side identifier)
    short side_w = DAT_00b0fa90->width;
    short side_h = DAT_00b0fa90->height;
    DAT_00b0fc60 = { x=0, y=bottom_y, w=side_w, h=side_h };

    // RECT: DAT_00b0fc64 -- SIDE*.SHP (second position, for non-side bar)
    DAT_00b0fc64 = { x=0, y=bottom_y, w=side_w, h=side_h };

    // RECT: DAT_00b0fc68 -- ADDON.SHP (tiled horizontally)
    short addon_w = DAT_00b0faa8->width;
    short addon_h = DAT_00b0faa8->height;
    DAT_00b0fc68 = { x=0, y=bottom_y, w=addon_w, h=addon_h };

    // RECT: DAT_00b0fc6c -- LWSCRNL/S.SHP (lower screen edge)
    short lw_w = DAT_00b0fabc->width;
    short lw_h = DAT_00b0fabc->height;
    DAT_00b0fc6c = { x=0, y=bottom_y, w=lw_w, h=lw_h };

    // ===== COMPUTE HORIZONTAL TILE COUNT FOR BOTTOM BAR =====
    DAT_00b0f9e4 = (screen_w - side_w - radar_w - lw_w) / addon_w;

    // ===== REPOSITION BOTTOM BAR PIECES RIGHT-TO-LEFT =====
    // LWSCRNL.SHP anchored relative to right panel
    DAT_00b0fc6c->x = right_x - lw_w;

    // ADDON tiled strip
    DAT_00b0fc68->x = DAT_00b0fc6c->x - addon_w * DAT_00b0f9e4;

    // SIDE*.SHP strip
    DAT_00b0fc60->x = DAT_00b0fc68->x - side_w;

    // Second side position
    DAT_00b0fc64->x = DAT_00b0fc6c->x - DAT_00b0fc64->w;

    // DAT_00b0fa78 = gap computation

    // ===== COMBINED RECTS =====
    // DAT_00b0fc40 = combined TOP + BKGDLG rect
    DAT_00b0fc40 = { x=DAT_00b0fc38->x, y=DAT_00b0fc38->y,
                     w=DAT_00b0fc38->w, h=DAT_00b0fc3c->h + DAT_00b0fc38->h };

    // DAT_00b0fc54 = combined CREDITS + tile fill + BTTNBKGD rect
    DAT_00b0fc54 = { x=DAT_00b0fc44->x, y=DAT_00b0fc44->y,
                     w=DAT_00b0fc44->w,
                     h=DAT_00b0fc48->h * DAT_00b0fadc + DAT_00b0fc4c->h + DAT_00b0fc44->h };
}
```

---

## 6. FUN_0072f540 -- Left Panel Draw Function (COMPLETE)

```c
// Address: 0x0072f540
// Draws all left-panel SHP pieces in order. Called with palette and clip rect.
void DrawLeftPanel(DSurface* this_surface, Rect* clip_rect, bool has_sidebar) {
    // Lazy init
    if (!DAT_00b0fc0c) {
        FUN_0072fa10();    // Load all left-panel SHPs
        FUN_0072fbc0();    // Load palette data
        FUN_0072fc60(screen_w, screen_h);  // Compute layout rects
        DAT_00b0fc0c = true;
    }

    // Virtual call to prepare surface
    this_surface->vtable[6](0);  // offset 0x18

    // 1. BACKGROUND (resolution-dependent)
    //    Position from DAT_00b0fc30 = {0, 0, w, h}
    if (screen_width == 640)
        DrawSHP(DAT_00b0fad4, 0, &DAT_00b0fc30->xy, clip, ...);  // BKGDSM(Y).SHP
    else if (screen_width == 800)
        DrawSHP(DAT_00b0fac8, 0, &DAT_00b0fc30->xy, clip, ...);  // BKGDMD(Y).SHP
    else
        DrawSHP(DAT_00b0fa50, 0, &DAT_00b0fc30->xy, clip, ...);  // SIDEBTTN.SHP
    // Palette = DAT_00b0fbf0

    // 2. RADAR.SHP strip
    //    Position from DAT_00b0fc34 = {screen_w - radar_w, 0, ...}
    DrawSHP(DAT_00b0fb08, 0, &DAT_00b0fc34->xy, clip, ...);
    // Palette = DAT_00b0fbe8

    // 3. TOP.SHP
    //    Position from DAT_00b0fc38 = {right_x, radar_h, ...}
    DrawSHP(DAT_00b0f9e0, 0, &DAT_00b0fc38->xy, clip, ...);
    // Palette = DAT_00b0fbe8

    // 4. BKGDLG(Y).SHP (radar frame area)
    //    Position from DAT_00b0fc3c = {right_x, radar_h + top_h, ...}
    DrawSHP(DAT_00b0fa68, 0, &DAT_00b0fc3c->xy, clip, ...);
    // Palette = DAT_00b0fbf8

    // 5. CREDITS.SHP
    //    Position from DAT_00b0fc44 = {right_x, radar_h + top_h + bkgdlg_h, ...}
    DrawSHP(DAT_00b0fa70, 0, &DAT_00b0fc44->xy, clip, ...);
    // Palette = DAT_00b0fbe8

    // 6. TILED FILL STRIPS (DAT_00b0fadc repetitions)
    //    Position from DAT_00b0fc48, incrementing Y by strip height
    for (int i = 0; i < DAT_00b0fadc; i++) {
        DrawSHP(DAT_00b0fa00, 0, &pos, clip, ...);  // RENDCAP.SHP
        pos.y += DAT_00b0fc48->h;
    }
    // Palette = DAT_00b0fbe8

    // 7. BTTNBKGD.SHP
    //    Position from DAT_00b0fc4c
    DrawSHP(DAT_00b0fa8c, 0, &DAT_00b0fc4c->xy, clip, ...);
    // Palette = DAT_00b0fbe8

    // 8. LENDCAP.SHP
    //    Position from DAT_00b0fc50
    DrawSHP(DAT_00b0fa48, 0, &DAT_00b0fc50->xy, clip, ...);
    // Palette = DAT_00b0fbe8

    // 9. LSPACER.SHP (bottom bar, with modified clip rect)
    //    Position from DAT_00b0fc5c
    //    Clip rect is modified: width = DAT_00886fb8 - 0xA8 (screen-sidebar)
    Rect bottom_clip = { clip->x, clip->y, DAT_00886fb8 - 0xA8, clip->h };
    DrawSHP(DAT_00b0fa3c, 0, &DAT_00b0fc5c->xy, &bottom_clip, ...);
    // Palette = DAT_00b0fbe8

    // 10. SIDE*.SHP (side identifier, frame 2)
    if (has_sidebar) {
        // With sidebar visible: draw side + tiled ADDON
        DrawSHP(DAT_00b0fa90, 2, &DAT_00b0fc60->xy, clip, ...);

        for (int i = 0; i < DAT_00b0f9e4; i++) {
            DrawSHP(DAT_00b0faa8, 0, &pos, clip, ...);  // ADDON.SHP
            pos.x += DAT_00b0fc68->w;
        }
    } else {
        // Without sidebar: draw side only
        DrawSHP(DAT_00b0fa90, 2, &DAT_00b0fc64->xy, clip, ...);
    }

    // 11. LWSCRNL/S.SHP (lower screen edge, final piece)
    DrawSHP(DAT_00b0fabc, 0, &DAT_00b0fc6c->xy, clip, ...);
}
```

**Left panel vertical stack (top to bottom at right_x = screen_w - RADAR.SHP.width):**
```
Y=0:                    RADAR.SHP          (DAT_00b0fc34)
Y=radar_h:              TOP.SHP            (DAT_00b0fc38)
Y=radar_h+top_h:        BKGDLG(Y).SHP     (DAT_00b0fc3c)  <-- radar frame area
Y+=bkgdlg_h:            CREDITS.SHP        (DAT_00b0fc44)
Y+=credits_h:           FILL STRIP x N     (DAT_00b0fc48)  <-- tiled vertically
Y+=fill*N:              BTTNBKGD.SHP       (DAT_00b0fc4c)
Y+=bttnbkgd_h:          LENDCAP.SHP        (DAT_00b0fc50)

Bottom bar (Y = screen_h - 32):
X=0:                    LSPACER.SHP        (DAT_00b0fc5c)
X=...:                  SIDE*.SHP          (DAT_00b0fc60)
X=...:                  ADDON.SHP x N      (DAT_00b0fc68)  <-- tiled horizontally
X=right_x-lw_w:         LWSCRNL.SHP        (DAT_00b0fc6c)
```

---

## 7. FUN_00652e90 -- RadarClass::Init_For_House (COMPLETE)

```c
// Address: 0x00652e90
// this = RadarClass (ECX)
// this+0x11E4 = sidebar_x (left edge of sidebar on screen)
// this+0x11E8 = sidebar_y (top edge of sidebar on screen)
// this+0x11F0 = minimap_center_x offset
void RadarClass::Init_For_House() {
    Debug("RadarClass::Init_For_House...");
    base_class_init();  // vtable+0xC8

    int side = Player->Side;  // DAT_00a8b230 + 0x34B8

    if (side == 0) {  // Allied (side 0)
        this->minimap_center_x = (SIDEBAR_WIDTH - 0x90) / 2 + 4;
        //   = (168 - 144) / 2 + 4 = 12 + 4 = 16

        radar_btn_x   = this->sidebar_x + 0x0B;   // +11
        radar_btn_y   = this->sidebar_y + 0x04;    // +4
        options_btn_x = this->sidebar_x + 0x53;    // +83
    } else {  // Soviet or Yuri (side != 0)
        this->minimap_center_x = (SIDEBAR_WIDTH - 0x91) / 2 + 5;
        //   = (168 - 145) / 2 + 5 = 11 + 5 = 16 (rounds differently)

        radar_btn_x   = this->sidebar_x + 0x0E;   // +14
        radar_btn_y   = this->sidebar_y + 0x05;    // +5
        options_btn_x = this->sidebar_x + 0x56;    // +86
    }

    // Store button positions
    DAT_00b04a00 = radar_btn_x;      // Diplomacy/Briefing button X
    DAT_00b04a04 = radar_btn_y;      // Both buttons share Y = sidebar_y + 4 or 5
    DAT_00b048c8 = options_btn_x;    // Options button X
    DAT_00b048cc = radar_btn_y;      // Options button Y (same Y)
    DAT_00b04a1c = this->minimap_center_x;

    // Load button SHPs
    DAT_00b048e0 = LoadSHP("DIPLOBTN.SHP");  // [0x8391F4]
    DAT_00b048ac = LoadSHP("OPTBTN.SHP");    // [0x8391F8]

    // Get button dimensions from SHP headers
    DAT_00b04a08 = DAT_00b048e0->width;   // DIPLOBTN width
    DAT_00b04a0c = DAT_00b048e0->height;  // DIPLOBTN height
    DAT_00b048d0 = DAT_00b048ac->width;   // OPTBTN width
    DAT_00b048d4 = DAT_00b048ac->height;  // OPTBTN height

    // Radar frame animation SHP = left panel BKGDLG(Y).SHP
    DAT_00b04a38 = DAT_00b0fa68;

    // Compute background color from palette
    this->bg_color = make_color(DAT_00b0fa1c);
}
```

**Key positions relative to sidebar origin (this+0x11E4, this+0x11E8):**

| Element         | Allied X offset | Allied Y offset | Soviet/Yuri X | Soviet/Yuri Y |
|-----------------|-----------------|-----------------|---------------|---------------|
| Diplomacy btn   | +11 (0x0B)      | +4              | +14 (0x0E)    | +5            |
| Options btn     | +83 (0x53)      | +4              | +86 (0x56)    | +5            |
| Minimap center  | +16             | --              | +16           | --            |

---

## 8. Minimap Rendering Area

The minimap is rendered inside the BKGDLG(Y).SHP frame area. From `FUN_00653100`:

```c
// Minimap draw position:
//   X = this->sidebar_x (offset 0x11E4)
//   Y = this->sidebar_y_for_radar (offset 0x11EC)

// The radar frame SHP (DAT_00b04a38 = BKGDLG(Y).SHP) is drawn at:
DrawSHP(DAT_00b04a38, frame_index,
        {this->sidebar_x, this->sidebar_y_for_radar}, ...);

// After drawing, the dirty rect is set to:
this->dirty_rect = {
    x: this->sidebar_x,        // 0x11E4
    y: this->sidebar_y_for_radar,  // 0x11EC
    w: SIDEBAR_WIDTH,           // 168 (0xA8)
    h: 0x6E                     // 110 pixels
};
```

**The chrome frame (BKGDLG.SHP) area is 168 x 110 pixels (SIDEBAR_WIDTH x 0x6E).**
The actual minimap content area within the chrome is **140 x 108 pixels** (0x8C x 0x6C),
inset at (16, 49) on the sidebar surface. See `RADAR_CHROME_COMPOSITING.md` for
verified margin math. The terrain fills the full rectangular minimap surface — cells
tile seamlessly because `IsValidCell` bounds are rectangular in the radar's isometric
coordinate space.

The DIPLOBTN and OPTBTN buttons are positioned at:
- `DAT_00b04a00, DAT_00b04a04` = top-left of DIPLOBTN (inside the radar frame)
- `DAT_00b048c8, DAT_00b048cc` = top-left of OPTBTN (inside the radar frame)

Hit testing from `FUN_00653850`:
```c
// DIPLOBTN hit test:
if (click_x >= radar_btn_x + scroll_offset &&
    click_x < radar_btn_x + scroll_offset + diplobtn_width &&
    click_y >= radar_btn_y &&
    click_y < radar_btn_y + diplobtn_height) { ... }

// OPTBTN hit test:
if (click_x >= options_btn_x + scroll_offset &&
    click_x < options_btn_x + scroll_offset + optbtn_width &&
    click_y >= optbtn_btn_y &&
    click_y < optbtn_btn_y + optbtn_height) { ... }
```

---

## 9. Right Panel (Screen Edge) Drawing -- FUN_0072e450

```c
// Address: 0x0072e450
// Draws the right-side screen edge pieces (between game area and sidebar)
void DrawRightPanel(DSurface* this_surface, Rect* clip, bool draw_caps) {
    // Lazy init
    if (!DAT_00b0fbe0) {
        LoadNeutralMix();          // Load NTRLMD.MIX + NEUTRAL.MIX
        FUN_0072eb50();            // Load right-panel SHPs
        LoadPalettes();
        FUN_0072ec70(screen_w, screen_h);  // Compute right-panel rects
        DAT_00b0fbe0 = true;
    }

    FUN_0072e2d0();  // Clear screen borders

    // Clamp to 800x600 max for drawing
    int clip_w = clip->w > 800 ? (clip->w - 800)/2 + 800 : clip->w;
    int clip_h = clip->h > 600 ? (clip->h - 600)/2 + 600 : clip->h;

    // 1. g_SDTP_SHP (top strip) -- corrected 2026-05-29: was FSASM.SHP; binary uses g_SDTP_SHP; ROOT_CAUSE: RTTI_LABEL_DRIFT; via decompile_function 0x0072e450
    DrawSHP(g_SDTP_SHP, 0, &DAT_00b0fc20->xy, &clip, ...);

    // 2. g_SDBTNBKGD_SHP tiled vertically (DAT_00b0fa20 times) -- corrected 2026-05-29: was MNSCRNL.SHP; binary uses g_SDBTNBKGD_SHP; ROOT_CAUSE: RTTI_LABEL_DRIFT; via decompile_function 0x0072e450
    for (int i = 0; i < DAT_00b0fa20; i++) {
        DrawSHP(g_SDBTNBKGD_SHP, 0, &pos, &clip, ...);
        pos.y += DAT_00b0fc24->h;
    }

    // 3. Left-edge caps (if draw_caps == false)
    if (!draw_caps) {
        for (int i = 0; i < DAT_00b0fa20; i++) {
            DrawSHP(g_SDBTNANM_SHP, 10, &DAT_00b0fc10->xy, &clip, ...);  // corrected 2026-05-29: was FSBKGDSM.SHP; binary uses g_SDBTNANM_SHP; ROOT_CAUSE: RTTI_LABEL_DRIFT; via decompile_function 0x0072e450
            pos.y += DAT_00b0fc10->h;
        }
    }

    // 4. MNSCRNS.SHP (bottom piece)
    DrawSHP(DAT_00b0fa38, 0, &DAT_00b0fc28->xy, &clip, ...);

    // 5. SDTP/SDBTM.SHP (side bottom edge)
    if (screen_width == 640)
        DrawSHP(DAT_00b0fae8, 0, &DAT_00b0fc2c->xy, &clip, ...);
    else
        DrawSHP(DAT_00b0fa54, 0, &DAT_00b0fc2c->xy, &clip, ...);
}
```

---

## 10. FUN_0072ec70 -- Right Panel Rect Calculator (COMPLETE)

```c
// Address: 0x0072ec70
// param_1 = screen_width, param_2 = screen_height
void ComputeRightPanelRects(int screen_w, int screen_h) {

    // ===== CENTERING OFFSETS FOR >800x600 =====
    int offset_x = 0, offset_y = 0;
    int effective_w = screen_w;
    int effective_h = screen_h;

    if (screen_w > 1023) {  // >1024
        offset_x = (screen_w - 800) / 2;
        effective_w = screen_w - offset_x;
    }
    if (screen_h > 767) {  // >768
        offset_y = (screen_h - 600) / 2;
        effective_h = screen_h - offset_y * 2;
    }

    // ===== SELECT FULLSCREEN BACKGROUND =====
    SHP* fs_shp;
    if (screen_w == 640)
        fs_shp = DAT_00b0fb50;   // FSBCLG.SHP
    else
        fs_shp = DAT_00b0fa04;   // FSBCSM.SHP

    SHP* fs_h_shp;
    if (screen_h == 480)
        fs_h_shp = DAT_00b0fb50;
    else
        fs_h_shp = DAT_00b0fa04;

    // ===== RECT: DAT_00b0fc1c -- RADAR BACKGROUND =====
    // This is THE key rect - where the radar/minimap background sits
    DAT_00b0fc1c = { x=offset_x, y=offset_y,
                     w=fs_shp->width, h=fs_h_shp->height };

    // ===== RECT: DAT_00b0fc20 -- g_SDTP_SHP strip (right edge top) =====
    // corrected 2026-05-29: was FSASM.SHP / DAT_00b0faf8; binary uses g_SDTP_SHP for both this rect and the draw call; ROOT_CAUSE: RTTI_LABEL_DRIFT; via decompile_function 0x0072ec70 + 0x0072e450
    int strip_w = g_SDTP_SHP->width;
    int strip_h = g_SDTP_SHP->height;
    int strip_x = effective_w - strip_w;
    DAT_00b0fc20 = { x=strip_x, y=offset_y, w=strip_w, h=strip_h };

    // ===== RECT: DAT_00b0fc18 -- FSSSM strip =====
    short fss_h = DAT_00b0fac0->height;  // FSSSM.SHP height (unverified name)
    DAT_00b0fc18 = { x=strip_x, y=offset_y, w=strip_w, h=fss_h };

    // ===== RECT: DAT_00b0fc24 -- g_SDBTNBKGD_SHP tiled (right edge body) =====
    // corrected 2026-05-29: was MNSCRNL.SHP / DAT_00b0fa74; binary uses g_SDBTNBKGD_SHP; ROOT_CAUSE: RTTI_LABEL_DRIFT; via decompile_function 0x0072ec70 + 0x0072e450
    int body_y = DAT_00b0fc20->h + offset_y;
    short body_h = g_SDBTNBKGD_SHP->height;
    DAT_00b0fc24 = { x=strip_x, y=body_y, w=strip_w, h=body_h };

    // ===== RECT: DAT_00b0fc10 -- g_SDBTNANM_SHP cap =====
    // corrected 2026-05-29: was FSBKGDSM.SHP / DAT_00b0fac4; binary uses g_SDBTNANM_SHP; ROOT_CAUSE: RTTI_LABEL_DRIFT; via decompile_function 0x0072ec70 + 0x0072e450
    int cap_x = DAT_00b0fc24->x + DAT_00b0fc24->w - g_SDBTNANM_SHP->width;
    DAT_00b0fc10 = { x=cap_x, y=body_y,
                     w=g_SDBTNANM_SHP->width, h=g_SDBTNANM_SHP->height };

    // ===== RECT: DAT_00b0fc14 -- FSSLG/FSSSM piece =====
    DAT_00b0fc14 = { x=effective_w - DAT_00b0f9dc->width,
                     y=body_y + DAT_00b0fc24->h - DAT_00b0f9dc->height,
                     w=DAT_00b0f9dc->width, h=DAT_00b0f9dc->height };

    // ===== VERTICAL TILE COUNT =====
    DAT_00b0fa20 = (effective_h - DAT_00b0fc20->h) / DAT_00b0fc24->h;

    // ===== RECT: DAT_00b0fc28 -- MNSCRNS (bottom piece) =====
    int bottom_y = DAT_00b0fc24->y + DAT_00b0fa20 * DAT_00b0fc24->h;
    DAT_00b0fc28 = { x=effective_w - DAT_00b0fa38->width,
                     y=bottom_y,
                     w=DAT_00b0fa38->width,
                     h=effective_h - (DAT_00b0fc20->h + DAT_00b0fa20 * DAT_00b0fc24->h) };

    // ===== RECT: DAT_00b0fc2c -- SDTP/SDBTM =====
    if (screen_w == 640) shp = DAT_00b0fae8;
    else                 shp = DAT_00b0fa54;
    if (screen_h == 480) shp2 = DAT_00b0fae8;
    else                 shp2 = DAT_00b0fa54;
    DAT_00b0fc2c = { x=offset_x, y=effective_h - shp2->height,
                     w=shp->width, h=shp2->height };
}
```

---

## 11. Complete Screen Layout Summary

### At 800x600 (standard YR resolution)

```
+---------------------------------------------+----+
|                                              |    |
|              GAME AREA                       | R  |  <-- Right edge: FSASM, MNSCRNL tiled, MNSCRNS
|              (800 - 168 = 632 wide)          | I  |
|                                              | G  |
|                                              | H  |
|                                              | T  |
|                                              |    |
|                                              |    |
+------+-------+------+------+--------+-------+----+
|LSPACR|SIDE*  |ADDON x N    |LWSCRNL |       |    |  <-- Bottom bar (Y = screen_h - 32 = 568)
+------+-------+------+------+--------+-------+----+

Right sidebar column (X = screen_w - RADAR.SHP.width):
+------------------+
| RADAR.SHP        |  Y=0, top
+------------------+
| TOP.SHP          |
+------------------+
| BKGDLG(Y).SHP   |  <-- Radar frame/minimap lives HERE
|  +------------+  |
|  | DIPLOBTN   |  |  X=sidebar_x+11/14, Y=sidebar_y+4/5
|  | OPTBTN     |  |  X=sidebar_x+83/86, Y=sidebar_y+4/5
|  | MINIMAP    |  |  168 x 110 pixels
|  +------------+  |
+------------------+
| CREDITS.SHP      |
+------------------+
| FILL STRIP x N   |  <-- Tiled to fill remaining height
+------------------+
| BTTNBKGD.SHP     |
+------------------+
| LENDCAP.SHP      |
+------------------+
```

### Key Rect Summary

| Rect Global     | Description            | X computation              | Y computation                    |
|-----------------|------------------------|----------------------------|----------------------------------|
| `DAT_00b0fc1c`  | Radar BG position      | center_offset_x            | center_offset_y                  |
| `DAT_00b0fc30`  | Top-left background    | 0                          | 0                                |
| `DAT_00b0fc34`  | RADAR.SHP              | screen_w - radar_w         | 0                                |
| `DAT_00b0fc38`  | TOP.SHP                | screen_w - radar_w         | radar_h                          |
| `DAT_00b0fc3c`  | BKGDLG(Y).SHP         | screen_w - radar_w         | radar_h + top_h                  |
| `DAT_00b0fc44`  | CREDITS.SHP            | screen_w - radar_w         | + bkgdlg_h                       |
| `DAT_00b0fc48`  | Fill strip             | screen_w - radar_w         | + credits_h                      |
| `DAT_00b0fc4c`  | BTTNBKGD.SHP           | screen_w - radar_w         | + fill_h * tile_count            |
| `DAT_00b0fc50`  | LENDCAP.SHP            | screen_w - radar_w         | + bttnbkgd_h                     |
| `DAT_00b0fc58`  | Bottom bar fill        | 0                          | screen_h - 32                    |
| `DAT_00b0fc5c`  | LSPACER.SHP            | 0                          | screen_h - 32                    |
| `DAT_00b0fc60`  | SIDE*.SHP              | computed right-to-left     | screen_h - 32                    |
| `DAT_00b0fc68`  | ADDON.SHP              | computed right-to-left     | screen_h - 32                    |
| `DAT_00b0fc6c`  | LWSCRNL.SHP            | right_x - lw_w             | screen_h - 32                    |

### Critical Positioning Values

- **Sidebar width** = 168 pixels (0xA8) -- constant at `0x007f5bf8`
- **Sidebar X** = `screen_width - sidebar_width` (stored at `this+0x11E4`)
- **Bottom bar height** = 32 pixels (0x20)
- **Bottom bar Y** = `screen_height - 32`
- **Bottom bar width** = `screen_width - 168` (gameplay area minus sidebar)
- **Minimap area** = 168 x 110 pixels within the BKGDLG frame
- **Radar BG offset at 800+** = +80px (0x50) added to X when drawing radar background
- **Vertical tile count** = `(screen_h - fixed_pieces_height) / fill_strip_height`
- **Horizontal tile count** = `(screen_w - side_w - radar_w - lwscrn_w) / addon_w`
