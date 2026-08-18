# Sidebar Strips, Tabs, and Cameo Management -- Ghidra Research Report

Source: live decompilation of `gamemd.exe` via Ghidra MCP.
Source file per RTTI string: `D:\ra2mdpost\Sidebar.CPP`

---

## Class Hierarchy

```
SidebarClass
  ├── 4x StripClass                  (one per tab)
  │     └── 75x CameoEntry           (inline array per strip)
  ├── 4x TabButtonGadget             (tab button UI gadgets at 0xB07C48, stride 0x60)
  ├── ScrollUpGadget                 (ID 0xC9 = 201)
  ├── ScrollDownGadget               (ID 0xC8 = 200)
  ├── RepairGadget                   (ID 0x65 = 101)
  ├── SellGadget                     (ID 0x66 = 102)
  └── SelectClass[0xF0]             (cameo click gadgets at 0xB07E80, stride 0x38)
```

RTTI names found:
- `.?AVSidebarClass@@`
- `.?AVSBGadgetClass@SidebarClass@@`
- `.?AVSelectClass@StripClass@SidebarClass@@`

---

## SidebarClass Layout (offsets from SidebarClass `this`)

The SidebarClass is embedded in the player class. All offsets below are from the
SidebarClass base pointer (param_1 in decompiled functions).

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0x0000 | 4 | vtable_ptr | Points to SidebarClass vtable at 0x7F3058 |
| ... | | (inherited GadgetClass fields) | |
| 0x1544 | 4x0xF94 | Strips[4] | 4 StripClass instances, each 0xF94 (3988) bytes |
| 0x539C | 4 | CurrentTab | Index of active tab (0-3) |
| 0x53A0 | 4 | FlashTimer | Frame counter for flash effect |
| 0x53A5 | 1 | IsActive | Sidebar visible/active flag |
| 0x53A6 | 1 | NeedsRedraw | Set to 1 to trigger redraw |
| 0x53A7 | 1 | ForceFullRedraw | Forces complete sidebar repaint |
| 0x53A8 | 1 | TopBarDirty | Top bar needs refresh |
| 0x5398 | 4 | TabFlashState | Non-zero = tab buttons are flashing |
| 0x5394 | 4 | TabFlashFrame | Current animation frame for tab flash |

### Strip Array Access
- Strip[N] starts at: `this + 0x1544 + N * 0xF94`
- `this + N * 0x3E5 + 0x551` (when accessed as int* array, multiply by 4)

---

## StripClass Layout (offsets from StripClass `this`)

Each StripClass is 0xF94 (3988) bytes. There are exactly 4 strips.

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0x00 | 4x7 | AnimState[7] | Button animation state machine (7 ints) |
| 0x1C | 1 | IsActive | Strip is enabled/visible |
| 0x1D | 1 | | (padding) |
| 0x1E | 1 | | (state flag) |
| 0x20 | 4 | XPos | Left edge X coordinate |
| 0x24 | 4 | YPos | Top edge Y coordinate |
| 0x28-0x34 | | (flags/state) | |
| 0x38 | 4 | TabIndex | Which tab this strip belongs to (0-3) |
| 0x3C | 1 | NeedsRedraw | Strip-level redraw flag |
| 0x3D | 1 | AutoBuild | Auto-production flag |
| 0x3E | 1 | ScrollDirection | 0 = scrolling up, 1 = scrolling down |
| 0x3F | 1 | IsScrolling | Currently animating a scroll |
| 0x40-0x43 | | (reserved) | |
| 0x44 | 4 | ScrollPosition | Current scroll offset in rows |
| 0x48 | 4 | ScrollRequest | Pending scroll amount |
| 0x4C | 4 | ScrollPixelOffset | Pixel offset during scroll animation |
| 0x50 | 4 | PrevScrollPixelOffset | Previous frame's pixel offset |
| 0x54 | 4 | CameoCount | Number of cameos in this strip |
| 0x58 | 75*0x34 | Cameos[75] | Array of CameoEntry structs |

### Maximum Cameos Per Strip
`0x4B = 75` entries maximum (hardcoded in FUN_006a80a0; loop count `iVar3 = 0x4b` confirmed via decompile_function 0x006A80A0 2026-05-28).

---

## CameoEntry Struct (0x34 = 52 bytes each)

Each cameo slot within a strip is 0x34 bytes (13 DWORDs).

> **2026-05-20 canonicalisation.** The +0x18..+0x30 labels in the table
> below are partially wrong (the +0x28 / +0x30 labels were swapped,
> +0x1C..+0x28 is a CDTimerClass-style block, and +0x2C is the per-step
> increment). The canonical layout lives in
> [FACTORYCLASS_AND_CAMEOENTRY_STRUCT_LAYOUT.md](FACTORYCLASS_AND_CAMEOENTRY_STRUCT_LAYOUT.md)
> §4. Use that doc as the source of truth for CameoEntry. This table
> stays here for the +0x00..+0x14 fields which are correct.

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0x00 | 4 | TypeIndex | Index into the type array |
| 0x04 | 4 | RTTIType | RTTI type ID (see table below) |
| 0x08 | 4 | AltTypeIndex | Alt index (for infantry/unit naval check) |
| 0x0C | 4 | FactoryPtr | Pointer to FactoryClass (0 = no factory) |
| 0x10 | 4 | Status | 0=None, 1=Building, 2=OnHold, 3=Ready |
| 0x14 | 4 | ProgressValue | Production progress (0-0x34 for bar display) |
| 0x18-0x1B | 4 | AnimFrame | Current frame counter for anim |
| 0x1C | 4 | AnimStartTime | Frame at which current anim started |
| 0x20 | 4 | (timer data) | |
| 0x24 | 4 | AnimSpeed | Frames per animation step |
| 0x28 | 4 | FlashEndFrame | Frame at which flash effect ends |
| 0x2C | 4 | (reserved) | |
| 0x30 | 4 | FlashTimer | Current flash countdown |

### CameoEntry Status Values
- `0` = Empty / not building
- `1` = Building (production in progress)
- `2` = On Hold (production paused)
- `3` = Ready (production complete)

