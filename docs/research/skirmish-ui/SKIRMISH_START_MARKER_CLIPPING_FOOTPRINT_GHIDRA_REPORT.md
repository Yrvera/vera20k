# Skirmish Start Marker Clipping Footprint - Ghidra Research Report

**Address(es):** `DrawStartPositions @ 0x00640710`, `CC_Draw_Shape @ 0x004AED70`, `AlphaShapeClass__ClipRect @ 0x00421B60`, caller `FUN_006AE3F0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `STARTBUT.SHP` live start-marker clipping when the translated marker rectangle is partially outside the generated preview image or fitted preview destination in the Skirmish map preview paint path.
**Non-Scope:** PreviewPack RGB/channel decode, generated-preview baked 4x4 red waypoint pixels, normal in-bounds marker constants, full font glyph rasterization, Choose Map modal `0x6B`, and unrelated Skirmish shell layout.
**Confidence:** High for marker submission, anchoring, and destination-surface clipping; Medium for the concrete runtime extent of `DAT_00887310`'s active clip because the vtable target was verified through call sites and downstream clipping, not renamed/type-recovered.
**Active in YR:** Conditional. The path is live in standard YR Skirmish `WM_PAINT` when the preview object exists and child `0x468` is not suppressing paint; live `STARTBUT.SHP` overlays require `[Header]`-derived `ScenarioClass+0x113C` in `1..8`.

## 0. Working Notes Required By Dispatch

Target question: What is the exact `STARTBUT.SHP`/start-marker clipping footprint when marker pixels are partially outside the generated preview surface or the fitted preview destination?

Non-goals: Do not redo PreviewPack channel order, normal in-bounds marker constants, baked generated-preview 4x4 red rectangles, or broad Skirmish UI layout.

Evidence needed to mark COMPLETE: `DrawStartPositions` decompile plus assembly ranges for projection/submission; `CC_Draw_Shape` plus `AlphaShapeClass__ClipRect` evidence for the clipping contract; caller/xref evidence that the path is active in YR; Rust-facing acceptance scenarios.

Stop conditions: Stop after resolving sprite footprint and clip boundary; do not use mutating Ghidra operations; do not modify Rust/INI/in-repo docs; if a vtable boundary cannot be typed read-only, record it as remaining uncertainty instead of expanding.

## 1. Overview

`DrawStartPositions` draws the decoded preview image first, then conditionally submits live `STARTBUT.SHP` frame `0` for `[Header]` start points. The live marker is not clipped to the generated preview source surface and is not rejected when its projected anchor falls outside the fitted preview rectangle. Its visible pixels are the translated SHP frame intersected by the active destination-surface clip used by `DAT_00887310`/`CC_Draw_Shape`.

Active in YR: Conditional. Evidence: `FUN_006AE3F0` handles `WM_PAINT`, checks `DAT_00AC1154`, calls `GetDlgItem(...,0x468)`, calls the suppress gate `FUN_006067A0`, then calls `DrawStartPositions`; `DrawStartPositions` gates overlays with `0 < ScenarioClass+0x113C < 9` at `0x006408D4..0x006408EF`.

## 2. Class Layout / Key Offsets

| Item / offset | Verified purpose | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00AC1154` | Preview object pointer checked by Skirmish `WM_PAINT` before drawing | `FUN_006AE3F0` decompile | Yes, when a selected map preview exists |
| child `0x468` | Preview anchor/static window converted to game-client coordinates | `FUN_006AE3F0`; `DrawStartPositions` `GetDlgItem`/`FUN_00775690` | Yes |
| preview surface vtable `+0x78` | Supplies preview source bounds for aspect fit | `DrawStartPositions @ 0x00640745..0x00640799` | Yes when preview object exists |
| `ScenarioClass+0x112C/+0x1130` | `[Header]` `StartX`/`StartY` projection origin | `FUN_00689D30`; reads in `0x006408F5..0x00640953` | Conditional on map header |
| `ScenarioClass+0x1134/+0x1138` | `[Header]` `Width`/`Height` projection divisors | `FUN_00689D30`; `IDIV` at `0x00640917`, `0x00640953` | Conditional |
| `ScenarioClass+0x113C` | live overlay count gate | `FUN_00689D30`; guard `0x006408D4..0x006408EF` | Conditional, accepted only `1..8` |
| `ScenarioClass+0x1140/+0x1144 + i*8` | `[Header] WaypointN` X/Y pair | `FUN_00689D30`; loop reads `0x006408F5`, `0x0064093B` | Conditional |
| `DAT_00AC4E80` | cached `STARTBUT.SHP` asset pointer | loader block `0x0064088A..0x006408B2`; null test `0x0064097C..0x00640980` | Conditional |
| `DAT_00887310` | destination surface for preview, markers, and labels | preview blit call near `0x00640860`; marker clip call `0x0064099D..0x006409D2`; label clip call `0x006409D7..0x00640A15` | Yes |

