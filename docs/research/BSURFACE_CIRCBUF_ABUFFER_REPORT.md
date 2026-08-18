# BSurface / CircBuf / ABuffer Structure Report

Reverse-engineered via Ghidra MCP (live decompilation of `gamemd.exe`).
Confidence: HIGH -- all offsets verified from constructor code and multiple usage sites.

---

## Overview

The game uses a **two-tier surface system** for the ABuffer (alpha/shroud overlay) and
ZBuffer (depth). The outer tier is a **circular buffer wrapper** (CircBuf, 0x30 bytes)
that provides viewport scrolling without copying data. The inner tier is a **BSurface**
(0x20 bytes) which owns the actual pixel memory.

Both `g_ABuffer` (0x0087e8a4) and `g_ZBuffer` (0x00887644) share the same structure --
they're constructed by nearly identical functions.

---

## CircBuf -- Outer Wrapper (0x30 bytes)

This is the object pointed to by `g_ABuffer` and `g_ZBuffer`. Size: 0x30 (48) bytes.
Allocated via `operator_new(0x30)`.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x00 | 4 | `origin_x` | Screen X origin (viewport left edge) |
| 0x04 | 4 | `origin_y` | Screen Y origin (viewport top edge) |
| 0x08 | 4 | `width` | Buffer width in pixels |
| 0x0C | 4 | `height` | Buffer height in pixels |
| 0x10 | 4 | `circ_offset` | Circular buffer byte offset from `buffer_base`. Tracks how much the viewport has scrolled. Updated by the scroll function (0x00410ed0). |
| 0x14 | 4 | `inner_surface` | Pointer to inner BSurface object (0x20 bytes, owns pixel data) |
| 0x18 | 4 | `buffer_base` | Pointer to start of pixel data. Set by calling `inner_surface->Lock(0,0)` during construction. |
| 0x1C | 4 | `buffer_upper` | Pointer to end of pixel data (`buffer_base + width * height * 2`). Used for wrap-around check: `if (ptr >= buffer_upper) ptr -= buffer_size`. |
| 0x20 | 4 | `buffer_size` | Total buffer size in bytes (`width * height * 2`). Subtracted when wrapping. |
| 0x24 | 4 | `default_value` | Default Z/alpha. ABuffer: `0x8000` (unused for alpha, set for compat). ZBuffer: `0x8000` (base depth). Updated by scroll function. |
| 0x28 | 4 | `stride` | Width in pixels (same as 0x08). Used as `stride * 2` for bytes per scanline. |
| 0x2C | 4 | `num_rows` | Height in pixels (same as 0x0C). Used by fill/scroll functions. |

### Constructor Addresses

- **ABuffer constructor:** `BSurface__Constructor` at `0x00410ce0`
- **ZBuffer constructor:** `ZBuffer_constructor` at `0x007bc970`
- Both are `__thiscall(this, x, y, width, height)`

### Constructor Logic (0x00410ce0 / 0x007bc970)

```c
CircBuf* CircBuf_Init(CircBuf* this, int x, int y, int width, int height) {
    this->origin_x = x;           // +0x00
    this->origin_y = y;           // +0x04
    this->width = width;          // +0x08
    this->height = height;        // +0x0C
    this->buffer_size = height * width * 2;  // +0x20, 16bpp
    this->stride = width;         // +0x28
    this->num_rows = height;      // +0x2C

    // Create inner BSurface
    BSurface* inner = new BSurface(width, height);  // operator_new(0x20)
    this->inner_surface = inner;  // +0x14

    // Fill with initial value
    inner->FillRect(full_rect, fill_value);
    //   ABuffer fills with 0x7F (neutral alpha)
    //   ZBuffer fills with 0xFFFF (max depth)

    // Lock the surface and get buffer pointer
    int ptr = inner->GetScanlinePtr(0, 0);  // vtable[0x5c]
    this->buffer_base = ptr;      // +0x18 (cached from inner->Lock call)
    inner->Unlock();              // vtable[0x60]

    this->circ_offset = 0;        // +0x10
    this->default_value = 0x8000; // +0x24
    this->buffer_upper = ptr + height * width * 2;  // +0x1C

    return this;
}
```

---

## BSurface / XSurface -- Inner Surface (0x20 bytes)

