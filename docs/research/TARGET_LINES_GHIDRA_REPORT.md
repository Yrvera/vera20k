# Target Lines System — Ghidra Research Report

**Primary address:** `0x004DC060` (TechnoClass::DrawActionLines)
**Confidence:** HIGH (verified from binary, all key functions decompiled)
**Active in YR:** Yes — enabled by default via `[Options] UnitActionLines=yes`

## 1. Overview

The target lines system draws colored lines from selected units to their current
action targets (move destination, attack target). Lines appear for 25 frames (~1.7s
at 15fps) after the player issues a command, then fade.

Only **mobile units** draw lines (Infantry, Unit, Aircraft). Buildings use an empty
stub. Only ONE line per unit — to the final destination, not through intermediate
waypoints. ArchiveTarget (combat) takes priority over NavCom (movement).

Controlled by `[Options] UnitActionLines` (in-game checkbox: "Target Lines").

---

## 2. Class Layout / Key Offsets

### TechnoClass Target Fields

| Byte Offset | Name | Type | Purpose |
|-------------|------|------|---------|
| 0x9C | Location.X | int | Unit world X (leptons) |
| 0xA0 | Location.Y | int | Unit world Y (leptons) |
| 0xA4 | Location.Z | int | Unit world Z (leptons) |
| 0x2B4 | ArchiveTarget | AbstractClass* | Active combat target — draws attack line |
| 0x2BC | CaptureManager | CaptureManagerClass* | Mind control (separate system) |

### FootClass Target Fields (extends TechnoClass)

| Byte Offset | Name | Type | Purpose |
|-------------|------|------|---------|
| 0x588 | NavQueue.vtable | ptr | DynamicVectorClass vtable |
| 0x58C | NavQueue.Items | AbstractClass** | Heap array of queued waypoints |
| 0x590 | NavQueue.Capacity | int | Allocated slots |
| 0x595 | NavQueue.IsAllocated | byte | Heap-allocated flag |
| 0x598 | NavQueue.Count | int | Active entries |
| 0x59C | NavQueue.GrowthIncrement | int | Growth step (init=10) |
| 0x5A4 | NavCom | AbstractClass* | Current nav destination — draws move line |
| 0x5CC | TarCom | AbstractClass* | Commanded combat target |

**Offset derivation:** `param_1` in DrawActionLines is `int*`, so
`param_1[0xad]` = byte offset 0xad × 4 = **0x2B4** (ArchiveTarget),
`param_1[0x169]` = byte offset 0x169 × 4 = **0x5A4** (NavCom),
`param_1[0x166]` = 0x598 (NavQueue.Count),
`param_1[0x163]` = 0x58C (NavQueue.Items).

### Global State

| Address | Type | Name | Purpose |
|---------|------|------|---------|
| 0x00B0EA80 | int | g_ActionLines_StartFrame | Frame when timer started (-1 = inactive) |
| 0x00B0EA88 | int | g_ActionLines_Duration | Duration in frames (default 0x19 = 25) |
| 0x00A8EB7E | byte | UnitActionLines | Option toggle (OptionsClass+0x1E) |
| 0x00843108 | byte | DrawActionLinesFlag | Set equal to UnitActionLines; gates DrawActionLines call |
| 0x0087F6C4 | ptr | ConvertClass* | PALETTE.PAL color lookup |
| 0x0088731C | ptr | g_CompositionSurface | Draw target for action lines |
| 0x00843128 | byte[8] | DashPattern | `{1,1,1,1,1,0,0,0}` = 5-on / 3-off |
| 0x00822540 | byte[8] | RadarDashPattern | `{1,1,1,1,1,0,0,0}` (identical to above) |

---

## 3. Core Logic

### 3.1 TechnoClass::DrawActionLines (`0x004DC060`)

Virtual method at **vtable offset 0x438** (index 270).

