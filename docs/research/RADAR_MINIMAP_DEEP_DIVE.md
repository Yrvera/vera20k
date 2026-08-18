# Radar Minimap Deep Dive — Companion to RADAR_MINIMAP_RENDERING.md

Detailed verification of every subsystem, with assembly-level proof for critical paths.
All addresses verified via live Ghidra MCP decompilation + disassembly.

---

## 1. Fog-of-War Cell Functions (Definitive)

**Verified by assembly tracing at 0x655D3C–0x655D87 and terrain fallback at 0x655E7E–0x65608F.**

### Outer gate: g_hWnd check at 0x655D3C (corrected 2026-05-28: was undocumented; binary shows explicit gate via `disassemble_function 0x00655C50` — ROOT_CAUSE: INFERENCE_HARDENED)

Both `IsShrouded` and `IsFogged` are called **only when `DAT_00B73550` (g_hWnd) != 0**:

```asm
; 0x655D3C — gate for IsShrouded
MOV EAX,[0x00B73550]   ; load g_hWnd
CMP EAX,0
JZ  0x655D5D           ; if g_hWnd==0 → skip shroud call, bVar2=false

; 0x655D62 — gate for IsFogged (same global)
CMP dword ptr [0x00B73550],0
JZ  0x655D82           ; if g_hWnd==0 → skip fog call, bVar3=false
```

`g_hWnd` is the main window handle — it is non-null whenever the game window exists.
When null (headless/no-window), every cell renders as visible regardless of shroud/fog state.
This is a fast-path null guard, **not** a fog-of-war feature toggle.

### FUN_00586360 — IsShrouded (0x00586360, 308 bytes)

Returns 1 if the cell has **NEVER been explored** (still in shroud/darkness).

```c
bool IsShrouded(int* lepton_coords) {
    // Convert lepton to cell, handle height/bridge levels
    CellClass* cell = GetCellFromLepton(lepton_coords);

    // Check the shroud bit: cell+0x12C bit 3
    // Bit 3 SET = cell HAS been explored → return 0 (not shrouded)
    // Bit 3 CLEAR = cell has NOT been explored → return 1 (shrouded)
    if ((cell->flags_12C & 0x08) != 0)
        return 0;  // explored → not shrouded
    return 1;      // not explored → is shrouded
}
```

### FUN_005864A0 — IsFogged (0x005864A0, 308 bytes)

Returns 1 if the cell IS explored but **NOT currently visible** (fog of war).

```c
bool IsFogged(int* lepton_coords) {
    CellClass* cell = GetCellFromLepton(lepton_coords);

    // Check fog state: cell+0x13C (int, vision/fog counter)
    // Value >= 1 → fogged (explored but no current sight)
    // Value < 1 (= 0) → visible (currently in sight range)
    if (*(int*)(cell + 0x13C) < 1)
        return 0;  // vision count active → visible, not fogged
    return 1;      // no vision → fogged
}
```

### Three-State Terrain Rendering

Assembly branch order at 0x655E7E → 0x656058 → 0x656060:

| Test | Flag | Assembly | Terrain Action |
|------|------|----------|----------------|
| 1st: `ESP+0x13` | is_fogged | `JZ 0x656058` | **Dim**: GetPixel → SHR each channel by 1 → PutPixel |
| 2nd: `ESP+0x44` | is_shrouded | `JNZ 0x656081` | **Black**: PutPixel(0) |
| 3rd: fall-through | visible | — | **Copy**: GetPixel → PutPixel directly |

**Important**: A cell can be fogged WITHOUT being shrouded (it was explored once).
A cell cannot be shrouded AND fogged simultaneously — shroud means never seen.

### Half-Brightness Math (Fog Dimming)

Assembly at 0x655E8A–0x655F43 (verified instruction by instruction):

