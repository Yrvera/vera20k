# Bridge Rendering — Complete Ghidra Report

Reverse-engineered from `gamemd.exe` via Ghidra MCP. All addresses verified from
live decompilation. This covers the full rendering pipeline for bridge overlays,
TMP tiles, shadows, railings, and depth interactions.

> **Correction 2026-08-14 — bridge body depth:** the body call's explicit
> `CC_Draw_Shape` flag-`0x10` gate is zero, so effective flags remain `0x4E00`;
> the native base `-2 - 15 * (signed level + 4)` is a different argument. Stock
> `BRIDGE/BRIDGB` body frames are format 3, so the active route is
> `Blitter_selector_extended @ 0x00490E50` slot `+0x158`, not the standard
> `+0xC0` route. `Extended_SHP_blitter @ 0x00437A10` uses gradient entry 0 and
> dispatches the strict Z-read/write leaf at `0x004990E0`; the candidate changes
> by one native Z unit per full-canvas row. Body-only claims below that rely on
> effective `0x4E10`, `cell+0x10E`, or standard slot `+0xC0` are superseded.

---

## 1. Bridge Rendering Pipeline Overview

Bridge visuals are composed of multiple rendering passes in Phase 1 (terrain pass):

| Component | Rendered By | Pass | Flags | Z Interaction |
|-----------|------------|------|-------|---------------|
| Bridge TMP tiles | `CellOverlay_TileDraw` → `TMP_TileBlitter` | Step 4 (base terrain) | z_enable=1 | Z R+W (per-pixel) |
| Bridge body overlay (SHP) | `DrawOverlay_Body` → `CC_Draw_Shape` | Step 7 (overlays) | `0x4E00`, explicit `0x10` gate zero | Z R+W (format-3 extended slot `+0x158`, leaf `0x004990E0`) |
| Bridge shadow (SHP) | `DrawOverlay_Shadow` → `CC_Draw_Shape` | Step 7 (overlays) | 0x4601 | No Z (shadow blitter) |
| Bridge railing/pavement | `FUN_00547230` → `CC_Draw_Shape` | Step 7 (overlays) | 0x4601 | No Z (shadow blitter) |

There is **NO post-object-pass bridge redraw**. All bridge rendering occurs in
Phase 1 before any game objects (units, buildings) are drawn.

**Confidence:** HIGH — all four rendering paths decompiled and verified.

---

## 2. Screen Position Computation

### 2.1 Base Position (from callers)

Both callers of `DrawOverlay_Body` compute the base position at **ground level**:

**Cell_ContentRendering (0x006d6d10):**
```c
CoordStruct__Set(cellX_lepton, cellY_lepton, 0);  // Z = 0 (ground level!)
piVar7 = CoordsToClient(&dest, &cell_coords);
local_90 = (*piVar7 - viewport_x) - 30;           // -30px X adjustment
local_8c = piVar7[1] - viewport_y;                 // screen Y (no height)
CellClass__DrawOverlay_Body(&local_90, &clip_rect);
```

**FUN_004d1890 — case 0x14:**
Same pattern — `CoordStruct__Set(X, Y, 0)` with Z=0.

### 2.2 CellClass__Get_Draw_Offset (0x00480110)

Called inside `DrawOverlay_Body` to get the cell's draw offset. This is where
the height adjustment and bridge-specific offset are applied.

```c
void CellClass__Get_Draw_Offset(int cell, int* result) {
    int* base = FUN_005fdcc0(overlay_type);  // overlay-type-specific Y offset
    int y_adjust = base[1];                   // typically 0 or -12

    if (cell_flags & 0x80) {
        // Bridge body cell: subtract 16 (0x10)
        y_adjust -= 16;
        if (damage_state >= 9 && damage_state <= 0x11) {
            // Damaged bridge: subtract additional 15
            y_adjust -= 15;  // total: -31
        }
    } else if (overlay_type == 0xEF) {
        // Overlay 239 special case
        y_adjust -= 15;
    }

    int height_y = viewport_y + heightLevel * -15;
    result[0] = base[0] + 30;                    // X = base_x + 30
    result[1] = y_adjust + height_y + 15;        // Y = base_y + heightLevel*-15 + 15 + bridge_adjust
}
```

**Critical findings:**
- **-16 for EW bridges** (damage_state 0-8)
- **-31 for NS bridges** (damage_state 9-17, which includes HEALTHY NS at state 9)
- The `>= 9 && <= 0x11` range check catches ALL NS bridges, not just damaged ones
- **heightLevel * -15 is included** in the returned position
- **+15 constant** added to Y (half tile height centering)

**IMPORTANT CORRECTION:** Earlier analysis incorrectly claimed -31 was only for
damaged bridges. The damage_state field encodes BOTH direction AND damage:
- State 0 = healthy EW, states 1-8 = damaged EW
- State 9 = healthy NS, states 10-17 = damaged NS
- The `>= 9` check catches healthy NS (state 9) too

### 2.3 FUN_005fdcc0 — Overlay Type Draw Offset

Returns an additional Y offset based on the overlay type's properties:

```c
void FUN_005fdcc0(int* result, int overlay_type_id) {
    int overlay_type = OverlayTypeClass_Array[overlay_type_id];
    int y = 0;

    if (overlay_type.flag_0x2a9 || overlay_type.flag_0x2a8
        || overlay_type.tileset_index == 0x7E || overlay_type.flag_0x2aa) {
        y = -12;
    }
    if (overlay_type.AnimRate == 9) {
        y -= 1;
    }
    if (overlay_type_id == 0x7E) {
        y -= 1;
    }

    result[0] = 0;     // X offset = 0
    result[1] = y;      // Y offset (typically 0 or -12)
}
```

For bridge overlays (BRIDGE1=24, BRIDGE2=25): the flags at +0x2A9, +0x2A8, +0x2AA
are NOT set for standard bridge types, so this returns Y=0.

### 2.4 Full Position Chain in DrawOverlay_Body

```c
draw_offset = CellClass__Get_Draw_Offset();
screen_x = draw_offset.x + caller_screen_x - clip_left;
screen_y = draw_offset.y + caller_screen_y - clip_top;

CC_Draw_Shape(shp, frame, &screen_pos, clip_rect, 0x4E00, ...);
```

Inside CC_Draw_Shape (0x4AED70), with 0x200 (center) flag:
```c
screen_x -= shp_full_width / 2;   // center horizontally
screen_y -= shp_full_height / 2;  // center vertically
screen_x += shp_frame_offset_x;   // SHP frame draw offset
screen_y += shp_frame_offset_y;   // SHP frame draw offset
```

### 2.5 Total Screen Y Formula

```
Y = (caller_iso_y_at_Z0)
  + (heightLevel * -15 + 15 - 16)           // from Get_Draw_Offset (bridge cell)
  + (caller_screen_y)                        // redundant with iso_y? depends on caller
  - (clip_top)
  - (shp_full_height / 2)                   // centering
  + (shp_frame_offset_y)                    // SHP frame draw offset
```

The -16 from Get_Draw_Offset is the ONLY bridge-specific screen adjustment.
Direction differences (EW vs NS) come from different SHP sprite dimensions
and frame draw offsets, not from engine code.

---

## 3. Z-Depth / CC_Draw_Shape param_7

### 3.1 param_7 is NOT a Screen Y Offset

In CC_Draw_Shape (0x4AED70):
```c
*(int*)(frame_data + 0x17C) = param_7;  // stored for blitter use
if (param_7 != 0) {
    flags |= 0x10;  // enable Z-buffer interaction
}
```

param_7 is stored in the frame data structure and enables the Z-buffer flag.
It is **never added to the screen position**. The screen position computation
(lines 80-95 of CC_Draw_Shape) only uses:
- Base position from caller
- Centering (-width/2, -height/2) when 0x200 flag is set
- SHP frame draw offset (frame_rect.x, frame_rect.y)

### 3.2 Bridge Z-Depth Values

**Bridge body overlay (corrected argument roles):**
```c
row_z_base = -2 - 15 * (signed(heightLevel) + 4);
CC_Draw_Shape(..., flags=0x4E00, explicit_0x10_gate=0, ..., row_z_base, ...);
```

**Bridge shadow:**
```c
z_depth = heightLevel * -15 - 2;  // ground-level Z (no +4)
CC_Draw_Shape(..., z_depth, 0, 1000, ...);  // Z-height 1000 = default (no Z interaction)
```

### 3.3 Blitter Selection for Bridge Overlays

Effective flags stay `0x4E00` because the explicit `0x10` gate is zero. Since
stock bridge body frames have format byte 3, `CC_Draw_Shape` uses
`Blitter_selector_extended @ 0x00490E50`; `0x4E00` selects slot `+0x158` and
vtable `0x007E53A0` dispatches `+4` to `0x004990E0`. That leaf accepts only
`candidate < stored`, writes both color and candidate Z for opaque literals, and
leaves transparent runs untouched. Gradient entry 0 changes the candidate by
`-1` per full-canvas row.

**Confidence:** HIGH for selection and actual Z R/W behavior of
the 0xC0 blitter (would need to decompile the blitter function itself to confirm).

---

## 4. Bridge Shadow Rendering

### CellClass__DrawOverlay_Shadow (0x47F510)

```c
void DrawOverlay_Shadow(CellClass* cell, int* screen_pos, int* clip_rect) {
    int height = cell->heightLevel;
    int* overlay_type = OverlayTypeClass_Array[cell->overlay_type];
    int* shp = overlay_type->GetShape();
    int* draw_offset = CellClass__Get_Draw_Offset();

    int x = screen_pos[0] + draw_offset[0] - clip_rect[0];
    int y = draw_offset[1] + screen_pos[1] - clip_rect[1];

    // Damaged bridge shadow displacement
    if ((cell_flags & 0x80) && damage_state > 8 && damage_state < 0x12) {
        x -= 15;    // shift left
        y += 7;     // shift down (sagging)
    }

    // Shadow frame = numFrames/2 + body_frame
    int shadow_frame = shp_num_frames / 2 + body_frame;

    CC_Draw_Shape(shp, shadow_frame, &pos, clip_rect,
        0x4601,                    // shadow blitter flags (darken, no Z)
        0,
        height * -15 - 2,          // Z-depth at GROUND level (not bridge deck)
        0,                         // gradient type 0
        1000,                      // Z-height = default (no Z interaction)
        0, 0, 0, 0, 0);
}
```

