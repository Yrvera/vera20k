---
name: BuildingClass DrawBody — Rendering Pipeline
description: Body/turret/upgrade layering, gate frames, power-down overlay, damage palette, cloak rendering, lighting interactions.
type: reference
---

# BuildingClass DrawBody — Ghidra Research Report

**Address:** `0x0043D290` (body `0x0043D290 - 0x0043DA75`, 2021 bytes)
**Dispatcher:** `0x0043CEA0` (vtable+0x104)
**Sibling (VXL/extras):** `0x0043DA80` (vtable+0x4E4) — newly created in Ghidra
this investigation; see §4.
**Confidence:** HIGH for pipeline structure, SHP path, damage-frame logic, gate
interpolation, overlay layering, and tint derivation. MEDIUM for
BuildingLight/LightSource hook-in (no direct DrawBody reference found). See §14.
**Active in YR:** Yes — core rendering for every building, every frame a
building is visible (Phase 2 of the Tactical object pass).

---

## 1. Overview and Invocation

DrawBody (at vtable slot `+0x114`) draws a building's SHP body, its
factory/production/upgrade overlays, and for voxel-turreted buildings a
secondary pass via the sibling function at vtable slot `+0x4E4`. It is NEVER
called directly — it is always invoked by the Draw dispatcher at
`+0x104` (function `0x0043CEA0`), which is itself called from Phase 2
of `TacticalClass::Draw` via `Tactical_ObjectRenderingLoop` (Layer 4
for buildings, per `DRAW_ORDER_DEPTH_SYSTEM.md`).

### 1.1 Dispatcher `0x0043CEA0` (vtable+0x104)

```
(this, rect_out, pass_flag) {
    if (!IsMapEditor && g_hWnd != 0) {
        // InTunnel/hidden-by-wall-logic gates
        if (!pass_flag_forced && !this[+0x80] && !(this[+0x74] && !this[+0x81])) return 0;

        this[+0x80] = 0;                       // clear dirty flag
        rect_out = AlphaShapeClass::ClipRect(g_RadarViewport);
        get_screen_bounds_via_vtable+0x12C(local_rect);
        if (screen_bounds intersects rect_out) {
            coords = vtable+0xAC(this);        // GetCoord
            CoordsToClient2(coords);
            if (pass_flag == 0) {
                vtable+0x114(this, rect_out);  // → DrawBody SHP (0x0043D290)
            } else if (this[+0x6E7] == 0) {
                vtable+0x4E4(this, rect_out);  // → DrawBody VXL/extras (0x0043DA80)
                return 1;
            }
            return 1;
        }
    }
    return 0;
}
```

Observations:
- `pass_flag == 0` → SHP body pass (0x0043D290)
- `pass_flag != 0 && !InVoxelHidden (+0x6E7)` → VXL body/turret pass (0x0043DA80)
- The dispatcher is called TWICE per frame for VXL-bodied buildings: once in
  the SHP layer pass and once in the VXL/turret extras pass. This preserves
  the original's "SHP chrome behind the turret" layering.

### 1.2 Call-site context

`Tactical_ObjectRenderingLoop` (`0x006D8D50` — address unconfirmed in Ghidra as of 2026-05-29; `get_function_by_address` returns no function at this address, stale label) walks the Y-sorted Layer 2
(Ground) and iterates objects. For each, it calls `vtable+0x104` twice (pass
0 then pass 1). For non-VXL buildings the second call is a no-op because
of `+0x6E7 == 0` → VXL path.

---

## 2. Z-Order (draw layering within one call to `0x0043D290`)

Verified by reading the raw disassembly `0x0043D290..0x0043DA75`.

### 2.1 Order of `TechnoClass_DrawSHP` invocations

For a **normal** (non-gate, non-construction) building, DrawBody issues up to
**three** draw calls, in this exact order:

| Step | What | Guard | Z layer arg |
|------|------|-------|-------------|
| 1. | Primary body SHP (damaged-state aware) | `unaff_retaddr+0xC > 0` (SHP valid) | `2` (normal body) |
| 2. | `BibShape=` SHP | `Type+0x1518 != 0 && this+0x534 != 0` — bib is ONLY drawn when `+0x534` (DamagedState) is set | `0` (behind body) |
| 3. | `DamageFireAnim` or `HealthyIdleAnim` overlay (Construction mission only) | `Mission == 0x10 (CONSTRUCTION)` AND either `Type+0x14EC` (healthy) OR `Type+0x1504` (damaged) set | `0` |

Key surprise: the **bib** drawn here is NOT the ground-tile bib (that's drawn
in terrain Phase 1 via `Tactical_layer_smudges`). This in-DrawBody bib is
actually the damaged-state alternate bib SHP drawn on TOP of the damaged
body underlay. For normal (healthy) buildings the `Type+0x1518 != 0` SHP
is an art-ini `DamagedBibShape` or similar; the `+0x534 != 0` gate confirms
it only blits on damaged buildings.

Construction-mode overlays (step 3) are drawn AFTER the body with `z=0`
layering — they sit ABOVE the body.

### 2.2 Z-layer encoding

`TechnoClass_DrawSHP` receives two layer hints:
- `param_8`: Z-layer (2 = body, 0 = overlay/bib)
- `param_9`: "draw behind flag" (1 or 0) — passed to CC_Draw_Shape flags as
  `0x600` / `0x601` mask bit

The `uVar9 | 0x600` or `uVar9 | 0x601` flag controls whether the shape writes
to the Z-buffer or just tests against it. Bib uses `0x601` (Z-test only, no
write) so it can slip under terrain correctly.

### 2.3 The VXL extras pass (sibling function `0x0043DA80`)

When called with `pass_flag != 0` for VXL-bodied buildings
(`Type+0x16C5 Turret=` or `Type+0x16C6` marker set), this function:

1. (Same construction-anim replay for CONSTRUCTION mission with frame-accurate
   gate animation — reused code block)
