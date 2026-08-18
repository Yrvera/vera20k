# CellClass::GetRadarColor Full Branch Inventory - Ghidra Research Report

**Address(es):** `CellClass__GetRadarColor @ 0x0047C060`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Complete live-YR branch inventory for raw minimap RGB triples produced by `CellClass__GetRadarColor` and direct color-producing callees.
**Non-Scope:** Radar zoom/sampling, dirty queues, object-dot tracker, spy satellite reveal, gap/shroud special effects, radar events, tactical draw composition.
**Confidence:** High for branch order, gates, constants, output writes, and direct color source calls. Medium for semantic names of some TMP tile helper internals.
**Active in YR:** Yes. Called by ordinary `RadarClass` terrain generation and terrain-dirty refresh.

Target question: What exact branches and direct callees produce the left/right raw RGB triples from `CellClass::GetRadarColor @ 0x0047C060`?
Non-goals: Do not re-investigate radar surface zoom, dirty queues, object dots, spy satellite, gap/shroud special effects, or radar event pings.
Evidence needed to mark COMPLETE: decompile plus assembly-context evidence for every material branch, output constant, and direct color-producing callee; caller proof that the function feeds active YR minimap raw RGB.
Stop conditions: Stop after the raw RGB branch inventory and implementation handoff are complete; defer actual asset dumps and non-color radar systems.

## 1. Overview

`CellClass__GetRadarColor` returns two 3-byte RGB triples for the raw radar terrain buffer. Active minimap callers can write those triples into one or two adjacent raw radar-space pixels, but this function itself sets both outputs to the same RGB value in every verified branch.

The live branch order is:

1. Terrain object in cell object list, RTTI `0x24` -> fixed `(200,200,160)`.
2. Structural bridge flag `Cell+0x140 & 0x100` -> `BRIDGE1` overlay SHP frame metadata, frame `0`.
3. Non-skipped overlay at `Cell+0x44`:
   - non-tiberium overlay -> overlay SHP frame metadata, frame `1` for low-bridge ranges, else `Cell+0x11E`;
   - tiberium overlay with `OverlayType+0x29C` cell anim/image pointer -> that pointer's SHP frame metadata, frame `Cell+0x11E`;
   - tiberium overlay without that pointer -> fixed `(170,170,130)` after `OverlayToTiberiumIndex`.
4. Terrain tile fallback -> tile subimage `+0x2B..+0x2D`, theater brightness, then `>> 1`.
5. Missing tile subimage fallback -> fixed `(60,60,60)`.

## 2. Key Offsets

| Offset / global | Role | Evidence | Active in YR |
|---|---|---|---|
| `Cell+0x38` | isometric tile type index, `0xFFFF` means clear tile fallback | `0x0047C0D9..0x0047C0EE`, `0x0047C24A..0x0047C28D` | Yes |
| `Cell+0x44` | overlay type index, `-1` means none | `0x0047C0FC`, `0x0047C143`, `0x0047C17F` | Conditional on overlay |
| `Cell+0x11A` | tile subimage index | `0x0047C260`, `0x0047C2CF..0x0047C2D5` | Yes |
| `Cell+0x11E` | overlay data / frame / density byte | `0x0047C173`, `0x0047C1CB` | Conditional on overlay |
| `Cell+0x140 bit 0x100` | structural bridge radar override | `0x0047C0AE..0x0047C0C9` | Conditional on bridge state |
| `Cell+0x140 bit 0x2000` | alt tile variant bit for slope-like tile branch | `0x0047C272..0x0047C27B` | Conditional on tile helper result |
| `OverlayType+0x294` | overlay array index | `0x005FED2E`; prior `OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md` | Yes |
| `OverlayType+0x29C` | cell anim / alternate pointer used by tiberium and overlay fallback paths | `0x0047C1B3`, `0x005FED00` | Conditional |
| `OverlayType+0x2A9` | `Tiberium=` bool | `0x0047C146..0x0047C14F`; prior overlay docs | Conditional |
| `OverlayType+0x2B6..0x2B8` | INI `RadarColor=` RGB, used by the overlay type image-getter family, not directly by terrain fallback | `OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md` | Conditional |
| `g_OverlayTypeClass_Array @ 0x00A83D84` | overlay type array | `0x0047C0B9`, `0x0047C13C`, `0x0047C17F` | Yes |
| `g_IsometricTileTypeClass_Array @ 0x00A8ED2C` | tile type array | `0x0047C0E3`, `0x0047C0EE` | Yes |
| `g_ClearTile @ 0x00AA10B0` | clear tile type index fallback | `0x0047C0F4`, `0x0047C293` | Conditional on `Cell+0x38 == 0xFFFF` |

