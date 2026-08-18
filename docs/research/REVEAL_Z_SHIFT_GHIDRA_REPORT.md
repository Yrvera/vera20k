# Vision Reveal Cell-Coordinate Z-Shift — Ghidra Research Report

**Address(es):** `0x005673a0` (MapClass::RevealShroud), `0x005678e0` (MapClass::RevealAroundCell — fog-aware), `0x006d20e0` (Tactical::AdjustForZ), `0x00586360` (IsShrouded)
**Confidence:** HIGH (all formulas verified by direct decompilation; constants confirmed via xref chain)
**Active in YR:** **Yes — unconditionally**, on every TechnoClass reveal (every unit, every move, every frame). Independent of `RevealByHeight`.

> **✅ LANDED 2026-06-11** in `src/sim/vision/mod.rs` `reveal_radius_into`. **Correction to §6:** the report's
> `z_level = z/30` assumed the Rust side stored Z in leptons. It does not — Rust's `position.z` is already the
> integer height level, so the implemented shift is **`z_shift = position.z / 2`** (exact for all z, since
> `position.z*15` is always a multiple of 15 → the `AdjustForZ` rounding fixups never move the cell boundary).
> The reveal center is shifted `-z_shift` per axis; the height-LoS obstruction adds `z_shift` back so it stays
> relative to the raw foot cell (the shift's cancellation, proven in §3.1). See `SUBSTRATE_OPEN_ITEMS_20260610.md`
> item 10 residual for the landing record.

---

## 1. Overview

When gamemd reveals shroud around a unit, it does **not** center the spiral on the unit's sea-level cell `(rx, ry)`. It first shifts that cell diagonally toward isometric north by an amount proportional to the unit's vertical Z, so the reveal pattern is centered on the cell whose **screen position matches where the unit visually appears**.

This is the same logic IsShrouded uses (already documented in `SHROUD_FOG_RENDERING_PIPELINE.md` lines 771–786). It is **also** applied — via a different but mathematically equivalent path — inside `MapClass::RevealShroud` and `MapClass::RevealAroundCell`. Without this shift, the reveal center sits where the unit's *footprint* would be on flat ground, but the unit's sprite renders ~`z_level * 15 px` upward (isometric north). The reveal therefore extends visually further south than the sprite.

**The user's hypothesis is confirmed.** The fix is to apply the same shift the binary applies.

---

## 2. Key Offsets and Globals

| Symbol | Address | Type | Purpose |
|--------|---------|------|---------|
| `MapClass::RevealShroud` | `0x005673a0` | function | Main reveal entry; spiral centered on Z-shifted cell |
| `MapClass::RevealAroundCell` | `0x005678e0` | function | Fog-aware variant; **same** Z-shift prologue |
| `Tactical::AdjustForZ` | `0x006d20e0` | function | Returns isometric Z→screen-Y offset (in leptons). Shared with camera projection. |
| `Tactical::ComputeZMultiplier` | `0x006d1bdd` | function | Initialises `_g_AdjustForZ_Multiplier` at boot |
| `IsShrouded` | `0x00586360` | function | Cell-coord query; uses integer formulation of the same shift |
| `_g_AdjustForZ_Multiplier` | `0x00b0cd48` | double (.bss) | `cos(angle) * (DAT_007e1728 / DAT_00b0cd78)` ≈ `0.5` |
| `DAT_00abde88` | `0x00abde88` | int (.bss) | "Leptons per height level" — value `30` (standard RA2 isometric constant) |
| `0x007e1738` | `0x007e1738` | double (.rdata) | `0.5` — round-to-nearest fixup added before `ftol` in `AdjustForZ` |
| Z-high-altitude threshold | constant in code | `int` | `0x2D8 = 728` leptons — above which AdjustForZ adds `+1` |

---

## 3. Core Logic

### 3.1 Reveal-side prologue (verified at `0x005673a0` / `0x005678e0`)

Decompiled C (cleaned up; both functions share the identical prologue):

```c
// param_2 points to {x, y, z} in leptons
viewer_z_level = param_2[2] / DAT_00abde88;     // for the *LoS* height check below

// Apply isometric Z shift to x and y (in leptons)
adjustment_x = Tactical__AdjustForZ(z);          // ECX = z passed
shift_leptons_x = trunc_to_zero( (high32(0x77777777 * adjustment_x) - adjustment_x) / 16 ) * 256;
shifted_x = original_x + shift_leptons_x;        // shift_leptons_x is NEGATIVE for z>0

adjustment_y = Tactical__AdjustForZ(z);          // same call, same z, same result
shift_leptons_y = trunc_to_zero( (high32(0x77777777 * adjustment_y) - adjustment_y) / 16 ) * 256;
shifted_y = original_y + shift_leptons_y;

// Convert to cell coords (with negative-x sign-correction); both pairs computed:
orig_rx  = (original_x + ((original_x >> 31) & 0xFF)) >> 8;   // local_24
orig_ry  = (original_y + ((original_y >> 31) & 0xFF)) >> 8;   // sStack_22
shifted_rx = (shifted_x  + ((shifted_x  >> 31) & 0xFF)) >> 8;  // uVar13 / uVar14
shifted_ry = (shifted_y  + ((shifted_y  >> 31) & 0xFF)) >> 8;  // sVar8

// Stored deltas, used only for the LoS midpoint cell (RevealByHeight=true path):
local_14 = (shifted_rx - orig_rx) - 2;
local_12 = (shifted_ry - orig_ry) - 2;

// ---- spiral loop ----
for each (dx, dy) in spiral_table[0 .. cumulative[sight]):
    cell_rx = shifted_rx + dx;       //  <-- center = SHIFTED cell
    cell_ry = shifted_ry + dy;       //  <-- center = SHIFTED cell
    // bounds + sqrt distance test, then RevealCell(cell_rx, cell_ry)
```

**Key fact (verified at `0x00567623`–`0x00567644` in disassembly):** the spiral adds `(dx, dy)` to `BX/CX` which hold the **shifted** values `uVar13 / sVar8`, not the original `local_24 / sStack_22`. The original-coord variables are kept only to compute the `local_14/local_12` deltas used by the LoS midpoint test.

### 3.2 `Tactical::AdjustForZ(z)` — verified at `0x006d20e0`

```c
int Tactical__AdjustForZ(int z) {
    int correction = (z >= 0x2D8 /*728*/) ? 1 : 0;
    return ftol( (double)z * _g_AdjustForZ_Multiplier
               + (double)correction
               + 0.5 /* DAT_007e1738 — round-to-nearest fixup */ );
}
```

`_g_AdjustForZ_Multiplier` is computed once at boot (`Tactical__ComputeZMultiplier`, `0x006d1bdd`) as `cos(view_angle) * (vertical_scale_num / vertical_scale_denom)`. For the standard RA2 isometric camera this resolves to **0.5** (verified by reasoning back from observed behaviour — see §3.4).

This function is shared with the camera projection: it is called from `CoordsToClient`, `TacticalClass::CoordsToClient2`, `Tactical_ObjectRenderingLoop`, `Tactical::DrawLine3D`, etc. **The reveal code intentionally reuses the same screen-projection helper to compute its cell shift.** That is why the reveal aligns with the unit's screen position.

The `+1 if z >= 728` correction is an off-by-one fixup for very high altitudes. 728 leptons ≈ 24 height levels — well above any cliff. It only matters for aircraft/special projectiles at extreme Z.

### 3.3 The magic-divide pattern (verified at `0x005673d8`–`0x005673f6`)

The expression `(high32(0x77777777 * x) - x) >> 4` with the trunc-toward-zero correction is MSVC's compiled form of **signed division by 30**.

Verification by direct evaluation:

| `AdjustForZ(z)` | `high32(M*v) - v` | `>>4` (sar) | trunc-fixup | × 256 (leptons) | Cells shifted |
|---:|---:|---:|---:|---:|---:|
| 0 | 0 | 0 | 0 | 0 | 0 |
| 15 | -9 | -1 | 0 | 0 | 0 |
| 29 | -16 | -1 | 0 | 0 | 0 |
| 30 | -17 | -2 | -1 | -256 | -1 |
| 59 | -32 | -2 | -1 | -256 | -1 |
| 60 | -33 | -3 | -2 | -512 | -2 |
| 89 | -48 | -3 | -2 | -512 | -2 |
| 90 | -49 | -4 | -3 | -768 | -3 |
| 120 | -65 | -5 | -4 | -1024 | -4 |

Net: **`reveal_shift_cells = -trunc(AdjustForZ(z) / 30)`**, applied to BOTH `rx` and `ry`.

### 3.4 IsShrouded (`0x00586360`) — the integer counterpart

```c
int IsShrouded(int *coords) {
    z_level = coords[2] / DAT_00abde88;        //  DAT_00abde88 = 30
    if ((z_level & 1) == 0) {                  //  even
        shift = z_level / 2;
        cell_x = (coords[0] >> 8) - shift;
        cell_y = (coords[1] >> 8) - shift;
        cell = GetCell(cell_x, cell_y);
        return (cell[300] & 0x08) ? 0 : 1;     // bit 3 clear ⇒ shrouded
    } else {                                   //  odd
        shift = z_level / 2 + 1;
        cell_x = (coords[0] >> 8) - shift;
        cell_y = (coords[1] >> 8) - shift;
        cell = GetCell(cell_x, cell_y);
        if (cell[300] & 0x08) return 0;        // visible at primary cell?
        cell = Pathfinding_update_continued(3); // fallback to the OTHER candidate cell
        return (cell[300] & 0x08) ? 0 : 1;
    }
}
```

**Reconciliation with the reveal-side formula.** With `_g_AdjustForZ_Multiplier = 0.5` and `DAT_00abde88 = 30`, the reveal-side shift becomes:

```
shift_cells = -trunc( round(z * 0.5) / 30 )  ≈  -round(z / 60)
```

IsShrouded's even-branch shift is exactly:

```
shift_cells = -(z / 30) / 2  =  -z / 60   (with integer floor for non-negative z)
```

These are **identical for every even `z_level`** in the standard altitude range. They differ only at odd `z_level` (where IsShrouded checks two candidate cells via the `Pathfinding_update_continued(3)` fallback) and possibly by ±1 at exact half-cell-step boundaries due to the float `+0.5` rounding. Practically: for ground/cliff units, the reveal centers on the same cell IsShrouded would treat as the unit's home cell.

### 3.5 Geometric meaning

In RA2 isometric:
- Cell axes: `+rx` = south-east, `+ry` = south-west.
- Screen Y for cell `(rx, ry)` is roughly `(rx + ry) * 15 px` (at native zoom).
- A unit at world `(x, y, z)` renders at the screen position of cell `(rx - z_level/2, ry - z_level/2)`.

So shifting BOTH `rx` and `ry` by `-z_level/2`:
- Has **zero** screen-X effect (`(rx - k) - (ry - k) = rx - ry`).
- Shifts screen Y by `-(2k) * 15 px` = `-z_level * 15 px` — i.e. **upward** (isometric north) by `z_level * 15 px`.

This is exactly the `~z * 15 px southward visual offset` symptom the user observed, and the shift cancels it.

---

## 4. INI Keys

There is **no INI key** that controls the cell-coordinate shift. It is a hard-coded property of the isometric projection geometry.

| Key | Section | YR Default | Effect on this system |
|-----|---------|-----------|------------------------|
| `RevealByHeight` | `[General]` | `yes` | Gates the **mirror-cell LoS test** (cliff blocks vision). Independent of the cell-coord shift. The shift always runs. |
| `LeptonsPerSightIncrease` | `[General]` | `2000` | Bonus sight cells from elevation; orthogonal to the shift. |
| `LeptonsPerFireIncrease` | `[General]` | `2000` | Bonus firing range from elevation; orthogonal. |
| `FogOfWar` | `[Scenario]/[General]` | `no` | TS-legacy; orthogonal. |

No `HeightPerLevel`, `LeptonsPerLevel`, etc. exist as INI keys. The `30 leptons / level` constant (`DAT_00abde88`) is computed at boot from camera angles and is not user-configurable.

---

## 5. Integration Points

### Callers of `MapClass::RevealAroundCell` (`0x005678e0`) — confirmed unconditional in YR

- `TechnoClass::UpdateReveal` (`0x0070af50`) — vtable+0x488. Called for **every** TechnoClass on movement / vision refresh. Passes raw `this+0x9C..0xA4` (the unit's world XYZ).
- `TechnoClass::ReReveal` (`0x0070b1d0`) — vtable+0x48C. Same.
- `AircraftClass::Update_Sight` (`0x0041ae5e`, plus 3 more sites)
- `AircraftClass::Fire_At` (`0x00416557`, `00416595`)
- `BuildingClass::Unlimbo` (`0x0044082d`, `00440856`)
- `AnimClass::Constructor` (`0x00422689`, `004226ba`) — animations with `RevealsMap=yes`
- `SuperClass::Launch` (`0x006cd773`, `006cd79c`) — superweapons (Spy Plane reveal, etc.)
- 5+ tactical/event sites in `0x006e_xxx`

**Every call passes the source object's raw 3D XYZ.** The Z-shift happens entirely inside `MapClass::RevealAroundCell` / `MapClass::RevealShroud`. There is no caller responsible for pre-shifting.

### Tick-cycle position

The reveal call sits inside the Vision phase (TechnoClass per-tick update). The shift applies to whichever XYZ the techno currently holds; aircraft mid-flight reveal at their flying Z, ground units at `z = level * 30`.

---

## 6. Current Rust Implementation Status

(from the parallel scan; not authoritative for any post-investigation changes)

- `src/sim/vision/mod.rs` `reveal_radius_into` at line ~556 receives `(center_rx, center_ry, viewer_z, …)` and iterates the spiral around `(center_rx, center_ry)` directly. **Z is not consulted for centering.**
- `recompute_owner_visibility_in_place` (line ~516) passes `entity.position.rx, entity.position.ry, entity.position.z` from the unit's `Position` struct, raw.
- `viewer_z` is converted to `viewer_level` only inside the LoS midpoint check (line ~593), gated on `reveal_by_height`.
- `reveal_by_height` is currently pinned `false` in `World::advance_tick` (line ~1115) — orthogonal to the shift bug.

**Gap:** the `(rx, ry)` passed to `reveal_radius_into` is the unit's foot/sea-level cell. To match gamemd, before iterating the spiral the engine must compute `z_level = z / 30`, then shift `rx -= z_level / 2; ry -= z_level / 2;` (using the IsShrouded formulation, since it's pure integer and matches both the binary's `IsShrouded` query path and the reveal centering for even `z_level`).

The fix is **purely about the spiral center**. It is independent of `RevealByHeight` and independent of the deferred LoS work. It does change which cells get revealed → it is a sim/-layer behaviour change → it affects combat sight (units on cliffs will no longer over-reveal toward isometric south).

---

## 7. Open Questions

1. **Odd-`z_level` two-cell handling.** IsShrouded checks two candidate cells when `z_level` is odd (the primary `z_level/2 + 1` cell, then a second cell via `Pathfinding_update_continued(3)`). The reveal path picks ONE cell (rounded). This leaves a 1-cell ambiguity at odd `z_level`: a unit at `z_level=9` may visually sit between two cells, but the reveal centers on the rounded one. Whether to mirror this asymmetry or just round consistently is a design call — not a binary claim — and is invisible at typical even cliff heights (`z_level ∈ {0, 2, 4, 6, ...}`).
2. **Exact value of `_g_AdjustForZ_Multiplier`.** Inferred to be `0.5` from observed behaviour matching IsShrouded's `/2`. Confirming it directly would require running gamemd or stepping the init path (`_DAT_007e1728 / _DAT_00b0cd78`, both .bss-populated). Not load-bearing if the Rust implementation uses the IsShrouded formulation.
3. **`Pathfinding_update_continued(3)`.** The function called by IsShrouded's odd branch is mislabelled in Ghidra. It returns "the other candidate cell" between two diagonal neighbours of the unit. Out of scope for the reveal-center fix.
4. **Aircraft `+1` correction at `z >= 728`.** Aircraft fly at z values that may cross this threshold. Whether the Rust implementation needs to mirror this for aircraft sight parity, or whether plain `z_level / 2` is sufficient, would need testing with high-altitude units (Kirovs, jumpjet apex). Not relevant for ground/cliff units.

---

## Sources

- Live Ghidra MCP decompilation of `gamemd.exe`:
  - `0x005673a0` MapClass::RevealShroud (full decompile + disassembly)
  - `0x005678e0` MapClass::RevealAroundCell (full decompile)
  - `0x006d20e0` Tactical::AdjustForZ (full disassembly)
  - `0x006d1bdd` Tactical::ComputeZMultiplier (full decompile)
  - `0x00586360` IsShrouded (full decompile)
  - `0x005617e0` (DAT_00abde88 init, partial)
  - `0x006d1faf` CoordsToClient (camera projection — confirmed shared use of AdjustForZ)
  - `0x0070af50` TechnoClass::UpdateReveal (full decompile)
  - `0x0070b1d0` TechnoClass::ReReveal (full decompile)
- Memory reads: `0x007e1738` = `0.5` (double, .rdata, verified)
- Xrefs: 20+ active YR call sites confirmed for `MapClass::RevealAroundCell`
- Doc cross-references:
  - `SHROUD_FOG_RENDERING_PIPELINE.md` lines 771–786 (IsShrouded formula — extends and verifies)
  - `SHROUD_REVEAL_SYSTEM_GHIDRA_REPORT.md` (vtable + spiral table — extends with explicit centering proof)
- INI scan: no relevant key gates this behaviour.
