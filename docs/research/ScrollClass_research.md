# ScrollClass — Viewport/Map Scrolling System

Reverse-engineered via Ghidra MCP (live decompilation of `gamemd.exe`).
Confidence: HIGH for struct layout and methods, MEDIUM for some behavioral nuances.

---

## 1. Class Hierarchy

The display/input class hierarchy is a single-inheritance chain. Each class adds
fields to a monolithic instance (the global `g_DisplayChain`). ScrollClass sits
near the top, between TabClass and MouseClass:

```
GScreenClass        (base)       vtable at 0x7EA6FC     size: 0x10
  └─ MapClass                    vtable at 0x7EB498
    └─ DisplayClass              vtable at 0x7EB860
      └─ RadarClass              vtable at 0x7EC2BC
        └─ PowerClass            vtable at 0x7ECEF8
          └─ SidebarClass        vtable at 0x7ED598
            └─ TabClass          vtable at 0x7F0F0C
              └─ ScrollClass     vtable at 0x7F1094     fields end around 0x555B
                └─ MouseClass    vtable at 0x7F1230
```

RTTI string: `.?AVScrollClass@@` at 0x00816C78.

ScrollClass also inherits from `INoticeSink` (secondary vtable for notification callbacks).

### Secondary vtable (INoticeSink)

- Primary vtable: stored at offset 0x0000 in the object
- Secondary vtable (INoticeSink): stored at offset **0x5518** in the object
  - ScrollClass secondary: 0x7F108C
  - Contains: 0x006D1770 (destructor thunk), 0x00809060 (typeinfo)

---

## 2. Struct Layout — ScrollClass-Specific Fields

All offsets are from the base of the monolithic display chain object.
ScrollClass constructor at **0x00692290** initializes these fields:

| Byte Offset | Size | Default | Field Name (inferred) | Notes |
|-------------|------|---------|----------------------|-------|
| 0x5518 | 4 | vtable ptr | INoticeSink vtable | Secondary vtable for notifications |
| 0x5548 | 4 | 0 | EdgeScrollCoastLevel | Current coast acceleration level (0-8) |
| 0x554C | 1 | 0 | IsRMBDragging | Right-mouse-button drag scroll active |
| 0x5550 | 4 | 0 | RMBDragStartX | Mouse X position when RMB drag started |
| 0x5554 | 4 | 0 | RMBDragStartY | Mouse Y position when RMB drag started |
| 0x5558 | 1 | 0 | RMBDragThresholdMet | Whether drag exceeded system drag threshold |
| 0x5559 | 1 | 1 | AutoScrollEnabled | Edge-of-screen scroll active (default: enabled) |
| 0x555A | 1 | 0 | ScrollInhibited | If true, all scrolling is suppressed |

**MouseClass** (subclass) adds fields starting at:
- 0x555C (param_1[0x1557]): mouse state flags
- 0x5560-0x5568: additional mouse tracking fields

### CreditsClass Embedded Object

At offset **0x551C**, there's an embedded `CreditsClass` object used for the
sidebar credits display.

---

## 3. Key Methods

### 3.1 Constructor — 0x00692290

```
ScrollClass::Constructor()
  -> INoticeSink::Constructor()  (which calls SidebarClass -> TabClass constructors)
  -> Zero-init all scroll fields
  -> Set AutoScrollEnabled = 1
  -> Set primary vtable = 0x7F1094
  -> Set secondary vtable (at +0x5518) = 0x7F108C
```

### 3.2 vtable[10] (0x28): Input Handler — 0x006922E0

Dispatches to `FUN_00692F30` (scroll input processing), then to
`CommandBar_Dispatch` for other UI input.

**FUN_00692F30** (0x00692F30) — The main per-frame scroll input handler:

1. Gets mouse position relative to viewport
2. Calls `FUN_0063AB60` to check if mouse is over radar minimap
3. If `ScrollInhibited` (0x555A == 1): handles RMB drag scroll via `FUN_00693440`
4. Otherwise: performs hover/cursor detection (`FUN_00692300`)
5. If `AutoScrollEnabled` (0x5559 != 0) and not paused: calls `FUN_00692B60`
   for edge-of-screen scroll

### 3.3 Edge Scroll — FUN_00692B60

**Address:** 0x00692B60  
**Confidence:** HIGH

This is the primary edge-of-screen scroll handler. Called every frame when the
mouse is near screen edges.

**Edge detection thresholds** (stored as doubles at 0x83E738):
- X threshold: 0.16 (16% of screen width from edges)
- Y threshold: 0.21 (21% of screen height from edges)

The function checks whether the cursor is within 1 pixel of the screen edge or
within the sidebar area. If so, it computes a scroll direction using `atan2` from
screen center to cursor position, then converts to an 8-direction index.

**Coast acceleration mechanic:**
- `EdgeScrollCoastLevel` (offset 0x5548) increases each time the edge scroll
  timer expires (checked via `GetRadarTimer()`)
- Max coast level is capped at `8 - (ScrollRate + 1)`
  - With ScrollRate = 0 (slowest): max coast = 7
  - With ScrollRate = 6 (fastest): max coast = 1
- The coast level feeds into `FUN_004A9840` (the scroll execution method) as
  the scroll distance multiplier
- When the cursor leaves the edge zone, the coast level **gradually decreases**
  back to 0 (the timer-based deceleration creates "scroll coasting")

**Timer globals:**
- `DAT_00B05638`: Last scroll time (from GetRadarTimer)
- `DAT_00B05640`: Coast timer interval
- `DAT_00B0563C`: Last scroll direction

### 3.4 RMB Drag Scroll — FUN_00693440

**Address:** 0x00693440  
**Confidence:** HIGH

Handles right-mouse-button drag scrolling (the "grab and drag" scroll).

**Entry conditions:**
- `g_GameActive` must be true
- Either `DAT_00A8E378` (game in progress) or `DAT_00A8ED6B` (observer mode)
- `RMBDragThresholdMet` (0x5558) must be set (checked via `GetSystemMetrics(SM_CXDRAG/SM_CYDRAG)`)

**Scroll method** is determined by `FUN_005FBF70()` which reads `OptionsClass+0x0C`
(ScrollMethod field):
- **Method 0**: Standard — computes scroll delta from mouse movement, applies
  directly as `(ScrollRate + 1)` pixels per axis
- **Method 1**: Move cursor — same as method 0 but warps the cursor back to
  the drag start position each frame
- **Method 2**: Inverse — reverses the scroll direction (opposite of mouse movement)
  and warps cursor

**Edge acceleration:** When cursor is within 10 pixels of screen edge during drag,
the minimum delta is set to 5 and multiplied by 4, creating faster edge scrolling.

**Scroll direction:** The function computes X and Y deltas, then calls
`FUN_004A9840` (scroll execute) with direction indices:
- 0 = Up, 2 = Right, 4 = Down, 6 = Left

### 3.5 Scroll Execution — FUN_004A9840

**Address:** 0x004A9840  
**Confidence:** HIGH

The central scroll dispatch function, called by both edge scroll and RMB drag.

