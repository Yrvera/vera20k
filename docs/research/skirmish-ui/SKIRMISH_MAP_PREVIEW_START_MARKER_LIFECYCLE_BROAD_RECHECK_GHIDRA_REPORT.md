# Skirmish Map Preview / Start Marker Lifecycle Broad Recheck - Ghidra Research Report

**Date:** 2026-05-23  
**Target:** `SKIRMISH_MAP_PREVIEW_START_MARKER_LIFECYCLE_BROAD_RECHECK`  
**Address(es):** `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x006067A0`, `0x00640710`, `0x00641B00`, `0x00641DB0`, `0x00641140`, `0x006418B0`, `0x00689D30`, `0x0069ADF0`, `0x00640A40`, `0x00598960`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** Reconciles selected-map preview object lifecycle and start-marker rendering across stock selected maps, Choose Map return, random-map preview products, missing/invalid preview fallback, `STARTBUT.SHP` live marker placement/clipping/labels, `mmpb.shp` separation, and current Rust backlog.  
**Non-Scope:** Full random-map terrain generation formulas, full `.SED` serialization, full Choose Map layout/row paint, online/network preview paths, and tactical-map start placement after launch.  
**Confidence:** High for the lifecycle contracts and marker source/placement. Medium for random-map exact per-pixel terrain colors because this report relies on the existing terrain-preview report rather than draining all color callees again.  
**Active in YR:** Yes / Conditional. Standard offline Skirmish stock preview and paint paths are active in YR; random-map `RandMap.img` paths are active when selected/accepted record is `RandMap.Sed` or the random-map dialog has generated a preview.

## 0. Working Notes Gate

Target question: What is the implementable lifecycle contract for selected-map preview refresh and start marker rendering across stock maps and random maps, including `PreviewPack`, `[Header]` source bounds, `DAT_00AC1154`, `RandMap.img`, Choose Map invalidation/refresh, live `STARTBUT` markers, `MMPB`, and current Rust deltas?

Non-goals: Do not re-investigate full RMG generation, start placement after launch, all Choose Map UI pixels, or online preview transfer. Do not patch Rust.

Evidence needed to mark COMPLETE:

- Decompile plus assembly/function-context evidence for active parent paint/init/command paths.
- Decompile evidence for `[Preview]` / `[PreviewPack]` decode and `[Header]` source-bound reader.
- Decompile plus string/caller evidence for `RandMap.img` loader and random-map preview products.
- Decompile plus caller evidence for live `STARTBUT` marker projection, clipping, label placement, and `mmpb.shp` separation.
- Current Rust scan naming existing surfaces and backlog.

Stop conditions: Stop once the stock-selection, Choose Map return, random-map preview image, failed-preview fallback, and live marker handoff are reconciled. Defer exact random terrain pixels, `.SED` layout, and online/network preview paths.

## 1. Overview

The offline Skirmish preview is a stateful preview-wrapper lifecycle, not a stateless render of the currently hovered map row. `DAT_00AC1154` points to a 4-byte wrapper whose first field is the current drawable preview surface. Init and committed selection paths replace that wrapper/surface, then invalidate/repaint; `WM_PAINT` consumes the current wrapper through `DrawStartPositions`.

Stock maps and random maps use different image sources. Normal maps load `[Preview]` / `[PreviewPack]` from the selected map and optionally read `[Header]` preview source bounds for live markers. Random-map previews load `RandMap.img`, a runtime PCX-style image written from the generated preview surface. Passive Choose Map row highlighting does not refresh either source. Active in YR: Yes for stock setup/paint, Conditional for random-map sentinel. Evidence: `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x00641B00`, `0x00641DB0`.

## 2. Class Layout / Key Offsets

