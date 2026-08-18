# Minimap Retail TMP / Overlay Radar Color Table Dump - Ghidra Research Report

**Address(es):** `CellClass__GetRadarColor @ 0x0047C060`, `OverlayClass__GetRadarColor @ 0x005FED00`, `GetTiberiumRadarColor @ 0x0069E860`  
**Investigation Mode:** exhaustive-slice for representative stock asset values  
**Claimed Scope:** Retail TMP subimage radar RGB metadata and overlay/tiberium SHP-frame radar RGB metadata consumed by the already-verified `CellClass::GetRadarColor` branches.  
**Non-Scope:** Dirty queues, object dots, radar scaling/sampling, gap/spy/shroud, radar events, or exhaustive all-assets CSV generation.  
**Confidence:** High for sampled offsets and values; Medium only for all-assets coverage not attempted here.  
**Active in YR:** Conditional. The code path is active in ordinary YR minimap generation; each value is active when the selected theater/overlay/frame is present.

Target question: What exact retail TMP/SHP metadata values does the native minimap color path consume for representative stock YR terrain and overlay assets?  
Non-goals: Do not re-investigate branch order, dirty queues, object dots, or click handling.  
Evidence needed to mark COMPLETE: binary proof of consumed offsets plus direct retail asset bytes for representative TMP terrain, ore/gem overlays, low/high bridge overlays, and fallback cases.  
Stop conditions: Stop after representative stock values prove the Rust fixture requirements; list unexhausted all-assets coverage as remaining uncertainty.

## 1. Overview

Native minimap color uses asset header metadata, not rendered-pixel averages. `GetTiberiumRadarColor` reads SHP frame-header bytes at `shp + 8 + frame * 0x18 + 0x0C..0x0E`. The terrain fallback reads TMP subimage bytes at `subimage + 0x2B..0x2D`, applies theater brightness, then halves channels.

Representative values already disprove current Rust parity: retail `TIB01` frame 11 is `(169,155,61)`, `GEM01` frame 11 is `(114,111,118)`, `GEM12` frame 11 is `(107,100,109)`, and `LOBRDG01` frame 1 in temperate is `(0,0,4)`. These are not INI `RadarColor=` values and not averages of rendered pixels.

## 2. Consumed Offsets

| Format | Consumed bytes | Evidence | Active in YR |
|---|---|---|---|
| SHP(TS) frame radar RGB | file header 8 bytes, then 24-byte frame headers; RGB at header `+0x0C,+0x0D,+0x0E` | `GetTiberiumRadarColor @ 0x0069E860`; `src/assets/shp_file.rs` documents bytes 12-15 as radar minimap color | Conditional |
| TMP subimage radar RGB | TMP tile-cell header bytes `+43..+45` and `+46..+48`; native selected subimage reads equivalent `+0x2B..+0x2D` | `CellClass__GetRadarColor @ 0x0047C060`; `src/assets/tmp_decode.rs` | Yes for terrain fallback |
| SHP missing/out-of-range frame | returns black `(0,0,0)` | `GetTiberiumRadarColor @ 0x0069E860` | Conditional |
| TMP missing subimage | returns `(60,60,60)` | `CellClass__GetRadarColor @ 0x0047C060` | Conditional |

## 3. Representative TMP Terrain Values

Retail asset root: `<ra2-install>/`.

| Asset | Source | Grid / tile | First non-empty subimage metadata | Active in YR |
|---|---|---|---|---|
| `clat01.tem` | `ra2.mix -> isotemp.mix (#BCCC4D97)` | `1x1`, `60x30` | terrain `13`, left `(144,151,64)`, right `(143,151,60)` | Yes, Temperate |
| `shore01.tem` | `ra2.mix -> isotemp.mix (#BCCC4D97)` | `2x2`, `60x30` | terrain `14`, left `(219,186,123)`, right `(229,191,119)` | Yes, Temperate |
| `water01.tem` | `ra2.mix -> isotemp.mix (#BCCC4D97)` | `2x2`, `60x30` | terrain `9`, left/right `(65,67,88)` | Yes, Temperate |
| `cliff01.tem` | `ra2.mix -> isotemp.mix (#BCCC4D97)` | `2x3`, `60x30` | height `4`, terrain `15`, left `(113,82,46)`, right `(119,100,52)` | Yes, Temperate |
| `clat01.sno` | `ra2.mix -> isosnow.mix (#0F5D1D99)` | `1x1`, `60x30` | terrain `14`, left `(199,211,197)`, right `(195,206,191)` | Yes, Snow |
| `clat01.urb` | `ra2.mix -> isourb.mix (#80E03363)` | `1x1`, `60x30` | terrain `11`, left `(105,79,47)`, right `(106,80,50)` | Yes, Urban |
| `clat01.ubn` | `ra2md.mix -> isoubn.mix (#3BFB683C)` | `1x1`, `60x30` | terrain `13`, left `(146,150,65)`, right `(144,149,61)` | Yes, YR NewUrban |

