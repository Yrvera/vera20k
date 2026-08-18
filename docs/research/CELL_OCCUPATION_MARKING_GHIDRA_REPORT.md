# Cell Occupation Marking During Vehicle Movement -- Ghidra Report

> **Verified 2026-03-23** against live Ghidra MCP decompilation of `gamemd.exe`.
> All addresses, offsets, flag bits, and algorithms confirmed from live decompilation.
> Confidence: ~90% on all findings.

## Overview

When a vehicle moves through cells, the original engine marks and unmarks occupation
bits in the CellClass to indicate which cells are physically occupied. This prevents
other units from entering occupied cells during pathfinding and movement. The system
handles ground-level and bridge-level occupation separately via dual flag fields and
dual object linked lists.

---

## 1. Mark_Occupation (vtable+0xF0 = 0x7441B0)

**Function:** `ObjectClass::Mark_Occupation`
**Labeled in Ghidra as:** `ObjectClass__Mark_Occupation`

```c
// Decompiled from gamemd.exe 0x7441B0
void ObjectClass__Mark_Occupation(int this)
{
    int cell = CellClass__Get_Cell_At(this);          // Get cell from object coords
    int ground_height = CellClass__GetGroundHeight(this);

    // Bridge layer check: if object Z >= ground_height + bridge_offset
    // AND cell has bridge flag (bit 0x100)
    if (ground_height + g_BridgeZOffset <= *(this + 8)
        && (*(cell + 0x140) & 0x100) != 0)
    {
        *(cell + 0x128) |= 0x20;   // Set bit 5 in AltOccupationFlags (bridge)
        return;
    }
    *(cell + 0x124) |= 0x20;       // Set bit 5 in OccupationFlags (ground)
}
```

**Key details:**
- `g_BridgeZOffset` is at global `0x00B1D0AC` (computed at map load from BridgeHeight rules value)
- `this + 8` is the object's Z coordinate
- Cell flag bit 0x100 at cell+0x140 means "bridge structural cell"
- **Bit 0x20** (bit 5) in OccupationFlags is the **vehicle occupation bit**
- Ground occupation goes to cell+0x124, bridge occupation to cell+0x128

## 2. Clear_Occupation (vtable+0xF4 = 0x744210)

**Function:** `ObjectClass::Clear_Occupation`
**Labeled in Ghidra as:** `ObjectClass__Clear_Occupation`

```c
// Decompiled from gamemd.exe 0x744210
void ObjectClass__Clear_Occupation(int this)
{
    int cell = CellClass__Get_Cell_At(this);
    int ground_height = CellClass__GetGroundHeight(this);

    // Bridge layer check (same as Mark but note: no bridge flag check here!)
    if (ground_height + g_BridgeZOffset <= *(this + 8))
    {
        *(cell + 0x128) &= ~0x20;  // Clear bit 5 in AltOccupationFlags (bridge)
        return;
    }
    *(cell + 0x124) &= ~0x20;      // Clear bit 5 in OccupationFlags (ground)
}
```

**CRITICAL DIFFERENCE:** Clear_Occupation does NOT check the bridge flag (bit 0x100) in
cell+0x140. It only checks the height comparison. Mark_Occupation checks BOTH height AND
bridge flag. This is intentional -- if a bridge is destroyed while a unit is on it, the
bridge flag may be cleared, but the unit still needs to clear its bridge-level occupation
from the cell it was occupying.

## 3. OccupationFlags Bit Layout

The OccupationFlags (cell+0x124, ground) and AltOccupationFlags (cell+0x128, bridge)
share the same bit layout:

| Bit | Mask | Meaning | Set By |
|-----|------|---------|--------|
| 2 | 0x04 | Infantry subcell 2 (NE) occupied | PlaceInfantryInCell |
| 3 | 0x08 | Infantry subcell 3 (SW) occupied | PlaceInfantryInCell |
| 4 | 0x10 | Infantry subcell 4 (SE) occupied | PlaceInfantryInCell |
| 5 | 0x20 | **Vehicle/unit occupied** | Mark_Occupation / Clear_Occupation |
| 6 | 0x40 | Building present (garrisonable check) | Building placement |
| 7 | 0x80 | (Reserved/unknown) | |

