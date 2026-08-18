# Gap Generator / Radar Shroud Minimap Interaction - Ghidra Research Report

Date: 2026-05-27

**Slot:** /re-swarm radar-minimap-adjacent-pixel-blockers slot 4  
**Target:** `GAP_RADAR_SHROUD_MINIMAP_INTERACTION`  
**Investigation Mode:** exhaustive slice for GapGenerator/CloakShroud effects that feed ordinary in-game minimap pixels.  
**Address(es):** `TechnoClass__UpdateCloakShroud @ 0x006FB170`, `TechnoClass__RemoveCloakShroud @ 0x006FB470`, `RadarClass__RenderCellPixel @ 0x00655C50`, `RadarClass__RefreshRadar @ 0x00657CE0`, `IsShrouded @ 0x00586360`, `IsFogged @ 0x005864A0`, `BuildingClass__UpdateGapAndSpecialEffects @ 0x004549B0`  
**Confidence:** High for normal GapGenerator apply/remove and minimap pixel consequences; Medium for exact semantic names of a few HouseClass/player flags.  
**Active in YR:** Yes. Retail `[GAGAP]` has `GapGenerator=yes`, `GapRadiusInCells=10`, and `SuperGapRadiusInCells=10` in `ini/rulesmd.ini`.

## Summary

The Allied Gap Generator affects ordinary in-game minimap pixels, but not through a special radar palette, special gap color, or separate radar-gap overlay. Its live `GapGenerator=yes` path calls `TechnoClass__UpdateCloakShroud`, which mutates the same cell shroud/fog fields that `RadarClass__RenderCellPixel` reads.

For hostile cells, the gap path clears the explored/visible bits used by `IsShrouded`, so the minimap falls into the normal shrouded branch and writes packed pixel `0` unless an eligible object-dot branch wins first. For local/allied/reveal-override cells, the path increments the cell fog counter used by `IsFogged`, so the minimap uses the existing half-bright terrain branch. Apply/remove calls force a radar refresh, which re-renders primary minimap pixels through `RenderCellPixel`; they do not enqueue a per-cell `MarkCellDirty` list.

## Target and Non-Scope

Target:

- Gap Generator / `GapGenerator=yes` cell effects that change minimap pixels.
- Whether gap coverage uses black, fog, or another color.
- Dirtying/refresh behavior after gap apply/remove.
- Standard YR liveness and retail INI gates.

Non-scope:

- Tactical shroud edge sprite rendering except where its cell flags are read by the minimap.
- Full selected range ring rendering; covered by `TACTICAL_DRAWUNITACTIONVISUALS_SENSOR_RINGS_GHIDRA_REPORT.md`.
- Full object-dot priority; covered by `RADAR_OBJECT_DOT_PRIORITY_VISIBILITY_GATES_GHIDRA_REPORT.md`.
- `CloakGenerator=yes` TS-style field radius animation; no retail YR building uses that key.
- Rust code edits.

## Verified Binary Findings

### 1. Retail YR has a live GapGenerator path

`BuildingClass__UpdateGapAndSpecialEffects @ 0x004549B0` checks `BuildingType/TechnoType+0xCD1`. If nonzero and the building's shroud-active latch `this+0x269` is clear while the building is operational, it calls vtable `+0x414`, resolved as `TechnoClass__UpdateCloakShroud @ 0x006FB170`. On the unpowered/remove side, if `+0xCD1` is nonzero and `this+0x269` is set, it calls vtable `+0x418`, resolved as `TechnoClass__RemoveCloakShroud @ 0x006FB470`.

Retail `rulesmd.ini` has:

| Section | Key | Value | Active in standard YR? |
|---|---|---:|---|
| `[GAGAP]` | `GapGenerator` | `yes` | Yes |
| `[GAGAP]` | `GapRadiusInCells` | `10` | Yes |
| `[GAGAP]` | `SuperGapRadiusInCells` | `10` | Conditional selected-ring/super state; not the normal update radius in this slice |
| `[MultiplayerDialogSettings]` | `FogOfWar` | `no` | TS fog default off, but gap writes its own cell fog counter |
| `[MultiplayerDialogSettings]` | `Shroud` | `yes` | Yes |

