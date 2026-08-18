# Skirmish Preview STARTBUT Overlay Rects - Ghidra Research Report

**Address(es):** `DrawStartPositions @ 0x00640710`, caller `FUN_006ae3f0 @ 0x006AE3F0`, preview suppress gate `FUN_006067a0 @ 0x006067A0`, right-anchor helper `FUN_0060b1d0 @ 0x0060B1D0`, `[Header]` reset/read helper `FUN_00689d30 @ 0x00689D30`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Offline Skirmish setup preview child `0x468` final rect, fitted preview image rect math, live `STARTBUT.SHP` overlay marker/label projection, source gates, draw order, and clipping boundary.
**Non-Scope:** `[PreviewPack]` RGB decode internals, generated-preview baked red marker internals, full `FUN_004A61C0` font engine behavior outside this caller, Choose Map modal `0x6B` preview behavior, and generic shell rect table for non-preview controls.
**Confidence:** High for binary-verified formulas/gates/order; Medium for final numeric label glyph raster dimensions because the broad font helper internals remain outside scope.
**Active in YR:** Conditional. The preview surface draw is active for standard offline Skirmish when `DAT_00AC1154` has a preview object and child `0x468` is not suppressing paint. Live `STARTBUT.SHP` overlays and numeric labels are active only when `[Header]`-derived `ScenarioClass+0x113C` is in `1..8`.

## 0. Working Notes Required By Dispatch

Target question: What exact `0x468` preview rect, fitted image rect, live `STARTBUT.SHP` sprite rect, numeric label placement/color, draw order, clipping boundary, and no-overlay gates does standard offline YR Skirmish use?

Non-goals: Do not re-investigate preview RGB decode, generated baked red marker pixels, full dropdown/text table, or modal `0x6B` preview behavior.

Evidence needed to mark COMPLETE: decompile plus assembly context for `DrawStartPositions`; caller evidence from `FUN_006ae3f0`; `0x468` right-anchor helper evidence; `[Header]` reset/read evidence; current Rust scan of `project_preview_start_positions`, `build_start_marker_instances`, `build_start_marker_label_instances`, and `PreviewSourceBounds`.

Stop conditions: Stop once the scoped rect/formula/gate/order/clip facts are verified and Rust-facing deltas are named; list broader source variants or font internals as Remaining Uncertainty instead of expanding.

## 1. Overview

The Skirmish preview child `0x468` is a coordinate anchor, not the owner of preview painting. The parent dialog `WM_PAINT` path calls common shell paint first, then `DrawStartPositions`, which validates the parent, converts child `0x468` to backbuffer coordinates, aspect-fits the preview surface, blits preview pixels, and conditionally draws live `STARTBUT.SHP` frame `0` plus 1-based numeric labels.

Active in YR: Yes for standard offline Skirmish preview surface paint; live marker overlays are conditional. Evidence: `FUN_006ae3f0` decompile checks `WM_PAINT`, `DAT_00AC1154`, child `0x468`, `FUN_006067a0`, then calls `DrawStartPositions`; assembly call boundary previously verified at `0x006AE47B`.

## 2. Rects, Fields, And Source Data

| Item | Verified role | Evidence | Active in YR |
|---|---|---|---|
| Child control `0x468` | Preview anchor/static placeholder; parent-drawn | `FUN_006ae3f0`; `DrawStartPositions` calls `GetDlgItem(...,0x468)` | Yes |
| Child final size | `144x112` pixels from resource/DLU conversion; no `0x0060B950` fixup for `0x468` | current resource-derived Rust base `dlu_rect(429,23,96,69)`; `FUN_0060b950` scoped cases omit `0x468` | Yes |
| Child right-anchor inset | `(168 - 144) / 2 = 12` from right-anchor helper fallback | `FUN_0060b1d0` uses `(g_SIDEBAR_WIDTH_CONST - nWidth)/2` when no per-record inset | Yes |
| Child final rect | `x = screen_w - max((screen_w-800)/2,0) - 144 - 12`; `y = 37 + max((screen_h-600)/2,0)`; `w=144`, `h=112` | `FUN_0060b1d0` plus `FUN_00775690` conversion; Rust scan confirms current table | Yes |
| `ScenarioClass+0x112C/+0x1130` | Preview source origin `StartX/StartY` | `FUN_00689d30`; `DrawStartPositions` reads before projection | Conditional on `[Header]` |
| `ScenarioClass+0x1134/+0x1138` | Preview source width/height divisors | `FUN_00689d30`; `DrawStartPositions` `IDIV` at `0x00640917`, `0x00640953` | Conditional |
| `ScenarioClass+0x113C` | Overlay count gate | `FUN_00689d30`; `DrawStartPositions` guard `0x006408D4..0x006408EF` | Conditional |
| `ScenarioClass+0x1140/+0x1144 + i*8` | `[Header] WaypointN` X/Y pair | `FUN_00689d30` loop; `DrawStartPositions` loop `0x006408F5..0x00640953` | Conditional |

