# High Bridge Under-Deck Occlusion — Render Mechanism

**Date:** 2026-05-19
**Status:** PARTIALLY CORRECTED — see CORRECTION block below. The high-level mechanism
(Z-buffer depth test, two-pass terrain-then-objects) is correct, but the specific claim
that `cell+0x10E` carries the `heightLevel+4` differentiation at map load is WRONG.
**Active in YR:** Yes — high bridges are a live YR feature; this code path fires every
frame any unit stands beneath a built-up bridge in a skirmish.

## CORRECTION 2026-08-14 — scanline mechanism resolved

The open body-blitter question is now closed on the active stock format-3 route.
`CellClass__DrawOverlay_Body @ 0x0047F6A0` passes an explicit flag-`0x10` gate
of zero, so effective flags remain `0x4E00`; the separate native row base is
`-2 - 15 * (signed cell.level + 4)`. Stock bridge body frames select
`Blitter_selector_extended @ 0x00490E50` slot `+0x158`, whose vtable `+4` leaf
at `0x004990E0` reads stored Z, rejects equality/farther candidates, and writes
accepted color plus Z. `Extended_SHP_blitter @ 0x00437A10` uses gradient entry 0,
changing candidate Z by `-1` for every full-canvas scanline; the stock optional
per-pixel Z table is neutral. This, not `cell+0x10E`, is the verified body-row
mechanism. Sections below that retain the older `0x4E10`, standard-blitter,
`<=`, or `cell+0x10E` narrative are historical and superseded by this block.

---

## CORRECTION 2026-05-19 — `cell+0x10E` IS NOT `heightLevel+4` at map load

The follow-up doc
[`FUN_00483E30_BRIDGE_Z_AT_MAP_LOAD_GHIDRA_REPORT.md`](../01-assets-map-load-overlay/FUN_00483E30_BRIDGE_Z_AT_MAP_LOAD_GHIDRA_REPORT.md)
verified that at map load:

- `cell+0x10E` is set to the **literal constant `1000`** via `FUN_00483E30 @
  0x00483E30`, called from `MapClass__InitCellAttributes @ 0x00568BB0` and the
  iterator `FUN_004AE450`. The 6th argument `param_7` is hardcoded `1000` at both
  call sites; the function stores it verbatim with no `heightLevel` read and no `+4`.
- **`cell+0x10A`, `cell+0x10C`, and `cell+0x10E` are all `1000` at map load** and
  remain equal until a LightningStorm or PsychicDominator triggers
  `Cell_ComputeZAdjust @ 0x00484680`, which is the ONLY function that applies the
  `heightLevel + 4` formula to `+0x10E`. In a normal YR skirmish with no superweapon
  active, `+0x10E` stays at `1000` for the entire match.

**Implication for this doc:** §4-§7's "bridge Z is larger because `+0x10E` uses
`heightLevel + 4`" cannot be the source of under-deck occlusion in normal play —
because in normal play `+0x10E` equals `+0x10C`. Under-deck occlusion clearly happens
every match (visually verifiable), so the actual per-pixel Z differentiation must be
derived from something other than the static `+0x10E` field. The two surviving
candidates flagged by the original investigation (§11 Open Question 1):

1. The **Y-adjust parameter** to `CC_Draw_Shape` (`(heightLevel + 4) * -15 - 2` for
   bridge cells, ~60 px upward) — likely interpolated to per-pixel Z by the scanline
   blitter from screen-Y rather than from the cell-anchor `+0x10E`.
2. **Draw-order within layers** — bridge pixels committed in the terrain pass may
   suppress object-pass pixels through the blitter, even at equal Z, depending on
   the scanline-blitter's per-pixel comparison rule.

The original §11 Open Question 1 ("scanline blitter functions not fully decompiled")
is the real next step. Until that's resolved, sections §4-§7 below should be read
as "these fields and call sites exist, but the static `+0x10E - +0x10C` differential
they imply is NOT present in normal play."

---

## 1. Headline

**Mechanism: Z-buffer depth test (two-pass terrain-then-objects).** A unit standing
beneath an intact high bridge appears occluded because the bridge-deck SHP overlay,
drawn in the earlier terrain pass, produces a larger Z value at the per-pixel scanline
level — **exact derivation still under investigation** (see CORRECTION block above). The
unit's scanline blitter, running in a later pass, tests Z per-pixel and discards pixels
where the buffer already holds a larger-than-unit Z value. The bridge appears opaque in
front of the unit.

