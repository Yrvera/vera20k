# Health Bar / Pip Positioning — Ghidra Research Report

## Overview

Health bars in RA2/YR are drawn by `DrawHealthBar` (**vtable+0x44C**),
called from `DrawExtras` (`0x6F5190`, vtable+0x110) during the overlay render pass.
Three functions form the health/bracket system:

- `0x6F60D0–0x6F649D` — **DrawBehind** (vtable+0x10C): isometric line brackets for buildings
  (Ghidra **misnames** this as `TechnoClass__DrawHealthBar`)
- `0x6F5190` — **DrawExtras** (vtable+0x110): master overlay orchestrator, calls DrawHealthBar
- `0x6F64A0–0x6F6ABF` — **DrawHealthBar** (vtable+0x44C): pip bars for ALL entity types

**IMPORTANT vtable correction:** The render loop call at `0x6d4750` (`CALL [EAX + 0x438]`)
is **DrawActionLines**, NOT DrawHealthBar. DrawHealthBar is at vtable offset **0x44C** and is
called from DrawExtras, which passes valid `pLocation` and `pBounds` parameters (never NULL).
All vtable offsets in this document use the YRpp convention (verified against constructors).

The DrawHealthBar function branches on `WhatAmI()` (AbstractType enum):
- `== 6` (Building) → Building path (pips along NW foundation edge, PIPS.SHP frames 0-4)
- `== 0xF` (Infantry) → Infantry path (8-pip bar, PIPBRD frame 1, PIPS.SHP frames 16-18)
- Everything else (Unit=1, Aircraft=2) → Vehicle/aircraft path (17-pip bar, PIPBRD frame 0, PIPS.SHP frames 16-18)

DrawExtras calls DrawHealthBar in **two scenarios** (decompiled from `0x6F5190`):

1. **Selected entities** (field `0x83` = IsSelected is non-zero): calls `DrawHealthBar(pLocation, pBounds, false)`.
   The `bUnk3=false` path draws PIPBRD.SHP background + health pips.

2. **Hovered entities** (field `0x431` is non-zero AND `0x83` is zero): calls `DrawHealthBar(pLocation, pBounds, true)`.
   Since IsSelected is 0, PIPBRD background is skipped — only the colored health pips are drawn (no bracket background).

DrawExtras also checks **alliance/EnemyHealth visibility** before calling vtable+0x448:
- Gated by `Health > 0` AND (`IsAlliedWith(localPlayer)` OR `EnemyHealth` flag at RulesClass+0x17E6)
- `EnemyHealth` is an `[AudioVisual]` INI key (at `0x83a354`) that enables showing enemy health bars

## Key INI Properties

### `PixelSelectionBracketDelta` (rules.ini / rulesmd.ini)

- **TechnoTypeClass field offset:** `0x3E0` (dword index `0xF8` in INI parser at `0x714173`)
- **Parsed from INI at:** `0x714173` in `TechnoTypeClass::ReadINI` at `0x00712170`
- **Type:** `int` (signed pixels)
- **Default:** `0`
- **Effect:** Shifts the health bar and selection bracket vertically. **Negative values move the bar UP** (closer to sprite top), positive values move DOWN.
- **Comment in INI:** `"higher number draws lower. Pixel difference from normal for selection bracket"`
- **Used for:** Units, infantry, aircraft — NOT buildings (buildings use foundation geometry)

#### Example values from rulesmd.ini:
| Unit | Value | Notes |
|------|-------|-------|
| Battle Fortress (DVRL variants) | `-6` | Vehicle |
| Kirov Airship | `-8` | Aircraft, tall sprite |
| Aircraft Carrier / Dreadnought | `-26` | Naval, very tall sprite |
| IFV | `-6` | Vehicle |
| Siege Chopper | `-6` | Vehicle/aircraft |

### `Height` (rules.ini)

- **Type:** `int` (in "leptons", meaning abstract height units)
- **Default:** varies
- **Effect:** For buildings, determines the vertical extent of the selection bracket. Phobos uses `Height * 12` pixels for the bracket's vertical span, but this formula is NOT found in gamemd.exe — the original engine computes building bracket height through isometric coordinate transforms (`FUN_006d1f10` = CoordsToClient) rather than a direct multiplication. Note: `FUN_0041c230` is just a **CoordStruct constructor** (stores X,Y,Z), NOT a coordinate transform.