---

## RTTI Type-to-Tab Mapping (from SidebarClass::AddCameo @ 0x6A6300)

The function maps RTTI type codes to strip/tab indices (0-3):

| RTTIType | Meaning | Tab Index | Tab Name |
|----------|---------|-----------|----------|
| 0x0F (15) | Infantry | 2 | Infantry |
| 0x10 (16) | InfantryType | 2 | Infantry |
| 0x01 (1) | Unit (Vehicle) | 3 | Vehicles/Aircraft |
| 0x28 (40) | UnitType | 3 | Vehicles/Aircraft |
| 0x02 (2) | Aircraft | 3 | Vehicles/Aircraft |
| 0x03 (3) | AircraftType | 3 | Vehicles/Aircraft |
| 0x06 (6) | Building | **0 or 1** | BuildCat==5→1 (Defense), else→0 (Structures) |
| 0x07 (7) | BuildingType | **0 or 1** | BuildCat==5→1 (Defense), else→0 (Structures) |
| 0x39 (57) | SuperWeaponType | 1 | Defense |
| 0x20 (32) | SuperWeapon2 | 1 | Defense |
| 0x1F (31) | SuperWeapon | 1 | Defense |

> (corrected 2026-07-18: the Meaning column was systematically shifted one class family —
> 0xF/0x10 were labeled Building, 1/0x28 Infantry, 2/3 Unit, 6/7 Aircraft. Re-derived from the
> live tab-switch in `decompile_function 0x6A6300` (0xF/0x10→2; 1/0x28/2/3→3; 6/7→BuildCat
> check; 0x39/0x20/0x1F→1) cross-anchored by same-day WhatAmI verifications:
> BuildingClass::WhatAmI returns 6 via `decompile_function 0x00459EC0`, InfantryClass::WhatAmI
> returns 0xF via `decompile_function 0x00523340`, UnitClass instance check `WhatAmI()==1` via
> `decompile_function 0x7192F0`, Aircraft instance = 2 per the {1,0xF,2} instance grouping in
> `BuildingClass::UpdateGapGenerator_Tick`; and `RTTI_To_TypeArray` (`decompile_function
> 0x0048DCD0`) maps 1/0x28→g_UnitTypeClass_Array, 2/3→g_AircraftTypeClass_Array,
> 6/7→g_BuildingTypeClass_Array, 0xF/0x10→g_InfantryTypeClass_Array. Within each pair, the
> instance code is the binary-anchored half (6, 0xF, 1, 2); the Type-class partner (7, 0x10,
> 0x28, 3) is inferred from case-pairing to the same type array, not independently verified —
> INFERENCE_HARDENED / PARAM1_TYPE_MISREAD)

### BuildCat Check (Buildings to Tab 0 vs Tab 1)
(corrected 2026-07-18: header/body said "Naval Check (Aircraft...)" — RTTI 6/7 are
Building/BuildingType per the table correction above; the Ghidra label `RTTI_Naval_Check` is
itself a stale/misleading name for a plain BuildCat fetch — verified via `decompile_function
0x6A6300` + `0x0048DCD0`)
For RTTIType 6/7 (buildings), `RTTI_Naval_Check` (@ 0x005004E0) reads offset `+0xE08` from a
**BuildingTypeClass\***, not the TechnoTypeClass. (corrected 2026-05-28: was FUN_005004e0 — get_function_by_address 0x005004E0 confirms label RTTI_Naval_Check) (corrected 2026-07-18: was
"TechnoTypeClass+0xE08 SpeedType"; `RTTI_Naval_Check` calls `RTTI_To_TypeArray` (0x0048DCD0),
whose `case 6: case 7:` indexes `g_BuildingTypeClass_Array` — so the pointer dereferenced at
+0xE08 is a BuildingTypeClass\*, per decompile_function 0x005004E0 + decompile_function
0x0048DCD0. That +0xE08 field is populated from the `BuildCat=` INI key in
`BuildingTypeClass::ReadINI` (0x0045FE50) — confirmed by reading the INI-key string bytes at
0x0081AEE4 via read_memory ("BuildCat\0"), not SpeedType. `RTTI_Naval_Check` itself performs no
naval/Float/Amphibious comparison — it is a plain field fetch — PARAM1_TYPE_MISREAD) If the
returned value == 5, the item goes to **Tab 1**; otherwise **Tab 0** — the "SpeedType::Float =
Naval" gloss for value 5 relied on the wrong field and is now UNVERIFIED (BuildCat's own enum
semantics were not re-checked this pass).

The binary expression is `param_2 = (uint)(BuildCat == 5)` (corrected 2026-07-18: field is
BuildCat, not SpeedType, per the offset-owner correction above) — boolean cast to uint yields 1
when BuildCat == 5, 0 otherwise (comparison site verified via `decompile_function 0x006A6300`,
2026-05-19; the field identity was not re-checked this pass).

### Tab Names (from string table references)
- Tab 0: Structures (buildings with BuildCat != 5; was "(context-dependent, often Naval)" —
  corrected 2026-07-18 per the RTTI-table correction above, `decompile_function 0x6A6300`)
- Tab 1: Defense (`TXT_DEFENSE_TAB_DESC`)
- Tab 2: Structure (`TXT_STRUCTURE_TAB_DESC`)
- Tab 3: Unit (`TXT_UNIT_TAB_DESC`) / Infantry (`TXT_INFANTRY_TAB_DESC`)

**Tab Button IDs**: 0xCB (203) through 0xCE (206) = tabs 0-3.

---

## Tab Switching (SidebarClass::SwitchTab @ 0x6A7590)