Concrete child rects for standard tested resolutions:

| Resolution | Final child `0x468` rect | Evidence | Active in YR |
|---|---:|---|---|
| `640x480` | `(484,37,144,112)` | formula above; `FUN_0060b1d0` | Yes |
| `800x600` | `(644,37,144,112)` | formula above; `FUN_0060b1d0` | Yes |
| `1024x768` | `(756,121,144,112)` | formula above; center offsets `(112,84)` | Yes in high-res shell |

## 3. Core Logic And Formulas

### 3.1 Parent paint entry and draw order

Active in YR: Yes. `FUN_006ae3f0` delegates to `FUN_00622b50` before its own `WM_PAINT` branch. Only after the common handler returns zero does it draw preview/start positions. Evidence: decompile `FUN_006ae3f0`; prior assembly context at `0x006AE40A` for common paint call and `0x006AE47B` for `DrawStartPositions`.

Order inside `DrawStartPositions`:

1. `ValidateRect(parent, NULL)`.
2. If preview object pointer is null, return.
3. Convert child `0x468` window rect to game client/backbuffer coordinates via `FUN_00775690`.
4. Query preview surface bounds through preview vtable `+0x78`.
5. Compute aspect-fitted destination rect with integer per-mille math.
6. Blit preview surface to `DAT_00887310`.
7. Lazy-load `STARTBUT.SHP` if needed.
8. Resolve `"Yellow"` color scheme through `FUN_0068ca50`.
9. If `0 < NumberStartingPoints < 9`, draw `STARTBUT.SHP` frame `0` for each point, then draw the 1-based numeric label.

### 3.2 Fitted preview image rect

Active in YR: Yes when preview surface exists. Evidence: `DrawStartPositions` decompile plus assembly `0x00640799..0x0064085D`.

The binary uses integer truncation, not float rounding:

```text
scale_x = trunc(child_w * 1000 / src_w)
scale_y = trunc(child_h * 1000 / src_h)
scale = min(scale_x, scale_y)

fit_w = trunc(src_w * scale / 1000)
fit_h = trunc(src_h * scale / 1000)
fit_x = child_x + trunc(child_w / 2) - trunc(src_w * scale / 2000)
fit_y = child_y + trunc(child_h / 2) - trunc(src_h * scale / 2000)
```

Assembly evidence: `child_w * 1000 / src_w` at `0x00640799..0x006407A8`, `child_h * 1000 / src_h` at `0x006407AC..0x006407BA`, min at `0x006407C0..0x006407C6`, half-scaled centering via signed magic division by `2000` at `0x006407CD..0x00640822`, fitted size via signed magic division by `1000` at `0x00640826..0x00640856`.

Example for Dustbowl preview pixels `138x75` inside child `144x112`: `scale=1043`, `fit_w=143`, `fit_h=78`, `fit_x=child_x+1`, `fit_y=child_y+17`. Current Rust's float/round path produces `fit_w=144`, `fit_x=child_x`, which is one pixel too wide/left for this sample.

### 3.3 Marker projection and STARTBUT rect

Active in YR: Conditional on `0 < ScenarioClass+0x113C < 9`. Evidence: `DrawStartPositions` guard at `0x006408D4..0x006408EF`; loop reads at `0x006408F5` and `0x0064093B`; divides at `0x00640917`, `0x00640953`.

For each `i` from `0` to `count-1`:

```text
x_per_mille = trunc((WaypointX[i] - StartX) * 1000 / Width)
y_per_mille = trunc((WaypointY[i] - StartY) * 1000 / Height)
anchor_x = fit_x + trunc(x_per_mille * fit_w / 1000)
anchor_y = fit_y + trunc(y_per_mille * fit_h / 1000)
startbut_top_left = (anchor_x - 9, anchor_y - 6)
```

