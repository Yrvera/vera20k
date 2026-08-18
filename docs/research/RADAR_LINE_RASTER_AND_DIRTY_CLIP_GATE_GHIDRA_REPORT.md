# Radar Line Raster And Dirty Clip Gate - Ghidra Research Report

**Address(es):** `0x004BDF00`, `0x007BC2B0`, `0x00660050`, `0x00660540`, `0x004BE9D0`  
**Investigation Mode:** exhaustive-slice for radar-event line raster/dirty gates; coverage-map for unrelated users of the same surface helper.  
**Claimed Scope:** the rasterization helper used by ordinary `RadarEventClass` outline edges, its clipping contract, endpoint inclusion, Z/A-buffer pixel predicate, packed-color write path, and the `DAT_00880C98/94` dirty-rect gate around radar event/old-viewport helper lines.  
**Non-Scope:** radar event type/color/fade/radius table, radar object-dot priority, terrain dirty caller matrix, viewport camera overlay, minimap click provenance, live debugger DD mask sampling, and full caller census for non-radar VFX lines.  
**Confidence:** High for clipping, branch selection, endpoint inclusion, Z/A-buffer predicate, packed write mechanism, and dirty-gate branch shape from decompile plus disassembly. Medium for semantic names of the seven surface-line parameters because the vtable wrapper prototypes are not recovered cleanly.  
**Active in YR:** Yes for ordinary in-game radar events when the radar update draws a visible event type; conditional for the dirty-rect gate because the code requires both `DAT_00880C98 == 1` and `DAT_00880C94 == 1`.

## Summary

Radar-event outline edges are not drawn with a floating DDA or RGBA overwrite path. Native routes each edge through the shared gradient surface line helper `0x004BDF00`, which clips before rasterization, chooses a strict dominant-axis integer loop, includes the clipped start pixel, excludes the final endpoint, and writes 16-bit pixels only when the candidate line depth is strictly in front of the Z-buffer. The pixel color path unpacks the current destination pixel, adds A-buffer-scaled line-channel increments, saturates/rebalances overflow in `0x004BE9D0`, then repacks through runtime DirectDraw shift/loss globals.

The low-level line helper does not mark radar dirty rectangles itself. `DrawRadarEvent @ 0x00660050` and the old-geometry helper `DrawViewportRect @ 0x00660540` run a separate post-line clip/dirty block, and that block is skipped unless both `DAT_00880C98` and `DAT_00880C94` are `1`.

## Target and Non-Scope

Target question: What exact line raster and dirty/clip behavior should Rust reproduce for radar-event outline pixels?

Non-goals:

- Do not redo the already-settled event type/color/fade/radius table.
- Do not investigate the active camera viewport rectangle overlay assigned elsewhere.
- Do not expand into object-dot, terrain-cell, or click provenance swarms.
- Do not mutate Ghidra symbols or Rust code.

Evidence needed to mark COMPLETE:

- Decompile plus assembly for `0x004BDF00`.
- Decompile plus assembly for `0x007BC2B0`.
- Decompile plus assembly for the `DAT_00880C98/94` readers in `0x00660050` and `0x00660540`.
- A Rust-facing handoff naming current drift surfaces and acceptance scenarios.

Stop conditions:

- If the helper prototype cannot be named, still record observed stack/branch/pixel effects and mark only parameter names uncertain.
- If live runtime values are needed for globals or DD masks, defer that separately; do not block static raster proof.

## Verified Binary Findings

### 1. `0x004BDF00` skips very dim lines before locking

At function entry, `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00` reads three source color bytes and multiplies each by the incoming float scale/fade parameter before converting through `Math__ftol`.

Evidence:

- `0x004BDF12..0x004BDF7C`: read bytes `[EDI]`, `[EDI+1]`, `[EDI+2]`, multiply by stack float `[ESP+0x90]`, convert to integer.
- `0x004BDF7C..0x004BDF8D`: if all three converted channels are `< 8`, branch to return `0`.

Active in YR: Yes for radar-event outline calls that reach this surface helper. This means a low fade/scale can suppress the entire edge before clip/raster work.

