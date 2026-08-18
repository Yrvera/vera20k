# Radar Event Ping Pixel Shapes Colors -- Ghidra Research Report

**Address(es):** `0x0065FA70`, `0x0065FB80`, `0x0065FDD0`, `0x0065FE00`, `0x00660000`, `0x00660050`, `0x006603B0`, `0x00660540`, `0x00660730`, `0x00656EC0`
**Investigation Mode:** exhaustive-slice for ordinary in-game RadarEventClass ping draw behavior; coverage-map for the general surface line-raster helper.
**Claimed Scope:** event diamond shape, color selection, per-frame tick/draw order, standard-YR drawable type liveness, and dirty/update integration for ordinary in-game radar events.
**Non-Scope:** generic terrain dirty pipeline, radar surface sizing/zoom sampling, radar transition movie lifecycle, spy-satellite reveal pixels, gap/shroud special effects, full surface line-raster internals, live DirectDraw RGB555/RGB565 mask sampling.
**Confidence:** High for event draw shape/color/order/timers from Ghidra; Medium for final per-pixel coverage inside the shared surface line helper because the helper was touched but not exhaustively reduced to a standalone raster algorithm.
**Active in YR:** Yes for RadarEventClass event queue and drawable types `0,1,2,3,4,5,11,12`; conditional for type `0` because stock engine callers do not pass it directly, but `TriggerAction::Execute` can.

## Summary

Ordinary radar event pings are generated event objects drawn as one rotating four-edge outline on the primary radar surface after terrain/object pixels and before spy-satellite overdraw. The native draw path is not a filled shape and not two separate diamonds. It computes four rotated corner points from the event radius and angle, offsets them by the radar pixel center, and draws four clipped line segments with bright/dim RGB triplets and native surface-line modulation.

Only explicit color-switch types draw visible minimap diamonds. Default-color event types, including bridge repair type `14` and impact/superweapon type `13`, still enter the event/ring/dedup system but skip the entire `DrawRadarEvent` block because their bright color resolves to black.

## Target and Non-Scope

Target:

- Verify ordinary in-game radar/minimap event ping pixel shape.
- Verify bright/dim color sources, drawable type set, and no-draw type set.
- Verify timing: event initialization, tick, phase transition, fade cadence, draw, cleanup.
- Verify dirty/update ordering relative to generated minimap content, object dots, and spy satellite.
- Include bridge/terrain-like event pings only where they use RadarEventClass.

Non-scope:

- Generic terrain dirtying and object-dot priority, already covered by sibling radar reports.
- Radar transition movies and MPSSCRN/SSCR lifecycle.
- Spy-satellite reveal shapes.
- Gap/shroud minimap interaction.
- Full reduction of the shared `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00` rasterizer.

Prior-state row that fired: a recent high-confidence `RADAR_EVENT_CLASS_GHIDRA_REPORT.md` existed, but it explicitly left pixel shapes/colors as a follow-up and contained stale/default-color wording in older sibling docs. This report proceeded as gap + verification only.

## Verified Binary Findings

### Entry points and integration

1. `RadarClass::Draw @ 0x00653100` is the ordinary in-game owner. It calls `FUN_0065FDD0` at `0x0065336D` before `RadarClass::Update`.
   - Active in YR: Yes, when not map editor and radar draw runs.
   - Why it matters: event state advances before the update pass that may draw/copy the primary radar surface.

2. `RadarClass::Update @ 0x00656EC0` calls `TickAndDrawRadarEvents @ 0x00660000` at `0x00657537` after dirty terrain/pixel/object rendering and before `DrawSpySatelliteVision`.
   - Evidence: decompile of `0x00656EC0`.
   - Ordered composition inside the update branch: clear/restore terrain background -> render dirty object/terrain pixels -> flush pixel dirty vector -> `TickAndDrawRadarEvents` -> `DrawSpySatelliteVision` -> active radar chrome/content blit to `g_SidebarSurface`.
   - Active in YR: Yes for active/open radar state.

