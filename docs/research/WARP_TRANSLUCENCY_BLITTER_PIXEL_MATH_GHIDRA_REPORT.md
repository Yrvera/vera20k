# Warp Translucency Blitter Pixel Math - Ghidra Research Report

**Date:** 2026-05-28
**Address(es):** `TechnoClass::Draw @ 0x00706640`, `TechnoClass::Render @ 0x00706ED0`, `VXL_CacheBlit @ 0x00707480`, `Blitter_selector @ 0x00490B90`, `Blitter_selector_extended @ 0x00490E50`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** VXL warp-translucency draw mode for ordinary techno warping where `flags & 6 == 4`, including cached and uncached selector paths and the selected 50% pixel formula.
**Non-Scope:** all SHP techno body variants, every non-warp blitter mode, temporal weapon phase-state logic, building-only `0x2006` visuals beyond selector contrast, and live framebuffer capture.
**Confidence:** High for VXL selector and integer pixel math; Medium for exact final display color against Rust because no live screenshot/pixel capture was taken.
**Active in YR:** Yes for ordinary VXL technos rendered while `TechnoClass+0x270` or `+0x271` is set.

## 1. Working Notes

Target question: does native warp translucency for `flags & 6 == 4` equal plain 50% alpha, or does it require a distinct material/blitter path?

Non-goals: do not reopen teleport locomotion, chrono-miner harvester clearing, temporal weapon damage phase logic, all cloak modes, or all SHP renderers.

Evidence needed to mark COMPLETE: prove the active VXL caller flags; prove standard and extended selector branches; identify the returned blitter slots and vtable methods; verify the per-pixel formula, remap/palette/A-buffer order, z-test behavior, and whether Z is written; compare to current Rust render surfaces.

Stop conditions: stop after VXL cached and uncached paths are resolved, 25/50/75 differences are bounded only enough to identify the warp mode, Rust handoff is concrete, and deferred SHP/live-capture gaps are named.

## 2. Overview

Native VXL warp translucency is not just a generic alpha field. Ordinary warping technos reach a selector path with effective flags `0x2804`: base VXL `0x2000`, warp mode bit `0x0004`, and the normal remap bit `0x0800`.

Both uncached and cached VXL paths select a 50/50 integer blend blitter that first remaps the source palette byte through the A-buffer/intensity lookup and house-remap palette table, then blends the resulting 16-bit source color with the existing 16-bit destination color using bit shifts and a channel mask. These warp blitters read the Z buffer and skip occluded pixels, but the observed selected methods do not write Z.

## 3. Key Offsets And Globals

| Field / global | Value | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `TechnoClass+0x270` | byte | `IsWarpingOut` accessor source | prior report plus `TechnoClass::Draw @ 0x00706640` caller evidence | Yes |
| `TechnoClass+0x271` | byte | `IsBeingWarped` accessor source | prior report plus `TechnoClass::Draw @ 0x00706640` caller evidence | Yes |
| `g_BlitterFlagMask_0x3000` | `0x3000` at `0x0081DC24` | standard selector mask for bits `0x1000/0x2000` | Ghidra `read_memory(0x0081DC24, 8)` returned two DWORDs `0x3000` | Yes |
| `_DAT_0081DC28` | `0x3000` | extended selector mask, same value as above | Ghidra `read_memory(0x0081DC24, 8)` second DWORD | Yes |
| standard warp slot | `+0xA4` | selected by `Blitter_selector(0x2804)` | selector branch at `0x00490B90`, init vtable `0x007E56A8` | Yes |
| extended warp slot | `+0x144` | selected by `Blitter_selector_extended(0x2804)` | selector branch at `0x00490E50`, init vtable `0x007E53F0` | Yes |

## 4. Core Logic

### 4.1 Caller flags

Active in YR: Yes. `TechnoClass::Draw @ 0x00706640` starts ordinary VXL draw flags at `0x2000`, ORs ordinary warp with `0x2004` when `IsWarpingOut || IsBeingWarped`, and later ORs `0x800` before masking. For ordinary units this makes the relevant selector input effectively `0x2804`, unless the caller mask explicitly strips a bit.

`TechnoClass::Render @ 0x00706ED0` rechecks the warp predicates on the uncached path and ORs `param_8` with `4` or `6` before calling `Blitter_selector(param_8 & 0xFFFFFFEF)`. `VXL_CacheBlit @ 0x00707480` calls `Blitter_selector_extended(param_5 & 0xFFFFFFEF)`. The `0x10` bit is cleared for both VXL paths before selection.

### 4.2 Uncached selector branch