## 3. Core Logic

### 3.1 Active paint entry

Active in YR: Yes for the offline Skirmish dialog. `FUN_006AE3F0` first delegates common shell processing to `FUN_00622B50`. On `WM_PAINT (0xF)`, if `DAT_00AC1154 != 0`, it locates child `0x468`, calls `FUN_006067A0`, and calls `DrawStartPositions` only if the suppress gate returns zero.

Handoff-critical evidence: decompile `FUN_006AE3F0`; xrefs to `DrawStartPositions` include active caller `0x006AE47B`.

### 3.2 Marker projection and submission

Active in YR: Conditional on `0 < ScenarioClass+0x113C < 9`. The guard at `0x006408D4..0x006408EF` exits for `-1`, `0`, or `>=9`, so no live `STARTBUT` overlays are emitted in those cases.

For each accepted start index:

```text
x_per_mille = trunc((WaypointX[i] - StartX) * 1000 / Width)
y_per_mille = trunc((WaypointY[i] - StartY) * 1000 / Height)
anchor_x = fit_x + trunc(x_per_mille * fit_w / 1000)
anchor_y = fit_y + trunc(y_per_mille * fit_h / 1000)
marker_top_left = (anchor_x - 9, anchor_y - 6)
```

Evidence: X projection at `0x006408F5..0x00640939`, Y projection at `0x0064093B..0x0064097A`, `STARTBUT` top-left at `0x0064098B` (`anchor_x - 9`) and `0x00640999` (`anchor_y - 6`), sprite call at `0x006409D2`.

There is no branch between projection and `CC_Draw_Shape` that tests whether `anchor_x/anchor_y`, `marker_top_left`, or the marker rectangle is inside the fitted preview image. Active in YR: Yes, because the code reaches this region directly after the `1..8` count gate when `DAT_00AC4E80` is non-null.

### 3.3 Clipping footprint

Active in YR: Yes on live overlay paths.

The marker's submitted footprint is the native `STARTBUT.SHP` frame translated to `(anchor_x - 9, anchor_y - 6)`. Downstream `CC_Draw_Shape` computes/uses the frame rectangle, obtains the destination-surface clip through vtable `+0x78`, intersects the caller draw rectangle with that clip through `AlphaShapeClass__ClipRect`, and returns before blitting if the clipped width or height is less than `1`.

Material clipping details:

- `DrawStartPositions` asks `DAT_00887310` vtable `+0x78` for a draw/clip structure immediately before the marker call (`0x0064099D..0x006409B6`), then passes it to `CC_Draw_Shape` (`0x006409C3..0x006409D2`).
- `CC_Draw_Shape` independently queries the destination surface clip through vtable `+0x78`, copies the caller-provided draw rect, intersects them with `AlphaShapeClass__ClipRect`, and returns if the resulting width or height is non-positive (`0x004AEDCD..0x004AEE46`).
- `AlphaShapeClass__ClipRect` treats rectangles as `(x, y, width, height)`, clamps left/top by reducing width/height, clamps right/bottom to `clip.x + clip.w` / `clip.y + clip.h`, and emits a zero rectangle when either input has non-positive dimensions or the intersection is empty.
- Standard SHP blitting also reaches `Blitter_ClipAndSetup @ 0x007BC040` via `Standard_SHP_blitter` (`0x00437461`), which calls `ClipRectPair @ 0x007BBE20` before locking/copying pixels. This is a second setup-level clip/validation stage, not a fitted-preview containment gate.

The fitted preview destination rectangle is used to position the anchor and draw the preview image; it is not installed as a marker scissor/clip in `DrawStartPositions`. Therefore a live marker partially outside the generated preview image or fitted image rectangle remains drawable where it overlaps the active destination surface. If the marker rectangle is outside the destination clip entirely, `AlphaShapeClass__ClipRect` produces width/height `0` and `CC_Draw_Shape` returns without pixels.

