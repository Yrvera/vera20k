# Skirmish GenerateTerrainPreview Baked Start Markers - Ghidra Research Report

Date: 2026-05-20

**Address(es):** `0x00641140` primary, `0x0068BD80`, `0x0068BCC0`, `0x0068BDC0`, `0x006418B0`, `0x00687CE0`, `0x00596300`, `0x00598960`, `0x006D62E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact baked start marker behavior inside `GenerateTerrainPreview @ 0x00641140`: waypoint index range, valid-waypoint predicate, coordinate projection and rounding, marker size, marker color, and whether this path is active for generated/saved `PreviewPack` data.  
**Non-Scope:** `STARTBUT.SHP` overlay placement, preview surface load/decode from existing `[PreviewPack]`, generic surface clipping internals behind vtable calls.  
**Confidence:** High for loop bounds, predicate, projection constants, rect size, color packing, and generation/storage call chain; Medium for the final clipped pixel footprint if a marker straddles a surface edge, because the surface vtable methods were not drained in this slot.  
**Active in YR:** Yes / Conditional. The function is active in YR random map preview generation and in the map-save/generated-preview path. It is not itself the stock map menu `[PreviewPack]` decode path.

## 1. Overview

`GenerateTerrainPreview @ 0x00641140` builds a low-resolution terrain preview surface and, after drawing terrain pixels, paints baked start markers for waypoint indices `0..7`. These are not `STARTBUT.SHP` overlays. They are small solid red rectangles written directly into the generated preview surface before `PreviewPack` storage.

Active in YR: Yes. Evidence: `0x00596300` invokes `GenerateTerrainPreview` from the random map dialog path; `0x00598960` invokes it repeatedly while random map generation progresses; `0x00687CE0` invokes it before preview storage when writing generated/saved map data.

## 2. Key Offsets And Globals

| Field / global | Purpose in this slice | Evidence | Active in YR |
| --- | --- | --- | --- |
| `g_ScenarioClass_Instance` / `0x00A8B230` | Scenario object used for waypoint validation/read helper calls | Calls at `0x0064177B..0x0064179B` pass this pointer to `0x0068BD80` and `0x0068BCC0` | Yes |
| `ScenarioClass+0x632 + index*4` | Packed waypoint cell pair table, two 16-bit coordinates per entry | `0x0068BD80`, `0x0068BCC0`, `0x0068BDC0` | Yes |
| `DAT_00B05458` / `DAT_00B0545A` | Invalid waypoint sentinel halves | `0x0068BD80`, `0x0068BDC0` | Yes |
| `MapClass singleton 0x0087F7E8` | Source of playable cells for preview bounds and terrain pixels | `0x0064114C`, `0x006412A0`, `0x0064175C` call map cell iterator helpers | Yes |
| `g_DD_RLoss/RShift`, `g_DD_GLoss/GShift`, `g_DD_BLoss/BShift` | Packs marker RGB into current display pixel format | `0x0064181A..0x00641868` | Yes |

## 3. Core Marker Logic

### 3.1 Ordering

The baked start marker pass runs after terrain preview pixels are drawn and before the preview surface is unlocked/flushed.

Active in YR: Yes. Evidence: in `0x00641140`, terrain pixel iteration ends before `0x00641770`; waypoint marker loop runs `0x00641770..0x0064188E`; surface unlock is `0x00641894..0x00641898`.

### 3.2 Waypoint Index Range

The marker loop initializes the index to `0`, increments by `1`, and loops while `index < 8`. This tests exactly indices `0,1,2,3,4,5,6,7`.

Active in YR: Yes. Evidence: `0x00641770` zeroes the index, `0x00641886` increments it, and `0x00641887..0x0064188E` compares against `0x8` with a signed `JL` back-edge.

Tiny detail: invalid entries do not terminate the loop. The branch at `0x00641787..0x00641789` skips drawing only the current index; the loop still increments and checks the next index.

### 3.3 Valid-Waypoint Predicate

`FUN_0068BD80 @ 0x0068BD80` returns valid only when:

```text
0 <= index < 0x2BE
and waypoint[index] != sentinel pair DAT_00B05458/DAT_00B0545A
```

For `GenerateTerrainPreview`, the outer loop already restricts the queried index to `0..7`; the helper still contains the larger `0x2BE` guard because it is a generic waypoint-table predicate.

Active in YR: Yes. Evidence: direct helper decompile at `0x0068BD80`; call site `0x0064177B..0x00641782`.

### 3.4 Waypoint Source Format

`FUN_0068BCC0 @ 0x0068BCC0` copies the packed 4-byte waypoint value from `ScenarioClass+0x632+index*4`. `FUN_0068BDC0 @ 0x0068BDC0` fills the same table from `[Waypoints]` INI entries: value `0` becomes the sentinel; otherwise X is `value % 1000` and Y is approximately `value / 1000` in the high 16 bits.

Active in YR: Yes. Evidence: `0x0068BCC0` copy helper and `0x0068BDC0` `[Waypoints]` reader.

### 3.5 Coordinate Projection And Rounding

For a valid waypoint, the baked marker uses cell centers:

```text
lepton_x = cell_x * 0x100 + 0x80
lepton_y = cell_y * 0x100 + 0x80
```

Then it calls `FUN_006D62E0 @ 0x006D62E0`, which computes:

```text
raw_x = (lepton_x * 0x3C) / 2 + (lepton_y * -0x3C) / 2
raw_y = (lepton_x * 0x1E) / 2 + (lepton_y *  0x1E) / 2
proj_x = trunc_toward_zero(raw_x / 0x100) + 0x3C00
proj_y = trunc_toward_zero(raw_y / 0x100)
```

The sign-bias expression before the final shift implements truncation toward zero for negative intermediate values, not mathematical floor.

The marker pass then divides the projected values again using signed integer division by `0x3C` for X and `0x1E` for Y. The compiler emits the `0x88888889` reciprocal-multiply sequence for these divisions at `0x006417D6..0x00641810`.

Final marker rectangle origin:

```text
marker_x = ((proj_x / 0x3C) - preview_min_x) * 2 - 1
marker_y =  (proj_y / 0x1E) - preview_min_y - 1
```

`preview_min_x` and `preview_min_y` are the minima computed during the first playable-cell bounds pass at `0x00641170..0x0064124A`.

Active in YR: Yes. Evidence: `0x006417A0..0x00641821` plus helper `0x006D62E0`; first pass minima at `0x00641226..0x00641240`.

### 3.6 Marker Size

The marker rectangle is exactly `4x4` in preview-surface pixels before clipping. The code writes width `4` and height `4` immediately before the surface draw helper.

Active in YR: Yes. Evidence: `0x00641772` loads `EBP=0x4`; `0x006417EC` stores width `4`; `0x0064182F` stores height `4`.

### 3.7 Marker Color

The baked marker is solid red in the active display pixel format:

```text
B input = 0x00
G input = 0x00
R input = 0xF0
packed = (B >> BLoss) << BShift
       | (G >> GLoss) << GShift
       | (R >> RLoss) << RShift