Rust currently reads these triplets but then averages left/right in `src/render/minimap_helpers.rs::radar_color_for_cell`. Native `CellClass::GetRadarColor` currently returns identical left/right outputs, but the raw radar writer still has two raw positions per cell; test fixtures should preserve source triplets and validate native selected-subimage brightness/halving, not category palettes.

## 4. Representative Overlay / Tiberium SHP Values

All values are raw frame-header RGB bytes at `8 + frame * 24 + 12..14`.

| Asset | Source | Frames | Sample values | Active in YR |
|---|---|---:|---|---|
| `tib01.tem` | `ra2.mix -> temperat.mix (#5AA5B016)` | 24 | f0 `(174,156,133)`, f1 `(175,158,122)`, f2 `(175,162,111)`, f11 `(169,155,61)` | Yes |
| `tib12.tem` | `ra2.mix -> temperat.mix (#5AA5B016)` | 24 | f0 `(181,168,95)`, f1 `(193,181,90)`, f2 `(186,172,80)`, f11 `(166,151,57)` | Yes |
| `gem01.tem` | `ra2.mix -> temperat.mix (#5AA5B016)` | 24 | f0 `(127,112,141)`, f1 `(106,96,126)`, f2 `(117,102,112)`, f11 `(114,111,118)` | Yes |
| `gem12.tem` | `ra2.mix -> temperat.mix (#5AA5B016)` | 24 | f0 `(122,61,87)`, f1 `(100,50,78)`, f2 `(105,65,98)`, f11 `(107,100,109)` | Yes |
| `tib01.ubn` | `ra2md.mix -> urbann.mix (#A01A9A03)` | 24 | f0 `(174,156,133)`, f1 `(175,158,122)`, f11 `(169,155,61)` | Yes for NewUrban |
| `gem01.ubn` | `ra2md.mix -> urbann.mix (#A01A9A03)` | 24 | f0 `(127,112,141)`, f1 `(106,96,126)`, f11 `(114,111,118)` | Yes for NewUrban |

The sampled ore/gem metadata is identical across `.tem`, `.sno`, `.urb`, and `.ubn` for these stock assets, but this is not a global all-assets proof. It is enough to prove Rust must use per-frame header data rather than interpolation.

## 5. Bridge / Crate Values

| Asset | Source | Frames | Sample values | Active in YR |
|---|---|---:|---|---|
| `lobrdg01.tem` | `ra2.mix -> isotemp.mix (#BCCC4D97)` | 6 | f0 empty/black `(0,0,0)`, f1 `(0,0,4)` | Yes when forced-frame range selects frame 1 |
| `lobrdg24.tem` | `ra2.mix -> isotemp.mix (#BCCC4D97)` | 6 | f0 empty/black, f1 empty/black | No for normal color branch; overlay index 100 is skipped |
| `lobrdg25.tem` | `ra2.mix -> isotemp.mix (#BCCC4D97)` | 6 | f0 empty/black, f1 `(0,0,5)` | No for normal color branch; overlay index 101 is skipped |
| `lobrdg26.tem` | `ra2.mix -> isotemp.mix (#BCCC4D97)` | 6 | f0 empty/black, f1 `(0,0,3)` | Yes when present |
| `bridge.tem` | `ra2.mix -> temperat.mix (#5AA5B016)` | 36 | f0 `(0,0,6)`, f1 `(0,0,5)` | Conditional; structural bridge flag uses frame 0 |
| `bridge.ubn` | `ra2md.mix -> urbann.mix (#A01A9A03)` | 36 | f0 `(109,110,109)`, f1 `(107,107,107)` | Conditional in NewUrban |
| `bridgb.tem` | `ra2.mix -> temperat.mix (#5AA5B016)` | 36 | f0 `(0,0,4)`, f1 `(0,0,4)` | Conditional for wooden high bridge variants |
| `wcrate.tem` | `ra2.mix -> temperat.mix (#5AA5B016)` | 2 | f0 `(103,85,59)`, f1 `(0,0,0)` | Conditional on water crate overlay |

## 6. Current Rust Implementation Status