## 3. Core Logic

### 3.1 Terrain object fixed color branch

Active in YR: Conditional. Runs when `g_GameActive != 0` and the cell's ground object list contains an object whose virtual `What_Am_I` returns `0x24`.

At entry, `GetRadarColor` calls `CellClass__FindOccupierByRTTI @ 0x0047C4D0` with RTTI `0x24` and list selector `0`. If found, it writes:

```text
out_left  = (0xC8, 0xC8, 0xA0)
out_right = (0xC8, 0xC8, 0xA0)
```

Evidence:

- Call setup: `0x0047C065 PUSH 0`, `0x0047C069 PUSH 0x24`, `0x0047C06B CALL 0x0047C4D0`.
- Fixed writes: `0x0047C074..0x0047C0AB`.
- `CellClass__FindOccupierByRTTI` scans `Cell+0xE4` when selector is `0`, calls vtable `+0x2C`, and compares return value to requested RTTI.
- `TerrainClass__What_Am_I @ 0x0071D300` returns `0x24`.
- `BuildingClass__WhatAmI @ 0x00459EC0` returns `6`.

Important correction: this is not a BuildingClass branch. Existing docs that say "RTTI 0x24 = BuildingClass" are stale. This branch is for TerrainClass occupants such as trees/terrain objects. Live building object dots are handled by the radar object tracker, not by this raw terrain color branch.

### 3.2 Structural bridge flag branch

Active in YR: Conditional on bridge/ramp cells with `Cell+0x140 & 0x100`.

If bit `0x100` is set, the cell's own overlay index is ignored. The code loads `g_OverlayTypeClass_Array[24]`, which is `BRIDGE1` by the 1-based `[OverlayTypes]` list, and calls `OverlayClass__GetRadarColor` with frame `0`.

Evidence:

- Flag test: `0x0047C0AE MOV EAX,[ESI+0x140]`, `0x0047C0B4 TEST AH,0x1`.
- Fixed type: `0x0047C0B9 MOV ECX,[0x00A83D84]`, `0x0047C0C6 MOV ECX,[ECX+0x60]` (`0x60 / 4 = 24`).
- Frame: `0x0047C0C3 PUSH 0`.
- Color helper call: `0x0047C0C9 CALL 0x005FED00`.
- Output copy to both halves: `0x0047C18F..0x0047C1AA`.

### 3.3 Overlay skip list and non-tiberium overlay color branch

Active in YR: Conditional on a non-skipped overlay at `Cell+0x44`.

The exact skip values are:

```text
-1, 100, 101, 231, 232, 239
```

Evidence: compare/jump chain at `0x0047C0FF..0x0047C136`.

For an overlay whose `OverlayType+0x2A9 Tiberium` byte is zero, the code calls `OverlayClass__GetRadarColor` for the cell's overlay type. The frame argument is:

- `1` if overlay index is in `[0x4A,0x63]` or `[0xCD,0xE6]`;
- otherwise `Cell+0x11E`.

Evidence:

- Tiberium byte read/test: `0x0047C143..0x0047C14F`.
- Low-bridge ranges and forced frame: `0x0047C151..0x0047C169`.
- Natural frame: `0x0047C171..0x0047C179`.
- Cell overlay type call: `0x0047C17F MOV ECX,[EDX+EAX*4]`, `0x0047C182 CALL 0x005FED00`.

### 3.4 Tiberium overlay color branch

Active in YR: Conditional on `OverlayType+0x2A9 Tiberium != 0`.

If `OverlayType+0x29C` is non-null, the code calls that pointer's vtable `+0x9C`, then calls `GetTiberiumRadarColor` with frame `Cell+0x11E`. The resulting RGB triple is copied to both outputs.

Evidence:

- Pointer load/test: `0x0047C1B3 MOV ECX,[ECX+0x29C]`, `0x0047C1B9 TEST ECX,ECX`.
- Image getter: `0x0047C1BD MOV EDX,[ECX]`, `0x0047C1BF CALL [EDX+0x9C]`.
- Frame: `0x0047C1CB MOV CL,[ESI+0x11E]`.
- Metadata helper call: `0x0047C1D5 CALL 0x0069E860`.
- Output copy: `0x0047C1DA..0x0047C201`.

If `OverlayType+0x29C` is null, the code calls `CellClass__OverlayToTiberiumIndex @ 0x005FDD20`. If the return is not `-1`, it writes:

```text
out_left  = (0xAA, 0xAA, 0x82)
out_right = (0xAA, 0xAA, 0x82)
```

Evidence:

- Helper call: `0x0047C204 CALL 0x005FDD20`.
- `-1` test: `0x0047C20B CMP EAX,-1`, `0x0047C20E JZ 0x0047C24A`.
- Fixed writes: `0x0047C210..0x0047C247`.
- `OverlayToTiberiumIndex` returns `-1` only for no overlay or `Tiberium=0`; for a `Tiberium=1` overlay outside configured tiberium ranges, it logs "not really tiberium" and returns `0`.

The common "wall overlay" wording for this branch is misleading. The binary condition here is tiberium-flagged overlay fallback, not `OverlayType+0x2A8 Wall`.

### 3.5 Terrain tile fallback branch

Active in YR: Yes for cells that do not hit the terrain-object, bridge, or overlay branches.

Tile type selection:

- If `Cell+0x38 == 0xFFFF`, use `g_ClearTile` through `g_IsometricTileTypeClass_Array`.
- Otherwise use `g_IsometricTileTypeClass_Array[Cell+0x38]`.

Variant selection:

- Start with variant `0`.
- If tile type `+0x2F0 <= 1`, keep variant `0`.
- If `+0x2F0 > 1`, call `FUN_005471F0(tile_type, Cell+0x11A)`.
- If that helper returns nonzero, variant is `(Cell+0x140 >> 13) & 1`.
- Otherwise call `FUN_004814F0(cell, tile_index_or_clear_tile, tile_type+0x2F0)`.

Tile image/color selection:

- `FUN_00544E00(tile_type, variant)` wraps variant by `tile_type+0x2F0` and follows `+0x2BC` linked variants until it reaches the selected tile type object.
- If selected tile type `+0xA4` is null and `+0x2F4` is nonzero, `FUN_00544C80` demand-loads TMP data.
- It indexes the loaded TMP subimage pointer table at `selected_type+0xA4 + 0x10 + Cell+0x11A * 4`.
- If that subimage pointer is null, it falls back to `(60,60,60)`.
- Otherwise it reads bytes `subimage+0x2B`, `+0x2C`, `+0x2D`, applies theater brightness through `ApplyTheaterBrightness @ 0x00661190`, then writes each channel after unsigned `>> 1`.

Evidence:

- Tile type fallback setup: `0x0047C0D9..0x0047C0F9`, `0x0047C24A..0x0047C28D`.
- Variant gate: `0x0047C257 CMP [EDI+0x2F0],1`, `0x0047C25E JLE 0x0047C2A2`.
- Slope-like helper and alt bit: `0x0047C260..0x0047C27E`.
- Pseudo-random variant helper: `0x0047C280..0x0047C29D`.
- Variant object helper: `0x0047C2A2 CALL 0x00544E00`.
- Demand-load check: `0x0047C2AC..0x0047C2C7`.
- Subimage pointer lookup: `0x0047C2CF..0x0047C2DF`.
- RGB bytes: `0x0047C2E7..0x0047C2FE`.
- Theater table: `0x0047C302..0x0047C324`.
- Halving and output writes: `0x0047C33C..0x0047C38E`.
- Fallback `(60,60,60)`: `0x0047C391..0x0047C3C6`.

## 4. Direct Callees

