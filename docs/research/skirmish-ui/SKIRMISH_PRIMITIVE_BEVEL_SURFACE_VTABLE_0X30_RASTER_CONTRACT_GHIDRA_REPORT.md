# Skirmish Primitive Bevel Surface Vtable +0x30 Raster Contract - Ghidra Research Report

**Address(es):** `FUN_006208F0 @ 0x006208F0`; `DSurface vtable @ 0x007E85D4`; `DSurface +0x30 wrapper @ 0x007BA5E0`; line worker `0x007BA610`; clip helper `0x007BC2B0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The concrete destination-surface `+0x30` line call reached by `FUN_006208F0` for standard offline Skirmish dialog `0x102` primitive combo/list/dropdown frames and trackbar bevel rails. This verifies axis-aligned endpoint inclusion, destination clipping, native-pixel color consumption, and Rust-facing implications.  
**Non-Scope:** Full DirectDraw/DSurface method inventory, generic SHP/text blitters, non-axis-aligned line visual parity outside this helper, runtime screenshot capture, and unrelated Win32 control behavior.  
**Confidence:** High for the axis-aligned line contract used by `FUN_006208F0`; Medium for diagonal line stepping because scoped Skirmish primitive frames do not issue diagonal lines.  
**Active in YR:** Yes. Standard offline dialog `0x102` installs the scoped owner-draw callbacks through `FUN_0060F9A0`, and those callbacks call `FUN_006208F0`, which dispatches `DAT_00887310` vtable `+0x30`.

## 1. Overview

`FUN_006208F0` does not hand the renderer abstract rectangle strokes. It emits four explicit line calls per ring to the destination `DSurface +0x30` slot, and those line calls are software-rastered as inclusive pixel spans after clipping to the destination surface bounds.

Active in YR: Yes. Evidence: `FUN_006208F0 @ 0x006208F0` calls `(**surface_vtable + 0x30)` for each frame edge; `DAT_00887310` uses `DSurface vtable @ 0x007E85D4`; file bytes for `0x007E85D4+0x30` resolve to `0x007BA5E0`.

## 2. Key Surface Slots

| Slot | Target | Verified behavior | Active in YR |
|---|---:|---|---|
| `DSurface +0x30` | `0x007BA5E0` | Thin line wrapper. It receives `point_a`, `point_b`, and already-native color, obtains a full-surface clip rect through `+0x78`, then calls worker `0x007BA610`. | Yes; reached from `FUN_006208F0` on scoped Skirmish controls. |
| worker | `0x007BA610` | Clips endpoints, locks the first visible pixel through `+0x5C`, writes inclusive horizontal/vertical spans, unlocks through `+0x60`, and returns success/failure. | Yes; direct callee of `+0x30`. |
| clip helper | `0x007BC2B0` | Cohen-Sutherland-style line clip against `[x,y,w,h]` bounds; right/bottom are exclusive, so max visible endpoints are `x+w-1` and `y+h-1`. | Yes; direct callee of worker. |
| `DSurface +0x5C` | `0x004BAD80` | Locks and returns `base + pitch*y + bytes_per_pixel*x`; rejects negative `x/y`; updates bytes-per-pixel and pitch on first lock. | Yes; worker uses it before writes. |
| `DSurface +0x60` | `0x004BAF40` | Unlocks/decrements lock depth; worker calls it after a non-null lock. | Yes. |
| `DSurface +0x70` | `0x004BAD60` | Returns bytes-per-pixel from surface field `+0x10`. | Yes. |
| `DSurface +0x74` | `0x004BAD70` | Returns scanline pitch from nested surface descriptor. | Yes. |
| `DSurface +0x78` | `0x00411510` | Writes full bounds `{0,0,width,height}` from fields `+0x04/+0x08`. | Yes. |

## 3. Raster Contract Used By Bevel Frames

The scoped primitive frames only issue horizontal and vertical segments. For those lines:

- Endpoints are inclusive after clipping. Active in YR: Yes. Evidence: horizontal loop uses `x2 - x1 + 1` at `0x007BA6F3..0x007BA71A` for 8-bit and `while eax <= x2-x1` at `0x007BA72F..0x007BA758` for non-8-bit; vertical loop uses `abs(y2-y1)+1` at `0x007BA793..0x007BA7BE`.
- Horizontal lines are normalized left-to-right before locking. Active in YR: Yes. Evidence: if clipped `x1 > x2`, worker swaps both endpoints at `0x007BA69E..0x007BA6B6`.
- Vertical lines keep x fixed and step by positive or negative pitch depending on clipped y ordering. Active in YR: Yes. Evidence: pitch from `+0x74` is negated when `y1 > y2` at `0x007BA779..0x007BA793`.
- The lock starts at the first clipped visible endpoint. Active in YR: Yes. Evidence: worker calls `+0x5C(x1,y1)` at `0x007BA6C1..0x007BA6D5` after clipping and endpoint normalization.
- A rejected/off-screen line returns false and writes no pixels. Active in YR: Conditional; the code path is live, but standard `0x102` frame rectangles normally sit inside the 800x600 shell surface. Evidence: `0x007BC2B0` returns `0` on shared outcode rejection; worker returns `0` at `0x007BA8B1..0x007BA8BA`.

`FUN_006208F0` edge emission remains as resolved by the prior color report:

```text
top:    (left, top)         -> (right - 1, top)
left:   (left, top + 1)     -> (left, bottom)
bottom: (right, bottom)     -> (left, bottom)
right:  (right, bottom - 1) -> (right, top + 1)
```

Because `+0x30` writes inclusive endpoints, the `right - 1`, `top + 1`, and `bottom - 1` adjustments are load-bearing. Active in YR: Yes. Evidence: helper line-call setup in `FUN_006208F0 @ 0x00620A1A..0x00620B7A`, plus inclusive span loops in worker `0x007BA610`.

## 4. Clipping Contract

The `+0x30` wrapper supplies full destination-surface bounds as its clip rectangle. For `DAT_00887310` in the Skirmish shell, there is no caller-supplied child-control scissor in this line API; clipping is to the destination `DSurface` bounds returned by `+0x78`.

Active in YR: Yes. Evidence: wrapper `0x007BA5E0` calls `surface +0x78` before worker `0x007BA610`; `0x00411510` fills `{0,0,width,height}`.

The clip helper treats rect width/height as extents, not last coordinates:

- left/top clipping clamps to `x` and `y`;
- right clipping clamps to `x + w - 1`;
- bottom clipping clamps to `y + h - 1`;
- accepted clipped endpoints are converted back to integers through `Math__ftol`.

Active in YR: Yes. Evidence: `0x007BC2B0` computes `right = rect.x + rect.w`, tests outside when endpoint `>= right`, and clips to `right - 1`; bottom does the same with `rect.y + rect.h - 1`.

## 5. Color Source And Conversion Labels

`DSurface +0x30` consumes a native packed surface pixel value. It does not convert `0x00BBGGRR` RGB itself.

Active in YR: Yes. Evidence: worker writes the color argument directly as a byte when bytes-per-pixel is `1`, or as a 16-bit word when bytes-per-pixel is not `1`, at `0x007BA6F3..0x007BA75A` and `0x007BA7AF..0x007BA7BB`.

For scoped Skirmish primitive frames, `FUN_006208F0` is the conversion boundary:

- input globals are 24-bit shell colors such as `DAT_00AC1B98 = 0x00C5BEA7` and `DAT_00AC1B94 = 0x00807A68`;
- `FUN_006208F0` converts them with `g_DD_*Loss` and `g_DD_*Shift`;
- the converted value is then passed to `+0x30` and `+0x24`.

Active in YR: Yes. Evidence: conversion in `FUN_006208F0 @ 0x00620917..0x00620A16`; native write in `0x007BA610`; `+0x24` point write has the same native-pixel expectation at `0x007BAEB0`.

No INI key participates in this raster path. Active in YR: Yes as binary UI code. Evidence: no INI reads in `FUN_006208F0`, `0x007BA5E0`, `0x007BA610`, or `0x007BC2B0`; caller color globals are initialized by owner-draw setup, not rules/art INI.

## 6. Current Rust Implementation Status

Current Rust Skirmish shell rendering still lacks this primitive frame surface. Codegraph and `rg` find shell chrome/buttons/flags/preview roles in `src/app_skirmish_shell_render.rs`, color-combo layout in `src/ui/skirmish_shell/layout.rs`, and hit-testing in `src/ui/skirmish_shell/state.rs`, but no `FUN_006208F0` analogue, no primitive bevel draw role, and no combo/list/dropdown/trackbar rail renderer.

Active in YR: Not applicable to Rust. Evidence: Codegraph context for Skirmish shell renderer and `rg -n "bevel|track|trakgrip|trofm|combo|dropdown|primitive"` over `src/app_skirmish_shell_render.rs`, `src/ui`, and `src/render`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_006208F0` edge endpoints | verified | `0x00620A1A..0x00620B7A` | none for scoped frames |
| `DSurface vtable +0x30` binding | verified | `gamemd.exe` bytes at `0x007E85D4+0x30 -> 0x007BA5E0` | none |
| `+0x30` wrapper clip source | verified | assembly `0x007BA5E0..0x007BA60D`, `+0x78 @ 0x00411510` | none |
| axis-aligned line raster | verified | `0x007BA6E2..0x007BA7D0` | none |
| line clip helper bounds | verified | `0x007BC2B0` | exact `Math__ftol` rounding mode is irrelevant for in-bounds scoped axis-aligned frames |
| diagonal line stepping | touched-not-exhausted | `0x007BA7D3..0x007BA8AE` | out-of-scope unless a future Skirmish primitive emits diagonals |
| full DirectDraw lock/restore behavior | deferred | `+0x5C/+0x60` touched | broader surface investigation, not needed for bevel endpoints |
| current Rust primitive shell frames | verified | Codegraph + `rg` over `src/` | missing feature; implement later |