3. `TickAndDrawRadarEvents @ 0x00660000` iterates the live event array in ascending index order from `0` to `g_RadarEventCount - 1`.
   - Evidence: decompile of `0x00660000`.
   - If two visible diamonds overlap, later array entries draw later and can overwrite earlier event-line pixels through the surface line helper.

### Event object fields used by drawing

| Offset | Field | Draw/tick role | Evidence |
|---:|---|---|---|
| `+0x00` | type | color switch and type-config index | `0x00660050`, `0x0065FE00` |
| `+0x04` | radar_x | center x relative to `g_RadarSurfaceOriginX` | `0x0065FB80`, `0x00660050` |
| `+0x08` | radar_y | center y relative to `g_RadarSurfaceOriginY` | `0x0065FB80`, `0x00660050` |
| `+0x0C` | radius float | diamond radius; starts at farthest edge distance and shrinks to min radius | `0x0065FB80`, `0x0065FE00`, `0x00660730` |
| `+0x10` | rotation_angle float | Z rotation input for corner generation | `0x0065FB80`, `0x0065FE00`, `0x00660730` |
| `+0x14` | rotation_speed float | added to angle and decelerated during phase 1 | `0x0065FE00` |
| `+0x18` | color_fade float | passed into `DrawRadarEvent` line helper; bounces 0.0..1.0 | `0x0065FE00`, `0x00660050` |
| `+0x1C` | fade_speed float | added to color_fade each tick; also participates in draw-line scale expression | `0x0065FE00`, `0x00660050` |
| `+0x20` | source_cell | ring/dedup source cell, low/high shorts | `0x0065FA70`, `0x0065FB80` |
| `+0x24/+0x2C` | timer1 start/duration | blink-duration cleanup timer | `0x0065FB80`, `0x006603B0` |
| `+0x30/+0x38` | timer2 start/duration | visible phase draw gating | `0x0065FE00`, `0x00660000` |
| `+0x3C` | expanding flag | phase-1 draw even before timer2; cleared at phase transition | `0x0065FE00`, `0x00660000` |
| `+0x3D` | needs_draw flag | tick early-out and cleanup eligibility | `0x0065FE00`, `0x006603B0` |

### Shape generation

4. `ComputeViewportCorners @ 0x00660730` is the event-corner generator used by both `DrawRadarEvent` and the sibling line draw at `0x00660540`.
   - It builds a rotation matrix from `event+0x10`, transforms vector `{event.radius, 0, 0}`, converts the resulting `x/y` through `Math__ftol`, then emits four corner offsets:
     - corner 0: `( dx,  dy)`
     - corner 1: `(-dy,  dx)`
     - corner 2: `(-dx, -dy)`
     - corner 3: `( dy, -dx)`
   - Evidence: decompile of `0x00660730`.

5. `DrawRadarEvent @ 0x00660050` offsets all four corners by `event.radar_x` and `event.radar_y`, then draws four consecutive line segments: `0->1`, `1->2`, `2->3`, `3->0`.
   - Evidence: loop in `0x00660050`, using `uVar7 = (uVar7 + 1)`, `uVar8 = uVar7 & 3`, and source pointer advanced by two ints per edge.
   - Shape is an outline. No separate native inner diamond was found in `DrawRadarEvent`.

6. The line segments go through the surface vtable line path (`*g_RadarDrawSurface + 0x78`, then `+0x90`) with bright color, dim color, a scale value derived from radius/edge length/fade speed, and `color_fade`.
   - Evidence: `0x00660050`.
   - The scale expression is:
     - `(((radius + radius) * DAT_007F0AF0) / max(abs(corner0.x-corner1.x), abs(corner0.y-corner1.y))) * event.fade_speed`
   - `event.color_fade` is passed separately.
   - Do not replace this with a sine-wave RGBA pulse without proving identical output.

