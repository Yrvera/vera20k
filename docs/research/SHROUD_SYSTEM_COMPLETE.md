# Shroud System — Complete Ghidra Research Report

**Scope:** Only documents systems ACTIVE in standard Yuri's Revenge (FogOfWar=false).
Fog-of-war (TS legacy, gated behind `SpecialFlags & 0x1000`) is excluded.

**Confidence:** HIGH — all findings verified from binary decompilation via Ghidra MCP.

**Supersedes:** This document consolidates and replaces the shroud-relevant parts of:
- `SHROUD_FOG_RENDERING_PIPELINE.md`
- `SHROUD_REVEAL_SYSTEM_GHIDRA_REPORT.md`
- `BSURFACE_CIRCBUF_ABUFFER_REPORT.md`
- `OBJECT_FOG_VISIBILITY_GHIDRA_REPORT.md`
- `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md`

---

## Overview

The shroud system renders unexplored map areas as **black**. Explored cells stay
permanently visible (no re-fogging). The only thing that can re-shroud explored
cells in standard YR is the **Gap Generator** building.

Two rendering layers:
1. **Fully shrouded cells** — Z-buffer cleared with a clear tile diamond shape
2. **Shroud edge cells** — SHROUD.SHP frames written to the ABuffer (per-pixel
   alpha overlay), which all tile/sprite blitters read to darken pixels

---

## Cell Visibility Flags

### CellClass+0x12C (byte offset 300) — Shroud bitflags

| Bit | Mask | Meaning |
|-----|------|---------|
| 3   | 0x08 | Cell has been explored (shroud cleared) |
| 4   | 0x10 | Fully revealed (no shroud edge transition needed) |

- `IsShrouded()` at 0x00586360: returns 1 when `(cell+300 & 0x08) == 0`
- Both bits are set together by `CellClass__RevealShroudFlags` (0x004876f0):
  ```c
  cell->flags_12C |= 0x18;  // set bits 3 + 4
  ```
- Once set, bits are **permanent** in standard YR (ShroudGrow=false)

### CellClass+0x130 — Gap Generator counter

- Reference count of gap generators covering this cell
- Incremented by `FUN_00487690`, decremented by `FUN_00487630`
- Clamped to `cell+0x134` (max gap coverage)
- When counter transitions 0→positive: sets `cell+0x140 |= 0x20` (GAP overlay)
- When counter transitions positive→0: restores `cell+0x12C |= 0x18` (explored)

### CellClass+0x11B — Cell height level (byte)

- Terrain elevation level. Used by height-based line of sight (RevealByHeight)
- Also passed to TMP_TileBlitter as the brightness/opacity row parameter

### CellClass+0x120 — Cached shroud edge frame (byte)

- Stores the last computed shroud bitmask result from `Shroud_EdgeBitmask_Calculator` (mode=0, shroud)
- Written by `Shroud_fog_edge_rendering` each time the cell is re-rendered

### CellClass+0x121 — Cached fog edge frame (byte)

- Stores the last computed fog bitmask result from `Shroud_EdgeBitmask_Calculator` (mode=1, fog)
- Written by `Shroud_fog_edge_rendering` immediately after the shroud edge write
- Only consumed when `*g_ScenarioClass_Instance & 0x1000` (FogOfWar enabled); dormant in standard YR
- (corrected 2026-05-29: field was absent; binary at Shroud_fog_edge_rendering 0x004801f0 writes
  `*(char *)(param_1 + 0x121) = cVar1` after the fog-mode EdgeBitmask_Calculator call — STALE)

---

## SHROUD.SHP — Loading and Selection

### Loading (lazy init in ShroudEdge_BlitToABuffer at 0x0047efe0)

```c
if (DAT_0089e7c5 == 0) {                       // first-call flag
    DAT_0089e7c5 = 1;
    DAT_0089e794 = LoadFileFromMIX("SHROUD.SHP");  // string at 0x0081ce18
    DAT_0089e790 = LoadFileFromMIX("FOG.SHP");     // string at 0x0081ce10
}
```

