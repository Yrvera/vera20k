# gamemd.exe Draw Order / Depth System

Reverse-engineered via Ghidra MCP (live decompilation of Yuri's Revenge `gamemd.exe`).
Documents the complete rendering pipeline: phase order, display layers, sort keys,
shadow/turret/animation special passes, and elevation handling.

---

## Overview

The engine renders in two major phases within `TacticalClass::Draw` (0x006d3f50):

1. **Phase 1 — Terrain Pass** (param_3 == 1 or 3): 8 sequential steps that draw
   the ground, shadows, overlays, and flat animations. All operate on the isometric
   tile grid in painter's order (back-to-front iso sweep).

2. **Phase 2 — Object Pass** (param_3 == 2 or 3): Draws all game objects via
   `Tactical_ObjectRenderingLoop` (0x006d8d50). Objects are organized into 5 display
   layers. Layer 2 (Ground) is Y-sorted. After all 5 layers, a second pass draws
   "extras" (turret fire arcs, lines, etc.).

Between the terrain and object passes, several overlay functions run (rally point
lines, building placement ghost, etc.).

---

## 1. TacticalClass::Draw — Master Function (0x006d3d10)

**Address:** 0x006d3d10, 447 lines decompiled.
(corrected 2026-05-29: was 0x006d3f50; binary entry point confirmed via get_function_by_address 0x006d3d10 — GHIDRA_ADDRESS_SHIFT)

The function dispatches based on `param_3`:
- `0` — Scroll detection / surface swap only (returns early before drawing)
- `1` — Terrain pass only (Phase 1)
- `2` — Object pass only (Phase 2)
- `3` — Both phases

### Phase 1 Call Order (Terrain Pass)

When `param_3 == 1` or `3`, after viewport setup:

```
Step 1: Tactical_ZBufferDirtyClear()    — 0x006d2b60
Step 2: Tactical_layer_shroud_edges()   — 0x006d3660
Step 3: Tactical_layer_terrain_shadows()— 0x006d2de0
Step 4: Tactical_layer_base_terrain()   — 0x006d3470
Step 5: Tactical_layer_smudges()        — 0x006d3290
Step 6: Tactical_layer_building_overlays() — 0x006d3ac0
Step 7: Tactical_layer_overlays()       — 0x006d3040
Step 8: Tactical_layer_animations()     — 0x006d3870
(corrected 2026-05-29: steps 2-8 addresses all shifted; confirmed via search_functions Tactical_layer* — GHIDRA_ADDRESS_SHIFT)
```

After these 8 steps, the dirty rect list is compacted, and the surface is unlocked.

### Phase 2 Call Order (Object Pass)

When `param_3 == 2` or `3`:

```
FUN_006d9ce0()                    — Build sorted building list for viewport
FUN_006dad60()                    — Draw rally point lines (shrouded)
FUN_006da9d0()                    — Draw rally point lines (visible)
BuildingPlacement_OverlayRenderer() — Building placement ghost
FUN_0053d850()                    — (unknown pre-render step)
Tactical_ObjectRenderingLoop()    — THE MAIN OBJECT RENDERER (0x006d8db0)
(corrected 2026-05-29: was 0x006d8d50; confirmed via search_functions Tactical_ObjectRender* — GHIDRA_ADDRESS_SHIFT)
FUN_005fffa0()                    — Post-object overlay (lines? selection boxes?)
FUN_00550240()                    — (post-render step)
FUN_004c2830()                    — (post-render step)
FUN_00556d40()                    — (post-render step)
FUN_006591b0()                    — (post-render step)
FUN_006dbe20()                    — (post-render step)
FUN_00430ac0()                    — (post-render step)
FUN_006da180()                    — (post-render step)
FUN_006dad60()                    — Rally point lines (second call)
FUN_006da9d0()                    — Rally point lines (second call)
BuildingPlacement_OverlayRenderer()
```

Then: laser/line rendering, TechnoClass health bars and selection circles,
floating text/house messages, and finally pixel FX updates.

**Confidence:** HIGH — all addresses verified from direct decompilation.

---

## 2. The 5 Display Layers

**Layer assignment function:** `DisplayClass::Submit_Object` (0x004a9720)

```c
void Submit_Object(ObjectClass* obj) {
    if (obj->layerIndex != -1)
        Remove_From_Layer(obj);           // 0x004a9770
    int layer = obj->vtable->InWhichLayer(); // vtable+0x78
    if (layer != -1) {
        bool sorted = (layer == 2);       // ONLY layer 2 is Y-sorted on insert!
        DynamicVector_Insert(&g_DisplayLayers[layer], obj, sorted);
        obj->layerIndex = layer;          // stored at object+0x94
    }
}
```

**Layer array:** `g_DisplayLayers` at 0x008a0360, 5 entries of 0x18 (24) bytes each,
spanning 0x8a0360–0x8a03E8. Each entry is a `DynamicVectorClass` (vtable, buffer,
capacity, count, flags, grow_step).

### Layer Enum Values

| Layer | Value | Name | Sorted? | What goes here |
|-------|-------|------|---------|----------------|
| 0 | 0 | Underground | No | Subterranean units (tunnel locomotion) |
| 1 | 1 | Surface | No | Flat ground-level effects |
| 2 | 2 | **Ground** | **Y-sorted** | Buildings, infantry, vehicles, ground anims |
| 3 | 3 | Air | No | Aircraft, airborne projectiles |
| 4 | 4 | Top | No | Top-most effects |

**Critical insight:** Only layer 2 (Ground) uses sorted insertion. The sort comparison
uses `GetYSort` (vtable+0xB8). All other layers are unsorted — objects render in
insertion order.

### InWhichLayer Virtual (vtable+0x78)

This virtual is called by `Submit_Object` to determine which layer an object belongs to.
Different classes override it:

- **BuildingClass:** Returns **2** (Ground layer). Buildings always render in the
  Ground layer where they participate in Y-sorting.
  (Verified: the rendering loop at 0x006d8d50 checks RTTI == 6 (building) within
  the layer 2 iteration and does a turret pass after it.)

- **InfantryClass / UnitClass (ground vehicles):** Return **2** (Ground layer).
  They participate in Y-sorting alongside buildings.

- **AircraftClass:** Returns **3** (Air layer) when airborne. When landed/docked,
  may return 2 (Ground).

- **AnimClass:** Returns the layer based on the animation type's properties:
  - Flat/surface anims (`Layer=ground` in art.ini) → layer 1 (Surface) or layer 2 (Ground)
  - Non-flat anims → layer 2 (Ground) to participate in Y-sorting
  - The terrain pass (Step 8) also draws certain anims via a separate path.

- **BulletClass (projectiles):** Typically layer 3 (Air) for airborne projectiles.

**Confidence:** HIGH for the Submit_Object mechanism and layer 2 sorting.
MEDIUM for specific class return values — derived from rendering loop behavior
rather than reading every vtable entry directly.

---

## 3. Object Sort Within Ground Layer (Layer 2)

### GetYSort (vtable+0xB8)

**Address of ObjectClass::GetYSort:** Found and decompiled.

```c
int ObjectClass__GetYSort(ObjectClass* this) {
    CoordStruct* coords = this->GetRenderCoords();  // vtable+0xAC
    CoordStruct* coords2 = this->GetRenderCoords(); // vtable+0xAC (called twice)
    return coords->X + coords2->Y;                  // Sum of lepton X + Y
}
```

The sort key is **X + Y in lepton coordinates** (game world coordinates, not screen
pixels). This is the isometric depth key — objects further "down-right" in the iso
grid get higher sort values and render later (in front).

### YSort Comparator (ObjectClass::YSortComparator)

```c
bool YSortComparator(ObjectClass* a, ObjectClass* b) {
    return b->GetYSort() < a->GetYSort();
    // Returns true if b should be drawn BEFORE a
    // (lower Y-sort = drawn first = behind)
}
```

**Insertion sort** is used (not qsort). When an object is added to layer 2 via
`FUN_00551a90`, it walks the existing array comparing GetYSort values and inserts
at the correct position, shifting later elements right.

### Secondary Sort Key

**There is NO secondary sort key.** Objects with identical X+Y lepton values have
no defined relative order — they sort by insertion order (which depends on when
`Submit_Object` was called). The Z coordinate (elevation) does NOT participate in
the sort comparison.

**Confidence:** HIGH — the YSortComparator is a trivial 2-line function that only
calls GetYSort on both objects and compares. No tiebreaker logic exists.

### Buildings vs Units Sort

Buildings and units use the **same GetYSort mechanism** — both call vtable+0xAC
(GetRenderCoords) and return X+Y. However, buildings may override GetRenderCoords
to return their foundation center rather than their position origin, which affects
their sort position.

---

## 4. The Object Rendering Loop (0x006d8db0)

**Address:** Tactical_ObjectRenderingLoop, 300 lines decompiled.
(corrected 2026-05-29: was 0x006d8d50; confirmed via search_functions Tactical_ObjectRender* — GHIDRA_ADDRESS_SHIFT)

### First Loop — Draw All Objects (5 layers, 0..4)

```c
int layer = 0;
do {
    for (int i = 0; i < g_DisplayLayers[layer].count; i++) {
        ObjectClass* obj = g_DisplayLayers[layer].buffer[i];
        obj->byte_0x99 = 0;  // clear "was drawn" flag

        if (obj is null pointer) {
            // Non-techno objects: dispatch by RTTI type
            int rtti = obj->WhatAmI();  // vtable+0x2c
            if (rtti == 4) {
                // SPECIAL CASE: type 4 objects use GetCoords (vtable+0x48)
                // and draw with vtable+0x104 (DrawAs) if in viewport
            }
            else if (rtti == 0x24) {
                // type 0x24: check specific flags for visibility
                // draw with vtable+0x104 if visible
            }
            else {
                // Default path: convert coords to screen via CoordsToClient
                // Apply AdjustForZ() for elevation offset
                // Call vtable+0x10C (SetDrawCoords) then vtable+0x104 (DrawAs)
            }
        }
        else if (obj->IsActive flag set) {
            // TECHNO OBJECTS (bit 2 of flags at obj+0x14)
            // Buildings: check RTTI==6, get coords, convert to screen
            //   Apply AdjustForZ for elevation
            //   Call vtable+0x10C then vtable+0x104
            // Other technos: similar path, coords → screen → draw
        }
        else if (obj has IsAlive flag) {
            // Standard alive objects
            // Convert coords to screen, check viewport bounds
            // Call vtable+0x10C (SetDrawCoords)
            // Call vtable+0x110 (DrawShadow) — SHADOW IS DRAWN FIRST
            // Then call vtable+0x104 (DrawAs) with sort flag
        }
    }

    // TURRET PASS — after layer 2 only
    if (layer == 2) {
        for (int i = 0; i < g_BuildingClass_Array_Count; i++) {
            BuildingClass* bld = g_BuildingClass_Array[i];
            if (bld->IsAlive && bld->wasDrawn) {
                // Convert coords, apply AdjustForZ
                BuildingClass__UpdateGarrisonFire(bld);
            }
        }
    }

    layer++;
} while (layer < 5);
```

**Key observations:**
- Shadow (vtable+0x110) is called BEFORE the main draw (vtable+0x104) for standard
  alive objects with the IsAlive flag.
- Building turrets/garrison fire render as a SEPARATE PASS after all layer 2 objects.
- The `wasDrawn` flag (byte at obj+0x99) tracks which objects were actually rendered
  (within viewport bounds), used by the turret pass and second loop.

### Second Loop — Draw Extras (all 5 layers again)

After the first loop completes all 5 layers, there's a second pass:

```c
DisplayLayerEntry* layerPtr = &g_DisplayLayers[0];
do {
    for (int i = 0; i < layerPtr->count; i++) {
        ObjectClass* obj = layerPtr->buffer[i];
        if (obj->wasDrawn && obj != null && obj->IsAlive) {
            // For technos with the "active" bit set:
            //   If shroud/fog conditions met, skip
            //   Otherwise: convert coords to screen
            //   Call vtable+0x110 (DrawExtras) — selection boxes, health bars, etc.
        }
    }
    layerPtr += 6;  // 6 dwords = 0x18 bytes per layer entry
} while (layerPtr < 0x8a03db);
```

This second loop calls vtable+0x110 on objects that were drawn in the first loop.
For technos, this renders the "extras" like the selection circle, health bar,
and waypoint lines. Note that for alive objects in the first loop, vtable+0x110
was called for shadow drawing with different parameters.

**Confidence:** HIGH — directly decompiled from the rendering loop.

---

## 5. Building Turret Draw Order

**After all objects in layer 2 are drawn**, the rendering loop iterates the
global `g_BuildingClass_Array` and calls `BuildingClass__UpdateGarrisonFire`
on buildings that have `IsAlive` and `wasDrawn` flags set.

```c
if (layer == 2) {
    for (int i = 0; i < g_BuildingClass_Array_Count; i++) {
        BuildingClass* bld = g_BuildingClass_Array[i];
        if (bld->field_0x74 != 0 && bld->byte_0x99 != 0) {
            // Convert building coords to screen
            // AdjustForZ for elevation
            // Draw turret/garrison fire on top
        }
    }
}
```

**Key implications:**
- Turrets ALWAYS draw on top of all layer-2 objects (buildings and ground units).
- Turrets from buildings further back in the array may overdraw turrets from buildings
  further forward, but since they're all drawn after the base sprites, they appear
  above their parent buildings and any units in between.
- The sort order within this turret pass is the order of `g_BuildingClass_Array`
  (creation/registration order), NOT Y-sorted.

**Confidence:** HIGH — the `if (local_d4 == 2)` check and the building array
iteration are clearly visible in the decompiled code.

---

## 6. Shadow Rendering

### When Shadows Draw

In the first rendering loop, for objects with the IsAlive flag and IsAlive set:

```c
// Step 1: Set draw coordinates
obj->vtable->SetDrawCoords(&screenPos, &viewport);  // vtable+0x10C

// Step 2: Draw shadow FIRST
obj->vtable->DrawShadow(&screenPos, &viewport);     // vtable+0x110

// Step 3: Draw main sprite
obj->vtable->DrawAs(&viewport, param, sortFlag);     // vtable+0x104
```

**Shadows draw BEFORE the main sprite** for the same object. Since the rendering
loop processes objects in layer order (and Y-sorted within layer 2), a shadow from
object A will be drawn before object A's body, but may be drawn after object B's
body if B has a lower Y-sort value (is further back).

### Shadow Depth

Shadows do not participate in Z-buffer operations. As documented in ZBUFFER_DEPTH_SYSTEM.md,
SHP blitters selected through TechnoClass::DrawSHP (which sets flag 0x800) do NOT
read or write the Z-buffer. Shadows are simply painted onto the surface and can be
overdrawn by any later sprite.

**Terrain shadows** (Step 3 in the terrain pass) are drawn BEFORE the base terrain
tiles (Step 4). This is correct — the shadow shapes are rendered, then the terrain
tiles paint over them, and the per-pixel Z-buffer in the terrain tiles handles
depth correctly.

**Confidence:** HIGH — verified from the rendering loop call order.

---

## 7. Animation Draw Order

### Terrain Pass Animations (Step 8)

The function `Tactical_layer_animations` (0x006d39a0) draws animations that are in
the special "flat display layer" (`DisplayLayerEntry_008a0390`). This is a separate
layer from the 5 main display layers.

```c
// FUN_006d9920 — draws flat animations
for (int i = 0; i < DisplayLayerEntry_008a0390.count; i++) {
    ObjectClass* obj = DisplayLayerEntry_008a0390.buffer[i];
    int rtti = obj->WhatAmI();  // vtable+0x2c
    if (rtti == 6) {  // RTTI 6 = AnimClass type for flat anims
        if (!obj->field_0x6E7 || DAT_00a8ed6b) {
            // Check if anim's screen rect intersects viewport
            // Draw with vtable+0x104
        }
    }
}
```

**These are "flat" animations** — things like scorch marks, fire effects on the ground,
tiberium growth animations. They render during the terrain pass (after overlays,
before the object pass) so they appear beneath all game objects.

### Terrain Pass Building Overlays (Step 6)

`Tactical_layer_building_overlays` (0x006d38a0) also draws from `DisplayLayerEntry_008a0390`,
but filters for RTTI == 0x24 (building-type anims):

```c
// FUN_006d97d0 — draws building overlay anims
for (int i = count-1; i >= 0; i--) {  // REVERSE order!
    ObjectClass* obj = DisplayLayerEntry_008a0390.buffer[i];
    int rtti = obj->WhatAmI();
    if (rtti == 0x24) {
        // Check IsFlat virtual (vtable+0x44)
        // Check type flags for non-standard rendering
        // Draw if intersects viewport
    }
}
```

These are building-attached animations (like the active animations on power plants,
construction yard glow, etc.) that render flat on the terrain during the terrain pass.
Note they iterate in **reverse order**.

### Object Pass Animations

Non-flat animations are placed into the main 5 display layers via `Submit_Object`
and rendered in the main object rendering loop (Phase 2). They participate in Y-sorting
if placed in layer 2 (Ground).

**Confidence:** HIGH — the RTTI checks and layer membership are clearly visible.

---

## 8. The Terrain Pass Sub-Functions in Detail

### Step 1: Tactical_ZBufferDirtyClear (0x006d2b60)

Processes the dirty rect list (`g_DirtyRectList` at `DAT_00b0ce7c`). For each dirty
rect: transforms coordinates, clips to viewport, and calls `ZBuffer_row_fill` to
clear the Z-buffer to 0xFFFF for that region. Also handles per-cell dirty updates.

### Step 2: Tactical_layer_shroud_edges (0x006d3660)

Renders shroud/fog edges for cells that are partially revealed. Uses
`Shroud_fog_edge_rendering` for individual cells, plus `FUN_006d71e0` for
full-rect shroud rendering. Processes dirty rects for incremental updates.

### Step 3: Tactical_layer_terrain_shadows (0x006d2de0)

Renders terrain shadows using `iso_to_screen` to convert iso coordinates and draw
shadow shapes. These are the shadows cast by terrain objects (trees, cliffs).
Processed before base terrain so shadows appear under tiles that write to the Z-buffer.

**Why before terrain?** The terrain tile renderer writes to the Z-buffer per-pixel.
Shadows are flat decals that should appear on the ground. By drawing shadows first,
then drawing terrain tiles over them with Z-test, the shadows correctly appear on
flat ground but get hidden by elevated terrain features.

### Step 4: Tactical_layer_base_terrain (0x006d3470)

Renders the isometric terrain tiles via `FUN_004d1890` (the main tile rendering
function). This is where all TMP tile data is drawn with per-pixel Z-buffer writes.
Iterates cells in iso sweep order (back-to-front). The tile renderer reads the
tile's embedded Z-shape data to write correct depth values.

### Step 5: Tactical_layer_smudges (0x006d3290)

Renders smudge overlays (craters, scorch marks) via `Cell_ContentRendering`.
These are simple flat decals drawn on top of the base terrain.

### Step 6: Tactical_layer_building_overlays (0x006d3ac0)

Renders building-attached flat animations from the special display layer
(`DisplayLayerEntry_008a0390`). Filters for RTTI 0x24 objects with IsFlat.
These are the ground-level visual effects of buildings (like the glow under
a Tesla Coil).

### Step 7: Tactical_layer_overlays (0x006d3040)

Renders wall/fence overlays and ore/tiberium via `FUN_006d7c00`. This function
does an isometric sweep to draw overlays cell-by-cell. Walls interact with the
Z-buffer through the terrain tile system — wall tiles are TMP-format with embedded
Z-data, so they write correct depth values.

### Step 8: Tactical_layer_animations (0x006d3870)

Renders flat animations from the special display layer. Filters for RTTI 6 (anim)
objects. These are ground-level animation effects that should appear above terrain
and overlays but below game objects.

**Confidence:** HIGH — all 8 functions decompiled and analyzed.

---

## 9. Height/Elevation Effects on Draw Order

### AdjustForZ Function

```c
void AdjustForZ(int z_coord) {
    Math__ftol(z_coord > 0x2D7, z_coord);
    return;  // Returns the height-adjusted Y offset
}
```

When objects are at non-zero elevation (Z coordinate), the rendering loop calls
`AdjustForZ` to compute a Y-offset that shifts the sprite upward on screen. This
is applied to the screen Y coordinate:

```c
screen_y = iso_y - AdjustForZ(z_coord);
```

### Height Does NOT Affect Sort Order

**Critical finding:** Elevation (the Z lepton coordinate) does NOT participate in
the Y-sort comparison. GetYSort returns `X + Y` only. Two objects at different
heights but the same X+Y will be drawn in insertion order.

However, elevation DOES affect the **screen position** — elevated objects are drawn
higher on screen. This means an elevated object and a ground-level object at the
same iso position will overlap vertically, with the rendering order determined
purely by X+Y sort key.

For the Z-buffer system (which only affects terrain tiles), elevation is factored
into the base Z computation via `heightLevel`, as documented in ZBUFFER_DEPTH_SYSTEM.md.

**Confidence:** HIGH — GetYSort verified to use only X+Y; AdjustForZ verified
to only affect screen position.

---

## 10. Bridge Draw Order

Bridge rendering is handled at the cell/terrain level, not the object level.
Bridge tiles are rendered during Step 4 (base terrain) and Step 7 (overlays),
with their per-pixel Z-data from the TMP tiles controlling depth.

### Units On vs Under Bridges

Units on a bridge have a higher Z coordinate (elevation) than units under a bridge.
Since GetYSort uses X+Y (not Z), **two units at the same X,Y but different Z
will have the same sort key**.

The Z-buffer handles the correct occlusion:
- Bridge tiles write their Z-values during the terrain pass.
- A unit under the bridge draws at ground-level screen Y — the bridge tile Z-values
  prevent it from overwriting bridge pixels (since terrain Z-test uses `<=`).
- A unit on the bridge draws at elevated screen Y (shifted up by AdjustForZ).

For unit-to-unit depth on/under bridges, the sort order is the same (same X+Y),
so the engine relies on the Z-buffer written by bridge terrain tiles to handle
the visual overlap correctly.

**However**, as documented in ZBUFFER_DEPTH_SYSTEM.md, SHP sprites do NOT read
the Z-buffer (flag 0x800 disables Z-test). This means unit sprites on and under
bridges may not occlude correctly with each other — the last-drawn unit wins.
The terrain tiles (which DO use Z-buffer) provide the main visual separation.

**Confidence:** MEDIUM — bridge-specific rendering not directly decompiled;
inferred from the general terrain Z-buffer system and GetYSort analysis.

---

## Summary: Complete Draw Order

1. **Z-buffer clear** (per dirty rect)
2. **Shroud edges**
3. **Terrain shadows** (flat decals, drawn first)
4. **Base terrain tiles** (with per-pixel Z-buffer write)
5. **Smudges** (craters, scorch marks)
6. **Building flat animations** (ground glow effects)
7. **Overlays** (walls, fences, ore/tiberium — with Z-buffer)
8. **Flat animations** (ground-level anim effects)
9. Rally point lines, building placement ghost
10. **Layer 0 (Underground)** — unsorted
11. **Layer 1 (Surface)** — unsorted
12. **Layer 2 (Ground)** — **Y-sorted by virtual `GetYSort`** (back-to-front):
    base objects use `X+Y`; `AnimClass` adds `YSortAdjust`; `BuildingClass`
    applies conditional `+32` / `-16` type-flag deltas.
    - For each object: shadow drawn first, then main sprite
    - **After all layer 2 objects:** building turret/garrison fire pass
13. **Layer 3 (Air)** — unsorted
14. **Layer 4 (Top)** — unsorted
15. **Second pass all layers:** selection circles, health bars, waypoint lines
16. Laser/line rendering, floating text, house messages
17. Pixel FX updates

### Key Addresses

| Function | Address | Purpose |
|----------|---------|---------|
| TacticalClass::Draw | 0x006d3d10 | Master render function |
| Tactical_ObjectRenderingLoop | 0x006d8db0 | Main object renderer |
| DisplayClass::Submit_Object | 0x004a9720 | Assigns objects to layers |
| DisplayClass::Remove_From_Layer | 0x004a9770 | Removes from layer |
| ObjectClass::GetYSort | (found, decompiled) | Returns X + Y of render coords |
| ObjectClass::YSortComparator | (found, decompiled) | Compares GetYSort values |
| Sorted insertion (layer 2) | 0x00551a90 | Binary-walk insert by YSort |
| Unsorted append (other layers) | 0x005519b0 | Simple append to end |
| AdjustForZ | (found, decompiled) | Height to screen-Y offset |
| BuildingClass::UpdateGarrisonFire | (found, decompiled) | Turret/garrison overlay |
| Terrain shadow step | 0x006d2de0 | iso_to_screen for shadow tiles |
| Base terrain step | 0x006d3470 | Main tile renderer dispatch |
| Overlay step | 0x006d3040 | Walls/ore cell renderer |
| Flat anim step (terrain) | 0x006d3870 | Ground anims in terrain pass |
| Building overlay step | 0x006d3ac0 | Building flat anims |
| FUN_006d9ce0 | 0x006d9ce0 | Pre-render: builds visible building list |
| FUN_006d1eb0 | 0x006d1eb0 | Lepton coords to screen pixel offset |
