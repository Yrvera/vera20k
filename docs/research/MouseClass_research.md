# MouseClass Research Report

## Summary

MouseClass is part of the "game screen" inheritance chain in gamemd.exe. It sits
at the top of the chain and is responsible for cursor state management -- selecting
which cursor shape to display based on the current game action (move, attack,
sell, deploy, etc.), animating multi-frame cursors, and delegating low-level
mouse I/O to WWMouseClass.

**Confidence level:** High (~90%) for struct layout, hierarchy, and cursor data
table. The action enum and cursor-to-action mapping are verified from binary data
and decompiled code.

---

## 1. Class Hierarchy

The game screen classes form a single-inheritance chain. MouseClass is the **most
derived** class in this chain. The full hierarchy (base to derived):

```
GScreenClass          (RTTI @ 0x00816ba8)
  DisplayClass        (RTTI @ 0x00816be0)
    RadarClass        (RTTI @ 0x00816c00)
      PowerClass      (RTTI @ 0x00816c20)
        SidebarClass  (RTTI @ 0x00816c40)
          TabClass    (RTTI @ 0x00816c60)
            ScrollClass (RTTI @ 0x00816c78)
              MouseClass (RTTI @ 0x00816c98)
```

There is also a secondary vtable for `INoticeSink` (observer/notification
interface) at a separate offset within the object.

**Constructor call chain (verified):**
- `MouseClass::ctor` (0x005bda40) calls `ScrollClass::ctor` (0x00692290)
- `ScrollClass::ctor` calls `INoticeSink::ctor` (0x005be9b0)  
- `INoticeSink::ctor` calls `SidebarClass::ctor` (0x006a4f20)
- `SidebarClass::ctor` calls `DisplayClass::ctor` (0x006ac840)
- `DisplayClass::ctor` calls `GScreenClass::ctor` (0x005652c0)

The SidebarClass constructor also sets up RadarClass and PowerClass vtables
internally, confirming the chain.

### WWMouseClass (separate class)

WWMouseClass is **not** part of the game screen hierarchy. It is the low-level
Win32 mouse handler responsible for:
- Hiding/showing the Windows cursor (`ShowCursor`)
- Tracking mouse position via mutex-protected state
- Blitting the SHP cursor sprite onto the screen surface
- Thread safety via `DAT_00b78168` (MouseMutex)

**WWMouseClass hierarchy:**
```
Mouse (base)           (RTTI @ 0x0084e660, vtable @ 0x007f7b78)
  WWMouseClass         (RTTI @ 0x0084e640, vtable @ 0x007f7b2c)
```

Global pointer to the active WWMouseClass instance: `g_DisplayChain`
(used throughout as `(*g_DisplayChain->vtable[N])(...)`)

Source file: `D:\ra2mdpost\wwmous.cpp` (confirmed from debug string)

---

## 2. MouseClass Struct Layout

MouseClass is a massive object. `param_1` in its methods is `undefined4 *`
(int pointer), so field indices are multiplied by 4 to get byte offsets.

### MouseClass-specific fields

These are the fields MouseClass adds beyond what ScrollClass provides:

| Index   | Byte Offset | Type    | Description |
|---------|-------------|---------|-------------|
| 0x1546  | 0x5518      | ptr     | Secondary vtable (INoticeSink / TabClass) |
| 0x1552  | 0x5548      | int     | ScrollClass: scroll direction/state |
| 0x1553  | 0x554C      | byte    | ScrollClass: is scrolling flag |
| 0x1554  | 0x5550      | int     | ScrollClass: field |
| 0x1555  | 0x5554      | int     | ScrollClass: field |
| 0x1556  | 0x5558      | byte    | ScrollClass: field |
| -       | 0x5559      | byte    | ScrollClass: initialized to 1 |
| -       | 0x555A      | byte    | ScrollClass: field |
| 0x1557  | 0x555C      | byte    | **MouseClass: isMiniCursor** (0=normal, 1=mini) |
| 0x1558  | 0x5560      | int     | **MouseClass: currentCursorID** (index into cursor data table) |
| 0x1559  | 0x5564      | int     | **MouseClass: requestedCursorID** (set via vtable[0x48]) |
| 0x155A  | 0x5568      | int     | **MouseClass: currentAnimFrame** (animation frame counter) |

### Key global variables

| Address      | Type | Description |
|--------------|------|-------------|
| 0x00ABF294   | ptr  | Pointer to loaded MOUSE.SHA SHP data |
| 0x00ABF2A0   | int  | Cursor animation timer (from GetRadarTimer) |
| 0x00ABF2A8   | int  | Current cursor animation interval (frame rate) |
| 0x00ABF2DD   | byte | Flag: cursor has been set at least once |
| g_DisplayChain | ptr | Pointer to WWMouseClass instance |

### DisplayClass fields used by cursor logic

| Byte Offset | Type  | Description |
|-------------|-------|-------------|
| 0x11B3      | byte  | Waypoint placement mode active |
| 0x11CC      | int   | Cursor RGB color cache (R component) |
| 0x11CD      | byte  | Cursor palette G component |
| 0x11CE      | byte  | Cursor palette B component |

---

## 3. Key Methods

### MouseClass::Constructor (0x005BDA40)

Sets up the vtable, initializes cursor fields:
```
ScrollClass::Constructor();
this->vtable = &vtable_MouseClass;      // 0x007E1964
this->isMiniCursor = 0;                 // +0x555C
this->currentCursorID = 0;              // +0x5560
this->requestedCursorID = 0;            // +0x5564
this->currentAnimFrame = 0;             // +0x5568
this->secondaryVtable = &vtable_MouseClass_INoticeSink; // +0x5518
```

### MouseClass::One_Time (0x005BDF30) -- vtable[5]

Loads MOUSE.SHA from mix files:
```
SidebarClass::One_Time();
DAT_00abf294 = CDFileClass::Open("MOUSE.SHA");
```

### MouseClass::Init_Clear (0x005BDF50) -- vtable[7]

Resets cursor state:
```
SidebarClass::Init_Clear();
this->isMiniCursor = 0;   // +0x555C
this->requestedCursorID = 0; // +0x5564
```

### MouseClass::SetCursor (0x005BDA80) -- vtable[0x48], index 18

High-level cursor set. Stores the requested cursor ID and delegates to the
actual shape-setting function:
```c
void MouseClass::SetCursor(int cursorID, int miniFlag) {
    this->requestedCursorID = cursorID; // +0x5564
    this->SetMouseShape(cursorID, miniFlag); // vtable[0x4C]
}
```

### MouseClass::SetMouseShape (0x005BDC80) -- vtable[0x4C], index 19

The core cursor shape setter. Reads from the cursor data table and calls
WWMouseClass to update the displayed cursor:

```c
int MouseClass::SetMouseShape(int cursorID, bool useMini) {
    int idx = cursorID * 0x1C; // entry size
    
    // If no mini variant exists, force normal
    if (CursorData[cursorID].MiniStartFrame == -1)
        useMini = false;
    
    // Early-out if cursor hasn't changed
    if (alreadySet && (shpLoaded == 0 ||
        (cursorID == this->currentCursorID && useMini == this->isMiniCursor)))
        return 0;
    
    // Read frame rate for animation
    animInterval = CursorData[cursorID].FrameRate;
    animTimer = GetRadarTimer();
    this->currentAnimFrame = 0;
    
    // Pick start frame (mini or normal)
    int startFrame;
    if (useMini && CursorData[cursorID].MiniStartFrame != -1)
        startFrame = CursorData[cursorID].MiniStartFrame;
    else
        startFrame = CursorData[cursorID].StartFrame;
    
    // Compute hotspot from SHP dimensions
    int hotX = 0, hotY = 0;
    if (CursorData[cursorID].HotSpotX == 12345) // CENTER
        hotX = shpWidth / 2;
    if (CursorData[cursorID].HotSpotX == 54321) // RIGHT
        hotX = shpWidth;
    if (CursorData[cursorID].HotSpotY == 12345) // CENTER
        hotY = shpHeight / 2;
    if (CursorData[cursorID].HotSpotY == 54321) // BOTTOM
        hotY = shpHeight;
    
    // Update WWMouseClass with new cursor shape
    g_DisplayChain->SetCursor(&hotspot, mouseShp, startFrame);
    
    this->currentCursorID = cursorID;
    this->isMiniCursor = useMini;
    return 1;
}
```

### MouseClass::AnimationUpdate (0x005BDDC0) -- vtable[0x28], index 10

Called each tick to advance cursor animation and dispatch input:

```c
void MouseClass::AnimationUpdate(param2, param3) {
    int cursorID = this->currentCursorID;
    int idx = cursorID * 0x1C;
    
    // Static cursor? Skip animation
    if (CursorData[cursorID].FrameRate == 0)
        goto dispatch;
    
    // Timer not elapsed? Skip
    int now = GetRadarTimer();
    if (now - animTimer < animInterval)
        goto dispatch;
    
    // Advance frame
    this->currentAnimFrame++;
    int frameCount;
    if (this->isMiniCursor)
        frameCount = CursorData[cursorID].MiniFrameCount;
    else
        frameCount = CursorData[cursorID].FrameCount;
    
    this->currentAnimFrame %= frameCount; // loop
    
    // Reset timer, re-apply cursor shape
    animTimer = GetRadarTimer();
    int startFrame = ...; // same logic as SetMouseShape
    g_DisplayChain->SetCursor(&hotspot, mouseShp, startFrame + currentAnimFrame);
    
dispatch:
    DisplayClass::Dispatch(param2, param3);
}
```

### DisplayClass::SetCursorFromAction (0x004AAE90)

The massive 449-line function that maps **Action IDs** to **Cursor IDs**. This
is called from the input pipeline whenever the mouse moves over a new cell or
object. It:

1. Determines the cell/object under the cursor
2. Maps the action result to a cursor ID via a large switch statement
3. Calls `MouseClass::SetCursor(cursorID, miniFlag)` via vtable[0x48]
4. Also handles waypoint mode cursors and palette color remapping

### DisplayClass::DetermineAction (0x00692610)

194-line function that determines what action would be taken if the player
clicked at the current mouse position. Returns an Action enum value based on:
- Current selected unit(s)
- Target object/cell under cursor
- Active special modes (sell, repair, waypoint, superweapon, etc.)
- Unit capabilities (garrison, deploy, enter, capture, etc.)

---

## 4. Cursor Data Table

### Location and Structure

The cursor definition table is a static array at **0x0082D028** in `.data`.
Each entry is **0x1C (28) bytes**:

```c
struct MouseCursorData {   // size = 0x1C
    int StartFrame;        // +0x00: First frame index in MOUSE.SHA
    int FrameCount;        // +0x04: Number of animation frames
    int FrameRate;         // +0x08: Animation interval (0 = static, 4 = animated)
    int MiniStartFrame;    // +0x0C: Start frame for mini variant (-1 = none)
    int MiniFrameCount;    // +0x10: Frame count for mini variant (-1 = none)
    int HotSpotX;          // +0x14: Horizontal hotspot (0=Left, 12345=Center, 54321=Right)
    int HotSpotY;          // +0x18: Vertical hotspot (0=Top, 12345=Center, 54321=Bottom)
};
```

### HotSpot Sentinel Values

| Value  | Decimal | Meaning |
|--------|---------|---------|
| 0x0000 | 0       | Left/Top edge |
| 0x3039 | 12345   | Center (width/2 or height/2) |
| 0xD431 | 54321   | Right/Bottom edge (full width or height) |

### Cursor Entries (decoded from binary)

The table contains approximately 86 entries (indices 0 through ~85).
Key entries with their IDs and meanings:

