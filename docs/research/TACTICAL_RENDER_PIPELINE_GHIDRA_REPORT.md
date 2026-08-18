# TacticalClass Complete Rendering Pipeline

Reverse-engineered via Ghidra MCP (live decompilation of `gamemd.exe`).
Confidence: HIGH -- all addresses verified from direct decompilation.

This document supersedes the Phase 2 section of `DRAW_ORDER_DEPTH_SYSTEM.md` with
much more detail on every function call, the three-pass architecture, and the
ABuffer/ZBuffer lifecycle.

---

## Overview: The Three-Pass Architecture

`TacticalClass_Draw` (0x006d3d10) is called **three times per frame** from
`RenderFrame_main` (0x004f4480):

```c
// RenderFrame_main at 0x004f4480
void RenderFrame_main(GScreenClass* this) {
    g_PrimarySurface = g_CompositionSurface;  // DAT_0088731c
    GScreenClass::Draw(this, g_CompositionSurface, 0);  // vtable+0x40

    int redrawFlags = this->field_0xC;
    this->field_0xC = 0;
    bool forceRedraw = (redrawFlags != 0);

    if (!FUN_0053bae0()) {  // check if loading screen active
        TacticalClass_Draw(g_Tactical, g_CompositionSurface, forceRedraw, 0);  // Pass 0: Scroll
        TacticalClass_Draw(g_Tactical, g_CompositionSurface, forceRedraw, 1);  // Pass 1: Terrain
        GScreenClass::Draw(this, redrawFlags == 2);  // vtable+0x40 (sidebar/UI between passes!)
        TacticalClass_Draw(g_Tactical, g_CompositionSurface, forceRedraw, 2);  // Pass 2: Objects
    }

    // Sidebar, tooltips, and house AI follow...
}
```

**Critical finding:** The sidebar/UI is drawn BETWEEN terrain (Pass 1) and objects (Pass 2).
This means the sidebar surface is composited onto the back buffer before objects are drawn.

---

## Pass 0: Scroll & Buffer Management (param_3 == 0)

**When called:** First of the three TacticalClass_Draw calls.
**Returns early** after buffer management -- never draws visible content.

### Flow

1. **Dirty region check:** Computes how much the viewport has scrolled since last frame
   (`uVar13` = X delta, `local_f0` = Y delta).

2. **Full redraw path** (no scroll, `forceRedraw` is true):
   ```c
   ZBuffer_rect_clear();     // 0x007bcf50 -- fills entire ZBuffer with default value
   CircBuf__FillAll();       // 0x004112d0 -- fills entire ABuffer with default value
   ```
   This clears both the ZBuffer (depth) and ABuffer (shroud/fog alpha) completely.

3. **Scroll path** (viewport moved):
   ```c
   (*g_CompositionSurface->vtable->Blit)();      // Copy visible region of old frame
   FUN_007bcb50();                                // ZBuffer scroll (same CircBuf scroll logic)
   CircBuf__Scroll();                             // 0x00410ed0 -- ABuffer scroll
   // Surface swap: back buffer <-> composition surface
   DAT_0088731c = DAT_008872fc;   // old comp surface becomes back buffer
   DAT_008872fc = old_comp;       // old back buffer becomes comp surface
   g_PrimarySurface = new_back;
   ```
   The ABuffer and ZBuffer are circular buffers that support scrolling without copying
   the entire buffer -- they adjust a `circ_offset` pointer and only clear the newly
   exposed strips.

4. **Returns** (`if (param_3 == 0) return;`).

### Key ABuffer/ZBuffer Lifecycle

- **g_ABuffer** (0x0087e8a4): CircBuf wrapper for shroud/fog alpha. 16-bit per pixel.
  Values: 0x00 = fully shrouded/black, 0x7F = fully visible. Written by shroud edge
  rendering (Pass 1, Step 2). Read during object rendering (Pass 2) for alpha blitting.