There is no special clip rect, no draw-order-only occlusion, and no explicit "under
bridge" flag that hides the unit. **Originally hypothesized to come from the `+0x10E`
field carrying a `heightLevel+4` bonus, but that turned out to be wrong at map load —
see CORRECTION above. The per-pixel Z derivation is the open question.**

---

## 2. Z-Buffer Infrastructure

**`g_ZBuffer`** — 16-bit per-pixel circular circular buffer, one entry per screen pixel in
the viewport. Initialized to `0xffff` (max / "far") each frame via `ZBuffer_rect_clear`
at `0x007bcf50` called from `TacticalClass_Draw`.

**Key constants (verified via `ZBuffer_constructor @ 0x007bc970`):**
- Each pixel entry is 2 bytes (`width * height * 2` allocation)
- Fill value = `0xffff`
- Smaller Z value = closer to the camera (conventional "less-than" depth test)

**`ZBuffer_scanline_ptr @ 0x007bd130`** — called from `Standard_SHP_blitter` to resolve
circular buffer offset for each scanline.

---

## 3. Two-Pass Render Architecture

`TacticalClass_Draw @ 0x006d3d10` orchestrates rendering in two logical passes:

| Pass | `param_3` | What runs |
|------|-----------|-----------|
| Terrain pass | 1 or 3 | `Tactical_layer_overlays @ 0x006d3040` → bridge SHP overlays → **Z written** |
| Object pass  | 2 or 3 | `Tactical_ObjectRenderingLoop @ 0x006d8db0` → units/buildings → **Z tested** |

The terrain pass always precedes the object pass. When both run in the same frame
(param_3 = 3), terrain Z values are already in the buffer when units render.

Verified by decompiling `TacticalClass_Draw @ 0x006d3d10`: the guard
`if ((param_3 != 1) && (param_3 != 3)) goto LAB_006d4582` skips the terrain layers,
and the call to `Tactical_ObjectRenderingLoop` is in the `param_3 == 2 || param_3 == 3`
branch that follows.

---

## 4. Bridge Overlay Z Computation

> **CORRECTION 2026-05-19:** `Cell_ComputeZAdjust @ 0x00484680` does NOT run at map
> load. It runs only per-tick during LightningStorm / PsychicDominator. The map-load
> initializer is `FUN_00483E30 @ 0x00483E30`, which writes the literal `1000` to
> `+0x10A`, `+0x10C`, and `+0x10E` (all three equal). See
> [`FUN_00483E30_BRIDGE_Z_AT_MAP_LOAD_GHIDRA_REPORT.md`](../01-assets-map-load-overlay/FUN_00483E30_BRIDGE_Z_AT_MAP_LOAD_GHIDRA_REPORT.md).
> The formula below is real but only active during superweapon ticks.

### `Cell_ComputeZAdjust @ 0x00484680` (decompiled, this session)

Pre-computes three Z fields per cell **during LightningStorm / PsychicDominator
ticks** (NOT at map load):

| Field | CellClass offset | Computed from | Used by |
|-------|-----------------|---------------|---------|
| `cellZAdjust_top` | `+0x10a` | `gradient * heightLevel - offset` | Buildings, normal overlays |
| `cellZAdjust` | `+0x10c` | `(+0x10a) * intensityFactor >> 16` | TMP tiles, standard units |
| `cellZAdjust_bottom` | `+0x10e` | `gradient * (heightLevel + 4) - offset`, then scaled | **Bridge overlay body only** |

The **hardcoded `+4`** in the `cellZAdjust_bottom` formula was hypothesized to be
the key differentiator. **CORRECTION 2026-05-19:** this formula does NOT run at map
load — at map load all three fields are equal (literal `1000`). The `+4` bonus is
only applied during LightningStorm / PsychicDominator ticks. In a normal skirmish,
`+0x10E == +0x10C == +0x10A == 1000`, so this formula cannot be the source of normal-play
occlusion. The actual per-pixel Z differentiation source is unresolved (see CORRECTION
block at top of doc).

Verified: `*(short *)(param_1 + 0x10e) = *(short *)(param_1 + 0x10e) + sVar5;` where
`sVar5 = gradient * (heightLevel + 4) - offset`, directly in `Cell_ComputeZAdjust` body.

