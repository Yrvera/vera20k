# Primary Surface Z-Buffer Bracket Ownership - Ghidra Research Report

**Address(es):** `0x006DBB60`, `0x004BFD30`, `0x006D3F50`, `0x006D2B60`, `0x00547CF0`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** selected building bracket line Z-buffer ownership: which tactical depth buffer is sampled, which normal frame paths clear/write it before bracket lines, and whether bracket lines write it.  
**Non-Scope:** screenshot/pixel probes, exhaustive VXL cloaking/temporal visual modes, all non-building line overlays, and every possible modded draw flag combination.  
**Confidence:** High for selected-building bracket path, global buffer identity, clear timing, terrain/TMP writes, normal building SHP non-write, and bracket no-write. Medium for the broader VXL/exotic visual-state matrix because only representative active paths were checked.  
**Active in YR:** Yes for normal selected-building brackets in tactical rendering. Conditional for object-only redraws and non-normal visual states as noted below.

## 1. Overview

Selected building bracket lines are drawn through `Tactical::DrawLine3D @ 0x006DBB60` into the current `g_PrimarySurface` vtable slot `+0x34`, resolving to `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30`. That surface routine does not use a depth buffer owned by the primary color surface object; it samples the global tactical `g_ZBuffer` and global `g_ABuffer` side buffers.

The sampled depth state is the tactical frame Z-buffer cleared during the terrain phase and populated primarily by TMP/tile paths before the object phase. Normal selected-building bracket lines Z-test against this global buffer but pass a zero Z-write flag, so they do not become occluders for later bracket or object pixels.

## 2. Buffer Identity / Key Offsets

| Item | Evidence | Purpose | Active in YR |
|---|---|---|---|
| `g_PrimarySurface` | `Tactical::DrawLine3D @ 0x006DBB60` calls `(*g_PrimarySurface).vtable+0x34` | color destination / surface method dispatch | Yes; bracket helpers call `g_Tactical->vtable+0x60` |
| `g_ZBuffer` | `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30` checks `g_ZBuffer != 0`, calls `ZBuffer_scanline_ptr`, and compares each candidate line depth to `*zbuf` | global tactical 16-bit depth surface sampled by brackets | Yes |
| `g_ABuffer` | same surface routine calls `CircBuf_GetScanlinePtr(g_ABuffer, ...)` and suppresses/modulates line color by sample value | global tactical shroud/brightness auxiliary surface | Yes |
| Z-buffer stride | `g_ZBuffer+0x28` read in `0x004BFD30`; `ZBuffer_scanline_ptr @ 0x007BD130` wraps against `+0x18/+0x1C/+0x20` | circular scanline stepping | Yes |
| Z-buffer clear value | `ZBuffer_row_fill @ 0x007BCFB0` writes `0xFFFF` / `0xFFFFFFFF` over dirty spans | far/default depth before terrain writes | Yes |

## 3. Core Logic

### 3.1 Brackets sample the global tactical Z-buffer

`Tactical::DrawLine3D @ 0x006DBB60` projects both 3D endpoints and dispatches to the current `g_PrimarySurface` slot `+0x34`. The callee `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30` obtains destination pixels from the surface object, but it obtains depth from global `g_ZBuffer`, not from a field of `g_PrimarySurface`.

Material finding: bracket Z-test uses the same global tactical `g_ZBuffer` that terrain/TMP rendering uses. Active in YR: Yes. Evidence: `0x004BFD30` guard on `g_ZBuffer`, `ZBuffer_scanline_ptr @ 0x007BD130`, and `TMP_TileBlitter @ 0x00547CF0` using the same global.

### 3.2 Per-pixel predicate

For each candidate line pixel in the surface routine:

```text
if ((ushort)line_z < *g_ZBuffer_pixel && g_ABuffer_pixel != 0) {
    write bracket color, possibly A-buffer-modulated
    if (z_write_flag != 0) *g_ZBuffer_pixel = (ushort)line_z
}
```

The strict comparison matters: line pixels at equal depth do not draw over the current Z-buffer value. Active in YR: Yes. Evidence: `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30`, branch bodies around the z-, x-, and y-dominant loops; prior spot addresses `0x004C024F`, `0x004C043A`, `0x004C062B` guard writes.

### 3.3 Selected building brackets do not write depth

`TechnoClass::DrawBracketCorner @ 0x006F5EF0`, `TechnoClass::DrawBehind @ 0x006F60D0`, and `TechnoClass::DrawExtras @ 0x006F5190` route selected-building bracket segments to `g_Tactical->vtable+0x60` / `Tactical::DrawLine3D` with the final flag set to `0`. That flag is the surface Z-write flag.

Material finding: selected building bracket lines can be hidden by prior depth, but they do not update `g_ZBuffer`. Active in YR: Yes. Evidence: bracket caller final argument `0` in `0x006F5EF0` and direct `DrawExtras` line sites; write guard in `0x004BFD30`.

## 4. Frame Clear / Write Ownership