2. Primary VXL body via `vtable+0x444` (`TechnoClass::DrawVXL`), with matrix
   from `BuildVXLTurretMatrix` + `Locomotion_Matrix`
3. Secondary turret VXL (`Type+0xC0`) and/or "spinning" VXL
   (`Type+0xB8`), each its own `DrawVXL` call

Turret draw order: **body first, turret second** (body at `Type+0xB8`, turret
at `Type+0xC0`). Each uses its own matrix pose (shear for body, rotate for
turret). This matches the V2 doc's assertion (§12) that voxel turret
buildings render the barrel after the hull.

---

## 3. Body Draw

### 3.1 SHP source selection (vtable+0x6C)

Before any blitting, DrawBody calls `vtable+0x6C` to fetch the SHP pointer:

```c
FUN_004513D0(this):
    if (this[+0x534] != 0 && this[+0x6E4] != 0)
        return Type->vtable[+0x9C]();   // damaged-state SHP (rare branch)
    else
        return Type->vtable[+0xC0]();   // normal SHP
```

- `this+0x534` = `DamagedState` flag (set when `HealthRatio <= Rules.ConditionYellow`)
- `this+0x6E4` = auxiliary damaged-art flag (set in `SetDamagedState` when the
  damaged SHP differs from the healthy one — the INI `DamagedArt=` behavior)
- `Type->vtable+0x9C` = TechnoType virtual getter for "damaged-variant SHP"
- `Type->vtable+0xC0` = TechnoType virtual getter for "primary SHP" (`Image=`)

**Fidelity note:** `+0x534` alone is NOT enough to switch SHP — both it AND
`+0x6E4` must be non-zero. So buildings whose `DamagedArt=` is empty keep
the same SHP and rely on `BuildingClass::GetCurrentFrame` (§3.2) to pick a
damaged FRAME from the same SHP.

### 3.2 Frame selection: `BuildingClass::GetCurrentFrame` (`0x0043EF90`)

Called for each drawn body/anim to pick the frame number. Exact branch
priority (highest to lowest):

1. **`Type+0x16BF LaserFence=yes`** → return `this->LaserFenceFrame` (live
   laser-connection frame; TS-era electric fence port)
2. **`Type+0x16C0 FirestormWall=yes`** → return `this->FirestormWallFrame`
   (TS-only — `Active in YR: NO`, but still in the jump table)
3. **`this+0x534 == 0` (not damaged)** branch:
   a. If `Type+0x16B7 Gate=yes`: `frame = (Type.StartFrame + Type.Frames - frame_index) - 1` (reverse)
   b. If `Mission == 0x13 SELLING`: (corrected 2026-05-29: was "uses `this[+0x534]*12` offset into Type stages (step-wise decay)"; binary shows `((Type[field_0x534*0xC + 0xF08] + Type[field_0x534*0xC + 0xF04]) - iVar3) - 1` — a reverse-frame lookup into the Anim slot indexed by `field_0x534`, using 0xC=12-byte stride via `decompile_function 0x0043EF90` — OPERATOR_OR_ORDER_DRIFT)
   c. Otherwise: return raw `frame_index` (animation phase stored at `this+0xF8`)
4. **`Type+0x157B CanBeOccupied=yes`** (garrisonable) branch:
   - `base = 0; if (GetOccupantCount() > 0) base = 2; if (health <= ConditionRed) base += 1; else if (OccupantCount > 0 && health <= ConditionYellow) base += 1`
   - Special case: `Type+0x634 == -1 && base == 3 → return 1`
   - Else return `base` (civilian garrison pip state)
5. **`Type+0x16B7 Gate=yes` + damaged** → return `Type.GateStages + 1` when
   `healthRatio <= ConditionYellow`, else return `0` (healthy-gate open frame).
   (corrected 2026-05-29: was "return `Type.GateStages + 1` (damaged-gate-closed frame)" which
   omitted the healthy sub-case returning 0; binary shows both branches via `decompile_function 0x0043EF90` — OPERATOR_OR_ORDER_DRIFT)
