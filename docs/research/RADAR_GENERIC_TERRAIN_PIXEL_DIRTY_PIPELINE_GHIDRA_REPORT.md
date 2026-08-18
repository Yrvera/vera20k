# Radar Generic Terrain / Pixel Dirty Pipeline - Ghidra Research Report

**Address(es):** `0x006551C0`, `0x00655250`, `0x006562D0`, `0x00655C50`, `0x00656EC0`, `0x00653100`, `0x00487190`, `0x00480A80`, `0x00655560`, `0x00655740`, `0x006565A0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Generic, non-bridge `RadarClass` terrain dirty list, pixel dirty list, dirty rect expansion, and update cadence; bridge paths are referenced only as comparison points.  
**Non-Scope:** Full 37-caller `MarkTerrainDirty` census, settled bridge repair/collapse specifics, object-dot priority details beyond dirty-list producers, spy-satellite geometry, radar event animation details, and final sidebar composition outside the minimap content blit.  
**Confidence:** High for dirty queue mechanics, bounds checks, duplicate handling, rect expansion, and `RadarClass::Update` cadence; Medium for complete generic producer breadth because this slot spot-checked representative non-bridge callers instead of enumerating every caller.  
**Active in YR:** Yes. Evidence: `RadarClass::Draw @ 0x00653100` calls `RadarClass::Update @ 0x00656EC0` in ordinary in-game radar draw; non-bridge ore placement/removal and object tracker paths call the dirty primitives.

## Working Notes Gate

Target question: How do native generic radar terrain dirty and pixel dirty queues work, and when are they flushed into minimap pixels?  
Non-goals: Do not redo bridge repair/collapse, radar aperture, Soviet chrome placement, object-dot priority, or minimap inverse input mapping.  
Evidence needed to mark COMPLETE: Decompile plus assembly/disassembly-range evidence for `MarkTerrainDirty`, `MarkCellDirty`, `ClearBackground`, `RenderCellPixel`, `Update`, and at least two non-bridge live producers; current Rust touchpoint scan and implementation handoff.  
Stop conditions: Stop before full caller census, spy-satellite rect decode, radar event animation internals, or Rust edits; if a Ghidra boundary is missing, record uncertainty instead of mutating Ghidra.

## 1. Overview

Native `RadarClass` has two distinct incremental dirty systems. Terrain dirty entries are packed map cells that refresh the raw RGB terrain buffer and the generated terrain surface. Pixel dirty entries are final radar-surface pixels that re-run object/fog/shroud/terrain composition into the live primary minimap surface.

`RadarClass::Update` is the flush owner. It calls `ClearBackground` first, then handles viewport-edge pixel dirtying, terrain-surface blit and terrain rect rerender, explicit pixel dirty list rendering, radar events, spy-satellite vision, and finally copies only the accumulated dirty rect to the sidebar surface when radar mode is active.

## 2. Class Layout / Key Offsets

| Struct / object | Offset | Type / shape | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `RadarClass` | `+0x120C/+0x1210/+0x1214/+0x1218` | four ints | Accumulated sidebar-local dirty rect: x, y, w, h. | `0x00655C50`, `0x00656EC0`, `0x00653100`; disasm range `0x0065608F..0x0065612F` | Yes |
| `RadarClass` | `+0x121C` | surface ptr | Primary/live minimap surface used by `RenderCellPixel` and sidebar blit. | `0x00655C50`, `0x00656EC0` | Yes |
| `RadarClass` | `+0x1220` | surface ptr | Secondary/generated terrain surface. | `0x00655250`, `0x00656EC0` | Yes |
| `RadarClass` | `+0x1228/+0x1234` | 4-byte coord vector + count | Terrain dirty list. Each entry is packed `CellStruct` copied as 4 bytes. | `0x006551C0`; disasm range `0x006551C0..0x0065524E` | Yes |
| `RadarClass` | `+0x123C` | byte buffer | Raw RGB terrain buffer, 3 bytes per radar-space pixel. | `0x00655250`, prior `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md` | Yes |
| `RadarClass` | `+0x1248/+0x124C/+0x1250/+0x1254` | four ints | Generated-terrain dirty rect produced by `ClearBackground`, source-space before sidebar offset. | `0x00655250`, `0x00656EC0` | Yes |
| `RadarClass` | `+0x1260/+0x126C` | 8-byte pixel vector + count | Pixel dirty list, entries `{x:int, y:int}`. | `0x006562D0`; disasm range `0x006562D0..0x006563BE` | Yes |
| `RadarClass` | `+0x1274` | bitfield ptr | Per-primary-surface-pixel visited bitfield suppressing duplicate pixel dirty entries within an update pass. | `0x006562D0`, `0x00656EC0` | Yes |
| `RadarClass` | `+0x14D9` | byte | Dirty/update-needed flag set by both terrain and pixel dirty producers. | `0x006551C0`, `0x006562D0`, `0x00653100`, `0x00656EC0` | Yes |
| `RadarClass` | `+0x14DA` | byte | Force/full redraw mode flag; `Draw` folds it into `+0x14D9`. | `0x00653100`, `0x00656EC0` | Yes |
| `RadarClass` | `+0x1490/+0x1498` | ints | Radar-space map iso offsets used by cell-to-radar projection. | `0x00655250`, `0x006565A0` | Yes |
| `RadarClass` | `+0x149C/+0x14A0` | ints | Sidebar-local content origin added to primary pixel coords for dirty rect expansion. | `0x00655C50`, `0x00656EC0` | Yes |

## 3. Core Logic

### 3.1 `MarkTerrainDirty @ 0x006551C0`

`MarkTerrainDirty(this, &cellCoord)` scans the existing terrain dirty list backward from `count - 1`. A duplicate is two signed 16-bit halves equal to the input coord; on duplicate it returns before appending and before setting `+0x14D9`.

If unique, it grows the vector when needed using the vector vtable at `+0x1224`, appends the 4-byte packed coord to `+0x1228[count]`, increments `+0x1234`, then sets `RadarClass+0x14D9 = 1`. If vector growth fails, no coord is appended, but `+0x14D9` is still set.

**Active in YR:** Yes. Non-bridge producers include new tiberium placement `CellClass::PlaceTiberium @ 0x00487190` and full ore removal `CellClass::Reduce_Tiberium @ 0x00480A80`; `0x00480BEA` is a direct `CALL 0x006551C0` after overlay clear/recalc.

### 3.2 `ClearBackground @ 0x00655250`

`ClearBackground` drains the terrain dirty list from back to front. For each packed coord:

1. Reject if `MapClass::Is_Cell_In_Playfield(coord, 1)` is false.
2. Project cell to raw radar-space: `x = (offset_x - y) + x`, `y = (y - offset_y) + x`.
3. Use width `2` normally; when projected x is `-1` or `map_width * 2 - 1`, clamp x to `0` or keep edge x and use width `1`.
4. Clip `{x, y, w, 1}` against the raw terrain buffer dimensions.
5. Call `MapClass::Get_CellClass` and `CellClass::GetRadarColor`, which returns left/right RGB triples.
6. Write one or two 3-byte RGB samples into `this+0x123C`.
7. Call `GenerateTerrainSurface` for the clipped raw rect and union its returned generated-surface rect into `+0x1248..+0x1254`.

At list exhaustion it clears the dynamic vector through vtable `+0x0C` and stores the union rect. The union math uses inclusive-style expansion: if current `x+w < new_x+new_w`, the new width becomes `(new_w - current_x) + 1 + new_x`.

**Active in YR:** Yes. `RadarClass::Update @ 0x00656EC0` calls this before rendering pixel dirty cells whenever dirty/update conditions fire.

### 3.3 `MarkCellDirty @ 0x006562D0`

`MarkCellDirty(this, &pixel)` is final-pixel dirtying for the primary minimap surface. It rejects:

- `x < 0`
- `x >= primary_surface.width()`
- `y < 0`
- `y >= primary_surface.height()`

It computes linear index `idx = width * y + x`, then tests `visited[idx >> 3] & (1 << (idx & 7))`. If that bit is already set, it returns without appending and without setting `+0x14D9`. Otherwise it sets the bit, grows the pixel dirty vector when needed, appends `{x, y}` as two 32-bit ints at `+0x1260[count]`, increments `+0x126C`, and sets `+0x14D9 = 1`.

**Active in YR:** Yes. Non-bridge producers include `RadarClass::AddObjectToTracker @ 0x00655560`, `RemoveObjectFromTracker @ 0x00655740`, `MarkObjectDirty @ 0x006565A0`, and viewport edge dirtying inside `Update @ 0x00656EC0`.

### 3.4 `RenderCellPixel @ 0x00655C50`

`RenderCellPixel` consumes one primary-surface pixel coordinate. It bounds-checks the primary surface rect, converts the pixel to a map cell, computes shroud/fog, scans the object tracker bucket, and writes exactly one final 16-bit pixel to the primary surface:

1. Eligible object color, including local-player inversion path, wins first.
2. Else fogged terrain reads secondary terrain pixel, unpacks channels, unsigned halves channels with `>> 1`, repacks, and writes.
3. Else shrouded terrain writes literal packed `0`.
4. Else visible terrain copies the secondary terrain pixel.

After the pixel write, it expands the accumulated sidebar-local dirty rect by adding `+0x149C/+0x14A0` to the primary-surface pixel. A first pixel initializes `w=1,h=1`. Later pixels expand left/up by increasing width/height by the previous offset delta, and expand right/down with a `+2` inclusive adjustment when the new point lies outside the current rect.

**Active in YR:** Yes. `Update` calls `RenderCellPixel` for generated-terrain dirty rect pixels and explicit pixel dirty list entries.

### 3.5 `RadarClass::Update @ 0x00656EC0`

`Update` first saves the incoming accumulated dirty rect. It then computes current viewport/scroll rectangle fields and checks whether work is needed. The update condition fires when any of these are true:

- `+0x14D9` is set.
- Current viewport rect differs from previous `+0x14EC..+0x14F8`.
- `ObjectsMovedCheck()` says no clean skip is available or terrain/pixel dirty counts are non-zero.
- `HasSpySatelliteUpdate()` is true.

When work fires, order is:

1. If active/open radar state is true (`+0x14B0 == 1 && +0x14AC == 1`), clear `+0x14D9`.
2. Call `ClearBackground`.
3. If viewport rect changed, mark old viewport rectangle edge pixels through `MarkCellDirty`.
4. If `+0x1248..+0x1254` is non-empty, blit secondary terrain surface over that rect into the primary surface, union that terrain rect into the accumulated sidebar dirty rect, then call `RenderCellPixel` for every pixel inside the generated-terrain dirty rect.
5. Iterate the explicit pixel dirty list from back to front, calling `RenderCellPixel` for each entry.
6. Clear the pixel dirty vector through vtable `+0x0C`.
7. Tick/draw radar events.
8. Draw spy-satellite vision.
9. If active/open radar state and the accumulated rect is non-empty, copy only that rect from primary surface `+0x121C` to `g_SidebarSurface`, using source rect = dirty rect minus `+0x149C/+0x14A0`.
10. Draw viewport rectangles.
11. Update previous viewport fields and cleanup expired events.
12. Clear the visited bitfield `+0x1274` after the pass.

If not active/open radar state, the saved incoming dirty rect is restored at the end. This means dirty bookkeeping can run while not active, but the global accumulated sidebar rect is not advanced the same way as the active visible mode.

**Active in YR:** Yes. `RadarClass::Draw @ 0x00653100` calls `FUN_0065FDD0` then `RadarClass::Update` every draw pass unless map editor early-out applies. Ordinary online radar uses the active/open state path.

## 4. INI Keys

| Key | Default / stock value | Effect | Evidence | Active in YR |
|---|---|---|---|---|
| `[General] FogOfWar` | `no` in `rules.ini`, no `rulesmd.ini` override found by prior minimap report | Does not create the dirty queues; affects `RenderCellPixel` fog branch when enabled. | `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `0x00655C50` | Conditional; ordinary YR default off |
| `[Riparius] Image` and tiberium overlay ranges | Stock tiberium image range | Non-bridge live producer: new ore placement creates terrain dirty through `PlaceTiberium`. | `0x00487190`, prior TIBTRE reports | Yes |
| `Radar=yes` / `SpySat=yes` | Building-type keys | Gates player radar availability upstream, not dirty queue semantics. | Rust/rules scan; power/radar docs | Conditional; when player owns powered provider |

