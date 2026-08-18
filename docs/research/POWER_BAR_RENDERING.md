# Power Bar UI Rendering (PowerClass) — Binary Research Report

> **Companion doc.** The authoritative power system reference is
> [POWER_SYSTEM_GHIDRA_REPORT.md](POWER_SYSTEM_GHIDRA_REPORT.md).
> This doc covers the rendering side in depth. If any detail here
> conflicts with the main report, the main report is correct.

Source file: `D:\ra2mdpost\Power.CPP` (confirmed by string reference at 0x00836db8)

Binary: `gamemd.exe` (Yuri's Revenge)

## Overview

The power bar is rendered as a vertical strip of 3-pixel-tall SHP segments on the left
edge of the sidebar. It uses `POWERP.SHP` (5 frames, 0–4) drawn with the sidebar palette.
The bar animates when power values change, using a two-phase system: a 10-flash blink
followed by smooth per-tick segment interpolation.

---

## Key Addresses and Globals

| Address | Type | Description |
|---------|------|-------------|
| `DAT_00ac4e74` | SHP* | Pointer to loaded POWERP.SHP |
| `DAT_00b0b504` | int | Power bar total height in pixels (set by SidebarClass::InitSidebarRect) |
| `DAT_00886f94` | int | Sidebar rect top Y (set to 0x9E = 158 during init) |
| `DAT_00884b8d` | byte | Power bar enabled flag (if 0, all power bar code is skipped) |
| `DAT_00884b8e` | byte | Sidebar needs-redraw flag |
| `DAT_00a83d4c` | HouseClass* | Player's HouseClass instance |
| `DAT_00a8b230` | int* | Scenario/player data (offset +0x34b8 = player country index) |
| `DAT_0087f6cc` | ConvertClass* | Sidebar palette (ConvertClass for SHP drawing) |
| `DAT_00887300` | Surface* | Primary draw surface |
| `DAT_00b0b518` | byte | Set to 1 each time Draw_It runs |

### HouseClass Power Fields

| Offset | Type | Description |
|--------|------|-------------|
| +0x24 | IHouse* | IHouse COM interface vtable (secondary vtable from multiple inheritance) |
| +0x53a4 | int | Power output (total production, accumulated from BuildingClass::GetPowerOutput) |
| +0x53a8 | int | Power drain (total consumption, accumulated from BuildingClass::GetPowerDrain) |
| +0x6c | int* | Array of building pointers |
| +0x78 | int | Building count |
| +0x2c0 | byte | Power blackout flag (set by spy infiltrating power plant) |

### PowerClass Instance Fields (offsets from `this`)

| Offset | Type | Description |
|--------|------|-------------|
| +0x150c | byte | Dirty/needs-draw flag |
| +0x1510 | int | Flash timer start tick (from GetTickCount) |
| +0x1514 | int | Flash timer start tick (high dword, 64-bit timestamp) |
| +0x1518 | int | Flash timer interval (3 ticks per flash) |
| +0x151c | int | Flash counter (counts down from 10 to 0) |
| +0x1520 | int | Stabilize timer start tick |
| +0x1524 | int | Stabilize timer start tick (high dword) |
| +0x1528 | int | Stabilize timer interval |
| +0x152c | int | Current displayed surplus segments (drawn with frame 1, green) |
| +0x1530 | int | Current displayed output segments (drawn with frame 2, yellow) |
| +0x1534 | int | Current displayed drain segments (drawn with frame 3, red; receives rounding residual) |
| +0x1538 | byte | "Still animating" flag |
| +0x153c | int | Cached IHouse::Power_Drain() value (for change detection) |
| +0x1540 | int | Cached IHouse::Power_Output() value (for change detection) |

---

## POWERP.SHP Frame Layout

POWERP.SHP contains 5 frames (indices 0–4), each 3 pixels tall:

| Frame | Meaning | Visual |
|-------|---------|--------|
| 0 | Empty/background | Dark/unlit segment |
| 1 | Surplus | Green segment (excess power beyond drain) |
| 2 | Output | Yellow segment (power production matching drain) |
| 3 | Drain | Red segment (power consumption) |
| 4 | Transition blink | Blinking frame at the empty/filled boundary during power changes |

The palette used is the sidebar ConvertClass at `DAT_0087f6cc`, created from the sidebar
palette merged with the theater palette during `SidebarClass::LoadSHPs`.

---

## Function Analysis

### FUN_0063f7c0 — PowerClass::Init_IO (Loads POWERP.SHP)

```
Address: 0x0063f7c0
Size: ~16 bytes
```

Calls parent `FUN_00652e90` (RadarClass::Init_IO), then loads "POWERP.SHP" via
`FUN_004a38d0` (LoadSHP) and stores the result in global `DAT_00ac4e74`. This is called
from `SidebarClass::LoadSHPs` (0x006a5840).

### FUN_0063f730 — PowerClass::Constructor

```
Address: 0x0063f730
Size: ~144 bytes
```

Initializes all PowerClass fields:
- Sets cached dimension sentinels to -1 (+0x153C, +0x1540)
- Initializes both timer pairs via GetTickCount (+0x1510, +0x1520)
- Zeroes flash counter (+0x151C), timer intervals (+0x1518, +0x1528)
- Zeroes all segment counts (+0x152C, +0x1530, +0x1534)
- Clears "animating" flag (+0x1538)

---

### FUN_0063f850 — PowerClass::Calc_Segments (Total Filled Segments)

```
Address: 0x0063f850
Size: ~148 bytes
```

**Purpose:** Calculates how many of the total power bar segments should be filled,
based on the sum of all buildings' THEORETICAL power (from BuildingTypeClass, not
current operational values). This determines the overall bar fill level. Uses an
inverse-proportional scale so the bar grows quickly at first but asymptotically
approaches full for very high power values.

