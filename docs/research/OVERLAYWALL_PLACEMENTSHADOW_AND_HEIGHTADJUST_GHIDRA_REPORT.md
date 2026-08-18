# OverlayWall_PlacementShadow & WallOverlay_HeightAdjust — Ghidra Research Report

**Status:** COMPLETE  
**Date:** 2026-05-19  
**Extends:** WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT.md (addresses were listed but not decompiled)

---

## CRITICAL FINDING: "WallOverlay_HeightAdjust" is a Mislabeled Function

The Ghidra label `WallOverlay_HeightAdjust @ 0x0056BEC0` is **incorrect**. The function at
that address is not a height-adjustment routine for wall drawing. It is a **crate-spawning /
overlay-placement helper** used by map triggers and building placement. It:
1. Resolves a cell coordinate, finds a nearby passable cell via `FootClass__Find_Nearby_Passable_Cell`
2. Calls `CrateSlot__PlaceOverlayAndInitTimer` to plant a crate overlay on the resolved cell
3. Optionally sets `CellClass + 0x11E` (the wall frame byte) to `param_3` if `param_3 != 0x14`
4. Returns 1 on success, 0 on failure

It has no slope table, no Z-offset formula, and no relationship to wall visual rendering.
**The name is wrong; the function should be renamed to something like `PlaceOverlayAtCellNearby`.**

---

## 1. OverlayWall_PlacementShadow @ 0x006D5C50

### 1.1 Purpose

Draws the **ghost/preview shadow** of the wall drag-placement band — the translucent
green/red SHP that appears on the tactical map while a player is dragging to place a run of
walls. Called once per frame during wall placement.

Active in YR: **Yes** — fires every render frame while a player has a wall type selected
in the sidebar and is in placement mode (triggered by mode-id == 6, checked in caller).

### 1.2 Call Path

```
TacticalClass_Draw @ 0x006D3D10
  └── BuildingPlacement_OverlayRenderer @ 0x006D5030
        └── OverlayWall_PlacementShadow @ 0x006D5C50
              [also sibling calls for FirestormWall and LaserFencePost variants]
```

`BuildingPlacement_OverlayRenderer` is called from `TacticalClass_Draw` — the top-level
render function — so this fires every rendered frame while placement is active.

`OverlayWall_PlacementShadow` is reached only when:
- `g_UIModeLock` is non-null (the UI is in building-placement mode)
- The active building type's mode ID == 6 (overlay-wall type)
- `BuildingTypeClass + 0x16BE == 0` (not a Firestorm wall)
- `BuildingTypeClass + 0x16C0 == 0` (not a Laser Fence Post)
- `BuildingTypeClass + 0x2A8 != 0` (has an overlay wall type assigned)

### 1.3 Behavior

Signature: `void __thiscall OverlayWall_PlacementShadow(int this, char param_2, undefined4 param_3)`

- `param_1` (this): pointer to the building-type or placement object; used to read `+0xB0` and `+0xB4` (screen X/Y scroll offsets) and `+0x294` (overlay type index of the wall type being placed)
- `param_2`: boolean flag — when non-zero, the ghost is drawn in "invalid" color (palette index triggers `0xFFFF0000` OR into the draw flags, producing a 32-wide band count vs 0x20606 for valid)
- `param_3`: packed cell coord (CellX in low 16, CellY in high 16) for the drag origin

**Algorithm:**

1. Reads `g_RulesClass_Instance + 0x520` (cached pointer, likely `OverlayWallType` or a count struct) and exits immediately if its drag-count field (`+0xE54`) is zero.
2. Iterates over 4 even-indexed directions (step 2 per loop pass, so directions 0, 2, 4, 6 — N, E, S, W — using `local_38 & 7` and `g_DirectionOffsets` at `0x0089F688`):
   - From the drag origin cell, walks up to `(OverlayWallDragCountField >> 8)` cells along that direction
   - For each cell, checks connectivity: `CellClass + 0x44 == current overlay type index` AND `CellClass + 0x50 == g_PlayerPtr + 0x30` (same owner)
   - If the first adjacent owned-same-type cell is found after a gap, the gap length `iVar9` is the number of cells to shade