| Function | Verified role | Active in YR |
|---|---|---|
| `CellClass__FindOccupierByRTTI @ 0x0047C4D0` | Scans `Cell+0xE4` or `+0xE8` object list only when `g_GameActive != 0`; compares object `What_Am_I` to requested RTTI. | Conditional |
| `OverlayClass__GetRadarColor @ 0x005FED00` | Gets an overlay image pointer through vtable `+0x9C`, falls back to `OverlayType+0x29C`, returns black if no image, otherwise reads SHP frame RGB metadata through `GetTiberiumRadarColor`; swaps bytes 1 and 2 for overlay indices `[0x7F,0x8A]` or `[0x93,0x9E]`. | Conditional on overlay/bridge |
| `GetTiberiumRadarColor @ 0x0069E860` | Calls `SHP_Resolve`; if frame is in range, reads RGB from `shp + 8 + frame * 0x18 + 0x0C..0x0E`; otherwise returns black. | Conditional |
| `OverlayTypeClass__GetRadarColor @ 0x005FEDE0` | Misleading label; it is an image getter/demand-loader. Returns `OverlayType+0xA4`, demand-loading only if `+0xA4 == 0 && +0x2AF != 0`. | Conditional |
| `CellClass__OverlayToTiberiumIndex @ 0x005FDD20` | Maps `Tiberium=yes` overlay index to a TiberiumClass index by configured primary/extra ranges; returns `0` with a warning for stray `Tiberium=yes` overlays outside the ranges. | Conditional |
| `FUN_005471F0 @ 0x005471F0` | Reads tile image metadata for `Cell+0x11A`; returns bit `(entry+0x24 >> 2) & 1` when present, else zero. Used only to select terrain variant. | Conditional |
| `FUN_004814F0 @ 0x004814F0` | Deterministic terrain variant selector using cell coords, tile index, variant count, and static tables initialized on first call. | Conditional |
| `FUN_00544E00 @ 0x00544E00` | Selects the nth tile type variant, modulo `+0x2F0`, by following `+0x2BC` links. | Yes for terrain fallback |
| `FUN_00544C80 @ 0x00544C80` | TMP demand-load helper when selected tile type has no loaded image and `+0x2F4` is set. | Conditional |
| `ApplyTheaterBrightness @ 0x00661190` | Applies theater brightness to a 3-byte RGB triple before `GetRadarColor` halves the channels. | Yes for terrain fallback |

## 5. Integration Points

Active in YR: Yes.

- `RadarClass__FillTerrainColors @ 0x00654EA0` iterates cells, calls `CellClass__GetRadarColor`, and writes one or two raw RGB triples into `RadarClass+0x123C`. Clipped left edge writes only the right color; clipped right edge writes only the left color; interior cells write both.
- `RadarClass__ClearBackground @ 0x00655250` drains terrain dirty cells, calls `MapClass__Get_CellClass`, then calls `CellClass__GetRadarColor` and refreshes the raw buffer plus generated terrain surface for that cell footprint.

Although the raw fill pipeline preserves separate left/right raw pixels, `CellClass__GetRadarColor` itself gives both outputs the same RGB. Do not average left/right before raw buffering, because the caller contract still has two positions and the raw write geometry is independently load-bearing.

## 6. Current Rust Implementation Status

Current Rust is not mechanism-parity for this color function:

- `src/render/minimap_helpers.rs::radar_color_for_cell` averages TMP `radar_left` and `radar_right`, uses `f32` brightness, and falls back to hardcoded land/water/elevated colors.
- `OverlayClassification::color` approximates ore/gem with interpolation and bridge/wall/tree constants. Native uses overlay SHP frame metadata, low-bridge forced frame `1`, tiberium fallback `(170,170,130)`, and specific skip ranges.
- `src/render/minimap.rs::new` pre-stamps overlay pixels outside the native raw `CellClass__GetRadarColor` branch order.
- `src/render/minimap.rs::update_unit_dots` uses khaki for building dots, but native khaki in this function is terrain-object raw color; live building/object dots are owner-color tracker pixels.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Entry/caller liveness | verified | `0x00654EA0`, `0x00655250`, `0x0047C060` | none |
| RTTI `0x24` first branch | verified | `0x0047C065..0x0047C0AB`, `0x0047C4D0`, `0x0071D300`, `0x00459EC0` | none |
| Bridge `0x100` branch | verified | `0x0047C0AE..0x0047C0D4` | none |
| Overlay skip list | verified | `0x0047C0FF..0x0047C136` | none |
| Non-tiberium overlay frame selection | verified | `0x0047C143..0x0047C182` | none |
| Tiberium overlay `+0x29C` branch | verified | `0x0047C1B3..0x0047C201` | none |
| Tiberium fallback `(170,170,130)` | verified | `0x0047C204..0x0047C247`, `0x005FDD20` | none |
| Terrain tile fallback | verified | `0x0047C24A..0x0047C38E` | exact human names for helper fields remain cosmetic |
| Missing subimage fallback | verified | `0x0047C391..0x0047C3C6` | none |
| Smudge/tree/building special colors outside these branches | verified negative for this function | no reads of `Cell+0x48`; RTTI proof for terrain/building | separate systems may color objects elsewhere |