### Selection

```c
if (*g_ScenarioClass_Instance & 0x1000)   // FogOfWar enabled? (NOT in standard YR)
    shp = DAT_0089e790;                    // FOG.SHP
else
    shp = DAT_0089e794;                    // SHROUD.SHP  ← standard YR path
```
// corrected 2026-05-29: was `*DAT_00a8b230 & 0x1000` — DAT_00a8b230 IS g_ScenarioClass_Instance;
// verified via decompile_function 0x0047efe0 which uses `*g_ScenarioClass_Instance & 0x1000`

### Frame layout

- **Frame 0**: Empty (0×0 dimensions) — represents "no edge needed"
- **Frames 1–46**: Edge transition shapes (47 unique patterns for all possible
  neighbor adjacency combinations on isometric cells)
- **Frame 15**: Full 60×30 isometric diamond filled with 0x00 pixels — used for
  completely unexplored cells

### Pixel value semantics

Each pixel in SHROUD.SHP is a **direct ABuffer alpha value**:

| Pixel | ABuffer result | Visual effect |
|-------|---------------|---------------|
| 0x00  | 0x0000        | Pitch black (full shroud) |
| 0x01–0x7E | proportional | Gradient (smooth edge transition) |
| 0x7F  | 0x007F        | Neutral (fully lit, no darkening) |
| 0xFE  | (skipped)     | Transparent — ABuffer unchanged |

---

## Edge Bitmask Calculator (0x006d8700)

### Purpose

For a given cell, determines which SHROUD.SHP frame to draw by checking
the shroud state of all 8 neighbors.

### Logic (mode=0, shroud only)

```c
int Shroud_EdgeBitmask_Calculator(CellXY* cell, int mode) {
    CellClass* c = GetCell(cell);

    // CRITICAL: check the CURRENT cell first
    if ((c->flags_12C & 0x18) == 0) return -2;  // never explored → fully shrouded
    if ((c->flags_12C & 0x08) == 0) return -1;  // not explored → no edge from here

    // Cell IS explored. Check 8 neighbors for shrouded ones.
    uint8 mask = 0;
    if (IsNeighborShrouded(cell, -1, -1)) mask |= 0x40;  // NW → bit 6
    if (IsNeighborShrouded(cell,  0, -1)) mask |= 0x80;  // N  → bit 7
    if (IsNeighborShrouded(cell, +1, -1)) mask |= 0x01;  // NE → bit 0
    if (IsNeighborShrouded(cell, -1,  0)) mask |= 0x20;  // W  → bit 5
    if (IsNeighborShrouded(cell, +1,  0)) mask |= 0x02;  // E  → bit 1
    if (IsNeighborShrouded(cell, -1, +1)) mask |= 0x10;  // SW → bit 4
    if (IsNeighborShrouded(cell,  0, +1)) mask |= 0x08;  // S  → bit 3
    if (IsNeighborShrouded(cell, +1, +1)) mask |= 0x04;  // SE → bit 2

    return (signed char) LUT_007f4194[mask];  // → frame index or 0xFF/0xFE
}
```

**Key insight:** Edges are drawn on the **EXPLORED side** of the boundary. The
function returns early if the current cell is shrouded — it only checks neighbors
when the cell itself is explored.

### Neighbor bit layout

```
  NW  N  NE         bit 6  bit 7  bit 0
   W  *   E    →    bit 5    *    bit 1
  SW  S  SE         bit 4  bit 3  bit 2
```

Each bit is SET when that neighbor IS shrouded (unexplored).

### Lookup table at 0x007f4194

256-byte table. Key return values:
- **0xFF** (signed: -1) = no edge needed (no shrouded neighbors)
- **0xFE** (signed: -2) = fully surrounded by shroud
- **0–46** = SHROUD.SHP frame index for that specific edge pattern

