# Skirmish Retail Stock Map Preview Census - Ghidra Research Report

**Date:** 2026-05-21  
**Address(es):** `0x00640710`, `0x00641B00`, `0x00689D30`, `0x006418B0`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** Standard retail offline Skirmish map files in `C:/Users/enok/Documents/Command and Conquer Red Alert II/` with extensions `.mmx`, `.yro`, `.map`, `.mpr`, `.yrm`: `[PreviewPack]` presence/decode, `[Header] NumberStartingPoints` / `WaypointN` presence, live `STARTBUT.SHP` overlay eligibility, and baked red preview-marker pixels.  
**Non-Scope:** MIX-contained maps not present as root stock map files, custom user maps, runtime screenshot capture, exact `STARTBUT.SHP` pixel clipping, map chooser list filtering/sorting, and generated random-map UI beyond prior binary facts.  
**Confidence:** High for the 54 local retail root map files and the binary gates checked here. Medium for treating this as the full "stock map" universe because this pass did not enumerate nested archive map lists beyond the local retail root files.  
**Active in YR:** Yes. The selected-map preview decode path is active in offline Skirmish, and the retail files checked here are the local map files consumed by that flow.

## 1. Overview

All 54 checked retail root map files have `[Preview]` and non-empty `[PreviewPack]`, and all 54 decode to RGB preview pixels whose byte count equals `width * height * 3`. Every checked map also contains baked solid-red `4x4` preview start-marker pixels.

Only 9 maps, all `.yro`, contain `[Header] NumberStartingPoints` with a value in the live overlay range `1..8`: `CrctBrd.yro`, `DeepFrze.yro`, `HighExpR.yro`, `Ice_Age.yro`, `IrvineCa.yro`, `MonsterM.yro`, `MoonPatr.yro`, `SinkSwim.yro`, and `Transylv.yro`. These maps exercise both baked red preview pixels and live `STARTBUT.SHP`/number label overlays. The remaining 45 maps, including both `Dustbowl.map` and `Dustbowl.mmx`, exercise baked-preview-only behavior: decode `[PreviewPack]`, skip live overlays.

## 2. Binary Gates Reused By The Census

| Finding | Active in YR | Evidence |
|---|---|---|
| Selected-map preview decode reads `[Preview]` / `[PreviewPack]`, uses width/height from `[Preview]`, LZO-decompresses, and writes one RGB triple per surface pixel. | Yes | `0x00641B00`; prior `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`. |
| `[Header]` preview metadata defaults to `-1` for `StartX`, `StartY`, `Width`, `Height`, and `NumberStartingPoints`; eight `WaypointN` pairs are zeroed before reads. | Yes | `FUN_00689D30 @ 0x00689D30`. |
| Live marker overlays draw only when `0 < ScenarioClass+0x113C < 9`. Missing `[Header] NumberStartingPoints` leaves `+0x113C = -1`, so overlays are skipped. | Yes | `DrawStartPositions @ 0x00640710`. |
| Baked red markers are direct preview-surface pixels, not `STARTBUT.SHP`; generated/saved preview writer uses solid RGB `(0xF0,0,0)` `4x4` rectangles for waypoint indices `0..7`. | Conditional: active for generated/saved preview data and observable in stock `[PreviewPack]` payloads. | `GenerateTerrainPreview @ 0x00641140`; writer `0x006418B0`; decoded retail payloads in this census. |

## 3. Census Method

Local data source:

`C:/Users/enok/Documents/Command and Conquer Red Alert II/`

Files scanned:

- `40` `.mmx`
- `13` `.yro`
- `1` `.map`
- `0` `.mpr` / `.yrm`
- Total: `54`

The retail map files are MIX-wrapped in this install, but their inner map INI payloads are visible in the file body. The census parsed INI sections from the local retail files, concatenated numbered `[PreviewPack]` values in numeric order, base64-decoded and LZO-decompressed them using the already-verified repo algorithm, and then scanned RGB triples for connected `4x4` `(240,0,0)` components. The decode byte counts below are `width * height * 3`, matching `0x00641B00`.