### `Foundation` (art.ini)

- **Type:** string like `"3x2"` (width x height in cells)
- **Effect:** In Phobos, building pip count uses `foundationHeight * 7 + foundationHeight / 2` (equivalent to `foundationHeight * 7.5` integer math — the screen-space width of an isometric NW edge). In gamemd.exe, the pip count is derived through full 3D coordinate transforms rather than this direct formula, but the result is equivalent.
- **Selection bracket geometry:** Foundation width/height determine the isometric diamond shape of the bracket.

### `YDrawOffset` (art.ini)

- **Type:** `int` (pixels)
- **Effect:** Vertical sprite drawing offset. Currently used in Rust engine for building pip vertical positioning. In the original game, this is NOT directly used for health bar positioning — the game uses the actual sprite bounds instead.

## Original Game Positioning Logic (from Ghidra decompilation)

### Units & Infantry (Non-Building) — DrawHealthBar at `0x6F64A0`

**YRpp confirmed signature** (from `TechnoClass.h:300`):
```cpp
virtual void DrawHealthBar(Point2D *pLocation, RectangleStruct *pBounds, bool bUnk3) const;
```

**`pLocation` origin:** Computed via `CoordsToClient2` (`0x6D2140`) from the entity's world
coordinates (via `GetCoords`, vtable+0x48). Called from the render pass at `FUN_006d8db0`.
**No additional pixel offsets** are applied between CoordsToClient and DrawExtras — pLocation
is the raw projected screen position. The isometric projection is:
- `screen_x = (worldX * 60 - worldY * 60) / 512` minus viewport scroll X
- `screen_y = (worldX * 30 + worldY * 30) / 512 - z_height` minus viewport scroll Y

This maps directly to the Rust engine's `screen_x`/`screen_y` (which includes sub-cell offsets
from `lepton_to_screen`). No +15 or +30 cell center offset is needed — the sub-cell center
lepton values (128, 128) already produce the correct screen position in both systems.

`pBounds` is the clipping rectangle, `bUnk3` = false for selected entities, true for hovered.
`PixelSelectionBracketDelta` is read from TechnoTypeClass field `0x3E0` (referred to as `delta` below).

#### PIPBRD.SHP Background

Drawn only when entity field `0x83` is non-zero (selected). Z-order: `0xE00`.

| | Frame | X | Y |
|---|---|---|---|
| **Vehicles/Aircraft** | 0 | pLocation.X **+ 1** | pLocation.Y + delta **- 26** (0x1A) |
| **Infantry** | 1 | pLocation.X **+ 11** (0xB) | pLocation.Y + delta **- 25** (0x19) |

#### Health Pips (PIPS.SHP)

Each pip is drawn 2 pixels apart horizontally. Z-order: `0x600`.

| | Max pips | X start offset | Y offset |
|---|---|---|---|
| **Vehicles/Aircraft** | **17** (0x11) | **-15** (0xFFFFFFF1) | delta **- 25** (0x19) |
| **Infantry** | **8** | **-5** (0xFFFFFFFB) | delta **- 24** (0x18) |

**Pip X formula:** `X = pLocation.X + xStartOffset + pipIndex * 2`
**Pip Y formula:** `Y = pLocation.Y + yOffset` (constant for all pips in a bar)

#### Health Color (PIPS.SHP frame selection — units/infantry only)

```
Frame 0x10 (16) = Green   (health > ConditionYellow)
Frame 0x11 (17) = Yellow   (health <= ConditionYellow, at RulesClass+0x1700)
Frame 0x12 (18) = Red      (health <= ConditionRed, at RulesClass+0x1708)
```

**Health ratio formula** (`ObjectClass::GetHealthRatio` at `0x5f5c60`):
`ratio = (float)Health / (float)MaxStrength` where Health is at entity+0x6C, MaxStrength at TypeClass+0xA0.

Filled pip count = `ratio * maxPips`, clamped to `[1, maxPips]`.

Color selection uses sequential override: start green, if `ratio <= ConditionYellow` → yellow, if `ratio <= ConditionRed` → red. Only filled pips are drawn (no empty pip background — PIPBRD.SHP provides the background).

All pips and brackets use the **theater palette** — no special palette is loaded. Colors come from
the frame selection within PIPS.SHP/PIPBRD.SHP.

#### Z-Order / Flags System