Evidence: `0x004549B0`, `0x006FB170`, `0x006FB470`, `ini/rulesmd.ini:12221-12228`, `ini/rulesmd.ini:3031`, `ini/rulesmd.ini:3040`.

### 2. Apply caches `GapRadiusInCells` and covers a strict circular footprint

`TechnoClass__UpdateCloakShroud @ 0x006FB170` starts only when:

- `g_PlayerPtr != 0`.
- `this+0x269 == 0`.
- `this->vtable+0x350` returns nonzero, so the building is operational.

If `this+0x26C` is zero, the function reads `TechnoType+0xCD2` and stores it in `this+0x26C`. For retail `[GAGAP]`, this is `10`.

The cell walk converts the object's world position to cell coordinates with signed `(value + (value >> 31 & 0xFF)) >> 8` truncation. The loop bounds run from `-(radius + 2)` to `< radius + 2`, but actual accepted cells satisfy:

```text
dx*dx + dy*dy < (radius + 1) * (radius + 1)
```

That strict `<` and the `radius + 1` threshold are part of the binary mechanism. They must not be replaced with `<= radius*radius`.

Evidence: `0x006FB170` decompile, radius read at `type+0xCD2`, loop/condition in `TechnoClass__UpdateCloakShroud`; `BuildingClass__GetSensorRange @ 0x004566B0` separately confirms `[GAGAP]` radius metadata priority for selected rings, but the minimap mutation path uses `+0xCD2`.

### 3. Hostile gap coverage feeds the normal minimap shrouded-black branch

For cells where the gap owner is not the local human/allied side and the local player reveal-override byte does not bypass it, `TechnoClass__UpdateCloakShroud` does all of the following:

- If `cell+0x130 != 1` and `cell+0x130 >= 0`, increment `cell+0x130`.
- Increment `cell+0x134`.
- If `cell+0x130 > 0`, clear bits `0x10` and `0x08` in `cell+0x12C` (`puVar4[300]` in decompile).

`IsShrouded @ 0x00586360` returns false only when `cell+0x12C & 0x08` is set. Therefore clearing bit `0x08` makes `IsShrouded` true for `RadarClass__RenderCellPixel @ 0x00655C50`.

`RenderCellPixel` then uses its normal branch order:

1. Eligible object dot can still win first.
2. Else fogged terrain if `IsFogged` is true.
3. Else shrouded terrain writes literal packed `0`.
4. Else visible terrain copies the secondary terrain pixel.

So a hostile gap does not write a special "gap color" on the minimap. It converts the cell into the same black shroud outcome used by unexplored cells, unless a prior object-dot branch wins.

Evidence: `0x006FB170`, `0x00586360`, `0x00655C50`; assembly xrefs to shroud/fog checks at `0x00655D4F` and `0x00655D74`.

### 4. Local/allied/reveal-override gap coverage feeds the minimap fog half-bright branch

For cells where the gap owner is the local human side, allied with `g_PlayerPtr`, or the local player reveal-override byte is set, `TechnoClass__UpdateCloakShroud` increments `cell+0x13C`.

`IsFogged @ 0x005864A0` returns true when `cell+0x13C >= 1`. `RadarClass__RenderCellPixel @ 0x00655C50` handles fog by reading the secondary terrain pixel, unpacking channels through the DirectDraw loss/shift globals, shifting each unpacked channel right by one, repacking, and writing that packed 16-bit pixel.

This branch is independent of the standard YR `FogOfWar=no` default. The global TS fog option is off by default, but the GapGenerator path directly increments the same per-cell fog counter that the minimap reads.