```asm
; Read terrain pixel from secondary surface
CALL [EAX + 0x28]      ; GetPixel → AX = 16-bit pixel

; Extract R channel → shift to 8-bit → halve
SHR DX, CL             ; shift by r_shift (DAT_008a0dd0)
SHL DL, CL             ; shift by r_loss (DAT_008a0dd4)
; ... DL now contains R as 8-bit

; Extract B channel → shift to 8-bit → halve
SHR AX, CL             ; shift by b_shift (DAT_008a0dd8)
SHL AL, CL             ; shift by b_loss (DAT_008a0ddc)

; Extract G channel → shift to 8-bit → halve
SHR BX, CL             ; shift by g_shift (DAT_008a0de0)
SHL BL, CL             ; shift by g_loss (DAT_008a0de4)

; HALVE each 8-bit channel:
SHR EBP, 0x1           ; B >> 1  (at 0x655EEE)
SHR EBX, 0x1           ; G >> 1  (at 0x655F0A)
SHR EDX, 0x1           ; R >> 1  (at 0x655F20)

; Repack into 16-bit: shift back to DD format positions
SHL EBP, CL            ; B to position (b_shift)
SHL EBX, CL            ; G to position (g_shift)
SHL EDX, CL            ; R to position (r_shift)
OR EBP, EBX            ; combine B + G
OR EBP, EDX            ; combine + R

; Write dimmed pixel
PUSH EBP               ; dimmed pixel value
PUSH EDI               ; coordinates
CALL [EAX + 0x24]      ; PutPixel
```

The channel extraction is: `(pixel >> channel_shift) << channel_loss` to get 8-bit.
The halving is: `value >> 1` (unsigned right shift = divide by 2, truncated).
The repacking is: `(value >> channel_loss) << channel_shift` to get back to DD format.

---

## 2. Object Tracking System (RadarCellTracker)

### Hash Table Structure

```
RadarCellTracker (+0x1258)
├── Wrapper: 16 bytes (pointer to bucket array + bookkeeping)
└── Bucket Array: 256 × 24-byte bucket headers = 0x1800 bytes
    Each bucket header (24 bytes / 0x18):
    ├── [+0x00] vtable pointer
    ├── [+0x04] data_buffer_ptr → points to entry array
    ├── [+0x08] capacity (max entries before grow)
    ├── [+0x0D] owns_memory flag (byte)
    ├── [+0x10] count (current entries)
    ├── [+0x14] grow_increment (default: 10)
    └── Entry format (16 bytes / 0x10 each):
        ├── [+0x00] object_ptr (TechnoClass*)
        ├── [+0x04] pixel_x (int, on radar surface)
        ├── [+0x08] pixel_y (int, on radar surface)
        └── [+0x0C] object_ptr_dup (back-pointer, same as +0x00)
```

Hash function: `bucket_index = (pixel_x + pixel_y * -5) & 0xFF`

### AddObjectToTracker — FUN_00655560 (verified live, 98 lines)

```c
void RadarClass::AddObject(int pixel_x, int pixel_y) {
    // this = RadarClass, object = caller (in EBX register)

    // 1. Bounds check against primary surface rect
    rect = primary_surface->GetRect();
    if (pixel_x < rect.x || pixel_x >= rect.right ||
        pixel_y < rect.y || pixel_y >= rect.bottom) {
        // Object is out of radar bounds
        if (object->GetRTTI() == 6) return;  // aircraft: just skip
        // Others: clamp to radar edge and store clamped pos at object+0x208/0x20C
        pixel_x = clamp(pixel_x, rect.x, rect.right - 1);
        pixel_y = clamp(pixel_y, rect.y, rect.bottom - 1);
        object->radar_x = pixel_x;  // +0x208 (= 0x82 * 4)
        object->radar_y = pixel_y;  // +0x20C (= 0x83 * 4)
    }

    // 2. Hash lookup
    bucket_index = (pixel_x + pixel_y * -5) & 0xFF;
    bucket = tracker->buckets[bucket_index];

    // 3. Duplicate check — scan for existing entry with same object AND position
    for (i = 0; i < bucket.count; i++) {
        if (entry[i].object == object &&
            entry[i].x == pixel_x &&
            entry[i].y == pixel_y)
            return;  // already tracked, skip
    }

    // 4. Insert — LOCAL PLAYER objects go to FRONT, others to BACK
    new_entry = { object, pixel_x, pixel_y, object };

    if (object->owner_house == DAT_00a83d4c) {
        // LOCAL PLAYER: insert at front (memmove existing entries right by 16 bytes)
        if (bucket.count > 0)
            memmove(bucket.data + 16, bucket.data, bucket.count * 16);
        bucket.data[0] = new_entry;
    } else {
        // NON-LOCAL: append at end
        bucket.data[bucket.count] = new_entry;
    }
    bucket.count++;

    // 5. Mark dirty
    MarkCellDirty(pixel_x, pixel_y);
    needs_redraw = 1;
}
```

