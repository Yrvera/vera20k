# Shroud/Fog Rendering Pipeline — Ghidra Research Report

## ⚠ CRITICAL: Fog-of-War Is DISABLED in Standard YR

**Yuri's Revenge uses SHROUD ONLY.** The `[MultiplayerDialogSettings] FogOfWar` defaults to
`false`. In standard YR gameplay:

- **Shroud (active):** Unexplored cells are fully black. Once a cell is explored by any
  unit's sight range, it becomes permanently visible — it NEVER darkens again (unless a
  gap generator re-shrouds it).
- **Fog-of-war (dormant):** The semi-transparent darkening of "previously seen but not
  currently visible" cells is **inherited from Tiberian Sun** and exists in the binary but
  is **gated behind `SpecialFlags & 0x1000`** which is OFF by default. Almost no YR players
  ever enable it.

**For our engine:** Do NOT render the fog-of-war layer (green channel / "revealed but not
visible" dimming) in standard mode. Only render shroud (black for unexplored). The fog
layer should only activate when the session FogOfWar flag is explicitly enabled.

**Our current code is wrong:** `fog_mask.rs` renders both shroud AND fog as if both are
always active. `FogState` tracks `FLAG_VISIBLE` separately from `FLAG_REVEALED`, but in
standard YR only `FLAG_REVEALED` matters — once set, the cell is fully clear forever.

### What the two layers mean

| Layer | YR Default | Condition | Visual |
|-------|-----------|-----------|--------|
| **Shroud** | ON | Cell never explored (`cell+0x12C bit 3 = 0`) | Opaque black |
| **Fog-of-war** | OFF | Cell explored but no friendly unit in range | Semi-transparent darkening (TS legacy) |

### Code sections gated behind FogOfWar flag

All fog-specific code in gamemd.exe checks `(*DAT_00a8b230 & 0x1000) != 0` before executing:
- `FogEdge_BlendToABuffer` — never called in standard YR
- FOG.SHP loading/usage — SHROUD.SHP is used instead
- AlphaShapeClass fog ghost rendering
- `MapClass::UpdateFogOfWarCell` / fog edge propagation
- FogRate timer / spy satellite fog cycle
- Fogged object creation (`FUN_00486a70`)

---

## Overview

The shroud/fog system in gamemd.exe uses a **two-layer approach**: shroud (unexplored areas = fully black) and fog-of-war (previously seen but not currently visible = darkened). Both layers are rendered via the **ABuffer** — a 16-bit per-pixel alpha/intensity overlay surface that runs parallel to the ZBuffer and the primary screen surface. **In standard YR only the shroud layer is active.**

**Key insight:** Shroud edges are rendered as **SHP sprites** loaded from `SHROUD.SHP` (or `FOG.SHP` when FogOfWar is enabled), NOT generated procedurally. The SHP frame index for each cell is determined by a **neighbor bitmask lookup table** at `0x007f4194`.

## Architecture Summary

```
Cell visibility flags (per CellClass)
    │
    ├── cell+0x12C bit 3 = "explored" (shroud cleared)
    │   IsShrouded checks: (cell+300 & 0x8) == 0 means shrouded
    │
    └── cell+0x140 bit 1 = "currently revealed" (fog cleared)
    │   Fog checks: (cell+0x140 & 0x2) == 0 means fogged
    │
    ▼
Shroud_EdgeBitmask_Calculator (0x006d8700)
    │   Checks 8 neighbors, builds 8-bit adjacency mask
    │   Looks up SHP frame index in table at 0x007f4194
    │
    ▼
Shroud_fog_edge_rendering (0x004801F0)
    │   Calls ShroudEdge_BlitToABuffer for shroud edges (writes to ABuffer)
    │   Calls FogEdge_BlendToABuffer for fog edges (writes to ABuffer, blended)
    │
    ▼
ABuffer (g_ABuffer) — 16-bit per-pixel overlay
    │   Read by all blitters (SHP, TMP, VXL) during normal rendering
    │   Values: 0x7F = neutral/clear, 0x00 = full black, 0xFE = transparent
    │
    ▼
DAT_0088a118 — 64KB blending table
    Precomputed: output = (alpha_hi * alpha_lo) / 0x7F
    Applied per-pixel during SHP/TMP blitting
```

---

## Complete Edge Frame Lookup Table (Verified)

**Address:** `0x007f4194`, 256 bytes. Indexed by the 8-bit neighbor adjacency bitmask.

The 8-bit bitmask uses this neighbor→bit assignment:

```
Neighbor layout and bit assignments:
  NW  N  NE         bit 6  bit 7  bit 0
   W  *   E    →    bit 5    *    bit 1
  SW  S  SE         bit 4  bit 3  bit 2

A set bit means that neighbor IS CLEAR (explored/visible).
```

**Full table (256 values):**

```
Index: Frame   (hex bytes from binary)
  0: 0xFF (no edge — all neighbors same state)
  1: 33    2: 2     3: 2     4: 34    5: 37    6: 2     7: 2
  8: 4     9: 26   10: 6    11: 6    12: 4    13: 26   14: 6    15: 6
 16: 35   17: 45   18: 17   19: 17   20: 38   21: 41   22: 17   23: 17
 24: 4    25: 26   26: 6    27: 6    28: 4    29: 26   30: 6    31: 6
 32: 8    33: 21   34: 10   35: 10   36: 27   37: 31   38: 10   39: 10
 40: 12   41: 23   42: 14   43: 14   44: 12   45: 23   46: 14   47: 14
 48: 8    49: 21   50: 10   51: 10   52: 27   53: 31   54: 10   55: 10
 56: 12   57: 23   58: 14   59: 14   60: 12   61: 23   62: 14   63: 14
 64: 32   65: 36   66: 25   67: 25   68: 44   69: 40   70: 25   71: 25
 72: 19   73: 30   74: 20   75: 20   76: 19   77: 30   78: 20   79: 20
 80: 39   81: 43   82: 29   83: 29   84: 42   85: 46   86: 29   87: 29
 88: 19   89: 30   90: 20   91: 20   92: 19   93: 30   94: 20   95: 20
 96: 8    97: 21   98: 10   99: 10  100: 27  101: 31  102: 10  103: 10
104: 12  105: 23  106: 14  107: 14  108: 12  109: 23  110: 14  111: 14
112: 8   113: 21  114: 10  115: 10  116: 27  117: 31  118: 10  119: 10
120: 12  121: 23  122: 14  123: 14  124: 12  125: 23  126: 14  127: 14
128: 1   129: 1   130: 3   131: 3   132: 16  133: 16  134: 3   135: 3
136: 5   137: 5   138: 7   139: 7   140: 5   141: 5   142: 7   143: 7
144: 24  145: 24  146: 18  147: 18  148: 28  149: 28  150: 18  151: 18
152: 5   153: 5   154: 7   155: 7   156: 5   157: 5   158: 7   159: 7
160: 9   161: 9   162: 11  163: 11  164: 22  165: 22  166: 11  167: 11
168: 13  169: 13  170: 0xFE 171: 0xFE 172: 13  173: 13  174: 0xFE 175: 0xFE
176: 9   177: 9   178: 11  179: 11  180: 22  181: 22  182: 11  183: 11
184: 13  185: 13  186: 0xFE 187: 0xFE 188: 13  189: 13  190: 0xFE 191: 0xFE
192: 1   193: 1   194: 3   195: 3   196: 16  197: 16  198: 3   199: 3
200: 5   201: 5   202: 7   203: 7   204: 5   205: 5   206: 7   207: 7
208: 24  209: 24  210: 18  211: 18  212: 28  213: 28  214: 18  215: 18
216: 5   217: 5   218: 7   219: 7   220: 5   221: 5   222: 7   223: 7
224: 9   225: 9   226: 11  227: 11  228: 22  229: 22  230: 11  231: 11
232: 13  233: 13  234: 0xFE 235: 0xFE 236: 13  237: 13  238: 0xFE 239: 0xFE
240: 9   241: 9   242: 11  243: 11  244: 22  245: 22  246: 11  247: 11
248: 13  249: 13  250: 0xFE 251: 0xFE 252: 13  253: 13  254: 0xFE 255: 0xFE
```

**Special values:**
- `0xFF` (255) at index 0 = no neighbors differ → no edge needed
- `0xFE` (254) = fully surrounded by opposite state → solid fill (frame 15)
- `0-46` = specific edge transition SHP frame index

**Key observation:** Corner bits (0, 2, 4, 6 = NE, SE, SW, NW) are often redundant when the adjacent cardinal bits (1, 3, 5, 7 = E, S, W, N) are set. Many pairs of adjacent entries differ only in a corner bit and produce the same frame index. This means the 47 unique frames map cleanly to combinations of the 4 cardinal directions plus corner refinements.

---

## Detailed Function Analysis

### 1. Shroud_EdgeBitmask_Calculator (0x006d8700)

**Parameters:** `(CellXY* cell_xy, int mode)` — mode 0 = shroud, mode 1 = fog
**Returns:** SHP frame index (0-46), or -1 (no edge needed), or -2 (fully surrounded = solid fill)

**CRITICAL: Uses 8 neighbors, not 4.** This function checks ALL 8 neighbors (N, NE, E, SE, S, SW, W, NW) to build an 8-bit bitmask, which gives 47 distinct edge shapes (not just 4 cardinal fades).

**Shroud mode (param_2 == 0):**
1. Test `cell+300 & 0x18` (bits 3-4): if both clear → fully interior shroud → return -2
2. Test `cell+300 & 0x8` (bit 3): if SET → cell is explored → edge cell, continue
3. If bit 3 is CLEAR → cell is shrouded → no edge from this cell → return pre-computed `iVar5` (-1 or -2)
4. For each of 8 neighbors: check `neighbor.cell+300 & 0x8`; if clear (shrouded) → set corresponding bit
5. Look up `table[bitmask]` at `0x007f4194`; return signed byte result

**Fog mode (param_2 == 1):**
1. Test `cell+0x140 & 0x3` (bits 0-1): if both clear → fully interior fog → return -2
2. Test `cell+0x140 & 0x2` (bit 1): if SET → cell is currently visible → edge cell, continue
3. If bit 1 CLEAR → cell is fogged → return pre-computed result
4. For each of 8 neighbors: check `neighbor.cell+0x140 & 0x2`; if clear → set bit
5. Look up `table[bitmask]`; return result

**Neighbor check order in code (all modes):**
```
(-1,-1) NW → bit 6 (0x40)
( 0,-1) N  → bit 7 (0x80)
(+1,-1) NE → bit 0 (0x01)
(-1, 0) W  → bit 5 (0x20)
(+1, 0) E  → bit 1 (0x02)
(-1,+1) SW → bit 4 (0x10)
( 0,+1) S  → bit 3 (0x08)
(+1,+1) SE → bit 2 (0x04)
```

### 2. Shroud_fog_edge_rendering (0x004801F0) — Confirmed

```c
void Shroud_fog_edge_rendering(this, screen_pos, clip_rect) {
    // Step 1: Compute shroud edge
    shroud_frame = Shroud_EdgeBitmask_Calculator(this+0x24, 0);
    *(char*)(this+0x120) = shroud_frame;    // cache in CellClass

    if (shroud_frame == -2) shroud_frame = 0x0F;  // fully shrouded → frame 15
    if (shroud_frame == -1) shroud_frame = 0;      // no edge → frame 0

    ShroudEdge_BlitToABuffer(screen_pos, clip_rect, shroud_frame);

    // Step 2: Compute fog edge (only if FogOfWar enabled AND player alive)
    fog_frame = Shroud_EdgeBitmask_Calculator(this+0x24, 1);
    *(char*)(this+0x121) = fog_frame;

    if ((*DAT_00a8b230 & 0x1000) != 0       // FogOfWar enabled
        && *(char*)(g_PlayerPtr + 0x1F5) == 0) {  // Player NOT defeated
        if (fog_frame == -2) fog_frame = 0x0F;
        if (fog_frame == -1) fog_frame = 0;
        FogEdge_BlendToABuffer(screen_pos, clip_rect, fog_frame);
    }
}
```

**HouseClass+0x1F5 = `IsDefeated`** (verified, high confidence). When a player is defeated via `HouseClass::MPlayer_Defeated()`, this flag is set to 1. The effect: defeated players see through fog (fog edges stop rendering). This is NOT a spectator mode flag — it specifically indicates the player has been eliminated from the game.

### 3. ShroudEdge_BlitToABuffer (0x0047efe0)

**Lazy-loads two SHP files on first call:**
- `DAT_0089e794` = `SHROUD.SHP` — used when FogOfWar **disabled** (bit 12 clear)
- `DAT_0089e790` = `FOG.SHP` — used when FogOfWar **enabled** (bit 12 set)

**SHP selection:**
```c
if (*DAT_00a8b230 & 0x1000)    // FogOfWar enabled?
    shp = DAT_0089e790;         // FOG.SHP (softer edges for shroud when fog also active)
else
    shp = DAT_0089e794;         // SHROUD.SHP (hard edges for shroud-only mode)
```

**Rendering algorithm:**
1. Get SHP frame rect for the given frame index
2. Clip against the provided clip rectangle
3. Get ABuffer scanline pointer via `CircBuf_GetScanlinePtr`
4. For each pixel in the SHP frame:
   - If pixel != 0xFE (transparent index): `*abuffer = (uint16_t)pixel_value`
   - Otherwise: skip (leave ABuffer unchanged)

**This is a direct pixel write** — the SHP pixel value becomes the 16-bit ABuffer alpha value. The ABuffer is 16-bit per pixel but values are in range 0x00-0x7F:
- SHP pixel 0x00 = full darkening (solid black in final render)
- SHP pixel 0x7F = neutral (no change)
- SHP pixel 0xFE = transparent (skip, don't write to ABuffer)

The ABuffer uses a **circular buffer** (`CircBuf`) for scanline management, likely to handle screen clipping efficiently without needing the entire surface in contiguous memory.

### 4. FogEdge_BlendToABuffer (0x0047f250)

**Lazy-loads:** `DAT_0089e760` = `FOG.SHP` (separate load from the shroud blitter's copy)

Unlike shroud which does direct writes, fog does **blended writes**:

```c
for each pixel:
    shp_value = *src++;
    if (shp_value < 0x80) {           // only process values 0..127
        if (*abuffer == 0x7F) {       // ABuffer is neutral?
            *abuffer = shp_value;     // direct write (fast path)
        } else {
            blended = (*abuffer - 0x7F) + shp_value;
            *abuffer = (blended < 1) ? 0 : blended;  // clamp to 0
        }
    }
    // values >= 0x80 are transparent/ignored
```

**Blending semantics:** The formula `(ABuffer - 0x7F) + shp_value` is an additive offset from the 0x7F neutral midpoint. When ABuffer is already darkened (< 0x7F from a shroud edge), adding a fog edge darkens it further. The clamp to 0 prevents underflow past full black.

### 5. ABuffer Clearing (0x00411330)

Fills ABuffer scanlines with the value `0x007F` (16-bit). Each 16-bit pixel in the ABuffer is set to 0x7F (neutral midpoint). The function handles the circular buffer wrapping correctly and writes the value as packed 32-bit words (`0x007F007F`) for performance, with 16-bit boundary handling.

### 6. TMP Tile Blitter — ABuffer Integration (0x00547cf0)

The tile blitter reads the ABuffer per-pixel and uses it as an **index into a brightness/remap lookup table**:

```c
// Per-pixel compositing formula (decoded from binary):
for each pixel in tile:
    lighting_index = lighting_lookup_table[*abuffer_ptr * 2];    // 16-bit lookup
    combined = (lighting_index | tile_palette_index);
    screen_pixel = remap_table[combined * 2];   // 16-bit output
```

Where:
- `lighting_lookup_table` = table at `DAT_00aa10d0`, stride 0x200 per intensity level
  - Setup: `DAT_00aa10d0 = intensity * 0x200 + FUN_00420140(brightness_table)`
  - `intensity` is derived from a per-tile brightness parameter, clamped to 0-254
- `remap_table` = `DAT_00aa10c0`, from `*(int*)(param_2 + 0x170)` (theater color table)

**Implication:** In the original engine, fog/shroud is not a post-process overlay. It is applied **during** tile rendering as a palette remap that darkens and potentially color-shifts each pixel based on the ABuffer value. This produces a more integrated look where fog darkening respects the original palette's color relationships rather than applying a uniform tint.

### 7. Standard SHP Blitter — ABuffer Integration (0x004373b0)

The SHP blitter also reads ABuffer per-pixel:
1. Gets ABuffer scanline pointer alongside screen surface pointer
2. Advances ABuffer pointer in lockstep with screen surface pointer
3. The ABuffer value modulates the pixel output through the same remap mechanism

**Both TMP and SHP blitters also read the ZBuffer** for depth testing. The ABuffer is always read regardless of Z-test results.

### 8. Alpha Blend Table (0x0088a118) — Confirmed

64KB table, initialized once by `AlphaShapeClass::Constructor` or `FUN_00420e90`:

```c
// Initialization (verified from binary):
for (i = 0; i < 0x10000; i++) {
    alpha_lo = i & 0xFF;             // low byte = current ABuffer value
    alpha_hi = (i >> 8);             // high byte = SHP pixel value
    result = (alpha_hi * alpha_lo) / 0x7F;
    table[i] = clamp(result, 0, 255);
}
```

**Usage:** In fogged-object alpha compositing:
```c
*abuffer = (uint16_t) table[(*abuffer & 0xFF) + shp_pixel * 256];
```

This is **multiplicative blending**: `output = (source * existing) / 127`. When ABuffer is neutral (0x7F), output equals source. When already darkened, it gets darker.

### 9. FoggedObject Alpha Compositing (0x00420f40)

**Purpose:** Draws "remembered" object silhouettes for objects that moved into fog.

For each entry in the alpha shapes array (`DAT_0088a0f4`, count in `DAT_0088a100`):

1. Skip if `entry+0x3C != 0` (entry disabled)
2. Calculate screen position: `(entry+0x28 - tactical_scroll_x) + viewport_x` and `(entry+0x2C - tactical_scroll_y) + viewport_y`
3. Get the SHP shape data
4. Clip against viewport
5. Use the **isometric diamond mask** at `DAT_007e2b20` (60×30 bytes)
6. For each pixel where mask != 0x20 (space):
   ```c
   *abuffer = (uint16_t) alpha_blend_table[(*abuffer & 0xFF) + shp_pixel * 256];
   ```

The mask advances by `0x3C - clip_width` per row to stay aligned with the 60-byte-wide diamond.

### 10. Isometric Diamond Mask (0x007e2b20)

**Size:** 60 × 30 bytes (1800 bytes total)
**Values:** `0x20` (space, 32) = skip pixel, `0xDB` (219, ████) = apply alpha

The mask forms a perfect isometric diamond centered on a 60×30 cell:
```
Row  0: [60 × space]                              (0 filled)
Row  1: [28 spaces][4 × 0xDB][28 spaces]          (4 filled)
Row  2: [26 spaces][8 × 0xDB][26 spaces]          (8 filled)
Row  3: [24 spaces][12 × 0xDB][24 spaces]         (12 filled)
...each row adds 4 filled pixels (2 per side)...
Row 14: [2 spaces][56 × 0xDB][2 spaces]           (56 filled)
Row 15: [60 × 0xDB]                               (60 filled, widest)
Row 16: [2 spaces][56 × 0xDB][2 spaces]           (56 filled)
...each row removes 4 filled pixels...
Row 28: [26 spaces][8 × 0xDB][26 spaces]          (8 filled)
Row 29: [28 spaces][4 × 0xDB][28 spaces]          (4 filled)
```

This exactly matches a 60-wide × 30-tall isometric cell diamond, which is the standard RA2 cell shape.

---

## Cell Visibility Fields (CellClass)

| Offset | Size | Field Name | Purpose |
|--------|------|------------|---------|
| `+0x5C` | int | `last_dirty_frame` | Frame counter when cell was last added to shroud dirty list |
| `+0x10C` | short | `tile_z` | Tile height/Z for TMP_TileBlitter Z-value computation |
| `+0x11B` | char | `Level` | Terrain height level (0-14). Used by RevealByHeight LOS check and gap generators. NOT gap opacity — it's the cell's actual terrain height. |
| `+0x120` | char | `shroud_edge_frame` | Cached shroud edge bitmask result (frame index or -1/-2) |
| `+0x121` | char | `fog_edge_frame` | Cached fog edge bitmask result (frame index or -1/-2) |
| `+0x12C` (300) | uint flags | `shroud_flags` | Bit 3 (0x08) = explored, Bit 4 (0x10) = fully interior |
| `+0x130` | int | `fog_vis_counter` | Fog visibility reference counter. >0 = currently visible for rendering |
| `+0x134` | int | `fog_vis_max` | Maximum fog visibility counter cap |
| `+0x138` | char | `needs_fog_redraw` | Set to 1 when edge frame changed; cleared when added to dirty list |
| `+0x13C` | int | `vis_counter` | Visibility counter used by `IsFogged()` game logic check |
| `+0x140` | uint flags | `fog_render_flags` | Multiple bits for fog rendering state (see below) |

### Cell+0x140 Fog Render Flags

| Bit | Mask | Meaning |
|-----|------|---------|
| 0 | 0x01 | Fog fully surrounded (all neighbors fogged → solid fog fill) |
| 1 | 0x02 | Currently revealed (fog cleared for this cell) |
| 5 | 0x20 | Set when fog counter transitions from 0→positive. Cleared when counter drops back to 0 while interior. |
| 6 | 0x40 | Cleared during fog border updates. Purpose: mark cell as processed. |
| 22 | 0x400000 | Cleared during fog border update (see `FUN_00486bf0`) |

### Cell+0x12C Shroud Flags

| Bit | Mask | Meaning |
|-----|------|---------|
| 3 | 0x08 | "Explored" — cell has been seen at least once. Once set, never cleared. |
| 4 | 0x10 | "Fully interior" — all neighbors also explored. Used as optimization to skip edge calculations. |

### Visibility Counter Management

**FUN_00487630 (decrement fog counter):**
```c
void decrement_fog_counter(CellClass* cell) {
    if (cell->fog_vis_counter == 1) cell->fog_vis_counter = 0;
    cell->fog_vis_counter--;

    if (!(shroud_flags & 0x18)) {    // not fully explored?
        if (counter < 1) shroud_flags |= 0x18;   // mark as explored
    } else if (counter < 1 && (fog_flags & 0x20)) {
        fog_flags &= ~0x20;   // clear fog-visible-transition flag
    }
}
```

**FUN_00487690 (increment fog counter):**
```c
void increment_fog_counter(CellClass* cell) {
    if (cell->fog_vis_counter == -1) cell->fog_vis_counter = 0;
    cell->fog_vis_counter++;
    if (cell->fog_vis_counter > cell->fog_vis_max)
        cell->fog_vis_counter = cell->fog_vis_max;   // cap
    if (prev_value < 1 && cell->fog_vis_counter > 0)
        fog_flags |= 0x20;   // transition: invisible → visible
}
```

These are called from `MapClass::RevealFogCell` when cells enter/leave unit vision ranges.

### CellClass::RevealShroudFlags (0x004876f0)

```c
void CellClass::RevealShroudFlags(CellClass* cell) {
    cell->shroud_flags |= 0x18;    // set explored + interior
    if (cell->fog_vis_counter > 0) {
        cell->fog_render_flags |= 0x20;   // mark fog-visible
    }
}
```

---

## Cell Edge Update Propagation

### RevealCell (0x004aa050) — Shroud Edge Update

When a cell becomes explored:
1. Set `cell+300 |= 0x08` (explored flag)
2. Recompute shroud edge bitmask; if changed, update cache at `cell+0x120`
3. If bitmask result is -1 (no edge needed), set `cell+300 |= 0x10` (interior flag)
4. Add cell to dirty list via `FUN_006da7d0`
5. **For each of 8 neighbors:** recompute THEIR edge bitmask
   - If neighbor bitmask result is -1: recursively call `RevealCell` on that neighbor
   - If neighbor bitmask changed: update cache, add to dirty list
   - If neighbor bitmask is -2 (fully surrounded) or already explored: just update cache

This recursive propagation is why exploring one cell can cause a cascade of edge updates across multiple neighbors.

### MapClass::UpdateFogOfWarCell (0x004a9dd0) — Fog Edge Update

Same pattern as RevealCell but for fog:
1. Set `cell+0x140 |= 0x02` and clear bit 6 (`& ~0x40`)
2. Recompute fog edge bitmask; if changed, update cache at `cell+0x121`
3. If bitmask result is -1, set `cell+0x140 |= 0x01` (fog interior)
4. Add cell to dirty list
5. **For each of 8 neighbors:** recompute their fog bitmask, propagate
6. If cell was newly revealed AND FogOfWar is enabled, call `FUN_00486bf0` (fogged object management)

### MapClass::RevealFogCell (0x004a9ca0) — Combined Update

Called when a cell transitions between visible/fogged state:
1. Check if cell was previously revealed: `(cell+0x140 & 2) == 0` OR `(cell+300 & 8) == 0`
2. Set `cell+0x140 = (cell+0x140 & ~0x40) | 0x02` (set revealed, clear processed)
3. Call fog counter increment or decrement (FUN_00487690 or FUN_00487630)
4. Recompute BOTH shroud and fog edge bitmasks; update caches
5. If bitmask results differ from cached values, add to dirty list
6. Call CellChangeNotify for render invalidation
7. If cell was newly revealed with FogOfWar enabled, call FUN_00486bf0

---

## Dirty List Management

### TacticalClass Shroud Edge Dirty List

- **`TacticalClass+0xE0`** = count of cells in dirty list (max 799)
- **`TacticalClass+0xE4`** = array of CellClass pointers (800 entries)

**FUN_006da7d0 — Add Cell to Dirty List:**
```c
void AddCellToDirtyList(TacticalClass* tac, CellClass* cell) {
    if (cell->last_dirty_frame == g_CurrentFrameCounter) return;  // already dirty this frame
    if (!(cell->shroud_flags & 0x08) && !cell->needs_fog_redraw) return;  // nothing to draw

    cell->last_dirty_frame = g_CurrentFrameCounter;
    // Convert cell to screen coords, check viewport bounds
    if (screen_pos within viewport ± margins) {
        if (tac->dirty_count < 799) {
            tac->dirty_list[tac->dirty_count++] = cell;
        }
        cell->needs_fog_redraw = 0;
        tac->field_0xd7d = 1;   // flag that dirty list has entries
    }
}
```

### Tactical_layer_shroud_edges (0x006d3660)

The main shroud edge rendering function, called during TacticalClass::Draw Phase 1:

```
Phase 1: Process the shroud edge dirty list
    For each cell in dirty list (0..TacticalClass+0xE0):
        - Get cell center coords → convert to screen coords
        - Offset by (-30, -15) for isometric diamond alignment
        - Call Shroud_fog_edge_rendering (ABuffer writes)
        - Call FoggedObject alpha compositing (0x00420f40)

Phase 2: Process additional region-based updates
    Call FUN_006d71e0 for the radar viewport rect
    Call FUN_006d71e0 for each dirty rect region

Phase 3: Process g_DirtyRectList entries
    For each dirty rect:
        - If marked for redraw: clear ABuffer to 0x7F (FUN_00411330)
        - Then call FUN_006d71e0 (full shroud edge pass over region)
```

### FUN_006d71e0 — Full Region Shroud Edge Pass

For a given screen rectangle:
1. Convert screen rect back to cell coordinates
2. Iterate all cells within the rectangle in isometric row-major order
3. For each cell: convert to screen, clip, call `Shroud_fog_edge_rendering`
4. At the end: call `FUN_00421350` (AlphaShape compositing for all registered fogged objects in the region)

---

## SHP File Details

### SHROUD.SHP
- **Loaded from:** MIX archives, string at 0x0081ce18
- **Used when:** FogOfWar is **disabled** (basic shroud-only mode)
- **Frame count:** 47 frames (indices 0-46, plus special frame 15 = full coverage)
- **Pixel semantics:** Each pixel is an ABuffer alpha value:
  - 0x00 = full black (maximum darkening)
  - 0x7F = neutral (no change)
  - 0xFE = transparent (don't write to ABuffer)
- **Stored at:** `DAT_0089e794`

### FOG.SHP
- **Loaded from:** MIX archives, string at 0x0081ce10
- **Used when:** FogOfWar is **enabled** — replaces SHROUD.SHP for shroud edges, AND used for fog edges
- **Stored at:** `DAT_0089e790` (for shroud-edge use) and `DAT_0089e760` (for fog-edge use)
- **Pixel semantics for shroud edges:** Same as SHROUD.SHP (direct ABuffer write)
- **Pixel semantics for fog edges:** Values < 0x80 are blended; values >= 0x80 are transparent

---

## Gap Generator System

### BuildingClass::UpdateGapGenerator_Tick (0x00454db0)

Gap generators manage a per-building opacity animation that gradually reveals/conceals cells:

**State machine:**
- **State 1 (deploying):** Increment opacity from 0 to 15 in single steps
  - At opacity 1, 6, 11: mark building for redraw
  - At opacity 15: transition to state 2 (fully deployed)
  - For each of 21 cells in the gap coverage area: set `cell+0x178 = opacity`
- **State 2 (active):** Gap fully deployed, cells concealed
- **State 3 (undeploying):** Decrement opacity from 15 to 0
  - At opacity 0, 5, 10: mark for redraw
  - At opacity 0: transition to state 0 (inactive)

The opacity value (0-16) is stored at `cell+0x11B` (also referenced as `cell+0x178` through a different path). This value is used during rendering to modulate the fog intensity — a cell with opacity 16 is fully concealed even if it would otherwise be visible.

**Coverage area:** 21 cells (`param_1 + 0x157`, array of 21 cell pointers)

### TechnoClass::UpdateGapVisual (0x0070e920)

Manages the visual "shimmer" effect of gap generator cloaking:
- 9-state animation sequence with frame-count-based timers
- States cycle through: deploy → transition → idle → wobble → idle → check → redeploy
- Random variation: wobble state has ±5 frame random duration around 20 frames
- Each state transition updates the visual effect frame

---

## Sight Range Lookup Table

**Address:** `0x007ed3d0` — cumulative cell count per sight range radius

| Sight Range | Total Cells | Offset Table (0x007ed3c4) |
|-------------|-------------|---------------------------|
| 0 | 1 | — |
| 1 | 9 | 1 |
| 2 | 21 | 9 |
| 3 | 37 | 21 |
| 4 | 61 | 37 |
| 5 | 89 | 61 |
| 6 | 121 | 89 |
| 7 | 161 | 121 |
| 8 | 205 | 161 |
| 9 | 253 | 205 |
| 10 | 309 (0x135) | 253 |
| 11 | 369 (0x171) | 309 |

The offset table at `0x007ed3c4` (shifted by 4 bytes = 1 int) stores the starting offset for each ring. For sight range R, cells at indices `[offset_table[R] .. total_table[R])` form the outermost ring, while `[0 .. offset_table[R])` are interior cells.

A separate spiral pattern table at `0x00abd490` stores (dx, dy) offsets in reveal order. `MapClass::RevealShroud` iterates this table to reveal cells around a unit's position.

**Max sight range:** Clamped to 10 in `MapClass::RevealShroud` (`if param_3 > 10: param_3 = 10`). This matches the RA2 engine's hard cap. The table has entries up to index 11 for fog border calculations (+3 cells beyond sight range for smooth edge fading).

---

## Session Flags

| Bit | Mask | Flag Name | Address |
|-----|------|-----------|---------|
| 5 | 0x0020 | Inert | `*DAT_00a8b230` |
| 6 | 0x0040 | TiberiumGrows | `*DAT_00a8b230` |
| 7 | 0x0080 | TiberiumSpreads | `*DAT_00a8b230` |
| 8 | 0x0100 | MCVDeploy | `*DAT_00a8b230` |
| 9 | 0x0200 | InitialVeteran | `*DAT_00a8b230` |
| 10 | 0x0400 | FixedAlliance | `*DAT_00a8b230` |
| 11 | 0x0800 | HarvesterImmune | `*DAT_00a8b230` |
| **12** | **0x1000** | **FogOfWar** | `*DAT_00a8b230` |
| 14 | 0x4000 | TiberiumExplosive | `*DAT_00a8b230` |
| 15 | 0x8000 | DestroyableBridges | `*DAT_00a8b230` |

---

## Global Variables Summary

| Address | Name | Type | Purpose |
|---------|------|------|---------|
| `0x00a8b230` | SpecialFlags ptr | `uint*` | Session special flags bitfield |
| `g_ABuffer` | ABuffer | `BSurface*` | 16-bit per-pixel alpha overlay surface |
| `g_ZBuffer` | ZBuffer | `ZBufferSurface*` | 16-bit per-pixel depth surface |
| `0x0089e794` | shroud_shp | `SHP*` | Loaded SHROUD.SHP data |
| `0x0089e790` | fog_as_shroud_shp | `SHP*` | Loaded FOG.SHP (used as shroud when FogOfWar on) |
| `0x0089e760` | fog_edge_shp | `SHP*` | Loaded FOG.SHP (used for fog edges) |
| `0x0089e7c5` | shroud_loaded | `bool` | Lazy init flag for SHROUD.SHP/FOG.SHP |
| `0x0089e7c6` | fog_loaded | `bool` | Lazy init flag for FOG.SHP (fog edge) |
| `0x007f4194` | edge_frame_lut | `byte[256]` | Neighbor bitmask → SHP frame index lookup |
| `0x0088a118` | alpha_blend_table | `byte[65536]` | Multiplicative blend: `(hi * lo) / 0x7F` |
| `0x007e2b20` | fogged_obj_mask | `byte[60×30]` | Isometric diamond mask (0x20=skip, 0xDB=draw) |
| `0x0088a0f4` | alpha_shapes_buf | `AlphaShapeClass**` | Array of registered alpha shapes |
| `0x0088a100` | alpha_shapes_count | `int` | Number of active alpha shapes |
| `0x007e4eb8` | transparent_index | `byte` | SHP transparent pixel value (0xFE) |
| `0x00886fa0` | viewport_x | `int` | Viewport left edge |
| `0x00886fa4` | viewport_y | `int` | Viewport top edge |
| `0x00b73550` | g_hWnd | `HWND` | Main window handle (used as "is initialized" check) |
| `g_PlayerPtr` | local player | `HouseClass*` | Local player house pointer |
| `0x007ed3d0` | sight_cell_counts | `int[12]` | Cumulative cells per sight radius |
| `0x00abd490` | reveal_spiral | `short[]` | (dx,dy) pairs for cell reveal order |

---

## Rendering Pipeline Order

Within TacticalClass::Draw Phase 1:

```
Step 1: ZBuffer dirty rect clear (0x006d2b60)
    └── Clears Z-values in regions needing redraw
    └── For shroud cells: blits shroud tile to screen+ZBuffer via TMP_TileBlitter

Step 2: Shroud edges and icons (0x006d3660)  ← Tactical_layer_shroud_edges
    ├── For each cell in shroud edge dirty list (max 799):
    │   ├── Get center coords → screen coords, offset by (-30, -15)
    │   ├── Call Shroud_fog_edge_rendering:
    │   │   ├── Compute shroud bitmask (8 neighbors) → SHP frame → BlitToABuffer
    │   │   └── If FogOfWar && !IsDefeated: compute fog bitmask → BlendToABuffer
    │   └── Call FoggedObject alpha compositing (0x00420f40)
    │
    ├── Process radar viewport region via full shroud edge pass
    ├── Process additional dirty rect regions
    │
    └── For each dirty rect in g_DirtyRectList:
        ├── Clear ABuffer region to 0x7F (FUN_00411330)
        └── Full shroud edge pass over region (FUN_006d71e0)

Step 3+: Terrain shadows, base cells, smudges, overlays, etc.
    └── ALL blitters READ ABuffer per-pixel and apply darkening via remap tables

Step N: Object rendering
    └── SHP/VXL blitters read ABuffer per-pixel during compositing
```

---

## Z-Buffer Interaction

**Shroud rendering does NOT write to the Z-buffer.**

The shroud edge blitters (`ShroudEdge_BlitToABuffer` and `FogEdge_BlendToABuffer`) write exclusively to `g_ABuffer`. They never reference `g_ZBuffer`.

However, `Tactical_ZBufferDirtyClear` (0x006d2d91) does clear Z-buffer regions for dirty rects that overlap shroud edges, and calls `FUN_00480180` which uses `TMP_TileBlitter` with the shroud tile to write both screen pixels and Z-values for fully-shrouded areas.

---

## Three-Tier Visibility System (CRITICAL)

The original engine has **three independent visibility tracking systems** that interact:

### Tier 1: Per-House Visibility Bitmask (cell+0x78)

```c
// CellClass__IsVisibleToHouse (0x004870b0):
bool IsVisibleToHouse(CellClass* cell, byte house_index) {
    return (*(uint*)(cell + 0x78) & (1 << (house_index & 0x1F))) != 0;
}
```

- **Type:** 32-bit bitmask (supports up to 32 houses)
- **Purpose:** Game logic visibility — used for cloaking/uncloaking decisions, gap generator checks, unit AI
- **Not used for fog rendering** — purely for gameplay mechanics
- **Per-house gap counter array** at cell+0x7C (documented in CLOAKING_VISUAL_PIPELINE.md)

### Tier 2: Friendly Vision Counter (cell+0x13C)

```c
// IsFogged (0x005864A0):
bool IsFogged(CoordStruct* pos) {
    CellClass* cell = GetCellFromCoords(pos);  // with height adjustment
    return *(int*)(cell + 0x13C) < 1;
}
```

- **Type:** int (reference counter)
- **Purpose:** Tracks how many **friendly** units (player + allies) can see this cell
- **Incremented by:** `TechnoClass::UpdateCloakShroud` for player-controlled/allied units
- **Decremented by:** `TechnoClass::RemoveCloakShroud` for player-controlled/allied units
- **Also incremented when** player is defeated (IsDefeated=1) — defeated players see everything
- **Used by:** `IsFogged()` for game logic (targeting, etc.)

### Tier 3: Gap Concealment Counter (cell+0x130 / cell+0x134)

```c
// UpdateCloakShroud — for ENEMY units (not player-controlled, not allied):
cell->gap_vis_counter++;           // cell+0x130
cell->gap_vis_max++;               // cell+0x134
if (cell->gap_vis_counter > 0) {
    cell->shroud_flags &= ~0x10;   // clear interior bit
    cell->shroud_flags &= ~0x08;   // clear explored bit → RE-SHROUD THE CELL
}
```

- **cell+0x130:** Gap concealment reference counter. When > 0, the cell is hidden by enemy gap generators.
- **cell+0x134:** Cap/tracking for gap concealment count.
- **Purpose:** Enemy gap generators UNDO exploration — they make previously-seen cells shrouded again.
- **On removal:** When gap counter reaches 0, `cell+300 |= 0x18` (re-explored + interior).

### How the three tiers interact

```
Unit enters cell's sight range:
  ├── Is it a player/allied unit?
  │   YES → cell+0x13C++ (friendly vision counter)
  │          Also: RevealShroud (set explored, update fog edges)
  │
  └── Is it an enemy gap generator?
      YES → cell+0x130++ (gap counter)
             cell+0x134++
             If gap counter > 0: CLEAR explored bit (re-shroud)

Rendering checks:
  ├── Shroud: cell+0x12C bit 3 (explored flag)
  ├── Fog edges: cell+0x140 bit 1 (currently revealed)
  └── Fog counter: cell+0x130 (gap concealment) managed separately from cell+0x13C (friendly vision)

Game logic checks:
  ├── IsShrouded: cell+300 & 0x08
  ├── IsFogged: cell+0x13C < 1
  └── IsVisibleToHouse: cell+0x78 & (1 << house_idx)
```

### IsShrouded (0x00586360) — Full Decompilation

Converts 3D world coordinates to cell coordinates, adjusting for terrain height:
```c
uint z_level = coords->z / HEIGHT_STEP;
if (z_level & 1) {  // odd z level: offset by 1
    cell_x = (coords->x >> 8) - (z_level / 2 + 1);
    cell_y = (coords->y >> 8) - (z_level / 2 + 1);
} else {             // even z level
    cell_x = (coords->x >> 8) - (z_level / 2);
    cell_y = (coords->y >> 8) - (z_level / 2);
}
CellClass* cell = GetCell(cell_x, cell_y);
// For odd z: checks cell first, then checks diagonal neighbor
return (cell->shroud_flags & 0x08) ? 0 : 1;  // bit 3 clear = shrouded
```

**Height adjustment:** Cells at higher Z levels have their coordinates shifted diagonally to account for the isometric projection. This prevents fog from "sliding" on cliffs.

### IsFogged (0x005864A0) — Same Pattern

```c
// Same height adjustment as IsShrouded
CellClass* cell = GetCellFromCoords(pos);
return (*(int*)(cell + 0x13C) < 1) ? 0 : 1;  // counter < 1 = NOT fogged (visible)
```

**Note:** IsFogged returns 1 when the cell IS visible (counter > 0), 0 when fogged. The name is misleading — it returns "is visible", not "is fogged".

---

## Shroud Initialization and Reset

### MapClass::ResetShroud (0x00577ab0) — Initial State

Called at map start to set all cells to shrouded:
```c
for each cell:
    cell->shroud_flags &= ~0x18;     // clear explored + interior
    cell->gap_vis_counter = 1;        // set to 1 (NOT 0!)
    cell->gap_vis_max = 0;            // gap cap = 0
    cell->fog_render_flags &= ~0x03;  // clear fog interior + revealed
```

**Important:** Gap counter starts at 1, not 0. This means the first gap counter decrement will bring it to 0, which triggers the transition logic in FUN_00487630.

### MapClass::ClearShroud (0x00577d90) — Reveal Entire Map

Called by spy satellite or cheat:
```c
for each cell (skipping map border cells):
    cell->gap_vis_counter = 0;        // clear gap concealment
    cell->gap_vis_max = 0;            // clear gap cap
    cell->shroud_flags |= 0x18;       // set explored + interior
    cell->fog_render_flags |= 0x03;   // set fog interior + revealed
```

### Shroud Regrow (FUN_004acac0)

Called periodically when `ShroudGrow=true` and `ShroudRate > 0` (timer in LogicClass::AI):

```c
// Phase 1: Mark cells for re-concealment
for each cell (512×512):
    if (explored && !interior):      // edge cells only
        cell->fog_render_flags |= 0x20;  // mark for reconcealment

// Phase 2: Actually re-conceal
for each cell:
    if (fog_render_flags & 0x20):
        fog_render_flags &= ~0x20;
        ReconcealCell(cell);          // FUN_004acda0
```

### ReconcealCell (FUN_004acda0) — Re-Shroud a Cell

```c
void ReconcealCell(CellXY* cell_xy) {
    CellClass* cell = GetCell(cell_xy);
    if (cell->shroud_flags & 0x08) {          // only if currently explored
        cell->shroud_flags &= ~0x18;          // clear explored + interior
        AddToDirtyList(cell);
        for each of 8 neighbors:
            neighbor->shroud_flags &= ~0x10;  // clear interior flag
            AddToDirtyList(neighbor);
    }
}
```

### Re-Fog a Cell (FUN_004acc50) — Recursive Fog Reconcealment

```c
void RefogCell(CellXY* cell_xy) {
    CellClass* cell = GetCell(cell_xy);
    cell->fog_edge_frame = 0xFE;              // fully fogged
    cell->fog_render_flags &= ~0x03;          // clear interior + revealed
    if (cell->shroud_flags & 0x08)            // if explored
        AddToDirtyList(cell);

    for each of 8 neighbors:
        new_frame = Shroud_EdgeBitmask_Calculator(neighbor, 1);  // fog mode
        if (new_frame == -2 && neighbor->fog_edge_frame != -2):
            RefogCell(neighbor);               // recursive!
        else if changed:
            neighbor->fog_edge_frame = new_frame;
            neighbor->fog_render_flags = (flags & ~0x01) | 0x02;
            AddToDirtyList(neighbor);

    if (original_flags & 0x03):                // was visible
        CreateFoggedObjects(cell);             // FUN_00486a70
}
```

---

## Shroud Reveal System (MapClass::RevealShroud)

### Reveal Spiral Table (DAT_00abd490)

The spiral table is populated at startup by `MapClass__InitRevealSpiralTable` (0x00561910)
with **hardcoded literal constants** — not generated algorithmically. It contains ~370 entries
of `(short dx, short dy)` pairs sorted approximately by distance from center.

A companion mirror table at `DAT_00abcf60` (populated by `MapClass__InitRevealMirrorTable`
at 0x005638d0) stores midpoint cells for line-of-sight checks.

The cumulative ring size table at `0x007ed3d0`:
```
Sight 0:  1 cell     Sight 5:  89 cells    Sight 10: 309 cells
Sight 1:  9 cells    Sight 6: 121 cells
Sight 2: 21 cells    Sight 7: 161 cells
Sight 3: 37 cells    Sight 8: 205 cells
Sight 4: 61 cells    Sight 9: 253 cells
```

### RevealShroud Flow (0x005673a0)

```c
void MapClass::RevealShroud(MapClass* this, CoordStruct* pos, int sight_range,
                            HouseClass* house, ...) {
    // 1. Height-adjust coordinates (shift cell coords based on Z level)
    int z_level = pos->z / HEIGHT_STEP;
    cell_x += AdjustForZ(z_level);
    cell_y += AdjustForZ(z_level);

    // 2. Clamp sight range to 10 (hard cap)
    if (sight_range > 10) sight_range = 10;

    // 3. Map bounds check (skip if center cell is outside playable area)
    if (cell_x + cell_y <= map_size || ...) return;

    // 4. RevealByHeight optimization: skip inner rings if height bonus active
    if (!rules->RevealByHeight && has_height_bonus && sight_range > 2) {
        start_offset = ring_start_table[sight_range];
        // Only process outermost ring, inner cells already revealed
    }

    // 5. Alliance check: only reveal for local player or allies
    if (house != g_PlayerPtr) {
        if (house->AlliedWith(g_PlayerPtr) && rules->AllyReveal)
            house = g_PlayerPtr;  // treat as player's own reveal
        else
            return;  // don't reveal for non-allied houses
    }

    // 6. Iterate spiral table
    for each (dx, dy) in spiral[start..count]:
        target_cell = (center_x + dx, center_y + dy);

        // Bounds check
        if (target_cell outside map) continue;

        // Circular distance check via Sqrt_Approx
        float dist = Sqrt_Approx((dx*dx) + (dy*dy));
        if (dist > sight_range) continue;

        // RevealByHeight line-of-sight check
        if (rules->RevealByHeight) {
            mirror = mirror_table[index];  // midpoint cell
            if (viewer_z + 3 < mirror_cell.Level)
                continue;  // LOS blocked by terrain
        }

        // Actually reveal the cell
        CellClass::RevealShroudFlags(target_cell);
}
```

### Height-Based Line of Sight (RevealByHeight)

When `rules+0x17EE` (RevealByHeight) is true (default), each reveal candidate is
checked against a **midpoint cell** from the mirror table:

```
Viewer ──────── Midpoint ──────── Target
  (z)            (Level)           (reveal?)

If midpoint.Level > viewer_z + 3 → LOS blocked, skip reveal
```

The `+3` tolerance means terrain must be **4+ levels above** the viewer at the midpoint
to block line of sight. `cell+0x11B` (CellClass::Level) is the terrain height level (0-14),
not gap opacity as previously documented.

### Sqrt_Approx (0x004cac40)

Uses a precomputed 8192-entry lookup table at `DAT_008650bc` to approximate square root.
Decomposes the input float into mantissa and exponent, halves the exponent, and uses the
mantissa's top 13 bits as a table index. This is faster than the FPU `fsqrt` instruction
on Pentium II-era hardware.

### How Buildings Reveal Shroud

Buildings reveal from their **single center point** with their Sight range — the foundation
size does NOT affect the reveal pattern. A 3x3 building with Sight=8 reveals identically
to a 1x1 with Sight=8 at the same position.

### TechnoClass Reveal Virtuals

| vtable offset | Name | Address | Purpose |
|---------------|------|---------|---------|
| +0x488 | `UpdateReveal` | 0x0070af50 | Compute effective sight (including veteran bonus), call RevealShroud |
| +0x48C | `ReReveal` | 0x0070b1d0 | Re-reveal using stored sight range (used by paranoid update) |

Both are inherited by all TechnoClass subclasses (not overridden).

### Paranoid Shroud Update (FUN_004adee0 / FUN_004adcd0)

Called during map initialization and shroud resets. Iterates ALL TechnoClass objects:
- For player-controlled units: calls vtable+0x48C (ReReveal) with full parameters
- For allied buildings (when AllyReveal=true): calls vtable+0x48C
- For enemy gap generators: calls vtable+0x418 (UpdateGapGenerator)

---

## Z-Buffer Clear for Shroud Cells (FUN_00480180)

**Critical correction:** `DAT_0087f69c` is NOT tile data. It is a
`DynamicVector<LightConvertClass*>` — the global lighting/color remap table array.
`*DAT_0087f69c` yields the first `LightConvertClass` (neutral lighting).

FUN_00480180 is called from `Tactical_ZBufferDirtyClear` for each cell in the shroud
dirty list. It calls `TMP_TileBlitter` with:

```c
TMP_TileBlitter(
    *DAT_0087f69c,    // LightConvertClass (lighting table, NOT tile data)
    0,                 // frame index 0
    g_PrimarySurface,  // screen surface
    screen_x, screen_y,
    clip_x, clip_y, clip_w, clip_h,
    cell->Level,       // cell+0x11B = terrain height
    cell->tile_z,      // cell+0x10C = tile Z for ZBuffer
    1,                 // param_13 = ZBuffer ENABLED
    0,                 // param_14 = frame sub-index 0
    0,                 // param_15 = NOT shroud-fill mode
    1,                 // param_16 = ZBuffer-ONLY clear mode
    0,                 // param_17 = NOT dithered fog
    0                  // param_18 = no fog tint
);
```

**What this does:** Writes `0xFFFF` (maximum depth) to the ZBuffer for each pixel in
the isometric diamond shape, WITHOUT writing to the screen surface. This resets the
Z-buffer in shrouded areas so that sprites behind the shroud are properly depth-occluded.
The actual black fill comes from the ABuffer (written by SHROUD.SHP frame 15 during
shroud edge rendering).

### TMP_TileBlitter Flag Parameters

| Param | Name | 0 | 1 |
|-------|------|---|---|
| param_13 | ZBuffer | No Z-test/write | Z-test and Z-write enabled |
| param_15 | Shroud fill | Normal | Fill ZBuffer with 0x0000 + screen with fog tint (solid black) |
| param_16 | ZBuffer-only | Normal | Write ZBuffer only, skip screen surface entirely |
| param_17 | Dithered fog | Normal | Checkerboard fog pattern (BlendedFog=false) |

### SHROUD.SHP Frame Semantics (Clarification)

| Bitmask Calculator Return | Frame Used | Behavior |
|---------------------------|------------|----------|
| **-2** (fully unexplored) | Frame 15 | **Active blackout** — writes 0x00 to ABuffer across entire 60×30 diamond |
| **-1** (shrouded, all neighbors same) | Frame 0 | **Passive** — frame is empty (0×0), leaves ABuffer unchanged |
| **0–46** (edge transition) | Corresponding frame | Writes gradient values (0x00–0x7F) defining the edge shape |

Frame 0 being empty is correct behavior: interior shrouded cells rely on frame 15
having already written 0x00 to their ABuffer area. The ABuffer is cleared to 0x7F
(neutral) at the start of each frame, then frame 15 writes black, then frame 0
leaves it black.

---

## FoggedObject / AlphaShapeClass Lifecycle (FogOfWar only — dormant in standard YR)

### Creation (ObjectClass::Unlimbo → 0x00420960)

AlphaShapes are created when an object (building, unit) is placed/unlimboed AND:
1. The object has an AlphaImage SHP (`ObjectTypeClass+0xAC`)
2. The player has previously discovered this object

**AlphaShapeClass structure (0x40 bytes / 64 bytes):**

| Offset | Size | Field | Purpose |
|--------|------|-------|---------|
| +0x00 | 4×4 | vtables | 4 vtable pointers (AlphaShapeClass, 3 secondary) |
| +0x10-0x20 | | | AbstractClass base fields |
| +0x24 | 4 | source_object | Pointer to the ObjectClass that this shape represents |
| +0x28 | 4 | world_x | Absolute world X coordinate |
| +0x2C | 4 | world_y | Absolute world Y coordinate |
| +0x30 | 4 | shp_width | SHP frame width |
| +0x34 | 4 | shp_height | SHP frame height |
| +0x38 | 4 | alpha_image | Pointer to the AlphaImage SHP data |
| +0x3C | 1 | disabled | Set to 1 when shape should be removed |

### Two Rendering Paths

1. **DrawAll_WithMask (0x00420f40):** Used during per-cell shroud edge rendering. Uses the 60×30 isometric diamond mask at `0x007e2b20`. Only draws within the cell's diamond footprint.

2. **DrawAll_NoMask (0x00421350):** Used during dirty-rect region repainting (FUN_006d71e0). Renders without the diamond mask, covering the full SHP extent.

Both use the alpha blend table: `*abuffer = table[(*abuffer & 0xFF) + shp_pixel * 256]`

### Disabling and Cleanup

- **Notification (vtable slot 10):** When source object changes state (re-revealed, destroyed), sets `+0x3C = 1` (disabled)
- **Cleanup (FUN_00420e90):** Called once per game tick; iterates backward through array, destroys any shape with `disabled != 0`
- **Destruction (0x00421730):** Removes from both tracking arrays, calls destructor

### Fog Object Creation at Cell Level (FUN_00486a70)

When a cell becomes re-fogged and FogOfWar is enabled:
```c
void CreateFoggedObjectsForCell(CellClass* cell) {
    if (!(*SpecialFlags & 0x1000)) return;  // FogOfWar must be enabled

    // Iterate diagonal strip (gap_opacity range check)
    for each cell in strip:
        if (cell+0x140 bit 22 not set):     // not already processed
            cell->fog_render_flags |= 0x400000;  // mark processed
            // For each object at cell:
            //   If building (type 6) and gap active → create fogged shape
            //   If infantry/vehicle/aircraft → mark for redraw
```

---

## Per-Pixel ABuffer→Screen Compositing (SHROUD-ONLY PATH)

This is how shroud darkening actually reaches the screen in standard YR (FogOfWar=false).

### The Per-Pixel Formula

Every tile and sprite blitter reads the ABuffer and applies this per-pixel compositing:

```c
// For each pixel in the isometric diamond:
uint16_t abuffer_val = *abuffer_ptr;                      // 0x00=black, 0x7F=neutral
uint16_t brightness  = brightness_table[abuffer_val * 2]; // → brightness level << 8
uint8_t  palette_idx = *tile_pixel_ptr;                    // palette index 0-255
uint16_t combined    = brightness | palette_idx;           // high byte=brightness, low byte=palette
uint16_t screen_rgb  = color_table[combined * 2];          // final RGB565 output
*screen_ptr = screen_rgb;
```

### What each ABuffer value produces

| ABuffer Value | Brightness | Visual Result |
|---------------|------------|---------------|
| `0x00` | 0 (darkest remap) | **Pitch black** — fully shrouded |
| `0x01`-`0x3E` | Low | Dark edge gradient (near shroud) |
| `0x3F` | ~50% | Half-brightness (mid-gradient) |
| `0x40`-`0x7E` | High | Light edge gradient (near visible) |
| `0x7F` | Maximum (neutral) | **Normal brightness** — fully visible |

SHROUD.SHP frames encode these gradient values as raw pixel bytes. The edge
transition frames have pixels ranging from 0x00 (black at the shroud center) to
0x7F (neutral at the visible edge), creating smooth per-pixel gradients within
each 60×30 isometric diamond.

### The Color/Remap Table (`ConvertClass+0x170`)

The final lookup table (`DAT_00aa10c0`) is a `LightConvertClass` remap table with
`max_brightness × 256` entries of RGB565 colors. Entry `[brightness * 256 + palette_index]`
gives the final screen color for that palette color at that brightness level. The first
`LightConvertClass` in the global array (`*DAT_0087f69c`) provides neutral lighting (no
tint). `DAT_0087f69c` is a `DynamicVector<LightConvertClass*>`, NOT tile data.

---

## Brightness Lookup Table (FUN_00420140)

The ABuffer modulates pixel brightness through a 128KB lookup table:

```c
short* CreateBrightnessTable(int max_brightness) {
    short* table = new short[0x10000 + 4];  // 128KB + header
    table[0x10000] = max_brightness;         // stored at end
    table[0x10002] = 0;                      // reference count

    for (uint i = 0; i < 0x10000; i++) {
        int lo = i & 0xFF;
        int hi = i >> 8;
        int result = (lo * hi * (max_brightness - 1)) / 0x7E02;
        result = min(result, max_brightness - 1);
        table[i] = (byte)result << 8;   // store in high byte of 16-bit entry
    }
    return table;
}
```

**Usage in TMP_TileBlitter:**
```c
// DAT_00aa10d0 = intensity * 0x200 + brightness_table
// For each pixel:
lighting_value = brightness_table[abuffer_value * 2];  // 16-bit, high byte = brightness
combined_index = lighting_value | tile_palette_index;   // OR with 8-bit palette index
screen_pixel = remap_table[combined_index * 2];         // final 16-bit color
```

The table is cached in a pool (`DAT_0088a084`, capacity tracked by `DAT_0088a090`). When the same `max_brightness` is requested again, the cached table is returned (reference counted).

---

## BSurface / CircBuf / ABuffer Architecture

### CircBuf Structure (0x30 bytes)

The ABuffer and ZBuffer both use a CircBuf wrapper that enables efficient viewport scrolling:

```c
struct CircBuf {
    int origin_x;        // +0x00: viewport origin X
    int origin_y;        // +0x04: viewport origin Y
    int width;           // +0x08: surface width
    int height;          // +0x0C: surface height
    int circ_offset;     // +0x10: circular scroll offset (bytes)
    BSurface* inner;     // +0x14: underlying pixel surface
    byte* buffer_base;   // +0x18: cached data pointer
    byte* buffer_upper;  // +0x1C: wrap boundary (base + size)
    int buffer_size;     // +0x20: total buffer size (for wrapping)
    uint16_t default_val;// +0x24: default fill value (0x8000 for ZBuffer)
    int stride;          // +0x28: bytes per scanline (= width * 2 for 16bpp)
    int num_rows;        // +0x2C: number of rows
};
```

### CircBuf_GetScanlinePtr (0x004114b0)

```c
uint16_t* GetScanlinePtr(CircBuf* buf, int x, int y) {
    // Get linear pointer from inner BSurface
    uint16_t* ptr = BSurface_Lock(buf->inner, x, y);
    // Add circular offset
    ptr += buf->circ_offset;
    // Wrap if past boundary
    if (ptr >= buf->buffer_upper)
        ptr -= buf->buffer_size;
    return ptr;
}
```

### BSurface Inner Object (0x20 bytes)

```c
struct BSurface {
    void* vtable;        // +0x00
    int width;           // +0x04
    int height;          // +0x08
    int lock_count;      // +0x0C
    int bpp;             // +0x10: bytes per pixel (= 2 for 16bpp)
    byte* data_ptr;      // +0x14: pixel data
    int data_size;       // +0x18
    bool owns_data;      // +0x1C
};
```

### ABuffer Initialization

Created in WinMain at `0x006bdedd`:
```c
g_ABuffer = BSurface_Constructor(viewport_x, viewport_y, 480, 480 - viewport_y);
// Filled with 0x7F (neutral alpha)
```

Destroyed and recreated by `Set_View_Dimensions` (0x004a8960) when viewport size changes.

---

## Rules.ini Fog/Shroud Settings (Complete Map)

### [General] Section

| INI Key | RulesClass Offset | Type | Default | Purpose |
|---------|-------------------|------|---------|---------|
| `BlendedFog` | 0x17E2 | bool | true | When false, uses checkerboard dithering instead of smooth blended fog transitions |
| `RevealByHeight` | 0x17EE | bool | true | Units at higher elevation see further (sight range bonus from height) |
| `AllowShroudedSubteranneanMoves` | 0x17EF | bool | false | Subterranean units can move through unexplored cells |
| `AircraftFogReveal` | 0x00F4 | int | 6 | Sight range override for aircraft fog reveal |
| `RevealTriggerRadius` | 0x062C | int | 5 | Radius for map trigger reveal actions |

### [AudioVisual] Section

| INI Key | RulesClass Offset | Type | Default | Purpose |
|---------|-------------------|------|---------|---------|
| `ShroudGrow` | 0x17F0 | bool | false | Master switch for shroud regrowth |
| `ShroudRate` | 0x1640 | double | 4.0 | Interval (minutes) between shroud regrow passes |
| `FogRate` | 0x1648 | double | 0.05 | Interval for fog-of-war refresh (spy satellite updates) |
| `AllyReveal` | 0x17E7 | bool | true | Allied players share vision |

### [MultiplayerDialogSettings] Section

| INI Key | SessionClass Offset | Type | Default | Purpose |
|---------|---------------------|------|---------|---------|
| `ShadowGrow` | 0x14AD | bool | false | Shroud regrows in multiplayer |
| `Shroud` | 0x14AE | bool | true | Map starts with shroud |
| `FogOfWar` | 0x14B7 | bool | false | Fog-of-war enabled for session |

### [SpecialFlags] Section

FogOfWar is also stored as **bit 12** (0x1000) of the SpecialFlags bitfield at `*DAT_00a8b230`. This is the runtime flag checked by all fog rendering code.

### Per-Type (TechnoTypeClass)

| INI Key | Offset | Purpose |
|---------|--------|---------|
| `GapRadiusInCells` | TypeClass | Gap generator concealment radius |
| `SuperGapRadiusInCells` | TypeClass | Super gap generator radius |

---

## CellChangeNotify (0x005865f0)

Called when a cell's visibility state changes:

```c
void CellChangeNotify(CellClass* cell, param2, bool was_revealed) {
    CellXY origin = cell->cell_xy;
    int step = 1;
    // Iterate 7-step diagonal strip from origin
    do {
        CellClass* target = GetCell(origin.x, origin.y);
        if (step-2 <= target->gap_opacity && target->gap_opacity <= step) {
            if (was_revealed)
                RadarClass::MarkObjectDirty(target->cell_xy);
            // Find nearest object at cell and call virtual 0x198
            ObjectClass* obj = target->FindNearestObject();
            if (obj) obj->vfunc_0x198(param2);
        }
        origin.x++; origin.y++;
        step += 2;
    } while (step < 15);
}
```

This notifies objects along a diagonal strip that visibility changed — used for radar updates and object state notifications (like stopping production animations for buildings that enter fog).

---

## Spy Satellite Reveal Cycle (FUN_004acbc0)

When the fog timer fires (controlled by `FogRate`):

```c
void SpySatRevealCycle() {
    // Phase 1: Mark cells that are visible but not interior
    for each cell:
        if (!(fog_flags & 0x01) && (fog_flags & 0x02)):  // not interior, is revealed
            fog_flags |= 0x40;                             // mark for processing

    // Phase 2: Re-reveal all visible units
    RevealAllVisibleUnits(shroud_mode=0, fog_mode=1);

    // Phase 3: Re-fog marked cells
    for each cell:
        if (fog_flags & 0x40):
            fog_flags &= ~0x40;
            RefogCell(cell_xy);   // recursive re-fogging
}
```

This is how spy satellite vision works: it periodically re-reveals all cells that player units can see, then re-fogs everything else. The fog layer is rebuilt from scratch each cycle.

---

## IsoDiamond Pixel Offset Table (0x007ec450)

30-entry table giving cumulative pixel offsets for isometric diamond scanlines:

```
Row  0: offset=0   width=4    (center 2px from each edge)
Row  1: offset=4   width=8
Row  2: offset=12  width=12
...grows by 4 pixels per row...
Row 14: offset=420 width=60   (full width — widest row)
Row 15: offset=480 width=56   (shrinks)
...shrinks by 4 pixels per row...
Row 28: offset=896 width=4
Row 29: offset=900 width=0    (end sentinel)
```

Total: 900 pixels in the diamond. Used by `TMP_TileBlitter` to clip tile rendering to the isometric diamond shape. Two companion tables (`g_IsoDiamond_ScreenOffsets` and `g_IsoDiamond_ScanlineWidths`) provide per-row X offsets and width counts for the blitter inner loop.

---

## Object Visibility and Fog

### The Core Draw Gate: ObjectClass+0x81 (IsUndiscovered)

Objects are NOT checked against fog every frame. Instead, they have a persistent `IsUndiscovered` flag:

- **Initialized to 1** in ObjectClass constructor — objects start hidden
- **Cleared to 0** by `ObjectClass::Reveal` (vtable 0xD8, at 0x005f4ec0) — makes the object drawable, submits it to the display layer, AND creates an AlphaShape for future fogging
- **Set back to 1** by `ObjectClass::Conceal` (vtable 0xD4, at 0x005f4d30) — removes from display layer
- **Checked in `ObjectClass::DrawIt`** (vtable 0x104) — if non-zero, draw returns immediately

### How Objects Get Revealed/Concealed

```
MapClass::RevealShroud called (unit sight covers cell)
  └── CellChangeNotify → iterates objects in cell
       └── TechnoClass::Discover (vtable 0x198)
            └── ObjectClass::Reveal → clears +0x81, submits to display layer

Cell re-fogged (unit moves away, shroud regrows)
  └── ObjectClass::Conceal → sets +0x81, removes from display layer
       └── FoggedObjectClass ghost created at CellClass+0x28
```

### Fogged Object Ghosts (FogOfWar mode only)

When FogOfWar is enabled and a cell transitions from visible to fogged:
1. A `FoggedObjectClass` record is created for each building in the cell
2. Stored at `CellClass+0x28` (pointer to fog record)
3. Rendered as dim/translucent using AlphaShape compositing during shroud edge pass
4. Destroyed when the cell is re-revealed

**Objects in shrouded (never-seen) cells are completely invisible** — they don't exist in the display layer at all.

### Visual State for Cloaked Units

`TechnoClass::GetVisualState` (vtable 0x68) returns 0-5:
- State 0-4: various visibility levels with translucency
- State 5: invisible (fully cloaked, no sensor)

This is separate from fog — it handles stealth/cloaking effects.

---

## BlendedFog Rendering Paths in TMP_TileBlitter

### BlendedFog=true (default): 50% Brightness Reduction

```c
// For each fogged tile pixel:
if (zbuffer_threshold < *zbuffer_pixel) {
    *screen = (*screen >> 1 & mask565) + tint_color;
}
*zbuffer = 0;
```

Halves the pixel brightness via right-shift, preserving RGB565 channel boundaries with the mask, then adds a tint offset. This produces smooth semi-dark fogged tiles.

### BlendedFog=false: Checkerboard Dithering

```c
uint toggle = (start_x + start_y + row) & 1;  // alternating
for each pixel:
    if (toggle != 0) {
        *screen = fog_tint_color;
    }
    *zbuffer = 0;
    toggle = (toggle == 0);  // flip every pixel
```

Creates a 50% checkerboard pattern — every other pixel is replaced with the fog tint color. This is the "retro" dithered look from older Westwood games.

---

## Radar/Minimap Fog Rendering

### Radar Fog Overlay (DrawRadarOverlay_fog at 0x0063cae0)

For each object with fog overlay needs:
1. Convert world coords to radar pixel coords
2. Compute viewport-relative position
3. Call `FUN_0063d810` to draw **dashed lines** on the radar:
   - `param_7=1` → dashed pattern: `DSurface::DrawDashedLine()` with time-varying phase (`timeGetTime() >> 5 & 0xF`) for animated shimmer
   - `param_7=0` → solid line: `DSurface::DrawLine()`

The dashed lines animate over time, creating the distinctive RA2 "radar shimmer" effect in fogged areas.

### Normal Radar Rendering (DrawRadarOverlay_normal at 0x0063c690)

For visible areas, the radar renders normally. Cells use `CellClass::GetRadarColor` which reads the cell's terrain type, overlay type, and tiberium type to produce RGB colors. Fogged cells show their last-known color with the dashed overlay on top.

---

## Observer/Spectator Mode

There is **NO** dedicated spectator mode in the original engine. Observing works through the defeated player path:

1. **ObserverMode** (`0x00ac10c8`): Lobby setting from `[MultiPlayer]ObserverMode` in RA2MD.INI
2. **Observer slot**: Observers are assigned to slot 7 (8th position) with team index -1, using `OBSERVER.PAL`
3. **Defeat → Observer**: When `HouseClass::MPlayer_Defeated` fires:
   - `IsDefeated` (+0x1F5) = true
   - `MapIsClear` (+0x241) = true
   - `RevealEntireMap` (0x00577f30) iterates every cell calling RevealCell
   - `Visionary` (+0x240) = true, radar enabled
4. **Fog skipped**: `Shroud_fog_edge_rendering` checks `g_PlayerPtr+0x1F5` — defeated players skip fog edge drawing

**Multiplayer recording**: The game has record/playback modes (bit 0/1 of `DAT_00a8d5f8`), but replays are tied to a specific player's perspective — no omniscient replay viewer.

**Cooperative mode check**: The `(g_GameMode == 3 || g_GameMode == 4) && DAT_00a8b23c` check in `MapClass::RevealShroud` is a cooperative mode check (NOT observer), controlling edge-cell skipping during shroud reveal.

---

## Full 3-Pass Rendering Pipeline

The main draw function `TacticalClass_Draw` (0x006d3d10) is called **three times per frame** from `RenderFrame_main` (0x004f4480):

### Pass 0 (Buffer Management)
- Computes viewport scroll delta
- **Full redraw**: Clears ZBuffer and ABuffer entirely
- **Scroll**: Scrolls both circular buffers (no data copy needed)
- Swaps back/composition surfaces
- Returns early — draws nothing visible

### Pass 1 (Terrain — draws to back buffer)
1. **`Tactical_ZBufferDirtyClear`** (0x006d2b60) — ZBuffer dirty rect clear; for shroud cells: blits shroud tile via TMP_TileBlitter
2. **`Tactical_layer_shroud_edges`** (0x006d3660) — **Writes ABuffer** with shroud/fog alpha values; clears ABuffer regions to 0x7F before re-rendering; calls AlphaShape compositing
3. **`Tactical_layer_terrain_shadows`** (0x006d2de0) — Shadow decals
4. **`Tactical_layer_base_terrain`** (0x006d3470) — ISO tiles + ZBuffer writes. **Reads ABuffer** per-pixel for fog darkening
5. **`Tactical_layer_smudges`** (0x006d3290) — Craters/scorch marks
6. **`Tactical_layer_building_overlays`** (0x006d3ac0) — Building flat anims
7. **`Tactical_layer_overlays`** (0x006d3040) — Walls/ore/tiberium
8. **`Tactical_layer_animations`** (0x006d3870) — Ground-level flat anims

### Between Passes: Sidebar/UI drawn

### Pass 2 (Objects — draws to composition surface)
27+ sequential steps including:
- Rally point lines, mind-control links, building placement ghost
- **Main object rendering loop** — 5 display layers with Y-sorting
- Building turret pass, extras pass
- Particle systems, laser beams, electric bolts, trails
- Wave effects (magnetron, etc.)
- **Selection brackets**, garrison pips
- Band-box selection rectangle
- **Radar overlays** (fog and normal)
- Super weapon circles
- PixelFX/tiberium glow, floating text

### ABuffer Lifecycle Per Frame

```
Pass 0: ABuffer fully cleared (CircBuf__FillAll with 0x7F)
  ↓
Pass 1 Step 2: Shroud edges write to ABuffer (per-pixel alpha values)
  ↓
Pass 1 Steps 4-8: Terrain blitters READ ABuffer (darken pixels via remap tables)
  ↓
Pass 2: Object blitters READ ABuffer (darken sprite pixels)
```

---

## SHROUD.SHP / FOG.SHP Frame Details

### SHP File Format
- **Header**: 8 bytes (zero, width, height, frame_count)
- **Per-frame header**: 24 bytes (x, y, width, height, flags, padding, data_offset)
- **Pixel data**: palette-indexed, compressed (RLE-zero or length-prefixed)

### Expected Dimensions
- **Frame canvas**: 60×30 pixels (matching `TILE_WIDTH` × `TILE_HEIGHT`)
- **Frame 0**: Likely empty (0×0) — represents "no shroud needed"
- **Frame 15 (0x0F)**: Full 60×30 solid diamond — used for fully-covered cells
- **Frames 1-46**: Edge transition shapes, individual frames may be smaller sub-regions with x/y offsets

### Pixel Semantics
- **Shroud mode**: Pixel 0x00 = full black, 0x7F = neutral, 0xFE = transparent (skip). Direct write to ABuffer.
- **Fog mode**: Pixel < 0x80 = fog intensity (blended with existing ABuffer). Pixel >= 0x80 = transparent.

### Current Engine Status
Neither SHROUD.SHP nor FOG.SHP is currently loaded by our engine. Our `mix-browser` tool can inspect them (search for "shroud.shp" or "fog.shp"), but note they use 0xFE as transparency instead of the standard 0.

---

## Companion Reports

Detailed companion reports from this investigation:
- **[TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md](TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md)** — Complete 3-pass rendering pipeline with all 50+ function addresses
- **[OBSERVER_SPECTATOR_FOG_GHIDRA_REPORT.md](OBSERVER_SPECTATOR_FOG_GHIDRA_REPORT.md)** — Observer mode, game modes, defeat→reveal flow
- **[OBJECT_FOG_VISIBILITY_GHIDRA_REPORT.md](OBJECT_FOG_VISIBILITY_GHIDRA_REPORT.md)** — Object visibility decision tree, IsUndiscovered flag
- **[ALPHA_SHAPE_CLASS_LIFECYCLE.md](ALPHA_SHAPE_CLASS_LIFECYCLE.md)** — AlphaShapeClass structure and lifecycle
- **[BSURFACE_CIRCBUF_ABUFFER_REPORT.md](BSURFACE_CIRCBUF_ABUFFER_REPORT.md)** — BSurface/CircBuf/ABuffer internals

---

## Comparison with Our Implementation

### Critical Differences

| Aspect | Original (gamemd.exe) | Our Implementation | Impact |
|--------|----------------------|-------------------|--------|
| **Edge bitmask** | 8-bit (8 neighbors) → 47 frames | 4-bit (4 neighbors) → 16 states | Less nuanced edge transitions; missing corner handling |
| **Edge rendering** | SHP sprites (SHROUD.SHP/FOG.SHP) per pixel | Bilinear GPU filtering + box blur | Different visual style (smoother/modern vs pixel-accurate) |
| **Fog application** | Per-pixel during ALL blitting via ABuffer remap | Post-process fullscreen overlay | Similar result; ours is simpler but sits on top of everything |
| **Object visibility** | IsUndiscovered flag + display layer management | Per-frame fog check in app_instances | Different mechanism, same effect |
| **Fogged objects** | AlphaShape SHP silhouettes via ABuffer blend table | BuildingSnapshot system | Needs verification that behavior matches |
| **Dirty tracking** | Per-cell dirty list (max 799), incremental updates | Full mask regeneration per fog generation | Less efficient but simpler; acceptable for GPU approach |
| **Gap generators** | 15-step animation, re-shrouds cells, 3-tier visibility | Not yet implemented | Missing feature |
| **Fog counters** | 3 separate: friendly vision, gap concealment, per-house bitmask | Single per-house grid | Missing gap counter system |
| **Shroud regrow** | Periodic reconcealment (ShroudGrow + ShroudRate timer) | Not implemented | Missing feature |
| **BlendedFog** | Two paths: smooth 50% darken vs checkerboard dither | Single GPU blend | Only smooth path; missing dither option |
| **Observer mode** | Defeated player path (RevealEntireMap) | Not implemented | Missing for multiplayer |
| **Radar fog** | Animated dashed lines on radar overlay | Dim factor on minimap | Different visual (ours simpler) |

### Our approach is valid but different

Our BAR-style GPU approach (bilinear-filtered fog mask + fullscreen overlay) is a deliberate modernization. It produces smooth, anti-aliased fog transitions that look good at any resolution. The original's SHP-based approach was constrained by the 640×480 software renderer.

**Key areas to potentially improve:**
1. **Gap generator visual effects** — 15-step animation, re-shrouds cells (not yet implemented)
2. **Three-tier visibility** — our sim only has per-house grids, needs gap concealment counters
3. **Shroud regrow** — periodic re-concealment controlled by ShroudGrow/ShroudRate
4. **Spy satellite fog cycle** — periodic re-reveal/re-fog controlled by FogRate
5. **Height-adjusted IsShrouded/IsFogged** — coordinate shift on elevated terrain
6. **Defeated player fog bypass** (HouseClass+0x1F5 check) for observer mode
7. **Object reveal/conceal lifecycle** — IsUndiscovered flag instead of per-frame checks
8. **BlendedFog=false dither mode** — checkerboard pattern option

---

## Confidence Levels

### High confidence (verified from binary)
- ABuffer is 16-bit per-pixel (CircBuf wrapper over BSurface), cleared to 0x7F, read by all blitters
- SHROUD.SHP and FOG.SHP loaded from MIX archives
- Edge frame determined by 8-neighbor bitmask → 256-byte lookup table (complete table extracted)
- Shroud edges write to ABuffer only, NOT to ZBuffer
- Fog edge blitter uses additive blending: `(ABuffer - 0x7F) + shp_value`, clamped to 0
- Alpha blend table formula: `(hi * lo) / 0x7F`, 64KB
- FogOfWar enabled = bit 12 (0x1000) of SpecialFlags
- HouseClass+0x1F5 = `IsDefeated` (set by MPlayer_Defeated, used in 53+ locations)
- Cell+0x120/0x121 = cached shroud/fog edge frame indices
- Cell+0x12C bits 3&4 = explored & interior flags
- **Three-tier visibility: cell+0x78 (per-house bitmask), cell+0x13C (friendly vision counter), cell+0x130 (gap concealment counter)**
- **cell+0x13C = friendly vision counter**, incremented for player/allied units, used by IsFogged()
- **cell+0x130 = gap concealment counter**, incremented for enemy gap generators, re-shrouds when > 0
- **cell+0x134 = gap counter cap/tracking**, decremented on gap removal
- Cell+0x140 = fog render flags (bits 0, 1, 5, 6, 22 identified)
- Diamond mask at 0x007e2b20 = 60×30 isometric shape (0x20=skip, 0xDB=draw)
- Sight range table: [1,9,21,37,61,89,121,161,205,253,309,369]
- Dirty list: TacticalClass+0xE0 count, +0xE4 array, max 799 entries
- Gap generators: 15-step animation, 21-cell coverage, state machine
- MapClass::ResetShroud initializes fog counter to 1 (not 0)
- MapClass::ClearShroud sets explored+interior+revealed, clears gap counters
- AlphaShapeClass: 0x40 bytes, stores source object ptr, world coords, SHP data, disabled flag
- AlphaShape lifecycle: created in ObjectClass::Unlimbo, disabled via notification, cleaned up per tick
- Brightness lookup table: `(lo * hi * (max_brightness-1)) / 0x7E02`, cached with refcount
- CircBuf wraps BSurface with circular offset for scroll-without-copy
- IsShrouded/IsFogged both adjust for terrain height (coordinate shift on elevated cells)
- Shroud regrow: marks edge cells, then clears explored+interior recursively (FUN_004acac0/004acda0)
- Re-fog: clears revealed+interior, sets fog frame to 0xFE, propagates to neighbors recursively
- CellChangeNotify: diagonal strip notification for radar + object state updates
- Rules.ini: BlendedFog, RevealByHeight, ShroudGrow, ShroudRate, FogRate, AllyReveal, AircraftFogReveal, RevealTriggerRadius

### Medium confidence (inferred from code structure)
- SHROUD.SHP/FOG.SHP have exactly 47 frames (based on max LUT value being 46)
- The isometric diamond offsets (-30, -15) for shroud tile positioning
- Cell+0x138 is a needs-redraw flag for fog edge changes
- Cell+0x5C stores last dirty frame counter
- cell+0x78 per-house bitmask is primarily used for cloaking/stealth mechanics, not fog rendering
- IsoDiamond pixel offset table widths: 4,8,12,...,60,56,...,8,4 (900 total pixels)
- Spy satellite cycle: mark visible cells → re-reveal all units → re-fog marked cells
- FUN_00486a70 creates AlphaShape objects when cells become re-fogged (only for buildings)

### Low confidence (needs further verification)
- Exact TMP tile blitter remap table structure (complex multi-level indirection)
- Exact pixel values within SHROUD.SHP/FOG.SHP frame data (would require hex-dumping the retail SHP files)
- The reveal_spiral data at 0x00abd490 appears to be .bss (populated at runtime during map init)
- g_PlayerPtr+0x577A flag checked in RemoveCloakShroud (some player state flag)
- Full interaction between spy satellite reveal and gap generator concealment