## 8. Open Questions - Final State

- `[RESOLVED] OQ1 - Is CellClass__GetRadarColor active in ordinary YR minimap terrain generation? -> Yes, FillTerrainColors and ClearBackground call it for raw RGB terrain buffer updates.` (evidence: `0x00654EA0`, `0x00655250`)
- `[RESOLVED] OQ2 - Does the first fixed khaki branch represent BuildingClass? -> No. It looks for RTTI 0x24; TerrainClass__What_Am_I returns 0x24, while BuildingClass returns 6.` (evidence: `0x0047C4D0`, `0x0071D300`, `0x00459EC0`)
- `[RESOLVED] OQ3 - Are left and right RGB outputs ever different inside this function? -> No branch found writes different values; both outputs get the same RGB value.` (evidence: output write blocks in `0x0047C060`)
- `[RESOLVED] OQ4 - What overlay indices are skipped? -> -1, 100, 101, 231, 232, and 239.` (evidence: `0x0047C0FF..0x0047C136`)
- `[RESOLVED] OQ5 - What forces overlay frame 1? -> Overlay index in [0x4A,0x63] or [0xCD,0xE6].` (evidence: `0x0047C151..0x0047C169`)
- `[RESOLVED] OQ6 - Does wall flag OverlayType+0x2A8 produce (170,170,130) here? -> No direct wall flag read in this function; the branch is reached after Tiberium=1, null +0x29C, and OverlayToTiberiumIndex != -1.` (evidence: `0x0047C146..0x0047C20E`)
- `[RESOLVED] OQ7 - Does TerrainTypeClass.RadarColor affect raw minimap color for terrain objects in this function? -> No direct read found; terrain-object occupier branch writes fixed (200,200,160).` (evidence: `0x0047C074..0x0047C0AB`)
- `[RESOLVED] OQ8 - Does smudge state affect GetRadarColor? -> No Cell+0x48 or SmudgeType read is present in the verified branch inventory.` (evidence: `0x0047C060` decompile and assembly contexts)
- `[RESOLVED] OQ9 - What happens when tile subimage lookup is null? -> Both outputs become (60,60,60).` (evidence: `0x0047C2DD..0x0047C391`, `0x0047C391..0x0047C3C6`)
- `[DEFERRED] OQ10 - What are the exact retail RGB values for every overlay SHP frame and TMP subimage?` (category: out-of-scope; reason: this report inventories binary branch sources, not asset contents; next-step-if-pursued: dump retail SHP/TMP frame metadata and compare against Rust loaders)

## 9. Visual Composition Ledger