```c
bool Scroll(int direction, int *distance, bool execute) {
    if (distance == 0) {
        if (!execute) return false;
    } else if (!execute) {
        return CanScrollInDirection(direction);  // FUN_006DA230
    }
    ApplyScrollDelta(direction, *distance);      // FUN_006D8530
    return true;
}
```

### 3.6 Scroll Delta Application — FUN_006D8530

**Address:** 0x006D8530  
**Confidence:** HIGH

Applies a scroll delta to the viewport's desired position.

**Direction table** (lazy-initialized at 0xB0CE38, 8 entries of {dx, dy}):
```
Direction 0 (N):  { 0, -1}
Direction 1 (NE): { 1, -1}
Direction 2 (E):  { 1,  0}
Direction 3 (SE): { 1,  1}
Direction 4 (S):  { 0,  1}
Direction 5 (SW): {-1,  1}
Direction 6 (W):  {-1,  0}
Direction 7 (NW): {-1, -1}
```

Reads current viewport position from `TacticalClass+0xD64/0xD68` and adds
`direction_vector * distance` to the desired viewport at `TacticalClass+0xD74/0xD78`.

### 3.7 Scroll Inhibit Check — FUN_00693060

**Address:** 0x00693060 (vtable[26], offset 0x68)  
**Confidence:** HIGH

Returns true (scroll inhibited) if any of:
1. `ScrollInhibited` flag (0x555A) is set
2. `DAT_00A8ED9C` is set (some global inhibit, possibly loading screen)
3. The global `g_Tactical` object (at 0x887324) has a non-zero float at +0xD8
   that doesn't equal 0.0 — this is the **smooth camera animation speed**, meaning
   scroll is inhibited during camera pans (e.g., jump-to-event animations)

### 3.8 Stop Scroll — FUN_006938C0

**Address:** 0x006938C0 (vtable[35], offset 0x8C)  
**Confidence:** HIGH

Clears `ScrollInhibited` flag and releases mouse capture (`ReleaseCapture()`).
Called when stopping RMB drag.

---

## 4. Scroll Speed Mechanics

### 4.1 OptionsClass Scroll Settings

The `OptionsClass` instance lives at global **0x00A8EB60** and contains:

| Offset | Field | INI Key | Default | Range |
|--------|-------|---------|---------|-------|
| 0x0C | ScrollMethod | `ScrollMethod` | 0 | 0-2 |
| 0x10 | ScrollRate | `ScrollRate` | 3 | 0-6 |
| 0x14 | AutoScroll | `AutoScroll` | true | bool |

Read from `[Options]` section in settings INI via `OptionsClass::ReadFromINI`
(0x005FA6A0).

**ScrollRate** is stored as 0-6 where **0 = fastest, 6 = slowest**. The in-game
UI slider inverts it: `slider_position = 6 - ScrollRate`.

### 4.2 Scroll Speed Table

There is a hardcoded speed table at **0x0083E748** (9 entries):

| Index | Pixels per scroll step |
|-------|----------------------|
| 0 | 448 |
| 1 | 384 |
| 2 | 320 |
| 3 | 256 |
| 4 | 192 |
| 5 | 128 |
| 6 | 64 |
| 7 | 32 |
| 8 | 16 |

The effective speed depends on the coast level (see edge scroll acceleration).

### 4.3 RulesClass ScrollMultiplier

- **INI key:** `[AudioVisual] > ScrollMultiplier`
- **Default value:** 0.07
- **RulesClass offset:** 0x5B8 (as double, 8 bytes)
- **Usage:** Applied as a multiplier to the base scroll distance during RMB drag
  scroll. The formula involves `(ScrollRate + 1) * abs(mouseDelta) * ScrollMultiplier`.

### 4.4 Edge Detection Constants

Stored as doubles at **0x0083E738**:
- X edge fraction: **0.16** — cursor within 16% of screen width from left/right
  edge triggers scroll
- Y edge fraction: **0.21** — cursor within 21% of screen height from top/bottom
  edge triggers scroll

These define three zones per axis:
- Near edge (< fraction): scroll in that direction
- Middle (fraction to 1-fraction): screen center (mapped to 50%)
- Far edge (> 1-fraction): scroll in opposite direction

---

## 5. Viewport Update Pipeline

### 5.1 TacticalClass Viewport Fields

All offsets relative to the TacticalClass instance (global at **0x00887324**):

| Offset | Type | Field |
|--------|------|-------|
| 0xA8 | int | LastUpdateFrame — frame counter for once-per-frame guard |
| 0xB0 | int | ViewportRectLeft — pixel rect left (computed from 0xD64) |
| 0xB4 | int | ViewportRectTop — pixel rect top (computed from 0xD68) |
| 0xB8 | int | ViewportRectRight — previous frame's rect left (for delta) |
| 0xBC | int | ViewportRectBottom — previous frame's rect top (for delta) |
| 0xC0 | double | SmoothScrollTarget (8 bytes) — camera animation target |
| 0xC8-0xCF | double | SmoothScrollStart — camera animation start position |
| 0xD0-0xD7 | double | SmoothScrollDelta — distance to travel |
| 0xD8 | float | SmoothScrollSpeed — animation speed (0.0 = no animation) |
| 0xDC | float | SmoothScrollProgress — current interpolation t (0.0 to 1.0) |
| 0xD64 | int | ViewportX — current viewport center X (pixel coordinates) |
| 0xD68 | int | ViewportY — current viewport center Y (pixel coordinates) |
| 0xD6C | int | PrevViewportX — last-drawn viewport X (for dirty detection) |
| 0xD70 | int | PrevViewportY — last-drawn viewport Y (for dirty detection) |
| 0xD74 | int | DesiredViewportX — scroll target X (where scroll wants to go) |
| 0xD78 | int | DesiredViewportY — scroll target Y (where scroll wants to go) |
| 0xD7C | bool | ForceFullRedraw — triggers complete redraw next frame |
| 0xD7D | bool | ViewportMoved — set when viewport position changed |
| 0xD7E | bool | (unknown flag) |
| 0xD80 | int | CellOriginX — viewport origin in cell-space (after iso transform) |
| 0xD84 | int | CellOriginY — viewport origin in cell-space |
| 0xD88 | int | CellCountX — viewport width in cells (ViewportWidth / 60 + 2) |
| 0xD8C | int | CellCountY — viewport height in cells (ViewportHeight / 15 + 4) |
| 0xDA0 | int | FrameTickCounter — increments each frame |
| 0xDA4 | int | LastTickTime — from GetRadarTimer |
| 0xDA8 | int | (timer-related) |
| 0xDAC | int | TickInterval — from RulesClass+0x50 |
| 0xDB0 | int | VisibleBuildingCount — count for phase 2 rendering |

### 5.2 Per-Frame Update Flow

The viewport update happens in three stages per game frame:

**Stage 1: Input Processing** — `GScreenClass::Input` (via `Main_Tick`)
- `ScrollClass::Input` (vtable[10], 0x006922E0) dispatches to `FUN_00692F30`
- Edge scroll (`FUN_00692B60`) or RMB drag (`FUN_00693440`) writes to
  `TacticalClass.DesiredViewportX/Y` via `FUN_006D8530`
- Keyboard scroll commands also call `FUN_004A9840` → `FUN_006D8530`

**Stage 2: Viewport Application** — `TacticalClass::Update` (0x006D2540)
- Called via `(*g_Tactical->vtable[0x5C])()` from `Main_Tick`
- If `DesiredViewportX/Y != ViewportX/Y`:
  1. Clamps desired position to map bounds via `FUN_006D8640`
  2. Sets `ViewportX/Y = DesiredViewportX/Y = clamped_position`
  3. Calls `FUN_006D8B30` to recompute ViewportRect and cell-space origin
  4. Sets `ViewportMoved = 1`
- Also handles smooth camera animation (e.g., jump-to-event): interpolates
  using `SmoothScrollProgress` and directly sets both current and desired

**Stage 3: Render** — `RenderFrame_main` (0x004F4480) → `TacticalClass_Draw`
- **Pass 0** (`param_3 == 0`): Computes scroll delta from old vs new ViewportRect
  - If delta > 0: blits old frame content, scrolls ABuffer and ZBuffer circularly,
    swaps composition/back surfaces
  - If full redraw needed: clears ABuffer and ZBuffer entirely
  - Updates `PrevViewportX/Y` = `ViewportX/Y`
  - Returns early (no drawing)
- **Pass 1**: Draws terrain into newly exposed strips
- **Pass 2**: Draws objects onto composition surface

### 5.3 Map Bounds Clamping — FUN_006D8640

**Address:** 0x006D8640  
**Confidence:** HIGH

Constrains viewport position to stay within the playable map area.

```
MinX = ViewportWidth/2 + (MapBoundX * 2 - MapWidth) * 30
MaxX = MinX + MapCellsX * 60 - ViewportWidth
MinY = (MapWidth - 5 + MapBoundY * 2) * 15 + ViewportHeight/2
MaxY = MinY + (MapCellsY * 60 + 270) / 2 - ViewportHeight
```

Uses map dimension globals:
- `DAT_0087F8DC`: MapWidth
- `DAT_0087F8E4`: MapBoundX
- `DAT_0087F8E8`: MapBoundY
- `DAT_0087F8EC`: MapCellsX
- `DAT_0087F8F0`: MapCellsY

Cell size constants: **60 pixels wide, 30 pixels half-width, 15 pixels half-height**
(standard RA2 isometric tile dimensions).

### 5.4 ViewportRect Recomputation — FUN_006D8B30

**Address:** 0x006D8B30  
**Confidence:** HIGH

Called after viewport position changes. Recomputes:
1. `ViewportRectLeft` = `ViewportX - ViewportWidth/2`
2. `ViewportRectTop` = `ViewportY - ViewportHeight/2`
3. Transforms top-left corner through the isometric 3x4 matrix to get cell-space
   origin (`CellOriginX/Y`)
4. Computes cell-space dimensions: `CellCountX = ViewportWidth / 60 + 2`,
   `CellCountY = ViewportHeight / 15 + 4`

---

## 6. Keyboard Scroll

Keyboard scroll commands go through the same `FUN_004A9840` → `FUN_006D8530`
path as edge/mouse scroll. The `Main_Tick` function at 0x0055D360 calls
`FUN_004A9840` directly for keyboard-initiated scrolls during the input
processing phase.

The direction parameter uses the same 8-direction encoding:
- 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW

Keyboard scroll does NOT use the coast acceleration mechanic — it applies
a fixed scroll distance per frame based on `ScrollRate + 1`.

---

## 7. Scroll Cursor Display

During RMB drag scroll, the mouse cursor is changed via vtable[18] (0x48)
call with a direction-based cursor index. The cursor mapping table is at
**0x0083E790** (indexed by a bitmask of blocked directions).

During edge scroll, the cursor is also updated to show the scroll direction
arrow. Direction index is computed from atan2 of screen center to cursor
position, quantized to 8 directions plus 1-based offset (indices 1-8 for
edge scroll cursors, 9-16 for "blocked" variants).

---

## 8. Global Variables Summary

| Address | Type | Purpose |
|---------|------|---------|
| 0x00887324 | TacticalClass* | g_Tactical — the singleton TacticalClass instance |
| 0x00A8EB60 | int | OptionsClass::GameSpeed |
| 0x00A8EB6C | int | OptionsClass::ScrollMethod (0/1/2) |
| 0x00A8EB70 | int | OptionsClass::ScrollRate (0-6, lower = faster) |
| 0x00A8EB74 | bool | OptionsClass::AutoScroll |
| 0x00A8ED6B | bool | Observer/replay mode flag |
| 0x00A8ED9C | bool | Global scroll inhibit (loading screen?) |
| 0x0083E738 | double | Edge scroll X threshold (0.16) |
| 0x0083E740 | double | Edge scroll Y threshold (0.21) |
| 0x0083E748 | int[9] | Scroll speed table (448..16 pixels) |
| 0x00B05638 | int | Edge scroll coast timer (GetRadarTimer) |
| 0x00B0563C | int | Edge scroll last direction |
| 0x00B05640 | int | Edge scroll coast interval |
| 0x00B05690 | int | Edge scroll raw angle |
| 0x00B0565C | int | Edge scroll quantized direction |
| 0x00B0CE28 | int | g_RadarViewportOffsetX |
| 0x00B0CE2C | int | g_RadarViewportOffsetY |
| 0x00B0CE30 | int | g_RadarViewportWidth |
| 0x00B0CE34 | int | g_RadarViewportHeight |

---

## 9. TS Legacy Notes

The `ScrollMethod` values 1 and 2 (cursor-warp and inverse drag) may be
Tiberian Sun holdovers. In standard YR, `ScrollMethod` defaults to 0 and
the in-game options UI does not expose scroll method selection (only scroll
speed and auto-scroll coasting). The "scroll coasting" checkbox in the options
dialog controls whether `AutoScroll` (0x5559) is enabled, not the coast
acceleration level.

The smooth camera animation system at TacticalClass offsets 0xD0-0xDC appears
to be used for jump-to-event camera pans (e.g., "your base is under attack"
camera movement). This is NOT the same as scroll smoothing — normal scrolling
is applied immediately each frame with no interpolation.

---

## 10. Key Addresses Summary