```
function DrawActionLines(this, param_2):
    -- Gate: must have at least one target
    if this.ArchiveTarget == NULL and this.NavCom == NULL:
        return

    -- Timer check (param_2 is always 0 in stock YR)
    if param_2 == 0:
        remaining = g_ActionLines_Duration
        if g_ActionLines_StartFrame != -1:
            elapsed = g_CurrentFrameCounter - g_ActionLines_StartFrame
            if elapsed >= g_ActionLines_Duration:
                return   -- timer expired
            remaining = g_ActionLines_Duration - elapsed
        if remaining < 1:
            return

    -- Branch 1: ArchiveTarget line (combat target, takes priority)
    if this.ArchiveTarget != NULL:
        start = this->Get_Fire_Coords(0)           -- vtable+0x300, turret/barrel position
        end   = Resolve_ArchiveTarget_Coords(this)  -- FUN_0070bcb0
        color = PALETTE_ENTRY(8)                     -- bright green
        ActionLines_DrawLine(start, end, color, 0, 0)
        return   -- does NOT fall through to Branch 2

    -- Branch 2: NavCom line (movement target)
    start = this.Location (0x9C, 0xA0, 0xA4)

    if this.NavQueue.Count > 0:
        -- Line to LAST queued waypoint (final destination)
        endpoint_obj = NavQueue.Items[Count - 1]
    else:
        -- Line to current NavCom
        endpoint_obj = NavCom

    end = endpoint_obj->Get_Coords()   -- vtable+0x48

    -- Bridge Z adjustment
    cell = CellAt(end)
    if cell.flags & 0x100:  -- bridge bit
        end.Z = cell.GetGroundHeight() + g_BridgeHeightOffset

    color = PALETTE_ENTRY(3)
    ActionLines_DrawLine(start, end, color, param_2, 0)
```

**Verified at:** `0x004DC060`, 92 lines decompiled. param_1 is `int*`.

### 3.2 Color Extraction

Colors come from **PALETTE.PAL** via `ConvertClass` at `0x0087F6C4`:

```
ConvertClass + 0x04  = bytesPerPixel (1 = 8-bit, 2 = 16-bit)
ConvertClass + 0x174 = pointer to shade-middle of color table
```

| Line Type | Palette Index | Byte Offset (8-bit) | Byte Offset (16-bit) |
|-----------|--------------|--------------------|--------------------|
| ArchiveTarget (combat) | **8** | byte[8] | ushort[8] = offset 0x10 |
| NavCom (movement) | **3** | byte[3] | ushort[3] = offset 0x06 |

The raw palette value is decomposed to R/G/B via display shift/loss globals:
```
R = (pixel >> g_DD_RShift) << g_DD_RLoss
G = (pixel >> g_DD_GShift) << g_DD_GLoss
B = (pixel >> g_DD_BShift) << g_DD_BLoss
```

These RGB bytes are then recomposed for the draw surface's pixel format in
`ActionLines_DrawLine`.

**Note:** The actual RGB values depend on the loaded PALETTE.PAL. The indices
are constant; the visible color varies by palette.

### 3.3 Timer Mechanism

**ActionLines::StartTimer** (`0x0070D150`):
```
g_ActionLines_StartFrame = g_CurrentFrameCounter   -- [0x00B0EA80]
g_ActionLines_Duration   = 0x19                     -- [0x00B0EA88] = 25 frames
```

**ActionLines::ClearTimer** (`0x006F2AB0`):
```
g_ActionLines_StartFrame = g_CurrentFrameCounter
g_ActionLines_Duration   = 0                        -- immediately expired
```

**Timer call sites** (verified via xrefs to `0x0070D150`):

| Address | Function | Trigger |
|---------|----------|---------|
| 0x004ABCF0 | DisplayClass::BandBox_LeftUp | Band-box (drag-select) completes |
| 0x004ABE83 | DisplayClass::BandBox_LeftUp | Left-click on target object |
| 0x004ABFAE | DisplayClass::BandBox_LeftUp | Command dispatched to selected units |
| 0x00731385 | FUN_007311c0 | Hotkey group recall (Ctrl+1..9) |

**Timer preservation** across scene changes: `FUN_00685120` adjusts remaining
duration and resets the start frame to current.

### 3.4 ActionLines_DrawLine (`0x007049C0`)

Low-level line renderer. Called exclusively from `DrawActionLines`.

**Pipeline:**
1. Convert both 3D endpoints to screen pixels via `TacticalClass::CoordsToClient2`
2. Add `g_RadarViewportOffsetY` to Y coordinates (viewport scroll)
3. Draw **two clipped 3x3 endpoint boxes** offset by (-2, -2) pixels from the
   projected endpoints, using `DSurface::FillRect`/surface vtable+0x14 with the
   caller's RGB color
4. Draw **one clipped line** using the actual RGB color:
   - When `param_2 == 0` (normal): **solid** line via Cohen-Sutherland clip →
     `DSurface::DrawLine_Simple` (vtable+0x30)
   - When `param_2 != 0` (forced/animated): **dashed** line via Cohen-Sutherland clip →
     `DSurface::DrawDashedLine` (vtable+0x4C) with dash pattern from `0x00843128`