**Bit semantics verified from `CellClass::CheckCellPassability` (0x4834A0):**
- `flags & 0xE0` masks infantry bits, keeping only vehicle+building bits (for infantry passability checks)
- `flags & 0x5F` masks vehicle bit, keeping only infantry+building bits (for vehicle passability checks)
- A cell with ANY nonzero occupation bits is considered blocked

**Bit semantics verified from `CellClass::IsSubCellFree` (0x481130):**
```c
// For subcell index (2, 3, or 4):
bool is_free = (occupation_flags & (1 << subcell_index)) == 0;
```

## 4. Cell Object Linked Lists

Each CellClass has two linked lists for tracking objects:

| Offset | Field | Purpose |
|--------|-------|---------|
| +0xE4 | `FirstObject` | Head of ground-level object linked list |
| +0xE8 | `AltObject` | Head of bridge-level object linked list |

**Object linkage:** Each ObjectClass has a `NextObject` pointer at offset +0x30
(i.e., `object[0xC]` when accessed as `int*`). This forms a singly-linked list.

### CellClass::AddContent (0x47E8A0)

```c
void CellClass::AddContent(Object* obj, bool bridge_layer)
{
    Object* list = bridge_layer ? this->AltObject : this->FirstObject;

    if (obj == NULL) return;

    if (obj->WhatAmI() == 6 /* Building */ && list != NULL) {
        // Buildings go at END of list
        Object* tail = list;
        while (tail->NextObject != NULL) tail = tail->NextObject;
        tail->NextObject = obj;
        obj->NextObject = NULL;
    } else {
        // All others go at HEAD of list
        if (list == NULL || list->NextObject != obj) {
            obj->NextObject = list;
            if (!bridge_layer) this->FirstObject = obj;
            else               this->AltObject = obj;
        }
    }
    // Then: Mark_Occupation (vtable+0xF0) if object is visible
}
```

### CellClass::RemoveContent (0x47EA90)

```c
void CellClass::RemoveContent(Object* obj, bool bridge_layer)
{
    if (obj == NULL) return;

    Object* list = bridge_layer ? this->AltObject : this->FirstObject;

    if (list == obj) {
        // Remove from head
        if (!bridge_layer) this->FirstObject = obj->NextObject;
        else               this->AltObject = obj->NextObject;
    } else {
        // Walk list to find predecessor
        Object* prev = list;
        while (prev != NULL && prev->NextObject != obj)
            prev = prev->NextObject;
        if (prev != NULL)
            prev->NextObject = obj->NextObject;
    }
    obj->NextObject = NULL;
    // Then: Clear_Occupation (vtable+0xF4) if object was visible
}
```

## 5. Bridge vs Ground Layer Selection

The engine uses a height-based test to determine which layer (ground or bridge) an
object occupies:

```
is_on_bridge = (object_z >= ground_height + bridge_z_offset)
               AND (cell_flags & 0x100)  // for Mark only; Clear skips flag check
```

Where:
- `object_z` = object coordinate Z component (object+0x08 for raw, or object+0xA4 for ObjectClass)
- `ground_height` = `CellClass::GetGroundHeight(object_coords)` (0x578080)
- `bridge_z_offset` = global at `0x00B1D0AC` (derived from `[General] BridgeHeight` rules value)
- `cell_flags & 0x100` = bridge structural cell flag at cell+0x140

**`CellClass::GetEffectiveHeight` (0x487D50) uses the same principle:**
```c
int CellClass::GetEffectiveHeight() {
    return (int)this->Level + ((this->Flags >> 7) & 1) * 4;
    // i.e., height_level + 4 if bridge flag bit 7 (0x80) is set
}
```

## 6. PerCellProcess / EnterCell (vtable+0x534 = 0x7416A0)

**Function:** `TechnoClass::PerCellProcess` (labeled in Ghidra)

Called each time a unit's center point enters a new cell during movement. The `param_3`
parameter indicates enter (0) vs. approach (nonzero).

### On approach (param_3 != 0):

Checks if the target cell has infantry subcells occupied (bits 0-4 of occupation flags).
If so, calls `CellClass::Scatter_Objects` to push them out of the way. Uses the bridge
layer's AltOccupationFlags (0x128) or ground OccupationFlags (0x124) based on whether
the cell has bridge flag (0x100).