| Item | Verified purpose | Active in YR | Evidence |
|---|---|---:|---|
| `DAT_00AC1154` | Offline setup / chooser preview wrapper pointer. Null skips preview paint; non-null wrapper can still have null inner surface. | Yes | `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0` |
| wrapper `+0x00` | Inner preview surface pointer. `DrawStartPositions` early-outs if null. | Yes | `0x00640710`, constructor/destructor prior docs |
| child control `0x468` | Preview anchor/static converted to client/backbuffer coordinates before aspect fit. | Yes | `0x006AE3F0`, `0x00640710` |
| `ScenarioClass+0x112C` | `[Header] StartX`, live marker source origin X. Defaults `-1`. | Conditional | `0x00689D30`, `0x00640710` |
| `ScenarioClass+0x1130` | `[Header] StartY`, live marker source origin Y. Defaults `-1`. | Conditional | `0x00689D30`, `0x00640710` |
| `ScenarioClass+0x1134` | `[Header] Width`, live marker X divisor. Defaults `-1`. | Conditional | `0x00689D30`, `0x00640710` |
| `ScenarioClass+0x1138` | `[Header] Height`, live marker Y divisor. Defaults `-1`. | Conditional | `0x00689D30`, `0x00640710` |
| `ScenarioClass+0x113C` | `[Header] NumberStartingPoints`; live overlays draw only for `1..8`. Defaults `-1`. | Conditional | `0x00689D30`, `0x00640710` |
| `ScenarioClass+0x1140 + i*8` | `[Header] Waypoint1..N` X/Y pairs, one-based keys. | Conditional | `0x00689D30`, `0x00640710` |
| `DAT_00AC4E80` | Cached `STARTBUT.SHP` pointer used for available-start marker sprites. | Conditional | string anchor `STARTBUT.SHP @ 0x00836DE4`, `0x00640710` |
| `mmpb.shp @ 0x00836DF4` | Assigned-player marker asset used by a separate path, not standard offline `STARTBUT` marker loop. | Conditional, separate path | string anchor to `0x00640A40` |
| `RandMap.img @ 0x00829ABC` | Runtime random-map preview image loaded by random sentinel/setup paths. | Conditional | string anchors to `0x006AE6E0`, `0x006ACEE0`, `0x00641DB0` |
| `RandMap.Sed @ 0x0082BC30` | Selected-record filename sentinel checked by `0x0069ADF0`. | Conditional | `0x0069ADF0` |

## 3. Core Logic

### 3.1 Normal stock selected-map preview source

Active in YR: Yes. The normal selected-map path reads `[Preview]` and `[PreviewPack]`. `0x00641B00` clears any previous inner surface, reads `[Preview]` size, allocates a `DSurface`, reads `[PreviewPack]`, LZO-decompresses it, and writes exactly one 3-byte RGB triple per preview pixel. Short reads clean up and return failure.

Material details:

- `[Preview]` dimensions drive source surface dimensions, not child `0x468`.
- The compressed payload must decompress to `width * height * 3` bytes.
- RGB order is the settled PreviewPack order from prior channel-order reports; this recheck confirmed the same function and did not reopen the channel-order debate.
- The stock root census found all 54 checked retail root maps have non-empty `[PreviewPack]` and decode to expected byte counts.

Evidence: decompile `0x00641B00`; assembly context at `0x00641B00` shows active function start; string anchor `PreviewPack @ 0x00836DD0` reaches this function. Active in YR: Yes, through offline Skirmish selected-map preview loaders.

### 3.2 `[Header]` live marker metadata source

Active in YR: Conditional on selected map having valid `[Header]` fields. `0x00689D30` initializes preview metadata before reading:

- `StartX`, `StartY`, `Width`, `Height`, and `NumberStartingPoints` are set to `-1`.
- Eight `WaypointN` X/Y pairs at `+0x1140` are zeroed.
- It reads `[Header] StartX`, `StartY`, `Width`, `Height`, `NumberStartingPoints`, `NumCoopHumanStartSpots`, then loops `i=1..NumberStartingPoints` and reads one-based `Waypoint%d`.

Evidence: decompile `0x00689D30`; assembly context at `0x00689D30`; string anchor `NumberStartingPoints @ 0x0083DE48` xrefs `0x00689D30` and scenario basic reader. Active in YR: Yes when map INI contains these keys; otherwise defaults remain.

Negative source fact: gameplay `[Waypoints]` are not the live `STARTBUT` source for the preview overlay. Stock maps without `[Header]` may still show baked red markers inside `[PreviewPack]`, but they do not get live `STARTBUT.SHP` overlays. Evidence: `0x00689D30` only reads `[Header]`; `0x00640710` reads only `ScenarioClass+0x112C..+0x1144`.

### 3.3 Paint consumer and preview fit

Active in YR: Yes. `0x006AE3F0` is the standard offline Skirmish dialog proc. On `WM_PAINT (0x0F)`, it checks `DAT_00AC1154 != 0`, gets child `0x468`, calls `0x006067A0`, and if that helper returns zero calls `DrawStartPositions @ 0x00640710`.

`DrawStartPositions` then:

1. validates the dialog rect;
2. early-outs unless wrapper `+0` is non-null;
3. gets child `0x468` coordinates via `0x00775690`;
4. asks the preview surface for source dimensions;
5. computes integer aspect fit with a fixed `1000` scale factor and truncating integer divisions;
6. blits the preview image to the destination surface;
7. lazily loads `STARTBUT.SHP`;
8. emits live marker sprites and labels only when `0 < ScenarioClass+0x113C < 9`.

Evidence: decompile `0x006AE3F0` and `0x00640710`; assembly contexts at `0x006AE3F0`, `0x00640710`, and `0x006067A0`. Active in YR: Yes for standard offline Skirmish first/normal paint.

### 3.4 Live `STARTBUT.SHP` marker projection, clipping, and labels