---

## 5. Bridge Overlay Draw Path (Z Written)

> **CORRECTION 2026-05-19:** The Z value passed to `CC_Draw_Shape` (the cell's
> `+0x10E` field) is **equal to** the non-bridge `+0x10C` value in normal play
> (both are `1000`). The bridge-vs-ground Z separation that occlusion depends on
> does NOT come from this field at map load. The per-pixel Z must be derived
> downstream (Y-adjust → scanline-blitter interpolation, or some other path).
> The function-and-branch tree below is still real and correct — but the
> implication "bridge Z > ground unit Z because of `+0x10E`" is wrong in normal play.

### `CellClass__DrawOverlay_Body @ 0x0047f6a0` (decompiled, this session)

Activated when `CellClass+0x140 & 0x80` (bridge structural flag) is set.

The bridge-overlay body draw call inside the function:
```c
CC_Draw_Shape(piVar5, uVar7, &local_10, piVar2, 0x4e00, 0,
              iVar8 * -0xf + -2,           // Y-adjust = (height+4) * -15 - 2
              0,
              (int)*(short *)(param_1 + 0x10e),  // Z = cellZAdjust_bottom
              0, 0, 0, 0, 0);
```

- **Flags = `0x4e00`**: does NOT include `0x10` directly, but `CC_Draw_Shape` at
  `0x004aed70` ORs `0x10` (Z-buffer enable) when `param_7 != 0`. Since `iVar8 * -0xf +
  -2` is always non-zero for elevated cells, Z-buffer is always activated.
- **Z value = `cellZAdjust_bottom` (`+0x10e`)**: uses `heightLevel + 4` in its
  derivation — always larger than the same cell's `cellZAdjust` (`+0x10c`).
- `iVar8 = *(char *)(param_1 + 0x11b) + (*(uint *)(param_1 + 0x140) >> 7 & 1) * 4`
  — this is `heightLevel + (is_bridge_flag * 4)`. For a bridge cell the +4 shifts
  the Y screen position 60px upward (4 × 15px).

### `FUN_004802a0 @ 0x004802a0` → `FUN_00547230 @ 0x00547230` (railing/pavement path)

The railing/pavement overlay uses flags `0x4601` and passes the height-based Y-adjust
as the last argument to `CC_Draw_Shape`, also activating Z-buffer writes.

---

## 6. Ground Unit Draw Path (Z Tested)

> **CORRECTION 2026-05-19:** "This is numerically smaller than the bridge's
> `cellZAdjust_bottom`" is FALSE in normal play — both are `1000` at map load.
> The narrative below assumes the `heightLevel + 4` differential is live, but
> `Cell_ComputeZAdjust` only fires under LightningStorm / PsychicDominator. The
> conclusion (under-deck unit pixels get occluded by bridge) is empirically
> correct, but the cited cause (`+0x10E > +0x10C`) is not the actual mechanism
> in normal skirmish. The real per-pixel Z derivation is unresolved.

### `TechnoClass_DrawSHP @ 0x00705e00` (decompiled, this session)

For non-building units (no garrison), the Z value passed to `CC_Draw_Shape` is supplied
from the calling draw function. Tracing `BuildingClass_DrawBody @ 0x0043d290`:

```c
iVar16 = (int)*(short *)(iVar8 + 0x10a) + (int)*(short *)(param_1->Type + 0x1548);
// iVar8 = CellClass* for the unit's occupied cell
// +0x10a = cellZAdjust_top (derived from heightLevel WITHOUT the +4)
```

A unit at ground level beneath a bridge occupies the cell at ground `heightLevel`. Its
Z is from `cellZAdjust` or `cellZAdjust_top` (both derived without the bridge +4). This
is **numerically smaller** than the bridge's `cellZAdjust_bottom` for the same cell.

In `CC_Draw_Shape → Standard_SHP_blitter`: the per-pixel scanline routine writes the
pixel only when the new Z ≤ the Z already stored in `g_ZBuffer` (standard depth-less
test). Because the bridge-deck Z (`+0x10e`, larger number) was written first by the
terrain pass, the ground unit's Z (smaller number from `+0x10c`) **fails the test** and
its pixels are rejected wherever the bridge overlay has already drawn.

Result: the unit appears occluded by the bridge deck.

### For units ON the bridge