```c
void SidebarClass::SwitchTab(int newTab) {
    if (newTab != this->CurrentTab) {
        // Deactivate old tab
        Strip[CurrentTab].IsActive = false;
        // Remove all SelectClass gadgets for old tab
        for (i = 0; i < 0x3C; i++) {
            RemoveGadget(&SelectGadgets[Strip[CurrentTab].TabIndex * 0x3C + i]);
        }
        // Set new tab
        this->CurrentTab = newTab;
        // Activate new tab
        Strip[newTab].IsActive = true;
        // Add SelectClass gadgets for new tab
        int visibleCount = GetVisibleSlotCount();  // already rows*2 -- do NOT multiply by 2 again
        // (corrected 2026-07-18: doc previously read "GetVisibleCameoCount() * 2" here,
        // which double-counts -- decompile_function 0x006A7590 shows SwitchTab inlines the
        // exact same formula as GetVisibleSlotCount (0x006AC430), ending in a single `* 2`,
        // and calls no such "GetVisibleCameoCount" function at all — OPERATOR_OR_ORDER_DRIFT,
        // stale pseudocode left over from before the GetVisibleSlotCount rows*2 correction)
        for (i = 0; i < visibleCount; i++) {
            AddGadget(&SelectGadgets[Strip[newTab].TabIndex * 0x3C + i]);
        }
        // Refresh scroll buttons
        UpdateScrollButtons();
        this->ForceFullRedraw = true;
    }
}
```

Tab switching in SidebarClass::Action (0x6A7780) handles IDs 0x80CB-0x80CE:
```
tabIndex = eventID - 0x80CB
```
// corrected 2026-05-28: was "SidebarClass::AI"; Ghidra label is SidebarClass__Action
// — verified via get_function_by_address 0x006A7780 — RTTI_LABEL_DRIFT

---

## Visible Slot Count Calculation (0x6AC430)

Ghidra label: `SidebarClass::GetVisibleSlotCount` — returns the slot
count (rows × 2) directly, not the row count (verified via
`decompile_function 0x006AC430`, 2026-05-19).

```c
int GetVisibleSlotCount() {
    int tabBarHeight = 0x1A;  // 26 pixels normally
    if (hasNewSidebar)        // RA2_NEWSIDEBAR mode
        tabBarHeight = 0x12;  // 18 pixels

    return (((sidebarBottom - sidebarCameoTop) - tabBarHeight - 7 + sidebarWidth) / 0x32) * 2;
    // 0x32 = 50 pixels per cameo row; * 2 because 2 columns
    // corrected 2026-05-28: was "sidebarLeft"; binary uses g_SidebarWidth (DAT_00886f98),
    // not sidebarLeft (DAT_00886f94) — verified via decompile_function 0x006AC430
    // Root cause: RTTI_LABEL_DRIFT (wrong variable name in pseudocode annotation)
}
```

---

## Cameo Size Constants

| Constant | Value | Meaning |
|----------|-------|---------|
| Cameo width | 0x3C (60) | Pixel width of each cameo |
| Cameo height | 0x30 (48) | Pixel height of each cameo |
| Row height (DAT_00b0b500) | 0x32 (50) | Pixel height per row (cameo + 2px gap) |
| Column width (DAT_00b0b4fc) | 0x3F (63) or 0x40 (64) | Depends on sidebar mode |
| Scroll speed (DAT_00b0b514) | 0x32 (50) | Pixels per scroll step |
| Max cameos per strip | 0x4B (75) | Hardcoded limit |
| SelectClass count | 0xF0 (240) | 4 tabs * 60 gadgets each |
| SelectClass per tab | 0x3C (60) | 30 rows * 2 columns max |

---

## Sidebar Layout Constants (FUN_006a5090 + FUN_006a5130)

### Normal sidebar mode (no new sidebar):
| Global | Value | Meaning |
|--------|-------|---------|
| DAT_00b0b4e0 | sidebarLeft + 8 | Tab buttons Y position |
| DAT_00b0b4e4 | 0x40 (64) | Tab button height |
| DAT_00b0b4ec | sidebarLeft + 0x27 | Tab button width area |
| DAT_00b0b4f0 | 0x1D (29) | Tab button spacing |
| DAT_00b0b4f8 | sidebarLeft + 0x45 | Cameo area top Y |
| DAT_00b0b4fc | 0x3F (63) | Column width |
| DAT_00b0b500 | 0x32 (50) | Row height |
| DAT_00b0b504 | visibleRows * 50 | Total cameo area height |
| DAT_00b0b508 | sidebarLeft + 0x27 | Scroll button X |
| DAT_00b0b50c | cameoTop + 7 + cameoAreaHeight | Scroll button Y |
| DAT_00b0b510 | 0x2E (46) | Scroll button width |
| DAT_00b0b514 | 0x32 (50) | Scroll animation speed |

### New sidebar mode:
| Global | Value | Meaning |
|--------|-------|---------|
| DAT_00b0b4e4 | 0x34 (52) | Tab button height (smaller) |
| DAT_00b0b4f0 | 0x20 (32) | Tab button spacing |
| DAT_00b0b4fc | 0x40 (64) | Column width |

### Sidebar Origin:
- `DAT_00886f94` = 0x9E (158) = sidebar column left X (fixed)
- `DAT_00886f90` = computed from screen layout (sidebar pixel X origin)
- `DAT_00886f9c` = screen height minus offsets (sidebar bottom)

---

## Scroll Logic

### Scroll State (per strip):
| Field | Offset | Meaning |
|-------|--------|---------|
| ScrollPosition | +0x44 | Current top row index |
| ScrollRequest | +0x48 | Pending scroll delta (+N or -N) |
| ScrollPixelOffset | +0x4C | Current smooth-scroll pixel offset |
| IsScrolling | +0x3F | Animation in progress |
| ScrollDirection | +0x3E | 0=up, 1=down |

### Scroll Processing (from StripClass::AI @ 0x6A8B30)
Verified via `decompile_function 0x006A8B30`, 2026-05-19. Note: the UP
and DOWN cases are asymmetric — UP pre-decrements ScrollPosition at
request time, then animates 0→rowHeight; DOWN starts the animation at
rowHeight, decrements down to <1, then post-increments ScrollPosition.