**Correction 2026-05-21:** `ACTIONLINES_DRAWLINE_007049C0_PIXEL_STYLE_GHIDRA_REPORT.md`
re-verified `0x007049C0` and found no stock selected-unit body-thickening pass.
The selected-unit path draws endpoint boxes plus one final line.

**Dashed animation:** `phase = (0x7FFFFFFF - g_CurrentFrameCounter) % 0xF`
— cycles 0..14 each frame, creating a "marching ants" scroll effect.

**Dash pattern at 0x00843128:** `{1, 1, 1, 1, 1, 0, 0, 0}` = 5 pixels on, 3 pixels off.

**Clipping:** `FUN_007bc2b0` (Cohen-Sutherland algorithm) clips to viewport rect
before the final draw call.

**Draw surface:** `DAT_0088731c` (g_CompositionSurface).

**IMPORTANT: In stock YR, param_2 is ALWAYS 0.** The dashed/animated path is dead
code — the forced-draw codepath exists but is never reached (see §6).

### 3.5 ArchiveTarget Coordinate Resolution (`0x0070bcb0`)

Resolves the endpoint for combat target lines. Special handling for buildings
undergoing chrono-warp locomotor transitions:

```
function Resolve_ArchiveTarget_Coords(this):
    if this.ArchiveTarget == NULL:
        return {0, 0, 0}

    coords = ArchiveTarget->Get_Center_Coords()    -- vtable+0x58

    if ArchiveTarget.WhatAmI() == BUILDING:
        locomotor = ArchiveTarget.Locomotor         -- +0x674
        if locomotor != NULL and locomotor->IsMoving():
            ArchiveTarget->Update_Position()        -- vtable+0x538
            angle = FUN_005f6360(ArchiveTarget)
            turret = this->Get_Turret_Type()        -- vtable+0x3F4
            if turret != NULL and *turret != 0:
                -- Apply sin/cos rotation correction for chrono-warp
                timer_val = RateTimer::Current(FUN_00773070(angle))
                theta = (timer_val - 0x3FFF) * PI_SCALE
                coords.Y += cos(theta)
                coords.X += sin(theta)

    return coords
```

---

## 4. INI Keys

### Direct Target Lines Configuration

| Key | Section | Type | Default | Effect |
|-----|---------|------|---------|--------|
| `UnitActionLines` | [Options] | bool | yes | Master toggle for action line drawing |

Read via `CCINIClass::ReadBool` in `OptionsClass::ReadFromINI` (`0x005FA620`).
Stored at `OptionsClass + 0x1E` = global `0x00A8EB7E`.

### In-Game UI

| Context | Tooltip String | Control ID |
|---------|---------------|------------|
| In-game options | `STT:IGGameOptCBoxTargetLines` (0x00834CFC) | 0x601 |
| Main menu options | `STT:MainOptCBoxTargetLines` (0x008351A4) | 0x601 |

### Related INI Keys (separate systems)

| Key | Section | Purpose |
|-----|---------|---------|
| `MindControlAttackLineFrames` | [CombatDamage] | Duration of mind control link lines (default 20) |
| `UseLineTrail` | [ProjectileType] in art(md).ini | Projectile trail lines (e.g., MEDUSA, DRAGON) |
| `LineTrailColor` | [ProjectileType] in art(md).ini | Trail color RGB |
| `LineTrailColorDecrement` | [ProjectileType] in art(md).ini | Trail fade rate |
| `LineTrailColorOverride` | [AudioVisual] | Global trail color override ("for maps only") |
| `IsLine` | [WeaponType] | Weapon draws as line effect (not related to target lines) |
| `LineMultiplier` | [SuperWeaponType] | Targeting circle line density |

---

## 5. Integration Points

### Render Pipeline Position

Target lines are drawn in **TacticalClass_Draw Pass 2** (param_3 == 2), in a dedicated
TechnoClass iteration loop. The call order within Pass 2:

```
 1-13. [Objects, particles, lasers, bolts, trails, waves...]
14.    Tactical__DrawUnitActionVisuals()   -- brackets, range/sensor circles
15.    FUN_00430ac0()                      -- garrison pips
16.    Tactical__DrawBandBoxRect()         -- band-box selection rect
17-19. [Rally points, mind control, placement — 2nd call]
20-21. [Radar overlays]
22.    *** TechnoClass iteration loop: ***
         For HUMAN player's selected units:
           → DrawActionLines (vtable+0x438) -- THIS IS WHERE TARGET LINES DRAW
           → CaptureManagerClass::DrawLinks -- mind control links
           → Tether/service lines (animated sin/cos line to building)
         For NON-HUMAN FootClass units passing Psychic Sensor detection:
           → DrawRadarActionLines (0x004DC340) -- tactical Psychic Sensor action lines
23-27. [Super weapon circles, PixelFX, floating text]
```

**CORRECTION from prior report:** The existing report attributed DrawActionLines
to `Tactical::DrawUnitActionVisuals` (0x006DBE20). That function actually handles
brackets (vtable+0x130), range circles, and sensor circles. DrawActionLines is called
from the PARENT function `TacticalClass_Draw` in a separate loop at step 22.

### Call Gate (verified from assembly at `0x006D473F`)

For human player units:
```asm
006d473f: CMP byte ptr [0x00843108], 0x0   ; DrawActionLinesFlag
006d4746: JZ  skip
006d4748: CMP byte ptr [piVar6+0x83], 0x0  ; IsSelected
006d474c: JZ  skip
006d474e: MOV ECX, ESI                      ; this = unit
006d4750: CALL [EAX+0x438]                  ; DrawActionLines(0, 0) virtual
```

Both `IsSelected` AND `DAT_00843108` must be nonzero.

### UnitActionLines → DrawActionLinesFlag Connection

Assembly at `0x004E1F3B` (in-game options apply handler):
```asm
004e1f3b: MOV byte ptr [0x00a8eb7e], CL    ; UnitActionLines = checkbox state
004e1f41: CALL 0x0070d180                   ; SetDrawHealthBarsFlag(ECX)
                                             ; → DAT_00843108 = checkbox state
```

`TechnoClass::SetDrawHealthBarsFlag` (`0x0070D180`) simply does:
```
DAT_00843108 = param_1
```

So **DAT_00843108 mirrors the UnitActionLines option**. When unchecked,
DAT_00843108 = 0, and the DrawActionLines call is skipped.

The same pattern occurs in `OptionsClass::ReadFromINI` (end of function at
`0x005FACFD`) — after reading all INI options, it calls SetDrawHealthBarsFlag
to sync the flag.

**Note:** `DrawRadarActionLines` (for Psychic Sensor-detected non-human FootClass
units) does NOT check this flag —
it is a separate Psychic Sensor-detection path gated by `FUN_0043B150`, not by
`UnitActionLines`.

### Vtable Layout

DrawActionLines at vtable+0x438, verified via memory reads:

| Class | Vtable Base | Entry at +0x438 | Implementation |
|-------|-------------|-----------------|----------------|
| AircraftClass | 0x007E22A4 | 0x007E26DC | 0x004DC060 (real) |
| FootClass | 0x007E8C94 | 0x007E90CC | 0x004DC060 (real) |
| InfantryClass | 0x007EB058 | 0x007EB490 | 0x004DC060 (real) |
| UnitClass | 0x007F5C70 | 0x007F60A8 | 0x004DC060 (real) |
| TechnoClass | (base) | — | 0x00459E60 (empty stub) |
| BuildingClass | — | — | 0x00459E60 (empty stub) |

Only xrefs to `0x004DC060` are DATA (vtable entries) — confirms virtual-only dispatch.

---

## 6. DrawRadarActionLines (`0x004DC340`)

Non-virtual function for **Psychic Sensor-detected non-human FootClass units** on
the tactical view. This is not a generic visible-enemy intent overlay.

Same two-branch structure as DrawActionLines (ArchiveTarget priority over NavCom),
with these differences:

| Feature | DrawActionLines | DrawRadarActionLines |
|---------|----------------|---------------------|
| Timer | 25-frame check | **No timer** - draws while psychic-detection eligibility passes |
| Color | PALETTE.PAL indices 8 / 3 | **House color** from HouseClass+0x56F9 (RGB) |
| Dots | **3x3 pixel dots** at both endpoints | **3x3 pixel dots** at both endpoints |
| Main line | Single solid line | Single animated dashed line |
| Animation | `(0x7FFFFFFF - frame) % 0xF` | `timeGetTime() & 0x3FF` (wall clock, ~1s period) |
| Dash pattern | 0x00843128 | 0x00822540 (both are `{1,1,1,1,1,0,0,0}`) |
| Bridge adjust | Yes | Yes |
| Gated by option | Yes (DAT_00843108) | **No** - gated by psychic detection, not UnitActionLines |

