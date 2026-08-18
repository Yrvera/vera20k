# Shroud System — Distilled Algorithmic Reference

How gamemd.exe implements shroud, and how to reproduce it exactly.

**Scope:** Standard YR (FogOfWar=false, ShroudGrow=false). Fog-of-war is TS legacy
and OFF by default — not covered here.

---

## 1. Data Model

### Per-cell state (2 bits needed)

| State | Meaning | Visual |
|-------|---------|--------|
| Unexplored | Never seen by any allied unit | Pitch black |
| Explored + no shrouded neighbors | Seen at least once, all neighbors also explored | Normal rendering, no overlay |
| Explored + some shrouded neighbors | On the boundary | Transition edge (partial darkening via SHROUD.SHP) |

In gamemd.exe, `CellClass+0x12C` bit 3 (0x08) = explored, bit 4 (0x10) = fully revealed.
Both bits are set together and are **permanent** (never cleared in standard YR except by
Gap Generator).

**Our model:** `FLAG_REVEALED` (0x01) per cell per owner. Once set, never cleared.
Equivalent to gamemd's bit 3. We don't need bit 4 separately — the edge bitmask check
handles it.

### Per-cell gap generator state

`CellClass+0x130` = reference count. When positive, the cell is re-shrouded for enemies.
When it drops to 0, the cell returns to explored. This is the **only** mechanism that can
un-reveal cells in standard YR.

---

## 2. Reveal Algorithm

### When to reveal

Every unit/building calls reveal each tick from its position. Buildings reveal from their
**center point only** (foundation size irrelevant). The engine also runs a "paranoid"
full-pass periodically to ensure no gaps.

### Effective sight range

```
base_sight = unit_type.Sight            (from rules.ini, integer 0-12)
vet_bonus  = if veterancy >= 1.0: rules.VeteranSight else 0
elev_bonus = (unit_z * 256) / rules.LeptonsPerSightIncrease
effective  = clamp(base + vet + elev, 0, 10)
```

**Hard cap: 10 cells.** The original crashes above 10. We clamp.

### Spiral table iteration

The reveal area is defined by a pre-built table of (dx, dy) offsets sorted by distance,
forming concentric rings:

```
Sight 0 →   1 cell   (center only)
Sight 1 →   9 cells  (+8 ring)
Sight 2 →  21 cells  (+12)
Sight 3 →  37 cells  (+16)
Sight 4 →  61 cells  (+24)
Sight 5 →  89 cells  (+28)
Sight 6 → 121 cells  (+32)
Sight 7 → 161 cells  (+40)
Sight 8 → 205 cells  (+44)
Sight 9 → 253 cells  (+48)
Sight 10→ ~310 cells  (+~57, uses coordinate conversion)
```

For sight N, iterate spiral entries `[0..RING_SIZES[N]]`. Each entry gives a cell offset
from the unit. Mark each cell as revealed if it passes bounds + distance + LOS checks.

The full 253-entry spiral table is extracted from gamemd.exe address 0x00ABD490 and
embedded verbatim in our code.

### Per-cell checks (for each spiral entry)

```
1. Bounds check: cell (cx+dx, cy+dy) must be inside the map
2. Euclidean distance: sqrt(dx² + dy²) ≤ sight_range
   (gamemd uses a fast 8192-entry sqrt LUT; we use integer squared comparison)
3. Height-based LOS (if RevealByHeight=true, which is the default):
   - Look up the "midpoint cell" from a parallel mirror table
   - If viewer_level + 3 < midpoint_cell.height → LOS blocked, skip
   - This means terrain 4+ levels above the viewer at the halfway point blocks sight
4. If all pass: mark cell as explored/revealed
```

### Alliance-aware reveal

If `AllyReveal=true` (default), allied players share vision. Our merged grid handles
this — we OR all allied owners' visibility into one grid once per tick.

---

## 3. Edge Frame Selection Algorithm

### The core question