Active in YR: Yes for uncached VXL rendering. In `Blitter_selector @ 0x00490B90`, `uVar1 = flags & 6`. For `0x2804`, `uVar1 == 4`, `(flags & 0x4000) == 0`, `(g_BlitterFlagMask_0x3000 & flags) != 0`, `(flags & 8) == 0`, and `(flags & 0x800) != 0`, so the selector returns `this + 0xA4`.

`Blitter_init @ 0x0048EBF0` initializes slot `+0xA4` with vtable pointer `0x007E56A8`. Ghidra memory for `0x007E56A8` begins with method pointers `0x0049A4E0, 0x004950C0, 0x004951F0, 0x004951C0, 0x00495220...`; the per-scanline method used by the blitter loop is `0x004950C0`.

### 4.3 Cached selector branch

Active in YR: Yes for cached VXL pixmaps. In `Blitter_selector_extended @ 0x00490E50`, the same `0x2804` flags take the `uVar1 == 4`, mask-nonzero, no-`0x8`, `0x800` branch, returning `this + 0x144`.

`Blitter_init @ 0x0048EBF0` initializes slot `+0x144` with vtable pointer `0x007E53F0`. Ghidra memory for `0x007E53F0` begins with method pointers `0x0049AC00, 0x004986D0, 0x00498880...`; method `0x004986D0` is the RLE/cached scanline path.

### 4.4 50% pixel formula

Active in YR: Yes for the selected VXL warp paths. The standard selected method `0x004950C0` performs:

1. Clamp the intensity parameter with `scaled = ((max(param, 0) * 0x105) >> 0xB); if scaled > 0xFE { scaled = 0xFE; }`.
2. Read the current A-buffer pixel and map it through `this+8` at `scaled * 0x200 + abuf_pixel * 2`.
3. OR the mapped value with the nonzero source byte.
4. Look up the 16-bit source color through the remap/palette table at `this+4`.
5. Blend source and destination as `(src >> 1 & mask) + (dst >> 1 & mask)`, where the mask is stored at `this+0xC`.

Assembly context: `0x00495160 OR EAX,ECX`, `0x00495165 MOV AX,[ECX+EAX*2]`, `0x0049516C SHR AX,1`, `0x0049516F SHR CX,1`, `0x00495172/0x00495174 AND ... mask`, `0x00495176 ADD`, `0x00495178 MOV [EDI],AX`.

The cached selected method `0x004986D0` uses the same source remap and 50/50 blend core: `0x0049880E OR EAX,ECX`, `0x00498817 MOV AX,[ECX+EAX*2]`, `0x0049881E SHR AX,1`, `0x00498821 SHR CX,1`, `0x00498824/0x00498826 AND ... mask`, `0x00498828 ADD`, `0x0049882F MOV [EBX],AX`.

### 4.5 Z behavior

Active in YR: Yes for the selected VXL warp paths when a Z buffer is active. The standard selected method reads the Z-buffer word before sampling the source byte. At `0x00495126..0x0049512E`, it loads `word [zptr]`, compares the incoming/base Z against it, and jumps over the pixel if the base Z is not in front. The method increments/wraps Z and A-buffer pointers, but no store to the Z-buffer word appears in the selected blend body.

The cached method also performs a Z test before blending. At `0x004987D9..0x004987E7`, it reads a signed per-pixel value, subtracts it from the incoming depth term, compares to `word [zptr]`, and skips the pixel on the failing branch. Its blend body writes only the screen word and advances/wraps Z and A-buffer pointers; no Z-buffer store appears in the selected method.

Therefore the ordinary warp material is "Z-tested translucent, no Z write" for the inspected selected VXL methods.

### 4.6 25/50/75 contrast, bounded to this target

Active in YR: Conditional, depending on selector flags. The related non-RLE methods confirm the ratio meanings:

| Method | Formula | Evidence |
|---|---|---|
| `Blitter_Scanline_Blend25pct_Remap @ 0x00494080` | `src/4 + 3*dst/4` | `0x00494107..0x00494116` |
| `Blitter_Scanline_Blend50pct_Remap @ 0x004941E0` | `src/2 + dst/2` | `0x00494266..0x00494272` |
| `Blitter_Shimmer_75pct_Remap @ 0x00494330` | `3*src/4 + dst/4` | `0x004943B7..0x004943C8` |

The scoped warp path uses the 50% formula. Older shorthand that names `+0x78` as "the 50% path" is incomplete for ordinary techno warp because `TechnoClass::Draw` contributes `0x2000`, making the `0x3000` mask branch choose `+0xA4` / `+0x144` instead.

