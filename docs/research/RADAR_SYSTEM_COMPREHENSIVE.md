# Radar System — Comprehensive Ghidra Analysis

## Overview

The radar/minimap in YR is managed by `RadarClass`, which sits in the C&C class hierarchy
between `DisplayClass` (parent) and `SidebarClass` (child). It renders a minimap of the
battlefield on the sidebar, shows unit blips, handles activation/deactivation transitions,
spy satellite vision, gap generator jamming, and animated radar events (combat pings).

**Source file:** `D:\ra2mdpost\Radar.CPP` (debug string at `0x00839388`)

---

## 1. RadarClass Object Layout (key fields)

All offsets relative to the RadarClass portion of the object (starts at `this+0x1190`-ish
in the full GScreenClass hierarchy, but the functions use `param_1` as the base).

| Offset | Type | Description |
|--------|------|-------------|
| +0x11E4 | int | Radar chrome draw X position |
| +0x11E8 | int | Radar chrome SHP alternate X |
| +0x11EC | int | Radar chrome draw Y position |
| +0x11F0 | int | Inner radar surface X offset |
| +0x11F4 | int | Inner radar surface Y offset |
| +0x11F8 | int | Inner radar surface width |
| +0x1200 | int | Inner radar surface width (alt) |
| +0x1204 | int | Inner radar surface height (alt) |
| +0x1208 | uint16 | Border/frame color (packed 16-bit) |
| +0x120C–121B | Rect | Dirty rect {x, y, w, h} — redraw tracking |
| +0x121C | Surface* | Primary radar surface (terrain+objects composited) |
| +0x1220 | Surface* | Secondary terrain-only surface (RGB backing) |
| +0x1234 | int | Count of pending dirty terrain cells |
| +0x123C | byte* | Raw RGB terrain buffer (140×140×3 bytes) |
| +0x1248–1257 | Rect | Dirty cell rect for incremental updates |
| +0x1258 | RadarCellTracker* | Hash table tracker (256 buckets) for object blips |
| +0x1260 | void* | Dirty cell list items pointer |
| +0x126C | int | Dirty cell list count |
| +0x1274 | byte* | Processed-cell bitmap (bitfield array) |
| +0x1488 | float | Zoom factor |
| +0x1490 | int | Map X offset (for coord transform) |
| +0x1494 | int | Map width in transformed coords |
| +0x1498 | int | Map diagonal base |
| +0x149C | float | Map left bound (cell X) |
| +0x14A0 | int | Map top bound (cell Y) |
| +0x14A4 | int | Map cell width |
| +0x14A8 | int | Map cell height |
| +0x14AC | int | **Animation state** (state machine, see §3) |
| +0x14B0 | int | **Radar mode** (0=off, 1=active, 2=jammed, 3=movie, 4=transitioning) |
| +0x14B4 | int | Pending radar mode (deferred during transitions) |
| +0x14D8 | bool | **IsTacticalMapAvailable** — true when player has working Radar building |
| +0x14D9 | bool | Needs full redraw flag |
| +0x14DA | bool | Chrome redraw flag |
| +0x14DC–14EB | Rect | Current viewport rect on radar (x, y, w, h) |
| +0x14EC–14FB | Rect | Previous viewport rect (for delta detection) |
| +0x14FC | int | Transition frame counter (animation progress) |
| +0x1500 | int | Transition timer start |
| +0x1508 | int | Transition timer interval (4 frames per step) |

---

## 2. Radar Mode State Machine

**`RadarClass::SetRadarMode` @ `0x00656CB0`**

The radar has 5 modes stored at `+0x14B0`:

| Mode | Name | Description |
|------|------|-------------|
| 0 | Off | Radar deactivated, shows closed frame |
| 1 | Active | Full minimap rendering with unit blips |
| 2 | Jammed | Gap generator is jamming — shows player list + "RADAR JAMMED" |
| 3 | Movie | Transition animation playing (open/close SHP movie) |
| 4 | Transitioning | Intermediate state during mode changes |

**Transition logic:**
- If currently in mode 3 (movie) or 4 (transitioning), the new mode is *deferred* to
  `+0x14B4` and applied after the movie finishes — UNLESS the target is mode 3, or
  the animation state (`+0x14AC`) is 5 (special "Init" state)