7. Line clipping is surface-helper based, not an event-local rectangle clamp. The shared clipper `FUN_007BC2B0` is Cohen-Sutherland-style, using inclusive right/bottom boundaries internally via `rect.x + rect.w - 1` and `rect.y + rect.h - 1`.
   - Evidence: decompile of `FUN_007BC2B0 @ 0x007BC2B0`.
   - The broader line raster helper `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00` was touched; it performs surface rect clipping, A-buffer/Z-buffer gated writes, and DirectDraw-format channel operations. This report does not claim exhaustive line-raster replacement.

### Colors and visible type set

8. Bright color switch in `DrawRadarEvent @ 0x00660050`:

| Type(s) | Bright RGB | Dim RGB | Visible diamond? | Notes |
|---|---:|---:|---|---|
| `0,3,4` | `255,255,255` | `128,128,128` | yes | combat/base/harvester-style white |
| `1,2,11,12` | `255,255,0` | `128,128,0` | yes | noncombat/dropzone/beacon/construction yellow |
| `5` | `0,255,255` | `0,128,128` | yes | enemy object sensed cyan |
| default `6,7,8,9,10,13,14,15,16` | `0,0,0` | `0,0,0` | no | event remains in queue/ring but skips draw block |

9. The guard is on the bright color triple after construction: if all bright channels are zero, the entire corner/line draw block is skipped.
   - Evidence: `if ((((char)local_94 != 0) || (local_94._1_1_ != 0)) || (local_92 != 0))` in `0x00660050`.
   - Active in YR: Yes.

10. `FUN_004355B0` is only a three-byte RGB helper; it writes `param_2,param_3,param_4` into consecutive bytes.
    - Evidence: decompile of `0x004355B0`.
    - Packed 16-bit conversion happens later in the surface line path, using DirectDraw shift/loss globals, not through sidebar palettes.

11. Bridge repair type `14` does not draw a visible minimap diamond through RadarEventClass.
    - Evidence: `InfantryClass__PerCellProcess @ 0x00519BB6` calls `CreateRadarEvent(14, cell)` per prior xref set; type `14` falls into the default black branch at `0x00660050`.
    - Active in YR: Yes for bridge-repair EVA/ring behavior; no visible RadarEventClass ping.

12. Impact/superweapon type `13` does not draw a visible minimap diamond through RadarEventClass.
    - Evidence: call sites in `BulletClassAiHomingDetonationPath @ 0x00467EA7`, `LightningStorm::Start @ 0x00539F89`, and `SuperClass::Launch` call `CreateRadarEvent(13, ...)`; type `13` falls into default black branch.
    - Active in YR: Yes for queue/ring behavior; visible nuke/lightning effects must come from other renderers.

### Timing and clearing

13. `InitRadarEvent @ 0x0065FB80` initializes:
    - `rotation_angle = 0x3F490FDB` (about pi/4)
    - `rotation_speed = Rules+0x84` (`RadarEventRotationSpeed`, default `.05`)
    - `color_fade = 0`
    - `fade_speed = Rules+0x78` (`RadarEventColorSpeed`, default `.1`)
    - `timer1_start = current frame`, `timer1_duration = 0`
    - `timer2_start = current frame`, `timer2_duration = 0`
    - `expanding_flag = 1`, `needs_draw = 1`
    - initial radius = farthest distance to radar surface edge: max of left/top/right/bottom distances.

14. `TickRadarEvent @ 0x0065FE00` first honors `needs_draw`. If it is zero, the event does nothing that frame.

15. In non-expanding phase, `TickRadarEvent` uses `timer2_start/timer2_duration`; if expired, it sets `needs_draw = 0` and returns before updating radius/rotation/fade.

16. While still drawable, `TickRadarEvent` calls the sibling four-edge line function at `0x00660540` before changing radius/angle/fade. Older docs call this `DrawViewportRect`; in this path it is invoked once per drawable event before the event state changes.
    - Evidence: direct call from `0x0065FE3B`.
    - Interpretation: this is part of the retained-surface/update cadence for the old event geometry, not a separate Rust-style UI viewport overlay. Exact visual side effect of that helper remains tied to the shared line helper and dirty rect globals.