The "z-priority" values passed to DrawSHP **double as flag fields**:
- Bit `0x200` = **canvas centering** (DrawSHP subtracts `canvas_w/2`, `canvas_h/2` from position before adding frame offsets)
- Bit `0x400` and `0x800` = other rendering flags

| Value | Flags | Used for |
|-------|-------|----------|
| `0xE00` | 0x800 + 0x400 + **0x200** | PIPBRD.SHP (bracket background, canvas centered) |
| `0x600` | 0x400 + **0x200** | PIPS.SHP (health pips, canvas centered) |
| `0x601` | 0x400 + **0x200** + 0x1 | Self-heal pip (blinking/translucent, canvas centered) |

**Both PIPBRD and PIPS use canvas centering.** DrawSHP formula with 0x200:
```
actualX = drawX - canvas_width/2 + frame_x
actualY = drawY - canvas_height/2 + frame_y
```

#### DrawSHP Parameter Map (`FUN_004aed70`)

The SHP drawing function uses **`__fastcall`** with 2 register + 14 stack parameters:

```
ECX = DSurface* (DAT_00887314 = compositing surface)
EDX = SHPDrawCtx* (DAT_0087f6c4 = palette ramp + z-storage)
arg0  = SHP* (e.g., PIPBRD.SHP or PIPS.SHP pointer)
arg1  = int frameIndex
arg2  = Point2D* position
arg3  = RECT* clipBounds (pBounds from DrawHealthBar)
arg4  = uint zPriority (0x600, 0xE00, etc.)
arg5  = uint flags (0x200 = center on position)
arg6  = int zAdjust (nonzero enables z-buffer testing)
arg7  = int unused (always 0)
arg8  = int brightness (1000 = normal, sentinel for "no tinting")
arg9  = int unused (always 0)
arg10 = SHP* remapSHP (optional overlay, 0 = none)
arg11 = int remapFrame
arg12 = int remapOffsetX
arg13 = int remapOffsetY
```

#### DrawPipScalePips (vtable+0x450 = `FUN_00709A90`)

After health bar pips, DrawHealthBar conditionally calls DrawPipScalePips (vtable+0x450)
which is gated by an alliance/visibility check (`FUN_004f9a50` at `0x4f9a50`). The position
passed is X = pLocation.X - 10, Y = pLocation.Y + 10.

The main DrawPipScalePips function (`FUN_00709A90`, 3553 bytes). Its YRpp signature:
```cpp
virtual void DrawPipScalePips(Point2D *pLocation, Point2D *pOriginalLocation,
                              RectangleStruct *pBounds) const;
```

It handles:
- **Cargo/occupant pips** using PIPS.SHP and PIPS2.SHP
- **Ammo pips** with PipWrap grouping (from TypeClass+0xD5C)
- **Self-heal blink** — two heal types determined by `TypeClass+0xD97` (`Organic` bool):
  - **Organic** (infantry with Organic=true): PIPS.SHP frame **0xD (13)**, blinks via
    `frameCounter % SelfHealInfantryFrames (RulesClass+0x30) < 6`. Requires `HouseClass::DoInfantrySelfHeal()`
    (house+0x164 > 0, from owning Hospital).
  - **Mechanical** (units with Organic=false): PIPS.SHP frame **0x14 (20)**, blinks via
    `frameCounter % SelfHealUnitFrames (RulesClass+0x38) < 6`. Requires `HouseClass::DoUnitsSelfHeal()`
    (house+0x168 > 0, from owning Armory).
  - Only drawn when `Health < MaxStrength`. Pip flashes with z-order 0x601 (translucent) during
    blink frames, 0x600 (solid) otherwise.
  - Position offsets from pip start: **Unit** (+38, -32), **Infantry** (+19, -35).
- **Group number** overlay using owner house color palette
- Chains to vtable+0x458 (DrawExtraInfo) for veterancy labels

### Selection Brackets

**Buildings** get 3D isometric line-drawn brackets via `DrawBehind` at `0x6F60D0`
(Ghidra **misnames** this as `TechnoClass__DrawHealthBar`). It uses `FUN_006f5ef0` to draw
4 L-shaped bracket corners from interpolated 3D coordinates. The bracket color is
palette index `0xF` (green, normal) or `0xC` (when GetHeight() < -4).