6. **Normal + damaged** — two sub-cases (corrected 2026-05-29 via `decompile_function 0x0043EF90`):
   - If `field_0x534 == 1`: return `phase + 1`
   - If `field_0x534 != 1` (including > 1): `frame = max(+0xF10+0xF14, +0xF1C+0xF20, +0xF34+0xF38, +0xF40+0xF44) + phase`
   - **`+0xF04/0xF08` (Anim1) is NOT included in the max.** The first pair in the 4-way max is `+0xF10/0xF14`. (was: "The max reduction stacks all four anim states (Anim1..Anim4 at `Type+0xF04..0xF44`)" — WRONG, Anim1 at +0xF04/0xF08 is skipped — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

Key constants:
- `Type+0xF04/F08` = Anim1.Start/Count
- `Type+0xF10/F14` = Anim2
- `Type+0xF1C/F20` = Anim3 (corrected from earlier doc — offsets match)
- `Type+0xF34/F38`, `Type+0xF40/F44` = sub-anims
- `Rules+0x1700` = `ConditionYellow` (double, default 0.5)
- `Rules+0x1708` = `ConditionRed` (double, default 0.25)

### 3.3 Foundation-based draw-offset calculation

After frame selection, DrawBody computes where on screen to blit:

```
footprint_h_cells = GetFoundationHeight(Type, 0)  // +0xEF0 → g_FoundationHeightTable[idx]
footprint_w_cells = GetFoundationWidth(Type)      // → g_FoundationWidthTable[idx]

footprint_h_leptons = footprint_h_cells * 256 - 256   // 0x100 shift - 0x100 offset
footprint_w_leptons = footprint_w_cells * 256 - 256

cell_delta = CellToPixel(footprint_coords)           // isometric 30:60 iso tilt
draw_x = 0xC6 + Type->DrawOffsetX (+0x1530) - cell_delta.x
draw_y = 0x1BE + Type->DrawOffsetY (+0x1534) - cell_delta.y

// SHP reference chosen:
if (Mission is 0x12=UNLOAD or 0x13=SELLING) {
    draw_x = 0xC6
    draw_y = 0x1BE
    // no DrawOffset added during sell/unload animation
}

// ShadowSHA suppression for large buildings:
z_shape_buf = g_BUILDNGZ_SHA
if (footprint_w_cells >= 8) z_shape_buf = 0    // Naval Yard or oversized
```

Magic constants:
- `0xC6` = 198 pixels (X origin of cell anchor — half of cell width * 6 or similar)
- `0x1BE` = 446 pixels (Y origin of cell anchor — half of cell height offset)
- `0x100` = 256 (lepton per cell)
- `0x100` subtract = "start at cell center, back off half a cell"
- Foundation width `>= 8` → drop the shadow SHA (pre-baked depth shape) — too
  large to fit in the atlas. Fidelity note: this suppresses depth occlusion
  for Naval Yard / Sub Pen footprints.

### 3.4 The body-draw call

```c
cell_depth_base = MapClass::GetCellClass(coord)->Level (+0x10A)  // short, signed
body_depth = cell_depth_base + Type->ExtraZAdjust (+0x1548)  // short, signed

vtable+0x1D0(2, 1, body_depth, tint_rgb, ...)    // Z-layer=2 (body), sort=1
z_adjust = Tactical::AdjustForZ()                // rounds with +0.5 epsilon

TechnoClass_DrawSHP(
    shp_buf,
    frame,
    some_draw_buf,
    unaff_retaddr,
    0, 0x100,
    y - z_adjust,
    2,            // Z layer
    1,            // flag bit
    body_depth,
    tint_rgb, ...
)
```

---

## 4. Turret Draw + Offset Calculation

**Key finding:** DrawBody `0x0043D290` (SHP path) does **NOT** draw a turret
itself — SHP buildings with turret-driven visuals use anim slot **9**
(`TurretAnim`) which is drawn by the animation pipeline (Layer 2 anims),
not DrawBody.

The voxel turret/body sibling at `0x0043DA80` draws the turret via
`vtable+0x444` → `TechnoClass::DrawVXL`:

```
body_offset_x = Type+0x11E0 TurretAnimXOffset  (actually VoxelBodyX)
body_offset_y = Type+0x11E4 TurretAnimYOffset  (actually VoxelBodyY)

// Body pass:
matrix = BuildVXLTurretMatrix()                  // 12 floats
matrix = Locomotion_Matrix(matrix, cellZ, tint)  // locomotion tilt
DrawVXL(Type->BodyVXL (+0xB8), 0, uVar18, Type+0x244 body_cache, ..., matrix)

// Turret pass (if Type+0xC0 turret VXL set):
Matrix_RotateZ(turret_facing_radians)            // +/- RateTimer facing delta
Matrix_ShearCol3ByCol0(Type+0x720 offset / 8)    // barrel-length shear
// some rotation for barrel pitch using BarrelStartPitch (+0x1710)
DrawVXL(Type->TurretVXL (+0xC0), turret_facing, uVar18, Type+0x280 cache, ..., matrix)
```

The turret offset itself is drawn from the building's center-cell origin.
`BarrelStartPitch=` (`Type+0x1710`) is loaded here as a **char** (byte), not
an int — it's applied via `RateTimer__Set(char << 8)` in the constructor
(see `BuildingClass__Constructor` at `0x0043BAB5`). In the DrawVXL path the
pitch is applied as a rotation shear. Per-facing lookup is NOT performed —
the turret uses a CONTINUOUS facing from `this+0x2BD` (TechnoClass facing).

`TurretAnim=`-driven SHP turrets (anim slot 9) work differently: the anim
pipeline picks a frame from the turret anim's SHP indexed by
`quantize(facing, Type+0x1710 BarrelStartPitch, Type+0x1714 VoxelTurretFrameBias)`.

---

## 5. Upgrade Layering

**Upgrades are NOT drawn by DrawBody.** They are drawn by the animation
pipeline as entries in the 21-slot `Anims[]` array (`this+0x55C..+0x5AF`).
Slots 0..2 are the three PowerUp slots. Each is an `AnimClass*` that lives
in Layer 2 and draws independently.

DrawBody's contribution to upgrade rendering is indirect:
1. **Upgrade anim attachment:** The PowerUp anim entries at Type+0xF4C
   (slot 0), +0xF90 (slot 1), +0xFD4 (slot 2), each 0x44 bytes, contain
   `XYZAdjust` at `+0x34` and `ZAdjust` at `+0x38`. These are applied by
   `BuildingClass::CreateAnimForSlot` (`0x00451890`) when the upgrade is
   attached.
2. **Upgrade anim Z-ordering:** The animation pipeline sorts by Y-pixel; the
   PowerUp anims are positioned at their `XYZAdjust` offset relative to the
   building's cell center, so they naturally layer ON TOP of the body
   because their Y-pixel is lower than the body's extent.

**Draw order for a 3-upgrade building:**
- Body SHP (z=2, sort=1) via DrawBody
- Bib SHP if damaged (z=0)
- PowerUp1 anim (Layer 2, Y-sorted at its Y)
- PowerUp2 anim (Layer 2)
- PowerUp3 anim (Layer 2)
- ActiveAnim / TurretAnim / etc. (slots 3..20)

The order among PowerUp anims is determined by **Y-sort in Layer 2**, NOT by
slot index. Two upgrades with the same Y-pixel break tie by `Anims[]` slot
index (insertion order into DynamicVector at Submit_Object).

---

## 6. Gate Rendering + Stages

Gate buildings (`Type+0x16B7 Gate=yes`, e.g., NAGATE/GAGATE) get their own
code path in DrawBody (`iVar8 == 0x18` branch, lines 0x0043D548..0x0043D690).

### 6.1 Gate timer struct at `this+0x350`

Byte offset 0x350 within BuildingClass holds an embedded 40-byte
`RateTimer/DoubleTimerStruct` (confirmed via DrawBody reading
`+0x18, +0x19` and calling `FUN_004A52F0` which reads `+0x08, +0x10, +0x14`):

| Offset | Size | Field | Purpose |
|--------|------|-------|---------|
| +0x00 | double | scaledDuration | totalTicks * g_Const_1_0 (frames) |
| +0x08 | int | startFrame | g_CurrentFrameCounter at timer start, -1 = never |
| +0x0C | int | aux | scratch for ftol |
| +0x10 | int | totalTicks | full duration in ticks — UNVERIFIABLE: binary uses this as inner cycle count; the denominator used in `(iVar1 - (iVar2 - elapsed)) / iVar1` is at +0x14, not +0x10; the exact semantics of +0x10 vs +0x14 require tracing the timer setter (see `FUN_004A52F0` via `decompile_function 0x004A52F0`) |
| +0x14 | int | remainingTicks | ticks left — UNVERIFIABLE: binary uses this field as the denominator (total), which contradicts "remainingTicks"; may be swapped with +0x10 |
| +0x18 | byte | isActive | 1 = timer running |
| +0x19 | byte | direction | 0 = opening, 1 = closing |

Helpers (all taking `this = BuildingClass+0x350`):
- `FUN_004A5110` → `isActive && direction==1` (is closing?)
- `FUN_004A5130` → `isActive && direction==0` (is opening?)
- `FUN_004A51B0` → `!isActive && direction==1` (closed?)
- `FUN_004A51D0` → `!isActive && direction==0` (open?)
- `FUN_004A52F0` → float10 `remainingRatio = (total - remaining) / total` (0.0 = just started, 1.0 = complete)
- `FUN_004A51F0` → start timer

### 6.2 Frame selection for gate

```
if (Mission == 0x18 GUARD && (is_opening || is_closing || is_closed)) {
    frame_index = ftol(remainingRatio * Type.GateStages)      // +0x16F8
    if (is_closing) frame_index = Type.GateStages - frame_index
    if (is_open)    frame_index = 0
    if (is_closed)  frame_index = Type.GateStages - 1

    // CLAMPS (OFF-BY-ONE — CRITICAL FOR PARITY):
    if (frame_index >= Type.GateStages) frame_index = Type.GateStages - 1
    if (frame_index < 0)                frame_index = 0

    // Damage offset (adds a second row of gate frames):
    shp = vtable+0x6C()  // same SHP as body
    health_ratio = GetHealthRatio()
    if (health_ratio <= ConditionYellow) damage_offset = GateStages + 1
    else                                  damage_offset = 0

    final_frame = damage_offset + frame_index
}
```

**Fidelity notes:**
- The clamp order is: first clamp-to-max (using `GateStages - 1`), then
  clamp-to-min (0). A value equal to `GateStages` becomes `GateStages - 1`.
  Parity implementations MUST clamp in this order; clamping in the other
  order would leave a negative index momentarily.
- Damage offset is `GateStages + 1`, not `GateStages`. So a
  `GateStages=11` gate has frames 0..10 for healthy states and 12..22
  for damaged states. Frame 11 is unused (the "transition" frame). This
  is where `art.ini NAGATE` damaged frames live.
- `frame_index = 0` means "fully open" (no gate drawn / transparent frame),
  `frame_index = GateStages - 1` means "fully closed". Parity
  implementations must preserve this semantic — inverting it rotates the
  gate backwards.

### 6.3 Gate depth and z-adjust

Gates use the SAME body depth as normal buildings (cell depth +
`Type+0x1548 ZAdjust`), but with `vtable+0x1D0(2, 1, depth, ...)` — no
special gate layer. This means the gate renders IN THE SAME LAYER as the
building body but the SHP frame itself contains the opening/closing
visual.

---

## 7. Power-Down Overlay

**DrawBody does NOT apply a power-down tint directly on the body.** The
power-down visual in YR is done via two mechanisms, neither of which
lives in DrawBody's body-blit code:

### 7.1 Red/yellow pulse on the Primary body (handled in CC_Draw_Shape)

Not implemented in DrawBody at all. The body draws at its normal tint.

### 7.2 Damaged-frame swap (handled in GetCurrentFrame, §3.2)

When health ≤ ConditionYellow, the body picks a frame from the damaged
range. This is a SHP-FRAME SWAP, not a color/palette change.

### 7.3 Dedicated power-off / low-power ANIM (slots 19, 20 = `LowPower=`)

Per `BUILDING_ANIM_STATE_MACHINE.md`, when `OnPowerOff` fires:
- Slot 19 (`LowPower=`) and slot 20 (`SuperLowPower=`) anims are created
- All other slots' PoweredEffect flag (charge state bit in
  `this+0x5B0..+0x5C4`) is cleared, which stops ChargeAnim phases