Units on the bridge surface have `IsOnBridge_ForFiring` true. Their Y-adjust is also
shifted by the `+4` factor via the locomotion draw path (verified in BRIDGE_SYSTEM.md §
"Z-Buffer: Units On vs Under Bridge"). Their Z therefore matches the bridge-deck Z
rather than being below it, so they render on top.

---

## 7. Load-Bearing Branch

> **CORRECTION 2026-05-19:** The branch `if ((cell+0x140) & 0x80)` and the
> `+0x10E` vs `+0x10C` field selection are real and live in the binary. But
> because both fields hold `1000` at map load (and remain equal in normal play),
> the field selection does NOT produce the Z differential this section claims it
> does. The `+4` hardcode IS present in `Cell_ComputeZAdjust`, but that function
> doesn't run at map load — it runs only during LightningStorm / PsychicDominator
> per-tick updates. In normal skirmish play, the branch chooses the same numeric
> value either way. The actual per-pixel Z mechanism is unresolved.

**In `CellClass__DrawOverlay_Body @ 0x0047f6a0`:**

```c
if ((*(byte *)(param_1 + 0x140) & 0x80) != 0) {
    // Bridge-overlay draw path:
    ...
    CC_Draw_Shape(..., (int)*(short *)(param_1 + 0x10e), ...);
    //                                           ^^^^
    //                      cellZAdjust_bottom: uses heightLevel+4
    ...
    return;
}
// Non-bridge overlay draw path:
...
CC_Draw_Shape(..., (int)*(short *)(param_1 + 0x10c), ...);
//                                         ^^^^
//                  cellZAdjust: uses heightLevel (no +4)
```

The branch `if ((*(byte *)(cell + 0x140) & 0x80) != 0)` at approximately `0x0047f700`
is the single point that selects `+0x10e` over `+0x10c`. All bridge occlusion of
under-deck units flows from this one field selection.

**The `+4` hardcode in `Cell_ComputeZAdjust @ 0x00484680`** is the ultimate source:

```c
sVar5 = gradient * (heightLevel + 4) - offset;   // +4 is hardcoded
*(short *)(param_1 + 0x10e) += sVar5;
```

---

## 8. Mechanism Classification

| Option | Conclusion |
|--------|-----------|
| (a) Z-buffer depth test | **YES — primary mechanism** |
| (b) Draw-order (bridge drawn over unit in same pass) | NO — they are in separate passes |
| (c) Explicit clip rect | NO — no clip rect logic found |
| (d) Other | Partial: the `+4` height bonus for bridge cells is the formula source |

The engine uses a **shared Z buffer across both terrain and object passes**. Bridge
overlays write larger Z values (from `cellZAdjust_bottom`) during terrain pass. Ground
units under the bridge write smaller Z values during the object pass. The depth test
rejects ground-unit pixels where bridge Z is already present.

---

## 9. TS-vs-YR Classification

**Active in YR: Yes.**

- High bridges exist in YR maps, ship with YR campaigns, and function identically to how
  this report describes in normal YR skirmish play.
- `CellClass__DrawOverlay_Body @ 0x0047f6a0` and `Cell_ComputeZAdjust @ 0x00484680`
  are on the hot render path every frame for any map with bridge overlays.
- The Z-buffer infrastructure (`g_ZBuffer`, `Standard_SHP_blitter`, `ZBuffer_rect_clear`)
  is unconditionally used in the tactical view — no TS-only gate.
- No `SpecialFlags` gate or `FogOfWar`-style opt-in was found for this path.

---

## 10. Rust Implementation Notes

To replicate this in Rust:

1. Pre-compute `cell_z_adjust_bottom` per cell at map load using `heightLevel + 4` in
   the gradient formula (same as non-bridge cells do, just with +4).
2. The terrain render pass must write Z values. For bridge-overlay body draws, use
   `cell_z_adjust_bottom` as the Z parameter.
3. For ground units in the object pass, use the standard `cell_z_adjust` (no +4).
4. The per-pixel scanline must test Z: write pixel only when `new_z <= buffer_z`.
5. No special "unit is under bridge" flag needed — the math handles it implicitly.

---

## 11. Open Questions

