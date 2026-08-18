# ObjectClass: Draw_It Pipeline, Limbo/Unlimbo Lifecycle, and Cell Linked Lists

Reverse-engineered via Ghidra MCP (live decompilation of `gamemd.exe`).
Confidence: HIGH on all findings unless noted otherwise.
All addresses verified from direct decompilation of gamemd.exe.

**Supplements:** `OBJECTCLASS_GHIDRA_REPORT.md` (struct layout, health, select, mark),
`TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md` (three-pass architecture),
`CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md` (vehicle mark/clear).

---

## 1. Draw_It Pipeline

### 1.1 How Objects Are Iterated for Drawing

The rendering system uses **5 display layers** stored in `g_DisplayLayers` (global array
at ~0x8A0390, 6 DWORDs per entry: vtable, buffer, ?, capacity, ?, ?):

| Layer | Index | Contents |
|-------|-------|----------|
| Underground | 0 | Tunneled/subterranean objects |
| Surface | 1 | Flat ground-level objects (smudges, low anims) |
| Ground | 2 | All ground units, buildings, most objects |
| Air | 3 | Aircraft, high-altitude objects |
| Top | 4 | Topmost overlays |

**`Tactical_ObjectRenderingLoop`** (0x006D8DB0) is called during Pass 2, Step 8 of the
three-pass rendering architecture. It iterates all 5 layers twice:

**First pass (draw):** For each object in each layer:
1. Clear `IsVisible` flag (byte at obj+0x99) to 0
2. Read object's Location (obj+0x9C/0xA0/0xA4) directly
3. Based on object RTTI type, decide how to compute screen coords:
   - **RTTI 4 (AnimClass):** Use `GetCoords` (vtable+0x48), check bounding box against
     viewport margin (0x168 pixels horizontal, 0xB4 pixels vertical). If TypeClass has
     AlphaImage flag (TypeClass+0x374) and not fog-gated, check shroud visibility.
   - **RTTI 0x24 (BuildingClass) with special turret/overlay:** Read coords from
     obj+0x9C/0xA0/0xA4 directly. Check shroud via `FUN_005865e0`.
   - **TechnoClass (IsTechno bit set, bit 0 of AbstractFlags at +0x14 — tested via `(byte @+0x14) & 1`):** Use
     `GetCoords` (vtable+0x48), convert via `CoordsToClient`, also call `DrawShadow`
     (vtable+0x110) before `DrawAs` (vtable+0x104).
   - **Non-techno with IsTechno clear:** Use `GetCoords`, convert via
     `Tactical__WorldToScreenSub` + `AdjustForZ`, check viewport margin.
4. If within viewport: set `IsVisible = 1` (obj+0x99)
5. Call `vtable+0x10C` (SetDrawCoords) with screen position + viewport bounds
6. Call `vtable+0x104` (DrawIt / DrawAs) with viewport bounds, draw flags, and force param

**After layer 2 only:** Iterates all buildings (g_BuildingClass_Array) to draw turret
fire and garrison fire overlays via `BuildingClass__UpdateGarrisonFire`.

**Second pass (extras):** For each object with `IsVisible` set:
- Call `vtable+0x110` (DrawExtras) -- selection brackets, health bars, pips, veterancy

### 1.2 ObjectClass::DrawIt (0x005F4B10, vtable+0x104)

**param_1 type:** `int *` (offsets are word-indexed, multiply by 4 for byte offset)

This is the base DrawIt. Subclasses override vtable+0x104 entirely (buildings, units,
aircraft, infantry each have their own). The ObjectClass base does:

```
DrawIt(this, viewportBounds, forceRedraw):
    // Skip if loading screen active or no window
    if DAT_00a8ed6b || g_hWnd == 0: skip checks
    
    // Visibility check: must need redraw (or forced) AND not in limbo
    if (!forceRedraw && this->NeedsRedraw == 0):  // +0x80
        return 0
    if (this->InLimbo):                            // +0x81
        return 0
    
    // Clear NeedsRedraw flag
    this->NeedsRedraw = 0                          // param_1[0x20] = 0 (byte at +0x80)
    
    // Get render coordinates and convert to screen
    coords = vtable+0xAC()                         // GetRenderCoords
    success = TacticalClass__CoordsToClient2(coords, &screenPos)
    
    // If conversion failed and object is NOT anim (RTTI 0x18), skip
    if (!success && WhatAmI() != 0x18):
        return 0
    
    // Clip screen rect against viewport bounds
    // ... (rect intersection with g_RadarViewport{OffsetX,OffsetY,Width,Height})
    
    // Adjust draw position for viewport offset
    // ... (subtract viewport origin from screen position)
    
    // Dispatch to subclass-specific drawing
    vtable+0x114(&drawPos, &clippedRect)           // DrawSHP_base or subclass override
    return 1
```

**Key points:**
- `NeedsRedraw` (+0x80) is the gatekeeper -- objects only draw when dirty or forced
- `InLimbo` (+0x81) objects are never drawn
- The coordinate transformation: game lepton coords -> screen pixel coords via
  `TacticalClass__CoordsToClient2` which applies the isometric projection matrix