3. For each cell in the gap, calls `CC_Draw_Shape(g_PLACE_SHP, 0, &screenPos, &g_RadarViewportOffsetX, drawFlags, 0, slopeAdjust, ...)`:
   - `g_PLACE_SHP` is the placement-shadow SHP (a simple translucent cell overlay)
   - Screen X is computed as `(cellX * 0x100 + 0x80 >> 8) * 1` → the standard cell-center X
   - Screen X is offset by `param_1 + 0xB0` (tactical scroll offset) and further adjusted by `-0x1E` (30px to the left — accounts for SHP left edge trim or iso diamond offset)
   - Screen Y uses `Tactical__AdjustForZ()` output and `CellClass + 0x11B` (slope byte) to compute: `y = ((rawY >> 8) - Tactical__AdjustForZ()) - cVar4 * 15 - 1`
   - The second Y term `cVar3 * -0xf - 2` (using `CellClass + 0x11B` again) is passed as the `int` slope argument to CC_Draw_Shape
   - Draw flags: `0x020606` for valid placement, `0xFFFF0000 | 0x020606` for invalid (red tint)

**Key offsets read:**
- `CellClass + 0x44`: OverlayTypeIndex (used to match wall type)
- `CellClass + 0x50`: owner house pointer (filters to player-owned walls only)
- `CellClass + 0x11B`: slope byte (used for Y-position adjustment, not connectivity nibble; `+0x11E` is the frame byte)
- `g_DirectionOffsets @ 0x0089F688`: 8-direction delta table (i16 pairs, NE-indexed)
- `g_PLACE_SHP`: global handle to the placement shadow SHP asset
- `g_RadarViewportOffsetX / Y`: viewport clip rect origin (passed as render target)

**Does not draw a flat-color silhouette.** Uses `CC_Draw_Shape` with a SHP asset, so it
renders actual sprite frames with translucency applied via draw-flag bits in `0x020606`.

### 1.4 Cell_passability_building_placement Check

When scanning for the gap, if a cell is not the right overlay type or owner, the code
calls `Cell_passability_building_placement` to test buildability. If that returns 0, the
scan stops (the wall cannot be extended through impassable terrain). This means the
preview shadow respects placement validity.

---

## 2. WallOverlay_HeightAdjust @ 0x0056BEC0 (MISLABELED)

### 2.1 Actual Purpose

A **map-trigger action and building-placement helper** for placing crate/overlay items at
or near a target cell. Not related to wall height, slope, or rendering.

Active in YR: **Yes** — called from three active code paths.

### 2.2 Call Sites (all 3 verified)

| Caller | Address | Context |
|--------|---------|---------|
| `BuildingClass__Place_OccupyMap` | 0x00441F60 | When placing a wall building (`BuildingTypeClass + 0x1767 != 0`), places an overlay on the building's origin cell. Passes `param_3 = 0` (HasBib branch) or `param_3 = 0x14` (non-bib branch, which skips the `+0x11E` write) |
| `TriggerAction__Execute` | 0x006DD8B0 | **Case 0x6C** in the trigger-action switch. A map trigger action that places an overlay at a waypoint cell. Passes cell coord and an INI-configured overlay-type value |
| `UnitClass__ReceiveDamage` | 0x00737C90 | When a unit dies and `DAT_00a8b261 != 0` and the nearby-passable cell differs from the unit position, places an overlay at the passable landing spot. Gated by `DAT_00a8b261` (likely a global flag that controls this behavior) |

### 2.3 Behavior

Signature: `undefined4 __thiscall WallOverlay_HeightAdjust(int param_1_unused, undefined4 param_2_cellCoord, int param_3_overlayValue)`