- Mode 0→1: calls `ActivateDeactivate(was_available, play_sound)` then sets mode 1
- Mode 1→0: calls `ActivateDeactivate(0, was_active)` then sets mode 0
- Mode →4: sets the mode, animation state to 4, and frame counter to 25 (0x19)

**Animation state** (`+0x14AC`):

| State | Description |
|-------|-------------|
| 0 | Idle/off |
| 1 | Fully active (drawing) |
| 2 | Deactivating (closing animation) |
| 3 | Activating (opening animation) |
| 4 | Transition in progress |
| 5 | Init/load state |

**`RadarClass::ActivateDeactivate` @ `0x00656BE0`**
- `param_2 == 1`: Activating → sets state=3, plays `RadarOn` sound, logs "Radar: ACTIVATING"
- `param_2 == 0`: Deactivating → sets state=2, plays `RadarOff` sound, logs "Radar: DEACTIVING" [sic]

---

## 3. Radar Activation — Building-Driven

**`HouseClass::CheckSuperweaponReady` @ `0x00509140`** (misleading name — this also handles
radar availability for the local player)

This is called periodically and when `HouseClass.RecheckRadar` is set to true. Flow:

1. Only runs for `g_PlayerPtr` (local human player)
2. Checks a recheck timer at offsets `+0x2B0` / `+0x2B8`
3. If `FreeRadar` scenario flag is ON (ScenarioClass `+0x34A4`) → radar is available
4. Otherwise, checks power ratio: `PowerOutput / PowerDrain >= 1.0` (float comparison)
5. Iterates through the house's building list looking for the **first** building where:
   - Building pointer is non-null
   - `BuildingClass.IsAlive` is true (offset `+0x198` area)
   - `BuildingTypeClass.Radar == true` (offset `+0x16A4`)
   - Building is not in limbo (`+0x81` == 0)
   - Building is active (`+0x1D` area)
   - Game mode conditions are met
   - Building is not being chronoshifted (mission != 0x13)
6. If a qualifying building is found and operational → radar available (`cVar2 = 1`)
7. Compares with `RadarClass::IsTacticalMapAvailable()` (reads `+0x14D8`)
8. If changed, calls the update function which sets `+0x14D8` and triggers
   `RadarClass::ActivateDeactivate` or `RadarClass::SetRadarMode`

### Buildings with `Radar=yes`

| ID | Name | Side |
|----|------|------|
| GAAIRC | Air Force Command HQ | Allied (generic) |
| NARADR | Radar Tower | Soviet |
| AMRADR | American Airforce Command HQ | American (uses GAAIRC image) |
| GASPYSAT | SpySat Uplink | Allied |
| NAPSIS | Psychic Sensor | Yuri (YR only) |

**`PrerequisiteRadar`**: GAAIRC, NARADR, AMRADR (base); + NAPSIS in rulesmd.ini
**`BuildRadar`** (AI): Same list as PrerequisiteRadar

### AMRADR Specifics

AMRADR is the **American country-specific variant** of the Air Force Command HQ:
- `RequiredHouses=Americans` — only buildable by the American sub-faction
- `Image=GAAIRC` — shares the same visual as the generic allied GAAIRC
- `Radar=yes` — provides radar like all other radar buildings
- `SuperWeapon=AmericanParaDropSpecial` — provides the American Paradrop superweapon
- `Helipad=yes`, `NumberOfDocks=4` — also serves as aircraft dock
- `Factory=AircraftType` — produces aircraft
- Cost: 1000, Power: -50, TechLevel: 3
- In `PrerequisiteRadar` list — owning it satisfies radar building requirements

---

## 4. INI Key Reference

### BuildingTypeClass fields (per-building)

| Key | Offset | Type | Default | Description |
|-----|--------|------|---------|-------------|
| `Radar` | +0x16A4 | bool | false | Building provides radar when owned |
| `SpySat` | +0x16A5 | bool | false | Building provides spy satellite vision |
| `GapGenerator` | +0x16C7 | bool | false | Building jams enemy radar in radius |
| `GapRadiusInCells` | +0x1707 | int | 0 | Radius of gap effect in cells |
| `SuperGapRadiusInCells` | — | int | 0 | Extended gap radius |