The power-off pulse you see in gamemd.exe (reddish flashing sidebar chrome
+ the building slightly darker) is driven by the **anim** with a palette-
swap frame range in the Low-Power SHP, NOT a shader tint on the body.

### 7.4 What DrawBody DOES regarding power

DrawBody does not read `this+0x660 HasPower` or `this+0x661 IsOverpowered`.
It relies entirely on:
- GetCurrentFrame having been updated (phase index at `+0xF8` was
  frozen when power was lost — see `BuildingClass::OnPowerOff`)
- The Low-Power anims being already spawned in the Anims[] array

**Fidelity implication:** To match YR, the Rust implementation must:
1. Freeze the building's animation-phase index (not DrawBody's job)
2. Ensure the LowPower anim draws on top (via anim pipeline)
3. NOT apply any shader darkening to the body itself

---

## 8. Damage-State Palette / Frame Swaps

DrawBody relies on TWO mechanisms:

### 8.1 SHP swap (rare, gated on `DamagedArt=`)

Via `vtable+0x6C` (§3.1). Only fires if BOTH `+0x534` (DamagedState) AND
`+0x6E4` (DamagedArt distinct) are set.

### 8.2 Frame range swap (common, every building)

Via `GetCurrentFrame` (§3.2). Damaged buildings add the sum of Anim
frame ranges (`+0xF04..F44`) to the base phase, which lands in a
different (damaged) portion of the same SHP. **No palette change.**