| ID  | Name (inferred)     | Start | Count | Rate | Mini | MiniCnt | HotX   | HotY   |
|-----|---------------------|-------|-------|------|------|---------|--------|--------|
| 0   | Default/Arrow       | 0     | 1     | 0    | 1    | 1       | Left   | Top    |
| 1   | ScrollN             | 2     | 1     | 0    | -1   | -1      | Center | Top    |
| 2   | ScrollNE            | 3     | 1     | 0    | -1   | -1      | Right  | Top    |
| 3   | ScrollE             | 4     | 1     | 0    | -1   | -1      | Right  | Center |
| 4   | ScrollSE            | 5     | 1     | 0    | -1   | -1      | Right  | Right  |
| 5   | ScrollS             | 6     | 1     | 0    | -1   | -1      | Center | Right  |
| 6   | ScrollSW            | 7     | 1     | 0    | -1   | -1      | Left   | Right  |
| 7   | ScrollW             | 8     | 1     | 0    | -1   | -1      | Top    | Center |
| 8   | ScrollNW            | 9     | 1     | 0    | -1   | -1      | Top    | Top    | *(corrected 2026-05-29: HotY was "Center"; binary cursor 8 HotY=0x00000000=Top, not 0x3039=Center — read_memory 0x0082D028 — OPERATOR_OR_ORDER_DRIFT)* |
| 9   | NoScrollN           | 10    | 1     | 0    | -1   | -1      | Top    | Top    |
| 10  | NoScrollNE          | 11    | 1     | 0    | -1   | -1      | Center | Top    |
| 11  | NoScrollE           | 12    | 1     | 0    | -1   | -1      | Right  | Top    |
| 12  | NoScrollSE          | 13    | 1     | 0    | -1   | -1      | Right  | Center |
| 13  | NoScrollS           | 14    | 1     | 0    | -1   | -1      | Right  | Right  |
| 14  | NoScrollSW          | 15    | 1     | 0    | -1   | -1      | Center | Right  |
| 15  | NoScrollW           | 16    | 1     | 0    | -1   | -1      | Left   | Center | *(corrected 2026-05-29: HotY was "Right"; binary cursor 15 HotY=0x3039=Center, not 0xD431=Right — read_memory 0x0082D028 — OPERATOR_OR_ORDER_DRIFT)* |
| 16  | NoScrollNW          | 17    | 1     | 0    | -1   | -1      | Top    | Top    | *(corrected 2026-05-29: HotY was "Center"; binary cursor 16 HotY=0x00000000=Top, not 0x3039=Center — read_memory 0x0082D028 — OPERATOR_OR_ORDER_DRIFT)* |
| 17  | NoMove              | 18    | 13    | 4    | -1   | -1      | Center | Center |
| 18  | Select              | 31    | 10    | 4    | 42   | 10      | Center | Center |
| 19  | Move                | 41    | 1     | 0    | 52   | 1       | Center | Center |
| 20  | Attack              | 53    | 5     | 4    | 63   | 5       | Center | Center |
| 21  | AttackOutOfRange     | 58    | 5     | 4    | 63   | 5       | Center | Center |
| 22  | DesolatorDeploy      | 68    | 5     | 4    | 73   | 5       | Center | Center |
| 23  | Harvest             | 78    | 10    | 4    | -1   | 10      | Center | Center |
| 24  | Sell                | 88    | 1     | 0    | -1   | 1       | Center | Center |
| 25  | SellUnit            | 89    | 10    | 4    | 100  | 10      | Center | Center |
| 26  | Repair              | 99    | 1     | 0    | 63   | 1       | Center | Center |
| 27  | Deploy              | 110   | 9     | 4    | -1   | -1      | Center | Center |
| 28  | NoDeploy            | 119   | 1     | 0    | -1   | -1      | Center | Center |
| 29  | NoEnter             | 120   | 9     | 4    | -1   | -1      | Center | Center |
| 30  | EnterTunnel         | 129   | 10    | 4    | -1   | -1      | Center | Center |
| 31  | NoEnterTunnel       | 139   | 10    | 4    | -1   | -1      | Center | Center |
| 32  | IronCurtain         | 149   | 1     | 0    | -1   | -1      | Center | Center |
| 33  | LightningStorm      | 150   | 20    | 4    | -1   | -1      | Center | Center |
| 34  | ChronoSphere        | 170   | 20    | 4    | -1   | -1      | Center | Center |
| 35  | ToggleSelect        | 190   | 1     | 0    | -1   | -1      | Center | Center |
| 36  | Garrison (Enter)    | 191   | 7     | 0    | -1   | -1      | Center | Center |
| 37  | ScrollN anim 1      | 199   | 5     | 0    | -1   | -1      | Center | Center |
| 38  | ScrollNE anim 1     | 204   | 5     | 0    | -1   | -1      | Center | Center |
| 39  | ScrollE anim 1      | 209   | 5     | 0    | -1   | -1      | Center | Center |
| 40  | ScrollSE anim 1     | 214   | 5     | 0    | -1   | -1      | Center | Center |
| 41  | ScrollS anim 1      | 219   | 5     | 0    | -1   | -1      | Center | Center |
| 42  | ScrollW anim 1      | 224   | 5     | 0    | -1   | -1      | Center | Center |
| 43  | ScrollSW anim 1     | 229   | 5     | 0    | -1   | -1      | Center | Center |
| 44  | ScrollNW anim 1     | 234   | 5     | 0    | -1   | -1      | Center | Center |
| 45  | MoveAnim N          | 239   | 10    | 0    | -1   | -1      | Center | Center |
| 46  | MoveAnim NE         | 249   | 10    | 0    | -1   | -1      | Center | Center |
| 47  | MoveAnim E          | 259   | 10    | 0    | 516  | -1      | Center | Center |
| 48  | MoveAnim SE         | 269   | 10    | 0    | -1   | -1      | Center | Center |
| 49  | Tote                | 356   | 1     | 0    | -1   | -1      | Center | Center |
| 50  | ChronoWarp          | 279   | 20    | 4    | 514  | 1       | Center | Center |
| 51  | ParaDrop            | 299   | 10    | 0    | -1   | -1      | Center | Center |
| 52  | PlaceWaypoint       | 309   | 10    | 4    | -1   | -1      | Center | Center |
| 53  | TibSunBug           | 319   | 10    | 4    | 513  | 1       | Center | Center |
| 54  | EnterWaypointMode   | 329   | 10    | 0    | -1   | -1      | Center | Center |
| 55  | FollowWaypoint      | 339   | 6     | 0    | -1   | -1      | Center | Center |
| 56  | SelectWaypoint      | 345   | 1     | 0    | -1   | -1      | Center | Center |
| 57  | LoopWaypointPath    | 346   | 5     | 0    | -1   | -1      | Center | Center |
| 58  | DragWaypoint        | 357   | 12    | 0    | -1   | -1      | Center | Center |
| 59  | AttackWaypoint      | 369   | 15    | 0    | -1   | -1      | Center | Center |
| 60  | EnterWaypoint       | 384   | 1     | 0    | -1   | -1      | Center | Center |
| 61  | PatrolWaypoint      | 385   | 1     | 0    | -1   | -1      | Center | Center |
| 62-65 | Placeholder       | 386-389| 1    | 0    | -1   | -1      | Center | Center |

*Note: entries 37-48 appear to be directional scroll/move cursor variants
used by the waypoint system or move-cursor compass directions. Names are
inferred from frame layout patterns.*

---

## 5. Action Enum

The Action enum is used by `DetermineAction` and passed to `SetCursorFromAction`.
It is parsed from INI by `CCINIClass::ReadAction` (0x00474EE0) using a string
lookup table at **0x007E4C50** (73 entries, ending at 0x007E4D74).

Complete Action enum (verified from binary string table):

| ID | Name               | ID | Name               |
|----|--------------------|----|---------------------|
| 0  | None               | 37 | DragWaypoint        |
| 1  | Move               | 38 | LoopWaypointPath    |
| 2  | NoMove             | 39 | SelectWaypoint      |
| 3  | Enter              | 40 | FollowWaypoint      |
| 4  | Self               | 41 | EnterWaypointMode   |
| 5  | Attack             | 42 | TibSunBug           |
| 6  | Harvest            | 43 | PlaceWaypoint       |
| 7  | Sabotage           | 44 | ParaDrop            |
| 8  | Tote               | 45 | ChronoWarp          |
| 9  | Capture            | 46 | ChronoSphere        |
| 10 | Eaten              | 47 | LightningStorm      |
| 11 | Repair             | 48 | IronCurtain         |
| 12 | Sell               | 49 | NoEnterTunnel       |
| 13 | SellUnit           | 50 | EnterTunnel         |
| 14 | NoSell             | 51 | NoTogglePower       |
| 15 | NoRepair           | 52 | NoGRepair           |
| 16 | Damage             | 53 | NoEnter             |
| 17 | TogglePower        | 54 | NoDeploy            |
| 18 | Nuke               | 55 | GRepair             |
| 19 | DontUse4           | 56 | Heal                |
| 20 | DontUse5           | 57 | GuardArea           |
| 21 | DontUse6           | 58 | DontUse8            |
| 22 | DontUse7           | 59 | DontUse7 (dup?)     |
| 23 | Guard/Select       | 60 | DontUse6 (dup?)     |
| 24 | GuardArea          | 61 | Demolish            |
| 25 | ToggleSelect       | 62 | AttackMoveTar       |
| 26 | Deploy             | 63 | AttackMoveNav       |
| 27 | GRepair (orig)     | 64 | SelectBeacon        |
| 28 | Airstrike          | 65 | PlaceBeacon         |
| 29 | AreaAttack         | 66 | AttackSupport       |
| 30 | IvanBomb           | 67 | SelectNode          |
| 31 | NoIvanBomb         | 68 | DisarmBomb          |
| 32 | Detonate           | 69 | DetonateAll         |
| 33 | DetonateAll        | 70 | GeneticConverter    |
| 34 | PatrolWaypoint     | 71 | SpyPlane            |
| 35 | EnterWaypoint      | 72 | PsychicDominator    |
| 36 | AttackWaypoint     | -- | (end of table)      |

*Note: Several "DontUse" entries are TS legacy placeholders. The exact ordering
was verified from pointer table at 0x007E4C50 but some middle entries may have
shifted -- the first ~18 and last ~25 are high-confidence.*

---

## 6. Action-to-Cursor Mapping

`DisplayClass::SetCursorFromAction` (0x004AAE90) contains a massive switch
statement that maps Action IDs to Cursor IDs. Key mappings (verified from
decompilation):

### When NOT force-firing (param_3 == 0):

| Action           | Cursor ID | Cursor Name     |
|------------------|-----------|-----------------|
| 1 (Move)         | 0x12 (18) | Select          |
| 2 (NoMove)       | 0x13 (19) | Move            |
| 3,9,0xB (Enter)  | 0x19 (25) | SellUnit        |
| 4 (Self)         | 0x1B (27) | Deploy          |
| 5 (Attack, enter)| 0x14 (20) | Attack          |
| 5,6 (Attack)     | 0x15 (21) | AttackOutOfRange|
| 7,8,0x3D         | 0x11 (17) | NoMove          |
| 0xA (Eaten)      | 0x22 (34) | ChronoSphere    |
| 0xC (Sell)       | 0x1E (30) | EnterTunnel     |
| 0xD              | 0x1F (31) | NoEnterTunnel   |
| 0xE (NoSell)     | 0x20 (32) | IronCurtain     |
| 0xF,0x20 (Repair)| 0x23 (35) | ToggleSelect    |
| 0x10,0x37-0x40   | 0x34 (52) | PlaceWaypoint   |
| 0x14             | 0x35 (53) | TibSunBug       |
| 0x1D             | 0x21 (33) | LightningStorm  |
| 0x1E             | 0x1C (28) | NoDeploy        |
| 0x1F,0x24        | 0x1A (26) | Repair          |
| 0x25             | 0x39 (57) | LoopWaypointPath|
| 0x26             | 0x32 (50) | ChronoWarp      |
| 0x27,0x28        | 0x3A (58) | DragWaypoint    |
| 0x29,0x41        | 0x2F (47) | LightningStorm  |
| 0x35             | 0x26 (38) | (scroll variant)|
| 0x3C             | 0x4E (78) | (high cursor)   |
| 0x42             | 0x53 (83) | (high cursor)   |
| 0x43             | 0x55 (85) | (high cursor)   |
| 0x44             | 0x51 (81) | (high cursor)   |
| 0x45             | 0x4F (79) | (high cursor)   |
| 0x46             | 0x50 (80) | (high cursor)   |
| 0x47             | 0x52 (82) | (high cursor)   |
| 0x48             | 0x54 (84) | (high cursor)   |
| default          | 0x00 (0)  | Default/Arrow   |

### When force-firing (param_3 != 0):

The same general mapping applies but some actions redirect differently
(e.g., action 1/5 both go to cursor 0x12, action 2 with ally check may
go to 0x13 or 0x12).

### Shroud-covered cell override

When the target cell is under shroud (`FUN_005023b0` returns nonzero),
certain actions are remapped:
- Move (1,2) -> cursor 0x2D (45)
- Enter (3,9,0xB) -> cursor 0x32 (50)
- Attack (5,6) -> cursor 0x31 (49)

---

## 7. INI Integration

### Rules-level cursor settings

Read in `RulesClass::ReadGeneral`:

- **`AttackCursorOnDisguise`** (rules offset 0xFD5, bool) -- When true, shows
  attack cursor on disguised enemy units. Default: `no` in RA2, `yes` in YR.
- **`AttackCursorOnFriendlies`** (TechnoTypeClass) -- Per-unit flag allowing
  attack cursor on allies.

### Per-type cursor settings

Read in `TechnoTypeClass::ReadINI`:
- **`AttackCursorOnFriendlies`** (bool) -- Shows attack cursor on friendly targets.

Read in `WeaponTypeClass::ReadINI`:
- **`SabotageCursor`** (bool) -- Overrides fire cursor with sabotage cursor.
- **`MigAttackCursor`** (bool) -- YR-only, overrides cursor for MiG-style attacks.

Read in `SuperWeaponTypeClass::ReadINI`:
- **`Action`** (Action enum) -- Parsed via `CCINIClass::ReadAction`. Determines
  cursor shape when superweapon targeting is active. Stored at SWType+0xBC.

### Mouse asset files

- **`MOUSE.SHA`** -- The SHP file containing all cursor frames. Loaded during
  `MouseClass::One_Time`. Each cursor entry in the data table references frame
  indices into this file.
- **`MOUSEPAL.PAL`** -- Palette for the mouse cursor sprites (string at
  0x00826084, loaded nearby).

---

## 8. Integration with Input Pipeline

The mouse cursor update flows through this pipeline each frame:

```
1. GScreenClass::Input (vtable[9], 0x004F4320)
   - Reads mouse position from WWMouseClass
   - Reads keyboard/mouse button state
   - Calls vtable[0x28] (AI/update method)

2. MouseClass::AnimationUpdate (vtable[0x28], 0x005BDDC0)
   - Advances cursor animation frame if timer elapsed
   - Re-applies cursor shape via WWMouseClass
   - Calls DisplayClass::Dispatch

3. DisplayClass::Dispatch (0x006922E0)
   - Calls FUN_00692F30 (main input processing)
   - Calls CommandBar_Dispatch

4. During input processing:
   a. DisplayClass::DetermineAction (0x00692610)
      - Checks selected unit capabilities
      - Checks target object/cell properties
      - Returns an Action enum value
   
   b. DisplayClass::SetCursorFromAction (0x004AAE90)  
      - Maps Action enum to Cursor ID
      - Handles special modes (sell, repair, waypoint, superweapon)
      - Calls MouseClass::SetCursor (vtable[0x48])
   
   c. MouseClass::SetCursor (0x005BDA80)
      - Stores cursor ID
      - Calls MouseClass::SetMouseShape (vtable[0x4C])
   
   d. MouseClass::SetMouseShape (0x005BDC80)
      - Looks up cursor data from static table
      - Computes hotspot from SHP dimensions
      - Calls WWMouseClass::SetCursor to update display
```

### WWMouseClass Integration

WWMouseClass operates at a lower level than MouseClass:
- **vtable[1] (0x007B8A00)** -- SetCursor: receives hotspot, SHP pointer, and
  frame index; blits the cursor sprite to screen
- **vtable[4] (0x007B9750)** -- Show: increments visibility counter, restores
  cursor sprite on screen
- **vtable[5] (0x007B9930)** -- Hide: decrements visibility counter, erases
  cursor from screen
- All operations are mutex-protected (`DAT_00b78168`)
- Source: `D:\ra2mdpost\wwmous.cpp`

### Global Singleton

There is a single MouseClass instance that serves as the game screen object.
The inheritance chain means this same object IS the DisplayClass, MapClass,
SidebarClass, etc. -- it's one monolithic object containing all game screen state.

---

## 9. TS Legacy Notes

- **`TibSunBug`** (action 42, cursor 53): Named as a TS bug workaround. Present
  in the action enum and has a cursor entry, but its usage should be verified
  in actual YR gameplay.
- **`DontUse4` through `DontUse8`**: Placeholder action entries inherited from
  TS. These should never appear in normal YR gameplay.
- **Fog-of-war cursor changes**: The shroud-override cursor mappings are active
  regardless of fog settings, but they only trigger for shrouded cells, which
  is standard YR behavior (unexplored = black).

---

## 10. Key Addresses Summary

| Address    | Description |
|------------|-------------|
| 0x007E1964 | MouseClass vtable (primary) |
| 0x007E195C | MouseClass vtable (INoticeSink secondary, at object+0x5518) |
| 0x007F7B2C | WWMouseClass vtable |
| 0x007F7B78 | Mouse (base) vtable |
| 0x0082D028 | Cursor data table (86 entries, 0x1C bytes each) |
| 0x007E4C50 | Action enum string table (73 entries) |
| 0x00ABF294 | MOUSE.SHA SHP data pointer |
| 0x00ABF2A0 | Cursor animation timer |
| 0x00ABF2A8 | Current cursor animation interval |
| 0x00ABF2DD | Cursor-set-once flag |
| 0x005BDA40 | MouseClass::Constructor |
| 0x005BDA80 | MouseClass::SetCursor (vtable[0x48]) |
| 0x005BDC80 | MouseClass::SetMouseShape (vtable[0x4C]) |
| 0x005BDDC0 | MouseClass::AnimationUpdate (vtable[0x28]) |
| 0x005BDF30 | MouseClass::One_Time (vtable[5]) |
| 0x005BDF50 | MouseClass::Init_Clear (vtable[7]) |
| 0x004AAE90 | DisplayClass::SetCursorFromAction |
| 0x00692610 | DisplayClass::DetermineAction |
| 0x00474EE0 | CCINIClass::ReadAction |
| 0x006CEF80 | SuperWeaponTypeClass::GetAction |