- After basic checks, dispatches to `vtable+0x114` for actual pixel drawing

### 1.3 Coordinate Transformation Chain

Game coordinates (leptons, 3D isometric) to screen pixels:

1. **GetRenderCoords** (vtable+0xAC, default 0x41BE00): Returns object's 3D position
   in lepton space. Usually same as `GetCoords` (Location at +0x9C/0xA0/0xA4).
   Buildings may override to return foundation center.

2. **Tactical__WorldToScreenSub** (used in rendering loop): Converts 3D lepton coords
   to 2D screen via the isometric projection:
   ```
   screenX = (X * 0x3C/2 + Y * -0x3C/2) / 256
   screenY = (X * 0x1E/2 + Y * 0x1E/2) / 256
   ```
   Then subtracts the viewport scroll offset (Tactical+0xB0, Tactical+0xB4).

3. **AdjustForZ**: Subtracts Z/256 (height in leptons divided by 256) from screenY.
   Higher objects appear further up on screen.

4. **Viewport clipping**: Objects are culled if their screen position is outside
   the viewport with a generous margin (0x168 pixels horizontal = 360px,
   0xB4 pixels vertical = 180px) to account for large sprites.

### 1.4 Draw Order and Z-Sorting

Objects are sorted by **display layer** first (Underground < Surface < Ground < Air < Top),
then within each layer by **insertion order** in the DynamicVector.

**`DisplayClass__Submit_Object`** (0x004A9720):
```
Submit_Object(obj):
    if obj->LastLayer != -1:
        RemoveFromLayer(obj)          // remove from old layer
    layer = obj->GetMapLayer()        // vtable+0x78
    if layer != -1:
        DynamicVector__Insert(obj, layer == 2)   // insert; for Ground layer, sorted insert
        obj->LastLayer = layer        // +0x94
```

**`DisplayClass__RemoveFromLayer`** (0x004A9770): Scans all 5 display layer vectors,
finds and removes the object, resets `LastLayer` to -1.

**Y-sort within layers** is done by `GetYSort` (0x5F6BD0):
```
GetYSort(this):
    renderCoords = vtable+0xAC()   // GetRenderCoords
    return renderCoords.Y + renderCoords.X   // combined for isometric sort
```
Objects with higher Y-sort values are drawn later (on top). The comparator at
0x5F6220 sorts in descending Y order.

### 1.5 Visibility Checks

Before drawing, several visibility gates apply:

| Check | Location | Purpose |
|-------|----------|---------|
| `InLimbo` (+0x81) | ObjectClass::DrawIt | Skip objects not on map |
| `NeedsRedraw` (+0x80) | ObjectClass::DrawIt | Skip objects that haven't changed |
| `IsVisible` (+0x99) | Rendering loop | Set by viewport bounds check |
| Shroud check | Rendering loop | `FUN_005865e0` checks if cell is shrouded |
| Fog-of-war check | Rendering loop | Only if SpecialFlags & 0x1000 (TS legacy, off by default) |

**Shroud gating:** For TechnoClass objects (IsTechno bit set), the shroud check uses
the object's coordinates directly. If the cell is fully shrouded (black), the object
is skipped. For anims with `AlphaImage`, a separate fog/shroud check gates visibility.

### 1.6 TechnoClass::Draw (0x00706640)

The main TechnoClass drawing dispatcher. Called from the object's vtable+0x104 slot
(overridden per class: UnitClass, InfantryClass, AircraftClass, BuildingClass).

**Draw flag assembly** (param_11 = 0 means compute flags):
```
if drawFlagsOverride == 0:
    baseFlags = 0x2000
    cloakState = vtable+0x68(0,0)   // GetCloakState
    switch(cloakState):
        case 1: flags = 0x2002      // cloaking
        case 2,3: flags = 0x2004    // cloaked
        case 4: flags = param_1[0x89]==0 ? 0x200A : 0x200C  // decloaking
        case 5: return              // fully invisible, skip draw entirely

// Iron Curtain / Force Shield check
if (IsIronCurtainActive() || IsForceShielded()):
    if not infantry with deployed flag:
        if building with TypeClass+0x16B1 flag:
            flags |= 0x2006         // special building IC visual
        else:
            flags |= 0x2004         // standard IC translucency

// Player spy detection: if allied and IsCloakable
if (IsHumanPlayer() && vtable+0xC4() != 0):
    if not infantry with deployed flag:
        flags = vtable+0x43C(flags)  // ModifyCloakDrawFlags
```

**SHP vs VXL dispatch:**
- If `param_3 != -1` (VXL frame index): attempts VXL cache blit via `VXL_CacheBlit`
- If VXL fails or not applicable: calls `TechnoClass__Render` (0x706ED0) for SHP rendering
- `TechnoClass__Render` handles palette selection, remap colors, alpha/translucency

**Temporal/Warp visual effects:**
- `TechnoClass__ScaleByTemporalVisualPhase` -- modifies draw intensity when being
  erased by Temporal weapon