The inner surface is a simple memory-backed pixel surface. It inherits from XSurface.
Size: 0x20 (32) bytes. Allocated via `operator_new(0x20)`.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x00 | 4 | `vtable` | Points to `vtable__BSurface` at `0x007e2070` |
| 0x04 | 4 | `width` | Surface width in pixels |
| 0x08 | 4 | `height` | Surface height in pixels |
| 0x0C | 4 | `lock_count` | Reference count for Lock/Unlock calls |
| 0x10 | 4 | `bytes_per_pixel` | Always 2 (16-bit surfaces) |
| 0x14 | 12 | `pixel_buffer` | Inline PixelBuffer struct (see below) |

### PixelBuffer (inline at BSurface+0x14, 12 bytes)

| Offset from BSurface | Size | Field | Description |
|----------------------|------|-------|-------------|
| 0x14 | 4 | `data_ptr` | Pointer to allocated pixel memory |
| 0x18 | 4 | `data_size` | Size of allocation in bytes (`width * height * 2`) |
| 0x1C | 1 | `owns_data` | 1 = PixelBuffer allocated the memory (must free), 0 = external |

### Construction Sequence

```c
BSurface* BSurface_Init(int width, int height) {
    BSurface* s = operator_new(0x20);
    s->vtable = &vtable__XSurface;   // initially XSurface
    s->width = width;                 // +0x04
    s->height = height;               // +0x08
    s->lock_count = 0;                // +0x0C
    s->bytes_per_pixel = 2;           // +0x10

    // PixelBuffer_Init at +0x14: allocates width*height*2 bytes
    PixelBuffer_Init(&s->pixel_buffer, NULL, width * height * 2);
    //   sets data_ptr = operator_new(size), data_size = size, owns_data = 1

    s->vtable = &vtable__BSurface;    // promote to BSurface
    return s;
}
```

### Key Virtual Methods (BSurface vtable at 0x007e2070)

| Vtable Offset | Function | Address | Description |
|---------------|----------|---------|-------------|
| 0x14 | FillRect | 0x007bb050 | Fill a rectangle with a 16-bit value (corrected 2026-05-29: was 0x007bb020; vtable[0x14] reads as 0x007bb050 via read_memory at 0x007e2070+0x14; 0x007bb020 is a small helper called by 0x007bb050 via vtable+0x78 — RTTI_LABEL_DRIFT) |
| 0x5c | Lock / GetScanlinePtr | 0x004115f0 | `lock_count++; return data_ptr + bpp*x + pitch*y` |
| 0x60 | Unlock | 0x00411570 | `lock_count--; return 1` |
| 0x70 | GetBytesPerPixel | 0x00411630 | Returns `this->bytes_per_pixel` (=2) |
| 0x74 | GetPitch | 0x00411640 | Returns `GetWidth() * bytes_per_pixel` (=`width * 2`) |
| 0x7c | GetWidth | 0x00411540 | Returns `this->width` |

The Lock function (vtable 0x5c) computes the scanline pointer as:
```c
int BSurface_Lock(BSurface* this, int x, int y) {
    this->lock_count++;
    int bpp = this->GetBytesPerPixel();  // 2
    int pitch = this->GetPitch();        // width * 2
    return this->pixel_buffer.data_ptr + bpp * x + pitch * y;
}
```

---

## CircBuf_GetScanlinePtr (0x004114b0)

The core function that maps (x, y) screen coordinates to a pointer in the circular buffer.

```c
uint CircBuf_GetScanlinePtr(CircBuf* this, int x, int y) {
    // Step 1: Get raw pointer from inner surface
    int raw_ptr = this->inner_surface->Lock(x, y);   // vtable[0x5c]
    this->inner_surface->Unlock();                     // vtable[0x60]

    // Step 2: Apply circular offset
    uint ptr = this->circ_offset + raw_ptr;  // +0x10

    // Step 3: Wrap around if past buffer end
    if (ptr >= this->buffer_upper) {         // +0x1C
        ptr -= this->buffer_size;            // +0x20
    }

    return ptr;
}
```

**How it works:**
- The inner BSurface's Lock(x,y) returns `data_ptr + 2*x + (width*2)*y`, a linear
  offset into the pixel array.
- `circ_offset` (field 0x10) is a byte offset that shifts the "start" of the logical
  buffer within the physical array. When the viewport scrolls down by N rows,
  `circ_offset` increases by `N * stride * 2`, and the vacated rows at the top are
  filled with the default value. This avoids copying the entire buffer on scroll.
- The wrap check ensures the pointer stays within `[buffer_base, buffer_upper)`.

### Caller Convention