| Function | Address | Purpose |
|----------|---------|---------|
| ScrollClass::Constructor | 0x00692290 | Initializes scroll fields |
| ScrollClass::Input (vtable[10]) | 0x006922E0 | Input dispatch → scroll processing |
| FUN_00692F30 | 0x00692F30 | Main scroll input handler |
| FUN_00692B60 | 0x00692B60 | Edge-of-screen scroll + coast |
| FUN_00693440 | 0x00693440 | RMB drag scroll |
| FUN_004A9840 | 0x004A9840 | Scroll execution dispatch |
| FUN_006D8530 | 0x006D8530 | Apply scroll delta to desired viewport |
| FUN_006DA230 | 0x006DA230 | Check if scroll direction is valid |
| FUN_006D2540 | 0x006D2540 | TacticalClass::Update — applies desired→current |
| FUN_006D8640 | 0x006D8640 | Map bounds clamping |
| FUN_006D8B30 | 0x006D8B30 | Recompute viewport rect + cell origin |
| FUN_006938C0 | 0x006938C0 | Stop scroll / release capture |
| FUN_00693060 | 0x00693060 | Scroll inhibit check |
| FUN_00693840 | 0x00693840 | Keyboard scroll key handler |
| OptionsClass::ReadFromINI | 0x005FA6A0 | Reads ScrollRate, ScrollMethod, AutoScroll |
| OptionsClass::Constructor | 0x005FA350 | Default option values |
| RenderFrame_main | 0x004F4480 | Orchestrates 3-pass rendering |
| TacticalClass_Draw | 0x006D3D10 | Pass 0: scroll buffer mgmt, Pass 1+2: render |

---

## 11. FUN_00692300 — Hover/Cursor Detection (Under-Cursor Query)

**Address:** 0x00692300
**Confidence:** HIGH

This function determines what is under the mouse cursor for tooltip and action-cursor
purposes. It is called from the main scroll input handler (FUN_00692F30) when scroll
is NOT inhibited and the mouse is NOT over the radar minimap.

### Signature (reconstructed)

```c
bool ScrollClass__QueryCursorTarget(
    int *screenPos,          // [in]  mouse screen position {x, y}
    short *outCellXY,        // [out] cell coordinate under cursor
    int *outCellCoords,      // [out] 3D cell position (x, y, z)
    int *outObject,          // [out] pointer to AbstractClass* under cursor (or 0)
    char *outFogStatus,      // [out] 1 if cell is under fog-of-war
    char *outShroudStatus    // [out] 1 if cell is shrouded
);
```

### Logic flow

1. **Early exit:** If mouse X or Y < 0, returns 0 (invalid position).

2. **Screen-to-cell conversion:** Adds radar viewport offset to mouse coords, then
   calls `FUN_006d6590` (TacticalClass::ScreenToCell) and `FUN_006d2280`
   (TacticalClass::ScreenTo3DCoord) to get the cell and 3D position under cursor.

3. **Cell validity check:** Calls `FUN_00568350` to verify the cell is within the
   valid map area. Returns 0 if invalid.

4. **Ground height query:** Gets the ground height at the cell center via
   `CellClass__GetGroundHeight`.

5. **Shroud check:** Calls `IsShrouded()` on the cell position. Outputs shroud
   status to `outShroudStatus`.

6. **Fog-of-war check:** Only runs if `*DAT_00a8b230 & 0x1000` (fog-of-war enabled
   flag). If fog is active, checks for buildings and calls `FUN_005865e0` (fogged
   status query). Outputs result to `outFogStatus`.
   **TS LEGACY NOTE:** The `& 0x1000` flag is the fog-of-war feature gate. This is
   off by default in YR. The fog check code only executes when fog is explicitly
   enabled.

7. **Object picking:** If neither shrouded nor fogged, calls
   `Tactical__PickObjectAtScreenPoint` to find the topmost game object under the
   cursor. Result stored in `outObject`.

8. **Object filtering:** The picked object goes through several filters:
   - Checks `object+0x41a` (IsDiscoveredByPlayer flag) — if not discovered, cleared
   - For buildings (`WhatAmI == 2`): checks sensor range for the player's house
   - For other objects: checks `TypeClass+0xC9A` (Insignificant flag)
   - For terrain objects (`WhatAmI == 6`): additional sensor and cloaking checks

9. **Return:** Always returns 1 if the cell was valid, even if no object was found.
   Returns 0 only for invalid positions or out-of-map cells.

### How it's used in FUN_00692F30

```c
// After calling FUN_00692300:
if (success) {
    action = DisplayClass__DetermineAction(cellXY, object, 1);
    DisplayClass__SetCursorFromAction(cellXY, shroudStatus, object, action, 0);
}
```

The action determination feeds into cursor shape selection — this is how the game
shows attack cursors, move cursors, enter cursors, etc. when hovering over objects.

---

## 12. FUN_006DA230 — CanScrollInDirection (Full Decompilation)

**Address:** 0x006DA230
**Confidence:** HIGH

Tests whether scrolling in a given direction is possible (not blocked by map bounds).
Returns either the original direction or a remapped "slide" direction when partially
blocked.

### Reconstructed logic

```c
int __thiscall TacticalClass__CanScrollInDirection(int this, int direction) {
    // Lazy-init direction vector table (8 directions, {dx,dy} pairs)
    static int dir_vectors[16];  // at 0x00B0CD98
    if (!initialized) {
        dir_vectors = {
            { 0, -1},  // 0: N
            { 1, -1},  // 1: NE
            { 1,  0},  // 2: E
            { 1,  1},  // 3: SE
            { 0,  1},  // 4: S
            {-1,  1},  // 5: SW
            {-1,  0},  // 6: W
            {-1, -1},  // 7: NW
        };
    }

    // Compute test position: current viewport + direction vector
    int testX = dir_vectors[direction].dx + this->ViewportX;   // +0xD64
    int testY = dir_vectors[direction].dy + this->ViewportY;   // +0xD68

    // If test position is same as current (shouldn't happen), return direction as-is
    if (testX == this->ViewportX && testY == this->ViewportY)
        return direction;

    // Clamp test position to map bounds
    int clampedX = testX, clampedY = testY;
    bool wasClamped = FUN_006d8640(&clampedX, &clampedY);

    // In observer mode, skip clamping (allow scrolling anywhere)
    if (wasClamped && !DAT_00a8ed6b) {
        testX = clampedX;
        testY = clampedY;
    }

    // Compute delta: how much did clamping move us?
    int deltaX = this->ViewportX - testX;  // sign indicates which axis was blocked
    int deltaY = this->ViewportY - testY;

    // Look up remapped direction from the clamp-result table
    return REMAP_TABLE[deltaX + deltaY * 3];
}
```

### Remap table at 0x0084291C

This 9-entry table maps `(deltaX + deltaY * 3)` to a direction or -1 (fully blocked).
The deltas are the sign of `(current - clamped)` per axis:

| deltaX | deltaY | Index | Result | Meaning |
|--------|--------|-------|--------|---------|
| -1 | -1 | -4 | -1 | Blocked SE: completely blocked (corner) |
| 0 | -1 | -3 | 2 | Blocked S: redirect to E |
| 1 | -1 | -2 | 5 | Blocked SW: redirect to SW |
| -1 | 0 | -1 | 4 | Blocked E: redirect to S |
| 0 | 0 | 0 | (passthrough) | Not blocked: return original direction |
| 1 | 0 | 1 | (N/A) | Blocked W: (see note) |
| -1 | 1 | 2 | 1 | Blocked NE: redirect to NE |
| 0 | 1 | 3 | (N/A) | Blocked N: (see note) |
| 1 | 1 | 4 | 1 | Blocked NW: redirect to NE |