For each **explored** cell near the shroud boundary: which SHROUD.SHP transition frame
to draw?

### Algorithm (Shroud_EdgeBitmask_Calculator at 0x006d8700)

```
function select_shroud_frame(cell) -> frame_index_or_special:
    if cell is NOT explored:
        return FULL_BLACK  // draw full opaque diamond (frame 15)

    // Cell IS explored. Check 8 neighbors.
    mask = 0 (8 bits)
    for each of 8 neighbors:
        if neighbor is unexplored (or out-of-bounds):
            set the corresponding bit

    // Bit layout:
    //   NW  N  NE         bit6  bit7  bit0
    //    W  *   E    →    bit5    *   bit1
    //   SW  S  SE         bit4  bit3  bit2

    frame = SHROUD_EDGE_LUT[mask]   // 256-byte lookup table

    if frame == 0xFF: return NO_EDGE    // no shrouded neighbors
    if frame == 0xFE: return FULL_BLACK // surrounded by shroud
    return frame  // index 0-46 into SHROUD.SHP
```

**Key insight:** Edges are drawn on the **EXPLORED** side of the boundary. The function
only runs for explored cells and checks which of their neighbors are still dark.

### The 256-byte lookup table

Extracted from gamemd.exe at 0x007f4194. Maps every possible 8-neighbor combination to
one of 47 SHROUD.SHP frames (or 0xFF/0xFE for no-edge/full-black). Many combinations
map to the same frame because corner bits (NW, NE, SW, SE) matter less than cardinal
bits (N, S, E, W) for selecting the edge shape.

Embedded in our code as `SHROUD_EDGE_LUT` in `shroud_buffer.rs`.

---

## 4. SHROUD.SHP Frame Data

### Frame layout

- **Frame 0:** Empty (0x0 pixels) — used when LUT returns "no edge needed"
- **Frames 1-46:** 47 edge transition shapes on a 60x30 isometric canvas
- **Frame 15:** Full 60x30 diamond filled with 0x00 — used for fully shrouded cells

### Pixel value semantics

Each pixel is a **direct brightness value**, not a palette index:

| Pixel | Effect |
|-------|--------|
| 0x00 | Full black (complete shroud) |
| 0x01-0x7E | Gradient (smooth transition at edges) |
| 0x7F | Full brightness (no darkening at all) |
| 0xFE | Transparent (skip — don't overwrite buffer) |

These are written directly into the ABuffer. No lookup, no blending — just overwrite.

---

## 5. Rendering Pipeline

### gamemd.exe approach (per-pixel ABuffer)

```
Per frame:
  1. Fill entire ABuffer with 0x7F (neutral/bright)
  2. For each cell in viewport:
     a. If unexplored: blit frame 15 (full black diamond) to ABuffer
     b. If explored with shrouded neighbors: blit edge frame to ABuffer
     c. If fully explored: do nothing (stays 0x7F)
  3. All tile/sprite blitters READ ABuffer per-pixel:
     final_color = color_table[brightness_remap[abuffer_value] | palette_index]
     - abuffer=0x00 → black pixel
     - abuffer=0x7F → normal color
```

### Our GPU approach (equivalent result)

```
Per frame:
  1. Fill CPU pixel buffer with 0x7F (neutral)
  2. For each cell in viewport:
     a. If unexplored: fill diamond area with 0x00 (black)
     b. If explored + has edge: blit SHROUD.SHP frame pixels (with transparency skip)
     c. If fully explored: skip (stays 0x7F)
  3. Upload R8 buffer to GPU texture
  4. Full-screen multiply pass: scene_color *= (shroud_value / 0x7F)
     - 0x00 → multiply by 0 → black
     - 0x7F → multiply by 1 → unchanged
     - Intermediate values → smooth gradients on edges
```

The visual result is identical. Our approach replaces per-pixel reads in each blitter
with a single post-process multiply pass.

---

## 6. Gap Generator (Re-shrouding)

### Algorithm

```
For each gap generator building (GapGenerator=yes in rules.ini):
  radius = GapRadiusInCells  (from rules.ini)
  For each cell within radius (Euclidean distance check):
    For each enemy owner:
      Clear their VISIBLE flag (turn Visible → Revealed-only)
    Increment cell's gap_counter

When gap generator is destroyed/unpowered:
  For each cell it was covering:
    Decrement gap_counter
    If gap_counter reaches 0: restore explored state
```

### Our implementation

We suppress enemy vision by clearing `FLAG_VISIBLE` in the gap radius for non-friendly
owners. This is applied AFTER SpySat (so gap wins over spy sat in contested areas).

---

## 7. Object Visibility

### How shroud hides objects

```
When a cell is unexplored:
  All objects in that cell are hidden (IsUndiscovered = true)
  They are removed from the rendering layer entirely

When the cell is revealed:
  CellChangeNotify iterates objects in the cell
  Each object's Discover() is called → IsUndiscovered = false
  Object is submitted to the display layer and appears

In standard YR (no fog): once discovered, objects stay visible forever
Only gap generators can re-hide objects (sets IsUndiscovered = true again)
```

---

## 8. INI Settings That Matter

### Active in standard YR

| Key | Section | Default | Effect |
|-----|---------|---------|--------|
| Sight | per unit | varies | Vision range in cells (0-12, capped to 10) |
| RevealByHeight | [General] | true | Height-based LOS obstruction |
| AllyReveal | [General] | true | Allied players share vision |
| LeptonsPerSightIncrease | [General] | 2000 | Elevation sight bonus divisor |
| VeteranSight | [General] | 0.0 | Veteran sight multiplier |
| Shroud | [MultiplayerDialogSettings] | yes | Map starts with shroud |
| GapRadiusInCells | per building | varies | Gap generator coverage radius |
| GapGenerator | per building | no | Building acts as gap generator |
| BlendedFog | [AudioVisual] | true | Smooth vs dithered edges |

### Dormant (NOT active by default)

| Key | Default | Why dormant |
|-----|---------|-------------|
| FogOfWar | false | TS legacy — OFF in YR |
| ShroudGrow | false | Shroud doesn't regrow |
| ShroudRate | 4 min | Only if ShroudGrow=true |

---

## 9. Implementation Status (Our Engine)

### Complete

- Per-owner visibility grids with `FLAG_REVEALED`/`FLAG_VISIBLE`
- Reveal spiral table (253 entries, exact match to gamemd 0x00ABD490)
- Sight capping at MAX_SIGHT_RANGE=10
- Veteran + elevation sight bonuses
- Alliance-aware merged visibility grid (O(1) lookups)
- 8-bit neighbor edge bitmask (exact bit layout match)
- 256-byte SHROUD_EDGE_LUT (exact match to gamemd 0x007f4194)
- SHROUD.SHP frame extraction and brightness blitting
- GPU ABuffer (R8 texture + multiplicative blend pass)
- SpySat full-map reveal
- Gap Generator enemy vision suppression
- Change detection (fog generation counter + camera position)
- 20+ unit tests covering all visibility mechanics

### Not yet implemented / needs verification

- **Height-based LOS (RevealByHeight):** ✅ IMPLEMENTED (2026-06-11, `a29b7886`). The mirror table from
  `0x00abcf60` (built by `InitRevealMirrorTable @ 0x00563908`) was extracted and verified **253/253** against
  Rust `REVEAL_MIRROR`. `reveal_radius_into()` does the per-cell LoS check: obstruction = `target + mirror[i] +
  (2,2)`, block when `obs_level > viewer_level + 3` (signed). The `+2` per-axis offset was the one fix needed;
  verified instruction-level at `MapClass::RevealShroud 0x005673a0`.
- **Object discovery/concealment lifecycle:** Objects aren't yet hidden/shown based on
  shroud state. Need `IsUndiscovered` flag and `Discover`/`Conceal` calls on
  CellChangeNotify when shroud state changes.
- **Paranoid reveal pass:** The periodic full-iteration re-reveal of all player technos
  is not explicitly implemented (our per-tick recompute achieves the same result).
- **Gap Generator cell-level reference counting:** Our implementation suppresses vision
  directly; the original uses per-cell reference counts for overlapping gap coverage.
- **Shroud=no game setting:** Starting a game with shroud disabled (all cells pre-revealed)
  is not wired up yet.
- **BlendedFog rendering:** Smooth vs dithered edge modes not differentiated.
- **Radar/minimap shroud rendering:** Minimap doesn't show shroud overlay yet.

---

## 10. Algorithmic Pseudocode Summary

### Per tick (simulation)

```python
def refresh_fog(entities, alliances, rules):
    # 1. Clear VISIBLE flags (preserve REVEALED)
    for each owner_grid:
        clear_all_visible()

    # 2. Reveal from each unit
    for each entity not inside transport:
        sight = compute_effective_sight(entity, rules)
        for (dx, dy) in SPIRAL_TABLE[0..RING_SIZES[sight]]:
            cell = (entity.x + dx, entity.y + dy)
            if out_of_bounds(cell): continue
            if sqrt(dx²+dy²) > sight: continue
            if RevealByHeight and terrain_blocks_los(entity, cell): continue
            owner_grid[cell] |= REVEALED | VISIBLE

    # 3. SpySat: full-map reveal for powered SpySat owners
    for each spy_sat_owner:
        fill owner_grid with REVEALED | VISIBLE

    # 4. Gap Generators: suppress enemy vision
    for each gap_gen:
        for cell in radius:
            for each enemy owner:
                enemy_grid[cell] &= ~VISIBLE

    # 5. Build merged grid for local player + allies
    merged = OR all allied grids together
    generation++
```

### Per frame (rendering)

```python
def render_shroud(fog, merged, camera, screen):
    # 1. Fill buffer with 0x7F (neutral)
    buffer.fill(NEUTRAL)

    # 2. For each cell in viewport
    for (rx, ry) in visible_cells:
        screen_pos = iso_to_screen(rx, ry, z=0) - camera
        if off_screen(screen_pos): continue

        if not merged.is_revealed(rx, ry):
            # Unexplored: full black diamond
            fill_diamond(screen_pos, BLACK)
            continue

        # Explored: check neighbors for edge
        mask = 0
        for each of 8 neighbors (NE=bit0, E=bit1, SE=bit2, S=bit3,
                                   SW=bit4, W=bit5, NW=bit6, N=bit7):
            if neighbor is unexplored: set bit

        frame = SHROUD_EDGE_LUT[mask]
        if frame == 0xFF: continue        # no edge needed
        if frame == 0xFE:                  # fully surrounded
            fill_diamond(screen_pos, BLACK)
            continue
        blit_shroud_frame(frame, screen_pos)  # transition edge

    # 3. Upload to GPU and multiply-blend over the scene
    upload(buffer)
    draw_fullscreen_quad(multiply_blend)
```

---

## 11. Key Data Tables (Embedded in Code)

| Table | Source | Size | Location in our code |
|-------|--------|------|---------------------|
| Reveal spiral (dx,dy) | 0x00ABD490 | 253 entries | `REVEAL_SPIRAL` in vision/mod.rs |
| Ring sizes (cumulative) | 0x007ED3D0 | 11 entries | `REVEAL_RING_SIZES` in vision/mod.rs |
| Edge frame LUT | 0x007F4194 | 256 bytes | `SHROUD_EDGE_LUT` in shroud_buffer.rs |
| Mirror/midpoint table | 0x00ABCF60 | 253 entries (309 w/ sight-10) | `REVEAL_MIRROR` in vision/mod.rs — **extracted + verified 253/253** vs `InitRevealMirrorTable 0x00563908` (2026-06-11) |
| Fast sqrt LUT | 0x008650BC | 8192 entries | Not needed (we use integer math) |