- **g_ZBuffer** (0x00887644): CircBuf wrapper for depth. 16-bit per pixel.
  Written by terrain tile rendering. Read by Z-tested sprite blitters.

- **Full clear** happens when `forceRedraw && (param_3 == 0 || param_3 == 3)`.
- **Scroll** happens when viewport has moved and `(param_3 == 0 || param_3 == 3)`.

**Addresses:**
| Function | Address | Purpose |
|----------|---------|---------|
| ZBuffer_rect_clear | 0x007bcf50 | Fill entire ZBuffer with default |
| CircBuf__FillAll | 0x004112d0 | Fill entire ABuffer with default |
| CircBuf__Scroll | 0x00410ed0 | ABuffer circular scroll |
| FUN_007bcb50 | 0x007bcb50 | ZBuffer circular scroll (same logic as ABuffer) |

---

## Pass 1: Terrain Layers (param_3 == 1)

**When called:** Second TacticalClass_Draw call.
**Draws to:** The back buffer surface (DAT_008872fc), which was swapped in Pass 0.

### Pre-terrain Setup

Before the 8 terrain steps:
1. **Anim dirty marking** (`FUN_006d9a50`): Scans flat animations in
   `DisplayLayerEntry_008a0390`. For anims whose screen rect overlaps the viewport,
   calls `TacticalClass__DirtyScreenRect` to mark cells dirty and clears the anim's
   "needs redraw" flag. This ensures cells with active ground anims are redrawn.

2. **Surface lock**: Locks DAT_008872fc (the back buffer) for pixel access via
   `(*vtable + 0x5c)()`.

3. **Clip rect computation**: Computes up to 4 clip rectangles based on scroll
   direction (horizontal strip, vertical strip, overlap). Uses
   `AlphaShapeClass__ClipRect` (0x00421b60) to clamp to viewport bounds.

### The 8 Terrain Steps (in exact order)

```
Step 1: Tactical_ZBufferDirtyClear()          @ 0x006d2b60
Step 2: Tactical_layer_shroud_edges()         @ 0x006d3660
Step 3: Tactical_layer_terrain_shadows()      @ 0x006d2de0
Step 4: Tactical_layer_base_terrain()         @ 0x006d3470
Step 5: Tactical_layer_smudges()              @ 0x006d3290
Step 6: Tactical_layer_building_overlays()    @ 0x006d3ac0
Step 7: Tactical_layer_overlays()             @ 0x006d3040
Step 8: Tactical_layer_animations()           @ 0x006d3870
```

#### Step 1: Tactical_ZBufferDirtyClear (0x006d2b60)

Processes the dirty rect list (`g_DirtyRectList`, count at `DAT_00b0ce88`).
For each dirty rect:
- Transforms rect coordinates and clips to viewport
- Calls `ZBuffer_row_fill` (0x007bcfb0) to reset Z-buffer to default for that region
- Removes invalid rects (width or height < 1) from the list
- Also processes per-cell dirty entries from `this+0xE0` (count) / `this+0xE4` (cell list):
  calls `FUN_00480180` which renders the cell's TMP tile to clear/redraw.

#### Step 2: Tactical_layer_shroud_edges (0x006d3660)

Renders shroud and fog edge transitions into the **ABuffer**.
- For each dirty cell: calls `Shroud_fog_edge_rendering` (0x004801f0) which:
  - Computes shroud edge bitmask via `Shroud_EdgeBitmask_Calculator`
  - Calls `ShroudEdge_BlitToABuffer` (0x0047efe0) -- writes shroud alpha values
  - If fog-of-war enabled: calls `FogEdge_BlendToABuffer` (0x0047f250) -- writes fog alpha
- Then calls `FUN_006d71e0` for full rect regions -- this does an isometric cell sweep
  rendering shroud/fog for entire dirty rectangles. Verified at `0x006D754C`, this
  helper finishes by calling `AlphaShapeClass__DrawAll_NoMask` (`0x00421350`), not
  `AlphaShapeClass__DrawAll_WithMask` (`0x00420F40`).