Evidence: `0x006FB170` increments `cell+0x13C`; `0x005864A0` tests `cell+0x13C`; `0x00655E8A..0x00656081` fog pixel packing branch in prior minimap report and direct `RadarClass__RenderCellPixel` decompile.

### 5. Remove path reverses the same counters and refreshes radar

`TechnoClass__RemoveCloakShroud @ 0x006FB470` starts only when:

- `g_PlayerPtr != 0`.
- `this+0x269 != 0`.

It reuses the cached radius from `this+0x26C` or reads `TechnoType+0xCD2` if still zero. It clears `this+0x269`.

For hostile cells it decrements `cell+0x134`. If the local player's restore/reveal byte at `g_PlayerPtr+0x577A` is nonzero and `cell+0x134 < 1`, it decrements `cell+0x130`. If `cell+0x130 < 1`, it sets bits `0x08` and `0x10` in `cell+0x12C`, making `IsShrouded` false again for the minimap.

For local/allied/reveal-override cells it decrements `cell+0x13C`, reversing the fog counter used by `IsFogged`.

Both apply and remove finish by clearing `g_PlayerPtr+0x240` when the player pointer exists, calling `RadarClass__RefreshRadar @ 0x00657CE0`, then calling `FUN_004F42F0(2)`.

Evidence: `0x006FB470`, refresh call xrefs at `0x006FB44B` and `0x006FB728`, `0x00657CE0`.

### 6. Dirtying is full radar refresh through RenderCellPixel, not per-cell MarkCellDirty

`RadarClass__RefreshRadar @ 0x00657CE0` is the dirty/render bridge used by gap apply/remove. When `g_hWnd != 0`, it loops over every primary minimap surface pixel and calls `RadarClass__RenderCellPixel` for each one. Because `RenderCellPixel` expands `RadarClass+0x120C..0x1218`, this creates a native accumulated dirty rect for the visible radar surface.

No `RadarClass__MarkCellDirty @ 0x006562D0` or `RadarClass__MarkTerrainDirty @ 0x006551C0` call is used by `UpdateCloakShroud` or `RemoveCloakShroud`. Gap changes are not terrain-color changes; they are visibility-composition changes over the current secondary terrain surface.

Evidence: `0x00657CE0`, xrefs from `0x006FB44B` and `0x006FB728`, `0x00655C50`.

### 7. Object-dot interaction is unchanged by a special gap color path

`RadarClass__RenderCellPixel` computes `IsShrouded` and `IsFogged` before scanning the tracker bucket. The object branch can still write a dot before the fog/shroud/terrain branch, subject to the object eligibility rules verified in `RADAR_OBJECT_DOT_PRIORITY_VISIBILITY_GATES_GHIDRA_REPORT.md`.

This report did not re-audit every object gate. The load-bearing pixel fact is that gap coverage changes the shroud/fog booleans read by `RenderCellPixel`; it does not add a new layer after object dots or a new minimap overlay.

Evidence: `0x00655C50`, `RADAR_OBJECT_DOT_PRIORITY_VISIBILITY_GATES_GHIDRA_REPORT.md`.

## Active in Standard YR?

