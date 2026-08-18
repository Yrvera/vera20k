# Minimap Click/Drag Inverse Transform - Ghidra Research Report

**Address(es):** `0x00692F30`, `0x0063AB60`, `0x00653F70`, `0x006D6070`, `0x006D8640`, `0x00656750`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** live in-game sidebar radar/minimap mouse-over and click/drag-to-camera behavior, radar pixel to cell/object reverse mapping, viewport/camera clamp, and current Rust minimap click/drag deltas.  
**Non-Scope:** minimap terrain/object rendering, radar chrome/transition SHPs, bridge dirty cells, command-bar buttons, shell/right-panel radar transitions, and non-sidebar preview maps.  
**Confidence:** High for scroll suppression, viewport setter/clamp, and Rust delta; Medium for exact gadget/button-up event plumbing because this read-only pass did not fully drain `GadgetClass` vtable ownership for every radar widget.  
**Active in YR:** Yes. Evidence: `GScreenClass__Input @ 0x004F4320` dispatches through `DisplayClass__Dispatch @ 0x006922E0`, which calls `FUN_00692F30`; `SidebarClass__Action @ 0x006A7780` calls `PowerClass__AnimationTick @ 0x0063FEA0`, which calls `Minimap_Chat_Dispatch @ 0x00653850` on the ordinary sidebar path.

## Required Investigation Notes

- Target question: What exact active YR sidebar minimap input path maps a screen pixel/click-drag to map cell and camera pan, and what happens when selected units exist?
- Non-goals: Do not re-investigate minimap content rendering, radar SHP placement, bridge dirty events, terrain/object dot order, or shell/right-panel minimaps.
- Evidence needed to mark COMPLETE: live input-owner proof, binary evidence for radar hit-test, reverse transform, viewport clamp, selected-unit-vs-camera precedence, current Rust scan, and implementation handoff with tests.
- Stop conditions: stop after ordinary in-game sidebar radar input/camera mapping is proven or explicitly deferred; do not mutate Ghidra; write only this report plus the shared swarm claims file.

## 1. Overview

Native YR treats sidebar radar hover as a separate input surface from tactical-map cursor/order handling. The per-frame scroll/cursor input handler converts the OS mouse point to tactical-relative coordinates, calls the radar hit-test, and returns immediately when the mouse is over the radar widget. The radar viewport path then sets the tactical viewport directly from a radar-derived cell; it does not run the selected-unit order pipeline.

The current Rust implementation diverges by making a selected-unit left-click on the minimap issue a move/attack-move order before camera drag. Native evidence found in this slice supports camera/viewport control as the radar-left-click behavior; no selected-unit command dispatch was found on the verified radar camera path.

## 2. Key Offsets, Globals, And Gates

| Field/global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `g_RadarViewportOffsetX/Y`, `g_RadarViewportWidth/Height` | tactical viewport origin and dimensions; mouse point is reduced to tactical-relative before radar hover test | `0x00692F30` decompile | Yes |
| `DAT_00ac4cf4` | radar-visible/active gate for radar hit-test | `0x0063AB60` early return if zero | Yes, conditional on online/active radar |
| `DAT_00ac4cb0` | radar-jammed/suppressed gate for radar hit-test | `0x0063AB60` early return if nonzero | Conditional |
| `DAT_00ac4ccc` / `DAT_00ac4c38` | current/previous radar widget handles used by radar hover arbitration | `0x0063AB60`, `0x00638E60`, `0x00637070` | Yes |
| `RadarClass+0x11F0/+0x11F4` | content/widget base x/y used for radar widget placement | `0x00652CF0`, `0x00654320` | Yes |
| `RadarClass+0x149C/+0x14A0` | generated content destination origin; reverse object/cell mapping subtracts these before bucket/inverse | `0x00656750` | Yes |
| `Tactical+0xD64/+0xD68` | current viewport x/y written by radar click camera setter | `0x006D6070` | Yes |
| `Tactical+0xD74/+0xD78` | desired viewport x/y written to the same values as current for immediate camera move | `0x006D6070` | Yes |
| `Tactical+0xD7D` | viewport moved/dirty flag set after radar camera write | `0x006D6070` | Yes |

## 3. Core Logic

### 3.1 Input owner and radar hover precedence

Active in YR: Yes.

