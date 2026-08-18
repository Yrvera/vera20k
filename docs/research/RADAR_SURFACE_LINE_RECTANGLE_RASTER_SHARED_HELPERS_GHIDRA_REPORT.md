# Radar Surface Line/Rectangle Shared Helpers - Ghidra Research Report

**Address(es):** `RadarClass::Update @ 0x00656EC0`, `RadarClass::Init_For_House @ 0x00652E90`, `SetSidebarTextColor @ 0x0072F440`, `XSurface/BSurface line slot @ 0x007BA610`, rectangle worker `0x007BADC0`, rectangle wrapper `0x007BAD90`, clip helper `0x007BC2B0`  
**Investigation Mode:** exhaustive-slice for the minimap viewport/content-boundary rectangle helper path; coverage note only for non-axis use of the same `+0x2C` line slot.  
**Claimed Scope:** helper identity, call order, clipping, endpoint inclusion for viewport rectangle edges, color input route, packed 8/16-bit pixel writes, and Rust-facing deltas.  
**Non-Scope:** radar-event gradient/Z/A-buffer line raster except as a negative comparison, full sidebar chrome, runtime RGB555/RGB565 sampling beyond consuming the existing pixel-format report, and arbitrary non-radar users of `0x007BA610`.  
**Confidence:** High for viewport rectangle helper identity, axis endpoint coverage, clipping, and color route; Medium for non-axis `0x007BA610` lines because they are described only to prevent accidental reuse assumptions.  
**Active in YR:** Yes. `RadarClass::Update` reaches `g_SidebarSurface+0x58` for the ordinary in-game radar when `RadarClass+0x14B0 == 1 && RadarClass+0x14AC == 1`.

## Working Notes Required By Slot

Target question: Verify the shared DSurface line/rectangle raster helpers used by the minimap viewport border and radar/sidebar surface copy paths.

Non-goals: Do not redo radar-event line raster except to prove helper differences; do not expand into unrelated sidebar chrome; do not edit Rust.

Evidence needed to mark COMPLETE: decompile plus assembly/vtable evidence for `+0x58 -> 0x007BAD90 -> 0x007BADC0 -> +0x2C/0x007BA610`, clipping, endpoint bounds, 16-bit color writes, side-color packing, and active `RadarClass::Update` caller evidence.

Stop conditions: Stop after the viewport rectangle path's helper semantics and Rust handoff are proven; defer arbitrary non-axis `0x007BA610` caller census and unrelated sidebar primitives.

## 1. Overview

The ordinary camera-window rectangle and the surrounding radar content boundary do not use the radar-event gradient/Z/A-buffer line helper. `RadarClass::Update` draws both outlines on `g_SidebarSurface` through vtable slot `+0x58`, which resolves for `XSurface/BSurface` to `0x007BAD90`; that wrapper calls worker `0x007BADC0`, which converts `{x,y,w,h}` into four axis-aligned line segments and dispatches each through the plain surface line slot `+0x2C` (`0x007BA610` for `XSurface/BSurface`).

For the rectangle case, endpoints are inclusive after clipping. The helper writes the supplied packed surface pixel value directly as byte or word pixels, so the YR radar/sidebar 16-bit path is a raw packed-16 overwrite. There is no Z-buffer, A-buffer modulation, additive brightening, radar-event dirty gate, or two-pixel thickness in this rectangle path.

## 2. Class Layout / Key Offsets