```c
// Scroll initiation (runs first; only when !IsScrolling and ScrollRequest != 0):
if (ScrollRequest != 0) {
    int maxVisible = visibleRows * 2;  // or visibleRows for observer
    if (maxVisible < totalCameos) {
        if (ScrollRequest < 0) {  // request to scroll up
            if (ScrollPosition > 0) {
                ScrollRequest++;
                ScrollDirection = 0;     // 0 = up
                IsScrolling = true;
                ScrollPosition--;        // pre-decrement
                ScrollPixelOffset = 0;   // animation starts at 0
            }
        } else {  // request to scroll down
            if ((ScrollPosition + visibleRows) * 2 < totalCameos) {
                ScrollRequest--;
                ScrollPixelOffset = rowHeight;  // animation starts at rowHeight
                ScrollDirection = 1;     // 1 = down
                IsScrolling = true;
            }
        }
    }
}

// Scroll animation (runs after initiation, or in subsequent ticks):
if (IsScrolling) {
    if (ScrollDirection == 0) {  // scrolling up
        ScrollPixelOffset += scrollSpeed;
        if (ScrollPixelOffset >= rowHeight) {
            IsScrolling = false;
            ScrollPixelOffset = 0;
            // NO position change — already pre-decremented at request time
        }
    } else {  // scrolling down
        ScrollPixelOffset -= scrollSpeed;
        if (ScrollPixelOffset < 1) {
            IsScrolling = false;
            ScrollPixelOffset = 0;
            ScrollPosition++;        // post-increment
        }
    }
}
```

### Scroll Buttons:
- **Scroll Down** button: ID 0xC9 (201), global gadget at `DAT_00b0b328`
- **Scroll Up** button: ID 0xC8 (200), global gadget at `DAT_00b0b408`
- Keyboard scroll: `SidebarUp` / `SidebarDown` commands (`TXT_SIDEBAR_UP`, `TXT_SIDEBAR_DOWN`)

---

## SelectClass (Cameo Click Gadgets)

### Global Array
- Base address: `0x00B07E80`
- Each entry: 0x38 (56) bytes
- Total: 0xF0 (240) entries = 4 tabs * 0x3C (60) per tab
- Per-tab offset: `tabIndex * 0x3C` entries

### SelectClass Fields (offset from entry base):

> **2026-05-19 correction.** The position fields below were previously listed
> 4 bytes too high. Verified offsets via `decompile_function 0x006a8220` —
> writes target `(&DAT_00b07e8c)[idx*0xe]` (X = base+0x0C), `DAT_00b07e90`
> (Y = base+0x10), `DAT_00b07e94` (W = base+0x14), `DAT_00b07e98` (H = base+0x18).
> ID/StripPtr/CameoIndex were already correct. See
> `INIT_SELECT_ZONES_GHIDRA_REPORT.md` for full evidence.

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0x00 | 4 | vtable | Points to SelectClass vtable @ 0x7F2FCC |
| 0x04-0x0B | | (inherited GadgetClass — 8 bytes) | |
| 0x0C | 4 | XPos | Left edge of this cameo gadget (verified `006a8220` writes to DAT_00b07e8c) |
| 0x10 | 4 | YPos | Top edge (verified write to DAT_00b07e90) |
| 0x14 | 4 | Width | Always 0x3C (60) — hardcoded literal in InitSelectZones |
| 0x18 | 4 | Height | Always 0x30 (48) — hardcoded literal in InitSelectZones |
| 0x24 | 4 | ID | Always 0xCA (202) = cameo click (verified `006a8220`) |
| 0x2C | 4 | StripPtr | Pointer back to owning StripClass (verified `006a8220`) |
| 0x30 | 4 | CameoIndex | row*2 + col within the visible grid (verified `006a8220`) |
| 0x34 | 1 | IsHighlighted | Mouse-over highlight state — **UNVERIFIED in 2026-05-19 audit** |

### Hit Test / Position Calculation (from SidebarClass::InitSelectZones @ 0x6A8220):
Ghidra's decompiler-assigned function name is `SidebarClass__InitSelectZones`,
but the verified **this-pointer at the call site is a StripClass instance,
not SidebarClass** (corrected 2026-07-18: the 2026-05-19 note above — "belongs
to SidebarClass not StripClass" — trusted the Ghidra function-name label only
and was WRONG; root cause RTTI_LABEL_DRIFT). Evidence: `disassemble_function
0x006A5310` (SidebarClass::Init, the function's sole caller per
`get_function_callers 0x006A8220`) shows the per-strip loop computing
`ESI = this+0x1568 + N*0xF94` (Strip[N]+0x24, i.e. Strip[N].YPos), then at the
call site `LEA ECX,[ESI-0x24]` — i.e. `ECX = Strip[N]` base exactly — with the
tab index N passed as the second (stack) argument via `PUSH EBP` beforehand.
Inside the callee, `*(int*)(param_1+0x38) = param_2` writes that tab index
into `Strip.TabIndex` (+0x38, matching the StripClass layout table above,
cross-confirmed independently in `AddCameo`/`SwitchTab`), and
`(&DAT_00b07eac)[iVar1*0xe] = param_1` stores `param_1` itself as each
gadget's StripPtr — self-consistent only if `param_1` is the owning
StripClass pointer, matching the SelectClass field table's own
"StripPtr: Pointer back to owning StripClass" note below.

```c
for (row = 0; row < visibleRows; row++) {
    for (col = 0; col < 2; col++) {
        int gadgetIndex = Strip.TabIndex * 0x3C + row * 2 + col;
        SelectGadgets[gadgetIndex].ID = 0xCA;
        SelectGadgets[gadgetIndex].XPos = Strip.XPos + columnWidth * col;
        SelectGadgets[gadgetIndex].YPos = Strip.YPos + 1 + rowHeight * row;
        SelectGadgets[gadgetIndex].Width = 0x3C;   // 60
        SelectGadgets[gadgetIndex].Height = 0x30;   // 48
        SelectGadgets[gadgetIndex].CameoIndex = row * 2 + col;
        SelectGadgets[gadgetIndex].StripPtr = &Strip;
    }
}
```

---

## SelectClass::Action (Click Handler @ 0x6AAD00)

This is the main handler when a cameo is clicked. 3207 bytes, very complex.