**Important:** This function sums raw BuildingTypeClass power values (+0xEE0 output,
+0xEE4 drain) for ALL buildings in the player's house array, regardless of whether
each building is operational, damaged, or powered down. This means the bar fill level
represents "total power infrastructure" rather than current actual power. The actual
drain/output split is handled separately by `Calc_Power_Distribution`.

**Algorithm (from assembly):**

```
total_segments = (bar_height_px + 3) / 3
// where bar_height_px = DAT_00b0b504

// Sum THEORETICAL power across all buildings in player's house
// This uses BuildingTypeClass values, NOT operational/current values
total_power = 0
for each building in HouseClass.buildings[0..building_count]:
    type = building->BuildingTypeClass  // building offset +0x520
    total_power += type->PowerOutput    // type offset +0xEE0
    total_power += type->PowerDrain     // type offset +0xEE4

// Inverse-proportional scale: empty_ratio = 400 / (total + 400)
// As total_power grows, empty_ratio approaches 0 (bar fills up)
empty_ratio = 400.0 / (total_power + 400.0)
empty_segments = ftol(total_segments * empty_ratio)

// Clamp to [0, total_segments - 1]
empty_segments = clamp(empty_segments, 0, total_segments - 1)

return total_segments - empty_segments  // = filled segments
```

**Key constant:** 400.0 (double at address 0x007ED8C8) is the scale factor that controls
how fast the bar fills. When total power equals 400, the bar is exactly half full
(empty_ratio = 0.5). The bar always has at least 1 filled segment: with 0 power,
`empty = ftol(total_segments * 1.0) = total_segments`, clamped to `total_segments - 1`,
so the return value is 1.

**Scaling behavior examples** (assuming total_segments = 50):

| Total Power | empty_ratio | Filled Segments |
|-------------|-------------|-----------------|
| 0 | 1.000 | 1 (minimum) |
| 100 | 0.800 | 10 |
| 400 | 0.500 | 25 |
| 1200 | 0.250 | 38 |
| 3600 | 0.100 | 45 |
| 10000 | 0.038 | 48 |

---

### FUN_0063f960 — PowerClass::Calc_Power_Distribution (Drain/Output/Surplus Split)

```
Address: 0x0063f960
Size: ~444 bytes (returns via RET 0xC — stdcall with 3 pointer params)
```

**Purpose:** Splits the filled segment count from `Calc_Segments` into three portions:
drain segments, output segments, and surplus segments.

**Parameters (VERIFIED from AI tick assembly at 0x640064-0x640241):**
- param_1 `[EBP+0x8]` (out): **surplus** segment count
- param_2 `[EBP+0xC]` (out): **output** segment count
- param_3 `[EBP+0x10]` (out): **drain** segment count (also receives rounding residual)
- Returns: `total_segments` = `(bar_height_px + 3) / 3`

**HouseClass+0x24 sub-object — RESOLVED:**

The object at HouseClass+0x24 is the **IHouse COM interface** vtable pointer, part
of HouseClass's multiple inheritance chain. HouseClass is declared as:

```cpp
class HouseClass : public AbstractClass, public IHouse, public IPublicHouse, public IConnectionPointContainer
```