| Behavior | Active in standard YR? | Condition / default |
|---|---|---|
| `[GAGAP]` GapGenerator apply/remove | Yes | Retail Allied Gap Generator has `GapGenerator=yes`; building must exist, be powered/operational, and `g_PlayerPtr != 0` |
| Hostile gap cells become minimap shrouded-black | Yes | Local player is not owner/allied and reveal-override byte does not bypass it |
| Owner/allied gap cells become minimap fog-half-bright | Yes/conditional | Applies when owner/allied/reveal-override branch runs; not dependent on `[MultiplayerDialogSettings] FogOfWar=yes` |
| Optional TS FogOfWar global | Conditional/off by default | `rulesmd.ini` has `FogOfWar=no`; gap writes `cell+0x13C` directly anyway |
| `CloakGenerator=yes` radius animation | No for retail YR stock content | No retail YR building uses `CloakGenerator=yes`; non-scope here |

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| GapGenerator apply/remove mutates minimap-visible cell shroud/fog fields, not a separate radar overlay. | `0x006FB170`, `0x006FB470`, `0x00655C50` | Mismatch likely: Rust has `FogState::FLAG_GAP_COVERED` and object hiding, but minimap terrain path mostly uses revealed/gap-covered checks rather than native cell counters. | `src/sim/vision/mod.rs`, `src/render/minimap.rs` | Model gap as native shroud/fog state transitions that feed minimap terrain composition. | A hostile `[GAGAP]` field over previously visible terrain produces packed black `0` on the minimap after refresh. | Do not draw a translucent gap circle or custom gap color on top of the minimap. |
| Hostile gap cells clear `cell+0x12C` bits `0x08/0x10`; `IsShrouded` then makes `RenderCellPixel` write packed `0`. | `0x006FB170`, `0x00586360`, `0x00655C50` | Mismatch: Rust keeps cells revealed and uses a separate `gap_covered` flag mostly for hiding objects. | `src/sim/vision/mod.rs`, `src/render/minimap.rs` | For minimap parity, hostile gap coverage must affect terrain visibility exactly like native shrouded-black, without erasing the need to restore prior state when removed. | Enemy gap over explored cell changes visible minimap terrain to black, then remove restores according to native counters/flags. | Do not treat gap as only an object-dot visibility gate. |
| Owner/allied/reveal-override gap cells increment `cell+0x13C`; `IsFogged` makes minimap terrain half-bright through packed 16-bit channel `>> 1`. | `0x006FB170`, `0x005864A0`, `0x00655C50` | Likely mismatch: Rust currently has optional fog semantics and separate gap flag; owner/allied gap fogging is not proven. | `src/sim/vision/mod.rs`, `src/render/minimap.rs` | Preserve the local/allied fog-counter branch even when global `FogOfWar=no`. | Local-owned `[GAGAP]` field over terrain renders half-bright minimap pixels, not normal visible terrain and not black shroud. | Do not gate this branch solely on `[MultiplayerDialogSettings] FogOfWar=yes`. |
| Apply/remove call `RadarClass__RefreshRadar`, which loops all primary minimap pixels through `RenderCellPixel`; it does not enqueue terrain dirty or pixel dirty lists per cell. | `0x00657CE0`, xrefs `0x006FB44B`, `0x006FB728` | Mismatch: Rust refreshes via generation/full RGBA texture logic; no retained native primary surface path. | `src/render/minimap.rs`, `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs` | Gap state transitions should force full minimap primary refresh through native composition order, then copy via retained sidebar surface. | Toggling power on a Gap Generator refreshes all affected minimap pixels in the same path as `RenderCellPixel` and expands native dirty rects. | Do not mark terrain dirty; gap does not recolor the secondary terrain surface. |
| Gap radius footprint uses cached `TechnoType+0xCD2`, signed cell truncation, and strict `dx*dx + dy*dy < (radius+1)^2`. | `0x006FB170`, `0x006FB470`, `ini/rulesmd.ini:12227` | Mismatch possible: Rust uses `gap_radius` circle with `dx*dx+dy*dy <= r*r` in `apply_gap_generators`. | `src/sim/vision/mod.rs`, rules object type parsing | Use native strict radius footprint for minimap-visible shroud/fog mutation. | Retail radius `10` accepts exactly cells satisfying native strict `(r+1)^2` test from the adjusted center. | Do not use `<= r*r` or a square area. |

## Negative Facts / Do Not Do

