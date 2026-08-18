# Spy Satellite Reveal Radar Pixel Pipeline - Ghidra Research Report

Date: 2026-05-27

**Slot:** /re-swarm radar/minimap follow-up slot 3  
**Target:** `SPY_SATELLITE_REVEAL_RADAR_PIXEL_PIPELINE`  
**Address(es):** `HouseClass__CheckLowPower @ 0x00508F60`, `MapClass__BlackoutShroud @ 0x00577D90`, `MapClass__RestoreShroud @ 0x00577AB0`, `MapClass__ResetShroudWithReveal @ 0x00577C90`, `MapClass__RevealEntireMap @ 0x00577F30`, `RadarClass__Update @ 0x00656EC0`, `HasSpySatelliteUpdate @ 0x00431800`, `DrawSpySatelliteVision @ 0x00431700`, `DrawOneSpySatellite @ 0x00430650`, `RadarClass__RefreshRadar @ 0x00657CE0`  
**Investigation Mode:** exhaustive-slice for `SpySat=yes` passive full-map reveal and its immediate radar/minimap pixel consequences; coverage-map for the separate animated spy-satellite radar overlay geometry.  
**Claimed Scope:** Active standard YR `SpySat=yes` building activation/deactivation, shroud/fog cell mutation, radar full-refresh trigger, and the periodic radar overlay/update ordering.  
**Non-Scope:** Soviet spy-plane superweapon, Psychic Reveal superweapon, crate Reveal, generic terrain color pipeline, generic radar dirty queues except exact consumers cited here, full runtime pixel-format sampling, and exhaustive satellite SHP frame asset dump.  
**Confidence:** High for passive SpySat reveal/restore state writes and `RadarClass::Update` ordering; Medium for the animated overlay's exact SHP asset identity/frame dimensions because this pass verified code shape and dirty coverage but did not dump the asset frames.  
**Active in YR:** Yes. Stock YR has `[GASPYSAT] Radar=yes SpySat=yes` and powered/online `SpySat=yes` buildings are checked by `HouseClass__CheckLowPower`.

## Summary

`SpySat=yes` full-map reveal does not directly paint terrain/minimap pixels. When an eligible powered SpySat building becomes available for the local player, `HouseClass__CheckLowPower` calls `MapClass__BlackoutShroud`, which mutates the player's shroud/fog cell flags across the map, sets player reveal state, calls `RadarClass__RefreshRadar`, and dirties tactical redraw state.

There is also a separate radar-surface spy-satellite overlay system. `RadarClass__Update` treats `HasSpySatelliteUpdate` as a redraw condition, then after terrain/pixel dirty recomposition and radar events calls `DrawSpySatelliteVision`; that path may draw a satellite SHP frame into the primary radar surface and then marks every pixel in the satellite coverage rectangle dirty for later recomposition. This overlay is not the mechanism that reveals the map.

## Target and Non-Scope

Target:

- Passive full-map reveal from `BuildingTypeClass+0x16A5 SpySat`.
- The active standard YR route that turns the passive reveal on and off.
- Whether reveal directly paints minimap/radar pixels or mutates map/shroud state.
- Timing/order relative to `RadarClass__RefreshRadar`, generic radar dirty lists, generated minimap primary surface, and sidebar blit.
- Any special radar colors/shapes found on the spy-satellite-specific path.

Non-scope:

- `SpyPlaneSpecial` / `[SPYP]` flyby reveal.
- `PsychicRevealSpecial`.
- Crate `Reveal`.
- Generic `RenderCellPixel` terrain/object/shroud formulas except as the immediate consumer after `RefreshRadar`.
- Full `g_SpySatSHPPtr` asset load census and all frame metadata.
- Rust implementation patches.

## Verified Binary Findings

### 1. `SpySat` is a building-type INI bool at `BuildingTypeClass+0x16A5`

Active in YR: Yes.

`BuildingTypeClass_ReadINI_Water @ 0x0045FE50` reads:

```text
Radar  -> BuildingTypeClass+0x16A4
SpySat -> BuildingTypeClass+0x16A5
```

The stock YR active buildable Allied Spy Satellite Uplink has both `Radar=yes` and `SpySat=yes`:

```text
rulesmd.ini:12187 [GASPYSAT]
rulesmd.ini:12194 Radar=yes
rulesmd.ini:12195 SpySat=yes
rulesmd.ini:12204 Power=-100
rulesmd.ini:12205 Powered=true
```

Stock YR also contains civilian/campaign structures with `Radar=yes` and `SpySat=yes` such as `[CACHIG05]` and `[CANEWY05]`, but `[GASPYSAT]` is the standard skirmish/player-owned route.

### 2. Passive SpySat activation is checked from the house power/radar maintenance path

Active in YR: Yes for houses with eligible `SpySat=yes` buildings.

`HouseClass__CheckLowPower @ 0x00508F60` scans the house building vector and selects a building only when all relevant conditions pass:

- Building pointer non-null.
- `building->Type+0x16A5 != 0`.
- `building+0x81 == 0` (not limbo/dead in the same pattern used elsewhere).
- `building+0x74 != 0` (active/placed).
- Single-player special gating: if `g_GameMode == 0`, the building must pass either current/player-control constraints or visible/local-player constraints; otherwise non-SP game mode permits local-player branch.
- Building must not be in mission/state `0x13` at fields `+0xAC` or `+0xB4`.
- Virtual `+0x1D4` must return `0`; this is the same online/powered-style gate used by power/radar checks.

When the first eligible building is found and `HouseClass::SpySatActive` is false, the function calls `MapClass__BlackoutShroud(this)`, sets `SpySatActive = true`, and plays the SpySat activation sound only if `this == g_PlayerPtr`.

When no eligible building remains and `SpySatActive` is true, it calls `MapClass__RestoreShroud(this)`, sets `SpySatActive = false`, and plays the deactivation sound only for `g_PlayerPtr`.

### 3. The activation function mutates shroud/fog flags; it does not directly paint radar pixels

Active in YR: Yes for the local player route.

Despite the decompiler label `MapClass__BlackoutShroud`, `0x00577D90` is the passive SpySat full-map reveal operation used by `CheckLowPower`.

Important details:

- If a non-null house argument is supplied, it sets that house-array entry `+0x241 = 1`.
- If the house argument is neither null nor `g_PlayerPtr`, it returns early. Therefore the visible local full-map reveal is local-player scoped.
- If `g_PlayerPtr+0x240` is already nonzero, the whole reveal body is skipped. This makes activation idempotent while already revealed.
- It calls `MapClass__ParanoidRevealAll(0,0)` before the cell loop and `MapClass__ParanoidUnrevealAll(0,0)` after.
- It initializes the map cell iterator from map fields `DAT_0087F8DC` / `g_CellArray_Base`.
- For each included cell it writes:
  - `CellClass+0x130 = 0`
  - `CellClass+0x134 = 0`
  - `CellClass+0x12C |= 0x18`
  - `CellClass+0x140 |= 0x03`
- It sets `g_PlayerPtr+0x240 = 1`.
- It calls `RadarClass__RefreshRadar()`.
- It calls `FUN_004F42F0(2)`, which sets `g_Tactical+0xD7D = 1` when tactical exists and, when its second argument is nonzero, may update a map/render state and bridge counter. In this call shape the decompiler's argument display is imperfect, but the tactical dirty side effect is verified.

The cell writes prove the passive reveal path changes shroud/fog state first. Radar pixels update afterward through `RefreshRadar` and normal `RenderCellPixel` rules.

### 4. Passive SpySat deactivation resets shroud/fog then refreshes radar

Active in YR: Yes when the local player loses their last eligible active SpySat building.

`MapClass__RestoreShroud @ 0x00577AB0` is called by `HouseClass__CheckLowPower` when `SpySatActive` transitions from true to false. It:

- Clears the house-array entry `+0x241 = 0` for a non-null argument.
- Returns early for non-local non-null house arguments.
- Calls `MapClass__ParanoidRevealAll(0,0)`.
- Iterates all cells and writes:
  - `CellClass+0x12C &= ~0x18`
  - `CellClass+0x130 = 1`
  - `CellClass+0x134 = 0`
  - `CellClass+0x140 &= ~0x03`
- Calls `MapClass__ParanoidUnrevealAll(0,0)`.
- Clears `g_PlayerPtr+0x240 = 0`.
- Calls `RadarClass__RefreshRadar()`.
- Calls the map vtable slot `+0x38` with `2`, matching the same broad redraw/update style as reset functions.

`MapClass__ResetShroudWithReveal @ 0x00577C90` is a related reset variant. It calls `FUN_00487690` per cell, then only clears `+0x12C`/`+0x140` if `CellClass+0x130 >= 0`; it is not the direct passive SpySat activation path from `CheckLowPower`, but it confirms the same reset/refresh family.

### 5. `MapClass__RevealEntireMap` is a different reveal path and uses `RevealCell`

Active in YR: Conditional; not the direct `SpySat=yes` building path verified in `CheckLowPower`.

`MapClass__RevealEntireMap @ 0x00577F30` also sets a house-array `+0x241` byte and `g_PlayerPtr+0x240`, but unlike passive SpySat activation it iterates cells and calls `RevealCell(cellCoord, g_PlayerPtr)`. It then calls `RadarClass__RefreshRadar()` and `FUN_004F42F0(1)`.

This matters because `RevealCell @ 0x004AA050` performs per-cell edge recomputation, calls `FUN_006DA7D0` for tactical shroud-edge dirtying, and calls `CellChangeNotify` when the explored state changed. Passive SpySat activation at `0x00577D90` uses direct flag writes instead of calling `RevealCell` for each cell.

### 6. `RadarClass__RefreshRadar` repaints through the normal primary minimap pixel path

Active in YR: Yes when reveal/restore calls it.

`RadarClass__RefreshRadar @ 0x00657CE0` does not draw a special SpySat color or shape. It refreshes the radar surface:

- If `g_hWnd != 0`, it loops over primary radar surface width and height and calls `RadarClass__RenderCellPixel(&pixel)` for every pixel.
- If `g_hWnd == 0`, it blits/copies the secondary/generated terrain surface into the primary surface and calls `RadarClass__RenderAllCells()`.

Therefore passive SpySat's player-visible minimap consequence is: cell shroud/fog flags change, then every radar pixel is recomposed by the ordinary minimap pipeline. Special spy-satellite SHP pixels are not part of this passive full-map reveal step.

### 7. Generic radar update treats spy-satellite overlay as a late overlay and redraw trigger

Active in YR: Yes when active radar state runs and allied satellite entries exist.

`RadarClass__Update @ 0x00656EC0` includes `HasSpySatelliteUpdate()` in the condition that decides whether a radar update pass should run. If the pass runs, ordering is:

1. Clear terrain dirty background.
2. Mark viewport-edge pixels if the viewport moved.
3. Blit generated terrain dirty rect from secondary to primary and rerender those final pixels.
4. Rerender explicit pixel dirty list.
5. Clear pixel dirty vector.
6. `TickAndDrawRadarEvents()`.
7. `DrawSpySatelliteVision(...)`.
8. If active/open radar state and accumulated dirty rect is positive, copy the dirty rect from primary radar surface to `g_SidebarSurface`.
9. Draw viewport rectangles.
10. Clear the visited bitfield after the pass.

The spy-satellite overlay is therefore above ordinary terrain/object/shroud pixels and above radar event timing in this update pass, but before the primary-surface-to-sidebar copy.

### 8. `HasSpySatelliteUpdate` checks a 24-slot satellite entry grid and frame modulo

Active in YR: Conditional on entries in the satellite tracker and frame cadence.

`HasSpySatelliteUpdate @ 0x00431800`:

- Returns false if `param_1[0x18] == 0`.
- Iterates `8` groups of `3` entries, for `24` total pointer slots.
- For each non-null object pointer, requires `object+0x0C bit0` set.
- Requires `HouseClass__Is_Ally_ByIndex(object+0x110) != 0`.
- Requires `HouseClass__IsAlliedWith(ownerHouse, g_PlayerPtr)`.
- Requires owner house byte `+0x1F5 == 0` (not observer/suppressed in this context).
- If any such entry exists, returns true only when:

```text
g_CurrentFrameCounter % param_1[0x1F] < g_SpySatRefreshFrameCount + 1
```

This is a redraw cadence gate for the overlay path. It is not the passive full-map shroud reveal activation gate.

### 9. `DrawSpySatelliteVision` uses the same 24-slot filter and calls `DrawOneSpySatellite`

Active in YR: Conditional, same slot and frame conditions as `HasSpySatelliteUpdate`.

`DrawSpySatelliteVision @ 0x00431700`:

- Requires `param_1[0x18] != 0`.
- Requires `g_CurrentFrameCounter % param_1[0x1F] < g_SpySatRefreshFrameCount + 1`.
- Iterates `8 x 3` entry pointers.
- Applies the same valid/ally/not-observer gates.
- Calls `DrawOneSpySatellite(..., 0)` for each matching entry.

The final constant `0` matters: inside `DrawOneSpySatellite`, the SHP frame draw runs only when `iVar5 < g_SpySatRefreshFrameCount` and that final flag is zero. The dirty marking still runs when `iVar5 < g_SpySatRefreshFrameCount + 1`.

### 10. `DrawOneSpySatellite` draws a SHP frame and queues primary radar pixels dirty over its coverage rectangle

Active in YR: Conditional on the overlay path above.

`DrawOneSpySatellite @ 0x00430650`:

- Computes `phase = g_CurrentFrameCounter % g_SpySatRefreshInterval`.
- Runs only when `phase < g_SpySatRefreshFrameCount + 1` or the caller's final flag is nonzero.
- Calls `FUN_006557F0(&local_30, satellite, 1)` to get a radar pixel anchor.
- If `phase < g_SpySatRefreshFrameCount` and final flag is zero, calls:

```text
CC_Draw_Shape(g_SpySatSHPPtr, phase, anchor, ..., flags 0x600, ..., z 1000, ...)
```

- Computes a coverage rectangle centered on the anchor:

```text
left = anchor_x - g_SpySatSHPWidth / 2
top  = anchor_y - g_SpySatSHPHeight / 2
right-exclusive loop bound = left + g_SpySatSHPWidth
bottom-exclusive loop bound = top + g_SpySatSHPHeight
```

- Unions that rectangle with global/cached radar dirty rect globals `DAT_008809F4/F8/FC/A00`, using inclusive-style `+1` expansion when extending right/down.
- Calls `AlphaShapeClass__ClipRect(&g_RadarSurfaceOriginX,0,0)` and stores the resulting rect back to `DAT_008809F4/F8/FC/A00`.
- Loops every integer pixel in the uncentered coverage rectangle and calls `RadarClass__MarkCellDirty(&pixel)`.

The important negative fact is that this overlay path does not itself clear shroud or reveal cells. It marks primary radar pixels dirty after drawing the overlay so a later update can recompose the affected pixels.

### 11. Passive reveal and animated overlay are related but not the same mechanism

Active in YR: Yes/conditional as above.

There are two separate pixel consequences:

- Passive full-map reveal/restore: `CheckLowPower -> MapClass__BlackoutShroud/RestoreShroud -> RadarClass__RefreshRadar -> RenderCellPixel all pixels`. This changes shroud/fog flags and immediately recomposes the minimap surface.
- Satellite overlay cadence: `RadarClass__Update -> HasSpySatelliteUpdate -> DrawSpySatelliteVision -> DrawOneSpySatellite -> MarkCellDirty coverage`. This is a periodic overlay/dirty mechanism over the primary radar surface.

