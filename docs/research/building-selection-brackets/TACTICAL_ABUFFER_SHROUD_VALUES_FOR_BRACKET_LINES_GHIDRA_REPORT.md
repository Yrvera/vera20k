# Tactical A-buffer Shroud Values For Building Bracket Lines - Ghidra Report

**Address(es):** `0x006D3660`, `0x00411330`, `0x004112D0`, `0x004801F0`, `0x0047EFE0`, `0x0047F250`, `0x006D8700`, `0x006FB170`, `0x006FB470`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard YR A-buffer producer values that can affect selected-building bracket line pixels after the line reaches `Surface::DrawLine_ABufModulated_ZClipped`: neutral reset, shroud/full suppression, shroud-edge modulation, enemy gap-generator covered cells, and conditional `FogOfWar=yes`.  
**Non-Scope:** Exact stock pixel-overlap screenshots, full AlphaShapeClass/cloak visual rendering, minimap/radar fog colors, and non-bracket readers of the A-buffer.  
**Confidence:** High for binary writer functions, constants, gates, and standard YR activity; Medium for asset-internal edge pixel distribution because this pass verified the binary's write semantics and value ranges, not every retail `SHROUD.SHP` pixel.  
**Active in YR:** Yes for reset, shroud, shroud-edge, and enemy gap-generator re-shroud paths; Conditional/off by default for fog-of-war blending.

## 1. Overview

Selected building bracket lines are drawn in the object pass, but the surface line rasterizer reads `g_ABuffer` for each candidate pixel. This report covers the producer side: which earlier tactical pass writes the A-buffer values that can leave a bracket pixel unchanged, dim it, or suppress it.

The scoped producer pass is `Tactical_layer_shroud_edges @ 0x006D3660`. It runs before terrain/object drawing in `TacticalClass_Draw @ 0x006D3D10` and writes A-buffer values through reset fills, `Shroud_fog_edge_rendering`, `ShroudEdge_BlitToABuffer`, `FogEdge_BlendToABuffer`, and `AlphaShapeClass__DrawAll_WithMask`.

## 2. Values And Writers

| Producer / value | A-buffer value written | Writer evidence | Active in YR |
|---|---:|---|---|
| Full redraw reset | `0x007F` | `TacticalClass_Draw @ 0x006D3F9F..0x006D3FA7` loads `g_ABuffer`, pushes `0x7F`, calls `CircBuf__FillAll @ 0x004112D0` | Yes. Standard tactical redraw path. |
| Dirty-rect reset | `0x007F` / packed `0x007F007F` | `Tactical_layer_shroud_edges @ 0x006D3660` calls `FUN_00411330`; `FUN_00411330 @ 0x00411330` stores `0x7F` for odd pixels and `DAT_007F007F` for paired pixels | Yes. Runs for dirty A-buffer regions in normal tactical pass. |
| Shroud/full suppression | `0x0000` when source pixel is `0x00` | `ShroudEdge_BlitToABuffer @ 0x0047EFE0` writes `(ushort)source_pixel` into `g_ABuffer` unless source is `0xFE`; line reader suppresses pixels when A-buffer sample is `0` per prior `BUILDING_BRACKET_ABUFFER_ZTEST_DEPTH_SEMANTICS` | Yes. Shroud is enabled by default (`ini/rulesmd.ini:3031`, `[MultiplayerDialogSettings] Shroud=yes`). |
| Shroud-edge modulation | source pixels `0x01..0x7E` dim; `0x7F` neutral; `0xFE` skips | `ShroudEdge_BlitToABuffer @ 0x0047EFE0` direct-writes all source bytes except `0xFE`; bracket line reader treats nonzero/non-`0x7F` as color modulation | Yes. Asset-driven edge pixels from `SHROUD.SHP`/`FOG.SHP` depending on fog flag. |
| No-edge clear cell | `0x7F` remains from reset unless another writer changes it | `Shroud_fog_edge_rendering @ 0x004801F0` maps `Shroud_EdgeBitmask_Calculator` result `-1` to frame `0`; transparent source pixels leave reset value intact | Yes. Normal explored/non-edge cells. |
| Fully shrouded cell frame selection | frame `0x0F`; actual A-buffer values are that SHP frame's source pixels | `Shroud_fog_edge_rendering @ 0x004801F0` maps result `-2` to `0x0F`; `Shroud_EdgeBitmask_Calculator @ 0x006D8700` returns `-2` when shroud flags `cell+0x12C & 0x18 == 0` | Yes. Applies to unrevealed shroud and enemy gap re-shroud. |
| Fog-of-war blend | if source `<0x80`: neutral `0x7F -> source`; otherwise `max(0, existing + source - 0x7F)`; source `>=0x80` ignored | `FogEdge_BlendToABuffer @ 0x0047F250` | Conditional. Requires `*g_ScenarioClass_Instance & 0x1000` and local player not defeated; standard `rulesmd.ini:3040 FogOfWar=no`. |