Active in YR: Conditional on valid preview wrapper, `STARTBUT.SHP` availability for the sprite layer, and `[Header]` count `1..8`.

For each live start index:

```text
x_per_mille = trunc((WaypointX - StartX) * 1000 / Width)
y_per_mille = trunc((WaypointY - StartY) * 1000 / Height)
anchor_x = fitted_x + trunc(x_per_mille * fitted_w / 1000)
anchor_y = fitted_y + trunc(y_per_mille * fitted_h / 1000)
STARTBUT top-left = (anchor_x - 9, anchor_y - 6)
label origin       = (anchor_x - 2, anchor_y - 6)
label text         = "1".."8" from loop index + 1
```

Material details:

- No branch rejects anchors outside the fitted preview rectangle.
- Live marker sprites are not clipped to the source preview image or fitted preview rect.
- The marker's submitted footprint is native `STARTBUT.SHP` frame `0`, clipped downstream by the destination surface / `CC_Draw_Shape` clip path.
- If `DAT_00AC4E80 == 0`, the sprite block is skipped, but the numeric label loop still executes after that branch.

Evidence: decompile `0x00640710`; prior clipping report verified downstream `CC_Draw_Shape @ 0x004AED70` and `AlphaShapeClass__ClipRect @ 0x00421B60`; string anchor `STARTBUT.SHP @ 0x00836DE4` reaches `DrawStartPositions`. Active in YR: Conditional.

### 3.5 `mmpb.shp` is separate from standard available-start markers

Active in YR: Conditional, but not on the standard offline `DrawStartPositions` marker loop. The only string anchor for `mmpb.shp @ 0x00836DF4` in this slice reaches `0x00640A40`, which is a separate assigned-player marker / map-preview rendering context. `DrawStartPositions @ 0x00640710` loads `STARTBUT.SHP`, not `mmpb.shp`, and labels are text rather than `number*.pcx`.

Evidence: string anchors `STARTBUT.SHP -> DrawStartPositions`, `mmpb.shp -> 0x00640A40`; decompile `0x00640A40` shows independent preview construction/marker logic. Active in YR: Yes only for that separate context; No for standard available-start marker rendering.

### 3.6 Choose Map browsing versus committed refresh

Active in YR: Yes. The Choose Map `0x6B` modal paints from the current global preview wrapper; passive browsing does not replace it.

Material details from prior modal slice, rechecked against parent command path:

- Parent command `0x5AA` in `0x006ACEE0` hides parent `0x102`, runs Choose Map, then shows parent again.
- The modal `WM_PAINT` draws current `DAT_00AC1154`; map-list `0x553` row highlighting has no normal preview-refresh branch.
- Category list `0x6EB` selection rebuilds/reselects map rows but does not reload preview.
- Use Map `0x6C5` is the normal commit boundary. Parent return path then refreshes the setup preview and invalidates.

Evidence: decompile `0x006ACEE0`; prior read-only disassembly of modal callback `0x005E6920..0x005E7041`; `0x005E7160` commit helper; `0x006ACEE0` replacement/invalidation branches. Active in YR: Yes.

### 3.7 `DAT_00AC1154` lifecycle and fallback

Active in YR: Yes for normal setup/paint; Conditional random branch for `RandMap.Sed`.

Lifecycle contract:

- Init `0x006AE6E0` selects/loads the current committed map preview.
- Choose Map return `0x006ACEE0` refreshes preview after modal exit/commit, not during passive row browse.
- Replacement paths destroy old wrapper/inner surface before installing a new wrapper.
- Paint has two guards: global wrapper non-null in `0x006AE3F0`, then wrapper `+0` non-null in `DrawStartPositions`.
- Start/Back cleanup destroys/frees `DAT_00AC1154` and clears it before leaving the dialog.
- Missing/invalid preview load can leave a wrapper with null inner surface; draw early-outs. Random branches inspect this and can fall back to normal refresh.

Evidence: decompile `0x006AE6E0`, `0x006ACEE0`, `0x006AE3F0`, `0x00640710`; prior lifecycle report for constructor/destructor `0x006406E0` / `0x006406F0`. Active in YR: Yes / Conditional as above.

### 3.8 Random-map preview products

Active in YR: Conditional on random-map dialog/generation or selected `RandMap.Sed`.

`RandMap.img` is not `[PreviewPack]`. The random-map dialog writes it from the generated preview surface; setup/chooser random branches load it through `0x00641DB0`.

Material details:

- `0x0069ADF0` checks the selected record filename at `+0x58` against `RandMap.Sed`.
- `0x00641DB0` constructs a file/temporary `BSurface`, destroys any old wrapper inner surface, checks decoded width and height are nonzero, allocates a destination `DSurface` with the decoded dimensions, copies the temporary surface into wrapper `+0`, and returns `1`.
- If the file is missing/invalid or decoded dimensions are zero, it returns `0`; the wrapper can remain with null inner surface.
- `GenerateTerrainPreview @ 0x00641140` creates dynamic dimensions `(max_projected_x - min_projected_x) * 2` by `(max_projected_y - min_projected_y)` from playable-cell projected bounds.
- Generated preview includes baked `4x4` red markers for valid waypoint indices `0..7` before the image is written.
- `0x00598960` calls `GenerateTerrainPreview` repeatedly only when its preview flag argument is nonzero; this is preview-output behavior, not the full generator contract.

Evidence: decompile `0x0069ADF0`, `0x00641DB0`, `0x00641140`, `0x00598960`; string anchors `RandMap.img`, `RandMap.Sed`; prior `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS` report. Active in YR: Conditional.

## 4. INI Keys / Map Data

No rulesmd/artmd key controls this preview lifecycle. The material sources are map INI sections and runtime random-map image files.

| Source | Key / data | Binary reader | Effect | Active in YR |
|---|---|---|---|---|
| `[Preview]` | `Size=...` | `0x00641B00` via selected-map loader | preview source dimensions | Yes for normal map preview |
| `[PreviewPack]` | numbered base64/LZO payload | `0x00641B00` | stock/custom preview pixels | Yes for normal map preview |
| `[Header]` | `StartX` | `0x00689D30` | live marker projection origin X | Conditional |
| `[Header]` | `StartY` | `0x00689D30` | live marker projection origin Y | Conditional |
| `[Header]` | `Width` | `0x00689D30` | live marker X divisor | Conditional |
| `[Header]` | `Height` | `0x00689D30` | live marker Y divisor | Conditional |
| `[Header]` | `NumberStartingPoints` | `0x00689D30` | live marker count gate, accepted only `1..8` | Conditional |
| `[Header]` | `Waypoint%d`, one-based | `0x00689D30` | live marker X/Y pairs | Conditional |
| `[Waypoints]` | gameplay starts `0..7` | not used by live `STARTBUT` projection | baked PreviewPack/RandMap source may already contain red pixels; no live fallback | No for live overlay |
| runtime file | `RandMap.img` | `0x00641DB0` | random-map preview image | Conditional |
| selected record filename | `RandMap.Sed` | `0x0069ADF0` | random-map sentinel branch | Conditional |

## 5. Integration Points

| Function / path | Role | Active in YR | Evidence |
|---|---|---:|---|
| `0x006AE3F0` | Offline Skirmish dialog proc; `WM_PAINT` preview consumer and `WM_COMMAND` dispatcher. | Yes | decompile; assembly context |
| `0x006AE6E0` | Dialog init; initializes controls and current preview. | Yes | decompile |
| `0x006ACEE0` | Parent command handler; Choose Map return, Start/Back cleanup, random branch fallback. | Yes | decompile |
| `0x006067A0` | Paint suppression/helper gate before `DrawStartPositions`. | Yes | decompile, assembly context |
| `0x00640710` | `DrawStartPositions`; preview blit, `STARTBUT` sprites, labels. | Conditional after preview exists | decompile |
| `0x00641B00` | Normal `[Preview]` / `[PreviewPack]` decode into selected preview surface. | Yes | decompile |
| `0x00689D30` | `[Header]` preview source-bound reader. | Conditional on map header | decompile |
| `0x00641DB0` | `RandMap.img` loader into preview wrapper. | Conditional on random sentinel/generated preview | decompile |
| `0x00641140` | Generated terrain preview surface builder. | Conditional | decompile |
| `0x006418B0` | PreviewPack writer path; supports generated/saved preview distinction. | Conditional | decompile |
| `0x00640A40` | Separate `mmpb.shp` assigned-player marker context. | Conditional, not standard available-start path | decompile, string anchor |
| `0x00598960` | RMG function that calls `GenerateTerrainPreview` when preview flag is nonzero. | Conditional | decompile |

## 6. Current Rust Implementation Status

Scanned Rust surfaces:

- `C:/Users/enok/Documents/ra2-rust-game/src/map/preview.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_list_maps.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/skirmish_scenarios.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/assets/pcx_file.rs`

Current matches observed:

- `[Preview]` / `[PreviewPack]` decode exists and uses RGB triples in `map/preview.rs`.
- Four-field `[Preview] Size=0,0,w,h` parsing exists.
- `[Header]` preview source bounds are parsed from `[Header]` only in `app_list_maps.rs`.
- Live marker count rejects `count <= 0 || count >= 9`, matching `0 < count < 9`.
- Marker projection and aspect fit use integer per-mille math in `app_skirmish_shell_render.rs`.
- `STARTBUT.SHP` and `mmpb.shp` are both loaded as shell assets, but standard marker draw uses `STARTBUT`.
- Live labels use one-based text and `(anchor_x - 2, anchor_y - 6)` origin.
- Preview cache is keyed to committed `selected_map_idx`, so passive Choose Map highlight does not automatically refresh the parent preview.