**Note:** When deltaX/deltaY are 0, the function returns the original `param_2`
(direction) unchanged — the table is only consulted when clamping actually occurred.
The function return replaces the direction parameter inline.

### Key insight

This function enables "wall sliding" — when you scroll diagonally into a corner,
instead of stopping completely, the scroll is redirected along the unblocked axis.
For example, scrolling NE into the top edge redirects to E-only scrolling.

---

## 13. Smooth Camera Animation System (TacticalClass offsets 0xC0-0xDC)

**Confidence:** HIGH

### Struct layout (refined)

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0xC0 | int | (unused/padding) | Not written by animation setup |
| 0xC4 | int | (unused/padding) | Not written by animation setup |
| 0xC8 | int | AnimStartX | Camera start X (viewport pixel coords at anim start) |
| 0xCC | int | AnimStartY | Camera start Y |
| 0xD0 | int | AnimTargetX | Camera destination X (pixel coords) |
| 0xD4 | int | AnimTargetY | Camera destination Y |
| 0xD8 | float | AnimSpeed | Per-frame progress increment (0.0 = no animation) |
| 0xDC | float | AnimProgress | Current interpolation t, 0.0 to 1.0 |

### Setup function: FUN_006D2420

**Address:** 0x006D2420
**Called from:** TriggerAction__Execute (0x006deea4), TeamClass__Recruit_Or_Add (0x006e9b57)

```c
void __thiscall TacticalClass__StartCameraAnim(
    int this,
    int *cellCoords,    // {cellX, cellY} — target map cell
    int speedIndex      // 0-4, indexes into speed table
) {
    this->AnimProgress = 0.0f;                    // +0xDC
    this->AnimSpeed = SPEED_TABLE[speedIndex];     // +0xD8

    // Convert cell coords to pixel position (isometric transform)
    int pixelX = (cellX * 60) / 2 + (cellY * -60) / 2;
    int pixelY = (cellX * 30) / 2 + (cellY * 30) / 2;
    pixelX = (pixelX + rounding) >> 8;
    pixelY = ((pixelY + rounding) >> 8) - AdjustForZ();

    this->AnimTargetX = pixelX;                    // +0xD0
    this->AnimTargetY = pixelY;                    // +0xD4

    // Save current viewport as animation start
    this->AnimStartX = this->ViewportX;            // +0xC8 = +0xD64
    this->AnimStartY = this->ViewportY;            // +0xCC = +0xD68
}
```

### Speed table at 0x008428EC (5 float entries)

| Index | Float value | Frames to complete | Use case |
|-------|-------------|-------------------|----------|
| 0 | 0.001500 | ~667 frames | Extremely slow pan |
| 1 | 0.003000 | ~333 frames | Slow pan |
| 2 | 0.007500 | ~133 frames | Medium pan |
| 3 | 0.030000 | ~33 frames | Fast pan (typical jump-to-event) |
| 4 | 0.060000 | ~17 frames | Very fast pan |

### Per-frame animation in TacticalClass::Update (0x006D2540)

The animation runs in the Update function every frame:

```c
// Guard: only run once per frame
if (this->LastUpdateFrame == g_CurrentFrameCounter) return;

// Is animation active?
if (DAT_00a8ed5c != 0 &&
    (this->AnimTargetX != DAT_00b0ce08 || this->AnimTargetY != DAT_00b0ce0c) &&
    this->AnimSpeed != 0.0f)
{
    // Advance progress
    this->AnimProgress += this->AnimSpeed;
    if (this->AnimProgress > 1.0f)
        this->AnimProgress = 1.0f;

    // LINEAR INTERPOLATION: lerp from start to target
    int newX = ftol(AnimStartX + (AnimTargetX - AnimStartX) * AnimProgress);
    int newY = ftol(AnimStartY + (AnimTargetY - AnimStartY) * AnimProgress);

    // When progress reaches 1.0, clear all animation fields
    if (this->AnimProgress >= 1.0f) {
        this->AnimTargetX = 0;
        this->AnimStartX = 0;
        this->AnimTargetY = 0;
        this->AnimStartY = 0;
        this->AnimSpeed = 0.0f;
        this->AnimProgress = 0.0f;
    }

    // Clamp to map bounds and apply
    clamp(&newX, &newY);
    this->ViewportX = this->DesiredViewportX = newX;
    this->ViewportY = this->DesiredViewportY = newY;
    FUN_006d8b30();  // recompute viewport rect
    this->ViewportMoved = 1;
}
```

### Interpolation function: FUN_0075F5C0

The interpolation helper at 0x0075F5C0 implements a simple **linear lerp** (no easing):

```asm
; param_1 = output {x, y}
; ECX = pointer to {startX, startY}   (AnimStart)
; EDX = pointer to {targetX, targetY} (AnimTarget)
; stack float = progress (AnimProgress)
;
; For each axis:
;   result = ftol( fild(target) * (1.0 - progress) + fild(start) * progress )
;
; Wait — actually looking more carefully:
;   FLD [ESP+8]          ; load progress
;   FSUBR [0x7e1718]     ; 1.0 - progress = invProgress
;   FILD [EBX]           ; load start.x (int -> float)
;   FMUL [ESP+0x18]      ; start.x * progress  ... wait
```

Re-examining the assembly more carefully:

```asm
FLD   float [ESP+8]       ; ST0 = progress
FSUBR double [0x7e1718]   ; ST0 = 1.0 - progress (note: the constant at 0x7e1718 = 1.0 as double)
; ... but progress is a float being subtracted from a double? 
; Actually FSUBR loads the double 1.0 and subtracts ST0: result = 1.0 - progress
MOV   EBX, [ESP+8]        ; EBX = pointer to AnimTarget
FILD  [EBX]               ; ST0 = target.x (int), ST1 = invProgress
FMUL  [ESP+0x18]          ; Hmm, this references a stack param

; The actual formula (from tracing the FPU stack):
; result.x = ftol(target.x * progress + start.x * (1.0 - progress))
; result.y = ftol(target.y * progress + start.y * (1.0 - progress))
```

**The easing function is LINEAR.** There is no ease-in/ease-out curve. The camera
moves at constant speed from start to target, with the speed determined by the
AnimSpeed value (progress increment per frame). The animation simply advances
`progress += speed` each frame until `progress >= 1.0`.

### Triggers

The smooth camera animation is triggered by:
1. **Trigger actions** (map scripting) — `TriggerAction__Execute` at 0x006deea4
2. **Team recruitment events** — `TeamClass__Recruit_Or_Add` at 0x006e9b57

These are both map/scripting systems. **Normal gameplay scrolling does NOT use
smooth animation** — it sets DesiredViewport directly and the update function
applies it immediately in the same frame. The smooth animation is exclusively for
scripted camera movements (e.g., "camera moves to location" trigger action).

### Scroll inhibition during animation

When `AnimSpeed != 0.0` (animation in progress), the scroll inhibit check
(FUN_00693060) returns true, preventing manual scrolling from interfering with
the camera pan.