---

## 11. DisplayClass::DetermineAction (0x00692610) -- Full Decompilation

**Confidence:** ~85%. Core logic verified from binary. Some vtable call semantics
inferred from context (e.g., `Is_Ally`, `Is_Cloaked`).

This 194-line function determines what action the player would take if they clicked
at the current mouse position. It returns an Action enum value. Parameters:

```
int DetermineAction(short* cellCoord, int* targetObject, int param_3)
```

- `cellCoord`: XY cell coordinate under the cursor
- `targetObject`: AbstractClass pointer to the object under cursor (or NULL)
- `param_3`: unknown context flag

### 11.1 Superweapon Targeting Override

If `DAT_00a8ecc8 != 0` (superweapon count / targeting active), the function
delegates immediately to `SelectBestObjectForAction()`:

- If `targetObject == NULL`: calls `selectedUnit->ActionOnCell(cellCoord, param_3, 0)` (vtable+0x70)
- If `targetObject != NULL`: calls `selectedUnit->ActionOnObject(targetObject, 0)` (vtable+0x74)

This short-circuits all other logic -- superweapon targeting replaces normal action
determination entirely.

### 11.2 Selected Object Validation

Calls `FUN_0040dd20()` to get a valid selected Techno. This function filters by
WhatAmI():
- Returns the object if WhatAmI == 1 (Infantry), 2 (Unit), 6 (Building), or 0xF (Aircraft)
- Returns NULL otherwise

The returned selected object (`piVar4`) is then checked:
- If `piVar4 != NULL` and `piVar4->field_0x41a == 0` (not limbo/destroyed):
  - If `piVar4->field_0x220 == 2` (mission == Guard?): checks sensor coverage at
    the cell. If sensor count for player house is 0, sets `bVar1 = false` (no valid
    action possible).
  - Otherwise: checks `piVar4->GetOwnerHouse()->field_0xC9A` (some house flag).
    If nonzero, action is still valid.

### 11.3 Tote/Crush Check

If `targetObject != NULL` and its WhatAmI() == 6 (Building) and
`targetObject->field_0x41a != 0` and `targetObject->field_0x6ED == 0x0F`:
- Additional sensor coverage check. If sensor is 0, goes to normal path.
- This appears to be checking for buildings that can be entered/toted.

### 11.4 Sabotage Check (bVar1 path)

When `bVar1 == true` (valid action context):
- If `targetObject != NULL` and `targetObject->vtable_0x138()` returns true
  (Is_Techno?) and WhatAmI != 6 or field_0x6E7 == 0:
  - If selected object is NULL or `piVar4->field_0x3D4 == 0`:
    - Sets `local_10 = 7` (Action::Sabotage)
- Then checks cell for bridge overlay via `FUN_00431090` (coordinate/height check):
  - If bridge found, sets `local_10 = 0x3D` (Action 61 = Demolish)

### 11.5 Sell Mode (DAT_00880998)

Global `DAT_00880998` at 0x00880998 is the **Sell Mode** flag. When active:
- If `targetObject != NULL` and `targetObject->GetOwner()` returns non-null:
  - Checks `HouseClass::IsHumanPlayer()` on the owner
  - If owner is human player:
    - Calls `targetObject->vtable_0x94()` (sellable check?): if true, action = 10 (Eaten/Sell target)
  - If not sellable or not human player: action = 0x0F (NoRepair / general forbidden)

### 11.6 Repair Mode (DAT_0088099a)

Global `DAT_0088099a` is the **Repair Mode** flag. When active:
- Default action = 0x22 (action 34 = PatrolWaypoint? -- likely misnamed, this is
  actually "Repair cursor" context)
- If `targetObject != NULL` and owner is human player:
  - Checks WhatAmI == 6 (Building):
    - Calls `targetObject->vtable_0x138()` (Is_Techno) and `vtable_0x80()` (Is_Powered_Down?):
      - If building and `TechnoTypeClass->field_0x154A != 0` (Repairable flag)
        and (`field_0xEE4 > 0` (damage taken) or `field_0x1573 != 0`):
        - action = 0x21 (action 33 = "DetonateAll" -- actually Repair Building)
      - Otherwise stays 0x22

### 11.7 Shroud Check

Calls `FUN_005023b0(cellCoord)` to check if cell is shrouded. The shroud state
(`iVar5`) is saved for later use by superweapon targeting.

### 11.8 Superweapon Targeting Mode (DAT_0088099b)

Global `DAT_0088099b` is the **Superweapon Targeting** flag. When active:
- Checks if Ctrl or Alt keys are held (force-fire / force-move checks via
  `FUN_0054f5c0` on key binding globals `DAT_00a8ec08` / `DAT_00a8ec0c`)
- If shrouded cell (`iVar5 != 0`): calls `FUN_00502460` to get cell owner info
- If `DAT_008809a4 == 0` (no specific superweapon index override):
  - If force-fire key held and cell is not shrouded:
    - Checks if cell belongs to player (`param_1 == g_PlayerPtr+0x20C`)
    - Calls `FUN_005090f0` (check if player has sufficient waypoint capacity?)
    - Calls `FUN_00763ba0(shroudResult)` (playfield/map bounds check?)
    - If all pass: action = 0x2F (action 47 = LightningStorm)
    - Otherwise: action = 0x2E (action 46 = ChronoSphere -- fallback invalid SW target)
  - If no force-fire: checks `FUN_005090f0` + `MapClass::Is_Cell_In_Playfield`:
    - Valid: action = 0x2A (action 42 = TibSunBug -- actually "valid SW target")
    - Invalid: action = 0x2B (action 43 = PlaceWaypoint -- "invalid SW target")
  - If force-fire NOT held: action = 0x30 (action 48 = IronCurtain)

### 11.9 Nuclear / Demolish Mode (DAT_0088099c)

If `DAT_0088099c` is set: action = 0x3C (action 60 = "DontUse6" -- likely the
Nuke targeting cursor)

### 11.10 Building Placement Mode (DAT_00880999)

Global `DAT_00880999` is the **Building Placement** flag. When active:
- If `targetObject != NULL` and owner is human player:
  - Calls `targetObject->vtable_0x98()` (can merge/upgrade?)
  - If WhatAmI == 6 (Building):
    - `vtable_0x80()` check (can power down?): modifies action
    - action = 0xC or 0xE (Sell or NoSell variants)
  - Otherwise falls through
- If no valid target: checks cell properties:
  - Converts cell to world coordinates
  - Calls `IsShrouded` and `FUN_005865e0` (always returns 0 in YR -- TS fog check!)
  - Checks cell overlay at +0x44 (bridge/overlay index) and overlay type flags
  - If cell has valid overlay and overlay type at +0x2A8 (buildable flag) and
    overlay at +0x50 (associated structure):
    - Checks if owner is human: action = 0xC (Sell)
  - Otherwise: action = 0xE (NoSell)

### 11.11 Superweapon Index Override (DAT_008809a0)

If `DAT_008809a0 != -1` (specific superweapon type is being targeted):
- Calls `SuperWeaponTypeClass->vtable_0x6C(cellCoord, targetObject)` to get the
  action for that specific superweapon
- If result != 0, overrides local_10

### 11.12 Final Fallback

- If action is still 0 and cell is shrouded: action = 0x2C (action 44 = ParaDrop)
- If `DAT_00880990 != 0` (some global disable flag, heavily used by building
  placement renderer): return 0 (no action)
- Otherwise: return the determined action

### 11.13 Global Mode Flags Summary

| Address      | Name (inferred)          | Description |
|--------------|--------------------------|-------------|
| 0x00880990   | g_BuildingPlacementActive | Building placement overlay active (shared with renderer) |
| 0x00880998   | g_SellMode               | Sell cursor mode active |
| 0x00880999   | g_PlacementMode          | Building placement mode active |
| 0x0088099a   | g_RepairMode             | Repair cursor mode active |
| 0x0088099b   | g_SuperweaponTargeting   | Superweapon targeting active |
| 0x0088099c   | g_NukeTargeting          | Nuclear strike targeting active |
| 0x008809a0   | g_ActiveSuperweaponIdx   | Index of active superweapon type (-1 = none) |
| 0x008809a4   | g_SuperweaponSubMode     | Superweapon sub-mode (e.g., Chrono second click) |
| 0x00a8ecc8   | g_SelectedCount          | Number of currently selected units |
| 0x00a8ecbc   | g_SelectedArray          | Pointer to array of selected TechnoClass pointers |

### 11.14 TS Legacy Notes

- `FUN_005865e0` at 0x005865E0 always returns 0. This is the fog-of-war visibility
  check -- confirmed dead in YR (fog disabled by default). It would check if a cell
  was "previously seen but not currently visible" in TS fog mode.
- Action 0x3C ("DontUse6" in the enum) is actually used for nuclear targeting in YR.
  The "DontUse" name is misleading -- it is live code.

---

## 12. DisplayClass::SetCursorFromAction (0x004AAE90) -- Full Decompilation

**Confidence:** ~90%. All switch cases verified from binary. Color remap logic
verified structurally, exact palette values are runtime-dependent.

### 12.1 Function Signature

```c
void __thiscall SetCursorFromAction(
    int* this,           // DisplayClass*
    short* cellCoord,    // cell XY under cursor
    char param_3,        // force-fire flag (0 = normal, nonzero = force-fire)
    int param_4,         // target object pointer (or 0)
    int action,          // Action enum from DetermineAction (param_5)
    int miniFlag         // mini cursor flag (param_6)
)
```

### 12.2 Initial Setup

1. Clears `this->field_0x11CC` (byte at DisplayClass+0x11CC, palette flag reset):
   `*(byte*)(this + 0x474) = 0` -- i.e., byte at offset 0x11D0 = 0.

2. If `param_4 == 0` (no target object): looks up the CellClass at the cell coordinate
   via `MapClass::Get_CellClass(cellCoord)` and stores in `local_8`.
   Otherwise: `local_8 = param_4` (the target object itself).

3. If `param_4 != 0` (target object exists): calls `Filter_AbstractType_InMap()` to get
   a filtered object. If the filtered object is not a Building (WhatAmI != 6) or
   `TechnoTypeClass+0x1701 == 0`, and `DAT_00a8ed6b == 0`:
   - Sets `filteredObject->field_0x431 = 1` (marks object as "cursor hovering")

### 12.3 Waypoint Placement Mode

If `this->field_0x11B3 != 0` (waypoint mode active) and `this->field_0x11BC != 0`
(waypoint object pointer exists):
- If cell is NOT shrouded and IS in playfield:
  - Updates the waypoint coordinate structure at `this->field_0x11BC`:
    - X = cellX * 256 + 128 (lepton center)
    - Y = cellY * 256 + 128
    - Z = ground height (+ bridge height if cell has bridge flag 0x100 at cell+0x140)

### 12.4 Shroud Override

If `FUN_005023b0(cellCoord)` returns nonzero (cell is shrouded), the action is
remapped before the main switch:

| Original Action | Remapped Action | Meaning |
|-----------------|-----------------|---------|
| 1 (Move)        | 0x2D (45)       | Move-into-shroud cursor |
| 2 (NoMove)      | 0x2D (45)       | Same |
| 3 (Enter)       | 0x32 (50)       | Enter-into-shroud |
| 9 (Capture)     | 0x32 (50)       | Same |
| 0xB (Repair)    | 0x32 (50)       | Same |
| 5 (Attack)      | 0x31 (49)       | Attack-into-shroud |
| 6 (Harvest)     | 0x31 (49)       | Same |
| All others      | unchanged       | |

### 12.5 Cursor Palette Color Remapping

Before the main switch, the function initializes cursor remap colors. There are
three color channels stored at DisplayClass offsets:

| Offset | Field | Description |
|--------|-------|-------------|
| 0x11CC | R     | Red component (byte at param_1+0x473, as int index) |
| 0x11CD | G     | Green component |
| 0x11CE | B     | Blue component |

**Initial color extraction (one-time):** If all three are zero, the function reads
the default color from the mouse cursor palette surface at `DAT_0087f6c8 + 0x174`
(pointer to the palette pixel data). It reads the 16-bit pixel at offset +2 (second
entry) and extracts R, G, B by shifting through the DirectDraw surface format:
- `R = (pixel >> g_DD_RShift) << g_DD_RLoss`
- `G = (pixel >> g_DD_GShift) << g_DD_GLoss`
- `B = (pixel >> g_DD_BShift) << g_DD_BLoss`

This baseline color is stored in the three fields.

### 12.6 Per-Action Palette Rewrite

The switch on actions 0x2A, 0x2B, 0x2F ("valid SW target", "invalid SW target",
"LightningStorm") performs a **house-color remap** of the first 8 palette entries:

```c
// Get house color index (player's color, 0-11)
int colorIdx = g_PlayerPtr->field_0x20C;  // ColorScheme index
if (colorIdx == -1) colorIdx = 0;
int baseOfs = (colorIdx % 12) * 8;  // 8 ramp entries per house color

// Remap first 8 palette entries to house color
for (int i = 0; i < 8; i++) {
    int idx = (i + baseOfs) & 0xFF;
    byte rgb[3] = DAT_00885180[idx * 3];  // 3-byte RGB entries
    uint16 pixel = (rgb.R >> RLoss) << RShift |
                   (rgb.G >> GLoss) << GShift |
                   (rgb.B >> BLoss) << BShift;
    paletteData[i + 1] = pixel;
}
```

The color table at `DAT_00885180` is a **96-entry (12 houses x 8 shades) RGB lookup
table**, 3 bytes per entry. It is initialized at runtime (zeroed in .data section).
Each house color has 8 gradient shades. This is used to tint superweapon cursor
circles to the player's house color.

For actions 0x2C, 0x2D, 0x2E, 0x31, 0x32, 0x33 (shroud-related actions):
- Same house-color remap, but the cell's owner is determined from the shrouded cell
  via `FUN_00502460` (extracting house index from shroud data).
- If NOT in waypoint mode: also writes the house index to `g_PlayerPtr+0x20C`.

For **all other actions** (the `default` case):
- Uses the stored DisplayClass RGB values (offsets 0x11CC-0x11CE) as a flat color
- Writes the same color to all 8 remap entries
- If NOT in waypoint mode: resets `g_PlayerPtr+0x20C = -1` (no house targeting)

### 12.7 Force-Fire Transform (FUN_0070f0b0)

Before the main cursor-setting switch, the action is passed through `FUN_0070f0b0`:

```c
int TransformForForceFire(int action) {
    bool forceFire = FUN_00731bf0();  // checks if Ctrl+Alt both held
    if (action == 1 && forceFire) return 0x3E;  // Move -> action 62 (AttackMoveTar)
    if (action == 5 && forceFire) return 0x3F;  // Attack -> action 63 (AttackMoveNav)
    return action;
}
```