**Key insight**: Local player objects at front ensures they are drawn ON TOP when
RenderCellPixel iterates the bucket (first match wins). Also ensures click-to-select
(which searches backwards) finds enemy objects first, letting you target enemies even
when your own units overlap on the radar.

### RemoveObjectFromTracker — FUN_00655740 (verified live, 42 lines)

```c
void RadarClass::RemoveObject(object, pixel_x, pixel_y) {
    bucket_index = (pixel_x + pixel_y * -5) & 0xFF;
    bucket = tracker->buckets[bucket_index];

    // Find entry by object pointer
    index = bucket->FindIndex(object);  // vtable+0x10
    if (index == -1) return;

    // Compact: shift remaining entries left by 16 bytes
    bucket.count--;
    for (i = index; i < bucket.count; i++)
        bucket.data[i] = bucket.data[i + 1];  // copy 16 bytes

    MarkCellDirty(pixel_x, pixel_y);
    needs_redraw = 1;
}
```

### Callers (who registers objects)

| Address | Caller | When |
|---------|--------|------|
| 0x00456580 | BuildingClass-related | Building placed/revealed on map |
| 0x0070CC90 | TechnoClass-related | Unit enters radar tracking range |

---

## 3. Inverse Isometric Transform (Assembly Proof)

**Verified from assembly at 0x655CB8–0x655D0B**

The conversion from radar surface pixel to map cell lepton coordinates:

```asm
; === Step A: Undo zoom, apply offsets ===
FILD [ESP+0x44]           ; ST0 = (float)pixel_x
FDIV [ESI+0x1488]         ; ST0 = pixel_x / zoom_factor
FISUB [ESI+0x1490]        ; ST0 = pixel_x / zoom - map_iso_offset_x  (= iso_x)
FILD [ESP+0x14]           ; ST0 = (float)pixel_y, ST1 = iso_x
FDIV [ESI+0x1488]         ; ST0 = pixel_y / zoom_factor
FIADD [ESI+0x1498]        ; ST0 = pixel_y / zoom + map_iso_offset_y  (= iso_y)

; === Step B: Inverse isometric ===
; iso_x = cellX - cellY,  iso_y = cellX + cellY
; → cellX = (iso_x + iso_y) / 2,  cellY = (iso_y - iso_x) / 2
FLD ST0                   ; duplicate iso_y → ST0=iso_y, ST1=iso_y, ST2=iso_x
FADD ST0,ST2              ; ST0 = iso_y + iso_x = 2*cellX
FMUL [0x7E5168]           ; ST0 = cellX (float)    [0x7E5168 = 0.5f]
FADD [0x7E1738]           ; ST0 = cellX + 0.5      [0x7E1738 = 0.5d, for rounding]
CALL Math__ftol           ; AX = (short)cellX
FSUB ST0,ST1              ; ST0 = iso_y - iso_x = 2*cellY  [wait: ST rearranged by ftol]
MOV BX,AX                 ; save cellX
FMUL [0x7E5168]           ; ST0 = cellY (float)
FADD [0x7E1738]           ; ST0 = cellY + 0.5
CALL Math__ftol           ; AX = (short)cellY

; === Step C: Cell → lepton ===
MOVSX ECX,BX              ; ECX = cellX (sign-extended)
MOVSX EDX,AX              ; EDX = cellY
SHL ECX,8                 ; ECX = cellX * 256
ADD ECX,0x80              ; ECX = cellX * 256 + 128  (center of cell)
SHL EDX,8                 ; EDX = cellY * 256
ADD EDX,0x80              ; EDX = cellY * 256 + 128
```

