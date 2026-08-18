# Skirmish Shell Layout Positioning System Model Synthesis

Date: 2026-05-22

Scope: player-visible positioning and render ordering for the YR skirmish setup shell, with emphasis on controls that are currently misplaced or missing in the Rust UI.

Output type: synthesis / conflict map. This document does not patch Rust behavior; it identifies implementation-safe facts and remaining research gaps.

## Bottom Line

The current Rust skirmish UI is no longer "mostly blank"; it has many of the verified control rectangles and several rendered widgets. It is still not fully positioned like `gamemd.exe`.

The most actionable mismatches are:

1. `SDBTM.SHP` bottom-cap rendering uses destination scaling where retail clips the source through the destination rect.
2. Right-panel static text controls are not represented as first-class layout rects: title, game type, and map/scenario text.
3. Combo dropdown windows likely use the wrong row height and cap geometry.
4. Parent/right-panel draw order is not modeled exactly.
5. Wide-screen / `>800` parent-background SHP behavior is now resolved for the normal fresh Skirmish lifecycle: retail does not load a fresh alternate background above exact width 800, and the null alternate pointer no-ops in `CC_Draw_Shape`. Full high-res screenshot comparison remains useful for aggregate composition parity.

## Claim Table

| Claim | Status | Confidence | Implementation posture |
| --- | --- | --- | --- |
| Skirmish child controls are positioned from dialog units, then adjusted by resize/snap helpers, not by global proportional scaling. | Verified | High | Safe foundation. |
| Standard 800x600 shell origin is `(80, 0)` and 640x480 origin is `(0, 0)`. Heights above 767 add vertical centering; widths above 1023 add horizontal centering around an 800-wide shell. | Verified | High | Safe. |
| Start/Choose Random/Cancel buttons use owner-draw snap metadata rather than proportional layout. | Verified | High | Safe. |
| Core option rows, checkboxes, trackbars, and map preview rects in current Rust layout match many verified formula-level rects. | Verified against current code and docs | High | Keep as baseline. |
| Right-panel static labels exist as separate static text controls, not incidental text baked into panel art. | Verified | High | Safe to add missing layout/render fields. |
| `SDBTM.SHP` bottom cap is clipped by the destination rect; retail does not scale the full source image into the shorter rect. | Verified | High | Safe to fix with source/UV clipping support. |
| Right-panel draw order is `SDTP`, repeated `SDBTNBKGD`, optional `SDBTNANM` frame, `SDBTM`, lower strip; child/control paint happens above parent chrome. | Verified | High | Safe to align, but screenshot-check overlap effects. |
| Dropdown row height is derived from font height plus padding, normally 23 px, with capped visible rows and scrollbar width when needed. | Verified from docs | High | Safe after comparing exact current Rust state code. |
| Current Rust `STARTBUT.SHP` support should be sourced from `[Header]` preview metadata, not gameplay waypoints; live marker clipping is destination-surface based, not fitted-preview-rect based. | Verified from docs and `SKIRMISH_START_MARKER_CLIPPING_FOOTPRINT_GHIDRA_REPORT.md` | High | Safe; renderer/backbuffer-edge pixel coverage is still useful, but the binary clipping rule is resolved. |
| High-res `>800` parent-background SHP behavior is known for normal fresh Skirmish entry: no `MnScrnLCoopGameSetup.shp` draw above exact width 800. | Verified for static binary path; screenshot comparison still deferred for full composition | High for parent-background decision; Medium for aggregate screenshot parity | Safe to keep no parent-background sprite above 800. |

## Current System Model

Retail skirmish setup is a Win32 dialog-style shell. Its visible layout is a combination of:

- dialog-unit child-control rectangles;
- resize helper branches for specific control classes and owner-draw buttons;
- right-panel chrome computed from live SHP dimensions;
- text rendering that uses the caller `RECT` as both layout and clip;
- owner-draw controls that apply local text/icon offsets rather than generic centering.

The right side is not one monolithic bitmap. `RightPanel__ComputeLayoutRects` derives the top cap, tiled middle, optional animation overlay, bottom cap, and lower strip from the screen size and SHP dimensions. `RightPanel__Draw` then draws those pieces in a fixed order.

The map preview is a child/static preview surface region. Preview start markers are overlaid from `STARTBUT.SHP` when valid preview header metadata exists. Loose map fallback behavior can rely on baked PreviewPack pixels instead.

## Implementation-Safe Layout Facts

These can be implemented or corrected without new reverse engineering:

- Add explicit layout fields for right-panel static text:
  - title static: `(475, 3, 162, 16)` with the known y+1 paint adjustment;
  - game type static: `(489, 167, 135, 16)`;
  - map/scenario static: `(489, 189, 135, 33)`.