No INI key was found that changes `MarkTerrainDirty`, `MarkCellDirty`, duplicate handling, bitfield behavior, or dirty rect expansion constants.

## 5. Integration Points

| Caller / callee | Relationship | Evidence | Active in YR |
|---|---|---|---|
| `RadarClass::Draw -> RadarClass::Update` | Per draw pass, after radar chrome/state handling. | `0x00653100` | Yes |
| `CellClass::PlaceTiberium -> MarkTerrainDirty` | Empty/new tiberium placement dirties terrain after tactical dirty work. | `0x00487190`; prior report cites direct call `0x00487685/nearby`; decompile verified in this pass | Yes |
| `CellClass::Reduce_Tiberium -> MarkTerrainDirty` | Full ore removal clears overlay, recalcs attributes, then dirties radar terrain. | `0x00480A80`; assembly `0x00480BE1..0x00480BEA` | Yes |
| `RadarClass::AddObjectToTracker -> MarkCellDirty` | Object registration dirties final pixel after tracker insert or clamped position update. | `0x00655560` | Yes |
| `RadarClass::RemoveObjectFromTracker -> MarkCellDirty` | Successful tracker removal compacts bucket and dirties final pixel. | `0x00655740` | Yes |
| `RadarClass::MarkObjectDirty -> MarkCellDirty` | Converts a map cell footprint to a clipped pixel range and marks each pixel. | `0x006565A0` | Yes |
| `RadarClass::Update -> ClearBackground -> RenderCellPixel` | Terrain dirty cells refresh raw/secondary terrain first, then final pixels are rerendered. | `0x00656EC0`, `0x00655250`, `0x00655C50` | Yes |

