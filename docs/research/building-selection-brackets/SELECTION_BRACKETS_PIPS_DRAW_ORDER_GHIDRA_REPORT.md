---
title: Selection Brackets, Pips & Overlay Draw Order (Ghidra Research Report)
date: 2026-04-22
---

# Selection Brackets, Pips & Overlay Draw Order — Ghidra Research Report

**Addresses (primary):**
- `TechnoClass::DrawExtras` @ `0x006F5190` (vtable+0x110) — master overlay orchestrator
- `TechnoClass::DrawBehind` @ `0x006F60D0` (vtable+0x10C) — back bracket edges
- `TechnoClass::DrawHealthBar` @ `0x006F64A0` (vtable+0x44C) — pip bars for all types
- `TechnoClass::DrawPipScalePips` @ `0x00709A90` (vtable+0x450) — cargo/ammo/tiberium/self-heal/group#
- `TechnoClass::DrawVeterancyPips` @ `0x0070A990` (vtable+0x454) — rank chevron
- `TechnoClass::DrawExtraInfo` @ `0x0070AA60` (vtable+0x458) — text label near building
- `TechnoClass::DrawBracketCorner` @ `0x006F5EF0` — 25% stub helper
- `Tactical::ObjectRenderingLoop` @ `0x006D8DB0` — per-layer two-pass main dispatcher
- `Tactical::DrawUnitActionVisuals` @ `0x006DBE20` — sensor-radius overlay (NOT brackets)
- `Garrison_DrawOccupantPips` @ `0x00430AC0` → `0x00430250` — garrison occupant indicators
- `TechnoClass::DrawRadarActionLines` @ `0x004DC340` — Psychic Sensor-detected enemy action lines
- `TechnoClass::IsDisguised_Getter` @ `0x0041C020` (vtable+0xC8) — veterancy visibility gate
- `TechnoClass::Select` @ `0x006FBFA0` — selection entry (no animation trigger)
- Pip SHP pointers: `PIPBRD.SHP` @ `0x00AC1478`, `PIPS.SHP` @ `0x00AC147C`,
  `PIPS2.SHP` @ `0x00AC1480`, `TALKBUBL.SHP` @ `0x00AC1484`

**Confidence:** HIGH for vtable slot mapping (verified via direct memory reads at
six independent vtables), DrawExtras intra-phase order, DrawPipScalePips pip frame
catalog, the health-vs-armor pip color question, the select-moment animation
question, and the FUN_006dbe20 / FUN_00430ac0 identities. MEDIUM for the exact
semantics of TypeClass+0x238 that gates vtable+0x130 (an empty stub on base).

**Active in YR:** Yes — all systems verified live in YR skirmish.

**Relationship to prior reports:** Extends
- `ra2-rust-game-docs/building-selection-brackets/SELECTION_BRACKETS_GHIDRA_REPORT.md` (building 3D bracket topology)
- `ra2-rust-game-docs/HEALTH_BAR_POSITIONING.md` (pip offsets per class, canvas centering)
- `ra2-rust-game-docs/TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md` (3-pass frame structure)
- `ra2-rust-game-docs/VETERANCY_SYSTEM_GHIDRA_REPORT.md` (rank thresholds, Rookie gate)

This report fills three gaps those reports left open:
1. **Exact intra-DrawExtras order** — prior docs listed 6 steps; the real order is 9
   steps and veterancy pips run EARLIER than previously reported (before brackets).
2. **vtable slot resolution** — prior docs conflicted on whether DrawVeterancyPips
   is at +0x118 (stub chain) or +0x454 (the actual implementation). Resolved here:
   both exist, +0x454 is the one DrawExtras actually calls.
3. **The Pass-2-Step-14 mystery** — `FUN_006dbe20` is NOT a second bracket pass; it
   draws sensor/radial-indicator circles for selected buildings with
   `SensorsSight > 0` and for special units flagged `TypeClass+0x238`. Brackets and
   pips never double-draw.

It also answers four user-facing parity questions:
- "Does the original have a select-moment bracket animation?" — **No.** Static.
- "Are pip colors driven by armor type?" — **No.** 100% health-driven.
- "How do brackets z-sort against the main sprite?" — Back edges behind, front
  edges + pips on top via the two-pass render loop (Section 4).
- "What about build-queue dots?" — Those are sidebar chrome, drawn in a separate
  GadgetClass surface between tactical Pass 1 and Pass 2; out of scope here, see
  `docs/GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md` and
  `docs/SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md`.

---

## 1. Overview

Selection-related rendering spans **three nested dispatch levels**:

1. **Frame-level** (`RenderFrame_main` @ `0x004F4480`): calls `TacticalClass::Draw`
   three times. Pass 2 is where objects and their overlays render.
2. **Object-level** (`Tactical::ObjectRenderingLoop` @ `0x006D8DB0`, Pass 2 Step 8):
   per-object two-loop structure. Loop 1 draws all sprites; Loop 2 draws all
   extras (`vtable+0x110` = `DrawExtras`) on top.
3. **Per-object overlay** (`TechnoClass::DrawExtras` @ `0x006F5190`): orchestrates
   the 9-step overlay sequence — bomb clock, wrench, veterancy, brackets, pips,
   health, hover health, talkbubble.

Three **additional** selection-related passes run AFTER the main two-loop object
render (also part of Pass 2):

- Step 14 `Tactical::DrawUnitActionVisuals` — sensor/psychic/gap radii around
  selected buildings + unit radial-indicator hook
- Step 15 `Garrison_DrawOccupantPips` — 8×3 cell grid around garrison buildings
  drawing occupant icons
- Step 22 TechnoClass loop — `DrawPipScalePips` (re-invoke for enemy-detected) +
  `CaptureManager::DrawLinks` + `DrawRadarActionLines` (Psychic Sensor-detected
  enemy action lines)

**Key Z-order invariant:** all sprites in a layer draw before any overlay in that
same layer. The object-rendering loop is explicitly two-pass. So
"later-drawn sibling is on top" works at the **class of overlay** level, not
per-object: brackets are never below another unit's sprite unless that sprite is
in a higher display layer.

---

## 2. Vtable slot catalog (TechnoClass base)

Verified by reading vtable memory directly at six known TechnoClass-derived
vtable xrefs (`0x007E23B0`, `0x007E3FC8`, `0x007E8DA0`, `0x007EB164`, `0x007F4A6C`,
`0x007F5D7C`) and walking slots backwards/forwards from the DrawBehind anchor.

### Draw-related slots