### Frame mapping in Shroud_fog_edge_rendering (0x004801f0)

```c
frame = Shroud_EdgeBitmask_Calculator(cell, 0);
cell->shroud_edge_frame = frame;  // cached at cell+0x120

if (frame == -2) frame = 15;     // fully shrouded → full black diamond
if (frame == -1) frame = 0;      // no edge → empty frame (noop)

ShroudEdge_BlitToABuffer(screen_pos, clip_rect, frame);
```

---

## ABuffer System

### Structure — CircBuf wrapper (0x30 bytes)

The ABuffer (`g_ABuffer` at 0x0087e8a4) is a circular buffer wrapping a BSurface:

| Offset | Field | Description |
|--------|-------|-------------|
| 0x00 | origin_x | Viewport left edge |
| 0x04 | origin_y | Viewport top edge |
| 0x08 | width | Buffer width in pixels |
| 0x0C | height | Buffer height in pixels |
| 0x10 | circ_offset | Circular scroll offset (bytes) |
| 0x14 | inner_surface | BSurface* (owns pixel memory) |
| 0x18 | buffer_base | Cached data pointer |
| 0x1C | buffer_upper | Wrap boundary pointer |
| 0x20 | buffer_size | Total bytes (width × height × 2) |
| 0x24 | default_value | 0x8000 |
| 0x28 | stride | Width in pixels |
| 0x2C | num_rows | Height in pixels |

- **16-bit per pixel** (BSurface bytes_per_pixel = 2)
- **Neutral value**: 0x7F (fully lit, no darkening)
- **Black value**: 0x00 (maximum darkening)
- Supports viewport scrolling without data copies via circular offset

### Scanline access — CircBuf_GetScanlinePtr (0x004114b0)

```c
uint16* CircBuf_GetScanlinePtr(CircBuf* buf, int x, int y) {
    uint16* ptr = inner_surface->Lock(x, y);  // linear pointer
    ptr += buf->circ_offset;                   // add circular offset
    if (ptr >= buf->buffer_upper)
        ptr -= buf->buffer_size;               // wrap around
    return ptr;
}
```

Callers pass y relative to viewport top (subtracting `g_RadarViewportOffsetY`).

### ABuffer lifecycle per frame

```
Pass 0 (scroll/buffer management):
  Full redraw → CircBuf__FillAll(0x7F)        // clear entire ABuffer
  Scroll → CircBuf__Scroll()                   // shift circular offset

Pass 1 Step 2 (shroud edges):
  For each dirty rect:
    FUN_00411330(rect) → fill with 0x007F      // clear region to neutral
    FUN_006d71e0(rect) → re-render edges       // write SHROUD.SHP pixels

  For each cell in shroud edge list:
    Shroud_fog_edge_rendering()                 // write SHROUD.SHP pixels

Pass 1 Steps 3–8 (terrain):
  TMP_TileBlitter reads ABuffer per-pixel      // darkens tile colors

Pass 2 (objects):
  SHP/VXL blitters read ABuffer per-pixel      // darkens sprite colors
```

### ABuffer fill (FUN_00411330)

Fills a rectangular region with `0x007F` (packed as `0x007F007F` for 32-bit
dword writes). Handles circular buffer wrap-around.

---

## ShroudEdge_BlitToABuffer (0x0047efe0) — The Core Blitter

### Per-pixel loop

```c
for (each scanline in SHP frame, clipped to viewport) {
    for (each pixel in scanline) {
        byte pixel = *src++;
        if (pixel != 0xFE) {              // 0xFE = transparent, skip
            *abuffer_ptr = (uint16)pixel;  // DIRECT write
        }
        abuffer_ptr++;
        // handle circular buffer wrap if needed
    }
}
```

This is a **direct pixel write** — the SHP pixel value becomes the ABuffer value.
No blending, no lookup. Shroud edges overwrite whatever was in the ABuffer.

---

## How ABuffer Darkens Rendered Pixels