Future Rust should not merge these into one high-level "SpySat draws minimap" effect.

## Active in Standard YR?

| Finding | Active in standard YR? | Condition / default |
|---|---|---|
| `SpySat` INI key at `BuildingTypeClass+0x16A5` | Yes | Read by `0x0045FE50`; `[GASPYSAT] SpySat=yes` in `rulesmd.ini` |
| Passive full-map reveal activation via `HouseClass__CheckLowPower` | Yes | Player/house owns an eligible active/powered `SpySat=yes` building; `[GASPYSAT] Powered=true Power=-100` |
| Passive reveal direct cell flag writes | Yes | Local-player route; skipped if `g_PlayerPtr+0x240` already nonzero |
| Passive deactivation restore | Yes | Last eligible `SpySat=yes` building lost/offline while `SpySatActive` true |
| `MapClass__RevealEntireMap` | Conditional | Binary-live for other reveal routes; not the direct `SpySat=yes` building route proven here |
| Animated satellite radar overlay | Conditional | Requires satellite entry grid count nonzero, allied/non-observer entries, and frame modulo within refresh window |
| Special SpySat minimap colors for passive reveal | No | Passive reveal uses ordinary `RenderCellPixel`; no special passive color/shape found |

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `SpySat=yes` is parsed into `BuildingTypeClass+0x16A5` and `[GASPYSAT]` is the standard active building route. | `0x0045FE50`; `ini/rulesmd.ini:12187..12205` | Partial: Rust parses `spy_sat` in `src/rules/object_type.rs` and uses it in radar/power/vision paths, but exact online/house transition semantics are unchecked. | `src/rules/object_type.rs`, `src/sim/world/mod.rs`, `src/sim/power_system.rs` | Preserve `SpySat` as building-type capability and gate full reveal on native active/online checks, not just type presence. | Build/power down/sell/destroy `[GASPYSAT]`; reveal activates only while an eligible active building exists. | Do not treat any `SpySat=yes` object as always-on regardless of power/limbo/current-player constraints. |
| Passive activation directly writes all local-player cell shroud/fog flags and calls `RadarClass__RefreshRadar`; it does not draw special minimap pixels. | `HouseClass__CheckLowPower @ 0x00508F60`; `MapClass__BlackoutShroud @ 0x00577D90`; `RadarClass__RefreshRadar @ 0x00657CE0` | Mismatch risk: Rust `apply_spy_sat` sets all cells visible/revealed each recompute; native activation is transition/idempotent via `g_PlayerPtr+0x240` and triggers a radar full refresh. | `src/sim/vision/mod.rs`, `src/sim/world/mod.rs`, `src/render/minimap.rs`, `src/app_render/draw_passes.rs` | Model transition: on first activation set reveal state and force full radar/minimap recomposition through normal pixel pipeline. | `test_spy_sat_activation_refreshes_minimap_via_shroud_flags_not_overlay`: acquiring GASPYSAT changes shrouded minimap pixels to ordinary terrain/object pixels with no special satellite color. | Do not implement passive reveal by painting the minimap texture directly or drawing a satellite overlay as the reveal mechanism. |
| Passive deactivation clears shroud/fog flags, resets reveal state, calls `RadarClass__RefreshRadar`, and only then leaves current LOS/reveal mechanisms to repopulate. | `MapClass__RestoreShroud @ 0x00577AB0`; `HouseClass__CheckLowPower @ 0x00508F60` | Partial/mismatch risk: Rust has SpySat reshroud hooks on sell/destroy, but per-power/offline transition and native reset fields are unchecked. | `src/sim/combat/mod.rs`, `src/sim/production/production_sell.rs`, `src/sim/world/mod.rs`, `src/sim/vision/mod.rs` | On loss of the last eligible active SpySat, reset full-map reveal and force full radar recomposition, then normal LOS/paranoid reveal paths can reapply current sight. | `test_spy_sat_loss_reshrouds_and_refreshes_minimap`: after destroying/powering off last GASPYSAT, previously only-SpySat-revealed cells become black on minimap until ordinary LOS reveals them. | Do not leave all cells visible until the next ordinary vision recompute without a full radar refresh. |
| `MapClass__RevealEntireMap` is a different route using `RevealCell` and tactical edge dirtying, not the passive SpySat activation function. | `0x00577F30`; `RevealCell @ 0x004AA050`; `CellChangeNotify @ 0x005865F0` | Unchecked: Rust may conflate reveal-map debug/dev toggles, crate reveal, psychic reveal, and SpySat. | `src/sim/vision/mod.rs`, `src/sim/superweapon/psychic_reveal.rs`, dev reveal toggles in `src/app.rs` | Keep full-map reveal routes distinct when parity requires edge/tactical dirty behavior. | `test_spy_sat_full_reveal_does_not_call_revealcell_edge_cascade`: passive SpySat activation should not consume the per-cell `RevealCell` recursion/edge path. | Do not reuse Psychic Reveal or crate Reveal code blindly for `SpySat=yes`. |
| Animated satellite overlay is late in `RadarClass__Update`, after terrain/pixel dirty and radar events, before sidebar-surface copy. | `0x00656EC0`, `0x00431800`, `0x00431700`, `0x00430650` | Missing: current Rust minimap renderer has no native SpySat overlay/coverage dirty pass. | `src/render/minimap.rs`, `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs` | If implementing overlay parity, draw/dirty the overlay in the primary radar surface update order, not as a separate top-level UI sprite. | `test_spy_sat_overlay_draws_before_sidebar_copy_and_marks_coverage_dirty`: overlay frame is visible for the native modulo window and its coverage pixels are dirty/recomposed afterward. | Do not draw the satellite overlay after the minimap has already been copied/drawn as final UI. |
| `DrawOneSpySatellite` dirties a centered `g_SpySatSHPWidth x g_SpySatSHPHeight` pixel rectangle and uses frame `phase` only while `phase < frame_count`; the `+1` frame window still marks dirty. | `0x00430650` | Missing/unchecked. | `src/render/minimap.rs` or future native radar surface model | Preserve the `frame_count + 1` dirty window and the exact centered rectangle loops. | `test_spy_sat_overlay_last_dirty_frame_without_shape_draw`: on phase `frame_count`, no SHP frame is drawn but coverage pixels are still marked dirty. | Do not assume the draw and dirty windows are identical. |

