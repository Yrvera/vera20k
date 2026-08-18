# AlphaBlendRect 0xAF Dark Strip Pixel Math - Ghidra Report

## Summary

`AlphaBlendRect @ 0x00621B80` is the exact helper used by the in-game sidebar dark rectangles behind queue-count, Ready/status, and Hold/status text. The standard-YR sidebar callers pass `blend_color = 0` and `alpha = 0xAF`.

The helper is not a float alpha blend and not a palette conversion. It locks the destination surface, treats it as 16-bit packed pixels, applies three runtime DirectDraw channel masks, and writes:

```text
new_component = ((src_component * alpha) + (dst_component * (0xFF - alpha))) >> 8
pixel = (new0 & mask0) | (new1 & mask1) | (new2 & mask2)
```

For sidebar dark strips, `src_component = 0`, `alpha = 0xAF`, and `0xFF - alpha = 0x50`, so each masked packed component becomes:

```text
new_component = ((dst_component & mask) * 0x50) >> 8, then re-masked
```

That is a packed-16-bit integer operation with truncation by `>> 8`, not `dst * (80 / 255)` and not GPU `src_alpha/one_minus_src_alpha`.

## Target and Non-Scope

Target: decode the exact native `AlphaBlendRect(0, 0xAF)` path used by sidebar dark text strips, including formula, rounding, channel masks/order, surface assumptions, clipping behavior, and which sidebar text backgrounds use it.

Non-scope:

- Main sidebar layout, retained-surface dirty cadence, SHP composition order, and palette routing already covered by recent sidebar reports.
- Tooltip background alpha callers, disabled shell controls, owner-draw shell widgets, and non-sidebar alpha users except as evidence that `0x00621B80` is a shared helper.
- Rust implementation patches.

## Verified Binary Findings

1. **The helper address is `AlphaBlendRect @ 0x00621B80`.**  
   Evidence: `StripClass::Draw @ 0x006A9540` decompile emits four `AlphaBlendRect(0,0xaf)` calls; xrefs to `AlphaBlendRect` resolve to calls at `0x006A9D4C`, `0x006A9E0C`, `0x006A9F59`, and `0x006A9FEE`. Assembly context at those call sites shows `CALL 0x00621B80`.

2. **Calling convention and arguments for the sidebar calls are rect/surface in registers plus color/alpha on stack.**  
   Evidence: call-site assembly at `0x006A9D13..0x006A9D4C` pushes `0xAF`, pushes `0`, loads `ECX` with the computed text rect, loads `EDX` from `g_SidebarSurface` (`0x00887300`), then calls `0x00621B80`. `0x00621D26` returns with `RET 0x8`, consuming the two stack arguments.

3. **The helper locks the destination surface and assumes 16-bit packed pixels.**  
   Evidence: `0x00621B8D..0x00621B9F` calls surface vtable `+0x5C` with `(0,0)` and skips the whole helper if the returned pointer is zero. `0x00621BB2` calls vtable `+0x74` for pitch; `0x00621BBB..0x00621BC6` computes `pitch_words = pitch / 2`; `0x00621C0A` derives the first pixel pointer with `base + (row_offset + x) * 2`; `0x00621C2F` reads `word ptr [EAX]`; `0x00621CCA` writes `word ptr [EAX - 2]`.

4. **The alpha math is integer packed-channel math with `0xFF - alpha` and `>> 8`.**  
   Evidence: `0x00621BFF..0x00621C0D` masks `alpha` to 8 bits and computes `0xFF - alpha`; `0x00621C4E..0x00621C6D`, `0x00621C73..0x00621CA6`, and `0x00621CAA..0x00621CC4` perform, for each channel mask, `(src_masked * alpha + dst_masked * inv_alpha) >> 8`, then mask/OR the result into the output pixel. There is no rounding add and no divide by 255.

5. **For black `0xAF`, native retains `0x50/0x100` of each packed destination channel after truncation.**  
   Evidence: sidebar callers pass source color `0` and alpha `0xAF`; `0xFF - 0xAF = 0x50`; source terms are zero because `blend_color & mask == 0`; the final channel write is `((dst & mask) * 0x50) >> 8`, re-masked.

6. **Channel order is mask-driven, not hardcoded RGB byte order inside the helper.**  
   Evidence: `AlphaBlendRect` reads three runtime masks: low 16 bits of `DAT_00AC48B8`, high 16 bits of `DAT_00AC48B8` via `word ptr [0x00AC48BA]`, and low 16 bits of `DAT_00AC48BC`. `FUN_0060F9A0 @ 0x0060F9A0` initializes those masks from DirectDraw loss/shift helper calls when `DAT_00AC48D4 == 0`. For the black sidebar call, semantic RGB order does not affect the source term, but the destination channel quantization is exactly these masks.