## 6. Current Rust Implementation Status

Rust currently models only a bridge-specific radar terrain dirty channel. `Simulation` stores `radar_terrain_dirty_cells` and a generation counter at `src/sim/world/mod.rs:349..358`, and `mark_radar_terrain_dirty_cells` deduplicates by `Vec::contains` before incrementing generation at `src/sim/world/mod.rs:495..509`.

The render path passes that bridge-specific list into `MinimapRenderer::update_unit_dots` at `src/app_render/build_instances.rs:453..475`. `update_unit_dots` rebuilds an RGBA scratch minimap from cached base terrain, overlay pixels, and unit dots when tick/fog/dirty generation changes, then uploads the full `MINIMAP_SIZE x MINIMAP_SIZE` texture (`src/render/minimap.rs:207..243`, `245..285`, and upload later in the same function).

`apply_bridge_terrain_dirty_cells` only re-evaluates bridge overlay visibility against `BridgeRuntimeState` (`src/render/minimap.rs:446..505`). Generic ore/overlay dirty cells drained in `app_sim_tick.rs:684..732` update passability and append new render overlay entries, but they do not publish native-style radar terrain dirty cells into `Simulation::radar_terrain_dirty_cells` or maintain a separate pixel dirty list/visited bitfield.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working-notes gate | verified | report gate section | none |
| `RadarClass::MarkTerrainDirty` duplicate and append semantics | verified | `0x006551C0`; disasm `0x006551C0..0x0065524E` | none |
| Terrain dirty growth failure still sets `+0x14D9` | verified | `0x006551C0` | none |
| `RadarClass::ClearBackground` terrain-list drain and raw RGB refresh | verified | `0x00655250`; disasm `0x00655250..0x006554CF` | none |
| ClearBackground generated rect union | verified | `0x00655250` | none |
| `RadarClass::MarkCellDirty` bounds, visited bit, append semantics | verified | `0x006562D0`; disasm `0x006562D0..0x006563BE` | none |
| `RadarClass::RenderCellPixel` final pixel order and dirty rect expansion | verified | `0x00655C50`; disasm `0x0065608F..0x0065612F` | none for dirty mechanics |
| `RadarClass::Update` flush cadence | verified | `0x00656EC0` | spy-satellite geometry out-of-scope |
| `RadarClass::Draw` per-draw integration | verified | `0x00653100` | higher sidebar composition out-of-scope |
| Non-bridge terrain dirty liveness: tiberium placement | verified | `0x00487190`, prior TIBTRE docs | exact direct call address in this Ghidra function body was not separately xref-listed by the tool |
| Non-bridge terrain dirty liveness: full ore removal | verified | `0x00480A80`; `0x00480BEA CALL 0x006551C0` | none |
| Non-bridge pixel dirty liveness: object add/remove | verified | `0x00655560`, `0x00655740`, `0x006565A0` | object priority semantics out-of-scope |
| Full 37-caller `MarkTerrainDirty` census | deferred | `ADDRESS_MAP.md` says 37 callers | follow-up caller matrix if needed |
| Rust generic radar dirty parity | verified mismatch | `src/render/minimap.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs` | implementation needed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is this coverage-map or exhaustive-slice? -> exhaustive-slice for the generic dirty primitives and update cadence, not for all producers.` (evidence: user scope; report header)
- `[RESOLVED] OQ-02 - What does `MarkTerrainDirty` store? -> unique 4-byte packed cell coords in `+0x1228`, count `+0x1234`.` (evidence: `0x006551C0`)
- `[RESOLVED] OQ-03 - Does terrain dirty dedupe compare packed dword or two shorts? -> two signed 16-bit halves are compared, then the 4-byte coord is copied.` (evidence: `0x006551C0`)
- `[RESOLVED] OQ-04 - Does duplicate terrain dirty set the dirty flag? -> no, duplicate returns before `+0x14D9` write.` (evidence: `0x006551C0`)
- `[RESOLVED] OQ-05 - Does append failure set the dirty flag? -> yes, `+0x14D9=1` is after the append/grow conditional.` (evidence: `0x006551C0`)
- `[RESOLVED] OQ-06 - How are terrain dirty entries flushed? -> `ClearBackground` drains back-to-front, recolors raw RGB, regenerates affected secondary-surface rect, and clears the vector.` (evidence: `0x00655250`)
- `[RESOLVED] OQ-07 - What are edge-cell bounds? -> projected `x == -1` or `x == map_width*2-1` uses one raw pixel instead of two.` (evidence: `0x00655250`)
- `[RESOLVED] OQ-08 - What does `MarkCellDirty` store? -> unique `{x:int,y:int}` primary-surface pixel entries in `+0x1260`, count `+0x126C`.` (evidence: `0x006562D0`)
- `[RESOLVED] OQ-09 - How are duplicate pixel dirty entries prevented? -> visited bitfield `+0x1274` indexed by `width*y+x` suppresses duplicates until `Update` clears it.` (evidence: `0x006562D0`, `0x00656EC0`)
- `[RESOLVED] OQ-10 - What are pixel dirty bounds? -> negative and `>= primary_surface.width/height` are rejected before bitfield append.` (evidence: `0x006562D0`)
- `[RESOLVED] OQ-11 - What expands the accumulated dirty rect? -> `RenderCellPixel` after writing a pixel, using sidebar origin offsets and `+2` right/down expansion.` (evidence: `0x00655C50`)
- `[RESOLVED] OQ-12 - Which flushes first, terrain or pixel dirty? -> terrain list/secondary rect first, then explicit pixel dirty list, then radar events/spy satellite/sidebar blit.` (evidence: `0x00656EC0`)
- `[RESOLVED] OQ-13 - Is this active in ordinary YR? -> yes, `RadarClass::Draw` calls `Update`, and ore/object generic producers call dirty primitives.` (evidence: `0x00653100`, `0x00480BEA`, `0x00655560`)
- `[RESOLVED] OQ-14 - Does standard `FogOfWar=no` disable dirty queues? -> no; it only affects fog branch visibility, not queue mechanics.` (evidence: `0x00655C50`, prior INI scan)
- `[RESOLVED] OQ-15 - Does Rust have a generic native-style terrain dirty queue? -> no; only bridge-specific cells/generation are carried to minimap update.` (evidence: `src/sim/world/mod.rs:349..358`, `src/render/minimap.rs:446..505`)
- `[RESOLVED] OQ-16 - Does Rust have native pixel dirty list/visited bitfield? -> no; it rebuilds/reuploads an RGBA texture on tick/fog/generation changes.` (evidence: `src/render/minimap.rs:207..243`)
- `[RESOLVED] OQ-17 - Null pointer edge case? -> `MarkCellDirty` assumes primary surface exists; `MarkObjectDirty` has an explicit `+0x121C == 0` early return before marking ranges.` (evidence: `0x006562D0`, `0x006565A0`)
- `[RESOLVED] OQ-18 - Empty container edge case? -> terrain list count starts at `+0x1234`; `ClearBackground` immediately clears/stores zero rect when count reaches below zero.` (evidence: `0x00655250`)
- `[RESOLVED] OQ-19 - First dirty pixel edge case? -> first `RenderCellPixel` initializes dirty rect to one pixel at sidebar-local coordinate.` (evidence: `0x00655C50`)
- `[RESOLVED] OQ-20 - Last tick / cleanup edge case? -> `Update` clears pixel visited bitfield after processing, so duplicate suppression is per update pass, not permanent.` (evidence: `0x00656EC0`)
- `[DEFERRED] OQ-21 - Full 37-caller `MarkTerrainDirty` matrix.` (category: bounded-cost-too-high; reason: primitive and representative non-bridge liveness are sufficient for this slot; next-step-if-pursued: caller census over all `0x006551C0` xrefs)
- `[DEFERRED] OQ-22 - Spy-satellite dirty rectangle geometry.` (category: out-of-scope; reason: target is generic terrain/pixel dirty queues, not satellite overlay coverage; next-step-if-pursued: investigate `DrawSpySatelliteVision` and satellite update producer)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `RadarClass::Draw @ 0x00653100` | Draw pass and not map editor | Radar chrome/state frame, if dirty/opening | `+0x11E4/+0x11EC`, full radar chrome for some states | `CC_Draw_Shape` route | Yes | chrome/state prep |
| 2 | `RadarClass::Update @ 0x00656EC0` -> `ClearBackground @ 0x00655250` | Dirty condition true | none | terrain dirty generated rect `+0x1248..+0x1254` | raw RGB -> secondary packed surface | Yes | terrain content refresh |
| 3 | `RadarClass::Update @ 0x00656EC0` -> secondary blit | generated terrain rect non-empty | none | source/dest `+0x1248..+0x1254` | secondary to primary surface blit | Yes | primary terrain base |
| 4 | `RadarClass::RenderCellPixel @ 0x00655C50` | generated rect pixels and explicit pixel dirty list | none | primary pixel plus sidebar origin offsets | object/fog/shroud/terrain 16-bit packed pixel | Yes | final minimap content pixels |
| 5 | radar event draw / spy satellite draw | after pixel dirty list | radar events / satellite overlay | primary surface rect | native surface drawing | Yes when active | overlay on minimap content |
| 6 | sidebar blit in `Update @ 0x00656EC0` | active/open radar and accumulated dirty rect positive | none | sidebar dirty rect, source minus `+0x149C/+0x14A0` | primary surface -> `g_SidebarSurface` | Yes | visible content copy |
| 7 | viewport rectangle draw | active/open radar | primitive | `+0x14DC..+0x14E8` plus one-pixel-expanded outline | surface rect draw | Yes | viewport overlay |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| Generated raw RGB / secondary / primary radar surfaces | Yes | Yes | Yes | Content | No | No | No | No | `0x00655250`, `0x00655C50`, `0x00656EC0` |
| `DAT_00B04A38` radar frame set | Yes | Conditional | Conditional | No | Yes | No | Opening/closing/jammed states | No | `0x00653100`, `0x00656EC0` |
| Radar event primitives | Conditional | Conditional | Conditional | No | No | Yes | No | No | `0x00656EC0` |
| Spy-satellite vision overlay | Conditional | Conditional | Conditional | No | No | Yes | No | No | `0x00656EC0` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Generic terrain dirty is a unique packed-cell list; duplicates return without setting dirty, unique entries refresh raw/secondary terrain first in `Update`. | `0x006551C0`, `0x00655250`, `0x00656EC0`; non-bridge `0x00480BEA` | Mismatch: Rust has bridge-specific dirty cells only and generic overlay/ore dirty does not feed minimap terrain dirty. | `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, `src/render/minimap.rs` | Publish generic terrain dirty cells for ore/overlay/removal/new placement, dedupe with native semantics, and refresh minimap terrain/overlay pixels from current cell state before object dots. | Deplete an ore cell fully and spawn a new TIBTRE/ore cell in the same visible radar frame; only the changed cells' terrain/overlay minimap pixels refresh before dot rendering. Proposed test: `minimap_generic_terrain_dirty_refreshes_ore_cells_before_dots`. | Do not keep the dirty channel bridge-only or rely on static `state.overlays` sync alone; stale ore/minimap pixels drift from native. |
| Pixel dirty uses a primary-surface `{x,y}` list plus a per-update visited bitfield; object add/remove and object dirty footprints mark final pixels, not terrain cells. | `0x006562D0`, `0x00655560`, `0x00655740`, `0x006565A0`, `0x00656EC0` | Missing: Rust rebuilds/reuploads the whole RGBA minimap on tick/fog/generation changes and has no pixel dirty list/visited bitfield. | `src/render/minimap.rs`, `src/app_render/build_instances.rs`, future retained minimap surface model | Track final pixel dirty entries separately from terrain dirty cells and suppress duplicates until the update pass clears the visited bitfield. | Move two objects through the same radar pixel and remove one; the pixel is queued once before update and rerendered once from current tracker state. Proposed test: `minimap_pixel_dirty_visited_bit_suppresses_duplicate_object_pixels`. | Do not model object movement as terrain dirty; object/fog/shroud composition belongs to the pixel dirty path. |
| `RenderCellPixel` expands sidebar-local dirty rect after each final pixel write, with first pixel `1x1` and right/down expansion using native `+2` inclusivity. | `0x00655C50`; disasm `0x0065608F..0x0065612F`; sidebar blit in `0x00656EC0` | Mismatch: Rust uploads full minimap texture and draws it as a sprite, so native rect/copy cadence is absent. | `src/render/minimap.rs`, `src/app_render/draw_passes.rs`, sidebar surface/chrome composition code | Preserve accumulated dirty rect semantics when aiming for pixel/flicker parity, and copy only the dirty source rect to the retained sidebar content surface. | Mark final pixels `(10,10)` then `(12,10)`; accumulated dirty rect matches native inclusive width behavior and blit source subtracts content origin. Proposed test: `minimap_dirty_rect_expands_with_native_plus_two_inclusivity`. | Do not use exclusive rectangle union or a full texture upload as a substitute when native sidebar dirty/flicker parity matters. |