## 3. Core Logic

`TacticalClass_Draw @ 0x006D3D10` pass 0/passthrough manages A-buffer lifetime. On full redraw it calls `CircBuf__FillAll(g_ABuffer, 0x7F)`. On scroll, it uses `CircBuf__Scroll`; on pass 1, it calls `Tactical_layer_shroud_edges @ 0x006D3660`.

`Tactical_layer_shroud_edges @ 0x006D3660` first handles dirty cells, calling `Shroud_fog_edge_rendering` and then `AlphaShapeClass__DrawAll_WithMask`. It then handles clipped dirty rectangles: when the dirty-rect flag byte is set, it calls `FUN_00411330` to reset the A-buffer region to `0x7F`, then `FUN_006D71E0` to sweep/render shroud/fog over the rect.

`Shroud_fog_edge_rendering @ 0x004801F0` computes two frame numbers:

```text
shroud_frame = Shroud_EdgeBitmask_Calculator(cell+0x24, 0)
cell+0x120 = shroud_frame
if shroud_frame == -2: frame = 0x0F
if shroud_frame == -1: frame = 0
ShroudEdge_BlitToABuffer(screen_pos, clip_rect, frame)

fog_frame = Shroud_EdgeBitmask_Calculator(cell+0x24, 1)
cell+0x121 = fog_frame
if (SpecialFlags & 0x1000) && !g_PlayerPtr->IsDefeated:
    map -2 -> 0x0F, -1 -> 0
    FogEdge_BlendToABuffer(screen_pos, clip_rect, frame)
```

`Shroud_EdgeBitmask_Calculator @ 0x006D8700` uses `cell+0x12C` bit `0x08` for shroud. If both `0x08` and `0x10` are clear, it marks the cell as fully interior shroud (`-2`). If the cell is not explored (`0x08` clear), it returns the precomputed `-1/-2` without building an edge mask. If explored, it checks all eight neighbors and looks up `UNK_007F4194[mask]`.

`ShroudEdge_BlitToABuffer @ 0x0047EFE0` lazy-loads `SHROUD.SHP` and `FOG.SHP`; it chooses `SHROUD.SHP` when `SpecialFlags & 0x1000` is clear and `FOG.SHP` when the bit is set. It writes each source byte as a 16-bit A-buffer value unless the source byte is `0xFE`.

`FogEdge_BlendToABuffer @ 0x0047F250` lazy-loads `FOG.SHP` and processes only source bytes `< 0x80`. If the destination A-buffer value is neutral `0x7F`, it stores the source. Otherwise it computes `existing + source - 0x7F` and clamps negative or zero results to `0`.

## 4. Gap-Covered Cells

Enemy gap generators do not introduce a separate hardcoded bracket-line A-buffer value in this slice. `TechnoClass::UpdateCloakShroud @ 0x006FB170` marks affected enemy cells by incrementing `cell+0x130`/`cell+0x134`; when `cell+0x130 > 0`, it clears `cell+0x12C` bits `0x10` and `0x08`. That makes the normal shroud edge calculator treat the cell as shrouded and feed the same `ShroudEdge_BlitToABuffer` values into `g_ABuffer`.

Removal is symmetric only after reference counts unwind. `TechnoClass::RemoveCloakShroud @ 0x006FB470` decrements `cell+0x134`; when the player flag at `g_PlayerPtr+0x577A` allows it and counters fall below one, it decrements `cell+0x130`, then restores `cell+0x12C` bits `0x08` and `0x10`.

Active in YR: Conditional/Yes. Retail YR has `[GAGAP] GapGenerator=yes`, `GapRadiusInCells=10`, `SuperGapRadiusInCells=10` at `ini/rulesmd.ini:12226-12228`; the path is active when an operational enemy gap generator covers the local player's cells. It is not the default state for every selected building bracket.

## 5. INI Keys

| Key | Value | Effect | Active in YR |
|---|---|---|---|
| `[MultiplayerDialogSettings] Shroud` | `yes` (`ini/rulesmd.ini:3031`) | Standard unexplored-cell shroud path is active | Yes |
| `[MultiplayerDialogSettings] FogOfWar` | `no` (`ini/rulesmd.ini:3040`) | Leaves `SpecialFlags & 0x1000` clear in standard YR | No by default |
| `[GAGAP] GapGenerator` | `yes` (`ini/rulesmd.ini:12226`) | Enables gap generator re-shroud path | Conditional/Yes |
| `[GAGAP] GapRadiusInCells` | `10` (`ini/rulesmd.ini:12227`) | Radius copied via type `+0xCD2` into `TechnoClass::UpdateCloakShroud` | Conditional/Yes |
| `[GAGAP] SuperGapRadiusInCells` | `10` (`ini/rulesmd.ini:12228`) | Alternate selected/active super-gap radius path in related docs; not distinct for A-buffer value once a cell is covered | Conditional |