`STARTBUT.SHP` is loaded from string `STARTBUT.SHP @ 0x00836DE4` once under `DAT_00AC4E90 & 1`, stored in `DAT_00AC4E80`, and drawn as frame `0`. Evidence: loader/string context `0x0064088A..0x006408B2`, string memory `0x00836DE4`, sprite draw call `CC_Draw_Shape @ 0x006409D2`.

The marker loop does not test whether the projected anchor lies inside the fitted preview rect before submitting the sprite. Clipping is delegated to the destination surface clip path. Evidence: no anchor containment branch between projection `0x006408F5..0x00640969` and draw call `0x006409D2`; clip acquired from `DAT_00887310 +0x78` at `0x006409A7..0x006409B6`.

### 3.4 Numeric label placement, color, and clipping

Active in YR: Conditional on the same overlay count; labels are drawn even if `STARTBUT.SHP` failed to load because the label call sits after the optional sprite block. Evidence: `DAT_00AC4E80` null branch `0x0064097C..0x00640980` jumps to label block `0x006409D7`.

Label behavior:

```text
label_number = i + 1
label_origin = (anchor_x - 2, anchor_y - 6)
format = DAT_0081B3D0
text helper = FUN_004A61C0
color source = color scheme named "Yellow"
clip = DAT_00887310 vtable +0x78 destination-surface clip
```

Evidence: label block `0x006409D7..0x00640A15` subtracts `2` from projected X, `6` from projected Y, increments loop index before pushing the label number, pushes `0x8` and `0x19` to the helper path, gets destination clip via `DAT_00887310 +0x78`, pushes format `0x0081B3D0`, and calls `FUN_004A61C0`. `FUN_0068ca50` is called with string `"Yellow" @ 0x00820524` before the loop (`0x006408BE..0x006408C7`), and `FUN_004A61C0` converts the resolved color through display color loss/shift before calling the font draw helper.

Alignment/rect note: this caller uses the map-overlay font helper, not `FUN_00621040`; there is no caller-provided bounded text rect like ordinary shell labels. The label origin is the anchor point above, with destination-surface clipping. Exact glyph width/height comes from `BitFont__MeasureText` inside `FUN_004A5EB0` and is not a fixed preview-table rectangle.

## 4. INI Keys / Map Fields

| Source | Key | Binary reader | Effect | Active in YR |
|---|---|---|---|---|
| `[Header]` | `StartX` | `FUN_00689d30` | origin X for projection | Conditional on map/header |
| `[Header]` | `StartY` | `FUN_00689d30` | origin Y for projection | Conditional |
| `[Header]` | `Width` | `FUN_00689d30` | X divisor | Conditional |
| `[Header]` | `Height` | `FUN_00689d30` | Y divisor | Conditional |
| `[Header]` | `NumberStartingPoints` | `FUN_00689d30` | live overlay count gate, accepted only `1..8` | Conditional |
| `[Header]` | `Waypoint%d` with one-based index | `FUN_00689d30` | live overlay coordinates | Conditional |
| `[Waypoints]` | `0=`, `1=`, etc. | gameplay/generator paths, not `DrawStartPositions` live overlay projection | Must not synthesize live overlays from this alone | No for live overlay source |
| `[Preview]`/`[PreviewPack]` | image data | preview surface loader | preview pixels only; may contain baked red markers | Yes for preview image |

No rules/art INI key controls `STARTBUT.SHP` overlay geometry, label placement, or the count gate. The SHP filename is a binary string literal.

## 5. Integration Points