**Endpoint dots:** Creates a 3×3 pixel filled rect at each endpoint via
`FUN_0045a130` → `DSurface::FillRect` (vtable+0x14).

**Correction 2026-05-21:** `DRAWRADARACTIONLINES_004DC340_ENEMY_LINES_GHIDRA_REPORT.md`
verified the caller-side eligibility helper `FUN_0043B150`. Retail YR reaches
this path through `[NAPSIS] PsychicDetectionRadius=15`, comparing the enemy
ArchiveTarget/final NavCom endpoint against sensor coverage.

**Dashed line animation phase:** `(int)(-timeGetTime()) >> 5 & 0xF` — shifts by
5 bits (~32ms per phase step), masked to cycle 0..15.

**Intensity modulation:** When `(timeGetTime() & 0x200) == 0`, applies
`FUN_006612c0` with `alpha` computed as a triangle wave: let `t = timeGetTime() & 0x3FF`;
if `(t & 0x100) != 0` then `alpha = (t ^ 0xFF) & 0xFF` (inverted), else `alpha = t & 0xFF`.
This produces a 0→255→0 brightness ramp rather than a simple `& 0xFF` sawtooth.
(corrected 2026-05-29: was `alpha = timeGetTime() & 0xFF`; binary at `0x004DC6AE`–`0x004DC6C6` shows
`AND EAX,0x3FF` / `TEST AH,0x1` / `XOR EAX,0xFF` (conditional) / `AND EAX,0xFF` — OPERATOR_OR_ORDER_DRIFT;
verified via disassemble_function 0x004DC340)

---

## 7. Dead Code: Forced Draw (param_2 != 0)

The forced-draw codepath in DrawActionLines skips the timer check and enables the
animated dashed line. However, param_2 is **ALWAYS 0** in stock YR:

```asm
006d474a: PUSH 0x0       ; param_3 = 0
006d474c: PUSH 0x0       ; param_2 = 0 (not forced)
006d474e: MOV  ECX, ESI
006d4750: CALL [EAX+0x438]
```

Both parameters hardcoded to 0. The forced-draw path is Tiberian Sun legacy.

---

## 8. Related Separate Systems

### Factory rally and planning path lines

**Correction 2026-05-21:** the older shorthand that rally points and planning
mode waypoints use `Tactical::DrawLine3D @ 0x006DBB60` was too broad for the
verified live YR paths.

| System | Verified renderer | Notes |
|---------|------------------|-------|
| Selected factory rally line | `FUN_006DA9D0` | gated by selected local eligible building, rally target at `TechnoClass+0x218`, owner-house RGB, `DAT_00842930` phase |
| Planning/queued waypoint path | `FUN_006DAD60` | draws adjacent `WaypointPathClass` segments, optional loop closure, `MOUSE.SHA` tactical marker |
| `Tactical::DrawLine3D` | `0x006DBB60` | generic Tactical vtable line primitive; not the verified selected factory rally/planning path renderer |

### Mind Control Links (CaptureManagerClass::DrawLinks @ 0x00472160)

Permanent animated pulsing lines from controller to controlled units. Drawn in the
same TechnoClass loop (step 22) but **separate from action lines**. Uses 32-segment
curved line renderer with scrolling phase animation via `timeGetTime()`.

### Tether/Service Lines

Animated line from unit to servicing building (repair depot, etc.). Also in the
TechnoClass loop, uses `timeGetTime()` with sin/cos rotation for animation.

---

## 9. Target Flow Summary

```
Player issues attack command
    ↓
TarCom (0x5CC) = commanded target
    ↓  (unit arrives / processes command)
ArchiveTarget (0x2B4) = resolved active target
    ↓
DrawActionLines reads ArchiveTarget → green line from fire coords to target center


Player issues move command
    ↓
NavCom (0x5A4) = navigation destination
    ↓  (if Shift+Q held, also pushed to NavQueue)
DrawActionLines reads NavCom or NavQueue.Last → line from unit position to endpoint
```

**Waypoint queuing condition** (`FUN_00731BF0`): returns true if either
Planning mode active (`DAT_00b0fe58 != 0`) OR Shift AND Q both held.