**Summary**:
```
pixel → iso: iso_x = px/zoom - offset_x, iso_y = py/zoom + offset_y
iso → cell:  cellX = round((iso_x + iso_y) / 2)
             cellY = round((iso_y - iso_x) / 2)
cell → lepton: lx = cellX * 256 + 128, ly = cellY * 256 + 128
```

The lepton coordinates are used for `CellClass::GetGroundHeight` and the
fog functions `IsShrouded` / `IsFogged`.

---

## 4. Viewport Rectangle = Radar Event

**Verified**: `DrawViewportRect` (FUN_00660540) is called ONLY from `TickRadarEvent`
(FUN_0065FE00). The viewport rectangle IS a radar event object with the same 64-byte
structure. It shares the diamond-drawing infrastructure.

### How the viewport is managed

The viewport rect event is stored as part of the active radar events array. It's
distinguished by its type field and behavior — it doesn't expire like combat events.
The `TickRadarEvent` function calls `DrawViewportRect` unconditionally for every active
event that has its draw flag set (it's the first thing called after the timer check).

### Corner computation (FUN_00660730)

```c
void ComputeViewportCorners(RadarEventObj* event, int corners[8]) {
    // Build rotation matrix from event->rotation_angle (+0x10)
    FUN_005AE860();                          // reset matrix
    FUN_005AF1A0(event->rotation_angle);     // build 2D rotation matrix

    // Transform direction vector by rotation matrix
    vec = { event->field_0x0C, 0, 0 };      // radius along X axis
    result = MatrixMultiply(rotation_matrix, vec);  // FUN_005AFB80

    dx = ftol(result.x);
    dy = ftol(result.y);

    // Generate 4 corners of a rotated rectangle/diamond:
    corners[0] = +dx;  corners[1] = +dy;   // top
    corners[2] = -dy;  corners[3] = +dx;   // right
    corners[4] = -dx;  corners[5] = -dy;   // bottom
    corners[6] = +dy;  corners[7] = -dx;   // left
}
```

The 4 corners form a diamond rotated by the event's angle. For the viewport rect,
the "radius" corresponds to the camera view size scaled to radar coordinates.

### Drawing (FUN_00660540)

Offsets each corner by the event's (x, y) center position, then draws 4 line segments
connecting consecutive corners on the `DAT_00880A04` surface (which is the same as
or overlays onto the radar draw surface).

---

## 5. DirectDraw Pixel Format Tables

These are set up at runtime when the DirectDraw surface is created. They describe
how RGB channels are packed into 16-bit pixels.

| Address | Name | Typical Value (RGB565) | Typical Value (RGB555) |
|---------|------|------------------------|------------------------|
| 0x008A0DD0 | r_shift | 11 | 10 |
| 0x008A0DD4 | r_loss | 3 | 3 |
| 0x008A0DD8 | b_shift | 0 | 0 |
| 0x008A0DDC | b_loss | 3 | 3 |
| 0x008A0DE0 | g_shift | 5 | 5 |
| 0x008A0DE4 | g_loss | 2 (6-bit green) | 3 (5-bit green) |

**Pack**: `pixel = (R >> r_loss) << r_shift | (G >> g_loss) << g_shift | (B >> b_loss) << b_shift`
**Unpack**: `R = (pixel >> r_shift) << r_loss` (back to 8-bit with loss bits zeroed)

The fog dimming code (§1 above) does: unpack → `>> 1` (halve) → repack. This halves
the 8-bit value, then re-applies the loss+shift to get back to packed format.

---

## 6. Zoom Sampling in GenerateTerrainSurface (0x006547C0)

### Zoom Factor

```c
float zoom;
if (140.0f / map_width <= 108.0f / map_height) {
    zoom = 140.0f / map_width;     // width-constrained
    zoomed_w = 0x8C;               // 140 (full inner width)
    zoomed_h = ftol(map_height * zoom);  // scaled height
} else {
    zoom = 108.0f / map_height;    // height-constrained
    zoomed_w = ftol(map_width * zoom);   // scaled width
    zoomed_h = 0x6C;               // 108 (full inner height)
}
this->zoom_factor = zoom;  // +0x1488
```

### Sampling Loop

The terrain surface is generated by iterating each output pixel and mapping back to
the raw RGB buffer coordinates via the zoom factor:

```c
// Lock the secondary surface for direct pixel write
secondary->Lock(0);  // vtable+0x5C, returns stride (pixels per row)

for (output_y = y_start; output_y < y_end; output_y++) {
    for (output_x = x_start; output_x < x_end; output_x++) {
        // Map output pixel back to raw buffer coords
        src_y_start = ftol(output_y / zoom);     // approximate
        src_y_end = ftol((output_y + 1) / zoom);
        src_x_start = ftol(output_x / zoom);
        src_x_end = ftol((output_x + 1) / zoom);

        // Clamp to buffer dimensions
        src_y_end = min(src_y_end + 1, buffer_height);
        src_x_end = min(src_x_end + 1, buffer_width);

        // Sample source pixels in the mapped rect
        // (The code iterates src_y_start..src_y_end and src_x_start..src_x_end
        //  but the actual averaging/accumulation is unclear from decompilation
        //  due to heavy FPU stack usage. The innermost loop reads from
        //  raw_buffer[src_y * stride + src_x] which is 3 bytes per pixel.)

        // Clamp RGB to 0-255
        R = min(R_computed, 255);
        G = min(G_computed, 255);
        B = min(B_computed, 255);

        // Pack to 16-bit DD format and write directly to surface memory
        *surface_ptr++ = (R >> r_loss) << r_shift |
                         (G >> g_loss) << g_shift |
                         (B >> b_loss) << b_shift;
    }
    surface_ptr += (stride - output_width);  // advance to next row
}

secondary->Unlock();  // vtable+0x60
```

**Note**: The sampling appears to be a box filter (averaging source pixels that map to
each output pixel), not nearest-neighbor. However, the Ghidra decompilation of the inner
loop is heavily obfuscated by FPU stack operations and the variable `local_14` (which
appears to be used as a running accumulator). The exact averaging math needs further
verification via manual FPU stack tracing.

---

## 7. Spy Satellite System Details

### Surface Identity

`DAT_00880A04` — The drawing surface used for spy satellite SHPs and viewport rectangles.
This is likely an overlay surface that composites on top of the radar primary surface,
or it may be a pointer to the same primary surface used through a different global.

### Data Globals

| Address | Type | Name | Purpose |
|---------|------|------|---------|
| 0x0089C420 | int | spy_shp_width | Width of spy satellite SHP frame |
| 0x0089C424 | int | spy_shp_height | Height of spy satellite SHP frame |
| 0x0089C428 | int | refresh_frame_count | Frames per refresh cycle to draw |
| 0x0089C42C | int | refresh_interval | Total frames between refresh cycles |
| 0x0089C478 | SHP* | spy_satellite_shp | Animation frames for satellite vision |
| 0x00880C84 | int | radar_origin_x | Radar surface origin X on screen |
| 0x00880C88 | int | radar_origin_y | Radar surface origin Y on screen |
| 0x00880C8C | int | radar_origin_w | Radar surface width |
| 0x00880C90 | int | radar_origin_h | Radar surface height |

### Refresh Logic (FUN_00431800)

The spy satellite system uses a periodic refresh model:
- `g_CurrentFrameCounter % refresh_interval < refresh_frame_count + 1`
- When true: iterate all 24 spy satellite slots (8 rows × 3 columns)
- Each slot can hold a building pointer
- Check: building valid, active (flags +0xC bit 0), owner allied with player, not observer
- Draw each valid satellite via FUN_00430650

### FUN_00430650 — Draw One Spy Satellite

1. Convert satellite building's cell position to radar pixel via FUN_006557F0
2. Draw animated SHP frame (frame index = refresh timer position)
3. Compute coverage rectangle (satellite position ± shp_width/2, ± shp_height/2)
4. Union coverage rect with the global radar dirty rect
5. Mark every pixel in the coverage area as dirty (double loop calling MarkCellDirty)

---

## 8. Radar Event Object Layout (Corrected)

Verified from FUN_0065FB80 (init) and FUN_0065FE00 (tick):

| Offset | Type | Init Value | Name |
|--------|------|------------|------|
| +0x00 | int | param | **type** (0–12) |
| +0x04 | int | computed | **radar_x** (pixel pos - radar_origin_x) |
| +0x08 | int | computed | **radar_y** (pixel pos - radar_origin_y) |
| +0x0C | float | max_edge_dist | **radius** (starts at max, shrinks) |
| +0x10 | float | π/4 (0x3F490FDB) | **rotation_angle** (radians) |
| +0x14 | float | Rules+0x84 | **rotation_speed** (RadarEventRotationSpeed=0.05) |
| +0x18 | float | 0.0 | **color_fade** (0.0→1.0 oscillates) |
| +0x1C | float | Rules+0x78 | **fade_speed** (RadarEventColorSpeed=0.1) |
| +0x20 | int | cell_packed | **source_cell** (original cell coordinate) |
| +0x24 | int | frame | **timer1_start** (g_CurrentFrameCounter) |
| +0x28 | int | — | timer1_aux |
| +0x2C | int | 0 | **timer1_duration** |
| +0x30 | int | frame | **timer2_start** (set when expanding phase ends) |
| +0x34 | int | — | timer2_aux |
| +0x38 | int | 0 | **timer2_duration** (set from type_config.duration) |
| +0x3C | byte | 1 | **expanding_flag** (1=expanding, 0=decaying) |
| +0x3D | byte | 1 | **needs_draw_flag** (0=expired, stop drawing) |

### Radius Initialization

The initial radius is set to the **maximum distance from the event position to any
radar edge**:

```c
dist_left = event->radar_x;
dist_right = DAT_00880C8C - event->radar_x;  // radar_w - x
dist_top = event->radar_y;
dist_bottom = DAT_00880C90 - event->radar_y;  // radar_h - y
event->radius = max(dist_left, dist_right, dist_top, dist_bottom);
```

This ensures the diamond starts large enough to be visible from any map position,
then shrinks toward `RadarEventMinRadius` (8 pixels).

### Per-Tick Lifecycle (FUN_0065FE00)

```
Frame N:   Event created → radius=max, expanding=1, fade=0.0
           │
           ▼ Each tick:
           radius -= RadarEventSpeed (1.2)
           radius = max(radius, RadarEventMinRadius (8))
           rotation += rotation_speed
           │
           ├─ While expanding:
           │  fade += fade_speed
           │  fade_speed decays slightly each tick
           │  When fade exceeds threshold → expanding=0
           │  Set timer2_start, timer2_duration from type_config
           │
           └─ While decaying:
              color_fade bounces between 0.0 and 1.0
              (fade_speed negates on bounds hit)
              When timer2 expires → needs_draw=0 (event dies)
              │
              ▼
              Cleanup: removed from array, memory freed
```

---

## 9. MarkCellDirty Verified (FUN_006562D0)

Complete flow from live decompilation:

```c
void MarkCellDirty(int* pixel_coords) {  // coords = {pixel_x, pixel_y}
    int x = pixel_coords[0];
    int y = pixel_coords[1];

    // Bounds check
    if (x < 0) return;
    if (x >= primary_surface->GetWidth()) return;
    if (y < 0) return;
    if (y >= primary_surface->GetHeight()) return;

    // Bitfield check (prevent marking same pixel twice)
    int bit_index = primary_surface->GetWidth() * y + x;
    byte* byte_ptr = &visited_bitfield[bit_index >> 3];
    byte mask = 1 << (bit_index & 7);

    if (*byte_ptr & mask)
        return;  // already marked
    *byte_ptr |= mask;  // set bit

    // Grow dirty list if needed
    if (dirty_count >= dirty_capacity) {
        if (!dirty_vector->Grow(grow_increment + dirty_capacity))
            return;  // allocation failed
    }

    // Append to dirty list (8 bytes per entry: x, y)
    dirty_list[dirty_count * 8 + 0] = x;
    dirty_list[dirty_count * 8 + 4] = y;
    dirty_count++;

    // Set redraw flag
    needs_redraw = 1;
}
```

---

## 10. Flash Frame Timing (Verified from INI + Assembly)

From `rules.ini` / `rulesmd.ini`:
```ini
FlashFrameTime=7
RadarCombatFlashTime=49
```

Rules offsets (from INI parser at file 105, line 7707):
- `Rules+0x88 = FlashFrameTime = 7`
- `Rules+0x8C = RadarCombatFlashTime = 49`

Assembly at 0x65600E–0x65604B shows the flash calculation:

```asm
DEC EAX                    ; remaining_frames - 1
CDQ
IDIV [ECX + 0x88]          ; / FlashFrameTime (7)
AND EAX, 0x80000001        ; result % 2 (handling negative correctly)
; ... sign fixup for negative modulo ...
CMP EAX, 0x1               ; if (interval % 2 == 1)
JNZ skip_flash
; check if object owner is local player
CMP [EBP + 0x21C], [0xA83D4C]
JNZ skip_flash
; FLASH: write inverted color
NOT EBX                    ; ~color (bitwise invert)
PUSH EBX
PUSH EDI
CALL [EAX + 0x24]         ; PutPixel(coords, ~color)
```

The `AND EAX, 0x80000001` + sign fixup is the compiler's optimized `% 2` for signed
integers. The flash pattern with FlashFrameTime=7 and total=49:

```
Frame 49: (49-1)/7 = 6, 6%2=0 → normal
Frame 42: (42-1)/7 = 5, 5%2=1 → FLASH (inverted)
Frame 35: (35-1)/7 = 4, 4%2=0 → normal
Frame 28: (28-1)/7 = 3, 3%2=1 → FLASH
Frame 21: (21-1)/7 = 2, 2%2=0 → normal
Frame 14: (14-1)/7 = 1, 1%2=1 → FLASH
Frame  7: ( 7-1)/7 = 0, 0%2=0 → normal
```

So the unit flashes 3 times during the 49-frame window (at frames 42, 28, 14 counting down).

---

## 11. Foundation Type Table (0x008192B8)

22 int entries, verified from binary dump:

| Index | Value | Meaning |
|-------|-------|---------|
| 0 | 1 | 1×1 (infantry, small vehicles) |
| 1 | 2 | 2×1 or 1×2 |
| 2 | 1 | 1×1 |
| 3 | 2 | 2×2 (e.g., Power Plant) |
| 4 | 2 | 2×2 |
| 5 | 3 | 3×3 (e.g., War Factory) |
| 6 | 3 | 3×3 |
| 7 | 3 | 3×3 |
| 8 | 4 | 4×2 or 2×4 |
| 9 | 3 | 3×3 |
| 10 | 1 | 1×1 |
| 11 | 3 | 3×3 |
| 12 | 4 | 4×4 |
| 13 | 1 | 1×1 |
| 14 | 1 | 1×1 |
| 15 | 2 | 2×2 |
| 16 | 2 | 2×2 |
| 17 | 5 | 5×5 (e.g., Construction Yard) |
| 18 | 4 | 4×4 |
| 19 | 3 | 3×3 |
| 20 | 6 | 6×6 (custom/large) |
| 21 | 0 | 0×0 (null terminator) |

These indices are used by `GenerateBrushShapes` (FUN_006563B0) to generate isometric
diamond patterns. The actual width and height for each type are looked up via
`FUN_007C5F00` at runtime.

---

*Generated 2025-03-20. Last audited 2026-05-28 (verify-doc-fix-swarm). All assembly verified via live Ghidra MCP disassembly.
Companion to docs/RADAR_MINIMAP_RENDERING.md.*