**Key details:**
- Shadow drawn at **ground-level Z** (heightLevel * -15, no +4 bridge adjustment)
- Z-height = 1000 (default) — shadow doesn't interact with Z-buffer
- Damaged bridge shadows shift: X-15, Y+7 (sagging visual)
- Shadow frame index = numFrames/2 + bodyFrame (second half of SHP file)

---

## 5. Bridge Railing/Pavement Rendering

### FUN_00547230 — Bridge Railing Overlay

Called from `FUN_004802A0` (0x4802A0), which is invoked from `FUN_006D7C00`
during Step 7 (Tactical_layer_overlays @ 0x6D3040). **Not called from
`CellOverlay_TileDraw`** — that function's tail call when `cell+0x48 != -1`
is a smudge vtable+0xA0 dispatch on `g_SmudgeTypeClass_Array[+0x48]`, not
bridge railing. `FUN_00547230` has exactly one caller in the binary.

```c
void DrawBridgeRailing(int overlay_type, int sub_tile, ...) {
    // Lookup railing sprite and offset from bridge data tables
    int tile_idx = overlay_type.tileset_index;

    // Match against known bridge tile ranges
    int railing_shp = bridge_table[offset].shp;
    int railing_x_offset = bridge_table[offset].x;
    int railing_y_offset = bridge_table[offset].y;

    screen_x = base_x + railing_x_offset + 30 + viewport_x - clip_left;
    screen_y = base_y + railing_y_offset + 15 + viewport_y - clip_top;

    CC_Draw_Shape(g_RailingOverlay_SHP, railing_frame,
        &screen_pos, &clip_rect,
        0x4601,          // shadow blitter (darken, no Z)
        0,
        height_param,    // Z-depth from caller (height-based)
        0,
        1000,            // Z-height = default
        0, 0, 0, 0, 0);
}
```

Railings use the shadow blitter (0x4601) — they don't interact with the Z-buffer.
Position is determined by lookup tables indexed by the overlay tileset.

---

## 6. Bridge TMP Tile Rendering

### CellOverlay_TileDraw (0x480350) for Bridge Cells

Bridge deck TMP tiles render through the same path as all terrain tiles:

```c
void CellOverlay_TileDraw(CellClass* cell, int* screen_pos, int* clip, char z_flag) {
    int screen_y = screen_pos[1] + cell->heightLevel * -15;  // height adjustment

    TMP_TileBlitter(
        cell->z_buffer_data,    // cell +0x34
        sub_tile_index,         // cell +0x11A
        g_PrimarySurface,
        screen_x,
        viewport_y + screen_y,
        clip_rect...,
        cell->heightLevel,      // for Z-base computation
        cell->cellZAdjust,      // +0x10C
        1,                      // z_enable = 1 (ALWAYS)
        pavement_flag,          // bit 13 of cell_flags
        0, 0, 0, 0
    );

    // Smudge dispatch when cell+0x48 != -1
    //   (NOT bridge railing — railing is rendered later in Step 7
    //    via FUN_006D7C00 → FUN_004802A0 → FUN_00547230)
    if (cell+0x48 != -1) {
        SmudgeTypeClass_Array[cell+0x48].vtable[0xA0](...);
    }
}
```

**Key: Bridge TMP tiles always have Z R+W enabled.** The `z_enable=1` parameter
is hardcoded. TMP_TileBlitter writes per-pixel Z values using:
```c
pixel_z = z_shape_value + base_z;
if (pixel_z <= zbuffer[pixel]) {
    zbuffer[pixel] = pixel_z;
    screen[pixel] = color;
}
```

For bridge cells with heightLevel = ground + 4, `base_z` is computed from the
elevated heightLevel, producing LOWER Z values (closer to camera). This means
bridge terrain pixels occlude ground-level terrain beneath the bridge.

---

## 7. Bridge Cell Height Values

### heightLevel at CellClass+0x11B

For bridge deck cells, heightLevel is set by two operations:

1. **ApplyBridgeTile (0x57B440):** `cell.heightLevel = sub_tile_height + ground_ref`
2. **Map init (0x59E740):** `cell.heightLevel += 4` (for bridge tile sets)

Result: **heightLevel = ground_height + sub_tile_relative + 4**

For flat bridge deck sub-tiles (sub_tile_height = 0) over water (ground = 0):
`heightLevel = 0 + 0 + 4 = 4`

### Runtime Height Adjustment — FUN_0059e740 (Map Init)

**CRITICAL:** After ApplyBridgeTile sets heightLevel from TMP tile sub-tile
data (which includes deck elevation), the map init function FUN_0059e740
**SUBTRACTS 4** from bridge deck body cells:

```c
// Loop over bridge body cells (case 0 direction, verified in cases 0, 4):
*(char *)(cell + 0x11b) += -4;  // heightLevel -= 4
```

Then specific bridgehead/transition cells get +4 added back:
```c
*(char *)(cell + 0x11b) += 4;   // heightLevel += 4
```

**Net result after all map init adjustments:**
- **Bridge deck body cells:** heightLevel = ground (NOT ground+4)
- **Bridgehead/transition cells:** heightLevel = ground + 4
- **Cells adjacent to bridge (perpendicular):** heightLevel += 4 (elevated to
  match bridge deck for smooth transitions)

This means `Get_Draw_Offset` for bridge body cells uses:
- heightLevel = ground (e.g., 0 for water)
- Bridge offset: -16 (from 0x80 flag)
- Height: ground * -15 = 0

