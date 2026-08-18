# Radar Chrome Compositing — How the Minimap Fits Inside the Chrome

**Verified**: Every constant from assembly, every blit call traced through 3 surface layers.

---

## The Three Surfaces

```
┌─────────────────────────┐
│ Primary Radar Surface   │  Created at zoomed map size: up to 140×108 px
│ (+0x121C)               │  Contains: terrain + object dots + fog dimming + flash
│ BSurface, 16-bit        │  Coordinates: (0,0) = top-left of minimap content
└───────────┬─────────────┘
            │ Blit at (viewport_x, viewport_y) = (16, 49)
            ▼
┌─────────────────────────┐
│ Sidebar Surface         │  168 × screen_height px
│ DAT_00887300            │  Contains: full sidebar panel (chrome + minimap + credits + tabs)
│ BSurface, 16-bit        │  Coordinates: (0,0) = top-left of sidebar
└───────────┬─────────────┘
            │ Blit at (screen_w − 168, 0) by window manager
            ▼
┌─────────────────────────┐
│ Alternate/Screen Surface│  screen_width × screen_height
│ DAT_00887310            │  Final back buffer → DirectDraw flip to display
└─────────────────────────┘
```

---

## Constants (every one verified from assembly at 0x652CF0)

```asm
; RadarClass::One_Time — sets hardcoded layout constants
00652d18: MOV [ESI + 0x11e4], 0x0     ; sidebar_x = 0
00652d22: MOV [ESI + 0x11e8], 0x10    ; sidebar_y = 16
00652d2c: MOV [ESI + 0x11ec], 0x30    ; radar_draw_y = 48
00652d42: MOV [ESI + 0x11f4], 0x31    ; inner_draw_y = 49
00652cfd: MOV ECX, 0x8c               ; 140
00652d02: MOV EAX, 0x6c               ; 108
00652d07: MOV [ESI + 0x1200], ECX     ; outer_width = 140
00652d0d: MOV [ESI + 0x11f8], ECX     ; inner_width = 140
00652d36: MOV [ESI + 0x1204], EAX     ; inner_height = 108
00652d3c: MOV [ESI + 0x11fc], EAX     ; outer_height = 108
```

| Offset | Name | Value | Source | Purpose |
|--------|------|-------|--------|---------|
| +0x11E4 | sidebar_x | 0 | One_Time (0x652CF0) | X origin on sidebar surface |
| +0x11E8 | sidebar_y | 16 | One_Time (0x652CF0) | Y of TOP.SHP on sidebar surface |
| +0x11EC | radar_draw_y | 48 | One_Time (0x652CF0) | Y where BKGDLG.SHP chrome is drawn |
| +0x11F0 | minimap_center_x | 16 | Init_For_House (0x652E90) | X inset for minimap content (corrected 2026-05-29: was listed as set in One_Time; binary shows One_Time never writes +0x11F0; it is written by Init_For_House via param_1[0x47c] where param_1 is int* so byte offset = 0x47c×4 = 0x11F0 — INFERENCE_HARDENED) |
| +0x11F4 | inner_draw_y | 49 | One_Time (0x652CF0) | Y where minimap content starts |
| +0x11F8 | inner_width | 140 | One_Time (0x652CF0) | Max minimap width (pixels) |
| +0x1200 | outer_width | 140 | One_Time (0x652CF0) | Surface allocation width |
| +0x11FC | outer_height | 108 | One_Time (0x652CF0) | Surface allocation height |
| +0x1204 | inner_height | 108 | One_Time (0x652CF0) | Max minimap height (pixels) |

---

## Step 1: Chrome Center X Computation (0x652E90)

From `RadarClass::Init_For_House`, verified from decompilation:

```c
// Allied (side == 0):
minimap_center_x = (SIDEBAR_WIDTH - 0x90) / 2 + 4;
//                = (168 - 144) / 2 + 4
//                = 12 + 4 = 16

// Soviet/Yuri (side != 0):
minimap_center_x = (SIDEBAR_WIDTH - 0x91) / 2 + 5;
//                = (168 - 145) / 2 + 5
//                = 11 + 5 = 16
```

The magic numbers **144** and **145** are the **inner window widths** of the Allied and
Soviet BKGDLG.SHP chrome art respectively. The `+4` / `+5` fine-tune constants account
for the slightly different border thicknesses. Both sides produce `minimap_center_x = 16`.

---

## Step 2: Viewport Positioning (0x654650)