### Brightness remap table (FUN_00420140)

A 65536-entry table of `uint16` values, built per LightConvertClass:

```c
for (i = 0; i < 0x10000; i++) {
    low  = i & 0xFF;         // ABuffer value (column)
    high = i >> 8;           // opacity level (row)
    result = (low * high * (max_brightness - 1)) / 0x7E02;
    table[i] = clamp(result, 0, max_brightness - 1) << 8;
}
```

### Per-pixel compositing in TMP_TileBlitter

```c
uint16 abuf_val   = *abuffer_ptr;                         // read ABuffer
uint16 brightness = remap_table[abuf_val];                 // brightness << 8
uint8  palette_idx = *tile_pixel_ptr;                      // palette color
uint16 color      = color_table[brightness | palette_idx]; // final RGB565
*screen_ptr = color;
```

**Effect of key ABuffer values:**
- `0x00` → brightness 0 → darkest remap → **black pixel**
- `0x3F` → ~half brightness → **dimmed pixel** (shroud edge gradient)
- `0x7F` → full brightness → **normal color**

### Color table (ConvertClass+0x170)

`max_brightness × 256` entries of RGB565. Entry `[brightness * 256 + palette_index]`
gives the final screen color. Built from the theater palette with lighting applied.

---

## Reveal System

### MapClass::RevealShroud (0x005673a0)

Called when a unit's sight range should reveal cells.

**Parameters:**
- `coords` — world position (leptons)
- `sight_range` — radius in cells (clamped to **max 10**)
- `house` — the revealing player
- `param_5` — height bonus flag
- `param_8` — RevealByHeight flag

**Flow:**
1. Adjust world coords for Z height (elevation → lepton offset)
2. Convert to cell coordinates
3. Early-exit bounds checks against map dimensions
4. Clamp sight range to 10 (hard cap: `if (param_3 > 10) param_3 = 10`)
5. **Alliance check**: if revealer is allied AND `rules+0x17E7` (AllyReveal=true),
   treat as local player reveal
6. Iterate reveal spiral table

### Reveal spiral table (DAT_00abd490)

Pre-built table of `(short dx, short dy)` pairs, ~370 entries, populated at
init by `MapClass__InitRevealSpiralTable` (0x00561910). Entries are sorted
approximately by distance from center.

Ring size table at `0x007ed3d0`:
```
sight 0 →   1 cell
sight 1 →   9 cells
sight 2 →  21 cells
sight 3 →  37 cells
sight 4 →  61 cells
sight 5 →  89 cells
sight 6 → 121 cells
sight 7 → 161 cells
sight 8 → 205 cells
sight 9 → 253 cells
sight 10→ ~310 cells
```

### Per-cell reveal check

For each cell in the spiral:
1. **Euclidean distance check**: `sqrt(dx² + dy²)` via `Sqrt_Approx` (0x004cac40)
   must be ≤ sight_range
2. **Height-based LOS** (if `RevealByHeight=true`, default): checks a mirror/midpoint
   cell from table at `DAT_00abcf60`. If `viewer_level + 3 < midpoint_cell.Level`,
   LOS is blocked (terrain 4+ levels above viewer at midpoint blocks sight)
3. **Cooperative mode skip**: in non-coop multiplayer, skips invisible border tiles
4. If all checks pass: `CellClass__RevealShroudFlags()` → sets `cell+0x12C |= 0x18`

### Sqrt_Approx (0x004cac40)

Fast square root using a 8192-entry lookup table at `DAT_008650bc`. Decomposes
float into mantissa + exponent, halves exponent, uses top 13 bits of mantissa
as table index.

### How units trigger reveal

- **vtable+0x48C** = `TechnoClass__ReReveal` (0x0070b1d0) — re-reveals using
  stored sight range. Called by the "paranoid" full-pass functions.
- **vtable+0x488** = `TechnoClass__UpdateReveal` (0x0070af50) — computes effective
  sight range (including veteran bonus), calls RevealShroud.