### Negative Facts / Do Not Do

- Do not merge terrain dirty and pixel dirty into one cell list. They have different element shapes, duplicate systems, flush order, and render responsibilities (`0x006551C0` vs `0x006562D0`; `0x00656EC0`).
- Do not set dirty on duplicate terrain entries or duplicate pixel entries. Native duplicate exits before `+0x14D9` for both terrain duplicate and visited-bit duplicate (`0x006551C0`, `0x006562D0`).
- Do not dirty terrain for partial tiberium density reduction. `Reduce_Tiberium` calls `MarkTerrainDirty` only on full removal after clearing overlay and recalculating attributes (`0x00480A80`, direct call at `0x00480BEA`).
- Do not treat `MarkObjectDirty` as cell terrain refresh. It expands a cell footprint into primary-surface pixel coordinates and calls `MarkCellDirty` for pixels (`0x006565A0`).
- Do not redraw every minimap pixel every frame if matching native dirty/copy cadence is the target. Native flushes terrain dirty rects and pixel dirty lists incrementally, then blits only the accumulated rect (`0x00656EC0`).

### Stale Docs / Follow-up Docs

- `docs/research/RADAR_MINIMAP_RENDERING.md`: replace "Processed by ClearBackground -> re-reads GetRadarColor, updates raw buffer + generated terrain surface" with "Processed by `ClearBackground @ 0x00655250`: drains terrain dirty cells back-to-front, skips out-of-playfield coords, refreshes one/two raw RGB radar-space samples through `CellClass::GetRadarColor`, regenerates the clipped secondary-surface rect, stores its union at `+0x1248..+0x1254`, then `Update` rerenders final pixels for that rect."
- `docs/research/RADAR_MINIMAP_RENDERING.md`: replace "Added by MarkCellDirty. Processed by RenderCellPixel loop." with "Added by `MarkCellDirty @ 0x006562D0` only if inside primary surface bounds and not already marked in visited bitfield `+0x1274`; processed by `Update @ 0x00656EC0` back-to-front after terrain dirty rect rendering, then the pixel vector and visited bitfield are cleared for the next pass."
- `docs/research/RADAR_MINIMAP_RENDERING.md`: replace "dirty rect tracking" wording that implies generic rectangle union with "Dirty rect expansion is performed by `RenderCellPixel @ 0x00655C50` in sidebar-local coordinates after a final pixel write; first pixel initializes `1x1`, and right/down growth uses native inclusive `+2` expansion."