| Function / path | Verified role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_006ae3f0` | Offline Skirmish dialog proc; calls common paint then preview draw | decompile; `0x006AE47B` prior assembly | Yes |
| `FUN_006067a0` | Child suppress/no-draw gate; nonzero skips parent preview draw | decompile reads owner-draw record byte `+0x30` | Yes, normally false |
| `DrawStartPositions @ 0x00640710` | Preview blit plus optional live overlays and labels | decompile/assembly ranges listed above | Conditional |
| `FUN_0060b1d0` | Right-anchor final placement for `0x468` | decompile; `0x102/0x468` allowlist in `FUN_00608cd0` | Yes |
| `FUN_00689d30` | Active selected-map reset/read helper for `[Header]` preview fields | decompile resets fields to `-1`, reads `[Header]` | Conditional |

## 6. Current Rust Implementation Status

Current Rust surfaces scanned:

- `src/ui/skirmish_shell/layout.rs`: `compute_layout`, `right_anchor`, `map_preview`.
- `src/app_list_maps.rs`: `preview_source_bounds_from_verified_source`.
- `src/app_skirmish_shell_render.rs`: `aspect_fit_rect`, `project_preview_start_positions`, `build_start_marker_instances`, `push_start_marker_labels`, `build_start_marker_label_instances`.
- `src/map/preview.rs`: `PreviewSourceBounds`, `PreviewStartPoint`.

Matches observed:

- Rust only builds `PreviewSourceBounds` from `[Header]` and rejects `count <= 0 || count >= 9`, matching the binary gate.
- Rust does not synthesize live overlay bounds from `[Waypoints]`, matching the no-overlay Dustbowl rule.
- Rust `start_marker_top_left` uses `(-9,-6)`, matching `STARTBUT.SHP`.
- Rust `layout.map_preview` currently matches the verified `0x468` child rect formula for `640`, `800`, and `1024`.

Mismatches observed:

- `aspect_fit_rect` uses floating-point ratios and `round()`. The binary uses integer per-mille truncation and half-scaled centering, which shifts Dustbowl `138x75` from Rust `(x=child_x,w=144)` to binary `(x=child_x+1,w=143)`.
- `push_start_marker_sprites` and `push_start_marker_labels` skip markers when the projected anchor is outside `preview_rect`; the binary submits overlays and relies on destination-surface clipping.
- `push_start_marker_labels` draws labels at `(anchor_x, anchor_y)` with `SHELL_BUTTON_TEXT_RGB_00000C05`; the binary label origin is `(anchor_x-2, anchor_y-6)` and uses the `"Yellow"` color scheme through `FUN_004A61C0`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x468` final rect formula | verified | `FUN_0060b1d0`, `FUN_00608cd0`, Rust resource scan | none |
| Parent paint draw order | verified | `FUN_006ae3f0`; prior assembly `0x006AE40A`, `0x006AE47B` | none |
| Preview null/suppress gates | verified | `DrawStartPositions @ 0x0064072C`; `FUN_006067a0` | none |
| Aspect-fit image rect | verified | `0x00640799..0x0064085D` | none |
| Marker source fields/count gate | verified | `FUN_00689d30`; `0x006408D4..0x00640953` | none |
| `STARTBUT.SHP` load/frame/offset | verified | string `0x00836DE4`; `0x0064088A..0x006409D2` | exact SHP pixel dimensions not needed for top-left formula |
| Numeric label origin/color/helper | verified | `0x006408BE..0x006408C7`, `0x006409D7..0x00640A15`, `FUN_004A61C0` | full glyph internals out of scope |
| Destination-surface clipping boundary | verified | `0x006409A7..0x006409B6`, `0x00640A00..0x00640A15`; `CC_Draw_Shape` clip path | runtime screenshot can validate edge pixels |
| Loose Dustbowl no-overlay case | verified via prior binary-backed doc and data | `SCENARIO_PREVIEW_HEADER_DEFAULTS_AND_DUSTBOWL_SOURCE_PATH_GHIDRA_REPORT.md`; `FUN_00689d30`; Dustbowl data | MIX variant census out of scope |
| Preview RGB decode internals | deferred | explicit non-scope; prior `PREVIEWPACK_DECODE_CHANNEL_ORDER` report | none for this slot |

## 8. Open Questions - Final State Of Investigation Log