```

Active in YR: Yes. Evidence: `0x00641825..0x00641868` clears EDX/EBP for blue/green components and loads `0xF0` for the red component before applying the DirectDraw loss/shift globals.

Tiny detail: this is not a palette-index write. It is a direct packed-color write through the preview surface draw helper, using the current DD pixel format.

### 3.8 Surface Call Boundary

After setting `(x,y,w,h,color)`, the code calls the preview surface vtable `+0x78` and then vtable `+0x10`. This slot verifies the rectangle and color parameters, but not the generic clipping semantics inside those surface methods.

Active in YR: Yes. Evidence: `0x00641862..0x0064187A`.

## 4. INI Keys

| INI path | Use in this slice | Evidence | Active in YR |
| --- | --- | --- | --- |
| `[Waypoints] <index>=<yyyxxx>` | Populates `ScenarioClass+0x632` waypoint table consumed by the baked marker loop | `0x0068BDC0` | Yes |
| `[Preview] Size=` | Written for generated/saved preview surface metadata, not read by the baked marker loop | `0x006418B0` writes `[Preview]` before `[PreviewPack]` | Conditional |
| `[PreviewPack]` | Stores the preview surface after terrain and baked marker pixels have been generated | `0x006418B0` | Conditional |

No INI key controls baked marker size, color, index range, or `-1,-1` placement bias in this function.

## 5. Integration Points

`GenerateTerrainPreview @ 0x00641140` is called from:

| Caller | Role | Active in YR |
| --- | --- | --- |
| `0x00687CE0` | Scenario/map INI write path. If writing generated preview data and no preview surface exists, calls `GenerateTerrainPreview`, then `Pipe__Constructor @ 0x006418B0` to write `[Preview]` / `[PreviewPack]`. | Conditional: active when saving/writing generated map preview data |
| `0x006418B0` | Preview storage helper. If `*param_1 == 0`, generates a preview before writing it. | Conditional: active when storage is requested without an existing surface |
| `0x00596300` | Random map dialog proc; random-map commands call `GenerateTerrainPreview` and invalidate/paint the dialog. | Yes for random map UI |
| `0x00598960` | Random map generation pipeline; repeatedly refreshes the generated preview during generation when preview updates are enabled. | Yes for random map generation |

`Pipe__Constructor @ 0x006418B0` writes the generated surface as `[Preview]` and `[PreviewPack]`. Therefore baked red start markers are part of generated/saved `PreviewPack` data when `GenerateTerrainPreview` produced the surface.

Active in stock pre-existing map menu load: No for this function as a decode path. Existing retail maps such as `Dustbowl.map` already carry `[Preview]` and `[PreviewPack]`; menu decode is a separate path.

## 6. Current Rust Implementation Status

Rust currently records preview metadata only:

- `src/map/preview.rs:28-47` parses `[Preview]` and records whether `[PreviewPack]` contains non-empty data.
- `src/map/preview.rs:50-61` currently returns the first two comma fields from `Size=`, so four-field `Size=0,0,138,75` is parsed as `(0,0)`.
- `src/app_skirmish_shell_render.rs:458-460` hard-gates real preview surface availability to false.
- `src/app_skirmish_shell_render.rs:305-312` suppresses `STARTBUT.SHP` overlay markers until real preview decode/source bounds exist.

This slot did not edit Rust. Implementation implications are research-only:

1. Decoding `[PreviewPack]` should display any baked red start markers already present in generated/saved preview data.
2. Drawing `STARTBUT.SHP` overlays is a separate layer and must not be confused with these baked `4x4` red rectangles.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
| --- | --- | --- | --- |
| `GenerateTerrainPreview @ 0x00641140` marker loop | verified | `0x00641770..0x0064188E` | none for loop bounds, size, color, projection inputs |
| Valid waypoint predicate | verified | `0x0068BD80` | none |
| Waypoint copy helper | verified | `0x0068BCC0` | none |
| `[Waypoints]` table population | verified | `0x0068BDC0` | none for source table format |
| Projection helper | verified | `0x006D62E0` | none for arithmetic and rounding inside helper |
| Generated preview storage | verified | `0x006418B0` | channel order of full PreviewPack remains sibling slot scope |
| Map save/generated-preview caller | verified | `0x00687CE0` | none for active generation before storage |
| Random map dialog caller | verified | `0x00596300` | none for active UI invocation |
| Random map generation caller | verified | `0x00598960` | none for repeated preview refresh calls |
| Surface vtable clipping internals | deferred | vtable calls at `0x00641862..0x0064187A` | out-of-scope; only matters for edge-clipped markers |
| Stock `[PreviewPack]` decode path | deferred | parent report / sibling slot scope | out-of-scope; this slot covers generation, not decode |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What waypoint indices are baked into `GenerateTerrainPreview` markers? Answer: exactly `0..7`, invalid entries skipped without terminating. Evidence: `0x00641770..0x0064188E`.

[RESOLVED] OQ-2 - What predicate decides whether a marker is drawn? Answer: `FUN_0068BD80` checks index bounds and sentinel pair at `ScenarioClass+0x632+index*4`. Evidence: `0x0068BD80`.

[RESOLVED] OQ-3 - What projection and rounding are used? Answer: cell center leptons, `FUN_006D62E0` iso projection with signed truncation toward zero, then signed integer divisions by `0x3C`/`0x1E`, then `*2` on X and `-1,-1` top-left bias. Evidence: `0x006D62E0`, `0x006417A0..0x00641821`.

[RESOLVED] OQ-4 - What size is the baked marker? Answer: `4x4` surface rectangle before clipping. Evidence: `0x00641772`, `0x006417EC`, `0x0064182F`.

[RESOLVED] OQ-5 - What color is the baked marker? Answer: packed direct-color red with RGB input `(0xF0,0x00,0x00)` through DD loss/shift globals. Evidence: `0x00641825..0x00641868`.

[RESOLVED] OQ-6 - Is this path active for generated/saved `PreviewPack` data? Answer: yes/conditional. Generated surfaces call `GenerateTerrainPreview` before `Pipe__Constructor @ 0x006418B0` writes `[PreviewPack]`; random map generation also refreshes this surface. Evidence: `0x00687CE0`, `0x006418B0`, `0x00596300`, `0x00598960`.

[DEFERRED] OQ-7 - What exact clipped footprint appears if a `4x4` marker straddles the preview-surface boundary? Category: out-of-scope. Reason: requires draining generic surface vtable `+0x78/+0x10`, while this slot was scoped to marker constants and generation activity.

[DEFERRED] OQ-8 - How does the stock Skirmish menu decode existing `[PreviewPack]` data? Category: out-of-scope. Reason: sibling slot covers PreviewPack decode; this report only verifies generated/saved data.

## Sources

- Ghidra decompile: `0x00641140`, `0x0068BD80`, `0x0068BCC0`, `0x0068BDC0`, `0x006418B0`, `0x00687CE0`, `0x00596300`, `0x00598960`, `0x006D62E0`
- Ghidra disassembly: `0x00641140`
- Parent report: `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_MAP_PREVIEW_SOURCE_BOUNDS_AND_PREVIEWPACK_GHIDRA_REPORT.md`
- Retail sample check: `C:/Users/enok/Documents/Command and Conquer Red Alert II/Dustbowl.map` contains `[Preview]`, `[PreviewPack]`, and `[Waypoints]`
- Rust status check: `C:/Users/enok/Documents/ra2-rust-game/src/map/preview.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`
