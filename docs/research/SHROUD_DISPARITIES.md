# Shroud System — Disparities Between gamemd.exe and Our Implementation

Findings from deep Ghidra decompilation compared against our Rust code.

---

## DISPARITY 1: Frame 15 vs Computed Diamond (VISUAL)

**gamemd:** When `Shroud_EdgeBitmask_Calculator` returns -2 (0xFE = fully surrounded by
shroud), `Shroud_fog_edge_rendering` remaps it to **frame 15** and calls
`ShroudEdge_BlitToABuffer(screen_pos, clip_rect, 15)`. Frame 15 is a real SHROUD.SHP
frame with actual SHP pixel data — a full 60x30 diamond filled with 0x00 pixels.

**Our code:** For the `0xFE` case, we call `blit_dark_diamond()` which computes a
diamond shape algorithmically and fills it with BLACK (0x00).

**The problem:** Our computed diamond includes row 0 (the 2-pixel tip). The comment at
[shroud_buffer.rs:349](src/render/shroud_buffer.rs#L349) says "Frame 15 starts at row 1
(width 4), missing row 0. We include it." This means our dark diamond extends 1 pixel
higher than frame 15's actual coverage. Whether this matters depends on whether the
1-pixel difference at the very tip creates a visible seam.

**Fix:** Use `blit_frame(15, ...)` for the `0xFE` case instead of `blit_dark_diamond()`.
This renders the actual SHP frame 15 data, matching gamemd exactly. Keep
`blit_dark_diamond()` only for cells that are unexplored (where no edge frame applies).

**But also:** For **unexplored cells** (the `!fog.is_cell_revealed()` path at line 250),
gamemd uses `FUN_00480180` which calls `TMP_TileBlitter` with a clear tile — NOT
SHROUD.SHP. This writes 0xFFFF to the ZBuffer (depth clear) and may or may not write
visible pixels. The blackness for those cells comes from the ABuffer being filled with
0x00 via frame 15 that was blitted during the edge pass. So gamemd's flow for
unexplored cells is:
1. Bitmask calculator returns -2 (fully surrounded)
2. Frame 15 blitted to ABuffer → all pixels become 0x00
3. TMP_TileBlitter reads ABuffer per-pixel → darkens to black
4. ZBuffer reset to 0xFFFF → prevents objects behind shroud from showing

We skip steps 3-4 and just fill our buffer with 0x00 directly. Visually equivalent for
the brightness buffer, but we miss the ZBuffer clear (see Disparity 5).

**Severity:** LOW — the 1-pixel seam is barely visible, but easy to fix.

---

## DISPARITY 2: Gap Generator Is Two Systems, Not One (BEHAVIORAL)

**gamemd:** The "Gap Generator" building (`GAGENR`) uses TWO separate INI flags processed
in the same tick function (`BuildingClass::UpdateGapGenerator_Tick` at 0x00454db0):

1. **`GapGenerator=yes`** (TechnoTypeClass+0xCD1): Manages a "Shroud Map" overlay. Has a
   state machine (states 0-3: inactive/expanding/active/contracting) with animation.
   Increments `cell+0x130` (gap counter), sets `cell+0x140 |= 0x20` (GAP flag).
   Uses `GapRadiusInCells` from the same type class.

2. **`CloakGenerator=yes`** (BuildingTypeClass+0x16C7): Manages per-house visibility bits
   at `cell+0x78`. Uses a **Bresenham circle rasterizer** (FUN_007bb920) to iterate cells
   in a circular radius. For each cell, sets owner's bit and clears enemy bits. Uses
   `CloakRadiusInCells` (BuildingTypeClass+0x1707).

The GAGENR building has BOTH flags set. The visual "shrouding" effect for enemies comes
from the CloakGenerator clearing their visibility bits. The GapGenerator part manages the
shroud map overlay (the expanding/contracting visual bubble).

**Our code:** We have a single `apply_gap_generators()` that clears `FLAG_VISIBLE` for
enemies in a radius. This approximates the CloakGenerator behavior but misses:
- The GapGenerator state machine (expand/contract animation)
- The separate cell+0x78 per-house visibility bitmask
- The `cell+0x130` gap coverage reference counting for overlapping gap generators
- The `cell+0x140` GAP overlay flag

**Visual impact for enemies:** Gap-covered cells appear **FULLY BLACK** (same as
never-explored) to enemies, NOT dimmed/fogged. The fog-of-war semi-transparent dimming
is gated behind `FogOfWar=true` which is OFF in standard YR. So for shroud rendering
purposes, our approach of clearing FLAG_VISIBLE is almost correct, BUT:

**Critical:** `RevealShroudFlags` at 0x004876f0 does:
```c
cell->flags_12C |= 0x18;       // mark explored
if (cell->gap_counter > 0) {
    cell->flags_140 |= 0x20;   // re-assert GAP flag
}
```
This means the gap generator **wins over normal reveal**. If a unit tries to reveal a
gap-covered cell, it gets marked explored but the GAP flag immediately reasserts. Our
implementation may allow reveals to "punch through" gap coverage because we apply gap
suppression after all reveals, not during each individual reveal call.

**Fix:** Our per-tick full recompute (clear VISIBLE, re-reveal, then suppress gap) is
functionally equivalent IF we suppress correctly. Verify that gap suppression clears
VISIBLE even for cells that were just revealed by a unit standing inside the gap radius.
Currently this should work because `apply_gap_generators()` runs AFTER all reveals.

**Severity:** MEDIUM — the ordering is probably correct, but the expand/contract
animation and reference counting for overlapping gap generators are missing.

---

## DISPARITY 3: Brightness Curve (AESTHETIC)

**gamemd:** The ABuffer brightness is applied via a precomputed **remap table** (built by
`FUN_00420140`). The formula for each table entry:
```c
table[abuf_val * 256 + opacity] = (abuf_val * opacity * (max_brightness - 1)) / 0x7E02;
```
This is then used during tile blitting:
```c
brightness = remap_table[abuffer_value];
final_color = palette_color_table[brightness | palette_index];
```
The remap goes through a **palette-based color table** that maps (brightness, palette_index)
to a final RGB565 color. This means darkening is **palette-aware** — it follows the
palette's specific color relationships, not a generic linear multiply.

**Our code:** Full-screen GPU multiply pass:
```wgsl
let brightness = clamp(shroud_val * 2.008, 0.0, 1.0);
let corrected = pow(brightness, 2.2);  // sRGB compensation
return vec4f(corrected, corrected, corrected, 1.0);
// Blended as: final = src * dst (multiplicative)
```

**Differences:**
1. Our multiply is **linear RGB** — every color channel scaled identically. gamemd's
   palette remap can produce different color shifts per palette entry.
2. The `pow(2.2)` sRGB compensation produces a different curve than gamemd's linear
   `(a * b) / 0x7E02` formula.
3. Gradient edges will darken differently in the mid-tones (values 0x20-0x60).

**Severity:** LOW — this is an inherent difference of GPU rendering vs palette-based
rendering. The visual result is close enough. No fix needed unless we want perfect
palette fidelity (which would require rendering through palette tables, defeating the
purpose of GPU acceleration).

---

## DISPARITY 4: ZBuffer Clear for Shrouded Cells (VISUAL)

**gamemd:** `FUN_00480180` is called for fully-shrouded cells and writes **0xFFFF** (max
depth) to the ZBuffer for the diamond area. This prevents any objects (buildings, units)
in shrouded cells from being visible — their depth values can never beat 0xFFFF.

```c
TMP_TileBlitter(
    LightConvertClass[0], 0, g_PrimarySurface,
    screen_x, screen_y, clip_rect,
    cell_level, cell_tile_z,
    1,  // ZBuffer write ON
    0, 0,
    1,  // ZBuffer-clear-only mode (write ZBuffer, NOT screen pixels)
    0, 0
);
```

**Our code:** We don't touch the depth buffer for shrouded cells. Our shroud is purely
a post-process brightness multiply — objects behind shroud are rendered normally and
then darkened to black by the multiply pass.

**Impact:** In practice, this works because the multiply pass makes everything black
anyway. BUT: if an object in a shrouded cell has alpha/transparency effects, it could
potentially "leak" through the shroud multiply. Also, any fragment that renders AFTER
the shroud multiply pass (e.g., UI elements drawn at the same pass level) would not
be darkened.

**Fix (if needed):** Before the shroud multiply pass, issue a depth-only pass that
writes max depth for all shrouded cell diamonds. Or simpler: ensure the shroud multiply
pass runs LAST in the scene rendering (which it currently does at step 9).

**Severity:** LOW — our rendering order (multiply last) produces correct results for
opaque sprites. Only transparent effects could theoretically leak.

---

## DISPARITY 5: Incremental vs Full Rebuild (PERFORMANCE)

**gamemd:** Uses a sophisticated incremental system:
- Per-cell dirty flag at `cell+0x138`
- Frame counter dedup at `cell+0x5C` (max once per frame)
- Dirty cell list at `DisplayClass+0xE4` (max 799 cells per frame)
- Dirty rect regions clear ABuffer to 0x7F then re-blit only affected cells
- When scrolling, circular buffer offset shifts without data copy

**Our code:** Full buffer rebuild every frame when camera moves OR fog generation changes.
Fills entire buffer with 0x7F, then iterates ALL map cells.

**Impact:** With a 200x200 map scrolling at 60fps, we process 40,000 cells per frame.
gamemd processes only the ~799 dirty cells plus visible cells in dirty rects. However,
our CPU blit loop is simple and cache-friendly, and we only run it when something
actually changes (fog generation counter or camera pixel position).

**Severity:** LOW for now — if performance becomes an issue, add viewport-bounded
iteration (only process cells overlapping the screen). The gamemd-style dirty cell
system is overkill for our architecture.

---

## DISPARITY 6: Two-Ring Neighbor Edge Propagation (CORRECTNESS)

**gamemd:** `DisplayClass::Reveal_Cell_And_Neighbors` (0x004a9890) does cascading edge
recomputation when a cell is revealed:
1. Compute new edge frame for the revealed cell
2. If edge frame changed, mark cell dirty for redraw
3. If cell is now fully clear (bitmask = -1), set `cell+0x12C |= 0x10` (fully revealed)
4. For each of 8 neighbors:
   - Recompute their edge frames
   - If a neighbor ALSO becomes fully clear (bitmask = -1):
     - Set its fully-revealed flag
     - **Recursively update THAT neighbor's 8 neighbors** (2-ring propagation)
5. This cascading can reveal chains of cells if revealing one cell causes adjacent
   unexplored cells to also have no shrouded neighbors

**Our code:** We don't do explicit edge propagation. We recompute ALL edges from scratch
every frame in `rebuild_if_needed()` by iterating all cells and calling
`shroud_edge_mask_8bit()`.

**Impact:** None — our full recompute produces the same result as gamemd's incremental
propagation. The cascading reveal doesn't apply because we recompute everything anyway.
The only edge case would be if the cascading reveal affects *visibility state* (not just
edge frames), but `Reveal_Cell_And_Neighbors` only updates cached edge frames and dirty
flags, not actual explored/revealed bits.

**Severity:** NONE — our full recompute is equivalent.

---

## DISPARITY 7: SHP Draw Offsets in Frame Blitting (VISUAL)

**gamemd:** `ShroudEdge_BlitToABuffer` reads the SHP frame's draw offsets (frame_x,
frame_y from the SHP header) and adds them to the screen position:
```c
actual_draw_x = canvas_origin_x + frame_x;
actual_draw_y = canvas_origin_y + frame_y;
```
The frame's sub-image is positioned within the 60x30 canvas using these offsets.