### TechnoTypeClass fields (per-unit-type)

| Key | Offset | Source | Description |
|-----|--------|--------|-------------|
| `RadarInvisible` | +0x22F | ObjectTypeClass::ReadINI @ `0x005F946E` | Unit does NOT show on radar |
| `RadarVisible` | +0x232 | TechnoTypeClass::ReadINI @ `0x00714AB8` | Unit visible on radar even under shroud |

Checked in `RadarClass::RenderCellPixel`:
```
if (RadarInvisible == true) → skip UNLESS target is allied with player
if (RadarVisible == true) → show UNLESS unit has cloakable flag active AND owner is enemy
```

### RulesClass globals

| Key | Description |
|-----|-------------|
| `RadarCombatFlashTime=49` | Frames for combat flash on radar (must be odd multiple of FlashFrameTime) |
| `RadarEventSuppressionDistances=8,8,8,8,8,6` | Min cell distance between events of each type |
| `RadarEventVisibilityDurations=200,200,200,200,200,200` | Frames event stays visible |
| `RadarEventDurations=400,400,400,400,400,400` | Total event lifetime in frames |
| `RadarEventMinRadius=8` | Minimum radius for event diamond |
| `RadarEventSpeed=1.2` | Speed at which event diamond shrinks |
| `RadarEventRotationSpeed=.05` | Rotation speed of event diamond |
| `RadarEventColorSpeed=.1` | Color interpolation speed |
| `LocalRadarColor=0,255,0` | Radar color for local player (overrides house color) |
| `RadarOn` / `RadarOff` | Sound event names |
| `AllyReveal=yes` | Allies share radar vision |
| `FreeRadar` | Scenario flag — bypasses building requirement |

---

## 5. Cell Color Rendering Pipeline

**`CellClass::GetRadarColor` @ `0x0047C060`**

Returns RGB color for a cell. Priority order:

1. **Building occupier** (RTTI 0x24) → fixed color `(0xC8, 0xC8, 0xA0)` — grayish tan
2. **Overlay with bit 8 set** in cell flags (`+0x140 & 0x100`) → `OverlayClass::GetRadarColor`
3. **Tiberium/Ore overlay** (`RadarColor` INI key on the overlay type) → `GetTiberiumRadarColor`
4. **Wall overlay** → fixed color `(0xAA, 0xAA, 0x82)` — grayish wall color
5. **Terrain** → looks up IsometricTileType palette entry, applies theater brightness,
   halves RGB for the "dark" variant. Falls back to `(0x3C, 0x3C, 0x3C)` if no tile found.

**`RadarClass::RenderCellPixel` @ `0x00655C50`**

Per-cell rendering with shroud/fog handling:

1. **Bounds check** — cell must be within the radar surface rect
2. **Shroud check** (`IsShrouded`) — if cell was never explored:
   - Write pixel as black (0x0000)
3. **Fog check** (`IsFogged`) — if cell was explored but not currently visible:
   - Read terrain color from the secondary (terrain-only) surface
   - **Half-brightness**: each R,G,B channel is right-shifted by 1 then repacked
   - This produces the classic "dimmed previously-seen area" effect
4. **Normal rendering** — if cell is fully visible:
   - Check the RadarCellTracker hash bucket for objects at this cell
   - For each tracked object:
     - Skip if shrouded AND player is not human
     - Skip if `RadarInvisible` AND not allied
     - Skip if `RadarVisible` AND cloaked AND enemy-owned
   - If object found: draw owner's house color (from `+0x56F9` RGB in HouseClass, packed to 16-bit)
   - **Combat flash**: if object has an active flash timer (`+0x5F` field), toggles between
     normal color and inverted color (`~color`) every N frames based on `RadarCombatFlashTime`
   - If no object: copy terrain color from secondary surface

### Object-to-Radar Registration

**`TechnoClass::RegisterOnRadar` @ `0x0070CC90`**
- Adds a single object at its cell position (`+0x208`, `+0x20C`) to the tracker
- Sets `+0x423` flag (IsRegisteredOnRadar)