- `TechnoClass__ScaleByWarpInVisualPhase` -- modifies draw intensity during
  Chronosphere warp-in

### 1.7 Draw Extras: Selection Brackets and Health Bars

**`TechnoClass__DrawExtras`** (0x006F5190, vtable+0x110 in second pass):

Called for all visible objects after the main draw pass. Renders:

1. **Ivan Bomb indicator:** If `this->AttachedBomb` (+0x38 via obj[0x1A]) is non-null
   and obj is marked (+0x74 != 0), draws the bomb clock shape using
   `IvanBomb__GetClockFrame` and `CC_Draw_Shape` with `g_RulesClass->IvanBombImage`.

2. **Wrench/repair indicator:** If WhatAmI()==6 (Building) and building has
   `IsRepairing` flag (+0x6E8), draws animated wrench SHP (`g_WRENCH_SHP`).

3. **Crate/upgrade pips:** If owned by non-human player, calls vtable+0x454 (DrawPips).

4. **Selection brackets:** If `IsSelected` (+0x83) is set:
   - **Buildings:** Gets foundation dimensions from TypeClass (vtable+0x84 -> vtable+0x7C),
     computes 4 corner positions in 3D, converts each to screen, draws bracket corners
     via `TechnoClass__DrawBracketCorner` (0x6F5EF0). For multi-height buildings,
     brackets extend vertically.
   - **Infantry:** Uses a simplified bracket with corner offset calculations.
   - **Vehicles/Aircraft:** Standard 4-corner bracket.

5. **Health bar:** If selected AND allied (or SpiedOnBy), calls vtable+0x448 (DrawHealthBar).

6. **Tag marker:** If `this->field_0x431` != 0 and not selected, draws a secondary
   indicator (capture manager or similar).

7. **Cursor highlight:** If `DAT_00b0eb38 == this`, draws the cursor selection SHP.

**`TechnoClass__DrawHealthBar`** (0x006F64A0):

Two rendering modes:
- **Buildings:** Draws isometric health pips along the foundation diagonal. Health
  ratio determines how many green/yellow/red pips vs empty pips. Uses
  `DAT_00ac147c` (PIPS.SHP). Frame 1 = green, 2 = yellow, 4 = red, 0 = empty.
- **Units/Infantry:** Draws horizontal health pips above the unit. Frame 0x10 = green,
  0x11 = yellow, 0x12 = red. Position offset from TypeClass+0x3E0 (DrawOffset.Y).

After health pips, draws **veterancy/group pips** via vtable+0x450 if the unit is
visible to the player.

### 1.8 Shadow Drawing

Shadows are drawn in the first pass via `vtable+0x110` (DrawShadow), called BEFORE
the main `vtable+0x104` (DrawAs). This ensures shadows appear beneath the object.

For TechnoClass objects with the `IsTechno` bit (bit 0 of `AbstractFlags @+0x14`) set
AND `WhatAmI() != BuildingClass` (non-building), the shadow is drawn using the same
sprite data but with shadow draw flags that project the shape flat onto the ground with
reduced alpha. (Previous wording referred to an "IsTechno bit 2" which did not exist —
the building check is done via the `WhatAmI` virtual, not a second bit.)

---

## 2. Limbo/Unlimbo Lifecycle

### 2.1 Overview

"Limbo" is the engine's term for an object being off-map -- not in any cell, not rendered,
not visible. Objects start in limbo (InLimbo=true in constructor) and must be "unlimboed"
to appear on the map. The reverse process (going back to limbo) happens through Conceal.

**State transitions:**
```
                 Reveal()                    Conceal()
  [LIMBO] ────────────────> [ON MAP] ────────────────> [LIMBO]
  InLimbo=1                 InLimbo=0                  InLimbo=1
  IsMarked=0                IsMarked=1                 IsMarked=0
  NeedsRedraw=0             NeedsRedraw=?              NeedsRedraw=0
                            LastLayer=0..4             LastLayer=-1
```

### 2.2 ObjectClass::Reveal (0x005F4EC0) -- Enter the Map

**vtable+0xD8.** Places an object from limbo onto the map.