| Slot | Address (base) | Function | Notes |
|---|---|---|---|
| `+0xC8` | `0x0041C020` | `IsDisguised_Getter` | Reads byte at `this+0x1D8`. Gate for `DrawVeterancyPips` visibility — disguised units hide their chevron. |
| `+0x104` | `0x005F4B10` | `DrawIt` (base) | Main sprite blit — overridden by each subclass (`BuildingClass::DrawIt`, `UnitClass::DrawIt`, etc.). |
| `+0x108` | `0x005F5B90` | (render-related helper) | Not called from the overlay path. |
| `+0x10C` | `0x006F60D0` | **`DrawBehind`** | Back bracket edges for buildings. Called in Loop 1 before DrawIt so edges render behind the sprite. |
| `+0x110` | `0x006F5190` | **`DrawExtras`** | Master overlay orchestrator (9 steps). Called in Loop 2 after all sprites. |
| `+0x114` | `0x004DB250` | (empty stub) | Return. Base only. Some subclass override chain destination. |
| `+0x118` | `0x005F65D0` | `DrawVeterancyPips` (stub chain) | TechnoClass stub that **chains to vtable+0x114**. Not called from DrawExtras — the actual call goes via `+0x454`. |
| `+0x11C` | `0x006F4A40` | (unidentified draw helper) | |
| `+0x120` | `0x0070ADC0` | (unidentified) | |
| `+0x124` | `0x004D3780` | `TechnoClass::DoCloak` | State-change call, not per-frame draw. |
| `+0x130` | `0x0041BE80` | **empty stub** | Called from `DrawUnitActionVisuals` when `TypeClass+0x238 != 0`. Almost no concrete class overrides it in stock YR — probably `ShowsSensorRange` override hook; **safe to treat as no-op for parity.** |
| `+0x438` | `0x004DC060` | `DrawActionLines` | Allied unit action lines (the yellow target line). Separate system — see `TARGET_LINES_GHIDRA_REPORT.md`. |
| `+0x448` | `0x006F60C0` | **empty stub** | Return. Called from DrawExtras for allied/EnemyHealth-gated entities. Base stub — no subclass override active in YR confirmed. Appears to be a disabled "DrawAllianceIcon" hook. |
| `+0x44C` | `0x006F64A0` | **`DrawHealthBar`** | Pip bars for every class. WhatAmI branches inside: buildings → NW-edge isometric pips; infantry → 8 pips w/ PIPBRD frame 1; units/aircraft → 17 pips w/ PIPBRD frame 0. |
| `+0x450` | `0x00709A90` | **`DrawPipScalePips`** | Cargo/ammo/tiberium/occupant/self-heal/group#/veterancy-text. 3553-byte function. |
| `+0x454` | `0x0070A990` | **`DrawVeterancyPips`** | Rank chevron — one SHP frame from PIPS.SHP. |
| `+0x458` | `0x0070AA60` | `DrawExtraInfo` | Garrison occupant-count text label above garrisoned buildings (rendered in house-color RGB, not palette). |

**Slot conflict resolved:** earlier reports disagreed whether
`DrawVeterancyPips` lives at `+0x118` or `+0x454`. Both slots exist. `+0x118` is a
tiny chain stub (`jmp vtable+0x114`) never invoked by the overlay path. The
single authoritative call site is `DrawExtras` → `vtable+0x454` → `0x0070A990`.
The `0x005F65D0` function at `+0x118` is effectively dead code in standard YR.

---

## 3. `TechnoClass::DrawExtras` — the 9-step overlay sequence

Verified by decompiling `0x006F5190` in full. The sequence runs every frame for
every visible techno (once per object in the second loop of
`ObjectRenderingLoop`). Each step is skipped if its guard fails; the steps do
NOT chain / early-exit each other.

### 3.0 Entry guard

```
if (this.IsSinking (+0x3CD) != 0) return;   // no overlays while sinking
```

### 3.1 Ivan bomb (BOMBCURS.SHP)

```
if ((byte)this[+0x68] != 0 && (ptr)this[+0x38] != NULL) {
    cell = Get_Current_Cell()
    if !Cell_Is_Hidden(cell):
        coords = this.GetCoords()
        screen = Tactical::CoordsToClient2(coords)
        frame = IvanBomb::GetClockFrame(this)   // 13-frame clock
        CC_Draw_Shape(g_RulesClass+0xFE0,       // BOMBCURS.SHP ptr
                      frame, screen, pBounds,
                      zOrder=0xE00, ...)
}
```

- Z-order **`0xE00`** — above pips but below health bar text.
- Field `+0x38` is the `AttachedBomb*`. Field `+0x68` (byte) is a companion flag.
- Uses `CoordsToClient2` (not CoordsToClient) — bounds-culling variant.

### 3.2 Deploy-ready wrench (WRENCH.SHP)

```
if (this.WhatAmI() == 6 && (byte)this[+0x6E8] != 0) {     // Building + IsReady-flag
    cell = Get_Current_Cell()
    if !Cell_Is_Hidden(cell):
        period = max(2, roundToInt(FUN_005FB2E0() / 4))    // animation period
        frame  = (g_CurrentFrameCounter % period) * 6 / (period - 1)
        CC_Draw_Shape(g_WRENCH_SHP, frame, screen, pBounds,
                      zOrder=0xE00, ...)
}
```

- **6-frame animation** driven by the global frame counter.
- Period is derived from `FUN_005FB2E0()` (an integer returning how many ticks
  per phase), divided by 4 and floored to at least 2. Effective period is
  typically ~13-15 ticks at standard game speed, so the full wrench cycles
  roughly once per second.
- Buildings only (RTTI = 6). The `+0x6E8` flag is set when a factory has just
  finished producing something and awaits placement.

### 3.3 Veterancy pips (vtable+0x454)

```
if (!this.IsDisguised() /* vtable+0xC8, reads +0x1D8 */) {
    if (this.GetVisualState() /* vtable+0x68 */ != 5) {
        this.DrawVeterancyPips(pLocation, pBounds)    // vtable+0x454
    }
}
```

**This step runs BEFORE selection brackets** — the prior HEALTH_BAR_POSITIONING
report listed it after brackets, which is wrong. Veterancy chevrons render for
every visible non-disguised unit regardless of selection state, so chevrons are
always present; selection adds bracket+pip ON TOP of them.

The gate at `vtable+0x68` (GetVisualState) being `!= 5` blocks drawing when the
object is in "invisible under fog" or similar suppressed state.

**Inside `DrawVeterancyPips` (`0x0070A990`):**

```
veterancy = this.GetVeterancyStruct()
frame     = -1
if IsVeteran(veterancy): frame = 0x0E        // 14 — single chevron
if IsElite(veterancy):   frame = 0x0F        // 15 — double chevron / star
if IsRookie(veterancy):  frame = 0x13        // 19 — rookie marker (red)

if frame == -1: return                       // rookie+elite-false but also veteran-false: no rank

offset_y = pLocation.y + 2
offset_x = pLocation.x + 5
if WhatAmI() != 0xF /* not infantry */:
    offset_x += 5     // vehicles/aircraft: pLoc + (10, +6)
    offset_y += 4

CC_Draw_Shape(g_PIPS_SHP, frame, (offset_x, offset_y),
              pBounds, zOrder=0xE00,
              zAdjust=0xFFFFFFFE /* -2 */, ...)
```

**PIPS.SHP frame map (verified from DrawVeterancyPips + DrawHealthBar):**