**`BuildingClass::RegisterOnRadar` @ `0x00456580`**
- Uses `GetBucketIndex` on the building's foundation
- Iterates through all foundation cells, adding each to the tracker
- Multi-cell buildings occupy multiple radar pixels

**`RadarClass::AddObjectToTracker` @ `0x00655560`**
- Hash bucket = `(cellX + cellY * -5) & 0xFF` — 256 buckets
- Checks for duplicates before inserting
- Player-owned objects are inserted at the FRONT of the bucket (priority rendering)
- Enemy objects are appended normally
- Marks cell dirty and sets full-redraw flag

---

## 6. SpySat System

**`DrawSpySatelliteVision` @ `0x00431700`**

SpySat buildings (`SpySat=yes`) reveal the entire map for allied players:

- Checks `g_CurrentFrameCounter % refreshInterval < g_SpySatRefreshFrameCount + 1`
- Iterates through an 8×3 array (likely player slots × some grouping)
- For each valid entry that is allied with the local player:
  - Calls `DrawOneSpySatellite` which renders full-map vision on the radar

**`HasSpySatelliteUpdate` @ `0x00431800`**
- Same iteration pattern — returns true if any allied house has an active SpySat
- Used by `RadarClass::Update` to trigger radar redraws when SpySat state changes

**INI sounds:** `SpySatActivationSound`, `SpySatDeactivationSound`

---

## 7. Gap Generator System

**`BuildingClass::UpdateGapGenerator_Tick` @ `0x00454DB0`**

Gap generators create a circular area that jams enemy radar:

### State machine (`BuildingClass +0x220` = gap state):
| State | Description |
|-------|-------------|
| 0 | Inactive |
| 1 | Expanding (radius growing from 0 to GapRadiusInCells) |
| 2 | Fully active (maintaining gap circle) |
| 3 | Contracting (radius shrinking back to 0) |

### Expansion/contraction:
- `+0x6ED` = current gap radius (byte, counts 0→15 during expand, 15→0 during contract)
- Visual update triggers at radius values 1, 6, 11 (state 1) and 0, 5, 10 (state 3)
- At radius 15: transitions from state 1→2 (fully active)
- At radius 0: transitions from state 3→0 (inactive)
- Uses a particle system for the visual gap dome effect (`+0xC3` = ParticleSystem ptr)

### Radar jamming:
- **`RadarClass::IsRadarJammed` @ `0x00656E50`**:
  Returns true when `mode == 2` (jammed) AND `animState == 1` (fully active)
- Jammed mode shows `DrawJammedMode` instead of the normal minimap

### Gap overlap detection:
- When a gap generator deactivates, it checks all other gap generators:
  - If another gap generator's center is within `(radius + 2)²` cells distance
  - It tells the overlapping generator to begin contracting too
  - This creates the chain-collapse effect when destroying gap generators

---

## 8. Jammed Mode Display

**`RadarClass::DrawJammedMode` @ `0x00653FA0`**

When radar is jammed (mode=2, active state):

1. Draws the radar background chrome (frame 0x20 of BKGDLG.SHP)
2. Draws "RADAR JAMMED" text (from string table entries 0x4F4, 0x4F6)
3. For each active house:
   - If the house has `+0x1A5` flag set (is a player) and is not the local player:
     - Draws their name in their house color
     - Calculates a "score" by summing 20 values from the house's stat arrays
     - Shows the score right-aligned
   - Local player is drawn with the generic color scheme
4. A colored line separates the player list from the header

---

## 9. Radar Event System

Radar events are the animated diamond/ring effects that appear on the minimap when
combat or other notable events occur.

### Event Types (from `DrawRadarEvent` switch at `0x00660050`):

| Type | Color |
|------|-------|
| 0, 3, 4 | White (0xFF, 0xFF, 0xFF) |
| 1, 2, 11, 12 | Yellow (0xFF, 0xFF, 0x00) |
| 5 | Cyan (0x00, 0xFF, 0xFF) |
| Default | Red (0xFF, 0x00, 0x00) |

### RadarEventClass object layout (size 0x40 = 64 bytes):