**Units/infantry/aircraft do NOT have separate line-drawn brackets.** Their "bracket" IS the
**PIPBRD.SHP background** drawn inside DrawHealthBar. PIPBRD.SHP frame 0 = vehicle/aircraft bracket,
frame 1 = infantry bracket. Only these 2 frames are ever referenced. There is no separate
bracket function for non-building entities.

**No class overrides DrawHealthBar** — all classes (BuildingClass, InfantryClass, UnitClass,
AircraftClass) use the same `TechnoClass::DrawHealthBar` at `0x6F64A0`. Aircraft use the
vehicle path (PIPBRD frame 0, 17 pips).

The master drawing function that orchestrates all overlays is `DrawExtras` at `0x6F5190`
(vtable+0x110). It is called from `TacticalClass::DrawObjects` (`0x443C60`) during the render
pass, with pLocation computed from `CoordsToClient`. No subclass overrides DrawExtras.

**DrawExtras rendering order** (if entity is not sinking):
1. **Ivan bomb** — BOMBCURS.SHP with timer animation (if AttachedBomb at +0x38 is non-null)
2. **Deploy-ready** — WRENCH.SHP with 6-frame blink (if IsDeployReady flag is set)
3. **Selection brackets** — isometric line brackets for buildings via `DrawBracketCorner`
4. **Alliance pips** — vtable+0x448 for allied/enemy-visible entities
5. **Health bar** — vtable+0x44C (`DrawHealthBar`) for selected and/or hovered entities
6. **Talk bubble** — TALKBUBL.SHP (triggered by map trigger scripts, one entity at a time via `DAT_00b0eb38`)

Its entry guard checks:
- **`IsSinking`** (field `0x3CD`): if true, DrawExtras returns immediately — no overlays at all
- **Cloaked units still show health bars when selected** — the cloak/disguise gate only
  suppresses veterancy pips (`DrawVeterancyPips`), NOT health bars

### Buildings — Health Pips

Building health pips are drawn along the **NW foundation edge** in isometric space.
Each subsequent pip steps by **(-4, +2) pixels** in screen coordinates (moving in the NW direction along the foundation edge, from the north corner toward the west corner). The first pip starts at the rightmost position (north corner end) and draws toward the left (west corner).

#### Building Pip Color (PIPS.SHP — different frames from units!)

Color is determined by dedicated check functions, NOT the same sequential-override used for units:
- `FUN_005f5d20` — returns 1 if `ConditionRed < healthRatio <= ConditionYellow` (yellow condition)
- `FUN_005f5cd0` — returns 1 if `healthRatio <= ConditionRed AND Health > 0` (red condition)

```
Frame 0 = Empty pip (unfilled background — drawn explicitly for missing health)
Frame 1 = Green   (healthy)
Frame 2 = Yellow  (damaged)
Frame 4 = Red     (critical)
```

**Key difference from units:** Buildings draw **explicit empty pips** (frame 0) for the unfilled
portion of the health bar. Units/infantry do NOT draw empty pips — PIPBRD.SHP serves as their
background instead.

#### Building Pip Geometry (fully traced from assembly)

**`unaff_retaddr` in Ghidra = `pLocation`** — Ghidra lost track of this parameter because the
stack slot `[ESP+0x50]` is overwritten with a counter variable at `0x6F6650`. After that point,
EDI holds the pLocation pointer.

**Call chain at entry of building path:**
1. `GetHeight()` (vtable+0x1C8, `0x5F5F40`) — return value is **discarded** (side-effect only)
2. `GetType()` (vtable+0x84) → `Dimension2()` (TypeClass vtable+0x7C, `0x464AF0`) — returns
   `{foundationWidth << 8, foundationHeight << 8, Height * HeightFactor}` in leptons.
   Foundation dimensions come from lookup tables at `0x8192B8` (width) and `0x819310` (height).

**Pip count derivation:**
1. Half-dimensions computed: `{width/2, height/2, z/2}`
2. Three lepton-space offsets projected to screen via `CoordsToClient` (`0x6D1F10`):
   - Left edge at full Z: `{-width/2, height/2, z}`
   - Top edge at full Z: `{-width/2, -height/2, z}`
   - Left edge at ground: `{-width/2, height/2, 0}`
3. `numPips = (leftEdge.screenY - topEdge.screenY) / 2`
   — mathematically equivalent to `7.5 * foundationHeight` (Phobos formula confirmed)

**First pip screen position:**
- `X = pLocation.X + leftEdge.screenX + 3 + numPips * 4`
- `Y = pLocation.Y + leftEdge.screenY + 2 + (2 - numPips * 2)`