## 4. Path Buckets

| Path bucket | Count | Maps |
|---|---:|---|
| PreviewPack decode + baked red pixels + no live overlays | 45 | All checked `.mmx`, `Dustbowl.map`, plus `.yro` maps without `[Header]`: `IsleLand.yro`, `MojoSprt.yro`, `RiverRam.yro`, `Unrepent.yro`. |
| PreviewPack decode + baked red pixels + live STARTBUT overlays eligible | 9 | `CrctBrd.yro`, `DeepFrze.yro`, `HighExpR.yro`, `Ice_Age.yro`, `IrvineCa.yro`, `MonsterM.yro`, `MoonPatr.yro`, `SinkSwim.yro`, `Transylv.yro`. |
| Missing or empty PreviewPack | 0 | None in the checked stock root set. |
| Header present but overlay-ineligible count | 0 | None in the checked stock root set. |
| No baked red `4x4` markers | 0 | None in the checked stock root set. |

## 5. Implementation Fixture Picks

| Fixture | Why it matters | Active in YR | Evidence |
|---|---|---|---|
| `Dustbowl.map` / `Dustbowl.mmx` | Baked-only path: `[PreviewPack]` decodes, no `[Header]`, no live overlays. | Yes | Both have `Preview=0,0,138,75`, no `[Header]`, `2` baked `4x4` red components. |
| `CrctBrd.yro` | Small live-overlay fixture with `NumberStartingPoints=4`, `[Header] Waypoint1..8`, and `4` baked markers. | Yes | Local file census; `0x00640710` count gate. |
| `IrvineCa.yro` | Max normal live-overlay fixture with `NumberStartingPoints=8`, `8` gameplay starts, and `8` baked markers. | Yes | Local file census; `0x00640710` count gate accepts `8`. |
| `HighExpR.yro` | Live-overlay fixture with non-RA2 theater `DESERT`, `NumberStartingPoints=6`, and `6` baked markers. | Yes | Local file census. |
| `MonsterM.yro` | Header count `8` with compact `130x71` preview, useful for dense marker/label checks. | Yes | Local file census. |

## 6. Census Table

`HeaderN` is `[Header] NumberStartingPoints`; `HeaderWps` counts present `[Header] Waypoint1..Waypoint8` keys; `Live` is whether `0 < HeaderN < 9`; `WP0-7` counts gameplay `[Waypoints]` indices `0..7`; `Red4` counts exact connected `4x4` RGB `(240,0,0)` preview components.