### Parameters:
- `param_1`: SelectClass `this`
- `param_2`: ActionFlags (bitmask: 0x01=LeftClick, 0x08=cleared
  unconditionally at the top of both the superweapon and normal-item
  branches (`if ((param_2 & 8) != 0) param_2 &= ~8;`) and never read
  again inside this function, 0x10=RightClick) (corrected 2026-07-12:
  was "0x08=?" — verified via `decompile_function 0x006AAD00`)
- `param_3`: KeyFlags
- `param_4`: EventFlags (bit 0 = something related to queuing)

### Flow:
1. Resolves which cameo was clicked: `cameoSlot = this->CameoIndex + Strip->ScrollPosition * 2`
2. Reads the CameoEntry at that slot to get RTTIType, TypeIndex, FactoryPtr
3. **SuperWeapon (RTTIType == 0x1F)**: Activates the super weapon directly
4. **Normal buildable**: Looks up the TechnoTypeClass
5. **Right-click (param_2 & 0x10)**:
   - If factory exists and building: sends **Cancel** network event (0x0F)
   - If factory complete: sends **Place** or **Cancel** event
   - If no factory but queued: removes from queue
6. **Left-click (param_2 & 0x01)**:
   - If factory exists and complete: enters placement mode (buildings) or deploys
   - If building in progress: **does nothing** (already building)
   - If not yet started: sends **Produce** network event (0x0E)
   - Sets CameoEntry.Status = 1 (Building)
   - Calculates initial progress bar speed from cost

### Network Events Sent:
- `0x0E` = RequestProduce (left-click to start building)
- `0x0F` = RequestCancel (right-click to cancel)
- `0x10` = RequestPlace (right-click on completed item)
- `0x12` = SuperWeapon activate
- `0x2E` = RequestProduce (queued variant)

---

## Factory Connection

The sidebar discovers what's being built through the FactoryClass pointer
stored in each CameoEntry at offset +0x0C.

### FactoryClass Key Functions:
- `FactoryClass::GetProgress @ 0x4CA120` — returns `this->Production_Value`
  directly (range 0..0x36 = 0..54), **not** a 0-100% normalised value
  (verified via `decompile_function 0x004CA120`, 2026-05-19).
- `FactoryClass::IsComplete @ 0x4CA130` — Ghidra label is `IsComplete`
  (not `HasCompleted`). Returns true when `Production_Value == 0x36`
  AND the factory has an Object or SpecialItem set (verified via
  `decompile_function 0x004CA130`, 2026-05-19). **Precision added
  2026-07-12:** "SpecialItem set" specifically means `SpecialItem !=
  -1` (not `!= 0`) — re-verified via `decompile_function 0x004CA130`
  this session; a Rust port using 0 as the empty sentinel would be
  wrong.

Note: `FactoryClass.Production_Value` (max 0x36) is distinct from
`CameoEntry.ProgressValue` (+0x14, max 0x34) — the latter is the
sidebar's visual progress bar field, animated independently of the
underlying factory progress.

### How production state maps to display:
1. CameoEntry.FactoryPtr is set when production starts
2. `StripClass::AI` (0x6A8B30) polls each CameoEntry's factory:
   - Calls `FactoryClass::IsComplete()` to check if ready
   - Calls `FactoryClass::GetProgress()` for the progress bar frame
3. The progress bar SHP (`DAT_00b0b484`) is drawn with frame = progress + 1
4. When Status == 2 (OnHold), a half-filled bar is shown: `progress / 2`
5. When complete, "Ready" text is overlaid (string table ID 0xD53)

---

## AddCameo Flow (SidebarClass::AddCameo @ 0x6A6300)

```
SidebarClass::AddCameo(RTTIType, TypeIndex):
    1. Map RTTIType to tab index (0-3)
    2. Check if Strip[tab].CameoCount < 0x4C (76 limit check, actually 75 used)
    3. Check for duplicate: scan existing cameos for same (TypeIndex, RTTIType)
    4. If not duplicate:
       a. Call StripClass__InsertEntry(RTTIType, TypeIndex) @ 0x6A8710 (corrected 2026-05-28: was InsertCameo — RTTI_LABEL_DRIFT)
       b. InsertEntry finds sorted position via SidebarClass__CompareItems @ 0x006A8420 (corrected 2026-05-28: doc had FUN_006a8420 — RTTI_LABEL_DRIFT)
       c. Shifts entries down, inserts at correct position
       d. Initializes new CameoEntry fields
       e. Sets CameoEntry.FlashTimer for "new item" flash effect
    5. Mark strip dirty, update scroll buttons
    6. If new tab has items but old didn't, auto-switch to new tab
```

### Cameo Sort Order (SidebarClass__CompareItems @ 0x006A8420):
Priority order:
1. SuperWeapons always sort first. When BOTH sides being compared are
   superweapons (RTTIType 0x1F/0x39/0x20), the comparator instead uses
   a dedicated path: sorts by `SuperWeaponTypeClass+0xB0` ascending,
   tiebreaking via the same name-compare helper as step 4 — **added
   2026-07-12**, the doc previously didn't mention this two-key
   superweapon-vs-superweapon path (verified via
   `decompile_function 0x006A8420` — root cause INFERENCE_HARDENED,
   this function had not actually been re-decompiled since the doc was
   written)
2. Items matching player's side sort before others (only reached when
   *neither* item is a superweapon) — code compares
   `*(int*)(*(int*)(g_PlayerPtr+0x34)+0xBC)` against
   `TechnoTypeClass+0x6D0` (`piVar11[0x1b4]`) for each side
3. Among same type: sort by `TechnoTypeClass+0x634` (`piVar11[0x18d]`) ascending
4. **Corrected 2026-07-12: doc was missing a sort key.** If the
   `+0x634` field ties, the binary next compares a `vtable+0x84` call
   result (`(*type->vtable[0x84])(g_PlayerPtr)`, ascending) — the old
   text jumped straight from `+0x634` to the name compare. Only if that
   call also ties does it fall through to the alphabetical name compare
   (`TechnoTypeClass+0x60`, via `FUN_007ca5d3`, result `<= 0` sorts A
   first). (verified via `decompile_function 0x006A8420`, 2026-07-12 —
   root cause INFERENCE_HARDENED)