- Also processes each dirty rect in the dirty rect list: clears ABuffer region to 0x7F
  (fully visible) via `FUN_00411330`, then re-renders shroud for that rect.

**This is the only pass that writes to the ABuffer.** The ABuffer is then READ during
Pass 2 when objects are drawn with alpha-aware blitters.

#### Step 3: Tactical_layer_terrain_shadows (0x006d2de0)

Renders terrain shadows (trees, cliffs) via `iso_to_screen` (isometric cell sweep).
Drawn BEFORE base terrain so shadows appear as flat decals under tiles.

#### Step 4: Tactical_layer_base_terrain (0x006d3470)

Renders isometric terrain tiles via `FUN_004d1890` (main tile renderer). This writes
per-pixel depth values to the ZBuffer using embedded Z-shape data from TMP tiles.

#### Step 5: Tactical_layer_smudges (0x006d3290)

Renders smudge overlays (craters, scorch marks) via `Cell_ContentRendering`.
Flat decals on top of base terrain.

#### Step 6: Tactical_layer_building_overlays (0x006d3ac0)

Draws building-attached flat animations from `DisplayLayerEntry_008a0390`.
Uses `FUN_006d97d0` which filters for RTTI == 0x24 (building anims) with `IsFlat`.
Iterates in **reverse order**. These are ground-level effects (Tesla Coil glow, etc.).

#### Step 7: Tactical_layer_overlays (0x006d3040)

Renders walls, fences, ore/tiberium overlays via `FUN_006d7c00` (isometric cell sweep).
Wall tiles use TMP format with embedded Z-data for correct depth.

#### Step 8: Tactical_layer_animations (0x006d3870)

Renders flat (ground-level) animations from `DisplayLayerEntry_008a0390`.
Uses `FUN_006d9920` which filters for RTTI == 6 (AnimClass) where the anim is flat.
These appear above terrain/overlays but below all game objects.

### Post-terrain

After the 8 steps:
- **Dirty rect compaction:** Removes processed rects from `g_DirtyRectList`
- **Surface unlock:** `(*g_PrimarySurface->vtable + 0x60)()` -- unlocks back buffer
- **Restore primary surface:** `g_PrimarySurface = piVar3` (the original primary)
- **Clear dirty cell count:** `this+0xE0 = 0`

---

## Between Pass 1 and Pass 2: Sidebar/UI Draw

`RenderFrame_main` calls `GScreenClass::Draw(this, redrawFlags==2)` after Pass 1
and before Pass 2. This draws the sidebar, tooltips, and other UI chrome on top of
the terrain but beneath game objects. The sidebar is drawn to a separate surface
(`g_SidebarSurface`) and composited later.

---

## Pass 2: Objects & Overlays (param_3 == 2)

**When called:** Third TacticalClass_Draw call.
**Draws to:** The composition surface (`param_1`), which is the final frame buffer.

### Gate Condition

```c
if (((param_3 != 1) && (param_3 != 3))
    || (Surface_Lock_primary(), param_3 != 1))
   && (param_3 == 2 || param_3 == 3))
```

When `param_3 == 1`, after terrain drawing, it locks the primary surface and then
skips Phase 2. When `param_3 == 2`, it skips terrain and enters Phase 2 directly.

### Phase 2 Call Order (26 steps)