**Our code:** `extract_shp_brightness()` pre-bakes the frame offsets into the canvas
buffer during extraction:
```rust
let dx = fx + col;  // fx = frame.frame_x
let dy = fy + row;  // fy = frame.frame_y
buf[(dy * canvas_w + dx) as usize] = pixel;
```
Then `blit_frame()` writes the entire canvas without additional offsets.

**Impact:** Functionally equivalent — we apply the offsets at extraction time rather
than at blit time. The pixels end up in the same screen positions. However, if any
SHROUD.SHP frame has a non-zero draw offset that positions pixels OUTSIDE the canvas
bounds, our extraction would clip them (the `if dx < canvas_w && dy < canvas_h` check).
gamemd would draw them at the adjusted position, potentially outside the canvas.

**Severity:** NONE in practice — SHROUD.SHP frames are designed to fit within their
canvas. The pre-baking approach is correct.

---

## DISPARITY 8: Shroud Coordinate System (NOT A BUG)

One investigation found that gamemd computes shroud canvas origin as:
```
canvas_x = CoordsToClient(cell_center).x - cam_x - 30
canvas_y = CoordsToClient(cell_center).y - cam_y - 15
```
Which gives `canvas_y = 15*(rx+ry) - cam_y`.

Our code gives `vy = iso_to_screen(rx, ry, 0).y - cam_y = 15*(rx+ry) + 15 - cam_y`.