### 3.4 Edge behavior examples

Active in YR: Conditional; these examples follow directly from the binary clipping path when the live overlay count gate is satisfied.

| Projected anchor case | Submitted marker top-left | Visible footprint rule | Evidence |
|---|---:|---|---|
| anchor at fitted preview left/top | `(fit_x - 9, fit_y - 6)` | left/top part of `STARTBUT` may draw outside the fitted preview image; only destination clip can trim it | no containment branch `0x006408F5..0x006409D2`; `CC_Draw_Shape` clip `0x004AEDCD..0x004AEE46` |
| anchor just right of fitted preview | `(anchor_x - 9, anchor_y - 6)` | any pixels overlapping destination clip draw; marker is not dropped for `anchor_x > fit_x + fit_w` | same |
| marker partly beyond the backbuffer/surface clip | translated frame intersected with destination clip | outside-surface pixels are removed; if width/height becomes `0`, no blit | `AlphaShapeClass__ClipRect @ 0x00421B60` |
| marker completely beyond destination clip | any | no pixels | `CC_Draw_Shape` tests clipped width/height and returns at `0x004AEE3E..0x004AEE46` |

## 4. INI Keys / Map Fields

| Source | Key | Binary reader | Effect | Active in YR |
|---|---|---|---|---|
| `[Header]` | `StartX` | `FUN_00689D30` | projection origin X | Conditional |
| `[Header]` | `StartY` | `FUN_00689D30` | projection origin Y | Conditional |
| `[Header]` | `Width` | `FUN_00689D30` | X projection divisor | Conditional |
| `[Header]` | `Height` | `FUN_00689D30` | Y projection divisor | Conditional |
| `[Header]` | `NumberStartingPoints` | `FUN_00689D30` | live overlay count; accepted only `1..8` by draw path | Conditional |
| `[Header]` | `Waypoint%d` | `FUN_00689D30`, one-based formatted key | live overlay coordinates | Conditional |
| `[Waypoints]` | `0..7` gameplay waypoints | not used by `DrawStartPositions` live overlay projection | no live marker fallback | No for this overlay path |
| `[Preview]` / `[PreviewPack]` | decoded preview pixels | preview object/surface path | background preview image only; may contain baked red generated markers | Yes for preview image |

No rules/art INI key controls the clipping footprint. `STARTBUT.SHP` is a binary string literal at `0x00836DE4`, loaded in `0x0064088A..0x006408B2`.

## 5. Integration Points