`FUN_00731bf0` checks:
1. `DAT_00b0fe58` -- direct force-fire flag (if set, always returns true)
2. Both Ctrl keys held (via `FUN_0054f5c0` on `DAT_00a8ec00` / `DAT_00a8ec04`)
3. Both Alt keys held (via `FUN_0054f5c0` on `DAT_00a8ec08` / `DAT_00a8ec0c`)
4. If both Ctrl AND Alt are held: iterates all selected units and checks
   `vtable_0x4C0()` (CanForceAttackMove?). If ANY selected unit returns false,
   returns false.

### 12.8 Complete Action-to-Cursor Mapping (Non-Force-Fire, param_3 == 0)

| Action (hex) | Action Name          | Cursor ID | Cursor Name        |
|--------------|----------------------|-----------|--------------------|
| 0x01 (Move)  | Move                 | 0x12 (18) | Select             |
| 0x02 (NoMove)| NoMove               | 0x13 (19) | Move               |
| 0x36 (Heal)  | Heal                 | 0x13 (19) | Move               |
| 0x03 (Enter) | Enter                | 0x19 (25) | SellUnit           |
| 0x09 (Capture)| Capture             | 0x19 (25) | SellUnit           |
| 0x0B (Repair)| Repair               | 0x19 (25) | SellUnit           |
| 0x23 (Guard) | Guard                | 0x19 (25) | SellUnit           |
| 0x04 (Self)  | Self/Deploy          | 0x1B (27) | Deploy             |
| 0x34 (Patrol)| PatrolWaypoint       | 0x1B (27) | Deploy             |
| 0x05 (Attack)| Attack (in-range*)   | 0x14 (20) | Attack             |
| 0x05 (Attack)| Attack (fallthrough) | 0x15 (21) | AttackOutOfRange   |
| 0x06 (Harvest)| Harvest             | 0x15 (21) | AttackOutOfRange   |
| 0x07 (Sabotage)| Sabotage           | 0x11 (17) | NoMove             |
| 0x08 (Tote)  | Tote                 | 0x11 (17) | NoMove             |
| 0x3D (Demolish)| Demolish           | 0x11 (17) | NoMove             |
| 0x0A (Eaten) | Eaten                | 0x22 (34) | ChronoSphere       |
| 0x0C (Sell)  | Sell                 | 0x1E (30) | EnterTunnel        |
| 0x0D (SellUnit)| SellUnit           | 0x1F (31) | NoEnterTunnel      |
| 0x0E (NoSell)| NoSell               | 0x20 (32) | IronCurtain        |
| 0x0F (NoRepair)| NoRepair           | 0x23 (35) | ToggleSelect       |
| 0x20 (DontUse5)| --                 | 0x23 (35) | ToggleSelect       |
| 0x10 (Damage)| Damage               | 0x34 (52) | PlaceWaypoint      |
| 0x37-0x38,0x40| Waypoint actions    | 0x34 (52) | PlaceWaypoint      |
| 0x11 (TogglePower)| TogglePower    | 0x3C (60) | (high cursor)      |
| 0x1B,0x21,0x22| Various SW/special  | 0x3C (60) | (high cursor)      |
| 0x2A-0x2F    | SW targeting states  | 0x3C (60) | (high cursor)      |
| 0x31-0x33    | SW shroud states     | 0x3C (60) | (high cursor)      |
| 0x14 (Nuke)  | Nuke                 | 0x35 (53) | TibSunBug          |
| 0x1A (Deploy)| Deploy (directional*)| FUN_00731CC0() -> 0x16 (22) | DesolatorDeploy |
| 0x1C (unused)| --                   | (return, no cursor set) | |
| 0x1D (ChronoSphere)| ChronoSphere  | 0x21 (33) | LightningStorm     |
| 0x1E (NoDeploy)| NoDeploy           | 0x1C (28) | NoDeploy           |
| 0x1F (GRepair)| GRepair             | 0x1A (26) | Repair             |
| 0x24 (GuardArea)| GuardArea         | 0x1A (26) | Repair             |
| 0x25 (ToggleSelect)| ToggleSelect   | 0x39 (57) | LoopWaypointPath   |
| 0x26 (Deploy)| Deploy               | 0x32 (50) | ChronoWarp         |
| 0x27,0x28    | GRepair/Airstrike    | 0x3A (58) | DragWaypoint       |
| 0x29 (AreaAttack)| AreaAttack       | 0x2F (47) | (scroll variant)   |
| 0x41 (AttackSupport)| AttackSupport | 0x2F (47) | (scroll variant)   |
| 0x35 (EnterWaypoint)| EnterWaypoint | 0x26 (38) | (scroll variant)   |
| 0x39 (SelectWaypoint)| SelectWaypoint| 0x3B (59) | AttackWaypoint    |
| 0x3C (DontUse6)| NukeTarget         | 0x4E (78) | (high cursor)      |
| 0x3E,0x3F    | AttackMoveTar/Nav    | FUN_00731CB0() -> 0x47 (71) | |
| 0x42 (AttackMoveTar)| --            | 0x53 (83) | |
| 0x43 (AttackMoveNav)| --            | 0x55 (85) | |
| 0x44 (SelectBeacon)| SelectBeacon   | 0x51 (81) | |
| 0x45 (PlaceBeacon)| PlaceBeacon     | 0x4F (79) | |
| 0x46 (AttackSupport)| --            | 0x50 (80) | |
| 0x47 (SelectNode)| SelectNode       | 0x52 (82) | |
| 0x48 (DisarmBomb)| DisarmBomb       | 0x54 (84) | |
| default      | --                   | 0x00 (0)  | Default/Arrow      |

*Action 5 (Attack) special: if exactly 1 unit selected, that unit exists, has
flag 0x14 & 1 set, and `vtable_0x3AC(targetObject)` returns true (weapon can fire
at target), cursor = 0x14 (Attack in-range). Otherwise falls through to case 6
and gets cursor 0x15 (AttackOutOfRange).

### 12.9 Force-Fire Cursor Mapping (param_3 != 0)

When force-fire is active, most mappings change:

| Action | Force-Fire Cursor | Difference from normal |
|--------|-------------------|------------------------|
| 1 (Move) | 0x12 (Select/Attack) | Same as 5 in force-fire context |
| 5 (Attack) | 0x12 (Select/Attack) | Changes to "target" cursor |
| 2 (NoMove) | 0x13 OR 0x12 | 0x12 if single selected unit is InfantryType with field_0xC8D set and is Building-type; else 0x13 |
| 0x0A,0x0F,0x20 | 0x23 (ToggleSelect) | Same as non-force |
| 0x0C,0x0D,0x0E | 0x20 (IronCurtain) | Groups all sell variants together |
| 0x1E (NoDeploy) | 0x1C (NoDeploy) | Same |
| 0x32 | 0x3C then 0x4E | Chain: sets 0x3C, then falls through to set 0x4E |
| 0x39 | 0x3B | Same |
| 0x3D | 0x11 (NoMove) | Same |
| 0x3E,0x3F | FUN_00731CB0() -> 0x47 | Same |
| default | 0x00 (Arrow) | Same |

All other force-fire cases share labels with the non-force-fire path (same cursor).

### 12.10 Deploy Directional Cursor

Actions 0x1A (in both paths) calls `FUN_00731CC0()` which always returns 0x16 (22),
the **DesolatorDeploy** cursor. This is the deploy-cursor for units like the
Desolator. The function is trivial -- returns a constant.

For actions 0x3E / 0x3F (AttackMove variants), `FUN_00731CB0()` always returns 0x47
(71), which maps to an attack-move specific cursor ID.

---

## 13. WWMouseClass Internals

**Confidence:** ~85%. Struct layout verified from constructor + methods. Mutex
pattern verified. Some field semantics inferred from usage context.

### 13.1 WWMouseClass Struct Layout

The constructor at `WWMouseClass::Constructor` reveals the full layout. The object
uses `int*` pointer arithmetic (multiply indices by 4 for byte offsets):

| Index | Byte Offset | Type      | Init Value | Description |
|-------|-------------|-----------|------------|-------------|
| 0x00  | 0x00        | ptr       | vtable     | VTable pointer (Mouse base, then WWMouseClass) |
| 0x01  | 0x04        | int       | 0          | Current SHP data pointer |
| 0x02  | 0x08        | int       | 0          | Current frame index |
| 0x03  | 0x0C        | int       | -1         | Visibility counter (-1 = hidden initially) |
| 0x04  | 0x10        | byte      | 0          | Flag: is DirectDraw mode active |
| 0x05  | 0x14        | int       | 0          | Mouse screen X position |
| 0x06  | 0x18        | int       | 0          | Mouse screen Y position |
| 0x07  | 0x1C        | int       | 0          | (unused/reserved) |
| 0x08  | 0x20        | int       | 0          | (unused/reserved) |
| 0x09  | 0x24        | ptr       | param_2    | Back-buffer surface pointer |
| 0x0A  | 0x28        | int       | param_3    | HWND (window handle) |
| 0x0B  | 0x2C        | int       | screen.left | Confining rect left |
| 0x0C  | 0x30        | int       | screen.top  | Confining rect top |
| 0x0D  | 0x34        | int       | screen.w    | Confining rect width |
| 0x0E  | 0x38        | int       | screen.h    | Confining rect height |
| 0x0F  | 0x3C        | int       | 0          | Hotspot X |
| 0x10  | 0x40        | int       | 0          | Hotspot Y |
| 0x11  | 0x44        | ptr       | 0          | Primary surface pointer (for blit) |
| 0x12  | 0x48        | int       | rect.left  | Saved cursor rect: X |
| 0x13  | 0x4C        | int       | rect.top   | Saved cursor rect: Y |
| 0x14  | 0x50        | int       | rect.w     | Saved cursor rect: Width |
| 0x15  | 0x54        | int       | rect.h     | Saved cursor rect: Height |
| 0x16  | 0x58        | int       | 0          | (reserved) |
| 0x17  | 0x5C        | int       | rect.left  | Previous rect field 1 |
| 0x18  | 0x60        | int       | rect.top   | Previous rect field 2 |
| 0x19  | 0x64        | int       | rect.w     | Previous rect field 3 |
| 0x1A  | 0x68        | int       | rect.h     | Previous rect field 4 |
| 0x1B  | 0x6C        | int       | 0          | (reserved) |
| 0x1C  | 0x70        | int       | rect.left  | Another rect copy |
| 0x1D  | 0x74        | int       | rect.top   | |
| 0x1E  | 0x78        | int       | rect.w     | |
| 0x1F  | 0x7C        | int       | rect.h     | |
| 0x20  | 0x80        | int       | rect.left  | Yet another rect copy |
| 0x21  | 0x84        | int       | rect.top   | |
| 0x22  | 0x88        | int       | rect.w     | |
| 0x23  | 0x8C        | int       | rect.h     | |
| 0x24  | 0x90        | int       | -1         | Timer/counter field |
| 0x25  | 0x94        | int       | 0          | (reserved) |

**Total size:** 0x98 bytes (confirmed from `operator_new(0x98)` in the video mode
reset code at 0x00560BF0).

### 13.2 VTable Layout

The WWMouseClass vtable at 0x007F7B2C contains 16 entries:

| VTable Index | Offset | Address    | Description |
|--------------|--------|------------|-------------|
| 0            | +0x00  | 0x007BA3A0 | Destructor (scalar deleting) |
| 1            | +0x04  | 0x007B8A00 | **SetCursor** -- update cursor shape |
| 2            | +0x08  | 0x007BA320 | Process_Message (not found, may be stub) |
| 3            | +0x0C  | 0x007B9930 | **Hide** -- decrement visibility, erase cursor |
| 4            | +0x10  | 0x007B9750 | **Show** -- increment visibility, draw cursor |
| 5            | +0x14  | 0x007B9C30 | Conditional_Hide (not decompilable) |
| 6            | +0x18  | 0x007B9A60 | Conditional_Show (not decompilable) |
| 7            | +0x1C  | 0x007BA330 | Is_DirectDraw_Active? (corrected 2026-05-29: was 0x007BA350; binary read_memory at vtable+0x1C = 0x007BA330, which contains `mov al,[ecx+0x10]; ret` — RTTI_LABEL_DRIFT) |
| 8            | +0x20  | 0x007B9D70 | Get_Mouse_XY variant (corrected 2026-05-29: was 0x007B9D80; binary read_memory at vtable+0x20 = 0x007B9D70 — RTTI_LABEL_DRIFT) |
| 9            | +0x24  | 0x007B9D80 | Get_Mouse_XY (duplicate) |
| 10           | +0x28  | 0x007B89F0 | Low_Hide_Mouse (stub/helper) |
| 11           | +0x2C  | 0x007BA340 | Get_Mouse_X |
| 12           | +0x30  | 0x007BA350 | Get_Mouse_Y (or Is_DD check) |
| 13           | +0x34  | 0x007BA360 | (stub) |
| 14           | +0x38  | 0x007BA380 | (stub) |
| 15           | +0x3C  | 0x007B90C0 | Set_Cursor_Clip / confining rect |

### 13.3 Mutex Pattern

All WWMouseClass operations that touch screen surfaces or cursor state are protected
by a global mutex: `DAT_00b78168` (MouseMutex).

The pattern is consistent:
```c
DWORD result = WaitForSingleObject(DAT_00b78168, 10000);
if (result == WAIT_TIMEOUT) {  // 0x102
    Register_heap_pool("Warning: Probable deadlock occurrence",
                       "D:\\ra2mdpost\\wwmous.cpp", 0x4FD);
}
// ... critical section ...
ReleaseMutex(DAT_00b78168);
```

The timeout is 10 seconds. If exceeded, a warning is logged (line 0x4FD = 1277 in
the original source). The mutex is used for:
- SetCursor (blit old cursor background, draw new cursor)
- Show (draw cursor from saved state)
- Hide (erase cursor to saved background)
- GetCursorRect
- Constructor (initial confining rect setup)

### 13.4 SetCursor (vtable[1], 0x007B8A00)

Full algorithm:
1. If `param_3 == 0` (no SHP data): early return.
2. Calls `vtable[7]` (Is_DirectDraw_Active). If active, enters mutex-protected path:
   a. First mutex: erase current cursor by blitting saved background
      (`this->backbuffer->Blit(savedRect, this->primarySurface, ...)`)
   b. Store new cursor state: `this->shpData = param_3`, `this->frameIndex = param_4`,
      `this->hotspotX = param_2[0]`, `this->hotspotY = param_2[1]`
   c. Second mutex: compute new cursor rect via `FUN_007B8E80`, save background from
      screen, then blit new cursor SHP frame onto primary surface.
   d. Clear the back-buffer surface.
3. If NOT DirectDraw: just store the new cursor state without blitting.

### 13.5 Show (vtable[4], 0x007B9750)

1. Acquires mutex.
2. Increments `this->visibilityCounter` (field at +0x0C).
3. If counter reaches 0 (was -1, now 0 = visible):
   - If DirectDraw active: acquires second mutex, computes cursor rect from SHP frame,
     saves screen background, blits cursor SHP to primary surface, clears back-buffer.
   - If NOT DirectDraw: calls `ShowCursor(TRUE)` in a loop until cursor count >= 0.