| Owner | Offset / address | Meaning | Active in YR | Evidence |
|---|---:|---|---|---|
| `RadarClass` | `+0x1208` | packed sidebar text/border color used for viewport and content-boundary rectangles | Yes | `0x00652E90`, `0x00657652..0x006576A2` |
| `RadarClass` | `+0x14DC..+0x14E8` | current camera-window rectangle `{x,y,w,h}` in sidebar coordinates | Yes | `0x00656EC0`, prior viewport report |
| `RadarClass` | `+0x149C/+0x14A0/+0x14A4/+0x14A8` | minimap content origin and size used by boundary rectangle | Yes | `0x00657669..0x006576A2` |
| `g_SidebarSurface` | `0x00887300` | DSurface pointer receiving final radar/sidebar outlines | Yes | `0x00657652..0x006576A2` |
| `vtable__XSurface` | `0x007E2104 + 0x2C = 0x007BA610` | plain surface line helper | Yes for XSurface users | local PE vtable read, `0x007BA610` |
| `vtable__XSurface` | `0x007E2104 + 0x58 = 0x007BAD90` | rectangle wrapper entry | Yes for XSurface users | local PE vtable read, `0x007BAD90` |
| `vtable__BSurface` | `0x007E2070 + 0x2C = 0x007BA610` | same plain line helper | Yes for BSurface users | local PE vtable read, `0x007BA610` |
| `vtable__BSurface` | `0x007E2070 + 0x58 = 0x007BAD90` | same rectangle wrapper entry | Yes for BSurface users | local PE vtable read, `0x007BAD90` |

## 3. Core Logic

### Active caller and helper identity

Active in YR: Yes.

Evidence:

- `RadarClass::Update @ 0x00656EC0` calls `g_SidebarSurface+0x58` twice at `0x00657652..0x006576A2`: first with the current viewport rectangle at `+0x14DC`, then with `(origin_x - 1, origin_y - 1, content_w + 2, content_h + 2)`.
- The `XSurface` and `BSurface` vtables both have `+0x58 = 0x007BAD90` and `+0x2C = 0x007BA610` in the local retail executable.
- Assembly at `0x007BAD90..0x007BADB8` obtains a surface clip/rect context through vtable `+0x78`, pushes it to worker `0x007BADC0`, and returns after `RET 0x8`.
- Assembly at `0x007BADC0..0x007BAE5B` computes `right = x + w - 1` and `bottom = y + h - 1`, then calls surface vtable `+0x2C` four times.

This proves the minimap viewport rectangle uses the plain surface rectangle helper, not `DrawViewportRect @ 0x00660540` and not radar-event `0x004BDF00`.

### Rectangle expansion

Active in YR: Yes for the current viewport outline and content-boundary outline.

`0x007BADC0` consumes a rectangle and constructs four axis-aligned line segments from the inclusive bounds:

```text
left   = x
top    = y
right  = x + w - 1
bottom = y + h - 1

draw line top edge
draw line right/vertical edge
draw line bottom edge
draw line left/vertical edge
```

The four calls all pass through vtable `+0x2C`, so the rectangle edge coverage is whatever `0x007BA610` does for horizontal and vertical lines. Corners are reached by both adjacent edges; because the same packed color is written, double-writing a corner has no visible difference.

### Plain line helper clipping and endpoint rules

Active in YR: Yes for the rectangle edges described above.

`0x007BA610` performs these steps:

1. Reads the surface clip rectangle through vtable `+0x78`.
2. Builds/intersects the caller clip context via `AlphaShapeClass::ClipRect @ 0x00421B60`.
3. Adds the clip rectangle origin to both input endpoints.
4. Clips the segment through `FUN_007BC2B0 @ 0x007BC2B0`.
5. If clipping rejects the segment, returns `0` without locking or writing.
6. If accepted, normalizes left-to-right when the second x is less than the first x.
7. Locks the start pixel through surface vtable `+0x5C`.
8. Writes direct byte pixels for 8-bit surfaces or direct word pixels for non-8-bit surfaces.
9. Unlocks through surface vtable `+0x60` and returns `1`.

`FUN_007BC2B0` clips to inclusive maximums: left/top are the rect origin, and right/bottom are `rect.x + rect.w - 1` and `rect.y + rect.h - 1`. Evidence is the decompile/assembly around `0x007BC40A..0x007BC45C`, where bottom/right clipping subtracts one.

For axis-aligned lines, which are the only lines produced by the viewport rectangle wrapper:

- Horizontal edges write `x2 - x1 + 1` pixels after clipping. Evidence: `0x007BA6F3..0x007BA71A` for the 8-bit fill count, and `0x007BA72F..0x007BA758` for the 16-bit loop using `<=`.
- Vertical edges write `abs(y2 - y1) + 1` pixels after clipping. Evidence: `0x007BA779..0x007BA7BE`.
- Therefore the viewport rectangle and content-boundary rectangle are inclusive on all four clipped axis edges.

For non-axis use of `0x007BA610`, the helper switches to Bresenham-style integer loops. Those loops write the start pixel and iterate the dominant delta count, so the final endpoint is not written in the diagonal case. Evidence: `0x007BA7D3..0x007BA857` and `0x007BA85A..0x007BA89E`. This is documented only to prevent reusing the axis-rectangle conclusion for arbitrary lines.

### Color route and pixel write

Active in YR: Yes.

`SetSidebarTextColor @ 0x0072F440` selects side-specific RGB globals:

```text
side 0 -> DAT_00B0F9D8..DA
side 1 -> DAT_00B0FB04..06
else   -> DAT_00B0FAA0..A2
```

`RadarClass::Init_For_House @ 0x00652E90` packs the selected `DAT_00B0FA1C/FA1D/FA1E` RGB through the runtime DirectDraw loss/shift globals into `RadarClass+0x1208`.

`RadarClass::Update @ 0x00657652..0x006576A2` passes `RadarClass+0x1208` to both `g_SidebarSurface+0x58` calls. The line helper then writes that packed pixel directly to the surface:

- 8-bit path writes a byte fill/write.
- non-8-bit path writes `word ptr [dst] = color`.

Because standard YR radar/sidebar surfaces are 16-bit in the verified DirectDraw reports, the viewport outline is a direct packed-16 overwrite using the active sidebar text/border color. It is not hardcoded white, not RGBA, and not additive.

## 4. INI Keys

No radar-specific INI key directly controls this helper path. The side-specific color values are initialized through side UI setup and packed through DirectDraw globals; the exact RGB literals are not re-enumerated in this report.

## 5. Integration Points

| Integration | Behavior | Active in YR | Evidence |
|---|---|---|---|
| `RadarClass::Update -> g_SidebarSurface+0x58` | draws viewport rectangle and content-boundary rectangle after the primary radar blit | Yes | `0x00657652..0x006576A2` |
| `+0x58 -> 0x007BAD90 -> 0x007BADC0` | rectangle wrapper/worker | Yes for XSurface/BSurface | `0x007BAD90..0x007BAE5B`, vtable read |
| `0x007BADC0 -> +0x2C -> 0x007BA610` | four axis-aligned line calls | Yes | `0x007BAE10..0x007BAE50`, vtable read |
| `0x007BA610 -> 0x007BC2B0` | clipped direct-pixel line raster | Yes | `0x007BA61F..0x007BA690`, `0x007BC2B0` |
| `SetSidebarTextColor -> Init_For_House -> +0x1208` | side RGB selection and packed 16-bit color | Yes | `0x0072F440`, `0x00652E90` |

## 6. Current Rust Implementation Status

Current Rust still renders the viewport rectangle as a GPU sprite overlay:

- `src/render/minimap_helpers.rs` defines `VIEWPORT_LINE_THICKNESS = 2.0`.
- `src/render/minimap.rs::build_viewport_rect_in_rect` computes normalized world/map bounds in a `200x200` texture model.
- The four edges are independent white sprite instances with `tint: [1.0, 1.0, 1.0]`.
- `src/render/minimap_helpers.rs::draw_line` uses floating DDA with `0..=steps`; this is used for radar-event-like pixel lines, not for the current sprite viewport rectangle, but it does not match either native plain `0x007BA610` diagonal semantics or radar-event `0x004BDF00` semantics.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `RadarClass::Update` rectangle call sites | verified | `0x00657652..0x006576A2` | none |
| `XSurface/BSurface +0x58` identity | verified | vtable addresses `0x007E2104/0x007E2070` -> `0x007BAD90` | none |
| rectangle worker `0x007BADC0` | verified | assembly `0x007BADD7..0x007BAE50` | none |
| `XSurface/BSurface +0x2C` identity | verified | vtable addresses `0x007E2104/0x007E2070` -> `0x007BA610` | none |
| plain line clip helper | verified | `0x007BA610`, `0x007BC2B0` | arbitrary caller census not attempted |
| viewport rectangle axis endpoint inclusion | verified | `0x007BA6F3..0x007BA7BE` | none |
| non-axis `0x007BA610` endpoint behavior | touched-not-exhausted | `0x007BA7D3..0x007BA89E` | broader caller survey |
| side color packing | verified | `0x0072F440`, `0x00652E90` | actual side RGB byte table values not enumerated |
| current Rust viewport overlay | verified-current | `src/render/minimap.rs`, `src/render/minimap_helpers.rs` | implementation design/fix |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does the viewport rectangle use the radar-event line helper? -> No. It uses `g_SidebarSurface+0x58 -> 0x007BAD90 -> 0x007BADC0 -> +0x2C/0x007BA610`; radar-event lines use a different helper family.` (evidence: `0x00657652..0x006576A2`, `0x007BAD90..0x007BAE50`, `RADAR_LINE_RASTER_AND_DIRTY_CLIP_GATE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-2 - Are rectangle endpoints inclusive or exclusive? -> For the axis-aligned rectangle path they are inclusive after clipping.` (evidence: `0x007BA6F3..0x007BA7BE`)
- `[RESOLVED] OQ-3 - What clips the lines? -> `0x007BA610` gets the surface clip, intersects caller context, and calls `0x007BC2B0`, whose right/bottom maxima are `x+w-1` and `y+h-1`.` (evidence: `0x007BA61F..0x007BA690`, `0x007BC40A..0x007BC45C`)
- `[RESOLVED] OQ-4 - What is the viewport rectangle color? -> `RadarClass+0x1208`, packed from active sidebar text/border RGB through runtime DD shift/loss globals.` (evidence: `0x0072F440`, `0x00652E90`, `0x00657652`)
- `[RESOLVED] OQ-5 - Does the helper write additive/Z/A-buffer pixels? -> No for this path. `0x007BA610` writes direct byte/word pixels; additive/Z/A-buffer behavior belongs to radar-event `0x004BDF00`.` (evidence: `0x007BA6F3..0x007BA89E`, `RADAR_LINE_RASTER_AND_DIRTY_CLIP_GATE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-6 - Does the line helper mark radar dirty rectangles? -> No. It locks, writes, unlocks, and returns; dirty management is caller/owner-side.` (evidence: `0x007BA6D5..0x007BA8AE`)
- `[RESOLVED] OQ-7 - Is a two-pixel viewport line supported by native helper evidence? -> No. `0x007BADC0` emits one-pixel axis line segments; thickness is one surface pixel.` (evidence: `0x007BADC0..0x007BAE50`)
- `[DEFERRED] OQ-8 - What are every non-radar caller's expectations for `0x007BA610` diagonal lines?` (category: out-of-scope; reason: this report only claims the minimap viewport/content-boundary rectangle path; next-step-if-pursued: generic DSurface primitive caller census)
- `[DEFERRED] OQ-9 - What are the exact active side RGB bytes for all side themes?` (category: requires-different-system-context; reason: color selection and packing route are proven but asset/UI side data enumeration is a separate side-theme task; next-step-if-pursued: dump `DAT_00B0F9D8/FB04/FAA0` initialization sources)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | primary radar blit in `RadarClass::Update` | dirty rect has positive width/height | primary radar surface | accumulated rect `+0x120C..+0x1218` | packed surface blit | Yes when dirty | content copy |
| 2 | `g_SidebarSurface+0x58` | `+0x14B0 == 1 && +0x14AC == 1` | none | current viewport rect `+0x14DC..+0x14E8` | packed `+0x1208` direct line writes | Yes | camera-window overlay |
| 3 | `g_SidebarSurface+0x58` | same active content gate | none | `(origin_x-1, origin_y-1, content_w+2, content_h+2)` | packed `+0x1208` direct line writes | Yes | minimap content boundary |

