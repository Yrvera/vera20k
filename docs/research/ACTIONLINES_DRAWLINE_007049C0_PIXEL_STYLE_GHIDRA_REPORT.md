# ActionLines_DrawLine 0x007049C0 Pixel Style - Ghidra Research Report

**Address(es):** `0x007049C0` (primary helper), `0x004DC060` (selected-unit caller), `0x006D473F` (stock tactical dispatch), `0x006D2140` (coordinate projection), `0x007BC2B0` (line clipping)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact pixel style of `ActionLines__DrawLine` as reached by selected-unit action/target lines, including projection, viewport offsets, endpoint rectangles, clipping, solid/dashed branch, dash pattern/phase, target surface, and selected-unit dashed reachability.
**Non-Scope:** enemy/AI `DrawRadarActionLines @ 0x004DC340`, timer start/clear xrefs, option dialog plumbing beyond the immediate selected-unit gate, final internals of the surface vtable implementations behind `+0x14/+0x30/+0x4C`.
**Confidence:** High for helper logic and selected-unit reachability; Medium for final surface raster internals because this slice verifies call shape/arguments, not the vtable target implementations.
**Active in YR:** Yes for the solid selected-unit path when `[Options] UnitActionLines`/`DAT_00843108` is enabled; dashed mode is Conditional but not reached by stock selected-unit tactical dispatch.

## 1. Overview

`ActionLines__DrawLine @ 0x007049C0` converts two world-coordinate endpoints to tactical client pixels, shifts Y into the tactical viewport surface area, draws 3x3 endpoint rectangles, clips the line to the tactical viewport rectangle, then draws either one solid line or one dashed line on `DAT_0088731C`.

The starting report's "two parallel solid lines offset by (-2,-2)" claim does not match the verified selected-unit path. The active selected-unit path draws two clipped 3x3 endpoint boxes offset by `(-2,-2)` from each endpoint, then one clipped line segment. There is no verified selected-unit body-thickening pass in this helper.

## 2. Key Arguments / Globals

| Item | Evidence | Purpose | Active in YR |
|---|---|---|---|
| `ActionLines__DrawLine` stack cleanup `RET 0x24` | `0x00704E3C` | Helper consumes 36 bytes: start coord, end coord, packed RGB/color bytes, dashed flag, optional pre-pass flag. | Yes |
| `DAT_0088731C` | calls at `0x00704D0A..0x00704D11`, `0x00704E1D..0x00704E30` | Target draw surface for endpoint rectangles and final line. | Yes |
| viewport rect globals `0x00886FA0..0x00886FAC` | `0x00704BB7..0x00704BD6` | Clip rectangle `{x,y,w,h}` passed to line clipper. | Yes |
| viewport Y offset `0x00886FA4` | `0x007049FD..0x00704A23` | Added to both projected Y values after `CoordsToClient2`. | Yes |
| current frame `0x00A8ED84` | `0x00704DB0..0x00704DC5` | Dashed phase source. | Conditional; only if dashed flag is nonzero. |
| dash pattern `0x00843128` | `read_memory 0x00843128 = 01 01 01 01 01 00 00 00 ...`; pushed at `0x00704DE6` | Pattern pointer for dashed line. | Conditional; not selected-unit stock path. |

## 3. Core Logic

### 3.1 Projection

The helper calls `TacticalClass__CoordsToClient2 @ 0x006D2140` twice, once per 3D endpoint (`0x007049CD..0x007049F4`). `CoordsToClient2` computes isometric screen coordinates using 60-pixel cell width and 30-pixel cell height constants:

```text
iso_x_raw = (x * 0x3C) / 2 + (y * -0x3C) / 2
iso_y_raw = (x * 0x1E) / 2 + (y *  0x1E) / 2
screen_x = signed_trunc_toward_zero(iso_x_raw / 256) - tactical+0xB0
screen_y = signed_trunc_toward_zero(iso_y_raw / 256) - AdjustForZ(z) - tactical+0xB4
```

`AdjustForZ(z)` is computed by the floating-point block at `0x006D21DE..0x006D21F7`: it multiplies Z by global `0x00B0CD48`, adds `1` first when `z >= 0x2D8`, adds `0.5` from `0x007E1738`, then calls `Math__ftol`.

Active in YR: Yes. `ActionLines__DrawLine` unconditionally uses this projection for selected-unit attack and movement lines.

### 3.2 Viewport Offsets

After projection, `ActionLines__DrawLine` adds `0x00886FA4` to both endpoint Y values (`0x007049FD..0x00704A23`). It does not add the viewport X global to endpoint X in this helper. X is later used through the clipping rectangle's left edge at `0x00886FA0`.

Active in YR: Yes. This occurs before all rectangle/line drawing.

### 3.3 Endpoint Rectangles, Not Parallel Thickness

For the selected-unit stock path, the helper draws two endpoint rectangles with the caller's RGB-converted color:

1. Build a point offset `(-2,-2)` from one endpoint via `AddPoints @ 0x00437F10`.
2. Create a rectangle `{x,y,3,3}` via `FUN_0045A130`.
3. Clip that rectangle against the viewport rect via `AlphaShapeClass__ClipRect @ 0x00421B60`.
4. Call `DAT_0088731C` vtable slot `+0x14` with the clipped rect and the caller color.
5. Repeat for the other endpoint.

Evidence: first active selected-unit endpoint rectangle block at `0x00704C8B..0x00704D11`; second at `0x00704D14..0x00704DA2`. `FUN_0045A130` stores four rect fields verbatim; `AlphaShapeClass__ClipRect @ 0x00421B60` zeroes the rect if width/height fall out of bounds.

Active in YR: Yes. These endpoint rectangles are outside the optional pre-pass and run before the solid/dashed branch.

### 3.4 Optional Pre-Pass

If the final helper flag at stack byte `[ESP+0xB0]` is nonzero, an earlier block draws two similar 3x3 endpoint rectangles and a clipped solid line using the first palette-table entry from `ConvertClass+0x174`. `TechnoClass__DrawActionLines @ 0x004DC060` passes this final flag as `0` for both attack and movement calls (`0x004DC12F`, `0x004DC2B6`), so this pre-pass is not stock selected-unit output.

Active in YR: Conditional. The code exists, but stock selected-unit tactical calls pass zero and skip it.

### 3.5 Solid vs Dashed Final Line

After endpoint rectangles, the helper clips the endpoint pair through `FUN_007BC2B0 @ 0x007BC2B0`.

Solid path:

- Gate: dashed flag byte at `[ESP+0xAC]` equals `0`.
- Evidence: `0x00704DA5..0x00704E30`.
- If clipping succeeds, calls `DAT_0088731C` vtable slot `+0x30` with start point, end point, and caller color.
- Active in YR: Yes for stock selected-unit lines.

Dashed path:

- Gate: dashed flag byte at `[ESP+0xAC]` is nonzero.
- Evidence: `0x00704DA5..0x00704DF8`.
- Phase formula is signed `IDIV` remainder: `(0x7FFFFFFF - g_CurrentFrameCounter) % 0xF`.
- Pattern pointer is `0x00843128`; memory is `1,1,1,1,1,0,0,0` repeated at least twice.
- Calls `DAT_0088731C` vtable slot `+0x4C` with start, end, color, pattern pointer, phase pointer/value register, and a final zero argument.
- Active in YR: Conditional. The helper supports it, but stock selected-unit callers do not set the dashed flag.

### 3.6 Line Clipping

`FUN_007BC2B0 @ 0x007BC2B0` is a Cohen-Sutherland-style 2D line clipper:

- Region bits: left `1`, right `2`, bottom/high-Y `4`, top/low-Y `8`.
- Rect right and bottom limits are exclusive in effect; intersections use `rect.x + rect.w - 1` and `rect.y + rect.h - 1`.
- On accept it writes clipped integer endpoints back through `Math__ftol`; on trivial reject it returns `0`.

Active in YR: Yes. Both solid and dashed final-line branches call it before drawing.

## 4. INI Keys

| Key | Evidence | Effect | Active in YR |
|---|---|---|---|
| `[Options] UnitActionLines` | `OPTIONS_DIALOG_CASE5_AND_FIELD_MAP_GHIDRA_REPORT.md:157`, selected-unit call gate reads `DAT_00843108` at `0x006D473F..0x006D4746` | Enables/disables selected-unit call into vtable `+0x438`, which reaches `TechnoClass__DrawActionLines` and then this helper. | Yes, conditional on user option. |

No line thickness, dash length, endpoint size, or color-style INI key was found for this helper in the repo INI scan. The endpoint size `3`, offset `-2`, dash modulus `0xF`, and pattern address are binary constants.

## 5. Integration Points

Selected-unit stock dispatch is in the tactical draw loop at `0x006D473F..0x006D4750`:

```text
read DAT_00843108
if zero: skip
push 0
push 0
call selected techno vtable+0x438
```

`TechnoClass__DrawActionLines @ 0x004DC060` then calls `ActionLines__DrawLine`:

- Attack/archive target branch: helper dashed flag `0`, optional pre-pass flag `0` (`0x004DC12F..0x004DC19B`).
- Movement/nav branch: helper dashed flag is the `DrawActionLines` parameter, optional pre-pass flag `0` (`0x004DC2B6..0x004DC323`).

Because the stock tactical dispatch pushes `0` for the `DrawActionLines` parameter, both stock selected-unit attack and movement lines reach the solid branch, not the dashed branch.

Active in YR: Yes for the call chain; dashed selected-unit mode is not active in stock selected-unit calls.

## 6. Current Rust Implementation Status