Total Y from Get_Draw_Offset: viewport_y + 0 + 15 - 16 = viewport_y - 1

### GetEffectiveHeight (0x487D50)

```c
int GetEffectiveHeight(CellClass* cell) {
    return cell->heightLevel + ((cell->flags >> 7) & 1) * 4;
}
```

Returns `heightLevel + 4` when bit 0x80 is set. For bridge body cells where
heightLevel = ground, this returns ground + 4 = deck height. Used for unit
positioning (not rendering).

**Note:** GetEffectiveHeight is for MOVEMENT/POSITIONING, not rendering.
The rendering code in Get_Draw_Offset uses heightLevel directly (with its own
-16px adjustment for 0x80 flag, NOT +4 height levels).

---

## 8. Bridge Cell Flags (cell+0x140) and Rendering Effects

| Bit | Mask | Rendering Effect |
|-----|------|-----------------|
| 7 | 0x0080 | Get_Draw_Offset: Y -= 16. DrawOverlay_Body: Z-depth uses heightLevel+4 |
| 8 | 0x0100 | No rendering effect — movement/pathfinding only |
| 9 | 0x0200 | No rendering effect — movement/pathfinding only |
| 10 | 0x0400 | No rendering effect — movement/pathfinding only |
| 11 | 0x0800 | No rendering effect — bridge orientation, setup only |
| 13 | 0x2000 | Pavement flag — passed to TMP_TileBlitter |

Only bits 0x80 and 0x2000 affect rendering. All other bridge flags are for
movement, pathfinding, and setup.

---

## 9. Bridge Damage State — Direction + Damage Encoding

The `bridge_damage_state` field at CellClass+0x11E encodes BOTH direction AND
damage level. Set by `SetBridgeDirection`:
```c
cell->field_0x11e = -(param_2 != 0) & 9;
// param_2 != 0 → state = 9 (NS direction)
// param_2 == 0 → state = 0 (EW direction)
```

### Full State Table

| State | Direction | Condition | SHP Frames | Get_Draw_Offset Y | Frame Selection |
|-------|-----------|-----------|------------|-------------------|-----------------|
| 0 | EW | Healthy | 0-3 | -16 | 0 + random(0-3) |
| 1-8 | EW | Damaged | 1-8 | -16 | state (fixed) |
| 9 | NS | Healthy | 9-12 | -31 | 9 + random(0-3) |
| 10-17 | NS | Damaged | 10-17 | -31 | state (fixed) |
| 18+ | — | Destroyed | N/A | N/A | Bridge removed |

### Frame Selection Logic (from DrawOverlay_Body)

```c
uint frame = cell->bridge_damage_state;  // 0 for EW, 9 for NS
if (frame == 0 || frame == 9) {
    // Healthy: add deterministic pseudo-random variation from cell position
    frame += DAT_0081cc30[(cell.ry & 3) << 2 | (cell.rx & 3)];
}
// frame index used directly as CC_Draw_Shape frame parameter
```

The lookup table `DAT_0081cc30` is stored as `int[16]` (64 bytes total; the
decompilation indexes with `*4` stride). It provides frame offsets in the
range **0-3** — only 4 visual variants per direction. Same cell always gets
the same variant (deterministic from cell.x/y low 2 bits).

### Shadow Frame

```c
shadow_frame = shp_num_frames / 2 + body_frame;
// bridge.tem: 36/2 = 18. Shadow frames 18-35.
// EW shadow: body frame 0-8 → shadow 18-26
// NS shadow: body frame 9-17 → shadow 27-35
```

### Damaged Bridge Visual Effects

- **Body offset:** -31 for NS (same as healthy NS — no additional sag for damaged)
- **Shadow displacement (damaged only, states 9-17 or 1-8 with matching damage):**
  ```c
  if ((cell_flags & 0x80) && damage_state > 8 && damage_state < 0x12) {
      shadow_x -= 15;    // shift left
      shadow_y += 7;     // shift down (sagging visual)
  }
  ```
  Note: This shadow displacement applies to states 9-17 (ALL NS, both healthy
  and damaged). It does NOT apply to EW states 0-8. This means NS bridge
  shadows are always slightly displaced compared to EW shadows.

**CORRECTION:** The "damaged" shadow displacement actually applies to ALL NS
bridges (states 9-17), not just damaged ones. This is the same range check as
Get_Draw_Offset. The shadow displacement for NS bridges is a DIRECTION-dependent
visual feature, not a damage-dependent one.

---

## 10. Ship Visibility Under Bridges

### Why Ships Appear Under Bridges (Geometry, Not Depth)

Ships use TechnoClass::DrawSHP which ORs 0x800, selecting blitters that ignore
the Z-buffer. Bridge Z-values cannot occlude ships. However, ships appear
"under" bridges through pure geometry:

- Bridge deck: heightLevel 4 → 60px above water surface
- Ship sprite: ~30-50px tall, centered at water level (heightLevel 0)
- Ship top pixel: ~25-40px above water surface
- Gap: 20-35px between ship top and bridge bottom
- **Sprites don't overlap** → ship naturally appears below bridge

For very tall ship sprites (Aircraft Carrier, Dreadnought), the gap may be
smaller but typically sufficient. There is NO special bridge rendering pass
to handle ship occlusion.