### 2. Clip rect is obtained before raster branch selection

The helper asks the destination surface for a clip rect, wraps it through `AlphaShapeClass__ClipRect`, offsets both input endpoints by the rect origin, normalizes endpoint order by X, then calls `FUN_007BC2B0`.

Evidence:

- `0x004BDF93..0x004BDFB4`: surface vtable `+0x78`, then `AlphaShapeClass__ClipRect`.
- `0x004BDFDD..0x004BE007`: add clip `x/y` to both endpoints.
- `0x004BE00E..0x004BE03D`: if second endpoint X is less than first endpoint X, swap endpoints and associated per-end values.
- `0x004BE045..0x004BE067`: call `FUN_007BC2B0`; rejected lines return `0`.

Active in YR: Yes. Radar events call the surface path from `DrawRadarEvent @ 0x00660050` via `g_RadarDrawSurface` vtable slots.

### 3. `FUN_007BC2B0` is Cohen-Sutherland-style and clips to `x+w-1`, `y+h-1`

`FUN_007BC2B0 @ 0x007BC2B0` computes outcodes for both endpoints, rejects when the outcodes share an outside bit, and clips against the four rectangle edges. Left/top are inclusive; right/bottom visible maxima are `rect.x + rect.w - 1` and `rect.y + rect.h - 1`.

Evidence:

- `0x007BC2C7..0x007BC356`: computes `right = rect.x + rect.w`, `bottom = rect.y + rect.h`.
- `0x007BC3DA..0x007BC3DC`: shared outside outcode returns reject path.
- `0x007BC40A..0x007BC427`: bottom clip uses `bottom - 1`.
- `0x007BC42D..0x007BC45C`: right clip uses `right - 1`.
- `0x007BC529..0x007BC55E`: accepted clipped doubles are converted back through `Math__ftol` and written to both endpoint structs.

Active in YR: Yes. This same helper is called directly by both `DrawRadarEvent` and `DrawViewportRect` after line drawing too, but those post calls are for dirty/clip bookkeeping, not the low-level raster loop.

### 4. Raster loops are integer dominant-axis, start-inclusive, final-end-exclusive

After clipping, `0x004BDF00` computes nonnegative screen `dx`, absolute `dy`, and absolute third-axis/depth delta, then chooses one of three strict dominant-axis loops:

```text
if dx < dz && abs(dy) < dz: z-dominant
else if abs(dy) < dx:       x-dominant
else:                       y-dominant
```

Each loop writes/samples the current candidate pixel before stepping, and the loop count is the dominant delta, so the clipped start pixel is included and the final endpoint is excluded. A zero dominant delta falls through to success without a candidate write.

Evidence:

- `0x004BE2F7..0x004BE39E`: compute `dx`, `abs(dy)`, `abs(dz)`, doubled error terms.
- `0x004BE37D..0x004BE3AA`: strict branch selection.
- `0x004BE3E4..0x004BE58D`: z-dominant loop writes before step, decrements dominant count.
- `0x004BE5E1..0x004BE78F`: x-dominant loop writes before step, loops while index `< dx`.
- `0x004BE7E9..0x004BE99B`: y-dominant loop writes before step, decrements dominant count.

Active in YR: Yes for all radar-event edge segments that survive clipping and dim-skip. This explicitly differs from an inclusive `0..=steps` DDA.

### 5. Pixel predicate is strict Z-test; the helper does not write Z

Every raster branch samples `g_ZBuffer` and `g_ABuffer`. The visible write path is gated by strict unsigned 16-bit depth comparison:

```text
if (line_depth_uint16 < *zbuffer_pixel) {
    write framebuffer pixel
}
```

No matching write to `*zbuffer_pixel` appears in the gradient helper loops.

Evidence:

- `0x004BE2FD..0x004BE31F`: destination, Z, and A-buffer pointer setup.
- `0x004BE3E4..0x004BE3F5`, `0x004BE5E1..0x004BE5F2`, `0x004BE7E9..0x004BE7FA`: load candidate line depth and current Z-buffer sample.
- `0x004BE3EE..0x004BE3F5`, `0x004BE5EB..0x004BE5F2`, `0x004BE7F3..0x004BE7FA`: `CMP AX, word ptr [ESI]` then `JNC` skip means equal or behind does not write.
- No store to `[ESI]`/Z-buffer pointer occurs in the write blocks; only destination pixel writes occur at `0x004BE4A1`, `0x004BE69A`, `0x004BE8A2`.

Active in YR: Yes. Radar event lines are depth-tested against the current primary-surface Z-buffer state but do not update that Z-buffer.

### 6. Packed-color write is additive A-buffer modulation, not RGBA replace

When the Z-test passes, the helper unpacks the existing 16-bit destination pixel through the runtime DD shift/loss globals, multiplies the A-buffer word by the current line-channel terms, arithmetic-shifts the products by `7`, then calls `FUN_004BE9D0` to add those increments to the destination RGB and repack.

Evidence:

- Destination unpack in z-dominant branch: `0x004BE406..0x004BE45C`; x-dominant: `0x004BE5F8..0x004BE659`; y-dominant: `0x004BE800..0x004BE861`.
- A-buffer/channel products and `>> 7`: z-dominant `0x004BE464..0x004BE486`; x-dominant `0x004BE661..0x004BE683`; y-dominant `0x004BE869..0x004BE88B`.
- Repack/add helper call: z `0x004BE489..0x004BE4A1`; x `0x004BE686..0x004BE69A`; y `0x004BE88E..0x004BE8A2`.
- `FUN_004BE9D0 @ 0x004BE9D0` adds the three increments to base RGB bytes, redistributes overflow when one or two channels exceed `0xFE`, clamps each channel to `0xFF`, then packs via `g_DD_R/G/BShift` and `_g_DD_R/G/BLoss`.

Active in YR: Yes. This is the line-pixel contract used by radar-event outlines. A-buffer `0` produces zero increments and effectively preserves the destination color if the Z-test passes; it is not a separate alpha blend or source overwrite.

### 7. Circular Z/A-buffer pointer wrap is part of the line stepper

All three loops advance destination, Z, and A-buffer pointers in lockstep. Y movement advances the Z/A pointers by circular-buffer pitch and wraps using buffer start/end/span fields.

Evidence:

- Z-buffer start/end/span/pitch offsets read at `g_ZBuffer+0x18/+0x1C/+0x20/+0x28` in `0x004BE2C9..0x004BE2D4`, with wrap blocks at `0x004BE4D9..0x004BE4F5`, `0x004BE6D3..0x004BE6F3`, and `0x004BE922..0x004BE94A`.
- A-buffer uses the same wrap shape against `g_ABuffer+0x18/+0x1C/+0x20`, e.g. `0x004BE4F5..0x004BE51E`, `0x004BE6FA..0x004BE723`, and `0x004BE955..0x004BE97E`.

Active in YR: Yes, including radar-event lines. Rust cannot reproduce exact edge pixels with a stateless RGBA buffer unless it also models the native 16-bit primary radar surface plus Z/A sampling or proves equivalence for all relevant states.

### 8. Radar dirty rects are wrapper-side and gated by `DAT_00880C98 == 1 && DAT_00880C94 == 1`

`DrawRadarEvent @ 0x00660050` first draws four gradient line edges, then calls the clip helper once per edge pair, then checks both globals. If either global is not `1`, the dirty-rect update block is skipped entirely.

Evidence:

- Edge line dispatch: `0x006601F1..0x00660242` calls `g_RadarDrawSurface` vtable `+0x78` then `+0x90` four times.
- Post-line clip calls: `0x00660244..0x0066027F` calls `FUN_007BC2B0` four times.
- Gate: `0x00660281..0x00660299` tests `[0x00880C98] == 1` and `[0x00880C94] == 1`.
- Dirty update writes: `0x0066032F..0x00660350` writes `DAT_008809F4/F8/FC/A00`.

`DrawViewportRect @ 0x00660540`, the old-geometry helper called from `TickRadarEvent`, has the same gate:

- Line dispatch through vtable `+0x78` then `+0x44`: `0x0066057C..0x006605BB`.
- Post-line clip calls: `0x006605BD..0x006605F8`.
- Gate: `0x006605FA..0x00660613`.
- Dirty union/clip/write path: `0x00660619..0x00660712`.

Active in YR: Conditional. The code is active in standard radar-event tick/draw paths, but the dirty-rect side effect requires both globals to be set to `1`. Static direct-displacement search for `0x00880C98` and `0x00880C94` found the three reader sites already listed by this report family (`0x006539e7/ed`, `0x00660281/93`, `0x006605FA/0D`) and no direct setter in this slice.

### 9. Dirty rect union has `+1`/`+2` style inclusive expansion

The dirty blocks maintain rect extents from point/shape bounds, not a whole-minimap redraw. When expanding an existing dirty rectangle, the code uses inclusive-bottom/right correction by adding one after subtracting the old origin.

Evidence:

- `DrawViewportRect` dirty block expands existing `DAT_008809F4/F8/FC/A00` around each corner/edge point and uses `+0x2` for 1-pixel line/point extents at `0x00660688..0x006606A4`.
- `DrawOneSpySatellite @ 0x00430650` uses the same global dirty rect style and `+1` extent expansion for sprite bounds at `0x004307B7..0x004307DE`, then clips to `g_RadarSurfaceOriginX/Y` at `0x00430829..0x0043085E`.
- `DrawRadarEvent` uses `FUN_00487F40` before clipping and writing `DAT_008809F4/F8/FC/A00` at `0x0066030D..0x00660350`.

Active in YR: Yes when the corresponding dirty gate is active. The exact semantic names of `DAT_00880C98/94` remain unresolved, but the rect math shape and write sites are verified.

## Active in Standard YR?

- `DrawRadarEvent @ 0x00660050`: Active in standard YR when ordinary radar events are visible and radar update runs. The event-type/color table is settled in `RADAR_EVENT_PING_PIXEL_SHAPES_COLORS_GHIDRA_REPORT.md`.
- `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00`: Active for the above through `g_RadarDrawSurface` vtable dispatch.
- `FUN_007BC2B0 @ 0x007BC2B0`: Active for the line helper and radar wrapper post-line clip calls.
- `DAT_00880C98/94` dirty block: Conditional; verified code path, but static direct-displacement scan did not prove the standard runtime setter or default live value.
- No TS-only feature flag was found in the ordinary radar-event draw path. Runtime DD mask identity remains delegated to `DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS_GHIDRA_REPORT.md`.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Radar-event edge raster is clipped integer dominant-axis, start-inclusive/final-end-exclusive, not DDA inclusive | `0x004BDF00`, `0x007BC2B0`; current Rust DDA in `src/render/minimap_helpers.rs` | mismatch | `src/render/minimap.rs`, `src/render/minimap_helpers.rs`, future native radar surface helper | Replace radar-event line pixels with native clipped integer raster contract | A clipped edge ending exactly on the aperture boundary writes the native start-to-before-final pixel set and never writes `x+w`/`y+h` | `test_radar_event_line_raster_start_inclusive_final_exclusive` |
| Radar-event line pixels are strict-Z-tested and A-buffer-modulated additive 16-bit writes | `0x004BE3E4..0x004BE8A2`, `0x004BE9D0`; Rust overwrites RGBA via `set_pixel` | mismatch | future retained 16-bit radar primary surface; `src/render/minimap.rs` | Sample native Z/A buffers and repack through DD shift/loss path before sidebar copy | A radar event edge over a non-neutral A-buffer sample brightens/dims by native `abuf * channel >> 7` and equal-depth pixels do not draw | `test_radar_event_line_uses_ztest_abuffer_additive_pack16` |
| Low-level line helper does not mark dirty; wrapper dirty rect writes require both `DAT_00880C98` and `DAT_00880C94` | `0x00660281..0x00660350`, `0x006605FA..0x00660712` | mismatch risk: Rust full-refreshes minimap texture | retained radar/sidebar copy model in `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs` | Track line dirty rects in the radar wrapper stage, with a dormant/conditional gate for the two globals | With the gate off, event lines may alter the primary surface but do not expand `DAT_008809F4/F8/FC/A00`; with the gate on, old/new corner rects are clipped and copied | `test_radar_event_dirty_gate_requires_both_globals` |