| Function / path | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_006AE3F0` | Offline Skirmish dialog proc; `WM_PAINT` caller | decompile; xref source `0x006AE47B` | Yes |
| `DrawStartPositions @ 0x00640710` | Preview image blit plus live start markers and labels | decompile and assembly contexts listed above | Conditional |
| `FUN_00775690` | Converts child `0x468` window rect to game client/backbuffer coords | decompile | Yes |
| destination surface vtable `+0x78` | Supplies clip/draw rectangle structures for preview, marker, and label calls | `0x00640860..0x00640879`, `0x0064099D..0x006409B6`, `0x006409D7..0x00640A01` | Yes |
| `CC_Draw_Shape @ 0x004AED70` | Draws SHP frame after rect intersection | decompile; call from `0x006409D2` | Yes |
| `AlphaShapeClass__ClipRect @ 0x00421B60` | Rectangle intersection; returns zero rect for empty overlap | decompile; call at `0x004AEE1A` | Yes |
| `Standard_SHP_blitter -> Blitter_ClipAndSetup -> ClipRectPair` | Lower blit setup clipping/lock validation | `0x00437461`, `0x007BC040`, `0x007BBE20` | Yes for standard SHP frames |

## 6. Current Rust Implementation Status

Current Rust surfaces scanned:

- `src/app_skirmish_shell_render.rs`
- `src/app_list_maps.rs`
- `src/map/preview.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/render/batch.rs`

Observed matches:

- `aspect_fit_rect` currently uses the integer per-mille/truncation formula (`src/app_skirmish_shell_render.rs:1764`), matching the binary fit math.
- `project_preview_start_positions` uses `[Header]` `PreviewSourceBounds` and integer per-mille projection (`src/app_skirmish_shell_render.rs:419`), matching the marker projection.
- `preview_source_bounds_from_verified_source` reads `[Header]` only and rejects `count <= 0 || count >= 9` (`src/app_list_maps.rs:88`), matching the live-overlay gate and avoiding `[Waypoints]` fallback.
- `push_start_marker_sprites` submits native `STARTBUT` sprites at `anchor + (-9,-6)` (`src/app_skirmish_shell_render.rs:397`), and the current test `start_marker_overlays_use_destination_surface_clip_not_preview_rect` documents that an outside-fitted-preview anchor is not rejected.
- `start_marker_label_origin` uses `(-2,-6)` and yellow shell label color (`src/app_skirmish_shell_render.rs:1688`), matching the previously settled label-origin rule.

Potential remaining Rust risk:

- The deterministic helper test proves no fitted-preview containment rejection, but a renderer-level pixel/scissor test for a marker partially outside the window/backbuffer clip was not found. WGPU viewport/window clipping may discard off-screen geometry correctly, but a targeted render or instance-footprint test would make the binary edge rule explicit.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_006AE3F0` Skirmish `WM_PAINT` caller | verified | decompile; `DrawStartPositions` xref `0x006AE47B` | none |
| `DrawStartPositions` overlay count gate | verified | `0x006408D4..0x006408EF` | none |
| Marker projection math | verified | `0x006408F5..0x0064097A` | none |
| `STARTBUT` marker top-left | verified | `0x0064098B`, `0x00640999`, `0x006409D2` | none |
| No fitted-preview containment branch | verified | linear assembly path `0x006408F5..0x006409D2` | none |
| Destination-surface clip handoff | verified | `0x0064099D..0x006409D2`; `CC_Draw_Shape` `0x004AEDCD..0x004AEE46` | concrete vtable symbol name remains untyped, but behavior is verified downstream |
| `AlphaShapeClass__ClipRect` semantics | verified | decompile `0x00421B60` | none |
| lower blitter clip validation | verified | `Standard_SHP_blitter` xref `0x00437461`; `Blitter_ClipAndSetup @ 0x007BC040`; `ClipRectPair @ 0x007BBE20` | none |
| Generated-preview baked red markers | deferred | explicit non-scope; parent supplied as settled | no work for this slot |
| PreviewPack channel order | deferred | explicit non-scope; parent supplied as settled | no work for this slot |

## 8. Open Questions - Final State Of Investigation Log

- `[RESOLVED] OQ-1 - Is the Skirmish `DrawStartPositions` path active in YR? -> Yes, conditionally from offline Skirmish `WM_PAINT` when a preview object exists and child `0x468` is not suppressed.` (evidence: `FUN_006AE3F0`; xref `0x006AE47B`)
- `[RESOLVED] OQ-2 - What gates live STARTBUT overlays? -> `ScenarioClass+0x113C` must be in `1..8`; otherwise no live overlay loop runs.` (evidence: `0x006408D4..0x006408EF`)
- `[RESOLVED] OQ-3 - Which map fields feed the marker projection? -> `[Header] StartX/StartY/Width/Height/NumberStartingPoints/Waypoint%d` loaded by `FUN_00689D30` into `ScenarioClass+0x112C..+0x1144`.` (evidence: `FUN_00689D30`)
- `[RESOLVED] OQ-4 - What is the marker top-left at submission? -> `anchor_x - 9`, `anchor_y - 6`.` (evidence: `0x0064098B`, `0x00640999`)
- `[RESOLVED] OQ-5 - Does the loop reject anchors outside the fitted preview rect? -> No; there is no containment branch before `CC_Draw_Shape`.` (evidence: `0x006408F5..0x006409D2`)
- `[RESOLVED] OQ-6 - Are markers clipped to the generated preview source surface? -> No; `STARTBUT` is a separate SHP draw on `DAT_00887310`, not part of the preview source surface blit.` (evidence: preview blit before marker loop `0x00640860..0x00640879`; marker SHP call `0x006409D2`)
- `[RESOLVED] OQ-7 - What clips partially off-destination marker pixels? -> destination-surface clip structures from `DAT_00887310` vtable `+0x78`, intersected in `CC_Draw_Shape` through `AlphaShapeClass__ClipRect`, with lower blitter validation afterward.` (evidence: `0x0064099D..0x006409D2`; `0x004AEDCD..0x004AEE46`; `0x00421B60`; `0x00437461`)
- `[RESOLVED] OQ-8 - What happens if clipping removes all pixels? -> `CC_Draw_Shape` returns before blitting when clipped width or height is less than `1`.` (evidence: tests after `AlphaShapeClass__ClipRect` at `0x004AEE38..0x004AEE46`)
- `[RESOLVED] OQ-9 - Does label drawing share the same destination-clip principle? -> Yes; after the sprite block, label drawing calls destination vtable `+0x78` and `FUN_004A61C0`.` (evidence: `0x006409D7..0x00640A15`)
- `[RESOLVED] OQ-10 - Is `[Waypoints]` a fallback source for live marker clipping cases? -> No for this path; Rust should keep `[Header]` as the source for live overlays.` (evidence: `FUN_00689D30`; `DrawStartPositions` reads `ScenarioClass+0x112C..+0x1144`)
- `[RESOLVED] OQ-11 - Does current Rust drop outside-fitted-preview anchors? -> No observed in current scan; `project_preview_start_positions` returns projected anchors and `push_start_marker_sprites` submits them without `preview.contains`; existing test covers an outside anchor.` (evidence: `src/app_skirmish_shell_render.rs:397`, `:419`, `:2397`)
- `[RESOLVED] OQ-12 - What Rust acceptance remains useful? -> Add/keep deterministic edge tests for outside fitted rect and add render/backbuffer-edge coverage for destination clipping.` (evidence: current Rust scan; binary clipping path above)
- `[DEFERRED] OQ-13 - What is the concrete class/name of `DAT_00887310` vtable slot `+0x78`?` (category: bounded-cost-too-high; reason: read-only behavior was proven through call sites and downstream clipping; type recovery/name binding is not needed for the pixel rule; next-step-if-pursued: run a separate surface-vtable investigation)
- `[DEFERRED] OQ-14 - Do generated-preview baked 4x4 red rectangles have any separate edge rule?` (category: out-of-scope; reason: parent supplied this as settled and requested not to redo generated marker constants; next-step-if-pursued: generated terrain preview slice)