Callers typically compute `y` relative to the CircBuf's origin:
```c
// In ShroudEdge_BlitToABuffer:
scanline = CircBuf_GetScanlinePtr(g_ABuffer, x, y - g_RadarViewportOffsetY);

// In Standard_SHP_blitter:
scanline = CircBuf_GetScanlinePtr(g_ABuffer, x, screen_y - g_ABuffer->origin_y);

// In AlphaShapeClass__DrawAll_NoMask:
scanline = CircBuf_GetScanlinePtr(g_ABuffer,
    x - g_RadarViewportOffsetX,
    y - g_RadarViewportOffsetY);
```

The y coordinate passed is always **relative to the viewport top**, not absolute screen Y.

---

## CircBuf Scroll Function (0x00410ed0)

Called when the viewport moves. Handles circular buffer advancement and clearing
of newly exposed scanlines.

```c
void CircBuf_Scroll(CircBuf* this, int dx, int dy, uint16_t fill_value) {
    int stride = this->stride;  // +0x28
    int current_col = (this->circ_offset / 2) % stride;

    // Handle horizontal scroll
    if (dx != 0) {
        this->circ_offset += dx * 2;
        // Normalize circ_offset to stay within buffer
        uint new_base = this->circ_offset + this->buffer_base;
        if (new_base < this->buffer_base)       // underflow
            new_base += this->buffer_size;
        if (new_base >= this->buffer_upper)     // overflow
            new_base -= this->buffer_size;
        this->circ_offset = new_base - this->buffer_base;

        // Fill newly exposed columns with fill_value
        // (handles wrap-around correctly)
        inner_surface->FillRect(exposed_rect, fill_value);
    }

    // Handle vertical scroll
    if (dy != 0) {
        this->default_value -= dy;  // +0x24
        this->circ_offset += stride * dy * 2;
        // Normalize and wrap...
        // Fill newly exposed rows with fill_value
    }
}
```

If the scroll exceeds the buffer dimensions, the entire buffer is re-filled.

---

## ABuffer Initialization

### In WinMain (0x006bb9a0)

The ABuffer is created once during game startup:

```asm
; At 0x006bdea5:
push    0x30                    ; allocate 48 bytes
call    operator_new
test    eax, eax
jz      set_null

mov     edx, 0x1e0              ; 480 pixels
mov     ecx, [0x00886fa4]       ; viewport Y offset
sub     edx, ecx                ; height = 480 - viewport_y
mov     esi, [0x00886fa0]       ; viewport X offset
mov     edi, 0x1e0              ; width = 480

; Push params on stack: {x, y, width, height}
mov     ecx, eax                ; this = allocated memory
call    BSurface__Constructor   ; 0x00410ce0
mov     [g_ABuffer], eax        ; store result
```

Parameters: `BSurface__Constructor(this, viewport_x, viewport_y, 480, 480 - viewport_y)`

The width is always **480 (0x1E0)** and height is **480 minus the sidebar viewport Y offset**.

### In Set_View_Dimensions (0x004a8960)

The ABuffer is **destroyed and recreated** whenever the viewport dimensions change
(e.g., resolution change, sidebar resize):

```c
void Set_View_Dimensions(RECT* dims) {
    g_RadarViewportOffsetX = dims->left;
    g_RadarViewportOffsetY = dims->top;
    g_RadarViewportWidth = dims->right;     // width, not right edge
    g_RadarViewportHeight = dims->bottom;   // height, not bottom edge

    // Delete old ZBuffer
    if (g_ZBuffer) { ZBuffer_Destroy(g_ZBuffer); free(g_ZBuffer); g_ZBuffer = NULL; }
    // Create new ZBuffer
    g_ZBuffer = new ZBuffer(viewport_x, viewport_y, viewport_width, viewport_height);
    g_ZBuffer->default_value = 0x8000;

    // Delete old ABuffer
    if (g_ABuffer) { ABuffer_Destroy(g_ABuffer); free(g_ABuffer); g_ABuffer = NULL; }
    // Create new ABuffer
    g_ABuffer = new CircBuf(viewport_x, viewport_y, viewport_width, viewport_height);
    // Filled with 0x7F by constructor
}
```

### Destruction

`FUN_00410e50` -- deletes the inner BSurface via its destructor (vtable[0]):
```c
void CircBuf_Destroy(CircBuf* this) {
    if (this->inner_surface != NULL) {
        this->inner_surface->destructor(1);  // scalar deleting destructor
    }
    this->inner_surface = NULL;
}
```
Then the caller frees the outer CircBuf with `operator_delete`.

---

## g_ShroudEnabled (0x00b73550) -- ACTUALLY g_hWnd