4. Clamps counter: if > 0, sets to 0 (prevents double-show).
5. Releases mutex.

### 13.6 Hide (vtable[3], 0x007B9930)

1. Acquires mutex.
2. If `visibilityCounter == 0` (cursor is currently visible):
   - If DirectDraw: acquires second mutex, blits saved background back to erase cursor.
   - If NOT DirectDraw: calls `ShowCursor(FALSE)` in a loop until count < 0.
3. Decrements `visibilityCounter`.
4. Releases mutex.

### 13.7 GetCursorRect (FUN_007B8E80)

Computes the screen rectangle occupied by the current cursor:
1. Acquires mutex.
2. Gets the SHP frame dimensions via `SHP_frame_rect_getter`.
3. Adjusts position: `x = frameX + (mouseX - hotspotX)`, `y = frameY + (mouseY - hotspotY)`.
4. Returns `{x, y, width, height}`.

### 13.8 CalcConfiningRect (FUN_007B8960)

Called from constructor to set the cursor confinement rectangle:
1. `GetClientRect(hwnd)` to get window client area.
2. `ClientToScreen` on both corners to convert to screen coordinates.
3. Stores: left, top, width, height into fields 0x2C-0x38.

### 13.9 Global Instance

The global WWMouseClass instance pointer is stored at `DAT_00b78164` (set in
constructor). The `g_DisplayChain` pointer used throughout the codebase points to
this same instance.

---

## 14. Mini Cursor Logic

**Confidence:** ~90%. All callers of SetCursor verified via xref analysis.

### 14.1 What is Mini Cursor?

The `miniFlag` parameter to `MouseClass::SetCursor` selects between the normal and
mini variant of a cursor. The mini variant uses `MiniStartFrame`/`MiniFrameCount`
from the cursor data table (section 4) instead of `StartFrame`/`FrameCount`.

Mini cursors are smaller versions of the same cursor, designed for use when hovering
over the sidebar or small UI elements where a full-size cursor would be visually
overwhelming.

### 14.2 How miniFlag Gets Set

The `miniFlag` parameter flows through this chain:
```
SetCursorFromAction(cell, forceFire, target, action, miniFlag)
  -> MouseClass::SetCursor(cursorID, miniFlag)   // vtable[0x48]
    -> MouseClass::SetMouseShape(cursorID, miniFlag)  // vtable[0x4C]
```

The `miniFlag` value (param_6) in SetCursorFromAction is passed directly through to
SetCursor as the second argument. It originates from the callers of SetCursorFromAction.

### 14.3 Callers That Pass miniFlag = 1

From xref analysis of `FUN_005bda80` (MouseClass::SetCursor):

1. **Waypoint system hover** (0x0063A5A0, `PlanningManager::LeftButtonDown`):
   - `FUN_005bda80(0, 1)` -- when the cursor is over a waypoint node that matches
     the currently selected/tracked waypoint. Sets cursor to Default(0) with mini=1.

2. **Waypoint system click** (0x0063A8E0, `PlanningManager::LeftButtonUp`):
   - `FUN_005bda80(0, 1)` -- when clicking on an existing waypoint that matches
     the tracked waypoint.

3. **Waypoint system drag** (0x0063AB60, `PlanningManager::MouseMove`):
   - `FUN_005bda80(0, uVar4)` where `uVar4 = 0 or 1`:
     - `1` when the cursor is over a new waypoint (different from the previously
       tracked one) -- indicates "can interact with this waypoint"
     - `0` when leaving a waypoint area

### 14.4 Callers That Pass miniFlag = 0

Most callers pass 0:

- **Game initialization** (0x00685670, 0x00685DC0, 0x006863E0): `FUN_005bda80(0, 0)`
  -- reset cursor to default arrow, normal size, during game state transitions
  (movie playback end, scenario load, etc.)
- **GameExit::BattleControlTerminated** (0x006865EE): `FUN_005bda80(0, 0)` -- reset
  on battle end
- **Main_Game** (0x0052DAF4): `FUN_005bda80(0, 0)` -- initial game setup

### 14.5 SetCursorFromAction miniFlag Passthrough

In `SetCursorFromAction`, the `param_6` (miniFlag) is passed directly to every
`(*vtable[0x48])(cursorID, param_6)` call. So any caller of SetCursorFromAction
controls the mini cursor state. The main caller (the input dispatch pipeline) passes
the flag based on whether the cursor is over the sidebar vs the main game area.

### 14.6 Cursor Entries Without Mini Variants

When `CursorData[id].MiniStartFrame == -1`, the mini flag is forced to false in
`SetMouseShape`, so the cursor always uses its normal size. Most cursors lack mini
variants -- only Select (18), Move (19), Attack (20), AttackOutOfRange (21),
DesolatorDeploy (22), SellUnit (25), and ChronoWarp (50) have them.

---

## 15. Cursor Palette / Color Remapping Details

**Confidence:** ~80%. Mechanism verified from binary. Exact runtime palette values
depend on loaded assets and cannot be verified statically.

### 15.1 Palette Surface

The cursor palette is stored in a DirectDraw surface pointed to by:
- `DAT_0087f6c8` -- pointer to a surface object (runtime, zeroed in .data)
- `DAT_0087f6c8 + 0x174` -- pointer to the actual palette pixel data (16-bit entries)

This surface is loaded from **MOUSEPAL.PAL** during initialization and converted to
the active DirectDraw pixel format.

### 15.2 Color Channel Extraction

The game uses DirectDraw shift/loss values to convert between 8-bit RGB and 16-bit
packed pixel format:

| Global       | Purpose |
|--------------|---------|
| g_DD_RShift  | Right-shift to extract R from 16-bit pixel |
| g_DD_RLoss   | Left-shift to expand R back to 8-bit |
| g_DD_GShift  | Right-shift for G |
| g_DD_GLoss   | Left-shift for G |
| g_DD_BShift  | Right-shift for B |
| g_DD_BLoss   | Left-shift for B |

### 15.3 House Color Remap Table

At runtime, `DAT_00885180` contains a 96-entry color ramp table (12 houses x 8 shades,
3 bytes per entry = 288 bytes total). Each house has 8 shades from dark to light.

The 12 house colors correspond to the standard RA2/YR multiplayer colors (indices 0-11).
The remap selects 8 consecutive shades starting at `(houseColorIndex % 12) * 8`.

### 15.4 When Remapping Occurs

Color remapping happens in `SetCursorFromAction` for specific action groups:

1. **Superweapon targeting cursors** (actions 0x2A, 0x2B, 0x2F): Remaps to the
   local player's house color. The cursor circle/ring tint matches your faction color.

2. **Shroud-targeted cursors** (actions 0x2C, 0x2D, 0x2E, 0x31, 0x32, 0x33): Remaps
   to the cell owner's color (extracted from shroud data). This shows whose territory
   you're targeting.

3. **All other cursors** (default): Uses a flat color from the DisplayClass cache
   (offsets 0x11CC-0x11CE). Resets the house targeting index to -1.

### 15.5 Remap Target

The remapping overwrites palette entries 1-8 (indices 1 through 8, skipping entry 0)
in the cursor palette surface. Entry 0 is preserved as transparent/background.
This means the cursor SHP frames use palette indices 1-8 for their "team colored"
portions, and these get dynamically rewritten each frame based on the action context.

---

## 16. GScreenClass::Input (0x004F4320)

**Confidence:** ~90%. Simple function, fully decompiled.

### 16.1 Function Signature

```c
void __thiscall GScreenClass::Input(
    int* this,             // GScreenClass*
    uint* outKeyEvent,     // output: keyboard/mouse event code
    int*  outMouseX,       // output: mouse X position
    int*  outMouseY        // output: mouse Y position
)
```

### 16.2 Full Logic

1. **Read mouse position from WWMouseClass:**
   - `*outMouseX = g_DisplayChain->vtable[0x0B]()` (Get_Mouse_X, vtable+0x2C)
   - `*outMouseY = g_DisplayChain->vtable[0x0C]()` (Get_Mouse_Y, vtable+0x30)

2. **Read input events:**
   - If `DAT_00a8ef54 == NULL` (no network/replay input source):
     - Calls `FUN_0054f000()` to poll keyboard input (returns 16-bit event code)
     - Masks to `& 0xFFFF` and stores in `*outKeyEvent`
     - If nonzero: calls `FUN_0054f050()` for secondary poll (mouse button state?)
       and updates `*outKeyEvent` again
   - If `DAT_00a8ef54 != NULL` (replay/network input source):
     - Calls `DAT_00a8ef54->vtable[0x17]()` (0x5C) -- checks for pending input
     - If pending: calls `this->vtable[0x0E]()` (vtable+0x38, some reset/flush)
     - Temporarily swaps `g_PrimarySurface` with `DAT_0088730c` (replay surface?)
     - Calls `DAT_00a8ef54->vtable[0x0A]()` (0x28) -- reads the input event
     - Restores `g_PrimarySurface`

3. **Dispatch to AI/Update:**
   - Copies mouse XY to local stack variables
   - Calls `this->vtable[0x0A](*outKeyEvent, &mouseXY)` (vtable+0x28)
   - In MouseClass, vtable[0x0A] is `MouseClass::AnimationUpdate` (0x005BDDC0)

### 16.3 Input Source Architecture

The dual-path design (direct poll vs DAT_00a8ef54) supports:
- **Normal gameplay:** Direct polling via `FUN_0054f000` / `FUN_0054f050` (Win32
  message pump / DirectInput)
- **Replay playback:** Input events read from a replay stream object, with the
  primary surface temporarily swapped (to render replay frames to a different buffer)

This is part of the lockstep architecture -- in multiplayer, input events are
serialized and replayed identically on all machines.

---

## 17. Key Addresses Addendum

| Address    | Description |
|------------|-------------|
| 0x00b78164 | WWMouseClass global instance pointer (set in constructor) |
| 0x00b78168 | WWMouseClass global mutex handle (MouseMutex) |
| 0x00b78128-0x00b78134 | Screen rect globals (left, top, width, height) |
| 0x00885180 | House color remap table (12x8 entries, 3 bytes each, runtime) |
| 0x0087f6c8 | Cursor palette surface pointer (runtime) |
| 0x00880990 | g_BuildingPlacementActive flag |
| 0x00880998 | g_SellMode flag |
| 0x00880999 | g_PlacementMode flag |
| 0x0088099a | g_RepairMode flag |
| 0x0088099b | g_SuperweaponTargeting flag |
| 0x0088099c | g_NukeTargeting flag |
| 0x008809a0 | g_ActiveSuperweaponIdx (-1 = none) |
| 0x008809a4 | g_SuperweaponSubMode |
| 0x00a8ecc8 | g_SelectedCount |
| 0x00a8ecbc | g_SelectedArray pointer |
| 0x00a8ef54 | Replay/network input source (NULL = direct input) |
| 0x00b0fe58 | Force-fire direct flag |
| 0x00a8ec00 | Ctrl key binding 1 |
| 0x00a8ec04 | Ctrl key binding 2 |
| 0x00a8ec08 | Alt key binding 1 |
| 0x00a8ec0c | Alt key binding 2 |
| 0x007B8960 | WWMouseClass::CalcConfiningRect |
| 0x007B8E80 | WWMouseClass::GetCursorRect |
| 0x0070f0b0 | TransformActionForForceFire |
| 0x00731bf0 | IsForceFireActive (checks Ctrl+Alt + unit capability) |
| 0x00731cb0 | GetAttackMoveCursorID (returns 0x47 always) |
| 0x00731cc0 | GetDeployCursorID (returns 0x16 always) |

---

## 18. Full MouseClass Primary VTable (0x007E1964)

**Confidence:** ~95%. All 71 entries read from binary. Function names are from
Ghidra labels or inferred from known addresses in prior sections.

The MouseClass primary vtable at 0x007E1964 has **71 entries** (slots 0-70).
This covers the entire GScreenClass -> DisplayClass -> RadarClass -> PowerClass
-> SidebarClass -> TabClass -> ScrollClass -> MouseClass inheritance chain.

param_1 in MouseClass methods is `undefined4 *` (int pointer), so vtable
indices correspond to byte offset = index * 4.

### 18.1 VTable Dump

