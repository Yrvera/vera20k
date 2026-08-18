# Radar Viewport Rect Camera Window Overlay -- Ghidra Research Report

**Address(es):** `RadarClass__Update @ 0x00656EC0`, `RadarClass__Init_For_House @ 0x00652E90`, `DSurface::DrawRect wrapper @ 0x007BAD90 -> 0x007BADC0`, `FUN_006D6590 @ 0x006D6590`  
**Investigation Mode:** exhaustive-slice for the ordinary in-game minimap camera-window overlay inside `RadarClass::Update`.  
**Claimed Scope:** calculation of the camera-window radar rectangle, previous/current fields, dirtying of old border pixels, draw order relative to generated content, radar events, and spy satellite, line primitive/color source, and current Rust deltas.  
**Non-Scope:** minimap click provenance, object-dot gates, radar-event ping shapes except as ordering context, spy-satellite reveal geometry, gap/shroud effects, and full generic line-raster internals.  
**Confidence:** High for rect math, field writes, dirtying, order, and color route; Medium for final one-pixel line raster internals because generic `DSurface+0x2C` was not exhausted.  
**Active in YR:** Yes. Ordinary sidebar draw reaches `PowerClass::Draw -> RadarClass::Draw -> RadarClass::Update`; visible overlay branch requires `RadarClass+0x14B0 == 1 && RadarClass+0x14AC == 1`.

## Target Question

What does native YR draw for the ordinary minimap camera-window overlay, how is its rectangle computed/dirtied, and where does current Rust drift?

## Non-Goals

- Do not redo minimap click/drag provenance.
- Do not redo RadarEventClass ping shapes/colors except ordering context.
- Do not investigate spy satellite or gap-generator geometry.
- Do not modify Rust.

## Evidence Needed To Mark Complete

- Active YR entry point for the overlay.
- Exact current/previous rect fields and formulas.
- Old/new border dirtying and dirty-rect effects.
- Draw order against content, radar events, and spy satellite.
- Line primitive and color source.
- Rust handoff and concrete tests.

## Stop Conditions

Stop once `RadarClass::Update`, the DSurface rectangle wrapper, and color initializer are verified. Defer only generic `DSurface+0x2C` line raster details and runtime DD-mask sampling to sibling/shared reports.

## 1. Overview

The ordinary camera-window overlay is not `DrawViewportRect @ 0x00660540`. That function is the RadarEventClass old-geometry/four-edge helper called from `TickRadarEvent`. The actual camera-window overlay is owned by `RadarClass::Update @ 0x00656EC0`: it computes a sidebar-local rect at `RadarClass+0x14DC..0x14E8`, dirties the previous border through `MarkCellDirty`, blits generated minimap content to `g_SidebarSurface`, then draws the current outline through `g_SidebarSurface` vtable `+0x58`.

## 2. Class Layout / Key Offsets

| Owner | Offset | Type | Purpose | Active in YR | Evidence |
|---|---:|---|---|---|---|
| `RadarClass` | `+0x1208` | packed 16-bit color | camera-window/content-border rectangle color | Yes | `0x00652E90`, `0x00657652` |
| `RadarClass` | `+0x120C..+0x1218` | rect `{x,y,w,h}` | accumulated sidebar-local dirty/content blit rect | Yes | `0x00656EC0` |
| `RadarClass` | `+0x121C` | surface ptr | primary/live minimap surface | Yes | `0x00656EC0` |
| `RadarClass` | `+0x1488` | float | radar zoom factor | Yes | `0x00656EC0`, surface-sizing sibling |
| `RadarClass` | `+0x1490/+0x1498` | ints | projection offsets for camera-window center | Yes | `0x00656F61..0x00656FC8` |
| `RadarClass` | `+0x149C/+0x14A0` | ints | minimap content origin in sidebar surface | Yes | `0x00656F6B`, `0x00656FC2` |
| `RadarClass` | `+0x14A4/+0x14A8` | ints | generated content width/height clamp bounds | Yes | `0x006570BB..0x00657117` |
| `RadarClass` | `+0x14DC..+0x14E8` | rect `{x,y,w,h}` | current camera-window overlay rect | Yes | `0x00657004..0x00657117`, `0x0065765F..0x00657668` |
| `RadarClass` | `+0x14EC..+0x14F8` | rect `{x,y,w,h}` | previous camera-window overlay rect | Yes | `0x0065713F..0x00657167`, `0x00657810..0x00657834` |
| Globals | `0x00886FA0/+A4/+A8/+AC` | ints | tactical viewport left/top/width/height | Yes | `0x00656F12..0x00656F4E`, `0x00657013..0x0065708E` |

