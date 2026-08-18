# UnitClass::DrawExtras — Ghidra Report

**Function:** `UnitClass__DrawExtras` at `0x0073cec0`
**Size:** 0x588 bytes (0x0073cec0 – 0x0073d448)
**Vtable:** Overrides `TechnoClass__DrawExtras` at `0x006f5190`
**Confidence:** HIGH — all findings verified from binary

## Overview

This is the main **UnitClass visual overlay/extras renderer**. It handles oregath.shp
(harvest overlay), the FLAGFLY.SHP (CTF flag banner), the main unit body draw dispatch,
disguise/image swap, and pip/indicator drawing. Called once per visible unit per frame.

## Section 1: OreGath Overlay (Harvest Arm Animation)

**Asset:** `OREGATH.SHP` — lazy-loaded from MIX on first use, cached in `DAT_00b1cf98`.

### Conditions to draw

All must be true:
1. `UnitTypeClass+0xE0E != 0` — type has `Harvester=yes`
2. `unit+0x6D2 != 0` — unit is actively harvesting (set when Mission == Harvest,
   cleared when mission changes at `0x00736860`)
3. `locomotor->vfunc_0x80() == false` — locomotor is NOT currently moving
   (harvester must be stationary, parked on ore)
4. `unit+0x278 == 0` — not in deploy state
5. `vtable+0x1D4() == false` — not cloaking
6. `vtable+0x1D8() == false` — not being chronoshifted/warped

### Position offset (sin/cos arm placement)

The overlay is NOT drawn at the unit center. The engine:
1. Reads the unit's body facing from `unit+0x388` (a Direction/RateTimer, 0–65535)
2. Extracts facing angle: `((direction >> 12) + 1) >> 1`, giving 0–7
3. Converts back to radian angle via `SHL 13` then multiply by pi constant at `0x007E2810`
4. Computes `cos(angle)` and `sin(angle)` offsets
5. Multiplies by the unit's body dimensions from `vtable+0xAC` (GetExtent/GetCoords)
6. Adds this world-space offset to the draw position before converting to screen coords

This makes the overlay follow the physical arm position as the harvester rotates.

### Frame calculation

```
facing_index = 7 - (((direction >> 12) + 1) >> 1) & 7)
anim_frame   = (unit+0x538 + g_CurrentFrameCounter) % 15
shp_frame    = facing_index * 15 + anim_frame
```

- `unit+0x538` (dword) = per-unit random desync offset, ensures multiple harvesters
  don't animate in lockstep
- 15 frames per facing, 8 facings = 120 total SHP frames
- Animation rate: 1 frame per game tick (hardcoded, no INI Rate)
- Facing reversal `7 - idx` matches SHP frame layout (clockwise → counter-clockwise)

### Draw call

```
CC_Draw_Shape(oregath_shp, shp_frame, screen_pos, clip_rect,
              flags=0x2A00, palette=0, z_priority, 0, z_buf_height,
              0, 0, 0, 0, 0)
```

Z-priority = `cellHeight + bridgeAdjust + RulesClass+0x17D4 - 2`

## Section 2: Bridge/Height Z Calculation

Before any drawing, computes Z-priority for proper depth sorting:

- Checks `cell+0x140 & 0x100` (on bridge surface)
- Checks `cell+0x140 & 0x800` (bridge direction — NS vs EW)
- If on bridge, checks adjacent cell for matching bridge flag
- Reads theater-specific bridge height from scenario data:
  - `Scenario+0x355C` — IonStorm active
  - `Scenario+0x3590` — Psychic Dominator active
  - `Scenario+0x3574` — Nuclear missile active
  - `Scenario+0x3544` — normal conditions
- Height = `cell+0x10A + bridgeHeight * 4`
- For underground/tunnel cells (`cell+0x140 & 0x10000`), applies -500 offset
- Final Z += `RulesClass+0x17D4` (global render Z adjustment)

## Section 3: Image/Disguise Swap

After oregath, the function temporarily swaps the unit's type pointer for rendering:

**Harvester image swap:**
```
if (TypeClass+0xE0E [Harvester=yes] AND unit+0x6D1 [some flag] AND TypeClass+0x6B8 != 0):
    unit->TypeClass = TypeClass+0x6B8   // Swap to alternate image type
```
`TypeClass+0x6B8` is likely the `Image=` override from art.ini.

**Disguise system:**
```
if (vtable+0x440(g_PlayerPtr) == false):   // Unit NOT owned by local player
    render_type = vtable+0xCC(1)           // Get disguised appearance
```
This is how spy units render as the enemy type when viewed by the opponent.

**Additional override:**
```
if (unit+0x6E0 [byte flag] AND TypeClass+0x6B8 != 0):
    render_type = TypeClass+0x6B8
```

## Section 4: Main Body Draw Dispatch

```
if (unit+0x684 == 0xFF):   // No special visual override state
    if (TypeClass+0x236 == 0):
        vtable+0x558(...)   // SHP body draw
    elif (TypeClass+0xB0 != 0):
        vtable+0x554(...)   // Voxel body draw
```

`TypeClass+0x236` is the **Voxel flag** — determines SHP vs VXL rendering path.
The type pointer is restored to the original after drawing.