| Frame | Semantic | Used by |
|---|---|---|
| `0` (0) | Empty building pip (unfilled) | DrawHealthBar building path |
| `1` | Green building pip | DrawHealthBar building path |
| `2` | Yellow building pip | DrawHealthBar building path |
| `4` | Red building pip | DrawHealthBar building path |
| `5` | Ore pip (tiberium storage) | DrawPipScalePips `PipScale=Tiberium` |
| `6..12` | Occupant slot colors (empty gray + 6 house colors) | `Garrison_DrawOccupantPips` (FUN_00430250) |
| `13` (0xD) | **Organic self-heal** pip | DrawPipScalePips (infantry w/ `Organic=yes`) |
| `14` (0xE) | **Veteran chevron** | DrawVeterancyPips |
| `15` (0xF) | **Elite chevron** | DrawVeterancyPips |
| `16` (0x10) | Green unit/infantry pip | DrawHealthBar non-building path |
| `17` (0x11) | Yellow unit/infantry pip | DrawHealthBar non-building path |
| `18` (0x12) | Red unit/infantry pip | DrawHealthBar non-building path |
| `19` (0x13) | **Rookie marker** | DrawVeterancyPips |
| `20` (0x14) | **Mechanical self-heal** pip | DrawPipScalePips (units, !Organic) |

### 3.4 Selection brackets (buildings only)

```
if (this.IsSelected (+0x83) != 0) {
    if (this.WhatAmI() == 6) {           // buildings only
        this.GetHeight()                   // vtable+0x1C8 (side-effect only)
        dims = this.GetType().Dimension2()  // {fw<<8, fh<<8, H*HeightFactor}
        half = dims / 2
        coord = this.GetCoords()

        if (WhatAmI() != 0xF) {            // effectively always true inside the RTTI==6 block
            // Draw 4 DrawBracketCorner edges forming front/right visible corners
            DrawBracketCorner(FL_ground → FR_ground)       // front ground
            DrawBracketCorner(BR_ground → FR_ground)       // right ground
            DrawBracketCorner(FL_roof   → FL_ground)       // front-left vertical
            DrawBracketCorner(BR_roof   → BR_ground)       // back-right vertical
            // 3 direct DrawLine3D calls — single stubs at visible corners converging
            // on FR_roof (hidden behind sprite). Each is a 25% stub computed via
            // VecAdd×3 + VecDiv(4). See SELECTION_BRACKETS_GHIDRA_REPORT §2 for full
            // 12-edge topology.
            ...
        }

        // Step 3.5 — alliance pip hook (see next)
        if (this.Strength > 0 &&
            (IsAlliedWith(this.Owner, g_PlayerPtr)
             || g_RulesClass[+0x17E6] /* EnemyHealth */ != 0)) {
            this.vtable[+0x448]()     // empty stub on base — deferred behavior
        }

        // Infantry path (unreachable for buildings — RTTI==6 here — but decompiler
        // emits it because DrawExtras is the common function for all technos)
        if (WhatAmI() == 0xF) {
            // DrawSingleBracketStub calls — infantry PIPBRD anchor stubs
            ...
        }
    }
    this.DrawHealthBar(pLocation, pBounds, false)    // vtable+0x44C, bUnk3=false
}
```

**Three critical parity invariants:**

1. **Only buildings get line-drawn brackets.** Units, infantry, and aircraft
   never draw line brackets — their "bracket" is `PIPBRD.SHP` drawn inside
   `DrawHealthBar` (see §3.7).
2. **Line bracket color is palette index `0xF` (white) normally, or `0xC` (dim)
   when `GetHeight() < -4`** (underground/limbo). Decompilation confirms the
   `< -4` threshold in both `DrawBehind` and the DrawExtras line path.
3. **Bracket edges use `Tactical::DrawLine3D` (vtable+0x60) to `g_PrimarySurface`**
   — this is the same renderer as rally lines and waypoint lines (see
   `docs/PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md` §3). Same depth
   characteristics; no Z-test; alpha-blended.

### 3.5 Alliance-pip hook (vtable+0x448)

`+0x448` is an empty stub (`FUN_006F60C0`) on the base TechnoClass vtable. No
subclass in standard YR was observed to override it. **Effectively a no-op
during gameplay.** Treat as a reserved extension slot; do not implement.

The gate is informative though: the call is guarded by
`Strength > 0 && (IsAlliedWith || EnemyHealth)`. That gate is REUSED to fence
later DrawPipScalePips calls. The `EnemyHealth` rule at `RulesClass+0x17E6` is
the `[AudioVisual] EnemyHealth=yes` key (default yes in YR). When enabled,
enemy-owned units show their health bar to the local player.

### 3.6 Health bar (vtable+0x44C) for SELECTED

```
if (IsSelected) {
    this.DrawHealthBar(pLocation, pBounds, bUnk3=false)   // vtable+0x44C
}
```

`DrawHealthBar` @ `0x006F64A0` is covered by the dedicated building-pip reports
and `HEALTH_BAR_POSITIONING.md`. Summary:

| Class | PIPBRD frame | PIPBRD offset from pLoc | Pip count | Pip start offset | Pip color frames |
|---|---|---|---|---|---|
| Building (WhatAmI=6) | (none — isometric pips directly) | — | `(fh × 15) / 2` | NW-edge: `pLoc + (3, -pipCount*2+2)` with step `(-4, +2)` | 0=empty, 1=green, 2=yellow, 4=red |
| Infantry (WhatAmI=0xF) | 1 | `pLoc + (11, delta-25)` | 8 | `pLoc + (-5, delta-24)` step `(+2, 0)` | 16=green, 17=yellow, 18=red |
| Unit/Aircraft (else) | 0 | `pLoc + (1, delta-26)` | 17 | `pLoc + (-15, delta-25)` step `(+2, 0)` | 16=green, 17=yellow, 18=red |

where `delta = TypeClass[+0x3E0] = PixelSelectionBracketDelta` (default 0).
For building pips, the listed NW-edge point is the `CC_Draw_Shape` caller draw
point. `PIPS.SHP` frames `0/1/2/4` are drawn with flags `0x600`, so their final
frame-rect top-left is `draw_point + (-5,-3)`.

**Pip color is 100% health-driven, NOT armor-driven.** Confirmed by reading
the frame selection:

```c
frame = GREEN
if GetHealthRatio() <= g_RulesClass[+0x1700] /* ConditionYellow */: frame = YELLOW
if GetHealthRatio() <= g_RulesClass[+0x1708] /* ConditionRed */:    frame = RED
```

There is no armor-type lookup, no per-Techno pip-palette override, and no
`Armor=` cross-reference inside DrawHealthBar. The "pip color per armor type"
guess is not supported by the binary.

After drawing pips, `DrawHealthBar` calls `vtable+0x450` (`DrawPipScalePips`)
**conditionally** — only if:
- Non-building: allied with local player OR `DisplayProductionTo[+0x210]` spy
  bitfield has the local player set.