```c
if (param_3 != 0) {
    if (on_bridge) {
        if ((cell->AltOccupationFlags & 0x1F) != 0)
            CellClass__Scatter_Objects(cell, ..., bridge=1);
    } else {
        if ((cell->OccupationFlags & 0x1F) != 0)
            CellClass__Scatter_Objects(cell, ..., bridge=0);
    }
}
```

### On enter (param_3 == 0) -- Crush Logic:

Walks the cell's object linked list (ground: cell+0xE4, bridge: cell+0xE8).
For each occupant:

1. Calls `TechnoClass::CanCrushCheck` -- can we crush this object?
2. If yes AND not an ally (or OmniCrusher):
   a. Check distance < 0x3FFF leptons AND object is not in-air
   b. If target is InfantryClass (WhatAmI == 0xF) and is being mind-controlled
      by us (OccupiedBy == this) AND NOT OmniCrusher:
      - Copy tilt byte, unmark occupation, execute "enter as cargo" path
   c. Otherwise (normal crush):
      - Set crush flag = true
      - Get victim's WhatAmI() (e.g., infantry type)
      - Call victim->ReceiveDamage(lethal)
      - Spawn crush animation at our coords
      - Call victim->Scatter()
      - Call victim->Mark_Occupation()  (vtable+0xE0)
      - Call victim->UnInit()           (vtable+0xD4)
      - Call victim->Remove()           (vtable+0xF8)
3. After crushing, if any were crushed:
   - Call `this->UpdatePosition()` (vtable+0x45C) with param=0
   - Adjust tread animation frame counter if applicable

## 7. Facing Lock / Cloak State During Cell Transitions

**Offset:** `TechnoClass+0x74` (accessed as `techno[0x1D]` when `int*`)
**Purpose:** Cloak state flag (0 = uncloaked, 1 = cloaked)

### The Save/Restore Pattern in Process_Drive_Track

When a unit moves within the SAME cell (subcell movement), Process_Drive_Track
uses this critical pattern:

```c
// Save cloak state
char saved_cloak = techno->cloak_state;  // techno + 0x74

// Temporarily clear cloak state
techno->cloak_state = 0;

// Move coordinates (vtable+0x1B4 = Set_Coords_With_Cloak at 0x4DB810)
techno->Set_Coords_With_Cloak(&new_coords);

// Restore cloak state
techno->cloak_state = saved_cloak;
```

**Why?** `Set_Coords_With_Cloak` (0x4DB810) checks the cloak state:
```c
void TechnoClass::Set_Coords_With_Cloak(Coord3D* coords) {
    bool moved_cells = (coords != current_coords);  // simplified

    if (this->cloak_state == 0) {
        ObjectClass::Set_Raw_Coords(coords);         // Just move
    } else {
        this->DoCloak(0);                             // Temporarily uncloak
        ObjectClass::Set_Raw_Coords(coords);          // Move
        this->DoCloak(1);                             // Re-cloak
    }
    if (moved_cells && typeClass->has_deploy_fire) {
        update_deploy_fire_target();
    }
}
```

If the cloak state were left as 1 during sub-cell moves (same cell), every coordinate
update would trigger an expensive uncloak/recloak cycle including removal/addition to
all cells in the building's foundation. By temporarily clearing the cloak state, the
engine avoids this overhead for intra-cell movement.

### When crossing to a DIFFERENT cell:

The full cloak/uncloak cycle IS desired because the unit needs to be removed from the
old cell's object list and added to the new cell's list:

```c
this->DoCloak(0);                        // vtable+0x124: uncloak (exit old cell)
this->Set_Coords_With_Cloak(&new_coords); // vtable+0x1B4: update position
this->Set_Height_On_Bridge(0);            // vtable+0x1CC: update Z for bridge
this->DoCloak(1);                         // vtable+0x124: recloak (enter new cell)
```

## 8. DoCloak (vtable+0x124 = 0x4D3780)

**Function:** `TechnoClass::DoCloak`

```c
int TechnoClass::DoCloak(int mode) {
    if (mode == 2) return 1;      // Force mode, always succeeds

    if (!ProcessCloakAndNotify(mode))  // Check if cloaking allowed
        return 0;

    if (this->GetOwnerHouse() == 2 /* ??? */) {
        this->GetCell(&temp);
        if (mode == 0) {
            // UNCLOAK: Remove from all foundation cells
            TechnoClass__ExitCell_RemoveFromMultiCells(map, this);
        } else if (mode == 1 || mode == 3) {
            // CLOAK: Add to all foundation cells
            TechnoClass__EnterCell_AddToMultiCells(map, this);
            return 1;
        }
    }
    return 1;
}
```

