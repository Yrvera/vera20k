# AlphaShapeClass Lifecycle — Ghidra Research Report

**Date:** 2026-03-23
**Confidence:** HIGH — all findings verified from binary decompilation via Ghidra MCP.

## Overview

AlphaShapeClass is the system that renders "fog ghost" silhouettes — the semi-transparent
shapes that appear where units/buildings were last seen before entering fog of war. When an
object enters fog, an AlphaShapeClass is created to store its visual footprint. When the
cell is re-revealed, the alpha shape is marked for deletion and cleaned up.

## Class Layout (0x40 bytes total)

Allocation: `operator_new(0x40)` in ObjectClass::Unlimbo (0x005f4ec0).

```
Offset  Size  Field               Description
------  ----  -----               -----------
0x00    4     vtable_primary      -> 0x007e32a4 (AlphaShapeClass vtable)
0x04    4     vtable_secondary_4  -> 0x007e3288 (IPersistStream adjustor)
0x08    4     vtable_secondary_8  -> 0x007e3280 (IRTTITypeInfo adjustor)
0x0C    4     vtable_secondary_C  -> 0x007e3278 (INoticeSink adjustor)
0x10    4     unique_id           AbstractClass::UniqueID (0xFFFFFFFF)
0x14    4     flags               AbstractClass bit flags
0x18    4     field_18            AbstractClass (dirty state)
0x1C    4     field_1C            AbstractClass (zero)
0x20    4     field_20            AbstractClass (zero)
0x24    4     source_object_ptr   Pointer to the ObjectClass that created this shape
0x28    4     screen_x            Screen X position (absolute, not viewport-relative)
0x2C    4     screen_y            Screen Y position (absolute, not viewport-relative)
0x30    4     width               SHP frame width (from AlphaImage SHP header +0x02)
0x34    4     height              SHP frame height (from AlphaImage SHP header +0x04)
0x38    4     shp_data_ptr        Pointer to the SHP file (ObjectTypeClass +0xAC = AlphaImage)
0x3C    1     disabled            0 = active, 1 = marked for removal (set by Notification)
0x3D-3F 3     padding             (constructor zeros full dword at 0xF via byte write)
```

**Inheritance chain:** AbstractClass -> AlphaShapeClass (no intermediate base).

## Global Data Structures

### Primary Alpha Shapes Array (DynamicVectorClass<AlphaShapeClass*>)

```
Address       Name                  Description
----------    ----                  -----------
0x0088a0f0    vtable_ptr            DynamicVectorClass vtable (-> 0x007e3258 or 0x007e3238)
0x0088a0f4    buffer_ptr            Pointer to heap-allocated array of AlphaShapeClass*
0x0088a0f8    capacity              Current allocated capacity
0x0088a0fd    can_grow              byte: 1 = vector can reallocate
0x0088a100    count                 Number of active alpha shapes
0x0088a104    grow_amount           Number of slots to add when growing (default: 10)
```

### Secondary Tracking Array (at 0x00b0f720)

A second DynamicVectorClass also tracks AlphaShapeClass pointers. This array is shared
with other AbstractClass-derived objects (ObjectClass, ParticleSystemClass, TeamClass,
FactoryClass, HouseClass all register into it). Its purpose is likely the master object
tracking list for save/load. The destructor removes from both arrays.

### Alpha Blending LUT (DAT_0088a118)

A 64KB (256x256) lookup table initialized once on first AlphaShapeClass creation.
Flag at `DAT_0089a134` tracks whether it has been initialized.

**Formula:**
```
for index in 0..65536:
    existing_alpha = index & 0xFF       // low byte: current ABuffer pixel value
    shape_alpha    = (index >> 8) & 0xFF // high byte: SHP frame pixel value
    result = clamp(0, 255, (shape_alpha * existing_alpha) / 127)
    LUT[index] = result
```

This computes a pre-multiplied alpha blend: the shape pixel modulates the existing
fog alpha value. The /127 (not /255) means the blending is scaled to treat 127 as
fully opaque, giving a stronger darkening effect.

