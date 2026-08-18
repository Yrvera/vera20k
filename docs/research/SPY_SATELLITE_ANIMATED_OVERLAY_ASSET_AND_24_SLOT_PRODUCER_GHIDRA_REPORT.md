# SPY_SATELLITE_ANIMATED_OVERLAY_ASSET_AND_24_SLOT_PRODUCER

**Date:** 2026-05-27  
**Address(es):** `Init_BeaconArt @ 0x00430A00`, `DrawOneSpySatellite @ 0x00430650`, `DrawSpySatelliteVision @ 0x00431700`, `HasSpySatelliteUpdate @ 0x00431800`, `RadarClass::PlaceBeacon @ 0x00430BA0`, `FUN_004311C0`, `FUN_00431450`, `RadarClass::Update @ 0x00656EC0`, `DisplayClass::BandBox_LeftUp @ 0x004AC060`, network dispatch loop `0x0048D1E0`.  
**Investigation mode:** targeted `/re-swarm` slot for the unresolved animated overlay asset/24-slot producer.  
**Status:** COMPLETE for binary ownership/producer/draw cadence; PARTIAL only for live asset header numeric sampling and exact palette conversion internals.

## Summary

The previously described "animated spy-satellite radar overlay" is not a `SpySat=yes` building effect. The verified binary path is the multiplayer beacon overlay.

`Init_BeaconArt @ 0x00430A00` loads `RDRBEACN.SHP` into the global previously called `g_SpySatSHPPtr` and stores its width, height, frame count, and refresh interval into the adjacent globals. `RadarClass::PlaceBeacon @ 0x00430BA0` owns the 8 x 3 = 24-slot producer table. `RadarClass::Update @ 0x00656EC0` calls the update/draw helpers late in the generated radar primary-surface pass, after terrain/object pixels and radar events.

This is a stale-label correction: passive `SpySat=yes` reveal remains the separate `HouseClass::CheckLowPower -> MapClass::BlackoutShroud/RestoreShroud -> RadarClass::RefreshRadar` path from the prior report. There is no verified active standard-YR animated satellite overlay tied to `GASPYSAT` in this slot.

## Target and Non-Scope

Target:

- Verify the asset/SHP pointer behind the late radar overlay helpers.
- Verify frame count/refresh interval derivation.
- Verify the 24-slot producer layout and active gates.
- Verify `DrawSpySatelliteVision` / `DrawOneSpySatellite` behavior and `MarkCellDirty` interaction.
- Verify draw order relative to terrain/object pixels, radar events, and retained sidebar copy.
- Label active standard-YR liveness.

Non-scope:

- Passive `SpySat=yes` full-map reveal/restore semantics, already handled by `SPY_SATELLITE_REVEAL_RADAR_PIXEL_PIPELINE_GHIDRA_REPORT.md`.
- Gap/shroud terrain pixel effects.
- Generic terrain dirty caller matrix.
- Radar event line raster internals.
- Rust/INI edits.

## Verified Binary Findings

### 1. The overlay asset is `RDRBEACN.SHP`, not a SpySat-specific SHP

`Init_BeaconArt @ 0x00430A00` constructs `PBEACON.SHP`, loads it into `g_BeaconArtSHPPtr`, then constructs `RDRBEACN.SHP` and loads it into the global previously labeled `g_SpySatSHPPtr`.

The loader stores `RDRBEACN.SHP` header fields as:

- `base+0x70` / `0x0089C420`: width from `*(short *)(g_SpySatSHPPtr + 2)`.
- `base+0x74` / `0x0089C424`: height from `*(short *)(g_SpySatSHPPtr + 4)`.
- `base+0x78` / `0x0089C428`: frame count from `*(short *)(g_SpySatSHPPtr + 6)`.
- `base+0x7C` / `0x0089C42C`: refresh interval as `frame_count << 2`.