---

## Recalculate / PurgeInvalid

### StripClass::Recalculate (0x006AA600):
Ghidra label is `StripClass__Recalculate`, not `RemoveCameo` (verified
via `get_function_by_address 0x006AA600`, 2026-05-19). **Re-derived
2026-07-12** via `decompile_function 0x006AA600` — the old hypothesis
below is confirmed at the structural level, with corrected mechanism
detail (root cause of the gap: INFERENCE_HARDENED — the doc had been
flagged NEEDS REDERIVATION but never actually re-decompiled):

- Early-out: returns 0 immediately if `g_IsMapEditor` is set, or if
  `Strip.CameoCount` (+0x54) is 0.
- Snapshots the currently-visible window (`ScrollPosition*2` ..
  `+visibleSlots`, using the same row-count formula as
  `GetVisibleSlotCount`) into a temp buffer before making any changes,
  to restore scroll position afterward.
- For each cameo (0..CameoCount): buildability is **not** a direct
  `TechnoType->CanBuild(player)` call as previously hypothesised — it
  resolves the type via an RTTI-to-type-array lookup, then calls a
  `vtable+0x94` method on the type-array entry (args `1,0,0,
  g_PlayerPtr`) and `HouseClass__CanBuild` on the result; failure of
  either (or a failed type-array lookup) triggers removal.
- Removal sends a network Cancel-style command (opcode `0x10` via
  `FUN_004c6970`) when the entry had a queued factory, and — if the
  removed item's RTTIType is 6 or 7 (Aircraft) — resets placement/UI
  mode-lock state (`DAT_0088098c`, `g_UIModeLock`, `DAT_00880994`).
- Shifts remaining entries down via a memmove-style call
  (`FUN_007ca090`) and clears the last slot's TypeIndex (+0x00),
  RTTIType (+0x04), AltTypeIndex (+0x08), FactoryPtr (+0x0C),
  ProgressValue (+0x14), AnimSpeed (+0x24), FlashEndFrame (+0x28), and
  FlashTimer (+0x30); AnimStartTime (+0x1C) is reset to the current
  frame counter rather than zeroed. Decrements `CameoCount` and sets
  `NeedsRedraw` (+0x3C).
- After the removal pass, restores `ScrollPosition` by locating where
  the pre-removal visible-window snapshot entries landed in the
  post-removal array — this confirms the doc's original "adjusts
  scroll position to keep visible items stable" hypothesis.