```
Reveal(this, coords):
    // Reject null/zero coordinates
    if coords == {0,0,0}: return false
    if !g_GameActive: return false
    
    if InLimbo:
        // Map editor bypass
        if !g_MapEditorMode:
            cell = CellClass__Get_Cell_At(coords)
            result = vtable+0x1AC(cell, -1, -1, 0, 0)   // CanEnterCell check
            if result != 0: return false                   // cell blocked
        
        // 1. Clear limbo state
        this->InLimbo = false          // +0x81 = 0
        this->NeedsRedraw = false      // +0x80 = 0
        
        // 2. Apply TypeClass coordinate transform
        typeClass = vtable+0x88()
        if typeClass != NULL:
            adjustedCoords = typeClass->vtable+0x6C(coords)  // type-specific coord adjust
        else:
            adjustedCoords = coords
        
        // 3. Set position
        vtable+0x1B4(adjustedCoords)    // Set_Raw_Coords
        
        // 4. Register in cell grid
        success = vtable+0x124(1)       // Mark(MARK_PUT) -- sets IsMarked=true
        
        if success:
            if IsAlive:
                // 5. Add to display layer
                layer = vtable+0x78()    // GetMapLayer
                if layer != -1:
                    DisplayClass__Submit_Object(this)
                
                // 6. Create alpha shape if TypeClass has AlphaImage
                typeClass = vtable+0x88()
                if typeClass->AlphaImageOffset (+0xAC) != 0:
                    AlphaShapeClass::Constructor(this, screenX, screenY)
                
                // 7. Create line trail if TypeClass has LineTrail
                if typeClass->HasLineTrail (+0x23A):
                    LineTrailClass::Constructor()
                    this->LineTrailer = newTrail    // +0xA8
                
            return true
        else:
            // Mark failed -- revert to limbo
            this->InLimbo = true
            return false
    
    return false   // not in limbo, nothing to do
```

**Critical sequence:** InLimbo=false -> Set_Raw_Coords -> Mark(PUT) -> Submit_Object.
The Mark(PUT) call triggers `Mark_Put` which sets cell occupation flags (bit 0x40).
Submit_Object adds the object to the appropriate display layer for rendering.

### 2.3 ObjectClass::Conceal (0x005F4D30) -- Leave the Map

**vtable+0xD4.** Removes an object from the map back to limbo.

```
Conceal(this):
    if !g_GameActive || InLimbo: return false
    
    // 1. Deselect
    vtable+0x150()                    // Deselect
    
    // 2. Mark dirty for redraw (so area is repainted without this object)
    vtable+0xDC(true)                 // NeedsRedraw -> true, then...
    
    // 3. Remove from cell grid
    vtable+0x124(0)                   // Mark(MARK_REMOVE) -- sets IsMarked=false
    
    // 4. Remove from display layer
    DisplayClass__RemoveFromLayer(this)  // 0x4A9770
    
    // 5. Detach visual effects
    AnimClass__Detach()               // detach any anim referencing this
    FUN_00405fd0()                    // timer cleanup
    
    // 6. Alpha shape cleanup
    typeClass = vtable+0x88()
    if typeClass && typeClass->HasAlphaImage:
        // Remove AlphaShapeClass if present
        FUN_0055bae0(this)
    
    // 7. Dirty screen rect for repaint
    if typeClass->AlphaImageOffset != 0:
        // Compute screen rect from alpha image dimensions
        TacticalClass__DirtyScreenRect(x, y, w, h, 1)
    
    // 8. Clear drawn state
    vtable+0x11C()                    // ClearDrawnState
    
    // 9. Set limbo state
    this->InLimbo = true              // +0x81 = 1
    this->NeedsRedraw = false         // +0x80 = 0
    
    return true
```

**Critical sequence:** Deselect -> Mark(REMOVE) -> RemoveFromLayer -> InLimbo=true.
The Mark(REMOVE) call triggers `Mark_Remove` which clears cell occupation flags (bit 0x40).

### 2.4 TechnoClass::Unlimbo (0x006F6CA0)

**vtable+0x74 override for TechnoClass.** The full Unlimbo used by TechnoClass and
its subclasses (UnitClass, InfantryClass, BuildingClass, AircraftClass).

```
TechnoClass__Unlimbo(this, coords, facing):
    // 1. Call ObjectClass::Reveal (the base reveal logic)
    success = ObjectClass__Reveal(this, coords)
    if !success: return false
    
    // 2. Record "is in playfield" flag
    cellCoord = {coords.X/256, coords.Y/256}
    this->IsInPlayfield = MapClass__Is_Cell_In_Playfield(cellCoord)  // +0x3D5
    
    if !IsAlive: return true    // +0x90, checked as param_1[0x24] byte
    
    // 3. Initialize vision/shroud reveal
    vtable+0x488(0, 0, 0, 0)         // UpdateVision(clear old)
    MapClass__UpdateFogBorder(coords, 0, this->Sight+3, 0)
    
    // 4. Check special building abilities
    typeClass = vtable+0x84()
    if typeClass->GapGenerator (+0x5F8) != 0:
        FUN_00439080(this)            // Gap generator setup
    
    // 5. Register with house tracking
    HouseClass__Added_To_Game(this, 0)
    
    // 6. Initialize facing
    FacingClass__UpdateFacing(facing)
    FacingClass__UpdateFacing(0x4000)  // turret facing
    // Also set turret ROT from TypeClass+0x3D0
    
    // 7. Initialize body state
    this->field_0x49C = 1             // param_1[0x127] -- body state initialized
    this->field_0x4A0 = 0             // param_1[0x128]
    
    // 8. Infantry special: set IsLaying flag
    if WhatAmI() == 0xF (Infantry):
        this->IsLaying = true          // param_1[0x7E]
    
    // 9. Update radar/sensor coverage
    vtable+0x484(1, 1)               // UpdateSensorArrays
    this->IsLaying = false
    
    // 10. Check and activate deploy-fire
    if vtable+0x200() (CanDeployFire):
        vtable+0x1EC()                // ActivateDeployFire
    
    // 11. Calculate initial fall speed from height
    this->FallSpeedRaw = (coords.Z / g_LevelHeight) * 10   // param_1[0x108]
    
    // 12. Check if cell is in playfield for waypoint management
    // ... (additional waypoint/zone setup)
    
    return true
```