| Offset | Type | Description |
|--------|------|-------------|
| +0x00 | int | Event type |
| +0x04 | float | Rotation angle (starts at π/4) |
| +0x08 | int[2] | Radar pixel position (x, y) |
| +0x0C | float | Max radius for this event |
| +0x10 | float | Rotation speed (from RulesClass) |
| +0x14 | float | Shrink speed (from RulesClass) |
| +0x18 | float | Color lerp factor |
| +0x1C | float | Color speed (from RulesClass) |
| +0x20 | CellStruct | Source cell coordinate |
| +0x24 | int | Creation frame (g_CurrentFrameCounter) |
| +0x28 | int[2] | Timer fields |
| +0x30 | int | Visibility duration |
| +0x34 | int | Event duration |
| +0x38 | int | Timer/tick fields |
| +0x3C | byte | Is alive flag |
| +0x3D | byte | Is in "expand" phase |

**`CreateRadarEvent` @ `0x0065FA70`**
- Suppression check: scans existing events of the same type — if another event of the
  same type exists within `RadarEventSuppressionDistances[type]` cells, suppresses (returns 0)
- Distance calculated as euclidean: `sqrt(dx² + dy²)`
- Allocates 0x40 bytes, calls `InitRadarEvent`

**`InitRadarEvent` @ `0x0065FB80`**
- Sets initial rotation to π/4 (0x3F490FDB)
- Copies `RadarEventSpeed` and `RadarEventColorSpeed` from RulesClass
- Converts cell to radar pixel via `RadarClass::CellToRadarPixel`
- Calculates max radius from distance to nearest radar surface edge
- Uses an 8-slot ring buffer for cell positions (`g_RadarEventCellRing`)
- Adds to a DynamicVector of RadarEventClass pointers

**`TickRadarEvent` @ `0x0065FE00`**
- If not in expand phase: checks duration timer → marks dead when expired
- Radius shrinks by `RadarEventSpeed` per tick toward `RadarEventMinRadius`
- During expand phase: rotation += rotation_speed; speed decelerates by factor
  until radius reaches minimum, then switches to visibility phase
- Color lerp oscillates between 0.0 and 1.0 using the color speed

**`DrawRadarEvent` @ `0x00660050`**
- Computes 4 corner points of a rotated diamond at the event's position
- Line color interpolates between bright and dim variants based on lerp factor
- Draws 4 lines forming the diamond shape on the radar surface
- Also draws corresponding viewport indicator lines

---

## 10. Radar Update Loop

**`RadarClass::Update` @ `0x00656EC0`**

Called from `RadarClass::Draw` every frame. Core update logic:

1. **Viewport tracking**: computes the current visible area as a radar-space rect
   (`+0x14DC` through `+0x14EB`), clamped to map bounds
2. **Redraw trigger conditions** (ANY of these forces a redraw):
   - Full redraw flag set (`+0x14D9`)
   - Viewport position changed from previous frame
   - Viewport size changed
   - Objects moved (checked via `ObjectsMovedCheck()`)
   - Dirty terrain cells pending (`+0x126C > 0` or `+0x1234 > 0`)
   - SpySat state changed (`HasSpySatelliteUpdate()`)
3. **Incremental rendering**:
   - Clear background
   - If viewport changed: marks old viewport border cells as dirty
   - Process dirty rect: iterate cells in the dirty region, call `RenderCellPixel` for each
   - Process individual dirty cells from the dirty list
   - Clear the dirty list
   - Call `TickAndDrawRadarEvents()`
   - Call `DrawSpySatelliteVision()`
4. **Surface compositing** (when both surfaces ready):
   - Blit rendered radar to sidebar surface
   - Draw viewport rectangle outline on sidebar surface
5. **Bitmap reset**: zeros out the processed-cell bitmap for next frame

---

## 11. Radar Draw (Main Entry Point)

**`RadarClass::Draw` @ `0x00653100`**

Called every frame as part of the sidebar rendering pipeline.

### Flow by animation state (`+0x14AC`):

**State 0 (idle/off):**
- If needs redraw: draws closed radar frame (BKGDLG.SHP frame 0) and updates dirty rect