```
 1. FUN_006d9ce0()                    -- Build visible building list for viewport
 2. Surface_Lock(param_1)            -- Lock composition surface for pixel access
 3. g_PrimarySurface = param_1       -- Set composition surface as primary
 4. FUN_006dad60()                    -- Rally point lines (1st call)
 5. FUN_006da9d0()                    -- Mind-control/capture links
 6. BuildingPlacement_OverlayRenderer() -- Building placement ghost overlay
 7. FUN_0053d850()                    -- Waypoint path lines
 8. Tactical_ObjectRenderingLoop()    -- THE MAIN OBJECT RENDERER
 9. FUN_005fffa0()                    -- Particle system rendering
10. FUN_00550240()                    -- Laser beam rendering
11. FUN_004c2830()                    -- Electric bolt rendering
12. FUN_00556d40()                    -- Trail/contrail rendering
13. FUN_006591b0()                    -- Wave/sonic effects (gravity gun, etc.)
14. FUN_006dbe20()                    -- Selection brackets / health bars on technos
15. FUN_00430ac0()                    -- Garrison occupant pips
16. Tactical__DrawBandBoxRect()       -- Band-box selection rectangle (0x006da180)
17. FUN_006dad60()                    -- Rally point lines (2nd call, post-objects)
18. FUN_006da9d0()                    -- Mind-control links (2nd call)
19. BuildingPlacement_OverlayRenderer() -- Building placement (2nd call)
20. DrawRadarOverlays_Normal()        -- Radar event markers (if radar available)
21. DrawRadarOverlays_Fog()           -- Fog-of-war radar markers
22. TechnoClass loop                  -- Selection circles, pips, capture links
23. FUN_0063b2f0()                    -- Super weapon target circles
24. FUN_006d7840()                    -- PixelFX / Tiberium glow per-pixel effects
25. Surface_Unlock(param_1)          -- Unlock composition surface
26. FUN_006d4b50() loop               -- Floating countdown text (EVA timers, etc.)
27. FUN_006d4e20()                    -- Centered loading/transition text overlay
```

### Detailed Phase 2 Breakdown

#### Step 1: Build Visible Building List (0x006d9ce0)

Scans all buildings (`g_BuildingClass_Array`). For each visible building within the
viewport, records it into a sorted draw list at `this+0xDB0` (max 500 entries). Each
entry stores the building pointer and its screen coordinates. This pre-computed list
is used later by the object rendering loop.

#### Steps 4-5: Rally Points & Capture Links

- **FUN_006dad60** (0x006dad60): Draws rally point lines from buildings to their
  rally point destinations. Uses `FUN_005be970/FUN_005be990` for line segment colors.
  Iterates player-owned buildings that have rally points set. The line is drawn with
  a pulsing frame counter modulo pattern. Called **twice** -- once before objects
  (beneath them) and once after (on top, for correct visual layering with alpha).

- **FUN_006da9d0** (0x006da9d0): Draws mind-control/capture manager links between
  controller and target. Iterates `g_TechnoClass_Array`, checks if techno is aircraft
  (RTTI == 6), player-controlled, has `CaptureManager`. Draws a colored line from
  controller position to captured unit position. Also called twice.

#### Step 6: Building Placement Ghost (0x006d5030)

Renders the translucent building footprint when the player is placing a building.
Shows green/red cells based on buildability. Uses the cell coordinate from
`DAT_00880990` (current placement building type) and `DAT_0088095c` (placement cell).

#### Step 7: Waypoint Path Lines (0x0053d850)

Draws unit waypoint/rally-point path lines. Iterates a waypoint list (stored at
`DAT_00aa0128`, count). For each waypoint entry, draws a line segment between
consecutive waypoints using the current surface pitch.

#### Step 8: Tactical_ObjectRenderingLoop (0x006d8db0)

The main object renderer. **300 lines decompiled.** See DRAW_ORDER_DEPTH_SYSTEM.md
for full details. Key structure:

**First loop** -- 5 display layers (0=Underground, 1=Surface, 2=Ground, 3=Air, 4=Top):
- For each object in each layer:
  - Clear `wasDrawn` flag (byte at obj+0x99)
  - Convert world coords to screen coords via `Tactical__WorldToScreenSub` + `AdjustForZ`
  - Clip to viewport bounds (with 0x168/0xB4 pixel margin)
  - Set `wasDrawn = 1` if visible
  - Call `vtable+0x10C` (SetDrawCoords) then `vtable+0x104` (DrawAs)
  - For alive non-techno objects: also calls `vtable+0x110` (DrawShadow) before DrawAs