- `param_1` (this): not used in body — treated as a lookup context but the actual work uses globals
- `param_2`: packed cell coordinate (same format as PlacementShadow — low 16 = X, high 16 = Y); validated against `g_CellArray_Base` bounds (0..0x3FFFF)
- `param_3`: value to write to `CellClass + 0x11E` (the wall frame byte) — skipped if `param_3 == 0x14` (decimal 20); the constant 0x14 acts as a "don't overwrite" sentinel

**Algorithm:**

1. Bounds-checks the cell index (`param_2._2_2_ * 0x200 + (short)param_2`) against [0, 0x3FFFF]; falls back to a sentinel cell `DAT_00ABDC50` if out of range
2. Checks `CellClass + 0xEC == 2` (likely `CellClass.LandType == Water` or similar) — if true, uses passability type `5` (bridge/water), else type `1` (ground) when calling `FootClass__Find_Nearby_Passable_Cell`
3. Searches `param_1 + 0x164` upward (stride 8 bytes per entry, up to 256 entries) for a slot where two shorts match `DAT_00ABD480` — this appears to be a free-slot scan in a crate-slot list on the owning object
4. If a slot is found, calls `CrateSlot__PlaceOverlayAndInitTimer(&resolvedCell)` to plant the overlay/crate
5. On success, if `param_3 != 0x14`, writes `(char)param_3` to the resolved cell's `+0x11E` (wall frame byte override)
6. Returns 1 on success, 0 if no slot found or placement failed

**Key offsets read:**
- `CellClass + 0xEC`: land/surface type (2 = water/bridge branch)
- `CellClass + 0x11E`: wall frame byte (written on success if param_3 != 0x14)
- `param_1 + 0x164`: crate-slot array start
- `DAT_00ABD480`: free-slot sentinel value
- `DAT_00ABDC50`: fallback sentinel cell for out-of-bounds coordinates

---

## 3. Summary Table

| Property | OverlayWall_PlacementShadow (0x006D5C50) | "WallOverlay_HeightAdjust" (0x0056BEC0) |
|----------|------------------------------------------|------------------------------------------|
| Label accurate? | Yes | **No — deeply misleading** |
| Actual role | Render: wall drag-placement ghost shadow | State: crate/overlay placement at nearby passable cell |
| Called from | Render pipeline (TacticalClass_Draw chain) | Placement (BuildingClass::Place_OccupyMap), triggers (case 0x6C), damage handler (UnitClass::ReceiveDamage) |
| Per-frame? | Yes, while wall placement is active | No — event-driven |
| Reads CellClass+0x11E? | No (reads +0x11B for slope) | Yes — **writes** it on success |
| Reads g_DirectionOffsets? | Yes (0x0089F688) | No |
| Height/slope logic? | Yes, via CellClass+0x11B and Tactical__AdjustForZ() | None |
| Active in YR | Yes (every frame of wall placement) | Yes (building placement + triggers + unit death) |

---

## 4. Open Questions

- What exactly is `DAT_00ABD480` (the crate-slot free sentinel)? Its size and source need tracing in CrateClass or related structs.
- What is the full signature of `CrateSlot__PlaceOverlayAndInitTimer`? Its internals would reveal what overlay type gets placed when called from trigger action 0x6C.
- `g_PLACE_SHP` — what filename/mix does this SHP load from? (Out of scope for these two functions; belongs in the broader placement-rendering doc.)
- The `BuildingTypeClass + 0x16BE` / `+0x16C0` flag checks in `BuildingPlacement_OverlayRenderer` that gate which of the three shadow routines is called — these are already documented in the wall connection doc but could be cross-referenced to INI keys.
- `DAT_00a8b261` in `UnitClass__ReceiveDamage` — the global flag gating the overlay-at-death call path — its INI key and default value are unknown.