AbstractClass occupies 0x24 bytes (vtable ptr + IRTTITypeInfo/INoticeSink/INoticeSource
vtable ptrs + UniqueID + AbstractFlags + unknown_18 + RefCount + Dirty + padding).
The IHouse vtable therefore sits at exactly offset +0x24 in every HouseClass instance.

**IHouse vtable layout** (from Interfaces.h, verified against assembly):

| Index | Offset | Method |
|-------|--------|--------|
| 0 | +0x00 | QueryInterface |
| 1 | +0x04 | AddRef |
| 2 | +0x08 | Release |
| 3 | +0x0C | ID_Number |
| 4 | +0x10 | Name |
| 5 | +0x14 | Get_Application |
| 6 | +0x18 | Available_Money |
| 7 | +0x1C | Available_Storage |
| 8 | +0x20 | **Power_Output** |
| 9 | +0x24 | **Power_Drain** |
| 10 | +0x28 | Category_Quantity |
| 11 | +0x2C | Category_Power |

The assembly calls vtable[9] first (Power_Drain → EBX), then vtable[8] (Power_Output
→ EAX), then computes `surplus = Power_Output - Power_Drain`.

**Algorithm (from assembly at 0x0063f960-0x0063fb1b):**

```
total_segments = (DAT_00b0b504 + 3) / 3
filled = Calc_Segments()  // call FUN_0063f850

drain  = IHouse::Power_Drain()   // vtable[9] at +0x24
output = IHouse::Power_Output()  // vtable[8] at +0x20
surplus = output - drain

// Partition surplus into output_portion and surplus_portion
// output_portion is clamped to [0, 100], surplus_portion is the excess
if surplus < 0:
    output_portion = 0.0
    surplus_portion = 0.0
elif surplus < 100:
    output_portion = (double)surplus  // all net power goes to output band
    surplus_portion = 0.0
else:
    output_portion = 100.0            // capped at 100
    surplus_portion = (double)surplus - 100.0  // excess becomes surplus band

// Compute proportional ratios against total visual range
total = (double)drain + output_portion + surplus_portion
if total > 0.0:
    // Re-reads drain from IHouse::Power_Drain()
    drain_frac    = (double)drain / total
    surplus_frac  = surplus_portion / total
    output_frac   = output_portion / total
else:
    // Default: all drain (bar shows fully red when total == 0)
    drain_frac    = 1.0
    surplus_frac  = 0.0
    output_frac   = 0.0

// Convert fractions to segment counts (FPU order: drain first, output, surplus)
drain_segs   = ftol(filled * drain_frac)    // first ftol → stored to *param_3
output_segs  = ftol(filled * output_frac)   // second ftol → stored to *param_2
surplus_segs = ftol(filled * surplus_frac)  // third ftol → stored to *param_1

// Rounding correction: collect fractional remainders from all three
// ftol() truncations, add 0.01 epsilon, and reassign to *param_3 (drain)
// This ensures drain + output + surplus == filled after integer rounding
error = (filled*drain_frac - drain_segs) +
        (filled*output_frac - output_segs) +
        (filled*surplus_frac - surplus_segs) +
        0.01
*param_3 = ftol(error)  // drain gets the rounding residual
```

**Param-to-offset mapping (verified from AI tick at 0x640064-0x640247):**

| Calc param | Stack local | PowerClass offset | Meaning |
|------------|------------|-------------------|---------|
| param_3 | [ESP+0x10] | +0x1534 | drain segments |
| param_1 | [ESP+0x1C] | +0x152C | surplus segments |
| param_2 | [ESP+0x20] | +0x1530 | output segments |

**Constants:**
- 100.0 (double at 0x007E2AC0): cap on the output_portion before surplus begins
- 0.0 (double at 0x007E2800): comparison baseline
- 0.01 (double at 0x007E3808): rounding epsilon to prevent off-by-one

**Confidence: HIGH.** All formerly uncertain areas resolved:
- HouseClass+0x24 identity confirmed via YRpp headers and RTTI strings
- Param-to-offset mapping traced through exact assembly push order and comparison sites
- IHouse::Power_Output/Power_Drain confirmed by HouseClass::AI_AssessPower at 0x508c30

---

### FUN_0063fea0 — PowerClass::AI (Per-Frame Animation Tick)

```
Address: 0x0063fea0
Size: 1279 bytes
```

**Purpose:** Manages the animated power bar transitions each game tick. Uses a two-phase
timer system: first a visual "flash" effect, then smooth segment interpolation.

**Guard:** Exits immediately if `DAT_00884b8d == 0` (power bar disabled).

**Phase 1: Flash Timer** (offsets +0x1510/+0x1518/+0x151C)