## Section 5: FLAGFLY.SHP — Capture the Flag Banner

**Asset:** `FLAGFLY.SHP` at string `0x008458F8`

**Mystery solved:** `unit+0x6CC` (param_1[0x1B3]) is the **CTF flag carrier house index**.
- Value `-1` (0xFFFFFFFF) = unit is not carrying a flag
- Any other value = index into the HouseClass array of the flag's owning house

### Attach/Detach functions

| Address | Name | Description |
|---------|------|-------------|
| 0x00740DF0 | `UnitClass__AttachFlag` | Sets `unit+0x6CC = houseIndex`, calls redraw |
| 0x00740E20 | `UnitClass__DetachFlag` | Sets `unit+0x6CC = -1`, calls redraw |

### Drawing logic

When `unit+0x6CC != -1`:
1. Look up the owning house: `HouseClass* owner = g_HouseArray[unit+0x6CC]`
2. Get the house's color scheme index: `owner+0x16054`
3. Look up the color remap table from `DAT_00b054d4 + colorIndex * 4`
4. Load `FLAGFLY.SHP` from MIX
5. Frame = `g_CurrentFrameCounter % 14` (14-frame looping animation)
6. Draw with flags `0x0A00` using the house-colored remap at `remap+0x30C`

The flag banner is drawn in the owning house's color, so you can tell whose flag
the unit is carrying. Uses a different blitter flag (0x0A00) than oregath (0x2A00).

### CTF Mode reference

The `CaptureTheFlag` string at `0x0083CFE8` is read in `[General]` section.
Related log strings at `0x00824A00`: `"MPlayer_Defeated() - Flag_To_Win"`,
`"MPlayer_Defeated() - Flag_To_Lose"`.

## Section 6: Pips and Indicators

At the end of the function:
- `FUN_004db250` — stub/no-op in this build
- `FUN_006ff960` — draws **pips** (cargo dots, veteran stars, etc.) as small
  colored rectangles on the primary surface

### Timed pip fade

```
if (unit+0x3BC != -1):              // Timer start frame
    elapsed = g_CurrentFrameCounter - unit+0x3BC
    if (elapsed >= unit+0x3C4):     // Duration
        return                       // Timer expired, skip pips
    remaining = unit+0x3C4 - elapsed
if (remaining != 0):
    FUN_006ff960()                  // Draw pips
```

## Key UnitClass Field Map (byte offsets)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x08C | byte | IsOnBridge | Bridge surface flag |
| 0x09C | 12 | Coords | Unit world position (X, Y, Z as ints) |
| 0x278 | int | DeployState | Non-zero = deploying/busy |
| 0x388 | int | BodyFacing | Body direction (RA2 Direction 0–65535) |
| 0x3BC | int | PipTimerStart | Frame counter when pip timer started (-1=inactive) |
| 0x3C4 | int | PipTimerDuration | How many frames pips remain visible |
| 0x424 | byte | ElevatedFlag | Adds +14px Y offset when set |
| 0x538 | int | AnimDesyncOffset | Random offset for oregath anim desync |
| 0x674 | ptr | Locomotor | ILocomotion interface pointer |
| 0x684 | byte | VisualOverrideState | 0xFF = normal draw, other = skip body |
| 0x6B4 | byte | DrawOffsetY | Pixel Y draw adjustment |
| 0x6C4 | ptr | TypeClass | Pointer to UnitTypeClass |
| 0x6CC | int | FlagCarrierHouse | CTF flag owner house index (-1=none) |
| 0x6D1 | byte | DisguiseFlag | Used for image swap in disguise system |
| 0x6D2 | byte | IsHarvesting | Active harvest state (oregath trigger) |
| 0x6E0 | byte | AltImageFlag | Triggers TypeClass+0x6B8 image override |
| 0x6E1 | byte | DrawDisable1 | Suppresses all drawing |
| 0x6E2 | byte | DrawDisable2 | Suppresses all drawing |

## Key UnitTypeClass Field Map (byte offsets)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x0B0 | ptr | VoxelData | VXL model pointer (null = no voxel) |
| 0x236 | byte | IsVoxel | 0 = SHP rendering, 1 = VXL rendering |
| 0x6B8 | ptr | ImageOverride | Alternate TypeClass for visual override |
| 0xE0E | byte | IsHarvester | `Harvester=yes` from rules.ini |

## Comparison with Current Rust Implementation

**Already correct:**
- 15 frames x 8 facings frame layout
- Per-unit random desync offset
- 1 frame per game tick animation rate
- Harvest state gating

**Not yet implemented:**
- Sin/cos position offset for arm placement (subtle visual improvement)
- Locomotor-not-moving gate (our MinerState::Harvest should cover this)
- Cloaking/chronoshift skip conditions
- FLAGFLY.SHP CTF flag banner overlay
- Disguise/image type swap for body rendering
- Pip timer fade system

## Ghidra Labels Created

| Address | Label |
|---------|-------|
| 0x0073CEC0 | `UnitClass__DrawExtras` |
| 0x00740DF0 | `UnitClass__AttachFlag` |
| 0x00740E20 | `UnitClass__DetachFlag` |