---

## 14. FUN_00693840 — Keyboard Scroll Handler (Escape/Command Key)

**Address:** 0x00693840
**Confidence:** HIGH

This function is surprisingly simple — it is NOT the main keyboard scroll direction
handler. It handles the **Escape key** and other command-key responses during scroll
state.

### Decompiled logic

```c
void __thiscall ScrollClass__HandleKeyPress(int *this, int keycode) {
    // If scroll is inhibited (RMB drag active)
    if ((char)this[0x1556] != 0) {   // offset 0x5558 = RMBDragThresholdMet
        // Cancel the drag: reset cursor and clear inhibit
        (*vtable->SetCursor)(0, DAT_00884d44);  // restore default cursor
        this->RMBDragThresholdMet = 0;           // clear at 0x5558
        return;
    }
    // Otherwise delegate to FUN_004aad30 (general key command handler)
    FUN_004aad30(keycode);
}
```

### FUN_004AAD30 — General Key Command Dispatcher

**Address:** 0x004AAD30
**Confidence:** HIGH

This handles various keyboard commands by checking state flags in sequence:

1. **Active command target** (`this[0x469]`, offset 0x11A4): If a targeting command
   is pending and `flag & 1` is set, cancels the command and resets cursor.
2. **Sell mode** (`this[0x46C]`, offset 0x11B0): Calls `FUN_004ac8c0(0)` to cancel.
3. **Power toggle mode** (`+0x11B1`): Calls `FUN_004ac660(0)` to cancel.
4. **Repair mode** (`+0x11B2`): Calls `FUN_004ac820(0)` to cancel.
5. **Waypoint mode** (`this[0x46E]`, offset 0x11B8): Resets waypoint index to -1.
6. **Planning mode** (`+0x11B3`): Calls `FUN_004ac700(0, 0)` to cancel.
7. **Beacon mode** (`this[0x46D]`, offset 0x11B4): Calls `FUN_004ac960(0)` to cancel.
8. **Default/fallback**: Calls `Desync_Handler()` as last resort.

All paths end by resetting the cursor via `vtable[18](0, DAT_00884d44)`.

### Actual keyboard scroll direction handling

The **real** keyboard scroll direction handling is in `LogicClass__AI` (at 0x0055DEE0)
and `Main_Tick` (at 0x0055DD2A). See section 15 for details.

---

## 15. Keyboard Scroll Direction System (Full Analysis)

**Confidence:** HIGH

### Key-to-direction flag mapping (LogicClass__AI)

Keyboard scroll uses a **bitmask accumulator** at global **0x00ABCE14**. Arrow keys
set/clear bits on key-down/key-up:

| Key | VK code | Bit mask | Direction |
|-----|---------|----------|-----------|
| Up Arrow | 0x26 | 0x0001 | North |
| Down Arrow | 0x28 | 0x0010 | South |
| Left Arrow | 0x25 | 0x0100 | West |
| Right Arrow | 0x27 | 0x1000 | East |

**Key-down** (bit 0x800 NOT set in event): sets the corresponding bit via OR.
**Key-up** (bit 0x800 IS set in event): clears the corresponding bit via AND NOT.

The bit 0x800 in the input event word is the key-release flag.

### Diagonal support

**Yes, diagonal keyboard scroll is fully supported.** Since each direction is an
independent bit, pressing Up+Right simultaneously produces bitmask `0x1001`, which
the Main_Tick code processes as two separate scroll calls (North + East), achieving
diagonal movement.

### Scroll distance calculation (Main_Tick at 0x0055DCBE)

```c
// Check for arrow keys or scroll-wheel-like input
if (KeyPressed(DAT_00a8ec08) || KeyPressed(DAT_00a8ec0c)) {
    // Scroll speed based on some float multiplier
    distance = ftol(fild(something) * FLOAT_0082a034);
}
else if (KeyPressed(DAT_00a8ec00) || KeyPressed(DAT_00a8ec04)) {
    // Scroll speed based on map dimensions
    int dim = max(MapWidth, MapHeight);
    distance = dim << 8;  // dim * 256 pixels
}
```

### Scroll dispatch (Main_Tick at 0x0055DD2A)

```c
int flags = DAT_00abce14;

if (flags & 0x0100)  // Left Arrow held
    Scroll(6, &distance, 1);  // direction 6 = West

if (flags & 0x1000)  // Right Arrow held
    Scroll(2, &distance, 1);  // direction 2 = East

if (flags & 0x0001)  // Up Arrow held
    Scroll(0, &distance, 1);  // direction 0 = North

if (flags & 0x0010)  // Down Arrow held
    Scroll(4, &distance, 1);  // direction 4 = South
```

Each direction is checked independently, so simultaneous keys produce diagonal
movement by applying two orthogonal scrolls in the same frame.

---

## 16. Scroll Cursor Table at 0x0083E790

**Address:** 0x0083E790
**Confidence:** HIGH

16 entries of `int32`, indexed by a 4-bit direction bitmask. The bitmask encodes
which edges/directions are blocked or active during RMB drag scroll.

### Full table dump

| Index | Bits (SNWE) | Cursor ID | Cursor Name |
|-------|-------------|-----------|-------------|
| 0 | 0000 | 61 | Default/No scroll |
| 1 | 0001 | 62 | Scroll N (blocked indicator) |
| 2 | 0010 | 64 | Scroll E |
| 3 | 0011 | 63 | Scroll NE |
| 4 | 0100 | 66 | Scroll S |
| 5 | 0101 | 61 | N+S blocked = default |
| 6 | 0110 | 65 | Scroll SE |
| 7 | 0111 | 61 | N+S+E = default (conflicting) |
| 8 | 1000 | 68 | Scroll W |
| 9 | 1001 | 69 | Scroll NW |
| 10 | 1010 | 61 | E+W blocked = default |
| 11 | 1011 | 61 | N+E+W = default |
| 12 | 1100 | 67 | Scroll SW |
| 13 | 1101 | 61 | N+S+W = default |
| 14 | 1110 | 61 | S+E+W = default |
| 15 | 1111 | 61 | All blocked = default |

### Bit assignment

- Bit 0 (0x01): North component active
- Bit 1 (0x02): East component active
- Bit 2 (0x04): South component active
- Bit 3 (0x08): West component active

### Cursor ID mapping

| ID | Cursor |
|----|--------|
| 61 | Default (no directional scroll arrow) |
| 62 | Arrow N |
| 63 | Arrow NE |
| 64 | Arrow E |
| 65 | Arrow SE |
| 66 | Arrow S |
| 67 | Arrow SW |
| 68 | Arrow W |
| 69 | Arrow NW |

Conflicting directions (e.g., N+S simultaneously) fall back to cursor 61 (default),
which makes sense — you can't display a meaningful directional arrow when opposite
directions cancel out.

---

## 17. ABuffer/ZBuffer Circular Scroll in TacticalClass_Draw

**Address:** 0x006D3D10 (TacticalClass_Draw), Pass 0
**Confidence:** HIGH

