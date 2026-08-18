# SHP Techno Warp Draw Path -- Ghidra Research Report

**Address(es):** `TechnoClass_DrawSHP @ 0x00705E00`, `CC_Draw_Shape @ 0x004AED70`, `Blitter_selector @ 0x00490B90`, `Blitter_selector_extended @ 0x00490E50`, `FootClass::GetVisualState @ 0x004DA4E0`, `TechnoClass_GetVisualState @ 0x00703860`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** SHP-body draw flag construction under TechnoClass warp predicates and the SHP blitter selector branch reached by those flags.
**Non-Scope:** VXL warp pixel math already covered by `WARP_TRANSLUCENCY_BLITTER_PIXEL_MATH_GHIDRA_REPORT.md`; full InfantryClass frame sequencing; complete UnitClass `Voxel=no` body function boundary recovery.
**Confidence:** High for `TechnoClass_DrawSHP` flag construction and selector deltas; Medium for stock SHP-unit call-chain liveness from UnitClass because the `0x0073B470` function boundary was not present in Ghidra.
**Active in YR:** Yes, conditional on a SHP-bodied techno being in `IsWarpingOut` or `IsBeingWarped`/warp visual state. Stock YR has SHP infantry with teleport locomotion (`rulesmd.ini:[CLEG] Teleporter=yes`, Teleport locomotor GUID) and SHP vehicles (`artmd.ini:[DLPH]`, `[DRON]`, `[SQD] Voxel=no`).

## Working Notes

- **Target question:** When TechnoClass warp flags reach an SHP-bodied techno/SHP branch, does stock gamemd use the same 50% mode as VXL, a different SHP blitter selector path, or no active path?
- **Non-goals:** Do not re-prove VXL `0x2804` pixel math; do not decode every SHP vehicle/infantry frame sequence; do not create Ghidra functions or labels.
- **Evidence needed to mark COMPLETE:** Decompile plus disassembly range for `TechnoClass_DrawSHP`, decompile of `CC_Draw_Shape`, decompile of both selectors, a stock-YR SHP teleport/SHP unit data proof, and Rust-facing handoff.
- **Stop conditions:** Stop after SHP warp flag bits, selector offsets, Z-test/write visibility, stock liveness conditions, Rust deltas, stale-doc replacements, and remaining uncertainty are recorded.

## 1. Overview

`TechnoClass_DrawSHP` has its own SHP flag builder. It does not simply inherit the VXL `0x2804` value, but it does set the same low `0x04` 50% blend bit when the object reports warp-out/being-warped through the same virtual predicates. After the mandatory SHP `0x800` remap and final `| 0x600` centering/window flags, the SHP selector path is either the same mask-family as VXL (`+0xA4` / `+0x144`) when `0x2000` is present, or the older no-mask SHP 50% family (`+0x78` / `+0x130`) when `0x2000` is absent.

## 2. Core Findings

1. Active in YR: Yes. `TechnoClass_DrawSHP @ 0x00705E00` calls visual-state vtable `+0x68`; if the state is `2` or `3`, it sets low flags `0x04`; state `1` sets `0x02`; state `5` skips drawing. Evidence: decompile `0x00705E00`; disassembly range `0x00705E00..0x0070641F` readable.

2. Active in YR: Yes, conditional on the warp predicates. After the visual-state switch, `TechnoClass_DrawSHP` calls vtable `+0x1D4` and `+0x1D8`; if either is true, ordinary non-building SHP technos OR `0x04` into the low flags. A building/type exception can set `0x06` instead when the object is RTTI `6` and its type byte at `+0x16B1` is set. Evidence: decompile `0x00705E00`.

3. Active in YR: Yes. `TechnoClass_DrawSHP` always builds `uVar9 = flags | 0x800`, optionally ORs `0x2000` when its `param_5 != -1`, optionally ORs `0x4000` when the mirror/flip argument is set, then passes `uVar9 | 0x600` to `CC_Draw_Shape`. The `0x600` contributes centering/window bits and does not change `flags & 6`. Evidence: decompile `0x00705E00`; `CC_Draw_Shape @ 0x004AED70`.