17. Radius shrinks by `Rules+0x80` (`RadarEventSpeed`, default `1.2`) and clamps to integer `Rules+0x7C` (`RadarEventMinRadius`, default `8`).

18. During expanding phase:
    - if `abs(radius - min_radius) >= 0.01`, angle advances by current rotation speed.
    - once radius is at min radius, rotation speed decays by `Rules.RadarEventRotationSpeed * 0.02` but not below `Rules.RadarEventRotationSpeed * 1/3`.
    - after the decel condition fails, `expanding_flag` clears and timers are loaded from `DAT_007F0998 + type*0x10`: visibility duration at `+4`, blink duration at `+8`.
    - Evidence: decompile of `0x0065FE00`; constants `_g_ImpassableSpeedThreshold_0_01`, `_DAT_007F0AE8`, `_DAT_007ED968`.

19. Rotation wraps by subtracting `2*pi` once when `rotation_angle > 2*pi`.
    - Evidence: `if ((float)_DAT_007e3cc0 < (float)param_1[4]) param_1[4] -= _DAT_007e3cc0`.

20. `color_fade` advances by `fade_speed`, bounces at `< 0.0` and `> 1.0`, flips the sign of `fade_speed`, and clamps the fade value exactly to `0.0` or `1.0`.
    - Evidence: tail of `0x0065FE00`.

21. `TickAndDrawRadarEvents @ 0x00660000` draws an event if its visible timer still has positive remaining time, or if `expanding_flag` is still set.
    - Evidence: branch to `DrawRadarEvent @ 0x00660039`.

22. `CleanupExpiredEvents @ 0x006603B0` removes events only after the blink timer is expired/zero and `expanding_flag == 0`; it iterates backwards, removes from the dynamic vector, shifts later entries left, then frees the 64-byte object.
    - Evidence: decompile of `0x006603B0`.

### Dirtying and composition

23. Radar event drawing does not call `RadarClass__MarkCellDirty @ 0x006562D0`. The MarkCellDirty path is for primary radar pixels/visited bitfield/list, not event lines.
    - Evidence: decompile of `0x00660050`, `0x00660540`, and `0x006562D0`.

24. Event-line drawing participates in radar/surface dirty rectangles through the surface/clip globals, including `DAT_008809F4`, `DAT_008809F8`, `DAT_008809FC`, and `DAT_00880A00`, but only through the branch gated by `DAT_00880C98 == 1 && DAT_00880C94 == 1`.
    - Evidence: tail of `0x00660050` and `0x00660540`.
    - Remaining uncertainty: exact runtime values and semantic names of `DAT_00880C98/94` were not proven in this slot.

25. The final sidebar-visible event pixels come from the primary radar surface `this+0x121C` being blitted into `g_SidebarSurface` by `RadarClass::Update`, not from a paletted sidebar SHP.
    - Evidence: `0x00656EC0` and sibling minimap pipeline report.

## Active in Standard YR?

Yes, conditionally by event type:

- Active and visible in ordinary YR: types `1,2,3,4,5,11,12` when their live call sites fire and radar is active/open.
- Active but stock-direct caller absent: type `0`, visible if spawned by trigger action data.
- Active but not visible through RadarEventClass: types `6,7,8,9,10,13,14,15,16`.
- Bridge repair: active standard-YR event/ring/EVA path, no visible RadarEventClass diamond.
- Bullet impact/superweapon launch: active standard-YR event/ring path as type `13`, no visible RadarEventClass diamond.