| File | Preview | HeaderN | HeaderWps | Live | WP0-7 | Red4 |
|---|---:|---:|---:|---:|---:|---:|
| `amazon.mmx` | `138x89` | none | 0 | no | 4 | 4 |
| `Arena.mmx` | `158x79` | none | 0 | no | 4 | 4 |
| `Barrel.mmx` | `138x72` | none | 0 | no | 2 | 2 |
| `BayOPigs.mmx` | `270x139` | none | 0 | no | 6 | 6 |
| `Bermuda.mmx` | `158x79` | none | 0 | no | 6 | 6 |
| `Break.mmx` | `178x97` | none | 0 | no | 3 | 3 |
| `Carville.mmx` | `158x85` | none | 0 | no | 4 | 4 |
| `CrctBrd.yro` | `160x81` | 4 | 8 | yes | 4 | 4 |
| `Deadman.mmx` | `138x64` | none | 0 | no | 2 | 2 |
| `Death.mmx` | `246x131` | none | 0 | no | 8 | 8 |
| `DeepFrze.yro` | `190x95` | 4 | 8 | yes | 4 | 4 |
| `Disaster.mmx` | `172x99` | none | 0 | no | 4 | 4 |
| `Dustbowl.map` | `138x75` | none | 0 | no | 2 | 2 |
| `Dustbowl.mmx` | `138x75` | none | 0 | no | 2 | 2 |
| `EB1.mmx` | `118x61` | none | 0 | no | 4 | 4 |
| `EB2.mmx` | `198x99` | none | 0 | no | 4 | 4 |
| `EB3.mmx` | `198x79` | none | 0 | no | 2 | 2 |
| `EB4.mmx` | `198x79` | none | 0 | no | 4 | 4 |
| `EB5.mmx` | `118x70` | none | 0 | no | 4 | 4 |
| `GoldSt.mmx` | `204x82` | none | 0 | no | 6 | 6 |
| `Grinder.mmx` | `186x96` | none | 0 | no | 2 | 2 |
| `HailMary.mmx` | `152x87` | none | 0 | no | 2 | 2 |
| `HighExpR.yro` | `110x76` | 6 | 8 | yes | 6 | 6 |
| `Hills.mmx` | `158x79` | none | 0 | no | 4 | 4 |
| `Ice_Age.yro` | `202x102` | 6 | 8 | yes | 6 | 6 |
| `invasion.mmx` | `158x79` | none | 0 | no | 4 | 4 |
| `IrvineCa.yro` | `230x72` | 8 | 8 | yes | 8 | 8 |
| `IsleLand.yro` | `168x79` | none | 0 | no | 6 | 6 |
| `Kaliforn.mmx` | `178x71` | none | 0 | no | 6 | 6 |
| `Killer.mmx` | `178x99` | none | 0 | no | 3 | 3 |
| `Lostlake.mmx` | `238x82` | none | 0 | no | 4 | 4 |
| `MojoSprt.yro` | `146x70` | none | 0 | no | 4 | 4 |
| `MonsterM.yro` | `130x71` | 8 | 8 | yes | 8 | 8 |
| `MoonPatr.yro` | `110x52` | 4 | 8 | yes | 4 | 4 |
| `NewHghts.mmx` | `98x56` | none | 0 | no | 2 | 2 |
| `Oceansid.mmx` | `168x100` | none | 0 | no | 4 | 4 |
| `Pacific.mmx` | `178x94` | none | 0 | no | 4 | 4 |
| `Potomac.mmx` | `178x89` | none | 0 | no | 6 | 6 |
| `PowdrKeg.mmx` | `238x124` | none | 0 | no | 8 | 8 |
| `RiverRam.yro` | `94x49` | none | 0 | no | 2 | 2 |
| `Rockets.mmx` | `198x99` | none | 0 | no | 4 | 4 |
| `Roulette.mmx` | `312x83` | none | 0 | no | 8 | 8 |
| `Round.mmx` | `134x77` | none | 0 | no | 4 | 4 |
| `SeaofIso.mmx` | `178x79` | none | 0 | no | 4 | 4 |
| `Shrapnel.mmx` | `162x81` | none | 0 | no | 4 | 4 |
| `SinkSwim.yro` | `50x36` | 4 | 8 | yes | 4 | 4 |
| `Tanyas.mmx` | `158x79` | none | 0 | no | 4 | 4 |
| `Tower.mmx` | `198x99` | none | 0 | no | 4 | 4 |
| `Transylv.yro` | `130x60` | 6 | 8 | yes | 6 | 6 |
| `Tsunami.mmx` | `186x98` | none | 0 | no | 4 | 4 |
| `Unrepent.yro` | `194x78` | none | 0 | no | 6 | 6 |
| `Valley.mmx` | `208x105` | none | 0 | no | 4 | 4 |
| `xmas.mmx` | `148x81` | none | 0 | no | 2 | 2 |
| `YuriPlot.mmx` | `198x119` | none | 0 | no | 3 | 3 |

## 7. Rust Implementation Status

Active in YR: Not applicable; this is implementation comparison.

Current Rust already has the right separation of concepts:

- `src/map/preview.rs` decodes `[PreviewPack]` as RGB triples and parses four-field `[Preview] Size=`.
- `src/app_list_maps.rs` still leaves `preview_source_bounds` as `None`, avoiding the stale `[Map] LocalSize` shortcut.
- `src/app_skirmish_shell_render.rs` only draws decoded preview surfaces and keeps live `STARTBUT.SHP` overlay roles gated by verified source bounds / real preview availability.

Implementation fixture implication:

- Use `Dustbowl.map`/`Dustbowl.mmx` to assert decoded preview appears and live overlay is skipped.
- Use one of the 9 header `.yro` maps to assert live marker/label eligibility once `[Header]` source bounds are wired.
- Do not synthesize live overlays from gameplay `[Waypoints]` on the 45 non-header maps; their start markers are already baked into `[PreviewPack]`.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Local retail root map census | verified | 54 files in local RA2/YR install; parsed sections and decoded PreviewPack payloads | nested archive-only map universe not enumerated |
| `[PreviewPack]` presence/decode | verified | all 54 maps; `0x00641B00`; RGB byte counts equal `w*h*3` | runtime screenshot comparison not done |
| Baked red marker pixels | verified | all 54 maps have exact connected `4x4` RGB `(240,0,0)` components | edge-clipped generated-marker cases not covered |
| `[Header] NumberStartingPoints` presence | verified | 9 maps have eligible counts; 45 do not have `[Header]` | custom/user map coverage not included |
| Live overlay eligibility | verified for gate and data | `0x00640710`; 9 maps satisfy `1..8` | exact label pixel comparison and STARTBUT clipping remain sibling-report scope |
| Header WaypointN overprovisioning | verified | all 9 header maps have `Waypoint1..Waypoint8`; count controls live loop | why saved `.yro` carries unused extra keys when count < 8 is not investigated |

## 9. Open Questions - Final State

[RESOLVED] OQ-1 - Do stock root maps generally have `[PreviewPack]`? Yes for this local stock root set: 54/54 have non-empty `[PreviewPack]`, and all decode to `width * height * 3` RGB payloads. Evidence: local census; `0x00641B00`.

[RESOLVED] OQ-2 - Which stock root maps exercise live `STARTBUT.SHP` overlay eligibility? Exactly 9 local `.yro` files listed in section 4; each has `[Header] NumberStartingPoints` in `1..8`. Evidence: local census; `0x00640710`.

[RESOLVED] OQ-3 - Do loose `Dustbowl.map` and `Dustbowl.mmx` draw live overlays? No. Both have `[PreviewPack]` and baked red pixels but no `[Header]`, leaving `NumberStartingPoints = -1` on the active helper path. Evidence: local census; `0x00689D30`; `0x00640710`.

[RESOLVED] OQ-4 - Are baked red preview-marker pixels separate from live overlays in stock files? Yes. All 54 decoded previews contain exact `4x4` RGB `(240,0,0)` components, while only 9 maps are live-overlay eligible. Evidence: decoded retail payloads; generated-marker constants from `0x00641140`.

[DEFERRED] OQ-5 - Are there additional stock maps hidden only inside nested MIX archives that the root-file census misses? Category: out-of-scope. Reason: this slot is constrained to standard local retail offline Skirmish stock map files present in the install root. Next step: archive-level scenario list audit if needed.

## Sources

- Ghidra decompile: `DrawStartPositions @ 0x00640710`.
- Ghidra decompile: selected PreviewPack load `0x00641B00`.
- Ghidra decompile: `[Header]` preview metadata helper `FUN_00689D30`.
- Ghidra decompile: PreviewPack writer `0x006418B0`.
- Prior docs: `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`, `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md`, `SCENARIO_PREVIEW_HEADER_DEFAULTS_AND_DUSTBOWL_SOURCE_PATH_GHIDRA_REPORT.md`.
- Local retail data: `C:/Users/enok/Documents/Command and Conquer Red Alert II/*.mmx`, `*.yro`, `Dustbowl.map`.
- Current Rust reference: `src/map/preview.rs`, `src/app_list_maps.rs`, `src/app_skirmish_shell_render.rs`.