| Slot | Offset | Address    | Owner Class    | Purpose (inferred)                    |
|------|--------|------------|----------------|---------------------------------------|
|  0   | +0x000 | 0x004F4240 | GScreenClass   | Destructor (scalar deleting)          |
|  1   | +0x004 | 0x0040D230 | GScreenClass   | Stub/size query                       |
|  2   | +0x008 | 0x0040D240 | GScreenClass   | Stub/size query 2                     |
|  3   | +0x00C | 0x005656D0 | SidebarClass   | Init (override)                       |
|  4   | +0x010 | 0x0040D290 | GScreenClass   | Stub                                  |
|  5   | +0x014 | 0x005BDF30 | **MouseClass** | **One_Time** -- loads MOUSE.SHA       |
|  6   | +0x018 | 0x004F42B0 | GScreenClass   | Init_IO                               |
|  7   | +0x01C | 0x005BDF50 | **MouseClass** | **Init_Clear** -- resets cursor state |
|  8   | +0x020 | 0x0040D270 | GScreenClass   | Stub                                  |
|  9   | +0x024 | 0x004F4320 | GScreenClass   | **Input** -- reads mouse pos + events |
| 10   | +0x028 | 0x005BDDC0 | **MouseClass** | **AnimationUpdate** -- cursor anim    |
| 11   | +0x02C | 0x004F43F0 | GScreenClass   | Draw (stub/passthrough)               |
| 12   | +0x030 | 0x004F4410 | GScreenClass   | Blit (stub/passthrough)               |
| 13   | +0x034 | 0x004F4450 | GScreenClass   | Mark_For_Redraw                       |
| 14   | +0x038 | 0x004F42F0 | GScreenClass   | Reset/Flush event                     |
| 15   | +0x03C | 0x004F4480 | GScreenClass   | Set_Dimensions                        |
| 16   | +0x040 | 0x006D0A20 | TabClass       | Activate (sidebar tab handler)        |
| 17   | +0x044 | 0x004F45B0 | GScreenClass   | Flag_To_Redraw                        |
| 18   | +0x048 | 0x005BDA80 | **MouseClass** | **SetCursor(cursorID, miniFlag)**     |
| 19   | +0x04C | 0x005BDC80 | **MouseClass** | **SetMouseShape(cursorID, miniFlag)** |
| 20   | +0x050 | 0x005BDAA0 | **MouseClass** | GetCursorID (returns currentCursorID) |
| 21   | +0x054 | 0x005BDAB0 | **MouseClass** | GetRequestedCursorID                  |
| 22   | +0x058 | 0x00565AA0 | SidebarClass   | Sidebar_Init                          |
| 23   | +0x05C | 0x00565B00 | SidebarClass   | Sidebar_Draw                          |
| 24   | +0x060 | 0x00565BC0 | SidebarClass   | Sidebar_Activate                      |
| 25   | +0x064 | 0x00577920 | PowerClass     | Power_Draw / PowerBar handler         |
| 26   | +0x068 | 0x00693060 | ScrollClass    | Scroll_Input_Handler                  |
| 27   | +0x06C | 0x0056BBE0 | SidebarClass   | Sidebar_StripClass handler            |
| 28   | +0x070 | 0x00653F50 | RadarClass     | Radar_Render                          |
| 29   | +0x074 | 0x00654490 | RadarClass     | Radar_ClickHandler                    |
| 30   | +0x078 | 0x005BDF70 | **MouseClass** | Override (calls ScrollClass base)     |
| 31   | +0x07C | 0x005BE6D0 | ScrollClass?   | Scroll/INoticeSink dispatch           |
| 32   | +0x080 | 0x004ACE70 | DisplayClass   | DisplayClass::Draw_Band_Box_Related   |
| 33   | +0x084 | 0x006D1800 | TabClass       | Tab draw / sidebar header             |
| 34   | +0x088 | 0x006ABD30 | SidebarClass   | SidebarClass::AI update               |
| 35   | +0x08C | 0x006938C0 | ScrollClass    | Scroll state update                   |
| 36   | +0x090 | 0x00653810 | RadarClass     | Radar update 1                        |
| 37   | +0x094 | 0x00653830 | RadarClass     | Radar update 2                        |
| 38   | +0x098 | 0x004A9DD0 | DisplayClass   | DisplayClass::Passable_Check          |
| 39   | +0x09C | 0x004AA050 | DisplayClass   | DisplayClass::Cell_Under_Cursor       |
| 40   | +0x0A0 | 0x0040D280 | GScreenClass   | Stub                                  |
| 41   | +0x0A4 | 0x004A9840 | DisplayClass   | DisplayClass::Selected_Object_Logic   |
| 42   | +0x0A8 | 0x004A8960 | DisplayClass   | DisplayClass::Selection handler       |
| 43   | +0x0AC | 0x0040D250 | GScreenClass   | Stub (returns 0)                      |
| 44   | +0x0B0 | 0x00693880 | ScrollClass    | Scroll_Dispatch                       |
| 45   | +0x0B4 | 0x004AC310 | DisplayClass   | DisplayClass::DetermineAction wrapper |
| 46   | +0x0B8 | 0x004AAE90 | DisplayClass   | **SetCursorFromAction**               |
| 47   | +0x0BC | 0x004AC380 | DisplayClass   | DisplayClass::Process_LeftClick       |
| 48   | +0x0C0 | 0x004AB9B0 | DisplayClass   | DisplayClass::Process_RightClick      |
| 49   | +0x0C4 | 0x00693840 | ScrollClass    | Scroll_Finish                         |
| 50   | +0x0C8 | 0x006D0270 | TabClass       | Tab_Input_Handler                     |
| 51   | +0x0CC | 0x00653760 | RadarClass     | Radar helper                          |
| 52   | +0x0D0 | 0x00653F70 | RadarClass     | Radar helper 2                        |
| 53   | +0x0D4 | 0x006D02B0 | TabClass       | Tab helper                            |
| 54   | +0x0D8 | 0x006D04F0 | TabClass       | Tab helper 2                          |
| 55   | +0x0DC | 0x007FABA0 | INoticeSink    | INoticeSink::QueryInterface           |
| 56   | +0x0E0 | 0x0040D850 | INoticeSink    | INoticeSink method                    |
| 57   | +0x0E4 | 0x0040D5A0 | INoticeSink    | INoticeSink method                    |
| 58   | +0x0E8 | 0x0040D720 | INoticeSink    | INoticeSink method                    |
| 59   | +0x0EC | 0x0040D540 | INoticeSink    | INoticeSink method                    |
| 60   | +0x0F0 | 0x0040D7D0 | INoticeSink    | INoticeSink method                    |
| 61   | +0x0F4 | 0x0040D570 | INoticeSink    | INoticeSink method                    |
| 62   | +0x0F8 | 0x0040D590 | INoticeSink    | INoticeSink method                    |
| 63   | +0x0FC | 0x007FABD0 | INoticeSink    | INoticeSink::QueryInterface variant   |
| 64   | +0x100 | 0x0040D800 | INoticeSink    | INoticeSink method                    |
| 65   | +0x104 | 0x0040D5A0 | INoticeSink    | INoticeSink method (dup of slot 57)   |
| 66   | +0x108 | 0x0040D5E0 | INoticeSink    | INoticeSink method                    |
| 67   | +0x10C | 0x0040D690 | INoticeSink    | INoticeSink method                    |
| 68   | +0x110 | 0x0040D6C0 | INoticeSink    | INoticeSink method                    |
| 69   | +0x114 | 0x0040D700 | INoticeSink    | INoticeSink method                    |
| 70   | +0x118 | 0x0040D590 | INoticeSink    | INoticeSink method (dup of slot 62)   |

### 18.2 Override Summary by Class

**MouseClass overrides** (8 slots): 5, 7, 10, 18, 19, 20, 21, 30 *(corrected 2026-05-29: was "(6 slots)"; 8 slots are listed and all verified from vtable binary — OPERATOR_OR_ORDER_DRIFT)*

**ScrollClass overrides** (5 slots): 26, 31, 35, 44, 49

**TabClass overrides** (4 slots): 16, 33, 50, 53, 54

**SidebarClass overrides** (4 slots): 3, 22, 23, 24, 27, 34

**RadarClass overrides** (4 slots): 28, 29, 36, 37, 51, 52

**PowerClass overrides** (1 slot): 25

**DisplayClass overrides** (7 slots): 32, 38, 39, 41, 42, 45, 46, 47, 48

**GScreenClass base** (remaining slots): 0, 1, 2, 4, 6, 8, 9, 11-15, 17, 40, 43

**INoticeSink interface** (slots 55-70): These are the secondary interface vtable
entries inlined into the primary vtable. The INoticeSink interface is used by the
notification/observer pattern in the engine.

### 18.3 Key Observations

- The vtable is unusually large (71 entries) because it includes both the game
  screen hierarchy AND the INoticeSink interface methods (slots 55-70).
- Slots 0-54 are the game screen class hierarchy methods.
- Slots 55-70 are the INoticeSink COM-like interface methods.
- Several slots are stubs (0x0040D230, 0x0040D240, etc.) that just return 0 or 1.
- MouseClass itself only overrides a small number of slots (cursor management).
- The bulk of the functionality is in DisplayClass (click handling, action
  determination, cursor-from-action mapping) and SidebarClass (sidebar drawing).

---

## 19. TechnoClass::What_Action_OnObject and What_Action_OnCell

**Confidence:** ~85%. Core logic verified from binary. Some field names inferred.

These are the per-unit action determination methods called from
`DisplayClass::DetermineAction` (section 11) when evaluating what action a
selected unit would take on a target object or cell.

### 19.1 Call Hierarchy

```
DisplayClass::DetermineAction (0x00692610)
  -> FUN_004dded0(selectedUnit, targetObject, param3)    // wrapper for OnObject
     -> TechnoClass::What_Action_OnObject (0x006FFEC0)   // base impl
     -> InfantryClass::What_Action_OnObject (0x0051E3B0) // infantry override
  -> FUN_004ddde0(selectedUnit, cellCoord, param3, param4) // wrapper for OnCell
     -> FUN_00700600 (TechnoClass base)                   // base impl
     -> InfantryClass::What_Action_OnCell (0x0051F800)    // infantry override
     -> AircraftClass::ActionOnCell (0x004196B0)          // aircraft override
```

### 19.2 Wrapper Functions

**FUN_004dded0** (OnObject wrapper, 0x004DDED0):
1. Calls the overridden `What_Action_OnObject` on the selected unit
2. Gets the target's cell coordinates via vtable+0x48 (GetCoords)
3. Checks if the target cell is shrouded via `IsShrouded`
4. If shrouded AND action != 0 AND not in map editor mode:
   - Reads `TechnoTypeClass+0xC8D` (AttackCursorOnDisguise flag)
   - If flag set: returns action 1 (Move)
   - Otherwise: returns action 2 (NoMove)
5. Otherwise returns the original action unchanged

**FUN_004ddde0** (OnCell wrapper, 0x004DDDE0):
1. Calls `FUN_00700600` (TechnoClass base cell action)
2. Converts cell to lepton world coords (cellX*256+128, cellY*256+128)
3. Gets ground height, checks bridge flag (cell+0x140 & 0x100)
4. Checks shroud via `IsShrouded`
5. If shrouded AND action != 0:
   - If `TechnoTypeClass+0xC8D` set AND cell is in playfield:
     - If action == 0x33 (some special): return 0x33
     - Otherwise: return 1 (Move)
   - Otherwise: return 2 (NoMove)
6. Otherwise returns the original action

### 19.3 TechnoClass::What_Action_OnObject (0x006FFEC0) -- Base Implementation

**454 lines of decompiled code.** Returns an Action enum value.

Parameters:
```c
int What_Action_OnObject(TechnoClass* this, TechnoClass* target, char forceParam)
```

Key logic flow:

1. **Early exits:**
   - If `this+0x298 != 0` (limbo/destroyed flag): return 0 (None)
   - If `this+0x2A8 != 0` AND `TechnoTypeClass+0x692 == 0` (no weapon?): return 0
   - If target is NULL: return 0