- `src/assets/tmp_decode.rs` already reads TMP radar triplets at the established offsets.
- `src/assets/shp_file.rs` documents SHP radar metadata but does not expose frame radar RGB in `ShpFrame`.
- `src/render/minimap_helpers.rs::OverlayClassification::color` interpolates ore/gem colors and uses constants.
- `src/render/overlay_atlas.rs::compute_tiberium_radar_colors` averages rendered non-transparent pixels; native minimap does not.
- `src/render/minimap_helpers.rs::radar_color_for_cell` averages TMP left/right and uses `f32` brightness, not native brightness plus unsigned `>> 1`.

## 7. Coverage Ledger

| Area / branch | Status | Evidence | What remains |
|---|---|---|---|
| SHP frame RGB offset | verified | `0x0069E860`; retail SHP frame reads | none for sampled assets |
| TMP metadata offset | verified | `0x0047C060`; `src/assets/tmp_decode.rs`; retail TMP reads | post-brightness numeric fixtures |
| Ore/gem representative values | verified | retail `tib01/tib12/gem01/gem12` frame headers | all frames/variants not tabulated |
| Low bridge representative values | verified | retail `lobrdg01/24/25/26`; branch skip/forced-frame evidence | all low-bridge files not tabulated |
| INI `RadarColor=` relationship | verified-negative | `0x005FED00`, `0x0069E860`; `rulesmd.ini` checked | none for this branch |
| Rust comparison | verified | `src/render/minimap_helpers.rs`, `src/render/overlay_atlas.rs`, `src/assets/shp_file.rs` | no Rust edits |

## 8. Open Questions - Final State

- `[RESOLVED] OQ1 - Is the sampled metadata on the active YR minimap path? -> Yes; these bytes are exactly the SHP/TMP metadata read by active `CellClass__GetRadarColor` callees.` (evidence: `0x0047C060`, `0x005FED00`, `0x0069E860`)
- `[RESOLVED] OQ2 - Are INI `RadarColor=` values sufficient? -> No; sampled `TIB01` INI `220,200,0` differs from retail `tib01.tem` f11 `(169,155,61)`, and the binary reads SHP frame metadata.` (evidence: `0x0069E860`, `rulesmd.ini`)
- `[RESOLVED] OQ3 - Are ore/gem colors one linear gradient? -> No; sampled frame metadata varies by asset and frame.` (evidence: retail `tib01/gem01/gem12` headers)
- `[RESOLVED] OQ4 - Do skipped low-bridge assets still exist? -> Yes; `lobrdg24.tem` and `lobrdg25.tem` exist, but indices 100 and 101 are skipped by `GetRadarColor`.` (evidence: retail headers; `0x0047C060`)
- `[RESOLVED] OQ5 - Are NewUrban values available from YR assets? -> Yes; `.ubn` overlay files resolve from `ra2md.mix` nested archives.` (evidence: retail reads)
- `[DEFERRED] OQ6 - Exhaustive all-frame/all-overlay table.` (category: bounded-cost-too-high; reason: representative fixtures were requested; next-step-if-pursued: automated all-assets CSV)
- `[DEFERRED] OQ7 - Exact post-`ApplyTheaterBrightness` fixtures per scenario lighting.` (category: requires-different-system-context; reason: this slot focuses retail asset bytes; next-step-if-pursued: brightness formula fixture report)

## 9. Visual Composition Ledger

This report covers visual color source metadata, not draw composition.

| Order | Function / address | Condition | Asset / frame | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|
| 1 | `CellClass__GetRadarColor @ 0x0047C060` | per terrain/dirty cell | TMP subimage or overlay SHP frame | raw RGB metadata | Yes | raw color source |
| 2 | `OverlayClass__GetRadarColor @ 0x005FED00` | overlay/bridge branch | resolved overlay SHP frame | raw SHP RGB, possible byte swap for specific ranges | Conditional | overlay metadata reader |
| 3 | `GetTiberiumRadarColor @ 0x0069E860` | SHP pointer and in-range frame | frame header RGB | raw SHP RGB | Conditional | frame color extractor |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Overlay/tiberium minimap colors come from SHP frame-header RGB, e.g. `tib01.tem` f11 `(169,155,61)` and `gem01.tem` f11 `(114,111,118)`. | `0x0069E860`; retail `ra2.mix -> temperat.mix (#5AA5B016)` | Rust interpolates/averages. | `src/assets/shp_file.rs`, `src/render/minimap_helpers.rs`, `src/render/overlay_atlas.rs` | Expose/load SHP frame radar RGB metadata and use it for overlay/tiberium minimap color. | Temperate `TIB01` frame 11 returns `(169,155,61)`, not interpolated yellow. Proposed test: `minimap_overlay_tib01_frame11_uses_shp_header_radar_rgb`. | Do not average rendered pixels or use INI `RadarColor=`. |
| TMP terrain fallback consumes per-subimage metadata triplets, e.g. `clat01.tem` left `(144,151,64)` and right `(143,151,60)`. | `0x0047C060`; `src/assets/tmp_decode.rs`; retail `isotemp.mix` | Rust averages left/right with `f32` brightness. | `src/assets/tmp_decode.rs`, `src/render/minimap_helpers.rs`, native raw-radar buffer | Preserve TMP triplets and test native brightness/`>>1` separately. | Temperate `clat01.tem` fixture verifies source triplets before shift. Proposed test: `minimap_tmp_clat01_tem_uses_header_triplets_before_shift`. | Do not replace TMP metadata with land/water/elevation colors. |
| Low-bridge forced frame and skip behavior must use real asset data: `lobrdg01.tem` frame 1 `(0,0,4)` is used; indices 100/101 are skipped despite files. | `0x0047C060`; retail low-bridge headers | Rust uses generic bridge constant. | `src/render/minimap_helpers.rs`, map overlay registry / raw cell color equivalent | Apply native skip list and forced-frame rules before SHP metadata read. | Overlay id 74 uses `LOBRDG01` frame 1; ids 100/101 fall through. Proposed test: `minimap_low_bridge_forced_frame_and_skip_assets_match_gamemd`. | Do not assume asset existence means branch liveness. |