### Diamond Mask (DAT_007e2b20)

A 60x30 byte array in .rdata that stores an isometric diamond shape. Each byte is either:
- `0x20` (space = 32) = transparent, skip this pixel
- `0xDB` (block = 219) = opaque, apply alpha blending

This mask ensures the fog ghost silhouettes are clipped to isometric cell boundaries.
The mask is only used by `DrawAll_WithMask` (0x00420f40), not by `DrawAll_NoMask`
(0x00421350).

## Lifecycle

### 1. Creation — ObjectClass::Unlimbo (0x005f4ec0)

When an object is placed on the map (Unlimbo), the function checks:

1. Was the object previously discovered by the player? (`this+0x81 != 0`)
2. Is the object now being re-discovered (not dead)? (`this+0x74 == 0`)
3. Does the ObjectTypeClass have an AlphaImage SHP? (`TypeClass+0xAC != 0`)
4. Does the game mode allow it? (not observer, etc.)

If all conditions are met:

```c
// Get object center in screen coords
coords = this->vt->GetCoords();
TacticalClass__CoordsToClient2(&coords, &screen_pos);

// Get SHP dimensions
type_class = this->vt->GetType();
shp = type_class->AlphaImage;  // offset +0xAC
screen_y = (client_y + Tactical->scroll_y) - shp->height / 2;
screen_x = client_x - shp->width / 2;

// Allocate and construct
mem = operator_new(0x40);
if (mem) {
    AlphaShapeClass__Constructor(mem, this, screen_x, screen_y);
}

// Dirty the screen area
TacticalClass__DirtyScreenRect(...);
```

The constructor (0x00420960):
1. Calls AbstractClass/INoticeSink base constructor
2. Stores the source object pointer at +0x24
3. Gets the AlphaImage SHP via source->vt->GetType()->AlphaImage
4. Stores screen position (+0x28, +0x2C) and SHP dimensions (+0x30, +0x34)
5. Stores SHP pointer at +0x38
6. Sets disabled flag to 0
7. Appends `this` to the global alpha shapes array (DAT_0088a0f4/DAT_0088a100)
8. Appends `this` to the secondary tracking array
9. Initializes the 64KB alpha blending LUT if not already done

### 2. Rendering — Two Draw Functions

Both iterate the global alpha shapes array and composite each shape onto the ABuffer
(the alpha/fog buffer, which is a 16-bit circular buffer).

#### DrawAll_WithMask (0x00420f40) — Called from shroud edge rendering

Called from `Tactical_layer_shroud_edges` (0x006d3660) during per-cell shroud rendering.

For each alpha shape where `disabled == 0`:
1. Converts absolute screen position to viewport-relative
2. Calls `ClipRect` to intersect with the viewport bounds
3. Gets the SHP frame rect and pixel data (frame 0)
4. Clips the shape rect against the cell's SHP bounds
5. For each pixel in the clipped region:
   - Reads the diamond mask byte from DAT_007e2b20
   - If mask byte != 0x20 (space): applies the LUT blend
   - `ABuffer[x,y] = LUT[ABuffer[x,y] | (shp_pixel << 8)]`
6. Handles circular buffer wraparound for the ABuffer

#### DrawAll_NoMask (0x00421350) — Called from dirty rect rendering

Called from `FUN_006d71e0` (0x006d71e0) during dirty rect repainting.

Same logic as DrawAll_WithMask but does NOT check the diamond mask — every pixel in
the clipped SHP frame is blended unconditionally. This is used for rectangular region
updates where the diamond clipping is not needed (the caller already handles cell
boundary clipping).

Both functions access the SHP data lazily: if `+0x38` (shp_data_ptr) is null, they
re-fetch it from `source_object_ptr->vt->GetType()->AlphaImage` (offset +0xAC).

### 3. Notification — Marking for Removal (0x00420e70)

```c
void AlphaShapeClass::Notification(AbstractClass* source) {
    if (source == this->source_object_ptr) {
        this->disabled = 1;
    }
}
```