No TS-only gate was found for the RadarEventClass renderer itself. Some callers are scenario/content conditional, but the queue, tick, and renderer are live YR code.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native ping is one rotated four-edge outline generated from `(dx,dy),(-dy,dx),(-dx,-dy),(dy,-dx)` and offset by radar pixel center | `0x00660730`, `0x00660050` | mismatch: Rust draws an outer diamond plus an extra inner dim diamond | `src/render/minimap.rs` | Draw only the native four edge segments through a native-equivalent line/raster path | A type 3 base-under-attack event at center produces one outline, not a filled diamond and not a second inner outline | Do not add a decorative inner diamond or filled polygon |
| Drawable color switch is `{0,3,4}=white`, `{1,2,11,12}=yellow`, `{5}=cyan`, default black skip | `0x00660050` | mismatch: Rust maps Dropzone cyan and Miner/Enemy yellow; it has only seven enum variants | `src/sim/radar.rs`, `src/render/minimap.rs` | Expand event type table and use native draw/no-draw switch | Type 5 enemy-object-sensed draws cyan; type 4 harvester-under-attack draws white; type 14 bridge repaired enqueues but draws nothing | Do not infer colors from ModEnc labels or EVA labels |
| Color pulse is stateful `color_fade += fade_speed` with sign flip at bounds, not sine-wave age math | `0x0065FE00`, `0x00660050` | mismatch: Rust uses `sin(abs(age * speed))` pulse | `src/sim/radar.rs`, `src/render/minimap.rs` | Store `color_fade` and signed `fade_speed`, bounce at 0/1, pass fade to draw | Two pings of equal age but different prior bounce state match native fade values | Do not derive brightness solely from age modulo |
| Initial radius is farthest radar-edge distance and shrinks by `RadarEventSpeed` to `RadarEventMinRadius` | `0x0065FB80`, `0x0065FE00` | mismatch: Rust starts at `min_radius * 4` | `src/sim/radar.rs`, `src/render/minimap.rs` | Initialize from generated radar surface dimensions and event center | Event near a corner starts with larger radius than centered event and contracts from off/aperture edge | Do not use a fixed multiple of min radius |
| Event draw order is after terrain/object pixel restoration and before spy satellite overlay, then primary radar surface blits to retained sidebar surface | `0x00656EC0`, `0x00660000` | mismatch risk: Rust draws minimap texture with events inside RGBA refresh path, not retained primary/sidebar surface order | `src/render/minimap.rs`, `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs` | Compose generated minimap primary surface in native order before sidebar copy | Overlapping object dot, event line, and spy satellite pixel resolves object < event < spy satellite | Do not draw event pings as final UI overlay above spy-satellite/sidebar tooltip layers |
| Event dirtying is not `MarkCellDirty`; event line update uses surface/clip dirty rect path and retained radar surface cadence | `0x00660050`, `0x00660540`, `0x006562D0` | mismatch: Rust full-refreshes/writes RGBA texture | `src/render/minimap.rs`, future retained radar surface model | Preserve event-line old/new geometry redraw and accumulated primary-surface dirty rect before sidebar blit | A single moving event only copies the native affected radar/sidebar rect, not the whole minimap texture | Do not hide dirty cadence behind a full-frame texture rewrite when targeting flicker parity |
| Type `13` impact/superweapon events are silent in RadarEventClass renderer | `0x00660050`, xrefs to `0x0065FA70` | mismatch: Rust currently pushes Combat-visible events from weapon fire paths | `src/sim/world/mod.rs`, `src/sim/combat/mod.rs`, `src/sim/radar.rs` | Weapon impacts should enqueue native type `13` semantics, not visible type `0` diamonds | Firing a normal weapon updates Spacebar/ring behavior as native but does not show a white minimap diamond | Do not equate "combat" INI label with bullet-impact visible ping |

## Negative Facts / Do Not Do

- Do not draw a filled diamond for RadarEventClass pings.
- Do not draw a separate inner dim diamond; native `DrawRadarEvent` draws four line segments through one surface line path.
- Do not render default event types as yellow. Types `6,7,8,9,10,13,14,15,16` skip visible drawing in `DrawRadarEvent`.
- Do not make bridge repair show a minimap diamond through RadarEventClass.
- Do not make bullet impacts or SuperClass launch type `13` show a RadarEventClass diamond.
- Do not color pings through `SIDEBAR.PAL`, `CAMEO.PAL`, `OBSERVER.PAL`, or `radar.shp`.
- Do not replace the native fade bounce with sine/abs/time-based pulse.
- Do not treat event drawing as `RadarClass__MarkCellDirty`; it is a separate line/clip surface path.
- Do not place event pings above spy-satellite reveal pixels unless a separate spy-sat slot proves otherwise.