Current gaps / backlog:

- `Create Random Map` command in `app.rs` is recognized but logs that random-map generation is not implemented.
- `RandMap.img` preview loading/decoding is missing; Rust currently routes preview decode through normal map INI `[PreviewPack]`.
- `assets/pcx_file.rs` is currently a paletted PCX decoder; random-map `RandMap.img` can use the PCX-style 3-plane direct RGB branch from the native writer.
- The random-map sentinel currently has default/empty preview metadata; it cannot yet replace the previous concrete-map thumbnail with runtime generated preview data.
- Missing/invalid random preview fallback should not retain stale previous preview; current cache invalidation covers committed map changes, but the native wrapper/null-inner distinction is not modeled.
- A renderer-level edge/pixel test for destination-surface clipping of partially off-screen `STARTBUT` markers remains useful, even though helper logic no longer rejects outside fitted-preview anchors.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes gate | verified | section 0 | none |
| Stock `[PreviewPack]` load | verified | `0x00641B00`, string anchor `PreviewPack` | runtime screenshot comparison not redone |
| `[Header]` source-bound reader | verified | `0x00689D30`, string anchor `NumberStartingPoints` | custom malformed-map behavior beyond count/range not exhausted |
| Stock root map buckets | verified by prior census | `SKIRMISH_RETAIL_STOCK_MAP_PREVIEW_CENSUS_GHIDRA_REPORT.md` | nested archive-only stock maps not re-enumerated |
| `DAT_00AC1154` init/paint/teardown lifecycle | verified | `0x006AE6E0`, `0x006AE3F0`, `0x006ACEE0`, `0x00640710` | online/network preview paths deferred |
| Choose Map passive browse no-refresh | verified from prior modal slice and parent decompile | `0x006ACEE0`, prior `0x005E6920..0x005E7041` | full modal UI pixels out-of-scope |
| Use Map / parent return refresh boundary | verified | `0x006ACEE0`, prior `0x005E7160` | exact modal state text updates out-of-scope |
| Random sentinel test | verified | `0x0069ADF0`, `RandMap.Sed` anchor | full `.SED` layout out-of-scope |
| `RandMap.img` loader | verified | `0x00641DB0`, `RandMap.img` anchor | runtime corrupt-file screenshot deferred |
| Generated preview dimensions and baked red markers | verified via prior slice and spot decompile | `0x00641140`, `0x00598960` | exact terrain pixel RGB for every seed deferred |
| Live `STARTBUT` marker placement | verified | `0x00640710`, `STARTBUT.SHP` anchor | none for placement |
| Live marker clipping and labels | verified via prior clipping report plus spot decompile | `0x00640710`, `CC_Draw_Shape`, `AlphaShapeClass__ClipRect` | renderer-level Rust pixel test remains |
| `mmpb.shp` separation | verified | string anchor `mmpb.shp -> 0x00640A40` | assigned-player marker UX separate follow-up if needed |
| Current Rust scan | verified | Codegraph context plus `rg`/file reads | implementation not performed |

## 8. Open Questions - Final State Of Investigation Log