This is vtable slot [10] at offset +0x28 in the primary vtable. It is called via the
AbstractClass notification system when the source object changes state (e.g., when the
object is re-revealed from fog, destroyed, or otherwise invalidated).

The notification chain:
1. `MapClass__UpdateFogOfWarCell` (0x004a9dd0) detects fog state change
2. Calls `CellChangeNotify` (0x005865f0) for the affected cell
3. `CellChangeNotify` finds objects in the cell, calls their `vt->NotifySources`
4. The source object notifies all registered sinks, which reaches
   `AlphaShapeClass::Notification`
5. If the notification source matches the stored object pointer, sets `disabled = 1`

### 4. Cleanup — PurgeDisabled (0x00420e90)

Called once per game tick from the main simulation loop (`FUN_0055afb0` at 0x0055afb0,
which is called from `Main_Tick`).

```c
void AlphaShapeClass::PurgeDisabled() {
    // Ensure LUT is initialized (same code as constructor)
    InitLUT();

    // Iterate backwards (safe for removal during iteration)
    for (int i = alpha_shapes_count - 1; i >= 0; i--) {
        AlphaShapeClass* shape = alpha_shapes_buf[i];
        if (shape->disabled != 0 && shape != NULL) {
            shape->vt->ScalarDeletingDestructor(1);  // vtable[8], param=1 means free memory
        }
    }
}
```

The backward iteration is critical — destroying a shape modifies the array (shifts
elements down), so iterating forward would skip entries.

### 5. Destruction — Destructor (0x00421730)

The scalar deleting destructor:

```c
AlphaShapeClass* AlphaShapeClass::Destructor(int free_flag) {
    // Reset vtable pointers (prevents double dispatch during teardown)
    this->vtable_0 = &AlphaShapeClass_vtable;
    this->vtable_4 = &AlphaShapeClass_secondary_4;
    this->vtable_8 = &AlphaShapeClass_secondary_8;
    this->vtable_C = &AlphaShapeClass_secondary_C;

    // Remove from primary alpha shapes array
    int idx = alpha_shapes_vector.FindIndex(this);
    if (idx != -1 && idx < alpha_shapes_count) {
        alpha_shapes_count--;
        // Shift remaining elements down
        for (int i = idx; i < alpha_shapes_count; i++) {
            alpha_shapes_buf[i] = alpha_shapes_buf[i + 1];
        }
    }

    // Remove from secondary tracking array
    idx = secondary_vector.FindIndex(this);
    if (idx != -1 && idx < secondary_count) {
        secondary_count--;
        for (int i = idx; i < secondary_count; i++) {
            secondary_buf[i] = secondary_buf[i + 1];
        }
    }

    // Call AbstractClass destructor
    AbstractClass::Destructor();

    // Free memory if flag bit 0 is set
    if (free_flag & 1) {
        operator_delete(this);
    }
    return this;
}
```

### 6. Global Cleanup — Game End (0x00534450)

During game shutdown (`FUN_00534450`), all alpha shapes are destroyed:

```c
while (alpha_shapes_count != 0) {
    if (*alpha_shapes_buf != NULL) {
        (*alpha_shapes_buf)->vt->ScalarDeletingDestructor(1);
    }
}
```

### 7. Array Init/Clear

**InitGlobalArray (0x004208e0):**
Called once at startup. Initializes the DynamicVectorClass with:
- buffer = NULL (grows on first insertion)
- capacity = 0, count = 0
- can_grow = 1
- grow_amount = 10
- vtable = 0x007e3238

**ClearGlobalArray (0x00420920):**
Called during cleanup. Frees the buffer if can_grow was set, resets all fields to zero.

## Primary Vtable Layout (0x007e32a4)