2. **Cloaked unit check:**
   - If `this->IsCloaked` (vtable+0x1D4 returns true):
     - If `this->CanMove` (vtable+0x2AC) AND is human player:
       - If target's owner != this's owner: return 0 (can't interact while cloaked)
     - If parasite flag (this+0x3D4) AND cell check: return 0
     - If target is Techno (vtable+0x138) and not disguised (target+0x83 == 0): return 7 (Sabotage)
     - Otherwise: return 0

3. **Self-click / disguise detection:**
   - If target == this AND exactly 1 unit selected AND target has disguise:
     - Reads `g_RulesClass+0xFD5` (`AttackCursorOnDisguise`)
     - Reads `g_RulesClass+0xFD4` (`AttackCursorOnFriendlies`)
     - Returns 0x37 (55 = GuardArea) if either is set

4. **Modifier key checks (Force-Move, Force-Fire, Force-Alt):**
   - If Shift held (DAT_00a8ebf8/ebfc): bForceMove = true
   - If Ctrl held (DAT_00a8ec00/ec04): bForceFire = true
   - If Alt held (DAT_00a8ec08/ec0c): force tote/pickup (returns 8 = Tote)
   - If both Ctrl AND Shift, Shift takes precedence (bForceFire cleared)

5. **Self-deploy:**
   - If target == this AND 1 unit selected AND human player:
     - Returns 4 (Self/Deploy)
   - AI: additional checks for infantry with passengers or deployable units

6. **Force-move + ally check:**
   - If force-move AND ally AND human AND can move (vtable+0xa0):
     - If also force-fire: returns 0x1A (Deploy)
     - Otherwise: returns 1 (Move)

7. **Weapon evaluation (main combat logic):**
   - Gets best weapon via `vtable+0x2E4(target)` -> `vtable+0x3F8(weapon)` -> WeaponTypeClass*
   - Checks if target is visible (vtable+0x7C returns true)
   - **Disguise timer check:** If `AttackCursorOnDisguise` is off, checks
     if target has active disguise (field_0x14 bit 2, timer at +0x1F4/0x1FC)
   - **Ally/enemy determination:** Calls `HouseClass::Is_Ally_ByObject`
   - Weapon ability checks:
     - `WeaponType+0xAC -> WarheadType+0x157` (special warhead flag)
     - `WeaponType+0xAC -> WarheadType+0x14B` (temporal warhead / can erase)
   - If weapon can fire AND target is valid AND (enemy OR force-fire OR special):
     - Returns 5 (Attack)

8. **Sabotage/enter fallback:**
   - If cloaked and can move and ally owner mismatch: return 0
   - If not parasite AND target is Techno AND not disguised AND not cloaked:
     - Returns 7 (Sabotage)
   - Otherwise: return 0 (None)

### 19.4 InfantryClass::What_Action_OnObject (0x0051E3B0) -- Infantry Override

**454 lines.** Extends the base with infantry-specific logic.

Key additions beyond TechnoClass base:

1. **Engineer logic (`TechnoTypeClass+0xEC3` = Engineer flag):**
   - On enemy buildings with `field_0xCCC` set (Capturable): return 9 (Capture)
   - On enemy buildings with `field_0x16B6` set (C4): checks radar color,
     returns 0x20 (32) if valid, 0x1D (29) if not
   - On allied buildings: checks if building needs repair (health < max) ->
     returns 3 (Enter) or 0x1D (29 = ChronoSphere action = repair building)
   - On capturable enemy buildings with `field_0x1572` set:
     - If health <= EngineerCaptureLevel (RulesClass+0x17F8): return 9 (Capture)
     - Otherwise: return 0x1C (28 = NoDeploy) -- can't capture yet

2. **Docking/transport logic:**
   - If target is allied building with `CanDock` returning true: return 9 (Enter/dock)

3. **Unarmed infantry:**
   - If `GetWeaponRange` returns < 0 (no weapon) AND human player:
     - On ally aircraft: checks health, returns 0x1B (27 = Repair)
     - On enemy: returns 0x3B (59 = AttackSupport -- calls for support fire)
     - Falls through to move/sabotage

4. **Garrison logic (`TechnoTypeClass+0xEC6` = CanOccupyBuilding):**
   - If target is a Building (WhatAmI == 6) with occupiable flag:
     - If `BuildingTypeClass+0xC94` set (e.g. bunker full): return 7 (Sabotage/blocked)
     - If different owner: return 9 (Capture/enter)

5. **Healing logic:**
   - Allied buildings with `field_0x16C1` (IsHospital/repairDepot):
     - If infantry health < ConditionYellow: return 3 (Enter for healing)
     - If at full health AND `field_0x16AD` set: return 0x1F (31 = already full)
     - Otherwise: return 0x1D (repair not needed)
   - Allied buildings with `field_0x16C2` (AcademyTraining):
     - If not Elite veterancy: similar enter/already-full logic

6. **Ivan Bomb logic (`TechnoTypeClass+0xEBE` = IvanBombAttach):**
   - On enemy buildings with `field_0x1572` (capturable) or `field_0x1576` (garrisonable):
     - Returns 9 (Capture) with zone pathfinding check for reachability
   - On enemies without weapon: returns 0x3B (AttackSupport)

7. **Sabotage/C4 logic (`TechnoTypeClass+0xEC2/EC4`):**
   - On buildings with `Sabotage` or `C4` flag and `field_0x1577` set (Wall=yes):
     - Returns 0x10 (16 = Damage) -- wall breaching
   - Standard C4: returns 5 (Attack)

8. **IvanBomb disarm (`TechnoTypeClass+0xEAE`):**
   - If target has active bomb (field_0x22E set, field_0x38 == 0):
     - Returns 0x35 (53 = EnterWaypoint -- actually "bomb present, can disarm")
   - If bomb already defused: returns 0x36 (54 = "bomb defused" cursor)

9. **Occupying check:**
   - If infantry is in a transport/occupier state (mission 0x1B-0x1E):
     - Move action -> NoMove (return 2)
     - Attack action -> checks weapon range, may return 2
     - Self action -> checks deploy, returns 0x1E (NoDeploy) if unable

10. **Tunnel/warp entry:**
    - If action is NoMove and target is enemy tunnel or warp building with
      `field_0x16AD` set:
      - Returns 0xB (Repair) if same owner, or 0x1F (31) if not

### 19.5 InfantryClass::What_Action_OnCell (0x0051F800) -- Infantry Cell Override

**119 lines.** Much simpler than OnObject.

Key logic:

1. If not human player: return 0 (no cursor feedback for AI)

2. Calls base `FUN_004DDDE0` to get initial action

3. **Unarmed infantry override:**
   - If `GetWeaponRange < 0` AND action == 5 (Attack): return 0x1A (Deploy)

4. **Tunnel movement (`TechnoTypeClass+0xD94` = teleporter/tunnel flag):**
   - If action is Move (1) or NoMove (2): checks if cell contains tunnel entrance
     via `FUN_00484AE0` / `FUN_00484D60`
   - If tunnel found: return 0 (None -- no normal move into tunnel cell)

5. **Occupant override (mission 0x1B-0x1E):**
   - If in occupier state:
     - If action == Move: return 2 (NoMove)
     - If action == Attack: checks weapon range, may return 2

6. **Bridge check:**
   - If action == Move and cell is a low bridge:
     - Checks if infantry can pass via `FUN_00484F10`
     - Returns 0x24 (36 = Garrison/Enter) if can pass, 0x23 (35 = GuardArea) if not

7. **Engineer on garrisonable building in cell:**
   - If Engineer flag set AND cell contains building with garrisonable/capturable flags:
     - Same C4/capture logic as OnObject

8. **Attack without weapon:**
   - If action == 5 AND `vtable+0x2AC` (CanAttackMove) returns false:
     - Returns 0x3B (AttackSupport)

9. **Final fallback:**
   - If human player AND action == 0 AND not teleporter AND can move:
     - Returns 2 (NoMove)

### 19.6 TechnoClass::What_Action_OnCell (0x00700600) -- Base Cell Implementation

**224 lines.** The base implementation for all unit types.

Key logic:

1. **Early exits:** Same as OnObject (limbo check, no-weapon check)

2. **Modifier keys:** Same Ctrl/Alt/Shift checks as OnObject

3. **Cloaked/warping check:**
   - If unit is cloaked or warping: early logic differences

4. **Cell object iteration (fog-of-war enabled only):**
   - If SpecialFlags & 0x1000 (fog active) AND forceParam set:
     - Iterates objects in the cell's occupant list
     - If finds a waypoint (type 0x14): stores waypoint owner
     - If finds an enemy building (type 6): marks as attackable

5. **Force-move to fire position:**
   - If human AND (force-fire AND force-alt OR unit has attack move flag):
     - If can move AND can shoot AND cell is not shrouded:
       - Returns 0x1A (Deploy -- fire at position)
     - If shrouded: returns 0x33 (51 = special shroud fire)

6. **Weapon fire at cell:**
   - Gets primary weapon via `vtable+0x3F4`
   - Checks cell terrain type via `FUN_00486900`
   - If weapon has `WarheadType+0x14C` set: special handling
   - If (forceParam OR cell has enemy building OR terrain target OR special):
     - Checks weapon flags for wall-attack, ground-attack ability
     - Checks pathfinding distance if needed
     - Returns 5 (Attack) if weapon can fire at this cell

7. **Movement:**
   - If human AND (can move OR can paradrop):
     - If cell is in playfield:
       - Force-Alt: return 1 (Move)
       - Can paradrop AND force-move: return 1
       - Can move: checks `ThreatPosing` via vtable+0x1AC:
         - If threat > 1: additional check for deployable buildings
           (returns 2/NoMove if blocked, 1/Move if passable)
         - If no threats: return 1 (Move)
     - If cell is NOT in playfield: return 2 (NoMove)

8. **Fallback:** return 0 (None)

### 19.7 Key TechnoTypeClass Fields Used

| Byte Offset | Type | INI Key (inferred)            | Usage |
|-------------|------|-------------------------------|-------|
| +0x692      | bool | ResourceGatherer?             | Disables action when no weapon |
| +0x6AC      | bool | Parasiteable / HasC4          | C4/weapon special check |
| +0x6C0      | bool | AttackCursorOnFriendlies (1)  | Shows attack cursor on allies |
| +0x6C1      | bool | AttackCursorOnFriendlies (2)  | Variant for AI context |
| +0xC8D      | bool | AttackCursorOnDisguise        | Per-type flag |
| +0xC94      | bool | Bunker max occupancy flag     | Building garrison full |
| +0xCCC      | bool | Capturable                    | Building can be captured |
| +0xD2C      | bool | Naval flag?                   | Movement type check |
| +0xD30      | bool | Cloakable                     | Disguise visibility |
| +0xD6A      | bool | NoEnterTransport              | Prevents transport entry |
| +0xD94      | bool | Teleporter / Tunnel entry     | Tunnel movement flag |
| +0xEAE      | bool | IvanBombAttach (read IvanBomb) | Can place bombs |
| +0xEBE      | bool | Agent / Saboteur              | Sabotage action |
| +0xEC2      | bool | Sabotage (flag 1)             | C4 sabotage |
| +0xEC3      | bool | Engineer                      | Can capture/repair buildings |
| +0xEC4      | bool | C4 (flag)                     | Has C4 charges |
| +0xEC6      | bool | CanOccupyBuilding             | Can garrison buildings |
| +0xEC8      | bool | Occupier (in transport)        | Currently in transport/garrison |

---

## 20. Tactical__PickObjectAtScreenPoint (0x006DA380)

**Confidence:** ~85%. Core logic verified. Some field semantics inferred.

### 20.1 Function Signature

```c
ObjectClass* Tactical__PickObjectAtScreenPoint(
    TacticalClass* this,
    int* screenPoint     // {x, y} screen coordinates
)
```

### 20.2 Algorithm

The function determines which game object is under the cursor at the given
screen coordinates. It uses a **distance-based priority** system.

**Phase 1: Iterate tracked objects**

The TacticalClass maintains a tracked-object array at global `DAT_00b0cec8`.
Each entry is 12 bytes (3 ints): `{ObjectClass*, screenX, screenY}`.
The count is at `TacticalClass+0xDB0`.

For each tracked object:

1. **Skip NULL entries**

2. **If no unit is selected** (FUN_0040dd20 returns NULL):
   - Gets the object's type via vtable+0x88 (GetObjectTypeClass)
   - If type's WhatAmI returns 0x25 (Waypoint) AND `type+0x2B4 != 0`:
     - Skip this object (waypoints are only pickable when units selected)

3. **If a unit IS selected:**
   - Check `object+0x90 != 0` (Selectable flag) AND `selectedUnit+0x81 == 0`
   - If `selectedUnit+0x41A == 0` AND `selectedUnit+0x220 == 2` (Guard mission):
     - Check sensor coverage at the object's cell for the player's house
     - If no sensor coverage: skip this object
   - If passes checks: `bValid = true`

4. **Distance calculation:**
   - `dx = objectScreenX - (viewport.x + mouseX)`
   - `dist = dx * dx` (note: only X distance, simplified)
   - Converted via `Math__ftol()` to uint

5. **Selection threshold:** Must be `< 200` pixels (distance squared)
   - If closer than the current best: update best match

**Phase 2: Cell fallback**

If no tracked object was found:
1. Convert screen point to map coordinates via `FUN_006d6590`
2. Get the CellClass at those coordinates via `MapClass::Get_CellClass`
3. Return `CellClass+0xE4` (FirstObject pointer) -- the first object in that cell

### 20.3 Key Data Structures

| Address / Offset | Type          | Description |
|------------------|---------------|-------------|
| DAT_00b0cec8     | int[N*3]      | Tracked object array: {obj*, screenX, screenY} per entry |
| this+0xDB0       | int           | Count of tracked objects |
| this+0xB0        | int           | Viewport X offset (scroll position) |
| this+0xB4        | int           | Viewport Y offset (scroll position) |
| CellClass+0xE4   | ObjectClass*  | First object in cell's occupant list |

### 20.4 Observations

- Object picking is based on **screen-space X distance only** (not full 2D distance).
  This is a simplification -- the Y component is computed (`local_18`) but not used
  in the distance check in the decompiled code (possibly an artifact of decompiler
  optimization or the original code used a combined metric via `Math__ftol`).
- The 200-pixel threshold means objects must be very close to the cursor to be picked.
- Waypoints are NOT pickable unless a unit is selected.
- Sensor coverage affects picking in fog-of-war mode (TS legacy, but code is present).
- The fallback path picks whatever object occupies the cell under the cursor, which
  is how you can click on buildings and terrain objects.

---

## 21. Cursor Data Entries 66-85

**Confidence:** ~95%. Raw binary data directly decoded.

The cursor data table at 0x0082D028 contains exactly **86 entries** (indices 0-85).
Entry 86's data area contains the string "Stretching movie..." confirming the end.

Entries 0-65 were decoded in section 4. Here are the remaining entries 66-85.
Context for names comes from the action-to-cursor mappings in section 12.

| ID  | Name (inferred)          | Start | Count | Rate | Mini | MiniCnt | HotX   | HotY   |
|-----|--------------------------|-------|-------|------|------|---------|--------|--------|
| 66  | Placeholder6             | 390   | 1     | 0    | -1   | -1      | Center | Center |
| 67  | Placeholder7             | 391   | 1     | 0    | -1   | -1      | Center | Center |
| 68  | Placeholder8             | 392   | 1     | 0    | -1   | -1      | Center | Center |
| 69  | Placeholder9             | 393   | 1     | 0    | -1   | -1      | Center | Center |
| 70  | GeneticConverter         | 394   | 10    | 4    | -1   | -1      | Center | Center |
| 71  | SpyPlane / AttackMove    | 404   | 9     | 4    | 63   | 5       | Center | Center |
| 72  | PsychicDominator         | 413   | 9     | 4    | -1   | -1      | Center | Center |
| 73  | (SW cursor 3)            | 422   | 9     | 4    | -1   | -1      | Center | Center |
| 74  | NoTogglePower            | 431   | 1     | 0    | -1   | -1      | Center | Center |
| 75  | TogglePower              | 432   | 1     | 0    | -1   | -1      | Center | Center |
| 76  | NoGRepair                | 433   | 1     | 0    | -1   | -1      | Center | Center |
| 77  | NoEnterMode              | 434   | 1     | 0    | -1   | -1      | Center | Center |
| 78  | NukeTarget               | 435   | 15    | 4    | -1   | -1      | Center | Center |
| 79  | PlaceBeacon              | 450   | 10    | 4    | -1   | 1       | Center | Center |
| 80  | AttackSupport            | 460   | 10    | 4    | -1   | -1      | Center | Center |
| 81  | SelectBeacon             | 470   | 10    | 4    | -1   | -1      | Center | Center |
| 82  | SelectNode               | 480   | 8     | 4    | -1   | -1      | Center | Center |
| 83  | DisguiseDetect / Cursor  | 488   | 8     | 4    | 516  | -1      | Center | Center |
| 84  | DisarmBomb               | 496   | 8     | 4    | 515  | 1       | Center | Center |
| 85  | DetonateAll              | 504   | 8     | 4    | 512  | 1       | Center | Center |

### 21.1 Name Justifications

Names are inferred from the action-to-cursor mappings in section 12.8:

- **70 (GeneticConverter):** Mapped from action 0x46 (GeneticConverter) per enum table
- **71 (SpyPlane/AttackMove):** Mapped from action 0x47 (SpyPlane). Also used by
  FUN_00731CB0 which returns 0x47 for attack-move variants (3E/3F). Has a mini
  variant sharing frames 63-67 with Attack cursor (cross-reference pattern).
- **72 (PsychicDominator):** Mapped from action 0x48 (PsychicDominator) per enum table
- **73 (SW cursor 3):** Animated 9-frame cursor, likely another superweapon variant
- **74-77:** Static 1-frame cursors for mode toggles (power, repair, enter)
- **78 (NukeTarget):** Mapped from action 0x3C (DontUse6 = NukeTarget). 15 frames
  animated -- the nuclear strike targeting cursor
- **79 (PlaceBeacon):** Mapped from action 0x45. Has MiniCount=1 (mini variant exists)
- **80 (AttackSupport):** Mapped from action 0x46
- **81 (SelectBeacon):** Mapped from action 0x44
- **82 (SelectNode):** Mapped from action 0x47
- **83-85:** Late-game YR cursors. Frame numbers 488-511 suggest these were added
  in the YR expansion. Entry 83 has mini start at frame 516, entries 84-85 have
  mini starts at 515 and 512 respectively.

### 21.2 Table Statistics

- **Total entries:** 86 (indices 0-85)
- **Total MOUSE.SHA frames referenced:** up to frame 519 (entry 83 mini start 516 +
  mini frames would extend further, but MiniCount = -1 means no mini for 83)
- **Animated cursors** (Rate > 0): 35 entries
- **Static cursors** (Rate == 0): 51 entries
- **Entries with mini variants** (MiniStart != -1): 12 entries
- **All entries 66-85** use Center/Center hotspot

---

## 22. Band-Box / Rubber-Band Selection

**Confidence:** ~90%. All band-box functions decompiled from TacticalClass.

### 22.1 Overview

The band-box (rubber-band selection rectangle) is implemented in **TacticalClass**,
not in DisplayClass or MouseClass. It uses 4 fields in TacticalClass for the
rectangle coordinates and has 6 related functions.

### 22.2 TacticalClass Band-Box Fields

| Offset | Type | Description |
|--------|------|-------------|
| +0xD90 | int  | Band rect start X (screen coordinates) |
| +0xD94 | int  | Band rect start Y |
| +0xD98 | int  | Band rect end X (updated as mouse moves) |
| +0xD9C | int  | Band rect end Y |
| +0xDB0 | int  | Tracked object count (used by iteration) |

### 22.3 Functions

**Tactical__InitBandRect (0x006D9F80):**
```c
void InitBandRect(TacticalClass* this, int* point) {
    if (this->bandStartX == 0 && this->bandStartY == 0) {
        this->bandStartX = point[0];   // +0xD90
        this->bandStartY = point[1];   // +0xD94
        this->bandEndX = point[0];     // +0xD98
        this->bandEndY = point[1];     // +0xD9C
    }
}
```
Called on left mouse button down. Only initializes if no band-box is active
(both start coords are 0). Sets both start and end to the click position.

**Tactical__UpdateBandRectEnd (0x006D9FC0):**
```c
void UpdateBandRectEnd(TacticalClass* this, int* point) {
    if (this->bandStartX != 0 || this->bandStartY != 0) {
        this->bandEndX = point[0];     // +0xD98
        this->bandEndY = point[1];     // +0xD9C
    }
}
```
Called on mouse move while left button held. Updates the end point only.

**Tactical__ClearBandRect (0x006DA160):**
```c
void ClearBandRect(TacticalClass* this) {
    this->bandStartX = 0;  // +0xD90
    this->bandEndX = 0;    // +0xD98
    this->bandStartY = 0;  // +0xD94
    this->bandEndY = 0;    // +0xD9C
}
```
Resets all four coords to 0, deactivating the band-box.

**Tactical__DrawBandBoxRect (0x006DA180):**
Draws the band-box rectangle on screen.
1. Normalizes the rectangle (ensures min/max ordering for start vs end).
2. Reads a color from the cursor palette surface:
   - If 8-bit mode: reads byte at `paletteData + 0x0F` (palette index 15)
   - If 16-bit mode: reads ushort at `paletteData + 0x1E` (palette index 15)
3. Calls `g_PrimarySurface->DrawRect(viewport, rect, color)` (vtable+0x54)
   to render the rectangle outline.

The rectangle color comes from palette entry 15 of the mouse cursor palette,
which is a bright white/green color in the standard palette.

**Tactical__ProcessBandBoxSelection (0x006D9FF0):**
Called on left mouse button up. Processes the selection:
1. If band-box is active (start != 0 OR start != 0):
2. Normalizes rectangle coordinates (min/max ordering)
3. Computes width and height
4. Calls `Tactical__IterateObjectsInRect(&rect, callback)` to process all
   objects within the rectangle
5. Clears the band-box (sets start to 0,0)

**Tactical__IterateObjectsInRect (0x006DA5C0):**
The core selection logic. Iterates all tracked objects and selects those inside
the rectangle.

For each tracked object in the `DAT_00b0cec8` array:
1. Skip NULL entries and entries with `object+0x90 == 0` (not selectable)
2. Convert object's screen position relative to viewport:
   - `objX = entry.screenX - viewport.x` (TacticalClass+0xB0)
   - `objY = entry.screenY - viewport.y` (TacticalClass+0xB4)
3. Check if `(objX, objY)` is inside the rectangle bounds
4. If inside AND `FUN_00732D00` returns false (not a special excluded type):
   - If callback is NULL (normal selection mode):
     - Skip buildings (WhatAmI == 6) UNLESS they are 1x1 with undeploy capability
       (`BuildingTypeClass__Is1x1WithUndeploy` returns true)
     - Check if object's owner is the human player
     - Check if object is a Techno (vtable+0x138) and not a building (or 1x1 deployable)
     - Check vtable+0x14C (is selectable by player?)
     - If all pass: set `DAT_00822CF2 = 0` (found at least one selectable object)
   - If callback is provided: call `callback(object)` for custom processing

5. After iteration: reset `DAT_00822CF2 = 1`

**Tactical__AnyObjectInBandRect (0x006DA080):**
Quick check: returns 1 if any selectable object (`object+0x90 != 0`) has its
screen position inside the current band rectangle. Returns 0 if none found.
Used to determine if the band-box drag should be shown (don't show for tiny drags
with no units).

### 22.4 Selection Rules

- **Buildings are excluded** from band-box selection unless they are 1x1 deployable
  structures (e.g., deployed MCV). This prevents accidentally selecting buildings.
- **Only owned units** are selected (human player check).
- **Only Techno objects** are selected (infantry, vehicles, aircraft -- not terrain,
  overlays, or other abstract objects).
- The `vtable+0x14C` check likely corresponds to the `Selectable=` INI key.
- The tracked object array is populated during rendering -- objects that are
  off-screen are not in the array and cannot be band-selected.

---

## 23. CommandClass / Hotkey System

**Confidence:** ~85%. Command registration fully traced. Key binding mechanism
verified structurally. Individual command vtable semantics partially inferred.

### 23.1 Architecture Overview

The hotkey system uses a **command pattern**. Each hotkey action is represented by
a small CommandClass-derived object (typically 4 or 8 bytes: vtable pointer +
optional parameter). All commands are registered into a global DynamicVector at
startup.

### 23.2 Command Registration

All commands are registered in `ToggleRepairCommandClass__Constructor` (0x00532150)
-- despite the misleading name, this function is actually the **global command
registration** function. It creates all command objects and adds them to the
command vector.

**Global command vector:**
| Address      | Type | Description |
|--------------|------|-------------|
| 0x0087F65C   | ptr  | Pointer to command array buffer |
| 0x0087F660   | int  | Capacity of command array |
| 0x0087F668   | int  | Current count of registered commands |
| 0x0087F665   | byte | Auto-grow flag |
| 0x0087F66C   | int  | Growth increment |
| 0x0087F658   | ptr  | Allocator vtable pointer |

### 23.3 Complete Command List

Commands are registered in this order (verified from binary):

| #  | Class Name                    | Size | Param | Description |
|----|-------------------------------|------|-------|-------------|
| 1* | MultiplayerDebugCommandClass  | 4    | -     | Debug (only if DAT_00a8b8b4 set) |
| 2* | MultiplayerSyncCommandClass   | 4    | -     | Sync debug (same condition) |
| 3  | FollowCommandClass            | 4    | -     | Follow selected unit |
| 4  | View1CommandClass             | 4    | -     | Recall view bookmark 1 |
| 5  | View2CommandClass             | 4    | -     | Recall view bookmark 2 |
| 6  | View3CommandClass             | 4    | -     | Recall view bookmark 3 |
| 7  | View4CommandClass             | 4    | -     | Recall view bookmark 4 |
| 8  | SetView1CommandClass          | 4    | -     | Set view bookmark 1 |
| 9  | SetView2CommandClass          | 4    | -     | Set view bookmark 2 |
| 10 | SetView3CommandClass          | 4    | -     | Set view bookmark 3 |
| 11 | SetView4CommandClass          | 4    | -     | Set view bookmark 4 |
| 12 | OptionsCommandClass           | 4    | -     | Open options menu |
| 13 | SidebarUpCommandClass         | 4    | -     | Scroll sidebar up |
| 14 | SidebarDownCommandClass       | 4    | -     | Scroll sidebar down |
| 15 | CenterREventCommandClass      | 4    | -     | Center on last radar event |
| 16 | BeaconPlacementCommandClass   | 4    | -     | Place beacon on map |
| 17 | ToggleSellCommandClass        | 4    | -     | Toggle sell mode |
| 18 | ToggleRepairCommandClass      | 4    | -     | Toggle repair mode |
| 19 | AllianceCommandClass          | 4    | -     | Toggle alliance |
| 20 | CenterBaseCommandClass        | 4    | -     | Center view on base |
| 21 | CenterViewCommandClass        | 4    | -     | Center view on selection |
| 22 | ScatterCommandClass           | 4    | -     | Scatter selected units |
| 23 | GuardCommandClass             | 4    | -     | Guard mode |
| 24 | StopCommandClass              | 4    | -     | Stop all selected units |
| 25 | AllToCheerCommandClass        | 4    | -     | All units cheer |
| 26 | DeployCommandClass            | 4    | -     | Deploy selected unit |
| 27 | PrevObjectCommandClass        | 4    | -     | Select previous object |
| 28 | NextObjectCommandClass        | 4    | -     | Select next object |
| 29 | PlanningModeCommandClass      | 4    | -     | Toggle waypoint planning |
| 30 | CombatantSelectCommandClass   | 4    | -     | Select all combat units |
| 31 | TypeSelectCommandClass        | 4    | -     | Select all of same type |
| 32 | HealthNavCommandClass         | 4    | -     | Navigate by health |
| 33 | VeterancyNavCommandClass      | 4    | -     | Navigate by veterancy |
| 34 | SetStructureTabCommandClass   | 4    | -     | Switch to structure tab |
| 35 | SetDefenseTabCommandClass     | 4    | -     | Switch to defense tab |
| 36 | SetUnitTabCommandClass        | 4    | -     | Switch to unit tab |
| 37 | SetInfantryTabCommandClass    | 4    | -     | Switch to infantry tab |
| 38-47 | CreateTeamCommandClass     | 8    | 1-10  | Create team 1-10 (Ctrl+0-9) |
| 48-57 | SelectTeamCommandClass     | 8    | 1-10  | Select team 1-10 (0-9) |
| 58-67 | AddTeamCommandClass        | 8    | 1-10  | Add to team 1-10 (Shift+0-9) |
| 68-77 | CenterTeamCommandClass     | 8    | 1-10  | Center on team 1-10 |
| 78-85 | TauntCommandClass          | 8    | 1-8   | Send taunt 1-8 (F1-F8) |
| 86 | ScreenCaptureCommandClass     | 4    | -     | Screenshot |
| 87 | PageUserCommandClass          | 4    | -     | Page/ping user |
| 88 | CursorPositionCommandClass    | 4    | -     | Debug cursor position |
| 89 | DeleteCommandClass            | 4    | -     | Delete (debug/editor) |

*Commands 1-2 only registered if `DAT_00a8b8b4` is nonzero (debug/multiplayer
debug mode active). Not present in normal gameplay.

### 23.4 CommandClass VTable Structure

Each CommandClass has a vtable with approximately 7-8 entries:

| VTable Slot | Offset | Purpose |
|-------------|--------|---------|
| 0           | +0x00  | Destructor (scalar deleting) |
| 1           | +0x04  | GetName -- returns command name string |
| 2           | +0x08  | GetDescription -- returns description string |
| 3           | +0x0C  | Execute -- main handler, called when hotkey pressed |
| 4           | +0x10  | GetDefaultKey -- returns default VK_* key code |
| 5           | +0x14  | CanExecute -- returns true if command is valid now |
| 6           | +0x18  | GetCategory? / secondary handler |
| 7           | +0x1C  | RTTI info |

### 23.5 Key Binding Dispatch

Key bindings are processed in `CommandBar_Dispatch` (0x006D0680), which is called
from the main input pipeline after sidebar/display processing.

The dispatch uses **keyboard scan codes** mapped to command indices. Key events
arrive as 16-bit codes in the range 0x80D6-0x80EE (normal keys) and
0xC0D6-0xC0EE (modified keys with Ctrl/Alt/Shift).

**Key event processing:**
1. Strip modifier bits: `keyIndex = (event & 0xFFFF7FFF) - 0xD6`
2. Compare against stored key bindings in global arrays starting at `DAT_00b0cb20`
3. Each comparison triggers a specific handler function

**Known key binding globals:**

| Global       | Slot                  |
|--------------|-----------------------|
| DAT_00b0cd24 | Follow key            |
| DAT_00b0cb3c | Center view key       |
| DAT_00b0c1b8 | Options key           |
| DAT_00b0cb20 | Sidebar scroll key    |
| DAT_00b0cb68 | Guard key             |
| DAT_00b0cc1c | Power toggle key      |
| DAT_00b0cb6c | Stop key              |
| DAT_00b0cb38 | Alliance key          |
| DAT_00b0cc20 | Team key range start  |
| DAT_00b0cd28 | Team key range end    |

**Special key events:**
- `0x80F0`: Left-click on sidebar -- plays click sound, processes sidebar selection
  with waypoint path dispatch
- `0x80F1`: Right-click on sidebar -- toggles sidebar mode with `DAT_00a8b538` check

**Team key dispatch:**
Team keys (0-9, with Ctrl/Shift modifiers) are handled as a range check:
- If `keyIndex` is between `DAT_00b0cc20` and `DAT_00b0cd28`:
  - Checks if Ctrl is held (`FUN_00730a10`): creates team
  - Checks if Shift is held (`FUN_00730990`): adds to team
  - Otherwise: recalls team (`ControlGroup__Recall`)
  - If Alt held: centers on team (`FUN_007313a0`)

### 23.6 Mode Toggle Commands

Several commands toggle global mode flags (from section 11.13):

| Command                  | Flag Address | Effect |
|--------------------------|-------------|--------|
| ToggleSellCommandClass   | 0x00880998  | Toggles g_SellMode |
| ToggleRepairCommandClass | 0x0088099a  | Toggles g_RepairMode |
| DeployCommandClass       | -           | Sends deploy order to selected |
| GuardCommandClass        | -           | Sends guard order to selected |
| StopCommandClass         | -           | Sends stop order to selected |
| ScatterCommandClass      | -           | Sends scatter order to selected |

### 23.7 INI Key Configuration

Key bindings are stored in `ra2md.ini` (or `ra2.ini` for base game) under the
`[Hotkey]` section. The engine reads this file using `CDFileClass` at the end of
command registration (address 0x00538990 area). Default keys are provided by each
CommandClass's `GetDefaultKey` vtable method but can be overridden by the INI file.

The key binding serialization uses VK_* scan codes. The function at 0x005387D0
writes key bindings, 0x005388A0 reads/removes existing bindings, and 0x00538980
checks if a binding exists.

### 23.8 TS Legacy Notes

- The `AllToCheerCommandClass` exists and is registered, but the cheer animation
  may not have visible effect in all situations in YR.
- `CursorPositionCommandClass` and `DeleteCommandClass` are debug/editor commands
  that may have no effect in normal skirmish gameplay.
- The `MultiplayerDebugCommandClass` and `MultiplayerSyncCommandClass` are gated
  behind a debug flag and will never appear in retail builds.

---

## 24. Key Addresses Addendum (Sections 18-23)

| Address    | Description |
|------------|-------------|
| 0x006DA380 | Tactical__PickObjectAtScreenPoint |
| 0x006DA5C0 | Tactical__IterateObjectsInRect |
| 0x006DA180 | Tactical__DrawBandBoxRect |
| 0x006DA080 | Tactical__AnyObjectInBandRect |
| 0x006D9F80 | Tactical__InitBandRect |
| 0x006D9FC0 | Tactical__UpdateBandRectEnd |
| 0x006D9FF0 | Tactical__ProcessBandBoxSelection |
| 0x006DA160 | Tactical__ClearBandRect |
| 0x006FFEC0 | TechnoClass::What_Action_OnObject |
| 0x0051E3B0 | InfantryClass::What_Action_OnObject |
| 0x0051F800 | InfantryClass::What_Action_OnCell |
| 0x00700600 | TechnoClass::What_Action_OnCell (base) |
| 0x004DDED0 | What_Action_OnObject wrapper (shroud check) |
| 0x004DDDE0 | What_Action_OnCell wrapper (shroud check) |
| 0x004196B0 | AircraftClass::ActionOnCell |
| 0x00532150 | Global command registration function |
| 0x006D0680 | CommandBar_Dispatch (key binding handler) |
| 0x00538780 | DynamicVector::Add (command registration helper) |
| 0x0087F65C | Command vector buffer pointer |
| 0x0087F668 | Command vector count |
| 0x00B0CEC8 | Tracked object array (for picking/band-box) |
| 0x00822CF2 | Band-box "found selectable" flag |
| TacticalClass+0xD90 | Band rect start X |
| TacticalClass+0xD94 | Band rect start Y |
| TacticalClass+0xD98 | Band rect end X |
| TacticalClass+0xD9C | Band rect end Y |
| TacticalClass+0xDB0 | Tracked object count |
| TacticalClass+0xB0  | Viewport X offset |
| TacticalClass+0xB4  | Viewport Y offset |
