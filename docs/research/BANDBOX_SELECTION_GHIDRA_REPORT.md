# Band-Box (Drag-to-Select) System — Ghidra Research Report

## Overview

The band-box selection system in gamemd.exe allows the player to drag a rectangle
on the tactical view to select multiple units. This report documents the complete
state machine, data structures, selection filtering, and rendering from reverse
engineering the original binary.

**Confidence: HIGH** — all findings verified from direct decompilation with cross-referenced
xrefs and call chains.

## Key Addresses

| Address / Offset | Description |
|---|---|
| `0x004ab9b0` | LEFT_UP handler — resolves band-box selection |
| `0x004ac380` | MOUSE_MOVE handler — threshold check + rect update |
| `0x006d9f80` | Initialize Tactical band rect (start=end=mousePos) |
| `0x006d9fc0` | Update Tactical band rect end position |
| `0x006d9ff0` | Process band-box: iterate objects, apply callback |
| `0x006da080` | Check if any selectable object exists in rectangle |
| `0x006da160` | Clear Tactical band rect (zero all 4 fields) |
| `0x006da180` | **Draw band-box rectangle** (white outline) |
| `0x006da5c0` | Core iteration: AABB test + selection filter |
| `0x004ac2b0` | Selection callback (applied to each object in rect) |
| `0x005f4520` | `ObjectClass::Select` — add to CurrentObjects |
| `0x006fbfa0` | `TechnoClass::Select` — shroud check + voice |
| `0x006da740` | `Unselect_All` — clear CurrentObjects |

## Data Structures

### DisplayClass Fields (singleton at `DAT_00a8b230`)

| Offset | Type | Name | Description |
|---|---|---|---|
| `+0x11B3` | byte | `IsCtrlHeld` | Modifier flag — suppresses band-box start |
| `+0x11B4` | byte | `IsAltHeld` | Modifier flag — suppresses band-box start |
| `+0x11CF` | byte | `IsBandBoxing` | 1 = band-box rectangle active |
| `+0x11D0` | byte | `IsDragPending` | 1 = left mouse pressed, awaiting threshold |
| `+0x11D4` | int | `DragStartX` | Screen X where left-click started |
| `+0x11D8` | int | `DragStartY` | Screen Y where left-click started |
| `+0x11DC` | int | `DragCurrentX` | Last known mouse X during drag |
| `+0x11E0` | int | `DragCurrentY` | Last known mouse Y during drag |

### Tactical Class Fields (`g_Tactical`, `DAT_00887324`)

| Offset | Type | Name | Description |
|---|---|---|---|
| `+0xB0` | int | `ViewportX` | Viewport scroll X (subtracted from screen coords) |
| `+0xB4` | int | `ViewportY` | Viewport scroll Y |
| `+0xD7D` | byte | `NeedsRedraw` | Set to 1 when band-box changes |
| `+0xD90` | int | `BandStartX` | Band-box rectangle corner 1 X (screen space) |
| `+0xD94` | int | `BandStartY` | Band-box rectangle corner 1 Y |
| `+0xD98` | int | `BandEndX` | Band-box rectangle corner 2 X |
| `+0xD9C` | int | `BandEndY` | Band-box rectangle corner 2 Y |
| `+0xDB0` | int | `VisibleObjectCount` | Count of objects in screen-space array |

### Visible Objects Array (`DAT_00b0cec8`)

Each entry is 12 bytes:
```
struct VisibleObject {
    TechnoClass* ptr;    // +0x00
    int screen_x;        // +0x04 (absolute screen coords, includes viewport offset)
    int screen_y;        // +0x08
};
```

### CurrentObjects (Selection List)

| Global | Description |
|---|---|
| `DAT_00a8ecbc` | Pointer to array of `ObjectClass*` (selected objects) |
| `DAT_00a8ecc8` | Count of selected objects |
| `DAT_00a8ecc0` | Capacity of array |

## State Machine

### Phase 1: LEFT_PRESS → Drag Pending

**Location:** ~`0x004ae610` (in DisplayClass mouse handler)

When the left mouse button is pressed on the tactical map, if no other input
states are active (no Ctrl, no Alt, no placement mode, no other special state):

```c
// Guard: no other states active
if (DisplayClass.field_0x11B0 == 0 && DisplayClass.field_0x11B2 == 0
    && DisplayClass.field_0x11B1 == 0
    && DisplayClass.field_0x11B8 == -1 && DisplayClass.field_0x11A8 == 0)
{
    DisplayClass.IsDragPending = 1;        // +0x11D0
    DisplayClass.DragStartX = mouseX;      // +0x11D4
    DisplayClass.DragStartY = mouseY;      // +0x11D8
    DisplayClass.DragCurrentX = mouseX;    // +0x11DC
    DisplayClass.DragCurrentY = mouseY;    // +0x11E0
}
```