7. **The helper has no explicit rectangle clipping against surface bounds.**  
   Evidence: loops use `rect.y < rect.y + rect.h` and `rect.x < rect.x + rect.w` (`0x00621BC8`, `0x00621BE8`, `0x00621CDA`, `0x00621D07`) and pointer arithmetic from `x`, `y`, and pitch. There is no read of surface width/height and no clamp. Empty or non-positive width/height skip through the signed loop tests; out-of-bounds non-empty rects are not protected by this helper.

8. **Unlock runs only after a successful lock.**  
   Evidence: lock result zero branches to function exit at `0x00621BA5`; the vtable `+0x60` unlock call is reached only at `0x00621D17..0x00621D1C` after drawing.

9. **Sidebar dark-strip callers are queue-count, Ready/status, and Hold/status split paths; credits text does not use this helper.**  
   Evidence: `StripClass::Draw @ 0x006A9540` call sites:
   - `0x006A9D0C..0x006A9D7B`: queue-count text, flags `0x242`.
   - `0x006A9DCC..0x006A9E36`: Ready/status text, flags `0x142`.
   - `0x006A9F19..0x006A9F7D`: Hold/status when queue text is present, flags `0x42`.
   - `0x006A9FAE..0x006AA014`: Hold/status without queue text, flags `0x142`.
   `CreditsClass::Draw @ 0x004A2370` calls `DrawCreditsSHPBackground @ 0x006D0E60` and `DrawText`, but its callee list contains no `AlphaBlendRect`.

10. **`AlphaBlendRect` is a shared helper, but shared users do not change sidebar semantics.**  
    Evidence: `get_function_xrefs("AlphaBlendRect")` shows shell/owner-draw callers such as disabled buttons, checkboxes, combo boxes, and trackbars, plus the four `StripClass::Draw` calls. This report uses those only to confirm `0x00621B80` is the common helper; it does not generalize their caller-specific alpha constants to sidebar dark strips.

## Active in Standard YR?

Yes, conditional on the text path existing for the affected cameo slot. The active standard-YR in-game sidebar path reaches `StripClass::Draw @ 0x006A9540`; when queue-count, Ready/status, or Hold/status text is drawn, the corresponding `ComputeTextRect -> AlphaBlendRect(0,0xAF) -> DrawText` sequence runs on `g_SidebarSurface`.

Credits/observer elapsed-time text is active in standard YR but does not use `AlphaBlendRect`; it uses `CreditsClass::Draw @ 0x004A2370`, `DrawCreditsSHPBackground @ 0x006D0E60`, and `DrawText`.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Sidebar dark strips use packed 16-bit integer alpha math: `((src&mask)*alpha + (dst&mask)*(255-alpha)) >> 8`, re-masked. For black `0xAF`, keep `0x50/0x100` of masked destination. | `0x00621B80`, especially `0x00621BFF..0x00621CC4`; call sites `0x006A9D4C`, `0x006A9E0C`, `0x006A9F59`, `0x006A9FEE` | Rust uses a 1x1 RGBA black texture with alpha `175` (`src/render/bit_font.rs:32`, `src/render/bit_font.rs:526`) and normal GPU alpha blending; pixel equivalence is unchecked and likely not exact. | `src/render/bit_font.rs`, `src/app_sidebar_build.rs`, render batch/dark-strip path | Either reproduce the native packed-surface blend in the retained sidebar surface, or prove a GPU path matches the packed 16-bit result for the target surface format. | `test_sidebar_dark_strip_black_af_uses_packed_16bit_shift8_math`: feed representative RGB565/RGB555 pixels and assert exact native output, including low-bit truncation. | Do not use `alpha / 255.0` compositing as the parity model. Do not round. Do not blend in linear/sRGB float space and assume equality. |
| Queue-count, Ready/status, and Hold/status use the same helper and same `0xAF` alpha. | `StripClass::Draw @ 0x006A9540`; assembly ranges above | Rust has dark strips for Ready and queue count in `src/app_sidebar_build.rs:518..566`; Hold/status split coverage is unchecked in this slot. | `src/app_sidebar_build.rs`, `src/render/sidebar_text.rs` | Share one native dark-strip blend path for all three text-background cases. | `test_sidebar_ready_queue_hold_dark_strips_share_alpha_af_helper`: render all text states over the same cameo pixel and compare same darkening equation. | Do not give queue and Ready separate opacity constants or use a color derived from sidebar text color. |
| Credits text does not get an `AlphaBlendRect(0,0xAF)` background. | `CreditsClass::Draw @ 0x004A2370`; callees are `DrawCreditsSHPBackground`, `DrawText`, string/sound helpers, not `AlphaBlendRect` | Rust credits path is egui/system text with hardcoded color (`src/app_sidebar_text.rs:1..57`), but the dark-strip question specifically should not add a `0xAF` rect behind credits. | `src/app_sidebar_text.rs`, future sidebar-surface text renderer | Keep credits on its verified background/text path; do not route it through Ready/queue dark-strip overlay logic. | `test_sidebar_credits_has_no_alpha_af_dark_strip`: credits update draws background SHP/text but no black `0xAF` rect. | Do not infer “credits background” from “dark strips”; the native credits path is separate. |
| `AlphaBlendRect` performs no explicit bounds clipping; callers must pass valid rects. | `0x00621BC8..0x00621D11` loops; no width/height clamp reads | Current Rust quads are naturally clipped by GPU target/scissor; this may hide invalid rects differently. | retained sidebar surface / dark rect rasterizer when implemented | Native-equivalent raster path should clip at the caller/compositor level only where native does; helper-level clamping is not part of the helper. | `test_alpha_blend_rect_empty_rect_noop_and_valid_rect_exact_pixels`; out-of-bounds behavior should be treated as caller-invalid unless a live caller proves it. | Do not add helper-level surface-bound clipping and then call that “native” for arbitrary rects. |