- Buildings reveal from their **center point only** — foundation size does NOT
  affect reveal area. A 3×3 building with Sight=8 reveals identically to a 1×1.

### Paranoid reveal functions

Called periodically to ensure all player units have their shroud properly revealed:
- `MapClass__ParanoidRevealAll` (0x004adee0) — iterates all TechnoClass objects,
  calls ReReveal for player-controlled and AllyReveal-eligible units
- `MapClass__ParanoidUnrevealAll` (0x004adcd0) — similar but for the shroud-removal
  path, also calls `MapClass__UpdateFogBorder`

---

## Gap Generator — Re-shrouding in YR

The only mechanism that can re-shroud explored cells in standard YR.

### Cell fields

| Offset | Type | Purpose |
|--------|------|---------|
| +0x130 | int  | Gap coverage reference count |
| +0x134 | int  | Max gap coverage (clamp) |
| +0x140 bit 5 | 0x20 | GAP overlay active flag |

### Cover a cell (FUN_00487690)

```c
if (gap_counter == -1) gap_counter = 0;  // reset sentinel
gap_counter++;
if (gap_counter > gap_max) gap_counter = gap_max;  // clamp
if (was_zero && now_positive) cell->flags_140 |= 0x20;  // set GAP flag
```

### Uncover a cell (FUN_00487630)

```c
gap_counter--;
if (cell was NOT fully explored (bits 3+4 not both set)) {
    if (gap_counter < 1) cell->flags_12C |= 0x18;  // restore explored
} else if (gap_counter < 1 && GAP flag set) {
    cell->flags_140 &= ~0x20;  // clear GAP overlay
}
```

### Building tick

`BuildingClass__UpdateGapGenerator_Tick` iterates cells within the gap radius
and calls the cover/uncover functions. When a gap generator is destroyed, all
its covered cells get uncovered.

---

## Object Visibility

### Key fields

| Offset | Field | Purpose |
|--------|-------|---------|
| ObjectClass+0x80 | NeedsDraw | Dirty flag, cleared after draw |
| ObjectClass+0x81 | IsUndiscovered | **1 = hidden from rendering entirely** |
| TechnoClass+0x41A | HasBeenDiscovered | Set once per any house discovery |
| TechnoClass+0x41B | DiscoveredByPlayer | Set when local player discovers |

### Visibility states

1. **Shrouded** (never seen): `+0x81 = 1`, object NOT in display layer. Invisible.
2. **Visible** (in sight range): `+0x81 = 0`, object in display layer, drawn normally.

(In standard YR without fog-of-war, there is no "fogged" intermediate state.
Once discovered, objects stay visible. Only gap generators can re-hide them.)

### Reveal lifecycle

When `RevealShroud` reveals a cell containing objects:
1. `CellClass__RevealShroudFlags()` → marks cell explored
2. `CellChangeNotify` iterates objects in cell
3. For each object: `TechnoClass::Discover` (vtable 0x198)
4. `Discover` calls `ObjectClass::Reveal` (vtable 0xD8):
   - Sets `+0x81 = 0` (now drawable)
   - Submits object to display layer
5. Object now appears in rendering

### Conceal lifecycle (gap generators only)

When a gap generator covers a cell:
1. `ObjectClass::Conceal` (vtable 0xD4)
2. Removes object from display layer (vtable 0x150)
3. Sets `+0x81 = 1` (hidden from rendering)

---

## Rendering Pipeline — Shroud Integration

### Three-pass architecture

`TacticalClass_Draw` (0x006d3d10) is called 3 times per frame:

```
RenderFrame_main (0x004f4480):
    TacticalClass_Draw(pass=0)  → scroll, ABuffer/ZBuffer management
    TacticalClass_Draw(pass=1)  → terrain layers (writes ABuffer, then reads it)
    GScreenClass::Draw()        → sidebar/UI
    TacticalClass_Draw(pass=2)  → objects (reads ABuffer for darkening)
```