**No post-object-pass functions are bridge-related.** All post-ObjectRenderingLoop
functions were decompiled and identified:
- FUN_005fffa0: Effect/particle iteration
- FUN_00550240: Laser beam drawing
- FUN_004c2830: Electric bolt/line drawing
- FUN_00556d40: Particle/radiation effects
- FUN_006591b0: Particle effects
- FUN_006dbe20: Waypoint/special overlay drawing
- FUN_00430ac0: Allied spy indicators
- FUN_006da180: Band box selection rectangle

---

## 11. Pre-Computed Z-Adjust Fields

`Cell_ComputeZAdjust` (0x484680) computes three Z-adjust values per cell:

| Field | Offset | Used By | Bridge Role |
|-------|--------|---------|-------------|
| cellZAdjust_top | +0x10A | Normal overlays, buildings | Ground-level Z reference |
| cellZAdjust | +0x10C | TMP_TileBlitter | Per-tile Z-base for terrain |
| cellZAdjust_bottom | +0x10E | Bridge body overlay CC_Draw_Shape | Bridge-level Z reference |

The +0x10E value uses `heightLevel + 4` in its formula, pre-computing the
bridge deck Z regardless of whether the cell actually has a bridge. This means
every cell has a "what if there were a bridge here" Z-adjust ready.

---

## 12. Bridge SHP Frame Data (from retail assets)

### bridge.tem (concrete bridge — shared by BRIDGE1 + BRIDGE2)

Full size: **180x180**, 36 frames (18 body + 18 shadow).

| Frame Range | Direction | frame_y | Purpose |
|-------------|-----------|---------|---------|
| 0-8 | EW | **3** | Body frames (east-west) |
| 9-17 | NS | **18** | Body frames (north-south) |
| 18-26 | EW shadow | **61** | Shadow frames (east-west) |
| 27-35 | NS shadow | **76** | Shadow frames (north-south) |

**The 15px difference** between EW frame_y (3) and NS frame_y (18) = exactly
one CellHeight. This direction difference is BAKED INTO THE SPRITE DATA.

### bridgb.tem (wooden bridge — shared by BRIDGEB1 + BRIDGEB2)

Full size: **253x242**, 36 frames (18 body + 18 shadow).

| Frame Range | Direction | frame_y | Purpose |
|-------------|-----------|---------|---------|
| 0-8 | EW | **34** | Body frames |
| 9-17 | NS | **48-49** | Body frames (~15px lower) |
| 18-26 | EW shadow | **99** | Shadow frames |
| 27-35 | NS shadow | **119** | Shadow frames (~20px lower) |

### lobrdg01.tem / lobrdg02.tem (low bridge)

Full size: **180x120**, 6 frames (3 body + 3 shadow). frame_y = **19** for body.

---

## 13. Complete Position Math Trace — Water Bridge Example

For a bridge over water at cell (100, 100), ground height = 0:

### gamemd.exe

```
heightLevel = 0 (ground; ApplyBridgeTile set ground+4, then map init subtracted 4)
cell_flags & 0x80 = true (bridge body)

Step 1: CoordsToClient(cellX=100*256+128, cellY=100*256+128, Z=0)
  screenY = ((100*30)/2 + (100*30)/2) >> 8 = 3000 >> 8 ≈ 11 (lepton math)
  (actual: iso position at ground level)

Step 2: Get_Draw_Offset
  FUN_005fdcc0: base_y = 0 (bridge overlay type has no special offset)
  bridge adjust: base_y -= 16 → base_y = -16
  height: viewport_y + 0 * -15 = viewport_y
  result_y = -16 + viewport_y + 15 = viewport_y - 1

Step 3: DrawOverlay_Body combines
  local_c = draw_offset_y + screen_y - clip_top = (viewport_y - 1) + iso_y - clip_top

Step 4: CC_Draw_Shape (bridge.tem 180x180)
  0x200 centering: Y -= 180/2 = 90
  frame_y for EW body (frame 0): +3
  final Y shift from centering: -90 + 3 = -87

Total: iso_y - 1 - 87 = iso_y - 88  (EW direction)
Total: iso_y - 1 - 72 = iso_y - 73  (NS direction, frame_y=18)
```

### Our Engine

```
height_map z = 0 (IsoMapPack5 stores ground height for bridge cells)

Step 1: iso_to_screen(100, 100, z=0)
  sy = (100+100)*7.5 + 7.5 - 0*15 = 1507.5

Step 2: bridge_y_offset
  EW: -16
  NS: -31

Step 3: overlay position
  screen_y = 1507.5 + (-16) = 1491.5  (EW)
  screen_y = 1507.5 + (-31) = 1476.5  (NS)

Step 4: atlas centering (bridge.tem 180x180)
  offset_y = -(180/2) + 0 = -90
  Frame blitted at frame_y=3 (EW) or frame_y=18 (NS) within 180px canvas

  final_y EW = 1491.5 + 15 + (-90)  = 1416.5  [TILE_HEIGHT/2 + offset_y]
  final_y NS = 1476.5 + 15 + (-90)  = 1401.5

Pixel where EW body appears: 1416.5 + 3 (frame_y within canvas) = 1419.5
Pixel where NS body appears: 1401.5 + 18 (frame_y within canvas) = 1419.5
→ Both at SAME height! (WAE-style rendering with -16/-31)
```

### Comparison (pixel where bridge body appears, relative to iso_y)