### Overview

When the viewport scrolls, the game avoids redrawing the entire screen. Instead, it
uses **circular buffers** for the ABuffer (alpha/compositing buffer) and ZBuffer
(depth buffer) to shift existing pixel data and only render the newly exposed strips.

### The circular buffer object (CircBufClass)

Both `CircBuf__Scroll` (0x00410ED0) and `FUN_007bcb50` (ZBuffer equivalent, at
0x007BCB50) share the same class structure:

| Offset | Type | Field |
|--------|------|-------|
| 0x10 | int | CurrentOffset — byte offset into the buffer (circular position) |
| 0x14 | ptr* | SurfacePtr — pointer to the underlying surface object |
| 0x18 | ptr | BufferStart — start of allocated buffer memory |
| 0x1C | ptr | BufferEnd — end of allocated buffer memory |
| 0x20 | int | BufferSize — total buffer size in bytes |
| 0x24 | int | ValidRows — number of valid rows (0x8000 = sentinel for "all valid") |
| 0x28 | int | Width — buffer width in pixels |
| 0x2C | int | Height — buffer height in pixels (rows) |

The buffer stores 16-bit values (2 bytes per pixel), hence the `* 2` operations
throughout the code.

### Circular scroll mechanism

The scroll function `CircBuf__Scroll(this, deltaX, deltaY, clearValue)`:

1. **Overflow check:** If `abs(deltaX) > Width` or `abs(deltaY) > Height`, the scroll
   exceeds the buffer dimensions — clear the entire buffer and reset.

2. **Horizontal scroll (deltaX != 0):**
   - Adjusts `CurrentOffset` by `deltaX * 2` bytes (2 bytes per pixel)
   - Wraps `CurrentOffset` circularly within `[BufferStart, BufferEnd)`
   - Fills the newly exposed column strip(s) with `clearValue`
   - Handles wrap-around: if the new column crosses the buffer end, fills two
     separate strips (one at the end, one at the start of the buffer)

3. **Vertical scroll (deltaY != 0):**
   - Adjusts `CurrentOffset` by `deltaY * Width * 2` bytes (full row stride)
   - Decrements `ValidRows` by `abs(deltaY)`
   - Wraps circularly
   - Fills the newly exposed row strip(s) with `clearValue`
   - Again handles wrap-around across the buffer boundary

4. **Fill operation:** Uses an optimized fill loop that writes `clearValue * 0x10001`
   (duplicating the 16-bit value into both halves of a 32-bit write) for 2x throughput,
   with alignment checks for odd-start and odd-end cases.

### How it fits into the render pipeline (Pass 0)

In `TacticalClass_Draw` with `param_3 == 0`:

```c
// Compute scroll delta from previous frame
int deltaX = abs(PrevViewportRect.left - ViewportRect.left);
int deltaY = abs(PrevViewportRect.top - ViewportRect.top);

if (deltaX == 0 && deltaY == 0) {
    if (forceRedraw) {
        ZBuffer_rect_clear();    // Clear entire ZBuffer
        CircBuf__FillAll();      // Clear entire ABuffer
    }
} else {
    // Blit old frame content on the composition surface
    (*DAT_0088731c->vtable[2])();  // Surface::Lock or Blt

    // Scroll the ZBuffer circularly
    FUN_007bcb50(deltaX, deltaY, clearValue);  // ZBuffer scroll

    // Scroll the ABuffer circularly
    CircBuf__Scroll(deltaX, deltaY, clearValue);  // ABuffer scroll

    // Swap composition surfaces (double-buffered)
    temp = DAT_0088731c;           // back surface
    DAT_0088731c = DAT_008872fc;   // composition surface
    DAT_008872fc = temp;
    g_PrimarySurface = temp;
}

// Pass 0 returns early — no drawing happens
return;
```

### Surface globals involved

| Address | Purpose |
|---------|---------|
| 0x0088731c | Back/composition surface pointer (swapped each scroll) |
| 0x008872fc | Primary composition surface pointer |
| g_PrimarySurface | Active drawing surface |

The double-buffer swap is critical: the game draws new content onto one surface while
the other holds the previous frame's data. The circular buffer scroll moves the "window"
into the data without copying pixels — only the newly exposed strips need rendering
in Pass 1.

### `CircBuf__FillAll` (0x004112D0)

Trivially fills the entire buffer by calling the surface's fill-rect method with
`{0, 0, Width, Height}` and the clear value. Used on full redraws.

---

## 18. Radar Minimap Click-to-Scroll

**Confidence:** HIGH

### Detection: FUN_0063AB60 — Is Mouse Over Radar?

**Address:** 0x0063AB60

Called from the main scroll input handler (FUN_00692F30) every frame with the mouse
position. Returns 1 if the mouse is over the radar minimap, 0 otherwise.

```c
int IsMouseOverRadar(int mouseX, int mouseY) {
    // Early exits
    if (!radarVisible)  return 0;    // DAT_00ac4cf4
    if (radarJammed)    return 0;    // DAT_00ac4cb0

    int hitWidget = 0;
    if (DAT_00ac4ccc != 0) {
        // Get radar widget bounds
        FUN_006343c0(bounds);
        int *center = FUN_006339e0();  // radar center point

        // Build hit-test rect: center +/- 4 pixels, expanded by 8 on each side
        CRect hitRect(center[0] - 4, center[1] - 4, 8, 8);

        // Test if mouse is within the expanded rect
        if (mouseX >= hitRect.left - 4 &&
            mouseX < hitRect.left - 4 + hitRect.width + 8 &&
            mouseY >= hitRect.top - 4 &&
            mouseY < hitRect.top - 4 + hitRect.height + 8)
        {
            hitWidget = DAT_00ac4ccc;
        }
    }

    int previousWidget = DAT_00ac4c38;
    if (previousWidget == 0) {
        if (hitWidget == 0) return 0;
    } else if (hitWidget != previousWidget) {
        // Mouse moved to a different widget or off radar
        FUN_005bda80(0, 0);  // notify: radar deactivated
        return 1;
    }

    // Mouse is on radar — notify with activation
    FUN_005bda80(0, 1);  // notify: radar activated
    return 1;
}
```

When this returns 1, the scroll input handler (FUN_00692F30) skips all scroll
processing for that frame — the radar handles its own input separately.

### Viewport setting from radar

The radar click does NOT go through the scroll system at all. Instead:

1. **Radar click handling** (in RadarClass input processing) converts the clicked
   radar pixel to a cell coordinate using `RadarClass__CellToRadarPixel` (inverse).