- **After layer 2 only:** Building turret/garrison fire pass iterates all buildings

**Second loop** -- all 5 layers again (extras pass):
- For each object with `wasDrawn` set:
  - Call `vtable+0x110` (DrawExtras) -- selection circles, health bars, pips

#### Step 9: Particle System Rendering (0x005fffa0)

Iterates particle systems (`DAT_00ac1688` count). Calls `FUN_005ff850` per particle
which converts coords to screen, checks viewport bounds, and draws particle sprites
with alpha blending.

#### Step 10: Laser Beam Rendering (0x00550240)

Iterates laser draw entries (`g_LaserDraw_Count`). Calls `FUN_00550260` per laser
which draws a laser line between two screen positions with fade, using per-pixel
line drawing with color interpolation.

#### Step 11: Electric Bolt Rendering (0x004c2830)

Iterates electric bolt entries (`DAT_008a0e98` count). For each bolt:
- Gets target coordinates from attached techno via `vtable+0xB0`
- Converts both endpoints to screen coordinates
- Draws the bolt line with a flicker/width parameter
- Cleans up expired bolts (removes from list and frees memory)

#### Step 12: Trail/Contrail Rendering (0x00556d40)

Iterates trail entries (`DAT_00abcb88` count). Calls `FUN_00556c00` per trail which
draws connected line segments between trail waypoint positions, adjusting for Z height.
Expired trails (all segments faded) are removed and freed.

#### Step 13: Wave/Sonic Effects (0x006591b0)

Renders wave class effects (graviton beams, sonic waves). Iterates wave objects
(`DAT_00b04a70` count). Computes intensity based on frame counter timing, draws
per-pixel wave distortion effect on the surface.

#### Step 14: Selection Brackets & Health Bars (0x006dbe20)

Draws selection brackets and health bars for selected units. Iterates
`g_TechnoClass_Array`. For technos with `GetType()->HasSelectionBrackets`:
calls `vtable+0x130` (DrawSelectionBracket). Also handles the deployment target
cursor (cell highlight) when a unit is deploying.

#### Step 15: Garrison Occupant Pips (0x00430ac0)

Iterates cells in a 8x3 grid around garrison buildings. For each cell containing
an occupied building, draws the occupant pip indicators showing which slots are
filled and by which player.

#### Step 16: Band-Box Selection Rect (0x006da180)

Draws the rubber-band selection rectangle when the player is drag-selecting units.

#### Steps 17-19: Second Rally Point/Capture/Placement Draw

The rally point lines, capture links, and building placement ghost are drawn a
**second time** after all objects. This is intentional -- the first call draws them
beneath objects (so they appear behind units), while the second call draws them on
top (so the full line is visible even where objects occlude).

#### Steps 20-21: Radar Event Overlays

- **DrawRadarOverlays_Normal** (0x0063b0a0): Draws radar event markers (combat events,
  attack markers) for allied units. Only runs if `DAT_00ac4cf4` is set (radar available).
- **DrawRadarOverlays_Fog** (0x0063b150): Draws fog-of-war radar event markers.
  Composites event positions with fog alpha from the ABuffer.

#### Step 22: TechnoClass Loop (Selection Circles, Pips, Capture Links)

Iterates all technos (`g_TechnoClass_Count`):
- **Player-controlled units:** If `IsSelected` flag set and `DAT_00843108` (show pips)
  enabled, calls `vtable+0x438` to draw unit pips/veterancy indicators.
  Also draws `CaptureManagerClass::DrawLinks` for mind-control lines.
  Also draws tethered transport lines (for units being carried/picked up).

- **Non-player units:** If detected by sensors (`FUN_0043b150` returns true),
  and unit is a techno with the "active" bit set and not cloaked, calls
  `FUN_004dc340` to draw detection brackets -- small colored squares at the
  endpoints of a line from detecting unit to detected unit, with a pulsing
  animation based on `timeGetTime()`.

#### Step 23: Super Weapon Target Circles (0x0063b2f0)