**This is NOT a visual bug.** Both terrain tiles and shroud diamonds use `iso_to_screen`
in our engine, so they're perfectly aligned with each other. The +15 offset is a
consistent difference in our coordinate system's origin vs gamemd's, compensated by our
camera positioning. The visual result is identical.

---

## ~~DISPARITY 9~~ CORRECTED: Gap Generator Is a Separate Visual System

**Previously claimed:** Gap-covered cells should render as dark through the shroud edge
system. This was **WRONG**.

**Actual gamemd behavior (verified from binary):**
- `Shroud_EdgeBitmask_Calculator` checks `cell+0x12C & 0x08` (explored bit)
- `GapGenerator_CoverCell` (0x487690) does NOT modify `cell+0x12C`
- Therefore the shroud edge renderer still sees gap-covered cells as **explored**
- No SHROUD.SHP edges are drawn around gap-covered areas

The gap generator's visual darkening uses a **completely separate rendering system**:
1. **AlphaShapeClass overlay** — 21 animation objects per gap gen (building+0x55C..+0x5B0)
   with translucency values 0-15 for gradual expand/contract
2. **Dedicated shroud map surface** (DAT_0089ddc0) drawn with Bresenham circle rasterizer
   (FUN_007bb920) using GapRadiusInCells
3. **4-state machine** (inactive/expanding/active/contracting) with 15-step counter at
   building+0x6ED