| Frame stage / writer | Relationship to bracket Z-test | Evidence | Active in YR |
|---|---|---|---|
| Tactical terrain phase clear | Dirty rects are clipped and cleared to `0xFFFF` before terrain/shroud/base tile drawing | `TacticalClass_Draw @ 0x006D3F50` calls `Tactical_ZBufferDirtyClear @ 0x006D2B60`; `ZBuffer_row_fill @ 0x007BCFB0` writes `0xFFFF` | Yes for terrain/full draws (`param_3 == 1` or `3`) |
| Object-only tactical draw | The clear/terrain sequence is skipped when `param_3 == 2`; object pass samples existing side-buffer state | `TacticalClass_Draw @ 0x006D3F50` jumps to object section when `param_3 == 2` | Conditional; depends on prior retained tactical buffers |
| TMP/base terrain and overlays | Write the same `g_ZBuffer` when tile has Z data and z-enable is passed | `CellOverlay_TileDraw @ 0x00480350` passes z-enable `1`; `TMP_TileBlitter @ 0x00547CF0` writes `*zbuf = pixel_z` on `pixel_z <= *zbuf` | Yes |
| Terrain-shadow tile redraws | Call the same tile draw path, so their tile pixels can write the same `g_ZBuffer` where TMP Z-data is present | `Tactical_layer_terrain_shadows @ 0x006D2DE0` -> `FUN_006D7F20` -> `CellOverlay_TileDraw @ 0x00480350` | Yes for those tile redraw cases |
| Normal building SHP body | Does not populate bracket-sampled depth in the normal selected-building path; normal visual state produces `0x800|0x600` style SHP flags without Z bits, selecting non-Z A-buffer/remap blitters | `BuildingClass_DrawBody @ 0x0043D290`; `TechnoClass_DrawSHP @ 0x00705E00`; `Blitter_selector @ 0x00490B90`; `Blitter_Opaque_RLE_Remap @ 0x004978C0` reads `g_ABuffer` but not `g_ZBuffer` | Yes for normal building body; non-normal visual states conditional |
| BUILDNGZ / Z-shape blitter | A Z-writing blitter exists but is not the selected normal building-body route under the checked flags | `Blitter_ZClip_Plain16_WritesZ @ 0x00497100` exists; normal body path reaches `Blitter_selector @ 0x00490B90` with `0x800` and no normal Z bits | Not active for normal selected building body |
| Building extras / pips / bracket-adjacent SHP overlays | The bracket line path itself uses `DrawLine3D`; adjacent SHP extras use `CC_Draw_Shape` and normal `TechnoClass_DrawSHP`/shape flags, not the line Z-write path | `TechnoClass::DrawExtras @ 0x006F5190`; health/pip reports; `TechnoClass_DrawSHP @ 0x00705E00` | Yes for draw order; depth write depends on each SHP flag path, not bracket path |
| VXL cached/uncached render | Representative normal VXL draw also builds flags from visual state and may select Z-capable blitters only for non-normal states; this was not exhausted for every visual mode | `TechnoClass__Draw @ 0x00706640`, `TechnoClass__Render @ 0x00706ED0`, `Blitter_selector_extended @ 0x00490E50`, `Blitter_ZBuf_Intensity25pct @ 0x00495BC0` | Conditional |

## 5. Integration Points

Normal full tactical rendering reaches:

```text
TacticalClass_Draw @ 0x006D3F50
  terrain/full phase:
    Tactical_ZBufferDirtyClear @ 0x006D2B60
    Tactical_layer_terrain_shadows @ 0x006D2DE0
    Tactical_layer_base_terrain / overlays / animations
  object phase:
    Tactical_ObjectRenderingLoop @ 0x006D8DB0
      per object: vtable+0x104 Draw_It
        selected building DrawBehind / DrawExtras
          Tactical::DrawLine3D @ 0x006DBB60
            g_PrimarySurface vtable+0x34
              Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30
```

Material finding: brackets run in the object phase after the terrain phase has had an opportunity to clear and populate `g_ZBuffer`; they do not clear it themselves. Active in YR: Yes for normal tactical frame order. Evidence: `0x006D3F50` phase order and `Tactical_ObjectRenderingLoop @ 0x006D8DB0`.

## 6. Current Rust Implementation Status