### 2.5 ObjectClass::Unlimbo_Full (0x005F5940, vtable+0xE8)

This is an extended Unlimbo that also handles parachute/drop animations. Called when
spawning objects with airborne entry.

```
Unlimbo_Full(this, coords):
    // Validate coords
    if !FUN_005785f0(coords): return false  // coord validation
    
    this->IsFallingDown = true              // +0x8D = 1
    
    // Get cell, check bridge
    cell = CellClass__Get_Cell_At(coords)
    if cell == NULL: return false
    
    if (cell->Flags & 0x100):               // bridge cell
        this->OnBridge = true               // +0x8C via param_1[0x23]
        if (cell->Flags & 0x200) == 0:      // bridge destroyed?
            return false
    
    // TechnoClass passability check
    if IsTechno (AbstractFlags bit 0):
        typeClass = vtable+0x84()
        speedType = typeClass->SpeedType (+0x5B4)
        cellCoord = {coords.X/256, coords.Y/256}
        zoneID = MapClass__GetZoneID(cellCoord, speedType, this->OnBridge)
        typeClass->SetZoneInfo(0, 0, zoneID, speedType, -1, 1)
        if !CellClass__CheckCellPassability(cell):
            return false
    
    // Call Reveal (vtable+0xD8) to place on map
    success = vtable+0xD8(coords, 0x80)
    if !success: return false
    
    // Set raw coordinates
    vtable+0x1B4(coords)                    // Set_Raw_Coords
    
    // Create drop/parachute animation
    if WhatAmI() == 8 (AircraftClass):
        // Air drop anim from RulesClass+3000 (ParachuteAnim)
        AnimClass__Constructor(RulesClass->ParachuteAnimType, coords, ...)
    else:
        // Ground drop anim from RulesClass+0xBBC (DropPodAnim?)
        AnimClass__Constructor(RulesClass->DropAnimType, coords, ...)
        this->Parachute = newAnim           // +0x88 via param_1[0x22]
    
    // Attach anim to object
    if newAnim != NULL:
        AnimClass__SetOwnerObject(this)
        if WhatAmI() != 8:
            // Set anim owner house color
            anim->OwnerHouse = vtable+0x1E4()  // GetOwnerHouse
            // Set anim ZAdjust from TypeClass
            typeClass = vtable+0x1BC()
            anim->ZAdjust = typeClass->field_0x10A
    
    return true
```

### 2.6 Subclass Unlimbo Overrides

**UnitClass::Unlimbo** (0x00737BA0):
```
UnitClass__Unlimbo(this, coords, facing):
    success = TechnoClass__Unlimbo(coords, facing)   // calls 0x6F6CA0
    if !success: return false
    
    // Set unit facing
    FacingClass__UpdateFacing(facing)
    
    // Initialize tread/locomotor animation state
    if this->HasTurretWalk (+0x3D2) && !this->IsDeployed (+0x3D5):
        this->field_0x220 = 2    // locomotor state
    
    typeClass = this->TypeClass (+0x6C4)
    if typeClass->IsAnimated (+0xE18) == false && typeClass->IsVoxel (+0xE19) == false:
        // SHP unit: start with frame 0
        this->AnimFrame = 0        // +0xF8
        this->AnimTimer = g_CurrentFrameCounter  // +0x100..0x10C
    else:
        // Animated/voxel unit: randomize starting frame
        this->AnimFrame = Random(0, 29)
        this->AnimTimer = g_CurrentFrameCounter
        this->AnimDirection = 1
    
    return true
```

**AircraftClass::Unlimbo** (0x00414310): Similar pattern, sets flight state.
**OverlayClass::Unlimbo** (0x005FD270): Overlay-specific cell registration.

### 2.7 Limbo Path (Removing from Map)

ObjectClass base `Limbo()` at vtable+0x70 (0x5F4250) is a **stub returning 0**.
The actual limbo functionality is distributed:

- **Going off-map:** `ObjectClass::Conceal` (vtable+0xD4, 0x5F4D30) handles it
- **Destruction:** `ObjectClass::UnInit` (vtable+0xF8, 0x5F65F0) calls Conceal then
  marks the object as dead
- **Entering transport:** The transport's load logic calls Mark(REMOVE) and
  RemoveFromLayer, sets InLimbo=true

**AnimClass::Limbo** (0x00425530) is one of the few real Limbo overrides.

---

## 3. Cell Linked List Management

### 3.1 Data Structure

Each CellClass has two singly-linked lists of objects:

| CellClass Offset | Field | Purpose |
|-----------------|-------|---------|
| +0xE4 | `FirstObject` | Head of ground-level object list |
| +0xE8 | `AltObject` | Head of bridge-level object list |

Each ObjectClass has:

| ObjectClass Offset | Field | Purpose |
|-------------------|-------|---------|
| +0x30 | `NextObject` | Pointer to next object in same cell's list |

The linked list is a singly-linked list with the cell storing only the head pointer.
`NextObject` chains from head to tail (NULL-terminated).

### 3.2 CellClass::AddContent (0x0047E8A0)

Adds an object to a cell's occupant list.

```
AddContent(cell, object, isBridgeLayer):
    if object == NULL: return
    
    // Select ground or bridge list
    list = isBridgeLayer ? cell->AltObject : cell->FirstObject
    
    // Buildings go at END of list (so they're drawn first / found last)
    if object->WhatAmI() == 6 (Building) AND list != NULL:
        tail = list
        while tail->NextObject != NULL:
            tail = tail->NextObject
        tail->NextObject = object
        object->NextObject = NULL
    
    // All other objects go at HEAD of list
    else:
        // Guard against double-insertion
        if list == NULL OR list->NextObject != object:
            object->NextObject = list
            if !isBridgeLayer:
                cell->FirstObject = object
            else:
                cell->AltObject = object
    
    // Shroud/vision check for new content
    cellCoord = {cell->MapCoord_X * 256 + 128, cell->MapCoord_Y * 256 + 128, ...}
    if IsShrouded(cellCoord) OR IsFogged(cellCoord):
        if g_GameMode != 0:  // not campaign
            object->vtable+0x198(g_PlayerPtr)   // Discovered_By
    
    // Mark cell occupation bits
    if object->WhatAmI() != 0xF (not Infantry) AND vtable+0xC0() (IsOccupier):
        if WhatAmI() == 6 (Building):
            // Use cell center coords for buildings
            coords = {cell->MapCoord_X * 256 + 128, cell->MapCoord_Y * 256 + 128, 0}
        else:
            // Use object's actual coords
            coords = object->Location
        object->vtable+0xF0(coords)             // Mark_Occupation
```

**Key design decisions:**
- Buildings are appended to END of list; everything else prepended to HEAD
- This means when traversing the list for rendering, buildings come last
- The guard `list->NextObject != object` prevents circular references
- Mark_Occupation (vtable+0xF0) sets bit 0x20 on cell flags for vehicles

### 3.3 CellClass::RemoveContent (0x0047EA90)

Removes an object from a cell's occupant list.

```
RemoveContent(cell, object, isBridgeLayer):
    if object == NULL: return
    
    list = isBridgeLayer ? cell->AltObject : cell->FirstObject
    
    // Case 1: removing head of list
    if list == object:
        if !isBridgeLayer:
            cell->FirstObject = object->NextObject
        else:
            cell->AltObject = object->NextObject
    
    // Case 2: removing from middle/end
    else:
        prev = list
        while prev != NULL AND prev->NextObject != object:
            prev = prev->NextObject
        if prev != NULL:
            prev->NextObject = object->NextObject
    
    // Clear the removed object's link
    object->NextObject = NULL
    
    // Clear cell occupation bits
    if object->WhatAmI() != 0xF (not Infantry) AND vtable+0xC0() (IsOccupier):
        if WhatAmI() == 6 (Building):
            coords = {cell center}
        else:
            coords = object->Location
        object->vtable+0xF4(coords)             // Clear_Occupation
```

### 3.4 Occupation Flag Layout (CellClass +0x124 / +0x128)

Two flag fields: `OccupationFlags` at +0x124 (ground) and `AltOccupationFlags` at
+0x128 (bridge). Same bit layout:

| Bit | Mask | Set By | Meaning |
|-----|------|--------|---------|
| 2 | 0x04 | PlaceInfantryInCell | Infantry subcell 2 (NE) occupied |
| 3 | 0x08 | PlaceInfantryInCell | Infantry subcell 3 (SW) occupied |
| 4 | 0x10 | PlaceInfantryInCell | Infantry subcell 4 (SE) occupied |
| 5 | 0x20 | Mark_Occupation | Vehicle/unit present |
| 6 | 0x40 | Mark_Put | Object placed (building/terrain) |

**Mark_Put** (0x5F60A0): Sets bit 0x40 using bridge height check:
```
if object_Z >= groundHeight + BridgeHeight AND cell has bridge flag (0x100):
    cell->AltOccupationFlags |= 0x40    // +0x128
else:
    cell->OccupationFlags |= 0x40       // +0x124
```

**Mark_Remove** (0x5F6120): Clears bit 0x40 with same height check:
```
if object_Z >= groundHeight + BridgeHeight AND cell has bridge flag (0x100):
    cell->AltOccupationFlags &= ~0x40
else:
    cell->OccupationFlags &= ~0x40
```

### 3.5 Infantry Sub-Cell Positioning

**`CellClass__PlaceInfantryInCell`** (0x00481180):