Evidence: `Init_BeaconArt @ 0x00430A00` decompile shows `RawFileClass__Constructor(s_RDRBEACN_SHP_00818a30)`, `g_SpySatSHPPtr = FUN_004a3890()` / `LoadFileFromMIX()`, and the exact header reads/stores above. `ScenarioClass::Full_Init @ 0x00687B80` calls `Init_BeaconArt()` after `MapClass::InitCellAttributes` and before later post-map initialization, so the asset path is active during scenario load.

Inference: the `g_SpySat*` names in older docs are stale labels inherited from previous analysis. The binary evidence ties these globals to radar beacon art.

### 2. The 24-slot table is an 8-house x 3-beacon producer table

`RadarClass::PlaceBeacon @ 0x00430BA0` indexes `param_1 + param_2 * 0x0C`, scans three dword slots for the selected row, allocates a `0x114`-byte beacon object, and stores the pointer at:

```text
(slot + house_row * 3) * 4
```

It maintains a count at `param_1+0x60`, sets the owner row at `beacon+0x110`, and sets flag bit 0 at `beacon+0x0C` when `param_2 < 8`.

The local placement route is active from `DisplayClass::BandBox_LeftUp @ 0x004AC060`: action code `0x3C` calls `RadarClass::PlaceBeacon(g_PlayerPtr+0x30, cell_x*0x100+0x80, cell_y*0x100+0x80, cell_z, -1)` when the beacon-disabled gate `DAT_00A8B538 == 0` passes.

The network route is active from the network dispatch loop `0x0048D1E0`: message case `0x20` calls `RadarClass::PlaceBeacon`; cases `0x21` and `0x22` call beacon delete/text update helpers.

### 3. `HasSpySatelliteUpdate` is a beacon overlay update predicate

`HasSpySatelliteUpdate @ 0x00431800` takes the beacon table pointer and:

- Returns false if table count `param_1[0x18]` is zero.
- Scans 8 rows x 3 slots.
- Requires a non-null beacon pointer.
- Requires beacon flag bit 0 at `beacon+0x0C`.
- Requires `HouseClass__Is_Ally_ByIndex(beacon+0x110)`.
- Requires `HouseClass__IsAlliedWith(owner_house, g_PlayerPtr)`.
- Requires owner house byte `+0x1F5 == 0`.
- Returns true only when `g_CurrentFrameCounter % param_1[0x1F] < g_SpySatRefreshFrameCount + 1`.

Since `param_1[0x1F]` is the `base+0x7C` refresh interval set by `Init_BeaconArt`, the active cadence is:

```text
phase = g_CurrentFrameCounter % (RDRBEACN_frame_count * 4)
update window: phase < RDRBEACN_frame_count + 1
draw window:   phase < RDRBEACN_frame_count
```

### 4. `DrawSpySatelliteVision` iterates the same 24 beacon slots

`DrawSpySatelliteVision @ 0x00431700` repeats the same count, slot, ally, and owner-suppression gates as `HasSpySatelliteUpdate`. For each matching beacon it calls:

```text
DrawOneSpySatellite(..., final_flag = 0)
```

Despite the stale function name, this draws the radar beacon art, not passive SpySat reveal pixels.

### 5. `DrawOneSpySatellite` draws one `RDRBEACN.SHP` frame and then dirties its coverage rectangle

`DrawOneSpySatellite @ 0x00430650` computes:

```text
phase = g_CurrentFrameCounter % g_SpySatRefreshInterval
```

If `phase < g_SpySatRefreshFrameCount` and the final flag is zero, it calls:

```text
CC_Draw_Shape(g_SpySatSHPPtr, phase, anchor, ..., flags 0x600, ..., z 1000, ...)
```

The anchor comes from `FUN_006557F0(..., beacon, ..., clamp=1)`, which converts/clamps the beacon world position into radar pixel coordinates.

After the optional shape draw, it loops over the centered rectangle:

```text
left = anchor_x - g_SpySatSHPWidth / 2
top = anchor_y - g_SpySatSHPHeight / 2
right-exclusive = left + g_SpySatSHPWidth
bottom-exclusive = top + g_SpySatSHPHeight
```