**Each subsequent pip:** steps (-4, +2) — marching from the bottom of the west foundation edge
upward toward the top (north) corner.

Filled pips = `healthRatio * numPips`, clamped to `[1, numPips]`.

#### Building DrawPipScale Visibility

After health pips, buildings conditionally call vtable+0x450 for DrawPipScale when ANY of:
- `PipsDrawForAll` (TechnoTypeClass+0x3D8, INI key `PipsDrawForAll`) is set, OR
- Allied with local player (`HouseClass::IsAlliedWith`), OR
- Spied by local player (`DisplayProductionTo` bitfield at entity+0x210), OR
- **`CanBeOccupied`** (BuildingTypeClass+0x157B) is true — garrisonable buildings (Battle Bunkers,
  civilian buildings) ALWAYS show occupant pips to ALL players, including enemies.

#### Pip Count Examples

Pip count depends only on `foundationHeight`, not width. Formula: `(15 * foundH) / 2`:

| Foundation | Example | foundH | Pips |
|-----------|---------|--------|------|
| 1x1 | Pillbox | 1 | 7 |
| 2x2 | Silo | 2 | 15 |
| 3x2 | Refinery | 2 | 15 |
| 5x3 | War Factory | 3 | 22 |
| 4x4 | Construction Yard | 4 | 30 |

### Buildings — Selection Bracket (Phobos reference)

**Source:** Phobos `GetBuildingSelectBracketPosition()` (Body.Visuals.cpp:288)

```cpp
int foundationWidth = pBuildingType->GetFoundationWidth();
int foundationHeight = pBuildingType->GetFoundationHeight(false);
int height = pBuildingType->Height * 12;              // Phobos formula, NOT in gamemd.exe
int lengthW = foundationWidth * 7 + foundationWidth / 2;
int lengthH = foundationHeight * 7 + foundationHeight / 2;

// Position offset from screen center:
position.X += positionFix.X + 3 + lengthH * 4;
position.Y += positionFix.Y + 4 - lengthH * 2;

// Building bracket has 6 anchor points:
// Top, LeftTop, LeftBottom, Bottom, RightBottom, RightTop
// Height affects vertical extent of the bracket (LeftBottom, Bottom, RightBottom add `height`)
```

Buildings do NOT use `PixelSelectionBracketDelta`. They use:
- Foundation dimensions for horizontal extent
- `Height` for vertical extent (via coordinate transforms in gamemd.exe, via `Height * 12` in Phobos)
- `Dimension2()` for sprite dimension correction

## Ghidra Function Addresses