Asset role matrix:

| Asset / primitive | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `g_SidebarSurface+0x58` rectangle primitive | n/a | Yes | Yes | No | boundary outline | Yes | No | No | `0x00657652..0x006576A2`, `0x007BAD90` |
| `0x007BA610` plain line primitive | n/a | Yes through rectangle | Yes | No | edge raster | Yes | No | No | `0x007BADC0..0x007BAE50` |
| radar-event `0x004BDF00` gradient line | n/a | Conditional for events | Not the viewport rectangle | No | No | RadarEvent only | No | inactive for viewport rectangle | `RADAR_LINE_RASTER_AND_DIRTY_CLIP_GATE_GHIDRA_REPORT.md` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Viewport/content-boundary rectangles are emitted through `+0x58`, expanded to four one-pixel axis lines, each clipped and drawn with inclusive endpoints. | `0x00657652..0x006576A2`, `0x007BADC0`, `0x007BA610` | mismatch: Rust emits 2-pixel white sprites in normalized screen space. | `src/render/minimap.rs::build_viewport_rect_in_rect`; future retained sidebar surface model | Draw a one-surface-pixel inclusive rectangle in native sidebar/minimap coordinates, not a 2-pixel GPU sprite box. | Move camera so the viewport rect touches each aperture edge; expected pixels are clipped to inclusive `x+w-1/y+h-1` bounds with one-pixel thickness. Proposed test: `minimap_viewport_rect_uses_plain_surface_inclusive_axis_edges`. | Do not apply radar-event final-end-exclusive semantics to the axis rectangle edges. |
| Rectangle color is `RadarClass+0x1208`, packed from active sidebar text/border RGB through runtime DD loss/shift globals. | `0x0072F440`, `0x00652E90`, `0x00657652` | mismatch: Rust hardcodes white tint. | sidebar color initialization and minimap overlay draw surfaces | Use the active side's packed 16-bit sidebar text/border color for both the camera window and content boundary. | Switch side theme and verify the outline packed pixel changes with native side text color while geometry remains unchanged. Proposed test: `minimap_viewport_outline_uses_active_sidebar_text_packed_color`. | Do not hardcode white or RGBA `[255,255,255]` unless it is the verified packed side color after conversion. |
| Plain rectangle helper writes direct byte/word pixels; no Z-buffer, A-buffer, additive blend, or radar-event dirty gate participates in the viewport rectangle path. | `0x007BA610`, `RADAR_LINE_RASTER_AND_DIRTY_CLIP_GATE_GHIDRA_REPORT.md` | mismatch risk: current helpers conflate radar-event DDA lines and viewport sprite lines. | `src/render/minimap_helpers.rs::draw_line`; future native DSurface primitive helper | Keep separate helper contracts for plain surface rectangles versus radar-event gradient lines. | Drawing the viewport border over a prior minimap pixel replaces that 16-bit pixel with the sidebar text packed color; a radar-event line in the same location follows the separate additive/Z-tested path. Proposed test: `minimap_plain_rect_overwrites_without_abuffer_or_ztest`. | Do not reuse radar-event additive line raster for the camera-window rectangle. |

## Negative Facts / Do Not Do

- Do not call `0x00660540` the ordinary camera-window viewport rectangle. Active in YR for that role: No; the active camera-window rectangle is drawn from `RadarClass::Update` through `g_SidebarSurface+0x58`.
- Do not apply radar-event `0x004BDF00` additive/Z/A-buffer semantics to the viewport rectangle. Active in YR for viewport rectangle: No; viewport rectangle uses `0x007BA610` direct stores.
- Do not treat the viewport rectangle as two pixels thick. Active in YR: No evidence; `0x007BADC0` emits one-pixel axis line segments.
- Do not make rectangle right/bottom exclusive for axis edges. Active in YR: No; axis edges write through inclusive clipped endpoints.
- Do not put dirty-rect or terrain-pixel invalidation inside `0x007BA610`; the helper locks/writes/unlocks only, while `RadarClass::Update` owns previous-border dirtying and sidebar copy invalidation.