## 3. Core Logic

### Activation and Center Cell

Active in YR: Yes. Evidence: `RadarClass::Update @ 0x00656EC0`.

The visible overlay branch is gated by:

```text
active_content = (RadarClass+0x14B0 == 1 && RadarClass+0x14AC == 1)
```

The camera-window center is derived from tactical viewport center, not from minimap input:

```text
screen_x = g_RadarViewportOffsetX + g_RadarViewportWidth / 2
screen_y = g_RadarViewportOffsetY + g_RadarViewportHeight / 2
center_cell = FUN_006D6590(g_Tactical, &out_cell, &screen_point)
cell_x = low 16 bits of CellClass+0x24
cell_y = high 16 bits of CellClass+0x24
```

The half-width/height calculation uses the native signed x86 `CDQ/SAR` pattern at `0x00656F12..0x00656F3E`.

### Current Rect Position

Active in YR: Yes. Evidence: `0x00656F5E..0x00656FC8`.

```text
center_x = ftol(((RadarClass+0x1490 - cell_y + cell_x) * zoom) + origin_x)
center_y = ftol(((cell_y - RadarClass+0x1498 + cell_x) * zoom) + origin_y)

if center_x == origin_x - 1:
    center_x += 1
```

`origin_x/origin_y` are `+0x149C/+0x14A0`, and `zoom` is `+0x1488`. The `origin_x - 1` fix exists only for x before size-centering and clamp.

### Current Rect Size and Centering

Active in YR: Yes. Evidence: `0x00656FE3..0x006570AA`. Local retail `gamemd.exe` float read confirms `0x007F046C = 60.0f`, `0x007F0468 = 30.0f`, `0x007E2AC8 = 1.0f`, `0x007E5168 = 0.5f`.

```text
denom_x = 60.0 / zoom
denom_y = 30.0 / zoom

rect_w = ftol((g_RadarViewportWidth  * 2) / denom_x + 1.0)
rect_h = ftol((g_RadarViewportHeight * 2) / denom_y)

rect_x = center_x - ftol(g_RadarViewportWidth / denom_x)
rect_y = center_y - ftol(((g_RadarViewportHeight * 2) / denom_y) * 0.5)
```

The y half-offset recomputes the float expression and multiplies by `0.5`; it is not proven identical to `rect_h / 2` for every FPU rounding case.

### Clamp

Active in YR: Yes. Evidence: `0x006570AD..0x00657117`.

```text
if rect_x < origin_x:
    rect_x = origin_x
else if origin_x + content_w <= rect_x + rect_w:
    rect_x = origin_x + content_w - 1 - rect_w

if rect_y < origin_y:
    rect_y = origin_y
else if origin_y + content_h <= rect_y + rect_h:
    rect_y = origin_y + content_h - 1 - rect_h
```

The right/bottom clamp uses `<=` and the `-1 - size` adjustment.

### Previous Rect and Dirtying

Active in YR: Yes. Evidence: `0x0065713F..0x00657253`, `0x00657810..0x00657834`.

If force redraw `+0x14DA` is set, previous rect `+0x14EC..+0x14F8` is overwritten with current rect before change testing.

If previous/current differ, native dirties only the previous border pixels on the primary radar surface before final pixel recomposition:

```text
for y in 0 .. prev_h-1:
    MarkCellDirty(prev_x - origin_x,              prev_y + y - origin_y)
    MarkCellDirty(prev_x + prev_w - 1 - origin_x, prev_y + y - origin_y)

for x in 0 .. prev_w-1:
    MarkCellDirty(prev_x + x - origin_x, prev_y - origin_y)
    MarkCellDirty(prev_x + x - origin_x, prev_y + prev_h - 1 - origin_y)
```

This erases/restores the old overlay through normal `RenderCellPixel`. The new/current border is drawn directly on `g_SidebarSurface` after the primary-surface blit. After direct drawing, if the viewport moved, `Update` also unions the accumulated sidebar-local dirty rect with previous and current rects using native inclusive `+1` merge behavior, then copies current into previous fields.

### Draw Order

Active in YR: Yes. Evidence: `0x0065730E..0x006576A2`.

Order inside an active update pass:

1. `RadarClass__ClearBackground`.
2. Dirty previous viewport border pixels if previous/current rect differ.
3. Apply terrain dirty generated-surface rect and rerender affected final pixels.
4. Process explicit pixel dirty list back-to-front with `RenderCellPixel`.
5. Clear pixel dirty vector.
6. `TickAndDrawRadarEvents`.
7. `DrawSpySatelliteVision`.
8. If force redraw, draw `BKGDLG/BKGDLGY` frame `0x20` chrome to `g_SidebarSurface`.
9. Blit accumulated dirty rect from primary radar surface `+0x121C` to `g_SidebarSurface`.
10. Draw current camera-window rect with `g_SidebarSurface+0x58`.
11. Draw expanded content boundary rect `(origin_x-1, origin_y-1, content_w+2, content_h+2)` with `g_SidebarSurface+0x58`.
12. Update old/new dirty union if moved, copy current rect to previous, cleanup expired events.

The camera-window overlay is above generated minimap content, radar events, and spy-satellite overlay. It is not part of the primary radar surface.

### Primitive and Color Source

Active in YR: Yes. Evidence: `0x00652E90`, `0x00657652..0x006576A2`, `0x007BAD90`, `0x007BADC0`, `SetSidebarTextColor @ 0x0072F440`.

`RadarClass__Init_For_House @ 0x00652E90` packs `DAT_00B0FA1C/FA1D/FA1E` through runtime DirectDraw loss/shift globals into `RadarClass+0x1208`. `SetSidebarTextColor @ 0x0072F440` selects those source RGB globals from side-specific sidebar text-color globals. The outline is therefore active sidebar text/border color, not hardcoded white.

`g_SidebarSurface` vtable `+0x58` resolves through `0x007BAD90`, which calls worker `0x007BADC0`. The worker converts `{x,y,w,h}` to inclusive endpoints:

```text
right  = x + w - 1
bottom = y + h - 1
```

It then draws four one-pixel line edges through surface vtable `+0x2C`.

## 4. INI Keys

No INI key directly controls this overlay rectangle. The color route depends on active side/sidebar color initialization, not a radar-specific INI key in this slice. `RadarEvent*` rules are out-of-scope except as event ordering context.

## 5. Integration Points

| Integration | Behavior | Active in YR | Evidence |
|---|---|---|---|
| `RadarClass::Draw -> RadarClass::Update` | ordinary in-game owner | Yes | `0x00653100`, `0x00656EC0` |
| `RadarClass::Update -> FUN_006D6590` | converts tactical viewport center to cell | Yes | `0x00656F42..0x00656F5E` |
| `RadarClass::Update -> MarkCellDirty` | marks previous border only when rect changed | Yes | `0x006571C0..0x00657253` |
| `RadarClass::Update -> g_SidebarSurface+0x58` | draws current camera-window and expanded content boundary | Yes | `0x00657652..0x006576A2` |
| `DSurface+0x58 -> 0x007BAD90 -> 0x007BADC0` | rectangle outline via four line calls | Yes | assembly/decompile |

## 6. Current Rust Implementation Status

Rust currently implements a sprite overlay in `src/render/minimap.rs::build_viewport_rect_in_rect`. It computes normalized world-space `left/top/right/bottom` from `camera_x/y` and `screen_w/h`, clamps in a `200x200` texture model, then emits four white textured rectangles with `VIEWPORT_LINE_THICKNESS = 2.0`.

