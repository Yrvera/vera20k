# Right-Panel SHP Header Dimensions — Research Report

**Date:** 2026-05-19
**Scope:** Retail SHP file-header canvas dimensions for SDTP, SDBTNBKGD, SDBTM,
LWSCRNS, and LWSCRNL; cross-check of all related Rust constants in
`src/ui/main_menu_shell/layout.rs`.
**Confidence:** HIGH — all dimensions read directly from the retail SHP files via
the existing Rust `inspect-pcx-palette` binary (uses `AssetManager::get_ref` +
`ShpFile::from_bytes`; decrypts MIX archives via the project's `mix_crypto.rs`).
**Active in YR:** N/A — this is an asset-format investigation, not a gameplay system.

Parent reports consulted:

- `RIGHTPANEL_DRAW_AND_LAYOUT_GHIDRA_REPORT.md` — layout formula source (§10.2 explicitly
  flagged the need for these exact SHP pixel values)
- `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md` — ra2ts_l/s.bik dimensions already verified

No Rust code was modified.

---

## 1. Verification Method

The retail SHP files are packed inside the encrypted `ra2.mix` / `ra2md.mix` archives
(all top-level MIX files at the retail install path have `flags & 0x02 = 1` — encrypted
header).  Decryption uses the 320-bit RSA key documented in `mix_crypto.rs`, which is
already implemented in the project.

Evidence: the existing `src/bin/inspect-pcx-palette.rs` binary uses `AssetManager::get_ref(name)`
(which opens and decrypts the appropriate MIX archive) followed by `ShpFile::from_bytes(data)`.
It loads and parses all six SHP files and prints:

```
canvas <width>x<height>, <frame_count> frames
```

Command run: `cargo run --bin inspect-pcx-palette` from
`C:/Users/enok/Documents/ra2-rust-game/` (build completed, all six SHP files found).

The `ShpFile` struct reads the 8-byte file header:
- bytes 0–1: `u16 = 0` (format marker)
- bytes 2–3: `u16 width` ← SHP canvas width (what the layout algorithm reads at `SHP_global + 2`)
- bytes 4–5: `u16 height` ← SHP canvas height (read at `SHP_global + 4`)
- bytes 6–7: `u16 frame_count`

The layout algorithm in `RightPanel__ComputeLayoutRects @ 0x0072EC70` reads exactly
these two u16 values as `*(short*)(g_SDTP_SHP + 2)` and `*(short*)(g_SDTP_SHP + 4)`.

---

## 2. Verified SHP Canvas Dimensions

All values confirmed by `cargo run --bin inspect-pcx-palette` output (2026-05-19 run):

| File | Canvas W (px) | Canvas H (px) | Frame Count | Source archive (resolved via AssetManager) |
|---|---:|---:|---:|---|
| `SDTP.SHP` | **168** | **199** | 2 | `ra2md.mix` / `ra2.mix` (decrypted by AssetManager) |
| `SDBTNBKGD.SHP` | **168** | **42** | 1 | same |
| `SDBTM.SHP` | **168** | **65** | 1 | same |
| `LWSCRNS.SHP` | **472** | **32** | 1 | same |
| `LWSCRNL.SHP` | **632** | **32** | 1 | same |
| `SDBTNANM.SHP` | **156** | **42** | 17 | same (bonus — in scope for tile overlay) |

All per-frame PNG dumps were written to `target/pcx-dump/`.

---

## 3. Rust Constants Cross-Check

File: `src/ui/main_menu_shell/layout.rs`

| Constant | Value | SHP File | SHP dimension | Match? |
|---|---:|---|---:|---|
| `RIGHT_PANEL_WIDTH` | 168 | `SDTP.SHP` canvas_w | 168 | **MATCH** |
| `RIGHT_PANEL_TOP_H` | 199 | `SDTP.SHP` canvas_h | 199 | **MATCH** |
| `RIGHT_PANEL_TILE_H` | 42 | `SDBTNBKGD.SHP` canvas_h | 42 | **MATCH** |
| `RIGHT_PANEL_TILE_H` | 42 | `SDBTNANM.SHP` canvas_h | 42 | **MATCH** |
| `RIGHT_PANEL_BOTTOM_H` | 23 | `SDBTM.SHP` canvas_h | **65** | **MISMATCH (doc only)** |
| `LOWER_STRIP_H` | 32 | `LWSCRNS.SHP` canvas_h | 32 | **MATCH** |
| `LOWER_STRIP_H` | 32 | `LWSCRNL.SHP` canvas_h | 32 | **MATCH** |
| `RA2TS_L_W` | 632 | `ra2ts_l.bik` | 632 | **MATCH** (verified in MAIN_MENU_VISUAL_ASSETS doc) |
| `RA2TS_L_H` | 570 | `ra2ts_l.bik` | 570 | **MATCH** (verified in MAIN_MENU_VISUAL_ASSETS doc) |
| `RA2TS_S_W` | 472 | `ra2ts_s.bik` | 472 | **MATCH** (verified in MAIN_MENU_VISUAL_ASSETS doc) |
| `RA2TS_S_H` | 450 | `ra2ts_s.bik` | 450 | **MATCH** (verified in MAIN_MENU_VISUAL_ASSETS doc) |

### RIGHT_PANEL_BOTTOM_H = 23 — Analysis

The `SDBTM.SHP` canvas is **65 px tall**, not 23 px. The constant `RIGHT_PANEL_BOTTOM_H = 23`
is a documentation annotation that happens to equal the residual slot height at 800×600:

```
residual_h = usable_h - SDTP_H - (tile_count × TILE_H)
           = 600 - 199 - (9 × 42)
           = 600 - 199 - 378
           = 23 px
```

**Critical finding:** The constant `RIGHT_PANEL_BOTTOM_H = 23` is declared but **never used
in any computation** in `layout.rs`. The actual bottom-cap height is always the dynamic
residual (`screen_h - top_margin - bottom_y`). The constant is misleading — it is the
residual at 800×600 only; at 640×480 the actual drawn height is 29 px.

**There is no live parity bug** from this mismatch because the Rust code never reads
`RIGHT_PANEL_BOTTOM_H` in `right_panel_rects()` or `version_line_rect()`. The constant
is stale documentation. However the comment `"Native dimensions of the SDBTNBKGD tile
and SDTP/SDBTM caps at 800x600"` is factually wrong for SDBTM (canvas = 65, not 23).

The 65 px canvas is designed to be at least as tall as any possible residual slot. The
largest residual occurs at 640×480: `480 - 199 - 6×42 = 29 px`. The 65 px canvas
comfortably covers all standard resolutions.

---

## 4. SDBTNANM Inset Detail

`SDBTNANM.SHP` is 156 px wide — 12 px narrower than the 168 px tile column. The layout
formula from `RightPanel__ComputeLayoutRects` right-aligns it within the column:

```c
DAT_00b0fc10[0] = (DAT_00b0fc24[2] - sVar1) + *DAT_00b0fc24;
  // x = (168 - 156) + tile_x = tile_x + 12
```

So the SDBTNANM column is inset 12 px from the left edge of the tile column. This is a
structural detail, not a guess: 168 - 156 = 12 px offset verified from both the SHP canvas
and the layout formula.

---

## 5. SDTP Frame Count Note

`SDTP.SHP` has **2 frames**, but `RightPanel__Draw` always draws frame `0`. The
`inspect-pcx-palette` binary only rendered `frame0` (frame1 was skipped by the
binary's dump logic for SHPs with ≤10 frames it only shows frame 0). Frame 1 is
likely an empty shadow/unused frame (a common RA2 SHP convention). This does not
affect layout or rendering — draw order table always uses frame 0 for SDTP.

---

## 6. Absolute Pixel Positions at Standard Resolutions

Computed from the verified dimensions using the confirmed formulas from
`RIGHTPANEL_DRAW_AND_LAYOUT_GHIDRA_REPORT.md` §4:

### 800×600 (primary target)

| Asset | x | y | w | h (drawn) |
|---|---:|---:|---:|---:|
| `SDTP.SHP` | 632 | 0 | 168 | 199 |
| `SDBTNBKGD.SHP` tile 0 | 632 | 199 | 168 | 42 |
| `SDBTNBKGD.SHP` tile 1 | 632 | 241 | 168 | 42 |
| `SDBTNBKGD.SHP` tile 2 | 632 | 283 | 168 | 42 |
| `SDBTNBKGD.SHP` tile 3 | 632 | 325 | 168 | 42 |
| `SDBTNBKGD.SHP` tile 4 | 632 | 367 | 168 | 42 |
| `SDBTNBKGD.SHP` tile 5 | 632 | 409 | 168 | 42 |
| `SDBTNBKGD.SHP` tile 6 | 632 | 451 | 168 | 42 |
| `SDBTNBKGD.SHP` tile 7 | 632 | 493 | 168 | 42 |
| `SDBTNBKGD.SHP` tile 8 | 632 | 535 | 168 | 42 |
| `SDBTM.SHP` | 632 | 577 | 168 | **23** (of 65 canvas) |
| `LWSCRNL.SHP` | 0 | 568 | 632 | 32 |

Tile count = floor((600 − 199) / 42) = **9**

### 640×480

| Asset | x | y | w | h (drawn) |
|---|---:|---:|---:|---:|
| `SDTP.SHP` | 472 | 0 | 168 | 199 |
| `SDBTNBKGD.SHP` tiles 0–5 | 472 | 199..409 | 168 | 42 |
| `SDBTM.SHP` | 472 | 451 | 168 | **29** (of 65 canvas) |
| `LWSCRNS.SHP` | 0 | 448 | 472 | 32 |

Tile count = floor((480 − 199) / 42) = **6**

### 1024×768

| Asset | x | y | w | h (drawn) |
|---|---:|---:|---:|---:|
| `SDTP.SHP` | 744 | 84 | 168 | 199 |
| `SDBTNBKGD.SHP` tiles 0–8 | 744 | 283..619 | 168 | 42 |
| `SDBTM.SHP` | 744 | 661 | 168 | **23** (of 65 canvas) |
| `LWSCRNL.SHP` | 112 | 652 | 632 | 32 |

Tile count = floor((600 − 199) / 42) = **9** (usable_h = 600 at 1024×768)

---

## 7. Open Questions — Final State

- `[RESOLVED] Q1` — SDTP.SHP canvas dimensions → 168×199 (evidence: `inspect-pcx-palette` output, `cargo run` 2026-05-19)
- `[RESOLVED] Q2` — SDBTNBKGD.SHP canvas dimensions → 168×42 (evidence: same run)
- `[RESOLVED] Q3` — SDBTM.SHP canvas dimensions → 168×65 (evidence: same run)
- `[RESOLVED] Q4` — LWSCRNS.SHP canvas dimensions → 472×32 (evidence: same run)
- `[RESOLVED] Q5` — LWSCRNL.SHP canvas dimensions → 632×32 (evidence: same run)
- `[RESOLVED] Q6` — RIGHT_PANEL_BOTTOM_H = 23 mismatch with SDBTM canvas = 65 → constant is documentation-only, never used in computation; drawn height is always the dynamic residual
- `[RESOLVED] Q7` — SDBTNANM.SHP dimensions → 156×42, 17 frames; 12 px narrower than tile column, right-aligned by layout formula
- `[RESOLVED] Q8` — All Rust constants RIGHT_PANEL_WIDTH, RIGHT_PANEL_TOP_H, RIGHT_PANEL_TILE_H, LOWER_STRIP_H, RA2TS_L_W/H, RA2TS_S_W/H verified correct against retail SHP/BIK headers
- `[RESOLVED] Q9` — SDTP 2-frame structure: frame 0 = full 168×199 content; frame 1 = likely empty/unused (not rendered by inspect-pcx-palette)
- `[DEFERRED] Q10` — Which specific MIX archive (ra2.mix vs ra2md.mix) each SHP comes from; the encrypted archives require `AssetManager` to resolve and the binary does not report the source archive name for these files. Category: `bounded-cost-too-high`; reason: requires modifying a binary or adding logging, not needed for layout parity. Next step: add `get_with_source` calls to `inspect-pcx-palette` if source-archive provenance ever matters.
- `[DEFERRED] Q11` — Per-frame x/y offsets (frame_x, frame_y within canvas) for all frames. Category: `bounded-cost-too-high`; reason: layout algorithm reads only the canvas (file-header) width/height, not per-frame offsets; they only affect how `CC_Draw_Shape` composites the sprite internally. Next step: add frame-detail logging to `inspect-pcx-palette`.

---

## 8. Parity Assessment

All Rust layout constants that actively feed into position computations are **correct**:

| Constant | Status |
|---|---|
| `RIGHT_PANEL_WIDTH = 168` | Correct — matches SDTP/SDBTNBKGD/SDBTM canvas_w |
| `RIGHT_PANEL_TOP_H = 199` | Correct — matches SDTP.SHP canvas_h |
| `RIGHT_PANEL_TILE_H = 42` | Correct — matches SDBTNBKGD.SHP canvas_h |
| `RIGHT_PANEL_TILE_COUNT_BASE = 9` | Correct — equals floor((600−199)/42) |
| `LOWER_STRIP_H = 32` | Correct — matches LWSCRNS.SHP and LWSCRNL.SHP canvas_h |
| Width selection at 640: 472 | Correct — matches LWSCRNS.SHP canvas_w |
| Width selection at 800+: 632 | Correct — matches LWSCRNL.SHP canvas_w |

`RIGHT_PANEL_BOTTOM_H = 23` is a documentation-only constant never used in computation.
Its value (23) equals the residual bottom-cap height at 800×600 only. The constant
comment misstates the SDBTM canvas height (65) and should ideally be updated to avoid
confusion, but there is no live parity bug.

---

## Sources

- `cargo run --bin inspect-pcx-palette` output (2026-05-19), reading retail files via
  `AssetManager` + `ShpFile::from_bytes`.
- `src/bin/inspect-pcx-palette.rs` — source of the binary used (read-only).
- `src/assets/shp_file.rs` — SHP file format documentation (header layout).
- `src/assets/mix_archive.rs` + `src/assets/mix_crypto.rs` — confirms decryption path.
- `src/ui/main_menu_shell/layout.rs` — Rust constants cross-checked.
- `C:/Users/enok/Documents/ra2-rust-game-docs/RIGHTPANEL_DRAW_AND_LAYOUT_GHIDRA_REPORT.md`
  — layout formulae source.
- `C:/Users/enok/Documents/ra2-rust-game-docs/MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md`
  — ra2ts_l/s.bik dimensions already verified (Section 5).