Rust has an app-layer target-line implementation in `src/app_target_lines.rs`. It uses a 25-tick timer and emits `1x1` `SpriteInstance` pixels along a rounded float DDA line (`emit_colored_line`), with hardcoded approximate RGB colors. It does not currently reproduce the verified `ActionLines__DrawLine` endpoint 3x3 rectangles, binary projection/viewport offset order, palette-derived colors, clip-to-viewport behavior, or optional dashed branch.

Evidence: `src/app_target_lines.rs:18`, `src/app_target_lines.rs:21`, `src/app_target_lines.rs:23`, `src/app_target_lines.rs:135`, `src/app_target_lines.rs:214`.

Active in YR: N/A for Rust status; this is implementation comparison only.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ActionLines__DrawLine @ 0x007049C0` selected-unit path | verified | decompile + assembly `0x007049C0..0x00704E3C` | none for call shape |
| Projection via `TacticalClass__CoordsToClient2 @ 0x006D2140` | verified | decompile + assembly `0x006D2140..0x006D226F` | final runtime value of Z multiplier global not statically initialized |
| Endpoint rectangle construction | verified | `0x00704C8B..0x00704D11`, `0x00704D14..0x00704DA2`, `0x0045A130`, `0x00421B60` | none |
| Solid final line branch | verified | `0x00704E07..0x00704E30` | surface vtable target internals deferred |
| Dashed final line branch | verified | `0x00704DB0..0x00704DF8`, `read_memory 0x00843128` | surface vtable target internals deferred |
| Selected-unit dashed reachability | verified | `0x006D474A..0x006D4750`, `0x004DC12F`, `0x004DC2B6` | none |
| Optional pre-pass final flag | verified | `0x00704A34..0x00704BB7`; caller passes zero | no stock caller search outside selected-unit scope |
| Enemy/AI `DrawRadarActionLines` | deferred | out-of-scope by slot request | handled by slot 4 |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does selected-unit `ActionLines__DrawLine` draw a thick/parallel body line? No verified selected-unit body-thickening pass exists; selected path draws 3x3 endpoint rectangles and one final line. Evidence: `0x00704C8B..0x00704E30`; optional pre-pass skipped by caller final flag `0`.

[RESOLVED] OQ-2 - What is the endpoint marker size and offset? Each endpoint rect starts at endpoint plus `(-2,-2)` and has size `3x3`, then is clipped. Evidence: pushes `0x3`, `0x3`, stores `0xFFFFFFFE`, and calls `FUN_0045A130`/`AlphaShapeClass__ClipRect` at `0x00704C8B..0x00704D11` and `0x00704D14..0x00704DA2`.

[RESOLVED] OQ-3 - Which surface receives the pixels? `DAT_0088731C` receives both endpoint rectangle calls at vtable `+0x14` and final line calls at `+0x30`/`+0x4C`. Evidence: `0x00704CFD..0x00704D11`, `0x00704DDD..0x00704DF8`, `0x00704E1D..0x00704E30`.

[RESOLVED] OQ-4 - What clips the final line? `FUN_007BC2B0` clips against rect globals `0x00886FA0..0x00886FAC`, using right/bottom `-1` intersections and `Math__ftol` writes. Evidence: decompile `0x007BC2B0`.

[RESOLVED] OQ-5 - What is the dashed pattern and phase? Pattern at `0x00843128` is `1,1,1,1,1,0,0,0`; phase is `(0x7FFFFFFF - g_CurrentFrameCounter) % 0xF`. Evidence: `read_memory 0x00843128`; `0x00704DB0..0x00704DE6`.

[RESOLVED] OQ-6 - Do stock selected-unit calls reach dashed mode? No. Tactical dispatch pushes two zero args at `0x006D474A..0x006D4750`; attack branch hardcodes helper dashed flag `0`, movement branch forwards the zero `DrawActionLines` arg. Evidence: `0x004DC12F..0x004DC19B`, `0x004DC2B6..0x004DC323`.

[DEFERRED] OQ-7 - What are the exact internal raster rules of surface vtable `+0x14`, `+0x30`, and `+0x4C` on `DAT_0088731C`? Category: out-of-scope. Reason: this slice verifies helper pixel style and arguments; surface implementation internals are a separate substrate investigation.

## Sources

- Ghidra decompile/assembly: `ActionLines__DrawLine @ 0x007049C0`
- Ghidra decompile/assembly: `TechnoClass__DrawActionLines @ 0x004DC060`
- Ghidra assembly: selected-unit dispatch at `0x006D473F..0x006D4750`
- Ghidra decompile/assembly: `TacticalClass__CoordsToClient2 @ 0x006D2140`
- Ghidra decompile: `FUN_007BC2B0`
- Ghidra decompile: `FUN_0045A130`, `AlphaShapeClass__ClipRect @ 0x00421B60`, `AddPoints @ 0x00437F10`
- Ghidra memory read: `0x00843128`
- Prior docs used as starting context: `TARGET_LINES_GHIDRA_REPORT.md`, `OPTIONS_DIALOG_CASE5_AND_FIELD_MAP_GHIDRA_REPORT.md`
- Rust scan: `src/app_target_lines.rs`