This function does not draw pixels directly. It provides raw RGB triples consumed by `RadarClass__FillTerrainColors` and `RadarClass__ClearBackground`.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `CellClass__GetRadarColor @ 0x0047C060` | called per terrain cell or dirty cell | overlay SHP frame metadata or TMP subimage metadata | cell footprint decided by caller | raw RGB, no sidebar palette | yes | raw terrain color source |
| 2 | `RadarClass__FillTerrainColors @ 0x00654EA0` | full terrain fill | raw buffer `RadarClass+0x123C` | one/two raw pixels per cell | raw RGB triples | yes | raw buffer writer |
| 3 | `RadarClass__ClearBackground @ 0x00655250` | terrain dirty cell flush | raw buffer plus generated terrain surface | dirty cell footprint | raw RGB then generated 16-bit pack | yes | incremental refresh |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Fixed `(200,200,160)` branch is TerrainClass RTTI `0x24`, not BuildingClass. | `0x0047C065..0x0047C0AB`, `0x0071D300`, `0x00459EC0` | Rust uses `COLOR_BUILDING` for live structure dots and has terrain-object overlay classification. | `src/render/minimap.rs`, `src/render/minimap_helpers.rs` | Use khaki only for raw terrain-object occupied cells in the `CellClass__GetRadarColor` equivalent; live building dots must use radar tracker owner color. | A cell with a tree/terrain object produces raw RGB `(200,200,160)`, while a building live dot on terrain uses owner-color tracker pixel. Proposed test: `minimap_get_radar_color_terrain_object_khaki_not_building_dot`. | Do not label RTTI `0x24` as BuildingClass or reuse this branch for structures. |
| Overlay branch uses skip list, tiberium byte, `+0x29C`, low-bridge forced frame `1`, and SHP frame metadata. | `0x0047C0FF..0x0047C1D5`, `0x005FED00`, `0x0069E860` | Rust classifies overlays broadly and interpolates ore/gem colors or uses constants. | `src/render/minimap_helpers.rs::OverlayClassification::color`, `src/render/overlay_atlas.rs`, `src/render/minimap.rs` | Build overlay raw color from native overlay index, `Tiberium=`, `CellAnim/Image` pointer equivalent, frame argument, and SHP frame metadata. | Low-bridge overlay in `[0x4A,0x63]` ignores density and uses frame `1`; ordinary overlay uses `Cell+0x11E`; skipped `LOBRDG24/25` falls through. Proposed test: `minimap_get_radar_color_overlay_skip_and_forced_frame_ranges`. | Do not replace native frame metadata with linear density interpolation or a single bridge constant. |
| Terrain fallback reads TMP subimage RGB, applies theater brightness, then unsigned halves channels; missing subimage returns `(60,60,60)`. | `0x0047C24A..0x0047C3C6`, `0x00544E00`, `0x00544C80`, `0x00661190` | Rust averages TMP left/right and uses `f32` fallback land/water/elevated colors. | `src/render/minimap_helpers.rs::radar_color_for_cell`, map TMP loading surfaces | Implement a raw `CellClass__GetRadarColor` equivalent over tile type, subimage index, variant selector, TMP metadata RGB, theater brightness, `>> 1`, and fallback constant. | Clear/urban/snow sample cells match native RGB triples before radar zoom sampling. Proposed test: `minimap_get_radar_color_terrain_tmp_brightness_shift_and_fallback`. | Do not average precomputed left/right or use category palettes as parity colors. |

## 11. Negative Facts / Do Not Do

- Do not treat RTTI `0x24` as BuildingClass in this function. It is TerrainClass; BuildingClass returns RTTI `6`.
- Do not use this function's khaki `(200,200,160)` as the live building-dot color. Live dots are object-tracker pixels handled later.
- Do not call the `(170,170,130)` branch a wall branch unless future evidence proves a wall-flag path elsewhere; this function reaches it through `Tiberium=1` fallback, not `Wall=`.
- Do not use INI `RadarColor=` alone for overlay/tiberium minimap pixels; `OverlayClass__GetRadarColor` reads SHP frame metadata and can swap bytes 1 and 2 for specific overlay index ranges.
- Do not introduce smudge, terrain lighting, tree-specific `RadarColor`, dynamic cloak, spy satellite, or gap/shroud behavior into `CellClass__GetRadarColor`; those are separate systems or absent from this branch inventory.

## 12. Remaining Uncertainty

- Exact human-readable names for tile helper fields `IsometricTileType+0x2F0`, `+0x2BC`, and `+0x2F4` were not renamed in Ghidra; behavior is verified enough for color branch parity.
- Retail asset RGB tables for every TMP subimage and overlay SHP frame were not dumped in this report.
- The runtime value of `g_GameActive` during every possible menu/editor generation path was not sampled; ordinary in-game minimap liveness is verified.

## 13. Stale Docs / Follow-up Docs

`C:/Users/enok/Documents/ra2-rust-game/docs/research/RADAR_MINIMAP_RENDERING.md`