When power changes are detected:
1. Flash counter is set to **10** (at +0x151C)
2. Flash interval is set to **3 ticks** (at +0x1518)
3. Each timer expiry:
   - Decrements flash counter by 1
   - Sets dirty flag (+0x150C = 1)
   - Sets sidebar redraw flag (`DAT_00884b8e = 1`)
   - Calls `FUN_004f42f0(0)` (InvalidateRect)
   - Resets timer to 3 ticks

The flash counter at +0x151C is used by Draw_It to alternate between frame 1 and
frame 4 for the drain segment blink effect (see Draw_It below).

**Change Detection:**

After flash processing, the function queries power values via the IHouse COM
interface at `HouseClass + 0x24`:
- `power_drain  = IHouse::Power_Drain()`   (vtable[9], offset +0x24)
- `power_output = IHouse::Power_Output()`  (vtable[8], offset +0x20)

These return HouseClass+0x53a8 (drain) and HouseClass+0x53a4 (output) respectively,
the same values accumulated by HouseClass::AI_AssessPower. They are compared against
cached values at +0x153C (drain) and +0x1540 (output). If EITHER changed since last tick:
- Sets dirty flag (+0x150C) and sidebar redraw flag (`DAT_00884b8e`)
- Calls `FUN_004f42f0(0)` (InvalidateRect)
- Resets flash counter to 10, interval to 3 (triggering a new blink sequence)
- Updates cached values to the new power state

**Phase 2: Stabilize Timer** (offsets +0x1520/+0x1528)

After the flash phase completes, the stabilize timer runs. When it expires:
1. Calls `Calc_Power_Distribution()` to get target segment counts
2. Compares current displayed segments (+0x152C, +0x1530, +0x1534) with targets
3. Applies **incremental +/-1 adjustments** with priority ordering

**Segment Interpolation Algorithm (verified from decompilation):**

The function calls `Calc_Power_Distribution(&surplus, &output, &drain)` to get the
target segment counts. It then compares current displayed values against targets in
this priority order:

**Check order (verified from assembly at 0x640064-0x640241): drain first (+0x1534),
then surplus (+0x152C), then output (+0x1530).**

```
if current_drain != target_drain:             // 0x640064: CMP [ESI+0x1534],[ESP+0x10]
    // Adjust drain by +/-1, then compensate
    if target < current:
        drain -= 1
        recalculate targets
        compensate: increment one of {drain, surplus, output} (first below target)
    else:
        drain += 1
        recalculate targets
        compensate: decrement one of {surplus, drain, output} (first above target)

elif current_surplus != target_surplus:       // 0x640156: CMP [ESI+0x152C],[ESP+0x1C]
    // Same pattern: adjust surplus by +/-1, then compensate
    ...

elif current_output != target_output:         // 0x640241: CMP [ESI+0x1530],[ESP+0x20]
    // Same pattern: adjust output by +/-1, then compensate
    ...
```

Each adjustment:
1. Changes ONE segment count by exactly +/-1
2. Calls `Calc_Power_Distribution` again to get updated targets
3. Applies a compensating +/-1 to another segment to keep the total constant
4. Compensation priority when **incrementing**: drain > surplus > output
5. Compensation priority when **decrementing**: surplus > drain > output

This means **at most 2 segment changes per tick** (one primary + one compensating).
The `still_animating` flag (+0x1538) is set to 1 whenever any segment differs from
its target, causing the function to continue adjusting on subsequent ticks.

**Clamping:** After all adjustments, the total (drain + output + surplus) is clamped
to `Calc_Segments()` maximum. The stabilize timer is reset with a new interval derived
from `GetTickCount` and `ftol`.

**Net effect:** When power changes, the colored segments "slide" smoothly: drain
(red) adjusts first, then surplus (green), then output (yellow). A building
being destroyed causes the bar to progressively shift colors over multiple ticks
rather than jumping instantly.

---

### FUN_0063fb20 — PowerClass::Draw_It (Render Power Bar)

```
Address: 0x0063fb20
Size: 664 bytes
```

**Purpose:** Renders the power bar as a vertical column of SHP segments.

**Guard:** Only draws if:
- `(force_redraw OR dirty_flag)` AND `DAT_00884b8d != 0`

**Allied vs Soviet X offset:**

```
player_index = *(DAT_00a8b230 + 0x34b8)
if player_index == 0:  // Allied
    x_offset = 5
else:                   // Soviet (and Yuri)
    x_offset = 0
```

This shifts the bar 5 pixels right for Allied sidebar to account for the different
sidebar chrome widths.

**Y positioning:**