- `[RESOLVED] OQ-01 - Mode and scope? -> coverage-map for lifecycle reconciliation across already-investigated slices; no claim of full RMG or full Choose Map UI completion.` (evidence: dispatch target and prior reports)
- `[RESOLVED] OQ-02 - What is the normal stock preview source? -> `[Preview]` dimensions plus `[PreviewPack]` LZO/RGB payload decoded by `0x00641B00`.` (evidence: `0x00641B00`)
- `[RESOLVED] OQ-03 - What source enables live `STARTBUT` overlays? -> `[Header] StartX/StartY/Width/Height/NumberStartingPoints/Waypoint%d`, loaded by `0x00689D30`.` (evidence: `0x00689D30`)
- `[RESOLVED] OQ-04 - What default disables live overlays on maps without `[Header]`? -> `NumberStartingPoints` defaults to `-1`, and `DrawStartPositions` requires `0 < count < 9`.` (evidence: `0x00689D30`, `0x00640710`)
- `[RESOLVED] OQ-05 - Do stock non-header maps use gameplay `[Waypoints]` as live marker fallback? -> No; visible red starts are baked preview pixels, not live `STARTBUT`.` (evidence: `0x00640710`; stock census)
- `[RESOLVED] OQ-06 - What owns current preview state? -> `DAT_00AC1154` wrapper; wrapper `+0` is drawable surface; null global or null inner suppresses drawing.` (evidence: `0x006AE3F0`, `0x00640710`)
- `[RESOLVED] OQ-07 - When does passive Choose Map browsing refresh preview? -> It does not; passive `0x553` row selection has no preview loader/invalidation path.` (evidence: prior `0x005E6920..0x005E7041`, `0x006ACEE0`)
- `[RESOLVED] OQ-08 - What is the normal refresh boundary after Choose Map? -> Use Map commit / parent return refresh in `0x006ACEE0`.` (evidence: `0x006ACEE0`, prior `0x005E7160`)
- `[RESOLVED] OQ-09 - What is random-map preview source? -> Runtime `RandMap.img`, not `[PreviewPack]`, loaded by `0x00641DB0` when selected record is `RandMap.Sed`.` (evidence: `0x0069ADF0`, `0x00641DB0`, `RandMap.img` anchor)
- `[RESOLVED] OQ-10 - What happens if `RandMap.img` is invalid/missing? -> Loader can leave wrapper `+0 == 0`; draw early-outs; parent random branches can fall back to normal refresh/blank behavior instead of retaining old inner surface.` (evidence: `0x00641DB0`, `0x00640710`, `0x006ACEE0`)
- `[RESOLVED] OQ-11 - Are random preview dimensions fixed to UI rects? -> No; generated preview dimensions are dynamic projected-playfield bounds and `RandMap.img` preserves them.` (evidence: `0x00641140`, `0x00641DB0`)
- `[RESOLVED] OQ-12 - Are generated start markers duplicated as live overlays inside `RandMap.img`? -> No; `RandMap.img` contains baked red `4x4` pixels from the generated surface, while live `STARTBUT` overlays are a later separate layer only if `[Header]` metadata is valid.` (evidence: `0x00641140`, `0x00640710`)
- `[RESOLVED] OQ-13 - What is `STARTBUT` top-left? -> `(anchor_x - 9, anchor_y - 6)`.` (evidence: `0x00640710`)
- `[RESOLVED] OQ-14 - What is live label origin and text? -> one-based loop labels at `(anchor_x - 2, anchor_y - 6)`.` (evidence: `0x00640710`)
- `[RESOLVED] OQ-15 - Are live markers clipped to fitted preview? -> No; destination-surface clipping applies downstream.` (evidence: `0x00640710`, prior `CC_Draw_Shape`/`AlphaShapeClass__ClipRect`)
- `[RESOLVED] OQ-16 - Does missing `STARTBUT.SHP` suppress numeric labels? -> No; sprite block is guarded by `DAT_00AC4E80`, but label draw follows outside that guard.` (evidence: `0x00640710`)
- `[RESOLVED] OQ-17 - Is `mmpb.shp` part of standard offline available-start markers? -> No; it anchors to separate `0x00640A40` context, while standard preview markers use `STARTBUT.SHP`.` (evidence: string anchors `STARTBUT`, `mmpb`)
- `[RESOLVED] OQ-18 - What current Rust surfaces matter? -> `map/preview.rs`, `app_list_maps.rs`, `app_skirmish_shell_render.rs`, `app.rs`, `skirmish_scenarios.rs`, `render/skirmish_shell_chrome.rs`, `assets/pcx_file.rs`.` (evidence: Codegraph context and `rg`)
- `[RESOLVED] OQ-19 - Does current Rust already parse `[Header]` preview source bounds? -> Yes, from `[Header]` only, rejecting `count <= 0 || count >= 9`.` (evidence: `src/app_list_maps.rs`)
- `[RESOLVED] OQ-20 - Does current Rust implement `RandMap.img`? -> No observed; `Create Random Map` still logs not implemented, and preview decode is normal map INI based.` (evidence: `src/app.rs`, `src/map/preview.rs`, `rg RandMap.img src`)
- `[DEFERRED] OQ-21 - Exact RGB for every generated random terrain preview pixel.` (category: out-of-scope; reason: belongs to terrain-preview/generator color formula slices; next-step-if-pursued: drain `CellClass__GetRadarPixelColor`, overlay colors, and runtime DD format values)
- `[DEFERRED] OQ-22 - Full `.SED` seed/options layout and generation semantics.` (category: out-of-scope; reason: preview data products only; next-step-if-pursued: use existing `SKIRMISH_RANDMAP_SED_WRITER_00597730_LAYOUT` / RMG reports)
- `[DEFERRED] OQ-23 - Runtime screenshot of intentionally corrupt/missing `RandMap.img`.` (category: needs-runtime-debugger; reason: static null-inner/fallback branches are proven, but no live capture was taken; next-step-if-pursued: corrupt file between dialog shutdown and parent load)
- `[DEFERRED] OQ-24 - Online/network preview transfer paths sharing preview helpers.` (category: out-of-scope; reason: target is offline Skirmish stock/random maps; next-step-if-pursued: separate online-lobby preview investigation)