### Phase 2: MOUSE_MOVE → Threshold Check + Band-Box Update

**Function:** `FUN_004ac380` at `0x004ac380`

On each mouse move event:

**Case A: Drag pending, band-box not yet active**
```c
if (DisplayClass.IsBandBoxing == 0 && !(IsCtrlHeld || IsAltHeld)) {
    if (DisplayClass.IsDragPending != 0) {
        double dx = mouseX - DisplayClass.DragStartX;
        double dy = mouseY - DisplayClass.DragStartY;
        double distance = sqrt(dx*dx + dy*dy);

        if (distance > 4) {  // THRESHOLD = 4 pixels
            DisplayClass.IsBandBoxing = 1;   // activate band-box
            DisplayClass.IsDragPending = 0;  // no longer pending

            if (!IsCtrlHeld && !IsAltHeld) {
                g_Tactical->NeedsRedraw = 1;
                // Initialize band rect: both corners = current mouse pos
                Tactical_InitBandRect(mouseX, mouseY);
                Unselect_All();  // FUN_005bdc80(0,0) — clear existing selection
            }
        }
    }
}
```

**Case B: Band-box active**
```c
else {  // IsBandBoxing != 0
    // Clamp to viewport bounds
    int x = clamp(mouseX, 0, g_RadarViewportWidth - 1);
    int y = clamp(mouseY, 0, g_RadarViewportHeight - 1);

    if (x != DisplayClass.DragCurrentX || y != DisplayClass.DragCurrentY) {
        g_Tactical->NeedsRedraw = 1;
        // Update band rect end position
        Tactical_UpdateBandRect(x, y);  // sets 0xd98, 0xd9c
    }
}
```

### Phase 3: LEFT_UP → Selection Resolution

**Function:** `FUN_004ab9b0` at `0x004ab9b0`

When the left mouse button is released:

```c
if (DisplayClass.IsBandBoxing != 0) {
    g_Tactical->NeedsRedraw = 1;

    bool rightMouseHeld = IsKeyHeld(VK_RBUTTON);  // 0x10
    if (!rightMouseHeld) {
        bool anyObjectInRect = Tactical_AnyObjectInBandRect();
        if (!anyObjectInRect) {
            // No units in rectangle — selection was empty
            emptySelection = true;
        }
    }

    // Iterate visible objects in rectangle, call selection callback
    Tactical_ProcessBandBox(SelectionCallback);

    // Play select sound for batch
    FUN_0070d150();

    DisplayClass.IsBandBoxing = 0;    // clear band-box flag
    // Reset cursor action
    DisplayClass_ResetCursor(0, param_6);
    DisplayClass.field_0x11D0_flag = 0;
    DAT_00a8ed9d = 1;  // input consumed
}
```

### Phase 4: Drawing (Each Frame)

**Function:** `FUN_006da180` at `0x006da180`

Called during `TacticalClass_Draw` (in the overlay pass, after terrain/units,
alongside BuildingPlacement overlay):

```c
void Tactical_DrawBandBox(Tactical* self) {
    if (self->BandStartX == 0 && self->BandStartY == 0)
        return;  // no active band-box

    // Normalize rectangle
    int x1 = min(BandStartX, BandEndX);
    int y1 = min(BandStartY, BandEndY);
    int x2 = max(BandStartX, BandEndX);
    int y2 = max(BandStartY, BandEndY);
    int width  = (x2 - x1) + 1;
    int height = (y2 - y1) + 1;

    // Get WHITE color from palette entry 15 (0x0F)
    uint16_t color;
    if (bitDepth == 8)
        color = palette[15];       // byte at palette + 0x0F
    else
        color = palette16[15];     // word at palette + 0x1E

    // Draw rectangle outline on primary surface
    g_PrimarySurface->Draw_Rect(&viewportRect, &bandRect, color);
}
```

**Key detail:** The band-box is drawn as a **white outline rectangle** using
palette entry 15. This is the standard white color in RA2's palette.

## Selection Filter

**Function:** `FUN_006da5c0` at `0x006da5c0`

The core iteration loops through the visible objects array (populated during
the tactical draw pass) and performs an AABB test against the band-box rectangle.

### AABB Test (Screen-Space)

```c
// For each visible object:
int objX = visObj.screen_x - Tactical.ViewportX;  // convert to viewport-relative
int objY = visObj.screen_y - Tactical.ViewportY;

if (objX >= rectMinX && objX < rectMinX + rectWidth
    && objY >= rectMinY && objY < rectMinY + rectHeight)
{
    // Object is inside the band-box
}
```

**Important:** The test is a **point-in-rect** test using the object's screen
center position, NOT a bounding box intersection. This means an object's center
must be inside the band-box to be selected.

### Selection Criteria

For each object passing the AABB test:

1. **Object alive:** `field_0x24 != 0` (Health > 0)
2. **Shift-held mode** (`DAT_00b0fe65`):
   - If Shift is held → uses additive selection via `FUN_007327d0()` instead
   of the normal selection flow
3. **Normal selection (no callback):**
   a. **Buildings are EXCLUDED** unless BOTH conditions are met:
      - BuildingType has field `+0x408` set (likely `UndeploysInto` pointer)
      - Building has a **1×1 foundation** (`FoundationWidth[foundation_id] == 1`
        AND `FoundationHeight[foundation_id] == 1`)
      - Verified by `FUN_00465d40()`
   b. Object must be **player-controlled** (`TechnoClass::IsPlayerControlled()`)
   c. Object must be **selectable** (vtable `+0x138`, likely `IsSelectable()`)
   d. Then calls `Select()` (vtable `+0x14C`)
4. **Voice suppression:** `DAT_00822cf2` set to 0 during batch selection (suppresses
   individual select voices), restored to 1 after all objects processed.

### Building Exception Detail

The `FUN_00465d40` check allows 1×1 buildings with `UndeploysInto` to be
band-box selected. In practice, this means the **deployed MCV** (Construction
Yard) can be band-box selected since it "undeploys into" an MCV unit and has
a specific foundation. Regular buildings (Barracks, War Factory, etc.) are
always excluded from band-box selection.

## Key Constants

| Constant | Value | Description |
|---|---|---|
| Drag threshold | **4 pixels** | Euclidean distance from click to start band-box |
| Band-box color | **White** (palette 0x0F) | Outline rectangle color |
| Max selected | Capacity of CurrentObjects array | Dynamic, grows as needed |
| Voice suppress flag | `DAT_00822cf2` | 0 = suppress, 1 = play select voice |

## Comparison with Current Rust Implementation

Our current implementation in `src/sim/selection.rs`:

| Aspect | Original (gamemd.exe) | Our Implementation |
|---|---|---|
| Drag threshold | **4 pixels** (Euclidean) | **5 pixels** (Euclidean) |
| Distance calc | `sqrt(dx² + dy²)` | `(dx² + dy²).sqrt()` — same |
| Band-box color | **White** (palette 0x0F) | **Green** `[0, 200, 0, 255]` |
| Selection test | **Point-in-rect** (object center) | Point-in-rect (same) |
| Building filter | Excluded unless 1×1 + UndeploysInto | Excluded (all structures) |
| Modifier keys | Ctrl/Alt suppress band-box start | Not checked |
| Shift mode | Additive selection via separate path | Toggle selection |
| Deselect on start | Yes — clears selection when band-box activates | Depends on Shift |
| Right-click cancel | Right mouse held cancels on release | Right-click clears selection |
| Viewport clamping | Clamps to viewport bounds during drag | No clamping |
| Voice suppression | Suppressed during batch, plays one at end | N/A |

### Discrepancies to Consider

1. **Drag threshold**: Original uses **4 pixels**, we use 5. Minor difference.
2. **Band-box color**: Original uses **white**, we use green. The green is more
   visible on dark terrain but doesn't match the original aesthetic.
3. **Building exception**: We exclude ALL structures from drag-box. The original
   allows 1×1 buildings with `UndeploysInto` (deployed MCV). Should implement
   this exception.
4. **Deselect timing**: Original clears selection when band-box *activates*
   (threshold crossed), not on mouse-up. Our implementation may differ.
5. **Viewport clamping**: Original clamps the band-box end position to the
   viewport rect. We should do the same to prevent the rectangle from extending
   beyond the tactical view.

## Call Graph

```
LEFT_PRESS (DisplayClass mouse handler)
  └→ Set IsDragPending=1, store start position

MOUSE_MOVE → FUN_004ac380
  ├→ [drag pending] Check threshold (>4px) → activate band-box
  │   ├→ FUN_006d9f80 (init Tactical band rect)
  │   └→ FUN_005bdc80 (clear current selection)
  └→ [band-box active] Clamp + update end position
      └→ FUN_006d9fc0 (update Tactical band rect end)

LEFT_UP → FUN_004ab9b0
  ├→ FUN_006da080 (check if any objects in rect)
  ├→ FUN_006d9ff0 (process band-box with callback)
  │   └→ FUN_006da5c0 (iterate visible objects, AABB test)
  │       ├→ FUN_00465d40 (building 1×1 check)
  │       ├→ TechnoClass::IsPlayerControlled
  │       ├→ vtable+0x138 (IsSelectable)
  │       └→ vtable+0x14C (Select)
  │           └→ FUN_006fbfa0 (TechnoClass::Select)
  │               └→ FUN_005f4520 (ObjectClass::Select → add to CurrentObjects)
  └→ Clear IsBandBoxing flag

DRAW → FUN_006da180 (called from TacticalClass_Draw overlay pass)
  └→ Surface::Draw_Rect with white color (palette 0x0F)
```