### 8.3 Damage thresholds

Hard-coded in DrawBody for the gate-damage offset:

```
if (GetHealthRatio() <= Rules+0x1700 ConditionYellow)  damage = GateStages + 1
else                                                   damage = 0
```

For the general body and gate frames, this is a **single threshold**
comparison — no intermediate "50%" vs "25%" state. The `ConditionRed`
threshold (`Rules+0x1708`) is used only by GetCurrentFrame's `CanBeOccupied`
branch (civilian garrison health pips) to add a third damage level.

### 8.4 Fire anims (separate from DrawBody)

On-fire animations (chimney smoke, flame pits) are spawned by
`BuildingClass::CreateDamageFireAnims` (`0x0043C0D0`) as regular AnimClass
instances. They Y-sort with Layer 2. Not DrawBody's job.

---

## 9. Cloaked Rendering

DrawBody does NOT implement cloak rendering itself. When a building is
cloaked (CloakState != 0), the pipeline in `TechnoClass_DrawSHP`
(`0x00705E00`) handles it:

```c
// Inside TechnoClass_DrawSHP, after DrawBody queued the blit:
vtable+0x43C → ModifyCloakDrawFlags (0x0070ED80)
// returns modified flags with 0x800 cloak bit set and alpha blending

// Cloak state advance (partial translucency during cloak/uncloak transition):
// driven by this+0x220 CloakState, this+0x224 CloakProgress, at +0x22C step timer
```

**Cloak Generator buildings (Gap Gen / Psychic Sensor):**
- `Type+0x16C7 CloakGenerator=yes` sets `this+0x6EB` direction, `+0x6EC` radius
- These affect world-state (cells marked as shrouded) via
  `BuildingClass::UpdateGapGenerator_Tick`, NOT DrawBody
- The gap-gen BUILDING itself is drawn normally (no cloak), but other
  buildings within range get the cloak flag applied to their DrawSHP call
  (their vtable+0xC4 IsVisible returns appropriate state)