Deferred entries are deliberate non-scope or runtime-capture items. The implementable offline Skirmish preview lifecycle and live marker contract are resolved for this coverage map.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Normal stock map preview source is `[Preview]` dimensions plus `[PreviewPack]` RGB/LZO payload; all checked stock root maps have valid packs. | `0x00641B00`; stock census | mostly present | `src/map/preview.rs`, `src/app_skirmish_shell_render.rs` | Keep normal map preview decode path and source dimensions separate from UI rect. | `Dustbowl.mmx` decodes and appears as a preview surface. | Do not use child `0x468` as source dimensions. |
| Live preview overlays are `[Header]`-gated only, accepted for `1..8`; missing `[Header]` means no live `STARTBUT` even if gameplay `[Waypoints]` exist. | `0x00689D30`; `0x00640710`; stock census | present / keep covered | `src/app_list_maps.rs::preview_source_bounds_from_verified_source`, `src/app_skirmish_shell_render.rs` | Preserve `[Header]` source-only gate and count range. | `Dustbowl.mmx` shows baked red preview pixels but no live `STARTBUT`/label overlay; `CrctBrd.yro` enables live overlays. | Do not synthesize live markers from `[Waypoints]`, `LocalSize`, or decoded red pixels. |
| Preview refresh after Choose Map occurs at commit/parent return; passive map-list browsing and category changes do not refresh preview. | `0x006ACEE0`; prior `0x005E6920..0x005E7041` | partially present; modal implementation still incomplete | `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Keep highlight state separate from committed `selected_map_idx`; invalidate preview cache only on accept/cancel/random command paths that commit/replace state. | Highlight several modal rows, then cancel: parent preview remains original; Use Map commit changes preview after close. | Do not make the preview "live update" while browsing rows. |
| `DAT_00AC1154` replacement destroys old surface before loading new one; failed random preview load must not retain stale previous preview. | `0x006AE6E0`; `0x006ACEE0`; `0x00641DB0`; `0x00640710` | partial: cache invalidates on committed map change, but null-inner/fallback model missing | `src/app_skirmish_shell_render.rs`, `src/app.rs` | Treat missing/failed preview decode as no drawable preview for that selected source; avoid stale previous thumbnail. | Select or generate random map with missing preview image: old concrete preview is not reused. | Do not equate "same selected index cache exists" with native wrapper success after a failed replacement. |
| Random-map sentinel preview source is runtime `RandMap.img`, not normal map INI `[PreviewPack]`; dimensions are dynamic from `GenerateTerrainPreview`. | `0x0069ADF0`; `0x00641DB0`; `0x00641140`; `RandMap.img` anchor | missing | `src/skirmish_scenarios.rs`, `src/map/preview.rs` or image decoder, `src/assets/pcx_file.rs`, `src/app_skirmish_shell_render.rs` | Add a random-preview image source that preserves decoded dimensions and is separate from normal map preview packs. | Accept Create Random Map, then selected random sentinel displays the generated preview image with its own dimensions. | Do not feed `RandMap.Sed` through normal map preview INI decode; do not hardcode `80x50`, `138x75`, or `0x468` sizes. |
| `RandMap.img` can require PCX-style 3-plane direct RGB decode; current paletted PCX assumptions are insufficient. | prior writer `0x007B05C0`; loader `0x00641DB0`; `GenerateTerrainPreview` report | missing | `src/assets/pcx_file.rs` or dedicated native IMG decoder | Decode the runtime image channel form used by native writer; preserve RGB data as image pixels. | Fixture with a 3-plane direct RGB `RandMap.img` decodes to expected dimensions/pixels. | Do not assume trailing VGA palette or 1-plane indexed pixels for random previews. |
| Generated `RandMap.img` already contains baked `4x4` red start markers from the generated preview surface. | `0x00641140`; prior terrain-preview report | missing random image path | future random preview renderer | Render decoded random preview as source image; do not draw extra baked red rectangles over it. | Generated preview with valid starts shows red baked pixels once. | Do not duplicate baked markers with an extra Rust overlay layer. |
| Live `STARTBUT` marker top-left is `(anchor_x-9, anchor_y-6)`, labels are `(anchor_x-2, anchor_y-6)`, labels are one-based text and are not suppressed by missing `STARTBUT`. | `0x00640710` | mostly present | `src/app_skirmish_shell_render.rs` | Keep offsets and label source; allow label generation independent of sprite asset availability when source bounds exist. | Header map with `STARTBUT.SHP` absent still submits numeric labels; normal case submits sprite plus labels. | Do not use `mmpb.shp`, number PCX files, zero-based labels, or alternate offsets for standard available-start markers. |
| Live markers are clipped by destination/backbuffer surface, not by fitted preview rect or source preview image. | `0x00640710`; prior `CC_Draw_Shape @ 0x004AED70`; `AlphaShapeClass__ClipRect @ 0x00421B60` | helper match observed; renderer pixel edge test still useful | `src/app_skirmish_shell_render.rs`, batch renderer tests | Submit native marker geometry even when partly outside fitted preview; let renderer/backbuffer clipping decide visible pixels. | Anchor near top-left yields native top-left partly outside fitted image but visible destination-overlapping pixels. | Do not pre-clamp or reject markers against preview/source/fitted rect. |
| `mmpb.shp` is not part of the standard offline available-start marker loop. | string anchors `mmpb.shp -> 0x00640A40`, `STARTBUT.SHP -> 0x00640710` | asset loaded, standard path should keep using `STARTBUT` | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render.rs` | Keep `mmpb` for separate assigned-player marker context only. | Standard Skirmish setup preview uses `STARTBUT.SHP` and labels, not `mmpb.shp`. | Do not substitute `mmpb.shp` for available starts. |