## 11. Negative Facts / Do Not Do

- Do not use INI `RadarColor=` as the active minimap overlay color. Active in YR: No for this branch; evidence: `0x005FED00`, `0x0069E860`.
- Do not linearly interpolate ore/gem density colors. Active in YR: No; evidence: exact frame-header metadata.
- Do not compute radar colors by averaging rendered non-transparent SHP pixels. Active in YR: No; evidence: `0x0069E860` reads header bytes.
- Do not use `LOBRDG24`/`LOBRDG25` file data in `CellClass::GetRadarColor`. Active in YR: No for overlay indices 100/101; evidence: skip list.
- Do not collapse all theater variants into one fixture. Active in YR: Conditional; evidence: `bridge.tem` f0 `(0,0,6)` differs from `bridge.ubn` f0 `(109,110,109)`.

## 12. Remaining Uncertainty

- Full all-assets table is not exhausted.
- Exact post-`ApplyTheaterBrightness` numeric fixtures per theater/scenario lighting value remain a separate formula task.
- The asset dump used a local Python reader matching the repo's documented MIX/SHP/TMP offsets. `cargo build -q --lib` was blocked by unrelated current compile errors in `sprite_atlas.rs` / `app_instances/shp.rs`.

## 13. Stale Docs / Follow-up Docs

`docs/research/RADAR_MINIMAP_RENDERING.md`

Replace any statement implying ore/gem minimap colors are density-interpolated, INI `RadarColor=` driven, or rendered-pixel averaged with:

> Overlay and tiberium minimap terrain colors are loaded from resolved SHP(TS) frame-header radar RGB bytes at frame header `+0x0C..+0x0E`. For example, retail `tib01.tem` frame 11 is `(169,155,61)` and retail `gem01.tem` frame 11 is `(114,111,118)`; these values do not match INI `RadarColor=` and are not rendered-pixel averages.

`docs/research/RADAR_SYSTEM_COMPREHENSIVE.md`

Replace the old high-level color list with:

> The terrain fallback reads TMP subimage radar metadata bytes, applies theater brightness, then halves channels. The overlay/tiberium branch reads resolved SHP frame-header RGB metadata through `GetTiberiumRadarColor`; INI `RadarColor=` is not the direct minimap color source for this branch.

## Sources

- Ghidra decompile: `CellClass__GetRadarColor @ 0x0047C060`, `OverlayClass__GetRadarColor @ 0x005FED00`, `GetTiberiumRadarColor @ 0x0069E860`, `ApplyTheaterBrightness @ 0x00661190`.
- Prior branch report: `docs/research/CELLCLASS_GETRADARCOLOR_FULL_BRANCH_INVENTORY_GHIDRA_REPORT.md`.
- Retail assets under `<ra2-install>/`: `ra2.mix`, `ra2md.mix`, nested `isotemp.mix`, `temperat.mix`, `isosnow.mix`, `snow.mix`, `isourb.mix`, `urban.mix`, `isoubn.mix`, `urbann.mix`.
- Rust parser surfaces: `src/assets/tmp_decode.rs`, `src/assets/shp_file.rs`, `src/render/minimap_helpers.rs`, `src/render/overlay_atlas.rs`, `src/map/overlay_types.rs`.

## Status

COMPLETE for representative stock YR asset metadata values and Rust-facing test handoff. Partial only for exhaustive all-assets CSV generation and post-brightness formula fixtures, which are intentionally outside this slot.