`GScreenClass__Input @ 0x004F4320` obtains the current input event and mouse position, then calls the vtable slot that reaches `DisplayClass__Dispatch @ 0x006922E0`. `DisplayClass__Dispatch` first calls `FUN_00692F30`, the per-frame scroll/cursor input handler, before command-bar/sidebar dispatch.

Inside `FUN_00692F30`:

1. It reads `g_RadarViewportOffsetX/Y`.
2. It fetches the mouse position through `g_DisplayChain+0x34`.
3. It forms `rel_x = mouse_x - viewport_offset_x` and `rel_y = mouse_y - viewport_offset_y`.
4. It calls `FUN_0063AB60(rel_x, rel_y)`.
5. If that function returns nonzero, `FUN_00692F30` returns immediately.

That early return skips the tactical cursor query (`FUN_00692300`), `DisplayClass__DetermineAction @ 0x00692610`, `DisplayClass__SetCursorFromAction`, band-box continuation, and edge-scroll handling for that frame.

Evidence: decompile `0x004F4320`, `0x006922E0`, `0x00692F30`; branch `0x00692F30` immediately exits the cursor/scroll body when `FUN_0063AB60` returns nonzero.

### 3.2 Radar hit-test bounds and gates

Active in YR: Yes when radar is visible and not suppressed.

`FUN_0063AB60` returns `0` immediately if `DAT_00ac4cf4 == 0` or `DAT_00ac4cb0 != 0`. If a radar widget is present (`DAT_00ac4ccc != 0`), it calls `FUN_006343C0` and `FUN_006339E0`, builds an 8x8 rect centered at the radar center point, and accepts the mouse when it lies in that rect expanded by four pixels on each side.

The accepted comparisons are left/top inclusive and right/bottom exclusive:

```text
x: center_x - 8 <= rel_x < center_x + 8
y: center_y - 8 <= rel_y < center_y + 8
```

The decompile expresses this through `CRect(center_x - 4, center_y - 4, 8, 8)` and comparisons against `left - 4` and `left - 4 + width + 8`, likewise for y.

Evidence: `0x0063AB60`; `FUN_006339E0 @ 0x006339E0` obtains a client point by `TacticalClass__CoordsToClient2` and adds global viewport offsets before the caller uses the tactical-relative result.

### 3.3 Radar pixel to object/cell reverse mapping

Active in YR: Yes for radar object/cell picking.

`RadarClass__GetObjectAtRadarPixel @ 0x00656750` subtracts the generated radar content origin before doing any lookup:

```text
radar_x = click_x - this+0x149C
radar_y = click_y - this+0x14A0
bucket = (radar_x + radar_y * -5) & 0xFF
```

It scans the bucket from last entry to first entry, comparing stored entry x/y to the radar-relative x/y. If an object is found, it calls the object's coord virtual (`vtable+0x4C`) and returns the object's cell by signed lepton-to-cell conversion using the `(coord + (coord >> 31 & 0xFF)) >> 8` pattern.

If no object is found, it computes the cell directly from the radar pixel. Prior assembly proof for the same radar inverse in `RADAR_MINIMAP_DEEP_DIVE.md` shows the formula:

```text
iso_x = radar_x / zoom_factor - map_iso_offset_x
iso_y = radar_y / zoom_factor + map_iso_offset_y
cell_x = ftol((iso_x + iso_y) * 0.5 + 0.5)
cell_y = ftol((iso_y - iso_x) * 0.5 + 0.5)
```

Evidence: `0x00656750` decompile for origin subtraction, bucket hash, reverse scan, object-cell fallback; `RADAR_MINIMAP_DEEP_DIVE.md:215-259` for assembly-backed inverse math at `0x00655CB8..0x00655D0B`.

### 3.4 Radar camera set and clamp

Active in YR: Yes.

The radar camera path reaches `FUN_00653F70`, a tiny wrapper that calls `FUN_006D6070`. `FUN_006D6070` treats its input as cell coordinates and computes a tactical viewport pixel:

```text
pixel_x = ((cell_x * 60) / 2 + (cell_y * -60) / 2 + sign_round) >> 8
pixel_y = (((cell_x * 30) / 2 + (cell_y * 30) / 2 + sign_round) >> 8) - Tactical__AdjustForZ()
```

It then calls `FUN_006D8640(&pixel_xy)`. If that helper returns nonzero and the game is not in map editor mode, the clamped coordinates replace the raw coordinates. Both current and desired viewport fields are written to the same value, so radar camera movement is immediate rather than smooth-scroll interpolation:

```text
this+0xD64 = this+0xD74 = viewport_x
this+0xD68 = this+0xD78 = viewport_y
call FUN_006D8B30()
this+0xD7D = 1
```

Evidence: `FUN_00653F70 @ 0x00653F70`; `FUN_006D6070 @ 0x006D6070`.

### 3.5 Clamp formula

Active in YR: Yes for radar and direct viewport setters.

`FUN_006D8640` clamps viewport pixel x/y against map-derived min/max values. The helper returns a truthy byte if it changed either coordinate.

```text
min_x = g_RadarViewportWidth / 2 + (DAT_0087F8E4 * 2 - DAT_0087F8DC) * 30
max_x = (DAT_0087F8EC * 60 - g_RadarViewportWidth) + min_x

min_y = (DAT_0087F8DC - 5 + DAT_0087F8E8 * 2) * 15 + g_RadarViewportHeight / 2
max_y = ((DAT_0087F8F0 * 60 + 270) / 2 - g_RadarViewportHeight) + min_y
```

It clamps y first, then x. The x comparisons are left inclusive and right saturating: if `x < min_x`, write `min_x`; if `max_x < x`, write `max_x`. Y uses the same pattern.

Evidence: `FUN_006D8640 @ 0x006D8640`; callers `FUN_006D6070 @ 0x006D6070`, `FUN_006D6000 @ 0x006D6000`, and `FUN_006D5F60 @ 0x006D5F60`.

### 3.6 Selected units do not take precedence over radar camera panning

Active in YR: Yes for the verified radar camera path.

No selected-unit order dispatch appears on the verified radar hover/camera path:

- `FUN_00692F30` returns before tactical cell/object query and action determination when over radar.
- `FUN_00653F70` only forwards to `FUN_006D6070`.
- `FUN_006D6070` only computes/clamps viewport fields, updates viewport bookkeeping, and sets the moved flag.
- The selected-unit order machinery is in `DisplayClass__BandBox_LeftUp @ 0x004AB9B0`, including `Selection__DispatchMultiUnitOrder`, and is not called by `FUN_00653F70` / `FUN_006D6070`.

Evidence: `0x00692F30`, `0x00653F70`, `0x006D6070`, `0x004AB9B0`.

Inference: a normal selected-unit left-click on the live sidebar radar should move/pan the camera, not issue move/attack-move orders. This is an inference from negative call-path evidence plus the positive viewport setter path; a runtime breakpoint on `0x00653F70` during a selected-unit radar click would fully close the last event-plumbing uncertainty.

## 4. INI Keys

No INI key directly controls the radar click inverse or viewport clamp. `FogOfWar=no` remains relevant to rendered minimap visibility but not to the click/camera path in this slice.

| Key | Default/source | Effect here | Active in YR |
|---|---|---|---|
| `[General] FogOfWar=no` | `ini/rules.ini`; no `rulesmd.ini` override found in prior radar reports | Not a radar click/camera gate | Conditional, not relevant to this input path |

## 5. Integration Points