## Negative Facts / Do Not Do

- Do not refresh selected-map preview on passive Choose Map row highlight. Active in YR: No. Evidence: modal branch has no `0x553` preview refresh; parent refresh happens after commit.
- Do not synthesize live `STARTBUT` markers from gameplay `[Waypoints]`, `[Map] LocalSize`, or baked red preview pixels. Active in YR: No for this overlay path. Evidence: `0x00689D30`, `0x00640710`.
- Do not decode `RandMap.img` as `[PreviewPack]`. Active in YR: No. Evidence: `0x00641DB0` PCX-style loader versus `0x00641B00` PreviewPack decoder.
- Do not retain the old concrete preview if a random preview load fails. Active in YR: No on replacement paths; old inner surface is destroyed before new load/fallback. Evidence: `0x00641DB0`, `0x006ACEE0`.
- Do not hardcode random preview dimensions. Active in YR: No. Evidence: `0x00641140` dynamic dimensions and `0x00641DB0` dimension-preserving load.
- Do not draw live `STARTBUT` overlays for maps whose `[Header] NumberStartingPoints` is `-1`, `0`, or `>=9`. Active in YR: No. Evidence: `0x00640710`.
- Do not clip live markers to fitted preview/source rect or clamp their top-left to the preview edge. Active in YR: No. Evidence: `0x00640710`, `CC_Draw_Shape` clipping report.
- Do not use `mmpb.shp` for standard available-start markers. Active in YR: No for this path. Evidence: string anchors.
- Do not draw extra baked red random-map start rectangles over decoded `RandMap.img`. Active in YR: No; they are already part of the generated preview image. Evidence: `0x00641140`.

## 10. Stale Docs / Follow-up Docs

No contradiction requiring an immediate patch to existing docs was found. The useful consolidation wording for future implementation docs is:

> Offline Skirmish preview refresh is commit-driven. Normal maps load `[Preview]`/`[PreviewPack]`; random-map sentinel loads runtime `RandMap.img`. Passive Choose Map browsing does not refresh the preview. Live `STARTBUT.SHP` markers are drawn only from valid `[Header]` preview metadata and are clipped by the destination surface, not the fitted preview rect. `mmpb.shp` is a separate assigned-player marker context.

## Sources

- Ghidra read-only decompiled / inspected: `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x006067A0`, `0x00640710`, `0x00640A40`, `0x00641140`, `0x006418B0`, `0x00641B00`, `0x00641DB0`, `0x00689D30`, `0x0069ADF0`, `0x00598960`.
- Ghidra assembly contexts: function starts for `0x006AE3F0`, `0x006ACEE0`, `0x006067A0`, `0x00640710`, `0x00641140`, `0x006418B0`, `0x00641B00`, `0x00641DB0`, `0x00689D30`, `0x00640A40`.
- Ghidra string anchors: `PreviewPack @ 0x00836DD0`, `STARTBUT.SHP @ 0x00836DE4`, `mmpb.shp @ 0x00836DF4`, `RandMap.img @ 0x00829ABC`, `RandMap.Sed @ 0x0082BC30`, `NumberStartingPoints @ 0x0083DE48`.
- Prior docs reconciled: `SKIRMISH_PREVIEW_DAT_00AC1154_LIFECYCLE_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_IMG_PREVIEW_LOADER_00641DB0_GHIDRA_REPORT.md`, `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_START_MARKER_CLIPPING_FOOTPRINT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`, `SKIRMISH_RETAIL_STOCK_MAP_PREVIEW_CENSUS_GHIDRA_REPORT.md`, `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_MAP_PREVIEW_MARKER_ASSET_LAYOUT_GHIDRA_REPORT.md`.
- Rust scanned: `C:/Users/enok/Documents/ra2-rust-game/src/map/preview.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_list_maps.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/skirmish_scenarios.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/assets/pcx_file.rs`.