Current Rust deltas:

- Uses normalized world bounds and `MINIMAP_SIZE=200`, not native tactical-center-cell projection into the `<=140x108` content rect.
- Draws hardcoded white, 2-pixel-thick GPU sprite lines, not active sidebar text/border color via DSurface `+0x58`.
- Draws as an independent UI overlay, not retained-sidebar-surface order after primary content blit.
- Does not dirty old border pixels through native `MarkCellDirty` or maintain previous/current rect fields.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `RadarClass::Update` overlay rect calculation | verified | `0x00656EC0`, `0x00656F12..0x00657117` | none |
| Previous rect old-border dirtying | verified | `0x0065713F..0x00657253` | none |
| Draw order inside update pass | verified | `0x0065730E..0x006576A2` | none |
| `+0x1208` color initialization | verified | `0x00652E90`, `0x0072F440` | runtime side RGB values not enumerated |
| DSurface `+0x58` rectangle wrapper | verified | `0x007BAD90`, `0x007BADC0` | generic `+0x2C` line raster deferred |
| `0x00660540` relation | verified-negative for camera-window overlay | `0x0065FE00`, `0x00660540`, `0x00656EC0` | none |
| Rust `build_viewport_rect_in_rect` | verified-current | `src/render/minimap.rs` | future implementation design |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- Is the visible camera-window overlay drawn by 0x00660540? -> No. 0x00660540 is called from RadarEvent tick; camera-window overlay is `RadarClass::Update -> g_SidebarSurface+0x58` after content blit.` (evidence: `0x0065FE00`, `0x00660540`, `0x00657652..0x00657668`)
- `[RESOLVED] OQ-2 -- What drives camera-window position? -> Tactical viewport center converted to a cell by `FUN_006D6590`, then projected through radar offsets/zoom.` (evidence: `0x00656F12..0x00656FC8`)
- `[RESOLVED] OQ-3 -- What constants drive size? -> `60.0f`, `30.0f`, `1.0f`, and `0.5f`.` (evidence: local binary float read plus assembly `0x00656FE3..0x0065708E`)
- `[RESOLVED] OQ-4 -- Does native mark old or new border dirty? -> Old/previous border is marked dirty before final pixel recomposition; current border is drawn directly after sidebar blit.` (evidence: `0x0065713F..0x00657253`, `0x00657652..0x00657668`)
- `[RESOLVED] OQ-5 -- What is the color source? -> Packed active sidebar text/border RGB stored at `+0x1208`, not hardcoded white.` (evidence: `0x00652E90`, `0x0072F440`, `0x00657652`)
- `[RESOLVED] OQ-6 -- What clips/clamps the rect? -> Native content-rect clamp with `<=` right/bottom tests and `-1 - size` adjustment.` (evidence: `0x006570AD..0x00657117`)
- `[RESOLVED] OQ-7 -- Is line thickness 2 px? -> No evidence for 2 px; DSurface rectangle wrapper draws four one-pixel line calls via `+0x2C`.` (evidence: `0x007BADC0`)
- `[DEFERRED] OQ-8 -- Exact generic `DSurface+0x2C` line raster coverage.` (category: bounded-cost-too-high; reason: shared surface-line helper is broader than camera-window overlay; next-step-if-pursued: standalone DSurface line raster report)
- `[DEFERRED] OQ-9 -- Runtime RGB555/RGB565 masks on user machine.` (category: needs-runtime-debugger; reason: sibling pixel-format report marks live descriptor sampling partial; next-step-if-pursued: runtime DD surface descriptor sample)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `RadarClass::Update -> ClearBackground` | update condition true | generated terrain surface | dirty terrain rect | packed surface | Yes | restore terrain base |
| 2 | previous-border dirty loop | previous/current rect differ | none | previous `+0x14EC..+0x14F8` border | n/a | Conditional | erase old overlay |
| 3 | dirty terrain/pixel rerender | dirty rect/list positive | none | primary radar pixels | `RenderCellPixel` packing | Conditional | content |
| 4 | `TickAndDrawRadarEvents` | event queue active | none | primary radar surface | event RGB line path | Conditional | event overlay |
| 5 | `DrawSpySatelliteVision` | spy satellite update/draw condition | spy-sat SHP, sibling scope | primary radar surface | sibling scope | Conditional | late primary overlay |
| 6 | `CC_Draw_Shape(DAT_00B04A38, 0x20)` | `+0x14DA` force redraw | `BKGDLG/BKGDLGY` frame `0x20` | `+0x11E4/+0x11EC`, normally `(0,48)` | sidebar convert route | Conditional | chrome refresh |
| 7 | `g_SidebarSurface+0x08` | dirty rect positive | primary surface `+0x121C` | `+0x120C..+0x1218` dest | surface blit | Yes when dirty | minimap content copy |
| 8 | `g_SidebarSurface+0x58` | active content | none | current camera-window `+0x14DC..+0x14E8` | packed `+0x1208` | Yes | camera-window overlay |
| 9 | `g_SidebarSurface+0x58` | active content | none | `(origin_x-1, origin_y-1, content_w+2, content_h+2)` | packed `+0x1208` | Yes | content boundary outline |

Asset role matrix:

| Asset / primitive | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| Generated primary radar surface `+0x121C` | Yes | Yes | Yes | Yes | No | Contains events/spy before copy | No | No | `0x00656EC0` |
| `BKGDLG/BKGDLGY` frame `0x20` | Yes | Conditional | Conditional | No | Yes | No | No | No | `0x0065758E..0x006575FC` |
| DSurface rectangle primitive `+0x58` | n/a | Yes | Yes | No | Boundary for aperture | Yes | No | No | `0x00657652..0x006576A2`, `0x007BADC0` |
| `0x00660540` event line helper | n/a | Conditional for events | Not the camera-window overlay | No | No | RadarEvent geometry | No | Not camera window | `0x0065FE00`, `0x00660540` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Camera-window rect is projected from tactical viewport center cell through radar offsets/zoom, with native `60/30` constants, `Math__ftol`, `+1` width, special `origin_x-1` x fix, and content-rect clamp. | `0x00656F12..0x00657117` | mismatch: Rust uses normalized world-space rect over a `200x200` texture. | `src/render/minimap.rs::build_viewport_rect_in_rect`, future native radar surface model | Compute/store `+0x14DC..+0x14E8` equivalent in sidebar-local radar content coordinates. | `minimap_viewport_rect_matches_native_projection_and_clamp`: same camera center near each map edge yields native `x/y/w/h`. | Do not derive the outline from texture-normalized `camera_x/screen_w` alone. |
| Old/previous border pixels are dirtied through `MarkCellDirty` before recomposition; current border is drawn directly after sidebar content blit. | `0x0065713F..0x00657253`, `0x00657652..0x00657668` | missing: Rust has no previous/current overlay dirty lifecycle. | `src/render/minimap.rs`, future retained sidebar/minimap surface update path | Maintain previous rect, dirty old border pixels, recompose primary surface, then draw current outline on sidebar surface. | `minimap_viewport_move_erases_old_border_via_pixel_dirty_before_new_outline`. | Do not just overdraw a new white rectangle each frame. |
| Overlay uses packed active sidebar text/border color via `+0x1208` and DSurface `+0x58` one-pixel outline, not hardcoded white 2 px sprites. | `0x00652E90`, `0x0072F440`, `0x007BAD90`, `0x007BADC0` | mismatch: Rust uses white texture and `VIEWPORT_LINE_THICKNESS = 2.0`. | `src/render/minimap.rs`, sidebar color/DirectDraw packing surfaces | Use native packed color route and one-pixel inclusive outline endpoints. | `minimap_viewport_outline_uses_sidebar_text_color_one_pixel`. | Do not hardcode white or 2-pixel thickness. |

## Negative Facts / Do Not Do

- Do not treat `DrawViewportRect @ 0x00660540` as the ordinary camera-window overlay. Active in YR: No for that role; it is called from RadarEvent tick.
- Do not draw the camera-window outline before radar events or spy satellite. Native order puts it after primary surface copy to `g_SidebarSurface`.
- Do not use a generic playable-area clamp. Native clamps to generated minimap content rectangle with `origin + size <= rect + rect_size` and `origin + size - 1 - rect_size`.
- Do not draw a 2-pixel white rectangle for parity. Native uses four one-pixel surface lines with packed active sidebar text/border color.
- Do not dirty current/new border pixels through `MarkCellDirty`; native dirties previous border for erasure and draws current border directly.

## Remaining Uncertainty

- Exact generic `DSurface+0x2C` line raster coverage is deferred to a shared surface-line report.
- Runtime RGB555/RGB565 descriptor identity remains delegated to the pixel-format slot; this report proves the color source and packing route.
- Runtime side-specific RGB values behind `DAT_00B0F9D8/FB04/FAA0` were not enumerated; `SetSidebarTextColor` selection and `+0x1208` packing were verified.

## Stale Docs / Follow-up Docs

`docs/research/RADAR_MINIMAP_RENDERING.md`

Replace the `## 10. Viewport Rectangle Drawing (0x00660540)` section heading and first paragraph with:

> `0x00660540` is the RadarEventClass four-edge line helper called from `TickRadarEvent`, not the ordinary in-game camera-window overlay. The camera-window overlay is computed in `RadarClass::Update @ 0x00656EC0` into `RadarClass+0x14DC..+0x14E8`, dirty-erases the previous border through `MarkCellDirty`, then after generated content/radar events/spy-satellite and primary-surface blit, draws the current outline on `g_SidebarSurface` through vtable `+0x58` using the packed active sidebar text/border color at `RadarClass+0x1208`.

`docs/research/RADAR_MINIMAP_DEEP_DIVE.md`

Replace "Viewport Rectangle = Radar Event" with:

> Radar events and the camera-window overlay are separate in the ordinary in-game minimap path. RadarEventClass uses `0x00660540`/`0x00660730` during event ticking. The camera-window overlay is owned by `RadarClass::Update @ 0x00656EC0`, which computes a sidebar-local rect from the tactical viewport center cell, draws it with `g_SidebarSurface+0x58`, and maintains previous/current rect fields for dirty erasure.

`docs/research/RADAR_SYSTEM_COMPREHENSIVE.md`

Replace "Draws 4 lines forming the diamond shape on the radar surface / Also draws corresponding viewport indicator lines" with:

> RadarEventClass line drawing and the camera-window overlay are not the same visible primitive. The camera-window overlay is a later `RadarClass::Update` sidebar-surface rectangle primitive using `+0x14DC..+0x14E8` and `+0x1208`.