## 5. INI Keys

No INI key directly selects this blitter. The path is driven by runtime `TechnoClass+0x270/+0x271` and draw flags. Unit type art/voxel data determine whether the VXL path is used, but this report did not inspect every SHP-bodied techno fallback.

## 6. Integration Points

| Order | Function / address | Condition / flag proof | Palette / convert | Z behavior | Active for target? | Role |
|---|---|---|---|---|---|---|
| 1 | `TechnoClass::Draw @ 0x00706640` | `IsWarpingOut || IsBeingWarped`, ordinary path ORs `0x2004`, then `0x800` | none yet | none | yes | sets draw mode |
| 2a | `TechnoClass::Render @ 0x00706ED0` | uncached VXL, calls `Blitter_selector(flags & 0xFFFFFFEF)` | VXL raster output bytes become shape source | computes draw and clip setup | conditional | uncached VXL surface |
| 2b | `VXL_CacheBlit @ 0x00707480` | cached VXL, calls `Blitter_selector_extended(flags & 0xFFFFFFEF)` | cached RLE/pixmap bytes become shape source | uses extended/RLE setup | conditional | cached VXL surface |
| 3a | `Blitter_selector @ 0x00490B90` | `0x2804 -> +0xA4` | none | selector only | yes for uncached | standard blitter choice |
| 3b | `Blitter_selector_extended @ 0x00490E50` | `0x2804 -> +0x144` | none | selector only | yes for cached | extended blitter choice |
| 4a | selected method `0x004950C0` | vtable `0x007E56A8 + 4` | A-buffer/intensity, source byte OR, remap table, 16-bit mask blend | read/skip, no write observed | yes for uncached | 50% pixel blend |
| 4b | selected method `0x004986D0` | vtable `0x007E53F0 + 4` | same core, with RLE and per-pixel signed depth term | read/skip, no write observed | yes for cached | 50% pixel blend |

Asset role matrix:

| Asset / table | Loaded | Drawn | Visible in target | Content | Overlay | Evidence |
|---|---|---|---|---|---|---|
| VXL raster/cached pixmap bytes | yes | yes | yes | source techno body | no | `TechnoClass::Render`, `VXL_CacheBlit` |
| A-buffer | yes when available | sampled | affects final pixel | lighting/intensity lookup | no | `0x0049514A`, `0x00498800` equivalent table index |
| remap/palette table `this+4` | yes | sampled | affects final pixel | house/remap palette conversion | no | `0x00495162..0x00495165`, `0x00498810..0x00498817` |
| destination framebuffer word | yes | read/write | yes | existing background | no | `0x00495169..0x00495178`, `0x0049881B..0x0049882F` |

## 7. Current Rust Implementation Status

Current Rust does not expose a native-equivalent warp material. `src/app_instances/units.rs` still sets unit `alpha` to `1.0` around the stale comment that chrono teleport is only the overlay. `src/render/batch.rs` has `SpriteInstance::alpha`, `fx_flags`, and `fx_params`, and the voxel pipeline uses ordinary GPU `ALPHA_BLENDING`.

`src/render/sprite_voxel_shader.wgsl` samples palette or house ramp RGB, multiplies tint, then returns `vec4(rgb * tint, alpha)`; any alpha blend is the GPU color target blend. That is not the same as native 16-bit integer post-remap blending because native blends after the A-buffer/intensity lookup and remap palette table, with `(src >> 1 & mask) + (dst >> 1 & mask)` on the framebuffer word.