| Function | Finding | Evidence | Active in YR |
|---|---|---|---|
| `GScreenClass__Input @ 0x004F4320` | reads input event and mouse position, then dispatches to display pipeline | decompile | Yes |
| `DisplayClass__Dispatch @ 0x006922E0` | calls scroll/radar input handler before command-bar/sidebar dispatch | decompile | Yes |
| `FUN_00692F30` | radar hover short-circuits tactical cursor/action/edge-scroll processing | decompile | Yes |
| `FUN_0063AB60` | radar hover test and mouse-shape notification | decompile | Yes when radar visible/not suppressed |
| `RadarClass__GetObjectAtRadarPixel @ 0x00656750` | radar pixel to object/cell reverse mapping with backward bucket scan | decompile | Yes |
| `FUN_00653F70` / `FUN_006D6070` | radar cell to immediate camera viewport write | decompile | Yes |
| `FUN_006D8640` | camera clamp helper | decompile | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Evidence | Delta |
|---|---|---|---|
| `src/app_sidebar_render.rs::try_begin_minimap_drag` | if cursor is over minimap, calls `minimap_move_order_if_selected` first; only starts camera drag if that returns false | `src/app_sidebar_render.rs:224` | mismatch risk |
| `src/app_sidebar_render.rs::minimap_move_order_if_selected` | selected non-structure entities receive Move or AttackMove command to minimap-derived iso cell | `src/app_sidebar_render.rs:241` | mismatch: native radar camera path does not dispatch selected-unit orders |
| `src/app_sidebar_render.rs::minimap_cursor_to_iso` | converts cursor to a camera-centered world point through Rust minimap aspect-fit and tactical `world_point_to_cell` | `src/app_sidebar_render.rs:315` | mismatch risk: native radar inverse uses radar zoom/iso offsets and object bucket fallback |
| `src/app_sidebar_render.rs::update_camera_from_minimap_cursor` | sets Rust camera top-left from minimap aspect-fit then clamps to playable area | `src/app_sidebar_render.rs:345` | partial: native writes current+desired viewport immediately through `FUN_006D6070` and clamps with `FUN_006D8640` |
| `src/render/minimap.rs::camera_top_left_for_screen_point_in_rect` | normalizes within a 200x200 Rust texture and clamps 0..1 | `src/render/minimap.rs:551` | mismatch risk: native works in generated radar surface coords after subtracting `+0x149C/+0x14A0`, not a stretched 200x200 texture |
| `src/app_sidebar_render.rs::active_minimap_screen_rect` | current chrome path uses `(left=13, top=0, w=140, h=120)` | `src/app_sidebar_render.rs:366` | mismatch vs settled in-game aperture `(16,49)` max `140x108` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Ordinary scroll/cursor radar hover early-out | verified | `0x00692F30` | none |
| Radar hit-test gates and inclusive/exclusive bounds | verified | `0x0063AB60` | exact semantic names of globals remain inherited |
| Radar pixel to object/cell reverse mapping | verified | `0x00656750`; prior inverse assembly `0x00655CB8..0x00655D0B` | exact high-level owner of every call site not drained |
| Camera viewport setter and immediate current/desired writes | verified | `0x00653F70`, `0x006D6070` | none |
| Camera clamp helper | verified | `0x006D8640` | none |
| Selected-unit command path exclusion from radar camera path | verified-with-negative-scan | `0x00692F30`, `0x00653F70`, `0x006D6070`, `0x004AB9B0` | runtime breakpoint would be stronger for button-event provenance |
| Current Rust minimap order/camera behavior | verified | `src/app_sidebar_render.rs`, `src/render/minimap.rs` | no Rust edits/tests run |
| Full `GadgetClass` button-down/up ownership of radar widget | touched-not-exhausted | `0x004E1640`, `0x004E13F0`, vtable pointer search for `0x00653F70` | separate gadget event deep dive if needed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the live per-frame input owner? -> GScreen input reaches DisplayClass dispatch, which calls `FUN_00692F30`.` (evidence: `0x004F4320`, `0x006922E0`, `0x00692F30`)
- `[RESOLVED] OQ-02 - Does radar hover run before tactical action/cursor query? -> Yes; `FUN_00692F30` calls `FUN_0063AB60` before `FUN_00692300`/`DisplayClass__DetermineAction`.` (evidence: `0x00692F30`)
- `[RESOLVED] OQ-03 - Does radar hover suppress tactical scroll/cursor/order processing? -> Yes for that frame; nonzero radar hit-test return exits the function before the tactical branches.` (evidence: `0x00692F30`)
- `[RESOLVED] OQ-04 - What gates radar hover? -> `DAT_00ac4cf4 != 0`, `DAT_00ac4cb0 == 0`, and a nonzero current radar widget/hit result.` (evidence: `0x0063AB60`)
- `[RESOLVED] OQ-05 - Are hit bounds inclusive/exclusive? -> left/top inclusive, right/bottom exclusive around a center-expanded 16x16 test rect.` (evidence: `0x0063AB60`)
- `[RESOLVED] OQ-06 - How does radar object/cell reverse mapping offset click coords? -> subtracts `this+0x149C/+0x14A0` before hash/inverse.` (evidence: `0x00656750`)
- `[RESOLVED] OQ-07 - What happens when a radar object exists under the click? -> bucket scan runs last-to-first and returns the object's own cell coords from its coord virtual.` (evidence: `0x00656750`)
- `[RESOLVED] OQ-08 - What happens when no radar object exists? -> falls back to radar pixel inverse to cell, consistent with prior assembly-proven zoom/offset inverse.` (evidence: `0x00656750`; `RADAR_MINIMAP_DEEP_DIVE.md:215-259`)
- `[RESOLVED] OQ-09 - Does radar camera movement animate like edge scroll? -> No; `FUN_006D6070` writes current and desired viewport fields to the same values.` (evidence: `0x006D6070`)
- `[RESOLVED] OQ-10 - What clamps radar camera movement? -> `FUN_006D8640` clamps x/y against map-derived min/max values.` (evidence: `0x006D8640`)
- `[RESOLVED] OQ-11 - Does map editor follow the same clamp replacement? -> No; if map editor is active, caller keeps raw values even when clamp helper changed the local copy.` (evidence: `0x006D6070`, `0x006D6000`)
- `[RESOLVED] OQ-12 - Does the verified radar camera path dispatch selected-unit orders? -> No call to selected-unit order dispatch exists in `0x00653F70`/`0x006D6070`; the scroll handler also skips tactical action path while over radar.` (evidence: `0x00692F30`, `0x00653F70`, `0x006D6070`, `0x004AB9B0`)
- `[RESOLVED] OQ-13 - Does current Rust issue selected-unit commands on minimap left-click? -> Yes; `try_begin_minimap_drag` calls `minimap_move_order_if_selected` before drag.` (evidence: `src/app_sidebar_render.rs:224`, `src/app_sidebar_render.rs:241`)
- `[RESOLVED] OQ-14 - Does current Rust use native radar inverse/clamp? -> No; it uses a 200x200 texture-normalized aspect-fit and app camera clamp.` (evidence: `src/render/minimap.rs:551`, `src/app_sidebar_render.rs:345`)
- `[DEFERRED] OQ-15 - Which exact `GadgetClass` button-up/down path invokes the radar viewport virtual under every button state?` (category: bounded-cost-too-high; reason: `GadgetClass` event dispatch and vtable ownership are broader than this slot, and the camera setter path plus hover suppression are enough for the Rust handoff; next-step-if-pursued: trace `0x004E1640 -> 0x004E13F0 -> vtable+0x7C` for the radar widget with a runtime click breakpoint)
- `[DEFERRED] OQ-16 - Runtime confirmation of selected-unit radar click with units selected.` (category: needs-runtime-debugger; reason: static path shows camera not order, but a breakpoint on `0x00653F70` and `Selection__DispatchMultiUnitOrder` during one selected-unit radar click would fully close the user-facing scenario; next-step-if-pursued: debug standard skirmish with one selected mobile unit)