### Pass 0 — Buffer management

- Full redraw: clears ZBuffer and ABuffer (fill with 0x7F)
- Scroll: circular buffer offset adjustment, no data copy

### Pass 1 — Terrain (8 steps)

1. `Tactical_ZBufferDirtyClear` (0x006d2b60) — ZBuffer dirty rect clear.
   For shroud edge cells: blits clear tile to ZBuffer only (resets depth)
2. **`Tactical_layer_shroud_edges`** (0x006d3660) — **writes SHROUD.SHP to ABuffer**
3. Terrain shadows
4. Base terrain tiles (TMP_TileBlitter **reads ABuffer** for brightness)
5. Smudges
6. Building overlays
7. Overlays (walls, ore)
8. Ground animations

### Pass 2 — Objects

All SHP/VXL blitters **read ABuffer** per-pixel for brightness modulation.
Objects near shroud edges appear darkened on the transition side.

### Shroud tile for fully-black cells

`FUN_00480180` — called from ZBuffer dirty clear for shroud cells:
```c
TMP_TileBlitter(
    LightConvertClass[0],  // *DAT_0087f69c = first lighting table
    0,                     // frame 0
    g_PrimarySurface,
    screen_x, screen_y,
    clip_x, clip_y, clip_w, clip_h,
    cell->Level,           // cell+0x11B
    cell->tile_z,          // cell+0x10C
    1,  // ZBuffer write ON
    0,  // frame sub-index
    0,  // NOT shroud-fill mode
    1,  // ZBuffer-clear-only mode (write ZBuffer, NOT screen pixels)
    0, 0
);
```

This writes `0xFFFF` (max depth) to ZBuffer for the diamond area — resetting
depth so subsequent terrain tiles render correctly. It does NOT draw visible
pixels; the blackness comes from the ABuffer (0x00 values from SHROUD.SHP frame 15).

---

## Rules.ini Settings (Shroud-relevant, active in YR)

### [General] section

| Offset | INI Key | Type | Default | Purpose |
|--------|---------|------|---------|---------|
| 0x17EE | RevealByHeight | bool | true | Height-based LOS obstruction |
| 0x17E7 | AllyReveal | bool | true | Allied players share vision |
| 0x17EF | AllowShroudedSubteranneanMoves | bool | false | Subterranean units can path through shroud |
| 0x00F4 | AircraftFogReveal | int | 6 | Aircraft sight range override |
| 0x062C | RevealTriggerRadius | int | 5 | Trigger action reveal radius |

### [AudioVisual] section

| Offset | INI Key | Type | Default | Purpose |
|--------|---------|------|---------|---------|
| 0x17E2 | BlendedFog | bool | true | Smooth vs dithered shroud edges |

### [MultiplayerDialogSettings] section

| Offset | INI Key | Type | Default | Purpose |
|--------|---------|------|---------|---------|
| 0x14AE | Shroud | bool | true | Map starts with shroud |
| 0x14B7 | FogOfWar | bool | **false** | TS legacy fog — OFF in standard YR |

### Dormant settings (NOT active in standard YR)

| Offset | INI Key | Default | Why dormant |
|--------|---------|---------|-------------|
| 0x17F0 | ShroudGrow | false | Shroud regrow — requires explicit enable |
| 0x1640 | ShroudRate | 4.0 min | Only active if ShroudGrow=true |
| 0x1648 | FogRate | 0.05 min | Only active if FogOfWar=true |
| 0x14AD | ShadowGrow | false | MP checkbox for shroud regrow |

---

## Global Variables