**Fidelity note:** DrawBody for a cloaked building (e.g., Mirage Tank — but
that's a UnitClass, not Building) still gets called; the cloaking effect is
applied inside CC_Draw_Shape via the flag bits added by
`TechnoClass::ModifyCloakDrawFlags`, which pick an alpha-blend blitter.

---

## 10. Lighting Integration (spotlight + ambient)

**Neither `BuildingLightClass*` (+0x600) nor `LightSourceClass*` (+0x614)
is read by DrawBody.** Searched the full 2021-byte decompilation — no
accesses to these fields.

The lights work via a separate pipeline:
- `LightSourceClass` contributes to the per-cell ambient color modifier,
  which `Tactical_layer_base_terrain` applies when drawing the tiles
  underneath / around the building. DrawBody reads
  `MapClass::GetCellClass(pos)->Level (+0x10A)` for depth but not the
  light value.
- `BuildingLightClass` (spotlight, HasSpotlight=yes) is drawn as a
  separate additive-blend pass in `Tactical_layer_animations` step, NOT
  here.

**This means the body SHP is drawn at its SHP's baked-in brightness — the
light values modify the SURROUNDING terrain, not the body itself.** Two
buildings next to each other with different light radii will illuminate the
ground between them, but each building's own SHP is drawn at its palette's
full intensity.

Potential fidelity subtlety: if you apply ambient lighting to building
SHPs in the Rust renderer, you'll get a darker look than gamemd.exe. The
original engine leaves building SHPs at palette intensity and only
modulates the terrain.

---

## 11. Selection Bracket Interaction

DrawBody's bbox does NOT directly contribute to selection brackets. Per
`building-selection-brackets/SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md`:
- vtable+0x10C `DrawBehind` (`0x006F60D0`) draws the BACK bracket edges
  BEFORE DrawBody runs
- vtable+0x110 `DrawExtras` (`0x006F5190`) draws the FRONT bracket edges
  AFTER DrawBody

The bracket geometry itself is computed from the building object coordinates and
the type dimensions:
```
BuildingClass::GetCoords (0x00447AC0)
BuildingTypeClass::Dimension2 (0x00464AF0)
  -> {FoundationWidthTable[Foundation] << 8,
      FoundationHeightTable[Foundation] << 8,
      Height * g_HeightFactor}
```

Correction from the 2026-05-20 bracket swarm:
`0x004263C0` is not a building selection-bracket offset function; it returns the
global "No name" string pointer. The checked bracket paths do not read
`Type+0x1538` as `SelectBracketOffsetX/Y`, and no local/retail INI key by that
name was found. `PixelSelectionBracketDelta` is a non-building pip/PIPBRD Y
offset and is not used for building line-bracket geometry.

DrawBody contributes only the cell depth via `vtable+0x1D0` — which is
used by DrawBehind/DrawExtras to match the building's Z-plane for the
bracket. Misalignment between DrawBody's `cell_depth + Type+0x1548` and
the bracket draw will cause brackets to render in front of / behind wrong
objects.

**Fidelity implication:** The Type+0x1548 ExtraZAdjust shift applied to
the body depth in DrawBody must also be applied to the bracket depth. If
Rust splits these calculations, keep them in sync via a single shared
depth helper.

**Current Rust status:** building bracket instance generation is disabled in
`src/app_render/build_instances.rs` (`build_selection_bracket_instances` is
commented out and the bracket vector is forced to `Vec::new()`). The dormant
builder lives in `src/app_selection_brackets.rs`.

---

## 12. Chronoshift / IronCurtain / Warped / ForceShield Overlays

### 12.1 IronCurtain / ForceShield

Applied by TWO mechanisms, both called from TechnoClass_DrawSHP:

**(a) Invulnerability alpha/scale phase** (ScaleByTemporalVisualPhase at
`0x0070E380`):
```
if (IsIronCurtainActive || IsWarping || ChronoTarget is this) {
    drawIntensity = ScaleByTemporalVisualPhase(intensity)
    drawIntensity = ScaleByWarpInVisualPhase(drawIntensity)
}
```
These functions implement a 12-case state machine that scales the draw
intensity up/down based on `this+0x198..0x1A4` and `this+0x1B4..0x1C0`
timer fields. The result is the pulsing "iron curtain wavy" effect.

**(b) Additive color tint** (computed in DrawBody itself):

```
// For charged units (this+0x294 ChargeSource != 0):
idx = Rules+0x18A4 BerserkColor index (default from [ColorAdd])
rgb = Rules+0x1874[idx*3 .. idx*3+2]
uVar18 |= pack_rgb(rgb, color_mode)  // packed 16-bit 5-5-5 / 5-6-5

// For ForceShield active (vtable+0x160 && this+0x1C4 == 1):
idx = Rules+0x18B0 ForceShieldColor index
rgb = Rules+0x1874[idx*3 .. idx*3+2]
uVar18 |= pack_rgb(rgb, color_mode)
```

The `FUN_004BBC90` returns `DAT_008205D0` = surface color mode:
- 0 → 5-5-5 (15bpp)
- 1 → 5-6-5 (16bpp Mode A)
- 2 → 5-6-5 (16bpp Mode B)
- 0xFFFFFFFF → other

The three pack-formulas build the same RGB tint three different ways,
OR'd into `uVar18` which is passed as the tint to `TechnoClass_DrawSHP`
and propagated into `CC_Draw_Shape` as an additive blend.

Note: `this+0x1C4 == 1` means **ForceShield** active (not IronCurtain).
IronCurtain tint uses `Rules+0x18A8 IronCurtainColor` and is applied
INSIDE TechnoClass_DrawSHP via a different code path (not in DrawBody's
uVar18 build).

### 12.2 Chronoshift / WarpIn

Same ScaleByWarpInVisualPhase (0x0070E4B0) handles 9 cases for the
warp-in animation. Called unconditionally inside TechnoClass_DrawSHP
when the phase timer is active.

### 12.3 Shroud check

Inside DrawBody (line 0x0043D53E) — if `FUN_00487950` (IsShrouded) returns
1, the tint is cleared:
```
if (IsShrouded(coord)) uVar18 = 0;
```

So shrouded buildings (discovered-but-not-visible in TS; not applicable to
YR-without-FogOfWar) drop all tints. In standard YR this branch is
effectively never taken because all discovered cells are visible.

---

## 13. Walkthroughs

### 13.1 Tesla Coil mid-charge (NATESLA)

Assume: healthy, powered, mid-charging (charge timer running), facing 0.

1. Dispatcher invoked with pass_flag=0
2. `vtable+0x1B8` Get_Cell_Packed — cache cell coord
3. `vtable+0x6C` SHP → Type+0xC0 (NATESLA.SHP), returned buf has Count > 0
4. `vtable+0x2C` WhatAmI → 6 (BUILDING)
5. `this+0x534 == 0` (healthy)
6. `vtable+0x184` Mission → 0x4 GUARD (idle)
7. `Type+0x1701 == 0` (not "Invisible")
8. `this+0x294 == 0` (not yet hit by a charge source) → no berserk tint
9. `vtable+0x160 IsIronCurtainActive` = false → no IC tint
10. Mission != 0x18 → skip gate branch
11. Mission != 0x10 → skip construction branch
12. Mission != 0x12/0x13 → `draw_x = 0xC6 + Type+0x1530, draw_y = 0x1BE + Type+0x1534`
13. Foundation width/height from `Type+0xEF0` → FOUNDATION_2X2 → 2×2 cells,
    `foundation_extent = 2*256-256 = 256`
14. `CellToPixel` → pixel offset for iso anchor
15. `g_BUILDNGZ_SHA` kept (2 < 8 cells)
16. `GetCurrentFrame` → `phase` (charge stage index from `this+0xF8`)
17. Body drawn: `DrawSHP(NATESLA, phase, ..., z=2, depth=cell.Level + Type+0x1548)`
18. Dispatcher called again with pass_flag=1
19. `this+0x6E7 == 0` → path taken → VXL extras function
20. Mission != 0x10 → skip construction reanim
21. `Type+0x16C5 Turret=yes` → VXL turret branch
22. BuildVXLTurretMatrix → Locomotion_Matrix → Matrix3x4_RotateZ(facing * DAT_007E4408)
23. `Matrix_ShearCol3ByCol0(Type+0x720 BarrelLength/8)` for barrel extrude
24. DrawVXL body (Type+0xB8 NATESLA.VXL), DrawVXL turret (Type+0xC0 NATES01.VXL — barrel)
25. Anim slot 0 TurretAnim (the charge glow) is handled elsewhere in Layer 2

**Spotlight (Type+0x154B HasSpotlight=yes for NATESLA):** The
BuildingLightClass at `this+0x600` updates the cell light map in
`BuildingClass::Update`, then is drawn as an additive overlay in
`Tactical_layer_animations`. DrawBody does nothing with it directly.

### 13.2 Allied ConYard, 3 upgrades, 50% HP, power off (GACNST)

1. `this+0x534 = 1` (damaged at ~50% HP; crosses ConditionYellow)
2. `this+0x660 = 0` (no power) — but DrawBody doesn't check this
3. `this+0x6E4 = 1` if `DamagedArt=` was parsed distinctly (GACNST has it
   in artmd.ini via damage frame range, but no DamagedArt override → 0)
4. `vtable+0x6C` → Type+0xC0 primary SHP (GACNST.SHP)
5. `GetCurrentFrame`:
   - `Type+0x16B7 Gate=no`, `Type+0x157B CanBeOccupied=no`
   - Not laser fence, not firestorm wall
   - Damaged + not gate + not mission==0x13:
     - `max(Frames1+Start1, Frames2+Start2, Frames3+Start3, Frames4+Start4) + phase`
     - Returns a damaged-animation frame
6. Body drawn at damaged frame
7. `Type+0x1518 != 0 && this+0x534 != 0` → damaged BibShape drawn under body
8. `Type+0x14EC` if ConYard has HealthyIdleAnim → no, CONSTRUCTION mission
   only → skipped
9. Dispatcher second pass: `Type+0x16C5 Turret=no` → no VXL draw
10. PowerUp1, 2, 3 anims (from `this->Anims[0..2]`) sort with Layer 2 and
    draw on top of the body at their `Type+0xF4C+0x34` XYZ offsets

**Power-off effect:** LowPower anim (slot 19) was spawned at OnPowerOff
time. It Y-sorts with Layer 2 and draws overlay "POWDOWN_A" frames on
top. The body itself is NOT tinted or darkened by DrawBody.

### 13.3 Opening/closing gate (NAGATE / GAGATE)

Assume: gate mid-opening, `GateStages=11`, timer 30% progress.

1. `vtable+0x184` Mission → 0x18 GUARD
2. `this+0x350` gate timer: `isActive=1, direction=0` (opening), `remaining=70%`
3. `FUN_004A5130` → true (is opening)
4. `remainingRatio = 0.30`, `frame_index = ftol(0.30 * 11) = 3`
5. `is_closing` false → skip subtract
6. `is_open` false → skip
7. `is_closed` false → skip
8. `frame_index (3) < GateStages (11)` → clamp skipped
9. `frame_index (3) >= 0` → clamp skipped
10. `GetHealthRatio() > ConditionYellow (0.5)` → healthy → `damage_offset = 0`
11. `final_frame = 0 + 3 = 3` (gate 27% open — frame 3 of 11)
12. `vtable+0x6C` → Type+0xC0 → GATE.SHP
13. DrawSHP at body depth

If health drops to 30% (below ConditionYellow), damage_offset becomes
`GateStages + 1 = 12`, so final_frame = 15 — the damaged gate row, frame 3.

**Fidelity trap:** if `ftol` is replaced by `as i32` cast in Rust without
proper rounding, the gate frame pick is off by one near the boundaries
(0.99 * 11 could round differently). gamemd.exe's `ftol` does
round-toward-zero with hardware FPU mode set by the engine.

---

## 14. Magic Constants and Clamps

| Constant | Purpose |
|----------|---------|
| `0xC6` (198) | Base draw X origin (pixel) |
| `0x1BE` (446) | Base draw Y origin (pixel) |
| `0x100` (256) | Leptons per cell |
| `0x100` offset | "Back up half a cell" from foundation center |
| `0x2D8` (728) | Z-threshold in `Tactical__AdjustForZ` — coords ≥ 0x2D8 get +1 adj |
| `0x15` (21) | Total Anims[] slots |
| `0x4E4` (1252) | vtable slot for VXL/extras DrawBody variant |
| `0x114` (276) | vtable slot for primary DrawBody |
| `0x104` (260) | vtable slot for Draw dispatcher |
| `0x1D0` (464) | vtable slot for Z-depth setup call |
| `0x160` (352) | vtable slot for IsIronCurtainActive |
| `0x184` (388) | vtable slot for GetCurrentMission |
| `0x6C` (108) | vtable slot for GetSHP (returns Type+0x9C or Type+0xC0) |
| `0x19D` | Per-AnimClass "dirty"/"skip" flag (set on sell/destroy) |
| `0x1701` | `Type.Invisible=` — if set, DrawBody returns without drawing |
| `0x1702` | `Type.OpensToNorthWest=` for VXL bunker-dock pass |
| `0x5B0..0x5C4` | Per-slot Anim charge flags (21 bytes) |
| `0x600` | Z-buffer test flag passed to CC_Draw_Shape |
| `0x601` | Z-buffer test + no-write flag |
| `0x800` | Cloak flag |
| `0x820` | Walls / special-draw combined flag |
| `0x2000` | Remap palette override flag (when `param_5 != -1`) |
| `0x4000` | Alt palette flag (when `param_6 != 0`) |
| `footprint >= 8` | Suppress g_BUILDNGZ_SHA for oversized buildings |
| `Rules+0x1700` | ConditionYellow (50% default) |
| `Rules+0x1708` | ConditionRed (25% default) |
| `Rules+0x18A4` | BerserkColor index |
| `Rules+0x18A8` | IronCurtainColor index |
| `Rules+0x18B0` | ForceShieldColor index |
| `Rules+0x1874` | ColorAdd table base (3 bytes per entry) |
| `GateStages + 1` | Gate-damage frame offset (NOT GateStages) |
| `GateStages - 1` | Max valid gate frame index (closed state) |

### Clamp order (gate frame)

```
if (frame >= GateStages) frame = GateStages - 1    // clamp high FIRST
if (frame < 0)           frame = 0                 // clamp low SECOND
```

Reversing these introduces a 1-frame window where a very close-to-1.0
ratio momentarily produces `GateStages`, breaking SHP bounds.

### Rounding

- `Math__ftol` is used for gate ratio → frame_index (truncation with
  FPU-mode-set semantics; NOT round-to-nearest)
- `Tactical__AdjustForZ` does `z * scale + 0.5` where scale comes from
  `DAT_00B0CD48` (runtime-initialized, default ≈ 0.0 early, set by Tactical
  init) and adds 1 if `z >= 0x2D8`
- Coord-to-screen uses ARITHMETIC right-shift by 8 with sign-correction:
  `(val + (val >> 31 & 0xFF)) >> 8`. This rounds TOWARD zero for negative
  values — critical because negative Y coords happen when the building is
  off the top of the tactical view.

---

## 15. Open Questions

1. **Lighting integration confidence (MEDIUM → HIGH).** I verified DrawBody
   doesn't read `+0x600` or `+0x614` directly, but didn't trace every
   possible callee-of-callee. A per-pixel dynamic-light system theoretically
   could be applied inside `CC_Draw_Shape` that was missed. TODO: decompile
   `CC_Draw_Shape` and search for `+0x600/+0x614` reads there.

2. **`+0x6E7` meaning.** The dispatcher uses `this+0x6E7 == 0` to gate the
   VXL/extras pass. Not in V2 doc layout. Likely "InTunnel" or
   "LandedAircraftSuppression" for aircraft-on-helipad. Needs a
   callers-of-`+0x6E7` sweep. The VXL pass CANNOT execute if `+0x6E7 != 0` —
   what in the game sets this flag?

3. **`Type+0x1518` BibShape vs `Type+0xBC` CoreBib.** V2 doc §3 lists two
   bib pointers. DrawBody reads `+0x1518` which is the damaged-only bib.
   The normal bib is the ground-tile smudge drawn in terrain Phase 1. Is
   `+0x1518` actually `DamagedBibShape=` in artmd.ini, or is it the
   BuildupShape for construction-end? Verify against
   `BIB_SYSTEM_GHIDRA_REPORT.md` — possible terminology drift.

4. **Construction-mission overlays `+0x14E4 / +0x14EC / +0x14FC / +0x1504`**.
   Four distinct SHP pointers inside DrawBody's construction branch:
   - `+0x14E4` = healthy-buildup?
   - `+0x14EC` = healthy-construction-complete anim overlay?
   - `+0x14FC` = damaged-buildup?
   - `+0x1504` = damaged-construction-complete anim overlay?
   They're indexed by an auxiliary flag passed on the stack (`ESP+0x13` =
   `cVar6 = (char)((uint)unaff_EBP >> 0x18)`). This value is read from the
   unaff_retaddr context — need to trace caller to determine what sets it.