**State 1 (active) when not mode 1/4:**
- Mode 2 (jammed): draws background, calls `DrawJammedMode()`
- Mode 3 (movie): calls `PerFrameMovieUpdate()`
- Default: draws closed frame

**State 1 (active) with mode 1 or 4:**
- Falls through to the normal update path → `RadarClass::Update()`

**State 2 (deactivating):**
- Timer-based animation: decrements `+0x14FC` frame counter
- When counter < 1: sets state=0, releases sound
- Draws the corresponding SHP frame from BKGDLG.SHP

**State 3 (activating):**
- Timer-based animation: increments `+0x14FC` frame counter
- When counter >= 0x20 (32 frames): sets state=1, triggers full redraw
- Draws the corresponding SHP frame

**After state handling:**
- Composites beacon overlay and sidebar gadgets if chrome was redrawn
- Merges dirty rects from multiple overlay sources

---

## 12. Map-to-Radar Coordinate System

**`RadarClass::ComputeRadarMapBounds` @ `0x00654490`**

Transforms the isometric map into radar-space coordinates:

- Iterates all playfield cells
- Computes the diamond-shaped bounds of the playable area
- Stores bounds at: `+0x149C` (left), `+0x14A0` (top), `+0x14A4` (width), `+0x14A8` (height)
- The diagonal base (`+0x1498`) accounts for the isometric rotation

**`RadarClass::CellToRadarPixel` @ `0x006550C0`**
- Converts a cell coordinate to a pixel position on the radar surface
- Used by radar events and object tracking

**`RadarClass::FillTerrainColors` @ `0x00654EA0`**
- Iterates all cells, calls `CellClass::GetRadarColor` for each
- Writes RGB triplets into the raw terrain buffer (`+0x123C`)
- Handles edge cells (left/right border of diamond) specially — uses the
  "dark" variant of the color for one edge and "light" for the other

---

## 13. Surface Architecture

Three surfaces are involved:

1. **Raw RGB buffer** (`+0x123C`): 140×140×3 bytes of terrain colors
2. **Terrain surface** (`+0x1220`): DirectDraw surface with terrain rendered at current zoom
3. **Primary surface** (`+0x121C`): Composited terrain + object blips, ready for blit to sidebar

The terrain surface is rebuilt by `RadarClass::RebuildRadarSurfaces` when zoom changes.
Object blips are drawn on top of the primary surface by `RenderCellPixel`.
The primary surface is blitted to `g_SidebarSurface` during `Update`.

---

## 14. Confidence Assessment

| Finding | Confidence | Source |
|---------|-----------|--------|
| RadarClass field layout | ~85% | Decompilation + existing docs cross-referenced |
| State machine (modes + anim states) | ~90% | Direct decompilation of SetRadarMode, ActivateDeactivate, Draw |
| Building-driven activation flow | ~85% | HouseClass::CheckSuperweaponReady decompilation |
| BuildingTypeClass.Radar at +0x16A4 | **Verified** | Direct ReadBool call with "Radar" string |
| BuildingTypeClass.SpySat at +0x16A5 | **Verified** | Direct ReadBool call with "SpySat" string |
| BuildingTypeClass.GapGenerator at +0x16C7 | ~80% | Referenced in UpdateGapGenerator_Tick |
| RadarEvent structure | ~80% | Decompilation of Init/Tick/Draw + partial field identification |
| Fog dimming = right-shift-by-1 | **Verified** | Assembly-level (also in existing RADAR_MINIMAP_DEEP_DIVE.md) |
| Gap generator state machine | ~75% | Decompilation, some fields uncertain |
| Spy satellite drawing | ~80% | Decompilation of DrawSpySatelliteVision |
| RenderCellPixel logic | ~85% | Full decompilation with shroud/fog/object paths traced |

### TS Legacy Warning

- The fog-of-war path in `RenderCellPixel` (half-brightness dimming) is only active when
  `g_hWnd != 0` AND `IsFogged` returns true. In standard YR skirmish, fog is disabled by
  default. The code IS reachable but only when fog is explicitly enabled.
- The `FreeRadar` scenario flag exists and works, but is not set in any standard YR maps.