Replace the start of section `5.2 CellClass::GetRadarColor` with:

> `CellClass::GetRadarColor @ 0x0047C060` first checks the cell object list for RTTI `0x24`; in YR this is `TerrainClass`, not `BuildingClass`, and the branch writes fixed `(200,200,160)` to both raw RGB outputs. Building/object dots are handled by the later radar object tracker. The bridge branch checks `Cell+0x140 & 0x100` and uses `BRIDGE1` frame `0`. The overlay branch skips `-1,100,101,231,232,239`; non-tiberium overlays use `OverlayClass::GetRadarColor` with frame `1` for `[0x4A,0x63]` or `[0xCD,0xE6]`, otherwise `Cell+0x11E`; tiberium overlays with `+0x29C` use SHP frame metadata at frame `Cell+0x11E`; tiberium fallback without `+0x29C` writes `(170,170,130)`. The terrain fallback reads TMP subimage RGB at `+0x2B..+0x2D`, applies theater brightness, then halves channels with `>> 1`; missing subimage returns `(60,60,60)`.

`C:/Users/enok/Documents/ra2-rust-game/docs/research/RADAR_SYSTEM_COMPREHENSIVE.md`

Replace the color priority list with the same wording above, or at minimum replace "Building occupier (RTTI 0x24)" with "TerrainClass occupier (RTTI 0x24)" and replace "Wall overlay" with "tiberium-overlay fallback `(170,170,130)` after `OverlayToTiberiumIndex`".

`C:/Users/enok/Documents/ra2-rust-game/docs/research/ADDRESS_MAP.md`

Replace:

> `0x0047C060 | CellClass::GetRadarColor (priority: bldg->bridge->overlay->terrain)`

with:

> `0x0047C060 | CellClass::GetRadarColor (priority: terrain-object RTTI 0x24 -> bridge flag -> overlay/tiberium -> terrain TMP)`

Replace:

> `0x005FED00 | OverlayClass::GetRadarColor (bridge byte-swap)`

with:

> `0x005FED00 | OverlayClass::GetRadarColor (SHP-frame RGB; swaps bytes 1/2 for overlay indices 0x7F-0x8A and 0x93-0x9E)`

## Sources

- Ghidra decompile: `CellClass__GetRadarColor @ 0x0047C060`, `CellClass__GetRadarPixelColor @ 0x0047BDB0`, `CellClass__FindOccupierByRTTI @ 0x0047C4D0`, `OverlayClass__GetRadarColor @ 0x005FED00`, `OverlayTypeClass__GetRadarColor @ 0x005FEDE0`, `GetTiberiumRadarColor @ 0x0069E860`, `CellClass__OverlayToTiberiumIndex @ 0x005FDD20`, `FUN_005471F0 @ 0x005471F0`, `FUN_004814F0 @ 0x004814F0`, `FUN_00544E00 @ 0x00544E00`, `FUN_00544C80 @ 0x00544C80`, `RadarClass__FillTerrainColors @ 0x00654EA0`, `RadarClass__ClearBackground @ 0x00655250`, `TerrainClass__What_Am_I @ 0x0071D300`, `BuildingClass__WhatAmI @ 0x00459EC0`.
- Ghidra assembly contexts: `0x0047C060`, `0x0047C074`, `0x0047C0AE`, `0x0047C0FF`, `0x0047C151`, `0x0047C18F`, `0x0047C1B3`, `0x0047C204`, `0x0047C24A`, `0x0047C2C7`, `0x0047C302`, `0x0047C34D`, `0x0047C391`, `0x00544E00`.
- Prior docs referenced: `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `BRIDGE_RADAR_MINIMAP_PIXEL_RENDER_GHIDRA_REPORT.md`, `OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md`, `ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md`, `RADAR_MINIMAP_RENDERING.md`, `RADAR_SYSTEM_COMPREHENSIVE.md`, `ADDRESS_MAP.md`.
- INI files checked: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`.
- Rust surfaces scanned: `src/render/minimap_helpers.rs`, `src/render/minimap.rs`, `src/render/overlay_atlas.rs`.

## Status

COMPLETE for the scoped live-YR `CellClass__GetRadarColor` raw RGB branch inventory and direct color-producing callees.
