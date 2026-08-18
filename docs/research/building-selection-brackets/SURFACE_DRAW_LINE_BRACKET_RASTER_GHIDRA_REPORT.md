# Surface::Draw_Line Bracket Raster Path - Ghidra Report

**Report path:** `docs/research/SURFACE_DRAW_LINE_BRACKET_RASTER_GHIDRA_REPORT.md`  
**Target:** `Surface::Draw_Line` bracket raster path  
**Status:** COMPLETE  
**Active in YR:** Yes  
**Investigation mode:** read-only Ghidra/live decompilation; no Ghidra mutations, Rust edits, INI edits, or in-repo doc edits.

## Scope

Verify exact clipping, endpoint inclusion, integer stepping, and whether current Rust sprite-stepping can match `gamemd.exe` building selection bracket line rasterization.

## Verified Binary Evidence

1. `Tactical::DrawLine3D @ 0x006DBB60` calls the primary surface vtable slot `+0x34`; `g_PrimarySurface` vtable memory at `0x007E85D4 + 0x34 = 0x007E8608` contains `0x004BFD30`, resolving the bracket path to `Surface__DrawLine_ABufModulated_ZClipped @ 0x004BFD30`. Active in YR: Yes, because bracket helpers call `g_Tactical->vtable+0x60`.

2. The surface line routine clips before rasterization. `0x004BFD42..0x004BFD52` obtains a clip rect, `0x004BFD80..0x004BFDFF` offsets the input endpoints by the rect origin and calls `FUN_007BC2B0`. That helper is Cohen-Sutherland-style: left/top are inclusive, while right/bottom use `x + width - 1` and `y + height - 1` intersections (`0x007BC447`, `0x007BC454`). Shared outside outcodes return 0. Active in YR: Yes.

3. Endpoint inclusion is start-inclusive and end-exclusive after clipping. Each dominant loop writes before stepping and iterates exactly the dominant delta count: z-dominant decrements a count at `0x004C0353..0x004C0358`, x-dominant increments until `< dx` at `0x004C052A..0x004C053C`, and y-dominant decrements at `0x004C0717..0x004C0720`. A zero-length line reaches the success return without a pixel write (`0x004C055D..0x004C072F`). Active in YR: Yes.

4. Integer stepping is Bresenham-style in three dimensions, not just a 2D screen DDA. After clipping, the routine derives `dx`, `abs(dy)`, and absolute endpoint depth delta, then selects z-dominant (`0x004C0171..0x004C018F`), x-dominant (`0x004C0373..0x004C0554`), or y-dominant (`0x004C0557..0x004C0738`) paths. Each path updates error accumulators and advances screen X/Y and Z-buffer/A-buffer pointers in integer increments. Active in YR: Yes.

5. Building bracket lines Z-test but do not Z-write. `TechnoClass::DrawBracketCorner @ 0x006F5EF0` and direct `DrawExtras` sites pass final argument `0` into `DrawLine3D`; `Surface @ 0x004C024F..0x004C0265`, `0x004C043A..0x004C0450`, and `0x004C062B..0x004C0639` write the Z-buffer only when that final byte is nonzero. Active in YR: Yes.

## Rust Comparison

Verified source read, not binary evidence: `src/app_selection_brackets.rs` currently emits one `SpriteInstance` per pixel from `emit_line`. It computes:

```text
steps = ceil(max(abs(dx), abs(dy)))
px = round(a.x + step_x * i)
py = round(a.y + step_y * i)
for i in 0..steps
```

That matches the binary's end-exclusive shape only in the broad sense that it omits the final endpoint. It does not match the verified binary raster exactly because it:

- uses floating-point DDA and `round`, while `gamemd.exe` uses integer clipped endpoints and Bresenham-style error terms;
- ignores the surface routine's depth-delta-dominant path;
- lacks the binary clip contract of inclusive left/top and right/bottom clipped to `max - 1`;
- does not apply A-buffer modulation or Z-test semantics, though bracket calls do not Z-write.

Inference: sprite quads can match the visible bracket raster only if Rust first computes the exact same clipped integer pixel set and emits those pixels in the same order. The current `emit_line` implementation is close for simple shallow 2D segments but is not a parity-equivalent rasterizer.

## Open Questions

- What concrete bracket segments, if any, hit the z-dominant surface path in normal stock building selections? This report proves the surface path can use depth as the dominant axis, but did not enumerate all stock projected bracket deltas.
- Does the runtime A-buffer value ever suppress or dim selected-building bracket pixels in ordinary visible terrain/shroud states? This report confirms the A-buffer sample participates in the surface write, but not the full shroud-state matrix.

## Sources

- Ghidra decompile/disassembly: `Tactical::DrawLine3D @ 0x006DBB60`
- Ghidra decompile/disassembly: `Surface__DrawLine_ABufModulated_ZClipped @ 0x004BFD30`
- Ghidra decompile: clip helper `FUN_007BC2B0`
- Ghidra memory read: DSurface vtable `0x007E85D4..0x007E8624`
- Ghidra decompile: `TechnoClass::DrawBracketCorner @ 0x006F5EF0`
- Source read only: `src/app_selection_brackets.rs`