**Why re-evaluate cloaking on cell change?**

1. **Sensor detection:** Different cells have different sensor coverage. Moving to a new
   cell might bring the unit into a sensor array's range, which should decloak it. Or
   moving out of sensor range allows re-cloaking.

2. **Cell content changes:** The multi-cell add/remove functions
   (`ExitCell_RemoveFromMultiCells` at 0x5687F0 and `EnterCell_AddToMultiCells` at
   0x5683C0) handle buildings/large objects that occupy multiple cells. When cloaked,
   these need to be properly added/removed from the CellClass linked lists as the unit
   moves between cells.

3. **Visibility triggers:** RecalcAttributes is called on affected cells after object
   list changes, which can trigger fog-of-war and shroud updates.

## 9. CellClass::Scatter_Objects (0x481670)

Called when a moving unit needs to push infantry/objects out of its way.

```c
void CellClass::Scatter_Objects(coord, force, threat_params, bridge_layer)
{
    Object* list = bridge_layer ? this->AltObject : this->FirstObject;

    // Phase 1: Check if any crushable infantry exist (for animation purposes)
    bool has_crushable = false;
    for (obj = list; obj != NULL; obj = obj->NextObject) {
        if (is_valid_target(obj) && can_be_crushed(obj)) {
            has_crushable = true;
            break;
        }
    }

    // Phase 2: Collect objects to scatter (up to 10)
    int scatter_list[10];
    int count = 0;
    for (obj = list; obj != NULL; obj = obj->NextObject) {
        if (count < 10 && passes_filter(obj))
            scatter_list[count++] = obj;
    }

    // Phase 3: Issue scatter commands
    for (i = 0; i < count; i++) {
        if (has_crushable || force || rules->ScatterEnabled
            || (is_techno(scatter_list[i])
                && (has_ability(3) || obj_size > rules->ScatterThreshold)))
        {
            scatter_list[i]->Scatter(coord, threat_params);  // vtable+0x174
        }
    }
}
```

## 10. CellClass Complete Structure Layout

Size: 328 bytes (0x148). Verified from constructor at 0x47BBF0 and Ghidra struct definition.