Our engine position[1] includes `+ TILE_HEIGHT/2 (+15)` for cell centering, same
as gamemd.exe's Get_Draw_Offset `+ 15`. Net offsets from iso_y:

**With -16/-31 (WAE-style, current):**
| Direction | Offset chain | Body pixel |
|-----------|-------------|------------|
| EW | -16 + 15 + (-90) + 3 = | **iso_y - 88** |
| NS | -31 + 15 + (-90) + 18 = | **iso_y - 88** |
→ Both at SAME height. WAE-style equal-height bridges.

**With -16/-16 (gamemd.exe-style):**
| Direction | Offset chain | Body pixel |
|-----------|-------------|------------|
| EW | -16 + 15 + (-90) + 3 = | **iso_y - 88** |
| NS | -16 + 15 + (-90) + 18 = | **iso_y - 73** |
→ EW 15px higher than NS. Correct isometric perspective.

**gamemd.exe verified:**
| Direction | Get_Draw_Offset | CC_Draw_Shape center+frame | Body pixel |
|-----------|----------------|---------------------------|------------|
| EW | -1 | -90 + 3 = -87 | **iso_y - 88** |
| NS | -1 | -90 + 18 = -72 | **iso_y - 73** |

**All three approaches match at the pixel level** when comparing equivalent
offset settings. The -16/-31 WAE offsets cancel the SHP frame_y difference.
The -16/-16 gamemd.exe offsets preserve it.

In isometric view, EW and NS bridges SHOULD be at different visual heights
because the projection views them from different angles. gamemd.exe's -16/-16
is isometrically correct. WAE's -16/-31 produces a simplified equal-height result.

---

## 14. Our Engine vs gamemd.exe — Discrepancies

### Bridge Y-Offset: Resolution of the -16 vs -31 Question

**All three rendering approaches produce the SAME visual result:**

| Engine | EW Offset | NS Offset | Why it works |
|--------|-----------|-----------|-------------|
| gamemd.exe | -16 | -16 | SHP frame_y difference (3 vs 18) handled by CC_Draw_Shape centering + frame_offset |
| WAE editor | -16 | -31 | Extra -15 for NS compensates because WAE doesn't apply frame_y the same way |
| Our engine | -16 | -31 | Same as WAE — BUT if our atlas blit matches gamemd.exe's approach, should be -16/-16 |

**Verification math for bridge.tem (180x180):**

gamemd.exe with -16 for both + CC_Draw_Shape centering:
- EW: centering=-90, frame_y=3 → sprite at -90+3 = **-87** from base. Plus -16 = **-103**
- NS: centering=-90, frame_y=18 → sprite at -90+18 = **-72** from base. Plus -16 = **-88**
- **EW is 15px higher than NS** (correct — EW bridge deck is higher in iso view)

WAE with -16/-31, no centering+frame_offset:
- EW: base offset + **-16** = different formula but targets same visual
- NS: base offset + **-31** = -16 - 15 = compensates for missing frame_y difference

Our engine with atlas centering (-(full_h/2)) + frame blit:
- Offset_y = -(180/2) = -90. Frame blitted at frame_y within canvas.
- EW frame at pixel 3: effective = -90 + 3 = -87. Plus bridge_y_offset.
- NS frame at pixel 18: effective = -90 + 18 = -72. Plus bridge_y_offset.

If bridge_y_offset = -16 for both: EW = -103, NS = -88. **Matches gamemd.exe.**
If bridge_y_offset = -16/-31: EW = -103, NS = -103. **Both at same height (WAE style).**

**IMPORTANT:** gamemd.exe renders EW and NS bridge decks at DIFFERENT heights
(EW is 15px higher). WAE renders them at the SAME height. These produce slightly
different visuals. To match gamemd.exe exactly, use -16 for both directions.

**Current recommendation:** Keep -16/-31 (WAE style) for now since it's been
visually tested. When implementing bridge rendering improvements, switch to
-16/-16 (gamemd.exe style) and verify visually.

### Bridge Damage Rendering

Our engine does not currently implement bridge damage states. When implemented:
- Damaged bridges (state 9-17): Y -= 31 (16 + 15) for sagging visual
- Shadow displacement: X -= 15, Y += 7

### Bridge Z-Buffer Interaction

gamemd.exe bridge body overlays DO interact with the Z-buffer (blitter at
vtable 0xC0 selected when Z-depth param is non-zero). Our engine currently
draws bridge overlays with passthrough (no Z interaction). This means:

- gamemd.exe: bridge body pixels write Z values, potentially occluding
  ground-level terrain/objects at the same screen position
- Our engine: bridge body pixels don't interact with depth buffer

This difference is minor for visual correctness since game objects (sprites)
ignore Z anyway in both engines. Bridge Z-writes only affect terrain-vs-bridge
depth ordering.

---

## 13. Address Reference