## Negative Facts / Do Not Do

- Do not implement passive `SpySat=yes` as direct minimap painting. The passive path mutates shroud/fog cell flags and calls `RadarClass__RefreshRadar`.
- Do not use the animated spy-satellite overlay as the full-map reveal mechanism. It is a separate late radar-surface overlay/dirty path.
- Do not collapse `MapClass__BlackoutShroud @ 0x00577D90`, `MapClass__RestoreShroud @ 0x00577AB0`, `MapClass__ResetShroudWithReveal @ 0x00577C90`, and `MapClass__RevealEntireMap @ 0x00577F30`; their cell writes and helper calls differ.
- Do not make passive SpySat reveal run every frame as a full cell rewrite. Native skips activation body when `g_PlayerPtr+0x240` is already set.
- Do not ignore deactivation/power loss. `HouseClass__CheckLowPower` actively calls restore when no eligible SpySat remains and `SpySatActive` is true.
- Do not add special passive SpySat minimap colors. Ordinary `RenderCellPixel` decides terrain/object/shroud/fog pixels after the shroud flags change.
- Do not treat the overlay's `g_SpySatRefreshFrameCount + 1` update window as the same as its SHP draw window; the final phase can dirty without drawing a frame.

## Remaining Uncertainty

- The exact SHP asset name/payload behind `g_SpySatSHPPtr`, its frame count, width, height, and palette conversion path were not exhaustively dumped in this slot.
- Runtime values for `g_SpySatRefreshInterval`, `g_SpySatRefreshFrameCount`, `g_SpySatSHPWidth`, and `g_SpySatSHPHeight` were read as globals in code but not sampled from a running process.
- The producer that fills the 24-slot satellite entry grid was not exhaustively traced; this report verifies the consumer/filter/update path.
- `FUN_004F42F0` decompilation has argument-display ambiguity, but the tactical dirty byte write at `g_Tactical+0xD7D = 1` is clear.
- Standard mission/campaign cases using civilian `SpySat=yes` structures were identified from INI but not separately runtime-proven.