4. Active in YR: Yes. For ordinary SHP warp with `0x2000` present, effective flags are `0x2E04` before any mirror/ally-cloak modification. `Blitter_selector(0x2E04)` takes `flags & 6 == 4`, `0x3000 & flags != 0`, `flags & 8 == 0`, `0x800 != 0`, and returns `this + 0xA4`; `Blitter_selector_extended(0x2E04)` returns `this + 0x144`. This is the same selector family as VXL `0x2804`, not because the whole flag value is identical, but because the tested bits match. Evidence: decompiles `0x00490B90`, `0x00490E50`; disassembly ranges `0x00490B90..0x00490DF7`, `0x00490E50..0x004910AF`.

5. Active in YR: Yes, conditional on callers that do not set `0x2000`. For SHP warp without `0x2000`, effective flags are `0x0E04`; `0x3000 & flags == 0`, so the standard selector returns `this + 0x78`, and the extended/RLE selector returns `this + 0x130`. The `+0x130` RLE leaf decompiles as a 50/50 post-remap blend that reads A-buffer/intensity and destination pixels and does not read or write `g_ZBuffer`. Evidence: selector decompiles; `Blitter_ZWriteOnly_RLE_Remap_NoZWrite @ 0x00497CF0` decompile.

6. Active in YR: Yes. `CC_Draw_Shape` chooses standard vs extended SHP blitter by checking the SHP frame compression flag, not by object class. If the frame flag check returns false, it calls `Blitter_selector`; otherwise it calls `Blitter_selector_extended`. Evidence: `CC_Draw_Shape @ 0x004AED70` decompile.

7. Active in YR: Yes. The SHP branch is not a dead stock mechanism. Stock `rulesmd.ini:[CLEG]` is an infantry type with `Teleporter=yes` and Teleport locomotor GUID `{4A582747-9839-11d1-B709-00A024DDAFD1}`, and infantry are SHP-bodied. Stock SHP vehicles also exist (`artmd.ini:[DLPH]`, `[DRON]`, `[SQD] Voxel=no`). Evidence: INI lines read from `rulesmd.ini` and `artmd.ini`; `FootClass::GetVisualState @ 0x004DA4E0` delegates locomotor visual state before `TechnoClass_GetVisualState`.

## 3. Visual Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | SHP body owner draw function | SHP-bodied techno selected by caller; exact UnitClass `Voxel=no` function boundary deferred | unit/infantry/building body SHP current frame | caller-provided draw point | caller/house remap | Yes, conditional | body source |
| 2 | `TechnoClass_DrawSHP @ 0x00705E00` | visual state plus `+0x1D4/+0x1D8` warp predicates OR low `0x04` | same SHP frame | adjusts screen Y by `Tactical__AdjustForZ` where applicable | `vtable+0x464` lighting scale | Yes | techno SHP wrapper |
| 3 | `CC_Draw_Shape @ 0x004AED70` | receives `flags | 0x600`, `0x800`, optional `0x2000` | SHP frame data | clips against destination rect | intensity/remap tables | Yes | shape dispatch |
| 4a | `Blitter_selector @ 0x00490B90` | non-extended frame | scanline bytes | clipped row | remap/intensity | Yes | standard SHP blitter selection |
| 4b | `Blitter_selector_extended @ 0x00490E50` | extended/RLE frame | RLE bytes | clipped row | remap/intensity | Yes | extended SHP blitter selection |

## 4. Current Rust Implementation Status