Read-only source scan found bracket instances in `src/app_selection_brackets.rs` and bracket draw ordering in `src/app_render/draw_passes.rs`. The existing bracket research index already notes that Rust does not yet model the original A-buffer modulation or primary-surface Z-test/no-Z-write behavior for bracket pixels. This report did not inspect or modify Rust beyond that read-only confirmation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Bracket line surface dispatch | verified | `Tactical::DrawLine3D @ 0x006DBB60`; prior vtable slot evidence in `SURFACE_DRAW_LINE_BRACKET_RASTER_GHIDRA_REPORT.md` | none |
| Surface depth buffer identity | verified | `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30`; `ZBuffer_scanline_ptr @ 0x007BD130` | none |
| Bracket Z-test and no-Z-write | verified | `0x004BFD30`, bracket callers `0x006F5EF0`, `0x006F5190`, `0x006F60D0` | none |
| Frame clear timing | verified | `TacticalClass_Draw @ 0x006D3F50`; `Tactical_ZBufferDirtyClear @ 0x006D2B60`; `ZBuffer_row_fill @ 0x007BCFB0` | no runtime dirty-rect log |
| Terrain/TMP writes | verified | `CellOverlay_TileDraw @ 0x00480350`; `TMP_TileBlitter @ 0x00547CF0` | none for ownership question |
| Terrain-shadow tile redraw writes | touched-not-exhausted | `Tactical_layer_terrain_shadows @ 0x006D2DE0`; `FUN_006D7F20`; `CellOverlay_TileDraw @ 0x00480350` | exact visual cases not enumerated |
| Normal building SHP body non-write | verified for normal selected-building path | `BuildingClass_DrawBody @ 0x0043D290`; `TechnoClass_DrawSHP @ 0x00705E00`; `Blitter_selector @ 0x00490B90`; `Blitter_Opaque_RLE_Remap @ 0x004978C0` | exotic visual states separate |
| BUILDNGZ/Z-writing blitter reachability | touched-not-exhausted | `Blitter_ZClip_Plain16_WritesZ @ 0x00497100` exists; normal route checked via `0x00705E00`/`0x00490B90` | all modded flag combinations |
| VXL writes | touched-not-exhausted | `TechnoClass__Draw @ 0x00706640`; `TechnoClass__Render @ 0x00706ED0`; `Blitter_selector_extended @ 0x00490E50`; `Blitter_ZBuf_Intensity25pct @ 0x00495BC0` | exhaustive visual-state matrix |
| Rust parity comparison | touched-not-exhausted | read-only `rg` of `src/app_selection_brackets.rs`, `src/app_render/draw_passes.rs` | implementation audit belongs to a code task |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does the selected building bracket line sample a depth buffer associated with `g_PrimarySurface` itself? No. The primary surface supplies the color destination and vtable dispatch, while `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30` explicitly samples global `g_ZBuffer`. Evidence: `0x006DBB60`, `0x004BFD30`, `0x007BD130`.

[RESOLVED] OQ-2 - Is the sampled buffer the same one terrain/TMP paths populate? Yes. `TMP_TileBlitter @ 0x00547CF0` obtains pointers through `ZBuffer_scanline_ptr` and writes the same global `g_ZBuffer` that `Surface::DrawLine_ABufModulated_ZClipped` samples. Active in YR: Yes.

[RESOLVED] OQ-3 - When is it cleared relative to brackets? In the terrain/full phase before terrain/shroud/base tile drawing, through `Tactical_ZBufferDirtyClear @ 0x006D2B60`; selected brackets are later in object rendering via `Tactical_ObjectRenderingLoop @ 0x006D8DB0`. Active in YR: Yes for full tactical draws; conditional for object-only draws.

[RESOLVED] OQ-4 - Do selected building brackets write depth? No. Bracket callers pass final flag `0`, and the surface routine writes `*zbuf` only when that flag byte is nonzero. Active in YR: Yes.

[RESOLVED] OQ-5 - Does normal building SHP body populate the bracket-sampled depth buffer? No for the checked normal selected-building body route. The normal route selects A-buffer/remap blitters that do not read/write `g_ZBuffer`; the Z-writing BUILDNGZ-capable blitter exists but is not reached by this route. Active in YR: Yes for normal body; conditional for non-normal visual modes.

[DEFERRED] OQ-6 - Which exact VXL/cloak/temporal states write `g_ZBuffer` before a selected building bracket in mixed scenes? Category: out-of-scope. This slot only checked representative VXL flag construction and one Z-capable blitter to classify ownership, not every visual state.

[DEFERRED] OQ-7 - Which exact terrain-shadow tile redraw cases produce player-visible bracket occlusion? Category: needs-runtime-debugger. Static evidence proves they use the same tile draw/Z path; pixel examples need runtime screenshots or watchpoints.

## Sources

- Ghidra decompiled/read-only: `0x006DBB60`, `0x004BFD30`, `0x007BD130`, `0x006D3F50`, `0x006D2B60`, `0x007BCFB0`, `0x00547CF0`, `0x00480350`, `0x006D2DE0`, `0x006D7F20`, `0x006D8DB0`, `0x006F5EF0`, `0x006F5190`, `0x006F60D0`, `0x0043D290`, `0x00705E00`, `0x00490B90`, `0x004978C0`, `0x00497100`, `0x00706640`, `0x00706ED0`, `0x00490E50`, `0x00495BC0`
- Prior docs cross-checked: `building-selection-brackets/BUILDING_BRACKET_ABUFFER_ZTEST_DEPTH_SEMANTICS_GHIDRA_REPORT.md`, `building-selection-brackets/SURFACE_DRAW_LINE_BRACKET_RASTER_GHIDRA_REPORT.md`, `building-selection-brackets/DRAWBRACKETCORNER_DRAWLINE3D_STUB_RASTER_GHIDRA_REPORT.md`, `ZBUFFER_DEPTH_SYSTEM.md`
- Source read-only scan: `C:/Users/enok/Documents/ra2-rust-game/src/app_selection_brackets.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_render/draw_passes.rs`