```
y_start = DAT_00886f94 + 0x45   // sidebar_top + 69 pixels
// Each segment: y += 3
```

The `0x45` (69 decimal) offset accounts for the radar/minimap area above the power bar.

**Drawing order (top to bottom):**

The bar is drawn from top to bottom in screen coordinates (Y increasing downward).
Empty segments are at the top; filled segments progress downward:

```
total_segments = (DAT_00b0b504 + 3) / 3

1. EMPTY segments (frame 0):                              [TOP of bar]
   count = total_segments - surplus - output - drain
   Drawn first

2. TRANSITION BLINK check (frame 4):
   if flash_counter > 0 AND flash_counter is EVEN:
       draw 1 segment with frame 4 (blink variant at empty/filled boundary)
       advance y by 3, set drawn_count = 1
   This segment visually replaces the first surplus segment (or would occupy
   its position even if surplus_count == 0).

3. SURPLUS segments (frame 1, green):
   count = this->surplus_segments (offset +0x152C)
   Starts from drawn_count (1 if blink was drawn, else 0)
   Resets drawn_count to 0 after loop

4. OUTPUT segments (frame 2, yellow):
   count = this->output_segments (offset +0x1530)
   Resets drawn_count to 0 after loop

5. DRAIN segments (frame 3, red):                         [BOTTOM of bar]
   count = this->drain_segments (offset +0x1534)
```

**DrawSHP call signature:**

Each segment is drawn via `FUN_004aed70` (CC_Draw_Shape), thiscall on `DAT_00887300`
(primary surface), with `DAT_0087f6cc` (sidebar ConvertClass palette) in EDX:

```
// ECX = DAT_00887300 (surface, thiscall this)
// EDX = DAT_0087f6cc (ConvertClass palette, fastcall-style)
CC_Draw_Shape(
    shp = DAT_00ac4e74,     // POWERP.SHP
    frame = 0/1/2/3/4,
    position = {x_offset, y},
    clip_rect = surface->GetRect(),
    flags = 0x400,
    0, 0, 0,
    brightness = 1000,       // normal brightness (0x3E8)
    0, 0, 0, 0, 0
)
```

**Blink effect detail (verified from disassembly at 0x63fc32-0x63fc3e):**

The blink uses `flash_counter & 0x80000001` to check if the counter is even.
This is MSVC's signed modulo-2 codegen (`counter % 2`). The three instructions at
0x63fc39-0x63fc3d (DEC/OR/INC) handle negative numbers but are never reached here
since the counter is always positive. For positive values, this simplifies to
`counter & 1`: if 0 (even) → draw blink; if 1 (odd) → skip.

When counter starts at 10 (even), the **first frame IS blinking**. The sequence is:
- 10: BLINK, 9: skip, 8: BLINK, 7: skip, 6: BLINK, 5: skip, 4: BLINK, 3: skip, 2: BLINK, 1: skip, 0: done

Frame 4 is drawn at the empty/filled boundary (the position of the first surplus
segment). It always counts against the surplus segment budget: if blink was drawn,
the surplus loop starts from index 1 instead of 0. Even if surplus_count == 0,
the blink occupies that position. Since the flash counter counts down from 10 to
0 with 3-tick intervals between decrements, this creates 5 blink cycles (on/off)
over 30 ticks total.

**CC_Draw_Shape flags (verified from decompilation at 0x4aed70):**

The power bar draws with flags=0x400. Analysis of CC_Draw_Shape shows:
- **0x200** = center shape (subtracts half width/height from position). Tested at 0x4af009.
- **0x400** = no functional effect in the blitter pipeline. Not tested anywhere in
  CC_Draw_Shape or Blitter_selector. Likely a reserved/marker flag.
- **0x600** (used by pip system) = 0x400 | 0x200 = centered draw.
- The power bar uses explicit coordinates, so centering is not needed (0x400 only).

**After drawing:** Calls `FUN_00653100` (parent Draw_It, likely RadarClass::Draw_It).

---

### FUN_00640450 — PowerClass::GetTooltipText

```
Address: 0x00640450
Size: ~80 bytes
```

**Purpose:** Returns the power bar tooltip string when the mouse hovers over it.

**Logic:**
```
if tooltip_id == 999:  // 0x3E7 — the power bar tooltip ID
    power_output = *(DAT_00a83d4c + 0x53a4)  // HouseClass.PowerOutput
    power_drain  = *(DAT_00a83d4c + 0x53a8)  // HouseClass.PowerDrain
    format = StringTable::LoadString("TXT_POWER_DRAIN")
    sprintf(DAT_00ac4d30, format, power_drain, power_output)
    return DAT_00ac4d30
else:
    return parent->GetTooltipText(tooltip_id)
```