| Address | Function | Purpose |
|---------|----------|---------|
| 0x47E040 | SetBridgeDirection_NESW | Sets bridge cell flags (0x80, 0x100, etc.) |
| 0x47E470 | SetBridgeDirection_NWSE | Sets bridge cell flags (alternate direction) |
| 0x47F510 | DrawOverlay_Shadow | Bridge shadow rendering |
| 0x47F6A0 | DrawOverlay_Body | Bridge body overlay rendering |
| 0x480110 | Get_Draw_Offset | Cell draw offset with bridge -16 adjustment |
| 0x480350 | CellOverlay_TileDraw | Bridge TMP tile rendering via TMP_TileBlitter |
| 0x484680 | Cell_ComputeZAdjust | Pre-computes Z-adjust fields (+0x10A/C/E) |
| 0x487D50 | GetEffectiveHeight | Returns heightLevel + bridge*4 (for movement) |
| 0x490B90 | Blitter_selector | Selects blitter based on draw flags |
| 0x4AED70 | CC_Draw_Shape | Core SHP drawing function |
| 0x4D1890 | FUN_004d1890 | FoggedObject snapshot walker — TS-legacy, dormant in YR (FogOfWar defaults false; see BRIDGE_DISPLAY_TABLE §2.1) |
| 0x547230 | FUN_00547230 | Bridge railing/pavement overlay |
| 0x547CF0 | TMP_TileBlitter | Per-pixel terrain tile renderer with Z R+W |
| 0x57B440 | ApplyBridgeTile | Sets bridge cell heightLevel during map load |
| 0x5FDCC0 | FUN_005fdcc0 | Overlay-type-specific Y draw offset |
| 0x6D6D10 | Cell_ContentRendering | Overlay rendering in Phase 1 Step 5 (smudges layer, per BRIDGE_DISPLAY_TABLE §2.2) |

---

## 14. Frame Variation Lookup Table (DAT_0081cc30)

The bridge body frame selection adds a pseudo-random offset from a 16-entry
lookup table indexed by `(cell.ry & 3) << 2 | (cell.rx & 3)`:

```
DAT_0081cc30: int[16] = {0, 1, 2, 3, 3, 2, 1, 0, 2, 3, 0, 1, 1, 0, 3, 2}
// 64 bytes total, indexed with *4 stride in the decompilation
```

Values range 0-3 (only 4 variations per direction). Each bridge cell gets a
deterministic visual variant based on its grid position. This prevents long
bridge spans from looking repetitively uniform.

Healthy bridges: `frame = base_state + lookup[cell_pos]` (0-3 for EW, 9-12 for NS)
Damaged bridges: `frame = damage_state` (fixed, no variation)

---

## 15. ZFudgeBridge — Unit Depth Adjustment Under Bridges

### INI Configuration

```ini
ZFudgeBridge=7    ; per TechnoType, fudge for tall units under bridges
TooBigToFitUnderBridge=true  ; prevents unit from going under bridges entirely
```

`ZFudgeBridge` is read in `TechnoTypeClass::ReadINI` (0x715486) and stored
in the TechnoTypeClass struct (exact offset not yet mapped). Value = 7 for
most units. `TooBigToFitUnderBridge` stored at UnitTypeClass+0xE16.

### Purpose

Adjusts the Z-depth of units when they pass under bridge cells, preventing
tall unit sprites from overlapping with bridge deck pixels. Without this fudge,
units like the Mammoth Tank (tall sprite) could show through the bridge deck.

### Struct Offsets (verified from ReadINI decompilation)

| Offset | Field | INI Key | Default |
|--------|-------|---------|---------|
| TechnoTypeClass+0xDC0 | ZFudgeCliff | ZFudgeCliff | varies |
| TechnoTypeClass+0xDC4 | ZFudgeColumn | ZFudgeColumn | 8-9 |
| TechnoTypeClass+0xDC8 | ZFudgeTunnel | ZFudgeTunnel | 13-15 |
| TechnoTypeClass+0xDCC | ZFudgeBridge | ZFudgeBridge | 7 |
| UnitTypeClass+0xE16 | TooBigToFitUnderBridge | TooBigToFitUnderBridge | false |

### ComputeZFudge Function (0x4DAFF0) — FULLY TRACED

```c
int ComputeZFudge(TechnoClass* techno) {
    int base_z = techno->locomotor->GetZHeight();

    TechnoTypeClass* type = techno->GetType();

    // Each fudge = INI value × proximity factor from helper function
    int column_fudge = type->ZFudgeColumn * IsNearColumn();   // 0x703E70
    int tunnel_fudge = type->ZFudgeTunnel * IsNearTunnel();   // 0x704000
    int cliff_fudge  = type->ZFudgeCliff  * IsNearCliff();    // 0x704240

    // Bridge fudge: raw value (no multiplier), only when near bridge
    int bridge_fudge = IsNearBridge() ? type->ZFudgeBridge : 0;  // 0x703B10

    // Take MAXIMUM of all fudge values
    int max_fudge = max(max(column_fudge, tunnel_fudge),
                        max(cliff_fudge, bridge_fudge));

    int additional = FUN_00704350();  // unknown additional Z adjustment
    return additional + max_fudge + base_z;
}
```

The result is used as the Z-depth parameter in CC_Draw_Shape calls (param_7),
which controls the per-scanline Z-gradient base value. A larger fudge pushes
the unit FURTHER from the camera in Z-depth, making it sort behind nearby
structures in the Z-buffer.

**Note:** Since sprites use 0x800 flag (ignoring Z-buffer), this fudge does
NOT affect sprite visibility through Z-buffer occlusion. Instead, it affects
the Z-depth stored at `frame_data + 0x17C` which is used by the per-scanline
Z-gradient system (entry 0 in `DAT_00817710`) to compute the base_z for
each scanline of the sprite. A higher fudge raises base_z, which can cause
the sprite to be occluded by terrain/wall Z-values if those were written by
TMP_TileBlitter.