- Building: same, OR `TypeClass[+0x3D8] PipsDrawForAll = yes`, OR
  `TypeClass[+0x157B] CanBeOccupied = yes` (garrisonable buildings always show
  occupancy pips to everyone — this is the mechanism that lets enemies see
  whether an IFV/bunker/civilian is garrisoned).

### 3.7 Health bar for HOVERED (vtable+0x44C) — cloaked variant

```
if ((byte)this[+0x431] != 0 /* IsMouseHovering */ && !IsSelected) {
    if (!this.IsDisguised()) {
        this.DrawHealthBar(pLocation, pBounds, bUnk3=true)
    } else if (this.vtable[+0xD0]() /* DisguiseAs? */ != 0) {
        this.DrawHealthBar(pLocation, pBounds, bUnk3=true)
    }
}
```

- `bUnk3=true` suppresses the PIPBRD background (since IsSelected=0 guards it
  inside `DrawHealthBar`). Only the colored pips show.
- The hover flag is `TechnoClass+0x431`; it is set by
  `DisplayClass::SetCursorFromAction` and cleared by `TechnoClass::AI_Update`.
  Damaged non-selected/non-hovered buildings do not reach `DrawHealthBar` through
  this verified render path.
- Disguised units must expose their disguise-target visuals through
  `vtable+0xD0` for the hover health to render.

### 3.8 Talk bubble (TALKBUBL.SHP)

```
if (g_TalkBubbleTarget == this
    && g_TalkBubbleFrameCount > 0
    && FUN_004F1B20() > 0 /* animation timer check */) {
    CC_Draw_Shape(g_TALKBUBL_SHP, g_TalkBubbleFrameCount - 1,
                  pLocation, pBounds, zOrder=0x600, ...)
}
```

- Z-order **`0x600`** (same layer as health pips, canvas-centered).
- **Only one entity at a time** — the global `DAT_00B0EB38` (target pointer)
  rules. If a second talk trigger fires while a first is active, the first is
  pre-empted.
- `DAT_00B0EB3C` counts down the frame index; when it reaches 1 the bubble
  pops on its last frame and stops.

### 3.9 Intra-phase draw order summary

The actual z-bottom-to-top stack within one object's DrawExtras call is:

1. **Ivan bomb clock** (z `0xE00`) — topmost overlay
2. **Wrench** (z `0xE00`) — deploy-ready blink
3. **Veterancy chevron** (z `0xE00`, z-adjust -2) — rank indicator
4. **Selection brackets** (building only; `Tactical::DrawLine3D` to primary surface)
5. **Alliance-pip hook** (currently no-op stub)
6. **Health pips** (z `0x600`) — if selected
7. **Hovered-only health pips** (z `0x600`) — if hovering, not selected
8. **Talk bubble** (z `0x600`) — one at a time

**Critical:** because steps 1-3 share z-order `0xE00` and steps 6-8 share
`0x600`, the *numeric* z values only distinguish bomb/wrench/veterancy (high)
from pips/talk (low). Actual pixel stacking within the high group and within
the low group is driven by **call order**: steps listed later overwrite steps
listed earlier at the same z. So:

- Wrench can obscure Ivan-bomb clock if both on same cell (rare but possible —
  wrench is drawn after).
- Veterancy chevron can obscure both bomb and wrench (drawn last in the high
  group).
- Hovered pips can obscure selected pips (the hovered path draws last when
  both IsSelected and IsMouseHovering are set — but they're mutually exclusive
  by guard, so this is not observable).

---

## 4. Two-pass object-level render (`ObjectRenderingLoop`)

`Tactical::ObjectRenderingLoop` @ `0x006D8DB0`. Runs once in Pass 2 Step 8.
Iterates the 5 display layers (`g_DisplayLayers` @ `0x008A0360`) twice.

### 4.1 The 5 display layers

| Index | Name | Contains |
|---|---|---|
| 0 | Underground | Subterranean units (Subterranean APC, burrowed pups) |
| 1 | Surface | Naval surface units, hover craft while moving over water |
| 2 | **Ground** | Buildings, tanks, infantry — the dominant layer |
| 3 | Air | Flying aircraft, rockets, airborne units |
| 4 | Top | Spyplane overlay, topmost effects |

**Only Layer 2 (Ground) is Y-sorted on insertion** (per
`LAYER_CLASS_GHIDRA_REPORT.md`). The others append unsorted. Objects re-submit
themselves to their layer on position change to keep Ground's sort valid.

### 4.2 Loop 1 — sprites

Per layer, per object:
- Clear `obj[+0x99] WasDrawn = 0`
- Compute screen position; clip to viewport with `168 × 180` pixel padding
- If visible:
  - Set `obj[+0x99] = 1` (marks for Loop 2)
  - Call `vtable[+0x10C]` (`SetDrawCoords` for most; `DrawBehind` for buildings)
    with cached screen coords. **For buildings**, this draws the 5 back bracket
    edges BEFORE the sprite, so they render behind.
  - If foot-class (flag bit 0 set on `+0x14`, bit 2 NOT set): also call
    `vtable[+0x110]` (DrawShadow for foot classes; note this is the same slot
    as DrawExtras — BuildingClass overrides `+0x110` to DrawExtras; FootClass
    subclasses override `+0x110` to DrawShadow. The vtable slot is shared;
    the semantics are per-subclass.)
  - Call `vtable[+0x104]` (`DrawIt`) — the main sprite blit.

After Layer 2 completes, `BuildingClass::UpdateGarrisonFire` runs for each
visible garrisoned building (to update muzzle flash state for occupants).

### 4.3 Loop 2 — extras

Per layer, per object with `WasDrawn == 1`:
- Compute screen position again (fresh projection)
- Call `vtable[+0x110]` — **`DrawExtras`** (the 9-step sequence from §3).

### 4.4 Z-order implications

- **Back bracket edges (building DrawBehind): behind the building sprite** —
  because DrawBehind is called in Loop 1 before DrawIt.
- **Front bracket edges, pips, health, bomb clock, wrench, veterancy, talk
  bubble: on top of ALL object sprites in the same layer** — because Loop 2
  draws them all after Loop 1 finishes.
- **Layer ordering stacks:** Underground → Surface → Ground → Air → Top. So
  an Air unit's bracket is on top of a Ground unit's sprite (as expected for
  flying units). Two Air units at similar screen Y don't Y-sort; their draw
  order is insertion order.
- **`vtable+0x10C` is overloaded per subclass.** For BuildingClass the slot
  at this offset is `TechnoClass::DrawBehind` (0x006F60D0); for FootClass
  derivatives it's a lighter `SetDrawCoords` that caches pLocation without
  drawing. The SAME slot, DIFFERENT overrides. Rust port should replicate
  this: a `PreDrawSetup(obj)` virtual that's a no-op for foot and draws
  back edges for buildings.

---

## 5. `DrawPipScalePips` @ `0x00709A90` — the overlay workhorse

The largest draw function in the overlay set (3553 bytes). Handles six sub-systems:

### 5.1 Base anchor — different for buildings vs non-buildings

```c
if (WhatAmI() == 6) {          // Building
    base_x = pLoc.x + 6;
    base_y = pLoc.y - 1;
    step_x = 2;                 // isometric NW-edge step
    step_y = 4;
} else {
    base_x = pLoc.x - 5;
    base_y = pLoc.y;
    step_x = 0;                 // horizontal row
    step_y = 4;
}
if (WhatAmI() == 0xF) base_x += 11;    // infantry right-shift
```

### 5.2 Spawn/docked count (aircraft pads, carrier decks)

Reads `TypeClass[+0xD5C]` (spawn count). Draws one pip per slot using PIPS.SHP.
Frame `0` = docked/available (actual spawn count at base), frame `1` = slot
empty. Used for Dreadnought launchers, Aircraft Carrier fighters,
Floating Disc spawners, Boomer subs.

### 5.3 Occupant slots (buildings with `CanBeOccupied=yes`)

Gated by `TypeClass[+0x1584]` (MaxOccupants). Reads owner's 3-byte occupant
color index from `HouseClass[+0x1580 × +0xDFC]` table.

### 5.4 Tiberium/ore storage (buildings, `PipScale=Tiberium`)

Gated by `TypeClass[+0x3D4] == 2`. Uses PIPS.SHP frames `0` (empty), `2`
(yellow/gem), `5` (green/ore). Reads `StorageClass::GetAmount(0..3)` for each
of the 4 ore types and distributes pips proportionally.

### 5.5 Passengers (transports, `PipScale=Passengers`)

Gated by `TypeClass[+0x3D4] == 5`. Reads the linked-list at `+0xD8`
(passenger list head) and assigns each pip a `HouseType[+0xDFC]` color.
Infantry passengers get house-colored pip; vehicle passengers get frame 5.
Unused slots = frame 0.

### 5.6 Ammo (aircraft, `PipScale=Ammo`)

Gated by `TypeClass[+0x3D4] == 1`. Complex path because of `PipWrap`:

```c
if TypeClass[+0x3E4] PipWrap == 0:                       // no wrap — simple row
    for each ammo pip: draw PIPS2.SHP frame 13 (amber)
else:                                                     // wrap — grouped rows
    row_count = MaxAmmo / PipWrap
    for row = 0 to row_count:
        for slot = 0 to (row * PipWrap + slot_in_row < CurrentAmmo)? filled : empty:
            frame = slot_in_row + 15     // shift by base
            draw at (base_x + step_y × slot, base_y + step_x × row)
```

The wrap pattern is used for the MIG's 2-round ammo and the Aegis Cruiser's
multi-slot layout. Without PipWrap, ammo becomes one long strip.

### 5.7 Self-heal indicator

Two modes based on `TypeClass[+0xD97] Organic`:

```c
if WhatAmI() == 0xF || (WhatAmI() == 1 && Organic):
    // Organic self-heal — infantry regeneration
    if House::HasPowerOutput() && Health < MaxStrength:
        frame = 0xD /* 13 */
        blink_period = g_RulesClass[+0x30] /* SelfHealInfantryFrames */
        is_flash = (g_CurrentFrameCounter % blink_period < 6)
        offset = (infantry ? (+19, -35) : (+38, -32))
elif WhatAmI() == 1 && !Organic:
    // Mechanical self-heal — vehicle repair
    if House::HasPowerDrain() && Health < MaxStrength:
        frame = 0x14 /* 20 */
        blink_period = g_RulesClass[+0x38] /* SelfHealUnitFrames */

z_order = is_flash ? 0x601 /* translucent */ : 0x600
draw PIPS.SHP frame at offset
```

- `HasPowerOutput()` requires the house to own a Hospital (for organic).
- `HasPowerDrain()` requires the house to own an Armory (for mechanical).
- Flash period defaults: `SelfHealInfantryFrames = 150`,
  `SelfHealUnitFrames = 300` (rules.ini).
- The 6-frame window per cycle is the "on" phase.

### 5.8 DrawExtraInfo (text label) — buildings only

```c
if WhatAmI() != 6:
    pos = pLoc + (-10, +10)
    this.vtable[+0x458](pos, pLoc, pBounds)   // DrawExtraInfo
```

- Non-buildings trigger the call (despite the decompilation's non-building
  guard ordering — the conditional selects which code path runs).
- DrawExtraInfo text is rendered in **owner's house color** read from
  `HouseClass[+0x56F9..+0x56FB]` (3 bytes RGB), not the palette.

### 5.9 Group-number overlay

```c
group_idx = this[+0x214]     // signed int, -1 if not in group
if group_idx < 0 || group_idx > 9: return
digit = ((group_idx + 1) & 9)       // "0" maps to group index 9

pos_x = pLoc.x - 4
pos_y = pLoc.y + (infantry ? -0x24 : -0x27)
      // infantry: -36px, others: -39px — above sprite

color_rgb = HouseClass[+0x56F9..+0x56FB]
rect = ComputeTextRect(pos, 0x49 /* 73 px wide */, 2 /* 2 px tall */, -2)
clipped = AlphaShapeClass::ClipRect(rect, 0, 0)
g_PrimarySurface::SetClip(clipped)
g_PrimarySurface::SetColor(packedRGB)
FUN_004A66D0(surface, ..., digit_char, rect, color, 0, 0x49, 0xFFFFFFFF, 1)
```

- **Text size: 73×2 pixels** — surprisingly small. The digit is drawn with
  `GAME.FNT` (the standard bitmap font).
- **Digit character comes from `DAT_0081B3D0`**, a string "1234567890" — the
  `+1` shift means group 0 shows "1" and group 9 shows "0" (matching hotkey
  labels).
- Rendered via `FUN_004A66D0` — a text renderer that writes to the primary
  surface directly (no SHP compositing).
- Z-order inherits from the text renderer (implicitly above pips because it's
  drawn after).

---

## 6. `Garrison_DrawOccupantPips` @ `0x00430AC0`

Runs once per frame as Pass 2 Step 15 (AFTER the main object loop). Separate
system from the per-techno `DrawPipScalePips` occupant path.

### 6.1 Structure

```c
if param_1[+0x60] /* active-region flag */ != 0:
    for y = 0..8 (3 rows):
        for x = 0..2 (3 cols):
            obj = *param_1
            if obj != NULL && (obj[+0xC] & 1) != 0:
                if HouseClass::Is_Ally_ByIndex(obj[+0x110]) &&
                   IsAlliedWith(Houses[obj[+0x110]], g_PlayerPtr) &&
                   Houses[obj[+0x110]][+0x1F5] /* !PersonalFlag */ == 0:
                    FUN_00430250(obj, ...)    // the actual pip draw
            param_1 += 1                       // advance cell
```

### 6.2 Inner draw — `FUN_00430250`

Not itself simple. Draws:
- **Occupant flag SHP** (`DAT_0089C474`) — cycling animation with
  `g_CurrentFrameCounter % (period/2)` plus offset