The tooltip format string "TXT_POWER_DRAIN" in the CSF string table is typically
something like "Power: %d/%d" showing drain and output as integers.

**Tooltip registration** (FUN_006403a0):

The power bar tooltip zone is registered during initialization:
```
tooltip_rect = {
    id: 999,
    x: DAT_00886f90 + x_offset,  // x_offset = 5 for Allied, 0 for Soviet
    y: DAT_00886f94 + 0x45,       // sidebar_top + 69
    width: 8,
    height: DAT_00b0b504           // full bar height
}
```

This creates a thin 8-pixel-wide hit zone over the entire power bar.

---

### FUN_006403a0 — PowerClass::Register_Tooltip

```
Address: 0x006403a0
Size: ~176 bytes
```

Calls parent `FUN_00654320`, then if `DAT_00887368 != 0` (UI system active):
1. Builds a tooltip descriptor with ID 999
2. Computes position based on Allied/Soviet offset
3. Sets tooltip rect: x, y=sidebar_top+69, width=8, height=bar_height
4. Registers via `FUN_00724730` (remove old) and `FUN_00724580` (add new)

---

## DAT_00b0b504 — Bar Height Calculation

Set by `SidebarClass::InitSidebarRect` (0x006a5200). Verified from disassembly
at 0x6a521a-0x6a5243:

```
// DAT_00b0b4f8 = sidebar strip top Y = g_SidebarWidth + 0x45 (set in InitLayoutConstants)
//   where g_SidebarWidth = DAT_00886f94 = sidebar top position
// DAT_00886f9c = calculated sidebar bottom area
// g_SidebarWidth [0x886f94] = 0x9E (158) — sidebar panel width

// Allied (player_index == 0): header = 0x1A (26)
// Soviet (player_index != 0): header = 0x12 (18)

available = DAT_00886f9c - DAT_00b0b4f8 - header - 7 + g_SidebarWidth
bar_height = (available / 50) * 50    // round DOWN to nearest multiple of 50

// Assembly: IMUL 0x51eb851f / SAR 4 = division by 50
// Then: LEA [EDX+EDX*4], LEA [EAX+EAX*4], SHL 1 = multiply by 50
```

The division by 50 followed by multiplication by 50 rounds down to the nearest
multiple of 50 pixels. This ensures a clean segment count.

`total_segments = (bar_height + 3) / 3` yields approximately `bar_height / 3`
segments. The `+3` handles integer rounding (ensures at least bar_height/3 segments).

`DAT_00b0b500 = 0x32 = 50` is stored as a named constant alongside the bar height.
`DAT_00b0b514 = 0x32 = 50` is the scroll/step size constant.

---

## Visual Behavior Summary

### Normal Operation (top-to-bottom screen order)
- **Empty (dark, frame 0):** Top of bar. Unfilled/unused capacity.
- **Surplus (green, frame 1):** Below empty. Shows excess power (output - drain > 0).
- **Output (yellow, frame 2):** Middle portion. Represents the "healthy" power band.
- **Drain (red, frame 3):** Bottom of bar. Shows power consumption.

The bar fills from the bottom upward conceptually: drain occupies the bottom,
output sits above it, surplus above that, and empty at top. As total power
infrastructure grows, more segments fill in from the top, pushing the empty
portion smaller (via the inverse-proportional Calc_Segments formula).

### No Top/Bottom Indicators
The power bar renders **no additional markers, arrows, or indicators** beyond the
colored segments. The transition between drain/output/surplus is purely the color
change between adjacent segments. There are exactly 5 CC_Draw_Shape call sites in
Draw_It: one per segment type (empty, blink, surplus, output, drain) and nothing else.

### Power Change Animation
1. Change detected via IHouse::Power_Drain/Output cache comparison
2. Flash counter set to 10, interval = 3 ticks
3. For 30 ticks: bar blinks (frame 4 alternates at the empty/filled boundary)
4. First frame (counter=10, even) IS blinking; blink shows on even values only
5. Simultaneously: `DAT_00884b8e` triggers full sidebar redraw each flash
6. After flashing: stabilize timer starts
7. Segments interpolate toward target values at +/-1 per tick
8. Priority: drain changes first, then surplus, then output

### Spy Power Plant Infiltration (Blackout)
When a spy infiltrates a power plant:
1. `BuildingClass::OnSpyInfiltrate` sets HouseClass+0x2C0 (blackout flag)
2. Sets `DAT_00884b8e = 1` (sidebar redraw)
3. Power output drops to 0 during blackout
4. The power bar naturally animates: surplus (green) and output (yellow) segments
   shrink to 0, drain (red) segments grow toward full bar