- Do not implement Gap Generator minimap pixels as a special overlay, alpha circle, palette remap, or unique gap color.
- Do not treat gap as object-dot hiding only. Native hostile gap changes terrain minimap pixels through the shrouded-black branch.
- Do not gate the gap-owned/allied half-bright branch on global `FogOfWar=yes`; the gap path writes the cell fog counter directly.
- Do not use tactical shroud-edge sprites or A-buffer edge values for minimap gap pixels.
- Do not dirty the secondary terrain surface for gap apply/remove. The terrain color source is unchanged; only final primary composition changes.
- Do not use inclusive `<= radius*radius` for the gap footprint.
- Do not generalize `CloakGenerator=yes` findings to stock YR; this report is about retail `GapGenerator=yes` and minimap pixels.

## Remaining Uncertainty

- The exact semantic names of `g_PlayerPtr+0x1F5`, `g_PlayerPtr+0x240`, and `g_PlayerPtr+0x577A` were not resolved in this slice. Their branch positions and effects are verified.
- No live screenshot/runtime watchpoint was taken to visually sample owner/allied half-bright gap pixels. The binary path to `cell+0x13C -> IsFogged -> half-bright minimap pixel` is verified.
- Exact interaction with multiple overlapping hostile and allied gap generators needs a separate counter-overlap scenario if implementation reaches that edge. The individual counters `cell+0x130`, `cell+0x134`, and `cell+0x13C` are verified.
- Object-dot eligibility under gap remains delegated to the existing object-dot report; this report only proves the terrain/fog/shroud pixel branch and composition ordering.

## Stale-Doc Replacement Wording

Replace any wording that says "Gap Generator only hides objects on the minimap" with:

> Retail `GapGenerator=yes` mutates the cell shroud/fog fields consumed by `RadarClass::RenderCellPixel`. Hostile gap coverage clears the cell explored/visible bits used by `IsShrouded`, so minimap terrain falls into the normal shrouded branch and writes packed pixel `0`. Local/allied/reveal-override coverage increments the cell fog counter used by `IsFogged`, so minimap terrain uses the existing packed 16-bit half-bright fog branch. Gap apply/remove calls `RadarClass::RefreshRadar`, which re-renders the primary minimap surface through `RenderCellPixel`; it is not a special radar overlay and not a terrain dirty-color update.

Replace any wording that says "standard YR fog is off, so gap cannot use minimap fog" with:

> Standard YR `FogOfWar=no` disables TS-style global fog, but the GapGenerator path directly increments/decrements `cell+0x13C`; `IsFogged` reads that counter regardless of the INI default.

## Status

COMPLETE for the scoped GapGenerator / radar-gap / minimap-pixel interaction: active YR gates, hostile black-shroud behavior, local/allied fog-half-bright behavior, refresh/dirtying path, and implementation-facing deltas are verified from binary evidence.

PARTIAL only for live screenshot sampling, semantic naming of three player bytes, and exhaustive multi-gap overlap scenarios.

## Sources

- Ghidra read-only decompile: `BuildingClass__UpdateGapAndSpecialEffects @ 0x004549B0`.
- Ghidra read-only decompile: `TechnoClass__UpdateCloakShroud @ 0x006FB170`.
- Ghidra read-only decompile: `TechnoClass__RemoveCloakShroud @ 0x006FB470`.
- Ghidra read-only decompile: `RadarClass__RenderCellPixel @ 0x00655C50`.
- Ghidra read-only decompile: `RadarClass__RefreshRadar @ 0x00657CE0`.
- Ghidra read-only decompile: `IsShrouded @ 0x00586360`, `IsFogged @ 0x005864A0`.
- Ghidra assembly context: shroud/fog calls at `0x00655D4F`, `0x00655D74`; refresh calls at `0x006FB44B`, `0x006FB728`.
- Prior docs: `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE_GHIDRA_REPORT.md`, `RADAR_OBJECT_DOT_PRIORITY_VISIBILITY_GATES_GHIDRA_REPORT.md`, `BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md`, `TACTICAL_DRAWUNITACTIONVISUALS_SENSOR_RINGS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: `src/sim/vision/mod.rs`, `src/render/minimap.rs`, `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs`, `src/rules/object_type.rs`.