```
Slot  Offset  Address     Name
----  ------  -------     ----
[0]   0x00    0x00410260  QueryInterface (IUnknown)
[1]   0x04    0x00410300  AddRef (IUnknown)
[2]   0x08    0x00410310  Release (IUnknown)
[3]   0x0C    0x00420d40  GetClassID (IPersist)
[4]   0x10    0x00410450  IsDirty (IPersistStream)
[5]   0x14    0x00420de0  Load (IPersistStream)
[6]   0x18    0x00420e40  Save (IPersistStream)
[7]   0x1C    0x004103e0  GetSizeMax (IPersistStream)
[8]   0x20    0x00421730  ScalarDeletingDestructor
[9]   0x24    0x00410470  Init (no-op)
[10]  0x28    0x00420e70  Notification (INoticeSink dispatch)
```

## Save/Load (IPersistStream)

**Load (0x00420de0):**
1. Calls base class Load
2. Reinitializes vtable pointers
3. Reads fields starting at dword[9] from the save stream (via `FUN_006cf240`)
4. Clears shp_data_ptr to 0 (will be re-fetched lazily from source object)

**Save (0x00420e40):**
Delegates to base class Save. The actual field serialization is handled by the
base class writing the memory block.

## Complete Function Index

| Address    | Name                              | Purpose                                    |
|------------|-----------------------------------|--------------------------------------------|
| 0x004208e0 | AlphaShapeClass__InitGlobalArray  | Initialize global DynamicVector at startup |
| 0x00420920 | AlphaShapeClass__ClearGlobalArray | Free/reset global DynamicVector            |
| 0x00420960 | AlphaShapeClass__Constructor      | Main constructor (object, x, y)            |
| 0x00420af0 | AlphaShapeClass__Constructor      | Default constructor (for deserialization)   |
| 0x00420d40 | AlphaShapeClass__GetClassID       | IPersist::GetClassID                       |
| 0x00420de0 | AlphaShapeClass__Load             | IPersistStream::Load                       |
| 0x00420e40 | AlphaShapeClass__Save             | IPersistStream::Save                       |
| 0x00420e70 | AlphaShapeClass__Notification     | Mark disabled when source object notifies  |
| 0x00420e90 | AlphaShapeClass__PurgeDisabled    | Destroy all shapes with disabled=1         |
| 0x00420f40 | AlphaShapeClass__DrawAll_WithMask | Render all shapes with diamond cell mask   |
| 0x00421350 | AlphaShapeClass__DrawAll_NoMask   | Render all shapes without cell mask        |
| 0x00421730 | AlphaShapeClass__Destructor       | Scalar deleting destructor                 |
| 0x00421b60 | AlphaShapeClass__ClipRect         | Rectangle intersection for clipping        |

## Rendering Pipeline Integration

```
Tactical_layer_shroud_edges (0x006d3660)
  Per shroud-edge cell:
    1. Shroud_fog_edge_rendering (edge tile compositing)
    2. AlphaShapeClass__DrawAll_WithMask (per-cell fog ghosts with diamond mask)
  Then for dirty rects:
    3. FUN_006d71e0 -> AlphaShapeClass__DrawAll_NoMask (rectangular fog ghosts)
```

## Key Implementation Notes

1. **Screen coordinates are absolute** (include scroll offset). The draw functions
   subtract `Tactical->scroll_x/y` and add `RadarViewportOffset` to convert to
   viewport-relative for the ABuffer.

2. **The diamond mask is critical** for per-cell rendering. Without it, ghost shapes
   would bleed across cell boundaries during the per-cell shroud pass.

3. **The LUT divides by 127, not 255.** This means shape pixels with value 127
   produce full modulation. Values above 127 actually amplify the alpha, though
   this is clamped to 255.

4. **SHP data is fetched lazily.** After a save/load cycle, the shp_data_ptr is
   cleared and re-resolved from `source_object->GetType()->AlphaImage` on next draw.

5. **The secondary tracking array at 0x00b0f720** is NOT AlphaShape-specific. It is
   a general purpose tracking vector used by multiple class types for save/load
   infrastructure.

6. **Cleanup happens per-tick, not immediately.** When a notification marks a shape
   as disabled, it is not destroyed until the next call to PurgeDisabled at the end
   of the game tick. This avoids iterator invalidation during notification dispatch.