## Remaining Uncertainty

- Exact side RGB byte origins for all sidebar themes are not enumerated here; only the selection and packing route are proven.
- A full non-radar caller census for `0x007BA610` is deferred. Non-axis lines in that helper appear start-inclusive/final-end-exclusive, but this report does not claim every caller's visual role.
- Exact runtime RGB555/RGB565 identity is handled by `DIRECTDRAW_LIVE_PIXEL_FORMAT_RUNTIME_SAMPLE_GHIDRA_REPORT.md`; this report only proves the viewport outline consumes the packed color route.

## Stale Docs / Follow-up Docs

`C:/Users/enok/Documents/ra2-rust-game/docs/research/RADAR_MINIMAP_RENDERING.md`

Replace the `## 10. Viewport Rectangle Drawing (0x00660540)` section heading and the claim that the viewport rectangle shares radar-event infrastructure with:

> The ordinary in-game camera-window rectangle is not `0x00660540` and does not use the radar-event gradient line path. `RadarClass::Update @ 0x00656EC0` draws the current viewport rectangle and the minimap content boundary on `g_SidebarSurface` through vtable `+0x58`, which resolves to `0x007BAD90 -> 0x007BADC0 -> +0x2C/0x007BA610` for XSurface/BSurface. The rectangle worker converts `{x,y,w,h}` to inclusive `right=x+w-1` and `bottom=y+h-1`, draws four one-pixel axis lines with inclusive clipped endpoints, and writes the packed active sidebar text/border color from `RadarClass+0x1208` directly to the 16-bit surface.

`C:/Users/enok/Documents/ra2-rust-game/docs/research/RADAR_MINIMAP_DEEP_DIVE.md`

Replace `## 4. Viewport Rectangle = Radar Event` with:

> RadarEventClass outline drawing and the ordinary camera-window rectangle are separate active paths. Radar events use `TickRadarEvent`/`DrawRadarEvent` helpers. The camera-window rectangle is owned by `RadarClass::Update`, stored at `RadarClass+0x14DC..+0x14E8`, and drawn after the primary radar blit through the plain sidebar-surface rectangle primitive.

`C:/Users/enok/Documents/ra2-rust-game/docs/research/RADAR_SYSTEM_COMPREHENSIVE.md`

Replace "Also draws corresponding viewport indicator lines" under `DrawRadarEvent` with:

> `DrawRadarEvent` draws radar-event outlines only. The ordinary camera-window indicator is a later `RadarClass::Update` sidebar-surface rectangle primitive using `g_SidebarSurface+0x58` and packed color `RadarClass+0x1208`.

## Sources

- Ghidra read-only decompile/assembly: `RadarClass::Update @ 0x00656EC0`, `RadarClass::Init_For_House @ 0x00652E90`, `SetSidebarTextColor @ 0x0072F440`, line helper `0x007BA610`, rectangle wrapper `0x007BAD90`, rectangle worker `0x007BADC0`, clip helper `0x007BC2B0`.
- Local retail PE vtable read: `vtable__XSurface @ 0x007E2104`, `vtable__BSurface @ 0x007E2070`.
- Prior reports: `RADAR_VIEWPORT_RECT_CAMERA_WINDOW_OVERLAY_GHIDRA_REPORT.md`, `RADAR_LINE_RASTER_AND_DIRTY_CLIP_GATE_GHIDRA_REPORT.md`, `DIRECTDRAW_LIVE_PIXEL_FORMAT_RUNTIME_SAMPLE_GHIDRA_REPORT.md`.
- Rust scan: `src/render/minimap.rs`, `src/render/minimap_helpers.rs`, `src/app_sidebar_render.rs`, `src/render/sidebar_chrome.rs`.

## Status

COMPLETE for the scoped viewport/content-boundary rectangle helper path: helper identity, clipping, axis endpoint inclusion, direct packed-pixel writes, side-color route, and Rust handoff are verified. Broader non-radar caller census and exact side RGB byte origins are intentionally deferred.