- **Garrison occupant name text** via `BitFont::MeasureText` + `FUN_00434CD0`
  for the text render
- Uses the HouseClass color scheme lookup `HouseType[+0x16054]` → `g_ColorSchemeArray`

The separate grid scan exists because **one garrison building can hold up to
10 occupants** but only draws the pip strip over a 3×3 footprint — the grid
scan lets the engine redraw all occupants of all visible garrison buildings
without iterating a per-building occupant array from `BuildingClass[+0x684]`.

---

## 7. `FUN_006DBE20` — "DrawUnitActionVisuals" (NOT brackets)

Prior pipeline report `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md` labeled this
Pass 2 Step 14 as "Selection brackets / health bars on technos" — **that
description is wrong**. Decompilation reveals two distinct responsibilities:

### 7.1 Per-unit radial indicator (vtable+0x130)

```c
for obj in g_CurrentObjects:
    if obj.GetType() != 0 && TypeClass[+0x238] != 0:
        obj.vtable[+0x130]()
```

`vtable+0x130` is **an empty stub (`FUN_0041BE80`) on the TechnoClass base**.
Only a narrow set of Yuri-era psychic/radar buildings override it. For
standard YR units, this is a no-op. Ignore for parity.

### 7.2 Building sensor-range / gap-generator circles

When `DAT_00A8EB7E` (probably "show sensor circles" debug/game flag) is non-zero
AND the selected object is a building with `GetSensorRange() > 0`:

```c
coords = Get_Current_Cell_CenterCoords()
dot_color = g_DD convert 3-byte house color
FUN_00456980(coords, circle_color, radius, 0, 1)    // draw circle outline
for each allied player:
    find nearest enemy unit within 2×radius
    draw ring line from that enemy to our building
```

**Parity implication:** this renders the radius rings you see when selecting a
Psychic Sensor or Gap Generator. The rings are:

- Drawn on `g_PrimarySurface` (above all sprites)
- Animated via `DAT_00B0E648` ± `DAT_00842950` (oscillates 0..100 for a pulse)
- Colored by owner's house RGB at `HouseClass[+0x56F9]`
- Multiple concentric rings when `fStack_2c != 0` (multi-ring effect for
  combined psychic sensor + guardian reveal scenarios)

This is visible to the player when they select a Psychic Sensor, Gap
Generator, or similar radial-effect building — the range ring is always
overlaid in their house color. Not to be confused with selection brackets.

---

## 8. `FUN_004DC340` — "Detection brackets" are sensor action lines

Pass 2 Step 22's "detection brackets" path. Decompilation shows it's
`TechnoClass::DrawRadarActionLines` — draws the **yellow/house-colored dashed
line** from a Psychic Sensor-detected non-human FootClass unit to its current
target/final navigation endpoint, plus 3x3 pixel dots at each endpoint.

**Not a bracket.** Animation period via `timeGetTime() & 0x3FF` for the dash
offset, giving a visible "marching ants" pulse at ~975 Hz (very fast).
Produced when a local Psychic Sensor coverage check passes. `FUN_0043B150`
compares the enemy's ArchiveTarget/final NavCom endpoint against the detecting
building's `PsychicDetectionRadius` (retail `[NAPSIS]` uses `15`).

---

## 9. Select-moment animation — **absent**

Decompilation of `TechnoClass::Select` @ `0x006FBFA0` and
`ObjectClass::Select` @ `0x005F4520` confirms:

```c
// TechnoClass::Select (post-base chain)
if is_mind_controlled: refresh MC visual
if is_human_player && g_SelectionVoice_Enable: play VoiceSelect
process deferred voice queue
return 1
```

**There is no bracket-grow, bracket-flash, pulse, fade-in, or scale animation
triggered by selection.** Brackets are:

1. Drawn every frame in `DrawExtras` regardless of when selection happened
2. Same size and position whether the unit was just clicked or was selected
   100 frames ago
3. Not animated as a function of time-since-select — the only time-varying
   element in the overlay stack is the wrench blink (on deploy-ready
   buildings) and the self-heal pip flash. Those are independent of
   selection.

The visual feedback for "I just selected this unit" is:
- PIPBRD background appears (instant, no fade)
- PIPS pip strip appears (instant)
- Brackets appear for buildings (instant)
- Voice line plays (VoiceSelect, 1 per selection batch)
- That's it. No tween.

**Rust parity:** do not implement a select-moment animation. Static brackets
match original behavior.

---

## 10. Pip color mapping — health, not armor

Every pip-color branch in `DrawHealthBar` reads:
- `ObjectClass::GetHealthRatio()` @ `0x005F5C60` (= `Health / MaxStrength`)
- Compares to two doubles: `g_RulesClass[+0x1700] ConditionYellow`,
  `g_RulesClass[+0x1708] ConditionRed`
- Selects frame by ratio alone

No branch inspects `Armor=` (TypeClass+0x188 / +0x20C), `WeaponTypeClass`,
`WarheadTypeClass::Verses`, or any armor-indexed table. The pip color is a
pure health indicator.

**Rust parity confirmed:** the current implementation at
`src/app_ui_overlays.rs` drives pip color from health ratio alone, which
matches.

---

## 11. INI keys that affect bracket/pip rendering (verified)

| Key | Section | Default | Effect | Reader address |
|---|---|---|---|---|
| `PixelSelectionBracketDelta` | `[UnitName]` | `0` | Vertical Y offset for PIPBRD + pip strip. Negative = move UP (closer to sprite top). Applied on all non-building classes. | `TechnoTypeClass::ReadINI` @ `0x00714173` → `+0x3E0` |
| `PipScale` | `[UnitName]` | (none) | Selects DrawPipScalePips sub-system: `Ammo`=1, `Tiberium`=2, `Passengers`=5. Building-scoped for Tiberium (Refinery/Silo); non-building for Ammo/Passengers. | `TechnoTypeClass[+0x3D4]` |
| `PipWrap` | `[UnitName]` | `0` | Rows for wrapped ammo display. 0 = single-row strip. Non-zero = multi-row grouping. | `TechnoTypeClass[+0x3E4]` |
| `PipsDrawForAll` | `[BuildingName]` | `no` | If yes, building pips show to ALL houses (not just allied/spied). | `TechnoTypeClass[+0x3D8]` |
| `Organic` | `[UnitName]` | `no` | Selects self-heal pip frame (13 for organic, 20 for mechanical) and blink period. | `TechnoTypeClass[+0xD97]` |
| `CanBeOccupied` | `[BuildingName]` | `no` | If yes, building occupant pips show to everyone. | `BuildingTypeClass[+0x157B]` |
| `MaxNumberOccupants` | `[BuildingName]` | `0` | Pip slot count for occupants. | `BuildingTypeClass[+0x1584]` |
| `ConditionYellow` | `[General]` | `0.5` | Pip color transition threshold (double). | `RulesClass[+0x1700]` |
| `ConditionRed` | `[General]` | `0.25` | Pip color transition threshold (double). | `RulesClass[+0x1708]` |
| `EnemyHealth` | `[AudioVisual]` | `yes` | Enable health-bar visibility on enemy units when selected/hovered. | `RulesClass[+0x17E6]` |
| `SelfHealInfantryFrames` | `[General]` | `150` | Blink period for organic self-heal pip. | `RulesClass[+0x30]` |
| `SelfHealUnitFrames` | `[General]` | `300` | Blink period for mechanical self-heal pip. | `RulesClass[+0x38]` |
| `Height` | `[BuildingName]` (art.ini redirect) | `2` | Bracket vertical extent for buildings (multiplied by `g_HeightFactor` ≈ 15 px/unit). | `BuildingTypeClass[+0xEF4]` via Image= redirect |
| `Foundation` | `[BuildingName]` (art.ini) | `1x1` | Bracket diamond footprint — determines all bracket geometry via Dimension2. | tables at `0x008192B8` (width), `0x00819310` (height) |