## Sources

- Ghidra read-only decompile/assembly: `RadarClass__Update @ 0x00656EC0`, `RadarClass__Init_For_House @ 0x00652E90`, `SetSidebarTextColor @ 0x0072F440`, `FUN_006D6590 @ 0x006D6590`, `TickRadarEvent @ 0x0065FE00`, `DrawViewportRect @ 0x00660540`, DSurface rectangle wrapper `0x007BAD90`, worker `0x007BADC0`.
- Local retail binary float read from `<ra2-install>/gamemd.exe`: `0x007F046C = 60.0f`, `0x007F0468 = 30.0f`, `0x007E2AC8 = 1.0f`, `0x007E5168 = 0.5f`.
- Existing docs referenced: `RADAR_EVENT_PING_PIXEL_SHAPES_COLORS_GHIDRA_REPORT.md`, `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`, `TOOLTIP_MANAGER_SIDEBAR_OVERLAP_PIXELS_GHIDRA_REPORT.md`.
- Rust scan: `src/render/minimap.rs`, `src/render/minimap_helpers.rs`.

## Status

COMPLETE for the scoped camera-window overlay calculation, dirtying, draw order, primitive, color route, and Rust handoff. Partial only for generic surface-line raster internals and live DD mask runtime sampling, which are sibling/shared targets.