- `[RESOLVED] OQ-1 - What is the exact target slice? -> Offline Skirmish `0x102` preview child rect and live `STARTBUT.SHP` overlay geometry only.` (evidence: dispatch scope)
- `[RESOLVED] OQ-2 - Is `0x468` child-owned paint or parent-owned? -> Parent-owned; child is an anchor/static placeholder.` (evidence: `FUN_006ae3f0`, `DrawStartPositions @ 0x00640710`)
- `[RESOLVED] OQ-3 - What final rect does child `0x468` use? -> `(screen_w - max((screen_w-800)/2,0) - 156, 37 + max((screen_h-600)/2,0), 144, 112)` because right-panel inset is 12.` (evidence: `FUN_0060b1d0`, `FUN_00608cd0`)
- `[RESOLVED] OQ-4 - Is there a `0x0060B950` one-pixel fixup for `0x468`? -> No scoped fixup branch for `0x102/0x468` was found.` (evidence: `FUN_0060b950`)
- `[RESOLVED] OQ-5 - How is the preview image fitted? -> integer per-mille truncation with `min(child_w*1000/src_w, child_h*1000/src_h)` and half-scaled centering.` (evidence: `0x00640799..0x0064085D`)
- `[RESOLVED] OQ-6 - What fields source live overlay markers? -> `[Header] StartX/StartY/Width/Height/NumberStartingPoints/Waypoint%d` loaded into `ScenarioClass+0x112C..+0x1144`.` (evidence: `FUN_00689d30`, `DrawStartPositions`)
- `[RESOLVED] OQ-7 - What count values draw overlays? -> only `1..8`; `-1`, `0`, and `>=9` skip live overlays.` (evidence: `0x006408D4..0x006408EF`)
- `[RESOLVED] OQ-8 - Does loose Dustbowl draw live overlays from gameplay `[Waypoints]`? -> No for the verified loose-map path; it can show baked red pixels but `NumberStartingPoints` remains `-1`.` (evidence: `SCENARIO_PREVIEW_HEADER_DEFAULTS...`; `FUN_00689d30`)
- `[RESOLVED] OQ-9 - Which STARTBUT frame and offset? -> `STARTBUT.SHP` frame `0` at `(anchor_x-9, anchor_y-6)`.` (evidence: `0x0064098B..0x006409D2`; string `0x00836DE4`)
- `[RESOLVED] OQ-10 - Are marker sprites clipped to fitted preview rect? -> No explicit fitted-rect containment check; destination surface clip is requested.` (evidence: `0x006409A7..0x006409D2`)
- `[RESOLVED] OQ-11 - What is numeric label origin/color? -> 1-based label at `(anchor_x-2, anchor_y-6)`, `"Yellow"` color scheme, `FUN_004A61C0` helper.` (evidence: `0x006408BE..0x006408C7`, `0x006409D7..0x00640A15`)
- `[RESOLVED] OQ-12 - Does label drawing require STARTBUT asset load success? -> No; the label block still runs if `DAT_00AC4E80 == 0`.` (evidence: branch `0x0064097C..0x00640980` to `0x006409D7`)
- `[RESOLVED] OQ-13 - Does current Rust synthesize from `[Waypoints]`? -> No; `PreviewSourceBounds` comes from `[Header]` only.` (evidence: `src/app_list_maps.rs`)
- `[DEFERRED] OQ-14 - Exact glyph bounding box for every possible marker number.` (category: out-of-scope; reason: this slot only needs caller origin/color/clip; full font helper is broader; next-step-if-pursued: investigate `FUN_004A5EB0`/BitFont as a text-helper slice)
- `[DEFERRED] OQ-15 - MIX-contained or patched Dustbowl variants.` (category: out-of-scope; reason: user specifically included loose Dustbowl no-`[Header]` case; next-step-if-pursued: archive map census)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Fitted preview image rect uses integer per-mille truncation, not float rounding; Dustbowl `138x75` in `144x112` fits to `(child_x+1, child_y+17, 143, 78)`. | `0x00640799..0x0064085D` | mismatch | `src/app_skirmish_shell_render.rs::aspect_fit_rect` | Replace float/round aspect fit with binary integer formula | At `800x600`, child `0x468=(644,37,144,112)` and Dustbowl `138x75` preview surface produces fitted rect `(645,54,143,78)` | `skirmish_preview_aspect_fit_uses_gamemd_integer_per_mille_truncation`; risk: one-pixel screenshot drift on common stock preview sizes |
| Live overlays are submitted for every `[Header]` point in count `1..8` and are clipped by destination surface, not by fitted preview rect containment. | `0x006408D4..0x006409D2`, `0x006409A7..0x006409B6` | mismatch | `build_start_marker_instances`, `build_start_marker_label_instances`, render scissor setup | Do not drop marker/label when anchor is outside fitted rect; do not apply fitted-preview scissor to live overlays | A header point projected just outside the fitted image edge still submits sprite/label and only render-target clipping cuts pixels | `start_marker_overlays_use_destination_surface_clip_not_preview_rect`; risk: edge starts currently lose pixels retail would draw |
| Numeric labels use 1-based text at `(anchor_x-2, anchor_y-6)` with `"Yellow"` color via `FUN_004A61C0`, not generic shell text at raw anchor. | `0x006408BE..0x006408C7`, `0x006409D7..0x00640A15` | mismatch | `push_start_marker_labels` | Shift labels by `(-2,-6)` and use yellow overlay color/source; keep 1-based numbering | Two live-overlay starts show label `1`/`2` aligned over STARTBUT like retail; label remains even if marker asset is absent | `start_marker_labels_use_startbut_overlay_origin_and_yellow_color`; risk: generic text path may still differ in glyph layout |
| Loose Dustbowl with no `[Header]` does not draw live `STARTBUT.SHP` overlays, even though `[PreviewPack]` can include baked red pixels. | `FUN_00689d30`; `DrawStartPositions`; `SCENARIO_PREVIEW_HEADER_DEFAULTS...`; Dustbowl data | none observed for source gate | `app_list_maps.rs::preview_source_bounds_from_verified_source`, `project_preview_start_positions` | Preserve `[Header]`-only source bounds; do not fall back to `[Waypoints]`/`LocalSize` | Loose `Dustbowl.map` decodes preview pixels but `projected_start_positions` is empty and no live marker roles appear | `loose_dustbowl_without_header_skips_live_startbut_overlays`; risk: tempting waypoint fallback creates duplicate/bogus markers |