### IsNearBridge Check (0x703B10)

```c
bool IsNearBridge(TechnoClass* techno) {
    CoordStruct pos = techno->GetRenderCoords();
    CellClass* cell = GetCellAt(pos);

    if (cell == NULL || techno->on_bridge) return false;  // skip if ON bridge

    // Check current cell + 4 diagonal neighbors
    CellClass* neighbors[4] = GetNeighborCells(pos, NE/NW/SE/SW);

    return (cell->flags & 0x100)                                    // current cell is bridge
        || (neighbor_NE->flags & 0x100 && !(flags & 0x800))        // NE + not NS orientation
        || (neighbor_SE->flags & 0x100 && (flags & 0x800) == 0)    // SE + not NS
        || (neighbor_SW->flags & 0x100 && !(flags & 0x800))        // SW + not NS
        || (neighbor_NW->flags & 0x100 && (flags & 0x800) != 0);   // NW + NS orientation
}
```

Key: `(char)param_1[0x23] == '\\0'` checks that the unit is NOT on the bridge
(on_bridge flag at FootClass+0x8C = offset 0x23 in int* terms). ZFudgeBridge
only applies when the unit is at GROUND LEVEL near a bridge cell, not when
the unit is ON the bridge deck.

---

## 16. Rendering Optimization Cache (DrawOverlay_Body)

Bridge body rendering includes a per-cell frame cache to avoid redundant
draw calls:

```c
// Skip if already drawn this frame with same viewport
if (cell+0x64 == g_CurrentFrameCounter
    && cell+0x118 == DAT_00880940
    && cell+0x68..+0x74 == clip_rect) {
    return;  // already drawn, skip
}

// ... render bridge ...

// Save cache for next check
cell+0x64 = g_CurrentFrameCounter;
cell+0x68..+0x74 = clip_rect;
cell+0x118 = DAT_00880940;
```

| Offset | Type | Field | Purpose |
|--------|------|-------|---------|
| +0x64 | u32 | last_draw_frame | Frame counter when last drawn |
| +0x68 | i32[4] | last_clip_rect | Clip rectangle when last drawn |
| +0x118 | u8 | last_draw_state | DAT_00880940 value when last drawn |

`DAT_00880940` appears to be a rendering pass identifier or surface index,
ensuring the cache is invalidated when the render target changes.

---

## 17. Open Questions

1. ~~**Blitter 0xC0 Z behavior**~~ **FULLY RESOLVED.** Blitter at vtable
   0x7E5618 decompiled (FUN_00495A50, 218 bytes). The inner loop:
   ```c
   if (z_depth < *zbuffer_pixel && *sprite_pixel != 0) {
       *screen_pixel = remapped_color;   // WRITE pixel
       *zbuffer_pixel = z_depth;          // WRITE Z to buffer
   }
   ```
   Uses `<` (Less) compare, same as TMP_TileBlitter. Bridge body overlay
   pixels READ the Z-buffer, only draw if closer, then WRITE their Z-depth.
   This is how bridge deck pixels occlude ground-level terrain beneath.

2. ~~**SHP frame draw offsets for BRIDGE1 vs BRIDGE2**~~ **RESOLVED:**
   Both use bridge.tem (180x180, 36 frames). EW frames have frame_y=3,
   NS frames have frame_y=18. The 15px difference is baked into the sprite.
   Our -16/-31 offsets correctly compensate for direction, matching gamemd.exe
   where the damage_state field (0=EW, 9=NS) triggers -16 or -31 respectively
   in Get_Draw_Offset.

3. ~~**Bridge cell IsoMapPack5 z values**~~ **RESOLVED.** Must be GROUND height
   (not deck). Proof: ApplyBridgeTile sets heightLevel from tile sub-tile
   data, then FUN_0059e740 subtracts 4 for body cells, giving heightLevel =
   ground. Our engine uses IsoMapPack5 z directly. If z were deck height
   (ground+4), bridges would render ~60px too high with our -16 offset.
   Since bridges look correct with -16, z = ground in the map file.

4. ~~**Low bridge rendering**~~ **RESOLVED:** LOBRDG/LOBRDB overlays render
   at ground level. lobrdg01.tem is 180x120, frame_y=19, no height offset
   needed. Low bridges don't have the 0x80 cell flag, so Get_Draw_Offset
   applies no bridge adjustment.

5. ~~**ZFudgeBridge exact rendering usage**~~ **RESOLVED.** ComputeZFudge
   at 0x4DAFF0 computes max(cliff, column, tunnel, bridge) fudge.
   ZFudgeBridge=7 applied when IsNearBridge (0x703B10) returns true
   (unit at ground level near bridge structural cell). Result added to
   Z-depth parameter of CC_Draw_Shape. Affects per-scanline Z-gradient
   base value, not screen position. At TechnoTypeClass+0xDCC.

6. **Bridge railing data tables:** DAT_00abc210 and DAT_00abc2d0 contain
   bridge railing sprite indices and position offsets. 16-byte entries with
   {shp_index, sub_tile, x_offset, y_offset}. Not fully mapped.

7. **Bridge overlay display layer entries:** Case 0x14 entries in the display
   layer contain overlay type and damage state overrides that are temporarily
   applied to the cell before DrawOverlay_Body. The structure of these display
   entries needs full documentation.