| Address | Function | Vtable | Notes |
|---------|----------|--------|-------|
| `0x006d3d10` | `TacticalClass::Draw` | — | Main render loop. Calls DrawActionLines (0x438) at `0x6d4750`, NOT DrawHealthBar |
| `0x006F5190` | **DrawExtras** | **+0x110** | Master overlay function. Calls DrawHealthBar at `0x6f5e27`/`0x6f5e7b`/`0x6f5e87` via vtable+0x44C |
| `0x006F60D0`–`0x6F649D` | **DrawBehind** | **+0x10C** | Draws isometric line brackets for buildings (WhatAmI==6). Ghidra **misnames** as `TechnoClass__DrawHealthBar` |
| `0x006F64A0`–`0x6F6ABF` | **DrawHealthBar** | **+0x44C** | Health pips for all entity types. Signature: `(Point2D*, RectangleStruct*, bool)`. `RET 0xC` |
| `0x006F5EF0`–`0x6F6021` | Bracket line helper | — | Draws L-shaped bracket corners via interpolated 3D coords, called from DrawBehind |
| `0x006F64A9` | DrawHealthBar entry | — | Phobos hook: `TechnoClass_DrawHealthBar_Hide` |
| `0x006F65D1` | DrawHealthBar buildings | — | Phobos hook: `TechnoClass_DrawHealthBar_Buildings` |
| `0x006F683C` | DrawHealthBar non-building | — | WhatAmI != 6 lands here; infantry check (== 0xF) follows |
| `0x006F6637` | DrawHealthBar building pips | — | Phobos hook: `TechnoClass_DrawHealthBar_HideBuildingsPips` |
| `0x006F6A58` | DrawHealthBar pip scale | — | Phobos hook: `TechnoClass_DrawHealthBar_PermanentPipScale` |
| `0x00712170` | TechnoTypeClass::ReadINI | — | PixelSelectionBracketDelta at dword index 0xF8 → offset 0x3E0 |
| `0x005f76b0` | Pip asset loader | — | Lazy-loads PIPBRD.SHP, PIPS.SHP, PIPS2.SHP, TALKBUBL.SHP |
| `0x0041c230` | CoordStruct constructor | — | **NOT a coordinate transform** — just stores {X,Y,Z}. Called in building path |
| `0x006d1f10` | CoordsToClient | — | Isometric transform: `screenX = 30*(X-Y)/256`, `screenY = 15*(X+Y)/256 - Z` |
| `0x004aed70` | SHP frame draw | — | Blits individual PIPBRD/PIPS frames to the drawing surface |
| `0x005F5F40` | GetHeight | **+0x1C8** | Returns entity height above ground. Called in building path but return value discarded (side-effect only) |
| `0x00464AF0` | BuildingTypeClass::Dimension2 | TypeClass+0x7C | Returns `{foundW<<8, foundH<<8, Height*HeightFactor}` in leptons. Tables at `0x8192B8`/`0x819310` |
| `0x005f5c60` | GetHealthRatio | — | Returns `(float)Health / (float)MaxStrength` (Health at +0x6C, Strength at TypeClass+0xA0) |
| `0x005f5d20` | IsYellowCondition | — | Returns 1 if `ConditionRed < ratio <= ConditionYellow` |
| `0x005f5cd0` | IsRedCondition | — | Returns 1 if `ratio <= ConditionRed AND Health > 0` |
| `0x00709A90` | DrawPipScalePips | **+0x450** | 3553 bytes. Cargo/ammo/self-heal/veterancy/group pips. Uses PIPS.SHP + PIPS2.SHP |
| `0x0070A990` | DrawVeterancyPips | **+0x454** | Self-heal/veterancy pip with frame selection |
| `0x0070AA60` | DrawExtraInfo | **+0x458** | Veterancy text labels. References `"D:\ra2mdpost\Techno.CPP"` |
| `0x004DC060` | DrawActionLines | **+0x438** | Called from render loop at `0x6d4750` — NOT DrawHealthBar! |
| `0x004f9a50` | Alliance check | — | Tests if two houses are allied (gates DrawPipScale visibility) |

## Global Data Addresses

| Address | Description |
|---------|-------------|
| `0x00AC1478` | Pointer to loaded PIPBRD.SHP |
| `0x00AC147C` | Pointer to loaded PIPS.SHP |
| `0x00AC1480` | Pointer to loaded PIPS2.SHP |
| `0x00AC1484` | Pointer to loaded TALKBUBL.SHP |
| `0x00887314` | Current drawing surface |
| `0x008871E0` | Pointer to RulesClass instance (ConditionYellow at +0x1700, ConditionRed at +0x1708) |
| `0x008192B8` | Foundation width lookup table (22 int32 entries: 1,2,1,2,2,3,3,3,4,3,1,3,4,1,1,2,2,5,4,3,6,0) |
| `0x00819310` | Foundation height lookup table (22 int32 entries: 1,1,2,2,3,2,3,5,2,3,3,1,3,4,5,6,5,3,4,4,4,0) |
| `0x0089DDB8` | HeightFactor (runtime-initialized, used in Dimension2 for Z = Height * HeightFactor) |
| `0x00843108` | Global flag gating health bar rendering (set by `FUN_0070D180`) |
| `0x008871E0` + `0x30` | RulesClass: self-heal blink timer period (infantry/organic) |
| `0x008871E0` + `0x38` | RulesClass: self-heal blink timer period (mechanical units) |