Plain `alpha = 0.5` is acceptable only as a temporary visual approximation to remove the fully opaque teleport bug. It is not sufficient as the final parity representation because it loses the draw-mode bits, native 16-bit rounding/channel mask, A-buffer remap order, and exact Z-tested/no-Z-write semantics.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::Draw` ordinary warp flags | verified | `0x00706640`, prior slot report | none for VXL path |
| `TechnoClass::Render` uncached selector call | verified | `0x00706ED0` decompile | live pixel capture optional |
| `VXL_CacheBlit` extended selector call | verified | `0x00707480` decompile | live pixel capture optional |
| `Blitter_selector(0x2804)` branch | verified | `0x00490B90`, `0x0081DC24=0x3000` | none |
| `Blitter_selector_extended(0x2804)` branch | verified | `0x00490E50`, `0x0081DC28=0x3000` | none |
| standard selected method `0x004950C0` | verified | vtable `0x007E56A8`, assembly contexts above | no live framebuffer capture |
| cached selected method `0x004986D0` | verified | vtable `0x007E53F0`, assembly contexts above | exact source of signed depth byte named only as per-pixel cached data |
| SHP-bodied techno path | deferred | scope boundary | follow-up only if stock SHP techno body uses this warp visual |
| exact final RGB vs Rust shader on sRGB surface | deferred | needs runtime capture | pixel fixture or debugger capture |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-1 - What flags enter the VXL selector for ordinary warping technos? -> effective `0x2804` in the normal case: base `0x2000`, warp `0x4`, remap `0x800`.` (evidence: `0x00706640`, `0x00706ED0`, `0x00707480`)
- `[RESOLVED] OQ-2 - Does `flags & 6 == 4` alone decide the exact blitter? -> no; `0x3000`, `0x800`, `0x8`, and `0x4000` alter the selected slot.` (evidence: `0x00490B90`, `0x00490E50`)
- `[RESOLVED] OQ-3 - Which standard slot is selected for ordinary warp? -> `+0xA4`, vtable `0x007E56A8`, method `0x004950C0`.` (evidence: `0x00490B90`, `Blitter_init @ 0x0048EBF0`, `read_memory(0x007E56A8)`)
- `[RESOLVED] OQ-4 - Which extended slot is selected for ordinary cached warp? -> `+0x144`, vtable `0x007E53F0`, method `0x004986D0`.` (evidence: `0x00490E50`, `Blitter_init @ 0x0048EBF0`, `read_memory(0x007E53F0)`)
- `[RESOLVED] OQ-5 - What is the 50% pixel formula? -> post-remap 16-bit source and destination are shifted right one bit, masked, added, and written to the framebuffer.` (evidence: `0x0049516C..0x00495178`, `0x0049881E..0x0049882F`)
- `[RESOLVED] OQ-6 - Does source color 0 draw? -> no; selected methods test the source byte and skip zero pixels.` (evidence: `0x00495134..0x0049513C`, `0x0049878C..0x00498795`)
- `[RESOLVED] OQ-7 - Does selected warp write Z? -> no Z write observed in the selected blend bodies; they read/compare Z and write only the screen word.` (evidence: `0x00495126..0x0049512E`, `0x004987D9..0x004987E7`, blend stores at `0x00495178`, `0x0049882F`)
- `[RESOLVED] OQ-8 - Is plain `alpha=0.5` exact? -> no; it can approximate visibility but not native palette/A-buffer/16-bit/channel-mask/Z semantics.` (evidence: selected blitter assembly plus `src/render/sprite_voxel_shader.wgsl`)
- `[DEFERRED] OQ-9 - Do any SHP-bodied technos use an adjacent warp body path?` (category: out-of-scope; reason: this slot was VXL-first and Rust-facing unit surface is VXL; next-step-if-pursued: trace `TechnoClass_DrawSHP @ 0x00705E00` with the same flags)
- `[DEFERRED] OQ-10 - What exact RGB values appear on a live surface for one test pixel?` (category: needs-runtime-debugger; reason: static formula is complete but no live framebuffer sample was captured; next-step-if-pursued: set up one known source/dest/A-buffer pixel and compare)

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary VXL warp selects a native 50% material through `0x2804 -> standard +0xA4 / extended +0x144`, not a generic opaque draw. | `TechnoClass::Draw @ 0x00706640`, `Blitter_selector @ 0x00490B90`, `Blitter_selector_extended @ 0x00490E50` | missing: unit render uses `alpha=1.0`; no draw-mode marker | `src/app_instances/units.rs`, `src/render/batch.rs`, `src/render/sprite_voxel_shader.wgsl` | set a warp-translucent material/flag when `being_warped_ticks > 0`; interim alpha may be `0.5`, but preserve native draw mode separately | non-harvester teleporter is translucent during cooldown and returns opaque after countdown | `unit_render_non_harvester_teleport_uses_warp_material_0x2804`; do not hardcode only UI alpha |
| Native 50% is post-remap integer blend: `(src16 >> 1 & mask) + (dst16 >> 1 & mask)`, with source color produced by A-buffer/intensity and remap table. | `0x00495160..0x00495178`, `0x0049880E..0x0049882F` | mismatch if using GPU linear/sRGB alpha as final parity | renderer material/blitter emulation layer | implement/test a native 16-bit blend helper or shader path that matches channel-mask rounding after palette/remap | known source index, house remap row, A-buffer value, and destination RGB565 word produce exact native output | `warp_blitter_50pct_matches_native_16bit_post_remap_blend`; do not blend source palette RGB before remap/A-buffer |
| Selected warp methods Z-test against the native Z buffer and do not write Z in the inspected bodies. | `0x00495126..0x0049512E`, `0x004987D9..0x004987E7`; absence of Z store in selected blend body | partially similar: Rust voxel pipeline has depth compare and depth write off, but exact depth predicate/source is unchecked | `src/render/batch.rs`, unit depth computation | keep translucent unit pixels depth-tested but no depth-write; verify compare direction and terrain/bridge depth source against native | warped unit behind terrain/cliff does not blend through foreground, and does not occlude later translucent pixels by writing depth | `warp_translucent_unit_depth_tests_without_writing_depth`; do not make warp a draw-order-only overlay |