2. The cell coordinate is converted to pixel viewport position via
   `FUN_006d6070` (TacticalClass::SetViewportFromCell), which:
   ```c
   void SetViewportFromCell(int this, int *cellXY) {
       int cellX = cellXY[0], cellY = cellXY[1];
       // Isometric cell-to-pixel conversion
       int pixelX = (cellX * 60) / 2 + (cellY * -60) / 2;
       int pixelY = (cellX * 30) / 2 + (cellY * 30) / 2;
       pixelX = (pixelX + rounding) >> 8;
       pixelY = ((pixelY + rounding) >> 8) - AdjustForZ();

       // Clamp to map bounds
       clamp(&pixelX, &pixelY);

       // Set viewport immediately (no animation)
       this->ViewportX = this->DesiredViewportX = pixelX;
       this->ViewportY = this->DesiredViewportY = pixelY;
       FUN_006d8b30();  // recompute viewport rect
       this->ViewportMoved = 1;
   }
   ```

3. **FUN_006d6000** (TacticalClass::SetViewportDirect) is the pixel-coordinate
   variant, called from Main_Tick during multiplayer sync (receiving viewport
   position from the network):
   ```c
   void SetViewportDirect(int this, int *pixelXY) {
       int x = pixelXY[0], y = pixelXY[1];
       clamp(&x, &y);
       this->ViewportX = this->DesiredViewportX = x;
       this->ViewportY = this->DesiredViewportY = y;
       FUN_006d8b30();
       this->ViewportMoved = 1;
   }
   ```

4. **FUN_006d5f60** (TacticalClass::SetViewportAndRadarRect) additionally updates
   the radar viewport globals (`g_RadarViewportOffsetX/Y`, `g_RadarViewportWidth/
   Height`) before setting the viewport. Called during initialization and window
   resize.

### Key distinction

Radar clicks and multiplayer sync set the viewport **immediately** (both ViewportX
and DesiredViewportX in one step). Normal scrolling only writes to DesiredViewportX/Y,
and the Update function applies it on the next frame. The smooth camera animation
system interpolates over many frames.

---

## 19. FUN_00692F30 — Full Decompilation (Main Per-Frame Scroll Input Handler)

**Address:** 0x00692F30
**Confidence:** HIGH

### Complete reconstructed logic

```c
void __fastcall ScrollClass__ProcessScrollInput(int scrollObj) {
    int viewportOffsetX = g_RadarViewportOffsetX;
    int viewportOffsetY = g_RadarViewportOffsetY;

    // Global scroll suppression (loading screen, etc.)
    if (DAT_00a8ed9c != 0) return;

    // Get mouse position (via vtable[13] = GetMousePos)
    int mousePos[2];
    (*g_DisplayChain->vtable[0x34])(mousePos);  // populates {x, y}

    // Convert to viewport-relative coordinates
    int relX = mousePos[0] - viewportOffsetX;
    int relY = mousePos[1] - viewportOffsetY;

    // Check if mouse is over the radar minimap
    if (FUN_0063ab60(relX, relY)) {
        return;  // Radar handles its own input — skip all scroll processing
    }

    // BRANCH: Scroll is inhibited (RMB drag is active)
    if (*(char*)(scrollObj + 0x555A) == 1) {  // ScrollInhibited flag
        // Check for left mouse button (band-box selection during drag?)
        if (FUN_0054f5c0(1)) {
            DisplayClass__BandBox_MouseMove(&relativePos);
            return;
        }
        // Check for right mouse button (RMB drag scroll)
        if (FUN_0054f5c0(2)) {
            FUN_00693440(&relativePos);  // RMB drag scroll handler
            return;
        }
        // Neither button held — do nothing while inhibited
        return;
    }

    // BRANCH: Normal mode (not inhibited)

    // 1. Query what's under the cursor for action/tooltip display
    short cellXY[2];
    int cellCoords[3];
    int objectUnderCursor;
    char fogStatus, shroudStatus;

    bool valid = FUN_00692300(
        &relativePos, cellXY, cellCoords,
        &objectUnderCursor, &fogStatus, &shroudStatus
    );

    if (valid) {
        // Determine what action the cursor represents (attack, move, enter, etc.)
        int action = DisplayClass__DetermineAction(cellXY, objectUnderCursor, 1);
        // Set the cursor shape to match the action
        DisplayClass__SetCursorFromAction(
            cellXY, shroudStatus, objectUnderCursor, action, 0
        );
    }

    // 2. Edge-of-screen auto-scroll
    if (*(char*)(scrollObj + 0x5559) != 0 &&   // AutoScrollEnabled
        DAT_00a8ed6b == 0)                       // Not in observer mode
    {
        FUN_00692b60(&relativePos);  // Edge scroll handler
    }
}
```

### Key observations

1. **Radar takes priority:** If the mouse is over the radar, ALL scroll processing
   is skipped for that frame. The radar minimap handles its own input pipeline.

2. **Two modes:** The function switches between "inhibited" mode (RMB drag active,
   checking for continued drag or band-box) and "normal" mode (cursor query + edge
   scroll).

3. **Cursor query happens every frame** in normal mode, even if no scrolling occurs.
   This is what keeps the cursor shape updated as you move the mouse.

4. **Edge scroll only in normal mode:** Auto-scroll from screen edges only runs
   when not in RMB drag mode and when AutoScroll is enabled.

5. **Observer mode suppresses edge scroll** but not cursor detection — observers
   can still see action cursors but don't get automatic screen edge scrolling.

---

## 20. Additional Key Addresses

| Function/Data | Address | Purpose |
|---------------|---------|---------|
| FUN_00692300 | 0x00692300 | Cursor-under-mouse query (cell, object, fog) |
| FUN_006DA230 | 0x006DA230 | CanScrollInDirection (with wall-slide remap) |
| FUN_006D2420 | 0x006D2420 | Start smooth camera animation |
| FUN_00693840 | 0x00693840 | Keyboard handler (escape/cancel during scroll) |
| FUN_004AAD30 | 0x004AAD30 | General key command dispatcher (cancel modes) |
| FUN_0063AB60 | 0x0063AB60 | Is mouse over radar minimap? |
| FUN_006D6000 | 0x006D6000 | Set viewport directly (pixel coords, for net sync) |
| FUN_006D6070 | 0x006D6070 | Set viewport from cell coords (for radar click) |
| FUN_006D5F60 | 0x006D5F60 | Set viewport + update radar rect globals |
| FUN_0075F5C0 | 0x0075F5C0 | Linear interpolation helper (lerp 2D int points) |
| CircBuf__Scroll | 0x00410ED0 | ABuffer circular scroll |
| FUN_007BCB50 | 0x007BCB50 | ZBuffer circular scroll (identical algorithm) |
| CircBuf__FillAll | 0x004112D0 | Clear entire circular buffer |
| ZBuffer_rect_clear | 0x007BCF50 | Clear entire ZBuffer |
| LogicClass__AI | 0x0055DEE0 | Keyboard direction flag accumulation |
| DAT_00ABCE14 | 0x00ABCE14 | Keyboard scroll direction bitmask |
| Cursor table | 0x0083E790 | 16-entry direction→cursor ID mapping |
| Camera speed table | 0x008428EC | 5 float entries for smooth pan speeds |
| Direction remap table | 0x0084291C | 9-entry wall-slide direction remap |
| DAT_0088731C | 0x0088731C | Back composition surface (double-buffered) |
| DAT_008872FC | 0x008872FC | Primary composition surface |