Deferred share is limited to naming/type recovery and explicit non-scope. No material clipping behavior remains open for the claimed live `STARTBUT` slice.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Live `STARTBUT` overlays are submitted even when the projected anchor or part of the marker is outside the fitted preview image; clipping is by destination surface, not fitted preview rect. | `0x006408F5..0x006409D2`; `CC_Draw_Shape @ 0x004AED70`; `AlphaShapeClass__ClipRect @ 0x00421B60` | none observed for helper logic; renderer-edge pixel coverage still useful | `src/app_skirmish_shell_render.rs::project_preview_start_positions`, `push_start_marker_sprites`, batch renderer/window clipping | Preserve outside-anchor submission; rely on destination/backbuffer clipping for off-screen pixels | `start_marker_overlays_use_destination_surface_clip_not_preview_rect`; add `start_marker_partially_off_backbuffer_is_submitted_for_destination_clip` | Do not add `preview_rect.contains(anchor)` or a fitted-preview scissor for live overlays |
| Marker footprint is native `STARTBUT.SHP` frame `0` translated to `(anchor_x-9, anchor_y-6)`, then intersected with the active destination clip; if the intersection width or height is `0`, no marker pixels appear. | `0x0064098B..0x006409D2`; `0x004AEE1A..0x004AEE46` | none observed for top-left; no explicit pixel-level clip test found | `src/app_skirmish_shell_render.rs::start_marker_top_left`; renderer/sprite instance tests | Keep native-size sprite instance; ensure negative/top-left or beyond-screen positions are not pre-clamped before rendering | `start_marker_top_left_outside_surface_keeps_native_rect_for_clip`; render test with anchor `(5,5)` should submit top-left `(-4,-1)` and show only destination-overlapping pixels | Do not pre-clamp marker top-left to preview/image/window bounds, because that shifts visible pixels |
| Live overlay source is `[Header]` preview metadata only; loose maps without valid `[Header] NumberStartingPoints` skip live `STARTBUT` overlays even if `[Waypoints]` exist or baked preview red pixels are visible. | `FUN_00689D30`; `0x006408D4..0x006408EF`; current Rust `preview_source_bounds_from_verified_source` | none observed | `src/app_list_maps.rs::preview_source_bounds_from_verified_source`, `src/map/preview.rs::PreviewSourceBounds` | Keep `[Header]` gate and count range; do not synthesize clipping cases from gameplay waypoints | `loose_dustbowl_without_header_skips_live_startbut_overlays` | Do not duplicate baked `[PreviewPack]` markers with live `STARTBUT` from `[Waypoints]` |