For every integer pixel in that rectangle, it calls:

```text
RadarClass__MarkCellDirty(&pixel)
```

This means the final `phase == frame_count` update frame marks dirty without drawing a SHP frame.

### 6. Beacon deletion uses `DrawOneSpySatellite(..., final_flag = 1)`

`FUN_004311C0` is the beacon delete helper. Before freeing an allied visible beacon, it calls:

```text
DrawOneSpySatellite(g_RadarDrawSurface, ..., final_flag = 1)
```

With final flag set, `DrawOneSpySatellite` enters the coverage-dirty path regardless of the modulo window, but the SHP draw branch is skipped because it requires final flag zero. This is the native cleanup path that forces the old beacon footprint to be recomposed after deletion.

### 7. Draw order is terrain/object pixels, radar events, then beacon overlay, then sidebar copy

`RadarClass::Update @ 0x00656EC0` orders the primary radar surface pass as:

1. `RadarClass__ClearBackground`.
2. Terrain dirty secondary-surface refresh and `RenderCellPixel` over the dirty terrain rectangle.
3. Explicit pixel dirty list `RenderCellPixel`.
4. Dirty-list cleanup via vtable `+0x0C`.
5. `TickAndDrawRadarEvents`.
6. `DrawSpySatelliteVision` over the primary radar surface.
7. Active radar/chrome/content copy into `g_SidebarSurface`.

Therefore beacon overlay pixels sit above generated minimap terrain/object pixels and radar event lines, but below later sidebar-surface/screen-copy consumers such as frame-level tooltips.

### 8. Colors/packing are asset-draw driven, not generated terrain or sidebar palette logic

The verified draw operation is `CC_Draw_Shape(g_SpySatSHPPtr, phase, ..., flags 0x600, ..., z 1000, ...)` using the `RDRBEACN.SHP` pointer. No generated minimap terrain color, house color-scheme dot packing, `SIDEBAR.PAL`, `CAMEO.PAL`, or passive `SpySat` shroud/fog branch participates in this overlay draw.

Remaining palette/conversion detail is not fully reduced in this slot: the `CC_Draw_Shape` ConvertClass/current draw-context path for `RDRBEACN.SHP` needs a separate palette-routing proof if exact beacon colors become a blocker.

## Active in Standard YR?

| Path | Active standard YR? | Evidence |
|---|---:|---|
| `RDRBEACN.SHP` load | Yes | `ScenarioClass::Full_Init @ 0x00687B80` calls `Init_BeaconArt`; loader names `RDRBEACN.SHP`. |
| 24-slot table producer | Yes, multiplayer beacon feature | `DisplayClass::BandBox_LeftUp` action `0x3C`; network dispatch case `0x20`; hotkey docs identify beacon placement. |
| Late overlay update/draw | Conditional | `RadarClass::Update` calls `HasSpySatelliteUpdate` / `DrawSpySatelliteVision` when beacon table has allied visible entries and frame modulo is inside the window. |
| Passive `SpySat=yes` building reveal | Yes, but separate | Prior report verifies `HouseClass::CheckLowPower` route. This report found no animated overlay tie to `GASPYSAT`. |
| Animated SpySat-building overlay | No verified active path | The resolved asset and producer are beacon-specific. |

## Implementation Handoff