Draws the target reticle/circle for active super weapons (chronosphere target,
iron curtain target, etc.). Uses `g_SelectedUnitHighlightColor` for the circle color.
Draws a shrinking rectangle with 3 nested outlines.

#### Step 24: PixelFX / Tiberium Glow (0x006d7840)

Per-pixel effect rendering for ore/tiberium glow. Only runs if certain conditions
are met (shroud level check, 16-bit color depth, `DAT_00a8eb78` enabled).
Locks the primary surface and iterates cells in a diamond pattern around the viewport:
- For cells containing ore/tiberium, creates or updates a `PixelFXClass` object
- Applies per-pixel color to the surface based on the PixelFX state
- Directly writes 16-bit RGB values to the surface buffer

#### Step 25: Surface Unlock

```c
(*g_PrimarySurface->vtable + 0x60)();  // Unlock composition surface
g_PrimarySurface = piVar3;              // Restore original primary
```

#### Steps 26-27: Floating Text & Loading Overlay

- **FUN_006d4b50** loop: Draws floating countdown text for EVA timer events.
  Iterates houses and EVA events, formats time as "HH:MM:SS" or "MM:SS", draws
  text using `BitFont__MeasureText` in the bottom-right corner of the viewport.
  Uses `StringTable__LoadString` for localized strings.

- **FUN_006d4e20** (0x006d4e20): Draws centered loading/scenario text overlay.
  If `this+0xA4` is set (a scenario image/text index), draws either an SHP frame
  or centered text on the viewport. Used for mission briefing overlays and loading text.

---

## Surface Architecture Summary

The engine uses a double-buffered approach with separate composition and back surfaces:

| Global | Address | Role |
|--------|---------|------|
| DAT_0088731c | g_CompositionSurface | The main composition surface |
| DAT_008872fc | g_BackSurface | The back buffer for terrain |
| g_PrimarySurface | (varies) | Points to whichever surface is currently being drawn to |
| g_ABuffer | 0x0087e8a4 | Shroud/fog alpha circular buffer |
| g_ZBuffer | 0x00887644 | Depth circular buffer |

**Pass 0:** Scroll handling, potentially swap comp <-> back surfaces
**Pass 1:** Terrain drawn to back surface, ABuffer written with shroud/fog data
**Between:** Sidebar/UI drawn
**Pass 2:** Objects drawn to composition surface, reading ABuffer for alpha blending

---

## When Is the ABuffer Read?

The ABuffer (shroud/fog alpha) is **written** during Pass 1, Step 2 (shroud edges).
It is **read** during Pass 2 whenever objects are drawn with alpha-aware blitters.

Specifically, the audited Pass 1 dirty-rect/full-rect shroud sweep reaches
`AlphaShapeClass__DrawAll_NoMask` (`0x00421350`) from `FUN_006d71e0` at `0x006D754C`,
after shroud/fog edges are written. `AlphaShapeClass__DrawAll_WithMask`
(`0x00420F40`) remains an alpha-shape helper, but this audit did not find it as the
post-shroud call on that path.

Object sprite blitters (SHP drawing) also read the ABuffer to apply fog-of-war
darkening per-pixel during Pass 2.

The ABuffer clear to 0x7F (fully visible) happens:
1. In Pass 0 via `CircBuf__FillAll` (full redraw)
2. In Pass 1 Step 2 via `FUN_00411330` (per dirty rect clear before re-rendering shroud)

---

## Key Addresses Summary