Infantry can share cells using 5 sub-cell positions (0=center, 1=NW, 2=NE, 3=SW, 4=SE).
Each sub-cell has a lepton offset from the cell center stored in a global table at
`DAT_0089E9F0` (3 ints per sub-cell: X offset, Y offset, Z offset).

Sub-cell selection algorithm:
1. Calculate distance from cell center using object coordinates
2. If distance < 0x3C leptons: assign sub-cell 0 (center)
3. Otherwise: compute quadrant from X/Y offsets:
   - `quadrant = (X > 0x80 ? 1 : 0) | (Y > 0x80 ? 2 : 0)`
   - If quadrant != 0: quadrant += 1 (so values are 2, 3, 4)
4. If preferred sub-cell is occupied (OccupationFlags bit test): try alternates
   from a lookup table at `DAT_0081CC84` (4 entries per starting sub-cell)
5. If all alternates occupied: return invalid coords (DAT_0089E778)
6. Final coords = cell_base + sub-cell_offset + ground_height (+ bridge height if on bridge)

Sub-cell occupation bits are checked/set using:
```
is_occupied = (OccupationFlags >> subcell_index) & 1
```

### 3.6 List Traversal Patterns

**For rendering:** The main rendering loop does NOT traverse cell linked lists directly.
Instead, it iterates the display layer vectors (`g_DisplayLayers`). Objects are added
to display layers via `DisplayClass__Submit_Object` during Reveal/Unlimbo.

**For target selection / cell queries:** Code walks the cell linked list:
```
obj = cell->FirstObject   // or AltObject for bridge
while obj != NULL:
    // check obj type, alliance, etc.
    obj = obj->NextObject   // +0x30
```

**For crush logic** (PerCellProcess): Walks the list checking each occupant for
crushability, then applies damage/removal.

**For occupation checks:** Read the occupation flags directly (0x124/0x128) rather
than walking the list. The flags are a fast bitmap summary of list contents.

### 3.7 Bridge vs Ground Layer Selection

The engine determines which list to use based on height:

```
isBridgeLayer = (object_Z >= groundHeight + g_BridgeZOffset)
                AND (cell->Flags & 0x100)  // bridge structural cell
```

- `g_BridgeZOffset` at global 0x00B1D0AC (computed from [General] BridgeHeight)
- `cell->Flags & 0x100` at cell+0x140: bridge structural cell flag
- **Important asymmetry:** Mark_Occupation checks BOTH height AND bridge flag.
  Clear_Occupation checks ONLY height (no bridge flag check). This is intentional --
  if a bridge is destroyed while a unit is on it, the unit must still clear its
  bridge-level occupation.

---

## 4. Key Struct Fields Summary

### ObjectClass Fields for Draw/Limbo/Cell

| Offset | Size | Type | Name | Purpose |
|--------|------|------|------|---------|
| 0x14 | 1 | byte | AbstractFlags | Bit 0=IsActive, Bit 1=Exists, Bit 2=IsTechno |
| 0x30 | 4 | ptr | NextObject | Linked list pointer in cell |
| 0x74 | 1 | bool | IsMarked | true when registered in cell grid via Mark(PUT) |
| 0x78 | 4 | int | Layer | Current map layer (0-4) |
| 0x80 | 1 | bool | NeedsRedraw | Dirty flag -- object needs to be redrawn |
| 0x81 | 1 | bool | InLimbo | true = off-map, not in any cell |
| 0x83 | 1 | bool | IsSelected | true = player has selected this object |
| 0x8C | 1 | bool | OnBridge | true = object is on bridge level |
| 0x8D | 1 | bool | IsFallingDown | true = currently falling (parachute/drop) |
| 0x90 | 1 | bool | IsAlive | false = pending destruction |
| 0x94 | 4 | int | LastLayer | Display layer index, -1 if not in any layer |
| 0x99 | 1 | bool | IsVisible | Set by render loop: true if within viewport bounds |
| 0x9C | 12 | Coord3D | Location | X/Y/Z in lepton coordinates |
| 0xA8 | 4 | ptr | LineTrailer | LineTrailClass* for visual trail |

### CellClass Fields for Object Lists

| Offset | Size | Type | Name | Purpose |
|--------|------|------|------|---------|
| 0xE4 | 4 | ptr | FirstObject | Head of ground-level object linked list |
| 0xE8 | 4 | ptr | AltObject | Head of bridge-level object linked list |
| 0x124 | 4 | uint | OccupationFlags | Ground occupation bitmap (bits 2-6) |
| 0x128 | 4 | uint | AltOccupationFlags | Bridge occupation bitmap (bits 2-6) |
| 0x140 | 4 | uint | Flags | Cell flags (bit 8=bridge, bit 9=bridge destroyed) |

---

## 5. Function Address Reference