## 11. Negative Facts / Do Not Do

- Do not treat `flags & 6 == 4` as enough to choose `+0x78`; ordinary techno warp includes `0x2000`, so the `0x3000` mask branch selects `+0xA4` or `+0x144`. Evidence: `0x00490B90`, `0x00490E50`, `0x0081DC24=0x3000`.
- Do not use plain `alpha=0.5` as the final parity implementation. Evidence: native selected methods blend 16-bit source/destination words after A-buffer and remap lookup at `0x00495160..0x00495178` and `0x0049880E..0x0049882F`.
- Do not write Z for the selected ordinary warp material unless a later live capture proves a different vtable path. Evidence: selected standard and cached methods write screen words but no Z-buffer word in their blend bodies.
- Do not blend source index 0. Evidence: both selected methods skip zero source pixels before remap/blend (`0x00495134..0x0049513C`, `0x0049878C..0x00498795`).
- Do not conflate the 25/50/75 modes: `0x00494080` is `src/4 + 3*dst/4`, `0x004941E0` and selected warp methods are `src/2 + dst/2`, and `0x00494330` is `3*src/4 + dst/4`.

## 12. Remaining Uncertainty

- Exact live framebuffer word for a concrete unit/background/A-buffer sample was not captured; static formula is complete enough for implementation handoff, but pixel fixture values need a controlled capture or helper test.
- SHP-bodied techno-body warp path is deferred. This does not block the current VXL unit handoff, but should be checked before claiming all techno bodies.
- The cached path's signed per-pixel depth term source was observed in the selected method but not fully named structurally; it appears to be cached/RLE auxiliary pixel data.

## 13. Stale Docs / Follow-up Docs

- `docs/research/CLOAKING_VISUAL_PIPELINE.md`: the table row saying `flags & 6 == 0x04` with `0x800` selects `+0x78` is incomplete for ordinary techno warp. Replacement wording: "`flags & 6 == 0x04` selects different 50% families depending on the higher flags. Ordinary `TechnoClass::Draw` VXL warp normally enters with `0x2804`; because `0x3000 & flags` is nonzero and `0x800` is set, `Blitter_selector` returns `+0xA4`, while `Blitter_selector_extended` returns `+0x144`. Both selected methods still perform a 50/50 post-remap blend, but they also Z-test and do not write Z in the inspected bodies."
- `docs/research/VXL_RASTERIZER_DISPATCH_GHIDRA_REPORT.md`: the row describing `flags & 6 == 4` as "standard opaque + remap" is wrong or overgeneralized for warp. Replacement wording: "`flags & 6 == 4` is not globally opaque; with ordinary warp flags `0x2804`, the selector reaches the 50% translucent Z-tested remap family (`+0xA4` standard, `+0x144` extended)."

## Sources

- Ghidra decompile/read-only: `TechnoClass::Draw @ 0x00706640`.
- Ghidra decompile/read-only: `TechnoClass::Render @ 0x00706ED0`.
- Ghidra decompile/read-only: `VXL_CacheBlit @ 0x00707480`.
- Ghidra decompile/read-only: `Blitter_selector @ 0x00490B90`.
- Ghidra decompile/read-only: `Blitter_selector_extended @ 0x00490E50`.
- Ghidra decompile/read-only: `Blitter_init @ 0x0048EBF0`.
- Ghidra `read_memory`: `0x0081DC24`, `0x007E56A8`, `0x007E53F0`, `0x007E5780`, `0x007E5440`.
- Ghidra assembly context/read-only: `0x0049512C`, `0x00495160`, `0x0049516C`, `0x004987E5`, `0x0049880E`, `0x0049881B`, `0x004940FD`, `0x00494104`, `0x0049425C`, `0x00494266`, `0x004943AD`, `0x004943B4`.
- Prior report: `docs/research/TECHNOCLASS_DRAW_BEINGWARPED_TRANSLUCENCY_GHIDRA_REPORT.md`.
- Rust scan: `src/app_instances/units.rs`, `src/render/batch.rs`, `src/render/sprite_voxel_shader.wgsl`.