| Function | Address | Phase | Purpose |
|----------|---------|-------|---------|
| RenderFrame_main | 0x004f4480 | -- | Orchestrates all 3 passes |
| TacticalClass_Draw | 0x006d3d10 | 0,1,2 | Master draw dispatcher |
| ZBuffer_rect_clear | 0x007bcf50 | 0 | Full ZBuffer clear |
| CircBuf__FillAll | 0x004112d0 | 0 | Full ABuffer clear |
| CircBuf__Scroll | 0x00410ed0 | 0 | ABuffer circular scroll |
| FUN_007bcb50 | 0x007bcb50 | 0 | ZBuffer circular scroll |
| Tactical_ZBufferDirtyClear | 0x006d2b60 | 1.1 | ZBuffer dirty rect clear |
| Tactical_layer_shroud_edges | 0x006d3660 | 1.2 | ABuffer shroud/fog write |
| Tactical_layer_terrain_shadows | 0x006d2de0 | 1.3 | Terrain shadow decals |
| Tactical_layer_base_terrain | 0x006d3470 | 1.4 | ISO tile rendering + ZBuffer write |
| Tactical_layer_smudges | 0x006d3290 | 1.5 | Crater/scorch decals |
| Tactical_layer_building_overlays | 0x006d3ac0 | 1.6 | Building flat anims |
| Tactical_layer_overlays | 0x006d3040 | 1.7 | Walls/ore/tiberium |
| Tactical_layer_animations | 0x006d3870 | 1.8 | Ground-level flat anims |
| FUN_006d9ce0 | 0x006d9ce0 | 2.1 | Build visible building list |
| FUN_006dad60 | 0x006dad60 | 2.4/17 | Rally point lines |
| FUN_006da9d0 | 0x006da9d0 | 2.5/18 | Mind-control links |
| BuildingPlacement_OverlayRenderer | 0x006d5030 | 2.6/19 | Placement ghost |
| FUN_0053d850 | 0x0053d850 | 2.7 | Waypoint path lines |
| Tactical_ObjectRenderingLoop | 0x006d8db0 | 2.8 | Main object renderer |
| FUN_005fffa0 | 0x005fffa0 | 2.9 | Particle systems |
| FUN_00550240 | 0x00550240 | 2.10 | Laser beams |
| FUN_004c2830 | 0x004c2830 | 2.11 | Electric bolts |
| FUN_00556d40 | 0x00556d40 | 2.12 | Trails/contrails |
| FUN_006591b0 | 0x006591b0 | 2.13 | Wave/sonic effects |
| FUN_006dbe20 | 0x006dbe20 | 2.14 | Selection brackets |
| FUN_00430ac0 | 0x00430ac0 | 2.15 | Garrison pips |
| Tactical__DrawBandBoxRect | 0x006da180 | 2.16 | Band-box selection rect |
| DrawRadarOverlays_Normal | 0x0063b0a0 | 2.20 | Radar event markers |
| DrawRadarOverlays_Fog | 0x0063b150 | 2.21 | Fog radar markers |
| FUN_0063b2f0 | 0x0063b2f0 | 2.23 | Super weapon target circles |
| FUN_006d7840 | 0x006d7840 | 2.24 | PixelFX / tiberium glow |
| FUN_006d4b50 | 0x006d4b50 | 2.26 | Floating timer text |
| FUN_006d4e20 | 0x006d4e20 | 2.27 | Loading/scenario text overlay |
| Shroud_fog_edge_rendering | 0x004801f0 | 1.2 | Per-cell shroud+fog to ABuffer |
| ShroudEdge_BlitToABuffer | 0x0047efe0 | 1.2 | Shroud alpha write |
| FogEdge_BlendToABuffer | 0x0047f250 | 1.2 | Fog alpha blend |
| AlphaShapeClass__DrawAll_NoMask | 0x00421350 | 1.2 / 2 | Alpha shape compositing; confirmed post-shroud call from `FUN_006d71e0` at `0x006D754C` |
| AlphaShapeClass__DrawAll_WithMask | 0x00420f40 | alpha system | Separate alpha-shape helper; not the confirmed post-shroud dirty-rect call |
| FUN_00411330 | 0x00411330 | 1.2 | ABuffer rect clear to 0x7F |
| FUN_004dc340 | 0x004dc340 | 2.22 | Detection bracket drawing |
| CaptureManagerClass__DrawLinks | 0x00472160 | 2.22 | Mind-control link lines |