**Not found (deprecated / TS-only):**
- `VeteranInsignia` — referenced in ModdingWiki but no reader found in binary.
- `PipScale=MindControl` / `PipScale=Power` — documented by modders but no
  corresponding `TypeClass[+0x3D4]` values 3 or 4 observed in decompilation.

---

## 12. Current Rust implementation status

From scan of `src/render/selection_overlay.rs`, `src/app_ui_overlays.rs`,
`src/app_selection_brackets.rs`, `src/app_render/`:

| Feature | gamemd.exe | Rust |
|---|---|---|
| Building line brackets (9 back + 4 front + 3 single-stub) | Yes | **Implemented** (12 edges total, fixed foundation table, integer end-exclusive line pixels; A-buffer modulation and Z-test/no-Z-write parity still deferred) |
| Building final pip frame-rect anchor | Yes | **Implemented/refined** - applies the `PIPS.SHP` frame/canvas offset `draw_point + (-5,-3)` |
| Building health-pip caller gate | Yes | **Implemented/refined** - selected buildings and cursor-hovered structures draw pips; damaged-only non-selected/non-hover buildings do not |
| Building bracket dim color (`GetHeight() < -4` -> palette `0xC`) | Conditional | **Not implemented** — standard selected buildings do not reach this in normal YR states; forced bridge/negative-height states remain conditional |
| Building NW-edge pip layout + canvas centering | Yes | **Implemented/refined** — formula handles odd-foundation anchor shifts such as `[TESLA] Image=NATSLA` and `GAREFN`, not only `sy - 11 - H*15` |
| Infantry PIPBRD frame 1 + 8 pips | Yes | **Implemented** |
| Vehicle/Aircraft PIPBRD frame 0 + 17 pips | Yes | **Implemented** |
| `PixelSelectionBracketDelta` offset | Yes | **Parsed but not applied** to non-building Y positions |
| Ivan bomb clock (BOMBCURS/CHRONOSK, 13-frame) | Yes | **Not implemented** |
| Deploy-ready wrench (WRENCH.SHP 6-frame blink, z `0xE00`) | Yes | **Not implemented** |
| Veterancy chevron (PIPS frames 14/15/19, z `0xE00`) | Yes | **Not implemented** (struct field exists, no draw) |
| Talk bubble (TALKBUBL.SHP) | Yes | **Not implemented** |
| Self-heal pip (organic / mechanical blink, z `0x600/0x601`) | Yes | **Not implemented** |
| Cargo/passenger pips (PIPS2.SHP frames 0/2/5) | Yes | **Implemented** for miner cargo (tiberium), partial for passengers |
| Occupant pips (PIPS frames 6..12) | Yes | **Implemented** (src/render/selection_overlay.rs) |
| Ammo pips (PipWrap grouping) | Yes | **Not implemented** |
| Group number overlay (73×2 px text at -36/-39 y) | Yes | **Not implemented** |
| Garrison pip separate scan (Pass 2 Step 15) | Yes | **N/A** — Rust combines into per-building draw |
| Sensor-range / gap-generator rings (DrawUnitActionVisuals) | Yes | **Not implemented** |
| Psychic Sensor detection action lines (DrawRadarActionLines) | Yes | **Not implemented** |
| Selected building Psychic Sensor / Gap Generator radius rings | Yes | **Implemented** as separate action-visual overlays, not selection brackets |
| Two-pass object render (first object pass plus later second `DrawExtras`) | Yes | **Partial** — Rust now has pre-body and final front-bracket submissions, but still approximates per-object interleaving as phase buffers |
| Bracket-behind-sprite for buildings (DrawBehind phase) | Yes | **Implemented** — back/left edges draw before object bodies |
| Select-moment animation | **Absent** in gamemd | **Absent** in Rust (matches — by not implementing an animation we don't diverge) |
| Pip color by health ratio (not armor) | Yes | **Matches** — `src/app_ui_overlays.rs` drives from health ratio |

---

## 13. Parity implications (ranked by visible impact)

1. **Veterancy chevrons** — immediately visible on every ranked unit, high
   density in late-game skirmishes. Cheap to implement: one SHP frame per
   unit per frame, driven by `veterancy` struct field. The 3 frames (14/15/19)
   are already in PIPS.SHP so no asset work. **High impact, trivial cost.**

2. **Group number overlay** — players who use hotkeys see this on every grouped
   unit. Distinctive "1"/"2"/etc. in house color above the unit. **Medium
   impact, medium cost** (text-rendering path + house color lookup).

3. **Building bracket behind-sprite effect** — the characteristic "only roof
   corners visible" look of RA2 building selection. If all edges draw on top,
   the brackets look like a plain wireframe box instead of the iconic "3
   corners above the roofline" visual. **Medium impact, low cost** (needs
   two-pass draw flag on the bracket lines).

4. **Self-heal pip flash** — crazy Ivan, hospital-equipped infantry, armory-
   equipped units all show this. Players learn to scan for it. 6-frame on /
   (period-6) off cycle with z-order 0x601 vs 0x600 for the translucent
   flash. **Medium impact.**

5. **Ammo pips + PipWrap** — currently zero feedback on aircraft ammo state.
   Siege Chopper, Kirov bomb count, MIG reload all need this. **Medium
   impact.**

6. **Deploy-ready wrench** — visible every time a factory finishes producing.
   A clear "click me to place" signal. 6-frame blink on buildings. **Medium
   impact, low cost.**

7. **Ivan bomb clock** — rare (only when Ivan-bombs are attached). Very
   visible when present. **Low-medium impact.**

8. **Talk bubble** — scripted-only (maps with trigger actions for AI
   dialogue). Invisible in pure skirmish. **Low impact for skirmish parity.**

9. **`PixelSelectionBracketDelta` application** — parsed but not applied.
   Kirov/Dreadnought/Aircraft Carrier all have large negative deltas that
   pull the bar up to the unit's center of mass. Without this, the pip bar
   floats 30px above a Carrier instead of centered on the deck. **Medium
   impact (naval/large aircraft only).**