- Also contains tab-auto-switch logic (activates the first non-empty
  strip if the current tab's count dropped to 0), sharing the same
  `DAT_00880d80` counter array used elsewhere in the sidebar.

### SidebarClass::PurgeInvalid (FUN_006a7d20):
**Confirmed 2026-07-12** via `decompile_function 0x006a7d20` (Ghidra
still has no assigned label — `get_function_by_address 0x006A7D20`
shows `FUN_006a7d20`, so "PurgeInvalid" remains a doc-inferred name).
Loops over all 4 strips (`param_1 + 0x560 + i*0xF94`, i.e. each
`Strip+0x3C`/NeedsRedraw byte, stride confirms StripClass is 0xF94
bytes) calling `StripClass::Recalculate` on each (fastcall `this` per
strip — not visible as an explicit argument in the decompile). If any
strip removed a cameo AND that strip was the current tab
(`i == *(int*)(param_1+0x539C)`, confirming `CurrentTab`@+0x539C),
marks that strip's NeedsRedraw (+0x3C). If any removal happened at
all, additionally sets `SidebarClass.NeedsRedraw` (+0x53A6) and calls
a `vtable+0x38` method with arg 0 (a redraw/refresh trigger — not
independently identified this session).

---

## Drawing Pipeline

### SidebarClass::Draw (0x6A6C30):
1. Draws sidebar background SHPs (top, middle tiles, bottom)
2. Draws tab flash buttons (3 SHPs at `DAT_00b0b478/7C/80`)
3. Calls `StripClass::Draw` for the active strip
4. Blits the sidebar surface to screen

### StripClass::Draw (0x6A9540):
For each visible cameo slot (row * 2 + col):
1. Resolve the CameoEntry: `slot = col + (row + ScrollPosition) * 2`
2. Load the cameo image SHP from `TechnoTypeClass->CameoSHP` (offset +0xB8)
3. Draw cameo SHP at calculated position
4. If mouse-over highlight: draw color tint overlay
5. If can't afford: draw darkened overlay (SHP at `DAT_00b07bc0`)
6. If flashing ("new item"): alternate frame visibility (8-frame cycle)
7. Draw name text (right-aligned, alpha-blended background)
8. If building: draw progress bar SHP (frame = progress + 1)
9. If complete and waiting to place: draw "Ready" or "On Hold" text
10. If queued (multiple of same): draw queue count text

### Cameo Position Calculation:
```c
int x = Strip.XPos + columnWidth * col - sidebarXOffset;
int y = Strip.YPos + 1 + rowHeight * row;
if (Strip.IsScrolling) {
    y += (Strip.ScrollPixelOffset - rowHeight);
}
```

---

## Observer/Spectator Mode

When `DAT_00a83d4c == DAT_00ac1198` (local player is observer), the sidebar
switches to a different mode:
- Shows all players' units in a list view (not cameo grid)
- Uses `DAT_00884cf8` for player count
- Each row shows player stats (income, units, etc.)
- Uses `DAT_00884b94[N]` to access player HouseClass pointers
- Side icons loaded from `DAT_00b0b490-0xb0b4c8` (12 faction SHPs)

---

## Key Global Addresses

| Address | Type | Name |
|---------|------|------|
| 0x00B07BC0 | SHP* | DarkenCameoOverlay (can't build) |
| 0x00B07C48 | GadgetClass[4] | TabButtons (stride 0x60, IDs 0xCB-0xCE) |
| 0x00B07DC8 | | End of TabButtons array |
| 0x00B07E48 | GadgetClass* | SellButton backptr |
| 0x00B07E58 | GadgetClass | PowerBar gadget |
| 0x00B07E80 | SelectClass[240] | CameoClickGadgets (stride 0x38) |
| 0x00B0B328 | GadgetClass | ScrollDownButton (ID 0xC9) |
| 0x00B0B3A0 | GadgetClass | RepairButton |
| 0x00B0B408 | GadgetClass | ScrollUpButton (ID 0xC8) |
| 0x00B0B468 | SHP* | SidebarTopSHP |
| 0x00B0B46C | SHP* | SidebarMiddleSHP |
| 0x00B0B470 | SHP* | SidebarBottomSHP |
| 0x00B0B474 | SHP* | SidebarBottom2SHP |
| 0x00B0B478 | SHP* | TabFlash1SHP |
| 0x00B0B47C | SHP* | TabFlash2SHP |
| 0x00B0B480 | SHP* | TabFlash3SHP |
| 0x00B0B484 | SHP* | ProgressBarSHP |
| 0x00B0B490-4C8 | SHP*[15] | FactionSideIcons (observer mode) |
| 0x00B0B4DC | int | TabButtonsX |
| 0x00B0B4E0 | int | TabButtonsY |
| 0x00B0B4E4 | int | TabButtonHeight (0x40 or 0x34) |
| 0x00B0B4E8 | int | TabButtonStartX |
| 0x00B0B4EC | int | CameoAreaWidth |
| 0x00B0B4F0 | int | TabButtonSpacing (0x1D or 0x20) |
| 0x00B0B4F4 | int | CameoColumnXStart |
| 0x00B0B4F8 | int | CameoAreaTopY |
| 0x00B0B4FC | int | ColumnWidth (0x3F or 0x40) |
| 0x00B0B500 | int | RowHeight (0x32 = 50) |
| 0x00B0B504 | int | CameoAreaPixelHeight |
| 0x00B0B508 | int | ScrollButtonX |
| 0x00B0B50C | int | ScrollButtonY |
| 0x00B0B510 | int | ScrollButtonWidth (0x2E or 0x2D) |
| 0x00B0B514 | int | ScrollAnimSpeed (0x32 = 50) |
| 0x00B0B518 | byte | GlobalRedrawFlag |
| 0x00B0B519 | byte | SurfaceBlitNeeded |
| 0x00884B84 | int | ActiveTabIndex (observer mode) |
| 0x00884B8E | byte | StripDirtyFlag |
| 0x00884B8F | byte | GadgetDirtyFlag |
| 0x00884CF8 | int | ObserverPlayerCount |
| 0x00886F90 | int | SidebarPixelX |
| 0x00886F94 | int | SidebarColumnLeft (always 0x9E = 158) |
| 0x00886F98 | int | SidebarWidth |
| 0x00886F9C | int | SidebarBottom |

---

## Function Address Table

| Address | Name | Description |
|---------|------|-------------|
| 0x006A4DC0 | SelectClass__StripClass__SidebarClass__Constructor | Constructor for cameo click gadgets (corrected 2026-05-28: was SelectClass::SelectClass — get_function_by_address 0x006A4DC0 — RTTI_LABEL_DRIFT) |
| 0x006A4E60 | SidebarClass__Constructor | Constructor (corrected 2026-05-28: was SidebarClass::SidebarClass — get_function_by_address 0x006A4E60 — RTTI_LABEL_DRIFT) |
| 0x006A5090 | SidebarClass__InitLayoutConstants | Computes tab button positions (corrected 2026-05-28: was CalcTabLayout — get_function_by_address 0x006A5090 — RTTI_LABEL_DRIFT) |
| 0x006A5130 | SidebarClass__InitSidebarRect | Computes all sidebar layout constants (corrected 2026-05-28: was CalcLayout — get_function_by_address 0x006A5130 — RTTI_LABEL_DRIFT) |
| 0x006A5310 | SidebarClass::Init | Full initialization (strips, gadgets, tabs) |
| 0x006A5840 | SidebarClass__LoadSHPs | Loads sidebar SHPs and palettes (corrected 2026-05-28: was LoadArt — get_function_by_address 0x006A5840 — RTTI_LABEL_DRIFT) |
| 0x006A5BF0 | SidebarClass__FreeSHPs | Frees all loaded sidebar art (corrected 2026-05-28: was FreeArt — get_function_by_address 0x006A5BF0 — RTTI_LABEL_DRIFT) |
| 0x006A5F20 | SelectionClass::RemoveFromSelection | NOT FlashStrip — Ghidra label is `SelectionClass__RemoveFromSelection` (verified via `get_function_by_address`, 2026-05-19). Likely unrelated to sidebar; was misattributed in this doc. |
| 0x006A6300 | SidebarClass::AddCameo | Adds item to correct strip |
| 0x006A6610 | SidebarClass::UpdateScrollButtons | Enable/disable scroll arrows |
| 0x006A6A00 | FUN_006a6a00 | Scroll one row up/down (Ghidra label unconfirmed — ScrollByRow is doc-inferred) |
| 0x006A6AF0 | FUN_006a6af0 | Scroll one page up/down (Ghidra label unconfirmed — ScrollByPage is doc-inferred) |
| 0x006A6C30 | SidebarClass::Draw | Main sidebar draw |
| 0x006A70E0 | SidebarClass__BlitToScreen | Blits sidebar to screen surface (corrected 2026-05-28: was Blit — get_function_by_address 0x006A70E0 — RTTI_LABEL_DRIFT) |
| 0x006A7590 | SidebarClass__SwitchTab | Switches active tab |
| 0x006A7780 | SidebarClass__Action | Main update/input loop (corrected 2026-05-28: was SidebarClass::AI — get_function_by_address 0x006A7780 — RTTI_LABEL_DRIFT) |
| 0x006A7D20 | FUN_006a7d20 | Remove unbuildable cameos (Ghidra label unconfirmed — PurgeInvalid is a doc-inferred name only) |
| 0x006A7D70 | SidebarClass__ToggleSidebar | Show/hide sidebar (corrected 2026-05-28: was Activate — get_function_by_address 0x006A7D70 — RTTI_LABEL_DRIFT) |
| 0x006A80A0 | FUN_006a80a0 | Constructor (inits 75 cameo slots — behavior confirmed; Ghidra label unconfirmed, StripClass::StripClass is doc-inferred) |
| 0x006A8220 | SidebarClass::InitSelectZones | Sets up clickable cameo areas. Ghidra's function-name label says SidebarClass, but the verified call-site this-pointer is a StripClass instance (corrected 2026-07-18: was "belongs to SidebarClass not StripClass" — verified via `disassemble_function 0x006A5310` — RTTI_LABEL_DRIFT; see Hit Test section above). |
| 0x006A8330 | FUN_006a8330 | Adds SelectClass gadgets to UI (Ghidra label unconfirmed — ActivateGadgets is doc-inferred) |
| 0x006A83E0 | FUN_006a83e0 | Removes SelectClass gadgets (Ghidra label unconfirmed — DeactivateGadgets is doc-inferred) |
| 0x006A8420 | SidebarClass__CompareItems | Comparison for sorted insert (corrected 2026-05-28: was StripClass::CameoSortCompare — get_function_by_address 0x006A8420 — RTTI_LABEL_DRIFT; note owner is SidebarClass not StripClass) |
| 0x006A8710 | StripClass__InsertEntry | Insert cameo at sorted position (corrected 2026-05-28: was InsertCameo — get_function_by_address 0x006A8710 — RTTI_LABEL_DRIFT) |
| 0x006A87F0 | FUN_006a87f0 | Add cameo (checks duplicates — Ghidra label unconfirmed; StripClass::AddCameo is doc-inferred) |
| 0x006A8B30 | StripClass::AI | Strip update (scroll, anim, auto-build) |
| 0x006A92E0 | SidebarClass__GetCameoTooltip | Tooltip for hovered cameo (corrected 2026-05-28: was StripClass::GetTooltipText; Ghidra confirms SidebarClass owner — get_function_by_address 0x006A92E0 — RTTI_LABEL_DRIFT) |
| 0x006A93F0 | StripClass__ActivateButtons | Enable strip + add gadgets (corrected 2026-05-28: was StripClass::Activate — get_function_by_address 0x006A93F0 — RTTI_LABEL_DRIFT) |
| 0x006A94B0 | FUN_006a94b0 | Disable strip + remove gadgets (Ghidra label unconfirmed — Deactivate is doc-inferred) |
| 0x006A9540 | StripClass::Draw | Draw all visible cameos |
| 0x006AA600 | StripClass::Recalculate | Ghidra label is `StripClass__Recalculate` not `RemoveCameo` (verified via `get_function_by_address`, 2026-05-19). Behaviour re-derived 2026-07-12 via `decompile_function 0x006AA600` — see the Recalculate/PurgeInvalid section above. |
| 0x006AAD00 | SelectClass::Action | Click handler (production) |
| 0x006AB990 | FUN_006ab990 | Mouse-enter highlight (Ghidra label unconfirmed — HighlightOn is doc-inferred) |
| 0x006AB9E0 | FUN_006ab9e0 | Mouse-leave unhighlight (Ghidra label unconfirmed — HighlightOff is doc-inferred) |
| 0x006ABD30 | SidebarClass__InitSurface | Recalculate gadget positions (corrected 2026-05-28: was SidebarClass::Recalc — get_function_by_address 0x006ABD30 — RTTI_LABEL_DRIFT) |
| 0x006AC210 | SidebarClass__GetTooltipText | Resolve tooltip by gadget ID (corrected 2026-05-28: was GetTooltipForID — get_function_by_address 0x006AC210 — RTTI_LABEL_DRIFT) |
| 0x006AC430 | SidebarClass::GetVisibleSlotCount | Returns slot count (rows × 2), NOT row count (verified via `decompile_function`, 2026-05-19). |
| 0x006AC480 | SidebarClass::DrawCameoText | Draw text on cameo area |
| 0x004CA120 | FactoryClass::GetProgress | Returns `this->Production_Value` (range 0..0x36 = 0..54), NOT a percent (verified via `decompile_function`, 2026-05-19). |
| 0x004CA130 | FactoryClass::IsComplete | Ghidra label `IsComplete` not `HasCompleted`; returns true when `Production_Value == 0x36` and Object/SpecialItem is set (verified via `decompile_function`, 2026-05-19). |

---

## Unverified (YELLOW)

Items the 2026-05-19 audit pass flagged but did not re-verify against
the binary. These claims may be correct, partially correct, or wrong —
treat as not load-bearing until a follow-up audit confirms.

- **Tab-flash draw conditional — NOW CONFIRMED.** The tab-flash SHP
  draws at `DAT_00b0b478/7C/80` are gated on `*(int *)(this+0x5398) != 0`
  (i.e. `TabFlashState != 0`), not unconditional. Confirmed via
  `decompile_function 0x006A6C30` 2026-05-28: binary shows
  `if (*(int *)((int)this + 0x5398) != 0) { CC_Draw_Shape(DAT_00b0b478,...); CC_Draw_Shape(DAT_00b0b47c,...); CC_Draw_Shape(DAT_00b0b480,...); }`.
  The Drawing Pipeline section description ("Draws tab flash buttons
  (3 SHPs at DAT_00b0b478/7C/80)") should be read as conditional-only.
- ~~`StripClass::Recalculate (0x006AA600)` behaviour~~ — **RESOLVED
  2026-07-12**, see the Recalculate/PurgeInvalid section above
  (re-derived via `decompile_function 0x006AA600`).
- ~~`SidebarClass::PurgeInvalid (0x006A7D20)` body~~ — **RESOLVED
  2026-07-12**, see the Recalculate/PurgeInvalid section above
  (re-derived via `decompile_function 0x006a7d20`). Ghidra still has no
  assigned label for this function (re-checked via
  `get_function_by_address 0x006A7D20` — still `FUN_006a7d20`).