4. Alpha shapes blended onto ABuffer via lookup table at DAT_0088a118

Also confirmed from INI: `[GAGAP]` has `GapGenerator=yes` but does NOT have
`CloakGenerator=yes`. These are separate features. CloakGenerator manages per-house
cell visibility bits (cell+0x78); GapGenerator manages the visual overlay.

**Our implementation status:** We approximate gap behavior by clearing FLAG_VISIBLE for
enemies in `apply_gap_generators()`. This correctly affects:
- Combat targeting (enemies can't target through gap)
- Entity visibility filtering (if using `is_cell_visible()`)

But we do NOT have:
- The AlphaShapeClass visual overlay system (the dark circle rendering)
- The expand/contract animation (15-step gradual transition)
- The shroud map surface for circular pattern rendering

**This is NOT a shroud system disparity — it's a missing feature (gap generator visual
overlay) that needs its own implementation, separate from shroud.**

**Severity:** N/A for shroud — moved to gap generator implementation scope.

---

## DISPARITY 10: Entity Visibility Check Uses Wrong Method (BEHAVIORAL)

**gamemd:** Objects in cells where the enemy has no visibility (cell+0x78 bit cleared)
are hidden via `ObjectClass+0x81` (IsUndiscovered = 1). The gap generator and
shroud systems both contribute to this.

**Our code:** Entity filtering in app_instances/helpers.rs uses `is_cell_revealed()`
to decide whether to render enemy entities. This is correct for normal shroud (once
revealed, always visible in standard YR), but it does NOT account for gap generator
suppression of `FLAG_VISIBLE`.

**Fix:** For enemy entities, check `is_cell_visible()` instead of `is_cell_revealed()`.
`is_cell_visible()` returns false when gap generators have suppressed VISIBLE, correctly
hiding enemies inside a gap field. Friendly entities should still use
`is_cell_revealed()` (always visible to their owner).

**Severity:** MEDIUM — enemy units inside a gap field should be hidden from rendering
and targeting. Currently they remain visible.

---

## Summary Table

| # | Disparity | Visual Impact | Severity | Fix Effort |
|---|-----------|--------------|----------|------------|
| 1 | Frame 15 vs computed diamond | 1px seam at diamond tip | LOW | LOW — use blit_frame(15) |
| 2 | Gap gen is separate visual system | Missing dark circle overlay | MEDIUM | HIGH (new subsystem) |
| 3 | Brightness curve (linear vs remap) | Slightly different gradient tones | LOW | N/A (acceptable) |
| 4 | No ZBuffer clear for shrouded cells | Transparent sprite leakage | LOW | LOW |
| 5 | Full rebuild vs incremental dirty | Performance only | LOW | MEDIUM |
| 6 | Two-ring edge propagation | None (full recompute equivalent) | NONE | N/A |
| 7 | SHP draw offset pre-baking | None (functionally equivalent) | NONE | N/A |
| 8 | Coordinate system +15 offset | None (self-consistent) | NONE | N/A |
| 9 | ~~Gap not shroud-rendered~~ | **CORRECTED** — gap uses separate overlay | N/A | N/A |
| 10 | Entities visible in gap areas | Enemies render through gap | MEDIUM | LOW |

---

## Priority Fix List

### Should Fix (causes visible incorrectness)

**1. Entity visibility in gap areas (Disparity 10)**
Change entity filtering from `is_cell_revealed()` to `is_cell_visible()` for enemy
entities. Keep `is_cell_revealed()` for friendly entities.

**2. Use actual SHP frame 15 for fully-surrounded cells (Disparity 1)**
In `rebuild_if_needed()`, change the `0xFE` branch to `blit_frame(15, ...)`.

### Separate Implementation Scope (not a shroud disparity)

**3. Gap Generator visual overlay (Disparity 2)**
The gap generator's dark circle is its own rendering subsystem:
- AlphaShapeClass overlay with 21 animation objects
- Dedicated shroud map surface (Bresenham circle rasterizer)
- 4-state machine with 15-step expand/contract animation
- Alpha-blended onto the ABuffer via lookup table

This needs its own implementation plan, separate from the shroud system. Our current
`apply_gap_generators()` correctly handles the gameplay side (targeting, visibility
suppression) but not the visual side (the dark circle).

### Nice to Have

**4. Overlapping gap generator reference counting**
Multiple gap generators covering the same area should stack properly. Currently we
recompute from scratch each tick, which handles this correctly for gameplay.