10. **Sensor-range / gap-generator rings** — only visible on these specific
    buildings, but when they are selected this is the main visual feedback
    (they have no bracket in the traditional sense). **Low impact until
    Psychic Sensor / Gap Generator ship.**

---

## 14. Open questions

1. **`TypeClass+0x238` semantic** — gates `vtable+0x130` in
   `DrawUnitActionVisuals`. Observed `+0x130` is an empty stub on base; one or
   two units may override it (likely Psychic/Gap building variants). Not
   decompiled this pass. Low priority — the empty stub means we can ignore
   this hook for all but the narrow set of buildings that override it.

2. **`vtable+0x448` reserved hook** — follow-up
   `TECHNO_DRAWEXTRAS_VTABLE_448_BUILDING_HOOK_OVERRIDES_GHIDRA_REPORT.md`
   verifies stock `BuildingClass` points this slot to empty `0x006F60C0`; no
   stock visible behavior remains to implement here. The "allied pip" wording is
   descriptive only. Non-building overrides remain out of scope for this
   building-selection-bracket folder.

3. **`vtable+0x118` vs `+0x454` DrawVeterancyPips dual definition** — why the
   two-slot architecture? Likely `+0x118` is the historical Tiberian Sun
   veterancy hook and `+0x454` is a YR-expansion override. The TS slot is
   effectively dead. Harmless but cosmetic oddity.

4. **Exact `CanBeOccupied` pip color source** — `DrawPipScalePips` occupant
   path reads `HouseType[+0xDFC]` for color index, then PIPS.SHP frame for
   that house color. Not traced to the specific HouseType field (probably
   `TextColor` or `Color`). If we need exact color match, spot-check the
   HouseType INI keys.

5. **`DrawExtraInfo` text content** — string 0x3C7B (and 0x3C8E / 0x3C90 for
   1×1 buildings) from `csf` string table. Not inspected which string these
   resolve to. Probably the occupant count (e.g., "2/5"). Low priority.

6. **`vtable+0x68 VisualState != 5` guard** — what is state 5? Presumably a
   "hidden under fog" or "limbo-ish-but-still-renders" state. Not traced;
   disguised + state-5 is the combined veterancy-suppression condition.

---

## Sources

**Ghidra addresses decompiled (this investigation):**

| Address | Name | Purpose |
|---|---|---|
| `0x006F5190` | `TechnoClass::DrawExtras` | 9-step overlay orchestrator |
| `0x006F60D0` | `TechnoClass::DrawBehind` | 5 back bracket edges (buildings only) |
| `0x006F6030` | `TechnoClass::DrawSingleBracketStub` | Single-stub helper (infantry path) |
| `0x006F64A0` | `TechnoClass::DrawHealthBar` | Pip bars for all classes |
| `0x00709A90` | `TechnoClass::DrawPipScalePips` | Cargo/ammo/tiberium/occupant/self-heal/group# |
| `0x0070A990` | `TechnoClass::DrawVeterancyPips` | Rank chevron |
| `0x0070AA60` | `TechnoClass::DrawExtraInfo` | House-color text label |
| `0x006F60C0` | (empty stub) | `vtable+0x448` allied-pip hook |
| `0x0041BE80` | (empty stub) | `vtable+0x130` radial-indicator hook |
| `0x0041C020` | `TechnoClass::IsDisguised_Getter` | `vtable+0xC8` — gates veterancy |
| `0x005F65D0` | `TechnoClass::DrawVeterancyPips (stub chain)` | `vtable+0x118` — chains to `+0x114` (another empty stub) |
| `0x006F5EF0` | `TechnoClass::DrawBracketCorner` | 25% stub line helper |
| `0x006DBE20` | `Tactical::DrawUnitActionVisuals` | Sensor rings + per-unit `vtable+0x130` dispatch |
| `0x006D8DB0` | `Tactical::ObjectRenderingLoop` | Per-layer two-pass object render |
| `0x00430AC0` | `Garrison_DrawOccupantPips` | 8×3 cell scan → inner draw |
| `0x00430250` | Garrison inner pip draw | Flag SHP + occupant name text |
| `0x004DC340` | `TechnoClass::DrawRadarActionLines` | Psychic Sensor-detected enemy action lines |
| `0x006FBFA0` | `TechnoClass::Select` | Selection entry (NO animation) |

**Vtable memory reads:**
- `0x007E8D98` (TechnoClass vtable slots `+0x104..+0x128`) — resolved DrawIt, DrawBehind, DrawExtras, DrawVeterancyPips-stub, DoCloak
- `0x007E90CC` (TechnoClass vtable slots `+0x438..+0x45C`) — resolved DrawActionLines, DrawHealthBar, DrawPipScalePips, DrawVeterancyPips, DrawExtraInfo
- `0x007E8D5C` (TechnoClass vtable slots `+0xC8..+0xE4`) — resolved IsDisguised, Limbo, and cell-mark helpers
- `0x007E8DC4` (TechnoClass vtable slot `+0x130`) — resolved empty stub

**INI keys verified (addresses where read):**
- `PixelSelectionBracketDelta` at `+0x3E0` (reader @ `0x00714173`)
- `PipScale`, `PipWrap`, `PipsDrawForAll` at `+0x3D4`, `+0x3E4`, `+0x3D8`
- `Organic`, `CanBeOccupied`, `MaxNumberOccupants` at `+0xD97`, `+0x157B`, `+0x1584`
- `ConditionYellow`, `ConditionRed` at RulesClass `+0x1700`, `+0x1708`
- `EnemyHealth` at RulesClass `+0x17E6`
- `SelfHealInfantryFrames`, `SelfHealUnitFrames` at RulesClass `+0x30`, `+0x38`

**String/data addresses referenced:**
- `0x00AC1478..0x00AC1484` — PIPBRD/PIPS/PIPS2/TALKBUBL SHP pointers
- `0x008192B8` / `0x00819310` — Foundation width/height tables
- `0x0081B3D0` — `"1234567890"` digit string (group-number overlay)
- `s_D:\ra2mdpost\Techno.CPP` @ `0x00843178` — string table key for extra info

**Prior reports extended:**
- `ra2-rust-game-docs/building-selection-brackets/SELECTION_BRACKETS_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/HEALTH_BAR_POSITIONING.md`
- `ra2-rust-game-docs/TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/VETERANCY_SYSTEM_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/SELECTION_SYSTEM_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/SELECTION_LIFECYCLE_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/LAYER_CLASS_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/TECHNOCLASS_VTABLE_COMPLETE.md`
- `ra2-rust-game-docs/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`

**Rust files referenced:**
- [`src/app_selection_brackets.rs`](src/app_selection_brackets.rs)
- [`src/render/selection_overlay.rs`](src/render/selection_overlay.rs)
- [`src/app_ui_overlays.rs`](src/app_ui_overlays.rs)
- [`src/app_render/draw_passes.rs`](src/app_render/draw_passes.rs)