## Remaining Uncertainty

- The full 37-caller `MarkTerrainDirty` xref matrix was not enumerated; this report proves the generic primitive and representative non-bridge liveness, not every producer.
- Exact spy-satellite dirty coverage geometry is out of scope; `Update` ordering relative to pixel dirty is verified.
- `CellClass::PlaceTiberium` direct call address for `MarkTerrainDirty` was verified by decompile and prior docs but not separately listed by Ghidra xref output in this pass.

## Sources

- Ghidra decompile: `RadarClass::MarkTerrainDirty @ 0x006551C0`; disasm range `0x006551C0..0x0065524E`.
- Ghidra decompile: `RadarClass::ClearBackground @ 0x00655250`; disasm range `0x00655250..0x006554CF`.
- Ghidra decompile: `RadarClass::MarkCellDirty @ 0x006562D0`; disasm range `0x006562D0..0x006563BE`.
- Ghidra decompile: `RadarClass::RenderCellPixel @ 0x00655C50`; assembly context `0x0065608F..0x0065612F`.
- Ghidra decompile: `RadarClass::Update @ 0x00656EC0`; assembly contexts around `0x006575E5` and `0x00657668` for sidebar surface copy/draw region.
- Ghidra decompile: `RadarClass::Draw @ 0x00653100`.
- Ghidra decompile: `CellClass::PlaceTiberium @ 0x00487190`, `CellClass::Reduce_Tiberium @ 0x00480A80`; assembly context `0x00480BE1..0x00480BEA`.
- Ghidra decompile: `RadarClass::AddObjectToTracker @ 0x00655560`, `RemoveObjectFromTracker @ 0x00655740`, `MarkObjectDirty @ 0x006565A0`.
- Prior docs referenced: `RADAR_MINIMAP_RENDERING.md`, `RADAR_MINIMAP_DEEP_DIVE.md`, `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `bridges/06-render-presentation-audio/BRIDGE_PRESENTATION_RADAR_DIRTY_GHIDRA_REPORT.md`, `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md`, `REDUCE_TIBERIUM_FULL_REMOVAL_SIDE_EFFECTS_AND_RETURN_VALUE_GHIDRA_REPORT.md`.
- Rust scanned: `src/render/minimap.rs`, `src/sim/world/mod.rs`, `src/app_render/build_instances.rs`, `src/app_sim_tick.rs`.