## Remaining Uncertainty

- The exact final raster coverage and A/Z-buffer modulation inside `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00` were touched, not exhaustively reduced. A future surface-line report should isolate this helper if byte-identical line pixels are required independently of radar events.
- Live runtime DirectDraw channel masks are still pending the adjacent pixel-format slot. This report verifies RGB triplets and packed-surface routing, not the final RGB555/RGB565 runtime identity.
- Semantic names/default runtime values for `DAT_00880C98` and `DAT_00880C94` were not proven. The gated clip/dirty global writes are real.
- The exact visible effect of `0x00660540` as called at the start of every event tick needs a runtime capture or deeper surface-helper analysis to distinguish old-geometry erase/dirty behavior from a visible line pass in all cases.
- TriggerAction dynamic type values were not enumerated from map trigger data; type `0` visibility is conditional on such external data.

## Stale-Doc Replacement Wording

Replace older wording that says default event types draw yellow with:

> `DrawRadarEvent @ 0x00660050` only draws visible diamonds for explicit color cases: types `0,3,4` white, `1,2,11,12` yellow, and `5` cyan. Default types `6,7,8,9,10,13,14,15,16` construct black bright/dim colors and then skip the entire draw block. They can still enqueue, deduplicate, gate EVA return paths, and populate the Spacebar event ring.

Replace Rust-facing wording that describes the native ping as two diamonds with:

> Native RadarEventClass pings are one rotated four-edge outline. Corners come from `ComputeViewportCorners @ 0x00660730`; `DrawRadarEvent @ 0x00660050` draws the four consecutive edges through the surface line helper with bright/dim color inputs and event fade state. No separate inner dim diamond or filled polygon appears in this function.

Replace wording that treats bullet impacts as visible combat pings with:

> Stock bullet impact and several superweapon launch callers pass type `13`, which is a default black/no-draw RadarEventClass type. It remains relevant to event/ring behavior but is not the source of visible minimap event diamonds.

## Status

COMPLETE for the scoped Ghidra slice of RadarEventClass visible diamond shape, drawable type colors, phase timing, and composition order.

PARTIAL only for the generic surface line rasterizer internals, live DirectDraw masks, and non-RadarEventClass special radar effects.

## Sources

- Ghidra read-only decompilation: `CreateRadarEvent @ 0x0065FA70`, `InitRadarEvent @ 0x0065FB80`, `FUN_0065FDD0`, `TickRadarEvent @ 0x0065FE00`, `TickAndDrawRadarEvents @ 0x00660000`, `DrawRadarEvent @ 0x00660050`, `CleanupExpiredEvents @ 0x006603B0`, `DrawViewportRect/sibling line draw @ 0x00660540`, `ComputeViewportCorners @ 0x00660730`, `RadarClass::Draw @ 0x00653100`, `RadarClass::Update @ 0x00656EC0`, `RadarClass::MarkCellDirty @ 0x006562D0`, `FUN_004355B0`, `FUN_007BC2B0`, touched `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00`.
- Existing docs: `docs/research/RADAR_EVENT_CLASS_GHIDRA_REPORT.md`, `docs/research/MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `docs/research/RADAR_OBJECT_DOT_PRIORITY_VISIBILITY_GATES_GHIDRA_REPORT.md`, `docs/research/RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE_GHIDRA_REPORT.md`, `docs/research/RADAR_MINIMAP_RENDERING.md`, `docs/research/RADAR_MINIMAP_DEEP_DIVE.md`.
- INI checked: `ini/rulesmd.ini` radar-event keys around lines `451-470`; base `ini/rules.ini` equivalent comments/keys.
- Rust scan: `src/sim/radar.rs`, `src/render/minimap.rs`, `src/rules/radar_event_config.rs`, `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs`.