5. **`DAT_00818CB0` and `DAT_00818CB4`** in the VXL extras path. These
   modulate the barrel-pitch wobble animation on charged/firing weapons.
   Runtime values unknown; likely set by a global "TacticalDrawPhase" tick
   counter. Not load-bearing for basic parity but could affect sub-frame
   barrel position.

6. **PowerUp anim DrawOrder when multiple PowerUp X/Y coincide.** The 21-
   slot Anims[] Layer 2 insertion order is "whoever was created first wins
   the tie". This is a first-to-install advantage. Parity implementations
   must replay the install order; random ordering will shift PowerUp1
   behind PowerUp2 in some frames.

7. **BarrelStartPitch frame bias confirmation.** Section 4 claims no
   per-facing lookup is done in DrawBody itself; the turret anim uses a
   quantization through the AnimClass. Verify by decompiling
   `AnimClass::DrawIt` for TurretAnim slot 9 and confirming
   `Type+0x1710` is read at that blit site (not at DrawBody).

---

## Sources

### Ghidra decompilations (all verified in live MCP session)

- `0x0043D290` BuildingClass_DrawBody (primary, 2021 bytes)
- `0x0043CEA0` Draw dispatcher (vtable+0x104, 386 bytes)
- `0x0043DA80` Building VXL/extras DrawBody sibling (vtable+0x4E4, 3362 bytes — created)
- `0x0043EF90` BuildingClass::GetCurrentFrame
- `0x004513D0` SHP source selector (vtable+0x6C dispatch)
- `0x00705E00` TechnoClass_DrawSHP (master blit wrapper, 1803 bytes)
- `0x0070E380` TechnoClass::ScaleByTemporalVisualPhase
- `0x0070E4B0` TechnoClass::ScaleByWarpInVisualPhase
- `0x0041BF40` TechnoClass::IsIronCurtainActive
- `0x005B3040` MissionClass::GetCurrentMission
- `0x0041BEA0` ObjectClass::Get_Cell_Packed
- `0x005F5F30` ObjectClass::GetHeight
- `0x004A5110 / 5130 / 51B0 / 51D0 / 52F0 / 51F0` Gate timer queries
- `0x00487950` IsShrouded wrapper
- `0x0065AD40` Cell-neighbor accessor
- `0x004BBC90` Surface color mode getter (returns `DAT_008205D0`)
- `0x0045EC90 / 0x0045ECA0` Foundation width/height lookups
- `0x006D1FE0` CellToPixel iso transform
- `0x006D20E0` Tactical::AdjustForZ
- `0x006F60D0` / `0x006F5190` DrawBehind / DrawExtras (sibling calls, context only)
- `0x0070ED80` ModifyCloakDrawFlags
- `0x004BAA80` DSurface::Constructor (for surface mode semantics)