## 6. Integration Points

Bracket lines are read-side consumers, not A-buffer producers. Prior bracket reports establish that `TechnoClass::DrawBehind`/`DrawExtras` bracket calls reach `Tactical::DrawLine3D @ 0x006DBB60` and then `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30`; the latter samples `g_ABuffer`, skips on `0`, writes original color on `0x7F`, and modulates on other nonzero values.

Producer ordering matters: `Tactical_layer_shroud_edges @ 0x006D3660` runs in pass 1 before the object/bracket pass. Therefore bracket lines see the A-buffer state already reset and overdrawn by shroud/fog/gap-derived shroud for that frame.

## 7. Current Rust Implementation Status

Read-only scan only. Current Rust has an A-buffer-like shroud multiply path in `src/render/shroud_buffer.rs`, including constants `0x7F`, `0x00`, `0xFE` and the 256-entry shroud edge LUT. The building bracket README notes remaining risk: bracket line rendering still does not model the original primary-surface A-buffer modulation plus Z-test/no-Z-write behavior for line pixels.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Full redraw A-buffer reset | verified | `0x006D3F9F..0x006D3FA7`, `0x004112D0` | none |
| Dirty-rect neutral reset | verified | `0x006D3660`, `0x00411330` | none |
| Shroud frame dispatch | verified | `0x004801F0`, `0x006D8700` | none |
| Shroud direct A-buffer write | verified | `0x0047EFE0` | exact retail frame pixel histogram not extracted |
| Fog conditional blend | verified | `0x004801F0`, `0x0047F250`, `ini/rulesmd.ini:3040` | only active in non-default FogOfWar sessions |
| Enemy gap re-shroud into normal shroud writer | verified | `0x006FB170`, `0x006FB470`, `ini/rulesmd.ini:12226-12228` | full gap visual AlphaShape overlay is out-of-scope |
| Selected building bracket read-side behavior | verified by prior scoped report | `BUILDING_BRACKET_ABUFFER_ZTEST_DEPTH_SEMANTICS_GHIDRA_REPORT.md`, `0x004BFD30` | no screenshot matrix |

## 9. Open Questions - Final State

[RESOLVED] OQ-1 - What value resets visible/non-shrouded A-buffer pixels? `0x7F`, via full redraw `CircBuf__FillAll` and dirty-rect `FUN_00411330`. Evidence: `0x006D3FA5`, `0x004112D0`, `0x00411330`. Active in YR: Yes.

[RESOLVED] OQ-2 - What value suppresses bracket line pixels? A-buffer sample `0`, produced by source pixel `0x00` in `ShroudEdge_BlitToABuffer`; the line reader suppresses on `abuf == 0`. Evidence: `0x0047EFE0`; prior `0x004BFD30` bracket report. Active in YR: Yes.

[RESOLVED] OQ-3 - What values dim bracket line pixels? Any nonzero value other than `0x7F`; shroud/fog edge source values are asset-driven bytes, with `0x01..0x7E` being dimming values and `0xFE` being source-side skip. Evidence: `0x0047EFE0`, `0x0047F250`, prior `0x004BFD30` report. Active in YR: Yes for shroud; Conditional for fog.

[RESOLVED] OQ-4 - Are gap-covered cells a distinct A-buffer value writer? For the re-shroud part, no: enemy gap generators clear `cell+0x12C` bits and the normal shroud writer supplies the A-buffer values. Evidence: `0x006FB170`, `0x006D8700`, `0x004801F0`, `0x0047EFE0`. Active in YR: Conditional/Yes with `[GAGAP]`.

[RESOLVED] OQ-5 - Is fog-of-war A-buffer blending active in standard YR defaults? No. It requires `SpecialFlags & 0x1000`; `rulesmd.ini` defaults `FogOfWar=no`. Evidence: `0x004801F0`, `ini/rulesmd.ini:3040`. Active in YR: Conditional, off by default.

[DEFERRED] OQ-6 - Enumerate every distinct byte present in retail `SHROUD.SHP`/`FOG.SHP` frames. Category: out-of-scope. Reason: this slice required producer values and writer semantics; the binary proves source bytes are asset-driven and not hardcoded. Next step: asset probe if a later visual trace needs exact histograms.

## Sources

- Ghidra decompiled/read-only: `0x006D3D10`, `0x006D3660`, `0x004112D0`, `0x00411330`, `0x004801F0`, `0x0047EFE0`, `0x0047F250`, `0x006D8700`, `0x006FB170`, `0x006FB470`
- Ghidra assembly context: `0x006D3F9F..0x006D3FA7`
- INI checked read-only: `ini/rulesmd.ini`
- Prior bracket source: `docs/research/building-selection-brackets/BUILDING_BRACKET_ABUFFER_ZTEST_DEPTH_SEMANTICS_GHIDRA_REPORT.md`
- Prior shroud source: `docs/research/SHROUD_FOG_RENDERING_PIPELINE.md`