## 8. Open Questions - Final State

- [RESOLVED] OQ1 - Is `FUN_006208F0` active on standard offline Skirmish controls? -> Yes; combo/list/trackbar owner-draw callers installed for dialog `0x102` call it. (evidence: prior caller reports; `0x00617893`, `0x0061926B`, `0x0061E204`, `0x0061E269`)
- [RESOLVED] OQ2 - Which concrete function is destination-surface vtable `+0x30`? -> `DSurface +0x30` resolves to wrapper `0x007BA5E0`, which calls worker `0x007BA610`. (evidence: `gamemd.exe` vtable bytes at `0x007E85D4+0x30`)
- [RESOLVED] OQ3 - Are line endpoints inclusive or half-open? -> Inclusive after clipping for horizontal and vertical lines. (evidence: `0x007BA6F3..0x007BA75A`, `0x007BA793..0x007BA7BE`)
- [RESOLVED] OQ4 - What clipping rectangle is used by this API? -> Full destination `DSurface` bounds from `+0x78`, not child-control-local scissor. (evidence: wrapper `0x007BA5E0`, `0x00411510`)
- [RESOLVED] OQ5 - Does `+0x30` convert RGB colors? -> No; it writes the native packed pixel supplied by the caller. (evidence: direct byte/word writes in `0x007BA610`; conversion happens in `FUN_006208F0`)
- [RESOLVED] OQ6 - Do right/bottom clip edges use inclusive rect coordinates? -> No; rect fields are extents, and right/bottom visible maxima are `x+w-1` and `y+h-1`. (evidence: `0x007BC2B0`)
- [RESOLVED] OQ7 - Does an off-screen clipped-out line write or lock? -> No successful write path; helper returns false before locking when clipping rejects. (evidence: `0x007BC2B0` rejection, worker `0x007BA68E..0x007BA690`, `0x007BA8B1`)
- [DEFERRED] OQ8 - What is exact diagonal Bresenham parity for future non-frame primitives? (category: out-of-scope; reason: scoped Skirmish bevel frames emit only horizontal/vertical lines; next-step-if-pursued: isolate a live diagonal primitive caller and screenshot-check stepping)
- [DEFERRED] OQ9 - What are all DirectDraw lost-surface restore side effects below `+0x5C/+0x60`? (category: out-of-scope; reason: this slice only needs successful shell surface pixel contract; next-step-if-pursued: separate DSurface lock lifecycle report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| `FUN_006208F0` line endpoints rely on inclusive `+0x30` spans. | `0x00620A1A..0x00620B7A`; `0x007BA6F3..0x007BA7BE` | missing | needed primitive renderer under `src/app_skirmish_shell_render.rs` or a small shell-surface helper | Draw all four bevel edges with inclusive endpoint semantics after applying the helper's `right-1/top+1/bottom-1` adjustments. | A 2-pixel combo-face bevel at 800x600 has no missing corner and no 1-pixel overrun on the top/right/bottom edges. | `skirmish_primitive_bevel_edges_are_inclusive` | Do not use half-open line spans or generic stroke rectangles. |
| `+0x30` clips to full destination surface bounds with right/bottom as exclusive extents. | `0x007BA5E0`, `0x00411510`, `0x007BC2B0` | missing | shell primitive raster tests | Clip partially off-screen primitive lines to `0..w-1`/`0..h-1` before writing; no child-control scissor is part of this line API. | A synthetic bevel partly outside a tiny test surface writes only visible edge pixels and never wraps/underflows. | `skirmish_primitive_line_clips_to_surface_extents` | Do not clip these primitive frame lines to the combo/list child rectangle unless a higher caller supplies a separate clip path. |
| `+0x30` consumes native packed pixels; RGB-to-DD conversion belongs before the line call. | `FUN_006208F0 @ 0x00620917..0x00620A16`; worker writes at `0x007BA6F3..0x007BA75A` | missing | shell color conversion / primitive pixel buffer bridge | Convert `0x00BBGGRR` shell globals to the target native pixel once, then write that native value directly in the line raster. | A rail bevel test observes the converted `0xC5BEA7`/`0x807A68` pair in the same pixel labels as the binary helper, not unconverted RGB bytes. | `skirmish_primitive_bevel_uses_native_converted_pixels` | Do not pass sRGB `Color32` values into the low-level raster and convert per write. |

## Negative Facts / Do Not Do

- Do not implement these frames as egui/native widget borders. Active in YR: Yes. Evidence: scoped paint paths call `FUN_006208F0` and then `DSurface +0x30`, not toolkit chrome (`0x00617893`, `0x0061926B`, `0x0061E204/269`).
- Do not treat the helper's line endpoints as half-open. Active in YR: Yes. Evidence: `+0x30` horizontal and vertical branches write `delta + 1` pixels (`0x007BA6F3..0x007BA7BE`).
- Do not re-convert colors inside the line raster. Active in YR: Yes. Evidence: `FUN_006208F0` performs `g_DD_*Loss/Shift` conversion before dispatch; `0x007BA610` writes the supplied native value directly.
- Do not assume child-control-local clipping for `+0x30`. Active in YR: Yes. Evidence: wrapper obtains `DSurface +0x78` full bounds (`0x007BA5FE`, `0x00411510`).
- Do not replace trackbar rail primitive bevels with `BTN-MINS.SHP`/`BTN-PLUS.SHP`. Active in YR: No for standard offline Skirmish trackbars. Evidence: trackbar owner draw calls `FUN_006208F0` at `0x0061E204/0x0061E269`; plus/minus SHPs belong to a different generic slider path per prior report.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`: replace the confidence sentence fragment "Medium for exact primitive rail bevel pixel appearance because `FUN_006208F0` is shared beveled-rectangle code and this slot did not screenshot-match its raster output." with "High for static primitive rail bevel endpoint, color-label, and clipping contract per `SKIRMISH_PRIMITIVE_BEVEL_SURFACE_VTABLE_0X30_RASTER_CONTRACT_GHIDRA_REPORT.md`; runtime screenshot validation remains optional for final monitor RGB."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`: replace coverage row "trackbar primitive rail raster | touched-not-exhausted ... screenshot/runtime validation for exact bevel pixels" with "trackbar primitive rail raster | verified-static | `FUN_006208F0 @ 0x006208F0`, DSurface `+0x30 -> 0x007BA5E0/0x007BA610` | runtime screenshot validation only."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`: replace coverage row "Surface vtable line/point internals | deferred" with "Surface vtable `+0x30` axis-aligned line internals | verified-static | `SKIRMISH_PRIMITIVE_BEVEL_SURFACE_VTABLE_0X30_RASTER_CONTRACT_GHIDRA_REPORT.md`, `0x007BA5E0`, `0x007BA610` | diagonal lines remain out-of-scope."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`: replace "exact line colors are broader owner-draw chrome context" with "primitive frame colors are resolved by `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`, and axis-aligned line endpoint/clipping raster is resolved by `SKIRMISH_PRIMITIVE_BEVEL_SURFACE_VTABLE_0X30_RASTER_CONTRACT_GHIDRA_REPORT.md`."

## Remaining Uncertainty

- Exact diagonal `+0x30` stepping is not claimed because the scoped Skirmish primitive frames issue only horizontal and vertical lines.
- Final monitor RGB still depends on the active DirectDraw pixel format globals and display path; this report labels native-pixel behavior, not a captured screenshot.

## Sources

- Ghidra read-only decompile/assembly: `FUN_006208F0 @ 0x006208F0`
- Ghidra read-only assembly/decompile: `DSurface +0x30 wrapper @ 0x007BA5E0`, worker `0x007BA610`, clip helper `0x007BC2B0`
- Ghidra read-only decompile/assembly: `0x004BAD80`, `0x004BAF40`, `0x004BAD60`, `0x004BAD70`, `0x00411510`
- Static `gamemd.exe` vtable bytes: `DSurface vtable @ 0x007E85D4`, `+0x30 -> 0x007BA5E0`, `+0x5C -> 0x004BAD80`, `+0x60 -> 0x004BAF40`, `+0x70 -> 0x004BAD60`, `+0x74 -> 0x004BAD70`, `+0x78 -> 0x00411510`
- Prior docs: `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_PREVIEW_SURFACE_VTABLE_AND_CLIPPING_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`
- Rust status: Codegraph context and `rg` over `C:/Users/enok/Documents/ra2-rust-game/src`