## Negative Facts / Do Not Do

- Do not make the child static `0x468` render the preview itself. Active in YR: No; the parent `WM_PAINT` branch calls `DrawStartPositions` after common paint. Evidence: `FUN_006ae3f0`.
- Do not use floating-point rounded aspect-fit math for preview pixels. Active in YR: No; binary uses integer per-mille truncation and signed magic division. Evidence: `0x00640799..0x0064085D`.
- Do not reject a live marker because its projected anchor is outside the fitted preview rect. Active in YR: No such check in the marker loop; clipping is destination-surface based. Evidence: `0x006408F5..0x006409D2`.
- Do not synthesize live `STARTBUT.SHP` overlays from gameplay `[Waypoints]` or `[Map] LocalSize`. Active in YR: No for the verified selected-map live overlay path; source is `[Header]` fields. Evidence: `FUN_00689d30`, `DrawStartPositions`, Dustbowl report.
- Do not place numeric labels at raw marker anchor or draw them with ordinary `FUN_00621040` shell text. Active in YR: No; labels use `(anchor_x-2, anchor_y-6)` and `FUN_004A61C0`. Evidence: `0x006409D7..0x00640A15`.

## Remaining Uncertainty

- Exact glyph pixel bounds for each number are left to a broader `FUN_004A5EB0`/BitFont slice; this report verifies the caller origin, color source, format, and clip boundary.
- A MIX-contained or patched Dustbowl variant could include `[Header]` and therefore draw live overlays; this report covers the loose retail Dustbowl path named in scope.

## Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`: replace "aspect-fits the preview surface using integer `*1000` scale math" with "aspect-fits the preview surface using integer per-mille truncation: `scale=min(child_w*1000/src_w, child_h*1000/src_h)`, fitted size truncates `src*scale/1000`, and centering subtracts truncated `src*scale/2000` half extents."
- `docs/research/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`: replace "label at projected anchor `(-2,-6)`, destination-surface clipping" with "label at projected anchor plus `(-2,-6)`, using the `"Yellow"` color scheme and `FUN_004A61C0`; the caller supplies destination-surface clipping, not a fitted-preview rect."

## Sources

- Ghidra decompiled/read-only: `DrawStartPositions @ 0x00640710`, `FUN_006ae3f0 @ 0x006AE3F0`, `FUN_006067a0 @ 0x006067A0`, `FUN_0060b1d0 @ 0x0060B1D0`, `FUN_00608cd0 @ 0x00608CD0`, `FUN_0060b950 @ 0x0060B950`, `FUN_00689d30 @ 0x00689D30`, `FUN_004A61C0 @ 0x004A61C0`, `FUN_004A5EB0 @ 0x004A5EB0`, `FUN_00775690 @ 0x00775690`, `FUN_0068CA50 @ 0x0068CA50`, `CC_Draw_Shape @ 0x004AED70`.
- Ghidra assembly contexts: `0x0064072C`, `0x00640745`, `0x00640799..0x0064085D`, `0x0064088A..0x006408EF`, `0x006408F5..0x006409D2`, `0x006409D7..0x00640A15`.
- Ghidra strings/data: `STARTBUT.SHP @ 0x00836DE4`, `Yellow @ 0x00820524`, `DAT_0081B3D0` label format pointer.
- Prior reports referenced: `SCENARIO_PREVIEW_HEADER_DEFAULTS_AND_DUSTBOWL_SOURCE_PATH_GHIDRA_REPORT.md`, `SCENARIO_PREVIEW_BOUNDS_STOCK_MAP_POPULATION_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_0X102_FIRST_PAINT_COMPOSITION_VS_RUST_DRAW_ORDER_GHIDRA_REPORT.md`, `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, `src/app_list_maps.rs`, `src/map/preview.rs`.