| Address | Name | Confidence | Notes |
|---------|------|------------|-------|
| 0x005F4B10 | ObjectClass__DrawIt | HIGH | Base DrawIt, vtable+0x104 |
| 0x005F4D10 | ObjectClass__MarkNeedsRedraw | HIGH | Sets NeedsRedraw=true, vtable+0x134 |
| 0x005F4D30 | ObjectClass__Conceal | HIGH | Leave map -> limbo, vtable+0xD4 |
| 0x005F4EC0 | ObjectClass__Reveal | HIGH | Enter map from limbo, vtable+0xD8 |
| 0x005F5850 | ObjectClass__Mark | HIGH | Mark PUT/REMOVE/CHANGE, vtable+0x124 |
| 0x005F5940 | ObjectClass__Unlimbo_Full | HIGH | Extended unlimbo with drop anim, vtable+0xE8 |
| 0x005F60A0 | ObjectClass__Mark_Put | HIGH | Set bit 0x40 on cell, vtable+0xF0 |
| 0x005F6120 | ObjectClass__Mark_Remove | HIGH | Clear bit 0x40 on cell, vtable+0xF4 |
| 0x005F65F0 | ObjectClass__UnInit | HIGH | Destroy + conceal + mark dead, vtable+0xF8 |
| 0x005F4730 | ObjectClass__GetDrawExtent | HIGH | Compute draw bounding box, vtable+0x128 |
| 0x005F4870 | ObjectClass__GetDrawRect | HIGH | Compute full draw rect, vtable+0x12C |
| 0x005F6BD0 | ObjectClass__GetYSort | HIGH | Y-sort key for render order, vtable+0xB8 |
| 0x006D8DB0 | Tactical_ObjectRenderingLoop | HIGH | Main object render dispatcher |
| 0x006F5190 | TechnoClass__DrawExtras | HIGH | Selection brackets, health, pips |
| 0x006F5EF0 | TechnoClass__DrawBracketCorner | HIGH | Individual bracket corner |
| 0x006F64A0 | TechnoClass__DrawHealthBar | HIGH | Health bar pips |
| 0x00706640 | TechnoClass__Draw | HIGH | Main TechnoClass draw dispatcher |
| 0x00706ED0 | TechnoClass__Render | HIGH | SHP rendering with palette/remap |
| 0x006F6CA0 | TechnoClass__Unlimbo | HIGH | Full TechnoClass unlimbo (vision, house, etc.) |
| 0x00737BA0 | UnitClass__Unlimbo | HIGH | UnitClass unlimbo (facing, anim) |
| 0x00744470 | UnitClass__Draw_It | HIGH | UnitClass vtable+0x104 override |
| 0x004144B0 | AircraftClass__Draw_It | HIGH | AircraftClass vtable+0x104 override |
| 0x00422CA0 | AnimClass__DrawIt | HIGH | AnimClass vtable+0x104 override |
| 0x004E0240 | BuildingClass__Draw | HIGH | BuildingClass draw entry |
| 0x0043D290 | BuildingClass_DrawBody | HIGH | Building body SHP rendering |
| 0x0047E8A0 | CellClass__AddContent | HIGH | Add object to cell linked list |
| 0x0047EA90 | CellClass__RemoveContent | HIGH | Remove object from cell linked list |
| 0x00481180 | CellClass__PlaceInfantryInCell | HIGH | Infantry sub-cell placement |
| 0x007441B0 | ObjectClass__Mark_Occupation | HIGH | Set bit 0x20 for vehicles |
| 0x00744210 | ObjectClass__Clear_Occupation | HIGH | Clear bit 0x20 for vehicles |
| 0x004A9720 | DisplayClass__Submit_Object | HIGH | Add to display layer |
| 0x004A9770 | DisplayClass__RemoveFromLayer | HIGH | Remove from display layer |
| 0x004AED70 | CC_Draw_Shape | HIGH | Core SHP blitter (153 xrefs) |

---

## 6. Inheritance Notes

**What ObjectClass provides (base for ALL visible objects):**
- DrawIt base (visibility checks, coord transform, dispatch to vtable+0x114)
- Conceal/Reveal (enter/leave map)
- Mark (cell grid registration with IsMarked)
- Mark_Put/Mark_Remove (cell occupation bit 0x40)
- GetDrawExtent/GetDrawRect (screen bounds computation)
- GetYSort (render order key)
- NextObject linked list pointer for cell occupancy

**What TechnoClass adds (units, buildings, infantry, aircraft):**
- Full Unlimbo with vision, house tracking, sensor arrays, facing
- Draw with cloak/IC/temporal visual state management
- DrawExtras with selection brackets, health bars, veterancy pips
- Mark_Occupation/Clear_Occupation (vehicle bit 0x20 -- from overriding vtable+0xF0/0xF4)
- VXL cache blit path alongside SHP rendering

**What subclasses override:**
- UnitClass: Unlimbo (facing, tread animation), Draw_It, Mark
- InfantryClass: Sub-cell positioning, prone/crawl draw states
- BuildingClass: Foundation-aware drawing, turret/overlay layers, garrison fire
- AircraftClass: Flight layer management, shadow projection
- AnimClass: Flat/non-flat draw modes, owner attachment, independent DrawIt