## Negative Facts / Do Not Do

- Do not use `for 0..=steps` floating DDA for radar-event outlines; native loop counts exclude the final endpoint after clipping. Evidence: `0x004BE3E4..0x004BE99B`.
- Do not treat event-line drawing as an RGBA source overwrite; native unpacks the existing 16-bit destination and uses `FUN_004BE9D0` additive/saturating pack. Evidence: `0x004BE406..0x004BE4A1`, `0x004BE9D0`.
- Do not skip the Z-buffer for radar event lines; equal depth fails because the comparison is strict. Evidence: `CMP AX,[ESI]` / `JNC skip` in all three raster branches.
- Do not put dirty-rect writes inside the low-level line helper; radar-event dirtying is wrapper-side and gated after the four line calls. Evidence: `0x00660281..0x00660350`.
- Do not assume `DAT_00880C98` or `DAT_00880C94` are always true; this report proves readers and conditional behavior, not the standard runtime setter/default.

## Remaining Uncertainty

- Exact semantic names and standard runtime values for `DAT_00880C98` and `DAT_00880C94`; static direct-displacement scan found readers but no setter in this scoped pass.
- Exact surface vtable wrapper parameter names for `0x004BDF00`; the observed branch, clip, raster, and pixel effects are verified, but type names are inferred.
- Live RGB555/RGB565 runtime descriptor identity; channel shift/loss mechanism is verified but machine-specific masks need runtime sampling.
- Runtime screenshot capture of a radar event line over non-neutral A-buffer/Z states was not taken.

## Stale-Doc Replacement Wording

For `docs/research/RADAR_EVENT_PING_PIXEL_SHAPES_COLORS_GHIDRA_REPORT.md`, replace the remaining uncertainty wording:

> The radar-event line helper is now reduced for pixel/raster purposes: `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00` clips through `FUN_007BC2B0`, rasterizes with strict integer dominant-axis loops, includes the clipped start pixel, excludes the final endpoint, strict-Z-tests against `g_ZBuffer`, and writes additive A-buffer-modulated 16-bit packed pixels through `FUN_004BE9D0`. Dirty rect updates are wrapper-side in `DrawRadarEvent @ 0x00660050` and `DrawViewportRect @ 0x00660540`, gated by `DAT_00880C98 == 1 && DAT_00880C94 == 1`.

No existing published doc was patched by this swarm slot.

## Status

COMPLETE for the scoped static Ghidra slice: raster algorithm, clip bounds, endpoint inclusion, packed-color write mechanism, Z/A-buffer predicate, and radar-event dirty gate are verified.

PARTIAL only for live runtime values/names of `DAT_00880C98/94`, live DD mask identity, and screenshots of final pixels under specific runtime A/Z-buffer states.

## Sources

- Ghidra read-only decompile/disassembly: `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00`
- Ghidra read-only decompile/disassembly: `FUN_007BC2B0 @ 0x007BC2B0`
- Ghidra read-only decompile/disassembly: `DrawRadarEvent @ 0x00660050`
- Ghidra read-only decompile/disassembly: `DrawViewportRect @ 0x00660540`
- Ghidra read-only decompile: `FUN_004BE9D0 @ 0x004BE9D0`
- Ghidra read-only decompile/disassembly: `DrawOneSpySatellite @ 0x00430650` for sibling dirty-rect union style
- Prior report: `docs/research/RADAR_EVENT_PING_PIXEL_SHAPES_COLORS_GHIDRA_REPORT.md`
- Prior report: `docs/research/DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS_GHIDRA_REPORT.md`
- Rust scan only: `src/render/minimap.rs`, `src/render/minimap_helpers.rs`, `src/sim/radar.rs`, `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs`