- Render those statics through the same clipped text path used by the rest of the skirmish UI.
- Replace `SDBTM` destination scaling with clipped drawing from the original source image into the shorter destination rect.
- Audit dropdown geometry against the verified combo/dropdown docs:
  - collapsed face remains one geometry;
  - dropdown rows use the verified font-derived row height;
  - visible row caps and scrollbar reservation must match owner-draw combo behavior.
- Keep `[Header]`-only live start-position marker logic. Do not infer live marker positions from gameplay `[Waypoints]`.

## Likely Rust Mismatches To Fix First

1. `SDBTM` bottom cap

Player-visible symptom: the lower right panel cap can look vertically squeezed instead of cropped. This is high-value because it affects the persistent shell chrome.

Fix shape: add a clipped source/destination quad path for the right-panel bottom cap, then screenshot-test 640x480 and 800x600.

2. Missing right-panel static text rects

Player-visible symptom: right-panel labels either do not appear or are placed indirectly. This makes the screen look incomplete even when buttons and controls are present.

Fix shape: model title, game type, and map/scenario text as named layout rects, then render with normal static text clipping.

3. Combo dropdown geometry

Player-visible symptom: opened dropdowns can sit too low, be too tall, or have rows that do not align with retail font rhythm.

Fix shape: replace hardcoded face-height row math with the verified dropdown row-height and visible-row cap rules.

4. Parent/right-panel draw order

Player-visible symptom: normally subtle, but incorrect ordering can show as wrong overlap or edge treatment around the right panel and lower strip.

Fix shape: align render pass ordering with `RightPanel__Draw` and child-control paint sequence, then compare screenshots.

5. Wide-screen shell background

Player-visible symptom: at widths above 800/1024, shell chrome may be offset correctly but aggregate composition can still need screenshot confirmation.

Fix shape: keep the parent-background SHP skipped above 800, and use a future retail screenshot comparison only to validate the full high-resolution composition.

## Stale Or Superseded Claims

Older skirmish UI docs and traces that describe the Rust UI as lacking all combo/checkbox/trackbar rendering are stale. Current Rust has substantially more widget rendering than those docs assumed.

Older notes that imply `SDBTNANM` frame 10 is always active in Rust should be rechecked against current code; the active flag now appears intentionally false for the standard first-paint path.

The previous "STARTBUT is skipped" implementation note is superseded by the current `[Header]`-based marker support. Marker edge clipping is now resolved by `SKIRMISH_START_MARKER_CLIPPING_FOOTPRINT_GHIDRA_REPORT.md`: markers are submitted at `(anchor_x-9, anchor_y-6)` and clipped through the destination surface, not the fitted preview rect. A renderer/backbuffer-edge pixel test remains useful implementation coverage.

## Needs Further Investigation

- Full high-resolution retail screenshot comparison above 800-wide shells, excluding the now-resolved parent-background SHP decision.
- Whether preview marker numeric labels are clipped to the preview static, the destination surface clip, or both in all edge cases.
- Exact text strings and invalidation timing for the right-panel game type and selected map/scenario statics after selection changes.

## Do Not Implement From Inference

- Do not derive live start markers from gameplay `[Waypoints]`; Dustbowl-style loose maps show why this is unsafe.
- Do not globally scale dialog controls to fit arbitrary resolutions.
- Do not replace owner-draw widget offsets with generic centered text.
- Do not treat baked PreviewPack red start pixels and live `STARTBUT.SHP` overlays as the same rendering layer.

## Source Ledger

Docs consulted:

- `skirmish-ui/SKIRMISH_SHELL_640X480_FINAL_VISIBLE_LAYOUT_TRACE.md`
- `skirmish-ui/SKIRMISH_SHELL_CHROME_800X600_TRACE.md`
- `skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_SDBTM_BOTTOM_CAP_SOURCE_CLIP_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_MAP_PREVIEW_MARKER_ASSET_LAYOUT_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_GT800_BACKGROUND_POINTER_LIFECYCLE_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_GT800_BACKGROUND_TARGETED_TRACE_RECONCILIATION.md`
- `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`

Ghidra spot checks:

- `0060C0C0` / resize helper: confirmed branch-based child-control positioning and final one-pixel fixups.
- `0072EC70` / right-panel layout: confirmed threshold handling, SHP-dimension-driven panel rects, and lower strip selection.
- `0072E450` / right-panel draw: confirmed top/middle/animation/bottom/lower-strip order and destination clipping behavior.
- `00621040` / text wrapper: confirmed caller rect acts as layout and clip, with optional vertical centering.

Rust surfaces inspected:

- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/app_skirmish_shell_render.rs`
- `src/app_list_maps.rs`
- `src/map/preview.rs`