1. **Exact blitter Z comparison direction** — confirmed as "smaller = closer" based on
   init to `0xffff` max and standard depth-test logic, but the assembly scanline blitter
   functions (vtable callbacks from `Standard_SHP_blitter`) were not fully decompiled.
   The `BRIDGE_SYSTEM.md §"Z-Buffer: Units Under vs On Bridge"` claim is consistent
   with this interpretation but the scanline assembly blitters warrant a future check.
2. **Voxel units under bridge** — `TechnoClass__Render @ 0x00706ed0` is the voxel
   render path. It uses `FUN_004af2a0` (not `Standard_SHP_blitter`) for the final blit.
   Whether voxel units are occluded by bridge Z in the same way was not verified this
   session. Likely yes (same Z buffer), but confirm before implementing.
3. **Partial occlusion at bridge edge cells** — for cells at the edge of a bridge where
   only part of the sprite is under the deck, the Z comparison naturally handles this
   per-pixel. No special edge logic was found, consistent with the claim.

---

## 12. Verified Facts (Top 5)

1. **`Cell_ComputeZAdjust @ 0x00484680`** writes `CellClass+0x10e` using `heightLevel + 4`
   (hardcoded +4). The other two Z fields (`+0x10a`, `+0x10c`) do NOT use the +4.
   Verified by decompiling `0x00484680` this session.

2. **`CellClass__DrawOverlay_Body @ 0x0047f6a0`**, bridge branch (`cell+0x140 & 0x80`),
   calls `CC_Draw_Shape` with Z = `*(short *)(cell + 0x10e)` (`cellZAdjust_bottom`).
   Non-bridge overlays use `*(short *)(cell + 0x10c)`. Verified by decompiling `0x0047f6a0`.

3. **`TacticalClass_Draw @ 0x006d3d10`** runs terrain layers (incl. bridge overlays) in
   pass 1 and `Tactical_ObjectRenderingLoop` in pass 2 — bridge Z is written before
   unit Z is tested. Verified by decompiling `0x006d3d10` this session.

4. **`g_ZBuffer` is initialized to `0xffff` per frame** via `ZBuffer_rect_clear @
   0x007bcf50`, called from `TacticalClass_Draw`. `ZBuffer_constructor @ 0x007bc970`
   confirms 16-bit per-pixel, `0xffff` fill. Verified by decompiling both functions.

5. **`CC_Draw_Shape @ 0x004aed70`** enables Z-buffer mode (ORs `0x10` into flags) when
   `param_7 != 0` (Y-adjust non-zero). Bridge cells always have non-zero Y-adjust, so
   bridge overlays always participate in Z-buffer occlusion. Verified by decompiling
   `0x004aed70`.

---

## 13. Ghidra Functions Decompiled (This Session)

- `CellClass__DrawOverlay_Body @ 0x0047f6a0`
- `Cell_ComputeZAdjust @ 0x00484680`
- `TacticalClass_Draw @ 0x006d3d10`
- `Tactical_ObjectRenderingLoop @ 0x006d8db0`
- `CC_Draw_Shape @ 0x004aed70`
- `Standard_SHP_blitter @ 0x004373b0`
- `Blitter_selector @ 0x00490b90`
- `ZBuffer_constructor @ 0x007bc970`
- `ZBuffer_rect_clear @ 0x007bcf50`
- `ZBuffer_row_fill @ 0x007bcfb0`
- `ZBuffer_scanline_ptr @ 0x007bd130`
- `Tactical_layer_overlays @ 0x006d3040`
- `FUN_006d7c00 @ 0x006d7c00` (overlay dispatch for cells in dirty-rect)
- `FUN_004802a0 @ 0x004802a0` (cell overlay → railing draw)
- `FUN_00547230 @ 0x00547230` (railing/pavement CC_Draw_Shape call)
- `TechnoClass_DrawSHP @ 0x00705e00`
- `TechnoClass__Render @ 0x00706ed0`
- `UnitClass__Draw_Body_And_Turret @ 0x0073c5f0`
- `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073b140`
- `BuildingClass_DrawBody @ 0x0043d290`
- `AircraftClass__Draw_It @ 0x004144b0`
- `ObjectClass__DrawIt @ 0x005f4b10`
- `ObjectClass__GetYSort @ 0x005f6bd0`
- `Tactical__AdjustForZ @ 0x006d20e0`
- `LocomotionClass__Draw_Point @ 0x0055a8c0`
- `FUN_004d1890 @ 0x004d1890` (dirty-rect draw loop, case 0x14 = bridge)

*End of report.*