5. When blackout expires, power restores and bar animates back

### No Power (No Buildings)
With 0 total power, `Calc_Segments` returns 1 (minimum 1 filled segment due to
clamping). The bar appears nearly empty with just 1 drain segment at the bottom.

---

## Palette and Colors

The actual colors of each frame in POWERP.SHP depend on the sidebar palette, which
is theater-dependent. The palette is constructed in `SidebarClass::LoadSHPs`:

```
sidebar_pal = FUN_0072f4a0()  // get sidebar palette data
DAT_0087f6cc = new ConvertClass(sidebar_pal, sidebar_pal, theater_pal, 1, 0)
```

In the original game:
- Frame 0 (empty): Dark grey/black
- Frame 1 (surplus): Green
- Frame 2 (output): Yellow/amber
- Frame 3 (drain): Red
- Frame 4 (transition blink): Bright variant for power change flash at empty/filled boundary

These colors are baked into the POWERP.SHP frames and rendered through the sidebar
palette conversion. The game does NOT dynamically color the segments — the colors
are entirely determined by the SHP frame pixel data + palette.

**VERIFIED visually (2026-03-21):** Frame-to-color assignments confirmed by running
the game — frame 1 renders green (surplus), frame 3 renders red (drain). The draw
order (surplus at top of filled, drain at bottom) matches the original game.

---

## Numeric Display

The power bar does **NOT** render numeric values directly on the bar itself. Power
values are only shown:

1. **Tooltip** (hover): "TXT_POWER_DRAIN" format string with drain and output values
2. **Sidebar info panel** (FUN_00653fa0): When `DAT_00884b8d` is set, renders power
   information text on the sidebar below the radar, including player names and
   production values for each house

No numbers are drawn on or adjacent to the power bar segments.

---

## Constants Summary

| Constant | Value | Source | Usage |
|----------|-------|--------|-------|
| Bar segment height | 3 px | Hardcoded | Each POWERP.SHP frame is 3px tall |
| Y offset from sidebar top | 69 (0x45) | Hardcoded | Position below radar |
| Allied X offset | 5 px | Hardcoded | Shift right for Allied sidebar chrome |
| Soviet X offset | 0 px | Hardcoded | No shift for Soviet |
| Scale factor | 400.0 | 0x007ED8C8 | Logarithmic bar fill curve |
| Output cap | 100.0 | 0x007E2AC0 | Max output_portion before surplus kicks in |
| Rounding epsilon | 0.01 | 0x007E3808 | Prevents ftol rounding errors |
| Flash count | 10 | Hardcoded | Number of blink cycles on power change |
| Flash interval | 3 ticks | Hardcoded | Ticks between each blink toggle |
| Tooltip ID | 999 (0x3E7) | Hardcoded | Power bar tooltip identifier |
| Tooltip width | 8 px | Hardcoded | Hit-test zone width |
| Bar height rounding | 50 px | Hardcoded | Bar height rounded to multiple of 50 |
| DrawSHP flags | 0x400 | Hardcoded | No-op flag (not tested in blitter); 0x200 = centering |
| Brightness | 1000 | Hardcoded | Normal brightness (no dimming) |

---

## Relationship to PIPS.SHP System

The power bar (POWERP.SHP) is a **completely separate** rendering system from
the general pip system (PIPS.SHP / PIPS2.SHP / PIPBRD.SHP):

- **POWERP.SHP** — sidebar power bar only. 5 frames (empty, surplus, output, drain, blink).
  Drawn by PowerClass::Draw_It. Fixed sidebar position. Uses ConvertClass palette.
- **PIPS.SHP** — health pips, cargo pips, occupant pips, veteran stars, ammo indicators.
  21+ frames. Drawn by DrawHealthBar/DrawPipScalePips/DrawVeterancyPips. World-space
  positions attached to entities.

**Power state does NOT affect PIPS.SHP rendering.** Building health pips use the
same green/yellow/red frames regardless of whether the building is powered. No power
function (IsOperational, PowerRatio, etc.) is called from any pip-drawing code.

### PipScale=Power (Vestigial Enum)

The PipScale enum at `TechnoTypeClass+0x3D4` includes a `Power` value (enum 4,
parsed from string "Power" by FUN_00474940 at table 0x81B9B0). However:

- **No unit or building** in vanilla rules.ini/rulesmd.ini uses `PipScale=Power`
- **No specific rendering code** handles PipScale==4 in DrawPipScalePips (0x709A90)
- It is a vestigial enum value — defined in the parser but never used or implemented
- Mods that set `PipScale=Power` would get no pip display (falls through without drawing)

---

## Original Engine: Index-0 Pixel Handling (Verified from Binary)

The plain blitter at `0x4914C0` (selected for flags=0x400) has an explicit
`TEST AL, AL` / `JZ` check: when a pixel index is 0, the write is **skipped**
and the destination pixel is left unchanged. The sidebar background behind the
power bar shows through index-0 pixels.

```asm
004914d5: MOV AL, [ESI]       ; read source pixel index
004914d8: TEST AL, AL         ; is index == 0?
004914de: JZ 0x004914f1       ; YES -> skip write, leave background
004914e4: MOV EBX, [ECX+0x4]  ; remap table pointer
004914ec: MOV AL, [EBX+EAX]   ; remap_table[index]
004914ef: MOV [EDX], AL       ; write remapped pixel to destination
004914f1: INC EDX             ; advance dest pointer (always, even if skipped)
```

The ConvertClass remap table at `+0x174` is only consulted AFTER the index-0
check. Index 0 never reaches the lookup — the ConvertClass cannot remap index 0.

**Key insight:** In the original engine, index-0 pixels in POWERP.SHP are
invisible because the blitter skips them, leaving the dark sidebar chrome
background visible. There is no "opaque black" — it's simply a no-op.

---

## Rust Implementation: Rendering Pipeline Differences

Our renderer uses textured quads with alpha blending instead of scanline blitting.
This creates several differences from the original that must be handled:

### 1. Index-0 Transparency vs Opaque Black

Our palette loader correctly sets index 0 to alpha=0 (transparent). But since we
draw POWERP.SHP frames as textured quads on top of the sidebar chrome, transparent
pixels create visible holes instead of showing the background.

**Current fix** (`sidebar_chrome.rs`): Force all powerp.shp pixels to alpha=255.
Index-0 pixels become opaque black, which visually matches the dark sidebar
background behind the power bar.

### 2. SHP Frame Offset Within Canvas (INVESTIGATE)

`render_shp()` creates a canvas at `shp.width × shp.height` and blits frame data
at offset `(frame_x, frame_y)`. If POWERP.SHP frames have non-zero offsets, the
canvas has padding around the frame data:

- **Before alpha fix:** padding is transparent (alpha=0) — could show as gap
- **After alpha fix:** padding is opaque black — could show as dark border/notch

**This is the most likely cause of the top-left "notch" artifact.** If `frame_x > 0`
or `frame_y > 0`, the canvas has opaque black pixels between the origin and the
actual colored frame data.

**Fix options:**
1. Crop to frame dimensions only: use `(frame_rgba, fw, fh)` instead of canvas
2. Or shift the UV coordinates to exclude the padding

**TODO:** Dump actual POWERP.SHP frame dimensions vs canvas dimensions to confirm.

### 3. Size Mismatch: SHP Pixels vs Render Quad

The layout spec draws each segment at `power_bar_width × power_bar_tile_height`
(10×3 at 1x scale). If the actual SHP frame is a different size (e.g., 12×2),
the GPU stretches the texture with **Nearest filtering**. Some texels map to
1 screen pixel while others map to 0 or 2, creating uneven banding.

The original engine draws POWERP.SHP at its native pixel dimensions (each frame is
drawn once per segment, no scaling). Our renderer should either match the quad size
to the SHP's `pixel_size` or accept the Nearest-filter artifacts.

### 4. sRGB Gamma on Linear Palette

The sidebar atlas texture uses `Rgba8UnormSrgb` format. Palette colors from
`sidebar.pal` are raw linear bytes. The GPU applies sRGB gamma correction during
sampling, making colors appear lighter than the original game's direct linear
blitting to a 16-bit surface.

**Impact:** Subtle color shift on all sidebar chrome, not just power bar. The
original uses `ConvertClass(sidebar_pal, sidebar_pal, theater_pal, 1, 0)` which
operates in linear color space. Our sRGB texture adds unintended gamma.

### 5. Theater Palette Merge (Missing)

The original creates the sidebar ConvertClass by merging `sidebar.pal` with the
theater palette:
```c
DAT_0087f6cc = new ConvertClass(sidebar_pal, sidebar_pal, theater_pal, 1, 0)
```

Our code uses `sidebar.pal` alone. For POWERP.SHP (simple solid colors in basic
palette range) this doesn't matter. But other sidebar SHPs using special remap
ranges (player colors, shadow indices) could render incorrectly.