Rust has a generic `SpriteInstance` with `alpha` and `fx_flags`, and `sprite_voxel_shader.wgsl` has a phase-1 `warp` alpha stub. The current unit render scan found VXL/unit surfaces in `src/app_instances/units.rs`, merge/draw surfaces in `src/app_render/merge_passes.rs`, `src/render/batch.rs`, and `src/render/sprite_voxel_shader.wgsl`. The SHP-paged unit path pushes `SpriteInstance { alpha: 1.0, ... }` and there is no native SHP warp material selector that distinguishes `0x2E04` from `0x0E04`.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass_DrawSHP` warp flag branch | verified | decompile `0x00705E00`, disassembly `0x00705E00..0x0070641F` | none |
| `CC_Draw_Shape` SHP selector split | verified | decompile `0x004AED70` | exact frame flag distribution per asset not enumerated |
| `Blitter_selector` for `0x2E04` / `0x0E04` | verified | decompile `0x00490B90` | leaf method body for `+0xA4` relies on prior VXL report/disassembly because Ghidra has no function at `0x004950C0` |
| `Blitter_selector_extended` for `0x2E04` / `0x0E04` | verified | decompile `0x00490E50` | none for selector |
| `+0x130` no-mask RLE 50% leaf | verified | decompile `0x00497CF0` | standard no-mask `+0x78` leaf not separately decompiled this pass |
| Stock SHP teleport example | verified | `rulesmd.ini:[CLEG]`, `artmd.ini:[CLEG]`, `FootClass::GetVisualState @ 0x004DA4E0` | exact InfantryClass draw caller not re-decompiled |
| UnitClass `Voxel=no` function boundary at `0x0073B470` | touched-not-exhausted | prior doc vtable proof; disassembly range readable | function boundary missing; do not overclaim exact call order |

## 6. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- Does SHP DrawSHP have warp-specific flag construction? -> Yes, vtable +0x1D4/+0x1D8 OR low `0x04`/`0x06`.` (evidence: `0x00705E00`; Active in YR: Yes)
- `[RESOLVED] OQ-2 -- Is the SHP warp low bit the same 50% family as VXL? -> Yes for ordinary non-building SHP technos: low `flags & 6 == 4`.` (evidence: `0x00705E00`, `0x00490B90`; Active in YR: Yes)
- `[RESOLVED] OQ-3 -- Does SHP always equal VXL `0x2804`? -> No, SHP adds `0x600` and may be `0x0E04` or `0x2E04`; selector equality to VXL happens only when `0x2000` is present.` (evidence: `0x00705E00`, `0x00490B90`; Active in YR: Conditional)
- `[RESOLVED] OQ-4 -- Does SHP warp have a stock liveness path? -> Yes, CLEG is stock SHP infantry with Teleport locomotor and `Teleporter=yes`; DLPH/DRON/SQD prove stock SHP vehicle body art exists.` (evidence: `rulesmd.ini`, `artmd.ini`; Active in YR: Yes)
- `[RESOLVED] OQ-5 -- Does `CC_Draw_Shape` use one selector for all SHP frames? -> No, frame compression flag chooses standard vs extended selector.` (evidence: `0x004AED70`; Active in YR: Yes)
- `[RESOLVED] OQ-6 -- Does the no-mask extended 50% path write Z? -> No observed Z-buffer read/write in `0x00497CF0`; it blends source and destination after remap/intensity and walks only A-buffer state.` (evidence: `0x00497CF0`; Active in YR: Conditional)
- `[DEFERRED] OQ-7 -- Exact UnitClass `Voxel=no` draw body function body at `0x0073B470`.` (category: bounded-cost-too-high; reason: Ghidra has no function boundary and mutation is forbidden; next-step-if-pursued: read-only raw assembly trace from vtable `+0x554` to `TechnoClass_DrawSHP` or VXL/SHP branch)
- `[DEFERRED] OQ-8 -- Exact standard no-mask `+0x78` leaf body.` (category: bounded-cost-too-high; reason: `+0x130` RLE leaf proves no-mask extended behavior; standard leaf address from prior docs was not decompiled here; next-step-if-pursued: focused blitter leaf table audit)
- `[DEFERRED] OQ-9 -- Exact final framebuffer comparison for CLEG warp frame over terrain/building.` (category: needs-runtime-debugger; reason: requires runtime capture or instrumented software blit; next-step-if-pursued: one-pixel fixture comparing native 16-bit output)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| SHP-bodied techno warp sets a native SHP material from low `0x04`, not generic opacity. | `TechnoClass_DrawSHP @ 0x00705E00`; Active in YR: Yes | missing/unchecked for SHP paged units; current scan shows `alpha: 1.0` for SHP page pushes | `src/app_instances/units.rs`, `src/render/batch.rs`, `src/render/sprite_voxel_shader.wgsl`, SHP unit render path | Preserve a render-mode equivalent to SHP warp flags, including whether `0x2000` is present, instead of only an untyped alpha. | Chrono Legionnaire during teleport cooldown draws through SHP warp material until the warp predicate clears. | Do not apply only VXL-unit warp handling. Proposed test: `shp_techno_warp_sets_native_shp_50pct_material`. |
| Selector family differs by `0x2000`: `0x2E04 -> +0xA4/+0x144`; `0x0E04 -> +0x78/+0x130`. | `0x00490B90`, `0x00490E50`; Active in YR: Yes/Conditional | missing: no selector-family field in `SpriteInstance` | render material batching / shader flags | Carry enough material metadata to distinguish mask-family 50% from no-mask 50% even if first implementation shares shader math. | Building/SHP route with `param_5 != -1` and SHP route without it produce different material ids in trace fixtures. | Do not treat `flags & 6 == 4` as a complete material key. Proposed test: `shp_warp_material_key_preserves_0x2000_selector_family`. |
| No-mask extended `+0x130` 50% path blends post-remap/intensity and does not write Z in the decompiled leaf. | `0x00497CF0`; Active in YR: Conditional | current GPU alpha blend is not proven equivalent to native 16-bit blend | `src/render/sprite_voxel_shader.wgsl`, SHP batch pipeline | Final parity needs native indexed/remap 50% blend semantics for SHP warp, not ordinary premultiplied alpha. | A CLEG warp pixel over a known 16-bit destination matches `(src16 >> 1 & mask) + (dst16 >> 1 & mask)` after remap/intensity for the selected branch. | Do not call plain `alpha=0.5` final parity. Proposed test: `shp_warp_50pct_blends_after_remap_not_rgba_alpha`. |