| Verified fact | Rust delta / risk | Likely touchpoints | Acceptance test |
|---|---|---|---|
| `g_SpySatSHPPtr` path is actually `RDRBEACN.SHP` beacon overlay art. | Avoid implementing a fake animated `SpySat=yes` overlay. | `src/render/minimap.rs`, future beacon module, `src/app_render/draw_passes.rs` | `test_spysat_building_has_no_rdrbeacn_overlay_without_beacon`: building `GASPYSAT` refreshes minimap via shroud flags but does not draw `RDRBEACN`. |
| Beacon table is 8 rows x 3 slots with count at `+0x60`. | Rust currently has no native multiplayer beacon overlay model in the minimap renderer. | future `sim/beacon.rs` or `sim/radar.rs`, `src/render/minimap.rs` | `test_beacon_table_accepts_three_slots_per_house_and_replaces_explicit_slot`. |
| `RDRBEACN` update window is `frame_count+1`; draw window is `frame_count`; interval is `frame_count*4`. | Do not use arbitrary animation speed or draw on the dirty-only final phase. | `src/render/minimap.rs` | `test_rdrbeacn_phase_frame_count_marks_dirty_without_drawing_shape`. |
| Deletion calls final-flag dirty pass. | Removing a beacon must recompose the old footprint even if the animation modulo would otherwise be outside the update window. | future beacon deletion/network path; `src/render/minimap.rs` | `test_beacon_delete_forces_dirty_footprint_without_shape_draw`. |
| Overlay order is after radar events, before primary surface copy to retained sidebar. | Do not draw beacon as final UI above tooltips/sidebar overlays or underneath event lines. | `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs`, `src/render/minimap.rs` | `test_beacon_overlay_overwrites_radar_event_pixel_before_sidebar_copy`. |

## Negative Facts / Do Not Do

- Do not implement the `RDRBEACN.SHP` overlay as a `SpySat=yes` building effect.
- Do not describe the 24-slot producer as a spy-satellite building grid; it is the multiplayer beacon grid.
- Do not use passive `SpySat=yes` reveal as direct minimap painting or as a `RDRBEACN` animation trigger.
- Do not treat draw and dirty windows as identical; `phase == frame_count` dirties without drawing.
- Do not place the beacon overlay above frame-level tooltips or outside the retained primary-radar/sidebar-surface update order.
- Do not color this path through generated terrain/object dot palettes; it is a SHP draw path.

## Remaining Uncertainty

- Exact live numeric `RDRBEACN.SHP` width, height, and frame count were verified as header-derived but not sampled from the retail asset in this slot.
- Exact `CC_Draw_Shape` ConvertClass/palette route for `RDRBEACN.SHP` was not fully reduced.
- The `FUN_006557F0` decompile shows clamped radar-pixel output but does not expose all floating-point inputs cleanly; exact cell/world-to-radar rounding should defer to the minimap inverse/coordinate reports if needed.
- Some function names in Ghidra and older docs remain stale (`DrawSpySatelliteVision`, `DrawOneSpySatellite`, `HasSpySatelliteUpdate`) and should not be used as semantic proof.

## Stale-Doc Replacement Wording

For docs that say `DrawSpySatelliteVision` draws animated spy-satellite vision or a `SpySat=yes` building overlay, replace with:

> The functions historically labeled `HasSpySatelliteUpdate @ 0x00431800`, `DrawSpySatelliteVision @ 0x00431700`, and `DrawOneSpySatellite @ 0x00430650` operate on the multiplayer beacon overlay table, not passive `SpySat=yes` building reveal. `Init_BeaconArt @ 0x00430A00` loads `RDRBEACN.SHP` into the global previously labeled `g_SpySatSHPPtr`, stores its width/height/frame count, and sets the refresh interval to `frame_count * 4`. `RadarClass::PlaceBeacon @ 0x00430BA0` owns the 8 x 3 beacon-slot producer. The overlay draws after radar events on the primary radar surface and then marks the beacon-art footprint dirty for recomposition.

For implementation notes that say `SpySat=yes` has an animated minimap overlay, replace with:

> Passive `SpySat=yes` reveal has no verified `RDRBEACN.SHP` overlay. Its active building path is the separate `HouseClass::CheckLowPower -> MapClass::BlackoutShroud/RestoreShroud -> RadarClass::RefreshRadar` mechanism. `RDRBEACN.SHP` belongs to the multiplayer beacon system.

## Status

COMPLETE for the binary ownership correction, 24-slot producer, active gates, draw/dirty cadence, and update order. PARTIAL only for live asset header numeric sampling and exact `RDRBEACN.SHP` palette conversion.