## 9. Visual/UI Composition Ledger

This report covers input hit-testing rather than paint composition. The relevant visible surface is the already settled ordinary in-game minimap aperture: max `140x108`, sidebar-local `(16,49)`, produced by `RadarClass__RebuildRadarSurfaces`/`RadarClass__Update`.

| Order | Function / address | Condition / flag proof | Asset / surface | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| input-1 | `FUN_00692F30` | runs from display dispatch | none | mouse relative to tactical viewport | none | yes | input owner |
| input-2 | `FUN_0063AB60` | `DAT_00ac4cf4 != 0`, `DAT_00ac4cb0 == 0`, widget present | none | center-expanded radar hit rect | none | yes/conditional | radar hover gate |
| input-3 | `0x00656750` / `0x00653F70` / `0x006D6070` | radar click/camera path | generated radar content surface coords | subtract `+0x149C/+0x14A0`; set tactical viewport | none | yes | camera mapping |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Radar hover/click suppresses tactical action processing; verified radar camera path does not issue selected-unit move orders. | `0x00692F30`, `0x00653F70`, `0x006D6070`, `0x004AB9B0` | mismatch: selected minimap left-click issues Move/AttackMove first | `src/app_sidebar_render.rs::try_begin_minimap_drag`, `minimap_move_order_if_selected` | Treat live sidebar minimap left-click/drag as camera control, not command dispatch, unless a later runtime trace proves a distinct command gesture. | Select one mobile unit, left-click minimap, assert camera changes and no pending Move/AttackMove command is enqueued; proposed test `minimap_left_click_with_selection_pans_camera_not_move_order` | Do not preserve the current "selected units take precedence" shortcut as native parity. |
| Radar reverse mapping subtracts active content origin and uses radar zoom/iso offsets/object bucket fallback. | `0x00656750`; `RADAR_MINIMAP_DEEP_DIVE.md:215-259` | mismatch risk: Rust maps through stretched 200x200 texture normalized to world bounds | `src/app_sidebar_render.rs::minimap_cursor_to_iso`, `src/render/minimap.rs::camera_top_left_for_screen_point_in_rect` | Use native radar-space mapping: content-local pixel -> object bucket/cell fallback or zoom/offset inverse; do not use texture-space world bounds as the parity mechanism. | Click a pixel inside content padding/centered small-map area and assert native cell/camera result; proposed test `minimap_click_uses_radar_content_origin_and_zoom_offsets` | Do not infer radar click cells from `MINIMAP_SIZE=200` or normalized terrain world bounds. |
| Radar camera setter writes current and desired viewport immediately, then clamps through `FUN_006D8640`. | `0x006D6070`, `0x006D8640` | partial: Rust sets camera top-left and app-clamps playable area | `src/app_sidebar_render.rs::update_camera_from_minimap_cursor`, `src/app_camera::clamp_camera_to_playable_area` | Match native immediate viewport set and clamp min/max formulas for radar clicks/drags. | Click each minimap corner on a large map and assert camera saturates to native min/max without smooth-scroll lag; proposed test `minimap_camera_corner_click_clamps_like_fun_006d8640` | Do not reuse edge-scroll interpolation or generic playable-area clamp if it produces different boundary pixels. |