## Negative Facts / Do Not Do

- Do not clip live `STARTBUT` overlays to the generated preview source surface. Active in YR: No; the preview image blit and marker SHP draw are separate calls. Evidence: `0x00640860..0x00640879`, `0x006409D2`.
- Do not clip or reject live markers against the fitted preview destination rectangle. Active in YR: No; no containment branch exists between projection and `CC_Draw_Shape`. Evidence: `0x006408F5..0x006409D2`.
- Do not clamp marker top-left to the preview/window edge before drawing. Active in YR: No; clipping adjusts the visible rectangle, not the submitted top-left anchor. Evidence: top-left arithmetic `0x0064098B`, `0x00640999`; `AlphaShapeClass__ClipRect @ 0x00421B60`.
- Do not synthesize live `STARTBUT` overlays from `[Waypoints]` or `LocalSize`. Active in YR: No for this path; `DrawStartPositions` reads `[Header]`-loaded `ScenarioClass+0x112C..+0x1144`. Evidence: `FUN_00689D30`; `0x006408F5..0x00640953`.
- Do not treat `DAT_00AC4E80 == 0` as disabling labels. Active in YR: No; the sprite block is skipped but label code still runs after `0x006409D7`. Evidence: branch `0x0064097C..0x00640980`.

## Remaining Uncertainty

- The concrete class/name for `DAT_00887310` vtable slot `+0x78` remains untyped in this report. This does not block the clipping footprint because the caller handoff, `CC_Draw_Shape` intersection, `AlphaShapeClass__ClipRect` semantics, and lower blitter validation were verified read-only.

## Proposed Rust Tests

- `start_marker_overlays_use_destination_surface_clip_not_preview_rect` (already present in current scan; keep it as the central deterministic projection test).
- `start_marker_top_left_outside_surface_keeps_native_rect_for_clip`.
- `start_marker_partially_off_backbuffer_is_submitted_for_destination_clip`.
- `start_marker_completely_outside_backbuffer_emits_no_visible_pixels`.
- `loose_dustbowl_without_header_skips_live_startbut_overlays`.

## Stale Docs / Follow-up Docs

- Replace any claim that "marker edge clipping remains unresolved" with: "Live `STARTBUT.SHP` markers are not clipped to the generated preview source or fitted preview rectangle; `DrawStartPositions` submits the native frame at `(anchor_x-9, anchor_y-6)`, and `CC_Draw_Shape` clips the resulting rectangle through the active `DAT_00887310` destination-surface clip via `AlphaShapeClass__ClipRect`."
- Replace any claim that "Rust drops markers whose anchor is outside the preview rect" with: "Current Rust no longer shows an anchor-containment drop in the scanned helper path; it has a deterministic outside-fitted-preview test, but still benefits from a renderer/backbuffer-edge pixel test."
- Keep the existing stale-doc correction that loose Dustbowl without `[Header]` skips live `STARTBUT` overlays; visible red starts there are baked preview pixels, not live overlay clipping.

## Sources

- Ghidra read-only decompiled: `DrawStartPositions @ 0x00640710`, `FUN_006AE3F0 @ 0x006AE3F0`, `FUN_00689D30 @ 0x00689D30`, `CC_Draw_Shape @ 0x004AED70`, `AlphaShapeClass__ClipRect @ 0x00421B60`, `FUN_004A61C0 @ 0x004A61C0`, `Blitter_ClipAndSetup @ 0x007BC040`, `ClipRectPair @ 0x007BBE20`.
- Ghidra assembly contexts: `0x006408D4..0x006408EF`, `0x006408F5..0x0064097A`, `0x0064098B..0x006409D2`, `0x006409D7..0x00640A15`, `0x004AEE1A..0x004AEE46`, `0x00437461`.
- Ghidra strings/xrefs: `STARTBUT.SHP @ 0x00836DE4`; `DrawStartPositions` xrefs including `FUN_006AE3F0 @ 0x006AE47B`.
- Prior docs referenced: `docs/research/skirmish-ui/SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_MAP_PREVIEW_START_MARKERS_TRACE.md`.
- Rust scanned: `src/app_skirmish_shell_render.rs`, `src/app_list_maps.rs`, `src/map/preview.rs`, `src/ui/skirmish_shell/layout.rs`, `src/render/batch.rs`.