| Offset | Size | Type | Field | Verified From |
|--------|------|------|-------|---------------|
| +0x00 | 16 | ptr[4] | vtable pointers (primary + 3 secondary) | Constructor |
| +0x10 | 4 | int | abstract_id | AbstractClass base |
| +0x14 | 4 | int | abstract_flags | AbstractClass base |
| +0x18 | 8 | ??? | unknown_18 | |
| +0x20 | 4 | int | unknown_20 | |
| +0x24 | 2 | short | MapCoord_X | Get_Center_Coords, constructor |
| +0x26 | 2 | short | MapCoord_Y | Get_Center_Coords, constructor |
| +0x28 | 4 | ptr | CellTag (pointer, nullable) | Constructor (zeroed, freed in dtor) |
| +0x2C | 4 | int | unknown_2C | Constructor (zeroed) |
| +0x30 | 4 | int | unknown_30 | Constructor (zeroed) |
| +0x34 | 4 | ptr | LightConvert (light source pointer) | Constructor (freed in dtor) |
| +0x38 | 4 | int | IsoTileTypeIndex (-1 = clear) | IsBridge, IsOnBridgeSurface, RecalcAttributes |
| +0x3C | 4 | ptr | AttachedTag | Constructor (zeroed) |
| +0x40 | 4 | int | unknown_40 | Constructor (zeroed) |
| +0x44 | 4 | int | OverlayTypeIndex (-1 = none) | Reduce_Tiberium, RecalcAttributes |
| +0x48 | 4 | int | SmudgeTypeIndex | Constructor (-1) |
| +0x4C | 4 | int | ZoneType (0-7) | RecalcZoneType (0x483c80) writes here — NOT RecalcLandType |
| +0x50 | 4 | int | unknown_50 (-1 init) | Constructor |
| +0x54 | 4 | int | unknown_54 (-1 init) | Constructor |
| +0x58 | 4 | int | unknown_58 (-1 init) | Constructor |
| +0x5C | 4 | int | unknown_5C (-1 init) | Constructor |
| +0x60 | 4 | int | unknown_60 (-1 init) | Constructor |
| +0x64 | 4 | int | unknown_64 (-1 init) | Constructor |
| +0x68 | 4 | int | unknown_68 | Constructor (zeroed) |
| +0x6C | 4 | int | unknown_6C | Constructor (zeroed) |
| +0x70 | 4 | int | unknown_70 | Constructor (zeroed) |
| +0x74 | 4 | int | unknown_74 | Constructor (zeroed) |
| +0x78 | 4 | u32 | VisibleToHouseBitmask | IsVisibleToHouse (1 << house_id) |
| +0x7C | 48 | short[24] | SensorCounts (per-house) | IncrementSensorCount, SensorCountForHouse |
| +0xAC | 48 | short[24] | DisguiseDetectCounts (per-house) | IncrementDisguiseDetectCount |
| +0xDC | 4 | int | unknown_DC | Constructor (zeroed) |
| +0xE0 | 4 | ptr | Jumpjet (occupying jumpjet unit) | Ghidra struct |
| +0xE4 | 4 | ptr | **FirstObject** (ground linked list head) | AddContent, RemoveContent, Scatter |
| +0xE8 | 4 | ptr | **AltObject** (bridge linked list head) | AddContent, RemoveContent, Scatter |
| +0xEC | 4 | int | **LandType** (enum, 0-11; 12 values: Clear through Weeds) | RecalcAttributes, RecalcLandType |
| +0xF0 | 8 | double | RadLevel (radiation level) | Ghidra struct |
| +0xF8 | 4 | ptr | RadSite | Ghidra struct |
| +0xFC | 4 | ptr | unknown_FC (zeroed in ctor) | Constructor |
| +0x100 | 4 | int | unknown_100 (zeroed in ctor) | Constructor |
| +0x104 | 4 | int | Walls/fences related | Constructor (0x10000 init) |
| +0x108 | 2 | short | PassabilityRate_0 (init 1000) | Constructor |
| +0x10A | 2 | short | PassabilityRate_1 (init 1000) | Constructor |
| +0x10C | 2 | short | PassabilityRate_2 (init 1000) | Constructor |
| +0x10E | 2 | short | PassabilityRate_3 (init 1000) | Constructor |
| +0x110 | 2 | short | PassabilityRate_4 (init 1000) | Constructor |
| +0x112 | 2 | short | PassabilityRate_5 (init 1000) | Constructor |
| +0x114 | 4 | int | unknown_114 (zeroed) | Constructor |
| +0x118 | 1 | u8 | ShroudState (0xFF init) | Constructor |
| +0x119 | 1 | u8 | unknown_119 | Constructor (zeroed) |
| +0x11A | 1 | u8 | bridge_sub_type | SetBridgeDirection |
| +0x11B | 1 | i8 | **Level** (height_level, signed) | GetEffectiveHeight, CheckCellPassability |
| +0x11C | 1 | u8 | **SlopeIndex** (0-20) | TMP_ReadSlopeType, RecalcAttributes |
| +0x11D | 1 | u8 | unknown_11D | RecalcAttributes (computed from height) |
| +0x11E | 1 | u8 | OverlayData / bridge_damage_state | Reduce_Tiberium, SetBridgeDirection |
| +0x11F | 1 | u8 | unknown_11F | Constructor (zeroed) |
| +0x120 | 1 | u8 | unknown_120 (0xFE init) | Constructor |
| +0x121 | 1 | u8 | unknown_121 (0xFE init) | Constructor |
| +0x122 | 1 | u8 | unknown_122 | Constructor (zeroed) |
| +0x124 | 4 | u32 | **OccupationFlags** (ground) | Mark_Occupation, Clear_Occupation, IsSubCellFree |
| +0x128 | 4 | u32 | **AltOccupationFlags** (bridge) | Mark_Occupation, Clear_Occupation, IsSubCellFree |
| +0x12C | 4 | u32 | ShroudUpdateFlags | RevealShroudFlags (bits 0x18) |
| +0x130 | 4 | int | SensorTotalCount | RevealShroudFlags (checked > 0) |
| +0x134 | 16 | ??? | unknown_134 | |
| +0x140 | 4 | u32 | **Flags** (master cell flags) | See flag table |
| +0x144 | 4 | int | unknown_144 | |