## 8. Negative Facts / Do Not Do

- Do not say SHP warp is absent. Active SHP warp branch exists in `TechnoClass_DrawSHP`, and stock CLEG gives a SHP teleport-locomotor use case.
- Do not collapse SHP warp into the exact literal VXL flag `0x2804`; SHP final flags include `0x600` and may lack `0x2000`.
- Do not key SHP material only on `flags & 6 == 4`; the `0x3000` mask branch changes selector offsets.
- Do not generalize VXL Z-test/no-Z-write findings to every SHP warp branch. The no-mask extended `+0x130` leaf has no observed Z-buffer access; `+0xA4/+0x144` inherits the prior VXL family only when `0x2000` is present.
- Do not implement final parity as generic GPU `alpha=0.5`; native paths blend post-remap/indexed 16-bit values.

## 9. Stale Docs / Follow-up Docs

- `docs/research/CLOAKING_VISUAL_PIPELINE.md`: replace any blanket wording like "`flags & 6 == 0x04` with `0x800` selects the SHP/VXL 50% path" with: "`flags & 6 == 0x04` is only the low translucency family. In `TechnoClass_DrawSHP`, warp ORs this low bit for ordinary SHP technos, but the final selector depends on higher bits: with `0x2000` present, SHP warp selects the same `+0xA4/+0x144` mask-family as VXL warp; without `0x2000`, it selects the no-mask SHP `+0x78/+0x130` family."
- `docs/research/VXL_RASTERIZER_DISPATCH_GHIDRA_REPORT.md`: replace any wording that generalizes VXL `0x2804` to all techno warp bodies with: "The proved VXL body path enters with `0x2804`; SHP bodies build their own flags in `TechnoClass_DrawSHP` and can enter as `0x2E04` or `0x0E04` after `|0x600|0x800`, so SHP warp needs a separate material key."
- `docs/research/ZBUFFER_DEPTH_SYSTEM.md`: refine any claim that all SHP 50% paths are one Z behavior with: "SHP 50% selector behavior is branch-specific. The no-mask extended `+0x130` leaf decompiled at `0x00497CF0` performs a post-remap 50% blend and does not access `g_ZBuffer`; the `0x2000` mask-family `+0xA4/+0x144` branch should be cited separately."

## Sources

- Ghidra read-only decompile: `TechnoClass_DrawSHP @ 0x00705E00`.
- Ghidra read-only disassembly success: `0x00705E00..0x0070641F`.
- Ghidra read-only decompile: `CC_Draw_Shape @ 0x004AED70`.
- Ghidra read-only decompile/disassembly: `Blitter_selector @ 0x00490B90`, `0x00490B90..0x00490DF7`.
- Ghidra read-only decompile/disassembly: `Blitter_selector_extended @ 0x00490E50`, `0x00490E50..0x004910AF`.
- Ghidra read-only decompile: `Blitter_ZWriteOnly_RLE_Remap_NoZWrite @ 0x00497CF0`.
- Ghidra read-only decompile: `FootClass::GetVisualState @ 0x004DA4E0`, `TechnoClass_GetVisualState @ 0x00703860`.
- `ini/rulesmd.ini:[CLEG]`.
- `ini/artmd.ini:[CLEG]`, `[DLPH]`, `[DRON]`, `[SQD]`.
- Prior context: `docs/research/WARP_TRANSLUCENCY_BLITTER_PIXEL_MATH_GHIDRA_REPORT.md`.
