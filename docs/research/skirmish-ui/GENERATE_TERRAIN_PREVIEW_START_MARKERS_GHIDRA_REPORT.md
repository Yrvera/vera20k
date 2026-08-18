# GenerateTerrainPreview Start Markers - Ghidra Research Report

**Date:** 2026-05-21  
**Address(es):** `0x00641140` primary; helpers `0x0068BD80`, `0x0068BCC0`, `0x0068BDC0`, `0x006D62E0`, `0x006418B0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** baked start-marker pixels inside `GenerateTerrainPreview @ 0x00641140`: waypoint index range, marker size, marker color, coordinate rounding/scaling, and whether baked markers are expected in stock `[PreviewPack]`.  
**Non-Scope:** `STARTBUT.SHP` overlay markers, numeric labels, full preview terrain color generation, generic DSurface clipping internals, and full `[PreviewPack]` decode lifecycle.  
**Confidence:** High for marker loop, coordinates, size, color, and generated/saved PreviewPack storage; High for the Dustbowl retail sample containing baked marker pixels; Medium for generalizing that every stock map with valid starts contains them, because only one stock map file was sampled here.  
**Active in YR:** Yes / Conditional. The marker-writing code is active in YR random map preview generation and generated/saved preview storage; stock map `[PreviewPack]` data can already contain these baked pixels, independently of live `STARTBUT.SHP` overlays.

## 1. Overview

`GenerateTerrainPreview @ 0x00641140` paints terrain into a generated preview surface and then paints baked start-position markers directly into that surface. These markers are solid red `4x4` pixel rectangles derived from gameplay waypoint indices `0..7`; they are not `STARTBUT.SHP` overlay sprites.

Active in YR: Yes. Evidence: marker pass is inside `GenerateTerrainPreview @ 0x00641140`, and YR callers include random map generation/update paths (`0x00596300`, `0x00598960`) and generated/saved preview storage (`0x006418B0`, `0x00687CE0`).

## 2. Key Offsets / Helpers

| Item | Use | Evidence | Active in YR |
| --- | --- | --- | --- |
| `g_ScenarioClass_Instance` / `0x00A8B230` | Scenario object passed to waypoint helpers | call setup at `0x0064177B` | Yes |
| `ScenarioClass+0x632 + index*4` | packed waypoint table, low/high 16-bit cell coordinates | `0x0068BD80`, `0x0068BCC0`, `0x0068BDC0` | Yes |
| `DAT_00B05458` / `DAT_00B0545A` | invalid waypoint sentinel halves | `0x0068BD80`, `0x0068BDC0` | Yes |
| DD pixel-format globals `0x008A0DD0..0x008A0DE4` | pack marker RGB into active display format | `0x00641825..0x00641868` | Yes |
| DSurface vtable `+0x78`, `+0x10` | rectangle/color setup and draw/fill call boundary | `0x00641874`, `0x0064187A` | Yes |

## 3. Core Logic

### Waypoint Range

The marker pass initializes the marker index to `0`, increments by `1`, and loops while the signed comparison `index < 8` holds. It tests exactly waypoint indices `0,1,2,3,4,5,6,7`.

Active in YR: Yes. Evidence: `0x00641770` zeroes `ESI`, `0x00641886` increments it, `0x00641887` compares it with `8`, and `0x0064188E` jumps back while less.

Invalid entries do not terminate the scan. The invalid branch skips the current marker draw and resumes at the loop increment.

Active in YR: Yes. Evidence: `0x00641782` calls `0x0068BD80`, `0x00641787..0x00641789` skip to `0x00641886` only for the current index.

### Valid-Waypoint Predicate

`0x0068BD80` returns valid only when `0 <= index < 0x2BE` and the waypoint table entry at `ScenarioClass+0x632+index*4` is not the sentinel pair `DAT_00B05458/DAT_00B0545A`. In this function, the outer loop limits index to `0..7`, so the `0x2BE` bound is a generic helper guard, not the marker range.

Active in YR: Yes. Evidence: helper decompile `0x0068BD80`; call site `0x0064177B..0x00641782`.

`0x0068BCC0` copies the packed waypoint value from `ScenarioClass+0x632+index*4`. `0x0068BDC0` populates the table from `[Waypoints]`: value `0` becomes the sentinel, otherwise low 16 bits are `value % 1000` and high 16 bits are the divided coordinate.

Active in YR: Yes. Evidence: helper decompiles `0x0068BCC0` and `0x0068BDC0`.

### Coordinate Scaling / Rounding

For each valid waypoint, the marker uses cell-center leptons:

```text
lepton_x = cell_x * 0x100 + 0x80
lepton_y = cell_y * 0x100 + 0x80
```

It calls `0x006D62E0`, which applies the RA2 isometric preview transform:

```text
raw_x = (lepton_x * 0x3C) / 2 + (lepton_y * -0x3C) / 2
raw_y = (lepton_x * 0x1E) / 2 + (lepton_y *  0x1E) / 2
proj_x = trunc_toward_zero(raw_x / 0x100) + 0x3C00
proj_y = trunc_toward_zero(raw_y / 0x100)
```

The sign-bias before shifting (`value + ((value >> 31) & 0xFF)`) implements truncation toward zero for negative intermediate values, not floor.

Active in YR: Yes. Evidence: `0x006D62E0`.

The marker pass then divides `proj_x` by `0x3C` and `proj_y` by `0x1E`, using compiler reciprocal-multiply sequences for signed division. The rectangle top-left is:

```text
marker_x = ((proj_x / 0x3C) - preview_min_x) * 2 - 1
marker_y =  (proj_y / 0x1E) - preview_min_y - 1
```

`preview_min_x` and `preview_min_y` are from the first playable-cell bounds pass in the same function.

Active in YR: Yes. Evidence: `0x006417D1` calls `0x006D62E0`; `0x006417D6..0x00641821` performs the signed divisions and `-1` placement bias; bounds are established earlier in `0x00641140`.

### Marker Size

The baked rectangle is `4x4` pixels before any generic surface clipping.

Active in YR: Yes. Evidence: `0x00641772` loads `EBP=4`; `0x006417EC` stores width `4`; `0x0064182F` stores height `4`.

### Marker Color

The baked marker color input is RGB `(0xF0, 0x00, 0x00)` packed through the current DirectDraw loss/shift globals. It is a direct packed-color fill, not a palette-index write.

Active in YR: Yes. Evidence: `0x00641825` clears the blue component path, `0x00641833` clears the green component path, `0x00641853` loads `0xF0` for red, and `0x00641868` pushes the packed color into the surface call path.

### PreviewPack Storage

`0x006418B0` writes `[Preview]` and `[PreviewPack]`. If the surface pointer is null, it calls `GenerateTerrainPreview` first, so the generated preview surface stored into `[PreviewPack]` includes any baked start-marker rectangles that the marker loop painted.

Active in YR: Conditional. Evidence: `0x006418B0` null-surface branch calls `GenerateTerrainPreview`; `0x00687CE0` map/generation write path calls `GenerateTerrainPreview` then `0x006418B0`.

## 4. Stock PreviewPack Check

Retail sample `C:/Users/enok/Documents/Command and Conquer Red Alert II/Dustbowl.map` contains `[Preview] Size=0,0,138,75`, `[PreviewPack]`, and `[Waypoints] 0=116070`, `1=34079`. Decoding the stock `[PreviewPack]` as base64-over-LZO chunks produced `31,050` bytes, exactly `138 * 75 * 3`.

The decoded RGB payload contains exactly two solid `4x4` red components with bytes `(240,0,0)`: one at preview pixels `x=113..116, y=20..23`, and one at `x=23..26, y=56..59`. No BGR-interpreted red components were found for this sample.

Active in YR: Yes for this stock map data. Evidence: retail file `Dustbowl.map` `[Preview]`/`[PreviewPack]`/`[Waypoints]`; decoded pixel payload matches the binary marker color and `4x4` size from `0x00641140`. This data check does not prove every stock map has markers, only that baked markers are expected/present in at least this standard bundled map's `[PreviewPack]`.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
| --- | --- | --- | --- |
| `GenerateTerrainPreview @ 0x00641140` marker loop | verified | `0x00641770..0x0064188E` | none for scoped marker constants |
| Waypoint validity | verified | `0x0068BD80` | none |
| Waypoint copy/source table | verified | `0x0068BCC0`, `0x0068BDC0` | none for marker source format |
| Projection helper | verified | `0x006D62E0` | none |
| Marker surface calls | touched-not-exhausted | `0x00641874`, `0x0064187A` | generic clipping behavior if marker straddles surface edge |
| Generated/saved PreviewPack storage | verified | `0x006418B0`, `0x00687CE0` | none for marker-before-storage ordering |
| Random map UI/generation callers | verified | `0x00596300`, `0x00598960` | none for activity |
| Stock Dustbowl PreviewPack marker pixels | verified sample | `Dustbowl.map` decoded payload | broader stock-map census |
| `STARTBUT.SHP` overlays / numeric labels | deferred | out-of-scope | separate overlay investigations |

## 6. Open Questions - Final State

[RESOLVED] OQ-1 - Which waypoint indices are baked? Exactly `0..7`; invalid entries are skipped without ending the loop. Evidence: `0x00641770..0x0064188E`.

[RESOLVED] OQ-2 - What size and color are baked? `4x4`, packed direct-color red from RGB `(0xF0,0,0)`. Evidence: `0x00641772`, `0x006417EC`, `0x0064182F`, `0x00641825..0x00641868`.

[RESOLVED] OQ-3 - What coordinate rounding/scaling is used? Cell centers, `0x006D62E0` isometric transform with truncation toward zero, then signed `/0x3C` and `/0x1E`, X doubled, and `-1,-1` top-left bias. Evidence: `0x006D62E0`, `0x006417D1..0x00641821`.

[RESOLVED] OQ-4 - Are baked markers expected in stock `[PreviewPack]`? Yes for at least retail `Dustbowl.map`: decoded stock payload has two exact `4x4` `(240,0,0)` rectangles matching the binary marker. Evidence: `Dustbowl.map` `[Preview] Size=0,0,138,75`, `[Waypoints] 0=116070/1=34079`, decoded red components at `113..116,20..23` and `23..26,56..59`.

[DEFERRED] OQ-5 - Exact clipping footprint for markers partially outside a generated surface. Category: out-of-scope. Reason: requires draining generic DSurface vtable methods, while this slot is limited to marker constants and storage expectations.

[DEFERRED] OQ-6 - Whether every bundled stock map with valid starts contains baked marker pixels. Category: out-of-scope. Reason: would require a stock-map census; this slot sampled Dustbowl only.

## Sources

- Ghidra decompile/assembly context: `0x00641140`, `0x00641770..0x0064188E`, `0x0068BD80`, `0x0068BCC0`, `0x0068BDC0`, `0x006D62E0`, `0x006418B0`, `0x00687CE0`, `0x00596300`, `0x00598960`.
- Prior related report checked for duplication/context: `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_GENERATE_TERRAIN_PREVIEW_BAKED_START_MARKERS_GHIDRA_REPORT.md`.
- Retail sample: `C:/Users/enok/Documents/Command and Conquer Red Alert II/Dustbowl.map`.