### Cell Flags at +0x140 (Complete)

| Bit | Mask | Meaning | Set By |
|-----|------|---------|--------|
| 5 | 0x0020 | Revealed to at least one house | RevealShroudFlags |
| 7 | 0x0080 | Has bridge overlay (body) | SetBridgeDirection |
| 8 | 0x0100 | **Bridge structural cell** (ramp/deck) | SetBridgeDirection |
| 9 | 0x0200 | Bridgehead (entry/exit) | SetBridgeDirection |
| 10 | 0x0400 | Bridge destroyed | SetBridgeDirection |
| 11 | 0x0800 | Bridge not-yet-destroyed flag | SetBridgeDirection |
| 12 | 0x1000 | Bridge direction-related | SetBridgeDirection |
| 13 | 0x2000 | Bridge pavement | ToggleBridgePavement |
| 16 | 0x10000 | Tall tile neighbor marker | RecalcAttributes |
| 17 | 0x20000 | Tile animation placed | RecalcAttributes |

**Mask used by SetBridgeDirection:** `0xFFFEE07F` clears bits 7-12,16 for reassignment.

## 11. The Complete Cell Transition Sequence

When Process_Drive_Track detects a cell boundary crossing (new cell != old cell),
the following sequence occurs:

### Step 1: DoCloak(0) -- Uncloak/Exit old cell
```c
techno->DoCloak(0);    // vtable+0x124
```
If the unit was cloaked, this triggers removal from all foundation cells
(`ExitCell_RemoveFromMultiCells`). For single-cell units, this removes from the
old cell's object list and clears occupation.

### Step 2: Set coordinates
```c
techno->Set_Coords_With_Cloak(&new_coords);  // vtable+0x1B4
```
Updates the object's actual coordinates. Since cloak was cleared in Step 1,
this is just a raw coordinate write.

### Step 3: Bridge ramp detection
```c
int old_cell = Get_CellClass(old_coords);
int new_cell = Get_CellClass(new_coords);
if (new_cell->Level == old_cell->Level - 4) {
    if (new_cell->Flags & 0x100)  // Bridge cell
        techno->on_bridge = 1;    // techno+0x8C
} else if (old_cell->Flags & 0x100) {
    techno->on_bridge = 0;
}
```

### Step 4: Crusher scatter (if applicable)
If the unit has crush ability (`typeClass+0xD28` or weapon ability 0x11):
- Walks new cell's object list, checks CanCrush on each
- Crushable enemies that are too close get crushed (receive lethal damage)
- Non-crushable infantry get scattered

### Step 5: DoCloak(1) -- Recloak/Enter new cell
```c
techno->DoCloak(1);    // vtable+0x124
```
If the unit has cloak capability, this triggers addition to all foundation cells
(`EnterCell_AddToMultiCells`). Adds to new cell's object list, sets occupation bits.

### Step 6: Update height for bridge
```c
techno->Set_Height_On_Bridge(0);  // vtable+0x1CC
```

### Step 7: Trigger processing
The cell-enter triggers and crate pickups are processed via
`UnitClass::OnEnterCell_Triggers` (vtable+0xE0 at 0x744720), which fires:
- Action 7 (entered cell trigger)
- Action 0x30 (zone entry trigger)
- Action 0x1D (enters trigger zone)

### Step 8: Clear_Occupation on old track endpoint
At track completion, `Apply_Track_Delta` is called with `mode=0` to clear
the old occupation:
```c
DriveLocomotionClass::Apply_Track_Delta(old_head_to_coords, 0);
// mode 0 calls vtable+0xF4 (Clear_Occupation)
```

And `mode=1` to mark the new position:
```c
DriveLocomotionClass::Apply_Track_Delta(new_head_to_coords, 1);
// mode 1 calls vtable+0xF0 (Mark_Occupation)
```

## 12. UnitClass VTable Reference (Key Entries)

Base: `0x7F5C70` (from constructor at 0x735780)