### Negative Facts / Do Not Do

- Do not issue selected-unit Move/AttackMove from ordinary minimap left-click as a native-parity behavior; the verified radar path goes to immediate viewport write, not `Selection__DispatchMultiUnitOrder` (`0x00692F30`, `0x006D6070`, `0x004AB9B0`).
- Do not feed minimap clicks through the tactical screen inverse `0x006D6590` as though the radar were tactical viewport pixels; radar reverse mapping uses radar content-relative pixels and radar zoom/iso offsets (`0x00656750`; `RADAR_MINIMAP_DEEP_DIVE.md:215-259`).
- Do not use a 200x200 normalized Rust minimap texture as the native coordinate space; the active native content surface is the generated radar surface positioned by `+0x149C/+0x14A0` (`0x00656750`, `SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`).
- Do not animate radar camera movement through edge-scroll smoothing; `0x006D6070` writes current and desired viewport fields together.
- Do not clamp radar camera with a broad "playable area" abstraction until it is proven equivalent to `FUN_006D8640`'s min/max formulas.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/ScrollClass_research.md`: replace "Radar click handling converts the clicked radar pixel to a cell coordinate using `RadarClass__CellToRadarPixel` (inverse)" with "Radar click handling uses radar content-relative pixel-to-object/cell reverse mapping (`RadarClass__GetObjectAtRadarPixel @ 0x00656750`) and then sets the tactical viewport through `FUN_006D6070`; `RadarClass__CellToRadarPixel @ 0x006550C0` is the forward cell-to-radar projection."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/MouseClass_research.md`: replace "slot +0x074 `0x00654490` Radar_ClickHandler" with "`0x00654490` is `RadarClass__ComputeRadarMapBounds`, not a click handler; the radar camera setter wrapper observed in this pass is `0x00653F70 -> 0x006D6070`."

## 11. Remaining Uncertainty

- Full `GadgetClass` mouse-down/up event ownership for the radar widget was touched but not exhausted. Static evidence proves radar hover suppression and the camera setter path, but a runtime selected-unit minimap click breakpoint would be the strongest final proof of button-event provenance.
- Exact semantic names of `DAT_00ac4cf4`, `DAT_00ac4cb0`, `DAT_00ac4ccc`, and `DAT_00ac4c38` remain inferred from use; field effects and branches are verified.

## Sources

- Ghidra read-only decompile: `0x004F4320`, `0x006922E0`, `0x00692F30`, `0x0063AB60`, `0x006339E0`, `0x006343C0`, `0x00653F70`, `0x006D6070`, `0x006D8640`, `0x00656750`, `0x006D6000`, `0x006D5F60`, `0x004AB9B0`, `0x004E1640`, `0x004E13F0`.
- Prior docs read: `RADAR_MINIMAP_DEEP_DIVE.md`, `RADAR_MINIMAP_RENDERING.md`, `SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`, `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `ScrollClass_research.md`, `MouseClass_research.md`, `TACTICAL_SCREEN_PIXEL_TO_CELL_INVERSE_RECHECK_GHIDRA_REPORT.md`.
- Current Rust scan: `src/app_sidebar_render.rs`, `src/render/minimap.rs`.

## Status

PARTIAL for exact `GadgetClass` button-event provenance, COMPLETE for Rust-facing behavior needed now: ordinary sidebar radar input suppresses tactical command processing, maps through radar content coordinates, and sets/clamps the camera immediately rather than issuing selected-unit movement orders.