| Address | Name | Type | Purpose |
|---------|------|------|---------|
| 0x0087e8a4 | g_ABuffer | CircBuf* | 16-bit per-pixel alpha overlay |
| 0x00887644 | g_ZBuffer | CircBuf* | 16-bit per-pixel depth |
| 0x0089e794 | shroud_shp | SHP* | Loaded SHROUD.SHP |
| 0x0089e7c5 | shroud_loaded | bool | Lazy init flag |
| 0x007f4194 | edge_frame_lut | byte[256] | Neighbor bitmask → frame index |
| 0x00a8b230 | g_ScenarioClass_Instance | ScenarioClass* | ScenarioClass instance; bit 12 of *ptr = FogOfWar flag (corrected 2026-05-29: was labelled "SpecialFlags_ptr" — wrong; binary shows `*g_ScenarioClass_Instance & 0x1000` check in ShroudEdge_BlitToABuffer 0x0047efe0 and FUN_006be1c0 confirms this global is g_ScenarioClass_Instance — RTTI_LABEL_DRIFT) |
| 0x007e4eb8 | transparent_index | byte | SHP transparent pixel (0xFE) |
| 0x00886fa0 | g_RadarViewportOffsetX | int | Viewport X origin |
| 0x00886fa4 | g_RadarViewportOffsetY | int | Viewport Y origin |
| 0x00abd490 | reveal_spiral_table | short[] | Pre-built (dx,dy) pairs |
| 0x007ed3d0 | reveal_ring_sizes | int[] | Cumulative cell count per sight range |
| 0x007ed3c4 | reveal_inner_skip | int[] | Inner ring skip for optimization |
| 0x00abcf60 | reveal_mirror_table | short[] | Midpoint cells for LOS checks |
| 0x008650bc | sqrt_lut | int[8192] | Fast square root lookup |
| 0x00b73550 | g_hWnd | HWND | Window handle (used as "initialized" check) |

---

## Function Reference

| Address | Name | Purpose |
|---------|------|---------|
| 0x0047efe0 | ShroudEdge_BlitToABuffer | Write SHROUD.SHP pixels to ABuffer |
| 0x004801f0 | Shroud_fog_edge_rendering | Dispatch: compute bitmask → select frame → blit |
| 0x006d8700 | Shroud_EdgeBitmask_Calculator | Check 8 neighbors, build mask, LUT lookup |
| 0x006d3660 | Tactical_layer_shroud_edges | Pass 1 Step 2: iterate edge cells, blit edges |
| 0x006d71e0 | FUN_006d71e0 | Full region shroud edge pass (dirty rect) |
| 0x00411330 | ABuffer_FillRect_Neutral | Fill ABuffer region with 0x7F |
| 0x006d2b60 | Tactical_ZBufferDirtyClear | ZBuffer dirty clear + shroud tile ZBuffer reset |
| 0x00480180 | ShroudTile_ZBufferReset | Blit clear tile to ZBuffer for shrouded cells |
| 0x004876f0 | CellClass__RevealShroudFlags | Set cell+0x12C bits 3+4 (explored) |
| 0x005673a0 | MapClass__RevealShroud | Reveal cells in sight radius |
| 0x00561910 | MapClass__InitRevealSpiralTable | Generate reveal spiral pattern |
| 0x005638d0 | MapClass__InitRevealMirrorTable | Generate LOS midpoint table |
| 0x004cac40 | Sqrt_Approx | Fast sqrt via 8192-entry LUT |
| 0x0070af50 | TechnoClass__UpdateReveal | Compute sight range + reveal |
| 0x0070b1d0 | TechnoClass__ReReveal | Re-reveal with stored sight range |
| 0x004adee0 | MapClass__ParanoidRevealAll | Full pass: ensure all units reveal |
| 0x00487690 | GapGenerator_CoverCell | Increment gap counter on cell |
| 0x00487630 | GapGenerator_UncoverCell | Decrement gap counter on cell |
| 0x00586360 | IsShrouded | Check if world coords are in shroud |
| 0x00420140 | BuildBrightnessRemapTable | Build ABuffer→brightness lookup |
| 0x004114b0 | CircBuf_GetScanlinePtr | Get ABuffer pointer for (x,y) |