## Negative Facts / Do Not Do

- Do not model `0xAF` as “69% black over the strip” using normalized GPU alpha. Native uses `inv = 0xFF - 0xAF = 0x50`, then shifts by 8.
- Do not divide by 255 or add a rounding bias. The binary uses `>> 8`.
- Do not unpack to 8-bit RGB, blend, then repack unless tests prove identical to masked packed math for every relevant channel value.
- Do not route credits through `AlphaBlendRect(0,0xAF)`. Credits text has its own background/text path.
- Do not treat channel order as fixed RGBA bytes in this helper. It is three runtime DirectDraw masks over a 16-bit pixel.
- Do not rely on helper-level clipping; `AlphaBlendRect` itself does not clamp to surface bounds.

## Remaining Uncertainty

- Exact runtime mask values for the configured retail display mode were not captured with a live debugger. The helper mechanism is verified; a runtime capture would name whether the active mode is RGB565, RGB555, or another 16-bit mask arrangement.
- GPU framebuffer color-space details in current Rust were not exhaustively audited. The current `DARKEN_ALPHA` texture path is enough to flag likely drift, but a full Rust render-output comparison belongs to implementation verification.
- Non-sidebar `AlphaBlendRect` users were not fully investigated. Their existence confirms shared helper semantics, but their caller-specific constants and rect setup are outside this report.

## Stale-Doc Replacement Wording

Replace wording that says:

> `AlphaBlendRect(0, 0xAF)` blends black at 175/255, roughly 69%.

With:

> `AlphaBlendRect(0, 0xAF)` uses packed 16-bit masked integer math. For black source color, each destination channel becomes `((dst & mask) * (0xFF - 0xAF)) >> 8`, re-masked; because `0xFF - 0xAF = 0x50`, the native operation keeps `0x50/0x100` of the packed destination component with truncation. It is not normalized `/255` alpha blending.

Replace wording that says:

> Disabled `0x80` is 50%.

With:

> For black source color and alpha `0x80`, native keeps `(0xFF - 0x80) / 0x100 = 0x7F / 0x100` of each packed destination component after truncation; it is close to, but not exactly, standard 50% alpha blending.

Replace any sidebar wording that groups credits with Ready/queue dark strips with:

> Ready/status, queue-count, and Hold/status dark text rectangles use `StripClass::Draw -> ComputeTextRect -> AlphaBlendRect(0,0xAF) -> DrawText`. Credits/observer elapsed-time text uses `CreditsClass::Draw -> DrawCreditsSHPBackground -> DrawText` and does not call `AlphaBlendRect`.

## Status

COMPLETE for the bounded target: exact helper formula, rounding/truncation, mask route, 16-bit surface assumption, no helper-level clipping, and active sidebar caller set.

Sources:

- Ghidra decompile/assembly: `AlphaBlendRect @ 0x00621B80`
- Ghidra decompile/assembly: `StripClass::Draw @ 0x006A9540`, call sites `0x006A9D4C`, `0x006A9E0C`, `0x006A9F59`, `0x006A9FEE`
- Ghidra decompile: `CreditsClass::Draw @ 0x004A2370`
- Ghidra decompile: `DrawCreditsSHPBackground @ 0x006D0E60`
- Ghidra decompile: `FUN_0060F9A0 @ 0x0060F9A0`
- Prior docs checked: `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`, `SIDEBAR_LAYER_PALETTE_CONVERTCLASS_PIXEL_COLORS_GHIDRA_REPORT.md`, `SIDEBAR_POWER_CREDITS_READY_TEXT_LAYOUT_GHIDRA_REPORT.md`, `SOVIET_SIDEBAR_TEXT_COLOR_READY_STATUS_FOLLOWUP_GHIDRA_REPORT.md`