### Existing docs cross-referenced

- `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` (layout §2/§3, vtable §4, anim slots §12)
- `BUILDING_ANIM_STATE_MACHINE.md` (21 anim slots, SetDamagedState, OnPowerOff)
- `BIB_SYSTEM_GHIDRA_REPORT.md` (bib context)
- `DRAW_ORDER_DEPTH_SYSTEM.md` (Phase 2 layer pipeline)
- `building-selection-brackets/SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md` (vtable slots for Draw/Extras)
- `IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md` (IC/FS tint, +0x1C4 semantics)
- `CLOAKING_VISUAL_PIPELINE.md` (CloakState fields)
- `TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md` (warp-phase scaling)

### INI files checked

- `ini/rulesmd.ini` — `[ColorAdd]` (3409), `IronCurtainColor` (626),
  `ForceShieldColor` (628), `BerserkColor` (627), `LaserTargetColor` (625)
- `ini/artmd.ini` — PowerUp1/2/3 anim block at building types,
  GateStages referenced by art.ini damaged frame ranges

### Binary memory inspected

- Vtable `0x007E3EBC..0x007E44A4` — slot layout verification
- `DAT_008205D0` — surface color mode
- `DAT_00B0CD48` / `DAT_007E1738` — Tactical::AdjustForZ scale/bias

### Not implemented (out of scope per plan)

- Per-building asset loading (LoadVisualAssets — v3 follow-up)
- AnimClass::DrawIt for slots 9/19/20 (TurretAnim / LowPower) —
  investigated only enough to confirm DrawBody does NOT draw them
- Full VXL pipeline for DrawVXL (vtable+0x444) — covered in
  `VXL_DRAW_MATRIX_GHIDRA_REPORT.md`