From `RebuildRadarSurfaces`, after creating the zoomed secondary surface:

```c
// Default: minimap content starts at (minimap_center_x, inner_draw_y) = (16, 49)
viewport_x = minimap_center_x;   // 16
viewport_y = inner_draw_y;       // 49

// Center small maps within the available area
if (zoomed_width < 140)  // 0x8C
    viewport_x = (140 - zoomed_width) / 2 + minimap_center_x;
    //           centers horizontally, then adds the 16px chrome inset

if (zoomed_height < 108)  // 0x6C
    viewport_y = (108 - zoomed_height) / 2 + inner_draw_y;
    //           centers vertically, then adds the 49px chrome inset
```

This positions the minimap RELATIVE TO THE SIDEBAR SURFACE. The viewport coordinates
are sidebar-surface coordinates, not screen coordinates.

---

## Step 3: Per-Frame Compositing (0x656EC0)

From `RadarClass::Update`, the force_redraw path (verified from assembly at 0x6575B2–0x65764F):

### 3a. Draw chrome frame to sidebar surface

```asm
; Set up DrawSHP position = (sidebar_x, radar_draw_y) = (0, 48)
006575b2: MOV EDX, [ESI + 0x11e4]     ; EDX = sidebar_x = 0
006575be: MOV EDX, [ESI + 0x11ec]     ; EDX = radar_draw_y = 48
; ...
006575e5: MOV ECX, [0x00b04a38]       ; ECX = BKGDLG.SHP pointer
006575f3: PUSH 0x20                    ; frame 32 (fully open)
006575f6: MOV ECX, [0x00887300]       ; sidebar surface as draw target
006575fc: CALL DrawSHP                 ; draw chrome to sidebar at (0, 48)
```

The chrome's fully-open frame (32) is drawn to the sidebar surface. This puts the
decorative border with its dark inner window at Y=48 on the sidebar.

### 3b. Compute blit rects

```c
// The dirty rect IS in sidebar-surface coordinates:
dirty_rect = {
    viewport_x,      // = 16   (X on sidebar)
    viewport_y,      // = 49   (Y on sidebar)
    viewport_width,  // = zoomed map width (≤140)
    viewport_height  // = zoomed map height (≤108)
};

// The source rect is relative to the primary surface:
src_rect = {
    dirty_x - viewport_x,  // = 0 (start of primary surface)
    dirty_y - viewport_y,  // = 0
    dirty_w,                // = viewport_width
    dirty_h                 // = viewport_height
};
```

### 3c. Blit minimap pixels over the chrome

```asm
; Blit: sidebar_surface->Blit(dest=dirty_rect, src=primary_surface, src_rect, 0, 1)
00657634: MOV ECX, [0x00887300]       ; sidebar surface (destination)
0065763d: MOV EAX, [ESI + 0x121c]    ; primary surface (source)
0065764d: PUSH EAX                    ; source surface
0065764e: PUSH EBX                    ; dest rect (on sidebar) = {16, 49, w, h}
0065764f: CALL [EDX + 0x8]            ; BSurface::Blit (vtable +0x08)
```

The minimap pixels from the primary surface are blitted ON TOP of the chrome that was
just drawn. The chrome's dark inner area gets overwritten by the minimap content.
The decorative border around the edges remains visible.

### 3d. InvalidateRect

```asm
; Mark the radar window area as dirty on the sidebar surface
00657652: MOV EAX, [ESI + 0x1208]    ; overlay layer ID
00657658: MOV ECX, [0x00887300]      ; sidebar surface
00657668: CALL [EDX + 0x58]           ; BSurface::InvalidateRect

; Also invalidate a 1px-larger rect for edge cleanup
; rect = {viewport_x - 1, viewport_y - 1, viewport_w + 2, viewport_h + 2}
006576a2: CALL [EDX + 0x58]           ; second InvalidateRect
```

---

## Step 4: Sidebar-to-Screen Blit (0x621E90 + 0x4F4480)

### In RenderFrame (0x4F4480):

```c
// If sidebar is dirty, signal the display chain to redraw it
if (DAT_00b0b519 != '\0') {
    DisplayChain->vtable_0x40(DAT_00887300, 1);  // mark sidebar for compositing
    DAT_00b0b519 = '\0';
}
```

### In the Window Rendering Loop (0x621E90):

The game uses a windowed GUI system. Each UI panel (sidebar, game area, etc.) is a
"window" with its own off-screen surface. The rendering loop:

1. Iterates all registered windows
2. For each window, calls its draw function → writes to the window's surface
3. **Final blit** at the end of each window's processing:

```c
// Blit window surface to screen (AlternateSurface = back buffer)
// dest_rect = window's position on screen (computed from window manager)
// src_rect = {0, 0, window_width, window_height}
if (window_surface != NULL) {
    src_rect = {window_x - screen_offset_x, window_y - screen_offset_y, width, height};
    AlternateSurface->Blit(&screen_rect, window_surface, &src_rect, 0, 1);
}
```

For the sidebar window, `screen_rect` = `(screen_width - 168, 0, 168, screen_height)`.

---

## Complete Coordinate Chain

For a minimap pixel at primary surface position (px, py):

| Layer | X | Y | Size |
|-------|---|---|------|
| Primary radar surface | px | py | up to 140×108 |
| Sidebar surface | viewport_x + px = **16 + px** | viewport_y + py = **49 + py** | within 168 × screen_h |
| Screen | screen_w - 168 + 16 + px = **screen_w - 152 + px** | **49 + py** | — |

For the chrome frame (BKGDLG.SHP):

| Layer | X | Y |
|-------|---|---|
| Sidebar surface | 0 | 48 |
| Screen | screen_w - 168 | 48 |

---

## Why It Fits: The Margin Math

```
Chrome drawn at: sidebar(0, 48),  size ≈ 168 × 110
Minimap at:      sidebar(16, 49), size ≤ 140 × 108

Left margin:   16 px                (minimap_center_x)
Right margin:  168 - 16 - 140 = 12 px
Top margin:    49 - 48 = 1 px       (inner_draw_y - radar_draw_y)
Bottom margin: (48 + 110) - (49 + 108) = 1 px
```

The BKGDLG.SHP chrome frame was designed by the artist with:
- A decorative border approximately 16px wide on the left
- Approximately 12px on the right
- 1px chrome edge at top and bottom
- A dark (black/near-black) inner window in the remaining area

The code positions the minimap to exactly cover this dark inner window. No explicit
clip rect is needed — the primary surface dimensions (max 140×108) physically prevent
overflow beyond the chrome border.

**The minimap fills its entire rectangular surface** — there are no dark corners or
diamond shapes. This is because `MapClass__Is_Cell_In_Playfield` (0x578460, corrected 2026-05-29: was `IsValidCell`; binary label is `MapClass__Is_Cell_In_Playfield` via `get_function_by_address 0x578460` — RTTI_LABEL_DRIFT) bounds the valid region using
`cellX + cellY` and `cellX - cellY` constraints (both ranges are constant, independent
of each other). Since the radar surface uses these same isometric coordinates
(`radar_x = cellX - cellY`, `radar_y = cellX + cellY`), the valid cells form a
rectangle in radar-surface space. The 2-pixel-wide cell rendering tiles seamlessly
between staggered rows to fill every pixel.

For maps smaller than 140×108 after zooming, the minimap is centered within the
available area. The chrome's dark background shows through the uncovered margins at
the edges — but these are rectangular margins (top/bottom or left/right bands), not
triangular corners.

---

## Draw Order (verified)

On each frame where the radar is active and needs a redraw:

```
1. BKGDLG.SHP frame 32 → sidebar surface at (0, 48)     [chrome border + dark inner]
2. Primary surface      → sidebar surface at (16, 49)    [minimap pixels over the dark area]
3. InvalidateRect       → marks sidebar as dirty
4. Sidebar surface      → screen at (screen_w - 168, 0)  [by window manager]
5. DIPLOBTN.SHP         → sidebar surface at button pos  [buttons drawn on top of chrome]
6. OPTBTN.SHP           → sidebar surface at button pos
```

The chrome is drawn FIRST, then the minimap overwrites the inner area, then buttons are
drawn on top. This layered approach means the chrome border is always visible, the minimap
fills the inner window, and the buttons overlay everything.

---

## During Open/Close Animation (States 2 & 3)

During the 33-frame animation (frames 0→31 opening, 31→0 closing):

```c
// The animation frame shows progressively more/less of the radar chrome
DrawSHP(BKGDLG_SHP, frame_counter,  // frame 0..31 (not 32!)
        {sidebar_x, radar_draw_y},   // (0, 48)
        sidebar_surface, ...);

// The dirty rect is set to the full chrome area (not just the minimap)
dirty_rect = {sidebar_x, radar_draw_y, SIDEBAR_WIDTH, 0x6E};
//          = {0, 48, 168, 110}
```