**IMPORTANT CORRECTION:** The label `g_ShroudEnabled` at `0x00b73550` is **misleading**.
This global stores the **main game window HWND**, not a shroud enable flag.

Evidence:
- Written by `CreateWindowExA` in `FUN_00777c30` (window creation function)
- Used as `HWND` parameter in `GetClientRect`, `SetWindowPos`, `ShowWindow`, etc.
- In tactical code, `if (g_ShroudEnabled != 0)` simply checks "does the window exist"
- The function at `0x00777080` receives it as `HWND param_2` and calls
  `GetClientRect(param_2, ...)` on it

**Actual shroud/fog control flags:**

| Flag | Location | Description |
|------|----------|-------------|
| FogOfWar enabled | `*DAT_00a8b230 & 0x1000` | Session special flags bit 12 |
| Shroud (explored) | `CellClass+0x12C` bit 3 (mask 0x08) | Per-cell: 0=shrouded, 1=explored |
| Fog (currently visible) | `CellClass+0x140` bit 1 (mask 0x02) | Per-cell: 0=fogged, 1=visible |
| Shroud setting (MP dialog) | `SessionClass+0x14AE` | bool, "Shroud" INI key |
| FogOfWar setting (MP dialog) | `SessionClass+0x14B7` | bool, "FogOfWar" INI key |

---

## CellChangeNotify (0x005865f0)

Called when a cell's visibility changes. Notifies nearby cells and triggers
radar/render updates.

```c
void CellChangeNotify(CellClass* cell, int notify_type, bool mark_radar) {
    // Extract cell coordinates from cell+0x24 (MapCoord)
    uint coords = *(uint*)(cell + 0x24);
    short cell_x = (short)(coords & 0xFFFF);
    short cell_y = (short)(coords >> 16);

    int step = 1;
    // Iterate over a diagonal strip of 7 cells (corrected 2026-05-29: was described as "2x2 grid (this cell + 3 neighbors)"; binary loop runs while step < 15, step starts at 1 and increments by 2, so 7 iterations — each step increments both cell_x and cell_y, tracing a diagonal NW→SE — OPERATOR_OR_ORDER_DRIFT)
    do {
        int index = cell_y * 0x200 + cell_x;
        CellClass* neighbor = g_CellArray[index];

        // Check if neighbor's height level is within range
        if (step - 2 <= neighbor->height_level && neighbor->height_level <= step) {
            if (mark_radar) {
                RadarClass__MarkObjectDirty(&neighbor->map_coord);
            }
            // Find nearest object in the cell and notify it
            TechnoClass* obj = CellClass__Find_Nearest_Object(cell, 0, 0);
            if (obj != NULL) {
                obj->vfunc_0x198(notify_type);  // visibility change callback
            }
        }

        cell_x++;
        step += 2;
        cell_y++;
    } while (step < 15);  // iterates 7 times: steps 1,3,5,7,9,11,13
}
```

This walks a diagonal strip of 7 cells (both cell_x and cell_y increment each iteration) and notifies objects within matching height ranges. The loop runs 7 iterations (step = 1,3,5,7,9,11,13, while step < 15), covering a NW→SE diagonal from the source cell. (corrected 2026-05-29: doc previously described this as a "2x2 grid (this cell + 3 neighbors)"; binary shows 7-cell diagonal — OPERATOR_OR_ORDER_DRIFT via decompile_function 0x005865f0)

---

## Vtable Addresses

| Vtable | Address | Class |
|--------|---------|-------|
| `vtable__BSurface` | `0x007e2070` | BSurface (16 entries) |
| `vtable__XSurface` | `0x007e2104` | XSurface (26 entries, BSurface overrides first 16) |
| `vtable__Surface` | (base class) | Abstract surface interface |

---

## Summary: ABuffer Pixel Format

Each pixel in the ABuffer is 16 bits (uint16), but only the low byte is meaningful
for alpha/intensity:

| Value | Meaning |
|-------|---------|
| 0x7F (127) | Neutral -- no darkening, fully lit |
| 0x00 | Full black (solid shroud) |
| 0x01-0x7E | Partial darkening (shroud/fog edge gradient) |
| 0xFE (254) | Transparent marker in SHP source data (skip, don't write) |

The value is used as an index into the 64KB alpha blend table at `0x0088a118`:
```c
output = blend_table[abuffer_value + shp_pixel * 256];
// Equivalent to: (shp_pixel * abuffer_value) / 127
```

When the ABuffer is 0x7F (neutral), the blend is identity. Lower values darken
the output proportionally.