## Stale-Doc Replacement Wording

For `docs/research/RADAR_MINIMAP_RENDERING.md` section "Spy Satellite Vision System", replace broad wording that says "`SpySat=yes` buildings reveal the entire map for allied players" with:

> `SpySat=yes` has two separate radar/minimap consequences. Passive full-map reveal is owned by `HouseClass__CheckLowPower @ 0x00508F60`: when a local eligible active `SpySat=yes` building appears, it calls `MapClass__BlackoutShroud @ 0x00577D90`, directly sets all local shroud/fog cell flags to revealed/interior, then calls `RadarClass__RefreshRadar @ 0x00657CE0`; it does not paint special minimap pixels. The animated spy-satellite radar overlay is separate: `RadarClass__Update @ 0x00656EC0` uses `HasSpySatelliteUpdate @ 0x00431800` as an update condition and later calls `DrawSpySatelliteVision @ 0x00431700`, which calls `DrawOneSpySatellite @ 0x00430650` to draw `g_SpySatSHPPtr` frames and mark the overlay coverage pixels dirty.

For any implementation note saying "SpySat reveal is equivalent to reveal all cells visible every tick", replace with:

> Native passive SpySat activation is transition/idempotent. `MapClass__BlackoutShroud @ 0x00577D90` skips the cell rewrite when `g_PlayerPtr+0x240` is already set; deactivation uses `MapClass__RestoreShroud @ 0x00577AB0` and refreshes radar.

## Status

COMPLETE for the passive `SpySat=yes` full-map reveal path and its immediate radar/minimap pixel pipeline consequences. PARTIAL for the separate animated spy-satellite overlay asset constants and 24-slot producer path.

## Sources

- Ghidra decompile: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`.
- Ghidra decompile: `HouseClass__CheckLowPower @ 0x00508F60`.
- Ghidra decompile: `MapClass__BlackoutShroud @ 0x00577D90`.
- Ghidra decompile: `MapClass__RestoreShroud @ 0x00577AB0`.
- Ghidra decompile: `MapClass__ResetShroudWithReveal @ 0x00577C90`.
- Ghidra decompile: `MapClass__RevealEntireMap @ 0x00577F30`.
- Ghidra decompile: `RevealCell @ 0x004AA050`, `CellChangeNotify @ 0x005865F0`, `FUN_006DA7D0`.
- Ghidra decompile: `RadarClass__RefreshRadar @ 0x00657CE0`.
- Ghidra decompile: `RadarClass__Update @ 0x00656EC0`.
- Ghidra decompile: `HasSpySatelliteUpdate @ 0x00431800`, `DrawSpySatelliteVision @ 0x00431700`, `DrawOneSpySatellite @ 0x00430650`.
- Ghidra string anchor report: `SpySat @ 0x0081AE58`, `SpySatActivationSound @ 0x0083A5E8`, `SpySatDeactivationSound @ 0x0083A5D0`.
- INI checked: `ini/rulesmd.ini` `[GASPYSAT]`, `[CACHIG05]`, `[CANEWY05]`.
- Prior docs referenced: `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE_GHIDRA_REPORT.md`, `RADAR_MINIMAP_RENDERING.md`, `RADAR_SYSTEM_COMPREHENSIVE.md`, `GLOBAL_SOUNDS_GHIDRA_REPORT.md`.
- Rust scanned: `src/sim/vision/mod.rs`, `src/sim/world/mod.rs`, `src/sim/combat/mod.rs`, `src/sim/production/production_sell.rs`, `src/render/minimap.rs`, `src/app_render/build_instances.rs`.