| Offset | Address | Function | Purpose |
|--------|---------|----------|---------|
| +0x2C | varies | WhatAmI() | Returns RTTI type ID |
| +0x48 | varies | GetCoords() | Get object coordinates |
| +0x78 | varies | GetOwnerHouse() | |
| +0x84 | varies | GetTechnoType() | |
| +0x88 | 0x741490 | GetTechnoType_Impl | |
| +0xBC | varies | IsOnBridge() | |
| +0xC0 | varies | IsActive() | |
| +0xD4 | 0x7440B0 | Limbo/Remove | Remove from game |
| +0xDC | 0x4D9720 | unknown (cleanup) | |
| +0xE0 | 0x744720 | **OnEnterCell_Triggers** | Fires cell triggers |
| +0xF0 | 0x7441B0 | **Mark_Occupation** | Sets bit 0x20 |
| +0xF4 | 0x744210 | **Clear_Occupation** | Clears bit 0x20 |
| +0xF8 | 0x4DE5D0 | UnInit | Release captures, free resources |
| +0x108 | varies | GetFoundationData() | Multi-cell foundation |
| +0x124 | 0x4D3780 | **DoCloak** | Cloak/uncloak, manages cell lists |
| +0x134 | varies | UpdateCloak() | |
| +0x170 | 0x746D60 | ReceiveMessage hook | |
| +0x174 | 0x743A50 | **Scatter** | Move away from threat |
| +0x18C | 0x739EC0 | Mission_Enter | Enter transport |
| +0x198 | varies | DiscoverByHouse() | |
| +0x1AC | varies | Can_Enter_Cell() | Pathfinding check |
| +0x1B4 | 0x4DB810 | **Set_Coords_With_Cloak** | Move + cloak handling |
| +0x1B8 | 0x41BEA0 | GetCell() | Get current CellClass* |
| +0x1CC | 0x5F5FA0 | **Set_Height_On_Bridge** | Adjust Z for bridge |
| +0x1D0 | 0x5F5F30 | GetHeight() | Returns object Z coord |
| +0x274 | varies | QueueVoice() | |
| +0x28C | varies | CanScatter() | |
| +0x3D4 | varies | EnterAsPassenger() | |
| +0x45C | varies | UpdatePosition() | |
| +0x480 | varies | MoveTo() | |
| +0x484 | varies | TryRepair() | |
| +0x504 | varies | Find_Path() | |
| +0x534 | 0x7416A0 | **PerCellProcess** | Crush, scatter, enter cell |
| +0x538 | varies | GetMoveSpeed() | |
| +0x544 | varies | SetMaxSpeed() | |

## 13. DriveLocomotionClass::Mark_All_Occupation_Bits (0x4B48D0)

Called to mark occupation for the destination cell when starting a new track:

```c
void DriveLocomotionClass::Mark_All_Occupation_Bits(int mode) {
    if (this->head_to != NullCoord) {
        Apply_Track_Delta(this->head_to, mode);
    }
}
```

`Apply_Track_Delta` (0x4B0AD0) computes the cell the track endpoint falls in and
calls either `Mark_Occupation` (mode=1) or `Clear_Occupation` (mode=0) on that cell.
If the track has a midpoint (non-reversed, valid track), it also marks/clears the
intermediate cell.

## 14. Summary of Occupation Lifecycle for a Moving Vehicle

```
START: Vehicle at cell A, wants to move to cell B

1. Path found, track assigned (track_index set)

2. Mark_All_Occupation_Bits(1) on cell B
   --> cell_B.OccupationFlags |= 0x20  (or AltOccupationFlags if bridge)
   --> Cell B is now "reserved" -- other pathfinders see it as occupied

3. Track stepping begins (Process_Drive_Track loop)
   Each step: update coordinates, check for cell boundary

4. At cell boundary crossing A->B:
   a. DoCloak(0)                    -- exit cell A lists
   b. Set_Coords(B_coords)         -- update position
   c. Bridge ramp detection         -- set on_bridge flag
   d. PerCellProcess (crush/scatter in B)
   e. DoCloak(1)                    -- enter cell B lists
   f. Set_Height_On_Bridge          -- Z adjustment

5. Track completes at cell B:
   a. Clear_Occupation on cell A    -- cell_A.OccupationFlags &= ~0x20
   b. OnEnterCell_Triggers          -- fire map triggers
   c. Find next path segment or stop

RESULT: Vehicle now at cell B, occupation transferred from A to B
```