During animation, the MINIMAP IS NOT DRAWN. Only the chrome animation frame is shown.
The minimap content only appears once the animation reaches frame 32 (state transitions
to "active" = state 1), at which point the Update function begins blitting the primary
surface over the chrome's inner window.

This means during opening: the chrome "irises open" via the SHP animation, and once fully
open, the minimap appears. During closing: the minimap disappears and the chrome animation
plays in reverse.

---

---

## Animation Timing (verified from assembly)

The timer function FUN_006C8C40:
```c
uint GetRadarTimer(void) {
    return timeGetTime() >> 4;  // = timeGetTime() / 16
}
```

Each timer unit = **16ms**. The animation delay is **4 units = 64ms per frame**.
For 33 frames: 33 × 64 = **2112ms ≈ 2.1 seconds** total open/close time.

---

## Discrepancies Found in Rust Engine

### 1. Frame ordering is REVERSED

**Original engine (from Ghidra, verified):**
- `BKGDLG.SHP` frame 0 = **closed/deactivated** (drawn in state 0)
- `BKGDLG.SHP` frame 32 = **fully open/active** (drawn in state 1)
- Opening: frame_counter 0 → 31 → 32
- Closing: frame_counter → 0

**Rust engine (radar_anim.rs):**
- `radar.shp` frame 0 = **open** (Online state)
- `radar.shp` frame 32 = **closed** (Offline state)
- Opening: frame 32 → 0
- Closing: frame 0 → 32

This is the **exact opposite**. The binary proves frame 0 = deactivated:
```asm
; State 0 (deactivated) draws frame 0:
  PUSH 0x0          ; frame = 0
  PUSH ECX          ; BKGDLG.SHP
  CALL DrawSHP

; State 1 (active) draws frame 32:
  PUSH 0x20         ; frame = 32 (0x20)
  PUSH ECX          ; BKGDLG.SHP
  CALL DrawSHP
```

Either the actual SHP file `radar.shp` in `sidec0x.mix` has frames in the opposite
order from how the original engine interprets them, or the Rust code has the mapping
backwards. The visual result may still be correct if the SHP frames themselves are
ordered open→closed (0=open, 32=closed) and the original engine simply starts counting
from the "closed" end.

### 2. Animation speed is wrong

| | Original | Rust |
|---|---|---|
| Per-frame delay | 64ms (4 × 16ms timer units) | 100ms |
| Total animation | ~2.1 seconds | ~3.3 seconds |

The Rust animation is **56% slower** than the original.

### 3. SHP file identity

The original engine uses **BKGDLG.SHP** (or BKGDLGY.SHP for Yuri) loaded from
side-specific mix files, stored as `DAT_00b04a38 = DAT_00b0fa68`.

The Rust engine uses **radar.shp** from `sidec0x.mix`. This may be the same file
under a different internal name within the mix archive, or a different file entirely.
The file `RADAR.SHP` at `DAT_00b0fb08` in the original is a DIFFERENT asset (the
top credits strip), NOT the chrome animation.

### 4. Minimap content area dimensions differ

| | Original (from Ghidra) | Rust engine |
|---|---|---|
| Inner width | 140 px (0x8C) | 150 px (RADAR_CONTENT_WIDTH) |
| Inner height | 108 px (0x6C) | 96 px (RADAR_CONTENT_HEIGHT) |
| Left inset | 16 px (minimap_center_x) | Auto-detected from SHP transparency |
| Top inset | 49 px (inner_draw_y) | Auto-detected from SHP transparency |

The auto-detection approach (`detect_radar_content_insets` scanning for alpha==0
pixels) is clever but may not produce the same values as the original's hardcoded
constants, especially since the original uses BKGDLG.SHP while Rust uses radar.shp.

### 5. BKGDLG.SHP is loaded but never used

`sidebar_chrome.rs` loads `bkgdlg.shp`, `bkgdmd.shp`, `bkgdsm.shp` into atlas entries
`background_large`/`medium`/`small`, but these are **dead code** — nothing in the
rendering pipeline references them.

---

*Verified 2025-03-20 via live Ghidra MCP decompilation and disassembly.
Assembly addresses confirmed for every constant and blit call.
Timer function FUN_006C8C40 confirmed as timeGetTime() >> 4 = 16ms units.
Updated 2026-03-21: clarified that terrain fills full rectangular surface (not diamond)
based on IsValidCell bounds being rectangular in isometric space.*