---

## 10. Current Rust Implementation Status

**Not implemented.** No target line rendering exists in the codebase.

**Available infrastructure:**
- `GameEntity.movement_target` (MovementTarget with path + final_goal) — maps to NavCom
- `GameEntity.attack_target` (AttackTarget with target entity ID) — maps to ArchiveTarget
- `GameEntity.selected` (bool) — maps to IsSelected
- `lepton_to_screen()` in `src/util/lepton.rs` — coordinate conversion
- `emit_line()` in `src/app_selection_brackets.rs` — Bresenham pixel-stepping line renderer
- Debug path overlay in `src/app_debug_overlays.rs` — reference pattern for path visualization
- SpriteInstance GPU pipeline — used for brackets, health bars, debug overlays

**What needs to be built:**
- 25-frame timer triggered on command dispatch
- Line instance builder reading movement_target/attack_target endpoints
- Color selection (palette index 8 for attack, 3 for move — or hardcoded RGB equivalents)
- Integration into render pipeline (after brackets, as overlay pass)
- Endpoint boxes (3x3 at both ends) plus a single clipped solid line for stock
  selected-unit output; dashed animation exists in the helper but is not reached
  by stock selected-unit calls

## 11. Open Questions

1. **Exact PALETTE.PAL colors:** What RGB values do indices 3 and 8 produce in the
   standard YR palette? Need to read from the actual retail PAL file to determine
   the visible colors. The report confirms the indices but not the final RGB.

2. **NavQueue in our engine:** We don't have shift+Q waypoint queuing yet. When
   implemented, the line endpoint logic needs to read the last queued waypoint
   rather than the immediate NavCom.

3. **Psychic Sensor action-line scope:** `DrawRadarActionLines` is not generic
   enemy intent. It is active for non-human FootClass units whose target/nav
   endpoint passes the local player's PsychicDetectionRadius coverage. The Rust
   implementation should model this with the Psychic Sensor system rather than
   the selected-unit `UnitActionLines` option.

---

## Sources

### Ghidra Functions Decompiled
- `TechnoClass::DrawActionLines` @ 0x004DC060 (92 lines)
- `Tactical::DrawUnitActionVisuals` @ 0x006DBE20 (203 lines)
- `TacticalClass_Draw` @ 0x006D3D10 (451 lines, Pass 2 section)
- `ActionLines_DrawLine` @ 0x007049C0 (178 lines)
- `DrawRadarActionLines` @ 0x004DC340 (165 lines)
- `ActionLines::StartTimer` @ 0x0070D150
- `ActionLines::ClearTimer` @ 0x006F2AB0
- `TechnoClass::SetDrawHealthBarsFlag` @ 0x0070D180
- `OptionsClass::ReadFromINI` @ 0x005FA620 (232 lines, UnitActionLines read)
- `FUN_004e1de0` @ 0x004E1DE0 (in-game options apply handler)
- `FUN_0070bcb0` (ArchiveTarget coord resolver, 61 lines)
- `Tactical::DrawLine3D` @ 0x006DBB60

### Memory Reads
- Dash pattern @ 0x00843128: `{1,1,1,1,1,0,0,0}`
- Radar dash pattern @ 0x00822540: `{1,1,1,1,1,0,0,0}`

### Assembly Verified
- Call site @ 0x006D473F–0x006D4750 (DrawActionLines dispatch)
- Option sync @ 0x004E1F3B–0x004E1F41 (UnitActionLines → DAT_00843108)
- Timer xrefs: 4 callers confirmed (3 in BandBox_LeftUp, 1 in group recall)

### Prior Reports Referenced
- `docs/research/TARGET_LINES_GHIDRA_REPORT.md` (existing, updated)
- `docs/research/TARGET_ACQUISITION_GHIDRA_REPORT.md`
- `docs/research/TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md`
- `docs/research/TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md`

### Corrections to Prior Report
1. **DrawActionLines call site:** Prior report attributed it to `Tactical::DrawUnitActionVisuals`.
   Actually called from `TacticalClass_Draw` in a separate TechnoClass iteration loop (step 22).
2. **Radar dash pattern:** Prior report said patterns at 0x00843128 and 0x00822540 differ.
   Both are identical: `{1,1,1,1,1,0,0,0}`.
3. **DrawRadarActionLines color:** Prior report said PALETTE.PAL. Actually uses
   **house color** from `HouseClass+0x56F9` (3-byte RGB).