## Entity Field Offsets (ObjectClass / TechnoClass)

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| `0x6C` | 4 | **Health** | Current HP (int). Used by GetHealthRatio |
| `0x83` | 1 | **IsSelected** | Gates PIPBRD background drawing inside DrawHealthBar |
| `0x3CD` | 1 | **IsSinking** | TechnoClass. When true, DrawExtras returns immediately — no overlays drawn at all |
| `0x431` | 1 | **IsMouseHovering** | When set AND IsSelected=0, DrawExtras calls DrawHealthBar with `bUnk3=true` (pips without PIPBRD background) |
| `0x3D8` | 1 | **PipsDrawForAll** | TechnoTypeClass field. Gates building DrawPipScale rendering |
| `0x3E0` | 4 | **PixelSelectionBracketDelta** | TechnoTypeClass field (dword index 0xF8). Signed int pixels |
| `0x210` | 4 | **DisplayProductionTo** | TechnoClass. Bitfield of houses that have spied this building (gates DrawPipScale) |
| `0x520` | 4 | **Type** (BuildingTypeClass*) | BuildingClass. First field after TechnoClass in BuildingClass |
| `0x157B` | 1 | **CanBeOccupied** | BuildingTypeClass. If true, pips always shown to all players (garrisonable buildings) |
| `0xD97` | 1 | **Organic** | TechnoTypeClass. Determines organic (frame 13) vs mechanical (frame 20) self-heal pip |
| `0x17E6` | 1 | **EnemyHealth** | RulesClass field (`[AudioVisual]`). When set, enemy entity health bars are visible |

## Current Rust Engine Status (updated 2026-03-20)

### Building health bar — IMPLEMENTED, matches gamemd.exe

**Anchor point**: DrawExtras (0x6F5190) calls GetCoords (vtable+0x48) which returns
the **foundation center** for buildings. DrawHealthBar receives this as pLocation.
Our entity `screen_y` is at the NW cell center; the foundation center offset
`7.5*(fw+fh) - 15` cancels exactly with the Dimension2 projection terms, yielding:

```
pip0.X = sx + 3
pip0.Y = sy - 11 - Height * 15
```

Where:
- `sx, sy` = entity screen position (NW cell center, from `lepton_to_screen`)
- `Height` = from art.ini (BuildingTypeClass+0xEF4, read via CCINIClass::INI_Art at 0x887180)
- Default Height = 2 (constructor at 0x45DD90: param_1[0x3BD] = 2)
- Height is read from the **Image= redirect** art section, NOT the type ID section
- `15` = approximate AdjustForZ per height unit (exact for integer heights: 104 * 0.14348 ≈ 14.92, rounds to 15)

**Derivation** (verified against assembly at 0x6F64A0):
```
foundCenter.Y = pLoc.Y + 7.5*(fw+fh) - 15
screen1.Y = 7.5*(fh-fw) - AdjustForZ(Height*104)
numPips = 7.5*fh (integer: (fh*15)/2)

pip0.Y = foundCenter.Y + screen1.Y + 4 - numPips*2
       = [sy + 7.5*(fw+fh) - 15] + [7.5*(fh-fw) - H*15] + 4 - 15*fh
       = sy + 15*fh - 15 + 4 - H*15 - 15*fh
       = sy - 11 - H*15     (all foundation terms cancel!)

pip0.X = foundCenter.X + screen1.X + 3 + numPips*4
       = [sx + 15*(fw-fh)] + [-15*(fw+fh)] + 3 + 30*fh
       = sx + 3              (all foundation terms cancel!)
```

**Canvas centering**: gamemd draws each pip via DrawSHP with flag 0x200, which applies
`-canvas_w/2 + frame_x` and `-canvas_h/2 + frame_y`. The Rust engine now bakes this
adjustment from the PIPS.SHP canvas/frame data at load time via `pip_canvas_adj`.

**Known remaining disparity** (~5px): Different pip frames (0=empty, 1=green, 2=yellow,
4=red) may have slightly different `frame_y` offsets within the PIPS.SHP canvas.
We use frame 0 as the reference for canvas centering. Per-frame adjustment would
fix this but requires per-variant offset tracking.

**Pip step**: (-4, +2) per pip — each pip moves 4px left and 2px down along the NW edge.

**Pip count**: `(foundationHeight * 15) / 2` with integer truncation (matches gamemd).

**Pip fill count**: `ftol(healthRatio * numPips)`, clamped to [1, numPips].

### What's still missing:
1. **`PixelSelectionBracketDelta`** — not parsed, not applied to unit/infantry health bar Y
2. **Building selection brackets** — isometric line brackets (DrawBehind at 0x6F60D0)
3. **Self-heal / cargo / ammo pips** — DrawPipScalePips (vtable+0x450)
4. **Per-frame pip centering** — using frame 0 reference for all variants (~5px error)

### Files:
- `src/app_ui_overlays.rs` — Building pip formula: `sy - 11 - H*15`, `sx + 3`
- `src/render/selection_overlay.rs` — Pip atlas loading, canvas centering adjustment
